use frame::MinecraftUi;
use frame::minecraft_ui::McBossBar;
use minecraft_terrain::elden_bar::{COLOR, EldenBar, OVERLAY};

pub(crate) fn show_elden_bars(ui: &mut MinecraftUi, bars: &[EldenBar], seconds: f32) {
    let keys: Vec<u64> = bars
        .iter()
        .map(|bar| minecraft_survival::session::elden_bar_id(bar.id))
        .collect();
    ui.elden_bars.retain(|shown| keys.contains(&shown.key));
    for (bar, key) in bars.iter().zip(keys) {
        let shown = match ui.elden_bars.iter().position(|shown| shown.key == key) {
            Some(index) => &mut ui.elden_bars[index],
            None => {
                let mut fresh = McBossBar::new(bar.name.to_owned(), bar.progress, false, true)
                    .styled(COLOR, OVERLAY);
                fresh.key = key;
                ui.elden_bars.push(fresh);
                ui.elden_bars.last_mut().expect("just pushed")
            }
        };
        shown.set_progress(bar.progress);
        shown.advance(seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_sees_a_red_bar_named_margit_that_asks_for_music() {
        let mut ui = MinecraftUi::default();
        let bar = |progress| EldenBar {
            id: 7,
            name: "Margit, the Fell Omen",
            progress,
        };
        show_elden_bars(&mut ui, &[bar(1.0)], 0.05);
        assert_eq!(ui.elden_bars.len(), 1);
        let shown = &ui.elden_bars[0];
        assert_eq!(
            (shown.name.as_str(), shown.color, shown.overlay),
            ("Margit, the Fell Omen", "red", "progress")
        );
        assert!(shown.music && !shown.world_fog);
        assert_eq!(shown.key, minecraft_survival::session::elden_bar_id(7));
        show_elden_bars(&mut ui, &[bar(0.5)], 1.0);
        assert_eq!(ui.elden_bars.len(), 1);
        assert!((ui.elden_bars[0].progress() - 0.5).abs() < 1e-6);
        show_elden_bars(&mut ui, &[], 0.05);
        assert!(ui.elden_bars.is_empty(), "he reset, died or is too far");
    }
}
