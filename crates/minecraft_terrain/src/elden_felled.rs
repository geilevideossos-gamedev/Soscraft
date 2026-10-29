use elden_boss::data::{Cue, FELLED};
use minecraftoss_entities::world::EntityWorld;

#[derive(Clone, Debug, PartialEq)]
pub struct Felled {
    pub player: u64,
    pub text: &'static str,
    pub sounds: Vec<(String, f32, f32)>,
}

pub fn felled(world: &mut EntityWorld) -> Vec<Felled> {
    let felled = world.take_elden_felled();
    felled
        .into_iter()
        .map(|(def, player)| Felled {
            player,
            text: FELLED,
            sounds: def
                .cues
                .felled
                .iter()
                .filter_map(|cue| match cue {
                    Cue::Sound(sound) => Some((
                        format!("minecraft:{}", sound.event),
                        sound.volume,
                        sound.pitch,
                    )),
                    Cue::Burst(_) => None,
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use elden_boss::margit::MARGIT;
    use glam::DVec3;
    use minecraftoss_entities::tempt::PlayerCandidate;
    use minecraftoss_entities::world::PlayerAttack;
    use minecraftoss_player::{Block, Pos, World};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn player_at(id: u64, position: DVec3) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position,
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
    fn each_fighter_is_told_once_with_the_challenge_chime() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        let front = DVec3::new(0.5, 64.0, 3.0);
        let players = [
            player_at(1, front),
            player_at(4, DVec3::new(9.5, 64.0, 0.5)),
        ];
        for _ in 0..5 {
            world.tick_with_players(&mut Field, &players);
        }
        assert!(felled(&mut world).is_empty(), "he lives");
        let brain = &mut world.elden_boss_mut(id).unwrap().state.brain;
        brain.set_health(1.0);
        let sword = PlayerAttack {
            player_id: 1,
            position: front,
            yaw: 180.0,
            attack_damage: 11.0,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        };
        assert!(world.player_attack(&sword, id).died);
        let told = felled(&mut world);
        let chime = vec![("minecraft:ui.toast.challenge_complete".to_owned(), 1.0, 1.0)];
        let each = |player| Felled {
            player,
            text: "GREAT ENEMY FELLED",
            sounds: chime.clone(),
        };
        assert_eq!(told, [each(1), each(4)]);
        assert!(felled(&mut world).is_empty(), "told once");
    }
}
