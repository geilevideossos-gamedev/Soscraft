use std::collections::BTreeMap;

use minecraft_survival::level::Level;
use minecraft_survival::player::Player;
use minecraft_survival::portal;
use minecraft_survival::server::Globals;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::terrain::Dimension;

use crate::minecraft_sounds::Sounds;

const TRAVEL_SOUND: &str = "minecraft:block.portal.travel";
const PORTAL_SOUND_VOLUME: f32 = 0.25;

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
    let Some(due) = portal::step(player, &level.terrain, &globals.game_rules, feet, seconds) else {
        return;
    };
    let Some(travel) = portal::depart(player, &level.terrain, &globals.registries, due) else {
        diag::warn!(World, "Minecraft portal at {:?} leads nowhere", due.entry);
        return;
    };
    if !portal::may_enter(&globals.game_rules, travel.to) {
        return;
    }
    diag::info!(
        World,
        "Minecraft portal at {:?} leads to {} near {:?}",
        due.entry,
        travel.to.dimension_type(),
        travel.target
    );
    portal::set_off(player, travel);
}

pub(crate) fn arriving(player: &mut Player, sounds: Option<&mut Sounds>, packs: &PackStack) {
    let known = player
        .arrival
        .is_some_and(|arrival| arrival.dimension == player.dimension);
    if !player.portal.travel_sound || !known {
        return;
    }
    player.portal.travel_sound = false;
    if let Some(sounds) = sounds {
        let pitch = sounds.random() * 0.4 + 0.8;
        sounds.play(packs, TRAVEL_SOUND, None, PORTAL_SOUND_VOLUME, pitch);
    }
}
