use std::collections::BTreeMap;

use frame::MinecraftUi;
use frame::minecraft_ui::McBossBar;
use minecraft_net::PlayerUuid;
use minecraft_survival::level::Level;
use minecraft_survival::player::Player;
use minecraft_survival::server::Globals;
use minecraft_survival::the_end::Trip;
use minecraft_terrain::terrain::Dimension;

const DRAGON_NAME: &str = "entity.minecraft.ender_dragon";

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
        Trip::OutOfTheEnd(to) => diag::info!(
            World,
            "Minecraft exit portal at {:?} leads to the respawn point in {}",
            entry,
            to.dimension_type()
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
    fn dragon_parts_carry_the_flat_hit_bit_the_bullets_read() {
        assert_eq!(
            minecraft_survival::entities::FLAT_HIT,
            sim::voxel::FLAT_HIT_KEY
        );
    }
}
