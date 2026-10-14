use std::collections::HashMap;

use minecraft_commands::Text;
use minecraftoss_entities::inside_blocks::Hazard;
use minecraftoss_player::GameMode;
use minecraftoss_player::inventory::{Inventory, ItemStack};

const DAMAGE_STATUS_TICKS: u64 = 100;
const COMBAT_STATUS_TICKS: u64 = 300;
const STATUS_RECHECK_TICKS: u64 = 20;
const KILL_CREDIT_TICKS: u64 = 100;
const SIGNIFICANT_FALL: f32 = 5.0;
const VICTIM: &str = "%1$s";
const ATTACKER: &str = "%2$s";
const WEAPON: &str = "%3$s";
const ITEM_FORMS: [&str; 4] = [PLAYER_KILL, "death.fell.assist", "death.fell.finish", "death.attack.explosion.player"];
const OWNERLESS_ARROW: &str = "minecraft:arrow";
const DEATH_EXPERIENCE_PER_LEVEL: u32 = 7;
const MAX_DEATH_EXPERIENCE: u32 = 100;
const STORAGE_SLOTS: usize = 36;
const EQUIPMENT_DROP_ORDER: [usize; 9] = [40, 36, 37, 38, 39, 41, 42, 43, 44];
const ENCHANTMENTS: &str = "minecraft:enchantments";
pub const PLAYER_KILL: &str = "death.attack.player";
pub const PLAYER_KILL_ITEM: &str = "death.attack.player.item";
const VANISHING_CURSE: &str = "minecraft:vanishing_curse";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FallLocation {
    Generic,
    Ladder,
    Vines,
    WeepingVines,
    TwistingVines,
    Scaffolding,
    OtherClimbable,
    Water,
}

impl FallLocation {
    pub fn of_climbable(block: &str) -> Self {
        match block.strip_prefix("minecraft:").unwrap_or(block) {
            "ladder" => Self::Ladder,
            trapdoor if trapdoor.ends_with("_trapdoor") => Self::Ladder,
            "vine" => Self::Vines,
            "weeping_vines" | "weeping_vines_plant" => Self::WeepingVines,
            "twisting_vines" | "twisting_vines_plant" => Self::TwistingVines,
            "scaffolding" => Self::Scaffolding,
            _ => Self::OtherClimbable,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Generic => "death.fell.accident.generic",
            Self::Ladder => "death.fell.accident.ladder",
            Self::Vines => "death.fell.accident.vines",
            Self::WeepingVines => "death.fell.accident.weeping_vines",
            Self::TwistingVines => "death.fell.accident.twisting_vines",
            Self::Scaffolding => "death.fell.accident.scaffolding",
            Self::OtherClimbable => "death.fell.accident.other_climbable",
            Self::Water => "death.fell.accident.water",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobAttack {
    Melee,
    Arrow,
    Explosion,
    Fireball,
    SmallFireball,
    MobProjectile,
    Trident,
    Spit,
    Magic,
    Fireworks,
    WitherSkull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageCause {
    Generic,
    Hazard(Hazard),
    Fall,
    EnderPearl,
    FlyIntoWall,
    Drown,
    Starve,
    Magic,
    Wither,
    Kill,
    OutOfWorld,
    BadRespawnPoint,
    Explosion,
    Fireworks,
    Mob {
        id: u64,
        type_id: &'static str,
        attack: MobAttack,
    },
    Player {
        client: u32,
    },
    PlayerExplosion {
        client: u32,
    },
}

pub const IS_EXPLOSION: [&str; 4] = [
    "minecraft:fireworks",
    "minecraft:explosion",
    "minecraft:player_explosion",
    "minecraft:bad_respawn_point",
];

impl DamageCause {
    pub fn mob(
        attack: MobAttack,
        source: Option<u64>,
        type_id: Option<&'static str>,
    ) -> Self {
        let type_id = type_id.or(match attack {
            MobAttack::Explosion => Some("minecraft:creeper"),
            MobAttack::Arrow => Some(OWNERLESS_ARROW),
            MobAttack::Melee | MobAttack::Fireball | MobAttack::SmallFireball | MobAttack::MobProjectile | MobAttack::Trident | MobAttack::Spit | MobAttack::Magic | MobAttack::Fireworks | MobAttack::WitherSkull => None,
        });
        match (source, type_id) {
            (Some(id), Some(type_id)) => Self::Mob {
                id,
                type_id,
                attack,
            },
            (None, Some(OWNERLESS_ARROW)) => Self::Mob {
                id: 0,
                type_id: OWNERLESS_ARROW,
                attack,
            },
            (None, _) if attack == MobAttack::Explosion => Self::Explosion,
            _ => Self::Generic,
        }
    }

    fn living_attacker(self) -> Option<(u64, &'static str)> {
        match self {
            Self::Mob { id, type_id, .. } if type_id != OWNERLESS_ARROW => Some((id, type_id)),
            _ => None,
        }
    }

    pub fn damage_type(self) -> &'static str {
        match self {
            Self::Generic => "minecraft:generic",
            Self::Hazard(Hazard::InFire) => "minecraft:in_fire",
            Self::Hazard(Hazard::Campfire) => "minecraft:campfire",
            Self::Hazard(Hazard::Lava) => "minecraft:lava",
            Self::Hazard(Hazard::Cactus) => "minecraft:cactus",
            Self::Hazard(Hazard::HotFloor) => "minecraft:hot_floor",
            Self::Hazard(Hazard::SweetBerryBush) => "minecraft:sweet_berry_bush",
            Self::Hazard(Hazard::OnFire) => "minecraft:on_fire",
            Self::Hazard(Hazard::InWall) => "minecraft:in_wall",
            Self::Hazard(Hazard::Cramming) => "minecraft:cramming",
            Self::Hazard(Hazard::Freeze) => "minecraft:freeze",
            Self::Fall => "minecraft:fall",
            Self::EnderPearl => "minecraft:ender_pearl",
            Self::FlyIntoWall => "minecraft:fly_into_wall",
            Self::Drown => "minecraft:drown",
            Self::Starve => "minecraft:starve",
            Self::Magic => "minecraft:magic",
            Self::Wither => "minecraft:wither",
            Self::Kill => "minecraft:generic_kill",
            Self::OutOfWorld => "minecraft:out_of_world",
            Self::BadRespawnPoint => "minecraft:bad_respawn_point",
            Self::Explosion => "minecraft:explosion",
            Self::Fireworks => "minecraft:fireworks",
            Self::Player { .. } => "minecraft:player_attack",
            Self::PlayerExplosion { .. } => "minecraft:player_explosion",
            Self::Mob { attack, .. } => match attack {
                MobAttack::Melee => "minecraft:mob_attack",
                MobAttack::Arrow => "minecraft:arrow",
                MobAttack::Explosion => "minecraft:explosion",
                MobAttack::Fireball | MobAttack::SmallFireball => "minecraft:fireball",
                MobAttack::MobProjectile => "minecraft:mob_projectile",
                MobAttack::Trident => "minecraft:trident",
                MobAttack::Spit => "minecraft:spit",
                MobAttack::Magic => "minecraft:indirect_magic",
                MobAttack::Fireworks => "minecraft:fireworks",
                MobAttack::WitherSkull => "minecraft:wither_skull",
            },
        }
    }

    fn message_id(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Player { .. } => "player",
            Self::PlayerExplosion { .. } => "explosion.player",
            Self::Hazard(Hazard::InFire | Hazard::Campfire) => "inFire",
            Self::Hazard(Hazard::Lava) => "lava",
            Self::Hazard(Hazard::Cactus) => "cactus",
            Self::Hazard(Hazard::HotFloor) => "hotFloor",
            Self::Hazard(Hazard::SweetBerryBush) => "sweetBerryBush",
            Self::Hazard(Hazard::OnFire) => "onFire",
            Self::Hazard(Hazard::InWall) => "inWall",
            Self::Hazard(Hazard::Cramming) => "cramming",
            Self::Hazard(Hazard::Freeze) => "freeze",
            Self::Fall | Self::EnderPearl => "fall",
            Self::FlyIntoWall => "flyIntoWall",
            Self::Drown => "drown",
            Self::Starve => "starve",
            Self::Magic => "magic",
            Self::Wither => "wither",
            Self::Kill => "genericKill",
            Self::OutOfWorld => "outOfWorld",
            Self::BadRespawnPoint => "badRespawnPoint",
            Self::Explosion => "explosion",
            Self::Fireworks
            | Self::Mob {
                attack: MobAttack::Fireworks,
                ..
            } => "fireworks",
            Self::Mob {
                attack: MobAttack::Melee | MobAttack::MobProjectile | MobAttack::Spit,
                ..
            } => "mob",
            Self::Mob {
                attack: MobAttack::Arrow,
                ..
            } => "arrow",
            Self::Mob {
                attack: MobAttack::Explosion,
                ..
            } => "explosion.player",
            Self::Mob {
                attack: MobAttack::Fireball | MobAttack::SmallFireball,
                ..
            } => "fireball",
            Self::Mob {
                attack: MobAttack::Trident,
                ..
            } => "trident",
            Self::Mob {
                attack: MobAttack::Magic,
                ..
            } => "indirectMagic",
            Self::Mob {
                attack: MobAttack::WitherSkull,
                ..
            } => "witherSkull",
        }
    }

    pub fn bypasses_armor(self) -> bool {
        matches!(
            self,
            Self::Magic
                | Self::Mob {
                    attack: MobAttack::Magic,
                    ..
                }
        )
    }

    pub fn is_explosion(self) -> bool {
        IS_EXPLOSION.contains(&self.damage_type())
    }

    pub fn burns(self) -> bool {
        matches!(
            self,
            Self::Mob {
                attack: MobAttack::Fireball | MobAttack::SmallFireball,
                ..
            }
        )
    }

    pub fn ignites(self) -> Option<i32> {
        match self {
            Self::Mob {
                attack: MobAttack::SmallFireball,
                ..
            } => Some(minecraftoss_entities::fireball::SMALL_IGNITE_TICKS),
            _ => None,
        }
    }

    pub fn melee_effect(self, difficulty: minecraftoss_player::survival::Difficulty) -> Option<(minecraftoss_player::survival::EffectKind, u32)> {
        match self {
            Self::Mob {
                type_id: "minecraft:wither_skeleton",
                attack: MobAttack::Melee,
                ..
            } => Some((minecraftoss_player::survival::EffectKind::Wither, 200)),
            Self::Mob {
                type_id: "minecraft:cave_spider",
                attack: MobAttack::Melee,
                ..
            } => minecraftoss_entities::spider::Spider::cave(glam::DVec3::ZERO)
                .poison_ticks(difficulty as i32)
                .map(|ticks| (minecraftoss_player::survival::EffectKind::Poison, ticks)),
            _ => None,
        }
    }

    fn link_key(self) -> Option<&'static str> {
        match self {
            Self::BadRespawnPoint => Some("death.attack.badRespawnPoint.link"),
            _ => None,
        }
    }

    fn is_fall(self) -> bool {
        matches!(self, Self::Fall | Self::EnderPearl)
    }

    fn has_direct_source(self) -> bool {
        matches!(self, Self::Explosion | Self::Fireworks)
    }

    fn always_most_significant_fall(self) -> bool {
        self == Self::OutOfWorld
    }

    fn entity(self) -> Option<&'static str> {
        match self {
            Self::Mob { type_id, .. } => Some(type_id),
            _ => None,
        }
    }

    fn block_name_key(self) -> Option<&'static str> {
        match self {
            Self::Hazard(Hazard::InFire) => Some("block.minecraft.fire"),
            Self::Hazard(Hazard::Campfire) => Some("block.minecraft.campfire"),
            Self::Hazard(Hazard::Lava) => Some("block.minecraft.lava"),
            Self::Hazard(Hazard::Cactus) => Some("block.minecraft.cactus"),
            Self::Hazard(Hazard::HotFloor) => Some("block.minecraft.magma_block"),
            Self::Hazard(Hazard::SweetBerryBush) => Some("block.minecraft.sweet_berry_bush"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CombatEntry {
    cause: DamageCause,
    damage: f32,
    location: Option<FallLocation>,
    fall_distance: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CombatTracker {
    entries: Vec<CombatEntry>,
    last_damage: u64,
    in_combat: bool,
    taking_damage: bool,
    hurt_by: Option<(u64, &'static str, u64)>,
    hurt_by_player: Option<(u32, u64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeathLine {
    pub message: String,
    pub killer: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Killer {
    pub name: String,
    pub weapon: Option<String>,
}

struct DeathKey {
    key: String,
    attacker: Option<&'static str>,
    cause: Option<DamageCause>,
    player: Option<Killer>,
}

fn by_player(
    key: &str,
    client: u32,
    cause: DamageCause,
    killer: impl Fn(u32) -> Option<Killer>,
    missing: &str,
) -> DeathKey {
    let Some(found) = killer(client) else {
        return DeathKey {
            key: missing.to_owned(),
            attacker: None,
            cause: Some(cause),
            player: None,
        };
    };
    let named = found.weapon.is_some() && ITEM_FORMS.contains(&key);
    DeathKey {
        key: if named { format!("{key}.item") } else { key.to_owned() },
        attacker: None,
        cause: Some(cause),
        player: Some(Killer {
            weapon: found.weapon.filter(|_| named),
            ..found
        }),
    }
}

impl CombatTracker {
    pub fn record(
        &mut self,
        tick: u64,
        cause: DamageCause,
        damage: f32,
        location: Option<FallLocation>,
        fall_distance: f32,
    ) {
        self.recheck(tick);
        self.entries.push(CombatEntry {
            cause,
            damage,
            location,
            fall_distance,
        });
        self.last_damage = tick;
        self.taking_damage = true;
        if cause.living_attacker().is_some() || matches!(cause, DamageCause::Player { .. } | DamageCause::PlayerExplosion { .. }) {
            self.in_combat = true;
        }
    }

    pub fn hurt_by(&mut self, tick: u64, cause: DamageCause) {
        if let Some((id, type_id)) = cause.living_attacker() {
            self.hurt_by = Some((id, type_id, tick));
        }
        if let DamageCause::Player { client } | DamageCause::PlayerExplosion { client } = cause {
            self.hurt_by_player = Some((client, tick));
        }
    }

    pub fn tick(&mut self, tick: u64) {
        if self
            .hurt_by
            .is_some_and(|(.., at)| tick.saturating_sub(at) > KILL_CREDIT_TICKS)
        {
            self.hurt_by = None;
        }
        if self
            .hurt_by_player
            .is_some_and(|(_, at)| tick.saturating_sub(at) > KILL_CREDIT_TICKS)
        {
            self.hurt_by_player = None;
        }
        if tick.is_multiple_of(STATUS_RECHECK_TICKS) {
            self.recheck(tick);
        }
    }

    fn recheck(&mut self, tick: u64) {
        let window = if self.in_combat {
            COMBAT_STATUS_TICKS
        } else {
            DAMAGE_STATUS_TICKS
        };
        if self.taking_damage && tick.saturating_sub(self.last_damage) > window {
            self.taking_damage = false;
            self.in_combat = false;
            self.entries.clear();
        }
    }

    fn most_significant_fall(&self) -> Option<&CombatEntry> {
        let mut fall: Option<(&CombatEntry, f32)> = None;
        let mut located: Option<(&CombatEntry, f32)> = None;
        for (index, entry) in self.entries.iter().enumerate() {
            let distance = if entry.cause.always_most_significant_fall() {
                f32::MAX
            } else {
                entry.fall_distance
            };
            if (entry.cause.is_fall() || entry.cause.always_most_significant_fall())
                && distance > 0.0
                && fall.is_none_or(|(_, farthest)| distance > farthest)
            {
                let before = index.checked_sub(1).map_or(entry, |i| &self.entries[i]);
                fall = Some((before, distance));
            }
            if entry.location.is_some() && located.is_none_or(|(_, hardest)| entry.damage > hardest)
            {
                located = Some((entry, entry.damage));
            }
        }
        fall.filter(|(_, distance)| *distance > SIGNIFICANT_FALL)
            .or(located.filter(|(_, damage)| *damage > SIGNIFICANT_FALL))
            .map(|(entry, _)| entry)
    }

    fn death_key(
        &self,
        present: impl Fn(u64) -> bool,
        killer: impl Fn(u32) -> Option<Killer>,
    ) -> DeathKey {
        let Some(last) = self.entries.last() else {
            return DeathKey {
                key: "death.attack.generic".to_owned(),
                attacker: None,
                cause: None,
                player: None,
            };
        };
        if last.cause.is_fall()
            && let Some(fall) = self.most_significant_fall()
        {
            if fall.cause.is_fall() || fall.cause.always_most_significant_fall() {
                return DeathKey {
                    key: fall
                        .location
                        .unwrap_or(FallLocation::Generic)
                        .key()
                        .to_owned(),
                    attacker: None,
                    cause: Some(fall.cause),
                    player: None,
                };
            }
            if let DamageCause::Player { client } | DamageCause::PlayerExplosion { client } = fall.cause {
                return by_player(
                    "death.fell.assist",
                    client,
                    last.cause,
                    killer,
                    "death.fell.killer",
                );
            }
            let finisher = last.cause.entity();
            let (key, attacker) = match fall.cause.entity() {
                Some(causer) if Some(causer) != finisher => ("death.fell.assist", Some(causer)),
                _ if finisher.is_some() => ("death.fell.finish", finisher),
                _ => ("death.fell.killer", None),
            };
            return DeathKey {
                key: key.to_owned(),
                attacker,
                cause: Some(last.cause),
                player: None,
            };
        }
        let key = format!("death.attack.{}", last.cause.message_id());
        if last.cause.link_key().is_some() {
            return DeathKey {
                key: format!("{key}.message"),
                attacker: None,
                cause: Some(last.cause),
                player: None,
            };
        }
        if let DamageCause::Player { client } = last.cause {
            return by_player(&key, client, last.cause, killer, "death.attack.generic");
        }
        if let DamageCause::PlayerExplosion { client } = last.cause {
            return by_player(&key, client, last.cause, killer, "death.attack.explosion");
        }
        if let Some(attacker) = last.cause.entity() {
            return DeathKey {
                key,
                attacker: Some(attacker),
                cause: Some(last.cause),
                player: None,
            };
        }
        if last.cause.has_direct_source() {
            return DeathKey {
                key,
                attacker: None,
                cause: Some(last.cause),
                player: None,
            };
        }
        if let Some(found) = self.hurt_by_player.and_then(|(client, _)| killer(client)) {
            return DeathKey {
                key: format!("{key}.player"),
                attacker: None,
                cause: Some(last.cause),
                player: Some(Killer {
                    weapon: None,
                    ..found
                }),
            };
        }
        match self.hurt_by.filter(|(id, ..)| present(*id)) {
            Some((_, credit, _)) => DeathKey {
                key: format!("{key}.player"),
                attacker: Some(credit),
                cause: Some(last.cause),
                player: None,
            },
            None => DeathKey {
                key,
                attacker: None,
                cause: Some(last.cause),
                player: None,
            },
        }
    }

    pub fn death_line(
        &self,
        language: &HashMap<String, String>,
        present: impl Fn(u64) -> bool,
        killer: impl Fn(u32) -> Option<Killer>,
    ) -> DeathLine {
        let death = self.death_key(present, killer);
        let template = language
            .get(&death.key)
            .cloned()
            .unwrap_or_else(|| death.key.clone());
        let attacker = death
            .attacker
            .map(|type_id| entity_name(language, type_id))
            .or_else(|| death.player.as_ref().map(|found| found.name.clone()))
            .or_else(|| {
                let key = death.cause?.link_key()?;
                Some(format!(
                    "[{}]",
                    language.get(key).map_or(key, String::as_str)
                ))
            });
        let message = attacker
            .as_deref()
            .map_or_else(|| template.clone(), |name| template.replace(ATTACKER, name));
        let message = match death.player.as_ref().and_then(|found| found.weapon.as_deref()) {
            Some(weapon) => message.replace(WEAPON, &format!("[{weapon}]")),
            None => message,
        };
        let killer = attacker
            .or_else(|| {
                death
                    .cause
                    .and_then(DamageCause::block_name_key)
                    .and_then(|key| language.get(key).cloned())
            })
            .unwrap_or_else(|| {
                message
                    .strip_prefix(VICTIM)
                    .unwrap_or(&message)
                    .trim()
                    .to_owned()
            });
        DeathLine { message, killer }
    }

    pub fn death_text(
        &self,
        victim: &str,
        present: impl Fn(u64) -> bool,
        killer: impl Fn(u32) -> Option<Killer>,
    ) -> Text {
        let death = self.death_key(present, killer);
        let mut args = vec![Text::literal(victim)];
        if let Some(type_id) = death.attacker {
            args.push(Text::entity(type_id));
        } else if let Some(found) = &death.player {
            args.push(Text::literal(found.name.clone()));
            if let Some(weapon) = &found.weapon {
                args.push(Text::literal(format!("[{weapon}]")));
            }
        } else if let Some(key) = death.cause.and_then(DamageCause::link_key) {
            let link = Text::translate(key, Vec::new());
            args.push(Text::literal("[").append(link).append(Text::literal("]")));
        }
        Text::translate(death.key, args)
    }
}

pub fn player_kill_text(victim: &str, attacker: &str, weapon: Option<&str>) -> Text {
    let (victim, attacker) = (Text::literal(victim), Text::literal(attacker));
    match weapon {
        Some(weapon) => {
            let weapon = Text::literal(format!("[{weapon}]"));
            Text::translate(PLAYER_KILL_ITEM, vec![victim, attacker, weapon])
        }
        None => Text::translate(PLAYER_KILL, vec![victim, attacker]),
    }
}

fn entity_name(language: &HashMap<String, String>, type_id: &str) -> String {
    let path = type_id.strip_prefix("minecraft:").unwrap_or(type_id);
    language
        .get(&format!("entity.minecraft.{path}"))
        .cloned()
        .unwrap_or_else(|| path.to_owned())
}

pub fn death_experience(level: u32, keep_inventory: bool) -> i32 {
    if keep_inventory {
        return 0;
    }
    level
        .saturating_mul(DEATH_EXPERIENCE_PER_LEVEL)
        .min(MAX_DEATH_EXPERIENCE) as i32
}

pub fn keeps_belongings(keep_inventory: bool, mode: GameMode) -> bool {
    keep_inventory || mode == GameMode::Spectator
}

pub fn life_state(pm_type: i32) -> (bool, bool) {
    let in_world = matches!(
        pm_type,
        0 | playerstate_iw4::PM_TYPE_NORMAL_LINKED | playerstate_iw4::PM_TYPE_LAST_STAND
    );
    let died = matches!(
        pm_type,
        playerstate_iw4::PM_TYPE_DEAD | playerstate_iw4::PM_TYPE_DEAD_LINKED
    );
    (in_world, died)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeathSpot {
    pub feet: [f64; 3],
    pub eye: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Death {
    pub spot: DeathSpot,
    pub experience: i32,
    pub keep_inventory: bool,
}

#[derive(Clone, Debug, Default)]
pub struct DeathWatch {
    in_world: bool,
    switching_teams: bool,
    switched_teams: bool,
    spot: Option<DeathSpot>,
    pending: Option<Death>,
}

impl DeathWatch {
    pub fn observe(&mut self, in_world: bool, died: bool, level: u32, keep_inventory: bool) {
        if self.in_world && died {
            if self.switching_teams {
                self.switched_teams = true;
            } else if let Some(spot) = self.spot.take() {
                self.pending = Some(Death {
                    spot,
                    experience: death_experience(level, keep_inventory),
                    keep_inventory,
                });
            }
        }
        self.in_world = in_world;
        if in_world {
            self.switched_teams = false;
        } else {
            self.spot = None;
            self.switching_teams = false;
        }
    }

    pub fn switch_teams(&mut self) {
        if self.in_world {
            self.switching_teams = true;
        }
    }

    pub fn switched_teams(&self) -> bool {
        self.switched_teams
    }

    pub fn stood(&mut self, spot: DeathSpot) {
        if self.in_world {
            self.spot = Some(spot);
        }
    }

    pub fn take(&mut self) -> Option<Death> {
        self.pending.take()
    }
}

fn vanishes(stack: &ItemStack) -> bool {
    stack
        .components
        .as_ref()
        .and_then(|components| components.get(ENCHANTMENTS))
        .and_then(|enchantments| enchantments.get(VANISHING_CURSE))
        .and_then(serde_json::Value::as_i64)
        .is_some_and(|level| level > 0)
}

pub fn dropped_on_death(death: &Death, inventory: &mut Inventory) -> Vec<ItemStack> {
    if death.keep_inventory {
        return Vec::new();
    }
    death_drops(inventory, |stack| crate::guns::weapon_of(stack).is_some())
}

pub fn death_drops(
    inventory: &mut Inventory,
    destroyed: impl Fn(&ItemStack) -> bool,
) -> Vec<ItemStack> {
    let mut drops = Vec::new();
    for slot in (0..STORAGE_SLOTS).chain(EQUIPMENT_DROP_ORDER) {
        let Some(stack) = inventory.slots.get_mut(slot).and_then(Option::take) else {
            continue;
        };
        if !destroyed(&stack) && !vanishes(&stack) {
            drops.push(stack);
        }
    }
    let mut loose: Vec<ItemStack> = inventory
        .crafting
        .iter_mut()
        .chain(inventory.workbench.iter_mut())
        .filter_map(Option::take)
        .collect();
    loose.extend(inventory.cursor.take());
    loose.extend(inventory.take_pending_drops());
    drops.extend(
        loose
            .into_iter()
            .filter(|stack| !destroyed(stack) && !vanishes(stack)),
    );
    drops
}

#[cfg(test)]
mod tests {
    use super::*;

    fn language() -> HashMap<String, String> {
        [
            ("death.attack.generic", "%1$s died"),
            ("death.attack.onFire", "%1$s burned to death"),
            ("death.attack.lava", "%1$s tried to swim in lava"),
            (
                "death.attack.lava.player",
                "%1$s tried to swim in lava to escape %2$s",
            ),
            ("death.attack.arrow", "%1$s was shot by %2$s"),
            ("death.attack.mob", "%1$s was slain by %2$s"),
            ("death.attack.explosion.player", "%1$s was blown up by %2$s"),
            ("death.attack.fall", "%1$s hit the ground too hard"),
            ("death.attack.drown", "%1$s drowned"),
            ("death.attack.starve", "%1$s starved to death"),
            ("death.attack.cactus", "%1$s was pricked to death"),
            (
                "death.attack.hotFloor",
                "%1$s discovered the floor was lava",
            ),
            (
                "death.attack.hotFloor.player",
                "%1$s walked into the danger zone due to %2$s",
            ),
            ("death.attack.magic", "%1$s was killed by magic"),
            ("death.attack.wither", "%1$s withered away"),
            ("death.attack.freeze", "%1$s froze to death"),
            ("death.attack.freeze.player", "%1$s was frozen to death by %2$s"),
            ("death.attack.outOfWorld", "%1$s fell out of the world"),
            ("death.attack.flyIntoWall", "%1$s experienced kinetic energy"),
            (
                "death.attack.flyIntoWall.player",
                "%1$s experienced kinetic energy while trying to escape %2$s",
            ),
            ("death.fell.accident.generic", "%1$s fell from a high place"),
            ("death.fell.accident.ladder", "%1$s fell off a ladder"),
            ("death.fell.accident.vines", "%1$s fell off some vines"),
            (
                "death.fell.accident.scaffolding",
                "%1$s fell off scaffolding",
            ),
            ("death.fell.assist", "%1$s was doomed to fall by %2$s"),
            ("entity.minecraft.skeleton", "Skeleton"),
            ("entity.minecraft.zombie", "Zombie"),
            ("entity.minecraft.creeper", "Creeper"),
            ("entity.minecraft.ghast", "Ghast"),
            ("entity.minecraft.blaze", "Blaze"),
            ("entity.minecraft.piglin", "Piglin"),
            ("entity.minecraft.piglin_brute", "Piglin Brute"),
            ("entity.minecraft.shulker", "Shulker"),
            ("entity.minecraft.wither", "Wither"),
            ("death.attack.witherSkull", "%1$s was shot by a skull from %2$s"),
            ("entity.minecraft.drowned", "Drowned"),
            ("entity.minecraft.llama", "Llama"),
            ("death.attack.trident", "%1$s was impaled by %2$s"),
            ("death.attack.fireball", "%1$s was fireballed by %2$s"),
            ("entity.minecraft.arrow", "Arrow"),
            ("block.minecraft.magma_block", "Magma Block"),
            ("block.minecraft.lava", "Lava"),
            (
                "death.attack.badRespawnPoint.message",
                "%1$s was killed by %2$s",
            ),
            (
                "death.attack.badRespawnPoint.link",
                "Intentional Game Design",
            ),
        ]
        .into_iter()
        .map(|(key, text)| (key.to_owned(), text.to_owned()))
        .collect()
    }

    fn skeleton_arrow() -> DamageCause {
        DamageCause::mob(MobAttack::Arrow, Some(7), Some("minecraft:skeleton"))
    }

    fn zombie_bite() -> DamageCause {
        DamageCause::mob(MobAttack::Melee, Some(9), Some("minecraft:zombie"))
    }

    fn line(tracker: &CombatTracker) -> DeathLine {
        tracker.death_line(&language(), |_| true, |_| None)
    }

    fn says(message: &str, killer: &str) -> DeathLine {
        DeathLine {
            message: message.to_owned(),
            killer: killer.to_owned(),
        }
    }

    #[test]
    fn the_translatable_line_reads_as_the_resolved_one() {
        let language = language();
        let lang = |key: &str| language.get(key).cloned();
        let mut bitten = CombatTracker::default();
        bitten.record(1, zombie_bite(), 3.0, None, 0.0);
        let mut doomed = CombatTracker::default();
        doomed.record(1, skeleton_arrow(), 2.0, None, 0.0);
        doomed.record(
            2,
            DamageCause::Fall,
            20.0,
            Some(FallLocation::Generic),
            12.0,
        );
        let mut bed = CombatTracker::default();
        bed.record(1, DamageCause::BadRespawnPoint, 20.0, None, 0.0);
        let mut escaping = CombatTracker::default();
        escaping.hurt_by(1, zombie_bite());
        escaping.record(1, DamageCause::Hazard(Hazard::Lava), 4.0, None, 0.0);
        for tracker in [CombatTracker::default(), bitten, doomed, bed, escaping] {
            let text = tracker.death_text("Steve", |_| true, |_| None).resolve(&lang).text;
            assert_eq!(text, line(&tracker).message.replace(VICTIM, "Steve"));
            assert!(!text.contains('%'), "{text}");
        }
    }

    #[test]
    fn a_bed_blast_is_intentional_game_design_even_after_a_mob_hit() {
        let mut tracker = CombatTracker::default();
        tracker.hurt_by(1, zombie_bite());
        tracker.record(1, zombie_bite(), 3.0, None, 0.0);
        tracker.record(2, DamageCause::BadRespawnPoint, 20.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says(
                "%1$s was killed by [Intentional Game Design]",
                "[Intentional Game Design]"
            )
        );
    }

    #[test]
    fn no_damage_dies_generically() {
        assert_eq!(line(&CombatTracker::default()), says("%1$s died", "died"));
    }

    #[test]
    fn wither_withers_away_and_only_a_wither_skeleton_s_blow_brings_it() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::Wither, 1.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s withered away", "withered away"));
        let blow = DamageCause::mob(MobAttack::Melee, Some(4), Some("minecraft:wither_skeleton"));
        let normal = minecraftoss_player::survival::Difficulty::Normal;
        assert_eq!(
            blow.melee_effect(normal),
            Some((minecraftoss_player::survival::EffectKind::Wither, 200))
        );
        assert_eq!(zombie_bite().melee_effect(normal), None);
        assert_eq!(skeleton_arrow().melee_effect(normal), None);
    }

    #[test]
    fn a_cave_spider_bite_poisons_for_seven_seconds_on_normal_and_fifteen_on_hard() {
        use minecraftoss_player::survival::{Difficulty, EffectKind};
        let bite = DamageCause::mob(MobAttack::Melee, Some(4), Some("minecraft:cave_spider"));
        let poison = [Difficulty::Peaceful, Difficulty::Easy, Difficulty::Normal, Difficulty::Hard].map(|difficulty| bite.melee_effect(difficulty));
        assert_eq!(poison, [None, None, Some((EffectKind::Poison, 140)), Some((EffectKind::Poison, 300))]);
        let spider = DamageCause::mob(MobAttack::Melee, Some(4), Some("minecraft:spider"));
        assert_eq!(spider.melee_effect(Difficulty::Hard), None);
    }

    #[test]
    fn freezing_to_death_has_the_vanilla_message() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::Hazard(Hazard::Freeze), 1.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s froze to death", "froze to death"));
        assert_eq!(DamageCause::Hazard(Hazard::Freeze).damage_type(), "minecraft:freeze");
        tracker.hurt_by(2, zombie_bite());
        tracker.record(2, DamageCause::Hazard(Hazard::Freeze), 1.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s was frozen to death by Zombie", "Zombie")
        );
    }

    #[test]
    fn burning_burns_to_death() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::Hazard(Hazard::OnFire), 1.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s burned to death", "burned to death")
        );
    }

    #[test]
    fn flying_into_a_wall_is_kinetic_energy_even_after_a_long_fall() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::FlyIntoWall, 6.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s experienced kinetic energy", "experienced kinetic energy")
        );
        tracker.hurt_by(2, zombie_bite());
        tracker.record(2, DamageCause::FlyIntoWall, 6.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says(
                "%1$s experienced kinetic energy while trying to escape Zombie",
                "Zombie"
            )
        );
    }

    #[test]
    fn the_void_kills_out_of_the_world_and_counts_as_the_highest_fall() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::OutOfWorld, 4.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s fell out of the world", "fell out of the world")
        );
        tracker.record(2, DamageCause::Fall, 3.0, None, 6.0);
        assert_eq!(
            line(&tracker),
            says("%1$s fell from a high place", "fell from a high place")
        );
    }

    #[test]
    fn a_skeleton_arrow_names_the_skeleton() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, skeleton_arrow(), 4.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s was shot by Skeleton", "Skeleton")
        );
    }

    #[test]
    fn a_piglin_shoots_and_a_brute_slays_by_name() {
        let mut tracker = CombatTracker::default();
        let bolt = DamageCause::mob(MobAttack::Arrow, Some(3), Some("minecraft:piglin"));
        tracker.record(1, bolt, 4.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was shot by Piglin", "Piglin"));
        let mut tracker = CombatTracker::default();
        let axe = DamageCause::mob(MobAttack::Melee, Some(4), Some("minecraft:piglin_brute"));
        tracker.record(1, axe, 13.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s was slain by Piglin Brute", "Piglin Brute")
        );
    }

    #[test]
    fn an_arrow_whose_shooter_is_gone_shoots_by_arrow_without_kill_credit() {
        let mut tracker = CombatTracker::default();
        let arrow = DamageCause::mob(MobAttack::Arrow, Some(7), None);
        tracker.hurt_by(1, arrow);
        tracker.record(1, arrow, 4.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was shot by Arrow", "Arrow"));
        tracker.record(2, DamageCause::Hazard(Hazard::Lava), 4.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s tried to swim in lava", "Lava"));
    }

    #[test]
    fn a_ghast_fireball_fireballs_the_player_and_its_blast_blows_them_up() {
        let mut tracker = CombatTracker::default();
        let direct = DamageCause::mob(MobAttack::Fireball, Some(5), Some("minecraft:ghast"));
        assert!(direct.burns());
        tracker.record(1, direct, 6.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was fireballed by Ghast", "Ghast"));
        let blast = DamageCause::mob(MobAttack::Explosion, Some(5), Some("minecraft:ghast"));
        assert!(!blast.burns());
        tracker.record(1, blast, 9.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was blown up by Ghast", "Ghast"));
        assert_eq!(DamageCause::mob(MobAttack::Fireball, None, None), DamageCause::Generic);
        let mut tracker = CombatTracker::default();
        let small = DamageCause::mob(MobAttack::SmallFireball, Some(6), Some("minecraft:blaze"));
        assert_eq!((small.burns(), small.ignites()), (true, Some(100)));
        tracker.record(1, small, 5.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was fireballed by Blaze", "Blaze"));
    }

    #[test]
    fn a_shulker_bullet_slays_the_player_by_the_shulker() {
        let mut tracker = CombatTracker::default();
        let bullet = DamageCause::mob(MobAttack::MobProjectile, Some(4), Some("minecraft:shulker"));
        assert_eq!(bullet.damage_type(), "minecraft:mob_projectile");
        assert!(!bullet.burns());
        tracker.record(1, bullet, 4.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was slain by Shulker", "Shulker"));
        assert_eq!(DamageCause::mob(MobAttack::MobProjectile, None, None), DamageCause::Generic);
    }

    #[test]
    fn a_wither_skull_shoots_the_player_by_the_wither() {
        let mut tracker = CombatTracker::default();
        let skull = DamageCause::mob(MobAttack::WitherSkull, Some(4), Some("minecraft:wither"));
        assert_eq!((skull.damage_type(), skull.message_id()), ("minecraft:wither_skull", "witherSkull"));
        assert!(!skull.burns() && !skull.is_explosion());
        tracker.record(1, skull, 8.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was shot by a skull from Wither", "Wither"));
        assert_eq!(DamageCause::mob(MobAttack::WitherSkull, None, None), DamageCause::Generic);
    }

    #[test]
    fn a_drowned_s_trident_impales_the_player() {
        let mut tracker = CombatTracker::default();
        let trident = DamageCause::mob(MobAttack::Trident, Some(6), Some("minecraft:drowned"));
        assert_eq!(trident.damage_type(), "minecraft:trident");
        tracker.record(1, trident, 8.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was impaled by Drowned", "Drowned"));
        assert_eq!(DamageCause::mob(MobAttack::Trident, None, None), DamageCause::Generic);
    }

    #[test]
    fn a_llama_s_spit_slays_the_player_by_the_llama() {
        let mut tracker = CombatTracker::default();
        let spit = DamageCause::mob(MobAttack::Spit, Some(5), Some("minecraft:llama"));
        assert_eq!(spit.damage_type(), "minecraft:spit");
        tracker.record(1, spit, 1.0, None, 0.0);
        assert_eq!(line(&tracker), says("%1$s was slain by Llama", "Llama"));
        assert_eq!(DamageCause::mob(MobAttack::Spit, None, None), DamageCause::Generic);
    }

    #[test]
    fn a_villager_s_firework_makes_the_player_go_off_with_a_bang() {
        let mut lang = language();
        lang.insert("death.attack.fireworks".to_owned(), "%1$s went off with a bang".to_owned());
        let bang = DamageCause::mob(MobAttack::Fireworks, Some(9), Some("minecraft:villager"));
        assert_eq!(bang.damage_type(), "minecraft:fireworks");
        assert!(bang.is_explosion() && DamageCause::Fireworks.is_explosion());
        let mut tracker = CombatTracker::default();
        tracker.record(1, bang, 4.0, None, 0.0);
        assert_eq!(tracker.death_line(&lang, |_| true, |_| None).message, "%1$s went off with a bang");
        let mut sourceless = CombatTracker::default();
        sourceless.record(1, DamageCause::Fireworks, 4.0, None, 0.0);
        assert_eq!(sourceless.death_line(&lang, |_| true, |_| None).message, "%1$s went off with a bang");
        let mut fighting = CombatTracker::default();
        fighting.hurt_by(1, zombie_bite());
        fighting.hurt_by(2, DamageCause::Player { client: 0 });
        fighting.record(3, DamageCause::Fireworks, 4.0, None, 0.0);
        let host = |client: u32| (client == 0).then(|| Killer { name: "Host".to_owned(), weapon: None });
        assert_eq!(fighting.death_line(&lang, |_| true, host).message, "%1$s went off with a bang", "the rocket is the direct source, so no one is credited");
    }

    #[test]
    fn a_creeper_that_is_gone_still_blows_the_player_up() {
        let mut tracker = CombatTracker::default();
        let blast = DamageCause::mob(MobAttack::Explosion, Some(3), None);
        tracker.record(1, blast, 20.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s was blown up by Creeper", "Creeper")
        );
    }

    #[test]
    fn lava_after_a_zombie_hit_escapes_the_zombie() {
        let mut tracker = CombatTracker::default();
        tracker.record(10, zombie_bite(), 3.0, None, 0.0);
        tracker.hurt_by(10, zombie_bite());
        tracker.record(30, DamageCause::Hazard(Hazard::Lava), 4.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s tried to swim in lava to escape Zombie", "Zombie")
        );
        assert_eq!(
            tracker.death_line(&language(), |_| false, |_| None),
            says("%1$s tried to swim in lava", "Lava")
        );
    }

    #[test]
    fn the_kill_credit_runs_out_after_a_hundred_ticks() {
        let mut tracker = CombatTracker::default();
        tracker.hurt_by(10, zombie_bite());
        tracker.tick(110);
        tracker.record(110, DamageCause::Hazard(Hazard::HotFloor), 1.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s walked into the danger zone due to Zombie", "Zombie")
        );
        tracker.tick(111);
        assert_eq!(
            line(&tracker),
            says("%1$s discovered the floor was lava", "Magma Block")
        );
    }

    #[test]
    fn a_fall_past_five_blocks_names_where_it_started() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::Fall, 4.0, Some(FallLocation::Ladder), 7.0);
        assert_eq!(
            line(&tracker),
            says("%1$s fell off a ladder", "fell off a ladder")
        );
        let mut plain = CombatTracker::default();
        plain.record(1, DamageCause::Fall, 4.0, None, 7.0);
        assert_eq!(
            line(&plain),
            says("%1$s fell from a high place", "fell from a high place")
        );
    }

    #[test]
    fn a_fall_of_five_blocks_or_less_hits_the_ground_too_hard() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::Fall, 2.0, Some(FallLocation::Vines), 5.0);
        assert_eq!(
            line(&tracker),
            says("%1$s hit the ground too hard", "hit the ground too hard")
        );
    }

    #[test]
    fn an_arrow_before_a_long_fall_dooms_the_player_to_fall() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, skeleton_arrow(), 2.0, Some(FallLocation::Ladder), 0.0);
        tracker.record(30, DamageCause::Fall, 9.0, None, 12.0);
        assert_eq!(
            line(&tracker),
            says("%1$s was doomed to fall by Skeleton", "Skeleton")
        );
    }

    #[test]
    fn a_short_fall_after_a_hard_hit_on_scaffolding_falls_off_it() {
        let mut tracker = CombatTracker::default();
        tracker.record(
            1,
            DamageCause::Fall,
            6.0,
            Some(FallLocation::Scaffolding),
            0.5,
        );
        tracker.record(2, DamageCause::Fall, 1.0, None, 4.0);
        assert_eq!(
            line(&tracker),
            says("%1$s fell off scaffolding", "fell off scaffolding")
        );
    }

    #[test]
    fn an_ender_pearl_hits_the_ground_too_hard() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::EnderPearl, 5.0, None, 0.0);
        assert_eq!(
            line(&tracker),
            says("%1$s hit the ground too hard", "hit the ground too hard")
        );
    }

    #[test]
    fn old_entries_are_forgotten_without_combat() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, DamageCause::Fall, 4.0, None, 7.0);
        tracker.tick(100);
        tracker.record(100, DamageCause::Fall, 1.0, None, 1.0);
        assert_eq!(
            line(&tracker),
            says("%1$s fell from a high place", "fell from a high place")
        );
        let mut forgetful = CombatTracker::default();
        forgetful.record(1, DamageCause::Fall, 4.0, None, 7.0);
        forgetful.tick(120);
        forgetful.record(121, DamageCause::Fall, 1.0, None, 1.0);
        assert_eq!(
            line(&forgetful),
            says("%1$s hit the ground too hard", "hit the ground too hard")
        );
    }

    #[test]
    fn combat_keeps_entries_for_three_hundred_ticks() {
        let mut tracker = CombatTracker::default();
        tracker.record(1, skeleton_arrow(), 2.0, None, 0.0);
        tracker.tick(300);
        tracker.record(300, DamageCause::Fall, 9.0, None, 12.0);
        assert_eq!(
            line(&tracker),
            says("%1$s was doomed to fall by Skeleton", "Skeleton")
        );
    }

    #[test]
    fn climbables_name_their_fall() {
        assert_eq!(
            FallLocation::of_climbable("minecraft:ladder"),
            FallLocation::Ladder
        );
        assert_eq!(
            FallLocation::of_climbable("minecraft:oak_trapdoor"),
            FallLocation::Ladder
        );
        assert_eq!(
            FallLocation::of_climbable("minecraft:weeping_vines_plant"),
            FallLocation::WeepingVines
        );
        assert_eq!(
            FallLocation::of_climbable("minecraft:cave_vines"),
            FallLocation::OtherClimbable
        );
    }

    fn spot(x: f64) -> DeathSpot {
        DeathSpot {
            feet: [x, 64.0, 0.0],
            eye: [x, 65.62, 0.0],
        }
    }

    fn enchanted(id: &str, key: &str, enchantment: &str) -> ItemStack {
        let mut stack = ItemStack::new(id, 1);
        stack.components = Some(serde_json::json!({ key: { enchantment: 1 } }));
        stack
    }

    fn ids(stacks: &[ItemStack]) -> Vec<&str> {
        stacks.iter().map(|stack| stack.id.as_str()).collect()
    }

    #[test]
    fn death_experience_is_seven_per_level_up_to_one_hundred() {
        assert_eq!(death_experience(0, false), 0);
        assert_eq!(death_experience(1, false), 7);
        assert_eq!(death_experience(14, false), 98);
        assert_eq!(death_experience(15, false), 100);
        assert_eq!(death_experience(40, false), 100);
        assert_eq!(death_experience(u32::MAX, false), 100);
        assert_eq!(death_experience(30, true), 0);
    }

    #[test]
    fn death_drops_follow_vanilla_order_drop_guns_and_destroy_killstreaks_and_vanishing_items() {
        let killstreak = |stack: &ItemStack| stack.id.starts_with("iw4:weapon/");
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("iw4:weapon/5", 1));
        inventory.slots[1] = Some(ItemStack::new("iw4l:m4", 1));
        inventory.slots[2] = Some(ItemStack::new("iw4l:rifle_ammo", 40));
        inventory.slots[3] = Some(ItemStack::new("minecraft:dirt", 64));
        inventory.slots[9] = Some(ItemStack::new("iw4l:frag_grenade", 5));
        inventory.slots[20] = Some(enchanted(
            "minecraft:iron_sword",
            ENCHANTMENTS,
            VANISHING_CURSE,
        ));
        inventory.slots[36] = Some(ItemStack::new("minecraft:iron_boots", 1));
        inventory.slots[39] = Some(ItemStack::new("minecraft:iron_helmet", 1));
        inventory.slots[40] = Some(ItemStack::new("minecraft:shield", 1));
        inventory.crafting[1] = Some(ItemStack::new("minecraft:stick", 2));
        inventory.workbench[4] = Some(ItemStack::new("iw4:weapon/9", 1));
        inventory.cursor = Some(ItemStack::new("minecraft:apple", 3));
        let drops = death_drops(&mut inventory, killstreak);
        assert_eq!(
            ids(&drops),
            [
                "iw4l:m4",
                "iw4l:rifle_ammo",
                "minecraft:dirt",
                "iw4l:frag_grenade",
                "minecraft:shield",
                "minecraft:iron_boots",
                "minecraft:iron_helmet",
                "minecraft:stick",
                "minecraft:apple",
            ]
        );
        assert_eq!(drops[2].count, 64);
        assert!(inventory.slots.iter().all(Option::is_none));
        assert!(inventory.crafting.iter().all(Option::is_none));
        assert!(inventory.workbench.iter().all(Option::is_none));
        assert!(inventory.cursor.is_none());
    }

    #[test]
    fn grenade_slots_drop_after_the_saddle_and_keep_inventory_keeps_them() {
        let loaded = || {
            let mut inventory = Inventory::default();
            inventory.slots[3] = Some(ItemStack::new("minecraft:dirt", 4));
            inventory.slots[40] = Some(ItemStack::new("minecraft:shield", 1));
            inventory.slots[42] = Some(ItemStack::new("minecraft:saddle", 1));
            inventory.slots[43] = Some(ItemStack::new("iw4l:semtex", 32));
            inventory.slots[44] = Some(ItemStack::new("iw4l:flash_grenade", 31));
            inventory.cursor = Some(ItemStack::new("minecraft:apple", 1));
            inventory
        };
        let death = |keep_inventory| Death {
            spot: spot(0.0),
            experience: 0,
            keep_inventory,
        };
        let mut lost = loaded();
        let drops = dropped_on_death(&death(false), &mut lost);
        assert_eq!(
            ids(&drops),
            [
                "minecraft:dirt",
                "minecraft:shield",
                "minecraft:saddle",
                "iw4l:semtex",
                "iw4l:flash_grenade",
                "minecraft:apple",
            ]
        );
        assert_eq!((drops[3].count, drops[4].count), (32, 31));
        assert!(lost.slots.iter().all(Option::is_none));
        let mut kept = loaded();
        assert!(dropped_on_death(&death(true), &mut kept).is_empty());
        assert_eq!(kept.slots, loaded().slots);
    }

    #[test]
    fn a_cursed_enchanted_book_still_drops() {
        let mut inventory = Inventory::default();
        inventory.slots[5] = Some(enchanted(
            "minecraft:enchanted_book",
            "minecraft:stored_enchantments",
            VANISHING_CURSE,
        ));
        inventory.slots[6] = Some(enchanted(
            "minecraft:diamond_pickaxe",
            ENCHANTMENTS,
            "minecraft:silk_touch",
        ));
        let drops = death_drops(&mut inventory, |_| false);
        assert_eq!(
            ids(&drops),
            ["minecraft:enchanted_book", "minecraft:diamond_pickaxe"]
        );
        assert!(drops[1].components.is_some());
    }

    #[test]
    fn keep_inventory_keeps_guns_and_killstreaks_and_a_normal_death_drops_only_the_guns() {
        let loaded = || {
            let mut inventory = Inventory::default();
            let mut gun = ItemStack::new("iw4l:m4", 1);
            gun.components = Some(serde_json::json!({"minecraft:custom_data": {"iw4l": {"clip": 17}}}));
            inventory.slots[0] = Some(gun);
            inventory.slots[1] = Some(ItemStack::new("iw4:weapon/12", 1));
            inventory.slots[2] = Some(ItemStack::new("iw4l:rifle_ammo", 30));
            inventory
        };
        let death = |keep_inventory| Death {
            spot: spot(0.0),
            experience: 0,
            keep_inventory,
        };
        let mut kept = loaded();
        assert!(dropped_on_death(&death(true), &mut kept).is_empty());
        assert_eq!(kept.slots, loaded().slots);
        let mut lost = loaded();
        let drops = dropped_on_death(&death(false), &mut lost);
        assert_eq!(ids(&drops), ["iw4l:m4", "iw4l:rifle_ammo"]);
        assert_eq!(drops[0], loaded().slots[0].clone().unwrap());
        assert!(lost.slots.iter().all(Option::is_none));
    }

    #[test]
    fn a_death_is_seen_once_at_the_last_spot_and_never_at_intermission() {
        let mut watch = DeathWatch::default();
        watch.observe(false, false, 3, false);
        watch.stood(spot(9.0));
        watch.observe(true, false, 3, false);
        watch.stood(spot(1.0));
        watch.observe(true, false, 3, false);
        watch.stood(spot(2.0));
        watch.observe(false, true, 3, false);
        watch.stood(spot(5.0));
        watch.observe(false, true, 0, false);
        assert_eq!(
            watch.take(),
            Some(Death {
                spot: spot(2.0),
                experience: 21,
                keep_inventory: false,
            })
        );
        assert_eq!(watch.take(), None);
        watch.observe(true, false, 3, false);
        watch.stood(spot(3.0));
        watch.observe(false, false, 3, false);
        assert_eq!(watch.take(), None);
        watch.observe(false, false, 3, false);
        watch.observe(false, true, 3, false);
        assert_eq!(watch.take(), None);
    }

    #[test]
    fn only_dead_player_states_count_as_a_death() {
        use playerstate_iw4::{
            PM_TYPE_DEAD, PM_TYPE_DEAD_LINKED, PM_TYPE_INTERMISSION, PM_TYPE_LAST_STAND,
            PM_TYPE_NORMAL_LINKED, PM_TYPE_SPECTATOR,
        };
        assert_eq!(life_state(0), (true, false));
        assert_eq!(life_state(PM_TYPE_NORMAL_LINKED), (true, false));
        assert_eq!(life_state(PM_TYPE_LAST_STAND), (true, false));
        assert_eq!(life_state(PM_TYPE_DEAD), (false, true));
        assert_eq!(life_state(PM_TYPE_DEAD_LINKED), (false, true));
        assert_eq!(life_state(PM_TYPE_SPECTATOR), (false, false));
        assert_eq!(life_state(PM_TYPE_INTERMISSION), (false, false));
    }

    #[test]
    fn last_stand_then_death_drops() {
        let mut watch = DeathWatch::default();
        let observe = |watch: &mut DeathWatch, pm_type: i32| {
            let (in_world, died) = life_state(pm_type);
            watch.observe(in_world, died, 15, false);
        };
        observe(&mut watch, 0);
        watch.stood(spot(1.0));
        observe(&mut watch, playerstate_iw4::PM_TYPE_LAST_STAND);
        watch.stood(spot(4.0));
        observe(&mut watch, playerstate_iw4::PM_TYPE_DEAD);
        assert_eq!(watch.take().map(|death| death.spot), Some(spot(4.0)));
    }

    #[test]
    fn a_team_switch_suicide_drops_nothing_and_the_next_death_does() {
        let mut watch = DeathWatch::default();
        watch.switch_teams();
        watch.observe(true, false, 4, false);
        watch.stood(spot(1.0));
        watch.switch_teams();
        watch.observe(false, true, 4, false);
        assert_eq!(watch.take(), None);
        assert!(watch.switched_teams());
        watch.observe(false, false, 4, false);
        assert!(watch.switched_teams());
        watch.observe(true, false, 4, false);
        assert!(!watch.switched_teams());
        watch.stood(spot(2.0));
        watch.observe(false, true, 4, false);
        assert!(!watch.switched_teams());
        assert_eq!(watch.take().map(|death| death.spot), Some(spot(2.0)));
    }

    #[test]
    fn spectators_keep_their_items_and_experience_like_keep_inventory() {
        assert!(!keeps_belongings(false, GameMode::Survival));
        assert!(!keeps_belongings(false, GameMode::Creative));
        assert!(!keeps_belongings(false, GameMode::Adventure));
        assert!(keeps_belongings(false, GameMode::Spectator));
        assert!(keeps_belongings(true, GameMode::Survival));
        let mut watch = DeathWatch::default();
        let keep = keeps_belongings(false, GameMode::Spectator);
        watch.observe(true, false, 20, keep);
        watch.stood(spot(1.0));
        watch.observe(false, true, 20, keep);
        let death = watch.take().unwrap();
        assert!(death.keep_inventory);
        assert_eq!(death.experience, 0);
    }

    #[test]
    fn keep_inventory_deaths_carry_no_experience() {
        let mut watch = DeathWatch::default();
        watch.observe(true, false, 20, true);
        watch.stood(spot(1.0));
        watch.observe(false, true, 20, true);
        assert_eq!(
            watch.take(),
            Some(Death {
                spot: spot(1.0),
                experience: 0,
                keep_inventory: true,
            })
        );
    }

    #[test]
    fn explosions_are_the_vanilla_is_explosion_types() {
        assert!(DamageCause::mob(MobAttack::Explosion, Some(7), Some("minecraft:creeper")).is_explosion());
        let sourceless = DamageCause::mob(MobAttack::Explosion, None, None);
        assert_eq!(sourceless, DamageCause::Explosion, "TNT and crystals");
        assert!(sourceless.is_explosion());
        assert_eq!((sourceless.damage_type(), sourceless.message_id()), ("minecraft:explosion", "explosion"));
        assert_eq!(IS_EXPLOSION.len(), 4);
        assert!(DamageCause::BadRespawnPoint.is_explosion(), "beds and anchors");
        for cause in [
            DamageCause::Generic,
            DamageCause::Fall,
            DamageCause::Kill,
            DamageCause::OutOfWorld,
            DamageCause::Magic,
            DamageCause::Hazard(Hazard::Lava),
            DamageCause::mob(MobAttack::Melee, Some(7), Some("minecraft:zombie")),
            DamageCause::mob(MobAttack::Arrow, Some(7), Some("minecraft:skeleton")),
            DamageCause::mob(MobAttack::Fireball, Some(7), Some("minecraft:ghast")),
        ] {
            assert!(!cause.is_explosion(), "{cause:?}");
        }
    }

    fn host(weapon: Option<&'static str>) -> impl Fn(u32) -> Option<Killer> {
        move |client| {
            (client == 0).then(|| Killer {
                name: "Host".to_owned(),
                weapon: weapon.map(str::to_owned),
            })
        }
    }

    fn with_player_lines() -> HashMap<String, String> {
        let mut language = language();
        for (key, text) in [
            ("death.attack.player", "%1$s was slain by %2$s"),
            (
                "death.attack.player.item",
                "%1$s was slain by %2$s using %3$s",
            ),
            (
                "death.fell.assist.item",
                "%1$s was doomed to fall by %2$s using %3$s",
            ),
        ] {
            language.insert(key.to_owned(), text.to_owned());
        }
        language
    }

    #[test]
    fn an_explosion_with_no_attacker_blew_up_even_right_after_a_hit() {
        let mut language = with_player_lines();
        language.insert("death.attack.explosion".to_owned(), "%1$s blew up".to_owned());
        let lang = |key: &str| language.get(key).cloned();
        let blow = DamageCause::Player { client: 0 };
        let blast = DamageCause::mob(MobAttack::Explosion, None, None);
        let mut tracker = CombatTracker::default();
        tracker.hurt_by(1, zombie_bite());
        tracker.record(1, zombie_bite(), 3.0, None, 0.0);
        tracker.hurt_by(2, blow);
        tracker.record(2, blow, 2.0, None, 0.0);
        tracker.record(3, blast, 30.0, None, 0.0);
        assert_eq!(
            tracker.death_line(&language, |_| true, host(None)),
            says("%1$s blew up", "blew up")
        );
        let text = tracker.death_text("Friend", |_| true, host(None));
        assert_eq!(text.resolve(&lang).text, "Friend blew up");
        let mut escaping = CombatTracker::default();
        escaping.hurt_by(1, blow);
        escaping.record(1, DamageCause::Hazard(Hazard::Lava), 4.0, None, 0.0);
        assert_eq!(
            escaping.death_line(&language, |_| true, host(None)).message,
            "%1$s tried to swim in lava to escape Host",
            "a death with no direct source still names the last attacker"
        );
    }

    #[test]
    fn a_players_blast_blows_up_by_name_and_counts_as_their_hit() {
        let mut language = with_player_lines();
        for (key, text) in [
            ("death.attack.explosion", "%1$s blew up"),
            (
                "death.attack.explosion.player.item",
                "%1$s was blown up by %2$s using %3$s",
            ),
        ] {
            language.insert(key.to_owned(), text.to_owned());
        }
        let lang = |key: &str| language.get(key).cloned();
        let blast = DamageCause::PlayerExplosion { client: 0 };
        assert_eq!(
            (blast.damage_type(), blast.message_id()),
            ("minecraft:player_explosion", "explosion.player")
        );
        assert!(blast.is_explosion());
        let mut tracker = CombatTracker::default();
        tracker.hurt_by(1, blast);
        tracker.record(1, blast, 30.0, None, 0.0);
        assert_eq!(
            tracker.death_line(&language, |_| true, host(None)),
            says("%1$s was blown up by Host", "Host")
        );
        let named = tracker.death_text("Friend", |_| true, host(Some("Excalibur")));
        assert_eq!(
            named.resolve(&lang).text,
            "Friend was blown up by Host using [Excalibur]"
        );
        assert_eq!(
            tracker.death_line(&language, |_| true, |_| None).message,
            "%1$s blew up",
            "an igniter who left"
        );
        let mut thrown = CombatTracker::default();
        thrown.hurt_by(1, blast);
        thrown.record(1, blast, 2.0, None, 0.0);
        thrown.record(
            2,
            DamageCause::Fall,
            20.0,
            Some(FallLocation::Generic),
            12.0,
        );
        assert_eq!(
            thrown.death_line(&language, |_| true, host(None)).message,
            "%1$s was doomed to fall by Host"
        );
        let mut escaping = CombatTracker::default();
        escaping.hurt_by(1, blast);
        escaping.record(1, DamageCause::Hazard(Hazard::Lava), 4.0, None, 0.0);
        assert_eq!(
            escaping.death_line(&language, |_| true, host(None)).message,
            "%1$s tried to swim in lava to escape Host"
        );
    }

    #[test]
    fn a_players_blow_slays_by_name_and_names_only_a_custom_named_weapon() {
        let language = with_player_lines();
        let lang = |key: &str| language.get(key).cloned();
        let blow = DamageCause::Player { client: 0 };
        assert_eq!(blow.damage_type(), "minecraft:player_attack");
        let mut tracker = CombatTracker::default();
        tracker.hurt_by(1, blow);
        tracker.record(1, blow, 6.0, None, 0.0);
        assert_eq!(
            tracker.death_line(&language, |_| true, host(None)),
            says("%1$s was slain by Host", "Host")
        );
        let text = tracker.death_text("Friend", |_| true, host(None));
        assert_eq!(text.resolve(&lang).text, "Friend was slain by Host");
        let named = tracker.death_text("Friend", |_| true, host(Some("Excalibur")));
        assert_eq!(
            named.resolve(&lang).text,
            "Friend was slain by Host using [Excalibur]"
        );
        assert_eq!(
            tracker
                .death_line(&language, |_| true, host(Some("Excalibur")))
                .message,
            "%1$s was slain by Host using [Excalibur]"
        );
        assert_eq!(
            tracker.death_line(&language, |_| true, |_| None).message,
            "%1$s died",
            "a killer who left"
        );
    }

    #[test]
    fn a_player_dooms_a_fall_and_is_escaped_into_lava_for_five_seconds() {
        let language = with_player_lines();
        let lang = |key: &str| language.get(key).cloned();
        let blow = DamageCause::Player { client: 0 };
        let mut doomed = CombatTracker::default();
        doomed.hurt_by(1, blow);
        doomed.record(1, blow, 2.0, None, 0.0);
        doomed.record(
            2,
            DamageCause::Fall,
            20.0,
            Some(FallLocation::Generic),
            12.0,
        );
        let text = doomed.death_text("Friend", |_| true, host(Some("Excalibur")));
        assert_eq!(
            text.resolve(&lang).text,
            "Friend was doomed to fall by Host using [Excalibur]"
        );
        assert_eq!(
            doomed.death_line(&language, |_| true, host(None)).message,
            "%1$s was doomed to fall by Host"
        );
        let mut escaping = CombatTracker::default();
        escaping.hurt_by(1, blow);
        escaping.record(1, DamageCause::Hazard(Hazard::Lava), 4.0, None, 0.0);
        let text = escaping.death_text("Friend", |_| true, host(Some("Excalibur")));
        assert_eq!(
            text.resolve(&lang).text,
            "Friend tried to swim in lava to escape Host"
        );
        escaping.tick(102);
        assert_eq!(
            escaping.death_line(&language, |_| true, host(None)).message,
            "%1$s tried to swim in lava"
        );
        let mut bitten = CombatTracker::default();
        bitten.hurt_by(1, blow);
        bitten.record(1, blow, 2.0, None, 0.0);
        bitten.record(2, zombie_bite(), 3.0, None, 0.0);
        assert_eq!(
            bitten.death_line(&language, |_| true, host(None)),
            says("%1$s was slain by Zombie", "Zombie"),
            "the mob that struck last is named"
        );
    }
}
