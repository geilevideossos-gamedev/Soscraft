//! The Minecraft world's mobs, as MinecraftOSS runs them: its integrated
//! server (`engine/viewer/src/server.rs`) ticks the level at 20 Hz on its own
//! thread with natural spawning and the entity world, every passive and
//! hostile mob's AI and pathfinding. Here it is fed the streamed chunks and
//! the player, and hands back the tracked mobs, which are drawn with the
//! viewer's mob renderers. MW2 bullets hurt the mobs with their real damage
//! (a hundred MW2 health to Minecraft's twenty), and mobs hurt the player
//! the other way round.
use glam::DVec3;
use minecraft_terrain::level_storage::WeatherData;
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::mesh::{Atlas, ChunkMesh};
use minecraft_terrain::scene::{Block, HandcraftedScene};
use minecraft_terrain::server::{HowBroken, PlayerEdit, ServerHandle, ServerSim, TickInput, TickPlayer};
use minecraft_terrain::terrain::{BlockStates, TerrainStream};
use minecraft_terrain::client_mobs::server_mobs;
use minecraft_terrain::mob_tracking::{Tracked, Watcher};
use minecraft_terrain::server::{EntitySnapshot, ServerOrb};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::items::ItemEntity;
use minecraftoss_player::loot::LootBook;
use minecraftoss_player::rng::XoroshiroRandom;
use minecraftoss_player::survival::EffectKind;
use std::collections::HashMap;
use minecraft_terrain::pack::PackStack;
use minecraftoss_entities::tempt::PlayerCandidate;
use minecraftoss_entities::world::{EntityWorld, MobHit, PlayerHitKind};
use minecraft_terrain::end_crystal::{EndDamage, EndTarget};
use minecraft_terrain::item_frame::FrameAction;

pub(crate) const TICK_SECONDS: f64 = 1.0 / 20.0;
/// Minecraft health per MW2 health.
const HEALTH_SCALE: f32 = 20.0 / 100.0;
const BLAST_HEALTH_SCALE: f32 = 80.0 / 100.0;
/// `Player.getEyeHeight` standing.
const EYE_HEIGHT: f32 = 1.62;
const SERVER_ITEM: u32 = 1 << 31;
pub const HOST_VIEW_DISTANCE: i32 = 8;
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
    pub view: crate::entity_view::EntityView,
    server: ServerHandle,
    level_events: Vec<(i32, (i32, i32, i32), i32)>,
    pub broken_items: Vec<String>,
    /// Client drops handed to the server: the command count that sent each.
    server_handed: HashMap<u32, (u64, ItemEntity)>,
    players: PlayerQueues,
    block_entity_changes: Vec<((i32, i32, i32), Option<minecraftoss_core::nbt::Tag>)>,
    server_snapshot: Option<EntitySnapshot>,
    server_handled: u64,
    loot: Option<std::sync::Arc<LootBook>>,
    loot_sequences: HashMap<String, XoroshiroRandom>,
    seed: i64,
    dragons_killed: Vec<[i32; 4]>,
    fight_reports: Vec<minecraft_terrain::server::FightReport>,
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
    pvp: bool,
    presenter: Option<u64>,
    watchers: Vec<Watcher>,
    view_distance: i32,
    tracked: Tracked,
    friend_snapshot: Option<EntitySnapshot>,
    pub heard: crate::heard::HeardLog,
    pub spawner_tags: Vec<((i32, i32, i32), minecraftoss_core::nbt::Tag)>,
    mount_viewers: HashMap<u64, u64>,
    player_boxes: Vec<crate::pvp::PlayerBox>,
    held_names: HashMap<u64, String>,
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
    pub glow: ChunkMesh,
    pub glow_cover: ChunkMesh,
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
    pub hero: Option<i32>,
    pub sleeping: Option<(i32, i32, i32)>,
    pub sleep_timer: i32,
    pub forward: f32,
    pub strafe: f32,
    pub jumping: bool,
    pub swirl: Option<minecraft_terrain::spell_particles::SwirlView>,
    pub omens: minecraft_terrain::server::PlayerOmens,
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
        sleeping: player.sleeping,
        sleep_timer: player.sleep_timer,
        forward: player.forward,
        strafe: player.strafe,
        jumping: player.jumping,
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
    pub raid_omens: Vec<(i32, (i32, i32, i32))>,
    pub raid_heroes: Vec<i32>,
    pub raid_horns: Vec<DVec3>,
    pub raid_bars: Option<Vec<minecraft_terrain::raids::RaidBar>>,
    pub told: Vec<minecraft_commands::Clientbound>,
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
    mount_menus: Vec<u64>,
    dismounts: Vec<(u64, Option<DVec3>)>,
    boost_waiting: Option<(usize, String)>,
    boosts_done: Vec<(usize, String)>,
    lecterns: Vec<minecraft_terrain::lectern::LecternView>,
    merchant_routed: Vec<MerchantNews>,
    merchant: Vec<MerchantNews>,
    merchant_waiting: usize,
    raid_omens_waiting: Vec<(i32, (i32, i32, i32))>,
    pub(crate) out: PlayerOutputs,
}

#[derive(Clone, Debug)]
pub enum MerchantNews {
    Opened(minecraft_terrain::server::MerchantView),
    Answer(minecraft_terrain::server::MerchantUpdate),
    Closing,
}

fn apply_merchant_answer(inventory: &mut Inventory, update: &minecraft_terrain::server::MerchantUpdate) {
    for (slot, stack) in &update.slots {
        if let Some(held) = inventory.slots.get_mut(*slot) {
            *held = stack.clone();
        }
    }
    inventory.cursor = update.cursor.clone();
}

fn settle_eggs(queue: &mut PlayerQueue, inventory: &mut Inventory) {
    for summoned in std::mem::take(&mut queue.egg_answers) {
        let egg = queue.eggs_waiting.answer(inventory, summoned.is_ok());
        if let (Some(egg), Err(error)) = (egg, summoned) {
            queue.out.told.extend(egg_refusal(&error));
            diag::warn!(World, "Minecraft {egg} hatched nothing: {error}");
        }
    }
}

fn egg_refusal(error: &str) -> Option<minecraft_commands::Clientbound> {
    (error == minecraft_terrain::spawn_egg::SPAWNER_DISABLED).then(|| minecraft_commands::Clientbound::System {
        text: minecraft_commands::Text::translate(error, Vec::new()),
        overlay: false,
    })
}

fn settle_merchant_news(queue: &mut PlayerQueue, inventory: &mut Inventory) {
    for news in std::mem::take(&mut queue.merchant_routed) {
        if let MerchantNews::Answer(update) = &news {
            apply_merchant_answer(inventory, update);
            queue.merchant_waiting = queue.merchant_waiting.saturating_sub(1);
        }
        queue.merchant.push(news);
    }
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
            queue.out.pushes.extend(explosion_push(&hit.kind));
            let Some(feet) = queue.feet else {
                continue;
            };
            if let Some(hit) = mob_hit(&hit, feet, difficulty, hit.source.and_then(&entity_type)) {
                queue.out.hits.push(hit);
            }
        }
        let (closings, answers): (Vec<_>, Vec<_>) = std::mem::take(&mut output.merchant).into_iter().partition(|(_, update)| update.closing);
        for (player, update) in answers {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.merchant_routed.push(MerchantNews::Answer(update));
            }
        }
        for (player, result) in &output.mob_results {
            if let Some(queue) = self.0.get_mut(player) {
                queue.mob_slots.push(result.slots.clone());
                queue.broken.extend(result.broken.iter().cloned());
                queue.mounts.extend(result.mounted);
                queue.mount_menus.extend(result.mount_inventory);
                queue.merchant_routed.extend(result.merchant.clone().map(MerchantNews::Opened));
            }
        }
        for (player, _) in closings {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.merchant_routed.push(MerchantNews::Closing);
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
        for (player, amplifier, at) in std::mem::take(&mut output.raid_omens) {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.out.raid_omens.push((amplifier, at));
            }
        }
        for (player, amplifier) in std::mem::take(&mut output.raid_heroes) {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.out.raid_heroes.push(amplifier);
            }
        }
        for (player, at) in std::mem::take(&mut output.raid_horns) {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.out.raid_horns.push(DVec3::from_array(at));
            }
        }
        for (player, bars) in output.raid_bars.take().into_iter().flatten() {
            if let Some(queue) = self.0.get_mut(&player) {
                queue.out.raid_bars = Some(bars);
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
            view: crate::entity_view::EntityView::new(seed),
            level_events: Vec::new(),
            broken_items: Vec::new(),
            server_handed: HashMap::new(),
            players: PlayerQueues::default(),
            block_entity_changes: Vec::new(),
            server_snapshot: None,
            server_handled: 0,
            loot: data.loot.clone(),
            loot_sequences: HashMap::new(),
            seed,
            dragons_killed: Vec::new(),
            fight_reports: Vec::new(),
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
            pvp: true,
            presenter: None,
            watchers: Vec::new(),
            view_distance: HOST_VIEW_DISTANCE,
            tracked: Tracked::default(),
            friend_snapshot: None,
            heard: Default::default(),
            spawner_tags: Vec::new(),
            mount_viewers: HashMap::new(),
            player_boxes: Vec::new(),
            held_names: HashMap::new(),
        }
    }

    pub fn set_game_rules(&mut self, rules: &minecraft_terrain::game_rules::GameRules) {
        self.spawn_mobs = rules.bool("spawn_mobs");
        self.block_drops = rules.bool("block_drops");
        self.pvp = rules.bool("pvp");
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
        let asleep = villager_asleep_in(&self.view.world, head);
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

    pub fn set_watchers(&mut self, watchers: Vec<Watcher>, view_distance: i32) {
        self.watchers = watchers;
        self.view_distance = view_distance;
    }

    pub fn take_tracked(&mut self) -> Tracked {
        std::mem::take(&mut self.tracked)
    }

    pub fn take_friend_snapshot(&mut self) -> Option<EntitySnapshot> {
        self.friend_snapshot.take()
    }

    pub fn hear(&mut self, said: crate::heard::Said) {
        self.heard.push(said);
    }

    pub fn sound(&mut self, event: &str, at: DVec3, volume: f32, pitch: f32) {
        self.view.sounds.push((event.to_owned(), at, volume, pitch));
        self.heard.push(crate::heard::Said::all(crate::heard::Heard::sound(event, at.to_array(), volume, pitch)));
    }

    pub fn lid_event(&mut self, scene: &HandcraftedScene, pos: (i32, i32, i32), id: u8, count: u32) {
        self.view.lids.block_event(scene, pos, id, count);
        let param = i32::try_from(count).unwrap_or(i32::MAX);
        self.heard.push(crate::heard::Said::all(crate::heard::Heard::Block { pos: [pos.0, pos.1, pos.2], kind: id, param }));
    }

    pub fn set_player_boxes(&mut self, boxes: Vec<crate::pvp::PlayerBox>) {
        self.player_boxes = boxes;
    }

    pub fn player_box(&self, id: u64) -> Option<[f64; 6]> {
        self.player_boxes.iter().find(|player| player.id == id).map(|player| player.bounds)
    }

    pub fn set_held_names(&mut self, names: HashMap<u64, String>) {
        self.held_names = names;
    }

    pub fn held_name(&self, id: u64) -> Option<String> {
        self.held_names.get(&id).cloned()
    }

    #[allow(clippy::too_many_arguments)]
    fn pvp_attack(&self, player: u64, held: Option<&ItemStack>, eye: DVec3, yaw: f32, attack_damage: f64, strength: f32, blow: &Blow) -> crate::pvp::Attack {
        crate::pvp::Attack {
            attacker: player,
            feet: eye - DVec3::Y * f64::from(EYE_HEIGHT),
            yaw,
            attack_damage,
            strength,
            sprinting: blow.sprinting,
            can_critical: blow.can_critical,
            can_sweep: blow.can_sweep,
            bonus: minecraft_terrain::melee::weapon_bonus(held),
            disable_shield_ticks: crate::pvp::shield_disable_ticks(&self.recipes, held),
            pvp: self.pvp,
        }
    }

    fn strike_players(&mut self, attacker: u64, blows: Vec<crate::pvp::Blow>) {
        for blow in blows {
            self.players.queue(blow.victim).out.hits.push(crate::pvp::hit(&blow, attacker));
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn punch_player(&mut self, player: u64, inventory: &Inventory, selected: usize, eye: DVec3, look: DVec3, yaw: f32, attack_damage: f64, strength: f32, blow: &Blow) -> Option<bool> {
        let (victim, distance) = crate::pvp::player_on_ray(&self.player_boxes, player, eye, look, blow.reach)?;
        let frame = minecraft_terrain::item_frame::on_ray(&self.view.frames, eye, look, blow.reach).map(|(_, distance)| distance);
        let nearer = self.entity_distance(player, eye, look, blow.reach).into_iter().chain(frame).chain(blow.block).any(|other| other <= distance);
        if nearer {
            return None;
        }
        let victim = *self.player_boxes.iter().find(|box_| box_.id == victim)?;
        let held = inventory.slots.get(selected).and_then(Option::as_ref);
        let attack = self.pvp_attack(player, held, eye, yaw, attack_damage, strength, blow);
        let hit = crate::pvp::melee(&attack, &victim);
        for sound in &hit.sounds {
            self.sound(sound, attack.feet, 1.0, 1.0);
        }
        let mut blows: Vec<_> = hit.blow.into_iter().collect();
        if hit.hurt && hit.sweep {
            blows.extend(crate::pvp::swept(&attack, victim.bounds, Some(victim.id), &self.player_boxes));
            let swing = minecraftoss_entities::world::PlayerAttack { player_id: player, position: attack.feet, yaw, attack_damage, strength, sprinting: blow.sprinting, can_critical: blow.can_critical, can_sweep: blow.can_sweep };
            self.server.mob_sweep(player, swing, victim.bounds, attack.bonus.damage);
        }
        self.strike_players(player, blows);
        Some(hit.hurt)
    }

    pub fn mob_box(&self, id: u64) -> Option<[f64; 6]> {
        self.view.boxes().into_iter().find(|(key, _)| key & ((1 << 56) - 1) == id).map(|(_, bounds)| bounds)
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
        let Some(thrown) = minecraft_terrain::thrown::Thrown::launch(&throw, &mut self.view.random) else {
            return false;
        };
        self.server.throw(thrown);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub fn release_trident(&mut self, player: u64, inventory: &mut Inventory, charging: crate::trident::Charging, creative: bool, wet: bool, eye: DVec3, yaw: f32, pitch: f32, motion: DVec3, on_ground: bool) -> minecraft_terrain::trident::Release {
        use minecraft_terrain::trident::{Release, ThrownTrident};
        let release = minecraft_terrain::trident::release(inventory, charging.slot, charging.ticks, creative, wet, &mut self.view.random);
        if let Release::Throw(stack) = &release {
            let throw = minecraft_terrain::thrown::Throw { owner: player, item: &stack.id, components: stack.components.as_ref(), eye, yaw, pitch, motion, on_ground };
            let trident = ThrownTrident::launch(&throw, stack.clone(), creative, &mut self.view.random);
            self.server.throw_trident(trident);
        }
        release
    }

    pub fn random_float(&mut self) -> f32 {
        self.view.random.next_float()
    }

    pub fn ticks(&self) -> u64 {
        self.view.ticks
    }

    pub fn load_chunk(&mut self, chunk: &std::sync::Arc<minecraftoss_core::Chunk>) {
        self.view.spawners.load_chunk(chunk);
        self.server.load_chunk(chunk);
    }

    pub fn unload_chunk(&mut self, pos: minecraftoss_core::ChunkPos) {
        self.view.spawners.unload_chunk(pos);
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
            let experience = minecraft_terrain::experience::block_experience(block, true, |bound| self.view.random.next_int(bound));
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
                    && self.view.random.next_float() >= 1.0 / power
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
                self.view.world_items.spawn_block_drop(drop, *pos);
            }
            if let Some(components) = kept.remove(pos) {
                let mut boxed = ItemStack::new(block.id.clone(), 1);
                boxed.max = boxed.max.min(self.recipes.max_stack(&boxed.id));
                boxed.components = Some(components);
                self.view.world_items.spawn_block_drop(boxed, *pos);
            }
        }
    }

    /// `ItemEntity`s to draw and pick up, as the viewer's
    /// `server_items_tick` does for a server-simulated world: client drops
    /// go to the server, stacks the server offered go into the inventory,
    /// and the server's items are mirrored for drawing.
    fn server_items_tick(&mut self, actors: &mut [TickActor<'_>]) {
        let entities = std::mem::take(&mut self.view.world_items.entities);
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
        self.view.world_items.tick_pickup_effects(target);
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
                self.view.world_items.note_pickup(snapshot, target, transfer);
            }
        }
        let _ = self.view.world_items.take_pickup_transfers();
        let Some(snapshot) = self.server_snapshot.take() else {
            self.view.world_items.entities = previous.into_values().collect();
            self.view.world_items.entities.sort_by_key(|e| e.entity_id);
            return;
        };
        let recipes = actors.first().map_or_else(|| self.recipes.clone(), |actor| actor.inventory.recipes.clone());
        self.view.show_items(snapshot.items, &recipes);
        let handled = self.server_handled;
        self.server_handed.retain(|_, (sent, _)| *sent > handled);
        for (_, entity) in self.server_handed.values() {
            self.view.world_items.entities.push(entity.clone());
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
        self.view.held_item_mesh(posed, light_at, packs, atlas, light)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn punch(&mut self, player: u64, inventory: &Inventory, selected: usize, eye: DVec3, look: DVec3, yaw: f32, attack_damage: f64, strength: f32, blow: Blow) -> bool {
        if let Some(frame) = frame_target(&self.view.frames, eye, look, blow.reach, blow.block, self.entity_distance(player, eye, look, blow.reach)) {
            self.server.frame_action(player, frame, FrameAction::Attack { infinite: blow.creative });
            return true;
        }
        let mob = mob_on_ray(&self.view.world, player, eye, look, blow.reach);
        if let Some((target, distance)) = self.end_on_ray(eye, look, blow.reach)
            && mob.is_none_or(|(_, nearest)| distance < nearest)
        {
            let damage = attack_damage as f32 * (0.2 + strength * strength * 0.8);
            self.server.end_hit(target, EndDamage::attack(damage, player).from_position(eye));
            return true;
        }
        if let Some((bullet, distance)) = self.view.world.shulker_bullet_on_ray(eye, look, blow.reach)
            && mob.is_none_or(|(_, nearest)| distance < nearest)
        {
            self.server.hurt_shulker_bullet(bullet);
            return true;
        }
        if let Some((fireball, distance)) = self.view.world.fireball_on_ray(eye, look, blow.reach)
            && mob.is_none_or(|(_, nearest)| distance < nearest)
        {
            self.server.deflect_fireball(fireball, look.normalize_or_zero().to_array(), player, true);
            let feet = eye - DVec3::Y * f64::from(EYE_HEIGHT);
            self.sound("minecraft:entity.player.attack.nodamage", feet, 1.0, 1.0);
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
        self.server.mob_action(player, hit, Some(attack), inventory, selected, blow.creative);
        let held = inventory.slots.get(selected).and_then(Option::as_ref);
        let pvp = self.pvp_attack(player, held, eye, yaw, attack_damage, strength, &blow);
        if let Some(bounds) = self.mob_box(hit.id()) {
            let blows = crate::pvp::swept(&pvp, bounds, None, &self.player_boxes);
            self.strike_players(player, blows);
        }
        true
    }

    pub fn mob_distance(&self, player: u64, eye: DVec3, look: DVec3, reach: f64) -> Option<f64> {
        let frame = minecraft_terrain::item_frame::on_ray(&self.view.frames, eye, look, reach).map(|(_, distance)| distance);
        let other = crate::pvp::player_on_ray(&self.player_boxes, player, eye, look, reach).map(|(_, distance)| distance);
        self.entity_distance(player, eye, look, reach).into_iter().chain(frame).chain(other).min_by(f64::total_cmp)
    }

    fn entity_distance(&self, player: u64, eye: DVec3, look: DVec3, reach: f64) -> Option<f64> {
        let mob = mob_on_ray(&self.view.world, player, eye, look, reach).map(|(_, distance)| distance);
        let fireball = self.view.world.fireball_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let bullet = self.view.world.shulker_bullet_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let end = self.end_on_ray(eye, look, reach).map(|(_, distance)| distance);
        [mob, fireball, bullet, end].into_iter().flatten().min_by(f64::total_cmp)
    }

    pub fn frame_on_ray(&self, player: u64, eye: DVec3, look: DVec3, reach: f64, block: Option<f64>) -> Option<u64> {
        frame_target(&self.view.frames, eye, look, reach, block, self.entity_distance(player, eye, look, reach))
    }

    pub fn use_frame(&mut self, player: u64, id: u64, held: Option<&ItemStack>, infinite: bool, sent: Option<SentItemUse>) -> bool {
        let Some(view) = self.view.frames.iter().find(|view| view.id == id) else {
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
        let fireball = self.view.world.fireball_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let end = self.end_on_ray(eye, look, reach).map(|(_, distance)| distance);
        let others = fireball.into_iter().chain(end).min_by(f64::total_cmp);
        let Some(hit) = mob_to_use(&self.view.world, player, eye, look, reach, block, others) else {
            return false;
        };
        if !minecraft_terrain::mob_actions::would_interact(&self.view.world, hit, inventory, slot, infinite, player) {
            return false;
        }
        self.server.mob_action(player, hit, None, inventory, slot, infinite);
        true
    }

    fn end_on_ray(&self, eye: DVec3, look: DVec3, reach: f64) -> Option<(EndTarget, f64)> {
        let crystal = minecraft_terrain::end_crystal::on_ray(&self.view.crystals, eye, look, reach).map(|(id, distance)| (EndTarget::Crystal(id), distance));
        let parts = self.view.dragons.iter().filter(|dragon| dragon.health > 0.0).flat_map(|dragon| {
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

    pub fn ring_bell(&mut self, pos: (i32, i32, i32), face: &'static str) {
        self.server.ring_bell(pos, face);
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
        minecraft_terrain::effect_cloud::breath_near(&self.view.clouds, minecraft_terrain::effect_cloud::bottle_reach(feet, height))
    }

    pub fn awaits_item_use(&self, player: u64) -> bool {
        self.players.0.get(&player).is_some_and(|queue| queue.item_use_waiting.is_some())
    }

    pub fn take_mounts(&mut self, player: u64) -> Vec<u64> {
        self.players.0.get_mut(&player).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.mounts))
    }

    pub fn take_mount_menus(&mut self, player: u64) -> Vec<u64> {
        self.players.0.get_mut(&player).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.mount_menus))
    }

    pub fn set_mount_inventory(&mut self, inventory: &minecraft_terrain::mount_inventory::MountInventory) {
        self.server.mount_inventory(inventory);
    }

    #[cfg(test)]
    pub(crate) fn want_mount_menu(&mut self, player: u64, vehicle: u64) {
        self.players.queue(player).mount_menus.push(vehicle);
    }

    pub fn mount_viewer(&self, vehicle: u64) -> Option<u64> {
        self.mount_viewers.get(&vehicle).copied()
    }

    pub fn set_mount_viewer(&mut self, player: u64, vehicle: Option<u64>) {
        self.mount_viewers.retain(|_, viewer| *viewer != player);
        if let Some(vehicle) = vehicle {
            self.mount_viewers.insert(vehicle, player);
        }
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

    pub fn merchant(&mut self, player: u64, op: minecraft_terrain::server::MerchantOp, inventory: &Inventory, selected: usize, feet: [f64; 3]) {
        self.server.merchant(player, op, inventory, selected, feet);
        self.players.queue(player).merchant_waiting += 1;
    }

    pub fn take_merchant(&mut self, player: u64) -> Vec<MerchantNews> {
        self.players.0.get_mut(&player).map_or_else(Vec::new, |queue| std::mem::take(&mut queue.merchant))
    }

    pub fn awaits_merchant(&self, player: u64) -> bool {
        self.players.0.get(&player).is_some_and(|queue| queue.merchant_waiting > 0)
    }

    pub fn close_merchant_now(&mut self, player: u64, inventory: &mut Inventory, selected: usize, feet: [f64; 3]) {
        self.settle_merchant_now(player, inventory);
        self.server.merchant(player, minecraft_terrain::server::MerchantOp::Close, inventory, selected, feet);
        self.settle_merchant_now(player, inventory);
    }

    fn settle_merchant_now(&mut self, player: u64, inventory: &mut Inventory) {
        self.server.wait_idle();
        self.poll_jobs();
        let mut answers = Vec::new();
        for output in &mut self.unread {
            let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut output.merchant).into_iter().partition(|(each, update)| *each == player && !update.closing);
            output.merchant = rest;
            answers.extend(mine.into_iter().map(|(_, update)| update));
        }
        let queue = self.players.queue(player);
        for news in std::mem::take(&mut queue.merchant_routed) {
            if let MerchantNews::Answer(update) = news {
                apply_merchant_answer(inventory, &update);
            }
        }
        for update in &answers {
            apply_merchant_answer(inventory, update);
        }
        queue.merchant.clear();
        queue.merchant_waiting = 0;
    }

    pub fn vehicle_uuid(&self, vehicle: u64) -> u128 {
        self.view.world.uuid_of(vehicle)
    }

    pub fn ride_seat(&mut self, player: u64, vehicle: u64) -> Option<DVec3> {
        if self.presenter == Some(player) {
            self.view.client.set_own_vehicle(Some(vehicle));
        }
        let mob = self.view.client.get(vehicle)?;
        let partial = (self.view.ride_clock / self.view.tick_seconds).clamp(0.0, 1.0);
        let pose = mob.pose(partial as f32);
        let point = self.view.world.passenger_point(vehicle)?;
        let strider = self.view.world.striders().iter().any(|e| e.id == vehicle);
        let bob = if strider { crate::riding::bob(pose.walk_position, pose.walk_speed) } else { 0.0 };
        Some(crate::riding::seat(mob.old_position, mob.position, partial, point.y, bob) + DVec3::new(point.x, 0.0, point.z))
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
            self.view.client.set_own_vehicle(None);
        }
    }

    pub fn can_boost(&self, player: u64, vehicle: u64) -> bool {
        self.players.0.get(&player).is_none_or(|queue| queue.boost_waiting.is_none())
            && self.view.world.controlling_player(vehicle) == Some(player)
            && self.view.world.striders().iter().any(|e| e.id == vehicle && !e.strider.steering.boosting)
    }

    pub fn shield_blocked(&mut self, attacker: u64, player: u64, feet: [f64; 3]) {
        self.server.shield_block(attacker, player, feet);
    }

    pub fn boost(&mut self, player: u64, slot: usize, held: String) {
        self.server.boost(player);
        self.players.queue(player).boost_waiting = Some((slot, held));
    }

    pub fn spend_boost(&mut self, inventory: &mut Inventory, slot: usize, infinite: bool) -> Option<minecraft_terrain::bucket::Sound> {
        let random = &mut self.view.random;
        if !minecraft_terrain::durability::spend_boost(inventory, slot, infinite, || random.next_float()) {
            return None;
        }
        self.broken_items.push(minecraft_terrain::durability::STEERING_ITEM.to_owned());
        Some(minecraft_terrain::flint::break_sound(self.view.random.next_float()))
    }

    pub fn leave(&mut self, player: u64, inventory: &mut Inventory, selected: usize, feet: Option<[f64; 3]>) -> u32 {
        self.set_mount_viewer(player, None);
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
            let y_rot = minecraftoss_core::entity_data::wrap_degrees(self.view.random.next_float() * 360.0);
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
        self.view.animate_tick(scene, feet)
    }

    pub fn present_level_events(&mut self, scene: &HandcraftedScene) {
        for (id, pos, data) in std::mem::take(&mut self.level_events) {
            self.view.level_event(scene, id, pos, data);
        }
    }

    pub fn evaporation_smoke(&mut self, pos: (i32, i32, i32)) {
        let effects = minecraft_terrain::level_events::evaporation(pos, &mut self.view.random);
        self.view.present(effects);
    }

    pub fn track_body(&mut self, key: u128, feet: [f64; 3], width: f64, height: f64) {
        self.view.track_body(key, feet, width, height);
    }

    pub fn hold_rope(&mut self, key: u128, feet: [f64; 3], yaw: f32, height: f64) {
        self.view.hold_rope(key, feet, yaw, height);
    }

    pub fn leash_fence(&mut self, player: u64, pos: (i32, i32, i32), sneaking: bool) -> bool {
        let mut world = self.view.world.clone();
        if !world.leash_fence(player, pos, sneaking) {
            return false;
        }
        self.server.leash_fence(player, pos, sneaking);
        true
    }

    pub fn has_knot(&self, pos: (i32, i32, i32)) -> bool {
        self.view.world.has_knot(pos)
    }

    pub fn totem_pop(&mut self, key: u128, feet: [f64; 3], width: f64, height: f64) {
        self.view.totem_pop(key, feet, width, height);
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
        self.view.ambient.douse(pos, signal);
    }

    pub fn rain_tick(&mut self, scene: &HandcraftedScene, camera: [f64; 3]) -> Vec<minecraft_terrain::display_ticks::AmbientSound> {
        self.view.rain_tick(scene, camera, self.weather.rain_level)
    }

    pub fn partial_tick(&self) -> f32 {
        (self.view.clock / TICK_SECONDS).clamp(0.0, 1.0) as f32
    }

    pub fn weather_mesh(&self, scene: &HandcraftedScene, camera: [f64; 3], light: &minecraft_terrain::lighting::SkyLight) -> (Vec<minecraft_terrain::weather::WeatherVertex>, Vec<u32>) {
        self.view.weather_mesh(scene, camera, light, (self.weather.rain_level, self.partial_tick()))
    }

    pub fn campfire_tick(&mut self, scene: &HandcraftedScene, food: impl Fn((i32, i32, i32)) -> [bool; 4]) {
        self.view.ambient.campfire_tick(scene, food);
    }

    /// Client ticks run so far.
    pub fn client_ticks(&self) -> u64 {
        self.view.ticks
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
            if let Some(entity) = self.view.world.fireballs().iter().find(|e| e.id == id) {
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
            if let Some(entity) = self.view.world.fireballs().iter().find(|e| e.id == id) {
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
        let presenter = actors.first().map(|actor| actor.id);
        self.presenter = presenter;
        let presenter_feet = actors.first().map_or([0.0; 3], |actor| actor.view.feet);
        for actor in actors.iter_mut() {
            let queue = self.players.queue(actor.id);
            queue.feet = Some(actor.view.feet);
            queue.raid_omens_waiting.append(&mut actor.view.omens.raid_omens);
        }
        let orb_pickup_target = DVec3::from_array(presenter_feet) + DVec3::Y * f64::from(EYE_HEIGHT / 2.0);
        let near: Vec<DVec3> = actors.iter().filter(|actor| actor.view.alive && !actor.view.spectator).map(|actor| DVec3::from_array(actor.view.feet)).collect();
        let ticked = self.view.advance(dt, client, &near, orb_pickup_target);
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
                heroes: actors.iter().map(|actor| (actor.id, actor.view.hero)).collect(),
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
                watchers: self.watchers.clone(),
                player_hurts: Vec::new(),
                spawn_mobs: self.spawn_mobs,
                controls: actors.iter().filter(|actor| actor.view.alive).map(|actor| player_controls(actor.id, &actor.view, actor.inventory)).collect(),
                omens: actors
                    .iter()
                    .map(|actor| {
                        let raid_omens = std::mem::take(&mut self.players.queue(actor.id).raid_omens_waiting);
                        (actor.id, minecraft_terrain::server::PlayerOmens { bad_omen: actor.view.omens.bad_omen, raid_omens })
                    })
                    .collect(),
            }, batch.normal.clone());
        }
        let mut changes = Vec::new();
        for mut output in self.take_outputs() {
            self.jobs_running = self.jobs_running.saturating_sub(output.jobs.len());
            self.server_handled = output.handled;
            for &(_, id, position, value) in output.orbs_taken.iter().filter(|orb| Some(orb.0) == presenter) {
                let tick_count = self.view.orbs.iter().find(|orb| orb.id == id).map_or(0, |orb| orb.tick_count);
                let orb = ServerOrb { id, position, previous_position: position, value, tick_count };
                self.view.orb_pickups.push(minecraft_terrain::orb_render::OrbPickup::new(orb, orb_pickup_target));
            }
            if let Some(mut snapshot) = output.entities.take() {
                snapshot.swirls.extend(actors.iter().filter(|actor| !actor.view.spectator).filter_map(|actor| actor.view.swirl.clone()));
                if !self.watchers.is_empty() {
                    let mut kept = snapshot.clone();
                    if let Some(mut older) = self.friend_snapshot.take() {
                        crate::mob_sender::keep_bursts(&mut older, &mut kept);
                    }
                    self.friend_snapshot = Some(kept);
                }
                self.view.receive(&mut snapshot, &output.changes);
                self.server_snapshot = Some(snapshot);
            }
            let difficulty = self.difficulty;
            for &(player, (_, position, ..)) in &output.picked {
                let by_friend = Some(player) != presenter;
                if !by_friend && !self.heard.listening() {
                    continue;
                }
                let pitch = ((self.view.random.next_float() - self.view.random.next_float()) * 0.7 + 1.0) * 2.0;
                if by_friend {
                    self.view.sounds.push((crate::heard::ITEM_PICKUP_SOUND.to_owned(), DVec3::from_array(position), 0.2, pitch));
                }
                self.heard.push(crate::heard::Said::all(crate::heard::Heard::sound(crate::heard::ITEM_PICKUP_SOUND, position, 0.2, pitch)));
            }
            let mobs = output.mobs.take();
            let players = &self.players;
            let is_player = |id: u64| players.0.contains_key(&id);
            unharmed_without_pvp(&mut output.players_struck, self.pvp, is_player);
            unharmed_by_potions_without_pvp(&mut output.player_potions, self.pvp, is_player);
            spared_by_blasts_without_pvp(&mut output.player_hits, &mut output.players_pushed, self.pvp, is_player);
            let unclaimed = {
                let world = mobs.as_deref().unwrap_or(&self.view.world);
                let dragons = &self.view.dragons;
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
                self.view.spell.burst(broken.position, broken.color, broken.instant);
                self.view.potion_crumbs.push(minecraft_terrain::spell_particles::break_center(broken.position));
                let event = if broken.instant { crate::heard::INSTANT_POTION_EVENT } else { crate::heard::SPLASH_POTION_EVENT };
                let pos = broken.position.floor().as_ivec3().to_array();
                self.heard.push(crate::heard::Said::all(crate::heard::Heard::Event { event, pos, data: broken.color as i32 }));
            }
            for (from, to) in output.egg_teleports {
                for (start, motion) in minecraft_terrain::dragon_egg::teleport_particles(from, to, &mut self.view.random) {
                    self.view.portal.emit(start, motion);
                }
            }
            for hit in output.thrown_hits {
                if hit.kind == minecraft_terrain::thrown::Throwable::EnderPearl {
                    for (start, motion) in minecraft_terrain::thrown::pearl_portal(hit.at, &mut self.view.random) {
                        self.view.portal.emit(start, motion);
                    }
                } else {
                    let velocities = minecraft_terrain::thrown::hit_crumbs(hit.kind, &mut self.view.random);
                    self.view.item_crumbs.push((hit.item, hit.at, velocities));
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
            self.view.spawners.block_entities(&output.block_entities);
            if self.heard.listening() {
                self.spawner_tags.extend(output.block_entities.iter().filter_map(|(pos, tag)| tag.as_ref().filter(|tag| minecraft_terrain::spawner_display::is_spawner(tag)).map(|tag| (*pos, tag.clone()))));
            }
            self.block_entity_changes.extend(output.block_entities);
            self.destroyed.extend(output.destroyed);
            let camera = DVec3::from_array(presenter_feet) + DVec3::Y * f64::from(EYE_HEIGHT);
            for sound in &output.mob_sounds {
                let heard = minecraft_terrain::ender_dragon::heard_at(&sound.event, sound.position, camera);
                self.view.sounds.push((sound.event.clone(), heard, sound.volume, sound.pitch));
                self.heard.push(crate::heard::Said::all(crate::heard::Heard::sound(&sound.event, sound.position.to_array(), sound.volume, sound.pitch)));
            }
            for sound in output.mob_results.iter().flat_map(|(_, result)| result.sounds.iter()) {
                self.sound(&sound.event, sound.position, sound.volume, sound.pitch);
            }
            // `ServerExplosion`'s sound: loud, pitched down.
            for blast in &output.explosions {
                let pitch = (1.0 + (self.view.random.next_float() - self.view.random.next_float()) * 0.2) * 0.7;
                self.sound("minecraft:entity.generic.explode", blast.position, 4.0, pitch);
            }
            for &(event, (x, y, z), data) in &output.level_events {
                self.heard.push(crate::heard::Said::all(crate::heard::Heard::Event { event, pos: [x, y, z], data }));
            }
            self.level_events.extend(output.level_events);
            for effect in &output.explosion_effects {
                self.view.ambient.explosion(DVec3::from_array(effect.center), effect.radius, effect.blocks, effect.interacts);
                let blocks = u32::try_from(effect.blocks).unwrap_or(u32::MAX);
                let heard = crate::heard::Heard::Explosion { center: effect.center, radius: effect.radius, blocks, interacts: effect.interacts };
                self.heard.push(crate::heard::Said::all(heard));
            }
            for puff in &output.bullet_puffs {
                self.view.ambient.puffs(bullet_puff_particle(puff.kind), puff.position, puff.kind.count(), 0.2);
            }
            if let Some(mobs) = mobs {
                self.view.world = *mobs;
                let shown = shown_mobs(&self.view.world, presenter_feet, self.view_distance);
                self.view.show_mobs(shown);
            }
            self.tracked.merge(std::mem::take(&mut output.tracked));
        }
        for actor in actors.iter_mut() {
            let Some(queue) = self.players.0.get_mut(&actor.id) else {
                continue;
            };
            settle_eggs(queue, actor.inventory);
            for slots in std::mem::take(&mut queue.mob_slots) {
                for (slot, stack) in slots {
                    if let Some(held) = actor.inventory.slots.get_mut(slot) {
                        *held = stack;
                    }
                }
            }
            settle_merchant_news(queue, actor.inventory);
            let broken = std::mem::take(&mut queue.broken);
            if presenter == Some(actor.id) {
                self.broken_items.extend(broken);
            }
        }
        if ticked {
            self.server_items_tick(actors);
            // `ItemEntity.playerTouch`'s pickup pop.
            for _ in 0..self.view.world_items.take_pickup_sounds() {
                let pitch = ((self.view.random.next_float() - self.view.random.next_float()) * 0.7 + 1.0) * 2.0;
                self.view.sounds.push(("minecraft:entity.item.pickup".to_owned(), DVec3::from_array(presenter_feet), 0.2, pitch));
            }
        }
        changes
    }

    pub fn mob_present(&self, id: u64) -> bool {
        self.entity_type(id).is_some()
    }

    fn entity_type(&self, id: u64) -> Option<&'static str> {
        self.view.world
            .entity_type(id)
            .or_else(|| self.view.dragons.iter().any(|dragon| dragon.id == id).then_some(minecraft_terrain::ender_dragon::ENDER_DRAGON))
    }

    pub fn dragons(&self) -> &[minecraft_terrain::ender_dragon::DragonView] {
        &self.view.dragons
    }

    pub fn gateway(&self, pos: (i32, i32, i32)) -> Option<&minecraft_terrain::end_gateway::GatewayView> {
        self.view.gateways.get(pos)
    }

    pub fn gateway_views(&self) -> &[minecraft_terrain::end_gateway::GatewayView] {
        self.view.gateways.views()
    }

    pub fn mark_cooling(&mut self, pos: (i32, i32, i32)) {
        self.view.gateways.mark_cooling(pos, self.view.ticks);
    }

    pub fn gateway_cooling(&self, pos: (i32, i32, i32)) -> bool {
        self.view.gateways.is_cooling(pos, self.view.ticks)
    }

    pub fn eye_views(&self) -> &[minecraft_terrain::eye_of_ender::EyeView] {
        self.view.eyes.views()
    }

    pub fn take_dragons_killed(&mut self) -> Vec<[i32; 4]> {
        std::mem::take(&mut self.dragons_killed)
    }

    pub fn take_fight_reports(&mut self) -> Vec<minecraft_terrain::server::FightReport> {
        std::mem::take(&mut self.fight_reports)
    }

    pub fn dragon_alive(&self, uuid: [i32; 4]) -> bool {
        self.view.dragons.iter().any(|dragon| dragon.uuid == uuid)
    }

    pub fn boxes(&self) -> Vec<(u64, [f64; 6])> {
        self.view.boxes()
    }

    pub fn monster_near(&self, area: [f64; 6], sleeper: u64) -> bool {
        crate::sleep::monster_in(area, resting_monsters(&self.view.world, sleeper))
    }

    pub fn shot_boxes(&self, riding: Option<u64>) -> Vec<(u64, [f64; 6])> {
        self.view.shot_boxes(riding)
    }
}

fn shown_mobs(world: &EntityWorld, feet: [f64; 3], view_distance: i32) -> Vec<minecraft_terrain::client_mobs::ServerMob> {
    server_mobs(world)
        .into_iter()
        .filter(|mob| {
            let range = world.entity_type(mob.id).map_or(minecraft_terrain::mob_tracking::DEFAULT_RANGE, minecraft_terrain::mob_tracking::tracking_range);
            minecraft_terrain::mob_tracking::tracks(range, view_distance, feet, mob.position)
        })
        .collect()
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
        out.push((e.spider.type_id(), false, bounds(&e.spider.body)));
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

fn unharmed_without_pvp(struck: &mut Vec<minecraft_terrain::thrown::PlayerStruck>, pvp: bool, is_player: impl Fn(u64) -> bool) {
    if !pvp {
        struck.retain(|struck| !is_player(struck.by));
    }
}

fn unharmed_by_potions_without_pvp(hits: &mut Vec<minecraft_terrain::potions::PlayerPotionHit>, pvp: bool, is_player: impl Fn(u64) -> bool) {
    if !pvp {
        hits.retain(|hit| !matches!(hit.potion, minecraft_terrain::potions::PlayerPotion::Harm { by: Some(by), .. } if is_player(by)));
    }
}

fn spared_by_blasts_without_pvp(hits: &mut Vec<minecraftoss_entities::world::PlayerHit>, pushed: &mut Vec<(u64, DVec3)>, pvp: bool, is_player: impl Fn(u64) -> bool) {
    if pvp {
        return;
    }
    hits.retain(|hit| match hit.kind {
        PlayerHitKind::Explosion { knockback, by: Some(by) } if is_player(by) => {
            pushed.push((hit.player_id, knockback));
            false
        }
        _ => true,
    });
}

fn struck_hit(feet: [f64; 3], struck: &minecraft_terrain::thrown::PlayerStruck) -> Hit {
    let from = DVec3::from_array(feet) - struck.velocity.normalize_or_zero();
    (struck.damage, Some(from.to_array()), Some([-struck.velocity.x, -struck.velocity.z]), crate::death::DamageCause::Generic, Default::default(), None)
}

fn explosion_push(kind: &PlayerHitKind) -> Option<DVec3> {
    match *kind {
        PlayerHitKind::Explosion { knockback, .. } | PlayerHitKind::BadRespawnPoint { knockback } => Some(knockback),
        _ => None,
    }
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
        PlayerHitKind::Fireworks { at } => Some(at.to_array()),
        _ => None,
    };
    let toward = match hit.kind {
        PlayerHitKind::Melee { attacker, .. } => [attacker.x - feet[0], attacker.z - feet[2]],
        PlayerHitKind::Arrow { velocity, .. } | PlayerHitKind::Fireball { velocity, .. } | PlayerHitKind::ShulkerBullet { velocity } | PlayerHitKind::Trident { velocity } | PlayerHitKind::Spit { velocity } => [-velocity.x, -velocity.z],
        PlayerHitKind::Explosion { knockback, .. } | PlayerHitKind::BadRespawnPoint { knockback } => [-knockback.x, -knockback.z],
        PlayerHitKind::Magic { at } | PlayerHitKind::Fireworks { at } => [at.x - feet[0], at.z - feet[2]],
    };
    let toward = (!matches!(hit.kind, PlayerHitKind::Magic { .. })).then_some(toward);
    let attack = match hit.kind {
        PlayerHitKind::Melee { .. } => crate::death::MobAttack::Melee,
        PlayerHitKind::Arrow { .. } => crate::death::MobAttack::Arrow,
        PlayerHitKind::Explosion { .. } | PlayerHitKind::BadRespawnPoint { .. } => crate::death::MobAttack::Explosion,
        PlayerHitKind::Fireball { ignite_ticks, .. } if ignite_ticks > 0 => crate::death::MobAttack::SmallFireball,
        PlayerHitKind::Fireball { .. } => crate::death::MobAttack::Fireball,
        PlayerHitKind::ShulkerBullet { .. } => crate::death::MobAttack::MobProjectile,
        PlayerHitKind::Trident { .. } => crate::death::MobAttack::Trident,
        PlayerHitKind::Spit { .. } => crate::death::MobAttack::Spit,
        PlayerHitKind::Magic { .. } => crate::death::MobAttack::Magic,
        PlayerHitKind::Fireworks { .. } => crate::death::MobAttack::Fireworks,
    };
    let cause = match hit.kind {
        PlayerHitKind::BadRespawnPoint { .. } => crate::death::DamageCause::BadRespawnPoint,
        PlayerHitKind::Magic { .. } if hit.source.is_none() => crate::death::DamageCause::Magic,
        PlayerHitKind::Fireworks { .. } if hit.source.is_none() || type_id.is_none() => crate::death::DamageCause::Fireworks,
        PlayerHitKind::Explosion { by: Some(by), .. } => crate::death::DamageCause::PlayerExplosion { client: u32::try_from(by).unwrap_or(u32::MAX) },
        _ => crate::death::DamageCause::mob(attack, hit.source, type_id),
    };
    let scales = matches!(hit.kind, PlayerHitKind::Explosion { .. } | PlayerHitKind::BadRespawnPoint { .. }) || hit.source.is_some();
    let damage = difficulty_scaled(hit.damage, difficulty, scales);
    let effect = match hit.kind {
        PlayerHitKind::Arrow { effect, .. } => effect,
        PlayerHitKind::ShulkerBullet { .. } => Some((EffectKind::Levitation, minecraftoss_entities::shulker_bullet::LEVITATION_TICKS)),
        _ => None,
    };
    (damage > 0.0).then_some((damage, from, toward, cause, melee_push(&hit.kind), effect))
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

pub(crate) fn mirrored(id: i32) -> u32 {
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
pub(crate) fn bob_offset(id: i32) -> f32 {
    let hash = (id as u32).wrapping_mul(0x9e37_79b9).rotate_left(13).wrapping_mul(0x85eb_ca6b);
    hash as f32 / u32::MAX as f32 * std::f32::consts::TAU
}

/// A mob hit as one number: the kind in the top byte, the id below.
pub(crate) fn encode(hit: MobHit) -> u64 {
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
        MobHit::Feline(_) => FELINE_KEY,
        MobHit::WanderingTrader(_) => WANDERING_TRADER_KEY,
        MobHit::SnowGolem(_) => SNOW_GOLEM_KEY,
        MobHit::Illager(_) => ILLAGER_KEY,
        MobHit::Vex(_) => VEX_KEY,
        MobHit::Ravager(_) => RAVAGER_KEY,
    };
    (kind << 56) | (hit.id() & ((1 << 56) - 1))
}

pub(crate) fn decode(key: u64) -> Option<MobHit> {
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
        FELINE_KEY => MobHit::Feline(id),
        WANDERING_TRADER_KEY => MobHit::WanderingTrader(id),
        SNOW_GOLEM_KEY => MobHit::SnowGolem(id),
        ILLAGER_KEY => MobHit::Illager(id),
        VEX_KEY => MobHit::Vex(id),
        RAVAGER_KEY => MobHit::Ravager(id),
        _ => return None,
    })
}

pub(crate) const FIREBALL_KEY: u64 = 17 << 56;
pub(crate) const SHULKER_BULLET_KEY: u64 = 27 << 56;
pub(crate) const CRYSTAL_KEY: u64 = 19 << 56;
const DRAGON_KEY: u64 = 20 << 56;
const STRIDER_KEY: u64 = 23;
const HOGLIN_KEY: u64 = 22;
const PIGLIN_KEY: u64 = 21;
const FELINE_KEY: u64 = 29;
const WANDERING_TRADER_KEY: u64 = 30;
const SNOW_GOLEM_KEY: u64 = 31;
const ILLAGER_KEY: u64 = 32;
const VEX_KEY: u64 = 33;
const RAVAGER_KEY: u64 = 34;
pub(crate) const FRAME_KEY: u64 = 28 << 56;
pub const FLAT_HIT: u64 = 1 << 55;

pub(crate) fn dragon_part_key(id: u64, part: usize) -> u64 {
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
            hero: None,
            sleeping: None,
            sleep_timer: 0,
            forward: 0.0,
            strafe: 0.0,
            jumping: false,
            swirl: None,
            omens: Default::default(),
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
        let view = PlayerView { feet: [0.5, 200.0, 0.5], alive: true, attackable: true, spectator: false, picks_up: false, health: 20.0, yaw: 0.0, pitch: 0.0, main_hand: None, offhand: None, sneaking: false, hero: None, sleeping: None, sleep_timer: 0, forward: 0.0, strafe: 0.0, jumping: false, swirl: None, omens: Default::default() };
        let started = std::time::Instant::now();
        let tick = |entities: &mut Entities, inventory: &mut Inventory| {
            assert!(started.elapsed().as_secs() < 60, "the server answers");
            std::thread::sleep(std::time::Duration::from_millis(5));
            let batch = crate::tick_rate::TickBatch { normal: if entities.ready_for_ticks() { vec![true] } else { Vec::new() }, overload: None };
            entities.tick(&mut [TickActor { id: HOST, view: view.clone(), inventory, selected: 3 }], TICK_SECONDS, &batch, Default::default(), Default::default(), true);
        };
        while entities.view.world.pigs().is_empty() {
            tick(&mut entities, &mut inventory);
        }
        let eye = entities.view.world.pigs()[0].pig.body.position + DVec3::new(0.0, 0.5, -2.0);
        assert!(!entities.use_on_mob(HOST, eye, DVec3::Z, 3.0, None, &inventory, 0, false), "an empty hand does nothing to a pig");
        assert!(entities.use_on_mob(HOST, eye, DVec3::Z, 3.0, None, &inventory, 3, false));
        while inventory.slots[3].is_some() {
            tick(&mut entities, &mut inventory);
        }
        while !entities.view.world.pigs()[0].pig.saddled {
            tick(&mut entities, &mut inventory);
        }
        assert!(!entities.use_on_mob(HOST, eye, DVec3::Z, 3.0, None, &inventory, 3, false), "a saddled pig takes no second saddle");
    }

    #[test]
    fn a_dragon_flaps_when_its_wings_pass_the_top_of_their_beat() {
        use minecraft_terrain::client_dragon::flapping as dragon_flapping;
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
        let view = PlayerView { feet: [0.0; 3], alive: true, attackable: true, spectator: false, picks_up: true, health: 20.0, yaw: 0.0, pitch: 0.0, main_hand: None, offhand: None, sneaking: false, hero: None, sleeping: None, sleep_timer: 0, forward: 0.0, strafe: 0.0, jumping: false, swirl: None, omens: Default::default() };
        let controls = player_controls(HOST, &view, &inventory);
        assert_eq!(controls.armor, [Some("minecraft:golden_boots".to_owned()), None, None, Some("minecraft:chainmail_helmet".to_owned())]);
    }

    #[test]
    fn a_cat_and_an_ocelot_hit_box_encodes_and_decodes() {
        let key = encode(MobHit::Feline(77));
        assert_eq!(key >> 56, 29);
        assert_eq!(decode(key), Some(MobHit::Feline(77)));
        assert_eq!(fireball_of(key), None);
        assert_eq!(crystal_of(key), None);
        assert_eq!(end_target_of(key), None);
    }

    #[test]
    fn a_pillager_hit_box_encodes_and_decodes() {
        let key = encode(MobHit::Illager(83));
        assert_eq!(key >> 56, 32);
        assert_eq!(decode(key), Some(MobHit::Illager(83)));
        assert_eq!(fireball_of(key), None);
        assert_eq!(crystal_of(key), None);
        assert_eq!(end_target_of(key), None);
    }

    #[test]
    fn a_ravager_hit_box_encodes_and_decodes() {
        let key = encode(MobHit::Ravager(91));
        assert_eq!(key >> 56, 34);
        assert_eq!(decode(key), Some(MobHit::Ravager(91)));
    }

    #[test]
    fn a_vex_hit_box_encodes_and_decodes() {
        let key = encode(MobHit::Vex(84));
        assert_eq!(key >> 56, 33);
        assert_eq!(decode(key), Some(MobHit::Vex(84)));
    }

    #[test]
    fn a_snow_golem_hit_box_encodes_and_decodes() {
        let key = encode(MobHit::SnowGolem(77));
        assert_eq!(key >> 56, 31);
        assert_eq!(decode(key), Some(MobHit::SnowGolem(77)));
        assert_eq!(fireball_of(key), None);
        assert_eq!(crystal_of(key), None);
        assert_eq!(end_target_of(key), None);
    }

    #[test]
    fn a_wandering_trader_hit_box_encodes_and_decodes() {
        let key = encode(MobHit::WanderingTrader(91));
        assert_eq!(key >> 56, 30);
        assert_eq!(decode(key), Some(MobHit::WanderingTrader(91)));
        assert_eq!(fireball_of(key), None);
        assert_eq!(crystal_of(key), None);
        assert_eq!(end_target_of(key), None);
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
    fn evoker_fangs_bite_past_shields_as_the_evokers_indirect_magic() {
        let hit = minecraftoss_entities::world::PlayerHit {
            player_id: HOST,
            damage: 6.0,
            kind: PlayerHitKind::Magic { at: DVec3::new(1.0, 64.0, 0.0) },
            source: Some(9),
        };
        let damage = |difficulty| mob_hit(&hit, [0.0, 64.0, 0.0], difficulty, Some("minecraft:evoker")).map(|h| h.0);
        assert_eq!((damage(0), damage(1), damage(2), damage(3)), (None, Some(4.0), Some(6.0), Some(9.0)));
        let (_, _, toward, cause, _, effect) = mob_hit(&hit, [0.0, 64.0, 0.0], 2, Some("minecraft:evoker")).unwrap();
        assert_eq!(toward, None, "no shield blocks it");
        assert_eq!(cause, crate::death::DamageCause::Mob { id: 9, type_id: "minecraft:evoker", attack: crate::death::MobAttack::Magic });
        assert_eq!(cause.damage_type(), "minecraft:indirect_magic");
        assert!(cause.bypasses_armor() && effect.is_none());
        assert_eq!(crate::offhand::blocked_damage(0.0, toward, 6.0), 0.0);
        let ownerless = minecraftoss_entities::world::PlayerHit { source: None, ..hit };
        let (damage, _, _, cause, _, _) = mob_hit(&ownerless, [0.0, 64.0, 0.0], 3, None).unwrap();
        assert_eq!((damage, cause), (6.0, crate::death::DamageCause::Magic));
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
    fn an_explosion_pushes_the_player_it_hit_even_when_it_does_no_damage() {
        let mut queues = two_queues();
        let mut output = minecraft_terrain::server::Output::default();
        let knockback = DVec3::new(0.6, 0.3, 0.0);
        output.player_hits.push(minecraftoss_entities::world::PlayerHit { player_id: FRIEND, damage: 20.0, kind: PlayerHitKind::Explosion { knockback, by: None }, source: None });
        output.player_hits.push(minecraftoss_entities::world::PlayerHit { player_id: HOST, damage: 9.0, kind: PlayerHitKind::BadRespawnPoint { knockback: DVec3::Z }, source: None });
        output.player_hits.push(minecraftoss_entities::world::PlayerHit { player_id: HOST, damage: 4.0, kind: PlayerHitKind::Arrow { velocity: DVec3::X, effect: None }, source: None });
        queues.route(&mut output, 2, |_| None);
        let friend = queues.queue(FRIEND);
        assert_eq!(friend.out.pushes, [knockback]);
        assert_eq!(friend.out.hits.len(), 1);
        assert_eq!(friend.out.hits[0].3, crate::death::DamageCause::Explosion);
        assert_eq!(queues.queue(HOST).out.pushes, [DVec3::Z], "an arrow pushes through its hit, not here");
        assert_eq!(queues.queue(HOST).out.hits.len(), 2);
        let mut output = minecraft_terrain::server::Output::default();
        output.player_hits.push(minecraftoss_entities::world::PlayerHit { player_id: FRIEND, damage: 20.0, kind: PlayerHitKind::Explosion { knockback, by: None }, source: None });
        let mut queues = two_queues();
        queues.route(&mut output, 0, |_| None);
        let friend = queues.queue(FRIEND);
        assert!(friend.out.hits.is_empty(), "no damage on peaceful");
        assert_eq!(friend.out.pushes, [knockback], "the blast still pushes");
        let mut queues = two_queues();
        let mut output = minecraft_terrain::server::Output::default();
        output.player_hits.push(minecraftoss_entities::world::PlayerHit { player_id: FRIEND, damage: 20.0, kind: PlayerHitKind::Explosion { knockback, by: Some(HOST) }, source: None });
        queues.route(&mut output, 2, |_| None);
        assert_eq!(queues.queue(FRIEND).out.hits[0].3, crate::death::DamageCause::PlayerExplosion { client: 0 }, "a player's TNT names them");
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
    fn trading_screens_reach_only_their_player_and_set_the_slots_before_anything_else() {
        use minecraft_terrain::server::{MerchantUpdate, MerchantView, MobResult};
        let view = |villager: u64| MerchantView { villager, profession: "minecraft:farmer", level: 1, xp: 0, offers: Vec::new(), payment: [None, None], result: None, future_xp: 0, max_stacks: Vec::new(), name: None, trader: false };
        let mut queues = two_queues();
        queues.queue(FRIEND).merchant_waiting = 2;
        let mut output = minecraft_terrain::server::Output { mob_results: vec![(FRIEND, MobResult { merchant: Some(view(9)), ..Default::default() })], ..Default::default() };
        let wheat = ItemStack::new("minecraft:wheat", 20);
        let answer = MerchantUpdate { view: Some(view(9)), slots: vec![(4, None), (12, Some(wheat.clone()))], cursor: Some(ItemStack::new("minecraft:emerald", 1)), closing: false };
        let closing = MerchantUpdate { view: None, slots: Vec::new(), cursor: None, closing: true };
        output.merchant = vec![(FRIEND, closing), (FRIEND, answer)];
        let _ = queues.route(&mut output, 2, |_| None);
        assert!(output.merchant.is_empty());
        assert!(queues.queue(HOST).merchant_routed.is_empty(), "the host traded nothing");
        let friend = queues.queue(FRIEND);
        assert!(friend.merchant.is_empty(), "nothing shows before its slots are set");
        let order: Vec<&str> = friend
            .merchant_routed
            .iter()
            .map(|news| match news {
                MerchantNews::Answer(_) => "answer",
                MerchantNews::Opened(_) => "opened",
                MerchantNews::Closing => "closing",
            })
            .collect();
        assert_eq!(order, ["answer", "opened", "closing"], "answers to the player's clicks, then a screen the tick opened, then the tick's end check");
        let mut inventory = Inventory::default();
        inventory.slots[4] = Some(ItemStack::new("minecraft:wheat", 3));
        inventory.cursor = Some(ItemStack::new("minecraft:dirt", 5));
        settle_merchant_news(friend, &mut inventory);
        assert_eq!(inventory.slots[4], None);
        assert_eq!(inventory.slots[12], Some(wheat));
        assert_eq!(inventory.cursor, Some(ItemStack::new("minecraft:emerald", 1)));
        assert_eq!(friend.merchant_waiting, 1, "one answer came of the two in flight");
        assert_eq!(friend.merchant.len(), 3);
        assert!(friend.merchant_routed.is_empty());
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
            hero: None,
            sleeping: None,
            sleep_timer: 0,
            forward: 0.0,
            strafe: 0.0,
            jumping: false,
            swirl: None,
            omens: Default::default(),
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

    #[test]
    fn a_poisoned_player_swirls_for_the_host_and_in_the_friends_snapshot() {
        let Some(mut entities) = friend_world() else {
            return;
        };
        entities.spawn_mobs = false;
        entities.set_watchers(vec![Watcher { id: FRIEND, feet: FRIEND_FEET, view_distance: 8 }], 8);
        let mut status = minecraftoss_player::survival::SurvivalStatus::default();
        status.add_effect(minecraftoss_player::survival::EffectKind::Poison, 900, 0);
        let poisoned = crate::swirls::player_swirl(&status, FRIEND_FEET, false).expect("a swirl");
        let watching = crate::swirls::player_swirl(&status, HOST_FEET, false);
        let (mut host, mut friend) = (Inventory::default(), Inventory::default());
        let started = std::time::Instant::now();
        while entities.view.swirls.is_empty() {
            assert!(started.elapsed().as_secs() < 300, "no swirl came back");
            let view = |feet, spectator: bool, swirl| PlayerView {
                feet,
                alive: true,
                attackable: !spectator,
                spectator,
                picks_up: !spectator,
                health: 20.0,
                yaw: 0.0,
                pitch: 0.0,
                main_hand: None,
                offhand: None,
                sneaking: false,
                hero: None,
                sleeping: None,
                sleep_timer: 0,
                forward: 0.0,
                strafe: 0.0,
                jumping: false,
                swirl,
                omens: Default::default(),
            };
            let batch = crate::tick_rate::TickBatch {
                normal: if entities.ready_for_ticks() { vec![true] } else { Vec::new() },
                overload: None,
            };
            let mut actors = [
                TickActor { id: HOST, view: view(HOST_FEET, true, watching.clone()), inventory: &mut host, selected: 0 },
                TickActor { id: FRIEND, view: view(FRIEND_FEET, false, Some(poisoned.clone())), inventory: &mut friend, selected: 0 },
            ];
            let _ = entities.tick(&mut actors, 0.05, &batch, crate::tick_rate::ClientTiming::default(), minecraft_terrain::server::ClockTicks::default(), true);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(entities.view.swirls, vec![poisoned.clone()], "a spectator gives off no swirl");
        let sent = entities.take_friend_snapshot().expect("the friend's snapshot");
        assert_eq!(sent.swirls, vec![poisoned]);
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
    fn the_hosts_mirror_skips_mobs_beyond_its_own_range() {
        let mut world = EntityWorld::default();
        let mut spawn = |kind: &str, x: f64| {
            let tag = minecraftoss_core::snbt::parse_compound(&format!("{{id:\"minecraft:{kind}\",Pos:[{x}d,64.0d,0.5d],NoAI:1b}}")).expect("the tag");
            minecraft_terrain::server_mobs::spawn_saved(&mut world, &tag).expect("the mob")
        };
        let near_cow = spawn("cow", 100.5);
        let far_cow = spawn("cow", 150.5);
        let near_zombie = spawn("zombie", 120.5);
        let far_zombie = spawn("zombie", 130.5);
        let bat = spawn("bat", 90.5);
        let shown: Vec<u64> = shown_mobs(&world, [0.5, 64.0, 0.5], 8).iter().map(|mob| mob.id).collect();
        assert!(shown.contains(&near_cow) && shown.contains(&near_zombie), "{shown:?}");
        assert!(!shown.contains(&far_cow), "a cow goes 10 chunks, capped at the view distance of 8");
        assert!(!shown.contains(&far_zombie), "a zombie goes 8 chunks");
        assert!(!shown.contains(&bat), "a bat goes 5 chunks");
    }

    #[test]
    fn a_friend_tracks_the_mob_next_to_them() {
        let Some(mut entities) = friend_world() else {
            return;
        };
        entities.spawn_mobs = false;
        entities.set_watchers(vec![minecraft_terrain::mob_tracking::Watcher { id: FRIEND, feet: FRIEND_FEET, view_distance: 8 }], 8);
        let (mut host, mut friend) = (Inventory::default(), Inventory::default());
        let still = minecraftoss_core::snbt::parse_compound("{NoAI:1b}").expect("the tag");
        entities.summon(minecraft_commands::PlayerId(0), "minecraft:cow".to_owned(), FRIEND_FEET, Some(still));
        let mut tracked = Tracked::default();
        let started = std::time::Instant::now();
        let is_cow = |tag: &minecraftoss_core::nbt::Tag| tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) == Some("minecraft:cow");
        let cow = loop {
            assert!(started.elapsed().as_secs() < 300, "the friend never tracked the cow");
            tick_both(&mut entities, &mut host, &mut friend);
            tracked.merge(entities.take_tracked());
            let added = tracked.events_of(FRIEND).find_map(|event| match event {
                minecraft_terrain::mob_tracking::MobEvent::Add(id, tag) if is_cow(tag) => Some(*id),
                _ => None,
            });
            if let Some(id) = added {
                break id;
            }
        };
        assert_eq!(tracked.events_of(HOST).count(), 0, "the host is not a watcher");
        let moved = loop {
            assert!(started.elapsed().as_secs() < 300, "the cow never moved for the friend");
            tick_both(&mut entities, &mut host, &mut friend);
            if let Some(moves) = entities.take_tracked().moves_of(FRIEND).map(<[_]>::to_vec) {
                break moves;
            }
        };
        assert!(moved.iter().any(|mob| mob.id == cow));
        assert!(entities.take_friend_snapshot().is_some(), "the level's items and projectiles go to the friend too");
        assert!(entities.view.client.get(cow).is_some(), "the host, 35 blocks away, shows the cow too");
    }

    #[test]
    fn a_punch_queues_one_hit_on_the_player_in_front_and_none_without_pvp() {
        let Some(mut entities) = friend_world() else {
            return;
        };
        let feet = [10.5, 201.0, 10.5];
        let host = crate::pvp::PlayerBox::standing(HOST, feet, 1.8, false);
        let friend = crate::pvp::PlayerBox::standing(FRIEND, [10.5, 201.0, 12.5], 1.8, false);
        let other = crate::pvp::PlayerBox::standing(2, [11.5, 201.0, 13.0], 1.8, false);
        entities.set_player_boxes(vec![host, friend, other]);
        let eye = DVec3::from_array(feet) + DVec3::Y * f64::from(EYE_HEIGHT);
        let blow = Blow { sprinting: false, can_critical: false, can_sweep: false, reach: 3.0, block: None, creative: false };
        let inventory = Inventory::default();
        assert_eq!(entities.punch_player(HOST, &inventory, 0, eye, DVec3::Z, 0.0, 1.0, 1.0, &blow), Some(true));
        let hits = entities.take_player_outputs(FRIEND).hits;
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].0, hits[0].3), (1.0, crate::death::DamageCause::Player { client: 0 }));
        assert!(entities.take_player_outputs(HOST).hits.is_empty(), "never the attacker");
        assert!(entities.take_player_outputs(2).hits.is_empty(), "no sweep without a sword");
        assert_eq!(entities.mob_distance(HOST, eye, DVec3::Z, 3.0).map(|d| (d * 100.0).round()), Some(170.0));
        let sweep = Blow { can_sweep: true, ..blow };
        assert_eq!(entities.punch_player(HOST, &inventory, 0, eye, DVec3::Z, 0.0, 1.0, 1.0, &sweep), Some(true));
        assert_eq!(entities.take_player_outputs(FRIEND).hits.len(), 1);
        assert_eq!(entities.take_player_outputs(2).hits.len(), 1, "the sweep reaches the player beside the target");
        let wall = Blow { block: Some(1.0), ..blow };
        assert_eq!(entities.punch_player(HOST, &inventory, 0, eye, DVec3::Z, 0.0, 1.0, 1.0, &wall), None);
        assert_eq!(entities.punch_player(HOST, &inventory, 0, eye, -DVec3::Z, 0.0, 1.0, 1.0, &blow), None);
        entities.pvp = false;
        assert_eq!(entities.punch_player(HOST, &inventory, 0, eye, DVec3::Z, 0.0, 1.0, 1.0, &sweep), Some(false));
        assert!(entities.take_player_outputs(FRIEND).hits.is_empty());
        assert!(entities.take_player_outputs(2).hits.is_empty());
        assert!(entities.view.sounds.iter().any(|(event, ..)| event == crate::pvp::NO_DAMAGE_SOUND));
    }

    #[test]
    fn a_host_drop_reaches_the_level_and_the_friend_picks_it_up() {
        let Some(mut entities) = friend_world() else {
            return;
        };
        let (mut host, mut friend) = (Inventory::default(), Inventory::default());
        entities.view.world_items.spawn(ItemStack::new("minecraft:cobblestone", 3), DVec3::new(12.5, 201.5, 12.5));
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
        assert!(entities.view.world_items.entities.iter().all(|item| item.stack.id != "minecraft:cobblestone"));
        let feet = Some(FRIEND_FEET);
        assert_eq!(entities.leave(FRIEND, &mut friend, 0, feet), 0);
        assert!(entities.player_outputs(FRIEND).is_none(), "the friend's queues are gone");
        assert!(entities.player_outputs(HOST).is_some(), "the host's stay");
    }

    #[test]
    fn a_friends_pickup_is_heard_by_all_and_the_mirrors_own_sounds_are_not_sent() {
        use crate::heard::{Heard, ITEM_PICKUP_SOUND, Said};
        let Some(mut entities) = friend_world() else {
            return;
        };
        assert!(entities.heard.listen(true).is_empty());
        let (mut host, mut friend) = (Inventory::default(), Inventory::default());
        entities.view.world_items.spawn(ItemStack::new("minecraft:cobblestone", 3), DVec3::new(12.5, 201.5, 12.5));
        let started = std::time::Instant::now();
        let mut said = Vec::new();
        while count(&friend, "minecraft:cobblestone") < 3 {
            assert!(started.elapsed().as_secs() < 300, "the friend never picked up the host's drop");
            tick_both(&mut entities, &mut host, &mut friend);
            said.extend(entities.heard.listen(true));
        }
        let pops: Vec<&Said> = said.iter().filter(|said| matches!(&said.heard, Heard::Sound { event, .. } if event == ITEM_PICKUP_SOUND)).collect();
        assert_eq!(pops.len(), 1, "{said:?}");
        assert_eq!(pops[0].only, None, "everyone near hears it");
        let Heard::Sound { at, volume, .. } = &pops[0].heard else {
            unreachable!();
        };
        assert!((at[0] - 12.5).abs() < 1.0 && (at[2] - 12.5).abs() < 1.0, "from the item: {at:?}");
        assert_eq!(*volume, 0.2);
        assert!(entities.view.sounds.iter().any(|(event, ..)| event == ITEM_PICKUP_SOUND), "the host hears the friend's pickup");
        entities.view.sounds.push(("minecraft:entity.blaze.burn".to_owned(), DVec3::new(12.5, 201.0, 12.5), 1.0, 1.0));
        entities.sound("minecraft:block.chest.open", DVec3::new(12.5, 201.5, 12.5), 0.5, 1.0);
        let chest = Heard::sound("minecraft:block.chest.open", [12.5, 201.5, 12.5], 0.5, 1.0);
        assert_eq!(entities.heard.listen(false), vec![Said::all(chest)], "only the level's own sound is sent");
        entities.sound("minecraft:block.chest.close", DVec3::new(12.5, 201.5, 12.5), 0.5, 1.0);
        assert!(entities.heard.listen(true).is_empty(), "nothing is kept while no friend listens");
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

    #[test]
    fn with_pvp_off_a_players_trident_or_wind_charge_spares_every_player() {
        let queues = two_queues();
        let struck = |player: u64, by: u64| minecraft_terrain::thrown::PlayerStruck {
            player,
            by,
            damage: 8.0,
            kind: minecraft_terrain::trident::DAMAGE_TYPE,
            velocity: DVec3::Z,
        };
        let all = vec![struck(HOST, FRIEND), struck(FRIEND, FRIEND), struck(FRIEND, 77)];
        let is_player = |id: u64| queues.0.contains_key(&id);
        let mut kept = all.clone();
        unharmed_without_pvp(&mut kept, true, is_player);
        assert_eq!(kept.len(), 3, "pvp on");
        unharmed_without_pvp(&mut kept, false, is_player);
        assert_eq!(kept.iter().map(|s| (s.player, s.by)).collect::<Vec<_>>(), [(FRIEND, 77)]);
    }

    #[test]
    fn with_pvp_off_a_players_blast_only_pushes_players() {
        let queues = two_queues();
        let blast = |player_id: u64, by: Option<u64>| minecraftoss_entities::world::PlayerHit { player_id, damage: 12.0, kind: PlayerHitKind::Explosion { knockback: DVec3::new(0.0, 0.2, player_id as f64), by }, source: None };
        let all = vec![blast(HOST, Some(FRIEND)), blast(FRIEND, Some(FRIEND)), blast(FRIEND, Some(77)), blast(HOST, None)];
        let is_player = |id: u64| queues.0.contains_key(&id);
        let (mut kept, mut pushed) = (all.clone(), Vec::new());
        spared_by_blasts_without_pvp(&mut kept, &mut pushed, true, is_player);
        assert_eq!(kept.len(), 4, "pvp on");
        assert!(pushed.is_empty());
        spared_by_blasts_without_pvp(&mut kept, &mut pushed, false, is_player);
        assert_eq!(kept.iter().map(|hit| hit.player_id).collect::<Vec<_>>(), [FRIEND, HOST], "a blast by someone gone, and an ownerless one, still hurt");
        assert_eq!(pushed, [(HOST, DVec3::new(0.0, 0.2, HOST as f64)), (FRIEND, DVec3::new(0.0, 0.2, FRIEND as f64))], "the igniter included");
        let mut queues = two_queues();
        let mut output = minecraft_terrain::server::Output { player_hits: all[..2].to_vec(), ..Default::default() };
        let is_player = |id: u64| id == HOST || id == FRIEND;
        spared_by_blasts_without_pvp(&mut output.player_hits, &mut output.players_pushed, false, is_player);
        queues.route(&mut output, 2, |_| None);
        for player in [HOST, FRIEND] {
            let queue = queues.queue(player);
            assert!(queue.out.hits.is_empty(), "no hearts lost");
            assert_eq!(queue.out.pushes.len(), 1, "pushed once");
        }
    }

    #[test]
    fn with_pvp_off_a_players_harming_potion_spares_every_player_but_other_effects_land() {
        use minecraft_terrain::potions::{PlayerPotion, PlayerPotionHit, PotionEffect};
        let queues = two_queues();
        let hit = |player: u64, potion: PlayerPotion| PlayerPotionHit { player, potion, at: DVec3::ZERO };
        let harm = |by| PlayerPotion::Harm { amount: 6.0, by };
        let poison = PlayerPotion::Effect(PotionEffect::new("minecraft:poison", 100, 0));
        let all = vec![hit(HOST, harm(Some(FRIEND))), hit(FRIEND, harm(Some(FRIEND))), hit(FRIEND, harm(Some(77))), hit(FRIEND, harm(None)), hit(HOST, poison.clone())];
        let is_player = |id: u64| queues.0.contains_key(&id);
        let mut kept = all.clone();
        unharmed_by_potions_without_pvp(&mut kept, true, is_player);
        assert_eq!(kept, all, "pvp on");
        unharmed_by_potions_without_pvp(&mut kept, false, is_player);
        assert_eq!(kept, vec![hit(FRIEND, harm(Some(77))), hit(FRIEND, harm(None)), hit(HOST, poison)], "a witch's potion and a poison still land");
    }

    #[test]
    fn a_spawner_that_refuses_an_egg_tells_only_the_player_who_used_it() {
        let mut queues = two_queues();
        let mut host = Inventory::default();
        let mut friend = Inventory::default();
        host.slots[0] = Some(ItemStack::new("minecraft:pig_spawn_egg", 2));
        friend.slots[0] = Some(ItemStack::new("minecraft:zombie_spawn_egg", 3));
        assert!(queues.queue(HOST).eggs_waiting.send_from(&host, 0, true));
        assert!(queues.queue(FRIEND).eggs_waiting.send_from(&friend, 0, true));
        assert!(queues.queue(FRIEND).eggs_waiting.send_from(&friend, 0, true));
        let mut output = minecraft_terrain::server::Output::default();
        output.summoned.push((FRIEND, Err(minecraft_terrain::spawn_egg::SPAWNER_DISABLED.to_owned())));
        output.summoned.push((HOST, Err("no monster spawner at (4, 205, 4)".to_owned())));
        output.summoned.push((FRIEND, Ok("minecraft:zombie".to_owned())));
        queues.route(&mut output, 2, |_| None);
        settle_eggs(queues.queue(HOST), &mut host);
        settle_eggs(queues.queue(FRIEND), &mut friend);
        let told = std::mem::take(&mut queues.queue(FRIEND).out.told);
        assert_eq!(told.len(), 1, "one line for the refused egg, none for the hatched one");
        let minecraft_commands::Clientbound::System { text, overlay } = &told[0] else {
            panic!("a system line");
        };
        assert!(!overlay, "vanilla's sendSystemMessage goes to the chat");
        let language = std::collections::HashMap::from([("advMode.notEnabled.spawner".to_owned(), "Spawner blocks are not enabled".to_owned())]);
        assert_eq!(text.resolve(&|key| language.get(key).cloned()).text, "Spawner blocks are not enabled");
        assert!(queues.queue(HOST).out.told.is_empty(), "another refusal says nothing in the chat");
        assert_eq!(host.slots[0].as_ref().map(|egg| egg.count), Some(2), "a refused egg is kept");
        assert_eq!(friend.slots[0].as_ref().map(|egg| egg.count), Some(2), "only the hatched egg is spent");
    }
}
