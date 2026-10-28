use glam::DVec3;
use minecraft_terrain::melee::WeaponBonus;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};

use crate::death::DamageCause;
use crate::impulse::MeleePush;

pub const KNOCKBACK_SOUND: &str = "minecraft:entity.player.attack.knockback";
pub const NO_DAMAGE_SOUND: &str = "minecraft:entity.player.attack.nodamage";
pub const SWEEP_SOUND: &str = "minecraft:entity.player.attack.sweep";
pub const CRIT_SOUND: &str = "minecraft:entity.player.attack.crit";
pub const STRONG_SOUND: &str = "minecraft:entity.player.attack.strong";
pub const WEAK_SOUND: &str = "minecraft:entity.player.attack.weak";
const HALF_WIDTH: f64 = 0.3;
const FULL_STRENGTH: f32 = 0.9;
const CRITICAL: f32 = 1.5;
const SPRINT_KNOCKBACK: f32 = 0.5;
const SWEEP_KNOCKBACK: f32 = 0.4;
const SWEEP_SPREAD: [f64; 3] = [1.0, 0.25, 1.0];
const SWEEP_REACH_SQUARED: f64 = 9.0;
const HIGHEST_THROW: f64 = 0.4;
const CUSTOM_NAME: &str = "minecraft:custom_name";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerBox {
    pub id: u64,
    pub bounds: [f64; 6],
    pub invulnerable: bool,
}

impl PlayerBox {
    pub fn standing(id: u64, feet: [f64; 3], height: f64, invulnerable: bool) -> Self {
        Self {
            id,
            bounds: [
                feet[0] - HALF_WIDTH,
                feet[1],
                feet[2] - HALF_WIDTH,
                feet[0] + HALF_WIDTH,
                feet[1] + height,
                feet[2] + HALF_WIDTH,
            ],
            invulnerable,
        }
    }

    pub fn feet(&self) -> DVec3 {
        let b = self.bounds;
        DVec3::new((b[0] + b[3]) / 2.0, b[1], (b[2] + b[5]) / 2.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attack {
    pub attacker: u64,
    pub feet: DVec3,
    pub yaw: f32,
    pub attack_damage: f64,
    pub strength: f32,
    pub sprinting: bool,
    pub can_critical: bool,
    pub can_sweep: bool,
    pub bonus: WeaponBonus,
    pub disable_shield_ticks: u32,
    pub pvp: bool,
}

impl Attack {
    fn full_strength(&self) -> bool {
        self.strength > FULL_STRENGTH
    }

    fn knockback(&self) -> bool {
        self.sprinting && self.full_strength()
    }

    fn critical(&self) -> bool {
        self.full_strength() && self.can_critical
    }

    pub fn sweeps(&self) -> bool {
        self.full_strength() && !self.critical() && !self.knockback() && self.can_sweep
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blow {
    pub victim: u64,
    pub damage: f32,
    pub from: [f64; 3],
    pub toward: [f64; 2],
    pub push: MeleePush,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PvpHit {
    pub blow: Option<Blow>,
    pub hurt: bool,
    pub critical: bool,
    pub knockback: bool,
    pub sweep: bool,
    pub sounds: Vec<&'static str>,
}

fn throw(yaw: f32, strength: f32) -> DVec3 {
    if strength <= 0.0 {
        return DVec3::ZERO;
    }
    let (sin, cos) = minecraftoss_player::minecraft_sin_cos(f64::from(yaw));
    let power = f64::from(strength);
    DVec3::new(-sin * power, power.min(HIGHEST_THROW), cos * power)
}

fn blow(attack: &Attack, victim: &PlayerBox, damage: f32, knockback: f32) -> Blow {
    let feet = victim.feet();
    Blow {
        victim: victim.id,
        damage,
        from: attack.feet.to_array(),
        toward: [attack.feet.x - feet.x, attack.feet.z - feet.z],
        push: MeleePush {
            throw: throw(attack.yaw, knockback),
            lift: 0.0,
            disable_shield_ticks: attack.disable_shield_ticks,
        },
    }
}

fn harmed(attack: &Attack, victim: &PlayerBox) -> bool {
    attack.pvp && !victim.invulnerable && victim.id != attack.attacker
}

pub fn melee(attack: &Attack, victim: &PlayerBox) -> PvpHit {
    let scale = attack.strength;
    let mut base = attack.attack_damage as f32 * (0.2 + scale * scale * 0.8);
    let magic = attack.bonus.damage * scale;
    if !(base > 0.0 || magic > 0.0) {
        return PvpHit::default();
    }
    let critical = attack.critical();
    if critical {
        base *= CRITICAL;
    }
    let mut hit = PvpHit {
        critical,
        knockback: attack.knockback(),
        sweep: attack.sweeps(),
        ..PvpHit::default()
    };
    if hit.knockback {
        hit.sounds.push(KNOCKBACK_SOUND);
    }
    if !harmed(attack, victim) {
        hit.sounds.push(NO_DAMAGE_SOUND);
        return PvpHit {
            critical: false,
            sweep: false,
            ..hit
        };
    }
    let knockback = attack.bonus.knockback + if hit.knockback { SPRINT_KNOCKBACK } else { 0.0 };
    hit.blow = Some(blow(attack, victim, base + magic, knockback));
    hit.hurt = true;
    if hit.sweep {
        hit.sounds.push(SWEEP_SOUND);
    }
    if critical {
        hit.sounds.push(CRIT_SOUND);
    } else if !hit.sweep {
        hit.sounds.push(if attack.full_strength() {
            STRONG_SOUND
        } else {
            WEAK_SOUND
        });
    }
    hit
}

pub fn swept(
    attack: &Attack,
    around: [f64; 6],
    target: Option<u64>,
    players: &[PlayerBox],
) -> Vec<Blow> {
    if !attack.sweeps() {
        return Vec::new();
    }
    let low = [0, 1, 2].map(|k| around[k] - SWEEP_SPREAD[k]);
    let high = [0, 1, 2].map(|k| around[k + 3] + SWEEP_SPREAD[k]);
    let damage = (1.0 + attack.bonus.damage) * attack.strength;
    players
        .iter()
        .filter(|victim| Some(victim.id) != target && harmed(attack, victim))
        .filter(|victim| {
            (0..3).all(|k| victim.bounds[k] < high[k] && victim.bounds[k + 3] > low[k])
        })
        .filter(|victim| attack.feet.distance_squared(victim.feet()) < SWEEP_REACH_SQUARED)
        .map(|victim| blow(attack, victim, damage, SWEEP_KNOCKBACK))
        .collect()
}

pub fn player_on_ray(
    players: &[PlayerBox],
    attacker: u64,
    eye: DVec3,
    look: DVec3,
    reach: f64,
) -> Option<(u64, f64)> {
    players
        .iter()
        .filter(|player| player.id != attacker)
        .filter_map(|player| {
            let b = player.bounds;
            let found = minecraft_terrain::end_crystal::ray_box(
                eye,
                look,
                reach,
                (DVec3::new(b[0], b[1], b[2]), DVec3::new(b[3], b[4], b[5])),
            )?;
            Some((player.id, found))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

pub fn shield_disable_ticks(recipes: &RecipeBook, held: Option<&ItemStack>) -> u32 {
    held.and_then(|stack| recipes.weapon(stack))
        .map_or(0, |(_, seconds)| crate::offhand::disable_ticks(seconds))
}

pub fn wear_weapon(inventory: &mut Inventory, selected: usize) -> Option<String> {
    let held = inventory.slots.get(selected)?.clone()?;
    let wear = inventory
        .recipes
        .weapon(&held)
        .map_or(0, |(per_attack, _)| per_attack);
    inventory.wear_tool(selected, wear).then_some(held.id)
}

pub fn hit(blow: &Blow, attacker: u64) -> crate::entities::Hit {
    let client = u32::try_from(attacker).unwrap_or(u32::MAX);
    (
        blow.damage,
        Some(blow.from),
        Some(blow.toward),
        DamageCause::Player { client },
        blow.push,
        None,
    )
}

pub fn custom_name(stack: &ItemStack) -> Option<String> {
    match stack.components.as_ref()?.get(CUSTOM_NAME)? {
        serde_json::Value::String(text) => Some(text.clone()),
        other => other
            .get("text")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: u64 = 0;
    const FRIEND: u64 = 1;
    const OTHER: u64 = 2;

    fn attack(strength: f32) -> Attack {
        Attack {
            attacker: HOST,
            feet: DVec3::new(0.0, 64.0, 0.0),
            yaw: 0.0,
            attack_damage: 6.0,
            strength,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
            bonus: WeaponBonus::default(),
            disable_shield_ticks: 0,
            pvp: true,
        }
    }

    fn standing(id: u64, x: f64, z: f64) -> PlayerBox {
        PlayerBox::standing(id, [x, 64.0, z], 1.8, false)
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-5
    }

    #[test]
    fn a_full_hit_lands_once_with_the_weapons_damage_and_the_strong_sound() {
        let hit = melee(&attack(1.0), &standing(FRIEND, 0.0, 2.0));
        let blow = hit.blow.unwrap();
        assert_eq!((blow.victim, blow.damage, hit.hurt), (FRIEND, 6.0, true));
        assert_eq!(blow.from, [0.0, 64.0, 0.0]);
        assert_eq!(blow.toward, [0.0, -2.0]);
        assert_eq!(blow.push, MeleePush::default());
        assert_eq!(hit.sounds, vec![STRONG_SOUND]);
        let (damage, from, toward, cause, ..) = super::hit(&blow, HOST);
        assert_eq!(
            (damage, from, toward, cause),
            (
                6.0,
                Some([0.0, 64.0, 0.0]),
                Some([0.0, -2.0]),
                DamageCause::Player { client: 0 }
            )
        );
    }

    #[test]
    fn half_strength_scales_the_damage_like_vanilla() {
        let hit = melee(&attack(0.5), &standing(FRIEND, 0.0, 2.0));
        assert!(close(hit.blow.unwrap().damage, 6.0 * 0.4), "{hit:?}");
        assert_eq!(hit.sounds, vec![WEAK_SOUND]);
        let one = Attack {
            attack_damage: 1.0,
            ..attack(0.5)
        };
        assert!(close(
            melee(&one, &standing(FRIEND, 0.0, 2.0))
                .blow
                .unwrap()
                .damage,
            0.4
        ));
    }

    #[test]
    fn with_pvp_off_or_on_a_creative_player_a_hit_only_makes_the_no_damage_sound() {
        let off = Attack {
            pvp: false,
            ..attack(1.0)
        };
        let hit = melee(&off, &standing(FRIEND, 0.0, 2.0));
        assert_eq!(
            (hit.blow, hit.hurt, hit.sounds.clone()),
            (None, false, vec![NO_DAMAGE_SOUND])
        );
        let creative = PlayerBox::standing(FRIEND, [0.0, 64.0, 2.0], 1.8, true);
        assert_eq!(melee(&attack(1.0), &creative).blow, None);
        let sprint = Attack {
            sprinting: true,
            pvp: false,
            ..attack(1.0)
        };
        assert_eq!(
            melee(&sprint, &standing(FRIEND, 0.0, 2.0)).sounds,
            vec![KNOCKBACK_SOUND, NO_DAMAGE_SOUND]
        );
    }

    #[test]
    fn a_falling_full_hit_is_critical_and_sharpness_is_not_multiplied() {
        let crit = Attack {
            can_critical: true,
            ..attack(1.0)
        };
        let hit = melee(&crit, &standing(FRIEND, 0.0, 2.0));
        assert!(hit.critical && !hit.sweep);
        assert_eq!(hit.blow.unwrap().damage, 9.0);
        assert_eq!(hit.sounds, vec![CRIT_SOUND]);
        let sharp = Attack {
            bonus: WeaponBonus {
                damage: 3.0,
                knockback: 0.0,
            },
            ..crit
        };
        assert_eq!(
            melee(&sharp, &standing(FRIEND, 0.0, 2.0))
                .blow
                .unwrap()
                .damage,
            12.0
        );
        let weak = Attack {
            can_critical: true,
            ..attack(0.5)
        };
        assert!(!melee(&weak, &standing(FRIEND, 0.0, 2.0)).critical);
    }

    #[test]
    fn sharpness_five_adds_three_at_full_strength_and_scales_with_it() {
        let bonus = minecraft_terrain::melee::WeaponBonus {
            damage: minecraft_terrain::melee::sharpness_damage(5),
            knockback: 0.0,
        };
        let sharp = Attack {
            bonus,
            ..attack(1.0)
        };
        assert_eq!(
            melee(&sharp, &standing(FRIEND, 0.0, 2.0))
                .blow
                .unwrap()
                .damage,
            9.0
        );
        let half = Attack {
            bonus,
            ..attack(0.5)
        };
        assert!(close(
            melee(&half, &standing(FRIEND, 0.0, 2.0))
                .blow
                .unwrap()
                .damage,
            2.4 + 1.5
        ));
    }

    #[test]
    fn a_sprinting_hit_and_knockback_two_throw_the_victim_along_the_attackers_facing() {
        let sprint = Attack {
            sprinting: true,
            ..attack(1.0)
        };
        let hit = melee(&sprint, &standing(FRIEND, 0.0, 2.0));
        assert!(hit.knockback && !hit.sweep);
        assert_eq!(hit.sounds, vec![KNOCKBACK_SOUND, STRONG_SOUND]);
        let throw = hit.blow.unwrap().push.throw;
        assert!(
            throw.x.abs() < 1.0e-6
                && (throw.z - 0.5).abs() < 1.0e-6
                && (throw.y - 0.4).abs() < 1.0e-9,
            "{throw}"
        );
        let enchanted = Attack {
            bonus: WeaponBonus {
                damage: 0.0,
                knockback: minecraft_terrain::melee::knockback_strength(2),
            },
            ..attack(1.0)
        };
        let throw = melee(&enchanted, &standing(FRIEND, 0.0, 2.0))
            .blow
            .unwrap()
            .push
            .throw;
        assert!(
            (throw.z - 1.0).abs() < 1.0e-6 && (throw.y - 0.4).abs() < 1.0e-9,
            "{throw}"
        );
        let both = Attack {
            sprinting: true,
            ..enchanted
        };
        let throw = melee(&both, &standing(FRIEND, 0.0, 2.0))
            .blow
            .unwrap()
            .push
            .throw;
        assert!((throw.z - 1.5).abs() < 1.0e-6, "{throw}");
        let west = Attack {
            yaw: 90.0,
            sprinting: true,
            ..attack(1.0)
        };
        let throw = melee(&west, &standing(FRIEND, -2.0, 0.0))
            .blow
            .unwrap()
            .push
            .throw;
        assert!(
            (throw.x + 0.5).abs() < 1.0e-6 && throw.z.abs() < 1.0e-6,
            "{throw}"
        );
    }

    #[test]
    fn an_axe_disables_a_shield_for_five_seconds() {
        let mut axe = ItemStack::new("minecraft:iron_axe", 1);
        axe.components = Some(
            serde_json::json!({"minecraft:weapon": {"item_damage_per_attack": 2, "disable_blocking_for_seconds": 5.0}}),
        );
        let recipes = RecipeBook::default();
        assert_eq!(shield_disable_ticks(&recipes, Some(&axe)), 100);
        assert_eq!(shield_disable_ticks(&recipes, None), 0);
        let chop = Attack {
            disable_shield_ticks: 100,
            ..attack(1.0)
        };
        assert_eq!(
            melee(&chop, &standing(FRIEND, 0.0, 2.0))
                .blow
                .unwrap()
                .push
                .disable_shield_ticks,
            100
        );
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let path = std::path::Path::new(&root).join(minecraft_terrain::crafting::ITEM_CATALOG);
        let Ok(catalog) = minecraftoss_player::item_catalog::ItemCatalog::from_path(&path) else {
            return;
        };
        let recipes = RecipeBook::default().with_item_catalog(std::sync::Arc::new(catalog));
        let plain = ItemStack::new("minecraft:iron_axe", 1);
        assert_eq!(shield_disable_ticks(&recipes, Some(&plain)), 100);
        assert_eq!(
            shield_disable_ticks(
                &recipes,
                Some(&ItemStack::new("minecraft:diamond_sword", 1))
            ),
            0
        );
    }

    #[test]
    fn a_sweep_reaches_a_second_player_in_range_and_skips_one_four_blocks_away() {
        let sweep = Attack {
            can_sweep: true,
            ..attack(1.0)
        };
        let target = standing(FRIEND, 0.0, 2.0);
        let near = standing(OTHER, 1.0, 2.5);
        let far = standing(3, 0.0, 6.0);
        let host = standing(HOST, 0.0, 1.2);
        let hit = melee(&sweep, &target);
        assert!(hit.sweep);
        assert_eq!(hit.sounds, vec![SWEEP_SOUND]);
        let blows = swept(
            &sweep,
            target.bounds,
            Some(FRIEND),
            &[host, target, near, far],
        );
        assert_eq!(blows.len(), 1, "{blows:?}");
        assert_eq!((blows[0].victim, blows[0].damage), (OTHER, 1.0));
        assert!((blows[0].push.throw.z - 0.4).abs() < 1.0e-6);
        let sharp = Attack {
            bonus: WeaponBonus {
                damage: 3.0,
                knockback: 0.0,
            },
            ..sweep
        };
        assert_eq!(
            swept(&sharp, target.bounds, Some(FRIEND), &[near])[0].damage,
            4.0
        );
        let cow = [-0.45, 64.0, 1.55, 0.45, 65.4, 2.45];
        let around_cow = swept(&sweep, cow, None, &[host, near, far]);
        assert_eq!(
            around_cow
                .iter()
                .map(|blow| blow.victim)
                .collect::<Vec<_>>(),
            vec![OTHER]
        );
        assert!(swept(&attack(1.0), target.bounds, Some(FRIEND), &[near]).is_empty());
        let off = Attack {
            pvp: false,
            ..sweep
        };
        assert!(swept(&off, target.bounds, Some(FRIEND), &[near]).is_empty());
        let beyond = standing(OTHER, 1.0, 3.2);
        let reach = Attack {
            feet: DVec3::new(0.0, 64.0, 0.2),
            ..sweep
        };
        assert!(swept(&reach, target.bounds, Some(FRIEND), &[beyond]).is_empty());
    }

    #[test]
    fn the_ray_finds_the_nearest_other_player_and_never_the_attacker() {
        let eye = DVec3::new(0.0, 65.62, 0.0);
        let look = DVec3::Z;
        let players = [
            standing(HOST, 0.0, 0.0),
            standing(FRIEND, 0.0, 2.0),
            standing(OTHER, 0.0, 1.0),
        ];
        let (id, distance) = player_on_ray(&players, HOST, eye, look, 3.0).unwrap();
        assert_eq!(id, OTHER);
        assert!((distance - 0.7).abs() < 1.0e-9);
        assert_eq!(player_on_ray(&players[..2], HOST, eye, look, 1.5), None);
        assert_eq!(player_on_ray(&players[..1], HOST, eye, look, 3.0), None);
        assert_eq!(
            player_on_ray(&players, FRIEND, DVec3::new(0.0, 65.62, 2.0), -look, 3.0)
                .unwrap()
                .0,
            OTHER
        );
    }

    #[test]
    fn a_hit_wears_the_weapon_by_its_damage_per_attack() {
        let mut inventory = Inventory::default();
        let mut sword = ItemStack::new("minecraft:iron_sword", 1);
        sword.components = Some(
            serde_json::json!({"minecraft:weapon": {"item_damage_per_attack": 1}, "minecraft:max_damage": 250, "minecraft:damage": 249}),
        );
        inventory.slots[0] = Some(sword);
        assert_eq!(
            wear_weapon(&mut inventory, 0),
            Some("minecraft:iron_sword".to_owned())
        );
        assert_eq!(inventory.slots[0], None);
        assert_eq!(wear_weapon(&mut inventory, 0), None);
    }

    #[test]
    fn a_weapon_is_named_only_by_its_custom_name() {
        let mut sword = ItemStack::new("minecraft:iron_sword", 1);
        assert_eq!(custom_name(&sword), None);
        sword.components = Some(serde_json::json!({"minecraft:custom_name": "Excalibur"}));
        assert_eq!(custom_name(&sword).as_deref(), Some("Excalibur"));
        sword.components = Some(serde_json::json!({"minecraft:custom_name": {"text": "Edge"}}));
        assert_eq!(custom_name(&sword).as_deref(), Some("Edge"));
        sword.components = Some(serde_json::json!({"minecraft:damage": 3}));
        assert_eq!(custom_name(&sword), None);
    }
}
