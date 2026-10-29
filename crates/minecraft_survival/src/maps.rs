use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use minecraft_net::PlayerUuid;
use minecraft_terrain::maps::{CarriedMap, Carrier, MapStore, wears_map_invisibility};

use crate::player::Player;

pub fn carrier(player: &Player) -> Option<Carrier> {
    let place = player.place?;
    Some(Carrier {
        uuid: player.uuid.clone(),
        dimension: place.dimension.dimension_type().to_owned(),
        x: place.feet[0],
        z: place.feet[2],
        yaw: place.yaw,
        maps: player
            .inventory
            .slots
            .iter()
            .flatten()
            .filter_map(CarriedMap::of)
            .collect(),
        hidden: wears_map_invisibility(&player.inventory),
    })
}

pub fn tick(
    maps: &Mutex<MapStore>,
    players: &BTreeMap<PlayerUuid, Player>,
    game_time: i64,
    ticks: usize,
) {
    if ticks == 0 {
        return;
    }
    let carriers: Vec<Carrier> = players.values().filter_map(carrier).collect();
    let mut maps = maps.lock().unwrap_or_else(PoisonError::into_inner);
    for tick in 0..ticks {
        let behind = (ticks - 1 - tick) as i64;
        maps.tick_carried(&carriers, game_time - behind);
    }
}

#[cfg(test)]
mod tests {
    use minecraft_commands::{CommandServer, PlayerId};
    use minecraft_net::Dimension;
    use minecraft_terrain::game_mode::{GameMode, PlayerMode};
    use minecraft_terrain::maps::{MapData, PLAYER};
    use minecraftoss_player::inventory::{Inventory, ItemStack};

    use super::*;
    use crate::player_nbt::Place;
    use crate::survival::Survival;

    fn player(id: u32, name: &str, place: Option<Place>) -> Player {
        let mut chat = CommandServer::new(PlayerId(id));
        chat.join(PlayerId(id), name);
        let mut player = Player::new(
            chat.profile(PlayerId(id)).expect("a profile"),
            PlayerMode::new(GameMode::Survival),
            Survival::default(),
            Inventory::default(),
        );
        player.place = place;
        player
    }

    fn map_stack(item: &str, id: i32) -> ItemStack {
        let mut stack = ItemStack::new(item, 1);
        stack.components = Some(serde_json::json!({ "minecraft:map_id": id }));
        stack
    }

    #[test]
    fn players_mark_the_maps_they_carry_where_they_stand() {
        let overworld = |x, z, yaw| {
            Some(Place::standing(
                Dimension::Overworld,
                [x, 70.0, z],
                yaw,
                0.0,
            ))
        };
        let mut steve = player(0, "Steve", overworld(10.4, -20.0, 90.0));
        steve.inventory.slots[40] = Some(map_stack("minecraft:filled_map", 0));
        let mut alex = player(1, "Alex", overworld(-2.0, 4.0, 180.0));
        alex.inventory.slots[12] = Some(map_stack("minecraft:filled_map", 0));
        alex.inventory.slots[3] = Some(ItemStack::new("minecraft:map", 1));
        let mut ghost = player(2, "Ghost", None);
        ghost.inventory.slots[0] = Some(map_stack("minecraft:filled_map", 0));
        let carried = carrier(&alex).expect("Alex stands somewhere");
        assert_eq!(
            carried
                .maps
                .iter()
                .map(|map| (map.item.as_str(), map.id))
                .collect::<Vec<_>>(),
            [("minecraft:filled_map", 0)],
            "an empty map is not a map yet"
        );
        assert_eq!(
            (carried.dimension.as_str(), carried.yaw, carried.hidden),
            ("minecraft:overworld", 180.0, false)
        );
        assert_eq!(
            carrier(&ghost),
            None,
            "a player with no place carries nothing"
        );
        let mut store = MapStore::default();
        store.insert(
            0,
            MapData::create_fresh(0.0, 0.0, 0, "minecraft:overworld", true, false),
        );
        let maps = Mutex::new(store);
        let players: BTreeMap<PlayerUuid, Player> = [steve, alex, ghost]
            .into_iter()
            .map(|player| (player.uuid.clone(), player))
            .collect();
        tick(&maps, &players, 100, 0);
        assert!(maps.lock().unwrap().held(0).is_none(), "no tick, no marker");
        tick(&maps, &players, 100, 3);
        let maps = maps.lock().unwrap();
        let held = maps.held(0).expect("the map is held");
        let mut marks: Vec<_> = held
            .decorations()
            .map(|d| (d.kind.as_str(), d.x, d.y, d.rot))
            .collect();
        marks.sort();
        assert_eq!(marks, [(PLAYER, -3, 8, 8), (PLAYER, 21, -39, 4)]);
        assert_eq!(held.holders().count(), 2);
    }
}
