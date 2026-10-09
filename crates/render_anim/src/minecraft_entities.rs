//! The Minecraft world's mobs, as MinecraftOSS runs them: its integrated
//! server (`engine/viewer/src/server.rs`) ticks the level at 20 Hz on its own
//! thread with natural spawning and the entity world, every passive and
//! hostile mob's AI and pathfinding. Here it is fed the streamed chunks and
//! the player, and hands back the tracked mobs, which are drawn with the
//! viewer's mob renderers. MW2 bullets hurt the mobs with their real damage
//! (a hundred MW2 health to Minecraft's twenty), and mobs hurt the player
//! the other way round.
use glam::{DVec3, Vec3};
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{Atlas, ChunkMesh};
use minecraft_terrain::scene::{Block, HandcraftedScene, Scene};
use minecraft_terrain::server::{HowBroken, PlayerEdit, ServerHandle, ServerSim, TickInput};
use minecraft_terrain::terrain::TerrainStream;
use minecraft_terrain::client_mobs::{ClientMobs, server_mobs};
use minecraft_terrain::mesh::ItemVisuals;
use minecraft_terrain::server::{EntitySnapshot, ServerItem, ServerOrb};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::items::{ItemEntity, WorldItems};
use minecraftoss_player::loot::LootBook;
use minecraftoss_player::rng::XoroshiroRandom;
use std::collections::HashMap;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::poof_particles::PoofParticles;
use minecraft_terrain::portal_particles::PortalParticles;
use minecraftoss_entities::tempt::PlayerCandidate;
use minecraftoss_entities::world::{EntityWorld, MobHit, PlayerHitKind};

const TICK_SECONDS: f64 = 1.0 / 20.0;
/// Minecraft health per MW2 health.
const HEALTH_SCALE: f32 = 20.0 / 100.0;
const BLAST_HEALTH_SCALE: f32 = 80.0 / 100.0;
/// The player's id in the entity world.
const PLAYER: u64 = 0;
/// `Player.getEyeHeight` standing.
const EYE_HEIGHT: f32 = 1.62;
const SERVER_ITEM: u32 = 1 << 31;
type Hits = Vec<(f32, Option<[f64; 3]>, Option<[f64; 2]>, minecraft_survival::death::DamageCause)>;
type Changes = Vec<((i32, i32, i32), Option<Block>)>;

pub(crate) struct Blow {
    pub sprinting: bool,
    pub can_critical: bool,
    pub can_sweep: bool,
    pub reach: f64,
}

pub(crate) struct SentItemUse {
    pub slot: usize,
    pub held: String,
    pub eye: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub outcome: minecraft_terrain::bucket::ItemUse,
}

pub(crate) struct Entities {
    server: ServerHandle,
    /// The mobs the player tracks, as of the last server tick.
    world: EntityWorld,
    /// The client's copies of them: tracked, interpolated, animated.
    client: ClientMobs,
    poof: PoofParticles,
    portal: PortalParticles,
    items: ItemVisuals,
    clock: f64,
    ticks: u64,
    /// The player's inventory: vanilla slots, stacking and recipes.
    pub(crate) inventory: Inventory,
    /// The selected hotbar slot.
    pub(crate) selected: usize,
    /// Item entities as the client shows them: the server's, and drops the
    /// client spawned that the server has not taken yet.
    pub(crate) world_items: WorldItems,
    /// Client drops handed to the server: the command count that sent each.
    server_handed: HashMap<u32, (u64, ItemEntity)>,
    burning_items: Vec<u32>,
    /// Stacks the server let the player pick up, not yet in the inventory.
    server_picked: Vec<(i32, [f64; 3], String, i32, Option<String>)>,
    server_snapshot: Option<EntitySnapshot>,
    server_handled: u64,
    loot: Option<LootBook>,
    loot_sequences: HashMap<String, XoroshiroRandom>,
    seed: i64,
    /// Sounds the mob world made since the last take: event, block point,
    /// volume, pitch.
    pub(crate) sounds: Vec<(String, DVec3, f32, f32)>,
    random: minecraftoss_player::rng::LegacyRandom,
    thrown: Vec<(String, [f64; 3], [f64; 3], i32)>,
    orbs: Vec<ServerOrb>,
    falling: minecraft_terrain::falling_render::FallingBlocks,
    orb_pickups: Vec<minecraft_terrain::orb_render::OrbPickup>,
    pub(crate) pearl_landings: Vec<DVec3>,
    pub(crate) destroyed: Vec<((i32, i32, i32), Block)>,
    pub(crate) experience_found: f32,
    pub(crate) weather: minecraft_terrain::server::WeatherView,
    pub(crate) stations: std::sync::Arc<minecraft_terrain::stations::StationBook>,
    pub(crate) enchantments: Option<std::sync::Arc<minecraftoss_entities::enchant::Enchantments>>,
    item_use_waiting: Option<SentItemUse>,
    pub(crate) item_uses_done: Vec<SentItemUse>,
    eggs_waiting: minecraft_terrain::spawn_egg::EggsWaiting,
    pub(crate) command_summons: Vec<Result<String, String>>,
    pub(crate) difficulty: i32,
    spawn_mobs: bool,
    block_drops: bool,
}

/// What bullets and blasts count as for block loot: vanilla drops nothing
/// from stone or ores broken bare-handed.
const LOOT_TOOL: &str = "minecraft:diamond_pickaxe";
const SILK_TOUCH: &str = "minecraft:silk_touch";
type BrokenBlock = ((i32, i32, i32), Block, Option<f32>);

/// The mobs drawn this frame: entity models (cut out, back-face culled,
/// translucent) and their shadows, in `mesh::Vertex`s; and what goes with
/// the particles (held items, puffs, flames, potions).
#[derive(Default)]
pub(crate) struct MobMeshes {
    pub models: ChunkMesh,
    pub culled: ChunkMesh,
    pub translucent: ChunkMesh,
    pub shadows: ChunkMesh,
    pub items: ChunkMesh,
}

/// The player as the mobs see it this frame.
pub(crate) struct PlayerView {
    pub feet: [f64; 3],
    pub alive: bool,
    pub attackable: bool,
    pub spectator: bool,
    pub picks_up: bool,
    pub health: f32,
    pub yaw: f32,
    pub pitch: f32,
}

fn player_candidate(player: &PlayerView) -> PlayerCandidate {
    PlayerCandidate {
        id: PLAYER,
        position: DVec3::from_array(player.feet),
        eye_height: EYE_HEIGHT,
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
        alive: player.alive,
        spectator: player.spectator,
        attackable: player.attackable,
    }
}

impl Entities {
    pub(crate) fn new(stream: &TerrainStream, seed: i64, root: Option<&std::path::Path>, jar: Option<std::path::PathBuf>) -> Self {
        let mut sim = ServerSim::new(stream.world_gen(), stream.states.clone(), "minecraft:overworld");
        sim.set_entity_storage(stream.storage());
        if let Ok(weather) = std::env::var("IW4L_MINECRAFT_WEATHER") {
            if sim.set_weather(weather.trim()) {
                diag::info!(World, "Minecraft weather: {} from IW4L_MINECRAFT_WEATHER", weather.trim());
            } else {
                diag::warn!(World, "Minecraft weather `{weather}` unknown: IW4L_MINECRAFT_WEATHER takes clear, rain or thunder");
            }
        }
        let mut server = ServerHandle::spawn(sim);
        // Mob and block loot and the recipes (stack sizes), from the game's
        // data JAR when MinecraftOSS has one.
        let mut inventory = Inventory::default();
        let mut loot = None;
        let mut stations = std::sync::Arc::default();
        let mut enchantments = None;
        if let Some(jar) = jar {
            server.load_loot(jar.clone(), seed as u64);
            let catalog = minecraft_terrain::crafting::find_item_catalog(root, &jar);
            match minecraft_terrain::crafting::load_recipes(&jar, catalog.as_deref()) {
                Ok((recipes, catalog_error)) => {
                    match (catalog_error, catalog.as_deref()) {
                        (Some(error), _) => diag::warn!(World, "Minecraft item catalog unavailable, stacks of 64 and no crafting remainders: {error:#}"),
                        (None, Some(path)) => diag::info!(World, "Minecraft recipes: {} with the item catalog {}", recipes.count(), path.display()),
                        (None, None) => {}
                    }
                    let recipes = std::sync::Arc::new(recipes);
                    server.set_recipe_book(recipes.clone());
                    inventory.recipes = recipes;
                }
                Err(error) => diag::warn!(World, "Minecraft recipes unavailable: {error:#}"),
            }
            match LootBook::from_jar(&jar) {
                Ok(book) => loot = Some(book),
                Err(error) => diag::warn!(World, "Minecraft block loot unavailable: {error:#}"),
            }
            match minecraft_terrain::stations::StationBook::from_jar(&jar) {
                Ok(book) => stations = std::sync::Arc::new(book),
                Err(error) => diag::warn!(World, "Minecraft station recipes unavailable: {error:#}"),
            }
            if let Some(catalog) = catalog.as_deref() {
                match minecraftoss_entities::trading::TradeBook::from_jar(&jar, catalog) {
                    Ok(book) => enchantments = Some(std::sync::Arc::new(book.enchantments)),
                    Err(error) => diag::warn!(World, "Minecraft enchantments unavailable: {error:#}"),
                }
            }
        } else {
            diag::warn!(World, "Minecraft data JAR not found: no loot or recipes");
        }
        if let Ok(kit) = std::env::var("IW4L_MINECRAFT_KIT") {
            let given = minecraft_terrain::crafting::give_kit(&mut inventory, &kit);
            diag::info!(World, "Minecraft kit: {given} stacks from IW4L_MINECRAFT_KIT");
        }
        Self {
            server,
            world: EntityWorld::default(),
            client: ClientMobs::default(),
            poof: PoofParticles::default(),
            portal: PortalParticles::default(),
            items: ItemVisuals::default(),
            clock: 0.0,
            ticks: 0,
            inventory,
            selected: 0,
            world_items: WorldItems::default(),
            server_handed: HashMap::new(),
            burning_items: Vec::new(),
            server_picked: Vec::new(),
            server_snapshot: None,
            server_handled: 0,
            loot,
            loot_sequences: HashMap::new(),
            seed,
            sounds: Vec::new(),
            random: minecraftoss_player::rng::LegacyRandom::new((seed ^ 0x1735) as u64),
            thrown: Vec::new(),
            orbs: Vec::new(),
            falling: Default::default(),
            orb_pickups: Vec::new(),
            pearl_landings: Vec::new(),
            destroyed: Vec::new(),
            experience_found: 0.0,
            weather: Default::default(),
            stations,
            enchantments,
            item_use_waiting: None,
            item_uses_done: Vec::new(),
            eggs_waiting: Default::default(),
            command_summons: Vec::new(),
            difficulty: 2,
            spawn_mobs: true,
            block_drops: true,
        }
    }

    pub(crate) fn set_game_rules(&mut self, rules: &minecraft_terrain::game_rules::GameRules) {
        self.spawn_mobs = rules.bool("spawn_mobs");
        self.block_drops = rules.bool("block_drops");
        self.server.set_game_rules(rules.clone());
    }

    pub(crate) fn set_weather(&mut self, weather: &'static str, duration: i32) {
        self.server.set_weather(weather, duration);
    }

    pub(crate) fn throw(&mut self, item: &str, eye: DVec3, yaw: f32, pitch: f32, motion: DVec3, on_ground: bool) -> bool {
        let Some(thrown) = minecraft_terrain::thrown::Thrown::throw(item, eye, yaw, pitch, motion, on_ground, &mut self.random) else {
            return false;
        };
        self.server.throw(thrown);
        true
    }

    pub(crate) fn random_float(&mut self) -> f32 {
        self.random.next_float()
    }

    pub(crate) fn load_chunk(&mut self, chunk: &std::sync::Arc<minecraftoss_core::Chunk>) {
        self.server.load_chunk(chunk);
    }

    pub(crate) fn unload_chunk(&mut self, pos: minecraftoss_core::ChunkPos) {
        self.server.unload_chunk(pos);
    }

    pub(crate) fn award_experience(&mut self, at: [f64; 3], amount: i32) {
        self.server.award_experience(at, amount);
    }

    pub(crate) fn award_block_experience(&mut self, broken: &[BrokenBlock], by_hand: &[(i32, i32, i32)]) {
        if !self.block_drops {
            return;
        }
        let silk_touch = self.hand_has_silk_touch();
        for (pos, block, blast) in broken {
            if silk_touch && blast.is_none() && by_hand.contains(pos) {
                continue;
            }
            let experience = minecraft_terrain::experience::block_experience(block, true, |bound| self.random.next_int(bound));
            if experience > 0 {
                let centre = [f64::from(pos.0) + 0.5, f64::from(pos.1) + 0.5, f64::from(pos.2) + 0.5];
                self.server.award_experience(centre, experience as i32);
            }
        }
    }

    /// Block loot for what the weapons broke, dropped as vanilla drops it
    /// (`Block.popResource`); a blast keeps each drop one time in its power.
    pub(crate) fn drop_blocks(&mut self, broken: &[BrokenBlock], mut kept: HashMap<(i32, i32, i32), serde_json::Value>, by_hand: &[(i32, i32, i32)]) {
        if !self.block_drops {
            return;
        }
        let tool = ItemStack::new(LOOT_TOOL, 1);
        let hand_tool = self.hand_tool();
        for (pos, block, blast) in broken {
            let block = minecraftoss_player::Block { id: block.id.key(), properties: block.properties.clone() };
            let tool = if blast.is_none() && by_hand.contains(pos) { &hand_tool } else { &tool };
            let drops = self
                .loot
                .as_ref()
                .and_then(|loot| loot.roll_drops_named(&block, Some(tool), self.seed as u64, &mut self.loot_sequences));
            let survives_blast = minecraft_terrain::containers::is_shulker_box(&block.id);
            for mut drop in drops.into_iter().flatten() {
                if let Some(power) = *blast
                    && !survives_blast
                    && self.random.next_float() >= 1.0 / power
                {
                    continue;
                }
                if drop.components.is_none() {
                    drop.max = drop.max.min(self.inventory.recipes.max_stack(&drop.id));
                }
                if drop.id == block.id
                    && let Some(components) = kept.remove(pos)
                {
                    drop.components = Some(components);
                }
                self.world_items.spawn_block_drop(drop, *pos);
            }
            if let Some(components) = kept.remove(pos) {
                let mut boxed = ItemStack::new(block.id.clone(), 1);
                boxed.max = boxed.max.min(self.inventory.recipes.max_stack(&boxed.id));
                boxed.components = Some(components);
                self.world_items.spawn_block_drop(boxed, *pos);
            }
        }
    }

    /// `ItemEntity`s to draw and pick up, as the viewer's
    /// `server_items_tick` does for a server-simulated world: client drops
    /// go to the server, stacks the server offered go into the inventory,
    /// and the server's items are mirrored for drawing.
    fn server_items_tick(&mut self, feet: DVec3) {
        let entities = std::mem::take(&mut self.world_items.entities);
        let to_hand: Vec<ItemEntity> = entities
            .iter()
            .filter(|e| e.entity_id & SERVER_ITEM == 0 && !self.server_handed.contains_key(&e.entity_id))
            .cloned()
            .collect();
        for entity in &to_hand {
            let components = entity.stack.components.as_ref().map(|c| c.to_string());
            self.server.spawn_item(
                &entity.stack.id,
                i32::from(entity.stack.count),
                components.as_deref(),
                entity.position.to_array(),
                entity.velocity.to_array(),
                i32::from(entity.pickup_delay),
                entity.age as i32,
            );
            self.server_handed.insert(entity.entity_id, (self.server.sent(), entity.clone()));
        }
        let previous: HashMap<u32, ItemEntity> = entities.into_iter().map(|e| (e.entity_id, e)).collect();
        let target = feet + DVec3::Y * 0.81;
        self.world_items.tick_pickup_effects(target);
        for (id, position, item, count, components) in std::mem::take(&mut self.server_picked) {
            let recipes = self.inventory.recipes.clone();
            let make_stack = |item: &str, count: i32| {
                let mut stack = ItemStack::new(item, count.clamp(0, 255) as u8);
                stack.components = components.as_deref().and_then(|c| serde_json::from_str(c).ok());
                stack.max = stack.max.min(recipes.max_stack(item));
                stack
            };
            let taken = match self.inventory.add_item(make_stack(&item, count), self.selected) {
                None => count,
                Some(rest) => {
                    let rest_count = i32::from(rest.count);
                    self.server.spawn_item(&item, rest_count, components.as_deref(), feet.to_array(), [0.0; 3], 0, 0);
                    count - rest_count
                }
            };
            if taken <= 0 {
                continue;
            }
            let transfer = make_stack(&item, taken);
            let snapshot = previous.get(&mirrored(id)).cloned().unwrap_or_else(|| ItemEntity {
                entity_id: mirrored(id),
                stack: transfer.clone(),
                position: DVec3::from_array(position),
                previous_position: DVec3::from_array(position),
                velocity: DVec3::ZERO,
                age: 0,
                bob_offset: bob_offset(id),
                pickup_delay: 0,
                on_ground: true,
            });
            self.world_items.note_pickup(snapshot, target, transfer);
        }
        let _ = self.world_items.take_pickup_transfers();
        let Some(snapshot) = self.server_snapshot.take() else {
            self.world_items.entities = previous.into_values().collect();
            self.world_items.entities.sort_by_key(|e| e.entity_id);
            return;
        };
        let recipes = self.inventory.recipes.clone();
        self.burning_items = snapshot.items.iter().filter(|item| item.on_fire).map(|item| mirrored(item.id)).collect();
        self.world_items.entities = snapshot
            .items
            .into_iter()
            .map(|item: ServerItem| {
                let mut stack = ItemStack::new(&item.item, item.count.clamp(0, 255) as u8);
                stack.components = item.components.as_deref().and_then(|c| serde_json::from_str(c).ok());
                stack.max = stack.max.min(recipes.max_stack(&item.item));
                ItemEntity {
                    entity_id: mirrored(item.id),
                    stack,
                    position: DVec3::from_array(item.position),
                    previous_position: DVec3::from_array(item.previous_position),
                    velocity: DVec3::from_array(item.velocity),
                    age: item.age.max(0) as u32,
                    bob_offset: bob_offset(item.id),
                    pickup_delay: item.pickup_delay.clamp(0, i32::from(u16::MAX)) as u16,
                    on_ground: item.on_ground,
                }
            })
            .collect();
        let handled = self.server_handled;
        self.server_handed.retain(|_, (sent, _)| *sent > handled);
        for (_, entity) in self.server_handed.values() {
            self.world_items.entities.push(entity.clone());
        }
    }

    /// A held item's model under a view-space pose, lit at `light_at`.
    pub(crate) fn held_item_mesh(
        &mut self,
        posed: &[(glam::Mat4, String, bool)],
        light_at: glam::Vec3,
        packs: &PackStack,
        atlas: &Atlas,
        light: &SkyLight,
    ) -> ChunkMesh {
        let mut mesh = ChunkMesh::default();
        for (pose, id, glint) in posed {
            let start = mesh.vertices.len() as u32;
            let _ = self.items.append_posed_items(&mut mesh, &[(*pose, light_at, id.clone())], packs, atlas, light);
            if *glint {
                mesh.glint.push(start..mesh.vertices.len() as u32);
            }
        }
        mesh
    }

    pub(crate) fn punch(&mut self, eye: DVec3, look: DVec3, yaw: f32, attack_damage: f64, strength: f32, blow: Blow) -> bool {
        let Some((hit, _)) = self.world.mob_on_ray(eye, look, blow.reach) else {
            return false;
        };
        let attack = minecraftoss_entities::world::PlayerAttack {
            player_id: PLAYER,
            position: eye - DVec3::Y * f64::from(EYE_HEIGHT),
            yaw,
            attack_damage,
            strength,
            sprinting: blow.sprinting,
            can_critical: blow.can_critical,
            can_sweep: blow.can_sweep,
        };
        self.server.mob_action(hit, Some(attack), &self.inventory.clone(), self.selected, false);
        true
    }

    pub(crate) fn mob_distance(&self, eye: DVec3, look: DVec3, reach: f64) -> Option<f64> {
        self.world.mob_on_ray(eye, look, reach).map(|(_, distance)| distance)
    }

    /// A block the player placed, for the level the mobs walk in.
    pub(crate) fn placed(&mut self, scene: &HandcraftedScene, pos: (i32, i32, i32)) {
        self.server.player_edit(scene, pos, PlayerEdit::Place);
    }

    pub(crate) fn keep_dropped_contents(&mut self, pos: (i32, i32, i32), id: &str, components: &serde_json::Value) {
        self.server.keep_dropped_contents(pos, id, &components.to_string());
    }

    pub(crate) fn use_block(&mut self, scene: &HandcraftedScene, pos: (i32, i32, i32), facing: &'static str) -> bool {
        self.server.use_block(scene, pos, facing)
    }

    pub(crate) fn item_on_block(&mut self, pos: (i32, i32, i32), action: minecraft_terrain::server::ItemOnBlock, sent: SentItemUse) {
        self.server.item_on_block(pos, action);
        self.item_use_waiting = Some(sent);
    }

    pub(crate) fn awaits_item_use(&self) -> bool {
        self.item_use_waiting.is_some()
    }

    pub(crate) fn hatch_egg(&mut self, slot: usize, kind: String, position: [f64; 3], spends: bool) {
        if self.eggs_waiting.send_from(&self.inventory, slot, spends) {
            let y_rot = minecraftoss_core::entity_data::wrap_degrees(self.random.next_float() * 360.0);
            self.server.hatch_egg(kind, position, y_rot);
        }
    }

    pub(crate) fn summon(&mut self, kind: String, position: [f64; 3], nbt: Option<minecraftoss_core::nbt::Tag>) {
        self.server.summon(kind, position, nbt);
    }

    pub(crate) fn drop_given(&mut self, stack: &ItemStack, feet: [f64; 3]) {
        let components = stack.components.as_ref().map(|c| c.to_string());
        self.server.spawn_item(&stack.id, i32::from(stack.count), components.as_deref(), feet, [0.0; 3], 0, 0);
    }

    pub(crate) fn animate_tick<S: minecraft_terrain::scene::Scene + ?Sized>(&mut self, scene: &S, feet: [f64; 3]) -> Vec<minecraft_terrain::display_ticks::AmbientSound> {
        let at = (feet[0].floor() as i32, feet[1].floor() as i32, feet[2].floor() as i32);
        let tick = minecraft_terrain::display_ticks::animate_tick(scene, at, &mut self.random);
        for (start, motion) in tick.portal_particles {
            self.portal.emit(start, motion);
        }
        tick.sounds
    }

    /// Client ticks run so far.
    pub(crate) fn client_ticks(&self) -> u64 {
        self.ticks
    }

    /// Blocks the player's weapons broke, for the level the mobs walk in.
    pub(crate) fn broke(&mut self, positions: &[(i32, i32, i32)]) {
        self.server.player_breaks(positions);
    }

    pub(crate) fn mined(&mut self, broken: &[BrokenBlock], by_hand: &[(i32, i32, i32)], creative: bool) {
        let silk_touch = self.hand_has_silk_touch();
        let how: Vec<_> = broken.iter().map(|(pos, _, blast)| (*pos, how_broken(*pos, *blast, by_hand, silk_touch, creative))).collect();
        self.server.player_mines(&how);
    }

    fn hand_enchantments(&self) -> Option<serde_json::Value> {
        self.inventory.slots[self.selected].as_ref()?.components.as_ref()?.get("minecraft:enchantments").cloned()
    }

    fn hand_has_silk_touch(&self) -> bool {
        has_silk_touch(self.hand_enchantments().as_ref())
    }

    fn hand_tool(&self) -> ItemStack {
        loot_tool_with(self.hand_enchantments())
    }

    /// A bullet on a mob: an attack with the bullet's damage from where it
    /// was fired.
    pub(crate) fn shoot(&mut self, key: u64, damage: f32, from: [f64; 3], yaw: f32) {
        let Some(hit) = decode(key) else {
            return;
        };
        let attack = minecraftoss_entities::world::PlayerAttack {
            player_id: PLAYER,
            position: DVec3::from_array(from),
            yaw,
            attack_damage: f64::from(damage * HEALTH_SCALE),
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        };
        self.server
            .mob_action(hit, Some(attack), &minecraftoss_player::inventory::Inventory::default(), 0, false);
    }

    pub(crate) fn blast(&mut self, key: u64, damage: f32, center: [f64; 3], impact: f32, by_player: bool, slow_ticks: i32) {
        let Some(hit) = decode(key) else {
            return;
        };
        self.server.mob_blast(hit, damage * BLAST_HEALTH_SCALE, center, impact, by_player.then_some(PLAYER), slow_ticks);
    }

    pub(crate) fn weapon_blast(&mut self, center: [f64; 3], power: f32) {
        self.server.weapon_blast(center, power);
    }

    /// Sends a server tick when one is due and takes what came back: block
    /// changes to show, and mob hits on the player in MW2 damage.
    pub(crate) fn tick(
        &mut self,
        dt: f64,
        day_ticks: i64,
        bright_outside: bool,
        player: &PlayerView,
    ) -> (Changes, Hits) {
        self.clock += dt;
        let orb_pickup_target = DVec3::from_array(player.feet) + DVec3::Y * f64::from(EYE_HEIGHT / 2.0);
        let ticked = self.clock >= TICK_SECONDS;
        if ticked {
            self.clock = (self.clock - TICK_SECONDS).min(TICK_SECONDS);
            self.ticks += 1;
            self.falling.tick_sent();
            // Packets first, then the client level's entity ticks.
            self.client.tick();
            for enderman in self.world.endermen() {
                if let Some(mob) = self.client.get(enderman.id) {
                    self.portal.emit_enderman(mob.position, 0.6, 2.9);
                }
            }
            self.portal.tick();
            minecraft_terrain::orb_render::tick_pickups(&mut self.orb_pickups, orb_pickup_target);
            if self.ticks % 600 == 0 {
                diag::info!(World, "Minecraft mobs: {} tracked", self.boxes().len());
            }
            let feet = DVec3::from_array(player.feet);
            let center = ((feet.x.floor() as i32) >> 4, (feet.z.floor() as i32) >> 4);
            let candidate = player_candidate(player);
            self.server.tick(TickInput {
                day_ticks,
                players: if player.alive { vec![player.feet] } else { Vec::new() },
                difficulty: self.difficulty,
                simulation_center: center,
                simulation_distance: 8,
                pickup: player
                    .picks_up
                    .then(|| (player.feet, Box::new(self.inventory.clone()), self.selected)),
                mob_players: vec![candidate],
                mob_views: vec![(
                    PLAYER,
                    minecraftoss_entities::enderman::PlayerView {
                        head_yaw: player.yaw,
                        pitch: player.pitch,
                        disguised: false,
                    },
                )],
                mob_vitals: vec![(
                    PLAYER,
                    minecraftoss_entities::monster_ai::PlayerVitals {
                        health: player.health,
                        ..Default::default()
                    },
                )],
                bright_outside,
                tracking: (player.feet, 160.0),
                player_hurts: Vec::new(),
                spawn_mobs: self.spawn_mobs,
            });
        }
        let mut changes = Vec::new();
        let mut hits = Vec::new();
        for output in self.server.poll() {
            self.server_handled = output.handled;
            for &(id, position, value) in &output.orbs_taken {
                let tick_count = self.orbs.iter().find(|orb| orb.id == id).map_or(0, |orb| orb.tick_count);
                let orb = ServerOrb { id, position, previous_position: position, value, tick_count };
                self.orb_pickups.push(minecraft_terrain::orb_render::OrbPickup::new(orb, orb_pickup_target));
            }
            if let Some(mut snapshot) = output.entities {
                self.thrown = snapshot.thrown.clone();
                self.orbs = snapshot.orbs.clone();
                self.falling.receive(std::mem::take(&mut snapshot.falling), std::mem::take(&mut snapshot.tnt), &output.changes, self.ticks);
                self.server_snapshot = Some(snapshot);
            }
            for landing in output.pearl_landings {
                let at = DVec3::from_array(landing);
                for _ in 0..16 {
                    self.portal.emit_enderman(at, 0.6, 1.8);
                }
                self.pearl_landings.push(at);
            }
            if let Some(weather) = output.weather {
                self.weather = weather;
            }
            self.server_picked.extend(output.picked);
            for used in output.item_uses {
                if let Some(sent) = self.item_use_waiting.take().filter(|_| used) {
                    self.item_uses_done.push(sent);
                }
            }
            for summoned in output.summoned {
                let egg = self.eggs_waiting.answer(&mut self.inventory, summoned.is_ok());
                if let (Some(egg), Err(error)) = (egg, summoned) {
                    diag::warn!(World, "Minecraft {egg} hatched nothing: {error}");
                }
            }
            self.command_summons.extend(output.command_summons);
            changes.extend(output.changes);
            self.destroyed.extend(output.destroyed);
            self.experience_found += output.orbs_taken.iter().map(|orb| orb.2 as f32).sum::<f32>();
            for sound in &output.mob_sounds {
                self.sounds.push((sound.event.clone(), sound.position, sound.volume, sound.pitch));
            }
            for sound in output.mob_results.iter().flat_map(|result| result.sounds.iter()) {
                self.sounds.push((sound.event.clone(), sound.position, sound.volume, sound.pitch));
            }
            // `ServerExplosion`'s sound: loud, pitched down.
            for blast in &output.explosions {
                let pitch = (1.0 + (self.random.next_float() - self.random.next_float()) * 0.2) * 0.7;
                self.sounds.push(("minecraft:entity.generic.explode".to_owned(), blast.position, 4.0, pitch));
            }
            if let Some(mobs) = output.mobs {
                self.world = *mobs;
                for (feet, width, height) in self.client.receive(server_mobs(&self.world)) {
                    self.poof.spawn(feet, width, height);
                }
            }
            for hit in output.player_hits.into_iter().filter(|h| h.player_id == PLAYER) {
                let from = match hit.kind {
                    PlayerHitKind::Melee { attacker, .. } => Some(attacker.to_array()),
                    _ => None,
                };
                let toward = match hit.kind {
                    PlayerHitKind::Melee { attacker, .. } => [attacker.x - player.feet[0], attacker.z - player.feet[2]],
                    PlayerHitKind::Arrow { velocity, .. } => [-velocity.x, -velocity.z],
                    PlayerHitKind::Explosion { knockback } => [-knockback.x, -knockback.z],
                };
                let attack = match hit.kind {
                    PlayerHitKind::Melee { .. } => minecraft_survival::death::MobAttack::Melee,
                    PlayerHitKind::Arrow { .. } => minecraft_survival::death::MobAttack::Arrow,
                    PlayerHitKind::Explosion { .. } => minecraft_survival::death::MobAttack::Explosion,
                };
                let cause = minecraft_survival::death::DamageCause::mob(attack, hit.source, hit.source.and_then(|id| self.world.entity_type(id)));
                let scales = matches!(hit.kind, PlayerHitKind::Explosion { .. }) || hit.source.is_some();
                let damage = difficulty_scaled(hit.damage, self.difficulty, scales);
                if damage > 0.0 {
                    hits.push((damage, from, Some(toward), cause));
                }
            }
        }
        if ticked {
            self.server_items_tick(DVec3::from_array(player.feet));
            // `ItemEntity.playerTouch`'s pickup pop.
            for _ in 0..self.world_items.take_pickup_sounds() {
                let pitch = ((self.random.next_float() - self.random.next_float()) * 0.7 + 1.0) * 2.0;
                self.sounds.push(("minecraft:entity.item.pickup".to_owned(), DVec3::from_array(player.feet), 0.2, pitch));
            }
        }
        (changes, hits)
    }

    pub(crate) fn mob_present(&self, id: u64) -> bool {
        self.world.entity_type(id).is_some()
    }

    /// Every living mob's key and box in blocks, for bullets.
    pub(crate) fn boxes(&self) -> Vec<(u64, [f64; 6])> {
        let w = &self.world;
        let mut out = Vec::new();
        let mut add = |hit: MobHit, body: &minecraftoss_entities::movement::Body| {
            let p = body.position;
            let half = f64::from(body.width) * 0.5;
            out.push((
                encode(hit),
                [p.x - half, p.y, p.z - half, p.x + half, p.y + f64::from(body.height), p.z + half],
            ));
        };
        for e in w.bats().iter().filter(|e| e.bat.health > 0.0) {
            add(MobHit::Bat(e.id), &e.bat.body);
        }
        for e in w.zombies().iter().filter(|e| e.zombie.health > 0.0) {
            add(MobHit::Zombie(e.id), &e.zombie.body);
        }
        for e in w.skeletons().iter().filter(|e| e.skeleton.health > 0.0) {
            add(MobHit::Skeleton(e.id), &e.skeleton.body);
        }
        for e in w.creepers().iter().filter(|e| e.creeper.health > 0.0 && !e.creeper.exploded) {
            add(MobHit::Creeper(e.id), &e.creeper.body);
        }
        for e in w.spiders().iter().filter(|e| e.spider.health > 0.0) {
            add(MobHit::Spider(e.id), &e.spider.body);
        }
        for e in w.slimes().iter().filter(|e| e.slime.health > 0.0) {
            add(MobHit::Slime(e.id), &e.slime.body);
        }
        for e in w.endermen().iter().filter(|e| e.enderman.health > 0.0) {
            add(MobHit::Enderman(e.id), &e.enderman.body);
        }
        for e in w.witches().iter().filter(|e| e.witch.health > 0.0) {
            add(MobHit::Witch(e.id), &e.witch.body);
        }
        for e in w.iron_golems().iter().filter(|e| e.golem.health > 0.0) {
            add(MobHit::IronGolem(e.id), &e.golem.body);
        }
        for e in w.wolves().iter().filter(|e| e.wolf.health > 0.0) {
            add(MobHit::Wolf(e.id), &e.wolf.body);
        }
        for e in w.villagers().iter().filter(|e| e.villager.health > 0.0) {
            add(MobHit::Villager(e.id), &e.villager.body);
        }
        for e in w.cows().iter().filter(|e| e.cow.health > 0.0) {
            let hit = if e.mooshroom.is_some() { MobHit::Mooshroom(e.id) } else { MobHit::Cow(e.id) };
            add(hit, &e.cow.body);
        }
        for e in w.sheep().iter().filter(|e| e.health > 0.0) {
            add(MobHit::Sheep(e.id), &e.body);
        }
        for e in w.pigs().iter().filter(|e| e.pig.health > 0.0) {
            add(MobHit::Pig(e.id), &e.pig.body);
        }
        for e in w.chickens().iter().filter(|e| e.chicken.health > 0.0) {
            add(MobHit::Chicken(e.id), &e.chicken.body);
        }
        out
    }

    /// Steps the client-side puffs, which settle on the scene's blocks.
    pub(crate) fn tick_scene(&mut self, scene: &HandcraftedScene, ticks: u32) {
        for _ in 0..ticks {
            self.poof.tick(scene);
        }
    }

    /// The mobs this frame, drawn with the viewer's renderers.
    pub(crate) fn meshes(
        &mut self,
        scene: &HandcraftedScene,
        packs: &PackStack,
        atlas: &Atlas,
        light: &SkyLight,
        forward: Vec3,
        camera: DVec3,
        sky_darken: u8,
        cell_settled: &dyn Fn((i32, i32, i32)) -> bool,
        placed_items: &[minecraft_terrain::mesh::HeldItem],
    ) -> MobMeshes {
        use minecraft_terrain::*;
        // Mobs appear on the client once their trackers start.
        self.client.spawn_missing(&server_mobs(&self.world));
        let w = &self.world;
        let poses = &self.client;
        let partial = (self.clock / TICK_SECONDS).clamp(0.0, 1.0) as f32;
        let mut out = MobMeshes::default();
        cow_render::append_cows(&mut out.models, w.cows().iter(), poses, atlas, light, partial);
        sheep_render::append_sheep(&mut out.models, w.sheep().iter(), poses, atlas, light, partial);
        pig_render::append_pigs(&mut out.models, w.pigs().iter(), poses, atlas, light, partial);
        chicken_render::append_chickens(&mut out.models, w.chickens().iter(), poses, atlas, light, partial);
        bat_render::append_bats(&mut out.culled, w.bats().iter(), poses, atlas, light, partial);
        let zombie_items = zombie_render::append_zombies(&mut out.models, w.zombies().iter(), poses, atlas, light, partial);
        creeper_render::append_creepers(&mut out.models, w.creepers().iter(), poses, atlas, light, partial);
        spider_render::append_spiders(&mut out.models, w.spiders().iter(), poses, atlas, light, partial);
        let skeleton_items =
            skeleton_render::append_skeletons(&mut out.models, w.skeletons().iter(), poses, atlas, light, partial);
        let villager_items = villager_render::append_villagers(
            &mut out.models,
            w.villagers().iter(),
            poses,
            &|pos| Scene::block(scene, pos).and_then(|b| b.properties.get("facing").cloned()),
            atlas,
            light,
            partial,
        );
        horse_render::append_horses(&mut out.models, &mut out.translucent, w.cows().iter(), poses, atlas, light, partial);
        slime_render::append_slimes(&mut out.models, &mut out.translucent, w.slimes().iter(), poses, atlas, light, partial);
        orb_render::append_orbs(&mut out.translucent, &self.orbs, &self.orb_pickups, partial, atlas, forward, light);
        let carried = enderman_render::append_endermen(
            &mut out.models,
            w.endermen().iter(),
            poses,
            atlas,
            light,
            partial,
            self.ticks.rotate_left(32) ^ (partial.to_bits() as u64),
        );
        let witch_items = witch_render::append_witches(&mut out.models, w.witches().iter(), poses, atlas, light, partial);
        let poppies = golem_render::append_iron_golems(&mut out.models, w.iron_golems().iter(), poses, atlas, light, partial);
        wolf_render::append_wolves(&mut out.models, w.wolves().iter(), poses, atlas, light, partial, w.game_time());
        flame_render::append_flames(
            &mut out.items,
            w.burning()
                .into_iter()
                .chain(self.world_items.entities.iter().filter(|e| self.burning_items.contains(&e.entity_id)).map(|e| (e.previous_position, e.position, 0.25, 0.25)))
                .map(|(previous, now, width, height)| (previous.lerp(now, f64::from(partial)), width, height)),
            atlas,
            forward,
            light,
        );
        witch_render::append_potions(
            &mut out.items,
            w.potions()
                .iter()
                .filter(|p| p.potion.alive)
                .map(|p| (p, p.previous_position.lerp(p.potion.position, f64::from(partial)))),
            atlas,
            forward,
            light,
        );
        if let Ok(items) = self.items.mesh(&self.world_items, packs, atlas, light, partial) {
            let base = out.items.vertices.len() as u32;
            out.items.vertices.extend(items.vertices);
            out.items.indices.extend(items.indices.iter().map(|i| i + base));
            out.items.glint.extend(items.glint.iter().map(|r| r.start + base..r.end + base));
        }
        self.falling.settle(scene, cell_settled, self.ticks);
        let moving = self.falling.moving_blocks(scene, partial);
        let _ = self.items.append_moving_blocks(&mut out.items, &moving, scene, packs, atlas, light);
        let _ = self.items.append_primed_tnt(&mut out.items, &self.falling.primed_tnt(partial), packs, atlas, light);
        minecraft_terrain::thrown::append_sprites(
            &mut out.items,
            self.thrown
                .iter()
                .map(|(item, previous, now, age)| (item.as_str(), DVec3::from_array(*previous).lerp(DVec3::from_array(*now), f64::from(partial)), *age))
                .filter(|(_, at, age)| minecraft_terrain::thrown::shown(*age, *at, camera))
                .map(|(item, at, _)| (item, at)),
            atlas,
            forward,
            light,
        );
        self.poof.append_mesh(&mut out.items, atlas, forward, partial, light);
        self.portal.append_mesh(&mut out.items, atlas, forward, partial, light);
        let held: Vec<_> =
            skeleton_items.into_iter().chain(zombie_items).chain(villager_items).chain(witch_items).chain(placed_items.iter().cloned()).collect();
        let _ = self.items.append_held_items(&mut out.items, &held, packs, atlas, light);
        let carried: Vec<_> = carried.into_iter().chain(poppies).collect();
        let _ = self.items.append_posed_blocks(&mut out.items, &carried, packs, atlas, light);
        // Their shadows, at `getMaxLocalRawBrightness`.
        let mut casters = client_mobs::shadow_casters(w, poses, camera, partial);
        casters.extend(orb_render::shadow_casters(&self.orbs, camera, partial));
        casters.extend(self.falling.shadow_casters(scene, camera, partial));
        let raw = |pos: (i32, i32, i32)| light.get(pos).saturating_sub(sky_darken).max(light.get_block(pos));
        if let Ok(shadows) = mesh::entity_shadows(&casters, scene, atlas, &raw, 0.0) {
            out.shadows = shadows;
        }
        out
    }
}

fn difficulty_scaled(damage: f32, difficulty: i32, scales: bool) -> f32 {
    if !scales {
        return damage;
    }
    match difficulty {
        0 => 0.0,
        1 => (damage / 2.0 + 1.0).min(damage),
        3 => damage * 3.0 / 2.0,
        _ => damage,
    }
}

fn mirrored(id: i32) -> u32 {
    id as u32 | SERVER_ITEM
}

fn how_broken(pos: (i32, i32, i32), blast: Option<f32>, by_hand: &[(i32, i32, i32)], silk_touch: bool, creative: bool) -> HowBroken {
    match blast {
        Some(_) => HowBroken::Blasted,
        None if creative => HowBroken::SilkTouched,
        None if silk_touch && by_hand.contains(&pos) => HowBroken::SilkTouched,
        None => HowBroken::Mined,
    }
}

fn has_silk_touch(enchantments: Option<&serde_json::Value>) -> bool {
    enchantments.and_then(|enchantments| enchantments.get(SILK_TOUCH)).and_then(serde_json::Value::as_i64).is_some_and(|level| level > 0)
}

fn loot_tool_with(enchantments: Option<serde_json::Value>) -> ItemStack {
    let mut tool = ItemStack::new(LOOT_TOOL, 1);
    tool.components = enchantments.map(|enchantments| serde_json::json!({ "minecraft:enchantments": enchantments }));
    tool
}

/// The viewer's `server_bob_offset`: a server item's bob phase from its id.
fn bob_offset(id: i32) -> f32 {
    let hash = (id as u32).wrapping_mul(0x9e37_79b9).rotate_left(13).wrapping_mul(0x85eb_ca6b);
    hash as f32 / u32::MAX as f32 * std::f32::consts::TAU
}

/// A mob hit as one number: the kind in the top byte, the id below.
fn encode(hit: MobHit) -> u64 {
    let kind: u64 = match hit {
        MobHit::Bat(_) => 0,
        MobHit::Zombie(_) => 1,
        MobHit::Skeleton(_) => 2,
        MobHit::Creeper(_) => 3,
        MobHit::Spider(_) => 4,
        MobHit::Slime(_) => 5,
        MobHit::Enderman(_) => 6,
        MobHit::Witch(_) => 7,
        MobHit::IronGolem(_) => 8,
        MobHit::Wolf(_) => 9,
        MobHit::Villager(_) => 10,
        MobHit::Cow(_) => 11,
        MobHit::Mooshroom(_) => 12,
        MobHit::Sheep(_) => 13,
        MobHit::Pig(_) => 14,
        MobHit::Chicken(_) => 15,
    };
    (kind << 56) | (hit.id() & ((1 << 56) - 1))
}

fn decode(key: u64) -> Option<MobHit> {
    let id = key & ((1 << 56) - 1);
    Some(match key >> 56 {
        0 => MobHit::Bat(id),
        1 => MobHit::Zombie(id),
        2 => MobHit::Skeleton(id),
        3 => MobHit::Creeper(id),
        4 => MobHit::Spider(id),
        5 => MobHit::Slime(id),
        6 => MobHit::Enderman(id),
        7 => MobHit::Witch(id),
        8 => MobHit::IronGolem(id),
        9 => MobHit::Wolf(id),
        10 => MobHit::Villager(id),
        11 => MobHit::Cow(id),
        12 => MobHit::Mooshroom(id),
        13 => MobHit::Sheep(id),
        14 => MobHit::Pig(id),
        15 => MobHit::Chicken(id),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobs_leave_creative_and_spectator_players_alone() {
        let view = |attackable, spectator| PlayerView {
            feet: [1.0, 64.0, 2.0],
            alive: true,
            attackable,
            spectator,
            picks_up: !spectator,
            health: 20.0,
            yaw: 0.0,
            pitch: 0.0,
        };
        let survival = player_candidate(&view(true, false));
        assert!(survival.alive && survival.attackable && !survival.spectator);
        assert_eq!(survival.position, DVec3::new(1.0, 64.0, 2.0));
        let creative = player_candidate(&view(false, false));
        assert!(creative.alive && !creative.attackable && !creative.spectator);
        let spectator = player_candidate(&view(false, true));
        assert!(spectator.alive && !spectator.attackable && spectator.spectator);
    }

    #[test]
    fn blasts_hurt_mobs_four_times_as_hard_as_bullets() {
        assert_eq!(HEALTH_SCALE, 0.2);
        assert_eq!(BLAST_HEALTH_SCALE, 0.8);
        assert_eq!(BLAST_HEALTH_SCALE, 4.0 * HEALTH_SCALE);
    }

    #[test]
    fn mob_hits_on_the_player_scale_with_difficulty_like_vanilla() {
        assert_eq!(difficulty_scaled(6.0, 0, true), 0.0);
        assert_eq!(difficulty_scaled(6.0, 1, true), 4.0);
        assert_eq!(difficulty_scaled(1.5, 1, true), 1.5);
        assert_eq!(difficulty_scaled(6.0, 2, true), 6.0);
        assert_eq!(difficulty_scaled(6.0, 3, true), 9.0);
        assert_eq!(difficulty_scaled(6.0, 3, false), 6.0);
        assert_eq!(difficulty_scaled(6.0, 0, false), 6.0);
    }

    #[test]
    fn only_silk_touch_in_the_mining_hand_keeps_ice_from_melting() {
        let by_hand = [(1, 2, 3)];
        assert_eq!(how_broken((1, 2, 3), None, &by_hand, true, false), HowBroken::SilkTouched);
        assert_eq!(how_broken((1, 2, 3), None, &by_hand, false, false), HowBroken::Mined);
        assert_eq!(how_broken((4, 2, 3), None, &by_hand, true, false), HowBroken::Mined);
        assert_eq!(how_broken((1, 2, 3), Some(4.0), &by_hand, true, false), HowBroken::Blasted);
    }

    #[test]
    fn creative_hand_and_bullet_breaks_leave_no_water_and_blasts_stay_blasts() {
        let by_hand = [(1, 2, 3)];
        assert_eq!(how_broken((1, 2, 3), None, &by_hand, false, true), HowBroken::SilkTouched);
        assert_eq!(how_broken((4, 2, 3), None, &by_hand, false, true), HowBroken::SilkTouched);
        assert_eq!(how_broken((1, 2, 3), Some(4.0), &by_hand, false, true), HowBroken::Blasted);
    }

    #[test]
    fn the_hand_loot_tool_carries_the_held_enchantments() {
        let silk = serde_json::json!({ "minecraft:silk_touch": 1 });
        assert!(has_silk_touch(Some(&silk)));
        assert!(!has_silk_touch(Some(&serde_json::json!({ "minecraft:efficiency": 5 }))));
        assert!(!has_silk_touch(None));
        let tool = loot_tool_with(Some(silk));
        assert_eq!(tool.id, LOOT_TOOL);
        assert_eq!(tool.components, Some(serde_json::json!({ "minecraft:enchantments": { "minecraft:silk_touch": 1 } })));
        assert_eq!(loot_tool_with(None).components, None);
    }
}
