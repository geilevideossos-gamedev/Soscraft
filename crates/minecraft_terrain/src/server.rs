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
pub enum HowBroken {
    Mined,
    SilkTouched,
    Blasted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemOnBlock {
    EmptyBucket { water: bool },
    FillBucket,
    Ignite,
    PrimeTnt,
    BoneMeal { face: &'static str },
    EnderEye,
    EndCrystal,
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
    take_xp_delay: std::collections::HashMap<u64, i32>,
    /// The client was asked to close its trading screen.
    merchant_closing: std::collections::HashSet<u64>,
    player_feet: std::collections::HashMap<u64, [f64; 3]>,
    thrown: Vec<crate::thrown::Thrown>,
    pearl_landings: Vec<(u64, [f64; 3])>,
    pearl_gateways: Vec<(u64, BlockPos)>,
    block_entity_changes: Vec<(BlockPos, Option<minecraftoss_core::nbt::Tag>)>,
    potion_breaks: Vec<minecraftoss_entities::world::PotionBreak>,
    sound_random: minecraftoss_player::rng::LegacyRandom,
    player_potions: Vec<crate::potions::PlayerPotionHit>,
    clouds: Vec<crate::effect_cloud::EffectCloud>,
    tridents: Vec<crate::trident::ThrownTrident>,
    next_trident: i32,
    players_struck: Vec<crate::thrown::PlayerStruck>,
    players_pushed: Vec<(u64, glam::DVec3)>,
    thrown_hits: Vec<crate::thrown::ThrownHit>,
    worldgen: &'static WorldGen,
    eyes: crate::eye_of_ender::Eyes,
    gateways: crate::end_gateway::Gateways,
    blasts: Vec<(Blast, BlastSource)>,
    block_explosion_drop_decay: bool,
    crystals: crate::end_crystal::Crystals,
    dragon_fight_level: bool,
    dragons: crate::ender_dragon::Dragons,
    fight_dragon: Option<[i32; 4]>,
    alive_crystals: i32,
    crystal_scan_ticks: u32,
    end_random: minecraftoss_player::rng::LegacyRandom,
    end_sounds: Vec<minecraftoss_entities::world::MobSound>,
    end_hits: Vec<minecraftoss_entities::world::PlayerHit>,
    dragons_killed: Vec<[i32; 4]>,
    egg_teleports: Vec<(BlockPos, BlockPos)>,
    fight: FightLink,
    dragon_drops: bool,
    respawn: Option<crate::dragon_respawn::Respawn>,
    fight_reports: Vec<FightReport>,
    dragon_fireballs: crate::dragon_fireball::DragonFireballs,
    end_players: Vec<crate::ender_dragon::PlayerBody>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    pub center: [f64; 3],
    pub power: f32,
    pub fire: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BlastSource {
    Nobody,
    KeepBlocks,
    RespawnPoint { shield: Option<BlockPos> },
    Player(u64),
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
    pub on_fire: bool,
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
    pub thrown_views: Vec<crate::thrown::ThrownView>,
    pub clouds: Vec<crate::effect_cloud::CloudView>,
    pub tridents: Vec<crate::trident::TridentView>,
    pub eyes: crate::eye_of_ender::EyeSnapshot,
    pub crystals: Vec<crate::end_crystal::CrystalView>,
    pub dragons: Vec<crate::ender_dragon::DragonView>,
    pub dragon_fireballs: Vec<crate::dragon_fireball::FireballView>,
    pub breath_bursts: Vec<glam::DVec3>,
    pub gateways: Vec<crate::end_gateway::GatewayView>,
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
    stack.components = components.and_then(crate::item_nbt::components_from_text);
    stack
}

fn components_of(stack: &minecraftoss_core::item::ItemStack) -> Option<String> {
    stack.components.as_ref().map(crate::item_nbt::components_text)
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
        level.sea_level = worldgen.library.generation.sea_level;
        level.fast_lava = level.sky.as_ref().is_some_and(|sky| crate::environment::dimension_value(&sky.environment, "gameplay/fast_lava").as_bool());
        let dragon_fight_level = minecraftoss_core::ident::Identifier::parse(dimension_type).ok().and_then(|id| level.registries().datapack.read_json("dimension_type", &id).ok()).is_some_and(|json| json.get("has_ender_dragon_fight").and_then(|flag| flag.as_bool()).unwrap_or(false));
        let mob_tables = crate::server_mobs::MobTables::new(&level, &states);
        // Natural spawning, when the entity catalog can create mobs.
        if level.registries().entities.is_some() {
            match minecraftoss_world::natural_spawner::CreatureSpawns::load(worldgen.terrain.registries.clone(), dimension_type, false) {
                Ok(spawns) => {
                    let mut natural = minecraftoss_world::level::spawning::NaturalSpawning::new(Arc::new(spawns));
                    natural.structures = Some(minecraftoss_world::level::spawning::structures_of(worldgen));
                    level.natural_spawning = Some(natural);
                }
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
            take_xp_delay: std::collections::HashMap::new(),
            merchant_closing: std::collections::HashSet::new(),
            player_feet: std::collections::HashMap::new(),
            thrown: Vec::new(),
            pearl_landings: Vec::new(),
            pearl_gateways: Vec::new(),
            block_entity_changes: Vec::new(),
            potion_breaks: Vec::new(),
            sound_random: minecraftoss_player::rng::LegacyRandom::new(0x5EED_B0771E),
            player_potions: Vec::new(),
            clouds: Vec::new(),
            tridents: Vec::new(),
            next_trident: TRIDENT_IDS,
            players_struck: Vec::new(),
            players_pushed: Vec::new(),
            thrown_hits: Vec::new(),
            worldgen,
            eyes: crate::eye_of_ender::Eyes::default(),
            gateways: crate::end_gateway::Gateways::default(),
            blasts: Vec::new(),
            block_explosion_drop_decay: true,
            crystals: crate::end_crystal::Crystals::default(),
            dragon_fight_level,
            dragons: crate::ender_dragon::Dragons::default(),
            fight_dragon: None,
            alive_crystals: 0,
            crystal_scan_ticks: 0,
            end_random: minecraftoss_player::rng::LegacyRandom::new(0x00D2_A60E_5EED),
            end_sounds: Vec::new(),
            end_hits: Vec::new(),
            dragons_killed: Vec::new(),
            egg_teleports: Vec::new(),
            fight: FightLink::default(),
            dragon_drops: true,
            respawn: None,
            fight_reports: Vec::new(),
            dragon_fireballs: crate::dragon_fireball::DragonFireballs::default(),
            end_players: Vec::new(),
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
    pub fn mob_action(&mut self, player: u64, hit: minecraftoss_entities::world::MobHit, attack: Option<minecraftoss_entities::world::PlayerAttack>, mut inventory: minecraftoss_player::inventory::Inventory, selected: usize, infinite: bool) -> MobResult {
        let before = inventory.slots.clone();
        let mut actor = crate::mob_actions::Actor {
            player,
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
        MobResult { sounds: outcome.sounds, slots, merchant: self.merchant_view(player) }
    }

    pub fn mob_blast(&mut self, hit: minecraftoss_entities::world::MobHit, damage: f32, center: [f64; 3], impact: f32, player: Option<u64>, slow_ticks: i32) {
        let result = self.mobs.player_blast(player, hit.id(), damage, glam::DVec3::from_array(center), f64::from(impact));
        if slow_ticks > 0 && result.is_some_and(|r| r.applied && !r.died) {
            self.mobs.add_mob_effect(hit.id(), minecraftoss_entities::effects::EffectInstance::new(minecraftoss_entities::effects::MobEffect::Slowness, slow_ticks, STUN_SLOWNESS_AMPLIFIER));
        }
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
    pub fn merchant(&mut self, player: u64, op: MerchantOp, mut inventory: minecraftoss_player::inventory::Inventory, selected: usize, feet: [f64; 3]) -> MerchantUpdate {
        let before = inventory.slots.clone();
        match op {
            MerchantOp::Select(index) => self.mobs.merchant_select(player, index, &mut inventory),
            MerchantOp::Payment { slot, right, shift } => self.mobs.merchant_click_payment(player, slot, right, shift, &mut inventory),
            MerchantOp::Result { shift } => self.mobs.merchant_click_result(player, shift, &mut inventory),
            MerchantOp::Close => {
                for stack in self.mobs.merchant_close(player, &mut inventory, selected) {
                    let components = stack.components.as_ref().map(|c| c.to_string());
                    self.level.spawn_at_location(feet, stack_of(&stack.id, i32::from(stack.count), components.as_deref()));
                }
                self.merchant_closing.remove(&player);
            }
        }
        self.spawn_trade_experience();
        let slots = inventory.slots.iter().zip(&before).enumerate().filter(|(_, (now, was))| now != was).map(|(slot, (now, _))| (slot, now.clone())).collect();
        MerchantUpdate { view: self.merchant_view(player), slots, cursor: inventory.cursor.clone(), closing: false }
    }

    /// `ServerPlayer.tick`'s `stillValid` check on an open trading screen:
    /// once the villager is out of reach, dead or done trading, the client
    /// is asked (once) to close it.
    fn check_merchant(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) -> Vec<(u64, MerchantUpdate)> {
        let mut closing = Vec::new();
        for player in players {
            if self.merchant_closing.contains(&player.id) || self.mobs.merchant_menu(player.id).is_none() {
                continue;
            }
            let eyes = player.position + glam::DVec3::Y * f64::from(player.eye_height);
            if self.mobs.merchant_still_valid(player.id, eyes) {
                continue;
            }
            self.merchant_closing.insert(player.id);
            closing.push((player.id, MerchantUpdate { view: None, slots: Vec::new(), cursor: None, closing: true }));
        }
        closing
    }

    pub fn player_left(&mut self, player: u64) {
        let feet = self.player_feet.remove(&player);
        self.take_xp_delay.remove(&player);
        self.merchant_closing.remove(&player);
        let mut inventory = minecraftoss_player::inventory::Inventory::default();
        let overflow = self.mobs.merchant_close(player, &mut inventory, 0);
        let stacks: Vec<_> = inventory.slots.into_iter().flatten().chain(inventory.cursor).chain(overflow).collect();
        if let Some(feet) = feet {
            for stack in stacks {
                let components = stack.components.as_ref().map(|c| c.to_string());
                self.level.spawn_at_location(feet, stack_of(&stack.id, i32::from(stack.count), components.as_deref()));
            }
        }
        self.spawn_trade_experience();
        self.mobs.forget_player(player);
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
        let mut copy = chunk.clone();
        crate::containers::leave_to_client(&mut copy.block_entities);
        self.level.insert_chunk(copy);
        self.gateways.load_chunk(chunk);
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
            self.gateways.sync((x, y, z), crate::end_gateway::tag_at(&self.level, (x, y, z)));
        }
    }

    /// A saved entity joins the level: items and experience orbs as level
    /// entities, mobs the entity world simulates into it, and the rest
    /// dormant.
    fn add_saved_entity(&mut self, tag: minecraftoss_core::nbt::Tag) {
        self.mobs.set_day_time(self.level.overworld_clock());
        if let Some(trident) = crate::trident::ThrownTrident::from_tag(&tag) {
            self.throw_trident(trident);
            return;
        }
        if crate::server_mobs::load_level_entity(&mut self.level, &tag) {
            return;
        }
        if self.crystals.add(&tag).is_some() || self.dragons.add(&tag).is_some() {
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
        tags.extend(self.tridents.iter().filter(|t| t.alive && inside(t.position)).map(|t| t.to_tag(t.owner_uuid)));
        tags.extend(self.crystals.tags_where(inside));
        tags.extend(self.dragons.tags_where(inside));
        if unload {
            self.tridents.retain(|t| !inside(t.position));
            self.crystals.remove_where(inside);
            self.dragons.remove_where(inside);
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
        self.dormant.retain(|(group, tag)| {
            let root = &group[0];
            let at = glam::DVec3::from_array(root.pos);
            let nearest = feet.iter().map(|p| p.distance_squared(at)).min_by(f64::total_cmp);
            !crate::dormant::despawns(&root.kind, root.persistent, tag, difficulty == 0, nearest)
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
        self.explode_fireballs();
        self.explode_blasts(players);
        self.sound_potion_breaks();
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
    pub fn take_experience(&mut self, player: u64, feet: Option<[f64; 3]>) -> Option<(i32, [f64; 3], i32)> {
        let delay = self.take_xp_delay.entry(player).or_insert(0);
        if *delay > 0 {
            *delay -= 1;
        }
        if *delay != 0 {
            return None;
        }
        let taken = self.level.player_touch_orb(feet?)?;
        self.take_xp_delay.insert(player, 2);
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

    pub fn spawner_egg(&mut self, pos: BlockPos, kind: &str) -> Result<(), String> {
        use minecraftoss_world::natural_spawner::spawning::MobCategory;
        if self.level.difficulty == 0 && MobCategory::of_type(kind) == Some(MobCategory::Monster) {
            return Err(format!("{kind} may not spawn in peaceful"));
        }
        if !self.level.natural_spawning.as_ref().is_none_or(|natural| natural.spawner_blocks_work) {
            return Err("advMode.notEnabled.spawner".to_owned());
        }
        let chunk = self.level.chunk(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2).chunk());
        let tag = chunk.and_then(|chunk| chunk.block_entities.entities.get(&pos));
        if tag.and_then(|tag| tag.get("id")).and_then(minecraftoss_core::nbt::Tag::as_str) != Some("minecraft:mob_spawner") {
            return Err(format!("no monster spawner at {pos:?}"));
        }
        minecraftoss_generator::feature::World::set_spawner_entity(&mut self.level, pos.0, pos.1, pos.2, kind);
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

    pub fn blast(&mut self, blast: Blast) {
        self.blasts.push((blast, BlastSource::Nobody));
    }

    pub fn respawn_point_blast(&mut self, blast: Blast, shield: Option<BlockPos>) {
        self.blasts.push((blast, BlastSource::RespawnPoint { shield }));
    }

    fn explode_fireballs(&mut self) {
        use minecraftoss_world::level::explosion::{BlockInteraction, Explosion};
        for blast in self.mobs.take_fireball_blasts() {
            let interaction = if self.mobs.mob_griefing() { BlockInteraction::DestroyWithDecay } else { BlockInteraction::Keep };
            self.level.explode(Explosion { center: blast.position.to_array(), radius: blast.power, fire: blast.fire, interaction, source: None });
            self.explosions.push(minecraftoss_entities::creeper::CreeperExplosion { position: blast.position, radius: blast.power, powered: false });
        }
        for (x, y, z) in self.mobs.take_fireball_fires() {
            self.level.light_fire(minecraftoss_core::BlockPos::new(x, y, z));
        }
    }

    pub fn deflect_fireball(&mut self, id: u64, look: [f64; 3], player: u64, by_attack: bool) -> bool {
        self.mobs.deflect_fireball(id, glam::DVec3::from_array(look), player, by_attack)
    }

    pub fn push_fireball(&mut self, id: u64, push: [f64; 3]) -> bool {
        self.mobs.push_fireball(id, glam::DVec3::from_array(push))
    }

    fn explode_blasts(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) {
        use minecraftoss_world::level::explosion::{BlockInteraction, Explosion};
        for (blast, source) in std::mem::take(&mut self.blasts) {
            let explosion = minecraftoss_entities::creeper::CreeperExplosion { position: glam::DVec3::from_array(blast.center), radius: blast.power, powered: false };
            {
                let Self { level, states, mobs, mob_tables, .. } = &mut *self;
                let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(level), states, tables: mob_tables };
                match source {
                    BlastSource::Nobody | BlastSource::KeepBlocks => mobs.blast(&world, explosion, players),
                    BlastSource::Player(player) => mobs.blast_by_player(&world, explosion, players, player),
                    BlastSource::RespawnPoint { .. } => mobs.bad_respawn_point_blast(&world, explosion, players),
                }
            }
            self.drop_death_loot();
            let interaction = if self.block_explosion_drop_decay { BlockInteraction::DestroyWithDecay } else { BlockInteraction::Destroy };
            let interaction = if source == BlastSource::KeepBlocks { BlockInteraction::Keep } else { interaction };
            let exploded = Explosion { center: blast.center, radius: blast.power, fire: blast.fire, interaction, source: None };
            match source {
                BlastSource::RespawnPoint { shield: Some((x, y, z)) } => self.level.explode_shielded(exploded, minecraftoss_core::BlockPos::new(x, y, z)),
                _ => self.level.explode(exploded),
            };
            self.explosions.push(explosion);
        }
    }

    pub fn weapon_blast(&mut self, center: [f64; 3], power: f32) {
        use minecraftoss_world::level::explosion::{BlockInteraction, Explosion};
        let effects = self.level.explosion_effects.len();
        self.level.explode(Explosion { center, radius: power, fire: false, interaction: BlockInteraction::DestroyWithDecay, source: None });
        self.level.explosion_effects.truncate(effects);
    }

    pub fn take_explosion_effects(&mut self) -> Vec<minecraftoss_world::level::explosion::ExplosionEffect> {
        std::mem::take(&mut self.level.explosion_effects)
    }

    /// The blasts since the last call, for the client's sound and particles.
    pub fn take_explosions(&mut self) -> Vec<minecraftoss_entities::creeper::CreeperExplosion> {
        std::mem::take(&mut self.explosions)
    }

    pub fn throw(&mut self, thrown: crate::thrown::Thrown) {
        self.thrown.push(thrown);
    }

    pub fn tick_clouds(&mut self, players: &[crate::thrown::PlayerBody]) {
        use crate::effect_cloud::{Visitor, INSTANT_SCALE};
        use crate::potions::{PlayerPotion, PlayerPotionHit};
        if self.clouds.is_empty() {
            return;
        }
        let mob_targets = crate::thrown::targets(&self.mobs);
        let mut clouds = std::mem::take(&mut self.clouds);
        for cloud in &mut clouds {
            let mut visitors = Vec::new();
            for target in &mob_targets {
                let able = |mobs: &mut minecraftoss_entities::world::EntityWorld, effect: &crate::potions::PotionEffect| match effect.mob_effect() {
                    Some(mob) => mobs.mob_effects_mut(target.id).is_some_and(|(_, undead, spider)| minecraftoss_entities::effects::can_be_affected(mob.effect, undead, spider)),
                    None => true,
                };
                let affected = cloud.effects.iter().any(|effect| able(&mut self.mobs, effect));
                visitors.push(Visitor { key: target.id, feet: crate::thrown::target_feet(target), min: target.min, max: target.max, affected });
            }
            for player in players.iter().filter(|p| p.targetable()) {
                let (min, max) = player.bounds();
                visitors.push(Visitor { key: crate::thrown::PLAYER_TARGET + player.id, feet: player.feet, min, max, affected: true });
            }
            let by = cloud.owner.map(|o| crate::thrown::PLAYER_TARGET + o);
            for key in cloud.tick(&visitors) {
                for effect in &cloud.effects {
                    if let Some(player) = crate::thrown::struck_player(key) {
                        let potion = if effect.instantaneous() {
                            match crate::potions::instant_amount(effect, false, INSTANT_SCALE) {
                                Some(PlayerPotion::Harm { amount, .. }) => PlayerPotion::Harm { amount, by: cloud.owner },
                                Some(other) => other,
                                None => continue,
                            }
                        } else {
                            PlayerPotion::Effect(effect.clone())
                        };
                        self.player_potions.push(PlayerPotionHit { player, potion, at: cloud.position });
                    } else if let Some(mob) = effect.mob_effect() {
                        if effect.instantaneous() {
                            let hit = self.mobs.splash_instant(key, mob.effect, mob.amplifier, INSTANT_SCALE, cloud.position, by);
                            if let (Some(owner), true) = (cloud.owner, hit.is_some_and(|h| h.applied)) {
                                crate::splash::blame_player(&mut self.mobs, key, owner);
                            }
                        } else {
                            self.mobs.add_mob_effect(key, mob);
                        }
                    }
                }
            }
        }
        clouds.retain(|cloud| cloud.alive);
        self.clouds = clouds;
    }

    pub fn throw_trident(&mut self, mut trident: crate::trident::ThrownTrident) {
        if trident.owner_uuid.is_none() && trident.owner != crate::trident::UNKNOWN_OWNER {
            trident.owner_uuid = Some(self.mobs.uuid_of(crate::thrown::PLAYER_TARGET + trident.owner));
        }
        trident.id = self.next_trident;
        self.next_trident = self.next_trident.wrapping_add(1).max(TRIDENT_IDS);
        self.tridents.push(trident);
    }

    pub fn trident_views(&self) -> Vec<crate::trident::TridentView> {
        self.tridents.iter().map(crate::trident::ThrownTrident::view).collect()
    }

    pub fn take_players_struck(&mut self) -> Vec<crate::thrown::PlayerStruck> {
        std::mem::take(&mut self.players_struck)
    }

    pub fn take_players_pushed(&mut self) -> Vec<(u64, glam::DVec3)> {
        std::mem::take(&mut self.players_pushed)
    }

    pub fn set_player_uuid(&mut self, player: u64, uuid: u128) {
        self.mobs.set_player_uuid(player, uuid);
    }

    fn wind_burst(&mut self, center: glam::DVec3, players: &[crate::thrown::PlayerBody]) {
        let pushes = {
            let Self { level, states, mob_tables, mobs, .. } = self;
            let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(level), states, tables: mob_tables };
            crate::wind::burst(&world, mobs, center, players)
        };
        self.players_pushed.extend(pushes);
        self.level.wind_burst(center.to_array(), crate::wind::RADIUS, crate::wind::KNOCKBACK);
        let (a, b) = (self.sound_random.next_float(), self.sound_random.next_float());
        self.level.sounds.push(minecraftoss_world::level::burning::LevelSound { event: crate::wind::BURST_SOUND, pos: center.to_array(), volume: 4.0, pitch: crate::wind::burst_pitch(a, b) });
    }

    pub fn tick_tridents(&mut self, players: &[crate::thrown::PlayerBody]) {
        use crate::trident::TridentEvent;
        if self.tridents.is_empty() {
            return;
        }
        let mob_targets = crate::thrown::targets(&self.mobs);
        let mut tridents = std::mem::take(&mut self.tridents);
        for trident in &mut tridents {
            let chunk = ChunkPos::new((trident.position.x.floor() as i32) >> 4, (trident.position.z.floor() as i32) >> 4);
            if self.level.chunk(chunk).is_none() {
                continue;
            }
            if trident.owner == crate::trident::UNKNOWN_OWNER {
                if let Some(player) = players.iter().find(|p| trident.owner_uuid == Some(self.mobs.uuid_of(crate::thrown::PLAYER_TARGET + p.id))) {
                    trident.owner = player.id;
                }
            }
            let mut targets = mob_targets.clone();
            targets.extend(crate::thrown::player_targets(players, trident.owner));
            let owner = players.iter().find(|p| p.id == trident.owner).map(crate::trident::TridentOwner::of);
            let events = {
                let Self { level, states, mob_tables, .. } = self;
                let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(level), states, tables: mob_tables };
                trident.tick(&world, &targets, owner)
            };
            for event in events {
                match event {
                    TridentEvent::Struck { target, velocity } => match crate::thrown::struck_player(target) {
                        Some(player) => self.players_struck.push(crate::thrown::PlayerStruck { player, by: trident.owner, damage: crate::trident::DAMAGE, kind: crate::trident::DAMAGE_TYPE, velocity }),
                        None => {
                            let seed = self.sound_random.next_long();
                            crate::thrown::hurt_by(&mut self.mobs, target, crate::trident::DAMAGE, crate::trident::DAMAGE_TYPE, velocity, trident.owner, &mut minecraftoss_player::rng::LegacyRandom::new(seed));
                        }
                    },
                    TridentEvent::Sound { event, at, volume, pitch } => {
                        self.level.sounds.push(minecraftoss_world::level::burning::LevelSound { event, pos: at.to_array(), volume, pitch });
                    }
                    TridentEvent::Dropped { at } => {
                        let components = trident.stack.components.as_ref().map(|c| c.to_string());
                        let stack = stack_of(&trident.stack.id, i32::from(trident.stack.count.max(1)), components.as_deref());
                        self.level.spawn_at_location(at.to_array(), stack);
                    }
                }
            }
        }
        tridents.retain(|t| t.alive);
        tridents.append(&mut self.tridents);
        self.tridents = tridents;
    }

    pub fn pickup_tridents(&mut self, player: &crate::thrown::PlayerBody, players: &[crate::thrown::PlayerBody], mut take: impl FnMut(&minecraftoss_player::inventory::ItemStack) -> bool) -> Vec<PickedStack> {
        let mut picked = Vec::new();
        for trident in &mut self.tridents {
            if !trident.touched_by(player) {
                continue;
            }
            let owner_here = players.iter().any(|p| p.id == trident.owner);
            match trident.taken_by(player.id, owner_here) {
                Some(Some(stack)) => {
                    if take(&stack) {
                        trident.alive = false;
                        let components = stack.components.as_ref().map(|c| c.to_string());
                        picked.push((trident.id, trident.position.to_array(), stack.id.clone(), i32::from(stack.count.max(1)), components));
                    }
                }
                Some(None) => trident.alive = false,
                None => {}
            }
        }
        self.tridents.retain(|t| t.alive);
        picked
    }

    pub fn cloud_views(&self) -> Vec<crate::effect_cloud::CloudView> {
        self.clouds.iter().map(crate::effect_cloud::EffectCloud::view).collect()
    }

    fn douse_fire(&mut self, (x, y, z): BlockPos) {
        let at = minecraftoss_core::BlockPos::new(x, y, z);
        let state = self.level.block(at);
        let registries = self.level.registries();
        let tagged = |name: &str| registries.block_tags.id(name).is_some_and(|tag| registries.block_in_tag(state, tag));
        let lit = registries.blocks.property(state, "lit") == Some("true");
        let (fire, candle, campfire) = (tagged("minecraft:fire"), lit && (tagged("minecraft:candles") || tagged("minecraft:candle_cakes")), lit && tagged("minecraft:campfires"));
        let unlit = registries.blocks.with_property(state, "lit", "false");
        let center = [f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5];
        if fire {
            self.level.destroy_block_drops(at, false, update::LIMIT);
        } else if let (true, Some(unlit)) = (candle, unlit) {
            self.level.set_block(at, unlit, update::ALL, update::LIMIT);
            self.level.sounds.push(minecraftoss_world::level::burning::LevelSound { event: crate::splash::CANDLE_SOUND, pos: center, volume: 1.0, pitch: 1.0 });
        } else if let (true, Some(unlit)) = (campfire, unlit) {
            let (a, b) = (self.sound_random.next_float(), self.sound_random.next_float());
            self.level.sounds.push(minecraftoss_world::level::burning::LevelSound { event: crate::splash::FIRE_SOUND, pos: center, volume: 0.5, pitch: 2.6 + (a - b) * 0.8 });
            self.level.set_block(at, unlit, update::ALL, update::LIMIT);
        }
    }

    pub fn take_thrown_hits(&mut self) -> Vec<crate::thrown::ThrownHit> {
        std::mem::take(&mut self.thrown_hits)
    }

    pub fn take_pearl_teleports(&mut self) -> Vec<(u64, [f64; 3])> {
        std::mem::take(&mut self.pearl_landings)
    }

    pub fn take_pearl_gateways(&mut self) -> Vec<(u64, BlockPos)> {
        std::mem::take(&mut self.pearl_gateways)
    }

    pub fn take_potion_breaks(&mut self) -> Vec<minecraftoss_entities::world::PotionBreak> {
        self.sound_potion_breaks();
        std::mem::take(&mut self.potion_breaks)
    }

    fn sound_potion_breaks(&mut self) {
        for broken in self.mobs.take_potion_breaks() {
            let pitch = crate::thrown::break_pitch(self.sound_random.next_float());
            self.level.sounds.push(minecraftoss_world::level::burning::LevelSound { event: crate::thrown::BREAK_SOUND, pos: crate::thrown::sound_cell(broken.position), volume: 1.0, pitch });
            self.potion_breaks.push(broken);
        }
    }

    pub fn thrown_views(&self) -> Vec<crate::thrown::ThrownView> {
        self.thrown.iter().map(crate::thrown::Thrown::view).collect()
    }

    pub fn throw_eye(&mut self, owner: u64, throw: &crate::ender_eye::EyeThrow) -> bool {
        self.eyes.throw(self.worldgen, &mut self.level, owner, throw)
    }

    pub fn take_eye_answers(&mut self) -> Vec<crate::eye_of_ender::EyeAnswer> {
        self.eyes.take_answers()
    }

    pub fn bottle_breath(&mut self, feet: glam::DVec3, height: f64) -> bool {
        crate::effect_cloud::bottle_breath(&mut self.clouds, crate::effect_cloud::bottle_reach(feet, height))
    }

    pub fn tick_eyes(&mut self) {
        self.eyes.tick(&mut self.level);
    }

    pub fn eye_snapshot(&mut self) -> crate::eye_of_ender::EyeSnapshot {
        self.eyes.snapshot()
    }

    pub fn take_player_potions(&mut self) -> Vec<crate::potions::PlayerPotionHit> {
        std::mem::take(&mut self.player_potions)
    }

    pub fn tick_thrown(&mut self, players: &[crate::thrown::PlayerBody]) {
        use crate::thrown::Throwable;
        self.thrown.retain(|t| t.kind != Throwable::EnderPearl || players.iter().any(|p| p.id == t.owner && p.alive));
        if self.thrown.is_empty() {
            return;
        }
        let mob_targets = crate::thrown::targets(&self.mobs);
        let mut struck = Vec::new();
        let mut hatched = Vec::new();
        let mut awards = Vec::new();
        let mut splashes = Vec::new();
        let mut lingering = Vec::new();
        let mut bursts = Vec::new();
        let mut aged = Vec::new();
        {
            let Self { level, states, mob_tables, thrown, pearl_landings, potion_breaks, thrown_hits, gateways, pearl_gateways, .. } = self;
            let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(level), states, tables: mob_tables };
            for t in thrown.iter_mut() {
                let mut targets = mob_targets.clone();
                targets.extend(crate::thrown::player_targets(players, t.owner));
                let landed = t.tick(&world, &targets, crate::thrown::owner_box(players, t.owner));
                if t.kind == Throwable::EnderPearl {
                    let to = landed.as_ref().map_or_else(|| t.position(), |landed| landed.at);
                    let through = crate::end_gateway::pearl_gateway(t.previous.to_array(), to.to_array(), |pos| match gateways.trigger_cooldown(pos) {
                        Some(age) => {
                            aged.push((pos, age));
                            true
                        }
                        None => false,
                    });
                    if let Some(gateway) = through {
                        t.discard();
                        pearl_gateways.push((t.owner, gateway));
                        continue;
                    }
                }
                let Some(landed) = landed else { continue };
                let hurts = matches!(t.kind, Throwable::Snowball | Throwable::Egg | Throwable::EnderPearl);
                if let Some(mob) = landed.mob.filter(|&id| hurts && crate::thrown::struck_player(id).is_none()) {
                    struck.push((mob, landed.velocity, t.owner, t.random().next_long()));
                }
                if hurts {
                    thrown_hits.push(crate::thrown::ThrownHit { item: t.item.clone(), kind: t.kind, at: landed.at });
                }
                match t.kind {
                    Throwable::Egg => {
                        for _ in 0..crate::thrown::chicks(t.random()) {
                            hatched.push((crate::thrown::hatch_point(landed.at, landed.before).to_array(), t.yaw(), crate::thrown::chick_variant(&t.item)));
                        }
                    }
                    Throwable::EnderPearl => pearl_landings.push((t.owner, landed.before.to_array())),
                    Throwable::Snowball => {}
                    Throwable::ExperienceBottle => {
                        let at = t.position();
                        potion_breaks.push(minecraftoss_entities::world::PotionBreak { position: at, color: crate::thrown::BOTTLE_COLOR, instant: false });
                        let pitch = crate::thrown::break_pitch(t.random().next_float());
                        let amount = crate::thrown::bottle_experience(t.random());
                        let direction = landed.face.map_or(-landed.velocity, crate::thrown::face_normal);
                        awards.push((crate::thrown::sound_cell(at), pitch, landed.at, direction, amount));
                    }
                    Throwable::WindCharge => {
                        let center = if landed.mob.is_some() { t.position() } else { crate::wind::burst_point(landed.at, landed.face) };
                        bursts.push((landed.mob, landed.velocity, t.owner, t.random().next_long(), center));
                    }
                    Throwable::SplashPotion | Throwable::LingeringPotion => {
                        let contents = t.contents();
                        let at = t.position();
                        potion_breaks.push(minecraftoss_entities::world::PotionBreak { position: at, color: contents.color(), instant: contents.instant_potion() });
                        let pitch = crate::thrown::break_pitch(t.random().next_float());
                        let scale = crate::potions::duration_scale(&t.item, t.components.as_ref());
                        let douse = match (contents.is_water(), landed.block, landed.face) {
                            (true, Some(block), Some(face)) => Some(crate::thrown::douse_cells(block, face)),
                            _ => None,
                        };
                        if t.kind == Throwable::LingeringPotion {
                            if contents.has_effects() {
                                let struck = landed.mob.and_then(|id| match crate::thrown::struck_player(id) {
                                    Some(player) => players.iter().find(|p| p.id == player).map(|p| p.feet),
                                    None => crate::thrown::targets_by_id(&mob_targets, id),
                                });
                                lingering.push(crate::effect_cloud::EffectCloud::lingering(struck.unwrap_or(at), Some(t.owner), &contents, scale));
                            }
                            splashes.push((at, t.margin(), t.owner, contents.is_water(), douse, Vec::new(), scale, pitch));
                        } else {
                            splashes.push((at, t.margin(), t.owner, contents.is_water(), douse, contents.all_effects(), scale, pitch));
                        }
                    }
                }
            }
            thrown.retain(crate::thrown::Thrown::alive);
        }
        for (pos, age) in aged {
            crate::end_gateway::write_age(&mut self.level, pos, age);
        }
        for (cell, pitch, hit, direction, amount) in awards {
            self.level.sounds.push(minecraftoss_world::level::burning::LevelSound { event: crate::thrown::BREAK_SOUND, pos: cell, volume: 1.0, pitch });
            self.level.award_experience_toward(hit.to_array(), direction.to_array(), amount);
        }
        for (at, margin, owner, water, douse, effects, scale, pitch) in splashes {
            for cell in douse.into_iter().flatten() {
                self.douse_fire(cell);
            }
            if water {
                let wet = crate::splash::wet(&mut self.mobs, at, owner, players);
                self.player_potions.extend(wet);
            }
            let hits = crate::splash::splash(&mut self.mobs, at, margin, owner, &effects, scale, players);
            self.player_potions.extend(hits);
            self.level.sounds.push(minecraftoss_world::level::burning::LevelSound { event: crate::thrown::BREAK_SOUND, pos: crate::thrown::sound_cell(at), volume: 1.0, pitch });
        }
        self.clouds.append(&mut lingering);
        for (mob, velocity, owner, seed, center) in bursts {
            match mob.map(|id| (id, crate::thrown::struck_player(id))) {
                Some((_, Some(player))) => self.players_struck.push(crate::thrown::PlayerStruck { player, by: owner, damage: crate::wind::DAMAGE, kind: crate::wind::DAMAGE_TYPE, velocity }),
                Some((id, None)) => {
                    crate::thrown::hurt_by(&mut self.mobs, id, crate::wind::DAMAGE, crate::wind::DAMAGE_TYPE, velocity, owner, &mut minecraftoss_player::rng::LegacyRandom::new(seed));
                }
                None => {}
            }
            self.wind_burst(center, players);
        }
        for (mob, velocity, owner, seed) in struck {
            crate::thrown::hurt(&mut self.mobs, mob, velocity, owner, &mut minecraftoss_player::rng::LegacyRandom::new(seed));
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
        self.collect_block_entity_changes();
        self.save_chunk_entities(pos, true);
        self.level.remove_chunk(pos);
        self.gateways.unload_chunk(pos);
        self.mobs.pois.unload_chunk(pos.x, pos.z);
    }

    fn state_of(&self, block: Option<&Block>) -> BlockStateId {
        block.and_then(|b| self.states.state_of(b)).unwrap_or(BlockStateId::AIR)
    }

    pub fn player_breaks(&mut self, positions: &[BlockPos]) {
        let at: Vec<_> = positions.iter().map(|p| minecraftoss_core::BlockPos::new(p.0, p.1, p.2)).collect();
        self.level.remove_blocks(&at, BlockStateId::AIR);
    }

    pub fn player_mines(&mut self, broken: &[(BlockPos, HowBroken)]) {
        let at: Vec<_> = broken.iter().map(|(p, _)| minecraftoss_core::BlockPos::new(p.0, p.1, p.2)).collect();
        let old: Vec<_> = at.iter().map(|&pos| self.level.block(pos)).collect();
        self.level.remove_blocks(&at, BlockStateId::AIR);
        let mut destroyed: Vec<_> = at
            .into_iter()
            .zip(old)
            .zip(broken.iter().map(|(_, how)| *how))
            .filter(|(_, how)| *how != HowBroken::Blasted)
            .collect();
        destroyed.sort_by_key(|((pos, _), _)| pos.y);
        for ((pos, old), how) in destroyed {
            self.level.ice_player_destroyed(pos, old, how == HowBroken::SilkTouched);
        }
    }

    pub fn take_destroyed(&mut self) -> Vec<(BlockPos, Block)> {
        let destroyed = self.level.destroyed.as_mut().map_or_else(Vec::new, std::mem::take);
        destroyed
            .into_iter()
            .filter_map(|(pos, state)| self.states.block(state).cloned().map(|block| ((pos.x, pos.y, pos.z), block)))
            .collect()
    }

    pub fn command_block(&self, pos: BlockPos) -> BlockStateId {
        self.level.block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2))
    }

    pub fn command_registries(&self) -> &minecraftoss_core::Registries {
        self.level.registries()
    }

    pub fn command_destroy(&mut self, pos: BlockPos) -> bool {
        self.level.destroy_block_drops(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2), true, update::LIMIT)
    }

    pub fn command_place(&mut self, pos: BlockPos, state: BlockStateId, defined: &[(String, String)], strict: bool) -> bool {
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        let lib = self.level.lib;
        let shaped = if strict {
            state
        } else {
            let mut ctx = minecraftoss_generator::feature::Ctx { lib, region: &mut self.level };
            minecraftoss_generator::feature::template::update_from_neighbour_shapes(&mut ctx, state, at)
        };
        let blocks = &lib.registries.blocks;
        let mut placed = if blocks.is_air(shaped) { state } else { shaped };
        for (name, value) in defined {
            if let Some(next) = blocks.with_property(placed, name, value) {
                placed = next;
            }
        }
        let flags = if strict {
            update::CLIENTS | update::SKIP_ON_PLACE | update::SKIP_BLOCK_ENTITY_SIDEEFFECTS | update::SUPPRESS_DROPS | update::KNOWN_SHAPE
        } else {
            update::CLIENTS | update::SKIP_BLOCK_ENTITY_SIDEEFFECTS
        };
        self.level.set_block(at, placed, flags, update::LIMIT)
    }

    pub fn command_updates(&mut self, pos: BlockPos, old: BlockStateId) {
        self.level.update_neighbours_on_block_set(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2), old);
    }

    pub fn command_closest_poi(&self, ids: &[String], center: BlockPos, radius: i32) -> Option<(BlockPos, String)> {
        let wanted = |kind: minecraftoss_entities::poi::PoiType| ids.iter().any(|id| id == kind.id());
        let pos = self.mobs.pois.find_closest(&wanted, center, radius, minecraftoss_entities::poi::Occupancy::Any)?;
        Some((pos, self.mobs.pois.kind(pos)?.id().to_owned()))
    }

    pub fn command_block_entity(&self, pos: BlockPos) -> Option<&minecraftoss_core::nbt::Tag> {
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        self.level.chunk(at.chunk())?.block_entities.entities.get(&pos)
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
        if matches!(edit, PlayerEdit::Place) && !self.may_place(at, target) {
            let refund = self.refund_of(target);
            self.level.refuse_placement(at, refund.as_deref());
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

    fn may_place(&self, at: minecraftoss_core::BlockPos, target: BlockStateId) -> bool {
        let blocks = &self.level.registries().blocks;
        let placed = blocks.block_of(target);
        if blocks.is_air(target) || blocks.block_of(self.level.block(at)) == placed {
            return true;
        }
        blocks.block(placed).name.as_str() == crate::scaffolding::SCAFFOLDING || self.level.can_survive_at(target, at)
    }

    fn refund_of(&self, target: BlockStateId) -> Option<String> {
        let blocks = &self.level.registries().blocks;
        if blocks.property(target, "half") == Some("upper") {
            return None;
        }
        let name = blocks.block(blocks.block_of(target)).name.as_str();
        Some(match name {
            "minecraft:redstone_wire" => "minecraft:redstone".to_owned(),
            "minecraft:sweet_berry_bush" => crate::berries::SWEET_BERRIES.to_owned(),
            "minecraft:cave_vines" | "minecraft:cave_vines_plant" => crate::berries::GLOW_BERRIES.to_owned(),
            _ => name.replace("wall_", "").replace("candle_cake", "candle"),
        })
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
        if self.teleport_egg(pos) {
            return true;
        }
        let facing = minecraftoss_core::pos::Direction::from_name(player_facing);
        self.level.use_block_facing(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2), facing)
    }

    /// `BoneMealItem.useOn` on a clicked face: grow the block, or water
    /// plants in front of a sturdy face. True when the bone meal is used.
    pub fn bone_meal(&mut self, pos: BlockPos, face: &str) -> bool {
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        if self.level.grow_crop(at) {
            self.level.level_events.push((crate::level_events::BONE_MEAL_USE, at, crate::level_events::BONE_MEAL_PARTICLES));
            return true;
        }
        let Some(face) = minecraftoss_core::pos::Direction::from_name(face) else { return false };
        let state = self.level.block(at);
        let grown = self.level.registries().blocks.is_face_sturdy(state, face, minecraftoss_core::SupportType::Full) && self.level.grow_water_plant(at.relative(face, 1));
        if grown {
            self.level.level_events.push((crate::level_events::BONE_MEAL_USE, at.relative(face, 1), crate::level_events::BONE_MEAL_PARTICLES));
        }
        grown
    }

    pub fn attack_block(&mut self, pos: BlockPos) -> bool {
        if self.teleport_egg(pos) {
            return true;
        }
        self.level.attack_block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2))
    }

    pub fn teleport_egg(&mut self, pos: BlockPos) -> bool {
        use minecraftoss_core::random::RandomSource;
        let at = |(x, y, z): BlockPos| minecraftoss_core::BlockPos::new(x, y, z);
        let state = self.level.block(at(pos));
        let blocks = &self.level.registries().blocks;
        if blocks.block_by_name(crate::dragon_egg::DRAGON_EGG) != Some(blocks.block_of(state)) {
            return false;
        }
        let mut random = self.level.random.clone();
        let level = &self.level;
        let (low, high) = (level.min_y(), level.max_y());
        let air = |pos: BlockPos| level.registries().blocks.is_air(level.block(at(pos)));
        let fits = |to: BlockPos| air(to) && !air((to.0, to.1 - 1, to.2)) && crate::dragon_egg::within_border(to) && (low..=high).contains(&to.1);
        let target = crate::dragon_egg::teleport_target(pos, &mut |bound| random.next_i32_bound(bound), fits);
        self.level.random = random;
        if let Some(to) = target {
            self.egg_teleports.push((pos, to));
            self.level.set_block(at(to), state, update::CLIENTS, update::LIMIT);
            self.level.remove_block(at(pos), false);
        }
        true
    }

    pub fn take_egg_teleports(&mut self) -> Vec<(BlockPos, BlockPos)> {
        std::mem::take(&mut self.egg_teleports)
    }

    pub fn item_on_block(&mut self, pos: BlockPos, action: ItemOnBlock) -> bool {
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        match action {
            ItemOnBlock::EmptyBucket { water } => self.level.empty_bucket_into(at, water),
            ItemOnBlock::FillBucket => self.level.bucket_pickup(at).is_some(),
            ItemOnBlock::Ignite => self.level.place_fire(at),
            ItemOnBlock::PrimeTnt => self.level.ignite_tnt(at),
            ItemOnBlock::BoneMeal { face } => self.bone_meal(pos, face),
            ItemOnBlock::EnderEye => self.level.place_ender_eye(at).is_some(),
            ItemOnBlock::EndCrystal => {
                let placed = self.place_end_crystal(pos);
                if placed && self.dragon_fight_level {
                    self.try_respawn();
                }
                placed
            }
        }
    }

    pub fn place_end_crystal(&mut self, base: BlockPos) -> bool {
        let at = |(x, y, z): BlockPos| minecraftoss_core::BlockPos::new(x, y, z);
        let blocks = &self.level.registries().blocks;
        let base_block = blocks.block_of(self.level.block(at(base)));
        let takes = [crate::end_crystal::OBSIDIAN, crate::end_crystal::BEDROCK].iter().any(|name| blocks.block_by_name(name) == Some(base_block));
        if !takes || !blocks.is_air(self.level.block(at((base.0, base.1 + 1, base.2)))) {
            return false;
        }
        let (low, high) = crate::end_crystal::placement_box(base);
        let overlaps = |position: glam::DVec3, half: f64, height: f64| {
            position.x - half < high.x && position.x + half > low.x && position.y < high.y && position.y + height > low.y && position.z - half < high.z && position.z + half > low.z
        };
        let occupied = self.crystals.touches((low, high))
            || self.mobs.census().iter().any(|mob| overlaps(mob.position, f64::from(mob.width / 2.0), f64::from(mob.height)))
            || self.level.players.iter().any(|feet| overlaps(glam::DVec3::from_array(*feet), 0.3, 1.8))
            || self.level.entities.iter().any(|entity| overlaps(glam::DVec3::from_array(entity.pos), 0.125, 0.25));
        if occupied {
            return false;
        }
        let tag = self.crystals.fresh_tag(crate::end_crystal::placed_at(base), false);
        self.crystals.add(&tag).is_some()
    }

    pub fn tick_end(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) {
        let blasts: Vec<(glam::DVec3, f32)> = self.explosions.iter().map(|blast| (blast.position, blast.radius)).collect();
        for (center, radius) in blasts {
            for destroyed in self.crystals.blast(center, radius, None) {
                self.crystal_destroyed(destroyed);
            }
            let mut events = crate::ender_dragon::DragonEvents::default();
            for dragon in &mut self.dragons.list {
                dragon.blast(center, radius, None, &mut events);
            }
            self.end_events(events);
        }
        for pos in self.crystals.tick() {
            if self.dragon_fight_level {
                self.level.light_fire(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2));
            }
        }
        if self.dragon_fight_level {
            self.crystal_scan_ticks += 1;
            if self.crystal_scan_ticks >= 100 {
                self.update_crystal_count();
            }
        }
        self.tick_dragon_fireballs(players);
        self.tick_dragons(players);
        self.tick_respawn(players);
    }

    fn update_crystal_count(&mut self) {
        self.crystal_scan_ticks = 0;
        let spikes = minecraftoss_generator::feature::kinds::the_end::spikes_for_seed(self.worldgen.terrain.seed);
        self.alive_crystals = spikes.iter().map(|spike| self.crystals.on_spike(spike.center_x, spike.center_z, spike.radius) as i32).sum();
    }

    fn tick_dragons(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) {
        use minecraftoss_generator::feature::World as _;
        let bodies: Vec<crate::ender_dragon::PlayerBody> = players
            .iter()
            .map(|player| crate::ender_dragon::PlayerBody {
                id: player.id,
                feet: player.position,
                height: match player.eye_height {
                    e if e >= 1.5 => 1.8,
                    e if e >= 1.0 => 1.5,
                    _ => 0.6,
                },
                eye_height: f64::from(player.eye_height),
                attackable: player.alive && !player.spectator && player.attackable,
                crouching: (1.0..1.5).contains(&player.eye_height),
            })
            .collect();
        self.end_players = bodies.clone();
        let mut experience = Vec::new();
        let mob_boxes: Vec<(u64, (glam::DVec3, glam::DVec3))> = crate::thrown::targets(&self.mobs).iter().map(|target| (target.id, (target.min, target.max))).collect();
        let mut mob_hits = Vec::new();
        let griefing = self.mobs.mob_griefing();
        let peaceful = self.level.difficulty == 0;
        {
            let Self { level, states, mob_tables, dragons, fight_dragon, alive_crystals, end_sounds, end_hits, dragons_killed, crystals: self_crystals, fight, dragon_drops, dragon_fireballs, clouds, players_pushed, .. } = self;
            let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(level), states, tables: mob_tables };
            let height = |x: i32, z: i32| world.level.borrow().height_at(minecraftoss_core::chunk::HeightmapKind::MotionBlockingNoLeaves, x, z);
            let sees = |from: glam::DVec3, to: glam::DVec3| minecraftoss_entities::sight::line_of_sight(&world, from, to);
            let empty = |x: i32, y: i32, z: i32| {
                let level = world.level.borrow();
                level.registries().blocks.is_air(level.block(minecraftoss_core::BlockPos::new(x, y, z)))
            };
            let crystals: Vec<(u64, glam::DVec3)> = self_crystals.list.iter().map(|crystal| (crystal.id, crystal.position)).collect();
            let (immune, transparent) = {
                let level = world.level.borrow();
                (level.registries().block_tags.id("minecraft:dragon_immune"), level.registries().block_tags.id("minecraft:dragon_transparent"))
            };
            let wall = |x: i32, y: i32, z: i32| {
                let level = world.level.borrow();
                let state = level.block(minecraftoss_core::BlockPos::new(x, y, z));
                let registries = level.registries();
                let tagged = |tag: Option<minecraftoss_core::tags::TagId>| tag.is_some_and(|tag| registries.block_in_tag(state, tag));
                if registries.blocks.is_air(state) || tagged(transparent) {
                    crate::ender_dragon::Wall::Pass
                } else if tagged(immune) {
                    crate::ender_dragon::Wall::Immune
                } else if level.fluid_legacy_block(level.fluid_state(state)) == state {
                    crate::ender_dragon::Wall::Stays
                } else {
                    crate::ender_dragon::Wall::Breaks
                }
            };
            for dragon in &mut dragons.list {
                let origin = dragon.fight_origin;
                let top = || world.level.borrow().height_at(minecraftoss_core::chunk::HeightmapKind::MotionBlocking, origin.0, origin.2);
                let ours = *fight_dragon == Some(dragon.uuid);
                let death_experience = crate::ender_dragon::death_experience(ours, fight.previously_killed, *dragon_drops);
                let env = crate::ender_dragon::DragonEnv { height: &height, podium_top: &top, alive_crystals: ours.then_some(*alive_crystals), players: &bodies, crystals: &crystals, death_experience, sees: &sees, empty: &empty, mobs: &mob_boxes, wall: &wall, griefing, peaceful };
                let events = dragon.tick(&env);
                for &(x, y, z) in &events.broken {
                    world.level.borrow_mut().remove_block(minecraftoss_core::BlockPos::new(x, y, z), false);
                }
                players_pushed.extend(events.player_pushes);
                mob_hits.extend(events.mob_hits.into_iter().map(|(id, push, damage)| (id, push, damage, dragon.position)));
                end_sounds.extend(events.sounds.into_iter().map(|(event, position, volume, pitch)| minecraftoss_entities::world::MobSound { event: event.to_owned(), position, volume, pitch, category: "hostile" }));
                end_hits.extend(events.player_hits.into_iter().map(|(player_id, damage, attacker)| minecraftoss_entities::world::PlayerHit { player_id, damage, kind: minecraftoss_entities::world::PlayerHitKind::Melee { attacker, hunger_ticks: 0, lift: 0.0 }, source: Some(dragon.id) }));
                experience.extend(events.experience);
                for (from, direction) in events.fireballs {
                    dragon_fireballs.shoot(dragon.id, from, direction);
                }
                clouds.extend(events.clouds);
            }
            for dragon in dragons.list.iter_mut().filter(|dragon| dragon.flame_out) {
                dragon.flame_out = false;
                clouds.retain(|cloud| cloud.breath != Some(crate::effect_cloud::Breath::Flame(dragon.id)));
            }

            dragons_killed.extend(dragons.list.iter().filter(|dragon| dragon.removed).map(|dragon| dragon.uuid));
            dragons.list.retain(|dragon| !dragon.removed);
        }
        for (position, amount) in experience {
            self.level.award_experience(position.to_array(), amount);
        }
        for (id, push, damage, attacker) in mob_hits {
            if let Some(body) = self.mobs.body_mut(id).filter(|_| push != glam::DVec3::ZERO) {
                body.velocity += push;
                body.needs_sync = true;
            }
            let full = damage > 0.0 && self.mobs.player_blast(None, id, damage, attacker, 0.0).is_some_and(|result| result.full);
            if let Some(body) = self.mobs.body_mut(id).filter(|_| full) {
                body.velocity = crate::ender_dragon::knockback(body.velocity, body.on_ground, attacker.x - body.position.x, attacker.z - body.position.z);
                body.needs_sync = true;
            }
        }
    }

    fn tick_dragon_fireballs(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) {
        use minecraftoss_entities::fireball::FireballTarget;
        if self.dragon_fireballs.list.is_empty() {
            return;
        }
        let mut targets: Vec<FireballTarget> = crate::thrown::targets(&self.mobs).iter().map(|target| FireballTarget { id: target.id, min: target.min, max: target.max, owner: false }).collect();
        let body = |feet: glam::DVec3| (feet - glam::DVec3::new(0.3, 0.0, 0.3), feet + glam::DVec3::new(0.3, 1.8, 0.3));
        targets.extend(players.iter().filter(|player| player.alive && !player.spectator).map(|player| {
            let (min, max) = body(player.position);
            FireballTarget { id: crate::thrown::PLAYER_TARGET + player.id, min, max, owner: false }
        }));
        let mut bursts = Vec::new();
        {
            let Self { level, states, mob_tables, dragons, dragon_fireballs, .. } = self;
            let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(level), states, tables: mob_tables };
            for fireball in &mut dragon_fireballs.list {
                let at = fireball.fireball.position;
                let loaded = world.level.borrow().chunk(ChunkPos::new((at.x.floor() as i32) >> 4, (at.z.floor() as i32) >> 4)).is_some();
                if !loaded || !dragons.list.iter().any(|dragon| dragon.id == fireball.owner) {
                    fireball.fireball.alive = false;
                    continue;
                }
                bursts.extend(fireball.tick(&world, &targets));
            }
            dragon_fireballs.list.retain(|fireball| fireball.fireball.alive);
        }
        for at in bursts {
            self.clouds.push(crate::dragon_fireball::breath_cloud(crate::dragon_fireball::cloud_position(at, &targets)));
            self.dragon_fireballs.burst(at);
            let pitch = crate::dragon_fireball::explode_pitch(self.sound_random.next_float());
            self.end_sounds.push(minecraftoss_entities::world::MobSound { event: crate::dragon_fireball::EXPLODE_SOUND.to_owned(), position: crate::ender_dragon::block_center(at), volume: 1.0, pitch, category: "hostile" });
        }
    }

    fn end_events(&mut self, events: crate::ender_dragon::DragonEvents) {
        self.end_sounds.extend(events.sounds.into_iter().map(|(event, position, volume, pitch)| minecraftoss_entities::world::MobSound { event: event.to_owned(), position, volume, pitch, category: "hostile" }));
    }

    pub fn take_end_sounds(&mut self) -> Vec<minecraftoss_entities::world::MobSound> {
        std::mem::take(&mut self.end_sounds)
    }

    pub fn take_dragons_killed(&mut self) -> Vec<[i32; 4]> {
        std::mem::take(&mut self.dragons_killed)
    }

    pub fn take_end_hits(&mut self) -> Vec<minecraftoss_entities::world::PlayerHit> {
        std::mem::take(&mut self.end_hits)
    }

    pub fn find_or_create_dragon(&mut self, origin: BlockPos) -> [i32; 4] {
        if let Some(dragon) = self.dragons.list.first() {
            let uuid = dragon.uuid;
            self.fight_dragon = Some(uuid);
            return uuid;
        }
        self.create_new_dragon(origin)
    }

    fn create_new_dragon(&mut self, origin: BlockPos) -> [i32; 4] {
        let position = glam::DVec3::new(f64::from(origin.0), f64::from(128 + origin.1), f64::from(origin.2));
        let yaw = self.end_random.next_float() * 360.0;
        let tag = crate::ender_dragon::Dragons::fresh_tag(position, yaw, &mut self.end_random);
        let id = self.dragons.add(&tag);
        let dragon = self.dragons.list.iter_mut().find(|dragon| Some(dragon.id) == id);
        let uuid = dragon.map_or([0; 4], |dragon| {
            dragon.fight_origin = origin;
            dragon.set_phase(crate::ender_dragon::Phase::HoldingPattern);
            dragon.uuid
        });
        self.fight_dragon = Some(uuid);
        uuid
    }

    pub fn link_fight(&mut self, link: FightLink) {
        self.fight_dragon = link.dragon;
        self.fight = link;
    }

    pub fn take_fight_reports(&mut self) -> Vec<FightReport> {
        std::mem::take(&mut self.fight_reports)
    }

    pub fn resume_respawn(&mut self, respawn: crate::dragon_respawn::Respawn) {
        self.respawn = Some(respawn);
    }

    fn try_respawn(&mut self) {
        if !self.fight.dragon_killed || self.respawn.is_some() {
            return;
        }
        let origin = minecraftoss_core::BlockPos::new(0, 0, 0);
        let location = match self.fight.exit_portal {
            Some((x, y, z)) => minecraftoss_core::BlockPos::new(x, y, z),
            None => match self.level.find_exit_portal(origin) {
                Some(found) => found,
                None => self.level.spawn_exit_portal(origin, None, true),
            },
        };
        let mut crystals = Vec::new();
        for cell in crate::dragon_respawn::crystal_cells((location.x, location.y, location.z)) {
            let found = self.crystals.in_cell(cell);
            if found.is_empty() {
                return;
            }
            crystals.extend(found);
        }
        self.level.dismantle_exit_portal(origin);
        let location = self.level.spawn_exit_portal(origin, Some(location), false);
        let exit_portal = (location.x, location.y, location.z);
        self.fight.exit_portal = Some(exit_portal);
        let respawn = crate::dragon_respawn::Respawn::new(crystals);
        self.fight_reports.push(FightReport::Respawning { stage: respawn.stage.name(), crystals: respawn.crystals.clone(), exit_portal });
        self.respawn = Some(respawn);
    }

    fn abort_respawn(&mut self) {
        self.respawn = None;
        self.reset_spike_crystals();
        let origin = minecraftoss_core::BlockPos::new(0, 0, 0);
        let at = |(x, y, z): BlockPos| minecraftoss_core::BlockPos::new(x, y, z);
        self.level.spawn_exit_portal(origin, self.fight.exit_portal.map(at), true);
        self.fight_reports.push(FightReport::Aborted);
    }

    fn reset_spike_crystals(&mut self) {
        for spike in minecraftoss_generator::feature::kinds::the_end::spikes_for_seed(self.worldgen.terrain.seed) {
            self.crystals.reset_on_spike(spike.center_x, spike.center_z, spike.radius);
        }
    }

    fn tick_respawn(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) {
        use crate::dragon_respawn::Action;
        if !self.fight.ticking {
            return;
        }
        let Some(mut respawn) = self.respawn.take() else {
            return;
        };
        if self.crystals.by_uuid(&respawn.crystals).is_empty() {
            self.respawn = Some(respawn);
            self.abort_respawn();
            return;
        }
        let spikes = minecraftoss_generator::feature::kinds::the_end::spikes_for_seed(self.worldgen.terrain.seed);
        let before = respawn.stage;
        let actions = respawn.tick(spikes.len());
        for action in actions {
            match action {
                Action::Beam(target) => {
                    for crystal in self.crystals.by_uuid(&respawn.crystals) {
                        crystal.beam_target = target;
                    }
                }
                Action::Growl => {
                    let pitch = 0.8 + self.end_random.next_float() * 0.3;
                    let (x, y, z) = crate::dragon_respawn::SUMMON_TARGET;
                    let at = glam::DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5);
                    self.end_sounds.push(minecraftoss_entities::world::MobSound { event: crate::ender_dragon::GROWL_SOUND.to_owned(), position: at, volume: crate::dragon_respawn::GROWL_VOLUME, pitch, category: "hostile" });
                }
                Action::SpikeBeam(index) => {
                    let spike = spikes[index];
                    for crystal in self.crystals.by_uuid(&respawn.crystals) {
                        crystal.beam_target = Some((spike.center_x, spike.height + 1, spike.center_z));
                    }
                }
                Action::RebuildSpike(index) => self.rebuild_spike(spikes[index], players),
                Action::Summon => {
                    self.fight.dragon_killed = false;
                    let uuid = self.create_new_dragon((0, 0, 0));
                    self.reset_spike_crystals();
                    for position in self.crystals.discard(&respawn.crystals) {
                        let blast = Blast { center: position.to_array(), power: crate::dragon_respawn::CRYSTAL_BLAST, fire: false };
                        self.blasts.push((blast, BlastSource::KeepBlocks));
                    }
                    self.fight_reports.push(FightReport::Respawned(uuid));
                    return;
                }
            }
        }
        if respawn.stage != before {
            self.fight_reports.push(FightReport::Respawning { stage: respawn.stage.name(), crystals: respawn.crystals.clone(), exit_portal: self.fight.exit_portal.unwrap_or((0, 0, 0)) });
        }
        self.respawn = Some(respawn);
    }

    fn rebuild_spike(&mut self, spike: minecraftoss_generator::feature::kinds::the_end::Spike, players: &[minecraftoss_entities::tempt::PlayerCandidate]) {
        let reach = crate::dragon_respawn::SPIKE_CLEARANCE;
        let blocks = &self.level.registries().blocks;
        let Some(air) = blocks.block_by_name("minecraft:air").map(|block| blocks.block(block).default_state()) else {
            return;
        };
        for x in spike.center_x - reach..=spike.center_x + reach {
            for y in spike.height - reach..=spike.height + reach {
                for z in spike.center_z - reach..=spike.center_z + reach {
                    let pos = minecraftoss_core::BlockPos::new(x, y, z);
                    if !self.level.registries().blocks.is_air(self.level.block(pos)) {
                        self.level.set_block_and_update(pos, air);
                    }
                }
            }
        }
        let center = [f64::from(spike.center_x) + 0.5, f64::from(spike.height), f64::from(spike.center_z) + 0.5];
        self.blasts.push((Blast { center, power: crate::dragon_respawn::PILLAR_BLAST, fire: false }, BlastSource::Nobody));
        self.explode_blasts(players);
        for destroyed in self.crystals.blast(glam::DVec3::from_array(center), crate::dragon_respawn::PILLAR_BLAST, None) {
            self.crystal_destroyed(destroyed);
        }
        let spawned = std::mem::take(&mut self.level.spawned);
        self.level.place_configured_feature(END_SPIKE, minecraftoss_core::BlockPos::new(spike.center_x, crate::dragon_respawn::SPIKE_ORIGIN_Y, spike.center_z));
        let placed = std::mem::replace(&mut self.level.spawned, spawned);
        let (tx, ty, tz) = crate::dragon_respawn::SUMMON_TARGET;
        for mut tag in placed {
            if let minecraftoss_core::nbt::Tag::Compound(map) = &mut tag {
                if map.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some(crate::end_crystal::END_CRYSTAL) {
                    map.insert("Invulnerable".to_owned(), minecraftoss_core::nbt::Tag::Byte(1));
                    map.insert("beam_target".to_owned(), minecraftoss_core::nbt::Tag::IntArray(vec![tx, ty, tz]));
                }
            }
            self.add_saved_entity(tag);
        }
    }

    pub fn dragon_killed(&mut self, origin: BlockPos, exit_portal_location: Option<BlockPos>, gateway: Option<i32>, egg: bool) -> BlockPos {
        use minecraftoss_generator::feature::World as _;
        let at = |(x, y, z): BlockPos| minecraftoss_core::BlockPos::new(x, y, z);
        let portal = self.level.spawn_exit_portal(at(origin), exit_portal_location.map(at), true);
        if let Some(pos) = gateway.map(end_gateway_position) {
            let pitch = (1.0 + (self.end_random.next_float() - self.end_random.next_float()) * 0.2) * 0.7;
            let center = glam::DVec3::new(f64::from(pos.0) + 0.5, f64::from(pos.1) + 0.5, f64::from(pos.2) + 0.5);
            self.end_sounds.push(minecraftoss_entities::world::MobSound { event: END_GATEWAY_SPAWN_SOUND.to_owned(), position: center, volume: 10.0, pitch, category: "blocks" });
            self.level.place_configured_feature(END_GATEWAY_DELAYED, at(pos));
        }
        if egg {
            let top = self.level.height_at(minecraftoss_core::chunk::HeightmapKind::MotionBlocking, origin.0, origin.2);
            let blocks = &self.level.registries().blocks;
            if let Some(state) = blocks.block_by_name(DRAGON_EGG).map(|block| blocks.block(block).default_state()) {
                self.level.set_block_and_update(minecraftoss_core::BlockPos::new(origin.0, top, origin.2), state);
            }
        }
        (portal.x, portal.y, portal.z)
    }

    pub fn dragon_views(&self) -> Vec<crate::ender_dragon::DragonView> {
        self.dragons.views()
    }

    pub fn end_hit(&mut self, target: crate::end_crystal::EndTarget, damage: crate::end_crystal::EndDamage) {
        match target {
            crate::end_crystal::EndTarget::Crystal(id) => {
                if let Some(destroyed) = self.crystals.hurt(id, &damage) {
                    self.crystal_destroyed(destroyed);
                }
            }
            crate::end_crystal::EndTarget::DragonPart { id, part } => {
                let mut events = crate::ender_dragon::DragonEvents::default();
                if let Some(dragon) = self.dragons.list.iter_mut().find(|dragon| dragon.id == id && part < 8) {
                    dragon.hurt_part(part, &damage, &mut events);
                }
                self.end_events(events);
            }
        }
    }

    fn crystal_destroyed(&mut self, destroyed: crate::end_crystal::Destroyed) {
        if destroyed.explodes {
            let blast = Blast { center: destroyed.position.to_array(), power: crate::end_crystal::EXPLOSION_POWER, fire: false };
            let source = destroyed.player.map_or(BlastSource::Nobody, BlastSource::Player);
            self.blasts.push((blast, source));
        }
        if !self.dragon_fight_level {
            return;
        }
        if self.respawn.as_ref().is_some_and(|respawn| destroyed.uuid.is_some_and(|uuid| respawn.crystals.contains(&uuid))) {
            self.abort_respawn();
            return;
        }
        self.update_crystal_count();
        let mut events = crate::ender_dragon::DragonEvents::default();
        let peaceful = self.level.difficulty == 0;
        let nearest = crate::ender_dragon::nearest_player(&self.end_players, destroyed.position.floor(), |_| !peaceful).map(|player| player.id);
        if let Some(dragon) = self.dragons.list.iter_mut().find(|dragon| Some(dragon.uuid) == self.fight_dragon) {
            dragon.crystal_destroyed(destroyed.id, destroyed.player.or(nearest), &mut events);
        }
        self.end_events(events);
    }

    pub fn crystal_views(&self) -> Vec<crate::end_crystal::CrystalView> {
        self.crystals.views()
    }

    pub fn tick_gateways(&mut self) {
        self.sync_pois();
        for (pos, age) in self.gateways.tick() {
            crate::end_gateway::write_age(&mut self.level, pos, age);
        }
    }

    pub fn trigger_gateway(&mut self, pos: BlockPos) -> bool {
        let Some(age) = self.gateways.trigger_cooldown(pos) else { return false };
        crate::end_gateway::write_age(&mut self.level, pos, age);
        true
    }

    pub fn set_gateway_exit(&mut self, pos: BlockPos, exit: BlockPos, exact: bool) {
        if let Some(age) = self.gateways.set_exit(pos, exit, exact) {
            crate::end_gateway::write_age(&mut self.level, pos, age);
        }
        minecraftoss_generator::feature::World::set_gateway_exit(&mut self.level, pos.0, pos.1, pos.2, exit, exact);
    }

    pub fn gateway_views(&self) -> Vec<crate::end_gateway::GatewayView> {
        self.gateways.views()
    }

    pub fn create_end_platform(&mut self, origin: BlockPos) {
        self.level.create_end_platform(minecraftoss_core::BlockPos::new(origin.0, origin.1, origin.2), true);
    }

    pub fn dormant_uuids(&self, kind: &str) -> Vec<[i32; 4]> {
        self.dormant
            .iter()
            .filter(|(group, _)| group.first().is_some_and(|root| root.kind == kind))
            .filter_map(|(_, tag)| match tag.get("UUID") {
                Some(minecraftoss_core::nbt::Tag::IntArray(uuid)) => uuid.as_slice().try_into().ok(),
                _ => None,
            })
            .collect()
    }

    pub fn discard_dormant(&mut self, uuid: [i32; 4]) {
        self.dormant.retain(|(_, tag)| !matches!(tag.get("UUID"), Some(minecraftoss_core::nbt::Tag::IntArray(found)) if found.as_slice() == uuid));
    }

    pub fn scan_dragon_fight(&mut self, origin: BlockPos, exit_portal_location: Option<BlockPos>) -> DragonFightScan {
        let at = |pos: BlockPos| minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        let origin = at(origin);
        let mut location = exit_portal_location.map(at);
        let previously_killed = self.level.has_active_exit_portal();
        if !previously_killed {
            match self.level.find_exit_portal(origin) {
                Some(found) => {
                    location.get_or_insert(found);
                }
                None => location = Some(self.level.spawn_exit_portal(origin, location, false)),
            }
        }
        let mut dragon_uuid = self.dragons.list.first().map(|dragon| dragon.uuid);
        let dragon_seen = dragon_uuid.is_some();
        let mut dragon_killed = !dragon_seen;
        if let Some(uuid) = dragon_uuid.filter(|_| !previously_killed) {
            self.dragons.list.retain(|dragon| dragon.uuid != uuid);
            dragon_uuid = None;
        }
        if dragon_uuid.is_some() {
            self.fight_dragon = dragon_uuid;
        }
        if !previously_killed {
            dragon_killed = false;
        }
        DragonFightScan { previously_killed, dragon_killed, dragon_seen, dragon_uuid, exit_portal_location: location.map(|pos| (pos.x, pos.y, pos.z)) }
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
        self.set_simulation_areas(&[center], distance);
    }

    pub fn set_simulation_areas(&mut self, centres: &[(i32, i32)], distance: i32) {
        let mut squares = std::collections::BTreeSet::new();
        for center in centres {
            for x in center.0 - distance..=center.0 + distance {
                for z in center.1 - distance..=center.1 + distance {
                    squares.insert((x, z));
                }
            }
        }
        let list = squares.into_iter().map(|(x, z)| ChunkPos::new(x, z)).filter(|&pos| self.level.chunk(pos).is_some()).collect();
        self.level.ticking_chunks = Some(list);
    }

    /// Keeps the level's default clock with the client's day time.
    pub fn set_time(&mut self, ticks: i64) {
        self.level.set_time(ticks);
    }

    pub fn set_clocks(&mut self, clocks: ClockTicks) {
        self.level.set_clock(OVERWORLD_CLOCK, clocks.overworld);
        self.level.set_clock(THE_END_CLOCK, clocks.the_end);
    }

    pub fn set_weather(&mut self, weather: &str) -> bool {
        const LASTING: i32 = 1_000_000_000;
        self.set_weather_for(weather, LASTING)
    }

    pub fn set_weather_for(&mut self, weather: &str, duration: i32) -> bool {
        match weather {
            "clear" => self.level.set_weather(duration, 0, false, false),
            "rain" => self.level.set_weather(0, duration, true, false),
            "thunder" => self.level.set_weather(0, duration, true, true),
            _ => return false,
        }
        true
    }

    pub fn weather_data(&self) -> Option<crate::level_storage::WeatherData> {
        self.level.sky.as_ref().filter(|sky| sky.can_have_weather).map(|sky| crate::level_storage::WeatherData {
            clear_weather_time: sky.weather.clear_time,
            rain_time: sky.weather.rain_time,
            thunder_time: sky.weather.thunder_time,
            raining: sky.weather.raining,
            thundering: sky.weather.thundering,
        })
    }

    pub fn set_weather_data(&mut self, data: &crate::level_storage::WeatherData) {
        let Some(sky) = &mut self.level.sky else { return };
        let weather = &mut sky.weather;
        weather.clear_time = data.clear_weather_time;
        weather.rain_time = data.rain_time;
        weather.thunder_time = data.thunder_time;
        weather.raining = data.raining;
        weather.thundering = data.thundering;
        if sky.can_have_weather && data.raining {
            weather.rain_level = 1.0;
            if data.thundering {
                weather.thunder_level = 1.0;
            }
        }
    }

    pub fn apply_game_rules(&mut self, rules: &crate::game_rules::GameRules) {
        let level = &mut self.level;
        level.random_tick_speed = rules.int("random_tick_speed");
        level.block_drops = rules.bool("block_drops");
        level.entity_drops = rules.bool("entity_drops");
        level.max_snow_accumulation_height = rules.int("max_snow_accumulation_height");
        level.fire_spread_radius = rules.int("fire_spread_radius_around_player");
        level.water_source_conversion = rules.bool("water_source_conversion");
        level.lava_source_conversion = rules.bool("lava_source_conversion");
        level.tnt_explodes = rules.bool("tnt_explodes");
        level.tnt_explosion_drop_decay = rules.bool("tnt_explosion_drop_decay");
        self.block_explosion_drop_decay = rules.bool("block_explosion_drop_decay");
        level.set_advance_weather(rules.bool("advance_weather"));
        self.mobs.set_mob_griefing(rules.bool("mob_griefing"));
        self.mobs.set_mob_drops(rules.bool("mob_drops"));
        self.dragon_drops = rules.bool("mob_drops");
        if let Some(natural) = &mut self.level.natural_spawning {
            natural.spawner_blocks_work = rules.bool("spawner_blocks_work");
        }
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
        self.level.tick_spawners();
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
            Some((_, data)) => data.stack.components = crate::item_nbt::components_from_text(components),
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
                on_fire: minecraftoss_world::level::burning::is_on_fire(e),
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

    fn collect_block_entity_changes(&mut self) {
        for (x, y, z) in self.level.take_changed_block_entities() {
            let chunk = self.level.chunk(minecraftoss_core::BlockPos::new(x, y, z).chunk());
            let Some(tag) = chunk.and_then(|chunk| chunk.block_entities.entities.get(&(x, y, z))) else {
                self.gateways.sync((x, y, z), None);
                if chunk.is_some() {
                    self.block_entity_changes.push(((x, y, z), None));
                }
                continue;
            };
            self.gateways.sync((x, y, z), Some(tag));
            if tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str).is_some_and(|kind| !crate::containers::owns_block_entity(kind)) {
                self.block_entity_changes.push(((x, y, z), Some(tag.clone())));
            }
        }
    }

    pub fn take_block_entity_changes(&mut self) -> Vec<(BlockPos, Option<minecraftoss_core::nbt::Tag>)> {
        self.collect_block_entity_changes();
        std::mem::take(&mut self.block_entity_changes)
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
    PlayerMines(Vec<(BlockPos, HowBroken)>),
    UseBlock { pos: BlockPos, facing: &'static str },
    BoneMeal { pos: BlockPos, face: &'static str },
    Attack(BlockPos),
    ItemOnBlock { player: u64, pos: BlockPos, action: ItemOnBlock },
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
    Summon { player: u64, kind: String, position: [f64; 3], nbt: Option<minecraftoss_core::nbt::Tag>, y_rot: f32 },
    HatchEgg { player: u64, kind: String, position: [f64; 3], y_rot: f32 },
    SpawnerEgg { player: u64, pos: BlockPos, kind: String },
    /// A player's hit (`attack`) or item use on a mob, with a copy of its
    /// inventory.
    MobAction { player: u64, hit: minecraftoss_entities::world::MobHit, attack: Option<minecraftoss_entities::world::PlayerAttack>, inventory: Box<minecraftoss_player::inventory::Inventory>, selected: usize, infinite: bool },
    MobBlast { hit: minecraftoss_entities::world::MobHit, damage: f32, center: [f64; 3], impact: f32, player: Option<u64>, slow_ticks: i32 },
    WeaponBlast { center: [f64; 3], power: f32 },
    Blast(Blast),
    RespawnPointBlast { blast: Blast, shield: Option<BlockPos> },
    DeflectFireball { id: u64, look: [f64; 3], player: u64, by_attack: bool },
    PushFireball { id: u64, push: [f64; 3] },
    /// An operation on the player's trading screen.
    Merchant { player: u64, op: MerchantOp, inventory: Box<minecraftoss_player::inventory::Inventory>, selected: usize, feet: [f64; 3] },
    /// One server tick, with the player's state for it.
    Tick(Box<TickInput>, Vec<bool>),
    Throw(Box<crate::thrown::Thrown>),
    ThrowTrident(Box<crate::trident::ThrownTrident>),
    ThrowEye { owner: u64, throw: Box<crate::ender_eye::EyeThrow> },
    BottleBreath { player: u64, feet: glam::DVec3, height: f64 },
    Save,
    SetWeather { weather: &'static str, duration: i32 },
    WeatherData(crate::level_storage::WeatherData),
    GameRules(Box<crate::game_rules::GameRules>),
    PlayerUuid { player: u64, uuid: u128 },
    PlayerLeft(u64),
    Job(Job),
    EndHit { target: crate::end_crystal::EndTarget, damage: crate::end_crystal::EndDamage },
}

pub type Job = Box<dyn FnOnce(&mut ServerSim) -> Box<dyn std::any::Any + Send> + Send>;

pub const ENDER_DRAGON: &str = "minecraft:ender_dragon";

pub const DRAGON_EGG: &str = "minecraft:dragon_egg";
pub const END_GATEWAY_DELAYED: &str = "minecraft:end_gateway_delayed";
pub const END_GATEWAY_SPAWN_SOUND: &str = "minecraft:block.end_gateway.spawn";
pub const GATEWAY_RING: f64 = 96.0;
pub const GATEWAY_Y: i32 = 75;

pub fn end_gateway_position(index: i32) -> BlockPos {
    let angle = 2.0 * (-std::f64::consts::PI + 0.157_079_632_679_489_66 * f64::from(index));
    ((GATEWAY_RING * angle.cos()).floor() as i32, GATEWAY_Y, (GATEWAY_RING * angle.sin()).floor() as i32)
}

pub const END_SPIKE: &str = "minecraft:end_spike";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FightLink {
    pub dragon: Option<[i32; 4]>,
    pub previously_killed: bool,
    pub dragon_killed: bool,
    pub exit_portal: Option<BlockPos>,
    pub ticking: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FightReport {
    Respawning { stage: &'static str, crystals: Vec<[i32; 4]>, exit_portal: BlockPos },
    Aborted,
    Respawned([i32; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DragonFightScan {
    pub previously_killed: bool,
    pub dragon_killed: bool,
    pub dragon_seen: bool,
    pub dragon_uuid: Option<[i32; 4]>,
    pub exit_portal_location: Option<BlockPos>,
}

pub fn advance_weather_timers(data: &mut crate::level_storage::WeatherData, random: &mut impl minecraftoss_core::random::RandomSource) {
    let mut weather = minecraftoss_world::level::time::Weather {
        clear_time: data.clear_weather_time,
        rain_time: data.rain_time,
        thunder_time: data.thunder_time,
        raining: data.raining,
        thundering: data.thundering,
        ..Default::default()
    };
    weather.advance_timers(random);
    *data = crate::level_storage::WeatherData { clear_weather_time: weather.clear_time, rain_time: weather.rain_time, thunder_time: weather.thunder_time, raining: weather.raining, thundering: weather.thundering };
}

pub fn weather_parameters(weather: &str, duration: i32) -> Option<crate::level_storage::WeatherData> {
    let (clear_weather_time, rain_time, raining, thundering) = match weather {
        "clear" => (duration, 0, false, false),
        "rain" => (0, duration, true, false),
        "thunder" => (0, duration, true, true),
        _ => return None,
    };
    Some(crate::level_storage::WeatherData { clear_weather_time, rain_time, thunder_time: rain_time, raining, thundering })
}

pub const OVERWORLD_CLOCK: &str = "minecraft:overworld";
pub const THE_END_CLOCK: &str = "minecraft:the_end";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClockTicks {
    pub overworld: i64,
    pub the_end: i64,
}

pub struct TickPlayer {
    pub id: u64,
    pub feet: [f64; 3],
    pub alive: bool,
    pub main_hand: Option<String>,
    pub pickup: Option<(Box<minecraftoss_player::inventory::Inventory>, usize)>,
}

/// The client state a server tick reads.
pub struct TickInput {
    pub clocks: ClockTicks,
    pub players: Vec<TickPlayer>,
    pub difficulty: i32,
    pub simulation_centres: Vec<(i32, i32)>,
    pub simulation_distance: i32,
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
    pub player_hurts: Vec<(u64, Option<u64>, &'static str)>,
    /// The `spawn_mobs` game rule.
    pub spawn_mobs: bool,
}

impl TickInput {
    pub fn remove_player(&mut self, player: u64) {
        self.players.retain(|p| p.id != player);
        self.mob_players.retain(|p| p.id != player);
        self.mob_views.retain(|(id, _)| *id != player);
        self.mob_vitals.retain(|(id, _)| *id != player);
        self.player_hurts.retain(|(id, _, _)| *id != player);
    }
}

const PLAYER_HEIGHT: f64 = 1.8;
const TRIDENT_IDS: i32 = 0x3000_0000;
pub type PickedStack = (i32, [f64; 3], String, i32, Option<String>);
const TNT_BLAST_RADIUS: f32 = 4.0;
const STUN_SLOWNESS_AMPLIFIER: i32 = 3;

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
    pub egg_teleports: Vec<(BlockPos, BlockPos)>,
    pub block_entities: Vec<(BlockPos, Option<minecraftoss_core::nbt::Tag>)>,
    pub destroyed: Vec<(BlockPos, Block)>,
    /// Entity snapshots, after a tick.
    pub entities: Option<EntitySnapshot>,
    /// Experience orbs the player took: (entity ID, position, value).
    pub orbs_taken: Vec<(u64, i32, [f64; 3], i32)>,
    /// The mobs the player tracks, after a tick.
    pub mobs: Option<Box<minecraftoss_entities::world::EntityWorld>>,
    /// What the player's mob actions did, in order.
    pub mob_results: Vec<(u64, MobResult)>,
    /// The trading screen's updates, in order.
    pub merchant: Vec<(u64, MerchantUpdate)>,
    /// Mobs' hits on the players, in order.
    pub player_hits: Vec<minecraftoss_entities::world::PlayerHit>,
    /// Splash potions that broke near players, in order.
    pub player_splashes: Vec<(u64, minecraftoss_entities::world::PlayerSplash)>,
    /// Where splash potions broke (their sound and colour).
    pub potion_breaks: Vec<minecraftoss_entities::world::PotionBreak>,
    pub player_potions: Vec<crate::potions::PlayerPotionHit>,
    pub players_struck: Vec<crate::thrown::PlayerStruck>,
    pub players_pushed: Vec<(u64, glam::DVec3)>,
    /// Creeper blasts this tick (`ClientboundExplodePacket`).
    pub explosions: Vec<minecraftoss_entities::creeper::CreeperExplosion>,
    pub explosion_effects: Vec<minecraftoss_world::level::explosion::ExplosionEffect>,
    pub level_events: Vec<(i32, (i32, i32, i32), i32)>,
    /// Sounds mobs made this tick.
    pub mob_sounds: Vec<minecraftoss_entities::world::MobSound>,
    /// Items the player picked up: (entity ID, position, item, count, components).
    pub picked: Vec<(u64, PickedStack)>,
    /// What each `/summon` did: the type it made, or why it failed.
    pub summoned: Vec<(u64, Result<String, String>)>,
    pub command_summons: Vec<(u64, Result<String, String>)>,
    /// Positions where bone meal was used.
    pub bone_meal_used: Vec<BlockPos>,
    pub item_uses: Vec<(u64, bool)>,
    pub pearl_teleports: Vec<(u64, [f64; 3])>,
    pub pearl_gateways: Vec<(u64, BlockPos)>,
    pub thrown_hits: Vec<crate::thrown::ThrownHit>,
    pub weather: Option<WeatherView>,
    pub weather_data: Option<crate::level_storage::WeatherData>,
    /// How many commands the server has handled so far.
    pub handled: u64,
    /// The last tick's phases, light solves and total milliseconds.
    pub tick_phases: Option<([f64; 6], (u32, f64), f64)>,
    pub jobs: Vec<Box<dyn std::any::Any + Send>>,
    pub dragons_killed: Vec<[i32; 4]>,
    pub fight_reports: Vec<FightReport>,
    pub tick_nanos: Vec<u64>,
    pub eye_answers: Vec<crate::eye_of_ender::EyeAnswer>,
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
    ticks_sent: u64,
    ticks_answered: u64,
}

impl ServerHandle {
    pub fn spawn(sim: ServerSim) -> Self {
        let states = sim.states.clone();
        let count = sim.level.registries().blocks.state_count();
        let uses: Vec<bool> = (0..count).map(|i| sim.level.handles_use(BlockStateId(i as u16))).collect();
        let egg = sim.level.registries().blocks.block_by_name(crate::dragon_egg::DRAGON_EGG);
        let is_egg = |i: usize| egg == Some(sim.level.registries().blocks.block_of(BlockStateId(i as u16)));
        let uses: Vec<bool> = uses.into_iter().enumerate().map(|(i, used)| used || is_egg(i)).collect();
        let attacks: Vec<bool> = (0..count).map(|i| sim.level.handles_attack(BlockStateId(i as u16)) || is_egg(i)).collect();
        let (commands, receiver) = std::sync::mpsc::channel::<Command>();
        let (sender, outputs) = std::sync::mpsc::channel::<Output>();
        let thread = std::thread::Builder::new()
            .name("Server thread".into())
            .spawn(move || server_loop(sim, receiver, sender))
            .expect("server thread starts");
        Self { commands: Some(commands), outputs, states, uses: Arc::new(uses), attacks: Arc::new(attacks), sent: 0, waited: Vec::new(), thread: Some(thread), ticks_sent: 0, ticks_answered: 0 }
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

    pub fn player_mines(&mut self, broken: &[(BlockPos, HowBroken)]) {
        if !broken.is_empty() {
            self.send(Command::PlayerMines(broken.to_vec()));
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

    pub fn item_on_block(&mut self, player: u64, pos: BlockPos, action: ItemOnBlock) {
        self.send(Command::ItemOnBlock { player, pos, action });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_item(&mut self, item: &str, count: i32, components: Option<&str>, position: [f64; 3], velocity: [f64; 3], pickup_delay: i32, age: i32) {
        self.send(Command::SpawnItem { item: item.to_owned(), count, components: components.map(str::to_owned), position, velocity, pickup_delay, age });
    }

    pub fn keep_dropped_contents(&mut self, pos: BlockPos, item: &str, components: &str) {
        self.send(Command::KeepDroppedContents { pos, item: item.to_owned(), components: components.to_owned() });
    }

    pub fn tick(&mut self, input: TickInput, normal: Vec<bool>) {
        if normal.is_empty() {
            return;
        }
        self.ticks_sent += normal.len() as u64;
        self.send(Command::Tick(Box::new(input), normal));
    }

    pub fn ticks_in_flight(&self) -> u64 {
        self.ticks_sent - self.ticks_answered
    }

    pub fn set_recipe_book(&mut self, recipes: Arc<minecraftoss_player::crafting::RecipeBook>) {
        self.send(Command::RecipeBook(recipes));
    }

    pub fn load_loot(&mut self, jar: std::path::PathBuf, seed: u64) {
        self.send(Command::LootTables { jar, seed });
    }

    /// An operation on the player's trading screen, with a copy of the
    /// player's inventory.
    pub fn merchant(&mut self, player: u64, op: MerchantOp, inventory: &minecraftoss_player::inventory::Inventory, selected: usize, feet: [f64; 3]) {
        self.send(Command::Merchant { player, op, inventory: Box::new(inventory.clone()), selected, feet });
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
    pub fn summon(&mut self, player: u64, kind: String, position: [f64; 3], nbt: Option<minecraftoss_core::nbt::Tag>) {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as i64);
        let y_rot = minecraftoss_core::random::LegacyRandom::new(nanos).next_f32() * (std::f64::consts::PI * 2.0) as f32;
        self.send(Command::Summon { player, kind, position, nbt, y_rot });
    }

    pub fn set_weather(&mut self, weather: &'static str, duration: i32) {
        self.send(Command::SetWeather { weather, duration });
    }

    pub fn set_game_rules(&mut self, rules: crate::game_rules::GameRules) {
        self.send(Command::GameRules(Box::new(rules)));
    }

    pub fn set_player_uuid(&mut self, player: u64, uuid: u128) {
        self.send(Command::PlayerUuid { player, uuid });
    }

    pub fn set_weather_data(&mut self, data: crate::level_storage::WeatherData) {
        self.send(Command::WeatherData(data));
    }

    pub fn hatch_egg(&mut self, player: u64, kind: String, position: [f64; 3], y_rot: f32) {
        self.send(Command::HatchEgg { player, kind, position, y_rot });
    }

    pub fn spawner_egg(&mut self, player: u64, pos: BlockPos, kind: String) {
        self.send(Command::SpawnerEgg { player, pos, kind });
    }

    /// Hands a player's hit or use on a mob to the server; its result
    /// arrives in an output.
    pub fn mob_action(&mut self, player: u64, hit: minecraftoss_entities::world::MobHit, attack: Option<minecraftoss_entities::world::PlayerAttack>, inventory: &minecraftoss_player::inventory::Inventory, selected: usize, infinite: bool) {
        self.send(Command::MobAction { player, hit, attack, inventory: Box::new(inventory.clone()), selected, infinite });
    }

    pub fn mob_blast(&mut self, hit: minecraftoss_entities::world::MobHit, damage: f32, center: [f64; 3], impact: f32, player: Option<u64>, slow_ticks: i32) {
        self.send(Command::MobBlast { hit, damage, center, impact, player, slow_ticks });
    }

    pub fn weapon_blast(&mut self, center: [f64; 3], power: f32) {
        self.send(Command::WeaponBlast { center, power });
    }

    pub fn blast(&mut self, blast: Blast) {
        self.send(Command::Blast(blast));
    }

    pub fn respawn_point_blast(&mut self, blast: Blast, shield: Option<BlockPos>) {
        self.send(Command::RespawnPointBlast { blast, shield });
    }

    pub fn deflect_fireball(&mut self, id: u64, look: [f64; 3], player: u64, by_attack: bool) {
        self.send(Command::DeflectFireball { id, look, player, by_attack });
    }

    pub fn push_fireball(&mut self, id: u64, push: [f64; 3]) {
        self.send(Command::PushFireball { id, push });
    }

    pub fn throw(&mut self, thrown: crate::thrown::Thrown) {
        self.send(Command::Throw(Box::new(thrown)));
    }

    pub fn throw_trident(&mut self, trident: crate::trident::ThrownTrident) {
        self.send(Command::ThrowTrident(Box::new(trident)));
    }

    pub fn throw_eye(&mut self, owner: u64, throw: crate::ender_eye::EyeThrow) {
        self.send(Command::ThrowEye { owner, throw: Box::new(throw) });
    }

    pub fn bottle_breath(&mut self, player: u64, feet: glam::DVec3, height: f64) {
        self.send(Command::BottleBreath { player, feet, height });
    }

    pub fn player_left(&mut self, player: u64) {
        self.send(Command::PlayerLeft(player));
    }

    pub fn end_hit(&mut self, target: crate::end_crystal::EndTarget, damage: crate::end_crystal::EndDamage) {
        self.send(Command::EndHit { target, damage });
    }

    pub fn save(&mut self) {
        self.send(Command::Save);
    }

    pub fn run_job(&mut self, job: Job) {
        self.send(Command::Job(job));
    }

    /// Outputs the server has produced since the last call.
    pub fn poll(&mut self) -> Vec<Output> {
        let mut outputs = std::mem::take(&mut self.waited);
        outputs.extend(self.outputs.try_iter());
        self.ticks_answered += outputs.iter().map(|out| out.tick_nanos.len() as u64).sum::<u64>();
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
        let mut batch: Option<(TickInput, std::collections::VecDeque<bool>)> = None;
        loop {
            let Some(command) = next.take().or_else(|| if out.tick_nanos.is_empty() || batch.is_some() { commands.try_recv().ok() } else { None }) else {
                let Some((input, flags)) = batch.as_mut() else {
                    break;
                };
                let normal = flags.pop_front().unwrap_or(true);
                run_tick(&mut sim, input, normal, &mut out);
                if flags.is_empty() {
                    finish_batch(&mut sim, input, &mut out);
                    batch = None;
                }
                continue;
            };
            handled += 1;
            match command {
                Command::LoadChunk(chunk) => sim.load_chunk(&chunk),
                Command::UnloadChunk(pos) => sim.unload_chunk(pos),
                Command::PlayerEdit { pos, block, edit } => sim.player_edit_block(pos, block.as_ref(), edit),
                Command::PlayerBreaks(positions) => sim.player_breaks(&positions),
                Command::PlayerMines(broken) => sim.player_mines(&broken),
                Command::UseBlock { pos, facing } => {
                    sim.use_block(pos, facing);
                    out.egg_teleports.extend(sim.take_egg_teleports());
                }
                Command::BoneMeal { pos, face } => {
                    if sim.bone_meal(pos, face) {
                        out.bone_meal_used.push(pos);
                    }
                }
                Command::Attack(pos) => {
                    sim.attack_block(pos);
                    out.egg_teleports.extend(sim.take_egg_teleports());
                }
                Command::ItemOnBlock { player, pos, action } => {
                    out.item_uses.push((player, sim.item_on_block(pos, action)));
                }
                Command::SpawnItem { item, count, components, position, velocity, pickup_delay, age } => {
                    sim.spawn_item(&item, count, components.as_deref(), position, velocity, pickup_delay, age);
                }
                Command::KeepDroppedContents { pos, item, components } => sim.keep_dropped_contents(pos, &item, &components),
                Command::RecipeBook(recipes) => sim.set_recipe_book(recipes),
                Command::LootTables { jar, seed } => sim.load_loot(&jar, seed),
                Command::SummonOrb { position, value } => sim.summon_orb(position, value),
                Command::AwardExperience { position, amount } => sim.award_experience(position, amount),
                Command::Summon { player, kind, position, nbt, y_rot } => {
                    out.command_summons.push((player, sim.summon(&kind, position, nbt.as_ref(), y_rot).map(|()| kind)));
                }
                Command::HatchEgg { player, kind, position, y_rot } => {
                    out.summoned.push((player, sim.hatch_egg(&kind, position, y_rot).map(|()| kind)));
                }
                Command::SpawnerEgg { player, pos, kind } => {
                    out.summoned.push((player, sim.spawner_egg(pos, &kind).map(|()| kind)));
                }
                Command::MobAction { player, hit, attack, inventory, selected, infinite } => {
                    out.mob_results.push((player, sim.mob_action(player, hit, attack, *inventory, selected, infinite)));
                }
                Command::MobBlast { hit, damage, center, impact, player, slow_ticks } => sim.mob_blast(hit, damage, center, impact, player, slow_ticks),
                Command::WeaponBlast { center, power } => sim.weapon_blast(center, power),
                Command::Blast(blast) => sim.blast(blast),
                Command::RespawnPointBlast { blast, shield } => sim.respawn_point_blast(blast, shield),
                Command::DeflectFireball { id, look, player, by_attack } => {
                    sim.deflect_fireball(id, look, player, by_attack);
                }
                Command::PushFireball { id, push } => {
                    sim.push_fireball(id, push);
                }
                Command::Merchant { player, op, inventory, selected, feet } => {
                    out.merchant.push((player, sim.merchant(player, op, *inventory, selected, feet)));
                }
                Command::Throw(thrown) => sim.throw(*thrown),
                Command::ThrowTrident(trident) => sim.throw_trident(*trident),
                Command::ThrowEye { owner, throw } => {
                    out.item_uses.push((owner, sim.throw_eye(owner, &throw)));
                    out.eye_answers.extend(sim.take_eye_answers());
                }
                Command::BottleBreath { player, feet, height } => out.item_uses.push((player, sim.bottle_breath(feet, height))),
                Command::SetWeather { weather, duration } => {
                    sim.set_weather_for(weather, duration);
                }
                Command::PlayerUuid { player, uuid } => sim.set_player_uuid(player, uuid),
                Command::PlayerLeft(player) => player_left(&mut sim, batch.as_mut().map(|(input, _)| input), player),
                Command::WeatherData(data) => sim.set_weather_data(&data),
                Command::GameRules(rules) => sim.apply_game_rules(&rules),
                Command::Job(job) => out.jobs.push(job(&mut sim)),
                Command::EndHit { target, damage } => sim.end_hit(target, damage),
                Command::Save => {
                    sim.ticks_since_save = 0;
                    sim.save_all_entities();
                }
                Command::Tick(_, normal) if normal.is_empty() => {}
                Command::Tick(input, normal) => match batch.as_mut() {
                    Some((running, flags)) => {
                        *running = *input;
                        flags.extend(normal);
                    }
                    None => {
                        let mut input = *input;
                        run_tick(&mut sim, &mut input, normal[0], &mut out);
                        let flags: std::collections::VecDeque<bool> = normal.into_iter().skip(1).collect();
                        if flags.is_empty() {
                            finish_batch(&mut sim, &input, &mut out);
                        } else {
                            batch = Some((input, flags));
                        }
                    }
                },
            }
        }
        out.changes = sim.take_changes();
        out.block_entities = sim.take_block_entity_changes();
        out.destroyed = sim.take_destroyed();
        out.handled = handled;
        if outputs.send(out).is_err() {
            break;
        }
    }
    // The client closed the world: its entities are saved.
    sim.save_all_entities();
}

fn run_tick(sim: &mut ServerSim, input: &mut TickInput, normal: bool, out: &mut Output) {
    let started = std::time::Instant::now();
    sim.ticks_since_save += 1;
    if sim.ticks_since_save >= 6000 {
        sim.ticks_since_save = 0;
        sim.save_all_entities();
    }
    sim.set_clocks(input.clocks);
    sim.player_feet.extend(input.players.iter().map(|p| (p.id, p.feet)));
    let alive: Vec<[f64; 3]> = input.players.iter().filter(|p| p.alive).map(|p| p.feet).collect();
    sim.set_players(&alive, input.difficulty);
    sim.level.living_players = input.mob_players.iter().filter(|p| p.alive && !p.spectator).map(|p| (p.position.to_array(), f64::from(p.eye_height))).collect();
    sim.set_simulation_areas(&input.simulation_centres, input.simulation_distance);
    if let Some(natural) = &mut sim.level.natural_spawning {
        natural.spawn_mobs = input.spawn_mobs;
    }
    if normal {
        sim.prepare_spawning(&input.mob_players);
        sim.tick();
    }
    let mobs_started = std::time::Instant::now();
    if normal {
        sim.take_spawned();
        sim.despawn_mobs(&input.mob_players, input.difficulty);
    }
    for player in &input.players {
        sim.mobs.tick_player(player.id);
    }
    for (player, source, kind) in std::mem::take(&mut input.player_hurts) {
        sim.mobs.player_hurt(player, source, kind);
    }
    sim.mobs.set_player_views(input.mob_views.clone());
    sim.mobs.set_player_vitals(input.mob_vitals.clone());
    for player in &input.players {
        sim.mobs.set_player_main_hand(player.id, player.main_hand.as_deref());
    }
    let bodies: Vec<_> = input.mob_players.iter().map(crate::thrown::PlayerBody::of).collect();
    if normal {
        sim.tick_mobs(&input.mob_players, input.bright_outside);
        sim.tick_clouds(&bodies);
        sim.tick_thrown(&bodies);
        sim.tick_tridents(&bodies);
        sim.tick_eyes();
        sim.tick_end(&input.mob_players);
        sim.tick_gateways();
    }
    out.player_hits.extend(sim.take_end_hits());
    out.mob_sounds.extend(sim.take_end_sounds());
    out.dragons_killed.extend(sim.take_dragons_killed());
    out.fight_reports.extend(sim.take_fight_reports());
    out.pearl_teleports.extend(sim.take_pearl_teleports());
    out.pearl_gateways.extend(sim.take_pearl_gateways());
    out.thrown_hits.extend(sim.take_thrown_hits());
    if normal {
        sim.spawn_trade_experience();
    }
    out.merchant.extend(sim.check_merchant(&input.mob_players));
    out.player_hits.extend(sim.mobs.take_player_hits());
    let witch_splashes = sim.mobs.take_player_splashes();
    out.player_potions.extend(witch_splashes.iter().map(|(player, splash)| crate::splash::of_witch(*player, splash)));
    out.player_potions.extend(sim.take_player_potions());
    out.player_splashes.extend(witch_splashes);
    out.players_struck.extend(sim.take_players_struck());
    out.players_pushed.extend(sim.take_players_pushed());
    out.potion_breaks.extend(sim.take_potion_breaks());
    out.explosions.extend(sim.take_explosions());
    out.explosion_effects.extend(sim.take_explosion_effects());
    out.level_events.extend(sim.level.level_events.drain(..).map(|(id, pos, data)| (id, (pos.x, pos.y, pos.z), data)));
    out.mob_sounds.extend(sim.mobs.take_sounds());
    out.mob_sounds.extend(sim.level.sounds.drain(..).map(|s| minecraftoss_entities::world::MobSound { event: s.event.to_owned(), position: glam::DVec3::from_array(s.pos), volume: s.volume, pitch: s.pitch, category: "ambient" }));
    sim.level.last_tick_phases[5] += mobs_started.elapsed().as_secs_f64() * 1000.0;
    for player in &mut input.players {
        let (id, feet) = (player.id, player.feet);
        let pickup_feet = player.pickup.as_ref().map(|_| feet);
        if let Some((inventory, selected)) = player.pickup.as_mut() {
            let selected = *selected;
            let recipes = inventory.recipes.clone();
            let picked = sim.pickup(feet, |item, count, components| {
                let stack = make_stack(&recipes, item, count, components);
                match inventory.add_item(stack, selected) {
                    None => count,
                    Some(rest) => count - i32::from(rest.count),
                }
            });
            out.picked.extend(picked.into_iter().map(|picked| (id, picked)));
            if let Some(body) = bodies.iter().find(|body| body.id == id) {
                let tridents = sim.pickup_tridents(body, &bodies, |stack| {
                    let mut stack = stack.clone();
                    stack.max = stack.max.min(recipes.max_stack(&stack.id));
                    inventory.add_item(stack, selected).is_none()
                });
                out.picked.extend(tridents.into_iter().map(|picked| (id, picked)));
            }
        }
        out.orbs_taken.extend(sim.take_experience(id, pickup_feet).map(|(orb, at, value)| (id, orb, at, value)));
    }
    out.tick_nanos.push(started.elapsed().as_nanos() as u64);
}

fn player_left(sim: &mut ServerSim, running: Option<&mut TickInput>, player: u64) {
    if let Some(input) = running {
        input.remove_player(player);
    }
    sim.player_left(player);
}

fn finish_batch(sim: &mut ServerSim, input: &TickInput, out: &mut Output) {
    out.mobs = Some(Box::new(sim.tracked_mobs(input.tracking.0, input.tracking.1)));
    out.weather = Some(sim.weather(input.players.first().filter(|p| p.alive).map(|p| p.feet)));
    out.weather_data = sim.weather_data();
    out.entities = Some(EntitySnapshot { items: sim.items(), tnt: sim.primed_tnt(), falling: sim.falling_blocks(), orbs: sim.orbs(), thrown_views: sim.thrown_views(), clouds: sim.cloud_views(), tridents: sim.trident_views(), eyes: sim.eye_snapshot(), crystals: sim.crystal_views(), dragons: sim.dragon_views(), dragon_fireballs: sim.dragon_fireballs.views(), breath_bursts: sim.dragon_fireballs.take_bursts(), gateways: sim.gateway_views() });
    let (solves, ms) = sim.level.light_solves.replace((0, 0.0));
    let total = out.tick_nanos.last().map_or(0.0, |&nanos| nanos as f64 / 1_000_000.0);
    out.tick_phases = Some((sim.level.last_tick_phases, (solves, ms), total));
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

    #[test]
    fn command_weather_sets_rain_for_its_duration() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        assert!(!server.set_weather_for("snow", 100));
        assert!(server.set_weather_for("rain", 150));
        for _ in 0..120 {
            server.tick();
        }
        let raining = server.weather(None);
        assert!(raining.rain_level > 0.99 && raining.thunder_level == 0.0, "{raining:?}");
        for _ in 0..140 {
            server.tick();
        }
        assert_eq!(server.weather(None).rain_level, 0.0);
        let mut rules = crate::game_rules::GameRules::default();
        rules.set("random_tick_speed", "0").unwrap();
        rules.set("mob_griefing", "false").unwrap();
        rules.set("tnt_explodes", "false").unwrap();
        rules.set("advance_weather", "false").unwrap();
        server.apply_game_rules(&rules);
        assert_eq!(server.level.random_tick_speed, 0);
        assert!(!server.level.tnt_explodes);
        assert!(!server.mobs.mob_griefing());
        assert!(server.level.block_drops);
        assert!(server.set_weather_for("thunder", 5));
        for _ in 0..120 {
            server.tick();
        }
        let stormy = server.weather(None);
        assert!(stormy.rain_level > 0.99 && stormy.thunder_level > 0.99, "{stormy:?}");
        server.apply_game_rules(&crate::game_rules::GameRules::default());
        assert_eq!(server.level.random_tick_speed, 3);
        assert!(server.mobs.mob_griefing());
    }

    #[test]
    fn saved_weather_comes_back_already_raining_and_keeps_counting_down() {
        use crate::level_storage::WeatherData;
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen.clone(), states.clone(), "minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        assert!(server.set_weather_for("thunder", 900));
        for _ in 0..150 {
            server.tick();
        }
        let saved = server.weather_data().expect("the overworld has weather");
        assert_eq!(saved, WeatherData { clear_weather_time: 0, rain_time: 750, thunder_time: 750, raining: true, thundering: true });
        let mut loaded = ServerSim::new(worldgen.clone(), states.clone(), "minecraft:overworld");
        loaded.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        assert_eq!(loaded.weather(None), WeatherView::default());
        loaded.set_weather_data(&saved);
        let stormy = loaded.weather(None);
        assert_eq!((stormy.rain_level, stormy.thunder_level), (1.0, 1.0));
        loaded.tick();
        assert_eq!(loaded.weather_data().map(|w| (w.rain_time, w.thunder_time, w.raining)), Some((749, 749, true)));
        let mut clear = ServerSim::new(worldgen, states, "minecraft:overworld");
        clear.set_weather_data(&WeatherData { clear_weather_time: 300, ..WeatherData::default() });
        assert_eq!(clear.weather(None), WeatherView::default());
        assert_eq!(clear.weather_data().map(|w| w.clear_weather_time), Some(300));
    }

    #[test]
    fn weather_timers_run_outside_a_level_like_the_overworld_runs_them() {
        use crate::level_storage::WeatherData;
        let mut random = minecraftoss_core::random::LegacyRandom::new(5);
        let mut weather = WeatherData { rain_time: 2, raining: true, thunder_time: 4, ..WeatherData::default() };
        advance_weather_timers(&mut weather, &mut random);
        assert_eq!(weather, WeatherData { rain_time: 1, raining: true, thunder_time: 3, ..WeatherData::default() });
        advance_weather_timers(&mut weather, &mut random);
        assert_eq!((weather.rain_time, weather.raining, weather.thunder_time), (0, false, 2));
        advance_weather_timers(&mut weather, &mut random);
        assert!((12_000..=180_000).contains(&weather.rain_time), "{weather:?}");
        assert!(!weather.raining);
        let mut cleared = WeatherData { clear_weather_time: 2, rain_time: 50, raining: true, thundering: true, ..WeatherData::default() };
        advance_weather_timers(&mut cleared, &mut random);
        assert_eq!(cleared, WeatherData { clear_weather_time: 1, rain_time: 0, thunder_time: 0, raining: false, thundering: false });
        let mut fresh = WeatherData::default();
        advance_weather_timers(&mut fresh, &mut random);
        assert!((12_000..=180_000).contains(&fresh.rain_time) && (12_000..=180_000).contains(&fresh.thunder_time), "{fresh:?}");
    }

    #[test]
    fn only_a_level_that_can_have_weather_reports_the_world_s_weather() {
        use crate::level_storage::WeatherData;
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let rainy = WeatherData { rain_time: 600, raining: true, ..WeatherData::default() };
        for (dimension, reports) in [("minecraft:overworld", true), ("minecraft:the_nether", false), ("minecraft:the_end", false)] {
            let mut server = ServerSim::new(worldgen.clone(), states.clone(), dimension);
            server.set_weather_data(&rainy);
            server.tick();
            assert_eq!(server.weather_data().is_some(), reports, "{dimension}");
            assert_eq!(server.weather(None).rain_level > 0.0, reports, "{dimension}");
        }
    }

    #[test]
    fn a_weather_command_sets_the_world_s_weather_like_set_weather_parameters() {
        use crate::level_storage::WeatherData;
        assert_eq!(weather_parameters("clear", 900), Some(WeatherData { clear_weather_time: 900, ..WeatherData::default() }));
        assert_eq!(weather_parameters("rain", 600), Some(WeatherData { rain_time: 600, thunder_time: 600, raining: true, ..WeatherData::default() }));
        assert_eq!(weather_parameters("thunder", 300), Some(WeatherData { rain_time: 300, thunder_time: 300, raining: true, thundering: true, ..WeatherData::default() }));
        assert_eq!(weather_parameters("snow", 300), None);
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut overworld = ServerSim::new(worldgen, states, "minecraft:overworld");
        for (weather, duration) in [("clear", 900), ("rain", 600), ("thunder", 300)] {
            overworld.set_weather_for(weather, duration);
            assert_eq!(overworld.weather_data(), weather_parameters(weather, duration), "{weather}");
        }
    }

    #[test]
    fn every_level_follows_the_world_clocks_and_time_set_moves_only_a_default_clock() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let clock = |server: &ServerSim, name: &str| server.level.sky.as_ref().and_then(|sky| sky.clocks.get(name).copied());
        for (dimension, default) in [("minecraft:overworld", Some(OVERWORLD_CLOCK)), ("minecraft:the_nether", None), ("minecraft:the_end", Some(THE_END_CLOCK))] {
            let mut server = ServerSim::new(worldgen.clone(), states.clone(), dimension);
            server.set_clocks(ClockTicks { overworld: 13_000, the_end: 250 });
            assert_eq!(server.level.overworld_clock(), 13_000, "{dimension}");
            assert_eq!(clock(&server, THE_END_CLOCK), Some(250), "{dimension}");
            server.tick();
            assert_eq!((server.level.overworld_clock(), clock(&server, THE_END_CLOCK)), (13_001, Some(251)), "{dimension}: both clocks run in every level");
            server.set_time(500);
            let expected = match default {
                Some(OVERWORLD_CLOCK) => (500, Some(251)),
                Some(_) => (13_001, Some(500)),
                None => (13_001, Some(251)),
            };
            assert_eq!((server.level.overworld_clock(), clock(&server, THE_END_CLOCK)), expected, "{dimension}");
        }
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

    #[test]
    fn mobs_items_and_orbs_come_back_when_the_world_is_opened_again() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 1234).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let chunks: Vec<Arc<Chunk>> = (15..=25).flat_map(|x| (123..=133).map(move |z| (x, z))).map(|(x, z)| map.load_now(ChunkPos::new(x, z))).collect();
        let dir = crate::test_dir::tempdir().unwrap();
        let level = crate::level_storage::LevelStorage::open(dir.path()).unwrap();
        let storage = || Some(Arc::new(level.chunk_storage("minecraft:overworld", registries.clone(), -64, 384)));
        let animals = |world: &minecraftoss_entities::world::EntityWorld| world.cows().iter().filter(|c| c.horse.is_none()).count() + world.pigs().len() + world.chickens().len() + world.sheep().len();
        let mut first = ServerSim::new(worldgen.clone(), states.clone(), "minecraft:overworld");
        first.set_entity_storage(storage());
        for chunk in &chunks {
            first.load_chunk(chunk);
        }
        let generated = animals(&first.mobs);
        assert!(generated > 1, "seed 1234 has animals there");
        let victim = first.mobs.mob_ids().next().unwrap();
        if let Some(e) = first.mobs.cow_mut(victim) { e.hurt(100.0, minecraftoss_entities::world::DamageSourceKind::Generic); }
        if let Some(e) = first.mobs.sheep_mut(victim) { e.hurt(100.0, minecraftoss_entities::world::DamageSourceKind::Generic); }
        if let Some(e) = first.mobs.pig_mut(victim) { e.hurt(100.0, minecraftoss_entities::world::DamageSourceKind::Generic); }
        if let Some(e) = first.mobs.chicken_mut(victim) { e.hurt(100.0); }
        let mut handle = ServerHandle::spawn(first);
        handle.spawn_item("minecraft:diamond", 3, None, [328.5, 200.0, 2056.5], [0.0; 3], 10, 0);
        handle.summon_orb([330.5, 200.0, 2058.5], 7);
        handle.save();
        handle.wait_idle();
        let written = storage().unwrap().load_entities(ChunkPos::new(20, 128)).unwrap().expect("the save command wrote the chunk's entities");
        assert!(written.iter().any(|t| t.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some("minecraft:item")), "{written:?}");
        drop(handle);
        let mut second = ServerSim::new(worldgen, states, "minecraft:overworld");
        second.set_entity_storage(storage());
        for chunk in &chunks {
            second.load_chunk(chunk);
        }
        assert_eq!(animals(&second.mobs), generated - 1, "the killed animal stays dead in the next session");
        assert!(second.items().iter().any(|i| i.item == "minecraft:diamond" && i.count == 3), "the dropped stack is back");
        assert!(second.orbs().iter().any(|o| o.value == 7), "the orb is back");
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
        let result = server.mob_action(0, minecraftoss_entities::world::MobHit::Villager(id), None, inventory.clone(), 0, false);
        let view = result.merchant.expect("the trading screen opens");
        assert_eq!((view.profession, view.level, view.offers.len()), ("minecraft:farmer", 1, 1));
        let apply = |inventory: &mut Inventory, update: &MerchantUpdate| {
            for (slot, stack) in &update.slots {
                inventory.slots[*slot] = stack.clone();
            }
            inventory.cursor = update.cursor.clone();
        };
        let update = server.merchant(0, MerchantOp::Select(0), inventory.clone(), 0, [0.0; 3]);
        apply(&mut inventory, &update);
        let view = update.view.expect("still open");
        assert_eq!(view.payment[0].as_ref().map(|s| s.count), Some(50), "the wheat moved in");
        assert_eq!(view.result.as_ref().map(|s| s.id.as_str()), Some("minecraft:emerald"));
        let update = server.merchant(0, MerchantOp::Result { shift: true }, inventory.clone(), 0, [0.0; 3]);
        apply(&mut inventory, &update);
        let view = update.view.expect("still open");
        assert_eq!(view.payment[0].as_ref().map(|s| s.count), Some(10), "two trades of twenty");
        assert_eq!(inventory.count("minecraft:emerald"), 2);
        assert_eq!((view.level, view.xp), (2, 12), "the first trade levelled the farmer up");
        assert!(view.offers.len() > 1, "with its next level's offers");
        assert!(server.level.experience_total() > 0, "trades dropped experience");
        let update = server.merchant(0, MerchantOp::Close, inventory.clone(), 0, [0.0; 3]);
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
        let fed = server.mob_action(0, hit, None, inventory.clone(), 0, false);
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
            let result = server.mob_action(0, hit, Some(attack), inventory.clone(), 0, false);
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
            if let Some((_, _, value)) = server.take_experience(0, Some(feet)) {
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
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(bots), 40.0, [0.5, 100.0, -1.5], 1.0, None, 0);
        assert!(server.mobs.zombies().iter().all(|z| z.id != bots || z.zombie.health <= 0.0));
        assert!(server.orbs().is_empty());
        let mine = zombie(&mut server, 8.5);
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(mine), 40.0, [8.5, 100.0, -1.5], 1.0, Some(0), 0);
        assert_eq!(server.orbs().iter().map(|o| o.value).sum::<i32>(), 5);
    }

    #[test]
    fn stun_slowness_walks_a_mob_at_the_shellshocked_players_scale() {
        let mut effects = minecraftoss_entities::effects::MobEffects::default();
        effects.add(minecraftoss_entities::effects::EffectInstance::new(minecraftoss_entities::effects::MobEffect::Slowness, 120, STUN_SLOWNESS_AMPLIFIER));
        assert!((effects.movement_speed(1.0) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn a_stun_slows_the_mob_it_hurts_and_other_blasts_do_not() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        let zombie = |server: &mut ServerSim, x: f64| server.mobs.spawn_zombie(minecraftoss_entities::zombie::Zombie::new(glam::DVec3::new(x, 100.0, 0.5)), true);
        let slowness = |server: &ServerSim, id: u64| {
            server.mobs.mob_effects(id).and_then(|e| e.get(minecraftoss_entities::effects::MobEffect::Slowness)).map(|e| (e.amplifier, e.duration))
        };
        let stunned = zombie(&mut server, 0.5);
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(stunned), 0.4, [0.5, 100.0, -1.5], 0.5, Some(0), 120);
        assert_eq!(slowness(&server, stunned), Some((STUN_SLOWNESS_AMPLIFIER, 120)));
        let shot = zombie(&mut server, 24.5);
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(shot), 5.0, [24.5, 100.0, -1.5], 0.0, Some(0), 0);
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(shot), 0.4, [24.5, 100.0, -1.5], 0.5, Some(0), 120);
        assert_eq!(slowness(&server, shot), None);
        let fragged = zombie(&mut server, 8.5);
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(fragged), 5.0, [8.5, 100.0, -1.5], 0.5, Some(0), 0);
        assert_eq!(slowness(&server, fragged), None);
        let killed = zombie(&mut server, 16.5);
        server.mob_blast(minecraftoss_entities::world::MobHit::Zombie(killed), 40.0, [16.5, 100.0, -1.5], 1.0, Some(0), 120);
        assert_eq!(slowness(&server, killed), None);
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
            if let Some((_, _, value)) = server.take_experience(0, Some(feet)) {
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
        assert_eq!(std::mem::take(&mut server.level.level_events), vec![(1505, minecraftoss_core::BlockPos::new(8, 201, 8), 15)]);
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

    fn survives_on_both_sides(server: &ServerSim, scene: &HandcraftedScene, pos: BlockPos, block: &Block) -> bool {
        let state = server.states.state_of(block).unwrap();
        let on_server = server.level.can_survive_at(state, minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2));
        let on_client = crate::placing::may_place(server.level.lib, &crate::placing::SurviveView::new(scene, None), pos, block);
        assert_eq!(on_client, on_server, "{block:?} at {pos:?}");
        on_server
    }

    #[test]
    fn sugar_cane_needs_water_frosted_ice_or_a_waterlogged_block_beside_its_ground() {
        let Some((mut server, mut scene)) = door_world() else { return };
        let cane = Block::new("minecraft:sugar_cane");
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:sand"));
        assert!(!survives_on_both_sides(&server, &scene, (8, 201, 8), &cane), "dry sand");
        for beside in [
            Block::new("minecraft:water").with("level", "0"),
            Block::new("minecraft:frosted_ice").with("age", "0"),
            Block::new("minecraft:oak_slab").with("type", "bottom").with("waterlogged", "true"),
        ] {
            place(&mut server, &mut scene, (9, 200, 8), beside.clone());
            assert!(survives_on_both_sides(&server, &scene, (8, 201, 8), &cane), "{beside:?}");
            take(&mut server, &mut scene, (9, 200, 8));
        }
        place(&mut server, &mut scene, (8, 199, 9), Block::new("minecraft:water").with("level", "0"));
        assert!(!survives_on_both_sides(&server, &scene, (8, 201, 8), &cane), "water diagonally below does not count");
        place(&mut server, &mut scene, (8, 200, 9), Block::new("minecraft:water").with("level", "0"));
        assert!(survives_on_both_sides(&server, &scene, (8, 201, 8), &cane));
        place(&mut server, &mut scene, (8, 201, 8), cane.clone());
        assert!(survives_on_both_sides(&server, &scene, (8, 202, 8), &cane), "cane on cane");
        place(&mut server, &mut scene, (6, 200, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (6, 200, 9), Block::new("minecraft:water").with("level", "0"));
        assert!(!survives_on_both_sides(&server, &scene, (6, 201, 8), &cane), "stone does not hold cane");
        place(&mut server, &mut scene, (4, 200, 8), Block::new("minecraft:grass_block"));
        place(&mut server, &mut scene, (4, 200, 9), Block::new("minecraft:water").with("level", "0"));
        assert!(survives_on_both_sides(&server, &scene, (4, 201, 8), &cane), "grass holds cane");
    }

    #[test]
    fn frosted_ice_ages_in_the_light_and_melts_back_into_water() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:glowstone"));
        place(&mut server, &mut scene, (9, 200, 8), Block::new("minecraft:frosted_ice").with("age", "0"));
        server.take_changes();
        let mut ages = Vec::new();
        let mut melted = None;
        for tick in 0..600 {
            server.tick();
            for (pos, block) in server.take_changes() {
                match block {
                    Some(block) if pos == (9, 200, 8) && block.id.path == "frosted_ice" => ages.push(block.properties["age"].clone()),
                    Some(block) if pos == (9, 200, 8) && block.id.path == "water" => {
                        melted.get_or_insert(tick);
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(ages, ["1", "2", "3"]);
        assert!(melted.is_some_and(|tick| tick >= 4 * 60), "every age change places the block again, which schedules 60 to 120 ticks: {melted:?}");
    }

    #[test]
    fn frosted_ice_left_with_one_frosted_neighbour_melts_when_that_neighbour_changes() {
        let Some((mut server, mut scene)) = door_world() else { return };
        for x in 8..=10 {
            place(&mut server, &mut scene, (x, 200, 8), Block::new("minecraft:frosted_ice").with("age", "0"));
        }
        server.take_changes();
        take(&mut server, &mut scene, (10, 200, 8));
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        let water = |pos| changes.get(&pos).cloned().flatten().is_some_and(|block: Block| block.id.path == "water");
        assert!(water((9, 200, 8)), "one frosted neighbour left: it melts");
        assert!(water((8, 200, 8)), "and so does the last one");
    }

    #[test]
    fn a_placement_that_cannot_survive_is_refused_by_the_client_and_the_server() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:sand"));
        server.take_changes();
        let aim = [8.5, 200.9, 8.5];
        assert_eq!(stage_as_client(&mut server, &mut scene, "minecraft:sugar_cane", [8.5, 201.0, 10.5], aim, false), None);
        assert!(Scene::block(&scene, (8, 201, 8)).is_none());
        place(&mut server, &mut scene, (8, 201, 8), Block::new("minecraft:sugar_cane"));
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(changes.get(&(8, 201, 8)), Some(&None), "the server sends its air back");
        for (pos, block) in changes {
            scene.set(pos, block);
        }
        assert_eq!(picked(&server, "minecraft:sugar_cane"), 1, "the refused cane drops as an item");
        place(&mut server, &mut scene, (9, 200, 8), Block::new("minecraft:water").with("level", "0"));
        server.take_changes();
        assert_eq!(place_as_client(&mut server, &mut scene, "minecraft:sugar_cane", [8.5, 201.0, 10.5], aim), Some((8, 201, 8)));
        assert_eq!(Scene::block(&scene, (8, 201, 8)).map(|b| b.id.key()).as_deref(), Some("minecraft:sugar_cane"));
        place(&mut server, &mut scene, (12, 199, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (12, 200, 8), Block::new("minecraft:sand"));
        place(&mut server, &mut scene, (13, 201, 8), Block::new("minecraft:stone"));
        assert_eq!(stage_as_client(&mut server, &mut scene, "minecraft:cactus", [12.5, 201.0, 10.5], [12.5, 200.9, 8.5], false), None, "cactus beside stone");
        take(&mut server, &mut scene, (13, 201, 8));
        assert_eq!(place_as_client(&mut server, &mut scene, "minecraft:cactus", [12.5, 201.0, 10.5], [12.5, 200.9, 8.5]), Some((12, 201, 8)));
    }

    #[test]
    fn a_refused_placement_gives_back_the_item_spent_and_never_leaves_a_lone_upper_half() {
        let Some((mut server, mut scene)) = door_world() else { return };
        server.take_changes();
        let door = |half: &str| Block::new("minecraft:oak_door").with("facing", "north").with("half", half).with("hinge", "left").with("open", "false").with("powered", "false");
        let refused = [
            ((2, 201, 8), Block::new("minecraft:tube_coral_wall_fan").with("facing", "south").with("waterlogged", "false")),
            ((4, 201, 8), Block::new("minecraft:cake").with("bites", "0")),
            ((6, 201, 8), Block::new("minecraft:wall_torch").with("facing", "south")),
            ((10, 201, 8), Block::new("minecraft:glow_lichen").with("down", "true").with("up", "false").with("north", "false").with("south", "false").with("west", "false").with("east", "false").with("waterlogged", "false")),
            ((12, 201, 8), door("lower")),
            ((12, 202, 8), door("upper")),
        ];
        for (pos, block) in refused {
            place(&mut server, &mut scene, pos, block);
        }
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        for pos in [(2, 201, 8), (4, 201, 8), (6, 201, 8), (10, 201, 8), (12, 201, 8), (12, 202, 8)] {
            assert_eq!(changes.get(&pos), Some(&None), "{pos:?}");
        }
        for item in ["minecraft:tube_coral_fan", "minecraft:cake", "minecraft:torch", "minecraft:glow_lichen", "minecraft:oak_door"] {
            assert_eq!(picked(&server, item), 1, "{item}");
        }
        place(&mut server, &mut scene, (8, 200, 8), door("lower"));
        place(&mut server, &mut scene, (8, 201, 8), door("upper"));
        let blocks = &server.level.registries().blocks;
        assert_eq!(blocks.property(server.level.block(minecraftoss_core::BlockPos::new(8, 201, 8)), "half"), Some("upper"));
        assert_eq!(picked(&server, "minecraft:oak_door"), 1);
    }

    #[test]
    fn cane_stays_when_only_its_water_goes_and_breaks_when_its_side_is_touched_like_vanilla() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:sand"));
        place(&mut server, &mut scene, (9, 199, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (9, 200, 8), Block::new("minecraft:water").with("level", "0"));
        for y in 201..=203 {
            place(&mut server, &mut scene, (8, y, 8), Block::new("minecraft:sugar_cane"));
        }
        server.take_changes();
        take(&mut server, &mut scene, (9, 200, 8));
        let dried = ticked(&mut server, 3);
        for y in 201..=203 {
            assert!(!dried.contains_key(&(8, y, 8)), "the cane only checks itself when a face neighbour changes: {dried:?}");
        }
        place(&mut server, &mut scene, (7, 201, 8), Block::new("minecraft:stone"));
        let changes = ticked(&mut server, 1);
        assert_eq!(changes.get(&(8, 201, 8)), Some(&None));
        assert!(!changes.contains_key(&(8, 203, 8)));
        let changes = ticked(&mut server, 4);
        for y in 202..=203 {
            assert_eq!(changes.get(&(8, y, 8)), Some(&None), "sugar cane at {y}");
        }
        assert_eq!(picked(&server, "minecraft:sugar_cane"), 3);
    }

    #[test]
    fn a_cactus_pops_off_when_a_block_is_placed_beside_it() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:sand"));
        place(&mut server, &mut scene, (8, 201, 8), Block::new("minecraft:cactus"));
        place(&mut server, &mut scene, (8, 202, 8), Block::new("minecraft:cactus"));
        server.take_changes();
        place(&mut server, &mut scene, (9, 202, 8), Block::new("minecraft:stone"));
        let changes = ticked(&mut server, 2);
        assert_eq!(changes.get(&(8, 202, 8)), Some(&None));
        assert!(!changes.contains_key(&(8, 201, 8)), "the lower cactus keeps its free sides");
        assert_eq!(picked(&server, "minecraft:cactus"), 1);
    }

    #[test]
    fn plants_torches_carpets_cakes_corals_and_rails_need_their_support() {
        let Some((mut server, mut scene)) = door_world() else { return };
        let slab = |half: &str| Block::new("minecraft:oak_slab").with("type", half);
        let cases = [
            (Block::new("minecraft:stone"), Block::new("minecraft:poppy"), false),
            (Block::new("minecraft:dirt"), Block::new("minecraft:poppy"), true),
            (Block::new("minecraft:air"), Block::new("minecraft:torch"), false),
            (slab("bottom"), Block::new("minecraft:torch"), false),
            (Block::new("minecraft:glass"), Block::new("minecraft:torch"), true),
            (Block::new("minecraft:air"), Block::new("minecraft:white_carpet"), false),
            (Block::new("minecraft:oak_fence"), Block::new("minecraft:white_carpet"), true),
            (Block::new("minecraft:air"), Block::new("minecraft:cake"), false),
            (Block::new("minecraft:stone"), Block::new("minecraft:cake"), true),
            (Block::new("minecraft:stone"), Block::new("minecraft:white_candle_cake"), true),
            (slab("bottom"), Block::new("minecraft:rail"), false),
            (slab("top"), Block::new("minecraft:rail"), true),
            (Block::new("minecraft:glass"), Block::new("minecraft:powered_rail"), true),
            (Block::new("minecraft:air"), Block::new("minecraft:brain_coral_fan").with("waterlogged", "false"), false),
            (Block::new("minecraft:stone"), Block::new("minecraft:dead_brain_coral").with("waterlogged", "false"), true),
            (Block::new("minecraft:dirt"), Block::new("minecraft:pitcher_crop"), false),
            (Block::new("minecraft:farmland"), Block::new("minecraft:pitcher_crop"), true),
        ];
        let mut held = Vec::new();
        for (x, (ground, block, holds)) in (-14..).step_by(2).zip(cases) {
            let (ground_at, at) = ((x, 209, 4), (x, 210, 4));
            if ground.id.key() != "minecraft:air" {
                place(&mut server, &mut scene, ground_at, ground.clone());
            }
            assert_eq!(survives_on_both_sides(&server, &scene, at, &block), holds, "{block:?} on {ground:?}");
            if holds {
                place(&mut server, &mut scene, at, block.clone());
                held.push((ground_at, at, block));
            }
        }
        server.take_changes();
        for (ground_at, _, _) in &held {
            take(&mut server, &mut scene, *ground_at);
        }
        let mut changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        changes.extend(ticked(&mut server, 3));
        for (_, at, block) in &held {
            assert_eq!(changes.get(at), Some(&None), "{block:?} goes with its support");
        }
        assert!(picked(&server, "minecraft:torch") >= 1);
        assert!(picked(&server, "minecraft:rail") >= 1);
    }

    #[test]
    fn wall_fans_and_shelf_mushrooms_need_a_sturdy_back() {
        let Some((mut server, mut scene)) = door_world() else { return };
        for (x, id) in [(8, "minecraft:tube_coral_wall_fan"), (10, "minecraft:dead_horn_coral_wall_fan"), (12, "minecraft:shelf_mushroom")] {
            let fan = |facing: &str| if id.ends_with("_fan") { Block::new(id).with("facing", facing).with("waterlogged", "false") } else { Block::new(id).with("facing", facing) };
            let block = fan("south");
            assert!(!survives_on_both_sides(&server, &scene, (x, 210, 9), &block), "{id} on air");
            place(&mut server, &mut scene, (x, 210, 8), Block::new("minecraft:stone"));
            assert!(survives_on_both_sides(&server, &scene, (x, 210, 9), &block), "{id} on stone");
            assert!(!survives_on_both_sides(&server, &scene, (x, 210, 9), &fan("north")), "{id} facing the stone");
            place(&mut server, &mut scene, (x, 210, 9), block);
        }
        server.take_changes();
        for x in [8, 10, 12] {
            take(&mut server, &mut scene, (x, 210, 8));
        }
        let mut changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        changes.extend(ticked(&mut server, 2));
        for x in [8, 10, 12] {
            assert_eq!(changes.get(&(x, 210, 9)), Some(&None), "{x}");
        }
    }

    #[test]
    fn a_stalagmite_breaks_and_a_stalactite_falls_without_support() {
        let Some((mut server, mut scene)) = door_world() else { return };
        let dripstone = |tip: &str, thickness: &str| Block::new("minecraft:pointed_dripstone").with("vertical_direction", tip).with("thickness", thickness).with("waterlogged", "false");
        place(&mut server, &mut scene, (8, 209, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (8, 210, 8), dripstone("up", "tip"));
        place(&mut server, &mut scene, (12, 215, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (12, 214, 8), dripstone("down", "frustum"));
        place(&mut server, &mut scene, (12, 213, 8), dripstone("down", "tip"));
        assert!(!survives_on_both_sides(&server, &scene, (4, 210, 8), &dripstone("up", "tip")));
        server.take_changes();
        take(&mut server, &mut scene, (8, 209, 8));
        take(&mut server, &mut scene, (12, 215, 8));
        let mut changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        changes.extend(ticked(&mut server, 1));
        assert_eq!(changes.get(&(8, 210, 8)), Some(&None), "the stalagmite breaks on the next tick");
        assert!(!changes.contains_key(&(12, 214, 8)), "a stalactite waits two ticks");
        changes.extend(ticked(&mut server, 2));
        assert_eq!(changes.get(&(12, 214, 8)), Some(&None));
        assert_eq!(changes.get(&(12, 213, 8)), Some(&None), "the whole stalactite falls");
        assert_eq!(picked(&server, "minecraft:pointed_dripstone"), 1);
        let falling = server.level.entities.iter().filter(|e| matches!(e.kind, minecraftoss_world::level::entity::EntityKind::FallingBlock(_))).count();
        assert_eq!(falling, 2);
    }

    #[test]
    fn a_stalactite_thickens_as_tips_are_added_below_and_merges_with_a_stalagmite() {
        let Some((mut server, mut scene)) = door_world() else { return };
        let dripstone = |tip: &str, thickness: &str| Block::new("minecraft:pointed_dripstone").with("vertical_direction", tip).with("thickness", thickness).with("waterlogged", "false");
        place(&mut server, &mut scene, (8, 215, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (8, 214, 8), dripstone("down", "tip"));
        place(&mut server, &mut scene, (8, 210, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (8, 211, 8), dripstone("up", "tip"));
        server.take_changes();
        place(&mut server, &mut scene, (8, 213, 8), dripstone("down", "tip"));
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(changes.get(&(8, 214, 8)), Some(&Some(dripstone("down", "frustum"))));
        place(&mut server, &mut scene, (8, 212, 8), dripstone("down", "tip_merge"));
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(changes.get(&(8, 213, 8)), Some(&Some(dripstone("down", "frustum"))));
        assert_eq!(changes.get(&(8, 214, 8)), Some(&Some(dripstone("down", "base"))));
        assert_eq!(changes.get(&(8, 211, 8)), Some(&Some(dripstone("up", "tip_merge"))), "the stalagmite merges with it");
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

    fn mined_to(server: &mut ServerSim, broken: &[(BlockPos, HowBroken)]) -> std::collections::HashMap<BlockPos, Option<String>> {
        server.take_changes();
        server.player_mines(broken);
        server.take_changes().into_iter().map(|(pos, block)| (pos, block.map(|b| b.id.key()))).collect()
    }

    #[test]
    fn mined_ice_on_stone_turns_into_water() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:ice"));
        let changes = mined_to(&mut server, &[((8, 200, 8), HowBroken::Mined)]);
        assert_eq!(changes.get(&(8, 200, 8)), Some(&Some("minecraft:water".to_owned())));
        assert!(server.items().is_empty());
    }

    #[test]
    fn silk_touched_ice_leaves_air() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:ice"));
        let changes = mined_to(&mut server, &[((8, 200, 8), HowBroken::SilkTouched)]);
        assert_eq!(changes.get(&(8, 200, 8)), Some(&None));
    }

    #[test]
    fn mined_ice_over_air_and_packed_and_blue_ice_leave_air() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 205, 8), Block::new("minecraft:ice"));
        place(&mut server, &mut scene, (10, 199, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (10, 200, 8), Block::new("minecraft:packed_ice"));
        place(&mut server, &mut scene, (12, 199, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (12, 200, 8), Block::new("minecraft:blue_ice"));
        let changes = mined_to(&mut server, &[((8, 205, 8), HowBroken::Mined), ((10, 200, 8), HowBroken::Mined), ((12, 200, 8), HowBroken::Mined)]);
        for pos in [(8, 205, 8), (10, 200, 8), (12, 200, 8)] {
            assert_eq!(changes.get(&pos), Some(&None), "{pos:?}");
        }
    }

    #[test]
    fn mined_ice_on_carpet_leaves_air() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:white_carpet"));
        place(&mut server, &mut scene, (8, 201, 8), Block::new("minecraft:ice"));
        let changes = mined_to(&mut server, &[((8, 201, 8), HowBroken::Mined)]);
        assert_eq!(changes.get(&(8, 201, 8)), Some(&None));
    }

    #[test]
    fn a_stack_of_mined_ice_all_melts() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:ice"));
        place(&mut server, &mut scene, (8, 201, 8), Block::new("minecraft:ice"));
        let changes = mined_to(&mut server, &[((8, 201, 8), HowBroken::Mined), ((8, 200, 8), HowBroken::Mined)]);
        for pos in [(8, 200, 8), (8, 201, 8)] {
            assert_eq!(changes.get(&pos), Some(&Some("minecraft:water".to_owned())), "{pos:?}");
        }
    }

    #[test]
    fn blasted_ice_leaves_no_water() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:ice"));
        let changes = mined_to(&mut server, &[((8, 200, 8), HowBroken::Blasted)]);
        assert_eq!(changes.get(&(8, 200, 8)), Some(&None));
    }

    #[test]
    fn mined_and_blasted_blocks_of_one_batch_drop_nothing_on_the_server() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:torch"));
        let changes = mined_to(&mut server, &[((8, 199, 8), HowBroken::Mined), ((8, 200, 8), HowBroken::Blasted)]);
        assert_eq!(changes.get(&(8, 199, 8)), Some(&None));
        assert_eq!(changes.get(&(8, 200, 8)), Some(&None));
        assert!(server.items().is_empty());
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

    fn player_at(id: u64, feet: [f64; 3], alive: bool) -> crate::thrown::PlayerBody {
        crate::thrown::PlayerBody { id, feet: glam::DVec3::from_array(feet), alive, spectator: false }
    }

    fn thrown_down_by(owner: u64, item: &str, seed: u64) -> crate::thrown::Thrown {
        let mut random = minecraftoss_player::rng::LegacyRandom::new(seed);
        let throw = crate::thrown::Throw { owner, item, components: None, eye: glam::DVec3::new(8.5, 203.0, 8.5), yaw: 0.0, pitch: 90.0, motion: glam::DVec3::ZERO, on_ground: true };
        crate::thrown::Thrown::launch(&throw, &mut random).expect("a throwable")
    }

    #[test]
    fn a_pearl_reports_where_the_player_lands() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw(thrown_down("minecraft:ender_pearl", 3));
        let mut landings = Vec::new();
        for _ in 0..20 {
            server.tick_thrown(&[player_at(0, [40.0, 250.0, 40.0], true)]);
            landings.extend(server.take_pearl_teleports());
        }
        assert_eq!(landings.len(), 1);
        assert_eq!(landings[0].0, 0);
        assert!(landings[0].1[1] >= 200.0 && landings[0].1[1] < 202.0, "{landings:?}");
        assert!(server.thrown_views().is_empty());
    }

    #[test]
    fn snowballs_eggs_and_pearls_report_where_they_hit() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw(thrown_down("minecraft:snowball", 1));
        server.throw(thrown_down("minecraft:brown_egg", 2));
        server.throw(thrown_down("minecraft:ender_pearl", 3));
        server.throw(thrown_down("minecraft:experience_bottle", 4));
        let mut hits = Vec::new();
        for _ in 0..20 {
            server.tick_thrown(&[player_at(0, [40.0, 250.0, 40.0], true)]);
            hits.extend(server.take_thrown_hits());
        }
        let mut items: Vec<(&str, crate::thrown::Throwable)> = hits.iter().map(|h| (h.item.as_str(), h.kind)).collect();
        items.sort_by_key(|(item, _)| *item);
        assert_eq!(items, vec![("minecraft:brown_egg", crate::thrown::Throwable::Egg), ("minecraft:ender_pearl", crate::thrown::Throwable::EnderPearl), ("minecraft:snowball", crate::thrown::Throwable::Snowball)]);
        assert!(hits.iter().all(|h| h.at.y >= 199.9 && h.at.y < 202.0 && (h.at.x - 8.5).abs() < 0.5), "{hits:?}");
    }

    #[test]
    fn a_pearl_vanishes_when_its_thrower_is_dead() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw(thrown_down("minecraft:ender_pearl", 3));
        server.throw(thrown_down("minecraft:snowball", 4));
        server.tick_thrown(&[]);
        let left: Vec<String> = server.thrown_views().into_iter().map(|t| t.item).collect();
        assert_eq!(left, vec!["minecraft:snowball".to_owned()]);
        assert!(server.take_pearl_teleports().is_empty());
    }

    #[test]
    fn each_pearl_teleports_its_own_thrower_and_dies_with_it() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw(thrown_down_by(1, "minecraft:ender_pearl", 3));
        server.throw(thrown_down_by(2, "minecraft:ender_pearl", 5));
        let players = [player_at(1, [40.0, 250.0, 40.0], true), player_at(2, [-40.0, 250.0, 40.0], false)];
        let mut teleports = Vec::new();
        for _ in 0..20 {
            server.tick_thrown(&players);
            teleports.extend(server.take_pearl_teleports());
        }
        assert_eq!(teleports.iter().map(|t| t.0).collect::<Vec<_>>(), vec![1]);
    }

    type PearlTrips = (Vec<(u64, BlockPos)>, Vec<(u64, [f64; 3])>);

    fn pearl_through_gateway(server: &mut ServerSim) -> PearlTrips {
        server.throw(thrown_down_by(4, "minecraft:ender_pearl", 3));
        let (mut gateways, mut teleports) = (Vec::new(), Vec::new());
        for _ in 0..20 {
            server.tick_thrown(&[player_at(4, [40.0, 250.0, 40.0], true)]);
            gateways.extend(server.take_pearl_gateways());
            teleports.extend(server.take_pearl_teleports());
        }
        (gateways, teleports)
    }

    #[test]
    fn a_pearl_through_a_gateway_is_reported_and_triggers_its_cooldown() {
        let Some((mut server, _)) = door_world() else { return };
        let gateway = (8, 201, 8);
        server.player_edit_block(gateway, Some(&Block::new(crate::end_gateway::END_GATEWAY)), PlayerEdit::Place);
        server.tick_gateways();
        server.take_block_entity_changes();
        let (gateways, teleports) = pearl_through_gateway(&mut server);
        assert_eq!((gateways, teleports), (vec![(4, gateway)], Vec::new()));
        assert!(server.thrown_views().is_empty(), "the pearl is gone");
        assert_eq!(server.gateway_views()[0].cooldown, crate::end_gateway::COOLDOWN_TIME);
        assert!(changed_tag(&server.take_block_entity_changes(), gateway).is_some(), "the trigger marks the block entity");
    }

    #[test]
    fn a_cooling_gateway_lets_a_pearl_through() {
        let Some((mut server, _)) = door_world() else { return };
        let gateway = (8, 201, 8);
        server.player_edit_block(gateway, Some(&Block::new(crate::end_gateway::END_GATEWAY)), PlayerEdit::Place);
        server.tick_gateways();
        assert!(server.trigger_gateway(gateway));
        let (gateways, teleports) = pearl_through_gateway(&mut server);
        assert_eq!(gateways, Vec::new());
        assert_eq!(teleports.len(), 1);
        assert!(teleports[0].1[1] >= 200.0 && teleports[0].1[1] < 202.0, "{teleports:?}");
    }

    #[test]
    fn a_pearl_that_hits_a_wall_first_lands_normally() {
        let Some((mut server, _)) = door_world() else { return };
        let gateway = (8, 198, 8);
        server.player_edit_block(gateway, Some(&Block::new(crate::end_gateway::END_GATEWAY)), PlayerEdit::Place);
        server.tick_gateways();
        let (gateways, teleports) = pearl_through_gateway(&mut server);
        assert_eq!((gateways, teleports.len()), (Vec::new(), 1));
        assert_eq!(server.gateway_views()[0].cooldown, 0, "the gateway under the floor is never reached");
    }

    #[test]
    fn a_bottle_on_stone_bursts_into_3_to_11_experience_above_it() {
        let Some((mut server, _)) = door_world() else { return };
        for seed in 0..8 {
            server.throw(thrown_down("minecraft:experience_bottle", seed));
            for _ in 0..40 {
                server.tick_thrown(&[]);
            }
            assert!(server.thrown_views().is_empty());
            let orbs: Vec<_> = server.level.entities.iter().filter(|e| !e.removed && e.orb_data().is_some()).collect();
            let total: i32 = orbs.iter().map(|e| e.orb_data().map_or(0, |d| d.value * d.count)).sum();
            assert!((3..=11).contains(&total), "{total}");
            assert!(orbs.iter().all(|e| (e.pos[1] - 200.25).abs() < 1e-6 && e.delta[1] >= 0.0), "orbs leave the top face upwards");
            let breaks = server.take_potion_breaks();
            assert_eq!(breaks.len(), 1);
            assert_eq!((breaks[0].color, breaks[0].instant), (crate::thrown::BOTTLE_COLOR, false));
            let sounds: Vec<_> = server.level.sounds.drain(..).filter(|s| s.event == crate::thrown::BREAK_SOUND).collect();
            assert_eq!(sounds.len(), 1);
            assert_eq!(sounds[0].pos, [8.5, 200.5, 8.5]);
            assert!((0.9..=1.0).contains(&sounds[0].pitch));
            for e in &mut server.level.entities {
                e.removed |= e.orb_data().is_some();
            }
        }
    }

    fn thrown_potion(owner: u64, item: &str, potion: &str, seed: u64) -> crate::thrown::Thrown {
        let mut random = minecraftoss_player::rng::LegacyRandom::new(seed);
        let components = serde_json::json!({"minecraft:potion_contents": {"potion": potion}});
        let throw = crate::thrown::Throw { owner, item, components: Some(&components), eye: glam::DVec3::new(8.5, 203.0, 8.5), yaw: 0.0, pitch: 90.0, motion: glam::DVec3::ZERO, on_ground: true };
        crate::thrown::Thrown::launch(&throw, &mut random).expect("a throwable")
    }

    fn cow_health(server: &ServerSim, x: f64) -> Option<f32> {
        server.mobs.cows().iter().find(|c| (c.cow.body.position.x - x).abs() < 1e-6).map(|c| c.cow.health)
    }

    #[test]
    fn a_splash_of_harming_hurts_a_near_cow_more_than_a_far_one() {
        let Some((mut server, _)) = door_world() else { return };
        for x in [9.9, 11.5, 14.5] {
            if server.summon("minecraft:cow", [x, 200.0, 8.5], None, 0.0).is_err() {
                return;
            }
        }
        server.throw(thrown_potion(0, "minecraft:splash_potion", "minecraft:harming", 1));
        for _ in 0..20 {
            server.tick_thrown(&[]);
        }
        let breaks = server.take_potion_breaks();
        assert_eq!(breaks.len(), 1);
        assert!(breaks[0].instant);
        assert_eq!(breaks[0].color, 11101546);
        let (near, far) = (cow_health(&server, 9.9).unwrap(), cow_health(&server, 11.5).unwrap());
        assert!(near < far && far < 10.0, "{near} {far}");
        assert_eq!(near, 5.0);
        assert_eq!(cow_health(&server, 14.5), Some(10.0));
        assert_eq!(server.level.sounds.iter().filter(|s| s.event == crate::thrown::BREAK_SOUND).count(), 1);
    }

    #[test]
    fn a_splash_of_poison_reaches_every_near_player_and_its_thrower() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw(thrown_potion(1, "minecraft:splash_potion", "minecraft:poison", 2));
        let players = [player_at(1, [8.5, 200.0, 8.5], true), player_at(2, [11.0, 200.0, 8.5], true), player_at(3, [30.0, 200.0, 8.5], true)];
        let mut hits = Vec::new();
        for _ in 0..20 {
            server.tick_thrown(&players);
            hits.extend(server.take_player_potions());
        }
        let poisoned: Vec<(u64, i32)> = hits
            .iter()
            .filter_map(|h| match &h.potion {
                crate::potions::PlayerPotion::Effect(e) if e.effect == "minecraft:poison" => Some((h.player, e.duration)),
                _ => None,
            })
            .collect();
        assert_eq!(poisoned.iter().map(|p| p.0).collect::<Vec<_>>(), vec![1, 2], "{hits:?}");
        assert!(poisoned[0].1 > poisoned[1].1);
        assert!(poisoned[0].1 <= 900);
    }

    fn level_block(server: &ServerSim, (x, y, z): BlockPos) -> Option<Block> {
        let state = server.level.block(minecraftoss_core::BlockPos::new(x, y, z));
        if server.level.registries().blocks.is_air(state) {
            None
        } else {
            server.states.block(state).cloned()
        }
    }

    #[test]
    fn splash_water_puts_out_fire_candles_and_campfires_around_where_it_lands() {
        let Some((mut server, mut scene)) = door_world() else { return };
        for (x, z) in [(9, 8), (7, 8), (8, 9), (6, 8)] {
            place(&mut server, &mut scene, (x, 199, z), Block::new("minecraft:stone"));
        }
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:fire"));
        place(&mut server, &mut scene, (7, 200, 8), Block::new("minecraft:fire"));
        place(&mut server, &mut scene, (6, 200, 8), Block::new("minecraft:fire"));
        place(&mut server, &mut scene, (9, 200, 8), Block::new("minecraft:candle").with("lit", "true").with("candles", "2"));
        place(&mut server, &mut scene, (8, 200, 9), Block::new("minecraft:campfire").with("lit", "true").with("facing", "north"));
        assert_eq!(level_block(&server, (6, 200, 8)).map(|b| b.id.key()), Some("minecraft:fire".to_owned()));
        server.level.sounds.clear();
        server.throw(thrown_potion(0, "minecraft:splash_potion", "minecraft:water", 4));
        for _ in 0..20 {
            server.tick_thrown(&[]);
        }
        assert_eq!(level_block(&server, (8, 200, 8)), None);
        assert_eq!(level_block(&server, (7, 200, 8)), None);
        assert_eq!(level_block(&server, (6, 200, 8)).map(|b| b.id.key()), Some("minecraft:fire".to_owned()), "two blocks off it keeps burning");
        assert_eq!(level_block(&server, (9, 200, 8)).and_then(|b| b.properties.get("lit").cloned()).as_deref(), Some("false"));
        assert_eq!(level_block(&server, (9, 200, 8)).and_then(|b| b.properties.get("candles").cloned()).as_deref(), Some("2"));
        assert_eq!(level_block(&server, (8, 200, 9)).and_then(|b| b.properties.get("lit").cloned()).as_deref(), Some("false"));
        let events: Vec<_> = server.level.sounds.iter().map(|s| s.event).collect();
        assert!(events.contains(&crate::splash::CANDLE_SOUND) && events.contains(&crate::splash::FIRE_SOUND), "{events:?}");
        let breaks = server.take_potion_breaks();
        assert_eq!((breaks.len(), breaks[0].color, breaks[0].instant), (1, crate::potions::DEFAULT_COLOR, false));
    }

    #[test]
    fn splash_water_puts_out_burning_mobs_and_players_and_stings_endermen() {
        let Some((mut server, _)) = door_world() else { return };
        if server.summon("minecraft:cow", [9.5, 200.0, 8.5], None, 0.0).is_err() || server.summon("minecraft:enderman", [10.5, 200.0, 8.5], None, 0.0).is_err() {
            return;
        }
        let cow = server.mobs.cows()[0].id;
        server.mobs.body_mut(cow).unwrap().fire_ticks = 100;
        let enderman = server.mobs.endermen()[0].id;
        let before = server.mobs.endermen()[0].enderman.health;
        server.throw(thrown_potion(3, "minecraft:splash_potion", "minecraft:water", 5));
        let players = [player_at(3, [8.5, 200.0, 8.5], true), player_at(4, [30.5, 200.0, 8.5], true)];
        let mut hits = Vec::new();
        for _ in 0..20 {
            server.tick_thrown(&players);
            hits.extend(server.take_player_potions());
        }
        assert_eq!(server.mobs.body_mut(cow).unwrap().fire_ticks, 0);
        let after = server.mobs.endermen().iter().find(|e| e.id == enderman).unwrap();
        assert_eq!(after.enderman.health, before - 1.0);
        assert!(after.dodge_pending);
        assert_eq!(hits.iter().map(|h| (h.player, h.potion.clone())).collect::<Vec<_>>(), vec![(3, crate::potions::PlayerPotion::Extinguish)]);
    }

    #[test]
    fn a_lingering_poison_cloud_poisons_whoever_stands_in_it_once_a_second() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw(thrown_potion(0, "minecraft:lingering_potion", "minecraft:poison", 6));
        let players = [player_at(1, [9.0, 200.0, 8.5], true), player_at(2, [13.5, 200.0, 8.5], true)];
        let mut poisoned = Vec::new();
        let mut radii = Vec::new();
        let mut gone = None;
        for tick in 0..140 {
            server.tick_clouds(&players);
            server.tick_thrown(&players);
            let views = server.cloud_views();
            radii.extend(views.iter().map(|c| c.radius));
            if let Some(view) = views.first() {
                assert!(view.position.distance(glam::DVec3::new(8.5, 200.0, 8.5)) < 0.05, "{view:?}");
            } else if !radii.is_empty() && gone.is_none() {
                gone = Some(tick);
            }
            for hit in server.take_player_potions() {
                if let crate::potions::PlayerPotion::Effect(effect) = hit.potion {
                    poisoned.push((tick, hit.player, effect.duration));
                }
            }
        }
        assert_eq!(server.take_potion_breaks().len(), 1);
        assert_eq!(poisoned.len(), 5, "{poisoned:?}");
        assert!(poisoned.iter().all(|&(_, player, duration)| player == 1 && duration == 225));
        assert!(poisoned.windows(2).all(|w| w[1].0 - w[0].0 == 20), "{poisoned:?}");
        assert!(radii.windows(2).all(|w| w[1] <= w[0]));
        assert_eq!(gone, Some(poisoned[4].0), "the fifth use shrinks it below half a block");
    }

    #[test]
    fn a_lingering_harming_cloud_hurts_a_cow_at_half_strength() {
        let Some((mut server, _)) = door_world() else { return };
        if server.summon("minecraft:cow", [9.5, 200.0, 8.5], None, 0.0).is_err() {
            return;
        }
        server.throw(thrown_potion(0, "minecraft:lingering_potion", "minecraft:harming", 7));
        let mut health = Vec::new();
        for _ in 0..40 {
            server.tick_clouds(&[]);
            server.tick_thrown(&[]);
            health.push(cow_health(&server, 9.5).unwrap_or(0.0));
        }
        health.dedup();
        assert_eq!(health[..2], [10.0, 7.0], "{health:?}");
    }

    fn trident_down(owner: u64, components: Option<serde_json::Value>, seed: u64) -> crate::trident::ThrownTrident {
        let mut random = minecraftoss_player::rng::LegacyRandom::new(seed);
        let mut stack = minecraftoss_player::inventory::ItemStack::new(crate::trident::ITEM, 1);
        stack.components = components;
        let components = stack.components.clone();
        let throw = crate::thrown::Throw { owner, item: crate::trident::ITEM, components: components.as_ref(), eye: glam::DVec3::new(8.5, 204.0, 8.5), yaw: 0.0, pitch: 90.0, motion: glam::DVec3::ZERO, on_ground: true };
        crate::trident::ThrownTrident::launch(&throw, stack, false, &mut random)
    }

    #[test]
    fn a_trident_hits_a_cow_for_8_sticks_and_only_its_thrower_takes_it_back() {
        let Some((mut server, _)) = door_world() else { return };
        if server.summon("minecraft:cow", [8.5, 200.0, 8.5], None, 0.0).is_err() {
            return;
        }
        server.throw_trident(trident_down(0, None, 5));
        let away = [player_at(0, [30.5, 200.0, 8.5], true)];
        for _ in 0..30 {
            server.tick_tridents(&away);
        }
        assert_eq!(cow_health(&server, 8.5), Some(2.0));
        let events: Vec<_> = server.level.sounds.iter().map(|s| s.event).collect();
        assert!(events.contains(&crate::trident::HIT_SOUND) && events.contains(&crate::trident::HIT_GROUND_SOUND), "{events:?}");
        let views = server.trident_views();
        assert_eq!(views.len(), 1);
        assert!((views[0].position.y - 200.0).abs() < 0.1, "{views:?}");
        let stranger = player_at(1, [9.5, 200.0, 8.5], true);
        let full = player_at(0, [9.5, 200.0, 8.5], true);
        assert!(server.pickup_tridents(&stranger, &[stranger, full], |_| true).is_empty());
        assert!(server.pickup_tridents(&full, &[stranger, full], |_| false).is_empty(), "no room, it stays");
        let picked = server.pickup_tridents(&full, &[stranger, full], |_| true);
        assert_eq!(picked.iter().map(|p| (p.0, p.2.as_str(), p.3)).collect::<Vec<_>>(), vec![(TRIDENT_IDS, crate::trident::ITEM, 1)]);
        assert!(server.trident_views().is_empty());
    }

    #[test]
    fn a_saved_trident_comes_back_and_still_knows_its_thrower() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw_trident(trident_down(0, None, 5));
        let me = [player_at(0, [30.5, 200.0, 8.5], true)];
        for _ in 0..30 {
            server.tick_tridents(&me);
        }
        let tags: Vec<_> = server.tridents.iter().map(|t| t.to_tag(t.owner_uuid)).collect();
        assert_eq!(tags.len(), 1);
        assert!(tags[0].get("Owner").is_some());
        let Some((mut again, _)) = door_world() else { return };
        for tag in tags {
            again.add_saved_entity(tag);
        }
        assert_eq!(again.trident_views().len(), 1);
        let stranger = player_at(7, [9.5, 200.0, 8.5], true);
        again.tick_tridents(&[stranger]);
        let mut offered = false;
        assert!(again.pickup_tridents(&stranger, &[stranger], |_| std::mem::replace(&mut offered, true)).is_empty());
        assert!(offered, "while its thrower is away anyone may take it");
        let near = player_at(0, [9.5, 200.0, 8.5], true);
        again.tick_tridents(&[stranger, near]);
        assert!(again.pickup_tridents(&stranger, &[stranger, near], |_| true).is_empty(), "its thrower is back");
        assert_eq!(again.pickup_tridents(&near, &[stranger, near], |_| true).len(), 1);
    }

    #[test]
    fn a_trident_waits_where_the_world_is_not_loaded() {
        let Some((mut server, _)) = door_world() else { return };
        let mut random = minecraftoss_player::rng::LegacyRandom::new(8);
        let stack = minecraftoss_player::inventory::ItemStack::new(crate::trident::ITEM, 1);
        let throw = crate::thrown::Throw { owner: 0, item: crate::trident::ITEM, components: None, eye: glam::DVec3::new(500.5, 204.0, 500.5), yaw: 0.0, pitch: 0.0, motion: glam::DVec3::ZERO, on_ground: true };
        server.throw_trident(crate::trident::ThrownTrident::launch(&throw, stack, false, &mut random));
        let start = server.trident_views()[0].position;
        for _ in 0..200 {
            server.tick_tridents(&[]);
        }
        let views = server.trident_views();
        assert_eq!(views.len(), 1, "it is kept, not lost below the world");
        assert_eq!(views[0].position, start);
    }

    #[test]
    fn a_trident_strikes_another_player_for_8() {
        let Some((mut server, _)) = door_world() else { return };
        server.throw_trident(trident_down(0, None, 6));
        let players = [player_at(0, [30.5, 200.0, 8.5], true), player_at(2, [8.5, 200.0, 8.5], true)];
        for _ in 0..10 {
            server.tick_tridents(&players);
        }
        let struck = server.take_players_struck();
        assert_eq!(struck.len(), 1);
        assert_eq!((struck[0].player, struck[0].by, struck[0].damage, struck[0].kind), (2, 0, 8.0, "minecraft:trident"));
    }

    #[test]
    fn a_trident_saves_its_throwers_own_uuid() {
        let Some((mut server, _)) = door_world() else { return };
        let host = 0xb50a_d385_829d_3141_a216_7e7d_7539_ba7f;
        server.set_player_uuid(0, host);
        server.throw_trident(trident_down(0, None, 5));
        assert_eq!(server.tridents[0].owner_uuid, Some(host));
        let tag = server.tridents[0].to_tag(server.tridents[0].owner_uuid);
        assert_eq!(tag.get("Owner"), Some(&minecraftoss_core::nbt::Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(host).to_vec())));
        assert_eq!(server.mobs.uuid_of(crate::thrown::PLAYER_TARGET), host);
    }

    #[test]
    fn a_wind_charge_bursts_on_the_ground_and_throws_cows_and_its_thrower_up() {
        let Some((mut server, _)) = door_world() else { return };
        if server.summon("minecraft:cow", [9.6, 200.0, 8.5], None, 0.0).is_err() {
            return;
        }
        let before = server.mobs.cows()[0].cow.health;
        server.throw(thrown_down_by(3, "minecraft:wind_charge", 9));
        let players = [player_at(3, [7.5, 200.0, 8.5], true), player_at(4, [20.5, 200.0, 8.5], true)];
        let mut pushed = Vec::new();
        for _ in 0..10 {
            server.tick_thrown(&players);
            pushed.extend(server.take_players_pushed());
        }
        assert!(server.thrown_views().is_empty());
        let cow = &server.mobs.cows()[0];
        assert_eq!(cow.cow.health, before, "the burst itself hurts nobody");
        assert!(cow.cow.body.velocity.x > 0.0 && cow.cow.body.velocity.y > 0.0, "{:?}", cow.cow.body.velocity);
        assert_eq!(pushed.len(), 1);
        assert_eq!(pushed[0].0, 3);
        assert!(pushed[0].1.x < 0.0 && pushed[0].1.y > 0.0, "{pushed:?}");
        assert!(server.level.sounds.iter().any(|s| s.event == crate::wind::BURST_SOUND && (s.volume - 4.0).abs() < 1e-6));
        assert!(server.take_potion_breaks().is_empty());
    }

    #[test]
    fn a_wind_charge_hits_a_mob_for_1_before_it_bursts() {
        let Some((mut server, _)) = door_world() else { return };
        if server.summon("minecraft:cow", [8.5, 200.0, 8.5], None, 0.0).is_err() {
            return;
        }
        server.throw(thrown_down_by(0, "minecraft:wind_charge", 10));
        for _ in 0..10 {
            server.tick_thrown(&[]);
        }
        assert_eq!(cow_health(&server, 8.5).or_else(|| server.mobs.cows().first().map(|c| c.cow.health)), Some(9.0));
    }

    #[test]
    fn a_witch_potion_breaks_with_the_glass_sound() {
        let Some((mut server, _)) = door_world() else { return };
        server.set_simulation_area((0, 0), 2);
        let mut random = minecraftoss_player::rng::LegacyRandom::new(1);
        let poison = minecraftoss_entities::potion::Potion::Poison;
        let potion = minecraftoss_entities::potion::ThrownPotion::shoot(poison, 999, glam::DVec3::new(8.5, 202.0, 8.5), glam::DVec3::new(0.0, -1.0, 0.0), 0.5, 0.0, &mut random);
        server.mobs.spawn_potion(potion);
        let mut breaks = Vec::new();
        for _ in 0..20 {
            server.tick_mobs(&[], true);
            breaks.extend(server.take_potion_breaks());
        }
        assert_eq!(breaks.len(), 1);
        assert_eq!(breaks[0].color, poison.color());
        let sounds: Vec<_> = server.level.sounds.iter().filter(|s| s.event == crate::thrown::BREAK_SOUND).collect();
        assert_eq!(sounds.len(), 1);
        assert_eq!(sounds[0].pos, [8.5, 200.5, 8.5]);
    }

    #[test]
    fn directed_orbs_never_fly_into_the_face() {
        let Some((mut server, _)) = door_world() else { return };
        server.level.award_experience_toward([20.0, 210.5, 20.5], [-1.0, 0.0, 0.0], 60);
        let orbs: Vec<_> = server.level.entities.iter().filter(|e| !e.removed && e.orb_data().is_some()).collect();
        assert!(orbs.len() > 3);
        assert!(orbs.iter().all(|e| e.delta[0] <= 0.0 && (e.pos[0] - 19.75).abs() < 1e-9), "{:?}", orbs.iter().map(|e| (e.pos, e.delta)).collect::<Vec<_>>());
    }

    #[test]
    fn eggs_now_and_then_hatch_baby_chickens() {
        let Some((mut server, _)) = door_world() else { return };
        let before = server.mobs.chickens().len();
        for seed in 0..64 {
            server.throw(thrown_down("minecraft:brown_egg", seed));
        }
        for _ in 0..20 {
            server.tick_thrown(&[]);
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
        assert_eq!(std::mem::take(&mut server.level.level_events), vec![(1501, minecraftoss_core::BlockPos::new(8, 200, 8), 0)], "and fizzes");
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
        let effects = server.take_explosion_effects();
        assert_eq!(effects.len(), 1);
        let effect = effects[0];
        assert_eq!((effect.radius, effect.interacts), (4.0, true));
        assert!(effect.blocks > 100, "{} blocks, air included as in vanilla", effect.blocks);
        assert!((effect.center[1] - 200.0).abs() < 0.1);
        assert!(server.take_explosion_effects().is_empty());
        server.weapon_blast([8.5, 200.2, 8.5], 3.0);
        assert!(server.take_explosion_effects().is_empty(), "MW2 blasts draw their own effects");
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
        handle.hatch_egg(0, "minecraft:cow".to_owned(), position, 45.0);
        handle.hatch_egg(0, "minecraft:not_a_mob".to_owned(), position, 0.0);
        handle.hatch_egg(0, "minecraft:fox".to_owned(), position, 0.0);
        handle.wait_idle();
        let answers: Vec<_> = handle.poll().into_iter().flat_map(|out| out.summoned).map(|(_, answer)| answer).collect();
        assert_eq!(answers.len(), 3);
        assert_eq!(answers[0].as_deref(), Ok("minecraft:cow"));
        assert!(answers[1].is_err());
        assert!(answers[2].is_err());
    }

    fn spawner_with_egg(difficulty: i32, rule: bool) -> Option<ServerSim> {
        let (mut server, _) = door_world()?;
        server.set_players(&[[4.5, 205.0, 7.5]], difficulty);
        let mut rules = crate::game_rules::GameRules::default();
        rules.set("spawner_blocks_work", if rule { "true" } else { "false" }).unwrap();
        server.apply_game_rules(&rules);
        server.player_edit_block((4, 205, 4), Some(&Block::new("minecraft:spawner")), PlayerEdit::Place);
        server.take_block_entity_changes();
        Some(server)
    }

    fn spawner_mob(server: &ServerSim) -> Option<String> {
        let tag = server.level.chunk(ChunkPos::new(0, 0))?.block_entities.entities.get(&(4, 205, 4))?;
        Some(tag.get("SpawnData")?.get("entity")?.get("id")?.as_str()?.to_owned())
    }

    #[test]
    fn an_egg_on_a_placed_spawner_sets_its_mob_and_keeps_it() {
        let Some(mut server) = spawner_with_egg(2, true) else { return };
        assert_eq!(spawner_mob(&server), None);
        server.spawner_egg((4, 205, 4), "minecraft:zombie").unwrap();
        assert_eq!(spawner_mob(&server).as_deref(), Some("minecraft:zombie"));
        let changes = server.take_block_entity_changes();
        assert!(changes.iter().any(|(pos, tag)| *pos == (4, 205, 4) && tag.is_some()), "{changes:?}");
        server.spawner_egg((4, 205, 4), "minecraft:ghast").unwrap();
        assert_eq!(spawner_mob(&server).as_deref(), Some("minecraft:ghast"));
        assert!(server.spawner_egg((4, 206, 4), "minecraft:ghast").is_err(), "no spawner there");
        server.level.natural_spawning.as_mut().unwrap().spawn_mobs = false;
        let player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: glam::DVec3::new(4.5, 205.0, 7.5),
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
        let mut ghasts = 0;
        for _ in 0..80 {
            server.set_players(&[[4.5, 205.0, 7.5]], 2);
            server.set_simulation_area((0, 0), 1);
            server.prepare_spawning(&[player]);
            server.tick();
            ghasts += server.level.spawned.iter().filter(|tag| tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some("minecraft:ghast")).count();
            server.take_spawned();
        }
        assert!(ghasts > 0, "the spawner made ghasts");
        assert_eq!(spawner_mob(&server).as_deref(), Some("minecraft:ghast"), "a placed spawner has no potentials to pick another mob from");
        let delay = server.level.chunk(ChunkPos::new(0, 0)).and_then(|c| c.block_entities.entities.get(&(4, 205, 4))).and_then(|t| t.get("Delay")).and_then(minecraftoss_core::nbt::Tag::as_i64);
        assert!(delay.is_some_and(|d| (100..800).contains(&d)), "{delay:?}");
    }

    #[test]
    fn an_egg_does_nothing_when_spawner_blocks_do_not_work() {
        let Some(mut server) = spawner_with_egg(2, false) else { return };
        assert_eq!(server.spawner_egg((4, 205, 4), "minecraft:pig"), Err("advMode.notEnabled.spawner".to_owned()));
        assert_eq!(spawner_mob(&server), None);
        assert!(server.take_block_entity_changes().is_empty());
    }

    #[test]
    fn a_monster_egg_does_nothing_to_a_spawner_in_peaceful() {
        let Some(mut server) = spawner_with_egg(0, true) else { return };
        assert!(server.spawner_egg((4, 205, 4), "minecraft:zombie").is_err());
        assert_eq!(spawner_mob(&server), None);
        assert!(server.take_block_entity_changes().is_empty());
        server.spawner_egg((4, 205, 4), "minecraft:pig").unwrap();
        assert_eq!(spawner_mob(&server).as_deref(), Some("minecraft:pig"));
        let mut handle = ServerHandle::spawn(server);
        handle.spawner_egg(0, (4, 205, 4), "minecraft:creeper".to_owned());
        handle.spawner_egg(0, (4, 205, 4), "minecraft:cow".to_owned());
        handle.wait_idle();
        let outputs = handle.poll();
        let answers: Vec<_> = outputs.iter().flat_map(|out| out.summoned.clone()).map(|(_, answer)| answer).collect();
        assert_eq!(answers.len(), 2);
        assert!(answers[0].is_err());
        assert_eq!(answers[1].as_deref(), Ok("minecraft:cow"));
        let changed: Vec<_> = outputs.iter().flat_map(|out| out.block_entities.clone()).collect();
        assert!(changed.iter().any(|(pos, tag)| *pos == (4, 205, 4) && tag.as_ref().and_then(|t| t.get("SpawnData")?.get("entity")?.get("id")?.as_str().map(str::to_owned)).as_deref() == Some("minecraft:cow")), "{changed:?}");
    }

    #[test]
    fn command_summons_do_not_reach_the_egg_results() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 200, 8), Block::new("minecraft:smooth_stone_slab").with("type", "bottom").with("waterlogged", "false"));
        server.set_players(&[[8.5, 200.0, 11.5]], 2);
        let position = [8.5, 200.5, 8.5];
        let dormant = server.dormant.len();
        let fresh = minecraftoss_core::snbt::parse_compound("{}").unwrap();
        for kind in crate::server_mobs::SIMULATED_KINDS {
            server.summon(kind, position, Some(&fresh), 0.0).unwrap_or_else(|error| panic!("{kind}: {error}"));
            assert_eq!(server.dormant.len(), dormant, "{kind} is simulated");
        }
        let mut handle = ServerHandle::spawn(server);
        handle.summon(0, "minecraft:cow".to_owned(), position, None);
        handle.hatch_egg(0, "minecraft:pig".to_owned(), position, 0.0);
        handle.summon(3, "minecraft:not_a_mob".to_owned(), position, None);
        handle.wait_idle();
        let outputs = handle.poll();
        let eggs: Vec<_> = outputs.iter().flat_map(|out| out.summoned.clone()).collect();
        let commands: Vec<_> = outputs.iter().flat_map(|out| out.command_summons.clone()).collect();
        assert_eq!(eggs, vec![(0, Ok("minecraft:pig".to_owned()))]);
        assert_eq!(commands.len(), 2);
        assert_eq!((commands[0].0, commands[0].1.as_deref()), (0, Ok("minecraft:cow")));
        assert!(commands[1].0 == 3 && commands[1].1.is_err(), "each answer names who summoned");
    }

    #[test]
    fn dormant_shulkers_stay_far_from_players_while_dormant_monsters_go() {
        let Some((mut server, _)) = door_world() else { return };
        let fresh = minecraftoss_core::snbt::parse_compound("{}").unwrap();
        server.summon("minecraft:shulker", [8.5, 200.0, 8.5], Some(&fresh), 0.0).unwrap();
        server.summon("minecraft:pillager", [9.5, 200.0, 8.5], Some(&fresh), 0.0).unwrap();
        let kinds = |server: &ServerSim| server.dormant.iter().map(|(group, _)| group[0].kind.clone()).collect::<Vec<_>>();
        assert_eq!(kinds(&server), ["minecraft:shulker", "minecraft:pillager"]);
        let player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: glam::DVec3::new(3008.5, 200.0, 8.5),
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
        server.despawn_mobs(&[player], 2);
        assert_eq!(kinds(&server), ["minecraft:shulker"]);
        server.despawn_mobs(&[player], 0);
        assert_eq!(kinds(&server), ["minecraft:shulker"], "shulkers are allowed in peaceful");
    }

    #[test]
    fn twelve_eyes_in_an_inward_ring_of_frames_open_the_end_portal() {
        use minecraftoss_world::level::end_portal::{FILL_SOUND, OPEN_SOUND};
        let Some((mut server, _)) = door_world() else { return };
        let centre = (8, 201, 8);
        let mut frames = Vec::new();
        for i in -1..=1 {
            frames.push(((centre.0 + i, centre.1, centre.2 - 2), "south"));
            frames.push(((centre.0 + i, centre.1, centre.2 + 2), "north"));
            frames.push(((centre.0 - 2, centre.1, centre.2 + i), "east"));
            frames.push(((centre.0 + 2, centre.1, centre.2 + i), "west"));
        }
        for &(pos, facing) in &frames {
            let frame = Block::new("minecraft:end_portal_frame").with("facing", facing).with("eye", "false");
            server.player_edit_block(pos, Some(&frame), PlayerEdit::Place);
        }
        server.player_edit_block(centre, Some(&Block::new("minecraft:cobblestone")), PlayerEdit::Place);
        server.take_changes();
        server.level.sounds.clear();
        assert!(!server.item_on_block((8, 199, 8), ItemOnBlock::EnderEye), "stone takes no eye");
        for (i, &(pos, _)) in frames.iter().enumerate() {
            assert!(server.item_on_block(pos, ItemOnBlock::EnderEye));
            assert!(!server.item_on_block(pos, ItemOnBlock::EnderEye), "a filled frame takes no second eye");
            let sounds: Vec<_> = server.level.sounds.drain(..).map(|s| (s.event, s.pos)).collect();
            let fill = (FILL_SOUND, [f64::from(pos.0) + 0.5, f64::from(pos.1) + 0.5, f64::from(pos.2) + 0.5]);
            if i + 1 < frames.len() {
                assert_eq!(sounds, [fill]);
                assert!(server.items().iter().all(|item| item.item != "minecraft:cobblestone"), "eye {i} opened nothing");
            } else {
                assert_eq!(sounds, [fill, (OPEN_SOUND, [8.5, 201.5, 8.5])]);
            }
        }
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        for x in 7..=9 {
            for z in 7..=9 {
                assert_eq!(block_at(&changes, (x, 201, z)).map(|b| b.id.key()), Some("minecraft:end_portal".to_owned()));
            }
        }
        for &(pos, facing) in &frames {
            let frame = block_at(&changes, pos);
            assert_eq!((property(&frame, "eye").as_deref(), property(&frame, "facing").as_deref()), (Some("true"), Some(facing)));
        }
        assert!(server.items().iter().any(|item| item.item == "minecraft:cobblestone"), "the cobblestone inside dropped");
    }

    #[test]
    fn a_frame_facing_out_keeps_the_portal_shut() {
        let Some((mut server, _)) = door_world() else { return };
        let mut frames = Vec::new();
        for i in -1..=1 {
            frames.push(((8 + i, 201, 6), if i == 0 { "north" } else { "south" }));
            frames.push(((8 + i, 201, 10), "north"));
            frames.push(((6, 201, 8 + i), "east"));
            frames.push(((10, 201, 8 + i), "west"));
        }
        for &(pos, facing) in &frames {
            let frame = Block::new("minecraft:end_portal_frame").with("facing", facing).with("eye", "false");
            server.player_edit_block(pos, Some(&frame), PlayerEdit::Place);
            assert!(server.item_on_block(pos, ItemOnBlock::EnderEye));
        }
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert!(changes.values().flatten().all(|b| b.id.key() != "minecraft:end_portal"));
        assert!(server.level.sounds.iter().all(|s| s.event != minecraftoss_world::level::end_portal::OPEN_SOUND));
    }

    fn end_world(chunks: impl Iterator<Item = (i32, i32)>) -> Option<ServerSim> {
        let paths = DataPaths::discover().ok()?;
        let registries = Arc::new(Registries::load(&paths).ok()?);
        let generator = TerrainGenerator::end(registries.clone(), 0).ok()?;
        let states = Arc::new(BlockStates::new(registries, 0, generator.chunk_min_y, generator.chunk_height).ok()?);
        let worldgen = Arc::new(WorldGen::for_dimension(Arc::new(generator), "minecraft:the_end").ok()?);
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:the_end");
        for (x, z) in chunks {
            server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
        }
        Some(server)
    }

    fn changed_tag(changes: &[(BlockPos, Option<minecraftoss_core::nbt::Tag>)], pos: BlockPos) -> Option<minecraftoss_core::nbt::Tag> {
        changes.iter().find(|(at, _)| *at == pos).and_then(|(_, tag)| tag.clone())
    }

    #[test]
    fn age_is_saved_and_a_flip_reports_the_block_entity() {
        use minecraftoss_core::nbt::Tag;
        let Some(mut server) = end_world([(5, 0)].into_iter()) else { return };
        let (pos, at) = ((85, 100, 5), ChunkPos::new(5, 0));
        let registries = server.level.registries();
        let mut chunk = server.level.chunk(at).unwrap().clone();
        chunk.set_block_raw(5, 100, 5, registries.blocks.parse_state(crate::end_gateway::END_GATEWAY).unwrap(), registries);
        let saved = [("id", Tag::String(crate::end_gateway::END_GATEWAY.to_owned())), ("x", Tag::Int(85)), ("y", Tag::Int(100)), ("z", Tag::Int(5)), ("Age", Tag::Long(150))];
        chunk.block_entities.entities.insert(pos, Tag::Compound(saved.into_iter().map(|(key, value)| (key.to_owned(), value)).collect()));
        server.unload_chunk(at);
        server.take_block_entity_changes();
        server.load_chunk(&chunk);
        assert_eq!(server.gateway_views(), [crate::end_gateway::GatewayView { pos, age: 150, cooldown: 0, exit: None, exact: false }]);
        assert!(server.gateway_views()[0].is_spawning());
        for _ in 0..49 {
            server.tick_gateways();
        }
        assert_eq!(changed_tag(&server.take_block_entity_changes(), pos), None, "still spawning at age 199");
        server.tick_gateways();
        let tag = changed_tag(&server.take_block_entity_changes(), pos).expect("the flip reports the block entity");
        assert_eq!(tag.get("Age"), Some(&Tag::Long(200)));
        assert!(!server.gateway_views()[0].is_spawning());
        server.tick_gateways();
        assert_eq!(changed_tag(&server.take_block_entity_changes(), pos), None);
        assert_eq!(server.gateway_views()[0].age, 201);
    }

    #[test]
    fn gateways_join_and_leave_with_their_chunks_and_setblock() {
        let Some(mut server) = end_world([(5, 0), (6, 0)].into_iter()) else { return };
        let gateway = Block::new(crate::end_gateway::END_GATEWAY);
        let at = |server: &ServerSim| server.gateway_views().iter().map(|view| (view.pos, view.age)).collect::<Vec<_>>();
        let pos = (85, 100, 5);
        server.player_edit_block(pos, Some(&gateway), PlayerEdit::Place);
        server.tick_gateways();
        assert_eq!(at(&server), [(pos, 1)], "a placed gateway starts at age 0 and ticks");
        server.player_edit_block(pos, None, PlayerEdit::Break);
        server.tick_gateways();
        assert_eq!(at(&server), []);
        let far = (100, 100, 5);
        server.player_edit_block(far, Some(&gateway), PlayerEdit::Place);
        server.tick_gateways();
        let chunk = server.level.chunk(ChunkPos::new(6, 0)).unwrap().clone();
        server.unload_chunk(ChunkPos::new(6, 0));
        assert_eq!(at(&server), [], "the gateway leaves with its chunk");
        server.load_chunk(&chunk);
        assert_eq!(at(&server), [(far, 0)], "and comes back with it, at its saved age");
        server.load_chunk(&chunk);
        assert_eq!(at(&server), [(far, 0)], "a chunk sent again keeps its gateways");
        let ring = (88, 120, 8);
        assert!(server.level.place_configured_feature(END_GATEWAY_DELAYED, minecraftoss_core::BlockPos::new(ring.0, ring.1, ring.2)));
        server.tick_gateways();
        assert_eq!(at(&server), [(ring, 1), (far, 1)], "a gateway placed by a feature ticks the same tick");
        assert_eq!(server.gateway_views()[0].exit, None);
    }

    #[test]
    fn setting_an_exit_writes_vanilla_keys() {
        use minecraftoss_core::nbt::Tag;
        let Some(mut server) = end_world([(5, 0)].into_iter()) else { return };
        let pos = (85, 100, 5);
        server.player_edit_block(pos, Some(&Block::new(crate::end_gateway::END_GATEWAY)), PlayerEdit::Place);
        for _ in 0..3 {
            server.tick_gateways();
        }
        server.take_block_entity_changes();
        server.set_gateway_exit(pos, (1000, 70, 3), false);
        let tag = changed_tag(&server.take_block_entity_changes(), pos).expect("the exit is reported");
        assert_eq!(tag.get("exit_portal"), Some(&Tag::IntArray(vec![1000, 70, 3])));
        assert_eq!(tag.get("ExactTeleport"), None);
        assert_eq!(tag.get("Age"), Some(&Tag::Long(3)));
        server.set_gateway_exit(pos, (100, 50, 0), true);
        let tag = changed_tag(&server.take_block_entity_changes(), pos).expect("the exit is reported");
        assert_eq!(tag.get("exit_portal"), Some(&Tag::IntArray(vec![100, 50, 0])));
        assert_eq!(tag.get("ExactTeleport"), Some(&Tag::Byte(1)));
        let view = server.gateway_views()[0];
        assert_eq!((view.exit, view.exact), (Some((100, 50, 0)), true));
        server.tick_gateways();
        assert!(server.trigger_gateway(pos));
        assert!(!server.trigger_gateway(pos), "a cooling gateway ignores a second trigger");
        assert!(!server.trigger_gateway((0, 0, 0)));
        let tag = changed_tag(&server.take_block_entity_changes(), pos).expect("a trigger reports the block entity");
        assert_eq!(tag.get("Age"), Some(&Tag::Long(4)));
        assert!(!tag.as_compound().unwrap().contains_key("cooldown"), "the cooldown is not saved");
        assert_eq!(server.gateway_views()[0].cooldown, crate::end_gateway::COOLDOWN_TIME);
    }

    #[test]
    fn the_end_platform_is_rebuilt_and_drops_what_it_breaks() {
        let Some(mut server) = end_world((5..=7).flat_map(|x| (-1..=1).map(move |z| (x, z)))) else { return };
        let name = |server: &ServerSim, pos: BlockPos| {
            let state = server.level.block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2));
            let blocks = &server.level.registries().blocks;
            blocks.block(blocks.block_of(state)).name.as_str().to_owned()
        };
        assert_eq!(name(&server, (100, 48, 0)), "minecraft:obsidian", "the End generates its platform");
        server.player_edit_block((101, 50, 0), Some(&Block::new("minecraft:cobblestone")), PlayerEdit::Place);
        server.player_edit_block((99, 48, -1), None, PlayerEdit::Break);
        server.player_edit_block((102, 48, 2), Some(&Block::new("minecraft:dirt")), PlayerEdit::Place);
        server.take_changes();
        let before = server.items().len();
        server.create_end_platform((100, 49, 0));
        for x in 98..=102 {
            for z in -2..=2 {
                assert_eq!(name(&server, (x, 48, z)), "minecraft:obsidian", "{x} 48 {z}");
                for y in 49..=51 {
                    assert_eq!(name(&server, (x, y, z)), "minecraft:air", "{x} {y} {z}");
                }
            }
        }
        let count = |server: &ServerSim, id: &str| server.items().iter().filter(|item| item.item == id).count();
        assert_eq!(server.items().len(), before + 2);
        assert_eq!((count(&server, "minecraft:cobblestone"), count(&server, "minecraft:dirt")), (1, 1));
        assert_eq!(count(&server, "minecraft:obsidian"), 0);
        let changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        assert_eq!(block_at(&changes, (101, 50, 0)), None);
        assert_eq!(block_at(&changes, (99, 48, -1)).map(|b| b.id.key()), Some("minecraft:obsidian".to_owned()));
        assert!(!changes.contains_key(&(100, 48, 0)), "blocks already right are left alone");
        server.create_end_platform((100, 49, 0));
        assert!(server.take_changes().is_empty(), "a whole platform changes nothing");
        assert_eq!(server.items().len(), before + 2);
    }

    #[test]
    fn a_first_fight_scan_builds_the_inactive_podium_and_later_scans_find_it() {
        let Some(mut server) = end_world((-1..=1).flat_map(|x| (-1..=1).map(move |z| (x, z)))) else { return };
        let block = |server: &ServerSim, pos: BlockPos| {
            let state = server.level.block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2));
            let blocks = &server.level.registries().blocks;
            blocks.block(blocks.block_of(state)).name.as_str().to_owned()
        };
        let top = server.level.exit_portal_location(minecraftoss_core::BlockPos::new(0, 0, 0));
        assert!(top.y > 40 && block(&server, (0, top.y, 0)) == "minecraft:end_stone", "{top:?}");
        server.take_changes();
        let scan = server.scan_dragon_fight((0, 0, 0), None);
        let at = (0, top.y, 0);
        assert_eq!(scan, DragonFightScan { previously_killed: false, dragon_killed: false, dragon_seen: false, dragon_uuid: None, exit_portal_location: Some(at) });
        for y in 0..4 {
            assert_eq!(block(&server, (0, at.1 + y, 0)), "minecraft:bedrock", "pillar {y}");
        }
        for (x, z) in [(3, 0), (-3, 1), (2, 2), (-1, -3)] {
            assert_eq!(block(&server, (x, at.1, z)), "minecraft:bedrock", "rim {x} {z}");
        }
        for (x, z) in [(1, 0), (-2, 1), (1, 1)] {
            assert_eq!(block(&server, (x, at.1, z)), "minecraft:air", "inside {x} {z}");
            assert_eq!(block(&server, (x, at.1 - 1, z)), "minecraft:bedrock", "floor {x} {z}");
        }
        assert_eq!(block(&server, (3, at.1 - 1, 1)), "minecraft:end_stone");
        assert_eq!(block(&server, (0, at.1 + 2, -1)), "minecraft:wall_torch");
        let torch = server.level.block(minecraftoss_core::BlockPos::new(1, at.1 + 2, 0));
        assert_eq!(server.level.registries().blocks.property(torch, "facing"), Some("east"));
        assert!(server.level.unsupported.is_empty(), "{:?}", server.level.unsupported);
        assert_eq!(server.level.find_exit_portal(minecraftoss_core::BlockPos::new(0, 0, 0)), Some(minecraftoss_core::BlockPos::new(0, at.1, 0)));
        server.take_changes();
        let again = server.scan_dragon_fight((0, 0, 0), None);
        assert_eq!(again.exit_portal_location, Some(at), "the podium is found, not built twice");
        assert!(server.take_changes().is_empty());
        server.summon(ENDER_DRAGON, [0.5, 128.0, 0.5], None, 0.0).expect("a dragon is summoned");
        assert_eq!(server.dragons.list.len(), 1);
        assert_eq!(server.dormant_count(), 0, "the dragon is live, not dormant");
        let discarded = server.scan_dragon_fight((0, 0, 0), Some(at));
        assert_eq!((discarded.dragon_killed, discarded.dragon_seen, discarded.dragon_uuid), (false, true, None));
        assert!(server.dragons.list.is_empty(), "a dragon without an exit portal goes");
        assert!(server.level.place_configured_feature("minecraft:end_podium_active", minecraftoss_core::BlockPos::new(0, at.1, 0)));
        assert_eq!(block(&server, (1, at.1, 0)), "minecraft:end_portal");
        assert!(server.level.has_active_exit_portal());
        let killed = server.scan_dragon_fight((0, 0, 0), Some(at));
        assert_eq!(killed, DragonFightScan { previously_killed: true, dragon_killed: true, dragon_seen: false, dragon_uuid: None, exit_portal_location: Some(at) });
        server.summon(ENDER_DRAGON, [0.5, 128.0, 0.5], None, 0.0).expect("a dragon is summoned");
        let alive = server.scan_dragon_fight((0, 0, 0), Some(at));
        assert_eq!((alive.previously_killed, alive.dragon_killed), (true, false));
        assert_eq!(alive.dragon_uuid, server.dragons.list.first().map(|dragon| dragon.uuid));
        assert!(alive.dragon_uuid.is_some());
        assert_eq!(server.fight_dragon, alive.dragon_uuid, "the kept dragon is the fight's");
    }

    #[test]
    fn an_end_gateway_in_the_arena_counts_as_an_active_exit_portal() {
        let Some(mut server) = end_world((-1..=1).flat_map(|x| (-1..=1).map(move |z| (x, z)))) else { return };
        assert!(!server.level.has_active_exit_portal());
        assert!(server.level.place_configured_feature("minecraft:end_gateway_return", minecraftoss_core::BlockPos::new(8, 90, 8)));
        assert!(server.level.has_active_exit_portal(), "TheEndGatewayBlockEntity extends TheEndPortalBlockEntity");
        server.take_changes();
        let scan = server.scan_dragon_fight((0, 0, 0), None);
        assert_eq!(scan, DragonFightScan { previously_killed: true, dragon_killed: true, dragon_seen: false, dragon_uuid: None, exit_portal_location: None });
        assert!(server.take_changes().is_empty(), "no podium is built");
    }

    #[test]
    fn features_placed_in_a_live_level_keep_their_entities_spawners_and_gateway_exits() {
        use minecraftoss_core::nbt::Tag;
        use minecraftoss_generator::feature::World;
        let Some((mut server, _)) = door_world() else { return };
        server.level.take_changed_block_entities();
        server.level.unsupported.clear();
        let block_entity = |server: &ServerSim, pos: (i32, i32, i32)| server.level.chunk(ChunkPos::new(pos.0 >> 4, pos.2 >> 4)).and_then(|c| c.block_entities.entities.get(&pos).cloned());
        assert!(server.level.place_configured_feature("minecraft:end_gateway_return", minecraftoss_core::BlockPos::new(8, 210, 8)));
        let gateway = block_entity(&server, (8, 210, 8)).expect("the gateway has its block entity");
        assert_eq!(gateway.get("exit_portal"), Some(&Tag::IntArray(vec![100, 50, 0])));
        assert_eq!(gateway.get("ExactTeleport"), Some(&Tag::Byte(1)));
        assert!(server.level.take_changed_block_entities().contains(&(8, 210, 8)));
        server.player_edit_block((4, 205, 4), Some(&Block::new("minecraft:spawner")), PlayerEdit::Place);
        World::set_spawner_entity(&mut server.level, 4, 205, 4, "minecraft:blaze");
        let spawner = block_entity(&server, (4, 205, 4)).expect("the spawner has its block entity");
        let kind = spawner.get("SpawnData").and_then(|d| d.get("entity")).and_then(|e| e.get("id")).and_then(Tag::as_str);
        assert_eq!(kind, Some("minecraft:blaze"));
        World::set_gateway_exit(&mut server.level, 4, 205, 4, (1, 2, 3), false);
        assert_eq!(block_entity(&server, (4, 205, 4)).and_then(|s| s.get("exit_portal").cloned()), None, "a spawner takes no exit");
        let uuid = World::next_uuid(&mut server.level);
        assert_ne!(uuid, World::next_uuid(&mut server.level));
        let shulker = minecraftoss_core::snbt::parse_compound(&format!("{{id:\"minecraft:shulker\",Pos:[8.5d,206.0d,8.5d],UUID:[I;{},{},{},{}]}}", uuid[0], uuid[1], uuid[2], uuid[3])).unwrap();
        World::add_entity(&mut server.level, shulker);
        server.take_spawned();
        assert!(server.dormant.iter().any(|(group, _)| group[0].kind == "minecraft:shulker"), "the entity joins the server");
        assert_eq!(server.level.unsupported, Vec::<String>::new());
    }

    #[test]
    fn the_server_answers_each_item_use_so_one_source_fills_one_bucket() {
        let Some((mut server, _)) = door_world() else { return };
        assert!(server.item_on_block((8, 200, 8), ItemOnBlock::EmptyBucket { water: true }));
        server.take_changes();
        let mut handle = ServerHandle::spawn(server);
        handle.item_on_block(0, (8, 200, 8), ItemOnBlock::FillBucket);
        handle.item_on_block(2, (8, 200, 8), ItemOnBlock::FillBucket);
        handle.wait_idle();
        let outputs = handle.poll();
        let answers: Vec<(u64, bool)> = outputs.iter().flat_map(|out| out.item_uses.iter().copied()).collect();
        assert_eq!(answers, vec![(0, true), (2, false)]);
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
        let primary = writes.iter().find(|(pos, _)| *pos == placed).and_then(|(_, b)| b.as_ref());
        if let Some(block) = primary.map(|b| Block { id: crate::pack::ResourceId::parse(&b.id).unwrap(), properties: b.properties.clone() }) {
            if !crate::placing::may_place(server.level.lib, &crate::placing::SurviveView::new(scene, None), placed, &block) {
                return None;
            }
        }
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
    fn a_vine_clicked_onto_a_wall_hangs_flat_and_goes_with_the_wall() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (8, 201, 7), Block::new("minecraft:stone"));
        server.take_changes();
        let placed = place_as_client(&mut server, &mut scene, "minecraft:vine", [8.5, 200.0, 8.5], [8.5, 201.5, 8.0]);
        assert_eq!(placed, Some((8, 201, 8)), "a vine goes into the player's own cell");
        let vine = Scene::block(&scene, (8, 201, 8)).cloned().unwrap();
        assert_eq!(vine.id.key(), "minecraft:vine");
        let sides: Vec<&str> = ["up", "north", "east", "south", "west"].into_iter().filter(|side| vine.properties.get(*side).map(String::as_str) == Some("true")).collect();
        assert_eq!(sides, ["north"]);
        place(&mut server, &mut scene, (12, 199, 12), Block::new("minecraft:stone"));
        assert_eq!(place_as_client(&mut server, &mut scene, "minecraft:vine", [12.5, 200.0, 12.5], [12.5, 199.9, 12.5]), None, "a vine never rests on the floor");
        take(&mut server, &mut scene, (8, 201, 7));
        let mut changes: std::collections::HashMap<_, _> = server.take_changes().into_iter().collect();
        changes.extend(ticked(&mut server, 3));
        assert_eq!(changes.get(&(8, 201, 8)), Some(&None), "{changes:?}");
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
        let contents = r#"{"minecraft:container":[{"item":{"count":3,"id":"minecraft:diamond"},"slot":0}]}"#;
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

    fn dirt_box(server: &mut ServerSim, scene: &mut HandcraftedScene, xz: std::ops::RangeInclusive<i32>, ys: std::ops::RangeInclusive<i32>) -> Vec<BlockPos> {
        let mut placed = Vec::new();
        for y in ys {
            for x in xz.clone() {
                for z in xz.clone() {
                    place(server, scene, (x, y, z), Block::new("minecraft:dirt"));
                    placed.push((x, y, z));
                }
            }
        }
        placed
    }

    #[test]
    fn an_mw2_blast_breaks_blocks_by_vanilla_resistance() {
        let Some((mut server, mut scene)) = door_world() else { return };
        dirt_box(&mut server, &mut scene, 5..=11, 199..=199);
        place(&mut server, &mut scene, (8, 200, 9), Block::new("minecraft:obsidian"));
        place(&mut server, &mut scene, (8, 199, 10), Block::new("minecraft:obsidian"));
        server.tick();
        server.take_changes();
        server.weapon_blast([8.5, 200.2, 8.5], 256.0 / 72.0);
        server.tick();
        let changes = server.take_changes();
        assert!(changes.contains(&((8, 199, 8), None)), "{changes:?}");
        assert!(!changes.iter().any(|(pos, _)| *pos == (8, 200, 9) || *pos == (8, 199, 10)), "{changes:?}");
    }

    #[test]
    fn an_mw2_blast_drops_one_in_its_power_of_the_blocks_it_breaks() {
        let Some((mut server, mut scene)) = door_world() else { return };
        let dirt = dirt_box(&mut server, &mut scene, 4..=12, 197..=199);
        server.tick();
        server.take_changes();
        let before = picked(&server, "minecraft:dirt");
        server.weapon_blast([8.5, 200.2, 8.5], 256.0 / 72.0);
        server.tick();
        let changes = server.take_changes();
        let broken = changes.iter().filter(|(pos, block)| block.is_none() && dirt.contains(pos)).count() as i32;
        let dropped = picked(&server, "minecraft:dirt") - before;
        assert!(broken > 8, "{broken}");
        assert!(0 < dropped && dropped < broken, "{dropped} of {broken}");
        assert!(server.orbs().is_empty());
    }

    #[test]
    fn an_mw2_blast_primes_tnt_it_reaches() {
        let Some((mut server, mut scene)) = door_world() else { return };
        place(&mut server, &mut scene, (9, 199, 8), Block::new("minecraft:stone"));
        place(&mut server, &mut scene, (9, 200, 8), Block::new("minecraft:tnt"));
        server.tick();
        server.take_changes();
        assert!(server.primed_tnt().is_empty());
        server.weapon_blast([8.5, 200.5, 8.5], 3.0);
        server.tick();
        assert!(server.take_changes().contains(&((9, 200, 8), None)));
        assert_eq!(server.primed_tnt().len(), 1);
    }

    #[test]
    fn an_mw2_blast_sent_to_the_server_thread_comes_back_as_block_changes() {
        let Some((mut server, mut scene)) = door_world() else { return };
        dirt_box(&mut server, &mut scene, 7..=9, 199..=199);
        server.tick();
        server.take_changes();
        let mut handle = ServerHandle::spawn(server);
        handle.weapon_blast([8.5, 200.2, 8.5], 3.0);
        handle.wait_idle();
        let blown = handle.poll().iter().flat_map(|out| out.changes.iter()).any(|(pos, block)| *pos == (8, 199, 8) && block.is_none());
        assert!(blown, "the output carries the crater");
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

    fn burn_world() -> Option<(ServerSim, HandcraftedScene)> {
        let (mut server, mut scene) = door_world()?;
        for x in 2..=14 {
            for z in 2..=14 {
                place(&mut server, &mut scene, (x, 200, z), Block::new("minecraft:stone"));
            }
        }
        server.take_changes();
        Some((server, scene))
    }

    fn lava_pool(server: &mut ServerSim, scene: &mut HandcraftedScene, centre: BlockPos) {
        for dx in -1..=1 {
            for dz in -1..=1 {
                let at = (centre.0 + dx, centre.1, centre.2 + dz);
                if at != centre {
                    place(server, scene, at, Block::new("minecraft:stone"));
                }
            }
        }
        place(server, scene, centre, Block::new("minecraft:lava").with("level", "0"));
        server.take_changes();
    }

    fn burn_sounds(server: &ServerSim) -> Vec<&minecraftoss_world::level::burning::LevelSound> {
        server.level.sounds.iter().filter(|s| s.event == "minecraft:entity.generic.burn").collect()
    }

    fn alive(server: &ServerSim, id: i32) -> Option<ServerItem> {
        server.items().into_iter().find(|i| i.id == id)
    }

    #[test]
    fn lava_burns_a_dropped_item_with_a_sizzle() {
        let Some((mut server, mut scene)) = burn_world() else { return };
        lava_pool(&mut server, &mut scene, (8, 201, 8));
        let id = server.spawn_item("minecraft:cobblestone", 3, None, [8.5, 202.5, 8.5], [0.0; 3], 10, 0);
        let mut ticks = 0;
        while alive(&server, id).is_some() && ticks < 40 {
            server.tick();
            ticks += 1;
        }
        assert!(alive(&server, id).is_none(), "the item should burn up in lava");
        let sounds = burn_sounds(&server);
        assert!(!sounds.is_empty(), "{:?}", server.level.sounds);
        assert!(sounds.iter().all(|s| s.volume == 0.4 && (2.0..2.4).contains(&s.pitch)), "{sounds:?}");
    }

    #[test]
    fn netherite_floats_on_lava_and_survives() {
        let Some((mut server, mut scene)) = burn_world() else { return };
        lava_pool(&mut server, &mut scene, (5, 201, 5));
        lava_pool(&mut server, &mut scene, (11, 201, 11));
        let ingot = server.spawn_item("minecraft:netherite_ingot", 1, None, [5.5, 202.5, 5.5], [0.0; 3], 10, 0);
        let debris = server.spawn_item("minecraft:ancient_debris", 1, None, [11.5, 202.5, 11.5], [0.0; 3], 10, 0);
        for _ in 0..200 {
            server.tick();
        }
        let mut heights = Vec::new();
        for _ in 0..100 {
            server.tick();
            for id in [ingot, debris] {
                let item = alive(&server, id).expect("fire-resistant items survive lava");
                assert!(!item.on_fire);
                heights.push(item.position[1]);
            }
        }
        let (low, high) = heights.iter().fold((f64::MAX, f64::MIN), |(lo, hi), &y| (lo.min(y), hi.max(y)));
        assert!(low > 201.4 && high < 202.0 && high > 201.7, "{low} {high}");
        assert!(burn_sounds(&server).is_empty());
    }

    #[test]
    fn fire_and_soul_fire_burn_items_silently() {
        let Some((mut server, mut scene)) = burn_world() else { return };
        place(&mut server, &mut scene, (5, 201, 5), Block::new("minecraft:netherrack"));
        place(&mut server, &mut scene, (5, 202, 5), Block::new("minecraft:fire"));
        place(&mut server, &mut scene, (11, 201, 11), Block::new("minecraft:soul_sand"));
        place(&mut server, &mut scene, (11, 202, 11), Block::new("minecraft:soul_fire"));
        place(&mut server, &mut scene, (8, 201, 11), Block::new("minecraft:netherrack"));
        place(&mut server, &mut scene, (8, 202, 11), Block::new("minecraft:fire"));
        server.take_changes();
        let log = server.spawn_item("minecraft:oak_log", 64, None, [5.5, 202.0, 5.5], [0.0; 3], 10, 0);
        let flesh = server.spawn_item("minecraft:rotten_flesh", 1, None, [11.5, 202.2, 11.5], [0.0; 3], 10, 0);
        let ingot = server.spawn_item("minecraft:netherite_ingot", 1, None, [8.5, 202.0, 11.5], [0.0; 3], 10, 0);
        server.tick();
        assert!(alive(&server, log).is_some_and(|i| i.on_fire), "an item in fire burns");
        for _ in 0..9 {
            server.tick();
        }
        assert!(alive(&server, log).is_none() && alive(&server, flesh).is_none(), "{:?}", server.items());
        for _ in 0..30 {
            server.tick();
        }
        assert!(alive(&server, ingot).is_some_and(|i| !i.on_fire));
        assert!(server.level.sounds.is_empty(), "{:?}", server.level.sounds);
    }

    #[test]
    fn a_lit_campfire_leaves_items_alone() {
        let Some((mut server, mut scene)) = burn_world() else { return };
        place(&mut server, &mut scene, (8, 201, 8), Block::new("minecraft:campfire"));
        server.take_changes();
        let at = minecraftoss_core::BlockPos::new(8, 201, 8);
        assert_eq!(server.level.registries().blocks.property(server.level.block(at), "lit"), Some("true"));
        let id = server.spawn_item("minecraft:cobblestone", 1, None, [8.5, 201.6, 8.5], [0.0; 3], 10, 0);
        for _ in 0..100 {
            server.tick();
        }
        assert!(alive(&server, id).is_some_and(|i| !i.on_fire && i.position[1] < 201.5));
        assert!(server.level.sounds.is_empty());
    }

    #[test]
    fn a_saved_burning_item_keeps_its_fire_and_health() {
        let Some((mut server, mut scene)) = burn_world() else { return };
        place(&mut server, &mut scene, (5, 201, 5), Block::new("minecraft:netherrack"));
        place(&mut server, &mut scene, (5, 202, 5), Block::new("minecraft:fire"));
        server.take_changes();
        let id = server.spawn_item("minecraft:oak_log", 1, None, [5.5, 202.0, 5.5], [0.0; 3], 10, 0);
        server.tick();
        let state = |server: &ServerSim| server.level.entities.iter().find(|e| e.item_data().is_some()).map(|e| (e.remaining_fire_ticks, e.item_data().map_or(0, |d| d.health)));
        let before = state(&server);
        assert_eq!(before, Some((160, 4)), "{:?}", alive(&server, id));
        let tags = crate::server_mobs::level_entity_tags(&server.level, |_| true);
        server.level.entities.clear();
        for tag in &tags {
            assert!(crate::server_mobs::load_level_entity(&mut server.level, tag));
        }
        assert_eq!(state(&server), before);
    }

    #[test]
    fn an_enchanted_dropped_item_saves_its_components_as_vanilla_nbt() {
        use minecraftoss_core::nbt::Tag;
        let Some((mut server, _)) = door_world() else { return };
        let components = r#"{"minecraft:custom_name":"Excalibur","minecraft:enchantments":{"minecraft:sharpness":5}}"#;
        let id = server.spawn_item("minecraft:diamond_sword", 1, Some(components), [8.5, 201.0, 8.5], [0.0; 3], 10, 0);
        assert_eq!(alive(&server, id).and_then(|i| i.components).as_deref(), Some(components));
        let sword = |server: &ServerSim| crate::server_mobs::level_entity_tags(&server.level, |_| true).into_iter().find(|tag| tag.get("Item").and_then(|item| item.get("id")).and_then(Tag::as_str) == Some("minecraft:diamond_sword"));
        let saved = sword(&server).expect("the sword is saved");
        let saved_components = saved.get("Item").and_then(|item| item.get("components")).expect("components");
        assert_eq!(saved_components.get("minecraft:enchantments").and_then(|e| e.get("minecraft:sharpness")), Some(&Tag::Int(5)));
        assert_eq!(saved_components.get("minecraft:custom_name"), Some(&Tag::String("Excalibur".to_owned())));
        server.level.entities.clear();
        assert!(crate::server_mobs::load_level_entity(&mut server.level, &saved));
        let loaded = server.items().into_iter().find(|i| i.item == "minecraft:diamond_sword");
        assert_eq!(loaded.and_then(|i| i.components).as_deref(), Some(components));
        let mut old = saved.clone();
        if let Tag::Compound(map) = &mut old {
            if let Some(Tag::Compound(item)) = map.get_mut("Item") {
                item.insert("components".to_owned(), Tag::String(components.to_owned()));
            }
        }
        server.level.entities.clear();
        assert!(crate::server_mobs::load_level_entity(&mut server.level, &old));
        let resaved = sword(&server).expect("the old sword is saved again");
        assert_eq!(resaved.get("Item").and_then(|item| item.get("components")), Some(saved_components));
    }

    #[test]
    fn hoppers_hand_their_changed_items_to_the_chunk_map_and_leave_chests_to_the_containers() {
        use minecraftoss_core::nbt::Tag;
        let Some((mut server, mut scene)) = door_world() else { return };
        if server.level.registries().block_entities.is_none() {
            return;
        }
        let (hopper, chest) = ((8, 201, 8), (8, 200, 8));
        place(&mut server, &mut scene, chest, Block::new("minecraft:chest").with("facing", "north").with("type", "single").with("waterlogged", "false"));
        place(&mut server, &mut scene, hopper, Block::new("minecraft:hopper").with("facing", "down").with("enabled", "true"));
        server.take_changes();
        server.take_block_entity_changes();
        let at = |pos: BlockPos| minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        let dirt = |tag: &Tag| tag.get("Items").and_then(Tag::as_list).map_or(0, |items| items.iter().filter_map(|item| item.get("count").and_then(Tag::as_i64)).sum::<i64>());
        let positions = |changes: &[(BlockPos, Option<Tag>)]| changes.iter().map(|(pos, _)| *pos).collect::<Vec<_>>();
        assert!(server.level.replace_block_item(at(hopper), 0, minecraftoss_core::item::ItemStack::new("minecraft:dirt", 3)));
        let filled = server.take_block_entity_changes();
        assert_eq!(positions(&filled), vec![hopper]);
        assert_eq!(filled[0].1.as_ref().map(dirt), Some(3));
        server.tick();
        let moved = server.take_block_entity_changes();
        assert_eq!(positions(&moved), vec![hopper], "the chest's items stay the containers'");
        let tag = moved[0].1.as_ref().unwrap();
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some("minecraft:hopper"));
        assert_eq!((dirt(tag), tag.get("TransferCooldown").and_then(Tag::as_i64)), (2, Some(8)));
        assert_eq!(server.level.block_container_items(at(chest)).map(|items| items.iter().map(|s| s.count).sum::<i32>()), Some(1));
        ticked(&mut server, 40);
        let emptied = server.take_block_entity_changes();
        assert_eq!(positions(&emptied), vec![hopper]);
        assert_eq!(emptied[0].1.as_ref().map(dirt), Some(0));
        ticked(&mut server, 5);
        assert!(server.take_block_entity_changes().is_empty(), "an idle hopper changes nothing");
        server.level.replace_block_item(at(hopper), 0, minecraftoss_core::item::ItemStack::new("minecraft:dirt", 1));
        server.unload_chunk(ChunkPos::new(0, 0));
        let unloaded = server.take_block_entity_changes();
        assert_eq!(positions(&unloaded), vec![hopper], "a chunk's last changes are handed over before it leaves");
        assert_eq!(unloaded[0].1.as_ref().map(dirt), Some(1));
    }

    #[test]
    fn a_broken_dropper_hands_over_that_its_items_are_gone() {
        let Some((mut server, mut scene)) = door_world() else { return };
        if server.level.registries().block_entities.is_none() {
            return;
        }
        let dropper = (8, 200, 8);
        place(&mut server, &mut scene, dropper, Block::new("minecraft:dropper").with("facing", "up").with("triggered", "false"));
        server.take_changes();
        server.take_block_entity_changes();
        let at = minecraftoss_core::BlockPos::new(dropper.0, dropper.1, dropper.2);
        assert!(server.level.replace_block_item(at, 0, minecraftoss_core::item::ItemStack::new("minecraft:dirt", 5)));
        let filled = server.take_block_entity_changes();
        assert_eq!(filled.len(), 1);
        assert!(filled[0].1.is_some());
        take(&mut server, &mut scene, dropper);
        assert_eq!(
            server.take_block_entity_changes(),
            vec![(dropper, None)],
            "the chunk map must not keep the items for a dropper placed there later"
        );
    }

    #[test]
    fn command_placements_shape_like_vanilla_and_jobs_answer() {
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
        let blocks = &registries.blocks;
        let fence = blocks.parse_state("minecraft:oak_fence").unwrap();
        let side = |server: &ServerSim, pos: BlockPos, name: &str| blocks.property(server.command_block(pos), name).map(str::to_owned);
        assert!(server.command_place((8, 200, 8), fence, &[], false));
        assert!(server.command_place((9, 200, 8), fence, &[], false));
        assert_eq!(side(&server, (8, 200, 8), "east").as_deref(), Some("true"));
        assert_eq!(side(&server, (9, 200, 8), "west").as_deref(), Some("true"));
        assert!(!server.command_place((9, 200, 8), fence, &[], false));
        assert!(server.command_place((8, 200, 10), fence, &[], true));
        assert!(server.command_place((9, 200, 10), fence, &[], true));
        assert_eq!(side(&server, (8, 200, 10), "east").as_deref(), Some("false"));
        assert_eq!(side(&server, (9, 200, 10), "west").as_deref(), Some("false"));
        let defined = [("west".to_owned(), "false".to_owned()), ("waterlogged".to_owned(), "true".to_owned())];
        assert!(server.command_place((10, 200, 8), fence, &defined, false));
        assert_eq!(side(&server, (10, 200, 8), "west").as_deref(), Some("true"));
        assert_eq!(side(&server, (10, 200, 8), "waterlogged").as_deref(), Some("true"));
        assert_eq!(side(&server, (9, 200, 8), "east").as_deref(), Some("true"));
        server.take_changes();
        server.take_destroyed();
        assert!(server.command_destroy((8, 200, 8)));
        assert!(!server.command_destroy((8, 200, 8)));
        assert_eq!(side(&server, (9, 200, 8), "west").as_deref(), Some("false"));
        let destroyed = server.take_destroyed();
        assert_eq!(destroyed.len(), 1);
        assert_eq!(destroyed[0].0, (8, 200, 8));
        assert!(server.items().iter().any(|item| item.item == "minecraft:oak_fence"));
        assert!(server.take_changes().contains(&((8, 200, 8), None)));
        let librarian = ["minecraft:librarian".to_owned()];
        assert_eq!(server.command_closest_poi(&librarian, (8, 200, 8), 256), None);
        assert!(server.command_place((12, 201, 12), blocks.parse_state("minecraft:lectern").unwrap(), &[], false));
        server.sync_pois();
        assert_eq!(server.command_closest_poi(&librarian, (8, 200, 8), 256), Some(((12, 201, 12), "minecraft:librarian".to_owned())));
        assert_eq!(server.command_closest_poi(&["minecraft:home".to_owned()], (8, 200, 8), 256), None);
        let mut handle = ServerHandle::spawn(server);
        handle.run_job(Box::new(|sim: &mut ServerSim| -> Box<dyn std::any::Any + Send> { Box::new(sim.command_block((9, 200, 8))) }));
        handle.wait_idle();
        let answers: Vec<BlockStateId> = handle.poll().into_iter().flat_map(|out| out.jobs).filter_map(|job| job.downcast::<BlockStateId>().ok()).map(|state| *state).collect();
        assert_eq!(answers.len(), 1);
        assert_eq!(blocks.property(answers[0], "east"), Some("true"));
        assert_eq!(blocks.property(answers[0], "west"), Some("false"));
    }

    #[test]
    fn fortress_monsters_spawn_on_its_nether_bricks_and_structures_override_their_biome() {
        use minecraftoss_world::natural_spawner::spawning::MobCategory;
        type BlockPos = minecraftoss_core::BlockPos;
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::for_dimension(Arc::new(TerrainGenerator::nether(registries.clone(), 77).unwrap()), "minecraft:the_nether").unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, 0, 256).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen.clone(), states, "minecraft:the_nether");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        let at = match crate::locate::nearest_structure(&worldgen, &["minecraft:fortress".to_owned()], [0, 64, 0], crate::locate::STRUCTURE_RADIUS, deadline) {
            crate::locate::Search::Found(at, _) => at,
            other => panic!("no fortress near the origin of seed 77: {other:?}"),
        };
        let fortress = worldgen.structures.id("minecraft:fortress").unwrap();
        let start = worldgen.structures.starts_in(&worldgen.library, &worldgen.terrain, ChunkPos::new(at[0] >> 4, at[2] >> 4)).iter().find(|start| start.structure == fortress).cloned().expect("the fortress starts in its chunk");
        let pieces: Vec<minecraftoss_generator::feature::template::BoundingBox> = start.pieces.lock().unwrap().iter().map(|piece| piece.base().bbox).collect();
        let bbox = start.bounding_box();
        let in_a_piece = |pos: BlockPos| pieces.iter().any(|piece| piece.is_inside(pos));
        let inside = pieces[0].center();
        let between = (bbox.min_x..=bbox.max_x)
            .flat_map(|x| (bbox.min_z..=bbox.max_z).map(move |z| BlockPos::new(x, bbox.min_y + 2, z)))
            .find(|&pos| !in_a_piece(pos) && !in_a_piece(pos.below()))
            .expect("a spot inside the fortress's bounds but outside its pieces");
        let far = BlockPos::new(at[0] + 2000, 64, at[2] + 2000);
        for pos in [inside, between, far] {
            server.load_chunk(&map.load_now(ChunkPos::new(pos.x >> 4, pos.z >> 4)));
        }
        let natural = server.level.natural_spawning.as_ref().unwrap();
        let (spawns, finder) = (natural.spawns.clone(), natural.structures.clone().unwrap());
        let here = finder(inside);
        assert!(here.iter().any(|s| s.name == "minecraft:fortress" && s.in_box && s.in_piece), "{:?}", here.iter().map(|s| (&s.name, s.in_box, s.in_piece)).collect::<Vec<_>>());
        assert!(finder(between).iter().any(|s| s.name == "minecraft:fortress" && s.in_box && !s.in_piece));
        assert!(finder(far).is_empty());
        let enemies: Vec<String> = ["blaze", "zombified_piglin", "wither_skeleton", "skeleton", "magma_cube"].iter().map(|kind| format!("minecraft:{kind}")).collect();
        let bricks = Block::new("minecraft:nether_bricks");
        let netherrack = Block::new("minecraft:netherrack");
        let kinds = |server: &mut ServerSim, pos: BlockPos, below: &Block, category: MobCategory| {
            server.player_edit_block((pos.x, pos.y - 1, pos.z), Some(below), PlayerEdit::Place);
            spawns.kinds_at(&server.level, &*finder, category, pos)
        };
        assert_eq!(kinds(&mut server, inside, &netherrack, MobCategory::Monster), enemies, "a fortress piece spawns its monsters on any block");
        assert_eq!(kinds(&mut server, between, &bricks, MobCategory::Monster), enemies, "nether bricks inside the fortress's bounds spawn them too");
        let biome = kinds(&mut server, between, &netherrack, MobCategory::Monster);
        assert_ne!(biome, enemies, "elsewhere in the bounds the biome decides");
        assert!(kinds(&mut server, inside, &netherrack, MobCategory::Creature).iter().all(|kind| kind == "minecraft:strider"), "only monsters are overridden");
    }

    #[test]
    fn zombified_piglins_hold_gold_keep_their_anger_and_come_out_of_overworld_portals() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let level_of = |terrain: TerrainGenerator, dimension_type: &str, min_y: i32, height: i32| {
            let worldgen = Arc::new(WorldGen::for_dimension(Arc::new(terrain), dimension_type).unwrap());
            let states = Arc::new(BlockStates::new(registries.clone(), 0, min_y, height).unwrap());
            let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
            let mut server = ServerSim::new(worldgen, states, dimension_type);
            for x in -1..=1 {
                for z in -1..=1 {
                    server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
                }
            }
            server
        };
        let mut nether = level_of(TerrainGenerator::nether(registries.clone(), 0).unwrap(), "minecraft:the_nether", 0, 256);
        nether.set_players(&[[0.5, 100.0, 0.5]], 2);
        nether.summon("minecraft:zombified_piglin", [0.5, 100.0, 0.5], None, 0.0).unwrap();
        let piglin = &nether.mobs.zombies()[0];
        assert_eq!(piglin.zombie.kind, minecraftoss_entities::zombie::ZombieKind::ZombifiedPiglin);
        assert!(matches!(piglin.zombie.main_hand.as_deref(), Some("minecraft:golden_sword" | "minecraft:golden_spear")), "{:?}", piglin.zombie.main_hand);
        assert!(piglin.zombie.armor.iter().all(Option::is_none), "no armour");
        let id = piglin.id;
        nether.mobs.zombie_mut(id).unwrap().ai.as_deref_mut().unwrap().state.enderman.anger_end_time = 1234;
        let tags = crate::server_mobs::mob_tags(&nether.mobs, |_| true, &nether.mob_tags);
        let (_, tag) = tags.iter().find(|(tag_id, _)| *tag_id == id).expect("the piglin is saved");
        assert_eq!(tag.get("anger_end_time").and_then(minecraftoss_core::nbt::Tag::as_i64), Some(1234));
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        let back = crate::server_mobs::spawn_saved(&mut loaded, tag).expect("zombified piglins load");
        let back = loaded.zombie_mut(back).unwrap();
        assert_eq!(back.zombie.kind, minecraftoss_entities::zombie::ZombieKind::ZombifiedPiglin);
        assert_eq!(back.zombie.main_hand, nether.mobs.zombies()[0].zombie.main_hand);
        assert_eq!(back.ai.as_ref().unwrap().state.enderman.anger_end_time, 1234);
        let mut overworld = level_of(TerrainGenerator::overworld(registries.clone(), 0).unwrap(), "minecraft:overworld", -64, 384);
        let obsidian = Block::new("minecraft:obsidian");
        for (x, y) in [(-1, 200), (0, 200), (1, 200), (2, 200), (-1, 201), (-1, 202), (-1, 203), (2, 201), (2, 202), (2, 203), (-1, 204), (0, 204), (1, 204), (2, 204)] {
            overworld.player_edit_block((x, y, 0), Some(&obsidian), PlayerEdit::Place);
        }
        let portal = Block::new("minecraft:nether_portal").with("axis", "x");
        for x in 0..=1 {
            for y in 201..=203 {
                overworld.player_edit_block((x, y, 0), Some(&portal), PlayerEdit::Place);
            }
        }
        let spawned = |server: &mut ServerSim, difficulty: i32, player: [f64; 3]| {
            server.set_players(&[player], difficulty);
            let candidate = minecraftoss_entities::tempt::PlayerCandidate {
                id: 0,
                position: glam::DVec3::from_array(player),
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
            server.prepare_spawning(&[candidate]);
            server.level.spawned.clear();
            for _ in 0..20000 {
                server.level.nether_portal_random_tick(minecraftoss_core::BlockPos::new(1, 203, 0));
            }
            std::mem::take(&mut server.level.spawned)
        };
        let near = [8.5, 200.0, 8.5];
        let piglins = spawned(&mut overworld, 3, near);
        assert!((10..=60).contains(&piglins.len()), "about 30 in 20000 random ticks on hard: {}", piglins.len());
        for tag in &piglins {
            assert_eq!(tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str), Some("minecraft:zombified_piglin"));
            assert_eq!(tag.get("PortalCooldown").and_then(minecraftoss_core::nbt::Tag::as_i64), Some(300));
            let pos = tag.get("Pos").and_then(minecraftoss_core::nbt::Tag::as_list).unwrap().iter().filter_map(minecraftoss_core::nbt::Tag::as_f64).collect::<Vec<_>>();
            assert_eq!(pos, vec![1.5, 201.0, 0.5], "on the portal's floor");
        }
        assert!(spawned(&mut overworld, 0, near).is_empty(), "none in peaceful");
        assert!(spawned(&mut overworld, 3, [500.0, 200.0, 500.0]).is_empty(), "none with nobody near");
    }

    #[test]
    fn magma_cubes_spawn_in_three_sizes_and_come_back_as_magma_cubes() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::for_dimension(Arc::new(TerrainGenerator::nether(registries.clone(), 0).unwrap()), "minecraft:the_nether").unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, 0, 256).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:the_nether");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        server.set_players(&[[0.5, 100.0, 0.5]], 2);
        let mut sizes = std::collections::BTreeSet::new();
        for _ in 0..40 {
            server.summon("minecraft:magma_cube", [0.5, 100.0, 0.5], None, 0.0).unwrap();
        }
        for e in server.mobs.slimes() {
            assert!(e.slime.magma);
            sizes.insert(e.slime.size);
        }
        assert!(sizes.iter().all(|size| [1, 2, 4].contains(size)) && sizes.len() > 1, "{sizes:?}");
        let tags = crate::server_mobs::mob_tags(&server.mobs, |_| true, &server.mob_tags);
        for (id, tag) in &tags {
            let cube = &server.mobs.slimes().iter().find(|e| e.id == *id).unwrap().slime;
            assert_eq!(tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str), Some("minecraft:magma_cube"));
            let armor = tag.get("attributes").and_then(minecraftoss_core::nbt::Tag::as_list).and_then(|list| list.iter().find(|a| a.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some("minecraft:armor"))).and_then(|a| a.get("base")).and_then(minecraftoss_core::nbt::Tag::as_f64);
            assert_eq!(armor, Some(f64::from(cube.size * 3)));
            let mut loaded = minecraftoss_entities::world::EntityWorld::default();
            let back = crate::server_mobs::spawn_saved(&mut loaded, tag).expect("magma cubes load");
            let back = &loaded.slime_mut(back).unwrap().slime;
            assert!(back.magma && back.size == cube.size);
        }
    }

    #[test]
    fn wither_skeletons_carry_stone_swords_and_come_back_with_them() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::for_dimension(Arc::new(TerrainGenerator::nether(registries.clone(), 0).unwrap()), "minecraft:the_nether").unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, 0, 256).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:the_nether");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        server.set_players(&[[0.5, 100.0, 0.5]], 2);
        server.summon("minecraft:wither_skeleton", [0.5, 100.0, 0.5], None, 0.0).unwrap();
        let skeleton = &server.mobs.skeletons()[0];
        assert_eq!(skeleton.skeleton.kind, minecraftoss_entities::skeleton::SkeletonKind::WitherSkeleton);
        assert_eq!((skeleton.skeleton.main_hand.as_deref(), skeleton.skeleton.holds_bow), (Some("minecraft:stone_sword"), false));
        let id = skeleton.id;
        let tags = crate::server_mobs::mob_tags(&server.mobs, |_| true, &server.mob_tags);
        let (_, tag) = tags.iter().find(|(tag_id, _)| *tag_id == id).unwrap();
        let attack = tag.get("attributes").and_then(minecraftoss_core::nbt::Tag::as_list).and_then(|list| list.iter().find(|a| a.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some("minecraft:attack_damage"))).and_then(|a| a.get("base")).and_then(minecraftoss_core::nbt::Tag::as_f64);
        assert_eq!(attack, Some(4.0));
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        let back = crate::server_mobs::spawn_saved(&mut loaded, tag).expect("wither skeletons load");
        let back = &loaded.skeleton_mut(back).unwrap().skeleton;
        assert_eq!((back.kind, back.main_hand.as_deref(), back.body.height), (minecraftoss_entities::skeleton::SkeletonKind::WitherSkeleton, Some("minecraft:stone_sword"), 2.4));
    }

    #[test]
    fn a_fortress_spawner_spawns_blazes_near_a_player_and_waits_between_rounds() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::for_dimension(Arc::new(TerrainGenerator::nether(registries.clone(), 77).unwrap()), "minecraft:the_nether").unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 77, 0, 256).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        let at = match crate::locate::nearest_structure(&worldgen, &["minecraft:fortress".to_owned()], [0, 64, 0], crate::locate::STRUCTURE_RADIUS, deadline) {
            crate::locate::Search::Found(at, _) => at,
            other => panic!("no fortress near the origin of seed 77: {other:?}"),
        };
        let mut run = |player_offset: f64, rule: bool, ticks: usize| {
            let mut server = ServerSim::new(worldgen.clone(), states.clone(), "minecraft:the_nether");
            let (cx, cz) = (at[0] >> 4, at[2] >> 4);
            for x in cx - 3..=cx + 3 {
                for z in cz - 3..=cz + 3 {
                    server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
                }
            }
            let spawner = server.level.chunks().flat_map(|chunk| chunk.block_entities.entities.iter()).find(|(_, tag)| tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some("minecraft:mob_spawner")).map(|(key, _)| *key).expect("the fortress has a spawner");
            let mut rules = crate::game_rules::GameRules::default();
            if !rule {
                rules.set("spawner_blocks_work", "false").unwrap();
            }
            server.apply_game_rules(&rules);
            server.level.natural_spawning.as_mut().unwrap().spawn_mobs = false;
            let feet = glam::DVec3::new(f64::from(spawner.0) + 0.5 + player_offset, f64::from(spawner.1), f64::from(spawner.2) + 0.5);
            let player = minecraftoss_entities::tempt::PlayerCandidate {
                id: 0,
                position: feet,
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
            let mut spawned = Vec::new();
            for _ in 0..ticks {
                server.set_players(&[feet.to_array()], 2);
                server.set_simulation_area((spawner.0 >> 4, spawner.2 >> 4), 4);
                server.prepare_spawning(&[player]);
                server.tick();
                spawned.extend(server.level.spawned.iter().filter(|tag| tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some("minecraft:blaze")).cloned());
                server.take_spawned();
            }
            let delay = server.level.chunk(ChunkPos::new(spawner.0 >> 4, spawner.2 >> 4)).and_then(|chunk| chunk.block_entities.entities.get(&spawner)).and_then(|tag| tag.get("Delay")).and_then(minecraftoss_core::nbt::Tag::as_i64);
            (spawner, spawned, delay)
        };
        let (spawner, blazes, delay) = run(3.0, true, 120);
        assert!(!blazes.is_empty(), "the spawner made blazes");
        assert!(blazes.len() <= 6, "no more than the spawner's limit in one wait: {}", blazes.len());
        for tag in &blazes {
            let pos: Vec<f64> = tag.get("Pos").and_then(minecraftoss_core::nbt::Tag::as_list).unwrap().iter().filter_map(minecraftoss_core::nbt::Tag::as_f64).collect();
            assert!((pos[0] - f64::from(spawner.0) - 0.5).abs() <= 4.0 && (pos[2] - f64::from(spawner.2) - 0.5).abs() <= 4.0 && (pos[1] - f64::from(spawner.1)).abs() <= 1.0, "within its spawn range: {pos:?}");
        }
        assert!(delay.is_some_and(|d| (80..800).contains(&d)), "it waits 200 to 799 ticks after a round: {delay:?}");
        let (_, none, kept) = run(40.0, true, 60);
        assert!(none.is_empty(), "nobody within 16 blocks: nothing");
        assert_eq!(kept, Some(20), "and its first wait does not run down");
        let (_, off, _) = run(3.0, false, 60);
        assert!(off.is_empty(), "spawner_blocks_work off: nothing");
    }

    #[test]
    fn nether_lava_spreads_farther_and_faster_and_the_nether_sea_is_at_32() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let level_of = |terrain: TerrainGenerator, dimension_type: &str| {
            let worldgen = Arc::new(WorldGen::for_dimension(Arc::new(terrain), dimension_type).unwrap());
            let states = Arc::new(BlockStates::new(registries.clone(), 0, 0, 256).unwrap());
            ServerSim::new(worldgen, states, dimension_type)
        };
        let nether = level_of(TerrainGenerator::nether(registries.clone(), 0).unwrap(), "minecraft:the_nether");
        assert_eq!((nether.level.sea_level, nether.level.fast_lava), (32, true));
        let end = level_of(TerrainGenerator::end(registries.clone(), 0).unwrap(), "minecraft:the_end");
        assert_eq!((end.level.sea_level, end.level.fast_lava), (0, false));
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let lava = registries.blocks.block_by_name("minecraft:lava").unwrap();
        let reach = |dimension_type: &str, ticks: usize| {
            let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
            let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
            let mut server = ServerSim::new(worldgen.clone(), states, dimension_type);
            assert_eq!(server.level.sea_level, 63);
            for x in -1..=1 {
                for z in -1..=1 {
                    server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
                }
            }
            for x in -9..=9 {
                for z in -9..=9 {
                    server.player_edit_block((x, 200, z), Some(&Block::new("minecraft:stone")), PlayerEdit::Place);
                }
            }
            server.player_edit_block((0, 201, 0), Some(&Block::new("minecraft:lava").with("level", "0")), PlayerEdit::Place);
            for _ in 0..ticks {
                server.set_players(&[[0.5, 202.0, 0.5]], 2);
                server.set_simulation_area((0, 0), 4);
                server.tick();
            }
            let mut farthest = 0;
            for x in -9..=9 {
                for z in -9..=9 {
                    if registries.blocks.block_of(server.level.block(minecraftoss_core::BlockPos::new(x, 201, z))) == lava {
                        farthest = farthest.max(x.abs() + z.abs());
                    }
                }
            }
            farthest
        };
        assert_eq!(reach("minecraft:overworld", 300), 3, "overworld lava drops two levels a block");
        assert_eq!(reach("minecraft:the_nether", 300), 7, "Nether lava drops one level a block");
        assert!(reach("minecraft:overworld", 75) <= 2, "overworld lava waits 30 ticks a step");
        assert!(reach("minecraft:the_nether", 75) >= 6, "Nether lava waits 10 ticks a step");
    }

    #[test]
    fn in_the_nether_mined_ice_leaves_no_water_and_a_wet_sponge_dries() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        for (dimension_type, melted, sponge) in [("minecraft:overworld", Some("minecraft:water"), "minecraft:wet_sponge"), ("minecraft:the_nether", None, "minecraft:sponge")] {
            let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
            let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
            let mut server = ServerSim::new(worldgen.clone(), states, dimension_type);
            for x in -1..=1 {
                for z in -1..=1 {
                    server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
                }
            }
            for x in 8..=10 {
                server.player_edit_block((x, 199, 8), Some(&Block::new("minecraft:stone")), PlayerEdit::Place);
            }
            server.player_edit_block((8, 200, 8), Some(&Block::new("minecraft:ice")), PlayerEdit::Place);
            let changes = mined_to(&mut server, &[((8, 200, 8), HowBroken::Mined)]);
            assert_eq!(changes.get(&(8, 200, 8)), Some(&melted.map(str::to_owned)), "{dimension_type}: mined ice on stone");
            server.player_edit_block((10, 200, 8), Some(&Block::new("minecraft:wet_sponge")), PlayerEdit::Place);
            let placed = server.level.block(minecraftoss_core::BlockPos::new(10, 200, 8));
            assert_eq!(registries.blocks.block(registries.blocks.block_of(placed)).name.as_str(), sponge, "{dimension_type}: a placed wet sponge");
        }
    }

    #[test]
    fn a_blast_hurts_mobs_and_players_breaks_blocks_and_may_start_fires() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let fire = registries.blocks.block_by_name("minecraft:fire").unwrap();
        let planks = registries.blocks.block_by_name("minecraft:oak_planks").unwrap();
        let player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: glam::DVec3::new(-2.5, 201.0, 0.5),
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
        let blow_up = |fire_on: bool, respawn_point: bool, shield: Option<BlockPos>| {
            let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
            let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
            let mut server = ServerSim::new(worldgen.clone(), states, "minecraft:overworld");
            for x in -1..=1 {
                for z in -1..=1 {
                    server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
                }
            }
            for x in -8..=8 {
                for z in -8..=8 {
                    server.player_edit_block((x, 200, z), Some(&Block::new("minecraft:obsidian")), PlayerEdit::Place);
                }
            }
            for z in -2..=2 {
                server.player_edit_block((1, 201, z), Some(&Block::new("minecraft:oak_planks")), PlayerEdit::Place);
            }
            let zombie = server.mobs.spawn_zombie(minecraftoss_entities::zombie::Zombie::new(glam::DVec3::new(0.5, 201.0, 3.5)), true);
            server.take_changes();
            if respawn_point {
                server.respawn_point_blast(Blast { center: [0.5, 201.5, 0.5], power: 5.0, fire: fire_on }, shield);
            } else {
                server.blast(Blast { center: [0.5, 201.5, 0.5], power: 5.0, fire: fire_on });
            }
            server.set_players(&[player.position.to_array()], 2);
            server.set_simulation_area((0, 0), 4);
            server.tick();
            server.tick_mobs(&[player], false);
            let hits = server.mobs.take_player_hits();
            let blasts = server.take_explosions();
            let health = server.mobs.zombies().iter().find(|z| z.id == zombie).map_or(0.0, |z| z.zombie.health);
            let mut fires = 0;
            let mut planks_left = 0;
            for x in -8..=8 {
                for z in -8..=8 {
                    let block = registries.blocks.block_of(server.level.block(minecraftoss_core::BlockPos::new(x, 201, z)));
                    fires += usize::from(block == fire);
                    planks_left += usize::from(block == planks);
                }
            }
            (hits, blasts, health, fires, planks_left)
        };
        let (hits, blasts, health, fires, planks_left) = blow_up(true, false, None);
        assert_eq!(blasts.len(), 1, "the client hears the blast");
        assert_eq!(blasts[0].radius, 5.0);
        let hit = hits.iter().find(|h| matches!(h.kind, minecraftoss_entities::world::PlayerHitKind::Explosion { .. })).expect("the blast hits the player");
        assert_eq!(hit.source, None, "a block's blast has no attacker");
        assert!(hit.damage > 10.0, "a close power 5 blast hurts: {}", hit.damage);
        assert!(health < 20.0, "the zombie is hurt: {health}");
        assert!(planks_left < 5, "the blast breaks the planks: {planks_left} left");
        assert!(fires > 0, "fire lands on the obsidian");
        let (_, _, _, fires, _) = blow_up(false, false, None);
        assert_eq!(fires, 0, "a blast without fire starts none");
        let (hits, blasts, health, fires, planks_left) = blow_up(true, true, None);
        assert_eq!(blasts.len(), 1, "the client hears a bed's blast");
        assert!(!hits.iter().any(|h| matches!(h.kind, minecraftoss_entities::world::PlayerHitKind::Explosion { .. })));
        let hit = hits.iter().find(|h| matches!(h.kind, minecraftoss_entities::world::PlayerHitKind::BadRespawnPoint { .. })).expect("a bed's blast is a bad respawn point");
        assert_eq!(hit.source, None);
        assert!(hit.damage > 10.0, "a bed's blast hurts as much: {}", hit.damage);
        assert!(health < 20.0 && planks_left < 5 && fires > 0, "and does the same to mobs and blocks");
        let (hits, blasts, health, fires, planks_left) = blow_up(true, true, Some((0, 201, 0)));
        assert_eq!(blasts.len(), 1, "the client hears an anchor's blast beside water");
        assert!(hits.iter().any(|h| matches!(h.kind, minecraftoss_entities::world::PlayerHitKind::BadRespawnPoint { .. }) && h.damage > 10.0));
        assert!(health < 20.0, "it hurts mobs as much");
        assert_eq!((planks_left, fires), (5, 0), "water at its centre keeps every block and starts no fire");
    }

    #[test]
    fn ghast_fireballs_blow_up_walls_unless_mobs_may_not_grief_and_ghasts_keep_their_power() {
        use minecraftoss_entities::fireball::{Fireball, FireballKind, Shooter};
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let planks = registries.blocks.block_by_name("minecraft:oak_planks").unwrap();
        let shoot = |griefing: bool| {
            let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
            let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
            let mut server = ServerSim::new(worldgen.clone(), states, "minecraft:overworld");
            for x in -1..=1 {
                for z in -1..=1 {
                    server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
                }
            }
            for x in -2..=2 {
                for y in 200..=204 {
                    server.player_edit_block((x, y, 8), Some(&Block::new("minecraft:oak_planks")), PlayerEdit::Place);
                }
            }
            server.take_changes();
            server.mobs.set_mob_griefing(griefing);
            server.summon("minecraft:ghast", [0.5, 200.0, -6.5], None, 0.0).unwrap();
            let ghast = server.mobs.ghasts()[0].id;
            server.mobs.ghast_mut(ghast).unwrap().no_ai = true;
            server.mobs.spawn_fireball(Fireball::shoot(FireballKind::Large { explosion_power: 1 }, Shooter::Mob(ghast), glam::DVec3::new(0.5, 201.5, -2.0), glam::DVec3::Z));
            server.set_players(&[[0.5, 200.0, -30.5]], 2);
            server.set_simulation_area((0, 0), 4);
            let mut blasts = Vec::new();
            for _ in 0..40 {
                server.tick();
                server.tick_mobs(&[], false);
                blasts.extend(server.take_explosions());
            }
            let mut left = 0;
            for x in -2..=2 {
                for y in 200..=204 {
                    left += usize::from(registries.blocks.block_of(server.level.block(minecraftoss_core::BlockPos::new(x, y, 8))) == planks);
                }
            }
            (blasts, left, server.mobs.fireballs().len())
        };
        let (blasts, left, flying) = shoot(true);
        assert_eq!((blasts.len(), flying), (1, 0), "the fireball bursts on the wall and the client hears it");
        assert_eq!(blasts[0].radius, 1.0);
        assert!((7.0..=8.0).contains(&blasts[0].position.z), "{:?}", blasts[0].position);
        assert!(left < 25, "a power 1 blast breaks planks: {left} left");
        let (blasts, left, _) = shoot(false);
        assert_eq!((blasts.len(), left), (1, 25), "without mob griefing it breaks nothing");
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen.clone(), states, "minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        server.set_players(&[[0.5, 200.0, 0.5]], 2);
        let powerful = minecraftoss_core::snbt::parse_compound("{ExplosionPower:3b}").unwrap();
        server.summon("minecraft:ghast", [0.5, 200.0, 0.5], Some(&powerful), 0.0).unwrap();
        assert_eq!(server.dormant_count(), 0, "ghasts are simulated");
        let ghast = server.mobs.ghasts()[0].id;
        assert_eq!(server.mobs.ghasts()[0].ghast.explosion_power, 3);
        server.mobs.spawn_fireball(Fireball::shoot(FireballKind::Large { explosion_power: 3 }, Shooter::Mob(ghast), glam::DVec3::new(0.5, 202.5, 4.5), glam::DVec3::Z));
        let tags = crate::server_mobs::mob_tags(&server.mobs, |_| true, &server.mob_tags);
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        for (_, tag) in &tags {
            crate::server_mobs::spawn_saved(&mut loaded, tag).expect("ghasts and fireballs load");
        }
        assert_eq!(loaded.ghasts()[0].ghast.explosion_power, 3);
        let fireball = &loaded.fireballs()[0].fireball;
        assert_eq!(fireball.kind, FireballKind::Large { explosion_power: 3 });
        assert!((fireball.velocity - glam::DVec3::new(0.0, 0.0, 0.1)).length() < 1e-9);
        let id = server.mobs.fireballs()[0].id;
        assert!(server.deflect_fireball(id, [0.0, 0.0, -1.0], 0, true));
        assert_eq!(server.mobs.fireballs()[0].fireball.owner, Some(Shooter::Player(0)));
    }

    #[test]
    fn blazes_live_and_their_small_fireballs_set_fire_beside_what_they_strike() {
        use minecraftoss_entities::fireball::{Fireball, FireballKind, Shooter};
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let fire = registries.blocks.block_by_name("minecraft:fire").unwrap();
        let shoot = |griefing: bool| {
            let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
            let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
            let mut server = ServerSim::new(worldgen.clone(), states, "minecraft:overworld");
            for x in -1..=1 {
                for z in -1..=1 {
                    server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
                }
            }
            for x in -2..=2 {
                for y in 200..=204 {
                    server.player_edit_block((x, y, 6), Some(&Block::new("minecraft:obsidian")), PlayerEdit::Place);
                }
                for z in 0..=5 {
                    server.player_edit_block((x, 200, z), Some(&Block::new("minecraft:obsidian")), PlayerEdit::Place);
                }
            }
            server.take_changes();
            server.mobs.set_mob_griefing(griefing);
            server.summon("minecraft:blaze", [0.5, 200.0, -8.5], None, 0.0).unwrap();
            let blaze = server.mobs.blazes()[0].id;
            server.mobs.blaze_mut(blaze).unwrap().no_ai = true;
            server.mobs.spawn_fireball(Fireball::shoot(FireballKind::Small, Shooter::Mob(blaze), glam::DVec3::new(0.5, 201.5, 0.5), glam::DVec3::Z));
            server.set_players(&[[0.5, 200.0, -30.5]], 2);
            server.set_simulation_area((0, 0), 4);
            for _ in 0..20 {
                server.tick();
                server.tick_mobs(&[], false);
            }
            let lit = registries.blocks.block_of(server.level.block(minecraftoss_core::BlockPos::new(0, 201, 5))) == fire;
            (lit, server.mobs.fireballs().len(), server.take_explosions().len())
        };
        assert_eq!(shoot(true), (true, 0, 0), "it bursts on the wall with no blast and sets fire in front of it");
        assert_eq!(shoot(false), (false, 0, 0), "a blaze's fireball lights nothing where mobs may not grief");
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen.clone(), states, "minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        server.set_players(&[[0.5, 200.0, 0.5]], 2);
        server.summon("minecraft:blaze", [0.5, 200.0, 0.5], None, 0.0).unwrap();
        assert_eq!(server.dormant_count(), 0, "blazes are simulated");
        let blaze = server.mobs.blazes()[0].id;
        server.mobs.spawn_fireball(Fireball::shoot(FireballKind::Small, Shooter::Mob(blaze), glam::DVec3::new(0.5, 201.5, 4.5), glam::DVec3::X));
        let tags = crate::server_mobs::mob_tags(&server.mobs, |_| true, &server.mob_tags);
        assert!(tags.iter().any(|(_, tag)| tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some("minecraft:small_fireball") && tag.get("ExplosionPower").is_none()));
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        for (_, tag) in &tags {
            crate::server_mobs::spawn_saved(&mut loaded, tag).expect("blazes and small fireballs load");
        }
        assert_eq!(loaded.blazes().len(), 1);
        assert_eq!(loaded.fireballs()[0].fireball.kind, FireballKind::Small);
    }

    fn end_player(position: [f64; 3]) -> minecraftoss_entities::tempt::PlayerCandidate {
        minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: glam::DVec3::from_array(position),
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
        }
    }

    fn block_name(server: &ServerSim, pos: BlockPos) -> String {
        let state = server.level.block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2));
        let blocks = &server.level.registries().blocks;
        blocks.block(blocks.block_of(state)).name.as_str().to_owned()
    }

    #[test]
    fn end_crystals_burn_on_the_spikes_and_explode_unless_a_blast_destroyed_them() {
        use crate::end_crystal::{EndDamage, EndTarget};
        let Some(mut server) = end_world((-3..=2).flat_map(|x| (-3..=2).map(move |z| (x, z)))) else { return };
        assert_eq!(server.crystals.list.len(), 10, "one crystal on each spike");
        assert_eq!(server.dormant_count(), 0, "the crystals are live, not dormant");
        assert!(server.dragon_fight_level);
        let first = server.crystals.list[0].clone();
        let below = first.block_position();
        assert_eq!(block_name(&server, below), "minecraft:fire");
        assert_eq!(block_name(&server, (below.0, below.1 - 1, below.2)), "minecraft:bedrock");
        server.player_edit_block(below, None, PlayerEdit::Break);
        assert_eq!(block_name(&server, below), "minecraft:air");
        server.tick_end(&[]);
        assert_eq!(block_name(&server, below), "minecraft:fire", "a crystal in the End keeps its fire burning");
        assert_eq!(server.crystals.list[0].time, first.time + 1);
        let near = [first.position.x + 4.0, first.position.y, first.position.z];
        let base = (near[0].floor() as i32, near[1] as i32 - 1, near[2].floor() as i32);
        server.player_edit_block(base, Some(&Block::new("minecraft:obsidian")), PlayerEdit::Place);
        assert!(server.place_end_crystal(base), "a crystal goes on obsidian");
        assert!(!server.place_end_crystal(base), "not twice in one place");
        let placed = server.crystals.list.last().unwrap().clone();
        assert!(!placed.show_bottom);
        assert_eq!(placed.position, crate::end_crystal::placed_at(base));
        let spike_top = (below.0, below.1 - 1, below.2);
        assert!(!server.place_end_crystal(spike_top), "a crystal is in the way");
        server.player_edit_block((base.0, base.1, base.2 + 6), Some(&Block::new("minecraft:end_stone")), PlayerEdit::Place);
        assert!(!server.place_end_crystal((base.0, base.1, base.2 + 6)), "end stone takes no crystal");
        server.end_hit(EndTarget::Crystal(first.id), EndDamage::attack(1.0, 0));
        assert!(server.crystals.list.iter().all(|crystal| crystal.id != first.id));
        assert_eq!(server.blasts.len(), 1);
        assert_eq!(server.blasts[0].0, Blast { center: first.position.to_array(), power: 6.0, fire: false });
        assert_eq!(server.blasts[0].1, BlastSource::Player(0));
        let player = end_player([first.position.x + 3.0, first.position.y, first.position.z + 3.0]);
        server.set_players(&[player.position.to_array()], 2);
        server.tick_mobs(&[player], false);
        let hits = server.mobs.take_player_hits();
        assert_eq!(hits.len(), 1, "the blast reaches the player");
        assert!(matches!(hits[0].kind, minecraftoss_entities::world::PlayerHitKind::Explosion { .. }));
        assert!(hits[0].damage > 20.0, "{}", hits[0].damage);
        server.tick_end(&[]);
        assert!(server.crystals.list.iter().all(|crystal| crystal.id != placed.id), "the blast destroyed the crystal beside it");
        assert!(server.blasts.is_empty(), "a crystal destroyed by a blast does not explode");
        assert_eq!(server.crystals.list.len(), 9);
        let second = server.crystals.list[0].clone();
        server.end_hit(EndTarget::Crystal(second.id), EndDamage::explosion(5.0, Some(0)));
        assert_eq!(server.crystals.list.len(), 8);
        assert!(server.blasts.is_empty(), "an MW2 grenade destroys it without a blast");
        let third = server.crystals.list[0].clone();
        let chunk = ChunkPos::new((third.position.x.floor() as i32) >> 4, (third.position.z.floor() as i32) >> 4);
        let tags = server.crystals.tags_where(|p| crate::server_mobs::in_chunk(p, chunk));
        assert!(!tags.is_empty());
        let mut again = crate::end_crystal::Crystals::default();
        for tag in &tags {
            again.add(tag).expect("crystals load back");
        }
        assert!(again.list.iter().any(|crystal| crystal.position == third.position));
    }

    #[test]
    fn the_fight_dragon_spawns_over_the_island_and_circles_it_while_the_arena_is_loaded() {
        let Some(mut server) = end_world((-5..=4).flat_map(|x| (-5..=4).map(move |z| (x, z)))) else { return };
        let player = end_player([100.5, 49.0, 0.5]);
        server.set_players(&[player.position.to_array()], 2);
        let uuid = server.find_or_create_dragon((0, 0, 0));
        assert_eq!(server.find_or_create_dragon((0, 0, 0)), uuid, "the dragon there is reused");
        assert_eq!(server.dragons.list.len(), 1);
        let dragon = &server.dragons.list[0];
        assert_eq!(dragon.position, glam::DVec3::new(0.0, 128.0, 0.0));
        assert_eq!(dragon.phase, crate::ender_dragon::Phase::HoldingPattern);
        assert_eq!(server.fight_dragon, Some(uuid));
        let mut lowest = f64::MAX;
        let mut targets = std::collections::HashSet::new();
        for _ in 0..1200 {
            server.tick_end(&[player]);
            let dragon = &server.dragons.list[0];
            lowest = lowest.min(dragon.position.y);
            for target in [dragon.holding.target, dragon.strafe.target, dragon.approach.target, dragon.takeoff.target].into_iter().flatten() {
                targets.insert((target.x as i32, target.z as i32));
            }
        }
        assert!(targets.len() >= 3, "{targets:?}");
        assert!(lowest > 60.0, "{lowest}");
        assert_eq!(server.alive_crystals, 10, "the crystals on the spikes were counted");
        let sounds = server.take_end_sounds();
        assert!(sounds.iter().all(|sound| (sound.event == crate::ender_dragon::AMBIENT_SOUND && sound.volume == 5.0) || [crate::ender_dragon::SHOOT_SOUND, crate::dragon_fireball::EXPLODE_SOUND].contains(&sound.event.as_str())));
        let views = server.dragon_views();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].uuid, uuid);
        assert_eq!(views[0].history.len(), crate::ender_dragon::HISTORY_SENT);
        let dragon_chunk = |server: &ServerSim| {
            let p = server.dragons.list[0].position;
            ChunkPos::new((p.x.floor() as i32) >> 4, (p.z.floor() as i32) >> 4)
        };
        let chunk = dragon_chunk(&server);
        let tags = server.dragons.tags_where(|p| crate::server_mobs::in_chunk(p, chunk));
        assert_eq!(tags.len(), 1);
        let mut loaded = crate::ender_dragon::Dragons::default();
        loaded.add(&tags[0]).expect("the dragon loads back");
        assert_eq!(loaded.list[0].uuid, uuid);
        assert_eq!(loaded.list[0].phase, server.dragons.list[0].phase, "it keeps its phase");
    }

    #[test]
    fn mw2_shots_and_blasts_hurt_the_dragon_through_its_parts_until_it_dies_and_is_reported() {
        use crate::end_crystal::{EndDamage, EndTarget};
        let Some(mut server) = end_world((-5..=4).flat_map(|x| (-5..=4).map(move |z| (x, z)))) else { return };
        let uuid = server.find_or_create_dragon((0, 0, 0));
        server.tick_end(&[]);
        let id = server.dragons.list[0].id;
        server.end_hit(EndTarget::DragonPart { id, part: crate::ender_dragon::BODY }, EndDamage::attack(20.0, 0));
        assert_eq!(server.dragons.list[0].health, 194.0, "the body takes a quarter and one");
        let sounds = server.take_end_sounds();
        assert!(sounds.iter().any(|sound| sound.event == crate::ender_dragon::HURT_SOUND));
        for _ in 0..20 {
            server.tick_end(&[]);
        }
        server.end_hit(EndTarget::DragonPart { id, part: crate::ender_dragon::HEAD }, EndDamage::attack(20.0, 0));
        assert_eq!(server.dragons.list[0].health, 174.0, "the head takes it all");
        for _ in 0..20 {
            server.tick_end(&[]);
        }
        let head = server.dragons.list[0].parts[crate::ender_dragon::HEAD];
        server.explosions.push(minecraftoss_entities::creeper::CreeperExplosion { position: head, radius: 3.0, powered: false });
        server.tick_end(&[]);
        server.explosions.clear();
        assert!(server.dragons.list[0].health < 174.0 - 40.0, "a blast at its head: {}", server.dragons.list[0].health);
        for _ in 0..20 {
            server.tick_end(&[]);
        }
        server.end_hit(EndTarget::DragonPart { id, part: crate::ender_dragon::HEAD }, EndDamage::attack(500.0, 0));
        assert_eq!(server.dragons.list[0].phase, crate::ender_dragon::Phase::Dying);
        for _ in 0..600 {
            server.tick_end(&[]);
            if server.dragons.list.is_empty() {
                break;
            }
        }
        assert!(server.dragons.list.is_empty(), "it rose and went");
        assert_eq!(server.take_dragons_killed(), vec![uuid]);
    }

    #[test]
    fn the_twenty_end_gateways_ring_the_island_96_blocks_out_like_vanilla() {
        let expected = [
            (96, 0), (91, 29), (77, 56), (56, 77), (29, 91), (-1, 96), (-30, 91), (-57, 77), (-78, 56), (-92, 29),
            (-96, -1), (-92, -30), (-78, -57), (-57, -78), (-30, -92), (0, -96), (29, -92), (56, -78), (77, -57), (91, -30),
        ];
        for (index, (x, z)) in expected.into_iter().enumerate() {
            assert_eq!(end_gateway_position(index as i32), (x, 75, z), "gateway {index}");
        }
    }

    #[test]
    fn a_kill_opens_the_exit_portal_lays_the_egg_on_the_pillar_and_spawns_a_gateway() {
        let Some(mut server) = end_world((-7..=6).flat_map(|x| (-7..=6).map(move |z| (x, z)))) else { return };
        let block = |server: &ServerSim, pos: BlockPos| {
            let state = server.level.block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2));
            let blocks = &server.level.registries().blocks;
            blocks.block(blocks.block_of(state)).name.as_str().to_owned()
        };
        let at = server.scan_dragon_fight((0, 0, 0), None).exit_portal_location.expect("the inactive podium");
        assert_eq!(block(&server, (1, at.1, 0)), "minecraft:air");
        server.take_end_sounds();
        let opened = server.dragon_killed((0, 0, 0), Some(at), Some(6), true);
        assert_eq!(opened, at);
        for (x, z) in [(1, 0), (-2, 1), (1, 1), (0, 2)] {
            assert_eq!(block(&server, (x, at.1, z)), "minecraft:end_portal", "portal {x} {z}");
        }
        assert_eq!(block(&server, (0, at.1 + 4, 0)), DRAGON_EGG, "the egg sits on the pillar");
        let gateway = end_gateway_position(6);
        assert_eq!(gateway, (-30, 75, 91));
        assert_eq!(block(&server, gateway), "minecraft:end_gateway");
        let sounds = server.take_end_sounds();
        assert_eq!(sounds.len(), 1);
        assert_eq!((sounds[0].event.as_str(), sounds[0].volume), (END_GATEWAY_SPAWN_SOUND, 10.0));
        assert!(server.level.has_active_exit_portal());
        server.dragon_killed((0, 0, 0), Some(at), Some(0), false);
        assert_eq!(block(&server, (0, at.1 + 4, 0)), DRAGON_EGG, "the first egg stays");
        assert_eq!(block(&server, (0, at.1 + 5, 0)), "minecraft:air", "and a later kill lays none");
        assert_eq!(block(&server, end_gateway_position(0)), "minecraft:end_gateway");
        assert!(!server.level.unsupported.iter().any(|note| note.starts_with("feature ")), "{:?}", server.level.unsupported);
    }

    fn respawn_world() -> Option<(ServerSim, BlockPos)> {
        let mut server = end_world((-4..=3).flat_map(|x| (-4..=3).map(move |z| (x, z))))?;
        let at = server.scan_dragon_fight((0, 0, 0), None).exit_portal_location?;
        server.dragon_killed((0, 0, 0), Some(at), None, true);
        server.link_fight(FightLink { dragon: None, previously_killed: true, dragon_killed: true, exit_portal: Some(at), ticking: true });
        Some((server, at))
    }

    fn block_named(server: &ServerSim, pos: BlockPos) -> String {
        let state = server.level.block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2));
        let blocks = &server.level.registries().blocks;
        blocks.block(blocks.block_of(state)).name.as_str().to_owned()
    }

    fn place_respawn_crystals(server: &mut ServerSim, at: BlockPos, cells: std::ops::Range<usize>) {
        for (x, y, z) in crate::dragon_respawn::crystal_cells(at)[cells].iter().copied() {
            assert_eq!(block_named(server, (x, y - 1, z)), "minecraft:bedrock", "the rim under {x} {z}");
            assert!(server.item_on_block((x, y - 1, z), ItemOnBlock::EndCrystal), "a crystal on the rim at {x} {z}");
        }
    }

    #[test]
    fn four_crystals_on_the_exit_portal_rebuild_the_pillars_and_summon_a_new_dragon() {
        use crate::end_crystal::{EndDamage, EndTarget};
        let Some((mut server, at)) = respawn_world() else { return };
        assert_eq!(block_named(&server, (1, at.1, 0)), "minecraft:end_portal");
        let spikes = minecraftoss_generator::feature::kinds::the_end::spikes_for_seed(0);
        let shot: Vec<u64> = server.crystals.list.iter().take(2).map(|crystal| crystal.id).collect();
        for id in shot {
            server.end_hit(EndTarget::Crystal(id), EndDamage::attack(1.0, 0));
        }
        server.blasts.clear();
        assert_eq!(server.crystals.list.len(), 8);
        place_respawn_crystals(&mut server, at, 0..3);
        assert!(server.respawn.is_none(), "three crystals are not enough");
        assert!(server.take_fight_reports().is_empty());
        place_respawn_crystals(&mut server, at, 3..4);
        let reports = server.take_fight_reports();
        assert_eq!(reports.len(), 1);
        let FightReport::Respawning { stage, crystals, exit_portal } = &reports[0] else { panic!("{reports:?}") };
        assert_eq!((*stage, crystals.len(), *exit_portal), ("start", 4, at));
        assert_eq!(block_named(&server, (1, at.1, 0)), "minecraft:air", "the portal goes out");
        assert_eq!(block_named(&server, (0, at.1, 0)), "minecraft:bedrock", "the inactive podium stands again");
        server.take_end_sounds();
        let mut stages = Vec::new();
        let mut respawned = None;
        for _ in 0..800 {
            server.tick_end(&[]);
            server.take_explosions();
            for report in server.take_fight_reports() {
                match report {
                    FightReport::Respawning { stage, .. } => stages.push(stage),
                    FightReport::Respawned(uuid) => respawned = Some(uuid),
                    FightReport::Aborted => panic!("aborted"),
                }
            }
            if respawned.is_some() {
                break;
            }
            if server.respawn.as_ref().is_some_and(|respawn| respawn.stage == crate::dragon_respawn::Stage::SummoningPillars) {
                let beams: Vec<_> = server.crystals.list.iter().filter(|crystal| crystal.beam_target.is_some()).map(|crystal| crystal.beam_target).collect();
                assert!(beams.len() >= 4, "the four crystals beam at a pillar or the sky");
            }
        }
        assert_eq!(stages, ["preparing_to_summon_pillars", "summoning_pillars", "summoning_dragon"]);
        let uuid = respawned.expect("a new dragon");
        assert_eq!(server.dragons.list.len(), 1);
        assert_eq!(server.dragons.list[0].uuid, uuid);
        assert_eq!(server.fight_dragon, Some(uuid));
        assert!(!server.fight.dragon_killed);
        assert!(server.respawn.is_none());
        assert_eq!(server.crystals.list.len(), 10, "the four were used up and every pillar has its crystal again");
        for spike in &spikes {
            assert_eq!(server.crystals.on_spike(spike.center_x, spike.center_z, spike.radius), 1, "{spike:?}");
            assert_eq!(block_named(&server, (spike.center_x, spike.height, spike.center_z)), "minecraft:bedrock");
        }
        assert!(server.crystals.list.iter().all(|crystal| !crystal.invulnerable && crystal.beam_target.is_none()), "reset at the end");
        assert_eq!(server.blasts.iter().filter(|(_, source)| *source == BlastSource::KeepBlocks).count(), 4);
        let growls = server.take_end_sounds().iter().filter(|sound| sound.event == crate::ender_dragon::GROWL_SOUND && sound.volume == 64.0).count();
        assert_eq!(growls, 9 + 4 + 20);
    }

    #[test]
    fn breaking_a_respawn_crystal_stops_the_respawn_and_lights_the_portal_again() {
        use crate::end_crystal::{EndDamage, EndTarget};
        let Some((mut server, at)) = respawn_world() else { return };
        place_respawn_crystals(&mut server, at, 0..4);
        server.take_fight_reports();
        for _ in 0..150 {
            server.tick_end(&[]);
            server.take_explosions();
        }
        server.take_fight_reports();
        let invulnerable = server.crystals.list.iter().filter(|crystal| crystal.invulnerable).count();
        assert!(invulnerable >= 1, "a rebuilt pillar's crystal is invulnerable while it summons");
        let (x, y, z) = crate::dragon_respawn::crystal_cells(at)[0];
        let respawn_crystal = server.crystals.list.iter().find(|crystal| crystal.block_position() == (x, y, z)).map(|crystal| crystal.id).expect("the respawn crystal");
        server.end_hit(EndTarget::Crystal(respawn_crystal), EndDamage::attack(1.0, 0));
        assert_eq!(server.take_fight_reports(), [FightReport::Aborted]);
        assert!(server.respawn.is_none());
        assert_eq!(block_named(&server, (1, at.1, 0)), "minecraft:end_portal", "the exit portal is lit again");
        assert!(server.crystals.list.iter().all(|crystal| !crystal.invulnerable));
        assert!(server.fight.dragon_killed);
    }

    #[test]
    fn a_respawn_waits_while_the_fight_does_not_tick_and_a_reloaded_one_waits_for_its_crystals() {
        let Some((mut server, at)) = respawn_world() else { return };
        server.link_fight(FightLink { ticking: false, ..server.fight });
        place_respawn_crystals(&mut server, at, 0..4);
        assert_eq!(server.take_fight_reports().len(), 1, "the respawn starts at once");
        for _ in 0..300 {
            server.tick_end(&[]);
        }
        assert!(server.take_fight_reports().is_empty(), "nothing moves with nobody near the arena");
        assert_eq!(server.respawn.as_ref().map(|respawn| respawn.stage), Some(crate::dragon_respawn::Stage::Start));
        let saved = server.respawn.take().expect("the respawn");
        let tags = server.crystals.tags_where(|_| true);
        server.crystals.remove_where(|_| true);
        server.resume_respawn(saved);
        for _ in 0..20 {
            server.tick_end(&[]);
        }
        assert!(server.respawn.is_some(), "a respawn loaded before its crystals is kept");
        assert!(server.take_fight_reports().is_empty());
        for tag in &tags {
            server.crystals.add(tag);
        }
        server.link_fight(FightLink { ticking: true, ..server.fight });
        server.tick_end(&[]);
        let reports = server.take_fight_reports();
        assert!(matches!(reports.as_slice(), [FightReport::Respawning { stage: "preparing_to_summon_pillars", .. }]), "{reports:?}");
    }

    fn orb_experience(server: &ServerSim) -> i32 {
        server.level.entities.iter().filter(|entity| !entity.removed).filter_map(|entity| entity.orb_data()).map(|orb| orb.value * orb.count).sum()
    }

    fn kill_dragon(server: &mut ServerSim, link: FightLink) -> [i32; 4] {
        use crate::end_crystal::{EndDamage, EndTarget};
        let uuid = server.find_or_create_dragon((0, 0, 0));
        server.link_fight(FightLink { dragon: Some(uuid), ..link });
        server.tick_end(&[]);
        let id = server.dragons.list[0].id;
        server.end_hit(EndTarget::DragonPart { id, part: crate::ender_dragon::HEAD }, EndDamage::attack(500.0, 0));
        for _ in 0..600 {
            server.tick_end(&[]);
            if server.dragons.list.is_empty() {
                break;
            }
        }
        assert!(server.dragons.list.is_empty(), "it rose and went");
        uuid
    }

    #[test]
    fn the_first_dragon_drops_12000_experience_and_a_later_one_500() {
        let Some(mut server) = end_world((-5..=4).flat_map(|x| (-5..=4).map(move |z| (x, z)))) else { return };
        let first = kill_dragon(&mut server, FightLink::default());
        assert_eq!(server.take_dragons_killed(), vec![first]);
        assert_eq!(orb_experience(&server), 12000);
        let sounds = server.take_end_sounds();
        assert_eq!(sounds.iter().filter(|sound| sound.event == crate::ender_dragon::DRAGON_DEATH_SOUND).count(), 1);
        let second = kill_dragon(&mut server, FightLink { previously_killed: true, ..FightLink::default() });
        assert_ne!(first, second);
        assert_eq!(orb_experience(&server), 12500);
        let mut rules = crate::game_rules::GameRules::default();
        rules.set("mob_drops", "false").unwrap();
        server.apply_game_rules(&rules);
        kill_dragon(&mut server, FightLink { previously_killed: true, ..FightLink::default() });
        assert_eq!(orb_experience(&server), 12500, "no experience without mob_drops");
    }

    #[test]
    fn spike_crystals_heal_the_fight_dragon_and_breaking_its_crystal_hurts_it() {
        use crate::end_crystal::{EndDamage, EndTarget};
        let Some(mut server) = end_world((-5..=4).flat_map(|x| (-5..=4).map(move |z| (x, z)))) else { return };
        server.find_or_create_dragon((0, 0, 0));
        server.update_crystal_count();
        assert_eq!(server.alive_crystals, 10);
        server.dragons.list[0].health = 100.0;
        let mut beam = None;
        for _ in 0..2000 {
            server.tick_end(&[]);
            if let Some(crystal) = server.dragons.list[0].nearest_crystal {
                beam = Some(crystal);
                break;
            }
        }
        let crystal = beam.expect("the circling dragon finds a spike crystal within 32 blocks");
        let healed = server.dragons.list[0].health;
        for _ in 0..40 {
            server.tick_end(&[]);
        }
        let view = server.dragon_views().remove(0);
        if view.nearest_crystal == Some(crystal) {
            assert!(server.dragons.list[0].health > healed, "it heals from its crystal");
        }
        server.dragons.list[0].nearest_crystal = Some(crystal);
        server.dragons.list[0].damage_cooldown = 0;
        let before = server.dragons.list[0].health;
        server.end_hit(EndTarget::Crystal(crystal), EndDamage::attack(1.0, 0));
        assert_eq!(server.dragons.list[0].health, before - 10.0, "the crystal's loss hurts its head for 10");
        assert_eq!(server.alive_crystals, 9, "the crystals are counted again");
        assert_eq!(server.blasts.len(), 1, "and the crystal explodes");
        let other = server.crystals.list[0].id;
        server.dragons.list[0].nearest_crystal = Some(other);
        server.dragons.list[0].damage_cooldown = 0;
        let before = server.dragons.list[0].health;
        let position = server.crystals.list[0].position;
        server.explosions.push(minecraftoss_entities::creeper::CreeperExplosion { position, radius: 1.0, powered: false });
        server.tick_end(&[]);
        assert!(server.dragons.list[0].health < before - 8.0, "a blast-destroyed crystal hurts it too");
        assert_eq!(server.alive_crystals, 8);
    }

    #[test]
    fn a_strafing_dragon_s_fireball_bursts_into_a_breath_cloud_that_harms_the_player() {
        use crate::ender_dragon::Phase;
        let Some(mut server) = end_world((-5..=4).flat_map(|x| (-5..=4).map(move |z| (x, z)))) else { return };
        let player = end_player([0.5, 120.0, -40.5]);
        server.set_players(&[player.position.to_array()], 2);
        server.find_or_create_dragon((0, 0, 0));
        {
            let dragon = &mut server.dragons.list[0];
            dragon.position = glam::DVec3::new(0.5, 130.0, 0.5);
            dragon.y_rot = 0.0;
            dragon.set_phase(Phase::StrafePlayer);
            dragon.strafe.attack_target = Some(player.id);
            dragon.strafe.target = Some(glam::DVec3::new(0.5, 130.0, -80.0));
        }
        let bodies = [crate::thrown::PlayerBody::of(&player)];
        let mut shot = false;
        let mut sounds = Vec::new();
        for _ in 0..200 {
            server.tick_clouds(&bodies);
            server.tick_end(&[player]);
            sounds.extend(server.take_end_sounds());
            shot |= !server.dragon_fireballs.views().is_empty();
            if !server.clouds.is_empty() {
                break;
            }
        }
        assert!(shot, "a fireball flew");
        assert!(sounds.iter().any(|sound| sound.event == crate::ender_dragon::SHOOT_SOUND && sound.volume == 10.0));
        let burst = sounds.iter().find(|sound| sound.event == crate::dragon_fireball::EXPLODE_SOUND).expect("the burst sound");
        assert!((0.9..=1.0).contains(&burst.pitch) && burst.volume == 1.0);
        assert_eq!(server.dragon_fireballs.take_bursts().len(), 1);
        assert_eq!(server.clouds.len(), 1);
        let cloud = server.clouds[0].clone();
        assert_eq!(cloud.breath, Some(crate::effect_cloud::Breath::Fireball));
        assert_eq!(cloud.position, player.position, "the cloud settles on the player it struck");
        assert_eq!(cloud.radius, 3.0);
        let mut harm = Vec::new();
        for _ in 0..45 {
            server.tick_clouds(&bodies);
            harm.extend(server.take_player_potions());
        }
        assert_eq!(harm.len(), 2, "after 20 ticks, then every 20");
        assert_eq!(harm[0].potion, crate::potions::PlayerPotion::Harm { amount: 6.0, by: None });
        let owner = server.dragons.list[0].id;
        server.dragon_fireballs.shoot(owner, glam::DVec3::new(0.5, 100.0, 0.5), glam::DVec3::X);
        server.dragons.list.clear();
        server.tick_end(&[player]);
        assert!(server.dragon_fireballs.views().is_empty(), "a fireball goes with its dragon");
    }

    #[test]
    fn the_dragon_s_wings_throw_a_player_and_an_enderman_and_hurt_them() {
        let Some(mut server) = end_world((-5..=4).flat_map(|x| (-5..=4).map(move |z| (x, z)))) else { return };
        server.find_or_create_dragon((0, 0, 0));
        {
            let dragon = &mut server.dragons.list[0];
            dragon.position = glam::DVec3::new(0.5, 110.0, 0.5);
            dragon.y_rot = 0.0;
        }
        server.tick_end(&[end_player([60.5, 110.0, 60.5])]);
        let dragon = &server.dragons.list[0];
        let right = dragon.parts[6] + glam::DVec3::new(1.0, -2.0, 0.0);
        let left = dragon.parts[7] + glam::DVec3::new(-1.0, -2.0, 0.0);
        let nbt = minecraftoss_core::snbt::parse_compound("{NoAI:1b}").unwrap();
        server.summon("minecraft:enderman", left.to_array(), Some(&nbt), 0.0).unwrap();
        let player = end_player(right.to_array());
        server.take_players_pushed();
        server.take_end_hits();
        server.tick_end(&[player]);
        let pushed = server.take_players_pushed();
        assert_eq!(pushed.len(), 1);
        assert_eq!(pushed[0].0, player.id);
        assert!(pushed[0].1.x > 0.5 && pushed[0].1.y == f64::from(0.2_f32), "{pushed:?}");
        let hits = server.take_end_hits();
        assert!(hits.iter().any(|hit| hit.player_id == player.id && hit.damage == crate::ender_dragon::WING_DAMAGE));
        let enderman = &server.mobs.endermen()[0];
        assert_eq!(enderman.enderman.health, 35.0, "the wing hurts it for 5");
        assert!(enderman.enderman.body.velocity.x < -0.5, "thrown away from the dragon");
    }

    #[test]
    fn a_dragon_egg_jumps_to_a_free_spot_nearby_when_clicked_or_hit() {
        use minecraftoss_generator::feature::World as _;
        let Some(mut server) = end_world((-2..=1).flat_map(|x| (-2..=1).map(move |z| (x, z)))) else { return };
        let ground = server.level.height_at(minecraftoss_core::chunk::HeightmapKind::MotionBlocking, 6, 6);
        let mut at = (6, ground, 6);
        server.player_edit_block(at, Some(&Block::new(crate::dragon_egg::DRAGON_EGG)), PlayerEdit::Place);
        assert_eq!(block_name(&server, at), crate::dragon_egg::DRAGON_EGG);
        server.take_changes();
        for attack in [true, false] {
            assert!(if attack { server.attack_block(at) } else { server.use_block(at, "north") });
            let jumps = server.take_egg_teleports();
            assert_eq!(jumps.len(), 1);
            let (from, to) = jumps[0];
            assert_eq!(from, at);
            assert!((to.0 - from.0).abs() <= 15 && (to.1 - from.1).abs() <= 7 && (to.2 - from.2).abs() <= 15, "{to:?}");
            assert_eq!(block_name(&server, from), "minecraft:air");
            assert_eq!(block_name(&server, to), crate::dragon_egg::DRAGON_EGG);
            assert_ne!(block_name(&server, (to.0, to.1 - 1, to.2)), "minecraft:air", "it lands on something");
            let changes = server.take_changes();
            assert!(changes.contains(&(from, None)) && changes.iter().any(|(pos, block)| *pos == to && block.is_some()));
            at = to;
        }
        assert!(!server.attack_block((6, ground + 30, 6)), "air is not an egg");
        assert!(server.take_egg_teleports().is_empty());
    }

    #[test]
    fn a_glass_bottle_near_the_dragon_s_breath_fills_and_shrinks_the_cloud() {
        let Some(mut server) = end_world([(0, 0)].into_iter()) else { return };
        let at = glam::DVec3::new(4.5, 70.0, 4.5);
        server.clouds.push(crate::dragon_fireball::breath_cloud(at));
        assert!(server.cloud_views()[0].breath);
        assert!(server.bottle_breath(at + glam::DVec3::new(4.0, 0.0, 0.0), 1.8));
        assert_eq!(server.clouds[0].radius, 2.5);
        assert!(!server.bottle_breath(at + glam::DVec3::new(9.0, 0.0, 0.0), 1.8), "out of reach");
        assert!(!server.bottle_breath(at + glam::DVec3::new(0.0, 4.5, 0.0), 1.8), "too high above");
        assert_eq!(server.clouds[0].radius, 2.5);
    }

    #[test]
    fn the_dragon_breaks_cobblestone_it_flies_through_but_not_end_stone() {
        use crate::ender_dragon::{part_box, BODY, HEAD, NECK};
        let Some(mut server) = end_world((-5..=4).flat_map(|x| (-5..=4).map(move |z| (x, z)))) else { return };
        server.find_or_create_dragon((0, 0, 0));
        server.dragons.list[0].position = glam::DVec3::new(0.5, 110.0, 0.5);
        let away = end_player([60.5, 110.0, 60.5]);
        let fill = |server: &mut ServerSim, name: &str| {
            let state = server.level.registries().blocks.parse_state(name).unwrap();
            for x in -12..=12 {
                for y in 100..=120 {
                    for z in -12..=12 {
                        server.level.set_block(minecraftoss_core::BlockPos::new(x, y, z), state, update::CLIENTS, update::LIMIT);
                    }
                }
            }
        };
        fill(&mut server, "minecraft:cobblestone");
        server.take_changes();
        server.tick_end(&[away]);
        let parts = server.dragons.list[0].parts;
        let inside = |pos: BlockPos| {
            [HEAD, NECK, BODY].iter().any(|&part| {
                let (low, high) = part_box(parts[part], part);
                (low.x.floor() as i32..=high.x.floor() as i32).contains(&pos.0) && (low.y.floor() as i32..=high.y.floor() as i32).contains(&pos.1) && (low.z.floor() as i32..=high.z.floor() as i32).contains(&pos.2)
            })
        };
        let changes = server.take_changes();
        let broken: Vec<BlockPos> = changes.iter().filter(|(_, block)| block.is_none()).map(|(pos, _)| *pos).collect();
        assert!(broken.len() > 50, "{}", broken.len());
        assert!(broken.iter().all(|&pos| inside(pos) && block_name(&server, pos) == "minecraft:air"));
        assert!(!server.dragons.list[0].in_wall, "cobblestone does not hold it");
        fill(&mut server, "minecraft:end_stone");
        server.take_changes();
        server.tick_end(&[away]);
        let near = |&(pos, _): &(BlockPos, Option<Block>)| (-12..=12).contains(&pos.0) && (100..=120).contains(&pos.1) && (-12..=12).contains(&pos.2);
        assert!(!server.take_changes().iter().any(near), "end stone is dragon immune");
        assert!(server.dragons.list[0].in_wall);
    }

    #[test]
    fn a_landing_dragon_perches_on_the_podium_breathes_flame_and_the_flame_goes_out_when_it_takes_off() {
        use crate::end_crystal::{EndDamage, EndTarget};
        use crate::ender_dragon::Phase;
        use minecraftoss_generator::feature::World as _;
        let Some(mut server) = end_world((-5..=4).flat_map(|x| (-5..=4).map(move |z| (x, z)))) else { return };
        server.scan_dragon_fight((0, 0, 0), None);
        let top = server.level.height_at(minecraftoss_core::chunk::HeightmapKind::MotionBlockingNoLeaves, 0, 0);
        let ground = server.level.height_at(minecraftoss_core::chunk::HeightmapKind::MotionBlockingNoLeaves, 6, 0);
        let player = end_player([6.5, f64::from(ground), 0.5]);
        server.set_players(&[player.position.to_array()], 2);
        server.find_or_create_dragon((0, 0, 0));
        {
            let dragon = &mut server.dragons.list[0];
            dragon.position = glam::DVec3::new(0.5, 100.0, 60.5);
            dragon.set_phase(Phase::LandingApproach);
        }
        let bodies = [crate::thrown::PlayerBody::of(&player)];
        let id = server.dragons.list[0].id;
        let flame = crate::effect_cloud::Breath::Flame(id);
        let mut sat = None;
        let mut phases = vec![Phase::LandingApproach];
        for _ in 0..4000 {
            server.tick_clouds(&bodies);
            server.tick_end(&[player]);
            let dragon = &server.dragons.list[0];
            if phases.last() != Some(&dragon.phase) {
                phases.push(dragon.phase);
            }
            if dragon.phase.is_sitting() && sat.is_none() {
                sat = Some(dragon.position);
            }
            if server.clouds.iter().any(|cloud| cloud.breath == Some(flame)) {
                break;
            }
        }
        assert_eq!(phases, [Phase::LandingApproach, Phase::Landing, Phase::SittingScanning, Phase::SittingAttacking, Phase::SittingFlaming]);
        let sat = sat.expect("it perched");
        assert!(sat.distance_squared(glam::DVec3::new(0.5, f64::from(top), 0.5)) < 1.0, "on the podium's pillar at {top}: {sat:?}");
        let cloud = server.clouds.iter().find(|cloud| cloud.breath == Some(flame)).expect("the flame");
        assert_eq!(cloud.radius, crate::ender_dragon::FLAME_RADIUS);
        assert!(cloud.position.distance(sat) < 10.5, "{:?}", cloud.position);
        for _ in 0..2 {
            server.dragons.list[0].damage_cooldown = 0;
            server.end_hit(EndTarget::DragonPart { id, part: crate::ender_dragon::HEAD }, EndDamage::attack(30.0, player.id));
        }
        assert_eq!(server.dragons.list[0].phase, Phase::Takeoff, "past 50 damage it takes off");
        server.tick_end(&[player]);
        assert!(server.clouds.iter().all(|cloud| cloud.breath != Some(flame)), "and its flame goes out");
    }

    fn idle_handle() -> Option<ServerHandle> {
        let paths = DataPaths::discover().ok()?;
        let registries = Arc::new(Registries::load(&paths).ok()?);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries, 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut sim = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in -1..=1 {
            for z in -1..=1 {
                sim.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        Some(ServerHandle::spawn(sim))
    }

    fn idle_input() -> TickInput {
        TickInput {
            clocks: ClockTicks::default(),
            players: vec![TickPlayer { id: 0, feet: [8.5, 120.0, 8.5], alive: true, main_hand: None, pickup: None }],
            difficulty: 2,
            simulation_centres: vec![(0, 0)],
            simulation_distance: 2,
            mob_players: Vec::new(),
            mob_views: Vec::new(),
            mob_vitals: Vec::new(),
            bright_outside: true,
            tracking: ([8.5, 120.0, 8.5], 64.0),
            player_hurts: Vec::new(),
            spawn_mobs: false,
        }
    }

    #[test]
    fn each_tick_gets_its_own_output() {
        let Some(mut handle) = idle_handle() else { return };
        for _ in 0..3 {
            handle.tick(idle_input(), vec![true]);
        }
        assert_eq!(handle.ticks_in_flight(), 3);
        handle.wait_idle();
        let outputs = handle.poll();
        assert_eq!(outputs.len(), 3);
        assert!(outputs.iter().all(|out| out.tick_nanos.len() == 1), "{:?}", outputs.iter().map(|out| out.tick_nanos.len()).collect::<Vec<_>>());
        assert_eq!(handle.ticks_in_flight(), 0);
    }

    #[test]
    fn commands_after_a_tick_do_not_hold_its_output() {
        let Some(mut handle) = idle_handle() else { return };
        handle.tick(idle_input(), vec![true]);
        for i in 0..200 {
            handle.summon_orb([8.5 + f64::from(i % 10), 120.0, 8.5], 1);
        }
        handle.wait_idle();
        let outputs = handle.poll();
        assert_eq!(outputs[0].tick_nanos.len(), 1);
        assert_eq!(outputs[0].handled, 1);
        assert!(outputs[1..].iter().all(|out| out.tick_nanos.is_empty()));
        assert_eq!(outputs.last().map(|out| out.handled), Some(201));
        assert_eq!(handle.ticks_in_flight(), 0);
    }

    fn game_time(handle: &mut ServerHandle) -> i64 {
        handle.run_job(Box::new(|sim: &mut ServerSim| Box::new(sim.level.game_time) as Box<dyn std::any::Any + Send>));
        handle.wait_idle();
        let mut jobs: Vec<_> = handle.poll().into_iter().flat_map(|out| out.jobs).collect();
        *jobs.pop().expect("the job answered").downcast::<i64>().expect("a game time")
    }

    #[test]
    fn a_batch_runs_each_tick() {
        let Some(mut handle) = idle_handle() else { return };
        let before = game_time(&mut handle);
        handle.tick(idle_input(), vec![true; 3]);
        handle.wait_idle();
        let outputs = handle.poll();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].tick_nanos.len(), 3);
        assert!(outputs[0].entities.is_some() && outputs[0].mobs.is_some());
        assert_eq!(handle.ticks_in_flight(), 0);
        assert_eq!(game_time(&mut handle), before + 3);
    }

    #[test]
    fn a_rule_sent_after_a_batch_applies_to_the_next_batch() {
        let Some(mut handle) = idle_handle() else { return };
        handle.tick(idle_input(), vec![true; 5]);
        let mut rules = crate::game_rules::GameRules::default();
        rules.set("random_tick_speed", "0").unwrap();
        handle.set_game_rules(rules);
        handle.run_job(Box::new(|sim: &mut ServerSim| Box::new(sim.level.random_tick_speed) as Box<dyn std::any::Any + Send>));
        handle.tick(idle_input(), vec![true]);
        handle.wait_idle();
        let outputs = handle.poll();
        assert_eq!(outputs.iter().map(|out| out.tick_nanos.len()).sum::<usize>(), 6);
        let speeds: Vec<i32> = outputs.into_iter().flat_map(|out| out.jobs).map(|job| *job.downcast::<i32>().unwrap()).collect();
        assert_eq!(speeds, [0]);
        assert_eq!(handle.ticks_in_flight(), 0);
    }

    fn frozen_sim() -> Option<(ServerSim, HandcraftedScene)> {
        let paths = DataPaths::discover().ok()?;
        let registries = Arc::new(Registries::load(&paths).ok()?);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries, 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut sim = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                sim.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        Some((sim, scene))
    }

    fn run_ticks(sim: &mut ServerSim, input: &mut TickInput, normal: bool, count: usize) -> Output {
        let mut out = Output::default();
        for _ in 0..count {
            run_tick(sim, input, normal, &mut out);
        }
        out
    }

    #[test]
    fn a_frozen_tick_holds_items_and_falling_blocks() {
        let Some((mut sim, mut scene)) = frozen_sim() else { return };
        let item = sim.spawn_item("minecraft:cobblestone", 1, None, [11.5, 210.0, 11.5], [0.0; 3], 40, 0);
        scene.set((8, 200, 8), Some(Block::new("minecraft:stone")));
        sim.player_edit(&scene, (8, 200, 8), PlayerEdit::Place);
        scene.set((8, 205, 8), Some(Block::new("minecraft:sand")));
        sim.player_edit(&scene, (8, 205, 8), PlayerEdit::Place);
        let mut input = idle_input();
        run_ticks(&mut sim, &mut input, true, 3);
        let item_at = |sim: &ServerSim| sim.items().into_iter().find(|i| i.id == item).map(|i| i.position);
        let sand_at = |sim: &ServerSim| sim.falling_blocks().first().map(|f| f.position);
        let (held_item, held_sand) = (item_at(&sim).expect("the item"), sand_at(&sim).expect("the sand falls"));
        run_ticks(&mut sim, &mut input, false, 20);
        assert_eq!((item_at(&sim), sand_at(&sim)), (Some(held_item), Some(held_sand)));
        run_ticks(&mut sim, &mut input, true, 20);
        assert!(item_at(&sim).expect("the item")[1] < held_item[1] - 1.0);
        assert!(sand_at(&sim).is_none_or(|at| at[1] < held_sand[1] - 1.0));
    }

    #[test]
    fn a_frozen_tick_still_picks_up() {
        let Some((mut sim, _)) = frozen_sim() else { return };
        let feet = [11.5, 210.0, 11.5];
        let item = sim.spawn_item("minecraft:cobblestone", 3, None, feet, [0.0; 3], 0, 0);
        let mut input = TickInput { players: vec![TickPlayer { id: 0, feet, alive: true, main_hand: None, pickup: Some((Box::default(), 0)) }], ..idle_input() };
        let out = run_ticks(&mut sim, &mut input, false, 1);
        assert_eq!(out.picked.iter().map(|(player, picked)| (*player, picked.0, picked.3)).collect::<Vec<_>>(), [(0, item, 3)]);
        assert!(sim.items().iter().all(|i| i.id != item));
    }

    #[test]
    fn a_frozen_gateway_neither_ages_nor_cools_and_the_snapshot_shows_it() {
        let Some((mut sim, _)) = frozen_sim() else { return };
        let pos = (8, 205, 8);
        sim.player_edit_block(pos, Some(&Block::new(crate::end_gateway::END_GATEWAY)), PlayerEdit::Place);
        let mut input = idle_input();
        run_ticks(&mut sim, &mut input, true, 3);
        assert!(sim.trigger_gateway(pos));
        run_ticks(&mut sim, &mut input, false, 10);
        let view = |sim: &ServerSim| sim.gateway_views().first().map(|view| (view.age, view.cooldown));
        assert_eq!(view(&sim), Some((3, crate::end_gateway::COOLDOWN_TIME)));
        let mut out = run_ticks(&mut sim, &mut input, true, 1);
        assert_eq!(view(&sim), Some((4, crate::end_gateway::COOLDOWN_TIME - 1)));
        finish_batch(&mut sim, &input, &mut out);
        let shown = out.entities.expect("a snapshot").gateways;
        assert_eq!(shown.iter().map(|view| (view.pos, view.age, view.cooldown)).collect::<Vec<_>>(), [(pos, 4, crate::end_gateway::COOLDOWN_TIME - 1)]);
    }

    #[test]
    fn primed_tnt_waits_while_frozen() {
        let Some((mut sim, mut scene)) = frozen_sim() else { return };
        scene.set((8, 200, 8), Some(Block::new("minecraft:stone")));
        sim.player_edit(&scene, (8, 200, 8), PlayerEdit::Place);
        scene.set((8, 201, 8), Some(Block::new("minecraft:tnt")));
        sim.player_edit(&scene, (8, 201, 8), PlayerEdit::Place);
        scene.set((9, 201, 8), Some(Block::new("minecraft:redstone_block")));
        sim.player_edit(&scene, (9, 201, 8), PlayerEdit::Place);
        let fuse = |sim: &ServerSim| sim.primed_tnt().first().map(|tnt| tnt.fuse);
        let primed = fuse(&sim).expect("the TNT primes");
        let mut input = idle_input();
        run_ticks(&mut sim, &mut input, false, 10);
        assert_eq!(fuse(&sim), Some(primed));
        run_ticks(&mut sim, &mut input, true, 10);
        assert_eq!(fuse(&sim), Some(primed - 10));
    }

    #[test]
    fn random_ticks_skip_frozen_ticks() {
        let Some((mut sim, _)) = frozen_sim() else { return };
        let mut input = idle_input();
        let before = sim.level.game_time;
        let out = run_ticks(&mut sim, &mut input, false, 5);
        assert_eq!(out.tick_nanos.len(), 5);
        assert_eq!(sim.level.game_time, before);
        run_ticks(&mut sim, &mut input, true, 5);
        assert_eq!(sim.level.game_time, before + 5);
    }

    fn standing(id: u64, feet: [f64; 3], picks_up: bool) -> TickPlayer {
        TickPlayer { id, feet, alive: true, main_hand: None, pickup: picks_up.then(|| (Box::default(), 0)) }
    }

    fn candidate(id: u64, feet: [f64; 3]) -> minecraftoss_entities::tempt::PlayerCandidate {
        minecraftoss_entities::tempt::PlayerCandidate {
            id,
            position: glam::DVec3::from_array(feet),
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
        }
    }

    fn two_players(feet: [[f64; 3]; 2]) -> TickInput {
        TickInput {
            players: vec![standing(0, feet[0], true), standing(1, feet[1], true)],
            mob_players: vec![candidate(0, feet[0]), candidate(1, feet[1])],
            ..idle_input()
        }
    }

    fn held(player: &TickPlayer) -> Vec<(String, u8)> {
        player.pickup.as_ref().map_or_else(Vec::new, |(inventory, _)| inventory.slots.iter().flatten().map(|stack| (stack.id.clone(), stack.count)).collect())
    }

    #[test]
    fn two_players_pick_up_their_own_items_and_orbs() {
        let Some((mut sim, _)) = frozen_sim() else { return };
        let feet = [[3.5, 210.0, 3.5], [12.5, 210.0, 12.5]];
        let cobblestone = sim.spawn_item("minecraft:cobblestone", 3, None, feet[0], [0.0; 3], 0, 0);
        let dirt = sim.spawn_item("minecraft:dirt", 5, None, feet[1], [0.0; 3], 0, 0);
        sim.summon_orb(feet[0], 3);
        sim.summon_orb(feet[1], 7);
        let mut input = two_players(feet);
        let out = run_ticks(&mut sim, &mut input, false, 1);
        let picked: Vec<(u64, i32, String)> = out.picked.iter().map(|(player, picked)| (*player, picked.0, picked.2.clone())).collect();
        assert_eq!(picked, [(0, cobblestone, "minecraft:cobblestone".to_owned()), (1, dirt, "minecraft:dirt".to_owned())]);
        let orbs: Vec<(u64, i32)> = out.orbs_taken.iter().map(|orb| (orb.0, orb.3)).collect();
        assert_eq!(orbs, [(0, 3), (1, 7)]);
        assert_eq!(held(&input.players[0]), [("minecraft:cobblestone".to_owned(), 3)]);
        assert_eq!(held(&input.players[1]), [("minecraft:dirt".to_owned(), 5)]);
    }

    #[test]
    fn each_player_waits_out_their_own_xp_delay() {
        let Some((mut sim, _)) = frozen_sim() else { return };
        let feet = [[3.5, 210.0, 3.5], [12.5, 210.0, 12.5]];
        for _ in 0..2 {
            sim.summon_orb(feet[0], 1);
            sim.summon_orb(feet[1], 1);
        }
        let mut input = two_players(feet);
        let mut taken = Vec::new();
        for tick in 0..4 {
            let out = run_ticks(&mut sim, &mut input, false, 1);
            taken.extend(out.orbs_taken.iter().map(|orb| (tick, orb.0)));
        }
        assert_eq!(taken, [(0, 0), (0, 1), (2, 0), (2, 1)]);
    }

    fn floor(sim: &mut ServerSim) {
        for x in 0..16 {
            for z in 0..16 {
                sim.player_edit_block((x, 200, z), Some(&Block::new("minecraft:stone")), PlayerEdit::Place);
            }
        }
        sim.take_changes();
    }

    fn night(input: TickInput) -> TickInput {
        TickInput { clocks: ClockTicks { overworld: 18_000, the_end: 0 }, bright_outside: false, ..input }
    }

    #[test]
    fn a_zombie_targets_the_nearer_player() {
        let Some((mut sim, _)) = frozen_sim() else { return };
        floor(&mut sim);
        sim.set_players(&[], 2);
        let nbt = minecraftoss_core::snbt::parse_compound("{IsBaby:0b}").unwrap();
        sim.summon("minecraft:zombie", [8.5, 201.0, 8.5], Some(&nbt), 0.0).unwrap();
        let mut input = night(two_players([[8.5, 201.0, 15.5], [11.5, 201.0, 8.5]]));
        let mut target = None;
        for _ in 0..100 {
            run_tick(&mut sim, &mut input, true, &mut Output::default());
            target = sim.mobs.zombies().first().and_then(|zombie| zombie.target_player_id);
            if target.is_some() {
                break;
            }
        }
        assert_eq!(target, Some(1), "player 1 stands 3 blocks away, player 0 seven");
    }

    #[test]
    fn a_hurt_on_player_one_sends_only_player_ones_wolf_after_the_mob() {
        let Some((mut sim, _)) = frozen_sim() else { return };
        floor(&mut sim);
        sim.set_players(&[], 2);
        let owners = [0x1111_u128, 0x2222_u128];
        sim.set_player_uuid(0, owners[0]);
        sim.set_player_uuid(1, owners[1]);
        let still = minecraftoss_core::snbt::parse_compound("{NoAI:1b,IsBaby:0b}").unwrap();
        sim.summon("minecraft:zombie", [8.5, 201.0, 2.5], Some(&still), 0.0).unwrap();
        let zombie = sim.mobs.zombies()[0].id;
        let mut wolves = Vec::new();
        for (owner, at) in owners.into_iter().zip([[4.5, 201.0, 8.5], [12.5, 201.0, 8.5]]) {
            sim.summon("minecraft:wolf", at, None, 0.0).unwrap();
            let id = sim.mobs.wolves().last().unwrap().id;
            let wolf = &mut sim.mobs.wolf_mut(id).unwrap().wolf;
            wolf.owner = Some(owner);
            wolf.set_tame(true, true);
            wolves.push(id);
        }
        let mut input = night(two_players([[3.5, 201.0, 8.5], [13.5, 201.0, 8.5]]));
        input.player_hurts = vec![(1, Some(zombie), "minecraft:mob_attack")];
        let target = |sim: &ServerSim, id: u64| sim.mobs.wolves().iter().find(|wolf| wolf.id == id).and_then(|wolf| wolf.ai.state.target);
        for _ in 0..40 {
            run_tick(&mut sim, &mut input, true, &mut Output::default());
            if target(&sim, wolves[1]).is_some() {
                break;
            }
        }
        assert_eq!(target(&sim, wolves[1]), Some(minecraftoss_entities::monster_ai::Target::Mob(zombie)), "player 1's wolf avenges them");
        assert_eq!(target(&sim, wolves[0]), None, "player 0 was not hurt");
    }

    #[test]
    fn random_ticks_run_around_every_player() {
        let Ok(paths) = DataPaths::discover() else { return };
        let Ok(registries) = Registries::load(&paths) else { return };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries, 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut sim = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in [-1, 0, 1, 9, 10, 11] {
            for z in -1..=1 {
                sim.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let mut input = TickInput { simulation_centres: vec![(10, 0), (0, 0)], simulation_distance: 1, ..idle_input() };
        run_ticks(&mut sim, &mut input, true, 1);
        let ticking: Vec<(i32, i32)> = sim.level.ticking_chunks.clone().unwrap().iter().map(|pos| (pos.x, pos.z)).collect();
        let both: Vec<(i32, i32)> = [-1, 0, 1, 9, 10, 11].into_iter().flat_map(|x| (-1..=1).map(move |z| (x, z))).collect();
        assert_eq!(ticking, both, "both squares, in X then Z order");
        sim.set_simulation_areas(&[(0, 0), (1, 0)], 1);
        assert_eq!(sim.level.ticking_chunks.as_ref().map(Vec::len), Some(9), "an overlap ticks once, and only loaded chunks tick");
    }

    #[test]
    fn a_player_who_leaves_keeps_the_picks_made_and_picks_nothing_more() {
        let Some((mut sim, _)) = frozen_sim() else { return };
        let feet = [[3.5, 210.0, 3.5], [12.5, 210.0, 12.5]];
        let first = sim.spawn_item("minecraft:dirt", 2, None, feet[1], [0.0; 3], 0, 0);
        sim.summon_orb(feet[1], 4);
        let mut input = two_players(feet);
        input.players[1].main_hand = Some("minecraft:emerald".to_owned());
        let out = run_ticks(&mut sim, &mut input, false, 1);
        assert_eq!(out.picked.iter().map(|(player, picked)| (*player, picked.0)).collect::<Vec<_>>(), [(1, first)]);
        assert_eq!(out.orbs_taken.iter().map(|orb| (orb.0, orb.3)).collect::<Vec<_>>(), [(1, 4)]);
        player_left(&mut sim, Some(&mut input), 1);
        assert_eq!(input.players.iter().map(|p| p.id).collect::<Vec<_>>(), [0]);
        assert_eq!(input.mob_players.iter().map(|p| p.id).collect::<Vec<_>>(), [0]);
        assert!(!sim.take_xp_delay.contains_key(&1) && !sim.player_feet.contains_key(&1));
        sim.spawn_item("minecraft:dirt", 2, None, feet[1], [0.0; 3], 0, 0);
        sim.summon_orb(feet[1], 4);
        let out = run_ticks(&mut sim, &mut input, false, 3);
        assert!(out.picked.is_empty() && out.orbs_taken.is_empty(), "nobody stands there any more");

        let Some(mut handle) = idle_handle() else { return };
        handle.tick(two_players(feet), vec![true; 3]);
        handle.player_left(1);
        handle.tick(TickInput { players: vec![standing(0, feet[0], true)], ..idle_input() }, vec![true]);
        handle.wait_idle();
        let outputs = handle.poll();
        assert_eq!(outputs.iter().map(|out| out.tick_nanos.len()).sum::<usize>(), 4);
        assert_eq!(handle.ticks_in_flight(), 0);
    }
}
