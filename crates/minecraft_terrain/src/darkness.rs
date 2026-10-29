use crate::day_cycle::SkyState;
use crate::lava_fog;

pub const BLEND_TICKS: u32 = 22;
pub const TICK_SECONDS: f64 = 1.0 / 20.0;
const FOG_DISTANCE: f32 = 15.0;
const FOG_START: f32 = 0.75;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DarknessBlend {
    factor: f32,
    previous: f32,
}

impl DarknessBlend {
    pub fn tick(&mut self, holding: bool) {
        self.previous = self.factor;
        let target = if holding { 1.0 } else { 0.0 };
        let step = 1.0 / BLEND_TICKS as f32;
        self.factor += (target - self.factor).clamp(-step, step);
    }

    pub fn factor(&self, partial: f32) -> f32 {
        self.previous + partial * (self.factor - self.previous)
    }
}

pub fn holds(remaining: u32) -> bool {
    remaining > BLEND_TICKS
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Darkness {
    blend: Option<DarknessBlend>,
    given: Option<(u32, u32)>,
    since: u32,
    clock: f64,
}

impl Darkness {
    pub fn advance(&mut self, seconds: f64, effect: Option<u32>, view: u32) {
        let given = effect.map(|ticks| (ticks, view));
        if given != self.given {
            self.given = given;
            self.since = 0;
        }
        if effect.is_none() {
            self.blend = None;
        }
        self.clock += seconds.clamp(0.0, 1.0);
        while self.clock >= TICK_SECONDS {
            self.clock -= TICK_SECONDS;
            self.tick();
        }
    }

    fn tick(&mut self) {
        let Some((given, _)) = self.given else {
            return;
        };
        self.since = self.since.saturating_add(1);
        let remaining = given.saturating_sub(self.since);
        self.blend
            .get_or_insert_with(DarknessBlend::default)
            .tick(holds(remaining));
    }

    pub fn partial(&self) -> f32 {
        (self.clock / TICK_SECONDS).clamp(0.0, 1.0) as f32
    }

    pub fn blend(&self) -> Option<f32> {
        let partial = self.partial();
        self.given
            .map(|_| self.blend.map_or(0.0, |blend| blend.factor(partial)))
    }
}

pub fn fog(mut sky: SkyState, blend: f32, render_distance: f32) -> SkyState {
    let distance = render_distance + blend * (FOG_DISTANCE - render_distance);
    sky.fog_start = distance * FOG_START;
    sky.fog_end = distance;
    sky.sky_fog_end = distance;
    sky.cloud_fog_end = distance;
    sky
}

pub fn camera_fog(
    sky: SkyState,
    lava: Option<(bool, bool)>,
    powder_snow: Option<bool>,
    darkness: Option<f32>,
    render_distance: f32,
) -> SkyState {
    match (lava, powder_snow, darkness) {
        (Some((spectator, fire_resistant)), _, _) => {
            lava_fog::in_lava(sky, spectator, fire_resistant, render_distance)
        }
        (None, Some(spectator), _) => lava_fog::in_powder_snow(sky, spectator, render_distance),
        (None, None, Some(blend)) => fog(sky, blend, render_distance),
        (None, None, None) => sky,
    }
}

pub fn fog_darkening(void: f32, blend: f32) -> f32 {
    let light = 1.0 - void.max(blend);
    light * light
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day_cycle::DayCycle;

    const RENDER_DISTANCE: f32 = 192.0;

    fn day() -> SkyState {
        DayCycle::default().sample()
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_blend_ramps_over_22_ticks_and_fades_before_the_end() {
        let mut blend = DarknessBlend::default();
        blend.tick(true);
        assert_eq!(blend.factor(0.0), 0.0);
        assert!(close(blend.factor(0.5), 0.5 / 22.0));
        for tick in 2..=22 {
            blend.tick(true);
            assert!(close(blend.factor(1.0), tick as f32 / 22.0), "{tick}");
        }
        blend.tick(true);
        assert_eq!(blend.factor(1.0), 1.0);

        let mut duration = 30_u32;
        let mut fading = None;
        while duration > 0 {
            duration -= 1;
            blend.tick(holds(duration));
            if fading.is_none() && blend.factor(1.0) < 1.0 {
                fading = Some(30 - duration);
            }
        }
        assert_eq!(
            fading,
            Some(8),
            "an effect with 30 ticks left fades from its 8th tick on"
        );
        assert_eq!(blend.factor(1.0), 0.0, "and is gone when it ends");

        let mut infinite = Darkness::default();
        for _ in 0..200 {
            infinite.advance(TICK_SECONDS, Some(u32::MAX), 0);
        }
        assert_eq!(infinite.blend(), Some(1.0));
        assert!(holds(i32::MAX as u32));
        assert!(!holds(22) && holds(23));
    }

    #[test]
    fn the_viewer_counts_the_duration_down_between_updates_and_milk_ends_it_at_once() {
        let mut darkness = Darkness::default();
        assert_eq!(darkness.blend(), None);
        darkness.advance(0.0, Some(600), 0);
        assert_eq!(
            darkness.blend(),
            Some(0.0),
            "a fresh effect starts dark-free"
        );
        for _ in 0..30 {
            darkness.advance(TICK_SECONDS, Some(600), 0);
        }
        assert_eq!(darkness.blend(), Some(1.0));
        darkness.advance(TICK_SECONDS * 0.5, Some(600), 0);
        assert!(close(darkness.partial(), 0.5));

        let mut friend = Darkness::default();
        let mut host = Darkness::default();
        for tick in 0..60 {
            friend.advance(TICK_SECONDS, Some(40), 1);
            host.advance(TICK_SECONDS, Some(40_u32.saturating_sub(tick)), 0);
            assert_eq!(friend.blend(), host.blend(), "{tick}");
        }
        assert_eq!(friend.blend(), Some(0.0));

        darkness.advance(0.0, None, 0);
        assert_eq!(darkness.blend(), None, "milk takes the look away at once");
        darkness.advance(TICK_SECONDS, Some(600), 0);
        assert!(
            close(darkness.blend().unwrap(), 0.5 / 22.0),
            "and a new effect blends in again from 0, half a tick on"
        );
    }

    #[test]
    fn the_same_length_given_again_in_a_new_view_counts_down_from_the_start() {
        let mut darkness = Darkness::default();
        for _ in 0..250 {
            darkness.advance(TICK_SECONDS, Some(260), 1);
        }
        assert!(darkness.blend().unwrap() < 1.0, "the first grant fades");
        let mut again = Darkness::default();
        for tick in 0..250 {
            let view = if tick < 61 { 1 } else { 2 };
            again.advance(TICK_SECONDS, Some(260), view);
        }
        assert_eq!(again.blend(), Some(1.0), "the second grant keeps it dark");
    }

    #[test]
    fn fog_closes_to_fifteen_blocks() {
        let dark = fog(day(), 1.0, RENDER_DISTANCE);
        assert_eq!((dark.fog_start, dark.fog_end), (11.25, 15.0));
        assert_eq!((dark.sky_fog_end, dark.cloud_fog_end), (15.0, 15.0));
        let clear = fog(day(), 0.0, RENDER_DISTANCE);
        assert_eq!((clear.fog_start, clear.fog_end), (144.0, 192.0));
        assert_eq!((clear.sky_fog_end, clear.cloud_fog_end), (192.0, 192.0));
        let half = fog(day(), 0.5, RENDER_DISTANCE);
        assert_eq!(half.fog_end, (RENDER_DISTANCE + 15.0) / 2.0);
        assert_eq!(half.fog_start, half.fog_end * 0.75);

        let lava = camera_fog(
            day(),
            Some((false, false)),
            None,
            Some(1.0),
            RENDER_DISTANCE,
        );
        assert_eq!((lava.fog_start, lava.fog_end), (0.25, 1.0));
        let snow = camera_fog(day(), None, Some(false), Some(1.0), RENDER_DISTANCE);
        assert_eq!((snow.fog_start, snow.fog_end), (0.0, 2.0));
        let dark = camera_fog(day(), None, None, Some(1.0), RENDER_DISTANCE);
        assert_eq!((dark.fog_start, dark.fog_end), (11.25, 15.0));
        let plain = camera_fog(day(), None, None, None, RENDER_DISTANCE);
        assert_eq!(
            (plain.fog_start, plain.fog_end),
            (day().fog_start, day().fog_end)
        );
    }

    #[test]
    fn fog_colour_darkens_by_the_square() {
        assert_eq!(fog_darkening(0.0, 0.0), 1.0);
        assert_eq!(fog_darkening(0.0, 0.5), 0.25);
        assert_eq!(fog_darkening(0.0, 1.0), 0.0);
        assert_eq!(
            fog_darkening(0.75, 0.5),
            0.0625,
            "the void wins when it is darker"
        );
        assert_eq!(fog_darkening(0.5, 0.75), 0.0625);
    }
}
