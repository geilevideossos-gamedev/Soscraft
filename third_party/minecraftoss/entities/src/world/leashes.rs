use super::*;
use minecraftoss_player::Pos;

pub const SNAP_DISTANCE: f64 = 12.0;
pub const ELASTIC_DISTANCE: f64 = 6.0;
pub const LEAD: &str = "minecraft:lead";
pub const TIED_SOUND: &str = "item.lead.tied";
pub const UNTIED_SOUND: &str = "item.lead.untied";
pub const BREAK_SOUND: &str = "item.lead.break";
pub const SNIP_SOUND: &str = "item.shears.snip";
pub const KNOT_WIDTH: f32 = 0.375;
pub const KNOT_HEIGHT: f32 = 0.5;
pub const KNOT_Y: f64 = 0.375;
const AREA_HALF: f64 = 16.0;
const KNOT_CHECK_TICKS: i32 = 100;
const RESTORE_TICKS: i32 = 100;
const ELASTICITY: DVec3 = DVec3::new(0.8, 0.2, 0.8);
const STIFFNESS: f64 = 0.11;
const SPRING_DAMPENING: f64 = 0.3;
const CLOSE_DISTANCE: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeashHolder {
    Player(u64),
    Mob(u64),
    Knot(Pos),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavedLeash {
    Uuid(u128),
    Knot(Pos),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leash {
    pub holder: Option<LeashHolder>,
    pub saved: Option<SavedLeash>,
    pub waited: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeashPlayer {
    pub id: u64,
    pub position: DVec3,
    pub width: f32,
    pub height: f32,
    pub alive: bool,
    pub movement: DVec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeashUse {
    Pass,
    Tied,
    Untied,
    Moved,
    Snipped,
}

pub fn knot_position(pos: Pos) -> DVec3 {
    DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + KNOT_Y,
        f64::from(pos.2) + 0.5,
    )
}

pub fn is_fence(id: &str) -> bool {
    id.ends_with("_fence")
}

fn center(position: DVec3, height: f32) -> DVec3 {
    position + DVec3::new(0.0, f64::from(height) / 2.0, 0.0)
}

fn in_area(around: DVec3, position: DVec3, width: f32, height: f32) -> bool {
    let half = f64::from(width) / 2.0;
    position.x + half > around.x - AREA_HALF
        && position.x - half < around.x + AREA_HALF
        && position.y + f64::from(height) > around.y - AREA_HALF
        && position.y < around.y + AREA_HALF
        && position.z + half > around.z - AREA_HALF
        && position.z - half < around.z + AREA_HALF
}

fn y_rot(v: DVec3, angle: f64) -> DVec3 {
    let (sin, cos) = (angle.sin(), angle.cos());
    DVec3::new(v.x * cos + v.z * sin, v.y, v.z * cos - v.x * sin)
}

pub fn spring_force(
    holder_point: DVec3,
    mob_point: DVec3,
    elastic: f64,
    movement: DVec3,
) -> Option<DVec3> {
    let distance = mob_point.distance(holder_point);
    if distance < elastic {
        return None;
    }
    let force = (holder_point - mob_point).normalize_or_zero() * (distance - elastic);
    if movement.dot(force) >= 0.0 {
        Some(force * SPRING_DAMPENING)
    } else {
        Some(force)
    }
}

impl EntityWorld {
    pub fn body_of(&self, id: u64) -> Option<&Body> {
        let found = self
            .cows
            .iter()
            .find(|e| e.id == id)
            .map(|e| &e.cow.body)
            .or_else(|| self.sheep.iter().find(|e| e.id == id).map(|e| &e.body))
            .or_else(|| self.pigs.iter().find(|e| e.id == id).map(|e| &e.pig.body))
            .or_else(|| {
                self.chickens
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.chicken.body)
            })
            .or_else(|| {
                self.wolves
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.wolf.body)
            })
            .or_else(|| {
                self.felines
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.feline.body)
            })
            .or_else(|| {
                self.iron_golems
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.golem.body)
            })
            .or_else(|| {
                self.striders
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.strider.body)
            })
            .or_else(|| {
                self.hoglins
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.hoglin.body)
            })
            .or_else(|| {
                self.villagers
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.villager.body)
            })
            .or_else(|| {
                self.wandering_traders
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.trader.body)
            })
            .or_else(|| {
                self.snow_golems
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.golem.body)
            });
        if found.is_some() {
            return found;
        }
        self.bats
            .iter()
            .find(|e| e.id == id)
            .map(|e| &e.bat.body)
            .or_else(|| {
                self.zombies
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.zombie.body)
            })
            .or_else(|| {
                self.skeletons
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.skeleton.body)
            })
            .or_else(|| {
                self.creepers
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.creeper.body)
            })
            .or_else(|| {
                self.spiders
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.spider.body)
            })
            .or_else(|| {
                self.slimes
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.slime.body)
            })
            .or_else(|| {
                self.endermen
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.enderman.body)
            })
            .or_else(|| {
                self.witches
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.witch.body)
            })
            .or_else(|| {
                self.ghasts
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.ghast.body)
            })
            .or_else(|| {
                self.blazes
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.blaze.body)
            })
            .or_else(|| {
                self.piglins
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.piglin.body)
            })
            .or_else(|| {
                self.shulkers
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| &e.shulker.body)
            })
    }

    fn leash_yaw(&self, id: u64) -> f32 {
        None.or_else(|| self.cows.iter().find(|e| e.id == id).map(|e| e.cow.yaw))
            .or_else(|| self.sheep.iter().find(|e| e.id == id).map(|e| e.yaw))
            .or_else(|| self.pigs.iter().find(|e| e.id == id).map(|e| e.pig.yaw))
            .or_else(|| {
                self.chickens
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.chicken.yaw)
            })
            .or_else(|| self.wolves.iter().find(|e| e.id == id).map(|e| e.wolf.yaw))
            .or_else(|| {
                self.felines
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.feline.yaw)
            })
            .or_else(|| {
                self.iron_golems
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.golem.yaw)
            })
            .or_else(|| {
                self.striders
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.strider.yaw)
            })
            .or_else(|| {
                self.hoglins
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.hoglin.yaw)
            })
            .unwrap_or(0.0)
    }

    fn leash_no_ai(&self, id: u64) -> bool {
        self.cows.iter().any(|e| e.id == id && e.no_ai)
            || self.sheep.iter().any(|e| e.id == id && e.no_ai)
            || self.pigs.iter().any(|e| e.id == id && e.no_ai)
            || self.chickens.iter().any(|e| e.id == id && e.no_ai)
            || self.wolves.iter().any(|e| e.id == id && e.no_ai)
            || self.felines.iter().any(|e| e.id == id && e.no_ai)
            || self.iron_golems.iter().any(|e| e.id == id && e.no_ai)
            || self.striders.iter().any(|e| e.id == id && e.no_ai)
            || self.hoglins.iter().any(|e| e.id == id && e.no_ai)
            || self.wandering_traders.iter().any(|e| e.id == id && e.no_ai)
            || self.snow_golems.iter().any(|e| e.id == id && e.no_ai)
    }

    fn leash_panicking(&self, id: u64) -> bool {
        self.cows.iter().any(|e| e.id == id && e.panic.running)
            || self.sheep.iter().any(|e| e.id == id && e.panic.running)
            || self.pigs.iter().any(|e| e.id == id && e.panic.running)
            || self.chickens.iter().any(|e| e.id == id && e.panic.running)
            || self.striders.iter().any(|e| e.id == id && e.panic.running)
    }

    pub fn mob_baby(&self, id: u64) -> bool {
        self.cows.iter().any(|e| e.id == id && e.cow.age.baby())
            || self.sheep.iter().any(|e| e.id == id && e.sheep.age.baby())
            || self.pigs.iter().any(|e| e.id == id && e.pig.age.baby())
            || self
                .chickens
                .iter()
                .any(|e| e.id == id && e.chicken.age.baby())
            || self.wolves.iter().any(|e| e.id == id && e.wolf.baby())
            || self.felines.iter().any(|e| e.id == id && e.feline.baby())
            || self
                .striders
                .iter()
                .any(|e| e.id == id && e.strider.age.baby())
            || self
                .hoglins
                .iter()
                .any(|e| e.id == id && e.hoglin.is_baby())
            || self
                .villagers
                .iter()
                .any(|e| e.id == id && e.villager.age.baby())
    }

    fn mob_alive(&self, id: u64) -> bool {
        self.damage_state(id).is_some_and(|damage| !damage.dead)
    }

    pub fn can_be_leashed(&self, id: u64) -> bool {
        let game_time = self.game_time;
        self.cows.iter().any(|e| e.id == id)
            || self.sheep.iter().any(|e| e.id == id)
            || self.pigs.iter().any(|e| e.id == id)
            || self.chickens.iter().any(|e| e.id == id)
            || self.felines.iter().any(|e| e.id == id)
            || self.iron_golems.iter().any(|e| e.id == id)
            || self.snow_golems.iter().any(|e| e.id == id)
            || self.striders.iter().any(|e| e.id == id)
            || self.hoglins.iter().any(|e| e.id == id)
            || self
                .wolves
                .iter()
                .any(|e| e.id == id && !e.angry(game_time))
    }

    pub fn leashes(&self) -> impl Iterator<Item = (u64, &Leash)> + '_ {
        self.leashes.iter().map(|(&id, leash)| (id, leash))
    }

    pub fn leash_of(&self, mob: u64) -> Option<Leash> {
        self.leashes.get(&mob).copied()
    }

    pub fn leash_holder(&self, mob: u64) -> Option<LeashHolder> {
        self.leashes.get(&mob).and_then(|leash| leash.holder)
    }

    pub fn has_knot(&self, pos: Pos) -> bool {
        self.knots.contains_key(&pos)
    }

    pub fn knots(&self) -> Vec<Pos> {
        let mut out: Vec<Pos> = self.knots.keys().copied().collect();
        for leash in self.leashes.values() {
            if let (None, Some(SavedLeash::Knot(pos))) = (leash.holder, leash.saved) {
                if !out.contains(&pos) {
                    out.push(pos);
                }
            }
        }
        out
    }

    pub fn mobs_leashed_to(&self, holder: LeashHolder) -> Vec<u64> {
        self.leashes
            .iter()
            .filter(|(_, leash)| leash.holder == Some(holder))
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn set_leash_players(&mut self, players: &[crate::tempt::PlayerCandidate]) {
        let before = std::mem::take(&mut self.leash_players);
        self.leash_players = players
            .iter()
            .map(|p| {
                let (width, height) = pushing::player_size(p.eye_height);
                let movement = before
                    .iter()
                    .find(|b| b.id == p.id)
                    .map_or(DVec3::ZERO, |b| p.position - b.position);
                LeashPlayer {
                    id: p.id,
                    position: p.position,
                    width,
                    height,
                    alive: p.alive && !p.spectator,
                    movement,
                }
            })
            .collect();
    }

    pub fn leash_player(&self, player: u64) -> Option<LeashPlayer> {
        self.leash_players.iter().find(|p| p.id == player).copied()
    }

    fn holder_box(&self, holder: LeashHolder) -> Option<(DVec3, f32, f32, DVec3, bool)> {
        match holder {
            LeashHolder::Player(id) => {
                let p = self.leash_player(id)?;
                Some((p.position, p.width, p.height, p.movement, p.alive))
            }
            LeashHolder::Mob(id) => {
                let body = self.body_of(id)?;
                let movement = if self.leash_no_ai(id) {
                    DVec3::ZERO
                } else {
                    body.velocity
                };
                Some((
                    body.position,
                    body.width,
                    body.height,
                    movement,
                    self.mob_alive(id),
                ))
            }
            LeashHolder::Knot(pos) => self.knots.contains_key(&pos).then(|| {
                (
                    knot_position(pos),
                    KNOT_WIDTH,
                    KNOT_HEIGHT,
                    DVec3::ZERO,
                    true,
                )
            }),
        }
    }

    fn leash_distance(&self, mob: u64, holder: LeashHolder) -> Option<f64> {
        let body = self.body_of(mob)?;
        let (position, _, height, _, _) = match holder {
            LeashHolder::Knot(pos) => (
                knot_position(pos),
                KNOT_WIDTH,
                KNOT_HEIGHT,
                DVec3::ZERO,
                true,
            ),
            _ => self.holder_box(holder)?,
        };
        Some(center(body.position, body.height).distance(center(position, height)))
    }

    pub fn can_attach_leash(&self, mob: u64, holder: LeashHolder) -> bool {
        if holder == LeashHolder::Mob(mob) {
            return false;
        }
        self.leash_distance(mob, holder)
            .is_some_and(|distance| distance <= SNAP_DISTANCE)
            && self.can_be_leashed(mob)
    }

    fn sync_llama_leash(&mut self, mob: u64) {
        let holder = self.leash_holder(mob);
        let trader = match holder {
            Some(LeashHolder::Mob(id)) => self
                .wandering_traders
                .iter()
                .find(|e| e.id == id)
                .map(|e| (id, e.trader.despawn_delay)),
            _ => None,
        };
        if let Some(horse) = self
            .cows
            .iter_mut()
            .find(|e| e.id == mob)
            .and_then(|e| e.horse.as_mut())
        {
            horse.leashed = holder.is_some();
            horse.trader = trader;
        }
    }

    pub fn set_leash(&mut self, mob: u64, holder: LeashHolder) {
        if holder == LeashHolder::Mob(mob) {
            return;
        }
        let old = self.leash_holder(mob);
        self.leashes.insert(
            mob,
            Leash {
                holder: Some(holder),
                saved: None,
                waited: 0,
            },
        );
        if let LeashHolder::Knot(pos) = holder {
            self.knots.entry(pos).or_insert(0);
        }
        if let Some(LeashHolder::Knot(pos)) = old.filter(|old| *old != holder) {
            self.forget_empty_knot(pos);
        }
        if self.vehicle_of(Rider::Mob(mob)).is_some() {
            self.unlink(Rider::Mob(mob));
        }
        self.sync_llama_leash(mob);
    }

    pub fn load_leash(&mut self, mob: u64, saved: SavedLeash) {
        self.leashes.insert(
            mob,
            Leash {
                holder: None,
                saved: Some(saved),
                waited: 0,
            },
        );
    }

    pub fn saved_leash(&self, mob: u64) -> Option<SavedLeash> {
        let leash = self.leashes.get(&mob)?;
        match leash.holder {
            Some(LeashHolder::Player(id)) => {
                Some(SavedLeash::Uuid(self.uuid_of(PLAYER_TARGET + id)))
            }
            Some(LeashHolder::Mob(id)) => Some(SavedLeash::Uuid(self.uuid_of(id))),
            Some(LeashHolder::Knot(pos)) => Some(SavedLeash::Knot(pos)),
            None => leash.saved,
        }
    }

    fn forget_empty_knot(&mut self, pos: Pos) {
        let used = self
            .leashes
            .values()
            .any(|leash| leash.holder == Some(LeashHolder::Knot(pos)));
        if !used {
            self.knots.remove(&pos);
        }
    }

    pub fn drop_leash(&mut self, mob: u64, item: bool) {
        let Some(leash) = self.leashes.get(&mob).copied() else {
            return;
        };
        let Some(holder) = leash.holder else {
            return;
        };
        self.leashes.remove(&mob);
        if item {
            if let Some(body) = self.body_of(mob) {
                self.lead_drops.push(body.position);
            }
        }
        if let LeashHolder::Knot(pos) = holder {
            self.forget_empty_knot(pos);
        }
        self.sync_llama_leash(mob);
    }

    pub fn take_lead_drops(&mut self) -> Vec<DVec3> {
        std::mem::take(&mut self.lead_drops)
    }

    fn leash_sound(&mut self, event: &str, position: DVec3, category: &'static str) {
        self.sounds.push(MobSound {
            event: event.to_owned(),
            position,
            volume: 1.0,
            pitch: 1.0,
            category,
        });
    }

    fn leashed_near(&self, holder: LeashHolder, around: DVec3) -> Vec<u64> {
        self.mobs_leashed_to(holder)
            .into_iter()
            .filter(|&id| {
                self.body_of(id)
                    .is_some_and(|b| in_area(around, b.position, b.width, b.height))
            })
            .collect()
    }

    pub fn leash_interact(
        &mut self,
        mob: u64,
        player: u64,
        held: Option<&str>,
        sneaking: bool,
        infinite: bool,
    ) -> LeashUse {
        let Some(body) = self.body_of(mob) else {
            return LeashUse::Pass;
        };
        let (position, mob_center) = (body.position, center(body.position, body.height));
        let alive = self.mob_alive(mob);
        if sneaking && self.can_be_leashed(mob) && alive && !self.mob_baby(mob) {
            let held_here = self.leashed_near(LeashHolder::Player(player), mob_center);
            let mut moved = false;
            for other in held_here {
                if self.can_attach_leash(other, LeashHolder::Mob(mob)) {
                    self.set_leash(other, LeashHolder::Mob(mob));
                    moved = true;
                }
            }
            if moved {
                self.leash_sound(TIED_SOUND, position, "friendly_volume");
                return LeashUse::Moved;
            }
        }
        if held == Some("minecraft:shears") {
            let mut cut = false;
            if self.leash_holder(mob).is_some() {
                self.drop_leash(mob, true);
                cut = true;
            }
            for other in self.leashed_near(LeashHolder::Mob(mob), mob_center) {
                self.drop_leash(other, true);
                cut = true;
            }
            if cut {
                self.leash_sound(SNIP_SOUND, position, "players_volume");
                return LeashUse::Snipped;
            }
        }
        if !alive {
            return LeashUse::Pass;
        }
        let holder = self.leash_holder(mob);
        if holder == Some(LeashHolder::Player(player)) {
            self.drop_leash(mob, !infinite);
            self.leash_sound(UNTIED_SOUND, position, "friendly_volume");
            return LeashUse::Untied;
        }
        if held == Some(LEAD)
            && !matches!(holder, Some(LeashHolder::Player(_)))
            && self.can_attach_leash(mob, LeashHolder::Player(player))
        {
            if holder.is_some() {
                self.drop_leash(mob, true);
            }
            self.set_leash(mob, LeashHolder::Player(player));
            self.leash_sound(TIED_SOUND, position, "friendly_volume");
            return LeashUse::Tied;
        }
        LeashUse::Pass
    }

    pub fn leash_fence(&mut self, player: u64, pos: Pos, sneaking: bool) -> bool {
        let knot = LeashHolder::Knot(pos);
        let mut changed = false;
        if self.knots.contains_key(&pos) {
            let Some(me) = self.leash_player(player) else {
                return false;
            };
            let around = center(me.position, me.height);
            for mob in self.leashed_near(LeashHolder::Player(player), around) {
                if self.can_attach_leash(mob, knot) {
                    self.set_leash(mob, knot);
                    changed = true;
                }
            }
            if !changed && !sneaking {
                let around = center(knot_position(pos), KNOT_HEIGHT);
                for mob in self.leashed_near(knot, around) {
                    if self.can_attach_leash(mob, LeashHolder::Player(player)) {
                        self.set_leash(mob, LeashHolder::Player(player));
                        changed = true;
                    }
                }
            }
        } else {
            let block = DVec3::new(
                f64::from(pos.0) + 0.5,
                f64::from(pos.1) + 0.5,
                f64::from(pos.2) + 0.5,
            );
            for mob in self.leashed_near(LeashHolder::Player(player), block) {
                if self.can_attach_leash(mob, knot) {
                    self.set_leash(mob, knot);
                    changed = true;
                }
            }
        }
        if changed {
            self.leash_sound(TIED_SOUND, knot_position(pos), "friendly_volume");
        }
        changed
    }

    fn restore_leash(&self, mob: u64, saved: SavedLeash) -> Option<LeashHolder> {
        match saved {
            SavedLeash::Knot(pos) => Some(LeashHolder::Knot(pos)),
            SavedLeash::Uuid(uuid) => {
                let player = self
                    .leash_players
                    .iter()
                    .find(|p| self.uuids.get(&(PLAYER_TARGET + p.id)) == Some(&uuid))
                    .map(|p| LeashHolder::Player(p.id));
                player.or_else(|| {
                    self.mob_ids()
                        .find(|&id| id != mob && self.uuids.get(&id) == Some(&uuid))
                        .map(LeashHolder::Mob)
                })
            }
        }
    }

    fn tick_knots(&mut self, world: &impl World) {
        let positions: Vec<Pos> = self.knots.keys().copied().collect();
        for pos in positions {
            let Some(ticks) = self.knots.get_mut(&pos) else {
                continue;
            };
            let check = *ticks == KNOT_CHECK_TICKS;
            *ticks = if check { 0 } else { *ticks + 1 };
            if !check || world.block(pos).is_some_and(|block| is_fence(&block.id)) {
                continue;
            }
            self.knots.remove(&pos);
            self.leash_sound(UNTIED_SOUND, knot_position(pos), "friendly_volume");
            for mob in self.mobs_leashed_to(LeashHolder::Knot(pos)) {
                self.drop_leash(mob, true);
            }
        }
    }

    pub(super) fn tick_leashes(
        &mut self,
        world: &impl World,
        players: &[crate::tempt::PlayerCandidate],
        ticking: &HashSet<u64>,
    ) {
        self.set_leash_players(players);
        if self.leashes.is_empty() && self.knots.is_empty() {
            return;
        }
        let present: HashSet<u64> = self.mob_ids().collect();
        self.leashes.retain(|id, _| present.contains(id));
        self.tick_knots(world);
        let mobs: Vec<u64> = self.leashes.keys().copied().collect();
        for mob in mobs {
            if !ticking.contains(&mob) {
                continue;
            }
            let Some(mut leash) = self.leashes.get(&mob).copied() else {
                continue;
            };
            if leash.holder.is_none() {
                if let Some(saved) = leash.saved {
                    match self.restore_leash(mob, saved) {
                        Some(holder) => {
                            self.set_leash(mob, holder);
                            leash = self.leashes[&mob];
                        }
                        None => {
                            leash.waited += 1;
                            if leash.waited > RESTORE_TICKS {
                                self.leashes.remove(&mob);
                                if let Some(body) = self.body_of(mob) {
                                    self.lead_drops.push(body.position);
                                }
                            } else {
                                self.leashes.insert(mob, leash);
                            }
                            continue;
                        }
                    }
                }
            }
            let Some(holder) = leash.holder else {
                continue;
            };
            self.sync_llama_leash(mob);
            let holder_box = self.holder_box(holder);
            let holder_ok = holder_box.is_some_and(|(_, _, _, _, alive)| alive);
            if !self.mob_alive(mob) || !holder_ok {
                self.drop_leash(mob, true);
                continue;
            }
            let Some((holder_position, holder_width, holder_height, holder_movement, _)) =
                holder_box
            else {
                continue;
            };
            let Some(body) = self.body_of(mob) else {
                continue;
            };
            let (position, width, height, velocity) =
                (body.position, body.width, body.height, body.velocity);
            let distance =
                center(position, height).distance(center(holder_position, holder_height));
            if distance > SNAP_DISTANCE {
                self.leash_sound(BREAK_SOUND, holder_position, "friendly_volume");
                self.drop_leash(mob, true);
                continue;
            }
            let reach = ELASTIC_DISTANCE - f64::from(holder_width) - f64::from(width);
            let mut pulled = false;
            if distance > reach {
                let yaw = f64::from(self.leash_yaw(mob)) * f64::from(0.017_453_292_f32);
                let offset = y_rot(
                    DVec3::new(0.0, f64::from(height) * 0.5, f64::from(width) * 0.5),
                    -yaw,
                );
                let mob_point = position + offset;
                let holder_point =
                    holder_position + DVec3::new(0.0, f64::from(holder_height) * 0.5, 0.0);
                let own = if self.leash_no_ai(mob) {
                    DVec3::ZERO
                } else {
                    velocity
                };
                if let Some(force) = spring_force(holder_point, mob_point, ELASTIC_DISTANCE, own) {
                    let push = force * ELASTICITY + (holder_movement - velocity) * STIFFNESS;
                    if let Some(body) = self.body_mut(mob) {
                        body.velocity += push;
                    }
                    pulled = true;
                }
            }
            if !pulled && !self.leash_panicking(mob) {
                let between = position.distance(holder_position);
                let toward = (holder_position - position).normalize_or_zero()
                    * (between - CLOSE_DISTANCE).max(0.0);
                let speed = self.leash_follow_speed(mob);
                self.leash_walk(mob, position + toward, speed, world);
            }
        }
    }

    fn leash_follow_speed(&self, mob: u64) -> f64 {
        let llama = self
            .cows
            .iter()
            .any(|e| e.id == mob && e.horse.as_ref().is_some_and(|h| h.kind.is_llama()));
        if llama { 2.0 } else { 1.0 }
    }

    fn leash_walk(&mut self, mob: u64, target: DVec3, speed: f64, world: &impl World) {
        if self.leash_no_ai(mob) {
            return;
        }
        if let Some(e) = self.cows.iter_mut().find(|e| e.id == mob) {
            let fluid = e.fluid;
            let _ = e.cow.navigate_to(world, fluid, target, speed);
        } else if let Some(e) = self.sheep.iter_mut().find(|e| e.id == mob) {
            let profile = WalkProfile::animal(e.body.width, e.body.height);
            let _ = navigate_walk_to(
                &e.body,
                &mut e.navigation,
                world,
                &profile,
                e.fluid,
                target,
                speed,
                1,
            );
        } else if let Some(e) = self.pigs.iter_mut().find(|e| e.id == mob) {
            let fluid = e.fluid;
            let _ = e.pig.navigate_to(world, fluid, target, speed);
        } else if let Some(e) = self.chickens.iter_mut().find(|e| e.id == mob) {
            let fluid = e.fluid;
            let _ = e.chicken.navigate_to(world, fluid, target, speed);
        } else if let Some(e) = self.striders.iter_mut().find(|e| e.id == mob) {
            let fluid = e.fluid;
            let _ = e.strider.navigate_to(world, fluid, target, speed);
        } else if let Some(e) = self.hoglins.iter_mut().find(|e| e.id == mob) {
            let profile = WalkProfile::animal(e.hoglin.body.width, e.hoglin.body.height);
            let _ = navigate_walk_to(
                &e.hoglin.body,
                &mut e.navigation,
                world,
                &profile,
                e.fluid,
                target,
                speed,
                1,
            );
        } else if let Some(e) = self.wolves.iter_mut().find(|e| e.id == mob) {
            let state = &mut e.ai.state;
            let _ = navigate_walk_to(
                &e.wolf.body,
                &mut state.navigation,
                world,
                &state.walk,
                state.fluid,
                target,
                speed,
                1,
            );
        } else if let Some(e) = self.felines.iter_mut().find(|e| e.id == mob) {
            let state = &mut e.ai.state;
            let _ = navigate_walk_to(
                &e.feline.body,
                &mut state.navigation,
                world,
                &state.walk,
                state.fluid,
                target,
                speed,
                1,
            );
        } else if let Some(e) = self.iron_golems.iter_mut().find(|e| e.id == mob) {
            let state = &mut e.ai.state;
            let _ = navigate_walk_to(
                &e.golem.body,
                &mut state.navigation,
                world,
                &state.walk,
                state.fluid,
                target,
                speed,
                1,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cow::Cow;
    use crate::tempt::PlayerCandidate;
    use crate::zombie::Zombie;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Blocks {
        blocks: BTreeMap<Pos, Block>,
    }

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.blocks.get(&pos).cloned()
        }
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.blocks.insert(pos, block),
                None => self.blocks.remove(&pos),
            };
        }
    }

    fn floor(radius: i32) -> Blocks {
        let mut world = Blocks::default();
        for x in -radius..=radius {
            for z in -radius..=radius {
                world
                    .blocks
                    .insert((x, 63, z), Block::new("minecraft:stone"));
            }
        }
        world
    }

    fn player(id: u64, position: DVec3) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position,
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

    fn cow_at(entities: &mut EntityWorld, at: DVec3) -> u64 {
        entities.spawn_cow(Cow::new(at), false)
    }

    fn sounds(entities: &mut EntityWorld) -> Vec<String> {
        std::mem::take(&mut entities.sounds)
            .into_iter()
            .map(|s| s.event)
            .collect()
    }

    #[test]
    fn a_lead_ties_a_cow_to_its_player_and_a_second_click_unties_it() {
        let mut entities = EntityWorld::default();
        let cow = cow_at(&mut entities, DVec3::new(2.5, 64.0, 0.5));
        entities.set_leash_players(&[player(0, DVec3::new(0.5, 64.0, 0.5))]);
        assert_eq!(
            entities.leash_interact(cow, 0, Some("minecraft:wheat"), false, false),
            LeashUse::Pass
        );
        assert_eq!(
            entities.leash_interact(cow, 0, Some(LEAD), false, false),
            LeashUse::Tied
        );
        assert_eq!(entities.leash_holder(cow), Some(LeashHolder::Player(0)));
        assert_eq!(sounds(&mut entities), [TIED_SOUND]);
        assert_eq!(
            entities.leash_interact(cow, 0, Some("minecraft:wheat"), false, false),
            LeashUse::Untied
        );
        assert_eq!(entities.leash_holder(cow), None);
        assert_eq!(sounds(&mut entities), [UNTIED_SOUND]);
        assert_eq!(entities.take_lead_drops(), [DVec3::new(2.5, 64.0, 0.5)]);
        entities.leash_interact(cow, 0, Some(LEAD), false, true);
        entities.leash_interact(cow, 0, None, false, true);
        assert!(
            entities.take_lead_drops().is_empty(),
            "creative keeps the lead"
        );
    }

    #[test]
    fn monsters_and_villagers_take_no_lead_and_golems_do() {
        let mut entities = EntityWorld::default();
        let at = DVec3::new(1.5, 64.0, 0.5);
        let zombie = entities.spawn_zombie(Zombie::new(at), true);
        let villager = entities.spawn_villager(crate::villager::Villager::new(at), true);
        let golem = entities.spawn_iron_golem(crate::iron_golem::IronGolem::new(at), true);
        entities.set_leash_players(&[player(0, DVec3::new(0.5, 64.0, 0.5))]);
        for mob in [zombie, villager] {
            assert!(!entities.can_be_leashed(mob));
            assert_eq!(
                entities.leash_interact(mob, 0, Some(LEAD), false, false),
                LeashUse::Pass
            );
        }
        assert!(entities.can_be_leashed(golem));
    }

    #[test]
    fn a_lead_breaks_beyond_twelve_blocks_with_its_sound_and_drops_the_lead() {
        let mut world = floor(40);
        let mut entities = EntityWorld::default();
        let cow = cow_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        entities.set_leash_players(&[player(0, DVec3::new(0.5, 64.0, 2.5))]);
        entities.set_leash(cow, LeashHolder::Player(0));
        entities.tick_with_players(&mut world, &[player(0, DVec3::new(0.5, 64.0, 11.5))]);
        assert_eq!(entities.leash_holder(cow), Some(LeashHolder::Player(0)));
        let feet = entities.body_of(cow).unwrap().position;
        let far = DVec3::new(feet.x, 64.0, feet.z + 12.2);
        entities.tick_with_players(&mut world, &[player(0, far)]);
        assert_eq!(entities.leash_holder(cow), None);
        assert_eq!(entities.take_lead_drops().len(), 1);
        let events: Vec<_> = entities
            .take_sounds()
            .into_iter()
            .filter(|s| s.event == BREAK_SOUND)
            .collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].position, far);
    }

    #[test]
    fn a_leashed_cow_is_pulled_after_its_player_and_walks_up_behind_it() {
        let mut world = floor(40);
        let mut entities = EntityWorld::default();
        let cow = cow_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let mut at = DVec3::new(0.5, 64.0, 1.5);
        entities.set_leash_players(&[player(0, at)]);
        entities.set_leash(cow, LeashHolder::Player(0));
        for _ in 0..200 {
            at.z += 0.05;
            entities.tick_with_players(&mut world, &[player(0, at)]);
        }
        let feet = entities.body_of(cow).unwrap().position;
        assert_eq!(entities.leash_holder(cow), Some(LeashHolder::Player(0)));
        assert!(feet.z > 6.0, "the cow follows: {feet}");
        assert!(at.z - feet.z < 9.0, "{feet} behind {at}");
        let pulled = spring_force(
            DVec3::new(0.0, 1.0, 9.0),
            DVec3::new(0.0, 1.0, 0.0),
            ELASTIC_DISTANCE,
            DVec3::ZERO,
        )
        .unwrap();
        assert!((pulled.z - 0.9).abs() < 1e-9, "damped: {pulled}");
        let toward = spring_force(
            DVec3::new(0.0, 1.0, 9.0),
            DVec3::new(0.0, 1.0, 0.0),
            ELASTIC_DISTANCE,
            DVec3::new(0.0, 0.0, -0.1),
        )
        .unwrap();
        assert!((toward.z - 3.0).abs() < 1e-9, "full: {toward}");
        assert!(
            spring_force(
                DVec3::ZERO,
                DVec3::new(0.0, 0.0, 5.9),
                ELASTIC_DISTANCE,
                DVec3::ZERO
            )
            .is_none()
        );
    }

    #[test]
    fn a_fence_ties_the_players_mobs_to_a_knot_and_hands_them_back() {
        let mut world = floor(20);
        world
            .blocks
            .insert((3, 64, 0), Block::new("minecraft:oak_fence"));
        let mut entities = EntityWorld::default();
        let cow = cow_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let other = cow_at(&mut entities, DVec3::new(0.5, 64.0, 2.5));
        let me = player(0, DVec3::new(1.5, 64.0, 0.5));
        entities.set_leash_players(&[me]);
        assert!(!entities.leash_fence(0, (3, 64, 0), false), "nothing held");
        assert!(!entities.has_knot((3, 64, 0)));
        entities.set_leash(cow, LeashHolder::Player(0));
        entities.set_leash(other, LeashHolder::Player(0));
        assert!(entities.leash_fence(0, (3, 64, 0), false));
        assert_eq!(entities.knots(), [(3, 64, 0)]);
        assert_eq!(
            entities.mobs_leashed_to(LeashHolder::Knot((3, 64, 0))),
            [cow, other]
        );
        assert_eq!(sounds(&mut entities), [TIED_SOUND]);
        assert!(
            !entities.leash_fence(0, (3, 64, 0), true),
            "sneaking keeps them tied"
        );
        assert!(entities.leash_fence(0, (3, 64, 0), false));
        assert_eq!(
            entities.mobs_leashed_to(LeashHolder::Player(0)),
            [cow, other]
        );
        assert!(!entities.has_knot((3, 64, 0)), "an empty knot goes");
        entities.leash_fence(0, (3, 64, 0), false);
        for _ in 0..50 {
            entities.tick_with_players(&mut world, &[me]);
        }
        assert_eq!(
            entities.leash_holder(cow),
            Some(LeashHolder::Knot((3, 64, 0)))
        );
    }

    #[test]
    fn a_knot_lets_its_mobs_go_at_its_check_once_its_fence_is_gone() {
        let mut world = floor(20);
        world
            .blocks
            .insert((3, 64, 0), Block::new("minecraft:oak_fence"));
        let mut entities = EntityWorld::default();
        let cow = cow_at(&mut entities, DVec3::new(2.5, 64.0, 0.5));
        let me = player(0, DVec3::new(1.5, 64.0, 0.5));
        entities.set_leash_players(&[me]);
        entities.set_leash(cow, LeashHolder::Knot((3, 64, 0)));
        world.blocks.remove(&(3, 64, 0));
        for _ in 0..100 {
            entities.tick_with_players(&mut world, &[me]);
        }
        assert_eq!(
            entities.leash_holder(cow),
            Some(LeashHolder::Knot((3, 64, 0)))
        );
        entities.tick_with_players(&mut world, &[me]);
        assert_eq!(entities.leash_holder(cow), None);
        assert!(entities.knots().is_empty());
        assert_eq!(entities.take_lead_drops().len(), 1);
    }

    #[test]
    fn a_saved_leash_finds_its_player_by_uuid_or_drops_after_a_hundred_ticks() {
        let mut world = floor(20);
        let mut entities = EntityWorld::default();
        let cow = cow_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let lost = cow_at(&mut entities, DVec3::new(4.5, 64.0, 0.5));
        entities.set_player_uuid(3, 0xabc);
        entities.load_leash(cow, SavedLeash::Uuid(0xabc));
        entities.load_leash(lost, SavedLeash::Uuid(0xdef));
        assert_eq!(entities.saved_leash(cow), Some(SavedLeash::Uuid(0xabc)));
        let me = player(3, DVec3::new(1.5, 64.0, 0.5));
        entities.tick_with_players(&mut world, &[me]);
        assert_eq!(entities.leash_holder(cow), Some(LeashHolder::Player(3)));
        assert_eq!(entities.saved_leash(cow), Some(SavedLeash::Uuid(0xabc)));
        for _ in 0..99 {
            entities.tick_with_players(&mut world, &[me]);
        }
        assert!(entities.leash_of(lost).is_some());
        entities.tick_with_players(&mut world, &[me]);
        assert!(entities.leash_of(lost).is_none());
        assert_eq!(entities.take_lead_drops().len(), 1);
        entities.load_leash(lost, SavedLeash::Knot((9, 64, 9)));
        assert_eq!(entities.knots(), [(9, 64, 9)]);
    }

    #[test]
    fn shears_cut_a_cow_loose_and_sneaking_ties_the_players_mobs_to_another() {
        let mut entities = EntityWorld::default();
        let cow = cow_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let other = cow_at(&mut entities, DVec3::new(2.5, 64.0, 0.5));
        entities.set_leash_players(&[player(0, DVec3::new(1.5, 64.0, 0.5))]);
        entities.set_leash(cow, LeashHolder::Player(0));
        assert_eq!(
            entities.leash_interact(other, 0, None, true, false),
            LeashUse::Moved
        );
        assert_eq!(entities.leash_holder(cow), Some(LeashHolder::Mob(other)));
        assert_eq!(
            entities.leash_interact(other, 0, Some("minecraft:shears"), false, false),
            LeashUse::Snipped
        );
        assert_eq!(entities.leash_holder(cow), None);
        assert_eq!(sounds(&mut entities), [TIED_SOUND, SNIP_SOUND]);
        assert_eq!(entities.take_lead_drops().len(), 1);
    }

    #[test]
    fn a_leashed_mob_stays_and_a_lead_to_a_gone_mob_drops() {
        let mut world = floor(20);
        let mut entities = EntityWorld::default();
        let cow = cow_at(&mut entities, DVec3::new(0.5, 64.0, 0.5));
        let holder = cow_at(&mut entities, DVec3::new(2.5, 64.0, 0.5));
        entities.set_leash(cow, LeashHolder::Mob(holder));
        let census = entities.census();
        assert!(
            census
                .iter()
                .any(|entry| entry.persistent && entry.position.x < 1.0)
        );
        assert!(
            census
                .iter()
                .any(|entry| !entry.persistent && entry.position.x > 2.0)
        );
        entities.remove_mobs(&[holder]);
        entities.tick_with_players(&mut world, &[]);
        assert_eq!(entities.leash_holder(cow), None);
        assert_eq!(entities.take_lead_drops().len(), 1);
    }
}
