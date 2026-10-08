use crate::frame::FrameWorld;
use crate::identities::{DamageSource, LifeSequence, PelletId};
use crate::match_state::ClientLifecycle;
use crate::world::{ClientId, Tick};
use crate::world_objects::{GlassPaneBasis, GlassPieceId};
use gamemode_iw4::{
    G_CAN_DAMAGE_CONTENTS_MASK, can_damage_player_vis_scale, radius_damage_amount,
    radius_damage_distance_to_aabb,
};

const AREA_ENTITY_CAPACITY: usize = 0x800;

const _: () = assert!(crate::MATCH_TICK_MS == 50);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageAttempt {
    pub splash: bool,
    pub source: crate::DamageSource,
    pub pellet: PelletId,
    pub attacker: ClientId,
    pub attacker_life: LifeSequence,
    pub target: ClientId,
    pub target_life: LifeSequence,
    pub weapon: u32,
    pub amount: i32,

    pub killcam_entity_start_time: i32,

    pub inflictor_origin: Option<[f32; 3]>,

    pub hitloc: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageRefusal {
    FriendlyFire,
    NonPositive,
    MissingTarget,
    TargetNotAlive,
    StaleLife,
    Invulnerable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeathCommit {
    pub victim: ClientId,
    pub victim_life: LifeSequence,
    pub attacker: ClientId,
    pub attacker_life: LifeSequence,
    pub source: crate::DamageSource,
    pub pellet: PelletId,
    pub weapon: u32,
    pub amount: i32,
    pub killcam_entity_start_time: i32,
    pub inflictor_origin: Option<[f32; 3]>,
    pub hitloc: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageOutcome {
    Refused(DamageRefusal),
    Nonlethal { health_after: i32 },
    Died(DeathCommit),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ExplosionBlast {
    pub cone: Option<([f32; 3], f32)>,
    pub origin: [f32; 3],
    pub radius: f32,
    pub inner_damage: f32,
    pub outer_damage: f32,
    pub weapon: u32,
    pub source: DamageSource,
    pub attacker: ClientId,
    pub attacker_life: LifeSequence,
    pub killcam_entity_start_time: i32,
}

impl ExplosionBlast {
    fn contains(&self, point: [f32; 3]) -> bool {
        cone_contains(self.origin, self.cone, point)
    }
}

fn cone_contains(origin: [f32; 3], cone: Option<([f32; 3], f32)>, point: [f32; 3]) -> bool {
    cone.is_none_or(|(forward, cosine)| {
        let delta: [f32; 3] = std::array::from_fn(|i| point[i] - origin[i]);
        let len = delta.iter().map(|v| v * v).sum::<f32>().sqrt();
        len <= f32::EPSILON
            || delta.iter().zip(forward).map(|(a, b)| a * b).sum::<f32>() >= cosine * len
    })
}

struct GlassBlastHit {
    id: GlassPieceId,
    amount: u32,
    hit: [f32; 3],
    dir: [f32; 3],
}

pub(crate) fn apply_explosion_blast(world: &mut FrameWorld, tick: Tick, blast: &ExplosionBlast) {
    if !world.publishes_snapshot() {
        return;
    }
    if crate::voxel::active() {
        if let Some(power) = crater_power(
            blast.inner_damage,
            blast.radius,
            is_tactical(world.equipment_facts_for(blast.weapon)),
        ) {
            crate::voxel::push_explosion(blast.origin, power);
        }
        push_mob_blasts(
            world,
            &crate::t5_destructible::RadiusDamage {
                origin: blast.origin,
                radius: blast.radius,
                inner: blast.inner_damage,
                outer: blast.outer_damage,
                attacker: Some(blast.attacker),
                exclude: None,
                cone: blast.cone,
            },
            crate::script_player::weapon_name(world, blast.weapon) == CONCUSSION_WEAPON,
        );
    }
    let attempts = radius_player_attempts(world, blast);
    let glass = radius_glass_hits(world, blast);
    for attempt in &attempts {
        let _ = apply_damage_attempt(world, tick, attempt);
    }
    apply_glass_blast_hits(world, tick, glass);
    apply_entity_blast(world, blast);
    crate::t5_destructible::apply_radius(
        world,
        tick,
        &crate::t5_destructible::RadiusDamage {
            origin: blast.origin,
            radius: blast.radius,
            inner: blast.inner_damage,
            outer: blast.outer_damage,
            attacker: Some(blast.attacker),
            exclude: None,
            cone: blast.cone,
        },
    );
    let means = crate::script_player::means(world, blast.source, blast.weapon, 0, true);
    let ignore_model = world
        .ecs()
        .get_resource::<crate::script::Runtime>()
        .and_then(|runtime| match blast.source {
            DamageSource::Projectile(id) => runtime
                .missiles
                .get(&id)
                .and_then(|object| runtime.entities.get(object))
                .and_then(|entity| entity.presence),
            DamageSource::Radius(id) => Some(id),
            _ => None,
        });
    crate::script::host::triggers::damage_blast(
        world.ecs(),
        &crate::script::host::triggers::TriggerBlast {
            origin: blast.origin,
            radius: blast.radius,
            max: blast.inner_damage,
            min: blast.outer_damage,
            client: Some(blast.attacker),
            missile: match blast.source {
                DamageSource::Projectile(id) => Some(id),
                _ => None,
            },
            means,
            ignore_model,
            cone: blast.cone,
        },
    );
}

pub(crate) fn apply_block_world_damage(world: &mut FrameWorld, tick: Tick) {
    if !world.publishes_snapshot() {
        return;
    }
    for damage in crate::voxel::take_player_damage() {
        let target = ClientId(damage.client);
        let Some(meta) = world.client_meta(target) else {
            continue;
        };
        if meta.lifecycle != ClientLifecycle::Alive || damage.amount <= 0 {
            continue;
        }
        let life = meta.life_sequence;
        let victim = entity_iw4::client_state_name(&meta.name)
            .unwrap_or("")
            .to_owned();
        let origin = world.player(target).map_or([0.0; 3], |ps| ps.origin);
        let point = damage.from.unwrap_or(origin);
        let dir: [f32; 3] = std::array::from_fn(|i| origin[i] - point[i]);
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let hit = crate::script_player::Hit {
            victim: target,
            attacker: None,
            amount: if damage.death.is_some() {
                damage.amount.max(MINECRAFT_DEATH_DAMAGE)
            } else {
                damage.amount
            },
            flags: 0,
            means: crate::script_player::MINECRAFT_MEANS,
            weapon: 0,
            point,
            dir: if len > 0.0 {
                dir.map(|c| c / len)
            } else {
                [0.0; 3]
            },
            hitloc: 0,
            inflictor: None,
            commit: None,
        };
        let credited = crate::script::minecraft_damage(world.ecs(), tick, &hit);
        let died = world
            .client_meta(target)
            .is_some_and(|m| m.lifecycle == ClientLifecycle::Dead && m.life_sequence == life);
        if let Some(death) = damage.death.filter(|_| died && credited) {
            world.push_print(crate::PendingPrint {
                recipient: None,
                bold: false,
                template: format!("{}{}", crate::HUD_STRING_PLAIN, death.line(&victim)),
                arg: String::new(),
            });
            crate::voxel::push_minecraft_death(damage.client, death.killer);
        }
    }
}

const MINECRAFT_DEATH_DAMAGE: i32 = 100_000;

const CRATER_MIN_DAMAGE: f32 = 5.0;

const MOB_EYE_HEIGHT_SCALE: f32 = 0.85;

const CRATER_MAX_POWER: f32 = 8.0;

const TACTICAL_OFFHAND_CLASSES: [i32; 2] = [2, 3];

const FLASHBANG_EXPLOSION: i32 = 2;

const CRATER_MEANS: [&str; 4] = [
    "MOD_EXPLOSIVE",
    "MOD_GRENADE_SPLASH",
    "MOD_PROJECTILE_SPLASH",
    "MOD_ARTILLERY",
];

fn is_tactical(facts: Option<crate::equipment::EquipmentRuntimeFacts>) -> bool {
    facts.is_some_and(|f| {
        TACTICAL_OFFHAND_CLASSES.contains(&f.offhand_class)
            || f.projectile_explosion_type == FLASHBANG_EXPLOSION
    })
}

fn crater_power(inner_damage: f32, radius: f32, tactical: bool) -> Option<f32> {
    (!tactical && inner_damage >= CRATER_MIN_DAMAGE && radius > 0.0)
        .then(|| (radius / crate::voxel::BLOCK / 2.0).min(CRATER_MAX_POWER))
}

fn script_crater_power(means: &str, max: f32, radius: f32, tactical: bool) -> Option<f32> {
    if CRATER_MEANS.contains(&means) {
        crater_power(max, radius, tactical)
    } else {
        None
    }
}

const CONCUSSION_WEAPON: &str = "concussion_grenade_mp";

const CONCUSSION_RADIUS: f32 = 512.0;

const TICKS_PER_SECOND: f32 = 20.0;

fn concussion_slow_ticks(origin: [f32; 3], mins: [f32; 3], maxs: [f32; 3]) -> i32 {
    let feet = [(mins[0] + maxs[0]) * 0.5, (mins[1] + maxs[1]) * 0.5, mins[2]];
    let dist = feet.iter().zip(origin).map(|(f, o)| (f - o) * (f - o)).sum::<f32>().sqrt();
    let scale = (1.0 - dist / CONCUSSION_RADIUS).max(0.0);
    ((2.0 + 4.0 * scale) * TICKS_PER_SECOND).round() as i32
}

fn push_mob_blasts(world: &FrameWorld, blast: &crate::t5_destructible::RadiusDamage, stun: bool) {
    let exclude = blast.exclude.map(crate::AuthorityModelOwner::ScriptModel);
    let mobs = crate::voxel::mob_targets();
    let hits = mob_blast_hits(blast, &mobs, |feet, view_height, right| {
        can_damage_player_vis_scale(feet, view_height, right, blast.origin, |start, end| {
            t_trace_passed(&world.trace_world_except(start, end, G_CAN_DAMAGE_CONTENTS_MASK, exclude))
        })
    });
    for (key, damage, impact, slow_ticks) in mob_blast_slows(blast.origin, stun, hits, &mobs) {
        crate::voxel::push_mob_blast(key, damage, blast.origin, impact, blast.attacker.map(|c| c.0), slow_ticks);
    }
}

fn mob_blast_slows(
    origin: [f32; 3],
    stun: bool,
    hits: Vec<(u64, f32, f32)>,
    mobs: &[(u64, [f32; 3], [f32; 3])],
) -> Vec<(u64, f32, f32, i32)> {
    hits.into_iter()
        .map(|(key, damage, impact)| {
            let slow_ticks = if stun {
                mobs.iter().find(|mob| mob.0 == key).map_or(0, |&(_, mins, maxs)| concussion_slow_ticks(origin, mins, maxs))
            } else {
                0
            };
            (key, damage, impact, slow_ticks)
        })
        .collect()
}

fn mob_blast_hits(
    blast: &crate::t5_destructible::RadiusDamage,
    mobs: &[(u64, [f32; 3], [f32; 3])],
    mut vis_scale: impl FnMut([f32; 3], f32, [f32; 3]) -> f32,
) -> Vec<(u64, f32, f32)> {
    let mut hits = Vec::new();
    if blast.radius <= 0.0 {
        return hits;
    }
    for &(key, mins, maxs) in mobs {
        let mid: [f32; 3] = std::array::from_fn(|i| (mins[i] + maxs[i]) * 0.5);
        let half: [f32; 3] = std::array::from_fn(|i| (maxs[i] - mins[i]) * 0.5);
        if !cone_contains(blast.origin, blast.cone, mid) {
            continue;
        }
        let dist = radius_damage_distance_to_aabb(blast.origin, mid, half);
        if dist >= blast.radius {
            continue;
        }
        let along = [mid[0] - blast.origin[0], mid[1] - blast.origin[1]];
        let len = along[0].hypot(along[1]);
        let right = if len > f32::EPSILON {
            [along[1] / len, -along[0] / len, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let view_height = (maxs[2] - mins[2]) * MOB_EYE_HEIGHT_SCALE;
        let vis = vis_scale([mid[0], mid[1], mins[2]], view_height, right);
        let amount = radius_damage_amount(blast.inner, blast.outer, blast.radius, dist, vis);
        if amount <= 0 {
            continue;
        }
        hits.push((key, amount as f32, (1.0 - dist / blast.radius) * vis));
    }
    hits
}

fn apply_entity_blast(world: &mut FrameWorld, blast: &ExplosionBlast) {
    if blast.radius <= 0.0 {
        return;
    }
    let means = crate::script_player::means(world, blast.source, blast.weapon, 0, true);
    for (target, mid, dist) in
        crate::script::radius_targets(world.ecs(), blast.origin, blast.radius)
    {
        if !blast.contains(mid) {
            continue;
        }
        let amount = radius_damage_amount(
            blast.inner_damage,
            blast.outer_damage,
            blast.radius,
            dist,
            1.0,
        );
        if amount <= 0 {
            continue;
        }
        crate::script::damage_entity(
            world.ecs(),
            &crate::script::EntityHit {
                target,
                amount,
                attacker: Some(blast.attacker),
                means,
                weapon: blast.weapon,
                point: mid,
                dir: std::array::from_fn(|i| mid[i] - blast.origin[i]),
                bone: None,
                flags: IDFLAGS_RADIUS,
            },
        );
    }
}

const IDFLAGS_RADIUS: i32 = 1;

pub(crate) fn apply_script_blast(
    world: &mut FrameWorld,
    tick: Tick,
    blast: &crate::script::ScriptBlast,
) {
    if blast.radius <= 0.0 {
        return;
    }
    if world.publishes_snapshot() {
        crate::script::host::triggers::damage_blast(
            world.ecs(),
            &crate::script::host::triggers::TriggerBlast {
                origin: blast.origin,
                radius: blast.radius,
                max: blast.max,
                min: blast.min,
                client: blast.attacker,
                missile: None,
                means: blast.means,
                ignore_model: blast.inflictor,
                cone: None,
            },
        );
    }
    if world.publishes_snapshot() && crate::voxel::active() {
        if let Some(power) = script_crater_power(
            blast.means,
            blast.max,
            blast.radius,
            is_tactical(world.equipment_facts_for(blast.weapon)),
        ) {
            crate::voxel::push_explosion(blast.origin, power);
        }
        push_mob_blasts(
            world,
            &crate::t5_destructible::RadiusDamage {
                origin: blast.origin,
                radius: blast.radius,
                inner: blast.max,
                outer: blast.min,
                attacker: blast.attacker,
                exclude: blast.inflictor,
                cone: None,
            },
            crate::script_player::weapon_name(world, blast.weapon) == CONCUSSION_WEAPON,
        );
    }
    for target in radius_player_candidates(world, blast.origin, blast.radius) {
        let Some(meta) = world.client_meta(target) else {
            continue;
        };
        if meta.lifecycle != ClientLifecycle::Alive || minecraft_invulnerable(world, target) {
            continue;
        }
        let victim_life = meta.life_sequence;
        let Some(bounds) = world.player_area_bounds(target) else {
            continue;
        };
        let dist = radius_damage_distance_to_aabb(blast.origin, bounds.mid(), bounds.half());
        let vis_scale = player_radius_vis_scale(
            world,
            blast.origin,
            target,
            blast.inflictor.map(crate::AuthorityModelOwner::ScriptModel),
        );
        let amount = radius_damage_amount(blast.max, blast.min, blast.radius, dist, vis_scale);
        if amount <= 0 {
            continue;
        }
        let victim_origin = world.player(target).map_or(bounds.mid(), |ps| ps.origin);
        let dir: [f32; 3] = std::array::from_fn(|i| victim_origin[i] - blast.origin[i]);
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let commit = blast.attacker.and_then(|attacker| {
            Some(DeathCommit {
                victim: target,
                victim_life,
                attacker,
                attacker_life: world.client_meta(attacker)?.life_sequence,
                source: DamageSource::Radius(
                    blast
                        .inflictor
                        .unwrap_or(crate::ScriptModelId::from_wire(u32::MAX)),
                ),
                pellet: PelletId(0),
                weapon: blast.weapon,
                amount,
                killcam_entity_start_time: 0,
                inflictor_origin: Some(blast.origin),
                hitloc: 0,
            })
        });
        let hit = crate::script_player::Hit {
            victim: target,
            attacker: blast.attacker,
            amount,
            flags: IDFLAGS_RADIUS,
            means: blast.means,
            weapon: blast.weapon,
            point: blast.origin,
            dir: if len > 0.0 {
                dir.map(|c| c / len)
            } else {
                [0.0; 3]
            },
            hitloc: 0,
            inflictor: None,
            commit,
        };
        crate::script::player_damage(world.ecs(), tick, &hit);
    }
    crate::t5_destructible::apply_radius(
        world,
        tick,
        &crate::t5_destructible::RadiusDamage {
            origin: blast.origin,
            radius: blast.radius,
            inner: blast.max,
            outer: blast.min,
            attacker: blast.attacker,
            exclude: blast.inflictor,
            cone: None,
        },
    );
    apply_shared_glass_blast(
        world,
        tick,
        blast.origin,
        blast.max as i32,
        blast.min as i32,
        blast.radius,
    );
}

pub(crate) fn apply_script_hit(world: &mut FrameWorld, tick: Tick, hit: &crate::script::ScriptHit) {
    let crate::script::HitTarget::Player(target) = hit.target else {
        return;
    };
    if hit.amount <= 0 {
        return;
    }
    let Some(meta) = world.client_meta(target) else {
        return;
    };
    if meta.lifecycle != ClientLifecycle::Alive || minecraft_invulnerable(world, target) {
        return;
    }
    let victim_life = meta.life_sequence;
    let Some(victim_origin) = world.player(target).map(|ps| ps.origin) else {
        return;
    };
    let dir: [f32; 3] = std::array::from_fn(|i| victim_origin[i] - hit.origin[i]);
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    let commit = hit.attacker.and_then(|attacker| {
        Some(DeathCommit {
            victim: target,
            victim_life,
            attacker,
            attacker_life: world.client_meta(attacker)?.life_sequence,
            source: DamageSource::Radius(
                hit.inflictor
                    .unwrap_or(crate::ScriptModelId::from_wire(u32::MAX)),
            ),
            pellet: PelletId(0),
            weapon: hit.weapon,
            amount: hit.amount,
            killcam_entity_start_time: 0,
            inflictor_origin: Some(hit.origin),
            hitloc: hit.hitloc,
        })
    });
    let player_hit = crate::script_player::Hit {
        victim: target,
        attacker: hit.attacker,
        amount: hit.amount,
        flags: hit.flags,
        means: hit.means,
        weapon: hit.weapon,
        point: hit.origin,
        dir: if len > 0.0 {
            dir.map(|c| c / len)
        } else {
            [0.0; 3]
        },
        hitloc: hit.hitloc,
        inflictor: None,
        commit,
    };
    crate::script::player_damage(world.ecs(), tick, &player_hit);
}

fn radius_player_attempts(world: &FrameWorld, blast: &ExplosionBlast) -> Vec<DamageAttempt> {
    let mut intents = Vec::new();
    if blast.radius <= 0.0 {
        return intents;
    }
    for target in radius_player_candidates(world, blast.origin, blast.radius) {
        let Some(meta) = world.client_meta(target) else {
            continue;
        };
        let Some(bounds) = world.player_area_bounds(target) else {
            continue;
        };
        if !blast.contains(bounds.mid()) {
            continue;
        }
        let vis_scale = player_radius_vis_scale(world, blast.origin, target, None);
        intents.extend(radius_player_attempt(
            blast,
            target,
            meta.life_sequence,
            (bounds.mid(), bounds.half()),
            vis_scale,
        ));
    }
    intents
}

fn radius_player_attempt(
    blast: &ExplosionBlast,
    target: ClientId,
    target_life: LifeSequence,
    (mid, half): ([f32; 3], [f32; 3]),
    vis_scale: f32,
) -> Option<DamageAttempt> {
    let dist = radius_damage_distance_to_aabb(blast.origin, mid, half);
    let amount = radius_damage_amount(
        blast.inner_damage,
        blast.outer_damage,
        blast.radius,
        dist,
        vis_scale,
    );
    (amount > 0).then_some(DamageAttempt {
        splash: true,
        source: blast.source,
        pellet: PelletId(0),
        attacker: blast.attacker,
        attacker_life: blast.attacker_life,
        target,
        target_life,
        weapon: blast.weapon,
        amount,
        killcam_entity_start_time: blast.killcam_entity_start_time,
        inflictor_origin: Some(blast.origin),
        hitloc: 0,
    })
}

fn radius_glass_hits(world: &FrameWorld, blast: &ExplosionBlast) -> Vec<GlassBlastHit> {
    let mut hits = Vec::new();
    if blast.radius <= 0.0 {
        return hits;
    }
    for (id, pane) in world.world_objects().glass_radius_targets() {
        let (mid, half) = glass_pane_aabb(pane);
        if !blast.contains(mid) {
            continue;
        }
        let dist = radius_damage_distance_to_aabb(blast.origin, mid, half);
        let amount = radius_damage_amount(
            blast.inner_damage,
            blast.outer_damage,
            blast.radius,
            dist,
            1.0,
        );
        if amount <= 0 {
            continue;
        }
        let dir = [
            mid[0] - blast.origin[0],
            mid[1] - blast.origin[1],
            mid[2] - blast.origin[2],
        ];
        hits.push(GlassBlastHit {
            id,
            amount: amount as u32,
            hit: mid,
            dir,
        });
    }
    hits
}

fn apply_glass_blast_hits(world: &mut FrameWorld, tick: Tick, hits: Vec<GlassBlastHit>) {
    if hits.is_empty() {
        return;
    }
    let at_time_ms = i32::try_from(tick.0.saturating_mul(crate::MATCH_TICK_MS)).unwrap_or(i32::MAX);
    let mut holdrand = *world.stuck_holdrand_mut();
    for hit in hits {
        world.world_objects_mut().apply_glass_hit(
            hit.id,
            hit.amount,
            at_time_ms,
            hit.hit,
            hit.dir,
            &mut || crate::item::random_unit(&mut holdrand),
        );
    }
    *world.stuck_holdrand_mut() = holdrand;
}

fn glass_pane_aabb(pane: GlassPaneBasis) -> ([f32; 3], [f32; 3]) {
    let mut min = pane.origin;
    let mut max = pane.origin;
    for corner in [
        [
            pane.origin[0] + pane.axis_s[0],
            pane.origin[1] + pane.axis_s[1],
            pane.origin[2] + pane.axis_s[2],
        ],
        [
            pane.origin[0] + pane.axis_t[0],
            pane.origin[1] + pane.axis_t[1],
            pane.origin[2] + pane.axis_t[2],
        ],
        [
            pane.origin[0] + pane.axis_s[0] + pane.axis_t[0],
            pane.origin[1] + pane.axis_s[1] + pane.axis_t[1],
            pane.origin[2] + pane.axis_s[2] + pane.axis_t[2],
        ],
    ] {
        for i in 0..3 {
            min[i] = min[i].min(corner[i]);
            max[i] = max[i].max(corner[i]);
        }
    }
    let mid = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let half = [
        (max[0] - min[0]) * 0.5,
        (max[1] - min[1]) * 0.5,
        (max[2] - min[2]) * 0.5,
    ];
    (mid, half)
}

pub(crate) fn radius_player_candidates(
    world: &FrameWorld,
    origin: [f32; 3],
    radius: f32,
) -> Vec<ClientId> {
    let area_half = gamemode_iw4::radius_damage_area_half_extent(radius);
    let Ok(query) = clipmap_iw4::AreaBounds::from_mid_half(origin, [area_half; 3]) else {
        diag::warn!(
            Sim,
            "radius damage at {origin:?} r={radius} has no valid area"
        );
        return Vec::new();
    };
    world
        .area_entity_candidates(query, u32::MAX, AREA_ENTITY_CAPACITY)
        .into_iter()
        .map(|entity_num| ClientId(u32::from(entity_num)))
        .collect()
}

pub(crate) fn apply_shared_glass_blast(
    world: &mut FrameWorld,
    tick: Tick,
    origin: [f32; 3],
    inner_damage: i32,
    outer_damage: i32,
    radius: f32,
) {
    let time = crate::level_time_ms(tick);
    let mut holdrand = *world.stuck_holdrand_mut();
    world.world_objects_mut().apply_glass_blast(
        origin,
        inner_damage,
        outer_damage,
        radius,
        time,
        &mut || crate::item::random_unit(&mut holdrand),
    );
    *world.stuck_holdrand_mut() = holdrand;
}

fn player_radius_vis_scale(
    world: &FrameWorld,
    inflictor: [f32; 3],
    target: ClientId,
    exclude: Option<crate::AuthorityModelOwner>,
) -> f32 {
    let Some(ps) = world.player(target) else {
        return 1.0;
    };
    let (_, right, _) = math_iw4::angle_vectors(ps.viewangles);
    can_damage_player_vis_scale(
        ps.origin,
        ps.view_height_current,
        right,
        inflictor,
        |start, end| {
            let trace = world.trace_world_except(start, end, G_CAN_DAMAGE_CONTENTS_MASK, exclude);
            t_trace_passed(&trace)
        },
    )
}

fn t_trace_passed(trace: &trace_iw4::Trace) -> bool {
    trace.fraction >= 1.0 && trace.startsolid == 0
}

pub(crate) fn apply_flashbang_blast(
    world: &mut FrameWorld,
    origin: [f32; 3],
    radius_max: f32,
    radius_min: f32,
    attacker: ClientId,
) {
    let min_r = radius_min.max(1.0);
    let max_r = if radius_max < min_r {
        min_r
    } else {
        radius_max
    };
    let mut hits = Vec::new();
    for target in radius_player_candidates(world, origin, max_r) {
        let Some(meta) = world.client_meta(target) else {
            continue;
        };
        if meta.lifecycle != ClientLifecycle::Alive {
            continue;
        }
        let Some(ps) = world.player(target) else {
            continue;
        };
        let dist = {
            let dx = ps.origin[0] - origin[0];
            let dy = ps.origin[1] - origin[1];
            let dz = ps.origin[2] - origin[2];
            (dx * dx + dy * dy + dz * dz).sqrt()
        };
        if dist > max_r {
            continue;
        }
        if player_radius_vis_scale(world, origin, target, None) <= 0.0 {
            continue;
        }
        let amount_distance = flashbang_amount_distance(dist, min_r, max_r);
        let view_origin = [
            ps.origin[0],
            ps.origin[1],
            ps.origin[2] + ps.view_height_current,
        ];
        let (forward, _, _) = math_iw4::angle_vectors(ps.viewangles);
        let to_blast = [
            origin[0] - view_origin[0],
            origin[1] - view_origin[1],
            origin[2] - view_origin[2],
        ];
        let len =
            (to_blast[0] * to_blast[0] + to_blast[1] * to_blast[1] + to_blast[2] * to_blast[2])
                .sqrt();
        let amount_angle = if len > 0.0 {
            flashbang_amount_angle(
                (to_blast[0] * forward[0] + to_blast[1] * forward[1] + to_blast[2] * forward[2])
                    / len,
            )
        } else {
            flashbang_amount_angle(1.0)
        };
        hits.push((target, amount_distance, amount_angle));
    }
    for (target, amount_distance, amount_angle) in hits {
        crate::script::flashbang(
            world.ecs(),
            target,
            origin,
            amount_distance,
            amount_angle,
            attacker,
        );
    }
}

fn flashbang_amount_distance(dist: f32, min_r: f32, max_r: f32) -> f32 {
    if dist <= min_r || max_r <= min_r {
        1.0
    } else {
        1.0 - (dist - min_r) / (max_r - min_r)
    }
}

fn flashbang_amount_angle(dot: f32) -> f32 {
    (dot + 1.0) * 0.5
}

pub(crate) fn apply_damage_attempt(
    world: &mut FrameWorld,
    tick: Tick,
    intent: &DamageAttempt,
) -> DamageOutcome {
    if intent.amount <= 0 {
        return DamageOutcome::Refused(DamageRefusal::NonPositive);
    }
    let Some(meta) = world.client_meta(intent.target) else {
        return DamageOutcome::Refused(DamageRefusal::MissingTarget);
    };
    if meta.lifecycle != ClientLifecycle::Alive {
        return DamageOutcome::Refused(DamageRefusal::TargetNotAlive);
    }
    if meta.life_sequence != intent.target_life {
        return DamageOutcome::Refused(DamageRefusal::StaleLife);
    }
    if minecraft_invulnerable(world, intent.target) {
        return DamageOutcome::Refused(DamageRefusal::Invulnerable);
    }
    let mut amount = intent.amount;
    if intent.hitloc != crate::shield::HITLOC
        && !matches!(intent.source, crate::DamageSource::Melee)
    {
        let scale = world
            .combat_facts_for(intent.weapon)
            .map(|facts| facts.location_scale(intent.hitloc))
            .unwrap_or(1.0);
        amount = (amount as f32 * scale) as i32;
    }
    if amount <= 0 {
        return DamageOutcome::Refused(DamageRefusal::NonPositive);
    }
    crate::script_player::damage(world, tick, intent, amount)
}

fn minecraft_invulnerable(world: &FrameWorld, target: ClientId) -> bool {
    world
        .player(target)
        .is_some_and(|ps| ps.pm_flags & playerstate_iw4::pm_flags::MC_INVULNERABLE != 0)
}

pub(crate) fn play_death(
    world: &mut FrameWorld,
    victim: ClientId,
    attacker: Option<ClientId>,
    commit: Option<&DeathCommit>,
) -> bool {
    let Some(self_ps) = world.player(victim).copied() else {
        return false;
    };
    let attacker_origin = attacker
        .filter(|&id| id != victim)
        .and_then(|id| world.player(id).map(|p| p.origin));
    let yaw =
        playerstate_iw4::look_at_killer_yaw(attacker_origin, self_ps.origin, self_ps.viewangles[1]);

    let dead_viewangles = [0.0, self_ps.viewangles[1], 0.0];

    let inflictor_origin = commit.and_then(|death| death.inflictor_origin);
    let hitscan_origin = match commit.map(|death| death.source) {
        Some(crate::DamageSource::Shot(_)) | Some(crate::DamageSource::Melee) => attacker_origin,
        _ => None,
    };
    let mt =
        crate::player_anim_script::event_anim_movetype(&self_ps, world.last_anim_movetype(victim));
    let (view, primary) = crate::pmove_anim_weapon_ids(&self_ps);
    let mut conds = crate::anim_conditions_from_pmove(
        &self_ps,
        world.combat_facts_for(view),
        world.combat_facts_for(primary),
        Some(mt),
        world.last_anim_strafing(victim),
        world.anim_command_buttons(victim),
    );
    if let Some(commit) = commit {
        let blast_origin = inflictor_origin.or(hitscan_origin);
        let dist_sq = blast_origin.map(|origin| {
            let dx = origin[0] - self_ps.origin[0];
            let dy = origin[1] - self_ps.origin[1];
            let dz = origin[2] - self_ps.origin[2];
            dx * dx + dy * dy + dz * dz
        });
        let weap_class = world.combat_facts_for(commit.weapon).map(|f| f.weap_class);
        let offhand_class = world
            .equipment_facts_for(commit.weapon)
            .map(|f| f.offhand_class);
        conds.set_value(
            anim_iw4::ANIM_COND_DAMAGETYPE,
            anim_script_damagetype(commit.source, weap_class, offhand_class, dist_sq),
        );
        conds.set_value(
            anim_iw4::ANIM_COND_HITLOCATION,
            u32::from(anim_iw4::anim_script_hit_location(commit.hitloc)),
        );

        let hitdir = match blast_origin {
            Some(origin) => {
                let v_dir = [self_ps.origin[0] - origin[0], self_ps.origin[1] - origin[1]];
                let (forward, _, _) = math_iw4::angle_vectors(self_ps.viewangles);
                anim_iw4::anim_script_hit_direction([forward[0], forward[1]], v_dir)
            }
            None => 0,
        };
        conds.set_value(anim_iw4::ANIM_COND_HITDIRECTION, u32::from(hitdir));
    }

    let script = world.player_anim_script();
    let mut seed = world.anim_event_seed();
    if let Some(script) = script.as_ref() {
        if let Some(ps) = world.player_mut(victim) {
            ps.viewangles = dead_viewangles;
            ps.pm_type = playerstate_iw4::PM_TYPE_DEAD;
            let _ = script.apply_event(ps, anim_iw4::ANIM_ET_DEATH, &conds, &mut seed, true);
        }
    } else if let Some(ps) = world.player_mut(victim) {
        ps.viewangles = dead_viewangles;
        ps.pm_type = playerstate_iw4::PM_TYPE_DEAD;
    }
    world.set_anim_event_seed(seed);
    world.client_meta_mut(victim).look_at_killer_yaw = yaw;
    true
}

fn anim_script_damagetype(
    source: crate::DamageSource,
    weap_class: Option<i32>,
    offhand_class: Option<i32>,
    dist_sq: Option<f32>,
) -> u32 {
    match source {
        crate::DamageSource::Projectile(_) | crate::DamageSource::Radius(_) => {
            if offhand_class == Some(1) {
                if dist_sq.is_some_and(|d| d < anim_iw4::ANIM_DAMAGE_EXPLOSION_NEAR_DIST_SQ) {
                    2
                } else {
                    1
                }
            } else {
                2
            }
        }
        crate::DamageSource::Shot(_) => {
            if weap_class == Some(4) {
                1
            } else {
                0
            }
        }
        crate::DamageSource::Melee => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAG: crate::t5_destructible::RadiusDamage = crate::t5_destructible::RadiusDamage {
        origin: [0.0; 3],
        radius: 256.0,
        inner: 130.0,
        outer: 55.0,
        attacker: Some(ClientId(3)),
        exclude: None,
        cone: None,
    };

    fn mob_at(key: u64, x: f32) -> (u64, [f32; 3], [f32; 3]) {
        (key, [x - 10.0, -10.0, 0.0], [x + 10.0, 10.0, 70.0])
    }

    #[test]
    fn a_mob_at_the_centre_takes_the_inner_damage_and_the_full_push() {
        let hits = mob_blast_hits(&FRAG, &[mob_at(1, 0.0)], |_, _, _| 1.0);
        assert_eq!(hits, vec![(1, 130.0, 1.0)]);
    }

    #[test]
    fn damage_and_push_fall_off_to_the_edge_and_stop_past_it() {
        let mut traced = 0;
        let hits = mob_blast_hits(&FRAG, &[mob_at(1, 138.0), mob_at(2, 266.0)], |feet, view_height, right| {
            traced += 1;
            assert_eq!(feet, [138.0, 0.0, 0.0]);
            assert!((view_height - 59.5).abs() < 1e-4);
            assert_eq!(right, [0.0, -1.0, 0.0]);
            1.0
        });
        assert_eq!(traced, 1, "a mob out of reach costs no traces");
        let falloff = 55.0 + (1.0 - 128.0 / 256.0) * (130.0 - 55.0);
        assert_eq!(hits, vec![(1, f32::round(falloff), 0.5)]);
    }

    #[test]
    fn blocks_between_shield_a_mob_like_a_soldier() {
        assert!(mob_blast_hits(&FRAG, &[mob_at(1, 0.0)], |_, _, _| 0.0).is_empty());
        let hits = mob_blast_hits(&FRAG, &[mob_at(1, 0.0)], |_, _, _| 1.0 / 3.0);
        assert_eq!(hits, vec![(1, f32::round(130.0 / 3.0), 1.0 / 3.0)]);
    }

    #[test]
    fn a_claymore_only_hurts_mobs_in_front_of_it() {
        let claymore = crate::t5_destructible::RadiusDamage {
            radius: 192.0,
            cone: Some(([1.0, 0.0, 0.0], 70f32.to_radians().cos())),
            ..FRAG
        };
        let hits = mob_blast_hits(&claymore, &[mob_at(1, 100.0), mob_at(2, -100.0)], |_, _, _| 1.0);
        assert_eq!(hits.iter().map(|h| h.0).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn only_damaging_explosives_blow_craters_sized_by_their_radius() {
        assert_eq!(crater_power(1.0, 512.0, false), None, "concussion");
        assert_eq!(crater_power(0.0, 256.0, false), None);
        assert_eq!(crater_power(130.0, 0.0, false), None);
        let frag = crater_power(130.0, 256.0, false).unwrap();
        assert!((frag - 256.0 / 72.0).abs() < 1e-6);
    }

    #[test]
    fn tactical_grenades_never_blow_craters() {
        assert_eq!(crater_power(130.0, 256.0, true), None);
        let facts = |offhand_class, projectile_explosion_type| {
            Some(crate::equipment::EquipmentRuntimeFacts {
                offhand_class,
                projectile_explosion_type,
                ..Default::default()
            })
        };
        assert!(is_tactical(facts(2, 0)), "smoke");
        assert!(is_tactical(facts(3, 0)), "flash and concussion");
        assert!(is_tactical(facts(0, 2)), "flashbang explosion");
        for lethal in [0, 1, 4, 5] {
            assert!(!is_tactical(facts(lethal, 0)), "offhand class {lethal}");
        }
        assert!(!is_tactical(None), "launchers and killstreaks");
    }

    #[test]
    fn huge_blasts_are_capped_at_a_wither_sized_crater() {
        assert_eq!(crater_power(300.0, 2000.0, false), Some(8.0));
    }

    #[test]
    fn killstreak_script_blasts_crater_only_for_explosive_means() {
        for means in ["MOD_PROJECTILE_SPLASH", "MOD_EXPLOSIVE", "MOD_ARTILLERY", "MOD_GRENADE_SPLASH"] {
            let power = script_crater_power(means, 300.0, 512.0, false).unwrap();
            assert!((power - 512.0 / 72.0).abs() < 1e-6, "{means}");
        }
        for means in ["MOD_MELEE", "MOD_CRUSH", "MOD_GAS", "MOD_BURNED", "MOD_FALLING"] {
            assert_eq!(script_crater_power(means, 300.0, 512.0, false), None, "{means}");
        }
        assert_eq!(script_crater_power("MOD_EXPLOSIVE", 1.0, 512.0, false), None);
        assert_eq!(script_crater_power("MOD_EXPLOSIVE", 300.0, 512.0, true), None);
    }

    #[test]
    fn a_stun_slows_a_mob_two_to_six_seconds_by_how_far_its_feet_are() {
        let (_, mins, maxs) = mob_at(1, 0.0);
        assert_eq!(concussion_slow_ticks([0.0; 3], mins, maxs), 120);
        assert_eq!(concussion_slow_ticks([256.0, 0.0, 0.0], mins, maxs), 80);
        assert_eq!(concussion_slow_ticks([0.0, 0.0, 256.0], mins, maxs), 80);
        assert_eq!(concussion_slow_ticks([512.0, 0.0, 0.0], mins, maxs), 40);
        assert_eq!(concussion_slow_ticks([900.0, 0.0, 0.0], mins, maxs), 40);
    }

    #[test]
    fn only_a_mob_the_stun_hurts_is_slowed() {
        let stun = crate::t5_destructible::RadiusDamage {
            radius: CONCUSSION_RADIUS,
            inner: 1.0,
            outer: 1.0,
            ..FRAG
        };
        assert!(mob_blast_hits(&stun, &[mob_at(1, 100.0)], |_, _, _| 1.0 / 3.0).is_empty());
        assert!(mob_blast_hits(&stun, &[mob_at(1, 600.0)], |_, _, _| 1.0).is_empty());
        let mobs = [mob_at(1, 0.0), mob_at(2, 100.0), mob_at(3, 256.0)];
        let slows = |is_stun: bool| {
            let hits = mob_blast_hits(&stun, &mobs, |feet, _, _| if feet[0] == 100.0 { 0.0 } else { 1.0 });
            mob_blast_slows(stun.origin, is_stun, hits, &mobs).iter().map(|h| (h.0, h.1, h.3)).collect::<Vec<_>>()
        };
        assert_eq!(slows(true), vec![(1, 1.0, 120), (3, 1.0, 80)]);
        assert_eq!(slows(false), vec![(1, 1.0, 0), (3, 1.0, 0)]);
    }

    #[test]
    fn a_self_frag_still_hurts_its_thrower_with_minecraft_damage_queued_in_the_same_tick() {
        let thrower = ClientId(3);
        crate::voxel::push_player_damage(thrower.0, 35, None, None);
        let frag = ExplosionBlast {
            cone: None,
            origin: [0.0; 3],
            radius: FRAG.radius,
            inner_damage: FRAG.inner,
            outer_damage: FRAG.outer,
            weapon: 7,
            source: DamageSource::Projectile(crate::identities::ProjectileId(1)),
            attacker: thrower,
            attacker_life: LifeSequence(2),
            killcam_entity_start_time: 0,
        };
        let attempt = radius_player_attempt(&frag, thrower, LifeSequence(2), ([0.0, 0.0, 36.0], [16.0, 16.0, 36.0]), 1.0)
            .expect("the thrower is in its own blast");
        assert_eq!((attempt.attacker, attempt.target, attempt.amount), (thrower, thrower, 130));
        assert!(attempt.splash);
        let queued = crate::voxel::take_player_damage();
        assert!(queued.iter().any(|d| d.client == thrower.0 && d.amount == 35));
    }

    fn knifed(flags: u32) -> (DamageOutcome, i32) {
        let mut sim = crate::SimWorld::default();
        let target = ClientId(2);
        let mut world = sim.frame();
        let ps = world.ensure_player(target);
        ps.pm_flags = flags;
        ps.health = 100;
        let meta = world.client_meta_mut(target);
        meta.lifecycle = ClientLifecycle::Alive;
        meta.life_sequence = LifeSequence(4);
        let attempt = DamageAttempt {
            splash: false,
            source: DamageSource::Melee,
            pellet: PelletId(0),
            attacker: ClientId(3),
            attacker_life: LifeSequence(1),
            target,
            target_life: LifeSequence(4),
            weapon: 0,
            amount: 40,
            killcam_entity_start_time: 0,
            inflictor_origin: None,
            hitloc: 0,
        };
        let outcome = apply_damage_attempt(&mut world, Tick(1), &attempt);
        (outcome, world.player(target).map_or(0, |ps| ps.health))
    }

    #[test]
    fn mw2_damage_is_refused_for_an_invulnerable_minecraft_player() {
        let flags = playerstate_iw4::pm_flags::MC_INVULNERABLE | playerstate_iw4::pm_flags::SPRINTING;
        assert_eq!(knifed(flags), (DamageOutcome::Refused(DamageRefusal::Invulnerable), 100));
        let (outcome, _) = knifed(playerstate_iw4::pm_flags::SPRINTING);
        assert!(!matches!(outcome, DamageOutcome::Refused(_)), "{outcome:?}");
    }
}
