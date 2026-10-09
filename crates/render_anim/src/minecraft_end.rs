use std::collections::BTreeMap;

use minecraft_survival::level::Level;
use minecraft_survival::player::Player;
use minecraft_survival::server::Globals;
use minecraft_survival::the_end::Trip;
use minecraft_terrain::terrain::Dimension;

pub(crate) fn step(
    player: &mut Player,
    levels: &BTreeMap<Dimension, Level>,
    globals: &Globals,
    feet: Option<[f64; 3]>,
    seconds: f64,
) {
    let Some(level) = levels.get(&player.dimension) else {
        return;
    };
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
