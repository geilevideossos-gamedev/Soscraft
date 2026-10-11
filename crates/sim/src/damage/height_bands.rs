use super::*;
use crate::bullet_collision::{BulletTraceQuery, ColliderId, MASK_SHOT, TraceOutcome};
use crate::voxel::{BLOCK, VoxelChunk};
use std::sync::Mutex;

static BAND: Mutex<()> = Mutex::new(());

const BANDS: [f32; 3] = [0.0, 45_000.0, -45_000.0];
const FLOOR_Y: i32 = 64;
const WALL_X: i32 = 10;
const WALL_TOP: i32 = FLOOR_Y + 3;
const STONE: u16 = 1;
const A: ClientId = ClientId(1);
const B: ClientId = ClientId(2);
const C: ClientId = ClientId(3);
const WORLD_ENTITY: i32 = trace_iw4::ENTITYNUM_WORLD as i32;

fn lock() -> std::sync::MutexGuard<'static, ()> {
    BAND.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct Band {
    sim: crate::SimWorld,
    origin: [f64; 3],
    feet: f32,
    tick: u32,
}

impl Band {
    fn new(feet: f32) -> Self {
        let mut builder = crate::SimContentBuilder::default();
        let cmodels = crate::SimClipCmodels {
            models: vec![clipmap_iw4::ClipCmodel {
                mins: [-3_000.0, -3_000.0, -500.0],
                maxs: [3_000.0, 3_000.0, 1_500.0],
                radius: 4_500.0,
                first_brush: 0,
                num_brushes: 1,
            }],
            triggers: Vec::new(),
        };
        let brush = crate::SimBrush {
            planes: Vec::new(),
            contents: 0,
            plane_surface_flags: Vec::new(),
            glass_encoded: 0,
        };
        builder.set_clip_map(
            vec![brush],
            crate::SimClipBsp::default(),
            crate::SimClipMesh::default(),
            cmodels,
        );
        let mut sim = crate::SimWorld::default();
        sim.install_content(builder.finish());
        sim.bootstrap(crate::MatchBootstrap::default())
            .expect("an empty bootstrap is accepted");
        sim.suppress_snapshot_publish();
        let origin = [
            0.5,
            f64::from(FLOOR_Y + 1) - f64::from(feet) / f64::from(BLOCK),
            0.5,
        ];
        crate::voxel::activate(
            sim.content().clip_brushes(),
            origin,
            vec![Vec::new(), vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]],
        );
        for cx in -2..2 {
            for cz in -2..2 {
                let (min_y, height) = (FLOOR_Y - 16, 32);
                let mut shapes = vec![0; (height * 256) as usize];
                let mut put = |x: i32, y: i32, z: i32| {
                    shapes[(((y - min_y) * 16 + (z & 15)) * 16 + (x & 15)) as usize] = STONE;
                };
                for x in cx * 16..cx * 16 + 16 {
                    for z in cz * 16..cz * 16 + 16 {
                        put(x, FLOOR_Y, z);
                        if x == WALL_X && (-6..=6).contains(&z) {
                            for y in FLOOR_Y + 1..WALL_TOP {
                                put(x, y, z);
                            }
                        }
                    }
                }
                crate::voxel::set_chunk(
                    cx,
                    cz,
                    VoxelChunk {
                        min_y,
                        height,
                        shapes,
                    },
                );
            }
        }
        Self {
            sim,
            origin,
            feet,
            tick: 0,
        }
    }

    fn at(&self, x: f64, z: f64) -> [f32; 3] {
        crate::voxel::to_map(self.origin, [x, f64::from(FLOOR_Y + 1), z])
    }

    fn spawn(&mut self, id: ClientId, x: f64, z: f64) -> [f32; 3] {
        self.tick += 1;
        let tick = Tick(self.tick);
        let mut frame = self.sim.frame();
        crate::script_player::spawn(&mut frame, tick, id, [0.0; 3], [0.0; 3], "playing");
        let spawned = frame
            .player(id)
            .expect("a spawned player has a state")
            .origin;
        drop(frame);
        let place = self.at(x, z);
        assert!(self.sim.teleport_facing(id, place, [0.0; 3]));
        spawned
    }

    fn step(&mut self, cmds: &[(ClientId, u32, i8)]) {
        self.tick += 1;
        let cmds = cmds
            .iter()
            .map(|&(id, buttons, forwardmove)| {
                let cmd = playerstate_iw4::UserCmd {
                    server_time: crate::level_time_ms(Tick(self.tick)),
                    buttons,
                    forwardmove,
                    ..playerstate_iw4::UserCmd::default()
                };
                (id, cmd)
            })
            .collect();
        crate::step(
            &mut self.sim,
            Tick(self.tick),
            &crate::TickInput::from_cmds(cmds),
            crate::MATCH_TICK_MS as i32,
            crate::StepReason::PredictNew,
        );
    }

    fn ps(&self, id: ClientId) -> playerstate_iw4::PlayerState {
        *self.sim.player(id).expect("the player is in the world")
    }

    fn alive(&self, id: ClientId) -> bool {
        self.sim
            .client_meta(id)
            .is_some_and(|meta| meta.lifecycle == ClientLifecycle::Alive)
    }

    fn settle(&mut self, ids: &[ClientId]) {
        let idle: Vec<_> = ids.iter().map(|&id| (id, 0, 0)).collect();
        for _ in 0..10 {
            self.step(&idle);
        }
        for &id in ids {
            let ps = self.ps(id);
            assert!(
                (ps.origin[2] - self.feet).abs() < 0.01,
                "band {}: {id:?} stands at {:?}",
                self.feet,
                ps.origin
            );
            assert_eq!(ps.ground_entity_num, WORLD_ENTITY, "band {}", self.feet);
        }
    }

    fn shoot(&self, from: ClientId, at: [f32; 3]) -> TraceOutcome {
        let eye = self.ps(from).origin;
        let start = [eye[0], eye[1], eye[2] + 40.0];
        let dir: [f32; 3] = std::array::from_fn(|k| at[k] - start[k]);
        let len = dir.iter().map(|v| v * v).sum::<f32>().sqrt();
        let end = std::array::from_fn(|k| start[k] + dir[k] / len * 8_192.0);
        self.sim.bullet_trace(
            BulletTraceQuery {
                start,
                end,
                mask: MASK_SHOT,
                ignore: Some(from),
                ignore_hit: None,
                ignore_model: None,
            },
            None,
        )
    }

    fn frag(&self, x: f32) -> ExplosionBlast {
        ExplosionBlast {
            cone: None,
            origin: [x, 0.0, self.feet + 10.0],
            radius: 256.0,
            inner_damage: 130.0,
            outer_damage: 55.0,
            weapon: 0,
            source: DamageSource::Projectile(crate::identities::ProjectileId(1)),
            attacker: A,
            attacker_life: LifeSequence(1),
            killcam_entity_start_time: 0,
        }
    }
}

fn walk_and_jump(feet: f32) -> Vec<[f32; 3]> {
    let mut band = Band::new(feet);
    band.spawn(A, 0.5, 0.5);
    band.settle(&[A]);
    let mut path = Vec::new();
    for _ in 0..60 {
        band.step(&[(A, 0, 127)]);
        path.push(band.ps(A).origin);
    }
    let stopped = band.ps(A).origin;
    let face = band.at(f64::from(WALL_X), 0.5)[0] - crate::voxel::HULL_HALF_WIDTH;
    assert!(
        stopped[0] > face - 0.2 && stopped[0] <= face,
        "band {feet}: walked to {stopped:?}, the wall's face is at x {face}"
    );
    assert!(
        path.iter().all(|p| (p[2] - feet).abs() < 0.01),
        "band {feet}: left the floor while walking"
    );
    band.step(&[(A, playerstate_iw4::buttons::JUMP, 0)]);
    path.push(band.ps(A).origin);
    for _ in 0..40 {
        band.step(&[(A, 0, 0)]);
        path.push(band.ps(A).origin);
    }
    let peak = path.iter().map(|p| p[2] - feet).fold(f32::MIN, f32::max);
    assert!(
        peak > 35.0 && peak < 40.0,
        "band {feet}: jumped {peak} high"
    );
    let landed = band.ps(A);
    assert!(
        (landed.origin[2] - feet).abs() < 0.2,
        "band {feet}: landed at {:?}",
        landed.origin
    );
    assert_eq!(landed.ground_entity_num, WORLD_ENTITY, "band {feet}");
    assert!(band.alive(A) && landed.health == 100, "band {feet}");
    path.iter().map(|p| [p[0], p[1], p[2] - feet]).collect()
}

#[test]
fn mw2_walks_jumps_and_lands_at_the_height_of_every_band() {
    let _band = lock();
    let control = walk_and_jump(BANDS[0]);
    for feet in &BANDS[1..] {
        let path = walk_and_jump(*feet);
        assert_eq!(path.len(), control.len());
        for (tick, (a, b)) in control.iter().zip(&path).enumerate() {
            let apart = (0..3).map(|k| (a[k] - b[k]).abs()).fold(0.0, f32::max);
            assert!(
                apart < 0.05,
                "band {feet}, tick {tick}: {b:?} against {a:?} at map z 0"
            );
        }
    }
}

#[test]
fn a_bullet_hits_a_block_and_a_player_at_the_height_of_every_band() {
    let _band = lock();
    for feet in BANDS {
        let mut band = Band::new(feet);
        band.spawn(A, 0.5, 0.5);
        band.spawn(B, 4.5, 4.5);
        band.settle(&[A, B]);
        let mut frame = band.sim.frame();
        frame.record_collision_history(Tick(band.tick), 0, None);
        drop(frame);
        let target = band.ps(B).origin;
        match band.shoot(A, [target[0], target[1], target[2] + 40.0]) {
            TraceOutcome::Hit {
                collider: ColliderId::Player { client, .. },
                end,
                ..
            } => {
                assert_eq!(client, B, "band {feet}");
                assert!((end[2] - feet - 40.0).abs() < 1.0, "band {feet}: {end:?}");
            }
            other => panic!("band {feet}: the shot at the player gave {other:?}"),
        }
        let wall = band.at(f64::from(WALL_X), 0.5);
        match band.shoot(A, [wall[0], wall[1], feet + 40.0]) {
            TraceOutcome::Hit {
                collider: ColliderId::World { .. },
                end,
                normal,
                ..
            } => {
                assert!(
                    (end[0] - wall[0]).abs() < 0.2 && (end[2] - feet - 40.0).abs() < 0.2,
                    "band {feet}: struck the wall at {end:?}"
                );
                assert_eq!(normal, [-1.0, 0.0, 0.0], "band {feet}");
            }
            other => panic!("band {feet}: the shot at the wall gave {other:?}"),
        }
        let sky = band.shoot(A, [0.0, 0.0, feet + 8_000.0]);
        assert!(
            matches!(sky, TraceOutcome::Miss { .. }),
            "band {feet}: {sky:?}"
        );
    }
}

#[test]
fn a_grenade_blast_hurts_a_player_and_a_wall_shields_one_at_the_height_of_every_band() {
    let _band = lock();
    let mut hurt = Vec::new();
    for feet in BANDS {
        let mut band = Band::new(feet);
        band.spawn(A, -6.5, 0.5);
        band.spawn(B, 4.5, 0.5);
        band.spawn(C, 12.5, 0.5);
        band.settle(&[A, B, C]);
        let blast = band.frag(band.at(8.5, 0.5)[0]);
        let frame = band.sim.frame();
        let attempts = radius_player_attempts(&frame, &blast);
        drop(frame);
        let targets: Vec<ClientId> = attempts.iter().map(|attempt| attempt.target).collect();
        assert_eq!(targets, [B], "band {feet}: {attempts:?}");
        let amount = attempts[0].amount;
        assert!(amount > 55 && amount < 130, "band {feet}: {amount}");
        let mut frame = band.sim.frame();
        let finish = crate::script_player::finish_damage(&mut frame, B, amount, None);
        drop(frame);
        assert_eq!(finish, crate::script_player::Finish::Hurt, "band {feet}");
        assert_eq!(band.ps(B).health, 100 - amount, "band {feet}");
        hurt.push(amount);
    }
    assert!(
        hurt.iter().all(|&amount| amount == hurt[0]),
        "the same blast hurts the same in every band: {hurt:?}"
    );
}

#[test]
fn a_killed_player_gets_an_obituary_and_respawns_into_its_band() {
    let _band = lock();
    for feet in BANDS {
        let mut band = Band::new(feet);
        band.spawn(A, 0.5, 0.5);
        band.spawn(B, 4.5, 0.5);
        band.settle(&[A, B]);
        band.tick += 1;
        let tick = Tick(band.tick);
        let mut frame = band.sim.frame();
        let finish = crate::script_player::finish_damage(&mut frame, B, 1_000, None);
        assert_eq!(finish, crate::script_player::Finish::Killed, "band {feet}");
        crate::script_player::kill(&mut frame, tick, B, Some(A), None);
        crate::script_player::obituary(&mut frame, tick, B, Some(A), 0, "MOD_GRENADE_SPLASH");
        drop(frame);
        assert_eq!(
            band.sim.client_meta(B).map(|meta| meta.lifecycle),
            Some(ClientLifecycle::Dead),
            "band {feet}"
        );
        let snapshot = band.sim.snapshot(tick);
        let obituary = snapshot
            .meta
            .entity_events
            .iter()
            .find(|record| record.event == entity_iw4::EntityEventKind::OBITUARY)
            .unwrap_or_else(|| panic!("band {feet}: no obituary"));
        assert_eq!(
            (
                obituary.payload.other_entity_num,
                obituary.payload.attacker_entity_num
            ),
            (B.0 as i32, A.0 as i32),
            "band {feet}"
        );
        let spawned = band.spawn(B, 2.5, 2.5);
        if feet != 0.0 {
            assert_eq!(
                spawned, [0.0; 3],
                "band {feet}: the sim respawns at map origin"
            );
        }
        band.settle(&[A, B]);
        assert!(band.alive(B), "band {feet}");
        assert_eq!(band.ps(B).health, 100, "band {feet}");
        let feet_at = crate::voxel::to_block(band.origin, band.ps(B).origin);
        assert!(
            (feet_at[0] - 2.5).abs() < 0.01 && (feet_at[2] - 2.5).abs() < 0.01,
            "band {feet}: {feet_at:?}"
        );
    }
}

#[test]
fn a_player_stays_where_it_stands_for_two_minutes_at_the_height_of_every_band() {
    let _band = lock();
    for feet in BANDS {
        let mut band = Band::new(feet);
        band.spawn(A, 0.5, 0.5);
        band.settle(&[A]);
        let start = band.ps(A).origin;
        let two_minutes = 2 * 60 * 1_000 / crate::MATCH_TICK_MS;
        for tick in 0..two_minutes {
            band.step(&[(A, 0, 0)]);
            if tick % 100 != 0 {
                continue;
            }
            let ps = band.ps(A);
            assert!(band.alive(A) && ps.health == 100, "band {feet}");
            assert!(
                (0..3).all(|k| (ps.origin[k] - start[k]).abs() < 0.01),
                "band {feet}: moved from {start:?} to {:?}",
                ps.origin
            );
            assert_eq!(ps.ground_entity_num, WORLD_ENTITY, "band {feet}");
            let snapshot = band.sim.snapshot(Tick(band.tick));
            let shown = snapshot
                .players
                .iter()
                .find(|(id, _)| *id == A)
                .map(|(_, ps)| ps.origin);
            assert_eq!(shown, Some(ps.origin), "band {feet}");
            assert!(
                snapshot
                    .meta
                    .area_entities
                    .as_ref()
                    .is_some_and(|area| area.validate().is_ok()),
                "band {feet}"
            );
            let frame = band.sim.frame();
            let near = radius_player_candidates(&frame, ps.origin, 64.0);
            let linked = frame.player_area_bounds(A).map(|bounds| bounds.mid()[2]);
            drop(frame);
            assert!(near.contains(&A), "band {feet}: {near:?}");
            assert!(
                linked.is_some_and(|z| (z - feet - 35.0).abs() < 1.0),
                "band {feet}: {linked:?}"
            );
        }
    }
}
