use std::collections::{BTreeMap, BTreeSet, HashMap};

use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use minecraft_commands::Clientbound;
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_survival::entities::Entities;
use minecraft_survival::level::{Level, Terrain};
use minecraft_survival::player::Player;
use minecraft_survival::server::Globals;
use minecraft_survival::switch::Switch;
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::BlockPos;
use minecraftoss_core::BlockStateId;
use playerstate_iw4::PlayerState;

use crate::minecraft_sounds::Sounds;
use crate::minecraft_spawn_gate::SpawnGate;
use crate::minecraft_world::{
    CRAWLING_BELOW_VIEW_HEIGHT, RECENT_PLACE_SECONDS, TICK_SECONDS, ability_bits, minecraft_yaw,
    place_player, sky_bits, write_blocks,
};

type Shapes = HashMap<BlockStateId, u16>;
type ShapeIds = HashMap<(Vec<[u32; 6]>, u16), u16>;
type Hits = Vec<(
    f32,
    Option<[f64; 3]>,
    Option<[f64; 2]>,
    minecraft_survival::death::DamageCause,
)>;

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
        self.deaths.remove(&client).unwrap_or_default()
    }

    pub(crate) fn take_totem(&mut self, client: u32) -> bool {
        self.totems.remove(&client)
    }

    pub(crate) fn take_switch(&mut self, client: u32) -> bool {
        self.switches.remove(&client)
    }
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
    gate: &mut SpawnGate,
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
                &globals.spawn,
                level,
            ));
        }
        crate::minecraft_portal::arriving(player, sounds.as_mut(), packs);
        let ps = ps.filter(|ps| ps.pm_type == 0);
        if let Some(ps) = ps.filter(|_| player.arrival.is_some()) {
            place_player(player, authority, client, ps, origin, gate, now_ms);
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
            place_player(player, authority, client, ps, origin, gate, now_ms);
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

pub(crate) struct Moved {
    pub(crate) on_ground: bool,
    pub(crate) wet: bool,
    pub(crate) hand_ticks: u32,
    pub(crate) holding: bool,
    pub(crate) holding_item: bool,
    pub(crate) empty_hand: bool,
    pub(crate) cooldowns: Vec<(String, f32)>,
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
    alive: bool,
    dt: f64,
    sounds: &mut Option<Sounds>,
    packs: &PackStack,
) -> Moved {
    let Player {
        survival,
        mode: player_mode,
        inventory,
        selected,
        hand,
        steps,
        ..
    } = player;
    let feet = sim::voxel::to_block(origin, ps.origin);
    let on_ground = alive && ps.ground_entity_num != playerstate_iw4::ENTITYNUM_NONE;
    let (hull_mins, hull_maxs) = sim::voxel::player_hull(ps);
    let water = sim::voxel::water(ps.origin, hull_mins, hull_maxs, ps.view_height_current);
    if water.wet() {
        steps.air_peak = None;
    }
    if let Some(last) = steps.last.filter(|_| alive && !player_mode.noclip()) {
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
        if (on_ground || climbing) && horizontal < 2.0 {
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
        if on_ground {
            if let Some(peak) = steps.air_peak.take().filter(|_| !caught) {
                let fall = survival.impulse_fall(peak - feet[1], feet[1]);
                let landed_on = under.as_ref().map(|(_, b)| b.id.key());
                let crouching = ps.pm_flags
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
    if let Some(ticks) = hand.swing.as_mut() {
        *ticks += (dt_hand * 20.0) as f32;
        if *ticks >= crate::minecraft_hand::SWING_TICKS {
            hand.swing = None;
        }
    }
    let holding = entities.as_ref().is_some_and(|_| {
        inventory.slots[*selected]
            .as_ref()
            .is_none_or(|s| minecraft_survival::guns::weapon_of(s).is_none())
    });
    let holding_item = alive && holding && player_mode.interacts();
    let empty_hand = holding_item
        && entities
            .as_ref()
            .is_some_and(|_| inventory.slots[*selected].is_none());
    hand.clock += dt_hand;
    let hand_ticks = (hand.clock / TICK_SECONDS) as u32;
    hand.clock -= f64::from(hand_ticks) * TICK_SECONDS;
    hand.cooldowns.tick(hand_ticks);
    let cooldowns = hand.cooldowns.shown((hand.clock / TICK_SECONDS) as f32);
    hand.miss_time = hand.miss_time.saturating_sub(hand_ticks);
    hand.attack_ticks = hand.attack_ticks.saturating_add(hand_ticks);
    let main_hand = entities
        .as_ref()
        .and_then(|_| inventory.slots[*selected].as_ref().map(|s| s.id.clone()));
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
                    shapes,
                    shape_ids,
                    entities,
                    frozen
                        .into_iter()
                        .map(|(pos, block)| (pos, scene_block(&block)))
                        .collect(),
                );
            }
            if entities.is_some() {
                hold_off_death(
                    survival,
                    inventory,
                    *selected,
                    sounds.as_mut(),
                    packs,
                    false,
                );
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
        hand_ticks,
        holding,
        holding_item,
        empty_hand,
        cooldowns,
    }
}

pub(crate) struct HostHand<'a> {
    pub(crate) ui: &'a mut frame::MinecraftUi,
    pub(crate) inventory_ui: &'a mut crate::minecraft_inventory::InventoryUi,
    pub(crate) hud_input: Option<&'a frame::HudInputView>,
    pub(crate) buttons: &'a ButtonInput<MouseButton>,
    pub(crate) pad: Option<&'a bevy::input::gamepad::Gamepad>,
    pub(crate) mining: &'a mut crate::minecraft_mining::Mining,
    pub(crate) atlas: &'a minecraft_terrain::mesh::Atlas,
    pub(crate) light: Option<&'a SkyLight>,
    pub(crate) now: f64,
}

pub(crate) struct Acted {
    pub(crate) open_table: Option<BlockPos>,
    pub(crate) open_block: Option<minecraft_terrain::containers::Opened>,
    pub(crate) open_bench: Option<(BlockPos, minecraft_terrain::stations::Workshop)>,
    pub(crate) hand_mined: Vec<BlockPos>,
    pub(crate) creative_broken: Vec<BlockPos>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn act(
    host: HostHand<'_>,
    player: &mut Player,
    globals: &Globals,
    level: &mut Level,
    (shapes, shape_ids): (&mut Shapes, &mut ShapeIds),
    authority: &mut sim::SimWorld,
    client: sim::ClientId,
    origin: [f64; 3],
    ps: &PlayerState,
    alive: bool,
    moved: &Moved,
    all_events: &mut Vec<sim::voxel::VoxelEvent>,
    sounds: &mut Option<Sounds>,
    packs: &PackStack,
) -> Acted {
    let HostHand {
        ui,
        inventory_ui,
        hud_input,
        buttons,
        pad,
        mining,
        atlas,
        light,
        now,
    } = host;
    let Player {
        survival,
        mode: player_mode,
        inventory,
        selected,
        hand,
        steps,
        respawn,
        ..
    } = player;
    let Level {
        entities,
        containers,
        terrain,
        dimension,
        ..
    } = level;
    let feet = sim::voxel::to_block(origin, ps.origin);
    let mc_yaw = minecraft_yaw(ps.viewangles[1]);
    let (on_ground, wet, hand_ticks, holding) =
        (moved.on_ground, moved.wet, moved.hand_ticks, moved.holding);
    let mut hand_mined = Vec::new();
    let mut creative_broken = Vec::new();
    let game_mode = *player_mode;
    let menu_captured =
        ui.chat.open || hud_input.is_some_and(|input| input.console_open || input.script_menu_open);
    let use_held =
        buttons.pressed(MouseButton::Right) || pad_trigger(pad, GamepadButton::LeftTrigger2, false);
    let offhand_key = alive && !holding && ui.offhand_key == Some(true);
    let use_live = !ui.inventory_open
        && !menu_captured
        && if hand.use_by_key {
            offhand_key
        } else {
            use_held && ui.holding_item
        };
    if survival.eating().is_some() && !use_live {
        survival.stop_eating();
    }
    let shield_held = entities.as_ref().is_some_and(|_| {
        hand.blocking
            .is_some_and(|blocking| blocking.holds_shield(inventory, *selected))
    });
    if !use_live || !shield_held {
        hand.blocking = None;
    }
    let (charging, released) =
        minecraft_survival::trident::released(hand.charging, use_live, alive, inventory, *selected);
    hand.charging = charging;
    if let (Some(released), Some(entities)) = (released, entities.as_mut()) {
        let wet = wet || entities.weather.player_in_rain;
        let eye = sim::voxel::to_block(
            origin,
            [
                ps.origin[0],
                ps.origin[1],
                ps.origin[2] + ps.view_height_current,
            ],
        );
        let motion = glam::DVec3::new(
            f64::from(ps.velocity[0]),
            f64::from(ps.velocity[2]),
            -f64::from(ps.velocity[1]),
        ) / (f64::from(sim::voxel::BLOCK) * 20.0);
        let release = entities.release_trident(
            inventory,
            released,
            !game_mode.consumes_items(),
            wet,
            glam::DVec3::from_array(eye),
            mc_yaw,
            ps.viewangles[0],
            motion,
            on_ground,
        );
        let thrown = match &release {
            minecraft_terrain::trident::Release::Throw(stack) => {
                Some(minecraft_terrain::trident::throw_sound(stack))
            }
            minecraft_terrain::trident::Release::Riptide { strength, sound } => {
                let push =
                    minecraft_terrain::trident::riptide_push(mc_yaw, ps.viewangles[0], *strength);
                let launch = minecraft_survival::impulse::riptide_velocity(
                    push,
                    on_ground,
                    f64::from(ps.gravity) / f64::from(sim::voxel::BLOCK),
                ) * f64::from(sim::voxel::BLOCK);
                authority
                    .push_velocity(client, [launch.x as f32, -launch.z as f32, launch.y as f32]);
                Some(*sound)
            }
            minecraft_terrain::trident::Release::Nothing => None,
        };
        if let (Some(sound), Some(sounds)) = (thrown, sounds.as_mut()) {
            sounds.play(packs, sound, None, 1.0, 1.0);
        }
    }
    let mut open_table = None;
    let mut open_block = None;
    let mut open_bench = None;
    if let Some(entities) = entities.as_mut()
        && (ui.holding_item
            || offhand_key
            || survival.eating().is_some()
            || hand.blocking.is_some()
            || hand.charging.is_some())
        && !ui.inventory_open
        && !menu_captured
        && game_mode.interacts()
    {
        let mut player = minecraftoss_player::Player::new(glam::DVec3::from_array(feet));
        player.yaw = f64::from(mc_yaw);
        player.pitch = f64::from(ps.viewangles[0]);
        player.selected = *selected;
        player.crouching = ps.pm_flags
            & (playerstate_iw4::pm_flags::CROUCH | playerstate_iw4::pm_flags::PRONE)
            != 0;
        player.swimming = ps.view_height_current < CRAWLING_BELOW_VIEW_HEIGHT;
        let eye_block = player.eye();
        let look = {
            let (yaw, pitch) = (
                f64::from(mc_yaw).to_radians(),
                f64::from(ps.viewangles[0]).to_radians(),
            );
            glam::DVec3::new(
                -yaw.sin() * pitch.cos(),
                -pitch.sin(),
                yaw.cos() * pitch.cos(),
            )
        };
        let attack_held = buttons.pressed(MouseButton::Left)
            || pad_trigger(pad, GamepadButton::RightTrigger2, false);
        if !attack_held {
            hand.miss_time = 0;
        }
        let mw2_offhand = (weapon_iw4::WeaponState::OffhandInit as i32
            ..=weapon_iw4::WeaponState::OffhandEnd as i32)
            .contains(&ps.weaponstate_primary);
        let busy = survival.eating().is_some()
            || hand.blocking.is_some()
            || hand.charging.is_some()
            || hand.miss_time > 0
            || mw2_offhand;
        let mut creative_press = false;
        let mut punched = false;
        if ui.holding_item
            && (buttons.just_pressed(MouseButton::Left)
                || pad_trigger(pad, GamepadButton::RightTrigger2, true))
            && !busy
        {
            hand.swing = Some(0.0);
            hand.swing_offhand = false;
            let held = inventory.slots[*selected].clone();
            let (damage, speed) = entities.recipes.attack_attributes(held.as_ref());
            let damage = (damage + survival.status.attack_damage_modifier()).max(0.0);
            let strength = minecraft_survival::survival::attack_strength(hand.attack_ticks, speed);
            let sprinting = ps.pm_flags & playerstate_iw4::pm_flags::SPRINTING != 0;
            let falling = steps.air_peak.is_some_and(|peak| peak > feet[1]);
            let blow = minecraft_survival::entities::Blow {
                sprinting,
                can_critical: falling && !on_ground && !wet && !sprinting,
                can_sweep: on_ground
                    && !sprinting
                    && held.as_ref().is_some_and(|s| s.id.ends_with("_sword")),
                reach: game_mode.entity_reach(),
            };
            if entities.punch(
                inventory, *selected, eye_block, look, mc_yaw, damage, strength, blow,
            ) {
                punched = true;
                hand.attack_ticks = 0;
                survival.exhaust(minecraft_survival::survival::ATTACK_EXHAUSTION);
            } else if player
                .target(
                    &minecraft_terrain::doors::Outlined(&terrain.scene),
                    game_mode.block_reach(),
                )
                .is_none()
            {
                hand.attack_ticks = 0;
                hand.miss_time = if game_mode.instant_break() { 0 } else { 10 };
            } else {
                creative_press = true;
            }
        }
        for _ in 0..hand_ticks {
            let Some((drinking, crumbs, item)) = survival.eating().map(|eating| {
                (
                    eating.info.animation.eq_ignore_ascii_case("drink"),
                    eating.info.particles,
                    eating.stack.id.clone(),
                )
            }) else {
                break;
            };
            let roll = entities.random_float();
            let bite = survival.chew(inventory, *selected, || roll);
            let (sound, finished) = match bite {
                minecraft_survival::survival::Bite::Nothing => continue,
                minecraft_survival::survival::Bite::Chew(sound) => (sound, None),
                minecraft_survival::survival::Bite::Done {
                    sound,
                    overflow,
                    fed,
                } => (sound, Some((overflow, fed))),
            };
            if crumbs
                && let Some(texture) = minecraft_terrain::pack::ResourceId::parse(&item)
                    .ok()
                    .and_then(|id| {
                        minecraft_terrain::model::item_particle_texture(packs, &id)
                            .ok()
                            .flatten()
                    })
                    .filter(|texture| atlas.contains(texture))
            {
                let count = if finished.is_some() { 16 } else { 5 };
                mining.eat_burst(
                    packs,
                    eye_block.as_vec3(),
                    mc_yaw,
                    ps.viewangles[0],
                    count,
                    &texture,
                );
            }
            let (r1, r2) = (entities.random_float(), entities.random_float());
            if let Some(sounds) = sounds.as_mut() {
                let (volume, pitch) = if drinking {
                    (0.5, r1 * 0.1 + 0.9)
                } else {
                    (if r1 < 0.5 { 0.5 } else { 1.0 }, (r1 - r2) * 0.2 + 1.0)
                };
                sounds.play(packs, &sound, None, volume, pitch);
            }
            if let Some((overflow, fed)) = finished {
                if let Some(sounds) = sounds.as_mut().filter(|_| fed) {
                    sounds.play(
                        packs,
                        "minecraft:entity.player.burp",
                        None,
                        0.5,
                        r2 * 0.1 + 0.9,
                    );
                }
                if let Some(stack) = overflow {
                    entities
                        .world_items
                        .toss(stack, eye_block, mc_yaw, ps.viewangles[0]);
                }
            }
        }
        if ui.holding_item
            && attack_held
            && !busy
            && game_mode.may_build()
            && game_mode.instant_break()
        {
            let held = inventory.slots[*selected]
                .as_ref()
                .map(|stack| stack.id.as_str());
            if let Some(hit) = player.target(
                &minecraft_terrain::doors::Outlined(&terrain.scene),
                game_mode.block_reach(),
            ) && !punched
                && entities
                    .mob_distance(eye_block, look, game_mode.entity_reach())
                    .is_none_or(|distance| distance > hit.distance)
                && minecraft_terrain::game_mode::can_destroy_in_creative(held)
                && minecraft_terrain::game_mode::creative_breaks(
                    &mut hand.creative_delay,
                    creative_press,
                    attack_held,
                    hand_ticks,
                ) > 0
            {
                creative_broken.push(hit.pos);
                hand_mined.push(hit.pos);
                if hand
                    .swing
                    .is_none_or(|t| t >= crate::minecraft_hand::SWING_TICKS * 0.5)
                {
                    hand.swing = Some(0.0);
                    hand.swing_offhand = false;
                }
            }
        } else if ui.holding_item && attack_held && !busy && game_mode.may_build() {
            for _ in 0..hand_ticks {
                if let Some(hit) =
                    player.target(&minecraft_terrain::doors::Outlined(&terrain.scene), 4.5)
                {
                    all_events.push(sim::voxel::VoxelEvent::Shot {
                        block: [hit.pos.0, hit.pos.1, hit.pos.2],
                        damage: 160.0 / 30.0,
                    });
                    hand_mined.push(hit.pos);
                    if hand
                        .swing
                        .is_none_or(|t| t >= crate::minecraft_hand::SWING_TICKS * 0.5)
                    {
                        hand.swing = Some(0.0);
                        hand.swing_offhand = false;
                    }
                }
            }
        }
        if let Some(blocking) = hand.blocking.as_mut() {
            blocking.ticks = blocking.ticks.saturating_add(hand_ticks);
        }
        if let Some(charging) = hand.charging.as_mut() {
            charging.ticks = charging.ticks.saturating_add(hand_ticks);
        }
        hand.place_delay = hand.place_delay.saturating_sub(hand_ticks);
        let idle = survival.eating().is_none()
            && hand.blocking.is_none()
            && hand.charging.is_none()
            && !mw2_offhand;
        let (use_pressed, use_down) = if ui.holding_item {
            (
                buttons.just_pressed(MouseButton::Right)
                    || pad_trigger(pad, GamepadButton::LeftTrigger2, true),
                use_held,
            )
        } else {
            (offhand_key && !hand.offhand_key_was, offhand_key)
        };
        let use_pressed = idle && use_pressed;
        let place = use_pressed || (idle && use_down && hand.place_delay == 0);
        let hands = if place {
            hand.place_delay = 4;
            hand.use_by_key = !ui.holding_item;
            minecraft_survival::offhand::hand_slots(*selected, !ui.holding_item)
        } else {
            Vec::new()
        };
        let has_item = inventory.slots[*selected].is_some()
            || inventory.slots[minecraft_survival::offhand::OFFHAND].is_some();
        let used = minecraft_survival::offhand::use_hands(&hands, |slot| {
            use minecraft_survival::offhand::HandUse;
            let main = slot != minecraft_survival::offhand::OFFHAND;
            player.selected = slot;
            let held = inventory.slots[slot].clone();
            let table = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|&pos| {
                    minecraft_terrain::scene::Scene::block(&terrain.scene, pos)
                        .is_some_and(minecraft_terrain::crafting::is_crafting_table)
                })
                .filter(|_| {
                    main && minecraft_terrain::crafting::opens_table(player.crouching, has_item)
                });
            let station = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|_| {
                    main && table.is_none()
                        && minecraft_terrain::crafting::opens_table(player.crouching, has_item)
                })
                .and_then(|pos| {
                    let block = minecraft_terrain::scene::Scene::block(&terrain.scene, pos)?;
                    let partner = minecraft_terrain::containers::chest_partner(pos, block)
                        .and_then(|at| minecraft_terrain::scene::Scene::block(&terrain.scene, at));
                    minecraft_terrain::containers::Opened::new(pos, block, partner)
                });
            let workshop = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|_| {
                    main && table.is_none()
                        && station.is_none()
                        && minecraft_terrain::crafting::opens_table(player.crouching, has_item)
                })
                .and_then(|pos| {
                    minecraft_terrain::scene::Scene::block(&terrain.scene, pos)
                        .and_then(minecraft_terrain::stations::Workshop::of)
                        .map(|kind| (pos, kind))
                });
            let campfire = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|&pos| {
                    minecraft_terrain::scene::Scene::block(&terrain.scene, pos)
                        .is_some_and(minecraft_terrain::campfire::is_campfire)
                })
                .filter(|_| !(player.crouching && has_item));
            let placeable = held.as_ref().is_some_and(|stack| {
                let id = if stack.id == "minecraft:redstone" {
                    "minecraft:redstone_wire"
                } else {
                    stack.id.as_str()
                };
                minecraft_terrain::pack::ResourceId::parse(id).is_ok_and(|id| {
                    terrain
                        .stream
                        .states
                        .state_of(&minecraft_terrain::scene::Block {
                            id,
                            properties: Default::default(),
                        })
                        .is_some()
                })
            });
            let used = if main
                && table.is_none()
                && station.is_none()
                && workshop.is_none()
                && !(player.crouching && has_item)
            {
                let facing = minecraft_terrain::doors::facing(f64::from(mc_yaw));
                player
                    .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                    .map(|hit| hit.pos)
                    .filter(|&pos| {
                        !minecraftoss_player::World::block(&terrain.scene, pos).is_some_and(
                            |block| {
                                minecraft_terrain::berries::bone_meal_passes(&block, held.as_ref())
                            },
                        )
                    })
                    .filter(|&pos| entities.use_block(&terrain.scene, pos, facing))
            } else {
                None
            };
            if let Some(pos) = table {
                if use_pressed {
                    open_table = Some(pos);
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(bench) = workshop {
                if use_pressed {
                    open_bench = Some(bench);
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(opened) = station {
                if use_pressed {
                    let conductor = |(x, y, z): (i32, i32, i32)| {
                        minecraft_terrain::scene::Scene::block(&terrain.scene, (x, y + 1, z))
                            .and_then(|b| terrain.stream.states.state_of(b))
                            .is_some_and(|state| {
                                globals
                                    .registries
                                    .blocks
                                    .is(state, minecraftoss_core::block::flags::REDSTONE_CONDUCTOR)
                            })
                    };
                    let blocked = match opened.station {
                        minecraft_terrain::containers::Station::Chest
                        | minecraft_terrain::containers::Station::Ender => {
                            opened.cells.iter().any(|&pos| conductor(pos))
                        }
                        minecraft_terrain::containers::Station::Shulker => {
                            let facing = minecraft_terrain::scene::Scene::block(
                                &terrain.scene,
                                opened.cells[0],
                            )
                            .and_then(|b| b.properties.get("facing").cloned())
                            .unwrap_or_else(|| "up".to_owned());
                            entities.lids.lid_closed(opened.cells[0])
                                && minecraft_terrain::containers::lid_obstructed(
                                    opened.cells[0],
                                    &facing,
                                    |at| {
                                        minecraftoss_player::World::collision_boxes(
                                            &terrain.scene,
                                            at,
                                        )
                                    },
                                )
                        }
                        _ => false,
                    };
                    if !blocked {
                        open_block = Some(opened);
                    }
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(pos) = campfire
                && let Some(stack) = held.as_ref()
                && let Some(placed) = containers.place_food(pos, stack, &entities.recipes)
            {
                if placed {
                    let slot = &mut inventory.slots[slot];
                    if let Some(left) = slot.as_mut().filter(|_| game_mode.consumes_items()) {
                        left.count -= 1;
                        if left.count == 0 {
                            *slot = None;
                        }
                    }
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(respawn_use) = (!(player.crouching && has_item))
                .then(|| {
                    crate::minecraft_respawn::use_block(
                        &player,
                        terrain,
                        &globals.registries,
                        *dimension,
                        crate::minecraft_respawn::Hand {
                            main,
                            held: held.as_ref().map(|stack| stack.id.as_str()),
                            offhand: inventory.slots[minecraft_survival::offhand::OFFHAND]
                                .as_ref()
                                .map(|stack| stack.id.as_str()),
                        },
                    )
                })
                .flatten()
            {
                write_blocks(
                    terrain,
                    &globals.registries,
                    shapes,
                    shape_ids,
                    entities,
                    respawn_use.writes.clone(),
                );
                crate::minecraft_respawn::finish(
                    respawn_use,
                    entities,
                    respawn,
                    inventory,
                    slot,
                    game_mode.consumes_items(),
                    sounds.as_mut(),
                    packs,
                    origin,
                    ui,
                    inventory_ui,
                );
                hand.swing = Some(0.0);
                HandUse::Consumed
            } else if let Some(pos) = used {
                hand.swing = Some(0.0);
                if let Some(block) = minecraftoss_player::World::block(&terrain.scene, pos)
                    && minecraft_terrain::doors::note_plays(
                        &block,
                        minecraftoss_player::World::block(
                            &terrain.scene,
                            (pos.0, pos.1 + 1, pos.2),
                        )
                        .as_ref(),
                    )
                    && let Some(sound) = minecraft_terrain::doors::toggle_sound(&block)
                    && let Some(sounds) = sounds.as_mut()
                {
                    let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                    let pitch = sound.pitch + sounds.random() * sound.pitch_spread;
                    sounds.play(
                        packs,
                        &sound.event,
                        Some(Vec3::from_array(sim::voxel::to_map(origin, centre))),
                        sound.volume,
                        pitch,
                    );
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && let Some(outcome) = minecraft_terrain::bottle::bottle_use(
                    &player,
                    &terrain.scene,
                    stack,
                    !entities.awaits_item_use()
                        && entities.dragon_breath_near(
                            player.pos,
                            if player.crouching { 1.5 } else { 1.8 },
                        ),
                )
            {
                use minecraft_terrain::bottle::{self, BottleUse};
                hand.swing = Some(0.0);
                let (result, heard) = match outcome {
                    BottleUse::Block {
                        pos,
                        block,
                        result,
                        sounds: events,
                    } => {
                        write_blocks(
                            terrain,
                            &globals.registries,
                            shapes,
                            shape_ids,
                            entities,
                            vec![(pos, block.and_then(|b| scene_block(&b)))],
                        );
                        let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                        (
                            Some(result),
                            events
                                .into_iter()
                                .map(|event| (event, Some(centre)))
                                .collect(),
                        )
                    }
                    BottleUse::Fill { result } => (Some(result), vec![(bottle::FILL_SOUND, None)]),
                    BottleUse::Breath(outcome) => {
                        let sent = minecraft_survival::entities::SentItemUse {
                            slot,
                            held: stack.id.clone(),
                            eye: eye_block,
                            yaw: mc_yaw,
                            pitch: ps.viewangles[0],
                            outcome,
                        };
                        entities.bottle_breath(
                            player.pos,
                            if player.crouching { 1.5 } else { 1.8 },
                            sent,
                        );
                        (None, Vec::new())
                    }
                    BottleUse::Drink => {
                        hand.swing = None;
                        survival.start_using(bottle::drink(slot, stack));
                        (None, Vec::new())
                    }
                };
                if let Some(result) = result
                    && let Some(rest) = bottle::filled_result(inventory, slot, result)
                {
                    entities
                        .world_items
                        .toss(rest, eye_block, mc_yaw, ps.viewangles[0]);
                }
                if let Some(sounds) = sounds.as_mut() {
                    for (event, at) in heard {
                        sounds.play(
                            packs,
                            event,
                            at.map(|centre| Vec3::from_array(sim::voxel::to_map(origin, centre))),
                            1.0,
                            1.0,
                        );
                    }
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && !entities.awaits_item_use()
                && game_mode.may_build()
                && let Some(outcome) = {
                    let catalog = minecraft_terrain::bucket::Catalog {
                        states: &terrain.stream.states,
                        water_evaporates: terrain.environment.water_evaporates(),
                    };
                    let outlined = minecraft_terrain::doors::Outlined(&terrain.scene);
                    minecraft_terrain::bucket::bucket_use(
                        &player,
                        &outlined,
                        &catalog,
                        stack,
                        || entities.random_float(),
                    )
                    .or_else(|| {
                        minecraft_terrain::flint::flint_use(
                            &player,
                            &outlined,
                            &catalog,
                            stack,
                            || entities.random_float(),
                        )
                    })
                    .or_else(|| {
                        minecraft_terrain::flint::fire_charge_use(
                            &player,
                            &outlined,
                            &catalog,
                            stack,
                            || entities.random_float(),
                        )
                    })
                    .or_else(|| {
                        minecraft_terrain::berries::bone_meal_use(&player, &outlined, stack)
                    })
                    .or_else(|| {
                        minecraft_terrain::campfire::douse_use(
                            &player,
                            &outlined,
                            entities.recipes.item_in_tag(
                                minecraft_terrain::campfire::DOUSES_CAMPFIRES,
                                &stack.id,
                            ),
                            || entities.random_float(),
                        )
                    })
                    .or_else(|| {
                        globals.data.transforms.as_ref()?.transform_use(
                            |tag| entities.recipes.item_in_tag(tag, &stack.id),
                            &player,
                            &outlined,
                            &*terrain.stream.states,
                            main && !player.crouching
                                && inventory.slots[minecraft_survival::offhand::OFFHAND]
                                    .as_ref()
                                    .is_some_and(|shield| {
                                        shield.id == minecraft_survival::offhand::SHIELD
                                    }),
                        )
                    })
                    .or_else(|| minecraft_terrain::ender_eye::eye_use(&player, &outlined, stack))
                    .or_else(|| {
                        minecraft_terrain::end_crystal::crystal_use(&player, &outlined, stack)
                    })
                    .or_else(|| {
                        minecraft_terrain::firework::rocket_use(
                            stack,
                            ps.pm_flags & playerstate_iw4::pm_flags::MC_GLIDING != 0,
                            || entities.random_float(),
                        )
                    })
                }
            {
                if !outcome.consumed
                    || matches!(outcome.change, minecraft_terrain::bucket::Change::Set(..))
                {
                    hand.swing = Some(0.0);
                }
                if let Some((pos, signal)) =
                    minecraft_terrain::campfire::doused(&terrain.scene, &outcome)
                {
                    entities.campfire_douse(pos, signal);
                }
                let change = outcome.change.clone();
                let sent = minecraft_survival::entities::SentItemUse {
                    slot,
                    held: stack.id.clone(),
                    eye: eye_block,
                    yaw: mc_yaw,
                    pitch: ps.viewangles[0],
                    outcome,
                };
                match change {
                    minecraft_terrain::bucket::Change::Set(pos, block) => {
                        write_blocks(
                            terrain,
                            &globals.registries,
                            shapes,
                            shape_ids,
                            entities,
                            vec![(pos, scene_block(&block))],
                        );
                        finish_item_use(
                            entities,
                            inventory,
                            sounds.as_mut(),
                            packs,
                            origin,
                            sent,
                            game_mode.consumes_items(),
                        );
                    }
                    minecraft_terrain::bucket::Change::Server(pos, action) => {
                        entities.item_on_block(pos, action, sent)
                    }
                    minecraft_terrain::bucket::Change::ThrowEye(throw) => {
                        entities.throw_eye(throw, sent)
                    }
                    minecraft_terrain::bucket::Change::Rocket(boost_ticks) => {
                        crate::minecraft_elytra::launch_rocket(
                            authority,
                            client,
                            boost_ticks,
                            &mut hand.swing,
                            || {
                                finish_item_use(
                                    entities,
                                    inventory,
                                    sounds.as_mut(),
                                    packs,
                                    origin,
                                    sent,
                                    game_mode.consumes_items(),
                                )
                            },
                        )
                    }
                    minecraft_terrain::bucket::Change::Breath => entities.bottle_breath(
                        player.pos,
                        if player.crouching { 1.5 } else { 1.8 },
                        sent,
                    ),
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && game_mode.may_build()
                && let Some(egg) = minecraft_terrain::spawn_egg::egg_use(
                    &player,
                    &minecraft_terrain::doors::Outlined(&terrain.scene),
                    &stack.id,
                )
            {
                if let minecraft_terrain::spawn_egg::EggUse::Spawn { kind, position } = egg {
                    entities.hatch_egg(inventory, slot, kind, position, game_mode.consumes_items());
                }
                hand.swing = Some(0.0);
                HandUse::Consumed
            } else if held
                .as_ref()
                .is_some_and(|stack| stack.id == minecraft_terrain::trident::ITEM)
            {
                match minecraft_survival::trident::Charging::start(
                    inventory,
                    slot,
                    wet || entities.weather.player_in_rain,
                ) {
                    Some(charging) => {
                        hand.charging = Some(charging);
                        HandUse::Consumed
                    }
                    None => HandUse::Failed,
                }
            } else if let Some((stack, kind)) = held
                .as_ref()
                .and_then(|s| minecraft_terrain::thrown::Throwable::of(&s.id).map(|k| (s, k)))
            {
                let eye = sim::voxel::to_block(
                    origin,
                    [
                        ps.origin[0],
                        ps.origin[1],
                        ps.origin[2] + ps.view_height_current,
                    ],
                );
                let motion = glam::DVec3::new(
                    f64::from(ps.velocity[0]),
                    f64::from(ps.velocity[2]),
                    -f64::from(ps.velocity[1]),
                ) / (f64::from(sim::voxel::BLOCK) * 20.0);
                if !hand.cooldowns.active(&stack.id)
                    && entities.throw(
                        stack,
                        glam::DVec3::from_array(eye),
                        mc_yaw,
                        ps.viewangles[0],
                        motion,
                        on_ground,
                    )
                {
                    let slot = &mut inventory.slots[slot];
                    if let Some(left) = slot.as_mut().filter(|_| game_mode.consumes_items()) {
                        left.count -= 1;
                        if left.count == 0 {
                            *slot = None;
                        }
                    }
                    if let Some(ticks) = kind.cooldown() {
                        hand.cooldowns.start(&stack.id, ticks);
                    }
                    hand.swing = Some(0.0);
                    let pitch = minecraft_terrain::thrown::throw_pitch(entities.random_float());
                    if let Some(sounds) = sounds.as_mut() {
                        sounds.play(packs, kind.sound(), None, 0.5, pitch);
                    }
                    HandUse::Consumed
                } else {
                    HandUse::Passed
                }
            } else if let Some((pos, writes)) = held
                .as_ref()
                .filter(|_| game_mode.may_build())
                .and_then(|stack| {
                    let outlined = minecraft_terrain::doors::Outlined(&terrain.scene);
                    let mut staged = minecraft_terrain::placing::Staged::new(&outlined);
                    let soil = minecraft_terrain::berries::Ground {
                        states: &terrain.stream.states,
                    };
                    let pos = minecraft_terrain::berries::plant(
                        &player,
                        &mut staged,
                        &stack.id,
                        &soil,
                        || entities.random_float(),
                    )?;
                    Some((pos, staged.into_writes()))
                })
            {
                let slot = &mut inventory.slots[slot];
                if let Some(left) = slot.as_mut().filter(|_| game_mode.consumes_items()) {
                    left.count -= 1;
                    if left.count == 0 {
                        *slot = None;
                    }
                }
                let writes = writes
                    .into_iter()
                    .map(|(at, block)| (at, block.and_then(|b| scene_block(&b))))
                    .collect();
                write_blocks(
                    terrain,
                    &globals.registries,
                    shapes,
                    shape_ids,
                    entities,
                    writes,
                );
                hand.swing = Some(0.0);
                if let Some(block) =
                    minecraft_terrain::scene::Scene::block(&terrain.scene, pos).cloned()
                    && let (Some(sounds), Some(kind)) =
                        (sounds.as_mut(), terrain.scene.sound_type(&block))
                {
                    let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                    sounds.play(
                        packs,
                        &kind.place,
                        Some(Vec3::from_array(sim::voxel::to_map(origin, centre))),
                        (kind.volume + 1.0) / 2.0,
                        kind.pitch * 0.8,
                    );
                }
                HandUse::Consumed
            } else if let Some(stack) = held
                .as_ref()
                .filter(|s| !placeable && minecraftoss_player::food::catalog().contains_key(&s.id))
            {
                if survival.start_eating(slot, stack) {
                    HandUse::Consumed
                } else {
                    HandUse::Passed
                }
            } else if let Some(stack) = held
                .as_ref()
                .filter(|s| s.id == minecraft_terrain::bottle::MILK_BUCKET)
            {
                survival.start_using(minecraft_terrain::bottle::drink_milk(slot, stack));
                HandUse::Consumed
            } else if held
                .as_ref()
                .is_some_and(|stack| stack.id == minecraft_survival::offhand::SHIELD)
            {
                hand.blocking = Some(minecraft_survival::offhand::Blocking::start(slot));
                HandUse::Consumed
            } else if placeable {
                let outlined = minecraft_terrain::doors::Outlined(&terrain.scene);
                let mut staged = minecraft_terrain::placing::Staged::new(&outlined);
                let opening = held
                    .as_ref()
                    .filter(|stack| minecraft_terrain::doors::Opening::of(&stack.id).is_some());
                let sturdy = |below: (i32, i32, i32)| {
                    minecraft_terrain::scene::Scene::block(&terrain.scene, below)
                        .and_then(|b| terrain.stream.states.state_of(b))
                        .is_some_and(|state| {
                            globals.registries.blocks.is_face_sturdy(
                                state,
                                minecraftoss_core::pos::Direction::Up,
                                minecraftoss_core::SupportType::Full,
                            )
                        })
                };
                let placed = match opening {
                    _ if !game_mode.may_build()
                        && (opening.is_some()
                            || held.as_ref().is_some_and(|s| {
                                s.id == minecraft_terrain::scaffolding::SCAFFOLDING
                            })) =>
                    {
                        None
                    }
                    Some(stack) => {
                        minecraft_terrain::doors::place(&player, &mut staged, &stack.id, &sturdy)
                            .inspect(|_| {
                                let slot = &mut inventory.slots[slot];
                                if let Some(left) = slot.as_mut() {
                                    left.count -= 1;
                                    if left.count == 0 {
                                        *slot = None;
                                    }
                                }
                            })
                    }
                    None if held
                        .as_ref()
                        .is_some_and(|s| s.id == minecraft_terrain::scaffolding::SCAFFOLDING) =>
                    {
                        minecraft_terrain::scaffolding::place(
                            &player,
                            &mut staged,
                            inventory,
                            ps.pm_flags
                                & (playerstate_iw4::pm_flags::CROUCH
                                    | playerstate_iw4::pm_flags::PRONE)
                                != 0,
                            &sturdy,
                            &|block: Option<&minecraftoss_player::Block>| {
                                block.is_none_or(|b| {
                                    scene_block(b)
                                        .and_then(|b| terrain.stream.states.state_of(&b))
                                        .is_some_and(|state| {
                                            globals.registries.blocks.is(
                                                state,
                                                minecraftoss_core::block::flags::REPLACEABLE,
                                            )
                                        })
                                })
                            },
                        )
                    }
                    None => player.place_selected(&mut staged, inventory, game_mode.mode),
                };
                let writes: Vec<_> = staged
                    .into_writes()
                    .into_iter()
                    .map(|(pos, block)| {
                        (
                            pos,
                            block.and_then(|b| scene_block(&b)).map(|b| {
                                minecraft_terrain::placing::base_if_covered(&terrain.scene, pos, b)
                            }),
                        )
                    })
                    .collect();
                if std::env::var_os("MINECRAFTOSS_DEBUG_SERVER").is_some()
                    && writes
                        .iter()
                        .any(|(_, b)| b.as_ref().is_some_and(|b| b.id.path.ends_with("chest")))
                {
                    eprintln!(
                        "chest placement: crouching {} pm_flags {:#x} view height {} yaw {} placed {:?} writes {:?}",
                        player.crouching,
                        ps.pm_flags,
                        ps.view_height_current,
                        player.yaw,
                        placed,
                        writes
                    );
                }
                let bodies = obstacles(ps, authority, client, entities);
                let blocked = writes.iter().any(|(pos, block)| {
                    let boxes = block
                        .as_ref()
                        .filter(|b| b.id.key() != minecraft_terrain::scaffolding::SCAFFOLDING)
                        .and_then(|b| terrain.stream.states.state_of(b))
                        .map_or_else(Vec::new, |state| {
                            globals.registries.blocks.collision_boxes(state)
                        });
                    minecraft_terrain::placing::obstructed(*pos, &boxes, &bodies)
                });
                let survives = placed.is_none_or(|pos| {
                    writes
                        .iter()
                        .find(|(at, _)| *at == pos)
                        .and_then(|(_, block)| block.as_ref())
                        .is_none_or(|block| {
                            let view =
                                minecraft_terrain::placing::SurviveView::new(&terrain.scene, light);
                            minecraft_terrain::placing::may_place(
                                &terrain.stream.world_gen().library,
                                &view,
                                pos,
                                block,
                            )
                        })
                });
                if let Some(pos) = placed.filter(|_| !blocked && survives) {
                    hand.recent
                        .retain(|(_, _, at)| now - at < RECENT_PLACE_SECONDS);
                    let written: Vec<_> = writes.iter().map(|(p, _)| *p).collect();
                    write_blocks(
                        terrain,
                        &globals.registries,
                        shapes,
                        shape_ids,
                        entities,
                        writes,
                    );
                    if let Some(stack) = held
                        .as_ref()
                        .filter(|stack| minecraft_terrain::containers::is_shulker_box(&stack.id))
                        && minecraft_terrain::scene::Scene::block(&terrain.scene, pos)
                            .and_then(minecraft_terrain::containers::Station::of)
                            == Some(minecraft_terrain::containers::Station::Shulker)
                    {
                        containers.place_shulker(pos, stack, &entities.recipes);
                    }
                    if !game_mode.consumes_items() {
                        inventory.slots[slot].clone_from(&held);
                    }
                    hand.recent.extend(written.into_iter().filter_map(|p| {
                        minecraftoss_player::World::block(&terrain.scene, p).map(|b| (p, b, now))
                    }));
                    hand.swing = Some(0.0);
                    if let Some(block) =
                        minecraft_terrain::scene::Scene::block(&terrain.scene, pos).cloned()
                        && let (Some(sounds), Some(kind)) =
                            (sounds.as_mut(), terrain.scene.sound_type(&block))
                    {
                        let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                        let at = Vec3::from_array(sim::voxel::to_map(origin, centre));
                        sounds.play(
                            packs,
                            &kind.place,
                            Some(at),
                            (kind.volume + 1.0) / 2.0,
                            kind.pitch * 0.8,
                        );
                    }
                    HandUse::Consumed
                } else {
                    inventory.slots[slot] = held;
                    hand.place_delay = 0;
                    if player
                        .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                        .is_some()
                    {
                        HandUse::Failed
                    } else {
                        HandUse::Passed
                    }
                }
            } else {
                HandUse::Passed
            }
        });
        hand.place_delay = minecraft_survival::offhand::repeat_delay(used, hand.place_delay);
        if let Some((slot, minecraft_survival::offhand::HandUse::Consumed)) = used
            && hand.swing == Some(0.0)
        {
            hand.swing_offhand = slot == minecraft_survival::offhand::OFFHAND;
        }
    }
    hand.offhand_key_was = offhand_key;
    Acted {
        open_table,
        open_block,
        open_bench,
        hand_mined,
        creative_broken,
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
    hits: Hits,
    teleports: &[(PlayerUuid, [f64; 3])],
    sounds: &mut Option<Sounds>,
    packs: &PackStack,
) {
    let Player {
        uuid,
        survival,
        mode: player_mode,
        inventory,
        hand,
        steps,
        ..
    } = player;
    let local_uuid = &*uuid;
    let feet = sim::voxel::to_block(origin, ps.origin);
    let mc_yaw = minecraft_yaw(ps.viewangles[1]);
    let at = |b: [f64; 3]| Vec3::from_array(sim::voxel::to_map(origin, b));
    let armor = minecraft_survival::survival::armor(inventory);
    for (amount, from, toward, cause) in hits.into_iter().filter(|_| !player_mode.invulnerable()) {
        let blocked = hand
            .blocking
            .filter(|blocking| blocking.raised())
            .map_or(0.0, |_| {
                minecraft_survival::offhand::blocked_damage(mc_yaw, toward, amount)
            });
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
        }
        if amount > blocked {
            survival.hurt(amount - blocked, armor, from, cause);
        } else {
            survival.hurt_blocked(cause);
        }
    }
    for used in std::mem::take(&mut entities.item_uses_done) {
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
    for to in teleports
        .iter()
        .filter(|(uuid, _)| alive && uuid == local_uuid)
        .map(|(_, to)| to)
    {
        authority.teleport(client, sim::voxel::to_map(origin, *to));
        steps.air_peak = None;
    }
    for landing in std::mem::take(&mut entities.pearl_landings) {
        if !alive {
            continue;
        }
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
    for push in std::mem::take(&mut entities.player_pushes)
        .into_iter()
        .filter(|_| pushed)
    {
        let launch = minecraft_survival::impulse::launch_velocity(
            push,
            f64::from(ps.gravity) / f64::from(sim::voxel::BLOCK),
        ) * f64::from(sim::voxel::BLOCK);
        authority.push_velocity(client, [launch.x as f32, -launch.z as f32, launch.y as f32]);
        survival.start_impulse(feet[1]);
    }
    for hit in std::mem::take(&mut entities.player_potions)
        .into_iter()
        .filter(|_| alive)
    {
        if let Some(sound) = survival.take_potion(&hit.potion, Some(hit.at.to_array()), || {
            entities.random_float()
        }) && let Some(sounds) = sounds.as_mut()
        {
            sounds.play(packs, sound.event, None, sound.volume, sound.pitch);
        }
    }
}

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
) -> Option<Clientbound> {
    let Player {
        id,
        survival,
        inventory,
        selected,
        ..
    } = player;
    let mut told = None;
    hold_off_death(
        survival,
        inventory,
        *selected,
        sounds.as_mut(),
        packs,
        totem_popped,
    );
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
                    Some(language) => survival.death_line(language, |id| entities.mob_present(id)),
                    None => survival.death_line(
                        &minecraft_terrain::item_icons::language(packs)
                            .unwrap_or_else(|_| HashMap::new()),
                        |id| entities.mob_present(id),
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
            if let Some(death) = &death
                && globals.game_rules.bool("show_death_messages")
            {
                let name = globals.chat.name(*id).unwrap_or("Player");
                told = Some(minecraft_survival::commands::death_message(
                    death.line(name),
                ));
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
    told
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
) {
    if minecraft_survival::player::hold_off_death(survival, inventory, selected, struck)
        && let Some(sounds) = sounds
    {
        sounds.play(
            packs,
            minecraft_survival::offhand::TOTEM_SOUND,
            None,
            1.0,
            1.0,
        );
    }
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
    authority.visit_players(|id, ps| {
        let standing = matches!(
            ps.pm_type,
            0 | playerstate_iw4::PM_TYPE_NORMAL_LINKED | playerstate_iw4::PM_TYPE_LAST_STAND
        );
        if id != local && standing {
            out.extend(hull(ps));
        }
    });
    out.extend(entities.boxes().into_iter().map(|(_, b)| b));
    out
}

fn pad_trigger(
    pad: Option<&bevy::input::gamepad::Gamepad>,
    button: GamepadButton,
    just: bool,
) -> bool {
    pad.is_some_and(|pad| {
        if just {
            pad.just_pressed(button)
        } else {
            pad.pressed(button)
        }
    })
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
}
