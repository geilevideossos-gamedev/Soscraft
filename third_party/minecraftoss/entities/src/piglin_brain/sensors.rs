use super::{PiglinBrain, PiglinCtx};
use crate::brain_behaviors::{Living, block_of, find, nearest_block, set_list, targetable_within};
use crate::monster_ai::Target;
use crate::piglin::PiglinKind;
use glam::DVec3;

pub const SCAN_RATE: i32 = 20;
pub const SENSORS: usize = 5;
pub const NEAREST_LIVING: usize = 0;
pub const NEAREST_PLAYERS: usize = 1;
pub const SPECIFIC: usize = 4;
pub const NEMESES: [&str; 2] = ["minecraft:wither_skeleton", "minecraft:wither"];
pub const ZOMBIFIED: [&str; 2] = ["minecraft:zombified_piglin", "minecraft:zoglin"];
pub const REPELLENTS: &str = "minecraft:piglin_repellents";

pub fn repellent(world: &dyn minecraftoss_player::World, pos: crate::brain_behaviors::Pos) -> bool {
    world.block_in_tag(pos, REPELLENTS)
        && world.block(pos).is_none_or(|block| {
            block.id != "minecraft:soul_campfire" || block.property("lit") == Some("true")
        })
}

pub fn adult_piglin(other: &Living) -> bool {
    match other.kind {
        "minecraft:piglin" => !other.baby,
        "minecraft:piglin_brute" => true,
        _ => false,
    }
}

impl PiglinBrain {
    pub(super) fn sense(&mut self, ctx: &mut PiglinCtx) {
        if !self.started {
            self.started = true;
            for clock in &mut self.sensors {
                *clock = i64::from(ctx.random.next_int(SCAN_RATE as u32));
            }
        }
        let mut fired = [false; SENSORS];
        for (clock, fired) in self.sensors.iter_mut().zip(fired.iter_mut()) {
            *clock -= 1;
            if *clock <= 0 {
                *clock = i64::from(SCAN_RATE);
                *fired = true;
            }
        }
        if fired[NEAREST_LIVING] {
            self.sense_living(ctx);
        }
        if fired[NEAREST_PLAYERS] {
            self.sense_players(ctx);
        }
        if fired[SPECIFIC] {
            self.sense_specific(ctx);
        }
    }

    fn sense_specific(&mut self, ctx: &PiglinCtx) {
        let visible: Vec<&Living> = self
            .memories
            .visible_living
            .get()
            .into_iter()
            .flatten()
            .filter_map(|&target| find(ctx.others, target))
            .collect();
        let nearby: Vec<Target> = self
            .memories
            .nearest_living
            .get()
            .into_iter()
            .flatten()
            .filter_map(|&target| find(ctx.others, target))
            .filter(|other| adult_piglin(other))
            .map(|other| other.target)
            .collect();
        if ctx.kind == PiglinKind::Brute {
            let nemesis = visible
                .iter()
                .find(|other| NEMESES.contains(&other.kind))
                .map(|other| other.target);
            self.memories.nearest_visible_nemesis.set_or_erase(nemesis);
            set_list(&mut self.memories.nearby_adult_piglins, nearby);
            return;
        }
        let mut nemesis = None;
        let mut zombified = None;
        let mut ungilded = None;
        let mut holding = None;
        let mut adults = Vec::new();
        let mut hoglins = 0;
        for other in visible {
            match other.kind {
                "minecraft:hoglin" => {
                    if !other.baby {
                        hoglins += 1;
                    }
                }
                "minecraft:piglin" | "minecraft:piglin_brute" => {
                    if adult_piglin(other) {
                        adults.push(other.target);
                    }
                }
                _ if other.is_player() => {
                    let attackable = other.alive && other.attackable && !other.spectator;
                    if ungilded.is_none() && attackable && !ctx.gilded.contains(&other.target) {
                        ungilded = Some(other.target);
                    }
                    if holding.is_none()
                        && !other.spectator
                        && ctx.loved_holders.contains(&other.target)
                    {
                        holding = Some(other.target);
                    }
                }
                kind if nemesis.is_some() || !NEMESES.contains(&kind) => {
                    if zombified.is_none() && ZOMBIFIED.contains(&kind) {
                        zombified = Some(other.target);
                    }
                }
                _ => nemesis = Some(other.target),
            }
        }
        let memories = &mut self.memories;
        memories.nearest_visible_nemesis.set_or_erase(nemesis);
        memories.nearest_visible_zombified.set_or_erase(zombified);
        memories
            .nearest_targetable_player_not_wearing_gold
            .set_or_erase(ungilded);
        memories
            .nearest_player_holding_wanted_item
            .set_or_erase(holding);
        memories.visible_adult_piglin_count.set(adults.len());
        memories.visible_adult_hoglin_count.set(hoglins);
        set_list(&mut memories.nearest_visible_adult_piglins, adults);
        set_list(&mut memories.nearby_adult_piglins, nearby);
        let repellent = nearest_block(ctx.world, block_of(ctx.position()), repellent);
        memories.nearest_repellent.set_or_erase(repellent);
    }

    fn sense_living(&mut self, ctx: &PiglinCtx) {
        let position = ctx.position();
        let eye = ctx.eye();
        let range = ctx.follow_range;
        let half = f64::from(ctx.body.width) / 2.0;
        let min = position - DVec3::new(half + range, range, half + range);
        let max = position
            + DVec3::new(
                half + range,
                f64::from(ctx.body.height) + range,
                half + range,
            );
        let mut found: Vec<(&Living, f64)> = ctx
            .others
            .iter()
            .filter(|other| other.alive && !other.spectator)
            .filter(|other| {
                let h = f64::from(other.width) / 2.0;
                let p = other.position;
                p.x - h < max.x
                    && p.x + h > min.x
                    && p.y < max.y
                    && p.y + f64::from(other.height) > min.y
                    && p.z - h < max.z
                    && p.z + h > min.z
            })
            .map(|other| (other, other.position.distance_squared(position)))
            .collect();
        found.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let visible: Vec<Target> = found
            .iter()
            .filter(|(other, _)| targetable_within(ctx.world, eye, position, other, false, range))
            .map(|(other, _)| other.target)
            .collect();
        set_list(
            &mut self.memories.nearest_living,
            found.iter().map(|(other, _)| other.target).collect(),
        );
        self.memories.visible_living.set(visible);
    }

    fn sense_players(&mut self, ctx: &PiglinCtx) {
        let position = ctx.position();
        let eye = ctx.eye();
        let range = ctx.follow_range;
        let mut players: Vec<(&Living, f64)> = ctx
            .others
            .iter()
            .filter(|other| other.is_player() && other.alive && !other.spectator)
            .map(|other| (other, other.position.distance_squared(position)))
            .filter(|(_, distance)| *distance < range * range)
            .collect();
        players.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        set_list(
            &mut self.memories.nearest_players,
            players.iter().map(|(other, _)| other.target).collect(),
        );
        let visible: Vec<&Living> = players
            .iter()
            .filter(|(other, _)| targetable_within(ctx.world, eye, position, other, false, range))
            .map(|(other, _)| *other)
            .collect();
        self.memories
            .nearest_visible_player
            .set_or_erase(visible.first().map(|other| other.target));
        let attackable = visible
            .iter()
            .find(|other| targetable_within(ctx.world, eye, position, other, true, range))
            .map(|other| other.target);
        self.memories
            .nearest_visible_attackable_player
            .set_or_erase(attackable);
    }
}
