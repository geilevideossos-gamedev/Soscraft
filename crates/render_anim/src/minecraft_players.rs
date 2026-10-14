use std::collections::{BTreeMap, BTreeSet, HashMap};

use bevy::prelude::*;
use minecraft_commands::{Clientbound, CommandServer};
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_survival::actions::{Actor, Body, Effect, Moved};
use minecraft_survival::entities::{Entities, TickActor};
use minecraft_survival::level::{Level, Terrain};
use minecraft_survival::menus::MenuInput;
use minecraft_survival::player::Player;
use minecraft_survival::server::Globals;
use minecraft_survival::session::{HandQueue, HandTake};
use minecraft_survival::switch::Switch;
use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::BlockPos;
use minecraftoss_core::{BlockStateId, Registries};
use playerstate_iw4::PlayerState;

use crate::minecraft_sounds::Sounds;
use crate::minecraft_spawn_gate::SpawnGate;
use crate::minecraft_world::{
    CRAWLING_BELOW_VIEW_HEIGHT, TICK_SECONDS, ability_bits, minecraft_yaw, place_friend,
    place_player, sky_bits, write_blocks,
};

type Shapes = HashMap<BlockStateId, u16>;
type ShapeIds = HashMap<(Vec<[u32; 6]>, u16), u16>;

#[derive(Default)]
pub(crate) struct VoxelQueues {
    deaths: BTreeMap<u32, Vec<String>>,
    totems: BTreeSet<u32>,
    switches: BTreeSet<u32>,
}

impl VoxelQueues {
    pub(crate) fn drain(&mut self, keep: impl Fn(u32) -> bool) {
        self.absorb(
            sim::voxel::take_minecraft_deaths(),
            sim::voxel::take_totem_pops(),
            sim::voxel::take_team_switches(),
            keep,
        );
    }

    fn absorb(
        &mut self,
        deaths: Vec<(u32, String)>,
        totems: Vec<u32>,
        switches: Vec<u32>,
        keep: impl Fn(u32) -> bool,
    ) {
        for (client, killer) in deaths {
            self.deaths.entry(client).or_default().push(killer);
        }
        self.totems.extend(totems);
        self.switches.extend(switches);
        self.deaths.retain(|&client, _| keep(client));
        self.totems.retain(|&client| keep(client));
        self.switches.retain(|&client| keep(client));
    }

    pub(crate) fn take_deaths(&mut self, client: u32) -> Vec<String> {
        self.deaths.remove(&client).into_iter().flatten().collect()
    }

    pub(crate) fn take_totem(&mut self, client: u32) -> bool {
        self.totems.remove(&client)
    }

    pub(crate) fn take_switch(&mut self, client: u32) -> bool {
        self.switches.remove(&client)
    }
}

pub(crate) const PLAYER_WIDTH: f64 = 0.6;

#[derive(Clone, Copy, Default)]
pub(crate) struct Skating {
    pub(crate) active: bool,
    pub(crate) airborne: bool,
}

pub(crate) enum Begun {
    Hidden,
    Left(Switch),
    Arriving { at: [f64; 3], standing: bool },
    Live,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn begin(
    player: &mut Player,
    levels: &mut BTreeMap<Dimension, Level>,
    globals: &Globals,
    authority: &mut sim::SimWorld,
    client: sim::ClientId,
    origin: [f64; 3],
    ps: Option<&PlayerState>,
    remote: bool,
    mut gate: Option<&mut SpawnGate>,
    now_ms: u64,
    dt: f64,
    sounds: &mut Option<Sounds>,
    packs: &PackStack,
    lines: &mut Vec<Clientbound>,
) -> Begun {
    player.set_remote(remote);
    if let Some(switch) = player.switching.filter(|switch| !switch.left) {
        if let Some(switching) = player.switching.as_mut() {
            switching.left = true;
        }
        if let Some(level) = levels.get_mut(&switch.from) {
            let rider = (client, u64::from(player.id.0));
            crate::minecraft_riding::dismount(
                &mut player.riding,
                level.entities.as_mut(),
                authority,
                rider,
            );
        }
        let bits = ability_bits(&player.mode) | playerstate_iw4::pm_flags::MC_INVULNERABLE;
        authority.set_minecraft_abilities(client, bits);
        diag::info!(
            World,
            "Minecraft player leaves {} for {}",
            switch.from.dimension_type(),
            switch.to.dimension_type()
        );
        return Begun::Left(switch);
    }
    let Some(level) = levels.get_mut(&player.dimension) else {
        return Begun::Hidden;
    };
    if player.switching.is_some() {
        if player.switching.is_some_and(|switch| switch.respawn) {
            let consume = player.switching.is_some_and(|switch| !switch.keep_charge);
            if let Some(switch) = player.switching.as_mut() {
                switch.respawn = false;
            }
            if minecraft_survival::respawn_block::return_to_block(globals, level, player, consume) {
                lines.push(minecraft_survival::player::missing_respawn_message());
            }
            if player.switching.is_some_and(|switch| !switch.left) {
                return Begun::Hidden;
            }
        }
        if player.arrival.is_none()
            && player
                .place
                .is_none_or(|place| place.dimension != player.dimension)
        {
            player.arrival = Some(minecraft_survival::respawn::level_spawn(
                globals,
                level,
            ));
        }
        crate::minecraft_portal::arriving(player, sounds.as_mut(), packs);
        let ps = ps.filter(|ps| ps.pm_type == 0);
        if let Some(ps) = ps.filter(|_| player.arrival.is_some()) {
            place(
                player,
                authority,
                client,
                ps,
                origin,
                gate.as_deref_mut(),
                now_ms,
            );
        }
        let bits = ability_bits(&player.mode)
            | sky_bits(&level.terrain.environment)
            | playerstate_iw4::pm_flags::MC_INVULNERABLE;
        authority.set_minecraft_abilities(client, bits);
        crate::minecraft_end::arriving(player, level);
        let at = player
            .place
            .filter(|place| place.dimension == player.dimension)
            .map_or(origin, |place| place.feet);
        return Begun::Arriving {
            at,
            standing: ps.is_some(),
        };
    }
    if let Some(ps) = ps {
        if player.observe_life(ps.pm_type) {
            if minecraft_survival::respawn::respawn(globals, level, player) {
                lines.push(minecraft_survival::player::missing_respawn_message());
            }
            if player.switching.is_some() {
                return Begun::Hidden;
            }
        }
        if ps.pm_type == 0 {
            place(player, authority, client, ps, origin, gate, now_ms);
        }
    }
    crate::minecraft_end::step(player, levels);
    crate::minecraft_elytra::step(
        player,
        authority,
        client,
        origin,
        dt,
        sounds.as_mut(),
        packs,
    );
    Begun::Live
}

pub(crate) struct Life {
    pub(crate) alive: bool,
    pub(crate) dead: bool,
    pub(crate) totem_popped: bool,
    pub(crate) killers: Vec<String>,
}

pub(crate) fn life(
    player: &mut Player,
    globals: &Globals,
    terrain: &Terrain,
    authority: &mut sim::SimWorld,
    client: sim::ClientId,
    ps: &PlayerState,
    queues: &mut VoxelQueues,
) -> Life {
    let Player {
        survival,
        mode: player_mode,
        death_watch,
        arrived,
        ..
    } = player;
    let alive = ps.pm_type == 0;
    let dead = matches!(
        ps.pm_type,
        playerstate_iw4::PM_TYPE_SPECTATOR
            | playerstate_iw4::PM_TYPE_INTERMISSION
            | playerstate_iw4::PM_TYPE_DEAD
            | playerstate_iw4::PM_TYPE_DEAD_LINKED
    );
    let (in_world, died) = minecraft_survival::death::life_state(ps.pm_type);
    if queues.take_switch(client.0) {
        death_watch.switch_teams();
    }
    let keep_inventory = minecraft_survival::death::keeps_belongings(
        globals.game_rules.keep_inventory(),
        player_mode.mode,
    );
    survival.rules =
        minecraft_survival::survival::SurvivalRules::of(&globals.game_rules, globals.difficulty);
    survival.set_game_mode(player_mode);
    authority.set_minecraft_abilities(
        client,
        ability_bits(player_mode) | sky_bits(&terrain.environment),
    );
    if alive && player_mode.abilities.may_fly {
        player_mode.abilities.flying = authority
            .player(client)
            .is_some_and(|p| p.pm_flags & playerstate_iw4::pm_flags::MC_FLYING != 0);
    }
    death_watch.observe(
        in_world,
        died,
        survival.status.experience_level,
        keep_inventory,
    );
    if dead && *arrived {
        if keep_inventory || death_watch.switched_teams() {
            survival.respawn_keeping_experience();
        } else {
            survival.respawn();
        }
    } else if !dead
        && !sim::voxel::player_damage_pending(client.0)
        && let Some(health) = authority.player(client).map(|p| (p.health, p.max_health))
    {
        survival.observe(health.0, health.1);
    }
    let totem_popped = queues.take_totem(client.0);
    let killers = queues.take_deaths(client.0);
    Life {
        alive,
        dead,
        totem_popped,
        killers,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn movement(
    player: &mut Player,
    globals: &Globals,
    terrain: &mut Terrain,
    entities: &mut Option<Entities>,
    (shapes, shape_ids): (&mut Shapes, &mut ShapeIds),
    authority: &sim::SimWorld,
    client: sim::ClientId,
    origin: [f64; 3],
    ps: &PlayerState,
    (alive, skate): (bool, Skating),
    dt: f64,
    sounds: &mut Option<Sounds>,
    packs: &PackStack,
) -> Moved {
    let level = minecraft_survival::bands::level(player.dimension);
    let Player {
        uuid,
        survival,
        mode: player_mode,
        inventory,
        selected,
        hand,
        steps,
        riding,
        drop_throttle,
        arsenal,
        ..
    } = player;
    let feet = sim::voxel::to_block(origin, ps.origin);
    let on_ground = alive
        && crate::minecraft_world::skate_on_ground(
            skate.active,
            skate.airborne,
            ps.ground_entity_num,
        );
    let (hull_mins, hull_maxs) = sim::voxel::player_hull(ps);
    let water = sim::voxel::water(ps.origin, hull_mins, hull_maxs, ps.view_height_current);
    if water.wet() {
        steps.air_peak = None;
    }
    let counts =
        crate::minecraft_riding::counts_steps(alive, player_mode.noclip(), riding.is_some());
    if let Some(last) = steps.last.filter(|_| counts) {
        let climbing = sim::voxel::climb(ps.origin).on;
        let horizontal = ((feet[0] - last[0])
            .hypot(feet[2] - last[2])
            .hypot(if climbing { feet[1] - last[1] } else { 0.0 })
            * 0.6) as f32;
        let block_at = |dy: f64| {
            let pos = (
                feet[0].floor() as i32,
                (feet[1] - dy).floor() as i32,
                feet[2].floor() as i32,
            );
            minecraft_terrain::scene::Scene::block(&terrain.scene, pos)
                .cloned()
                .map(|b| (pos, b))
        };
        let under = block_at(0.2);
        if climbing {
            survival.set_climbable(Some(block_at(0.0).map_or(
                minecraft_survival::death::FallLocation::OtherClimbable,
                |(_, b)| minecraft_survival::death::FallLocation::of_climbable(&b.id.key()),
            )));
        }
        let centre = |pos: (i32, i32, i32)| {
            Vec3::from_array(sim::voxel::to_map(
                origin,
                [pos.0 as f64 + 0.5, pos.1 as f64 + 1.0, pos.2 as f64 + 0.5],
            ))
        };
        if (on_ground || climbing) && horizontal < 2.0 && !skate.active {
            steps.move_dist += horizontal;
            if steps.move_dist > steps.next_step
                && let Some((pos, block)) = under.as_ref()
            {
                steps.next_step = steps.move_dist as i32 as f32 + 1.0;
                let sounding = on_ground
                    || block.id.path == "powder_snow"
                    || terrain.stream.states.state_of(block).is_some_and(|state| {
                        globals
                            .registries
                            .block_tags
                            .id("minecraft:climbable")
                            .is_some_and(|tag| globals.registries.block_in_tag(state, tag))
                    });
                let inside = block_at(-0.01).filter(|(_, b)| {
                    let p = b.id.path.as_str();
                    p == "snow" || p.ends_with("_carpet") || p == "moss_carpet"
                });
                let (pos, block) = inside.as_ref().map_or((*pos, block), |(p, b)| (*p, b));
                if sounding
                    && let (Some(sounds), Some(kind)) =
                        (sounds.as_mut(), terrain.scene.sound_type(block))
                {
                    sounds.play(
                        packs,
                        &kind.step,
                        Some(centre(pos)),
                        kind.volume * 0.15,
                        kind.pitch,
                    );
                }
            }
        }
        let caught = [-0.01, -1.0].into_iter().any(|dy| {
            block_at(dy)
                .is_some_and(|(_, b)| minecraft_survival::survival::breaks_fall(&b.id.key()))
        });
        let powder_snow = |found: Option<&(BlockPos, minecraft_terrain::scene::Block)>| {
            found.is_some_and(|(_, b)| b.id.key() == "minecraft:powder_snow")
        };
        if (powder_snow(block_at(-0.01).as_ref()) || (on_ground && powder_snow(under.as_ref())))
            && let Some(peak) = steps.air_peak
            && let Some(sound) = minecraft_survival::survival::powder_snow_landing(peak - feet[1])
            && let Some(sounds) = sounds.as_mut()
        {
            sounds.play(packs, sound, None, 1.0, 1.0);
        }
        if on_ground {
            let on_wall = minecraft_survival::bands::at_bottom_wall(
                minecraftoss_player::World::min_y(&terrain.scene),
                feet[1],
            );
            if let Some(peak) = steps.air_peak.take().filter(|_| !caught && !on_wall) {
                let fall = survival.impulse_fall(peak - feet[1], feet[1]);
                let landed_on = under.as_ref().map(|(_, b)| b.id.key());
                let crouching = !skate.active
                    && ps.pm_flags
                        & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE)
                        != 0;
                let damage = survival.fall(fall, landed_on.as_deref(), crouching);
                if damage > 0
                    && let Some(sounds) = sounds.as_mut()
                {
                    sounds.play(
                        packs,
                        minecraft_survival::survival::fall_sound(damage),
                        None,
                        1.0,
                        1.0,
                    );
                    let voice = (sounds.random() - sounds.random()) * 0.2 + 1.0;
                    if survival.status.health > 0.0 {
                        sounds.play(packs, "minecraft:entity.player.hurt", None, 1.0, voice);
                    }
                    if let Some((pos, block)) = under.as_ref()
                        && let Some(kind) = terrain.scene.sound_type(block)
                    {
                        sounds.play(
                            packs,
                            &kind.fall,
                            Some(centre(*pos)),
                            kind.volume * 0.5,
                            kind.pitch * 0.75,
                        );
                    }
                }
            }
            survival.set_climbable(None);
        } else if caught
            || ps.pm_flags & playerstate_iw4::pm_flags::MC_FLYING != 0
            || minecraft_survival::elytra::floating(&survival.status)
            || (climbing && sim::voxel::lava(ps.origin, hull_mins, hull_maxs) <= 0.0)
        {
            steps.air_peak = Some(feet[1]);
        } else {
            steps.air_peak = Some(steps.air_peak.map_or(feet[1], |p| p.max(feet[1])));
        }
    } else {
        steps.air_peak = None;
    }
    steps.last = alive.then_some(feet);

    let dt_hand = dt;
    hand.swing_predicted = false;
    if let Some(ticks) = hand.swing.as_mut() {
        *ticks += (dt_hand * 20.0) as f32;
        if *ticks >= crate::minecraft_hand::SWING_TICKS {
            hand.swing = None;
        }
    }
    let holding = entities
        .as_ref()
        .is_some_and(|_| !arsenal.holds_gun(inventory, *selected));
    let holding_item = alive && holding && player_mode.interacts();
    let empty_hand = holding_item
        && entities
            .as_ref()
            .is_some_and(|_| inventory.slots[*selected].is_none());
    hand.clock += dt_hand;
    let hand_ticks = (hand.clock / TICK_SECONDS) as u32;
    hand.clock -= f64::from(hand_ticks) * TICK_SECONDS;
    hand.cooldowns.tick(hand_ticks);
    drop_throttle.tick(hand_ticks);
    let cooldowns = hand.cooldowns.shown((hand.clock / TICK_SECONDS) as f32);
    hand.miss_time = hand.miss_time.saturating_sub(hand_ticks);
    hand.attack_ticks = hand.attack_ticks.saturating_add(hand_ticks);
    let main_hand = entities
        .as_ref()
        .and_then(|_| inventory.slots[*selected].as_ref().map(|s| s.id.clone()));
    let mut totem_popped = false;
    if hand.attack_item != main_hand {
        hand.attack_item = main_hand;
        hand.attack_ticks = 0;
    }
    if alive {
        let motion = minecraft_survival::survival::Motion {
            feet,
            on_ground,
            sprinting: ps.pm_flags & playerstate_iw4::pm_flags::SPRINTING != 0,
            jumping: ps.pm_flags & playerstate_iw4::pm_flags::JUMPING != 0,
            in_water: water.wet(),
            eye_in_water: water.eye,
            in_lava: sim::voxel::lava(ps.origin, hull_mins, hull_maxs) > 0.0,
            eye_in_lava: sim::voxel::eye_in_lava(ps.origin, ps.view_height_current),
            in_rain: entities.as_ref().is_some_and(|e| e.weather.player_in_rain),
            crouching: ps.pm_flags
                & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE)
                != 0,
            height: hull_maxs[2] / sim::voxel::BLOCK,
            riding: skate.active,
        };
        let armor = entities
            .as_ref()
            .map_or_else(minecraftoss_player::survival::Armor::default, |_| {
                minecraft_survival::survival::armor(inventory)
            });
        survival.wear(minecraft_terrain::protection::Protection::worn(
            globals.data.protection.clone(),
            inventory,
        ));
        survival.set_freeze_immune(minecraft_survival::survival::freeze_immune(inventory));
        for _ in 0..hand_ticks {
            let from = survival.last_feet().unwrap_or(feet);
            let breath = survival.tick(&motion);
            if let Some(sounds) = sounds.as_mut() {
                if breath.drowned && survival.status.health > 0.0 {
                    sounds.play(packs, "minecraft:entity.player.hurt_drown", None, 1.0, 1.0);
                }
                if let Some(popped) = breath.popped {
                    let (volume, pitch) = minecraft_survival::survival::pop_sound(popped);
                    sounds.play(packs, "minecraft:ui.hud.bubble_pop", None, volume, pitch);
                }
            }
            if motion.in_lava {
                steps.air_peak = steps.air_peak.map(|peak| feet[1] + (peak - feet[1]) * 0.5);
            }
            let heard = survival.hazards(&terrain.scene, &motion, from, armor, || {
                entities.as_mut().map_or(0.5, |e| e.random_float())
            });
            if (feet[0] != from[0] || feet[2] != from[2])
                && let Some(entities) = entities.as_mut()
            {
                entities.view.snowflakes(&terrain.scene, feet, motion.height);
            }
            let bodies = entities
                .as_ref()
                .map_or_else(Vec::new, |e| obstacles(ps, authority, client, e));
            let frozen = survival.location_changed(
                motion.feet,
                motion.on_ground,
                &minecraft_terrain::doors::Outlined(&terrain.scene),
                &*terrain.stream.states,
                &|pos| {
                    !minecraft_terrain::placing::obstructed(
                        pos,
                        &[[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]],
                        &bodies,
                    )
                },
            );
            if let Some(entities) = entities.as_mut().filter(|_| !frozen.is_empty()) {
                write_blocks(
                    terrain,
                    &globals.registries,
                    (shapes, shape_ids, level),
                    entities,
                    frozen
                        .into_iter()
                        .map(|(pos, block)| (pos, scene_block(&block)))
                        .collect(),
                );
            }
            let melted = survival.take_melted();
            if let Some(entities) = entities.as_mut().filter(|_| !melted.is_empty()) {
                let gone: Vec<_> = melted
                    .into_iter()
                    .filter_map(|pos| {
                        minecraft_terrain::scene::Scene::block(&terrain.scene, pos)
                            .filter(|b| b.id.key() == "minecraft:powder_snow")
                            .cloned()
                            .map(|b| (pos, b))
                    })
                    .collect();
                let writes = gone.iter().map(|(pos, _)| (*pos, None)).collect();
                write_blocks(
                    terrain,
                    &globals.registries,
                    (shapes, shape_ids, level),
                    entities,
                    writes,
                );
                entities.destroyed.extend(gone);
            }
            if let Some(entities) = entities.as_mut()
                && hold_off_death(
                    survival,
                    inventory,
                    *selected,
                    sounds.as_mut(),
                    packs,
                    false,
                )
            {
                let height = f64::from(motion.height);
                pop_totem(entities, (uuid, client.0), feet, height);
                totem_popped = true;
            }
            if survival.stuck() {
                steps.air_peak = steps.air_peak.map(|_| feet[1]);
            }
            if let Some(sounds) = sounds.as_mut() {
                for sound in heard {
                    sounds.play(packs, sound.event, None, sound.volume, sound.pitch);
                }
            }
        }
    }
    Moved {
        on_ground,
        wet: water.wet(),
        eye_in_water: water.eye,
        hand_ticks,
        holding,
        holding_item,
        empty_hand,
        cooldowns,
        totem_popped,
    }
}

pub(crate) fn frost_scale(
    scene: &minecraft_terrain::scene::HandcraftedScene,
    feet: [f64; 3],
    ticks_frozen: i32,
    alive: bool,
) -> f32 {
    let under = (
        feet[0].floor() as i32,
        (feet[1] - 0.2).floor() as i32,
        feet[2].floor() as i32,
    );
    let on_block = minecraft_terrain::scene::Scene::block(scene, under).is_some_and(|b| {
        !(b.id.namespace == "minecraft"
            && matches!(b.id.path.as_str(), "air" | "cave_air" | "void_air"))
    });
    minecraft_survival::survival::frost_scale(ticks_frozen, alive && on_block)
}

pub(crate) fn actor(ps: &PlayerState, origin: [f64; 3], alive: bool) -> Actor {
    let crouch = playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE;
    let offhand =
        weapon_iw4::WeaponState::OffhandInit as i32..=weapon_iw4::WeaponState::OffhandEnd as i32;
    Actor {
        alive,
        feet: sim::voxel::to_block(origin, ps.origin),
        eye: sim::voxel::to_block(
            origin,
            [
                ps.origin[0],
                ps.origin[1],
                ps.origin[2] + ps.view_height_current,
            ],
        ),
        yaw: minecraft_yaw(ps.viewangles[1]),
        pitch: ps.viewangles[0],
        velocity: glam::DVec3::new(
            f64::from(ps.velocity[0]),
            f64::from(ps.velocity[2]),
            -f64::from(ps.velocity[1]),
        ) / (f64::from(sim::voxel::BLOCK) * 20.0),
        gravity: f64::from(ps.gravity) / f64::from(sim::voxel::BLOCK),
        crouching: ps.pm_flags & crouch != 0,
        swimming: ps.view_height_current < CRAWLING_BELOW_VIEW_HEIGHT,
        sprinting: ps.pm_flags & playerstate_iw4::pm_flags::SPRINTING != 0,
        gliding: ps.pm_flags & playerstate_iw4::pm_flags::MC_GLIDING != 0,
        mw2_offhand: offhand.contains(&ps.weaponstate_primary),
        skating: false,
    }
}

pub(crate) struct MwBody<'a> {
    pub(crate) authority: &'a mut sim::SimWorld,
    pub(crate) client: sim::ClientId,
    pub(crate) ps: &'a PlayerState,
    pub(crate) registries: &'a Registries,
    pub(crate) shapes: &'a mut Shapes,
    pub(crate) shape_ids: &'a mut ShapeIds,
    pub(crate) level: u8,
}

impl Body for MwBody<'_> {
    fn write_blocks(
        &mut self,
        terrain: &mut Terrain,
        entities: &mut Entities,
        writes: Vec<(BlockPos, Option<minecraft_terrain::scene::Block>)>,
    ) {
        write_blocks(
            terrain,
            self.registries,
            (self.shapes, self.shape_ids, self.level),
            entities,
            writes,
        );
    }

    fn obstacles(&self, entities: &Entities) -> Vec<[f64; 6]> {
        obstacles(self.ps, self.authority, self.client, entities)
    }

    fn push(&mut self, velocity: glam::DVec3) {
        let launch = velocity * f64::from(sim::voxel::BLOCK);
        self.authority.push_velocity(
            self.client,
            [launch.x as f32, -launch.z as f32, launch.y as f32],
        );
    }

    fn launch_rocket(&mut self, boost_ticks: i32) -> bool {
        self.authority.launch_rocket(self.client, boost_ticks)
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn struck(
    player: &mut Player,
    entities: &mut Entities,
    authority: &mut sim::SimWorld,
    client: sim::ClientId,
    origin: [f64; 3],
    ps: &PlayerState,
    alive: bool,
    teleports: &[(PlayerUuid, [f64; 3])],
    sounds: &mut Option<Sounds>,
    packs: &PackStack,
) {
    let player_id = u64::from(player.id.0);
    let dimension = player.dimension;
    let outputs = entities.take_player_outputs(player_id);
    let rider = (client, player_id);
    let Player {
        uuid,
        survival,
        mode: player_mode,
        inventory,
        hand,
        steps,
        riding,
        reattach,
        jump_bar,
        raid_bars,
        ..
    } = player;
    let local_uuid = &*uuid;
    let feet = sim::voxel::to_block(origin, ps.origin);
    let mc_yaw = minecraft_yaw(ps.viewangles[1]);
    let at = |b: [f64; 3]| Vec3::from_array(sim::voxel::to_map(origin, b));
    let armor = minecraft_survival::survival::armor(inventory);
    let knockback_resistance = inventory.armor_toughness().1;
    let mut landed = Vec::new();
    for (amount, from, toward, cause, push, effect) in outputs
        .hits
        .into_iter()
        .filter(|_| !player_mode.invulnerable())
    {
        let blocked = hand
            .blocking
            .filter(|blocking| blocking.raised())
            .map_or(0.0, |_| {
                minecraft_survival::offhand::blocked_damage(mc_yaw, toward, amount)
            });
        if let minecraft_survival::death::DamageCause::Mob {
            id,
            attack: minecraft_survival::death::MobAttack::Melee,
            ..
        } = cause
            && blocked > 0.0
        {
            entities.shield_blocked(id, player_id, feet);
        }
        if let Some(blocking) = hand.blocking.filter(|_| blocked > 0.0) {
            if inventory.wear_tool(
                blocking.slot,
                minecraft_survival::offhand::shield_wear(blocked),
            ) {
                hand.blocking = None;
                entities
                    .broken_items
                    .push(minecraft_survival::offhand::SHIELD.to_owned());
                let pitch = 0.8 + entities.random_float() * 0.4;
                if let Some(sounds) = sounds.as_mut() {
                    sounds.play(
                        packs,
                        minecraft_survival::offhand::BREAK_SOUND,
                        None,
                        0.8,
                        pitch,
                    );
                }
            }
            let pitch = 0.8 + entities.random_float() * 0.4;
            if let Some(sounds) = sounds.as_mut() {
                sounds.play(
                    packs,
                    minecraft_survival::offhand::BLOCK_SOUND,
                    None,
                    1.0,
                    pitch,
                );
            }
            if push.disable_shield_ticks > 0 {
                hand.cooldowns.start(
                    minecraft_survival::offhand::SHIELD,
                    push.disable_shield_ticks,
                );
                hand.blocking = None;
                let pitch = 0.8 + entities.random_float() * 0.4;
                if let Some(sounds) = sounds.as_mut() {
                    sounds.play(
                        packs,
                        minecraft_survival::offhand::BREAK_SOUND,
                        None,
                        0.8,
                        pitch,
                    );
                }
            }
        }
        let hurt = survival.struck(amount, blocked, armor, from, cause, effect);
        landed.extend(push.landing(hurt, blocked > 0.0, knockback_resistance));
    }
    for (slot, held) in entities.take_boosts(player_id) {
        hand.swing = Some(0.0);
        hand.swing_offhand = slot == minecraft_survival::offhand::OFFHAND;
        if inventory.slots[slot]
            .as_ref()
            .is_some_and(|stack| stack.id == held)
            && let Some(sound) =
                entities.spend_boost(inventory, slot, !player_mode.consumes_items())
            && let Some(sounds) = sounds.as_mut()
        {
            sounds.play(packs, sound.event, None, sound.volume, sound.pitch);
        }
    }
    for used in outputs.item_uses_done {
        if used.outcome.consumed {
            hand.swing = Some(0.0);
            hand.swing_offhand = used.slot == minecraft_survival::offhand::OFFHAND;
        }
        finish_item_use(
            entities,
            inventory,
            sounds.as_mut(),
            packs,
            origin,
            used,
            player_mode.consumes_items(),
        );
    }
    let present = alive && player_mode.mode != minecraftoss_player::GameMode::Spectator;
    crate::minecraft_riding::step(
        riding, reattach, steps, entities, authority, rider, origin, present,
    );
    *jump_bar = riding.and_then(|_| entities.view.world.ride_jump_scale(player_id));
    for to in teleports
        .iter()
        .filter(|(uuid, _)| alive && uuid == local_uuid)
        .map(|(_, to)| to)
    {
        crate::minecraft_riding::dismount(riding, Some(&mut *entities), authority, rider);
        let to = minecraft_survival::bands::kept_in_walls(dimension, *to);
        authority.teleport(client, sim::voxel::to_map(origin, to));
        steps.air_peak = None;
    }
    for landing in outputs.pearl_landings {
        if !alive {
            continue;
        }
        crate::minecraft_riding::dismount(riding, Some(&mut *entities), authority, rider);
        authority.teleport(client, sim::voxel::to_map(origin, landing.to_array()));
        survival.hurt(
            minecraft_terrain::thrown::PEARL_DAMAGE,
            minecraftoss_player::survival::Armor::default(),
            None,
            minecraft_survival::death::DamageCause::EnderPearl,
        );
        steps.air_peak = None;
        if let Some(sounds) = sounds.as_mut() {
            sounds.play(
                packs,
                "minecraft:entity.player.teleport",
                Some(at(landing.to_array())),
                1.0,
                1.0,
            );
        }
    }
    let pushed = alive
        && player_mode.mode != minecraftoss_player::GameMode::Spectator
        && ps.pm_flags & playerstate_iw4::pm_flags::MC_FLYING == 0;
    for push in outputs.pushes.into_iter().chain(landed).filter(|_| pushed) {
        let launch = minecraft_survival::impulse::launch_velocity(
            push,
            f64::from(ps.gravity) / f64::from(sim::voxel::BLOCK),
        ) * f64::from(sim::voxel::BLOCK);
        authority.push_velocity(client, [launch.x as f32, -launch.z as f32, launch.y as f32]);
        survival.start_impulse(feet[1]);
    }
    for hit in outputs.potions.into_iter().filter(|_| alive) {
        if let Some(sound) = survival.take_potion(&hit.potion, Some(hit.at.to_array()), || {
            entities.random_float()
        }) && let Some(sounds) = sounds.as_mut()
        {
            sounds.play(packs, sound.event, None, sound.volume, sound.pitch);
        }
    }
    for (amplifier, omen_at) in outputs.raid_omens.into_iter().filter(|_| alive) {
        if survival.turn_bad_omen(amplifier, omen_at)
            && let Some(sounds) = sounds.as_mut()
        {
            sounds.play(packs, RAID_OMEN_SOUND, Some(at(feet)), 1.0, 1.0);
        }
    }
    for amplifier in outputs.raid_heroes {
        survival.become_hero(amplifier);
    }
    for horn in outputs.raid_horns {
        if let Some(sounds) = sounds.as_mut() {
            let horn = at(horn.to_array());
            sounds.play(packs, minecraft_terrain::raids::HORN_SOUND, Some(horn), minecraft_terrain::raids::HORN_VOLUME, 1.0);
        }
    }
    if let Some(bars) = outputs.raid_bars {
        *raid_bars = bars;
    }
}

const RAID_OMEN_SOUND: &str = "minecraft:event.mob_effect.raid_omen";

#[allow(clippy::too_many_arguments)]
pub(crate) fn settle(
    player: &mut Player,
    entities: &mut Entities,
    globals: &Globals,
    authority: &mut sim::SimWorld,
    client: sim::ClientId,
    origin: [f64; 3],
    alive: bool,
    totem_popped: bool,
    language: Option<&HashMap<String, String>>,
    sounds: &mut Option<Sounds>,
    packs: &PackStack,
) -> Settled {
    let Player {
        id,
        uuid,
        survival,
        inventory,
        selected,
        ..
    } = player;
    let mut told = None;
    let popped = hold_off_death(
        survival,
        inventory,
        *selected,
        sounds.as_mut(),
        packs,
        totem_popped,
    );
    if popped && let Some(ps) = authority.player(client) {
        let (_, maxs) = sim::voxel::player_hull(ps);
        let feet = sim::voxel::to_block(origin, ps.origin);
        pop_totem(
            entities,
            (uuid, client.0),
            feet,
            f64::from(maxs[2] / sim::voxel::BLOCK),
        );
    }
    if alive
        && !sim::voxel::player_damage_pending(client.0)
        && let Some(player) = authority.player(client).copied()
    {
        let settle = survival.settle(player.health, player.max_health);
        if let Some(health) = settle.set {
            authority.set_health(client, health);
        }
        if let Some((amount, from)) = settle.damage {
            let death = (survival.status.health <= 0.0).then(|| {
                let line = match language {
                    Some(language) => survival.death_line(
                        language,
                        |id| entities.mob_present(id),
                        |client| killer(globals, entities, client),
                    ),
                    None => survival.death_line(
                        &minecraft_terrain::item_icons::language(packs)
                            .unwrap_or_else(|_| HashMap::new()),
                        |id| entities.mob_present(id),
                        |client| killer(globals, entities, client),
                    ),
                };
                sim::voxel::MinecraftDeath {
                    message: line.message,
                    killer: line.killer,
                }
            });
            if death.is_some()
                && let Some(sounds) = sounds.as_mut()
            {
                let sound = minecraft_survival::survival::death_sound(|| entities.random_float());
                sounds.play(packs, sound.event, None, sound.volume, sound.pitch);
            }
            if death.is_some() {
                let name = globals.chat.name(*id).unwrap_or("Player");
                let text = survival.death_text(
                    name,
                    |id| entities.mob_present(id),
                    |client| killer(globals, entities, client),
                );
                told =
                    minecraft_survival::commands::tell_death(&globals.game_rules, survival, text);
            }
            sim::voxel::push_player_damage(
                client.0,
                amount,
                from.map(|b| sim::voxel::to_map(origin, b)),
                death,
            );
        }
        if let Some(blocked) =
            survival.sprint_change(player.pm_flags & playerstate_iw4::pm_flags::SPRINT_BLOCKED != 0)
        {
            authority.set_sprint_blocked(client, blocked);
        }
    }
    Settled { told, popped }
}

pub(crate) struct Settled {
    pub(crate) told: Option<Clientbound>,
    pub(crate) popped: bool,
}

pub(crate) fn set_player_boxes(
    levels: &mut BTreeMap<Dimension, Level>,
    players: &BTreeMap<PlayerUuid, Player>,
    authority: &sim::SimWorld,
    origins: &HashMap<Dimension, [f64; 3]>,
    (host, host_client): (&PlayerUuid, sim::ClientId),
) {
    let names: HashMap<u64, String> = players
        .values()
        .filter_map(|player| {
            let held = player.inventory.slots.get(player.selected)?.as_ref()?;
            Some((
                u64::from(player.id.0),
                minecraft_survival::pvp::custom_name(held)?,
            ))
        })
        .collect();
    for (dimension, level) in levels.iter_mut() {
        let Some(entities) = level.entities.as_mut() else {
            continue;
        };
        let boxes = origins.get(dimension).map_or_else(Vec::new, |&origin| {
            players
                .values()
                .filter(|player| player.dimension == *dimension)
                .filter_map(|player| {
                    let client = if player.uuid == *host {
                        host_client
                    } else {
                        sim::ClientId(player.id.0)
                    };
                    player_box(player, authority.player(client)?, origin)
                })
                .collect()
        });
        entities.set_player_boxes(boxes);
        entities.set_held_names(names.clone());
    }
}

fn killer(
    globals: &Globals,
    entities: &Entities,
    client: u32,
) -> Option<minecraft_survival::death::Killer> {
    Some(minecraft_survival::death::Killer {
        name: globals
            .chat
            .name(minecraft_commands::PlayerId(client))?
            .to_owned(),
        weapon: entities.held_name(u64::from(client)),
    })
}

fn player_box(
    player: &Player,
    ps: &PlayerState,
    origin: [f64; 3],
) -> Option<minecraft_survival::pvp::PlayerBox> {
    let present = !player.dead
        && player.switching.is_none()
        && player.arrival.is_none()
        && ps.pm_type == 0
        && player.mode.mode != minecraftoss_player::GameMode::Spectator
        && minecraft_survival::bands::holds(player.dimension, f64::from(ps.origin[2]));
    if !present {
        return None;
    }
    let (_, maxs) = sim::voxel::player_hull(ps);
    Some(minecraft_survival::pvp::PlayerBox::standing(
        u64::from(player.id.0),
        sim::voxel::to_block(origin, ps.origin),
        f64::from(maxs[2] / sim::voxel::BLOCK),
        player.mode.invulnerable(),
    ))
}

pub(crate) fn tell_pvp_deaths(
    deaths: Vec<sim::voxel::PvpDeath>,
    players: &mut BTreeMap<PlayerUuid, Player>,
    (rules, chat): (&GameRules, &CommandServer),
    (host, host_client): (&PlayerUuid, u32),
    weapon_name: impl Fn(u32) -> Option<String>,
) -> Vec<Clientbound> {
    let mut told = Vec::new();
    for death in deaths {
        let player_of = |client: u32| {
            players
                .iter()
                .find(|(uuid, player)| {
                    if client == host_client {
                        *uuid == host
                    } else {
                        *uuid != host && player.id.0 == client
                    }
                })
                .map(|(uuid, player)| (uuid.clone(), player.id))
        };
        let (Some((victim, victim_id)), Some((_, attacker_id))) =
            (player_of(death.victim), player_of(death.attacker))
        else {
            continue;
        };
        let name = |id| chat.name(id).unwrap_or("Player").to_owned();
        let weapon = weapon_name(death.weapon).filter(|_| death.weapon > 0);
        let text = minecraft_survival::death::player_kill_text(
            &name(victim_id),
            &name(attacker_id),
            weapon.as_deref(),
        );
        if let Some(player) = players.get_mut(&victim) {
            told.extend(minecraft_survival::commands::tell_death(
                rules,
                &mut player.survival,
                text,
            ));
        }
    }
    told
}

pub(crate) fn pop_totem(
    entities: &mut Entities,
    (uuid, client): (&PlayerUuid, u32),
    feet: [f64; 3],
    height: f64,
) {
    let key = minecraft_survival::level::opener_key(uuid);
    entities.totem_pop(key, feet, PLAYER_WIDTH, height);
    let event = minecraft_survival::heard::TOTEM_EVENT;
    let heard = minecraft_survival::heard::Heard::Player {
        client,
        event,
        at: feet,
    };
    entities.hear(minecraft_survival::heard::Said::all(heard));
}

pub(crate) fn track_body(
    entities: &mut Entities,
    uuid: &PlayerUuid,
    ps: &PlayerState,
    origin: [f64; 3],
) {
    let (_, maxs) = sim::voxel::player_hull(ps);
    let feet = sim::voxel::to_block(origin, ps.origin);
    let key = minecraft_survival::level::opener_key(uuid);
    entities.track_body(
        key,
        feet,
        PLAYER_WIDTH,
        f64::from(maxs[2] / sim::voxel::BLOCK),
    );
    entities.hold_rope(
        key,
        feet,
        minecraft_yaw(ps.viewangles[1]),
        f64::from(maxs[2] / sim::voxel::BLOCK),
    );
}

fn place(
    player: &mut Player,
    authority: &mut sim::SimWorld,
    client: sim::ClientId,
    ps: &PlayerState,
    origin: [f64; 3],
    gate: Option<&mut SpawnGate>,
    now_ms: u64,
) {
    match gate {
        Some(gate) => place_player(player, authority, client, ps, origin, gate, now_ms),
        None => place_friend(player, authority, client, ps, origin),
    }
}

pub(crate) fn drop_on_death(
    death_watch: &mut minecraft_survival::death::DeathWatch,
    inventory: &mut minecraftoss_player::inventory::Inventory,
    hand: &mut minecraft_survival::player::Hand,
    entities: &mut Entities,
) {
    let Some(death) = death_watch.take().filter(|death| !death.keep_inventory) else {
        return;
    };
    let eye = glam::DVec3::from_array(death.spot.eye);
    for stack in minecraft_survival::death::dropped_on_death(&death, inventory) {
        entities.view.world_items.drop_on_death(stack, eye);
    }
    entities.award_experience(death.spot.feet, death.experience);
    hand.blocking = None;
}

pub(crate) fn owned_weapons(ps: &PlayerState, authority: &sim::SimWorld) -> Vec<u32> {
    ps.weapons
        .iter()
        .filter(|&&w| w > 0)
        .map(|&w| w as u32)
        .filter(|&w| {
            authority
                .weapon_combat_row(w)
                .is_some_and(|facts| facts.inventory_type == 0)
                && sim::voxel::is_killstreak_weapon(authority.weapon_script_name(w))
        })
        .collect()
}

pub(crate) fn tick_actor<'a>(
    player: &'a mut Player,
    authority: &sim::SimWorld,
    origin: [f64; 3],
) -> Option<(TickActor<'a>, PlayerUuid)> {
    if player.switching.is_some() || player.arrival.is_some() {
        return None;
    }
    let ps = authority.player(sim::ClientId(player.id.0))?;
    let alive = ps.pm_type == 0;
    let (forward, strafe, jumping) =
        crate::minecraft_riding::ride_input(authority, sim::ClientId(player.id.0));
    let view = minecraft_survival::entities::PlayerView {
        feet: sim::voxel::to_block(origin, ps.origin),
        alive,
        attackable: alive && player.mode.attackable_by_mobs(),
        spectator: !alive || player.mode.mode == minecraftoss_player::GameMode::Spectator,
        picks_up: alive && player.mode.picks_up(),
        health: player.survival.status.health,
        yaw: minecraft_yaw(ps.viewangles[1]),
        pitch: ps.viewangles[0],
        main_hand: player.inventory.slots[player.selected]
            .as_ref()
            .filter(|_| {
                alive
                    && player.mode.interacts()
                    && !player.arsenal.holds_gun(&player.inventory, player.selected)
            })
            .map(|stack| stack.id.clone()),
        offhand: player.inventory.slots[minecraft_survival::offhand::OFFHAND]
            .as_ref()
            .map(|stack| stack.id.clone()),
        sneaking: ps.pm_flags
            & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE)
            != 0,
        hero: player
            .survival
            .status
            .effect(minecraftoss_player::survival::EffectKind::HeroOfTheVillage)
            .map(|effect| i32::from(effect.amplifier)),
        sleeping: player.sleeping.map(|sleeping| sleeping.head),
        sleep_timer: player.sleep_timer,
        forward,
        strafe,
        jumping,
        swirl: minecraft_survival::swirls::player_swirl(
            &player.survival.status,
            sim::voxel::to_block(origin, ps.origin),
            ps.pm_flags & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE)
                != 0,
        ),
        omens: player.survival.omens(),
    };
    let actor = TickActor {
        id: u64::from(player.id.0),
        view,
        inventory: &mut player.inventory,
        selected: player.selected,
    };
    Some((actor, player.uuid.clone()))
}

#[derive(Default)]
pub(crate) struct Stepped {
    pub(crate) lines: Vec<Clientbound>,
    pub(crate) told: Option<Clientbound>,
    pub(crate) killers: Vec<String>,
    pub(crate) books: Vec<Option<minecraft_net::messages::BookView>>,
    pub(crate) bed: Option<Bed>,
    pub(crate) open_book: Option<minecraft_survival::books::OpenBook>,
    pub(crate) board_allowed: bool,
}

type Bed = minecraft_survival::respawn_block::Bed;

pub(crate) fn friend_beds(
    (globals, levels, players): (
        &Globals,
        &mut BTreeMap<Dimension, Level>,
        &mut BTreeMap<PlayerUuid, Player>,
    ),
    published: bool,
    authority: &mut sim::SimWorld,
    beds: Vec<(PlayerUuid, minecraft_survival::sleep::BedAction)>,
    (shapes, shape_ids): (&mut Shapes, &mut ShapeIds),
) -> Vec<(PlayerUuid, Clientbound)> {
    use minecraft_survival::sleep::BedAction;
    let mut messages = Vec::new();
    for (uuid, action) in beds {
        let Some(player) = players
            .get_mut(&uuid)
            .filter(|player| player.switching.is_none())
        else {
            continue;
        };
        let dimension = player.dimension;
        let Some(level) = levels.get_mut(&dimension) else {
            continue;
        };
        if matches!(action, BedAction::Lie(_)) {
            let rider = (sim::ClientId(player.id.0), u64::from(player.id.0));
            let entities = level.entities.as_mut();
            crate::minecraft_riding::dismount(&mut player.riding, entities, authority, rider);
        }
        let events = match action {
            BedAction::Lie(bed) => {
                minecraft_survival::sleep::start(globals, level, players, &uuid, &bed, published)
            }
            BedAction::Leave => {
                minecraft_survival::sleep::stop(globals, level, players, &uuid, false, published)
            }
        };
        let band = minecraft_survival::bands::level(dimension);
        let tables = (&mut *shapes, &mut *shape_ids, band);
        let registries = &globals.registries;
        crate::minecraft_world::refresh_shapes(&level.terrain, registries, tables, &events.written);
        messages.extend(events.messages);
    }
    messages
}

pub(crate) fn lay_friends(
    players: &BTreeMap<PlayerUuid, Player>,
    (host, shown): (&PlayerUuid, Option<Dimension>),
    authority: &mut sim::SimWorld,
    origins: &HashMap<Dimension, [f64; 3]>,
    in_bed: &mut BTreeSet<u32>,
    bodies: &mut Vec<(u32, Mat4)>,
) {
    let mut lying = BTreeSet::new();
    for player in players.values().filter(|player| &player.uuid != host) {
        let client = player.id.0;
        let sleeping = player.sleeping.filter(|_| !player.dead);
        let (Some(sleeping), Some(&origin)) = (sleeping, origins.get(&player.dimension)) else {
            if in_bed.contains(&client) && player.riding.is_none() {
                authority.ride(sim::ClientId(client), None);
            }
            continue;
        };
        let feet = sleeping.feet();
        authority.ride(sim::ClientId(client), Some(sim::voxel::to_map(origin, feet)));
        lying.insert(client);
        if shown == Some(player.dimension) {
            let root = crate::minecraft_sleep::lying_root(origin, feet, sleeping.facing);
            bodies.push((client, root));
        }
    }
    *in_bed = lying;
}

pub(crate) type LecternClicks<'a> = (&'a mut Option<BlockPos>, Vec<([i32; 3], Option<i32>)>);

pub(crate) struct FriendWorld<'a> {
    pub(crate) levels: &'a mut BTreeMap<Dimension, Level>,
    pub(crate) globals: &'a Globals,
    pub(crate) authority: &'a mut sim::SimWorld,
    pub(crate) queues: &'a mut VoxelQueues,
    pub(crate) shapes: &'a mut Shapes,
    pub(crate) shape_ids: &'a mut ShapeIds,
    pub(crate) language: Option<&'a HashMap<String, String>>,
    pub(crate) packs: &'a PackStack,
    pub(crate) atlas: &'a minecraft_terrain::mesh::Atlas,
    pub(crate) mining: &'a mut crate::minecraft_mining::Mining,
    pub(crate) minings: &'a mut HashMap<Dimension, crate::minecraft_mining::Mining>,
    pub(crate) shown: Option<Dimension>,
    pub(crate) sounds: Option<&'a mut Sounds>,
    pub(crate) light: Option<&'a minecraft_terrain::lighting::SkyLight>,
    pub(crate) dt: f64,
    pub(crate) now: f64,
    pub(crate) instant: std::time::Instant,
    pub(crate) guns: &'a mut crate::minecraft_fork_items::ForkLook,
}

pub(crate) fn mining_in<'m>(
    shown: (&'m mut crate::minecraft_mining::Mining, Option<Dimension>),
    others: &'m mut HashMap<Dimension, crate::minecraft_mining::Mining>,
    dimension: Dimension,
) -> &'m mut crate::minecraft_mining::Mining {
    match shown {
        (mining, Some(here)) if here == dimension => mining,
        _ => others.entry(dimension).or_default(),
    }
}

fn leave_level(player: &mut Player, level: &mut Level) {
    let feet = player
        .place
        .filter(|place| place.dimension == level.dimension)
        .map(|place| place.feet);
    if let (Some(entities), Some(feet)) = (level.entities.as_mut(), feet) {
        minecraft_survival::menus::close_merchant_now(player, entities, feet);
    }
    let thrown = minecraft_survival::menus::close(player);
    if let Some(entities) = level.entities.as_mut() {
        let id = u64::from(player.id.0);
        let selected = player.selected;
        let experience = entities.leave(id, &mut player.inventory, selected, feet);
        let _ = player.survival.gain_experience(experience);
        if let Some(feet) = feet {
            for stack in &thrown {
                entities.drop_given(stack, feet);
            }
        }
    }
    minecraft_survival::level::leave(level, &player.uuid);
    let Player {
        survival,
        hand,
        steps,
        ..
    } = player;
    survival.stop_eating();
    hand.blocking = None;
    hand.charging = None;
    hand.scoping = None;
    hand.swing = None;
    hand.recent.clear();
    steps.air_peak = None;
    steps.last = None;
}

pub(crate) fn step_friend(
    player: &mut Player,
    world: &mut FriendWorld<'_>,
    client: sim::ClientId,
    origin: [f64; 3],
    (selected, skate): (Option<u8>, Skating),
    hands: &mut HandQueue,
    (open, clicks): LecternClicks<'_>,
) -> Stepped {
    let mut stepped = Stepped::default();
    let Some(ps) = world.authority.player(client).copied() else {
        hands.take(world.instant, |_| false);
        return stepped;
    };
    let eye = sim::voxel::to_block(
        origin,
        [
            ps.origin[0],
            ps.origin[1],
            ps.origin[2] + ps.view_height_current,
        ],
    );
    let mode = player.mode;
    let entities = world
        .levels
        .get(&player.dimension)
        .and_then(|level| level.entities.as_ref());
    let own = u64::from(player.id.0);
    let taken = hands.take(world.instant, |target| {
        minecraft_survival::actions::allows(
            target,
            eye,
            &mode,
            |id| entities.and_then(|entities| entities.mob_box(id)),
            |id| {
                entities
                    .filter(|_| id != own)
                    .and_then(|entities| entities.player_box(id))
            },
        )
    });
    if let Some(slot) = selected {
        player.selected = usize::from(slot).min(minecraft_survival::guns::HOTBAR - 1);
    }
    if player.dead {
        player.held = None;
    }
    let mut sounds = Some(Sounds::recorder());
    let remote = crate::minecraft_world::remote_view(ps.pm_type, ps.other_flags);
    let begun = begin(
        player,
        world.levels,
        world.globals,
        world.authority,
        client,
        origin,
        Some(&ps),
        remote,
        None,
        0,
        world.dt,
        &mut sounds,
        world.packs,
        &mut stepped.lines,
    );
    match begun {
        Begun::Left(switch) => {
            if let Some(level) = world.levels.get_mut(&switch.from) {
                leave_level(player, level);
                if let Some(at) = open.take() {
                    stepped.books.push(None);
                    if let Some(entities) = level.entities.as_mut() {
                        let close = minecraft_terrain::lectern::LecternAction::Close;
                        entities.lectern(u64::from(player.id.0), at, close, None);
                    }
                }
            }
        }
        Begun::Arriving { standing: true, .. }
            if player.arrival.is_none()
                && !minecraft_survival::the_end::holding_for_credits(player) =>
        {
            player.switching = None;
            diag::info!(
                World,
                "Minecraft {} arrived in {}",
                player.name,
                player.dimension.dimension_type()
            );
        }
        _ => {}
    }
    if !matches!(begun, Begun::Live) {
        let recorded = sounds.as_mut().map_or_else(Vec::new, Sounds::take_recorded);
        let arrived = player
            .arrival
            .or(player.place)
            .filter(|place| place.dimension == player.dimension)
            .map(|place| {
                let [x, y, z] = place.feet;
                [x, y + crate::minecraft_spawn_gate::SPAWN_EYE_HEIGHT, z]
            });
        if let (Some(entities), Some(eye)) = (
            world
                .levels
                .get_mut(&player.dimension)
                .and_then(|level| level.entities.as_mut()),
            arrived,
        ) {
            retell(recorded, None, world.packs, entities, (client.0, origin, eye));
        }
        return stepped;
    }
    let feet = (ps.pm_type == 0).then(|| sim::voxel::to_block(origin, ps.origin));
    crate::minecraft_portal::step(
        player,
        world.levels,
        world.globals,
        feet,
        world.dt,
        sounds.as_mut(),
        world.packs,
        None,
    );
    crate::minecraft_end::pearls_through(player, world.levels);
    if player.switching.is_some() {
        return stepped;
    }
    let Some(level) = world.levels.get_mut(&player.dimension) else {
        return stepped;
    };
    let Level {
        entities, terrain, ..
    } = level;
    let broken_before = entities.as_ref().map_or(0, |e| e.broken_items.len());
    let life = life(
        player,
        world.globals,
        terrain,
        world.authority,
        client,
        &ps,
        world.queues,
    );
    stepped.killers.clone_from(&life.killers);
    let moved = movement(
        player,
        world.globals,
        terrain,
        entities,
        (&mut *world.shapes, &mut *world.shape_ids),
        world.authority,
        client,
        origin,
        &ps,
        (life.alive, skate),
        world.dt,
        &mut sounds,
        world.packs,
    );
    let feet = sim::voxel::to_block(origin, ps.origin);
    stepped.board_allowed = crate::minecraft_world::board_allowed(
        life.alive,
        player.mode.mode == minecraftoss_player::GameMode::Spectator,
        ps.pm_flags,
        moved.eye_in_water,
        player.sleeping.is_some(),
        player.survival.in_powder_snow(),
    );
    let walker = minecraft_survival::survival::walks_on_powder_snow(&player.inventory);
    world.authority.set_powder_snow_walker(client, walker);
    let frozen = player.survival.ticks_frozen();
    let frost = frost_scale(&terrain.scene, feet, frozen, life.alive);
    world.authority.set_minecraft_move_scale(client, frost);
    player
        .death_watch
        .stood(minecraft_survival::death::DeathSpot { feet, eye });
    if let Some(entities) = entities.as_mut() {
        struck(
            player,
            entities,
            world.authority,
            client,
            origin,
            &ps,
            life.alive,
            &[],
            &mut sounds,
            world.packs,
        );
        let found = entities.take_experience(u64::from(player.id.0));
        let chime = player.survival.gain_experience(found);
        if let Some(sounds) = sounds.as_mut() {
            if found > 0 {
                let (r1, r2) = (entities.random_float(), entities.random_float());
                let pitch = (r1 - r2) * 0.35 + 0.9;
                let orb = minecraft_survival::heard::ORB_PICKUP_SOUND;
                sounds.play(world.packs, orb, None, 0.1, pitch);
            }
            if let Some(volume) = chime {
                let chime = minecraft_survival::heard::LEVEL_UP_SOUND;
                sounds.play(world.packs, chime, None, volume, 1.0);
            }
        }
        track_body(entities, &player.uuid, &ps, origin);
        stepped.told = settle(
            player,
            entities,
            world.globals,
            world.authority,
            client,
            origin,
            life.alive,
            life.totem_popped,
            world.language,
            &mut sounds,
            world.packs,
        )
        .told;
        let id = u64::from(player.id.0);
        let views = entities.take_lecterns(id);
        let may_build = player.mode.may_build();
        let lectern =
            minecraft_survival::session::friend_lectern(open, clicks, views, eye, may_build);
        for (pos, action) in lectern.actions {
            entities.lectern(id, pos, action, None);
        }
        for book in lectern.taken {
            let stack = entities.book_stack(&book);
            let (dropped, _) = minecraft_survival::commands::give_stacks(
                &mut player.inventory,
                player.selected,
                &stack,
                1,
            );
            for stack in &dropped {
                entities.drop_given(stack, feet);
            }
        }
        stepped.books.extend(lectern.books);
        if !life.alive {
            minecraft_survival::menus::close_merchant_now(player, entities, feet);
            let yaw = minecraft_yaw(ps.viewangles[1]);
            for stack in minecraft_survival::menus::close(player) {
                let eye = glam::DVec3::from_array(eye);
                entities.view.world_items.toss(stack, eye, yaw, ps.viewangles[0]);
            }
        }
        let Player {
            death_watch,
            inventory,
            hand,
            ..
        } = player;
        drop_on_death(death_watch, inventory, hand, entities);
    }
    let (effects, (table, block, bench), bed, open_book) = friend_hand(
        player,
        world,
        client,
        origin,
        &ps,
        &moved,
        taken,
        (life.alive, skate.active),
    );
    stepped.bed = bed;
    stepped.open_book = open_book;
    if let Some(level) = world.levels.get_mut(&player.dimension) {
        let recipes = player.inventory.recipes.clone();
        minecraft_survival::menus::open_station(
            player,
            &mut level.containers,
            &level.terrain,
            world.globals,
            &recipes,
            table,
            block,
            bench,
        );
    }
    if let Some(entities) = world
        .levels
        .get_mut(&player.dimension)
        .and_then(|level| level.entities.as_mut())
    {
        let random = || entities.random_float();
        for said in minecraft_survival::heard::of_effects(&effects, client.0, eye, random) {
            entities.hear(said);
        }
    }
    let here = world.shown == Some(player.dimension);
    hear(
        world.sounds.as_deref_mut().filter(|_| here),
        world.packs,
        (origin, Some(eye)),
        effects,
        &mut stepped.lines,
    );
    if let Some(entities) = world
        .levels
        .get_mut(&player.dimension)
        .and_then(|level| level.entities.as_mut())
    {
        let recorded = sounds.as_mut().map_or_else(Vec::new, Sounds::take_recorded);
        let host = world.sounds.as_deref_mut().filter(|_| here);
        retell(
            recorded,
            host,
            world.packs,
            entities,
            (client.0, origin, eye),
        );
        let from = broken_before.min(entities.broken_items.len());
        let at = glam::DVec3::from_array(eye).as_vec3();
        let yaw = minecraft_yaw(ps.viewangles[1]);
        let shown = (&mut *world.mining, world.shown);
        let mining = mining_in(shown, &mut *world.minings, player.dimension);
        for item in entities.broken_items.split_off(from) {
            let (packs, atlas) = (world.packs, world.atlas);
            mining.break_burst(packs, atlas, at, yaw, ps.viewangles[0], &item);
        }
    }
    let class = crate::minecraft_fork_items::class_of(world.authority, client);
    let given = world.guns.first_loadout(
        (&mut player.inventory, player.selected, &mut player.tags),
        life.alive,
        class,
    );
    if let Some(entities) = world
        .levels
        .get_mut(&player.dimension)
        .and_then(|level| level.entities.as_mut())
    {
        for stack in &given {
            entities.drop_given(stack, feet);
        }
    }
    let owned = owned_weapons(&ps, world.authority);
    let room = crate::minecraft_fork_items::gun_room(&ps, world.authority);
    let Player {
        survival,
        inventory,
        selected,
        follow,
        mode,
        arsenal,
        dimension,
        ..
    } = player;
    minecraft_survival::guns::sync_weapons(inventory, &owned);
    let hotbar = minecraft_survival::guns::Hotbar {
        selected: *selected,
        room,
        binding: &mut *arsenal,
    };
    let flare_at =
        crate::minecraft_fork_items::flare_place(*dimension, origin, minecraft_yaw(ps.viewangles[1]));
    let settled = world.guns.settle(inventory, mode, client.0, hotbar, flare_at);
    if let Some(entities) = world
        .levels
        .get_mut(dimension)
        .and_then(|level| level.entities.as_mut())
    {
        for stack in settled.iter().flat_map(|settled| &settled.dropped) {
            entities.drop_given(stack, feet);
        }
    }
    if settled.is_some_and(|settled| settled.too_many) {
        stepped.lines.push(minecraft_commands::Clientbound::System {
            text: minecraft_commands::Text::literal(minecraft_survival::guns::TOO_MANY_GUNS),
            overlay: true,
        });
    }
    follow.request_with(inventory, arsenal.guns(), selected, ps.weapon);
    let protected = life.alive
        && !survival.killed()
        && minecraft_survival::offhand::totem_slot(inventory, *selected).is_some();
    sim::voxel::set_death_protected(client.0, protected);
    crate::minecraft_fork_items::mirror_gun_guard(client.0, survival, inventory);
    stepped
}

#[allow(clippy::too_many_arguments)]
fn friend_hand(
    player: &mut Player,
    world: &mut FriendWorld<'_>,
    client: sim::ClientId,
    origin: [f64; 3],
    ps: &PlayerState,
    moved: &Moved,
    taken: HandTake,
    (alive, skating): (bool, bool),
) -> (
    Vec<Effect>,
    Opens,
    Option<Bed>,
    Option<minecraft_survival::books::OpenBook>,
) {
    let Some(level) = world.levels.get_mut(&player.dimension) else {
        return (Vec::new(), (None, None, None), None, None);
    };
    let interacts = alive && player.mode.interacts();
    if interacts
        && taken.swap
        && minecraft_survival::offhand::swap_hands(&mut player.inventory, player.selected)
    {
        player.survival.stop_eating();
        player.hand.blocking = None;
    }
    let mut actor = actor(ps, origin, alive);
    actor.skating = skating;
    if let Some(entities) = level.entities.as_mut().filter(|_| interacts) {
        for whole in taken.drops {
            let selected = player.selected;
            let Some(stack) = player.inventory.drop_selected(selected, whole) else {
                continue;
            };
            if minecraft_survival::guns::weapon_of(&stack).is_some() {
                if let Some(back) = player.inventory.add_item(stack, selected) {
                    player.inventory.cursor = Some(back);
                }
            } else {
                let eye = glam::DVec3::from_array(actor.eye);
                entities
                    .view.world_items
                    .toss(stack, eye, actor.yaw, actor.pitch);
            }
        }
    }
    let mut body = MwBody {
        authority: &mut *world.authority,
        client,
        ps,
        registries: &world.globals.registries,
        shapes: &mut *world.shapes,
        shape_ids: &mut *world.shape_ids,
        level: minecraft_survival::bands::level(player.dimension),
    };
    let light = world.light.filter(|_| world.shown == Some(level.dimension));
    let acted = minecraft_survival::actions::act(
        player,
        level,
        world.globals,
        &mut body,
        &actor,
        taken.hands,
        moved,
        light,
        world.now,
    );
    let mut effects = acted.effects;
    let mut refs = crate::minecraft_mining::WorldRefs {
        stream: &mut level.terrain.stream,
        scene: &mut level.terrain.scene,
        packs: world.packs,
        atlas: world.atlas,
        registries: &world.globals.registries,
        level: minecraft_survival::bands::level(level.dimension),
    };
    let shown = (&mut *world.mining, world.shown);
    let mining = mining_in(shown, &mut *world.minings, level.dimension);
    let broken = mining.break_now(&acted.creative_broken, &mut level.shots, &mut refs);
    let dug = mining.break_now(&acted.hand_broken, &mut level.shots, &mut refs);
    for (pos, block, _) in broken.iter().chain(&dug) {
        if let Some(kind) = level.terrain.scene.sound_type(block) {
            let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
            effects.push(Effect::Sound {
                event: kind.break_sound.to_string(),
                at: Some(centre),
                volume: (kind.volume + 1.0) / 2.0,
                pitch: kind.pitch * 0.8,
                spread: 0.0,
            });
        }
    }
    let harvest = minecraft_survival::actions::harvest(
        player,
        level,
        world.globals,
        (broken, dug),
        &acted.hand_mined,
        actor.feet,
    );
    effects.extend(harvest.effects);
    if let Some(entities) = level.entities.as_mut() {
        entities.broken_items.extend(harvest.worn_out);
    }
    (
        effects,
        (acted.open_table, acted.open_block, acted.open_bench),
        acted.bed,
        acted.open_book,
    )
}

fn retell(
    recorded: Vec<crate::minecraft_sounds::Played>,
    mut host: Option<&mut Sounds>,
    packs: &PackStack,
    entities: &mut Entities,
    (client, origin, eye): (u32, [f64; 3], [f64; 3]),
) {
    for played in recorded {
        let from = played
            .at
            .map(|at| sim::voxel::to_block(origin, at.to_array()));
        let volume_pitch = (played.volume, played.pitch);
        let said =
            minecraft_survival::heard::played(&played.event, from, volume_pitch, client, eye);
        if let Some(sounds) = host.as_deref_mut().filter(|_| said.only.is_none()) {
            let at = played
                .at
                .unwrap_or_else(|| Vec3::from_array(sim::voxel::to_map(origin, eye)));
            sounds.play(packs, &played.event, Some(at), played.volume, played.pitch);
        }
        entities.hear(said);
    }
}

type Opens = (
    Option<BlockPos>,
    Option<minecraft_terrain::containers::Opened>,
    Option<(BlockPos, minecraft_terrain::stations::Workshop)>,
);

fn hear(
    mut sounds: Option<&mut Sounds>,
    packs: &PackStack,
    (origin, heard_at): ([f64; 3], Option<[f64; 3]>),
    effects: Vec<Effect>,
    lines: &mut Vec<Clientbound>,
) {
    for effect in effects {
        match effect {
            Effect::Sound {
                event,
                at,
                volume,
                pitch,
                spread,
            } => {
                let at = at.or(heard_at);
                if let (Some(sounds), Some(at)) = (sounds.as_deref_mut(), at) {
                    let pitch = if spread == 0.0 {
                        pitch
                    } else {
                        pitch + sounds.random() * spread
                    };
                    let at = Vec3::from_array(sim::voxel::to_map(origin, at));
                    sounds.play(packs, &event, Some(at), volume, pitch);
                }
            }
            Effect::Chat(line) => lines.push(line),
            Effect::Overlay(key) => lines.push(Clientbound::System {
                text: minecraft_commands::Text::translate(&key, Vec::new()),
                overlay: true,
            }),
            Effect::Crumbs { .. } | Effect::Dig { .. } => {}
        }
    }
}

pub(crate) fn friend_menus(
    player: &mut Player,
    world: &mut FriendWorld<'_>,
    (client, origin): (sim::ClientId, [f64; 3]),
    inputs: Vec<MenuInput>,
    was: Option<minecraft_terrain::containers::Opened>,
) -> Vec<Clientbound> {
    let mut lines = Vec::new();
    let ps = world.authority.player(client).copied();
    let Some(level) = world
        .levels
        .get_mut(&player.dimension)
        .filter(|_| player.switching.is_none())
    else {
        return lines;
    };
    let Level {
        entities,
        containers,
        terrain,
        openers,
        ..
    } = level;
    let Some(entities) = entities.as_mut() else {
        return lines;
    };
    let guns = world.guns.rules();
    let books = minecraft_survival::menus::Books::of(&world.globals.data)
        .with_guns(guns.as_ref().map(|guns| guns as &dyn minecraft_terrain::gun_mods::GunRules));
    let globals = world.globals;
    let known = |id: &str| globals.chat.registries().items.iter().any(|item| item == id);
    let mut clicked = minecraft_survival::menus::take_inputs(
        player,
        containers,
        books,
        inputs,
        world.language,
        &known,
        || entities.random_float(),
    );
    let at = player.place.map_or([0.0; 3], |place| place.feet);
    let traded = minecraft_survival::menus::merchant(player, entities, at, true, !player.dead);
    clicked.thrown.extend(traded.thrown);
    let Some(ps) = ps else {
        if let Some(feet) = player.place.map(|place| place.feet) {
            for stack in &clicked.thrown {
                entities.drop_given(stack, feet);
            }
        }
        return lines;
    };
    let eye = sim::voxel::to_block(
        origin,
        [
            ps.origin[0],
            ps.origin[1],
            ps.origin[2] + ps.view_height_current,
        ],
    );
    let mounted = minecraft_survival::menus::mount(player, entities, eye, false, !player.dead);
    clicked.thrown.extend(mounted.thrown);
    let invalid = player
        .menus
        .open
        .as_ref()
        .is_some_and(|menu| !minecraft_survival::menus::still_valid(menu, &terrain.scene, eye));
    if invalid {
        clicked
            .thrown
            .extend(minecraft_survival::menus::close(player));
    }
    let yaw = minecraft_yaw(ps.viewangles[1]);
    for stack in clicked.thrown {
        let at = glam::DVec3::from_array(eye);
        entities.view.world_items.toss(stack, at, yaw, ps.viewangles[0]);
    }
    let _ = player.survival.gain_experience(clicked.experience);
    let mut body = MwBody {
        authority: &mut *world.authority,
        client,
        ps: &ps,
        registries: &world.globals.registries,
        shapes: &mut *world.shapes,
        shape_ids: &mut *world.shape_ids,
        level: minecraft_survival::bands::level(player.dimension),
    };
    let mut effects = minecraft_survival::menus::finish(
        entities,
        terrain,
        &mut body,
        clicked.used,
        clicked.anvil,
    );
    let now = player.menus.block().cloned();
    let spectator = player.mode.mode == minecraftoss_player::GameMode::Spectator;
    let lids = minecraft_survival::menus::Lids {
        entities,
        terrain,
        openers,
        body: &mut body,
    };
    let feet = sim::voxel::to_block(origin, ps.origin);
    effects.extend(minecraft_survival::menus::lids(
        lids,
        (&player.uuid, u64::from(player.id.0), feet),
        was.as_ref(),
        now.as_ref(),
        spectator,
    ));
    let random = || entities.random_float();
    for said in minecraft_survival::heard::of_effects(&effects, client.0, eye, random) {
        entities.hear(said);
    }
    let here = world.shown == Some(player.dimension);
    hear(
        world.sounds.as_deref_mut().filter(|_| here),
        world.packs,
        (origin, Some(eye)),
        effects,
        &mut lines,
    );
    lines
}

fn finish_item_use(
    entities: &mut Entities,
    inventory: &mut minecraftoss_player::inventory::Inventory,
    sounds: Option<&mut Sounds>,
    packs: &PackStack,
    origin: [f64; 3],
    used: minecraft_survival::entities::SentItemUse,
    consumes: bool,
) {
    let heard = minecraft_survival::level::finish_item_use(entities, inventory, used, consumes);
    if let Some(sounds) = sounds {
        for sound in heard {
            let at = sound
                .at
                .map(|at| Vec3::from_array(sim::voxel::to_map(origin, at)));
            sounds.play(packs, sound.event, at, sound.volume, sound.pitch);
        }
    }
}

fn hold_off_death(
    survival: &mut minecraft_survival::survival::Survival,
    inventory: &mut minecraftoss_player::inventory::Inventory,
    selected: usize,
    sounds: Option<&mut Sounds>,
    packs: &PackStack,
    struck: bool,
) -> bool {
    let popped = minecraft_survival::player::hold_off_death(survival, inventory, selected, struck);
    if popped && let Some(sounds) = sounds {
        sounds.play(
            packs,
            minecraft_survival::offhand::TOTEM_SOUND,
            None,
            1.0,
            1.0,
        );
    }
    popped
}

fn scene_block(block: &minecraftoss_player::Block) -> Option<minecraft_terrain::scene::Block> {
    Some(minecraft_terrain::scene::Block {
        id: minecraft_terrain::pack::ResourceId::parse(&block.id).ok()?,
        properties: block.properties.clone(),
    })
}

fn obstacles(
    predicted: &PlayerState,
    authority: &sim::SimWorld,
    local: sim::ClientId,
    entities: &Entities,
) -> Vec<[f64; 6]> {
    let hull = |ps: &PlayerState| {
        let (mins, maxs) = sim::voxel::player_hull(ps);
        sim::voxel::body_box(ps.origin, mins, maxs)
    };
    let mut out: Vec<[f64; 6]> = authority
        .player(local)
        .map_or_else(|| hull(predicted), hull)
        .into_iter()
        .collect();
    let own = authority.player(local).map_or(predicted.origin[2], |ps| ps.origin[2]);
    authority.visit_players(|id, ps| {
        let standing = matches!(
            ps.pm_type,
            0 | playerstate_iw4::PM_TYPE_NORMAL_LINKED | playerstate_iw4::PM_TYPE_LAST_STAND
        );
        if id != local && standing && sim::voxel::shown_to(Some(own), ps.origin[2]) {
            out.extend(hull(ps));
        }
    });
    out.extend(entities.boxes().into_iter().map(|(_, b)| b));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_death_queued_for_one_client_waits_for_that_clients_own_step() {
        let mut queues = VoxelQueues::default();
        let deaths = vec![(1, "Zombie".to_owned()), (0, "Skeleton".to_owned())];
        queues.absorb(deaths, vec![1], vec![1], |_| true);
        assert_eq!(queues.take_deaths(0), ["Skeleton"]);
        assert!(!queues.take_totem(0));
        assert!(!queues.take_switch(0));
        queues.absorb(Vec::new(), Vec::new(), Vec::new(), |_| true);
        assert_eq!(queues.take_deaths(1), ["Zombie"]);
        assert!(queues.take_totem(1));
        assert!(queues.take_switch(1));
        assert!(queues.take_deaths(1).is_empty());
        assert!(!queues.take_totem(1), "a pop is taken once");
    }

    #[test]
    fn queued_entries_of_a_client_without_a_minecraft_player_are_dropped() {
        let mut queues = VoxelQueues::default();
        let deaths = vec![(2, "Creeper".to_owned()), (0, "Fall".to_owned())];
        queues.absorb(deaths, vec![2], vec![2], |client| client != 2);
        assert!(queues.take_deaths(2).is_empty());
        assert!(!queues.take_totem(2));
        assert!(!queues.take_switch(2));
        assert_eq!(queues.take_deaths(0), ["Fall"]);
    }

    #[test]
    fn friends_in_bed_lie_in_the_hosts_level_and_are_let_go_when_they_wake() {
        use minecraft_commands::PlayerId;
        use minecraft_survival::commands::HOST;
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        chat.join(PlayerId(1), "Alex");
        chat.join(PlayerId(2), "Bob");
        let player = |id| {
            Player::new(
                chat.profile(id).unwrap(),
                minecraft_terrain::game_mode::PlayerMode::default(),
                minecraft_survival::survival::Survival::default(),
                minecraftoss_player::inventory::Inventory::default(),
            )
        };
        let sleeping = minecraft_survival::sleep::Sleeping {
            head: (0, 64, 0),
            facing: (0, 1),
            straw: false,
            hurts: 0,
            height: 0.5625,
        };
        let (mut host, mut alex, mut bob) = (player(HOST), player(PlayerId(1)), player(PlayerId(2)));
        host.sleeping = Some(sleeping);
        alex.sleeping = Some(sleeping);
        bob.sleeping = Some(sleeping);
        bob.dimension = Dimension::Nether;
        let host_uuid = host.uuid.clone();
        let mut players = BTreeMap::new();
        for each in [host, alex, bob] {
            players.insert(each.uuid.clone(), each);
        }
        let origins = HashMap::from([
            (Dimension::Overworld, [0.0, 0.0, 0.0]),
            (Dimension::Nether, [0.0, 0.0, -45000.0]),
        ]);
        let mut authority = sim::SimWorld::default();
        let (mut in_bed, mut bodies) = (BTreeSet::new(), Vec::new());
        let shown = (&host_uuid, Some(Dimension::Overworld));
        lay_friends(&players, shown, &mut authority, &origins, &mut in_bed, &mut bodies);
        assert_eq!(in_bed, BTreeSet::from([1, 2]), "the host lies by its own rule");
        assert_eq!(bodies.iter().map(|(client, _)| *client).collect::<Vec<_>>(), [1]);
        for each in players.values_mut() {
            each.sleeping = None;
        }
        bodies.clear();
        lay_friends(&players, shown, &mut authority, &origins, &mut in_bed, &mut bodies);
        assert!(in_bed.is_empty() && bodies.is_empty());
    }

    #[test]
    fn only_a_living_player_in_its_own_band_gets_a_melee_box() {
        use minecraft_survival::commands::HOST;
        use minecraft_terrain::game_mode::PlayerMode;
        use minecraftoss_player::GameMode;
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        let mut player = Player::new(
            chat.profile(HOST).unwrap(),
            PlayerMode::default(),
            minecraft_survival::survival::Survival::default(),
            minecraftoss_player::inventory::Inventory::default(),
        );
        let origin = [0.0; 3];
        let ps = PlayerState {
            origin: sim::voxel::to_map(origin, [3.5, 64.0, 0.5]),
            ..PlayerState::ZERO
        };
        let found = player_box(&player, &ps, origin).expect("a standing player");
        assert_eq!(found.id, 0);
        assert!(!found.invulnerable);
        assert!((found.bounds[0] - 3.2).abs() < 1.0e-4, "{found:?}");
        assert!((found.bounds[1] - 64.0).abs() < 1.0e-4, "{found:?}");
        assert!((found.bounds[5] - 0.8).abs() < 1.0e-4, "{found:?}");
        player.mode = PlayerMode::new(GameMode::Creative);
        assert!(player_box(&player, &ps, origin).unwrap().invulnerable);
        player.mode = PlayerMode::new(GameMode::Spectator);
        assert_eq!(player_box(&player, &ps, origin), None, "a spectator");
        player.mode = PlayerMode::default();
        player.dead = true;
        assert_eq!(player_box(&player, &ps, origin), None, "a dead player");
        player.dead = false;
        player.dimension = Dimension::Nether;
        assert_eq!(player_box(&player, &ps, origin), None, "outside its own band");
    }

    #[test]
    fn a_gun_kill_between_players_is_told_once_with_both_names_and_the_gun() {
        use minecraft_commands::PlayerId;
        use minecraft_survival::commands::HOST;
        use sim::voxel::PvpDeath;
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        chat.join(PlayerId(1), "Alex");
        let player = |id| {
            Player::new(
                chat.profile(id).unwrap(),
                minecraft_terrain::game_mode::PlayerMode::default(),
                minecraft_survival::survival::Survival::default(),
                minecraftoss_player::inventory::Inventory::default(),
            )
        };
        let (host, friend) = (player(HOST), player(PlayerId(1)));
        let host_uuid = host.uuid.clone();
        let mut players =
            BTreeMap::from([(host.uuid.clone(), host), (friend.uuid.clone(), friend)]);
        let rules = GameRules::default();
        let gun = |weapon: u32| (weapon == 7).then(|| "M4A1".to_owned());
        let shot = |victim, attacker, weapon| PvpDeath {
            victim,
            attacker,
            weapon,
        };
        let deaths = vec![shot(1, 5, 7), shot(5, 1, 0), shot(1, 5, 7), shot(9, 5, 7)];
        let told = tell_pvp_deaths(deaths, &mut players, (&rules, &chat), (&host_uuid, 5), gun);
        let lang = |key: &str| match key {
            "death.attack.player" => Some("%1$s was slain by %2$s".to_owned()),
            "death.attack.player.item" => Some("%1$s was slain by %2$s using %3$s".to_owned()),
            _ => None,
        };
        let lines: Vec<String> = told
            .iter()
            .filter_map(Clientbound::chat_text)
            .map(|text| text.resolve(&lang).text)
            .collect();
        assert_eq!(
            lines,
            [
                "Alex was slain by Steve using [M4A1]",
                "Steve was slain by Alex"
            ]
        );
        for player in players.values_mut() {
            player.survival.respawn();
        }
        let mut quiet = GameRules::default();
        quiet.set("show_death_messages", "false").unwrap();
        let told = tell_pvp_deaths(
            vec![shot(5, 1, 7)],
            &mut players,
            (&quiet, &chat),
            (&host_uuid, 5),
            gun,
        );
        assert!(told.is_empty(), "death messages are off");
    }
}
