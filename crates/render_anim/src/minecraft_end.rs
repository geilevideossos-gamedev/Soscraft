use std::collections::BTreeMap;

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
