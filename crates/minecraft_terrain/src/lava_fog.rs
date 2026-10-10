use crate::day_cycle::{SkyState, Skybox};
use glam::Vec3;

pub const COLOR: u32 = 0x99_1A_00;
const SPECTATOR_START: f32 = -8.0;
const FIRE_RESISTANT_END: f32 = 5.0;
const START: f32 = 0.25;
const END: f32 = 1.0;

pub fn color() -> Vec3 {
    Vec3::new(
        ((COLOR >> 16) & 255) as f32 / 255.0,
        ((COLOR >> 8) & 255) as f32 / 255.0,
        (COLOR & 255) as f32 / 255.0,
    )
}

pub fn in_lava(
    mut sky: SkyState,
    spectator: bool,
    fire_resistant: bool,
    render_distance: f32,
) -> SkyState {
    let (start, end) = if spectator {
        (SPECTATOR_START, render_distance * 0.5)
    } else if fire_resistant {
        (0.0, FIRE_RESISTANT_END)
    } else {
        (START, END)
    };
    sky.fog = color();
    sky.fog_start = start;
    sky.fog_end = end;
    sky.sky_fog_end = end;
    sky.cloud_fog_end = end;
    sky.cloud_alpha = 0.0;
    sky.skybox = Skybox::None;
    sky
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day_cycle::DayCycle;

    fn day() -> SkyState {
        DayCycle::default().sample()
    }

    #[test]
    fn lava_hides_everything_past_a_block_in_orange() {
        let sky = in_lava(day(), false, false, 192.0);
        assert_eq!(sky.fog, Vec3::new(0.6, 26.0 / 255.0, 0.0));
        assert_eq!((sky.fog_start, sky.fog_end), (0.25, 1.0));
        assert_eq!((sky.sky_fog_end, sky.cloud_fog_end), (1.0, 1.0));
        assert_eq!(sky.skybox, Skybox::None);
        assert_eq!(sky.cloud_alpha, 0.0);
    }

    #[test]
    fn fire_resistance_sees_five_blocks_and_spectators_half_the_view() {
        let resistant = in_lava(day(), false, true, 192.0);
        assert_eq!((resistant.fog_start, resistant.fog_end), (0.0, 5.0));
        let spectator = in_lava(day(), true, true, 192.0);
        assert_eq!((spectator.fog_start, spectator.fog_end), (-8.0, 96.0));
    }
}
