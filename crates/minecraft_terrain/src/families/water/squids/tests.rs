use super::*;
use crate::ambient_particles::AmbientParticles;
use crate::client_mobs::server_mobs;
use crate::mesh::ChunkMesh;
use crate::mob_tracking::{MobEvent, Trackers, Watcher, look, shows, spawn_tracked};
use crate::poof_particles::PoofParticles;
use crate::portal_particles::PortalParticles;
use crate::server_mobs::{mob_tags, spawn_saved};
use crate::spell_particles::SpellParticles;
use minecraftoss_entities::monster_ai::Target;
use minecraftoss_entities::world::families::Hurt;
use minecraftoss_entities::world::{DamageSourceKind, MobHit};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::rng::LegacyRandom;
use std::collections::HashMap;

fn parse(text: &str) -> Tag {
    minecraftoss_core::snbt::parse_compound(text).unwrap()
}

fn squid(world: &EntityWorld, id: u64) -> &FamilyMob<Squid> {
    world
        .family::<Squid>()
        .iter()
        .find(|mob| mob.core.id == id)
        .unwrap()
}

fn saved_tag(world: &EntityWorld, id: u64, tag: Tag) -> Tag {
    let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
    let saved = mob_tags(world, |_| true, &originals);
    saved.into_iter().find(|(saved, _)| *saved == id).unwrap().1
}

fn use_item(world: &mut EntityWorld, inventory: &mut Inventory, id: u64) -> MobOutcome {
    let mut actor = Actor {
        player: 1,
        inventory,
        selected: 0,
        infinite: false,
        entity_loot: None,
        shearing_loot: None,
    };
    crate::mob_actions::interact(world, MobHit::Family(64, id), &mut actor)
}

fn atlas() -> Atlas {
    let temp = crate::test_dir::tempdir().unwrap();
    std::fs::write(
        temp.path().join("pack.mcmeta"),
        r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
    )
    .unwrap();
    let packs = crate::pack::PackStack::open(vec![temp.path().into()]).unwrap();
    crate::mesh::make_atlas(&packs, Vec::new()).unwrap()
}

fn light() -> SkyLight {
    SkyLight::from_levels((-8, 56, -8), (17, 17, 17), vec![15; 4913], vec![0; 4913])
}

fn drawn(world: &EntityWorld, atlas: &Atlas, partial: f32) -> Vec<[f32; 3]> {
    let mut poses = ClientMobs::default();
    poses.spawn_missing(&server_mobs(world));
    let mut mesh = ChunkMesh::default();
    let mobs = world.family::<Squid>();
    render::append_squids(&mut mesh, mobs, &poses, atlas, &light(), partial);
    mesh.vertices.iter().map(|v| v.position).collect()
}

fn poses_of(world: &EntityWorld) -> ClientMobs {
    let mut poses = ClientMobs::default();
    poses.spawn_missing(&server_mobs(world));
    poses
}

struct Viewer {
    ambient: AmbientParticles,
    spell: SpellParticles,
    portal: PortalParticles,
    poof: PoofParticles,
    sounds: Vec<(String, DVec3, f32, f32)>,
    random: LegacyRandom,
    seen: HashMap<u64, i32>,
}

impl Viewer {
    fn new(seed: u64) -> Self {
        Self {
            ambient: AmbientParticles::new(seed),
            spell: SpellParticles::new(seed),
            portal: PortalParticles::default(),
            poof: PoofParticles::default(),
            sounds: Vec::new(),
            random: LegacyRandom::new(seed),
            seen: HashMap::new(),
        }
    }

    fn effects(&mut self, world: &EntityWorld) {
        let poses = poses_of(world);
        let mut item_crumbs = Vec::new();
        let mut fx = ClientFx {
            ambient: &mut self.ambient,
            spell: &mut self.spell,
            portal: &mut self.portal,
            poof: &mut self.poof,
            item_crumbs: &mut item_crumbs,
            sounds: &mut self.sounds,
            random: &mut self.random,
            seen: &mut self.seen,
        };
        crate::families::client_effects(world, &poses, &mut fx);
    }

    fn at(&self, kind: Kind) -> Vec<DVec3> {
        let particles = self.ambient.kinds_at();
        let shown = particles.filter(|(shown, _)| *shown == kind);
        shown.map(|(_, at)| at).collect()
    }
}

#[test]
fn a_squid_saves_and_loads_with_its_age_and_a_glow_squid_with_its_dark_ticks() {
    let text = "{id:\"minecraft:squid\",Pos:[1.5d,60.0d,2.5d],Rotation:[90.0f,0.0f],Age:-1200,ForcedAge:40,AgeLocked:1b,Health:4.0f,PersistenceRequired:1b}";
    let tag = parse(text);
    let mut world = EntityWorld::default();
    let id = spawn_saved(&mut world, &tag).expect("a squid loads");
    assert_eq!(world.entity_type(id), Some(squids::SQUID));
    assert_eq!(world.family_core(id).map(|(key, _)| key), Some(64));
    let mob = squid(&world, id);
    assert!(!mob.state.glow);
    let age = &mob.state.age;
    assert_eq!((age.ticks, age.forced, age.locked), (-1200, 40, true));
    assert!(mob.core.baby && mob.core.persistent);
    let body = &mob.core.body;
    assert_eq!(
        (body.width, body.height, mob.core.eye_height),
        (0.5, 0.5, 0.37)
    );
    assert_eq!(
        (mob.core.health, mob.core.yaw, mob.state.y_body_rot_o),
        (4.0, 90.0, 90.0)
    );
    let saved = saved_tag(&world, id, tag);
    for (key, value) in [
        ("Age", Tag::Int(-1200)),
        ("ForcedAge", Tag::Int(40)),
        ("AgeLocked", Tag::Byte(1)),
    ] {
        assert_eq!(saved.get(key), Some(&value), "{key}");
    }
    assert_eq!(
        saved.get("DarkTicksRemaining"),
        None,
        "only a glow squid goes dark"
    );
    let mut again = EntityWorld::default();
    let back = spawn_saved(&mut again, &saved).unwrap();
    let back = squid(&again, back);
    assert_eq!(back.state.age, squid(&world, id).state.age);
    assert!(back.core.baby && back.core.persistent);

    let glow = parse("{id:\"minecraft:glow_squid\",Pos:[1.5d,30.0d,2.5d],DarkTicksRemaining:37}");
    let mut world = EntityWorld::default();
    let id = spawn_saved(&mut world, &glow).unwrap();
    assert_eq!(world.entity_type(id), Some(GLOW_SQUID));
    let mob = squid(&world, id);
    assert!(mob.state.glow && !mob.core.baby);
    assert_eq!(
        (
            mob.state.dark_ticks,
            mob.core.body.width,
            mob.core.eye_height
        ),
        (37, 0.8, 0.4)
    );
    world.family_mob_mut::<Squid>(id).unwrap().state.dark_ticks = 12;
    let saved = saved_tag(&world, id, glow);
    assert_eq!(saved.get("DarkTicksRemaining"), Some(&Tag::Int(12)));
    assert_eq!(saved.get("Age"), Some(&Tag::Int(0)));
    let mut again = EntityWorld::default();
    let back = spawn_saved(&mut again, &saved).unwrap();
    assert_eq!(squid(&again, back).state.dark_ticks, 12);

    let shown = parse("{id:\"minecraft:squid\",Pos:[1.5d,60.0d,2.5d],Baby:1b}");
    let mut world = EntityWorld::default();
    let id = spawn_saved(&mut world, &shown).unwrap();
    assert_eq!(squid(&world, id).state.age.ticks, Age::BABY_START);
    assert!(squid(&world, id).core.baby);
}

#[test]
fn a_friend_sees_a_squid_from_its_look_tag() {
    let mut host = EntityWorld::default();
    let tag = parse(
        "{id:\"minecraft:glow_squid\",Pos:[1.5d,30.0d,2.5d],Age:-500,ForcedAge:20,AgeLocked:1b,DarkTicksRemaining:60}",
    );
    let id = spawn_saved(&mut host, &tag).unwrap();
    let saved = saved_tag(&host, id, tag);
    let seen = look(&saved);
    for key in ["Age", "AgeLocked", "DarkTicksRemaining"] {
        assert_eq!(seen.get(key), None, "{key} changes while it lives");
    }
    assert_eq!(seen.get("ForcedAge"), Some(&Tag::Int(20)));
    assert_eq!(
        seen.get("Baby"),
        Some(&Tag::Byte(1)),
        "growing up is a new look"
    );
    let mut friend = EntityWorld::default();
    assert!(spawn_tracked(&mut friend, id, &saved));
    assert_eq!(friend.entity_type(id), Some(GLOW_SQUID));
    let mirror = squid(&friend, id);
    assert!(mirror.state.glow && mirror.core.baby);
    assert_eq!(mirror.core.body.height, 0.5);
    let show = shows(&host)[&id].family.expect("a squid shows its state");
    assert_eq!(show.flags, 0);
    assert_eq!(show.values[DARK_TICKS], 60);
    Squid::show_on(friend.family_mob_mut::<Squid>(id).unwrap(), &show);
    assert_eq!(squid(&friend, id).state.dark_ticks, 60);
}

#[test]
fn a_squid_tracks_at_eight_chunks_and_a_glow_squid_at_ten() {
    use crate::families::tracking_range;
    assert_eq!(
        (tracking_range(squids::SQUID), tracking_range("squid")),
        (Some(8), Some(8))
    );
    assert_eq!(
        (tracking_range(GLOW_SQUID), tracking_range("glow_squid")),
        (Some(10), Some(10))
    );
    assert_eq!(
        (
            crate::mob_tracking::tracking_range("minecraft:squid"),
            crate::mob_tracking::tracking_range("glow_squid")
        ),
        (8, 10)
    );
    let tags = [
        parse("{id:\"minecraft:squid\",Pos:[150.5d,60.0d,0.5d]}"),
        parse("{id:\"minecraft:glow_squid\",Pos:[150.5d,30.0d,2.5d]}"),
        parse("{id:\"minecraft:squid\",Pos:[100.5d,60.0d,0.5d]}"),
    ];
    let mut host = EntityWorld::default();
    let ids: Vec<u64> = tags
        .iter()
        .map(|tag| spawn_saved(&mut host, tag).unwrap())
        .collect();
    let originals: HashMap<u64, Tag> = ids.iter().copied().zip(tags).collect();
    let mut trackers = Trackers::default();
    trackers.tick();
    let watcher = Watcher {
        id: 3,
        feet: [0.5, 60.0, 0.5],
        view_distance: 12,
    };
    let tracked = trackers.update(&host, &originals, &[watcher]);
    let mut added: Vec<u64> = tracked
        .events_of(3)
        .filter_map(|event| match event {
            MobEvent::Add(id, _) => Some(*id),
            _ => None,
        })
        .collect();
    added.sort_unstable();
    assert_eq!(
        added,
        vec![ids[1], ids[2]],
        "150 blocks away a squid is past its eight chunks and a glow squid within its ten"
    );
}

#[test]
fn a_friend_sees_a_squid_pulse_tilt_and_roll_in_the_same_pose() {
    let tag = parse("{id:\"minecraft:squid\",Pos:[0.5d,64.0d,0.5d],Rotation:[30.0f,0.0f]}");
    let mut host = EntityWorld::default();
    let id = spawn_saved(&mut host, &tag).unwrap();
    let saved = saved_tag(&host, id, tag);
    let mut friend = EntityWorld::default();
    assert!(spawn_tracked(&mut friend, id, &saved));
    let mut before = (0.0_f32, 0.0_f32, 0.0_f32, 30.0_f32);
    let steps: [(f32, f32, f32, f32); 2] = [(0.3, -20.0, 45.0, 30.0), (0.6, -35.0, 49.5, 34.0)];
    for (angle, tilt, roll, yaw) in steps {
        let mob = host.family_mob_mut::<Squid>(id).unwrap();
        let state = &mut mob.state;
        (
            state.tentacle_angle_old,
            state.x_body_rot_o,
            state.z_body_rot_o,
            state.y_body_rot_o,
        ) = before;
        (state.tentacle_angle, state.x_body_rot, state.z_body_rot) = (angle, tilt, roll);
        mob.core.yaw = yaw;
        before = (angle, tilt, roll, yaw);
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!(
            show.values[..4],
            [angle, tilt, roll, yaw].map(|value| value.to_bits() as i32)
        );
        Squid::show_on(friend.family_mob_mut::<Squid>(id).unwrap(), &show);
    }
    let pair = |mob: &FamilyMob<Squid>| {
        let state = &mob.state;
        [
            state.tentacle_angle_old,
            state.tentacle_angle,
            state.x_body_rot_o,
            state.x_body_rot,
            state.z_body_rot_o,
            state.z_body_rot,
            state.y_body_rot_o,
            mob.core.yaw,
        ]
    };
    assert_eq!(pair(squid(&friend, id)), pair(squid(&host, id)));
    assert_eq!(
        pair(squid(&friend, id)),
        [0.3, 0.6, -20.0, -35.0, 45.0, 49.5, 30.0, 34.0]
    );
    let atlas = atlas();
    for partial in [0.0, 0.5, 1.0] {
        assert_eq!(
            drawn(&friend, &atlas, partial),
            drawn(&host, &atlas, partial),
            "one pose in both windows at {partial}"
        );
    }
    assert_ne!(drawn(&host, &atlas, 0.0), drawn(&host, &atlas, 1.0));
}

#[test]
fn a_golden_dandelion_locks_a_baby_squid_s_age() {
    let mut world = EntityWorld::default();
    let baby = spawn_saved(
        &mut world,
        &parse("{id:\"minecraft:squid\",Pos:[1.5d,60.0d,2.5d],Age:-1000}"),
    )
    .unwrap();
    let adult = spawn_saved(
        &mut world,
        &parse("{id:\"minecraft:squid\",Pos:[4.5d,60.0d,2.5d]}"),
    )
    .unwrap();
    let mut inventory = Inventory::default();
    inventory.slots[0] = Some(ItemStack::new("minecraft:golden_dandelion", 2));
    let count = |inventory: &Inventory| inventory.slots[0].as_ref().map(|stack| stack.count);
    let locked = use_item(&mut world, &mut inventory, baby);
    assert!(locked.handled && locked.inventory_changed);
    let sounds: Vec<&str> = locked.sounds.iter().map(|s| s.event.as_str()).collect();
    assert_eq!(sounds, ["item.golden_dandelion.use"]);
    assert_eq!(locked.sounds[0].position, DVec3::new(1.5, 60.0, 2.5));
    assert_eq!(count(&inventory), Some(1));
    let mob = squid(&world, baby);
    assert!(mob.state.age.locked && mob.core.persistent && mob.core.baby);
    assert_eq!(mob.state.age.ticks, Age::BABY_START);
    assert!(
        !use_item(&mut world, &mut inventory, baby).handled,
        "the lock waits two seconds"
    );
    world
        .family_mob_mut::<Squid>(baby)
        .unwrap()
        .state
        .age
        .lock_particle_ticks = 0;
    let unlocked = use_item(&mut world, &mut inventory, baby);
    assert!(unlocked.handled);
    assert_eq!(unlocked.sounds[0].event, "item.golden_dandelion.unuse");
    assert!(!squid(&world, baby).state.age.locked);
    assert!(inventory.slots[0].is_none(), "both were used");

    inventory.slots[0] = Some(ItemStack::new("minecraft:golden_dandelion", 1));
    assert!(
        !use_item(&mut world, &mut inventory, adult).handled,
        "an adult is not locked"
    );
    world
        .family_mob_mut::<Squid>(baby)
        .unwrap()
        .state
        .age
        .lock_particle_ticks = 0;
    inventory.slots[0] = Some(ItemStack::new("minecraft:cod", 1));
    assert!(
        !use_item(&mut world, &mut inventory, baby).handled,
        "a squid eats nothing"
    );
    inventory.slots[0] = Some(ItemStack::new("minecraft:golden_dandelion", 1));
    let hurt = Hurt::new(100.0, "minecraft:generic", DamageSourceKind::Generic);
    world.hurt_family(baby, &hurt);
    assert!(
        !use_item(&mut world, &mut inventory, baby).handled,
        "a dying squid"
    );
    assert_eq!(count(&inventory), Some(1));
}

#[test]
fn a_squirt_draws_thirty_specks_behind_the_squid_once_and_none_on_first_sight() {
    let mut world = EntityWorld::default();
    let tag = parse("{id:\"minecraft:squid\",Pos:[0.5d,60.0d,0.5d]}");
    let id = spawn_saved(&mut world, &tag).unwrap();
    let mob = world.family_mob_mut::<Squid>(id).unwrap();
    mob.state.ink_spurts = 3;
    mob.state.x_body_rot_o = -90.0;
    mob.core.body.position = DVec3::new(0.5, 60.0, 2.5);
    let mut viewer = Viewer::new(1);
    viewer.effects(&world);
    assert!(
        viewer.at(Kind::SquidInk).is_empty(),
        "squirts from before it was seen are not drawn"
    );
    assert_eq!(viewer.seen[&id], 3);
    world.family_mob_mut::<Squid>(id).unwrap().state.ink_spurts = 4;
    viewer.effects(&world);
    let specks = viewer.at(Kind::SquidInk);
    assert_eq!(specks.len(), 30);
    let behind = DVec3::new(0.5, 60.5, -0.5);
    assert!(
        specks.iter().all(|at| at.distance(behind) < 1e-3),
        "from behind where it swam forward at the tick's start: {specks:?}"
    );
    viewer.effects(&world);
    assert_eq!(viewer.at(Kind::SquidInk).len(), 30, "each squirt once");
    world.family_mob_mut::<Squid>(id).unwrap().state.ink_spurts = 6;
    viewer.effects(&world);
    assert_eq!(viewer.at(Kind::SquidInk).len(), 90, "two squirts in a tick");
    world.family_mob_mut::<Squid>(id).unwrap().state.ink_spurts = 106;
    viewer.effects(&world);
    let most = 90 + 30 * MAX_SQUIRTS_SEEN as usize;
    assert_eq!(viewer.at(Kind::SquidInk).len(), most);
    assert!(viewer.at(Kind::Glow).is_empty() && viewer.at(Kind::GlowSquidInk).is_empty());

    let glow = spawn_saved(
        &mut world,
        &parse("{id:\"minecraft:glow_squid\",Pos:[4.5d,30.0d,0.5d]}"),
    )
    .unwrap();
    viewer.effects(&world);
    viewer.effects(&world);
    let sparkles = viewer.at(Kind::Glow);
    assert_eq!(sparkles.len(), 2, "a sparkle a tick");
    for at in &sparkles {
        let inside = (at.x - 4.5).abs() <= 0.48 && (at.z - 0.5).abs() <= 0.48;
        assert!(inside && (30.0..30.8).contains(&at.y), "{at}");
    }
    assert!(viewer.at(Kind::GlowSquidInk).is_empty());
    world
        .family_mob_mut::<Squid>(glow)
        .unwrap()
        .state
        .ink_spurts = 1;
    viewer.effects(&world);
    assert_eq!(viewer.at(Kind::GlowSquidInk).len(), 30, "glow ink");
    assert_eq!(viewer.at(Kind::SquidInk).len(), most);
    assert_eq!(viewer.at(Kind::Glow).len(), 3);
    world.family_core_mut(glow).unwrap().1.removed = true;
    viewer.effects(&world);
    assert_eq!(viewer.at(Kind::Glow).len(), 3, "a glow squid that left");
}

#[test]
fn a_friend_sees_a_squid_squirt_and_a_glow_squid_go_dark_at_the_same_tick() {
    let tag = parse("{id:\"minecraft:glow_squid\",Pos:[0.5d,64.0d,0.5d],Rotation:[60.0f,0.0f]}");
    let mut host = EntityWorld::default();
    let id = spawn_saved(&mut host, &tag).unwrap();
    let saved = saved_tag(&host, id, tag);
    let mut friend = EntityWorld::default();
    assert!(spawn_tracked(&mut friend, id, &saved));
    let atlas = atlas();
    let lit = |world: &EntityWorld| {
        let mut mesh = ChunkMesh::default();
        let mobs = world.family::<Squid>();
        render::append_squids(&mut mesh, mobs, &poses_of(world), &atlas, &light(), 0.0);
        mesh.vertices[0].block_light
    };
    let (mut on_host, mut on_friend) = (Viewer::new(1), Viewer::new(2));
    on_host.effects(&host);
    on_friend.effects(&friend);
    assert_eq!((lit(&host), lit(&friend)), (15.0, 15.0));
    let hurt = Hurt::new(
        1.0,
        "minecraft:player_attack",
        DamageSourceKind::PlayerAttack,
    )
    .by(Some(Target::Player(1)), None);
    host.hurt_family(id, &hurt);
    let state = &squid(&host, id).state;
    assert_eq!((state.ink_spurts, state.dark_ticks), (1, 100));
    let show = shows(&host)[&id].family.unwrap();
    Squid::show_on(friend.family_mob_mut::<Squid>(id).unwrap(), &show);
    let mirror = &squid(&friend, id).state;
    assert_eq!((mirror.ink_spurts, mirror.dark_ticks), (1, 100));
    on_host.effects(&host);
    on_friend.effects(&friend);
    let (here, there) = (
        on_host.at(Kind::GlowSquidInk),
        on_friend.at(Kind::GlowSquidInk),
    );
    assert_eq!((here.len(), there.len()), (30, 30));
    assert_eq!(here, there, "the cloud starts in one place in both windows");
    assert_eq!(
        (lit(&host), lit(&friend)),
        (0.0, 0.0),
        "dark in both windows"
    );
}
