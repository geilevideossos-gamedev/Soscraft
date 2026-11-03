use super::FamilySpawn;
use crate::natural_spawner::spawning::{DifficultyInstance, MobCategory, SpawnContext};
use crate::natural_spawner::{
    CreatureSpawns, Placement, Rules, SpawnLevel, TypeInfo, Walk, set, type_info,
};
use minecraftoss_core::BlockPos;
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::nbt::Tag;

pub(super) const PHANTOM: &str = "minecraft:phantom";
pub(super) const ANCHOR_UP: i32 = 5;
const DARK_SKY: i32 = 5;
const OPEN_SKY: i32 = 15;
const SLEEPLESS: i32 = 72000;

pub(super) const ROWS: &[FamilySpawn] = &[FamilySpawn {
    kind: PHANTOM,
    info: TypeInfo {
        width: 0.9,
        height: 0.5,
        placement: Placement::NoRestrictions,
        heightmap: HeightmapKind::MotionBlockingNoLeaves,
        fire_immune: false,
        rules: Rules::Family { creature: false },
        walk: Walk::Neutral,
        remove_far: true,
        cluster: 4,
    },
    category: MobCategory::Monster,
    follow_range: 16.0,
    baby_chance: None,
    rule: Some(phantom_rule),
    finalize: Some(phantom_finalize),
}];

fn phantom_rule(
    spawns: &CreatureSpawns,
    level: &mut dyn SpawnLevel,
    _context: &SpawnContext,
    kind: &str,
    info: &TypeInfo,
    pos: BlockPos,
    spawner: bool,
) -> bool {
    spawner || spawns.mob_spawn_rules(level, kind, info, pos)
}

fn phantom_finalize(
    _spawns: &CreatureSpawns,
    _level: &mut dyn SpawnLevel,
    _context: Option<(&SpawnContext, DifficultyInstance)>,
    tag: &mut Tag,
    pos: BlockPos,
    _data: &mut Option<i32>,
) {
    let anchor = vec![pos.x, pos.y + ANCHOR_UP, pos.z];
    set(tag, "anchor_pos", Tag::IntArray(anchor));
    set(tag, "size", Tag::Int(0));
}

impl CreatureSpawns {
    pub fn spawn_phantoms(
        &self,
        level: &mut dyn SpawnLevel,
        context: &mut SpawnContext,
        players: &[(BlockPos, bool, i32)],
        sky_light: bool,
    ) -> Vec<Tag> {
        let mut phantoms = Vec::new();
        let Some(info) = type_info(PHANTOM) else {
            return phantoms;
        };
        if level.sky_darken() < DARK_SKY && sky_light {
            return phantoms;
        }
        for &(feet, spectator, rest) in players {
            if spectator {
                continue;
            }
            if sky_light && (feet.y < level.sea_level() || level.sky_brightness(feet) < OPEN_SKY) {
                continue;
            }
            let difficulty = Self::difficulty_at(level, context, feet);
            if difficulty.effective <= level.random().next_f32() * 3.0 {
                continue;
            }
            if level.random().next_i32_bound(rest.max(1)) < SLEEPLESS {
                continue;
            }
            let up = 20 + level.random().next_i32_bound(15);
            let east = -10 + level.random().next_i32_bound(21);
            let south = -10 + level.random().next_i32_bound(21);
            let spot = BlockPos::new(feet.x + east, feet.y + up, feet.z + south);
            let state = level.block(spot);
            if !self.is_valid_empty_spawn_block(&self.registries, state, PHANTOM, &info) {
                continue;
            }
            let count = 1 + level.random().next_i32_bound(difficulty.base + 1);
            let at = [
                f64::from(spot.x) + 0.5,
                f64::from(spot.y),
                f64::from(spot.z) + 0.5,
            ];
            for _ in 0..count {
                if let Ok(phantom) = self.summon(level, context, PHANTOM, at, None, 0.0) {
                    phantoms.push(phantom);
                }
            }
        }
        phantoms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::natural_spawner::{StructureHere, type_info};
    use minecraftoss_core::random::{LegacyRandom, RandomSource};
    use minecraftoss_core::{BiomeId, BlockStateId, ChunkPos};

    struct Field {
        floor: BlockStateId,
        biome: BiomeId,
        random: LegacyRandom,
        made: i32,
        notes: Vec<String>,
        darken: i32,
        sky: i32,
        sea: i32,
        high: Option<(i32, BlockStateId)>,
    }

    impl SpawnLevel for Field {
        fn block(&self, pos: BlockPos) -> BlockStateId {
            match self.high {
                _ if pos.y <= 63 => self.floor,
                Some((from, state)) if pos.y >= from => state,
                _ => BlockStateId::AIR,
            }
        }
        fn height(&self, _kind: HeightmapKind, _x: i32, _z: i32) -> i32 {
            64
        }
        fn biome(&self, _pos: BlockPos) -> BiomeId {
            self.biome
        }
        fn noise_biome(&self, _qx: i32, _qy: i32, _qz: i32) -> BiomeId {
            self.biome
        }
        fn raw_brightness(&mut self, pos: BlockPos, darkening: i32) -> i32 {
            (self.sky_brightness(pos) - darkening).max(0)
        }
        fn sky_brightness(&mut self, pos: BlockPos) -> i32 {
            if pos.y <= 63 { 0 } else { self.sky }
        }
        fn block_brightness(&mut self, _pos: BlockPos) -> i32 {
            0
        }
        fn sky_darken(&self) -> i32 {
            self.darken
        }
        fn sea_level(&self) -> i32 {
            self.sea
        }
        fn min_y(&self) -> i32 {
            -64
        }
        fn random(&mut self) -> &mut dyn RandomSource {
            &mut self.random
        }
        fn next_uuid(&mut self) -> [i32; 4] {
            self.made += 1;
            [self.made, 0, 0, 1]
        }
        fn add_entity(&mut self, _entity: Tag) {}
        fn note_unsupported(&mut self, what: &str) {
            self.notes.push(what.to_owned());
        }
    }

    fn no_slimes(_: BiomeId) -> f32 {
        0.0
    }

    fn any_chunk(_: ChunkPos) -> bool {
        true
    }

    fn no_structures(_: BlockPos) -> Vec<StructureHere> {
        Vec::new()
    }

    fn context(players: &[[f64; 3]]) -> SpawnContext<'_> {
        SpawnContext {
            players,
            respawn: None,
            difficulty: 2,
            overworld_time: 0,
            moon_brightness: 0.0,
            raining: false,
            thundering: false,
            seed: 0,
            surface_slime_chance: &no_slimes,
            halloween: false,
            can_spawn_in_chunk: &any_chunk,
            obstacles: Vec::new(),
            chickens: Vec::new(),
            structures: &no_structures,
            spawning_monsters: true,
        }
    }

    fn setup(floor: &str) -> Option<(CreatureSpawns, Field)> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        registries.entities.as_ref()?;
        let biome = registries.biomes.id("minecraft:plains")?;
        let floor = registries.blocks.parse_state(floor).ok()?;
        let registries = std::sync::Arc::new(registries);
        let spawns = CreatureSpawns::load(registries, "minecraft:overworld", false).ok()?;
        let field = Field {
            floor,
            biome,
            random: LegacyRandom::new(9),
            made: 0,
            notes: Vec::new(),
            darken: 0,
            sky: 15,
            sea: 63,
            high: None,
        };
        Some((spawns, field))
    }

    fn int(tag: &Tag, key: &str) -> Option<i64> {
        tag.get(key).and_then(Tag::as_i64)
    }

    #[test]
    fn a_phantom_row_spawns_from_spawners_and_finalizes_its_anchor_five_up() {
        let info = type_info(PHANTOM).unwrap();
        assert_eq!(
            (info.width, info.height, info.placement, info.fire_immune),
            (0.9, 0.5, Placement::NoRestrictions, false)
        );
        assert!(matches!(
            info.heightmap,
            HeightmapKind::MotionBlockingNoLeaves
        ));
        assert_eq!(
            (info.rules, info.walk, info.remove_far, info.cluster),
            (Rules::Family { creature: false }, Walk::Neutral, true, 4)
        );
        assert_eq!(MobCategory::of_type(PHANTOM), Some(MobCategory::Monster));
        let row = super::super::row(PHANTOM).unwrap();
        assert_eq!((row.follow_range, row.baby_chance), (16.0, None));
        let Some((spawns, mut level)) = setup("minecraft:grass_block") else {
            return;
        };
        let pos = BlockPos::new(0, 64, 0);
        let players: [[f64; 3]; 0] = [];
        let rule = |spawns: &CreatureSpawns, level: &mut Field, spawner: bool| {
            let context = context(&players);
            super::super::rule(spawns, level, &context, PHANTOM, &info, pos, spawner)
        };
        assert!(rule(&spawns, &mut level, false), "on grass");
        assert!(rule(&spawns, &mut level, true), "from a spawner");
        let (glass_spawns, mut glass) = setup("minecraft:glass").unwrap();
        assert!(!rule(&glass_spawns, &mut glass, false), "not on glass");
        assert!(rule(&glass_spawns, &mut glass, true), "a spawner anywhere");
        let at = [3.5, 70.0, -2.5];
        let tag = spawns
            .summon(&mut level, &mut context(&players), PHANTOM, at, None, 0.0)
            .unwrap();
        assert_eq!(
            tag.get("anchor_pos").and_then(Tag::as_ints),
            Some(vec![3, 75, -3])
        );
        assert_eq!(int(&tag, "size"), Some(0));
        assert!(level.notes.is_empty(), "{:?}", level.notes);
        let sized = minecraftoss_core::snbt::parse_compound("{size:6}").unwrap();
        let tag = spawns
            .summon(
                &mut level,
                &mut context(&players),
                PHANTOM,
                at,
                Some(&sized),
                0.0,
            )
            .unwrap();
        assert_eq!(int(&tag, "size"), Some(6));
        assert_eq!(
            tag.get("anchor_pos"),
            None,
            "a summon with a tag is not finalized"
        );
    }

    fn looks(
        spawns: &CreatureSpawns,
        level: &mut Field,
        players: &[(BlockPos, bool, i32)],
        difficulty: i32,
    ) -> Vec<Vec<Tag>> {
        (0..400)
            .map(|_| {
                let mut context = context(&[]);
                context.difficulty = difficulty;
                spawns.spawn_phantoms(level, &mut context, players, true)
            })
            .collect()
    }

    fn none(looks: &[Vec<Tag>]) -> bool {
        looks.iter().all(Vec::is_empty)
    }

    fn position(tag: &Tag) -> [f64; 3] {
        let pos = tag.get("Pos").and_then(Tag::as_list).unwrap();
        [0, 1, 2].map(|i| pos[i].as_f64().unwrap())
    }

    #[test]
    fn phantoms_come_for_the_sleepless_at_night_under_open_sky() {
        let Some((spawns, mut level)) = setup("minecraft:grass_block") else {
            return;
        };
        let feet = BlockPos::new(4, 64, -7);
        let awake = 1_000_000;
        let sleepless = [(feet, false, awake)];
        assert!(none(&looks(&spawns, &mut level, &sleepless, 2)), "by day");
        level.darken = DARK_SKY - 1;
        assert!(none(&looks(&spawns, &mut level, &sleepless, 2)), "at dusk");
        level.darken = 11;
        let normal = looks(&spawns, &mut level, &sleepless, 2);
        let came = normal.iter().filter(|look| !look.is_empty()).count();
        assert!((140..=240).contains(&came), "about half on Normal: {came}");
        let hard = looks(&spawns, &mut level, &sleepless, 3);
        let came = hard.iter().filter(|look| !look.is_empty()).count();
        assert!((240..=320).contains(&came), "most on Hard: {came}");
        for (group, most) in [(&normal, 3), (&hard, 4)] {
            let mut sizes = std::collections::BTreeSet::new();
            for look in group.iter().filter(|look| !look.is_empty()) {
                sizes.insert(look.len());
                let at = position(&look[0]);
                let spot = at.map(|v| v.floor() as i32);
                assert_eq!(at.map(|v| v - v.floor()), [0.5, 0.0, 0.5], "{at:?}");
                assert!((20..=34).contains(&(spot[1] - feet.y)), "{at:?}");
                assert!((spot[0] - feet.x).abs() <= 10 && (spot[2] - feet.z).abs() <= 10);
                let mut uuids = std::collections::BTreeSet::new();
                for phantom in look {
                    assert_eq!(position(phantom), at, "one spot for the whole group");
                    assert_eq!(
                        phantom.get("anchor_pos").and_then(Tag::as_ints),
                        Some(vec![spot[0], spot[1] + ANCHOR_UP, spot[2]])
                    );
                    assert_eq!(int(phantom, "size"), Some(0));
                    uuids.insert(format!("{:?}", phantom.get("UUID")));
                }
                assert_eq!(uuids.len(), look.len());
            }
            let expected: std::collections::BTreeSet<usize> = (1..=most).collect();
            assert_eq!(sizes, expected, "1 to difficulty + 1");
        }
        for (rest, why) in [(SLEEPLESS, "three days awake"), (0, "rested")] {
            let players = [(feet, false, rest)];
            assert!(none(&looks(&spawns, &mut level, &players, 3)), "{why}");
        }
        let watching = [(feet, true, awake)];
        assert!(none(&looks(&spawns, &mut level, &watching, 3)), "spectator");
        assert!(none(&looks(&spawns, &mut level, &sleepless, 0)), "peaceful");
        level.sky = OPEN_SKY - 1;
        assert!(none(&looks(&spawns, &mut level, &sleepless, 3)), "a roof");
        level.sky = OPEN_SKY;
        level.sea = feet.y + 1;
        let below = none(&looks(&spawns, &mut level, &sleepless, 3));
        assert!(below, "below sea level");
        level.sea = 63;
        let blocks = &spawns.registries.blocks;
        for spot in ["minecraft:stone", "minecraft:water"] {
            level.high = Some((feet.y + 20, blocks.parse_state(spot).unwrap()));
            assert!(none(&looks(&spawns, &mut level, &sleepless, 3)), "{spot}");
        }
        level.high = None;
        let far = BlockPos::new(200, 64, 200);
        let two = [(feet, false, 0), (far, false, awake)];
        let both = looks(&spawns, &mut level, &two, 3);
        assert!(!none(&both));
        for phantom in both.iter().flatten() {
            let at = position(phantom);
            assert!((at[0] - 200.5).abs() <= 10.0, "only the sleepless: {at:?}");
        }
        assert!(level.notes.is_empty(), "{:?}", level.notes);
    }
}
