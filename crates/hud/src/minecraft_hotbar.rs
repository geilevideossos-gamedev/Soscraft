use assets::PreparedWeapons;
use frame::{McStack, MinecraftUi};

use crate::minecraft_gui::{AQUA, Painter, WHITE};
use crate::minecraft_vitals::{bubbles, drumsticks, heart_rows, hearts};

const XP_TEXT: u32 = 0xFF80_FF20;
const OUTLINE: u32 = 0xFF00_0000;
const NAME_TICKS: i32 = 40;
const TICK: f64 = 0.05;

pub(crate) fn sprite_name(key: &str) -> &str {
    key.strip_prefix("gui:").unwrap_or(key)
}

pub(crate) fn name_alpha(timer: i32) -> u32 {
    ((timer as f32 * 256.0 / 10.0) as i32).clamp(0, 255) as u32
}

#[derive(Default)]
pub(crate) struct Highlight {
    last: Option<(String, String)>,
    timer: i32,
    clock: f64,
}

impl Highlight {
    pub(crate) fn advance(&mut self, dt: f64, current: Option<(String, String)>) {
        self.clock = (self.clock + dt).min(1.0);
        while self.clock >= TICK {
            self.clock -= TICK;
            self.tick(current.clone());
        }
    }

    pub(crate) fn tick(&mut self, current: Option<(String, String)>) {
        match &current {
            None => self.timer = 0,
            Some(item) if self.last.as_ref() != Some(item) => self.timer = NAME_TICKS,
            Some(_) => self.timer = (self.timer - 1).max(0),
        }
        self.last = current;
    }

    pub(crate) fn shown(&self) -> Option<(&str, u32)> {
        let (_, name) = self.last.as_ref()?;
        let alpha = name_alpha(self.timer);
        (alpha > 0 && !name.is_empty()).then_some((name.as_str(), alpha))
    }
}

pub(crate) fn name_color(stack: Option<&McStack>) -> u32 {
    if stack.is_some_and(|stack| !stack.enchantments.is_empty()) {
        AQUA
    } else {
        WHITE
    }
}

pub(crate) fn xp_fill(progress: f32) -> i32 {
    (progress * 183.0) as i32
}

pub(crate) fn armor_sprites(armor: u8) -> [&'static str; 10] {
    std::array::from_fn(|i| {
        let point = i as i32 * 2 + 1;
        let armor = i32::from(armor);
        if point < armor {
            "hud/armor_full"
        } else if point == armor {
            "hud/armor_half"
        } else {
            "hud/armor_empty"
        }
    })
}

pub(crate) fn draw_hud(
    p: &mut Painter<'_>,
    ui: &MinecraftUi,
    weapons: Option<&PreparedWeapons>,
    name: Option<(&str, u32)>,
) {
    crate::minecraft_boss::draw_boss_bars(p, ui);
    if !ui.game_mode.shows_hotbar() {
        return;
    }
    let cx = p.space.width / 2;
    let h = p.space.height;
    p.sprite("hud/hotbar", cx - 91, h - 22, 182, 22);
    p.sprite(
        "hud/hotbar_selection",
        cx - 91 - 1 + ui.selected.min(8) as i32 * 20,
        h - 22 - 1,
        24,
        23,
    );
    let offhand = ui.slots.get(40).and_then(Option::as_ref);
    if offhand.is_some() {
        p.sprite("hud/hotbar_offhand_left", cx - 91 - 29, h - 23, 29, 24);
    }
    let mut items: Vec<(&McStack, i32, i32)> = (0..9)
        .filter_map(|i| {
            ui.slots
                .get(i)
                .and_then(Option::as_ref)
                .map(|stack| (stack, cx - 90 + i as i32 * 20 + 2, h - 16 - 3))
        })
        .collect();
    if let Some(stack) = offhand {
        items.push((stack, cx - 91 - 26, h - 16 - 3));
    }
    for &(stack, x, y) in &items {
        p.item(ui, weapons, stack, x, y);
    }
    for &(stack, x, y) in &items {
        p.decorations(ui, stack, [x, y], None);
    }
    if let Some(vitals) = ui.vitals.as_ref().filter(|_| ui.game_mode.shows_vitals()) {
        let left = cx - 91;
        let right = cx + 91;
        let line = h - 39;
        let (rows, row_height) = heart_rows(vitals);
        let row_height = row_height as i32;
        if ui.armor > 0 {
            let y = line - (rows as i32 - 1) * row_height - 10;
            for (i, sprite) in armor_sprites(ui.armor).into_iter().enumerate() {
                p.sprite(sprite, left + i as i32 * 8, y, 9, 9);
            }
        }
        for heart in hearts(vitals) {
            let x = left + heart.column as i32 * 8;
            let y = line - heart.row as i32 * row_height - heart.lift as i32;
            for layer in &heart.layers {
                p.sprite(sprite_name(layer), x, y, 9, 9);
            }
        }
        for (j, drumstick) in drumsticks(vitals).iter().enumerate() {
            let x = right - j as i32 * 8 - 9;
            let y = line + drumstick.shake as i32;
            p.sprite(sprite_name(drumstick.empty), x, y, 9, 9);
            if let Some(fill) = drumstick.fill {
                p.sprite(sprite_name(fill), x, y, 9, 9);
            }
        }
        if let Some(row) = bubbles(vitals.air, vitals.max_air, vitals.eye_in_water) {
            for (i, bubble) in row.iter().enumerate() {
                if let Some(sprite) = bubble.sprite() {
                    p.sprite(
                        sprite_name(sprite),
                        right - i as i32 * 8 - 9,
                        line - 10,
                        9,
                        9,
                    );
                }
            }
        }
        let bar_x = (p.space.width - 182) / 2;
        let bar_y = h - 24 - 5;
        p.sprite("hud/experience_bar_background", bar_x, bar_y, 182, 5);
        let fill = xp_fill(vitals.experience_progress.clamp(0.0, 1.0));
        if fill > 0 {
            p.sprite_part(
                "hud/experience_bar_progress",
                182,
                5,
                0,
                0,
                bar_x,
                bar_y,
                fill,
                5,
            );
        }
        if vitals.experience_level > 0 {
            let text = p.gui.format(
                "gui.experience.level",
                &[&vitals.experience_level.to_string()],
            );
            let x = (p.space.width - p.width(&text)) / 2;
            let y = h - 24 - 9 - 2;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                p.text(&text, x + dx, y + dy, OUTLINE, false);
            }
            p.text(&text, x, y, XP_TEXT, false);
        }
    }
    if let Some((name, alpha)) = name {
        let selected = ui.slots.get(ui.selected).and_then(Option::as_ref);
        let color = (name_color(selected) & 0x00ff_ffff) | (alpha << 24);
        let italic = selected.is_some_and(|stack| stack.named);
        let x = (p.space.width - p.width(name)) / 2;
        let y = if ui.game_mode.shows_vitals() { h - 59 } else { h - 45 };
        p.styled_text(false, name, x, y, color, true, italic);
    }
    if let Some(message) = ui.overlay_message.as_ref().filter(|m| m.alpha() > 8) {
        let x = (p.space.width - p.width(&message.text)) / 2;
        p.text(&message.text, x, h - 72, (message.alpha() << 24) | 0x00ff_ffff, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_item_name_holds_for_thirty_ticks_then_fades_over_ten() {
        assert_eq!(name_alpha(40), 255);
        assert_eq!(name_alpha(10), 255);
        assert_eq!(name_alpha(9), 230);
        assert_eq!(name_alpha(1), 25);
        assert_eq!(name_alpha(0), 0);
        let mut highlight = Highlight::default();
        let dirt = || Some(("minecraft:dirt".to_owned(), "Dirt".to_owned()));
        highlight.advance(0.06, dirt());
        assert_eq!(highlight.shown(), Some(("Dirt", 255)));
        for _ in 0..30 {
            highlight.tick(dirt());
        }
        assert_eq!(highlight.shown(), Some(("Dirt", 255)));
        highlight.tick(dirt());
        assert_eq!(highlight.shown(), Some(("Dirt", 230)));
        for _ in 0..9 {
            highlight.tick(dirt());
        }
        assert_eq!(highlight.shown(), None);
    }

    #[test]
    fn the_item_name_follows_the_selected_stack_like_vanilla() {
        let mut highlight = Highlight::default();
        let stack = |id: &str, name: &str| Some((id.to_owned(), name.to_owned()));
        highlight.tick(stack("minecraft:dirt", "Dirt"));
        for _ in 0..5 {
            highlight.tick(stack("minecraft:dirt", "Dirt"));
        }
        assert_eq!(highlight.timer, 35);
        highlight.tick(stack("minecraft:dirt", "Dirt"));
        assert_eq!(highlight.timer, 34);
        highlight.tick(stack("minecraft:dirt", "Garden"));
        assert_eq!(highlight.timer, 40);
        highlight.tick(None);
        assert_eq!(highlight.shown(), None);
        highlight.tick(stack("minecraft:stone", "Stone"));
        assert_eq!(highlight.shown(), Some(("Stone", 255)));
    }

    #[test]
    fn the_overlay_message_holds_for_two_seconds_then_fades_over_one() {
        let at = |ticks| {
            frame::minecraft_ui::McOverlayMessage {
                text: "Hello".into(),
                ticks,
            }
            .alpha()
        };
        assert_eq!(at(frame::minecraft_ui::McOverlayMessage::TICKS), 255);
        assert_eq!(at(20.0), 255);
        assert_eq!(at(10.0), 127);
        assert_eq!(at(0.5), 6);
        assert_eq!(at(-1.0), 0);
    }

    #[test]
    fn armor_points_fill_icons_in_halves() {
        let row = armor_sprites(15);
        assert_eq!(row[6], "hud/armor_full");
        assert_eq!(row[7], "hud/armor_half");
        assert_eq!(row[8], "hud/armor_empty");
        assert!(armor_sprites(20).iter().all(|s| *s == "hud/armor_full"));
    }

    #[test]
    fn the_experience_bar_is_183_wide_at_full() {
        assert_eq!(xp_fill(0.0), 0);
        assert_eq!(xp_fill(0.5), 91);
        assert_eq!(xp_fill(1.0), 183);
        assert_eq!(sprite_name("gui:hud/air"), "hud/air");
    }
}
