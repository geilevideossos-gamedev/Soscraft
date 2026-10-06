use std::collections::HashMap;

use minecraftoss_entities::inside_blocks::Hazard;

const DAMAGE_STATUS_TICKS: u64 = 100;
const COMBAT_STATUS_TICKS: u64 = 300;
const STATUS_RECHECK_TICKS: u64 = 20;
const KILL_CREDIT_TICKS: u64 = 100;
const SIGNIFICANT_FALL: f32 = 5.0;
const VICTIM: &str = "%1$s";
const ATTACKER: &str = "%2$s";
const OWNERLESS_ARROW: &str = "minecraft:arrow";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FallLocation {
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
    pub(crate) fn of_climbable(block: &str) -> Self {
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
pub(crate) enum MobAttack {
    Melee,
    Arrow,
    Explosion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DamageCause {
    Generic,
    Hazard(Hazard),
    Fall,
    EnderPearl,
    Drown,
    Starve,
    Magic,
    Mob {
        id: u64,
        type_id: &'static str,
        attack: MobAttack,
    },
}

impl DamageCause {
    pub(crate) fn mob(
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
pub(crate) struct CombatTracker {
    entries: Vec<CombatEntry>,
    last_damage: u64,
    in_combat: bool,
    taking_damage: bool,
    hurt_by: Option<(u64, &'static str, u64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeathLine {
    pub(crate) message: String,
    pub(crate) killer: String,
}

struct DeathKey {
    key: String,
    attacker: Option<&'static str>,
    cause: Option<DamageCause>,
}

impl CombatTracker {
    pub(crate) fn record(
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

    pub(crate) fn hurt_by(&mut self, tick: u64, cause: DamageCause) {
        if let Some((id, type_id)) = cause.living_attacker() {
            self.hurt_by = Some((id, type_id, tick));
        }
    }

    pub(crate) fn tick(&mut self, tick: u64) {
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
            if entry.cause.is_fall()
                && entry.fall_distance > 0.0
                && fall.is_none_or(|(_, farthest)| entry.fall_distance > farthest)
            {
                let before = index.checked_sub(1).map_or(entry, |i| &self.entries[i]);
                fall = Some((before, entry.fall_distance));
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
            if fall.cause.is_fall() {
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

    pub(crate) fn death_line(
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
}
