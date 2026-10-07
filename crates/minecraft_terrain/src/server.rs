//! The integrated server's world simulation for a streamed world: the
//! exact `minecraftoss_world::level::Level` owns the loaded FULL chunks,
//! takes the player's edits and uses, ticks at 20 Hz, and hands back the
//! positions whose blocks changed so the client scene and the saved chunks
//! follow it.

use crate::scene::{Block, BlockPos, HandcraftedScene, Scene};
use crate::terrain::BlockStates;
use minecraftoss_core::{BlockStateId, Chunk, ChunkPos};
use minecraftoss_world::chunk_map::WorldGen;
use minecraftoss_world::level::{update, Level};
use minecraftoss_world::natural_spawner::tick::{CensusMob, SpawnPlayer};
use std::sync::Arc;

/// How a player changed a block (`BlockItem.place` sets with flags 11,
/// breaking removes the block with flags 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEdit {
    Place,
    Break,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemOnBlock {
    EmptyBucket { water: bool },
    FillBucket,
    Ignite,
    PrimeTnt,
    BoneMeal { face: &'static str },
}

pub struct ServerSim {
    level: Level<'static>,
    states: Arc<BlockStates>,
    /// Mobs, ticked after the level each tick.
    mobs: minecraftoss_entities::world::EntityWorld,
    /// Each block state as mobs read it.
    mob_tables: crate::server_mobs::MobTables,
    /// The tag each mob was loaded or spawned from, by entity ID: saving
    /// overwrites what the entity world simulates and keeps the rest.
    mob_tags: std::collections::HashMap<u64, minecraftoss_core::nbt::Tag>,
    /// The entity storage the level's entities are saved to, shared with
    /// the chunk map (`EntityStorage`).
    storage: Option<Arc<minecraftoss_world::storage::ChunkStorage>>,
    /// Server ticks since the last autosave.
    ticks_since_save: u32,
    entity_loot: Option<minecraftoss_entities::loot::EntityLootBook>,
    shearing_loot: Option<minecraftoss_entities::loot::ShearingLootBook>,
    /// Spawned or loaded mobs the entity world does not simulate yet, each
    /// with its riders, and the tag that saves them unchanged.
    dormant: Vec<(Vec<CensusMob>, minecraftoss_core::nbt::Tag)>,
    /// Creeper blasts since the client last heard of them.
    explosions: Vec<minecraftoss_entities::creeper::CreeperExplosion>,
    /// The player's `takeXpDelay`: ticks until it can take another orb.
    take_xp_delay: i32,
    /// The client was asked to close its trading screen.
    merchant_closing: bool,
    thrown: Vec<crate::thrown::Thrown>,
    pearl_landings: Vec<[f64; 3]>,
}

/// What a player's hit or use on a mob did, for the client to present.
#[derive(Clone, Debug, Default)]
pub struct MobResult {
    pub sounds: Vec<crate::mob_actions::MobSound>,
    /// The acting player's inventory slots the action changed, with what
    /// they now hold.
    pub slots: Vec<(usize, Option<minecraftoss_player::inventory::ItemStack>)>,
    /// A trading screen the action opened (`openTradingScreen`).
    pub merchant: Option<MerchantView>,
}

/// A player's trading screen as the client shows it: the villager's
/// offers, profession, level and experience
/// (`ClientboundMerchantOffersPacket`) and the menu's slots.
#[derive(Clone, Debug, PartialEq)]
pub struct MerchantView {
    pub villager: u64,
    pub profession: &'static str,
    pub level: i32,
    pub xp: i32,
    pub offers: Vec<minecraftoss_entities::trading::MerchantOffer>,
    pub payment: [Option<minecraftoss_player::inventory::ItemStack>; 2],
    pub result: Option<minecraftoss_player::inventory::ItemStack>,
    pub future_xp: i32,
    /// Items' maximum stack sizes the offers name, for the client's
    /// prices.
    pub max_stacks: Vec<(String, i32)>,
}

/// What the player does on the trading screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MerchantOp {
    /// An offer picked in the list.
    Select(i32),
    /// A payment slot clicked (shift: moved back).
    Payment { slot: usize, right: bool, shift: bool },
    /// The result clicked (shift: traded as often as it goes).
    Result { shift: bool },
    Close,
}

/// A trading screen's new state after an operation, or its closing
/// (`view` none), with the acting player's inventory slots it changed and
/// the cursor.
#[derive(Clone, Debug)]
pub struct MerchantUpdate {
    pub view: Option<MerchantView>,
    pub slots: Vec<(usize, Option<minecraftoss_player::inventory::ItemStack>)>,
    pub cursor: Option<minecraftoss_player::inventory::ItemStack>,
    /// The server asks the client to close the screen (the villager went
    /// away or stopped trading); the client answers with `Close`.
    pub closing: bool,
}

/// An item entity on the server, as the client shows it. Components travel
/// as their JSON text.
#[derive(Clone, Debug)]
pub struct ServerItem {
    pub id: i32,
    pub item: String,
    pub count: i32,
    pub components: Option<String>,
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub velocity: [f64; 3],
    pub age: i32,
    pub pickup_delay: i32,
    pub on_ground: bool,
}

/// A falling block entity as the client shows it.
#[derive(Clone, Debug)]
pub struct ServerFallingBlock {
    pub id: i32,
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub block: Block,
    pub start: BlockPos,
}

/// An experience orb as the client shows it.
#[derive(Clone, Copy, Debug)]
pub struct ServerOrb {
    pub id: i32,
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub value: i32,
    /// `tickCount`, which drives its colour.
    pub tick_count: i32,
}

/// The level's entities as the client shows them after a tick.
#[derive(Clone, Debug, Default)]
pub struct EntitySnapshot {
    pub items: Vec<ServerItem>,
    pub tnt: Vec<ServerTnt>,
    pub falling: Vec<ServerFallingBlock>,
    pub orbs: Vec<ServerOrb>,
    pub thrown: Vec<(String, [f64; 3], [f64; 3], i32)>,
}

/// A primed TNT entity as the client shows it.
#[derive(Clone, Copy, Debug)]
pub struct ServerTnt {
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub fuse: i32,
}

fn stack_of(item: &str, count: i32, components: Option<&str>) -> minecraftoss_core::item::ItemStack {
    let mut stack = minecraftoss_core::item::ItemStack::new(item, count);
    stack.components = components.map(|json| minecraftoss_core::nbt::Tag::String(json.to_owned()));
    stack
}

fn components_of(stack: &minecraftoss_core::item::ItemStack) -> Option<String> {
    match &stack.components {
        Some(minecraftoss_core::nbt::Tag::String(json)) => Some(json.clone()),
        Some(other) => Some(format!("{other:?}")),
        None => None,
    }
}

impl ServerSim {
    /// A level for a dimension. The world generation data lives as long as
    /// the process (one small leak per world opened).
    pub fn new(worldgen: Arc<WorldGen>, states: Arc<BlockStates>, dimension_type: &str) -> Self {
        let leaked: &'static Arc<WorldGen> = Box::leak(Box::new(worldgen));
        let worldgen: &'static WorldGen = leaked;
        let range = states.vertical_range();
        let mut level = Level::new(&worldgen.library, range.start, range.end - range.start);
        level.random_sequences = minecraftoss_core::loot::RandomSequences::new(worldgen.terrain.seed);
        level.destroyed = Some(Vec::new());
        if let Err(e) = level.set_dimension(dimension_type) {
            eprintln!("world simulation without environment: {e}");
        }
        let mob_tables = crate::server_mobs::MobTables::new(&level, &states);
        // Natural spawning, when the entity catalog can create mobs.
        if level.registries().entities.is_some() {
            match minecraftoss_world::natural_spawner::CreatureSpawns::load(worldgen.terrain.registries.clone(), dimension_type, false) {
                Ok(spawns) => level.natural_spawning = Some(minecraftoss_world::level::spawning::NaturalSpawning::new(Arc::new(spawns))),
                Err(e) => eprintln!("natural spawning unavailable: {e}"),
            }
        }
        // The players are real entities: arrows hit them.
        let mut mobs = minecraftoss_entities::world::EntityWorld::default();
        mobs.set_players_pickable(true);
        // New mobs' UUIDs differ from session to session, as vanilla's
        // random ones do.
        mobs.set_uuid_salt(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64));
        mobs.pois = minecraftoss_entities::poi::PoiManager::new(range.start >> 4, (range.end - 1) >> 4);
        Self {
            level,
            states,
            mobs,
            mob_tables,
            mob_tags: std::collections::HashMap::new(),
            storage: None,
            ticks_since_save: 0,
            entity_loot: None,
            shearing_loot: None,
            dormant: Vec::new(),
            explosions: Vec::new(),
            take_xp_delay: 0,
            merchant_closing: false,
            thrown: Vec::new(),
            pearl_landings: Vec::new(),
        }
    }

    /// Loads the entity and shearing loot tables and the villager trade
    /// sets from the data JAR, with the world seed's random sequences.
    pub fn load_loot(&mut self, jar: &std::path::Path, seed: u64) {
        self.entity_loot = minecraftoss_entities::loot::EntityLootBook::from_jar(jar, seed).map_err(|e| eprintln!("server entity loot unavailable: {e:#}")).ok();
        self.shearing_loot = minecraftoss_entities::loot::ShearingLootBook::from_jar(jar, seed).map_err(|e| eprintln!("server shearing loot unavailable: {e:#}")).ok();
        // The trade sets read the items' enchantability and stack sizes
        // from the exported item catalog: beside a JAR fetched into a
        // MinecraftOSS-shaped folder, in a checkout, or under the working
        // directory.
        const CATALOG: &str = "artifacts/item-catalog/26.3.json";
        let catalog = [
            jar.parent().map(|root| root.join(CATALOG)),
            std::env::var_os("MINECRAFTOSS_ROOT").map(|root| std::path::Path::new(&root).join(CATALOG)),
            Some(std::path::PathBuf::from(CATALOG)),
        ]
        .into_iter()
        .flatten()
        .find(|path| path.is_file());
        match catalog.map(|path| minecraftoss_entities::trading::TradeBook::from_jar(jar, &path)) {
            Some(Ok(book)) => self.mobs.set_trades(Arc::new(book), seed),
            Some(Err(e)) => eprintln!("server villager trades unavailable: {e:#}"),
            None => eprintln!("server villager trades unavailable: no item catalog"),
        }
    }

    /// A player's hit (`attack`, with what the player brings to it) or item
    /// use on a mob, with a copy of the player's inventory. Drops enter the
    /// level as item entities.
    pub fn mob_action(&mut self, hit: minecraftoss_entities::world::MobHit, attack: Option<minecraftoss_entities::world::PlayerAttack>, mut inventory: minecraftoss_player::inventory::Inventory, selected: usize, infinite: bool) -> MobResult {
        let before = inventory.slots.clone();
        let mut actor = crate::mob_actions::Actor {
            inventory: &mut inventory,
            selected,
            infinite,
            entity_loot: self.entity_loot.as_mut(),
            shearing_loot: self.shearing_loot.as_mut(),
        };
        let outcome = if let Some(attack) = &attack {
            crate::mob_actions::attack(&mut self.mobs, hit, &mut actor, attack)
        } else {
            crate::mob_actions::interact(&mut self.mobs, hit, &mut actor)
        };
        for (stack, position) in outcome.drops {
            let components = stack.components.as_ref().map(|c| c.to_string());
            let stack = stack_of(&stack.id, i32::from(stack.count), components.as_deref());
            self.level.spawn_at_location(position.to_array(), stack);
        }
        for (position, amount) in outcome.experience {
            self.level.award_experience(position.to_array(), amount);
        }
        let slots = inventory
            .slots
            .iter()
            .zip(&before)
            .enumerate()
            .filter(|(_, (now, was))| now != was)
            .map(|(slot, (now, _))| (slot, now.clone()))
            .collect();
        self.spawn_trade_experience();
        MobResult { sounds: outcome.sounds, slots, merchant: self.merchant_view(0) }
    }

    pub fn mob_blast(&mut self, hit: minecraftoss_entities::world::MobHit, damage: f32, center: [f64; 3], impact: f32, player: Option<u64>) {
        let _ = self.mobs.player_blast(player, hit.id(), damage, glam::DVec3::from_array(center), f64::from(impact));
        self.drop_death_loot();
    }

    /// The player's trading screen as the client shows it.
    pub fn merchant_view(&mut self, player: u64) -> Option<MerchantView> {
        let menu = self.mobs.merchant_menu(player)?.clone();
        let offers = self.mobs.villager_offers(menu.villager).map(<[_]>::to_vec).unwrap_or_default();
        let villager = self.mobs.villagers().iter().find(|e| e.id == menu.villager)?;
        let mut max_stacks = Vec::new();
        for offer in &offers {
            for id in std::iter::once(&offer.buy.id).chain(offer.buy_b.as_ref().map(|b| &b.id)).chain(std::iter::once(&offer.sell.id)) {
                if !max_stacks.iter().any(|(item, _): &(String, i32)| item == id) {
                    max_stacks.push((id.clone(), self.mobs.item_max_stack(id)));
                }
            }
        }
        Some(MerchantView {
            villager: menu.villager,
            profession: villager.villager.profession.id(),
            level: villager.villager.level,
            xp: villager.villager.xp,
            offers,
            payment: menu.payment.clone(),
            result: menu.result.clone(),
            future_xp: menu.future_xp,
            max_stacks,
        })
    }

    /// An operation on the player's trading screen, with a copy of the
    /// player's inventory (and the hotbar slot selected): the screen's new
    /// state, the slots it changed and the cursor. Closing returns the
    /// payments and the cursor to the inventory; what does not fit drops
    /// at the player's feet.
    pub fn merchant(&mut self, op: MerchantOp, mut inventory: minecraftoss_player::inventory::Inventory, selected: usize, feet: [f64; 3]) -> MerchantUpdate {
        let before = inventory.slots.clone();
        match op {
            MerchantOp::Select(index) => self.mobs.merchant_select(0, index, &mut inventory),
            MerchantOp::Payment { slot, right, shift } => self.mobs.merchant_click_payment(0, slot, right, shift, &mut inventory),
            MerchantOp::Result { shift } => self.mobs.merchant_click_result(0, shift, &mut inventory),
            MerchantOp::Close => {
                for stack in self.mobs.merchant_close(0, &mut inventory, selected) {
                    let components = stack.components.as_ref().map(|c| c.to_string());
                    self.level.spawn_at_location(feet, stack_of(&stack.id, i32::from(stack.count), components.as_deref()));
                }
                self.merchant_closing = false;
            }
        }
        self.spawn_trade_experience();
        let slots = inventory.slots.iter().zip(&before).enumerate().filter(|(_, (now, was))| now != was).map(|(slot, (now, _))| (slot, now.clone())).collect();
        MerchantUpdate { view: self.merchant_view(0), slots, cursor: inventory.cursor.clone(), closing: false }
    }

    /// `ServerPlayer.tick`'s `stillValid` check on an open trading screen:
    /// once the villager is out of reach, dead or done trading, the client
    /// is asked (once) to close it.
    fn check_merchant(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) -> Option<MerchantUpdate> {
        if self.merchant_closing || self.mobs.merchant_menu(0).is_none() {
            return None;
        }
        let player = players.iter().find(|p| p.id == 0)?;
        let eyes = player.position + glam::DVec3::Y * f64::from(player.eye_height);
        if self.mobs.merchant_still_valid(0, eyes) {
            return None;
        }
        self.merchant_closing = true;
        Some(MerchantUpdate { view: None, slots: Vec::new(), cursor: None, closing: true })
    }

    /// The orbs trades dropped, each whole into the level.
    fn spawn_trade_experience(&mut self) {
        for (position, value) in self.mobs.take_trade_experience() {
            self.level.spawn_experience_orb(position.to_array(), value);
        }
    }

    /// Where the level's entities are saved (the chunk map's storage).
    pub fn set_entity_storage(&mut self, storage: Option<Arc<minecraftoss_world::storage::ChunkStorage>>) {
        self.storage = storage;
    }

    /// A chunk joins the level with its entities (`EntityStorage`): those
    /// saved for it, or for a chunk the level never saved, the ones
    /// generation made. A chunk sent again keeps the entities it has.
    pub fn load_chunk(&mut self, chunk: &Chunk) {
        let fresh = self.level.chunk(chunk.pos).is_none();
        self.level.insert_chunk(chunk.clone());
        if !fresh {
            return;
        }
        self.load_pois(chunk);
        let saved = self.storage.as_ref().and_then(|storage| {
            storage.load_entities(chunk.pos).unwrap_or_else(|e| {
                eprintln!("entities of chunk {:?} failed to load: {e}", chunk.pos);
                None
            })
        });
        for tag in saved.unwrap_or_else(|| chunk.generation.entities.clone()) {
            self.add_saved_entity(tag);
        }
    }

    /// A chunk's points of interest (`PoiManager.checkConsistencyWithBlocks`
    /// for each section that may hold one).
    fn load_pois(&mut self, chunk: &Chunk) {
        let table = &self.mob_tables.pois;
        let kind = |state: BlockStateId| table.get(state.0 as usize).copied().flatten();
        for (i, section) in chunk.sections().iter().enumerate() {
            let sy = chunk.min_section_y() + i as i32;
            let maybe = match &section.blocks {
                minecraftoss_core::palette::PalettedContainer::Single(state) => kind(*state).is_some(),
                minecraftoss_core::palette::PalettedContainer::Direct(values) => values.iter().any(|&s| kind(s).is_some()),
            };
            if maybe {
                self.mobs.pois.load_section((chunk.pos.x, sy, chunk.pos.z), |(x, y, z)| kind(section.block((x & 15) as usize, (y & 15) as usize, (z & 15) as usize)));
            }
        }
    }

    /// The blocks set since the last call move the points of interest
    /// (`ServerLevel.updatePOIOnBlockStateChange`).
    fn sync_pois(&mut self) {
        for (x, y, z) in self.level.take_block_log() {
            let state = self.level.block(minecraftoss_core::BlockPos::new(x, y, z));
            let new = self.mob_tables.pois.get(state.0 as usize).copied().flatten();
            let old = self.mobs.pois.kind((x, y, z));
            self.mobs.pois.block_changed((x, y, z), old, new);
        }
    }

    /// A saved entity joins the level: items and experience orbs as level
    /// entities, mobs the entity world simulates into it, and the rest
    /// dormant.
    fn add_saved_entity(&mut self, tag: minecraftoss_core::nbt::Tag) {
        self.mobs.set_day_time(self.level.overworld_clock());
        if crate::server_mobs::load_level_entity(&mut self.level, &tag) {
            return;
        }
        match crate::server_mobs::spawn_saved(&mut self.mobs, &tag) {
            Some(id) => {
                self.mob_tags.insert(id, tag);
            }
            None => self.dormant.push((minecraftoss_world::natural_spawner::tick::census_of(&tag), tag)),
        }
    }

    /// Saves a chunk's entities: its mobs (from the tag each came from,
    /// with what the entity world changed), its dormant mobs as they came,
    /// and its items and experience orbs. Unloading also takes them out.
    fn save_chunk_entities(&mut self, pos: ChunkPos, unload: bool) {
        let inside = |p: glam::DVec3| crate::server_mobs::in_chunk(p, pos);
        let mut tags: Vec<minecraftoss_core::nbt::Tag> = crate::server_mobs::mob_tags(&self.mobs, inside, &self.mob_tags).into_iter().map(|(_, tag)| tag).collect();
        tags.extend(self.dormant.iter().filter(|(group, _)| inside(glam::DVec3::from_array(group[0].pos))).map(|(_, tag)| tag.clone()));
        tags.extend(crate::server_mobs::level_entity_tags(&self.level, inside));
        if unload {
            self.mobs.remove_where(inside);
            self.dormant.retain(|(group, _)| !inside(glam::DVec3::from_array(group[0].pos)));
            self.level.entities.retain(|e| !(inside(glam::DVec3::from_array(e.pos)) && (e.item_data().is_some() || e.orb_data().is_some())));
            let alive: std::collections::HashSet<u64> = self.mobs.mob_ids().collect();
            self.mob_tags.retain(|id, _| alive.contains(id));
        }
        if let Some(storage) = &self.storage {
            if let Err(e) = storage.save_entities(pos, &tags) {
                eprintln!("entities of chunk {pos:?} failed to save: {e}");
            }
        }
    }

    /// Saves every loaded chunk's entities and writes the entity storage
    /// (autosave and shutdown).
    pub fn save_all_entities(&mut self) {
        let loaded: Vec<ChunkPos> = self.level.chunks().map(|c| c.pos).collect();
        for pos in loaded {
            self.save_chunk_entities(pos, false);
        }
        if let Some(storage) = &self.storage {
            if let Err(e) = storage.flush() {
                eprintln!("entity storage write failed: {e}");
            }
        }
    }

    /// Before the level ticks: natural spawning's players (registered with
    /// its spawn counter), the mob census for its caps, and the simulation
    /// area.
    pub fn prepare_spawning(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) {
        let Some(natural) = &mut self.level.natural_spawning else { return };
        let spawn_players: Vec<SpawnPlayer> = players.iter().map(|p| SpawnPlayer { pos: p.position.to_array(), spectator: p.spectator }).collect();
        natural.set_players(&spawn_players);
        let mut census: Vec<CensusMob> = self
            .mobs
            .census()
            .into_iter()
            .map(|m| {
                let half = f64::from(m.width / 2.0);
                let p = m.position;
                CensusMob {
                    kind: m.kind.to_owned(),
                    pos: p.to_array(),
                    bb: [p.x - half, p.y, p.z - half, p.x + half, p.y + f64::from(m.height), p.z + half],
                    persistent: m.persistent,
                    riding: false,
                }
            })
            .collect();
        census.extend(self.dormant.iter().flat_map(|(group, _)| group.iter().cloned()));
        natural.census = census;
        natural.simulation = self.level.ticking_chunks.iter().flatten().copied().collect();
    }

    /// Mobs natural spawning made join the entity world; types it does not
    /// simulate yet (and jockeys) wait dormant: they count toward the caps
    /// and block spawns until they despawn.
    fn take_spawned(&mut self) {
        for tag in std::mem::take(&mut self.level.spawned) {
            self.add_saved_entity(tag);
        }
    }

    /// `Mob.checkDespawn` for the mobs in loaded chunks, before they tick.
    fn despawn_mobs(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate], difficulty: i32) {
        let feet: Vec<glam::DVec3> = players.iter().filter(|p| !p.spectator).map(|p| p.position).collect();
        let level = &self.level;
        let loaded = |p: glam::DVec3| level.chunk(ChunkPos::new((p.x.floor() as i32) >> 4, (p.z.floor() as i32) >> 4)).is_some();
        self.mobs.check_despawn(&feet, difficulty == 0, &loaded);
        // Dormant monsters go in peaceful and beyond 128 blocks (they never
        // idle: they do not tick).
        self.dormant.retain(|(group, _)| {
            let root = &group[0];
            if root.persistent || minecraftoss_world::natural_spawner::spawning::MobCategory::of_type(&root.kind) != Some(minecraftoss_world::natural_spawner::spawning::MobCategory::Monster) {
                return true;
            }
            if difficulty == 0 {
                return false;
            }
            let at = glam::DVec3::from_array(root.pos);
            feet.iter().map(|p| p.distance_squared(at)).min_by(f64::total_cmp).is_none_or(|d| d <= 128.0 * 128.0)
        });
    }

    /// The mobs spawned that the entity world does not simulate.
    pub fn dormant_count(&self) -> usize {
        self.dormant.len()
    }

    /// The recipe book mobs consult (breeding colours).
    pub fn set_recipe_book(&mut self, recipes: Arc<minecraftoss_player::crafting::RecipeBook>) {
        self.mobs.set_recipe_book(recipes);
    }

    /// Mobs tick after the level, inside the entity-ticking range (the
    /// level's ticking chunks, `DistanceManager.inEntityTickingRange`).
    pub fn tick_mobs(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate], bright_outside: bool) {
        self.sync_pois();
        let ticking: std::collections::HashSet<ChunkPos> = self.level.ticking_chunks.iter().flatten().copied().collect();
        self.mobs.set_bright_outside(bright_outside);
        self.mobs.set_monsters_burn(self.level.monsters_burn());
        let Self { level, states, mobs, mob_tables, .. } = self;
        let mut world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(level), states, tables: mob_tables };
        let ticks = |p: glam::DVec3| ticking.contains(&ChunkPos::new((p.x.floor() as i32) >> 4, (p.z.floor() as i32) >> 4));
        // Brains draw from the level's own random, and villagers keep the
        // overworld clock's schedule.
        mobs.set_day_time(world.level.borrow().overworld_clock());
        let shared = match &world.level.borrow().random {
            minecraftoss_core::random::AnyRandom::Legacy(random) => Some((random.state(), random.gaussian_cache())),
            _ => None,
        };
        if let Some((state, _)) = shared {
            *mobs.level_random_mut() = minecraftoss_player::rng::LegacyRandom::from_raw_state(state);
        }
        mobs.tick_with_players_where(&mut world, players, &ticks);
        // Births and summons are listed for replays that tag them; the game
        // needs no list.
        let _ = (mobs.take_born_villagers(), mobs.take_born_wolves(), mobs.take_summoned_golems());
        if let Some((_, gaussian)) = shared {
            if let minecraftoss_core::random::AnyRandom::Legacy(random) = &mut world.level.borrow_mut().random {
                random.set_seed((mobs.level_random_mut().raw_state() ^ 0x5DEE_CE66D) as i64);
                random.set_gaussian_cache(gaussian);
            }
        }
        // `ServerPlayer.doTick` (the connection tick, after the levels):
        // the players push the mobs they walk into.
        mobs.push_from_players(players, &ticks);
        // Entity events (a villager's hearts, anger, happiness) are for the
        // client's particles, which it does not draw yet.
        let _ = mobs.take_entity_events();
        // A blast's victims drop their loot before its blocks break
        // (`ServerExplosion.explode`: `hurtEntities`, then the blocks).
        self.drop_death_loot();
        self.explode_creepers();
    }

    /// The loot and experience of the mobs that died, into the level as
    /// item entities and experience orbs (`dropAllDeathLoot`).
    fn drop_death_loot(&mut self) {
        let (drops, experience) = crate::mob_actions::death_remains(&mut self.mobs, self.entity_loot.as_mut());
        for (stack, position) in drops {
            let components = stack.components.as_ref().map(|c| c.to_string());
            let stack = stack_of(&stack.id, i32::from(stack.count), components.as_deref());
            self.level.spawn_at_location(position.to_array(), stack);
        }
        for (position, amount) in experience {
            self.level.award_experience(position.to_array(), amount);
        }
    }

    /// `Player.tick`'s `takeXpDelay` countdown, then `Player.aiStep` touching
    /// one orb (at random) when the player at `feet` can take one: returns
    /// the orb's ID, position and value.
    pub fn take_experience(&mut self, feet: Option<[f64; 3]>) -> Option<(i32, [f64; 3], i32)> {
        if self.take_xp_delay > 0 {
            self.take_xp_delay -= 1;
        }
        if self.take_xp_delay != 0 {
            return None;
        }
        let taken = self.level.player_touch_orb(feet?)?;
        self.take_xp_delay = 2;
        Some(taken)
    }

    /// The server's experience orbs.
    pub fn orbs(&self) -> Vec<ServerOrb> {
        self.level
            .entities
            .iter()
            .filter_map(|e| e.orb_data().map(|d| ServerOrb { id: e.id, position: e.pos, previous_position: e.old_position(), value: d.value, tick_count: e.tick_count }))
            .collect()
    }

    /// Experience at a position (`ExperienceOrb.award`), as orbs.
    pub fn award_experience(&mut self, position: [f64; 3], amount: i32) {
        self.level.award_experience(position, amount);
    }

    /// `summon minecraft:experience_orb` with its `Value`: a still orb.
    pub fn summon_orb(&mut self, position: [f64; 3], value: i32) {
        self.level.add_entity(minecraftoss_world::level::entity::Entity::experience_orb(position, value));
    }

    /// `/summon` for a mob: the level makes its tag (`SummonCommand`), and
    /// it joins the entity world as a loaded mob would.
    pub fn summon(&mut self, kind: &str, position: [f64; 3], nbt: Option<&minecraftoss_core::nbt::Tag>, y_rot: f32) -> Result<(), String> {
        let tag = self.level.summon_mob(kind, position, nbt, y_rot)?;
        // A new brain reads the schedule for the time it is.
        self.mobs.set_day_time(self.level.overworld_clock());
        self.add_saved_entity(tag);
        Ok(())
    }

    pub fn hatch_egg(&mut self, kind: &str, position: [f64; 3], y_rot: f32) -> Result<(), String> {
        let tag = self.level.summon_mob(kind, position, None, y_rot)?;
        self.mobs.set_day_time(self.level.overworld_clock());
        let id = crate::server_mobs::spawn_saved(&mut self.mobs, &tag).ok_or_else(|| format!("{kind} is not simulated"))?;
        self.mob_tags.insert(id, tag);
        Ok(())
    }

    /// Creeper blasts reach the level (`ServerLevel.explode` with
    /// `ExplosionInteraction.MOB`): blocks break when mobs may grief, with
    /// the drops decaying (`mob_explosion_drop_decay`), and items, TNT and
    /// falling blocks are hurt and pushed. The entity world already hurt
    /// its mobs and the players.
    fn explode_creepers(&mut self) {
        use minecraftoss_world::level::explosion::{BlockInteraction, Explosion};
        for blast in self.mobs.take_creeper_explosions() {
            let interaction = if self.mobs.mob_griefing() { BlockInteraction::DestroyWithDecay } else { BlockInteraction::Keep };
            self.level.explode(Explosion { center: blast.position.to_array(), radius: blast.radius, fire: false, interaction, source: None });
            self.explosions.push(blast);
        }
    }

    /// The blasts since the last call, for the client's sound and particles.
    pub fn take_explosions(&mut self) -> Vec<minecraftoss_entities::creeper::CreeperExplosion> {
        std::mem::take(&mut self.explosions)
    }

    pub fn throw(&mut self, thrown: crate::thrown::Thrown) {
        self.thrown.push(thrown);
    }

    pub fn take_pearl_landings(&mut self) -> Vec<[f64; 3]> {
        std::mem::take(&mut self.pearl_landings)
    }

    pub fn thrown_view(&self) -> Vec<(String, [f64; 3], [f64; 3], i32)> {
        self.thrown.iter().map(|t| (t.item.clone(), t.previous.to_array(), t.position().to_array(), t.age())).collect()
    }

    pub fn tick_thrown(&mut self, thrower: Option<[f64; 3]>) {
        use crate::thrown::Throwable;
        if thrower.is_none() {
            self.thrown.retain(|t| t.kind != Throwable::EnderPearl);
        }
        if self.thrown.is_empty() {
            return;
        }
        let targets = crate::thrown::targets(&self.mobs);
        let owner = thrower.map(|f| (glam::DVec3::new(f[0] - 0.3, f[1], f[2] - 0.3), glam::DVec3::new(f[0] + 0.3, f[1] + 1.8, f[2] + 0.3)));
        let mut struck = Vec::new();
        let mut hatched = Vec::new();
        {
            let Self { level, states, mob_tables, thrown, pearl_landings, .. } = self;
            let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(level), states, tables: mob_tables };
            for t in thrown.iter_mut() {
                let Some(landed) = t.tick(&world, &targets, owner) else { continue };
                if let Some(mob) = landed.mob {
                    struck.push((mob, landed.velocity, t.random().next_long()));
                }
                match t.kind {
                    Throwable::Egg => {
                        for _ in 0..crate::thrown::chicks(t.random()) {
                            hatched.push((crate::thrown::hatch_point(landed.at, landed.before).to_array(), t.yaw(), crate::thrown::chick_variant(&t.item)));
                        }
                    }
                    Throwable::EnderPearl => pearl_landings.push(landed.before.to_array()),
                    Throwable::Snowball => {}
                }
            }
            thrown.retain(crate::thrown::Thrown::alive);
        }
        for (mob, velocity, seed) in struck {
            crate::thrown::hurt(&mut self.mobs, mob, velocity, 0, &mut minecraftoss_player::rng::LegacyRandom::new(seed));
        }
        for (at, yaw, variant) in hatched {
            if let Ok(nbt) = minecraftoss_core::snbt::parse_compound(&format!("{{Age:-24000,variant:\"{variant}\"}}")) {
                let _ = self.summon("minecraft:chicken", at, Some(&nbt), yaw);
            }
        }
    }

    /// The mobs a player at `position` tracks (`ChunkMap.TrackedEntity`:
    /// within `range` blocks horizontally).
    pub fn tracked_mobs(&self, position: [f64; 3], range: f64) -> minecraftoss_entities::world::EntityWorld {
        let range_sq = range * range;
        self.mobs.clone_where(|p| {
            let (dx, dz) = (p.x - position[0], p.z - position[2]);
            dx * dx + dz * dz <= range_sq
        })
    }

    /// A chunk leaves the level; its entities are saved and taken out.
    pub fn unload_chunk(&mut self, pos: ChunkPos) {
        self.save_chunk_entities(pos, true);
        self.level.remove_chunk(pos);
        self.mobs.pois.unload_chunk(pos.x, pos.z);
    }

    fn state_of(&self, block: Option<&Block>) -> BlockStateId {
        block.and_then(|b| self.states.state_of(b)).unwrap_or(BlockStateId::AIR)
    }

    pub fn player_breaks(&mut self, positions: &[BlockPos]) {
        let at: Vec<_> = positions.iter().map(|p| minecraftoss_core::BlockPos::new(p.0, p.1, p.2)).collect();
        self.level.remove_blocks(&at, BlockStateId::AIR);
    }

    pub fn take_destroyed(&mut self) -> Vec<(BlockPos, Block)> {
        let destroyed = self.level.destroyed.as_mut().map_or_else(Vec::new, std::mem::take);
        destroyed
            .into_iter()
            .filter_map(|(pos, state)| self.states.block(state).cloned().map(|block| ((pos.x, pos.y, pos.z), block)))
            .collect()
    }

    /// Applies a player's edit that the client scene already shows.
    pub fn player_edit(&mut self, scene: &HandcraftedScene, pos: BlockPos, edit: PlayerEdit) {
        self.player_edit_block(pos, Scene::block(scene, pos), edit);
    }

    /// A player's edit to `block` (`None` for air) at a position.
    pub fn player_edit_block(&mut self, pos: BlockPos, block: Option<&Block>, edit: PlayerEdit) {
        let target = self.state_of(block);
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        if self.level.block(at) == target {
            return;
        }
        let flags = match edit {
            PlayerEdit::Place => update::ALL | 8,
            PlayerEdit::Break => update::ALL,
        };
        self.level.set_block(at, target, flags, update::LIMIT);
        if matches!(edit, PlayerEdit::Place) {
            self.power_placed_opening(at, block);
        }
    }

    fn power_placed_opening(&mut self, at: minecraftoss_core::BlockPos, block: Option<&Block>) {
        let Some(block) = block else { return };
        let path = block.id.path.as_str();
        let door = path.ends_with("_door") && !path.ends_with("_trapdoor");
        if !(door || path.ends_with("_trapdoor") || path.ends_with("_fence_gate")) {
            return;
        }
        let other = if block.properties.get("half").map(String::as_str) == Some("upper") { at.y - 1 } else { at.y + 1 };
        let signal = self.level.has_neighbor_signal(at) || (door && self.level.has_neighbor_signal(minecraftoss_core::BlockPos::new(at.x, other, at.z)));
        if !signal {
            return;
        }
        let blocks = &self.level.registries().blocks;
        let state = self.level.block(at);
        if let Some(state) = blocks.with_property(state, "powered", "true").and_then(|s| blocks.with_property(s, "open", "true")) {
            self.level.set_block(at, state, update::CLIENTS | 8, update::LIMIT);
        }
    }

    /// `useWithoutItem` on a simulated block; false when not simulated.
    pub fn use_block(&mut self, pos: BlockPos, player_facing: &str) -> bool {
        let facing = minecraftoss_core::pos::Direction::from_name(player_facing);
        self.level.use_block_facing(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2), facing)
    }

    /// `BoneMealItem.useOn` on a clicked face: grow the block, or water
    /// plants in front of a sturdy face. True when the bone meal is used.
    pub fn bone_meal(&mut self, pos: BlockPos, face: &str) -> bool {
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        if self.level.grow_crop(at) {
            return true;
        }
        let Some(face) = minecraftoss_core::pos::Direction::from_name(face) else { return false };
        let state = self.level.block(at);
        self.level.registries().blocks.is_face_sturdy(state, face, minecraftoss_core::SupportType::Full) && self.level.grow_water_plant(at.relative(face, 1))
    }

    pub fn attack_block(&mut self, pos: BlockPos) -> bool {
        self.level.attack_block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2))
    }

    pub fn item_on_block(&mut self, pos: BlockPos, action: ItemOnBlock) -> bool {
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        match action {
            ItemOnBlock::EmptyBucket { water } => self.level.empty_bucket_into(at, water),
            ItemOnBlock::FillBucket => self.level.bucket_pickup(at).is_some(),
            ItemOnBlock::Ignite => self.level.place_fire(at),
            ItemOnBlock::PrimeTnt => self.level.ignite_tnt(at),
            ItemOnBlock::BoneMeal { face } => self.bone_meal(pos, face),
        }
    }

    /// The players the level can see (fire spreads near them), and the
    /// difficulty (`Difficulty.getId`).
    pub fn set_players(&mut self, positions: &[[f64; 3]], difficulty: i32) {
        self.level.players = positions.to_vec();
        self.level.difficulty = difficulty;
    }

    /// The chunks random ticks run in: every loaded chunk within
    /// `distance` (Chebyshev) of `center`, in X then Z order.
    pub fn set_simulation_area(&mut self, center: (i32, i32), distance: i32) {
        let mut list = Vec::with_capacity(((2 * distance + 1) * (2 * distance + 1)) as usize);
        for x in center.0 - distance..=center.0 + distance {
            for z in center.1 - distance..=center.1 + distance {
                let pos = ChunkPos::new(x, z);
                if self.level.chunk(pos).is_some() {
                    list.push(pos);
                }
            }
        }
        self.level.ticking_chunks = Some(list);
    }

    /// Keeps the level's default clock with the client's day time.
    pub fn set_time(&mut self, ticks: i64) {
        self.level.set_time(ticks);
    }

    pub fn set_weather(&mut self, weather: &str) -> bool {
        const LASTING: i32 = 1_000_000_000;
        match weather {
            "clear" => self.level.set_weather(LASTING, 0, false, false),
            "rain" => self.level.set_weather(0, LASTING, true, false),
            "thunder" => self.level.set_weather(0, LASTING, true, true),
            _ => return false,
        }
        true
    }

    pub fn weather(&mut self, feet: Option<[f64; 3]>) -> WeatherView {
        let (rain_level, thunder_level) = self.level.sky.as_ref().map_or((0.0, 0.0), |sky| (sky.weather.rain_level, sky.weather.thunder_level * sky.weather.rain_level));
        let player_in_rain = feet.is_some_and(|feet| {
            let (x, z) = (feet[0].floor() as i32, feet[2].floor() as i32);
            self.level.is_raining_at(minecraftoss_core::BlockPos::new(x, feet[1].floor() as i32, z)) || self.level.is_raining_at(minecraftoss_core::BlockPos::new(x, (feet[1] + PLAYER_HEIGHT).floor() as i32, z))
        });
        WeatherView { rain_level, thunder_level, player_in_rain }
    }

    pub fn tick(&mut self) {
        let bursting: Vec<(i32, [f64; 3])> = self.level.entities.iter().filter(|e| e.tnt_data().is_some_and(|d| d.fuse <= 1)).map(|e| (e.id, e.pos)).collect();
        self.level.tick();
        self.level.unsupported.clear();
        for (id, position) in bursting {
            if self.level.tnt_explodes && !self.level.entities.iter().any(|e| e.id == id) {
                self.explosions.push(minecraftoss_entities::creeper::CreeperExplosion { position: glam::DVec3::from_array(position), radius: TNT_BLAST_RADIUS, powered: false });
            }
        }
    }

    /// Hands an item entity the client created (a drop, a spill) to the
    /// server, which simulates it from then on; returns its entity ID.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_item(&mut self, item: &str, count: i32, components: Option<&str>, position: [f64; 3], velocity: [f64; 3], pickup_delay: i32, age: i32) -> i32 {
        self.level.spawn_item_with(position, stack_of(item, count, components), velocity, pickup_delay, age)
    }

    pub fn keep_dropped_contents(&mut self, pos: BlockPos, item: &str, components: &str) {
        let centre = [f64::from(pos.0) + 0.5, f64::from(pos.1) + 0.5, f64::from(pos.2) + 0.5];
        let distance = |at: [f64; 3]| (0..3).map(|k| (at[k] - centre[k]).powi(2)).sum::<f64>();
        let dropped = self
            .level
            .entities
            .iter_mut()
            .filter_map(|e| {
                let at = e.pos;
                match &mut e.kind {
                    minecraftoss_world::level::entity::EntityKind::Item(data) if data.stack.id == item && data.stack.components.is_none() && distance(at) < 4.0 => Some((distance(at), data)),
                    _ => None,
                }
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match dropped {
            Some((_, data)) => data.stack.components = Some(minecraftoss_core::nbt::Tag::String(components.to_owned())),
            None => {
                self.level.spawn_item_with(centre, stack_of(item, 1, Some(components)), [0.0; 3], 10, 0);
            }
        }
    }

    /// The server's item entities.
    pub fn items(&self) -> Vec<ServerItem> {
        self.level
            .entities
            .iter()
            .filter_map(|e| e.item_data().map(|d| (e, d)))
            .map(|(e, d)| ServerItem {
                id: e.id,
                item: d.stack.id.clone(),
                count: d.stack.count,
                components: components_of(&d.stack),
                position: e.pos,
                previous_position: e.old_position(),
                velocity: e.delta,
                age: d.age,
                pickup_delay: d.pickup_delay,
                on_ground: e.on_ground,
            })
            .collect()
    }

    /// The server's falling blocks.
    pub fn falling_blocks(&self) -> Vec<ServerFallingBlock> {
        self.level
            .entities
            .iter()
            .filter_map(|e| {
                e.falling_data().map(|d| ServerFallingBlock {
                    id: e.id,
                    position: e.pos,
                    previous_position: e.old_position(),
                    block: self.states.block(d.state).cloned().unwrap_or_else(|| Block::new(&self.level.registries().blocks.block(self.level.registries().blocks.block_of(d.state)).name.to_string())),
                    start: (d.start.x, d.start.y, d.start.z),
                })
            })
            .collect()
    }

    /// The server's primed TNT.
    pub fn primed_tnt(&self) -> Vec<ServerTnt> {
        self.level
            .entities
            .iter()
            .filter_map(|e| e.tnt_data().map(|d| ServerTnt { position: e.pos, previous_position: e.old_position(), fuse: d.fuse }))
            .collect()
    }

    /// `ItemEntity.playerTouch` for a player standing at `feet`: `take`
    /// offers each touching stack (item, count, components) and returns how
    /// many the inventory took. Returns (entity ID, position, item, count,
    /// components) per pickup.
    pub fn pickup(&mut self, feet: [f64; 3], mut take: impl FnMut(&str, i32, Option<&str>) -> i32) -> Vec<(i32, [f64; 3], String, i32, Option<String>)> {
        self.level
            .player_touch_items(feet, |stack| {
                let components = components_of(stack);
                take(&stack.id, stack.count, components.as_deref())
            })
            .into_iter()
            .map(|(id, position, stack)| {
                let components = components_of(&stack);
                (id, position, stack.id, stack.count, components)
            })
            .collect()
    }

    /// Positions changed since the last call, with the block now there.
    pub fn take_changes(&mut self) -> Vec<(BlockPos, Option<Block>)> {
        let changed = self.level.take_changed();
        if std::env::var_os("MINECRAFTOSS_DEBUG_SERVER").is_some() && !changed.is_empty() {
            eprintln!("server changes at tick {}: {} (first {:?})", self.level.game_time, changed.len(), &changed[..changed.len().min(4)]);
        }
        changed
            .into_iter()
            .map(|(x, y, z)| {
                let state = self.level.block(minecraftoss_core::BlockPos::new(x, y, z));
                let block = if self.level.registries().blocks.is_air(state) { None } else { self.states.block(state).cloned() };
                ((x, y, z), block)
            })
            .collect()
    }
}

/// What the client asks of the integrated server thread, in order.
pub enum Command {
    LoadChunk(Arc<Chunk>),
    UnloadChunk(ChunkPos),
    /// A block the client set, as the client scene now shows it.
    PlayerEdit { pos: BlockPos, block: Option<Block>, edit: PlayerEdit },
    PlayerBreaks(Vec<BlockPos>),
    UseBlock { pos: BlockPos, facing: &'static str },
    BoneMeal { pos: BlockPos, face: &'static str },
    Attack(BlockPos),
    ItemOnBlock { pos: BlockPos, action: ItemOnBlock },
    SpawnItem { item: String, count: i32, components: Option<String>, position: [f64; 3], velocity: [f64; 3], pickup_delay: i32, age: i32 },
    KeepDroppedContents { pos: BlockPos, item: String, components: String },
    /// The recipe book, for mobs.
    RecipeBook(Arc<minecraftoss_player::crafting::RecipeBook>),
    /// The data JAR and world seed the server's loot tables come from.
    LootTables { jar: std::path::PathBuf, seed: u64 },
    /// `summon minecraft:experience_orb`.
    SummonOrb { position: [f64; 3], value: i32 },
    AwardExperience { position: [f64; 3], amount: i32 },
    /// `/summon` for a mob, with the command's NBT and the new mob's own
    /// random yaw.
    Summon { kind: String, position: [f64; 3], nbt: Option<minecraftoss_core::nbt::Tag>, y_rot: f32 },
    HatchEgg { kind: String, position: [f64; 3], y_rot: f32 },
    /// A player's hit (`attack`) or item use on a mob, with a copy of its
    /// inventory.
    MobAction { hit: minecraftoss_entities::world::MobHit, attack: Option<minecraftoss_entities::world::PlayerAttack>, inventory: Box<minecraftoss_player::inventory::Inventory>, selected: usize, infinite: bool },
    MobBlast { hit: minecraftoss_entities::world::MobHit, damage: f32, center: [f64; 3], impact: f32, player: Option<u64> },
    /// An operation on the player's trading screen.
    Merchant { op: MerchantOp, inventory: Box<minecraftoss_player::inventory::Inventory>, selected: usize, feet: [f64; 3] },
    /// One server tick, with the player's state for it.
    Tick(Box<TickInput>),
    Throw(Box<crate::thrown::Thrown>),
}

/// The client state a server tick reads.
pub struct TickInput {
    pub day_ticks: i64,
    pub players: Vec<[f64; 3]>,
    pub difficulty: i32,
    pub simulation_center: (i32, i32),
    pub simulation_distance: i32,
    /// The player's feet, when it can pick items up, with its inventory and
    /// selected slot (the server offers touching stacks to this copy).
    pub pickup: Option<([f64; 3], Box<minecraftoss_player::inventory::Inventory>, usize)>,
    /// The player as mobs see it (held food, whether it can be targeted).
    pub mob_players: Vec<minecraftoss_entities::tempt::PlayerCandidate>,
    /// Where each of those players looks (endermen read stares from it).
    pub mob_views: Vec<(u64, minecraftoss_entities::enderman::PlayerView)>,
    /// Each of those players' health, effects and motion (witches pick
    /// their potions by them).
    pub mob_vitals: Vec<(u64, minecraftoss_entities::monster_ai::PlayerVitals)>,
    /// `Level.isBrightOutside`.
    pub bright_outside: bool,
    /// The player's position and how far away it tracks mobs.
    pub tracking: ([f64; 3], f64),
    /// The hurts that landed on the player since the last tick: the mob
    /// behind each and the damage type (its tame wolves avenge it).
    pub player_hurts: Vec<(Option<u64>, &'static str)>,
    /// The `spawn_mobs` game rule.
    pub spawn_mobs: bool,
}

const PLAYER_HEIGHT: f64 = 1.8;
const TNT_BLAST_RADIUS: f32 = 4.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeatherView {
    pub rain_level: f32,
    pub thunder_level: f32,
    pub player_in_rain: bool,
}

/// What the server thread sends back after handling commands.
#[derive(Default)]
pub struct Output {
    /// Blocks that changed, with the block now there.
    pub changes: Vec<(BlockPos, Option<Block>)>,
    pub destroyed: Vec<(BlockPos, Block)>,
    /// Entity snapshots, after a tick.
    pub entities: Option<EntitySnapshot>,
    /// Experience orbs the player took: (entity ID, position, value).
    pub orbs_taken: Vec<(i32, [f64; 3], i32)>,
    /// The mobs the player tracks, after a tick.
    pub mobs: Option<Box<minecraftoss_entities::world::EntityWorld>>,
    /// What the player's mob actions did, in order.
    pub mob_results: Vec<MobResult>,
    /// The trading screen's updates, in order.
    pub merchant: Vec<MerchantUpdate>,
    /// Mobs' hits on the players, in order.
    pub player_hits: Vec<minecraftoss_entities::world::PlayerHit>,
    /// Splash potions that broke near players, in order.
    pub player_splashes: Vec<(u64, minecraftoss_entities::world::PlayerSplash)>,
    /// Where splash potions broke (their sound and colour).
    pub potion_breaks: Vec<minecraftoss_entities::world::PotionBreak>,
    /// Creeper blasts this tick (`ClientboundExplodePacket`).
    pub explosions: Vec<minecraftoss_entities::creeper::CreeperExplosion>,
    /// Sounds mobs made this tick.
    pub mob_sounds: Vec<minecraftoss_entities::world::MobSound>,
    /// Items the player picked up: (entity ID, position, item, count, components).
    pub picked: Vec<(i32, [f64; 3], String, i32, Option<String>)>,
    /// What each `/summon` did: the type it made, or why it failed.
    pub summoned: Vec<Result<String, String>>,
    /// Positions where bone meal was used.
    pub bone_meal_used: Vec<BlockPos>,
    pub item_uses: Vec<bool>,
    pub pearl_landings: Vec<[f64; 3]>,
    pub weather: Option<WeatherView>,
    /// How many commands the server has handled so far.
    pub handled: u64,
    /// The last tick's phases, light solves and total milliseconds.
    pub tick_phases: Option<([f64; 6], (u32, f64), f64)>,
}

/// The integrated server on its own thread, as vanilla runs it: the client
/// sends commands and never waits for a tick.
pub struct ServerHandle {
    commands: Option<std::sync::mpsc::Sender<Command>>,
    outputs: std::sync::mpsc::Receiver<Output>,
    states: Arc<BlockStates>,
    /// Per block state: whether the level acts on a use or an attack.
    uses: Arc<Vec<bool>>,
    attacks: Arc<Vec<bool>>,
    sent: u64,
    /// Outputs [`Self::wait_idle`] received ahead of the next poll.
    waited: Vec<Output>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ServerHandle {
    pub fn spawn(sim: ServerSim) -> Self {
        let states = sim.states.clone();
        let count = sim.level.registries().blocks.state_count();
        let uses: Vec<bool> = (0..count).map(|i| sim.level.handles_use(BlockStateId(i as u16))).collect();
        let attacks: Vec<bool> = (0..count).map(|i| sim.level.handles_attack(BlockStateId(i as u16))).collect();
        let (commands, receiver) = std::sync::mpsc::channel::<Command>();
        let (sender, outputs) = std::sync::mpsc::channel::<Output>();
        let thread = std::thread::Builder::new()
            .name("Server thread".into())
            .spawn(move || server_loop(sim, receiver, sender))
            .expect("server thread starts");
        Self { commands: Some(commands), outputs, states, uses: Arc::new(uses), attacks: Arc::new(attacks), sent: 0, waited: Vec::new(), thread: Some(thread) }
    }

    fn send(&mut self, command: Command) {
        self.sent += 1;
        if let Some(commands) = &self.commands {
            let _ = commands.send(command);
        }
    }

    /// Commands sent so far (compare with `Output::handled`).
    pub fn sent(&self) -> u64 {
        self.sent
    }

    pub fn load_chunk(&mut self, chunk: &Arc<Chunk>) {
        self.send(Command::LoadChunk(chunk.clone()));
    }

    pub fn unload_chunk(&mut self, pos: ChunkPos) {
        self.send(Command::UnloadChunk(pos));
    }

    pub fn player_edit(&mut self, scene: &HandcraftedScene, pos: BlockPos, edit: PlayerEdit) {
        let block = Scene::block(scene, pos).cloned();
        self.send(Command::PlayerEdit { pos, block, edit });
    }

    pub fn player_breaks(&mut self, positions: &[BlockPos]) {
        if !positions.is_empty() {
            self.send(Command::PlayerBreaks(positions.to_vec()));
        }
    }

    fn state_in(&self, scene: &HandcraftedScene, pos: BlockPos) -> Option<BlockStateId> {
        Scene::block(scene, pos).and_then(|b| self.states.state_of(b))
    }

    /// `useWithoutItem` on a simulated block: true (and sent) when the level
    /// acts on the block the scene shows.
    pub fn use_block(&mut self, scene: &HandcraftedScene, pos: BlockPos, facing: &'static str) -> bool {
        let handled = self.state_in(scene, pos).is_some_and(|s| self.uses.get(usize::from(s.0)).copied().unwrap_or(false));
        if handled {
            self.send(Command::UseBlock { pos, facing });
        }
        handled
    }

    /// Bone meal on a clicked face; whether it was used arrives in an output.
    pub fn bone_meal(&mut self, pos: BlockPos, face: &'static str) {
        self.send(Command::BoneMeal { pos, face });
    }

    /// A player's attack; true (and sent) when the level acts on the block.
    pub fn attack_block(&mut self, scene: &HandcraftedScene, pos: BlockPos) -> bool {
        let handled = self.state_in(scene, pos).is_some_and(|s| self.attacks.get(usize::from(s.0)).copied().unwrap_or(false));
        if handled {
            self.send(Command::Attack(pos));
        }
        handled
    }

    pub fn item_on_block(&mut self, pos: BlockPos, action: ItemOnBlock) {
        self.send(Command::ItemOnBlock { pos, action });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_item(&mut self, item: &str, count: i32, components: Option<&str>, position: [f64; 3], velocity: [f64; 3], pickup_delay: i32, age: i32) {
        self.send(Command::SpawnItem { item: item.to_owned(), count, components: components.map(str::to_owned), position, velocity, pickup_delay, age });
    }

    pub fn keep_dropped_contents(&mut self, pos: BlockPos, item: &str, components: &str) {
        self.send(Command::KeepDroppedContents { pos, item: item.to_owned(), components: components.to_owned() });
    }

    pub fn tick(&mut self, input: TickInput) {
        self.send(Command::Tick(Box::new(input)));
    }

    pub fn set_recipe_book(&mut self, recipes: Arc<minecraftoss_player::crafting::RecipeBook>) {
        self.send(Command::RecipeBook(recipes));
    }

    pub fn load_loot(&mut self, jar: std::path::PathBuf, seed: u64) {
        self.send(Command::LootTables { jar, seed });
    }

    /// An operation on the player's trading screen, with a copy of the
    /// player's inventory.
    pub fn merchant(&mut self, op: MerchantOp, inventory: &minecraftoss_player::inventory::Inventory, selected: usize, feet: [f64; 3]) {
        self.send(Command::Merchant { op, inventory: Box::new(inventory.clone()), selected, feet });
    }

    /// Summons a still experience orb worth `value`.
    pub fn summon_orb(&mut self, position: [f64; 3], value: i32) {
        self.send(Command::SummonOrb { position, value });
    }

    pub fn award_experience(&mut self, position: [f64; 3], amount: i32) {
        if amount > 0 {
            self.send(Command::AwardExperience { position, amount });
        }
    }

    /// `/summon` for a mob; what it did arrives in an output. The new
    /// mob's yaw comes from its own random (`LivingEntity`'s constructor:
    /// `nextFloat() * (float)(Math.PI * 2)`, in degrees), seeded here from
    /// the clock as a fresh entity's is.
    pub fn summon(&mut self, kind: String, position: [f64; 3], nbt: Option<minecraftoss_core::nbt::Tag>) {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as i64);
        let y_rot = minecraftoss_core::random::LegacyRandom::new(nanos).next_f32() * (std::f64::consts::PI * 2.0) as f32;
        self.send(Command::Summon { kind, position, nbt, y_rot });
    }

    pub fn hatch_egg(&mut self, kind: String, position: [f64; 3], y_rot: f32) {
        self.send(Command::HatchEgg { kind, position, y_rot });
    }

    /// Hands a player's hit or use on a mob to the server; its result
    /// arrives in an output.
    pub fn mob_action(&mut self, hit: minecraftoss_entities::world::MobHit, attack: Option<minecraftoss_entities::world::PlayerAttack>, inventory: &minecraftoss_player::inventory::Inventory, selected: usize, infinite: bool) {
        self.send(Command::MobAction { hit, attack, inventory: Box::new(inventory.clone()), selected, infinite });
    }

    pub fn mob_blast(&mut self, hit: minecraftoss_entities::world::MobHit, damage: f32, center: [f64; 3], impact: f32, player: Option<u64>) {
        self.send(Command::MobBlast { hit, damage, center, impact, player });
    }

    pub fn throw(&mut self, thrown: crate::thrown::Thrown) {
        self.send(Command::Throw(Box::new(thrown)));
    }

    /// Outputs the server has produced since the last call.
    pub fn poll(&mut self) -> Vec<Output> {
        let mut outputs = std::mem::take(&mut self.waited);
        outputs.extend(self.outputs.try_iter());
        outputs
    }

    /// Blocks until the server has handled every command sent so far (a
    /// capture's settle ticks, each answered before the next), keeping the
    /// outputs for the next [`Self::poll`].
    pub fn wait_idle(&mut self) {
        while self.waited.last().is_none_or(|out| out.handled < self.sent) {
            match self.outputs.recv() {
                Ok(out) => self.waited.push(out),
                Err(_) => break,
            }
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        // Closing the channel ends the loop.
        self.commands = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn make_stack(recipes: &minecraftoss_player::crafting::RecipeBook, item: &str, count: i32, components: Option<&str>) -> minecraftoss_player::inventory::ItemStack {
    let mut stack = minecraftoss_player::inventory::ItemStack::new(item, count.clamp(0, 255) as u8);
    stack.components = components.and_then(|c| serde_json::from_str(c).ok());
    stack.max = stack.max.min(recipes.max_stack(item));
    stack
}

fn server_loop(mut sim: ServerSim, commands: std::sync::mpsc::Receiver<Command>, outputs: std::sync::mpsc::Sender<Output>) {
    let mut handled = 0u64;
    while let Ok(first) = commands.recv() {
        let mut out = Output::default();
        // Everything already queued is handled before answering.
        let mut next = Some(first);
        while let Some(command) = next.take().or_else(|| commands.try_recv().ok()) {
            handled += 1;
            match command {
                Command::LoadChunk(chunk) => sim.load_chunk(&chunk),
                Command::UnloadChunk(pos) => sim.unload_chunk(pos),
                Command::PlayerEdit { pos, block, edit } => sim.player_edit_block(pos, block.as_ref(), edit),
                Command::PlayerBreaks(positions) => sim.player_breaks(&positions),
                Command::UseBlock { pos, facing } => {
                    sim.use_block(pos, facing);
                }
                Command::BoneMeal { pos, face } => {
                    if sim.bone_meal(pos, face) {
                        out.bone_meal_used.push(pos);
                    }
                }
                Command::Attack(pos) => {
                    sim.attack_block(pos);
                }
                Command::ItemOnBlock { pos, action } => {
                    out.item_uses.push(sim.item_on_block(pos, action));
                }
                Command::SpawnItem { item, count, components, position, velocity, pickup_delay, age } => {
                    sim.spawn_item(&item, count, components.as_deref(), position, velocity, pickup_delay, age);
                }
                Command::KeepDroppedContents { pos, item, components } => sim.keep_dropped_contents(pos, &item, &components),
                Command::RecipeBook(recipes) => sim.set_recipe_book(recipes),
                Command::LootTables { jar, seed } => sim.load_loot(&jar, seed),
                Command::SummonOrb { position, value } => sim.summon_orb(position, value),
                Command::AwardExperience { position, amount } => sim.award_experience(position, amount),
                Command::Summon { kind, position, nbt, y_rot } => {
                    out.summoned.push(sim.summon(&kind, position, nbt.as_ref(), y_rot).map(|()| kind));
                }
                Command::HatchEgg { kind, position, y_rot } => {
                    out.summoned.push(sim.hatch_egg(&kind, position, y_rot).map(|()| kind));
                }
                Command::MobAction { hit, attack, inventory, selected, infinite } => {
                    out.mob_results.push(sim.mob_action(hit, attack, *inventory, selected, infinite));
                }
                Command::MobBlast { hit, damage, center, impact, player } => sim.mob_blast(hit, damage, center, impact, player),
                Command::Merchant { op, inventory, selected, feet } => {
                    out.merchant.push(sim.merchant(op, *inventory, selected, feet));
                }
                Command::Throw(thrown) => sim.throw(*thrown),
                Command::Tick(input) => {
                    let started = std::time::Instant::now();
                    let input = *input;
                    // `MinecraftServer.autoSave`: every five minutes.
                    sim.ticks_since_save += 1;
                    if sim.ticks_since_save >= 6000 {
                        sim.ticks_since_save = 0;
                        sim.save_all_entities();
                    }
                    sim.set_time(input.day_ticks);
                    sim.set_players(&input.players, input.difficulty);
                    // Living, non-spectating players draw experience orbs.
                    sim.level.living_players = input.mob_players.iter().filter(|p| p.alive && !p.spectator).map(|p| (p.position.to_array(), f64::from(p.eye_height))).collect();
                    sim.set_simulation_area(input.simulation_center, input.simulation_distance);
                    if let Some(natural) = &mut sim.level.natural_spawning {
                        natural.spawn_mobs = input.spawn_mobs;
                    }
                    sim.prepare_spawning(&input.mob_players);
                    sim.tick();
                    let mobs_started = std::time::Instant::now();
                    sim.take_spawned();
                    sim.despawn_mobs(&input.mob_players, input.difficulty);
                    // The player's fight memory: its `tickCount` moves on and
                    // the hurts the client took land in it.
                    sim.mobs.tick_player(0);
                    for &(source, kind) in &input.player_hurts {
                        sim.mobs.player_hurt(0, source, kind);
                    }
                    sim.mobs.set_player_views(input.mob_views.clone());
                    sim.mobs.set_player_vitals(input.mob_vitals.clone());
                    // What the player holds, as villagers see it
                    // (`ShowTradesToPlayer`).
                    let held = input.pickup.as_ref().and_then(|(_, inventory, selected)| inventory.slots.get(*selected).and_then(Option::as_ref).map(|s| s.id.clone()));
                    sim.mobs.set_player_main_hand(0, held.as_deref());
                    sim.tick_mobs(&input.mob_players, input.bright_outside);
                    sim.tick_thrown(input.players.first().copied());
                    out.pearl_landings.extend(sim.take_pearl_landings());
                    sim.spawn_trade_experience();
                    out.merchant.extend(sim.check_merchant(&input.mob_players));
                    out.player_hits.extend(sim.mobs.take_player_hits());
                    out.player_splashes.extend(sim.mobs.take_player_splashes());
                    out.potion_breaks.extend(sim.mobs.take_potion_breaks());
                    out.explosions.extend(sim.take_explosions());
                    out.mob_sounds.extend(sim.mobs.take_sounds());
                    sim.level.last_tick_phases[5] += mobs_started.elapsed().as_secs_f64() * 1000.0;
                    out.mobs = Some(Box::new(sim.tracked_mobs(input.tracking.0, input.tracking.1)));
                    out.weather = Some(sim.weather(input.players.first().copied()));
                    let pickup_feet = input.pickup.as_ref().map(|(feet, _, _)| *feet);
                    if let Some((feet, mut inventory, selected)) = input.pickup {
                        let recipes = inventory.recipes.clone();
                        let picked = sim.pickup(feet, |item, count, components| {
                            let stack = make_stack(&recipes, item, count, components);
                            match inventory.add_item(stack, selected) {
                                None => count,
                                Some(rest) => count - i32::from(rest.count),
                            }
                        });
                        out.picked.extend(picked);
                    }
                    // After the items, one experience orb.
                    out.orbs_taken.extend(sim.take_experience(pickup_feet));
                    out.entities = Some(EntitySnapshot { items: sim.items(), tnt: sim.primed_tnt(), falling: sim.falling_blocks(), orbs: sim.orbs(), thrown: sim.thrown_view() });
                    let (solves, ms) = sim.level.light_solves.replace((0, 0.0));
                    out.tick_phases = Some((sim.level.last_tick_phases, (solves, ms), started.elapsed().as_secs_f64() * 1000.0));
                }
            }
        }
        out.changes = sim.take_changes();
        out.destroyed = sim.take_destroyed();
        out.handled = handled;
        if outputs.send(out).is_err() {
            break;
        }
    }
    // The client closed the world: its entities are saved.
    sim.save_all_entities();
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_core::Registries;
    use minecraftoss_generator::terrain::TerrainGenerator;
    use minecraftoss_world::chunk_map::ChunkMap;

    /// A block the player places reaches the level, which runs the block's
    /// behaviour and reports the blocks it changed back to the scene.
    #[test]
    fn placed_water_flows_through_the_level() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        // A water source in the air above the terrain.
        let pos = (8, 200, 8);
        scene.set(pos, Some(Block::new("minecraft:water")));
        server.player_edit(&scene, pos, PlayerEdit::Place);
        let placed = server.take_changes();
        assert_eq!(placed.len(), 1, "{placed:?}");
        let mut flowed = 0;
        for _ in 0..10 {
            server.tick();
            flowed += server.take_changes().len();
        }
        assert!(flowed > 0, "water should fall from the source");
    }

    #[test]
    fn rain_falls_on_a_player_under_the_open_sky() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in -1..=1 {
            for z in -1..=1 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        assert!(!server.set_weather("snow"));
        assert!(server.set_weather("thunder"));
        for _ in 0..120 {
            server.tick();
        }
        let surface: Vec<[f64; 3]> = (-16..32)
            .flat_map(|x| (-16..32).map(move |z| (x, z)))
            .map(|(x, z)| {
                let top = minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::HeightmapKind::MotionBlocking, x, z);
                [f64::from(x) + 0.5, f64::from(top), f64::from(z) + 0.5]
            })
            .collect();
        let stormy = server.weather(None);
        assert!(stormy.rain_level > 0.99 && stormy.thunder_level > 0.99 && !stormy.player_in_rain, "{stormy:?}");
        assert!(surface.iter().any(|&feet| server.weather(Some(feet)).player_in_rain), "rain reaches the surface");
        assert!(surface.iter().all(|&feet| !server.weather(Some([feet[0], feet[1] - 12.0, feet[2]])).player_in_rain), "no rain under the ground");
        assert!(server.set_weather("clear"));
        for _ in 0..120 {
            server.tick();
        }
        assert_eq!(server.weather(None), WeatherView::default());
        assert!(surface.iter().all(|&feet| !server.weather(Some(feet)).player_in_rain));
    }

    /// Creatures the SPAWN step generated join the server's mobs with their
    /// chunk, once per session, and wander while inside the ticking area.
    #[test]
    fn generated_mobs_join_the_server_and_wander() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        // Plains around chunk (20, 128), a SPAWN probe with creatures.
        let mut generated = 0;
        for x in 15..=25 {
            for z in 123..=133 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                generated += chunk.generation.entities.iter().filter(|t| t.get("id").and_then(minecraftoss_core::nbt::Tag::as_str).is_some_and(|id| ["minecraft:cow", "minecraft:pig", "minecraft:chicken", "minecraft:sheep"].contains(&id))).count();
                server.load_chunk(&chunk);
            }
        }
        assert!(generated > 0, "seed 1234 generates animals there");
        let animals = |world: &minecraftoss_entities::world::EntityWorld| world.cows().iter().filter(|c| c.horse.is_none()).count() + world.pigs().len() + world.chickens().len() + world.sheep().len();
        assert_eq!(animals(&server.mobs), generated, "every generated animal joins");
        // Loading a chunk again does not add its creatures twice.
        server.load_chunk(&map.load_now(ChunkPos::new(20, 128)));
        assert_eq!(animals(&server.mobs), generated);
        let start: Vec<glam::DVec3> = server.mobs.cows().iter().map(|c| c.cow.body.position).chain(server.mobs.sheep().iter().map(|s| s.body.position)).collect();
        server.set_simulation_area((20, 128), 5);
        for _ in 0..400 {
            server.tick();
            server.tick_mobs(&[], true);
        }
        let end: Vec<glam::DVec3> = server.mobs.cows().iter().map(|c| c.cow.body.position).chain(server.mobs.sheep().iter().map(|s| s.body.position)).collect();
        let moved = start.iter().zip(&end).filter(|(a, b)| a.distance(**b) > 0.5).count();
        assert!(moved > 0, "some animal strolls within 20 seconds: {start:?} -> {end:?}");
        assert!(end.iter().all(|p| p.y > 40.0), "animals stay on the ground: {end:?}");
        // The client tracks the ones near its player.
        let near = server.tracked_mobs([328.0, 80.0, 2056.0], 32.0);
        assert!(animals(&near) <= animals(&server.mobs));
    }

    /// Generated animals wander about a player among them, climbing the
    /// terrain's steps (`jumpFromGround`); around a spectator they idle
    /// once `noActionTime` passes 100, as `Mob.checkDespawn` only resets it
    /// near a player it counts.
    #[test]
    fn idle_animals_wander_near_a_player() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 15..=25 {
            for z in 123..=133 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        // Among the first animals generated.
        let herd = server.mobs.cows().iter().map(|e| e.cow.body.position).chain(server.mobs.sheep().iter().map(|e| e.body.position)).next().expect("animals");
        let mut player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: herd + glam::DVec3::new(3.0, 4.0, 0.0),
            eye_height: 1.62,
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
            alive: true,
            spectator: false,
            attackable: true,
        };
        let mut share = Vec::new();
        for spectator in [false, true] {
            player.spectator = spectator;
            let (mut moving, mut samples) = (0, 0);
            let mut last: std::collections::HashMap<u64, glam::DVec3> = Default::default();
            for tick in 0..1200 {
                server.set_players(&[player.position.to_array()], 2);
                server.set_simulation_area((20, 128), 5);
                server.tick();
                server.despawn_mobs(&[player], 2);
                server.tick_mobs(&[player], true);
                let w = &server.mobs;
                let animals = w.cows().iter().map(|e| (e.id, e.cow.body.position)).chain(w.sheep().iter().map(|e| (e.id, e.body.position)));
                for (id, p) in animals {
                    if p.distance(player.position) > 32.0 {
                        continue;
                    }
                    if let Some(old) = last.insert(id, p) {
                        if tick >= 200 {
                            samples += 1;
                            moving += usize::from(glam::DVec2::new(old.x - p.x, old.z - p.z).length() > 0.01);
                        }
                    }
                }
            }
            share.push(moving as f64 / samples.max(1) as f64);
        }
        assert!(share[0] > 0.15, "animals walk about a player: {share:?}");
        assert!(share[1] < 0.01, "and idle about a spectator: {share:?}");
    }

    /// Mobs leave with their chunk into the entity storage and come back
    /// from it as they were: where they wandered, hurt, sheared, and one
    /// killed stays dead, not respawned from generation.
    #[test]
    fn mobs_are_saved_with_their_chunks() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let dir = std::env::temp_dir().join(format!("minecraftoss-entity-save-{}", std::process::id()));
        let storage = Arc::new(minecraftoss_world::storage::ChunkStorage::new(&dir, "minecraft:overworld", registries.clone(), -64, 384));
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.set_entity_storage(Some(storage));
        let chunks: Vec<std::sync::Arc<Chunk>> = (15..=25).flat_map(|x| (123..=133).map(move |z| (x, z))).map(|(x, z)| map.load_now(ChunkPos::new(x, z))).collect();
        for chunk in &chunks {
            server.load_chunk(chunk);
        }
        server.set_simulation_area((20, 128), 5);
        for _ in 0..100 {
            server.tick();
            server.tick_mobs(&[], true);
        }
        let animals = |world: &minecraftoss_entities::world::EntityWorld| world.cows().iter().filter(|c| c.horse.is_none()).count() + world.pigs().len() + world.chickens().len() + world.sheep().len();
        let before = animals(&server.mobs);
        assert!(before > 1, "seed 1234 has animals there");
        // Hurt one animal and kill another.
        let hurt_id = server.mobs.mob_ids().next().unwrap();
        let victim = server.mobs.mob_ids().nth(1).unwrap();
        let hurt_at = server.mobs.body_mut(hurt_id).unwrap().position;
        for (id, amount) in [(hurt_id, 1.0), (victim, 100.0)] {
            if let Some(e) = server.mobs.cow_mut(id) { e.hurt(amount, minecraftoss_entities::world::DamageSourceKind::Generic); }
            if let Some(e) = server.mobs.sheep_mut(id) { e.hurt(amount, minecraftoss_entities::world::DamageSourceKind::Generic); }
            if let Some(e) = server.mobs.pig_mut(id) { e.hurt(amount, minecraftoss_entities::world::DamageSourceKind::Generic); }
            if let Some(e) = server.mobs.chicken_mut(id) { e.hurt(amount); }
        }
        let positions: Vec<glam::DVec3> = server.mobs.cows().iter().map(|c| c.cow.body.position).collect();
        for chunk in &chunks {
            server.unload_chunk(chunk.pos);
        }
        assert_eq!(animals(&server.mobs), 0, "unloading takes the mobs out");
        for chunk in &chunks {
            server.load_chunk(chunk);
        }
        assert_eq!(animals(&server.mobs), before - 1, "they come back without the one killed");
        let back: Vec<glam::DVec3> = server.mobs.cows().iter().map(|c| c.cow.body.position).collect();
        assert!(positions.iter().all(|p| back.iter().any(|b| b.distance(*p) < 1.0e-9)), "where they were: {positions:?} -> {back:?}");
        let health = [server.mobs.cows().iter().map(|c| (c.cow.body.position, c.cow.health)).collect::<Vec<_>>(), server.mobs.sheep().iter().map(|s| (s.body.position, s.health)).collect(), server.mobs.pigs().iter().map(|p| (p.pig.body.position, p.pig.health)).collect(), server.mobs.chickens().iter().map(|c| (c.chicken.body.position, c.chicken.health)).collect()].concat();
        let (_, restored) = health.iter().find(|(p, _)| p.distance(hurt_at) < 2.0).copied().expect("the hurt animal is back");
        assert!(restored < 10.0, "hurt as it was: {restored}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// At midnight on Normal, natural spawning fills the dark around a
    /// player with monsters up to the cap, and they despawn once the player
    /// is far away.
    #[test]
    fn night_spawns_monsters_around_the_player() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 11..=29 {
            for z in 119..=137 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, 328, 2056);
        let mut player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: glam::DVec3::new(328.5, f64::from(surface), 2056.5),
            eye_height: 1.62,
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
            alive: true,
            spectator: false,
            attackable: true,
        };
        let monsters = |server: &ServerSim| {
            let census = server.mobs.census();
            // Dormant monsters count; generated minecarts and horses wait
            // dormant too.
            let monster = |kind: &str| minecraftoss_world::natural_spawner::spawning::MobCategory::of_type(kind) == Some(minecraftoss_world::natural_spawner::spawning::MobCategory::Monster);
            census.iter().filter(|m| ["minecraft:zombie", "minecraft:skeleton"].contains(&m.kind)).count() + server.dormant.iter().filter(|(g, _)| monster(&g[0].kind)).count()
        };
        server.set_time(18000);
        for _ in 0..40 {
            server.set_players(&[player.position.to_array()], 2);
            server.set_simulation_area((20, 128), 5);
            server.prepare_spawning(&[player]);
            server.tick();
            server.take_spawned();
            server.despawn_mobs(&[player], 2);
            server.tick_mobs(&[player], false);
        }
        let spawned = monsters(&server);
        assert!(spawned > 20, "the night brings monsters: {spawned}");
        // The cap is 70 for the 289 chunks around one player, overshot at
        // most by one tick's spawning.
        assert!(spawned <= 70 + 40, "monsters stay near the cap: {spawned}");
        assert!(server.mobs.zombies().iter().all(|z| z.zombie.body.position.distance(player.position) <= 132.0 + 40.0));
        // Far away, they despawn.
        player.position.x += 3000.0;
        server.prepare_spawning(&[player]);
        server.despawn_mobs(&[player], 2);
        assert_eq!(monsters(&server), 0, "monsters beyond 128 blocks despawn");
    }

    /// `/summon`: a mob made without NBT is finalized as a fresh spawn and
    /// joins the entity world; NBT is loaded over the fresh mob; monsters
    /// are refused in peaceful.
    /// A summoned villager's brain walks it about at the time of day it
    /// idles in.
    #[test]
    fn villagers_wander_by_day() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, 328, 2056);
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        server.summon("minecraft:villager", at, None, 0.0).unwrap();
        let start = server.mobs.villagers()[0].villager.body.position;
        let mut farthest = 0.0_f64;
        for _ in 0..600 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            let villager = &server.mobs.villagers()[0];
            assert!(villager.villager.health > 0.0, "it lives");
            farthest = farthest.max(villager.villager.body.position.distance(start));
        }
        assert!(farthest > 1.0, "it wandered ({farthest})");
        // At night a homeless villager rests: it heads for (any) village.
        server.set_time(13000);
        for _ in 0..40 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
        }
        let brain = &server.mobs.villagers()[0].ai.as_ref().unwrap().brain;
        assert!(brain.activities.active.contains(&minecraftoss_entities::villager_brain::Activity::Rest), "it rests at night");
    }

    /// A summoned iron golem strolls about (no village near: any spot),
    /// and comes back from its saved tag cracked, built by a player or not.
    #[test]
    fn iron_golems_stroll_and_keep_their_tags() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, 328, 2056);
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let nbt = minecraftoss_core::snbt::parse_compound("{PlayerCreated:1b}").unwrap();
        server.summon("minecraft:iron_golem", at, Some(&nbt), 0.0).unwrap();
        let start = server.mobs.iron_golems()[0].golem.body.position;
        let mut farthest = 0.0_f64;
        for _ in 0..2000 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            let golem = &server.mobs.iron_golems()[0];
            assert!(golem.golem.health > 0.0, "it lives");
            farthest = farthest.max(golem.golem.body.position.distance(start));
        }
        assert!(farthest > 1.0, "it strolled ({farthest})");
        let id = server.mobs.iron_golems()[0].id;
        server.mobs.iron_golem_mut(id).unwrap().hurt(55.0);
        let tags = crate::server_mobs::mob_tags(&server.mobs, |_| true, &std::collections::HashMap::new());
        let (_, tag) = tags.iter().find(|(tag_id, _)| *tag_id == id).expect("the golem is saved");
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        crate::server_mobs::spawn_saved(&mut loaded, tag).expect("golems load");
        let golem = &loaded.iron_golems()[0].golem;
        assert_eq!(golem.health, 45.0);
        assert!(golem.player_created, "built by a player");
        assert_eq!(golem.crackiness(), minecraftoss_entities::iron_golem::Crackiness::Medium);
    }

    /// A summoned wolf wanders about, and comes back from its saved tag
    /// with its variant, voice, collar, owner and sitting order.
    #[test]
    fn wolves_wander_and_keep_their_tags() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, 328, 2056);
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let nbt = minecraftoss_core::snbt::parse_compound("{variant:\"minecraft:woods\",sound_variant:\"minecraft:big\"}").unwrap();
        server.summon("minecraft:wolf", at, Some(&nbt), 0.0).unwrap();
        let start = server.mobs.wolves()[0].wolf.body.position;
        let mut farthest = 0.0_f64;
        for _ in 0..1200 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            let wolf = &server.mobs.wolves()[0];
            assert!(wolf.wolf.health > 0.0, "it lives");
            farthest = farthest.max(wolf.wolf.body.position.distance(start));
        }
        assert!(farthest > 1.0, "it wandered ({farthest})");
        let id = server.mobs.wolves()[0].id;
        {
            // Tamed by someone and told to sit, in a blue collar.
            let wolf = &mut server.mobs.wolf_mut(id).unwrap().wolf;
            wolf.owner = Some(0x1234_5678_9abc_def0_0fed_cba9_8765_4321);
            wolf.set_tame(true, true);
            wolf.ordered_to_sit = true;
            wolf.collar = 11;
        }
        let tags = crate::server_mobs::mob_tags(&server.mobs, |_| true, &std::collections::HashMap::new());
        let (_, tag) = tags.iter().find(|(tag_id, _)| *tag_id == id).expect("the wolf is saved");
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        crate::server_mobs::spawn_saved(&mut loaded, tag).expect("wolves load");
        let wolf = &loaded.wolves()[0].wolf;
        assert_eq!((wolf.variant.as_str(), wolf.sound_variant.as_str()), ("woods", "big"));
        assert_eq!(wolf.max_health(), 40.0, "taming's health stays");
        assert_eq!(wolf.collar, 11);
        assert_eq!(wolf.owner, Some(0x1234_5678_9abc_def0_0fed_cba9_8765_4321));
        assert!(wolf.tame && wolf.ordered_to_sit && wolf.sitting, "tame and sitting");
    }

    /// A librarian makes its offers from the data JAR's trade sets when
    /// they are first needed; they are saved with it (`Offers`, typed as
    /// vanilla writes them) and come back unchanged, while a villager that
    /// never made any saves none. Its gossip and UUID come back too.
    #[test]
    fn villagers_keep_their_offers_and_gossip() {
        use minecraftoss_core::nbt::Tag;
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let jar = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/.gradle/loom-cache/minecraftMaven/net/minecraft/minecraft-common-1fad6b3808/26.3/minecraft-common-1fad6b3808-26.3.jar");
        if !jar.exists() || registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_loot(&jar, 1234);
        let nbt = minecraftoss_core::snbt::parse_compound("{NoAI:1b,VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:librarian\",level:3}}").unwrap();
        server.summon("minecraft:villager", [0.5, 100.0, 0.5], Some(&nbt), 0.0).unwrap();
        server.summon("minecraft:villager", [4.5, 100.0, 0.5], Some(&nbt), 0.0).unwrap();
        let (id, idle) = (server.mobs.villagers()[0].id, server.mobs.villagers()[1].id);
        let offers = server.mobs.villager_offers(id).expect("a villager").to_vec();
        let about = server.mobs.uuid_of(idle);
        server.mobs.villager_hurt_by(id, idle);
        std::sync::Arc::make_mut(&mut server.mobs.villager_mut(id).unwrap().gossips).add(7, minecraftoss_entities::gossip::GossipType::Trading, 12);
        server.mobs.villager_mut(id).unwrap().last_gossip_decay = 1234;
        let gossip = server.mobs.villager_mut(id).unwrap().gossips.unpack();
        assert_eq!(gossip.len(), 2);
        // `updateTrades`: the two of its current level's set only.
        assert_eq!(offers.len(), 2, "{offers:?}");
        let tags = crate::server_mobs::mob_tags(&server.mobs, |_| true, &std::collections::HashMap::new());
        let (_, tag) = tags.iter().find(|(tag_id, _)| *tag_id == id).expect("the villager is saved");
        let recipes = tag.get("Offers").and_then(|o| o.get("Recipes")).and_then(Tag::as_list).expect("its offers are saved");
        assert_eq!(recipes.len(), 2);
        assert!(recipes.iter().all(|r| matches!(r.get("priceMultiplier"), Some(Tag::Float(_)))), "price multipliers are floats");
        assert!(matches!(tag.get("LastRestock"), Some(Tag::Long(0))));
        let (_, idle_tag) = tags.iter().find(|(tag_id, _)| *tag_id == idle).unwrap();
        assert!(idle_tag.get("Offers").is_none(), "no offers made, none saved");
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        let back = crate::server_mobs::spawn_saved(&mut loaded, tag).expect("villagers load");
        assert_eq!(loaded.villager_mut(back).unwrap().offers.as_deref(), Some(&offers[..]));
        assert_eq!(loaded.villager_mut(back).unwrap().gossips.unpack(), gossip);
        assert_eq!(loaded.villager_mut(back).unwrap().last_gossip_decay, 1234);
        assert_eq!(loaded.uuid_of(back), server.mobs.uuid_of(id), "it keeps its UUID");
        assert!(gossip.iter().any(|g| g.0 == about), "gossip about the other villager");
    }

    /// Using a farmer opens its trades; picking one moves the wheat in,
    /// shift-clicking the result trades until the wheat runs short (levelling
    /// the farmer up and dropping experience), and closing gives the rest
    /// back.
    #[test]
    fn players_trade_with_villagers() {
        use minecraftoss_player::inventory::{Inventory, ItemStack};
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let jar = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/.gradle/loom-cache/minecraftMaven/net/minecraft/minecraft-common-1fad6b3808/26.3/minecraft-common-1fad6b3808-26.3.jar");
        if !jar.exists() || registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_loot(&jar, 1234);
        let nbt = minecraftoss_core::snbt::parse_compound(
            "{NoAI:1b,Xp:8,VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:1},Offers:{Recipes:[{buy:{id:\"minecraft:wheat\",count:20},sell:{id:\"minecraft:emerald\",count:1},maxUses:16,xp:2,priceMultiplier:0.05f}]}}",
        )
        .unwrap();
        server.summon("minecraft:villager", [0.5, 100.0, 0.5], Some(&nbt), 0.0).unwrap();
        let id = server.mobs.villagers()[0].id;
        let mut inventory = Inventory::default();
        let mut wheat = ItemStack::new("minecraft:wheat", 50);
        wheat.max = 64;
        inventory.slots[9] = Some(wheat);
        let result = server.mob_action(minecraftoss_entities::world::MobHit::Villager(id), None, inventory.clone(), 0, false);
        let view = result.merchant.expect("the trading screen opens");
        assert_eq!((view.profession, view.level, view.offers.len()), ("minecraft:farmer", 1, 1));
        let apply = |inventory: &mut Inventory, update: &MerchantUpdate| {
            for (slot, stack) in &update.slots {
                inventory.slots[*slot] = stack.clone();
            }
            inventory.cursor = update.cursor.clone();
        };
        let update = server.merchant(MerchantOp::Select(0), inventory.clone(), 0, [0.0; 3]);
        apply(&mut inventory, &update);
        let view = update.view.expect("still open");
        assert_eq!(view.payment[0].as_ref().map(|s| s.count), Some(50), "the wheat moved in");
        assert_eq!(view.result.as_ref().map(|s| s.id.as_str()), Some("minecraft:emerald"));
        let update = server.merchant(MerchantOp::Result { shift: true }, inventory.clone(), 0, [0.0; 3]);
        apply(&mut inventory, &update);
        let view = update.view.expect("still open");
        assert_eq!(view.payment[0].as_ref().map(|s| s.count), Some(10), "two trades of twenty");
        assert_eq!(inventory.count("minecraft:emerald"), 2);
        assert_eq!((view.level, view.xp), (2, 12), "the first trade levelled the farmer up");
        assert!(view.offers.len() > 1, "with its next level's offers");
        assert!(server.level.experience_total() > 0, "trades dropped experience");
        let update = server.merchant(MerchantOp::Close, inventory.clone(), 0, [0.0; 3]);
        apply(&mut inventory, &update);
        assert!(update.view.is_none(), "closed");
        assert_eq!(inventory.count("minecraft:wheat"), 10, "the rest came back");
        assert!(server.mobs.villagers()[0].trading_player.is_none());
    }

    /// A villager claims the bed and bell placed near it by day, and sleeps
    /// in the bed at night: the level's blocks feed the points of interest.
    #[test]
    fn villagers_claim_beds_and_sleep() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, x, z);
        let surface = height(&server, 328, 2056);
        let place = |server: &mut ServerSim, x: i32, z: i32, block: Block| {
            let y = height(server, x, z);
            let state = server.state_of(Some(&block));
            server.level.set_block_and_update(minecraftoss_core::BlockPos::new(x, y, z), state);
            (x, y, z)
        };
        let bell = place(&mut server, 330, 2056, Block::new("minecraft:bell"));
        let bed = |part: &str| Block::new("minecraft:red_bed").with("facing", "east").with("part", part).with("occupied", "false");
        place(&mut server, 326, 2058, bed("foot"));
        let head = place(&mut server, 327, 2058, bed("head"));
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        server.summon("minecraft:villager", at, None, 0.0).unwrap();
        let tick = |server: &mut ServerSim| {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
        };
        for _ in 0..200 {
            tick(&mut server);
        }
        let memories = &server.mobs.villagers()[0].ai.as_ref().unwrap().brain.memories;
        assert_eq!(memories.home.get().copied(), Some(head), "it claims the bed");
        assert_eq!(memories.meeting_point.get().copied(), Some(bell), "it claims the bell");
        assert_eq!(server.mobs.pois.record(head).map(|r| r.free_tickets), Some(0));
        server.set_time(12500);
        let mut slept = false;
        for _ in 0..600 {
            tick(&mut server);
            slept |= server.mobs.villagers()[0].sleeping == Some(head);
        }
        assert!(slept, "it sleeps in its bed at night");
        // A lectern nearby by day: it becomes a librarian and works there.
        let lectern = place(&mut server, 331, 2054, Block::new("minecraft:lectern"));
        server.set_time(1800);
        let mut worked = false;
        for _ in 0..800 {
            tick(&mut server);
            worked |= server.mobs.villagers()[0].ai.as_ref().unwrap().brain.running().iter().any(|b| b == "RunOne:WorkAtPoi");
        }
        let villager = &server.mobs.villagers()[0];
        assert_eq!(villager.villager.profession, minecraftoss_entities::villager::Profession::Librarian, "it takes the lectern's profession");
        assert_eq!(villager.ai.as_ref().unwrap().brain.memories.job_site.get().copied(), Some(lectern));
        assert!(worked, "it works at its lectern");
    }

    /// A farmer takes the composter by its field, harvests the ripe wheat
    /// (the level drops its loot, which the farmer picks up) and sows the
    /// bare farmland again.
    #[test]
    fn farmers_harvest_and_sow() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, x, z);
        let ground = height(&server, 328, 2056);
        let set = |server: &mut ServerSim, (x, y, z): (i32, i32, i32), block: Block| {
            let state = server.state_of(Some(&block));
            server.level.set_block_and_update(minecraftoss_core::BlockPos::new(x, y, z), state);
        };
        // A flat stone yard with a three-by-three field of ripe wheat beside
        // a composter.
        for x in 322..=334 {
            for z in 2050..=2062 {
                for y in ground..ground + 4 {
                    set(&mut server, (x, y, z), Block::new("minecraft:air"));
                }
                set(&mut server, (x, ground - 1, z), Block::new("minecraft:stone"));
            }
        }
        let mut field = Vec::new();
        for x in 328..=330 {
            for z in 2055..=2057 {
                set(&mut server, (x, ground - 1, z), Block::new("minecraft:farmland").with("moisture", "7"));
                set(&mut server, (x, ground, z), Block::new("minecraft:wheat").with("age", "7"));
                field.push((x, ground, z));
            }
        }
        set(&mut server, (327, ground, 2056), Block::new("minecraft:composter").with("level", "0"));
        let at = [326.5, f64::from(ground), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(3000);
        let farmer = minecraftoss_core::snbt::parse_compound(
            "{CanPickUpLoot:1b,Xp:1,VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:1},Inventory:[{id:\"minecraft:wheat_seeds\",count:4}]}",
        )
        .unwrap();
        server.summon("minecraft:villager", at, Some(&farmer), 0.0).unwrap();
        let age = |server: &ServerSim, (x, y, z): (i32, i32, i32)| {
            let state = server.level.block(minecraftoss_core::BlockPos::new(x, y, z));
            let blocks = &server.level.registries().blocks;
            (blocks.block(blocks.block_of(state)).name.as_str().to_owned(), blocks.property(state, "age").map(str::to_owned))
        };
        let mut sown = false;
        for _ in 0..2400 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            sown |= field.iter().any(|&pos| age(&server, pos) == ("minecraft:wheat".to_owned(), Some("0".to_owned())));
            if sown {
                break;
            }
        }
        let villager = &server.mobs.villagers()[0];
        assert!(villager.ai.as_ref().unwrap().brain.memories.job_site.get().is_some(), "it takes the composter");
        assert!(sown, "it harvested ripe wheat and sowed the farmland again");
        assert!(field.iter().any(|&pos| age(&server, pos).1.as_deref() != Some("7")), "a ripe crop was harvested");
    }

    /// A villager with plenty of bread throws some to one with none as they
    /// meet, and the hungry one picks it up from the level.
    #[test]
    fn villagers_share_food() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, x, z);
        let at = [328.5, f64::from(height(&server, 328, 2056)), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let plenty = minecraftoss_core::snbt::parse_compound("{CanPickUpLoot:1b,Inventory:[{id:\"minecraft:bread\",count:30}]}").unwrap();
        let none = minecraftoss_core::snbt::parse_compound("{CanPickUpLoot:1b}").unwrap();
        server.summon("minecraft:villager", at, Some(&plenty), 0.0).unwrap();
        server.summon("minecraft:villager", [at[0] + 2.0, at[1], at[2] + 1.0], Some(&none), 90.0).unwrap();
        let hungry = server.mobs.villagers()[1].id;
        let fed = |server: &ServerSim| server.mobs.villagers().iter().find(|v| v.id == hungry).unwrap().inventory.count("minecraft:bread");
        for _ in 0..1200 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            if fed(&server) > 0 {
                break;
            }
        }
        assert_eq!(fed(&server), 6, "six loaves thrown (thirty less twenty-four) and picked up");
        assert_eq!(server.mobs.villagers()[0].inventory.count("minecraft:bread"), 24);
    }

    /// Two villagers with bread near free beds have a baby, which takes a
    /// bed; they eat their bread and rest from breeding.
    #[test]
    fn villagers_breed_into_a_free_bed() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, x, z);
        let place = |server: &mut ServerSim, x: i32, z: i32, block: Block| {
            let y = height(server, x, z);
            let state = server.state_of(Some(&block));
            server.level.set_block_and_update(minecraftoss_core::BlockPos::new(x, y, z), state);
            (x, y, z)
        };
        let bed = |part: &str| Block::new("minecraft:red_bed").with("facing", "east").with("part", part).with("occupied", "false");
        let mut beds = Vec::new();
        for z in [2052, 2055, 2058] {
            place(&mut server, 326, z, bed("foot"));
            beds.push(place(&mut server, 327, z, bed("head")));
        }
        let at = [329.5, f64::from(height(&server, 329, 2055)), 2055.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let fed = minecraftoss_core::snbt::parse_compound("{CanPickUpLoot:1b,Inventory:[{id:\"minecraft:bread\",count:3}]}").unwrap();
        server.summon("minecraft:villager", at, Some(&fed), 0.0).unwrap();
        server.summon("minecraft:villager", [at[0] + 2.0, at[1], at[2] + 1.0], Some(&fed), 90.0).unwrap();
        let parents: Vec<u64> = server.mobs.villagers().iter().map(|v| v.id).collect();
        for _ in 0..2400 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            if server.mobs.villagers().len() > 2 {
                break;
            }
        }
        let villagers = server.mobs.villagers();
        let baby = villagers.iter().find(|v| !parents.contains(&v.id)).expect("the villagers had a baby");
        assert!(baby.villager.age.baby());
        let home = baby.ai.as_ref().unwrap().brain.memories.home.get().copied();
        assert!(home.is_some_and(|h| beds.contains(&h)), "the baby has a bed: {home:?}");
        for parent in villagers.iter().filter(|v| parents.contains(&v.id)) {
            assert!(parent.villager.age.ticks > 0, "the parents rest from breeding");
            assert_eq!(parent.inventory.food_points(), 0, "the parents ate their bread");
        }
    }

    /// A villager that can pick things up walks to bread lying near it and
    /// takes it; one summoned without `CanPickUpLoot` leaves its bread be,
    /// and the flag survives saving.
    #[test]
    fn villagers_pick_up_food() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, x, z);
        let at = [328.5, f64::from(height(&server, 328, 2056)), 2056.5];
        let other = [334.5, f64::from(height(&server, 334, 2062)), 2062.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let loot = minecraftoss_core::snbt::parse_compound("{CanPickUpLoot:1b}").unwrap();
        server.summon("minecraft:villager", at, Some(&loot), 0.0).unwrap();
        server.summon("minecraft:villager", other, None, 0.0).unwrap();
        let (picker, idle) = (server.mobs.villagers()[0].id, server.mobs.villagers()[1].id);
        let bread = |count| minecraftoss_world::level::container::Stack::new("minecraft:bread", count);
        let near = [at[0] + 2.0, f64::from(height(&server, 330, 2056)), at[2]];
        let kept = server.level.spawn_item_with([other[0] + 0.5, other[1], other[2]], bread(2), [0.0; 3], 0, 0);
        let taken = server.level.spawn_item_with(near, bread(3), [0.0; 3], 0, 0);
        for _ in 0..200 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
        }
        let villager = |id| server.mobs.villagers().iter().find(|v| v.id == id).unwrap();
        assert_eq!(villager(picker).inventory.food_points(), 12, "it took the three loaves");
        assert!(server.level.item_entity(taken).is_none(), "the bread is gone");
        assert!(!villager(idle).can_pick_up_loot, "a summoned villager is told whether it may");
        assert_eq!(villager(idle).inventory.food_points(), 0);
        assert_eq!(server.level.item_entity(kept).map(|(_, stack, _)| stack.count), Some(2), "its bread lies where it was");
        let tags = crate::server_mobs::mob_tags(&server.mobs, |_| true, &std::collections::HashMap::new());
        let (_, tag) = tags.iter().find(|(tag_id, _)| *tag_id == picker).unwrap();
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        let back = crate::server_mobs::spawn_saved(&mut loaded, tag).unwrap();
        let back = loaded.villager_mut(back).unwrap();
        assert!(back.can_pick_up_loot);
        assert_eq!(back.inventory.food_points(), 12, "its inventory is saved");
    }

    #[test]
    fn summon_makes_mobs() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(&server.level, minecraftoss_core::chunk::HeightmapKind::MotionBlocking, 328, 2056);
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        let before = server.mobs.zombies().len();
        server.summon("minecraft:husk", at, None, 1.5).unwrap();
        let nbt = minecraftoss_core::snbt::parse_compound(r#"{IsBaby:1b,PersistenceRequired:1b,VillagerData:{type:"minecraft:desert",profession:"minecraft:farmer",level:1}}"#).unwrap();
        server.summon("minecraft:zombie_villager", [at[0] + 2.0, at[1], at[2]], Some(&nbt), 0.0).unwrap();
        let zombies = server.mobs.zombies();
        assert_eq!(zombies.len(), before + 2, "both join the entity world");
        let husk = zombies.iter().find(|z| z.zombie.kind == minecraftoss_entities::zombie::ZombieKind::Husk).unwrap();
        assert_eq!(husk.zombie.body.position.to_array(), at, "at the command's position");
        let villager = zombies.iter().find(|z| z.zombie.kind == minecraftoss_entities::zombie::ZombieKind::ZombieVillager).unwrap();
        assert!(villager.zombie.baby && villager.zombie.persistence_required, "the NBT is loaded");
        assert_eq!(villager.zombie.villager, Some(("minecraft:desert".to_owned(), "minecraft:farmer".to_owned())));
        server.summon("minecraft:cow", at, None, 0.0).unwrap();
        assert_eq!(server.mobs.cows().len(), 1, "animals are summoned too");
        // A villager loads without its brain (not ported) instead of
        // tripping the entity world's assertion.
        server.summon("minecraft:villager", at, None, 0.0).unwrap();
        assert_eq!(server.mobs.villagers().len(), 1);
        // `NoAI` holds a mob still, turned to its saved yaw.
        let still = minecraftoss_core::snbt::parse_compound("{NoAI:1b,Rotation:[180f,0f]}").unwrap();
        server.summon("minecraft:zombie", [at[0] - 2.0, at[1], at[2]], Some(&still), 0.0).unwrap();
        let zombie = server.mobs.zombies().iter().find(|z| z.no_ai).expect("a NoAI zombie");
        assert_eq!((zombie.yaw, zombie.body_rotation.body_yaw, zombie.look_control.head_yaw), (180.0, 180.0, 180.0));
        let id = zombie.id;
        for _ in 0..20 {
            server.tick_mobs(&[], false);
        }
        let zombie = server.mobs.zombies().iter().find(|z| z.id == id).unwrap();
        assert_eq!(zombie.zombie.body.position.to_array(), [at[0] - 2.0, at[1], at[2]], "it stays put");
        // A horse loads its variant and grazing (`EatingHaystack`), which
        // lasts fifty ticks though it has no AI.
        let grazing = minecraftoss_core::snbt::parse_compound("{NoAI:1b,Variant:1027,EatingHaystack:1b,Age:-24000}").unwrap();
        server.summon("minecraft:horse", [at[0] + 4.0, at[1], at[2]], Some(&grazing), 0.0).unwrap();
        let horse = server.mobs.cows().iter().find_map(|c| c.horse.clone()).expect("a horse");
        assert_eq!((horse.variant, horse.eating), (1027, true));
        for _ in 0..40 {
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], false);
        }
        let horse = server.mobs.cows().iter().find_map(|c| c.horse.clone()).unwrap();
        assert!(horse.eating && horse.eat_anim == 1.0, "{horse:?}");
        server.set_players(&[at], 0);
        assert!(server.summon("minecraft:zombie", at, None, 0.0).is_err(), "no monsters in peaceful");
        assert!(server.summon("minecraft:cow", at, None, 0.0).is_ok(), "animals come in peaceful");
    }

    /// A creeper sees the player across a platform, walks up, swells and
    /// blows up: the blast breaks the planks (mobs may grief), hurts and
    /// pushes the player, and the client hears of it.
    #[test]
    fn creeper_stalks_and_blows_up_the_player() {
        use minecraftoss_core::nbt::Tag;
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in -1..=2 {
            for z in -1..=1 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        // A plank platform high above the terrain.
        for x in 0..=24 {
            for z in 0..=12 {
                server.player_edit_block((x, 200, z), Some(&Block::new("minecraft:oak_planks")), PlayerEdit::Place);
            }
        }
        server.take_changes();
        let player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: glam::DVec3::new(18.5, 201.0, 6.5),
            eye_height: 1.62,
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
            alive: true,
            spectator: false,
            attackable: true,
        };
        let mut tag = std::collections::BTreeMap::new();
        tag.insert("id".to_owned(), Tag::String("minecraft:creeper".to_owned()));
        tag.insert("Pos".to_owned(), Tag::List(vec![Tag::Double(8.5), Tag::Double(201.0), Tag::Double(6.5)]));
        tag.insert("Rotation".to_owned(), Tag::List(vec![Tag::Float(0.0), Tag::Float(0.0)]));
        tag.insert("UUID".to_owned(), Tag::IntArray(vec![1, 2, 3, 4]));
        let id = crate::server_mobs::spawn_saved(&mut server.mobs, &Tag::Compound(tag)).expect("creepers are simulated");
        assert!(server.mobs.creepers().iter().any(|c| c.id == id && !c.no_ai));
        server.set_time(18000);
        let (mut hits, mut blasts, mut broken) = (Vec::new(), Vec::new(), 0);
        for _ in 0..400 {
            server.set_players(&[player.position.to_array()], 2);
            server.set_simulation_area((0, 0), 4);
            server.tick();
            server.tick_mobs(&[player], false);
            hits.extend(server.mobs.take_player_hits());
            blasts.extend(server.take_explosions());
            broken += server.take_changes().iter().filter(|(pos, block)| pos.1 == 200 && block.is_none()).count();
            if !blasts.is_empty() {
                break;
            }
        }
        assert_eq!(blasts.len(), 1, "the creeper explodes");
        assert!(server.mobs.creepers().iter().all(|c| c.id != id), "and is gone");
        let blast = blasts[0];
        assert!(blast.position.distance(player.position) < 3.5, "beside the player: {:?}", blast.position);
        assert!(broken > 0, "the blast breaks planks");
        let hit = hits.iter().find(|h| matches!(h.kind, minecraftoss_entities::world::PlayerHitKind::Explosion { .. })).expect("the blast hits the player");
        assert!(hit.damage > 10.0, "a close blast hurts: {}", hit.damage);
    }

    /// A player's wheat makes a generated cow fall in love (using up the
    /// wheat), and hits kill it, dropping its loot into the level.
    #[test]
    fn players_feed_and_hit_server_mobs() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let jar = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/.gradle/loom-cache/minecraftMaven/net/minecraft/minecraft-common-1fad6b3808/26.3/minecraft-common-1fad6b3808-26.3.jar");
        if !jar.exists() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 4, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_loot(&jar, 1234);
        let recipes = Arc::new(minecraftoss_player::crafting::RecipeBook::from_jar(&jar).unwrap());
        server.set_recipe_book(recipes.clone());
        for x in 20..=23 {
            for z in 129..=132 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        server.set_simulation_area((21, 130), 4);
        let cow = server.mobs.cows().iter().find(|c| !c.cow.age.baby()).map(|c| c.id).expect("an adult cow near (356, 65, 2097)");
        let hit = minecraftoss_entities::world::MobHit::Cow(cow);
        let mut inventory = minecraftoss_player::inventory::Inventory::default();
        inventory.recipes = recipes;
        inventory.slots[0] = Some(minecraftoss_player::inventory::ItemStack::new("minecraft:wheat", 3));
        let fed = server.mob_action(hit, None, inventory.clone(), 0, false);
        assert_eq!(fed.slots.len(), 1, "one wheat is used: {:?}", fed.slots);
        assert_eq!(fed.slots[0].1.as_ref().map(|s| s.count), Some(2));
        assert!(server.mobs.cow_mut(cow).unwrap().cow.in_love > 0, "the cow falls in love");
        // Full-strength bare-handed hits, one point each past each hurt
        // cooldown, until it dies.
        let items_before = server.items().len();
        let mut sounds = Vec::new();
        for _ in 0..40 {
            let at = server.mobs.cows().iter().find(|c| c.id == cow).map(|c| c.cow.body.position);
            let Some(at) = at else { break };
            let attack = minecraftoss_entities::world::PlayerAttack {
                player_id: 0,
                position: at + glam::DVec3::X,
                yaw: 90.0,
                attack_damage: 1.0,
                strength: 1.0,
                sprinting: false,
                can_critical: false,
                can_sweep: false,
            };
            let result = server.mob_action(hit, Some(attack), inventory.clone(), 0, false);
            sounds.extend(result.sounds.into_iter().map(|s| s.event).filter(|s| s.starts_with("entity.cow")));
            for _ in 0..11 {
                server.tick_mobs(&[], true);
            }
            if sounds.last().is_some_and(|s| s.ends_with("death")) {
                break;
            }
        }
        assert!(sounds.first().is_some_and(|s| s.ends_with(".hurt")), "{sounds:?}");
        assert!(sounds.last().is_some_and(|s| s.ends_with(".death")), "the cow dies: {sounds:?}");
        assert!(server.items().len() > items_before, "its loot drops into the level");
    }

    /// Five experience land as orbs of vanilla's sizes (3, 1 and 1); the
    /// orbs are drawn to a player standing beside them, who takes one every
    /// other tick until none are left.
    #[test]
    fn experience_orbs_are_awarded_and_taken() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        // A floating platform, high above the terrain.
        for x in 2..=14 {
            for z in 2..=14 {
                scene.set((x, 200, z), Some(Block::new("minecraft:stone")));
                server.player_edit(&scene, (x, 200, z), PlayerEdit::Place);
            }
        }
        server.award_experience([8.5, 201.0, 8.5], 5);
        let mut values: Vec<i32> = server.orbs().iter().map(|o| o.value).collect();
        values.sort_unstable();
        assert_eq!(values, [1, 1, 3]);
        let feet = [10.5, 201.0, 8.5];
        server.level.living_players = vec![(feet, 1.62)];
        let mut taken = Vec::new();
        for tick in 0..60 {
            server.tick();
            if let Some((_, _, value)) = server.take_experience(Some(feet)) {
                taken.push((tick, value));
            }
        }
        assert_eq!(taken.iter().map(|(_, v)| v).sum::<i32>(), 5, "every orb is taken: {taken:?}");
        assert!(taken.windows(2).all(|w| w[1].0 - w[0].0 >= 2), "one orb every other tick at most: {taken:?}");
        assert!(server.orbs().is_empty());
    }

    #[test]
    fn a_players_blast_kill_leaves_experience_and_a_bots_does_not() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        let zombie = |server: &mut ServerSim, x: f64| server.mobs.spawn_zombie(minecraftoss_entities::zombie::Zombie::new(glam::DVec3::new(x, 100.0, 0.5)), true);
        let bots = zombie(&mut server, 0.5);
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(bots), 40.0, [0.5, 100.0, -1.5], 1.0, None);
        assert!(server.mobs.zombies().iter().all(|z| z.id != bots || z.zombie.health <= 0.0));
        assert!(server.orbs().is_empty());
        let mine = zombie(&mut server, 8.5);
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(mine), 40.0, [8.5, 100.0, -1.5], 1.0, Some(0));
        assert_eq!(server.orbs().iter().map(|o| o.value).sum::<i32>(), 5);
    }

    #[test]
    fn broken_ore_experience_lands_as_an_orb_the_player_takes() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        for x in 2..=14 {
            for z in 2..=14 {
                scene.set((x, 200, z), Some(Block::new("minecraft:stone")));
                server.player_edit(&scene, (x, 200, z), PlayerEdit::Place);
            }
        }
        let ore = Block::new("minecraft:diamond_ore");
        scene.set((8, 201, 8), Some(ore.clone()));
        server.player_edit(&scene, (8, 201, 8), PlayerEdit::Place);
        let at = minecraftoss_core::BlockPos::new(8, 201, 8);
        assert!(!server.level.registries().blocks.is_air(server.level.block(at)));
        server.player_breaks(&[(8, 201, 8)]);
        assert!(server.level.registries().blocks.is_air(server.level.block(at)));
        let amount = crate::experience::block_experience(&ore, true, |bound| bound - 1);
        assert_eq!(amount, 7);
        server.award_experience([8.5, 201.5, 8.5], amount as i32);
        assert_eq!(server.orbs().iter().map(|o| (o.value, o.position[1])).collect::<Vec<_>>(), [(7, 201.5)]);
        assert!(!server.level.unsupported.iter().any(|u| u.contains("experience orb")), "{:?}", server.level.unsupported);
        let feet = [10.5, 201.0, 8.5];
        server.level.living_players = vec![(feet, 1.62)];
        let mut taken = 0;
        for _ in 0..60 {
            server.tick();
            if let Some((_, _, value)) = server.take_experience(Some(feet)) {
                taken += value;
            }
        }
        assert_eq!(taken, 7);
        assert!(server.orbs().is_empty());
    }

    /// `cargo test --release -p minecraftoss-viewer --lib mob_tick_cost -- --ignored --nocapture`
    #[test]
    #[ignore = "timing measurement"]
    fn mob_tick_cost() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 10, 16);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 10..=30 {
            for z in 118..=138 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        server.set_simulation_area((20, 128), 10);
        let mut times = Vec::new();
        for _ in 0..600 {
            let started = std::time::Instant::now();
            server.tick_mobs(&[], true);
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        let p = |q: usize| times[(times.len() - 1) * q / 100];
        eprintln!("{} mobs: tick p50 {:.2} p95 {:.2} p99 {:.2} max {:.2} ms", server.mobs.len(), p(50), p(95), p(99), times[times.len() - 1]);
    }

    /// TNT primed by a placed redstone block becomes an entity the client
    /// can render, explodes, and leaves items the player can pick up.
    #[test]
    fn primed_tnt_explodes_and_drops_items() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        // A floating platform of planks with TNT on it, high above terrain.
        for x in 4..=12 {
            for z in 4..=12 {
                scene.set((x, 200, z), Some(Block::new("minecraft:oak_planks")));
                server.player_edit(&scene, (x, 200, z), PlayerEdit::Place);
            }
        }
        scene.set((8, 201, 8), Some(Block::new("minecraft:tnt")));
        server.player_edit(&scene, (8, 201, 8), PlayerEdit::Place);
        scene.set((9, 201, 8), Some(Block::new("minecraft:redstone_block")));
        server.player_edit(&scene, (9, 201, 8), PlayerEdit::Place);
        server.take_changes();
        assert_eq!(server.primed_tnt().len(), 1, "the TNT should prime");
        for _ in 0..90 {
            server.tick();
        }
        assert!(server.primed_tnt().is_empty(), "the TNT should have exploded");
        let changes = server.take_changes();
        assert!(changes.iter().any(|(pos, block)| *pos == (8, 200, 8) && block.is_none()), "the blast should break the planks under it");
        let items = server.items();
        assert!(items.iter().any(|i| i.item == "minecraft:oak_planks"), "broken planks drop: {items:?}");
        // Let the drops land, then pick them up standing among them.
        for _ in 0..60 {
            server.tick();
        }
        let item = server.items().into_iter().find(|i| i.pickup_delay == 0).expect("a landed item");
        let mut taken = 0;
        let picked = server.pickup([item.position[0], item.position[1], item.position[2]], |_, count, _| {
            taken += count;
            count
        });
        assert!(!picked.is_empty() && taken > 0, "the player should pick up touching items");
        assert!(server.items().iter().all(|i| i.id != picked[0].0), "a fully picked-up item is removed");
    }

    /// Sand placed in the air falls as an entity and lands on the ground.
    #[test]
    fn placed_sand_falls_and_lands() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        scene.set((8, 200, 8), Some(Block::new("minecraft:stone")));
        server.player_edit(&scene, (8, 200, 8), PlayerEdit::Place);
        scene.set((8, 205, 8), Some(Block::new("minecraft:sand")));
        server.player_edit(&scene, (8, 205, 8), PlayerEdit::Place);
        server.take_changes();
        for _ in 0..3 {
            server.tick();
        }
        let falling = server.falling_blocks();
        assert_eq!(falling.len(), 1, "the sand should be falling");
        assert_eq!((falling[0].block.clone(), falling[0].start), (Block::new("minecraft:sand"), (8, 205, 8)));
        server.tick();
        let later = server.falling_blocks();
        assert_eq!(later.iter().map(|f| f.id).collect::<Vec<_>>(), vec![falling[0].id], "a falling block keeps its id while it falls");
        assert_eq!(later[0].previous_position, falling[0].position);
        for _ in 0..40 {
            server.tick();
        }
        assert!(server.falling_blocks().is_empty(), "the sand should have landed");
        let changes = server.take_changes();
        assert!(changes.iter().any(|(pos, block)| *pos == (8, 201, 8) && block.as_ref().is_some_and(|b| *b == Block::new("minecraft:sand"))), "{changes:?}");
    }

    /// Bone meal the player uses on a crop grows it through the level.
    #[test]
    fn bone_meal_grows_a_crop() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        scene.set((8, 200, 8), Some(Block::new("minecraft:farmland")));
        server.player_edit(&scene, (8, 200, 8), PlayerEdit::Place);
        scene.set((8, 201, 8), Some(Block::new("minecraft:wheat")));
        server.player_edit(&scene, (8, 201, 8), PlayerEdit::Place);
        server.take_changes();
        assert!(server.bone_meal((8, 201, 8), "up"), "wheat takes bone meal");
        let changes = server.take_changes();
        let grown = changes.iter().find(|(pos, _)| *pos == (8, 201, 8)).and_then(|(_, b)| b.clone()).expect("the wheat changed");
        let age: i32 = grown.properties.get("age").and_then(|a| a.parse().ok()).unwrap_or(0);
        assert!((2..=5).contains(&age), "bone meal adds 2 to 5 ages: {age}");
        assert!(!server.bone_meal((8, 200, 8), "up"), "farmland does not take bone meal");
    }

    fn door_world() -> Option<(ServerSim, HandcraftedScene)> {
        let paths = DataPaths::discover().ok()?;
        let registries = Arc::new(Registries::load(&paths).ok()?);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).ok()?)).ok()?);
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).ok()?);
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        scene.set((8, 199, 8), Some(Block::new("minecraft:stone")));
        server.player_edit(&scene, (8, 199, 8), PlayerEdit::Place);
        Some((server, scene))
    }

    fn place_door(server: &mut ServerSim, scene: &mut HandcraftedScene, id: &str) -> std::collections::HashMap<BlockPos, Option<Block>> {
        for (y, half) in [(200, "lower"), (201, "upper")] {
            let block = Block::new(id).with("facing", "north").with("half", half).with("hinge", "left").with("open", "false").with("powered", "false");
            scene.set((8, y, 8), Some(block));
            server.player_edit(scene, (8, y, 8), PlayerEdit::Place);
        }
        server.take_changes().into_iter().collect()
    }

    fn open(changes: &std::collections::HashMap<BlockPos, Option<Block>>, pos: BlockPos) -> Option<bool> {
        changes.get(&pos)?.as_ref().map(|b| b.properties.get("open").map(String::as_str) == Some("true"))
    }

    #[test]
    fn a_door_opens_both_halves_and_breaking_one_takes_both() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place_door(&mut server, &mut scene, "minecraft:oak_door");
        assert!(server.use_block((8, 200, 8), "north"));
        let opened: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(open(&opened, (8, 200, 8)), Some(true));
        assert_eq!(open(&opened, (8, 201, 8)), Some(true));
        scene.set((8, 201, 8), None);
        server.player_edit(&scene, (8, 201, 8), PlayerEdit::Break);
        let broken: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(broken.get(&(8, 200, 8)), Some(&None));
    }

    fn place(server: &mut ServerSim, scene: &mut HandcraftedScene, pos: BlockPos, block: Block) {
        scene.set(pos, Some(block));
        server.player_edit(scene, pos, PlayerEdit::Place);
    }

    fn take(server: &mut ServerSim, scene: &mut HandcraftedScene, pos: BlockPos) {
        scene.set(pos, None);
        server.player_edit(scene, pos, PlayerEdit::Break);
    }

    fn ticked(server: &mut ServerSim, ticks: usize) -> std::collections::HashMap<BlockPos, Option<Block>> {
        let mut changes = std::collections::HashMap::new();
        for _ in 0..ticks {
            server.tick();
            changes.extend(server.take_changes());
        }
        changes
    }

    #[test]
    fn columns_break_block_by_block_when_their_ground_goes() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:sand"));
        place(&mut server, &mut scene, (9, 200, 8), Block::new("minecraft:water").with("level", "0"));
        place(&mut server, &mut scene, (9, 199, 8), Block::new("minecraft:stone"));
        for y in 201..=203 {
            place(&mut server, &mut scene, (8, y, 8), Block::new("minecraft:sugar_cane"));
        }
        place(&mut server, &mut scene, (6, 199, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (6, 200, 8), Block::new("minecraft:dirt"));
        for y in 201..=202 {
            place(&mut server, &mut scene, (6, y, 8), Block::new("minecraft:bamboo"));
        }
        server.take_changes();
        take(&mut server, &mut scene, (8, 200, 8));
        take(&mut server, &mut scene, (6, 200, 8));
        let after_one = ticked(&mut server, 1);
        assert_eq!(after_one.get(&(8, 201, 8)), Some(&None), "{after_one:?}");
        assert!(!after_one.contains_key(&(8, 203, 8)), "a column breaks one block a tick");
        let changes = ticked(&mut server, 6);
        for y in 202..=203 {
            assert_eq!(changes.get(&(8, y, 8)), Some(&None), "sugar cane at {y}");
        }
        assert_eq!(changes.get(&(6, 202, 8)), Some(&None), "bamboo");
        let items: Vec<String> = server.items().into_iter().map(|i| i.item).collect();
        assert!(items.iter().any(|i| i == "minecraft:sugar_cane"), "{items:?}");
        assert!(items.iter().any(|i| i == "minecraft:bamboo"), "{items:?}");
    }

    #[test]
    fn scaffolding_reaches_six_out_and_falls_or_breaks_beyond() {
        let Some((mut server, mut scene)) = door_world() else { return };
        let mut latest = std::collections::HashMap::new();
        for x in 8..=15 {
            place(&mut server, &mut scene, (x, 200, 8), Block::new("minecraft:scaffolding"));
            latest.extend(server.take_changes());
            latest.extend(ticked(&mut server, 2));
        }
        let distance = |x: i32| latest.get(&(x, 200, 8)).cloned().flatten().and_then(|b| b.properties.get("distance").cloned());
        assert_eq!(distance(8).as_deref(), Some("0"));
        assert_eq!(distance(14).as_deref(), Some("6"));
        assert_eq!(latest.get(&(15, 200, 8)), Some(&None), "the seventh out leaves its place");
        let falling = server.falling_blocks();
        assert_eq!(falling.len(), 1, "unsupported scaffolding falls");
        assert_eq!(falling[0].block.properties.get("distance").map(String::as_str), Some("7"), "the falling block keeps its state");
        take(&mut server, &mut scene, (8, 199, 8));
        let changes = ticked(&mut server, 12);
        for x in 8..=14 {
            assert_eq!(changes.get(&(x, 200, 8)), Some(&None), "scaffolding at {x}");
        }
        assert!(server.items().iter().any(|i| i.item == "minecraft:scaffolding"));
    }

    #[test]
    fn hanging_signs_and_torches_drop_with_their_support() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 205, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (8, 204, 8), Block::new("minecraft:oak_hanging_sign").with("rotation", "0").with("attached", "false").with("waterlogged", "false"));
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:torch"));
        server.take_changes();
        take(&mut server, &mut scene, (8, 205, 8));
        take(&mut server, &mut scene, (8, 199, 8));
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(changes.get(&(8, 204, 8)), Some(&None), "{changes:?}");
        assert_eq!(changes.get(&(8, 200, 8)), Some(&None), "{changes:?}");
        let items: Vec<String> = server.items().into_iter().map(|i| i.item).collect();
        assert!(items.iter().any(|i| i == "minecraft:oak_hanging_sign"), "{items:?}");
        assert!(items.iter().any(|i| i == "minecraft:torch"), "{items:?}");
    }

    #[test]
    fn a_batch_of_player_breaks_drops_nothing_twice() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:torch"));
        place(&mut server, &mut scene, (9, 199, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (9, 200, 8), Block::new("minecraft:torch"));
        server.take_changes();
        server.take_destroyed();
        server.player_breaks(&[(8, 199, 8), (8, 200, 8), (9, 199, 8)]);
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        for pos in [(8, 199, 8), (8, 200, 8), (9, 199, 8), (9, 200, 8)] {
            assert_eq!(changes.get(&pos), Some(&None), "{pos:?}");
        }
        let items: Vec<String> = server.items().into_iter().map(|i| i.item).collect();
        assert_eq!(items, vec!["minecraft:torch".to_owned()], "only the torch outside the batch drops on the server");
        let destroyed: Vec<_> = server.take_destroyed().into_iter().map(|(pos, block)| (pos, block.id.key())).collect();
        assert_eq!(destroyed, vec![((9, 200, 8), "minecraft:torch".to_owned())]);
    }

    #[test]
    fn an_iron_door_ignores_hands_but_opens_on_redstone() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place_door(&mut server, &mut scene, "minecraft:iron_door");
        assert!(!server.use_block((8, 200, 8), "north"));
        scene.set((9, 200, 8), Some(Block::new("minecraft:redstone_block")));
        server.player_edit(&scene, (9, 200, 8), PlayerEdit::Place);
        let powered: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(open(&powered, (8, 200, 8)), Some(true));
        assert_eq!(open(&powered, (8, 201, 8)), Some(true));
    }

    fn thrown_down(item: &str, seed: u64) -> crate::thrown::Thrown {
        let mut random = minecraftoss_player::rng::LegacyRandom::new(seed);
        crate::thrown::Thrown::throw(item, glam::DVec3::new(8.5, 203.0, 8.5), 0.0, 90.0, glam::DVec3::ZERO, true, &mut random).expect("a throwable")
    }

    #[test]
    fn a_pearl_reports_where_the_player_lands() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw(thrown_down("minecraft:ender_pearl", 3));
        let mut landings = Vec::new();
        for _ in 0..20 {
            server.tick_thrown(Some([40.0, 250.0, 40.0]));
            landings.extend(server.take_pearl_landings());
        }
        assert_eq!(landings.len(), 1);
        assert!(landings[0][1] >= 200.0 && landings[0][1] < 202.0, "{landings:?}");
        assert!(server.thrown_view().is_empty());
    }

    #[test]
    fn a_pearl_vanishes_when_its_thrower_is_dead() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw(thrown_down("minecraft:ender_pearl", 3));
        server.throw(thrown_down("minecraft:snowball", 4));
        server.tick_thrown(None);
        let left: Vec<String> = server.thrown_view().into_iter().map(|t| t.0).collect();
        assert_eq!(left, vec!["minecraft:snowball".to_owned()]);
        assert!(server.take_pearl_landings().is_empty());
    }

    #[test]
    fn eggs_now_and_then_hatch_baby_chickens() {
        let Some((mut server, _)) = door_world() else { return };
        let before = server.mobs.chickens().len();
        for seed in 0..64 {
            server.throw(thrown_down("minecraft:brown_egg", seed));
        }
        for _ in 0..20 {
            server.tick_thrown(None);
        }
        let chickens: Vec<_> = server.mobs.chickens().iter().skip(before).collect();
        assert!(!chickens.is_empty());
        assert!(chickens.iter().all(|c| c.chicken.age.ticks < 0));
    }

    #[test]
    fn a_door_placed_beside_power_opens_at_once() {
        let Some((mut server, mut scene)) = door_world() else { return };
        scene.set((9, 200, 8), Some(Block::new("minecraft:redstone_block")));
        server.player_edit(&scene, (9, 200, 8), PlayerEdit::Place);
        server.take_changes();
        let placed = place_door(&mut server, &mut scene, "minecraft:iron_door");
        assert_eq!(open(&placed, (8, 200, 8)), Some(true));
        assert_eq!(open(&placed, (8, 201, 8)), Some(true));
    }

    fn block_at(changes: &std::collections::HashMap<BlockPos, Option<Block>>, pos: BlockPos) -> Option<Block> {
        changes.get(&pos).cloned().flatten()
    }

    fn property(block: &Option<Block>, name: &str) -> Option<String> {
        block.as_ref().and_then(|b| b.properties.get(name).cloned())
    }

    #[test]
    fn a_poured_bucket_flows_and_an_empty_one_takes_the_source_back() {
        let Some((mut server, _)) = door_world() else { return };
        server.take_changes();
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::EmptyBucket { water: true }));
        let poured = ticked(&mut server, 1);
        let source = block_at(&poured, (8, 200, 8));
        assert_eq!(source.as_ref().map(|b| b.id.key()), Some("minecraft:water".to_owned()));
        assert_eq!(property(&source, "level").as_deref(), Some("0"));
        let flowed = ticked(&mut server, 20);
        assert!(flowed.keys().any(|pos| *pos != (8, 200, 8)), "the source should spread");
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::FillBucket));
        assert_eq!(server.take_changes().into_iter().find(|(pos, _)| *pos == (8, 200, 8)), Some(((8, 200, 8), None)));
        assert!(!server.item_on_block((8, 200, 8), ItemOnBlock::FillBucket), "nothing is left to take");
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::EmptyBucket { water: false }));
        let hardened = server.take_changes().into_iter().find(|(pos, _)| *pos == (8, 200, 8)).and_then(|(_, b)| b);
        assert_eq!(hardened.map(|b| b.id.key()), Some("minecraft:obsidian".to_owned()), "lava poured beside the flowing water hardens");
    }

    #[test]
    fn water_from_a_bucket_douses_and_waterlogs_a_lit_campfire() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:campfire").with("lit", "true").with("waterlogged", "false"));
        server.take_changes();
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::EmptyBucket { water: true }));
        let doused: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        let campfire = block_at(&doused, (8, 200, 8));
        assert_eq!(property(&campfire, "lit").as_deref(), Some("false"));
        assert_eq!(property(&campfire, "waterlogged").as_deref(), Some("true"));
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::FillBucket));
        let drained: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(property(&block_at(&drained, (8, 200, 8)), "waterlogged").as_deref(), Some("false"));
    }

    #[test]
    fn lava_poured_on_a_torch_knocks_it_off_as_an_item() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:torch"));
        server.take_changes();
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::EmptyBucket { water: false }));
        assert!(server.items().iter().any(|item| item.item == "minecraft:torch"), "{:?}", server.items());
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(block_at(&changes, (8, 200, 8)).map(|b| b.id.key()), Some("minecraft:lava".to_owned()));
    }

    #[test]
    fn flint_fire_needs_ground_turns_to_soul_fire_on_soul_soil_and_burns_out() {
        let Some((mut server, mut scene)) = door_world() else { return };
        server.set_players(&[[8.5, 200.0, 8.5]], 2);
        server.take_changes();
        assert!(!server.item_on_block((8, 205, 8), ItemOnBlock::Ignite), "no fire in mid-air");
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::Ignite));
        let lit: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(block_at(&lit, (8, 200, 8)).map(|b| b.id.key()), Some("minecraft:fire".to_owned()));
        assert!(!server.item_on_block((8, 200, 8), ItemOnBlock::Ignite), "fire only goes into air");
        place(&mut server, &mut scene, (12, 200, 8), Block::new("minecraft:soul_soil"));
        assert!(server.item_on_block((12, 201, 8), ItemOnBlock::Ignite));
        let soul: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(block_at(&soul, (12, 201, 8)).map(|b| b.id.key()), Some("minecraft:soul_fire".to_owned()));
        let burnt = ticked(&mut server, 2400);
        assert!(burnt.contains_key(&(8, 200, 8)) && block_at(&burnt, (8, 200, 8)).is_none(), "fire on bare stone burns out");
        assert!(!burnt.contains_key(&(12, 201, 8)), "soul fire keeps burning");
    }

    fn obsidian_frame(server: &mut ServerSim, scene: &mut HandcraftedScene, along_x: bool, width: i32, height: i32, corners: bool) {
        for i in 0..=width + 1 {
            for y in 0..=height + 1 {
                let edge_side = i == 0 || i == width + 1;
                let edge_end = y == 0 || y == height + 1;
                if (edge_side || edge_end) && (corners || !(edge_side && edge_end)) {
                    let pos = if along_x { (7 + i, 200 + y, 8) } else { (8, 200 + y, 7 + i) };
                    place(server, scene, pos, Block::new("minecraft:obsidian"));
                }
            }
        }
        server.take_changes();
    }

    fn portal_blocks(changes: &std::collections::HashMap<BlockPos, Option<Block>>) -> Vec<(BlockPos, String)> {
        let mut portals: Vec<_> = changes
            .iter()
            .filter_map(|(&pos, block)| block.as_ref().filter(|b| b.id.key() == "minecraft:nether_portal").map(|b| (pos, b.properties["axis"].clone())))
            .collect();
        portals.sort();
        portals
    }

    #[test]
    fn fire_in_an_obsidian_frame_opens_a_portal_even_where_it_could_not_burn() {
        let Some((mut server, mut scene)) = door_world() else { return };
        obsidian_frame(&mut server, &mut scene, true, 2, 3, true);
        assert!(server.item_on_block((8, 202, 8), ItemOnBlock::Ignite), "mid-air fire is allowed where it opens a portal");
        let lit: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        let expected: Vec<_> = [(8, 201, 8), (8, 202, 8), (8, 203, 8), (9, 201, 8), (9, 202, 8), (9, 203, 8)].into_iter().map(|pos| (pos, "x".to_owned())).collect();
        assert_eq!(portal_blocks(&lit), expected);
        assert!(lit.values().flatten().all(|b| b.id.key() != "minecraft:fire"));
        assert!(!server.item_on_block((8, 202, 8), ItemOnBlock::Ignite), "a portal is not air");
        server.take_destroyed();
        take(&mut server, &mut scene, (7, 202, 8));
        let broken: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        for (pos, _) in &expected {
            assert_eq!(broken.get(pos), Some(&None), "{pos:?}");
        }
        let destroyed = server.take_destroyed();
        assert_eq!(destroyed.iter().filter(|(_, b)| b.id.key() == "minecraft:nether_portal").count(), 6, "{destroyed:?}");
    }

    #[test]
    fn a_frame_along_z_without_corners_opens_and_an_open_frame_does_not() {
        let Some((mut server, mut scene)) = door_world() else { return };
        obsidian_frame(&mut server, &mut scene, false, 4, 5, false);
        assert!(server.item_on_block((8, 201, 10), ItemOnBlock::Ignite));
        let lit: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        let portals = portal_blocks(&lit);
        assert_eq!(portals.len(), 20);
        assert!(portals.iter().all(|((x, y, z), axis)| *x == 8 && (201..=205).contains(y) && (8..=11).contains(z) && axis == "z"));
        let Some((mut server, mut scene)) = door_world() else { return };
        obsidian_frame(&mut server, &mut scene, true, 2, 3, true);
        take(&mut server, &mut scene, (8, 204, 8));
        server.take_changes();
        assert!(!server.item_on_block((8, 202, 8), ItemOnBlock::Ignite), "no top, no portal, no mid-air fire");
        assert!(server.item_on_block((8, 201, 8), ItemOnBlock::Ignite));
        let lit: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert!(portal_blocks(&lit).is_empty());
        assert_eq!(block_at(&lit, (8, 201, 8)).map(|b| b.id.key()), Some("minecraft:fire".to_owned()));
    }

    #[test]
    fn flint_primes_tnt_which_blows_up_with_a_bang() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:tnt"));
        server.take_changes();
        assert!(!server.item_on_block((8, 199, 8), ItemOnBlock::PrimeTnt), "stone does not prime");
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::PrimeTnt));
        assert_eq!(server.primed_tnt().len(), 1);
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(changes.get(&(8, 200, 8)), Some(&None));
        for _ in 0..79 {
            server.tick();
        }
        assert!(server.take_explosions().is_empty(), "the fuse lasts 80 ticks");
        server.tick();
        assert!(server.primed_tnt().is_empty());
        assert_eq!(server.take_explosions().len(), 1);
    }

    #[test]
    fn a_spawn_egg_hatches_its_mob_on_the_clicked_slab_and_each_egg_is_answered() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:smooth_stone_slab").with("type", "bottom").with("waterlogged", "false"));
        server.set_players(&[[8.5, 200.0, 11.5]], 2);
        let mut player = minecraftoss_player::Player::new(glam::DVec3::new(8.5, 202.5 - 1.62, 8.5));
        player.pitch = 90.0;
        let aimed = crate::spawn_egg::egg_use(&player, &crate::doors::Outlined(&scene), "minecraft:zombie_spawn_egg");
        let Some(crate::spawn_egg::EggUse::Spawn { kind, position }) = aimed else { panic!("{aimed:?}") };
        assert_eq!((kind.as_str(), position), ("minecraft:zombie", [8.5, 200.5, 8.5]));
        server.hatch_egg(&kind, position, -120.0).unwrap();
        let zombie = server.mobs.zombies().last().expect("a zombie");
        assert_eq!((zombie.zombie.body.position.to_array(), zombie.yaw), (position, -120.0));
        let dormant = server.dormant.len();
        assert!(server.hatch_egg("minecraft:fox", position, 0.0).is_err());
        assert_eq!(server.dormant.len(), dormant);
        let mut handle = ServerHandle::spawn(server);
        handle.hatch_egg("minecraft:cow".to_owned(), position, 45.0);
        handle.hatch_egg("minecraft:not_a_mob".to_owned(), position, 0.0);
        handle.hatch_egg("minecraft:fox".to_owned(), position, 0.0);
        handle.wait_idle();
        let answers: Vec<_> = handle.poll().into_iter().flat_map(|out| out.summoned).collect();
        assert_eq!(answers.len(), 3);
        assert_eq!(answers[0].as_deref(), Ok("minecraft:cow"));
        assert!(answers[1].is_err());
        assert!(answers[2].is_err());
    }

    #[test]
    fn the_server_answers_each_item_use_so_one_source_fills_one_bucket() {
        let Some((mut server, _)) = door_world() else { return };
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::EmptyBucket { water: true }));
        server.take_changes();
        let mut handle = ServerHandle::spawn(server);
        handle.item_on_block((8, 200, 8), ItemOnBlock::FillBucket);
        handle.item_on_block((8, 200, 8), ItemOnBlock::FillBucket);
        handle.wait_idle();
        let outputs = handle.poll();
        let answers: Vec<bool> = outputs.iter().flat_map(|out| out.item_uses.iter().copied()).collect();
        assert_eq!(answers, vec![true, false]);
        let drained = outputs.iter().flat_map(|out| out.changes.iter()).any(|(pos, block)| *pos == (8, 200, 8) && block.is_none());
        assert!(drained, "the answer carries the emptied block");
    }

    fn picked(server: &ServerSim, item: &str) -> i32 {
        server.items().iter().filter(|i| i.item == item).map(|i| i.count).sum()
    }

    #[test]
    fn ripe_bushes_and_laden_cave_vines_are_picked_and_bone_meal_ripens_a_bush() {
        let Some((mut server, mut scene)) = door_world() else { return };
        let bush = |age: &str| Block::new("minecraft:sweet_berry_bush").with("age", age);
        for (age, usable) in [("0", false), ("1", false), ("2", true), ("3", true)] {
            let state = server.states.state_of(&bush(age)).unwrap();
            assert_eq!(server.level.handles_use(state), usable, "age {age}");
        }
        for (berries, usable) in [("true", true), ("false", false)] {
            for vine in ["minecraft:cave_vines", "minecraft:cave_vines_plant"] {
                let state = server.states.state_of(&Block::new(vine).with("berries", berries)).unwrap();
                assert_eq!(server.level.handles_use(state), usable, "{vine} {berries}");
            }
        }
        place(&mut server, &mut scene, (8, 199, 8), Block::new("minecraft:dirt"));
        place(&mut server, &mut scene, (8, 200, 8), bush("1"));
        server.take_changes();
        let age = |server: &mut ServerSim| {
            let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
            property(&block_at(&changes, (8, 200, 8)), "age")
        };
        assert!(!server.use_block((8, 200, 8), "north"), "nothing to pick yet");
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::BoneMeal { face: "up" }));
        assert_eq!(age(&mut server).as_deref(), Some("2"));
        assert!(server.use_block((8, 200, 8), "north"));
        assert_eq!(age(&mut server).as_deref(), Some("1"), "picking leaves the bush at age 1");
        let young = picked(&server, "minecraft:sweet_berries");
        assert!((1..=2).contains(&young), "{young}");
        for _ in 0..2 {
            assert!(server.item_on_block((8, 200, 8), ItemOnBlock::BoneMeal { face: "up" }));
        }
        assert_eq!(age(&mut server).as_deref(), Some("3"));
        assert!(!server.item_on_block((8, 200, 8), ItemOnBlock::BoneMeal { face: "up" }), "a ripe bush takes no bone meal");
        assert!(server.use_block((8, 200, 8), "north"));
        let ripe = picked(&server, "minecraft:sweet_berries") - young;
        assert!((2..=3).contains(&ripe), "{ripe}");
        place(&mut server, &mut scene, (8, 206, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (8, 205, 8), Block::new("minecraft:cave_vines").with("age", "0").with("berries", "true"));
        server.take_changes();
        assert!(server.use_block((8, 205, 8), "north"));
        assert_eq!(picked(&server, "minecraft:glow_berries"), 1);
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(property(&block_at(&changes, (8, 205, 8)), "berries").as_deref(), Some("false"));
        assert!(!server.use_block((8, 205, 8), "north"), "the berries are gone");
        assert!(server.item_on_block((8, 205, 8), ItemOnBlock::BoneMeal { face: "down" }));
        let regrown: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(property(&block_at(&regrown, (8, 205, 8)), "berries").as_deref(), Some("true"));
    }

    fn place_as_client(server: &mut ServerSim, scene: &mut HandcraftedScene, item: &str, feet: [f64; 3], aim: [f64; 3]) -> Option<BlockPos> {
        place_crouching(server, scene, item, feet, aim, false)
    }

    fn place_crouching(server: &mut ServerSim, scene: &mut HandcraftedScene, item: &str, feet: [f64; 3], aim: [f64; 3], crouching: bool) -> Option<BlockPos> {
        let placed = stage_as_client(server, scene, item, feet, aim, crouching)?;
        for _ in 0..3 {
            server.tick();
            for (pos, block) in server.take_changes() {
                scene.set(pos, block);
            }
        }
        Some(placed)
    }

    fn stage_as_client(server: &mut ServerSim, scene: &mut HandcraftedScene, item: &str, feet: [f64; 3], aim: [f64; 3], crouching: bool) -> Option<BlockPos> {
        let mut player = minecraftoss_player::Player::new(glam::DVec3::from_array(feet));
        player.crouching = crouching;
        let eye = player.eye();
        let look = glam::DVec3::from_array(aim) - eye;
        player.yaw = (-look.x).atan2(look.z).to_degrees();
        player.pitch = (-look.y).atan2(look.x.hypot(look.z)).to_degrees();
        let mut inventory = minecraftoss_player::inventory::Inventory::default();
        inventory.slots[0] = Some(minecraftoss_player::inventory::ItemStack::new(item, 1));
        let outlined = crate::doors::Outlined(&*scene);
        let mut staged = crate::placing::Staged::new(&outlined);
        let placed = player.place_selected(&mut staged, &mut inventory, minecraftoss_player::GameMode::Survival)?;
        let writes: Vec<_> = staged.into_writes();
        for (pos, block) in &writes {
            let block = block.as_ref().map(|b| Block { id: crate::pack::ResourceId::parse(&b.id).unwrap(), properties: b.properties.clone() });
            assert!(block.as_ref().is_none_or(|b| server.states.state_of(b).is_some()), "{block:?} has a state");
            scene.set(*pos, block);
        }
        for (pos, _) in &writes {
            server.player_edit(scene, *pos, PlayerEdit::Place);
        }
        Some(placed)
    }

    #[test]
    fn a_torch_clicked_onto_a_wall_hangs_there_and_drops_when_the_wall_goes() {
        let Some((mut server, mut scene)) = door_world() else { return };
        for (x, item, wall_torch) in [(8, "minecraft:torch", "minecraft:wall_torch"), (10, "minecraft:redstone_torch", "minecraft:redstone_wall_torch"), (12, "minecraft:copper_torch", "minecraft:copper_wall_torch")] {
            place(&mut server, &mut scene, (x, 201, 7), Block::new("minecraft:stone"));
            server.take_changes();
            let feet = [x as f64 + 0.5, 200.0, 8.5];
            let placed = place_as_client(&mut server, &mut scene, item, feet, [x as f64 + 0.5, 201.5, 8.0]);
            assert_eq!(placed, Some((x, 201, 8)), "{item} goes into the player's own cell");
            let torch = Scene::block(&scene, (x, 201, 8)).cloned().unwrap();
            assert_eq!(torch.id.key(), wall_torch);
            assert_eq!(torch.properties.get("facing").map(String::as_str), Some("south"));
            let lit = (item == "minecraft:redstone_torch").then_some("true");
            assert_eq!(torch.properties.get("lit").map(String::as_str), lit);
            take(&mut server, &mut scene, (x, 201, 7));
            let mut changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
            changes.extend(ticked(&mut server, 3));
            assert_eq!(changes.get(&(x, 201, 8)), Some(&None), "{item}: {changes:?}");
            assert_eq!(picked(&server, item), 1, "{item} drops itself");
            assert_eq!(picked(&server, wall_torch), 0);
        }
    }

    #[test]
    fn a_torch_needs_a_floor_that_holds_its_centre() {
        let Some((mut server, mut scene)) = door_world() else { return };
        for (x, floor, holds) in [(8, "minecraft:soul_sand", true), (11, "minecraft:oak_leaves", false), (14, "minecraft:glass_pane", true)] {
            place(&mut server, &mut scene, (x, 200, 8), Block::new(floor));
            server.take_changes();
            let placed = stage_as_client(&mut server, &mut scene, "minecraft:torch", [x as f64 + 0.5, 201.0, 10.5], [x as f64 + 0.5, 200.9, 8.5], false);
            assert_eq!(placed.is_some(), holds, "{floor}");
            let standing = Scene::block(&scene, (x, 201, 8)).map(|b| b.id.key());
            assert_eq!(standing.as_deref(), holds.then_some("minecraft:torch"), "{floor}");
        }
    }

    #[test]
    fn a_shulker_box_the_server_blows_up_drops_once_with_its_contents() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 199, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:red_shulker_box").with("facing", "up"));
        server.tick();
        server.take_changes();
        let before = server.items().len();
        server.level.explode(minecraftoss_world::level::explosion::Explosion { center: [8.5, 200.5, 8.5], radius: 2.0, fire: false, interaction: minecraftoss_world::level::explosion::BlockInteraction::DestroyWithDecay, source: None });
        server.tick();
        assert!(server.take_changes().contains(&((8, 200, 8), None)));
        let contents = r#"{"minecraft:container":[{"slot":0,"item":{"id":"minecraft:diamond","count":3}}]}"#;
        server.keep_dropped_contents((8, 200, 8), "minecraft:red_shulker_box", contents);
        let boxes: Vec<_> = server.items().into_iter().filter(|i| i.item == "minecraft:red_shulker_box").collect();
        assert_eq!(boxes.len(), 1, "{boxes:?}");
        assert_eq!(boxes[0].components.as_deref(), Some(contents));
        assert!(server.items().len() > before);
        server.spawn_item("minecraft:red_shulker_box", 1, None, [3.5, 200.5, 3.5], [0.0; 3], 10, 0);
        server.keep_dropped_contents((3, 200, 3), "minecraft:red_shulker_box", contents);
        assert_eq!(server.items().iter().filter(|i| i.item == "minecraft:red_shulker_box" && i.components.as_deref() == Some(contents)).count(), 2);
        assert_eq!(server.items().iter().filter(|i| i.item == "minecraft:red_shulker_box").count(), 2);
    }

    #[test]
    fn a_late_echo_of_the_first_chest_does_not_keep_the_pair_apart() {
        let Some((mut server, mut scene)) = door_world() else { return };
        for x in 5..=8 {
            place(&mut server, &mut scene, (x, 199, 8), Block::new("minecraft:stone"));
        }
        server.tick();
        server.take_changes();
        let first = stage_as_client(&mut server, &mut scene, "minecraft:chest", [6.7, 200.0, 11.5], [6.5, 200.0, 8.5], false);
        server.tick();
        let late = server.take_changes();
        assert!(late.iter().any(|(pos, block)| *pos == (6, 200, 8) && block.as_ref().is_some_and(|b| b.properties.get("type").map(String::as_str) == Some("single"))), "{late:?}");
        let second = stage_as_client(&mut server, &mut scene, "minecraft:chest", [7.2, 200.0, 11.5], [7.5, 200.0, 8.5], false);
        assert_eq!((first, second), (Some((6, 200, 8)), Some((7, 200, 8))));
        for (pos, block) in late {
            scene.set(pos, block);
        }
        for _ in 0..3 {
            server.tick();
            for (pos, block) in server.take_changes() {
                scene.set(pos, block);
            }
        }
        let types: Vec<_> = [(6, 200, 8), (7, 200, 8)].iter().map(|&at| Scene::block(&scene, at).and_then(|b| b.properties.get("type").cloned())).collect();
        assert_eq!(types, vec![Some("right".to_owned()), Some("left".to_owned())]);
    }

    #[test]
    fn chests_placed_side_by_side_join_through_the_server() {
        let Some((mut server, mut scene)) = door_world() else { return };
        for x in 5..=12 {
            for z in 5..=12 {
                place(&mut server, &mut scene, (x, 199, z), Block::new("minecraft:stone"));
            }
        }
        server.take_changes();
        for (id, other, x) in [("minecraft:chest", "minecraft:chest", 6), ("minecraft:trapped_chest", "minecraft:trapped_chest", 8), ("minecraft:weathered_copper_chest", "minecraft:copper_chest", 11)] {
            let first = place_as_client(&mut server, &mut scene, id, [x as f64 + 0.7, 200.0, 11.5], [x as f64 + 0.5, 200.0, 8.5]);
            let second = place_as_client(&mut server, &mut scene, other, [x as f64 + 1.2, 200.0, 11.5], [x as f64 + 1.5, 200.0, 8.5]);
            assert_eq!((first, second), (Some((x, 200, 8)), Some((x + 1, 200, 8))));
            let right = Scene::block(&scene, (x, 200, 8)).cloned().unwrap();
            let left = Scene::block(&scene, (x + 1, 200, 8)).cloned().unwrap();
            assert_eq!(left.properties.get("type").map(String::as_str), Some("left"), "{left:?}");
            assert_eq!(right.properties.get("type").map(String::as_str), Some("right"), "{right:?}");
            let opened = crate::containers::Opened::new((x, 200, 8), &right, crate::containers::chest_partner((x, 200, 8), &right).and_then(|at| Scene::block(&scene, at))).unwrap();
            assert_eq!(opened.size(), 54);
            assert_eq!((left.id.key(), right.id.key()), (other.to_owned(), other.to_owned()));
        }
        place(&mut server, &mut scene, (6, 200, 5), Block::new("minecraft:chest").with("facing", "south").with("type", "single").with("waterlogged", "false"));
        let beside = place_crouching(&mut server, &mut scene, "minecraft:chest", [8.5, 200.0, 5.5], [6.9375, 200.4, 5.5], true);
        assert_eq!(beside, Some((7, 200, 5)), "a crouched click on the side of a chest places against it");
        let joined: Vec<_> = [(7, 200, 5), (6, 200, 5)].iter().map(|&at| Scene::block(&scene, at).map(|b| (b.properties["facing"].clone(), b.properties["type"].clone()))).collect();
        assert_eq!(joined, vec![Some(("south".to_owned(), "left".to_owned())), Some(("south".to_owned(), "right".to_owned()))]);
    }
}
