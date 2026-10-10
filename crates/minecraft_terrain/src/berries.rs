use crate::bucket::{Change, ItemUse};
use crate::pack::ResourceId;
use crate::server::ItemOnBlock;
use crate::terrain::BlockStates;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::{BlockStateId, SupportType};
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::{Block, Face, Player, Pos, World};

pub const BONE_MEAL: &str = "minecraft:bone_meal";
pub const SWEET_BERRIES: &str = "minecraft:sweet_berries";
pub const GLOW_BERRIES: &str = "minecraft:glow_berries";
pub const BONE_MEAL_SOUND: &str = "minecraft:item.bone_meal.use";
const SWEET_BERRY_BUSH: &str = "minecraft:sweet_berry_bush";
const CAVE_VINES: &str = "minecraft:cave_vines";
const CAVE_VINES_PLANT: &str = "minecraft:cave_vines_plant";
const VEGETATION_SOIL: &str = "minecraft:supports_vegetation";
const RIPE_AGE: &str = "3";
const VINE_AGES: f32 = 25.0;
const OLDEST_PLACED_VINE: u32 = 24;
const USE_REACH: f64 = 4.5;
const PLACE_REACH: f64 = 5.0;

pub trait Soil {
    fn supports_vegetation(&self, block: &Block) -> bool;
    fn sturdy_underside(&self, block: &Block) -> bool;
}

pub struct Ground<'a> {
    pub states: &'a BlockStates,
}

impl Ground<'_> {
    fn state(&self, block: &Block) -> Option<BlockStateId> {
        let id = ResourceId::parse(&block.id).ok()?;
        self.states.state_of(&crate::scene::Block {
            id,
            properties: block.properties.clone(),
        })
    }
}

impl Soil for Ground<'_> {
    fn supports_vegetation(&self, block: &Block) -> bool {
        let registries = self.states.registries();
        match (self.state(block), registries.block_tags.id(VEGETATION_SOIL)) {
            (Some(state), Some(tag)) => registries.block_in_tag(state, tag),
            _ => false,
        }
    }

    fn sturdy_underside(&self, block: &Block) -> bool {
        let blocks = &self.states.registries().blocks;
        self.state(block)
            .is_some_and(|state| blocks.is_face_sturdy(state, Direction::Down, SupportType::Full))
    }
}

pub fn bone_meal_passes(block: &Block, held: Option<&ItemStack>) -> bool {
    held.is_some_and(|stack| stack.id == BONE_MEAL)
        && block.id == SWEET_BERRY_BUSH
        && block.property("age") != Some(RIPE_AGE)
}

fn face_name(face: Face) -> &'static str {
    match face {
        Face::Down => "down",
        Face::Up => "up",
        Face::North => "north",
        Face::South => "south",
        Face::West => "west",
        Face::East => "east",
    }
}

pub fn bone_meal_use(player: &Player, world: &impl World, held: &ItemStack) -> Option<ItemUse> {
    if held.id != BONE_MEAL {
        return None;
    }
    let hit = player.target(world, USE_REACH)?;
    Some(ItemUse {
        change: Change::Server(
            hit.pos,
            ItemOnBlock::BoneMeal {
                face: face_name(hit.face),
            },
        ),
        result: None,
        wear: 0,
        consumed: true,
        sounds: Vec::new(),
    })
}

fn is_cave_vine(block: &Block) -> bool {
    matches!(block.id.as_str(), CAVE_VINES | CAVE_VINES_PLANT)
}

fn shifted(pos: Pos, dy: i32) -> Pos {
    (pos.0, pos.1 + dy, pos.2)
}

pub fn plant(
    player: &Player,
    world: &mut impl World,
    held: &str,
    soil: &impl Soil,
    mut random: impl FnMut() -> f32,
) -> Option<Pos> {
    if held != SWEET_BERRIES && held != GLOW_BERRIES {
        return None;
    }
    let hit = player.target(world, PLACE_REACH)?;
    let pos = minecraftoss_player::placement_pos(world, &hit);
    let below = world.block(shifted(pos, -1));
    let block = if held == SWEET_BERRIES {
        if !below.is_some_and(|b| soil.supports_vegetation(&b)) {
            return None;
        }
        Block::new(SWEET_BERRY_BUSH).with("age", "0")
    } else {
        let hung = world
            .block(shifted(pos, 1))
            .is_some_and(|b| is_cave_vine(&b) || soil.sturdy_underside(&b));
        if !hung {
            return None;
        }
        if below.is_some_and(|b| is_cave_vine(&b)) {
            Block::new(CAVE_VINES_PLANT).with("berries", "false")
        } else {
            let age = ((random() * VINE_AGES) as u32).min(OLDEST_PLACED_VINE);
            Block::new(CAVE_VINES)
                .with("age", &age.to_string())
                .with("berries", "false")
        }
    };
    player.place_target_with(world, block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Blocks(HashMap<Pos, Block>);

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.0.get(&pos).cloned()
        }

        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.0.insert(pos, block),
                None => self.0.remove(&pos),
            };
        }
    }

    struct Dirt;

    impl Soil for Dirt {
        fn supports_vegetation(&self, block: &Block) -> bool {
            matches!(
                block.id.as_str(),
                "minecraft:dirt" | "minecraft:grass_block"
            )
        }

        fn sturdy_underside(&self, block: &Block) -> bool {
            matches!(block.id.as_str(), "minecraft:dirt" | "minecraft:stone")
        }
    }

    fn world(blocks: &[(Pos, Block)]) -> Blocks {
        let mut world = Blocks::default();
        for (pos, block) in blocks {
            world.set_block(*pos, Some(block.clone()));
        }
        world
    }

    fn looking(feet: [f64; 3], pitch: f64) -> Player {
        let mut player = Player::new(DVec3::from_array(feet));
        player.pitch = pitch;
        player
    }

    fn bush(age: &str) -> Block {
        Block::new(SWEET_BERRY_BUSH).with("age", age)
    }

    fn stack(id: &str) -> ItemStack {
        ItemStack::new(id, 4)
    }

    #[test]
    fn bone_meal_passes_an_unripe_bush_by_so_the_meal_grows_it() {
        let meal = stack(BONE_MEAL);
        for age in ["0", "1", "2"] {
            assert!(bone_meal_passes(&bush(age), Some(&meal)), "age {age}");
        }
        assert!(
            !bone_meal_passes(&bush("3"), Some(&meal)),
            "a ripe bush is picked"
        );
        assert!(!bone_meal_passes(&bush("2"), Some(&stack(SWEET_BERRIES))));
        assert!(!bone_meal_passes(&bush("2"), None));
        let vines = Block::new(CAVE_VINES).with("berries", "true");
        assert!(!bone_meal_passes(&vines, Some(&meal)));
    }

    #[test]
    fn bone_meal_asks_the_server_about_the_clicked_face_and_is_spent() {
        let blocks = world(&[
            ((0, 0, 0), Block::new("minecraft:dirt")),
            ((0, 1, 0), bush("1")),
        ]);
        let player = looking([0.5, 2.0, 0.5], 90.0);
        let used = bone_meal_use(&player, &blocks, &stack(BONE_MEAL)).expect("aimed at the bush");
        assert_eq!(
            used.change,
            Change::Server((0, 1, 0), ItemOnBlock::BoneMeal { face: "up" })
        );
        assert!(used.consumed && used.result.is_none() && used.wear == 0);
        assert!(used.sounds.is_empty(), "the sound comes with level event 1505");
        assert_eq!(bone_meal_use(&player, &blocks, &stack(SWEET_BERRIES)), None);
        let sky = looking([0.5, 2.0, 0.5], -90.0);
        assert_eq!(bone_meal_use(&sky, &blocks, &stack(BONE_MEAL)), None);
    }

    #[test]
    fn sweet_berries_plant_a_sapling_bush_only_on_vegetation_soil() {
        let player = looking([0.5, 2.0, 0.5], 90.0);
        let mut grassy = world(&[((0, 0, 0), Block::new("minecraft:grass_block"))]);
        let planted = plant(&player, &mut grassy, SWEET_BERRIES, &Dirt, || 0.5);
        assert_eq!(planted, Some((0, 1, 0)));
        assert_eq!(grassy.block((0, 1, 0)), Some(bush("0")));
        let mut stony = world(&[((0, 0, 0), Block::new("minecraft:stone"))]);
        assert_eq!(
            plant(&player, &mut stony, SWEET_BERRIES, &Dirt, || 0.5),
            None
        );
        assert_eq!(stony.block((0, 1, 0)), None);
        let mut on_bush = world(&[
            ((0, 0, 0), Block::new("minecraft:dirt")),
            ((0, 1, 0), bush("1")),
        ]);
        assert_eq!(
            plant(&player, &mut on_bush, SWEET_BERRIES, &Dirt, || 0.5),
            None,
            "no bush on a bush"
        );
        assert_eq!(
            plant(&player, &mut grassy, "minecraft:apple", &Dirt, || 0.5),
            None
        );
    }

    #[test]
    fn glow_berries_hang_new_vines_under_sturdy_blocks_and_extend_old_ones() {
        let player = looking([0.5, 0.0, 0.5], -90.0);
        let mut cave = world(&[((0, 3, 0), Block::new("minecraft:stone"))]);
        assert_eq!(
            plant(&player, &mut cave, GLOW_BERRIES, &Dirt, || 0.5),
            Some((0, 2, 0))
        );
        let head = Block::new(CAVE_VINES)
            .with("age", "12")
            .with("berries", "false");
        assert_eq!(cave.block((0, 2, 0)), Some(head));
        let mut oldest = world(&[((0, 3, 0), Block::new("minecraft:stone"))]);
        plant(&player, &mut oldest, GLOW_BERRIES, &Dirt, || 0.999_999_9);
        assert_eq!(oldest.block((0, 2, 0)).unwrap().property("age"), Some("24"));
        let mut tunnel = world(&[((0, 2, 0), Block::new("minecraft:stone"))]);
        assert_eq!(
            plant(&player, &mut tunnel, GLOW_BERRIES, &Dirt, || 0.5),
            Some((0, 1, 0)),
            "vines hang into the space the player stands in"
        );
        let mut glass = world(&[((0, 3, 0), Block::new("minecraft:glass"))]);
        assert_eq!(
            plant(&player, &mut glass, GLOW_BERRIES, &Dirt, || 0.5),
            None
        );
        let under_vine = world(&[
            ((0, 4, 0), Block::new("minecraft:stone")),
            (
                (0, 3, 0),
                Block::new(CAVE_VINES)
                    .with("age", "3")
                    .with("berries", "true"),
            ),
        ]);
        let mut longer = under_vine;
        assert_eq!(
            plant(&player, &mut longer, GLOW_BERRIES, &Dirt, || 0.0),
            Some((0, 2, 0))
        );
        assert_eq!(longer.block((0, 2, 0)).unwrap().id, CAVE_VINES);
        let ahead = looking([0.5, 1.0, 0.5], 0.0);
        let mut joined = world(&[
            ((0, 2, -2), Block::new("minecraft:stone")),
            ((0, 3, -1), Block::new("minecraft:stone")),
            (
                (0, 1, -1),
                Block::new(CAVE_VINES)
                    .with("age", "3")
                    .with("berries", "true"),
            ),
        ]);
        assert_eq!(
            plant(&ahead, &mut joined, GLOW_BERRIES, &Dirt, || 0.5),
            Some((0, 2, -1))
        );
        let body = Block::new(CAVE_VINES_PLANT).with("berries", "false");
        assert_eq!(joined.block((0, 2, -1)), Some(body));
    }
}
