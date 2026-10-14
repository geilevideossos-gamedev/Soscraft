use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;
use crate::posed_cube::{Cube, Skin, append, part_pose};
use glam::{Mat4, Vec3};
use minecraftoss_entities::world::EvokerFangsEntity;
use std::f32::consts::PI;

pub const TEXTURE: &str = "minecraft:entity/illager/evoker_fangs";
pub const TEXTURE_SIZE: [f32; 2] = [64.0, 32.0];
const BASE: Cube = Cube::new([0.0, 0.0, 0.0], [10.0, 12.0, 10.0], [0.0, 0.0]);
const JAW: Cube = Cube::new([0.0, 0.0, 0.0], [4.0, 14.0, 8.0], [40.0, 0.0]);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FangsPose {
    pub root: Mat4,
    pub base: Mat4,
    pub upper_jaw: Mat4,
    pub lower_jaw: Mat4,
}

pub fn fangs_pose(bite: f32) -> FangsPose {
    let opened = (bite * 2.0).min(1.0);
    let opened = 1.0 - opened * opened * opened;
    let lift = (bite + minecraftoss_player::mth::sin(f64::from(bite * 2.7))) * 7.2;
    let scale = if bite > 0.9 { (1.0 - bite) / 0.1 } else { 1.0 };
    let root = Mat4::from_translation(Vec3::new(0.0, (24.0 - 20.0 * scale) / 16.0, 0.0))
        * Mat4::from_scale(Vec3::splat(scale));
    let base = part_pose([-5.0, 24.0 - lift, -5.0], [0.0; 3]);
    FangsPose {
        root,
        base,
        upper_jaw: part_pose([6.5, 0.0, 1.0], [0.0, 0.0, PI - opened * 0.35 * PI]),
        lower_jaw: part_pose([3.5, 0.0, 9.0], [0.0, PI, PI + opened * 0.35 * PI]),
    }
}

pub fn model_root(feet: Vec3, yaw: f32) -> Mat4 {
    Mat4::from_translation(feet)
        * Mat4::from_rotation_y((90.0 - yaw).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0))
}

pub fn append_model(mesh: &mut ChunkMesh, root: Mat4, skin: &Skin, pose: &FangsPose) {
    let base = root * pose.root * pose.base;
    append(mesh, base, skin, &BASE);
    append(mesh, base * pose.upper_jaw, skin, &JAW);
    append(mesh, base * pose.lower_jaw, skin, &JAW);
}

pub fn append_fangs<'a>(
    mesh: &mut ChunkMesh,
    fangs: impl IntoIterator<Item = &'a EvokerFangsEntity>,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let Ok(texture) = ResourceId::parse(TEXTURE) else {
        return;
    };
    let region = atlas.entity_region(&texture);
    for entity in fangs {
        let bite = entity.fangs.bite_progress(partial.clamp(0.0, 1.0));
        if bite == 0.0 {
            continue;
        }
        let feet = entity.fangs.position;
        let block = (
            feet.x.floor() as i32,
            feet.y.floor() as i32,
            feet.z.floor() as i32,
        );
        let skin = Skin {
            region,
            texture_size: TEXTURE_SIZE,
            sky: light.get(block) as f32,
            block: light.get_block(block) as f32,
            alpha: 1.0,
            shaded: true,
        };
        let root = model_root(feet.as_vec3(), entity.fangs.yaw);
        append_model(mesh, root, &skin, &fangs_pose(bite));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin() -> Skin {
        Skin {
            region: [0.0, 0.0, 1.0, 1.0],
            texture_size: TEXTURE_SIZE,
            sky: 15.0,
            block: 0.0,
            alpha: 1.0,
            shaded: true,
        }
    }

    fn top(bite: f32) -> f32 {
        let mut mesh = ChunkMesh::default();
        append_model(
            &mut mesh,
            model_root(Vec3::ZERO, 0.0),
            &skin(),
            &fangs_pose(bite),
        );
        mesh.vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::MIN, f32::max)
    }

    #[test]
    fn the_fangs_rise_out_of_the_ground_and_sink_back() {
        let mut mesh = ChunkMesh::default();
        append_model(
            &mut mesh,
            model_root(Vec3::ZERO, 0.0),
            &skin(),
            &fangs_pose(0.5),
        );
        assert_eq!(mesh.faces, 18, "a base and two jaws");
        assert!(top(0.5) > top(0.05), "{} {}", top(0.5), top(0.05));
        assert!(top(0.5) > 0.5, "{}", top(0.5));
        assert!(
            top(1.0).abs() < 0.01,
            "shrunk into the ground: {}",
            top(1.0)
        );
    }

    #[test]
    fn the_jaws_open_wide_then_snap_shut() {
        let open = fangs_pose(0.0);
        let shut = fangs_pose(0.6);
        let roll = |m: Mat4| {
            m.to_scale_rotation_translation()
                .1
                .to_euler(glam::EulerRot::ZYX)
                .0
        };
        assert!((roll(open.upper_jaw) - 2.042).abs() < 1e-3);
        assert!((roll(shut.upper_jaw).abs() - PI).abs() < 1e-3);
    }
}
