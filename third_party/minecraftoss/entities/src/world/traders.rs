use super::emissions::{MovementSounds, base_tick_fluid, play_movement};
use super::*;
use crate::monster_ai::{MonsterKind, Target};
use crate::wandering_trader::{self as trader, Drink, WanderingTrader};

const SOUNDS: MovementSounds = MovementSounds::creature(None);

#[derive(Clone)]
pub struct TraderEntity {
    pub id: u64,
    pub trader: WanderingTrader,
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
    pub offers: Option<Vec<crate::trading::MerchantOffer>>,
    pub trading_player: Option<u64>,
    pub despawned: bool,
}

impl TraderEntity {
    pub fn position(&self) -> DVec3 {
        self.trader.body.position
    }

    pub fn invisible(&self) -> bool {
        self.effects.has(MobEffect::Invisibility)
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
        if self.trader.health > 0.0 && !self.trader.damage.dead {
            self.no_action_time = 0;
        }
        let result =
            self.trader
                .damage
                .hurt_generic(&mut self.trader.health, trader::MAX_HEALTH, amount);
        let position = self.position();
        self.trader
            .damage
            .place_death(result, position, self.trader.body.fire_ticks > 0);
        if result.applied {
            self.last_damage = Some((kind, game_time));
            if let Some(attacker) = attacker {
                self.ai.state.hurt_by = Some((attacker, self.tick_count));
            }
            if result.full {
                if !result.died {
                    self.ambient_sound_time = -trader::AMBIENT_INTERVAL;
                }
                let pitch = voice_pitch(&mut self.random, false);
                let event = if result.died {
                    trader::DEATH_SOUND
                } else {
                    trader::HURT_SOUND
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
        self.ambient_sound_time = -trader::AMBIENT_INTERVAL;
        let event = if self.trading_player.is_some() {
            trader::TRADE_SOUND
        } else {
            trader::AMBIENT_SOUND
        };
        let pitch = voice_pitch(&mut self.random, false);
        self.voices
            .push((Voice::Event(event, 1.0, pitch), self.position()));
    }

    fn speak(&mut self, event: &'static str) {
        let pitch = voice_pitch(&mut self.random, false);
        self.voices
            .push((Voice::Event(event, 1.0, pitch), self.position()));
    }

    fn tick_ai(&mut self, world: &impl World, game_time: i64) -> Option<f64> {
        let invisible = self.invisible();
        let within_home = self.trader.within_home();
        let ai = &mut self.ai;
        let body = &mut self.trader.body;
        if ai.no_jump_delay > 0 {
            ai.no_jump_delay -= 1;
        }
        body.trim_small_velocity();
        ai.state.body = body.clone();
        ai.state.health = self.trader.health;
        ai.state.max_health = trader::MAX_HEALTH;
        ai.state.game_time = game_time;
        ai.state.tick_count = self.tick_count;
        ai.state.on_fire = body.fire_ticks > 0;
        ai.state.eye_height = trader::EYE_HEIGHT;
        let state = &mut ai.state.trader;
        state.trading = self.trading_player;
        state.invisible = invisible;
        state.drinking = self.trader.drinking.is_some();
        state.hurt_recently = self.trader.damage.hurt_ticks > 0;
        state.wander_target = self.trader.wander_target;
        state.home = self.trader.home;
        state.within_home = within_home;
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
        self.trader.wander_target = ai.state.trader.wander_target;
        if let Some(drink) = ai.state.trader.drink.take() {
            self.trader.held = Some(drink);
            self.trader.drinking = Some((drink, trader::DRINK_TICKS));
        }
        if std::mem::take(&mut ai.state.trader.stop_trading) {
            self.trading_player = None;
        }
        let finished = ai.state.trader.finished.take();
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
        self.random = std::mem::take(&mut ai.state.random);
        let obstacle_top = crate::control::obstacle_top(world, position);
        let speed = self.effects.movement_speed(trader::MOVEMENT_SPEED);
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
            trader::EYE_HEIGHT,
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
        self.trader.yaw = ai.yaw;
        if let Some(drink) = finished {
            self.trader.held = None;
            self.trader.drinking = None;
            let pitch = self.random.next_float() * 0.2 + 0.9;
            let at = self.position();
            self.voices
                .push((Voice::Event(drink.finish_sound(), 1.0, pitch), at));
        }
        landed
    }

    fn tick_drink(&mut self) {
        let (sound, finished) = self.trader.tick_drinking();
        if let Some(event) = sound {
            let pitch = self.random.next_float() * 0.1 + 0.9;
            let at = self.position();
            self.voices.push((Voice::Event(event, 0.5, pitch), at));
        }
        match finished {
            Some(Drink::Invisibility) => {
                self.effects.add(crate::effects::EffectInstance::new(
                    MobEffect::Invisibility,
                    trader::INVISIBILITY_TICKS,
                    0,
                ));
            }
            Some(Drink::Milk) => {
                for effect in MobEffect::ALL {
                    self.effects.remove(effect);
                }
            }
            None => {}
        }
    }
}

impl EntityWorld {
    pub fn spawn_wandering_trader(&mut self, trader: WanderingTrader, no_ai: bool) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let ai = MonsterAi::of_kind(MonsterKind::WanderingTrader, &trader.body, trader.yaw);
        self.wandering_traders.push(TraderEntity {
            id,
            previous_position: trader.body.position,
            trader,
            no_ai,
            tick_count: 0,
            ambient_sound_time: 0,
            voices: Vec::new(),
            no_action_time: 0,
            random: LegacyRandom::new(0),
            ai: Box::new(ai),
            effects: Default::default(),
            last_damage: None,
            offers: None,
            trading_player: None,
            despawned: false,
        });
        self.order.push(EntityKey::Trader(id));
        self.file_in_section(EntityKey::Trader(id));
        id
    }

    pub fn wandering_traders(&self) -> &[TraderEntity] {
        &self.wandering_traders
    }

    pub fn wandering_trader_mut(&mut self, id: u64) -> Option<&mut TraderEntity> {
        self.wandering_traders.iter_mut().find(|e| e.id == id)
    }

    pub fn is_wandering_trader(&self, id: u64) -> bool {
        self.wandering_traders.iter().any(|e| e.id == id)
    }

    pub fn wandering_trader_offers(&mut self, id: u64) -> Option<&[crate::trading::MerchantOffer]> {
        let book = self.trades.clone();
        let entity = self.wandering_traders.iter_mut().find(|e| e.id == id)?;
        if entity.offers.is_none() {
            let Some(book) = book else { return Some(&[]) };
            let mut offers = Vec::new();
            for set in trader::TRADE_SETS {
                offers.extend(book.offers_from(set, "", &mut self.trade_sequences));
            }
            entity.offers = Some(offers);
        }
        entity.offers.as_deref()
    }

    pub fn wandering_trader_interact(
        &mut self,
        id: u64,
        player: u64,
        held: Option<&str>,
    ) -> VillagerUse {
        let Some(entity) = self.wandering_traders.iter().find(|e| e.id == id) else {
            return VillagerUse::Pass;
        };
        if held == Some("minecraft:villager_spawn_egg")
            || entity.trader.health <= 0.0
            || entity.trading_player.is_some()
            || entity.trader.age < 0
        {
            return VillagerUse::Pass;
        }
        if self
            .wandering_trader_offers(id)
            .is_none_or(<[crate::trading::MerchantOffer]>::is_empty)
        {
            return VillagerUse::Consume;
        }
        if let Some(entity) = self.wandering_trader_mut(id) {
            entity.trading_player = Some(player);
        }
        self.merchant_menus
            .insert(player, crate::merchant::MerchantMenu::new(id));
        VillagerUse::Success
    }

    pub(super) fn wandering_trader_trade_updated(&mut self, id: u64, valid: bool) {
        let Some(entity) = self.wandering_trader_mut(id) else {
            return;
        };
        if entity.ambient_sound_time > -trader::AMBIENT_INTERVAL + 20 {
            entity.ambient_sound_time = -trader::AMBIENT_INTERVAL;
            entity.speak(if valid {
                trader::YES_SOUND
            } else {
                trader::NO_SOUND
            });
        }
    }

    pub(super) fn wandering_trader_notify_trade(&mut self, id: u64, index: usize) {
        let Some(entity) = self.wandering_traders.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let Some(offer) = entity.offers.as_mut().and_then(|o| o.get_mut(index)) else {
            return;
        };
        offer.uses += 1;
        let reward = offer.reward_exp;
        entity.ambient_sound_time = -trader::AMBIENT_INTERVAL;
        if reward {
            let pop = 3 + entity.random.next_int(4) as i32;
            let p = entity.trader.body.position;
            self.trade_experience
                .push((DVec3::new(p.x, p.y + 0.5, p.z), pop));
        }
    }

    pub fn take_departed_traders(&mut self) -> Vec<u64> {
        let gone: Vec<u64> = self
            .wandering_traders
            .iter()
            .filter(|e| e.despawned)
            .map(|e| e.id)
            .collect();
        if !gone.is_empty() {
            self.remove_mobs(&gone);
        }
        gone
    }

    pub(super) fn tick_wandering_trader(
        &mut self,
        id: u64,
        world: &mut impl World,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let (game_time, difficulty, griefing) =
            (self.game_time, self.difficulty, self.mob_griefing);
        let bright = self.bright_outside;
        let mobs = self.mob_candidates();
        let Some(entity) = self.wandering_traders.iter_mut().find(|e| e.id == id) else {
            return;
        };
        entity.previous_position = entity.trader.body.position;
        entity.tick_count += 1;
        base_tick_fluid(
            &mut entity.trader.body,
            &*world,
            entity.tick_count == 1,
            &mut entity.random,
            &mut entity.voices,
            SOUNDS,
        );
        super::hazards::burn(entity, &*world, game_time);
        super::hazards::suffocate(entity, &*world, trader::EYE_HEIGHT, game_time);
        let water_breathing = entity.effects.has(MobEffect::WaterBreathing);
        if entity.trader.health > 0.0
            && super::living::breathe(
                &mut entity.trader.body,
                &*world,
                trader::EYE_HEIGHT,
                water_breathing,
            )
        {
            entity.hurt_from(2.0, "minecraft:drown", None, game_time);
        }
        let removed = entity.trader.damage.tick();
        if entity
            .ai
            .state
            .hurt_by
            .is_some_and(|(_, when)| entity.tick_count - when > 100)
        {
            entity.ai.state.hurt_by = None;
        }
        for work in entity.effects.tick(false) {
            match work.resolve(entity.trader.health, trader::MAX_HEALTH) {
                Some(EffectWork::Heal(amount)) => {
                    heal(&mut entity.trader.health, trader::MAX_HEALTH, amount)
                }
                Some(EffectWork::HurtMagic(amount)) => {
                    entity.hurt_from(amount, "minecraft:magic", None, game_time);
                }
                _ => {}
            }
        }
        entity.trader.body.float_from(&entity.effects);
        if entity.trader.health > 0.0 {
            entity.ambient();
        }
        if removed {
            return;
        }
        if entity.trader.health > 0.0 {
            entity.tick_drink();
        }
        let mut level_random = std::mem::take(&mut self.level_random);
        let Some(entity) = self.wandering_traders.iter_mut().find(|e| e.id == id) else {
            self.level_random = level_random;
            return;
        };
        if entity.trader.health <= 0.0 && !entity.no_ai {
            let yaw = entity.trader.yaw;
            let landed = super::living::dying_travel(
                &mut entity.trader.body,
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
                    &mut entity.trader.body,
                    world,
                    fallen,
                    &mut level_random,
                    griefing,
                )
            }) {
                entity.previous_position = entity.trader.body.position;
            }
        } else if !entity.no_ai {
            let state = &mut entity.ai.state;
            state.difficulty = difficulty;
            state.bright_outside = bright;
            state.players = players.to_vec();
            state.mobs = mobs;
            state.level_random = Some(level_random);
            let landed = entity.tick_ai(&*world, game_time);
            level_random = entity.ai.state.level_random.take().unwrap_or_default();
            if let Some(fallen) = landed {
                if super::living::fall_on_farmland(
                    &mut entity.trader.body,
                    world,
                    fallen,
                    &mut level_random,
                    griefing,
                ) {
                    entity.previous_position = entity.trader.body.position;
                }
                let sounds = &mut entity.voices;
                if let Some(damage) =
                    fall_damage(&entity.trader.body, &*world, fallen, false, sounds)
                {
                    entity.hurt_from(damage, "minecraft:fall", None, game_time);
                }
            }
        } else {
            entity.trader.body.trim_small_velocity();
        }
        self.level_random = level_random;
        let Some(entity) = self.wandering_traders.iter_mut().find(|e| e.id == id) else {
            return;
        };
        let old = entity.previous_position;
        super::hazards::blocks_act(entity, &*world, old, game_time);
        if entity.trader.health > 0.0 {
            let trading = entity.trading_player.is_some();
            if entity.trader.tick_despawn(trading) {
                entity.despawned = true;
            }
        }
        self.push_entities(EntityKey::Trader(id), &*world, game_time, ticks);
        self.file_in_section(EntityKey::Trader(id));
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

    fn trader_at(world: &mut EntityWorld, at: DVec3) -> u64 {
        world.spawn_wandering_trader(WanderingTrader::new(at), false)
    }

    #[test]
    fn a_wandering_trader_is_a_live_mob_of_its_size_and_health() {
        let mut world = EntityWorld::default();
        let id = trader_at(&mut world, DVec3::new(0.5, 64.0, 0.5));
        assert_eq!(world.entity_type(id), Some(trader::TYPE_ID));
        let entity = &world.wandering_traders()[0];
        assert_eq!(
            (
                entity.trader.body.width,
                entity.trader.body.height,
                entity.trader.health
            ),
            (0.6, 1.95, 20.0)
        );
        assert!(world.census().iter().any(|e| e.kind == trader::TYPE_ID));
        assert!(
            !world.can_be_leashed(id),
            "villagers and traders take no lead"
        );
        let hit = world.wandering_trader_mut(id).unwrap().hurt(4.0);
        assert!(hit.applied);
        let sounds: Vec<_> = world.take_sounds().into_iter().map(|s| s.event).collect();
        assert!(
            sounds.contains(&trader::HURT_SOUND.to_owned()),
            "{sounds:?}"
        );
    }

    #[test]
    fn a_trader_drinks_invisibility_at_night_and_milk_by_day() {
        let mut world = EntityWorld::default();
        let id = trader_at(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let mut floor = Floor;
        world.set_bright_outside(false);
        for _ in 0..80 {
            world.tick_with_players(&mut floor, &[]);
        }
        let entity = &world.wandering_traders()[0];
        assert!(entity.invisible(), "it drank the potion at night");
        let effect = entity.effects.get(MobEffect::Invisibility).unwrap();
        assert!(effect.duration > trader::INVISIBILITY_TICKS - 80);
        let events: Vec<_> = world.take_sounds().into_iter().map(|s| s.event).collect();
        assert!(events.contains(&trader::DRINK_POTION_SOUND.to_owned()));
        assert!(events.contains(&trader::DISAPPEARED_SOUND.to_owned()));
        world.set_bright_outside(true);
        for _ in 0..80 {
            world.tick_with_players(&mut floor, &[]);
        }
        let entity = world.wandering_trader_mut(id).unwrap();
        assert!(!entity.invisible(), "milk by day");
        assert_eq!(entity.trader.held, None);
        let events: Vec<_> = world.take_sounds().into_iter().map(|s| s.event).collect();
        assert!(events.contains(&trader::DRINK_MILK_SOUND.to_owned()));
        assert!(events.contains(&trader::REAPPEARED_SOUND.to_owned()));
    }

    #[test]
    fn a_trader_leaves_once_its_delay_runs_out_but_not_while_trading() {
        let mut world = EntityWorld::default();
        world.set_bright_outside(true);
        let id = trader_at(&mut world, DVec3::new(0.5, 64.0, 0.5));
        world.wandering_trader_mut(id).unwrap().trader.despawn_delay = 5;
        world.wandering_trader_mut(id).unwrap().trading_player = Some(0);
        let mut floor = Floor;
        let me = player(0, DVec3::new(1.5, 64.0, 0.5));
        for _ in 0..10 {
            world.tick_with_players(&mut floor, &[me]);
        }
        assert_eq!(
            world.wandering_trader_mut(id).unwrap().trader.despawn_delay,
            5
        );
        world.wandering_trader_mut(id).unwrap().trading_player = None;
        for _ in 0..4 {
            world.tick_with_players(&mut floor, &[me]);
        }
        assert!(world.take_departed_traders().is_empty());
        world.tick_with_players(&mut floor, &[me]);
        assert_eq!(world.take_departed_traders(), [id]);
        assert!(world.wandering_traders().is_empty());
        assert!(world.take_deaths().is_empty(), "it leaves, it does not die");
    }

    #[test]
    fn walking_away_from_a_trader_ends_the_trade() {
        let mut world = EntityWorld::default();
        world.set_bright_outside(true);
        let id = trader_at(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let mut floor = Floor;
        for _ in 0..3 {
            world.tick_with_players(&mut floor, &[]);
        }
        world.wandering_trader_mut(id).unwrap().trading_player = Some(0);
        for _ in 0..4 {
            world.tick_with_players(&mut floor, &[player(0, DVec3::new(2.5, 64.0, 0.5))]);
        }
        assert_eq!(world.wandering_traders()[0].trading_player, Some(0));
        assert!(
            world.wandering_traders()[0]
                .running_goals()
                .contains(&"TradeWithPlayerGoal")
        );
        for _ in 0..4 {
            world.tick_with_players(&mut floor, &[player(0, DVec3::new(6.5, 64.0, 0.5))]);
        }
        assert_eq!(world.wandering_traders()[0].trading_player, None);
    }

    fn trader_llama(world: &mut EntityWorld, at: DVec3) -> u64 {
        let mut cow = crate::cow::Cow::new(at);
        let mut horse = crate::horse::HorseState::new(crate::horse::HorseKind::TraderLlama);
        horse.max_health = 20.0;
        cow.health = 20.0;
        horse.set_strength(3);
        world.spawn_horse(cow, horse, false)
    }

    #[test]
    fn trader_llamas_on_its_lead_leave_with_the_trader_and_drop_no_lead() {
        let mut world = EntityWorld::default();
        world.set_bright_outside(true);
        let id = trader_at(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let llamas = [
            trader_llama(&mut world, DVec3::new(2.5, 64.0, 0.5)),
            trader_llama(&mut world, DVec3::new(-1.5, 64.0, 0.5)),
        ];
        for llama in llamas {
            world.set_leash(llama, LeashHolder::Mob(id));
        }
        world.wandering_trader_mut(id).unwrap().trader.despawn_delay = 6;
        let mut floor = Floor;
        for left in [5, 4] {
            world.tick_with_players(&mut floor, &[]);
            assert_eq!(world.wandering_traders()[0].trader.despawn_delay, left);
            for llama in llamas {
                let horse = world.cow_mut(llama).unwrap().horse.clone().unwrap();
                assert_eq!(horse.trader.map(|(trader, _)| trader), Some(id));
                assert!(horse.leashed);
                assert_eq!(horse.despawn_delay, left, "it keeps the trader's time");
            }
        }
        let mut gone = Vec::new();
        for _ in 0..5 {
            world.tick_with_players(&mut floor, &[]);
            gone.extend(world.take_departed_traders());
        }
        assert_eq!(gone, [id]);
        assert!(llamas.iter().all(|&llama| world.cow_mut(llama).is_none()));
        assert_eq!(world.leashes().count(), 0);
        assert!(
            world.take_lead_drops().is_empty(),
            "the lead goes with them"
        );
    }

    #[test]
    fn trader_llamas_turn_on_whoever_hurts_their_trader() {
        let mut world = EntityWorld::default();
        world.set_bright_outside(true);
        let id = trader_at(&mut world, DVec3::new(0.5, 64.0, 0.5));
        let llama = trader_llama(&mut world, DVec3::new(2.5, 64.0, 0.5));
        let free = trader_llama(&mut world, DVec3::new(0.5, 64.0, 3.5));
        world.wandering_trader_mut(id).unwrap().trader.despawn_delay = 1000;
        world.set_leash(llama, LeashHolder::Mob(id));
        let mut floor = Floor;
        let me = player(0, DVec3::new(-2.5, 64.0, 0.5));
        world.tick_with_players(&mut floor, &[me]);
        let time = world.game_time();
        world.wandering_trader_mut(id).unwrap().hurt_from(
            1.0,
            "player",
            Some(Target::Player(0)),
            time,
        );
        for _ in 0..3 {
            world.tick_with_players(&mut floor, &[me]);
        }
        let target = |world: &mut EntityWorld, id: u64| {
            world
                .cow_mut(id)
                .unwrap()
                .horse
                .as_ref()
                .unwrap()
                .fight
                .target
        };
        assert_eq!(target(&mut world, llama), Some(Target::Player(0)));
        assert_eq!(
            target(&mut world, free),
            None,
            "only its own trader's llamas"
        );
    }

    #[test]
    fn a_trader_heads_for_its_wander_target_and_forgets_it_there() {
        let mut world = EntityWorld::default();
        world.set_bright_outside(true);
        let id = trader_at(&mut world, DVec3::new(0.5, 64.0, 0.5));
        world.wandering_trader_mut(id).unwrap().trader.wander_target = Some((8, 64, 0));
        let mut floor = Floor;
        for _ in 0..400 {
            world.tick_with_players(&mut floor, &[]);
        }
        let entity = &world.wandering_traders()[0];
        assert_eq!(entity.trader.wander_target, None, "reached and forgotten");
        assert!(entity.position().x > 5.0, "{}", entity.position());
    }
}
