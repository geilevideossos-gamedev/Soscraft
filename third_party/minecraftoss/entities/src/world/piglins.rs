use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::brain_behaviors::{Living, Pos, find};
use crate::monster_ai::Target;
use crate::piglin::{self, ArmPose, Piglin, PiglinKind};
use crate::piglin_brain::{Activity, ItemUse, PiglinBrain, PiglinCtx, WantedItem};

pub const LOVED_TAG: &str = "minecraft:piglin_loved";
pub const SAFE_ARMOR_TAG: &str = "minecraft:piglin_safe_armor";
pub const FOOD_TAG: &str = "minecraft:piglin_food";
pub const REPELLENT_ITEMS_TAG: &str = "minecraft:piglin_repellents";
pub const IGNORED_BY_BABIES_TAG: &str = "minecraft:ignored_by_piglin_babies";
pub const PREFERRED_WEAPONS_TAG: &str = "minecraft:piglin_preferred_weapons";
pub const THROW_SPEED: f64 = 0.3;
pub const PICKUP_DELAY: i32 = 10;
pub const ANGER_RANGE: f64 = 16.0;
const HOGLIN: &str = "minecraft:hoglin";
const ANGER_PLAYER_WIDTH: f64 = 0.6;
const ANGER_PLAYER_HEIGHT: f64 = 1.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarterThrow {
    pub piglin: u64,
    pub from: DVec3,
    pub velocity: DVec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PiglinAnger {
    pub player: u64,
    pub feet: DVec3,
    pub require_sight: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PiglinRules {
    pub universal_anger: bool,
    pub forgive_dead_players: bool,
}

impl Default for PiglinRules {
    fn default() -> Self {
        Self {
            universal_anger: false,
            forgive_dead_players: true,
        }
    }
}

pub(super) struct PiglinSees<'a> {
    others: &'a [Living],
    uuids: &'a [(Target, u128)],
    gilded: &'a [Target],
    loved_holders: &'a [Target],
    unhuntable: &'a [Target],
    rules: PiglinRules,
    wanted_items: Option<&'a [WantedItem]>,
    offhand_loved: bool,
    zombify: bool,
}

#[derive(Default)]
pub(super) struct PiglinThought {
    doors: Vec<(Pos, bool)>,
    attack: Option<Target>,
    shot: Option<Target>,
    stop_holding: Option<bool>,
    hunt: Option<Target>,
    convert: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PiglinConversion {
    pub zombified: u64,
    pub equipment: [Option<ItemStack>; 6],
    pub drop_chances: [f32; 6],
    pub can_pick_up_loot: bool,
    pub left_handed: bool,
}

#[derive(Clone)]
pub struct PiglinEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub piglin: Piglin,
    pub no_ai: bool,
    pub tick_count: i32,
    pub ambient_sound_time: i32,
    pub voices: Vec<(Voice, DVec3)>,
    pub no_action_time: i32,
    pub random: LegacyRandom,
    pub previous_position: DVec3,
    pub fluid: FluidFrame,
    pub jumping: bool,
    pub no_jump_delay: i32,
    pub look_control: LookControl,
    pub body_rotation: BodyRotation,
    pub brain: PiglinBrain,
    pub navigation: GroundNavigation,
    pub move_control: MoveControl,
    pub speed: f32,
    pub forward: f32,
    pub sideways: f32,
    pub hurt_by: Vec<Target>,
    pub arm_pose: ArmPose,
    pub using_item: Option<i32>,
    pub loading_sounds: [bool; 2],
    pub swings: u32,
}

pub(super) fn piglin_sounds(kind: PiglinKind) -> MovementSounds {
    match kind {
        PiglinKind::Piglin => MovementSounds::monster(Some("entity.piglin.step")),
        PiglinKind::Brute => MovementSounds::monster(Some("entity.piglin_brute.step")),
    }
    .with_step_volume(piglin::STEP_VOLUME)
}

impl PiglinEntity {
    pub fn position(&self) -> DVec3 {
        self.piglin.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.piglin.health > 0.0 && !self.piglin.damage.dead {
            self.no_action_time = 0;
        }
        let result = self.piglin.hurt_generic(amount);
        let position = self.position();
        let on_fire = self.piglin.body.fire_ticks > 0;
        self.piglin.damage.place_death(result, position, on_fire);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -piglin::AMBIENT_INTERVAL;
            }
            let voice = hurt_voice(&mut self.random, result.died, self.piglin.is_baby());
            self.voices.push((voice, position));
        }
        result
    }

    pub fn hurt_armored(&mut self, amount: f32) -> DamageResult {
        let (armor, toughness) = self.piglin.armor_defense();
        self.hurt(damage_after_armor(amount, armor, toughness))
    }

    pub fn walk_profile(&self) -> WalkProfile {
        let mut profile = WalkProfile::animal(self.piglin.body.width, self.piglin.body.height);
        profile.can_float = false;
        profile.can_open_doors = true;
        profile.follow_range = self.piglin.follow_range() as f32;
        profile.walk_target = crate::walk_path::WalkTarget::Monster;
        profile
    }

    fn make_sound(&mut self, event: &'static str) {
        let pitch = voice_pitch(&mut self.random, self.piglin.is_baby());
        self.voices
            .push((Voice::Event(event, 1.0, pitch), self.position()));
    }

    fn ambient_event(&self, others: &[Living]) -> &'static str {
        match self.piglin.kind {
            PiglinKind::Piglin => {
                self.brain
                    .activity_sound(self.position(), others, self.piglin.converting)
            }
            PiglinKind::Brute => "entity.piglin_brute.ambient",
        }
    }

    fn tick_using_item(&mut self) {
        let Some(ticks) = self.using_item else {
            return;
        };
        if !self.piglin.holds_crossbow() {
            self.using_item = None;
            return;
        }
        let progress = ticks as f32 / crate::piglin_brain::CROSSBOW_CHARGE as f32;
        if progress < 0.2 {
            self.loading_sounds = [false; 2];
        }
        let position = self.position();
        for (index, (at, event)) in [
            (0.2, "item.crossbow.loading_start"),
            (0.5, "item.crossbow.loading_middle"),
        ]
        .into_iter()
        .enumerate()
        {
            if progress >= at && !self.loading_sounds[index] {
                self.loading_sounds[index] = true;
                self.voices.push((Voice::Event(event, 0.5, 1.0), position));
            }
        }
        self.using_item = Some(ticks + 1);
    }

    fn use_item(&mut self, used: ItemUse, level_random: &mut LegacyRandom) {
        match used {
            ItemUse::Start => {
                self.using_item = Some(0);
                self.loading_sounds = [false; 2];
            }
            ItemUse::Stop => self.using_item = None,
            ItemUse::Release => {
                self.using_item = None;
                if self.piglin.holds_crossbow() && !self.piglin.crossbow_charged() {
                    self.piglin.set_crossbow_charged(true);
                    let pitch = 1.0 / (level_random.next_float() * 0.5 + 1.0) + 0.2;
                    let position = self.position();
                    self.voices.push((
                        Voice::Event("item.crossbow.loading_end", 1.0, pitch),
                        position,
                    ));
                }
            }
        }
    }

    fn think(
        &mut self,
        world: &dyn World,
        sees: &PiglinSees,
        game_time: i64,
        dimension: &str,
        level_random: &mut LegacyRandom,
    ) -> PiglinThought {
        let others = sees.others;
        let profile = self.walk_profile();
        let mut ctx = PiglinCtx {
            id: self.id,
            kind: self.piglin.kind,
            body: &self.piglin.body,
            eye_height: self.piglin.eye_height(),
            baby: self.piglin.is_baby(),
            follow_range: self.piglin.follow_range(),
            others,
            world,
            time: game_time,
            dimension,
            random: &mut self.random,
            level_random,
            navigation: &mut self.navigation,
            profile: &profile,
            fluid: self.fluid,
            converting: self.piglin.converting,
            uuids: sees.uuids,
            gilded: sees.gilded,
            loved_holders: sees.loved_holders,
            unhuntable: sees.unhuntable,
            can_hunt: self.piglin.kind == PiglinKind::Piglin && !self.piglin.cannot_hunt,
            universal_anger: sees.rules.universal_anger,
            forgive_dead_players: sees.rules.forgive_dead_players,
            holds_crossbow: self.piglin.holds_crossbow(),
            using_item: self.using_item,
            wanted_items: sees.wanted_items,
            offhand: self.piglin.offhand.is_some(),
            offhand_shield: self
                .piglin
                .offhand
                .as_ref()
                .is_some_and(|stack| stack.id == "minecraft:shield"),
            offhand_loved: sees.offhand_loved,
            look_at: None,
            sounds: Vec::new(),
            doors: Vec::new(),
            attack: None,
            shot: None,
            strafe: None,
            item_use: None,
            charging: None,
            stop_holding: None,
            hunt: None,
        };
        self.brain.tick(&mut ctx);
        self.brain.update_activity(&mut ctx);
        let thought = PiglinThought {
            doors: std::mem::take(&mut ctx.doors),
            attack: ctx.attack,
            shot: ctx.shot,
            stop_holding: ctx.stop_holding,
            hunt: ctx.hunt,
            convert: false,
        };
        let (look_at, sounds, strafe) = (ctx.look_at, ctx.sounds, ctx.strafe);
        let (item_use, charging) = (ctx.item_use, ctx.charging);
        if let Some(at) = look_at {
            self.look_control.set_look_at(at);
        }
        for sound in sounds {
            self.make_sound(sound);
        }
        if let Some(forward) = strafe {
            self.move_control.strafe(forward, 0.0);
            self.piglin.yaw = self.look_control.head_yaw;
        }
        if let Some(used) = item_use {
            self.use_item(used, level_random);
        }
        if let Some(charging) = charging {
            self.piglin.charging_crossbow = charging;
        }
        if self.piglin.kind == PiglinKind::Brute
            && level_random.next_float() < 0.0125
            && self.brain.activity == Activity::Fight
        {
            self.make_sound("entity.piglin_brute.angry");
        }
        self.piglin.aggressive = self.brain.aggressive();
        self.piglin.dancing = self.brain.dancing();
        thought
    }

    fn steer(&mut self, world: &impl World) {
        let position = self.piglin.body.position;
        let (can_update, surface) =
            crate::navigation::ground_view(world, &self.piglin.body, self.fluid, false);
        if let Some((target, speed)) = self.navigation.tick_in(
            world,
            position,
            can_update,
            surface,
            self.piglin.body.width,
            self.speed,
        ) {
            self.move_control.set_wanted_position(target, speed);
        }
        let obstacle_top = crate::control::obstacle_top(world, position);
        let movement_speed = self.effects.movement_speed(self.piglin.movement_speed());
        let control = self.move_control.tick(
            position,
            self.piglin.body.on_ground,
            self.piglin.yaw,
            self.speed,
            self.forward,
            self.sideways,
            movement_speed,
            self.piglin.body.width,
            self.piglin.body.step_height,
            obstacle_top,
            |_, _| true,
        );
        self.piglin.yaw = control.yaw;
        self.speed = control.speed;
        self.forward = control.forward;
        self.sideways = control.sideways;
        self.jumping = control.jump;
    }

    fn travel(&mut self, world: &impl World) -> Option<f64> {
        let input = DVec3::new(f64::from(self.sideways), 0.0, f64::from(self.forward));
        let speed = self.speed;
        let body = &mut self.piglin.body;
        let yaw = self.piglin.yaw;
        let fluid = self.fluid;
        if fluid.in_water() {
            body.travel_water(world, input, yaw);
            None
        } else if fluid.in_lava() {
            body.travel_lava(world, input, yaw, fluid.lava_height);
            None
        } else {
            body.travel_air_jumping(world, input, speed, yaw, self.jumping)
        }
    }

    pub(super) fn tick_body(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        sees: &PiglinSees,
        game_time: i64,
        dimension: &str,
        level_random: &mut LegacyRandom,
    ) -> (bool, PiglinThought) {
        let others = sees.others;
        self.previous_position = self.piglin.body.position;
        self.piglin.converting =
            !self.no_ai && !self.piglin.immune_to_zombification && sees.zombify;
        self.tick_count += 1;
        let sounds = piglin_sounds(self.piglin.kind);
        self.fluid = base_tick_fluid(
            &mut self.piglin.body,
            world,
            self.tick_count == 1,
            &mut self.random,
            &mut self.voices,
            sounds,
        );
        hazards::burn(self, world, game_time);
        let eye = self.piglin.eye_height();
        hazards::suffocate(self, world, eye, game_time);
        let water_breathing = self.effects.has(MobEffect::WaterBreathing);
        if self.piglin.health > 0.0 && breathe(&mut self.piglin.body, world, eye, water_breathing) {
            self.hurt(2.0);
        }
        let removed = self.piglin.damage.tick();
        let max_health = self.piglin.max_health();
        for work in self.effects.tick(false) {
            match work.resolve(self.piglin.health, max_health) {
                Some(EffectWork::Heal(amount)) => heal(&mut self.piglin.health, max_health, amount),
                Some(EffectWork::HurtMagic(amount)) => {
                    self.hurt(amount);
                }
                _ => {}
            }
        }
        if self.piglin.health > 0.0 {
            if (self.random.next_int(1000) as i32) < self.ambient_sound_time {
                self.ambient_sound_time = -piglin::AMBIENT_INTERVAL;
                if self.brain.activity == crate::piglin_brain::Activity::Idle {
                    let event = self.ambient_event(others);
                    self.make_sound(event);
                }
            } else {
                self.ambient_sound_time += 1;
            }
        }
        if removed {
            return (false, PiglinThought::default());
        }
        let old = self.previous_position;
        if self.piglin.health <= 0.0 {
            if !self.no_ai {
                let yaw = self.piglin.yaw;
                dying_travel(
                    &mut self.piglin.body,
                    world,
                    yaw,
                    self.tick_count,
                    &mut self.random,
                    &mut self.voices,
                    sounds,
                    Some(true),
                );
            }
            hazards::blocks_act(self, world, old, game_time);
            return (true, PiglinThought::default());
        }
        let position = self.piglin.body.position;
        self.piglin.body.trim_small_velocity();
        monster_idle_without_ai(world, players, position, eye, &mut self.no_action_time);
        let mut thought = PiglinThought::default();
        if !self.no_ai {
            self.no_action_time += 1;
            self.tick_using_item();
            thought = self.think(world, sees, game_time, dimension, level_random);
            if self.piglin.converting {
                self.piglin.time_in_overworld += 1;
            } else {
                self.piglin.time_in_overworld = 0;
            }
            thought.convert = self.piglin.time_in_overworld > piglin::CONVERSION_TIME;
            self.steer(world);
            if self.no_jump_delay > 0 {
                self.no_jump_delay -= 1;
            }
            let jumping = self.jumping;
            self.piglin
                .body
                .living_jump(world, self.fluid, jumping, &mut self.no_jump_delay, 0.4);
            self.look_control.tick(
                self.piglin.body.position,
                eye,
                self.body_rotation.body_yaw,
                !self.navigation.is_done(),
            );
            let landed = self.travel(world);
            play_movement(
                &mut self.piglin.body,
                self.tick_count,
                &mut self.random,
                &mut self.voices,
                sounds,
            );
            self.body_rotation.tick(
                self.piglin.yaw,
                &mut self.look_control,
                self.previous_position,
                self.piglin.body.position,
            );
            if let Some(damage) = landed.and_then(|fallen| {
                fall_damage(&self.piglin.body, world, fallen, true, &mut self.voices)
            }) {
                self.hurt(damage);
            }
        }
        hazards::blocks_act(self, world, old, game_time);
        (true, thought)
    }

    pub(super) fn drop_equipment(
        &mut self,
        killed_by_player: bool,
        max_damage: &dyn Fn(&ItemStack) -> Option<u32>,
    ) -> Vec<ItemStack> {
        let mut out = Vec::new();
        for slot in 0..6 {
            let chance = self.piglin.drop_chances[slot];
            if chance == 0.0 {
                continue;
            }
            let preserve = chance > 1.0;
            let Some(stack) = self.piglin.equipment(slot).cloned() else {
                continue;
            };
            if !(killed_by_player || preserve) || self.random.next_float() >= chance {
                continue;
            }
            let mut stack = stack;
            if !preserve {
                if let Some(max) = max_damage(&stack) {
                    let spread = self.random.next_int(max.saturating_sub(3).max(1));
                    let wear = max - self.random.next_int(1 + spread);
                    let mut components = match stack.components.take() {
                        Some(serde_json::Value::Object(map)) => map,
                        _ => serde_json::Map::new(),
                    };
                    components.insert("minecraft:damage".to_owned(), serde_json::Value::from(wear));
                    stack.components = Some(serde_json::Value::Object(components));
                }
            }
            out.push(stack);
            if let Some(cell) = self.piglin.equipment_mut(slot) {
                *cell = None;
            }
        }
        if self.piglin.kind == PiglinKind::Piglin {
            out.extend(self.piglin.inventory.iter_mut().filter_map(Option::take));
        }
        out
    }

    pub(super) fn experience(&mut self) -> i32 {
        match self.piglin.kind {
            PiglinKind::Piglin => piglin::EXPERIENCE,
            PiglinKind::Brute => {
                let mut experience = piglin::BRUTE_EXPERIENCE;
                for slot in 0..6 {
                    if self.piglin.equipment(slot).is_some()
                        && self.piglin.drop_chances[slot] <= 1.0
                    {
                        experience += 1 + self.random.next_int(3) as i32;
                    }
                }
                experience
            }
        }
    }
}

impl EntityWorld {
    pub fn spawn_piglin(&mut self, mut piglin: Piglin, no_ai: bool) -> u64 {
        self.next_id += 1;
        piglin.sync_dimensions();
        let id = self.next_id;
        let previous_position = piglin.body.position;
        let yaw = piglin.yaw;
        self.piglins.push(PiglinEntity {
            id,
            effects: Default::default(),
            piglin,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            previous_position,
            fluid: FluidFrame::default(),
            jumping: false,
            no_jump_delay: 0,
            look_control: LookControl::new(yaw),
            body_rotation: BodyRotation::new(yaw),
            brain: PiglinBrain::default(),
            navigation: GroundNavigation::default(),
            move_control: MoveControl::default(),
            speed: 0.0,
            forward: 0.0,
            sideways: 0.0,
            hurt_by: Vec::new(),
            arm_pose: ArmPose::Default,
            using_item: None,
            loading_sounds: [false; 2],
            swings: 0,
        });
        self.order.push(EntityKey::Piglin(id));
        self.file_in_section(EntityKey::Piglin(id));
        id
    }

    pub fn piglins(&self) -> &[PiglinEntity] {
        &self.piglins
    }

    pub fn piglin_mut(&mut self, id: u64) -> Option<&mut PiglinEntity> {
        self.piglins.iter_mut().find(|entity| entity.id == id)
    }

    pub fn set_dimension(&mut self, dimension: &str) {
        if self.dimension != dimension {
            self.dimension = dimension.to_owned();
        }
    }

    pub fn dimension(&self) -> &str {
        &self.dimension
    }

    pub(super) fn item_loved(&self, id: &str) -> bool {
        self.recipes
            .as_ref()
            .is_some_and(|recipes| recipes.item_in_tag(LOVED_TAG, id))
    }

    pub fn take_barter_throws(&mut self) -> Vec<BarterThrow> {
        std::mem::take(&mut self.barter_throws)
    }

    fn item_tagged(&self, tag: &str, id: &str) -> bool {
        self.recipes
            .as_ref()
            .is_some_and(|recipes| recipes.item_in_tag(tag, id))
    }

    fn piglin_max_stack(&self, id: &str) -> u8 {
        self.recipes
            .as_ref()
            .map_or(64, |recipes| recipes.max_stack(id))
            .clamp(1, 99)
    }

    fn piglin_offhand_loved(&self, id: u64) -> bool {
        self.piglins
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.piglin.offhand.as_ref())
            .is_some_and(|stack| self.item_loved(&stack.id))
    }

    fn slot_for(&self, stack: &ItemStack) -> usize {
        let named = self
            .recipes
            .as_ref()
            .and_then(|recipes| recipes.equipment_slot(stack).map(str::to_owned));
        match named.as_deref() {
            Some("head") => piglin::HEAD,
            Some("chest") => piglin::CHEST,
            Some("legs") => piglin::LEGS,
            Some("feet") => piglin::FEET,
            Some(_) => piglin::MAINHAND,
            None => piglin::armor_slot(&stack.id).unwrap_or(piglin::MAINHAND),
        }
    }

    fn preferred_weapon(&self, piglin: &Piglin, stack: &ItemStack) -> bool {
        !piglin.is_baby() && self.item_tagged(PREFERRED_WEAPONS_TAG, &stack.id)
    }

    fn can_replace(
        &self,
        piglin: &Piglin,
        new: &ItemStack,
        current: Option<&ItemStack>,
        slot: usize,
    ) -> bool {
        if current.is_some_and(piglin::has_binding_curse) {
            return false;
        }
        let liked =
            |stack: &ItemStack| self.item_loved(&stack.id) || self.preferred_weapon(piglin, stack);
        let new_liked = liked(new);
        let current_liked = current.is_some_and(liked);
        if new_liked && !current_liked {
            return true;
        }
        if !new_liked && current_liked {
            return false;
        }
        let Some(current) = current else {
            return true;
        };
        if slot >= piglin::FEET {
            let (new_armor, new_toughness) = piglin::armor_value(&new.id);
            let (armor, toughness) = piglin::armor_value(&current.id);
            if new_armor != armor {
                return new_armor > armor;
            }
            if new_toughness != toughness {
                return new_toughness > toughness;
            }
            return piglin::can_replace_equal_item(new, current);
        }
        if slot != piglin::MAINHAND {
            return false;
        }
        let (new_preferred, preferred) = (
            self.preferred_weapon(piglin, new),
            self.preferred_weapon(piglin, current),
        );
        if preferred && !new_preferred {
            return false;
        }
        if !preferred && new_preferred {
            return true;
        }
        let (new_damage, damage) = (self.weapon_damage(new), self.weapon_damage(current));
        if new_damage != damage {
            return new_damage > damage;
        }
        piglin::can_replace_equal_item(new, current)
    }

    pub fn piglin_wants(&self, id: u64, stack: &ItemStack) -> bool {
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return false;
        };
        let piglin = &entity.piglin;
        if !self.mob_griefing || !piglin.can_pick_up_loot {
            return false;
        }
        if piglin.is_brute() {
            return stack.id == "minecraft:golden_axe";
        }
        let memories = &entity.brain.memories;
        if piglin.is_baby() && self.item_tagged(IGNORED_BY_BABIES_TAG, &stack.id) {
            return false;
        }
        if self.item_tagged(REPELLENT_ITEMS_TAG, &stack.id) {
            return false;
        }
        if memories.admiring_disabled.present() && memories.attack_target.present() {
            return false;
        }
        let offhand_free = !piglin
            .offhand
            .as_ref()
            .is_some_and(|held| self.item_loved(&held.id));
        if stack.id == piglin::BARTERING_ITEM {
            return offhand_free;
        }
        let room = piglin.can_add_to_inventory(stack, self.piglin_max_stack(&stack.id));
        if stack.id == piglin::GOLD_NUGGET {
            return room;
        }
        if self.item_tagged(FOOD_TAG, &stack.id) {
            return !memories.ate_recently.present() && room;
        }
        if !self.item_loved(&stack.id) {
            let slot = self.slot_for(stack);
            return self.can_replace(piglin, stack, piglin.equipment(slot), slot);
        }
        offhand_free && room
    }

    fn wanted_items_of(&self, id: u64, world: &dyn World) -> Vec<WantedItem> {
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return Vec::new();
        };
        let body = &entity.piglin.body;
        let half = f64::from(body.width) / 2.0;
        let min = body.position - DVec3::new(half + 32.0, 16.0, half + 32.0);
        let max =
            body.position + DVec3::new(half + 32.0, f64::from(body.height) + 16.0, half + 32.0);
        let eye = body.position + DVec3::Y * f64::from(entity.piglin.eye_height());
        let mut items = world.items_in(min, max);
        items.sort_by(|a, b| {
            let (a, b) = (
                a.position.distance_squared(body.position),
                b.position.distance_squared(body.position),
            );
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
        });
        items
            .into_iter()
            .filter(|item| item.count > 0)
            .filter(|item| {
                let stack = ItemStack {
                    id: item.item.clone(),
                    count: item.count.clamp(1, 255) as u8,
                    max: 64,
                    components: item.components.clone(),
                };
                self.piglin_wants(id, &stack)
            })
            .filter(|item| item.position.distance_squared(body.position) < 32.0 * 32.0)
            .filter(|item| {
                let target = item.position + DVec3::Y * crate::piglin_brain::ITEM_EYE_HEIGHT;
                crate::sight::line_of_sight(world, eye, target)
            })
            .map(|item| WantedItem {
                id: item.id,
                position: item.position,
                loved: self.item_loved(&item.item),
            })
            .collect()
    }

    fn spawn_at_location<W: World>(&mut self, world: &mut W, at: DVec3, stack: &ItemStack) {
        let x = self.level_random.next_double() * 0.2 - 0.1;
        let z = self.level_random.next_double() * 0.2 - 0.1;
        world.spawn_item(at, stack, DVec3::new(x, 0.2, z), PICKUP_DELAY);
    }

    fn throw_from(&self, id: u64) -> Option<(DVec3, DVec3)> {
        let entity = self.piglins.iter().find(|e| e.id == id)?;
        let position = entity.position();
        let from = position + DVec3::Y * (f64::from(entity.piglin.eye_height()) - 0.3);
        Some((position, from))
    }

    fn throw_target<W: World>(
        &mut self,
        id: u64,
        others: &[Living],
        world: &W,
        toward_player: bool,
    ) -> DVec3 {
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return DVec3::ZERO;
        };
        let player = entity
            .brain
            .memories
            .nearest_visible_player
            .get()
            .and_then(|&player| find(others, player))
            .map(|player| player.position);
        let target = match player.filter(|_| toward_player) {
            Some(position) => position,
            None => {
                let profile = entity.walk_profile();
                let position = entity.position();
                crate::stroll::land_random_position(
                    world,
                    &profile,
                    position,
                    4,
                    2,
                    &mut entity.random,
                )
                .unwrap_or(position)
            }
        };
        target + DVec3::Y
    }

    fn throw_velocity(position: DVec3, target: DVec3) -> DVec3 {
        (target - position).normalize_or_zero() * THROW_SPEED
    }

    fn piglin_throw<W: World>(
        &mut self,
        id: u64,
        stacks: Vec<ItemStack>,
        others: &[Living],
        world: &mut W,
        toward_player: bool,
    ) {
        let stacks: Vec<ItemStack> = stacks.into_iter().filter(|s| s.count > 0).collect();
        if stacks.is_empty() {
            return;
        }
        let Some((position, from)) = self.throw_from(id) else {
            return;
        };
        let target = self.throw_target(id, others, &*world, toward_player);
        let velocity = Self::throw_velocity(position, target);
        for stack in stacks {
            world.spawn_item(from, &stack, velocity, PICKUP_DELAY);
        }
    }

    fn piglin_put_in_inventory<W: World>(
        &mut self,
        id: u64,
        stack: ItemStack,
        others: &[Living],
        world: &mut W,
    ) {
        let max = self.piglin_max_stack(&stack.id);
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if let Some(left) = entity.piglin.add_to_inventory(stack, max) {
            self.piglin_throw(id, vec![left], others, world, false);
        }
    }

    fn piglin_equip<W: World>(
        &mut self,
        id: u64,
        stack: ItemStack,
        world: &mut W,
    ) -> Option<ItemStack> {
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return Some(stack);
        };
        let piglin = &entity.piglin;
        let mut slot = self.slot_for(&stack);
        let mut replace = self.can_replace(piglin, &stack, piglin.equipment(slot), slot);
        if slot != piglin::MAINHAND && !replace {
            slot = piglin::MAINHAND;
            replace = piglin.main_hand.is_none();
        }
        if !replace {
            return Some(stack);
        }
        let position = entity.position();
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return Some(stack);
        };
        let chance = entity.piglin.drop_chances[slot];
        let roll = (entity.random.next_float() - 0.1).max(0.0);
        let current = entity.piglin.equipment_mut(slot).and_then(Option::take);
        let mut stack = stack;
        if slot != piglin::MAINHAND {
            stack.count = 1;
        }
        if let Some(cell) = entity.piglin.equipment_mut(slot) {
            *cell = Some(stack);
        }
        entity.piglin.drop_chances[slot] = piglin::GUARANTEED_DROP;
        entity.piglin.persistence_required = true;
        if let Some(current) = current.filter(|_| roll < chance) {
            self.spawn_at_location(world, position, &current);
        }
        None
    }

    fn hold_in_offhand<W: World>(&mut self, id: u64, stack: ItemStack, world: &mut W) {
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let position = entity.position();
        let previous = entity.piglin.offhand.take();
        if stack.id != piglin::BARTERING_ITEM {
            entity.piglin.persistence_required = true;
        }
        entity.piglin.offhand = Some(stack);
        entity.piglin.drop_chances[piglin::OFFHAND] = piglin::GUARANTEED_DROP;
        if let Some(previous) = previous {
            self.spawn_at_location(world, position, &previous);
        }
    }

    fn stop_holding_offhand<W: World>(
        &mut self,
        id: u64,
        barter: bool,
        others: &[Living],
        world: &mut W,
    ) {
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let Some(stack) = entity.piglin.offhand.take() else {
            return;
        };
        let currency = stack.id == piglin::BARTERING_ITEM;
        if !entity.piglin.is_baby() {
            if barter && currency {
                let Some((position, from)) = self.throw_from(id) else {
                    return;
                };
                let target = self.throw_target(id, others, &*world, true);
                let velocity = Self::throw_velocity(position, target);
                self.barter_throws.push(BarterThrow {
                    piglin: id,
                    from,
                    velocity,
                });
            } else if !currency {
                if let Some(stack) = self.piglin_equip(id, stack, world) {
                    self.piglin_put_in_inventory(id, stack, others, world);
                }
            }
            return;
        }
        let Some(stack) = self.piglin_equip(id, stack, world) else {
            return;
        };
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let held = entity.piglin.main_hand.take();
        entity.piglin.main_hand = Some(stack);
        entity.piglin.drop_chances[piglin::MAINHAND] = piglin::GUARANTEED_DROP;
        entity.piglin.persistence_required = true;
        if let Some(held) = held {
            if self.item_loved(&held.id) {
                self.piglin_put_in_inventory(id, held, others, world);
            } else {
                self.piglin_throw(id, vec![held], others, world, true);
            }
        }
    }

    fn stop_walking(entity: &mut PiglinEntity) {
        entity.brain.memories.walk_target.erase();
        entity.brain.walk.stop(&mut entity.navigation);
        entity.navigation.stop();
    }

    fn piglin_pick_up<W: World>(&mut self, id: u64, others: &[Living], world: &mut W) {
        if !self.mob_griefing {
            return;
        }
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return;
        };
        if !entity.piglin.can_pick_up_loot || entity.piglin.health <= 0.0 {
            return;
        }
        let body = &entity.piglin.body;
        let half = f64::from(body.width) / 2.0;
        let min = body.position - DVec3::new(half + 1.0, 0.0, half + 1.0);
        let max = body.position + DVec3::new(half + 1.0, f64::from(body.height), half + 1.0);
        for item in world.items_in(min, max) {
            let Some(item) = world.item(item.id) else {
                continue;
            };
            if item.count <= 0 || item.pickup_delay > 0 {
                continue;
            }
            let stack = ItemStack {
                id: item.item.clone(),
                count: item.count.clamp(1, 255) as u8,
                max: self.piglin_max_stack(&item.item),
                components: item.components.clone(),
            };
            if !self.piglin_wants(id, &stack) {
                continue;
            }
            if self
                .piglins
                .iter()
                .any(|e| e.id == id && e.piglin.is_brute())
            {
                let one = ItemStack { count: 1, ..stack };
                if self.piglin_equip(id, one, world).is_none() {
                    world.take_item(item.id, 1);
                }
                continue;
            }
            self.piglin_picks(id, item.id, stack, others, world);
        }
    }

    fn piglin_picks<W: World>(
        &mut self,
        id: u64,
        item: i32,
        mut stack: ItemStack,
        others: &[Living],
        world: &mut W,
    ) {
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        Self::stop_walking(entity);
        if stack.id == piglin::GOLD_NUGGET {
            world.take_item(item, i32::from(stack.count));
        } else {
            world.take_item(item, 1);
            stack.count = 1;
        }
        if self.item_loved(&stack.id) {
            if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
                entity
                    .brain
                    .memories
                    .time_trying_to_reach_admire_item
                    .erase();
            }
            self.hold_in_offhand(id, stack, world);
            if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
                entity.brain.admire_for(crate::piglin_brain::ADMIRE_TICKS);
            }
            return;
        }
        let ate = self
            .piglins
            .iter()
            .find(|e| e.id == id)
            .is_some_and(|e| e.brain.memories.ate_recently.present());
        if self.item_tagged(FOOD_TAG, &stack.id) && !ate {
            if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
                entity
                    .brain
                    .memories
                    .ate_recently
                    .set_for((), crate::piglin_brain::ATE_RECENTLY_TICKS);
            }
            return;
        }
        if let Some(stack) = self.piglin_equip(id, stack, world) {
            self.piglin_put_in_inventory(id, stack, others, world);
        }
    }

    pub fn piglin_admire_held(
        &mut self,
        id: u64,
        inventory: &mut minecraftoss_player::inventory::Inventory,
        selected: usize,
        infinite: bool,
    ) -> Option<Vec<(ItemStack, DVec3)>> {
        let held = inventory.slots.get(selected)?.as_ref()?;
        if held.id != piglin::BARTERING_ITEM {
            return None;
        }
        let entity = self.piglins.iter_mut().find(|e| e.id == id)?;
        let memories = &entity.brain.memories;
        if entity.piglin.kind != PiglinKind::Piglin
            || entity.piglin.is_baby()
            || entity.piglin.health <= 0.0
            || memories.admiring_disabled.present()
            || memories.admiring_item.present()
        {
            return None;
        }
        let mut taken = held.clone();
        taken.count = 1;
        if !infinite {
            crate::animal::consume_one(inventory, selected);
        }
        let position = entity.position();
        let previous = entity.piglin.offhand.replace(taken);
        entity.piglin.drop_chances[piglin::OFFHAND] = piglin::GUARANTEED_DROP;
        entity.brain.admire_for(crate::piglin_brain::ADMIRE_TICKS);
        Self::stop_walking(entity);
        Some(
            previous
                .map(|stack| (stack, position))
                .into_iter()
                .collect(),
        )
    }

    pub fn set_universal_anger(&mut self, on: bool) {
        self.piglin_rules.universal_anger = on;
    }

    pub fn set_forgive_dead_players(&mut self, on: bool) {
        self.piglin_rules.forgive_dead_players = on;
    }

    pub fn piglin_rules(&self) -> PiglinRules {
        self.piglin_rules
    }

    fn item_safe_armor(&self, id: &str) -> bool {
        self.recipes
            .as_ref()
            .is_some_and(|recipes| recipes.item_in_tag(SAFE_ARMOR_TAG, id))
    }

    pub fn player_wears_gold(&self, player: u64) -> bool {
        self.player_controls(player)
            .is_some_and(|controls| controls.wears(|item| self.item_safe_armor(item)))
    }

    pub fn player_holds_loved(&self, player: u64) -> bool {
        self.player_controls(player)
            .is_some_and(|controls| controls.holds(|item| self.item_loved(item)))
    }

    fn target_uuid(&self, target: Target) -> u128 {
        match target {
            Target::Player(player) => self.uuid_of(PLAYER_TARGET + player),
            Target::Mob(id) | Target::Villager(id) => self.uuid_of(id),
        }
    }

    pub(super) fn weapon_damage(&self, stack: &ItemStack) -> f32 {
        match self.recipes.as_ref().filter(|r| r.item_catalog().is_some()) {
            Some(recipes) => (recipes.attack_attributes(Some(stack)).0 - 1.0) as f32,
            None => piglin::weapon_bonus(&stack.id),
        }
    }

    pub(super) fn disable_shield_seconds(&self, stack: &ItemStack) -> f32 {
        match self.recipes.as_ref().filter(|r| r.item_catalog().is_some()) {
            Some(recipes) => recipes.weapon(stack).map_or(0.0, |(_, seconds)| seconds),
            None => piglin::disables_shield_for(&stack.id),
        }
    }

    fn piglin_anger(&self, id: u64, uuids: &[(Target, u128)]) -> Option<Target> {
        let entity = self.piglins.iter().find(|e| e.id == id)?;
        let &uuid = entity.brain.memories.angry_at.get()?;
        uuids
            .iter()
            .find(|(_, other)| *other == uuid)
            .map(|(target, _)| *target)
    }

    fn set_piglin_anger(&mut self, id: u64, target: Target, others: &[Living]) {
        let uuid = self.target_uuid(target);
        let universal = self.piglin_rules.universal_anger;
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let Some(other) = find(others, target) else {
            return;
        };
        let (position, range) = (entity.position(), entity.piglin.follow_range());
        if !crate::piglin_brain::attackable_ignoring_sight(position, range, other) {
            return;
        }
        entity.brain.set_anger(uuid, other.is_player(), universal);
        let hunts = entity.piglin.kind == PiglinKind::Piglin && !entity.piglin.cannot_hunt;
        if hunts && other.kind == HOGLIN {
            self.piglin_rests_from_hunting(id);
        }
    }

    fn piglin_can_hunt(&self, id: u64) -> bool {
        self.piglins
            .iter()
            .any(|e| e.id == id && e.piglin.kind == PiglinKind::Piglin && !e.piglin.cannot_hunt)
    }

    fn hoglin_huntable(&self, target: Target) -> bool {
        let Target::Mob(id) = target else {
            return false;
        };
        self.hoglins.iter().any(|e| {
            e.id == id
                && e.hoglin.kind == crate::hoglin::HoglinKind::Hoglin
                && !e.hoglin.cannot_be_hunted
        })
    }

    fn piglin_rests_from_hunting(&mut self, id: u64) {
        use crate::piglin_brain::{TIME_BETWEEN_HUNTS_MAX, TIME_BETWEEN_HUNTS_MIN};
        let spread = (TIME_BETWEEN_HUNTS_MAX - TIME_BETWEEN_HUNTS_MIN + 1) as u32;
        let ticks = TIME_BETWEEN_HUNTS_MIN + i64::from(self.level_random.next_int(spread));
        if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
            entity.brain.memories.hunted_recently.set_for((), ticks);
        }
    }

    fn visible_adult_piglins(&self, id: u64) -> Vec<u64> {
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return Vec::new();
        };
        let adults = entity.brain.memories.nearest_visible_adult_piglins.get();
        adults
            .into_iter()
            .flatten()
            .filter_map(|&adult| match adult {
                Target::Mob(adult) => Some(adult),
                _ => None,
            })
            .collect()
    }

    fn piglin_hunts(
        &mut self,
        id: u64,
        hoglin: Target,
        others: &[Living],
        uuids: &[(Target, u128)],
    ) {
        let adults = self.visible_adult_piglins(id);
        let rested = adults.iter().any(|&adult| {
            self.piglins
                .iter()
                .any(|e| e.id == adult && e.brain.memories.hunted_recently.present())
        });
        if rested {
            return;
        }
        self.set_piglin_anger(id, hoglin, others);
        self.piglin_rests_from_hunting(id);
        self.broadcast_piglin_anger(id, hoglin, others, uuids);
        for adult in adults {
            self.piglin_rests_from_hunting(adult);
        }
    }

    fn piglin_retreats(&mut self, id: u64, target: Target) {
        use crate::piglin_brain::{RETREAT_MAX_TICKS, RETREAT_MIN_TICKS};
        let spread = (RETREAT_MAX_TICKS - RETREAT_MIN_TICKS + 1) as u32;
        let ticks = RETREAT_MIN_TICKS + i64::from(self.level_random.next_int(spread));
        if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
            let memories = &mut entity.brain.memories;
            memories.angry_at.erase();
            memories.attack_target.erase();
            memories.walk_target.erase();
            memories.avoid_target.set_for(target, ticks);
        }
        self.piglin_rests_from_hunting(id);
    }

    fn broadcast_piglin_retreat(&mut self, id: u64, target: Target, others: &[Living]) {
        for adult in self.visible_adult_piglins(id) {
            let Some(entity) = self
                .piglins
                .iter()
                .find(|e| e.id == adult && e.piglin.kind == PiglinKind::Piglin)
            else {
                continue;
            };
            let position = entity.position();
            let memories = &entity.brain.memories;
            let avoided = memories.avoid_target.get().copied();
            let nearest = crate::brain_behaviors::nearer(position, others, avoided, target);
            let attacked = memories.attack_target.get().copied();
            let nearest = crate::brain_behaviors::nearer(position, others, attacked, nearest);
            self.piglin_retreats(adult, nearest);
        }
    }

    fn set_piglin_anger_if_closer(
        &mut self,
        id: u64,
        target: Target,
        others: &[Living],
        uuids: &[(Target, u128)],
    ) {
        let current = self.piglin_anger(id, uuids);
        let Some(position) = self
            .piglins
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.position())
        else {
            return;
        };
        let nearest = crate::brain_behaviors::nearer(position, others, current, target);
        if current != Some(nearest) {
            self.set_piglin_anger(id, nearest, others);
        }
    }

    fn broadcast_piglin_anger(
        &mut self,
        id: u64,
        target: Target,
        others: &[Living],
        uuids: &[(Target, u128)],
    ) {
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return;
        };
        let hoglin = find(others, target).is_some_and(|other| other.kind == HOGLIN);
        let huntable = self.hoglin_huntable(target);
        for adult in entity.brain.memories.nearby_adult_piglins() {
            if let Target::Mob(adult) = adult {
                if hoglin && !(self.piglin_can_hunt(adult) && huntable) {
                    continue;
                }
                self.set_piglin_anger_if_closer(adult, target, others, uuids);
            }
        }
    }

    fn broadcast_piglin_universal_anger(&mut self, id: u64, others: &[Living]) {
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return;
        };
        for adult in entity.brain.memories.nearby_adult_piglins() {
            let Target::Mob(adult) = adult else {
                continue;
            };
            let player = self.piglins.iter().find(|e| e.id == adult).and_then(|e| {
                e.brain
                    .memories
                    .nearest_visible_attackable_player
                    .get()
                    .copied()
            });
            if let Some(player) = player {
                self.set_piglin_anger(adult, player, others);
            }
        }
    }

    fn piglin_maybe_retaliate(
        &mut self,
        id: u64,
        target: Target,
        others: &[Living],
        uuids: &[(Target, u128)],
    ) {
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return;
        };
        if entity.brain.active(Activity::Avoid) {
            return;
        }
        let Some(other) = find(others, target) else {
            return;
        };
        let (position, range) = (entity.position(), entity.piglin.follow_range());
        if !crate::piglin_brain::attackable_ignoring_sight(position, range, other) {
            return;
        }
        let current = entity.brain.memories.attack_target.get().copied();
        if let Some(current) = current.and_then(|current| find(others, current)) {
            let to_current = current.position.distance_squared(position);
            if other.position.distance_squared(position) > to_current + 16.0 {
                return;
            }
        }
        if other.is_player() && self.piglin_rules.universal_anger {
            let player = entity
                .brain
                .memories
                .nearest_visible_attackable_player
                .get()
                .copied()
                .unwrap_or(target);
            self.set_piglin_anger(id, player, others);
            self.broadcast_piglin_universal_anger(id, others);
        } else {
            self.set_piglin_anger(id, target, others);
            self.broadcast_piglin_anger(id, target, others, uuids);
        }
    }

    fn piglin_was_hurt_by<W: World>(
        &mut self,
        id: u64,
        attacker: Target,
        others: &[Living],
        uuids: &[(Target, u128)],
        world: &mut W,
    ) {
        let Some(other) = find(others, attacker).copied() else {
            return;
        };
        let holding = self
            .piglins
            .iter()
            .find(|e| e.id == id)
            .is_some_and(|e| e.piglin.kind == PiglinKind::Piglin && e.piglin.offhand.is_some());
        if holding && other.kind != "minecraft:piglin" {
            self.stop_holding_offhand(id, false, others, world);
        }
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entity.piglin.health <= 0.0 {
            return;
        }
        if entity.piglin.kind == PiglinKind::Brute {
            if !matches!(other.kind, "minecraft:piglin" | "minecraft:piglin_brute") {
                self.piglin_maybe_retaliate(id, attacker, others, uuids);
            }
            return;
        }
        if other.kind == "minecraft:piglin" {
            return;
        }
        let baby = entity.piglin.is_baby();
        let (position, range) = (entity.position(), entity.piglin.follow_range());
        let outnumbered = entity.brain.hoglins_outnumber_piglins();
        let memories = &mut entity.brain.memories;
        memories.celebrate_location.erase();
        memories.dancing.erase();
        memories.admiring_item.erase();
        if other.is_player() {
            memories
                .admiring_disabled
                .set_for((), crate::piglin_brain::ADMIRING_DISABLED_TICKS);
        }
        let avoided = memories
            .avoid_target
            .get()
            .and_then(|&avoided| find(others, avoided));
        if avoided.is_some_and(|avoided| avoided.kind != other.kind) {
            memories.avoid_target.erase();
        }
        if baby {
            memories
                .avoid_target
                .set_for(attacker, crate::piglin_brain::BABY_FLEE_TICKS);
            if crate::piglin_brain::attackable_ignoring_sight(position, range, &other) {
                self.broadcast_piglin_anger(id, attacker, others, uuids);
            }
            return;
        }
        if other.kind == HOGLIN && outnumbered {
            self.piglin_retreats(id, attacker);
            self.broadcast_piglin_retreat(id, attacker, others);
            return;
        }
        self.piglin_maybe_retaliate(id, attacker, others, uuids);
    }

    pub fn take_piglin_conversions(&mut self) -> Vec<PiglinConversion> {
        std::mem::take(&mut self.piglin_conversions)
    }

    pub fn convert_piglin_to_zombified<W: World>(
        &mut self,
        old_id: u64,
        world: &mut W,
    ) -> Option<u64> {
        let peaceful = self.difficulty == 0;
        let entity = self.piglins.iter_mut().find(|e| e.id == old_id)?;
        if !peaceful {
            entity.make_sound(match entity.piglin.kind {
                PiglinKind::Piglin => "entity.piglin.converted_to_zombified",
                PiglinKind::Brute => "entity.piglin_brute.converted_to_zombified",
            });
        }
        let position = entity.position();
        let mut dropped = Vec::new();
        if entity.piglin.kind == PiglinKind::Piglin {
            if entity.brain.memories.admiring_item.present() {
                dropped.extend(entity.piglin.offhand.take());
            }
            dropped.extend(entity.piglin.inventory.iter_mut().filter_map(Option::take));
        }
        for stack in &dropped {
            self.spawn_at_location(world, position, stack);
        }
        let index = self.piglins.iter().position(|e| e.id == old_id)?;
        let old = self.piglins.remove(index);
        self.order
            .retain(|key| !matches!(key, EntityKey::Piglin(id) if *id == old_id));
        let piglin = &old.piglin;
        let mut zombie = crate::zombie::Zombie::new(position);
        zombie.kind = ZombieKind::ZombifiedPiglin;
        zombie.set_baby(piglin.is_baby());
        zombie.body.velocity = piglin.body.velocity;
        zombie.body.on_ground = piglin.body.on_ground;
        zombie.body.fall_distance = piglin.body.fall_distance;
        zombie.body.fire_ticks = piglin.body.fire_ticks;
        zombie.damage.cooldown_ticks = piglin.damage.cooldown_ticks;
        zombie.persistence_required = piglin.persistence_required;
        zombie.main_hand = piglin.main_hand.as_ref().map(|stack| stack.id.clone());
        zombie.armor = piglin.armor_ids();
        zombie.head_item = piglin.armor[0]
            .as_ref()
            .map(|stack| piglin::max_damage(&stack.id).is_some());
        let id = self.spawn_zombie_active(zombie, piglin.yaw);
        let equipment = std::array::from_fn(|slot| piglin.equipment(slot).cloned());
        self.piglin_conversions.push(PiglinConversion {
            zombified: id,
            equipment,
            drop_chances: piglin.drop_chances,
            can_pick_up_loot: piglin.can_pick_up_loot,
            left_handed: piglin.left_handed,
        });
        let entity = self.zombie_mut(id)?;
        entity.effects = old.effects;
        entity.effects.add(crate::effects::EffectInstance::new(
            MobEffect::Nausea,
            piglin::NAUSEA_TICKS,
            0,
        ));
        entity.voices = old.voices;
        entity.random = old.random;
        entity.look_control.head_yaw = old.look_control.head_yaw;
        entity.body_rotation.body_yaw = old.body_rotation.body_yaw;
        Some(id)
    }

    pub fn anger_nearby_piglins(&mut self, player: u64, feet: DVec3, require_sight: bool) {
        self.piglin_angers.push(PiglinAnger {
            player,
            feet,
            require_sight,
        });
    }

    pub(super) fn resolve_piglin_angers(&mut self, players: &[PlayerCandidate]) {
        let angers = std::mem::take(&mut self.piglin_angers);
        if angers.is_empty() {
            return;
        }
        let others: Vec<Living> = self
            .hoglin_living(u64::MAX, players)
            .into_iter()
            .filter(Living::is_player)
            .collect();
        for anger in angers {
            let half = ANGER_PLAYER_WIDTH / 2.0 + ANGER_RANGE;
            let min = anger.feet - DVec3::new(half, ANGER_RANGE, half);
            let max = anger.feet + DVec3::new(half, ANGER_PLAYER_HEIGHT + ANGER_RANGE, half);
            let player = Target::Player(anger.player);
            let angered: Vec<(u64, Option<Target>)> = self
                .piglins
                .iter()
                .filter(|e| e.piglin.kind == PiglinKind::Piglin)
                .filter(|e| {
                    let body = &e.piglin.body;
                    let w = f64::from(body.width) / 2.0;
                    let (low, high) = (
                        body.position - DVec3::new(w, 0.0, w),
                        body.position + DVec3::new(w, f64::from(body.height), w),
                    );
                    low.cmplt(max).all() && high.cmpgt(min).all()
                })
                .filter(|e| e.brain.active(Activity::Idle))
                .filter(|e| !anger.require_sight || e.brain.memories.sees(player))
                .map(|e| {
                    let nearest = e.brain.memories.nearest_visible_attackable_player.get();
                    (e.id, nearest.copied())
                })
                .collect();
            for (id, nearest) in angered {
                let target = if self.piglin_rules.universal_anger {
                    nearest.unwrap_or(player)
                } else {
                    player
                };
                self.set_piglin_anger(id, target, &others);
            }
        }
    }

    pub fn note_piglin_hurt(&mut self, id: u64, attacker: Target) {
        if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
            entity.hurt_by.push(attacker);
        }
    }

    fn piglin_hits(&mut self, id: u64, target: Target) {
        let Some(entity) = self.piglins.iter().find(|e| e.id == id) else {
            return;
        };
        let position = entity.position();
        let mut damage = entity.piglin.attack_damage();
        let mut disable_shield_seconds = 0.0;
        if let Some(stack) = entity.piglin.main_hand.clone() {
            damage += self.weapon_damage(&stack);
            disable_shield_seconds = self.disable_shield_seconds(&stack);
        }
        if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
            entity.swings += 1;
        }
        match target {
            Target::Player(player_id) => self.player_hits.push(PlayerHit {
                player_id,
                damage,
                kind: PlayerHitKind::Melee {
                    attacker: position,
                    hunger_ticks: 0,
                    lift: 0.0,
                    throw: DVec3::ZERO,
                    disable_shield_seconds,
                },
                source: Some(id),
            }),
            Target::Mob(victim) | Target::Villager(victim) => {
                self.mob_hits_mob(id, victim, damage, position, 0.0);
                let killed = self.hoglins.iter().any(|e| {
                    e.id == victim
                        && e.hoglin.kind == crate::hoglin::HoglinKind::Hoglin
                        && e.hoglin.health <= 0.0
                });
                if killed {
                    self.piglin_rests_from_hunting(id);
                }
            }
        }
    }

    fn piglin_shoots(&mut self, id: u64, target: Target, others: &[Living]) {
        let Some(other) = find(others, target).copied() else {
            return;
        };
        let uncertainty = (14 - self.difficulty * 4) as f32;
        let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if !entity.piglin.crossbow_charged() {
            return;
        }
        let position = entity.position();
        let arrow = crate::projectile::Arrow::skeleton_shot(
            position,
            entity.piglin.eye_height(),
            other.position,
            other.height,
            uncertainty,
            &mut entity.random,
        );
        entity.piglin.set_crossbow_charged(false);
        entity.no_action_time = 0;
        let pitch = 1.0 / (entity.random.next_float() * 0.4 + 0.8);
        entity
            .voices
            .push((Voice::Event("item.crossbow.shoot", 1.0, pitch), position));
        let arrow_id = self.spawn_arrow(id, arrow);
        if let Some(arrow) = self.arrows.iter_mut().find(|e| e.id == arrow_id) {
            arrow.hit_sound = "item.crossbow.hit";
        }
    }

    pub(super) fn tick_piglin(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let game_time = self.game_time;
        let others = self.hoglin_living(id, players);
        let uuids: Vec<(Target, u128)> = others
            .iter()
            .map(|other| (other.target, self.target_uuid(other.target)))
            .collect();
        let hurts = match self.piglins.iter_mut().find(|e| e.id == id) {
            Some(entity) => std::mem::take(&mut entity.hurt_by),
            None => return,
        };
        for attacker in hurts {
            self.piglin_was_hurt_by(id, attacker, &others, &uuids, world);
        }
        let wanted_items = self
            .piglins
            .iter()
            .find(|e| e.id == id)
            .filter(|e| e.brain.sees_items_this_tick())
            .map(|_| self.wanted_items_of(id, &*world));
        let offhand_loved = self.piglin_offhand_loved(id);
        let players_of = |test: &dyn Fn(u64) -> bool| -> Vec<Target> {
            others
                .iter()
                .filter_map(|other| match other.target {
                    Target::Player(player) if test(player) => Some(other.target),
                    _ => None,
                })
                .collect()
        };
        let gilded = players_of(&|player| self.player_wears_gold(player));
        let loved_holders = players_of(&|player| self.player_holds_loved(player));
        let unhuntable: Vec<Target> = self
            .hoglins
            .iter()
            .filter(|e| e.hoglin.cannot_be_hunted)
            .map(|e| Target::Mob(e.id))
            .collect();
        let sees = PiglinSees {
            others: &others,
            uuids: &uuids,
            gilded: &gilded,
            loved_holders: &loved_holders,
            unhuntable: &unhuntable,
            rules: self.piglin_rules,
            wanted_items: wanted_items.as_deref(),
            offhand_loved,
            zombify: self.piglins_zombify,
        };
        let dimension = self.dimension.clone();
        let mut level_random = std::mem::take(&mut self.level_random);
        let ticked = self.piglins.iter_mut().find(|e| e.id == id).map(|entity| {
            entity.tick_body(
                &*world,
                players,
                &sees,
                game_time,
                &dimension,
                &mut level_random,
            )
        });
        let Some((stepped, thought)) = ticked else {
            self.level_random = level_random;
            return;
        };
        for (pos, open) in thought.doors {
            let pitch = level_random.next_float() * 0.1 + 0.9;
            if let Some(event) = super::villagers::set_door_open(world, pos, open) {
                let center = DVec3::new(
                    f64::from(pos.0) + 0.5,
                    f64::from(pos.1) + 0.5,
                    f64::from(pos.2) + 0.5,
                );
                if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
                    entity
                        .voices
                        .push((Voice::Event(event, 1.0, pitch), center));
                }
            }
        }
        self.level_random = level_random;
        if !stepped {
            return;
        }
        if let Some(hoglin) = thought.hunt {
            self.piglin_hunts(id, hoglin, &others, &uuids);
        }
        if let Some(target) = thought.attack {
            self.piglin_hits(id, target);
        }
        if let Some(target) = thought.shot {
            self.piglin_shoots(id, target, &others);
        }
        if let Some(barter) = thought.stop_holding {
            self.stop_holding_offhand(id, barter, &others, world);
        }
        if thought.convert {
            self.convert_piglin_to_zombified(id, world);
            return;
        }
        self.piglin_pick_up(id, &others, world);
        let offhand = self
            .piglins
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.piglin.offhand.as_ref())
            .map(|stack| stack.id.clone());
        let loved = offhand.is_some_and(|item| self.item_loved(&item));
        if let Some(entity) = self.piglins.iter_mut().find(|e| e.id == id) {
            entity.arm_pose = entity.piglin.arm_pose(loved);
        }
        self.push_entities(EntityKey::Piglin(id), &*world, game_time, ticks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piglin_brain::GlobalPos;
    use minecraftoss_player::{Block, Pos};
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
                    .insert((x, 63, z), Block::new("minecraft:netherrack"));
            }
        }
        world
    }

    fn spawn(world: &mut EntityWorld, kind: PiglinKind, x: f64, z: f64) -> u64 {
        let piglin = Piglin::new(kind, DVec3::new(x, 64.0, z));
        world.spawn_piglin(piglin, false)
    }

    fn attack(damage: f64) -> PlayerAttack {
        PlayerAttack {
            player_id: 7,
            position: DVec3::new(0.5, 64.0, 3.5),
            yaw: 180.0,
            attack_damage: damage,
            strength: 1.0,
            sprinting: false,
            can_critical: false,
            can_sweep: false,
        }
    }

    #[test]
    fn piglins_and_brutes_are_monsters_of_their_own_types() {
        let mut world = EntityWorld::default();
        let piglin = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        let brute = spawn(&mut world, PiglinKind::Brute, 3.5, 0.5);
        assert_eq!(world.entity_type(piglin), Some("minecraft:piglin"));
        assert_eq!(world.entity_type(brute), Some("minecraft:piglin_brute"));
        let candidates = world.mob_candidates();
        assert!(
            candidates
                .iter()
                .any(|c| c.id == brute && c.kind == "minecraft:piglin_brute")
        );
        world.piglin_mut(piglin).unwrap().piglin.set_baby(true);
        assert!(
            world
                .mob_candidates()
                .iter()
                .any(|c| c.id == piglin && c.baby)
        );
    }

    #[test]
    fn idle_piglins_stroll_about() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        world.piglin_mut(id).unwrap().set_random_seed(5);
        let start = world.piglin_mut(id).unwrap().position();
        let mut furthest: f64 = 0.0;
        for _ in 0..600 {
            world.tick_with_players(&mut blocks, &[]);
            let now = world.piglin_mut(id).unwrap().position();
            furthest = furthest.max(now.distance(start));
        }
        assert!(furthest > 1.5, "it walked {furthest}");
        assert_eq!(
            world.piglin_mut(id).unwrap().brain.activity,
            crate::piglin_brain::Activity::Idle
        );
    }

    #[test]
    fn a_brute_strolls_back_to_its_home() {
        let mut blocks = floor(16);
        let mut world = EntityWorld::default();
        world.set_dimension("minecraft:the_nether");
        let id = spawn(&mut world, PiglinKind::Brute, 10.5, 0.5);
        let entity = world.piglin_mut(id).unwrap();
        entity.set_random_seed(3);
        entity.brain.memories.home.set(GlobalPos {
            dimension: "minecraft:the_nether".to_owned(),
            pos: (0, 64, 0),
        });
        let mut nearest = f64::MAX;
        for _ in 0..1200 {
            world.tick_with_players(&mut blocks, &[]);
            let now = world.piglin_mut(id).unwrap().position();
            nearest = nearest.min(now.distance(DVec3::new(0.5, 64.0, 0.5)));
        }
        assert!(nearest < 4.0, "closest to home {nearest}");
    }

    #[test]
    fn a_piglin_dies_dropping_its_inventory_and_lucky_gear() {
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        let entity = world.piglin_mut(id).unwrap();
        entity.piglin.inventory[0] = Some(ItemStack::new("minecraft:gold_nugget", 9));
        entity.piglin.main_hand = Some(ItemStack::new("minecraft:golden_sword", 1));
        entity.piglin.offhand = Some(ItemStack::new("minecraft:gold_ingot", 1));
        entity.piglin.drop_chances[piglin::OFFHAND] = 2.0;
        assert!(world.player_attack(&attack(100.0), id).died);
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        assert_eq!(deaths[0].table, Some("minecraft:piglin"));
        assert_eq!(deaths[0].experience, 5);
        let ids: Vec<&str> = deaths[0].equipment.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"minecraft:gold_nugget"), "{ids:?}");
        assert!(ids.contains(&"minecraft:gold_ingot"), "{ids:?}");
        let ingot = deaths[0]
            .equipment
            .iter()
            .find(|s| s.id == "minecraft:gold_ingot")
            .unwrap();
        assert!(
            ingot.components.is_none(),
            "a guaranteed drop keeps its wear"
        );
    }

    #[test]
    fn a_brute_earns_twenty_and_more_per_held_item() {
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, PiglinKind::Brute, 0.5, 0.5);
        world.piglin_mut(id).unwrap().piglin.main_hand =
            Some(ItemStack::new("minecraft:golden_axe", 1));
        assert!(world.player_attack(&attack(100.0), id).died);
        let deaths = world.take_deaths();
        let axe = deaths[0]
            .equipment
            .iter()
            .any(|s| s.id == "minecraft:golden_axe");
        let experience = deaths[0].experience;
        if axe {
            assert_eq!(experience, 20);
        } else {
            assert!((21..=23).contains(&experience), "{experience}");
        }
        assert_eq!(deaths[0].table, Some("minecraft:piglin_brute"));
    }

    #[test]
    fn golden_armour_softens_a_blow() {
        let mut world = EntityWorld::default();
        let bare = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        let armoured = spawn(&mut world, PiglinKind::Piglin, 4.5, 0.5);
        let entity = world.piglin_mut(armoured).unwrap();
        entity.piglin.armor[1] = Some(ItemStack::new("minecraft:golden_chestplate", 1));
        world.player_attack(&attack(10.0), bare);
        world.player_attack(&attack(10.0), armoured);
        let health = |world: &mut EntityWorld, id| world.piglin_mut(id).unwrap().piglin.health;
        assert!(health(&mut world, armoured) > health(&mut world, bare));
    }

    fn piglin_book() -> std::sync::Arc<minecraftoss_player::crafting::RecipeBook> {
        use std::io::Write;
        let jar = std::env::temp_dir().join(format!(
            "piglin-tags-{}-{:?}.jar",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        let tags = [
            (
                "piglin_safe_armor",
                serde_json::json!({ "values": [
                    "minecraft:golden_helmet",
                    "minecraft:golden_chestplate",
                    "minecraft:golden_leggings",
                    "minecraft:golden_boots"
                ] }),
            ),
            (
                "piglin_loved",
                serde_json::json!({ "values": [
                    "minecraft:gold_ingot",
                    "minecraft:golden_carrot",
                    "minecraft:golden_sword"
                ] }),
            ),
            (
                "piglin_food",
                serde_json::json!({ "values": ["minecraft:porkchop", "minecraft:cooked_porkchop"] }),
            ),
            (
                "ignored_by_piglin_babies",
                serde_json::json!({ "values": ["minecraft:leather"] }),
            ),
            (
                "piglin_repellents",
                serde_json::json!({ "values": ["minecraft:soul_torch"] }),
            ),
            (
                "piglin_preferred_weapons",
                serde_json::json!({ "values": ["minecraft:crossbow", "minecraft:golden_spear"] }),
            ),
        ];
        for (tag, value) in tags {
            zip.start_file(
                format!("data/minecraft/tags/item/{tag}.json"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(value.to_string().as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        let book = minecraftoss_player::crafting::RecipeBook::from_jar(&jar).unwrap();
        let _ = std::fs::remove_file(&jar);
        std::sync::Arc::new(book)
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

    fn wearing(id: u64, armor: [Option<&str>; 4]) -> crate::controls::PlayerControls {
        crate::controls::PlayerControls {
            id,
            armor: armor.map(|item| item.map(str::to_owned)),
            ..Default::default()
        }
    }

    fn armed(world: &mut EntityWorld, kind: PiglinKind, at: DVec3, item: &str) -> u64 {
        let mut piglin = Piglin::new(kind, at);
        piglin.main_hand = Some(ItemStack::new(item, 1));
        let id = world.spawn_piglin(piglin, false);
        world.piglin_mut(id).unwrap().set_random_seed(id * 31);
        id
    }

    fn slowed(world: &mut EntityWorld, id: u64) {
        let slowness = crate::effects::EffectInstance::new(MobEffect::Slowness, 10_000, 9);
        world.piglin_mut(id).unwrap().effects.add(slowness);
    }

    #[test]
    fn one_golden_piece_makes_a_player_safe() {
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let pieces = [
            "minecraft:golden_boots",
            "minecraft:golden_leggings",
            "minecraft:golden_chestplate",
            "minecraft:golden_helmet",
        ];
        for (slot, piece) in pieces.into_iter().enumerate() {
            let mut armor = [None; 4];
            armor[slot] = Some(piece);
            world.set_player_controls(&[wearing(1, armor)]);
            assert!(world.player_wears_gold(1), "{piece}");
        }
        world.set_player_controls(&[wearing(
            1,
            [None, None, None, Some("minecraft:chainmail_helmet")],
        )]);
        assert!(!world.player_wears_gold(1));
        world.set_player_controls(&[crate::controls::PlayerControls {
            id: 1,
            offhand: Some("minecraft:gold_ingot".to_owned()),
            ..Default::default()
        }]);
        assert!(!world.player_wears_gold(1));
        assert!(world.player_holds_loved(1));
    }

    #[test]
    fn the_gear_of_each_player_reaches_the_piglins() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        slowed(&mut world, id);
        let players = [
            player(1, DVec3::new(3.5, 64.0, 0.5)),
            player(2, DVec3::new(-5.5, 64.0, 0.5)),
        ];
        world.set_player_controls(&[
            wearing(1, [Some("minecraft:golden_boots"), None, None, None]),
            wearing(2, [None, None, Some("minecraft:diamond_chestplate"), None]),
        ]);
        for _ in 0..40 {
            world.tick_with_players(&mut blocks, &players);
        }
        let memories = &world.piglin_mut(id).unwrap().brain.memories;
        assert_eq!(memories.attack_target.get(), Some(&Target::Player(2)));
        assert_eq!(
            memories.nearest_targetable_player_not_wearing_gold.get(),
            Some(&Target::Player(2))
        );
    }

    #[test]
    fn a_creative_or_spectator_player_is_never_a_target() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        let brute = armed(
            &mut world,
            PiglinKind::Brute,
            DVec3::new(0.5, 64.0, 4.5),
            "minecraft:golden_axe",
        );
        let mut creative = player(1, DVec3::new(3.5, 64.0, 0.5));
        creative.attackable = false;
        let mut spectator = player(2, DVec3::new(-3.5, 64.0, 0.5));
        spectator.spectator = true;
        for _ in 0..100 {
            world.tick_with_players(&mut blocks, &[creative, spectator]);
        }
        for id in [id, brute] {
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            assert!(!memories.attack_target.present(), "{id}");
        }
        assert!(world.take_player_hits().is_empty());
    }

    #[test]
    fn a_baby_never_starts_attacking_and_flees_its_attacker_for_100_ticks() {
        let mut blocks = floor(20);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let baby = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        let entity = world.piglin_mut(baby).unwrap();
        entity.piglin.set_baby(true);
        entity.set_random_seed(4);
        let players = [player(7, DVec3::new(0.5, 64.0, 3.5))];
        for _ in 0..60 {
            world.tick_with_players(&mut blocks, &players);
        }
        let memories = &world.piglin_mut(baby).unwrap().brain.memories;
        assert!(!memories.attack_target.present());
        assert!(world.player_attack(&attack(0.5), baby).hurt);
        world.tick_with_players(&mut blocks, &players);
        let brain = &world.piglin_mut(baby).unwrap().brain;
        assert_eq!(brain.memories.avoid_target.get(), Some(&Target::Player(7)));
        assert!((98..=100).contains(&brain.memories.avoid_target.ttl().unwrap()));
        assert_eq!(brain.activity, crate::piglin_brain::Activity::Avoid);
        let from = players[0].position;
        let start = world.piglin_mut(baby).unwrap().position().distance(from);
        for _ in 0..60 {
            world.tick_with_players(&mut blocks, &players);
        }
        let after = world.piglin_mut(baby).unwrap().position().distance(from);
        assert!(after > start + 2.0, "{start} {after}");
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &players);
        }
        let brain = &world.piglin_mut(baby).unwrap().brain;
        assert!(!brain.memories.avoid_target.present());
        assert!(!brain.memories.attack_target.present());
    }

    #[test]
    fn a_hit_angers_the_piglin_and_its_adult_neighbours_for_600_ticks() {
        let mut blocks = floor(24);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let spots = [
            (PiglinKind::Piglin, 0.5),
            (PiglinKind::Piglin, 10.5),
            (PiglinKind::Piglin, -20.5),
            (PiglinKind::Brute, -4.5),
        ];
        let ids: Vec<u64> = spots
            .iter()
            .map(|&(kind, x)| {
                let id = armed(
                    &mut world,
                    kind,
                    DVec3::new(x, 64.0, 0.5),
                    "minecraft:golden_sword",
                );
                slowed(&mut world, id);
                id
            })
            .collect();
        let players = [player(7, DVec3::new(0.5, 64.0, 3.5))];
        world.set_player_controls(&[wearing(
            7,
            [None, None, None, Some("minecraft:golden_helmet")],
        )]);
        let mut brute_hits = 0;
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &players);
            brute_hits += world.take_player_hits().len();
        }
        assert_eq!(brute_hits, 0, "the slowed brute cannot reach the player");
        for &id in &ids[..3] {
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            assert!(!memories.angry_at.present(), "gold keeps them calm: {id}");
        }
        assert!(world.player_attack(&attack(1.0), ids[0]).hurt);
        world.tick_with_players(&mut blocks, &players);
        let uuid = world.uuid_of(PLAYER_TARGET + 7);
        for (index, &id) in ids.iter().enumerate() {
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            if index == 2 {
                assert!(!memories.angry_at.present(), "20 blocks away stays calm");
            } else {
                assert_eq!(memories.angry_at.get(), Some(&uuid), "{index}");
                let ttl = memories.angry_at.ttl().unwrap();
                assert!((598..=600).contains(&ttl), "{index} {ttl}");
            }
        }
    }

    #[test]
    fn a_brute_attacks_a_player_in_golden_armour_and_hits_for_13_on_normal() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let brute = armed(
            &mut world,
            PiglinKind::Brute,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_axe",
        );
        let players = [player(7, DVec3::new(0.5, 64.0, 2.5))];
        world.set_player_controls(&[wearing(
            7,
            [
                Some("minecraft:golden_boots"),
                Some("minecraft:golden_leggings"),
                Some("minecraft:golden_chestplate"),
                Some("minecraft:golden_helmet"),
            ],
        )]);
        let mut hits = Vec::new();
        for _ in 0..80 {
            world.tick_with_players(&mut blocks, &players);
            hits.extend(world.take_player_hits());
        }
        assert!(!hits.is_empty());
        assert_eq!((hits[0].damage, hits[0].source), (13.0, Some(brute)));
        let PlayerHitKind::Melee {
            disable_shield_seconds,
            ..
        } = hits[0].kind
        else {
            panic!("a melee hit");
        };
        assert_eq!(disable_shield_seconds, 5.0);
        assert!(world.piglins()[0].swings >= 1);
        assert_eq!(
            world.piglins()[0].arm_pose,
            ArmPose::AttackingWithMeleeWeapon
        );
    }

    #[test]
    fn a_crossbow_piglin_charges_25_ticks_waits_20_to_39_and_shoots_a_non_critical_arrow() {
        let mut blocks = floor(16);
        let mut shots = Vec::new();
        for seed in 0..6 {
            let mut world = EntityWorld::default();
            world.set_recipe_book(piglin_book());
            let id = armed(
                &mut world,
                PiglinKind::Piglin,
                DVec3::new(0.5, 64.0, 0.5),
                "minecraft:crossbow",
            );
            world.piglin_mut(id).unwrap().set_random_seed(seed);
            slowed(&mut world, id);
            let players = [player(7, DVec3::new(0.5, 64.0, 6.5))];
            let (mut started, mut charged, mut shot) = (None, None, None);
            let mut events = Vec::new();
            for tick in 0..200 {
                world.tick_with_players(&mut blocks, &players);
                let entity = world.piglins().iter().find(|e| e.id == id).unwrap();
                if started.is_none() && entity.using_item == Some(0) {
                    started = Some(tick);
                    assert_eq!(entity.arm_pose, ArmPose::CrossbowCharge);
                }
                if charged.is_none() && entity.piglin.crossbow_charged() {
                    charged = Some(tick);
                    assert_eq!(entity.arm_pose, ArmPose::CrossbowHold);
                }
                if shot.is_none() && !world.arrows().is_empty() {
                    shot = Some(tick);
                    assert!(!world.piglins()[0].piglin.crossbow_charged());
                }
                events.extend(world.take_sounds().into_iter().map(|s| s.event));
                if shot.is_some() {
                    break;
                }
            }
            let (started, charged, shot) = (started.unwrap(), charged.unwrap(), shot.unwrap());
            assert_eq!(charged - started, 25, "{seed}");
            assert!(
                (21..=40).contains(&(shot - charged)),
                "{seed}: {}",
                shot - charged
            );
            shots.push(shot - charged);
            let arrow = &world.arrows()[0];
            assert_eq!(arrow.owner_id, id);
            assert_eq!(arrow.arrow.base_damage, 2.0);
            assert_eq!(arrow.hit_sound, "item.crossbow.hit");
            assert!((arrow.arrow.velocity.length() - 1.6).abs() < 0.2);
            for event in [
                "item.crossbow.loading_start",
                "item.crossbow.loading_middle",
                "item.crossbow.loading_end",
                "item.crossbow.shoot",
            ] {
                assert!(events.iter().any(|e| e == event), "{seed} {event}");
            }
        }
        shots.sort();
        assert!(shots.first() != shots.last(), "{shots:?}");
    }

    #[test]
    fn a_crossbow_piglin_backs_off_inside_five_blocks() {
        let mut blocks = floor(16);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:crossbow",
        );
        let players = [player(7, DVec3::new(0.5, 64.0, 3.0))];
        let from = players[0].position;
        for _ in 0..25 {
            world.tick_with_players(&mut blocks, &players);
        }
        let start = world.piglin_mut(id).unwrap().position().distance(from);
        for _ in 0..30 {
            world.tick_with_players(&mut blocks, &players);
        }
        let after = world.piglin_mut(id).unwrap().position().distance(from);
        assert!(after > start + 0.5, "{start} {after}");
        assert!(
            world.take_player_hits().is_empty(),
            "a crossbow never melees"
        );
    }

    #[test]
    fn anger_ends_after_its_target_dies_with_forgive_dead_players() {
        for forgive in [true, false] {
            let mut blocks = floor(12);
            let mut world = EntityWorld::default();
            world.set_recipe_book(piglin_book());
            world.set_forgive_dead_players(forgive);
            let id = armed(
                &mut world,
                PiglinKind::Piglin,
                DVec3::new(0.5, 64.0, 0.5),
                "minecraft:golden_sword",
            );
            slowed(&mut world, id);
            let mut players = [player(7, DVec3::new(0.5, 64.0, 6.5))];
            world.set_player_controls(&[wearing(
                7,
                [Some("minecraft:golden_boots"), None, None, None],
            )]);
            for _ in 0..25 {
                world.tick_with_players(&mut blocks, &players);
            }
            world.note_piglin_hurt(id, Target::Player(7));
            for _ in 0..3 {
                world.tick_with_players(&mut blocks, &players);
            }
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            assert!(memories.angry_at.present());
            assert_eq!(memories.attack_target.get(), Some(&Target::Player(7)));
            players[0].alive = false;
            world.tick_with_players(&mut blocks, &players);
            let brain = &world.piglin_mut(id).unwrap().brain;
            assert!(brain.memories.celebrate_location.present(), "{forgive}");
            assert_eq!(brain.memories.angry_at.present(), !forgive);
            assert!(!brain.memories.attack_target.present());
        }
    }

    #[test]
    fn wither_skeletons_and_piglins_fight() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let piglin = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        let mut skeleton = crate::skeleton::Skeleton::of_kind(
            crate::skeleton::SkeletonKind::WitherSkeleton,
            DVec3::new(6.5, 64.0, 0.5),
        );
        skeleton.main_hand = Some("minecraft:stone_sword".to_owned());
        let skeleton = world.spawn_skeleton_active(skeleton, 0.0);
        let (mut piglin_target, mut skeleton_target) = (false, false);
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &[]);
            if let Some(entity) = world.piglins().iter().find(|e| e.id == piglin) {
                let target = entity.brain.memories.attack_target.get();
                piglin_target |= target == Some(&Target::Mob(skeleton));
            }
            if let Some(entity) = world.skeletons().iter().find(|e| e.id == skeleton) {
                let target = entity.ai.as_ref().unwrap().state.target().map(|t| t.target);
                skeleton_target |= target == Some(Target::Mob(piglin));
            }
        }
        assert!(piglin_target && skeleton_target);
        let piglin_hurt = world
            .piglins()
            .iter()
            .find(|e| e.id == piglin)
            .is_none_or(|e| e.piglin.health < 16.0);
        assert!(piglin_hurt);
    }

    fn repelled_by(block: Block) -> (bool, bool) {
        let mut blocks = floor(16);
        blocks.blocks.insert((3, 64, 0), block);
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        world.piglin_mut(id).unwrap().set_random_seed(8);
        let (mut remembered, mut fled) = (false, false);
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &[]);
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            remembered |= memories.nearest_repellent.get() == Some(&(3, 64, 0));
            fled |= memories
                .walk_target
                .get()
                .is_some_and(|goal| goal.speed == 1.0);
        }
        (remembered, fled)
    }

    #[test]
    fn a_piglin_walks_away_from_a_soul_torch_within_eight_and_ignores_a_normal_torch() {
        assert_eq!(
            repelled_by(Block::new("minecraft:soul_torch")),
            (true, true)
        );
        assert_eq!(repelled_by(Block::new("minecraft:torch")), (false, false));
        let mut blocks = floor(16);
        blocks
            .blocks
            .insert((3, 64, 0), Block::new("minecraft:soul_lantern"));
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        for _ in 0..25 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let brain = &world.piglin_mut(id).unwrap().brain;
        assert!(brain.near_repellent());
        assert_eq!(
            brain.activity_sound(DVec3::ZERO, &[], false),
            "entity.piglin.retreat"
        );
    }

    #[test]
    fn an_unlit_soul_campfire_repels_nobody() {
        let campfire = Block::new("minecraft:soul_campfire");
        assert_eq!(
            repelled_by(campfire.clone().with("lit", "false")),
            (false, false)
        );
        assert_eq!(repelled_by(campfire.with("lit", "true")), (true, true));
    }

    #[test]
    fn a_piglin_runs_from_a_zombified_piglin_within_six_and_drops_its_target() {
        let mut blocks = floor(20);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        slowed(&mut world, id);
        let players = [player(7, DVec3::new(0.5, 64.0, 5.5))];
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &players);
        }
        let memories = &world.piglin_mut(id).unwrap().brain.memories;
        assert_eq!(memories.attack_target.get(), Some(&Target::Player(7)));
        let mut zombified = crate::zombie::Zombie::new(DVec3::new(-3.5, 64.0, 0.5));
        zombified.kind = crate::zombie::ZombieKind::ZombifiedPiglin;
        let zombified = world.spawn_zombie(zombified, true);
        world.piglin_mut(id).unwrap().effects = Default::default();
        let mut avoided = None;
        for _ in 0..25 {
            world.tick_with_players(&mut blocks, &players);
            let brain = &world.piglin_mut(id).unwrap().brain;
            if brain.memories.avoid_target.get() == Some(&Target::Mob(zombified)) {
                avoided = brain.memories.avoid_target.ttl();
                assert!(!brain.memories.attack_target.present());
                break;
            }
        }
        let ttl = avoided.expect("it avoids the zombified piglin");
        assert!((98..=140).contains(&ttl), "{ttl}");
        let from = DVec3::new(-3.5, 64.0, 0.5);
        let start = world.piglin_mut(id).unwrap().position().distance(from);
        for _ in 0..60 {
            world.tick_with_players(&mut blocks, &players);
        }
        let entity = &world.piglins()[0];
        let after = entity.position().distance(from);
        assert!(after > start + 2.0, "{start} {after}");
        assert!(!entity.brain.memories.attack_target.present());
    }

    #[test]
    fn a_baby_runs_from_a_wither_skeleton_and_an_adult_attacks_it() {
        let mut blocks = floor(16);
        let mut world = EntityWorld::default();
        let baby = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        world.piglin_mut(baby).unwrap().piglin.set_baby(true);
        let adult = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, -3.5),
            "minecraft:golden_sword",
        );
        let skeleton = crate::skeleton::Skeleton::of_kind(
            crate::skeleton::SkeletonKind::WitherSkeleton,
            DVec3::new(6.5, 64.0, 0.5),
        );
        let skeleton = world.spawn_skeleton(skeleton, true);
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let brain = &world.piglin_mut(baby).unwrap().brain;
        assert_eq!(
            brain.memories.avoid_target.get(),
            Some(&Target::Mob(skeleton))
        );
        assert!((55..=140).contains(&brain.memories.avoid_target.ttl().unwrap()));
        assert_eq!(brain.activity, crate::piglin_brain::Activity::Avoid);
        let brain = &world.piglin_mut(adult).unwrap().brain;
        assert_eq!(
            brain.memories.attack_target.get(),
            Some(&Target::Mob(skeleton))
        );
        assert!(!brain.memories.avoid_target.present());
    }

    #[derive(Default)]
    struct Ground {
        blocks: Blocks,
        items: Vec<minecraftoss_player::WorldItem>,
        next: i32,
    }

    impl Ground {
        fn new(radius: i32) -> Self {
            Self {
                blocks: floor(radius),
                ..Default::default()
            }
        }

        fn drop(&mut self, item: &str, count: i32, at: DVec3) -> i32 {
            self.next += 1;
            self.items.push(minecraftoss_player::WorldItem {
                id: self.next,
                position: at,
                item: item.to_owned(),
                count,
                components: None,
                pickup_delay: 0,
            });
            self.next
        }

        fn has(&self, id: i32) -> bool {
            self.items.iter().any(|item| item.id == id)
        }
    }

    impl World for Ground {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.blocks.block(pos)
        }
        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            self.blocks.set_block(pos, block);
        }
        fn items_in(&self, min: DVec3, max: DVec3) -> Vec<minecraftoss_player::WorldItem> {
            self.items
                .iter()
                .filter(|item| {
                    let p = item.position;
                    p.x + 0.125 > min.x
                        && p.x - 0.125 < max.x
                        && p.y + 0.25 > min.y
                        && p.y < max.y
                        && p.z + 0.125 > min.z
                        && p.z - 0.125 < max.z
                })
                .cloned()
                .collect()
        }
        fn item(&self, id: i32) -> Option<minecraftoss_player::WorldItem> {
            self.items.iter().find(|item| item.id == id).cloned()
        }
        fn take_item(&mut self, id: i32, count: i32) {
            if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
                item.count -= count;
            }
            self.items.retain(|item| item.count > 0);
        }
        fn spawn_item(&mut self, position: DVec3, stack: &ItemStack, _velocity: DVec3, delay: i32) {
            self.next += 1;
            self.items.push(minecraftoss_player::WorldItem {
                id: self.next,
                position,
                item: stack.id.clone(),
                count: i32::from(stack.count),
                components: stack.components.clone(),
                pickup_delay: delay,
            });
        }
    }

    fn calm_player(world: &mut EntityWorld, at: DVec3) -> [PlayerCandidate; 1] {
        world.set_player_controls(&[wearing(
            7,
            [Some("minecraft:golden_boots"), None, None, None],
        )]);
        [player(7, at)]
    }

    fn offhand(world: &EntityWorld, id: u64) -> Option<String> {
        let entity = world.piglins().iter().find(|e| e.id == id)?;
        entity.piglin.offhand.as_ref().map(|stack| stack.id.clone())
    }

    #[test]
    fn a_dropped_gold_ingot_is_admired_for_119_ticks_then_bartered_toward_the_player() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        let players = calm_player(&mut world, DVec3::new(0.5, 64.0, 5.5));
        let ingot = ground.drop("minecraft:gold_ingot", 1, DVec3::new(3.5, 64.0, 0.5));
        let mut picked = None;
        let mut thrown = None;
        for tick in 0..400 {
            world.tick_with_players(&mut ground, &players);
            if picked.is_none() && offhand(&world, id).as_deref() == Some("minecraft:gold_ingot") {
                picked = Some(tick);
                assert!(!ground.has(ingot));
                let entity = world.piglins().iter().find(|e| e.id == id).unwrap();
                assert_eq!(entity.brain.memories.admiring_item.ttl(), Some(119));
                assert_eq!(entity.arm_pose, ArmPose::AdmiringItem);
            }
            let throws = world.take_barter_throws();
            if !throws.is_empty() {
                thrown = Some((tick, throws));
                break;
            }
        }
        let picked = picked.expect("it picks the ingot up");
        let (tick, throws) = thrown.expect("it barters");
        assert!((119..=121).contains(&(tick - picked)), "{}", tick - picked);
        assert_eq!(throws.len(), 1);
        let throw = throws[0];
        let entity = world.piglins().iter().find(|e| e.id == id).unwrap();
        assert!(entity.piglin.offhand.is_none());
        let eye = entity.position().y + 1.79 - 0.3;
        assert!((throw.from.y - eye).abs() < 1.0e-6, "{throw:?}");
        assert!((throw.velocity.length() - 0.3).abs() < 1.0e-6);
        let toward = players[0].position + DVec3::Y - entity.position();
        assert!(throw.velocity.dot(toward) > 0.0, "{throw:?}");
    }

    #[test]
    fn a_baby_keeps_the_gold_it_admires_and_never_barters() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let baby = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        world.piglin_mut(baby).unwrap().piglin.set_baby(true);
        let players = calm_player(&mut world, DVec3::new(0.5, 64.0, 5.5));
        ground.drop("minecraft:gold_ingot", 1, DVec3::new(2.5, 64.0, 0.5));
        let mut barters = 0;
        for _ in 0..300 {
            world.tick_with_players(&mut ground, &players);
            barters += world.take_barter_throws().len();
        }
        assert_eq!(barters, 0);
        let entity = world.piglins().iter().find(|e| e.id == baby).unwrap();
        assert!(entity.piglin.offhand.is_none());
        let held = entity.piglin.main_hand.as_ref().map(|s| s.id.as_str());
        assert_eq!(held, Some("minecraft:gold_ingot"));
        assert!(entity.piglin.persistence_required);
    }

    #[test]
    fn a_piglin_eats_a_porkchop_and_takes_the_next_after_200_ticks() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        slowed(&mut world, id);
        let players = calm_player(&mut world, DVec3::new(0.5, 64.0, 6.5));
        let at = world.piglin_mut(id).unwrap().position();
        let first = ground.drop("minecraft:porkchop", 1, at);
        world.tick_with_players(&mut ground, &players);
        assert!(!ground.has(first), "eaten at once");
        let memories = &world.piglin_mut(id).unwrap().brain.memories;
        assert!(memories.ate_recently.present());
        let at = world.piglin_mut(id).unwrap().position();
        let second = ground.drop("minecraft:porkchop", 1, at);
        let mut eaten = None;
        for tick in 1..260 {
            world.tick_with_players(&mut ground, &players);
            if !ground.has(second) {
                eaten = Some(tick);
                break;
            }
        }
        let eaten = eaten.expect("the second one goes too");
        assert!((195..=205).contains(&eaten), "{eaten}");
        let entity = world.piglins().iter().find(|e| e.id == id).unwrap();
        assert!(entity.piglin.inventory.iter().all(Option::is_none));
    }

    #[test]
    fn gold_nuggets_go_whole_into_the_inventory_and_drop_on_death() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        slowed(&mut world, id);
        let players = calm_player(&mut world, DVec3::new(0.5, 64.0, 6.5));
        let at = world.piglin_mut(id).unwrap().position();
        let nuggets = ground.drop("minecraft:gold_nugget", 20, at);
        world.tick_with_players(&mut ground, &players);
        assert!(!ground.has(nuggets));
        let entity = world.piglin_mut(id).unwrap();
        let held = entity.piglin.inventory[0].as_ref().unwrap();
        assert_eq!(
            (held.id.as_str(), held.count),
            ("minecraft:gold_nugget", 20)
        );
        assert!(world.player_attack(&attack(100.0), id).died);
        let deaths = world.take_deaths();
        assert!(
            deaths[0]
                .equipment
                .iter()
                .any(|s| s.id == "minecraft:gold_nugget" && s.count == 20)
        );
    }

    #[test]
    fn a_hit_while_admiring_ends_it_without_a_barter_and_disables_admiring_for_400_ticks() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        let players = calm_player(&mut world, DVec3::new(0.5, 64.0, 3.5));
        let entity = world.piglin_mut(id).unwrap();
        entity.piglin.offhand = Some(ItemStack::new("minecraft:gold_ingot", 1));
        entity.brain.admire_for(119);
        for _ in 0..10 {
            world.tick_with_players(&mut ground, &players);
        }
        assert!(world.player_attack(&attack(1.0), id).hurt);
        world.tick_with_players(&mut ground, &players);
        assert!(world.take_barter_throws().is_empty());
        let entity = world.piglins().iter().find(|e| e.id == id).unwrap();
        assert!(entity.piglin.offhand.is_none());
        assert!(entity.piglin.inventory.iter().all(Option::is_none));
        let memories = &entity.brain.memories;
        assert!(!memories.admiring_item.present());
        let disabled = memories.admiring_disabled.ttl().unwrap();
        assert!((398..=400).contains(&disabled), "{disabled}");
        assert!(ground.items.is_empty(), "vanilla loses the ingot");
    }

    #[test]
    fn babies_ignore_leather() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let baby = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        world.piglin_mut(baby).unwrap().piglin.set_baby(true);
        assert!(!world.piglin_wants(baby, &ItemStack::new("minecraft:leather", 1)));
        let adult = spawn(&mut world, PiglinKind::Piglin, 4.5, 0.5);
        assert!(world.piglin_wants(adult, &ItemStack::new("minecraft:leather", 1)));
        assert!(!world.piglin_wants(adult, &ItemStack::new("minecraft:soul_torch", 1)));
        let armed = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(-4.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        assert!(!world.piglin_wants(armed, &ItemStack::new("minecraft:leather", 1)));
        assert!(world.piglin_wants(armed, &ItemStack::new("minecraft:crossbow", 1)));
        assert!(world.piglin_wants(armed, &ItemStack::new("minecraft:golden_carrot", 1)));
        let at = world.piglin_mut(baby).unwrap().position();
        let leather = ground.drop("minecraft:leather", 1, at);
        for _ in 0..5 {
            world.tick_with_players(&mut ground, &[]);
        }
        assert!(ground.has(leather));
    }

    #[test]
    fn a_piglin_gives_up_on_an_unreachable_ingot_after_200_ticks() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        slowed(&mut world, id);
        let players = calm_player(&mut world, DVec3::new(0.5, 64.0, -6.5));
        let ingot = ground.drop("minecraft:gold_ingot", 1, DVec3::new(6.5, 64.0, 0.5));
        let mut gave_up = None;
        for tick in 0..500 {
            world.tick_with_players(&mut ground, &players);
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            if memories.disable_walk_to_admire_item.present() {
                gave_up = Some(tick);
                assert!(!memories.admiring_item.present());
                break;
            }
        }
        let gave_up = gave_up.expect("it gives up");
        assert!((200..=260).contains(&gave_up), "{gave_up}");
        assert!(ground.has(ingot));
        for _ in 0..150 {
            world.tick_with_players(&mut ground, &players);
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            assert!(!memories.admiring_item.present());
        }
    }

    #[test]
    fn no_mob_griefing_no_pickup() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_recipe_book(piglin_book());
        world.set_mob_griefing(false);
        let id = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        let players = calm_player(&mut world, DVec3::new(0.5, 64.0, 6.5));
        let ingot = ground.drop("minecraft:gold_ingot", 1, DVec3::new(1.0, 64.0, 0.5));
        for _ in 0..100 {
            world.tick_with_players(&mut ground, &players);
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            assert!(!memories.admiring_item.present());
        }
        assert!(ground.has(ingot));
        assert!(offhand(&world, id).is_none());
    }

    #[test]
    fn iron_golems_attack_piglins() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        let piglin = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        let brute = spawn(&mut world, PiglinKind::Brute, 0.5, 4.5);
        let golem = crate::iron_golem::IronGolem::new(DVec3::new(6.5, 64.0, 0.5));
        let golem = world.spawn_iron_golem(golem, false);
        world.iron_golem_mut(golem).unwrap().random = LegacyRandom::new(5);
        for _ in 0..400 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let hurt = |world: &EntityWorld, id| {
            world
                .piglins()
                .iter()
                .find(|e| e.id == id)
                .is_none_or(|e| e.piglin.health < e.piglin.max_health())
        };
        assert!(hurt(&world, piglin) || hurt(&world, brute));
        assert!(crate::monster_ai::is_enemy("minecraft:piglin"));
        assert!(crate::monster_ai::is_enemy("minecraft:piglin_brute"));
    }

    fn guarded_scene(world: &mut EntityWorld) -> (Blocks, [PlayerCandidate; 1], [u64; 3]) {
        let mut blocks = floor(24);
        for y in 64..67 {
            for z in -3..=3 {
                blocks
                    .blocks
                    .insert((-3, y, z), Block::new("minecraft:stone"));
            }
        }
        world.set_recipe_book(piglin_book());
        let players = calm_player(world, DVec3::new(0.5, 64.0, 0.5));
        let ids = [6.5, -8.5, 20.5].map(|x| {
            let id = armed(
                world,
                PiglinKind::Piglin,
                DVec3::new(x, 64.0, 0.5),
                "minecraft:golden_sword",
            );
            slowed(world, id);
            id
        });
        (blocks, players, ids)
    }

    fn angry_at(world: &mut EntityWorld, id: u64) -> Option<(u128, i64)> {
        let memories = &world.piglin_mut(id).unwrap().brain.memories;
        Some((*memories.angry_at.get()?, memories.angry_at.ttl()?))
    }

    #[test]
    fn mining_a_gold_block_angers_idle_piglins_within_16_even_unseen() {
        let mut world = EntityWorld::default();
        let (mut blocks, players, [seen, unseen, far]) = guarded_scene(&mut world);
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &players);
        }
        for id in [seen, unseen, far] {
            let entity = world.piglin_mut(id).unwrap();
            assert_eq!(entity.brain.activity, Activity::Idle, "{id}");
            assert!(!entity.brain.memories.angry_at.present(), "{id}");
        }
        let memories = &world.piglin_mut(unseen).unwrap().brain.memories;
        assert!(
            !memories.sees(Target::Player(7)),
            "the wall hides the player"
        );
        world.anger_nearby_piglins(7, players[0].position, false);
        world.tick_with_players(&mut blocks, &players);
        let uuid = world.uuid_of(PLAYER_TARGET + 7);
        for id in [seen, unseen] {
            let (angry, ttl) = angry_at(&mut world, id).expect("angry");
            assert_eq!(angry, uuid);
            assert!((598..=600).contains(&ttl), "{ttl}");
        }
        assert!(
            angry_at(&mut world, far).is_none(),
            "20 blocks away stays calm"
        );
        for _ in 0..20 {
            world.tick_with_players(&mut blocks, &players);
        }
        let brain = &world.piglin_mut(seen).unwrap().brain;
        assert_eq!(brain.memories.attack_target.get(), Some(&Target::Player(7)));
    }

    #[test]
    fn opening_a_chest_angers_only_the_piglins_that_see_it() {
        let mut world = EntityWorld::default();
        let (mut blocks, players, [seen, unseen, far]) = guarded_scene(&mut world);
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &players);
        }
        world.anger_nearby_piglins(7, players[0].position, true);
        world.tick_with_players(&mut blocks, &players);
        let uuid = world.uuid_of(PLAYER_TARGET + 7);
        assert_eq!(angry_at(&mut world, seen).map(|(at, _)| at), Some(uuid));
        assert!(angry_at(&mut world, unseen).is_none());
        assert!(angry_at(&mut world, far).is_none());
    }

    #[test]
    fn brutes_and_busy_piglins_are_not_angered_by_a_chest() {
        let mut world = EntityWorld::default();
        let (mut blocks, players, [idle, _, _]) = guarded_scene(&mut world);
        let busy = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(4.5, 64.0, 4.5),
            "minecraft:golden_sword",
        );
        slowed(&mut world, busy);
        let brute = armed(
            &mut world,
            PiglinKind::Brute,
            DVec3::new(4.5, 64.0, -4.5),
            "minecraft:golden_axe",
        );
        slowed(&mut world, brute);
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &players);
        }
        let entity = world.piglin_mut(busy).unwrap();
        entity.piglin.offhand = Some(ItemStack::new(piglin::BARTERING_ITEM, 1));
        entity.brain.admire_for(crate::piglin_brain::ADMIRE_TICKS);
        world.tick_with_players(&mut blocks, &players);
        let brain = &world.piglin_mut(busy).unwrap().brain;
        assert_eq!(brain.activity, Activity::AdmireItem);
        for id in [busy, brute, idle] {
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            assert!(memories.sees(Target::Player(7)), "{id}");
        }
        world.anger_nearby_piglins(7, players[0].position, true);
        world.tick_with_players(&mut blocks, &players);
        assert!(angry_at(&mut world, idle).is_some());
        assert!(angry_at(&mut world, busy).is_none(), "admiring");
        assert!(
            angry_at(&mut world, brute).is_none(),
            "brutes are not piglins"
        );
    }

    #[test]
    fn universal_anger_turns_a_chest_s_piglins_on_the_nearest_player() {
        let mut world = EntityWorld::default();
        let (mut blocks, players, [seen, _, _]) = guarded_scene(&mut world);
        world.set_universal_anger(true);
        let near = player(8, DVec3::new(8.5, 64.0, 0.5));
        world.set_player_controls(&[
            wearing(7, [Some("minecraft:golden_boots"), None, None, None]),
            wearing(8, [Some("minecraft:golden_boots"), None, None, None]),
        ]);
        let players = [players[0], near];
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &players);
        }
        world.anger_nearby_piglins(7, players[0].position, true);
        world.tick_with_players(&mut blocks, &players);
        let uuid = world.uuid_of(PLAYER_TARGET + 8);
        assert_eq!(angry_at(&mut world, seen).map(|(at, _)| at), Some(uuid));
        let memories = &world.piglin_mut(seen).unwrap().brain.memories;
        assert!(memories.universal_anger.present());
    }

    fn heard(world: &mut EntityWorld, event: &str) -> bool {
        world.take_sounds().iter().any(|sound| sound.event == event)
    }

    fn zombified(world: &EntityWorld) -> Vec<&crate::world::ZombieEntity> {
        world
            .zombies()
            .iter()
            .filter(|e| e.zombie.kind == ZombieKind::ZombifiedPiglin)
            .collect()
    }

    #[test]
    fn a_piglin_converts_on_the_301st_overworld_tick_with_nausea_and_its_gear() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        world.set_piglins_zombify(true);
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = armed(&mut world, PiglinKind::Piglin, at, "minecraft:golden_sword");
        let entity = world.piglin_mut(id).unwrap();
        entity.piglin.armor[0] = Some(ItemStack::new("minecraft:golden_helmet", 1));
        entity.piglin.drop_chances[piglin::MAINHAND] = piglin::GUARANTEED_DROP;
        entity.piglin.persistence_required = true;
        entity.piglin.can_pick_up_loot = false;
        for _ in 0..300 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let piglin = &world.piglin_mut(id).unwrap().piglin;
        assert!(piglin.converting);
        assert_eq!(piglin.time_in_overworld, 300);
        assert!(zombified(&world).is_empty());
        let before = world.piglin_mut(id).unwrap().position();
        world.take_sounds();
        world.tick_with_players(&mut blocks, &[]);
        assert!(world.piglins().is_empty());
        let zombies = zombified(&world);
        assert_eq!(zombies.len(), 1);
        let zombie = zombies[0];
        let new_id = zombie.id;
        assert_ne!(new_id, id);
        assert!(zombie.ai.is_some(), "it thinks like a zombified piglin");
        assert_eq!(
            zombie.zombie.main_hand.as_deref(),
            Some("minecraft:golden_sword")
        );
        assert_eq!(
            zombie.zombie.armor[0].as_deref(),
            Some("minecraft:golden_helmet")
        );
        assert!(!zombie.zombie.baby && zombie.zombie.persistence_required);
        assert!(zombie.zombie.body.position.distance(before) < 0.5);
        let nausea = zombie.effects.get(MobEffect::Nausea).unwrap();
        assert_eq!((nausea.duration, nausea.amplifier), (200, 0));
        assert!(heard(&mut world, "entity.piglin.converted_to_zombified"));
        let conversions = world.take_piglin_conversions();
        assert_eq!(conversions.len(), 1);
        let conversion = &conversions[0];
        assert_eq!(conversion.zombified, new_id);
        assert_eq!(
            conversion.equipment[piglin::MAINHAND]
                .as_ref()
                .map(|s| s.id.as_str()),
            Some("minecraft:golden_sword")
        );
        assert_eq!(
            conversion.drop_chances[piglin::MAINHAND],
            piglin::GUARANTEED_DROP
        );
        assert!(!conversion.can_pick_up_loot);
    }

    #[test]
    fn an_immune_or_no_ai_piglin_never_converts() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        world.set_piglins_zombify(true);
        let mut immune = Piglin::new(PiglinKind::Piglin, DVec3::new(0.5, 64.0, 0.5));
        immune.immune_to_zombification = true;
        let immune = world.spawn_piglin(immune, false);
        let still = Piglin::new(PiglinKind::Piglin, DVec3::new(4.5, 64.0, 0.5));
        let still = world.spawn_piglin(still, true);
        for _ in 0..400 {
            world.tick_with_players(&mut blocks, &[]);
        }
        for id in [immune, still] {
            let piglin = &world.piglin_mut(id).unwrap().piglin;
            assert!(!piglin.converting && piglin.time_in_overworld == 0, "{id}");
        }
        assert!(zombified(&world).is_empty());
    }

    #[test]
    fn the_nether_resets_the_count() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        let id = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        world.set_piglins_zombify(true);
        for _ in 0..250 {
            world.tick_with_players(&mut blocks, &[]);
        }
        assert_eq!(world.piglin_mut(id).unwrap().piglin.time_in_overworld, 250);
        world.set_piglins_zombify(false);
        world.tick_with_players(&mut blocks, &[]);
        let piglin = &world.piglin_mut(id).unwrap().piglin;
        assert!(!piglin.converting && piglin.time_in_overworld == 0);
        world.set_piglins_zombify(true);
        for _ in 0..300 {
            world.tick_with_players(&mut blocks, &[]);
        }
        assert_eq!(world.piglins().len(), 1, "the clock started over");
    }

    #[test]
    fn a_brute_converts_too() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        world.set_piglins_zombify(true);
        let at = DVec3::new(0.5, 64.0, 0.5);
        armed(&mut world, PiglinKind::Brute, at, "minecraft:golden_axe");
        let mut sounded = false;
        for _ in 0..301 {
            world.tick_with_players(&mut blocks, &[]);
            sounded |= heard(&mut world, "entity.piglin_brute.converted_to_zombified");
        }
        assert!(sounded);
        assert!(world.piglins().is_empty());
        let zombies = zombified(&world);
        assert_eq!(zombies.len(), 1);
        assert_eq!(
            zombies[0].zombie.main_hand.as_deref(),
            Some("minecraft:golden_axe")
        );
    }

    #[test]
    fn the_inventory_and_admired_item_drop_on_conversion() {
        let mut ground = Ground::new(12);
        let mut world = EntityWorld::default();
        world.set_piglins_zombify(true);
        let admiring = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        let holding = spawn(&mut world, PiglinKind::Piglin, 6.5, 0.5);
        for (id, offhand) in [
            (admiring, piglin::BARTERING_ITEM),
            (holding, "minecraft:shield"),
        ] {
            let entity = world.piglin_mut(id).unwrap();
            entity.piglin.offhand = Some(ItemStack::new(offhand, 1));
            entity.piglin.inventory[0] = Some(ItemStack::new(piglin::GOLD_NUGGET, 9));
            entity.piglin.inventory[3] = Some(ItemStack::new("minecraft:leather", 2));
            entity.piglin.time_in_overworld = 300;
        }
        world
            .piglin_mut(admiring)
            .unwrap()
            .brain
            .admire_for(crate::piglin_brain::ADMIRE_TICKS);
        world.tick_with_players(&mut ground, &[]);
        assert!(world.piglins().is_empty());
        let count = |item: &str| {
            ground
                .items
                .iter()
                .filter(|i| i.item == item)
                .map(|i| i.count)
                .sum::<i32>()
        };
        assert_eq!(count(piglin::GOLD_NUGGET), 18);
        assert_eq!(count("minecraft:leather"), 4);
        assert_eq!(count(piglin::BARTERING_ITEM), 1, "the admired ingot drops");
        assert_eq!(count("minecraft:shield"), 0, "a held shield is kept");
        let conversions = world.take_piglin_conversions();
        let kept: Vec<Option<&str>> = conversions
            .iter()
            .map(|c| c.equipment[piglin::OFFHAND].as_ref().map(|s| s.id.as_str()))
            .collect();
        assert!(kept.contains(&Some("minecraft:shield")), "{kept:?}");
        assert!(kept.contains(&None), "{kept:?}");
    }

    #[test]
    fn no_converted_sound_in_peaceful() {
        let mut blocks = floor(12);
        let mut world = EntityWorld::default();
        world.set_difficulty(0);
        world.set_piglins_zombify(true);
        let id = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
        world.piglin_mut(id).unwrap().piglin.time_in_overworld = 300;
        world.tick_with_players(&mut blocks, &[]);
        assert_eq!(zombified(&world).len(), 1);
        assert!(!heard(&mut world, "entity.piglin.converted_to_zombified"));
    }

    fn hoglin_at(world: &mut EntityWorld, x: f64, z: f64) -> u64 {
        let kind = crate::hoglin::HoglinKind::Hoglin;
        let hoglin = crate::hoglin::Hoglin::new(kind, DVec3::new(x, 64.0, z));
        let id = world.spawn_hoglin(hoglin, false);
        world.hoglin_mut(id).unwrap().random = LegacyRandom::new(id * 17);
        id
    }

    fn hoglin_dead(world: &EntityWorld, id: u64) -> bool {
        world
            .hoglins()
            .iter()
            .find(|e| e.id == id)
            .is_none_or(|e| e.hoglin.health <= 0.0)
    }

    #[test]
    fn adult_piglins_hunt_a_visible_hoglin_and_rest_30_to_120_seconds_after() {
        let mut blocks = floor(20);
        let mut world = EntityWorld::default();
        let piglins = [-2.5, 0.5, 3.5].map(|x| {
            armed(
                &mut world,
                PiglinKind::Piglin,
                DVec3::new(x, 64.0, 0.5),
                "minecraft:golden_sword",
            )
        });
        let hoglin = hoglin_at(&mut world, 0.5, 6.5);
        world.hoglin_mut(hoglin).unwrap().hoglin.health = 12.0;
        let uuid = world.uuid_of(hoglin);
        let mut hunted = false;
        for _ in 0..1200 {
            world.tick_with_players(&mut blocks, &[]);
            let angry = piglins.iter().any(|&id| {
                world.piglin_mut(id).unwrap().brain.memories.angry_at.get() == Some(&uuid)
            });
            if angry && !hunted {
                hunted = true;
                for id in piglins {
                    let memories = &world.piglin_mut(id).unwrap().brain.memories;
                    let ttl = memories.hunted_recently.ttl().expect("resting");
                    assert!((598..=2400).contains(&ttl), "{ttl}");
                }
            }
            if hoglin_dead(&world, hoglin) {
                break;
            }
        }
        assert!(hunted, "they went for the hoglin");
        assert!(hoglin_dead(&world, hoglin), "and killed it");
        let second = hoglin_at(&mut world, 0.5, -5.5);
        let second_uuid = world.uuid_of(second);
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &[]);
        }
        for id in piglins {
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            assert!(memories.hunted_recently.present(), "{id}");
            assert_ne!(memories.angry_at.get(), Some(&second_uuid), "{id}");
            assert_ne!(memories.attack_target.get(), Some(&Target::Mob(second)));
        }
    }

    #[test]
    fn outnumbered_piglins_retreat_from_hoglins() {
        let mut blocks = floor(20);
        let mut world = EntityWorld::default();
        let piglins = [0.5, 2.5].map(|x| spawn(&mut world, PiglinKind::Piglin, x, 0.5));
        for id in piglins {
            let memories = &mut world.piglin_mut(id).unwrap().brain.memories;
            memories.hunted_recently.set_for((), 5000);
        }
        let hoglins = [-1.5, 1.5, 4.5].map(|x| hoglin_at(&mut world, x, 6.5));
        for id in hoglins {
            slowed_hoglin(&mut world, id);
        }
        for _ in 0..45 {
            world.tick_with_players(&mut blocks, &[]);
        }
        let brain = &world.piglin_mut(piglins[0]).unwrap().brain;
        assert_eq!(brain.memories.visible_adult_hoglin_count.get(), Some(&3));
        assert!(brain.hoglins_outnumber_piglins());
        world.note_piglin_hurt(piglins[0], Target::Mob(hoglins[0]));
        world.tick_with_players(&mut blocks, &[]);
        for id in piglins {
            let brain = &world.piglin_mut(id).unwrap().brain;
            let memories = &brain.memories;
            assert_eq!(
                memories.avoid_target.get(),
                Some(&Target::Mob(hoglins[0])),
                "{id}"
            );
            let ttl = memories.avoid_target.ttl().unwrap();
            assert!((98..=400).contains(&ttl), "{id} {ttl}");
            let rest = memories.hunted_recently.ttl().unwrap();
            assert!(rest <= 2400, "{id} {rest}");
            assert!(!memories.angry_at.present() && !memories.attack_target.present());
            assert_eq!(brain.activity, Activity::Avoid, "{id}");
        }
    }

    fn slowed_hoglin(world: &mut EntityWorld, id: u64) {
        let slowness = crate::effects::EffectInstance::new(MobEffect::Slowness, 10_000, 9);
        world.hoglin_mut(id).unwrap().effects.add(slowness);
    }

    #[test]
    fn a_piglin_with_cannot_hunt_leaves_hoglins_alone() {
        let mut blocks = floor(20);
        let mut world = EntityWorld::default();
        let lazy = armed(
            &mut world,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        world.piglin_mut(lazy).unwrap().piglin.cannot_hunt = true;
        let hoglin = hoglin_at(&mut world, 0.5, 5.5);
        slowed_hoglin(&mut world, hoglin);
        let mut other = EntityWorld::default();
        let hunter = armed(
            &mut other,
            PiglinKind::Piglin,
            DVec3::new(0.5, 64.0, 0.5),
            "minecraft:golden_sword",
        );
        let spared = hoglin_at(&mut other, 0.5, 5.5);
        slowed_hoglin(&mut other, spared);
        other.hoglin_mut(spared).unwrap().hoglin.cannot_be_hunted = true;
        let mut other_blocks = floor(20);
        for _ in 0..200 {
            world.tick_with_players(&mut blocks, &[]);
            other.tick_with_players(&mut other_blocks, &[]);
        }
        for (world, id) in [(&mut world, lazy), (&mut other, hunter)] {
            let memories = &world.piglin_mut(id).unwrap().brain.memories;
            assert!(!memories.angry_at.present(), "{id}");
            assert!(!memories.attack_target.present(), "{id}");
            assert!(!memories.hunted_recently.present(), "{id}");
        }
        let memories = &other.piglin_mut(hunter).unwrap().brain.memories;
        assert!(!memories.nearest_visible_huntable_hoglin.present());
        assert_eq!(memories.visible_adult_hoglin_count.get(), Some(&1));
    }

    #[test]
    fn about_one_kill_in_ten_ends_in_a_dance() {
        let living = crate::brain_behaviors::Living {
            target: Target::Mob(1),
            kind: HOGLIN,
            position: DVec3::ZERO,
            eye_height: 0.8,
            width: 1.4,
            height: 1.4,
            alive: false,
            baby: false,
            attackable: true,
            spectator: false,
        };
        let wants = |time: i64| crate::piglin_brain::wants_to_dance(time, &living);
        let kills = (0..3000).map(|kill| kill * 1009 + 1);
        let dances = kills.clone().filter(|&time| wants(time)).count();
        assert!((200..=400).contains(&dances), "{dances}");
        let dance_time = kills.clone().find(|&time| wants(time)).unwrap();
        let calm_time = kills.clone().find(|&time| !wants(time)).unwrap();
        for (time, dances) in [(dance_time, true), (calm_time, false)] {
            let mut blocks = floor(12);
            let mut world = EntityWorld::default();
            let piglin = spawn(&mut world, PiglinKind::Piglin, 0.5, 0.5);
            let hoglin = hoglin_at(&mut world, 0.5, 2.5);
            world.hoglin_mut(hoglin).unwrap().hurt(100.0);
            let memories = &mut world.piglin_mut(piglin).unwrap().brain.memories;
            memories.attack_target.set(Target::Mob(hoglin));
            world.set_game_time(time - 1);
            world.tick_with_players(&mut blocks, &[]);
            let entity = world.piglin_mut(piglin).unwrap();
            assert!(entity.brain.memories.celebrate_location.present(), "{time}");
            assert_eq!(entity.brain.memories.dancing.present(), dances, "{time}");
            assert_eq!(entity.piglin.dancing, dances, "{time}");
            assert_eq!(entity.arm_pose == ArmPose::Dancing, dances, "{time}");
        }
    }
}
