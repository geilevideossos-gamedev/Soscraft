use std::cmp::Ordering;

use frame::minecraft_ui::McEffect;
use frame::{McGui, McScreen, MinecraftUi};

use crate::minecraft_gui::{Painter, TextLine, WHITE};

const BACKGROUND: &str = "hud/effect_background";
const BACKGROUND_AMBIENT: &str = "hud/effect_background_ambient";
const LIST_BACKGROUND: &str = "container/inventory/effect_background";
const LIST_BACKGROUND_AMBIENT: &str = "container/inventory/effect_background_ambient";
const LIST_SIZE: i32 = 32;
const LIST_STEP: i32 = 33;
const LIST_SPREAD: i32 = 132;
const LIST_UNPACKED: usize = 5;
const LIST_WIDE: i32 = 120;
const LIST_GAP: i32 = 2;
const LIST_PAD: i32 = 7;
const LIST_LINE: i32 = 9;
const LIST_TIME: u32 = 0xFF80_8080;
const ELLIPSIS: &str = "...";
const TICKS_PER_SECOND: i32 = 20;
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Listed {
    pub(crate) sprite: String,
    pub(crate) background: &'static str,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) name: String,
    pub(crate) time: String,
    pub(crate) label: Option<String>,
    pub(crate) hover: Option<[i32; 4]>,
}

pub(crate) fn list_spot(ui: &MinecraftUi, gui: [i32; 2]) -> Option<[i32; 3]> {
    if !ui.inventory_open {
        return None;
    }
    let layout = match ui.screen {
        McScreen::Inventory => crate::minecraft_screens::layout(McScreen::Inventory),
        McScreen::Creative(kind) => {
            crate::minecraft_creative::layout(kind, ui.creative.first_shown())
        }
        _ => return None,
    };
    let [left, top] = crate::minecraft_recipe_book::origin(&layout, gui, ui.recipe_book.open);
    Some([left, layout.size[0], top])
}

fn list_x([left, image_width, _]: [i32; 3]) -> i32 {
    left + image_width + LIST_GAP
}

pub(crate) fn hides_icons(ui: &MinecraftUi, gui: [i32; 2]) -> bool {
    list_spot(ui, gui).is_some_and(|spot| gui[0] - list_x(spot) >= LIST_SIZE)
}

fn name(gui: &McGui, effect: &McEffect) -> String {
    let (namespace, id) = effect
        .id
        .split_once(':')
        .unwrap_or(("minecraft", &effect.id));
    let mut name = gui.text(&format!("effect.{namespace}.{id}"));
    if (1..=9).contains(&effect.amplifier) {
        name.push(' ');
        name.push_str(&gui.text(&format!("enchantment.level.{}", effect.amplifier + 1)));
    }
    name
}

fn time(gui: &McGui, ticks: i32) -> String {
    if ticks == INFINITE {
        return gui.text("effect.duration.infinite");
    }
    let seconds = ticks.div_euclid(TICKS_PER_SECOND);
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub(crate) fn listed(
    gui: &McGui,
    effects: &[McEffect],
    spot: [i32; 3],
    width: i32,
) -> Option<Vec<Listed>> {
    let x = list_x(spot);
    let room = width - x;
    if effects.is_empty() || room < LIST_SIZE {
        return None;
    }
    let max_width = if room >= LIST_WIDE {
        room - LIST_PAD
    } else {
        LIST_SIZE
    };
    let step = if effects.len() > LIST_UNPACKED {
        LIST_SPREAD / (effects.len() as i32 - 1)
    } else {
        LIST_STEP
    };
    let font = &gui.font;
    let mut sorted: Vec<&McEffect> = effects.iter().collect();
    sorted.sort_by(|a, b| compare(a, b));
    let listed = sorted
        .into_iter()
        .enumerate()
        .map(|(index, effect)| {
            let y = spot[2] + step * index as i32;
            let name = name(gui, effect);
            let time = time(gui, effect.duration);
            let fit = |text: &str| LIST_SIZE + font.width(text) + LIST_PAD;
            let box_width = max_width.min(fit(&name).max(fit(&time)));
            let text_room = box_width - LIST_SIZE - LIST_PAD;
            let (label, clipped) = if text_room <= 0 {
                (None, true)
            } else if font.width(&name) > text_room {
                let head = font.head_by_width(&name, text_room - font.width(ELLIPSIS));
                (Some(format!("{head}{ELLIPSIS}")), true)
            } else {
                (Some(name.clone()), false)
            };
            Listed {
                sprite: format!("mob_effect/{}", path(&effect.id)),
                background: if effect.ambient {
                    LIST_BACKGROUND_AMBIENT
                } else {
                    LIST_BACKGROUND
                },
                x,
                y,
                width: box_width,
                name,
                time,
                label,
                hover: clipped.then_some([x, y, x + box_width, y + step]),
            }
        })
        .collect();
    Some(listed)
}

pub(crate) fn draw_listed(p: &mut Painter<'_>, listed: &[Listed]) {
    for entry in listed {
        p.sprite(entry.background, entry.x, entry.y, entry.width, LIST_SIZE);
        if let Some(label) = &entry.label {
            let text_x = entry.x + LIST_SIZE;
            let text_y = entry.y + LIST_PAD;
            p.text(label, text_x, text_y, WHITE, true);
            p.text(&entry.time, text_x, text_y + LIST_LINE, LIST_TIME, true);
        }
        p.sprite(
            &entry.sprite,
            entry.x + LIST_PAD,
            entry.y + LIST_PAD,
            ICON,
            ICON,
        );
    }
}

pub(crate) fn listed_tooltip(listed: &[Listed], mouse: [i32; 2]) -> Option<Vec<TextLine>> {
    let [mx, my] = mouse;
    listed
        .iter()
        .find(|entry| {
            entry
                .hover
                .is_some_and(|[x0, y0, x1, y1]| mx >= x0 && mx <= x1 && my >= y0 && my <= y1)
        })
        .map(|entry| {
            vec![
                TextLine::new(entry.name.clone(), WHITE),
                TextLine::new(entry.time.clone(), WHITE),
            ]
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame::McTabKind;

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

    fn texts() -> McGui {
        let mut gui = McGui::default();
        for (key, text) in [
            ("effect.minecraft.speed", "Speed"),
            ("effect.minecraft.haste", "Haste"),
            ("effect.minecraft.strength", "Strength"),
            ("effect.minecraft.luck", "Luck"),
            ("effect.minecraft.absorption", "Absorption"),
            ("effect.minecraft.regeneration", "Regeneration"),
            ("effect.minecraft.night_vision", "Night Vision"),
            (
                "effect.minecraft.hero_of_the_village",
                "Hero of the Village",
            ),
            ("effect.minecraft.bad_omen", "Bad Omen"),
            ("effect.duration.infinite", "\u{221e}"),
            ("enchantment.level.2", "II"),
            ("enchantment.level.5", "V"),
        ] {
            gui.texts.insert(key.into(), text.into());
        }
        gui
    }

    fn leveled(id: &str, amplifier: u8, duration: i32) -> McEffect {
        McEffect {
            amplifier,
            ..effect(id, duration)
        }
    }

    fn screen(screen: McScreen, book: bool) -> MinecraftUi {
        let mut ui = MinecraftUi {
            inventory_open: true,
            screen,
            ..MinecraftUi::default()
        };
        ui.recipe_book.open = book;
        ui
    }

    fn boxes(listed: &[Listed]) -> Vec<(&str, i32, i32, i32, Option<&str>)> {
        listed
            .iter()
            .map(|l| (l.sprite.as_str(), l.x, l.y, l.width, l.label.as_deref()))
            .collect()
    }

    #[test]
    fn the_list_sits_right_of_the_inventory_in_natural_order_33_apart() {
        let gui = texts();
        let ui = screen(McScreen::Inventory, false);
        let spot = list_spot(&ui, [480, 252]).unwrap();
        assert_eq!(spot, [152, 176, 43]);
        let effects = [
            effect("hero_of_the_village", 12000),
            leveled("regeneration", 1, 300),
            effect("night_vision", 9600),
            effect("speed", 100),
        ];
        let listed = listed(&gui, &effects, spot, 480).unwrap();
        assert_eq!(
            boxes(&listed),
            vec![
                ("mob_effect/speed", 330, 43, 69, Some("Speed")),
                (
                    "mob_effect/regeneration",
                    330,
                    76,
                    129,
                    Some("Regeneration II")
                ),
                (
                    "mob_effect/night_vision",
                    330,
                    109,
                    111,
                    Some("Night Vision")
                ),
                (
                    "mob_effect/hero_of_the_village",
                    330,
                    142,
                    143,
                    Some("Hero of the Vi...")
                ),
            ]
        );
        assert_eq!(listed[0].time, "00:05");
        assert!(listed.iter().all(|l| l.background == LIST_BACKGROUND));
        assert_eq!(
            listed.iter().map(|l| l.hover).collect::<Vec<_>>(),
            [None, None, None, Some([330, 142, 473, 175])]
        );
        let mut ambient = effect("speed", 100);
        ambient.ambient = true;
        let listed = super::listed(&gui, &[ambient], spot, 480).unwrap();
        assert_eq!(listed[0].background, LIST_BACKGROUND_AMBIENT);
        assert_eq!(super::listed(&gui, &[], spot, 480), None);
    }

    #[test]
    fn more_than_five_effects_pack_into_132_pixels() {
        let gui = texts();
        let spot = [152, 176, 43];
        let ids = [
            "speed",
            "haste",
            "strength",
            "luck",
            "absorption",
            "regeneration",
            "night_vision",
        ];
        let effects: Vec<McEffect> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| effect(id, 200 * (i as i32 + 1)))
            .collect();
        let ys = |count: usize| -> Vec<i32> {
            listed(&gui, &effects[..count], spot, 480)
                .unwrap()
                .iter()
                .map(|l| l.y)
                .collect()
        };
        assert_eq!(ys(7), [43, 65, 87, 109, 131, 153, 175]);
        assert_eq!(ys(6), [43, 69, 95, 121, 147, 173]);
        assert_eq!(ys(5), [43, 76, 109, 142, 175]);
    }

    #[test]
    fn a_narrow_room_gives_icon_boxes_with_a_tooltip_and_too_little_gives_nothing() {
        let gui = texts();
        let ui = screen(McScreen::Inventory, true);
        let spot = list_spot(&ui, [480, 252]).unwrap();
        assert_eq!(spot, [229, 176, 43]);
        let effects = [effect("speed", 100), effect("luck", INFINITE)];
        let listed = listed(&gui, &effects, spot, 480).unwrap();
        assert_eq!(
            boxes(&listed),
            vec![
                ("mob_effect/speed", 407, 43, 32, None),
                ("mob_effect/luck", 407, 76, 32, None),
            ]
        );
        let lines = |mouse: [i32; 2]| {
            listed_tooltip(&listed, mouse).map(|lines| {
                lines
                    .into_iter()
                    .map(|line| (line.text, line.color))
                    .collect::<Vec<_>>()
            })
        };
        let speed = Some(vec![
            ("Speed".to_owned(), WHITE),
            ("00:05".to_owned(), WHITE),
        ]);
        assert_eq!(lines([417, 48]), speed);
        assert_eq!(lines([439, 76]), speed, "the upper box wins its last row");
        assert_eq!(
            lines([407, 100]),
            Some(vec![
                ("Luck".to_owned(), WHITE),
                ("\u{221e}".to_owned(), WHITE)
            ])
        );
        assert_eq!(lines([440, 50]), None);
        assert_eq!(lines([406, 50]), None);
        let tight = list_spot(&ui, [396, 252]).unwrap();
        assert_eq!(396 - list_x(tight), 31);
        assert_eq!(super::listed(&gui, &effects, tight, 396), None);
        let snug = list_spot(&ui, [398, 252]).unwrap();
        assert_eq!(398 - list_x(snug), 32);
        assert_eq!(
            super::listed(&gui, &effects, snug, 398).unwrap()[0].width,
            32
        );
    }

    #[test]
    fn names_carry_levels_two_to_ten_and_times_count_twenty_ticks_a_second() {
        let gui = texts();
        assert_eq!(
            name(&gui, &leveled("regeneration", 1, 100)),
            "Regeneration II"
        );
        assert_eq!(name(&gui, &leveled("bad_omen", 4, 100)), "Bad Omen V");
        assert_eq!(name(&gui, &leveled("bad_omen", 0, 100)), "Bad Omen");
        assert_eq!(name(&gui, &leveled("bad_omen", 10, 100)), "Bad Omen");
        assert_eq!(
            name(&gui, &leveled("bad_omen", 9, 100)),
            "Bad Omen enchantment.level.10"
        );
        assert_eq!(time(&gui, 100), "00:05");
        assert_eq!(time(&gui, 119), "00:05");
        assert_eq!(time(&gui, 9600), "08:00");
        assert_eq!(time(&gui, 71999), "59:59");
        assert_eq!(time(&gui, 120000), "01:40:00");
        assert_eq!(time(&gui, INFINITE), "\u{221e}");
    }

    #[test]
    fn a_long_name_is_cut_to_fit_with_three_dots() {
        let gui = texts();
        let listed = listed(
            &gui,
            &[effect("hero_of_the_village", 400)],
            [152, 176, 43],
            480,
        )
        .unwrap();
        let entry = &listed[0];
        let room = entry.width - LIST_SIZE - LIST_PAD;
        let label = entry.label.as_deref().unwrap();
        assert_eq!(label, "Hero of the Vi...");
        assert!(gui.font.width(label) <= room);
        assert_eq!(entry.name, "Hero of the Village");
        assert!(listed_tooltip(&listed, [400, 60]).is_some());
    }

    #[test]
    fn the_hud_icons_hide_exactly_when_the_list_has_room() {
        let gui = texts();
        let effects = [effect("speed", 100)];
        let cases = [
            (screen(McScreen::Inventory, false), 480, true),
            (screen(McScreen::Inventory, true), 480, true),
            (screen(McScreen::Inventory, true), 396, false),
            (screen(McScreen::Inventory, true), 398, true),
            (screen(McScreen::Inventory, true), 378, true),
            (
                screen(McScreen::Creative(McTabKind::Inventory), false),
                480,
                true,
            ),
            (
                screen(McScreen::Creative(McTabKind::Search), false),
                320,
                true,
            ),
            (screen(McScreen::Chest { rows: 3 }, false), 480, false),
            (screen(McScreen::CraftingTable, false), 480, false),
            (
                MinecraftUi {
                    inventory_open: false,
                    ..MinecraftUi::default()
                },
                480,
                false,
            ),
        ];
        for (index, (ui, width, hides)) in cases.into_iter().enumerate() {
            let gui_size = [width, 252];
            assert_eq!(hides_icons(&ui, gui_size), hides, "case {index}");
            let shown = list_spot(&ui, gui_size)
                .and_then(|spot| listed(&gui, &effects, spot, width))
                .is_some();
            assert_eq!(shown, hides, "case {index}");
        }
        let creative = screen(McScreen::Creative(McTabKind::Inventory), false);
        assert_eq!(list_spot(&creative, [480, 252]), Some([142, 195, 58]));
    }

    #[test]
    fn a_listed_effect_draws_its_frame_text_and_icon_in_vanilla_order() {
        let mut gui = texts();
        gui.size = [512, 512];
        let glyph = frame::McGlyph {
            rect: [0, 0, 5, 8],
            scale: 1.0,
            ascent: 7,
            advance: 6.0,
        };
        for c in ' '..='~' {
            gui.font.glyphs.insert(c, glyph);
        }
        for name in [LIST_BACKGROUND, "mob_effect/speed"] {
            gui.sprites.insert(
                name.into(),
                frame::minecraft_gui::McSprite {
                    rect: [0, 0, 32, 32],
                    scaling: frame::minecraft_gui::McScaling::Stretch,
                },
            );
        }
        let listed = listed(&gui, &[effect("speed", 100)], [152, 176, 43], 480).unwrap();
        let space = crate::minecraft_gui::GuiSpace::new([480, 252], [480.0, 252.0], [480.0, 252.0]);
        let mut painter = Painter::new(&gui, space);
        draw_listed(&mut painter, &listed);
        let quads = &painter.quads;
        assert_eq!(quads.len(), 2 + 2 * 5 + 2 * 5);
        assert_eq!(
            (quads[0].xy[0], quads[0].xy[2]),
            ([330.5, 43.5], [399.5, 75.5])
        );
        let last = quads.last().unwrap();
        assert_eq!((last.xy[0], last.xy[2]), ([337.5, 50.5], [355.5, 68.5]));
        let at = |color: u32, xy: [f32; 2]| {
            quads
                .iter()
                .any(|q| q.color == crate::minecraft_gui::argb(color) && q.xy[0] == xy)
        };
        assert!(at(WHITE, [362.5, 50.5]), "the name");
        assert!(at(LIST_TIME, [362.5, 59.5]), "the time");
        assert!(
            at(crate::minecraft_gui::shadow_color(WHITE), [363.5, 51.5]),
            "the shadow"
        );
        let mut compact = listed[0].clone();
        compact.label = None;
        let mut painter = Painter::new(&gui, space);
        draw_listed(&mut painter, &[compact]);
        assert_eq!(painter.quads.len(), 2);
    }
}
