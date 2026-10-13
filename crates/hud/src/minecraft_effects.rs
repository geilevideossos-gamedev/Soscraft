use std::cmp::Ordering;

use frame::MinecraftUi;
use frame::minecraft_ui::McEffect;

use crate::minecraft_gui::Painter;

const BACKGROUND: &str = "hud/effect_background";
const BACKGROUND_AMBIENT: &str = "hud/effect_background_ambient";
const INFINITE: i32 = -1;
const LONG: i32 = 32147;
const BLINK_TICKS: i32 = 200;
const SIZE: i32 = 24;
const ICON: i32 = 18;
const STEP: i32 = 25;
const TOP: i32 = 1;
const HARMFUL_ROW: i32 = 26;

const EFFECTS: [(&str, bool, u32); 40] = [
    ("speed", true, 3402751),
    ("slowness", false, 9154528),
    ("haste", true, 14270531),
    ("mining_fatigue", false, 4866583),
    ("strength", true, 16762624),
    ("instant_health", true, 16262179),
    ("instant_damage", false, 11101546),
    ("jump_boost", true, 16646020),
    ("nausea", false, 5578058),
    ("regeneration", true, 13458603),
    ("resistance", true, 9520880),
    ("fire_resistance", true, 16750848),
    ("water_breathing", true, 10017472),
    ("invisibility", true, 16185078),
    ("blindness", false, 2039587),
    ("night_vision", true, 12779366),
    ("hunger", false, 5797459),
    ("weakness", false, 4738376),
    ("poison", false, 8889187),
    ("wither", false, 7561558),
    ("health_boost", true, 16284963),
    ("absorption", true, 2445989),
    ("saturation", true, 16262179),
    ("glowing", false, 9740385),
    ("levitation", false, 13565951),
    ("luck", true, 5882118),
    ("unluck", false, 12624973),
    ("slow_falling", true, 15978425),
    ("conduit_power", true, 1950417),
    ("dolphins_grace", true, 8954814),
    ("bad_omen", false, 745784),
    ("hero_of_the_village", true, 4521796),
    ("darkness", false, 2696993),
    ("trial_omen", false, 1484454),
    ("raid_omen", false, 14565464),
    ("wind_charged", false, 12438015),
    ("weaving", false, 7891290),
    ("oozing", false, 10092451),
    ("infested", false, 9214860),
    ("breath_of_the_nautilus", true, 65518),
];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) sprite: String,
    pub(crate) background: &'static str,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) alpha: f32,
}

fn path(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

fn kind(effect: &McEffect) -> (bool, u32) {
    EFFECTS
        .iter()
        .find(|(known, _, _)| *known == path(&effect.id))
        .map_or((false, 0), |&(_, beneficial, color)| (beneficial, color))
}

pub(crate) fn compare(a: &McEffect, b: &McEffect) -> Ordering {
    let color = |effect: &McEffect| kind(effect).1;
    if (a.duration > LONG && b.duration > LONG) || (a.ambient && b.ambient) {
        a.ambient.cmp(&b.ambient).then(color(a).cmp(&color(b)))
    } else {
        let infinite = |effect: &McEffect| effect.duration == INFINITE;
        a.ambient
            .cmp(&b.ambient)
            .then(infinite(a).cmp(&infinite(b)))
            .then(a.duration.cmp(&b.duration))
            .then(color(a).cmp(&color(b)))
    }
}

pub(crate) fn alpha(effect: &McEffect) -> f32 {
    if effect.ambient || effect.duration == INFINITE || effect.duration > BLINK_TICKS {
        return 1.0;
    }
    let duration = effect.duration;
    let left = 10 - duration / 20;
    let base = (duration as f32 / 10.0 / 5.0 * 0.5).clamp(0.0, 0.5);
    let pulse = (duration as f32 * std::f32::consts::PI / 5.0).cos()
        * (left as f32 / 10.0 * 0.25).clamp(0.0, 0.25);
    (base + pulse).clamp(0.0, 1.0)
}

pub(crate) fn layout(effects: &[McEffect], width: i32) -> Vec<Placed> {
    let mut sorted: Vec<&McEffect> = effects.iter().collect();
    sorted.sort_by(|a, b| compare(b, a));
    let (mut beneficial, mut harmful) = (0, 0);
    sorted
        .into_iter()
        .map(|effect| {
            let (x, y) = if kind(effect).0 {
                beneficial += 1;
                (width - STEP * beneficial, TOP)
            } else {
                harmful += 1;
                (width - STEP * harmful, TOP + HARMFUL_ROW)
            };
            Placed {
                sprite: format!("mob_effect/{}", path(&effect.id)),
                background: if effect.ambient {
                    BACKGROUND_AMBIENT
                } else {
                    BACKGROUND
                },
                x,
                y,
                alpha: alpha(effect),
            }
        })
        .collect()
}

pub(crate) fn white(alpha: f32) -> u32 {
    (((alpha * 255.0).floor() as u32) << 24) | 0x00FF_FFFF
}

pub(crate) fn draw_effects(p: &mut Painter<'_>, ui: &MinecraftUi) {
    for placed in layout(&ui.effects, p.space.width) {
        p.sprite(placed.background, placed.x, placed.y, SIZE, SIZE);
        p.sprite_tinted(
            &placed.sprite,
            placed.x + 3,
            placed.y + 3,
            ICON,
            ICON,
            white(placed.alpha),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect(id: &str, duration: i32) -> McEffect {
        McEffect {
            id: format!("minecraft:{id}"),
            amplifier: 0,
            duration,
            ambient: false,
        }
    }

    fn spots(placed: &[Placed]) -> Vec<(&str, i32, i32)> {
        placed
            .iter()
            .map(|p| (p.sprite.as_str(), p.x, p.y))
            .collect()
    }

    #[test]
    fn beneficial_effects_fill_the_top_row_and_harmful_ones_the_second_from_the_right() {
        let effects = [
            effect("poison", 400),
            effect("hero_of_the_village", 48000),
            effect("regeneration", 300),
            effect("bad_omen", 120000),
            effect("darkness", 260),
            effect("absorption", INFINITE),
        ];
        assert_eq!(
            spots(&layout(&effects, 320)),
            vec![
                ("mob_effect/absorption", 295, 1),
                ("mob_effect/hero_of_the_village", 270, 1),
                ("mob_effect/bad_omen", 295, 27),
                ("mob_effect/poison", 270, 27),
                ("mob_effect/regeneration", 245, 1),
                ("mob_effect/darkness", 245, 27),
            ]
        );
        assert!(
            layout(&effects, 320)
                .iter()
                .all(|p| p.background == BACKGROUND)
        );
    }

    #[test]
    fn long_effects_sort_by_colour_and_short_ones_by_time_left() {
        let order = |effects: &[McEffect]| -> Vec<String> {
            let mut sorted: Vec<&McEffect> = effects.iter().collect();
            sorted.sort_by(|a, b| compare(b, a));
            sorted.iter().map(|e| path(&e.id).to_owned()).collect()
        };
        assert_eq!(
            order(&[effect("bad_omen", 120000), effect("trial_omen", 40000)]),
            ["trial_omen", "bad_omen"],
            "both over 32147 ticks: the higher colour first"
        );
        assert_eq!(
            order(&[
                effect("speed", 100),
                effect("haste", 600),
                effect("luck", INFINITE)
            ]),
            ["luck", "haste", "speed"],
            "unending first, then the most time left"
        );
        let mut ambient = effect("conduit_power", 260);
        ambient.ambient = true;
        assert_eq!(
            order(&[ambient, effect("speed", 100)]),
            ["conduit_power", "speed"]
        );
    }

    #[test]
    fn an_ambient_effect_gets_its_frame_and_never_blinks() {
        let mut ambient = effect("conduit_power", 40);
        ambient.ambient = true;
        let placed = layout(&[ambient], 200);
        assert_eq!(placed[0].background, BACKGROUND_AMBIENT);
        assert_eq!(placed[0].alpha, 1.0);
    }

    #[test]
    fn the_icon_blinks_in_the_last_ten_seconds_and_fades_out() {
        assert_eq!(alpha(&effect("speed", 201)), 1.0);
        assert_eq!(alpha(&effect("speed", INFINITE)), 1.0);
        let at = |ticks: i32| alpha(&effect("speed", ticks));
        assert!((at(200) - 0.5).abs() < 1e-6, "{}", at(200));
        assert!((at(195) - (0.5 - 0.025)).abs() < 1e-4, "{}", at(195));
        assert!((at(100) - (0.5 + 0.125)).abs() < 1e-4, "{}", at(100));
        assert!((at(105) - (0.5 - 0.125)).abs() < 1e-4, "{}", at(105));
        assert!((at(10) - (0.1 + 0.25)).abs() < 1e-4, "{}", at(10));
        assert_eq!(at(5), 0.0, "clamped at nothing");
        let changes = (1..200)
            .filter(|t| (at(*t) > 0.4) != (at(t + 5) > 0.4))
            .count();
        assert!(changes > 10, "it pulses: {changes}");
        assert_eq!(white(1.0), 0xFFFF_FFFF);
        assert_eq!(white(0.5), 0x7FFF_FFFF);
    }
}
