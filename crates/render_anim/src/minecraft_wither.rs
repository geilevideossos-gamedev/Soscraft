use std::collections::HashMap;

use frame::MinecraftUi;
use frame::minecraft_ui::McBossBar;
use minecraft_terrain::wither_bar::{COLOR, NAME, OVERLAY, WitherBar};

pub(crate) fn show_wither_bars(
    ui: &mut MinecraftUi,
    bars: &[WitherBar],
    language: &HashMap<String, String>,
    seconds: f32,
) {
    let name = language
        .get(NAME)
        .cloned()
        .unwrap_or_else(|| NAME.to_owned());
    let keys: Vec<u64> = bars
        .iter()
        .map(|bar| minecraft_survival::session::wither_bar_id(bar.id))
        .collect();
    ui.wither_bars.retain(|shown| keys.contains(&shown.key));
    for (bar, key) in bars.iter().zip(keys) {
        let shown = match ui.wither_bars.iter().position(|shown| shown.key == key) {
            Some(index) => &mut ui.wither_bars[index],
            None => {
                let mut fresh =
                    McBossBar::new(name.clone(), bar.progress, false, false).styled(COLOR, OVERLAY);
                fresh.key = key;
                ui.wither_bars.push(fresh);
                ui.wither_bars.last_mut().expect("just pushed")
            }
        };
        shown.name.clone_from(&name);
        shown.set_progress(bar.progress);
        shown.advance(seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_sees_a_purple_bar_per_wither_it_tracks() {
        let mut ui = MinecraftUi::default();
        let language: HashMap<String, String> = [(NAME.to_owned(), "Wither".to_owned())]
            .into_iter()
            .collect();
        let bar = |id, progress| WitherBar { id, progress };
        show_wither_bars(&mut ui, &[bar(7, 0.0), bar(9, 1.0)], &language, 0.05);
        assert_eq!(ui.wither_bars.len(), 2);
        let first = &ui.wither_bars[0];
        assert_eq!(
            (first.name.as_str(), first.color, first.overlay),
            ("Wither", "purple", "progress")
        );
        assert!(!first.world_fog && !first.music);
        assert_eq!(first.key, minecraft_survival::session::wither_bar_id(7));
        show_wither_bars(&mut ui, &[bar(7, 0.5)], &language, 1.0);
        assert_eq!(
            ui.wither_bars.len(),
            1,
            "a wither out of range drops its bar"
        );
        assert!((ui.wither_bars[0].progress() - 0.5).abs() < 1e-6);
        show_wither_bars(&mut ui, &[], &language, 0.05);
        assert!(ui.wither_bars.is_empty());
    }
}
