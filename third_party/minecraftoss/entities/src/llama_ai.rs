use crate::cow_ai::{CowGoalContext, CowGoalEffect};
use crate::goals::{Control, Controls, Goal};
use crate::horse::{CARAVAN_SPEED, LlamaFight};
use crate::monster_ai::Target;
use glam::DVec3;
use minecraftoss_player::World;

pub const SPIT_INTERVAL: i32 = 40;
pub const SPIT_RADIUS: f32 = 20.0;
pub const SPIT_SPEED: f64 = 1.25;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const WOLF_RANGE: f64 = FOLLOW_RANGE * 0.25;
pub const CARAVAN_LIMIT: i32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub target: Target,
    pub position: DVec3,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub alive: bool,
    pub attackable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foe {
    Player,
    Wolf { tame: bool },
    Zombie,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caravaner {
    pub id: u64,
    pub position: DVec3,
    pub width: f32,
    pub height: f32,
    pub leashed: bool,
    pub head: Option<u64>,
    pub has_tail: bool,
    pub alive: bool,
}

#[derive(Clone, Debug, Default)]
pub struct LlamaContext {
    pub id: u64,
    pub trader: bool,
    pub leashed: bool,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub fight: LlamaFight,
    pub seen: Vec<(Seen, Foe)>,
    pub llamas: Vec<Caravaner>,
    pub trader_attacker: Option<(Target, i32)>,
}

impl LlamaContext {
    pub fn info(&self, target: Target) -> Option<Seen> {
        self.seen
            .iter()
            .find(|(seen, _)| seen.target == target)
            .map(|(seen, _)| *seen)
    }

    fn caravaner(&self, id: u64) -> Option<Caravaner> {
        if id == self.id {
            return Some(Caravaner {
                id,
                position: DVec3::ZERO,
                width: self.width,
                height: self.height,
                leashed: self.leashed,
                head: self.fight.caravan_head,
                has_tail: self.fight.caravan_tail.is_some(),
                alive: true,
            });
        }
        self.llamas.iter().find(|l| l.id == id).copied()
    }

    pub fn first_is_leashed(&self, llama: u64, depth: i32) -> bool {
        if depth > CARAVAN_LIMIT {
            return false;
        }
        let Some(head) = self.caravaner(llama).and_then(|l| l.head) else {
            return false;
        };
        match self.caravaner(head) {
            Some(h) if h.leashed => true,
            Some(_) => self.first_is_leashed(head, depth + 1),
            None => false,
        }
    }
}

fn eye(ctx: &CowGoalContext, llama: &LlamaContext) -> DVec3 {
    ctx.child.position + DVec3::new(0.0, f64::from(llama.eye_height), 0.0)
}

fn sees(ctx: &CowGoalContext, llama: &LlamaContext, world: &dyn World, seen: Seen) -> bool {
    crate::sight::line_of_sight(
        world,
        eye(ctx, llama),
        seen.position + DVec3::new(0.0, f64::from(seen.eye_height), 0.0),
    )
}

fn reduced(ticks: i32) -> i32 {
    (ticks + 1) / 2
}

fn keeps_target(
    ctx: &mut CowGoalContext,
    world: &dyn World,
    range: f64,
    must_see: bool,
    memory: i32,
) -> bool {
    let Some(llama) = ctx.llama.as_ref() else {
        return false;
    };
    let Some(seen) = llama.fight.target.and_then(|t| llama.info(t)) else {
        return false;
    };
    if !seen.alive || !seen.attackable {
        return false;
    }
    if ctx.child.position.distance_squared(seen.position) > range * range {
        return false;
    }
    if must_see {
        let visible = sees(ctx, llama, world, seen);
        let llama = ctx.llama.as_mut().expect("checked");
        if visible {
            llama.fight.unseen_ticks = 0;
        } else {
            llama.fight.unseen_ticks += 1;
            if llama.fight.unseen_ticks > reduced(memory) {
                return false;
            }
        }
    }
    true
}

#[derive(Clone)]
pub(crate) struct HurtByGoal;
impl Goal<CowGoalContext> for HurtByGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Target])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        let Some(llama) = ctx.llama.as_ref() else {
            return false;
        };
        let Some((attacker, stamp)) = llama.fight.hurt_by else {
            return false;
        };
        stamp != llama.fight.hurt_seen
            && llama
                .info(attacker)
                .is_some_and(|seen| seen.alive && seen.attackable)
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        if let Some(llama) = ctx.llama.as_mut() {
            if llama.fight.did_spit {
                llama.fight.did_spit = false;
                return false;
            }
        }
        keeps_target(ctx, world, FOLLOW_RANGE, true, 300)
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        let Some(llama) = ctx.llama.as_mut() else {
            return;
        };
        if let Some((attacker, stamp)) = llama.fight.hurt_by {
            llama.fight.target = Some(attacker);
            llama.fight.hurt_seen = stamp;
            llama.fight.unseen_ticks = 0;
        }
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        if let Some(llama) = ctx.llama.as_mut() {
            llama.fight.target = None;
        }
    }
}

#[derive(Clone)]
pub(crate) struct NearestFoeGoal {
    pub(crate) zombies: bool,
    pub(crate) interval: u32,
    pub(crate) range: f64,
    pub(crate) must_see: bool,
    pub(crate) found: Option<Target>,
}

impl NearestFoeGoal {
    pub(crate) fn wolves() -> Self {
        Self {
            zombies: false,
            interval: reduced(16) as u32,
            range: WOLF_RANGE,
            must_see: false,
            found: None,
        }
    }

    pub(crate) fn zombies() -> Self {
        Self {
            zombies: true,
            interval: reduced(10) as u32,
            range: FOLLOW_RANGE,
            must_see: true,
            found: None,
        }
    }

    fn wanted(&self, foe: Foe) -> bool {
        match foe {
            Foe::Wolf { tame } => !self.zombies && !tame,
            Foe::Zombie => self.zombies,
            Foe::Player | Foe::Other => false,
        }
    }
}

impl Goal<CowGoalContext> for NearestFoeGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Target])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        if ctx.llama.is_none() || ctx.random.next_int(self.interval) != 0 {
            return false;
        }
        let llama = ctx.llama.as_ref().expect("checked");
        let p = ctx.child.position;
        let half = f64::from(llama.width / 2.0);
        let (min, max) = (
            DVec3::new(p.x - half - self.range, p.y - 4.0, p.z - half - self.range),
            DVec3::new(
                p.x + half + self.range,
                p.y + f64::from(llama.height) + 4.0,
                p.z + half + self.range,
            ),
        );
        let from = eye(ctx, llama);
        let mut best: Option<(f64, Target)> = None;
        for (seen, foe) in &llama.seen {
            if !self.wanted(*foe) || !seen.alive || !seen.attackable {
                continue;
            }
            let q = seen.position;
            let h = f64::from(seen.width / 2.0);
            let inside = q.x - h < max.x
                && q.x + h > min.x
                && q.y < max.y
                && q.y + f64::from(seen.height) > min.y
                && q.z - h < max.z
                && q.z + h > min.z;
            if !inside || p.distance_squared(q) > self.range * self.range {
                continue;
            }
            if self.must_see && !sees(ctx, llama, world, *seen) {
                continue;
            }
            let distance = from.distance_squared(q);
            if best.is_none_or(|(d, _)| distance < d) {
                best = Some((distance, seen.target));
            }
        }
        self.found = best.map(|(_, target)| target);
        self.found.is_some()
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        keeps_target(ctx, world, self.range, self.must_see, 60)
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        if let Some(llama) = ctx.llama.as_mut() {
            llama.fight.target = self.found;
            llama.fight.unseen_ticks = 0;
        }
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        if let Some(llama) = ctx.llama.as_mut() {
            llama.fight.target = None;
        }
        self.found = None;
    }
}

#[derive(Clone)]
pub(crate) struct DefendTraderGoal;
impl Goal<CowGoalContext> for DefendTraderGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Target])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        let Some(llama) = ctx.llama.as_ref() else {
            return false;
        };
        let Some((attacker, stamp)) = llama.trader_attacker else {
            return false;
        };
        stamp != llama.fight.defend_seen
            && llama
                .info(attacker)
                .is_some_and(|seen| seen.alive && seen.attackable)
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        keeps_target(ctx, world, FOLLOW_RANGE, false, 60)
    }
    fn start(&mut self, ctx: &mut CowGoalContext) {
        let Some(llama) = ctx.llama.as_mut() else {
            return;
        };
        if let Some((attacker, stamp)) = llama.trader_attacker {
            llama.fight.target = Some(attacker);
            llama.fight.defend_seen = stamp;
            llama.fight.unseen_ticks = 0;
        }
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        if let Some(llama) = ctx.llama.as_mut() {
            llama.fight.target = None;
        }
    }
}

#[derive(Clone)]
pub(crate) struct NoTargetGoal;
impl Goal<CowGoalContext> for NoTargetGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Target])
    }
    fn can_start(&mut self, _ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        false
    }
}

#[derive(Clone)]
pub(crate) struct SpitGoal {
    pub(crate) target: Option<Target>,
}
impl Goal<CowGoalContext> for SpitGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move, Control::Look])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        let Some(llama) = ctx.llama.as_ref() else {
            return false;
        };
        match llama.fight.target.and_then(|t| llama.info(t)) {
            Some(seen) if seen.alive => {
                self.target = Some(seen.target);
                true
            }
            _ => false,
        }
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, world: &dyn World) -> bool {
        if self.can_start(ctx, world) {
            return true;
        }
        let alive = ctx.llama.as_ref().is_some_and(|llama| {
            self.target
                .and_then(|t| llama.info(t))
                .is_some_and(|seen| seen.alive)
        });
        alive && !ctx.navigation_done
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        self.target = None;
        if let Some(llama) = ctx.llama.as_mut() {
            llama.fight.see_time = 0;
            llama.fight.attack_time = -1;
        }
    }
    fn tick_world(&mut self, ctx: &mut CowGoalContext, world: &dyn World) {
        let Some(llama) = ctx.llama.as_ref() else {
            return;
        };
        let Some(seen) = self.target.and_then(|t| llama.info(t)) else {
            return;
        };
        let distance_sq = ctx.child.position.distance_squared(seen.position);
        let visible = sees(ctx, llama, world, seen);
        let llama = ctx.llama.as_mut().expect("checked");
        if visible {
            llama.fight.see_time += 1;
        } else {
            llama.fight.see_time = 0;
        }
        let radius_sq = f64::from(SPIT_RADIUS * SPIT_RADIUS);
        if distance_sq <= radius_sq && llama.fight.see_time >= 5 {
            ctx.effects.push(CowGoalEffect::StopNavigation);
        } else {
            ctx.effects.push(CowGoalEffect::NavigateToEntity {
                target: seen.position,
                speed: SPIT_SPEED,
            });
        }
        ctx.effects.push(CowGoalEffect::LookAt {
            target: seen.position + DVec3::new(0.0, f64::from(seen.eye_height), 0.0),
            y_max: 30.0,
            x_max: 30.0,
        });
        llama.fight.attack_time -= 1;
        if llama.fight.attack_time == 0 {
            if !visible {
                return;
            }
            let aim = seen.position + DVec3::new(0.0, f64::from(seen.height) / 3.0, 0.0);
            ctx.effects.push(CowGoalEffect::Spit { aim });
            llama.fight.did_spit = true;
            llama.fight.attack_time = SPIT_INTERVAL;
        } else if llama.fight.attack_time < 0 {
            llama.fight.attack_time = SPIT_INTERVAL;
        }
    }
}

#[derive(Clone)]
pub(crate) struct CaravanGoal;
impl Goal<CowGoalContext> for CaravanGoal {
    fn controls(&self) -> Controls {
        Controls::new(&[Control::Move])
    }
    fn can_start(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        let Some(llama) = ctx.llama.as_ref() else {
            return false;
        };
        if llama.leashed || llama.fight.caravan_head.is_some() {
            return false;
        }
        let p = ctx.child.position;
        let half = f64::from(llama.width / 2.0);
        let (min, max) = (
            DVec3::new(p.x - half - 9.0, p.y - 4.0, p.z - half - 9.0),
            DVec3::new(
                p.x + half + 9.0,
                p.y + f64::from(llama.height) + 4.0,
                p.z + half + 9.0,
            ),
        );
        let near: Vec<Caravaner> = llama
            .llamas
            .iter()
            .filter(|l| l.id != llama.id && l.alive)
            .filter(|l| {
                let q = l.position;
                let h = f64::from(l.width / 2.0);
                q.x - h < max.x
                    && q.x + h > min.x
                    && q.y < max.y
                    && q.y + f64::from(l.height) > min.y
                    && q.z - h < max.z
                    && q.z + h > min.z
            })
            .copied()
            .collect();
        let nearest = |accept: &dyn Fn(&Caravaner) -> bool| {
            let mut best: Option<(f64, Caravaner)> = None;
            for l in near.iter().filter(|l| accept(l)) {
                let d = p.distance_squared(l.position);
                if best.is_none_or(|(b, _)| d <= b) {
                    best = Some((d, *l));
                }
            }
            best
        };
        let found = nearest(&|l| l.head.is_some() && !l.has_tail)
            .or_else(|| nearest(&|l| l.leashed && !l.has_tail));
        let Some((distance, head)) = found else {
            return false;
        };
        if distance < 4.0 {
            return false;
        }
        if !head.leashed && !llama.first_is_leashed(head.id, 1) {
            return false;
        }
        let llama = ctx.llama.as_mut().expect("checked");
        llama.fight.caravan_head = Some(head.id);
        ctx.effects.push(CowGoalEffect::JoinCaravan(head.id));
        true
    }
    fn can_continue(&mut self, ctx: &mut CowGoalContext, _world: &dyn World) -> bool {
        let Some(llama) = ctx.llama.as_ref() else {
            return false;
        };
        let Some(head) = llama
            .fight
            .caravan_head
            .and_then(|id| llama.llamas.iter().find(|l| l.id == id).copied())
        else {
            return false;
        };
        if !head.alive || !llama.first_is_leashed(llama.id, 0) {
            return false;
        }
        let distance = ctx.child.position.distance_squared(head.position);
        let llama = ctx.llama.as_mut().expect("checked");
        if distance > 676.0 {
            if llama.fight.caravan_speed <= 3.0 {
                llama.fight.caravan_speed *= 1.2;
                llama.fight.dist_check = reduced(40);
                return true;
            }
            if llama.fight.dist_check == 0 {
                return false;
            }
        }
        if llama.fight.dist_check > 0 {
            llama.fight.dist_check -= 1;
        }
        true
    }
    fn stop(&mut self, ctx: &mut CowGoalContext) {
        let Some(llama) = ctx.llama.as_mut() else {
            return;
        };
        if let Some(head) = llama.fight.caravan_head.take() {
            ctx.effects.push(CowGoalEffect::LeaveCaravan(head));
        }
        llama.fight.caravan_speed = CARAVAN_SPEED;
    }
    fn tick(&mut self, ctx: &mut CowGoalContext) {
        let Some(llama) = ctx.llama.as_ref() else {
            return;
        };
        let Some(head) = llama
            .fight
            .caravan_head
            .and_then(|id| llama.llamas.iter().find(|l| l.id == id).copied())
        else {
            return;
        };
        let p = ctx.child.position;
        let distance = p.distance(head.position);
        let toward = (head.position - p).normalize_or_zero() * (distance - 2.0).max(0.0);
        ctx.effects.push(CowGoalEffect::Navigate {
            target: p + toward,
            speed: llama.fight.caravan_speed,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caravaner(id: u64, leashed: bool, head: Option<u64>) -> Caravaner {
        Caravaner {
            id,
            position: DVec3::ZERO,
            width: 0.9,
            height: 1.87,
            leashed,
            head,
            has_tail: false,
            alive: true,
        }
    }

    #[test]
    fn a_caravan_counts_only_when_its_first_llama_is_on_a_lead() {
        let mut llama = LlamaContext {
            id: 1,
            llamas: vec![caravaner(2, false, Some(3)), caravaner(3, true, None)],
            ..LlamaContext::default()
        };
        llama.fight.caravan_head = Some(2);
        assert!(llama.first_is_leashed(1, 0));
        llama.llamas[1].leashed = false;
        assert!(!llama.first_is_leashed(1, 0));
        let long: Vec<Caravaner> = (2..20)
            .map(|id| caravaner(id, id == 19, Some(id + 1)))
            .collect();
        let chain = LlamaContext {
            id: 1,
            llamas: long,
            fight: LlamaFight {
                caravan_head: Some(2),
                ..LlamaFight::default()
            },
            ..LlamaContext::default()
        };
        assert!(!chain.first_is_leashed(1, 0), "eight llamas at most");
    }
}
