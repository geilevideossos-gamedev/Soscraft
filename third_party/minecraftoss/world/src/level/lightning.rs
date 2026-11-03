use super::burning;
use super::entity::{Entity, EntityKind, LightningData};
use super::physics::Aabb;
use super::redstone::Kind;
use super::{Level, update};
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::random::RandomSource;
use minecraftoss_core::{BlockPos, BlockStateId, ChunkPos};
use minecraftoss_generator::feature::Ctx;
use minecraftoss_generator::feature::update::base_fire_state;

pub const ELECTRIC_SPARK: i32 = 3002;
pub const DAMAGE: f32 = 5.0;
pub const DAMAGE_TYPE: &str = "minecraft:lightning_bolt";
const IGNITE_TICKS: i32 = 160;
const REACH: f64 = 3.0;
const HEIGHT: f64 = 6.0;
const ROD_REACH: i32 = 128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightningHit {
    pub bolt: i32,
    pub uuid: [i32; 4],
    pub at: [f64; 3],
}

impl LightningHit {
    pub fn bounds(&self) -> ([f64; 3], [f64; 3]) {
        let [x, y, z] = self.at;
        (
            [x - REACH, y - REACH, z - REACH],
            [x + REACH, y + HEIGHT + REACH, z + REACH],
        )
    }

    pub fn touches(&self, min: [f64; 3], max: [f64; 3]) -> bool {
        let (low, high) = self.bounds();
        (0..3).all(|axis| min[axis] < high[axis] && max[axis] > low[axis])
    }
}

pub fn thunder_hit(e: &mut Entity) {
    e.remaining_fire_ticks += 1;
    if e.remaining_fire_ticks == 0 {
        e.remaining_fire_ticks = IGNITE_TICKS;
    }
    let health = match &mut e.kind {
        EntityKind::Item(data) if !burning::resists(&data.stack, DAMAGE_TYPE) => &mut data.health,
        EntityKind::ExperienceOrb(data) => &mut data.health,
        _ => return,
    };
    *health = (*health as f32 - DAMAGE) as i32;
    if *health <= 0 {
        e.removed = true;
    }
}

fn block_of(pos: [f64; 3]) -> BlockPos {
    BlockPos::new(
        pos[0].floor() as i32,
        pos[1].floor() as i32,
        pos[2].floor() as i32,
    )
}

impl Level<'_> {
    pub fn add_lightning(&mut self, pos: [f64; 3], visual_only: bool) -> i32 {
        let uuid = crate::natural_spawner::SpawnLevel::next_uuid(self);
        let seed = self.entity_random.next_i64();
        let flashes = self.entity_random.next_i32_bound(3) + 1;
        let data = LightningData {
            life: 2,
            flashes,
            visual_only,
            seed,
            uuid,
        };
        self.add_entity(Entity::new(
            0,
            EntityKind::LightningBolt(data),
            pos,
            0.0,
            0.0,
        ))
    }

    pub fn take_lightning_hits(&mut self) -> Vec<LightningHit> {
        std::mem::take(&mut self.lightning_hits)
    }

    pub fn lightning_rod_near(&self, center: BlockPos) -> Option<BlockPos> {
        let reach = i64::from(ROD_REACH) * i64::from(ROD_REACH);
        let mut nearest: Option<(i64, BlockPos)> = None;
        for cx in (center.x - ROD_REACH) >> 4..=(center.x + ROD_REACH) >> 4 {
            for cz in (center.z - ROD_REACH) >> 4..=(center.z + ROD_REACH) >> 4 {
                let Some(chunk) = self.chunk(ChunkPos::new(cx, cz)) else {
                    continue;
                };
                for (lx, lz) in (0..16).flat_map(|lx| (0..16).map(move |lz| (lx, lz))) {
                    let y = chunk.heightmaps.get(HeightmapKind::WorldSurface, lx, lz) - 1;
                    let pos = BlockPos::new(cx * 16 + lx as i32, y, cz * 16 + lz as i32);
                    let d = [pos.x - center.x, pos.y - center.y, pos.z - center.z].map(i64::from);
                    let distance = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
                    let closer =
                        distance <= reach && nearest.is_none_or(|(best, _)| distance < best);
                    let rod =
                        self.redstone_kind(chunk.block(lx, y, lz)) == Some(Kind::LightningRod);
                    if closer && rod {
                        nearest = Some((distance, pos));
                    }
                }
            }
        }
        nearest.map(|(_, pos)| pos)
    }

    pub(super) fn lightning_fire(&mut self, e: &Entity, tries: i32) {
        let EntityKind::LightningBolt(data) = &e.kind else {
            return;
        };
        let pos = block_of(e.pos);
        if data.visual_only || !self.can_spread_fire_around(pos) {
            return;
        }
        self.light_by_lightning(pos);
        for _ in 0..tries {
            let x = self.entity_random.next_i32_bound(3) - 1;
            let y = self.entity_random.next_i32_bound(3) - 1;
            let z = self.entity_random.next_i32_bound(3) - 1;
            self.light_by_lightning(pos.offset(x, y, z));
        }
    }

    fn light_by_lightning(&mut self, pos: BlockPos) {
        let lib = self.lib;
        let ctx: Ctx = Ctx { lib, region: self };
        let fire = base_fire_state(&ctx, pos);
        if lib.registries.blocks.is_air(self.block(pos)) && self.fire_can_survive(fire, pos) {
            self.set_block(pos, fire, update::ALL, update::LIMIT);
        }
    }

    fn fire_can_survive(&self, fire: BlockStateId, pos: BlockPos) -> bool {
        if self.name(fire) == "minecraft:fire" {
            self.fire_survives(pos)
        } else {
            let soul_base = self.lib.update_rules.soul_fire_base;
            self.lib
                .registries
                .block_in_tag(self.block(pos.below()), soul_base)
        }
    }

    pub(super) fn lightning_strike_entities(&mut self, e: &Entity) {
        let EntityKind::LightningBolt(data) = &e.kind else {
            return;
        };
        if data.life < 0 || data.visual_only {
            return;
        }
        let hit = LightningHit {
            bolt: e.id,
            uuid: data.uuid,
            at: e.pos,
        };
        let (min, max) = hit.bounds();
        let area = Aabb::new(min[0], min[1], min[2], max[0], max[1], max[2]);
        for other in self.entities.iter_mut() {
            if other.id != e.id && !other.removed && other.bb.intersects(&area) {
                thunder_hit(other);
            }
        }
        self.lightning_hits.push(hit);
    }
}

#[cfg(test)]
mod tests {
    use super::super::container::Stack;
    use super::*;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_core::{Chunk, ChunkPos, Registries};
    use minecraftoss_generator::feature::Library;

    const SPOT: [f64; 3] = [0.5, 65.0, 0.5];

    fn library() -> Option<Library> {
        let paths = DataPaths::discover().ok()?;
        let registries = std::sync::Arc::new(Registries::load(&paths).ok()?);
        let bounds = minecraftoss_generator::providers::GenContext {
            min_y: -64,
            depth: 384,
            sea_level: 63,
        };
        Library::load(registries, bounds).ok()
    }

    fn level(lib: &Library) -> Level<'_> {
        let mut level = Level::new(lib, -64, 384);
        let stone = lib
            .registries
            .blocks
            .parse_state("minecraft:stone")
            .unwrap();
        for cx in -1..=1 {
            for cz in -1..=1 {
                let mut chunk = Chunk::new(
                    ChunkPos::new(cx, cz),
                    -64,
                    384,
                    minecraftoss_core::BiomeId(0),
                );
                chunk.status = minecraftoss_core::chunk::ChunkStatus::Full;
                for x in 0..16 {
                    for z in 0..16 {
                        chunk.set_block_raw(x, 64, z, stone, &lib.registries);
                    }
                }
                level.insert_chunk(chunk);
            }
        }
        level.players = vec![SPOT];
        level
    }

    fn place(level: &mut Level<'_>, pos: BlockPos, state: &str) {
        let state = level.registries().blocks.parse_state(state).unwrap();
        level.set_block(pos, state, update::ALL, update::LIMIT);
    }

    fn named(level: &Level<'_>, pos: BlockPos) -> String {
        level.name(level.block(pos)).to_owned()
    }

    fn fires(level: &Level<'_>) -> Vec<BlockPos> {
        let mut out = Vec::new();
        for x in -2..=2 {
            for y in 63..=67 {
                for z in -2..=2 {
                    let pos = BlockPos::new(x, y, z);
                    if named(level, pos) == "minecraft:fire" {
                        out.push(pos);
                    }
                }
            }
        }
        out
    }

    fn bolt_alive(level: &Level<'_>, id: i32) -> bool {
        level.entities.iter().any(|e| e.id == id && !e.removed)
    }

    fn run_out(level: &mut Level<'_>, id: i32) -> usize {
        let mut ticks = 0;
        while bolt_alive(level, id) {
            level.tick_entities();
            ticks += 1;
            assert!(ticks < 100, "a bolt dies within a few dozen ticks");
        }
        ticks
    }

    #[test]
    fn a_bolt_on_normal_lights_its_block_and_up_to_four_more() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        level.add_lightning(SPOT, false);
        level.tick_entities();
        let lit = fires(&level);
        assert!(lit.contains(&BlockPos::new(0, 65, 0)), "{lit:?}");
        assert!(lit.len() <= 5, "{lit:?}");
        assert!(
            lit.iter()
                .all(|pos| pos.y == 65 && pos.x.abs() <= 1 && pos.z.abs() <= 1)
        );

        let mut easy = self::level(&lib);
        easy.difficulty = 1;
        easy.add_lightning(SPOT, false);
        easy.tick_entities();
        assert!(fires(&easy).is_empty(), "no first burst on Easy");

        let mut visual = self::level(&lib);
        let id = visual.add_lightning(SPOT, true);
        run_out(&mut visual, id);
        assert!(fires(&visual).is_empty(), "a visual bolt lights nothing");

        let mut lonely = self::level(&lib);
        lonely.players = vec![[1000.5, 65.0, 0.5]];
        let id = lonely.add_lightning(SPOT, false);
        run_out(&mut lonely, id);
        assert!(
            fires(&lonely).is_empty(),
            "no fire beyond the fire spread radius"
        );
    }

    #[test]
    fn a_reflash_lights_only_its_own_block() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        level.difficulty = 1;
        let id = level.add_lightning(SPOT, false);
        run_out(&mut level, id);
        assert_eq!(fires(&level), vec![BlockPos::new(0, 65, 0)]);
    }

    #[test]
    fn a_bolt_burns_items_and_orbs_inside_its_box_only() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        let near = level.spawn_item_entity(
            [1.5, 65.0, 1.5],
            Stack::new("minecraft:oak_planks", 64),
            [0.0; 3],
        );
        let netherite = level.spawn_item_entity(
            [-1.5, 65.0, 0.5],
            Stack::new("minecraft:netherite_ingot", 1),
            [0.0; 3],
        );
        let far = level.spawn_item_entity(
            [6.5, 65.0, 0.5],
            Stack::new("minecraft:oak_planks", 64),
            [0.0; 3],
        );
        let orb = level.add_entity(Entity::experience_orb([0.5, 66.0, -1.5], 7));
        level.add_lightning(SPOT, false);
        level.tick_entities();
        let alive = |level: &Level<'_>, id: i32| level.entities.iter().any(|e| e.id == id);
        assert!(!alive(&level, near), "planks inside the box burn");
        assert!(!alive(&level, netherite), "netherite resists only fire");
        assert!(!alive(&level, orb), "the orb is struck too");
        assert!(alive(&level, far), "planks six blocks away stay");
    }

    #[test]
    fn a_bolt_reports_a_hit_each_lit_tick() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        let id = level.add_lightning(SPOT, false);
        let Some(EntityKind::LightningBolt(data)) = level
            .entities
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.kind.clone())
        else {
            panic!("the bolt is added");
        };
        let mut hits = Vec::new();
        while bolt_alive(&level, id) {
            level.tick_entities();
            let lit = level.entities.iter().any(|e| {
                e.id == id && matches!(&e.kind, EntityKind::LightningBolt(bolt) if bolt.life >= 0)
            });
            let taken = level.take_lightning_hits();
            assert_eq!(taken.len(), usize::from(lit));
            hits.extend(taken);
        }
        assert_eq!(hits.len() as i32, 2 * (data.flashes + 1));
        assert!(
            hits.iter()
                .all(|hit| hit.bolt == id && hit.uuid == data.uuid && hit.at == SPOT)
        );
        assert!(hits[0].touches([2.0, 70.0, 2.0], [2.5, 71.0, 2.5]));
        assert!(!hits[0].touches([3.6, 65.0, 0.0], [4.0, 66.0, 0.5]));

        let visual = level.add_lightning(SPOT, true);
        run_out(&mut level, visual);
        assert!(
            level.take_lightning_hits().is_empty(),
            "a visual bolt hurts nothing"
        );
    }

    #[test]
    fn a_struck_rod_powers_and_sparks() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        let rod = BlockPos::new(0, 65, 0);
        place(&mut level, rod, "minecraft:lightning_rod");
        for pos in [
            BlockPos::new(1, 65, 0),
            BlockPos::new(-1, 65, 0),
            BlockPos::new(0, 65, 1),
            BlockPos::new(0, 65, -1),
            BlockPos::new(0, 64, 0),
        ] {
            place(&mut level, pos, "minecraft:weathered_copper");
        }
        level.level_events.clear();
        level.add_lightning([0.5, 66.0, 0.5], false);
        level.tick_entities();
        assert_eq!(
            level
                .registries()
                .blocks
                .property(level.block(rod), "powered"),
            Some("true")
        );
        assert!(
            level.level_events.contains(&(ELECTRIC_SPARK, rod, 1)),
            "{:?}",
            level.level_events
        );
        let cleaned: Vec<BlockPos> = level
            .level_events
            .iter()
            .filter(|&&(event, _, data)| event == ELECTRIC_SPARK && data == -1)
            .map(|&(_, pos, _)| pos)
            .collect();
        assert!(
            cleaned
                .iter()
                .any(|&pos| named(&level, pos).contains("copper")),
            "the copper walk sparks on the copper it cleans: {cleaned:?}"
        );
        for pos in cleaned {
            let name = named(&level, pos);
            assert!(!name.contains("weathered"), "{name}");
        }
    }

    fn spawning_draws(lib: &Library, rain: f32, thunder: f32) -> Option<(bool, i64)> {
        let mut level = level(lib);
        level.set_dimension("minecraft:overworld").ok()?;
        let sky = level.sky.as_mut()?;
        sky.weather.rain_level = rain;
        sky.weather.thunder_level = thunder;
        let spawns = crate::natural_spawner::CreatureSpawns::load(
            lib.registries.clone(),
            "minecraft:overworld",
            false,
        )
        .ok()?;
        let mut natural = super::super::spawning::NaturalSpawning::new(std::sync::Arc::new(spawns));
        natural.spawn_mobs = false;
        natural.set_players(&[crate::natural_spawner::tick::SpawnPlayer {
            pos: SPOT,
            spectator: false,
            alive: true,
        }]);
        natural.simulation = level.chunks().map(|chunk| chunk.pos).collect();
        level.natural_spawning = Some(natural);
        level.tick_natural_spawning();
        Some((level.is_thundering(), level.random.next_i64()))
    }

    #[test]
    fn thundering_needs_rain_too() {
        let Some(lib) = library() else { return };
        let Some((false, clear)) = spawning_draws(&lib, 0.0, 0.0) else {
            return;
        };
        assert_eq!(
            spawning_draws(&lib, 0.5, 1.0),
            Some((false, clear)),
            "full thunder in half rain is no storm: no lightning roll"
        );
        assert_eq!(spawning_draws(&lib, 1.0, 0.85), Some((false, clear)));
        let (thundering, storm) = spawning_draws(&lib, 1.0, 1.0).unwrap();
        assert!(thundering);
        assert_ne!(storm, clear, "a storm rolls in every spawning chunk");
    }

    #[test]
    fn a_bolt_in_an_empty_obsidian_frame_lights_the_portal() {
        let Some(lib) = library() else { return };
        let mut level = level(&lib);
        level.dimension = Some("minecraft:overworld".to_owned());
        for x in 0..=3 {
            place(&mut level, BlockPos::new(x, 65, 0), "minecraft:obsidian");
            place(&mut level, BlockPos::new(x, 69, 0), "minecraft:obsidian");
        }
        for y in 66..=68 {
            place(&mut level, BlockPos::new(0, y, 0), "minecraft:obsidian");
            place(&mut level, BlockPos::new(3, y, 0), "minecraft:obsidian");
        }
        level.add_lightning([1.5, 66.0, 0.5], false);
        level.tick_entities();
        for pos in [
            BlockPos::new(1, 66, 0),
            BlockPos::new(2, 66, 0),
            BlockPos::new(1, 68, 0),
            BlockPos::new(2, 68, 0),
        ] {
            assert_eq!(named(&level, pos), "minecraft:nether_portal", "{pos:?}");
        }
    }
}
