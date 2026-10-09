use std::collections::HashMap;

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
const OWNERLESS_ARROW: &str = "minecraft:arrow";
const DEATH_EXPERIENCE_PER_LEVEL: u32 = 7;
const MAX_DEATH_EXPERIENCE: u32 = 100;
const STORAGE_SLOTS: usize = 36;
const EQUIPMENT_DROP_ORDER: [usize; 7] = [40, 36, 37, 38, 39, 41, 42];
const ENCHANTMENTS: &str = "minecraft:enchantments";
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageCause {
    Generic,
    Hazard(Hazard),
    Fall,
    EnderPearl,
    Drown,
    Starve,
    Magic,
    Kill,
    OutOfWorld,
    Mob {
        id: u64,
        type_id: &'static str,
        attack: MobAttack,
    },
}

impl DamageCause {
    pub fn mob(
        attack: MobAttack,
        source: Option<u64>,
        type_id: Option<&'static str>,
    ) -> Self {
        let type_id = type_id.or(match attack {
            MobAttack::Explosion => Some("minecraft:creeper"),
            MobAttack::Arrow => Some(OWNERLESS_ARROW),
            MobAttack::Melee => None,
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
            _ => Self::Generic,
        }
    }

    fn living_attacker(self) -> Option<(u64, &'static str)> {
        match self {
            Self::Mob { id, type_id, .. } if type_id != OWNERLESS_ARROW => Some((id, type_id)),
            _ => None,
        }
    }

    fn message_id(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Hazard(Hazard::InFire | Hazard::Campfire) => "inFire",
            Self::Hazard(Hazard::Lava) => "lava",
            Self::Hazard(Hazard::Cactus) => "cactus",
            Self::Hazard(Hazard::HotFloor) => "hotFloor",
            Self::Hazard(Hazard::SweetBerryBush) => "sweetBerryBush",
            Self::Hazard(Hazard::OnFire) => "onFire",
            Self::Hazard(Hazard::InWall) => "inWall",
            Self::Hazard(Hazard::Cramming) => "cramming",
            Self::Fall | Self::EnderPearl => "fall",
            Self::Drown => "drown",
            Self::Starve => "starve",
            Self::Magic => "magic",
            Self::Kill => "genericKill",
            Self::OutOfWorld => "outOfWorld",
            Self::Mob {
                attack: MobAttack::Melee,
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
        }
    }

    fn is_fall(self) -> bool {
        matches!(self, Self::Fall | Self::EnderPearl)
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeathLine {
    pub message: String,
    pub killer: String,
}

struct DeathKey {
    key: String,
    attacker: Option<&'static str>,
    cause: Option<DamageCause>,
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
        if cause.living_attacker().is_some() {
            self.in_combat = true;
        }
    }

    pub fn hurt_by(&mut self, tick: u64, cause: DamageCause) {
        if let Some((id, type_id)) = cause.living_attacker() {
            self.hurt_by = Some((id, type_id, tick));
        }
    }

    pub fn tick(&mut self, tick: u64) {
        if self
            .hurt_by
            .is_some_and(|(.., at)| tick.saturating_sub(at) > KILL_CREDIT_TICKS)
        {
            self.hurt_by = None;
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

    fn death_key(&self, present: impl Fn(u64) -> bool) -> DeathKey {
        let Some(last) = self.entries.last() else {
            return DeathKey {
                key: "death.attack.generic".to_owned(),
                attacker: None,
                cause: None,
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
                };
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
            };
        }
        let key = format!("death.attack.{}", last.cause.message_id());
        if let Some(attacker) = last.cause.entity() {
            return DeathKey {
                key,
                attacker: Some(attacker),
                cause: Some(last.cause),
            };
        }
        match self.hurt_by.filter(|(id, ..)| present(*id)) {
            Some((_, credit, _)) => DeathKey {
                key: format!("{key}.player"),
                attacker: Some(credit),
                cause: Some(last.cause),
            },
            None => DeathKey {
                key,
                attacker: None,
                cause: Some(last.cause),
            },
        }
    }

    pub fn death_line(
        &self,
        language: &HashMap<String, String>,
        present: impl Fn(u64) -> bool,
    ) -> DeathLine {
        let death = self.death_key(present);
        let template = language
            .get(&death.key)
            .cloned()
            .unwrap_or_else(|| death.key.clone());
        let attacker = death.attacker.map(|type_id| entity_name(language, type_id));
        let message = attacker
            .as_deref()
            .map_or_else(|| template.clone(), |name| template.replace(ATTACKER, name));
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

pub fn death_drops(
    inventory: &mut Inventory,
    stays: impl Fn(&ItemStack) -> bool,
) -> Vec<ItemStack> {
    let mut drops = Vec::new();
    for slot in (0..STORAGE_SLOTS).chain(EQUIPMENT_DROP_ORDER) {
        let Some(stack) = inventory.slots.get_mut(slot).and_then(Option::take) else {
            continue;
        };
        if stays(&stack) {
            inventory.slots[slot] = Some(stack);
        } else if !vanishes(&stack) {
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
            .filter(|stack| !stays(stack) && !vanishes(stack)),
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
            ("death.attack.outOfWorld", "%1$s fell out of the world"),
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
            ("entity.minecraft.arrow", "Arrow"),
            ("block.minecraft.magma_block", "Magma Block"),
            ("block.minecraft.lava", "Lava"),
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
        tracker.death_line(&language(), |_| true)
    }

    fn says(message: &str, killer: &str) -> DeathLine {
        DeathLine {
            message: message.to_owned(),
            killer: killer.to_owned(),
        }
    }

    #[test]
    fn no_damage_dies_generically() {
        assert_eq!(line(&CombatTracker::default()), says("%1$s died", "died"));
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
            tracker.death_line(&language(), |_| false),
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
    fn death_drops_follow_vanilla_order_keep_guns_and_destroy_vanishing_items() {
        let gun = |stack: &ItemStack| stack.id.starts_with("iw4:weapon/");
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("iw4:weapon/5", 1));
        inventory.slots[3] = Some(ItemStack::new("minecraft:dirt", 64));
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
        let drops = death_drops(&mut inventory, gun);
        assert_eq!(
            ids(&drops),
            [
                "minecraft:dirt",
                "minecraft:shield",
                "minecraft:iron_boots",
                "minecraft:iron_helmet",
                "minecraft:stick",
                "minecraft:apple",
            ]
        );
        assert_eq!(drops[0].count, 64);
        assert_eq!(
            inventory.slots[0].as_ref().map(|stack| stack.id.as_str()),
            Some("iw4:weapon/5")
        );
        assert!(inventory.slots[1..].iter().all(Option::is_none));
        assert!(inventory.crafting.iter().all(Option::is_none));
        assert!(inventory.workbench.iter().all(Option::is_none));
        assert!(inventory.cursor.is_none());
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
}
