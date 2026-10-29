use std::collections::{BTreeSet, HashMap, HashSet};

use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::movement::Body;
use minecraftoss_entities::piglin::ArmPose;
use minecraftoss_entities::world::EntityWorld;
use serde::{Deserialize, Serialize};

use crate::client_mobs::{ServerMob, server_mobs};

pub const LOOK_TICKS: u64 = 10;
pub const DEFAULT_RANGE: i32 = 5;

const VOLATILE: [&str; 37] = [
    "Pos",
    "Motion",
    "Rotation",
    "OnGround",
    "fall_distance",
    "Fire",
    "Health",
    "Air",
    "AbsorptionAmount",
    "HurtTime",
    "HurtByTimestamp",
    "DeathTime",
    "anger_end_time",
    "Age",
    "InLove",
    "EggLayTime",
    "wasOnGround",
    "TimeInOverworld",
    "Peek",
    "LastRestock",
    "RestocksToday",
    "Gossips",
    "Offers",
    "Xp",
    "LastGossipDecay",
    "FoodLevel",
    "active_effects",
    "Brain",
    "ConversionTime",
    "DespawnDelay",
    "SpellTicks",
    "Warmup",
    "life_ticks",
    "AttackTick",
    "StunTick",
    "RoarTick",
    "Invul",
];

const ARM_POSES: [ArmPose; 6] = [
    ArmPose::Default,
    ArmPose::Dancing,
    ArmPose::AdmiringItem,
    ArmPose::AttackingWithMeleeWeapon,
    ArmPose::CrossbowCharge,
    ArmPose::CrossbowHold,
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MobShow {
    pub fire: bool,
    pub aggressive: bool,
    pub angry: bool,
    pub charging: bool,
    pub converting: bool,
    pub swell: [i32; 2],
    pub peek: [f32; 2],
    pub arm_pose: u8,
    pub using_item: Option<i32>,
    pub sitting: bool,
    pub lying: bool,
    pub relaxed: bool,
    pub crouching: bool,
    pub sprinting: bool,
    pub on_sleeper: bool,
    pub ridden: bool,
    pub invisible: bool,
    pub drink: u8,
    pub pumpkin: bool,
    pub spell: u8,
    pub bite: [i32; 2],
    pub ravager: [i32; 3],
    pub glowing: bool,
    pub wither: WitherShow,
    pub family: Option<crate::families::FamilyShow>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WitherShow {
    pub heads: [f32; 4],
    pub invulnerable: i32,
    pub powered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Watcher {
    pub id: u64,
    pub feet: [f64; 3],
    pub view_distance: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MobEvent {
    Add(u64, Tag),
    Look(u64, Tag),
    Remove(Vec<u64>),
}

#[derive(Clone, Debug, Default)]
pub struct Tracked {
    pub events: Vec<(u64, MobEvent)>,
    pub moves: Vec<(u64, Vec<ServerMob>)>,
}

impl Tracked {
    pub fn merge(&mut self, later: Tracked) {
        self.events.extend(later.events);
        self.moves
            .retain(|(player, _)| later.moves.iter().all(|(newer, _)| newer != player));
        self.moves.extend(later.moves);
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty() && self.moves.is_empty()
    }

    pub fn events_of(&self, player: u64) -> impl Iterator<Item = &MobEvent> {
        self.events
            .iter()
            .filter(move |(to, _)| *to == player)
            .map(|(_, event)| event)
    }

    pub fn moves_of(&self, player: u64) -> Option<&[ServerMob]> {
        self.moves
            .iter()
            .find(|(to, _)| *to == player)
            .map(|(_, moves)| moves.as_slice())
    }
}

#[derive(Default)]
pub struct Trackers {
    players: HashMap<u64, HashMap<u64, Tag>>,
    ticks: u64,
    looked: u64,
}

impl Trackers {
    pub fn tick(&mut self) {
        self.ticks += 1;
    }

    pub fn forget(&mut self, player: u64) {
        self.players.remove(&player);
    }

    pub fn tracks(&self, player: u64, mob: u64) -> bool {
        self.players
            .get(&player)
            .is_some_and(|known| known.contains_key(&mob))
    }

    pub fn update(
        &mut self,
        world: &EntityWorld,
        originals: &HashMap<u64, Tag>,
        watchers: &[Watcher],
    ) -> Tracked {
        let mut out = Tracked::default();
        let left: Vec<u64> = self
            .players
            .keys()
            .copied()
            .filter(|id| watchers.iter().all(|watcher| watcher.id != *id))
            .collect();
        for player in left {
            let Some(known) = self.players.remove(&player) else {
                continue;
            };
            let mut gone: Vec<u64> = known.into_keys().collect();
            gone.sort_unstable();
            if !gone.is_empty() {
                out.events.push((player, MobEvent::Remove(gone)));
            }
        }
        if watchers.is_empty() {
            return out;
        }
        let look_due = self.ticks >= self.looked + LOOK_TICKS;
        if look_due {
            self.looked = self.ticks;
        }
        let mut mobs = server_mobs(world);
        let flying = projectiles(world);
        let flying_ids: HashSet<u64> = flying.iter().map(|mob| mob.id).collect();
        mobs.extend(flying);
        let ranges: HashMap<u64, i32> = mobs
            .iter()
            .map(|mob| {
                let range = world
                    .entity_type(mob.id)
                    .map_or(DEFAULT_RANGE, tracking_range);
                (mob.id, range)
            })
            .collect();
        let mut near_each = Vec::new();
        let mut wanted = HashSet::new();
        for watcher in watchers {
            let known = self.players.entry(watcher.id).or_default();
            let near: BTreeSet<u64> = mobs
                .iter()
                .filter(|mob| !mob.dead || known.contains_key(&mob.id))
                .filter(|mob| {
                    let range = ranges.get(&mob.id).copied().unwrap_or(DEFAULT_RANGE);
                    tracks(range, watcher.view_distance, watcher.feet, mob.position)
                })
                .map(|mob| mob.id)
                .collect();
            wanted.extend(
                near.iter()
                    .copied()
                    .filter(|id| (look_due && !flying_ids.contains(id)) || !known.contains_key(id)),
            );
            near_each.push((watcher.id, near));
        }
        let tags: HashMap<u64, Tag> = if wanted.is_empty() {
            HashMap::new()
        } else {
            crate::server_mobs::mob_tags_where(
                world,
                |id, _| wanted.contains(&id),
                originals,
                false,
            )
            .into_iter()
            .collect()
        };
        let looks: HashMap<u64, Tag> = tags.iter().map(|(&id, tag)| (id, look(tag))).collect();
        for (player, near) in near_each {
            let known = self.players.entry(player).or_default();
            let mut gone: Vec<u64> = known
                .keys()
                .copied()
                .filter(|id| !near.contains(id))
                .collect();
            gone.sort_unstable();
            for id in &gone {
                known.remove(id);
            }
            if !gone.is_empty() {
                out.events.push((player, MobEvent::Remove(gone)));
            }
            for id in near {
                let (Some(tag), Some(seen)) = (tags.get(&id), looks.get(&id)) else {
                    continue;
                };
                match known.get(&id) {
                    None => {
                        known.insert(id, seen.clone());
                        out.events.push((player, MobEvent::Add(id, tag.clone())));
                    }
                    Some(sent) if sent != seen => {
                        known.insert(id, seen.clone());
                        out.events.push((player, MobEvent::Look(id, tag.clone())));
                    }
                    Some(_) => {}
                }
            }
            let moves = mobs
                .iter()
                .filter(|mob| known.contains_key(&mob.id))
                .copied()
                .collect();
            out.moves.push((player, moves));
        }
        out
    }
}

pub fn tracking_range(kind: &str) -> i32 {
    if let Some(range) = crate::families::tracking_range(kind) {
        return range;
    }
    match kind.strip_prefix("minecraft:").unwrap_or(kind) {
        "player" => 32,
        "end_crystal" => 16,
        "cow" | "mooshroom" | "horse" | "donkey" | "sheep" | "pig" | "chicken" | "wolf" | "ocelot"
        | "villager" | "iron_golem" | "ghast" | "slime" | "strider" | "shulker"
        | "ender_dragon" | "tnt" | "falling_block" | "item_frame" | "glow_item_frame"
        | "area_effect_cloud" | "llama" | "trader_llama" | "wandering_trader" | "ravager" | "wither" => 10,
        "zombie" | "husk" | "drowned" | "zombie_villager" | "zombified_piglin" | "skeleton"
        | "stray" | "bogged" | "wither_skeleton" | "parched" | "creeper" | "spider"
        | "cave_spider" | "enderman" | "witch" | "blaze" | "magma_cube" | "hoglin" | "zoglin"
        | "piglin" | "piglin_brute" | "shulker_bullet" | "cat" | "mule" | "snow_golem"
        | "pillager" | "vindicator" | "evoker" | "vex" => 8,
        "ominous_item_spawner" => 8,
        "evoker_fangs" => 6,
        "item" | "experience_orb" => 6,
        "arrow" | "spectral_arrow" | "snowball" | "egg" | "ender_pearl" | "eye_of_ender"
        | "trident" | "fireball" | "small_fireball" | "dragon_fireball" | "splash_potion"
        | "lingering_potion" | "experience_bottle" | "firework_rocket" | "wither_skull"
        | "wind_charge" | "llama_spit" => 4,
        _ => DEFAULT_RANGE,
    }
}

pub fn tracks(range: i32, view_distance: i32, feet: [f64; 3], at: DVec3) -> bool {
    let reach = f64::from(range.min(view_distance).max(0) * 16);
    let (dx, dz) = (at.x - feet[0], at.z - feet[2]);
    dx * dx + dz * dz <= reach * reach
}

pub fn look(tag: &Tag) -> Tag {
    let Tag::Compound(map) = tag else {
        return tag.clone();
    };
    let mut map = map.clone();
    let baby = map
        .get("Age")
        .and_then(Tag::as_i64)
        .is_some_and(|age| age < 0);
    for key in VOLATILE {
        map.remove(key);
    }
    let family = map
        .get("id")
        .and_then(Tag::as_str)
        .and_then(crate::families::volatile)
        .unwrap_or(&[]);
    for key in family {
        map.remove(*key);
    }
    map.insert("Baby".to_owned(), Tag::Byte(i8::from(baby)));
    Tag::Compound(map)
}

pub fn shows(world: &EntityWorld) -> HashMap<u64, MobShow> {
    let mut out = HashMap::new();
    let mut put = |id: u64, show: MobShow| {
        out.insert(id, show);
    };
    for e in world.zombies() {
        put(
            e.id,
            MobShow {
                aggressive: e.aggressive,
                converting: e.zombie.shaking(),
                ..MobShow::default()
            },
        );
    }
    for e in world.skeletons() {
        put(
            e.id,
            MobShow {
                aggressive: e.bow.aggressive,
                ..MobShow::default()
            },
        );
    }
    for e in world.creepers() {
        put(
            e.id,
            MobShow {
                swell: [e.creeper.old_swell, e.creeper.swell],
                ..MobShow::default()
            },
        );
    }
    for e in world.endermen() {
        put(
            e.id,
            MobShow {
                angry: e.creepy(),
                ..MobShow::default()
            },
        );
    }
    for e in world.wolves() {
        put(
            e.id,
            MobShow {
                angry: e.angry(world.game_time()),
                ..MobShow::default()
            },
        );
    }
    for e in world.felines() {
        let f = &e.feline;
        put(
            e.id,
            MobShow {
                sitting: f.sitting,
                lying: f.lying,
                relaxed: f.relax_state_one,
                crouching: f.crouching,
                sprinting: f.sprinting,
                on_sleeper: f.lying_on_sleeping_player,
                ..MobShow::default()
            },
        );
    }
    for e in world.cows() {
        if let Some(horse) = &e.horse {
            put(
                e.id,
                MobShow {
                    ridden: horse.ridden,
                    ..MobShow::default()
                },
            );
        }
    }
    for e in world.wandering_traders() {
        let drink = match e.trader.held {
            Some(minecraftoss_entities::wandering_trader::Drink::Invisibility) => 1,
            Some(minecraftoss_entities::wandering_trader::Drink::Milk) => 2,
            None => 0,
        };
        put(
            e.id,
            MobShow {
                invisible: e.invisible(),
                drink,
                ..MobShow::default()
            },
        );
    }
    for e in world.snow_golems() {
        put(
            e.id,
            MobShow {
                pumpkin: e.golem.pumpkin,
                ..MobShow::default()
            },
        );
    }
    for e in world.illagers() {
        put(
            e.id,
            MobShow {
                aggressive: e.illager.aggressive,
                charging: e.illager.charging_crossbow,
                arm_pose: minecraftoss_entities::illager::arm_pose_index(e.arm_pose),
                using_item: e.using_item,
                spell: e.illager.spell.id(),
                ..MobShow::default()
            },
        );
    }
    for e in world.vexes() {
        put(
            e.id,
            MobShow {
                charging: e.vex.charging,
                ..MobShow::default()
            },
        );
    }
    for e in world.ravagers() {
        let ravager = &e.ravager;
        put(
            e.id,
            MobShow {
                ravager: [ravager.attack_tick, ravager.stunned_tick, ravager.roar_tick],
                ..MobShow::default()
            },
        );
    }
    for e in world.withers() {
        let w = &e.wither;
        put(
            e.id,
            MobShow {
                wither: WitherShow {
                    heads: [w.head_x_rots[0], w.head_y_rots[0], w.head_x_rots[1], w.head_y_rots[1]],
                    invulnerable: w.invulnerable_ticks,
                    powered: w.powered(),
                },
                ..MobShow::default()
            },
        );
    }
    for e in world.ghasts() {
        put(
            e.id,
            MobShow {
                charging: e.ghast.charging,
                ..MobShow::default()
            },
        );
    }
    for e in world.shulkers() {
        put(
            e.id,
            MobShow {
                peek: [e.shulker.peek_o, e.shulker.peek],
                ..MobShow::default()
            },
        );
    }
    for e in world.hoglins() {
        put(
            e.id,
            MobShow {
                converting: e.hoglin.converting,
                ..MobShow::default()
            },
        );
    }
    for e in world.piglins() {
        let arm_pose = ARM_POSES
            .iter()
            .position(|pose| *pose == e.arm_pose)
            .unwrap_or(0);
        put(
            e.id,
            MobShow {
                aggressive: e.piglin.aggressive,
                converting: e.piglin.converting,
                arm_pose: arm_pose as u8,
                using_item: e.using_item,
                ..MobShow::default()
            },
        );
    }
    crate::families::shows(world, &mut put);
    for raider in world.raiders() {
        if world.glowing(raider.id) {
            out.entry(raider.id).or_default().glowing = true;
        }
    }
    out
}

pub fn projectiles(world: &EntityWorld) -> Vec<ServerMob> {
    let mut out = Vec::new();
    let mut push = |id: u64, position: DVec3, turn: (f32, f32), size: f64| {
        out.push(ServerMob {
            id,
            position,
            y_rot: turn.0,
            x_rot: turn.1,
            y_head_rot: turn.0,
            on_ground: false,
            baby: false,
            eye_height: 0.0,
            max_head_y_rot: 0.0,
            update_interval: 1,
            sync: Default::default(),
            resting: false,
            hurts: 0,
            dead: false,
            swings: 0,
            size: (size as f32, size as f32),
            vehicle: None,
            show: MobShow::default(),
        });
    };
    for e in world.fireballs().iter().filter(|e| e.fireball.alive) {
        let size = e.fireball.kind.size();
        push(e.id, e.fireball.position, (e.fireball.yaw, e.fireball.pitch), size);
    }
    for e in world.shulker_bullets().iter().filter(|e| e.bullet.alive) {
        let turn = (e.bullet.yaw, e.bullet.pitch);
        push(e.id, e.bullet.position, turn, 0.3125);
    }
    for e in world.potions().iter().filter(|e| e.potion.alive) {
        push(e.id, e.potion.position, (0.0, 0.0), 0.25);
    }
    for e in world.llama_spits().iter().filter(|e| e.spit.alive) {
        push(e.id, e.spit.position, (e.spit.yaw, e.spit.pitch), minecraftoss_entities::llama_spit::SIZE);
    }
    for e in world.evoker_fangs().iter().filter(|e| e.fangs.alive) {
        push(e.id, e.fangs.position, (e.fangs.yaw, 0.0), minecraftoss_entities::evoker_fangs::WIDTH);
    }
    for mob in &mut out {
        if let Some(e) = world.evoker_fangs().iter().find(|e| e.id == mob.id) {
            mob.show.bite = [e.fangs.warmup, e.fangs.life_ticks];
        }
    }
    out
}

pub fn is_projectile(world: &EntityWorld, id: u64) -> bool {
    world.fireballs().iter().any(|e| e.id == id)
        || world.shulker_bullets().iter().any(|e| e.id == id)
        || world.potions().iter().any(|e| e.id == id)
        || world.llama_spits().iter().any(|e| e.id == id)
        || world.evoker_fangs().iter().any(|e| e.id == id)
}

pub(crate) fn place(previous: &mut DVec3, body: &mut Body, mob: &ServerMob) {
    *previous = body.position;
    body.position = mob.position;
    body.on_ground = mob.on_ground;
    body.fire_ticks = i32::from(mob.show.fire);
}

pub fn show_on(world: &mut EntityWorld, mob: &ServerMob) {
    let (id, show) = (mob.id, mob.show);
    if let Some(e) = world.cow_mut(id) {
        place(&mut e.previous_position, &mut e.cow.body, mob);
        if let Some(horse) = e.horse.as_mut() {
            horse.ridden = show.ridden;
        }
    }
    if let Some(e) = world.sheep_mut(id) {
        place(&mut e.previous_position, &mut e.body, mob);
    }
    if let Some(e) = world.pig_mut(id) {
        place(&mut e.previous_position, &mut e.pig.body, mob);
    }
    if let Some(e) = world.chicken_mut(id) {
        place(&mut e.previous_position, &mut e.chicken.body, mob);
    }
    if let Some(e) = world.bat_mut(id) {
        place(&mut e.previous_position, &mut e.bat.body, mob);
        e.bat.resting = mob.resting;
    }
    if let Some(e) = world.zombie_mut(id) {
        place(&mut e.previous_position, &mut e.zombie.body, mob);
        e.aggressive = show.aggressive;
        if e.zombie.shaking() != show.converting {
            if e.zombie.kind == minecraftoss_entities::zombie::ZombieKind::ZombieVillager {
                e.zombie.villager_conversion_time = if show.converting { 0 } else { -1 };
            } else {
                e.zombie.underwater_converting = show.converting;
            }
        }
    }
    if let Some(e) = world.skeleton_mut(id) {
        place(&mut e.previous_position, &mut e.skeleton.body, mob);
        e.bow.aggressive = show.aggressive;
    }
    if let Some(e) = world.spider_mut(id) {
        place(&mut e.previous_position, &mut e.spider.body, mob);
    }
    if let Some(e) = world.creeper_mut(id) {
        place(&mut e.previous_position, &mut e.creeper.body, mob);
        [e.creeper.old_swell, e.creeper.swell] = show.swell;
    }
    if let Some(e) = world.villager_mut(id) {
        place(&mut e.previous_position, &mut e.villager.body, mob);
    }
    if let Some(e) = world.slime_mut(id) {
        place(&mut e.previous_position, &mut e.slime.body, mob);
    }
    if let Some(e) = world.enderman_mut(id) {
        place(&mut e.previous_position, &mut e.enderman.body, mob);
        e.ai.state.enderman.creepy = show.angry;
    }
    if let Some(e) = world.witch_mut(id) {
        place(&mut e.previous_position, &mut e.witch.body, mob);
    }
    if let Some(e) = world.iron_golem_mut(id) {
        place(&mut e.previous_position, &mut e.golem.body, mob);
    }
    if let Some(e) = world.wolf_mut(id) {
        place(&mut e.previous_position, &mut e.wolf.body, mob);
        e.ai.state.enderman.anger_end_time = if show.angry { i64::MAX } else { i64::MIN };
    }
    if let Some(e) = world.feline_mut(id) {
        place(&mut e.previous_position, &mut e.feline.body, mob);
        let f = &mut e.feline;
        f.sitting = show.sitting;
        f.lying = show.lying;
        f.relax_state_one = show.relaxed;
        f.crouching = show.crouching;
        f.sprinting = show.sprinting;
        f.lying_on_sleeping_player = show.on_sleeper;
        f.update_lie_down_amount();
        f.update_relax_state_one_amount();
    }
    if let Some(e) = world.wandering_trader_mut(id) {
        use minecraftoss_entities::effects::{EffectInstance, MobEffect};
        use minecraftoss_entities::wandering_trader::Drink;
        place(&mut e.previous_position, &mut e.trader.body, mob);
        if show.invisible {
            e.effects.add(EffectInstance::new(MobEffect::Invisibility, -1, 0));
        } else {
            e.effects.remove(MobEffect::Invisibility);
        }
        e.trader.held = match show.drink {
            1 => Some(Drink::Invisibility),
            2 => Some(Drink::Milk),
            _ => None,
        };
    }
    if let Some(e) = world.snow_golem_mut(id) {
        place(&mut e.previous_position, &mut e.golem.body, mob);
        e.golem.pumpkin = show.pumpkin;
    }
    if let Some(e) = world.illager_mut(id) {
        place(&mut e.previous_position, &mut e.illager.body, mob);
        e.illager.aggressive = show.aggressive;
        e.illager.charging_crossbow = show.charging;
        e.arm_pose = minecraftoss_entities::illager::arm_pose_of(show.arm_pose);
        e.using_item = show.using_item;
        e.illager.spell = minecraftoss_entities::illager::IllagerSpell::from_id(show.spell);
    }
    if let Some(e) = world.vex_mut(id) {
        place(&mut e.previous_position, &mut e.vex.body, mob);
        e.vex.charging = show.charging;
    }
    if let Some(e) = world.ravager_mut(id) {
        place(&mut e.previous_position, &mut e.ravager.body, mob);
        let [attack, stunned, roar] = show.ravager;
        e.ravager.attack_tick = attack;
        e.ravager.stunned_tick = stunned;
        e.ravager.roar_tick = roar;
    }
    if let Some(e) = world.wither_mut(id) {
        use minecraftoss_entities::wither::MAX_HEALTH;
        place(&mut e.previous_position, &mut e.wither.body, mob);
        let [x0, y0, x1, y1] = show.wither.heads;
        e.wither.head_x_rots = [x0, x1];
        e.wither.head_y_rots = [y0, y1];
        e.wither.invulnerable_ticks = show.wither.invulnerable;
        if show.wither.powered != e.wither.powered() {
            e.wither.health = if show.wither.powered { MAX_HEALTH / 2.0 } else { MAX_HEALTH };
        }
    }
    if let Some(e) = world.ghast_mut(id) {
        place(&mut e.previous_position, &mut e.ghast.body, mob);
        e.ghast.charging = show.charging;
    }
    if let Some(e) = world.blaze_mut(id) {
        place(&mut e.previous_position, &mut e.blaze.body, mob);
    }
    if let Some(e) = world.strider_mut(id) {
        place(&mut e.previous_position, &mut e.strider.body, mob);
    }
    if let Some(e) = world.hoglin_mut(id) {
        place(&mut e.previous_position, &mut e.hoglin.body, mob);
        e.hoglin.converting = show.converting;
    }
    if let Some(e) = world.piglin_mut(id) {
        place(&mut e.previous_position, &mut e.piglin.body, mob);
        e.piglin.aggressive = show.aggressive;
        e.piglin.converting = show.converting;
        e.arm_pose = ARM_POSES
            .get(usize::from(show.arm_pose))
            .copied()
            .unwrap_or(ArmPose::Default);
        e.using_item = show.using_item;
    }
    if let Some(e) = world.shulker_mut(id) {
        place(&mut e.previous_position, &mut e.shulker.body, mob);
        [e.shulker.peek_o, e.shulker.peek] = show.peek;
    }
    if let Some(e) = world.fireball_mut(id) {
        e.previous_position = e.fireball.position;
        e.fireball.position = mob.position;
        e.fireball.yaw = mob.y_rot;
        e.fireball.pitch = mob.x_rot;
    }
    if let Some(e) = world.shulker_bullet_mut(id) {
        e.previous_position = e.bullet.position;
        e.bullet.position = mob.position;
        e.bullet.yaw_o = e.bullet.yaw;
        e.bullet.pitch_o = e.bullet.pitch;
        e.bullet.yaw = mob.y_rot;
        e.bullet.pitch = mob.x_rot;
        e.bullet.tick_count += 1;
    }
    if let Some(e) = world.potion_mut(id) {
        e.previous_position = e.potion.position;
        e.potion.position = mob.position;
    }
    if let Some(e) = world.evoker_fangs_mut(id) {
        e.fangs.position = mob.position;
        e.fangs.yaw = mob.y_rot;
        [e.fangs.warmup, e.fangs.life_ticks] = show.bite;
        e.fangs.sent_spike = show.bite[0] < 0;
    }
    if let Some(e) = world.llama_spit_mut(id) {
        e.previous_position = e.spit.position;
        e.spit.position = mob.position;
        e.spit.yaw_o = e.spit.yaw;
        e.spit.pitch_o = e.spit.pitch;
        e.spit.yaw = mob.y_rot;
        e.spit.pitch = mob.x_rot;
    }
    crate::families::show_on(world, mob);
    if show.glowing != world.glowing(id) {
        use minecraftoss_entities::effects::{EffectInstance, MobEffect};
        if show.glowing {
            world.add_mob_effect(id, EffectInstance::new(MobEffect::Glowing, -1, 0));
        } else if let Some((effects, _, _)) = world.mob_effects_mut(id) {
            effects.remove(MobEffect::Glowing);
        }
    }
}

pub fn spawn_tracked(world: &mut EntityWorld, id: u64, tag: &Tag) -> bool {
    if id == 0 {
        return false;
    }
    world.remove_mobs(&[id]);
    world.set_next_entity_id(id);
    match crate::server_mobs::spawn_saved(world, tag) {
        Some(spawned) if spawned == id => true,
        Some(other) => {
            world.remove_mobs(&[other]);
            false
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).expect("the tag")
    }

    fn spawn(world: &mut EntityWorld, text: &str) -> (u64, Tag) {
        let tag = saved(text);
        let id = crate::server_mobs::spawn_saved(world, &tag).expect("the mob spawns");
        (id, tag)
    }

    fn watcher(id: u64, x: f64) -> Watcher {
        Watcher {
            id,
            feet: [x, 64.0, 0.5],
            view_distance: 8,
        }
    }

    fn update(trackers: &mut Trackers, world: &EntityWorld, watchers: &[Watcher]) -> Tracked {
        trackers.tick();
        trackers.update(world, &HashMap::new(), watchers)
    }

    #[test]
    fn the_tracking_range_is_vanillas_per_kind_and_the_view_distance_caps_it() {
        assert_eq!(tracking_range("minecraft:zombie"), 8);
        assert_eq!(tracking_range("minecraft:magma_cube"), 8);
        assert_eq!(tracking_range("minecraft:cow"), 10);
        assert_eq!(tracking_range("minecraft:slime"), 10);
        assert_eq!(tracking_range("minecraft:bat"), 5);
        assert_eq!(tracking_range("minecraft:end_crystal"), 16);
        assert_eq!(tracking_range("minecraft:player"), 32);
        assert_eq!(tracking_range("minecraft:item"), 6);
        assert_eq!(tracking_range("minecraft:snowball"), 4);
        assert_eq!(tracking_range("minecraft:not_a_mob"), DEFAULT_RANGE);
        let feet = [0.0, 64.0, 0.0];
        assert!(tracks(8, 8, feet, DVec3::new(128.0, 64.0, 0.0)));
        assert!(!tracks(8, 8, feet, DVec3::new(128.1, 64.0, 0.0)));
        assert!(tracks(8, 8, feet, DVec3::new(90.0, 300.0, 90.0)));
        assert!(!tracks(10, 8, feet, DVec3::new(140.0, 64.0, 0.0)));
        assert!(tracks(10, 12, feet, DVec3::new(140.0, 64.0, 0.0)));
        assert!(!tracks(5, 8, feet, DVec3::new(0.0, 64.0, 81.0)));
    }

    #[test]
    fn a_mob_is_added_moved_and_removed_as_a_watcher_comes_and_goes() {
        let mut world = EntityWorld::default();
        let (cow, _) = spawn(
            &mut world,
            "{id:\"minecraft:cow\",Pos:[100.5d,64.0d,0.5d],NoAI:1b}",
        );
        let (bat, _) = spawn(
            &mut world,
            "{id:\"minecraft:bat\",Pos:[100.5d,70.0d,0.5d],NoAI:1b}",
        );
        let mut trackers = Trackers::default();
        let first = update(&mut trackers, &world, &[watcher(5, 0.5)]);
        assert_eq!(first.events.len(), 1, "{:?}", first.events);
        let MobEvent::Add(added, tag) = &first.events[0].1 else {
            panic!("{:?}", first.events);
        };
        assert_eq!((first.events[0].0, *added), (5, cow));
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some("minecraft:cow"));
        let moves: Vec<u64> = first
            .moves_of(5)
            .unwrap()
            .iter()
            .map(|mob| mob.id)
            .collect();
        assert_eq!(moves, vec![cow], "the bat is beyond its 5 chunks");
        assert!(trackers.tracks(5, cow));
        let second = update(&mut trackers, &world, &[watcher(5, 0.5)]);
        assert!(second.events.is_empty());
        assert_eq!(second.moves_of(5).map(<[ServerMob]>::len), Some(1));
        let near = update(&mut trackers, &world, &[watcher(5, 60.5)]);
        assert_eq!(near.events.len(), 1);
        assert!(matches!(&near.events[0].1, MobEvent::Add(id, _) if *id == bat));
        let gone = update(&mut trackers, &world, &[watcher(5, 400.5)]);
        let MobEvent::Remove(ids) = &gone.events[0].1 else {
            panic!("{:?}", gone.events);
        };
        let mut both = vec![cow, bat];
        both.sort_unstable();
        assert_eq!(ids, &both);
        assert_eq!(gone.moves_of(5).map(<[ServerMob]>::len), Some(0));
        let left = update(&mut trackers, &world, &[]);
        assert!(left.is_empty());
        assert!(!trackers.tracks(5, cow));
    }

    #[test]
    fn a_watcher_who_stops_watching_is_told_to_remove_what_they_had() {
        let mut world = EntityWorld::default();
        let (cow, _) = spawn(
            &mut world,
            "{id:\"minecraft:cow\",Pos:[4.5d,64.0d,0.5d],NoAI:1b}",
        );
        let mut trackers = Trackers::default();
        let both = [watcher(5, 0.5), watcher(6, 0.5)];
        assert_eq!(update(&mut trackers, &world, &both).events.len(), 2);
        let after = update(&mut trackers, &world, &both[..1]);
        assert_eq!(after.events.len(), 1, "{:?}", after.events);
        assert_eq!(after.events[0], (6, MobEvent::Remove(vec![cow])));
        assert!(trackers.tracks(5, cow) && !trackers.tracks(6, cow));
        let back = update(&mut trackers, &world, &both);
        assert!(matches!(&back.events[..], [(6, MobEvent::Add(id, _))] if *id == cow));
    }

    #[test]
    fn projectiles_are_tracked_moved_and_never_resent_as_looks() {
        let mut host = EntityWorld::default();
        let (fireball, _) = spawn(
            &mut host,
            "{id:\"minecraft:small_fireball\",Pos:[3.5d,66.0d,0.5d],Motion:[0.5d,0.0d,0.0d]}",
        );
        let (bullet, _) = spawn(
            &mut host,
            "{id:\"minecraft:shulker_bullet\",Pos:[5.5d,66.0d,0.5d],Motion:[0.0d,0.1d,0.0d]}",
        );
        let (far, _) = spawn(
            &mut host,
            "{id:\"minecraft:small_fireball\",Pos:[90.5d,66.0d,0.5d],Motion:[0.5d,0.0d,0.0d]}",
        );
        let mut trackers = Trackers::default();
        let watchers = [watcher(2, 0.5)];
        let first = update(&mut trackers, &host, &watchers);
        let mut added: Vec<u64> = first
            .events_of(2)
            .map(|event| match event {
                MobEvent::Add(id, _) => *id,
                other => panic!("{other:?}"),
            })
            .collect();
        added.sort_unstable();
        assert_eq!(added, vec![fireball, bullet], "a fireball goes 4 chunks");
        assert!(!trackers.tracks(2, far));
        let mut friend = EntityWorld::default();
        for event in first.events_of(2) {
            let MobEvent::Add(id, tag) = event else {
                unreachable!();
            };
            assert!(spawn_tracked(&mut friend, *id, tag), "{tag:?}");
        }
        host.fireball_mut(fireball).unwrap().fireball.position = DVec3::new(4.0, 66.0, 0.5);
        host.shulker_bullet_mut(bullet).unwrap().bullet.yaw = 90.0;
        let mut later = Vec::new();
        for _ in 0..LOOK_TICKS * 2 {
            let tracked = update(&mut trackers, &host, &watchers);
            for mob in tracked.moves_of(2).unwrap() {
                show_on(&mut friend, mob);
            }
            later.extend(tracked.events);
        }
        assert!(later.is_empty(), "{later:?}");
        let shown = &friend.fireballs()[0];
        assert_eq!(shown.fireball.position, DVec3::new(4.0, 66.0, 0.5));
        assert_eq!(shown.previous_position, DVec3::new(4.0, 66.0, 0.5));
        assert!(is_projectile(&friend, fireball) && is_projectile(&friend, bullet));
        assert_eq!(friend.shulker_bullets()[0].bullet.yaw, 90.0);
        host.remove_mobs(&[fireball]);
        let gone = update(&mut trackers, &host, &watchers);
        assert_eq!(gone.events, vec![(2, MobEvent::Remove(vec![fireball]))]);
        friend.remove_mobs(&[fireball]);
        assert!(friend.fireballs().is_empty());
    }

    #[test]
    fn a_look_change_is_sent_once_and_a_health_change_is_not() {
        let mut world = EntityWorld::default();
        let (pig, _) = spawn(
            &mut world,
            "{id:\"minecraft:pig\",Pos:[10.5d,64.0d,0.5d],NoAI:1b}",
        );
        let mut trackers = Trackers::default();
        let watchers = [watcher(3, 0.5)];
        assert_eq!(update(&mut trackers, &world, &watchers).events.len(), 1);
        world.pig_mut(pig).unwrap().pig.health -= 3.0;
        for _ in 0..LOOK_TICKS {
            assert!(update(&mut trackers, &world, &watchers).events.is_empty());
        }
        world.pig_mut(pig).unwrap().pig.saddled = true;
        let mut looks = Vec::new();
        for _ in 0..LOOK_TICKS * 3 {
            looks.extend(update(&mut trackers, &world, &watchers).events);
        }
        assert_eq!(looks.len(), 1, "only the llama looks new, not the trader llama's clock: {looks:?}");
        let MobEvent::Look(id, tag) = &looks[0].1 else {
            panic!("{looks:?}");
        };
        assert_eq!(*id, pig);
        let saddle = tag
            .get("equipment")
            .and_then(|equipment| equipment.get("saddle"))
            .and_then(|saddle| saddle.get("id"))
            .and_then(Tag::as_str);
        assert_eq!(saddle, Some("minecraft:saddle"));
    }

    #[test]
    fn a_trade_changes_no_look_but_a_new_level_does() {
        let before = saved(
            "{id:\"minecraft:villager\",Xp:0,Offers:{Recipes:[{buy:{id:\"minecraft:wheat\",count:20},sell:{id:\"minecraft:emerald\",count:1}}]},VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:1}}",
        );
        let traded = saved(
            "{id:\"minecraft:villager\",Xp:2,Offers:{Recipes:[{buy:{id:\"minecraft:wheat\",count:20},sell:{id:\"minecraft:emerald\",count:1},uses:1}]},VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:1}}",
        );
        assert_eq!(look(&before), look(&traded));
        let levelled = saved(
            "{id:\"minecraft:villager\",Xp:10,VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:2}}",
        );
        assert_ne!(look(&before), look(&levelled), "the badge shows the level");
    }

    #[test]
    fn an_added_tag_keeps_equipment_profession_and_the_hosts_id() {
        let mut world = EntityWorld::default();
        world.set_next_entity_id(4000);
        let villager_text = "{id:\"minecraft:villager\",Pos:[4.5d,64.0d,0.5d],NoAI:1b,VillagerData:{type:\"minecraft:desert\",profession:\"minecraft:farmer\",level:3}}";
        let zombie_text = "{id:\"minecraft:zombie\",Pos:[6.5d,64.0d,0.5d],NoAI:1b,equipment:{mainhand:{id:\"minecraft:iron_sword\",count:1}}}";
        let (villager, villager_tag) = spawn(&mut world, villager_text);
        let (zombie, zombie_tag) = spawn(&mut world, zombie_text);
        let originals: HashMap<u64, Tag> = [(villager, villager_tag), (zombie, zombie_tag)]
            .into_iter()
            .collect();
        let mut trackers = Trackers::default();
        trackers.tick();
        let tracked = trackers.update(&world, &originals, &[watcher(9, 0.5)]);
        assert_eq!(tracked.events.len(), 2);
        let mut friend = EntityWorld::default();
        for (_, event) in &tracked.events {
            let MobEvent::Add(id, tag) = event else {
                panic!("{event:?}");
            };
            assert!(spawn_tracked(&mut friend, *id, tag), "{tag:?}");
        }
        let shown = &friend.villagers()[0];
        assert_eq!(shown.id, villager);
        assert_eq!(shown.villager.profession.id(), "minecraft:farmer");
        assert_eq!(shown.villager.level, 3);
        assert_eq!(shown.villager.kind, "minecraft:desert");
        let shown = &friend.zombies()[0];
        assert_eq!(shown.id, zombie);
        assert_eq!(
            shown.zombie.main_hand.as_deref(),
            Some("minecraft:iron_sword")
        );
        assert!(spawn_tracked(&mut friend, zombie, &saved(zombie_text)));
        assert_eq!(friend.zombies().len(), 1, "a second add replaces the mob");
    }

    #[test]
    fn a_friend_sees_a_drowned() {
        let mut host = EntityWorld::default();
        host.set_next_entity_id(700);
        let (drowned, tag) = spawn(
            &mut host,
            "{id:\"minecraft:drowned\",Pos:[3.5d,64.0d,0.5d],Health:9.0f,IsBaby:1b,equipment:{mainhand:{id:\"minecraft:trident\",count:1}}}",
        );
        assert_eq!(tracking_range("minecraft:drowned"), 8);
        let mut trackers = Trackers::default();
        trackers.tick();
        let originals: HashMap<u64, Tag> = [(drowned, tag)].into_iter().collect();
        let tracked = trackers.update(&host, &originals, &[watcher(3, 0.5)]);
        let added: Vec<(u64, Tag)> = tracked
            .events_of(3)
            .map(|event| match event {
                MobEvent::Add(id, tag) => (*id, tag.clone()),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(added.len(), 1);
        let (id, sent) = &added[0];
        assert_eq!(*id, drowned);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, drowned, sent));
        assert_eq!(friend.entity_type(drowned), Some("minecraft:drowned"));
        let shown = friend.zombie_mut(drowned).unwrap();
        assert!(shown.zombie.baby);
        assert_eq!(shown.zombie.main_hand.as_deref(), Some("minecraft:trident"));
        assert_eq!(shown.zombie.body.step_height, 1.0);
    }

    #[test]
    fn a_friend_sees_a_cat_and_an_ocelot_from_their_look_tags() {
        let mut host = EntityWorld::default();
        host.set_next_entity_id(900);
        let (cat, cat_tag) = spawn(
            &mut host,
            "{id:\"minecraft:cat\",Pos:[3.5d,64.0d,0.5d],variant:\"minecraft:calico\",sound_variant:\"minecraft:royal\",CollarColor:11b,Owner:[I;1,2,3,4],Sitting:1b,Age:-24000}",
        );
        let (ocelot, ocelot_tag) = spawn(&mut host, "{id:\"minecraft:ocelot\",Pos:[5.5d,64.0d,0.5d],Trusting:1b}");
        assert_eq!((tracking_range("minecraft:cat"), tracking_range("minecraft:ocelot")), (8, 10));
        let mut trackers = Trackers::default();
        trackers.tick();
        let originals: HashMap<u64, Tag> = [(cat, cat_tag), (ocelot, ocelot_tag)].into_iter().collect();
        let tracked = trackers.update(&host, &originals, &[watcher(3, 0.5)]);
        let mut friend = EntityWorld::default();
        for event in tracked.events_of(3) {
            let MobEvent::Add(id, sent) = event else { panic!("{event:?}") };
            assert!(spawn_tracked(&mut friend, *id, sent));
        }
        let shown = &friend.feline_mut(cat).unwrap().feline;
        assert_eq!((shown.variant.as_str(), shown.sound_variant.as_str(), shown.collar), ("calico", "royal", 11));
        assert!(shown.tame && shown.ordered_to_sit && shown.sitting && shown.baby() && shown.shows_collar());
        assert!(friend.feline_mut(ocelot).unwrap().feline.trusting);
        {
            let f = &mut host.feline_mut(ocelot).unwrap().feline;
            f.crouching = true;
        }
        {
            let f = &mut host.feline_mut(cat).unwrap().feline;
            f.lying = true;
            f.relax_state_one = true;
            f.sitting = false;
            f.lying_on_sleeping_player = true;
        }
        let seen = shows(&host);
        assert!(seen[&cat].lying && seen[&cat].relaxed && !seen[&cat].sitting && seen[&ocelot].crouching);
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        let shown = &friend.feline_mut(cat).unwrap().feline;
        assert!(shown.lying && shown.relax_state_one && !shown.sitting && shown.lie_down_amount > 0.0);
        assert!(shown.lying_on_sleeping_player);
        assert!(friend.feline_mut(ocelot).unwrap().feline.crouching);
    }

    #[test]
    fn a_friend_sees_mules_and_llamas_from_their_look_tags_and_their_new_chests_and_carpets() {
        let mut host = EntityWorld::default();
        host.set_next_entity_id(700);
        let (mule, mule_tag) = spawn(&mut host, "{id:\"minecraft:mule\",Pos:[2.5d,64.0d,0.5d],ChestedHorse:1b,Age:-24000}");
        let (llama, llama_tag) = spawn(
            &mut host,
            "{id:\"minecraft:llama\",Pos:[5.5d,64.0d,0.5d],Variant:3,Strength:2,Tame:1b,equipment:{body:{id:\"minecraft:white_carpet\",count:1}}}",
        );
        let (trader, trader_tag) = spawn(&mut host, "{id:\"minecraft:trader_llama\",Pos:[8.5d,64.0d,0.5d],Variant:1}");
        assert_eq!(
            (tracking_range("minecraft:mule"), tracking_range("minecraft:llama"), tracking_range("minecraft:trader_llama"), tracking_range("minecraft:llama_spit")),
            (8, 10, 10, 4)
        );
        let mut trackers = Trackers::default();
        trackers.tick();
        let originals: HashMap<u64, Tag> = [(mule, mule_tag), (llama, llama_tag), (trader, trader_tag)].into_iter().collect();
        let tracked = trackers.update(&host, &originals, &[watcher(3, 0.5)]);
        let mut friend = EntityWorld::default();
        for event in tracked.events_of(3) {
            let MobEvent::Add(id, sent) = event else { panic!("{event:?}") };
            assert!(spawn_tracked(&mut friend, *id, sent));
        }
        let horse = |world: &mut EntityWorld, id: u64| world.cow_mut(id).unwrap().horse.clone().unwrap();
        let m = horse(&mut friend, mule);
        assert!(m.chest && friend.cow_mut(mule).unwrap().cow.age.baby());
        let l = horse(&mut friend, llama);
        assert_eq!((l.variant, l.strength, l.body_item.as_deref(), l.chest), (3, 2, Some("minecraft:white_carpet"), false));
        assert_eq!(horse(&mut friend, trader).kind, minecraftoss_entities::horse::HorseKind::TraderLlama);
        {
            let h = host.cow_mut(llama).unwrap().horse.as_mut().unwrap();
            h.chest = true;
            h.body_item = Some("minecraft:black_carpet".to_owned());
        }
        host.cow_mut(trader).unwrap().horse.as_mut().unwrap().despawn_delay -= 5;
        let mut looks = Vec::new();
        for _ in 0..LOOK_TICKS {
            trackers.tick();
            looks.extend(trackers.update(&host, &originals, &[watcher(3, 0.5)]).events);
        }
        assert_eq!(looks.len(), 1, "only the llama looks new, not the trader llama's clock: {looks:?}");
        let (_, MobEvent::Look(id, sent)) = &looks[0] else { panic!("{looks:?}") };
        assert!(spawn_tracked(&mut friend, *id, sent));
        let l = horse(&mut friend, llama);
        assert_eq!((l.chest, l.body_item.as_deref()), (true, Some("minecraft:black_carpet")));
    }

    #[test]
    fn a_friend_sees_a_horse_get_its_saddle_and_armour_and_a_rider() {
        let mut host = EntityWorld::default();
        host.set_next_entity_id(800);
        let (horse, horse_tag) = spawn(&mut host, "{id:\"minecraft:horse\",Pos:[2.5d,64.0d,0.5d],Tame:1b}");
        let mut trackers = Trackers::default();
        trackers.tick();
        let originals: HashMap<u64, Tag> = [(horse, horse_tag)].into_iter().collect();
        let tracked = trackers.update(&host, &originals, &[watcher(3, 0.5)]);
        let mut friend = EntityWorld::default();
        for event in tracked.events_of(3) {
            let MobEvent::Add(id, sent) = event else { panic!("{event:?}") };
            assert!(spawn_tracked(&mut friend, *id, sent));
        }
        assert!(!friend.cow_mut(horse).unwrap().horse.as_ref().unwrap().saddled);
        {
            let h = host.cow_mut(horse).unwrap().horse.as_mut().unwrap();
            h.saddled = true;
            h.body_item = Some("minecraft:golden_horse_armor".to_owned());
        }
        let mut looks = Vec::new();
        for _ in 0..LOOK_TICKS {
            trackers.tick();
            looks.extend(trackers.update(&host, &originals, &[watcher(3, 0.5)]).events);
        }
        let (_, MobEvent::Look(id, sent)) = &looks[0] else { panic!("{looks:?}") };
        assert!(spawn_tracked(&mut friend, *id, sent));
        let h = friend.cow_mut(horse).unwrap().horse.clone().unwrap();
        assert!(h.saddled && h.body_item.as_deref() == Some("minecraft:golden_horse_armor"));
        host.cow_mut(horse).unwrap().horse.as_mut().unwrap().ridden = true;
        let show = shows(&host)[&horse];
        assert!(show.ridden);
        let moved = crate::client_mobs::server_mobs(&host).into_iter().find(|m| m.id == horse).map(|m| ServerMob { show, ..m }).unwrap();
        show_on(&mut friend, &moved);
        assert!(friend.cow_mut(horse).unwrap().horse.as_ref().unwrap().ridden, "the reins show while someone rides it");
    }

    #[test]
    fn a_friend_sees_a_horse_s_dyed_armour_and_its_new_dye() {
        let mut host = EntityWorld::default();
        host.set_next_entity_id(850);
        let (horse, horse_tag) = spawn(&mut host, "{id:\"minecraft:horse\",Pos:[2.5d,64.0d,0.5d],Tame:1b,equipment:{body:{id:\"minecraft:leather_horse_armor\",count:1,components:{\"minecraft:dyed_color\":11546150}}}}");
        let mut trackers = Trackers::default();
        trackers.tick();
        let originals: HashMap<u64, Tag> = [(horse, horse_tag)].into_iter().collect();
        let tracked = trackers.update(&host, &originals, &[watcher(3, 0.5)]);
        let mut friend = EntityWorld::default();
        for event in tracked.events_of(3) {
            let MobEvent::Add(id, sent) = event else { panic!("{event:?}") };
            assert!(spawn_tracked(&mut friend, *id, sent));
        }
        let worn = |world: &mut EntityWorld| world.cow_mut(horse).unwrap().horse.as_ref().unwrap().body_components.clone();
        assert_eq!(worn(&mut friend), Some(serde_json::json!({"minecraft:dyed_color": 0xB02E26})));
        host.cow_mut(horse).unwrap().horse.as_mut().unwrap().body_components = Some(serde_json::json!({"minecraft:dyed_color": 0x80C71F}));
        let mut looks = Vec::new();
        for _ in 0..LOOK_TICKS {
            trackers.tick();
            looks.extend(trackers.update(&host, &originals, &[watcher(3, 0.5)]).events);
        }
        assert_eq!(looks.len(), 1, "a new dye is a new look: {looks:?}");
        let (_, MobEvent::Look(id, sent)) = &looks[0] else { panic!("{looks:?}") };
        assert!(spawn_tracked(&mut friend, *id, sent));
        assert_eq!(worn(&mut friend), Some(serde_json::json!({"minecraft:dyed_color": 0x80C71F})));
    }

    #[test]
    fn a_friend_sees_llama_spit_fly() {
        let mut host = EntityWorld::default();
        let (spit, _) = spawn(&mut host, "{id:\"minecraft:llama_spit\",Pos:[3.5d,66.0d,0.5d],Motion:[0.0d,0.0d,1.5d],Rotation:[0.0f,0.0f]}");
        let mut trackers = Trackers::default();
        let watchers = [watcher(2, 0.5)];
        let first = update(&mut trackers, &host, &watchers);
        let mut friend = EntityWorld::default();
        for event in first.events_of(2) {
            let MobEvent::Add(id, tag) = event else { panic!("{event:?}") };
            assert!(spawn_tracked(&mut friend, *id, tag));
        }
        assert!(is_projectile(&friend, spit));
        {
            let e = host.llama_spit_mut(spit).unwrap();
            e.spit.position = DVec3::new(3.5, 66.0, 2.0);
            e.spit.yaw = 45.0;
        }
        let later = update(&mut trackers, &host, &watchers);
        for mob in later.moves_of(2).unwrap() {
            show_on(&mut friend, mob);
        }
        let shown = &friend.llama_spits()[0];
        assert_eq!((shown.spit.position, shown.spit.yaw, shown.previous_position), (DVec3::new(3.5, 66.0, 2.0), 45.0, DVec3::new(3.5, 66.0, 0.5)));
    }

    #[test]
    fn a_friend_sees_a_drowned_raise_its_trident() {
        let mut host = EntityWorld::default();
        let (drowned, tag) = spawn(
            &mut host,
            "{id:\"minecraft:drowned\",Pos:[2.5d,64.0d,0.5d],equipment:{mainhand:{id:\"minecraft:trident\",count:1}}}",
        );
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, drowned, &tag));
        assert!(!crate::zombie_render::raises_trident(friend.zombie_mut(drowned).unwrap()));
        host.zombie_mut(drowned).unwrap().aggressive = true;
        assert!(shows(&host)[&drowned].aggressive);
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert!(crate::zombie_render::raises_trident(friend.zombie_mut(drowned).unwrap()));
        host.zombie_mut(drowned).unwrap().aggressive = false;
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert!(!crate::zombie_render::raises_trident(friend.zombie_mut(drowned).unwrap()));
    }

    #[test]
    fn a_friend_sees_a_zombie_villager_shake_while_it_is_cured_and_a_zombie_while_it_drowns() {
        let mut host = EntityWorld::default();
        let (villager, tag) = spawn(
            &mut host,
            "{id:\"minecraft:zombie_villager\",Pos:[2.5d,64.0d,0.5d],VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:1},ConversionTime:-1}",
        );
        let (zombie, zombie_tag) = spawn(&mut host, "{id:\"minecraft:zombie\",Pos:[6.5d,64.0d,0.5d]}");
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, villager, &tag));
        assert!(spawn_tracked(&mut friend, zombie, &zombie_tag));
        assert!(!friend.zombie_mut(villager).unwrap().zombie.shaking());
        host.zombie_mut(villager).unwrap().zombie.villager_conversion_time = 4000;
        host.zombie_mut(zombie).unwrap().zombie.underwater_converting = true;
        assert!(shows(&host)[&villager].converting && shows(&host)[&zombie].converting);
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert!(friend.zombie_mut(villager).unwrap().zombie.shaking());
        assert!(friend.zombie_mut(zombie).unwrap().zombie.shaking());
        host.zombie_mut(villager).unwrap().zombie.villager_conversion_time = -1;
        host.zombie_mut(zombie).unwrap().zombie.underwater_converting = false;
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert!(!friend.zombie_mut(villager).unwrap().zombie.shaking());
        assert!(!friend.zombie_mut(zombie).unwrap().zombie.shaking());
        let mut curing = tag.clone();
        if let Tag::Compound(map) = &mut curing {
            map.insert("ConversionTime".to_owned(), Tag::Int(3999));
        }
        assert_eq!(look(&curing), look(&tag), "the countdown resends no look");
    }

    #[test]
    fn a_loaded_strider_and_its_rider_are_added_as_two_mobs_the_friend_can_spawn() {
        let mut host = EntityWorld::default();
        let jockey = saved(
            "{id:\"minecraft:strider\",Pos:[3.5d,64.0d,0.5d],NoAI:1b,Passengers:[{id:\"minecraft:zombified_piglin\",Pos:[3.5d,65.0d,0.5d],NoAI:1b}]}",
        );
        let spawned = crate::server_mobs::spawn_saved_all(&mut host, &jockey);
        assert_eq!(spawned.len(), 2);
        let originals: HashMap<u64, Tag> = spawned.into_iter().collect();
        let mut trackers = Trackers::default();
        trackers.tick();
        let tracked = trackers.update(&host, &originals, &[watcher(4, 0.5)]);
        let mut friend = EntityWorld::default();
        for event in tracked.events_of(4) {
            let MobEvent::Add(id, tag) = event else {
                panic!("{event:?}");
            };
            assert!(tag.get("Passengers").is_none(), "{tag:?}");
            assert!(spawn_tracked(&mut friend, *id, tag), "{tag:?}");
        }
        assert_eq!(friend.striders().len(), 1);
        assert_eq!(friend.zombies().len(), 1);
    }

    #[test]
    fn the_mirror_moves_and_shows_what_the_host_sends() {
        let mut host = EntityWorld::default();
        let (creeper, creeper_tag) = spawn(
            &mut host,
            "{id:\"minecraft:creeper\",Pos:[2.5d,64.0d,0.5d],NoAI:1b}",
        );
        let (zombie, zombie_tag) = spawn(
            &mut host,
            "{id:\"minecraft:zombie\",Pos:[4.5d,64.0d,0.5d],NoAI:1b}",
        );
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, creeper, &creeper_tag));
        assert!(spawn_tracked(&mut friend, zombie, &zombie_tag));
        {
            let e = host.creeper_mut(creeper).unwrap();
            e.creeper.old_swell = 4;
            e.creeper.swell = 5;
            e.creeper.body.position = DVec3::new(3.0, 65.0, 1.0);
            e.creeper.body.fire_ticks = 40;
        }
        host.zombie_mut(zombie).unwrap().aggressive = true;
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        let shown = &friend.creepers()[0];
        assert_eq!((shown.creeper.old_swell, shown.creeper.swell), (4, 5));
        assert_eq!(shown.creeper.body.position, DVec3::new(3.0, 65.0, 1.0));
        assert_eq!(shown.previous_position, DVec3::new(2.5, 64.0, 0.5));
        assert_eq!(friend.burning().len(), 1, "the burning creeper's flames");
        assert!(friend.zombies()[0].aggressive, "the zombie raises its arms");
    }

    #[test]
    fn a_friend_sees_the_wandering_trader_drink_and_vanish() {
        use minecraftoss_entities::effects::{EffectInstance, MobEffect};
        use minecraftoss_entities::wandering_trader::Drink;
        let mut host = EntityWorld::default();
        let (trader, trader_tag) = spawn(&mut host, "{id:\"minecraft:wandering_trader\",Pos:[2.5d,64.0d,0.5d],NoAI:1b}");
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, trader, &trader_tag));
        assert_eq!(friend.entity_type(trader), Some("minecraft:wandering_trader"));
        host.wandering_trader_mut(trader).unwrap().trader.held = Some(Drink::Invisibility);
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert_eq!(friend.wandering_traders()[0].trader.held, Some(Drink::Invisibility));
        assert!(!friend.wandering_traders()[0].invisible());
        {
            let e = host.wandering_trader_mut(trader).unwrap();
            e.trader.held = None;
            e.effects.add(EffectInstance::new(MobEffect::Invisibility, 3600, 0));
        }
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert!(friend.wandering_traders()[0].invisible(), "the friend loses sight of it too");
        assert_eq!(friend.wandering_traders()[0].trader.held, None);
        host.wandering_trader_mut(trader).unwrap().effects.remove(MobEffect::Invisibility);
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert!(!friend.wandering_traders()[0].invisible());
    }

    #[test]
    fn a_friend_sees_a_pillager_from_its_look_tag_with_its_crossbow() {
        use minecraftoss_entities::illager::IllagerArmPose;
        let mut host = EntityWorld::default();
        let (pillager, tag) = spawn(&mut host, "{id:\"minecraft:pillager\",Pos:[2.5d,64.0d,0.5d],NoAI:1b,equipment:{mainhand:{id:\"minecraft:crossbow\",count:1}}}");
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, pillager, &tag));
        assert_eq!(friend.entity_type(pillager), Some("minecraft:pillager"));
        let seen = &friend.illagers()[0];
        assert!(seen.illager.holds_crossbow());
        assert_eq!(seen.arm_pose, IllagerArmPose::CrossbowHold);
    }

    #[test]
    fn a_friend_sees_a_patrol_captain_with_its_ominous_banner() {
        let mut host = EntityWorld::default();
        let (captain, tag) = spawn(&mut host, "{id:\"minecraft:pillager\",Pos:[2.5d,64.0d,0.5d],NoAI:1b,PatrolLeader:1b,Patrolling:1b,patrol_target:[I;100,0,-40],equipment:{mainhand:{id:\"minecraft:crossbow\",count:1},head:{id:\"minecraft:white_banner\",count:1,components:{\"minecraft:item_name\":{translate:\"block.minecraft.ominous_banner\"}}}},drop_chances:{head:2.0f}}");
        assert!(host.illagers()[0].is_captain());
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, captain, &tag));
        let seen = &friend.illagers()[0];
        assert!(seen.is_captain());
        assert_eq!(
            seen.illager.equipment[minecraftoss_entities::illager::HEAD].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:white_banner")
        );
        assert!(crate::illager_render::worn_on_head("minecraft:white_banner"));
    }

    #[test]
    fn a_friend_sees_a_vindicator_cross_its_arms_and_then_raise_its_axe() {
        use minecraftoss_entities::illager::IllagerArmPose;
        let mut host = EntityWorld::default();
        let (vindicator, tag) = spawn(&mut host, "{id:\"minecraft:vindicator\",Pos:[2.5d,64.0d,0.5d],NoAI:1b,equipment:{mainhand:{id:\"minecraft:iron_axe\",count:1}}}");
        assert_eq!(tracking_range("minecraft:vindicator"), 8);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, vindicator, &tag));
        assert_eq!(friend.entity_type(vindicator), Some("minecraft:vindicator"));
        assert_eq!(friend.illagers()[0].arm_pose, IllagerArmPose::Crossed);
        assert!(!friend.illagers()[0].illager.shows_held_items());
        {
            let e = host.illager_mut(vindicator).unwrap();
            e.illager.aggressive = true;
            e.arm_pose = e.illager.arm_pose();
        }
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        let seen = &friend.illagers()[0];
        assert_eq!(seen.arm_pose, IllagerArmPose::Attacking);
        assert!(seen.illager.shows_held_items(), "the axe shows while it attacks");
    }

    #[test]
    fn a_friend_sees_an_evoker_cast_and_its_fangs_bite() {
        use minecraftoss_entities::illager::{IllagerArmPose, IllagerSpell};
        let mut host = EntityWorld::default();
        let (evoker, tag) = spawn(&mut host, "{id:\"minecraft:evoker\",Pos:[2.5d,64.0d,0.5d],NoAI:1b}");
        let (fangs, fangs_tag) = spawn(&mut host, "{id:\"minecraft:evoker_fangs\",Pos:[4.5d,64.0d,0.5d],Warmup:0}");
        assert_eq!(tracking_range("minecraft:evoker"), 8);
        assert_eq!(tracking_range("minecraft:evoker_fangs"), 6);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, evoker, &tag));
        assert!(spawn_tracked(&mut friend, fangs, &fangs_tag));
        assert!(is_projectile(&friend, fangs));
        {
            let e = host.illager_mut(evoker).unwrap();
            e.illager.spell = IllagerSpell::Wololo;
            e.arm_pose = e.illager.arm_pose();
        }
        for _ in 0..5 {
            host.evoker_fangs_mut(fangs).unwrap().fangs.tick();
        }
        for mob in server_mobs(&host).iter().chain(&projectiles(&host)) {
            show_on(&mut friend, mob);
        }
        let seen = &friend.illagers()[0];
        assert_eq!(seen.arm_pose, IllagerArmPose::Spellcasting);
        assert_eq!(seen.illager.spell, IllagerSpell::Wololo);
        let (host_fangs, friend_fangs) = (&host.evoker_fangs()[0].fangs, &friend.evoker_fangs()[0].fangs);
        assert!(host_fangs.sent_spike && friend_fangs.sent_spike);
        assert_eq!(friend_fangs.bite_progress(0.5), host_fangs.bite_progress(0.5));
        assert!(friend_fangs.bite_progress(0.5) > 0.0);
    }

    #[test]
    fn a_friend_sees_a_ravager_and_its_rider_and_watches_it_bite_stagger_and_roar() {
        let mut host = EntityWorld::default();
        let tag = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:ravager\",Pos:[2.5d,64.0d,0.5d],UUID:[I;1,2,3,9],NoAI:1b,Passengers:[{id:\"minecraft:pillager\",Pos:[2.5d,65.6d,0.5d],UUID:[I;1,2,3,10],NoAI:1b,equipment:{mainhand:{id:\"minecraft:crossbow\",count:1}}}]}").unwrap();
        let spawned = crate::server_mobs::spawn_saved_all(&mut host, &tag);
        assert_eq!(spawned.len(), 2);
        let (ravager, pillager) = (spawned[0].0, spawned[1].0);
        assert_eq!(tracking_range("minecraft:ravager"), 10);
        let originals: HashMap<u64, Tag> = spawned.into_iter().collect();
        let mut trackers = Trackers::default();
        trackers.tick();
        let tracked = trackers.update(&host, &originals, &[watcher(4, 0.5)]);
        let mut friend = EntityWorld::default();
        for event in tracked.events_of(4) {
            let MobEvent::Add(id, tag) = event else {
                panic!("{event:?}");
            };
            assert!(spawn_tracked(&mut friend, *id, tag), "{tag:?}");
        }
        assert_eq!(friend.entity_type(ravager), Some("minecraft:ravager"));
        assert_eq!(friend.entity_type(pillager), Some("minecraft:pillager"));
        let ticks = [(7, 0, 0), (0, 25, 0), (0, 0, 12)];
        for (attack, stunned, roar) in ticks {
            {
                let e = host.ravager_mut(ravager).unwrap();
                e.ravager.attack_tick = attack;
                e.ravager.stunned_tick = stunned;
                e.ravager.roar_tick = roar;
            }
            for mob in server_mobs(&host) {
                show_on(&mut friend, &mob);
            }
            let seen = &friend.ravagers()[0].ravager;
            assert_eq!((seen.attack_tick, seen.stunned_tick, seen.roar_tick), (attack, stunned, roar));
        }
        let rider = server_mobs(&host).into_iter().find(|m| m.id == pillager).unwrap();
        assert_eq!(rider.vehicle.map(|(vehicle, _)| vehicle), Some(ravager));
        let mut saved = minecraftoss_core::snbt::parse_compound("{id:\"minecraft:ravager\",Pos:[2.5d,64.0d,0.5d],StunTick:30}").unwrap();
        let quiet = look(&saved);
        if let Tag::Compound(map) = &mut saved {
            map.insert("StunTick".to_owned(), Tag::Int(29));
        }
        assert_eq!(look(&saved), quiet, "the clocks never resend the look");
    }

    #[test]
    fn a_friend_sees_a_wither_charge_and_turn_its_three_heads() {
        let mut host = EntityWorld::default();
        let (wither, tag) = spawn(&mut host, "{id:\"minecraft:wither\",Pos:[2.5d,66.0d,0.5d],NoAI:1b,Health:100.0f,Invul:200}");
        assert_eq!(tracking_range("minecraft:wither"), 10);
        assert!(look(&tag).get("Invul").is_none(), "the charge clock is not a look change");
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, wither, &tag));
        assert_eq!(friend.entity_type(wither), Some("minecraft:wither"));
        {
            let w = &mut host.wither_mut(wither).unwrap().wither;
            w.invulnerable_ticks = 150;
            w.head_x_rots = [10.0, -20.0];
            w.head_y_rots = [45.0, -60.0];
            w.health = 300.0;
        }
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        let seen = &friend.withers()[0].wither;
        assert_eq!(seen.invulnerable_ticks, 150);
        assert_eq!((seen.head_x_rots, seen.head_y_rots), ([10.0, -20.0], [45.0, -60.0]));
        assert!(!seen.powered(), "its armour goes once healed");
        host.wither_mut(wither).unwrap().wither.health = 120.0;
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert!(friend.withers()[0].wither.powered(), "and comes back below half health");
    }

    #[test]
    fn a_friend_sees_a_blue_wither_skull_turn_as_it_flies() {
        let mut host = EntityWorld::default();
        let (skull, tag) = spawn(&mut host, "{id:\"minecraft:wither_skull\",Pos:[2.5d,66.0d,0.5d],Motion:[0.0d,0.0d,0.3d],dangerous:1b}");
        assert_eq!(tracking_range("minecraft:wither_skull"), 4);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, skull, &tag));
        assert!(is_projectile(&friend, skull));
        assert_eq!(friend.fireballs()[0].fireball.kind, minecraftoss_entities::fireball::FireballKind::WitherSkull { dangerous: true });
        {
            let e = host.fireball_mut(skull).unwrap();
            e.fireball.position = DVec3::new(2.5, 66.0, 1.0);
            e.fireball.yaw = 120.0;
            e.fireball.pitch = -15.0;
        }
        for mob in projectiles(&host) {
            show_on(&mut friend, &mob);
        }
        let seen = &friend.fireballs()[0].fireball;
        assert_eq!((seen.position, seen.yaw, seen.pitch), (DVec3::new(2.5, 66.0, 1.0), 120.0, -15.0));
    }

    #[test]
    fn a_friend_sees_a_vex_charge() {
        let mut host = EntityWorld::default();
        let (vex, tag) = spawn(&mut host, "{id:\"minecraft:vex\",Pos:[2.5d,66.0d,0.5d],NoAI:1b,equipment:{mainhand:{id:\"minecraft:iron_sword\",count:1}}}");
        assert_eq!(tracking_range("minecraft:vex"), 8);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, vex, &tag));
        assert_eq!(friend.entity_type(vex), Some("minecraft:vex"));
        assert!(!friend.vexes()[0].vex.charging);
        host.vex_mut(vex).unwrap().vex.charging = true;
        host.vex_mut(vex).unwrap().vex.body.position.y = 67.5;
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        let seen = &friend.vexes()[0].vex;
        assert!(seen.charging, "the charging texture shows");
        assert_eq!(seen.body.position.y, 67.5);
        assert_eq!(seen.main_hand().map(|s| s.id.as_str()), Some("minecraft:iron_sword"));
    }

    #[test]
    fn a_friend_sees_a_silverfish_and_an_endermite_from_their_look_tags() {
        use crate::families::FamilyShow;
        use minecraftoss_entities::world::families::pests::endermites::Endermite;
        let mut host = EntityWorld::default();
        host.set_next_entity_id(800);
        let (fish, fish_tag) = spawn(&mut host, "{id:\"minecraft:silverfish\",Pos:[3.5d,64.0d,0.5d],Rotation:[45.0f,0.0f],NoAI:1b}");
        let (mite, mite_tag) = spawn(&mut host, "{id:\"minecraft:endermite\",Pos:[5.5d,64.0d,0.5d],NoAI:1b,Lifetime:100}");
        assert_eq!((tracking_range("minecraft:silverfish"), tracking_range("minecraft:endermite"), tracking_range("endermite")), (8, 8, 8));
        let mut trackers = Trackers::default();
        trackers.tick();
        let originals: HashMap<u64, Tag> = [(fish, fish_tag), (mite, mite_tag)].into_iter().collect();
        let tracked = trackers.update(&host, &originals, &[watcher(3, 0.5)]);
        let mut friend = EntityWorld::default();
        let mut added = Vec::new();
        for event in tracked.events_of(3) {
            let MobEvent::Add(id, sent) = event else { panic!("{event:?}") };
            assert!(spawn_tracked(&mut friend, *id, sent));
            added.push(*id);
        }
        added.sort_unstable();
        assert_eq!(added, vec![fish, mite]);
        assert_eq!((friend.entity_type(fish), friend.entity_type(mite)), (Some("minecraft:silverfish"), Some("minecraft:endermite")));
        assert_eq!(friend.family::<Endermite>()[0].state.life, 100);
        assert_eq!(friend.family_core(fish).unwrap().1.yaw, 45.0);
        let mut moved: Vec<u64> = tracked.moves_of(3).unwrap().iter().map(|mob| mob.id).collect();
        moved.sort_unstable();
        assert_eq!(moved, vec![fish, mite], "both move every tick");
        let seen = shows(&host);
        assert_eq!((seen[&fish].family, seen[&mite].family), (Some(FamilyShow::default()), Some(FamilyShow::default())));
        host.family_mob_mut::<Endermite>(mite).unwrap().state.life = 160;
        {
            let core = host.family_core_mut(fish).unwrap().1;
            core.body.position = DVec3::new(4.0, 65.0, 1.5);
            core.body.fire_ticks = 20;
            core.swings = 3;
        }
        for _ in 0..LOOK_TICKS {
            trackers.tick();
        }
        let later = trackers.update(&host, &originals, &[watcher(3, 0.5)]);
        assert!(later.events_of(3).next().is_none(), "a lifetime counting up is no new look: {:?}", later.events);
        let moves = later.moves_of(3).unwrap();
        let sent = moves.iter().find(|mob| mob.id == fish).unwrap();
        assert_eq!((sent.swings, sent.show.fire, sent.show.family), (3, true, Some(FamilyShow::default())));
        for mob in moves {
            show_on(&mut friend, mob);
        }
        let shown = friend.family_core(fish).unwrap().1;
        assert_eq!((shown.body.position, shown.body.fire_ticks), (DVec3::new(4.0, 65.0, 1.5), 1));
        assert_eq!(shown.previous_position, DVec3::new(3.5, 64.0, 0.5));
    }

    #[test]
    fn a_registered_look_tag_drops_only_its_own_volatile_keys() {
        let mite = look(&saved("{id:\"minecraft:endermite\",Lifetime:100,Size:2,Health:8.0f}"));
        assert_eq!((mite.get("Lifetime"), mite.get("Size"), mite.get("Health")), (None, Some(&Tag::Int(2)), None));
        let fish = look(&saved("{id:\"minecraft:silverfish\",Lifetime:100}"));
        assert_eq!(fish.get("Lifetime"), Some(&Tag::Int(100)), "the silverfish lists no volatile keys");
        let slime = look(&saved("{id:\"minecraft:slime\",Size:2,Lifetime:100}"));
        assert_eq!((slime.get("Size"), slime.get("Lifetime")), (Some(&Tag::Int(2)), Some(&Tag::Int(100))), "an old family keeps every key of its look");
        assert_eq!(crate::families::volatile("minecraft:endermite"), Some(&["Lifetime"][..]));
        assert_eq!((crate::families::volatile("minecraft:silverfish"), crate::families::volatile("minecraft:slime")), (Some(&[][..]), None));
    }

    #[test]
    fn a_friend_sees_a_pillager_charge_and_then_hold_its_crossbow() {
        use minecraftoss_entities::illager::{Illager, IllagerArmPose};
        let at = DVec3::new(2.5, 64.0, 0.5);
        let mut host = EntityWorld::default();
        let id = host.spawn_illager(Illager::pillager(at), true);
        let mut friend = EntityWorld::default();
        assert_eq!(friend.spawn_illager(Illager::pillager(at), true), id);
        assert_eq!(tracking_range("minecraft:pillager"), 8);
        {
            let e = host.illager_mut(id).unwrap();
            e.illager.charging_crossbow = true;
            e.arm_pose = IllagerArmPose::CrossbowCharge;
            e.using_item = Some(12);
        }
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        let seen = &friend.illagers()[0];
        assert_eq!((seen.arm_pose, seen.using_item), (IllagerArmPose::CrossbowCharge, Some(12)));
        assert!(seen.illager.charging_crossbow);
        {
            let e = host.illager_mut(id).unwrap();
            e.illager.charging_crossbow = false;
            e.arm_pose = IllagerArmPose::CrossbowHold;
            e.using_item = None;
            e.illager.aggressive = true;
        }
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        let seen = &friend.illagers()[0];
        assert_eq!((seen.arm_pose, seen.using_item), (IllagerArmPose::CrossbowHold, None));
        assert!(seen.illager.aggressive && !seen.illager.charging_crossbow);
    }

    #[test]
    fn a_friend_keys_a_pillager_s_and_a_piglin_s_crossbow_like_the_host() {
        use minecraftoss_entities::illager::{IllagerArmPose, MAINHAND};
        let mut host = EntityWorld::default();
        let (pillager, pillager_tag) = spawn(&mut host, "{id:\"minecraft:pillager\",Pos:[2.5d,64.0d,0.5d],equipment:{mainhand:{id:\"minecraft:crossbow\",count:1}}}");
        let (piglin, piglin_tag) = spawn(&mut host, "{id:\"minecraft:piglin\",Pos:[3.5d,64.0d,0.5d],LeftHanded:1b,equipment:{mainhand:{id:\"minecraft:crossbow\",count:1}}}");
        let originals: HashMap<u64, Tag> = [(pillager, pillager_tag), (piglin, piglin_tag)].into_iter().collect();
        let mut trackers = Trackers::default();
        let mut friend = EntityWorld::default();
        let mut sync = |host: &EntityWorld, friend: &mut EntityWorld| {
            for _ in 0..LOOK_TICKS {
                trackers.tick();
                let tracked = trackers.update(host, &originals, &[watcher(3, 0.5)]);
                for event in tracked.events_of(3) {
                    if let MobEvent::Add(id, sent) | MobEvent::Look(id, sent) = event {
                        assert!(spawn_tracked(friend, *id, sent));
                    }
                }
                for mob in tracked.moves_of(3).unwrap_or_default() {
                    show_on(friend, mob);
                }
            }
        };
        let keys = |world: &EntityWorld| {
            let shown_illager = world.illagers().iter().find(|e| e.id == pillager).unwrap();
            let shown_piglin = world.piglins().iter().find(|e| e.id == piglin).unwrap();
            (crate::illager_render::hand_key(shown_illager, MAINHAND), crate::piglin_render::hand_key(shown_piglin, true))
        };
        sync(&host, &mut friend);
        let plain = Some("minecraft:crossbow".to_owned());
        assert_eq!(keys(&friend), (plain.clone(), plain.clone()));
        {
            let e = host.illager_mut(pillager).unwrap();
            e.illager.charging_crossbow = true;
            e.arm_pose = IllagerArmPose::CrossbowCharge;
            e.using_item = Some(15);
            let p = host.piglin_mut(piglin).unwrap();
            p.piglin.charging_crossbow = true;
            p.arm_pose = ArmPose::CrossbowCharge;
            p.using_item = Some(20);
        }
        sync(&host, &mut friend);
        let pulling = (Some("minecraft:crossbow|use=15".to_owned()), Some("minecraft:crossbow|use=20".to_owned()));
        assert_eq!((keys(&host), keys(&friend)), (pulling.clone(), pulling));
        {
            let e = host.illager_mut(pillager).unwrap();
            e.illager.charging_crossbow = false;
            e.arm_pose = IllagerArmPose::CrossbowHold;
            e.using_item = None;
            e.illager.set_crossbow_charged(true);
            let p = host.piglin_mut(piglin).unwrap();
            p.piglin.charging_crossbow = false;
            p.arm_pose = ArmPose::CrossbowHold;
            p.using_item = None;
            p.piglin.set_crossbow_charged(true);
        }
        sync(&host, &mut friend);
        let loaded = (Some("minecraft:crossbow|charge=arrow".to_owned()), Some("minecraft:crossbow|charge=arrow".to_owned()));
        assert_eq!((keys(&host), keys(&friend)), (loaded.clone(), loaded));
        host.illager_mut(pillager).unwrap().illager.set_crossbow_charged(false);
        host.piglin_mut(piglin).unwrap().piglin.set_crossbow_charged(false);
        sync(&host, &mut friend);
        assert_eq!(keys(&friend), (plain.clone(), plain), "the shot empties the friend's crossbows too");
    }

    #[test]
    fn a_friend_sees_a_snow_golem_from_its_look_tag_and_loses_its_pumpkin_to_shears() {
        let mut host = EntityWorld::default();
        let (golem, golem_tag) = spawn(&mut host, "{id:\"minecraft:snow_golem\",Pos:[2.5d,64.0d,0.5d],NoAI:1b,Pumpkin:1b}");
        assert_eq!(tracking_range("minecraft:snow_golem"), 8);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, golem, &golem_tag));
        assert_eq!(friend.entity_type(golem), Some("minecraft:snow_golem"));
        assert!(friend.snow_golems()[0].golem.pumpkin);
        host.shear_snow_golem(golem).expect("sheared");
        for mob in server_mobs(&host) {
            show_on(&mut friend, &mob);
        }
        assert!(!friend.snow_golems()[0].golem.pumpkin, "the friend sees its bare head");
        assert_eq!(friend.snow_golems()[0].golem.body.position, DVec3::new(2.5, 64.0, 0.5));
    }

    #[test]
    fn a_friend_sees_a_cow_tied_to_the_host_and_then_to_a_fence_knot() {
        use crate::leash_render::{rope_ends, RopeEnd};
        use minecraftoss_entities::world::LeashHolder;
        let mut host = EntityWorld::default();
        let (cow, cow_tag) = spawn(&mut host, "{id:\"minecraft:cow\",Pos:[2.5d,64.0d,0.5d]}");
        host.set_player_uuid(0, 0x5eed);
        host.set_leash(cow, LeashHolder::Player(0));
        let mut trackers = Trackers::default();
        trackers.tick();
        let originals: HashMap<u64, Tag> = [(cow, cow_tag)].into_iter().collect();
        let tracked = trackers.update(&host, &originals, &[watcher(3, 0.5)]);
        let mut friend = EntityWorld::default();
        for event in tracked.events_of(3) {
            let MobEvent::Add(id, sent) = event else { panic!("{event:?}") };
            assert!(spawn_tracked(&mut friend, *id, sent));
        }
        assert_eq!(rope_ends(&friend), [(cow, RopeEnd::Player(0x5eed))]);
        host.set_leash(cow, LeashHolder::Knot((4, 64, 0)));
        let mut looks = Vec::new();
        for _ in 0..LOOK_TICKS {
            trackers.tick();
            looks.extend(trackers.update(&host, &originals, &[watcher(3, 0.5)]).events);
        }
        let (_, MobEvent::Look(id, sent)) = &looks[0] else { panic!("{looks:?}") };
        assert!(spawn_tracked(&mut friend, *id, sent));
        assert_eq!(rope_ends(&friend), [(cow, RopeEnd::Knot((4, 64, 0)))]);
        assert_eq!(friend.knots(), [(4, 64, 0)]);
    }
}
