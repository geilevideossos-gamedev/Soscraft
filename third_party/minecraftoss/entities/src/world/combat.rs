//! A player's melee hit on a mob (26.3 `Player.attack`, the server's half):
//! the weapon's damage scaled by the attack strength, criticals, the
//! sprinting knockback attack and the sword's sweep; then each mob's
//! `hurtServer` with `player_attack` (armor, the damage cooldown, the
//! default knockback away from the player) and its reaction: animals panic
//! (`#panic_causes`), creepers remember the attacker for `HurtByTargetGoal`,
//! and zombies and skeletons turn on the player.
use super::*;
use minecraftoss_player::minecraft_sin_cos;

/// What the attacking player brings to `Player.attack`. The player runs on
/// the client, which works these out from its state and held item.
#[derive(Clone, Copy, Debug)]
pub struct PlayerAttack {
    pub player_id: u64,
    /// Feet position and facing.
    pub position: DVec3,
    pub yaw: f32,
    /// `ATTACK_DAMAGE` with the main hand's modifiers.
    pub attack_damage: f64,
    /// `getAttackStrengthScale(0.5)` before the attack resets it.
    pub strength: f32,
    pub sprinting: bool,
    /// `canCriticalAttack`'s player half: falling, off the ground, not
    /// climbing, in water, riding, restricted or sprinting.
    pub can_critical: bool,
    /// `isSweepAttack`'s player half: on the ground, slower than two and a
    /// half times its speed, a sword in the main hand.
    pub can_sweep: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ProjectileHit {
    pub amount: f32,
    pub kind: &'static str,
    pub player: u64,
    pub velocity: DVec3,
}

pub const NO_ANGER_FROM_WIND_CHARGE: [&str; 9] = [
    "minecraft:breeze",
    "minecraft:skeleton",
    "minecraft:bogged",
    "minecraft:stray",
    "minecraft:zombie",
    "minecraft:husk",
    "minecraft:spider",
    "minecraft:cave_spider",
    "minecraft:slime",
];

/// What an attack did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttackResult {
    /// The target took damage (`hurtOrSimulate`).
    pub hurt: bool,
    pub died: bool,
    /// A full hit: outside the target's damage cooldown.
    pub full_hit: bool,
    pub full_strength: bool,
    /// Sprinting at full strength: extra knockback; the player slows and
    /// stops sprinting.
    pub knockback: bool,
    pub critical: bool,
    pub sweep: bool,
    /// The mobs the sweep hurt: ID, full hit, died.
    pub swept: Vec<(u64, bool, bool)>,
}

/// `LivingEntity.knockback(power, xd, zd)`: away along (xd, zd), halving
/// the motion, and up off the ground. Too small a direction is replaced by
/// a random one from the mob's own random.
fn knockback(body: &mut Body, random: &mut LegacyRandom, power: f64, mut xd: f64, mut zd: f64) {
    if power <= 0.0 {
        return;
    }
    while xd * xd + zd * zd < f64::from(1.0e-5_f32) {
        xd = (random.next_double() - random.next_double()) * 0.01;
        zd = (random.next_double() - random.next_double()) * 0.01;
    }
    let length = (xd * xd + zd * zd).sqrt();
    let (dx, dz) = (xd / length * power, zd / length * power);
    let v = body.velocity;
    let y = if body.on_ground { (v.y / 2.0 + power).min(0.4) } else { v.y };
    body.velocity = DVec3::new(v.x / 2.0 - dx, y, v.z / 2.0 - dz);
    body.needs_sync = true;
}

impl EntityWorld {
    /// `Player.attack` on the mob `target`.
    pub fn player_attack(&mut self, attack: &PlayerAttack, target: u64) -> AttackResult {
        self.player_attack_with(attack, target, 0.0, 0.0)
    }

    pub fn player_attack_with(&mut self, attack: &PlayerAttack, target: u64, enchanted: f32, extra_knockback: f32) -> AttackResult {
        let mut result = AttackResult::default();
        let scale = attack.strength;
        let magic = enchanted * scale;
        let mut base = attack.attack_damage as f32 * (0.2 + scale * scale * 0.8);
        if !(base > 0.0 || magic > 0.0) {
            return result;
        }
        result.full_strength = scale > 0.9;
        result.knockback = attack.sprinting && result.full_strength;
        result.critical = result.full_strength && attack.can_critical;
        if result.critical {
            base *= 1.5;
        }
        result.sweep = result.full_strength && !result.critical && !result.knockback && attack.can_sweep;
        let Some(hit) = self.hurt_by_player(target, base + magic, attack) else { return AttackResult::default() };
        (result.hurt, result.died, result.full_hit) = (hit.applied, hit.died, hit.full);
        if !hit.applied {
            return result;
        }
        // `setLastHurtMob` at the player's tick count.
        let victim = if self.villagers.iter().any(|e| e.id == target) { crate::monster_ai::Target::Villager(target) } else { crate::monster_ai::Target::Mob(target) };
        let fights = self.player_fights.entry(attack.player_id).or_default();
        fights.hurt_mob = Some((victim, fights.tick_count));
        // `causeExtraKnockback` along the player's facing.
        let (sin, cos) = minecraft_sin_cos(f64::from(attack.yaw));
        let knockback = extra_knockback + if result.knockback { 0.5 } else { 0.0 };
        if knockback > 0.0 {
            self.knock_back(target, f64::from(knockback), sin, -cos);
        }
        if result.sweep {
            let Some((target_body, _)) = self.mob_body(target) else { return result };
            let half = f64::from(target_body.width / 2.0);
            let p = target_body.position;
            let around = [p.x - half, p.y, p.z - half, p.x + half, p.y + f64::from(target_body.height), p.z + half];
            result.swept = self.player_sweep(attack, around, Some(target), enchanted);
        }
        result
    }

    pub fn player_sweep(&mut self, attack: &PlayerAttack, around: [f64; 6], target: Option<u64>, enchanted: f32) -> Vec<(u64, bool, bool)> {
        let damage = (1.0_f32 + enchanted) * attack.strength;
        let (sin, cos) = minecraft_sin_cos(f64::from(attack.yaw));
        let min = DVec3::new(around[0] - 1.0, around[1] - 0.25, around[2] - 1.0);
        let max = DVec3::new(around[3] + 1.0, around[4] + 0.25, around[5] + 1.0);
        let mut swept = Vec::new();
        for key in self.order.clone() {
            let id = key.id();
            if Some(id) == target {
                continue;
            }
            let Some((body, _)) = self.mob_body(id) else { continue };
            let half = f64::from(body.width / 2.0);
            let q = body.position;
            let touches = q.x - half < max.x && q.x + half > min.x && q.y < max.y && q.y + f64::from(body.height) > min.y && q.z - half < max.z && q.z + half > min.z;
            if !touches || attack.position.distance_squared(q) >= 9.0 {
                continue;
            }
            if let Some(hit) = self.hurt_by_player(id, damage, attack) {
                if hit.applied {
                    self.knock_back(id, f64::from(0.4_f32), sin, -cos);
                    swept.push((id, hit.full, hit.died));
                }
            }
        }
        swept
    }

    /// A living mob's body and health, by ID.
    pub(super) fn mob_body(&self, id: u64) -> Option<(Body, f32)> {
        if let Some(e) = self.witches.iter().find(|e| e.id == id) {
            return Some((e.witch.body.clone(), e.witch.health));
        }
        if let Some(e) = self.bats.iter().find(|e| e.id == id) {
            return Some((e.bat.body.clone(), e.bat.health));
        }
        if let Some(e) = self.zombies.iter().find(|e| e.id == id) {
            return Some((e.zombie.body.clone(), e.zombie.health));
        }
        if let Some(e) = self.skeletons.iter().find(|e| e.id == id) {
            return Some((e.skeleton.body.clone(), e.skeleton.health));
        }
        if let Some(e) = self.creepers.iter().find(|e| e.id == id && !e.creeper.exploded) {
            return Some((e.creeper.body.clone(), e.creeper.health));
        }
        if let Some(e) = self.spiders.iter().find(|e| e.id == id) {
            return Some((e.spider.body.clone(), e.spider.health));
        }
        if let Some(e) = self.slimes.iter().find(|e| e.id == id) {
            return Some((e.slime.body.clone(), e.slime.health));
        }
        if let Some(e) = self.endermen.iter().find(|e| e.id == id) {
            return Some((e.enderman.body.clone(), e.enderman.health));
        }
        if let Some(e) = self.villagers.iter().find(|e| e.id == id) {
            return Some((e.villager.body.clone(), e.villager.health));
        }
        if let Some(e) = self.iron_golems.iter().find(|e| e.id == id) {
            return Some((e.golem.body.clone(), e.golem.health));
        }
        if let Some(e) = self.wolves.iter().find(|e| e.id == id) {
            return Some((e.wolf.body.clone(), e.wolf.health));
        }
        if let Some(e) = self.felines.iter().find(|e| e.id == id) {
            return Some((e.feline.body.clone(), e.feline.health));
        }
        if let Some(e) = self.wandering_traders.iter().find(|e| e.id == id) {
            return Some((e.trader.body.clone(), e.trader.health));
        }
        if let Some(e) = self.snow_golems.iter().find(|e| e.id == id) {
            return Some((e.golem.body.clone(), e.golem.health));
        }
        if let Some(e) = self.illagers.iter().find(|e| e.id == id) {
            return Some((e.illager.body.clone(), e.illager.health));
        }
        if let Some(e) = self.vexes.iter().find(|e| e.id == id) {
            return Some((e.vex.body.clone(), e.vex.health));
        }
        if let Some(e) = self.ravagers.iter().find(|e| e.id == id) {
            return Some((e.ravager.body.clone(), e.ravager.health));
        }
        if let Some(e) = self.withers.iter().find(|e| e.id == id) {
            return Some((e.wither.body.clone(), e.wither.health));
        }
        if let Some(e) = self.cows.iter().find(|e| e.id == id) {
            return Some((e.cow.body.clone(), e.cow.health));
        }
        if let Some(e) = self.sheep.iter().find(|e| e.id == id) {
            return Some((e.body.clone(), e.health));
        }
        if let Some(e) = self.pigs.iter().find(|e| e.id == id) {
            return Some((e.pig.body.clone(), e.pig.health));
        }
        if let Some(e) = self.chickens.iter().find(|e| e.id == id) {
            return Some((e.chicken.body.clone(), e.chicken.health));
        }
        if let Some(e) = self.ghasts.iter().find(|e| e.id == id) {
            return Some((e.ghast.body.clone(), e.ghast.health));
        }
        if let Some(e) = self.blazes.iter().find(|e| e.id == id) {
            return Some((e.blaze.body.clone(), e.blaze.health));
        }
        if let Some(e) = self.striders.iter().find(|e| e.id == id) {
            return Some((e.strider.body.clone(), e.strider.health));
        }
        if let Some(e) = self.hoglins.iter().find(|e| e.id == id) {
            return Some((e.hoglin.body.clone(), e.hoglin.health));
        }
        if let Some(e) = self.piglins.iter().find(|e| e.id == id) {
            return Some((e.piglin.body.clone(), e.piglin.health));
        }
        if let Some(e) = self.shulkers.iter().find(|e| e.id == id) {
            return Some((e.shulker.body.clone(), e.shulker.health));
        }
        None
    }

    /// `LivingEntity.knockback` on a mob, by ID.
    pub(super) fn knock_back(&mut self, id: u64, power: f64, xd: f64, zd: f64) {
        let power = if self.hoglins.iter().any(|e| e.id == id) { power * (1.0 - crate::hoglin::KNOCKBACK_RESISTANCE) } else { power };
        let power = if self.ravagers.iter().any(|e| e.id == id) { power * (1.0 - crate::ravager::KNOCKBACK_RESISTANCE) } else { power };
        let (body, random) = if let Some(e) = self.witches.iter_mut().find(|e| e.id == id) {
            (&mut e.witch.body, &mut e.random)
        } else if let Some(e) = self.bats.iter_mut().find(|e| e.id == id) {
            (&mut e.bat.body, &mut e.random)
        } else if let Some(e) = self.zombies.iter_mut().find(|e| e.id == id) {
            (&mut e.zombie.body, &mut e.random)
        } else if let Some(e) = self.skeletons.iter_mut().find(|e| e.id == id) {
            (&mut e.skeleton.body, &mut e.random)
        } else if let Some(e) = self.creepers.iter_mut().find(|e| e.id == id) {
            (&mut e.creeper.body, &mut e.random)
        } else if let Some(e) = self.spiders.iter_mut().find(|e| e.id == id) {
            (&mut e.spider.body, &mut e.random)
        } else if let Some(e) = self.slimes.iter_mut().find(|e| e.id == id) {
            (&mut e.slime.body, &mut e.random)
        } else if let Some(e) = self.endermen.iter_mut().find(|e| e.id == id) {
            (&mut e.enderman.body, &mut e.random)
        } else if let Some(e) = self.villagers.iter_mut().find(|e| e.id == id) {
            (&mut e.villager.body, &mut e.random)
        } else if self.iron_golems.iter().any(|e| e.id == id) || self.shulkers.iter().any(|e| e.id == id) {
            // `KNOCKBACK_RESISTANCE` 1: knockback has no power left.
            return;
        } else if let Some(e) = self.wolves.iter_mut().find(|e| e.id == id) {
            (&mut e.wolf.body, &mut e.random)
        } else if let Some(e) = self.felines.iter_mut().find(|e| e.id == id) {
            (&mut e.feline.body, &mut e.random)
        } else if let Some(e) = self.wandering_traders.iter_mut().find(|e| e.id == id) {
            (&mut e.trader.body, &mut e.random)
        } else if let Some(e) = self.snow_golems.iter_mut().find(|e| e.id == id) {
            (&mut e.golem.body, &mut e.random)
        } else if let Some(e) = self.illagers.iter_mut().find(|e| e.id == id) {
            (&mut e.illager.body, &mut e.random)
        } else if let Some(e) = self.vexes.iter_mut().find(|e| e.id == id) {
            (&mut e.vex.body, &mut e.random)
        } else if let Some(e) = self.ravagers.iter_mut().find(|e| e.id == id) {
            (&mut e.ravager.body, &mut e.random)
        } else if let Some(e) = self.withers.iter_mut().find(|e| e.id == id) {
            (&mut e.wither.body, &mut e.random)
        } else if let Some(e) = self.cows.iter_mut().find(|e| e.id == id) {
            (&mut e.cow.body, &mut e.random)
        } else if let Some(e) = self.sheep.iter_mut().find(|e| e.id == id) {
            (&mut e.body, &mut e.random)
        } else if let Some(e) = self.pigs.iter_mut().find(|e| e.id == id) {
            (&mut e.pig.body, &mut e.random)
        } else if let Some(e) = self.chickens.iter_mut().find(|e| e.id == id) {
            (&mut e.chicken.body, &mut e.random)
        } else if let Some(e) = self.ghasts.iter_mut().find(|e| e.id == id) {
            (&mut e.ghast.body, &mut e.random)
        } else if let Some(e) = self.blazes.iter_mut().find(|e| e.id == id) {
            (&mut e.blaze.body, &mut e.random)
        } else if let Some(e) = self.striders.iter_mut().find(|e| e.id == id) {
            (&mut e.strider.body, &mut e.random)
        } else if let Some(e) = self.hoglins.iter_mut().find(|e| e.id == id) {
            (&mut e.hoglin.body, &mut e.random)
        } else if let Some(e) = self.piglins.iter_mut().find(|e| e.id == id) {
            (&mut e.piglin.body, &mut e.random)
        } else {
            return;
        };
        knockback(body, random, power, xd, zd);
    }

    /// `hurtServer` with `player_attack` on a mob: its armor, the damage
    /// cooldown, the attacker remembered, and on a full hit the default
    /// knockback away from the player. `None` when no such mob lives.
    fn hurt_by_player(&mut self, id: u64, amount: f32, attack: &PlayerAttack) -> Option<DamageResult> {
        let hit = self.hurt_for_player(id, amount, attack.player_id, "minecraft:player_attack", DamageSourceKind::PlayerAttack, true)?;
        if hit.applied && hit.full {
            if let Some((body, _)) = self.mob_body(id) {
                let (xd, zd) = (attack.position.x - body.position.x, attack.position.z - body.position.z);
                self.knock_back(id, f64::from(0.4_f32), xd, zd);
            }
        }
        Some(hit)
    }

    pub fn player_projectile_hurt(&mut self, id: u64, hit: ProjectileHit) -> Option<DamageResult> {
        let refused = DamageResult { applied: false, dealt: 0.0, died: false, full: false };
        if let Some(e) = self.endermen.iter_mut().find(|e| e.id == id) {
            e.dodge_pending = true;
            return Some(refused);
        }
        if hit.kind == "minecraft:trident" && self.shulkers.iter().any(|e| e.id == id && e.refuses_arrows()) {
            return Some(refused);
        }
        let angers = hit.kind != "minecraft:wind_charge" || !self.entity_type(id).is_some_and(|kind| NO_ANGER_FROM_WIND_CHARGE.contains(&kind));
        let result = self.hurt_for_player(id, hit.amount, hit.player, hit.kind, DamageSourceKind::Projectile, angers)?;
        if result.applied && result.full {
            self.knock_back(id, f64::from(0.4_f32), -hit.velocity.x, -hit.velocity.z);
        }
        Some(result)
    }

    fn hurt_for_player(&mut self, id: u64, amount: f32, player: u64, kind: &'static str, source: DamageSourceKind, angers: bool) -> Option<DamageResult> {
        let hit = if let Some(e) = self.bats.iter_mut().find(|e| e.id == id) {
            e.hurt(amount)
        } else if let Some(e) = self.zombies.iter_mut().find(|e| e.id == id) {
            // `Zombie.createAttributes`: 2 armor.
            let hit = e.hurt(damage_after_armor(amount, 2.0, 0.0));
            if hit.applied && angers {
                if let Some(ai) = e.ai.as_deref_mut() {
                    // `setLastHurtByMob`, for `HurtByTargetGoal`.
                    ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
                } else {
                    // The attack-only profile: its `HurtByTargetGoal` outranks
                    // the other targets.
                    e.target_player_id = Some(player);
                    e.target_villager_id = None;
                }
            }
            hit
        } else if let Some(e) = self.skeletons.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied && angers {
                if let Some(ai) = e.ai.as_deref_mut() {
                    ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
                } else {
                    e.target_player_id = Some(player);
                }
            }
            hit
        } else if let Some(e) = self.creepers.iter_mut().find(|e| e.id == id && !e.creeper.exploded) {
            let hit = e.hurt(amount);
            if hit.applied {
                // `setLastHurtByMob`, stamped with its tick count.
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.spiders.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied && angers {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.slimes.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(damage_after_armor(amount, e.slime.armor(), 0.0));
            if hit.applied && angers {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.endermen.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.witches.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount, false, false);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.villagers.iter_mut().find(|e| e.id == id) {
            e.hurt_from(amount, kind, Some(PLAYER_TARGET + player), self.game_time)
        } else if let Some(e) = self.iron_golems.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.wolves.iter_mut().find(|e| e.id == id) {
            let time = self.game_time;
            e.hurt_from(amount, kind, Some(crate::monster_ai::Target::Player(player)), time)
        } else if let Some(e) = self.felines.iter_mut().find(|e| e.id == id) {
            let time = self.game_time;
            e.hurt_from(amount, kind, Some(crate::monster_ai::Target::Player(player)), time)
        } else if let Some(e) = self.wandering_traders.iter_mut().find(|e| e.id == id) {
            let time = self.game_time;
            e.hurt_from(amount, kind, Some(crate::monster_ai::Target::Player(player)), time)
        } else if let Some(e) = self.snow_golems.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.illagers.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.vexes.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.ravagers.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.withers.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt_typed(amount, kind, false, false);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.cows.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount, source);
            if hit.applied {
                e.note_attacker(crate::monster_ai::Target::Player(player));
            }
            hit
        } else if let Some(e) = self.sheep.iter_mut().find(|e| e.id == id) {
            e.hurt(amount, source)
        } else if let Some(e) = self.pigs.iter_mut().find(|e| e.id == id) {
            e.hurt(amount, source)
        } else if let Some(e) = self.chickens.iter_mut().find(|e| e.id == id) {
            e.hurt_with_source(amount, source)
        } else if let Some(e) = self.ghasts.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else if let Some(e) = self.striders.iter_mut().find(|e| e.id == id) {
            e.hurt(amount, source)
        } else if let Some(e) = self.hoglins.iter_mut().find(|e| e.id == id) {
            e.hurt(amount)
        } else if let Some(e) = self.piglins.iter_mut().find(|e| e.id == id) {
            e.hurt_armored(amount)
        } else if let Some(e) = self.shulkers.iter_mut().find(|e| e.id == id) {
            let hit = e.hurt_armored(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        } else {
            let e = self.blazes.iter_mut().find(|e| e.id == id)?;
            let hit = e.hurt(amount);
            if hit.applied {
                e.ai.state.hurt_by = Some((crate::monster_ai::Target::Player(player), e.tick_count));
            }
            hit
        };
        // `resolvePlayerResponsibleForDamage`: the player is remembered.
        self.credit(id, hit, "minecraft:player", true);
        if hit.applied {
            self.villager_hurt_by(id, PLAYER_TARGET + player);
            self.note_hoglin_hurt(id, crate::monster_ai::Target::Player(player));
        }
        Some(hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monster_ai::Target;

    fn trident(player: u64) -> ProjectileHit {
        ProjectileHit { amount: 8.0, kind: "minecraft:trident", player, velocity: DVec3::new(0.0, 0.0, 2.0) }
    }

    fn snowball(player: u64) -> ProjectileHit {
        ProjectileHit { amount: 0.0, kind: "minecraft:thrown", player, velocity: DVec3::new(0.0, 0.0, 1.5) }
    }

    fn at(x: f64) -> DVec3 {
        DVec3::new(x, 64.0, 0.5)
    }

    fn raiders_and_nether_mobs(world: &mut EntityWorld) -> Vec<u64> {
        use crate::{hoglin::HoglinKind, illager::IllagerKind, piglin::PiglinKind};
        vec![
            world.spawn_illager(crate::illager::Illager::new(IllagerKind::Pillager, at(0.5)), true),
            world.spawn_illager(crate::illager::Illager::new(IllagerKind::Vindicator, at(3.5)), true),
            world.spawn_illager(crate::illager::Illager::new(IllagerKind::Evoker, at(6.5)), true),
            world.spawn_vex(crate::vex::Vex::new(at(9.5)), true),
            world.spawn_ravager(crate::ravager::Ravager::new(at(12.5)), true),
            world.spawn_ghast(crate::ghast::Ghast::new(at(20.5)), true),
            world.spawn_blaze(crate::blaze::Blaze::new(at(30.5)), true),
            world.spawn_strider(crate::strider::Strider::new(at(33.5)), true),
            world.spawn_hoglin(crate::hoglin::Hoglin::new(HoglinKind::Hoglin, at(36.5)), true),
            world.spawn_hoglin(crate::hoglin::Hoglin::new(HoglinKind::Zoglin, at(40.5)), true),
            world.spawn_piglin(crate::piglin::Piglin::new(PiglinKind::Piglin, at(44.5)), true),
            world.spawn_piglin(crate::piglin::Piglin::new(PiglinKind::Brute, at(47.5)), true),
        ]
    }

    fn health(world: &EntityWorld, id: u64) -> f32 {
        world.mob_body(id).map_or(0.0, |(_, health)| health)
    }

    #[test]
    fn a_thrown_hit_lands_on_pillagers_vexes_ravagers_and_the_nether_mobs() {
        let mut world = EntityWorld::default();
        for id in raiders_and_nether_mobs(&mut world) {
            let before = health(&world, id);
            let hit = world.player_projectile_hurt(id, snowball(3));
            assert!(hit.is_some_and(|hit| hit.applied && hit.full), "{:?}", world.entity_type(id));
            assert_eq!(health(&world, id), before, "{:?}", world.entity_type(id));
        }
        let mut world = EntityWorld::default();
        for id in raiders_and_nether_mobs(&mut world) {
            let before = health(&world, id);
            assert!(world.player_projectile_hurt(id, trident(3)).is_some_and(|hit| hit.applied), "{:?}", world.entity_type(id));
            assert!(health(&world, id) < before, "{:?}", world.entity_type(id));
        }
        assert_eq!(world.player_projectile_hurt(9999, trident(3)), None);
    }

    #[test]
    fn a_projectile_knocks_a_hoglin_and_a_ravager_back_less() {
        let mut world = EntityWorld::default();
        let pillager = world.spawn_illager(crate::illager::Illager::new(crate::illager::IllagerKind::Pillager, at(0.5)), true);
        let hoglin = world.spawn_hoglin(crate::hoglin::Hoglin::new(crate::hoglin::HoglinKind::Hoglin, at(5.5)), true);
        let ravager = world.spawn_ravager(crate::ravager::Ravager::new(at(10.5)), true);
        let mut pushed = Vec::new();
        for id in [pillager, hoglin, ravager] {
            world.player_projectile_hurt(id, snowball(1));
            pushed.push(world.mob_body(id).unwrap().0.velocity.z);
        }
        let power = f64::from(0.4_f32);
        for (z, share) in pushed.into_iter().zip([1.0, 1.0 - crate::hoglin::KNOCKBACK_RESISTANCE, 1.0 - crate::ravager::KNOCKBACK_RESISTANCE]) {
            assert!((z - power * share).abs() < 1.0e-9, "{z} {share}");
        }
    }

    #[test]
    fn a_firework_burst_knocks_mobs_away_from_it() {
        let mut world = EntityWorld::default();
        let zombie = world.spawn_zombie(crate::zombie::Zombie::new(at(0.5)), true);
        let golem = world.spawn_iron_golem(crate::iron_golem::IronGolem::new(at(10.5)), true);
        let ravager = world.spawn_ravager(crate::ravager::Ravager::new(at(20.5)), true);
        let power = f64::from(0.4_f32);
        for (id, share) in [(zombie, 1.0), (golem, 0.0), (ravager, 1.0 - crate::ravager::KNOCKBACK_RESISTANCE)] {
            world.body_mut(id).unwrap().on_ground = true;
            let before = health(&world, id);
            let feet = world.mob_body(id).unwrap().0.position;
            let hit = world.firework_blast(FireworkOwner::Nobody, id, 5.0, feet + DVec3::new(1.0, 0.5, 0.0));
            assert!(hit.is_some_and(|hit| hit.applied && hit.full), "{:?}", world.entity_type(id));
            assert!(health(&world, id) < before, "{:?}", world.entity_type(id));
            let v = world.mob_body(id).unwrap().0.velocity;
            assert!((v.x + power * share).abs() < 1.0e-6 && (v.y - power * share).abs() < 1.0e-6 && v.z == 0.0, "{:?} {v}", world.entity_type(id));
        }
        assert_eq!(world.firework_blast(FireworkOwner::Nobody, 9999, 5.0, at(0.5)), None);
    }

    #[test]
    fn a_second_firework_burst_inside_the_cooldown_does_not_push() {
        let mut world = EntityWorld::default();
        let zombie = world.spawn_zombie(crate::zombie::Zombie::new(at(0.5)), true);
        assert!(world.firework_blast(FireworkOwner::Player(6), zombie, 3.0, at(1.5)).is_some_and(|hit| hit.full));
        world.body_mut(zombie).unwrap().velocity = DVec3::ZERO;
        let again = world.firework_blast(FireworkOwner::Player(6), zombie, 5.0, at(1.5)).unwrap();
        assert!(again.applied && !again.full, "{again:?}");
        assert_eq!(world.mob_body(zombie).unwrap().0.velocity, DVec3::ZERO);
        let angry = world.zombie_mut(zombie).unwrap();
        let blamed = angry.target_player_id == Some(6) || angry.ai.as_ref().is_some_and(|ai| ai.state.hurt_by.map(|(by, _)| by) == Some(Target::Player(6)));
        assert!(blamed, "a player's firework angers the mob at the player");
    }

    #[test]
    fn a_firework_burst_makes_a_cow_panic_and_credits_a_player_with_the_kill() {
        let mut world = EntityWorld::default();
        let cow = world.spawn_cow(crate::cow::Cow::new(at(5.5)), true);
        world.body_mut(cow).unwrap().on_ground = true;
        assert!(world.firework_blast(FireworkOwner::Mob(77), cow, 3.0, at(6.5)).is_some_and(|hit| hit.applied));
        let entity = world.cows.iter().find(|e| e.id == cow).unwrap();
        assert!(DamageSourceKind::panics(entity.last_damage_source));
        assert!(entity.cow.body.velocity.x < -0.3, "pushed away from the burst: {}", entity.cow.body.velocity);
        let mut weak = crate::zombie::Zombie::new(at(10.5));
        weak.health = 0.5;
        let weak = world.spawn_zombie(weak, true);
        assert!(world.firework_blast(FireworkOwner::Player(6), weak, 7.0, at(10.5)).is_some_and(|hit| hit.died));
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        assert!(deaths[0].context.killed_by_player);
        let nobody = world.spawn_zombie(crate::zombie::Zombie::new(at(15.5)), true);
        world.firework_blast(FireworkOwner::Nobody, nobody, 1.0, at(15.5));
        let calm = world.zombie_mut(nobody).unwrap();
        assert_eq!((calm.target_player_id, calm.ai.as_ref().and_then(|ai| ai.state.hurt_by)), (None, None));
    }

    #[test]
    fn a_trident_angers_a_hoglin_and_a_piglin_at_the_thrower() {
        let mut world = EntityWorld::default();
        let hoglin = world.spawn_hoglin(crate::hoglin::Hoglin::new(crate::hoglin::HoglinKind::Hoglin, at(0.5)), true);
        let piglin = world.spawn_piglin(crate::piglin::Piglin::new(crate::piglin::PiglinKind::Piglin, at(5.5)), true);
        world.player_projectile_hurt(hoglin, trident(5));
        world.player_projectile_hurt(piglin, trident(5));
        assert_eq!(world.hoglin_mut(hoglin).unwrap().hurt_by, vec![Target::Player(5)]);
        assert_eq!(world.piglin_mut(piglin).unwrap().hurt_by, vec![Target::Player(5)]);
    }

    #[test]
    fn a_closed_shulker_turns_a_trident_away() {
        let mut world = EntityWorld::default();
        let id = world.spawn_shulker(crate::shulker::Shulker::new(at(0.5)), true);
        let max = health(&world, id);
        assert_eq!(world.player_projectile_hurt(id, trident(2)).map(|hit| hit.applied), Some(false));
        assert_eq!(health(&world, id), max);
        world.shulker_mut(id).unwrap().shulker.raw_peek = 30;
        assert!(world.player_projectile_hurt(id, trident(2)).is_some_and(|hit| hit.applied));
        assert!(health(&world, id) < max);
        assert_eq!(world.shulker_mut(id).unwrap().ai.state.hurt_by.map(|(t, _)| t), Some(Target::Player(2)));
        assert_eq!(world.mob_body(id).unwrap().0.velocity, DVec3::ZERO);
    }

    #[test]
    fn a_wind_charge_leaves_a_zombie_calm_but_angers_a_pillager() {
        let mut world = EntityWorld::default();
        let wind = ProjectileHit { amount: 1.0, kind: "minecraft:wind_charge", player: 4, velocity: DVec3::new(0.0, 0.0, 1.0) };
        let zombie = world.spawn_zombie_active(crate::zombie::Zombie::new(at(0.5)), 0.0);
        let spider = world.spawn_spider(crate::spider::Spider::new(at(3.5)), true);
        let slime = world.spawn_slime(crate::slime::Slime::new(at(6.5), 2), true);
        let pillager = world.spawn_illager(crate::illager::Illager::new(crate::illager::IllagerKind::Pillager, at(9.5)), true);
        let drowned = world.spawn_drowned(crate::zombie::Zombie::new(at(12.5)));
        let mut weak = crate::zombie::Zombie::new(at(15.5));
        weak.health = 0.5;
        let weak = world.spawn_zombie(weak, true);
        for id in [zombie, spider, slime, pillager, drowned, weak] {
            let before = health(&world, id);
            assert!(world.player_projectile_hurt(id, wind).is_some_and(|hit| hit.applied && hit.full), "{:?}", world.entity_type(id));
            assert!(health(&world, id) < before, "{:?}", world.entity_type(id));
        }
        let calm = world.zombie_mut(zombie).unwrap();
        assert_eq!((calm.ai.as_ref().map(|ai| ai.state.hurt_by), calm.target_player_id), (Some(None), None));
        assert_eq!(world.spider_mut(spider).unwrap().ai.state.hurt_by, None);
        assert_eq!(world.slime_mut(slime).unwrap().ai.state.hurt_by, None);
        assert_ne!(world.mob_body(zombie).unwrap().0.velocity, DVec3::ZERO, "it is still pushed");
        assert_eq!(world.illager_mut(pillager).unwrap().ai.state.hurt_by.map(|(by, _)| by), Some(Target::Player(4)));
        assert_eq!(world.zombie_mut(drowned).unwrap().target_player_id, Some(4), "a drowned is not on the calm list");
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        assert!(deaths[0].context.killed_by_player, "the player is still credited");
        let trident = world.player_projectile_hurt(spider, ProjectileHit { amount: 8.0, kind: "minecraft:trident", ..wind });
        assert!(trident.is_some_and(|hit| hit.applied));
        assert_eq!(world.spider_mut(spider).unwrap().ai.state.hurt_by.map(|(by, _)| by), Some(Target::Player(4)));
    }

    #[test]
    fn an_enderman_dodges_every_thrown_thing() {
        let mut world = EntityWorld::default();
        let id = world.spawn_enderman(crate::enderman::Enderman::new(at(0.5)), true);
        let max = health(&world, id);
        for kind in ["minecraft:thrown", "minecraft:trident", "minecraft:wind_charge"] {
            world.enderman_mut(id).unwrap().dodge_pending = false;
            let hit = world.player_projectile_hurt(id, ProjectileHit { kind, ..trident(1) });
            assert_eq!(hit.map(|hit| hit.applied), Some(false));
            assert!(world.enderman_mut(id).unwrap().dodge_pending, "{kind}");
        }
        assert_eq!(health(&world, id), max);
    }
}
