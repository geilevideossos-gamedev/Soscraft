use glam::DVec3;
use minecraft_commands::{Clientbound, Text};
use minecraft_terrain::lighting::SkyLight;
use minecraft_terrain::scene::{Block, BlockPos, Scene};
use minecraft_terrain::server::Blast;
use minecraft_terrain::terrain::Dimension;
use minecraftoss_core::{BlockStateId, Registries};
use minecraftoss_player::Face;
use minecraftoss_player::inventory::Inventory;

use crate::entities::{Entities, SentItemUse};
use crate::level::{Level, Terrain};
use crate::player::Player;
use crate::player_nbt::Respawn;
use crate::respawn_block::{
    self, ANCHOR_CHARGE_SOUND, ANCHOR_SET_SPAWN_SOUND, AnchorUse, BAD_RESPAWN_POINT_POWER, Bed,
    BedUse, BlockView, Fuel, GLOWSTONE, OCCUPIED, Outcome, Pos, SPAWN_SET, Sleeper,
};
use crate::server::Globals;

pub const SWING_TICKS: f32 = 6.0;
pub const RECENT_PLACE_SECONDS: f64 = 1.0;
const RESPAWN_BLOCK_REACH: f64 = 5.0;
const BLOCK_RANGE_BUFFER: f64 = 1.0;
const ENTITY_RANGE_BUFFER: f64 = 3.0;
const HIT_SLACK: f64 = 1.0000001;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hands {
    pub attack_held: bool,
    pub attack_pressed: bool,
    pub use_held: bool,
    pub use_pressed: bool,
    pub offhand_key: bool,
    pub captured: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Actor {
    pub alive: bool,
    pub feet: [f64; 3],
    pub eye: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub velocity: DVec3,
    pub gravity: f64,
    pub crouching: bool,
    pub swimming: bool,
    pub sprinting: bool,
    pub gliding: bool,
    pub mw2_offhand: bool,
    pub skating: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Moved {
    pub on_ground: bool,
    pub wet: bool,
    pub eye_in_water: bool,
    pub hand_ticks: u32,
    pub holding: bool,
    pub holding_item: bool,
    pub empty_hand: bool,
    pub cooldowns: Vec<(String, f32)>,
    pub totem_popped: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Sound {
        event: String,
        at: Option<[f64; 3]>,
        volume: f32,
        pitch: f32,
        spread: f32,
    },
    Crumbs {
        item: String,
        eye: DVec3,
        yaw: f32,
        pitch: f32,
        count: u32,
    },
    Dig {
        pos: BlockPos,
        face: Face,
        block: Block,
        sound: Option<(String, f32, f32)>,
    },
    Chat(Clientbound),
    Overlay(String),
}

fn sound(event: &str, at: Option<[f64; 3]>, volume: f32, pitch: f32) -> Effect {
    Effect::Sound {
        event: event.to_owned(),
        at,
        volume,
        pitch,
        spread: 0.0,
    }
}

pub trait Body {
    fn write_blocks(
        &mut self,
        terrain: &mut Terrain,
        entities: &mut Entities,
        writes: Vec<(BlockPos, Option<Block>)>,
    );
    fn obstacles(&self, entities: &Entities) -> Vec<[f64; 6]>;
    fn push(&mut self, velocity: DVec3);
    fn launch_rocket(&mut self, boost_ticks: i32) -> bool;
}

#[derive(Default)]
pub struct Acted {
    pub open_table: Option<BlockPos>,
    pub open_block: Option<minecraft_terrain::containers::Opened>,
    pub open_bench: Option<(BlockPos, minecraft_terrain::stations::Workshop)>,
    pub hand_mined: Vec<BlockPos>,
    pub creative_broken: Vec<BlockPos>,
    pub hand_broken: Vec<BlockPos>,
    pub effects: Vec<Effect>,
    pub bed: Option<Bed>,
    pub open_book: Option<crate::books::OpenBook>,
}

pub fn reaches_block(eye: [f64; 3], pos: BlockPos, range: f64) -> bool {
    let low = [f64::from(pos.0), f64::from(pos.1), f64::from(pos.2)];
    let high = low.map(|value| value + 1.0);
    distance_sq_to_box(eye, low, high) < (range + BLOCK_RANGE_BUFFER).powi(2)
}

pub fn reaches_box(eye: [f64; 3], bounds: [f64; 6], range: f64) -> bool {
    let low = [bounds[0], bounds[1], bounds[2]];
    let high = [bounds[3], bounds[4], bounds[5]];
    distance_sq_to_box(eye, low, high) < (range + ENTITY_RANGE_BUFFER).powi(2)
}

pub fn hit_on_block(pos: BlockPos, hit: [f32; 3]) -> bool {
    let centre = [
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    ];
    (0..3).all(|k| (f64::from(hit[k]) - centre[k]).abs() < HIT_SLACK)
}

pub fn allows(
    target: &minecraft_net::messages::Target,
    eye: [f64; 3],
    mode: &minecraft_terrain::game_mode::PlayerMode,
    mob_box: impl Fn(u64) -> Option<[f64; 6]>,
) -> bool {
    match *target {
        minecraft_net::messages::Target::Miss => true,
        minecraft_net::messages::Target::Block { pos, hit, .. } => {
            let pos = (pos[0], pos[1], pos[2]);
            reaches_block(eye, pos, mode.block_reach()) && hit_on_block(pos, hit)
        }
        minecraft_net::messages::Target::Mob { id } => {
            mob_box(id).is_some_and(|bounds| reaches_box(eye, bounds, mode.entity_reach()))
        }
        minecraft_net::messages::Target::Player { .. } => false,
    }
}

pub fn face_index(face: Face) -> u8 {
    match face {
        Face::Down => 0,
        Face::Up => 1,
        Face::North => 2,
        Face::South => 3,
        Face::West => 4,
        Face::East => 5,
    }
}

fn distance_sq_to_box(point: [f64; 3], low: [f64; 3], high: [f64; 3]) -> f64 {
    (0..3)
        .map(|k| (low[k] - point[k]).max(point[k] - high[k]).max(0.0).powi(2))
        .sum()
}

fn scene_block(block: &minecraftoss_player::Block) -> Option<Block> {
    Some(Block {
        id: minecraft_terrain::pack::ResourceId::parse(&block.id).ok()?,
        properties: block.properties.clone(),
    })
}

fn finish_item_use(
    entities: &mut Entities,
    inventory: &mut Inventory,
    used: SentItemUse,
    consumes: bool,
    effects: &mut Vec<Effect>,
) {
    let heard = crate::level::finish_item_use(entities, inventory, used, consumes);
    effects.extend(
        heard
            .into_iter()
            .map(|heard| sound(heard.event, heard.at, heard.volume, heard.pitch)),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn act(
    player: &mut Player,
    level: &mut Level,
    globals: &Globals,
    body: &mut dyn Body,
    actor: &Actor,
    input: Hands,
    moved: &Moved,
    light: Option<&SkyLight>,
    now: f64,
) -> Acted {
    let mut effects = Vec::new();
    let Player {
        id: player_id,
        survival,
        mode: player_mode,
        inventory,
        selected,
        hand,
        steps,
        respawn,
        riding,
        ..
    } = player;
    let player_id = u64::from(player_id.0);
    let dark_outside = level.dark_outside(globals);
    let Level {
        entities,
        containers,
        terrain,
        dimension,
        ..
    } = level;
    let alive = actor.alive;
    let feet = actor.feet;
    let mc_yaw = actor.yaw;
    let pitch = actor.pitch;
    let (on_ground, wet, hand_ticks, holding) =
        (moved.on_ground, moved.wet, moved.hand_ticks, moved.holding);
    let holding_item = moved.holding_item;
    let mut hand_mined = Vec::new();
    let mut creative_broken = Vec::new();
    let mut hand_broken = Vec::new();
    let mut digging = false;
    let mut bed = None;
    let game_mode = *player_mode;
    let captured = input.captured;
    let skating = actor.skating;
    let use_held = !skating && input.use_held;
    let offhand_key = alive && !holding && !skating && input.offhand_key;
    let use_live = !captured
        && if hand.use_by_key {
            offhand_key
        } else {
            use_held && holding_item
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
    let (charging, released) = crate::trident::released(
        hand.charging,
        use_live,
        alive && !skating,
        inventory,
        *selected,
    );
    hand.charging = charging;
    if let (Some(released), Some(entities)) = (released, entities.as_mut()) {
        let wet = wet || entities.weather.player_in_rain;
        let release = entities.release_trident(
            player_id,
            inventory,
            released,
            !game_mode.consumes_items(),
            wet,
            DVec3::from_array(actor.eye),
            mc_yaw,
            pitch,
            actor.velocity,
            on_ground,
        );
        let thrown = match &release {
            minecraft_terrain::trident::Release::Throw(stack) => {
                Some(minecraft_terrain::trident::throw_sound(stack))
            }
            minecraft_terrain::trident::Release::Riptide { strength, sound } => {
                let push = minecraft_terrain::trident::riptide_push(mc_yaw, pitch, *strength);
                body.push(crate::impulse::riptide_velocity(
                    push,
                    on_ground,
                    actor.gravity,
                ));
                Some(*sound)
            }
            minecraft_terrain::trident::Release::Nothing => None,
        };
        if let Some(event) = thrown {
            effects.push(sound(event, None, 1.0, 1.0));
        }
    }
    let (scoping, stopped) = crate::spyglass::step(
        hand.scoping,
        use_live,
        alive && !skating,
        inventory,
        *selected,
    );
    hand.scoping = scoping;
    if stopped {
        effects.push(sound(crate::spyglass::STOP_SOUND, None, 1.0, 1.0));
    }
    let mut open_table = None;
    let mut open_block = None;
    let mut open_bench = None;
    let mut open_book = None;
    if let Some(entities) = entities.as_mut()
        && (holding_item
            || offhand_key
            || survival.eating().is_some()
            || hand.blocking.is_some()
            || hand.charging.is_some())
        && !captured
        && game_mode.interacts()
    {
        let mut player = minecraftoss_player::Player::new(DVec3::from_array(feet));
        player.yaw = f64::from(mc_yaw);
        player.pitch = f64::from(pitch);
        player.selected = *selected;
        player.crouching = actor.crouching;
        player.swimming = actor.swimming;
        let eye_block = player.eye();
        let look = {
            let (yaw, pitch) = (
                f64::from(mc_yaw).to_radians(),
                f64::from(pitch).to_radians(),
            );
            DVec3::new(
                -yaw.sin() * pitch.cos(),
                -pitch.sin(),
                yaw.cos() * pitch.cos(),
            )
        };
        let attack_held = !skating && input.attack_held;
        if !attack_held {
            hand.miss_time = 0;
        }
        let mw2_offhand = actor.mw2_offhand;
        let busy = survival.eating().is_some()
            || hand.blocking.is_some()
            || hand.charging.is_some()
            || hand.scoping.is_some()
            || hand.miss_time > 0
            || mw2_offhand;
        let mut block_press = false;
        let mut punched = false;
        if holding_item && !skating && input.attack_pressed && !busy {
            hand.swing = Some(0.0);
            hand.swing_offhand = false;
            let held = inventory.slots[*selected].clone();
            let (damage, speed) = entities.recipes.attack_attributes(held.as_ref());
            let speed = (speed * survival.status.attack_speed_multiplier()).clamp(0.0, 1024.0);
            let damage = (damage + survival.status.attack_damage_modifier()).max(0.0);
            let strength = crate::survival::attack_strength(hand.attack_ticks, speed);
            let sprinting = actor.sprinting;
            let falling = steps.air_peak.is_some_and(|peak| peak > feet[1]);
            let blow = crate::entities::Blow {
                sprinting,
                can_critical: falling && !on_ground && !wet && !sprinting,
                can_sweep: on_ground
                    && !sprinting
                    && held.as_ref().is_some_and(|s| s.id.ends_with("_sword")),
                reach: game_mode.entity_reach(),
                block: player
                    .target(
                        &minecraft_terrain::doors::Outlined(&terrain.scene),
                        game_mode.block_reach(),
                    )
                    .map(|hit| hit.distance),
                creative: !game_mode.consumes_items(),
            };
            if entities.punch(
                player_id, inventory, *selected, eye_block, look, mc_yaw, damage, strength, blow,
            ) {
                punched = true;
                hand.attack_ticks = 0;
                survival.exhaust(crate::survival::ATTACK_EXHAUSTION);
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
                block_press = true;
                hand.dig.press();
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
            let (chewed, finished) = match bite {
                crate::survival::Bite::Nothing => continue,
                crate::survival::Bite::Chew(chewed) => (chewed, None),
                crate::survival::Bite::Done {
                    sound,
                    overflow,
                    fed,
                } => (sound, Some((overflow, fed))),
            };
            if crumbs {
                let count = if finished.is_some() { 16 } else { 5 };
                effects.push(Effect::Crumbs {
                    item,
                    eye: eye_block,
                    yaw: mc_yaw,
                    pitch,
                    count,
                });
            }
            let (r1, r2) = (entities.random_float(), entities.random_float());
            let (volume, chew_pitch) = if drinking {
                (0.5, r1 * 0.1 + 0.9)
            } else {
                (if r1 < 0.5 { 0.5 } else { 1.0 }, (r1 - r2) * 0.2 + 1.0)
            };
            effects.push(sound(&chewed, None, volume, chew_pitch));
            if let Some((overflow, fed)) = finished {
                if fed {
                    effects.push(sound(
                        "minecraft:entity.player.burp",
                        None,
                        0.5,
                        r2 * 0.1 + 0.9,
                    ));
                }
                if let Some(stack) = overflow {
                    entities.view.world_items.toss(stack, eye_block, mc_yaw, pitch);
                }
            }
        }
        if holding_item
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
                    .mob_distance(player_id, eye_block, look, game_mode.entity_reach())
                    .is_none_or(|distance| distance > hit.distance)
                && minecraft_terrain::game_mode::can_destroy_in_creative(held)
                && minecraft_terrain::game_mode::creative_breaks(
                    &mut hand.creative_delay,
                    block_press,
                    attack_held,
                    hand_ticks,
                ) > 0
            {
                creative_broken.push(hit.pos);
                hand_mined.push(hit.pos);
                if hand.swing.is_none_or(|t| t >= SWING_TICKS * 0.5) {
                    hand.swing = Some(0.0);
                    hand.swing_offhand = false;
                }
            }
        } else if holding_item
            && (attack_held || hand.dig.pressed)
            && !busy
            && game_mode.may_build()
        {
            digging = true;
            let outlined = minecraft_terrain::doors::Outlined(&terrain.scene);
            let mut staged = minecraft_terrain::placing::Staged::new(&outlined);
            let mob = entities.mob_distance(player_id, eye_block, look, game_mode.entity_reach());
            let digger = crate::digging::digger(
                inventory,
                *selected,
                &survival.status,
                moved.eye_in_water,
                on_ground,
            );
            let progress = |block: &minecraftoss_player::Block| {
                let hardness = scene_block(block)
                    .and_then(|block| terrain.stream.states.state_of(&block))
                    .map_or(-1.0, |state| {
                        globals.registries.blocks.state(state).destroy_speed
                    });
                minecraft_terrain::mining_speed::destroy_progress(
                    globals.data.mining.as_deref(),
                    &*terrain.stream.states,
                    block,
                    hardness,
                    &digger,
                )
            };
            for _ in 0..hand_ticks {
                let hit = player
                    .target(&staged, game_mode.block_reach())
                    .filter(|hit| mob.is_none_or(|distance| distance > hit.distance));
                let block = hit
                    .as_ref()
                    .and_then(|hit| minecraftoss_player::World::block(&staged, hit.pos));
                let clicked = hand.dig.take_press();
                let dug = hand.dig.tick(crate::digging::DigInput {
                    target: hit
                        .as_ref()
                        .zip(block.as_ref())
                        .map(|(hit, block)| (hit.pos, hit.face, block)),
                    held: inventory.slots[*selected].as_ref(),
                    clicked,
                    held_down: attack_held,
                    progress: &progress,
                });
                if dug.stopped {
                    hand.attack_ticks = 0;
                }
                for event in dug.events {
                    use crate::digging::DigEvent;
                    let (pos, face, heard) = match event {
                        DigEvent::Attacked(pos) => {
                            entities.attack_block(&terrain.scene, pos);
                            continue;
                        }
                        DigEvent::Broke { pos, .. } => {
                            minecraftoss_player::World::set_block(&mut staged, pos, None);
                            hand_broken.push(pos);
                            hand_mined.push(pos);
                            continue;
                        }
                        DigEvent::Aborted(_) => continue,
                        DigEvent::Started { pos, face } => (pos, face, true),
                        DigEvent::Hit { pos, face, sound } => (pos, face, sound),
                    };
                    let Some(block) = Scene::block(&terrain.scene, pos) else {
                        continue;
                    };
                    let hit = heard
                        .then(|| terrain.scene.sound_type(block))
                        .flatten()
                        .map(|kind| {
                            (
                                kind.hit.to_string(),
                                (kind.volume + 1.0) / 8.0,
                                kind.pitch * 0.5,
                            )
                        });
                    effects.push(Effect::Dig {
                        pos,
                        face,
                        block: block.clone(),
                        sound: hit,
                    });
                }
                if dug.swing && hand.swing.is_none_or(|t| t >= SWING_TICKS * 0.5) {
                    hand.swing = Some(0.0);
                    hand.swing_offhand = false;
                }
            }
        }
        if let Some(blocking) = hand.blocking.as_mut() {
            blocking.ticks = blocking.ticks.saturating_add(hand_ticks);
        }
        if let Some(charging) = hand.charging.as_mut() {
            charging.ticks = charging.ticks.saturating_add(hand_ticks);
        }
        if let Some(scoping) = hand.scoping.as_mut() {
            scoping.ticks = scoping.ticks.saturating_add(hand_ticks);
        }
        hand.place_delay = hand.place_delay.saturating_sub(hand_ticks);
        let idle = survival.eating().is_none()
            && hand.blocking.is_none()
            && hand.charging.is_none()
            && hand.scoping.is_none()
            && !mw2_offhand;
        let (use_pressed, use_down) = if holding_item {
            (!skating && input.use_pressed, use_held)
        } else {
            (offhand_key && !hand.offhand_key_was, offhand_key)
        };
        let use_pressed = idle && use_pressed;
        if !crate::offhand::eye_held(hand, use_down, *selected) {
            hand.using_eye = None;
        }
        let place =
            use_pressed || (idle && use_down && hand.place_delay == 0 && hand.using_eye.is_none());
        let hands = if place {
            hand.place_delay = 4;
            hand.use_by_key = !holding_item;
            crate::offhand::hand_slots(*selected, !holding_item)
        } else {
            Vec::new()
        };
        let has_item = inventory.slots[*selected].is_some()
            || inventory.slots[crate::offhand::OFFHAND].is_some();
        let block_distance = player
            .target(
                &minecraft_terrain::doors::Outlined(&terrain.scene),
                game_mode.block_reach(),
            )
            .map(|hit| hit.distance);
        let framed = entities.frame_on_ray(
            player_id,
            eye_block,
            look,
            game_mode.entity_reach(),
            block_distance,
        );
        let used = crate::offhand::use_hands(&hands, |slot| {
            use crate::offhand::HandUse;
            let main = slot != crate::offhand::OFFHAND;
            player.selected = slot;
            let held = inventory.slots[slot].clone();
            if let Some(frame) = framed {
                let sent = held.as_ref().map(|stack| SentItemUse {
                    slot,
                    held: stack.id.clone(),
                    eye: eye_block,
                    yaw: mc_yaw,
                    pitch,
                    outcome: minecraft_terrain::item_frame::frame_item_use(frame),
                });
                let infinite = !game_mode.consumes_items();
                if !entities.use_frame(player_id, frame, held.as_ref(), infinite, sent) {
                    return HandUse::Passed;
                }
                hand.swing = Some(0.0);
                return HandUse::Consumed;
            }
            if entities.use_on_mob(
                player_id,
                eye_block,
                look,
                game_mode.entity_reach(),
                block_distance,
                inventory,
                slot,
                !game_mode.consumes_items(),
            ) {
                hand.swing = Some(0.0);
                return HandUse::Consumed;
            }
            let table = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|&pos| {
                    Scene::block(&terrain.scene, pos)
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
                    let block = Scene::block(&terrain.scene, pos)?;
                    let partner = minecraft_terrain::containers::chest_partner(pos, block)
                        .and_then(|at| Scene::block(&terrain.scene, at));
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
                    Scene::block(&terrain.scene, pos)
                        .and_then(minecraft_terrain::stations::Workshop::of)
                        .map(|kind| (pos, kind))
                });
            let campfire = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|&pos| {
                    Scene::block(&terrain.scene, pos)
                        .is_some_and(minecraft_terrain::campfire::is_campfire)
                })
                .filter(|_| !(player.crouching && has_item));
            let lectern = player
                .target(&minecraft_terrain::doors::Outlined(&terrain.scene), 5.0)
                .map(|hit| hit.pos)
                .filter(|_| !(player.crouching && has_item))
                .and_then(|pos| {
                    Scene::block(&terrain.scene, pos)
                        .filter(|block| minecraft_terrain::lectern::is_lectern(block))
                        .map(|block| (pos, minecraft_terrain::lectern::has_book(block)))
                })
                .filter(|&(_, has_book)| {
                    if has_book {
                        main
                    } else {
                        held.as_ref().is_some_and(|stack| {
                            minecraft_terrain::lectern::is_lectern_book(&stack.id)
                        })
                    }
                });
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
                        .state_of(&Block {
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
                        Scene::block(&terrain.scene, (x, y + 1, z))
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
                            let facing = Scene::block(&terrain.scene, opened.cells[0])
                                .and_then(|b| b.properties.get("facing").cloned())
                                .unwrap_or_else(|| "up".to_owned());
                            entities.view.lids.lid_closed(opened.cells[0])
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
            } else if let Some((pos, has_book)) = lectern {
                if has_book {
                    if use_pressed {
                        let open = minecraft_terrain::lectern::LecternAction::Open;
                        entities.lectern(player_id, pos, open, None);
                        hand.swing = Some(0.0);
                    }
                } else if let Some(stack) = held.as_ref()
                    && !entities.awaits_item_use(player_id)
                {
                    let book = minecraft_terrain::lectern::BookStack {
                        id: stack.id.clone(),
                        components: stack
                            .components
                            .as_ref()
                            .map(|components| components.to_string()),
                    };
                    let sent = SentItemUse {
                        slot,
                        held: stack.id.clone(),
                        eye: eye_block,
                        yaw: mc_yaw,
                        pitch,
                        outcome: minecraft_terrain::lectern::book_item_use(pos),
                    };
                    let place = minecraft_terrain::lectern::LecternAction::PlaceBook(book);
                    entities.lectern(player_id, pos, place, Some(sent));
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
                    let monsters = |area| entities.monster_near(area, player_id);
                    use_respawn_block(
                        &player,
                        terrain,
                        &globals.registries,
                        *dimension,
                        RespawnHand {
                            main,
                            held: held.as_ref().map(|stack| stack.id.as_str()),
                            offhand: inventory.slots[crate::offhand::OFFHAND]
                                .as_ref()
                                .map(|stack| stack.id.as_str()),
                        },
                        Night {
                            dark: dark_outside,
                            creative: game_mode.mode
                                == minecraft_terrain::game_mode::GameMode::Creative,
                            monsters: &monsters,
                        },
                    )
                })
                .flatten()
            {
                body.write_blocks(terrain, entities, respawn_use.writes.clone());
                bed = finish_respawn_use(
                    respawn_use,
                    entities,
                    respawn,
                    inventory,
                    slot,
                    game_mode.consumes_items(),
                    &mut effects,
                )
                .or(bed);
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
                    && let Some(toggled) = minecraft_terrain::doors::toggle_sound(&block)
                {
                    let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                    effects.push(Effect::Sound {
                        event: toggled.event,
                        at: Some(centre),
                        volume: toggled.volume,
                        pitch: toggled.pitch,
                        spread: toggled.pitch_spread,
                    });
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref().filter(|stack| crate::books::is_book(stack)) {
                if use_pressed {
                    open_book = crate::books::book_use(stack).map(|kind| crate::books::OpenBook { slot, kind });
                    hand.swing = Some(0.0);
                }
                HandUse::Consumed
            } else if let Some(stack) = held
                .as_ref()
                .filter(|stack| stack.id == minecraft_terrain::durability::STEERING_ITEM)
                && let Some(vehicle) = riding.as_ref().map(|ride| ride.vehicle)
                && entities.can_boost(player_id, vehicle)
            {
                entities.boost(player_id, slot, stack.id.clone());
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && let Some(outcome) = minecraft_terrain::bottle::bottle_use(
                    &player,
                    &terrain.scene,
                    stack,
                    !entities.awaits_item_use(player_id)
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
                        body.write_blocks(
                            terrain,
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
                        let sent = SentItemUse {
                            slot,
                            held: stack.id.clone(),
                            eye: eye_block,
                            yaw: mc_yaw,
                            pitch,
                            outcome,
                        };
                        entities.bottle_breath(
                            player_id,
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
                    entities.view.world_items.toss(rest, eye_block, mc_yaw, pitch);
                }
                for (event, at) in heard {
                    effects.push(sound(event, at, 1.0, 1.0));
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && !entities.awaits_item_use(player_id)
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
                                && inventory.slots[crate::offhand::OFFHAND]
                                    .as_ref()
                                    .is_some_and(|shield| shield.id == crate::offhand::SHIELD),
                        )
                    })
                    .or_else(|| minecraft_terrain::ender_eye::frame_use(&player, &outlined, stack))
                    .or_else(|| {
                        minecraft_terrain::end_crystal::crystal_use(&player, &outlined, stack)
                    })
                    .or_else(|| minecraft_terrain::item_frame::frame_use(&player, &outlined, stack))
                    .or_else(|| {
                        minecraft_terrain::firework::rocket_use(stack, actor.gliding, || {
                            entities.random_float()
                        })
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
                let sent = SentItemUse {
                    slot,
                    held: stack.id.clone(),
                    eye: eye_block,
                    yaw: mc_yaw,
                    pitch,
                    outcome,
                };
                match change {
                    minecraft_terrain::bucket::Change::Set(pos, block) => {
                        body.write_blocks(terrain, entities, vec![(pos, scene_block(&block))]);
                        finish_item_use(
                            entities,
                            inventory,
                            sent,
                            game_mode.consumes_items(),
                            &mut effects,
                        );
                    }
                    minecraft_terrain::bucket::Change::Server(pos, action) => {
                        entities.item_on_block(player_id, pos, action, sent)
                    }
                    minecraft_terrain::bucket::Change::ThrowEye(throw) => {
                        entities.throw_eye(player_id, throw, sent)
                    }
                    minecraft_terrain::bucket::Change::Rocket(boost_ticks) => {
                        if body.launch_rocket(boost_ticks) {
                            hand.swing = Some(0.0);
                            finish_item_use(
                                entities,
                                inventory,
                                sent,
                                game_mode.consumes_items(),
                                &mut effects,
                            );
                        }
                    }
                    minecraft_terrain::bucket::Change::Breath => entities.bottle_breath(
                        player_id,
                        player.pos,
                        if player.crouching { 1.5 } else { 1.8 },
                        sent,
                    ),
                    minecraft_terrain::bucket::Change::Frame(_)
                    | minecraft_terrain::bucket::Change::Lectern(_) => {}
                }
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && !entities.awaits_item_use(player_id)
                && game_mode.interacts()
                && let Some(outcome) = minecraft_terrain::ender_eye::throw_use(
                    &player,
                    &minecraft_terrain::doors::Outlined(&terrain.scene),
                    stack,
                )
                && let minecraft_terrain::bucket::Change::ThrowEye(throw) = outcome.change.clone()
            {
                hand.using_eye = Some(slot);
                let sent = SentItemUse {
                    slot,
                    held: stack.id.clone(),
                    eye: eye_block,
                    yaw: mc_yaw,
                    pitch,
                    outcome,
                };
                entities.throw_eye(player_id, throw, sent);
                HandUse::Consumed
            } else if let Some(stack) = held.as_ref()
                && game_mode.may_build()
                && let Some(egg) = minecraft_terrain::spawn_egg::egg_use(
                    &player,
                    &minecraft_terrain::doors::Outlined(&terrain.scene),
                    &stack.id,
                )
            {
                match egg {
                    minecraft_terrain::spawn_egg::EggUse::Spawn { kind, position } => {
                        entities.hatch_egg(
                            player_id,
                            inventory,
                            slot,
                            kind,
                            position,
                            game_mode.consumes_items(),
                        );
                    }
                    minecraft_terrain::spawn_egg::EggUse::Spawner(pos) => {
                        if let Some(kind) = minecraft_terrain::spawn_egg::spawned_kind(&stack.id) {
                            entities.egg_on_spawner(
                                player_id,
                                inventory,
                                slot,
                                kind,
                                pos,
                                game_mode.consumes_items(),
                            );
                        }
                    }
                }
                hand.swing = Some(0.0);
                HandUse::Consumed
            } else if held
                .as_ref()
                .is_some_and(|stack| stack.id == minecraft_terrain::trident::ITEM)
            {
                match crate::trident::Charging::start(
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
            } else if held
                .as_ref()
                .is_some_and(|stack| stack.id == crate::spyglass::ITEM)
            {
                match crate::spyglass::Scoping::start(inventory, slot) {
                    Some(scoping) => {
                        hand.scoping = Some(scoping);
                        effects.push(sound(crate::spyglass::USE_SOUND, None, 1.0, 1.0));
                        HandUse::Consumed
                    }
                    None => HandUse::Failed,
                }
            } else if let Some((stack, kind)) = held
                .as_ref()
                .and_then(|s| minecraft_terrain::thrown::Throwable::of(&s.id).map(|k| (s, k)))
            {
                if !hand.cooldowns.active(&stack.id)
                    && entities.throw(
                        player_id,
                        stack,
                        DVec3::from_array(actor.eye),
                        mc_yaw,
                        pitch,
                        actor.velocity,
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
                    let throw_pitch =
                        minecraft_terrain::thrown::throw_pitch(entities.random_float());
                    effects.push(sound(kind.sound(), None, 0.5, throw_pitch));
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
                body.write_blocks(terrain, entities, writes);
                hand.swing = Some(0.0);
                if let Some(block) = Scene::block(&terrain.scene, pos).cloned()
                    && let Some(kind) = terrain.scene.sound_type(&block)
                {
                    let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                    effects.push(sound(
                        &kind.place,
                        Some(centre),
                        (kind.volume + 1.0) / 2.0,
                        kind.pitch * 0.8,
                    ));
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
                .is_some_and(|stack| stack.id == crate::offhand::SHIELD)
                && !hand.cooldowns.active(crate::offhand::SHIELD)
            {
                hand.blocking = Some(crate::offhand::Blocking::start(slot));
                HandUse::Consumed
            } else if placeable {
                let outlined = minecraft_terrain::doors::Outlined(&terrain.scene);
                let mut staged = minecraft_terrain::placing::Staged::new(&outlined);
                let opening = held
                    .as_ref()
                    .filter(|stack| minecraft_terrain::doors::Opening::of(&stack.id).is_some());
                let sturdy = |below: (i32, i32, i32)| {
                    Scene::block(&terrain.scene, below)
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
                            actor.crouching,
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
                        "chest placement: crouching {} swimming {} yaw {} placed {:?} writes {:?}",
                        player.crouching, actor.swimming, player.yaw, placed, writes
                    );
                }
                let bodies = body.obstacles(entities);
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
                    body.write_blocks(terrain, entities, writes);
                    if let Some(stack) = held
                        .as_ref()
                        .filter(|stack| minecraft_terrain::containers::is_shulker_box(&stack.id))
                        && Scene::block(&terrain.scene, pos)
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
                    if let Some(block) = Scene::block(&terrain.scene, pos).cloned()
                        && let Some(kind) = terrain.scene.sound_type(&block)
                    {
                        let centre = [pos.0 as f64 + 0.5, pos.1 as f64 + 0.5, pos.2 as f64 + 0.5];
                        effects.push(sound(
                            &kind.place,
                            Some(centre),
                            (kind.volume + 1.0) / 2.0,
                            kind.pitch * 0.8,
                        ));
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
        hand.place_delay = crate::offhand::repeat_delay(used, hand.place_delay);
        if let Some((slot, crate::offhand::HandUse::Consumed)) = used
            && hand.swing == Some(0.0)
        {
            hand.swing_offhand = slot == crate::offhand::OFFHAND;
        }
    }
    hand.offhand_key_was = offhand_key;
    if !digging {
        hand.dig.pressed = false;
        if hand.dig.stop() {
            hand.attack_ticks = 0;
        }
    }
    Acted {
        open_table,
        open_block,
        open_bench,
        hand_mined,
        creative_broken,
        hand_broken,
        effects,
        bed,
        open_book,
    }
}

pub type Broken = (BlockPos, Block, Option<f32>);

#[derive(Default)]
pub struct Harvest {
    pub worn_out: Vec<String>,
    pub effects: Vec<Effect>,
}

pub fn harvest(
    player: &mut Player,
    level: &mut Level,
    globals: &Globals,
    (mut broken, dug): (Vec<Broken>, Vec<Broken>),
    hand_mined: &[BlockPos],
    feet: [f64; 3],
) -> Harvest {
    let player_id = u64::from(player.id.0);
    let mut harvest = Harvest::default();
    let Player {
        survival,
        mode: player_mode,
        inventory,
        selected,
        ..
    } = player;
    let Level {
        entities,
        containers,
        terrain,
        openers,
        ..
    } = level;
    let dug_blocks: Vec<_> = dug
        .iter()
        .map(|(pos, block, _)| {
            let block = minecraftoss_player::Block {
                id: block.id.key(),
                properties: block.properties.clone(),
            };
            (*pos, block)
        })
        .collect();
    let wrong_tool = crate::digging::wrong_tool(
        globals.data.mining.as_deref(),
        &*terrain.stream.states,
        &dug_blocks,
        inventory.slots[*selected].as_ref(),
    );
    let worn: Vec<f32> = dug
        .iter()
        .map(|(_, block, _)| {
            terrain.stream.states.state_of(block).map_or(-1.0, |state| {
                globals.registries.blocks.state(state).destroy_speed
            })
        })
        .collect();
    broken.extend(dug);
    if let Some(breaks) = terrain.friend_breaks.as_mut() {
        breaks.extend(
            broken
                .iter()
                .filter(|(_, _, blast)| blast.is_none())
                .filter_map(|(pos, block, _)| {
                    let state = terrain.stream.states.state_of(block)?;
                    Some(([pos.0, pos.1, pos.2], u32::from(state.0)))
                }),
        );
    }
    let mined = broken
        .iter()
        .filter(|(pos, _, blast)| blast.is_none() && !wrong_tool.contains(pos))
        .count();
    survival.exhaust(crate::survival::MINE_EXHAUSTION * mined as f32);
    let Some(entities) = entities.as_mut() else {
        return harvest;
    };
    let held = inventory.slots[*selected].clone();
    let positions: Vec<_> = broken.iter().map(|(pos, ..)| *pos).collect();
    entities.mined(
        held.as_ref(),
        &broken,
        hand_mined,
        !player_mode.drops_from_breaking(),
    );
    entities.guard_mined(player_id, &terrain.stream.states, &broken, feet);
    let kept: std::collections::HashMap<_, _> = broken
        .iter()
        .filter(|(_, block, _)| {
            minecraft_terrain::containers::Station::of(block)
                == Some(minecraft_terrain::containers::Station::Shulker)
        })
        .filter_map(|(pos, ..)| {
            containers
                .take_shulker(*pos)
                .map(|components| (*pos, components))
        })
        .collect();
    let dropping: Vec<_> = broken
        .iter()
        .filter(|(pos, _, blast)| {
            player_mode.drops_from_breaking()
                || blast.is_some()
                || kept.get(pos).is_some_and(|components| {
                    components
                        .get(minecraft_terrain::containers::CONTAINER)
                        .is_some()
                })
        })
        .filter(|(pos, ..)| !wrong_tool.contains(pos))
        .cloned()
        .collect();
    entities.award_block_experience(held.as_ref(), &dropping, hand_mined);
    entities.drop_blocks(held.as_ref(), &dropping, kept, hand_mined);
    for &pos in &positions {
        openers.removed(pos);
        crate::level::spill(containers, entities, pos);
    }
    for hardness in worn {
        if let Some(item) = crate::digging::wear_after_break(inventory, *selected, hardness, || {
            entities.random_float()
        }) {
            harvest.worn_out.push(item);
            let broke = minecraft_terrain::flint::break_sound(entities.random_float());
            harvest
                .effects
                .push(sound(broke.event, None, broke.volume, broke.pitch));
        }
    }
    harvest
}

struct SceneView<'a>(&'a Terrain);

impl BlockView for SceneView<'_> {
    fn state_at(&mut self, pos: Pos) -> BlockStateId {
        Scene::block(&self.0.scene, pos)
            .and_then(|block| self.0.stream.states.state_of(block))
            .unwrap_or(BlockStateId::AIR)
    }
}

#[derive(Clone, Copy)]
struct RespawnHand<'a> {
    main: bool,
    held: Option<&'a str>,
    offhand: Option<&'a str>,
}

struct Night<'a> {
    dark: bool,
    creative: bool,
    monsters: &'a dyn Fn([f64; 6]) -> bool,
}

#[derive(Default)]
struct RespawnBlockUse {
    writes: Vec<(Pos, Option<Block>)>,
    blast: Option<(Blast, Option<Pos>)>,
    sounds: Vec<(&'static str, [f64; 3])>,
    respawn: Option<(Respawn, Option<[f64; 3]>)>,
    overlay: Option<String>,
    spends: bool,
    occupied: Option<Pos>,
    sleep: Option<Bed>,
}

fn bad_respawn_point(center: [f64; 3]) -> Blast {
    Blast {
        center,
        power: BAD_RESPAWN_POINT_POWER,
        fire: true,
    }
}

fn use_respawn_block(
    aim: &minecraftoss_player::Player,
    terrain: &Terrain,
    registries: &Registries,
    dimension: Dimension,
    hand: RespawnHand,
    night: Night,
) -> Option<RespawnBlockUse> {
    let pos = aim
        .target(
            &minecraft_terrain::doors::Outlined(&terrain.scene),
            RESPAWN_BLOCK_REACH,
        )?
        .pos;
    let mut view = SceneView(terrain);
    let fuel = Fuel {
        main: hand.main,
        held: hand.held == Some(GLOWSTONE),
        offhand: hand.main && hand.offhand == Some(GLOWSTONE),
    };
    if let Some(anchor) =
        respawn_block::use_anchor(&mut view, registries, &terrain.rules, pos, dimension, fuel)
    {
        return Some(use_anchor(anchor, terrain));
    }
    if !hand.main {
        return None;
    }
    let mut used = RespawnBlockUse::default();
    match respawn_block::use_bed(&mut view, registries, &terrain.rules, pos)? {
        BedUse::Destroy {
            removed,
            blast,
            sound,
            message,
        } => {
            let head = respawn_block::centre(removed[0]);
            used.writes = removed.into_iter().map(|at| (at, None)).collect();
            used.blast = blast.map(|center| (bad_respawn_point(center), None));
            used.sounds.extend(sound.map(|event| (event, head)));
            used.overlay = message;
        }
        BedUse::Occupied(head) => used.occupied = Some(head),
        BedUse::Sleep(bed) => {
            let sleeper = Sleeper {
                feet: aim.pos.to_array(),
                yaw: aim.yaw as f32,
                pitch: aim.pitch as f32,
                dimension,
                dark: night.dark,
                creative: night.creative,
            };
            let rest = respawn_block::rest_in_bed(
                &mut view,
                registries,
                &terrain.rules,
                &bed,
                &sleeper,
                night.monsters,
            );
            used.respawn = rest.respawn.map(|point| (point, None));
            match rest.outcome {
                Outcome::Problem(problem) => used.overlay = problem,
                Outcome::Sleep => used.sleep = Some(bed),
            }
        }
        BedUse::Incomplete => {}
    }
    Some(used)
}

fn use_anchor(anchor: AnchorUse, terrain: &Terrain) -> RespawnBlockUse {
    let mut used = RespawnBlockUse::default();
    match anchor {
        AnchorUse::Charge { pos, state } => {
            used.writes = vec![(pos, terrain.stream.states.block(state).cloned())];
            used.sounds
                .push((ANCHOR_CHARGE_SOUND, respawn_block::centre(pos)));
            used.spends = true;
        }
        AnchorUse::SetSpawn(point) => {
            used.respawn = Some((point, Some(respawn_block::centre(point.at.pos))));
        }
        AnchorUse::Explode { pos, shielded } => {
            used.writes = vec![(pos, None)];
            used.blast = Some((
                bad_respawn_point(respawn_block::centre(pos)),
                shielded.then_some(pos),
            ));
        }
    }
    used
}

fn finish_respawn_use(
    used: RespawnBlockUse,
    entities: &mut Entities,
    respawn: &mut Option<Respawn>,
    inventory: &mut Inventory,
    slot: usize,
    consumes: bool,
    effects: &mut Vec<Effect>,
) -> Option<Bed> {
    if let Some((blast, shield)) = used.blast {
        entities.explode_respawn_point(blast, shield);
    }
    if used.spends
        && consumes
        && let Some(stack) = inventory.slots.get_mut(slot)
        && let Some(held) = stack.as_mut()
    {
        held.count -= 1;
        if held.count == 0 {
            *stack = None;
        }
    }
    let mut heard = used.sounds;
    if let Some((point, at)) = used.respawn
        && respawn_block::set_respawn(respawn, point)
    {
        effects.push(Effect::Chat(Clientbound::System {
            text: Text::translate(SPAWN_SET, Vec::new()),
            overlay: false,
        }));
        heard.extend(at.map(|at| (ANCHOR_SET_SPAWN_SOUND, at)));
    }
    if let Some(key) = used.overlay {
        effects.push(Effect::Overlay(key));
    }
    if let Some(head) = used.occupied
        && !entities.wake_villager_in(head)
    {
        effects.push(Effect::Overlay(OCCUPIED.to_owned()));
    }
    effects.extend(
        heard
            .into_iter()
            .map(|(event, at)| sound(event, Some(at), 1.0, 1.0)),
    );
    used.sleep
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::entities::{PlayerView, TickActor};
    use minecraft_net::messages::Target;
    use minecraft_terrain::game_mode::{GameMode, PlayerMode};
    use minecraftoss_player::inventory::ItemStack;

    pub(crate) const FEET: [f64; 3] = [4.5, 201.0, 4.5];

    #[test]
    fn reach_is_refused_beyond_five_and_a_half_blocks() {
        let eye = [0.5, 65.62, 0.5];
        let survival = PlayerMode::new(GameMode::Survival);
        let creative = PlayerMode::new(GameMode::Creative);
        assert!(reaches_block(eye, (5, 65, 0), survival.block_reach()));
        assert!(!reaches_block(eye, (6, 65, 0), survival.block_reach()));
        assert!(reaches_block(eye, (6, 65, 0), creative.block_reach()));
        assert!(!reaches_block(eye, (7, 65, 0), creative.block_reach()));
        assert!(reaches_block(eye, (-5, 65, 0), survival.block_reach()));
        assert!(!reaches_block(eye, (-5, 60, 0), survival.block_reach()));
        let block = |x: i32, hit_x: f32| Target::Block {
            pos: [x, 65, 0],
            face: 4,
            hit: [hit_x, 65.5, 0.5],
        };
        assert!(allows(&block(5, 5.0), eye, &survival, |_| None));
        assert!(!allows(&block(6, 6.0), eye, &survival, |_| None));
        let zombie = [5.7, 65.0, 0.2, 6.3, 66.95, 0.8];
        let mob = |id| (id == 7).then_some(zombie);
        assert!(allows(&Target::Mob { id: 7 }, eye, &survival, mob));
        assert!(
            !allows(&Target::Mob { id: 8 }, eye, &survival, mob),
            "an unknown mob"
        );
        let far = [9.7, 65.0, 0.2, 10.3, 66.95, 0.8];
        assert!(!allows(&Target::Mob { id: 7 }, eye, &survival, |_| Some(
            far
        )));
        assert!(allows(&Target::Miss, eye, &survival, |_| None));
        assert!(!allows(
            &Target::Player { client: 1 },
            eye,
            &survival,
            |_| None
        ));
    }

    #[test]
    fn a_hit_point_off_the_face_is_refused() {
        assert!(hit_on_block((3, 64, 3), [4.0, 64.5, 3.5]));
        assert!(hit_on_block((3, 64, 3), [3.0, 65.0, 4.0]));
        assert!(hit_on_block((3, 64, 3), [4.4, 64.5, 3.5]));
        assert!(!hit_on_block((3, 64, 3), [4.6, 64.5, 3.5]));
        assert!(!hit_on_block((3, 64, 3), [3.5, 62.9, 3.5]));
        let eye = [3.5, 66.62, 0.5];
        let mode = PlayerMode::new(GameMode::Survival);
        let target = |hit| Target::Block {
            pos: [3, 64, 3],
            face: 1,
            hit,
        };
        assert!(allows(&target([3.5, 65.0, 3.5]), eye, &mode, |_| None));
        assert!(!allows(&target([3.5, 66.5, 3.5]), eye, &mode, |_| None));
    }

    pub(crate) struct Flat;

    impl Body for Flat {
        fn write_blocks(
            &mut self,
            terrain: &mut Terrain,
            entities: &mut Entities,
            writes: Vec<(BlockPos, Option<Block>)>,
        ) {
            crate::level::write_blocks(terrain, entities, writes);
        }

        fn obstacles(&self, _: &Entities) -> Vec<[f64; 6]> {
            vec![[
                FEET[0] - 0.3,
                FEET[1],
                FEET[2] - 0.3,
                FEET[0] + 0.3,
                FEET[1] + 1.8,
                FEET[2] + 0.3,
            ]]
        }

        fn push(&mut self, _: DVec3) {}

        fn launch_rocket(&mut self, _: i32) -> bool {
            false
        }
    }

    pub(crate) fn level_and_globals() -> Option<(Level, Globals)> {
        let mut level =
            crate::commands::tests::test_stream().and_then(crate::commands::tests::test_level)?;
        let registries = level.terrain.stream.states.registries().clone();
        let globals = crate::server::tests::test_globals(registries, None);
        let terrain = &mut level.terrain;
        let mut entities = Entities::new(
            &terrain.stream,
            minecraft_net::Dimension::Overworld,
            0,
            &globals.data,
            None,
            None,
        );
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = terrain
                    .stream
                    .load_chunk_now(minecraftoss_core::ChunkPos::new(x, z));
                entities.load_chunk(&chunk);
                terrain.scene.insert_chunk(chunk);
            }
        }
        for x in 0..10 {
            for z in 0..10 {
                terrain
                    .scene
                    .set((x, 200, z), Some(Block::new("minecraft:stone")));
                entities.placed(&terrain.scene, (x, 200, z));
            }
        }
        level.entities = Some(entities);
        Some((level, globals))
    }

    pub(crate) fn actor(yaw: f32, pitch: f32) -> Actor {
        Actor {
            alive: true,
            feet: FEET,
            eye: [FEET[0], FEET[1] + 1.62, FEET[2]],
            yaw,
            pitch,
            velocity: DVec3::ZERO,
            gravity: 22.0,
            crouching: false,
            swimming: false,
            sprinting: false,
            gliding: false,
            mw2_offhand: false,
            skating: false,
        }
    }

    pub(crate) fn moved() -> Moved {
        Moved {
            on_ground: true,
            hand_ticks: 1,
            holding: true,
            holding_item: true,
            ..Moved::default()
        }
    }

    pub(crate) fn pitch_to(eye: [f64; 3], at: [f64; 3]) -> f32 {
        let across = (at[0] - eye[0]).hypot(at[2] - eye[2]);
        (eye[1] - at[1]).atan2(across).to_degrees() as f32
    }

    pub(crate) fn holding(id: &str, count: u8) -> Player {
        let mut player = crate::the_end::tests::player(minecraft_net::Dimension::Overworld);
        player.inventory.slots[0] = Some(ItemStack::new(id, count));
        player
    }

    pub(crate) fn count(player: &Player) -> Option<u8> {
        player.inventory.slots[0].as_ref().map(|stack| stack.count)
    }

    fn press() -> Hands {
        Hands {
            use_pressed: true,
            use_held: true,
            ..Hands::default()
        }
    }

    #[test]
    fn a_held_block_is_placed_on_the_face_the_player_looks_at() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut player = holding("minecraft:cobblestone", 32);
        let looking = actor(0.0, pitch_to(actor(0.0, 0.0).eye, [4.5, 201.0, 6.5]));
        let acted = act(
            &mut player,
            &mut level,
            &globals,
            &mut Flat,
            &looking,
            press(),
            &moved(),
            None,
            0.0,
        );
        let placed = Scene::block(&level.terrain.scene, (4, 201, 6)).map(|b| b.id.key());
        assert_eq!(placed.as_deref(), Some("minecraft:cobblestone"));
        assert_eq!(count(&player), Some(31));
        assert_eq!(player.hand.swing, Some(0.0));
        let at_block = |effect: &Effect| matches!(effect, Effect::Sound { at: Some(at), .. } if *at == [4.5, 201.5, 6.5]);
        assert!(acted.effects.iter().any(at_block), "{:?}", acted.effects);
        let idle = Hands::default();
        let again = act(
            &mut player,
            &mut level,
            &globals,
            &mut Flat,
            &looking,
            idle,
            &moved(),
            None,
            0.0,
        );
        assert!(again.effects.is_empty(), "a press acts once");
        assert_eq!(count(&player), Some(31));
    }

    #[test]
    fn a_door_opens_when_used() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let door = |half: &str| {
            Block::new("minecraft:oak_door")
                .with("facing", "east")
                .with("half", half)
                .with("hinge", "left")
                .with("open", "false")
                .with("powered", "false")
        };
        let terrain = &mut level.terrain;
        let entities = level.entities.as_mut().expect("entities");
        for (pos, half) in [((6, 201, 4), "lower"), ((6, 202, 4), "upper")] {
            terrain.scene.set(pos, Some(door(half)));
            entities.placed(&terrain.scene, pos);
        }
        let mut player = holding("minecraft:cobblestone", 32);
        let looking = actor(-90.0, pitch_to(actor(0.0, 0.0).eye, [6.05, 201.8, 4.5]));
        let acted = act(
            &mut player,
            &mut level,
            &globals,
            &mut Flat,
            &looking,
            press(),
            &moved(),
            None,
            0.0,
        );
        assert_eq!(
            count(&player),
            Some(32),
            "a door is used, not built against"
        );
        let opening = |effect: &Effect| {
            matches!(effect, Effect::Sound { event, spread, .. }
                if event == "minecraft:block.wooden_door.open" && *spread > 0.0)
        };
        assert!(acted.effects.iter().any(opening), "{:?}", acted.effects);
        let entities = level.entities.as_mut().expect("entities");
        let mut inventory = Inventory::default();
        let started = std::time::Instant::now();
        loop {
            assert!(
                started.elapsed().as_secs() < 300,
                "the level never opened the door"
            );
            let normal = if entities.ready_for_ticks() {
                vec![true]
            } else {
                Vec::new()
            };
            let batch = crate::tick_rate::TickBatch {
                normal,
                overload: None,
            };
            let view = PlayerView {
                feet: FEET,
                alive: true,
                attackable: true,
                spectator: false,
                picks_up: true,
                health: 20.0,
                yaw: -90.0,
                pitch: 0.0,
                main_hand: None,
                offhand: None,
                sneaking: false,
                swirl: None,
            };
            let mut actors = [TickActor {
                id: 0,
                view,
                inventory: &mut inventory,
                selected: 0,
            }];
            let changes = entities.tick(
                &mut actors,
                0.05,
                &batch,
                crate::tick_rate::ClientTiming::default(),
                minecraft_terrain::server::ClockTicks::default(),
                true,
            );
            let open = |block: &Option<Block>| {
                block
                    .as_ref()
                    .and_then(|b| b.properties.get("open"))
                    .map(String::as_str)
                    == Some("true")
            };
            if changes
                .iter()
                .any(|(pos, block)| *pos == (6, 201, 4) && open(block))
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn held_food_is_eaten_while_use_is_held() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut player = holding("minecraft:cooked_beef", 4);
        player.survival.status.food.level = 10;
        let looking = actor(0.0, -30.0);
        let held = Hands {
            use_held: true,
            ..Hands::default()
        };
        let mut chewed = 0;
        for tick in 0..40 {
            if count(&player) == Some(3) {
                break;
            }
            let input = if tick == 0 { press() } else { held };
            let acted = act(
                &mut player,
                &mut level,
                &globals,
                &mut Flat,
                &looking,
                input,
                &moved(),
                None,
                0.0,
            );
            let eating = |effect: &&Effect| matches!(effect, Effect::Sound { event, .. } if event.contains("eat"));
            chewed += acted.effects.iter().filter(eating).count();
        }
        assert_eq!(count(&player), Some(3), "one beef is eaten");
        assert!(
            player.survival.eating().is_some(),
            "holding use goes on to the next beef"
        );
        assert!(player.survival.status.food.level > 10);
        assert!(chewed > 1, "chewing is heard while eating");
        let mut stopped = holding("minecraft:cooked_beef", 4);
        stopped.survival.status.food.level = 10;
        let idle = Hands::default();
        let _ = act(
            &mut stopped,
            &mut level,
            &globals,
            &mut Flat,
            &looking,
            press(),
            &moved(),
            None,
            0.0,
        );
        assert!(stopped.survival.eating().is_some());
        let _ = act(
            &mut stopped,
            &mut level,
            &globals,
            &mut Flat,
            &looking,
            idle,
            &moved(),
            None,
            0.0,
        );
        assert!(
            stopped.survival.eating().is_none(),
            "letting go stops eating"
        );
    }

    #[test]
    fn a_held_spyglass_scopes_while_use_is_held_and_stops_with_its_sound() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut player = holding(crate::spyglass::ITEM, 1);
        let looking = actor(0.0, -30.0);
        let held = Hands {
            use_held: true,
            attack_pressed: true,
            ..Hands::default()
        };
        let heard = |acted: &Acted, wanted: &str| {
            acted
                .effects
                .iter()
                .any(|effect| matches!(effect, Effect::Sound { event, .. } if event == wanted))
        };
        let mut step = |player: &mut Player, input: Hands| {
            act(
                player,
                &mut level,
                &globals,
                &mut Flat,
                &looking,
                input,
                &moved(),
                None,
                0.0,
            )
        };
        let started = step(&mut player, press());
        assert!(heard(&started, crate::spyglass::USE_SOUND));
        assert_eq!(
            player.hand.scoping.map(|scoping| scoping.slot),
            Some(0),
            "the spyglass in the main hand scopes"
        );
        player.hand.swing = None;
        for _ in 0..5 {
            let acted = step(&mut player, held);
            assert!(!heard(&acted, crate::spyglass::STOP_SOUND));
        }
        assert_eq!(
            player.hand.scoping.map(|scoping| scoping.ticks),
            Some(5),
            "scoping counts the hand ticks"
        );
        assert_eq!(player.hand.swing, None, "a scoping hand does not punch");
        let stopped = step(&mut player, Hands::default());
        assert!(heard(&stopped, crate::spyglass::STOP_SOUND));
        assert_eq!(player.hand.scoping, None, "letting go stops scoping");
    }

    fn written_book() -> ItemStack {
        let mut stack = ItemStack::new(crate::books::WRITTEN_BOOK, 1);
        stack.components = Some(serde_json::json!({
            crate::books::WRITTEN_BOOK_CONTENT: {
                "title": {"raw": "Test"},
                "author": "Owner",
                "pages": [{"raw": "one"}, {"raw": "two"}]
            }
        }));
        stack
    }

    fn use_once(level: &mut Level, globals: &Globals, player: &mut Player, input: Hands) -> Acted {
        act(
            player,
            level,
            globals,
            &mut Flat,
            &actor(0.0, -30.0),
            input,
            &moved(),
            None,
            0.0,
        )
    }

    #[test]
    fn right_clicking_with_a_written_book_opens_it_for_that_hand() {
        use crate::books::{BookUse, OpenBook};
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut player = crate::the_end::tests::player(minecraft_net::Dimension::Overworld);
        player.selected = 3;
        player.inventory.slots[3] = Some(written_book());
        let acted = use_once(&mut level, &globals, &mut player, press());
        assert_eq!(
            acted.open_book,
            Some(OpenBook {
                slot: 3,
                kind: BookUse::Read
            })
        );
        assert_eq!(player.hand.swing, Some(0.0));
        assert!(!player.hand.swing_offhand);
        assert_eq!(
            player.inventory.slots[3].as_ref().map(|stack| stack.count),
            Some(1),
            "reading keeps the book"
        );
        let mut blank = holding(crate::books::WRITTEN_BOOK, 1);
        let acted = use_once(&mut level, &globals, &mut blank, press());
        assert_eq!(acted.open_book, None, "a written book without content");
        assert_eq!(blank.hand.swing, Some(0.0), "it still swings");
    }

    #[test]
    fn a_quill_in_the_offhand_opens_the_editor_for_slot_40() {
        use crate::books::{BookUse, OpenBook};
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut player = crate::the_end::tests::player(minecraft_net::Dimension::Overworld);
        player.inventory.slots[crate::offhand::OFFHAND] =
            Some(ItemStack::new(crate::books::WRITABLE_BOOK, 1));
        let acted = use_once(&mut level, &globals, &mut player, press());
        assert_eq!(
            acted.open_book,
            Some(OpenBook {
                slot: crate::offhand::OFFHAND,
                kind: BookUse::Write
            })
        );
        assert_eq!(player.hand.swing, Some(0.0));
        assert!(player.hand.swing_offhand, "the offhand swings");
    }

    #[test]
    fn a_lectern_still_takes_a_held_book_before_it_opens() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let pos = (6, 201, 4);
        let terrain = &mut level.terrain;
        let entities = level.entities.as_mut().expect("entities");
        let lectern = Block::new("minecraft:lectern")
            .with("facing", "west")
            .with("has_book", "false")
            .with("powered", "false");
        terrain.scene.set(pos, Some(lectern));
        entities.placed(&terrain.scene, pos);
        let mut player = crate::the_end::tests::player(minecraft_net::Dimension::Overworld);
        player.inventory.slots[0] = Some(written_book());
        let looking = actor(-90.0, pitch_to(actor(0.0, 0.0).eye, [6.5, 201.5, 4.5]));
        let acted = act(
            &mut player,
            &mut level,
            &globals,
            &mut Flat,
            &looking,
            press(),
            &moved(),
            None,
            0.0,
        );
        assert_eq!(acted.open_book, None, "the lectern takes the book");
        assert_eq!(player.hand.swing, Some(0.0));
        let entities = level.entities.as_ref().expect("entities");
        assert!(
            entities.awaits_item_use(u64::from(player.id.0)),
            "the book goes onto the lectern"
        );
    }

    #[test]
    fn a_spectator_does_not_open_a_book() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut player = crate::the_end::tests::player(minecraft_net::Dimension::Overworld);
        player.mode = PlayerMode::new(GameMode::Spectator);
        player.inventory.slots[0] = Some(written_book());
        let acted = use_once(&mut level, &globals, &mut player, press());
        assert_eq!(acted.open_book, None);
        assert_eq!(player.hand.swing, None);
        player.mode = PlayerMode::new(GameMode::Adventure);
        let acted = use_once(&mut level, &globals, &mut player, press());
        assert!(acted.open_book.is_some(), "adventure players read books");
    }

    #[test]
    fn holding_use_opens_a_book_only_once() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut player = holding(crate::books::WRITABLE_BOOK, 1);
        let first = use_once(&mut level, &globals, &mut player, press());
        assert!(first.open_book.is_some());
        let held = Hands {
            use_held: true,
            ..Hands::default()
        };
        for tick in 0..12 {
            let acted = use_once(&mut level, &globals, &mut player, held);
            assert_eq!(acted.open_book, None, "tick {tick}");
        }
        let again = use_once(&mut level, &globals, &mut player, press());
        assert!(again.open_book.is_some(), "a new press opens it again");
    }
}
