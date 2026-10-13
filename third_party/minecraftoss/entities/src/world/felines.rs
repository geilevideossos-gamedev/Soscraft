use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::feline::{self, Feline, FelineKind};
use crate::feline_ai::{FelineMate, FelinePlayer};
use crate::monster_ai::{MonsterKind, Target};
use minecraftoss_player::inventory::Inventory;

const SOUNDS: MovementSounds = MovementSounds::creature(None);

#[derive(Clone, Default)]
struct Surroundings {
    owner: Option<u64>,
    people: Vec<FelinePlayer>,
    owner_hurt: bool,
    owner_sleeping: Option<(i32, i32, i32)>,
    owner_sleep_timer: i32,
    resting: Vec<(DVec3, f32, f32)>,
    gift_chance: f32,
    mates: Vec<FelineMate>,
    sleeper_near: bool,
}

#[derive(Default)]
struct Step {
    struck: Option<(Target, DVec3)>,
    landed: Option<f64>,
    gift: Option<(DVec3, f32)>,
    breed_with: Option<u64>,
}

#[derive(Clone)]
pub struct FelineEntity {
    pub id: u64,
    pub feline: Feline,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub ai: Box<MonsterAi>,
    pub effects: crate::effects::MobEffects,
    pub last_damage: Option<(&'static str, i64)>,
}

impl FelineEntity {
    pub fn position(&self) -> DVec3 {
        self.feline.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn tempted(&self) -> bool {
        self.ai.state.feline.tempt.running
    }

    pub fn running_goals(&self) -> Vec<&'static str> {
        self.ai.running_goals()
    }

    pub fn hurt_from(
        &mut self,
        amount: f32,
        kind: &'static str,
        attacker: Option<Target>,
        game_time: i64,
    ) -> DamageResult {
        if self.feline.health > 0.0 && !self.feline.damage.dead {
            self.no_action_time = 0;
        }
        let max = self.feline.max_health();
        let result = self
            .feline
            .damage
            .hurt_generic(&mut self.feline.health, max, amount);
        let position = self.position();
        self.feline
            .damage
            .place_death(result, position, self.feline.body.fire_ticks > 0);
        if result.applied {
            self.feline.in_love = 0;
            self.last_damage = Some((kind, game_time));
            if let Some(attacker) = attacker {
                self.ai.state.hurt_by = Some((attacker, self.tick_count));
            }
            if result.full {
                if !result.died {
                    self.ambient_sound_time = -self.feline.ambient_interval();
                }
                let sounds = self.feline.sounds();
                let pitch = voice_pitch(&mut self.random, self.feline.baby());
                let event = if result.died {
                    sounds.death
                } else {
                    sounds.hurt
                };
                self.voices
                    .push((Voice::Event(event, 1.0, pitch), position));
            }
        }
        result
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        self.hurt_from(amount, "minecraft:generic", None, 0)
    }

    fn ambient(&mut self) {
        if (self.random.next_int(1000) as i32) >= self.ambient_sound_time {
            self.ambient_sound_time += 1;
            return;
        }
        self.ambient_sound_time = -self.feline.ambient_interval();
        let sounds = self.feline.sounds();
        let event = match self.feline.kind {
            FelineKind::Ocelot => sounds.ambient,
            FelineKind::Cat if !self.feline.tame => sounds.stray_ambient,
            FelineKind::Cat if self.feline.in_love > 0 => sounds.purr,
            FelineKind::Cat if self.random.next_int(4) == 0 => sounds.purreow,
            FelineKind::Cat => sounds.ambient,
        };
        let pitch = voice_pitch(&mut self.random, self.feline.baby());
        self.voices
            .push((Voice::Event(event, 1.0, pitch), self.position()));
    }

    fn tick_ai(&mut self, world: &impl World, game_time: i64, around: Surroundings) -> Step {
        let (max_health, eye) = (self.feline.max_health(), self.feline.eye_height());
        let (cat, tame, trusting, ordered_to_sit, sitting) = (
            self.feline.is_cat(),
            self.feline.tame,
            self.feline.trusting,
            self.feline.ordered_to_sit,
            self.feline.sitting,
        );
        let (lying, relaxed, in_love) = (
            self.feline.lying,
            self.feline.relax_state_one,
            self.feline.in_love > 0,
        );
        let ai = &mut self.ai;
        let body = &mut self.feline.body;
        if ai.no_jump_delay > 0 {
            ai.no_jump_delay -= 1;
        }
        body.trim_small_velocity();
        ai.state.body = body.clone();
        ai.state.health = self.feline.health;
        ai.state.max_health = max_health;
        ai.state.game_time = game_time;
        ai.state.tick_count = self.tick_count;
        ai.state.on_fire = body.fire_ticks > 0;
        ai.state.eye_height = eye;
        let state = &mut ai.state.feline;
        state.cat = cat;
        state.tame = tame;
        state.trusting = trusting;
        state.owner = around.owner;
        state.ordered_to_sit = ordered_to_sit;
        state.sitting = sitting;
        state.people = around.people;
        state.owner_hurt = around.owner_hurt;
        state.owner_sleeping = around.owner_sleeping;
        state.owner_sleep_timer = around.owner_sleep_timer;
        state.resting = around.resting;
        state.gift_chance = around.gift_chance;
        state.mates = around.mates;
        state.lying = lying;
        state.relaxed = relaxed;
        state.in_love = in_love;
        state.teleported = false;
        state.gift = false;
        state.last_damage = self
            .last_damage
            .filter(|&(_, at)| game_time - at <= 40)
            .map(|(kind, _)| kind);
        ai.state.random = std::mem::take(&mut self.random);
        self.no_action_time += 1;
        let position = body.position;
        let fluid = FluidFrame::sample(world, position, body.width, body.height);
        ai.state.fluid = fluid;
        ai.state.no_action_time = self.no_action_time;
        let full = self.tick_count <= 1 || (self.tick_count + self.id as i32) % 2 == 0;
        ai.tick_goals(world, full);
        let mut step = Step {
            struck: ai.state.attack.take().map(|target| (target, body.position)),
            breed_with: ai.state.feline.breed_with.take(),
            ..Step::default()
        };
        if std::mem::take(&mut ai.state.feline.gift) {
            let p = ai.state.body.position;
            let block = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
            let x = f64::from(block.0 + ai.state.random.next_int(11) as i32 - 5);
            let y = f64::from(block.1 + ai.state.random.next_int(5) as i32 - 2);
            let z = f64::from(block.2 + ai.state.random.next_int(11) as i32 - 5);
            if crate::feline_ai::random_teleport(&mut ai.state.body, world, x, y, z) {
                ai.state.navigation.stop();
                ai.state.feline.teleported = true;
            }
            let q = ai.state.body.position;
            let at = DVec3::new(q.x.floor(), q.y.floor(), q.z.floor());
            step.gift = Some((at, ai.body_rotation.body_yaw));
        }
        body.position = ai.state.body.position;
        self.feline.sitting = ai.state.feline.sitting;
        self.feline.lying = ai.state.feline.lying;
        self.feline.relax_state_one = ai.state.feline.relaxed;
        if ai.state.feline.teleported {
            self.previous_position = body.position;
        }
        if let Some(leap) = ai.state.leap.take() {
            body.velocity = leap;
        }
        let position = body.position;
        let (can_update, surface) =
            crate::navigation::ground_view(world, body, fluid, ai.state.walk.can_float);
        if let Some((wanted, speed)) = ai
            .state
            .navigation
            .tick_in(world, position, can_update, surface, body.width, ai.speed)
        {
            ai.move_control.set_wanted_position(wanted, speed);
        }
        let speed_modifier = ai
            .move_control
            .has_wanted()
            .then_some(ai.move_control.speed_modifier);
        self.feline.crouching = speed_modifier == Some(feline::TEMPT_SPEED);
        self.feline.sprinting = speed_modifier == Some(feline::SPRINT_SPEED);
        self.random = std::mem::take(&mut ai.state.random);
        let obstacle_top = crate::control::obstacle_top(world, position);
        let sprint = if self.feline.sprinting {
            1.0 + feline::SPRINT_SPEED_BONUS
        } else {
            1.0
        };
        let speed = self.effects.movement_speed(feline::MOVEMENT_SPEED) * sprint;
        let control = ai.move_control.tick(
            position,
            body.on_ground,
            ai.yaw,
            ai.speed,
            ai.forward,
            ai.sideways,
            speed,
            body.width,
            body.step_height,
            obstacle_top,
            |_, _| true,
        );
        ai.yaw = control.yaw;
        ai.speed = control.speed;
        ai.forward = control.forward;
        ai.sideways = control.sideways;
        ai.state.look_control.tick(
            position,
            eye,
            ai.body_rotation.body_yaw,
            !ai.state.navigation.is_done(),
        );
        ai.jumping = ai.state.jump || control.jump;
        body.living_jump(world, fluid, ai.jumping, &mut ai.no_jump_delay, 0.4);
        let input = DVec3::new(f64::from(ai.sideways), 0.0, f64::from(ai.forward));
        let mut landed = None;
        if fluid.in_water() {
            body.travel_water(world, input, ai.yaw);
        } else if fluid.in_lava() {
            body.travel_lava(world, input, ai.yaw, fluid.lava_height);
        } else {
            landed = body.travel_air_jumping(world, input, ai.speed, ai.yaw, ai.jumping);
        }
        play_movement(
            body,
            self.tick_count,
            &mut self.random,
            &mut self.voices,
            SOUNDS,
        );
        ai.body_rotation.tick(
            ai.yaw,
            &mut ai.state.look_control,
            self.previous_position,
            body.position,
        );
        self.feline.yaw = ai.yaw;
        step.landed = landed;
        step
    }

    fn tail(&mut self, sleeper_near: bool) {
        if self.feline.health <= 0.0 || !self.feline.is_cat() {
            return;
        }
        let position = self.position();
        if self.tempted() && !self.feline.tame && self.tick_count % 100 == 0 {
            self.voices.push((
                Voice::Event(self.feline.sounds().beg_for_food, 1.0, 1.0),
                position,
            ));
        }
        if (self.feline.lying || self.feline.relax_state_one) && self.tick_count % 5 == 0 {
            let (a, b) = (self.random.next_float(), self.random.next_float());
            self.voices.push((
                Voice::Event(self.feline.sounds().purr, 0.6 + 0.4 * (a - b), 1.0),
                position,
            ));
        }
        self.feline.update_lie_down_amount();
        self.feline.update_relax_state_one_amount();
        self.feline.lying_on_sleeping_player = self.feline.lying && sleeper_near;
    }
}

impl EntityWorld {
    pub fn spawn_feline(&mut self, feline: Feline, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let kind = match feline.kind {
            FelineKind::Cat => MonsterKind::Cat,
            FelineKind::Ocelot => MonsterKind::Ocelot,
        };
        let ai = MonsterAi::of_kind(kind, &feline.body, feline.yaw);
        self.felines.push(FelineEntity {
            id,
            previous_position: feline.body.position,
            feline,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
            effects: Default::default(),
            last_damage: None,
        });
        self.order.push(EntityKey::Feline(id));
        self.file_in_section(EntityKey::Feline(id));
        id
    }

    pub fn felines(&self) -> &[FelineEntity] {
        &self.felines
    }

    pub fn feline_mut(&mut self, id: u64) -> Option<&mut FelineEntity> {
        self.felines.iter_mut().find(|entity| entity.id == id)
    }

    fn feline_people(&self, players: &[PlayerCandidate], kind: FelineKind) -> Vec<FelinePlayer> {
        let tag = match kind {
            FelineKind::Cat => feline::FOOD_TAG,
            FelineKind::Ocelot => feline::OCELOT_FOOD_TAG,
        };
        let food = |item: &str| match &self.recipes {
            Some(recipes) => recipes.item_in_tag(tag, item),
            None => feline::is_food(item),
        };
        players
            .iter()
            .map(|p| {
                let controls = self.player_controls(p.id);
                FelinePlayer {
                    id: p.id,
                    position: p.position,
                    eye_height: p.eye_height,
                    yaw: controls.map_or(0.0, |c| c.yaw),
                    pitch: controls.map_or(0.0, |c| c.pitch),
                    sneaking: controls.is_some_and(|c| c.sneaking),
                    holds_food: controls.is_some_and(|c| c.holds(food)),
                }
            })
            .collect()
    }

    pub fn set_cat_gift_chance(&mut self, chance: f32) {
        self.cat_gift_chance = chance;
    }

    pub fn take_cat_gifts(&mut self) -> Vec<(DVec3, f32)> {
        std::mem::take(&mut self.cat_gifts)
    }

    fn feline_surroundings(
        &self,
        id: u64,
        kind: FelineKind,
        owner: Option<u64>,
        players: &[PlayerCandidate],
    ) -> Surroundings {
        let controls = owner.and_then(|o| self.player_controls(o));
        let block = self
            .felines
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.feline.body.position.floor());
        let sleeper_near = block.is_some_and(|b| {
            let (min, max) = (b - DVec3::splat(2.0), b + DVec3::splat(3.0));
            players.iter().any(|p| {
                self.player_controls(p.id)
                    .is_some_and(|c| c.sleeping.is_some())
                    && p.position.cmpge(min).all()
                    && p.position.cmplt(max).all()
            })
        });
        let others = self.felines.iter().filter(|e| e.id != id);
        Surroundings {
            owner,
            people: self.feline_people(players, kind),
            owner_hurt: owner.is_some_and(|o| self.player_fights(o).hurt_by.is_some()),
            owner_sleeping: controls.and_then(|c| c.sleeping),
            owner_sleep_timer: controls.map_or(0, |c| c.sleep_timer),
            resting: others
                .clone()
                .filter(|e| e.feline.is_cat() && (e.feline.lying || e.feline.relax_state_one))
                .map(|e| {
                    (
                        e.feline.body.position,
                        e.feline.body.width,
                        e.feline.body.height,
                    )
                })
                .collect(),
            gift_chance: self.cat_gift_chance,
            mates: others
                .filter(|e| e.feline.kind == kind)
                .map(|e| FelineMate {
                    id: e.id,
                    position: e.feline.body.position,
                    width: e.feline.body.width,
                    height: e.feline.body.height,
                    alive: e.feline.health > 0.0,
                    cat: e.feline.is_cat(),
                    tame: e.feline.tame,
                    in_love: e.feline.in_love > 0,
                    panicking: e.ai.state.feline.panic.running,
                })
                .collect(),
            sleeper_near,
        }
    }

    pub fn feline_interact(
        &mut self,
        id: u64,
        player: u64,
        inventory: &mut Inventory,
        hand: usize,
        infinite_materials: bool,
    ) -> crate::cow::InteractionResult {
        use crate::cow::InteractionResult;
        let player_uuid = self.uuid_of(PLAYER_TARGET + player);
        let food_tag = match self
            .felines
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.feline.kind)
        {
            Some(FelineKind::Cat) => feline::FOOD_TAG,
            Some(FelineKind::Ocelot) => feline::OCELOT_FOOD_TAG,
            None => return InteractionResult::Pass,
        };
        let recipes = self.recipes.clone();
        let is_food = |item: &str| match &recipes {
            Some(recipes) => recipes.item_in_tag(food_tag, item),
            None => feline::is_food(item),
        };
        let mut events = Vec::new();
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            return InteractionResult::Pass;
        };
        let item = inventory
            .slots
            .get(hand)
            .and_then(Option::as_ref)
            .map_or_else(|| "minecraft:air".to_owned(), |s| s.id.clone());
        let position = entity.position();
        let result = 'result: {
            if !entity.feline.is_cat() {
                let near = entity
                    .ai
                    .state
                    .feline
                    .people
                    .iter()
                    .find(|p| p.id == player)
                    .is_some_and(|p| p.position.distance_squared(position) < 9.0);
                if entity.tempted() && !entity.feline.trusting && is_food(&item) && near {
                    use_player_item(inventory, hand, infinite_materials);
                    if entity.random.next_int(3) == 0 {
                        entity.feline.trusting = true;
                        events.push((id, feline::TRUSTING_EVENT));
                    } else {
                        events.push((id, feline::DISTRUSTING_EVENT));
                    }
                    break 'result InteractionResult::SuccessPredicted;
                }
                break 'result animal_interact(
                    entity,
                    inventory,
                    hand,
                    infinite_materials,
                    &is_food,
                    &mut events,
                );
            }
            let owned = entity.feline.tame && entity.feline.owner == Some(player_uuid);
            if owned {
                let dye = crate::wolf::dye_color(&item);
                if let Some(color) = dye {
                    if color != entity.feline.collar {
                        entity.feline.collar = color;
                        use_player_item(inventory, hand, infinite_materials);
                        entity.feline.persistence_required = true;
                        break 'result InteractionResult::SuccessPredicted;
                    }
                } else if is_food(&item) && entity.feline.health < entity.feline.max_health() {
                    let food = minecraftoss_player::food::catalog().get(&item);
                    let heal_by = food.map_or(1.0, |f| f32::from(f.nutrition));
                    use_player_item(inventory, hand, infinite_materials);
                    let max = entity.feline.max_health();
                    crate::effects::heal(&mut entity.feline.health, max, heal_by);
                    break 'result InteractionResult::SuccessPredicted;
                }
                let result = animal_interact(
                    entity,
                    inventory,
                    hand,
                    infinite_materials,
                    &is_food,
                    &mut events,
                );
                if result == InteractionResult::Pass {
                    entity.feline.ordered_to_sit = !entity.feline.ordered_to_sit;
                    break 'result InteractionResult::SuccessPredicted;
                }
                break 'result result;
            }
            if !entity.feline.tame && is_food(&item) {
                use_player_item(inventory, hand, infinite_materials);
                if entity.random.next_int(3) == 0 {
                    entity.feline.tame = true;
                    entity.feline.owner = Some(player_uuid);
                    entity.feline.ordered_to_sit = true;
                    events.push((id, 7));
                } else {
                    events.push((id, 6));
                }
                entity.feline.persistence_required = true;
                let eat = entity.feline.sounds().eat;
                entity.voices.push((Voice::Event(eat, 1.0, 1.0), position));
                break 'result InteractionResult::SuccessPredicted;
            }
            let result = animal_interact(
                entity,
                inventory,
                hand,
                infinite_materials,
                &is_food,
                &mut events,
            );
            if result != InteractionResult::Pass {
                entity.feline.persistence_required = true;
            }
            result
        };
        self.entity_events.extend(events);
        result
    }

    fn feline_breed(&mut self, parent: u64, partner: u64) {
        let Some(partner_state) = self
            .felines
            .iter()
            .find(|e| e.id == partner)
            .map(|e| (e.feline.variant.clone(), e.feline.collar))
        else {
            return;
        };
        let recipes = self.recipes.clone();
        let mob_drops = self.mob_drops;
        let mut level_random = std::mem::take(&mut self.level_random);
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == parent) else {
            self.level_random = level_random;
            return;
        };
        let kind = entity.feline.kind;
        let at = entity.feline.body.position;
        let mut child = Feline::new(kind, at);
        if kind == FelineKind::Cat {
            child.variant = if entity.random.next_boolean() {
                entity.feline.variant.clone()
            } else {
                partner_state.0.clone()
            };
            if entity.feline.tame {
                let (a, b) = (entity.feline.collar, partner_state.1);
                let mixed = recipes.as_deref().and_then(|book| {
                    let grid = [
                        Some(minecraftoss_player::inventory::ItemStack::new(
                            format!("minecraft:{}_dye", crate::wolf::DYES[usize::from(a & 15)]),
                            1,
                        )),
                        Some(minecraftoss_player::inventory::ItemStack::new(
                            format!("minecraft:{}_dye", crate::wolf::DYES[usize::from(b & 15)]),
                            1,
                        )),
                    ];
                    book.matching(&grid, 2, 1)
                        .and_then(|result| crate::wolf::dye_color(&result.id))
                });
                child.owner = entity.feline.owner;
                child.tame = true;
                child.collar =
                    mixed.unwrap_or_else(|| if level_random.next_boolean() { a } else { b });
            }
        }
        entity.feline.set_age(6000);
        entity.feline.in_love = 0;
        if mob_drops {
            let _ = entity.random.next_int(7);
        }
        self.level_random = level_random;
        if let Some(p) = self.felines.iter_mut().find(|e| e.id == partner) {
            p.feline.set_age(6000);
            p.feline.in_love = 0;
        }
        self.entity_events.push((parent, 18));
        let seed = self.born_seeds.pop_front().unwrap_or_else(|| {
            self.seed_uniquifier = self.seed_uniquifier.wrapping_mul(1_181_783_497_276_652_981);
            self.seed_uniquifier
        });
        let mut random = LegacyRandom::new(seed);
        let turn = random.next_float() * (std::f64::consts::PI * 2.0) as f32;
        child.set_age(crate::age::Age::BABY_START);
        child.yaw = turn;
        let id = self.spawn_feline(child, false);
        if let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) {
            entity.random = random;
            entity.feline.yaw = 0.0;
            entity.ai.yaw = 0.0;
            entity.ai.body_rotation = crate::look::BodyRotation::new(0.0);
        }
    }

    pub(super) fn tick_feline(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty, griefing) =
            (self.game_time, self.difficulty, self.mob_griefing);
        let Some((owner_uuid, kind)) = self
            .felines
            .iter()
            .find(|e| e.id == id)
            .map(|e| (e.feline.owner, e.feline.kind))
        else {
            return;
        };
        let owner = owner_uuid
            .and_then(|uuid| {
                players
                    .iter()
                    .find(|p| self.uuid_of(PLAYER_TARGET + p.id) == uuid)
            })
            .map(|p| p.id);
        let around = self.feline_surroundings(id, kind, owner, players);
        let sleeper_near = around.sleeper_near;
        let mobs = self.mob_candidates();
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.previous_position = entity.feline.body.position;
        entity.tick_count += 1;
        base_tick_fluid(
            &mut entity.feline.body,
            &*world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            SOUNDS,
        );
        super::hazards::burn(entity, &*world, game_time);
        let eye = entity.feline.eye_height();
        super::hazards::suffocate(entity, &*world, eye, game_time);
        let water_breathing = entity.effects.has(MobEffect::WaterBreathing);
        if entity.feline.health > 0.0
            && super::living::breathe(&mut entity.feline.body, &*world, eye, water_breathing)
        {
            entity.hurt_from(2.0, "minecraft:drown", None, game_time);
        }
        let removed = entity.feline.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        let max = entity.feline.max_health();
        for work in entity.effects.tick(false) {
            match work.resolve(entity.feline.health, max) {
                Some(EffectWork::Heal(amount)) => heal(&mut entity.feline.health, max, amount),
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt_from(amount, "minecraft:magic", None, game_time);
                }
                _ => {}
            }
        }
        entity.feline.body.float_from(&entity.effects);
        if entity.feline.health > 0.0 {
            entity.ambient();
        }
        if removed {
            return;
        }
        let mut step = Step::default();
        let mut level_random = std::mem::take(&mut self.level_random);
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            self.level_random = level_random;
            return;
        };
        if entity.feline.health <= 0.0 && !entity.no_ai {
            let yaw = entity.feline.yaw;
            let landed = super::living::dying_travel(
                &mut entity.feline.body,
                &*world,
                yaw,
                entity.tick_count,
                &mut entity.random,
                &mut entity.voices,
                SOUNDS,
                Some(false),
            );
            if landed.is_some_and(|fallen| {
                super::living::fall_on_farmland(
                    &mut entity.feline.body,
                    world,
                    fallen,
                    &mut level_random,
                    griefing,
                )
            }) {
                entity.previous_position = entity.feline.body.position;
            }
        } else if !entity.no_ai {
            let state = &mut entity.ai.state;
            state.difficulty = difficulty;
            state.players = players.to_vec();
            state.mobs = mobs;
            state.level_random = Some(level_random);
            step = entity.tick_ai(&*world, game_time, around);
            level_random = entity.ai.state.level_random.take().unwrap_or_default();
            if let Some(fallen) = step.landed {
                if super::living::fall_on_farmland(
                    &mut entity.feline.body,
                    world,
                    fallen,
                    &mut level_random,
                    griefing,
                ) {
                    entity.previous_position = entity.feline.body.position;
                }
                let sounds = &mut entity.voices;
                if let Some(damage) =
                    fall_damage(&entity.feline.body, &*world, fallen, false, sounds)
                {
                    entity.hurt_from(damage, "minecraft:fall", None, game_time);
                }
            }
        } else {
            entity.feline.body.trim_small_velocity();
        }
        self.level_random = level_random;
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, &*world, old, game_time);
        if let Some(gift) = step.gift {
            self.cat_gifts.push(gift);
        }
        if let Some(partner) = step.breed_with {
            self.feline_breed(id, partner);
        }
        if let Some((target, from)) = step.struck {
            match target {
                Target::Mob(victim) | Target::Villager(victim) => {
                    self.mob_hits_mob(id, victim, feline::ATTACK_DAMAGE, from, 0.0);
                }
                Target::Player(player_id) => {
                    self.player_hits.push(PlayerHit {
                        player_id,
                        damage: feline::ATTACK_DAMAGE,
                        kind: PlayerHitKind::Melee {
                            attacker: from,
                            hunger_ticks: 0,
                            lift: 0.0,
                            throw: glam::DVec3::ZERO,
                            disable_shield_seconds: 0.0,
                        },
                        source: Some(id),
                    });
                }
            }
        }
        self.push_entities(EntityKey::Feline(id), &*world, game_time, ticks);
        let Some(entity) = self.felines.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let was_baby = entity.feline.baby();
        let _ = entity.feline.age.tick(entity.feline.health > 0.0);
        if was_baby != entity.feline.baby() {
            let ticks = entity.feline.age.ticks;
            entity.feline.set_age(ticks);
        }
        if entity.feline.age.ticks != 0 {
            entity.feline.in_love = 0;
        } else if entity.feline.in_love > 0 {
            entity.feline.in_love -= 1;
            if entity.feline.in_love % 10 == 0 {
                for _ in 0..3 {
                    let _ = entity.random.next_gaussian();
                }
                for _ in 0..3 {
                    let _ = entity.random.next_double();
                }
            }
        }
        entity.tail(sleeper_near);
        self.file_in_section(EntityKey::Feline(id));
    }
}

fn animal_interact(
    entity: &mut FelineEntity,
    inventory: &mut Inventory,
    hand: usize,
    infinite_materials: bool,
    is_food: &dyn Fn(&str) -> bool,
    events: &mut Vec<(u64, u8)>,
) -> crate::cow::InteractionResult {
    let f = &mut entity.feline;
    let (result, animal_events) = crate::animal::interact(
        &mut f.age,
        &mut f.in_love,
        &mut f.persistence_required,
        inventory,
        hand,
        infinite_materials,
        is_food,
        false,
    );
    if animal_events.contains(&crate::animal::AnimalEvent::Hearts) {
        events.push((entity.id, 18));
    }
    if f.age.baby() != (f.body.height < feline::HEIGHT) {
        let ticks = f.age.ticks;
        f.set_age(ticks);
    }
    result
}

fn use_player_item(inventory: &mut Inventory, hand: usize, infinite_materials: bool) {
    if !infinite_materials {
        crate::animal::consume_one(inventory, hand);
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

    #[test]
    fn a_cat_and_an_ocelot_are_live_mobs_of_their_kind() {
        let mut world = EntityWorld::default();
        let cat = world.spawn_feline(
            Feline::new(FelineKind::Cat, DVec3::new(0.5, 64.0, 0.5)),
            false,
        );
        let ocelot = world.spawn_feline(
            Feline::new(FelineKind::Ocelot, DVec3::new(4.5, 64.0, 0.5)),
            false,
        );
        assert_eq!(world.entity_type(cat), Some("minecraft:cat"));
        assert_eq!(world.entity_type(ocelot), Some("minecraft:ocelot"));
        assert_eq!(world.felines().len(), 2);
        let mut floor = Floor;
        for _ in 0..40 {
            world.tick(&mut floor);
        }
        let census = world.census();
        assert!(
            census.iter().any(|entry| entry.kind == "minecraft:cat")
                && census.iter().any(|entry| entry.kind == "minecraft:ocelot")
        );
        assert!(
            world
                .felines()
                .iter()
                .all(|e| e.feline.health == feline::MAX_HEALTH && e.feline.body.position.y >= 63.9)
        );
        let hit = world
            .felines
            .iter_mut()
            .find(|e| e.id == cat)
            .unwrap()
            .hurt(4.0);
        assert!(hit.applied);
        assert_eq!(world.felines()[0].feline.health, 6.0);
        assert!(
            world
                .mob_candidates()
                .iter()
                .any(|m| m.id == ocelot && m.kind == "minecraft:ocelot")
        );
    }

    #[test]
    fn a_wild_ocelot_runs_from_a_player_and_a_tempted_one_crouches_after_fish() {
        let mut world = EntityWorld::default();
        let id = world.spawn_feline(
            Feline::new(FelineKind::Ocelot, DVec3::new(0.5, 64.0, 0.5)),
            false,
        );
        world
            .felines
            .iter_mut()
            .find(|e| e.id == id)
            .unwrap()
            .set_random_seed(7);
        let players = [player(1, DVec3::new(5.5, 64.0, 0.5))];
        let mut floor = Floor;
        let mut tempting = world.clone();
        let mut ran = false;
        for _ in 0..80 {
            world.tick_with_players(&mut floor, &players);
            ran |= world.felines()[0]
                .running_goals()
                .contains(&"OcelotAvoidEntityGoal");
        }
        assert!(ran, "an ocelot that does not trust flees");
        assert!(
            world.felines()[0]
                .feline
                .body
                .position
                .distance(players[0].position)
                > 5.0
        );
        tempting.set_player_controls(&[crate::controls::PlayerControls {
            id: 1,
            main_hand: Some("minecraft:cod".to_owned()),
            ..Default::default()
        }]);
        let mut tempted = false;
        for _ in 0..200 {
            tempting.tick_with_players(&mut floor, &players);
            tempted |= tempting.felines()[0].tempted();
        }
        assert!(tempted, "raw cod tempts it");
    }

    struct Room {
        bed: Pos,
    }
    impl World for Room {
        fn block(&self, pos: Pos) -> Option<Block> {
            if pos == self.bed {
                return Some(
                    Block::new("minecraft:red_bed")
                        .with("facing", "north")
                        .with("part", "head"),
                );
            }
            let pit = pos.0 == self.bed.0 && pos.2 == self.bed.2 && pos.1 > self.bed.1;
            (pos.1 < 64 && !pit).then(|| Block::new("minecraft:grass_block"))
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
        fn block_in_tag(&self, pos: Pos, tag: &str) -> bool {
            let Some(block) = self.block(pos) else {
                return false;
            };
            match tag {
                "minecraft:beds" | "minecraft:cats_can_lie_on" | "minecraft:cats_can_sit_on" => {
                    block.id.ends_with("_bed")
                }
                "minecraft:entities_can_teleport_to" => block.id == "minecraft:grass_block",
                _ => false,
            }
        }
    }

    fn holding(item: &str) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(minecraftoss_player::inventory::ItemStack::new(item, 64));
        inventory
    }

    const OWNER: u128 = 0x1234_5678_9abc_def0_0fed_cba9_8765_4321;

    fn tame_cat(world: &mut EntityWorld, at: DVec3) -> u64 {
        let mut cat = Feline::new(FelineKind::Cat, at);
        cat.tame = true;
        cat.owner = Some(OWNER);
        world.spawn_feline(cat, false)
    }

    #[test]
    fn a_wild_cat_takes_raw_fish_and_is_tamed_one_time_in_three() {
        use crate::cow::InteractionResult;
        let mut world = EntityWorld::default();
        world.set_player_uuid(1, OWNER);
        let mut tamed = 0;
        for seed in 0..60 {
            let id = world.spawn_feline(
                Feline::new(FelineKind::Cat, DVec3::new(0.5, 64.0, 0.5)),
                true,
            );
            world.feline_mut(id).unwrap().set_random_seed(seed);
            let mut inventory = holding("minecraft:cod");
            assert_eq!(
                world.feline_interact(id, 1, &mut inventory, 0, false),
                InteractionResult::SuccessPredicted
            );
            assert_eq!(
                inventory.slots[0].as_ref().unwrap().count,
                63,
                "the fish is eaten"
            );
            let cat = &world.feline_mut(id).unwrap().feline;
            assert!(cat.persistence_required);
            if cat.tame {
                tamed += 1;
                assert_eq!(cat.owner, Some(OWNER));
                assert!(cat.ordered_to_sit && cat.shows_collar());
                assert_eq!(cat.collar, feline::DEFAULT_COLLAR);
            }
        }
        assert!((10..=30).contains(&tamed), "{tamed} of 60");
        let id = world.spawn_feline(
            Feline::new(FelineKind::Cat, DVec3::new(0.5, 64.0, 0.5)),
            true,
        );
        let mut stick = holding("minecraft:stick");
        assert_eq!(
            world.feline_interact(id, 1, &mut stick, 0, false),
            InteractionResult::Pass
        );
    }

    #[test]
    fn its_owner_sits_a_tame_cat_down_dyes_its_collar_and_others_cannot() {
        use crate::cow::InteractionResult;
        let mut world = EntityWorld::default();
        world.set_player_uuid(1, OWNER);
        let id = tame_cat(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let mut empty = Inventory::default();
        assert_eq!(
            world.feline_interact(id, 1, &mut empty, 0, false),
            InteractionResult::SuccessPredicted
        );
        assert!(world.feline_mut(id).unwrap().feline.ordered_to_sit);
        assert_eq!(
            world.feline_interact(id, 2, &mut empty, 0, false),
            InteractionResult::Pass
        );
        assert!(world.feline_mut(id).unwrap().feline.ordered_to_sit);
        let mut dye = holding("minecraft:blue_dye");
        assert_eq!(
            world.feline_interact(id, 1, &mut dye, 0, false),
            InteractionResult::SuccessPredicted
        );
        assert_eq!(world.feline_mut(id).unwrap().feline.collar, 11);
        assert_eq!(dye.slots[0].as_ref().unwrap().count, 63);
        assert_eq!(
            world.feline_interact(id, 1, &mut empty, 0, false),
            InteractionResult::SuccessPredicted
        );
        assert!(!world.feline_mut(id).unwrap().feline.ordered_to_sit);
    }

    #[test]
    fn a_tame_cat_follows_the_player_with_its_owner_s_uuid() {
        let mut world = EntityWorld::default();
        world.set_player_uuid(1, OWNER);
        let id = tame_cat(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let players = [
            player(0, DVec3::new(-30.5, 64.0, 0.5)),
            player(1, DVec3::new(30.5, 64.0, 0.5)),
        ];
        let mut floor = Floor;
        for _ in 0..60 {
            world.tick_with_players(&mut floor, &players);
        }
        let at = world
            .felines()
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .feline
            .body
            .position;
        assert!(
            at.distance(players[1].position) < 6.0,
            "it came to its owner: {at}"
        );
        world.feline_mut(id).unwrap().feline.ordered_to_sit = true;
        let moved = [
            player(0, DVec3::new(-30.5, 64.0, 0.5)),
            player(1, DVec3::new(-10.5, 64.0, 20.5)),
        ];
        for _ in 0..60 {
            world.tick_with_players(&mut floor, &moved);
        }
        let cat = &world.felines().iter().find(|e| e.id == id).unwrap().feline;
        assert!(cat.sitting, "told to sit, it sits");
        assert!(cat.body.position.distance(at) < 1.0, "and stays put");
    }

    #[test]
    fn a_creeper_runs_from_a_cat_and_from_an_ocelot() {
        for kind in [FelineKind::Cat, FelineKind::Ocelot] {
            let mut world = EntityWorld::default();
            world.spawn_feline(Feline::new(kind, DVec3::new(0.5, 64.0, 0.5)), true);
            let creeper = world.spawn_creeper(
                crate::creeper::Creeper::new(DVec3::new(3.5, 64.0, 0.5)),
                false,
            );
            let mut floor = Floor;
            let mut fled = false;
            for _ in 0..80 {
                world.tick(&mut floor);
                let e = world.creepers().iter().find(|e| e.id == creeper).unwrap();
                fled |= e.ai.running_goals().contains(&"AvoidEntityGoal");
            }
            let e = world.creepers().iter().find(|e| e.id == creeper).unwrap();
            assert!(fled, "{kind:?}");
            assert!(
                e.creeper.body.position.distance(DVec3::new(0.5, 64.0, 0.5)) > 5.0,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn an_ocelot_learns_to_trust_only_while_tempted_and_close() {
        use crate::cow::InteractionResult;
        let mut world = EntityWorld::default();
        let id = world.spawn_feline(
            Feline::new(FelineKind::Ocelot, DVec3::new(0.5, 64.0, 0.5)),
            true,
        );
        let mut cod = holding("minecraft:cod");
        assert_eq!(
            world.feline_interact(id, 1, &mut cod, 0, false),
            InteractionResult::SuccessServer
        );
        assert_eq!(
            world.feline_mut(id).unwrap().feline.in_love,
            600,
            "untempted, fish only puts it in love"
        );
        let id = world.spawn_feline(
            Feline::new(FelineKind::Ocelot, DVec3::new(0.5, 64.0, 0.5)),
            true,
        );
        {
            let e = world.feline_mut(id).unwrap();
            e.set_random_seed(3);
            e.ai.state.feline.tempt.running = true;
            e.ai.state.feline.people = vec![crate::feline_ai::FelinePlayer {
                id: 1,
                position: DVec3::new(2.0, 64.0, 0.5),
                ..Default::default()
            }];
        }
        let mut tries = 0;
        while !world.feline_mut(id).unwrap().feline.trusting {
            assert_eq!(
                world.feline_interact(id, 1, &mut cod, 0, false),
                InteractionResult::SuccessPredicted
            );
            tries += 1;
            assert!(tries < 30);
        }
        assert_eq!(cod.slots[0].as_ref().unwrap().count, 63 - tries);
        let far = world.spawn_feline(
            Feline::new(FelineKind::Ocelot, DVec3::new(0.5, 64.0, 0.5)),
            true,
        );
        {
            let e = world.feline_mut(far).unwrap();
            e.ai.state.feline.tempt.running = true;
            e.ai.state.feline.people = vec![crate::feline_ai::FelinePlayer {
                id: 1,
                position: DVec3::new(4.0, 64.0, 0.5),
                ..Default::default()
            }];
        }
        let _ = world.feline_interact(far, 1, &mut cod, 0, false);
        assert!(
            !world.feline_mut(far).unwrap().feline.trusting,
            "three blocks or nearer"
        );
    }

    #[test]
    fn a_tame_cat_lies_down_on_a_bed() {
        let mut world = EntityWorld::default();
        world.set_player_uuid(1, OWNER);
        let id = tame_cat(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let mut room = Room { bed: (3, 61, 0) };
        let players = [player(1, DVec3::new(6.5, 64.0, 0.5))];
        let mut lay = false;
        for _ in 0..300 {
            world.tick_with_players(&mut room, &players);
            lay |= world.felines()[0].feline.lying;
        }
        let cat = &world.felines().iter().find(|e| e.id == id).unwrap().feline;
        assert!(lay, "it lies on the bed");
        assert!(
            cat.body.position.distance(DVec3::new(3.5, 62.0, 0.5)) < 1.5,
            "{}",
            cat.body.position
        );
    }

    #[test]
    fn a_cat_curls_up_on_its_sleeping_owner_s_bed_and_brings_a_morning_gift() {
        let mut world = EntityWorld::default();
        world.set_player_uuid(1, OWNER);
        world.set_cat_gift_chance(1.0);
        tame_cat(&mut world, DVec3::new(3.5, 64.0, 1.5));
        let mut room = Room { bed: (3, 64, 0) };
        let players = [player(1, DVec3::new(3.5, 64.6875, 0.5))];
        let asleep = crate::controls::PlayerControls {
            id: 1,
            sleeping: Some((3, 64, 0)),
            sleep_timer: 100,
            ..Default::default()
        };
        world.set_player_controls(&[asleep]);
        let mut rested = false;
        for _ in 0..200 {
            world.tick_with_players(&mut room, &players);
            let cat = &world.felines()[0].feline;
            rested |= cat.relax_state_one || cat.lying;
        }
        assert!(rested, "it relaxes and lies down at the foot of the bed");
        assert!(
            world.take_cat_gifts().is_empty(),
            "no gift while the owner sleeps"
        );
        let awake = crate::controls::PlayerControls {
            id: 1,
            sleep_timer: 100,
            ..Default::default()
        };
        world.set_player_controls(&[awake]);
        for _ in 0..4 {
            world.tick_with_players(&mut room, &players);
        }
        assert_eq!(
            world.take_cat_gifts().len(),
            1,
            "a gift when the owner wakes up"
        );
        assert!(!world.felines()[0].feline.lying);
    }

    #[test]
    fn two_tame_cats_in_love_have_a_tame_kitten() {
        let mut world = EntityWorld::default();
        world.set_player_uuid(1, OWNER);
        let a = tame_cat(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let b = tame_cat(&mut world, DVec3::new(2.5, 64.0, 0.5));
        for id in [a, b] {
            let cat = &mut world.feline_mut(id).unwrap().feline;
            cat.in_love = 600;
            cat.variant = if id == a {
                "calico".to_owned()
            } else {
                "jellie".to_owned()
            };
        }
        let players = [player(1, DVec3::new(4.5, 64.0, 4.5))];
        let mut floor = Floor;
        for _ in 0..200 {
            world.tick_with_players(&mut floor, &players);
        }
        let kittens: Vec<_> = world.felines().iter().filter(|e| e.feline.baby()).collect();
        assert_eq!(kittens.len(), 1);
        let kitten = &kittens[0].feline;
        assert!(kitten.tame && kitten.owner == Some(OWNER));
        assert!(["calico", "jellie"].contains(&kitten.variant.as_str()));
        assert!(
            world
                .felines()
                .iter()
                .filter(|e| !e.feline.baby())
                .all(|e| e.feline.in_love == 0 && e.feline.age.ticks > 5000)
        );
    }
}
