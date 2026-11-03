use crate::client_mobs::ClientMobs;
use crate::families::{ClientFx, FamilyDraw, FamilyShow, TerrainFamily};
use crate::item_nbt::stack_tag;
use crate::keyframes;
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use crate::piglin_save::stack_of;
use glam::{DVec3, Vec3};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::age::Age;
use minecraftoss_entities::enderman::view_vector;
use minecraftoss_entities::gossip::{uuid_from_ints, uuid_to_ints};
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::land::foxes::Fox;
use minecraftoss_entities::world::families::land::foxes::{
    self, CROUCHING, FoxVariant, SITTING, SLEEPING,
};
use minecraftoss_player::mth;
use minecraftoss_player::rng::LegacyRandom;
use std::collections::BTreeMap;

mod render;

pub const SHADOW_RADIUS: f32 = 0.4;
pub const EAT_VALUE: usize = 0;
pub const CRUMBS: usize = 8;
pub const CRUMB_BURSTS: i32 = 4;
const DEGREES: f32 = 0.017453292;
const MAINHAND: &str = "mainhand";
const SAVED_FLAGS: [(&str, u8); 3] = [
    ("Sleeping", SLEEPING),
    ("Sitting", SITTING),
    ("Crouching", CROUCHING),
];

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|value| value as i32)
}

fn uuid_of(tag: &Tag) -> Option<u128> {
    let ints = tag.as_ints()?;
    <[i32; 4]>::try_from(ints).ok().map(uuid_from_ints)
}

fn uuid_tag(uuid: u128) -> Tag {
    Tag::IntArray(uuid_to_ints(uuid).to_vec())
}

fn x_rot(v: DVec3, angle: f32) -> DVec3 {
    let (cos, sin) = (
        f64::from(mth::cos(f64::from(angle))),
        f64::from(mth::sin(f64::from(angle))),
    );
    DVec3::new(v.x, v.y * cos + v.z * sin, v.z * cos - v.y * sin)
}

fn y_rot(v: DVec3, angle: f32) -> DVec3 {
    let (cos, sin) = (
        f64::from(mth::cos(f64::from(angle))),
        f64::from(mth::sin(f64::from(angle))),
    );
    DVec3::new(v.x * cos + v.z * sin, v.y, v.z * cos - v.x * sin)
}

pub fn eating_crumbs(
    item: &str,
    position: DVec3,
    (pitch, yaw): (f32, f32),
    random: &mut LegacyRandom,
) -> (String, DVec3, Vec<Vec3>) {
    let look = view_vector(pitch, yaw);
    let at = DVec3::new(position.x + look.x / 2.0, position.y, position.z + look.z / 2.0);
    let velocities = (0..CRUMBS)
        .map(|_| {
            let x = (f64::from(random.next_float()) - 0.5) * 0.1;
            let y = f64::from(random.next_float()) * 0.1 + 0.1;
            let pitched = x_rot(DVec3::new(x, y, 0.0), -pitch * DEGREES);
            let v = y_rot(pitched, -yaw * DEGREES);
            Vec3::new(v.x as f32, (v.y + 0.05) as f32, v.z as f32)
        })
        .collect();
    (item.to_owned(), at, velocities)
}

impl TerrainFamily for Fox {
    const TRACKING_RANGE: i32 = 8;
    const VOLATILE: &'static [&'static str] = &["Sleeping", "Sitting", "Crouching", "ForcedAge"];
    const TEXTURES: &'static [&'static str] = &render::TEXTURES;
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;
    const CAGE_MARGIN: f32 = 0.21;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        let yaw = mob.core.yaw;
        let ai = &mut mob.state.ai;
        ai.yaw = yaw;
        ai.body_rotation.body_yaw = yaw;
        ai.state.look_control.head_yaw = yaw;
        let state = &mut mob.state;
        let variant = tag.get("Type").and_then(Tag::as_str).unwrap_or("");
        state.variant = FoxVariant::from_name(variant);
        state.trusted.clear();
        let trusted = tag.get("Trusted").and_then(Tag::as_list).unwrap_or(&[]);
        for uuid in trusted.iter().filter_map(uuid_of) {
            state.trust(uuid);
        }
        for (key, flag) in SAVED_FLAGS {
            let set = int(tag, key).is_some_and(|value| value != 0);
            state.set_flag(flag, set);
        }
        let shown_baby = int(tag, "Baby").is_some_and(|baby| baby != 0);
        let ticks = int(tag, "Age").unwrap_or(if shown_baby { Age::BABY_START } else { 0 });
        state.age = Age {
            ticks,
            forced: int(tag, "ForcedAge").unwrap_or(0),
            locked: int(tag, "AgeLocked").is_some_and(|locked| locked != 0),
            ..Age::default()
        };
        state.in_love = int(tag, "InLove").unwrap_or(0);
        state.love_cause = tag.get("LoveCause").and_then(uuid_of);
        state.can_pick_up_loot = int(tag, "CanPickUpLoot").is_none_or(|can| can != 0);
        state.held = tag
            .get("equipment")
            .and_then(|equipment| equipment.get(MAINHAND))
            .and_then(stack_of);
        state.drop_chance = tag
            .get("drop_chances")
            .and_then(|chances| chances.get(MAINHAND))
            .and_then(Tag::as_f64)
            .map_or(foxes::DROP_CHANCE, |chance| chance as f32);
        let baby = mob.state.age.baby();
        foxes::set_baby(mob, baby);
        mob.state.set_target_goals();
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        let state = &mob.state;
        let trusted = state.trusted.iter().map(|&uuid| uuid_tag(uuid)).collect();
        let mut equipment = BTreeMap::new();
        if let Some(stack) = state.held.as_ref().and_then(stack_tag) {
            equipment.insert(MAINHAND.to_owned(), stack);
        }
        let mut chances = BTreeMap::new();
        if state.drop_chance != foxes::DROP_CHANCE {
            chances.insert(MAINHAND.to_owned(), Tag::Float(state.drop_chance));
        }
        let mut fields = vec![
            ("Type", Tag::String(state.variant.name().to_owned())),
            ("Trusted", Tag::List(trusted)),
        ];
        for (key, flag) in SAVED_FLAGS {
            fields.push((key, Tag::Byte(i8::from(state.flag(flag)))));
        }
        fields.extend([
            ("Age", Tag::Int(state.age.ticks)),
            ("ForcedAge", Tag::Int(state.age.forced)),
            ("AgeLocked", Tag::Byte(i8::from(state.age.locked))),
            ("InLove", Tag::Int(state.in_love)),
            ("CanPickUpLoot", Tag::Byte(i8::from(state.can_pick_up_loot))),
            ("equipment", Tag::Compound(equipment)),
            ("drop_chances", Tag::Compound(chances)),
        ]);
        if let Some(uuid) = state.love_cause {
            fields.push(("LoveCause", uuid_tag(uuid)));
        }
        fields
    }

    fn show(mob: &FamilyMob<Self>) -> FamilyShow {
        let mut show = FamilyShow {
            flags: u32::from(mob.state.flags()),
            ..FamilyShow::default()
        };
        show.values[EAT_VALUE] = mob.state.eat_events.unwrap_or(0) as i32;
        show
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        mob.state.goals_mut().flags = show.flags as u8;
        mob.state.eat_events = Some(show.values[EAT_VALUE] as u32);
        mob.state.step_angles();
    }

    fn draw(
        out: &mut FamilyDraw<'_>,
        mobs: &[FamilyMob<Self>],
        poses: &ClientMobs,
        atlas: &Atlas,
        light: &SkyLight,
        partial: f32,
    ) {
        let walk = render::baby_walk(keyframes::shared());
        let shown = mobs.iter().filter(|mob| !mob.core.removed);
        render::append_foxes(out, shown, poses, atlas, light, partial, walk.as_deref());
    }

    fn client_effects(mobs: &[FamilyMob<Self>], poses: &ClientMobs, fx: &mut ClientFx<'_>) {
        for mob in mobs.iter().filter(|mob| !mob.core.removed) {
            let Some(events) = mob.state.eat_events else {
                continue;
            };
            let last = fx.seen.insert(mob.core.id, events as i32);
            let bursts = last.map_or(0, |last| (events as i32).wrapping_sub(last));
            let held = mob.state.held.as_ref();
            let (Some(held), Some(shown)) = (held, poses.get(mob.core.id)) else {
                continue;
            };
            let turns = (shown.x_rot, shown.y_rot);
            for _ in 0..bursts.clamp(0, CRUMB_BURSTS) {
                let crumbs = eating_crumbs(&held.id, shown.position, turns, fx.random);
                fx.item_crumbs.push(crumbs);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob_tracking::{look, shows, spawn_tracked};
    use crate::server_mobs::{is_simulated, mob_tags, spawn_saved};
    use glam::DVec3;
    use minecraftoss_entities::world::EntityWorld;
    use minecraftoss_entities::world::families::Family;
    use minecraftoss_entities::world::families::land::foxes::{INTERESTED, POUNCING};
    use minecraftoss_player::{Block, Pos, World};
    use std::collections::HashMap;

    struct Meadow;

    impl World for Meadow {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:grass_block"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    fn fox(world: &EntityWorld, id: u64) -> &FamilyMob<Fox> {
        world
            .family::<Fox>()
            .iter()
            .find(|mob| mob.core.id == id)
            .unwrap()
    }

    fn saved_tag(world: &EntityWorld, id: u64, tag: Option<Tag>) -> Tag {
        let originals: HashMap<u64, Tag> = tag.map(|tag| (id, tag)).into_iter().collect();
        let mut saved = mob_tags(world, |_| true, &originals);
        saved.retain(|(saved, _)| *saved == id);
        saved.remove(0).1
    }

    #[test]
    fn a_fox_saves_and_loads_with_its_type_trust_sleep_sit_crouch_love_and_mouth_item() {
        let text = "{id:\"minecraft:fox\",Pos:[1.5d,64.0d,2.5d],Rotation:[90.0f,0.0f],Type:\"snow\",Trusted:[[I;1,2,3,4],[I;5,6,7,8]],Sleeping:1b,Sitting:1b,Crouching:1b,Age:-1200,ForcedAge:40,AgeLocked:1b,InLove:300,LoveCause:[I;9,9,9,9],CanPickUpLoot:0b,Health:6.0f,equipment:{mainhand:{id:\"minecraft:emerald\",count:1}},drop_chances:{mainhand:2.0f},PersistenceRequired:1b}";
        let tag = parse(text);
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("a fox loads");
        assert_eq!(world.entity_type(id), Some(foxes::KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(49));
        let mob = fox(&world, id);
        let state = &mob.state;
        assert_eq!(state.variant, FoxVariant::Snow);
        assert_eq!(
            state.trusted,
            [uuid_from_ints([1, 2, 3, 4]), uuid_from_ints([5, 6, 7, 8])]
        );
        assert!(state.sleeping() && state.sitting() && state.flag(CROUCHING));
        assert_eq!(
            (state.age.ticks, state.age.forced, state.age.locked),
            (-1200, 40, true)
        );
        assert_eq!(state.in_love, 300);
        assert_eq!(state.love_cause, Some(uuid_from_ints([9, 9, 9, 9])));
        assert!(!state.can_pick_up_loot);
        let held = state.held.as_ref().unwrap();
        assert_eq!((held.id.as_str(), held.count), ("minecraft:emerald", 1));
        assert_eq!(state.drop_chance, 2.0);
        assert!(mob.core.baby && mob.core.persistent);
        assert_eq!(
            (mob.core.body.width, mob.core.eye_height, mob.core.health),
            (foxes::BABY_WIDTH, foxes::BABY_EYE_HEIGHT, 6.0)
        );
        assert_eq!(
            (state.ai.yaw, state.ai.body_rotation.body_yaw),
            (90.0, 90.0)
        );
        let saved = saved_tag(&world, id, Some(tag.clone()));
        for (key, value) in [
            ("Type", Tag::String("snow".to_owned())),
            ("Sleeping", Tag::Byte(1)),
            ("Sitting", Tag::Byte(1)),
            ("Crouching", Tag::Byte(1)),
            ("Age", Tag::Int(-1200)),
            ("ForcedAge", Tag::Int(40)),
            ("AgeLocked", Tag::Byte(1)),
            ("InLove", Tag::Int(300)),
            ("LoveCause", Tag::IntArray(vec![9, 9, 9, 9])),
            ("CanPickUpLoot", Tag::Byte(0)),
            ("Trusted", tag.get("Trusted").unwrap().clone()),
        ] {
            assert_eq!(saved.get(key), Some(&value), "{key}");
        }
        let stack = saved
            .get("equipment")
            .and_then(|e| e.get(MAINHAND))
            .unwrap();
        assert_eq!(
            stack.get("id").and_then(Tag::as_str),
            Some("minecraft:emerald")
        );
        let chance = saved.get("drop_chances").and_then(|c| c.get(MAINHAND));
        assert_eq!(chance.and_then(Tag::as_f64), Some(2.0));
        let mut again = EntityWorld::default();
        let back = spawn_saved(&mut again, &saved).unwrap();
        let back = &fox(&again, back).state;
        assert_eq!(back.variant, FoxVariant::Snow);
        assert_eq!(back.trusted.len(), 2);
        assert!(back.sleeping() && back.age.locked);
        assert_eq!(
            back.held.as_ref().map(|s| s.id.as_str()),
            Some("minecraft:emerald")
        );

        let plain = parse(
            "{id:\"minecraft:fox\",Pos:[1.5d,64.0d,2.5d],Trusted:[[I;1,1,1,1],[I;2,2,2,2],[I;3,3,3,3]],Type:\"arctic\"}",
        );
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &plain).unwrap();
        let state = &fox(&world, id).state;
        assert_eq!(state.variant, FoxVariant::Red, "an unknown type is red");
        assert_eq!(
            state.trusted,
            [uuid_from_ints([1, 1, 1, 1]), uuid_from_ints([3, 3, 3, 3])],
            "a third trust takes the second slot"
        );
        assert!(state.can_pick_up_loot && state.held.is_none());
        assert_eq!((state.flags(), state.drop_chance), (0, foxes::DROP_CHANCE));
        assert!(!fox(&world, id).core.baby);
        let saved = saved_tag(&world, id, Some(plain));
        assert_eq!(saved.get("Type"), Some(&Tag::String("red".to_owned())));
        assert_eq!(saved.get("LoveCause"), None);
        assert_eq!(saved.get("Sleeping"), Some(&Tag::Byte(0)));
        let empty = |key: &str| saved.get(key).and_then(|t| t.get(MAINHAND)).is_none();
        assert!(empty("equipment") && empty("drop_chances"));
    }

    #[test]
    fn a_friend_sees_a_fox_from_its_look_tag() {
        let mut host = EntityWorld::default();
        let tag = parse(
            "{id:\"minecraft:fox\",Pos:[1.5d,64.0d,2.5d],Type:\"snow\",Age:-500,ForcedAge:20,Sleeping:1b,Sitting:0b,Crouching:0b,equipment:{mainhand:{id:\"minecraft:feather\",count:1}}}",
        );
        let id = spawn_saved(&mut host, &tag).unwrap();
        let saved = saved_tag(&host, id, Some(tag));
        let seen = look(&saved);
        for key in [
            "Sleeping",
            "Sitting",
            "Crouching",
            "ForcedAge",
            "Age",
            "InLove",
        ] {
            assert_eq!(seen.get(key), None, "{key} changes while it lives");
        }
        assert_eq!(seen.get("Type"), Some(&Tag::String("snow".to_owned())));
        assert_eq!(seen.get("Baby"), Some(&Tag::Byte(1)));
        assert!(
            seen.get("equipment").is_some(),
            "the mouth item seldom changes"
        );
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved));
        let mirror = &fox(&friend, id);
        assert!(mirror.core.baby);
        assert_eq!(mirror.state.variant, FoxVariant::Snow);
        assert_eq!(
            mirror.state.held.as_ref().map(|s| s.id.as_str()),
            Some("minecraft:feather")
        );
        assert_eq!(crate::families::tracking_range(foxes::KIND), Some(8));
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!(show.flags, u32::from(SLEEPING));
        let mob = host.family_mob_mut::<Fox>(id).unwrap();
        mob.state.goals_mut().flags = SITTING | CROUCHING | INTERESTED | POUNCING;
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!(
            show.flags,
            u32::from(SITTING | CROUCHING | INTERESTED | POUNCING)
        );
        let mirror = friend.family_mob_mut::<Fox>(id).unwrap();
        for tick in 1..=30 {
            Fox::show_on(mirror, &show);
            let state = &mirror.state;
            assert!(state.sitting() && !state.sleeping());
            let crouch = (0.2_f32 * tick as f32).min(5.0);
            assert!((state.crouch_amount - crouch).abs() < 1.0e-4, "{tick}");
        }
        assert_eq!(mirror.state.crouch_amount, 5.0, "fully crouched");
        assert!(mirror.state.interested_angle > 0.99, "head cocked");
        Fox::show_on(mirror, &FamilyShow::default());
        let state = &mirror.state;
        assert_eq!((state.flags(), state.crouch_amount), (0, 0.0));
        assert!((state.interested_angle - state.interested_angle_old * 0.6).abs() < 1.0e-6);
    }

    #[test]
    fn a_loaded_fox_hunts_with_the_target_goals_of_its_type() {
        let mut world = EntityWorld::default();
        for (text, variant) in [
            ("", FoxVariant::Red),
            (",Type:\"red\"", FoxVariant::Red),
            (",Type:\"snow\"", FoxVariant::Snow),
            (",Type:\"snow\",Age:-24000", FoxVariant::Snow),
        ] {
            let tag = parse(&format!(
                "{{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d]{text}}}"
            ));
            let id = spawn_saved(&mut world, &tag).unwrap();
            let goals = fox(&world, id).state.goals().unwrap();
            assert_eq!(goals.hunts, Some(variant), "{text}");
        }
        let bred = Fox::spawn(foxes::KIND, DVec3::new(0.5, 64.0, 0.5)).unwrap();
        assert_eq!(
            bred.state.goals().unwrap().hunts,
            None,
            "a fox that was never loaded"
        );
    }

    #[test]
    fn a_dormant_fox_from_an_old_save_wakes() {
        assert!(is_simulated(foxes::KIND), "no longer kept dormant");
        let tag = parse(
            "{id:\"minecraft:fox\",Pos:[0.5d,64.0d,0.5d],Motion:[0.0d,0.0d,0.0d],Rotation:[0.0f,0.0f],FallDistance:0.0d,Fire:-1s,Air:300s,OnGround:1b,Invulnerable:0b,PortalCooldown:0,UUID:[I;1,2,3,4],Health:10.0f,HurtTime:0s,HurtByTimestamp:0,DeathTime:0s,AbsorptionAmount:0.0f,FallFlying:0b,LeftHanded:0b,CanPickUpLoot:1b,PersistenceRequired:0b,Age:0,ForcedAge:0,InLove:0,Trusted:[],Sleeping:0b,Type:\"red\",Sitting:0b,Crouching:0b,attributes:[{id:\"minecraft:follow_range\",base:32.0d}]}",
        );
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("an old fox loads");
        let player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 1,
            position: DVec3::new(6.5, 64.0, 0.5),
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
        };
        let mut meadow = Meadow;
        for _ in 0..40 {
            world.tick_with_players(&mut meadow, &[player]);
        }
        let mob = fox(&world, id);
        assert!(mob.core.tick_count >= 40, "it ticks");
        assert!(
            mob.core.body.position.x < 0.0,
            "it runs from the player: {}",
            mob.core.body.position
        );
    }

    fn fox_mut(world: &mut EntityWorld, id: u64) -> &mut FamilyMob<Fox> {
        world.family_mob_mut::<Fox>(id).unwrap()
    }

    struct Viewer {
        ambient: crate::ambient_particles::AmbientParticles,
        spell: crate::spell_particles::SpellParticles,
        portal: crate::portal_particles::PortalParticles,
        poof: crate::poof_particles::PoofParticles,
        item_crumbs: Vec<(String, DVec3, Vec<Vec3>)>,
        sounds: Vec<(String, DVec3, f32, f32)>,
        random: minecraftoss_player::rng::LegacyRandom,
        seen: HashMap<u64, i32>,
    }

    impl Viewer {
        fn new(seed: u64) -> Self {
            Self {
                ambient: crate::ambient_particles::AmbientParticles::new(seed),
                spell: crate::spell_particles::SpellParticles::new(seed),
                portal: Default::default(),
                poof: Default::default(),
                item_crumbs: Vec::new(),
                sounds: Vec::new(),
                random: minecraftoss_player::rng::LegacyRandom::new(seed),
                seen: HashMap::new(),
            }
        }

        fn crumbs(
            &mut self,
            world: &EntityWorld,
            poses: &ClientMobs,
        ) -> Vec<(String, DVec3, Vec<Vec3>)> {
            let mut fx = ClientFx {
                ambient: &mut self.ambient,
                spell: &mut self.spell,
                portal: &mut self.portal,
                poof: &mut self.poof,
                item_crumbs: &mut self.item_crumbs,
                sounds: &mut self.sounds,
                random: &mut self.random,
                seen: &mut self.seen,
            };
            crate::families::client_effects(world, poses, &mut fx);
            std::mem::take(&mut self.item_crumbs)
        }
    }

    #[test]
    fn a_friend_sees_fox_crumbs() {
        let mut host = EntityWorld::default();
        let tag = parse(
            "{id:\"minecraft:fox\",Pos:[1.5d,64.0d,2.5d],Rotation:[90.0f,0.0f],NoAI:1b,equipment:{mainhand:{id:\"minecraft:sweet_berries\",count:1}}}",
        );
        let id = spawn_saved(&mut host, &tag).unwrap();
        let saved = saved_tag(&host, id, Some(tag));
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved));
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&crate::client_mobs::server_mobs(&host));
        let mut viewers = [Viewer::new(3), Viewer::new(4)];
        let mut fired = [Vec::new(), Vec::new()];
        let events = [0, 0, 1, 1, 1, 2, 4, 4, 5];
        for (step, events) in events.into_iter().enumerate() {
            fox_mut(&mut host, id).state.eat_events = Some(events);
            let show = shows(&host)[&id].family.unwrap();
            assert_eq!(show.values[EAT_VALUE], events as i32);
            Fox::show_on(fox_mut(&mut friend, id), &show);
            for (index, world) in [&host, &friend].into_iter().enumerate() {
                for (item, at, velocities) in viewers[index].crumbs(world, &poses) {
                    fired[index].push(step);
                    assert_eq!((item.as_str(), velocities.len()), ("minecraft:sweet_berries", CRUMBS));
                    let look = view_vector(0.0, 90.0);
                    let mouth = DVec3::new(1.5 + look.x / 2.0, 64.0, 2.5 + look.z / 2.0);
                    assert!(at.distance(mouth) < 1e-9, "{at}");
                    for v in velocities {
                        assert!((0.15..=0.25).contains(&v.y), "{v}");
                        assert!(v.z.abs() <= 0.05 && v.x.abs() <= 1e-3, "{v}");
                    }
                }
            }
        }
        assert_eq!(fired[0], [2, 5, 6, 6, 8], "one burst for each event");
        assert_eq!(fired[1], fired[0], "the friend sees the same bursts");

        assert!(spawn_tracked(&mut friend, id, &saved), "a new look");
        assert_eq!(fox(&friend, id).state.eat_events, None);
        assert!(viewers[1].crumbs(&friend, &poses).is_empty(), "not shown yet");
        let show = shows(&host)[&id].family.unwrap();
        Fox::show_on(fox_mut(&mut friend, id), &show);
        assert!(viewers[1].crumbs(&friend, &poses).is_empty(), "no old events again");
        fox_mut(&mut host, id).state.eat_events = Some(6);
        let show = shows(&host)[&id].family.unwrap();
        Fox::show_on(fox_mut(&mut friend, id), &show);
        assert_eq!(viewers[1].crumbs(&friend, &poses).len(), 1);

        fox_mut(&mut friend, id).state.held = None;
        fox_mut(&mut friend, id).state.eat_events = Some(7);
        assert!(viewers[1].crumbs(&friend, &poses).is_empty(), "an empty mouth");
        fox_mut(&mut host, id).state.eat_events = Some(0);
        assert!(viewers[0].crumbs(&host, &poses).is_empty(), "a reloaded fox");
        fox_mut(&mut host, id).state.eat_events = Some(1);
        assert_eq!(viewers[0].crumbs(&host, &poses).len(), 1);
    }
}
