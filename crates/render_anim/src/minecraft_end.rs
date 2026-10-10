use std::collections::BTreeMap;

use bevy::prelude::{Assets, Image};
use frame::MinecraftUi;
use frame::minecraft_credits::{
    McCredits, McCreditsDiscipline, McCreditsSection, McCreditsText, McCreditsTitle,
};
use frame::minecraft_ui::McBossBar;
use minecraft_net::PlayerUuid;
use minecraft_survival::level::Level;
use minecraft_survival::player::Player;
use minecraft_survival::server::Globals;
use minecraft_survival::the_end::Trip;
use minecraft_terrain::pack::{PackStack, ResourceId};
use minecraft_terrain::terrain::Dimension;

const DRAGON_NAME: &str = "entity.minecraft.ender_dragon";
const END_POEM: &str = "texts/end.txt";
const CREDITS: &str = "texts/credits.json";
const POSTCREDITS: &str = "texts/postcredits.txt";
const LOGO: &str = "minecraft:gui/title/minecraft";
const EDITION: &str = "minecraft:gui/title/edition";

pub(crate) fn step(player: &mut Player, levels: &mut BTreeMap<Dimension, Level>) {
    let Some(level) = levels.get_mut(&player.dimension) else {
        return;
    };
    arriving(player, level);
}

pub(crate) fn travel(
    player: &mut Player,
    level: &Level,
    globals: &Globals,
    entry: (i32, i32, i32),
) {
    match minecraft_survival::the_end::travel(globals, level, player) {
        Trip::IntoTheEnd => diag::info!(
            World,
            "Minecraft end portal at {:?} leads to minecraft:the_end",
            entry
        ),
        Trip::OutOfTheEnd { to, credits } => diag::info!(
            World,
            "Minecraft exit portal at {:?} leads to the respawn point in {}{}",
            entry,
            to.dimension_type(),
            if credits { " behind the credits" } else { "" }
        ),
    }
}

pub(crate) fn arriving(player: &mut Player, level: &mut Level) {
    if minecraft_survival::the_end::build_platform(level, player) {
        diag::info!(
            World,
            "Minecraft End platform rebuilt at {:?}",
            minecraft_survival::the_end::platform_origin()
        );
    }
}

pub(crate) fn tick_fight(
    levels: &mut BTreeMap<Dimension, Level>,
    players: &BTreeMap<PlayerUuid, Player>,
    globals: &Globals,
    seconds: f64,
    local: &PlayerUuid,
    ui: &mut MinecraftUi,
) {
    let Some(level) = levels.get_mut(&Dimension::End) else {
        ui.boss_bars.clear();
        return;
    };
    let events = minecraft_survival::the_end::dragon_fight::tick_level(
        level,
        players.values(),
        globals.save.as_ref(),
        globals.seed,
        seconds,
    );
    match events.loaded {
        Some(Ok(true)) => diag::info!(World, "Minecraft End dragon fight read from the save"),
        Some(Err(error)) => diag::warn!(World, "Minecraft End dragon fight unreadable: {error}"),
        _ => {}
    }
    match events.arena {
        Some(0) => diag::info!(World, "Minecraft End arena released"),
        Some(count) => diag::info!(World, "Minecraft End arena held for {count} players"),
        None => {}
    }
    if let Some(scan) = events.scanned {
        diag::info!(
            World,
            "Minecraft End dragon fight scanned: exit portal at {:?}, dragon killed before {}",
            scan.exit_portal_location,
            scan.previously_killed
        );
    }
    if let Some(uuid) = events.killed {
        diag::info!(
            World,
            "Minecraft ender dragon of the fight killed: {uuid:?}"
        );
    }
    if let Some(at) = events.opened {
        diag::info!(World, "Minecraft exit portal opened at {at:?}");
    }
    if let Some(uuid) = events.spawned {
        diag::info!(World, "Minecraft ender dragon of the fight: {uuid:?}");
    }
    if let Some(Err(error)) = events.saved {
        diag::warn!(World, "Minecraft End dragon fight not saved: {error}");
    }
    let shown = players
        .get(local)
        .filter(|player| player.dimension == Dimension::End && player.switching.is_none())
        .and_then(|_| level.dragon_fight.as_ref()?.boss_bar(local));
    show_boss_bar(ui, shown, seconds);
}

pub(crate) fn credit_sections(json: &serde_json::Value) -> Vec<McCreditsSection> {
    let text = |value: &serde_json::Value, key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let list = |value: &serde_json::Value, key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    json.as_array()
        .into_iter()
        .flatten()
        .map(|section| McCreditsSection {
            section: text(section, "section"),
            disciplines: list(section, "disciplines")
                .iter()
                .map(|discipline| McCreditsDiscipline {
                    discipline: text(discipline, "discipline"),
                    titles: list(discipline, "titles")
                        .iter()
                        .map(|title| McCreditsTitle {
                            title: text(title, "title"),
                            names: list(title, "names")
                                .iter()
                                .filter_map(|name| name.as_str().map(str::to_owned))
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

fn credits_text(packs: &PackStack, images: &mut Assets<Image>) -> McCreditsText {
    let read = |path: &str| {
        ResourceId::parse("minecraft:texts")
            .ok()
            .and_then(|id| packs.get(&id, path).ok().flatten())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    };
    let image = |name: &str, images: &mut Assets<Image>| {
        let id = ResourceId::parse(name).ok()?;
        let bytes = packs.texture(&id).ok()??;
        let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
        Some(images.add(crate::minecraft_loading::gpu_image(
            decoded.to_rgba8(),
            false,
        )))
    };
    let sections = read(CREDITS)
        .and_then(|json| serde_json::from_str(&json).ok())
        .map_or_else(Vec::new, |json| credit_sections(&json));
    McCreditsText {
        poem: read(END_POEM).unwrap_or_default(),
        sections,
        postcredits: read(POSTCREDITS).unwrap_or_default(),
        logo: image(LOGO, images),
        edition: image(EDITION, images),
    }
}

pub(crate) fn credits(
    player: &mut Player,
    ui: &mut MinecraftUi,
    packs: &PackStack,
    images: &mut Assets<Image>,
) {
    if !minecraft_survival::the_end::holding_for_credits(player) {
        ui.credits = None;
        return;
    }
    if ui.credits.as_ref().is_some_and(|credits| credits.finished) {
        minecraft_survival::the_end::credits_done(player);
        ui.credits = None;
        diag::info!(World, "Minecraft credits closed");
        return;
    }
    if ui.credits.is_some() {
        return;
    }
    let Some(font) = ui.gui.as_ref().map(|gui| gui.font.clone()) else {
        minecraft_survival::the_end::credits_done(player);
        diag::warn!(World, "Minecraft credits skipped: no GUI font");
        return;
    };
    let text = credits_text(packs, images);
    let credits = McCredits::new(&text, &player.name, &font);
    diag::info!(
        World,
        "Minecraft credits roll: {} lines, {} sections",
        credits.lines.len(),
        text.sections.len()
    );
    ui.credits = Some(credits);
}

fn show_boss_bar(ui: &mut MinecraftUi, progress: Option<f32>, seconds: f64) {
    let Some(progress) = progress else {
        ui.boss_bars.clear();
        return;
    };
    if ui.boss_bars.is_empty() {
        let name = ui
            .chat
            .language
            .get(DRAGON_NAME)
            .cloned()
            .unwrap_or_else(|| DRAGON_NAME.to_owned());
        ui.boss_bars.push(McBossBar::new(name, progress, true));
    }
    for bar in &mut ui.boss_bars {
        bar.set_progress(progress);
        bar.advance(seconds as f32);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_credits_json_becomes_sections_disciplines_titles_and_names() {
        let json = serde_json::json!([
            {
                "section": "Mojang Studios",
                "disciplines": [
                    {
                        "discipline": "Leadership",
                        "titles": [{ "title": "Studio Head", "names": ["A", "B"] }]
                    }
                ]
            },
            { "section": "Empty" }
        ]);
        let sections = super::credit_sections(&json);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].section, "Mojang Studios");
        assert_eq!(sections[0].disciplines[0].discipline, "Leadership");
        assert_eq!(sections[0].disciplines[0].titles[0].names, ["A", "B"]);
        assert!(sections[1].disciplines.is_empty());
    }

    #[test]
    fn the_vanilla_credits_and_poem_are_read_from_the_jar() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = minecraft_terrain::pack::PackStack::open(vec![pack]).unwrap();
        let mut images = bevy::prelude::Assets::<bevy::prelude::Image>::default();
        let text = super::credits_text(&packs, &mut images);
        assert!(text.poem.contains("PLAYERNAME"));
        assert!(text.sections.len() > 5, "{}", text.sections.len());
        assert!(!text.postcredits.trim().is_empty());
        assert!(text.logo.is_some() && text.edition.is_some());
        let credits =
            frame::minecraft_credits::McCredits::new(&text, "Steve", &frame::McFont::default());
        assert!(credits.lines.len() > 1000, "{}", credits.lines.len());
        assert!(credits.lines.iter().any(|line| line.text() == "Steve?"));
        assert!(
            credits
                .lines
                .iter()
                .any(|line| line.spans.iter().any(|span| span.obfuscated))
        );
    }

    #[test]
    fn dragon_parts_carry_the_flat_hit_bit_the_bullets_read() {
        assert_eq!(
            minecraft_survival::entities::FLAT_HIT,
            sim::voxel::FLAT_HIT_KEY
        );
    }
}
