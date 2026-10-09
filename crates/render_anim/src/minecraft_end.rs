use std::collections::BTreeMap;

use minecraft_net::PlayerUuid;
use minecraft_survival::level::Level;
use minecraft_survival::player::Player;
use minecraft_survival::server::Globals;
use minecraft_survival::the_end::Trip;
use minecraft_terrain::terrain::Dimension;

pub(crate) fn step(
    player: &mut Player,
    levels: &mut BTreeMap<Dimension, Level>,
    globals: &Globals,
    feet: Option<[f64; 3]>,
    seconds: f64,
) {
    let Some(level) = levels.get_mut(&player.dimension) else {
        return;
    };
    arriving(player, level);
    if !minecraft_survival::the_end::step(player, &level.terrain, feet, seconds) {
        return;
    }
    match minecraft_survival::the_end::travel(globals, level, player) {
        Trip::IntoTheEnd => diag::info!(
            World,
            "Minecraft end portal at {:?} leads to minecraft:the_end",
            feet
        ),
        Trip::OutOfTheEnd(to) => diag::info!(
            World,
            "Minecraft exit portal at {:?} leads to the respawn point in {}",
            feet,
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
) {
    let Some(level) = levels.get_mut(&Dimension::End) else {
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
    if let Some(Err(error)) = events.saved {
        diag::warn!(World, "Minecraft End dragon fight not saved: {error}");
    }
}
