use std::sync::OnceLock;

use minecraftoss_player::Difficulty;
use minecraftoss_world::game_rules::Value;

use crate::args::{Arg, ArgKind, Registry, TICKS_PER_DAY, marker_ticks};
use crate::permissions::{ADMINS, GAMEMASTERS};
use crate::players::game_mode_key;
use crate::protocol::PlayerId;
use crate::reader::{CommandError, literal};
use crate::server::{Ctx, SEND_COMMAND_FEEDBACK, Weather, registered, system};
use crate::text::{GREEN, Text};
use crate::tree::{Node, Redirect, parse, smart_usage, smart_usages, sorted};

pub const CLOCK: &str = "minecraft:overworld";
const RAIN_DELAY: (i32, i32) = (12_000, 180_000);
const RAIN_DURATION: (i32, i32) = (12_000, 24_000);
const THUNDER_DURATION: (i32, i32) = (3600, 15_600);
pub const MAX_ALLOWED_ITEMSTACKS: u32 = 100;
pub const INSTANT_EFFECTS: [&str; 3] = [
    "minecraft:instant_damage",
    "minecraft:instant_health",
    "minecraft:saturation",
];
const MOB_EFFECT_REGISTRY: &str = "minecraft:mob_effect";

pub fn tree() -> &'static [Node] {
    static TREE: OnceLock<Vec<Node>> = OnceLock::new();
    TREE.get_or_init(build)
}

fn teleport_node(name: &'static str) -> Node {
    let players = ArgKind::Players { single: false };
    let destination = ArgKind::Players { single: true };
    Node::literal(name)
        .requires(GAMEMASTERS)
        .then(Node::argument("location", ArgKind::Vec3).executes(teleport_to_location))
        .then(Node::argument("destination", destination).executes(teleport_to_player))
        .then(
            Node::argument("targets", players)
                .then(Node::argument("location", ArgKind::Vec3).executes(teleport_to_location))
                .then(Node::argument("destination", destination).executes(teleport_to_player)),
        )
}

fn effect_node() -> Node {
    let players = ArgKind::Players { single: false };
    let effect = ArgKind::Resource(Registry::MobEffect);
    let amplifier = || {
        Node::argument("amplifier", ArgKind::Int { min: 0, max: 255 })
            .executes(give_effect)
            .then(Node::argument("hideParticles", ArgKind::Bool).executes(give_effect))
    };
    Node::literal("effect")
        .requires(GAMEMASTERS)
        .then(
            Node::literal("clear").executes(clear_effects).then(
                Node::argument("targets", players)
                    .executes(clear_effects)
                    .then(Node::argument("effect", effect).executes(clear_effects)),
            ),
        )
        .then(
            Node::literal("give").then(
                Node::argument("targets", players).then(
                    Node::argument("effect", effect)
                        .executes(give_effect)
                        .then(
                            Node::argument(
                                "seconds",
                                ArgKind::Int {
                                    min: 1,
                                    max: 1_000_000,
                                },
                            )
                            .executes(give_effect)
                            .then(amplifier()),
                        )
                        .then(
                            Node::literal("infinite")
                                .executes(give_effect)
                                .then(amplifier()),
                        ),
                ),
            ),
        )
}

fn build() -> Vec<Node> {
    let players = ArgKind::Players { single: false };
    let weather = |name: &'static str| {
        Node::literal(name)
            .executes(set_weather)
            .then(Node::argument("duration", ArgKind::Time { min: 1 }).executes(set_weather))
    };
    sorted(vec![
        Node::literal("help")
            .executes(help)
            .then(Node::argument("command", ArgKind::Greedy).executes(help_command)),
        Node::literal("seed").executes(seed),
        Node::literal("op")
            .requires(ADMINS)
            .then(Node::argument("targets", players).executes(op)),
        Node::literal("deop")
            .requires(ADMINS)
            .then(Node::argument("targets", players).executes(deop)),
        ["peaceful", "easy", "normal", "hard"].into_iter().fold(
            Node::literal("difficulty")
                .requires(GAMEMASTERS)
                .executes(query_difficulty),
            |node, name| node.then(Node::literal(name).executes(set_difficulty)),
        ),
        Node::literal("gamemode").requires(GAMEMASTERS).then(
            Node::argument("gamemode", ArgKind::GameMode)
                .executes(game_mode)
                .then(Node::argument("target", players).executes(game_mode)),
        ),
        Node::literal("gamerule").requires(GAMEMASTERS).then(
            Node::argument("rule", ArgKind::Rule)
                .executes(query_rule)
                .then(Node::argument("value", ArgKind::RuleValue).executes(set_rule)),
        ),
        Node::literal("time")
            .requires(GAMEMASTERS)
            .then(
                Node::literal("set")
                    .then(Node::argument("time", ArgKind::Time { min: 0 }).executes(set_time))
                    .then(Node::argument("timemarker", ArgKind::TimeMarker).executes(set_marker)),
            )
            .then(
                Node::literal("add").then(
                    Node::argument("time", ArgKind::Time { min: i32::MIN }).executes(add_time),
                ),
            )
            .then(Node::literal("pause").executes(pause_time))
            .then(Node::literal("resume").executes(resume_time))
            .then(
                Node::literal("query")
                    .then(Node::literal("time").executes(query_time))
                    .then(Node::literal("gametime").executes(query_game_time)),
            ),
        Node::literal("weather")
            .requires(GAMEMASTERS)
            .then(weather("clear"))
            .then(weather("rain"))
            .then(weather("thunder")),
        Node::literal("kill")
            .requires(GAMEMASTERS)
            .executes(kill)
            .then(Node::argument("targets", players).executes(kill)),
        Node::literal("give").requires(GAMEMASTERS).then(
            Node::argument("targets", players).then(
                Node::argument("item", ArgKind::Item).executes(give).then(
                    Node::argument(
                        "count",
                        ArgKind::Int {
                            min: 1,
                            max: i32::MAX,
                        },
                    )
                    .executes(give),
                ),
            ),
        ),
        teleport_node("teleport"),
        Node::literal("tp")
            .requires(GAMEMASTERS)
            .redirect(Redirect::Command("teleport")),
        effect_node(),
        Node::literal("summon").requires(GAMEMASTERS).then(
            Node::argument("entity", ArgKind::Resource(Registry::EntityType))
                .executes(summon)
                .then(
                    Node::argument("pos", ArgKind::Vec3)
                        .executes(summon)
                        .then(Node::argument("nbt", ArgKind::Nbt).executes(summon)),
                ),
        ),
        crate::clear::node(),
        crate::experience::node(),
        crate::experience::alias(),
        crate::enchant::node(),
        crate::blocks::setblock_node(),
        crate::blocks::fill_node(),
        crate::spawnpoint::node(),
        crate::locate::node(),
        crate::execute::node(),
        crate::kit::node(),
    ])
}

fn wrap_time(ticks: i64) -> i32 {
    (ticks % i64::from(i32::MAX)) as i32
}

fn help(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let usages = smart_usages(tree(), false, ctx.level);
    for usage in &usages {
        ctx.success(Text::literal(format!("/{usage}")), false);
    }
    Ok(usages.len() as i32)
}

fn help_command(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(Arg::Text(command)) = ctx.arg("command").cloned() else {
        return Err(CommandError::new("commands.help.failed", Vec::new()));
    };
    let parsed = parse(tree(), &command, ctx.level);
    let Some(last) = parsed.nodes[..parsed.stage_nodes(0).end].last() else {
        return Err(CommandError::new("commands.help.failed", Vec::new()));
    };
    let usages = smart_usages(&last.children, last.exec.is_some(), ctx.level);
    for usage in &usages {
        ctx.success(Text::literal(format!("/{command} {usage}")), false);
    }
    Ok(usages.len() as i32)
}

fn seed(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let seed = ctx.world.seed();
    let bracketed = Text::translate(
        "chat.square_brackets",
        vec![Text::literal(seed.to_string()).color(GREEN)],
    );
    ctx.success(
        Text::translate("commands.seed.success", vec![bracketed]),
        false,
    );
    Ok(seed as i32)
}

fn op(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let mut count = 0;
    for player in ctx.targets("targets")? {
        if ctx.server.op(player) {
            let name = ctx.server.name(player).unwrap_or("").to_owned();
            ctx.success(
                Text::translate("commands.op.success", vec![Text::literal(name)]),
                true,
            );
            count += 1;
        }
    }
    if count == 0 {
        return Err(CommandError::new("commands.op.failed", Vec::new()));
    }
    Ok(count)
}

fn deop(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let mut count = 0;
    for player in ctx.targets("targets")? {
        if ctx.server.deop(player) {
            let name = ctx.server.name(player).unwrap_or("").to_owned();
            ctx.success(
                Text::translate("commands.deop.success", vec![Text::literal(name)]),
                true,
            );
            count += 1;
        }
    }
    if count == 0 {
        return Err(CommandError::new("commands.deop.failed", Vec::new()));
    }
    Ok(count)
}

pub fn difficulty_name(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Peaceful => "peaceful",
        Difficulty::Easy => "easy",
        Difficulty::Normal => "normal",
        Difficulty::Hard => "hard",
    }
}

pub fn difficulty_id(difficulty: Difficulty) -> i32 {
    match difficulty {
        Difficulty::Peaceful => 0,
        Difficulty::Easy => 1,
        Difficulty::Normal => 2,
        Difficulty::Hard => 3,
    }
}

fn difficulty_text(difficulty: Difficulty) -> Text {
    Text::translate(
        format!("options.difficulty.{}", difficulty_name(difficulty)),
        Vec::new(),
    )
}

fn query_difficulty(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let difficulty = ctx.world.difficulty();
    ctx.success(
        Text::translate(
            "commands.difficulty.query",
            vec![difficulty_text(difficulty)],
        ),
        false,
    );
    Ok(difficulty_id(difficulty))
}

fn set_difficulty(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(difficulty) = ctx.literals.last().and_then(|name| Difficulty::parse(name)) else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    if ctx.world.difficulty() == difficulty {
        return Err(CommandError::new(
            "commands.difficulty.failure",
            vec![difficulty_text(difficulty)],
        ));
    }
    ctx.world.set_difficulty(difficulty);
    ctx.success(
        Text::translate(
            "commands.difficulty.success",
            vec![difficulty_text(difficulty)],
        ),
        true,
    );
    Ok(0)
}

fn command_result(value: Value) -> i32 {
    match value {
        Value::Bool(on) => i32::from(on),
        Value::Int(value) => value,
    }
}

fn query_rule(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(Arg::Rule(rule)) = ctx.arg("rule").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let value = ctx.world.game_rules().get(rule.id).unwrap_or(rule.default);
    ctx.success(
        Text::translate(
            "commands.gamerule.query",
            vec![literal(rule.id), literal(value)],
        ),
        false,
    );
    Ok(command_result(value))
}

fn set_rule(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let (Some(Arg::Rule(rule)), Some(Arg::RuleValue(value))) =
        (ctx.arg("rule").cloned(), ctx.arg("value").cloned())
    else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    if ctx.world.game_rules().get(rule.id) == Some(value) {
        return Err(CommandError::new(
            "commands.gamerule.not_set",
            vec![literal(rule.id), literal(value)],
        ));
    }
    ctx.world.set_game_rule(rule, value);
    ctx.success(
        Text::translate(
            "commands.gamerule.set",
            vec![literal(rule.id), literal(value)],
        ),
        true,
    );
    Ok(command_result(value))
}

fn ticks(ctx: &Ctx) -> i32 {
    match ctx.arg("time").or_else(|| ctx.arg("duration")) {
        Some(Arg::Ticks(ticks)) => *ticks,
        _ => 0,
    }
}

fn default_clock(ctx: &Ctx) -> Result<String, CommandError> {
    let dimension = ctx.dimension();
    ctx.world.default_clock(&dimension).ok_or_else(|| {
        CommandError::new("commands.time.no_default_clock", vec![literal(&dimension)])
    })
}

fn set_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let clock = default_clock(ctx)?;
    let value = ticks(ctx);
    if ctx.world.clock(&clock).total == i64::from(value) {
        return Err(CommandError::new(
            "commands.time.set.already_at_time",
            vec![literal(&clock), literal(value)],
        ));
    }
    ctx.world.set_time(&clock, i64::from(value));
    ctx.success(
        Text::translate(
            "commands.time.set.absolute",
            vec![literal(&clock), literal(value)],
        ),
        true,
    );
    Ok(value)
}

pub fn ticks_to_marker(total: i64, marker: i32) -> Option<i64> {
    let period = i64::from(TICKS_PER_DAY);
    let phase = total.rem_euclid(period);
    let to = i64::from(marker) - phase;
    match to {
        0 => None,
        to if to > 0 => Some(total + to),
        to => Some(total + period + to),
    }
}

fn set_marker(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let clock = default_clock(ctx)?;
    let Some(Arg::Marker(id)) = ctx.arg("timemarker").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let Some(marker) = marker_ticks(&id).filter(|_| clock == CLOCK) else {
        return Err(CommandError::new(
            "commands.time.no_time_marker_found",
            vec![literal(&id), literal(&clock)],
        ));
    };
    let Some(total) = ticks_to_marker(ctx.world.clock(&clock).total, marker) else {
        return Err(CommandError::new(
            "commands.time.set.already_at_time_marker",
            vec![literal(&clock), literal(&id)],
        ));
    };
    ctx.world.set_time(&clock, total);
    ctx.success(
        Text::translate(
            "commands.time.set.time_marker",
            vec![literal(&clock), literal(&id)],
        ),
        true,
    );
    Ok(wrap_time(total))
}

fn add_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let clock = default_clock(ctx)?;
    let total = ctx.world.clock(&clock).total + i64::from(ticks(ctx));
    ctx.world.set_time(&clock, total);
    ctx.success(
        Text::translate(
            "commands.time.set.absolute",
            vec![literal(&clock), literal(total)],
        ),
        true,
    );
    Ok(wrap_time(total))
}

fn set_paused(ctx: &mut Ctx, paused: bool) -> Result<i32, CommandError> {
    let clock = default_clock(ctx)?;
    if ctx.world.clock(&clock).paused == paused {
        let key = if paused {
            "commands.time.pause.already_paused"
        } else {
            "commands.time.pause.already_running"
        };
        return Err(CommandError::new(key, vec![literal(&clock)]));
    }
    ctx.world.set_time_paused(&clock, paused);
    let key = if paused {
        "commands.time.pause"
    } else {
        "commands.time.resume"
    };
    ctx.success(Text::translate(key, vec![literal(&clock)]), true);
    Ok(1)
}

fn pause_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    set_paused(ctx, true)
}

fn resume_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    set_paused(ctx, false)
}

fn query_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let clock = default_clock(ctx)?;
    let total = ctx.world.clock(&clock).total;
    ctx.success(
        Text::translate(
            "commands.time.query.absolute",
            vec![literal(&clock), literal(total)],
        ),
        false,
    );
    Ok(wrap_time(total))
}

fn query_game_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let game_time = ctx.world.clock(CLOCK).game_time;
    ctx.success(
        Text::translate("commands.time.query.gametime", vec![literal(game_time)]),
        false,
    );
    Ok(wrap_time(game_time))
}

fn set_weather(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let (weather, (min, max), key) = match ctx.literals.last().copied() {
        Some("clear") => (Weather::Clear, RAIN_DELAY, "commands.weather.set.clear"),
        Some("rain") => (Weather::Rain, RAIN_DURATION, "commands.weather.set.rain"),
        Some("thunder") => (
            Weather::Thunder,
            THUNDER_DURATION,
            "commands.weather.set.thunder",
        ),
        _ => return Err(CommandError::new("command.unknown.argument", Vec::new())),
    };
    let duration = match ctx.arg("duration") {
        Some(Arg::Ticks(ticks)) => *ticks,
        _ => ctx.random_between(min, max),
    };
    ctx.world.set_weather(weather, duration);
    ctx.success(Text::translate(key, Vec::new()), true);
    Ok(duration)
}

fn kill(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let targets = ctx.targets("targets")?;
    for &player in &targets {
        ctx.world.kill(player);
    }
    let text = match targets.as_slice() {
        [only] => Text::translate(
            "commands.kill.success.single",
            vec![Text::literal(ctx.server.name(*only).unwrap_or(""))],
        ),
        _ => Text::translate(
            "commands.kill.success.multiple",
            vec![literal(targets.len())],
        ),
    };
    ctx.success(text, true);
    Ok(targets.len() as i32)
}

fn game_mode(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(Arg::GameMode(mode)) = ctx.arg("gamemode").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let name = Text::translate(game_mode_key(mode), Vec::new());
    let mut count = 0;
    for player in ctx.targets("target")? {
        if !ctx.world.set_game_mode(player, mode) {
            continue;
        }
        count += 1;
        if player == ctx.source {
            ctx.success(
                Text::translate("commands.gamemode.success.self", vec![name.clone()]),
                true,
            );
            continue;
        }
        if ctx.world.game_rules().bool(SEND_COMMAND_FEEDBACK) {
            ctx.out.push(system(
                player,
                Text::translate("gameMode.changed", vec![name.clone()]),
            ));
        }
        let target = Text::literal(ctx.server.name(player).unwrap_or(""));
        ctx.success(
            Text::translate(
                "commands.gamemode.success.other",
                vec![target, name.clone()],
            ),
            true,
        );
    }
    Ok(count)
}

fn give(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(Arg::Item(item)) = ctx.arg("item").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let count = match ctx.arg("count") {
        Some(Arg::Int(count)) => *count as u32,
        _ => 1,
    };
    let registries = ctx.server.registries();
    if !registered(&registries.items, &item.id) {
        return Err(
            CommandError::new("argument.item.id.invalid", vec![literal(&item.id)]).at(item.at),
        );
    }
    if let Some((unknown, _)) = item
        .enchantments
        .iter()
        .find(|(id, _)| !registered(&registries.enchantments, id))
    {
        let reason = format!(
            "Unknown registry key in ResourceKey[minecraft:root / minecraft:enchantment]: {unknown}"
        );
        return Err(CommandError::new(
            "arguments.item.component.malformed",
            vec![literal("minecraft:enchantments"), literal(reason)],
        )
        .at(item.at));
    }
    let limit = ctx.world.max_stack(&item.id).max(1) * MAX_ALLOWED_ITEMSTACKS;
    if count > limit {
        return Err(CommandError::new(
            "commands.give.failed.toomanyitems",
            vec![literal(limit), Text::item(&item.id)],
        ));
    }
    let mut given = Vec::new();
    for player in ctx.targets("targets")? {
        if ctx.world.give(player, &item, count) {
            given.push(player);
        }
    }
    let text = match given.as_slice() {
        [] => return Ok(0),
        [only] => Text::translate(
            "commands.give.success.single",
            vec![
                literal(count),
                Text::item(&item.id),
                ctx.display_name(*only),
            ],
        ),
        _ => Text::translate(
            "commands.give.success.multiple",
            vec![literal(count), Text::item(&item.id), literal(given.len())],
        ),
    };
    ctx.success(text, true);
    Ok(given.len() as i32)
}

pub fn in_spawnable_bounds(position: [f64; 3]) -> bool {
    let [x, y, z] = position.map(f64::floor);
    let horizontal = |c: f64| (-30_000_000.0..30_000_000.0).contains(&c);
    horizontal(x) && horizontal(z) && (-20_000_000.0..20_000_000.0).contains(&y)
}

fn format_double(value: f64) -> Text {
    literal(format!("{value:.6}"))
}

fn teleport_all(
    ctx: &mut Ctx,
    dimension: &str,
    position: [f64; 3],
) -> Result<Vec<PlayerId>, CommandError> {
    if !in_spawnable_bounds(position) {
        return Err(CommandError::new(
            "commands.teleport.invalidPosition",
            Vec::new(),
        ));
    }
    let mut moved = Vec::new();
    for player in ctx.targets("targets")? {
        let teleported = if ctx.world.dimension(player) == dimension {
            ctx.world.teleport(player, position)
        } else {
            ctx.world.teleport_to_dimension(player, dimension, position)
        };
        if teleported {
            moved.push(player);
        }
    }
    Ok(moved)
}

fn teleport_to_location(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(Arg::Vec3(location)) = ctx.arg("location").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let position = location.position(ctx.pose());
    let dimension = ctx.dimension();
    let moved = teleport_all(ctx, &dimension, position)?;
    let [x, y, z] = position.map(format_double);
    let text = match moved.as_slice() {
        [] => return Ok(0),
        [only] => Text::translate(
            "commands.teleport.success.location.single",
            vec![ctx.display_name(*only), x, y, z],
        ),
        _ => Text::translate(
            "commands.teleport.success.location.multiple",
            vec![literal(moved.len()), x, y, z],
        ),
    };
    ctx.success(text, true);
    Ok(moved.len() as i32)
}

fn teleport_to_player(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let destination = ctx.targets("destination")?[0];
    let Some(pose) = ctx.world.pose(destination) else {
        return Err(CommandError::new(
            "argument.entity.notfound.player",
            Vec::new(),
        ));
    };
    let dimension = ctx.world.dimension(destination);
    let moved = teleport_all(ctx, &dimension, pose.feet)?;
    let name = ctx.display_name(destination);
    let text = match moved.as_slice() {
        [] => return Ok(0),
        [only] => Text::translate(
            "commands.teleport.success.entity.single",
            vec![ctx.display_name(*only), name],
        ),
        _ => Text::translate(
            "commands.teleport.success.entity.multiple",
            vec![literal(moved.len()), name],
        ),
    };
    ctx.success(text, true);
    Ok(moved.len() as i32)
}

fn effect_arg(ctx: &Ctx) -> Result<Option<String>, CommandError> {
    let Some(Arg::Resource { id, at }) = ctx.arg("effect") else {
        return Ok(None);
    };
    if registered(&ctx.server.registries().effects, id) {
        Ok(Some(id.clone()))
    } else {
        Err(CommandError::new(
            "argument.resource.not_found",
            vec![literal(id), literal(MOB_EFFECT_REGISTRY)],
        )
        .at(*at))
    }
}

fn effect_name(id: &str) -> Text {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    Text::translate(format!("effect.{namespace}.{path}"), Vec::new())
}

pub fn effect_ticks(seconds: Option<i32>, instant: bool) -> Option<u32> {
    match seconds {
        Some(-1) => None,
        Some(seconds) if instant => Some(seconds.max(0) as u32),
        Some(seconds) => Some(seconds.max(0) as u32 * 20),
        None if instant => Some(1),
        None => Some(600),
    }
}

fn give_effect(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(effect) = effect_arg(ctx)? else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let seconds = match ctx.arg("seconds") {
        Some(Arg::Int(seconds)) => Some(*seconds),
        _ if ctx.literals.contains(&"infinite") => Some(-1),
        _ => None,
    };
    let amplifier = match ctx.arg("amplifier") {
        Some(Arg::Int(amplifier)) => *amplifier as u8,
        _ => 0,
    };
    let ticks = effect_ticks(seconds, INSTANT_EFFECTS.contains(&effect.as_str()));
    let mut affected = Vec::new();
    for player in ctx.targets("targets")? {
        if ctx.world.effect_give(player, &effect, ticks, amplifier) {
            affected.push(player);
        }
    }
    let seconds = literal(ticks.map_or(0, |ticks| ticks / 20));
    let text = match affected.as_slice() {
        [] => {
            return Err(CommandError::new("commands.effect.give.failed", Vec::new()));
        }
        [only] => Text::translate(
            "commands.effect.give.success.single",
            vec![effect_name(&effect), ctx.display_name(*only), seconds],
        ),
        _ => Text::translate(
            "commands.effect.give.success.multiple",
            vec![effect_name(&effect), literal(affected.len()), seconds],
        ),
    };
    ctx.success(text, true);
    Ok(affected.len() as i32)
}

fn clear_effects(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let effect = effect_arg(ctx)?;
    let mut cleared = Vec::new();
    for player in ctx.targets("targets")? {
        if ctx.world.effect_clear(player, effect.as_deref()) {
            cleared.push(player);
        }
    }
    let text = match (&effect, cleared.as_slice()) {
        (None, []) => {
            return Err(CommandError::new(
                "commands.effect.clear.everything.failed",
                Vec::new(),
            ));
        }
        (Some(_), []) => {
            return Err(CommandError::new(
                "commands.effect.clear.specific.failed",
                Vec::new(),
            ));
        }
        (None, [only]) => Text::translate(
            "commands.effect.clear.everything.success.single",
            vec![ctx.display_name(*only)],
        ),
        (None, _) => Text::translate(
            "commands.effect.clear.everything.success.multiple",
            vec![literal(cleared.len())],
        ),
        (Some(effect), [only]) => Text::translate(
            "commands.effect.clear.specific.success.single",
            vec![effect_name(effect), ctx.display_name(*only)],
        ),
        (Some(effect), _) => Text::translate(
            "commands.effect.clear.specific.success.multiple",
            vec![effect_name(effect), literal(cleared.len())],
        ),
    };
    ctx.success(text, true);
    Ok(cleared.len() as i32)
}

fn summon(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(Arg::Resource { id, .. }) = ctx.arg("entity").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    if !registered(&ctx.server.registries().entities, &id) {
        return Err(CommandError::new("commands.summon.failed", Vec::new()));
    }
    let position = match ctx.arg("pos") {
        Some(Arg::Vec3(pos)) => pos.position(ctx.pose()),
        _ => ctx.pose().feet,
    };
    if !in_spawnable_bounds(position) {
        return Err(CommandError::new(
            "commands.summon.invalidPosition",
            Vec::new(),
        ));
    }
    let nbt = match ctx.arg("nbt") {
        Some(Arg::Nbt(nbt)) => Some(nbt.clone()),
        _ => None,
    };
    ctx.world
        .summon(ctx.source, &id, position, nbt.as_deref())
        .map_err(|text| CommandError { text, cursor: None })?;
    Ok(1)
}

pub fn usage(command: &str, level: u8) -> Option<String> {
    tree()
        .iter()
        .find(|node| node.name() == command)
        .and_then(|node| smart_usage(node, level, false, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{ALL, OWNERS};
    use crate::protocol::Recipient;
    use crate::server::Clock;
    use crate::server::CommandWorld;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};
    use crate::text::RED;
    use minecraftoss_player::GameMode;

    #[test]
    fn a_level_zero_source_sees_help_and_seed_only() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, FRIEND, "help");
        assert_eq!(texts(&out), vec!["/help [<command>]", "/seed"]);
        let out = run(&mut server, &mut world, FRIEND, "seed");
        assert_eq!(texts(&out), vec!["Seed: [-4172144997902289642]"]);
        let out = run(&mut server, &mut world, FRIEND, "help kill");
        assert_eq!(
            texts(&out),
            vec!["Unknown command or insufficient permissions"]
        );
        assert_eq!(out[0].2, RED);
    }

    #[test]
    fn help_lists_smart_usages_of_visible_commands() {
        let mut server = server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "help");
        assert_eq!(
            texts(&out),
            vec![
                "/clear [<targets>]",
                "/deop <targets>",
                "/difficulty [easy|hard|normal|peaceful]",
                "/effect (clear|give)",
                "/enchant <targets> <enchantment> [<level>]",
                "/execute (in|positioned|run)",
                "/experience (add|query|set)",
                "/fill <from> <to> <block> [destroy|hollow|keep|outline|replace|strict]",
                "/gamemode <gamemode> [<target>]",
                "/gamerule <rule> [<value>]",
                "/give <targets> <item> [<count>]",
                "/help [<command>]",
                "/kill [<targets>]",
                "/kit [<targets>]",
                "/locate (biome|poi|structure)",
                "/op <targets>",
                "/seed",
                "/setblock <pos> <block> [destroy|keep|replace|strict]",
                "/spawnpoint [<targets>]",
                "/summon <entity> [<pos>]",
                "/teleport (<destination>|<location>|<targets>)",
                "/time (add|pause|query|resume|set)",
                "/tp -> teleport",
                "/weather (clear|rain|thunder)",
                "/xp -> experience",
            ]
        );
        let out = run(&mut server, &mut world, HOST, "help weather");
        assert_eq!(
            texts(&out),
            vec![
                "/weather clear [<duration>]",
                "/weather rain [<duration>]",
                "/weather thunder [<duration>]",
            ]
        );
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "help effect give @s regeneration",
        );
        assert_eq!(
            texts(&out),
            vec![
                "/effect give @s regeneration [infinite] [<amplifier>]",
                "/effect give @s regeneration [<seconds>] [<amplifier>]",
            ]
        );
        let out = run(&mut server, &mut world, HOST, "help time set");
        assert_eq!(
            texts(&out),
            vec!["/time set <time>", "/time set <timemarker>"]
        );
        assert_eq!(
            usage("gamerule", OWNERS).as_deref(),
            Some("gamerule <rule> [<value>]")
        );
        assert_eq!(usage("gamerule", ALL), None);
    }

    #[test]
    fn difficulty_query_set_and_same_value_failure() {
        let mut server = server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "difficulty");
        assert_eq!(texts(&out), vec!["The difficulty is Normal"]);
        let out = run(&mut server, &mut world, HOST, "difficulty hard");
        assert_eq!(texts(&out), vec!["The difficulty has been set to Hard"]);
        assert_eq!(world.difficulty, Some(Difficulty::Hard));
        let out = run(&mut server, &mut world, HOST, "difficulty hard");
        assert_eq!(
            texts(&out),
            vec!["The difficulty did not change; it is already set to Hard"]
        );
        assert_eq!(out[0].2, RED);
        let out = run(&mut server, &mut world, HOST, "difficulty extreme");
        assert_eq!(texts(&out)[0], "Incorrect argument for command");
        assert_eq!(
            [
                Difficulty::Peaceful,
                Difficulty::Easy,
                Difficulty::Normal,
                Difficulty::Hard
            ]
            .map(difficulty_id),
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn gamerule_uses_the_world_rules_store() {
        let mut server = server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "gamerule keep_inventory");
        assert_eq!(
            texts(&out),
            vec!["Game rule keep_inventory is currently set to false"]
        );
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "gamerule minecraft:keep_inventory true",
        );
        assert_eq!(
            texts(&out),
            vec!["Game rule keep_inventory is now set to true"]
        );
        assert!(world.rules.keep_inventory());
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "gamerule keep_inventory true",
        );
        assert_eq!(
            texts(&out),
            vec!["Game rule keep_inventory is already set to true"]
        );
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "gamerule random_tick_speed 30",
        );
        assert_eq!(
            texts(&out),
            vec!["Game rule random_tick_speed is now set to 30"]
        );
        assert_eq!(world.rules.int("random_tick_speed"), 30);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "gamerule max_snow_accumulation_height 9",
        );
        assert_eq!(out.len(), 2);
        assert_eq!(world.rules.int("max_snow_accumulation_height"), 1);
        let out = run(&mut server, &mut world, HOST, "gamerule keepInventory");
        assert_eq!(texts(&out)[0], "Incorrect argument for command");
    }

    #[test]
    fn weather_and_time_call_the_world() {
        let mut server = server();
        let mut world = FakeWorld {
            clock: Clock {
                total: 25_000,
                game_time: 777,
                paused: false,
            },
            ..FakeWorld::default()
        };
        let out = run(&mut server, &mut world, HOST, "weather rain 600");
        assert_eq!(texts(&out), vec!["Set the weather to rain"]);
        run(&mut server, &mut world, HOST, "weather thunder 1d");
        run(&mut server, &mut world, HOST, "weather clear");
        assert_eq!(world.weather[0], (Weather::Rain, 600));
        assert_eq!(world.weather[1], (Weather::Thunder, 24_000));
        assert_eq!(world.weather[2].0, Weather::Clear);
        assert!((12_000..=180_000).contains(&world.weather[2].1));
        let out = run(&mut server, &mut world, HOST, "weather rain 0");
        assert_eq!(out.len(), 2);
        assert_eq!(world.weather.len(), 3);
        let out = run(&mut server, &mut world, HOST, "time set noon");
        assert_eq!(
            texts(&out),
            vec!["Set minecraft:overworld to time marker minecraft:noon"]
        );
        assert_eq!(world.clock.total, 30_000);
        let out = run(&mut server, &mut world, HOST, "time set noon");
        assert_eq!(
            texts(&out)[0],
            "Clock minecraft:overworld is already at time marker minecraft:noon"
        );
        run(&mut server, &mut world, HOST, "time set minecraft:night");
        assert_eq!(world.clock.total, 37_000);
        run(&mut server, &mut world, HOST, "time set day");
        assert_eq!(world.clock.total, 49_000);
        let out = run(&mut server, &mut world, HOST, "time set dusk");
        assert_eq!(
            texts(&out),
            vec!["Time marker minecraft:dusk does not exist for clock minecraft:overworld"]
        );
        let out = run(&mut server, &mut world, HOST, "time set 1000");
        assert_eq!(texts(&out), vec!["Set minecraft:overworld to 1000 tick(s)"]);
        assert_eq!(world.clock.total, 1000);
        let out = run(&mut server, &mut world, HOST, "time set 1000");
        assert_eq!(out[0].2, RED);
        run(&mut server, &mut world, HOST, "time add 2d");
        assert_eq!(world.clock.total, 49_000);
        run(&mut server, &mut world, HOST, "time add -500");
        assert_eq!(world.clock.total, 48_500);
        let out = run(&mut server, &mut world, HOST, "time query time");
        assert_eq!(
            texts(&out),
            vec!["Clock minecraft:overworld is at 48500 tick(s)"]
        );
        let out = run(&mut server, &mut world, HOST, "time query gametime");
        assert_eq!(texts(&out), vec!["The game time is 777 tick(s)"]);
        let out = run(&mut server, &mut world, HOST, "time pause");
        assert_eq!(texts(&out), vec!["Paused clock minecraft:overworld"]);
        assert!(world.clock.paused);
        let out = run(&mut server, &mut world, HOST, "time pause");
        assert_eq!(
            texts(&out),
            vec!["Clock minecraft:overworld is already paused"]
        );
        let out = run(&mut server, &mut world, HOST, "time resume");
        assert_eq!(texts(&out), vec!["Resumed clock minecraft:overworld"]);
        assert!(!world.clock.paused);
    }

    #[test]
    fn time_acts_on_the_default_clock_of_the_command_s_dimension() {
        let mut server = server();
        let mut world = FakeWorld {
            clock: Clock {
                total: 1000,
                game_time: 777,
                paused: false,
            },
            ..FakeWorld::default()
        };
        world
            .player_dimensions
            .push((HOST, "minecraft:the_nether".to_owned()));
        for command in [
            "time set 5000",
            "time set noon",
            "time add 1d",
            "time pause",
            "time resume",
            "time query time",
        ] {
            let out = run(&mut server, &mut world, HOST, command);
            assert_eq!(
                texts(&out),
                vec!["There is no default clock in dimension minecraft:the_nether"],
                "{command}"
            );
            assert_eq!(out[0].2, RED);
        }
        let out = run(&mut server, &mut world, HOST, "time query gametime");
        assert_eq!(texts(&out), vec!["The game time is 777 tick(s)"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:overworld run time set 2000",
        );
        assert_eq!(texts(&out), vec!["Set minecraft:overworld to 2000 tick(s)"]);
        assert_eq!((world.clock.total, world.end_clock.total), (2000, 0));
        world
            .player_dimensions
            .push((HOST, "minecraft:the_end".to_owned()));
        let out = run(&mut server, &mut world, HOST, "time set 5000");
        assert_eq!(texts(&out), vec!["Set minecraft:the_end to 5000 tick(s)"]);
        let out = run(&mut server, &mut world, HOST, "time add 500");
        assert_eq!(texts(&out), vec!["Set minecraft:the_end to 5500 tick(s)"]);
        let out = run(&mut server, &mut world, HOST, "time set noon");
        assert_eq!(
            texts(&out),
            vec!["Time marker minecraft:noon does not exist for clock minecraft:the_end"]
        );
        let out = run(&mut server, &mut world, HOST, "time pause");
        assert_eq!(texts(&out), vec!["Paused clock minecraft:the_end"]);
        let out = run(&mut server, &mut world, HOST, "time query time");
        assert_eq!(
            texts(&out),
            vec!["Clock minecraft:the_end is at 5500 tick(s)"]
        );
        assert_eq!(
            (world.end_clock.total, world.end_clock.paused),
            (5500, true)
        );
        assert_eq!((world.clock.total, world.clock.paused), (2000, false));
    }

    #[test]
    fn markers_move_forward_to_their_next_occurrence() {
        assert_eq!(ticks_to_marker(0, 6000), Some(6000));
        assert_eq!(ticks_to_marker(6000, 6000), None);
        assert_eq!(ticks_to_marker(7000, 6000), Some(30_000));
        assert_eq!(ticks_to_marker(30_500, 0), Some(48_000));
    }

    #[test]
    fn gamemode_sets_self_with_the_self_message() {
        let mut server = server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "gamemode creative");
        assert_eq!(texts(&out), vec!["Set own game mode to Creative Mode"]);
        assert_eq!(out[0].0, Recipient::Player(HOST));
        assert_eq!(world.game_mode(HOST), Some(GameMode::Creative));
        let out = run(&mut server, &mut world, HOST, "gamemode spectator @s");
        assert_eq!(texts(&out), vec!["Set own game mode to Spectator Mode"]);
        assert_eq!(world.game_mode(HOST), Some(GameMode::Spectator));
    }

    #[test]
    fn gamemode_to_the_same_mode_says_nothing_and_changes_nothing() {
        let mut server = server();
        let mut world = FakeWorld::default();
        assert!(run(&mut server, &mut world, HOST, "gamemode survival").is_empty());
        assert!(world.mode_changes.is_empty());
        run(&mut server, &mut world, HOST, "gamemode adventure");
        assert!(run(&mut server, &mut world, HOST, "gamemode adventure").is_empty());
        assert_eq!(world.mode_changes.len(), 1);
    }

    #[test]
    fn gamemode_on_another_player_tells_both() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "gamemode creative Alex");
        assert_eq!(
            out,
            vec![
                (
                    Recipient::Player(FRIEND),
                    "Your game mode has been updated to Creative Mode".to_owned(),
                    crate::text::WHITE
                ),
                (
                    Recipient::Player(HOST),
                    "Set Alex's game mode to Creative Mode".to_owned(),
                    crate::text::WHITE
                ),
            ]
        );
        let out = run(&mut server, &mut world, HOST, "gamemode creative @a");
        assert_eq!(texts(&out), vec!["Set own game mode to Creative Mode"]);
        world.rules.set(SEND_COMMAND_FEEDBACK, "false").unwrap();
        assert!(run(&mut server, &mut world, HOST, "gamemode adventure @a").is_empty());
        assert_eq!(world.game_mode(FRIEND), Some(GameMode::Adventure));
    }

    #[test]
    fn mode_changes_reach_the_world_once_with_old_and_new() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld::default();
        run(&mut server, &mut world, HOST, "gamemode creative");
        run(&mut server, &mut world, HOST, "gamemode creative");
        run(&mut server, &mut world, HOST, "gamemode spectator @a");
        assert_eq!(
            world.mode_changes,
            vec![
                (HOST, GameMode::Survival, GameMode::Creative),
                (HOST, GameMode::Creative, GameMode::Spectator),
                (FRIEND, GameMode::Survival, GameMode::Spectator),
            ]
        );
    }

    #[test]
    fn gamemode_needs_level_two_and_a_known_mode() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, FRIEND, "gamemode creative");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
        assert!(world.mode_changes.is_empty());
        let out = run(&mut server, &mut world, HOST, "gamemode god");
        assert_eq!(
            texts(&out),
            vec!["Unknown game mode: god", "...memode god<--[HERE]"]
        );
        let out = run(&mut server, &mut world, HOST, "gamemode creative Herobrine");
        assert_eq!(texts(&out), vec!["No player was found"]);
        assert!(world.mode_changes.is_empty());
    }

    #[test]
    fn kill_without_target_kills_the_source() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "kill");
        assert_eq!(texts(&out), vec!["Killed Steve"]);
        assert_eq!(world.killed, vec![HOST]);
        let out = run(&mut server, &mut world, HOST, "kill @a");
        assert_eq!(texts(&out), vec!["Killed 2 entities"]);
        assert_eq!(world.killed, vec![HOST, HOST, FRIEND]);
        let out = run(&mut server, &mut world, HOST, "kill alex");
        assert_eq!(texts(&out), vec!["Killed Alex"]);
        let out = run(&mut server, &mut world, HOST, "kill @s");
        assert_eq!(out[0].0, Recipient::Player(HOST));
        assert_eq!(world.killed.len(), 5);
        let out = run(&mut server, &mut world, HOST, "kill Herobrine");
        assert_eq!(texts(&out), vec!["No player was found"]);
        assert_eq!(world.killed.len(), 5);
    }

    fn registered_server() -> crate::server::CommandServer {
        let mut server = server();
        let owned = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect();
        *server.registries_mut() = crate::server::CommandRegistries {
            items: owned(&["minecraft:stone", "minecraft:diamond_sword"]),
            entities: owned(&["minecraft:cow", "minecraft:zombie"]),
            effects: owned(&[
                "minecraft:regeneration",
                "minecraft:speed",
                "minecraft:instant_health",
            ]),
            enchantments: owned(&["minecraft:sharpness"]),
        };
        server
    }

    fn feet_at(world: &mut FakeWorld, player: crate::protocol::PlayerId, feet: [f64; 3]) {
        world.poses.push((
            player,
            crate::server::Pose {
                feet,
                yaw: 0.0,
                pitch: 0.0,
            },
        ));
    }

    #[test]
    fn give_counts_and_too_many_items() {
        let mut server = registered_server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "give @s stone");
        assert_eq!(texts(&out), vec!["Gave 1 [Stone] to Steve"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "give Steve minecraft:stone 6400",
        );
        assert_eq!(texts(&out), vec!["Gave 6400 [Stone] to Steve"]);
        let out = run(&mut server, &mut world, HOST, "give @s stone 6401");
        assert_eq!(texts(&out), vec!["Can't give more than 6400 of [Stone]"]);
        assert_eq!(out[0].2, RED);
        let out = run(&mut server, &mut world, HOST, "give @s diamond_sword 101");
        assert_eq!(
            texts(&out),
            vec!["Can't give more than 100 of [Diamond Sword]"]
        );
        let out = run(&mut server, &mut world, HOST, "give @s stone 0");
        assert_eq!(out.len(), 2);
        let counts: Vec<(String, u32)> = world
            .given
            .iter()
            .map(|(_, item, count)| (item.id.clone(), *count))
            .collect();
        assert_eq!(
            counts,
            vec![
                ("minecraft:stone".to_owned(), 1),
                ("minecraft:stone".to_owned(), 6400)
            ]
        );
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "give @s diamond_sword[enchantments={sharpness:5}]",
        );
        assert_eq!(texts(&out), vec!["Gave 1 [Diamond Sword] to Steve"]);
        assert_eq!(
            world.given[2].1.enchantments,
            vec![("minecraft:sharpness".to_owned(), 5)]
        );
        run(
            &mut server,
            &mut world,
            HOST,
            "give @s diamond_sword+sharpness=3 2",
        );
        assert_eq!(
            (world.given[3].1.enchantments.clone(), world.given[3].2),
            (vec![("minecraft:sharpness".to_owned(), 3)], 2)
        );
    }

    #[test]
    fn unknown_items_are_invalid() {
        let mut server = registered_server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "give @s notanitem");
        assert_eq!(
            texts(&out),
            vec![
                "Unknown item 'minecraft:notanitem'",
                "give @s notanitem<--[HERE]"
            ]
        );
        assert_eq!(out[1].0, Recipient::Player(HOST));
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "give @s diamond_sword[enchantments={unbreaking:3}]",
        );
        assert!(texts(&out)[0].starts_with("Malformed 'minecraft:enchantments' component"));
        assert!(texts(&out)[0].contains("minecraft:unbreaking"));
        assert!(world.given.is_empty());
        let mut open = server.clone();
        open.registries_mut().items.clear();
        run(&mut open, &mut world, HOST, "give @s modded:thing");
        assert_eq!(world.given[0].1.id, "modded:thing");
    }

    #[test]
    fn tp_variants_resolve_targets_and_positions() {
        let mut server = registered_server();
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld::default();
        feet_at(&mut world, HOST, [10.5, 64.0, -3.5]);
        feet_at(&mut world, FRIEND, [100.0, 70.0, 100.0]);
        let out = run(&mut server, &mut world, HOST, "tp ~ ~10 ~");
        assert_eq!(
            texts(&out),
            vec!["Teleported Steve to 10.500000, 74.000000, -3.500000"]
        );
        let out = run(&mut server, &mut world, HOST, "teleport 2000 120 2000");
        assert_eq!(
            texts(&out),
            vec!["Teleported Steve to 2000.500000, 120.000000, 2000.500000"]
        );
        run(&mut server, &mut world, HOST, "tp Alex 0 100 0");
        let out = run(&mut server, &mut world, HOST, "tp Alex");
        assert_eq!(texts(&out), vec!["Teleported Steve to Alex"]);
        let out = run(&mut server, &mut world, HOST, "tp @a Steve");
        assert_eq!(out.len(), 1);
        assert_eq!(
            world.teleported,
            vec![
                (HOST, [10.5, 74.0, -3.5]),
                (HOST, [2000.5, 120.0, 2000.5]),
                (FRIEND, [0.5, 100.0, 0.5]),
                (HOST, [0.5, 100.0, 0.5]),
                (HOST, [0.5, 100.0, 0.5]),
                (FRIEND, [0.5, 100.0, 0.5]),
            ]
        );
        let out = run(&mut server, &mut world, HOST, "tp 30000000 64 0");
        assert_eq!(texts(&out), vec!["Invalid position for teleport"]);
        let out = run(&mut server, &mut world, HOST, "tp ~ ~");
        assert_eq!(
            texts(&out),
            vec!["Incorrect argument for command", "tp ~ ~<--[HERE]"]
        );
        let out = run(&mut server, &mut world, HOST, "tp 1 2");
        assert_eq!(texts(&out), vec!["No player was found"]);
        let out = run(&mut server, &mut world, HOST, "tp Herobrine");
        assert_eq!(texts(&out), vec!["No player was found"]);
        assert_eq!(world.teleported.len(), 6);
        let out = run(&mut server, &mut world, FRIEND, "tp 0 0 0");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
    }

    #[test]
    fn effect_defaults_to_thirty_seconds_amplifier_zero() {
        let mut server = registered_server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "effect give @s regeneration");
        assert_eq!(texts(&out), vec!["Applied effect Regeneration to Steve"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "effect give @s minecraft:regeneration 10 1 true",
        );
        assert_eq!(out.len(), 1);
        run(
            &mut server,
            &mut world,
            HOST,
            "effect give @s instant_health",
        );
        run(
            &mut server,
            &mut world,
            HOST,
            "effect give @s instant_health 5 2",
        );
        assert_eq!(
            world.effects,
            vec![
                (HOST, "minecraft:regeneration".to_owned(), Some(600), 0),
                (HOST, "minecraft:regeneration".to_owned(), Some(200), 1),
                (HOST, "minecraft:instant_health".to_owned(), Some(1), 0),
                (HOST, "minecraft:instant_health".to_owned(), Some(5), 2),
            ]
        );
        let out = run(&mut server, &mut world, HOST, "effect give @s speed");
        assert_eq!(
            texts(&out),
            vec![
                "Unable to apply this effect (target is either immune to effects, or has something stronger)"
            ]
        );
        let out = run(&mut server, &mut world, HOST, "effect give @s glowing");
        assert_eq!(
            texts(&out),
            vec![
                "Can't find element 'minecraft:glowing' of type 'minecraft:mob_effect'",
                "...@s glowing<--[HERE]"
            ]
        );
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "effect give @s regeneration 0",
        );
        assert_eq!(out.len(), 2);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "effect give @s regeneration 1 256",
        );
        assert_eq!(out.len(), 2);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "effect clear @s minecraft:regeneration",
        );
        assert_eq!(texts(&out), vec!["Removed effect Regeneration from Steve"]);
        let out = run(&mut server, &mut world, HOST, "effect clear");
        assert_eq!(texts(&out), vec!["Removed every effect from Steve"]);
        let out = run(&mut server, &mut world, HOST, "effect clear @s");
        assert_eq!(texts(&out), vec!["Target has no effects to remove"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "effect clear @s regeneration",
        );
        assert_eq!(
            texts(&out),
            vec!["Target doesn't have the requested effect"]
        );
    }

    #[test]
    fn effect_infinite_has_no_end() {
        let mut server = registered_server();
        let mut world = FakeWorld::default();
        run(
            &mut server,
            &mut world,
            HOST,
            "effect give @s regeneration infinite",
        );
        run(
            &mut server,
            &mut world,
            HOST,
            "effect give @s regeneration infinite 3 true",
        );
        assert_eq!(world.effects[0].2, None);
        assert_eq!(world.effects[1].2, None);
        assert_eq!(world.effects[1].3, 3);
        assert_eq!(effect_ticks(Some(-1), false), None);
        assert_eq!(effect_ticks(Some(-1), true), None);
        assert_eq!(effect_ticks(Some(7), true), Some(7));
        assert_eq!(effect_ticks(Some(30), false), Some(600));
        assert_eq!(effect_ticks(None, true), Some(1));
    }

    #[test]
    fn summon_defaults_to_the_source_feet() {
        let mut server = registered_server();
        let mut world = FakeWorld::default();
        feet_at(&mut world, HOST, [5.25, 70.0, -8.75]);
        assert!(run(&mut server, &mut world, HOST, "summon cow").is_empty());
        run(&mut server, &mut world, HOST, "summon minecraft:cow ~ ~ ~3");
        run(
            &mut server,
            &mut world,
            HOST,
            "summon zombie 10 64 10 {NoAI:1b,CustomName:'a b'}",
        );
        assert_eq!(
            world.summoned,
            vec![
                ("minecraft:cow".to_owned(), [5.25, 70.0, -8.75], None),
                ("minecraft:cow".to_owned(), [5.25, 70.0, -5.75], None),
                (
                    "minecraft:zombie".to_owned(),
                    [10.5, 64.0, 10.5],
                    Some("{NoAI:1b,CustomName:'a b'}".to_owned())
                ),
            ]
        );
        let out = run(&mut server, &mut world, HOST, "summon cow ~ ~ ~ {bad}");
        assert_eq!(texts(&out), vec!["Expected value"]);
        assert_eq!(out[0].2, RED);
        let out = run(&mut server, &mut world, HOST, "summon cow 0 30000000 0");
        assert_eq!(texts(&out), vec!["Invalid position for summon"]);
    }

    #[test]
    fn summon_rejects_unsimulated_kinds() {
        let mut server = registered_server();
        let mut world = FakeWorld::default();
        let out = run(&mut server, &mut world, HOST, "summon minecraft:fox");
        assert_eq!(texts(&out), vec!["Unable to summon entity"]);
        assert_eq!(out[0].2, RED);
        let out = run(&mut server, &mut world, HOST, "summon Cow");
        assert_eq!(out.len(), 2);
        assert!(world.summoned.is_empty());
    }
}
