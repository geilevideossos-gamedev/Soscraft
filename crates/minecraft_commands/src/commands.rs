use std::sync::OnceLock;

use minecraftoss_player::Difficulty;
use minecraftoss_world::game_rules::Value;

use crate::args::{Arg, ArgKind, TICKS_PER_DAY, marker_ticks};
use crate::permissions::{ADMINS, GAMEMASTERS};
use crate::players::game_mode_key;
use crate::reader::{CommandError, literal};
use crate::server::{Ctx, SEND_COMMAND_FEEDBACK, Weather, system};
use crate::text::{GREEN, Text};
use crate::tree::{Node, parse, smart_usage, smart_usages, sorted};

pub const CLOCK: &str = "minecraft:overworld";
const RAIN_DELAY: (i32, i32) = (12_000, 180_000);
const RAIN_DURATION: (i32, i32) = (12_000, 24_000);
const THUNDER_DURATION: (i32, i32) = (3600, 15_600);

pub fn tree() -> &'static [Node] {
    static TREE: OnceLock<Vec<Node>> = OnceLock::new();
    TREE.get_or_init(build)
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
    let Some(last) = parsed.nodes.last() else {
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

fn set_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let value = ticks(ctx);
    if ctx.world.clock().total == i64::from(value) {
        return Err(CommandError::new(
            "commands.time.set.already_at_time",
            vec![literal(CLOCK), literal(value)],
        ));
    }
    ctx.world.set_time(i64::from(value));
    ctx.success(
        Text::translate(
            "commands.time.set.absolute",
            vec![literal(CLOCK), literal(value)],
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
    let Some(Arg::Marker(id)) = ctx.arg("timemarker").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let Some(marker) = marker_ticks(&id) else {
        return Err(CommandError::new(
            "commands.time.no_time_marker_found",
            vec![literal(&id), literal(CLOCK)],
        ));
    };
    let Some(total) = ticks_to_marker(ctx.world.clock().total, marker) else {
        return Err(CommandError::new(
            "commands.time.set.already_at_time_marker",
            vec![literal(CLOCK), literal(&id)],
        ));
    };
    ctx.world.set_time(total);
    ctx.success(
        Text::translate(
            "commands.time.set.time_marker",
            vec![literal(CLOCK), literal(&id)],
        ),
        true,
    );
    Ok(wrap_time(total))
}

fn add_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let total = ctx.world.clock().total + i64::from(ticks(ctx));
    ctx.world.set_time(total);
    ctx.success(
        Text::translate(
            "commands.time.set.absolute",
            vec![literal(CLOCK), literal(total)],
        ),
        true,
    );
    Ok(wrap_time(total))
}

fn set_paused(ctx: &mut Ctx, paused: bool) -> Result<i32, CommandError> {
    if ctx.world.clock().paused == paused {
        let key = if paused {
            "commands.time.pause.already_paused"
        } else {
            "commands.time.pause.already_running"
        };
        return Err(CommandError::new(key, vec![literal(CLOCK)]));
    }
    ctx.world.set_time_paused(paused);
    let key = if paused {
        "commands.time.pause"
    } else {
        "commands.time.resume"
    };
    ctx.success(Text::translate(key, vec![literal(CLOCK)]), true);
    Ok(1)
}

fn pause_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    set_paused(ctx, true)
}

fn resume_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    set_paused(ctx, false)
}

fn query_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let total = ctx.world.clock().total;
    ctx.success(
        Text::translate(
            "commands.time.query.absolute",
            vec![literal(CLOCK), literal(total)],
        ),
        false,
    );
    Ok(wrap_time(total))
}

fn query_game_time(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let game_time = ctx.world.clock().game_time;
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
                "/deop <targets>",
                "/difficulty [easy|hard|normal|peaceful]",
                "/gamemode <gamemode> [<target>]",
                "/gamerule <rule> [<value>]",
                "/help [<command>]",
                "/kill [<targets>]",
                "/op <targets>",
                "/seed",
                "/time (add|pause|query|resume|set)",
                "/weather (clear|rain|thunder)",
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
}
