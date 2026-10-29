use crate::ambient_particles::Kind;
use crate::display_ticks::{AmbientSound, DisplayTick};
use crate::scene::{Block, BlockPos, HandcraftedScene, Scene};
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const LAVA_FIZZ: i32 = 1501;
pub const BONE_MEAL_USE: i32 = 1505;
pub const BONE_MEAL_PARTICLES: i32 = 15;
pub const LECTERN_PAGE_TURN: i32 = 1043;
pub const ZOMBIE_INFECT: i32 = 1026;
pub const ZOMBIE_INFECT_SOUND: &str = "minecraft:entity.zombie.infect";
pub const ZOMBIE_VILLAGER_CONVERTED: i32 = 1027;
pub const ZOMBIE_VILLAGER_CONVERTED_SOUND: &str = "minecraft:entity.zombie_villager.converted";
pub const LAVA_EXTINGUISH: &str = "minecraft:block.lava.extinguish";
pub const SCULK_SHRIEK: i32 = 3007;
pub const SHRIEK_SOUND: &str = "minecraft:block.sculk_shrieker.shriek";
const FIZZ_SMOKE: usize = 8;
const EVAPORATION_SMOKE: usize = 8;
const SPREAD_FACTOR: i32 = 3;
const SPREAD: f64 = 3.0;
const BONEMEALABLE: [&str; 35] = [
    "AzaleaBlock",
    "BambooSaplingBlock",
    "BambooStalkBlock",
    "BigDripleafBlock",
    "BigDripleafStemBlock",
    "BonemealableFeaturePlacerBlock",
    "BushBlock",
    "CocoaBlock",
    "CropBlock",
    "FireflyBushBlock",
    "FlowerBedBlock",
    "GlowLichenBlock",
    "GrassBlock",
    "GrowingPlantBodyBlock",
    "GrowingPlantHeadBlock",
    "HangingMossBlock",
    "MangroveLeavesBlock",
    "MossyCarpetBlock",
    "MushroomBlock",
    "NetherFungusBlock",
    "NetherrackBlock",
    "NyliumBlock",
    "PitcherCropBlock",
    "RootedDirtBlock",
    "SaplingBlock",
    "SeaPickleBlock",
    "SeagrassBlock",
    "ShelfMushroomBlock",
    "ShortDryGrassBlock",
    "SmallDripleafBlock",
    "StemBlock",
    "SweetBerryBushBlock",
    "TallDryGrassBlock",
    "TallFlowerBlock",
    "TallGrassBlock",
];
const SPREADERS: [&str; 4] = [
    "GrassBlock",
    "NyliumBlock",
    "BonemealableFeaturePlacerBlock",
    "NetherrackBlock",
];
const PARTICLES_BELOW: [&str; 2] = ["MangroveLeavesBlock", "RootedDirtBlock"];

fn corner((x, y, z): BlockPos) -> DVec3 {
    DVec3::new(f64::from(x), f64::from(y), f64::from(z))
}

fn is_air(block: Option<&Block>) -> bool {
    block.is_none_or(|block| {
        block.id.namespace == "minecraft"
            && matches!(block.id.path.as_str(), "air" | "cave_air" | "void_air")
    })
}

pub fn class_test(scene: &HandcraftedScene) -> impl Fn(&Block, &str) -> bool + '_ {
    move |block, class| {
        scene.states().is_some_and(|states| {
            states.state_of(block).is_some_and(|state| {
                let blocks = &states.registries().blocks;
                blocks.block(blocks.block_of(state)).is_a(class)
            })
        })
    }
}

pub fn level_event<S: Scene + ?Sized>(
    id: i32,
    pos: BlockPos,
    data: i32,
    scene: &S,
    is_a: &dyn Fn(&Block, &str) -> bool,
    random: &mut LegacyRandom,
) -> DisplayTick {
    let mut out = DisplayTick::default();
    let base = corner(pos);
    match id {
        LAVA_FIZZ => {
            out.sounds.push(AmbientSound {
                event: LAVA_EXTINGUISH,
                at: (base + DVec3::splat(0.5)).to_array(),
                volume: 0.5,
                pitch: 2.6 + (random.next_float() - random.next_float()) * 0.8,
            });
            for _ in 0..FIZZ_SMOKE {
                let x = base.x + random.next_double();
                let z = base.z + random.next_double();
                let at = DVec3::new(x, base.y + 1.2, z);
                out.particles.push((Kind::LargeSmoke, at, DVec3::ZERO));
            }
        }
        LECTERN_PAGE_TURN => {
            out.sounds.push(AmbientSound {
                event: crate::lectern::PAGE_TURN_SOUND,
                at: (base + DVec3::splat(0.5)).to_array(),
                volume: 1.0,
                pitch: random.next_float() * 0.1 + 0.9,
            });
        }
        ZOMBIE_INFECT | ZOMBIE_VILLAGER_CONVERTED => {
            out.sounds.push(AmbientSound {
                event: if id == ZOMBIE_INFECT { ZOMBIE_INFECT_SOUND } else { ZOMBIE_VILLAGER_CONVERTED_SOUND },
                at: (base + DVec3::splat(0.5)).to_array(),
                volume: 2.0,
                pitch: (random.next_float() - random.next_float()) * 0.2 + 1.0,
            });
        }
        SCULK_SHRIEK => {
            let waterlogged = scene
                .block(pos)
                .is_some_and(|block| block.properties.get("waterlogged").is_some_and(|value| value == "true"));
            if !waterlogged {
                out.sounds.push(AmbientSound {
                    event: SHRIEK_SOUND,
                    at: crate::shriek_particles::shriek_at(pos).to_array(),
                    volume: 2.0,
                    pitch: 0.6 + random.next_float() * 0.4,
                });
            }
        }
        BONE_MEAL_USE => {
            growth_particles(scene, pos, data, is_a, random, &mut out);
            out.sounds.push(AmbientSound {
                event: crate::berries::BONE_MEAL_SOUND,
                at: (base + DVec3::splat(0.5)).to_array(),
                volume: 1.0,
                pitch: 1.0,
            });
        }
        _ => {}
    }
    out
}

fn growth_particles<S: Scene + ?Sized>(
    scene: &S,
    pos: BlockPos,
    count: i32,
    is_a: &dyn Fn(&Block, &str) -> bool,
    random: &mut LegacyRandom,
    out: &mut DisplayTick,
) {
    let Some(block) = scene.block(pos) else {
        return;
    };
    let any = |classes: &[&str]| classes.iter().any(|class| is_a(block, class));
    if any(&BONEMEALABLE) {
        let at = if any(&PARTICLES_BELOW) {
            (pos.0, pos.1 - 1, pos.2)
        } else {
            pos
        };
        if any(&SPREADERS) {
            spawn(
                scene,
                at,
                count * SPREAD_FACTOR,
                SPREAD,
                1.0,
                false,
                random,
                out,
            );
        } else {
            let height = shape_height(scene.block(at), is_a);
            spawn(scene, at, count, 0.5, height, true, random, out);
        }
    } else if block.id.namespace == "minecraft" && block.id.path == "water" {
        spawn(
            scene,
            pos,
            count * SPREAD_FACTOR,
            SPREAD,
            1.0,
            false,
            random,
            out,
        );
    }
}

pub fn shape_height(block: Option<&Block>, is_a: &dyn Fn(&Block, &str) -> bool) -> f64 {
    let Some(block) = block.filter(|block| !is_air(Some(block))) else {
        return 1.0;
    };
    let age = block
        .properties
        .get("age")
        .and_then(|age| age.parse::<i32>().ok())
        .unwrap_or(0);
    let sixteenths = if is_a(block, "CarrotBlock") || is_a(block, "PotatoBlock") {
        2 + age
    } else if is_a(block, "TorchflowerCropBlock") {
        6 + age * 4
    } else if is_a(block, "CropBlock") || is_a(block, "StemBlock") {
        2 + age * 2
    } else if is_a(block, "SweetBerryBushBlock") {
        if age == 0 { 8 } else { 16 }
    } else if is_a(block, "SaplingBlock")
        || is_a(block, "BambooSaplingBlock")
        || is_a(block, "SeagrassBlock")
    {
        12
    } else if is_a(block, "TallGrassBlock") || is_a(block, "SmallDripleafBlock") {
        13
    } else if is_a(block, "ShortDryGrassBlock") {
        10
    } else if is_a(block, "NetherFungusBlock") || is_a(block, "KelpBlock") {
        9
    } else if is_a(block, "MushroomBlock") {
        6
    } else if is_a(block, "TwistingVinesBlock") {
        15
    } else {
        16
    };
    f64::from(sixteenths) / 16.0
}

#[allow(clippy::too_many_arguments)]
fn spawn<S: Scene + ?Sized>(
    scene: &S,
    pos: BlockPos,
    count: i32,
    spread: f64,
    height: f64,
    in_air: bool,
    random: &mut LegacyRandom,
    out: &mut DisplayTick,
) {
    let base = corner(pos);
    let start = 0.5 - spread;
    for _ in 0..count {
        let motion = DVec3::new(
            random.next_gaussian() * 0.02,
            random.next_gaussian() * 0.02,
            random.next_gaussian() * 0.02,
        );
        let x = base.x + start + random.next_double() * spread * 2.0;
        let y = base.y + random.next_double() * height;
        let z = base.z + start + random.next_double() * spread * 2.0;
        let under = (x.floor() as i32, y.floor() as i32 - 1, z.floor() as i32);
        if in_air || !is_air(scene.block(under)) {
            out.particles
                .push((Kind::HappyVillager, DVec3::new(x, y, z), motion));
        }
    }
}

pub fn evaporation(pos: BlockPos, random: &mut LegacyRandom) -> DisplayTick {
    let mut out = DisplayTick::default();
    let base = corner(pos);
    for _ in 0..EVAPORATION_SMOKE {
        let x = base.x + random.next_double();
        let y = base.y + random.next_double();
        let z = base.z + random.next_double();
        out.particles
            .push((Kind::LargeSmoke, DVec3::new(x, y, z), DVec3::ZERO));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::ChunkPos;
    use std::collections::HashMap;

    struct Blocks(HashMap<BlockPos, Block>);

    impl Scene for Blocks {
        fn block(&self, pos: BlockPos) -> Option<&Block> {
            self.0.get(&pos)
        }

        fn chunks(&self) -> Vec<ChunkPos> {
            Vec::new()
        }

        fn vertical_range(&self) -> std::ops::Range<i32> {
            -64..320
        }

        fn revision(&self) -> u64 {
            0
        }
    }

    fn classes(block: &Block, class: &str) -> bool {
        let chain: &[&str] = match block.id.path.as_str() {
            "wheat" | "beetroots" => &["CropBlock"],
            "carrots" => &["CarrotBlock", "CropBlock"],
            "oak_sapling" => &["SaplingBlock"],
            "grass_block" => &["GrassBlock"],
            "rooted_dirt" => &["RootedDirtBlock"],
            "sweet_berry_bush" => &["SweetBerryBushBlock"],
            _ => &[],
        };
        chain.contains(&class)
    }

    fn no_scene() -> Blocks {
        Blocks(HashMap::new())
    }

    #[test]
    fn lava_fizzes_with_a_hiss_and_eight_puffs_over_the_block() {
        let mut random = LegacyRandom::new(1);
        let fizz = level_event(
            LAVA_FIZZ,
            (3, 10, -2),
            0,
            &no_scene(),
            &classes,
            &mut random,
        );
        assert_eq!(fizz.sounds.len(), 1);
        let hiss = &fizz.sounds[0];
        assert_eq!(
            (hiss.event, hiss.at, hiss.volume),
            (LAVA_EXTINGUISH, [3.5, 10.5, -1.5], 0.5)
        );
        assert!((1.8..3.4).contains(&hiss.pitch));
        assert_eq!(fizz.particles.len(), 8);
        for (kind, at, motion) in fizz.particles {
            assert_eq!((kind, motion), (Kind::LargeSmoke, DVec3::ZERO));
            assert_eq!(at.y, 11.2);
            assert!((3.0..4.0).contains(&at.x) && (-2.0..-1.0).contains(&at.z));
        }
    }

    #[test]
    fn a_lectern_page_turn_rustles_at_the_lectern() {
        let mut random = LegacyRandom::new(4);
        let mut mirror = LegacyRandom::new(4);
        let event = level_event(
            LECTERN_PAGE_TURN,
            (2, 70, -5),
            0,
            &no_scene(),
            &classes,
            &mut random,
        );
        assert_eq!(event.sounds.len(), 1);
        let sound = &event.sounds[0];
        assert_eq!(sound.event, "minecraft:item.book.page_turn");
        assert_eq!(sound.at, [2.5, 70.5, -4.5]);
        assert_eq!((sound.volume, sound.pitch), (1.0, mirror.next_float() * 0.1 + 0.9));
        assert!(event.particles.is_empty());
    }

    #[test]
    fn a_zombie_infecting_a_villager_groans_loudly_where_it_stands() {
        let mut random = LegacyRandom::new(9);
        let mut mirror = LegacyRandom::new(9);
        let event = level_event(
            ZOMBIE_INFECT,
            (4, 64, -2),
            0,
            &no_scene(),
            &classes,
            &mut random,
        );
        assert_eq!(event.sounds.len(), 1);
        let sound = &event.sounds[0];
        assert_eq!(sound.event, "minecraft:entity.zombie.infect");
        assert_eq!(sound.at, [4.5, 64.5, -1.5]);
        let pitch = (mirror.next_float() - mirror.next_float()) * 0.2 + 1.0;
        assert_eq!((sound.volume, sound.pitch), (2.0, pitch));
        assert!(event.particles.is_empty());
    }

    #[test]
    fn a_cured_zombie_villager_rings_out_where_it_stood() {
        let mut random = LegacyRandom::new(11);
        let mut mirror = LegacyRandom::new(11);
        let event = level_event(
            ZOMBIE_VILLAGER_CONVERTED,
            (-3, 70, 5),
            0,
            &no_scene(),
            &classes,
            &mut random,
        );
        assert_eq!(event.sounds.len(), 1);
        let sound = &event.sounds[0];
        assert_eq!(sound.event, "minecraft:entity.zombie_villager.converted");
        assert_eq!(sound.at, [-2.5, 70.5, 5.5]);
        let pitch = (mirror.next_float() - mirror.next_float()) * 0.2 + 1.0;
        assert_eq!((sound.volume, sound.pitch), (2.0, pitch));
    }

    #[test]
    fn a_shriek_screams_over_the_shrieker_unless_it_is_under_water() {
        let mut random = LegacyRandom::new(6);
        let mut mirror = LegacyRandom::new(6);
        let dry = Blocks(HashMap::from([((1, 64, 1), Block::new("minecraft:sculk_shrieker"))]));
        let event = level_event(SCULK_SHRIEK, (1, 64, 1), 0, &dry, &classes, &mut random);
        assert_eq!(event.sounds.len(), 1);
        let sound = &event.sounds[0];
        assert_eq!((sound.event, sound.at, sound.volume), (SHRIEK_SOUND, [1.5, 64.5, 1.5], 2.0));
        assert_eq!(sound.pitch, 0.6 + mirror.next_float() * 0.4);
        let wet = Blocks(HashMap::from([(
            (1, 64, 1),
            Block::new("minecraft:sculk_shrieker").with("waterlogged", "true"),
        )]));
        let event = level_event(SCULK_SHRIEK, (1, 64, 1), 0, &wet, &classes, &mut random);
        assert!(event.sounds.is_empty());
    }

    #[test]
    fn other_level_events_do_nothing_yet() {
        let mut random = LegacyRandom::new(2);
        let event = level_event(1009, (0, 0, 0), 0, &no_scene(), &classes, &mut random);
        assert_eq!(event, DisplayTick::default());
    }

    #[test]
    fn bone_meal_on_a_crop_sparkles_fifteen_times_within_its_height() {
        let wheat = Block::new("minecraft:wheat").with("age", "2");
        let scene = Blocks(HashMap::from([((0, 64, 0), wheat)]));
        let mut random = LegacyRandom::new(3);
        let event = level_event(BONE_MEAL_USE, (0, 64, 0), 15, &scene, &classes, &mut random);
        assert_eq!(event.sounds.len(), 1);
        assert_eq!(event.sounds[0].event, crate::berries::BONE_MEAL_SOUND);
        assert_eq!(event.sounds[0].at, [0.5, 64.5, 0.5]);
        assert_eq!(event.particles.len(), 15);
        for (kind, at, motion) in event.particles {
            assert_eq!(kind, Kind::HappyVillager);
            assert!((0.0..1.0).contains(&at.x) && (0.0..1.0).contains(&at.z));
            assert!((64.0..64.0 + 6.0 / 16.0).contains(&at.y), "{at}");
            assert!(motion.length() < 0.2);
        }
    }

    #[test]
    fn bone_meal_on_grass_spreads_forty_five_tries_over_the_ground_around() {
        let mut blocks = HashMap::new();
        for x in -3..=3 {
            for z in -3..=3 {
                blocks.insert((x, 63, z), Block::new("minecraft:dirt"));
            }
        }
        blocks.insert((0, 64, 0), Block::new("minecraft:grass_block"));
        let scene = Blocks(blocks);
        let mut random = LegacyRandom::new(4);
        let event = level_event(BONE_MEAL_USE, (0, 64, 0), 15, &scene, &classes, &mut random);
        assert_eq!(event.particles.len(), 45);
        for (_, at, _) in &event.particles {
            assert!((-2.5..3.5).contains(&at.x) && (-2.5..3.5).contains(&at.z));
            assert!((64.0..65.0).contains(&at.y));
        }
        let floating = Blocks(HashMap::from([(
            (0, 64, 0),
            Block::new("minecraft:grass_block"),
        )]));
        let event = level_event(
            BONE_MEAL_USE,
            (0, 64, 0),
            15,
            &floating,
            &classes,
            &mut random,
        );
        assert!(event.particles.len() < 10, "{}", event.particles.len());
    }

    #[test]
    fn rooted_dirt_sparkles_below_itself_and_unbonemealable_blocks_only_sound() {
        let scene = Blocks(HashMap::from([
            ((0, 64, 0), Block::new("minecraft:rooted_dirt")),
            ((5, 64, 0), Block::new("minecraft:stone")),
        ]));
        let mut random = LegacyRandom::new(5);
        let roots = level_event(BONE_MEAL_USE, (0, 64, 0), 15, &scene, &classes, &mut random);
        assert_eq!(roots.particles.len(), 15);
        assert!(
            roots
                .particles
                .iter()
                .all(|(_, at, _)| (63.0..64.0).contains(&at.y))
        );
        let stone = level_event(BONE_MEAL_USE, (5, 64, 0), 15, &scene, &classes, &mut random);
        assert!(stone.particles.is_empty());
        assert_eq!(stone.sounds.len(), 1);
    }

    #[test]
    fn plant_heights_follow_the_vanilla_shapes() {
        let height =
            |id: &str, age: &str| shape_height(Some(&Block::new(id).with("age", age)), &classes);
        assert_eq!(height("minecraft:wheat", "7"), 1.0);
        assert_eq!(height("minecraft:wheat", "0"), 2.0 / 16.0);
        assert_eq!(height("minecraft:carrots", "3"), 5.0 / 16.0);
        assert_eq!(height("minecraft:oak_sapling", "0"), 12.0 / 16.0);
        assert_eq!(height("minecraft:sweet_berry_bush", "0"), 0.5);
        assert_eq!(height("minecraft:sweet_berry_bush", "1"), 1.0);
        assert_eq!(height("minecraft:stone", "0"), 1.0);
        assert_eq!(shape_height(None, &classes), 1.0);
    }

    #[test]
    fn evaporating_water_puffs_eight_smokes_inside_the_block() {
        let mut random = LegacyRandom::new(3);
        let steam = evaporation((0, 40, 0), &mut random);
        assert!(steam.sounds.is_empty());
        assert_eq!(steam.particles.len(), 8);
        for (kind, at, _) in steam.particles {
            assert_eq!(kind, Kind::LargeSmoke);
            assert!((0.0..1.0).contains(&at.x) && (40.0..41.0).contains(&at.y));
        }
    }
}
