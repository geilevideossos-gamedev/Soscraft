use minecraft_terrain::mining_speed::Digger;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::survival::{EffectKind, SurvivalStatus};
use minecraftoss_player::{Block, Face, Pos};

pub const DESTROY_DELAY: u32 = 5;
pub const HIT_SOUND_TICKS: u32 = 4;
pub const HEAD: usize = 39;
pub const FEET: usize = 36;
const AIR: [&str; 3] = ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dig {
    pub destroying: Option<(Pos, Face, Option<ItemStack>)>,
    pub progress: f32,
    pub ticks: u32,
    pub delay: u32,
    pub pressed: bool,
}

pub struct DigInput<'a> {
    pub target: Option<(Pos, Face, &'a Block)>,
    pub held: Option<&'a ItemStack>,
    pub clicked: bool,
    pub held_down: bool,
    pub progress: &'a dyn Fn(&Block) -> f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DigEvent {
    Aborted(Pos),
    Attacked(Pos),
    Started { pos: Pos, face: Face },
    Hit { pos: Pos, face: Face, sound: bool },
    Broke { pos: Pos, instant: bool },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DigTick {
    pub events: Vec<DigEvent>,
    pub swing: bool,
    pub stopped: bool,
}

impl Dig {
    pub fn press(&mut self) {
        self.pressed = true;
    }

    pub fn take_press(&mut self) -> bool {
        std::mem::take(&mut self.pressed)
    }

    pub fn tick(&mut self, input: DigInput) -> DigTick {
        let mut out = DigTick::default();
        let target = input.target.filter(|(_, _, block)| !is_air(block));
        let mut down = input.held_down;
        if input.clicked
            && let Some((pos, face, block)) = target
        {
            out.swing = true;
            if self.start(pos, face, block, &input, &mut out.events) {
                down = false;
            }
        }
        if !down || input.target.is_none() {
            out.stopped = self.stop();
            return out;
        }
        let Some((pos, face, block)) = target else {
            self.forget();
            return out;
        };
        out.swing = true;
        if self.delay > 0 {
            self.delay -= 1;
            return out;
        }
        if !self.same_target(pos, input.held) {
            self.start(pos, face, block, &input, &mut out.events);
            return out;
        }
        self.progress += (input.progress)(block);
        self.ticks += 1;
        if self.progress >= 1.0 {
            self.forget();
            self.delay = DESTROY_DELAY;
            out.events.push(DigEvent::Broke {
                pos,
                instant: false,
            });
        } else {
            out.events.push(DigEvent::Hit {
                pos,
                face,
                sound: self.ticks % HIT_SOUND_TICKS == 0,
            });
        }
        out
    }

    pub fn stop(&mut self) -> bool {
        let digging = self.destroying.is_some();
        self.forget();
        digging
    }

    pub fn crack(&self) -> Option<(Pos, u8)> {
        let (pos, ..) = self.destroying.as_ref()?;
        (self.progress > 0.0).then(|| (*pos, ((self.progress * 10.0) as i32).clamp(0, 9) as u8))
    }

    fn forget(&mut self) {
        self.destroying = None;
        self.progress = 0.0;
        self.ticks = 0;
    }

    fn same_target(&self, pos: Pos, held: Option<&ItemStack>) -> bool {
        self.destroying.as_ref().is_some_and(|(at, _, item)| {
            *at == pos
                && match (item, held) {
                    (Some(item), Some(held)) => item.same_item(held),
                    (None, None) => true,
                    _ => false,
                }
        })
    }

    fn start(
        &mut self,
        pos: Pos,
        face: Face,
        block: &Block,
        input: &DigInput,
        events: &mut Vec<DigEvent>,
    ) -> bool {
        if self.same_target(pos, input.held) {
            return false;
        }
        if let Some((old, ..)) = self.destroying.take() {
            events.push(DigEvent::Aborted(old));
        }
        self.forget();
        events.push(DigEvent::Attacked(pos));
        if (input.progress)(block) >= 1.0 {
            events.push(DigEvent::Broke { pos, instant: true });
            return true;
        }
        self.destroying = Some((pos, face, input.held.cloned()));
        events.push(DigEvent::Started { pos, face });
        false
    }
}

pub fn is_air(block: &Block) -> bool {
    AIR.contains(&block.id.as_str())
}

pub fn digger<'a>(
    inventory: &'a Inventory,
    selected: usize,
    status: &SurvivalStatus,
    eye_in_water: bool,
    on_ground: bool,
) -> Digger<'a> {
    let slot = |index: usize| inventory.slots.get(index).and_then(Option::as_ref);
    let amplifier = |kind| status.effect(kind).map(|effect| effect.amplifier);
    Digger {
        held: slot(selected),
        offhand: slot(crate::offhand::OFFHAND),
        armor: std::array::from_fn(|index| slot(FEET + index)),
        haste: amplifier(EffectKind::Haste),
        conduit: amplifier(EffectKind::ConduitPower),
        fatigue: amplifier(EffectKind::MiningFatigue),
        eye_in_water,
        on_ground,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIRT: f32 = 1.0 / 0.5 / 30.0;

    fn dirt() -> Block {
        Block::new("minecraft:dirt")
    }

    fn input<'a>(
        target: Option<(Pos, Face, &'a Block)>,
        held: Option<&'a ItemStack>,
        clicked: bool,
        progress: &'a dyn Fn(&Block) -> f32,
    ) -> DigInput<'a> {
        DigInput {
            target,
            held,
            clicked,
            held_down: true,
            progress,
        }
    }

    fn broke(tick: &DigTick) -> bool {
        tick.events
            .iter()
            .any(|event| matches!(event, DigEvent::Broke { .. }))
    }

    #[test]
    fn dirt_by_hand_starts_on_the_click_and_breaks_on_the_fifteenth_tick() {
        let block = dirt();
        let pos = (3, 64, -2);
        let speed = |_: &Block| DIRT;
        let mut dig = Dig::default();
        let first = dig.tick(input(Some((pos, Face::Up, &block)), None, true, &speed));
        assert_eq!(
            first.events,
            vec![
                DigEvent::Attacked(pos),
                DigEvent::Started {
                    pos,
                    face: Face::Up
                },
                DigEvent::Hit {
                    pos,
                    face: Face::Up,
                    sound: false
                },
            ]
        );
        assert!(first.swing && !first.stopped);
        assert_eq!(dig.progress, DIRT);
        assert_eq!(dig.crack(), Some((pos, 0)));
        let mut stages = vec![0];
        let mut tick = 1;
        loop {
            tick += 1;
            let step = dig.tick(input(Some((pos, Face::Up, &block)), None, false, &speed));
            if broke(&step) {
                assert_eq!(
                    step.events,
                    vec![DigEvent::Broke {
                        pos,
                        instant: false
                    }]
                );
                break;
            }
            stages.push(dig.crack().unwrap().1);
            assert!(tick < 20);
        }
        assert_eq!(tick, 15);
        assert!(stages.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(stages.first(), Some(&0));
        assert_eq!(stages.last(), Some(&9));
        assert_eq!(dig.crack(), None);
        assert_eq!(dig.delay, DESTROY_DELAY);
    }

    #[test]
    fn holding_on_waits_five_ticks_then_starts_the_next_block_without_progress() {
        let block = dirt();
        let (first, second) = ((0, 64, 0), (0, 63, 0));
        let speed = |_: &Block| DIRT;
        let mut dig = Dig::default();
        let mut broken = Vec::new();
        for tick in 1..=40 {
            let at = if broken.is_empty() { first } else { second };
            let step = dig.tick(input(Some((at, Face::Up, &block)), None, tick == 1, &speed));
            assert!(step.swing);
            if broke(&step) {
                broken.push(tick);
            }
            if (16..=20).contains(&tick) {
                assert!(step.events.is_empty(), "tick {tick}: {:?}", step.events);
            }
            if tick == 21 {
                assert_eq!(
                    step.events,
                    vec![
                        DigEvent::Attacked(second),
                        DigEvent::Started {
                            pos: second,
                            face: Face::Up
                        }
                    ]
                );
                assert_eq!(dig.progress, 0.0);
                assert_eq!(dig.crack(), None);
            }
        }
        assert_eq!(broken, vec![15, 36]);
    }

    #[test]
    fn an_instant_block_breaks_every_tick_with_no_crack_and_no_delay() {
        let block = Block::new("minecraft:netherrack");
        let speed = |_: &Block| 2.8;
        let mut dig = Dig::default();
        for tick in 0..10 {
            let pos = (tick, 70, 0);
            let step = dig.tick(input(
                Some((pos, Face::North, &block)),
                None,
                tick == 0,
                &speed,
            ));
            assert_eq!(
                step.events,
                vec![
                    DigEvent::Attacked(pos),
                    DigEvent::Broke { pos, instant: true }
                ]
            );
            assert!(step.swing && !step.stopped);
            assert_eq!(dig.crack(), None);
            assert_eq!(dig.delay, 0);
            assert!(dig.destroying.is_none());
        }
    }

    #[test]
    fn an_instant_click_ends_the_tick_without_a_dig() {
        let block = Block::new("minecraft:netherrack");
        let speed = |_: &Block| 1.0;
        let mut dig = Dig::default();
        let pos = (1, 2, 3);
        let step = dig.tick(DigInput {
            target: Some((pos, Face::South, &block)),
            held: None,
            clicked: true,
            held_down: false,
            progress: &speed,
        });
        assert!(broke(&step) && step.swing && !step.stopped);
        assert_eq!(dig, Dig::default());
    }

    #[test]
    fn the_hit_sound_plays_on_the_start_and_every_fourth_tick_with_a_particle_each_tick() {
        let block = dirt();
        let pos = (0, 64, 0);
        let speed = |_: &Block| DIRT;
        let mut dig = Dig::default();
        let mut sounds = Vec::new();
        let mut hits = 0;
        for tick in 1..15 {
            let step = dig.tick(input(
                Some((pos, Face::East, &block)),
                None,
                tick == 1,
                &speed,
            ));
            for event in step.events {
                match event {
                    DigEvent::Started { .. } => sounds.push(0),
                    DigEvent::Hit { sound, face, .. } => {
                        assert_eq!(face, Face::East);
                        hits += 1;
                        if sound {
                            sounds.push(tick);
                        }
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(sounds, vec![0, 4, 8, 12]);
        assert_eq!(hits, 14);
    }

    #[test]
    fn letting_go_forgets_the_crack_and_a_new_click_starts_from_nothing() {
        let block = dirt();
        let pos = (0, 64, 0);
        let speed = |_: &Block| DIRT;
        let mut dig = Dig::default();
        for tick in 0..7 {
            dig.tick(input(
                Some((pos, Face::Up, &block)),
                None,
                tick == 0,
                &speed,
            ));
        }
        assert_eq!(dig.crack().map(|(_, stage)| stage), Some(4));
        let released = dig.tick(DigInput {
            target: Some((pos, Face::Up, &block)),
            held: None,
            clicked: false,
            held_down: false,
            progress: &speed,
        });
        assert!(released.stopped && released.events.is_empty() && !released.swing);
        assert_eq!(dig.crack(), None);
        assert!(!dig.stop());
        let again = dig.tick(input(Some((pos, Face::Up, &block)), None, true, &speed));
        assert_eq!(again.events[0], DigEvent::Attacked(pos));
        assert_eq!(dig.progress, DIRT);
        assert!(dig.stop());
        assert_eq!(dig.progress, 0.0);
    }

    #[test]
    fn another_item_restarts_the_dig_but_another_count_does_not() {
        let block = dirt();
        let pos = (0, 64, 0);
        let speed = |_: &Block| 0.01;
        let pick = ItemStack::new("minecraft:iron_pickaxe", 1);
        let mut worn = pick.clone();
        worn.components = Some(serde_json::json!({"minecraft:damage": 3}));
        let mut more = pick.clone();
        more.count = 2;
        let mut dig = Dig::default();
        dig.tick(input(
            Some((pos, Face::Up, &block)),
            Some(&pick),
            true,
            &speed,
        ));
        let counted = dig.tick(input(
            Some((pos, Face::Up, &block)),
            Some(&more),
            false,
            &speed,
        ));
        assert!(matches!(counted.events[..], [DigEvent::Hit { .. }]));
        assert!((dig.progress - 0.02).abs() < 1e-6);
        let changed = dig.tick(input(
            Some((pos, Face::Up, &block)),
            Some(&worn),
            false,
            &speed,
        ));
        assert_eq!(
            changed.events,
            vec![
                DigEvent::Aborted(pos),
                DigEvent::Attacked(pos),
                DigEvent::Started {
                    pos,
                    face: Face::Up
                }
            ]
        );
        assert_eq!(dig.progress, 0.0);
        let emptied = dig.tick(input(Some((pos, Face::Up, &block)), None, false, &speed));
        assert_eq!(emptied.events[0], DigEvent::Aborted(pos));
    }

    #[test]
    fn a_block_gone_to_air_ends_the_dig_without_a_break() {
        let block = dirt();
        let air = Block::new("minecraft:air");
        let pos = (0, 64, 0);
        let speed = |_: &Block| DIRT;
        let mut dig = Dig::default();
        dig.tick(input(Some((pos, Face::Up, &block)), None, true, &speed));
        let gone = dig.tick(input(Some((pos, Face::Up, &air)), None, false, &speed));
        assert!(gone.events.is_empty() && !gone.swing && !gone.stopped);
        assert!(dig.destroying.is_none());
        assert_eq!(dig.crack(), None);
        let lost = dig.tick(input(None, None, false, &speed));
        assert!(!lost.stopped && lost.events.is_empty());
    }

    #[test]
    fn losing_the_target_while_digging_stops_like_letting_go() {
        let block = dirt();
        let speed = |_: &Block| DIRT;
        let mut dig = Dig::default();
        dig.tick(input(
            Some(((0, 64, 0), Face::Up, &block)),
            None,
            true,
            &speed,
        ));
        let lost = dig.tick(input(None, None, false, &speed));
        assert!(lost.stopped && !lost.swing);
        assert!(dig.destroying.is_none());
    }

    #[test]
    fn a_moved_target_aborts_and_starts_again_with_an_attack() {
        let block = dirt();
        let (first, second) = ((0, 64, 0), (1, 64, 0));
        let speed = |_: &Block| DIRT;
        let mut dig = Dig::default();
        dig.tick(input(Some((first, Face::Up, &block)), None, true, &speed));
        dig.tick(input(Some((first, Face::Up, &block)), None, false, &speed));
        let moved = dig.tick(input(
            Some((second, Face::West, &block)),
            None,
            false,
            &speed,
        ));
        assert_eq!(
            moved.events,
            vec![
                DigEvent::Aborted(first),
                DigEvent::Attacked(second),
                DigEvent::Started {
                    pos: second,
                    face: Face::West
                }
            ]
        );
        assert!(moved.swing && !moved.stopped);
        assert_eq!(dig.progress, 0.0);
        assert_eq!(dig.crack(), None);
    }

    #[test]
    fn a_press_waits_for_the_next_tick() {
        let mut dig = Dig::default();
        assert!(!dig.take_press());
        dig.press();
        assert!(dig.take_press());
        assert!(!dig.take_press());
    }

    #[test]
    fn the_digger_reads_the_hands_the_armour_and_the_effects() {
        let mut inventory = Inventory::default();
        inventory.slots[2] = Some(ItemStack::new("minecraft:diamond_pickaxe", 1));
        inventory.slots[crate::offhand::OFFHAND] = Some(ItemStack::new("minecraft:shield", 1));
        inventory.slots[HEAD] = Some(ItemStack::new("minecraft:diamond_helmet", 1));
        inventory.slots[FEET] = Some(ItemStack::new("minecraft:iron_boots", 1));
        let mut status = SurvivalStatus::default();
        status.add_effect(EffectKind::Haste, 100, 1);
        status.add_effect(EffectKind::MiningFatigue, 100, 2);
        let digger = digger(&inventory, 2, &status, true, false);
        assert_eq!(
            digger.held.map(|stack| stack.id.as_str()),
            Some("minecraft:diamond_pickaxe")
        );
        assert_eq!(
            digger.offhand.map(|stack| stack.id.as_str()),
            Some("minecraft:shield")
        );
        assert_eq!(
            digger.armor[3].map(|stack| stack.id.as_str()),
            Some("minecraft:diamond_helmet")
        );
        assert_eq!(
            digger.armor[0].map(|stack| stack.id.as_str()),
            Some("minecraft:iron_boots")
        );
        assert!(digger.armor[1].is_none() && digger.armor[2].is_none());
        assert_eq!(
            (digger.haste, digger.conduit, digger.fatigue),
            (Some(1), None, Some(2))
        );
        assert!(digger.eye_in_water && !digger.on_ground);
        assert!(
            super::digger(&inventory, 5, &status, false, true)
                .held
                .is_none()
        );
    }
}
