use crate::ambient_particles::{AmbientParticles, Kind};
use crate::client_mobs::{ClientMobs, server_mobs};
use crate::display_ticks::AmbientSound;
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::scene::{Block, BlockPos, Scene};
use crate::trial_spawner::{Emission, TrialState};
use glam::{DVec3, Quat, Vec3};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_player::rng::LegacyRandom;
use std::collections::BTreeMap;

pub const VIEW_DISTANCE: f64 = 64.0;
const SPAWNER: &str = "minecraft:mob_spawner";
const TRIAL_SPAWNER: &str = "minecraft:trial_spawner";
const TRIAL_AMBIENT: &str = "minecraft:block.trial_spawner.ambient";
const TRIAL_AMBIENT_OMINOUS: &str = "minecraft:block.trial_spawner.ambient_ominous";
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
    "minecraft:evoker",
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
    "minecraft:pillager",
    "minecraft:ravager",
    "minecraft:sheep",
    "minecraft:shulker",
    "minecraft:skeleton",
    "minecraft:slime",
    "minecraft:snow_golem",
    "minecraft:spider",
    "minecraft:stray",
    "minecraft:strider",
    "minecraft:trader_llama",
    "minecraft:vex",
    "minecraft:villager",
    "minecraft:vindicator",
    "minecraft:wandering_trader",
    "minecraft:witch",
    "minecraft:wither",
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

fn next_mob_spawns_at(tag: &Tag) -> i64 {
    tag.get("next_mob_spawns_at")
        .and_then(Tag::as_i64)
        .unwrap_or(0)
}

pub fn is_displayed_block_entity(tag: &Tag) -> bool {
    matches!(
        tag.get("id").and_then(Tag::as_str),
        Some(SPAWNER | TRIAL_SPAWNER)
    )
}

pub fn is_displayed_block(block: &Block) -> bool {
    block.id.namespace == "minecraft" && matches!(block.id.path.as_str(), "spawner" | "trial_spawner")
}

fn is_trial(tag: &Tag) -> bool {
    tag.get("id").and_then(Tag::as_str) == Some(TRIAL_SPAWNER)
}

pub fn trial_block_state(block: &Block) -> Option<(TrialState, bool)> {
    if block.id.namespace != "minecraft" || block.id.path != "trial_spawner" {
        return None;
    }
    let state = block
        .properties
        .get(crate::trial_spawner::STATE)
        .and_then(|name| TrialState::from_name(name))
        .unwrap_or(TrialState::Inactive);
    let ominous = block
        .properties
        .get(crate::trial_spawner::OMINOUS)
        .is_some_and(|value| value == "true");
    Some((state, ominous))
}

fn shows_mob(block: &Block, trial: bool) -> bool {
    if trial {
        trial_block_state(block).is_some_and(|(state, _)| state.has_spinning_mob())
    } else {
        block.id.namespace == "minecraft" && block.id.path == "spawner"
    }
}

fn entity_of(tag: &Tag) -> Option<Tag> {
    let key = if is_trial(tag) { "spawn_data" } else { "SpawnData" };
    tag.get(key)
        .and_then(|data| data.get("entity"))
        .cloned()
}

fn offset_random(center: DVec3, random: &mut LegacyRandom, spread: f32) -> DVec3 {
    let mut offset = || f64::from((random.next_float() - 0.5) * spread);
    let x = offset();
    let y = offset();
    let z = offset();
    center + DVec3::new(x, y, z)
}

fn emit(
    emission: Emission,
    ominous: bool,
    pos: BlockPos,
    game_time: i64,
    random: &mut LegacyRandom,
    particles: &mut AmbientParticles,
) {
    let flame = if ominous { Kind::SoulFireFlame } else { Kind::Flame };
    let small = if ominous { Kind::SoulFireFlame } else { Kind::SmallFlame };
    match emission {
        Emission::None => {}
        Emission::SmallFlames => {
            if random.next_int(2) == 0 {
                let at = offset_random(center(pos), random, 0.9);
                particles.add(small, at, DVec3::ZERO);
            }
        }
        Emission::FlamesAndSmoke => {
            let at = offset_random(center(pos), random, 1.0);
            particles.add(Kind::Smoke, at, DVec3::ZERO);
            particles.add(flame, at, DVec3::ZERO);
        }
        Emission::SmokeInsideAndTopFace => {
            let at = offset_random(center(pos), random, 0.9);
            if random.next_int(3) == 0 {
                particles.add(Kind::Smoke, at, DVec3::ZERO);
            }
            if game_time % 20 == 0 {
                let top = center(pos) + DVec3::new(0.0, 0.5, 0.0);
                for _ in 0..random.next_int(4) + 20 {
                    particles.add(Kind::Smoke, top, DVec3::ZERO);
                }
            }
        }
    }
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
        if !DRAWN_KINDS.contains(&kind.as_str()) && !crate::families::knows(&kind) {
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
            "minecraft:snow_golem" => {
                crate::snow_golem_render::append_snow_golems(
                    models,
                    w.snow_golems().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:pillager" | "minecraft:vindicator" | "minecraft:evoker" => {
                crate::illager_render::append_illagers(
                    models,
                    w.illagers().iter(),
                    poses,
                    atlas,
                    light,
                    partial,
                );
            }
            "minecraft:vex" => {
                crate::vex_render::append_vexes(models, w.vexes().iter(), poses, atlas, light, partial);
            }
            "minecraft:ravager" => {
                crate::ravager_render::append_ravagers(models, w.ravagers().iter(), poses, atlas, light, partial);
            }
            "minecraft:wither" => {
                crate::wither_render::append_withers(models, translucent, w.withers().iter(), poses, atlas, light, partial);
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
            kind if crate::families::knows(kind) => crate::families::draw(
                &mut crate::families::FamilyDraw { models, culled, translucent, held: &mut Vec::new() },
                w,
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
    trial: bool,
    next_mob_spawns_at: i64,
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
            trial: is_trial(tag),
            next_mob_spawns_at: next_mob_spawns_at(tag),
        }
    }

    fn trial_spin(&mut self, state: TrialState, game_time: i64) {
        if !state.has_spinning_mob() {
            return;
        }
        let waiting = if state == TrialState::Active {
            (self.next_mob_spawns_at - game_time).max(0)
        } else {
            0
        };
        self.o_spin = self.spin;
        self.spin = (self.spin + state.spin_speed() / (waiting as f64 + 200.0)) % 360.0;
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
            if is_displayed_block_entity(tag) {
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
            let Some(tag) = tag.as_ref().filter(|tag| is_displayed_block_entity(tag)) else {
                self.displays.remove(pos);
                continue;
            };
            let Some(display) = self
                .displays
                .get_mut(pos)
                .filter(|display| display.trial == is_trial(tag))
            else {
                self.displays.insert(*pos, SpawnerDisplay::read(tag));
                continue;
            };
            if display.trial {
                display.next_mob_spawns_at = next_mob_spawns_at(tag);
                let entity = entity_of(tag);
                if entity != display.entity {
                    display.entity = entity;
                    display.shown = None;
                }
                continue;
            }
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
            if display.trial {
                continue;
            }
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

    pub fn tick_trials<S: Scene + ?Sized>(
        &mut self,
        scene: &S,
        game_time: i64,
        particles: &mut AmbientParticles,
    ) -> Vec<AmbientSound> {
        let mut sounds = Vec::new();
        for (&pos, display) in &mut self.displays {
            if !display.trial {
                continue;
            }
            let Some((state, ominous)) = scene.block(pos).and_then(trial_block_state) else {
                continue;
            };
            let random = &mut self.random;
            emit(state.emission(), ominous, pos, game_time, random, particles);
            display.trial_spin(state, game_time);
            if state.can_spawn() && random.next_float() <= 0.02 {
                let event = if ominous { TRIAL_AMBIENT_OMINOUS } else { TRIAL_AMBIENT };
                let volume = random.next_float() * 0.25 + 0.75;
                let pitch = random.next_float() + 0.5;
                sounds.push(AmbientSound {
                    event,
                    at: center(pos).to_array(),
                    volume,
                    pitch,
                });
            }
        }
        sounds
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
            if !scene
                .block(pos)
                .is_some_and(|block| shows_mob(block, display.trial))
            {
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
            ("minecraft:snow_golem", 0.05),
            ("minecraft:pillager", 0.05),
            ("minecraft:vindicator", 0.05),
            ("minecraft:evoker", 0.05),
            ("minecraft:vex", 0.05),
            ("minecraft:ravager", 0.1),
            ("minecraft:wither", 0.1),
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
    fn every_registered_kind_is_shown_inside_its_cage() {
        let kinds = crate::families::kinds();
        assert!(kinds.contains(&"minecraft:silverfish") && kinds.contains(&"minecraft:endermite"));
        for mob in kinds {
            let margin = crate::families::cage_margin(mob).unwrap();
            let pos = (10, 70, -3);
            let mut displays = one(pos, Some(mob), 20);
            let scene = spawners(&[pos]);
            let light = light_at(pos, 15, 0);
            let cell = Vec3::new(10.0, 70.0, -3.0);
            let mut seen = Vec::new();
            for spin in [0.0, 90.0, 200.0] {
                set_spin(&mut displays, spin);
                let mesh = draw(&mut displays, &scene, &light, center(pos));
                let Some(Some(shown)) = &displays.displays[&pos].shown else {
                    panic!("{mob} has no display");
                };
                assert_eq!((shown.kind.as_str(), shown.world.entity_type(shown.id)), (mob, Some(mob)));
                assert!(
                    mesh.vertices.iter().all(|v| {
                        let at = Vec3::from_array(v.position);
                        at.cmpge(cell - margin).all() && at.cmple(cell + 1.0 + margin).all()
                    }),
                    "{mob} at {spin}"
                );
                seen.push(mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>());
            }
            assert!(seen[0].is_empty() || (seen[0] != seen[1] && seen[1] != seen[2]), "{mob} turns");
            if ["minecraft:silverfish", "minecraft:endermite"].contains(&mob) {
                assert!(!seen[0].is_empty(), "{mob} is drawn in its cage");
            }
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

    fn trial_tag(mob: Option<&str>, next: i64) -> Tag {
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(TRIAL_SPAWNER.to_owned()));
        map.insert("next_mob_spawns_at".to_owned(), Tag::Long(next));
        map.insert(
            "normal_config".to_owned(),
            Tag::String("minecraft:trial_chamber/melee/zombie/normal".to_owned()),
        );
        if let Some(mob) = mob {
            let mut entity = BTreeMap::new();
            entity.insert("id".to_owned(), Tag::String(mob.to_owned()));
            let mut data = BTreeMap::new();
            data.insert("entity".to_owned(), Tag::Compound(entity));
            map.insert("spawn_data".to_owned(), Tag::Compound(data));
        }
        Tag::Compound(map)
    }

    fn trial_scene(pos: BlockPos, state: TrialState, ominous: bool) -> Blocks {
        let ominous = if ominous { "true" } else { "false" };
        let block = Block::new("minecraft:trial_spawner")
            .with(crate::trial_spawner::STATE, state.name())
            .with(crate::trial_spawner::OMINOUS, ominous);
        Blocks(HashMap::from([(pos, block)]))
    }

    fn one_trial(pos: BlockPos, mob: Option<&str>, next: i64) -> SpawnerDisplays {
        let mut displays = SpawnerDisplays::new(1);
        displays.block_entities(&[(pos, Some(trial_tag(mob, next)))]);
        displays
    }

    #[test]
    fn a_waiting_trial_spawner_shows_its_mob_and_spins_slower_than_an_active_one() {
        let pos = (4, -30, 9);
        let mut displays = one_trial(pos, Some("minecraft:zombie"), 0);
        assert!(displays.displays[&pos].trial);
        let mut particles = AmbientParticles::new(2);
        let waiting = trial_scene(pos, TrialState::WaitingForPlayers, false);
        let active = trial_scene(pos, TrialState::Active, false);
        displays.tick_trials(&waiting, 1000, &mut particles);
        let display = &displays.displays[&pos];
        assert_eq!((display.o_spin, display.spin), (0.0, 1.0));
        displays.tick_trials(&active, 1001, &mut particles);
        assert_eq!(displays.displays[&pos].spin, 6.0);
        displays.block_entities(&[(pos, Some(trial_tag(Some("minecraft:zombie"), 1042)))]);
        displays.tick_trials(&active, 1002, &mut particles);
        let slowed = 6.0 + 1000.0 / 240.0;
        assert_eq!(displays.displays[&pos].spin, slowed);
        displays.tick_trials(&waiting, 1003, &mut particles);
        assert_eq!(
            displays.displays[&pos].spin,
            slowed + 1.0,
            "the next spawn only counts while active"
        );
        let light = light_at(pos, 15, 0);
        for (state, drawn) in [
            (TrialState::WaitingForPlayers, true),
            (TrialState::Active, true),
            (TrialState::Inactive, false),
            (TrialState::WaitingForRewardEjection, false),
            (TrialState::EjectingReward, false),
            (TrialState::Cooldown, false),
        ] {
            let mesh = draw(
                &mut displays,
                &trial_scene(pos, state, false),
                &light,
                center(pos),
            );
            assert_eq!(!mesh.vertices.is_empty(), drawn, "{state:?}");
        }
        let spin = displays.displays[&pos].spin;
        let cooldown = trial_scene(pos, TrialState::Cooldown, false);
        displays.tick_trials(&cooldown, 1004, &mut particles);
        assert_eq!(displays.displays[&pos].spin, spin);
        assert!(
            draw(&mut displays, &spawners(&[pos]), &light, center(pos))
                .vertices
                .is_empty(),
            "a mob spawner block under a trial display shows nothing"
        );
        let mut empty = one_trial(pos, None, 0);
        assert!(draw(&mut empty, &waiting, &light, center(pos))
            .vertices
            .is_empty());
    }

    #[test]
    fn trial_flames_follow_the_state_and_burn_soul_blue_when_ominous() {
        let pos = (0, -20, 0);
        let top = center(pos) + DVec3::new(0.0, 0.5, 0.0);
        let mut displays = one_trial(pos, Some("minecraft:husk"), 0);
        let mut run = |state: TrialState, ominous: bool, ticks: i64| {
            let mut particles = AmbientParticles::new(5);
            let scene = trial_scene(pos, state, ominous);
            for time in 1..=ticks {
                displays.tick_trials(&scene, time, &mut particles);
            }
            particles.kinds_at().collect::<Vec<_>>()
        };
        let inside = |added: &[(Kind, DVec3)]| {
            added.iter().all(|(_, at)| {
                let local = *at - DVec3::new(0.0, -20.0, 0.0);
                local.min_element() >= -0.051 && local.max_element() <= 1.051
            })
        };
        let waiting = run(TrialState::WaitingForPlayers, false, 400);
        assert!(waiting.iter().all(|(kind, _)| *kind == Kind::SmallFlame));
        assert!((150..250).contains(&waiting.len()), "{}", waiting.len());
        assert!(inside(&waiting));
        let ominous = run(TrialState::WaitingForPlayers, true, 400);
        assert!(!ominous.is_empty());
        assert!(ominous.iter().all(|(kind, _)| *kind == Kind::SoulFireFlame));
        let ejecting = run(TrialState::EjectingReward, false, 100);
        assert!(!ejecting.is_empty());
        assert!(ejecting.iter().all(|(kind, _)| *kind == Kind::SmallFlame));
        assert!(run(TrialState::Inactive, false, 100).is_empty());
        for (ominous, flame) in [(false, Kind::Flame), (true, Kind::SoulFireFlame)] {
            let active = run(TrialState::Active, ominous, 10);
            let kinds: Vec<_> = active.iter().map(|(kind, _)| *kind).collect();
            assert_eq!(kinds, [Kind::Smoke, flame].repeat(10));
            assert!(inside(&active));
        }
        let cooldown = run(TrialState::Cooldown, false, 40);
        assert!(cooldown.iter().all(|(kind, _)| *kind == Kind::Smoke));
        let bursts = cooldown.iter().filter(|(_, at)| *at == top).count();
        assert!((40..=46).contains(&bursts), "two bursts of 20 to 23: {bursts}");
        assert!(cooldown.len() > bursts, "smoke inside one tick in three");
        let mut particles = AmbientParticles::new(5);
        displays.tick(&[center(pos)], &mut particles);
        assert!(particles.is_empty(), "the mob spawner tick leaves trials alone");
    }

    #[test]
    fn a_trial_spawner_hums_two_ticks_in_a_hundred_while_it_can_spawn() {
        let pos = (8, 0, 8);
        let mut displays = one_trial(pos, Some("minecraft:zombie"), 0);
        let mut hums = |state: TrialState, ominous: bool| {
            let scene = trial_scene(pos, state, ominous);
            let mut particles = AmbientParticles::new(1);
            (1..=5000)
                .flat_map(|time| displays.tick_trials(&scene, time, &mut particles))
                .collect::<Vec<_>>()
        };
        let waiting = hums(TrialState::WaitingForPlayers, false);
        assert!((60..140).contains(&waiting.len()), "{}", waiting.len());
        for sound in &waiting {
            assert_eq!(sound.event, TRIAL_AMBIENT);
            assert_eq!(sound.at, center(pos).to_array());
            assert!((0.75..=1.0).contains(&sound.volume));
            assert!((0.5..=1.5).contains(&sound.pitch));
        }
        let active = hums(TrialState::Active, true);
        assert!(!active.is_empty());
        assert!(active.iter().all(|sound| sound.event == TRIAL_AMBIENT_OMINOUS));
        for state in [
            TrialState::Inactive,
            TrialState::WaitingForRewardEjection,
            TrialState::EjectingReward,
            TrialState::Cooldown,
        ] {
            assert!(hums(state, false).is_empty(), "{state:?}");
        }
    }

    #[test]
    fn trial_spawners_load_with_chunks_and_follow_their_block_entity() {
        let mut chunk = minecraftoss_core::Chunk::new(
            minecraftoss_core::ChunkPos::new(0, 0),
            -64,
            384,
            minecraftoss_core::BiomeId(0),
        );
        chunk
            .block_entities
            .entities
            .insert((3, -30, 3), trial_tag(Some("minecraft:skeleton"), 0));
        chunk
            .block_entities
            .pending
            .insert((5, -30, 3), trial_tag(None, 0));
        let mut displays = SpawnerDisplays::new(1);
        displays.load_chunk(&chunk);
        assert_eq!(displays.len(), 2);
        assert!(displays.displays[&(3, -30, 3)].has_mob());
        assert!(!displays.displays[&(5, -30, 3)].has_mob());
        let pos = (3, -30, 3);
        let scene = trial_scene(pos, TrialState::WaitingForPlayers, false);
        let light = light_at(pos, 15, 0);
        draw(&mut displays, &scene, &light, center(pos));
        assert!(
            matches!(&displays.displays[&pos].shown, Some(Some(shown)) if shown.kind == "minecraft:skeleton")
        );
        displays.displays.get_mut(&pos).unwrap().spin = 33.0;
        displays.block_entities(&[(pos, Some(trial_tag(Some("minecraft:skeleton"), 90)))]);
        let display = &displays.displays[&pos];
        assert!(display.shown.is_some(), "the same mob is kept");
        assert_eq!((display.next_mob_spawns_at, display.spin), (90, 33.0));
        displays.block_entities(&[(pos, Some(trial_tag(Some("minecraft:stray"), 90)))]);
        assert!(displays.displays[&pos].shown.is_none());
        draw(&mut displays, &scene, &light, center(pos));
        assert!(
            matches!(&displays.displays[&pos].shown, Some(Some(shown)) if shown.kind == "minecraft:stray")
        );
        displays.block_entities(&[(pos, Some(spawner_tag(Some("minecraft:zombie"), 20)))]);
        let display = &displays.displays[&pos];
        assert!(!display.trial);
        assert_eq!(display.delay, 20);
        displays.block_entities(&[(pos, Some(trial_tag(Some("minecraft:husk"), 0)))]);
        assert!(displays.displays[&pos].trial);
        displays.block_entities(&[(pos, None)]);
        assert_eq!(displays.len(), 1);
    }

    #[test]
    fn spawners_and_trial_spawners_are_the_displayed_block_entities() {
        assert!(is_displayed_block_entity(&spawner_tag(None, 0)));
        assert!(is_displayed_block_entity(&trial_tag(None, 0)));
        let mut vault = BTreeMap::new();
        vault.insert("id".to_owned(), Tag::String("minecraft:vault".to_owned()));
        assert!(!is_displayed_block_entity(&Tag::Compound(vault)));
        assert!(is_displayed_block(&Block::new("minecraft:spawner")));
        assert!(is_displayed_block(&Block::new("minecraft:trial_spawner")));
        assert!(!is_displayed_block(&Block::new("minecraft:stone")));
        let ominous = Block::new("minecraft:trial_spawner")
            .with(crate::trial_spawner::STATE, "cooldown")
            .with(crate::trial_spawner::OMINOUS, "true");
        assert_eq!(
            trial_block_state(&ominous),
            Some((TrialState::Cooldown, true))
        );
        assert_eq!(
            trial_block_state(&Block::new("minecraft:trial_spawner")),
            Some((TrialState::Inactive, false))
        );
        assert_eq!(trial_block_state(&Block::new("minecraft:spawner")), None);
    }
}
