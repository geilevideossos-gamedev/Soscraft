use std::path::{Path, PathBuf};

use minecraftoss_player::{Difficulty, GameMode};
use minecraftoss_world::game_rules::{GameRules, Rule, Value};

use crate::args::{Arg, ItemArg, Selector};
use crate::permissions::{ALL, OWNERS, OpList};
use crate::players::offline_uuid;
use crate::protocol::{
    Clientbound, Outgoing, PlayerId, Recipient, Serverbound, allowed_in_chat, normalize_chat,
};
use crate::reader::CommandError;
use crate::text::{GRAY, RED, Text};
use crate::tree::{parse, runnable};

pub const SEND_COMMAND_FEEDBACK: &str = "send_command_feedback";
pub const LOG_ADMIN_COMMANDS: &str = "log_admin_commands";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weather {
    Clear,
    Rain,
    Thunder,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Clock {
    pub total: i64,
    pub game_time: i64,
    pub paused: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub feet: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}

impl Pose {
    pub const ORIGIN: Pose = Pose {
        feet: [0.0; 3],
        yaw: 0.0,
        pitch: 0.0,
    };
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandRegistries {
    pub items: Vec<String>,
    pub entities: Vec<String>,
    pub effects: Vec<String>,
    pub enchantments: Vec<String>,
}

pub fn registered(list: &[String], id: &str) -> bool {
    list.is_empty() || list.iter().any(|known| known == id)
}

pub trait CommandWorld {
    fn seed(&self) -> i64;
    fn game_rules(&self) -> &GameRules;
    fn set_game_rule(&mut self, rule: &'static Rule, value: Value);
    fn difficulty(&self) -> Difficulty;
    fn set_difficulty(&mut self, difficulty: Difficulty);
    fn clock(&self) -> Clock;
    fn set_time(&mut self, total: i64);
    fn set_time_paused(&mut self, paused: bool);
    fn set_weather(&mut self, weather: Weather, duration: i32);
    fn kill(&mut self, player: PlayerId) -> bool;
    fn game_mode(&self, player: PlayerId) -> Option<GameMode>;
    fn set_game_mode(&mut self, player: PlayerId, mode: GameMode) -> bool;
    fn pose(&self, player: PlayerId) -> Option<Pose>;
    fn max_stack(&self, item: &str) -> u32;
    fn give(&mut self, player: PlayerId, item: &ItemArg, count: u32) -> bool;
    fn teleport(&mut self, player: PlayerId, position: [f64; 3]) -> bool;
    fn effect_give(
        &mut self,
        player: PlayerId,
        effect: &str,
        ticks: Option<u32>,
        amplifier: u8,
    ) -> bool;
    fn effect_clear(&mut self, player: PlayerId, effect: Option<&str>) -> bool;
    fn summon(
        &mut self,
        source: PlayerId,
        kind: &str,
        position: [f64; 3],
        nbt: Option<&str>,
    ) -> Result<(), Text>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub id: PlayerId,
    pub name: String,
    pub uuid: String,
}

#[derive(Clone, Debug)]
pub struct CommandServer {
    players: Vec<Profile>,
    host: Option<PlayerId>,
    ops: OpList,
    ops_path: Option<PathBuf>,
    warnings: Vec<String>,
    random: u64,
    registries: CommandRegistries,
}

impl Default for CommandServer {
    fn default() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Self {
            players: Vec::new(),
            host: None,
            ops: OpList::default(),
            ops_path: None,
            warnings: Vec::new(),
            random: nanos | 1,
            registries: CommandRegistries::default(),
        }
    }
}

pub struct Ctx<'a> {
    pub server: &'a mut CommandServer,
    pub world: &'a mut dyn CommandWorld,
    pub source: PlayerId,
    pub level: u8,
    pub input: &'a str,
    pub args: Vec<(&'static str, Arg)>,
    pub literals: Vec<&'static str>,
    pub out: Vec<Outgoing>,
}

pub(crate) fn system(to: PlayerId, text: Text) -> Outgoing {
    Outgoing {
        to: Recipient::Player(to),
        message: Clientbound::System {
            text,
            overlay: false,
        },
    }
}

pub fn error_line(to: PlayerId, text: Text) -> Outgoing {
    system(to, Text::literal("").color(RED).append(text))
}

impl Ctx<'_> {
    pub fn arg(&self, name: &str) -> Option<&Arg> {
        self.args
            .iter()
            .find(|(arg, _)| *arg == name)
            .map(|(_, value)| value)
    }

    pub fn success(&mut self, text: Text, broadcast: bool) {
        let out = self
            .server
            .feedback(self.source, text, broadcast, self.world.game_rules());
        self.out.extend(out);
    }

    pub fn pose(&self) -> Pose {
        self.world.pose(self.source).unwrap_or(Pose::ORIGIN)
    }

    pub fn display_name(&self, player: PlayerId) -> Text {
        Text::literal(self.server.name(player).unwrap_or(""))
    }

    pub fn targets(&mut self, name: &str) -> Result<Vec<PlayerId>, CommandError> {
        let selector = match self.arg(name) {
            Some(Arg::Players(selector)) => selector.clone(),
            _ => Selector::Me,
        };
        let players = &self.server.players;
        let found: Vec<PlayerId> = match selector {
            Selector::Me | Selector::Nearest => players
                .iter()
                .filter(|player| player.id == self.source)
                .map(|player| player.id)
                .collect(),
            Selector::All => players.iter().map(|player| player.id).collect(),
            Selector::Random => {
                let count = players.len() as u64;
                if count == 0 {
                    Vec::new()
                } else {
                    let index = (self.server.next_random() % count) as usize;
                    vec![self.server.players[index].id]
                }
            }
            Selector::Name(name) => players
                .iter()
                .filter(|player| player.name.eq_ignore_ascii_case(&name))
                .map(|player| player.id)
                .collect(),
        };
        if found.is_empty() {
            Err(CommandError::new(
                "argument.entity.notfound.player",
                Vec::new(),
            ))
        } else {
            Ok(found)
        }
    }

    pub fn random_between(&mut self, min: i32, max: i32) -> i32 {
        let span = (i64::from(max) - i64::from(min) + 1).max(1) as u64;
        min + (self.server.next_random() % span) as i32
    }
}

impl CommandServer {
    pub fn new(host: PlayerId) -> Self {
        Self {
            host: Some(host),
            ..Self::default()
        }
    }

    pub fn join(&mut self, id: PlayerId, name: impl Into<String>) {
        let name = name.into();
        let uuid = offline_uuid(&name);
        match self.players.iter_mut().find(|player| player.id == id) {
            Some(known) => {
                known.name = name;
                known.uuid = uuid;
            }
            None => self.players.push(Profile { id, name, uuid }),
        }
    }

    pub fn leave(&mut self, id: PlayerId) {
        self.players.retain(|player| player.id != id);
    }

    pub fn profile(&self, id: PlayerId) -> Option<&Profile> {
        self.players.iter().find(|player| player.id == id)
    }

    pub fn name(&self, id: PlayerId) -> Option<&str> {
        self.profile(id).map(|player| player.name.as_str())
    }

    pub fn is_op(&self, id: PlayerId) -> bool {
        self.host == Some(id)
            || self
                .profile(id)
                .is_some_and(|player| self.ops.level_of(&player.uuid).is_some())
    }

    pub fn level(&self, id: PlayerId) -> u8 {
        if self.host == Some(id) {
            return OWNERS;
        }
        self.profile(id)
            .and_then(|player| self.ops.level_of(&player.uuid))
            .unwrap_or(ALL)
    }

    pub fn ops(&self) -> &OpList {
        &self.ops
    }

    pub fn open_ops(&mut self, path: &Path) -> Result<(), String> {
        self.ops_path = Some(path.to_owned());
        self.ops = OpList::load(path)?;
        Ok(())
    }

    pub fn op(&mut self, id: PlayerId) -> bool {
        if self.is_op(id) {
            return false;
        }
        let Some(player) = self.profile(id).cloned() else {
            return false;
        };
        self.ops.add(&player.uuid, &player.name, OWNERS);
        self.save_ops();
        true
    }

    pub fn deop(&mut self, id: PlayerId) -> bool {
        if !self.is_op(id) {
            return false;
        }
        if let Some(uuid) = self.profile(id).map(|player| player.uuid.clone())
            && self.ops.remove(&uuid)
        {
            self.save_ops();
        }
        true
    }

    fn save_ops(&mut self) {
        if let Some(path) = &self.ops_path
            && let Err(error) = self.ops.save(path)
        {
            self.warnings
                .push(format!("Minecraft ops.json not written: {error}"));
        }
    }

    pub fn registries(&self) -> &CommandRegistries {
        &self.registries
    }

    pub fn registries_mut(&mut self) -> &mut CommandRegistries {
        &mut self.registries
    }

    pub fn feedback(
        &self,
        source: PlayerId,
        text: Text,
        broadcast: bool,
        rules: &GameRules,
    ) -> Vec<Outgoing> {
        let mut out = Vec::new();
        if rules.bool(SEND_COMMAND_FEEDBACK) {
            out.push(system(source, text.clone()));
        }
        if broadcast && rules.bool(LOG_ADMIN_COMMANDS) {
            let name = self.name(source).unwrap_or("").to_owned();
            let admin = Text::translate("chat.type.admin", vec![Text::literal(name), text])
                .color(GRAY)
                .italic();
            out.extend(
                self.players
                    .iter()
                    .map(|player| player.id)
                    .filter(|&id| id != source && self.is_op(id))
                    .map(|id| system(id, admin.clone())),
            );
        }
        out
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    fn next_random(&mut self) -> u64 {
        let mut x = self.random;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.random = x;
        x
    }

    pub fn handle(
        &mut self,
        from: PlayerId,
        message: Serverbound,
        world: &mut dyn CommandWorld,
    ) -> Vec<Outgoing> {
        let Some(name) = self.name(from).map(str::to_owned) else {
            return Vec::new();
        };
        match message {
            Serverbound::Chat(line) => {
                let filtered: String = line.chars().filter(|c| allowed_in_chat(*c)).collect();
                let message = normalize_chat(&filtered);
                if message.is_empty() {
                    return Vec::new();
                }
                vec![Outgoing {
                    to: Recipient::All,
                    message: Clientbound::Player {
                        sender: from,
                        name,
                        message,
                    },
                }]
            }
            Serverbound::Command(command) => self.perform(from, &command, world),
        }
    }

    fn perform(
        &mut self,
        from: PlayerId,
        command: &str,
        world: &mut dyn CommandWorld,
    ) -> Vec<Outgoing> {
        let level = self.level(from);
        let parsed = parse(crate::commands::tree(), command, level);
        let exec = match runnable(&parsed, command) {
            Ok(exec) => exec,
            Err(error) => return report(from, error, command),
        };
        let mut ctx = Ctx {
            server: self,
            world,
            source: from,
            level,
            input: command,
            args: parsed.args,
            literals: parsed.literals,
            out: Vec::new(),
        };
        let result = exec(&mut ctx);
        let mut out = ctx.out;
        if let Err(error) = result {
            out.extend(report(from, error, command));
        }
        out
    }
}

fn report(to: PlayerId, error: CommandError, input: &str) -> Vec<Outgoing> {
    match error.cursor {
        Some(cursor) => failure(to, error.text, input, cursor),
        None => vec![error_line(to, error.text)],
    }
}

pub fn failure(to: PlayerId, error: Text, input: &str, cursor: usize) -> Vec<Outgoing> {
    let mut cursor = cursor.min(input.len());
    while !input.is_char_boundary(cursor) {
        cursor -= 1;
    }
    let before: String = {
        let head = &input[..cursor];
        let skip = head.chars().count().saturating_sub(10);
        head.chars().skip(skip).collect()
    };
    let mut context = Text::literal("").color(GRAY);
    if input[..cursor].chars().count() > 10 {
        context = context.append(Text::literal("..."));
    }
    context = context.append(Text::literal(before));
    if cursor < input.len() {
        context = context.append(Text::literal(&input[cursor..]).color(RED).underlined());
    }
    context = context.append(
        Text::translate("command.context.here", Vec::new())
            .color(RED)
            .italic(),
    );
    vec![
        system(to, Text::literal("").color(RED).append(error)),
        system(to, context),
    ]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const HOST: PlayerId = PlayerId(0);
    pub(crate) const FRIEND: PlayerId = PlayerId(1);

    #[derive(Debug, Default)]
    pub(crate) struct FakeWorld {
        pub(crate) rules: GameRules,
        pub(crate) difficulty: Option<Difficulty>,
        pub(crate) clock: Clock,
        pub(crate) weather: Vec<(Weather, i32)>,
        pub(crate) killed: Vec<PlayerId>,
        pub(crate) modes: Vec<(PlayerId, GameMode)>,
        pub(crate) mode_changes: Vec<(PlayerId, GameMode, GameMode)>,
        pub(crate) poses: Vec<(PlayerId, Pose)>,
        pub(crate) given: Vec<(PlayerId, ItemArg, u32)>,
        pub(crate) teleported: Vec<(PlayerId, [f64; 3])>,
        pub(crate) effects: Vec<(PlayerId, String, Option<u32>, u8)>,
        pub(crate) cleared: Vec<(PlayerId, Option<String>)>,
        pub(crate) summoned: Vec<(String, [f64; 3], Option<String>)>,
    }

    impl CommandWorld for FakeWorld {
        fn seed(&self) -> i64 {
            -4_172_144_997_902_289_642
        }

        fn game_rules(&self) -> &GameRules {
            &self.rules
        }

        fn set_game_rule(&mut self, rule: &'static Rule, value: Value) {
            self.rules.set(rule.id, &value.to_string()).unwrap();
        }

        fn difficulty(&self) -> Difficulty {
            self.difficulty.unwrap_or(Difficulty::Normal)
        }

        fn set_difficulty(&mut self, difficulty: Difficulty) {
            self.difficulty = Some(difficulty);
        }

        fn clock(&self) -> Clock {
            self.clock
        }

        fn set_time(&mut self, total: i64) {
            self.clock.total = total;
        }

        fn set_time_paused(&mut self, paused: bool) {
            self.clock.paused = paused;
        }

        fn set_weather(&mut self, weather: Weather, duration: i32) {
            self.weather.push((weather, duration));
        }

        fn kill(&mut self, player: PlayerId) -> bool {
            self.killed.push(player);
            true
        }

        fn game_mode(&self, player: PlayerId) -> Option<GameMode> {
            let set = self.modes.iter().rev().find(|(id, _)| *id == player);
            Some(set.map_or(GameMode::Survival, |(_, mode)| *mode))
        }

        fn set_game_mode(&mut self, player: PlayerId, mode: GameMode) -> bool {
            let old = self.game_mode(player).unwrap_or(GameMode::Survival);
            if old == mode {
                return false;
            }
            self.modes.push((player, mode));
            self.mode_changes.push((player, old, mode));
            true
        }

        fn pose(&self, player: PlayerId) -> Option<Pose> {
            self.poses
                .iter()
                .rev()
                .find(|(id, _)| *id == player)
                .map(|(_, pose)| *pose)
        }

        fn max_stack(&self, item: &str) -> u32 {
            if item.ends_with("_sword") { 1 } else { 64 }
        }

        fn give(&mut self, player: PlayerId, item: &ItemArg, count: u32) -> bool {
            self.given.push((player, item.clone(), count));
            true
        }

        fn teleport(&mut self, player: PlayerId, position: [f64; 3]) -> bool {
            self.teleported.push((player, position));
            let pose = Pose {
                feet: position,
                ..self.pose(player).unwrap_or(Pose::ORIGIN)
            };
            self.poses.push((player, pose));
            true
        }

        fn effect_give(
            &mut self,
            player: PlayerId,
            effect: &str,
            ticks: Option<u32>,
            amplifier: u8,
        ) -> bool {
            if effect == "minecraft:speed" {
                return false;
            }
            self.effects
                .push((player, effect.to_owned(), ticks, amplifier));
            true
        }

        fn effect_clear(&mut self, player: PlayerId, effect: Option<&str>) -> bool {
            let before = self.effects.len();
            self.effects.retain(|(id, kind, _, _)| {
                *id != player || effect.is_some_and(|effect| effect != kind)
            });
            self.cleared.push((player, effect.map(str::to_owned)));
            self.effects.len() < before
        }

        fn summon(
            &mut self,
            _source: PlayerId,
            kind: &str,
            position: [f64; 3],
            nbt: Option<&str>,
        ) -> Result<(), Text> {
            if nbt == Some("{bad}") {
                return Err(Text::translate("argument.nbt.expected.value", Vec::new()));
            }
            self.summoned
                .push((kind.to_owned(), position, nbt.map(str::to_owned)));
            Ok(())
        }
    }

    pub(crate) fn lang(key: &str) -> Option<String> {
        let text = match key {
            "chat.type.text" => "<%s> %s",
            "chat.type.admin" => "[%s: %s]",
            "chat.square_brackets" => "[%s]",
            "command.unknown.command" => "Unknown or incomplete command. See below for error",
            "command.unknown.argument" => "Incorrect argument for command",
            "command.context.here" => "<--[HERE]",
            "commands.seed.success" => "Seed: %s",
            "commands.help.failed" => "Unknown command or insufficient permissions",
            "commands.op.success" => "Made %s a server operator",
            "commands.op.failed" => "Nothing changed. The player is already an operator",
            "commands.deop.success" => "Made %s no longer a server operator",
            "commands.deop.failed" => "Nothing changed. The player is not an operator",
            "commands.difficulty.query" => "The difficulty is %s",
            "commands.difficulty.success" => "The difficulty has been set to %s",
            "commands.difficulty.failure" => {
                "The difficulty did not change; it is already set to %s"
            }
            "commands.gamerule.query" => "Game rule %s is currently set to %s",
            "commands.gamerule.set" => "Game rule %s is now set to %s",
            "commands.gamerule.not_set" => "Game rule %s is already set to %s",
            "commands.kill.success.single" => "Killed %s",
            "commands.kill.success.multiple" => "Killed %s entities",
            "commands.time.set.absolute" => "Set %s to %s tick(s)",
            "commands.time.set.time_marker" => "Set %s to time marker %s",
            "commands.time.set.already_at_time" => "Clock %s is already set to %s tick(s)",
            "commands.time.set.already_at_time_marker" => "Clock %s is already at time marker %s",
            "commands.time.no_time_marker_found" => "Time marker %s does not exist for clock %s",
            "commands.time.pause" => "Paused clock %s",
            "commands.time.resume" => "Resumed clock %s",
            "commands.time.pause.already_paused" => "Clock %s is already paused",
            "commands.time.pause.already_running" => "Clock %s is already running",
            "commands.time.query.absolute" => "Clock %s is at %s tick(s)",
            "commands.time.query.gametime" => "The game time is %s tick(s)",
            "commands.weather.set.clear" => "Set the weather to clear",
            "commands.weather.set.rain" => "Set the weather to rain",
            "commands.weather.set.thunder" => "Set the weather to rain and thunder",
            "options.difficulty.hard" => "Hard",
            "options.difficulty.normal" => "Normal",
            "options.difficulty.peaceful" => "Peaceful",
            "parsing.bool.invalid" => "Invalid boolean: expected 'true' or 'false' but found '%s'",
            "argument.entity.notfound.player" => "No player was found",
            "argument.gamemode.invalid" => "Unknown game mode: %s",
            "commands.gamemode.success.self" => "Set own game mode to %s",
            "commands.gamemode.success.other" => "Set %s's game mode to %s",
            "gameMode.changed" => "Your game mode has been updated to %s",
            "gameMode.survival" => "Survival Mode",
            "gameMode.creative" => "Creative Mode",
            "gameMode.adventure" => "Adventure Mode",
            "gameMode.spectator" => "Spectator Mode",
            "commands.give.success.single" => "Gave %s %s to %s",
            "commands.give.success.multiple" => "Gave %s %s to %s players",
            "commands.give.failed.toomanyitems" => "Can't give more than %s of %s",
            "argument.item.id.invalid" => "Unknown item '%s'",
            "arguments.item.component.malformed" => "Malformed '%s' component: '%s'",
            "item.minecraft.diamond_sword" => "Diamond Sword",
            "block.minecraft.stone" => "Stone",
            "commands.teleport.success.location.single" => "Teleported %s to %s, %s, %s",
            "commands.teleport.success.location.multiple" => "Teleported %s entities to %s, %s, %s",
            "commands.teleport.success.entity.single" => "Teleported %s to %s",
            "commands.teleport.invalidPosition" => "Invalid position for teleport",
            "commands.effect.give.success.single" => "Applied effect %s to %s",
            "commands.effect.give.failed" => {
                "Unable to apply this effect (target is either immune to effects, or has something stronger)"
            }
            "commands.effect.clear.everything.success.single" => "Removed every effect from %s",
            "commands.effect.clear.everything.failed" => "Target has no effects to remove",
            "commands.effect.clear.specific.success.single" => "Removed effect %s from %s",
            "commands.effect.clear.specific.failed" => "Target doesn't have the requested effect",
            "argument.resource.not_found" => "Can't find element '%s' of type '%s'",
            "effect.minecraft.regeneration" => "Regeneration",
            "effect.minecraft.speed" => "Speed",
            "commands.summon.failed" => "Unable to summon entity",
            "commands.summon.invalidPosition" => "Invalid position for summon",
            "argument.nbt.expected.value" => "Expected value",
            "argument.pos3d.incomplete" => "Incomplete (expected 3 coordinates)",
            _ => return None,
        };
        Some(text.to_owned())
    }

    pub(crate) fn server() -> CommandServer {
        let mut server = CommandServer::new(HOST);
        server.join(HOST, "Steve");
        server
    }

    pub(crate) fn run(
        server: &mut CommandServer,
        world: &mut FakeWorld,
        from: PlayerId,
        command: &str,
    ) -> Vec<(Recipient, String, u32)> {
        server
            .handle(from, Serverbound::Command(command.into()), world)
            .into_iter()
            .map(|out| {
                let resolved = out.message.chat_text().unwrap().resolve(&lang);
                (out.to, resolved.text, resolved.color)
            })
            .collect()
    }

    pub(crate) fn texts(lines: &[(Recipient, String, u32)]) -> Vec<&str> {
        lines.iter().map(|(_, text, _)| text.as_str()).collect()
    }

    #[test]
    fn chat_broadcasts_the_sender_name() {
        let mut server = server();
        let mut world = FakeWorld::default();
        let out = server.handle(
            HOST,
            Serverbound::Chat("  hello\u{a7}  there ".into()),
            &mut world,
        );
        assert_eq!(
            out,
            vec![Outgoing {
                to: Recipient::All,
                message: Clientbound::Player {
                    sender: HOST,
                    name: "Steve".into(),
                    message: "hello there".into(),
                },
            }]
        );
        let text = out[0].message.chat_text().unwrap().resolve(&lang);
        assert_eq!(text.text, "<Steve> hello there");
        assert!(
            server
                .handle(HOST, Serverbound::Chat("   ".into()), &mut world)
                .is_empty()
        );
        assert!(
            server
                .handle(PlayerId(7), Serverbound::Chat("who".into()), &mut world)
                .is_empty()
        );
        server.join(HOST, "Alex");
        assert_eq!(server.name(HOST), Some("Alex"));
        server.leave(HOST);
        assert_eq!(server.name(HOST), None);
    }

    #[test]
    fn an_unknown_command_reports_context_with_here_marker() {
        let mut server = server();
        let mut world = FakeWorld::default();
        let out = server.handle(HOST, Serverbound::Command("frob".into()), &mut world);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|o| o.to == Recipient::Player(HOST)));
        let lines: Vec<_> = out
            .iter()
            .map(|o| o.message.chat_text().unwrap().resolve(&lang))
            .collect();
        assert_eq!(
            lines[0].text,
            "Unknown or incomplete command. See below for error"
        );
        assert_eq!(lines[0].color, RED);
        assert_eq!(lines[1].text, "frob<--[HERE]");
        assert_eq!(lines[1].color, GRAY);
        assert_eq!(lines[1].spans, vec![(0..13, RED)]);
        let incomplete = run(&mut server, &mut world, HOST, "weather");
        assert_eq!(texts(&incomplete)[1], "weather<--[HERE]");
    }

    #[test]
    fn trailing_data_is_incorrect_argument() {
        let mut server = server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "seed now");
        assert_eq!(
            texts(&out),
            vec!["Incorrect argument for command", "seed now<--[HERE]"]
        );
        let out = run(&mut server, &mut world, HOST, "weather snow");
        assert_eq!(
            texts(&out),
            vec!["Incorrect argument for command", "weather snow<--[HERE]"]
        );
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "gamerule keep_inventory maybe",
        );
        assert_eq!(
            texts(&out),
            vec![
                "Invalid boolean: expected 'true' or 'false' but found 'maybe'",
                "...inventory maybe<--[HERE]",
            ]
        );
    }

    #[test]
    fn the_error_context_shows_ten_chars_before_the_cursor() {
        let out = failure(
            HOST,
            Text::literal("bad"),
            "gamerule keep_inventory maybe",
            24,
        );
        let context = out[1].message.chat_text().unwrap().resolve(&lang);
        assert_eq!(context.text, "...inventory maybe<--[HERE]");
        assert_eq!(context.spans, vec![(13..27, RED)]);
        let at_end = failure(HOST, Text::literal("bad"), "seed", 4);
        let context = at_end[1].message.chat_text().unwrap().resolve(&lang);
        assert_eq!(context.text, "seed<--[HERE]");
        assert_eq!(context.spans, vec![(4..13, RED)]);
    }

    #[test]
    fn the_host_is_owner_even_without_an_ops_entry() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        assert_eq!(server.level(HOST), OWNERS);
        assert!(server.is_op(HOST));
        assert!(server.ops().0.is_empty());
        assert_eq!(server.level(FRIEND), ALL);
        assert!(!server.is_op(FRIEND));
        assert!(!server.op(HOST));
        assert!(server.deop(HOST));
        assert_eq!(server.level(HOST), OWNERS);
        assert!(server.op(FRIEND));
        assert_eq!(server.level(FRIEND), OWNERS);
        assert_eq!(server.ops().0[0].name, "Alex");
        assert_eq!(server.ops().0[0].uuid, offline_uuid("Alex"));
        assert!(server.deop(FRIEND));
        assert!(!server.deop(FRIEND));
        assert_eq!(server.level(FRIEND), ALL);
        assert_eq!(CommandServer::default().level(HOST), ALL);
    }

    #[test]
    fn op_needs_level_three_and_gamerule_level_two() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld::default();
        for command in ["op Steve", "gamerule keep_inventory", "kill", "time set 0"] {
            let out = run(&mut server, &mut world, FRIEND, command);
            assert_eq!(
                texts(&out)[0],
                "Unknown or incomplete command. See below for error",
                "{command}"
            );
        }
        assert!(world.killed.is_empty());
        server.ops.add(&offline_uuid("Alex"), "Alex", 2);
        let out = run(&mut server, &mut world, FRIEND, "gamerule keep_inventory");
        assert_eq!(
            texts(&out),
            vec!["Game rule keep_inventory is currently set to false"]
        );
        let out = run(&mut server, &mut world, FRIEND, "op Alex");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
        server.ops.add(&offline_uuid("Alex"), "Alex", 3);
        let out = run(&mut server, &mut world, FRIEND, "op Steve");
        assert_eq!(
            texts(&out),
            vec!["Nothing changed. The player is already an operator"]
        );
    }

    #[test]
    fn ops_json_is_written_when_an_operator_is_added_or_removed() {
        let dir = std::env::temp_dir().join(format!(
            "minecraft_commands-server-ops-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(crate::permissions::OPS_FILE);
        let mut server = server();
        server.open_ops(&path).unwrap();
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "op alex");
        assert_eq!(
            texts(&out),
            vec![
                "Made Alex a server operator",
                "[Steve: Made Alex a server operator]"
            ]
        );
        assert_eq!(out[1].0, Recipient::Player(FRIEND));
        let saved = OpList::load(&path).unwrap();
        assert_eq!(saved.level_of(&offline_uuid("Alex")), Some(OWNERS));
        let mut reopened = CommandServer::new(HOST);
        reopened.open_ops(&path).unwrap();
        reopened.join(FRIEND, "Alex");
        assert_eq!(reopened.level(FRIEND), OWNERS);
        let out = run(&mut server, &mut world, HOST, "deop Alex");
        assert_eq!(texts(&out), vec!["Made Alex no longer a server operator"]);
        assert!(OpList::load(&path).unwrap().0.is_empty());
        let out = run(&mut server, &mut world, HOST, "deop Alex");
        assert_eq!(
            texts(&out),
            vec!["Nothing changed. The player is not an operator"]
        );
        let out = run(&mut server, &mut world, HOST, "op Nobody");
        assert_eq!(texts(&out), vec!["No player was found"]);
        assert!(server.take_warnings().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn admins_see_other_operators_commands_unless_logging_is_off() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        server.ops.add(&offline_uuid("Alex"), "Alex", 2);
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, FRIEND, "weather rain 100");
        assert_eq!(
            out,
            vec![
                (
                    Recipient::Player(FRIEND),
                    "Set the weather to rain".to_owned(),
                    crate::text::WHITE
                ),
                (
                    Recipient::Player(HOST),
                    "[Alex: Set the weather to rain]".to_owned(),
                    GRAY
                ),
            ]
        );
        let out = run(&mut server, &mut world, FRIEND, "seed");
        assert_eq!(out.len(), 1);
        world.rules.set(LOG_ADMIN_COMMANDS, "false").unwrap();
        let out = run(&mut server, &mut world, FRIEND, "weather clear");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, Recipient::Player(FRIEND));
    }

    #[test]
    fn feedback_is_silent_when_send_command_feedback_is_off() {
        let mut server = server();
        let mut world = FakeWorld::default();
        world.rules.set(SEND_COMMAND_FEEDBACK, "false").unwrap();
        assert!(run(&mut server, &mut world, HOST, "weather thunder").is_empty());
        assert_eq!(world.weather.len(), 1);
        let failed = run(&mut server, &mut world, HOST, "difficulty normal");
        assert_eq!(
            texts(&failed),
            vec!["The difficulty did not change; it is already set to Normal"]
        );
        assert_eq!(failed[0].2, RED);
    }
}
