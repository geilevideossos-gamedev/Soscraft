use crate::args::{Arg, ArgKind};
use crate::permissions::ADMINS;
use crate::reader::{CommandError, literal};
use crate::server::{Ctx, SprintReport, TickQuery};
use crate::text::Text;
use crate::tree::Node;

pub const MIN_RATE: f32 = 1.0;
pub const MAX_RATE: f32 = 10000.0;
static RATE_SUGGESTIONS: [&str; 1] = ["20"];
static STEP_SUGGESTIONS: [&str; 2] = ["1t", "1s"];
static SPRINT_SUGGESTIONS: [&str; 3] = ["60s", "1d", "3d"];
const NANOS_PER_MILLISECOND: f32 = 1_000_000.0;

pub(crate) fn node() -> Node {
    let time = || Node::argument("time", ArgKind::Time { min: 1 });
    let rate = ArgKind::Float {
        min: MIN_RATE,
        max: MAX_RATE,
    };
    Node::literal("tick")
        .requires(ADMINS)
        .then(Node::literal("query").executes(query))
        .then(
            Node::literal("rate").then(
                Node::argument("rate", rate)
                    .suggests(&RATE_SUGGESTIONS)
                    .executes(set_rate),
            ),
        )
        .then(
            Node::literal("step")
                .executes(step)
                .then(Node::literal("stop").executes(stop_stepping))
                .then(time().suggests(&STEP_SUGGESTIONS).executes(step)),
        )
        .then(
            Node::literal("sprint")
                .then(Node::literal("stop").executes(stop_sprinting))
                .then(time().suggests(&SPRINT_SUGGESTIONS).executes(sprint)),
        )
        .then(Node::literal("unfreeze").executes(unfreeze))
        .then(Node::literal("freeze").executes(freeze))
}

pub fn java_fixed(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return format!("{value}");
    }
    let text = format!("{}", value.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let mut digits: Vec<u8> = whole
        .bytes()
        .chain(
            fraction
                .bytes()
                .chain(std::iter::repeat(b'0'))
                .take(decimals),
        )
        .collect();
    if fraction
        .as_bytes()
        .get(decimals)
        .is_some_and(|&digit| digit >= b'5')
    {
        let mut at = digits.len();
        loop {
            if at == 0 {
                digits.insert(0, b'1');
                break;
            }
            at -= 1;
            if digits[at] == b'9' {
                digits[at] = b'0';
            } else {
                digits[at] += 1;
                break;
            }
        }
    }
    let point = digits.len() - decimals;
    let mut out = String::new();
    if value.is_sign_negative() {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&digits[..point]).unwrap_or("0"));
    if decimals > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(&digits[point..]).unwrap_or(""));
    }
    out
}

pub fn millis(nanos: u64) -> String {
    java_fixed(f64::from(nanos as f32 / NANOS_PER_MILLISECOND), 1)
}

pub fn sprint_report(report: SprintReport) -> Text {
    Text::translate(
        "commands.tick.sprint.report",
        vec![
            literal(report.ticks_per_second),
            literal(java_fixed(report.ms_per_tick, 2)),
        ],
    )
}

fn manager(ctx: &Ctx) -> Result<TickQuery, CommandError> {
    ctx.world
        .tick_query()
        .ok_or_else(|| CommandError::new("command.unknown.command", Vec::new()))
}

fn line(key: &str) -> Text {
    Text::translate(key, Vec::new())
}

fn query(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let tick = manager(ctx)?;
    let average = millis(tick.average_nanos);
    let rate = java_fixed(f64::from(tick.rate), 1);
    if tick.sprinting {
        ctx.success(line("commands.tick.status.sprinting"), false);
        ctx.success(
            Text::translate(
                "commands.tick.query.rate.sprinting",
                vec![literal(rate), literal(average)],
            ),
            false,
        );
    } else {
        let status = if tick.frozen {
            "commands.tick.status.frozen"
        } else if tick.nanos_per_tick < tick.average_nanos {
            "commands.tick.status.lagging"
        } else {
            "commands.tick.status.running"
        };
        ctx.success(line(status), false);
        ctx.success(
            Text::translate(
                "commands.tick.query.rate.running",
                vec![
                    literal(rate),
                    literal(average),
                    literal(millis(tick.nanos_per_tick)),
                ],
            ),
            false,
        );
    }
    let mut sorted = tick.samples;
    sorted.sort_unstable();
    let count = sorted.len();
    let at = |index: usize| literal(millis(sorted[index.min(count - 1)]));
    ctx.success(
        Text::translate(
            "commands.tick.query.percentiles",
            vec![
                at(count / 2),
                at((count as f64 * 0.95) as usize),
                at((count as f64 * 0.99) as usize),
                literal(count),
            ],
        ),
        false,
    );
    Ok(tick.rate as i32)
}

fn set_rate(ctx: &mut Ctx) -> Result<i32, CommandError> {
    manager(ctx)?;
    let Some(Arg::Float(rate)) = ctx.arg("rate").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    ctx.world.set_tick_rate(rate);
    ctx.success(
        Text::translate(
            "commands.tick.rate.success",
            vec![literal(java_fixed(f64::from(rate), 1))],
        ),
        true,
    );
    Ok(rate as i32)
}

fn step(ctx: &mut Ctx) -> Result<i32, CommandError> {
    manager(ctx)?;
    let ticks = match ctx.arg("time") {
        Some(Arg::Ticks(ticks)) => *ticks,
        _ => 1,
    };
    if !ctx.world.step_ticks(ticks) {
        return Err(CommandError::new("commands.tick.step.fail", Vec::new()));
    }
    ctx.success(
        Text::translate("commands.tick.step.success", vec![literal(ticks)]),
        true,
    );
    Ok(1)
}

fn stop_stepping(ctx: &mut Ctx) -> Result<i32, CommandError> {
    manager(ctx)?;
    if !ctx.world.stop_stepping() {
        return Err(CommandError::new(
            "commands.tick.step.stop.fail",
            Vec::new(),
        ));
    }
    ctx.success(line("commands.tick.step.stop.success"), true);
    Ok(1)
}

fn sprint(ctx: &mut Ctx) -> Result<i32, CommandError> {
    manager(ctx)?;
    let Some(Arg::Ticks(ticks)) = ctx.arg("time").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    if ctx.world.sprint_ticks(ticks) {
        ctx.success(line("commands.tick.sprint.stop.success"), true);
    }
    ctx.success(line("commands.tick.status.sprinting"), true);
    Ok(1)
}

fn report(ctx: &mut Ctx, report: SprintReport) {
    let out = ctx
        .server
        .server_feedback(sprint_report(report), ctx.world.game_rules());
    ctx.out.extend(out);
}

fn stop_sprinting(ctx: &mut Ctx) -> Result<i32, CommandError> {
    manager(ctx)?;
    let Some(finished) = ctx.world.stop_sprinting() else {
        return Err(CommandError::new(
            "commands.tick.sprint.stop.fail",
            Vec::new(),
        ));
    };
    report(ctx, finished);
    ctx.success(line("commands.tick.sprint.stop.success"), true);
    Ok(1)
}

fn set_frozen(ctx: &mut Ctx, frozen: bool) -> Result<i32, CommandError> {
    manager(ctx)?;
    if frozen {
        if let Some(finished) = ctx.world.stop_sprinting() {
            report(ctx, finished);
        }
        ctx.world.stop_stepping();
    }
    ctx.world.set_tick_frozen(frozen);
    let status = if frozen {
        "commands.tick.status.frozen"
    } else {
        "commands.tick.status.running"
    };
    ctx.success(line(status), true);
    Ok(i32::from(frozen))
}

fn freeze(ctx: &mut Ctx) -> Result<i32, CommandError> {
    set_frozen(ctx, true)
}

fn unfreeze(ctx: &mut Ctx) -> Result<i32, CommandError> {
    set_frozen(ctx, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::OWNERS;
    use crate::protocol::Recipient;
    use crate::server::tests::{FRIEND, FakeTick, FakeWorld, HOST, op_at, run, server, texts};
    use crate::suggest::{CommandHints, complete};
    use crate::text::{GRAY, RED, WHITE};
    use crate::tree::{parse, runnable};

    const MS: u64 = 1_000_000;

    fn world() -> FakeWorld {
        FakeWorld {
            tick: Some(FakeTick::default()),
            ..FakeWorld::default()
        }
    }

    fn tick(world: &mut FakeWorld) -> &mut FakeTick {
        world.tick.as_mut().unwrap()
    }

    fn result(world: &mut FakeWorld, input: &str) -> Result<i32, CommandError> {
        let mut server = server();
        let parsed = parse(crate::commands::tree(), input, OWNERS);
        let exec = runnable(&parsed, input)?;
        let mut ctx = Ctx {
            server: &mut server,
            world,
            source: HOST,
            level: OWNERS,
            input,
            args: parsed.args.clone(),
            literals: parsed.literals.clone(),
            out: Vec::new(),
            at: None,
            dimension: None,
        };
        exec(&mut ctx)
    }

    fn suggestions(input: &str) -> Vec<String> {
        complete(input, input.len(), OWNERS, &CommandHints::default())
            .list
            .into_iter()
            .map(|suggestion| suggestion.text)
            .collect()
    }

    #[test]
    fn numbers_round_half_up_like_java_format() {
        assert_eq!(java_fixed(2.25, 1), "2.3");
        assert_eq!(java_fixed(2.24, 1), "2.2");
        assert_eq!(java_fixed(0.5, 2), "0.50");
        assert_eq!(java_fixed(9.96, 1), "10.0");
        assert_eq!(java_fixed(99.999, 2), "100.00");
        assert_eq!(java_fixed(20.0, 1), "20.0");
        assert_eq!(java_fixed(1e-7, 1), "0.0");
        assert_eq!(java_fixed(1234.5, 0), "1235");
        assert_eq!(java_fixed(f64::from(0.15f32), 1), "0.2");
        assert_eq!(millis(50 * MS), "50.0");
        assert_eq!(millis(333_333_333), "333.3");
        assert_eq!(millis(0), "0.0");
    }

    #[test]
    fn query_shows_the_status_the_rates_and_the_percentiles() {
        let mut server = server();
        let mut world = world();
        let state = tick(&mut world);
        for (index, sample) in state.query.samples.iter_mut().enumerate() {
            *sample = (index as u64 + 1) * MS;
        }
        state.query.average_nanos = 2_250_000;
        let percentiles = "Percentiles: P50: 51.0ms P95: 96.0ms P99: 100.0ms. Sample: 100";
        let out = run(&mut server, &mut world, HOST, "tick query");
        assert_eq!(
            texts(&out),
            vec![
                "The game is running normally",
                "Target tick rate: 20.0 per second.\nAverage time per tick: 2.3ms (Target: 50.0ms)",
                percentiles,
            ]
        );
        assert_eq!(result(&mut world, "tick query"), Ok(20));
        tick(&mut world).query.average_nanos = 60 * MS;
        let out = run(&mut server, &mut world, HOST, "tick query");
        assert_eq!(
            texts(&out)[..2],
            [
                "The game is running, but can't keep up with the target tick rate",
                "Target tick rate: 20.0 per second.\nAverage time per tick: 60.0ms (Target: 50.0ms)",
            ]
        );
        tick(&mut world).query.frozen = true;
        let out = run(&mut server, &mut world, HOST, "tick query");
        assert_eq!(texts(&out)[0], "The game is frozen");
        tick(&mut world).query.sprinting = true;
        let out = run(&mut server, &mut world, HOST, "tick query");
        assert_eq!(
            texts(&out),
            vec![
                "The game is sprinting",
                "Target tick rate: 20.0 per second (ignored, reference only).\nAverage time per tick: 60.0ms",
                percentiles,
            ]
        );
        assert!(out.iter().all(|(to, _, _)| *to == Recipient::Player(HOST)));
    }

    #[test]
    fn the_percentiles_read_the_whole_ring_with_its_zeros() {
        let mut server = server();
        let mut world = world();
        tick(&mut world).query.samples[0] = 7 * MS;
        let out = run(&mut server, &mut world, HOST, "tick query");
        assert_eq!(
            texts(&out)[2],
            "Percentiles: P50: 0.0ms P95: 0.0ms P99: 7.0ms. Sample: 100"
        );
    }

    #[test]
    fn rate_sets_the_target_and_reports_float_bounds() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        server.op(FRIEND);
        let mut world = world();
        let out = run(&mut server, &mut world, HOST, "tick rate 40");
        assert_eq!(
            out,
            vec![
                (
                    Recipient::Player(HOST),
                    "Set the target tick rate to 40.0 per second".to_owned(),
                    WHITE
                ),
                (
                    Recipient::Player(FRIEND),
                    "[Steve: Set the target tick rate to 40.0 per second]".to_owned(),
                    GRAY
                ),
            ]
        );
        assert_eq!(tick(&mut world).query.rate, 40.0);
        assert_eq!(result(&mut world, "tick rate 12.5"), Ok(12));
        assert_eq!(tick(&mut world).query.rate, 12.5);
        let low = run(&mut server, &mut world, HOST, "tick rate 0.5");
        assert_eq!(
            texts(&low),
            vec![
                "Float must not be less than 1.0: found 0.5",
                "tick rate 0.5<--[HERE]"
            ]
        );
        assert_eq!(low[0].2, RED);
        let big = run(&mut server, &mut world, HOST, "tick rate 20000");
        assert_eq!(
            texts(&big)[0],
            "Float must not be more than 10000.0: found 20000.0"
        );
        assert_eq!(tick(&mut world).query.rate, 12.5);
    }

    #[test]
    fn steps_need_a_frozen_game_and_stop_once() {
        let mut server = server();
        let mut world = world();
        let out = run(&mut server, &mut world, HOST, "tick step");
        assert_eq!(
            texts(&out),
            vec!["Unable to step the game - the game must be frozen first"]
        );
        assert_eq!(out[0].2, RED);
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick freeze")),
            vec!["The game is frozen"]
        );
        assert_eq!(result(&mut world, "tick unfreeze"), Ok(0));
        assert_eq!(result(&mut world, "tick freeze"), Ok(1));
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick step")),
            vec!["Stepping 1 tick(s)"]
        );
        assert_eq!(tick(&mut world).stepping, 1);
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick step 1s")),
            vec!["Stepping 20 tick(s)"]
        );
        assert_eq!(tick(&mut world).stepping, 20);
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick step stop")),
            vec!["Interrupted the current tick step"]
        );
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick step stop")),
            vec!["No tick step in progress"]
        );
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick step 0")),
            vec![
                "The tick count must not be less than 1: found 0",
                "...ick step 0<--[HERE]"
            ]
        );
        run(&mut server, &mut world, HOST, "tick step 5");
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick unfreeze")),
            vec!["The game is running normally"]
        );
        assert_eq!(tick(&mut world).stepping, 5, "unfreezing keeps the steps");
        assert!(!tick(&mut world).query.frozen);
    }

    #[test]
    fn a_second_sprint_interrupts_the_first_without_a_report() {
        let mut server = server();
        let mut world = world();
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick sprint 60s")),
            vec!["The game is sprinting"]
        );
        assert_eq!(tick(&mut world).sprinting, 1200);
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick sprint 1d")),
            vec![
                "Interrupted the current tick sprint",
                "The game is sprinting"
            ]
        );
        assert_eq!(tick(&mut world).sprinting, 24_000);
        assert_eq!(result(&mut world, "tick sprint 3d"), Ok(1));
    }

    #[test]
    fn stopping_a_sprint_reports_it_from_the_server_first() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick sprint stop")),
            vec!["No tick sprint in progress"]
        );
        run(&mut server, &mut world, HOST, "tick sprint 3d");
        let out = run(&mut server, &mut world, HOST, "tick sprint stop");
        assert_eq!(
            out,
            vec![
                (
                    Recipient::Player(HOST),
                    "[Server: Sprint completed with 2000 ticks per second, or 0.50 ms per tick]"
                        .to_owned(),
                    GRAY
                ),
                (
                    Recipient::Player(HOST),
                    "Interrupted the current tick sprint".to_owned(),
                    WHITE
                ),
            ]
        );
        assert_eq!(
            texts(&run(&mut server, &mut world, HOST, "tick sprint stop")),
            vec!["No tick sprint in progress"]
        );
    }

    #[test]
    fn freezing_stops_a_sprint_with_its_report_and_any_steps() {
        let mut server = server();
        let mut world = world();
        run(&mut server, &mut world, HOST, "tick sprint 60s");
        let out = run(&mut server, &mut world, HOST, "tick freeze");
        assert_eq!(
            texts(&out),
            vec![
                "[Server: Sprint completed with 2000 ticks per second, or 0.50 ms per tick]",
                "The game is frozen"
            ]
        );
        assert_eq!(tick(&mut world).sprinting, 0);
        run(&mut server, &mut world, HOST, "tick step 10");
        let out = run(&mut server, &mut world, HOST, "tick freeze");
        assert_eq!(texts(&out), vec!["The game is frozen"]);
        assert_eq!(tick(&mut world).stepping, 0);
    }

    #[test]
    fn server_reports_stay_silent_without_command_feedback() {
        let mut server = server();
        let mut world = world();
        world
            .rules
            .set(crate::server::SEND_COMMAND_FEEDBACK, "false")
            .unwrap();
        run(&mut server, &mut world, HOST, "tick sprint 60s");
        assert!(run(&mut server, &mut world, HOST, "tick sprint stop").is_empty());
        assert_eq!(tick(&mut world).sprinting, 0);
    }

    #[test]
    fn tick_needs_level_three_and_a_world_that_ticks() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        op_at(&mut server, "Alex", 2);
        let out = run(&mut server, &mut world, FRIEND, "tick query");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
        op_at(&mut server, "Alex", 3);
        let out = run(&mut server, &mut world, FRIEND, "tick query");
        assert_eq!(texts(&out)[0], "The game is running normally");
        let mut plain = FakeWorld::default();
        let out = run(&mut server, &mut plain, HOST, "tick freeze");
        assert_eq!(
            texts(&out),
            vec!["Unknown or incomplete command. See below for error"]
        );
    }

    #[test]
    fn tick_suggests_its_subcommands_rates_and_times() {
        assert_eq!(
            suggestions("/tick "),
            ["freeze", "query", "rate", "sprint", "step", "unfreeze"]
        );
        assert_eq!(suggestions("/tick rate "), ["20"]);
        assert_eq!(suggestions("/tick step "), ["1s", "1t", "stop"]);
        assert_eq!(suggestions("/tick sprint "), ["1d", "3d", "60s", "stop"]);
        assert_eq!(suggestions("/tick sprint 6"), ["60s"]);
        assert!(suggestions("/tick sprint 60s").is_empty());
    }
}
