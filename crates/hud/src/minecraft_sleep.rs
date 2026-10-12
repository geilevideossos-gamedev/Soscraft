use frame::minecraft_ui::McSleep;

use crate::minecraft_gui::Painter;

pub(crate) const SLEEP_COLOR: u32 = 0x10_1020;
pub(crate) const BUTTON: &str = "widget/button";
pub(crate) const BUTTON_HIGHLIGHTED: &str = "widget/button_highlighted";
pub(crate) const LEAVE_BED: &str = "multiplayer.stopSleeping";
const BUTTON_WIDTH: i32 = 200;
const BUTTON_HEIGHT: i32 = 20;
const BUTTON_ABOVE_BOTTOM: i32 = 40;
const LABEL: u32 = 0xFFFF_FFFF;

pub(crate) fn overlay_color(timer: i32) -> Option<u32> {
    if timer <= 0 {
        return None;
    }
    let mut amount = timer as f32 / 100.0;
    if amount > 1.0 {
        amount = 1.0 - (timer - 100) as f32 / 10.0;
    }
    let alpha = (220.0 * amount) as i32;
    (alpha > 0).then(|| ((alpha as u32) << 24) | SLEEP_COLOR)
}

pub(crate) fn button_rect(width: i32, height: i32) -> [i32; 4] {
    [
        width / 2 - BUTTON_WIDTH / 2,
        height - BUTTON_ABOVE_BOTTOM,
        BUTTON_WIDTH,
        BUTTON_HEIGHT,
    ]
}

pub(crate) fn hovers([x, y, w, h]: [i32; 4], mouse: Option<[f64; 2]>) -> bool {
    mouse.is_some_and(|[mx, my]| {
        mx >= f64::from(x) && my >= f64::from(y) && mx < f64::from(x + w) && my < f64::from(y + h)
    })
}

pub(crate) fn draw_overlay(p: &mut Painter, sleep: Option<McSleep>) {
    if let Some(color) = sleep.and_then(|sleep| overlay_color(sleep.timer)) {
        p.fill(0, 0, p.space.width, p.space.height, color);
    }
}

pub(crate) fn draw_button(p: &mut Painter, mouse: Option<[f64; 2]>) -> bool {
    let rect = button_rect(p.space.width, p.space.height);
    let [x, y, w, h] = rect;
    let hovered = hovers(rect, mouse);
    p.sprite(
        if hovered { BUTTON_HIGHLIGHTED } else { BUTTON },
        x,
        y,
        w,
        h,
    );
    let label = p.gui.text(LEAVE_BED);
    let width = p.width(&label);
    p.text(&label, x + (w - width) / 2, y + (h - 8) / 2, LABEL, true);
    hovered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft_chat::{Mods, Typed, apply_typed};
    use crate::minecraft_gui::GuiSpace;
    use bevy::input::keyboard::{Key, KeyCode, NativeKeyCode};
    use frame::minecraft_gui::{McScaling, McSprite};
    use frame::{McChat, McGui};

    fn gui() -> McGui {
        let mut gui = McGui {
            size: [512, 512],
            white: [101, 101],
            ..McGui::default()
        };
        for (name, top) in [(BUTTON, 0), (BUTTON_HIGHLIGHTED, 20)] {
            gui.sprites.insert(
                name.into(),
                McSprite {
                    rect: [0, top, 200, 20],
                    scaling: McScaling::NineSlice {
                        width: 200,
                        height: 20,
                        border: [3; 4],
                        stretch_inner: false,
                    },
                },
            );
        }
        gui
    }

    fn space() -> GuiSpace {
        GuiSpace::new([456, 256], [456.0, 256.0], [456.0, 256.0])
    }

    fn press(chat: &mut McChat, key: Key) {
        let typed = Typed {
            key,
            code: KeyCode::Unidentified(NativeKeyCode::Unidentified),
            text: None,
            mods: Mods::default(),
        };
        apply_typed(chat, &typed, &mut ());
    }

    #[test]
    fn the_sleep_overlay_darkens_over_100_ticks_and_fades_in_10() {
        assert_eq!(overlay_color(0), None);
        assert_eq!(overlay_color(50), Some(0x6E10_1020));
        assert_eq!(overlay_color(100), Some(0xDC10_1020));
        assert_eq!(overlay_color(105), Some(0x6E10_1020));
        assert_eq!(overlay_color(110), None);
        assert_eq!(overlay_color(1).map(|c| c >> 24), Some(2));
    }

    #[test]
    fn the_leave_bed_button_sits_40_pixels_above_the_bottom() {
        assert_eq!(button_rect(456, 256), [128, 216, 200, 20]);
        assert_eq!(button_rect(320, 240), [60, 200, 200, 20]);
        let rect = button_rect(456, 256);
        assert!(hovers(rect, Some([128.0, 216.0])));
        assert!(hovers(rect, Some([327.9, 235.9])));
        assert!(!hovers(rect, Some([328.0, 220.0])));
        assert!(!hovers(rect, Some([200.0, 215.9])));
        assert!(!hovers(rect, None));
        let gui = gui();
        let mut painter = Painter::new(&gui, space());
        assert!(draw_button(&mut painter, Some([200.0, 220.0])));
        let mut painter = Painter::new(&gui, space());
        assert!(!draw_button(&mut painter, Some([10.0, 10.0])));
        assert_eq!(painter.quads[0].xy[0], [128.5, 216.5]);
    }

    #[test]
    fn escape_in_bed_asks_to_leave() {
        let mut chat = McChat::default();
        chat.lie_in_bed();
        press(&mut chat, Key::Escape);
        assert!(chat.leave_bed && chat.open && chat.in_bed);
        chat.input = "hello".into();
        chat.cursor = 5;
        chat.highlight = 5;
        press(&mut chat, Key::Enter);
        assert_eq!(chat.sent, vec!["hello".to_owned()]);
        assert!(chat.open && chat.input.is_empty() && chat.cursor == 0);
        let mut awake = McChat::default();
        awake.open = true;
        press(&mut awake, Key::Escape);
        assert!(!awake.open && !awake.leave_bed);
    }

    #[test]
    fn a_nine_slice_keeps_three_pixel_borders() {
        let gui = gui();
        let mut painter = Painter::new(&gui, space());
        painter.sprite(BUTTON, 10, 10, 120, 20);
        let strips: Vec<_> = painter.quads.iter().map(|quad| quad.xy).collect();
        assert_eq!((strips[0][0], strips[0][2]), ([10.5, 10.5], [13.5, 30.5]));
        let right = strips.iter().find(|xy| xy[0] == [127.5, 10.5]);
        assert!(right.is_some_and(|xy| xy[2] == [130.5, 30.5]));
        let mut painter = Painter::new(&gui, space());
        painter.sprite(BUTTON, 10, 10, 120, 30);
        let corners: Vec<_> = painter.quads.iter().map(|quad| quad.xy).collect();
        assert_eq!((corners[0][0], corners[0][2]), ([10.5, 10.5], [13.5, 13.5]));
        let bottom = corners.iter().find(|xy| xy[0] == [127.5, 37.5]);
        assert!(bottom.is_some_and(|xy| xy[2] == [130.5, 40.5]));
    }
}
