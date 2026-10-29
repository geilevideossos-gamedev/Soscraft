use super::*;
use crate::client_mobs::{server_mobs, shadow_casters};
use crate::mob_actions::would_interact;
use minecraftoss_entities::world::MobHit;
use minecraftoss_entities::world::families::pests::endermites::Endermite;
use minecraftoss_entities::world::families::pests::silverfish::Silverfish;
use minecraftoss_player::inventory::{Inventory, ItemStack};

const FISH: &str = "minecraft:silverfish";
const MITE: &str = "minecraft:endermite";

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

fn spawn(world: &mut EntityWorld, kind: &str, x: f64) -> u64 {
    world
        .spawn_family_kind(kind, DVec3::new(x, 64.0, 0.5))
        .unwrap()
}

fn seen_by_client(world: &EntityWorld) -> ClientMobs {
    let mut mobs = ClientMobs::default();
    mobs.spawn_missing(&server_mobs(world));
    mobs
}

#[test]
fn a_registered_mob_is_tracked_textured_and_shadowed_through_its_family() {
    let mut world = EntityWorld::default();
    let fish = spawn(&mut world, FISH, 0.5);
    let mite = spawn(&mut world, MITE, 3.5);
    let zombie = world.spawn_zombie(
        minecraftoss_entities::zombie::Zombie::new(DVec3::new(6.5, 64.0, 0.5)),
        true,
    );
    world.family_core_mut(mite).unwrap().1.baby = true;
    assert_eq!(
        textures(),
        [
            "minecraft:entity/silverfish/silverfish",
            "minecraft:entity/endermite/endermite"
        ]
    );
    assert!(knows(FISH) && knows(MITE) && !knows("minecraft:zombie"));
    assert_eq!(
        (cage_margin(FISH), cage_margin("silverfish")),
        (Some(0.05), Some(0.05))
    );
    assert_eq!(cage_margin("minecraft:zombie"), None);
    assert_eq!(
        (max_head_y_rot(&world, fish), max_head_y_rot(&world, zombie)),
        (Some(75.0), None)
    );
    let tracked = server_mobs(&world);
    let turns: Vec<(u64, f32)> = tracked
        .iter()
        .map(|mob| (mob.id, mob.max_head_y_rot))
        .collect();
    assert!(turns.contains(&(fish, 75.0)) && turns.contains(&(mite, 75.0)));
    let mobs = seen_by_client(&world);
    let radius = |world: &EntityWorld, x: f64| {
        shadow_casters(world, &mobs, DVec3::ZERO, 0.0)
            .iter()
            .find(|caster| caster.position.x == x)
            .map(|caster| caster.radius)
    };
    assert_eq!(radius(&world, 0.5), Some(0.3));
    assert_eq!(radius(&world, 3.5), Some(0.15), "a baby's shadow is half");
    world.family_core_mut(fish).unwrap().1.removed = true;
    assert_eq!(radius(&world, 0.5), None, "a mob that left casts none");
    assert!(server_mobs(&world).iter().all(|mob| mob.id != fish));
}

#[test]
fn a_glowing_registered_mob_is_drawn_alone_for_its_outline() {
    let mut world = EntityWorld::default();
    spawn(&mut world, FISH, 0.5);
    let second = spawn(&mut world, FISH, 2.5);
    let mite = spawn(&mut world, MITE, 4.5);
    let mobs = seen_by_client(&world);
    let (atlas, light) = (atlas(), light());
    let outlined = |world: &EntityWorld, glows: &dyn Fn(u64) -> bool| {
        let mut seen = Vec::new();
        draw_glowing(world, &mobs, &atlas, &light, 0.5, glows, &mut |id, mesh| {
            seen.push((
                id,
                mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
            ));
        });
        seen
    };
    let glowing = |id: u64| id == second || id == mite;
    let seen = outlined(&world, &glowing);
    let ids: Vec<u64> = seen.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, [second, mite], "only the glowing ones, each alone");
    let alone = world
        .family::<Silverfish>()
        .iter()
        .find(|mob| mob.core.id == second)
        .unwrap();
    let mut models = ChunkMesh::default();
    let mut culled = ChunkMesh::default();
    let mut translucent = ChunkMesh::default();
    let mut held = Vec::new();
    Silverfish::draw(
        &mut FamilyDraw {
            models: &mut models,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut held,
        },
        std::slice::from_ref(alone),
        &mobs,
        &atlas,
        &light,
        0.5,
    );
    let own: Vec<_> = models
        .vertices
        .iter()
        .chain(&culled.vertices)
        .chain(&translucent.vertices)
        .map(|v| v.position)
        .collect();
    assert_eq!(
        seen[0].1, own,
        "its outline is its own model and nothing else"
    );
    assert!(!own.is_empty() && !seen[1].1.is_empty(), "both are drawn");
    assert!(outlined(&world, &|_| false).is_empty());
    world.family_core_mut(mite).unwrap().1.removed = true;
    let ids: Vec<u64> = outlined(&world, &glowing)
        .iter()
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(ids, [second], "a mob that left glows no more");
}

#[test]
fn a_click_on_a_registered_mob_passes_unless_its_family_takes_it() {
    let mut world = EntityWorld::default();
    let fish = spawn(&mut world, FISH, 0.5);
    let mut inventory = Inventory::default();
    inventory.slots[0] = Some(ItemStack::new("minecraft:lead", 1));
    let mut actor = Actor {
        player: 0,
        inventory: &mut inventory,
        selected: 0,
        infinite: false,
        entity_loot: None,
        shearing_loot: None,
    };
    let mut outcome = MobOutcome::default();
    assert!(!interact(&mut world, 36, fish, &mut actor, &mut outcome));
    assert!(
        !interact(&mut world, 38, fish, &mut actor, &mut outcome),
        "no family has key 38"
    );
    assert!(!outcome.handled && outcome.sounds.is_empty());
    let used = crate::mob_actions::interact(&mut world, MobHit::Family(36, fish), &mut actor);
    assert!(
        !used.handled && !used.inventory_changed,
        "a silverfish takes no lead and no item"
    );
    assert_eq!(
        inventory.slots[0].as_ref().map(|stack| stack.count),
        Some(1)
    );
    assert!(!would_interact(
        &world,
        MobHit::Family(36, fish),
        &inventory,
        0,
        false,
        0
    ));
}

#[test]
fn client_effects_forget_registered_mobs_that_are_gone() {
    let mut world = EntityWorld::default();
    let fish = spawn(&mut world, FISH, 0.5);
    let mite = spawn(&mut world, MITE, 2.5);
    let mobs = seen_by_client(&world);
    let mut ambient = AmbientParticles::new(1);
    let mut spell = SpellParticles::new(1);
    let mut portal = PortalParticles::default();
    let mut poof = PoofParticles::default();
    let mut sounds = Vec::new();
    let mut random = LegacyRandom::new(1);
    let mut seen: HashMap<u64, i32> = [(fish, 3), (mite, 4), (999, 5)].into_iter().collect();
    let mut fx = ClientFx {
        ambient: &mut ambient,
        spell: &mut spell,
        portal: &mut portal,
        poof: &mut poof,
        sounds: &mut sounds,
        random: &mut random,
        seen: &mut seen,
    };
    client_effects(&world, &mobs, &mut fx);
    assert_eq!(fx.seen.len(), 2);
    world.remove_where(|_| true);
    client_effects(&world, &mobs, &mut fx);
    assert!(fx.seen.is_empty(), "both left with their chunk");
    assert!(world.family::<Endermite>().is_empty());
}
