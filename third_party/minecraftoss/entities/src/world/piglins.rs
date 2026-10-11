use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::brain_behaviors::{Living, Pos, find};
use crate::monster_ai::Target;
use crate::piglin::{self, ArmPose, Piglin, PiglinKind};
use crate::piglin_brain::{Activity, ItemUse, PiglinBrain, PiglinCtx};

pub const LOVED_TAG: &str = "minecraft:piglin_loved";
pub const SAFE_ARMOR_TAG: &str = "minecraft:piglin_safe_armor";

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
    rules: PiglinRules,
}

#[derive(Default)]
pub(super) struct PiglinThought {
    doors: Vec<(Pos, bool)>,
    attack: Option<Target>,
    shot: Option<Target>,
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
            universal_anger: sees.rules.universal_anger,
            forgive_dead_players: sees.rules.forgive_dead_players,
            holds_crossbow: self.piglin.holds_crossbow(),
            using_item: self.using_item,
            look_at: None,
            sounds: Vec::new(),
            doors: Vec::new(),
            attack: None,
            shot: None,
            strafe: None,
            item_use: None,
            charging: None,
        };
        self.brain.tick(&mut ctx);
        self.brain.update_activity(&mut ctx);
        let thought = PiglinThought {
            doors: std::mem::take(&mut ctx.doors),
            attack: ctx.attack,
            shot: ctx.shot,
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
        for adult in entity.brain.memories.nearby_adult_piglins() {
            if let Target::Mob(adult) = adult {
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

    fn piglin_was_hurt_by(
        &mut self,
        id: u64,
        attacker: Target,
        others: &[Living],
        uuids: &[(Target, u128)],
    ) {
        let Some(other) = find(others, attacker).copied() else {
            return;
        };
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
        self.piglin_maybe_retaliate(id, attacker, others, uuids);
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
            self.piglin_was_hurt_by(id, attacker, &others, &uuids);
        }
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
        let sees = PiglinSees {
            others: &others,
            uuids: &uuids,
            gilded: &gilded,
            loved_holders: &loved_holders,
            rules: self.piglin_rules,
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
        if let Some(target) = thought.attack {
            self.piglin_hits(id, target);
        }
        if let Some(target) = thought.shot {
            self.piglin_shoots(id, target, &others);
        }
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
                serde_json::json!({ "values": ["minecraft:gold_ingot", "minecraft:golden_carrot"] }),
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
}
