use super::{Fish, FishAi};
use crate::movement::Body;
use crate::world::EntityWorld;
use crate::world::families::FamilyMob;
use glam::DVec3;

pub const REACH: f64 = 8.0;
pub const LEADER_RANGE_SQR: f64 = 121.0;
pub const LONELY_CHANCE: u32 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct School {
    pub leader: Option<u64>,
    pub size: i32,
}

impl Default for School {
    fn default() -> Self {
        Self {
            leader: None,
            size: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Schoolmate {
    pub id: u64,
    pub position: DVec3,
    pub alive: bool,
    pub follower: bool,
    pub can_be_followed: bool,
    pub size: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SchoolBoard {
    pub id: u64,
    pub max: i32,
    pub size: i32,
    pub leader: Option<Schoolmate>,
    pub mates: Vec<Schoolmate>,
    pub recruits: Option<(u64, Vec<u64>)>,
    pub stop_following: Option<u64>,
}

impl SchoolBoard {
    pub fn follower(&self) -> bool {
        self.leader.is_some_and(|leader| leader.alive)
    }

    pub fn has_followers(&self) -> bool {
        self.size > 1
    }

    pub fn in_range_of_leader(&self, position: DVec3) -> bool {
        self.leader
            .is_some_and(|leader| leader.position.distance_squared(position) <= LEADER_RANGE_SQR)
    }

    pub fn recruit(&mut self) -> bool {
        let list: Vec<Schoolmate> = self
            .mates
            .iter()
            .copied()
            .filter(|mate| mate.can_be_followed || !mate.follower)
            .collect();
        let me = Schoolmate {
            id: self.id,
            position: DVec3::ZERO,
            alive: true,
            follower: false,
            can_be_followed: false,
            size: self.size,
        };
        let leader = list
            .iter()
            .copied()
            .find(|mate| mate.can_be_followed)
            .unwrap_or(me);
        let room = (self.max - leader.size).max(0) as usize;
        let followers: Vec<u64> = list
            .iter()
            .filter(|mate| !mate.follower)
            .take(room)
            .filter(|mate| mate.id != leader.id)
            .map(|mate| mate.id)
            .collect();
        if followers.contains(&self.id) {
            self.leader = Some(leader);
        }
        if leader.id == self.id {
            self.size += followers.len() as i32;
        }
        self.recruits = Some((leader.id, followers));
        self.follower()
    }

    pub fn stop_following(&mut self) {
        if let Some(leader) = self.leader.take() {
            self.stop_following = Some(leader.id);
        }
    }
}

fn grown_box_meets(body: &Body, grown: f64, other: &Body) -> bool {
    let (p, half) = (body.position, f64::from(body.width) / 2.0 + grown);
    let (q, other_half) = (other.position, f64::from(other.width) / 2.0);
    p.x - half < q.x + other_half
        && p.x + half > q.x - other_half
        && p.y - grown < q.y + f64::from(other.height)
        && p.y + f64::from(body.height) + grown > q.y
        && p.z - half < q.z + other_half
        && p.z + half > q.z - other_half
}

fn schoolmates<'a>(
    fish: &'a [FamilyMob<Fish>],
    me: &'a FamilyMob<Fish>,
) -> impl Iterator<Item = &'a FamilyMob<Fish>> {
    fish.iter().filter(move |mob| {
        let body = &mob.core.body;
        mob.core.kind == me.core.kind && !mob.core.removed && grown_box_meets(&me.core.body, REACH, body)
    })
}

pub fn board(world: &EntityWorld, id: u64) -> Option<SchoolBoard> {
    let fish = world.family::<Fish>();
    let me = fish.iter().find(|mob| mob.core.id == id)?;
    let max = me.state.variant.max_school()?;
    let living = |id: u64| {
        fish.iter()
            .any(|mob| mob.core.id == id && mob.core.alive() && !mob.core.removed)
    };
    let mate = |mob: &FamilyMob<Fish>| {
        let school = mob.state.school;
        Schoolmate {
            id: mob.core.id,
            position: mob.core.body.position,
            alive: mob.core.alive() && !mob.core.removed,
            follower: school.leader.is_some_and(living),
            can_be_followed: school.size > 1 && school.size < max,
            size: school.size,
        }
    };
    let leader = me
        .state
        .school
        .leader
        .and_then(|leader| fish.iter().find(|mob| mob.core.id == leader))
        .map(mate);
    Some(SchoolBoard {
        id,
        max,
        size: me.state.school.size,
        leader,
        mates: schoolmates(fish, me).map(mate).collect(),
        recruits: None,
        stop_following: None,
    })
}

pub fn post(world: &mut EntityWorld, id: u64, board: Option<SchoolBoard>) {
    let Some(mob) = world.family_mob_mut::<Fish>(id) else {
        return;
    };
    mob.state.goals_mut().school = board;
}

pub fn apply(world: &mut EntityWorld, id: u64) {
    let Some(mob) = world.family_mob_mut::<Fish>(id) else {
        return;
    };
    let Some(board) = mob.state.goals_mut().school.as_mut() else {
        return;
    };
    let (stop, recruits) = (board.stop_following.take(), board.recruits.take());
    if let Some(old) = stop {
        mob.state.school.leader = None;
        if let Some(leader) = world.family_mob_mut::<Fish>(old) {
            leader.state.school.size -= 1;
        }
    }
    let Some((leader, followers)) = recruits else {
        return;
    };
    for follower in followers {
        if let Some(mob) = world.family_mob_mut::<Fish>(follower) {
            mob.state.school.leader = Some(leader);
        }
        if let Some(mob) = world.family_mob_mut::<Fish>(leader) {
            mob.state.school.size += 1;
        }
    }
}

pub fn forget_when_alone(world: &mut EntityWorld, id: u64) {
    let fish = world.family::<Fish>();
    let Some(me) = fish.iter().find(|mob| mob.core.id == id) else {
        return;
    };
    if me.state.variant.max_school().is_none() || me.state.school.size <= 1 {
        return;
    }
    if world.level_random.next_int(LONELY_CHANCE) != 1 {
        return;
    }
    let fish = world.family::<Fish>();
    let Some(me) = fish.iter().find(|mob| mob.core.id == id) else {
        return;
    };
    if schoolmates(fish, me).count() <= 1 {
        if let Some(mob) = world.family_mob_mut::<Fish>(id) {
            mob.state.school.size = 1;
        }
    }
}

pub fn follower_of(ai: &FishAi) -> bool {
    ai.school.as_ref().is_some_and(SchoolBoard::follower)
}
