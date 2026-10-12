use crate::day_cycle::{SkyState, Skybox};
use glam::Vec3;

pub const COLOR: u32 = 0x99_1A_00;
pub const POWDER_SNOW_COLOR: u32 = 0x9F_BE_CC;
const POWDER_SNOW_END: f32 = 2.0;
const SPECTATOR_START: f32 = -8.0;
const FIRE_RESISTANT_END: f32 = 5.0;
const START: f32 = 0.25;
const END: f32 = 1.0;

pub fn color() -> Vec3 {
    rgb(COLOR)
}

fn rgb(color: u32) -> Vec3 {
    Vec3::new(
        ((color >> 16) & 255) as f32 / 255.0,
        ((color >> 8) & 255) as f32 / 255.0,
        (color & 255) as f32 / 255.0,
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

pub fn in_powder_snow(mut sky: SkyState, spectator: bool, render_distance: f32) -> SkyState {
    let (start, end) = if spectator {
        (SPECTATOR_START, render_distance * 0.5)
    } else {
        (0.0, POWDER_SNOW_END)
    };
    sky.fog = rgb(POWDER_SNOW_COLOR);
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
    fn powder_snow_fog_is_two_blocks_of_pale_blue() {
        let sky = in_powder_snow(day(), false, 192.0);
        assert_eq!(sky.fog, Vec3::new(159.0 / 255.0, 190.0 / 255.0, 204.0 / 255.0));
        assert_eq!((sky.fog_start, sky.fog_end), (0.0, 2.0));
        assert_eq!((sky.sky_fog_end, sky.cloud_fog_end), (2.0, 2.0));
        assert_eq!(sky.skybox, Skybox::None);
        assert_eq!(sky.cloud_alpha, 0.0);
        let spectator = in_powder_snow(day(), true, 192.0);
        assert_eq!((spectator.fog_start, spectator.fog_end), (-8.0, 96.0));
    }

    #[test]
    fn fire_resistance_sees_five_blocks_and_spectators_half_the_view() {
        let resistant = in_lava(day(), false, true, 192.0);
        assert_eq!((resistant.fog_start, resistant.fog_end), (0.0, 5.0));
        let spectator = in_lava(day(), true, true, 192.0);
        assert_eq!((spectator.fog_start, spectator.fog_end), (-8.0, 96.0));
    }
}
