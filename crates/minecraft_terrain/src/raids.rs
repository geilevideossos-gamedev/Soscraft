use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::raid::{
    MAX_NO_ACTION_TIME, RAID_REMOVAL_THRESHOLD_SQR, RaidView, dist_sqr, nearby_raid,
};
use minecraftoss_entities::world::{RaiderMut, RaiderView};
use minecraftoss_player::Pos;
use minecraftoss_player::rng::LegacyRandom;
use std::collections::{BTreeMap, BTreeSet};

pub const RAIDS_FILE: &str = "raids.dat";
pub const HORN_SOUND: &str = "minecraft:event.raid.horn";
pub const RAID_OMEN_TICKS: u32 = 600;
pub const HERO_TICKS: u32 = 48000;
pub const MAX_RAID_OMEN_LEVEL: i32 = 5;
pub const PRE_RAID_TICKS: i32 = 300;
pub const RAID_TIMEOUT_TICKS: i64 = 48000;
pub const POST_RAID_TICKS: i32 = 40;
pub const MAX_CELEBRATION_TICKS: i32 = 600;
pub const OUTSIDE_RAID_BOUNDS_TIMEOUT: i32 = 30;
pub const NUM_SPAWN_ATTEMPTS: i32 = 5;
pub const VILLAGE_SEARCH_SECTIONS: i32 = 2;
pub const ALLOW_SPAWNING_WITHIN_VILLAGE_SECONDS: i32 = 7;
pub const LOW_MOB_THRESHOLD: usize = 2;
pub const VILLAGE_POI_RANGE: i32 = 64;
pub const HORN_DISTANCE: f64 = 13.0;
pub const HORN_RANGE: f64 = 64.0;
pub const HORN_VOLUME: f32 = 64.0;
pub const OLD_RAIDER_TICKS: i32 = 600;
pub const SAVE_INTERVAL: i32 = 200;
pub const RAID_KEY: &str = "event.minecraft.raid";
pub const RAIDERS_REMAINING_KEY: &str = "event.minecraft.raid.raiders_remaining";
pub const VICTORY_KEY: &str = "event.minecraft.raid.victory.full";
pub const DEFEAT_KEY: &str = "event.minecraft.raid.defeat.full";
pub const FIRST_ID: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaiderType {
    Vindicator,
    Evoker,
    Pillager,
    Witch,
    Ravager,
}

impl RaiderType {
    pub const VALUES: [RaiderType; 5] = [
        RaiderType::Vindicator,
        RaiderType::Evoker,
        RaiderType::Pillager,
        RaiderType::Witch,
        RaiderType::Ravager,
    ];

    pub fn kind(self) -> &'static str {
        match self {
            RaiderType::Vindicator => "minecraft:vindicator",
            RaiderType::Evoker => "minecraft:evoker",
            RaiderType::Pillager => "minecraft:pillager",
            RaiderType::Witch => "minecraft:witch",
            RaiderType::Ravager => "minecraft:ravager",
        }
    }

    pub fn spawns_per_wave(self) -> [i32; 8] {
        match self {
            RaiderType::Vindicator => [0, 0, 2, 0, 1, 4, 2, 5],
            RaiderType::Evoker => [0, 0, 0, 0, 0, 1, 1, 2],
            RaiderType::Pillager => [0, 4, 3, 3, 4, 4, 4, 2],
            RaiderType::Witch => [0, 0, 0, 0, 3, 0, 0, 1],
            RaiderType::Ravager => [0, 0, 0, 1, 0, 1, 0, 2],
        }
    }

    pub fn can_be_leader(self) -> bool {
        !matches!(self, RaiderType::Witch | RaiderType::Ravager)
    }
}

pub fn num_groups(difficulty: i32) -> i32 {
    match difficulty {
        1 => 3,
        2 => 5,
        3 => 7,
        _ => 0,
    }
}

pub fn default_num_spawns(kind: RaiderType, wave: i32, bonus: bool, groups: i32) -> i32 {
    let index = if bonus { groups } else { wave };
    kind.spawns_per_wave()
        .get(index.clamp(0, 7) as usize)
        .copied()
        .unwrap_or(0)
}

pub fn potential_bonus_spawns(
    kind: RaiderType,
    random: &mut LegacyRandom,
    wave: i32,
    difficulty: i32,
    bonus: bool,
) -> i32 {
    let easy = difficulty == 1;
    let normal = difficulty == 2;
    let most = match kind {
        RaiderType::Witch => {
            if !easy && wave > 2 && wave != 4 {
                1
            } else {
                return 0;
            }
        }
        RaiderType::Vindicator | RaiderType::Pillager => {
            if easy {
                random.next_int(2) as i32
            } else if normal {
                1
            } else {
                2
            }
        }
        RaiderType::Ravager => i32::from(!easy && bonus),
        RaiderType::Evoker => return 0,
    };
    if most > 0 {
        random.next_int(most as u32 + 1) as i32
    } else {
        0
    }
}

pub fn enchant_odds(raid_omen_level: i32) -> f32 {
    match raid_omen_level {
        2 => 0.1,
        3 => 0.25,
        4 => 0.5,
        5 => 0.75,
        _ => 0.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaidStatus {
    Ongoing,
    Victory,
    Loss,
    Stopped,
}

impl RaidStatus {
    pub fn name(self) -> &'static str {
        match self {
            RaidStatus::Ongoing => "ongoing",
            RaidStatus::Victory => "victory",
            RaidStatus::Loss => "loss",
            RaidStatus::Stopped => "stopped",
        }
    }

    pub fn parse(name: &str) -> Self {
        match name {
            "victory" => RaidStatus::Victory,
            "loss" => RaidStatus::Loss,
            "stopped" => RaidStatus::Stopped,
            _ => RaidStatus::Ongoing,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarName {
    Raid,
    RaidersRemaining(usize),
    Victory,
    Defeat,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaidBar {
    pub id: i32,
    pub name: BarName,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaidPlayer {
    pub id: u64,
    pub uuid: u128,
    pub feet: [f64; 3],
    pub alive: bool,
    pub spectator: bool,
}

impl RaidPlayer {
    pub fn block(&self) -> Pos {
        (
            self.feet[0].floor() as i32,
            self.feet[1].floor() as i32,
            self.feet[2].floor() as i32,
        )
    }
}

pub trait RaidLevel {
    fn difficulty(&self) -> i32;
    fn is_village(&mut self, pos: Pos) -> bool;
    fn has_chunk(&self, pos: Pos) -> bool;
    fn has_chunks(&self, from: (i32, i32), to: (i32, i32)) -> bool;
    fn entity_ticking(&self, pos: Pos) -> bool;
    fn surface(&self, x: i32, z: i32) -> i32;
    fn spawn_ok(&mut self, pos: Pos) -> bool;
    fn can_start_raid(&self, pos: Pos) -> bool;
    fn occupied_village_pois(&self, pos: Pos, radius: i32) -> Vec<Pos>;
    fn player_uuid(&self, player: u64) -> u128;
    fn raiders(&self) -> Vec<RaiderView>;
    fn raider_mut(&mut self, id: u64) -> Option<RaiderMut<'_>>;
    fn spawn_raider(&mut self, member: RaidMember) -> Vec<u64>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaidMember {
    pub kind: &'static str,
    pub pos: Pos,
    pub wave: i32,
    pub raid: i32,
    pub leader: bool,
    pub enchant_odds: f32,
    pub rider: Option<&'static str>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RaidOutput {
    pub horns: Vec<(u64, [f64; 3])>,
    pub heroes: Vec<(u64, i32)>,
}

#[derive(Clone, Debug)]
pub struct Raid {
    pub started: bool,
    pub active: bool,
    pub ticks_active: i64,
    pub raid_omen_level: i32,
    pub groups_spawned: i32,
    pub cooldown_ticks: i32,
    pub post_raid_ticks: i32,
    pub total_health: f32,
    pub num_groups: i32,
    pub status: RaidStatus,
    pub center: Pos,
    pub heroes: BTreeSet<u128>,
    pub celebration_ticks: i32,
    pub groups: BTreeMap<i32, Vec<u64>>,
    pub leaders: BTreeMap<i32, u64>,
    pub wave_spawn_pos: Option<Pos>,
    pub bar_players: Vec<u64>,
    pub bar_name: BarName,
    pub bar_progress: f32,
    pub bar_visible: bool,
    pub random: LegacyRandom,
    pub seen: BTreeMap<u64, f32>,
}

impl Raid {
    pub fn new(center: Pos, difficulty: i32, seed: u64) -> Self {
        Self {
            started: false,
            active: true,
            ticks_active: 0,
            raid_omen_level: 0,
            groups_spawned: 0,
            cooldown_ticks: PRE_RAID_TICKS,
            post_raid_ticks: 0,
            total_health: 0.0,
            num_groups: num_groups(difficulty),
            status: RaidStatus::Ongoing,
            center,
            heroes: BTreeSet::new(),
            celebration_ticks: 0,
            groups: BTreeMap::new(),
            leaders: BTreeMap::new(),
            wave_spawn_pos: None,
            bar_players: Vec::new(),
            bar_name: BarName::Raid,
            bar_progress: 0.0,
            bar_visible: true,
            random: LegacyRandom::new(seed),
            seen: BTreeMap::new(),
        }
    }

    pub fn is_over(&self) -> bool {
        matches!(self.status, RaidStatus::Victory | RaidStatus::Loss)
    }

    pub fn is_stopped(&self) -> bool {
        self.status == RaidStatus::Stopped
    }

    pub fn has_first_wave_spawned(&self) -> bool {
        self.groups_spawned > 0
    }

    pub fn is_between_waves(&self) -> bool {
        self.has_first_wave_spawned() && self.raiders_alive() == 0 && self.cooldown_ticks > 0
    }

    pub fn raiders_alive(&self) -> usize {
        self.groups.values().map(Vec::len).sum()
    }

    pub fn raider_ids(&self) -> Vec<u64> {
        self.groups.values().flatten().copied().collect()
    }

    pub fn has_bonus_wave(&self) -> bool {
        self.raid_omen_level > 1
    }

    pub fn has_spawned_bonus_wave(&self) -> bool {
        self.groups_spawned > self.num_groups
    }

    pub fn is_final_wave(&self) -> bool {
        self.groups_spawned == self.num_groups
    }

    pub fn has_more_waves(&self) -> bool {
        if self.has_bonus_wave() {
            !self.has_spawned_bonus_wave()
        } else {
            !self.is_final_wave()
        }
    }

    pub fn should_spawn_bonus_group(&self) -> bool {
        self.is_final_wave() && self.raiders_alive() == 0 && self.has_bonus_wave()
    }

    pub fn should_spawn_group(&self) -> bool {
        self.cooldown_ticks == 0
            && (self.groups_spawned < self.num_groups || self.should_spawn_bonus_group())
            && self.raiders_alive() == 0
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.bar_players.clear();
        self.status = RaidStatus::Stopped;
    }

    pub fn absorb_raid_omen(&mut self, amplifier: i32) {
        self.raid_omen_level = (self.raid_omen_level + amplifier + 1).clamp(0, MAX_RAID_OMEN_LEVEL);
    }

    pub fn view(&self, id: i32) -> RaidView {
        RaidView {
            id,
            center: self.center,
            active: self.active,
            over: self.is_over(),
            loss: self.status == RaidStatus::Loss,
            groups_spawned: self.groups_spawned,
            victory: self.status == RaidStatus::Victory,
            stopped: self.is_stopped(),
            between_waves: self.is_between_waves(),
        }
    }

    fn registered(&self, id: u64) -> bool {
        self.groups.values().any(|ids| ids.contains(&id))
    }

    fn add_wave_mob(&mut self, wave: i32, id: u64, health: Option<f32>) {
        let group = self.groups.entry(wave).or_default();
        if !group.contains(&id) {
            group.push(id);
        }
        if let Some(health) = health {
            self.total_health += health;
        }
    }

    fn remove(&mut self, wave: i32, id: u64, health: Option<f32>) -> bool {
        let Some(group) = self.groups.get_mut(&wave) else {
            return false;
        };
        let Some(index) = group.iter().position(|&raider| raider == id) else {
            return false;
        };
        group.remove(index);
        if let Some(health) = health {
            self.total_health -= health;
        }
        true
    }

    fn update_bar(&mut self, raiders: &BTreeMap<u64, RaiderView>) {
        let mut living = 0.0;
        for id in self.raider_ids() {
            if let Some(view) = raiders.get(&id) {
                let health = view.health.max(0.0);
                self.seen.insert(id, health);
                living += health;
            }
        }
        let ids = self.raider_ids();
        self.seen.retain(|id, _| ids.contains(id));
        self.bar_progress = if self.total_health > 0.0 {
            (living / self.total_health).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    pub fn bar(&self, id: i32) -> RaidBar {
        RaidBar {
            id,
            name: self.bar_name,
            progress: self.bar_progress,
        }
    }

    fn find_random_spawn_pos(&mut self, level: &mut dyn RaidLevel, attempts: i32) -> Option<Pos> {
        let i = self.cooldown_ticks / 20;
        let f = 0.22 * i as f32 - 0.24;
        let angle = self.random.next_float() * std::f32::consts::TAU;
        for j in 0..attempts {
            let a = angle + std::f32::consts::PI * j as f32 / 8.0;
            let spread = f.floor() as i32;
            let x = self.center.0
                + (minecraftoss_player::mth::cos(f64::from(a)) * 32.0 * f).floor() as i32
                + self.random.next_int(3) as i32 * spread;
            let z = self.center.2
                + (minecraftoss_player::mth::sin(f64::from(a)) * 32.0 * f).floor() as i32
                + self.random.next_int(3) as i32 * spread;
            let y = level.surface(x, z);
            if (y - self.center.1).abs() > 96 {
                continue;
            }
            let pos = (x, y, z);
            if level.is_village(pos) && i > ALLOW_SPAWNING_WITHIN_VILLAGE_SECONDS {
                continue;
            }
            if !level.has_chunks((x - 10, z - 10), (x + 10, z + 10)) {
                continue;
            }
            if !level.entity_ticking(pos) {
                continue;
            }
            if level.spawn_ok(pos) {
                return Some(pos);
            }
        }
        None
    }

    fn move_center_to_village(&mut self, level: &mut dyn RaidLevel) {
        let section = (self.center.0 >> 4, self.center.1 >> 4, self.center.2 >> 4);
        let radius = VILLAGE_SEARCH_SECTIONS;
        let mut best: Option<(f64, Pos)> = None;
        for dz in -radius..=radius {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let centre = (
                        ((section.0 + dx) << 4) + 8,
                        ((section.1 + dy) << 4) + 8,
                        ((section.2 + dz) << 4) + 8,
                    );
                    if !level.is_village(centre) {
                        continue;
                    }
                    let distance = dist_sqr(centre, self.center);
                    if best.is_none_or(|(nearest, _)| distance < nearest) {
                        best = Some((distance, centre));
                    }
                }
            }
        }
        if let Some((_, centre)) = best {
            self.center = centre;
        }
    }

    fn spawn_group(
        &mut self,
        id: i32,
        level: &mut dyn RaidLevel,
        pos: Pos,
        raiders: &mut BTreeMap<u64, RaiderView>,
    ) {
        let wave = self.groups_spawned + 1;
        self.total_health = 0.0;
        let difficulty = level.difficulty();
        let bonus = self.should_spawn_bonus_group();
        let odds = enchant_odds(self.raid_omen_level);
        let mut leader_set = false;
        for kind in RaiderType::VALUES {
            let count = default_num_spawns(kind, wave, bonus, self.num_groups)
                + potential_bonus_spawns(kind, &mut self.random, wave, difficulty, bonus);
            let mut riders = 0;
            for _ in 0..count {
                let leader = !leader_set && kind.can_be_leader();
                leader_set |= leader;
                let rider = (kind == RaiderType::Ravager)
                    .then(|| {
                        let rider = if wave == num_groups(2) {
                            Some("minecraft:pillager")
                        } else if wave >= num_groups(3) {
                            Some(if riders == 0 {
                                "minecraft:evoker"
                            } else {
                                "minecraft:vindicator"
                            })
                        } else {
                            None
                        };
                        riders += 1;
                        rider
                    })
                    .flatten();
                let member = RaidMember {
                    kind: kind.kind(),
                    pos,
                    wave,
                    raid: id,
                    leader,
                    enchant_odds: odds,
                    rider,
                };
                let spawned = level.spawn_raider(member);
                if spawned.is_empty() {
                    break;
                }
                let views: BTreeMap<u64, RaiderView> = level
                    .raiders()
                    .into_iter()
                    .map(|view| (view.id, view))
                    .collect();
                for &raider in &spawned {
                    let health = views.get(&raider).map(|view| view.health);
                    self.add_wave_mob(wave, raider, health);
                    if leader && raider == spawned[0] {
                        self.leaders.insert(wave, raider);
                    }
                }
                raiders.extend(views);
            }
        }
        self.wave_spawn_pos = None;
        self.groups_spawned += 1;
        self.update_bar(raiders);
    }

    fn horns(&mut self, players: &[RaidPlayer], pos: Pos, out: &mut RaidOutput) {
        let _seed = self.random.next_long();
        let at = [
            f64::from(pos.0) + 0.5,
            f64::from(pos.1) + 0.5,
            f64::from(pos.2) + 0.5,
        ];
        for player in players {
            let (dx, dz) = (at[0] - player.feet[0], at[2] - player.feet[2]);
            let distance = (dx * dx + dz * dz).sqrt();
            if distance > HORN_RANGE && !self.bar_players.contains(&player.id) {
                continue;
            }
            let step = if distance > 0.0 {
                HORN_DISTANCE / distance
            } else {
                0.0
            };
            let x = player.feet[0] + step * dx;
            let z = player.feet[2] + step * dz;
            out.horns.push((player.id, [x, player.feet[1], z]));
        }
    }

    fn update_players(&mut self, players: &[RaidPlayer], here: &dyn Fn(&RaidPlayer) -> bool) {
        let valid: Vec<u64> = players
            .iter()
            .filter(|player| player.alive && here(player))
            .map(|player| player.id)
            .collect();
        for &id in &valid {
            if !self.bar_players.contains(&id) {
                self.bar_players.push(id);
            }
        }
        self.bar_players.retain(|id| valid.contains(id));
    }

    fn update_raiders(&mut self, level: &mut dyn RaidLevel, raiders: &BTreeMap<u64, RaiderView>) {
        let mut leaving = Vec::new();
        for id in self.raider_ids() {
            let Some(view) = raiders.get(&id) else {
                leaving.push((id, self.seen.get(&id).copied()));
                continue;
            };
            let far = dist_sqr(self.center, block_of(view)) >= RAID_REMOVAL_THRESHOLD_SQR;
            if far {
                leaving.push((id, Some(view.health)));
                continue;
            }
            if view.tick_count <= OLD_RAIDER_TICKS {
                continue;
            }
            let mut outside = view.raider.ticks_outside_raid;
            if !level.is_village(block_of(view)) && view.no_action_time > MAX_NO_ACTION_TIME {
                outside += 1;
                if let Some(raider) = level.raider_mut(id) {
                    raider.raider.ticks_outside_raid = outside;
                }
            }
            if outside >= OUTSIDE_RAID_BOUNDS_TIMEOUT {
                leaving.push((id, Some(view.health)));
            }
        }
        for (id, health) in leaving {
            let wave = self
                .groups
                .iter()
                .find(|(_, ids)| ids.contains(&id))
                .map(|(&wave, _)| wave);
            let Some(wave) = wave else { continue };
            self.remove(wave, id, Some(health.unwrap_or(0.0)));
            if let Some(raider) = level.raider_mut(id) {
                raider.raider.raid_id = None;
                if raider.raider.patrol_leader {
                    self.leaders.remove(&wave);
                }
            } else if self.leaders.get(&wave) == Some(&id) {
                self.leaders.remove(&wave);
            }
        }
        self.update_bar(raiders);
    }

    #[allow(clippy::too_many_arguments)]
    fn tick(
        &mut self,
        id: i32,
        level: &mut dyn RaidLevel,
        players: &[RaidPlayer],
        others: &[RaidView],
        raiders: &mut BTreeMap<u64, RaiderView>,
        out: &mut RaidOutput,
    ) {
        if self.is_stopped() {
            return;
        }
        let here = |player: &RaidPlayer| {
            nearby_raid(others, player.block()).is_some_and(|raid| raid.id == id)
        };
        if self.status == RaidStatus::Ongoing {
            let was_active = self.active;
            self.active = level.has_chunk(self.center);
            if level.difficulty() == 0 {
                self.stop();
                return;
            }
            if was_active != self.active {
                self.bar_visible = self.active;
            }
            if !self.active {
                return;
            }
            if !level.is_village(self.center) {
                self.move_center_to_village(level);
            }
            if !level.is_village(self.center) {
                if self.groups_spawned > 0 {
                    self.status = RaidStatus::Loss;
                } else {
                    self.stop();
                }
            }
            self.ticks_active += 1;
            if self.ticks_active >= RAID_TIMEOUT_TICKS {
                self.stop();
                return;
            }
            let alive = self.raiders_alive();
            if alive == 0 && self.has_more_waves() {
                if self.cooldown_ticks > 0 {
                    let known = self.wave_spawn_pos.is_some();
                    let mut find = !known && self.cooldown_ticks % 5 == 0;
                    if let Some(pos) = self.wave_spawn_pos {
                        if !level.entity_ticking(pos) {
                            find = true;
                        }
                    }
                    if find {
                        self.wave_spawn_pos = self.find_random_spawn_pos(level, 8);
                    }
                    if self.cooldown_ticks == PRE_RAID_TICKS || self.cooldown_ticks % 20 == 0 {
                        self.update_players(players, &here);
                    }
                    self.cooldown_ticks -= 1;
                    let elapsed = (PRE_RAID_TICKS - self.cooldown_ticks) as f32;
                    self.bar_progress = (elapsed / PRE_RAID_TICKS as f32).clamp(0.0, 1.0);
                } else if self.groups_spawned > 0 {
                    self.cooldown_ticks = PRE_RAID_TICKS;
                    self.bar_name = BarName::Raid;
                    return;
                }
            }
            if self.ticks_active % 20 == 0 {
                self.update_players(players, &here);
                self.update_raiders(level, raiders);
                self.bar_name = if alive > 0 && alive <= LOW_MOB_THRESHOLD {
                    BarName::RaidersRemaining(alive)
                } else {
                    BarName::Raid
                };
            }
            let mut horn = false;
            let mut failures = 0;
            while self.should_spawn_group() {
                let pos = match self.wave_spawn_pos {
                    Some(pos) => Some(pos),
                    None => self.find_random_spawn_pos(level, 20),
                };
                match pos {
                    Some(pos) => {
                        self.started = true;
                        self.spawn_group(id, level, pos, raiders);
                        if !horn {
                            self.horns(players, pos, out);
                            horn = true;
                        }
                    }
                    None => failures += 1,
                }
                if failures > NUM_SPAWN_ATTEMPTS {
                    self.stop();
                    break;
                }
            }
            if self.started && !self.has_more_waves() && alive == 0 {
                if self.post_raid_ticks < POST_RAID_TICKS {
                    self.post_raid_ticks += 1;
                } else {
                    self.status = RaidStatus::Victory;
                    for uuid in &self.heroes {
                        if let Some(player) =
                            players.iter().find(|p| p.uuid == *uuid && !p.spectator)
                        {
                            out.heroes.push((player.id, self.raid_omen_level - 1));
                        }
                    }
                }
            }
        } else if self.is_over() {
            self.celebration_ticks += 1;
            if self.celebration_ticks >= MAX_CELEBRATION_TICKS {
                self.stop();
                return;
            }
            if self.celebration_ticks % 20 == 0 {
                self.update_players(players, &here);
                self.bar_visible = true;
                if self.status == RaidStatus::Victory {
                    self.bar_progress = 0.0;
                    self.bar_name = BarName::Victory;
                } else {
                    self.bar_name = BarName::Defeat;
                }
            }
        }
    }
}

fn block_of(view: &RaiderView) -> Pos {
    (
        view.position.x.floor() as i32,
        view.position.y.floor() as i32,
        view.position.z.floor() as i32,
    )
}

#[derive(Clone, Debug)]
pub struct Raids {
    pub raids: BTreeMap<i32, Raid>,
    pub next_id: i32,
    pub tick: i32,
    pub dirty: bool,
}

impl Default for Raids {
    fn default() -> Self {
        Self {
            raids: BTreeMap::new(),
            next_id: FIRST_ID,
            tick: 0,
            dirty: false,
        }
    }
}

impl Raids {
    pub fn views(&self) -> Vec<RaidView> {
        self.raids.iter().map(|(&id, raid)| raid.view(id)).collect()
    }

    pub fn raid_at(&self, pos: Pos) -> Option<i32> {
        nearby_raid(&self.views(), pos).map(|raid| raid.id)
    }

    pub fn omen_turns(&self, level: &mut dyn RaidLevel, player: &RaidPlayer) -> bool {
        if player.spectator || level.difficulty() == 0 || !level.is_village(player.block()) {
            return false;
        }
        match self.raid_at(player.block()) {
            Some(id) => self.raids[&id].raid_omen_level < MAX_RAID_OMEN_LEVEL,
            None => true,
        }
    }

    pub fn create_or_extend(
        &mut self,
        level: &mut dyn RaidLevel,
        player: &RaidPlayer,
        at: Pos,
        amplifier: i32,
        rule: bool,
        seed: u64,
    ) -> Option<i32> {
        if player.spectator || !rule || !level.can_start_raid(at) {
            return None;
        }
        let pois = level.occupied_village_pois(at, VILLAGE_POI_RANGE);
        let center = if pois.is_empty() {
            at
        } else {
            let count = pois.len() as f64;
            let sum = pois.iter().fold((0.0, 0.0, 0.0), |(x, y, z), p| {
                (x + f64::from(p.0), y + f64::from(p.1), z + f64::from(p.2))
            });
            (
                (sum.0 / count).floor() as i32,
                (sum.1 / count).floor() as i32,
                (sum.2 / count).floor() as i32,
            )
        };
        let id = match self.raid_at(center) {
            Some(id) => id,
            None => {
                self.next_id += 1;
                let id = self.next_id;
                self.raids
                    .insert(id, Raid::new(center, level.difficulty(), seed));
                id
            }
        };
        let raid = self.raids.get_mut(&id)?;
        if !raid.started || raid.raid_omen_level < MAX_RAID_OMEN_LEVEL {
            raid.absorb_raid_omen(amplifier);
        }
        self.dirty = true;
        Some(id)
    }

    pub fn tick(
        &mut self,
        level: &mut dyn RaidLevel,
        players: &[RaidPlayer],
        rule: bool,
        game_time: i64,
    ) -> RaidOutput {
        let mut out = RaidOutput::default();
        self.tick += 1;
        let mut raiders: BTreeMap<u64, RaiderView> = level
            .raiders()
            .into_iter()
            .map(|view| (view.id, view))
            .collect();
        self.rejoin_saved(level, &raiders);
        self.handle_deaths(level, &raiders);
        if game_time % 20 == 0 {
            self.join_nearby(level, &raiders);
        }
        self.recruit(level, &raiders);
        let ids: Vec<i32> = self.raids.keys().copied().collect();
        for id in ids {
            let others = self.views();
            let Some(raid) = self.raids.get_mut(&id) else {
                continue;
            };
            if !rule {
                raid.stop();
            }
            if raid.is_stopped() {
                for raider in raid.raider_ids() {
                    if let Some(found) = level.raider_mut(raider) {
                        found.raider.raid_id = None;
                    }
                }
                self.raids.remove(&id);
                self.dirty = true;
                continue;
            }
            raid.tick(id, level, players, &others, &mut raiders, &mut out);
            if raid.status == RaidStatus::Ongoing && raid.raiders_alive() > 0 {
                raid.update_bar(&raiders);
            }
            self.dirty = true;
        }
        out
    }

    fn rejoin_saved(&mut self, level: &mut dyn RaidLevel, raiders: &BTreeMap<u64, RaiderView>) {
        for view in raiders.values() {
            let Some(raid_id) = view.raider.raid_id else {
                continue;
            };
            match self.raids.get_mut(&raid_id) {
                Some(raid) if !raid.registered(view.id) && view.health > 0.0 => {
                    raid.add_wave_mob(view.raider.wave, view.id, None);
                    if view.raider.patrol_leader {
                        raid.leaders.insert(view.raider.wave, view.id);
                    }
                }
                Some(_) => {}
                None => {
                    if let Some(raider) = level.raider_mut(view.id) {
                        raider.raider.raid_id = None;
                    }
                }
            }
        }
    }

    fn handle_deaths(&mut self, level: &mut dyn RaidLevel, raiders: &BTreeMap<u64, RaiderView>) {
        for raid in self.raids.values_mut() {
            for id in raid.raider_ids() {
                let Some(view) = raiders.get(&id) else {
                    continue;
                };
                if view.health > 0.0 {
                    continue;
                }
                let wave = view.raider.wave;
                if view.raider.patrol_leader {
                    raid.leaders.remove(&wave);
                }
                if let Some(player) = view.hurt_by_player {
                    raid.heroes.insert(level.player_uuid(player));
                }
                if !raid.remove(wave, id, None) {
                    for ids in raid.groups.values_mut() {
                        ids.retain(|&raider| raider != id);
                    }
                }
                if let Some(raider) = level.raider_mut(id) {
                    raider.raider.raid_id = None;
                }
            }
        }
    }

    fn join(&mut self, level: &mut dyn RaidLevel, raid_id: i32, view: &RaiderView) {
        let Some(raid) = self.raids.get_mut(&raid_id) else {
            return;
        };
        let wave = raid.groups_spawned;
        raid.add_wave_mob(wave, view.id, Some(view.health));
        if let Some(found) = level.raider_mut(view.id) {
            found.raider.raid_id = Some(raid_id);
            found.raider.wave = wave;
            found.raider.can_join_raid = true;
            found.raider.ticks_outside_raid = 0;
        }
    }

    fn can_join(view: &RaiderView) -> bool {
        view.health > 0.0 && view.raider.can_join_raid && view.no_action_time <= MAX_NO_ACTION_TIME
    }

    fn join_nearby(&mut self, level: &mut dyn RaidLevel, raiders: &BTreeMap<u64, RaiderView>) {
        for view in raiders.values() {
            if view.raider.raid_id.is_some() {
                if view.hunts_player_or_golem {
                    if let Some(raider) = level.raider_mut(view.id) {
                        *raider.no_action_time = 0;
                    }
                }
                continue;
            }
            if !(view.health > 0.0 && view.raider.can_join_raid) {
                continue;
            }
            let Some(raid) = self.raid_at(block_of(view)) else {
                continue;
            };
            if Self::can_join(view) {
                self.join(level, raid, view);
            }
        }
    }

    fn recruit(&mut self, level: &mut dyn RaidLevel, raiders: &BTreeMap<u64, RaiderView>) {
        let recruiters: Vec<&RaiderView> = raiders.values().filter(|view| view.recruits).collect();
        let mut joined = BTreeSet::new();
        for recruiter in recruiters {
            if let Some(found) = level.raider_mut(recruiter.id) {
                *found.recruit = false;
            }
            let Some(raid_id) = recruiter.raider.raid_id else {
                continue;
            };
            if !self.raids.get(&raid_id).is_some_and(|raid| raid.active) {
                continue;
            }
            let (p, half, height) = (
                recruiter.position,
                f64::from(recruiter.width / 2.0),
                f64::from(recruiter.height),
            );
            let range = minecraftoss_entities::raid::RECRUIT_RANGE;
            for view in raiders.values() {
                let q = view.position;
                let other = f64::from(view.width / 2.0);
                let inside = q.x - other < p.x + half + range
                    && q.x + other > p.x - half - range
                    && q.y < p.y + height + range
                    && q.y + f64::from(view.height) > p.y - range
                    && q.z - other < p.z + half + range
                    && q.z + other > p.z - half - range;
                let active = view
                    .raider
                    .raid_id
                    .and_then(|raid| self.raids.get(&raid))
                    .is_some_and(|raid| raid.active);
                if inside && !active && Self::can_join(view) && joined.insert(view.id) {
                    self.join(level, raid_id, view);
                }
            }
        }
    }

    pub fn bars_for(&self, player: u64) -> Vec<RaidBar> {
        self.raids
            .iter()
            .filter(|(_, raid)| {
                raid.bar_visible && !raid.is_stopped() && raid.bar_players.contains(&player)
            })
            .map(|(&id, raid)| raid.bar(id))
            .collect()
    }

    pub fn read(tag: &Tag) -> Self {
        let mut raids = BTreeMap::new();
        for entry in tag.get("raids").and_then(Tag::as_list).unwrap_or(&[]) {
            let int = |key: &str| entry.get(key).and_then(Tag::as_i64).unwrap_or(0);
            let flag = |key: &str| int(key) != 0;
            let Some(center) = entry
                .get("center")
                .and_then(Tag::as_ints)
                .and_then(|ints| <[i32; 3]>::try_from(ints).ok())
            else {
                continue;
            };
            let id = int("id") as i32;
            let heroes = entry
                .get("heroes_of_the_village")
                .and_then(Tag::as_list)
                .unwrap_or(&[])
                .iter()
                .filter_map(Tag::as_ints)
                .filter_map(|ints| <[i32; 4]>::try_from(ints).ok())
                .map(uuid_of_ints)
                .collect();
            let mut raid = Raid::new((center[0], center[1], center[2]), 0, id as u64);
            raid.started = flag("started");
            raid.active = flag("active");
            raid.ticks_active = int("ticks_active");
            raid.raid_omen_level = int("raid_omen_level") as i32;
            raid.groups_spawned = int("groups_spawned") as i32;
            raid.cooldown_ticks = int("cooldown_ticks") as i32;
            raid.post_raid_ticks = int("post_raid_ticks") as i32;
            raid.total_health = entry
                .get("total_health")
                .and_then(Tag::as_f64)
                .unwrap_or(0.0) as f32;
            raid.num_groups = int("group_count") as i32;
            raid.status =
                RaidStatus::parse(entry.get("status").and_then(Tag::as_str).unwrap_or(""));
            raid.heroes = heroes;
            raids.insert(id, raid);
        }
        let int = |key: &str| tag.get(key).and_then(Tag::as_i64).map(|value| value as i32);
        Self {
            raids,
            next_id: int("next_id").unwrap_or(FIRST_ID),
            tick: int("tick").unwrap_or(0),
            dirty: false,
        }
    }

    pub fn write(&self) -> Tag {
        let raids = self
            .raids
            .iter()
            .map(|(&id, raid)| {
                let heroes = raid
                    .heroes
                    .iter()
                    .map(|&uuid| Tag::IntArray(ints_of_uuid(uuid).to_vec()))
                    .collect();
                let (x, y, z) = raid.center;
                Tag::Compound(
                    [
                        ("id", Tag::Int(id)),
                        ("started", Tag::Byte(i8::from(raid.started))),
                        ("active", Tag::Byte(i8::from(raid.active))),
                        ("ticks_active", Tag::Long(raid.ticks_active)),
                        ("raid_omen_level", Tag::Int(raid.raid_omen_level)),
                        ("groups_spawned", Tag::Int(raid.groups_spawned)),
                        ("cooldown_ticks", Tag::Int(raid.cooldown_ticks)),
                        ("post_raid_ticks", Tag::Int(raid.post_raid_ticks)),
                        ("total_health", Tag::Float(raid.total_health)),
                        ("group_count", Tag::Int(raid.num_groups)),
                        ("status", Tag::String(raid.status.name().to_owned())),
                        ("center", Tag::IntArray(vec![x, y, z])),
                        ("heroes_of_the_village", Tag::List(heroes)),
                    ]
                    .into_iter()
                    .map(|(key, value)| (key.to_owned(), value))
                    .collect(),
                )
            })
            .collect();
        Tag::Compound(
            [
                ("raids".to_owned(), Tag::List(raids)),
                ("next_id".to_owned(), Tag::Int(self.next_id)),
                ("tick".to_owned(), Tag::Int(self.tick)),
            ]
            .into_iter()
            .collect(),
        )
    }
}

pub fn uuid_of_ints(ints: [i32; 4]) -> u128 {
    ints.iter()
        .fold(0u128, |uuid, &word| (uuid << 32) | u128::from(word as u32))
}

pub fn ints_of_uuid(uuid: u128) -> [i32; 4] {
    [
        (uuid >> 96) as u32 as i32,
        (uuid >> 64) as u32 as i32,
        (uuid >> 32) as u32 as i32,
        uuid as u32 as i32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use minecraftoss_entities::monster_ai::Target;
    use minecraftoss_entities::poi::PoiType;
    use minecraftoss_entities::world::EntityWorld;

    const BELL: Pos = (8, 64, 8);

    struct Village {
        world: EntityWorld,
        difficulty: i32,
        spawned: Vec<RaidMember>,
    }

    impl Village {
        fn new(difficulty: i32) -> Self {
            let mut world = EntityWorld::default();
            assert!(world.pois.add(BELL, PoiType::Meeting));
            assert!(world.pois.acquire(BELL));
            Self {
                world,
                difficulty,
                spawned: Vec::new(),
            }
        }

        fn living(&self) -> Vec<u64> {
            self.world
                .raiders()
                .into_iter()
                .filter(|view| view.health > 0.0)
                .map(|view| view.id)
                .collect()
        }

        fn kill_all(&mut self, by: u64) {
            for id in self.living() {
                if let Some(e) = self.world.illager_mut(id) {
                    e.illager.health = 0.0;
                    e.ai.state.hurt_by = Some((Target::Player(by), 0));
                } else if let Some(e) = self.world.ravager_mut(id) {
                    e.ravager.health = 0.0;
                    e.ai.state.hurt_by = Some((Target::Player(by), 0));
                } else if let Some(e) = self.world.witch_mut(id) {
                    e.witch.health = 0.0;
                    e.ai.state.hurt_by = Some((Target::Player(by), 0));
                }
            }
        }
    }

    impl RaidLevel for Village {
        fn difficulty(&self) -> i32 {
            self.difficulty
        }
        fn is_village(&mut self, pos: Pos) -> bool {
            self.world.pois.is_village(pos)
        }
        fn has_chunk(&self, _: Pos) -> bool {
            true
        }
        fn has_chunks(&self, _: (i32, i32), _: (i32, i32)) -> bool {
            true
        }
        fn entity_ticking(&self, _: Pos) -> bool {
            true
        }
        fn surface(&self, _: i32, _: i32) -> i32 {
            64
        }
        fn spawn_ok(&mut self, _: Pos) -> bool {
            true
        }
        fn can_start_raid(&self, _: Pos) -> bool {
            true
        }
        fn occupied_village_pois(&self, pos: Pos, radius: i32) -> Vec<Pos> {
            let village = |kind: PoiType| kind.village();
            self.world
                .pois
                .in_range(
                    &village,
                    pos,
                    radius,
                    minecraftoss_entities::poi::Occupancy::IsOccupied,
                )
                .into_iter()
                .map(|record| record.pos)
                .collect()
        }
        fn player_uuid(&self, player: u64) -> u128 {
            u128::from(player) + 1000
        }
        fn raiders(&self) -> Vec<RaiderView> {
            self.world.raiders()
        }
        fn raider_mut(&mut self, id: u64) -> Option<RaiderMut<'_>> {
            self.world.raider_mut(id)
        }
        fn spawn_raider(&mut self, member: RaidMember) -> Vec<u64> {
            self.spawned.push(member);
            let at = DVec3::new(
                f64::from(member.pos.0) + 0.5,
                f64::from(member.pos.1) + 1.0,
                f64::from(member.pos.2) + 0.5,
            );
            let mut ids = Vec::new();
            for kind in std::iter::once(member.kind).chain(member.rider) {
                let raider = minecraftoss_entities::illager::Raider {
                    wave: member.wave,
                    can_join_raid: true,
                    raid_id: Some(member.raid),
                    patrol_leader: member.leader && kind == member.kind,
                    ..Default::default()
                };
                let id = match kind {
                    "minecraft:ravager" => {
                        let mut ravager = minecraftoss_entities::ravager::Ravager::new(at);
                        ravager.raider = raider;
                        self.world.spawn_ravager(ravager, true)
                    }
                    "minecraft:witch" => {
                        let mut witch = minecraftoss_entities::witch::Witch::new(at);
                        witch.raider = raider;
                        self.world.spawn_witch(witch, true)
                    }
                    _ => {
                        let illager_kind = match kind {
                            "minecraft:vindicator" => {
                                minecraftoss_entities::illager::IllagerKind::Vindicator
                            }
                            "minecraft:evoker" => {
                                minecraftoss_entities::illager::IllagerKind::Evoker
                            }
                            _ => minecraftoss_entities::illager::IllagerKind::Pillager,
                        };
                        let mut illager =
                            minecraftoss_entities::illager::Illager::new(illager_kind, at);
                        illager.raider = raider;
                        self.world.spawn_illager(illager, true)
                    }
                };
                ids.push(id);
            }
            ids
        }
    }

    fn player(id: u64, feet: [f64; 3]) -> RaidPlayer {
        RaidPlayer {
            id,
            uuid: u128::from(id) + 1000,
            feet,
            alive: true,
            spectator: false,
        }
    }

    fn started(village: &mut Village, amplifier: i32) -> (Raids, i32) {
        let mut raids = Raids::default();
        let host = player(0, [8.5, 64.0, 8.5]);
        let id = raids
            .create_or_extend(village, &host, BELL, amplifier, true, 7)
            .expect("a raid starts in a village");
        (raids, id)
    }

    fn run_until(
        raids: &mut Raids,
        village: &mut Village,
        players: &[RaidPlayer],
        mut done: impl FnMut(&Raids, &mut Village, &RaidOutput) -> bool,
    ) -> Vec<RaidOutput> {
        let mut outs = Vec::new();
        for tick in 1..=20_000 {
            let out = raids.tick(village, players, true, tick);
            let finished = done(raids, village, &out);
            outs.push(out);
            if finished {
                return outs;
            }
        }
        panic!("the raid never got there");
    }

    #[test]
    fn waves_per_difficulty_follow_the_raider_table() {
        assert_eq!(
            [num_groups(0), num_groups(1), num_groups(2), num_groups(3)],
            [0, 3, 5, 7]
        );
        let wave = |n: i32| RaiderType::VALUES.map(|kind| default_num_spawns(kind, n, false, 5));
        assert_eq!(wave(1), [0, 0, 4, 0, 0]);
        assert_eq!(wave(2), [2, 0, 3, 0, 0]);
        assert_eq!(wave(3), [0, 0, 3, 0, 1]);
        assert_eq!(wave(4), [1, 0, 4, 3, 0]);
        assert_eq!(wave(5), [4, 1, 4, 0, 1]);
        assert_eq!(wave(6), [2, 1, 4, 0, 0]);
        assert_eq!(wave(7), [5, 2, 2, 1, 2]);
        let bonus = RaiderType::VALUES.map(|kind| default_num_spawns(kind, 6, true, 5));
        assert_eq!(
            bonus,
            wave(5),
            "the bonus wave repeats the last wave's numbers"
        );
        let mut random = LegacyRandom::new(3);
        for _ in 0..200 {
            let easy = potential_bonus_spawns(RaiderType::Pillager, &mut random, 2, 1, false);
            let hard = potential_bonus_spawns(RaiderType::Vindicator, &mut random, 2, 3, false);
            assert!((0..=1).contains(&easy) && (0..=2).contains(&hard));
            assert_eq!(
                potential_bonus_spawns(RaiderType::Evoker, &mut random, 5, 3, true),
                0
            );
            assert_eq!(
                potential_bonus_spawns(RaiderType::Witch, &mut random, 4, 3, false),
                0
            );
            assert_eq!(
                potential_bonus_spawns(RaiderType::Witch, &mut random, 3, 1, false),
                0
            );
            assert_eq!(
                potential_bonus_spawns(RaiderType::Ravager, &mut random, 3, 2, false),
                0
            );
        }
        assert_eq!(
            [1, 2, 3, 4, 5, 6].map(enchant_odds),
            [0.0, 0.1, 0.25, 0.5, 0.75, 0.0]
        );
        assert!(RaiderType::Pillager.can_be_leader() && !RaiderType::Witch.can_be_leader());
    }

    #[test]
    fn an_easy_raid_has_three_waves_and_a_raid_omen_above_one_adds_a_bonus_wave() {
        let players = [player(0, [8.5, 64.0, 8.5])];
        for (amplifier, waves) in [(0, 3), (1, 4)] {
            let mut village = Village::new(1);
            let (mut raids, id) = started(&mut village, amplifier);
            assert_eq!(raids.raids[&id].raid_omen_level, amplifier + 1);
            assert_eq!(raids.raids[&id].has_bonus_wave(), amplifier > 0);
            run_until(&mut raids, &mut village, &players, |raids, village, _| {
                village.kill_all(0);
                raids.raids[&id].status == RaidStatus::Victory
            });
            assert_eq!(raids.raids[&id].groups_spawned, waves);
            let kinds = |wave: i32| -> Vec<&str> {
                village
                    .spawned
                    .iter()
                    .filter(|member| member.wave == wave)
                    .map(|member| member.kind)
                    .collect()
            };
            assert!(kinds(1).iter().all(|kind| *kind == "minecraft:pillager"));
            assert!((4..=5).contains(&kinds(1).len()), "{:?}", kinds(1));
            if waves == 4 {
                let last = kinds(4);
                assert!(
                    last.contains(&"minecraft:ravager"),
                    "easy's bonus wave is wave three's: {last:?}"
                );
                assert!(
                    last.iter()
                        .filter(|kind| **kind == "minecraft:pillager")
                        .count()
                        >= 3
                );
            }
            let leaders: Vec<&RaidMember> = village
                .spawned
                .iter()
                .filter(|member| member.leader)
                .collect();
            assert_eq!(leaders.len() as i32, waves, "one captain a wave");
            assert!(
                leaders
                    .iter()
                    .all(|member| member.kind != "minecraft:ravager")
            );
        }
    }

    #[test]
    fn raids_dat_reads_back_what_it_wrote() {
        let mut village = Village::new(2);
        let (mut raids, id) = started(&mut village, 2);
        let raid = raids.raids.get_mut(&id).unwrap();
        raid.started = true;
        raid.ticks_active = 1234;
        raid.groups_spawned = 2;
        raid.cooldown_ticks = 77;
        raid.post_raid_ticks = 5;
        raid.total_health = 96.5;
        raid.status = RaidStatus::Victory;
        raid.heroes
            .insert(0x0123_4567_89ab_cdef_0011_2233_4455_6677);
        raids.tick = 42;
        let tag = raids.write();
        assert_eq!(id, 2, "vanilla's first raid id: next_id starts at 1");
        assert_eq!(tag.get("next_id").and_then(Tag::as_i64), Some(2));
        let entry = &tag.get("raids").and_then(Tag::as_list).unwrap()[0];
        assert_eq!(entry.get("status").and_then(Tag::as_str), Some("victory"));
        assert_eq!(
            entry.get("center").and_then(Tag::as_ints),
            Some(vec![8, 64, 8])
        );
        assert_eq!(entry.get("group_count").and_then(Tag::as_i64), Some(5));
        let back = Raids::read(&tag);
        assert_eq!((back.next_id, back.tick), (2, 42));
        assert_eq!(
            Raids::read(&Tag::Compound(BTreeMap::new())).next_id,
            FIRST_ID
        );
        let summary = |raid: &Raid| {
            (
                raid.started,
                raid.active,
                raid.ticks_active,
                raid.raid_omen_level,
                raid.groups_spawned,
                raid.cooldown_ticks,
                raid.post_raid_ticks,
                raid.total_health,
                raid.num_groups,
                raid.status,
                raid.center,
                raid.heroes.clone(),
            )
        };
        assert_eq!(summary(&back.raids[&id]), summary(&raids.raids[&id]));
        assert_eq!(back.raids[&id].raid_omen_level, 3);
        let uuid = 0xdead_beef_0000_0001_8000_0000_ffff_ffff;
        assert_eq!(uuid_of_ints(ints_of_uuid(uuid)), uuid);
    }

    #[test]
    fn bad_omen_turns_into_raid_omen_only_inside_a_village() {
        let mut village = Village::new(2);
        let raids = Raids::default();
        let inside = player(0, [9.5, 64.0, 9.5]);
        assert!(raids.omen_turns(&mut village, &inside));
        assert!(!raids.omen_turns(&mut village, &player(0, [200.5, 64.0, 9.5])));
        let mut watcher = inside;
        watcher.spectator = true;
        assert!(!raids.omen_turns(&mut village, &watcher));
        village.difficulty = 0;
        assert!(!raids.omen_turns(&mut village, &inside), "not in peaceful");
        village.difficulty = 2;
        let (raids, id) = started(&mut village, 4);
        assert_eq!(raids.raids[&id].raid_omen_level, MAX_RAID_OMEN_LEVEL);
        assert!(
            !raids.omen_turns(&mut village, &inside),
            "the raid there is at its highest level"
        );
        let mut off = Raids::default();
        assert_eq!(
            off.create_or_extend(&mut village, &inside, BELL, 0, false, 1),
            None,
            "the raids rule"
        );
    }

    #[test]
    fn a_won_raid_makes_every_player_who_killed_raiders_a_hero() {
        let mut village = Village::new(1);
        let (mut raids, id) = started(&mut village, 2);
        let players = [
            player(0, [8.5, 64.0, 8.5]),
            player(5, [20.5, 64.0, 8.5]),
            player(9, [500.5, 64.0, 500.5]),
        ];
        let mut turn = 0u64;
        let mut horns = Vec::new();
        let mut friend_bars = Vec::new();
        let outs = run_until(&mut raids, &mut village, &players, |raids, village, out| {
            horns.extend(out.horns.iter().map(|(player, _)| *player));
            friend_bars.extend(raids.bars_for(5));
            assert!(
                raids.bars_for(9).is_empty(),
                "a player far away sees no bar"
            );
            if !village.living().is_empty() {
                village.kill_all(if turn.is_multiple_of(2) { 0 } else { 5 });
                turn += 1;
            }
            !out.heroes.is_empty()
        });
        let heroes: Vec<(u64, i32)> = outs.iter().flat_map(|out| out.heroes.clone()).collect();
        assert_eq!(heroes, [(0, 2), (5, 2)]);
        assert!(
            friend_bars.iter().any(|bar| bar.id == id),
            "the friend in the village sees the bar"
        );
        assert!(
            horns.contains(&0) && horns.contains(&5) && !horns.contains(&9),
            "{horns:?}"
        );
        let raid = &raids.raids[&id];
        assert_eq!(raid.status, RaidStatus::Victory);
        assert_eq!(raid.groups_spawned, 4, "three waves and the bonus wave");
        for _ in 0..20 {
            raids.tick(&mut village, &players, true, 0);
        }
        assert_eq!(
            raids
                .bars_for(0)
                .first()
                .map(|bar| (bar.name, bar.progress)),
            Some((BarName::Victory, 0.0))
        );
        for _ in 0..MAX_CELEBRATION_TICKS {
            raids.tick(&mut village, &players, true, 0);
        }
        assert!(
            raids.raids.is_empty(),
            "the raid ends after its celebration"
        );
    }

    #[test]
    fn a_village_left_without_its_centre_is_a_lost_raid() {
        let players = [player(0, [8.5, 64.0, 8.5])];
        let mut village = Village::new(2);
        let (mut raids, id) = started(&mut village, 0);
        run_until(&mut raids, &mut village, &players, |raids, _, _| {
            raids.raids[&id].groups_spawned > 0
        });
        assert!(raids.raids[&id].raiders_alive() > 0);
        assert!(village.world.pois.release(BELL));
        raids.tick(&mut village, &players, true, 0);
        let raid = &raids.raids[&id];
        assert_eq!(raid.status, RaidStatus::Loss);
        assert!(raid.view(id).loss && raid.view(id).over);
        for _ in 0..20 {
            raids.tick(&mut village, &players, true, 0);
        }
        assert_eq!(
            raids.bars_for(0).first().map(|bar| bar.name),
            Some(BarName::Defeat)
        );
        let mut unstarted = Village::new(2);
        let (mut raids, id) = started(&mut unstarted, 0);
        assert!(unstarted.world.pois.release(BELL));
        raids.tick(&mut unstarted, &players, true, 0);
        raids.tick(&mut unstarted, &players, true, 0);
        assert!(
            !raids.raids.contains_key(&id),
            "a raid with no wave yet just stops"
        );
    }

    #[test]
    fn a_raid_is_lost_once_the_villagers_holding_the_village_are_dead() {
        let players = [player(0, [8.5, 64.0, 8.5])];
        let mut village = Village::new(2);
        let villager = village.world.spawn_villager_active(
            minecraftoss_entities::villager::Villager::new(DVec3::new(8.5, 64.0, 10.5)),
            0.0,
        );
        let held = village.world.villager_mut(villager).unwrap();
        let ai = held.ai.as_deref_mut().unwrap();
        ai.brain.memories.meeting_point.set(BELL);
        let (mut raids, id) = started(&mut village, 0);
        run_until(&mut raids, &mut village, &players, |raids, _, _| {
            raids.raids[&id].groups_spawned > 0
        });
        raids.tick(&mut village, &players, true, 0);
        assert_eq!(raids.raids[&id].status, RaidStatus::Ongoing);
        village
            .world
            .villager_mut(villager)
            .unwrap()
            .villager
            .health = 0.0;
        village.world.release_all_villager_pois();
        raids.tick(&mut village, &players, true, 0);
        assert_eq!(raids.raids[&id].status, RaidStatus::Loss);
        assert!(raids.views()[0].loss, "its raiders celebrate");
    }

    #[test]
    fn raiders_that_wander_into_a_raid_join_its_current_wave() {
        let mut village = Village::new(2);
        let (mut raids, id) = started(&mut village, 0);
        let pillager_at = |x: f64| {
            let mut pillager = minecraftoss_entities::illager::Illager::new(
                minecraftoss_entities::illager::IllagerKind::Pillager,
                DVec3::new(x, 64.0, 8.5),
            );
            pillager.raider.can_join_raid = true;
            pillager
        };
        let wanderer = village.world.spawn_illager(pillager_at(30.5), true);
        let stranger = village.world.spawn_illager(pillager_at(300.5), true);
        raids.tick(&mut village, &[], true, 20);
        let views = village.world.raiders();
        let joined = views.iter().find(|view| view.id == wanderer).unwrap();
        assert_eq!((joined.raider.raid_id, joined.raider.wave), (Some(id), 0));
        assert_eq!(raids.raids[&id].raiders_alive(), 1);
        assert_eq!(raids.raids[&id].total_health, joined.health);
        let far = views.iter().find(|view| view.id == stranger).unwrap();
        assert_eq!(far.raider.raid_id, None);
        village.kill_all(3);
        raids.tick(&mut village, &[], true, 21);
        assert_eq!(raids.raids[&id].raiders_alive(), 0);
        assert!(raids.raids[&id].heroes.contains(&1003));
    }
}
