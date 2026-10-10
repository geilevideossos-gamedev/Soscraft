use crate::pack::ResourceId;
use crate::server::ItemOnBlock;
use crate::terrain::BlockStates;
use minecraftoss_core::block::flags;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::{BlockStateId, SupportType};
use minecraftoss_generator::feature::update::fire_can_burn;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::{Block, Hit, Player, Pos, World};

pub const BUCKET: &str = "minecraft:bucket";
pub const WATER_BUCKET: &str = "minecraft:water_bucket";
pub const LAVA_BUCKET: &str = "minecraft:lava_bucket";
pub const POWDER_SNOW_BUCKET: &str = "minecraft:powder_snow_bucket";
pub const EMPTY_SOUND: &str = "minecraft:item.bucket.empty";
pub const EMPTY_LAVA_SOUND: &str = "minecraft:item.bucket.empty_lava";
pub const EMPTY_POWDER_SNOW_SOUND: &str = "minecraft:item.bucket.empty_powder_snow";
pub const FILL_SOUND: &str = "minecraft:item.bucket.fill";
pub const FILL_LAVA_SOUND: &str = "minecraft:item.bucket.fill_lava";
pub const FILL_POWDER_SNOW_SOUND: &str = "minecraft:item.bucket.fill_powder_snow";
pub const EVAPORATE_SOUND: &str = "minecraft:block.fire.extinguish";
pub const CAMPFIRE_DOUSE_SOUND: &str = "minecraft:entity.generic.extinguish_fire";
pub const CANDLE_DOUSE_SOUND: &str = "minecraft:block.candle.extinguish";
pub const DRIED_GHAST_SOAK_SOUND: &str = "minecraft:block.dried_ghast.place_in_water";
const REACH: f64 = 4.5;
const POWDER_SNOW: &str = "minecraft:powder_snow";
const EMPTY_BUCKET_STACK: u8 = 16;
const FILLED_BUCKET_STACK: u8 = 1;
const FULL_CAULDRON: &str = "3";
const EVAPORATE_VOLUME: f32 = 0.5;
const EVAPORATE_PITCH: f32 = 2.6;
const EVAPORATE_PITCH_SPREAD: f32 = 0.8;
const POWDER_SNOW_PLACE_PITCH: f32 = 0.8;
const WATER_PLANTS: [&str; 4] = [
    "minecraft:kelp",
    "minecraft:kelp_plant",
    "minecraft:seagrass",
    "minecraft:tall_seagrass",
];

pub trait Materials {
    fn can_be_replaced(&self, block: &Block) -> bool;
    fn sturdy_top(&self, block: &Block) -> bool;
    fn flammable(&self, block: &Block) -> bool;
    fn water_evaporates(&self) -> bool;
}

pub struct Catalog<'a> {
    pub states: &'a BlockStates,
    pub water_evaporates: bool,
}

impl Catalog<'_> {
    fn state(&self, block: &Block) -> Option<BlockStateId> {
        let id = ResourceId::parse(&block.id).ok()?;
        self.states.state_of(&crate::scene::Block {
            id,
            properties: block.properties.clone(),
        })
    }
}

impl Materials for Catalog<'_> {
    fn can_be_replaced(&self, block: &Block) -> bool {
        let blocks = &self.states.registries().blocks;
        let fixed = matches!(
            block.id.as_str(),
            "minecraft:end_portal" | "minecraft:end_gateway"
        );
        !fixed
            && self.state(block).is_some_and(|state| {
                blocks.is(state, flags::REPLACEABLE) || !blocks.is(state, flags::LEGACY_SOLID)
            })
    }

    fn sturdy_top(&self, block: &Block) -> bool {
        let blocks = &self.states.registries().blocks;
        self.state(block)
            .is_some_and(|state| blocks.is_face_sturdy(state, Direction::Up, SupportType::Full))
    }

    fn flammable(&self, block: &Block) -> bool {
        self.state(block)
            .is_some_and(|state| fire_can_burn(self.states.registries(), state))
    }

    fn water_evaporates(&self) -> bool {
        self.water_evaporates
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fluid {
    Water,
    Lava,
}

impl Fluid {
    pub fn of_bucket(id: &str) -> Option<Self> {
        match id {
            WATER_BUCKET => Some(Self::Water),
            LAVA_BUCKET => Some(Self::Lava),
            _ => None,
        }
    }

    fn empty_sound(self) -> &'static str {
        match self {
            Self::Water => EMPTY_SOUND,
            Self::Lava => EMPTY_LAVA_SOUND,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sound {
    pub event: &'static str,
    pub at: Option<[f64; 3]>,
    pub volume: f32,
    pub pitch: f32,
}

impl Sound {
    pub fn at_block(event: &'static str, pos: Pos) -> Self {
        Self {
            event,
            at: Some(centre(pos)),
            volume: 1.0,
            pitch: 1.0,
        }
    }

    pub fn on_player(event: &'static str) -> Self {
        Self {
            event,
            at: None,
            volume: 1.0,
            pitch: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Set(Pos, Block),
    Server(Pos, ItemOnBlock),
    ThrowEye(crate::ender_eye::EyeThrow),
    Rocket(i32),
    Breath,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemUse {
    pub change: Change,
    pub result: Option<ItemStack>,
    pub wear: u32,
    pub consumed: bool,
    pub sounds: Vec<Sound>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CauldronUse {
    Changed {
        block: Block,
        result: ItemStack,
        sound: &'static str,
    },
    Consumed,
}

pub fn centre(pos: Pos) -> [f64; 3] {
    [
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    ]
}

pub fn offset(pos: Pos, by: Pos) -> Pos {
    (pos.0 + by.0, pos.1 + by.1, pos.2 + by.2)
}

pub fn aim(player: &Player, world: &impl World, source_fluids: bool) -> Option<Hit> {
    if source_fluids {
        player.target_source_fluid(world, REACH)
    } else {
        player.target(world, REACH)
    }
}

pub fn bucket_stack(id: &str) -> ItemStack {
    let mut stack = ItemStack::new(id, 1);
    stack.max = if id == BUCKET {
        EMPTY_BUCKET_STACK
    } else {
        FILLED_BUCKET_STACK
    };
    stack
}

fn holds_water(block: &Block) -> bool {
    block.id != "minecraft:lava" && minecraftoss_player::holds_fluid(block)
}

fn liquid_container(block: &Block) -> bool {
    block.property("waterlogged").is_some() || WATER_PLANTS.contains(&block.id.as_str())
}

fn accepts_water(block: &Block) -> bool {
    block.property("waterlogged").is_some()
        && !(block.id.ends_with("_slab") && block.property("type") == Some("double"))
        && block.id != "minecraft:barrier"
}

fn is_campfire(block: &Block) -> bool {
    matches!(
        block.id.as_str(),
        "minecraft:campfire" | "minecraft:soul_campfire"
    )
}

fn is_candle(block: &Block) -> bool {
    block.id == "minecraft:candle" || block.id.ends_with("_candle")
}

pub fn cauldron_use(block: &Block, held: &str, above: Option<&Block>) -> Option<CauldronUse> {
    let cauldron = matches!(
        block.id.as_str(),
        "minecraft:cauldron"
            | "minecraft:water_cauldron"
            | "minecraft:lava_cauldron"
            | "minecraft:powder_snow_cauldron"
    );
    if !cauldron {
        return None;
    }
    let under_water = above.is_some_and(holds_water);
    let full = block.property("level") == Some(FULL_CAULDRON);
    let emptied = |block: Block, sound: &'static str| {
        Some(CauldronUse::Changed {
            block,
            result: bucket_stack(BUCKET),
            sound,
        })
    };
    let filled = |bucket: &str, sound: &'static str| {
        Some(CauldronUse::Changed {
            block: Block::new("minecraft:cauldron"),
            result: bucket_stack(bucket),
            sound,
        })
    };
    match held {
        WATER_BUCKET => emptied(
            Block::new("minecraft:water_cauldron").with("level", FULL_CAULDRON),
            EMPTY_SOUND,
        ),
        LAVA_BUCKET | POWDER_SNOW_BUCKET if under_water => Some(CauldronUse::Consumed),
        LAVA_BUCKET => emptied(Block::new("minecraft:lava_cauldron"), EMPTY_LAVA_SOUND),
        POWDER_SNOW_BUCKET => emptied(
            Block::new("minecraft:powder_snow_cauldron").with("level", FULL_CAULDRON),
            EMPTY_POWDER_SNOW_SOUND,
        ),
        BUCKET => match block.id.as_str() {
            "minecraft:water_cauldron" if full => filled(WATER_BUCKET, FILL_SOUND),
            "minecraft:lava_cauldron" => filled(LAVA_BUCKET, FILL_LAVA_SOUND),
            "minecraft:powder_snow_cauldron" if full => {
                filled(POWDER_SNOW_BUCKET, FILL_POWDER_SNOW_SOUND)
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn pickup(block: &Block) -> Option<(&'static str, &'static str)> {
    let source = block.property("level").is_none_or(|level| level == "0");
    match block.id.as_str() {
        "minecraft:water" => source.then_some((WATER_BUCKET, FILL_SOUND)),
        "minecraft:lava" => source.then_some((LAVA_BUCKET, FILL_LAVA_SOUND)),
        "minecraft:bubble_column" => Some((WATER_BUCKET, FILL_SOUND)),
        POWDER_SNOW => Some((POWDER_SNOW_BUCKET, FILL_POWDER_SNOW_SOUND)),
        "minecraft:barrier" => None,
        _ => (block.property("waterlogged") == Some("true")).then_some((WATER_BUCKET, FILL_SOUND)),
    }
}

pub fn bucket_use(
    player: &Player,
    world: &impl World,
    materials: &impl Materials,
    held: &ItemStack,
    random: impl FnMut() -> f32,
) -> Option<ItemUse> {
    let fluid = Fluid::of_bucket(&held.id);
    let empty = held.id == BUCKET;
    let powder_snow = held.id == POWDER_SNOW_BUCKET;
    if fluid.is_none() && !empty && !powder_snow {
        return None;
    }
    if !player.crouching {
        let cauldron = aim(player, world, false).and_then(|hit| {
            let block = world.block(hit.pos)?;
            let above = world.block(offset(hit.pos, (0, 1, 0)));
            cauldron_use(&block, &held.id, above.as_ref()).map(|used| (hit.pos, used))
        });
        match cauldron {
            Some((
                pos,
                CauldronUse::Changed {
                    block,
                    result,
                    sound,
                },
            )) => {
                return Some(ItemUse {
                    change: Change::Set(pos, block),
                    result: Some(result),
                    wear: 0,
                    consumed: false,
                    sounds: vec![Sound::at_block(sound, pos)],
                });
            }
            Some((_, CauldronUse::Consumed)) => return None,
            None => {}
        }
    }
    match fluid {
        Some(fluid) => pour(player, world, materials, fluid, random),
        None if empty => fill(player, world),
        None => place_powder_snow(player, world),
    }
}

fn fill(player: &Player, world: &impl World) -> Option<ItemUse> {
    let hit = aim(player, world, true)?;
    let (bucket, sound) = pickup(&world.block(hit.pos)?)?;
    Some(ItemUse {
        change: Change::Server(hit.pos, ItemOnBlock::FillBucket),
        result: Some(bucket_stack(bucket)),
        wear: 0,
        consumed: false,
        sounds: vec![Sound::on_player(sound)],
    })
}

fn pour(
    player: &Player,
    world: &impl World,
    materials: &impl Materials,
    fluid: Fluid,
    mut random: impl FnMut() -> f32,
) -> Option<ItemUse> {
    let hit = aim(player, world, false)?;
    let adjacent = offset(hit.pos, hit.face.offset());
    let into_clicked =
        fluid == Fluid::Water && world.block(hit.pos).is_some_and(|b| liquid_container(&b));
    let target = if into_clicked { hit.pos } else { adjacent };
    let (pos, sounds) = empty_contents(
        player,
        world,
        materials,
        fluid,
        target,
        Some(adjacent),
        &mut random,
    )?;
    Some(ItemUse {
        change: Change::Server(
            pos,
            ItemOnBlock::EmptyBucket {
                water: fluid == Fluid::Water,
            },
        ),
        result: Some(bucket_stack(BUCKET)),
        wear: 0,
        consumed: false,
        sounds,
    })
}

pub fn evaporated(outcome: &ItemUse) -> Option<Pos> {
    let Change::Server(pos, ItemOnBlock::EmptyBucket { water: true }) = outcome.change else {
        return None;
    };
    outcome
        .sounds
        .iter()
        .any(|sound| sound.event == EVAPORATE_SOUND)
        .then_some(pos)
}

fn empty_contents(
    player: &Player,
    world: &impl World,
    materials: &impl Materials,
    fluid: Fluid,
    pos: Pos,
    retry: Option<Pos>,
    random: &mut impl FnMut() -> f32,
) -> Option<(Pos, Vec<Sound>)> {
    let block = world.block(pos);
    let water = fluid == Fluid::Water;
    let replaceable = block.as_ref().is_none_or(|b| materials.can_be_replaced(b));
    let placeable = replaceable || water && block.as_ref().is_some_and(accepts_water);
    let fits = block.is_none() || placeable && (!player.crouching || retry.is_none());
    if !fits {
        let next = retry?;
        return empty_contents(player, world, materials, fluid, next, None, random);
    }
    if water && materials.water_evaporates() {
        let pitch = EVAPORATE_PITCH + (random() - random()) * EVAPORATE_PITCH_SPREAD;
        let hiss = Sound {
            event: EVAPORATE_SOUND,
            at: Some(centre(pos)),
            volume: EVAPORATE_VOLUME,
            pitch,
        };
        return Some((pos, vec![hiss]));
    }
    let mut sounds = Vec::new();
    if let Some(container) = block.as_ref().filter(|b| water && liquid_container(b)) {
        if container.property("waterlogged") == Some("false") {
            let lit = container.property("lit") == Some("true");
            if lit && is_campfire(container) {
                sounds.push(Sound::at_block(CAMPFIRE_DOUSE_SOUND, pos));
            } else if lit && is_candle(container) {
                sounds.push(Sound::at_block(CANDLE_DOUSE_SOUND, pos));
            } else if container.id == "minecraft:dried_ghast" {
                sounds.push(Sound::at_block(DRIED_GHAST_SOAK_SOUND, pos));
            }
        }
        sounds.push(Sound::at_block(fluid.empty_sound(), pos));
        return Some((pos, sounds));
    }
    if !(world.min_y()..=world.max_y()).contains(&pos.1) {
        return None;
    }
    sounds.push(Sound::at_block(fluid.empty_sound(), pos));
    Some((pos, sounds))
}

fn place_powder_snow(player: &Player, world: &impl World) -> Option<ItemUse> {
    let mut staged = crate::placing::Staged::new(world);
    let pos = player.place_target_with(&mut staged, Block::new(POWDER_SNOW))?;
    Some(ItemUse {
        change: Change::Set(pos, Block::new(POWDER_SNOW)),
        result: Some(bucket_stack(BUCKET)),
        wear: 0,
        consumed: false,
        sounds: vec![Sound {
            event: EMPTY_POWDER_SNOW_SOUND,
            at: Some(centre(pos)),
            volume: 1.0,
            pitch: POWDER_SNOW_PLACE_PITCH,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use minecraftoss_player::Face;
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

    #[derive(Default)]
    struct Rules {
        evaporates: bool,
    }

    impl Materials for Rules {
        fn can_be_replaced(&self, block: &Block) -> bool {
            matches!(
                block.id.as_str(),
                "minecraft:water"
                    | "minecraft:lava"
                    | "minecraft:short_grass"
                    | "minecraft:torch"
                    | "minecraft:kelp"
            )
        }

        fn sturdy_top(&self, block: &Block) -> bool {
            matches!(
                block.id.as_str(),
                "minecraft:stone" | "minecraft:tnt" | "minecraft:oak_planks"
            )
        }

        fn flammable(&self, block: &Block) -> bool {
            matches!(block.id.as_str(), "minecraft:oak_planks" | "minecraft:tnt")
        }

        fn water_evaporates(&self) -> bool {
            self.evaporates
        }
    }

    fn looking_down(blocks: &[(Pos, Block)]) -> (Player, Blocks) {
        let mut world = Blocks::default();
        for (pos, block) in blocks {
            world.set_block(*pos, Some(block.clone()));
        }
        let mut player = Player::new(DVec3::new(0.5, 2.0, 0.5));
        player.pitch = 90.0;
        (player, world)
    }

    fn stone() -> (Pos, Block) {
        ((0, 0, 0), Block::new("minecraft:stone"))
    }

    fn held(id: &str) -> ItemStack {
        bucket_stack(id)
    }

    fn no_random() -> f32 {
        0.5
    }

    fn served(used: &ItemUse) -> Option<(Pos, ItemOnBlock)> {
        match used.change {
            Change::Server(pos, action) => Some((pos, action)),
            Change::Set(..) | Change::ThrowEye(..) | Change::Rocket(..) | Change::Breath => None,
        }
    }

    fn events(used: &ItemUse) -> Vec<&'static str> {
        used.sounds.iter().map(|s| s.event).collect()
    }

    #[test]
    fn filled_buckets_pour_a_source_onto_the_clicked_face() {
        let (player, world) = looking_down(&[stone()]);
        let rules = Rules::default();
        let water = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
        assert_eq!(
            served(&water),
            Some(((0, 1, 0), ItemOnBlock::EmptyBucket { water: true }))
        );
        let result = water.result.clone().unwrap();
        assert_eq!((result.id.as_str(), result.max), (BUCKET, 16));
        assert_eq!(water.sounds, vec![Sound::at_block(EMPTY_SOUND, (0, 1, 0))]);
        let lava = bucket_use(&player, &world, &rules, &held(LAVA_BUCKET), no_random).unwrap();
        assert_eq!(
            served(&lava),
            Some(((0, 1, 0), ItemOnBlock::EmptyBucket { water: false }))
        );
        assert_eq!(events(&lava), vec![EMPTY_LAVA_SOUND]);
    }

    #[test]
    fn water_waterlogs_the_clicked_block_unless_sneaking_and_lava_never_does() {
        let stairs = Block::new("minecraft:oak_stairs").with("waterlogged", "false");
        let (mut player, world) = looking_down(&[((0, 0, 0), stairs)]);
        let rules = Rules::default();
        let water = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
        assert_eq!(
            served(&water),
            Some(((0, 0, 0), ItemOnBlock::EmptyBucket { water: true }))
        );
        let lava = bucket_use(&player, &world, &rules, &held(LAVA_BUCKET), no_random).unwrap();
        assert_eq!(served(&lava).map(|(pos, _)| pos), Some((0, 1, 0)));
        player.crouching = true;
        let sneaking = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
        assert_eq!(served(&sneaking).map(|(pos, _)| pos), Some((0, 1, 0)));
    }

    #[test]
    fn a_double_slab_takes_no_water_so_it_lands_on_top() {
        let slab = Block::new("minecraft:stone_slab")
            .with("type", "double")
            .with("waterlogged", "false");
        let (player, world) = looking_down(&[((0, 0, 0), slab)]);
        let water = bucket_use(
            &player,
            &world,
            &Rules::default(),
            &held(WATER_BUCKET),
            no_random,
        )
        .unwrap();
        assert_eq!(served(&water).map(|(pos, _)| pos), Some((0, 1, 0)));
    }

    #[test]
    fn water_douses_lit_campfires_and_candles_with_their_sounds() {
        let rules = Rules::default();
        for (block, douse) in [
            (Block::new("minecraft:campfire"), CAMPFIRE_DOUSE_SOUND),
            (Block::new("minecraft:soul_campfire"), CAMPFIRE_DOUSE_SOUND),
            (Block::new("minecraft:red_candle"), CANDLE_DOUSE_SOUND),
        ] {
            let lit = block.with("lit", "true").with("waterlogged", "false");
            let (player, world) = looking_down(&[((0, 0, 0), lit.clone())]);
            let used = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
            assert_eq!(events(&used), vec![douse, EMPTY_SOUND], "{lit:?}");
            let unlit = lit.with("lit", "false");
            let (player, world) = looking_down(&[((0, 0, 0), unlit)]);
            let used = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
            assert_eq!(events(&used), vec![EMPTY_SOUND]);
        }
    }

    #[test]
    fn water_evaporates_where_the_dimension_says_so_but_lava_does_not() {
        let (player, world) = looking_down(&[stone()]);
        let rules = Rules { evaporates: true };
        let mut rolls = [0.75, 0.25].into_iter();
        let water = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), || {
            rolls.next().unwrap()
        })
        .unwrap();
        assert_eq!(water.sounds.len(), 1);
        assert_eq!(water.sounds[0].event, EVAPORATE_SOUND);
        assert_eq!(water.sounds[0].volume, 0.5);
        assert!((water.sounds[0].pitch - (2.6 + 0.5 * 0.8)).abs() < 1e-6);
        assert!(evaporated(&water).is_some());
        assert_eq!(water.result.map(|r| r.id), Some(BUCKET.to_owned()));
        let lava = bucket_use(&player, &world, &rules, &held(LAVA_BUCKET), no_random).unwrap();
        assert_eq!(events(&lava), vec![EMPTY_LAVA_SOUND]);
        assert_eq!(evaporated(&lava), None);
        let kept = Rules { evaporates: false };
        let poured = bucket_use(&player, &world, &kept, &held(WATER_BUCKET), no_random).unwrap();
        assert_eq!(evaporated(&poured), None);
    }

    #[test]
    fn nothing_pours_above_the_build_height() {
        let mut world = Blocks::default();
        world.set_block((0, 319, 0), Some(Block::new("minecraft:stone")));
        let mut player = Player::new(DVec3::new(0.5, 321.0, 0.5));
        player.pitch = 90.0;
        let used = bucket_use(
            &player,
            &world,
            &Rules::default(),
            &held(WATER_BUCKET),
            no_random,
        );
        assert_eq!(used, None);
    }

    #[test]
    fn an_empty_bucket_takes_sources_and_waterlogged_water_only() {
        let rules = Rules::default();
        let bucket = held(BUCKET);
        for (block, filled, sound) in [
            (
                Block::new("minecraft:water").with("level", "0"),
                WATER_BUCKET,
                FILL_SOUND,
            ),
            (
                Block::new("minecraft:lava").with("level", "0"),
                LAVA_BUCKET,
                FILL_LAVA_SOUND,
            ),
            (
                Block::new("minecraft:oak_stairs").with("waterlogged", "true"),
                WATER_BUCKET,
                FILL_SOUND,
            ),
            (
                Block::new(POWDER_SNOW),
                POWDER_SNOW_BUCKET,
                FILL_POWDER_SNOW_SOUND,
            ),
        ] {
            let (player, world) = looking_down(&[stone(), ((0, 1, 0), block.clone())]);
            let used = bucket_use(&player, &world, &rules, &bucket, no_random).unwrap();
            assert_eq!(
                served(&used),
                Some(((0, 1, 0), ItemOnBlock::FillBucket)),
                "{block:?}"
            );
            let result = used.result.unwrap();
            assert_eq!((result.id.as_str(), result.max), (filled, 1));
            assert_eq!(used.sounds, vec![Sound::on_player(sound)]);
        }
        for block in [
            Block::new("minecraft:water").with("level", "3"),
            Block::new("minecraft:oak_stairs").with("waterlogged", "false"),
            Block::new("minecraft:barrier").with("waterlogged", "true"),
        ] {
            let (player, world) = looking_down(&[stone(), ((0, 1, 0), block.clone())]);
            assert_eq!(
                bucket_use(&player, &world, &rules, &bucket, no_random),
                None,
                "{block:?}"
            );
        }
    }

    #[test]
    fn cauldrons_take_whole_buckets_and_give_them_back_when_full() {
        let empty = Block::new("minecraft:cauldron");
        let water = Block::new("minecraft:water_cauldron").with("level", "3");
        let changed = |block: &Block, held: &str| match cauldron_use(block, held, None) {
            Some(CauldronUse::Changed {
                block,
                result,
                sound,
            }) => Some((block, result.id, sound)),
            _ => None,
        };
        assert_eq!(
            changed(&empty, WATER_BUCKET),
            Some((water.clone(), BUCKET.to_owned(), EMPTY_SOUND))
        );
        assert_eq!(
            changed(&empty, LAVA_BUCKET),
            Some((
                Block::new("minecraft:lava_cauldron"),
                BUCKET.to_owned(),
                EMPTY_LAVA_SOUND
            ))
        );
        assert_eq!(
            changed(&empty, POWDER_SNOW_BUCKET).map(|(block, ..)| block),
            Some(Block::new("minecraft:powder_snow_cauldron").with("level", "3"))
        );
        assert_eq!(
            changed(&water, BUCKET),
            Some((empty.clone(), WATER_BUCKET.to_owned(), FILL_SOUND))
        );
        assert_eq!(
            changed(&Block::new("minecraft:lava_cauldron"), BUCKET),
            Some((empty.clone(), LAVA_BUCKET.to_owned(), FILL_LAVA_SOUND))
        );
        assert_eq!(
            changed(&water.clone().with("level", "2"), BUCKET),
            None,
            "only a full cauldron fills a bucket"
        );
        assert_eq!(
            changed(&Block::new("minecraft:lava_cauldron"), WATER_BUCKET).map(|(block, ..)| block),
            Some(water.clone())
        );
        assert_eq!(
            cauldron_use(&Block::new("minecraft:stone"), BUCKET, None),
            None
        );
    }

    #[test]
    fn lava_and_powder_snow_do_not_go_into_a_cauldron_under_water() {
        let above = Block::new("minecraft:water").with("level", "0");
        let cauldron = Block::new("minecraft:cauldron");
        for held in [LAVA_BUCKET, POWDER_SNOW_BUCKET] {
            assert_eq!(
                cauldron_use(&cauldron, held, Some(&above)),
                Some(CauldronUse::Consumed)
            );
        }
        assert!(matches!(
            cauldron_use(&cauldron, WATER_BUCKET, Some(&above)),
            Some(CauldronUse::Changed { .. })
        ));
        let (player, world) = looking_down(&[((0, 0, 0), cauldron), ((0, 1, 0), above)]);
        let used = bucket_use(
            &player,
            &world,
            &Rules::default(),
            &held(LAVA_BUCKET),
            no_random,
        );
        assert_eq!(used, None);
    }

    #[test]
    fn a_bucket_on_a_cauldron_sets_it_unless_sneaking() {
        let (mut player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:cauldron"))]);
        let rules = Rules::default();
        let used = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
        assert_eq!(
            used.change,
            Change::Set(
                (0, 0, 0),
                Block::new("minecraft:water_cauldron").with("level", "3")
            )
        );
        assert_eq!(used.sounds, vec![Sound::at_block(EMPTY_SOUND, (0, 0, 0))]);
        player.crouching = true;
        let used = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
        assert_eq!(served(&used).map(|(pos, _)| pos), Some((0, 1, 0)));
    }

    #[test]
    fn a_powder_snow_bucket_places_powder_snow_and_leaves_a_bucket() {
        let (player, world) = looking_down(&[stone()]);
        let used = bucket_use(
            &player,
            &world,
            &Rules::default(),
            &held(POWDER_SNOW_BUCKET),
            no_random,
        )
        .unwrap();
        assert_eq!(used.change, Change::Set((0, 1, 0), Block::new(POWDER_SNOW)));
        assert_eq!(used.result.map(|r| r.id), Some(BUCKET.to_owned()));
        assert_eq!(used.sounds[0].event, EMPTY_POWDER_SNOW_SOUND);
        assert_eq!((used.sounds[0].volume, used.sounds[0].pitch), (1.0, 0.8));
    }

    #[test]
    fn torch_and_plant_outlines_catch_the_aim() {
        let rules = Rules::default();
        let torch = ((0, 1, 0), Block::new("minecraft:torch"));
        let (mut player, world) = looking_down(&[stone(), torch]);
        let onto = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
        assert_eq!(served(&onto).map(|(pos, _)| pos), Some((0, 2, 0)));
        player.pos = DVec3::new(0.1, 2.0, 0.1);
        let past = bucket_use(&player, &world, &rules, &held(WATER_BUCKET), no_random).unwrap();
        assert_eq!(served(&past).map(|(pos, _)| pos), Some((0, 1, 0)));
        let (mut player, world) = looking_down(&[
            stone(),
            (
                (0, 1, 0),
                Block::new("minecraft:tall_grass").with("half", "lower"),
            ),
            (
                (0, 2, 0),
                Block::new("minecraft:tall_grass").with("half", "upper"),
            ),
        ]);
        player.pos = DVec3::new(0.5, 1.0, 0.5);
        let inside = aim(&player, &world, false).unwrap();
        assert_eq!(
            (inside.pos, inside.face, inside.distance),
            ((0, 2, 0), Face::Up, 0.0)
        );
    }

    #[test]
    fn other_items_are_not_buckets() {
        let (player, world) = looking_down(&[stone()]);
        let stick = ItemStack::new("minecraft:stick", 1);
        assert_eq!(
            bucket_use(&player, &world, &Rules::default(), &stick, no_random),
            None
        );
    }

    #[test]
    fn the_block_catalog_answers_replacement_support_and_flammability() {
        use minecraftoss_core::Registries;
        use minecraftoss_core::registries::DataPaths;
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let states = BlockStates::new(std::sync::Arc::new(registries), 0, -64, 384).unwrap();
        let catalog = Catalog {
            states: &states,
            water_evaporates: false,
        };
        let block = Block::new;
        assert!(catalog.can_be_replaced(&block("minecraft:short_grass")));
        assert!(catalog.can_be_replaced(&block("minecraft:water")));
        assert!(catalog.can_be_replaced(&block("minecraft:torch")));
        assert!(!catalog.can_be_replaced(&block("minecraft:stone")));
        assert!(!catalog.can_be_replaced(&block("minecraft:end_portal")));
        assert!(catalog.sturdy_top(&block("minecraft:stone")));
        assert!(!catalog.sturdy_top(&block("minecraft:glass_pane")));
        assert!(catalog.flammable(&block("minecraft:oak_planks")));
        assert!(!catalog.flammable(&block("minecraft:stone")));
        assert!(!catalog.flammable(&block("minecraft:oak_stairs").with("waterlogged", "true")));
    }
}
