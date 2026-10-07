use crate::bucket::{Change, ItemUse, Materials, Sound, aim, centre, offset};
use crate::server::ItemOnBlock;
use minecraftoss_core::BlockPos;
use minecraftoss_core::pos::Axis;
use minecraftoss_generator::feature::portal::{Cell, NETHER_PORTAL, PortalShape};
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::{Block, Hit, Player, Pos, World};

pub const FLINT_AND_STEEL: &str = "minecraft:flint_and_steel";
pub const FIRE_CHARGE: &str = "minecraft:fire_charge";
pub const USE_SOUND: &str = "minecraft:item.flintandsteel.use";
pub const FIRE_CHARGE_SOUND: &str = "minecraft:item.firecharge.use";
pub const TNT_PRIMED_SOUND: &str = "minecraft:entity.tnt.primed";
pub const BREAK_SOUND: &str = "minecraft:entity.item.break";
const WEAR: u32 = 1;
const USE_PITCH: f32 = 0.8;
const USE_PITCH_SPREAD: f32 = 0.4;
const BREAK_VOLUME: f32 = 0.8;
const BREAK_PITCH: f32 = 0.8;
const BREAK_PITCH_SPREAD: f32 = 0.4;
const FIRE_CHARGE_PITCH_SPREAD: f32 = 0.2;
const SIDES: [Pos; 6] = [
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
    (-1, 0, 0),
    (1, 0, 0),
];

pub fn can_light(block: &Block) -> bool {
    let unlit = block.property("lit") == Some("false");
    let dry = block.property("waterlogged") != Some("true");
    let id = block.id.as_str();
    let campfire = matches!(id, "minecraft:campfire" | "minecraft:soul_campfire");
    let candle = id == "minecraft:candle" || id.ends_with("_candle");
    let candle_cake = id == "minecraft:candle_cake" || id.ends_with("_candle_cake");
    unlit && ((campfire || candle) && dry || candle_cake)
}

fn is_air(block: Option<&Block>) -> bool {
    block.is_none_or(|b| {
        matches!(
            b.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

fn soul_fire_base(block: &Block) -> bool {
    matches!(
        block.id.as_str(),
        "minecraft:soul_sand" | "minecraft:soul_soil"
    )
}

pub fn fire_fits(world: &impl World, materials: &impl Materials, pos: Pos) -> bool {
    if !is_air(world.block(pos).as_ref()) {
        return false;
    }
    let below = world.block(offset(pos, (0, -1, 0)));
    if below.as_ref().is_some_and(soul_fire_base) {
        return true;
    }
    below.as_ref().is_some_and(|b| materials.sturdy_top(b))
        || SIDES.iter().any(|&side| {
            world
                .block(offset(pos, side))
                .is_some_and(|b| materials.flammable(&b))
        })
}

fn portal_cell(block: Option<Block>) -> Cell {
    match block.as_ref().map(|b| b.id.as_str()) {
        Some("minecraft:obsidian") => Cell::Frame,
        Some("minecraft:fire" | "minecraft:soul_fire") => Cell::Empty,
        Some(NETHER_PORTAL) => Cell::Portal,
        _ if is_air(block.as_ref()) => Cell::Empty,
        _ => Cell::Other,
    }
}

pub fn opens_portal(world: &impl World, pos: Pos) -> bool {
    let cell = |at: BlockPos| portal_cell(world.block((at.x, at.y, at.z)));
    is_air(world.block(pos).as_ref())
        && SIDES
            .iter()
            .any(|&side| portal_cell(world.block(offset(pos, side))) == Cell::Frame)
        && PortalShape::find_empty(&cell, BlockPos::new(pos.0, pos.1, pos.2), Axis::X, i32::MIN)
            .is_some()
}

fn light_or_ignite(
    world: &impl World,
    materials: &impl Materials,
    block: Block,
    hit: &Hit,
) -> Option<(Change, Pos)> {
    if can_light(&block) {
        return Some((Change::Set(hit.pos, block.with("lit", "true")), hit.pos));
    }
    let adjacent = offset(hit.pos, hit.face.offset());
    if !fire_fits(world, materials, adjacent) && !opens_portal(world, adjacent) {
        return None;
    }
    Some((Change::Server(adjacent, ItemOnBlock::Ignite), adjacent))
}

pub fn break_sound(random: f32) -> Sound {
    Sound {
        event: BREAK_SOUND,
        at: None,
        volume: BREAK_VOLUME,
        pitch: BREAK_PITCH + random * BREAK_PITCH_SPREAD,
    }
}

fn prime_tnt(pos: Pos, wear: u32, consumed: bool) -> ItemUse {
    let fuse_at = [
        f64::from(pos.0) + 0.5,
        f64::from(pos.1),
        f64::from(pos.2) + 0.5,
    ];
    ItemUse {
        change: Change::Server(pos, ItemOnBlock::PrimeTnt),
        result: None,
        wear,
        consumed,
        sounds: vec![Sound {
            event: TNT_PRIMED_SOUND,
            at: Some(fuse_at),
            volume: 1.0,
            pitch: 1.0,
        }],
    }
}

pub fn flint_use(
    player: &Player,
    world: &impl World,
    materials: &impl Materials,
    held: &ItemStack,
    mut random: impl FnMut() -> f32,
) -> Option<ItemUse> {
    if held.id != FLINT_AND_STEEL {
        return None;
    }
    let hit = aim(player, world, false)?;
    let block = world.block(hit.pos)?;
    if !player.crouching && block.id == "minecraft:tnt" {
        return Some(prime_tnt(hit.pos, WEAR, false));
    }
    let (change, at) = light_or_ignite(world, materials, block, &hit)?;
    let pitch = random() * USE_PITCH_SPREAD + USE_PITCH;
    Some(ItemUse {
        change,
        result: None,
        wear: WEAR,
        consumed: false,
        sounds: vec![Sound {
            event: USE_SOUND,
            at: Some(centre(at)),
            volume: 1.0,
            pitch,
        }],
    })
}

pub fn fire_charge_use(
    player: &Player,
    world: &impl World,
    materials: &impl Materials,
    held: &ItemStack,
    mut random: impl FnMut() -> f32,
) -> Option<ItemUse> {
    if held.id != FIRE_CHARGE {
        return None;
    }
    let hit = aim(player, world, false)?;
    let block = world.block(hit.pos)?;
    if !player.crouching && block.id == "minecraft:tnt" {
        return Some(prime_tnt(hit.pos, 0, true));
    }
    let (change, at) = light_or_ignite(world, materials, block, &hit)?;
    let pitch = (random() - random()) * FIRE_CHARGE_PITCH_SPREAD + 1.0;
    Some(ItemUse {
        change,
        result: None,
        wear: 0,
        consumed: true,
        sounds: vec![Sound {
            event: FIRE_CHARGE_SOUND,
            at: Some(centre(at)),
            volume: 1.0,
            pitch,
        }],
    })
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

    struct Rules;

    impl Materials for Rules {
        fn can_be_replaced(&self, _block: &Block) -> bool {
            false
        }

        fn sturdy_top(&self, block: &Block) -> bool {
            matches!(
                block.id.as_str(),
                "minecraft:stone" | "minecraft:tnt" | "minecraft:oak_planks"
            )
        }

        fn flammable(&self, block: &Block) -> bool {
            matches!(
                block.id.as_str(),
                "minecraft:oak_planks" | "minecraft:tnt" | "minecraft:short_grass"
            )
        }

        fn water_evaporates(&self) -> bool {
            false
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

    fn flint() -> ItemStack {
        ItemStack::new(FLINT_AND_STEEL, 1)
    }

    fn strike(player: &Player, world: &Blocks) -> Option<ItemUse> {
        flint_use(player, world, &Rules, &flint(), || 0.5)
    }

    #[test]
    fn unlit_campfires_candles_and_candle_cakes_light_up() {
        for block in [
            Block::new("minecraft:campfire").with("waterlogged", "false"),
            Block::new("minecraft:soul_campfire").with("waterlogged", "false"),
            Block::new("minecraft:candle").with("waterlogged", "false"),
            Block::new("minecraft:lime_candle").with("waterlogged", "false"),
            Block::new("minecraft:candle_cake"),
            Block::new("minecraft:black_candle_cake"),
        ] {
            let unlit = block.with("lit", "false");
            let (player, world) = looking_down(&[((0, 0, 0), unlit.clone())]);
            let used = strike(&player, &world).unwrap();
            assert_eq!(
                used.change,
                Change::Set((0, 0, 0), unlit.clone().with("lit", "true"))
            );
            assert_eq!(used.wear, 1);
            assert_eq!(used.sounds.len(), 1);
            assert_eq!(used.sounds[0].event, USE_SOUND);
            assert_eq!(used.sounds[0].at, Some([0.5, 0.5, 0.5]));
            assert!((used.sounds[0].pitch - 1.0).abs() < 1e-6, "{unlit:?}");
        }
    }

    #[test]
    fn lit_or_waterlogged_ones_do_not() {
        assert!(!can_light(
            &Block::new("minecraft:campfire")
                .with("lit", "true")
                .with("waterlogged", "false")
        ));
        assert!(!can_light(
            &Block::new("minecraft:campfire")
                .with("lit", "false")
                .with("waterlogged", "true")
        ));
        assert!(!can_light(
            &Block::new("minecraft:candle")
                .with("lit", "false")
                .with("waterlogged", "true")
        ));
        assert!(!can_light(
            &Block::new("minecraft:candle_cake").with("lit", "true")
        ));
        assert!(!can_light(
            &Block::new("minecraft:furnace").with("lit", "false")
        ));
    }

    #[test]
    fn fire_goes_on_the_clicked_face_where_it_can_burn() {
        let (player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:stone"))]);
        let used = strike(&player, &world).unwrap();
        assert_eq!(used.change, Change::Server((0, 1, 0), ItemOnBlock::Ignite));
        assert_eq!(used.sounds[0].at, Some([0.5, 1.5, 0.5]));
        let (player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:glass"))]);
        assert_eq!(strike(&player, &world), None, "glass holds no fire");
        let (player, world) = looking_down(&[
            ((0, 0, 0), Block::new("minecraft:glass")),
            ((1, 1, 0), Block::new("minecraft:oak_planks")),
        ]);
        assert!(strike(&player, &world).is_some(), "fire clings to planks");
        let (player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:soul_soil"))]);
        assert!(strike(&player, &world).is_some(), "soul fire on soul soil");
    }

    #[test]
    fn fire_goes_above_the_short_grass_it_was_aimed_at() {
        let (player, world) = looking_down(&[
            ((0, 0, 0), Block::new("minecraft:grass_block")),
            ((0, 1, 0), Block::new("minecraft:short_grass")),
        ]);
        let used = strike(&player, &world).unwrap();
        assert_eq!(used.change, Change::Server((0, 2, 0), ItemOnBlock::Ignite));
    }

    #[test]
    fn fire_needs_air() {
        let mut world = Blocks::default();
        world.set_block((0, 0, 0), Some(Block::new("minecraft:stone")));
        assert!(fire_fits(&world, &Rules, (0, 1, 0)));
        world.set_block((0, 1, 0), Some(Block::new("minecraft:fire")));
        assert!(!fire_fits(&world, &Rules, (0, 1, 0)));
        world.set_block((0, 1, 0), Some(Block::new("minecraft:cave_air")));
        assert!(fire_fits(&world, &Rules, (0, 1, 0)));
    }

    #[test]
    fn tnt_is_primed_unless_sneaking() {
        let (mut player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:tnt"))]);
        let used = strike(&player, &world).unwrap();
        assert_eq!(
            used.change,
            Change::Server((0, 0, 0), ItemOnBlock::PrimeTnt)
        );
        assert_eq!(used.wear, 1);
        assert_eq!(used.sounds[0].event, TNT_PRIMED_SOUND);
        assert_eq!(used.sounds[0].at, Some([0.5, 0.0, 0.5]));
        player.crouching = true;
        let used = strike(&player, &world).unwrap();
        assert_eq!(used.change, Change::Server((0, 1, 0), ItemOnBlock::Ignite));
    }

    #[test]
    fn only_flint_and_steel_strikes() {
        let (player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:stone"))]);
        let stick = ItemStack::new("minecraft:stick", 1);
        assert_eq!(flint_use(&player, &world, &Rules, &stick, || 0.5), None);
    }

    #[test]
    fn a_broken_tool_cracks_between_pitch_0_8_and_1_2() {
        assert_eq!(break_sound(0.0).pitch, 0.8);
        assert!((break_sound(1.0).pitch - 1.2).abs() < 1e-6);
        assert_eq!(break_sound(0.5).volume, 0.8);
    }

    fn portal_frame() -> Blocks {
        let mut world = Blocks::default();
        for x in 0..=3 {
            for y in 0..=4 {
                if x == 0 || x == 3 || y == 0 || y == 4 {
                    world.set_block((x, y, 0), Some(Block::new("minecraft:obsidian")));
                }
            }
        }
        world
    }

    #[test]
    fn fire_may_float_where_it_opens_a_portal() {
        let mut world = portal_frame();
        assert!(!fire_fits(&world, &Rules, (1, 2, 0)));
        assert!(opens_portal(&world, (1, 2, 0)));
        assert!(opens_portal(&world, (2, 3, 0)));
        assert!(!opens_portal(&world, (1, 2, 1)), "next to no frame");
        let hit = Hit {
            pos: (0, 2, 0),
            face: minecraftoss_player::Face::East,
            distance: 1.0,
            point: DVec3::new(1.0, 2.5, 0.5),
        };
        let pillar = Block::new("minecraft:obsidian");
        assert_eq!(
            light_or_ignite(&world, &Rules, pillar.clone(), &hit),
            Some((Change::Server((1, 2, 0), ItemOnBlock::Ignite), (1, 2, 0)))
        );
        world.set_block((2, 4, 0), None);
        assert!(!opens_portal(&world, (1, 2, 0)), "the top is open");
        assert_eq!(light_or_ignite(&world, &Rules, pillar.clone(), &hit), None);
    }

    fn charge(player: &Player, world: &Blocks) -> Option<ItemUse> {
        let mut rolls = [0.75, 0.25].into_iter();
        let held = ItemStack::new(FIRE_CHARGE, 16);
        fire_charge_use(player, world, &Rules, &held, move || rolls.next().unwrap())
    }

    #[test]
    fn a_fire_charge_sets_fire_and_is_used_up() {
        let (player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:stone"))]);
        let used = charge(&player, &world).unwrap();
        assert_eq!(used.change, Change::Server((0, 1, 0), ItemOnBlock::Ignite));
        assert!(used.consumed);
        assert_eq!(used.wear, 0);
        assert_eq!(used.sounds.len(), 1);
        assert_eq!(used.sounds[0].event, FIRE_CHARGE_SOUND);
        assert_eq!(used.sounds[0].at, Some([0.5, 1.5, 0.5]));
        assert_eq!(used.sounds[0].volume, 1.0);
        assert!((used.sounds[0].pitch - 1.1).abs() < 1e-6);
        let (player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:glass"))]);
        assert_eq!(charge(&player, &world), None);
        let stick = ItemStack::new("minecraft:stick", 1);
        assert_eq!(
            fire_charge_use(&player, &world, &Rules, &stick, || 0.5),
            None
        );
    }

    #[test]
    fn a_fire_charge_lights_a_campfire_and_primes_tnt_unless_sneaking() {
        let unlit = Block::new("minecraft:campfire")
            .with("waterlogged", "false")
            .with("lit", "false");
        let (player, world) = looking_down(&[((0, 0, 0), unlit.clone())]);
        let used = charge(&player, &world).unwrap();
        assert_eq!(
            used.change,
            Change::Set((0, 0, 0), unlit.with("lit", "true"))
        );
        assert_eq!(used.sounds[0].at, Some([0.5, 0.5, 0.5]));
        let (mut player, world) = looking_down(&[((0, 0, 0), Block::new("minecraft:tnt"))]);
        let used = charge(&player, &world).unwrap();
        assert_eq!(
            used.change,
            Change::Server((0, 0, 0), ItemOnBlock::PrimeTnt)
        );
        assert!(used.consumed);
        assert_eq!(used.wear, 0);
        assert_eq!(used.sounds.len(), 1);
        assert_eq!(used.sounds[0].event, TNT_PRIMED_SOUND);
        assert_eq!(used.sounds[0].at, Some([0.5, 0.0, 0.5]));
        player.crouching = true;
        let used = charge(&player, &world).unwrap();
        assert_eq!(used.change, Change::Server((0, 1, 0), ItemOnBlock::Ignite));
        assert!(used.consumed);
        assert_eq!(used.sounds[0].event, FIRE_CHARGE_SOUND);
    }
}
