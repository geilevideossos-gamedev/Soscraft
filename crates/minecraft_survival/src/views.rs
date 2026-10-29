use std::collections::BTreeMap;

use minecraft_net::messages::{
    Abilities, CooldownView, EffectView, GameMode, Hand, Nbt, PLAYER_SLOTS, PlayerView, UseKind,
    UsingView,
};
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::survival::SurvivalStatus;

use crate::guns::{HOTBAR, WEAPON_PREFIX, weapon_item, weapon_of};
use crate::offhand::OFFHAND;
use crate::player::Player;
use crate::player_nbt::EFFECTS;
use crate::survival::{DAMAGE_BLINK, SETTLE_DISPLAY, TICKS_TO_FREEZE, Vitals, percent_frozen};

const MAX_AIR: i32 = 300;
const POISON: &str = "minecraft:poison";
const WITHER: &str = "minecraft:wither";
const HUNGER: &str = "minecraft:hunger";
const REGENERATION: &str = "minecraft:regeneration";
pub const SELECTION_WAIT: f64 = 1.0;

pub fn game_mode(mode: minecraftoss_player::GameMode) -> GameMode {
    match mode {
        minecraftoss_player::GameMode::Survival => GameMode::Survival,
        minecraftoss_player::GameMode::Creative => GameMode::Creative,
        minecraftoss_player::GameMode::Adventure => GameMode::Adventure,
        minecraftoss_player::GameMode::Spectator => GameMode::Spectator,
    }
}

fn count(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

pub fn player_view(player: &Player) -> PlayerView {
    let survival = &player.survival;
    let status = &survival.status;
    let abilities = &player.mode.abilities;
    let armor = crate::survival::armor(&player.inventory)
        .value
        .map_or(0, |(points, _)| points as i32);
    PlayerView {
        health: status.health,
        max_health: status.max_health,
        absorption: status.absorption,
        food: i32::from(status.food.level),
        saturation: status.food.saturation,
        air: status.air,
        fire_ticks: survival.fire_ticks(),
        frozen: survival.ticks_frozen(),
        armor,
        xp_level: count(status.experience_level),
        xp_progress: status.experience_progress,
        xp_total: count(status.total_experience),
        game_mode: game_mode(player.mode.mode),
        abilities: Abilities {
            invulnerable: abilities.invulnerable,
            flying: abilities.flying,
            may_fly: abilities.may_fly,
            instabuild: abilities.instabuild,
            may_build: abilities.may_build,
            fly_speed: abilities.flying_speed,
            walk_speed: abilities.walking_speed,
        },
        effects: effect_views(status),
        slots: (0..PLAYER_SLOTS)
            .map(|slot| {
                player
                    .inventory
                    .slots
                    .get(slot)
                    .and_then(Option::as_ref)
                    .and_then(slot_nbt)
            })
            .collect(),
        selected: u8::try_from(player.selected).unwrap_or(0),
        cooldowns: player
            .hand
            .cooldowns
            .entries()
            .into_iter()
            .map(|(group, remaining, total)| CooldownView {
                group,
                remaining: count(remaining),
                total: count(total),
            })
            .collect(),
        using: using(player),
        jump_bar: player.jump_bar,
    }
}

pub fn effect_views(status: &SurvivalStatus) -> Vec<EffectView> {
    EFFECTS
        .iter()
        .filter_map(|&(kind, id)| {
            let effect = status.effect(kind)?;
            Some(EffectView {
                kind: id.to_owned(),
                amplifier: effect.amplifier,
                duration: count(effect.duration),
                ambient: false,
            })
        })
        .collect()
}

fn using(player: &Player) -> Option<UsingView> {
    let hand = |slot: usize| {
        if slot == OFFHAND {
            Hand::Off
        } else {
            Hand::Main
        }
    };
    if let Some(eating) = player.survival.eating() {
        return Some(UsingView {
            hand: hand(eating.slot),
            ticks: count(eating.remaining_ticks()),
            kind: UseKind::Eat {
                duration: count(eating.info.consume_ticks),
            },
        });
    }
    if let Some(blocking) = player.hand.blocking {
        return Some(UsingView {
            hand: hand(blocking.slot),
            ticks: count(blocking.ticks),
            kind: UseKind::Block,
        });
    }
    if let Some(charging) = player.hand.charging {
        return Some(UsingView {
            hand: hand(charging.slot),
            ticks: count(charging.ticks),
            kind: UseKind::Charge,
        });
    }
    player.hand.scoping.map(|scoping| UsingView {
        hand: hand(scoping.slot),
        ticks: count(scoping.ticks),
        kind: UseKind::Scope,
    })
}

pub fn sent_key(view: &PlayerView) -> PlayerView {
    let mut key = view.clone();
    for effect in &mut key.effects {
        effect.duration = 0;
    }
    for cooldown in &mut key.cooldowns {
        cooldown.remaining = 0;
    }
    if let Some(using) = key.using.as_mut() {
        using.ticks = 0;
    }
    key
}

pub fn lengthened(before: &[EffectView], now: &[EffectView]) -> bool {
    now.iter().any(|effect| {
        before
            .iter()
            .any(|old| old.kind == effect.kind && effect.duration > old.duration)
    })
}

pub fn slot_nbt(stack: &ItemStack) -> Option<Nbt> {
    let tag = match weapon_of(stack) {
        Some(_) => Tag::Compound(BTreeMap::from([
            ("id".to_owned(), Tag::String(stack.id.clone())),
            ("count".to_owned(), Tag::Int(i32::from(stack.count))),
        ])),
        None => minecraft_terrain::item_nbt::stack_tag(stack)?,
    };
    Some(Nbt(minecraftoss_core::nbt::write(&tag, "")))
}

pub fn view_stack(nbt: &Nbt, recipes: &RecipeBook) -> Option<ItemStack> {
    let tag = minecraftoss_core::nbt::read(&nbt.0).ok()?;
    let id = tag.get("id")?.as_str()?;
    if let Some(weapon) = id
        .strip_prefix(WEAPON_PREFIX)
        .and_then(|weapon| weapon.parse().ok())
    {
        return Some(weapon_item(weapon));
    }
    minecraft_terrain::item_nbt::stack_of(&tag, recipes)
}

#[derive(Clone, Debug, Default)]
pub struct HudClock {
    tick: u64,
    clock: f64,
    last_health: Option<i32>,
    blink_until: u64,
    changed_at: u64,
    display_health: i32,
    effects_seen: Vec<EffectView>,
    effects_view: u32,
    effects_at: u64,
}

impl HudClock {
    pub fn vitals(
        &mut self,
        view: &PlayerView,
        seconds: f64,
        eye_in_water: bool,
        eye_in_lava: bool,
    ) -> Vitals {
        let health = view.health.ceil() as i32;
        if self.last_health.is_none() {
            self.last_health = Some(health);
            self.display_health = health;
        }
        self.clock += seconds.max(0.0);
        let ticks = (self.clock * 20.0).floor();
        self.clock -= ticks / 20.0;
        for _ in 0..ticks as u64 {
            self.tick += 1;
            self.update(health);
        }
        let has = |id: &str| view.effects.iter().any(|effect| effect.kind == id);
        let regen_index = if has(REGENERATION) {
            (self.tick % (view.max_health + 5.0).ceil() as u64) as i32
        } else {
            -1
        };
        Vitals {
            air: view.air,
            max_air: MAX_AIR,
            eye_in_water,
            health: view.health,
            max_health: view.max_health,
            absorption: view.absorption,
            food: u8::try_from(view.food.clamp(0, 20)).unwrap_or(0),
            saturation: view.saturation,
            poisoned: has(POISON),
            withered: has(WITHER),
            hunger: has(HUNGER),
            regen_index,
            blink: self.blink_until > self.tick && (self.blink_until - self.tick) / 3 % 2 == 1,
            display_health: self.display_health,
            tick: self.tick,
            experience_level: u32::try_from(view.xp_level).unwrap_or(0),
            experience_progress: view.xp_progress,
            on_fire: view.fire_ticks > 0,
            eye_in_lava,
            frozen: percent_frozen(view.frozen),
            fully_frozen: view.frozen >= TICKS_TO_FREEZE,
        }
    }

    pub fn effects(&mut self, view: &PlayerView, received: u32) -> Vec<EffectView> {
        if self.effects_seen != view.effects || self.effects_view != received {
            self.effects_seen = view.effects.clone();
            self.effects_view = received;
            self.effects_at = self.tick;
        }
        let passed = i32::try_from(self.tick - self.effects_at).unwrap_or(i32::MAX);
        view.effects
            .iter()
            .filter_map(|effect| {
                let duration = if effect.duration == i32::MAX {
                    i32::MAX
                } else {
                    effect.duration.saturating_sub(passed)
                };
                (duration > 0).then(|| EffectView {
                    duration,
                    ..effect.clone()
                })
            })
            .collect()
    }

    fn update(&mut self, health: i32) {
        let last = self.last_health.unwrap_or(health);
        if health < last {
            self.blink_until = self.tick + DAMAGE_BLINK;
        }
        if health != last {
            self.changed_at = self.tick;
        }
        if self.tick.saturating_sub(self.changed_at) > SETTLE_DISPLAY {
            self.display_health = health;
        }
        self.last_health = Some(health);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Selection {
    pub slot: usize,
    asked: Option<(u8, f64)>,
    seen: Option<u8>,
}

impl Selection {
    pub fn pick(&mut self, slot: usize, now: f64) -> Option<u8> {
        let slot = slot.min(HOTBAR - 1);
        if slot == self.slot {
            return None;
        }
        self.slot = slot;
        let asked = u8::try_from(slot).unwrap_or(0);
        self.asked = Some((asked, now));
        Some(asked)
    }

    pub fn follow(&mut self, shown: u8, now: f64) {
        let shown_slot = usize::from(shown).min(HOTBAR - 1);
        match self.asked {
            Some((asked, _)) if shown == asked => self.asked = None,
            Some((_, at)) if now - at < SELECTION_WAIT => {}
            Some(_) => {
                self.asked = None;
                self.slot = shown_slot;
            }
            None if self.seen != Some(shown) => self.slot = shown_slot,
            None => {}
        }
        self.seen = Some(shown);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use minecraft_commands::{CommandServer, PlayerId};
    use minecraft_terrain::game_mode::PlayerMode;
    use minecraftoss_player::inventory::Inventory;

    use crate::survival::{Motion, Survival};

    fn player(chat: &mut CommandServer, id: u32, name: &str) -> Player {
        let id = PlayerId(id);
        chat.join(id, name);
        Player::new(
            chat.profile(id).unwrap(),
            PlayerMode::default(),
            Survival::default(),
            Inventory::default(),
        )
    }

    fn standing(feet: [f64; 3], sprinting: bool) -> Motion {
        Motion {
            feet,
            on_ground: true,
            sprinting,
            jumping: false,
            in_water: false,
            eye_in_water: false,
            in_lava: false,
            eye_in_lava: false,
            in_rain: false,
            crouching: false,
            height: 1.8,
            riding: false,
        }
    }

    #[test]
    fn two_players_lose_hunger_and_health_on_their_own() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut runner = player(&mut chat, 0, "Steve");
        let mut idle = player(&mut chat, 1, "Alex");
        let fresh = player_view(&idle);
        for tick in 0..2400 {
            let x = f64::from(tick) * 0.28;
            runner.survival.tick(&standing([x, 64.0, 0.0], true));
            idle.survival.tick(&standing([0.0, 64.0, 0.0], false));
        }
        runner.survival.hurt(
            5.0,
            minecraftoss_player::survival::Armor::default(),
            None,
            crate::death::DamageCause::Generic,
        );
        let ran = player_view(&runner);
        let rested = player_view(&idle);
        assert!(ran.food < fresh.food, "{} < {}", ran.food, fresh.food);
        assert!(ran.health < rested.health);
        assert_eq!(rested.food, fresh.food);
        assert_eq!(rested.health, fresh.health);
    }

    #[test]
    fn the_view_carries_items_and_guns_and_they_come_back() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut alex = player(&mut chat, 1, "Alex");
        alex.inventory.slots[0] = Some(weapon_item(12));
        let mut gun = ItemStack::new("iw4l:m4", 1);
        gun.components = Some(serde_json::json!({
            "minecraft:custom_data": {"iw4l": {"attachments": ["reflex", "gl"], "clip": 27, "alt_clip": 1}}
        }));
        alex.inventory.slots[1] = Some(gun.clone());
        alex.inventory.slots[2] = Some(ItemStack::new("iw4l:frag_grenade", 9));
        alex.inventory.slots[3] = Some(ItemStack::new("minecraft:dirt", 17));
        alex.inventory.slots[OFFHAND] = Some(ItemStack::new("minecraft:shield", 1));
        alex.inventory.slots[43] = Some(ItemStack::new("iw4l:semtex", 30));
        alex.inventory.slots[44] = Some(ItemStack::new("iw4l:smoke_grenade", 2));
        alex.selected = 3;
        let view = player_view(&alex);
        assert_eq!(view.slots.len(), 45);
        assert_eq!(view.selected, 3);
        let recipes = RecipeBook::default();
        let back = |slot: usize| {
            view.slots[slot]
                .as_ref()
                .and_then(|nbt| view_stack(nbt, &recipes))
                .map(|stack| (stack.id, stack.count))
        };
        assert_eq!(back(0), Some(("iw4:weapon/12".to_owned(), 1)));
        assert_eq!(back(3), Some(("minecraft:dirt".to_owned(), 17)));
        assert_eq!(back(OFFHAND), Some(("minecraft:shield".to_owned(), 1)));
        assert_eq!(back(4), None);
        assert_eq!(back(1), Some(("iw4l:m4".to_owned(), 1)));
        assert_eq!(back(2), Some(("iw4l:frag_grenade".to_owned(), 9)));
        assert_eq!(back(43), Some(("iw4l:semtex".to_owned(), 30)));
        assert_eq!(back(44), Some(("iw4l:smoke_grenade".to_owned(), 2)));
        let loaded = view.slots[1]
            .as_ref()
            .and_then(|nbt| view_stack(nbt, &recipes))
            .unwrap();
        assert_eq!(loaded.components, gun.components, "clip and attachments survive");
    }

    #[test]
    fn a_friend_sees_a_fork_item_through_slot_nbt() {
        let mut gun = ItemStack::new("iw4l:barrett", 1);
        gun.components = Some(serde_json::json!({
            "minecraft:custom_name": "Lucky",
            "minecraft:custom_data": {"iw4l": {"attachments": ["thermal"], "clip": 4}}
        }));
        let nbt = slot_nbt(&gun).expect("a fork gun goes on the wire");
        let tag = minecraftoss_core::nbt::read(&nbt.0).unwrap();
        assert_eq!(
            tag.get("id").and_then(Tag::as_str),
            Some(minecraft_terrain::item_nbt::GUN_CARRIER)
        );
        let recipes = RecipeBook::default();
        let seen = view_stack(&nbt, &recipes).unwrap();
        assert_eq!((&seen.id, seen.count, &seen.components), (&gun.id, 1, &gun.components));
        let rockets = ItemStack::new("iw4l:rocket", 16);
        let cells = vec![slot_nbt(&gun), None, slot_nbt(&rockets), slot_nbt(&weapon_item(3))];
        let shown: Vec<Option<(String, u8)>> = crate::menus::stacks(&cells, &recipes)
            .into_iter()
            .map(|cell| cell.map(|stack| (stack.id, stack.count)))
            .collect();
        assert_eq!(
            shown,
            [
                Some(("iw4l:barrett".to_owned(), 1)),
                None,
                Some(("iw4l:rocket".to_owned(), 16)),
                Some(("iw4:weapon/3".to_owned(), 1)),
            ]
        );
    }

    #[test]
    fn only_a_change_that_shows_makes_a_new_key() {
        let mut view = PlayerView::default();
        view.effects.push(EffectView {
            kind: REGENERATION.to_owned(),
            amplifier: 0,
            duration: 100,
            ambient: false,
        });
        let key = sent_key(&view);
        view.effects[0].duration = 99;
        assert_eq!(
            sent_key(&view),
            key,
            "a running effect counts down on its own"
        );
        view.food = 19;
        assert_ne!(sent_key(&view), key);
    }

    #[test]
    fn a_friend_s_effects_count_down_between_views_and_restart_from_a_new_one() {
        let mut clock = HudClock::default();
        let mut view = PlayerView::default();
        let effect = |kind: &str, duration: i32| EffectView {
            kind: kind.to_owned(),
            amplifier: 1,
            duration,
            ambient: false,
        };
        view.effects = vec![effect(REGENERATION, 230), effect(HUNGER, i32::MAX)];
        clock.vitals(&view, 0.0, false, false);
        assert_eq!(clock.effects(&view, 0), view.effects);
        clock.vitals(&view, 1.5, false, false);
        let shown = clock.effects(&view, 0);
        assert_eq!(
            shown.iter().map(|e| e.duration).collect::<Vec<_>>(),
            [200, i32::MAX],
            "thirty ticks later, and the unending one stays"
        );
        view.effects[0].duration = 120;
        assert_eq!(clock.effects(&view, 0)[0].duration, 120, "a new view is taken as it is");
        clock.vitals(&view, 6.0, false, false);
        assert_eq!(
            clock.effects(&view, 0),
            vec![effect(HUNGER, i32::MAX)],
            "a run-out effect is gone before the host says so"
        );
    }

    #[test]
    fn a_friend_s_effect_given_again_with_the_same_length_counts_down_from_the_start() {
        let mut clock = HudClock::default();
        let view = PlayerView {
            effects: vec![EffectView {
                kind: "minecraft:darkness".to_owned(),
                amplifier: 0,
                duration: 260,
                ambient: false,
            }],
            ..Default::default()
        };
        clock.vitals(&view, 0.0, false, false);
        assert_eq!(clock.effects(&view, 1)[0].duration, 260);
        clock.vitals(&view, 3.0, false, false);
        assert_eq!(clock.effects(&view, 1)[0].duration, 200);
        assert_eq!(
            clock.effects(&view, 2)[0].duration,
            260,
            "the same view sent again is a new grant"
        );
        clock.vitals(&view, 13.0, false, false);
        assert!(clock.effects(&view, 2).is_empty());
    }

    #[test]
    fn the_hud_blinks_on_damage_and_settles_the_shown_health() {
        let mut clock = HudClock::default();
        let mut view = PlayerView::default();
        let first = clock.vitals(&view, 0.05, false, false);
        assert_eq!(first.display_health, 20);
        assert!(!first.blink);
        view.health = 14.0;
        let hurt = clock.vitals(&view, 0.2, false, false);
        assert_eq!(hurt.display_health, 20, "the old hearts stay a moment");
        let blinking = (0..10).any(|_| clock.vitals(&view, 0.05, false, false).blink);
        assert!(blinking);
        let settled = clock.vitals(&view, 2.0, false, false);
        assert_eq!(settled.display_health, 14);
        assert!(!settled.blink);
        assert_eq!(settled.food, 20);
    }

    #[test]
    fn a_friends_frost_reaches_their_hearts_and_outline() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut alex = player(&mut chat, 1, "Alex");
        let mut clock = HudClock::default();
        let mut view = player_view(&alex);
        assert_eq!(view.frozen, 0);
        let status = alex.survival.status;
        alex.survival.restore(status, 0, 90, 0);
        assert_eq!(player_view(&alex).frozen, 90);
        let thawed = clock.vitals(&view, 0.05, false, false);
        assert_eq!((thawed.frozen, thawed.fully_frozen), (0.0, false));
        view.frozen = 70;
        let half = clock.vitals(&view, 0.05, false, false);
        assert_eq!((half.frozen, half.fully_frozen), (0.5, false));
        view.frozen = TICKS_TO_FREEZE;
        let full = clock.vitals(&view, 0.05, false, false);
        assert_eq!((full.frozen, full.fully_frozen), (1.0, true));
    }

    #[test]
    fn a_picked_slot_waits_for_the_host_and_an_mw2_switch_is_followed() {
        let mut selection = Selection::default();
        selection.follow(0, 0.0);
        assert_eq!(selection.pick(4, 0.0), Some(4));
        assert_eq!(selection.pick(4, 0.1), None);
        selection.follow(0, 0.2);
        assert_eq!(selection.slot, 4, "a view sent before the pick arrived");
        selection.follow(4, 0.3);
        assert_eq!(selection.slot, 4);
        selection.follow(2, 0.4);
        assert_eq!(selection.slot, 2, "MW2 raised the gun in slot 2");
        assert_eq!(selection.pick(7, 1.0), Some(7));
        selection.follow(2, 1.5);
        assert_eq!(selection.slot, 7);
        selection.follow(2, 2.5);
        assert_eq!(selection.slot, 2, "the host never took the pick");
        assert_eq!(selection.pick(20, 3.0), Some(8));
    }

    #[test]
    fn a_friend_is_told_what_kind_of_use_their_hand_is_in() {
        let mut chat = CommandServer::new(crate::commands::HOST);
        let mut alex = player(&mut chat, 1, "Alex");
        assert_eq!(player_view(&alex).using, None);
        alex.survival.status.food.level = 4;
        let steak = ItemStack::new("minecraft:cooked_beef", 8);
        alex.inventory.slots[2] = Some(steak.clone());
        alex.selected = 2;
        assert!(alex.survival.start_eating(2, &steak));
        let eating = player_view(&alex).using.expect("eating");
        assert_eq!(
            (eating.hand, eating.ticks, eating.kind),
            (Hand::Main, 32, UseKind::Eat { duration: 32 })
        );
        alex.survival.stop_eating();
        let kelp = ItemStack::new("minecraft:dried_kelp", 8);
        alex.inventory.slots[OFFHAND] = Some(kelp.clone());
        assert!(alex.survival.start_eating(OFFHAND, &kelp));
        let kelp = player_view(&alex).using.expect("eating kelp");
        assert_eq!(
            (kelp.hand, kelp.ticks, kelp.kind),
            (Hand::Off, 16, UseKind::Eat { duration: 16 })
        );
        alex.survival.stop_eating();
        alex.hand.blocking = Some(crate::offhand::Blocking {
            slot: OFFHAND,
            ticks: 6,
        });
        let block = player_view(&alex).using.expect("blocking");
        assert_eq!(
            (block.hand, block.ticks, block.kind),
            (Hand::Off, 6, UseKind::Block)
        );
        alex.hand.blocking = None;
        alex.hand.charging = Some(crate::trident::Charging { slot: 2, ticks: 11 });
        let charge = player_view(&alex).using.expect("charging");
        assert_eq!(
            (charge.hand, charge.ticks, charge.kind),
            (Hand::Main, 11, UseKind::Charge)
        );
        alex.hand.charging = None;
        alex.hand.scoping = Some(crate::spyglass::Scoping { slot: 2, ticks: 3 });
        let scope = player_view(&alex).using.expect("scoping");
        assert_eq!(scope.kind, UseKind::Scope);
        let mut later = player_view(&alex);
        later.using.as_mut().expect("scoping").ticks = 40;
        assert_eq!(sent_key(&later), sent_key(&player_view(&alex)));
    }
}
