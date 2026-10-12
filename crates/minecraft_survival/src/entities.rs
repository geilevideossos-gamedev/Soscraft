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
use minecraft_terrain::server::{HowBroken, PlayerEdit, ServerHandle, ServerSim, TickInput, TickPlayer};
use minecraft_terrain::terrain::{BlockStates, TerrainStream};
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
use minecraft_terrain::item_frame::FrameAction;

const TICK_SECONDS: f64 = 1.0 / 20.0;
/// Minecraft health per MW2 health.
const HEALTH_SCALE: f32 = 20.0 / 100.0;
const BLAST_HEALTH_SCALE: f32 = 80.0 / 100.0;
/// `Player.getEyeHeight` standing.
const EYE_HEIGHT: f32 = 1.62;
const SERVER_ITEM: u32 = 1 << 31;
const GUARDED_BY_PIGLINS: &str = "minecraft:guarded_by_piglins";
pub type Hit = (f32, Option<[f64; 3]>, Option<[f64; 2]>, crate::death::DamageCause, crate::impulse::MeleePush, Option<(EffectKind, u32)>);
pub type Hits = Vec<Hit>;
type Changes = Vec<((i32, i32, i32), Option<Block>)>;
type Picked = (i32, [f64; 3], String, i32, Option<String>);

pub struct Blow {
    pub sprinting: bool,
    pub can_critical: bool,
    pub can_sweep: bool,
    pub reach: f64,
    pub block: Option<f64>,
    pub creative: bool,
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
    bodies: HashMap<u128, minecraft_terrain::ambient_particles::Body>,
    level_events: Vec<(i32, (i32, i32, i32), i32)>,
    pub broken_items: Vec<String>,
    items: ItemVisuals,
    clock: f64,
    tick_seconds: f64,
    ticks: u64,
    /// Item entities as the client shows them: the server's, and drops the
    /// client spawned that the server has not taken yet.
    pub world_items: WorldItems,
    /// Client drops handed to the server: the command count that sent each.
    server_handed: HashMap<u32, (u64, ItemEntity)>,
    burning_items: Vec<u32>,
    players: PlayerQueues,
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
    gateways: minecraft_terrain::end_gateway::ClientGateways,
    dragons: Vec<minecraft_terrain::ender_dragon::DragonView>,
    dragon_fireballs: Vec<minecraft_terrain::dragon_fireball::FireballView>,
    frames: Vec<minecraft_terrain::item_frame::FrameView>,
    growls: HashMap<u64, i32>,
    dragons_killed: Vec<[i32; 4]>,
    fight_reports: Vec<minecraft_terrain::server::FightReport>,
    orbs: Vec<ServerOrb>,
    falling: minecraft_terrain::falling_render::FallingBlocks,
    pub lids: minecraft_terrain::lids::Lids,
    spawners: minecraft_terrain::spawner_display::SpawnerDisplays,
    orb_pickups: Vec<minecraft_terrain::orb_render::OrbPickup>,
    pub item_crumbs: Vec<(String, DVec3, Vec<Vec3>)>,
    pub potion_crumbs: Vec<DVec3>,
    pub destroyed: Vec<((i32, i32, i32), Block)>,
    pub weather: minecraft_terrain::server::WeatherView,
    pub weather_data: Option<WeatherData>,
    pub world_weather: Option<WeatherData>,
    pub stations: std::sync::Arc<minecraft_terrain::stations::StationBook>,
    pub enchantments: Option<std::sync::Arc<minecraftoss_entities::enchant::Enchantments>>,
    pub recipes: std::sync::Arc<minecraftoss_player::crafting::RecipeBook>,
    pub command_summons: Vec<(minecraft_commands::PlayerId, Result<String, String>)>,
    pub command_jobs: Vec<Box<dyn std::any::Any + Send>>,
    jobs_running: usize,
    unread: Vec<minecraft_terrain::server::Output>,
    finished_ticks: Vec<u64>,
    pub difficulty: i32,
    spawn_mobs: bool,
    block_drops: bool,
    presenter: Option<u64>,
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
#[derive(Clone)]
pub struct PlayerView {
    pub feet: [f64; 3],
    pub alive: bool,
    pub attackable: bool,
    pub spectator: bool,
    pub picks_up: bool,
    pub health: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub main_hand: Option<String>,
    pub offhand: Option<String>,
    pub sneaking: bool,
}

fn player_controls(id: u64, player: &PlayerView, inventory: &Inventory) -> minecraftoss_entities::controls::PlayerControls {
    minecraftoss_entities::controls::PlayerControls {
        id,
        yaw: player.yaw,
        pitch: player.pitch,
        main_hand: player.main_hand.clone(),
        offhand: player.offhand.clone(),
        sneaking: player.sneaking,
        armor: [36, 37, 38, 39].map(|slot| inventory.slots[slot].as_ref().map(|stack| stack.id.clone())),
    }
}

pub struct TickActor<'a> {
    pub id: u64,
    pub view: PlayerView,
    pub inventory: &'a mut Inventory,
    pub selected: usize,
}

#[derive(Default)]
pub struct PlayerOutputs {
    pub hits: Hits,
    pub pearl_landings: Vec<DVec3>,
    pub potions: Vec<minecraft_terrain::potions::PlayerPotionHit>,
    pub pushes: Vec<DVec3>,
    pub item_uses_done: Vec<SentItemUse>,
}

#[derive(Default)]
pub(crate) struct PlayerQueue {
    uuid: Option<u128>,
    feet: Option<[f64; 3]>,
    picked: Vec<Picked>,
    pub(crate) pearl_gateways: Vec<(i32, i32, i32)>,
    experience: u32,
    item_use_waiting: Option<SentItemUse>,
    eggs_waiting: minecraft_terrain::spawn_egg::EggsWaiting,
    egg_answers: Vec<Result<String, String>>,
    mob_slots: Vec<Vec<(usize, Option<ItemStack>)>>,
    broken: Vec<String>,
    mounts: Vec<u64>,
    dismounts: Vec<(u64, Option<DVec3>)>,
    boost_waiting: Option<(usize, String)>,
    boosts_done: Vec<(usize, String)>,
    lecterns: Vec<minecraft_terrain::lectern::LecternView>,
    pub(crate) out: PlayerOutputs,
}

#[derive(Default)]
struct Unclaimed {
    picked: Vec<Picked>,
    orbs: Vec<([f64; 3], i32)>,
}

#[derive(Default)]
struct PlayerQueues(HashMap<u64, PlayerQueue>);

impl PlayerQueues {
    fn queue(&mut self, id: u64) -> &mut PlayerQueue {
        self.0.entry(id).or_default()
    }

    fn route(&mut self, output: &mut minecraft_terrain::server::Output, difficulty: i32, entity_type: impl Fn(u64) -> Option<&'static str>) -> Unclaimed {
        let mut unclaimed = Unclaimed::default();
        for (player, picked) in std::mem::take(&mut output.picked) {
            match self.0.get_mut(&player) {
                Some(queue) => queue.picked.push(picked),
                None => unclaimed.picked.push(picked),
            }
        }
        for (player, _, position, value) in std::mem::take(&mut output.orbs_taken) {
            match self.0.get_mut(&player) {
                Some(queue) => queue.experience = queue.experience.saturating_add(value.max(0) as u32),
                None => unclaimed.orbs.push((position, value)),
            }
        }
        for (player, landing) in std::mem::take(&mut output.pearl_teleports) {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.out.pearl_landings.push(DVec3::from_array(landing));
            }
        }
        for (player, gateway) in std::mem::take(&mut output.pearl_gateways) {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.pearl_gateways.push(gateway);
            }
        }
        for hit in std::mem::take(&mut output.player_potions) {
            if let Some(queue) = self.0.get_mut(&hit.player) {
                queue.out.potions.push(hit);
            }
        }
        for &(player, push) in &output.players_pushed {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.out.pushes.push(push);
            }
        }
        for (player, used) in std::mem::take(&mut output.item_uses) {
            if let Some(queue) = self.0.get_mut(&player)
                && let Some(sent) = queue.item_use_waiting.take().filter(|_| used)
            {
                queue.out.item_uses_done.push(sent);
            }
        }
        for (player, summoned) in std::mem::take(&mut output.summoned) {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.egg_answers.push(summoned);
            }
        }
        for struck in &output.players_struck {
            if let Some(queue) = self.0.get_mut(&struck.player)
                && let Some(feet) = queue.feet
            {
                queue.out.hits.push(struck_hit(feet, struck));
            }
        }
        for hit in std::mem::take(&mut output.player_hits) {
            let Some(queue) = self.0.get_mut(&hit.player_id) else {
                continue;
            };
            let Some(feet) = queue.feet else {
                continue;
            };
            if let Some(hit) = mob_hit(&hit, feet, difficulty, hit.source.and_then(&entity_type)) {
                queue.out.hits.push(hit);
            }
        }
        for (player, result) in &output.mob_results {
            if let Some(queue) = self.0.get_mut(player) {
                queue.mob_slots.push(result.slots.clone());
                queue.broken.extend(result.broken.iter().cloned());
                queue.mounts.extend(result.mounted);
            }
        }
        for &(player, vehicle, spot) in &output.dismounts {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.dismounts.push((vehicle, spot.map(DVec3::from_array)));
            }
        }
        for &(player, vehicle) in &output.mounts {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.mounts.push(vehicle);
            }
        }
        for &(player, boosted) in &output.boosts {
            if let Some(queue) = self.0.get_mut(&player)
                && let Some(sent) = queue.boost_waiting.take().filter(|_| boosted)
            {
                queue.boosts_done.push(sent);
            }
        }
        for view in std::mem::take(&mut output.lecterns) {
            if let Some(queue) = self.0.get_mut(&view.player) {
                queue.lecterns.push(view);
            }
        }
        unclaimed
    }
}

fn player_candidate(id: u64, player: &PlayerView) -> PlayerCandidate {
    PlayerCandidate {
        id,
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
            bodies: HashMap::new(),
            level_events: Vec::new(),
            broken_items: Vec::new(),
            items: ItemVisuals::default(),
            clock: 0.0,
            tick_seconds: TICK_SECONDS,
            ticks: 0,
            world_items: WorldItems::default(),
            server_handed: HashMap::new(),
            burning_items: Vec::new(),
            players: PlayerQueues::default(),
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
            gateways: Default::default(),
            frames: Vec::new(),
            dragons: Vec::new(),
            dragon_fireballs: Vec::new(),
            growls: HashMap::new(),
            dragons_killed: Vec::new(),
            fight_reports: Vec::new(),
            orbs: Vec::new(),
            falling: Default::default(),
            lids: Default::default(),
            spawners: minecraft_terrain::spawner_display::SpawnerDisplays::new((seed ^ 0x5b3d) as u64),
            orb_pickups: Vec::new(),
            item_crumbs: Vec::new(),
            potion_crumbs: Vec::new(),
            destroyed: Vec::new(),
            weather: Default::default(),
            weather_data,
            world_weather: None,
            stations: data.stations.clone(),
            enchantments: data.enchantments.clone(),
            recipes: data.recipes.clone(),
            command_summons: Vec::new(),
            command_jobs: Vec::new(),
            jobs_running: 0,
            unread: Vec::new(),
            finished_ticks: Vec::new(),
            difficulty: 2,
            spawn_mobs: true,
            block_drops: true,
            presenter: None,
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

    pub fn reset_weather_cycle(&mut self) -> Option<WeatherData> {
        let data = crate::sleep::cleared_weather(self.weather_data?);
        self.set_weather_data(data);
        Some(data)
    }

    pub fn wake_villager_in(&mut self, head: (i32, i32, i32)) -> bool {
        let asleep = villager_asleep_in(&self.world, head);
        if asleep {
            self.server.wake_villager_in(head);
        }
        asleep
    }

    pub fn set_players(&mut self, players: &[(u64, minecraft_net::PlayerUuid)]) {
        for (id, uuid) in players {
            let key = crate::level::opener_key(uuid);
            let queue = self.players.queue(*id);
            if queue.uuid != Some(key) {
                queue.uuid = Some(key);
                self.server.set_player_uuid(*id, key);
            }
        }
    }

    pub fn take_player_outputs(&mut self, player: u64) -> PlayerOutputs {
        self.players.0.get_mut(&player).map_or_else(PlayerOutputs::default, |queue| std::mem::take(&mut queue.out))
    }

    pub fn player_outputs(&self, player: u64) -> Option<&PlayerOutputs> {
        self.players.0.get(&player).map(|queue| &queue.out)
    }

    pub fn take_experience(&mut self, player: u64) -> u32 {
        self.players.0.get_mut(&player).map_or(0, |queue| std::mem::take(&mut queue.experience))
    }

    pub fn take_pearl_gateways(&mut self, player: u64) -> Vec<(i32, i32, i32)> {
        self.players.0.get_mut(&player).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.pearl_gateways))
    }

    pub fn land_pearl(&mut self, player: u64, landing: DVec3) {
        if let Some(queue) = self.players.0.get_mut(&player) {
            queue.out.pearl_landings.push(landing);
        }
    }

    #[cfg(test)]
    pub(crate) fn queue(&mut self, player: u64) -> &mut PlayerQueue {
        self.players.queue(player)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn throw(&mut self, player: u64, stack: &ItemStack, eye: DVec3, yaw: f32, pitch: f32, motion: DVec3, on_ground: bool) -> bool {
        let throw = minecraft_terrain::thrown::Throw { owner: player, item: &stack.id, components: stack.components.as_ref(), eye, yaw, pitch, motion, on_ground };
        let Some(thrown) = minecraft_terrain::thrown::Thrown::launch(&throw, &mut self.random) else {
            return false;
        };
        self.server.throw(thrown);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn release_trident(&mut self, player: u64, inventory: &mut Inventory, charging: crate::trident::Charging, creative: bool, wet: bool, eye: DVec3, yaw: f32, pitch: f32, motion: DVec3, on_ground: bool) -> minecraft_terrain::trident::Release {
        use minecraft_terrain::trident::{Release, ThrownTrident};
        let release = minecraft_terrain::trident::release(inventory, charging.slot, charging.ticks, creative, wet, &mut self.random);
        if let Release::Throw(stack) = &release {
            let throw = minecraft_terrain::thrown::Throw { owner: player, item: &stack.id, components: stack.components.as_ref(), eye, yaw, pitch, motion, on_ground };
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
        self.spawners.load_chunk(chunk);
        self.server.load_chunk(chunk);
    }

    pub fn unload_chunk(&mut self, pos: minecraftoss_core::ChunkPos) {
        self.spawners.unload_chunk(pos);
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
        for (pos, block, blast) in broken {
            let block = minecraftoss_player::Block { id: block.id.key(), properties: block.properties.clone() };
            let tool = if blast.is_none() && by_hand.contains(pos) { held } else { Some(&tool) };
            let drops = self
                .loot
                .as_ref()
                .and_then(|loot| loot.roll_drops_named(&block, tool, self.seed as u64, &mut self.loot_sequences));
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
    fn server_items_tick(&mut self, actors: &mut [TickActor<'_>]) {
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
        let presenter = actors.first().map(|actor| actor.id);
        let target = DVec3::from_array(actors.first().map_or([0.0; 3], |actor| actor.view.feet)) + DVec3::Y * 0.81;
        self.world_items.tick_pickup_effects(target);
        for actor in actors.iter_mut() {
            let feet = DVec3::from_array(actor.view.feet);
            let picked = self.players.0.get_mut(&actor.id).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.picked));
            for (id, position, item, count, components) in picked {
                let recipes = actor.inventory.recipes.clone();
                let make_stack = |item: &str, count: i32| {
                    let mut stack = ItemStack::new(item, count.clamp(0, 255) as u8);
                    stack.components = components.as_deref().and_then(|c| serde_json::from_str(c).ok());
                    stack.max = stack.max.min(recipes.max_stack(item));
                    stack
                };
                let taken = match actor.inventory.add_item(make_stack(&item, count), actor.selected) {
                    None => count,
                    Some(rest) => {
                        let rest_count = i32::from(rest.count);
                        self.server.spawn_item(&item, rest_count, components.as_deref(), feet.to_array(), [0.0; 3], 0, 0);
                        count - rest_count
                    }
                };
                if taken <= 0 || presenter != Some(actor.id) {
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
        }
        let _ = self.world_items.take_pickup_transfers();
        let Some(snapshot) = self.server_snapshot.take() else {
            self.world_items.entities = previous.into_values().collect();
            self.world_items.entities.sort_by_key(|e| e.entity_id);
            return;
        };
        let recipes = actors.first().map_or_else(|| self.recipes.clone(), |actor| actor.inventory.recipes.clone());
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
    pub fn punch(&mut self, player: u64, inventory: &Inventory, selected: usize, eye: DVec3, look: DVec3, yaw: f32, attack_damage: f64, strength: f32, blow: Blow) -> bool {
        if let Some(frame) = frame_target(&self.frames, eye, look, blow.reach, blow.block, self.entity_distance(player, eye, look, blow.reach)) {
            self.server.frame_action(player, frame, FrameAction::Attack { infinite: blow.creative });
            return true;
        }
        let mob = mob_on_ray(&self.world, player, eye, look, blow.reach);
        if let Some((target, distance)) = self.end_on_ray(eye, look, blow.reach)
            && mob.is_none_or(|(_, nearest)| distance < nearest)
        {
            let damage = attack_damage as f32 * (0.2 + strength * strength * 0.8);
            self.server.end_hit(target, EndDamage::attack(damage, player).from_position(eye));
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
            self.server.deflect_fireball(fireball, look.normalize_or_zero().to_array(), player, true);
            let feet = eye - DVec3::Y * f64::from(EYE_HEIGHT);
            self.sounds.push(("minecraft:entity.player.attack.nodamage".to_owned(), feet, 1.0, 1.0));
            return true;
        }
        let Some((hit, _)) = mob else {
            return false;
        };
        let attack = minecraftoss_entities::world::PlayerAttack {
            player_id: player,
            position: eye - DVec3::Y * f64::from(EYE_HEIGHT),
            yaw,
            attack_damage,
            strength,
            sprinting: blow.sprinting,
            can_critical: blow.can_critical,
            can_sweep: blow.can_sweep,
        };
        self.server.mob_action(player, hit, Some(attack), inventory, selected, false);
        true
    }

    pub fn mob_distance(&self, player: u64, eye: DVec3, look: DVec3, reach: f64) -> Option<f64> {
        let frame = minecraft_terrain::item_frame::on_ray(&self.frames, eye, look, reach).map(|(_, distance)| distance);
        self.entity_distance(player, eye, look, reach).into_iter().chain(frame).min_by(f64::total_cmp)
    }

    fn entity_distance(&self, player: u64, eye: DVec3, look: DVec3, reach: f64) -> Option<f64> {
        let mob = mob_on_ray(&self.world, player, eye, look, reach).map(|(_, distance)| distance);
        let fireball = self.world.fireball_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let bullet = self.world.shulker_bullet_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let end = self.end_on_ray(eye, look, reach).map(|(_, distance)| distance);
        [mob, fireball, bullet, end].into_iter().flatten().min_by(f64::total_cmp)
    }

    pub fn frame_on_ray(&self, player: u64, eye: DVec3, look: DVec3, reach: f64, block: Option<f64>) -> Option<u64> {
        frame_target(&self.frames, eye, look, reach, block, self.entity_distance(player, eye, look, reach))
    }

    pub fn use_frame(&mut self, player: u64, id: u64, held: Option<&ItemStack>, infinite: bool, sent: Option<SentItemUse>) -> bool {
        let Some(view) = self.frames.iter().find(|view| view.id == id) else {
            return false;
        };
        if minecraft_terrain::item_frame::use_passes(view, held.is_some()) {
            return false;
        }
        if held.is_some() && self.awaits_item_use(player) {
            return true;
        }
        self.server.frame_action(player, id, FrameAction::Use { held: held.cloned(), infinite });
        if held.is_some() {
            self.players.queue(player).item_use_waiting = sent;
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn use_on_mob(&mut self, player: u64, eye: DVec3, look: DVec3, reach: f64, block: Option<f64>, inventory: &Inventory, slot: usize, infinite: bool) -> bool {
        let fireball = self.world.fireball_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let end = self.end_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let others = fireball.into_iter().chain(end).min_by(f64::total_cmp);
        let Some(hit) = mob_to_use(&self.world, player, eye, look, reach, block, others) else {
            return false;
        };
        if !minecraft_terrain::mob_actions::would_interact(&self.world, hit, inventory, slot, infinite, player) {
            return false;
        }
        self.server.mob_action(player, hit, None, inventory, slot, infinite);
        true
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

    pub fn item_on_block(&mut self, player: u64, pos: (i32, i32, i32), action: minecraft_terrain::server::ItemOnBlock, sent: SentItemUse) {
        self.server.item_on_block(player, pos, action);
        self.players.queue(player).item_use_waiting = Some(sent);
    }

    pub fn throw_eye(&mut self, player: u64, throw: minecraft_terrain::ender_eye::EyeThrow, sent: SentItemUse) {
        self.server.throw_eye(player, throw);
        self.players.queue(player).item_use_waiting = Some(sent);
    }

    pub fn bottle_breath(&mut self, player: u64, feet: DVec3, height: f64, sent: SentItemUse) {
        self.server.bottle_breath(player, feet, height);
        self.players.queue(player).item_use_waiting = Some(sent);
    }

    pub fn dragon_breath_near(&self, feet: DVec3, height: f64) -> bool {
        minecraft_terrain::effect_cloud::breath_near(&self.clouds, minecraft_terrain::effect_cloud::bottle_reach(feet, height))
    }

    pub fn awaits_item_use(&self, player: u64) -> bool {
        self.players.0.get(&player).is_some_and(|queue| queue.item_use_waiting.is_some())
    }

    pub fn take_mounts(&mut self, player: u64) -> Vec<u64> {
        self.players.0.get_mut(&player).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.mounts))
    }

    pub fn take_dismounts(&mut self, player: u64) -> Vec<(u64, Option<DVec3>)> {
        self.players.0.get_mut(&player).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.dismounts))
    }

    pub fn take_boosts(&mut self, player: u64) -> Vec<(usize, String)> {
        self.players.0.get_mut(&player).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.boosts_done))
    }

    pub fn take_lecterns(&mut self, player: u64) -> Vec<minecraft_terrain::lectern::LecternView> {
        self.players.0.get_mut(&player).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.lecterns))
    }

    pub fn vehicle_uuid(&self, vehicle: u64) -> u128 {
        self.world.uuid_of(vehicle)
    }

    pub fn ride_seat(&mut self, player: u64, vehicle: u64) -> Option<DVec3> {
        if self.presenter == Some(player) {
            self.client.set_own_vehicle(Some(vehicle));
        }
        let mob = self.client.get(vehicle)?;
        let partial = (self.clock / self.tick_seconds).clamp(0.0, 1.0);
        let pose = mob.pose(partial as f32);
        let baby = self.world.striders().iter().any(|e| e.id == vehicle && e.strider.age.baby());
        let attachment = minecraftoss_entities::strider::Strider::riding_attachment(baby);
        let bob = crate::riding::bob(pose.walk_position, pose.walk_speed);
        Some(crate::riding::seat(mob.old_position, mob.position, partial, attachment, bob))
    }

    pub fn dismount(&mut self, player: u64) {
        self.server.dismount(player);
        self.leave_seat(player);
    }

    pub fn reattach(&mut self, player: u64, root: minecraftoss_core::nbt::Tag) {
        self.server.reattach(player, root);
    }

    pub fn root_vehicle(&mut self, player: u64, remove: bool) -> Option<minecraftoss_core::nbt::Tag> {
        self.server.root_vehicle(player, remove);
        self.server.wait_idle();
        let mut found = None;
        for mut output in self.server.poll() {
            output.root_vehicles.retain(|(rider, root)| {
                if *rider == player {
                    found = root.clone();
                }
                *rider != player
            });
            self.unread.push(output);
        }
        if remove {
            self.leave_seat(player);
        }
        found
    }

    pub fn leave_seat(&mut self, player: u64) {
        if self.presenter == Some(player) {
            self.client.set_own_vehicle(None);
        }
    }

    pub fn can_boost(&self, player: u64, vehicle: u64) -> bool {
        self.players.0.get(&player).is_none_or(|queue| queue.boost_waiting.is_none())
            && self.world.controlling_player(vehicle) == Some(player)
            && self.world.striders().iter().any(|e| e.id == vehicle && !e.strider.steering.boosting)
    }

    pub fn boost(&mut self, player: u64, slot: usize, held: String) {
        self.server.boost(player);
        self.players.queue(player).boost_waiting = Some((slot, held));
    }

    pub fn spend_boost(&mut self, inventory: &mut Inventory, slot: usize, infinite: bool) -> Option<minecraft_terrain::bucket::Sound> {
        let random = &mut self.random;
        if !minecraft_terrain::durability::spend_boost(inventory, slot, infinite, || random.next_float()) {
            return None;
        }
        self.broken_items.push(minecraft_terrain::durability::STEERING_ITEM.to_owned());
        Some(minecraft_terrain::flint::break_sound(self.random.next_float()))
    }

    pub fn leave(&mut self, player: u64, inventory: &mut Inventory, selected: usize, feet: Option<[f64; 3]>) -> u32 {
        self.server.player_left(player);
        self.server.wait_idle();
        self.poll_jobs();
        let (mut picked, mut experience) = take_late(&mut self.unread, player);
        if let Some(mut queue) = self.players.0.remove(&player) {
            for summoned in std::mem::take(&mut queue.egg_answers) {
                queue.eggs_waiting.answer(inventory, summoned.is_ok());
            }
            picked.splice(0..0, queue.picked);
            experience = experience.saturating_add(queue.experience);
        }
        for rest in take_picked(inventory, selected, picked) {
            if let Some(feet) = feet {
                self.server.spawn_item(&rest.id, i32::from(rest.count), rest.components.map(|c| c.to_string()).as_deref(), feet, [0.0; 3], 0, 0);
            }
        }
        experience
    }

    pub fn hatch_egg(&mut self, player: u64, inventory: &Inventory, slot: usize, kind: String, position: [f64; 3], spends: bool) {
        if self.players.queue(player).eggs_waiting.send_from(inventory, slot, spends) {
            let y_rot = minecraftoss_core::entity_data::wrap_degrees(self.random.next_float() * 360.0);
            self.server.hatch_egg(player, kind, position, y_rot);
        }
    }

    pub fn egg_on_spawner(&mut self, player: u64, inventory: &Inventory, slot: usize, kind: String, pos: (i32, i32, i32), spends: bool) {
        if self.players.queue(player).eggs_waiting.send_from(inventory, slot, spends) {
            self.server.spawner_egg(player, pos, kind);
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
        for mut output in self.poll_server() {
            self.jobs_running = self.jobs_running.saturating_sub(output.jobs.len());
            self.command_jobs.append(&mut output.jobs);
            self.block_entity_changes.extend(output.block_entities);
        }
        self.take_block_entity_changes()
    }

    pub fn summon(&mut self, source: minecraft_commands::PlayerId, kind: String, position: [f64; 3], nbt: Option<minecraftoss_core::nbt::Tag>) {
        self.server.summon(u64::from(source.0), kind, position, nbt);
    }

    pub fn run_job(&mut self, job: minecraft_terrain::server::Job) {
        self.jobs_running += 1;
        self.server.run_job(job);
    }

    pub fn runs_jobs(&self) -> bool {
        self.jobs_running > 0
    }

    fn poll_server(&mut self) -> Vec<minecraft_terrain::server::Output> {
        let outputs = self.server.poll();
        for output in &outputs {
            self.finished_ticks.extend_from_slice(&output.tick_nanos);
        }
        outputs
    }

    pub fn ready_for_ticks(&mut self) -> bool {
        self.poll_jobs();
        self.server.ticks_in_flight() == 0
    }

    pub fn take_finished_ticks(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.finished_ticks)
    }

    pub fn poll_jobs(&mut self) {
        for mut output in self.poll_server() {
            self.jobs_running = self.jobs_running.saturating_sub(output.jobs.len());
            self.command_jobs.append(&mut output.jobs);
            self.unread.push(output);
        }
    }

    fn take_outputs(&mut self) -> Vec<minecraft_terrain::server::Output> {
        let mut outputs = std::mem::take(&mut self.unread);
        outputs.extend(self.poll_server());
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

    pub fn track_body(&mut self, key: u128, feet: [f64; 3], width: f64, height: f64) {
        self.bodies.insert(key, minecraft_terrain::ambient_particles::Body { feet: DVec3::from_array(feet), width, height });
    }

    pub fn totem_pop(&mut self, key: u128, feet: [f64; 3], width: f64, height: f64) {
        self.track_body(key, feet, width, height);
        let body = self.bodies[&key];
        self.ambient.totem_emitter(key, body);
    }

    pub fn lectern(&mut self, player: u64, pos: (i32, i32, i32), action: minecraft_terrain::lectern::LecternAction, sent: Option<SentItemUse>) {
        self.server.lectern(player, pos, action);
        if sent.is_some() {
            self.players.queue(player).item_use_waiting = sent;
        }
    }

    pub fn book_stack(&self, book: &minecraft_terrain::lectern::BookStack) -> ItemStack {
        let mut stack = self.recipes.stack(&book.id, 1);
        stack.components = book.components.as_deref().and_then(|text| serde_json::from_str(text).ok());
        stack
    }

    pub fn chest_openers(&mut self, pos: (i32, i32, i32), count: u32) {
        self.server.chest_openers(pos, count);
    }

    pub fn campfire_douse(&mut self, pos: (i32, i32, i32), signal: bool) {
        self.ambient.douse(pos, signal);
    }

    pub fn rain_tick(&mut self, scene: &HandcraftedScene, camera: [f64; 3]) -> Vec<minecraft_terrain::display_ticks::AmbientSound> {
        self.ambient.rain_tick(scene, DVec3::from_array(camera), self.weather.rain_level)
    }

    pub fn partial_tick(&self) -> f32 {
        (self.clock / TICK_SECONDS).clamp(0.0, 1.0) as f32
    }

    pub fn weather_mesh(&self, scene: &HandcraftedScene, camera: [f64; 3], light: &minecraft_terrain::lighting::SkyLight) -> (Vec<minecraft_terrain::weather::WeatherVertex>, Vec<u32>) {
        let rain_level = self.weather.rain_level;
        if rain_level <= 0.0 {
            return Default::default();
        }
        let camera = DVec3::from_array(camera);
        let radius = minecraft_terrain::rain::WEATHER_RADIUS;
        let columns = minecraft_terrain::weather::columns(
            camera,
            radius,
            self.ticks as i64,
            self.partial_tick(),
            |x, z| minecraft_terrain::rain::motion_blocking_height(scene, x, z),
            |pos| self.ambient.precipitation_at(scene, pos),
            |pos| (light.get(pos), light.get_block(pos)),
        );
        minecraft_terrain::weather::mesh(&columns, camera, radius, rain_level)
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

    pub fn anger_piglins(&mut self, player: u64, feet: [f64; 3], sight: bool) {
        self.server.anger_piglins(player, feet, sight);
    }

    pub fn guard_mined(&mut self, player: u64, states: &BlockStates, broken: &[BrokenBlock], feet: [f64; 3]) {
        if mining_angers_piglins(states, broken) {
            self.anger_piglins(player, feet, false);
        }
    }

    fn hand_enchantments(held: Option<&ItemStack>) -> Option<serde_json::Value> {
        held?.components.as_ref()?.get("minecraft:enchantments").cloned()
    }

    fn hand_has_silk_touch(held: Option<&ItemStack>) -> bool {
        has_silk_touch(Self::hand_enchantments(held).as_ref())
    }

    /// A bullet on a mob: an attack with the bullet's damage from where it
    /// was fired.
    pub fn shoot(&mut self, player: u64, key: u64, damage: f32, from: [f64; 3], yaw: f32, creative: bool) {
        if let Some(id) = frame_of(key) {
            self.server.frame_action(player, id, FrameAction::Shot { infinite: creative });
            return;
        }
        if let Some(target) = end_target_of(key) {
            self.server.end_hit(target, EndDamage::attack(damage * HEALTH_SCALE, player).from_position(DVec3::from_array(from)));
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
                self.server.deflect_fireball(id, aim.to_array(), player, true);
            }
            return;
        }
        let Some(hit) = decode(key) else {
            return;
        };
        let attack = minecraftoss_entities::world::PlayerAttack {
            player_id: player,
            position: DVec3::from_array(from),
            yaw,
            attack_damage: f64::from(damage * HEALTH_SCALE),
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        };
        self.server
            .mob_action(player, hit, Some(attack), &minecraftoss_player::inventory::Inventory::default(), 0, false);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn blast(&mut self, key: u64, damage: f32, center: [f64; 3], impact: f32, by_player: Option<u64>, slow_ticks: i32, creative: bool) {
        if let Some(id) = frame_of(key) {
            self.server.frame_action(by_player.unwrap_or(u64::MAX), id, FrameAction::Blast { infinite: by_player.is_some() && creative });
            return;
        }
        if let Some(target) = end_target_of(key) {
            self.server.end_hit(target, EndDamage::explosion(damage * BLAST_HEALTH_SCALE, by_player));
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
        self.server.mob_blast(hit, damage * BLAST_HEALTH_SCALE, center, impact, by_player, slow_ticks);
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
        actors: &mut [TickActor<'_>],
        dt: f64,
        batch: &crate::tick_rate::TickBatch,
        client: crate::tick_rate::ClientTiming,
        clocks: minecraft_terrain::server::ClockTicks,
        bright_outside: bool,
    ) -> Changes {
        self.tick_seconds = client.tick_seconds;
        if client.runs_normally {
            self.clock += dt;
        }
        let presenter = actors.first().map(|actor| actor.id);
        self.presenter = presenter;
        let presenter_feet = actors.first().map_or([0.0; 3], |actor| actor.view.feet);
        for actor in actors.iter() {
            self.players.queue(actor.id).feet = Some(actor.view.feet);
        }
        let orb_pickup_target = DVec3::from_array(presenter_feet) + DVec3::Y * f64::from(EYE_HEIGHT / 2.0);
        let ticked = client.runs_normally && self.clock >= self.tick_seconds;
        if ticked {
            self.clock = (self.clock - self.tick_seconds).min(self.tick_seconds);
            self.ticks += 1;
            self.falling.tick_sent();
            for tnt in self.falling.primed.iter().filter(|tnt| tnt.fuse > 0) {
                let smoke = DVec3::from_array(tnt.position) + DVec3::Y * 0.5;
                self.ambient.add(minecraft_terrain::ambient_particles::Kind::Smoke, smoke, DVec3::ZERO);
            }
            self.lids.tick();
            let near: Vec<DVec3> = actors.iter().filter(|actor| actor.view.alive && !actor.view.spectator).map(|actor| DVec3::from_array(actor.view.feet)).collect();
            self.spawners.tick(&near, &mut self.ambient);
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
                if cloud.breath {
                    self.ambient.breath_cloud(cloud.position, cloud.radius, cloud.waiting);
                } else {
                    self.spell.cloud(cloud.position, cloud.radius, cloud.color, cloud.waiting);
                }
            }
            self.spell.tick();
            minecraft_terrain::orb_render::tick_pickups(&mut self.orb_pickups, orb_pickup_target);
            if self.ticks % 600 == 0 {
                diag::info!(World, "Minecraft mobs: {} tracked", self.boxes().len());
            }
        }
        if !batch.is_empty() {
            let centre = |feet: [f64; 3]| ((feet[0].floor() as i32) >> 4, (feet[2].floor() as i32) >> 4);
            self.server.tick(TickInput {
                clocks,
                players: actors
                    .iter()
                    .map(|actor| TickPlayer {
                        id: actor.id,
                        feet: actor.view.feet,
                        alive: actor.view.alive,
                        main_hand: actor.inventory.slots.get(actor.selected).and_then(Option::as_ref).filter(|_| actor.view.picks_up).map(|s| s.id.clone()),
                        pickup: actor.view.picks_up.then(|| (Box::new(actor.inventory.clone()), actor.selected)),
                    })
                    .collect(),
                difficulty: self.difficulty,
                simulation_centres: actors.iter().map(|actor| centre(actor.view.feet)).collect(),
                simulation_distance: 8,
                mob_players: actors.iter().map(|actor| player_candidate(actor.id, &actor.view)).collect(),
                mob_views: actors
                    .iter()
                    .map(|actor| {
                        (
                            actor.id,
                            minecraftoss_entities::enderman::PlayerView {
                                head_yaw: actor.view.yaw,
                                pitch: actor.view.pitch,
                                disguised: false,
                            },
                        )
                    })
                    .collect(),
                mob_vitals: actors
                    .iter()
                    .map(|actor| {
                        (
                            actor.id,
                            minecraftoss_entities::monster_ai::PlayerVitals {
                                health: actor.view.health,
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
                bright_outside,
                tracking: (presenter_feet, 160.0),
                player_hurts: Vec::new(),
                spawn_mobs: self.spawn_mobs,
                controls: actors.iter().filter(|actor| actor.view.alive).map(|actor| player_controls(actor.id, &actor.view, actor.inventory)).collect(),
            }, batch.normal.clone());
        }
        let mut changes = Vec::new();
        for mut output in self.take_outputs() {
            self.jobs_running = self.jobs_running.saturating_sub(output.jobs.len());
            self.server_handled = output.handled;
            for &(_, id, position, value) in output.orbs_taken.iter().filter(|orb| Some(orb.0) == presenter) {
                let tick_count = self.orbs.iter().find(|orb| orb.id == id).map_or(0, |orb| orb.tick_count);
                let orb = ServerOrb { id, position, previous_position: position, value, tick_count };
                self.orb_pickups.push(minecraft_terrain::orb_render::OrbPickup::new(orb, orb_pickup_target));
            }
            if let Some(mut snapshot) = output.entities.take() {
                self.thrown = std::mem::take(&mut snapshot.thrown_views);
                self.eyes.receive(std::mem::take(&mut snapshot.eyes), &mut self.portal, &mut self.item_crumbs);
                self.clouds = std::mem::take(&mut snapshot.clouds);
                self.tridents = std::mem::take(&mut snapshot.tridents);
                self.crystals = std::mem::take(&mut snapshot.crystals);
                self.gateways.receive(std::mem::take(&mut snapshot.gateways), self.ticks);
                self.dragons = std::mem::take(&mut snapshot.dragons);
                self.dragon_fireballs = std::mem::take(&mut snapshot.dragon_fireballs);
                for fireball in &self.dragon_fireballs {
                    self.ambient.breath_trail(fireball.position);
                }
                self.frames = std::mem::take(&mut snapshot.frames);
                for at in std::mem::take(&mut snapshot.breath_bursts) {
                    self.ambient.breath_burst(at);
                }
                for breath in std::mem::take(&mut snapshot.dragon_breath) {
                    self.ambient.mouth_breath(&breath);
                }
                for (at, emitter) in std::mem::take(&mut snapshot.dragon_explosions) {
                    let kind = if emitter { minecraft_terrain::ambient_particles::Kind::ExplosionEmitter } else { minecraft_terrain::ambient_particles::Kind::Explosion };
                    self.ambient.add(kind, at, DVec3::ZERO);
                }
                self.dragon_voices();
                self.orbs = snapshot.orbs.clone();
                self.falling.receive(std::mem::take(&mut snapshot.falling), std::mem::take(&mut snapshot.tnt), &output.changes, self.ticks);
                self.server_snapshot = Some(snapshot);
            }
            let difficulty = self.difficulty;
            let mobs = output.mobs.take();
            let unclaimed = {
                let world = mobs.as_deref().unwrap_or(&self.world);
                let dragons = &self.dragons;
                let entity_type = |id: u64| world.entity_type(id).or_else(|| dragons.iter().any(|dragon| dragon.id == id).then_some(minecraft_terrain::ender_dragon::ENDER_DRAGON));
                self.players.route(&mut output, difficulty, entity_type)
            };
            for (_, position, item, count, components) in unclaimed.picked {
                self.server.spawn_item(&item, count, components.as_deref(), position, [0.0; 3], 0, 0);
            }
            for (position, value) in unclaimed.orbs {
                self.server.award_experience(position, value);
            }
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
            for answer in &output.eye_answers {
                if answer.flew() {
                    diag::info!(World, "{}", answer.line());
                } else {
                    diag::warn!(World, "{}", answer.line());
                }
            }
            for (source, summoned) in output.command_summons {
                self.command_summons.push((minecraft_commands::PlayerId(source as u32), summoned));
            }
            self.dragons_killed.extend(output.dragons_killed);
            self.fight_reports.extend(output.fight_reports);
            self.command_jobs.extend(output.jobs);
            changes.extend(output.changes);
            self.spawners.block_entities(&output.block_entities);
            self.block_entity_changes.extend(output.block_entities);
            self.destroyed.extend(output.destroyed);
            let camera = DVec3::from_array(presenter_feet) + DVec3::Y * f64::from(EYE_HEIGHT);
            for sound in &output.mob_sounds {
                let heard = minecraft_terrain::ender_dragon::heard_at(&sound.event, sound.position, camera);
                self.sounds.push((sound.event.clone(), heard, sound.volume, sound.pitch));
            }
            for sound in output.mob_results.iter().flat_map(|(_, result)| result.sounds.iter()) {
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
            if let Some(mobs) = mobs {
                self.world = *mobs;
                for (feet, width, height) in self.client.receive(server_mobs(&self.world)) {
                    self.poof.spawn(feet, width, height);
                }
            }
        }
        for actor in actors.iter_mut() {
            let Some(queue) = self.players.0.get_mut(&actor.id) else {
                continue;
            };
            for summoned in std::mem::take(&mut queue.egg_answers) {
                let egg = queue.eggs_waiting.answer(actor.inventory, summoned.is_ok());
                if let (Some(egg), Err(error)) = (egg, summoned) {
                    diag::warn!(World, "Minecraft {egg} hatched nothing: {error}");
                }
            }
            for slots in std::mem::take(&mut queue.mob_slots) {
                for (slot, stack) in slots {
                    if let Some(held) = actor.inventory.slots.get_mut(slot) {
                        *held = stack;
                    }
                }
            }
            let broken = std::mem::take(&mut queue.broken);
            if presenter == Some(actor.id) {
                self.broken_items.extend(broken);
            }
        }
        if ticked {
            self.server_items_tick(actors);
            // `ItemEntity.playerTouch`'s pickup pop.
            for _ in 0..self.world_items.take_pickup_sounds() {
                let pitch = ((self.random.next_float() - self.random.next_float()) * 0.7 + 1.0) * 2.0;
                self.sounds.push(("minecraft:entity.item.pickup".to_owned(), DVec3::from_array(presenter_feet), 0.2, pitch));
            }
        }
        changes
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

    pub fn gateway(&self, pos: (i32, i32, i32)) -> Option<&minecraft_terrain::end_gateway::GatewayView> {
        self.gateways.get(pos)
    }

    pub fn gateway_views(&self) -> &[minecraft_terrain::end_gateway::GatewayView] {
        self.gateways.views()
    }

    pub fn mark_cooling(&mut self, pos: (i32, i32, i32)) {
        self.gateways.mark_cooling(pos, self.ticks);
    }

    pub fn gateway_cooling(&self, pos: (i32, i32, i32)) -> bool {
        self.gateways.is_cooling(pos, self.ticks)
    }

    pub fn eye_views(&self) -> &[minecraft_terrain::eye_of_ender::EyeView] {
        self.eyes.views()
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
        for e in w.striders().iter().filter(|e| e.strider.health > 0.0) {
            add(MobHit::Strider(e.id), &e.strider.body);
        }
        for e in w.hoglins().iter().filter(|e| e.hoglin.health > 0.0) {
            add(MobHit::Hoglin(e.id), &e.hoglin.body);
        }
        for e in w.piglins().iter().filter(|e| e.piglin.health > 0.0) {
            add(MobHit::Piglin(e.id), &e.piglin.body);
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

    pub fn monster_near(&self, area: [f64; 6], sleeper: u64) -> bool {
        crate::sleep::monster_in(area, resting_monsters(&self.world, sleeper))
    }

    pub fn shot_boxes(&self, riding: Option<u64>) -> Vec<(u64, [f64; 6])> {
        let mut out = self.boxes();
        out.retain(|(key, _)| riding.is_none() || decode(*key).map(|hit| hit.id()) != riding);
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
        for frame in &self.frames {
            let (low, high) = minecraft_terrain::item_frame::hanging_box(frame.position, frame.facing);
            out.push((FRAME_KEY | (frame.id & ((1 << 56) - 1)), [low.x, low.y, low.z, high.x, high.y, high.z]));
        }
        out
    }

    /// Steps the client-side puffs, which settle on the scene's blocks.
    pub fn tick_scene(&mut self, scene: &HandcraftedScene, ticks: u32) {
        for _ in 0..ticks {
            self.poof.tick(scene);
            let bodies = &self.bodies;
            self.ambient.tick_emitters(|key| bodies.get(&key).copied());
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
        let partial = (self.clock / self.tick_seconds).clamp(0.0, 1.0) as f32;
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
        end_gateway_render::append_beams(&mut out.models, &mut out.translucent, self.gateways.views(), atlas, camera, scene.vertical_range().end - 1, w.game_time(), partial);
        blaze_render::append_blazes(&mut out.models, w.blazes().iter(), poses, atlas, light, partial);
        self.spawners.append(&mut out.models, &mut out.culled, &mut out.translucent, scene, atlas, light, camera, partial);
        strider_render::append_striders(&mut out.models, w.striders().iter(), poses, atlas, light, partial);
        hoglin_render::append_hoglins(&mut out.models, w.hoglins().iter(), poses, atlas, light, partial);
        let piglin_items = piglin_render::append_piglins(&mut out.models, w.piglins().iter(), poses, atlas, light, partial);
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
        let frames: Vec<_> = self.frames.iter().filter(|frame| minecraft_terrain::item_frame::tracked(frame, camera)).cloned().collect();
        let _ = self.items.append_frames(&mut out.items, &frames, packs, atlas, light);
        let _ = self.items.append_primed_tnt(&mut out.items, &self.falling.primed_tnt(partial), packs, atlas, light);
        minecraft_terrain::thrown::append_views(&mut out.items, &self.thrown, partial, camera, atlas, forward, light);
        minecraft_terrain::eye_of_ender::append_eyes(&mut out.items, self.eyes.views(), partial, camera, atlas, forward);
        minecraft_terrain::trident::append_tridents(&mut out.items, &self.tridents, partial, atlas, light);
        self.poof.append_mesh(&mut out.items, atlas, forward, partial, light);
        self.ambient.append_mesh(&mut out.items, &mut out.translucent, atlas, forward, partial, light);
        self.portal.append_mesh(&mut out.items, atlas, forward, partial, light);
        self.spell.append_mesh(&mut out.translucent, atlas, forward, partial, light);
        let held: Vec<_> =
            skeleton_items.into_iter().chain(zombie_items).chain(villager_items).chain(witch_items).chain(piglin_items).chain(placed_items.iter().cloned()).collect();
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

pub(crate) fn villager_asleep_in(world: &EntityWorld, (x, y, z): (i32, i32, i32)) -> bool {
    let block = [f64::from(x), f64::from(y), f64::from(z), f64::from(x) + 1.0, f64::from(y) + 1.0, f64::from(z) + 1.0];
    world.villagers().iter().filter(|e| e.sleeping.is_some()).any(|e| {
        let body = &e.villager.body;
        let half = f64::from(body.width) * 0.5;
        let p = body.position;
        crate::sleep::intersects([p.x - half, p.y, p.z - half, p.x + half, p.y + f64::from(body.height), p.z + half], block)
    })
}

fn resting_monsters(w: &EntityWorld, sleeper: u64) -> Vec<(&'static str, bool, [f64; 6])> {
    let bounds = |body: &minecraftoss_entities::movement::Body| {
        let p = body.position;
        let half = f64::from(body.width) * 0.5;
        [p.x - half, p.y, p.z - half, p.x + half, p.y + f64::from(body.height), p.z + half]
    };
    let mut out = Vec::new();
    for e in w.zombies().iter().filter(|e| e.zombie.health > 0.0) {
        let angry = e.ai.as_ref().is_some_and(|ai| ai.state.enderman.anger_target == Some(minecraftoss_entities::monster_ai::Target::Player(sleeper)));
        out.push((e.zombie.kind.type_id(), angry, bounds(&e.zombie.body)));
    }
    for e in w.skeletons().iter().filter(|e| e.skeleton.health > 0.0) {
        out.push((e.skeleton.kind.type_id(), false, bounds(&e.skeleton.body)));
    }
    for e in w.creepers().iter().filter(|e| e.creeper.health > 0.0 && !e.creeper.exploded) {
        out.push(("minecraft:creeper", false, bounds(&e.creeper.body)));
    }
    for e in w.spiders().iter().filter(|e| e.spider.health > 0.0) {
        out.push(("minecraft:spider", false, bounds(&e.spider.body)));
    }
    for e in w.endermen().iter().filter(|e| e.enderman.health > 0.0) {
        out.push(("minecraft:enderman", false, bounds(&e.enderman.body)));
    }
    for e in w.witches().iter().filter(|e| e.witch.health > 0.0) {
        out.push(("minecraft:witch", false, bounds(&e.witch.body)));
    }
    for e in w.blazes().iter().filter(|e| e.blaze.health > 0.0) {
        out.push(("minecraft:blaze", false, bounds(&e.blaze.body)));
    }
    for e in w.slimes().iter().filter(|e| e.slime.health > 0.0) {
        out.push((e.slime.type_id(), false, bounds(&e.slime.body)));
    }
    for e in w.hoglins().iter().filter(|e| e.hoglin.health > 0.0) {
        out.push((e.hoglin.kind.type_id(), false, bounds(&e.hoglin.body)));
    }
    for e in w.piglins().iter().filter(|e| e.piglin.health > 0.0) {
        out.push((e.piglin.kind.type_id(), false, bounds(&e.piglin.body)));
    }
    out
}

fn mob_on_ray(world: &EntityWorld, player: u64, eye: DVec3, look: DVec3, reach: f64) -> Option<(MobHit, f64)> {
    world.mob_on_ray_except(eye, look, reach, world.vehicle_of(minecraftoss_entities::world::Rider::Player(player)))
}

fn mob_to_use(world: &EntityWorld, player: u64, eye: DVec3, look: DVec3, reach: f64, block: Option<f64>, others: Option<f64>) -> Option<MobHit> {
    let (hit, distance) = mob_on_ray(world, player, eye, look, reach)?;
    (block.is_none_or(|block| distance < block) && others.is_none_or(|other| distance <= other)).then_some(hit)
}

fn struck_hit(feet: [f64; 3], struck: &minecraft_terrain::thrown::PlayerStruck) -> Hit {
    let from = DVec3::from_array(feet) - struck.velocity.normalize_or_zero();
    (struck.damage, Some(from.to_array()), Some([-struck.velocity.x, -struck.velocity.z]), crate::death::DamageCause::Generic, Default::default(), None)
}

fn melee_push(kind: &PlayerHitKind) -> crate::impulse::MeleePush {
    match *kind {
        PlayerHitKind::Melee { throw, lift, disable_shield_seconds, .. } => crate::impulse::MeleePush { throw, lift: f64::from(lift), disable_shield_ticks: crate::offhand::disable_ticks(disable_shield_seconds) },
        _ => Default::default(),
    }
}

fn mob_hit(
    hit: &minecraftoss_entities::world::PlayerHit,
    feet: [f64; 3],
    difficulty: i32,
    type_id: Option<&'static str>,
) -> Option<Hit> {
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
    (damage > 0.0).then_some((damage, from, Some(toward), cause, melee_push(&hit.kind), effect))
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

pub fn guarded_by_piglins(states: &BlockStates, block: &Block) -> bool {
    let registries = states.registries();
    match (states.state_of(block), registries.block_tags.id(GUARDED_BY_PIGLINS)) {
        (Some(state), Some(tag)) => registries.block_in_tag(state, tag),
        _ => false,
    }
}

fn mining_angers_piglins(states: &BlockStates, broken: &[BrokenBlock]) -> bool {
    broken.iter().any(|(_, block, blast)| blast.is_none() && guarded_by_piglins(states, block))
}

fn has_silk_touch(enchantments: Option<&serde_json::Value>) -> bool {
    enchantments.and_then(|enchantments| enchantments.get(SILK_TOUCH)).and_then(serde_json::Value::as_i64).is_some_and(|level| level > 0)
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
        MobHit::Strider(_) => STRIDER_KEY,
        MobHit::Hoglin(_) => HOGLIN_KEY,
        MobHit::Piglin(_) => PIGLIN_KEY,
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
        STRIDER_KEY => MobHit::Strider(id),
        HOGLIN_KEY => MobHit::Hoglin(id),
        PIGLIN_KEY => MobHit::Piglin(id),
        26 => MobHit::Shulker(id),
        _ => return None,
    })
}

const FIREBALL_KEY: u64 = 17 << 56;
const SHULKER_BULLET_KEY: u64 = 27 << 56;
const CRYSTAL_KEY: u64 = 19 << 56;
const DRAGON_KEY: u64 = 20 << 56;
const STRIDER_KEY: u64 = 23;
const HOGLIN_KEY: u64 = 22;
const PIGLIN_KEY: u64 = 21;
const FRAME_KEY: u64 = 28 << 56;
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

fn take_late(outputs: &mut [minecraft_terrain::server::Output], player: u64) -> (Vec<Picked>, u32) {
    let mut picked = Vec::new();
    let mut experience = 0u32;
    for output in outputs {
        let (mine, others): (Vec<_>, Vec<_>) = std::mem::take(&mut output.picked).into_iter().partition(|(id, _)| *id == player);
        output.picked = others;
        picked.extend(mine.into_iter().map(|(_, stack)| stack));
        for orb in output.orbs_taken.iter().filter(|orb| orb.0 == player) {
            experience = experience.saturating_add(orb.3.max(0) as u32);
        }
        output.orbs_taken.retain(|orb| orb.0 != player);
        output.pearl_teleports.retain(|(id, _)| *id != player);
        output.player_potions.retain(|hit| hit.player != player);
        output.players_pushed.retain(|(id, _)| *id != player);
    }
    (picked, experience)
}

fn frame_of(key: u64) -> Option<u64> {
    (key >> 56 == FRAME_KEY >> 56).then_some(key & ((1 << 56) - 1))
}

fn frame_target(frames: &[minecraft_terrain::item_frame::FrameView], eye: DVec3, look: DVec3, reach: f64, block: Option<f64>, others: Option<f64>) -> Option<u64> {
    let (id, distance) = minecraft_terrain::item_frame::on_ray(frames, eye, look, reach)?;
    (block.is_none_or(|block| distance < block) && others.is_none_or(|other| distance < other)).then_some(id)
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

    const HOST: u64 = 0;
    const FRIEND: u64 = 5;

    #[test]
    fn a_leaving_player_gets_the_late_picks_and_loses_their_pearls_and_pushes() {
        let mut outputs = vec![minecraft_terrain::server::Output::default(), minecraft_terrain::server::Output::default()];
        outputs[0].picked.push((HOST, (7, [1.0, 64.0, 2.0], "minecraft:dirt".to_owned(), 3, None)));
        outputs[0].picked.push((5, (9, [1.0, 64.0, 2.0], "minecraft:sand".to_owned(), 2, None)));
        outputs[0].pearl_teleports = vec![(HOST, [0.0; 3]), (5, [1.0; 3])];
        outputs[0].orbs_taken = vec![(HOST, 3, [1.0, 64.0, 2.0], 7), (5, 4, [1.0, 64.0, 2.0], 2)];
        outputs[1].picked.push((HOST, (8, [1.0, 64.0, 2.0], "minecraft:cobblestone".to_owned(), 1, None)));
        outputs[1].players_pushed = vec![(HOST, DVec3::X), (5, DVec3::Y)];
        let (late, experience) = take_late(&mut outputs, HOST);
        assert_eq!(experience, 7, "the orbs taken before the leave still count");
        assert_eq!(outputs[0].orbs_taken.iter().map(|orb| orb.0).collect::<Vec<_>>(), vec![5]);
        assert_eq!(late.iter().map(|picked| picked.0).collect::<Vec<_>>(), vec![7, 8]);
        assert_eq!(outputs[0].picked.iter().map(|(player, picked)| (*player, picked.0)).collect::<Vec<_>>(), vec![(5, 9)], "another player's pick stays");
        assert!(outputs[1].picked.is_empty());
        assert_eq!(outputs[0].pearl_teleports, vec![(5, [1.0; 3])]);
        assert_eq!(outputs[1].players_pushed, vec![(5, DVec3::Y)]);
        let mut inventory = Inventory::default();
        assert!(take_picked(&mut inventory, 0, late).is_empty());
        let held: Vec<_> = inventory.slots.iter().flatten().map(|stack| (stack.id.as_str(), stack.count)).collect();
        assert_eq!(held, vec![("minecraft:dirt", 3), ("minecraft:cobblestone", 1)]);
    }

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
            main_hand: None,
            offhand: None,
            sneaking: false,
        };
        let survival = player_candidate(HOST, &view(true, false));
        assert!(survival.alive && survival.attackable && !survival.spectator);
        assert_eq!(survival.position, DVec3::new(1.0, 64.0, 2.0));
        let creative = player_candidate(HOST, &view(false, false));
        assert!(creative.alive && !creative.attackable && !creative.spectator);
        let spectator = player_candidate(FRIEND, &view(false, true));
        assert!(spectator.alive && !spectator.attackable && spectator.spectator);
        assert_eq!((survival.id, spectator.id), (HOST, FRIEND), "each candidate is the player it stands for");
    }

    fn pig_ahead(distance: f64) -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        let pig = world.spawn_pig_no_ai(minecraftoss_entities::pig::Pig::new(DVec3::new(0.5, 64.0, 0.5 + distance)));
        (world, pig)
    }

    const EYE: DVec3 = DVec3::new(0.5, 64.5, 0.5);

    #[test]
    fn the_bed_check_sees_zombies_zoglins_and_piglins_and_only_angry_zombified_piglins() {
        use minecraftoss_entities::zombie::{Zombie, ZombieKind};
        let area = crate::sleep::monster_box((0, 64, 0));
        let mut world = EntityWorld::default();
        world.spawn_slime(minecraftoss_entities::slime::Slime::new(DVec3::new(2.5, 64.0, 0.5), 2), true);
        world.spawn_hoglin(minecraftoss_entities::hoglin::Hoglin::new(minecraftoss_entities::hoglin::HoglinKind::Hoglin, DVec3::new(-2.5, 64.0, 0.5)), true);
        world.spawn_strider(minecraftoss_entities::strider::Strider::new(DVec3::new(0.5, 64.0, 3.5)), true);
        let mut piglin = Zombie::new(DVec3::new(0.5, 64.0, -3.5));
        piglin.kind = ZombieKind::ZombifiedPiglin;
        let calm = world.spawn_zombie_active(piglin, 0.0);
        assert!(!monsters_in(&world, area, 7));
        world.zombie_mut(calm).unwrap().ai.as_mut().unwrap().state.enderman.anger_target = Some(minecraftoss_entities::monster_ai::Target::Player(3));
        assert!(!monsters_in(&world, area, 7), "angry at someone else");
        assert!(monsters_in(&world, area, 3));
        let mut world = EntityWorld::default();
        world.spawn_zombie(Zombie::new(DVec3::new(8.7, 64.0, 0.5)), true);
        assert!(monsters_in(&world, area, 7));
        let mut world = EntityWorld::default();
        world.spawn_zombie(Zombie::new(DVec3::new(8.9, 64.0, 0.5)), true);
        assert!(!monsters_in(&world, area, 7));
        let mut world = EntityWorld::default();
        world.spawn_hoglin(minecraftoss_entities::hoglin::Hoglin::new(minecraftoss_entities::hoglin::HoglinKind::Zoglin, DVec3::new(-2.5, 64.0, 0.5)), true);
        assert!(monsters_in(&world, area, 7));
        let mut world = EntityWorld::default();
        world.spawn_piglin(minecraftoss_entities::piglin::Piglin::new(minecraftoss_entities::piglin::PiglinKind::Brute, DVec3::new(-2.5, 60.0, 0.5)), true);
        assert!(monsters_in(&world, area, 7));
    }

    fn monsters_in(world: &EntityWorld, area: [f64; 6], sleeper: u64) -> bool {
        crate::sleep::monster_in(area, resting_monsters(world, sleeper))
    }

    #[test]
    fn a_mob_behind_a_block_is_not_used() {
        let (world, pig) = pig_ahead(2.0);
        assert_eq!(mob_to_use(&world, HOST, EYE, DVec3::Z, 3.0, Some(1.0), None), None);
        assert_eq!(mob_to_use(&world, HOST, EYE, DVec3::Z, 3.0, Some(2.5), None), Some(MobHit::Pig(pig)));
        assert_eq!(mob_to_use(&world, HOST, EYE, DVec3::Z, 3.0, None, None), Some(MobHit::Pig(pig)));
        assert_eq!(mob_to_use(&world, HOST, EYE, DVec3::Z, 3.0, None, Some(1.0)), None, "a fireball or crystal in front takes the click");
        assert_eq!(mob_to_use(&world, HOST, EYE, -DVec3::Z, 3.0, None, None), None);
    }

    #[test]
    fn the_players_own_strider_is_never_picked_but_a_mob_behind_it_is() {
        use minecraftoss_entities::world::Rider;
        let (mut world, pig) = pig_ahead(2.0);
        let strider = world.spawn_strider(minecraftoss_entities::strider::Strider::new(DVec3::new(0.5, 64.0, 1.0)), true);
        let eye = DVec3::new(0.5, 64.5, -0.5);
        assert_eq!(mob_to_use(&world, HOST, eye, DVec3::Z, 5.0, None, None), Some(MobHit::Strider(strider)));
        assert!(world.force_riding(Rider::Player(HOST + 1), strider));
        assert_eq!(mob_on_ray(&world, HOST, eye, DVec3::Z, 5.0).map(|(hit, _)| hit), Some(MobHit::Strider(strider)), "someone else's strider");
        world.drop_player(HOST + 1);
        assert!(world.force_riding(Rider::Player(HOST), strider));
        assert_eq!(mob_to_use(&world, HOST, eye, DVec3::Z, 5.0, None, None), Some(MobHit::Pig(pig)));
        assert_eq!(mob_on_ray(&world, HOST, eye, DVec3::Z, 5.0).map(|(hit, _)| hit), Some(MobHit::Pig(pig)));
        let seat_eye = DVec3::new(0.5, 66.72, 1.0);
        assert_eq!(world.mob_on_ray(seat_eye, -DVec3::Y, 3.0).map(|(hit, _)| hit), Some(MobHit::Strider(strider)));
        assert_eq!(mob_on_ray(&world, HOST, seat_eye, -DVec3::Y, 3.0), None, "looking down at it");
    }

    #[test]
    fn creative_reaches_mobs_at_five_blocks_and_survival_at_three() {
        let survival = minecraft_terrain::game_mode::PlayerMode::new(minecraft_terrain::game_mode::GameMode::Survival).entity_reach();
        let creative = minecraft_terrain::game_mode::PlayerMode::new(minecraft_terrain::game_mode::GameMode::Creative).entity_reach();
        assert_eq!((survival, creative), (3.0, 5.0));
        let (world, pig) = pig_ahead(3.8);
        assert_eq!(mob_to_use(&world, HOST, EYE, DVec3::Z, survival, None, None), None);
        assert_eq!(mob_to_use(&world, HOST, EYE, DVec3::Z, creative, None, None), Some(MobHit::Pig(pig)));
        let (world, pig) = pig_ahead(2.9);
        assert_eq!(mob_to_use(&world, HOST, EYE, DVec3::Z, survival, None, None), Some(MobHit::Pig(pig)));
    }

    #[test]
    fn a_saddle_used_on_a_pig_leaves_the_inventory_and_saddles_it() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = std::sync::Arc::new(registries);
        let Ok(terrain) = crate::level::Terrain::open(registries, 0, 2, minecraft_net::Dimension::Overworld, None, None) else {
            return;
        };
        let data = crate::data::GameData::default();
        let mut entities = Entities::new(&terrain.stream, minecraft_net::Dimension::Overworld, 0, &data, None, None);
        let nbt = minecraftoss_core::snbt::parse_compound("{NoAI:1b}").unwrap();
        entities.summon(minecraft_commands::PlayerId(0), "minecraft:pig".to_owned(), [0.5, 200.0, 2.5], Some(nbt));
        let mut inventory = Inventory::default();
        inventory.slots[3] = Some(ItemStack::new("minecraft:saddle", 1));
        let view = PlayerView { feet: [0.5, 200.0, 0.5], alive: true, attackable: true, spectator: false, picks_up: false, health: 20.0, yaw: 0.0, pitch: 0.0, main_hand: None, offhand: None, sneaking: false };
        let started = std::time::Instant::now();
        let tick = |entities: &mut Entities, inventory: &mut Inventory| {
            assert!(started.elapsed().as_secs() < 60, "the server answers");
            std::thread::sleep(std::time::Duration::from_millis(5));
            let batch = crate::tick_rate::TickBatch { normal: if entities.ready_for_ticks() { vec![true] } else { Vec::new() }, overload: None };
            entities.tick(&mut [TickActor { id: HOST, view: view.clone(), inventory, selected: 3 }], TICK_SECONDS, &batch, Default::default(), Default::default(), true);
        };
        while entities.world.pigs().is_empty() {
            tick(&mut entities, &mut inventory);
        }
        let eye = entities.world.pigs()[0].pig.body.position + DVec3::new(0.0, 0.5, -2.0);
        assert!(!entities.use_on_mob(HOST, eye, DVec3::Z, 3.0, None, &inventory, 0, false), "an empty hand does nothing to a pig");
        assert!(entities.use_on_mob(HOST, eye, DVec3::Z, 3.0, None, &inventory, 3, false));
        while inventory.slots[3].is_some() {
            tick(&mut entities, &mut inventory);
        }
        while !entities.world.pigs()[0].pig.saddled {
            tick(&mut entities, &mut inventory);
        }
        assert!(!entities.use_on_mob(HOST, eye, DVec3::Z, 3.0, None, &inventory, 3, false), "a saddled pig takes no second saddle");
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
    fn piglin_keys_round_trip() {
        let key = encode(MobHit::Piglin(77));
        assert_eq!(key >> 56, 21);
        assert_eq!(decode(key), Some(MobHit::Piglin(77)));
        assert_eq!(fireball_of(key), None);
        assert_eq!(crystal_of(key), None);
        assert_eq!(end_target_of(key), None);
    }

    #[test]
    fn a_hoglin_hit_box_encodes_and_decodes() {
        let key = encode(MobHit::Hoglin(77));
        assert_eq!(key >> 56, 22);
        assert_eq!(decode(key), Some(MobHit::Hoglin(77)));
        assert_eq!(fireball_of(key), None);
        assert_eq!(crystal_of(key), None);
        assert_eq!(end_target_of(key), None);
    }

    #[test]
    fn a_hoglin_throw_reaches_the_player_hit_list() {
        let throw = DVec3::new(0.1, 0.4, -0.2);
        let melee = PlayerHitKind::Melee { attacker: DVec3::ZERO, hunger_ticks: 0, lift: 0.0, throw, disable_shield_seconds: 0.0 };
        assert_eq!(melee_push(&melee), crate::impulse::MeleePush { throw, lift: 0.0, disable_shield_ticks: 0 });
        let golem = PlayerHitKind::Melee { attacker: DVec3::ZERO, hunger_ticks: 0, lift: 0.4, throw: DVec3::ZERO, disable_shield_seconds: 0.0 };
        assert_eq!(melee_push(&golem).lift, f64::from(0.4_f32));
        let arrow = PlayerHitKind::Arrow { velocity: DVec3::X, effect: None };
        assert_eq!(melee_push(&arrow), crate::impulse::MeleePush::default());
    }

    #[test]
    fn a_brute_s_axe_hit_carries_a_hundred_ticks_of_shield_cooldown() {
        let axe = PlayerHitKind::Melee { attacker: DVec3::ZERO, hunger_ticks: 0, lift: 0.0, throw: DVec3::ZERO, disable_shield_seconds: 5.0 };
        assert_eq!(melee_push(&axe).disable_shield_ticks, 100);
        assert_eq!(melee_push(&axe).landing(false, true, 0.0), None);
    }

    #[test]
    fn the_worn_armour_reaches_the_mobs_feet_first() {
        let mut inventory = Inventory::default();
        inventory.slots[36] = Some(ItemStack::new("minecraft:golden_boots", 1));
        inventory.slots[39] = Some(ItemStack::new("minecraft:chainmail_helmet", 1));
        let view = PlayerView { feet: [0.0; 3], alive: true, attackable: true, spectator: false, picks_up: true, health: 20.0, yaw: 0.0, pitch: 0.0, main_hand: None, offhand: None, sneaking: false };
        let controls = player_controls(HOST, &view, &inventory);
        assert_eq!(controls.armor, [Some("minecraft:golden_boots".to_owned()), None, None, Some("minecraft:chainmail_helmet".to_owned())]);
    }

    #[test]
    fn a_strider_hit_box_encodes_and_decodes() {
        let key = encode(MobHit::Strider(77));
        assert_eq!(key >> 56, 23);
        assert_eq!(decode(key), Some(MobHit::Strider(77)));
        assert_eq!(fireball_of(key), None);
        assert_eq!(crystal_of(key), None);
        assert_eq!(end_target_of(key), None);
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
    fn the_frame_key_round_trips() {
        let key = FRAME_KEY | 42;
        assert_eq!(frame_of(key), Some(42));
        assert_eq!((decode(key), crystal_of(key), fireball_of(key), shulker_bullet_of(key), end_target_of(key)), (None, None, None, None, None));
        assert_eq!(frame_of(SHULKER_BULLET_KEY | 42), None);
        assert_eq!(frame_of(encode(MobHit::Shulker(42))), None);
    }

    #[test]
    fn a_frame_behind_a_wall_is_not_hit() {
        let mut frames = minecraft_terrain::item_frame::Frames::default();
        let tag = frames.fresh_tag(false, (0, 64, 3), minecraftoss_core::pos::Direction::North);
        let id = frames.add(&tag).unwrap();
        let views = frames.views();
        let eye = DVec3::new(0.5, 64.5, 0.5);
        assert_eq!(frame_target(&views, eye, DVec3::Z, 4.5, None, None), Some(id));
        assert_eq!(frame_target(&views, eye, DVec3::Z, 4.5, Some(2.0), None), None, "a block nearer than the frame");
        assert_eq!(frame_target(&views, eye, DVec3::Z, 4.5, Some(3.5), Some(1.0)), None, "a mob nearer than the frame");
        assert_eq!(frame_target(&views, eye, DVec3::Z, 2.0, None, None), None, "out of reach");
        let (low, high) = minecraft_terrain::item_frame::hanging_box(views[0].position, views[0].facing);
        assert!((high - low - DVec3::new(0.75, 0.75, 0.0625)).length() < 1e-12);
        assert!((low.z - 3.9375).abs() < 1e-12);
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
            player_id: HOST,
            damage: 4.0,
            kind: PlayerHitKind::ShulkerBullet { velocity: DVec3::new(0.15, 0.0, 0.0) },
            source: Some(9),
        };
        let damage = |difficulty| mob_hit(&hit, [0.0; 3], difficulty, Some("minecraft:shulker")).map(|h| h.0);
        assert_eq!((damage(0), damage(1), damage(2), damage(3)), (None, Some(3.0), Some(4.0), Some(6.0)));
        let (_, from, toward, cause, _, effect) = mob_hit(&hit, [0.0; 3], 2, Some("minecraft:shulker")).unwrap();
        assert_eq!((from, toward), (None, Some([-0.15, -0.0])));
        assert_eq!(cause, crate::death::DamageCause::Mob { id: 9, type_id: "minecraft:shulker", attack: crate::death::MobAttack::MobProjectile });
        assert_eq!(effect, Some((EffectKind::Levitation, 200)));
        let ownerless = minecraftoss_entities::world::PlayerHit { source: None, ..hit };
        let (damage, _, _, cause, _, effect) = mob_hit(&ownerless, [0.0; 3], 3, None).unwrap();
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
            player: HOST,
            by: 3,
            damage: minecraft_terrain::trident::DAMAGE,
            kind: minecraft_terrain::trident::DAMAGE_TYPE,
            velocity: DVec3::new(2.0, 0.0, 0.0),
        };
        let (damage, from, toward, cause, _, _) = struck_hit([10.0, 64.0, 5.0], &struck);
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
    fn silk_touch_is_read_from_the_held_enchantments() {
        let silk = serde_json::json!({ "minecraft:silk_touch": 1 });
        assert!(has_silk_touch(Some(&silk)));
        assert!(!has_silk_touch(Some(&serde_json::json!({ "minecraft:efficiency": 5 }))));
        assert!(!has_silk_touch(None));
        let mut pick = ItemStack::new("minecraft:diamond_pickaxe", 1);
        pick.components = Some(serde_json::json!({ "minecraft:enchantments": silk }));
        assert!(Entities::hand_has_silk_touch(Some(&pick)));
        assert!(!Entities::hand_has_silk_touch(None));
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

    fn two_queues() -> PlayerQueues {
        let mut queues = PlayerQueues::default();
        queues.queue(HOST).feet = Some([0.5, 64.0, 0.5]);
        queues.queue(FRIEND).feet = Some([40.5, 64.0, 0.5]);
        queues
    }

    fn sent_use(slot: usize) -> SentItemUse {
        SentItemUse {
            slot,
            held: "minecraft:bucket".to_owned(),
            eye: DVec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            outcome: minecraft_terrain::bucket::ItemUse {
                change: minecraft_terrain::bucket::Change::Breath,
                result: None,
                wear: 0,
                consumed: true,
                sounds: Vec::new(),
            },
        }
    }

    #[test]
    fn a_skeleton_arrow_hurts_the_friend_it_hit_and_not_the_host() {
        let mut queues = two_queues();
        let mut output = minecraft_terrain::server::Output::default();
        output.player_hits.push(minecraftoss_entities::world::PlayerHit {
            player_id: FRIEND,
            damage: 4.0,
            kind: PlayerHitKind::Arrow { velocity: DVec3::new(-2.0, 0.0, 0.0), effect: None },
            source: Some(77),
        });
        let skeleton = |id: u64| (id == 77).then_some("minecraft:skeleton");
        let unclaimed = queues.route(&mut output, 2, skeleton);
        assert!(unclaimed.picked.is_empty() && unclaimed.orbs.is_empty());
        assert!(queues.queue(HOST).out.hits.is_empty(), "the host was not hit");
        let hits = std::mem::take(&mut queues.queue(FRIEND).out.hits);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0], (4.0, None, Some([2.0, -0.0]), crate::death::DamageCause::mob(crate::death::MobAttack::Arrow, Some(77), Some("minecraft:skeleton")), Default::default(), None));
    }

    #[test]
    fn a_friends_picks_orbs_pearls_pushes_and_item_uses_reach_only_the_friend() {
        let mut queues = two_queues();
        queues.queue(HOST).item_use_waiting = Some(sent_use(1));
        queues.queue(FRIEND).item_use_waiting = Some(sent_use(2));
        let mut output = minecraft_terrain::server::Output::default();
        output.picked.push((FRIEND, (9, [40.5, 64.0, 0.5], "minecraft:beef".to_owned(), 2, None)));
        output.orbs_taken = vec![(FRIEND, 4, [40.5, 64.0, 0.5], 3), (FRIEND, 5, [40.5, 64.0, 0.5], 1)];
        output.pearl_teleports = vec![(FRIEND, [41.0, 65.0, 2.0])];
        output.players_pushed = vec![(FRIEND, DVec3::Y)];
        output.item_uses = vec![(FRIEND, true)];
        output.players_struck.push(minecraft_terrain::thrown::PlayerStruck {
            player: FRIEND,
            by: 3,
            damage: 8.0,
            kind: minecraft_terrain::trident::DAMAGE_TYPE,
            velocity: DVec3::new(0.0, 0.0, 1.0),
        });
        let unclaimed = queues.route(&mut output, 2, |_| None);
        assert!(unclaimed.picked.is_empty() && unclaimed.orbs.is_empty());
        let friend = queues.queue(FRIEND);
        assert_eq!(friend.picked.iter().map(|picked| picked.2.as_str()).collect::<Vec<_>>(), ["minecraft:beef"]);
        assert_eq!(friend.experience, 4);
        assert_eq!(friend.out.pearl_landings, [DVec3::new(41.0, 65.0, 2.0)]);
        assert_eq!(friend.out.pushes, [DVec3::Y]);
        assert_eq!(friend.out.item_uses_done.iter().map(|used| used.slot).collect::<Vec<_>>(), [2]);
        assert!(friend.item_use_waiting.is_none());
        assert_eq!(friend.out.hits.len(), 1);
        assert_eq!(friend.out.hits[0].1, Some([40.5, 64.0, -0.5]), "the trident came in at the friend's feet");
        let host = queues.queue(HOST);
        assert!(host.picked.is_empty() && host.experience == 0);
        assert!(host.out.pearl_landings.is_empty() && host.out.pushes.is_empty() && host.out.hits.is_empty());
        assert!(host.item_use_waiting.is_some(), "the host's own use still waits for its answer");
    }

    #[test]
    fn rides_boosts_lecterns_and_mob_uses_reach_only_their_player() {
        let mut queues = two_queues();
        queues.queue(FRIEND).boost_waiting = Some((2, "minecraft:warped_fungus_on_a_stick".to_owned()));
        queues.queue(HOST).boost_waiting = Some((4, "minecraft:warped_fungus_on_a_stick".to_owned()));
        let mut output = minecraft_terrain::server::Output::default();
        let saddled = minecraft_terrain::server::MobResult { slots: vec![(3, None)], broken: vec!["minecraft:shears".to_owned()], mounted: Some(41), ..Default::default() };
        output.mob_results = vec![(FRIEND, saddled)];
        output.dismounts = vec![(FRIEND, 40, Some([1.0, 65.0, 2.0])), (HOST, 39, None)];
        output.mounts = vec![(FRIEND, 42)];
        output.boosts = vec![(FRIEND, true), (HOST, false)];
        let view = |player: u64| minecraft_terrain::lectern::LecternView { player, pos: (1, 64, 1), book: None, page: 0, taken: None, closed: true };
        output.lecterns = vec![view(FRIEND)];
        let _ = queues.route(&mut output, 2, |_| None);
        let friend = queues.queue(FRIEND);
        assert_eq!(friend.mob_slots, [vec![(3, None)]]);
        assert_eq!(friend.broken, ["minecraft:shears"]);
        assert_eq!(friend.mounts, [41, 42]);
        assert_eq!(friend.dismounts, [(40, Some(DVec3::new(1.0, 65.0, 2.0)))]);
        assert_eq!(friend.boosts_done, [(2, "minecraft:warped_fungus_on_a_stick".to_owned())]);
        assert_eq!(friend.lecterns.iter().map(|view| view.player).collect::<Vec<_>>(), [FRIEND]);
        let host = queues.queue(HOST);
        assert!(host.mob_slots.is_empty() && host.mounts.is_empty() && host.lecterns.is_empty());
        assert_eq!(host.dismounts, [(39, None)]);
        assert!(host.boosts_done.is_empty() && host.boost_waiting.is_none(), "a refused boost is let go");
    }

    #[test]
    fn outputs_for_a_player_no_longer_in_the_level_go_back_to_the_world() {
        let mut queues = two_queues();
        queues.0.remove(&FRIEND);
        let mut output = minecraft_terrain::server::Output::default();
        output.picked.push((FRIEND, (9, [40.5, 64.0, 0.5], "minecraft:beef".to_owned(), 2, None)));
        output.picked.push((HOST, (10, [0.5, 64.0, 0.5], "minecraft:dirt".to_owned(), 1, None)));
        output.orbs_taken = vec![(FRIEND, 4, [40.5, 64.0, 0.5], 3)];
        output.pearl_teleports = vec![(FRIEND, [41.0, 65.0, 2.0])];
        output.players_pushed = vec![(FRIEND, DVec3::Y)];
        output.player_potions.push(minecraft_terrain::potions::PlayerPotionHit {
            player: FRIEND,
            potion: minecraft_terrain::potions::PlayerPotion::Extinguish,
            at: DVec3::ZERO,
        });
        let unclaimed = queues.route(&mut output, 2, |_| None);
        assert_eq!(unclaimed.picked.iter().map(|picked| (picked.0, picked.1, picked.2.as_str())).collect::<Vec<_>>(), [(9, [40.5, 64.0, 0.5], "minecraft:beef")], "a late pick is spawned again where it was picked up");
        assert_eq!(unclaimed.orbs, [([40.5, 64.0, 0.5], 3)]);
        assert!(!queues.0.contains_key(&FRIEND), "teleports, pushes and potions for them are dropped");
        assert_eq!(queues.queue(HOST).picked.len(), 1);
    }

    const SEED: i64 = 4986579828511054607;

    fn friend_world() -> Option<Entities> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        let mut stream = TerrainStream::for_dimension(std::sync::Arc::new(registries), SEED, 2, minecraft_net::Dimension::Overworld, None).ok()?;
        let mut entities = Entities::new(&stream, minecraft_net::Dimension::Overworld, SEED, &crate::data::GameData::default(), None, None);
        let mut scene = HandcraftedScene::streamed(stream.states.clone());
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = stream.load_chunk_now(minecraftoss_core::ChunkPos::new(x, z));
                entities.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        for x in 8..17 {
            for z in 8..17 {
                scene.set((x, 200, z), Some(Block::new("minecraft:stone")));
                entities.placed(&scene, (x, 200, z));
            }
        }
        Some(entities)
    }

    const HOST_FEET: [f64; 3] = [-12.5, 201.0, -12.5];
    const FRIEND_FEET: [f64; 3] = [12.5, 201.0, 12.5];

    fn tick_both(entities: &mut Entities, host: &mut Inventory, friend: &mut Inventory) {
        let view = |feet| PlayerView {
            feet,
            alive: true,
            attackable: true,
            spectator: false,
            picks_up: true,
            health: 20.0,
            yaw: 0.0,
            pitch: 0.0,
            main_hand: None,
            offhand: None,
            sneaking: false,
        };
        let batch = crate::tick_rate::TickBatch {
            normal: if entities.ready_for_ticks() { vec![true] } else { Vec::new() },
            overload: None,
        };
        let mut actors = [
            TickActor { id: HOST, view: view(HOST_FEET), inventory: host, selected: 0 },
            TickActor { id: FRIEND, view: view(FRIEND_FEET), inventory: friend, selected: 0 },
        ];
        let _ = entities.tick(&mut actors, 0.05, &batch, crate::tick_rate::ClientTiming::default(), minecraft_terrain::server::ClockTicks::default(), true);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    fn count(inventory: &Inventory, id: &str) -> u32 {
        inventory.slots.iter().flatten().filter(|stack| stack.id == id).map(|stack| u32::from(stack.count)).sum()
    }

    #[test]
    fn a_friends_kill_gives_the_friend_the_experience() {
        let Some(mut entities) = friend_world() else {
            return;
        };
        entities.spawn_mobs = false;
        let players = [(HOST, minecraft_net::PlayerUuid("00000000-0000-0000-0000-000000000001".to_owned())), (FRIEND, minecraft_net::PlayerUuid("00000000-0000-0000-0000-000000000005".to_owned()))];
        let sent = entities.server.sent();
        entities.set_players(&players);
        entities.set_players(&players);
        assert_eq!(entities.server.sent() - sent, 2, "each player's UUID goes to the level once");
        let (mut host, mut friend) = (Inventory::default(), Inventory::default());
        let still = minecraftoss_core::snbt::parse_compound("{NoAI:1b}").expect("the tag");
        entities.summon(minecraft_commands::PlayerId(0), "minecraft:cow".to_owned(), FRIEND_FEET, Some(still));
        let started = std::time::Instant::now();
        let cow = loop {
            assert!(started.elapsed().as_secs() < 300, "the cow never showed");
            let mine = |at: &[f64; 6]| ((at[0] + at[3]) / 2.0 - FRIEND_FEET[0]).abs() < 1.0 && ((at[2] + at[5]) / 2.0 - FRIEND_FEET[2]).abs() < 1.0;
            if let Some((key, _)) = entities.boxes().into_iter().find(|(key, at)| key >> 56 == 11 && mine(at)) {
                break key;
            }
            tick_both(&mut entities, &mut host, &mut friend);
        };
        entities.shoot(FRIEND, cow, 1000.0, [12.5, 202.0, 9.5], 0.0, false);
        let (mut host_experience, mut friend_experience) = (0, 0);
        while friend_experience == 0 {
            assert!(started.elapsed().as_secs() < 300, "the friend never got the cow's experience");
            tick_both(&mut entities, &mut host, &mut friend);
            host_experience += entities.take_experience(HOST);
            friend_experience += entities.take_experience(FRIEND);
        }
        for _ in 0..20 {
            tick_both(&mut entities, &mut host, &mut friend);
            host_experience += entities.take_experience(HOST);
            friend_experience += entities.take_experience(FRIEND);
        }
        assert!((1..=3).contains(&friend_experience), "a cow leaves 1 to 3 points: {friend_experience}");
        assert_eq!(host_experience, 0, "the host stood far away");
        assert!(!entities.boxes().iter().any(|(key, _)| *key == cow), "the cow died");
    }

    #[test]
    fn a_host_drop_reaches_the_level_and_the_friend_picks_it_up() {
        let Some(mut entities) = friend_world() else {
            return;
        };
        let (mut host, mut friend) = (Inventory::default(), Inventory::default());
        entities.world_items.spawn(ItemStack::new("minecraft:cobblestone", 3), DVec3::new(12.5, 201.5, 12.5));
        let started = std::time::Instant::now();
        while count(&friend, "minecraft:cobblestone") < 3 {
            assert!(started.elapsed().as_secs() < 300, "the friend never picked up the host's drop");
            tick_both(&mut entities, &mut host, &mut friend);
        }
        for _ in 0..20 {
            tick_both(&mut entities, &mut host, &mut friend);
        }
        assert_eq!(count(&friend, "minecraft:cobblestone"), 3);
        assert_eq!(count(&host, "minecraft:cobblestone"), 0, "only the friend stood on it");
        assert!(entities.server_handed.is_empty(), "the level took the drop over");
        assert!(entities.world_items.entities.iter().all(|item| item.stack.id != "minecraft:cobblestone"));
        let feet = Some(FRIEND_FEET);
        assert_eq!(entities.leave(FRIEND, &mut friend, 0, feet), 0);
        assert!(entities.player_outputs(FRIEND).is_none(), "the friend's queues are gone");
        assert!(entities.player_outputs(HOST).is_some(), "the host's stay");
    }

    #[test]
    fn only_guarded_blocks_send_anger() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        let Ok(states) = BlockStates::new(std::sync::Arc::new(registries), 0, -64, 384) else {
            return;
        };
        let guarded = [
            "gold_block", "raw_gold_block", "gilded_blackstone", "gold_ore", "deepslate_gold_ore", "nether_gold_ore",
            "chest", "trapped_chest", "copper_chest", "barrel", "ender_chest", "shulker_box", "red_shulker_box",
        ];
        for id in guarded {
            let block = Block::new(&format!("minecraft:{id}"));
            assert!(guarded_by_piglins(&states, &block), "{id}");
            assert!(mining_angers_piglins(&states, &[((0, 64, 0), block.clone(), None)]), "{id}");
            assert!(!mining_angers_piglins(&states, &[((0, 64, 0), block, Some(4.0))]), "a blast angers nobody: {id}");
        }
        for id in ["stone", "iron_block", "netherrack", "furnace"] {
            let block = Block::new(&format!("minecraft:{id}"));
            assert!(!guarded_by_piglins(&states, &block), "{id}");
            assert!(!mining_angers_piglins(&states, &[((0, 64, 0), block, None)]), "{id}");
        }
    }
}
