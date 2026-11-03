//! Pack-backed pinned 26.3 creeper: `CreeperModel.createBodyLayer` (a head
//! and body on four legs), its `setupAnim` (head turn and pitch, the legs
//! swinging in diagonal pairs with the walk), and `CreeperRenderer`'s
//! swell (`scale`: wider and slightly taller, with a wobble) and white fuse
//! flash (`getWhiteOverlayProgress`). Vanilla mixes the white overlay into
//! the texture; here the vertex colour brightens it, as for primed TNT.
use crate::{
    cow_render::cube_scaled,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh, Vertex},
    pack::ResourceId,
    client_mobs::{ClientMobs, MobPose},
};
use glam::{Quat, Vec3};
use minecraftoss_entities::{creeper::Creeper, world::CreeperEntity};

type Part = ([f32; 3], [f32; 3], [f32; 2], [f32; 3]);

const HEAD: Part = ([-4., -8., -4.], [4., 0., 4.], [0., 0.], [0., 6., 0.]);
const BODY: Part = ([-4., 0., -2.], [4., 12., 2.], [16., 16.], [0., 6., 0.]);
/// The legs' pivots, each with the phase of its swing: the model's
/// "right_hind_leg" and "left_front_leg" pair up, as do the other two.
const LEGS: [([f32; 3], f32); 4] = [
    ([-2., 18., 4.], std::f32::consts::PI),
    ([2., 18., 4.], 0.0),
    ([-2., 18., -4.], 0.0),
    ([2., 18., -4.], std::f32::consts::PI),
];
pub const POWER_TEXTURE: &str = "minecraft:entity/creeper/creeper_armor";
const POWER_GROWTH: f32 = 2.0;
const POWER_COLOUR: f32 = 128.0 / 255.0;

/// `CreeperRenderer.scale`: the swell widens the creeper by up to 40%
/// and heightens it by up to 10%, wobbling as it goes.
pub fn swell_scale(swelling: f32) -> Vec3 {
    let wobble = 1.0 + (swelling * 100.0).sin() * swelling * 0.01;
    let g = swelling.clamp(0.0, 1.0);
    let g = g * g;
    let g = g * g;
    let s = (1.0 + g * 0.4) * wobble;
    let hs = (1.0 + g * 0.1) / wobble;
    Vec3::new(s, hs, s)
}

/// `CreeperRenderer.getWhiteOverlayProgress`: white on alternate tenths of
/// the swell, from half strength.
pub fn white_overlay(swelling: f32) -> f32 {
    if (swelling * 10.0) as i32 % 2 == 0 {
        0.0
    } else {
        swelling.clamp(0.5, 1.0)
    }
}

pub fn append_creepers<'a>(
    mesh: &mut ChunkMesh,
    creepers: impl IntoIterator<Item = &'a CreeperEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    light: &SkyLight,
    partial: f32,
) {
    let texture = ResourceId::parse("minecraft:entity/creeper/creeper").unwrap();
    let region = atlas.entity_region(&texture);
    let partial = partial.clamp(0.0, 1.0);
    // Each mob's first vertex and overlay (`getOverlayCoords`).
    let mut marks = Vec::new();
    for entity in creepers {
        let creeper = &entity.creeper;
        if creeper.exploded {
            continue;
        }
        let Some(mob) = poses.pose(entity.id, partial) else { continue };
        let feet = mob.feet;
        let sample = mob.light_block();
        let (sky, block) = (light.get(sample) as f32, light.get_block(sample) as f32);
        let rotation = mob.body_rotation(90.0);
        let swelling = swelling_of(creeper, partial);
        let scale = swell_scale(swelling);
        // The fuse's white flash is the overlay's white rows.
        marks.push((mesh.vertices.len(), mob.overlay(white_overlay(swelling))));
        let tint = [1.0; 3];
        for ((from, to, uv, pivot), pose) in posed_parts(&mob) {
            cube_scaled(mesh, feet, rotation, scale, region, sky, block, from, to, uv, pivot, pose, tint, [64., 32.], None, false);
        }
    }
    crate::cow_render::apply_overlays(mesh, &marks);
}

fn swelling_of(creeper: &Creeper, partial: f32) -> f32 {
    let swell = creeper.old_swell as f32 + partial * (creeper.swell - creeper.old_swell) as f32;
    swell / (creeper.max_swell - 2) as f32
}

fn posed_parts(mob: &MobPose) -> Vec<(Part, Quat)> {
    let head = Quat::from_euler(glam::EulerRot::ZYX, 0.0, mob.head_yaw.to_radians(), mob.head_pitch.to_radians());
    let (swing, speed) = (mob.walk_position, mob.walk_speed);
    let mut parts: Vec<(Part, Quat)> = vec![(HEAD, head), (BODY, Quat::IDENTITY)];
    for (pivot, phase) in LEGS {
        let angle = crate::client_mobs::mth_cos(swing * 0.6662 + phase) * 1.4 * speed;
        parts.push((([-2., 0., -2.], [2., 6., 2.], [0., 16.], pivot), Quat::from_rotation_x(angle)));
    }
    parts
}

pub fn append_creeper_power<'a>(
    mesh: &mut ChunkMesh,
    creepers: impl IntoIterator<Item = &'a CreeperEntity>,
    poses: &ClientMobs,
    atlas: &Atlas,
    partial: f32,
) {
    let region = atlas.entity_region(&ResourceId::parse(POWER_TEXTURE).unwrap());
    let partial = partial.clamp(0.0, 1.0);
    for entity in creepers {
        let creeper = &entity.creeper;
        if creeper.exploded || !creeper.powered {
            continue;
        }
        let Some(mob) = poses.pose(entity.id, partial) else { continue };
        let rotation = mob.body_rotation(90.0);
        let scale = swell_scale(swelling_of(creeper, partial));
        let mut shell = ChunkMesh::default();
        for ((from, to, uv, pivot), pose) in posed_parts(&mob) {
            let size = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
            let (from, to) = (from.map(|c| c - POWER_GROWTH), to.map(|c| c + POWER_GROWTH));
            cube_scaled(&mut shell, mob.feet, rotation, scale, [0.0, 0.0, 1.0, 1.0], 15.0, 15.0, from, to, uv, pivot, pose, [1.0; 3], [64., 32.], Some(size), false);
        }
        let offset = (mob.age_in_ticks * 0.01) % 1.0;
        for quad in shell.vertices.as_chunks::<4>().0 {
            append_swirl_quad(mesh, quad, offset, region);
        }
    }
}

fn append_swirl_quad(mesh: &mut ChunkMesh, quad: &[Vertex], offset: f32, region: [f32; 4]) {
    let corner = |i: usize| (Vec3::from_array(quad[i].position), quad[i].uv);
    let (origin, [u, v]) = corner(1);
    let (along_u, [u_end, _]) = corner(0);
    let (along_v, [_, v_end]) = corner(2);
    let cuts = |a: f32, b: f32| {
        let (lo, hi) = (a.min(b) + offset, a.max(b) + offset);
        if lo < 1.0 && hi > 1.0 { vec![(lo, 1.0), (1.0, hi)] } else { vec![(lo, hi)] }
    };
    let at = |su: f32, sv: f32| {
        origin + (along_u - origin) * ((su - offset - u) / (u_end - u)) + (along_v - origin) * ((sv - offset - v) / (v_end - v))
    };
    for (u0, u1) in cuts(u, u_end) {
        for (v0, v1) in cuts(v, v_end) {
            let (wrap_u, wrap_v) = (((u0 + u1) * 0.5).floor(), ((v0 + v1) * 0.5).floor());
            let start = mesh.vertices.len() as u32;
            for (su, sv) in [(u0, v0), (u1, v0), (u1, v1), (u0, v1)] {
                mesh.vertices.push(Vertex {
                    position: at(su, sv).to_array(),
                    uv: [
                        region[0] + (region[2] - region[0]) * (su - wrap_u),
                        region[1] + (region[3] - region[1]) * (sv - wrap_v),
                    ],
                    color: [POWER_COLOUR, POWER_COLOUR, POWER_COLOUR, 1.0],
                    sky_light: 15.0,
                    block_light: 15.0,
                });
            }
            mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use minecraftoss_entities::world::EntityWorld;

    fn atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("pack.mcmeta"), r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#).unwrap();
        let skins = root.join("assets/minecraft/textures/entity/creeper");
        std::fs::create_dir_all(&skins).unwrap();
        for (name, shade) in [("creeper", 90), ("creeper_armor", 200)] {
            image::RgbaImage::from_pixel(64, 32, image::Rgba([shade, shade, shade, 255])).save(skins.join(format!("{name}.png"))).unwrap();
        }
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let ids = ["minecraft:entity/creeper/creeper", POWER_TEXTURE].map(|id| ResourceId::parse(id).unwrap());
        crate::mesh::make_atlas(&packs, ids.to_vec()).unwrap()
    }

    fn quad_area(mesh: &ChunkMesh) -> f32 {
        mesh.vertices
            .as_chunks::<4>()
            .0
            .iter()
            .map(|q| {
                let [a, b, c, d] = [0, 1, 2, 3].map(|i| Vec3::from_array(q[i].position));
                ((b - a).cross(c - a).length() + (c - a).cross(d - a).length()) * 0.5
            })
            .sum()
    }

    #[test]
    fn a_charged_creeper_wears_the_scrolling_energy_layer() {
        let atlas = atlas();
        let light = SkyLight::from_levels((-2, -1, -2), (5, 5, 5), vec![15; 125], vec![0; 125]);
        let feet = DVec3::new(0.5, 1.0, 0.5);
        let draw = |powered: bool, ticks: usize| {
            let mut world = EntityWorld::default();
            let mut creeper = Creeper::new(feet);
            creeper.powered = powered;
            world.spawn_creeper(creeper, true);
            let mut poses = ClientMobs::default();
            poses.spawn_missing(&crate::client_mobs::server_mobs(&world));
            for _ in 0..ticks {
                poses.tick();
            }
            let (mut body, mut swirl) = (ChunkMesh::default(), ChunkMesh::default());
            append_creepers(&mut body, world.creepers(), &poses, &atlas, &light, 0.0);
            append_creeper_power(&mut swirl, world.creepers(), &poses, &atlas, 0.0);
            (body, swirl)
        };
        let (plain_body, plain) = draw(false, 0);
        assert!(plain.vertices.is_empty() && plain.indices.is_empty(), "a plain creeper has no swirl");
        let (body, still) = draw(true, 0);
        assert_eq!(body.vertices, plain_body.vertices, "the charge leaves the body as it is");
        assert_eq!((still.faces, still.vertices.len(), still.indices.len()), (36, 144, 216), "six boxes of six faces at age 0");
        let armour = atlas.entity_region(&ResourceId::parse(POWER_TEXTURE).unwrap());
        let inside = |uv: [f32; 2]| uv[0] >= armour[0] - 1e-6 && uv[0] <= armour[2] + 1e-6 && uv[1] >= armour[1] - 1e-6 && uv[1] <= armour[3] + 1e-6;
        assert!(still.vertices.iter().all(|v| inside(v.uv)), "the swirl reads creeper_armor");
        assert!(still.vertices.iter().all(|v| v.color == [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0]), "0xFF808080, unshaded");
        let height = |mesh: &ChunkMesh| {
            let ys = mesh.vertices.iter().map(|v| v.position[1]);
            (ys.clone().fold(f32::MAX, f32::min), ys.fold(f32::MIN, f32::max))
        };
        let ((body_low, body_high), (swirl_low, swirl_high)) = (height(&body), height(&still));
        assert!((body_low - swirl_low - 2.0 / 16.0).abs() < 1e-4 && (swirl_high - body_high - 2.0 / 16.0).abs() < 1e-4, "two pixels out: {body_low} {body_high} {swirl_low} {swirl_high}");
        let (_, scrolled) = draw(true, 25);
        assert!(scrolled.faces > 36, "a quarter turn of the texture cuts faces where it repeats");
        assert!(scrolled.vertices.iter().all(|v| inside(v.uv)));
        assert!((quad_area(&scrolled) - quad_area(&still)).abs() < 1e-3, "the cut faces cover the same shell");
        let ((low, high), (still_low, still_high)) = (height(&scrolled), height(&still));
        assert!((low - still_low).abs() < 1e-4 && (high - still_high).abs() < 1e-4);
    }

    #[test]
    fn a_swirl_face_wraps_where_its_scrolled_texture_repeats() {
        let quad = [([2.0, 0.0, 0.0], [1.0, 0.0]), ([0.0, 0.0, 0.0], [0.5, 0.0]), ([0.0, 1.0, 0.0], [0.5, 0.5]), ([2.0, 1.0, 0.0], [1.0, 0.5])]
            .map(|(position, uv)| Vertex { position, uv, color: [1.0; 4], sky_light: 0.0, block_light: 0.0 });
        let corners = |mesh: &ChunkMesh| mesh.vertices.iter().map(|v| (v.position, v.uv)).collect::<Vec<_>>();
        let mut still = ChunkMesh::default();
        append_swirl_quad(&mut still, &quad, 0.0, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(corners(&still), vec![([0.0, 0.0, 0.0], [0.5, 0.0]), ([2.0, 0.0, 0.0], [1.0, 0.0]), ([2.0, 1.0, 0.0], [1.0, 0.5]), ([0.0, 1.0, 0.0], [0.5, 0.5])]);
        let mut cut = ChunkMesh::default();
        append_swirl_quad(&mut cut, &quad, 0.25, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(
            corners(&cut),
            vec![
                ([0.0, 0.0, 0.0], [0.75, 0.25]),
                ([1.0, 0.0, 0.0], [1.0, 0.25]),
                ([1.0, 1.0, 0.0], [1.0, 0.75]),
                ([0.0, 1.0, 0.0], [0.75, 0.75]),
                ([1.0, 0.0, 0.0], [0.0, 0.25]),
                ([2.0, 0.0, 0.0], [0.25, 0.25]),
                ([2.0, 1.0, 0.0], [0.25, 0.75]),
                ([1.0, 1.0, 0.0], [0.0, 0.75]),
            ],
            "split where the shifted u passes 1, the far piece reading the texture's left edge"
        );
        assert_eq!((cut.faces, cut.indices[6..].to_vec()), (2, vec![4, 5, 6, 4, 6, 7]));
        let mut placed = ChunkMesh::default();
        append_swirl_quad(&mut placed, &quad, 0.25, [0.5, 0.25, 0.75, 0.5]);
        assert_eq!(placed.vertices[4].uv, [0.5, 0.3125], "the texture's own coordinates map into its atlas region");
        let square = quad.map(|v| Vertex { uv: [v.uv[0] - 0.25, v.uv[1] + 0.25], ..v });
        let mut both = ChunkMesh::default();
        append_swirl_quad(&mut both, &square, 0.5, [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(both.faces, 4, "a face crossing both wraps is cut in four");
        assert!((quad_area(&both) - 2.0).abs() < 1e-5);
        assert!(both.vertices.iter().all(|v| (0.0..=1.0).contains(&v.uv[0]) && (0.0..=1.0).contains(&v.uv[1])));
    }

    #[test]
    fn the_fuse_swells_and_flashes() {
        assert_eq!(swell_scale(0.0), Vec3::ONE);
        let full = swell_scale(1.0);
        assert!(full.x > 1.35 && full.y > 1.05, "{full:?}");
        assert_eq!(white_overlay(0.05), 0.0);
        assert_eq!(white_overlay(0.15), 0.5);
        assert_eq!(white_overlay(0.95), 0.95);
    }
}
