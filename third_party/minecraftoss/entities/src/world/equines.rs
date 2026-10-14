use super::*;
use crate::cow::InteractionResult;
use crate::horse::{Feeding, HorseKind, HorseState};
use crate::llama_ai::{Caravaner, Foe, LlamaContext, Seen};
use crate::llama_spit::{LlamaSpit, SpitHit, SpitTarget};
use crate::monster_ai::Target;
use minecraftoss_player::inventory::{Inventory, ItemStack};

#[derive(Clone)]
pub struct LlamaSpitEntity {
    pub id: u64,
    pub spit: LlamaSpit,
    pub previous_position: DVec3,
}

pub(super) struct HorseInputs {
    pub mates: Option<Vec<CowCandidate>>,
    pub tempters: Vec<u64>,
    pub llama: Option<LlamaContext>,
}

#[derive(Default)]
pub(super) struct HorseOutputs {
    pub spit: Option<DVec3>,
    pub join: Vec<u64>,
    pub leave: Vec<u64>,
    pub tamed: Option<u64>,
    pub buck: bool,
}

pub(super) struct HorseRide {
    pub player: Option<crate::controls::PlayerControls>,
    pub mob_controlled: bool,
    pub carrying: bool,
    pub crazy: Option<crate::cow_ai::Crazy>,
}

const HORSE_COATS: i32 = 7;
const HORSE_MARKINGS: i32 = 5;
const LLAMA_VARIANTS: i32 = 4;

pub fn horse_event(kind: HorseKind, baby: bool, event: &str) -> Option<&'static str> {
    Some(match (kind.sound_family(baby), event) {
        ("horse", "angry") => "entity.horse.angry",
        ("baby_horse", "angry") => "entity.baby_horse.angry",
        ("donkey", "angry") => "entity.donkey.angry",
        ("mule", "angry") => "entity.mule.angry",
        ("llama", "angry") => "entity.llama.angry",
        ("horse", "eat") => "entity.horse.eat",
        ("baby_horse", "eat") => "entity.baby_horse.eat",
        ("donkey", "eat") => "entity.donkey.eat",
        ("mule", "eat") => "entity.mule.eat",
        ("llama", "eat") => "entity.llama.eat",
        ("donkey", "chest") => "entity.donkey.chest",
        ("mule", "chest") => "entity.mule.chest",
        ("llama", "chest") => "entity.llama.chest",
        ("llama", "spit") => "entity.llama.spit",
        ("llama", "swag") => "entity.llama.swag",
        _ => return None,
    })
}

pub fn is_carpet(item: &str) -> bool {
    item.ends_with("_carpet") && !item.contains("moss")
}

pub(super) fn horse_movement_sounds(horse: &HorseState, baby: bool) -> MovementSounds {
    if horse.kind.is_llama() {
        return MovementSounds::creature(Some("entity.llama.step"));
    }
    let step = if horse.kind.sound_family(baby) == "baby_horse" {
        "entity.baby_horse.step"
    } else {
        "entity.horse.step"
    };
    MovementSounds {
        horse_step: Some((step, "entity.horse.step_wood")),
        ..MovementSounds::creature(None)
    }
}

pub(super) fn horse_death_equipment(horse: &HorseState, mob_drops: bool) -> Vec<ItemStack> {
    let mut out = Vec::new();
    if let (Some(item), true) = (&horse.body_item, mob_drops) {
        out.push(ItemStack::new(item, 1));
    }
    if horse.saddled && mob_drops {
        out.push(ItemStack::new("minecraft:saddle", 1));
    }
    if horse.chest {
        out.extend(horse.inventory.iter().flatten().cloned());
        out.push(ItemStack::new("minecraft:chest", 1));
    }
    out
}

fn player_height(eye_height: f32) -> f32 {
    match eye_height {
        e if e >= 1.5 => 1.8,
        e if e >= 1.0 => 1.5,
        _ => 0.6,
    }
}

impl HorseState {
    pub fn despawned(&self) -> bool {
        self.kind == HorseKind::TraderLlama && self.despawn_delay <= 0
    }

    fn can_parent(&self, cow: &crate::cow::Cow) -> bool {
        self.tame && !cow.age.baby() && cow.health >= cow.max_health && cow.in_love > 0
    }
}

impl CowEntity {
    pub(super) fn make_mad(&mut self) {
        let baby = self.cow.age.baby();
        let position = self.cow.body.position;
        let Some(horse) = self.horse.as_mut() else {
            return;
        };
        if horse.standing {
            return;
        }
        horse.stand_if_possible();
        if let Some(event) = horse_event(horse.kind, baby, "angry") {
            let pitch = voice_pitch(&mut self.random, baby);
            self.voices.push((
                Voice::Event(event, crate::horse::SOUND_VOLUME, pitch),
                position,
            ));
        }
    }

    pub(super) fn ride_tick(
        &mut self,
        world: &impl World,
        controls: &crate::controls::PlayerControls,
    ) {
        let fall = {
            let Some(horse) = self.horse.as_mut() else {
                return;
            };
            let cow = &mut self.cow;
            cow.body.trim_small_velocity();
            self.no_action_time = 0;
            cow.forward = 0.0;
            cow.sideways = 0.0;
            self.jumping = false;
            self.no_jump_delay = 0;
            cow.navigation.stop();
            let yaw = controls.yaw;
            cow.yaw = yaw;
            self.previous_yaw = yaw;
            self.look_control.head_yaw = yaw;
            self.look_control.pitch = controls.pitch * 0.5;
            self.body_rotation.body_yaw = yaw;
            let input = horse.ridden_input(cow.body.on_ground, controls.strafe, controls.forward);
            if cow.body.on_ground {
                if horse.jump_pending > 0.0 {
                    let scale = horse.jump_pending;
                    let power = cow.body.jump_power(world, horse.jump_strength, scale);
                    cow.body.velocity.y = f64::from(power);
                    cow.body.needs_sync = true;
                    if input.z > 0.0 {
                        let angle = f64::from(yaw * 0.017_453_292_f32);
                        let sin = minecraftoss_player::mth::sin(angle);
                        let cos = minecraftoss_player::mth::cos(angle);
                        cow.body.velocity += DVec3::new(
                            f64::from(-0.4 * sin * scale),
                            0.0,
                            f64::from(0.4 * cos * scale),
                        );
                    }
                }
                horse.jump_pending = 0.0;
            }
            let speed = self.effects.movement_speed(horse.movement_speed) as f32;
            let was_water = self.fluid.in_water();
            let mut fall = None;
            if was_water {
                cow.body.travel_water(world, input, yaw);
            } else if self.fluid.in_lava() {
                cow.body
                    .travel_lava(world, input, yaw, self.fluid.lava_height);
            } else if let Some(fallen) =
                cow.body
                    .travel_air_flying(world, input, speed, yaw, false, speed * 0.1)
            {
                fall = fall_damage(&cow.body, world, fallen, false, &mut self.voices);
            }
            let sounds = horse_movement_sounds(horse, cow.age.baby());
            play_movement(
                &mut cow.body,
                self.tick_count,
                &mut self.random,
                &mut self.voices,
                sounds,
            );
            self.body_rotation.tick(
                yaw,
                &mut self.look_control,
                self.previous_position,
                cow.body.position,
            );
            self.body_rotation.body_yaw = yaw;
            self.look_control.head_yaw = yaw;
            if !was_water {
                self.fluid =
                    FluidFrame::sample(world, cow.body.position, cow.body.width, cow.body.height);
                self.was_touching_water = cow.body.touching_water;
            }
            fall
        };
        if let Some(damage) = fall {
            self.hurt(damage, DamageSourceKind::Generic);
        }
    }

    pub fn note_attacker(&mut self, attacker: Target) {
        let tick = self.tick_count;
        if let Some(horse) = self.horse.as_mut() {
            horse.fight.hurt_by = Some((attacker, tick));
        }
    }

    pub(super) fn tick_trader_despawn(&mut self) {
        let Some(horse) = self.horse.as_mut() else {
            return;
        };
        if horse.kind != HorseKind::TraderLlama
            || horse.tame
            || (horse.leashed && horse.trader.is_none())
            || horse.ridden
            || self.cow.age.locked
            || self.cow.persistence_required
        {
            return;
        }
        horse.despawn_delay = match horse.trader {
            Some((_, delay)) => delay - 1,
            None => horse.despawn_delay - 1,
        };
    }

    pub(super) fn forget_old_attacker(&mut self) {
        let tick = self.tick_count;
        if let Some(horse) = self.horse.as_mut() {
            if horse.fight.hurt_by.is_some_and(|(_, at)| tick - at > 100) {
                horse.fight.hurt_by = None;
            }
        }
    }
}

impl EntityWorld {
    pub fn spawn_llama_spit(&mut self, spit: LlamaSpit) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.llama_spits.push(LlamaSpitEntity {
            id,
            previous_position: spit.position,
            spit,
        });
        self.order.push(EntityKey::LlamaSpit(id));
        id
    }

    pub fn llama_spits(&self) -> &[LlamaSpitEntity] {
        &self.llama_spits
    }

    pub fn llama_spit_mut(&mut self, id: u64) -> Option<&mut LlamaSpitEntity> {
        self.llama_spits.iter_mut().find(|e| e.id == id)
    }

    fn seen_mob(&self, id: u64, foe: Foe) -> Option<(Seen, Foe)> {
        let (body, health) = self.mob_body(id)?;
        let eye_height = body.height * 0.85;
        Some((
            Seen {
                target: Target::Mob(id),
                position: body.position,
                width: body.width,
                height: body.height,
                eye_height,
                alive: health > 0.0,
                attackable: true,
            },
            foe,
        ))
    }

    pub(super) fn horse_inputs(
        &self,
        index: usize,
        candidates: &[CowCandidate],
        players: &[PlayerCandidate],
    ) -> HorseInputs {
        let entity = &self.cows[index];
        let Some(horse) = &entity.horse else {
            return HorseInputs {
                mates: None,
                tempters: Vec::new(),
                llama: None,
            };
        };
        let parent = horse.can_parent(&entity.cow);
        let mates = candidates
            .iter()
            .zip(&self.cows)
            .map(|(candidate, other)| CowCandidate {
                alive: parent
                    && other.id != entity.id
                    && other.cow.health > 0.0
                    && other.horse.as_ref().is_some_and(|o| {
                        horse.kind.can_mate_with(o.kind) && o.can_parent(&other.cow)
                    }),
                ..*candidate
            })
            .collect();
        let tag = horse.kind.tempt_tag();
        let kind = horse.kind;
        let tempting = |item: &str| match &self.recipes {
            Some(recipes) => recipes.item_in_tag(tag, item),
            None if kind.is_llama() => item == "minecraft:hay_block",
            None => matches!(
                item,
                "minecraft:golden_carrot"
                    | "minecraft:golden_apple"
                    | "minecraft:enchanted_golden_apple"
            ),
        };
        let tempters = players
            .iter()
            .filter(|p| {
                self.player_controls(p.id)
                    .is_some_and(|controls| controls.holds(tempting))
            })
            .map(|p| p.id)
            .collect();
        let llama = horse.kind.is_llama().then(|| {
            let p = entity.cow.body.position;
            let near = |q: DVec3| q.distance_squared(p) < 24.0 * 24.0;
            let mut seen: Vec<(Seen, Foe)> = players
                .iter()
                .map(|player| {
                    (
                        Seen {
                            target: Target::Player(player.id),
                            position: player.position,
                            width: 0.6,
                            height: player_height(player.eye_height),
                            eye_height: player.eye_height,
                            alive: player.alive && !player.spectator,
                            attackable: player.attackable,
                        },
                        Foe::Player,
                    )
                })
                .collect();
            for wolf in self.wolves.iter().filter(|w| near(w.wolf.body.position)) {
                let tame = wolf.wolf.tame;
                seen.extend(self.seen_mob(wolf.id, Foe::Wolf { tame }));
            }
            if horse.kind == HorseKind::TraderLlama {
                for zombie in self.zombies.iter().filter(|z| {
                    z.zombie.kind != ZombieKind::ZombifiedPiglin && near(z.zombie.body.position)
                }) {
                    seen.extend(self.seen_mob(zombie.id, Foe::Zombie));
                }
            }
            let trader_attacker = horse.trader.and_then(|(trader, _)| {
                self.wandering_traders
                    .iter()
                    .find(|e| e.id == trader)
                    .and_then(|e| e.ai.state.hurt_by)
            });
            for target in [
                horse.fight.target,
                horse.fight.hurt_by.map(|(t, _)| t),
                trader_attacker.map(|(t, _)| t),
            ]
            .into_iter()
            .flatten()
            {
                if let Target::Mob(id) | Target::Villager(id) = target {
                    if !seen.iter().any(|(s, _)| s.target == Target::Mob(id)) {
                        if let Some((mut s, foe)) = self.seen_mob(id, Foe::Other) {
                            s.target = target;
                            seen.push((s, foe));
                        }
                    }
                }
            }
            let llamas = self
                .cows
                .iter()
                .filter_map(|other| {
                    let o = other.horse.as_ref().filter(|o| o.kind.is_llama())?;
                    Some(Caravaner {
                        id: other.id,
                        position: other.cow.body.position,
                        width: other.cow.body.width,
                        height: other.cow.body.height,
                        leashed: o.leashed,
                        head: o.fight.caravan_head,
                        has_tail: o.fight.caravan_tail.is_some(),
                        alive: other.cow.health > 0.0,
                    })
                })
                .collect();
            LlamaContext {
                id: entity.id,
                trader: horse.kind == HorseKind::TraderLlama,
                leashed: horse.leashed,
                width: entity.cow.body.width,
                height: entity.cow.body.height,
                eye_height: horse.kind.eye_height(entity.cow.age.baby()),
                fight: horse.fight.clone(),
                seen,
                llamas,
                trader_attacker,
            }
        });
        HorseInputs {
            mates: Some(mates),
            tempters,
            llama,
        }
    }

    pub(super) fn horse_ride(&self, index: usize) -> HorseRide {
        let entity = &self.cows[index];
        let id = entity.id;
        let first = self.passengers_of(id).first().copied();
        let carrying = first.is_some();
        let player = self
            .controlling_player(id)
            .and_then(|p| self.player_controls(p))
            .cloned();
        let crazy = entity
            .horse
            .as_ref()
            .filter(|h| !h.tame && carrying)
            .map(|h| crate::cow_ai::Crazy {
                rider: match first {
                    Some(Rider::Player(p)) => Some(p),
                    _ => None,
                },
                mob_controlled: matches!(first, Some(Rider::Mob(_))),
                temper: h.temper,
                max_temper: h.kind.max_temper(),
                target: None,
            });
        HorseRide {
            player,
            mob_controlled: self.controlling_mob(id).is_some(),
            carrying,
            crazy,
        }
    }

    pub(super) fn horse_outputs(&mut self, index: usize, out: HorseOutputs, world: &impl World) {
        let id = self.cows[index].id;
        if let Some(player) = out.tamed {
            let uuid = self.uuid_of(PLAYER_TARGET + player);
            if let Some(horse) = self.cows[index].horse.as_mut() {
                horse.tame = true;
                horse.owner = Some(uuid);
            }
            self.entity_events.push((id, 7));
        }
        if out.buck {
            self.eject_passengers(id, world);
            if let Some(index) = self.cows.iter().position(|e| e.id == id) {
                self.cows[index].make_mad();
            }
            self.entity_events.push((id, 6));
        }
        let Some(index) = self.cows.iter().position(|e| e.id == id) else {
            return;
        };
        for head in out.leave {
            if let Some(h) = self
                .cows
                .iter_mut()
                .find(|e| e.id == head)
                .and_then(|e| e.horse.as_mut())
            {
                if h.fight.caravan_tail == Some(id) {
                    h.fight.caravan_tail = None;
                }
            }
        }
        for head in out.join {
            if let Some(h) = self
                .cows
                .iter_mut()
                .find(|e| e.id == head)
                .and_then(|e| e.horse.as_mut())
            {
                h.fight.caravan_tail = Some(id);
            }
        }
        let Some(aim) = out.spit else {
            return;
        };
        let seed = self.fresh_seed();
        let entity = &mut self.cows[index];
        let Some(horse) = &entity.horse else {
            return;
        };
        let p = entity.cow.body.position;
        let eye = p.y + f64::from(horse.kind.eye_height(entity.cow.age.baby()));
        let from = crate::llama_spit::spat_from(
            p,
            entity.cow.body.width,
            entity.body_rotation.body_yaw,
            eye,
        );
        let mut random = LegacyRandom::new(seed);
        let spit = LlamaSpit::spit(id, from, aim, &mut random);
        let (a, b) = (entity.random.next_float(), entity.random.next_float());
        entity.voices.push((
            Voice::Event("entity.llama.spit", 1.0, 1.0 + (a - b) * 0.2),
            p,
        ));
        self.spawn_llama_spit(spit);
    }

    pub(super) fn tick_llama_spit(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
    ) {
        let Some(entity) = self.llama_spits.iter().find(|e| e.id == id && e.spit.alive) else {
            return;
        };
        let mut owner = entity.spit.owner;
        if owner.is_none() {
            owner = entity
                .spit
                .owner_uuid
                .and_then(|uuid| self.id_of_uuid(uuid));
        }
        let owner_box = owner.and_then(|o| self.mob_body(o)).map(|(body, _)| {
            let half = f64::from(body.width / 2.0);
            let p = body.position;
            (
                DVec3::new(p.x - half, p.y, p.z - half),
                DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
            )
        });
        let mut targets: Vec<SpitTarget> = self
            .order
            .iter()
            .filter_map(|key| {
                if matches!(
                    key,
                    EntityKey::Arrow(_)
                        | EntityKey::Potion(_)
                        | EntityKey::Fireball(_)
                        | EntityKey::ShulkerBullet(_)
                        | EntityKey::LlamaSpit(_)
                        | EntityKey::EvokerFangs(_)
                ) {
                    return None;
                }
                let mob = key.id();
                let (body, health) = self.mob_body(mob)?;
                if health <= 0.0 {
                    return None;
                }
                let half = f64::from(body.width / 2.0);
                let p = body.position;
                let (min, max) = self.shulker_box(mob).unwrap_or((
                    DVec3::new(p.x - half, p.y, p.z - half),
                    DVec3::new(p.x + half, p.y + f64::from(body.height), p.z + half),
                ));
                Some(SpitTarget {
                    id: mob,
                    min,
                    max,
                    owner: owner == Some(mob),
                })
            })
            .collect();
        targets.extend(
            players
                .iter()
                .filter(|p| self.players_pickable && p.alive && !p.spectator)
                .map(|p| SpitTarget {
                    id: PLAYER_TARGET + p.id,
                    min: p.position - DVec3::new(0.3, 0.0, 0.3),
                    max: p.position + DVec3::new(0.3, f64::from(player_height(p.eye_height)), 0.3),
                    owner: false,
                }),
        );
        let Some(entity) = self.llama_spits.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if owner.is_some() {
            entity.spit.owner = owner;
        }
        entity.previous_position = entity.spit.position;
        let velocity = entity.spit.velocity;
        let Some(SpitHit::Entity(target)) = entity.spit.tick(world, &targets, owner_box) else {
            return;
        };
        let Some(llama) = owner.filter(|o| self.mob_body(*o).is_some()) else {
            return;
        };
        if target >= PLAYER_TARGET {
            let player = target - PLAYER_TARGET;
            if players.iter().any(|p| p.id == player && p.attackable) {
                self.player_hits.push(PlayerHit {
                    player_id: player,
                    damage: crate::llama_spit::DAMAGE,
                    kind: PlayerHitKind::Spit { velocity },
                    source: Some(llama),
                });
            }
        } else {
            self.mob_shot_mob(llama, target, crate::llama_spit::DAMAGE, velocity);
        }
    }

    pub fn horse_interact(
        &mut self,
        id: u64,
        player: u64,
        inventory: &mut Inventory,
        hand: usize,
        infinite_materials: bool,
    ) -> InteractionResult {
        let sneaking = self.player_controls(player).is_some_and(|c| c.sneaking);
        let recipes = self.recipes.clone();
        let mut events = Vec::new();
        let Some(entity) = self.cows.iter_mut().find(|e| e.id == id) else {
            return InteractionResult::Pass;
        };
        let Some(kind) = entity.horse.as_ref().map(|h| h.kind) else {
            return InteractionResult::Pass;
        };
        let food = |item: &str| match &recipes {
            Some(recipes) => recipes.item_in_tag(kind.food_tag(), item),
            None => kind.eats(item),
        };
        let carpet = |item: &str| match &recipes {
            Some(recipes) => recipes.item_in_tag("minecraft:wool_carpets", item),
            None => is_carpet(item),
        };
        let baby = entity.cow.age.baby();
        let position = entity.cow.body.position;
        let item = inventory
            .slots
            .get(hand)
            .and_then(Option::as_ref)
            .map(|s| s.id.clone());
        let horse = entity.horse.as_mut().expect("a horse");
        let tamed = horse.tame;
        if !baby && tamed && sneaking {
            return InteractionResult::Pass;
        }
        let Some(item) = item else {
            return InteractionResult::Pass;
        };
        if baby && item == "minecraft:golden_dandelion" {
            let cow = &mut entity.cow;
            let (result, _) = crate::animal::interact(
                &mut cow.age,
                &mut cow.in_love,
                &mut cow.persistence_required,
                inventory,
                hand,
                infinite_materials,
                |_| false,
                false,
            );
            return result;
        }
        if food(&item) {
            let Some(feeding) = Feeding::of(kind, &item) else {
                return InteractionResult::Pass;
            };
            let cow = &mut entity.cow;
            let mut handled = false;
            if feeding.love && horse.tame && cow.age.ticks == 0 && cow.in_love <= 0 {
                cow.in_love = 600;
                events.push((id, 18));
                handled = true;
            }
            if cow.health < cow.max_health && feeding.heal > 0.0 {
                crate::effects::heal(&mut cow.health, cow.max_health, feeding.heal);
                handled = true;
            }
            if cow.age.baby() && feeding.age_up > 0 && !cow.age.locked {
                cow.age.grow(feeding.age_up, false);
                handled = true;
            }
            if feeding.temper > 0 && (handled || !horse.tame) && horse.temper < kind.max_temper() {
                horse.modify_temper(feeding.temper);
                handled = true;
            }
            if !handled {
                return InteractionResult::Pass;
            }
            if !kind.is_llama() {
                horse.mouth_counter = 1;
                horse.mouth_open = true;
            }
            if let Some(event) = horse_event(kind, baby, "eat") {
                let (a, b) = (entity.random.next_float(), entity.random.next_float());
                entity
                    .voices
                    .push((Voice::Event(event, 1.0, 1.0 + (a - b) * 0.2), position));
            }
            if !infinite_materials {
                crate::animal::consume_one(inventory, hand);
            }
            entity.cow.sync_dimensions();
            self.entity_events.extend(events);
            return InteractionResult::SuccessServer;
        }
        if !tamed {
            if !horse.standing {
                horse.stand_if_possible();
                if let Some(event) = horse_event(kind, baby, "angry") {
                    let pitch = voice_pitch(&mut entity.random, baby);
                    entity.voices.push((
                        Voice::Event(event, crate::horse::SOUND_VOLUME, pitch),
                        position,
                    ));
                }
            }
            return InteractionResult::SuccessPredicted;
        }
        if kind.is_chested() && !horse.chest && item == "minecraft:chest" {
            horse.chest = true;
            horse.resize_inventory();
            if let Some(event) = horse_event(kind, baby, "chest") {
                let (a, b) = (entity.random.next_float(), entity.random.next_float());
                entity
                    .voices
                    .push((Voice::Event(event, 1.0, (a - b) * 0.2 + 1.0), position));
            }
            if !infinite_materials {
                crate::animal::consume_one(inventory, hand);
            }
            return InteractionResult::SuccessPredicted;
        }
        if kind.saddleable() && !horse.saddled && !baby && item == "minecraft:saddle" {
            horse.saddled = true;
            entity
                .voices
                .push((Voice::Event("entity.horse.saddle", 1.0, 1.0), position));
            if !infinite_materials {
                crate::animal::consume_one(inventory, hand);
            }
            return InteractionResult::SuccessPredicted;
        }
        if baby {
            return InteractionResult::Pass;
        }
        if kind.wears_armor() && horse.body_item.is_none() && item.ends_with("_horse_armor") {
            horse.body_item = Some(item);
            entity
                .voices
                .push((Voice::Event("entity.horse.armor", 1.0, 1.0), position));
            if !infinite_materials {
                crate::animal::consume_one(inventory, hand);
            }
            return InteractionResult::SuccessPredicted;
        }
        if kind.is_llama() && horse.body_item.is_none() && carpet(&item) {
            horse.body_item = Some(item);
            entity
                .voices
                .push((Voice::Event("entity.llama.swag", 1.0, 1.0), position));
            if !infinite_materials {
                crate::animal::consume_one(inventory, hand);
            }
            return InteractionResult::SuccessPredicted;
        }
        InteractionResult::Pass
    }

    pub fn horse_opens_inventory(&self, id: u64, player: u64) -> bool {
        let Some(entity) = self.cows.iter().find(|e| e.id == id) else {
            return false;
        };
        let riding = self.vehicle_of(Rider::Player(player));
        entity.horse.as_ref().is_some_and(|h| h.tame)
            && entity.cow.health > 0.0
            && !entity.cow.age.baby()
            && match riding {
                Some(vehicle) => vehicle == id,
                None => {
                    self.passengers_of(id).is_empty()
                        && self.player_controls(player).is_some_and(|c| c.sneaking)
                }
            }
    }

    pub fn set_mount_inventory(
        &mut self,
        id: u64,
        saddled: bool,
        body: Option<String>,
        items: Vec<Option<ItemStack>>,
    ) {
        let Some(entity) = self.cows.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let position = entity.cow.body.position;
        let Some(horse) = entity.horse.as_mut() else {
            return;
        };
        let mut sounds = Vec::new();
        if horse.kind.saddleable() && horse.saddled != saddled {
            horse.saddled = saddled;
            if saddled {
                sounds.push("entity.horse.saddle");
            }
        }
        if horse.body_item != body {
            if body.is_some() {
                sounds.push(if horse.kind.is_llama() {
                    "entity.llama.swag"
                } else {
                    "entity.horse.armor"
                });
            }
            horse.body_item = body;
        }
        if horse.chest && items.len() == horse.inventory.len() {
            horse.inventory = items;
        }
        for event in sounds {
            entity
                .voices
                .push((Voice::Event(event, 1.0, 1.0), position));
        }
    }

    pub fn horse_rides_on_use(&self, id: u64, player: u64, held: Option<&str>) -> bool {
        let Some(entity) = self.cows.iter().find(|e| e.id == id) else {
            return false;
        };
        let Some(horse) = &entity.horse else {
            return false;
        };
        if entity.cow.health <= 0.0
            || entity.cow.age.baby()
            || !self.passengers_of(id).is_empty()
            || self.vehicle_of(Rider::Player(player)).is_some()
        {
            return false;
        }
        let sneaking = self.player_controls(player).is_some_and(|c| c.sneaking);
        if horse.tame && sneaking {
            return false;
        }
        let Some(item) = held else {
            return true;
        };
        let kind = horse.kind;
        let food = match &self.recipes {
            Some(recipes) => recipes.item_in_tag(kind.food_tag(), item),
            None => kind.eats(item),
        };
        let carpet = match &self.recipes {
            Some(recipes) => recipes.item_in_tag("minecraft:wool_carpets", item),
            None => is_carpet(item),
        };
        !(food
            || !horse.tame
            || kind.is_chested() && !horse.chest && item == "minecraft:chest"
            || kind.saddleable() && !horse.saddled && item == "minecraft:saddle"
            || kind.wears_armor() && horse.body_item.is_none() && item.ends_with("_horse_armor")
            || kind.is_llama() && horse.body_item.is_none() && carpet)
    }

    pub(super) fn breed_horses(&mut self, index: usize, partner: u64) -> bool {
        let Some(kind) = self.cows[index].horse.as_ref().map(|h| h.kind) else {
            return false;
        };
        let Some(partner_index) = self.cows.iter().position(|e| e.id == partner) else {
            return true;
        };
        let Some(mate) = self.cows[partner_index].horse.clone() else {
            return true;
        };
        for i in [index, partner_index] {
            self.cows[i].cow.age.set(6000);
            self.cows[i].cow.in_love = 0;
        }
        let child_kind = kind.offspring(mate.kind);
        let entity = &mut self.cows[index];
        let parent = entity.horse.clone().expect("a horse");
        let random = &mut entity.random;
        let mut child = HorseState::new(child_kind);
        if child_kind == HorseKind::Horse {
            let coat = match random.next_int(9) {
                roll if roll < 4 => parent.variant & 255,
                roll if roll < 8 => mate.variant & 255,
                _ => random.next_int(HORSE_COATS as u32) as i32,
            };
            let markings = match random.next_int(5) {
                roll if roll < 2 => (parent.variant >> 8) & 255,
                roll if roll < 4 => (mate.variant >> 8) & 255,
                _ => random.next_int(HORSE_MARKINGS as u32) as i32,
            };
            child.variant = (coat & 255) | ((markings << 8) & 65280);
        }
        child.max_health = crate::horse::offspring_attribute(
            f64::from(parent.max_health),
            f64::from(mate.max_health),
            crate::horse::MIN_HEALTH,
            crate::horse::MAX_HEALTH,
            random,
        ) as f32;
        child.jump_strength = crate::horse::offspring_attribute(
            parent.jump_strength,
            mate.jump_strength,
            crate::horse::MIN_JUMP_STRENGTH,
            crate::horse::MAX_JUMP_STRENGTH,
            random,
        );
        child.movement_speed = crate::horse::offspring_attribute(
            parent.movement_speed,
            mate.movement_speed,
            crate::horse::MIN_MOVEMENT_SPEED,
            crate::horse::MAX_MOVEMENT_SPEED,
            random,
        );
        if child_kind.is_llama() {
            let mut strength =
                random.next_int(parent.strength.max(mate.strength).max(1) as u32) as i32 + 1;
            if random.next_float() < 0.03 {
                strength += 1;
            }
            child.set_strength(strength);
            child.variant = if random.next_boolean() {
                parent.variant
            } else {
                mate.variant
            }
            .clamp(0, LLAMA_VARIANTS - 1);
        }
        let mut cow = crate::cow::Cow::new(entity.cow.body.position);
        cow.age.set(crate::age::Age::BABY_START);
        cow.health = child.max_health;
        cow.persistence_required = child_kind == HorseKind::TraderLlama;
        self.spawn_horse(cow, child, false);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};

    struct Floor;
    impl World for Floor {
        fn block(&self, (_, y, _): Pos) -> Option<Block> {
            (y < 64).then(|| Block::new("minecraft:grass_block"))
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
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

    fn equine(world: &mut EntityWorld, kind: HorseKind, at: DVec3) -> u64 {
        let mut cow = crate::cow::Cow::new(at);
        let mut horse = HorseState::new(kind);
        horse.max_health = 20.0;
        cow.health = 20.0;
        if kind.is_llama() {
            horse.set_strength(3);
        }
        world.spawn_horse(cow, horse, false)
    }

    fn holding(item: &str) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new(item, 8));
        inventory
    }

    #[test]
    fn mules_and_llamas_take_their_sizes_health_and_voices() {
        let mut world = EntityWorld::default();
        let mule = equine(&mut world, HorseKind::Mule, DVec3::new(0.5, 64.0, 0.5));
        let llama = equine(&mut world, HorseKind::Llama, DVec3::new(4.5, 64.0, 0.5));
        let trader = equine(
            &mut world,
            HorseKind::TraderLlama,
            DVec3::new(8.5, 64.0, 0.5),
        );
        assert_eq!(world.entity_type(mule), Some("minecraft:mule"));
        assert_eq!(world.entity_type(llama), Some("minecraft:llama"));
        assert_eq!(world.entity_type(trader), Some("minecraft:trader_llama"));
        let mut body = |id: u64| {
            world
                .cow_mut(id)
                .map(|e| (e.cow.body.width, e.cow.body.height))
                .unwrap()
        };
        assert_eq!(body(mule), (1.3964844, 1.6));
        assert_eq!(body(llama), (0.9, 1.87));
        assert_eq!(HorseKind::Llama.eye_height(false), 1.7765);
        assert_eq!(HorseKind::Mule.eye_height(true), 0.76);
        assert_eq!(HorseKind::Llama.sound_family(true), "llama");
        assert_eq!(HorseKind::Mule.sound_family(false), "mule");
        assert_eq!(
            world.cow_mut(llama).unwrap().running_goals(),
            Vec::<&str>::new()
        );
        assert!(!HorseKind::Llama.can_rear() && HorseKind::Mule.can_rear());
        assert_eq!(HorseKind::Llama.max_temper(), 30);
        assert_eq!(HorseKind::Donkey.max_temper(), 100);
    }

    #[test]
    fn a_tame_horse_and_a_tame_donkey_in_love_have_a_mule() {
        let mut world = EntityWorld::default();
        let horse = equine(&mut world, HorseKind::Horse, DVec3::new(0.5, 64.0, 0.5));
        let donkey = equine(&mut world, HorseKind::Donkey, DVec3::new(2.5, 64.0, 0.5));
        let mule = equine(&mut world, HorseKind::Mule, DVec3::new(-3.5, 64.0, 0.5));
        let mut carrots = holding("minecraft:golden_carrot");
        for id in [horse, donkey, mule] {
            assert_eq!(
                world.horse_interact(id, 0, &mut carrots, 0, false),
                InteractionResult::SuccessServer
            );
            let e = world.cow_mut(id).unwrap();
            assert_eq!(
                (e.cow.in_love, e.horse.as_ref().unwrap().temper),
                (0, 5),
                "a wild one only warms up"
            );
            e.horse.as_mut().unwrap().tame = true;
        }
        for id in [horse, donkey, mule] {
            assert_eq!(
                world.horse_interact(id, 0, &mut carrots, 0, false),
                InteractionResult::SuccessServer
            );
            assert_eq!(world.cow_mut(id).unwrap().cow.in_love, 600);
        }
        assert_eq!(carrots.slots[0].as_ref().unwrap().count, 2);
        let players = [player(0, DVec3::new(20.5, 64.0, 20.5))];
        let before = world.cows().len();
        for _ in 0..200 {
            world.tick_with_players(&mut Floor, &players);
            if world.cows().len() > before {
                break;
            }
        }
        let kinds: Vec<_> = world
            .cows()
            .iter()
            .filter_map(|e| e.horse.as_ref().map(|h| (h.kind, e.cow.age.baby())))
            .collect();
        assert!(kinds.contains(&(HorseKind::Mule, true)), "{kinds:?}");
        assert_eq!(
            kinds.iter().filter(|(_, baby)| *baby).count(),
            1,
            "the mule itself never breeds"
        );
        let foal = world.cows().iter().find(|e| e.cow.age.baby()).unwrap();
        let state = foal.horse.as_ref().unwrap();
        assert!(
            (15.0..=30.0).contains(&state.max_health),
            "{}",
            state.max_health
        );
        assert!(
            (crate::horse::MIN_MOVEMENT_SPEED..=crate::horse::MAX_MOVEMENT_SPEED)
                .contains(&state.movement_speed)
        );
        assert_eq!(world.cow_mut(horse).unwrap().cow.age.ticks, 6000);
        assert_eq!(world.cow_mut(mule).unwrap().cow.in_love > 0, true);
    }

    #[test]
    fn llamas_breed_llamas_with_strength_from_their_parents() {
        let mut world = EntityWorld::default();
        let a = equine(&mut world, HorseKind::Llama, DVec3::new(0.5, 64.0, 0.5));
        let b = equine(&mut world, HorseKind::Llama, DVec3::new(2.5, 64.0, 0.5));
        for (id, variant) in [(a, 1), (b, 3)] {
            let e = world.cow_mut(id).unwrap();
            let h = e.horse.as_mut().unwrap();
            h.tame = true;
            h.variant = variant;
        }
        let mut hay = holding("minecraft:hay_block");
        for id in [a, b] {
            world.horse_interact(id, 0, &mut hay, 0, false);
        }
        assert!(world.breed_horses(0, b));
        let child = world.cows().last().unwrap();
        let state = child.horse.as_ref().unwrap();
        assert_eq!(state.kind, HorseKind::Llama);
        assert!(child.cow.age.baby());
        assert!((1..=4).contains(&state.strength), "{}", state.strength);
        assert!(state.variant == 1 || state.variant == 3);
    }

    #[test]
    fn a_tame_llama_takes_a_chest_and_a_carpet_and_a_wild_one_gets_angry() {
        let mut world = EntityWorld::default();
        let llama = equine(&mut world, HorseKind::Llama, DVec3::new(0.5, 64.0, 0.5));
        let mut chest = holding("minecraft:chest");
        assert_eq!(
            world.horse_interact(llama, 0, &mut chest, 0, false),
            InteractionResult::SuccessPredicted
        );
        assert!(!world.cow_mut(llama).unwrap().horse.as_ref().unwrap().chest);
        let sounds = world.take_sounds();
        assert!(
            sounds.iter().any(|s| s.event == "entity.llama.angry"),
            "{sounds:?}"
        );
        world.cow_mut(llama).unwrap().horse.as_mut().unwrap().tame = true;
        assert_eq!(
            world.horse_interact(llama, 0, &mut chest, 0, false),
            InteractionResult::SuccessPredicted
        );
        let mut carpet = holding("minecraft:white_carpet");
        assert_eq!(
            world.horse_interact(llama, 0, &mut carpet, 0, false),
            InteractionResult::SuccessPredicted
        );
        assert_eq!(
            world.horse_interact(llama, 0, &mut carpet, 0, false),
            InteractionResult::Pass,
            "one carpet"
        );
        let horse = world.cow_mut(llama).unwrap().horse.clone().unwrap();
        assert!(horse.chest);
        assert_eq!(horse.body_item.as_deref(), Some("minecraft:white_carpet"));
        assert_eq!(horse.inventory_columns(), 3);
        assert_eq!(chest.slots[0].as_ref().unwrap().count, 7);
        let dropped = horse_death_equipment(&horse, true);
        assert_eq!(
            dropped.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["minecraft:white_carpet", "minecraft:chest"]
        );
    }

    #[test]
    fn a_llama_spits_at_the_player_who_hit_it_once() {
        let mut world = EntityWorld::default();
        world.set_players_pickable(true);
        let llama = equine(&mut world, HorseKind::Llama, DVec3::new(0.5, 64.0, 0.5));
        let players = [player(0, DVec3::new(0.5, 64.0, 8.5))];
        for _ in 0..5 {
            world.tick_with_players(&mut Floor, &players);
        }
        world
            .cow_mut(llama)
            .unwrap()
            .note_attacker(Target::Player(0));
        let mut spat = 0;
        let mut hits = Vec::new();
        for _ in 0..200 {
            world.tick_with_players(&mut Floor, &players);
            spat = spat.max(world.llama_spits().len());
            hits.extend(world.take_player_hits());
        }
        assert!(spat > 0, "it spat");
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(
            (hits[0].player_id, hits[0].damage, hits[0].source),
            (0, 1.0, Some(llama))
        );
        assert!(matches!(hits[0].kind, PlayerHitKind::Spit { .. }));
        assert!(world.llama_spits().is_empty(), "the spit is gone");
        assert_eq!(
            world
                .cow_mut(llama)
                .unwrap()
                .horse
                .as_ref()
                .unwrap()
                .fight
                .target,
            None
        );
    }

    #[test]
    fn a_trader_llama_leaves_when_its_delay_runs_out() {
        let mut world = EntityWorld::default();
        let trader = equine(
            &mut world,
            HorseKind::TraderLlama,
            DVec3::new(0.5, 64.0, 0.5),
        );
        let tame = equine(
            &mut world,
            HorseKind::TraderLlama,
            DVec3::new(4.5, 64.0, 0.5),
        );
        world.cow_mut(tame).unwrap().horse.as_mut().unwrap().tame = true;
        world
            .cow_mut(trader)
            .unwrap()
            .horse
            .as_mut()
            .unwrap()
            .despawn_delay = 3;
        let players = [player(0, DVec3::new(0.5, 64.0, 8.5))];
        for _ in 0..3 {
            world.tick_with_players(&mut Floor, &players);
        }
        assert!(world.cow_mut(trader).is_none());
        assert!(world.cow_mut(tame).is_some());
        assert_eq!(
            world
                .cow_mut(tame)
                .unwrap()
                .horse
                .as_ref()
                .unwrap()
                .despawn_delay,
            crate::horse::TRADER_DESPAWN_DELAY
        );
    }

    fn ridden(world: &mut EntityWorld, kind: HorseKind, tame: bool, saddled: bool) -> u64 {
        let id = equine(world, kind, DVec3::new(0.5, 64.0, 0.5));
        let horse = world.cow_mut(id).unwrap().horse.as_mut().unwrap();
        horse.tame = tame;
        horse.saddled = saddled;
        world.set_player_uuid(0, 77);
        assert!(world.start_riding(Rider::Player(0), id, &Floor));
        id
    }

    fn steer(world: &mut EntityWorld, yaw: f32, forward: f32, jumping: bool) {
        world.set_player_controls(&[crate::controls::PlayerControls {
            id: 0,
            yaw,
            forward,
            jumping,
            ..Default::default()
        }]);
    }

    #[test]
    fn a_wild_horse_is_tamed_by_its_rider_when_its_temper_allows() {
        let mut world = EntityWorld::default();
        let id = ridden(&mut world, HorseKind::Horse, false, false);
        world.cow_mut(id).unwrap().horse.as_mut().unwrap().temper = 100;
        steer(&mut world, 0.0, 0.0, false);
        let players = [player(0, DVec3::new(0.5, 65.0, 0.5))];
        let mut ticks = 0;
        while !world.cow_mut(id).unwrap().horse.as_ref().unwrap().tame {
            world.tick_with_players(&mut Floor, &players);
            ticks += 1;
            assert!(ticks < 4000, "never tamed");
        }
        let horse = world.cow_mut(id).unwrap().horse.clone().unwrap();
        assert_eq!(horse.owner, Some(77));
        assert_eq!(
            world.vehicle_of(Rider::Player(0)),
            Some(id),
            "the rider stays on"
        );
        assert!(world.take_entity_events().contains(&(id, 7)));
        assert!(world.take_player_dismounts().is_empty());
    }

    #[test]
    fn a_wild_horse_bucks_its_rider_off_and_grows_tamer() {
        let mut world = EntityWorld::default();
        let id = ridden(&mut world, HorseKind::Donkey, false, false);
        steer(&mut world, 0.0, 0.0, false);
        let players = [player(0, DVec3::new(0.5, 65.0, 0.5))];
        let mut ticks = 0;
        while world.vehicle_of(Rider::Player(0)).is_some() {
            world.tick_with_players(&mut Floor, &players);
            ticks += 1;
            assert!(ticks < 4000, "never bucked");
        }
        let entity = world.cow_mut(id).unwrap();
        let horse = entity.horse.clone().unwrap();
        assert!(!horse.tame && horse.temper == 5 && horse.standing);
        let feet = entity.cow.body.position;
        let dismounts = world.take_player_dismounts();
        assert_eq!(dismounts.len(), 1);
        let (player, vehicle, spot) = dismounts[0];
        assert_eq!((player, vehicle), (0, id));
        let spot = spot.expect("thrown beside it");
        assert!(
            (spot.y - 64.0).abs() < 1.0e-9 && spot.distance(feet) > 0.9,
            "{spot} {feet}"
        );
        assert!(world.take_entity_events().contains(&(id, 6)));
    }

    #[test]
    fn a_tamed_horse_takes_a_saddle_and_its_rider_steers_it() {
        let mut world = EntityWorld::default();
        let id = equine(&mut world, HorseKind::Horse, DVec3::new(0.5, 64.0, 0.5));
        let mut saddle = holding("minecraft:saddle");
        assert!(
            !world.horse_rides_on_use(id, 0, Some("minecraft:saddle")),
            "a wild horse gets angry"
        );
        assert_eq!(
            world.horse_interact(id, 0, &mut saddle, 0, false),
            InteractionResult::SuccessPredicted
        );
        assert!(!world.cow_mut(id).unwrap().horse.as_ref().unwrap().saddled);
        world
            .cow_mut(id)
            .unwrap()
            .horse
            .as_mut()
            .unwrap()
            .clear_standing();
        world.cow_mut(id).unwrap().horse.as_mut().unwrap().tame = true;
        assert!(!world.horse_rides_on_use(id, 0, Some("minecraft:saddle")));
        assert_eq!(
            world.horse_interact(id, 0, &mut saddle, 0, false),
            InteractionResult::SuccessPredicted
        );
        assert!(world.cow_mut(id).unwrap().horse.as_ref().unwrap().saddled);
        assert_eq!(saddle.slots[0].as_ref().unwrap().count, 7);
        assert!(
            world.horse_rides_on_use(id, 0, Some("minecraft:saddle")),
            "a second saddle just mounts"
        );
        assert!(world.horse_rides_on_use(id, 0, None));
        let mut armor = holding("minecraft:iron_horse_armor");
        assert!(!world.horse_rides_on_use(id, 0, Some("minecraft:iron_horse_armor")));
        world.horse_interact(id, 0, &mut armor, 0, false);
        assert_eq!(
            world
                .cow_mut(id)
                .unwrap()
                .horse
                .as_ref()
                .unwrap()
                .body_item
                .as_deref(),
            Some("minecraft:iron_horse_armor")
        );
        assert!(world.start_riding(Rider::Player(0), id, &Floor));
        assert!(
            !world.horse_rides_on_use(id, 1, None),
            "one rider at a time"
        );
        assert_eq!(world.controlling_player(id), Some(0));
        let seat = world.seat(Rider::Player(0)).unwrap();
        assert!(
            (seat.y - (64.0 + f64::from(1.44375_f32) - 0.6)).abs() < 1.0e-6,
            "{seat}"
        );
        steer(&mut world, 90.0, 0.98, false);
        let players = [player(0, seat)];
        for _ in 0..40 {
            world.tick_with_players(&mut Floor, &players);
        }
        let entity = world.cow_mut(id).unwrap();
        assert_eq!(entity.cow.yaw, 90.0);
        let moved = entity.cow.body.position;
        assert!(moved.x < -5.0 && (moved.z - 0.5).abs() < 1.0e-6, "{moved}");
        world.cow_mut(id).unwrap().horse.as_mut().unwrap().saddled = false;
        assert_eq!(
            world.controlling_player(id),
            None,
            "without a saddle it walks its own way"
        );
    }

    fn jump_peak(strength: f64) -> f64 {
        let mut world = EntityWorld::default();
        let id = ridden(&mut world, HorseKind::Horse, true, true);
        world
            .cow_mut(id)
            .unwrap()
            .horse
            .as_mut()
            .unwrap()
            .jump_strength = strength;
        let players = [player(0, DVec3::new(0.5, 65.0, 0.5))];
        steer(&mut world, 0.0, 0.0, false);
        for _ in 0..5 {
            world.tick_with_players(&mut Floor, &players);
        }
        steer(&mut world, 0.0, 0.0, true);
        for _ in 0..11 {
            world.tick_with_players(&mut Floor, &players);
        }
        assert!((world.ride_jump_scale(0).unwrap() - 1.0).abs() < 1.0e-6);
        steer(&mut world, 0.0, 0.0, false);
        let mut peak = 0.0_f64;
        for _ in 0..40 {
            world.tick_with_players(&mut Floor, &players);
            peak = peak.max(world.cow_mut(id).unwrap().cow.body.position.y - 64.0);
        }
        peak
    }

    #[test]
    fn a_full_charge_jumps_as_high_as_the_horse_s_jump_strength_allows() {
        let low = jump_peak(0.5);
        let high = jump_peak(1.0);
        assert!((1.4..2.0).contains(&low), "{low}");
        assert!((5.0..6.5).contains(&high), "{high}");
        let mut world = EntityWorld::default();
        let id = ridden(&mut world, HorseKind::Llama, true, false);
        assert_eq!(world.ride_jump_scale(0), None, "a llama has no jump bar");
        assert_eq!(world.controlling_player(id), None);
        let point = world.passenger_point(id).unwrap();
        assert!(
            (point.y - f64::from(1.37_f32)).abs() < 1.0e-6
                && (point.z + f64::from(0.3_f32)).abs() < 1.0e-6,
            "{point}"
        );
    }

    #[test]
    fn a_tame_horse_s_inventory_opens_on_a_crouching_click_or_from_its_saddle_and_the_screen_sets_its_slots()
     {
        let mut world = EntityWorld::default();
        let wild = equine(&mut world, HorseKind::Donkey, DVec3::new(0.5, 64.0, 0.5));
        let donkey = equine(&mut world, HorseKind::Donkey, DVec3::new(4.5, 64.0, 0.5));
        world.cow_mut(donkey).unwrap().horse.as_mut().unwrap().tame = true;
        assert!(
            !world.horse_opens_inventory(donkey, 0),
            "a plain click rides it"
        );
        world.set_player_controls(&[crate::controls::PlayerControls {
            id: 0,
            sneaking: true,
            ..Default::default()
        }]);
        assert!(world.horse_opens_inventory(donkey, 0));
        assert!(
            !world.horse_opens_inventory(wild, 0),
            "a wild one has no inventory for you"
        );
        world.set_player_controls(&[crate::controls::PlayerControls {
            id: 0,
            ..Default::default()
        }]);
        assert!(world.start_riding(Rider::Player(0), donkey, &Floor));
        assert!(
            world.horse_opens_inventory(donkey, 0),
            "its rider opens it with the inventory key"
        );
        let mut chest = holding("minecraft:chest");
        world.drop_player(0);
        world.horse_interact(donkey, 0, &mut chest, 0, false);
        assert_eq!(
            world
                .cow_mut(donkey)
                .unwrap()
                .horse
                .as_ref()
                .unwrap()
                .inventory
                .len(),
            15
        );
        let mut items = vec![None; 15];
        items[14] = Some(ItemStack::new("minecraft:wheat", 9));
        world.set_mount_inventory(
            donkey,
            true,
            Some("minecraft:iron_horse_armor".to_owned()),
            items.clone(),
        );
        let horse = world.cow_mut(donkey).unwrap().horse.clone().unwrap();
        assert!(horse.saddled && horse.inventory == items);
        assert_eq!(
            horse.body_item.as_deref(),
            Some("minecraft:iron_horse_armor")
        );
        world.set_mount_inventory(donkey, true, None, vec![None; 3]);
        assert_eq!(
            world
                .cow_mut(donkey)
                .unwrap()
                .horse
                .as_ref()
                .unwrap()
                .inventory,
            items,
            "a stale screen of another size is ignored"
        );
        let drops =
            horse_death_equipment(&world.cow_mut(donkey).unwrap().horse.clone().unwrap(), true);
        let ids: Vec<_> = drops.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            ["minecraft:saddle", "minecraft:wheat", "minecraft:chest"]
        );
    }
}
