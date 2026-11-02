use crate::server_mobs::{MobTables, MobWorld};
use crate::terrain::BlockStates;
use crate::thrown::PlayerBody;
use glam::DVec3;
use minecraftoss_entities::tempt::PlayerCandidate;
use minecraftoss_entities::world::{Bolt, EntityWorld};
use minecraftoss_world::level::Level;
use minecraftoss_world::level::lightning::LightningHit;
use std::cell::RefCell;

pub const BOLT: &str = "minecraft:lightning_bolt";
pub const DAMAGE: f32 = minecraftoss_world::level::lightning::DAMAGE;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerStrike {
    pub player: u64,
    pub at: [f64; 3],
}

pub fn summon(level: &mut Level<'static>, position: [f64; 3]) -> Result<(), String> {
    level.add_lightning(position, false);
    Ok(())
}

pub fn summon_kinds() -> Vec<&'static str> {
    let mut kinds = crate::server_mobs::simulated_kinds();
    kinds.push(BOLT);
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

pub fn strike(
    level: &mut Level<'static>,
    mobs: &mut EntityWorld,
    states: &BlockStates,
    tables: &MobTables,
    players: &[PlayerCandidate],
    struck: &mut Vec<PlayerStrike>,
) {
    let hits = level.take_lightning_hits();
    if hits.is_empty() {
        return;
    }
    let world = MobWorld {
        level: RefCell::new(level),
        states,
        tables,
    };
    for hit in hits {
        let bolt = Bolt {
            uuid: hit.uuid,
            at: DVec3::from_array(hit.at),
        };
        mobs.thunder_hit(&bolt, &world);
        struck.extend(players_in(&hit, players));
    }
}

pub fn players_in(hit: &LightningHit, players: &[PlayerCandidate]) -> Vec<PlayerStrike> {
    players
        .iter()
        .map(PlayerBody::of)
        .filter(|body| {
            let (min, max) = body.bounds();
            body.targetable() && hit.touches(min.to_array(), max.to_array())
        })
        .map(|body| PlayerStrike {
            player: body.id,
            at: hit.at,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(id: u64, feet: [f64; 3]) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position: DVec3::from_array(feet),
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true,
        }
    }

    #[test]
    fn summon_accepts_lightning_bolt() {
        let kinds = summon_kinds();
        assert!(kinds.contains(&BOLT));
        assert!(kinds.contains(&"minecraft:cow"));
        assert!(
            kinds.windows(2).all(|pair| pair[0] < pair[1]),
            "sorted, each once"
        );
    }

    #[test]
    fn a_bolt_strikes_the_living_players_in_its_box() {
        let hit = LightningHit {
            bolt: 1,
            uuid: [1, 2, 3, 4],
            at: [0.5, 64.0, 0.5],
        };
        let spectator = PlayerCandidate {
            spectator: true,
            ..player(3, [0.5, 64.0, 0.5])
        };
        let dead = PlayerCandidate {
            alive: false,
            ..player(4, [0.5, 64.0, 0.5])
        };
        let players = [
            player(1, [2.5, 70.0, -2.5]),
            player(2, [4.5, 64.0, 0.5]),
            spectator,
            dead,
            player(5, [0.5, 54.5, 0.5]),
        ];
        assert_eq!(
            players_in(&hit, &players),
            vec![PlayerStrike {
                player: 1,
                at: hit.at
            }]
        );
    }
}
