use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
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
}
