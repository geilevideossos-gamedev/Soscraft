use std::collections::HashMap;

use bevy::prelude::*;
use minecraft_terrain::elytra_render::{ElytraAnim, WingAngles, flying_roll};
use net::{FrameClock, PresentedSnapshot};
use playerstate_iw4::pm_flags::MC_GLIDING;
use playerstate_iw4::{PM_TYPE_DEAD, PlayerState, eflags};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlideInput {
    pub gliding: bool,
    pub crouching: bool,
    pub motion: [f64; 3],
    pub look: [f64; 3],
    pub pitch_deg: f32,
}

pub fn glide_input(ps: &PlayerState) -> GlideInput {
    let per_tick = f64::from(sim::voxel::BLOCK) * 20.0;
    let [pitch, yaw] = [ps.viewangles[0], ps.viewangles[1]].map(|deg| f64::from(deg).to_radians());
    GlideInput {
        gliding: ps.pm_flags & MC_GLIDING != 0 && ps.pm_type < PM_TYPE_DEAD,
        crouching: ps.e_flags & eflags::DUCK != 0,
        motion: ps.velocity.map(|speed| f64::from(speed) / per_tick),
        look: [
            pitch.cos() * yaw.cos(),
            pitch.cos() * yaw.sin(),
            -pitch.sin(),
        ],
        pitch_deg: ps.viewangles[0],
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyGlide {
    pub anim: ElytraAnim,
    pub input: GlideInput,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlidePose {
    pub scale: f32,
    pub roll: Option<f32>,
    pub pitch_deg: f32,
    pub wings: WingAngles,
    pub crouching: bool,
}

impl BodyGlide {
    pub fn advance(&mut self, seconds: f32, input: GlideInput) {
        self.input = input;
        self.anim
            .advance(seconds, input.gliding, input.crouching, input.motion);
    }

    pub fn pose(&self) -> GlidePose {
        let gliding = self.anim.gliding;
        GlidePose {
            scale: self.anim.scale(),
            roll: gliding
                .then(|| flying_roll(self.input.look, self.input.motion))
                .flatten(),
            pitch_deg: self.input.pitch_deg,
            wings: self.anim.wings(),
            crouching: self.input.crouching,
        }
    }
}

#[derive(Resource, Default)]
pub struct McGlide {
    by_client: HashMap<u32, BodyGlide>,
}

impl McGlide {
    pub fn get(&self, client: u32) -> Option<&BodyGlide> {
        self.by_client.get(&client)
    }

    pub fn pose(&self, client: u32) -> Option<GlidePose> {
        self.get(client).map(BodyGlide::pose)
    }

    pub fn step(&mut self, seconds: f32, clients: impl IntoIterator<Item = (u32, GlideInput)>) {
        let mut seen = Vec::new();
        for (client, input) in clients {
            self.by_client
                .entry(client)
                .or_default()
                .advance(seconds, input);
            seen.push(client);
        }
        self.retain(&seen);
    }

    pub fn retain(&mut self, clients: &[u32]) {
        self.by_client.retain(|client, _| clients.contains(client));
    }

    pub fn clear(&mut self) {
        self.by_client.clear();
    }
}

pub fn step_body_glides(
    time: Res<Time>,
    clock: Option<Res<FrameClock>>,
    presented: Res<PresentedSnapshot>,
    mut glide: ResMut<McGlide>,
) {
    let seconds = clock
        .as_ref()
        .map(|clock| clock.frametime_secs())
        .unwrap_or_else(|| time.delta_secs());
    let Some(snapshot) = presented.snapshot() else {
        glide.clear();
        return;
    };
    let clients = snapshot.players.iter().filter_map(|(client, _)| {
        let ps = presented.player(*client)?;
        Some((client.0, glide_input(ps)))
    });
    glide.step(seconds, clients);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(pm_flags: u32, velocity: [f32; 3]) -> PlayerState {
        PlayerState {
            pm_flags,
            velocity,
            ..PlayerState::ZERO
        }
    }

    #[test]
    fn a_glider_is_read_from_its_pm_flags_and_a_dead_one_is_not() {
        assert!(glide_input(&player(MC_GLIDING, [0.0; 3])).gliding);
        assert!(!glide_input(&player(0, [0.0; 3])).gliding);
        let dead = PlayerState {
            pm_type: PM_TYPE_DEAD,
            ..player(MC_GLIDING, [0.0; 3])
        };
        assert!(!glide_input(&dead).gliding);
        let ducked = PlayerState {
            e_flags: eflags::DUCK,
            ..player(0, [0.0; 3])
        };
        assert!(glide_input(&ducked).crouching);
        assert!(!glide_input(&player(0, [0.0; 3])).crouching);
    }

    #[test]
    fn velocity_becomes_blocks_per_tick() {
        let input = glide_input(&player(MC_GLIDING, [720.0, -360.0, 72.0]));
        assert_eq!(input.motion, [1.0, -0.5, 0.1]);
        let looking = PlayerState {
            viewangles: [30.0, 90.0, 0.0],
            ..PlayerState::ZERO
        };
        let input = glide_input(&looking);
        let want = [0.0, 30f64.to_radians().cos(), -0.5];
        assert!(
            input
                .look
                .iter()
                .zip(want)
                .all(|(got, want)| (got - want).abs() < 1e-6),
            "{:?}",
            input.look
        );
        assert_eq!(input.pitch_deg, 30.0);
    }

    fn gliding(client: u32) -> (u32, GlideInput) {
        (client, glide_input(&player(MC_GLIDING, [720.0, 0.0, 0.0])))
    }

    fn standing(client: u32) -> (u32, GlideInput) {
        (client, glide_input(&player(0, [0.0; 3])))
    }

    #[test]
    fn each_client_keeps_its_own_glide_and_gone_clients_are_dropped() {
        let mut glide = McGlide::default();
        for _ in 0..4 {
            glide.step(0.05, [gliding(0), standing(3)]);
        }
        let flyer = glide.get(0).unwrap();
        let walker = glide.get(3).unwrap();
        assert_eq!(flyer.anim.fall_fly_ticks, 4);
        assert_eq!(walker.anim.fall_fly_ticks, 0);
        assert!(flyer.anim.now.z < walker.anim.now.z);
        assert!(glide.pose(0).unwrap().scale > 0.0);
        assert_eq!(glide.pose(3).unwrap().scale, 0.0);
        glide.step(0.05, [standing(3), standing(5)]);
        assert!(glide.get(0).is_none());
        assert!(glide.get(3).is_some() && glide.get(5).is_some());
        assert_eq!(glide.get(5).unwrap().anim.now.x, WingAngles::REST.x * 0.3);
        glide.clear();
        assert!(glide.pose(3).is_none());
    }

    #[test]
    fn the_wings_spread_over_frames_while_gliding_and_fold_after_landing() {
        let mut glide = McGlide::default();
        for _ in 0..60 {
            glide.step(1.0 / 60.0, [gliding(1)]);
        }
        let pose = glide.pose(1).unwrap();
        assert!(
            (pose.wings.z - WingAngles::SPREAD.z).abs() < 0.01,
            "{pose:?}"
        );
        assert_eq!(pose.scale, 1.0);
        assert_eq!(pose.roll, Some(0.0));
        glide.step(1.0 / 60.0, [standing(1)]);
        let landed = glide.pose(1).unwrap();
        assert_eq!(landed.scale, 0.0);
        assert_eq!(landed.roll, None);
        for _ in 0..60 {
            glide.step(1.0 / 60.0, [standing(1)]);
        }
        let folded = glide.pose(1).unwrap().wings;
        assert!((folded.z - WingAngles::REST.z).abs() < 0.01, "{folded:?}");
        let crouched = (
            1,
            glide_input(&PlayerState {
                e_flags: eflags::DUCK,
                ..PlayerState::ZERO
            }),
        );
        for _ in 0..60 {
            glide.step(1.0 / 60.0, [crouched]);
        }
        let pose = glide.pose(1).unwrap();
        assert!(pose.crouching);
        assert!((pose.wings.x - WingAngles::CROUCH.x).abs() < 0.01);
    }
}
