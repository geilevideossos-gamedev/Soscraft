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
    if let Some(due) =
        minecraft_survival::portal::step(player, &level.terrain, &globals.game_rules, feet, seconds)
    {
        diag::info!(
            World,
            "Minecraft player stood long enough in the {:?} portal at {:?}",
            due.kind,
            due.entry
        );
    }
}
