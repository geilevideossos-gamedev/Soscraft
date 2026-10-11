use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::brain_behaviors::{Living, Pos};
use crate::monster_ai::Target;
use crate::piglin::{self, ArmPose, Piglin, PiglinKind};
use crate::piglin_brain::{PiglinBrain, PiglinCtx};

pub const LOVED_TAG: &str = "minecraft:piglin_loved";

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

    fn think(
        &mut self,
        world: &dyn World,
        others: &[Living],
        game_time: i64,
        dimension: &str,
        level_random: &mut LegacyRandom,
    ) -> Vec<(Pos, bool)> {
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
            look_at: None,
            sounds: Vec::new(),
            doors: Vec::new(),
        };
        self.brain.tick(&mut ctx);
        self.brain.update_activity(&mut ctx);
        let (look_at, sounds, doors) = (ctx.look_at, ctx.sounds, ctx.doors);
        if let Some(at) = look_at {
            self.look_control.set_look_at(at);
        }
        for sound in sounds {
            self.make_sound(sound);
        }
        self.piglin.aggressive = self.brain.aggressive();
        self.piglin.dancing = self.brain.dancing();
        doors
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
        others: &[Living],
        game_time: i64,
        dimension: &str,
        level_random: &mut LegacyRandom,
    ) -> (bool, Vec<(Pos, bool)>) {
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
            return (false, Vec::new());
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
            return (true, Vec::new());
        }
        let position = self.piglin.body.position;
        self.piglin.body.trim_small_velocity();
        monster_idle_without_ai(world, players, position, eye, &mut self.no_action_time);
        let mut doors = Vec::new();
        if !self.no_ai {
            self.no_action_time += 1;
            doors = self.think(world, others, game_time, dimension, level_random);
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
        (true, doors)
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

    pub(super) fn tick_piglin(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let game_time = self.game_time;
        let others = self.hoglin_living(id, players);
        let dimension = self.dimension.clone();
        let mut level_random = std::mem::take(&mut self.level_random);
        let ticked = self.piglins.iter_mut().find(|e| e.id == id).map(|entity| {
            entity.tick_body(
                &*world,
                players,
                &others,
                game_time,
                &dimension,
                &mut level_random,
            )
        });
        let Some((stepped, doors)) = ticked else {
            self.level_random = level_random;
            return;
        };
        for (pos, open) in doors {
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
}
