//! The Minecraft world's mobs, as MinecraftOSS runs them: its integrated
//! server (`engine/viewer/src/server.rs`) ticks the level at 20 Hz on its own
//! thread with natural spawning and the entity world, every passive and
//! hostile mob's AI and pathfinding. Here it is fed the streamed chunks and
//! the player, and hands back the tracked mobs, which are drawn with the
//! viewer's mob renderers. MW2 bullets hurt the mobs with their real damage
//! (a hundred MW2 health to Minecraft's twenty), and mobs hurt the player
//! the other way round.
use glam::{DVec3, Vec3};
use minecraft_terrain::level_storage::WeatherData;
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
use minecraftoss_player::survival::EffectKind;
use std::collections::HashMap;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::poof_particles::PoofParticles;
use minecraft_terrain::portal_particles::PortalParticles;
use minecraftoss_entities::tempt::PlayerCandidate;
use minecraftoss_entities::world::{EntityWorld, MobHit, PlayerHitKind};
use minecraft_terrain::end_crystal::{EndDamage, EndTarget};

const TICK_SECONDS: f64 = 1.0 / 20.0;
/// Minecraft health per MW2 health.
const HEALTH_SCALE: f32 = 20.0 / 100.0;
const BLAST_HEALTH_SCALE: f32 = 80.0 / 100.0;
/// The player's id in the entity world.
const PLAYER: u64 = 0;
/// `Player.getEyeHeight` standing.
const EYE_HEIGHT: f32 = 1.62;
const SERVER_ITEM: u32 = 1 << 31;
type Hits = Vec<(f32, Option<[f64; 3]>, Option<[f64; 2]>, crate::death::DamageCause, Option<(EffectKind, u32)>)>;
type Changes = Vec<((i32, i32, i32), Option<Block>)>;
type Picked = (i32, [f64; 3], String, i32, Option<String>);

pub struct Blow {
    pub sprinting: bool,
    pub can_critical: bool,
    pub can_sweep: bool,
    pub reach: f64,
}

pub struct SentItemUse {
    pub slot: usize,
    pub held: String,
    pub eye: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub outcome: minecraft_terrain::bucket::ItemUse,
}

pub struct Entities {
    server: ServerHandle,
    /// The mobs the player tracks, as of the last server tick.
    world: EntityWorld,
    /// The client's copies of them: tracked, interpolated, animated.
    client: ClientMobs,
    poof: PoofParticles,
    portal: PortalParticles,
    eyes: minecraft_terrain::eye_of_ender::EyeTrails,
    spell: minecraft_terrain::spell_particles::SpellParticles,
    ambient: minecraft_terrain::ambient_particles::AmbientParticles,
    level_events: Vec<(i32, (i32, i32, i32), i32)>,
    pub broken_items: Vec<String>,
    items: ItemVisuals,
    clock: f64,
    ticks: u64,
    /// Item entities as the client shows them: the server's, and drops the
    /// client spawned that the server has not taken yet.
    pub world_items: WorldItems,
    /// Client drops handed to the server: the command count that sent each.
    server_handed: HashMap<u32, (u64, ItemEntity)>,
    burning_items: Vec<u32>,
    /// Stacks the server let the player pick up, not yet in the inventory.
    server_picked: Vec<(i32, [f64; 3], String, i32, Option<String>)>,
    block_entity_changes: Vec<((i32, i32, i32), Option<minecraftoss_core::nbt::Tag>)>,
    server_snapshot: Option<EntitySnapshot>,
    server_handled: u64,
    loot: Option<std::sync::Arc<LootBook>>,
    loot_sequences: HashMap<String, XoroshiroRandom>,
    seed: i64,
    /// Sounds the mob world made since the last take: event, block point,
    /// volume, pitch.
    pub sounds: Vec<(String, DVec3, f32, f32)>,
    random: minecraftoss_player::rng::LegacyRandom,
    thrown: Vec<minecraft_terrain::thrown::ThrownView>,
    clouds: Vec<minecraft_terrain::effect_cloud::CloudView>,
    tridents: Vec<minecraft_terrain::trident::TridentView>,
    crystals: Vec<minecraft_terrain::end_crystal::CrystalView>,
    dragons: Vec<minecraft_terrain::ender_dragon::DragonView>,
    dragon_fireballs: Vec<minecraft_terrain::dragon_fireball::FireballView>,
    growls: HashMap<u64, i32>,
    dragons_killed: Vec<[i32; 4]>,
    fight_reports: Vec<minecraft_terrain::server::FightReport>,
    orbs: Vec<ServerOrb>,
    falling: minecraft_terrain::falling_render::FallingBlocks,
    pub lids: minecraft_terrain::lids::Lids,
    orb_pickups: Vec<minecraft_terrain::orb_render::OrbPickup>,
    pub pearl_landings: Vec<DVec3>,
    pub player_potions: Vec<minecraft_terrain::potions::PlayerPotionHit>,
    pub player_pushes: Vec<DVec3>,
    pub item_crumbs: Vec<(String, DVec3, Vec<Vec3>)>,
    pub potion_crumbs: Vec<DVec3>,
    pub destroyed: Vec<((i32, i32, i32), Block)>,
    pub experience_found: f32,
    pub weather: minecraft_terrain::server::WeatherView,
    pub weather_data: Option<WeatherData>,
    pub world_weather: Option<WeatherData>,
    pub stations: std::sync::Arc<minecraft_terrain::stations::StationBook>,
    pub enchantments: Option<std::sync::Arc<minecraftoss_entities::enchant::Enchantments>>,
    pub recipes: std::sync::Arc<minecraftoss_player::crafting::RecipeBook>,
    item_use_waiting: Option<SentItemUse>,
    pub item_uses_done: Vec<SentItemUse>,
    eggs_waiting: minecraft_terrain::spawn_egg::EggsWaiting,
    pub command_summons: Vec<Result<String, String>>,
    pub command_jobs: Vec<Box<dyn std::any::Any + Send>>,
    jobs_running: usize,
    unread: Vec<minecraft_terrain::server::Output>,
    pub difficulty: i32,
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
pub struct MobMeshes {
    pub models: ChunkMesh,
    pub culled: ChunkMesh,
    pub translucent: ChunkMesh,
    pub shadows: ChunkMesh,
    pub items: ChunkMesh,
}

/// The player as the mobs see it this frame.
pub struct PlayerView {
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
    pub fn new(stream: &TerrainStream, dimension: minecraft_net::Dimension, seed: i64, data: &crate::data::GameData, saved_weather: Option<&WeatherData>, forced_weather: Option<&str>) -> Self {
        let mut sim = ServerSim::new(stream.world_gen(), stream.states.clone(), dimension.dimension_type());
        sim.set_entity_storage(stream.storage());
        let has_weather = crate::server::has_weather(dimension);
        if let Some(saved) = saved_weather.filter(|_| has_weather) {
            sim.set_weather_data(saved);
            diag::info!(World, "Minecraft weather: {} from weather.dat", weather_name(saved));
        }
        if has_weather
            && let Some(weather) = forced_weather
            && !weather.trim().is_empty()
        {
            if sim.set_weather(weather.trim()) {
                diag::info!(World, "Minecraft weather: {} from IW4L_MINECRAFT_WEATHER", weather.trim());
            } else {
                diag::warn!(World, "Minecraft weather `{weather}` unknown: IW4L_MINECRAFT_WEATHER takes clear, rain or thunder");
            }
        }
        let weather_data = sim.weather_data();
        let mut server = ServerHandle::spawn(sim);
        // Mob and block loot and the recipes (stack sizes), from the game's
        // data JAR when MinecraftOSS has one.
        if let Some(jar) = &data.jar {
            server.load_loot(jar.clone(), seed as u64);
            server.set_recipe_book(data.recipes.clone());
        }
        Self {
            server,
            world: EntityWorld::default(),
            client: ClientMobs::default(),
            poof: PoofParticles::default(),
            portal: PortalParticles::default(),
            eyes: minecraft_terrain::eye_of_ender::EyeTrails::default(),
            spell: minecraft_terrain::spell_particles::SpellParticles::new((seed ^ 0x5be1) as u64),
            ambient: minecraft_terrain::ambient_particles::AmbientParticles::new((seed ^ 0xa3b1) as u64),
            level_events: Vec::new(),
            broken_items: Vec::new(),
            items: ItemVisuals::default(),
            clock: 0.0,
            ticks: 0,
            world_items: WorldItems::default(),
            server_handed: HashMap::new(),
            burning_items: Vec::new(),
            server_picked: Vec::new(),
            block_entity_changes: Vec::new(),
            server_snapshot: None,
            server_handled: 0,
            loot: data.loot.clone(),
            loot_sequences: HashMap::new(),
            seed,
            sounds: Vec::new(),
            random: minecraftoss_player::rng::LegacyRandom::new((seed ^ 0x1735) as u64),
            thrown: Vec::new(),
            clouds: Vec::new(),
            tridents: Vec::new(),
            crystals: Vec::new(),
            dragons: Vec::new(),
            dragon_fireballs: Vec::new(),
            growls: HashMap::new(),
            dragons_killed: Vec::new(),
            fight_reports: Vec::new(),
            orbs: Vec::new(),
            falling: Default::default(),
            lids: Default::default(),
            orb_pickups: Vec::new(),
            pearl_landings: Vec::new(),
            player_potions: Vec::new(),
            player_pushes: Vec::new(),
            item_crumbs: Vec::new(),
            potion_crumbs: Vec::new(),
            destroyed: Vec::new(),
            experience_found: 0.0,
            weather: Default::default(),
            weather_data,
            world_weather: None,
            stations: data.stations.clone(),
            enchantments: data.enchantments.clone(),
            recipes: data.recipes.clone(),
            item_use_waiting: None,
            item_uses_done: Vec::new(),
            eggs_waiting: Default::default(),
            command_summons: Vec::new(),
            command_jobs: Vec::new(),
            jobs_running: 0,
            unread: Vec::new(),
            difficulty: 2,
            spawn_mobs: true,
            block_drops: true,
        }
    }

    pub fn set_game_rules(&mut self, rules: &minecraft_terrain::game_rules::GameRules) {
        self.spawn_mobs = rules.bool("spawn_mobs");
        self.block_drops = rules.bool("block_drops");
        self.server.set_game_rules(rules.clone());
    }

    pub fn set_weather(&mut self, weather: &'static str, duration: i32) {
        self.server.set_weather(weather, duration);
        self.world_weather = minecraft_terrain::server::weather_parameters(weather, duration);
    }

    pub fn set_weather_data(&mut self, data: WeatherData) {
        self.server.set_weather_data(data);
        self.weather_data = Some(data);
    }

    pub fn set_player_uuid(&mut self, uuid: &minecraft_net::PlayerUuid) {
        self.server.set_player_uuid(PLAYER, crate::level::opener_key(uuid));
    }

    pub fn throw(&mut self, stack: &ItemStack, eye: DVec3, yaw: f32, pitch: f32, motion: DVec3, on_ground: bool) -> bool {
        let throw = minecraft_terrain::thrown::Throw { owner: PLAYER, item: &stack.id, components: stack.components.as_ref(), eye, yaw, pitch, motion, on_ground };
        let Some(thrown) = minecraft_terrain::thrown::Thrown::launch(&throw, &mut self.random) else {
            return false;
        };
        self.server.throw(thrown);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn release_trident(&mut self, inventory: &mut Inventory, charging: crate::trident::Charging, creative: bool, wet: bool, eye: DVec3, yaw: f32, pitch: f32, motion: DVec3, on_ground: bool) -> minecraft_terrain::trident::Release {
        use minecraft_terrain::trident::{Release, ThrownTrident};
        let release = minecraft_terrain::trident::release(inventory, charging.slot, charging.ticks, creative, wet, &mut self.random);
        if let Release::Throw(stack) = &release {
            let throw = minecraft_terrain::thrown::Throw { owner: PLAYER, item: &stack.id, components: stack.components.as_ref(), eye, yaw, pitch, motion, on_ground };
            let trident = ThrownTrident::launch(&throw, stack.clone(), creative, &mut self.random);
            self.server.throw_trident(trident);
        }
        release
    }

    pub fn random_float(&mut self) -> f32 {
        self.random.next_float()
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn load_chunk(&mut self, chunk: &std::sync::Arc<minecraftoss_core::Chunk>) {
        self.server.load_chunk(chunk);
    }

    pub fn unload_chunk(&mut self, pos: minecraftoss_core::ChunkPos) {
        self.server.unload_chunk(pos);
    }

    pub fn award_experience(&mut self, at: [f64; 3], amount: i32) {
        self.server.award_experience(at, amount);
    }

    pub fn award_block_experience(&mut self, held: Option<&ItemStack>, broken: &[BrokenBlock], by_hand: &[(i32, i32, i32)]) {
        if !self.block_drops {
            return;
        }
        let silk_touch = Self::hand_has_silk_touch(held);
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
    pub fn drop_blocks(&mut self, held: Option<&ItemStack>, broken: &[BrokenBlock], mut kept: HashMap<(i32, i32, i32), serde_json::Value>, by_hand: &[(i32, i32, i32)]) {
        if !self.block_drops {
            return;
        }
        let tool = ItemStack::new(LOOT_TOOL, 1);
        let hand_tool = Self::hand_tool(held);
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
                    drop.max = drop.max.min(self.recipes.max_stack(&drop.id));
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
                boxed.max = boxed.max.min(self.recipes.max_stack(&boxed.id));
                boxed.components = Some(components);
                self.world_items.spawn_block_drop(boxed, *pos);
            }
        }
    }

    /// `ItemEntity`s to draw and pick up, as the viewer's
    /// `server_items_tick` does for a server-simulated world: client drops
    /// go to the server, stacks the server offered go into the inventory,
    /// and the server's items are mirrored for drawing.
    fn server_items_tick(&mut self, inventory: &mut Inventory, selected: usize, feet: DVec3) {
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
            let recipes = inventory.recipes.clone();
            let make_stack = |item: &str, count: i32| {
                let mut stack = ItemStack::new(item, count.clamp(0, 255) as u8);
                stack.components = components.as_deref().and_then(|c| serde_json::from_str(c).ok());
                stack.max = stack.max.min(recipes.max_stack(item));
                stack
            };
            let taken = match inventory.add_item(make_stack(&item, count), selected) {
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
        let recipes = inventory.recipes.clone();
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
    pub fn held_item_mesh(
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

    #[allow(clippy::too_many_arguments)]
    pub fn punch(&mut self, inventory: &Inventory, selected: usize, eye: DVec3, look: DVec3, yaw: f32, attack_damage: f64, strength: f32, blow: Blow) -> bool {
        let mob = self.world.mob_on_ray(eye, look, blow.reach);
        if let Some((target, distance)) = self.end_on_ray(eye, look, blow.reach)
            && mob.is_none_or(|(_, nearest)| distance < nearest)
        {
            let damage = attack_damage as f32 * (0.2 + strength * strength * 0.8);
            self.server.end_hit(target, EndDamage::attack(damage, PLAYER));
            return true;
        }
        if let Some((bullet, distance)) = self.world.shulker_bullet_on_ray(eye, look, blow.reach)
            && mob.is_none_or(|(_, nearest)| distance < nearest)
        {
            self.server.hurt_shulker_bullet(bullet);
            return true;
        }
        if let Some((fireball, distance)) = self.world.fireball_on_ray(eye, look, blow.reach)
            && mob.is_none_or(|(_, nearest)| distance < nearest)
        {
            self.server.deflect_fireball(fireball, look.normalize_or_zero().to_array(), PLAYER, true);
            let feet = eye - DVec3::Y * f64::from(EYE_HEIGHT);
            self.sounds.push(("minecraft:entity.player.attack.nodamage".to_owned(), feet, 1.0, 1.0));
            return true;
        }
        let Some((hit, _)) = mob else {
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
        self.server.mob_action(hit, Some(attack), inventory, selected, false);
        true
    }

    pub fn mob_distance(&self, eye: DVec3, look: DVec3, reach: f64) -> Option<f64> {
        let mob = self.world.mob_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let fireball = self.world.fireball_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let bullet = self.world.shulker_bullet_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let end = self.end_on_ray(eye, look, reach).map(|(_, distance)| distance);
        [mob, fireball, bullet, end].into_iter().flatten().min_by(f64::total_cmp)
    }

    fn end_on_ray(&self, eye: DVec3, look: DVec3, reach: f64) -> Option<(EndTarget, f64)> {
        let crystal = minecraft_terrain::end_crystal::on_ray(&self.crystals, eye, look, reach).map(|(id, distance)| (EndTarget::Crystal(id), distance));
        let parts = self.dragons.iter().filter(|dragon| dragon.health > 0.0).flat_map(|dragon| {
            dragon.parts.iter().enumerate().filter_map(move |(part, &feet)| {
                let found = minecraft_terrain::end_crystal::ray_box(eye, look, reach, minecraft_terrain::ender_dragon::part_box(feet, part))?;
                Some((EndTarget::DragonPart { id: dragon.id, part }, found))
            })
        });
        crystal.into_iter().chain(parts).min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// A block the player placed, for the level the mobs walk in.
    pub fn placed(&mut self, scene: &HandcraftedScene, pos: (i32, i32, i32)) {
        self.server.player_edit(scene, pos, PlayerEdit::Place);
    }

    pub fn keep_dropped_contents(&mut self, pos: (i32, i32, i32), id: &str, components: &serde_json::Value) {
        self.server.keep_dropped_contents(pos, id, &components.to_string());
    }

    pub fn use_block(&mut self, scene: &HandcraftedScene, pos: (i32, i32, i32), facing: &'static str) -> bool {
        self.server.use_block(scene, pos, facing)
    }

    pub fn attack_block(&mut self, scene: &HandcraftedScene, pos: (i32, i32, i32)) -> bool {
        self.server.attack_block(scene, pos)
    }

    pub fn item_on_block(&mut self, pos: (i32, i32, i32), action: minecraft_terrain::server::ItemOnBlock, sent: SentItemUse) {
        self.server.item_on_block(pos, action);
        self.item_use_waiting = Some(sent);
    }

    pub fn throw_eye(&mut self, throw: minecraft_terrain::ender_eye::EyeThrow, sent: SentItemUse) {
        self.server.throw_eye(PLAYER, throw);
        self.item_use_waiting = Some(sent);
    }

    pub fn bottle_breath(&mut self, feet: DVec3, height: f64, sent: SentItemUse) {
        self.server.bottle_breath(feet, height);
        self.item_use_waiting = Some(sent);
    }

    pub fn dragon_breath_near(&self, feet: DVec3, height: f64) -> bool {
        minecraft_terrain::effect_cloud::breath_near(&self.clouds, minecraft_terrain::effect_cloud::bottle_reach(feet, height))
    }

    pub fn awaits_item_use(&self) -> bool {
        self.item_use_waiting.is_some()
    }

    pub fn leave(&mut self, inventory: &mut Inventory, selected: usize, feet: Option<[f64; 3]>) {
        let picked = std::mem::take(&mut self.server_picked);
        for rest in take_picked(inventory, selected, picked) {
            if let Some(feet) = feet {
                self.server.spawn_item(&rest.id, i32::from(rest.count), rest.components.map(|c| c.to_string()).as_deref(), feet, [0.0; 3], 0, 0);
            }
        }
        self.item_use_waiting = None;
        self.pearl_landings.clear();
        self.player_potions.clear();
        self.player_pushes.clear();
    }

    pub fn hatch_egg(&mut self, inventory: &Inventory, slot: usize, kind: String, position: [f64; 3], spends: bool) {
        if self.eggs_waiting.send_from(inventory, slot, spends) {
            let y_rot = minecraftoss_core::entity_data::wrap_degrees(self.random.next_float() * 360.0);
            self.server.hatch_egg(kind, position, y_rot);
        }
    }

    pub fn take_block_entity_changes(&mut self) -> Vec<((i32, i32, i32), Option<minecraftoss_core::nbt::Tag>)> {
        std::mem::take(&mut self.block_entity_changes)
    }

    pub fn finish_block_entity_changes(&mut self) -> Vec<((i32, i32, i32), Option<minecraftoss_core::nbt::Tag>)> {
        self.server.save();
        self.server.wait_idle();
        for output in &mut self.unread {
            self.block_entity_changes.append(&mut output.block_entities);
        }
        for mut output in self.server.poll() {
            self.jobs_running = self.jobs_running.saturating_sub(output.jobs.len());
            self.command_jobs.append(&mut output.jobs);
            self.block_entity_changes.extend(output.block_entities);
        }
        self.take_block_entity_changes()
    }

    pub fn summon(&mut self, kind: String, position: [f64; 3], nbt: Option<minecraftoss_core::nbt::Tag>) {
        self.server.summon(kind, position, nbt);
    }

    pub fn run_job(&mut self, job: minecraft_terrain::server::Job) {
        self.jobs_running += 1;
        self.server.run_job(job);
    }

    pub fn runs_jobs(&self) -> bool {
        self.jobs_running > 0
    }

    pub fn poll_jobs(&mut self) {
        for mut output in self.server.poll() {
            self.jobs_running = self.jobs_running.saturating_sub(output.jobs.len());
            self.command_jobs.append(&mut output.jobs);
            self.unread.push(output);
        }
    }

    fn take_outputs(&mut self) -> Vec<minecraft_terrain::server::Output> {
        let mut outputs = std::mem::take(&mut self.unread);
        outputs.extend(self.server.poll());
        outputs
    }

    pub fn drop_given(&mut self, stack: &ItemStack, feet: [f64; 3]) {
        let components = stack.components.as_ref().map(|c| c.to_string());
        self.server.spawn_item(&stack.id, i32::from(stack.count), components.as_deref(), feet, [0.0; 3], 0, 0);
    }

    pub fn animate_tick<S: minecraft_terrain::scene::Scene + ?Sized>(&mut self, scene: &S, feet: [f64; 3]) -> Vec<minecraft_terrain::display_ticks::AmbientSound> {
        let at = (feet[0].floor() as i32, feet[1].floor() as i32, feet[2].floor() as i32);
        let tick = minecraft_terrain::display_ticks::animate_tick(scene, at, &mut self.random);
        for (start, motion) in tick.portal_particles {
            self.portal.emit(start, motion);
        }
        for (kind, start, motion) in tick.particles {
            self.ambient.add(kind, start, motion);
        }
        tick.sounds
    }

    fn present(&mut self, effects: minecraft_terrain::display_ticks::DisplayTick) {
        for sound in effects.sounds {
            self.sounds.push((sound.event.to_owned(), DVec3::from_array(sound.at), sound.volume, sound.pitch));
        }
        for (kind, at, motion) in effects.particles {
            self.ambient.add(kind, at, motion);
        }
    }

    pub fn present_level_events(&mut self, scene: &HandcraftedScene) {
        let is_a = minecraft_terrain::level_events::class_test(scene);
        for (id, pos, data) in std::mem::take(&mut self.level_events) {
            let effects = minecraft_terrain::level_events::level_event(id, pos, data, scene, &is_a, &mut self.random);
            self.present(effects);
        }
    }

    pub fn evaporation_smoke(&mut self, pos: (i32, i32, i32)) {
        let effects = minecraft_terrain::level_events::evaporation(pos, &mut self.random);
        self.present(effects);
    }

    pub fn powder_snow_flake(&mut self, feet: [f64; 3], block_y: i32) {
        let mut across = || f64::from((self.random.next_float() * 2.0 - 1.0) * 0.083_333_336);
        let motion = DVec3::new(across(), f64::from(0.05_f32), across());
        let at = DVec3::new(feet[0], f64::from(block_y + 1), feet[2]);
        self.ambient.add(minecraft_terrain::ambient_particles::Kind::Snowflake, at, motion);
    }

    pub fn campfire_douse(&mut self, pos: (i32, i32, i32), signal: bool) {
        self.ambient.douse(pos, signal);
    }

    pub fn rain_tick(&mut self, scene: &HandcraftedScene, camera: [f64; 3]) -> Vec<minecraft_terrain::display_ticks::AmbientSound> {
        self.ambient.rain_tick(scene, DVec3::from_array(camera), self.weather.rain_level)
    }

    pub fn campfire_tick(&mut self, scene: &HandcraftedScene, food: impl Fn((i32, i32, i32)) -> [bool; 4]) {
        self.ambient.campfire_tick(scene, food);
    }

    /// Client ticks run so far.
    pub fn client_ticks(&self) -> u64 {
        self.ticks
    }

    /// Blocks the player's weapons broke, for the level the mobs walk in.
    pub fn broke(&mut self, positions: &[(i32, i32, i32)]) {
        self.server.player_breaks(positions);
    }

    pub fn mined(&mut self, held: Option<&ItemStack>, broken: &[BrokenBlock], by_hand: &[(i32, i32, i32)], creative: bool) {
        let silk_touch = Self::hand_has_silk_touch(held);
        let how: Vec<_> = broken.iter().map(|(pos, _, blast)| (*pos, how_broken(*pos, *blast, by_hand, silk_touch, creative))).collect();
        self.server.player_mines(&how);
    }

    fn hand_enchantments(held: Option<&ItemStack>) -> Option<serde_json::Value> {
        held?.components.as_ref()?.get("minecraft:enchantments").cloned()
    }

    fn hand_has_silk_touch(held: Option<&ItemStack>) -> bool {
        has_silk_touch(Self::hand_enchantments(held).as_ref())
    }

    fn hand_tool(held: Option<&ItemStack>) -> ItemStack {
        loot_tool_with(Self::hand_enchantments(held))
    }

    /// A bullet on a mob: an attack with the bullet's damage from where it
    /// was fired.
    pub fn shoot(&mut self, key: u64, damage: f32, from: [f64; 3], yaw: f32) {
        if let Some(target) = end_target_of(key) {
            self.server.end_hit(target, EndDamage::attack(damage * HEALTH_SCALE, PLAYER));
            return;
        }
        if let Some(id) = shulker_bullet_of(key) {
            self.server.hurt_shulker_bullet(id);
            return;
        }
        if let Some(id) = fireball_of(key) {
            if let Some(entity) = self.world.fireballs().iter().find(|e| e.id == id) {
                let center = entity.fireball.position + DVec3::Y * (entity.fireball.kind.size() / 2.0);
                let aim = (center - DVec3::from_array(from)).normalize_or_zero();
                self.server.deflect_fireball(id, aim.to_array(), PLAYER, true);
            }
            return;
        }
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

    pub fn blast(&mut self, key: u64, damage: f32, center: [f64; 3], impact: f32, by_player: bool, slow_ticks: i32) {
        if let Some(target) = end_target_of(key) {
            self.server.end_hit(target, EndDamage::explosion(damage * BLAST_HEALTH_SCALE, by_player.then_some(PLAYER)));
            return;
        }
        if let Some(id) = shulker_bullet_of(key) {
            self.server.hurt_shulker_bullet(id);
            return;
        }
        if let Some(id) = fireball_of(key) {
            if let Some(entity) = self.world.fireballs().iter().find(|e| e.id == id) {
                let eye = entity.fireball.position + DVec3::Y * (entity.fireball.kind.size() * 0.85);
                let push = (eye - DVec3::from_array(center)).normalize_or_zero() * f64::from(impact);
                self.server.push_fireball(id, push.to_array());
            }
            return;
        }
        let Some(hit) = decode(key) else {
            return;
        };
        self.server.mob_blast(hit, damage * BLAST_HEALTH_SCALE, center, impact, by_player.then_some(PLAYER), slow_ticks);
    }

    pub fn weapon_blast(&mut self, center: [f64; 3], power: f32) {
        self.server.weapon_blast(center, power);
    }

    pub fn explode(&mut self, blast: minecraft_terrain::server::Blast) {
        self.server.blast(blast);
    }

    pub fn explode_respawn_point(&mut self, blast: minecraft_terrain::server::Blast, shield: Option<(i32, i32, i32)>) {
        self.server.respawn_point_blast(blast, shield);
    }

    /// Sends a server tick when one is due and takes what came back: block
    /// changes to show, and mob hits on the player in MW2 damage.
    pub fn tick(
        &mut self,
        inventory: &mut Inventory,
        selected: usize,
        dt: f64,
        clocks: minecraft_terrain::server::ClockTicks,
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
            for tnt in self.falling.primed.iter().filter(|tnt| tnt.fuse > 0) {
                let smoke = DVec3::from_array(tnt.position) + DVec3::Y * 0.5;
                self.ambient.add(minecraft_terrain::ambient_particles::Kind::Smoke, smoke, DVec3::ZERO);
            }
            self.lids.tick();
            // Packets first, then the client level's entity ticks.
            self.client.tick();
            for enderman in self.world.endermen() {
                if let Some(mob) = self.client.get(enderman.id) {
                    self.portal.emit_enderman(mob.position, 0.6, 2.9);
                }
            }
            self.eyes.tick(&mut self.portal);
            for blaze in self.world.blazes() {
                if let Some(mob) = self.client.get(blaze.id).filter(|_| self.random.next_int(24) == 0) {
                    let volume = 1.0 + self.random.next_float();
                    let pitch = self.random.next_float() * 0.7 + 0.3;
                    self.sounds.push(("minecraft:entity.blaze.burn".to_owned(), mob.position + DVec3::splat(0.5), volume, pitch));
                }
            }
            self.portal.tick();
            for cloud in &self.clouds {
                self.spell.cloud(cloud.position, cloud.radius, cloud.color, cloud.waiting);
            }
            self.spell.tick();
            minecraft_terrain::orb_render::tick_pickups(&mut self.orb_pickups, orb_pickup_target);
            if self.ticks % 600 == 0 {
                diag::info!(World, "Minecraft mobs: {} tracked", self.boxes().len());
            }
            let feet = DVec3::from_array(player.feet);
            let center = ((feet.x.floor() as i32) >> 4, (feet.z.floor() as i32) >> 4);
            let candidate = player_candidate(player);
            self.server.tick(TickInput {
                clocks,
                players: if player.alive { vec![player.feet] } else { Vec::new() },
                difficulty: self.difficulty,
                simulation_center: center,
                simulation_distance: 8,
                pickup: player
                    .picks_up
                    .then(|| (player.feet, Box::new(inventory.clone()), selected)),
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
        for output in self.take_outputs() {
            self.jobs_running = self.jobs_running.saturating_sub(output.jobs.len());
            self.server_handled = output.handled;
            for &(id, position, value) in &output.orbs_taken {
                let tick_count = self.orbs.iter().find(|orb| orb.id == id).map_or(0, |orb| orb.tick_count);
                let orb = ServerOrb { id, position, previous_position: position, value, tick_count };
                self.orb_pickups.push(minecraft_terrain::orb_render::OrbPickup::new(orb, orb_pickup_target));
            }
            if let Some(mut snapshot) = output.entities {
                self.thrown = std::mem::take(&mut snapshot.thrown_views);
                self.eyes.receive(std::mem::take(&mut snapshot.eyes), &mut self.portal, &mut self.item_crumbs);
                self.clouds = std::mem::take(&mut snapshot.clouds);
                self.tridents = std::mem::take(&mut snapshot.tridents);
                self.crystals = std::mem::take(&mut snapshot.crystals);
                self.dragons = std::mem::take(&mut snapshot.dragons);
                self.dragon_fireballs = std::mem::take(&mut snapshot.dragon_fireballs);
                for at in std::mem::take(&mut snapshot.breath_bursts) {
                    self.spell.burst(at, minecraft_terrain::effect_cloud::BREATH_COLOR, false);
                }
                self.dragon_voices();
                self.orbs = snapshot.orbs.clone();
                self.falling.receive(std::mem::take(&mut snapshot.falling), std::mem::take(&mut snapshot.tnt), &output.changes, self.ticks);
                self.server_snapshot = Some(snapshot);
            }
            for (_, landing) in output.pearl_teleports.into_iter().filter(|&(player, _)| player == PLAYER) {
                self.pearl_landings.push(DVec3::from_array(landing));
            }
            self.player_potions.extend(output.player_potions.into_iter().filter(|hit| hit.player == PLAYER));
            self.player_pushes.extend(output.players_pushed.iter().filter(|(player, _)| *player == PLAYER).map(|(_, push)| *push));
            for broken in &output.potion_breaks {
                self.spell.burst(broken.position, broken.color, broken.instant);
                self.potion_crumbs.push(minecraft_terrain::spell_particles::break_center(broken.position));
            }
            for (from, to) in output.egg_teleports {
                for (start, motion) in minecraft_terrain::dragon_egg::teleport_particles(from, to, &mut self.random) {
                    self.portal.emit(start, motion);
                }
            }
            for hit in output.thrown_hits {
                if hit.kind == minecraft_terrain::thrown::Throwable::EnderPearl {
                    for (start, motion) in minecraft_terrain::thrown::pearl_portal(hit.at, &mut self.random) {
                        self.portal.emit(start, motion);
                    }
                } else {
                    let velocities = minecraft_terrain::thrown::hit_crumbs(hit.kind, &mut self.random);
                    self.item_crumbs.push((hit.item, hit.at, velocities));
                }
            }
            if let Some(weather) = output.weather {
                self.weather = weather;
            }
            if let Some(weather) = output.weather_data {
                self.weather_data = Some(weather);
            }
            self.server_picked.extend(output.picked);
            for used in output.item_uses {
                if let Some(sent) = self.item_use_waiting.take().filter(|_| used) {
                    self.item_uses_done.push(sent);
                }
            }
            for summoned in output.summoned {
                let egg = self.eggs_waiting.answer(inventory, summoned.is_ok());
                if let (Some(egg), Err(error)) = (egg, summoned) {
                    diag::warn!(World, "Minecraft {egg} hatched nothing: {error}");
                }
            }
            self.command_summons.extend(output.command_summons);
            self.dragons_killed.extend(output.dragons_killed);
            self.fight_reports.extend(output.fight_reports);
            self.command_jobs.extend(output.jobs);
            changes.extend(output.changes);
            self.block_entity_changes.extend(output.block_entities);
            self.destroyed.extend(output.destroyed);
            self.experience_found += output.orbs_taken.iter().map(|orb| orb.2 as f32).sum::<f32>();
            let camera = DVec3::from_array(player.feet) + DVec3::Y * f64::from(EYE_HEIGHT);
            for sound in &output.mob_sounds {
                let heard = minecraft_terrain::ender_dragon::heard_at(&sound.event, sound.position, camera);
                self.sounds.push((sound.event.clone(), heard, sound.volume, sound.pitch));
            }
            for sound in output.mob_results.iter().flat_map(|result| result.sounds.iter()) {
                self.sounds.push((sound.event.clone(), sound.position, sound.volume, sound.pitch));
            }
            // `ServerExplosion`'s sound: loud, pitched down.
            for blast in &output.explosions {
                let pitch = (1.0 + (self.random.next_float() - self.random.next_float()) * 0.2) * 0.7;
                self.sounds.push(("minecraft:entity.generic.explode".to_owned(), blast.position, 4.0, pitch));
            }
            self.level_events.extend(output.level_events);
            for effect in &output.explosion_effects {
                self.ambient.explosion(DVec3::from_array(effect.center), effect.radius, effect.blocks, effect.interacts);
            }
            for puff in &output.bullet_puffs {
                self.ambient.puffs(bullet_puff_particle(puff.kind), puff.position, puff.kind.count(), 0.2);
            }
            if let Some(mobs) = output.mobs {
                self.world = *mobs;
                for (feet, width, height) in self.client.receive(server_mobs(&self.world)) {
                    self.poof.spawn(feet, width, height);
                }
            }
            hits.extend(output.players_struck.iter().filter(|struck| struck.player == PLAYER).map(|struck| struck_hit(player.feet, struck)));
            for hit in output.player_hits.into_iter().filter(|h| h.player_id == PLAYER) {
                let type_id = hit.source.and_then(|id| self.entity_type(id));
                hits.extend(mob_hit(&hit, player.feet, self.difficulty, type_id));
            }
        }
        if ticked {
            self.server_items_tick(inventory, selected, DVec3::from_array(player.feet));
            // `ItemEntity.playerTouch`'s pickup pop.
            for _ in 0..self.world_items.take_pickup_sounds() {
                let pitch = ((self.random.next_float() - self.random.next_float()) * 0.7 + 1.0) * 2.0;
                self.sounds.push(("minecraft:entity.item.pickup".to_owned(), DVec3::from_array(player.feet), 0.2, pitch));
            }
        }
        (changes, hits)
    }

    pub fn mob_present(&self, id: u64) -> bool {
        self.entity_type(id).is_some()
    }

    fn entity_type(&self, id: u64) -> Option<&'static str> {
        self.world
            .entity_type(id)
            .or_else(|| self.dragons.iter().any(|dragon| dragon.id == id).then_some(minecraft_terrain::ender_dragon::ENDER_DRAGON))
    }

    pub fn dragons(&self) -> &[minecraft_terrain::ender_dragon::DragonView] {
        &self.dragons
    }

    pub fn take_dragons_killed(&mut self) -> Vec<[i32; 4]> {
        std::mem::take(&mut self.dragons_killed)
    }

    pub fn take_fight_reports(&mut self) -> Vec<minecraft_terrain::server::FightReport> {
        std::mem::take(&mut self.fight_reports)
    }

    pub fn dragon_alive(&self, uuid: [i32; 4]) -> bool {
        self.dragons.iter().any(|dragon| dragon.uuid == uuid)
    }

    fn dragon_voices(&mut self) {
        use minecraft_terrain::ender_dragon::{FLAP_SOUND, GROWL_SOUND, SOUND_VOLUME};
        self.growls.retain(|id, _| self.dragons.iter().any(|dragon| dragon.id == *id));
        for dragon in &self.dragons {
            if dragon_flapping(dragon.o_flap_time, dragon.flap_time) {
                let pitch = 0.8 + self.random.next_float() * 0.3;
                self.sounds.push((FLAP_SOUND.to_owned(), dragon.position, SOUND_VOLUME, pitch));
            }
            if dragon.phase == minecraft_terrain::ender_dragon::Phase::SittingAttacking.id() {
                let pitch = 0.8 + self.random.next_float() * 0.3;
                self.sounds.push((GROWL_SOUND.to_owned(), dragon.position, 2.5, pitch));
            }
            if dragon.sitting {
                continue;
            }
            let growl = self.growls.entry(dragon.id).or_insert(100);
            *growl -= 1;
            if *growl < 0 {
                let pitch = 0.8 + self.random.next_float() * 0.3;
                self.sounds.push((GROWL_SOUND.to_owned(), dragon.position, 2.5, pitch));
                *growl = 200 + self.random.next_int(200) as i32;
            }
        }
    }

    /// Every living mob's key and box in blocks, for bullets.
    pub fn boxes(&self) -> Vec<(u64, [f64; 6])> {
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
        for e in w.ghasts().iter().filter(|e| e.ghast.health > 0.0) {
            add(MobHit::Ghast(e.id), &e.ghast.body);
        }
        for e in w.blazes().iter().filter(|e| e.blaze.health > 0.0) {
            add(MobHit::Blaze(e.id), &e.blaze.body);
        }
        for e in w.shulkers().iter().filter(|e| e.shulker.health > 0.0) {
            let (low, high) = e.shulker.bounding_box();
            out.push((encode(MobHit::Shulker(e.id)), [low.x, low.y, low.z, high.x, high.y, high.z]));
        }
        for crystal in &self.crystals {
            let (low, high) = minecraft_terrain::end_crystal::pick_box(crystal.position);
            out.push((CRYSTAL_KEY | (crystal.id & ((1 << 56) - 1)), [low.x, low.y, low.z, high.x, high.y, high.z]));
        }
        out
    }

    pub fn shot_boxes(&self) -> Vec<(u64, [f64; 6])> {
        let mut out = self.boxes();
        for e in self.world.fireballs().iter().filter(|e| e.fireball.alive && e.fireball.kind.redirectable()) {
            let (min, max) = minecraftoss_entities::fireball::pick_box(e.fireball.kind, e.fireball.position);
            out.push((FIREBALL_KEY | (e.id & ((1 << 56) - 1)), [min.x, min.y, min.z, max.x, max.y, max.z]));
        }
        for e in self.world.shulker_bullets().iter().filter(|e| e.bullet.alive) {
            let (min, max) = e.bullet.bounds();
            out.push((SHULKER_BULLET_KEY | (e.id & ((1 << 56) - 1)), [min.x, min.y, min.z, max.x, max.y, max.z]));
        }
        for dragon in self.dragons.iter().filter(|dragon| dragon.health > 0.0) {
            for (part, &feet) in dragon.parts.iter().enumerate() {
                let (low, high) = minecraft_terrain::ender_dragon::part_box(feet, part);
                out.push((dragon_part_key(dragon.id, part), [low.x, low.y, low.z, high.x, high.y, high.z]));
            }
        }
        out
    }

    /// Steps the client-side puffs, which settle on the scene's blocks.
    pub fn tick_scene(&mut self, scene: &HandcraftedScene, ticks: u32) {
        for _ in 0..ticks {
            self.poof.tick(scene);
            self.ambient.tick(scene);
            for entity in self.world.shulker_bullets().iter().filter(|e| e.bullet.alive) {
                self.ambient.add(minecraft_terrain::ambient_particles::Kind::EndRod, entity.bullet.trail(), DVec3::ZERO);
            }
        }
    }

    /// The mobs this frame, drawn with the viewer's renderers.
    #[allow(clippy::too_many_arguments)]
    pub fn meshes(
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
        magma_cube_render::append_magma_cubes(&mut out.models, w.slimes().iter(), poses, atlas, light, partial);
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
        ghast_render::append_ghasts(&mut out.models, w.ghasts().iter(), poses, atlas, light, partial);
        end_crystal_render::append_crystals(&mut out.models, &self.crystals, atlas, light, partial);
        dragon_render::append_dragons(&mut out.models, &self.dragons, atlas, light, partial);
        dragon_render::append_beams(&mut out.models, &self.dragons, &self.crystals, atlas, light, partial);
        dragon_render::append_rays(&mut out.translucent, &self.dragons, atlas, partial);
        blaze_render::append_blazes(&mut out.models, w.blazes().iter(), poses, atlas, light, partial);
        shulker_render::append_shulkers(&mut out.models, w.shulkers().iter(), poses, atlas, light, partial);
        shulker_render::append_bullets(&mut out.models, &mut out.translucent, w.shulker_bullets(), atlas, partial);
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
        minecraft_terrain::dragon_fireball::append_fireballs(&mut out.items, &self.dragon_fireballs, atlas, forward, partial);
        ghast_render::append_fireballs(
            &mut out.items,
            w.fireballs().iter().filter(|f| f.fireball.alive).map(|f| (f, f.previous_position.lerp(f.fireball.position, f64::from(partial)))),
            atlas,
            forward,
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
        self.lids.settle(scene, cell_settled);
        let lids = self.lids.posed(scene, partial);
        let _ = self.items.append_open_lids(&mut out.items, &lids, scene, packs, atlas, light);
        let _ = self.items.append_primed_tnt(&mut out.items, &self.falling.primed_tnt(partial), packs, atlas, light);
        minecraft_terrain::thrown::append_views(&mut out.items, &self.thrown, partial, camera, atlas, forward, light);
        minecraft_terrain::eye_of_ender::append_eyes(&mut out.items, self.eyes.views(), partial, camera, atlas, forward);
        minecraft_terrain::trident::append_tridents(&mut out.items, &self.tridents, partial, atlas, light);
        self.poof.append_mesh(&mut out.items, atlas, forward, partial, light);
        self.ambient.append_mesh(&mut out.items, &mut out.translucent, atlas, forward, partial, light);
        self.portal.append_mesh(&mut out.items, atlas, forward, partial, light);
        self.spell.append_mesh(&mut out.translucent, atlas, forward, partial, light);
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

fn struck_hit(feet: [f64; 3], struck: &minecraft_terrain::thrown::PlayerStruck) -> (f32, Option<[f64; 3]>, Option<[f64; 2]>, crate::death::DamageCause, Option<(EffectKind, u32)>) {
    let from = DVec3::from_array(feet) - struck.velocity.normalize_or_zero();
    (struck.damage, Some(from.to_array()), Some([-struck.velocity.x, -struck.velocity.z]), crate::death::DamageCause::Generic, None)
}

fn mob_hit(
    hit: &minecraftoss_entities::world::PlayerHit,
    feet: [f64; 3],
    difficulty: i32,
    type_id: Option<&'static str>,
) -> Option<(f32, Option<[f64; 3]>, Option<[f64; 2]>, crate::death::DamageCause, Option<(EffectKind, u32)>)> {
    let from = match hit.kind {
        PlayerHitKind::Melee { attacker, .. } => Some(attacker.to_array()),
        _ => None,
    };
    let toward = match hit.kind {
        PlayerHitKind::Melee { attacker, .. } => [attacker.x - feet[0], attacker.z - feet[2]],
        PlayerHitKind::Arrow { velocity, .. } | PlayerHitKind::Fireball { velocity, .. } | PlayerHitKind::ShulkerBullet { velocity } => [-velocity.x, -velocity.z],
        PlayerHitKind::Explosion { knockback } | PlayerHitKind::BadRespawnPoint { knockback } => [-knockback.x, -knockback.z],
    };
    let attack = match hit.kind {
        PlayerHitKind::Melee { .. } => crate::death::MobAttack::Melee,
        PlayerHitKind::Arrow { .. } => crate::death::MobAttack::Arrow,
        PlayerHitKind::Explosion { .. } | PlayerHitKind::BadRespawnPoint { .. } => crate::death::MobAttack::Explosion,
        PlayerHitKind::Fireball { ignite_ticks, .. } if ignite_ticks > 0 => crate::death::MobAttack::SmallFireball,
        PlayerHitKind::Fireball { .. } => crate::death::MobAttack::Fireball,
        PlayerHitKind::ShulkerBullet { .. } => crate::death::MobAttack::MobProjectile,
    };
    let cause = match hit.kind {
        PlayerHitKind::BadRespawnPoint { .. } => crate::death::DamageCause::BadRespawnPoint,
        _ => crate::death::DamageCause::mob(attack, hit.source, type_id),
    };
    let scales = matches!(hit.kind, PlayerHitKind::Explosion { .. } | PlayerHitKind::BadRespawnPoint { .. }) || hit.source.is_some();
    let damage = difficulty_scaled(hit.damage, difficulty, scales);
    let effect = match hit.kind {
        PlayerHitKind::Arrow { effect, .. } => effect,
        PlayerHitKind::ShulkerBullet { .. } => Some((EffectKind::Levitation, minecraftoss_entities::shulker_bullet::LEVITATION_TICKS)),
        _ => None,
    };
    (damage > 0.0).then_some((damage, from, Some(toward), cause, effect))
}

fn bullet_puff_particle(kind: minecraftoss_entities::world::BulletPuffKind) -> minecraft_terrain::ambient_particles::Kind {
    match kind {
        minecraftoss_entities::world::BulletPuffKind::Hit => minecraft_terrain::ambient_particles::Kind::Explosion,
        minecraftoss_entities::world::BulletPuffKind::Hurt => minecraft_terrain::ambient_particles::Kind::Crit,
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
        MobHit::Ghast(_) => 16,
        MobHit::Blaze(_) => 18,
        MobHit::Shulker(_) => 26,
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
        16 => MobHit::Ghast(id),
        18 => MobHit::Blaze(id),
        26 => MobHit::Shulker(id),
        _ => return None,
    })
}

const FIREBALL_KEY: u64 = 17 << 56;
const SHULKER_BULLET_KEY: u64 = 27 << 56;
const CRYSTAL_KEY: u64 = 19 << 56;
const DRAGON_KEY: u64 = 20 << 56;
pub const FLAT_HIT: u64 = 1 << 55;

fn dragon_part_key(id: u64, part: usize) -> u64 {
    DRAGON_KEY | FLAT_HIT | ((id << 3) & (FLAT_HIT - 1)) | part as u64
}

fn end_target_of(key: u64) -> Option<EndTarget> {
    if let Some(id) = crystal_of(key) {
        return Some(EndTarget::Crystal(id));
    }
    (key >> 56 == DRAGON_KEY >> 56).then_some(EndTarget::DragonPart {
        id: (key & (FLAT_HIT - 1)) >> 3,
        part: (key & 7) as usize,
    })
}

fn dragon_flapping(o_flap_time: f32, flap_time: f32) -> bool {
    let before = minecraftoss_player::mth::cos(f64::from(o_flap_time * std::f32::consts::TAU));
    let now = minecraftoss_player::mth::cos(f64::from(flap_time * std::f32::consts::TAU));
    before <= -0.3 && now >= -0.3
}

fn crystal_of(key: u64) -> Option<u64> {
    (key >> 56 == CRYSTAL_KEY >> 56).then_some(key & ((1 << 56) - 1))
}

fn fireball_of(key: u64) -> Option<u64> {
    (key >> 56 == FIREBALL_KEY >> 56).then_some(key & ((1 << 56) - 1))
}

fn shulker_bullet_of(key: u64) -> Option<u64> {
    (key >> 56 == SHULKER_BULLET_KEY >> 56).then_some(key & ((1 << 56) - 1))
}

fn take_picked(inventory: &mut Inventory, selected: usize, picked: Vec<Picked>) -> Vec<ItemStack> {
    let mut rests = Vec::new();
    for (_, _, item, count, components) in picked {
        let mut stack = ItemStack::new(&item, count.clamp(0, 255) as u8);
        stack.components = components.as_deref().and_then(|c| serde_json::from_str(c).ok());
        stack.max = stack.max.min(inventory.recipes.max_stack(&item));
        rests.extend(inventory.add_item(stack, selected));
    }
    rests
}

fn weather_name(weather: &WeatherData) -> &'static str {
    match (weather.raining, weather.thundering) {
        (true, true) => "thunder",
        (true, false) => "rain",
        (false, _) => "clear",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_weather_is_named_like_the_weather_command() {
        let clear = WeatherData { clear_weather_time: 600, ..WeatherData::default() };
        assert_eq!(weather_name(&clear), "clear");
        assert_eq!(weather_name(&WeatherData { thundering: true, ..clear }), "clear");
        assert_eq!(weather_name(&WeatherData { raining: true, ..clear }), "rain");
        assert_eq!(weather_name(&WeatherData { raining: true, thundering: true, ..clear }), "thunder");
    }

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
    fn a_dragon_flaps_when_its_wings_pass_the_top_of_their_beat() {
        assert!(dragon_flapping(0.69, 0.71), "cos rises through -0.3 late in the beat");
        assert!(!dragon_flapping(0.1, 0.2));
        assert!(!dragon_flapping(0.55, 0.6));
        let flaps = (0..400).filter(|&i| dragon_flapping(i as f32 * 0.01, (i + 1) as f32 * 0.01)).count();
        assert_eq!(flaps, 4, "once a beat");
    }

    #[test]
    fn dragon_parts_are_flat_hit_boxes_keyed_by_dragon_and_part() {
        let id = minecraft_terrain::ender_dragon::FIRST_ID + 7;
        for part in 0..8 {
            let key = dragon_part_key(id, part);
            assert_ne!(key & FLAT_HIT, 0, "MW2 headshot scaling never stacks on the dragon");
            assert_eq!(end_target_of(key), Some(EndTarget::DragonPart { id, part }));
            assert_eq!(decode(key), None);
            assert_eq!(fireball_of(key), None);
        }
        assert_eq!(end_target_of(CRYSTAL_KEY | 9), Some(EndTarget::Crystal(9)));
        assert_eq!(end_target_of(encode(MobHit::Zombie(9))), None);
    }

    #[test]
    fn end_crystals_have_their_own_bullet_key() {
        let key = CRYSTAL_KEY | 42;
        assert_eq!(crystal_of(key), Some(42));
        assert_eq!(fireball_of(key), None);
        assert_eq!(decode(key), None);
        assert_eq!(crystal_of(FIREBALL_KEY | 42), None);
        assert_eq!(crystal_of(encode(MobHit::Blaze(42))), None);
    }

    #[test]
    fn shulkers_take_bullet_key_26() {
        let key = encode(MobHit::Shulker(42));
        assert_eq!(key >> 56, 26);
        assert_eq!(decode(key), Some(MobHit::Shulker(42)));
        assert_eq!((crystal_of(key), fireball_of(key), end_target_of(key)), (None, None, None));
    }

    #[test]
    fn shulker_bullets_take_bullet_key_27() {
        let key = SHULKER_BULLET_KEY | 42;
        assert_eq!((key >> 56, shulker_bullet_of(key)), (27, Some(42)));
        assert_eq!((decode(key), crystal_of(key), fireball_of(key), end_target_of(key)), (None, None, None, None));
        assert_eq!(shulker_bullet_of(encode(MobHit::Shulker(42))), None);
    }

    #[test]
    fn a_struck_bullet_sparks_and_one_that_lands_puffs() {
        use minecraft_terrain::ambient_particles::Kind;
        use minecraftoss_entities::world::BulletPuffKind;
        assert_eq!((bullet_puff_particle(BulletPuffKind::Hit), BulletPuffKind::Hit.count()), (Kind::Explosion, 2));
        assert_eq!((bullet_puff_particle(BulletPuffKind::Hurt), BulletPuffKind::Hurt.count()), (Kind::Crit, 15));
    }

    #[test]
    fn a_shulker_bullet_hurts_by_difficulty_and_lifts_for_ten_seconds() {
        let hit = minecraftoss_entities::world::PlayerHit {
            player_id: PLAYER,
            damage: 4.0,
            kind: PlayerHitKind::ShulkerBullet { velocity: DVec3::new(0.15, 0.0, 0.0) },
            source: Some(9),
        };
        let damage = |difficulty| mob_hit(&hit, [0.0; 3], difficulty, Some("minecraft:shulker")).map(|h| h.0);
        assert_eq!((damage(0), damage(1), damage(2), damage(3)), (None, Some(3.0), Some(4.0), Some(6.0)));
        let (_, from, toward, cause, effect) = mob_hit(&hit, [0.0; 3], 2, Some("minecraft:shulker")).unwrap();
        assert_eq!((from, toward), (None, Some([-0.15, -0.0])));
        assert_eq!(cause, crate::death::DamageCause::Mob { id: 9, type_id: "minecraft:shulker", attack: crate::death::MobAttack::MobProjectile });
        assert_eq!(effect, Some((EffectKind::Levitation, 200)));
        let ownerless = minecraftoss_entities::world::PlayerHit { source: None, ..hit };
        let (damage, _, _, cause, effect) = mob_hit(&ownerless, [0.0; 3], 3, None).unwrap();
        assert_eq!((damage, cause, effect), (4.0, crate::death::DamageCause::Generic, Some((EffectKind::Levitation, 200))));
    }

    #[test]
    fn blasts_hurt_mobs_four_times_as_hard_as_bullets() {
        assert_eq!(HEALTH_SCALE, 0.2);
        assert_eq!(BLAST_HEALTH_SCALE, 0.8);
        assert_eq!(BLAST_HEALTH_SCALE, 4.0 * HEALTH_SCALE);
    }

    #[test]
    fn a_thrown_trident_hits_the_player_for_8_from_where_it_flew_in() {
        let struck = minecraft_terrain::thrown::PlayerStruck {
            player: PLAYER,
            by: 3,
            damage: minecraft_terrain::trident::DAMAGE,
            kind: minecraft_terrain::trident::DAMAGE_TYPE,
            velocity: DVec3::new(2.0, 0.0, 0.0),
        };
        let (damage, from, toward, cause, _) = struck_hit([10.0, 64.0, 5.0], &struck);
        assert_eq!(damage, 8.0);
        assert_eq!(from, Some([9.0, 64.0, 5.0]));
        assert_eq!(toward, Some([-2.0, -0.0]));
        assert_eq!(cause, crate::death::DamageCause::Generic);
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

    #[test]
    fn stacks_the_old_level_let_the_player_pick_up_reach_the_inventory_on_leaving() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:dirt", 60));
        let picked = vec![
            (7, [1.0, 64.0, 1.0], "minecraft:dirt".to_owned(), 10, None),
            (8, [1.0, 64.0, 1.0], "minecraft:diamond_sword".to_owned(), 1, Some(r#"{"minecraft:damage":5}"#.to_owned())),
        ];
        assert!(take_picked(&mut inventory, 0, picked).is_empty());
        let count = |id: &str| inventory.slots.iter().flatten().filter(|stack| stack.id == id).map(|stack| u32::from(stack.count)).sum::<u32>();
        assert_eq!(count("minecraft:dirt"), 70);
        let sword = inventory.slots.iter().flatten().find(|stack| stack.id == "minecraft:diamond_sword").expect("the sword");
        assert_eq!(sword.components, Some(serde_json::json!({ "minecraft:damage": 5 })));
    }

    #[test]
    fn stacks_that_no_longer_fit_on_leaving_come_back_to_be_dropped() {
        let mut inventory = Inventory::default();
        for slot in inventory.slots.iter_mut().take(36) {
            *slot = Some(ItemStack::new("minecraft:stone", 64));
        }
        let picked = vec![(7, [1.0, 64.0, 1.0], "minecraft:dirt".to_owned(), 10, Some(r#"{"minecraft:custom_name":"x"}"#.to_owned()))];
        let rests = take_picked(&mut inventory, 0, picked);
        assert_eq!(rests.len(), 1, "the part that did not fit is not lost");
        assert_eq!((rests[0].id.as_str(), rests[0].count), ("minecraft:dirt", 10));
        assert_eq!(rests[0].components, Some(serde_json::json!({ "minecraft:custom_name": "x" })));
    }
}
