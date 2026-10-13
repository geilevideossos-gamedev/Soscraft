use crate::ambient_particles::{AmbientParticles, Kind};
use crate::client_mobs::{ClientMobs, server_mobs};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::scene::{BlockPos, Scene};
use glam::{DVec3, Quat, Vec3};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_player::rng::LegacyRandom;
use std::collections::BTreeMap;

pub const VIEW_DISTANCE: f64 = 64.0;
const SPAWNER: &str = "minecraft:mob_spawner";
const CAVE_SPIDER: &str = "minecraft:cave_spider";
const SPIDER: &str = "minecraft:spider";
const SPIDER_SKIN: &str = "minecraft:entity/spider/spider";
pub const CAVE_SPIDER_SKIN: &str = "minecraft:entity/spider/cave_spider";
const CAVE_SPIDER_SCALE: f32 = 0.7;
const DRAWN_KINDS: &[&str] = &[
    "minecraft:bat",
    "minecraft:blaze",
    "minecraft:bogged",
    "minecraft:cat",
    "minecraft:chicken",
    "minecraft:cow",
    "minecraft:creeper",
    "minecraft:donkey",
    "minecraft:enderman",
    "minecraft:ghast",
    "minecraft:hoglin",
    "minecraft:horse",
    "minecraft:husk",
    "minecraft:iron_golem",
    "minecraft:llama",
    "minecraft:magma_cube",
    "minecraft:mooshroom",
    "minecraft:mule",
    "minecraft:ocelot",
    "minecraft:parched",
    "minecraft:pig",
    "minecraft:piglin",
    "minecraft:piglin_brute",
    "minecraft:sheep",
    "minecraft:shulker",
    "minecraft:skeleton",
    "minecraft:slime",
    "minecraft:spider",
    "minecraft:stray",
    "minecraft:strider",
    "minecraft:trader_llama",
    "minecraft:villager",
    "minecraft:wandering_trader",
    "minecraft:witch",
    "minecraft:wither_skeleton",
    "minecraft:wolf",
    "minecraft:zoglin",
    "minecraft:zombie",
    "minecraft:zombie_villager",
    "minecraft:zombified_piglin",
];

pub fn display_scale(width: f32, height: f32) -> f32 {
    let longest = width.max(height);
    if longest > 1.0 {
        0.53125 / longest
    } else {
        0.53125
    }
}

pub fn spin_step(spin: f64, delay: i32) -> f64 {
    (spin + f64::from(1000.0_f32 / (delay as f32 + 200.0))) % 360.0
}

pub fn display_turn(spin_degrees: f32) -> Quat {
    Quat::from_rotation_y(spin_degrees.to_radians())
        * Quat::from_rotation_x((-30.0_f32).to_radians())
}

pub fn display_feet(pos: BlockPos) -> DVec3 {
    DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.2,
        f64::from(pos.2) + 0.5,
    )
}

fn center(pos: BlockPos) -> DVec3 {
    DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2)) + 0.5
}

fn short(tag: &Tag, key: &str, default: i32) -> i32 {
    tag.get(key)
        .and_then(Tag::as_i64)
        .map_or(default, |v| v as i32)
}

pub fn is_spawner(tag: &Tag) -> bool {
    tag.get("id").and_then(Tag::as_str) == Some(SPAWNER)
}

fn entity_of(tag: &Tag) -> Option<Tag> {
    tag.get("SpawnData")
        .and_then(|data| data.get("entity"))
        .cloned()
}

struct Shown {
    world: EntityWorld,
    mobs: ClientMobs,
    id: u64,
    kind: String,
    size: (f32, f32),
    cave: bool,
}

impl Shown {
    fn build(pos: BlockPos, entity: &Tag) -> Option<Self> {
        let kind = entity.get("id").and_then(Tag::as_str)?;
        let cave = kind == CAVE_SPIDER;
        let kind = if cave { SPIDER } else { kind }.to_owned();
        if !DRAWN_KINDS.contains(&kind.as_str()) {
            return None;
        }
        let mut tag = entity.clone();
        let Tag::Compound(map) = &mut tag else {
            return None;
        };
        map.insert("id".to_owned(), Tag::String(kind.clone()));
        let feet = display_feet(pos);
        map.insert(
            "Pos".to_owned(),
            Tag::List(vec![
                Tag::Double(feet.x),
                Tag::Double(feet.y),
                Tag::Double(feet.z),
            ]),
        );
        let mut world = EntityWorld::default();
        let id = crate::server_mobs::spawn_saved(&mut world, &tag)?;
        if let Some(body) = world.body_mut(id) {
            body.position = feet;
        }
        let tracked = server_mobs(&world);
        let size = if cave {
            (0.7, 0.5)
        } else {
            tracked.iter().find(|mob| mob.id == id)?.size
        };
        let mut mobs = ClientMobs::default();
        mobs.spawn_missing(&tracked);
        Some(Self {
            world,
            mobs,
            id,
            kind,
            size,
            cave,
        })
    }

    fn scale(&self) -> f32 {
        let model = if self.cave { CAVE_SPIDER_SCALE } else { 1.0 };
        display_scale(self.size.0, self.size.1) * model
    }

    fn reskin(&self, mesh: &mut ChunkMesh, start: usize, atlas: &Atlas) {
        if !self.cave {
            return;
        }
        let parse = |id: &str| crate::pack::ResourceId::parse(id).ok();
        let (Some(spider), Some(cave)) = (parse(SPIDER_SKIN), parse(CAVE_SPIDER_SKIN)) else {
            return;
        };
        let from = atlas.entity_region(&spider);
        let to = atlas.entity_region(&cave);
        for vertex in &mut mesh.vertices[start..] {
            let [u, v] = vertex.uv;
            if u < from[0] || u > from[2] || v < from[1] || v > from[3] {
                continue;
            }
            vertex.uv = [
                to[0] + (u - from[0]) / (from[2] - from[0]) * (to[2] - to[0]),
                to[1] + (v - from[1]) / (from[3] - from[1]) * (to[3] - to[1]),
            ];
        }
    }

    fn draw(
        &self,
        models: &mut ChunkMesh,
        culled: &mut ChunkMesh,
        translucent: &mut ChunkMesh,
        atlas: &Atlas,
        light: &SkyLight,
        partial: f32,
    ) {
        use crate::*;
        let w = &self.world;
        let poses = &self.mobs;
        match self.kind.as_str() {
            "minecraft:cow" | "minecraft:mooshroom" => {
                cow_render::append_cows(models, w.cows().iter(), poses, atlas, light, partial)
            }
            "minecraft:horse" | "minecraft:donkey" | "minecraft:mule" => horse_render::append_horses(
                models,
                translucent,
                w.cows().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:llama" | "minecraft:trader_llama" => {
                llama_render::append_llamas(models, w.cows().iter(), poses, atlas, light, partial)
            }
            "minecraft:sheep" => {
                sheep_render::append_sheep(models, w.sheep().iter(), poses, atlas, light, partial)
            }
            "minecraft:pig" => {
                pig_render::append_pigs(models, w.pigs().iter(), poses, atlas, light, partial)
            }
            "minecraft:chicken" => chicken_render::append_chickens(
                models,
                w.chickens().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:bat" => {
                bat_render::append_bats(culled, w.bats().iter(), poses, atlas, light, partial)
            }
            "minecraft:zombie"
            | "minecraft:husk"
            | "minecraft:zombie_villager"
            | "minecraft:zombified_piglin" => {
                zombie_render::append_zombies(
                    models,
                    w.zombies().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:creeper" => creeper_render::append_creepers(
                models,
                w.creepers().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:spider" => spider_render::append_spiders(
                models,
                w.spiders().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:skeleton"
            | "minecraft:stray"
            | "minecraft:bogged"
            | "minecraft:parched"
            | "minecraft:wither_skeleton" => {
                skeleton_render::append_skeletons(
                    models,
                    w.skeletons().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:villager" => {
                villager_render::append_villagers(
                    models,
                    w.villagers().iter(),
                    poses,
                    &|_| None,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:wandering_trader" => {
                villager_render::append_wandering_traders(
                    models,
                    w.wandering_traders().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:slime" => slime_render::append_slimes(
                models,
                translucent,
                w.slimes().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:magma_cube" => magma_cube_render::append_magma_cubes(
                models,
                w.slimes().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:enderman" => {
                enderman_render::append_endermen(
                    models,
                    w.endermen().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                    0,
                );
            }
            "minecraft:witch" => {
                witch_render::append_witches(
                    models,
                    w.witches().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:ghast" => {
                ghast_render::append_ghasts(models, w.ghasts().iter(), poses, atlas, light, partial)
            }
            "minecraft:blaze" => {
                blaze_render::append_blazes(models, w.blazes().iter(), poses, atlas, light, partial)
            }
            "minecraft:iron_golem" => {
                golem_render::append_iron_golems(
                    models,
                    w.iron_golems().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:wolf" => wolf_render::append_wolves(
                models,
                w.wolves().iter(),
                poses,
                atlas,
                light,
                partial,
                0,
            ),
            "minecraft:cat" | "minecraft:ocelot" => crate::cat_render::append_felines(
                models,
                w.felines().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:strider" => strider_render::append_striders(
                models,
                w.striders().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:hoglin" | "minecraft:zoglin" => hoglin_render::append_hoglins(
                models,
                w.hoglins().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            "minecraft:piglin" | "minecraft:piglin_brute" => {
                piglin_render::append_piglins(
                    models,
                    w.piglins().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:shulker" => shulker_render::append_shulkers(
                models,
                w.shulkers().iter(),
                poses,
                atlas,
                light,
                partial,
            ),
            _ => {}
        }
    }
}

struct SpawnerDisplay {
    entity: Option<Tag>,
    delay: i32,
    min_delay: i32,
    range: f64,
    spin: f64,
    o_spin: f64,
    shown: Option<Option<Shown>>,
}

impl SpawnerDisplay {
    fn read(tag: &Tag) -> Self {
        Self {
            entity: entity_of(tag),
            delay: short(tag, "Delay", 20),
            min_delay: short(tag, "MinSpawnDelay", 200),
            range: f64::from(short(tag, "RequiredPlayerRange", 16)),
            spin: 0.0,
            o_spin: 0.0,
            shown: None,
        }
    }

    fn has_mob(&self) -> bool {
        self.entity
            .as_ref()
            .and_then(|entity| entity.get("id"))
            .and_then(Tag::as_str)
            .is_some()
    }

    fn near(&self, pos: BlockPos, players: &[DVec3]) -> bool {
        let center = center(pos);
        players
            .iter()
            .any(|feet| self.range < 0.0 || feet.distance_squared(center) < self.range * self.range)
    }
}

pub struct SpawnerDisplays {
    displays: BTreeMap<BlockPos, SpawnerDisplay>,
    random: LegacyRandom,
}

impl SpawnerDisplays {
    pub fn new(seed: u64) -> Self {
        Self {
            displays: BTreeMap::new(),
            random: LegacyRandom::new(seed),
        }
    }

    pub fn len(&self) -> usize {
        self.displays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.displays.is_empty()
    }

    pub fn load_chunk(&mut self, chunk: &minecraftoss_core::Chunk) {
        let store = &chunk.block_entities;
        for (pos, tag) in store.entities.iter().chain(
            store
                .pending
                .iter()
                .filter(|(pos, _)| !store.entities.contains_key(pos)),
        ) {
            if is_spawner(tag) {
                self.displays.insert(*pos, SpawnerDisplay::read(tag));
            }
        }
    }

    pub fn unload_chunk(&mut self, pos: minecraftoss_core::ChunkPos) {
        self.displays
            .retain(|&(x, _, z), _| (x >> 4, z >> 4) != (pos.x, pos.z));
    }

    pub fn block_entities(&mut self, changes: &[(BlockPos, Option<Tag>)]) {
        for (pos, tag) in changes {
            let Some(tag) = tag.as_ref().filter(|tag| is_spawner(tag)) else {
                self.displays.remove(pos);
                continue;
            };
            let Some(display) = self.displays.get_mut(pos) else {
                self.displays.insert(*pos, SpawnerDisplay::read(tag));
                continue;
            };
            display.min_delay = short(tag, "MinSpawnDelay", 200);
            display.range = f64::from(short(tag, "RequiredPlayerRange", 16));
            let entity = entity_of(tag);
            if entity != display.entity {
                display.entity = entity;
                display.shown = None;
                display.delay = short(tag, "Delay", 20);
            } else {
                display.delay = display.min_delay;
            }
        }
    }

    pub fn tick(&mut self, players: &[DVec3], particles: &mut AmbientParticles) {
        for (&pos, display) in &mut self.displays {
            if !display.near(pos, players) {
                display.o_spin = display.spin;
                continue;
            }
            if !display.has_mob() {
                continue;
            }
            let random = &mut self.random;
            let at = DVec3::new(
                f64::from(pos.0) + random.next_double(),
                f64::from(pos.1) + random.next_double(),
                f64::from(pos.2) + random.next_double(),
            );
            particles.add(Kind::Smoke, at, DVec3::ZERO);
            particles.add(Kind::Flame, at, DVec3::ZERO);
            if display.delay > 0 {
                display.delay -= 1;
            }
            display.o_spin = display.spin;
            display.spin = spin_step(display.spin, display.delay);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn append<S: Scene + ?Sized>(
        &mut self,
        models: &mut ChunkMesh,
        culled: &mut ChunkMesh,
        translucent: &mut ChunkMesh,
        scene: &S,
        atlas: &Atlas,
        light: &SkyLight,
        camera: DVec3,
        partial: f32,
    ) {
        let partial = partial.clamp(0.0, 1.0);
        for (&pos, display) in &mut self.displays {
            if center(pos).distance_squared(camera) >= VIEW_DISTANCE * VIEW_DISTANCE {
                continue;
            }
            if !scene.block(pos).is_some_and(|block| {
                block.id.namespace == "minecraft" && block.id.path == "spawner"
            }) {
                continue;
            }
            let Some(entity) = &display.entity else {
                continue;
            };
            let Some(shown) = display
                .shown
                .get_or_insert_with(|| Shown::build(pos, entity))
            else {
                continue;
            };
            let spin = (display.o_spin + f64::from(partial) * (display.spin - display.o_spin))
                as f32
                * 10.0;
            shown
                .mobs
                .show_in_spawner(shown.id, display_turn(spin), center(pos));
            let Some(feet) = shown
                .mobs
                .pose(shown.id, partial)
                .map(|pose| pose.feet.as_vec3())
            else {
                continue;
            };
            let starts = [
                models.vertices.len(),
                culled.vertices.len(),
                translucent.vertices.len(),
            ];
            shown.draw(models, culled, translucent, atlas, light, partial);
            shown.reskin(models, starts[0], atlas);
            let scale = shown.scale();
            for (mesh, start) in [&mut *models, &mut *culled, &mut *translucent]
                .into_iter()
                .zip(starts)
            {
                for vertex in &mut mesh.vertices[start..] {
                    let at = feet + (Vec3::from_array(vertex.position) - feet) * scale;
                    vertex.position = at.to_array();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Block, ChunkPos};
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

    fn spawners(at: &[BlockPos]) -> Blocks {
        Blocks(
            at.iter()
                .map(|&pos| (pos, Block::new("minecraft:spawner")))
                .collect(),
        )
    }

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        crate::mesh::make_atlas(&packs, Vec::new()).unwrap()
    }

    fn spawner_tag(mob: Option<&str>, delay: i16) -> Tag {
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(SPAWNER.to_owned()));
        map.insert("Delay".to_owned(), Tag::Short(delay));
        map.insert("MinSpawnDelay".to_owned(), Tag::Short(200));
        map.insert("MaxSpawnDelay".to_owned(), Tag::Short(800));
        map.insert("RequiredPlayerRange".to_owned(), Tag::Short(16));
        if let Some(mob) = mob {
            let mut entity = BTreeMap::new();
            entity.insert("id".to_owned(), Tag::String(mob.to_owned()));
            let mut data = BTreeMap::new();
            data.insert("entity".to_owned(), Tag::Compound(entity));
            map.insert("SpawnData".to_owned(), Tag::Compound(data));
        }
        Tag::Compound(map)
    }

    fn one(pos: BlockPos, mob: Option<&str>, delay: i16) -> SpawnerDisplays {
        let mut displays = SpawnerDisplays::new(1);
        displays.block_entities(&[(pos, Some(spawner_tag(mob, delay)))]);
        displays
    }

    fn light_at(pos: BlockPos, sky: u8, block: u8) -> SkyLight {
        let min = (pos.0 - 2, pos.1 - 2, pos.2 - 2);
        SkyLight::from_levels(min, (5, 5, 5), vec![sky; 125], vec![block; 125])
    }

    fn draw(
        displays: &mut SpawnerDisplays,
        scene: &Blocks,
        light: &SkyLight,
        camera: DVec3,
    ) -> ChunkMesh {
        let mut models = ChunkMesh::default();
        let mut culled = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        displays.append(
            &mut models,
            &mut culled,
            &mut translucent,
            scene,
            &atlas(),
            light,
            camera,
            0.5,
        );
        models.vertices.extend(culled.vertices);
        models.vertices.extend(translucent.vertices);
        models
    }

    fn set_spin(displays: &mut SpawnerDisplays, degrees: f64) {
        for display in displays.displays.values_mut() {
            display.spin = degrees / 10.0;
            display.o_spin = degrees / 10.0;
        }
    }

    #[test]
    fn a_display_scale_fits_the_longest_side_into_the_cage() {
        assert_eq!(display_scale(0.6, 1.95), 0.53125 / 1.95);
        assert_eq!(display_scale(1.4, 0.9), 0.53125 / 1.4);
        assert_eq!(display_scale(0.4, 0.7), 0.53125);
        assert_eq!(display_scale(1.0, 1.0), 0.53125);
    }

    #[test]
    fn spin_steps_a_thousand_over_delay_plus_two_hundred() {
        assert_eq!(spin_step(10.0, 200), 12.5);
        assert_eq!(spin_step(10.0, 0), 15.0);
        assert_eq!(spin_step(0.0, -1), f64::from(1000.0_f32 / 199.0));
        assert_eq!(spin_step(359.0, 0), 4.0);
    }

    #[test]
    fn a_far_spawner_freezes_its_spin_and_delay() {
        let pos = (0, 64, 0);
        let mut displays = one(pos, Some("minecraft:zombie"), 100);
        displays.tick(
            &[DVec3::new(17.5, 64.5, 0.5)],
            &mut AmbientParticles::new(0),
        );
        let display = &displays.displays[&pos];
        assert_eq!(
            (display.delay, display.spin, display.o_spin),
            (100, 0.0, 0.0)
        );
        displays.tick(
            &[DVec3::new(15.5, 64.5, 0.5)],
            &mut AmbientParticles::new(0),
        );
        let display = &displays.displays[&pos];
        assert_eq!(display.delay, 99);
        assert_eq!(display.o_spin, 0.0);
        assert_eq!(display.spin, spin_step(0.0, 99));
        displays.tick(&[], &mut AmbientParticles::new(0));
        let display = &displays.displays[&pos];
        assert_eq!(display.delay, 99);
        assert_eq!(display.o_spin, display.spin);
    }

    #[test]
    fn a_near_spawner_puts_one_smoke_and_one_flame_at_one_point_each_tick() {
        let pos = (-4, 20, 9);
        let mut displays = one(pos, Some("minecraft:silverfish"), 20);
        let mut particles = AmbientParticles::new(3);
        let near = [DVec3::new(-2.0, 20.0, 9.0)];
        for tick in 1..=30 {
            let before = particles.len();
            displays.tick(&near, &mut particles);
            assert_eq!(particles.len(), before + 2);
            let added: Vec<_> = particles.kinds_at().skip(before).collect();
            assert_eq!(added[0].0, Kind::Smoke);
            assert_eq!(added[1].0, Kind::Flame);
            assert!((added[1].1 - added[0].1).abs().max_element() <= 0.05);
            for (_, at) in &added {
                let inside = *at - DVec3::new(-4.0, 20.0, 9.0);
                assert!(
                    inside.min_element() >= -0.05 && inside.max_element() <= 1.05,
                    "tick {tick}: {inside}"
                );
            }
        }
        assert!(displays.displays[&pos].spin > 0.0);
        let before = particles.len();
        displays.tick(&[DVec3::new(30.0, 20.0, 9.0)], &mut particles);
        assert_eq!(particles.len(), before);
        let mut empty = one(pos, None, 20);
        empty.tick(&near, &mut particles);
        assert_eq!(particles.len(), before);
    }

    #[test]
    fn an_empty_spawner_neither_spins_nor_draws() {
        let pos = (0, 64, 0);
        let mut displays = one(pos, None, 20);
        for _ in 0..5 {
            displays.tick(&[DVec3::new(1.5, 64.0, 0.5)], &mut AmbientParticles::new(0));
        }
        let display = &displays.displays[&pos];
        assert_eq!((display.delay, display.spin), (20, 0.0));
        let mesh = draw(
            &mut displays,
            &spawners(&[pos]),
            &light_at(pos, 15, 0),
            center(pos),
        );
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn a_spawn_round_restarts_the_spin_at_the_minimum_delay() {
        let pos = (3, 40, -7);
        let mut displays = one(pos, Some("minecraft:zombie"), 30);
        assert_eq!(displays.displays[&pos].delay, 30);
        displays.displays.get_mut(&pos).unwrap().spin = 77.0;
        displays.block_entities(&[(pos, Some(spawner_tag(Some("minecraft:zombie"), 600)))]);
        assert_eq!(displays.displays[&pos].delay, 200);
        assert_eq!(displays.displays[&pos].spin, 77.0);
        let scene = spawners(&[pos]);
        let light = light_at(pos, 15, 0);
        draw(&mut displays, &scene, &light, center(pos));
        assert!(matches!(displays.displays[&pos].shown, Some(Some(_))));
        displays.block_entities(&[(pos, Some(spawner_tag(Some("minecraft:skeleton"), 450)))]);
        let display = &displays.displays[&pos];
        assert_eq!(display.delay, 450);
        assert_eq!(display.spin, 77.0);
        assert!(display.shown.is_none());
        let mesh = draw(&mut displays, &scene, &light, center(pos));
        assert!(!mesh.vertices.is_empty());
        assert!(
            matches!(&displays.displays[&pos].shown, Some(Some(shown)) if shown.kind == "minecraft:skeleton")
        );
        displays.block_entities(&[(pos, None)]);
        assert!(displays.is_empty());
        displays.block_entities(&[
            ((3, 40, -7), Some(spawner_tag(Some("minecraft:zombie"), 5))),
            ((40, 0, 0), Some(spawner_tag(None, 5))),
        ]);
        let mut chest = BTreeMap::new();
        chest.insert("id".to_owned(), Tag::String("minecraft:chest".to_owned()));
        displays.block_entities(&[((40, 0, 0), Some(Tag::Compound(chest)))]);
        assert_eq!(displays.len(), 1);
        displays.unload_chunk(minecraftoss_core::ChunkPos::new(0, -1));
        assert!(displays.is_empty());
    }

    #[test]
    fn a_chunk_brings_its_spawners_and_their_delay() {
        let mut chunk = minecraftoss_core::Chunk::new(
            minecraftoss_core::ChunkPos::new(1, 0),
            -64,
            384,
            minecraftoss_core::BiomeId(0),
        );
        chunk
            .block_entities
            .entities
            .insert((17, 30, 4), spawner_tag(Some("minecraft:blaze"), 44));
        chunk
            .block_entities
            .pending
            .insert((18, 30, 4), spawner_tag(Some("minecraft:cave_spider"), 20));
        chunk
            .block_entities
            .pending
            .insert((17, 30, 4), spawner_tag(None, 20));
        let mut chest = BTreeMap::new();
        chest.insert("id".to_owned(), Tag::String("minecraft:chest".to_owned()));
        chunk
            .block_entities
            .entities
            .insert((19, 30, 4), Tag::Compound(chest));
        let mut displays = SpawnerDisplays::new(1);
        displays.load_chunk(&chunk);
        assert_eq!(displays.len(), 2);
        assert_eq!(displays.displays[&(17, 30, 4)].delay, 44);
        assert!(displays.displays[&(17, 30, 4)].has_mob());
        assert!(displays.displays[&(18, 30, 4)].has_mob());
        displays.unload_chunk(minecraftoss_core::ChunkPos::new(0, 0));
        assert_eq!(displays.len(), 2);
        displays.unload_chunk(minecraftoss_core::ChunkPos::new(1, 0));
        assert!(displays.is_empty());
    }

    fn bounds(mesh: &ChunkMesh) -> (Vec3, Vec3) {
        mesh.vertices.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(lo, hi), v| {
                let p = Vec3::from_array(v.position);
                (lo.min(p), hi.max(p))
            },
        )
    }

    #[test]
    fn a_zombie_a_blaze_the_nether_mobs_and_a_shulker_sit_inside_their_cage() {
        for (mob, margin) in [
            ("minecraft:zombie", 0.05),
            ("minecraft:blaze", 0.05),
            ("minecraft:strider", 0.05),
            ("minecraft:hoglin", 0.1),
            ("minecraft:zoglin", 0.1),
            ("minecraft:piglin", 0.05),
            ("minecraft:piglin_brute", 0.05),
            ("minecraft:shulker", 0.1),
            ("minecraft:cat", 0.1),
            ("minecraft:ocelot", 0.25),
            ("minecraft:mule", 0.05),
            ("minecraft:llama", 0.05),
            ("minecraft:trader_llama", 0.05),
        ] {
            let pos = (10, 70, -3);
            let mut displays = one(pos, Some(mob), 20);
            let scene = spawners(&[pos]);
            let light = light_at(pos, 15, 0);
            let mut seen = Vec::new();
            for spin in [0.0, 90.0, 200.0] {
                set_spin(&mut displays, spin);
                let mesh = draw(&mut displays, &scene, &light, center(pos));
                assert!(!mesh.vertices.is_empty(), "{mob}");
                let (lo, hi) = bounds(&mesh);
                let cell = Vec3::new(10.0, 70.0, -3.0);
                assert!(
                    lo.cmpge(cell - margin).all() && hi.cmple(cell + 1.0 + margin).all(),
                    "{mob} at {spin}: {lo} {hi}"
                );
                seen.push(mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>());
            }
            assert_ne!(seen[0], seen[1], "{mob}");
            assert_ne!(seen[1], seen[2], "{mob}");
        }
    }

    #[test]
    fn a_display_is_lit_by_its_spawner_cell() {
        let pos = (0, 64, 0);
        let mut displays = one(pos, Some("minecraft:zombie"), 20);
        let mut sky = vec![15; 125];
        let mut block = vec![0; 125];
        let inside = 2 * 25 + 2 * 5 + 2;
        sky[inside] = 4;
        block[inside] = 9;
        let light = SkyLight::from_levels((-2, 62, -2), (5, 5, 5), sky, block);
        assert_eq!((light.get(pos), light.get_block(pos)), (4, 9));
        let mesh = draw(&mut displays, &spawners(&[pos]), &light, center(pos));
        assert!(!mesh.vertices.is_empty());
        assert!(
            mesh.vertices
                .iter()
                .all(|v| v.sky_light == 4.0 && v.block_light == 9.0)
        );
    }

    fn spider_atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let skins = root.join("assets/minecraft/textures/entity/spider");
        std::fs::create_dir_all(&skins).unwrap();
        for (name, shade) in [("spider", 60), ("cave_spider", 120), ("spider_eyes", 250)] {
            image::RgbaImage::from_pixel(64, 32, image::Rgba([shade, shade, shade, 255]))
                .save(skins.join(format!("{name}.png")))
                .unwrap();
        }
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let ids = [
            SPIDER_SKIN,
            CAVE_SPIDER_SKIN,
            "minecraft:entity/spider/spider_eyes",
        ]
        .map(|id| crate::pack::ResourceId::parse(id).unwrap());
        crate::mesh::make_atlas(&packs, ids.to_vec()).unwrap()
    }

    #[test]
    fn a_cave_spider_display_is_a_smaller_spider_with_its_own_skin() {
        let atlas = spider_atlas();
        let pos = (2, 30, 2);
        let scene = spawners(&[pos]);
        let light = light_at(pos, 15, 0);
        let mut drawn = Vec::new();
        for mob in [SPIDER, CAVE_SPIDER] {
            let mut displays = one(pos, Some(mob), 20);
            set_spin(&mut displays, 40.0);
            let mut models = ChunkMesh::default();
            let mut culled = ChunkMesh::default();
            let mut translucent = ChunkMesh::default();
            displays.append(
                &mut models,
                &mut culled,
                &mut translucent,
                &scene,
                &atlas,
                &light,
                center(pos),
                0.0,
            );
            assert!(culled.vertices.is_empty() && translucent.vertices.is_empty());
            drawn.push(models.vertices);
        }
        let (spider, cave) = (&drawn[0], &drawn[1]);
        assert!(!spider.is_empty());
        assert_eq!(spider.len(), cave.len());
        let feet = display_feet(pos).as_vec3();
        let ratio = 0.53125 * 0.7 / (0.53125 / 1.4);
        for (a, b) in spider.iter().zip(cave) {
            let expected = feet + (Vec3::from_array(a.position) - feet) * ratio;
            assert!(Vec3::from_array(b.position).abs_diff_eq(expected, 1e-4));
        }
        let region = |id: &str| atlas.entity_region(&crate::pack::ResourceId::parse(id).unwrap());
        let (skin, cave_skin) = (region(SPIDER_SKIN), region(CAVE_SPIDER_SKIN));
        let inside = |uv: [f32; 2], r: [f32; 4]| {
            uv[0] >= r[0] && uv[0] <= r[2] && uv[1] >= r[1] && uv[1] <= r[3]
        };
        let mut skinned = 0;
        let mut eyes = 0;
        for (a, b) in spider.iter().zip(cave) {
            if inside(a.uv, skin) {
                skinned += 1;
                assert!(inside(b.uv, cave_skin), "{:?}", b.uv);
                let back = [
                    skin[0]
                        + (b.uv[0] - cave_skin[0]) / (cave_skin[2] - cave_skin[0])
                            * (skin[2] - skin[0]),
                    skin[1]
                        + (b.uv[1] - cave_skin[1]) / (cave_skin[3] - cave_skin[1])
                            * (skin[3] - skin[1]),
                ];
                assert!((back[0] - a.uv[0]).abs() < 1e-5 && (back[1] - a.uv[1]).abs() < 1e-5);
            } else {
                eyes += 1;
                assert_eq!(a.uv, b.uv);
            }
        }
        assert!(skinned > 0 && eyes > 0, "{skinned} {eyes}");
        assert_ne!(skin, cave_skin);
    }

    #[test]
    fn spawners_further_than_64_blocks_are_not_drawn() {
        let pos = (0, 64, 0);
        let mut displays = one(pos, Some("minecraft:zombie"), 20);
        let scene = spawners(&[pos]);
        let light = light_at(pos, 15, 0);
        assert!(
            !draw(
                &mut displays,
                &scene,
                &light,
                center(pos) + DVec3::new(63.9, 0.0, 0.0)
            )
            .vertices
            .is_empty()
        );
        assert!(
            draw(
                &mut displays,
                &scene,
                &light,
                center(pos) + DVec3::new(64.0, 0.0, 0.0)
            )
            .vertices
            .is_empty()
        );
        assert!(
            draw(&mut displays, &spawners(&[]), &light, center(pos))
                .vertices
                .is_empty()
        );
    }
}
