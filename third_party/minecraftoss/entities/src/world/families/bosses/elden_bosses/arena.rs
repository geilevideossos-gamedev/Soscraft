use super::EldenBoss;
use crate::fluid::FluidFrame;
use crate::navigation::{GroundNavigation, ground_view, plan_walk_path};
use crate::tempt::PlayerCandidate;
use crate::walk_path::{WalkProfile, WalkTarget};
use crate::world::families::{FamilyMob, MobCore};
use elden_boss::data::Motion;
use elden_boss::shape::{self, flat_distance, yaw_to};
use elden_boss::{Act, BossDef, Seen};
use glam::DVec3;
use minecraftoss_player::World;
use minecraftoss_player::collision::{Bounds, no_block_collision};
use minecraftoss_player::path_type::PathType;

pub const PLAN_GAP: i32 = 10;
pub const REACH: i32 = 1;
pub const WALK_REACH: i32 = 0;
pub const MAX_FALL: i32 = 3;
pub const GROUND_DEPTH: i32 = 8;
pub const SPEED_MODIFIER: f64 = 1.0;
const SURFACE: f64 = 1.0e-4;
const APART: f64 = 1.0e-6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Way {
    pub player: u64,
    pub reachable: bool,
    pub room: Option<DVec3>,
}

pub fn eligible(player: &PlayerCandidate) -> bool {
    player.alive && !player.spectator && player.attackable
}

pub fn profile(def: &BossDef) -> WalkProfile {
    let mut profile = WalkProfile::animal(def.width as f32, def.height as f32);
    profile.clear_malus(PathType::FireInNeighbor);
    profile.clear_malus(PathType::Fire);
    profile.max_up_step = def.step_height as f32;
    profile.max_fall_distance = MAX_FALL;
    profile.can_float = false;
    profile.follow_range = def.arena.reset as f32;
    profile.walk_target = WalkTarget::Monster;
    profile
}

pub fn ground_under<W: World>(world: &W, feet: DVec3) -> f64 {
    let half = shape::PLAYER_WIDTH / 2.0;
    let (west, east) = (feet.x - half, feet.x + half);
    let (north, south) = (feet.z - half, feet.z + half);
    let top = feet.y.floor() as i32;
    for y in (top - GROUND_DEPTH..=top).rev() {
        let mut ground: Option<f64> = None;
        for x in west.floor() as i32..=east.floor() as i32 {
            for z in north.floor() as i32..=south.floor() as i32 {
                let (bx, bz) = (f64::from(x), f64::from(z));
                for b in world.collision_boxes((x, y, z)) {
                    let surface = f64::from(y) + b[4];
                    let under = bx + b[0] < east
                        && bx + b[3] > west
                        && bz + b[2] < south
                        && bz + b[5] > north;
                    if under && surface <= feet.y + SURFACE {
                        ground = Some(ground.map_or(surface, |g| g.max(surface)));
                    }
                }
            }
        }
        if let Some(ground) = ground {
            return ground;
        }
    }
    feet.y - f64::from(GROUND_DEPTH)
}

pub fn stands<W: World>(world: &W, at: DVec3, width: f32, height: f32) -> bool {
    let bounds = Bounds::standing(at, width, height);
    if !no_block_collision(world, bounds.min, bounds.max) {
        return false;
    }
    let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
    let y = (at.y - 0.5).floor() as i32;
    let (cx, cz) = (at.x - f64::from(x), at.z - f64::from(z));
    world.collision_boxes((x, y, z)).iter().any(|b| {
        (f64::from(y) + b[4] - at.y).abs() < SURFACE
            && b[0] <= cx
            && cx <= b[3]
            && b[2] <= cz
            && cz <= b[5]
    })
}

fn overlaps(bounds: Bounds, min: DVec3, max: DVec3) -> bool {
    (0..3).all(|axis| bounds.max[axis].min(max[axis]) - bounds.min[axis].max(min[axis]) > APART)
}

pub fn spot_near<W: World>(
    world: &W,
    def: &BossDef,
    centre: DVec3,
    clear_of: Option<(DVec3, DVec3)>,
) -> Option<DVec3> {
    let (width, height) = (def.width as f32, def.height as f32);
    let across = def.arena.fade_across.floor() as i32;
    let rise = def.arena.fade_rise.floor() as i32;
    let (x, y, z) = (
        centre.x.floor() as i32,
        centre.y.floor() as i32,
        centre.z.floor() as i32,
    );
    let mut spots = vec![centre];
    for dy in -rise..=rise {
        for dx in -across..=across {
            for dz in -across..=across {
                spots.push(DVec3::new(
                    f64::from(x + dx) + 0.5,
                    f64::from(y + dy),
                    f64::from(z + dz) + 0.5,
                ));
            }
        }
    }
    spots.sort_by(|a, b| {
        a.distance_squared(centre)
            .total_cmp(&b.distance_squared(centre))
    });
    spots.into_iter().find(|&at| {
        let bounds = Bounds::standing(at, width, height);
        let apart = clear_of.map_or(true, |(min, max)| !overlaps(bounds, min, max));
        apart && stands(world, at, width, height)
    })
}

pub fn room_beside<W: World>(world: &W, def: &BossDef, feet: DVec3, eye: f32) -> Option<DVec3> {
    let body = shape::Body::player(feet, f64::from(eye));
    spot_near(world, def, feet, Some((body.min, body.max)))
}

pub fn survey<W: World>(mob: &mut FamilyMob<EldenBoss>, level: &W, players: &[PlayerCandidate]) {
    let FamilyMob { core, state } = mob;
    if !state.brain.awake() {
        state.ways.clear();
        return;
    }
    if core.tick_count - state.planned < PLAN_GAP {
        return;
    }
    let body = &core.body;
    let fluid = FluidFrame::sample(level, body.position, body.width, body.height);
    let profile = profile(state.def);
    let reset = state.def.arena.reset;
    let mut ways = Vec::new();
    for player in players
        .iter()
        .filter(|player| eligible(player) && flat_distance(player.position, state.anchor) <= reset)
    {
        let before = state.ways.iter().find(|way| way.player == player.id);
        let mut scratch = GroundNavigation::default();
        let path = plan_walk_path(
            body,
            &mut scratch,
            level,
            &profile,
            fluid,
            player.position,
            REACH,
        );
        let way = match path {
            Some(path) => Way {
                player: player.id,
                reachable: path.reached(),
                room: None,
            },
            None => match before {
                Some(before) => *before,
                None => Way {
                    player: player.id,
                    reachable: true,
                    room: None,
                },
            },
        };
        let room = if way.reachable {
            None
        } else {
            room_beside(level, state.def, player.position, player.eye_height)
        };
        ways.push(Way { room, ..way });
    }
    state.ways = ways;
    state.planned = core.tick_count;
}

pub fn seen<W: World>(level: &W, players: &[PlayerCandidate], ways: &[Way]) -> Vec<Seen> {
    players
        .iter()
        .filter(|player| eligible(player))
        .map(|player| {
            let way = ways.iter().find(|way| way.player == player.id);
            Seen {
                id: player.id,
                feet: player.position,
                eye: f64::from(player.eye_height),
                ground: ground_under(level, player.position),
                reachable: way.map_or(true, |way| way.reachable),
                room: way.and_then(|way| way.room),
            }
        })
        .collect()
}

fn leaping(def: &'static BossDef, act: &Act) -> bool {
    let clock = act.clip.clock;
    act.clip.played(def).is_some_and(|mv| {
        mv.motion.iter().any(|motion| {
            let window = motion.window();
            matches!(motion, Motion::Leap { .. }) && window.from <= clock && clock < window.to
        })
    })
}

fn stride<W: World>(
    core: &MobCore,
    state: &mut EldenBoss,
    level: &W,
    to: DVec3,
) -> Option<(f32, f32)> {
    let def = state.def;
    let body = &core.body;
    let fluid = FluidFrame::sample(level, body.position, body.width, body.height);
    let block = (
        to.x.floor() as i32,
        to.y.floor() as i32,
        to.z.floor() as i32,
    );
    let changed = state.walk_block != Some(block);
    let due = core.tick_count - state.walked >= PLAN_GAP;
    let done = state.navigation.is_done();
    if (changed && done) || (due && (changed || done)) {
        let profile = profile(def);
        let planned = plan_walk_path(
            body,
            &mut state.navigation,
            level,
            &profile,
            fluid,
            to,
            WALK_REACH,
        );
        if planned.is_some() {
            state
                .navigation
                .move_to_in(level, planned, SPEED_MODIFIER, body.position);
            state.walk_block = Some(block);
            state.walked = core.tick_count;
        }
    }
    let speed = def.speed as f32;
    let (can_update, surface) = ground_view(level, body, fluid, false);
    let (wanted, modifier) =
        state
            .navigation
            .tick_in(level, body.position, can_update, surface, body.width, speed)?;
    let yaw = yaw_to(body.position, wanted)?;
    Some((yaw as f32, speed * modifier as f32))
}

fn halt(state: &mut EldenBoss) {
    state.navigation.stop();
    state.walk_block = None;
}

pub fn carry_out<W: World>(mob: &mut FamilyMob<EldenBoss>, level: &W, act: &Act) {
    let FamilyMob { core, state } = mob;
    if let Some(to) = act.fade {
        let spot = spot_near(level, state.def, to, None).unwrap_or(to);
        core.body.position = spot;
        core.body.velocity = DVec3::ZERO;
        core.body.fall_distance = 0.0;
        state.stride = DVec3::ZERO;
        state.dropped = false;
        halt(state);
        return;
    }
    let yaw = core.yaw;
    if act.motion != DVec3::ZERO {
        halt(state);
        let leap = leaping(state.def, act);
        let body = &mut core.body;
        if !leap {
            body.velocity.x = act.motion.x;
            body.velocity.z = act.motion.z;
        } else if state.dropped {
            body.velocity.x = 0.0;
            body.velocity.z = 0.0;
        } else {
            body.velocity = act.motion;
        }
        let _ = body.travel_air(level, DVec3::ZERO, 0.0, yaw);
        body.fall_distance = 0.0;
        state.dropped = state.dropped || (leap && body.horizontal_collision);
        state.stride = DVec3::ZERO;
        return;
    }
    state.dropped = false;
    let walking = match act.walk {
        Some(to) => stride(core, state, level, to),
        None => {
            halt(state);
            None
        }
    };
    let body = &mut core.body;
    let _ = match walking {
        Some((heading, speed)) => {
            let input = DVec3::new(0.0, 0.0, f64::from(speed));
            body.travel_air(level, input, speed, heading)
        }
        None => body.travel_air(level, DVec3::ZERO, 0.0, yaw),
    };
    body.fall_distance = 0.0;
    state.stride = DVec3::new(body.velocity.x, 0.0, body.velocity.z);
}

#[cfg(test)]
mod tests {
    use super::super::tests::{margit, player_at};
    use super::*;
    use crate::world::EntityWorld;
    use elden_boss::brain::HOME;
    use elden_boss::margit::MARGIT;
    use elden_boss::shape::wrap;
    use elden_boss::{Source, State};
    use minecraftoss_player::{Block, Pos};
    use std::collections::HashSet;

    #[derive(Default)]
    struct Scene {
        stone: HashSet<Pos>,
        air: HashSet<Pos>,
    }

    impl World for Scene {
        fn block(&self, pos: Pos) -> Option<Block> {
            if self.air.contains(&pos) {
                return None;
            }
            (pos.1 < 64 || self.stone.contains(&pos)).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn at(x: f64, y: f64, z: f64) -> DVec3 {
        DVec3::new(x, y, z)
    }

    fn summoned(world: &mut EntityWorld, at: DVec3) -> u64 {
        world.spawn_elden_boss(&MARGIT, at, 0.0, at, false)
    }

    fn feet(world: &EntityWorld, id: u64) -> DVec3 {
        margit(world, id).core.body.position
    }

    fn state(world: &EntityWorld, id: u64) -> State {
        margit(world, id).state.brain.state()
    }

    fn until_idle(
        world: &mut EntityWorld,
        scene: &mut Scene,
        id: u64,
        players: &[PlayerCandidate],
    ) {
        for _ in 0..100 {
            world.tick_with_players(scene, players);
            if state(world, id) == State::Idle {
                return;
            }
        }
        panic!("he never woke");
    }

    fn wall(scene: &mut Scene, z: i32, high: i32) {
        for x in -6..=6 {
            for y in 64..64 + high {
                scene.stone.insert((x, y, z));
            }
        }
    }

    #[test]
    fn margit_walks_to_a_player_10_blocks_away() {
        let mut world = EntityWorld::default();
        let mut scene = Scene::default();
        let id = summoned(&mut world, at(0.5, 64.0, 0.5));
        let player = at(0.5, 64.0, 10.5);
        let players = [player_at(1, player)];
        let (mut walked, mut fastest, mut closest) = (0.0, 0.0_f64, f64::MAX);
        for _ in 0..300 {
            let before = feet(&world, id);
            world.tick_with_players(&mut scene, &players);
            let mob = margit(&world, id);
            let now = mob.core.body.position;
            if mob.state.brain.state() == State::Idle && mob.state.walk.is_some() {
                let step = flat_distance(before, now);
                walked += step;
                fastest = fastest.max(step);
            }
            closest = closest.min(flat_distance(now, player));
            assert!(now.y >= 64.0 - 1e-6, "never inside the ground: {now}");
        }
        assert!(walked > 3.0, "he walks between his moves: {walked}");
        assert!(fastest < 0.25, "at his walking speed: {fastest}");
        assert!(closest < 5.5, "he comes to the player: {closest}");
        assert_eq!(margit(&world, id).state.brain.target(), Some(1));
    }

    #[test]
    fn margit_stops_at_his_leash() {
        let mut world = EntityWorld::default();
        let mut scene = Scene::default();
        let anchor = at(0.5, 64.0, 0.5);
        let id = summoned(&mut world, anchor);
        let near = [player_at(1, at(0.5, 64.0, 10.5))];
        until_idle(&mut world, &mut scene, id, &near);
        let player = at(0.5, 64.0, 34.5);
        let far = [player_at(1, player)];
        let mut farthest = 0.0_f64;
        for _ in 0..400 {
            world.tick_with_players(&mut scene, &far);
            farthest = farthest.max(flat_distance(feet(&world, id), anchor));
        }
        let leash = MARGIT.arena.leash;
        assert!(farthest <= leash + 1.0, "{farthest}");
        assert!(
            farthest >= leash - 3.0,
            "he goes as far as his leash: {farthest}"
        );
        let mob = margit(&world, id);
        assert!(
            mob.state.brain.awake(),
            "a player at 34 blocks keeps the fight"
        );
        assert_eq!(mob.state.brain.target(), Some(1));
        let wanted = yaw_to(mob.core.body.position, player).unwrap();
        let facing = wrap(wanted - f64::from(mob.core.yaw)).abs();
        assert!(facing <= MARGIT.turn + 1.0, "he faces the player: {facing}");
    }

    fn leap_toward(scene: &mut Scene) -> (Vec<DVec3>, DVec3) {
        let mut world = EntityWorld::default();
        let start = at(0.5, 64.0, 0.5);
        let id = summoned(&mut world, start);
        let players = [player_at(1, at(0.5, 64.0, 12.5))];
        until_idle(&mut world, scene, id, &players);
        assert_eq!(feet(&world, id), start);
        let brain = &mut world.elden_boss_mut(id).unwrap().state.brain;
        assert!(brain.begin("leap_slam"));
        let mut path = Vec::new();
        for _ in 0..80 {
            world.tick_with_players(scene, &players);
            path.push(feet(&world, id));
        }
        let mob = margit(&world, id);
        assert!(mob.core.body.on_ground, "he lands");
        (path, mob.core.body.position)
    }

    #[test]
    fn a_leap_over_a_one_block_wall_lands_beyond_it() {
        let mut low = Scene::default();
        wall(&mut low, 5, 1);
        let (path, landed) = leap_toward(&mut low);
        let top = path.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        assert!(top > 66.0, "he rises over the wall: {top}");
        assert!(landed.z > 6.5, "beyond the wall: {landed}");
        assert_eq!(landed.y, 64.0, "on the ground, not the wall");
        let mut high = Scene::default();
        wall(&mut high, 5, 6);
        let (path, stopped) = leap_toward(&mut high);
        assert!(path.iter().all(|p| p.z < 5.0), "a wall too high stops him");
        assert!(
            stopped.z < 5.0 && stopped.y == 64.0,
            "and he drops to its foot: {stopped}"
        );
    }

    #[test]
    fn margit_resets_when_everyone_leaves_for_5_seconds() {
        let mut world = EntityWorld::default();
        let mut scene = Scene::default();
        let anchor = at(0.5, 64.0, 0.5);
        let id = summoned(&mut world, anchor);
        let players = [player_at(1, at(0.5, 64.0, 8.5))];
        until_idle(&mut world, &mut scene, id, &players);
        let mob = world.elden_boss_mut(id).unwrap();
        mob.core.body.position = at(0.5, 64.0, 6.5);
        let explosion = Source::Explosion { damage: 40.0 };
        assert!(super::super::take(mob, explosion, Some(1)).applied);
        assert!(margit(&world, id).state.brain.health() < 800.0);
        let gone = [player_at(1, at(0.5, 64.0, 60.5))];
        for _ in 0..99 {
            world.tick_with_players(&mut scene, &gone);
            assert!(margit(&world, id).state.brain.awake());
        }
        world.tick_with_players(&mut scene, &gone);
        let mob = margit(&world, id);
        assert_eq!(mob.state.brain.state(), State::Return);
        assert_eq!((mob.state.brain.health(), mob.core.health), (800.0, 800.0));
        assert_eq!(mob.state.brain.phase(), 1);
        let mut faded = false;
        for _ in 0..200 {
            world.tick_with_players(&mut scene, &gone);
            faded |= state(&world, id) == State::Fade;
            if state(&world, id) == State::Asleep {
                break;
            }
        }
        assert!(!faded, "he walks home");
        assert_eq!(state(&world, id), State::Asleep);
        assert!(flat_distance(feet(&world, id), anchor) <= HOME);
    }

    #[test]
    fn a_margit_killed_while_walking_stops_where_he_falls() {
        let mut world = EntityWorld::default();
        let mut scene = Scene::default();
        let id = summoned(&mut world, at(0.5, 64.0, 0.5));
        let players = [player_at(1, at(0.5, 64.0, 9.5))];
        let mut walking = false;
        for _ in 0..600 {
            world.tick_with_players(&mut scene, &players);
            if margit(&world, id).state.stride.length() > 0.08 {
                walking = true;
                break;
            }
        }
        assert!(walking, "he walks toward the player");
        let mob = world.elden_boss_mut(id).unwrap();
        mob.state.brain.set_health(1.0);
        let blow = Source::Explosion { damage: 10.0 };
        assert!(super::super::take(mob, blow, Some(1)).died);
        let fell = feet(&world, id);
        for _ in 0..15 {
            world.tick_with_players(&mut scene, &players);
        }
        let slid = flat_distance(feet(&world, id), fell);
        assert!(slid < 0.5, "he slows to a stop as he dies: {slid}");
    }

    #[test]
    fn margit_fades_inside_a_wall_ring_he_cannot_reach() {
        let mut world = EntityWorld::default();
        let mut scene = Scene::default();
        for x in -4..=4_i32 {
            for z in -4..=4_i32 {
                if x.abs().max(z.abs()) == 4 {
                    for y in 64..70 {
                        scene.stone.insert((x, y, z));
                    }
                }
            }
        }
        let id = summoned(&mut world, at(0.5, 64.0, 14.5));
        let players = [player_at(1, at(0.5, 64.0, 0.5))];
        let mut faded = None;
        for tick in 0..600 {
            world.tick_with_players(&mut scene, &players);
            let mob = margit(&world, id);
            assert!(
                mob.state.hits.is_empty(),
                "no swing reaches him through the wall"
            );
            if mob.state.brain.state() == State::Fade && faded.is_none() {
                let way = mob.state.ways.iter().find(|way| way.player == 1).copied();
                faded = Some((tick, way));
            }
            if faded.is_some() && mob.state.brain.state() == State::Idle {
                break;
            }
        }
        let (tick, way) = faded.expect("he fades");
        let stuck = i32::from(MARGIT.arena.stuck_ticks);
        assert!(tick >= stuck, "after 5 stuck seconds: {tick}");
        let way = way.unwrap();
        assert!(!way.reachable && way.room.is_some(), "{way:?}");
        let mob = margit(&world, id);
        let now = mob.core.body.position;
        assert!(
            (now.x - 0.5).abs() < 3.5 && (now.z - 0.5).abs() < 3.5,
            "inside: {now}"
        );
        assert_eq!(now.y, 64.0);
        assert!(stands(&scene, now, 1.4, 3.0));
        for _ in 0..PLAN_GAP {
            world.tick_with_players(&mut scene, &players);
        }
        let mob = margit(&world, id);
        let way = mob.state.ways.iter().find(|way| way.player == 1).unwrap();
        assert!(way.reachable, "inside the ring a path reaches the player");
    }

    #[test]
    fn a_player_in_a_two_deep_hole_beside_him_is_still_hit() {
        let mut world = EntityWorld::default();
        let mut scene = Scene::default();
        scene.air.extend([(3, 63, 0), (3, 62, 0)]);
        let hole = at(3.5, 62.0, 0.5);
        assert_eq!(ground_under(&scene, hole), 62.0);
        let id = summoned(&mut world, at(0.5, 64.0, 0.5));
        let players = [player_at(1, hole)];
        let mut hit = false;
        for _ in 0..600 {
            world.tick_with_players(&mut scene, &players);
            let mob = margit(&world, id);
            hit |= mob.state.hits.iter().any(|hit| hit.player == 1);
            if hit {
                break;
            }
            assert!(
                mob.core.body.position.y >= 64.0 - 1e-6,
                "he does not fit in the hole"
            );
        }
        assert!(hit, "his swings reach a block below his feet");
        let way = margit(&world, id).state.ways[0];
        assert!(!way.reachable, "no path ends in the hole");
    }

    #[test]
    fn a_standing_spot_needs_a_block_under_him_and_room_for_his_box() {
        let mut scene = Scene::default();
        assert!(stands(&scene, at(0.5, 64.0, 0.5), 1.4, 3.0));
        assert!(!stands(&scene, at(0.5, 65.0, 0.5), 1.4, 3.0), "in the air");
        assert!(
            !stands(&scene, at(0.5, 63.0, 0.5), 1.4, 3.0),
            "in the ground"
        );
        scene.stone.insert((1, 66, 0));
        assert!(
            !stands(&scene, at(0.5, 64.0, 0.5), 1.4, 3.0),
            "a block in his box"
        );
        let mut pillar = Scene::default();
        for y in 64..69 {
            pillar.stone.insert((0, y, 0));
        }
        let top = at(0.5, 69.0, 0.5);
        assert_eq!(
            room_beside(&pillar, &MARGIT, top, 1.62),
            None,
            "no room on a pillar"
        );
        let room = room_beside(&Scene::default(), &MARGIT, at(0.5, 64.0, 0.5), 1.62).unwrap();
        assert_eq!(room.y, 64.0);
        assert!(
            flat_distance(room, at(0.5, 64.0, 0.5)) <= 1.5,
            "beside him: {room}"
        );
    }
}
