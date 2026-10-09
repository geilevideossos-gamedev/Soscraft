use std::collections::BTreeMap;

use minecraft_survival::level::Level;
use minecraft_survival::player::Player;
use minecraft_survival::server::Globals;
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
    let Some(due) = minecraft_survival::portal::step(
        player,
        &level.terrain,
        &globals.game_rules,
        feet,
        seconds,
    ) else {
        return;
    };
    match minecraft_survival::portal::depart(player, &level.terrain, &globals.registries, due) {
        Some(travel) => diag::info!(
            World,
            "Minecraft portal at {:?} leads to {} near {:?}",
            due.entry,
            travel.to.dimension_type(),
            travel.target
        ),
        None => diag::warn!(World, "Minecraft portal at {:?} leads nowhere", due.entry),
    }
}
