use super::Level;
use super::container::Stack;
use super::entity::{Entity, EntityKind, aabb_clip};
use super::physics::Aabb;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::RandomSource;
use minecraftoss_core::{BlockPos, BlockStateId};
use serde_json::Value;

pub const FIRE_RESISTANT_ITEMS: [&str; 16] = [
    "minecraft:ancient_debris",
    "minecraft:netherite_block",
    "minecraft:netherite_ingot",
    "minecraft:netherite_scrap",
    "minecraft:netherite_sword",
    "minecraft:netherite_shovel",
    "minecraft:netherite_pickaxe",
    "minecraft:netherite_axe",
    "minecraft:netherite_hoe",
    "minecraft:netherite_helmet",
    "minecraft:netherite_chestplate",
    "minecraft:netherite_leggings",
    "minecraft:netherite_boots",
    "minecraft:netherite_horse_armor",
    "minecraft:netherite_spear",
    "minecraft:netherite_nautilus_armor",
];

const IS_FIRE_DAMAGE_TYPES: [&str; 8] = [
    "minecraft:in_fire",
    "minecraft:campfire",
    "minecraft:on_fire",
    "minecraft:lava",
    "minecraft:hot_floor",
    "minecraft:sulfur_cube_hot",
    "minecraft:unattributed_fireball",
    "minecraft:fireball",
];

const DAMAGE_RESISTANT: &str = "minecraft:damage_resistant";
const REMOVED_DAMAGE_RESISTANT: &str = "!minecraft:damage_resistant";
const IS_FIRE_TAG: &str = "#minecraft:is_fire";
const FIRE_IGNITE_TICKS: i32 = 160;
const LAVA_IGNITE_TICKS: i32 = 300;
const LAVA_DAMAGE: f32 = 4.0;
const ON_FIRE_DAMAGE: f32 = 1.0;
pub const BURN_SOUND: &str = "minecraft:entity.generic.burn";
pub const EXTINGUISH_SOUND: &str = "minecraft:entity.generic.extinguish_fire";

static EXTRA_FIRE_RESISTANT: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

pub fn add_fire_resistant_items(ids: Vec<String>) {
    let _ = EXTRA_FIRE_RESISTANT.set(ids);
}

fn extra_fire_resistant(id: &str) -> bool {
    EXTRA_FIRE_RESISTANT
        .get()
        .is_some_and(|ids| ids.iter().any(|extra| extra == id))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireDamage {
    InFire,
    OnFire,
    Lava,
}

impl FireDamage {
    pub fn damage_type(self) -> &'static str {
        match self {
            Self::InFire => "minecraft:in_fire",
            Self::OnFire => "minecraft:on_fire",
            Self::Lava => "minecraft:lava",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LevelSound {
    pub event: &'static str,
    pub pos: [f64; 3],
    pub volume: f32,
    pub pitch: f32,
}

fn qualified(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}

fn tag_types(types: Option<&Tag>) -> Vec<String> {
    match types {
        Some(Tag::String(single)) => vec![single.clone()],
        Some(Tag::List(list)) => list
            .iter()
            .filter_map(Tag::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn json_types(types: Option<&Value>) -> Vec<String> {
    match types {
        Some(Value::String(single)) => vec![single.clone()],
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn resistant_types(stack: &Stack) -> Option<Vec<String>> {
    let patched = match &stack.components {
        Some(Tag::Compound(map)) => {
            if map.contains_key(REMOVED_DAMAGE_RESISTANT) {
                return None;
            }
            map.get(DAMAGE_RESISTANT)
                .map(|component| tag_types(component.get("types")))
        }
        Some(Tag::String(json)) => {
            let value: Value = serde_json::from_str(json).unwrap_or(Value::Null);
            if value.get(REMOVED_DAMAGE_RESISTANT).is_some() {
                return None;
            }
            value
                .get(DAMAGE_RESISTANT)
                .map(|component| json_types(component.get("types")))
        }
        _ => None,
    };
    patched.or_else(|| {
        (FIRE_RESISTANT_ITEMS.contains(&stack.id.as_str()) || extra_fire_resistant(&stack.id))
            .then(|| vec![IS_FIRE_TAG.to_owned()])
    })
}

pub fn resists(stack: &Stack, damage_type: &str) -> bool {
    resistant_types(stack).is_some_and(|types| {
        types.iter().any(|t| match t.strip_prefix('#') {
            Some(tag) => {
                format!("#{}", qualified(tag)) == IS_FIRE_TAG
                    && IS_FIRE_DAMAGE_TYPES.contains(&damage_type)
            }
            None => qualified(t) == damage_type,
        })
    })
}

pub fn fire_immune(stack: &Stack) -> bool {
    resists(stack, FireDamage::InFire.damage_type())
}

fn item_fire_immune(e: &Entity) -> bool {
    e.item_data().is_some_and(|data| fire_immune(&data.stack))
}

pub fn is_on_fire(e: &Entity) -> bool {
    e.remaining_fire_ticks > 0 && !item_fire_immune(e)
}

pub fn hurt_item(e: &mut Entity, damage: FireDamage, amount: f32) -> bool {
    if e.removed {
        return false;
    }
    let EntityKind::Item(data) = &mut e.kind else {
        return false;
    };
    if fire_immune(&data.stack) || resists(&data.stack, damage.damage_type()) {
        return false;
    }
    data.health = (data.health as f32 - amount) as i32;
    if data.health <= 0 {
        e.removed = true;
    }
    true
}

pub fn item_fire_tick(e: &mut Entity) {
    if e.remaining_fire_ticks <= 0 {
        return;
    }
    if item_fire_immune(e) {
        e.remaining_fire_ticks = 0;
        return;
    }
    let in_lava = !e.first_tick && e.lava_height > 0.0;
    if e.remaining_fire_ticks % 20 == 0 && !in_lava {
        hurt_item(e, FireDamage::OnFire, ON_FIRE_DAMAGE);
    }
    e.remaining_fire_ticks -= 1;
}

fn ignite_for_ticks(e: &mut Entity, ticks: i32) {
    if e.remaining_fire_ticks < ticks {
        e.remaining_fire_ticks = ticks;
    }
}

fn fire_ignite(e: &mut Entity) {
    if item_fire_immune(e) {
        return;
    }
    if e.remaining_fire_ticks < 0 {
        e.remaining_fire_ticks += 1;
    }
    if e.remaining_fire_ticks >= 0 {
        ignite_for_ticks(e, FIRE_IGNITE_TICKS);
    }
}

fn lava_hurt_sound_plays(e: &Entity) -> bool {
    e.item_data().is_some_and(|data| data.health <= 0) || e.tick_count % 10 == 0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsideEffect {
    ClearFreeze,
    FireIgnite,
    LavaIgnite,
    Extinguish,
}

const APPLY_ORDER: [InsideEffect; 4] = [
    InsideEffect::ClearFreeze,
    InsideEffect::FireIgnite,
    InsideEffect::LavaIgnite,
    InsideEffect::Extinguish,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockAction {
    Effect(InsideEffect),
    FireHurt(f32),
    LavaHurt,
}

#[derive(Clone, Debug, Default)]
pub struct EffectCollector {
    in_step: Vec<InsideEffect>,
    after: Vec<(InsideEffect, BlockAction)>,
    last_step: Option<i32>,
    out: Vec<BlockAction>,
}

impl EffectCollector {
    pub fn advance_step(&mut self, step: i32) {
        if self.last_step != Some(step) {
            self.last_step = Some(step);
            self.flush();
        }
    }

    pub fn apply(&mut self, effect: InsideEffect) {
        if !self.in_step.contains(&effect) {
            self.in_step.push(effect);
        }
    }

    pub fn run_after(&mut self, effect: InsideEffect, action: BlockAction) {
        self.after.push((effect, action));
    }

    pub fn fire(&mut self, damage: f32) {
        self.apply(InsideEffect::ClearFreeze);
        self.apply(InsideEffect::FireIgnite);
        self.run_after(InsideEffect::FireIgnite, BlockAction::FireHurt(damage));
    }

    pub fn lava(&mut self) {
        self.apply(InsideEffect::ClearFreeze);
        self.apply(InsideEffect::LavaIgnite);
        self.run_after(InsideEffect::LavaIgnite, BlockAction::LavaHurt);
    }

    pub fn extinguish(&mut self) {
        self.apply(InsideEffect::Extinguish);
    }

    fn flush(&mut self) {
        for effect in APPLY_ORDER {
            if let Some(i) = self.in_step.iter().position(|&e| e == effect) {
                self.in_step.remove(i);
                self.out.push(BlockAction::Effect(effect));
            }
            let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.after)
                .into_iter()
                .partition(|(e, _)| *e == effect);
            self.out.extend(now.into_iter().map(|(_, action)| action));
            self.after = later;
        }
    }

    pub fn finish(&mut self) -> Vec<BlockAction> {
        self.flush();
        self.last_step = None;
        std::mem::take(&mut self.out)
    }
}

pub fn apply_block_actions(e: &mut Entity, actions: &[BlockAction]) -> usize {
    let mut sizzles = 0;
    for &action in actions {
        if e.removed {
            break;
        }
        match action {
            BlockAction::Effect(InsideEffect::ClearFreeze) => {}
            BlockAction::Effect(InsideEffect::FireIgnite) => fire_ignite(e),
            BlockAction::Effect(InsideEffect::LavaIgnite) => {
                if !item_fire_immune(e) {
                    ignite_for_ticks(e, LAVA_IGNITE_TICKS);
                }
            }
            BlockAction::Effect(InsideEffect::Extinguish) => {
                e.remaining_fire_ticks = e.remaining_fire_ticks.min(0)
            }
            BlockAction::FireHurt(damage) => {
                hurt_item(e, FireDamage::InFire, damage);
            }
            BlockAction::LavaHurt => {
                if !item_fire_immune(e)
                    && hurt_item(e, FireDamage::Lava, LAVA_DAMAGE)
                    && lava_hurt_sound_plays(e)
                {
                    sizzles += 1;
                }
            }
        }
    }
    sizzles
}

pub fn settle_fire(e: &mut Entity, was_on_fire: bool, fire_before: i32) -> bool {
    let on_fire = is_on_fire(e);
    if !on_fire && e.remaining_fire_ticks <= fire_before {
        e.remaining_fire_ticks = 0;
    }
    was_on_fire && !on_fire
}

pub fn collided_along_vector(bb: &Aabb, travel: [f64; 3], boxes: &[Aabb]) -> bool {
    let from: [f64; 3] = std::array::from_fn(|i| bb.min[i] + 0.5 * (bb.max[i] - bb.min[i]));
    let to: [f64; 3] = std::array::from_fn(|i| from[i] + travel[i]);
    let grow: [f64; 3] = std::array::from_fn(|i| (bb.max[i] - bb.min[i]) * 0.5 - 1.0e-7);
    boxes.iter().any(|b| {
        let grown = Aabb::new(
            b.min[0] - grow[0],
            b.min[1] - grow[1],
            b.min[2] - grow[2],
            b.max[0] + grow[0],
            b.max[1] + grow[1],
            b.max[2] + grow[2],
        );
        let contains = |p: [f64; 3]| (0..3).all(|i| p[i] >= grown.min[i] && p[i] < grown.max[i]);
        contains(to) || contains(from) || aabb_clip(grown.min, grown.max, from, to).is_some()
    })
}

impl Level<'_> {
    pub(super) fn entity_inside_boxes(
        &self,
        state: BlockStateId,
        pos: BlockPos,
    ) -> Option<Vec<Aabb>> {
        if !self.is_a(state, "LavaCauldronBlock") {
            return None;
        }
        let (x, y, z) = (f64::from(pos.x), f64::from(pos.y), f64::from(pos.z));
        let mut boxes = vec![Aabb::new(
            x + 2.0 / 16.0,
            y + 4.0 / 16.0,
            z + 2.0 / 16.0,
            x + 14.0 / 16.0,
            y + 15.0 / 16.0,
            z + 14.0 / 16.0,
        )];
        match self.registries().blocks.collision_shape(state) {
            Some(minecraftoss_core::block::FaceShape::Full) => {
                boxes.push(Aabb::new(x, y, z, x + 1.0, y + 1.0, z + 1.0))
            }
            Some(minecraftoss_core::block::FaceShape::Boxes(shape)) => boxes.extend(
                shape
                    .iter()
                    .map(|b| Aabb::new(x + b[0], y + b[1], z + b[2], x + b[3], y + b[4], z + b[5])),
            ),
            _ => {}
        }
        Some(boxes)
    }

    pub(super) fn item_burn_effects(
        &mut self,
        e: &mut Entity,
        actions: &[BlockAction],
        was_on_fire: bool,
        fire_before: i32,
    ) {
        for _ in 0..apply_block_actions(e, actions) {
            let pitch = 2.0 + self.entity_random.next_f32() * 0.4;
            self.sounds.push(LevelSound {
                event: BURN_SOUND,
                pos: e.pos,
                volume: 0.4,
                pitch,
            });
        }
        if e.remaining_fire_ticks > 0 && self.entity_in_rain(e) {
            e.remaining_fire_ticks = 0;
        }
        if settle_fire(e, was_on_fire, fire_before) {
            let first = self.entity_random.next_f32();
            let second = self.entity_random.next_f32();
            let pitch = 1.6 + (first - second) * 0.4;
            self.sounds.push(LevelSound {
                event: EXTINGUISH_SOUND,
                pos: e.pos,
                volume: 0.7,
                pitch,
            });
        }
    }

    fn entity_in_rain(&mut self, e: &Entity) -> bool {
        let (x, z) = (e.pos[0].floor() as i32, e.pos[2].floor() as i32);
        self.is_raining_at(BlockPos::new(x, e.pos[1].floor() as i32, z))
            || self.is_raining_at(BlockPos::new(x, e.bb.max[1].floor() as i32, z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn item(id: &str) -> Entity {
        let mut e = Entity::item(1, [0.5, 0.0, 0.5], Stack::new(id, 1), [0.0; 3]);
        e.first_tick = false;
        e
    }

    fn health(e: &Entity) -> i32 {
        e.item_data().map_or(i32::MIN, |data| data.health)
    }

    fn set_health(e: &mut Entity, health: i32) {
        if let EntityKind::Item(data) = &mut e.kind {
            data.health = health;
        }
    }

    fn tick(e: &mut Entity, inside: impl Fn(&mut EffectCollector)) -> usize {
        e.tick_count += 1;
        item_fire_tick(e);
        let mut collector = EffectCollector::default();
        collector.advance_step(0);
        inside(&mut collector);
        let actions = collector.finish();
        let (was_on_fire, before) = (is_on_fire(e), e.remaining_fire_ticks);
        let sizzles = apply_block_actions(e, &actions);
        settle_fire(e, was_on_fire, before);
        sizzles
    }

    fn compound(entries: Vec<(&str, Tag)>) -> Tag {
        Tag::Compound(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn netherite_and_ancient_debris_resist_fire_and_lava() {
        for id in [
            "minecraft:netherite_ingot",
            "minecraft:ancient_debris",
            "minecraft:netherite_sword",
            "minecraft:netherite_block",
        ] {
            let stack = Stack::new(id, 1);
            assert!(fire_immune(&stack), "{id}");
            assert!(resists(&stack, "minecraft:lava"), "{id}");
            assert!(resists(&stack, "minecraft:on_fire"), "{id}");
            assert!(!resists(&stack, "minecraft:explosion"), "{id}");
        }
        for id in [
            "minecraft:netherite_upgrade_smithing_template",
            "minecraft:cobblestone",
            "minecraft:diamond",
            "minecraft:nether_star",
        ] {
            assert!(!fire_immune(&Stack::new(id, 1)), "{id}");
        }
    }

    #[test]
    fn added_items_resist_fire_like_netherite() {
        add_fire_resistant_items(vec!["test:fire_proof".to_owned()]);
        let stack = Stack::new("test:fire_proof", 1);
        assert!(fire_immune(&stack));
        assert!(resists(&stack, "minecraft:lava"));
        assert!(!resists(&stack, "minecraft:cactus"));
        assert!(!fire_immune(&Stack::new("test:plain", 1)));
    }

    #[test]
    fn the_damage_resistant_component_decides_over_the_item() {
        let mut cobblestone = Stack::new("minecraft:cobblestone", 1);
        cobblestone.components = Some(compound(vec![(
            DAMAGE_RESISTANT,
            compound(vec![(
                "types",
                Tag::String("#minecraft:is_fire".to_owned()),
            )]),
        )]));
        assert!(fire_immune(&cobblestone));
        let mut lava_only = Stack::new("minecraft:cobblestone", 1);
        lava_only.components = Some(Tag::String(
            r#"{"minecraft:damage_resistant":{"types":"minecraft:lava"}}"#.to_owned(),
        ));
        assert!(!fire_immune(&lava_only));
        assert!(resists(&lava_only, "minecraft:lava"));
        let mut json_fire = Stack::new("minecraft:dirt", 1);
        json_fire.components = Some(Tag::String(
            r##"{"minecraft:damage_resistant":{"types":"#minecraft:is_fire"}}"##.to_owned(),
        ));
        assert!(fire_immune(&json_fire));
        let mut stripped = Stack::new("minecraft:netherite_ingot", 1);
        stripped.components = Some(compound(vec![(
            REMOVED_DAMAGE_RESISTANT,
            compound(Vec::new()),
        )]));
        assert!(!fire_immune(&stripped));
        let mut renamed = Stack::new("minecraft:netherite_ingot", 1);
        renamed.components = Some(Tag::String(r#"{"minecraft:custom_name":"Bar"}"#.to_owned()));
        assert!(fire_immune(&renamed));
    }

    #[test]
    fn the_collector_flushes_each_step_in_effect_order() {
        use BlockAction::{Effect, FireHurt, LavaHurt};
        use InsideEffect::{ClearFreeze, Extinguish, FireIgnite, LavaIgnite};
        let mut collector = EffectCollector::default();
        collector.advance_step(0);
        collector.extinguish();
        collector.lava();
        collector.fire(2.0);
        collector.fire(1.0);
        collector.advance_step(1);
        collector.fire(1.0);
        let actions = collector.finish();
        assert_eq!(
            actions,
            [
                Effect(ClearFreeze),
                Effect(FireIgnite),
                FireHurt(2.0),
                FireHurt(1.0),
                Effect(LavaIgnite),
                LavaHurt,
                Effect(Extinguish),
                Effect(ClearFreeze),
                Effect(FireIgnite),
                FireHurt(1.0),
            ]
        );
        assert!(collector.finish().is_empty());
    }

    #[test]
    fn fire_burns_an_item_in_three_ticks_without_a_sound() {
        let mut e = item("minecraft:cobblestone");
        let mut sizzles = 0;
        let mut ticks = 0;
        while !e.removed && ticks < 20 {
            sizzles += tick(&mut e, |c| c.fire(1.0));
            ticks += 1;
        }
        assert_eq!((ticks, sizzles), (3, 0));
    }

    #[test]
    fn soul_fire_burns_an_item_in_two_ticks() {
        let mut e = item("minecraft:rotten_flesh");
        let mut ticks = 0;
        while !e.removed && ticks < 20 {
            tick(&mut e, |c| c.fire(2.0));
            ticks += 1;
        }
        assert_eq!(ticks, 2);
    }

    #[test]
    fn lava_burns_an_item_in_two_ticks_with_one_sizzle() {
        let mut e = item("minecraft:diamond");
        e.lava_height = 0.5;
        let first = tick(&mut e, |c| c.lava());
        assert_eq!(
            (health(&e), e.remaining_fire_ticks, first),
            (1, LAVA_IGNITE_TICKS, 0)
        );
        let second = tick(&mut e, |c| c.lava());
        assert!(e.removed);
        assert_eq!(second, 1);
        let mut on_the_tenth = item("minecraft:diamond");
        on_the_tenth.lava_height = 0.5;
        on_the_tenth.tick_count = 9;
        assert_eq!(tick(&mut on_the_tenth, |c| c.lava()), 1);
    }

    #[test]
    fn netherite_neither_burns_nor_catches_fire() {
        for id in ["minecraft:netherite_ingot", "minecraft:ancient_debris"] {
            let mut e = item(id);
            e.lava_height = 0.5;
            let mut sizzles = 0;
            for _ in 0..200 {
                sizzles += tick(&mut e, |c| {
                    c.lava();
                    c.fire(2.0);
                });
            }
            assert!(!e.removed, "{id}");
            assert_eq!(
                (health(&e), e.remaining_fire_ticks, sizzles),
                (5, 0, 0),
                "{id}"
            );
            assert!(!hurt_item(&mut e, FireDamage::InFire, 10.0));
            e.remaining_fire_ticks = 40;
            item_fire_tick(&mut e);
            assert_eq!(e.remaining_fire_ticks, 0);
        }
    }

    #[test]
    fn an_item_that_left_the_fire_keeps_burning_for_eight_seconds() {
        let mut e = item("minecraft:oak_log");
        set_health(&mut e, 100);
        tick(&mut e, |c| c.fire(1.0));
        assert_eq!(
            (health(&e), e.remaining_fire_ticks),
            (99, FIRE_IGNITE_TICKS)
        );
        let mut burning = 0;
        while is_on_fire(&e) {
            tick(&mut e, |_| {});
            burning += 1;
        }
        assert_eq!((burning, health(&e)), (160, 91));
        tick(&mut e, |_| {});
        assert_eq!(e.remaining_fire_ticks, 0);
    }

    #[test]
    fn a_burning_item_dies_three_seconds_after_leaving_the_fire() {
        let mut e = item("minecraft:stick");
        tick(&mut e, |c| c.fire(1.0));
        let mut ticks = 0;
        while !e.removed && ticks < 200 {
            tick(&mut e, |_| {});
            ticks += 1;
        }
        assert_eq!(ticks, 61);
    }

    #[test]
    fn water_puts_a_burning_item_out() {
        let mut e = item("minecraft:stick");
        set_health(&mut e, 100);
        tick(&mut e, |c| c.fire(1.0));
        let (was_on_fire, before) = (is_on_fire(&e), e.remaining_fire_ticks);
        let mut collector = EffectCollector::default();
        collector.advance_step(0);
        collector.extinguish();
        apply_block_actions(&mut e, &collector.finish());
        assert!(settle_fire(&mut e, was_on_fire, before));
        assert_eq!(e.remaining_fire_ticks, 0);
        assert!(!settle_fire(&mut e, false, 0));
    }

    #[test]
    fn on_fire_damage_skips_items_in_lava() {
        let mut e = item("minecraft:stick");
        e.remaining_fire_ticks = 20;
        e.lava_height = 0.3;
        item_fire_tick(&mut e);
        assert_eq!((health(&e), e.remaining_fire_ticks), (5, 19));
        e.remaining_fire_ticks = 20;
        e.lava_height = 0.0;
        item_fire_tick(&mut e);
        assert_eq!(health(&e), 4);
    }

    #[test]
    fn a_falling_box_collides_with_the_lava_surface_only_when_it_reaches_it() {
        let lava = [Aabb::new(0.0, 0.0, 0.0, 1.0, 8.0 / 9.0, 1.0)];
        let falling = Aabb::for_entity(0.5, 1.2, 0.5, 0.25, 0.25);
        assert!(collided_along_vector(&falling, [0.0, -0.5, 0.0], &lava));
        assert!(!collided_along_vector(&falling, [0.0, -0.2, 0.0], &lava));
        let resting = Aabb::for_entity(0.5, 0.95, 0.5, 0.25, 0.25);
        assert!(!collided_along_vector(&resting, [0.0; 3], &lava));
        let floating = Aabb::for_entity(0.5, 0.8, 0.5, 0.25, 0.25);
        assert!(collided_along_vector(&floating, [0.0; 3], &lava));
    }
}
