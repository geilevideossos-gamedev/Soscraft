use glam::DVec3;
use minecraftoss_core::entity_data::wrap_degrees;

use crate::client_mobs::{ClientMobs, ServerMob, SyncFlags};
use crate::dragon_fireball::FireballView;
use crate::ender_dragon::{DEATH_TICKS, DragonView, EYE_HEIGHT, FlightHistory, HISTORY_SENT};

pub const UPDATE_INTERVAL: i32 = 3;
const MAX_HEAD_Y_ROT: f32 = 75.0;
const SIZE: (f32, f32) = (16.0, 8.0);
const RESYNC_TICKS: i32 = 2;
const FIREBALL_SNAP: f64 = 1.0;
const FIREBALL_TRACK_TICKS: i32 = 2;

pub fn flapping(o_flap_time: f32, flap_time: f32) -> bool {
    let before = minecraftoss_player::mth::cos(f64::from(o_flap_time * std::f32::consts::TAU));
    let now = minecraftoss_player::mth::cos(f64::from(flap_time * std::f32::consts::TAU));
    before <= -0.3 && now >= -0.3
}

#[derive(Clone, Debug)]
struct ClientDragon {
    latest: DragonView,
    history: FlightHistory,
    flap_time: f32,
    o_flap_time: f32,
    step: f32,
    tick_count: i32,
    death_time: i32,
    recorded: bool,
}

impl ClientDragon {
    fn new(view: &DragonView) -> Self {
        Self {
            latest: view.clone(),
            history: FlightHistory::default(),
            flap_time: view.flap_time,
            o_flap_time: view.o_flap_time,
            step: 0.0,
            tick_count: view.tick_count,
            death_time: view.death_time,
            recorded: false,
        }
    }

    fn receive(&mut self, view: &DragonView) {
        self.step = view.flap_time - view.o_flap_time;
        if (self.tick_count - view.tick_count).abs() > RESYNC_TICKS {
            self.tick_count = view.tick_count;
        }
        if (self.death_time - view.death_time).abs() > RESYNC_TICKS {
            self.death_time = view.death_time;
        }
        if self.step == 0.0 {
            self.flap_time = view.flap_time;
            self.o_flap_time = view.flap_time;
        }
        self.latest = view.clone();
    }

    fn dying(&self) -> bool {
        self.latest.health <= 0.0
    }
}

fn body(view: &DragonView) -> ServerMob {
    ServerMob {
        id: view.id,
        position: view.position,
        y_rot: view.y_rot,
        x_rot: 0.0,
        y_head_rot: view.y_rot,
        on_ground: false,
        baby: false,
        eye_height: EYE_HEIGHT as f32,
        max_head_y_rot: MAX_HEAD_Y_ROT,
        update_interval: UPDATE_INTERVAL,
        sync: SyncFlags::default(),
        resting: false,
        hurts: 0,
        dead: view.health <= 0.0,
        swings: 0,
        size: SIZE,
        vehicle: None,
        show: Default::default(),
    }
}

#[derive(Default)]
pub struct ClientDragons {
    bodies: ClientMobs,
    dragons: Vec<ClientDragon>,
}

impl ClientDragons {
    pub fn receive(&mut self, views: &[DragonView]) {
        let _ = self.bodies.receive(views.iter().map(body).collect());
        self.dragons
            .retain(|dragon| views.iter().any(|view| view.id == dragon.latest.id));
        for view in views {
            match self
                .dragons
                .iter_mut()
                .find(|dragon| dragon.latest.id == view.id)
            {
                Some(dragon) => dragon.receive(view),
                None => {
                    let mut dragon = ClientDragon::new(view);
                    dragon.receive(view);
                    self.dragons.push(dragon);
                }
            }
        }
    }

    pub fn tick(&mut self) -> Vec<(u64, DVec3, bool)> {
        self.bodies.tick();
        let mut out = Vec::new();
        for dragon in &mut self.dragons {
            let Some(body) = self.bodies.get(dragon.latest.id) else {
                continue;
            };
            dragon.tick_count += 1;
            dragon.o_flap_time = dragon.flap_time;
            if dragon.dying() {
                dragon.death_time = (dragon.death_time + 1).min(DEATH_TICKS);
            } else {
                dragon.flap_time += dragon.step;
                dragon
                    .history
                    .record(body.position.y, wrap_degrees(body.y_rot));
                dragon.recorded = true;
            }
            out.push((
                dragon.latest.id,
                body.position,
                flapping(dragon.o_flap_time, dragon.flap_time),
            ));
        }
        out
    }

    pub fn views(&self) -> Vec<DragonView> {
        self.dragons
            .iter()
            .filter_map(|dragon| {
                let body = self.bodies.get(dragon.latest.id)?;
                let history = if dragon.recorded {
                    dragon.history.recent(HISTORY_SENT)
                } else {
                    dragon.latest.history.clone()
                };
                Some(DragonView {
                    position: body.position,
                    previous_position: body.old_position,
                    y_rot: body.y_rot,
                    flap_time: dragon.flap_time,
                    o_flap_time: dragon.o_flap_time,
                    history,
                    tick_count: dragon.tick_count,
                    death_time: dragon.death_time,
                    ..dragon.latest.clone()
                })
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
struct ClientFireball {
    id: u64,
    position: DVec3,
    previous_position: DVec3,
    velocity: DVec3,
}

impl ClientFireball {
    fn on_track(&self, host: DVec3) -> bool {
        (-FIREBALL_TRACK_TICKS..=FIREBALL_TRACK_TICKS).any(|ticks| {
            (self.position + self.velocity * f64::from(ticks)).distance(host) <= FIREBALL_SNAP
        })
    }
}

#[derive(Default)]
pub struct ClientFireballs {
    fireballs: Vec<ClientFireball>,
}

impl ClientFireballs {
    pub fn receive(&mut self, views: &[FireballView]) {
        self.fireballs
            .retain(|fireball| views.iter().any(|view| view.id == fireball.id));
        for view in views {
            let velocity = view.position - view.previous_position;
            match self
                .fireballs
                .iter_mut()
                .find(|fireball| fireball.id == view.id)
            {
                Some(fireball) => {
                    if !fireball.on_track(view.position) {
                        fireball.position = view.position;
                        fireball.previous_position = view.previous_position;
                    }
                    fireball.velocity = velocity;
                }
                None => self.fireballs.push(ClientFireball {
                    id: view.id,
                    position: view.position,
                    previous_position: view.previous_position,
                    velocity,
                }),
            }
        }
    }

    pub fn tick(&mut self) {
        use minecraftoss_entities::fireball::{ACCELERATION_POWER, INERTIA, normalize};
        for fireball in &mut self.fireballs {
            fireball.velocity = (fireball.velocity
                + normalize(fireball.velocity) * ACCELERATION_POWER)
                * f64::from(INERTIA);
            fireball.previous_position = fireball.position;
            fireball.position += fireball.velocity;
        }
    }

    pub fn views(&self) -> Vec<FireballView> {
        self.fireballs
            .iter()
            .map(|fireball| FireballView {
                id: fireball.id,
                position: fireball.position,
                previous_position: fireball.previous_position,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ender_dragon::Sample;

    const PARTIALS: [f64; 4] = [0.0, 0.25, 0.5, 0.75];

    fn flying(id: u64, tick: i32) -> DragonView {
        let x = f64::from(tick);
        let mut history = FlightHistory::default();
        for _ in 0..HISTORY_SENT {
            history.record(70.0, -90.0);
        }
        DragonView {
            id,
            uuid: [0; 4],
            position: DVec3::new(x, 70.0, 0.0),
            previous_position: DVec3::new(x - 1.0, 70.0, 0.0),
            y_rot: -90.0,
            flap_time: tick as f32 * 0.05,
            o_flap_time: (tick - 1) as f32 * 0.05,
            history: history.recent(HISTORY_SENT),
            health: 200.0,
            hurt_time: 0,
            death_time: 0,
            sitting: false,
            phase: 0,
            parts: [DVec3::new(x, 70.0, 0.0); 8],
            tick_count: tick,
            nearest_crystal: None,
        }
    }

    fn at(view: &DragonView, partial: f64) -> DVec3 {
        view.previous_position.lerp(view.position, partial)
    }

    fn drawn(dragons: &ClientDragons) -> DragonView {
        dragons.views().remove(0)
    }

    #[test]
    fn a_drawn_dragon_never_steps_back_when_frames_arrive_off_the_client_tick() {
        let pattern = [1, 0, 2, 1, 1, 0, 2, 1, 1, 1];
        let mut dragons = ClientDragons::default();
        let mut server_tick = 0;
        dragons.receive(&[flying(1, server_tick)]);
        let mut newest = flying(1, server_tick);
        let mut drawn_x = Vec::new();
        let mut old_x = Vec::new();
        for _ in 0..4 {
            for &frames in &pattern {
                for _ in 0..frames {
                    server_tick += 1;
                    newest = flying(1, server_tick);
                    dragons.receive(&[newest.clone()]);
                }
                dragons.tick();
                let view = drawn(&dragons);
                for partial in PARTIALS {
                    drawn_x.push(at(&view, partial).x);
                    old_x.push(at(&newest, partial).x);
                }
            }
        }
        for pair in drawn_x.windows(2) {
            assert!(pair[1] >= pair[0], "drawn x went back: {pair:?}");
            assert!(pair[1] - pair[0] <= 1.5, "drawn x jumped: {pair:?}");
        }
        assert!(drawn_x.last().unwrap() > &30.0, "it keeps flying");
        assert!(
            old_x.windows(2).any(|pair| pair[1] < pair[0]),
            "the newest server view drawn at the client's partial goes back"
        );
    }

    #[test]
    fn the_drawn_dragon_reaches_a_stopped_server_position_within_a_few_ticks() {
        for stop in 10..13 {
            let mut dragons = ClientDragons::default();
            for tick in 0..=stop {
                dragons.receive(&[flying(1, tick)]);
                dragons.tick();
            }
            let mut stopped = flying(1, stop);
            stopped.previous_position = stopped.position;
            let target = stopped.position;
            let mut reached = None;
            for tick in 1..=5 {
                dragons.receive(&[stopped.clone()]);
                dragons.tick();
                if drawn(&dragons).position.distance(target) <= 1.0 / 4096.0 {
                    reached = Some(tick);
                    break;
                }
            }
            assert!(
                reached.is_some(),
                "stopped after {stop} ticks: not reached in 5"
            );
            for _ in 0..5 {
                dragons.receive(&[stopped.clone()]);
                dragons.tick();
                let view = drawn(&dragons);
                assert!(view.position.distance(target) <= 1.0 / 4096.0);
                assert!(view.previous_position.distance(target) <= 1.0 / 4096.0);
            }
        }
    }

    #[test]
    fn the_flight_history_is_recorded_from_the_drawn_body_each_client_tick_and_not_while_dying() {
        let mut dragons = ClientDragons::default();
        let mut view = flying(1, 0);
        view.history = vec![
            Sample {
                y: 999.0,
                y_rot: 45.0
            };
            HISTORY_SENT
        ];
        dragons.receive(&[view.clone()]);
        assert_eq!(
            drawn(&dragons).history,
            view.history,
            "the host's until the first tick"
        );
        for tick in 1..8 {
            let mut next = flying(1, tick);
            next.position.y = 70.0 + f64::from(tick);
            next.history = view.history.clone();
            dragons.receive(&[next]);
            dragons.tick();
            let shown = drawn(&dragons);
            assert_eq!(shown.history.len(), HISTORY_SENT);
            assert_eq!(
                shown.history[0].y, shown.position.y,
                "the newest sample is the drawn body"
            );
            assert_eq!(shown.history[1].y, shown.previous_position.y);
            assert_eq!(shown.history[0].y_rot, wrap_degrees(shown.y_rot));
            assert!(shown.history.iter().all(|sample| sample.y != 999.0));
        }
        let mut dying = flying(1, 8);
        dying.health = 0.0;
        dying.death_time = 1;
        dying.flap_time = dying.o_flap_time;
        dragons.receive(&[dying.clone()]);
        let before = drawn(&dragons).history;
        for tick in 9..14 {
            let mut next = dying.clone();
            next.position.y += f64::from(tick - 8);
            dragons.receive(&[next]);
            dragons.tick();
            assert_eq!(
                drawn(&dragons).history,
                before,
                "nothing recorded while dying"
            );
        }
    }

    #[test]
    fn the_wings_beat_at_the_host_rate_between_frames() {
        let mut dragons = ClientDragons::default();
        let mut view = flying(1, 0);
        view.flap_time = 0.3;
        view.o_flap_time = 0.2;
        dragons.receive(&[view.clone()]);
        let mut flaps = 0;
        for tick in 1..=10 {
            let reports = dragons.tick();
            let shown = drawn(&dragons);
            assert!((shown.flap_time - shown.o_flap_time - 0.1).abs() < 1e-5);
            assert!((shown.flap_time - (0.3 + 0.1 * tick as f32)).abs() < 1e-4);
            flaps += usize::from(reports[0].2);
        }
        assert_eq!(flaps, 1, "one beat in ten ticks at a step of 0.1");
        let mut held = view.clone();
        held.flap_time = 0.5;
        held.o_flap_time = 0.5;
        dragons.receive(&[held]);
        for _ in 0..3 {
            assert!(!dragons.tick()[0].2);
            let shown = drawn(&dragons);
            assert_eq!((shown.o_flap_time, shown.flap_time), (0.5, 0.5));
        }
    }

    #[test]
    fn a_dragon_missing_from_a_snapshot_is_forgotten_and_a_new_one_starts_at_its_position() {
        let mut dragons = ClientDragons::default();
        dragons.receive(&[flying(1, 0)]);
        dragons.tick();
        assert_eq!(dragons.views().len(), 1);
        dragons.receive(&[]);
        assert!(dragons.views().is_empty());
        assert!(dragons.tick().is_empty());
        let mut fresh = flying(2, 40);
        fresh.position = DVec3::new(12.25, 80.5, -3.75);
        dragons.receive(&[fresh.clone()]);
        let shown = dragons.views();
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].id, 2);
        assert_eq!(shown[0].position, fresh.position);
        assert_eq!(shown[0].previous_position, fresh.position);
        assert_eq!(shown[0].tick_count, 40);
        assert_eq!(shown[0].history, fresh.history);
    }

    #[test]
    fn a_dragon_more_than_64_blocks_away_snaps_instead_of_gliding() {
        let mut dragons = ClientDragons::default();
        let mut still = flying(1, 0);
        still.previous_position = still.position;
        for _ in 0..6 {
            dragons.receive(&[still.clone()]);
            dragons.tick();
        }
        let mut far = still.clone();
        far.position.x += 100.0;
        far.previous_position = far.position;
        let mut seen = Vec::new();
        for _ in 0..6 {
            dragons.receive(&[far.clone()]);
            dragons.tick();
            let view = drawn(&dragons);
            for partial in PARTIALS {
                seen.push(at(&view, partial).x);
            }
        }
        assert!(
            seen.iter()
                .all(|x| *x == still.position.x || *x == far.position.x),
            "no glide: {seen:?}"
        );
        assert_eq!(*seen.last().unwrap(), far.position.x);
        let mut near = far.clone();
        near.position.x += 10.0;
        let mut glide = Vec::new();
        for _ in 0..6 {
            dragons.receive(&[near.clone()]);
            dragons.tick();
            glide.push(drawn(&dragons).position.x);
        }
        assert!(
            glide
                .iter()
                .any(|x| *x > far.position.x && *x < near.position.x),
            "a shorter move glides: {glide:?}"
        );
    }

    #[test]
    fn the_beam_and_ray_clocks_follow_client_ticks_and_resync_to_the_host() {
        let mut dragons = ClientDragons::default();
        let mut view = flying(1, 100);
        view.nearest_crystal = Some(7);
        dragons.receive(&[view.clone()]);
        for _ in 0..3 {
            dragons.tick();
        }
        assert_eq!(drawn(&dragons).tick_count, 103);
        assert_eq!(drawn(&dragons).nearest_crystal, Some(7));
        view.tick_count = 101;
        dragons.receive(&[view.clone()]);
        assert_eq!(
            drawn(&dragons).tick_count,
            103,
            "close enough: the client clock runs on"
        );
        view.tick_count = 110;
        dragons.receive(&[view.clone()]);
        assert_eq!(drawn(&dragons).tick_count, 110, "resynced to the host");
        let mut dying = view.clone();
        dying.health = 0.0;
        dying.death_time = 5;
        dying.hurt_time = 3;
        dragons.receive(&[dying.clone()]);
        for _ in 0..4 {
            dragons.tick();
        }
        let shown = drawn(&dragons);
        assert_eq!(shown.death_time, 9);
        assert_eq!(
            (shown.health, shown.hurt_time),
            (0.0, 3),
            "the host's own values"
        );
        dying.death_time = 8;
        dragons.receive(&[dying.clone()]);
        assert_eq!(drawn(&dragons).death_time, 9);
        dying.death_time = 30;
        dragons.receive(&[dying.clone()]);
        assert_eq!(drawn(&dragons).death_time, 30);
        dying.death_time = DEATH_TICKS;
        dragons.receive(&[dying]);
        for _ in 0..5 {
            dragons.tick();
        }
        assert_eq!(drawn(&dragons).death_time, DEATH_TICKS);
    }

    struct HostFireball {
        position: DVec3,
        velocity: DVec3,
    }

    impl HostFireball {
        fn tick(&mut self) -> FireballView {
            use minecraftoss_entities::fireball::{ACCELERATION_POWER, INERTIA, normalize};
            self.velocity = (self.velocity + normalize(self.velocity) * ACCELERATION_POWER)
                * f64::from(INERTIA);
            let previous_position = self.position;
            self.position += self.velocity;
            FireballView {
                id: 9,
                position: self.position,
                previous_position,
            }
        }
    }

    #[test]
    fn a_drawn_fireball_moves_every_client_tick_and_snaps_when_far_off() {
        let mut host = HostFireball {
            position: DVec3::new(0.0, 80.0, 0.0),
            velocity: DVec3::new(0.1, 0.0, 0.0),
        };
        let mut fireballs = ClientFireballs::default();
        let mut newest = host.tick();
        fireballs.receive(&[newest]);
        assert_eq!(
            fireballs.views(),
            vec![newest],
            "a new one starts where the host has it"
        );
        let mut drawn = Vec::new();
        let mut old = Vec::new();
        for frames in [1, 0, 2, 1, 1, 0, 2, 1, 1, 1].repeat(3) {
            for _ in 0..frames {
                newest = host.tick();
                let kept = fireballs.views()[0].position;
                fireballs.receive(&[newest]);
                assert_eq!(fireballs.views()[0].position, kept, "on its track: no snap");
            }
            let before = fireballs.views()[0].position;
            fireballs.tick();
            let view = fireballs.views()[0];
            assert!(view.position.x > before.x, "it moves on every client tick");
            assert_eq!(view.previous_position, before);
            for partial in PARTIALS {
                drawn.push(at_fireball(&view, partial).x);
                old.push(at_fireball(&newest, partial).x);
            }
        }
        for pair in drawn.windows(2) {
            assert!(pair[1] >= pair[0], "the drawn fireball went back: {pair:?}");
        }
        assert!(
            old.windows(2).any(|pair| pair[1] < pair[0]),
            "the host's newest view drawn at the client's partial goes back"
        );
        let shown = fireballs.views()[0];
        assert!(
            shown.position.distance(host.position) <= 2.0 * host.velocity.length() + FIREBALL_SNAP,
            "it keeps up with the host"
        );
        host.position.z += 5.0;
        let far = host.tick();
        fireballs.receive(&[far]);
        assert_eq!(fireballs.views(), vec![far], "snapped to the host");
        fireballs.receive(&[]);
        assert!(fireballs.views().is_empty(), "a gone fireball is forgotten");
        fireballs.tick();
        assert!(fireballs.views().is_empty());
    }

    fn at_fireball(view: &FireballView, partial: f64) -> DVec3 {
        view.previous_position.lerp(view.position, partial)
    }
}
