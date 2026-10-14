use std::collections::HashMap;

use frame::MinecraftUi;
use frame::minecraft_ui::McBossBar;
use minecraft_terrain::raids::RaidBar;

pub(crate) fn show_raid_bars(
    ui: &mut MinecraftUi,
    bars: &[RaidBar],
    language: &HashMap<String, String>,
    seconds: f32,
) {
    let lang = |key: &str| language.get(key).cloned();
    let keys: Vec<u64> = bars
        .iter()
        .map(|bar| minecraft_survival::session::raid_bar_id(bar.id))
        .collect();
    ui.raid_bars.retain(|shown| keys.contains(&shown.key));
    for (bar, key) in bars.iter().zip(keys) {
        let name = minecraft_survival::session::raid_bar_name(bar.name)
            .resolve(&lang)
            .text;
        let shown = match ui.raid_bars.iter().position(|shown| shown.key == key) {
            Some(index) => &mut ui.raid_bars[index],
            None => {
                let mut fresh = McBossBar::new(name.clone(), bar.progress, false, false)
                    .styled("red", "notched_10");
                fresh.key = key;
                ui.raid_bars.push(fresh);
                ui.raid_bars.last_mut().expect("just pushed")
            }
        };
        shown.name = name;
        shown.set_progress(bar.progress);
        shown.advance(seconds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::raids::BarName;

    fn language() -> HashMap<String, String> {
        [
            ("event.minecraft.raid", "Raid"),
            (
                "event.minecraft.raid.raiders_remaining",
                "Raiders Remaining: %s",
            ),
            ("event.minecraft.raid.victory.full", "Raid - Victory"),
        ]
        .into_iter()
        .map(|(key, text)| (key.to_owned(), text.to_owned()))
        .collect()
    }

    #[test]
    fn the_host_sees_a_red_notched_raid_bar_named_like_vanilla() {
        let mut ui = MinecraftUi::default();
        let lang = language();
        let bar = |name, progress| RaidBar {
            id: 1,
            name,
            progress,
        };
        show_raid_bars(&mut ui, &[bar(BarName::Raid, 0.5)], &lang, 0.05);
        assert_eq!(ui.raid_bars.len(), 1);
        let shown = &ui.raid_bars[0];
        assert_eq!(
            (shown.name.as_str(), shown.color, shown.overlay),
            ("Raid", "red", "notched_10")
        );
        assert!(!shown.world_fog && !shown.music);
        show_raid_bars(
            &mut ui,
            &[bar(BarName::RaidersRemaining(2), 0.25)],
            &lang,
            0.1,
        );
        assert_eq!(ui.raid_bars.len(), 1);
        assert_eq!(ui.raid_bars[0].name, "Raid - Raiders Remaining: 2");
        assert_eq!(ui.raid_bars[0].progress(), 0.25);
        show_raid_bars(&mut ui, &[bar(BarName::Victory, 0.0)], &lang, 0.1);
        assert_eq!(ui.raid_bars[0].name, "Raid - Victory");
        show_raid_bars(&mut ui, &[], &lang, 0.1);
        assert!(ui.raid_bars.is_empty());
    }
}
