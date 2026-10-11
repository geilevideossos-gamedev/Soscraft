use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::hoglin::{self, Hoglin, HoglinKind};

#[derive(Clone)]
pub struct HoglinEntity {
    pub id: u64,
    pub effects: crate::effects::MobEffects,
    pub hoglin: Hoglin,
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
}

pub(super) fn hoglin_sounds(kind: HoglinKind) -> MovementSounds {
    match kind {
        HoglinKind::Hoglin => MovementSounds::creature(Some("entity.hoglin.step")),
        HoglinKind::Zoglin => MovementSounds::monster(Some("entity.zoglin.step")),
    }
    .with_step_volume(hoglin::STEP_VOLUME)
}

impl HoglinEntity {
    pub fn position(&self) -> DVec3 {
        self.hoglin.body.position
    }

    pub fn set_random_seed(&mut self, seed: u64) {
        self.random = LegacyRandom::new(seed);
    }

    pub fn hurt(&mut self, amount: f32) -> DamageResult {
        if self.hoglin.health > 0.0 && !self.hoglin.damage.dead {
            self.no_action_time = 0;
        }
        let result = self.hoglin.hurt_generic(amount);
        let position = self.position();
        let on_fire = self.hoglin.body.fire_ticks > 0 && !self.hoglin.fire_immune();
        self.hoglin.damage.place_death(result, position, on_fire);
        if result.applied && result.full {
            if !result.died {
                self.ambient_sound_time = -hoglin::AMBIENT_INTERVAL;
            }
            let voice = hurt_voice(&mut self.random, result.died, self.hoglin.is_baby());
            self.voices.push((voice, position));
        }
        result
    }

    fn hostile(&self) -> bool {
        self.hoglin.kind == HoglinKind::Zoglin
    }

    fn travel(&mut self, world: &impl World) -> Option<f64> {
        let body = &mut self.hoglin.body;
        let yaw = self.hoglin.yaw;
        let fluid = self.fluid;
        if fluid.in_water() {
            body.travel_water(world, DVec3::ZERO, yaw);
            None
        } else if fluid.in_lava() {
            body.travel_lava(world, DVec3::ZERO, yaw, fluid.lava_height);
            None
        } else {
            body.travel_air_jumping(world, DVec3::ZERO, 0.0, yaw, self.jumping)
        }
    }

    pub(super) fn tick_body(
        &mut self,
        world: &impl World,
        players: &[PlayerCandidate],
        game_time: i64,
    ) -> bool {
        self.previous_position = self.hoglin.body.position;
        self.tick_count += 1;
        let sounds = hoglin_sounds(self.hoglin.kind);
        self.fluid = base_tick_fluid(
            &mut self.hoglin.body,
            world,
            self.tick_count == 1,
            &mut self.random,
            &mut self.voices,
            sounds,
        );
        hazards::burn(self, world, game_time);
        let eye = self.hoglin.eye_height();
        hazards::suffocate(self, world, eye, game_time);
        let water_breathing = self.effects.has(MobEffect::WaterBreathing) || self.hostile();
        if self.hoglin.health > 0.0 && breathe(&mut self.hoglin.body, world, eye, water_breathing) {
            self.hurt(2.0);
        }
        let removed = self.hoglin.damage.tick();
        let undead = self.hostile();
        for work in self.effects.tick(undead) {
            match work.resolve(self.hoglin.health, hoglin::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut self.hoglin.health, hoglin::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    self.hurt(amount);
                }
                _ => {}
            }
        }
        if self.hoglin.health > 0.0 {
            if (self.random.next_int(1000) as i32) < self.ambient_sound_time {
                self.ambient_sound_time = -hoglin::AMBIENT_INTERVAL;
                let voice = Voice::Ambient(voice_pitch(&mut self.random, self.hoglin.is_baby()));
                self.voices.push((voice, self.position()));
            } else {
                self.ambient_sound_time += 1;
            }
        }
        if removed {
            return false;
        }
        let old = self.previous_position;
        if self.hoglin.health <= 0.0 {
            if !self.no_ai {
                let yaw = self.hoglin.yaw;
                let hostile = self.hostile();
                dying_travel(
                    &mut self.hoglin.body,
                    world,
                    yaw,
                    self.tick_count,
                    &mut self.random,
                    &mut self.voices,
                    sounds,
                    Some(hostile),
                );
            }
            hazards::blocks_act(self, world, old, game_time);
            return true;
        }
        self.hoglin.tick_attack_animation();
        let position = self.hoglin.body.position;
        if self.no_ai {
            self.hoglin.body.trim_small_velocity();
            if self.hostile() {
                monster_idle_without_ai(world, players, position, eye, &mut self.no_action_time);
            } else if players.iter().any(|p| {
                p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0
            }) {
                self.no_action_time = 0;
            }
        } else {
            self.hoglin.body.trim_small_velocity();
            if self.hostile() {
                monster_idle_without_ai(world, players, position, eye, &mut self.no_action_time);
            } else if players.iter().any(|p| {
                p.alive && !p.spectator && p.position.distance_squared(position) < 32.0 * 32.0
            }) {
                self.no_action_time = 0;
            }
            self.no_action_time += 1;
            if self.no_jump_delay > 0 {
                self.no_jump_delay -= 1;
            }
            let jumping = self.jumping;
            self.hoglin
                .body
                .living_jump(world, self.fluid, jumping, &mut self.no_jump_delay, 0.4);
            self.look_control.tick(
                self.hoglin.body.position,
                eye,
                self.body_rotation.body_yaw,
                false,
            );
            let landed = self.travel(world);
            play_movement(
                &mut self.hoglin.body,
                self.tick_count,
                &mut self.random,
                &mut self.voices,
                sounds,
            );
            self.body_rotation.tick(
                self.hoglin.yaw,
                &mut self.look_control,
                self.previous_position,
                self.hoglin.body.position,
            );
            let hostile = self.hostile();
            if let Some(damage) = landed.and_then(|fallen| {
                fall_damage(&self.hoglin.body, world, fallen, hostile, &mut self.voices)
            }) {
                self.hurt(damage);
            }
        }
        hazards::blocks_act(self, world, old, game_time);
        true
    }

    pub(super) fn tick_age(&mut self) {
        if self.hoglin.kind != HoglinKind::Hoglin {
            return;
        }
        let was_baby = self.hoglin.age.baby();
        self.hoglin.age.tick(true);
        if was_baby != self.hoglin.age.baby() {
            self.hoglin.sync_dimensions();
        }
        if self.hoglin.age.ticks != 0 {
            self.hoglin.in_love = 0;
        } else if self.hoglin.in_love > 0 {
            self.hoglin.in_love -= 1;
            if self.hoglin.in_love % 10 == 0 {
                for _ in 0..3 {
                    let _ = self.random.next_gaussian();
                }
                for _ in 0..3 {
                    let _ = self.random.next_double();
                }
            }
        }
    }
}

impl EntityWorld {
    pub fn spawn_hoglin(&mut self, mut hoglin: Hoglin, no_ai: bool) -> u64 {
        self.next_id += 1;
        hoglin.sync_dimensions();
        let id = self.next_id;
        let previous_position = hoglin.body.position;
        let yaw = hoglin.yaw;
        self.hoglins.push(HoglinEntity {
            id,
            effects: Default::default(),
            hoglin,
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
        });
        self.order.push(EntityKey::Hoglin(id));
        self.file_in_section(EntityKey::Hoglin(id));
        id
    }

    pub fn hoglins(&self) -> &[HoglinEntity] {
        &self.hoglins
    }

    pub fn hoglin_mut(&mut self, id: u64) -> Option<&mut HoglinEntity> {
        self.hoglins.iter_mut().find(|entity| entity.id == id)
    }

    pub(super) fn tick_hoglin(
        &mut self,
        id: u64,
        world: &impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let game_time = self.game_time;
        let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if !entity.tick_body(world, players, game_time) {
            return;
        }
        self.push_entities(EntityKey::Hoglin(id), world, game_time, ticks);
        let Some(entity) = self.hoglins.iter_mut().find(|e| e.id == id) else {
            return;
        };
        if entity.hoglin.health > 0.0 {
            entity.tick_age();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hoglin::HoglinKind;
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

    fn floor() -> Blocks {
        let mut world = Blocks::default();
        for x in -8..=8 {
            for z in -8..=8 {
                world
                    .blocks
                    .insert((x, 63, z), Block::new("minecraft:netherrack"));
            }
        }
        world
    }

    fn spawn(world: &mut EntityWorld, kind: HoglinKind, baby: bool, x: f64) -> u64 {
        let mut hoglin = Hoglin::new(kind, DVec3::new(x, 64.0, 0.5));
        hoglin.set_baby(baby);
        world.spawn_hoglin(hoglin, false)
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
    fn a_hoglin_and_a_zoglin_have_forty_health_and_vanilla_sizes() {
        let mut world = EntityWorld::default();
        let hoglin = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
        let zoglin = spawn(&mut world, HoglinKind::Zoglin, false, 4.5);
        let piglet = spawn(&mut world, HoglinKind::Hoglin, true, -4.5);
        for id in [hoglin, zoglin] {
            let e = &world.hoglin_mut(id).unwrap().hoglin;
            assert_eq!(e.health, 40.0);
            assert_eq!((e.body.width, e.body.height), (1.3964844, 1.4));
            assert!((e.eye_height() - 1.19).abs() < 1.0e-6);
        }
        let e = &world.hoglin_mut(piglet).unwrap().hoglin;
        assert_eq!((e.body.width, e.body.height), (0.75, 0.85));
        assert_eq!(e.eye_height(), 0.625);
        assert_eq!(world.entity_type(hoglin), Some("minecraft:hoglin"));
        assert_eq!(world.entity_type(zoglin), Some("minecraft:zoglin"));
        let candidates = world.mob_candidates();
        let baby = candidates.iter().find(|c| c.id == piglet).unwrap();
        assert!(baby.baby && baby.kind == "minecraft:hoglin");
        assert!(!candidates.iter().find(|c| c.id == hoglin).unwrap().baby);
    }

    #[test]
    fn a_baby_hoglin_drops_experience_but_no_loot_and_a_baby_zoglin_drops_both() {
        let mut world = EntityWorld::default();
        let piglet = spawn(&mut world, HoglinKind::Hoglin, true, 0.5);
        assert!(world.player_attack(&attack(100.0), piglet).died);
        let deaths = world.take_deaths();
        assert_eq!(deaths.len(), 1);
        assert_eq!((deaths[0].table, deaths[0].experience), (None, 3));
        let zoglin = spawn(&mut world, HoglinKind::Zoglin, true, 0.5);
        assert!(world.player_attack(&attack(100.0), zoglin).died);
        let deaths = world.take_deaths();
        assert_eq!(
            (deaths[0].table, deaths[0].experience),
            (Some("minecraft:zoglin"), 5)
        );
        let adult = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
        assert!(world.player_attack(&attack(100.0), adult).died);
        let deaths = world.take_deaths();
        assert_eq!(
            (deaths[0].table, deaths[0].experience),
            (Some("minecraft:hoglin"), 5)
        );
    }

    #[test]
    fn a_burning_hoglin_drops_cooked_porkchops() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let mut book = crate::loot::EntityLootBook::from_jar(&jar, 0).expect("the loot tables");
        let mut world = EntityWorld::default();
        let (mut raw, mut cooked, mut leather) = (0, 0, 0);
        for burning in [false, true] {
            for _ in 0..40 {
                let id = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
                if burning {
                    world.hoglin_mut(id).unwrap().hoglin.body.fire_ticks = 100;
                }
                assert!(world.player_attack(&attack(100.0), id).died);
                let deaths = world.take_deaths();
                let drops = book.death_drops(deaths);
                let count = |item: &str| {
                    drops
                        .iter()
                        .filter(|(s, _)| s.id == item)
                        .map(|(s, _)| u32::from(s.count))
                        .sum::<u32>()
                };
                let meat = count("minecraft:porkchop") + count("minecraft:cooked_porkchop");
                assert!((2..=4).contains(&meat), "{drops:?}");
                assert!(count("minecraft:leather") <= 1);
                if burning {
                    assert_eq!(count("minecraft:porkchop"), 0);
                    cooked += count("minecraft:cooked_porkchop");
                } else {
                    assert_eq!(count("minecraft:cooked_porkchop"), 0);
                    raw += count("minecraft:porkchop");
                }
                leather += count("minecraft:leather");
                world.remove_mobs(&[id]);
            }
        }
        assert!(raw > 0 && cooked > 0 && leather > 0);
    }

    #[test]
    fn hoglins_and_zoglins_leave_on_peaceful() {
        let mut world = EntityWorld::default();
        let hoglin = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
        let zoglin = spawn(&mut world, HoglinKind::Zoglin, false, 4.5);
        world
            .hoglin_mut(zoglin)
            .unwrap()
            .hoglin
            .persistence_required = true;
        let player = [DVec3::new(0.5, 64.0, 2.5)];
        assert_eq!(world.check_despawn(&player, false, &|_| true), 0);
        let far = [DVec3::new(0.5, 64.0, 200.5)];
        assert_eq!(world.check_despawn(&far, false, &|_| true), 1);
        assert!(world.hoglin_mut(hoglin).is_none());
        assert!(world.hoglin_mut(zoglin).is_some(), "persistent");
        assert_eq!(world.check_despawn(&player, true, &|_| true), 1);
        assert!(world.hoglins().is_empty());
    }

    #[test]
    fn a_zoglin_shrugs_off_lava_and_a_hoglin_burns() {
        let mut world = floor();
        for x in -4..=4 {
            world
                .blocks
                .insert((x, 64, 0), Block::new("minecraft:lava").with("level", "0"));
        }
        let mut entities = EntityWorld::default();
        let hoglin = spawn(&mut entities, HoglinKind::Hoglin, false, -2.5);
        let zoglin = spawn(&mut entities, HoglinKind::Zoglin, false, 2.5);
        for _ in 0..40 {
            entities.tick_with_players(&mut world, &[]);
        }
        let burnt = &entities.hoglin_mut(hoglin).unwrap().hoglin;
        assert!(burnt.health < 40.0 && burnt.body.fire_ticks > 0);
        let zoglin = &entities.hoglin_mut(zoglin).unwrap().hoglin;
        assert_eq!((zoglin.health, zoglin.body.fire_ticks), (40.0, 0));
        assert!(crate::world::fire_immune("minecraft:zoglin"));
        assert!(!crate::world::fire_immune("minecraft:hoglin"));
    }

    #[test]
    fn a_hoglin_takes_four_tenths_of_the_knockback() {
        let mut world = EntityWorld::default();
        let hoglin = spawn(&mut world, HoglinKind::Hoglin, false, 0.5);
        world.hoglin_mut(hoglin).unwrap().hoglin.body.on_ground = true;
        assert!(world.player_attack(&attack(1.0), hoglin).hurt);
        let v = world.hoglin_mut(hoglin).unwrap().hoglin.body.velocity;
        let expected = f64::from(0.4_f32) * (1.0 - crate::hoglin::KNOCKBACK_RESISTANCE);
        assert!((v.z + expected).abs() < 1.0e-9, "{v}");
        assert!((v.y - expected).abs() < 1.0e-9, "{v}");
    }

    #[test]
    fn an_iron_golem_goes_for_a_hoglin() {
        let mut world = floor();
        let mut entities = EntityWorld::default();
        let hoglin = spawn(&mut entities, HoglinKind::Hoglin, false, 0.5);
        let zoglin = spawn(&mut entities, HoglinKind::Zoglin, false, -5.5);
        let golem = crate::iron_golem::IronGolem::new(DVec3::new(5.5, 64.0, 0.5));
        let golem = entities.spawn_iron_golem(golem, false);
        entities.iron_golem_mut(golem).unwrap().random = LegacyRandom::new(5);
        for _ in 0..400 {
            entities.tick_with_players(&mut world, &[]);
        }
        let hurt = |entities: &mut EntityWorld, id| {
            entities
                .hoglin_mut(id)
                .is_none_or(|e| e.hoglin.health < crate::hoglin::MAX_HEALTH)
        };
        assert!(hurt(&mut entities, hoglin) || hurt(&mut entities, zoglin));
        assert!(crate::monster_ai::is_enemy("minecraft:hoglin"));
        assert!(crate::monster_ai::is_enemy("minecraft:zoglin"));
    }
}
