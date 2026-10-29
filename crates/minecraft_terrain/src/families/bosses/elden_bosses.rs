use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::bosses::elden_bosses::EldenBoss;

pub const TRACKING_RANGE: i32 = 10;
pub const SHADOW_RADIUS: f32 = 1.0;

impl TerrainFamily for EldenBoss {
    const TRACKING_RANGE: i32 = TRACKING_RANGE;
    const TEXTURES: &'static [&'static str] = &[];
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        crate::elden_save::read(mob, tag);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        crate::elden_save::fields(mob)
    }

    fn draw(
        _out: &mut FamilyDraw<'_>,
        _mobs: &[FamilyMob<Self>],
        _poses: &ClientMobs,
        _atlas: &Atlas,
        _light: &SkyLight,
        _partial: f32,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob_actions::{Actor, attack};
    use elden_boss::margit::MARGIT;
    use glam::DVec3;
    use minecraftoss_entities::tempt::PlayerCandidate;
    use minecraftoss_entities::world::families::Family;
    use minecraftoss_entities::world::{EntityWorld, MobHit, PlayerAttack};
    use minecraftoss_player::inventory::Inventory;
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

    fn sword(player_id: u64, position: DVec3) -> PlayerAttack {
        PlayerAttack {
            player_id,
            position,
            yaw: 0.0,
            attack_damage: 11.0,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        }
    }

    #[test]
    fn two_players_sword_hits_in_one_tick_both_count() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        let one = DVec3::new(0.5, 64.0, 3.0);
        let two = DVec3::new(3.0, 64.0, 0.5);
        let players = [player_at(1, one), player_at(2, two)];
        for _ in 0..5 {
            world.tick_with_players(&mut Field, &players);
        }
        let health = |world: &EntityWorld| world.elden_bosses()[0].state.brain.health();
        let before = health(&world);
        assert_eq!(before, 1280.0, "two fighters at the wake");
        let hit = MobHit::Family(EldenBoss::KEY, id);
        let mut sounds = Vec::new();
        for (player, position) in [(1, one), (2, two)] {
            let mut inventory = Inventory::default();
            let mut actor = Actor {
                player,
                inventory: &mut inventory,
                selected: 0,
                infinite: false,
                entity_loot: None,
                shearing_loot: None,
            };
            let outcome = attack(&mut world, hit, &mut actor, &sword(player, position));
            assert!(outcome.handled);
            sounds.extend(outcome.sounds.into_iter().map(|sound| sound.event));
        }
        assert_eq!(health(&world), before - 22.0);
        assert_eq!(world.elden_bosses()[0].core.health, before - 22.0);
        assert_eq!(world.elden_bosses()[0].state.brain.hurt_by(), &[1, 2]);
        let count = |event: &str| sounds.iter().filter(|sound| *sound == event).count();
        assert_eq!(count("entity.player.attack.strong"), 2, "{sounds:?}");
        assert_eq!(
            count("entity.illusioner.hurt"),
            1,
            "one hurt voice for both"
        );
    }
}
