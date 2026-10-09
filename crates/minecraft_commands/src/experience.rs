use crate::args::{Arg, ArgKind};
use crate::permissions::GAMEMASTERS;
use crate::reader::{CommandError, literal};
use crate::server::Ctx;
use crate::text::Text;
use crate::tracker::Tracker;
use crate::tree::{Node, Redirect};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Experience {
    pub level: i32,
    pub progress: f32,
    pub total: i32,
}

pub fn floor(value: f32) -> i32 {
    let truncated = value as i32;
    if value < truncated as f32 {
        truncated - 1
    } else {
        truncated
    }
}

impl Experience {
    pub fn needed(&self) -> i32 {
        if self.level >= 30 {
            (self.level - 30).wrapping_mul(9).wrapping_add(112)
        } else if self.level >= 15 {
            37 + (self.level - 15) * 5
        } else {
            7 + self.level * 2
        }
    }

    pub fn points(&self) -> i32 {
        floor(self.progress * self.needed() as f32)
    }

    pub fn give_points(&mut self, points: i32) {
        if points == 0 {
            return;
        }
        self.progress += points as f32 / self.needed() as f32;
        self.total = self.total.wrapping_add(points).clamp(0, i32::MAX);
        while self.progress < 0.0 {
            let rest = self.progress * self.needed() as f32;
            if self.level > 0 {
                self.give_levels(-1);
                self.progress = 1.0 + rest / self.needed() as f32;
            } else {
                self.give_levels(-1);
                self.progress = 0.0;
            }
        }
        while self.progress >= 1.0 {
            self.progress = (self.progress - 1.0) * self.needed() as f32;
            self.give_levels(1);
            self.progress /= self.needed() as f32;
        }
    }

    pub fn give_levels(&mut self, levels: i32) {
        if levels == 0 {
            return;
        }
        self.level = self.level.saturating_add(levels);
        if self.level < 0 {
            self.level = 0;
            self.progress = 0.0;
            self.total = 0;
        }
    }

    pub fn set_points(&mut self, points: i32) -> bool {
        if points >= self.needed() {
            return false;
        }
        let needed = self.needed() as f32;
        let most = (needed - 1.0) / needed;
        self.progress = (points as f32 / needed).clamp(0.0, most);
        true
    }

    pub fn set_levels(&mut self, levels: i32) {
        self.level = levels;
    }
}

pub(crate) fn node() -> Node {
    let players = ArgKind::Players { single: false };
    let amount = |min: i32, exec: crate::tree::Exec| {
        Node::argument("amount", ArgKind::Int { min, max: i32::MAX })
            .executes(exec)
            .then(Node::literal("points").executes(exec))
            .then(Node::literal("levels").executes(exec))
    };
    Node::literal("experience")
        .requires(GAMEMASTERS)
        .then(
            Node::literal("add")
                .then(Node::argument("target", players).then(amount(i32::MIN, add))),
        )
        .then(Node::literal("set").then(Node::argument("target", players).then(amount(0, set))))
        .then(
            Node::literal("query").then(
                Node::argument("target", ArgKind::Players { single: true })
                    .then(Node::literal("points").executes(query))
                    .then(Node::literal("levels").executes(query)),
            ),
        )
}

pub(crate) fn alias() -> Node {
    Node::literal("xp")
        .requires(GAMEMASTERS)
        .redirect(Redirect::Command("experience"))
}

fn unit(ctx: &Ctx) -> &'static str {
    if ctx.literals.contains(&"levels") {
        "levels"
    } else {
        "points"
    }
}

fn amount(ctx: &Ctx) -> i32 {
    match ctx.arg("amount") {
        Some(Arg::Int(amount)) => *amount,
        _ => 0,
    }
}

fn change(
    ctx: &mut Ctx,
    apply: impl Fn(&mut Experience, i32) -> bool,
) -> Result<Tracker, CommandError> {
    let amount = amount(ctx);
    let mut tracker = Tracker::default();
    for player in ctx.targets("target")? {
        let Some(mut experience) = ctx.world.experience(player) else {
            continue;
        };
        let changed = apply(&mut experience, amount);
        if changed && !ctx.world.set_experience(player, experience) {
            continue;
        }
        tracker.track(player, i32::from(changed));
    }
    if tracker.count(false) == 0 {
        return Err(CommandError::new(
            "argument.entity.notfound.player",
            Vec::new(),
        ));
    }
    Ok(tracker)
}

fn respond(ctx: &mut Ctx, tracker: Tracker, key: &str) {
    let amount = literal(amount(ctx));
    let text = match tracker.single(true) {
        Some(only) => Text::translate(
            format!("{key}.success.single"),
            vec![amount, ctx.display_name(only)],
        ),
        None => Text::translate(
            format!("{key}.success.multiple"),
            vec![amount, literal(tracker.count(true))],
        ),
    };
    ctx.success(text, true);
}

fn add(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let levels = unit(ctx) == "levels";
    let tracker = change(ctx, |experience, amount| {
        if levels {
            experience.give_levels(amount);
        } else {
            experience.give_points(amount);
        }
        true
    })?;
    let key = format!("commands.experience.add.{}", unit(ctx));
    respond(ctx, tracker, &key);
    Ok(tracker.total)
}

fn set(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let levels = unit(ctx) == "levels";
    let tracker = change(ctx, |experience, amount| {
        if levels {
            experience.set_levels(amount);
            true
        } else {
            experience.set_points(amount)
        }
    })?;
    if tracker.count(true) == 0 {
        return Err(CommandError::new(
            "commands.experience.set.points.invalid",
            Vec::new(),
        ));
    }
    let key = format!("commands.experience.set.{}", unit(ctx));
    respond(ctx, tracker, &key);
    Ok(tracker.total)
}

fn query(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let player = ctx.targets("target")?[0];
    let Some(experience) = ctx.world.experience(player) else {
        return Err(CommandError::new(
            "argument.entity.notfound.player",
            Vec::new(),
        ));
    };
    let unit = unit(ctx);
    let value = if unit == "levels" {
        experience.level
    } else {
        experience.points()
    };
    let text = Text::translate(
        format!("commands.experience.query.{unit}"),
        vec![ctx.display_name(player), literal(value)],
    );
    ctx.success(text, false);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Recipient;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};
    use crate::text::RED;

    fn at(level: i32, progress: f32, total: i32) -> Experience {
        Experience {
            level,
            progress,
            total,
        }
    }

    #[test]
    fn points_cross_level_thresholds_like_vanilla() {
        let mut experience = Experience::default();
        experience.give_points(7);
        assert_eq!(experience, at(1, 0.0, 7));
        experience.give_points(4);
        assert_eq!((experience.level, experience.total), (1, 11));
        assert!((experience.progress - 4.0 / 9.0).abs() < 1e-6);
        assert_eq!(experience.points(), 4);
        experience.give_points(-6);
        assert_eq!(
            (experience.level, experience.points(), experience.total),
            (0, 4, 5)
        );
        experience.give_points(-100);
        assert_eq!(experience, at(0, 0.0, 0));
        let mut big = Experience::default();
        big.give_points(1395);
        assert_eq!((big.level, big.points()), (30, 0));
        assert_eq!(
            [0, 15, 30, 31].map(|level| at(level, 0.0, 0).needed()),
            [7, 37, 112, 121]
        );
    }

    #[test]
    fn levels_saturate_and_reset_below_zero() {
        let mut experience = at(5, 0.5, 100);
        experience.give_levels(-3);
        assert_eq!(experience, at(2, 0.5, 100));
        experience.give_levels(-3);
        assert_eq!(experience, at(0, 0.0, 0));
        let mut top = at(i32::MAX - 1, 0.0, 0);
        top.give_levels(5);
        assert_eq!(top.level, i32::MAX);
        assert_eq!(top.needed(), 2_147_483_481);
        assert_eq!(at(477_218_606, 0.0, 0).needed(), 0);
        assert_eq!(at(1_000_000_000, 0.0, 0).needed(), 410_065_250);
        top.give_points(5);
        assert_eq!((top.level, top.total), (i32::MAX, 5));
    }

    #[test]
    fn setting_points_stays_below_the_next_level() {
        let mut experience = at(3, 0.0, 0);
        assert!(!experience.set_points(13));
        assert!(experience.set_points(12));
        assert_eq!(experience.points(), 12);
        assert!(experience.set_points(5));
        assert_eq!(experience.points(), 5);
        assert!(experience.set_points(0));
        assert_eq!(experience.progress, 0.0);
        assert_eq!([-0.5, 0.0, 2.9].map(floor), [-1, 0, 2]);
    }

    fn world() -> FakeWorld {
        FakeWorld {
            experience: vec![(HOST, Experience::default())],
            ..FakeWorld::default()
        }
    }

    #[test]
    fn xp_adds_sets_and_queries_like_vanilla() {
        let mut server = server();
        let mut world = world();
        let out = run(&mut server, &mut world, HOST, "xp add @s 5 levels");
        assert_eq!(texts(&out), vec!["Gave 5 experience levels to Steve"]);
        let out = run(&mut server, &mut world, HOST, "xp query @s levels");
        assert_eq!(texts(&out), vec!["Steve has 5 experience levels"]);
        let out = run(&mut server, &mut world, HOST, "experience add Steve 10");
        assert_eq!(texts(&out), vec!["Gave 10 experience points to Steve"]);
        let out = run(&mut server, &mut world, HOST, "xp query @s points");
        assert_eq!(texts(&out), vec!["Steve has 10 experience points"]);
        let out = run(&mut server, &mut world, HOST, "xp set @s 30 points");
        assert_eq!(
            texts(&out),
            vec![
                "Cannot set experience points above the maximum points for the player's current level"
            ]
        );
        assert_eq!(out[0].2, RED);
        let out = run(&mut server, &mut world, HOST, "xp set @s 16");
        assert_eq!(texts(&out), vec!["Set 16 experience points on Steve"]);
        let out = run(&mut server, &mut world, HOST, "xp set @s 30 levels");
        assert_eq!(texts(&out), vec!["Set 30 experience levels on Steve"]);
        assert_eq!(world.experience[0].1.level, 30);
        let out = run(&mut server, &mut world, HOST, "xp add @s -40 levels");
        assert_eq!(texts(&out), vec!["Gave -40 experience levels to Steve"]);
        assert_eq!(world.experience[0].1, Experience::default());
        let out = run(&mut server, &mut world, HOST, "xp set @s -1");
        assert_eq!(out.len(), 2);
        let out = run(&mut server, &mut world, HOST, "xp query @s");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
        run(&mut server, &mut world, HOST, "xp set @s 2147483647 levels");
        let out = run(&mut server, &mut world, HOST, "xp query @s points");
        assert_eq!(texts(&out), vec!["Steve has 0 experience points"]);
        let out = run(&mut server, &mut world, HOST, "xp set @s 100 points");
        assert_eq!(texts(&out), vec!["Set 100 experience points on Steve"]);
    }

    #[test]
    fn xp_counts_players_and_reports_missing_ones() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        world.experience.push((FRIEND, Experience::default()));
        let out = run(&mut server, &mut world, HOST, "xp add @a 3 levels");
        assert_eq!(texts(&out), vec!["Gave 3 experience levels to 2 players"]);
        assert_eq!(out[0].0, Recipient::Player(HOST));
        world.experience[0].1.level = 0;
        let out = run(&mut server, &mut world, HOST, "xp set @a 8 points");
        assert_eq!(texts(&out), vec!["Set 8 experience points on Alex"]);
        let out = run(&mut server, &mut world, HOST, "xp query @a points");
        assert_eq!(out.len(), 2);
        let out = run(&mut server, &mut FakeWorld::default(), HOST, "xp add @s 1");
        assert_eq!(texts(&out), vec!["No player was found"]);
        let out = run(&mut server, &mut world, FRIEND, "xp add @s 1");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
    }
}
