use bevy::prelude::*;
use frame::{AppScreen, LifeStarted, UiDraw, ViewSubject};
use hud_iw4::{
    HUD_BLOOD_OVERLAY_LERP_RATE_DEFAULT, blood_overlay_lerp, get_health_fraction,
    should_draw_blood_overlay,
};
use net::{FrameClock, LocalPresentClient, PresentedSnapshot};

use crate::draw2d::{Draw2dCmd, Draw2dList, Draw2dOp, Draw2dProvenance, tessellate};
use crate::gaps::{GapCause, HudGap, HudPresentationGaps};
use crate::gpu_list::{PackedList, pack_splatter_alt};
use crate::images::{BLOOD_OVERLAY_COLOR, BLOOD_OVERLAY_MASK, HudImages, HudSampling};

#[derive(Resource, Default, Clone, Debug)]
pub(crate) struct HudRootVisible(pub Option<i32>);

#[derive(Resource, Default)]
pub(crate) struct BloodOverlayLatch {
    intensity: f32,
    pub(crate) packed: PackedList,
    last_gpu_intensity: Option<f32>,
    last_w: Option<f32>,
    last_h: Option<f32>,
    flash: HurtFlash,
}

const MC_HURT_FLASH_MS: i32 = 500;
const MC_HURT_FLASH_PEAK: f32 = 0.3;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct HurtFlash {
    last_event: Option<u32>,
    hit_at: Option<i32>,
}

impl HurtFlash {
    fn observe(&mut self, damage_event: u32, now_ms: i32) -> f32 {
        if self.last_event.is_some_and(|last| last != damage_event) {
            self.hit_at = Some(now_ms);
        }
        self.last_event = Some(damage_event);
        let Some(hit_at) = self.hit_at else {
            return 0.0;
        };
        let elapsed = now_ms.wrapping_sub(hit_at);
        if !(0..MC_HURT_FLASH_MS).contains(&elapsed) {
            self.hit_at = None;
        }
        mc_hurt_flash(elapsed)
    }
}

fn mc_hurt_flash(elapsed_ms: i32) -> f32 {
    if !(0..MC_HURT_FLASH_MS).contains(&elapsed_ms) {
        return 0.0;
    }
    let t = 1.0 - elapsed_ms as f32 / MC_HURT_FLASH_MS as f32;
    (MC_HURT_FLASH_PEAK * (t.powi(4) * std::f32::consts::PI).sin()).max(0.0)
}

fn next_intensity(
    intensity: f32,
    flash: &mut HurtFlash,
    on_minecraft: bool,
    health_frac: f32,
    damage_event: u32,
    now_ms: i32,
    frametime_ms: i32,
) -> f32 {
    if on_minecraft {
        return flash.observe(damage_event, now_ms);
    }
    blood_overlay_lerp(
        intensity,
        health_frac,
        frametime_ms,
        HUD_BLOOD_OVERLAY_LERP_RATE_DEFAULT,
    )
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BloodGpuJob {
    #[default]
    Idle,
    Hide,
    Show,
    Write,
}

fn request_hide(job: &mut BloodGpuJob, packed_empty: bool) {
    if !packed_empty {
        *job = BloodGpuJob::Hide;
    }
}

fn blood_shown(settings: Option<&frame::GameSettings>) -> bool {
    settings.is_none_or(|settings| settings.show_blood)
}

fn gpu_stretch_bits_unchanged(
    latch: &BloodOverlayLatch,
    intensity: f32,
    win_w: f32,
    win_h: f32,
) -> bool {
    !latch.packed.is_empty()
        && latch.last_gpu_intensity == Some(intensity)
        && latch.last_w == Some(win_w)
        && latch.last_h == Some(win_h)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn update_blood_overlay(
    cg_clock: Res<FrameClock>,
    screen: Res<AppScreen>,
    surface: Res<crate::surface::Hud2dSurface>,
    presented: Res<PresentedSnapshot>,
    local: Res<LocalPresentClient>,
    mut latch: ResMut<BloodOverlayLatch>,
    mut hud_images: ResMut<HudImages>,
    mut images: ResMut<Assets<Image>>,
    mut gaps: ResMut<HudPresentationGaps>,
    mut job: ResMut<BloodGpuJob>,
    mut started: MessageReader<LifeStarted>,
    view: Res<ViewSubject>,
    ui_draw: Option<Res<UiDraw>>,
    settings: Option<Res<frame::GameSettings>>,
    minecraft: Option<Res<frame::MinecraftUi>>,
) {
    *job = BloodGpuJob::Idle;
    for ev in started.read() {
        if ev.client == local.0.0 {
            latch.intensity = 0.0;
            latch.flash = HurtFlash::default();
        }
    }
    let in_game = matches!(*screen, AppScreen::InGame);
    let on_minecraft = minecraft.is_some_and(|ui| ui.active || ui.credits.is_some());
    let Some(ps) = presented.player(local.0) else {
        latch.intensity = 0.0;
        latch.flash = HurtFlash::default();
        gaps.clear(HudGap::BloodOverlay);
        request_hide(&mut job, latch.packed.is_empty());
        return;
    };
    let health_frac = get_health_fraction(ps.health, ps.max_health, ps.pm_type);
    let in_killcam = view.in_killcam();
    let in_killcam_hud_gate = in_killcam && ps.kill_cam_entity != playerstate_iw4::ENTITYNUM_NONE;

    if !in_game {
        latch.intensity = 0.0;
        latch.flash = HurtFlash::default();
        gaps.clear(HudGap::BloodOverlay);
        request_hide(&mut job, latch.packed.is_empty());
        return;
    }

    if !surface.is_ready() {
        gaps.clear(HudGap::BloodOverlay);
        request_hide(&mut job, latch.packed.is_empty());
        return;
    }
    let win_w = surface.width();
    let win_h = surface.height();

    let blood_enabled = blood_shown(settings.as_deref());
    let should_draw_hud = ui_draw.is_some_and(|d| d.0);
    let should_draw = should_draw_blood_overlay(
        blood_enabled,
        should_draw_hud,
        in_killcam_hud_gate,
        ps.pm_type,
    );
    if !should_draw {
        if !blood_enabled {
            latch.intensity = 0.0;
            latch.flash = HurtFlash::default();
        } else if on_minecraft {
            latch.flash.observe(ps.damage_event, cg_clock.time());
        }
        gaps.clear(HudGap::BloodOverlay);
        request_hide(&mut job, latch.packed.is_empty());
        return;
    }

    let frametime_ms = cg_clock.frametime();
    let latch = &mut *latch;
    latch.intensity = next_intensity(
        latch.intensity,
        &mut latch.flash,
        on_minecraft,
        health_frac,
        ps.damage_event,
        cg_clock.time(),
        frametime_ms,
    );

    if latch.intensity == 0.0 {
        gaps.clear(HudGap::BloodOverlay);
        request_hide(&mut job, latch.packed.is_empty());
        return;
    }

    if gpu_stretch_bits_unchanged(&latch, latch.intensity, win_w, win_h) {
        gaps.clear(HudGap::BloodOverlay);
        *job = BloodGpuJob::Show;
        return;
    }

    let binding = match hud_images.blood_material_binding() {
        Ok(binding) => binding,
        Err(detail) => {
            gaps.raise(GapCause::BloodOverlayMaterialUnsupported {
                detail: detail.to_owned(),
            });
            request_hide(&mut job, latch.packed.is_empty());
            return;
        }
    };

    let Some(color) = hud_images.get_sampled_with_sampler(
        crate::images::HUD_CHROME_NAMESPACE,
        BLOOD_OVERLAY_COLOR,
        HudSampling::Data,
        Some(binding.color_sampler),
        &mut images,
    ) else {
        gaps.raise(GapCause::BloodOverlayImageMissing {
            name: BLOOD_OVERLAY_COLOR.to_owned(),
            miss: hud_images.miss_reason(),
        });
        request_hide(&mut job, latch.packed.is_empty());
        return;
    };
    let Some(mask) = hud_images.get_sampled_with_sampler(
        crate::images::HUD_CHROME_NAMESPACE,
        BLOOD_OVERLAY_MASK,
        HudSampling::Data,
        Some(binding.mask_sampler),
        &mut images,
    ) else {
        gaps.raise(GapCause::BloodOverlayImageMissing {
            name: BLOOD_OVERLAY_MASK.to_owned(),
            miss: hud_images.miss_reason(),
        });
        request_hide(&mut job, latch.packed.is_empty());
        return;
    };

    gaps.clear(HudGap::BloodOverlay);
    let list = Draw2dList {
        cmds: vec![Draw2dCmd {
            material_namespace: crate::images::HUD_CHROME_NAMESPACE,
            x: 0.0,
            y: 0.0,
            w: win_w,
            h: win_h,
            s0: 0.0,
            t0: 0.0,
            s1: 1.0,
            t1: 1.0,
            color: [1.0, 1.0, 1.0, latch.intensity],
            material: BLOOD_OVERLAY_COLOR.into(),
            op: Draw2dOp::StretchPic,
            provenance: Draw2dProvenance::CgDraw {
                site: "blood_overlay",
            },
            layer: 0,
        }],
    };
    let Some(quad) = tessellate(&list).0.into_iter().next() else {
        request_hide(&mut job, latch.packed.is_empty());
        return;
    };
    latch.packed = pack_splatter_alt(&quad, color, mask, binding.state);
    latch.last_w = Some(win_w);
    latch.last_h = Some(win_h);
    latch.last_gpu_intensity = Some(latch.intensity);
    *job = BloodGpuJob::Write;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blood_follows_the_show_blood_option() {
        let mut settings = frame::GameSettings::default();
        assert!(blood_shown(Some(&settings)));
        settings.show_blood = false;
        assert!(!blood_shown(Some(&settings)));
        assert!(blood_shown(None));
    }

    #[test]
    fn turning_blood_off_hides_the_overlay_even_at_low_health() {
        let settings = frame::GameSettings {
            show_blood: false,
            ..Default::default()
        };
        assert!(!should_draw_blood_overlay(
            blood_shown(Some(&settings)),
            true,
            false,
            0
        ));
        assert!(should_draw_blood_overlay(
            blood_shown(Some(&frame::GameSettings::default())),
            true,
            false,
            0
        ));
    }

    #[test]
    fn the_first_damage_count_seen_does_not_flash() {
        let mut flash = HurtFlash::default();
        assert_eq!(flash.observe(7, 1000), 0.0);
        for now in (1016..2000).step_by(16) {
            assert_eq!(flash.observe(7, now), 0.0);
        }
    }

    #[test]
    fn a_hit_flashes_faintly_and_fades_within_half_a_second() {
        let mut flash = HurtFlash::default();
        flash.observe(3, 1000);
        assert_eq!(flash.observe(4, 2000), 0.0);
        let peak = flash.observe(4, 2080);
        assert!(peak > 0.25 && peak <= MC_HURT_FLASH_PEAK, "{peak}");
        let tail = flash.observe(4, 2499);
        assert!(tail < 0.05, "{tail}");
        assert_eq!(flash.observe(4, 2500), 0.0);
        assert_eq!(flash.observe(4, 4000), 0.0);
    }

    #[test]
    fn the_flash_peak_stays_far_below_the_mw2_low_health_overlay() {
        let highest = (0..MC_HURT_FLASH_MS)
            .map(mc_hurt_flash)
            .fold(0.0f32, f32::max);
        assert!(highest <= 0.35, "{highest}");
        let mw2 = blood_overlay_lerp(0.0, 0.1, 16, HUD_BLOOD_OVERLAY_LERP_RATE_DEFAULT);
        assert!(mw2 > 0.85, "{mw2}");
        assert!(highest * 2.5 < mw2);
    }

    #[test]
    fn a_wrapped_damage_counter_still_flashes() {
        let mut flash = HurtFlash::default();
        flash.observe(u32::MAX, 1000);
        flash.observe(0, 1100);
        assert!(flash.observe(0, 1180) > 0.25);
    }

    #[test]
    fn low_health_alone_shows_no_flash_on_minecraft() {
        let mut flash = HurtFlash::default();
        let mut intensity = 0.0;
        for frame in 0..600 {
            intensity = next_intensity(intensity, &mut flash, true, 0.1, 9, frame * 16, 16);
            assert_eq!(intensity, 0.0);
        }
        let mut mw2 = HurtFlash::default();
        let lasting = next_intensity(0.0, &mut mw2, false, 0.1, 9, 0, 16);
        assert!(lasting > 0.85, "{lasting}");
    }

    #[test]
    fn a_new_life_forgets_the_last_hit() {
        let mut latch = BloodOverlayLatch::default();
        latch.flash.observe(1, 1000);
        assert_eq!(latch.flash.observe(2, 1100), 0.0);
        assert!(latch.flash.observe(2, 1180) > 0.25);
        latch.flash = HurtFlash::default();
        assert_eq!(latch.flash.observe(0, 1200), 0.0);
        assert_eq!(latch.flash.observe(0, 1280), 0.0);
    }
}
