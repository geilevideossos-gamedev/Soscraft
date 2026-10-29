use crate::ambient_particles::{AmbientParticles, Kind};
use crate::display_ticks::AmbientSound;
use crate::scene::{Block, BlockPos, Scene};
use crate::vault::{OMINOUS, SharedData, is_vault, is_vault_block};
use glam::{DVec3, Vec3};
use minecraftoss_core::item::ItemStack;
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::rng::LegacyRandom;
use std::collections::BTreeMap;

pub const AMBIENT_SOUND: &str = "minecraft:block.vault.ambient";
pub const VIEW_DISTANCE: f64 = 64.0;
pub const ACTIVATION_PARTICLES: usize = 20;
const ROTATION_SPEED: f32 = 10.0;
const PARTICLE_TICK_RATE: i64 = 20;
const IDLE_PARTICLE_CHANCE: f32 = 0.5;
const AMBIENT_SOUND_CHANCE: f32 = 0.02;
const DISPLAY_HEIGHT: f64 = 0.4;
const KEYHOLE_HEIGHT: f64 = 1.75;
const CLUSTER_SPREAD: f32 = 0.15;

#[derive(Clone, Debug, PartialEq)]
pub struct VaultItemView {
    pub at: DVec3,
    pub spin: f32,
    pub item: String,
    pub count: i32,
    pub glint: bool,
}

fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn corner(pos: BlockPos) -> DVec3 {
    DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2))
}

fn between(random: &mut LegacyRandom, low: f64, high: f64) -> f64 {
    if low >= high {
        low
    } else {
        random.next_double() * (high - low) + low
    }
}

pub fn inside_cage(pos: BlockPos, random: &mut LegacyRandom) -> DVec3 {
    let x = between(random, 0.1, 0.9);
    let y = between(random, 0.25, 0.75);
    let z = between(random, 0.1, 0.9);
    corner(pos) + DVec3::new(x, y, z)
}

pub fn center_of_cage(pos: BlockPos, random: &mut LegacyRandom) -> DVec3 {
    let x = between(random, 0.4, 0.6);
    let y = between(random, 0.4, 0.6);
    let z = between(random, 0.4, 0.6);
    corner(pos) + DVec3::new(x, y, z)
}

pub fn flame(ominous: bool) -> Kind {
    if ominous {
        Kind::SoulFireFlame
    } else {
        Kind::SmallFlame
    }
}

fn ominous(block: &Block) -> bool {
    block
        .properties
        .get(OMINOUS)
        .is_some_and(|value| value == "true")
}

pub fn keyhole(pos: BlockPos, block: Option<&Block>) -> DVec3 {
    let facing = block.and_then(|block| block.properties.get("facing"));
    let (x, z) = match facing.map(String::as_str) {
        Some("south") => (0.0, 1.0),
        Some("west") => (-1.0, 0.0),
        Some("east") => (1.0, 0.0),
        _ => (0.0, -1.0),
    };
    DVec3::new(
        f64::from(pos.0) + 0.5 + x * 0.5,
        f64::from(pos.1) + KEYHOLE_HEIGHT,
        f64::from(pos.2) + 0.5 + z * 0.5,
    )
}

fn offset_random(base: DVec3, random: &mut LegacyRandom, spread: f32) -> DVec3 {
    let mut offset = || f64::from((random.next_float() - 0.5) * spread);
    let x = offset();
    let y = offset();
    let z = offset();
    base + DVec3::new(x, y, z)
}

fn in_range(feet: DVec3, pos: BlockPos, range: f64) -> bool {
    let axis = |a: f64, b: i32| a.floor() - f64::from(b);
    let (x, y, z) = (
        axis(feet.x, pos.0),
        axis(feet.y, pos.1),
        axis(feet.z, pos.2),
    );
    x * x + y * y + z * z <= range * range
}

fn shown_stack(stack: &ItemStack) -> minecraftoss_player::inventory::ItemStack {
    let count = stack.count.clamp(1, i32::from(u8::MAX)) as u8;
    let mut shown = minecraftoss_player::inventory::ItemStack::new(stack.id.clone(), count);
    shown.components = stack.components.as_ref().map(crate::item_nbt::tag_json);
    shown
}

pub fn rendered_amount(count: i32) -> usize {
    match count {
        i32::MIN..=1 => 1,
        2..=16 => 2,
        17..=32 => 3,
        33..=48 => 4,
        _ => 5,
    }
}

pub fn cluster_offsets(item: &str, count: i32, flat: bool) -> Vec<Vec3> {
    let seed = item.bytes().fold(i64::from(count), |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(i64::from(byte))
    });
    let mut random = LegacyRandom::new(seed as u64);
    let mut spread = || (random.next_float() * 2.0 - 1.0) * CLUSTER_SPREAD;
    (0..rendered_amount(count))
        .map(|copy| {
            if copy == 0 {
                return Vec3::ZERO;
            }
            let x = spread();
            let y = spread();
            let z = spread();
            Vec3::new(x, y, if flat { 0.0 } else { z })
        })
        .collect()
}

struct VaultDisplay {
    shared: SharedData,
    spin: f32,
    o_spin: f32,
}

impl VaultDisplay {
    fn read(tag: &Tag) -> Self {
        Self {
            shared: SharedData::read(tag.get("shared_data")),
            spin: 0.0,
            o_spin: 0.0,
        }
    }
}

fn connect(
    pos: BlockPos,
    display: &VaultDisplay,
    block: Option<&Block>,
    players: &dyn Fn(u128) -> Option<(DVec3, f64)>,
    random: &mut LegacyRandom,
    particles: &mut AmbientParticles,
) {
    if display.shared.connected_players.is_empty() {
        return;
    }
    let from = keyhole(pos, block);
    let range = display.shared.connected_particles_range;
    for uuid in &display.shared.connected_players {
        let Some((feet, height)) = players(*uuid) else {
            continue;
        };
        if !in_range(feet, pos, range) {
            continue;
        }
        let toward = feet + DVec3::new(0.0, height / 2.0, 0.0) - from;
        for _ in 0..random.next_int(4) + 2 {
            let motion = offset_random(toward, random, 1.0);
            particles.add(Kind::VaultConnection, from, motion);
        }
    }
}

pub struct VaultDisplays {
    displays: BTreeMap<BlockPos, VaultDisplay>,
    random: LegacyRandom,
}

impl VaultDisplays {
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
            if is_vault(tag) {
                self.displays.insert(*pos, VaultDisplay::read(tag));
            }
        }
    }

    pub fn unload_chunk(&mut self, pos: minecraftoss_core::ChunkPos) {
        self.displays
            .retain(|&(x, _, z), _| (x >> 4, z >> 4) != (pos.x, pos.z));
    }

    pub fn block_entities(&mut self, changes: &[(BlockPos, Option<Tag>)]) {
        for (pos, tag) in changes {
            let Some(tag) = tag.as_ref().filter(|tag| is_vault(tag)) else {
                self.displays.remove(pos);
                continue;
            };
            let shared = SharedData::read(tag.get("shared_data"));
            match self.displays.get_mut(pos) {
                Some(display) => display.shared = shared,
                None => {
                    self.displays.insert(*pos, VaultDisplay::read(tag));
                }
            }
        }
    }

    pub fn tick<S: Scene + ?Sized>(
        &mut self,
        scene: &S,
        game_time: i64,
        players: &dyn Fn(u128) -> Option<(DVec3, f64)>,
        particles: &mut AmbientParticles,
    ) -> Vec<AmbientSound> {
        let mut sounds = Vec::new();
        for (&pos, display) in &mut self.displays {
            let Some(block) = scene.block(pos).filter(|block| is_vault_block(block)) else {
                continue;
            };
            display.o_spin = display.spin;
            display.spin = wrap_degrees(display.spin + ROTATION_SPEED);
            let random = &mut self.random;
            if game_time % PARTICLE_TICK_RATE == 0 {
                connect(pos, display, Some(block), players, random, particles);
            }
            let shows = display.shared.display_item.is_some();
            if random.next_float() <= IDLE_PARTICLE_CHANCE {
                let at = inside_cage(pos, random);
                particles.add(Kind::Smoke, at, DVec3::ZERO);
                if shows {
                    particles.add(flame(ominous(block)), at, DVec3::ZERO);
                }
            }
            if shows && random.next_float() <= AMBIENT_SOUND_CHANCE {
                let volume = random.next_float() * 0.25 + 0.75;
                let pitch = random.next_float() + 0.5;
                sounds.push(AmbientSound {
                    event: AMBIENT_SOUND,
                    at: (corner(pos) + 0.5).to_array(),
                    volume,
                    pitch,
                });
            }
        }
        sounds
    }

    pub fn activation<S: Scene + ?Sized>(
        &mut self,
        pos: BlockPos,
        scene: &S,
        players: &dyn Fn(u128) -> Option<(DVec3, f64)>,
        particles: &mut AmbientParticles,
    ) {
        if let Some(display) = self.displays.get(&pos) {
            connect(
                pos,
                display,
                scene.block(pos),
                players,
                &mut self.random,
                particles,
            );
        }
    }

    pub fn items(&self, camera: DVec3, partial: f32) -> Vec<VaultItemView> {
        let partial = partial.clamp(0.0, 1.0);
        self.displays
            .iter()
            .filter_map(|(&pos, display)| {
                let stack = display.shared.display_item.as_ref()?;
                let at = corner(pos) + DVec3::new(0.5, DISPLAY_HEIGHT, 0.5);
                if (corner(pos) + 0.5).distance_squared(camera) >= VIEW_DISTANCE * VIEW_DISTANCE {
                    return None;
                }
                let shown = shown_stack(stack);
                Some(VaultItemView {
                    at,
                    spin: display.o_spin + partial * wrap_degrees(display.spin - display.o_spin),
                    item: crate::item_tint::of_stack(&shown),
                    count: stack.count,
                    glint: crate::glint::has_glint(&shown),
                })
            })
            .collect()
    }
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

    const AT: BlockPos = (4, 64, 4);
    const EYE: DVec3 = DVec3::new(4.5, 65.0, 8.5);

    fn vault_scene(ominous: bool) -> Blocks {
        let block = Block::new(crate::vault::ID)
            .with("facing", "south")
            .with(crate::vault::STATE, "active")
            .with(OMINOUS, if ominous { "true" } else { "false" });
        Blocks(HashMap::from([(AT, block)]))
    }

    fn vault_tag(item: Option<ItemStack>, connected: &[u128]) -> Tag {
        let shared = SharedData {
            display_item: item,
            connected_players: connected.to_vec(),
            ..SharedData::default()
        };
        let mut map = BTreeMap::new();
        map.insert("id".to_owned(), Tag::String(crate::vault::ID.to_owned()));
        map.insert("shared_data".to_owned(), shared.to_tag());
        Tag::Compound(map)
    }

    fn shown(item: Option<ItemStack>, connected: &[u128]) -> VaultDisplays {
        let mut displays = VaultDisplays::new(5);
        displays.block_entities(&[(AT, Some(vault_tag(item, connected)))]);
        displays
    }

    fn nobody(_: u128) -> Option<(DVec3, f64)> {
        None
    }

    fn count(particles: &AmbientParticles, kind: Kind) -> usize {
        particles
            .kinds_at()
            .filter(|(made, _)| *made == kind)
            .count()
    }

    #[test]
    fn the_reward_item_turns_ten_degrees_a_tick_and_hides_without_one() {
        let emerald = ItemStack::new("minecraft:emerald", 3);
        let mut displays = shown(Some(emerald), &[]);
        let scene = vault_scene(false);
        let mut particles = AmbientParticles::new(1);
        for tick in 0..3 {
            displays.tick(&scene, tick, &nobody, &mut particles);
        }
        let views = displays.items(EYE, 0.5);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].at, DVec3::new(4.5, 64.4, 4.5));
        assert_eq!(views[0].spin, 25.0);
        assert_eq!(views[0].item, "minecraft:emerald");
        assert_eq!((views[0].count, views[0].glint), (3, false));
        for _ in 0..16 {
            displays.tick(&scene, 0, &nobody, &mut particles);
        }
        let turned = displays.items(EYE, 0.0)[0].spin;
        assert_eq!(turned, -180.0, "it wraps around at 180 degrees");
        assert_eq!(displays.items(EYE, 0.5)[0].spin, -175.0);
        assert!(
            displays.items(DVec3::new(4.5, 64.0, 70.0), 0.0).is_empty(),
            "64 blocks away"
        );
        displays.block_entities(&[(AT, Some(vault_tag(None, &[])))]);
        assert!(
            displays.items(EYE, 0.0).is_empty(),
            "a dim vault shows nothing"
        );
        assert_eq!(displays.len(), 1);
    }

    #[test]
    fn idle_flames_burn_only_while_an_item_shows_and_turn_soul_blue_when_ominous() {
        let mut particles = AmbientParticles::new(1);
        let mut dim = shown(None, &[]);
        for tick in 1..=400 {
            dim.tick(&vault_scene(false), tick, &nobody, &mut particles);
        }
        let smoke = count(&particles, Kind::Smoke);
        assert!(
            (150..=250).contains(&smoke),
            "smoke half the ticks: {smoke}"
        );
        assert_eq!(count(&particles, Kind::SmallFlame), 0);
        for (_, at) in particles.kinds_at() {
            let inside = at - DVec3::new(4.0, 64.0, 4.0);
            assert!((0.1..=0.9).contains(&inside.x) && (0.25..=0.75).contains(&inside.y));
            assert!((0.1..=0.9).contains(&inside.z));
        }
        for (ominous, kind) in [(false, Kind::SmallFlame), (true, Kind::SoulFireFlame)] {
            let mut particles = AmbientParticles::new(1);
            let mut lit = shown(Some(ItemStack::new("minecraft:diamond", 1)), &[]);
            for tick in 1..=400 {
                lit.tick(&vault_scene(ominous), tick, &nobody, &mut particles);
            }
            assert_eq!(count(&particles, kind), count(&particles, Kind::Smoke));
        }
        let mut gone = shown(Some(ItemStack::new("minecraft:diamond", 1)), &[]);
        let mut particles = AmbientParticles::new(1);
        gone.tick(&Blocks(HashMap::new()), 1, &nobody, &mut particles);
        assert!(particles.is_empty(), "no vault block, nothing");
    }

    #[test]
    fn a_lit_vault_hums_about_two_ticks_in_a_hundred() {
        let mut lit = shown(Some(ItemStack::new("minecraft:diamond", 1)), &[]);
        let mut dim = shown(None, &[]);
        let mut particles = AmbientParticles::new(1);
        let (mut hums, mut quiet) = (Vec::new(), 0);
        for tick in 1..=5000 {
            hums.extend(lit.tick(&vault_scene(false), tick, &nobody, &mut particles));
            quiet += dim
                .tick(&vault_scene(false), tick, &nobody, &mut particles)
                .len();
        }
        assert!((60..=140).contains(&hums.len()), "{}", hums.len());
        assert_eq!(quiet, 0);
        for hum in hums {
            assert_eq!((hum.event, hum.at), (AMBIENT_SOUND, [4.5, 64.5, 4.5]));
            assert!((0.75..1.0).contains(&hum.volume) && (0.5..1.5).contains(&hum.pitch));
        }
    }

    #[test]
    fn connection_particles_fly_from_the_keyhole_to_its_players_once_a_second() {
        let near = DVec3::new(6.5, 64.0, 7.5);
        let far = DVec3::new(9.5, 64.0, 4.5);
        let players = |uuid: u128| match uuid {
            1 => Some((near, 1.8)),
            2 => Some((far, 1.8)),
            _ => None,
        };
        let mut displays = shown(None, &[1, 2, 3]);
        let scene = vault_scene(false);
        let mut particles = AmbientParticles::new(1);
        displays.tick(&scene, 41, &players, &mut particles);
        assert_eq!(
            count(&particles, Kind::VaultConnection),
            0,
            "only on the 20-tick phase"
        );
        displays.tick(&scene, 40, &players, &mut particles);
        let made = count(&particles, Kind::VaultConnection);
        assert!((2..=5).contains(&made), "{made}");
        let keyhole = keyhole(AT, scene.block(AT));
        assert_eq!(keyhole, DVec3::new(4.5, 65.75, 5.0));
        let mut activated = AmbientParticles::new(1);
        displays.activation(AT, &scene, &players, &mut activated);
        let made = count(&activated, Kind::VaultConnection);
        assert!((2..=5).contains(&made), "the activation event too: {made}");
    }

    #[test]
    fn vaults_load_with_chunks_and_follow_their_block_entity() {
        let mut chunk = minecraftoss_core::Chunk::new(
            minecraftoss_core::ChunkPos::new(0, 0),
            -64,
            384,
            minecraftoss_core::BiomeId(0),
        );
        chunk
            .block_entities
            .entities
            .insert(AT, vault_tag(Some(ItemStack::new("minecraft:bow", 1)), &[]));
        let mut other = BTreeMap::new();
        other.insert("id".to_owned(), Tag::String("minecraft:chest".to_owned()));
        chunk
            .block_entities
            .entities
            .insert((5, 64, 4), Tag::Compound(other.clone()));
        let mut displays = VaultDisplays::new(1);
        displays.load_chunk(&chunk);
        assert_eq!(displays.len(), 1);
        displays.block_entities(&[(AT, Some(Tag::Compound(other)))]);
        assert!(displays.is_empty(), "another block entity replaced it");
        displays.load_chunk(&chunk);
        displays.block_entities(&[(AT, None)]);
        assert!(displays.is_empty());
        displays.load_chunk(&chunk);
        displays.unload_chunk(minecraftoss_core::ChunkPos::new(0, 0));
        assert!(displays.is_empty());
        let mut enchanted = ItemStack::new("minecraft:bow", 1);
        let mut components = BTreeMap::new();
        let mut levels = BTreeMap::new();
        levels.insert("minecraft:power".to_owned(), Tag::Int(2));
        components.insert("minecraft:enchantments".to_owned(), Tag::Compound(levels));
        enchanted.components = Some(Tag::Compound(components));
        let lit = shown(Some(enchanted), &[]);
        assert!(lit.items(EYE, 0.0)[0].glint);
    }

    #[test]
    fn a_cluster_draws_more_copies_for_bigger_stacks() {
        let amounts: Vec<usize> = [0, 1, 2, 16, 17, 32, 33, 48, 49, 64]
            .into_iter()
            .map(rendered_amount)
            .collect();
        assert_eq!(amounts, [1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
        let solid = cluster_offsets("minecraft:emerald_block", 64, false);
        assert_eq!(solid.len(), 5);
        assert_eq!(solid[0], Vec3::ZERO);
        assert!(
            solid[1..]
                .iter()
                .all(|offset| offset.abs().max_element() <= 0.15)
        );
        assert!(solid[1..].iter().any(|offset| offset.z != 0.0));
        let flat = cluster_offsets("minecraft:arrow", 8, true);
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[1].z, 0.0);
        assert_eq!(
            cluster_offsets("minecraft:arrow", 8, true),
            flat,
            "the same every frame"
        );
    }

    fn bar_pack(root: &std::path::Path) -> crate::pack::PackStack {
        for (name, contents) in [
            (
                "pack.mcmeta",
                r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
            ),
            (
                "assets/test/items/bar.json",
                r#"{"model":{"type":"minecraft:model","model":"test:item/bar"}}"#,
            ),
            (
                "assets/test/models/item/bar.json",
                r#"{"parent":"builtin/generated","textures":{"layer0":"test:item/bar"}}"#,
            ),
        ] {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let sprite = root.join("assets/test/textures/item/bar.png");
        std::fs::create_dir_all(sprite.parent().unwrap()).unwrap();
        let mut image = image::RgbaImage::new(16, 16);
        for x in 2..14 {
            image.put_pixel(x, 8, image::Rgba([200, 150, 100, 255]));
        }
        image.save(sprite).unwrap();
        crate::pack::PackStack::open(vec![root.into()]).unwrap()
    }

    #[test]
    fn the_reward_item_is_drawn_in_the_cage_turned_with_its_copies_and_glint() {
        let temp = crate::test_dir::tempdir().unwrap();
        let packs = bar_pack(temp.path());
        let atlas = crate::mesh::make_atlas(
            &packs,
            vec![crate::pack::ResourceId::parse("test:item/bar").unwrap()],
        )
        .unwrap();
        let light = crate::lighting::SkyLight::build(&crate::scene::HandcraftedScene::new());
        let centre = DVec3::new(4.5, 64.4, 4.5);
        let view = |count: i32, spin: f32, glint: bool| VaultItemView {
            at: centre,
            spin,
            item: "test:bar".to_owned(),
            count,
            glint,
        };
        let draw = |views: &[VaultItemView]| {
            let mut mesh = crate::mesh::ChunkMesh::default();
            crate::mesh::ItemVisuals::default()
                .append_vault_items(&mut mesh, views, &packs, &atlas, &light)
                .unwrap();
            mesh
        };
        let extent = |mesh: &crate::mesh::ChunkMesh, axis: usize| {
            let values = mesh.vertices.iter().map(|vertex| vertex.position[axis]);
            values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
        };
        let one = draw(&[view(1, 0.0, false)]);
        assert!(!one.vertices.is_empty() && one.glint.is_empty());
        for vertex in &one.vertices {
            let offset = Vec3::from_array(vertex.position) - centre.as_vec3();
            assert!(offset.abs().max_element() < 1.0, "{offset:?}");
        }
        let turned = draw(&[view(1, 90.0, false)]);
        assert!(extent(&one, 0) > extent(&one, 2) * 4.0);
        assert!(
            (extent(&turned, 2) - extent(&one, 0)).abs() < 1.0e-4,
            "a quarter turn swaps x and z"
        );
        assert_eq!(
            draw(&[view(20, 0.0, false)]).vertices.len(),
            one.vertices.len() * 3
        );
        let shiny = draw(&[view(1, 0.0, true)]);
        assert_eq!(shiny.glint, [0..shiny.vertices.len() as u32]);
    }
}
