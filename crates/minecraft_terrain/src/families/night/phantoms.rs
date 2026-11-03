use crate::ambient_particles::Kind;
use crate::client_mobs::ClientMobs;
use crate::families::{ClientFx, FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::night::phantoms;
pub use minecraftoss_entities::world::families::night::phantoms::Phantom;

mod render;

pub const SHADOW_RADIUS: f32 = 0.75;

impl TerrainFamily for Phantom {
    const TRACKING_RANGE: i32 = 8;
    const VOLATILE: &'static [&'static str] = &["anchor_pos"];
    const TEXTURES: &'static [&'static str] = &render::TEXTURES;
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;
    const CAGE_MARGIN: f32 = render::CAGE_MARGIN;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        let yaw = mob.core.yaw;
        mob.state.ai.yaw = yaw;
        mob.state.ai.body_rotation.body_yaw = yaw;
        mob.core.look.head_yaw = yaw;
        let size = tag.get("size").and_then(Tag::as_i64).unwrap_or(0);
        phantoms::set_size(mob, size.clamp(0, i64::from(phantoms::MAX_SIZE)) as i32);
        let anchor = tag.get("anchor_pos").and_then(Tag::as_ints);
        let anchor = anchor.and_then(|ints| <[i32; 3]>::try_from(ints).ok());
        mob.state.goals_mut().anchor = anchor.map(|[x, y, z]| (x, y, z));
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        let mut fields = vec![("size", Tag::Int(mob.state.size))];
        let anchor = mob.state.goals().and_then(|goals| goals.anchor);
        if let Some((x, y, z)) = anchor {
            fields.push(("anchor_pos", Tag::IntArray(vec![x, y, z])));
        }
        fields
    }

    fn draw(
        out: &mut FamilyDraw<'_>,
        mobs: &[FamilyMob<Self>],
        poses: &ClientMobs,
        atlas: &Atlas,
        light: &SkyLight,
        partial: f32,
    ) {
        let shown = mobs.iter().filter(|mob| !mob.core.removed);
        render::append_phantoms(out.models, shown, poses, atlas, light, partial);
    }

    fn client_effects(mobs: &[FamilyMob<Self>], poses: &ClientMobs, fx: &mut ClientFx<'_>) {
        for mob in mobs.iter().filter(|mob| !mob.core.removed) {
            let Some(shown) = poses.get(mob.core.id) else {
                continue;
            };
            let offset = render::flap_offset(mob.core.id);
            let beat = render::wing_beat(offset, shown.tick_count);
            if render::flaps(offset, shown.tick_count) {
                let volume = 0.95 + fx.random.next_float() * 0.05;
                let pitch = 0.95 + fx.random.next_float() * 0.05;
                let sound = render::FLAP_SOUND.to_owned();
                fx.sounds.push((sound, shown.position, volume, pitch));
            }
            let (width, height) = (mob.core.body.width, mob.core.body.height);
            for at in render::specks(shown.position, shown.y_rot, width, height, beat) {
                fx.ambient.add(Kind::Mycelium, at, DVec3::ZERO);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_mobs::server_mobs;
    use crate::families::FamilyShow;
    use crate::mob_tracking::{look, shows, spawn_tracked};
    use crate::server_mobs::{mob_tags, spawn_saved};
    use minecraftoss_entities::world::EntityWorld;
    use std::collections::HashMap;

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    fn phantom(world: &EntityWorld, id: u64) -> &FamilyMob<Phantom> {
        world
            .family::<Phantom>()
            .iter()
            .find(|mob| mob.core.id == id)
            .unwrap()
    }

    fn saved_tag(world: &EntityWorld, id: u64, tag: Tag) -> Tag {
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let mut saved = mob_tags(world, |_| true, &originals);
        saved.retain(|(saved, _)| *saved == id);
        saved.remove(0).1
    }

    #[test]
    fn a_phantom_saves_and_loads_with_its_size_and_anchor() {
        let text = "{id:\"minecraft:phantom\",Pos:[1.5d,80.0d,2.5d],Rotation:[45.0f,10.0f],Health:12.0f,size:4,anchor_pos:[I;1,85,2],PersistenceRequired:1b}";
        let tag = parse(text);
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("a phantom loads");
        assert_eq!(world.entity_type(id), Some(phantoms::KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(80));
        let mob = phantom(&world, id);
        assert_eq!((mob.state.size, mob.state.attack_damage), (4, 10.0));
        assert!((mob.core.body.width - 1.44).abs() < 1e-6);
        assert!((mob.core.body.height - 0.8).abs() < 1e-6);
        assert_eq!((mob.core.health, mob.core.persistent), (12.0, true));
        assert_eq!(mob.state.goals().unwrap().anchor, Some((1, 85, 2)));
        assert_eq!((mob.core.yaw, mob.core.look.head_yaw), (45.0, 45.0));
        let saved = saved_tag(&world, id, tag);
        assert_eq!(saved.get("size"), Some(&Tag::Int(4)));
        assert_eq!(
            saved.get("anchor_pos"),
            Some(&Tag::IntArray(vec![1, 85, 2]))
        );
        let mut again = EntityWorld::default();
        let back = spawn_saved(&mut again, &saved).unwrap();
        let back = phantom(&again, back);
        assert_eq!((back.state.size, back.state.attack_damage), (4, 10.0));
        assert_eq!(back.state.goals().unwrap().anchor, Some((1, 85, 2)));

        let fresh = parse("{id:\"minecraft:phantom\",Pos:[1.5d,80.0d,2.5d]}");
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &fresh).unwrap();
        let mob = phantom(&world, id);
        assert_eq!((mob.state.size, mob.state.attack_damage), (0, 2.0));
        assert_eq!((mob.core.body.width, mob.core.health), (0.9, 20.0));
        assert_eq!(mob.state.goals().unwrap().anchor, None);
        let saved = saved_tag(&world, id, fresh);
        assert_eq!(saved.get("size"), Some(&Tag::Int(0)));
        assert_eq!(saved.get("anchor_pos"), None, "no anchor yet, none saved");
        let huge = parse("{id:\"minecraft:phantom\",Pos:[1.5d,80.0d,2.5d],size:900}");
        let id = spawn_saved(&mut world, &huge).unwrap();
        assert_eq!(phantom(&world, id).state.size, phantoms::MAX_SIZE);
    }

    #[test]
    fn a_friend_sees_a_phantom_from_its_look_tag() {
        let mut host = EntityWorld::default();
        let tag =
            parse("{id:\"minecraft:phantom\",Pos:[1.5d,80.0d,2.5d],size:6,anchor_pos:[I;1,85,2]}");
        let id = spawn_saved(&mut host, &tag).unwrap();
        let saved = saved_tag(&host, id, tag);
        let seen = look(&saved);
        assert_eq!(
            seen.get("anchor_pos"),
            None,
            "the anchor moves while it lives"
        );
        assert_eq!(seen.get("size"), Some(&Tag::Int(6)));
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved));
        let mirror = phantom(&friend, id);
        assert_eq!(mirror.state.size, 6);
        assert!((mirror.core.body.width - 0.9 * 1.9).abs() < 1e-5);
        assert_eq!(crate::families::tracking_range(phantoms::KIND), Some(8));
        assert_eq!(crate::families::tracking_range("phantom"), Some(8));
        assert_eq!(shows(&host)[&id].family, Some(FamilyShow::default()));
        host.family_mob_mut::<Phantom>(id).unwrap().core.look.pitch = -35.0;
        let tracked = server_mobs(&host);
        let mob = tracked.iter().find(|mob| mob.id == id).unwrap();
        assert_eq!(mob.x_rot, -35.0, "its dive reaches every client");
        assert!((mob.size.0 - 0.9 * 1.9).abs() < 1e-5);
    }

    struct Viewer {
        poses: ClientMobs,
        ambient: crate::ambient_particles::AmbientParticles,
        spell: crate::spell_particles::SpellParticles,
        portal: crate::portal_particles::PortalParticles,
        poof: crate::poof_particles::PoofParticles,
        sounds: Vec<(String, DVec3, f32, f32)>,
        random: minecraftoss_player::rng::LegacyRandom,
        seen: HashMap<u64, i32>,
    }

    impl Viewer {
        fn new(world: &EntityWorld, seed: u64) -> Self {
            let mut poses = ClientMobs::default();
            poses.spawn_missing(&server_mobs(world));
            Self {
                poses,
                ambient: crate::ambient_particles::AmbientParticles::new(seed),
                spell: crate::spell_particles::SpellParticles::new(seed),
                portal: Default::default(),
                poof: Default::default(),
                sounds: Vec::new(),
                random: minecraftoss_player::rng::LegacyRandom::new(seed),
                seen: HashMap::new(),
            }
        }

        fn tick(&mut self, world: &EntityWorld) {
            self.poses.tick();
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
            crate::families::client_effects(world, &self.poses, &mut fx);
        }

        fn flaps(&mut self, tick: i32, heard: &mut Vec<i32>) {
            for (sound, at, volume, pitch) in self.sounds.drain(..) {
                assert_eq!(sound, "minecraft:entity.phantom.flap");
                assert_eq!(at, FEET);
                assert!((0.95..=1.0).contains(&volume), "{volume}");
                assert!((0.95..=1.0).contains(&pitch), "{pitch}");
                heard.push(tick);
            }
        }
    }

    const FEET: DVec3 = DVec3::new(0.5, 80.0, 0.5);

    fn hovering(yaw: f32, size: i32) -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        let text = format!(
            "{{id:\"minecraft:phantom\",Pos:[0.5d,80.0d,0.5d],Rotation:[{yaw:.1}f,0.0f],NoAI:1b,size:{size}}}"
        );
        let id = spawn_saved(&mut world, &parse(&text)).unwrap();
        (world, id)
    }

    #[test]
    fn a_phantom_flaps_once_every_wing_beat_for_every_viewer() {
        let (world, id) = hovering(0.0, 0);
        let offset = render::flap_offset(id);
        let mut host = Viewer::new(&world, 1);
        let mut friend = None;
        let mut stranger = Viewer::new(&EntityWorld::default(), 3);
        let (mut host_heard, mut friend_heard, mut none) = (Vec::new(), Vec::new(), Vec::new());
        for tick in 1..=400 {
            if tick == 11 {
                friend = Some(Viewer::new(&world, 2));
            }
            host.tick(&world);
            host.flaps(tick, &mut host_heard);
            if let Some(friend) = friend.as_mut() {
                friend.tick(&world);
                friend.flaps(tick, &mut friend_heard);
            }
            stranger.tick(&world);
            stranger.flaps(tick, &mut none);
        }
        assert!(host_heard.len() >= 7, "{host_heard:?}");
        assert!(
            host_heard
                .windows(2)
                .all(|pair| (48..=49).contains(&(pair[1] - pair[0]))),
            "one flap a wing beat: {host_heard:?}"
        );
        assert!(host_heard.iter().all(|&tick| render::flaps(offset, tick)));
        let shifted: Vec<i32> = host_heard
            .iter()
            .filter(|&&tick| tick <= 390)
            .map(|tick| tick + 10)
            .collect();
        assert_eq!(friend_heard, shifted, "each viewer counts its own ticks");
        assert!(
            none.is_empty(),
            "a viewer that has not seen it hears nothing"
        );
        assert!(stranger.ambient.is_empty());
    }

    #[test]
    fn a_phantom_trails_two_grey_specks_from_its_wingtips_each_tick() {
        for (yaw, size) in [(0.0, 0), (90.0, 0), (0.0, 4)] {
            let (world, id) = hovering(yaw, size);
            let offset = render::flap_offset(id);
            let mut viewer = Viewer::new(&world, 4);
            let scale = phantoms::scale(size);
            let reach = f64::from(phantoms::WIDTH * scale * 1.48);
            let mut lifts = Vec::new();
            for tick in 1..=30 {
                let before = viewer.ambient.len();
                viewer.tick(&world);
                let added: Vec<(Kind, DVec3)> = viewer.ambient.kinds_at().skip(before).collect();
                assert_eq!(added.len(), 2, "two specks a tick");
                assert!(added.iter().all(|(kind, _)| *kind == Kind::Mycelium));
                let (left, right) = (added[0].1, added[1].1);
                let beat = render::wing_beat(offset, tick);
                let lift = f64::from((0.3 + beat * 0.45) * phantoms::HEIGHT * scale * 2.5);
                assert!((left.y - FEET.y - lift).abs() < 1e-5, "{left} {lift}");
                assert_eq!(left.y, right.y);
                let middle = (left + right) / 2.0;
                assert!((middle.x - FEET.x).abs() < 1e-5 && (middle.z - FEET.z).abs() < 1e-5);
                let across = left - right;
                let expected = if yaw == 0.0 {
                    DVec3::new(2.0 * reach, 0.0, 0.0)
                } else {
                    DVec3::new(0.0, 0.0, 2.0 * reach)
                };
                assert!(across.abs_diff_eq(expected, 1e-4), "{across} {expected}");
                lifts.push(lift);
            }
            let low = lifts.iter().copied().fold(f64::MAX, f64::min);
            let high = lifts.iter().copied().fold(f64::MIN, f64::max);
            assert!(high - low > 0.4 * f64::from(scale), "{low} {high}");
        }
        let (mut world, id) = hovering(0.0, 0);
        let mut unseen = Viewer::new(&EntityWorld::default(), 5);
        unseen.tick(&world);
        assert!(
            unseen.ambient.is_empty(),
            "a phantom the client has not seen"
        );
        let mut viewer = Viewer::new(&world, 6);
        world.family_mob_mut::<Phantom>(id).unwrap().core.removed = true;
        for _ in 0..60 {
            viewer.tick(&world);
        }
        assert!(
            viewer.ambient.is_empty() && viewer.sounds.is_empty(),
            "a removed phantom"
        );
    }
}
