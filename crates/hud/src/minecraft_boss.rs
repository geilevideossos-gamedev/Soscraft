use frame::MinecraftUi;

use crate::minecraft_gui::{Painter, WHITE};

pub(crate) const BAR_WIDTH: i32 = 182;
pub(crate) const BAR_HEIGHT: i32 = 5;

pub(crate) fn lerp_discrete(delta: f32, start: i32, end: i32) -> i32 {
    let range = end - start;
    start + (delta * (range - 1) as f32).floor() as i32 + i32::from(delta > 0.0)
}

pub(crate) fn rows(count: usize, gui_height: i32) -> Vec<i32> {
    let mut rows = Vec::new();
    let mut y = 12;
    for _ in 0..count {
        rows.push(y);
        y += 10 + 9;
        if y >= gui_height / 3 {
            break;
        }
    }
    rows
}

pub(crate) fn sprites(bar: &frame::minecraft_ui::McBossBar) -> [Option<String>; 4] {
    let color = if bar.color.is_empty() { "pink" } else { bar.color };
    let notched = !bar.overlay.is_empty() && bar.overlay != "progress";
    [
        Some(format!("boss_bar/{color}_background")),
        Some(format!("boss_bar/{color}_progress")),
        notched.then(|| format!("boss_bar/{}_background", bar.overlay)),
        notched.then(|| format!("boss_bar/{}_progress", bar.overlay)),
    ]
}

pub(crate) fn draw_boss_bars(p: &mut Painter<'_>, ui: &MinecraftUi) {
    let width = p.space.width;
    let bars: Vec<&frame::minecraft_ui::McBossBar> = ui.boss_bars.iter().chain(ui.raid_bars.iter()).collect();
    let rows = rows(bars.len(), p.space.height);
    for (bar, y) in bars.into_iter().zip(rows) {
        let x = width / 2 - 91;
        let [background, progress, notches, notch_progress] = sprites(bar);
        for sprite in [&background, &notches].into_iter().flatten() {
            p.sprite(sprite, x, y, BAR_WIDTH, BAR_HEIGHT);
        }
        let fill = lerp_discrete(bar.progress(), 0, BAR_WIDTH);
        if fill > 0 {
            for sprite in [&progress, &notch_progress].into_iter().flatten() {
                p.sprite_part(
                    sprite, BAR_WIDTH, BAR_HEIGHT, 0, 0, x, y, fill, BAR_HEIGHT,
                );
            }
        }
        let name_width = p.width(&bar.name);
        p.text(&bar.name, width / 2 - name_width / 2, y - 9, WHITE, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_fills_182_pixels_in_steps_like_lerp_discrete() {
        assert_eq!(lerp_discrete(1.0, 0, BAR_WIDTH), 182);
        assert_eq!(lerp_discrete(0.5, 0, BAR_WIDTH), 91);
        assert_eq!(lerp_discrete(0.001, 0, BAR_WIDTH), 1);
        assert_eq!(lerp_discrete(0.0, 0, BAR_WIDTH), 0);
    }

    #[test]
    fn boss_bars_stack_down_from_the_top_until_a_third_of_the_screen() {
        assert_eq!(rows(1, 240), vec![12]);
        assert_eq!(rows(9, 240), vec![12, 31, 50, 69]);
        assert!(rows(0, 240).is_empty());
    }

    #[test]
    fn a_bar_draws_its_colour_and_its_notches_over_it() {
        let dragon = frame::minecraft_ui::McBossBar::new("Ender Dragon".to_owned(), 1.0, true, true);
        assert_eq!(
            sprites(&dragon),
            [Some("boss_bar/pink_background".to_owned()), Some("boss_bar/pink_progress".to_owned()), None, None]
        );
        let raid = frame::minecraft_ui::McBossBar::new("Raid".to_owned(), 0.5, false, false).styled("red", "notched_10");
        assert_eq!(
            sprites(&raid),
            [
                Some("boss_bar/red_background".to_owned()),
                Some("boss_bar/red_progress".to_owned()),
                Some("boss_bar/notched_10_background".to_owned()),
                Some("boss_bar/notched_10_progress".to_owned())
            ]
        );
        assert_eq!(sprites(&frame::minecraft_ui::McBossBar::default())[0].as_deref(), Some("boss_bar/pink_background"));
    }
}
