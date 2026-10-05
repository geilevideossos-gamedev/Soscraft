use crate::{
    cow_render::entity_shade,
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh, ShadowCaster, Vertex},
    pack::ResourceId,
    server::ServerOrb,
};
use glam::{DVec3, Vec3};
use minecraftoss_world::level::entity::orb_icon;

const TEXTURE: &str = "minecraft:entity/experience/experience_orb";
const SHEET_SIZE: f32 = 64.0;
const ICON_SIZE: u32 = 16;
const ICONS_PER_ROW: u32 = 4;
const LIFT: f32 = 0.1;
const SCALE: f32 = 0.3;
const QUAD_X: (f32, f32) = (-0.5, 0.5);
const QUAD_Y: (f32, f32) = (-0.25, 0.75);
const ALPHA: u8 = 128;
const BLUE_PHASE: f32 = 4.1887903;
const BLOCK_LIGHT_BOOST: u8 = 7;
const EYE_HEIGHT: f64 = 0.5 * 0.85;
const SHADOW_RADIUS: f32 = 0.15;
const SHADOW_STRENGTH: f32 = 0.75;
const PICKUP_TICKS: u8 = 3;
const TAKEN_PARTIAL_TICK: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct OrbPickup {
    pub orb: ServerOrb,
    pub age: u8,
    pub target: DVec3,
    pub previous_target: DVec3,
}

impl OrbPickup {
    pub fn new(orb: ServerOrb, target: DVec3) -> Self {
        Self {
            orb,
            age: 0,
            target,
            previous_target: target,
        }
    }

    pub fn position(&self, partial: f32) -> DVec3 {
        let partial = partial.clamp(0.0, 1.0);
        let progress = (f32::from(self.age) + partial) / f32::from(PICKUP_TICKS);
        let target = self.previous_target.lerp(self.target, f64::from(partial));
        DVec3::from_array(self.orb.position).lerp(target, f64::from(progress * progress))
    }
}

pub fn tick_pickups(pickups: &mut Vec<OrbPickup>, target: DVec3) {
    for pickup in pickups.iter_mut() {
        pickup.age += 1;
        pickup.previous_target = pickup.target;
        pickup.target = target;
    }
    pickups.retain(|pickup| pickup.age < PICKUP_TICKS);
}

pub fn colour(age_in_ticks: f32) -> [f32; 4] {
    let phase = age_in_ticks / 2.0;
    let sin = |angle: f32| minecraftoss_player::mth::sin(f64::from(angle));
    let red = ((sin(phase + 0.0) + 1.0) * 0.5 * 255.0) as i32;
    let blue = ((sin(phase + BLUE_PHASE) + 1.0) * 0.1 * 255.0) as i32;
    [
        red as f32 / 255.0,
        1.0,
        blue as f32 / 255.0,
        f32::from(ALPHA) / 255.0,
    ]
}

pub fn icon_uv(icon: u32) -> [f32; 4] {
    let column = icon % ICONS_PER_ROW * ICON_SIZE;
    let row = icon / ICONS_PER_ROW * ICON_SIZE;
    [
        column as f32 / SHEET_SIZE,
        row as f32 / SHEET_SIZE,
        (column + ICON_SIZE) as f32 / SHEET_SIZE,
        (row + ICON_SIZE) as f32 / SHEET_SIZE,
    ]
}

pub fn block_light(level: u8) -> u8 {
    level.saturating_add(BLOCK_LIGHT_BOOST).min(15)
}

fn interpolated(orb: &ServerOrb, partial: f32) -> DVec3 {
    DVec3::from_array(orb.previous_position)
        .lerp(DVec3::from_array(orb.position), f64::from(partial))
}

struct OrbSprite {
    at: DVec3,
    lit_at: DVec3,
    age_in_ticks: f32,
    value: i32,
}

pub fn append_orbs(
    mesh: &mut ChunkMesh,
    orbs: &[ServerOrb],
    pickups: &[OrbPickup],
    partial: f32,
    atlas: &Atlas,
    forward: Vec3,
    light: &SkyLight,
) {
    let Ok(texture) = ResourceId::parse(TEXTURE) else {
        return;
    };
    if !atlas.contains(&texture) {
        return;
    }
    let [a0, b0, a1, b1] = atlas.entity_region(&texture);
    let partial = partial.clamp(0.0, 1.0);
    let right = forward.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
    let up = right.cross(forward).try_normalize().unwrap_or(Vec3::Y);
    let shade = entity_shade(up);
    let live = orbs.iter().map(|orb| {
        let at = interpolated(orb, partial);
        OrbSprite {
            at,
            lit_at: at,
            age_in_ticks: orb.tick_count as f32 + partial,
            value: orb.value,
        }
    });
    let taken = pickups.iter().map(|pickup| OrbSprite {
        at: pickup.position(partial),
        lit_at: DVec3::from_array(pickup.orb.position),
        age_in_ticks: pickup.orb.tick_count as f32 + TAKEN_PARTIAL_TICK,
        value: pickup.orb.value,
    });
    for sprite in live.chain(taken) {
        let probe = sprite.lit_at + DVec3::Y * EYE_HEIGHT;
        let cell = (
            probe.x.floor() as i32,
            probe.y.floor() as i32,
            probe.z.floor() as i32,
        );
        let sky = f32::from(light.get(cell));
        let block = f32::from(block_light(light.get_block(cell)));
        let [red, green, blue, alpha] = colour(sprite.age_in_ticks);
        let color = [red * shade, green * shade, blue * shade, alpha];
        let [c0, d0, c1, d1] = icon_uv(orb_icon(sprite.value));
        let (u0, u1) = (a0 + (a1 - a0) * c0, a0 + (a1 - a0) * c1);
        let (v0, v1) = (b0 + (b1 - b0) * d0, b0 + (b1 - b0) * d1);
        let center = sprite.at.as_vec3() + Vec3::Y * LIFT;
        let start = mesh.vertices.len() as u32;
        for (x, y, uv) in [
            (QUAD_X.0, QUAD_Y.0, [u0, v1]),
            (QUAD_X.1, QUAD_Y.0, [u1, v1]),
            (QUAD_X.1, QUAD_Y.1, [u1, v0]),
            (QUAD_X.0, QUAD_Y.1, [u0, v0]),
        ] {
            let point = center + (right * x + up * y) * SCALE;
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                uv,
                color,
                sky_light: sky,
                block_light: block,
            });
        }
        mesh.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
}

pub fn shadow_casters(orbs: &[ServerOrb], camera: DVec3, partial: f32) -> Vec<ShadowCaster> {
    let partial = partial.clamp(0.0, 1.0);
    orbs.iter()
        .map(|orb| ShadowCaster {
            position: interpolated(orb, partial),
            distance_squared: DVec3::from_array(orb.position).distance_squared(camera),
            radius: SHADOW_RADIUS,
            strength: SHADOW_STRENGTH,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orb(value: i32, previous: [f64; 3], position: [f64; 3]) -> ServerOrb {
        ServerOrb {
            id: 1,
            position,
            previous_position: previous,
            value,
            tick_count: 0,
        }
    }

    #[test]
    fn icons_index_the_four_by_four_sheet() {
        assert_eq!(icon_uv(0), [0.0, 0.0, 0.25, 0.25]);
        assert_eq!(icon_uv(3), [0.75, 0.0, 1.0, 0.25]);
        assert_eq!(icon_uv(4), [0.0, 0.25, 0.25, 0.5]);
        assert_eq!(icon_uv(10), [0.5, 0.5, 0.75, 0.75]);
        assert_eq!(orb_icon(1), 0);
        assert_eq!(orb_icon(7), 2);
        assert_eq!(orb_icon(2477), 10);
    }

    #[test]
    fn colour_pulses_red_against_a_little_blue() {
        let start = colour(0.0);
        assert_eq!(start[0], 127.0 / 255.0);
        assert_eq!(start[1], 1.0);
        assert_eq!(start[3], 128.0 / 255.0);
        let blue =
            ((minecraftoss_player::mth::sin(f64::from(BLUE_PHASE)) + 1.0) * 0.1 * 255.0) as i32;
        assert_eq!(start[2], blue as f32 / 255.0);
        let peak = colour(std::f32::consts::PI);
        assert!(peak[0] > 0.99, "{peak:?}");
        let trough = colour(3.0 * std::f32::consts::PI);
        assert!(trough[0] < 0.01, "{trough:?}");
        for age in 0..200 {
            let [red, green, blue, alpha] = colour(age as f32 * 0.37);
            assert!((0.0..=1.0).contains(&red));
            assert_eq!(green, 1.0);
            assert!((0.0..=51.0 / 255.0).contains(&blue));
            assert_eq!(alpha, 128.0 / 255.0);
        }
    }

    #[test]
    fn orbs_glow_seven_levels_brighter() {
        assert_eq!(block_light(0), 7);
        assert_eq!(block_light(5), 12);
        assert_eq!(block_light(8), 15);
        assert_eq!(block_light(15), 15);
    }

    fn orb_atlas(with_sheet: bool) -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let sheet = root.join("assets/minecraft/textures/entity/experience/experience_orb.png");
        std::fs::create_dir_all(sheet.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(64, 64, image::Rgba([90, 200, 40, 255]))
            .save(sheet)
            .unwrap();
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let textures = if with_sheet {
            vec![ResourceId::parse(TEXTURE).unwrap()]
        } else {
            Vec::new()
        };
        crate::mesh::make_atlas(&packs, textures).unwrap()
    }

    #[test]
    fn an_orb_is_a_lifted_camera_facing_quad_of_its_icon() {
        let atlas = orb_atlas(true);
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![9], vec![3]);
        let mut mesh = ChunkMesh::default();
        let orbs = [ServerOrb {
            tick_count: 6,
            ..orb(7, [0.5, 64.0, 0.5], [0.5, 64.0, 0.5])
        }];
        append_orbs(&mut mesh, &orbs, &[], 0.5, &atlas, Vec3::Z, &light);
        assert_eq!(
            (mesh.vertices.len(), mesh.indices, mesh.faces),
            (4, vec![0, 1, 2, 0, 2, 3], 1)
        );
        let [a0, b0, a1, b1] = atlas.entity_region(&ResourceId::parse(TEXTURE).unwrap());
        let u = |cell: f32| a0 + (a1 - a0) * cell;
        let v = |cell: f32| b0 + (b1 - b0) * cell;
        let expected = [
            ([0.65, 64.025, 0.5], [u(0.5), v(0.25)]),
            ([0.35, 64.025, 0.5], [u(0.75), v(0.25)]),
            ([0.35, 64.325, 0.5], [u(0.75), v(0.0)]),
            ([0.65, 64.325, 0.5], [u(0.5), v(0.0)]),
        ];
        for (vertex, (position, uv)) in mesh.vertices.iter().zip(expected) {
            let offset = Vec3::from_array(vertex.position) - Vec3::from_array(position);
            assert!(offset.length() < 1e-4, "{vertex:?}");
            assert_eq!(vertex.uv, uv);
            assert_eq!(vertex.color, colour(6.5));
            assert_eq!((vertex.sky_light, vertex.block_light), (9.0, 10.0));
        }
    }

    #[test]
    fn no_sheet_no_orbs() {
        let atlas = orb_atlas(false);
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![15], vec![0]);
        let mut mesh = ChunkMesh::default();
        append_orbs(
            &mut mesh,
            &[orb(1, [0.0; 3], [0.0; 3])],
            &[],
            0.0,
            &atlas,
            Vec3::Z,
            &light,
        );
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn shadows_follow_the_interpolated_feet() {
        let orbs = [orb(3, [0.0, 64.0, 0.0], [1.0, 64.0, 0.0])];
        let casters = shadow_casters(&orbs, DVec3::new(1.0, 66.0, 0.0), 0.25);
        assert_eq!(casters.len(), 1);
        assert_eq!(casters[0].position, DVec3::new(0.25, 64.0, 0.0));
        assert_eq!(casters[0].distance_squared, 4.0);
        assert_eq!(
            (casters[0].radius, casters[0].strength),
            (SHADOW_RADIUS, SHADOW_STRENGTH)
        );
    }

    #[test]
    fn looking_down_shades_orbs_by_the_camera_up_normal() {
        let atlas = orb_atlas(true);
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![15], vec![0]);
        let mut mesh = ChunkMesh::default();
        let forward = Vec3::new(0.01, -1.0, 0.0).normalize();
        append_orbs(
            &mut mesh,
            &[orb(3, [0.5, 64.0, 0.5], [0.5, 64.0, 0.5])],
            &[],
            0.0,
            &atlas,
            forward,
            &light,
        );
        let up = forward
            .cross(Vec3::Y)
            .normalize()
            .cross(forward)
            .normalize();
        let shade = entity_shade(up);
        assert!((0.45..0.55).contains(&shade), "{shade}");
        let [red, green, blue, alpha] = colour(0.0);
        assert_eq!(mesh.vertices.len(), 4);
        for vertex in &mesh.vertices {
            assert_eq!(
                vertex.color,
                [red * shade, green * shade, blue * shade, alpha]
            );
        }
    }

    #[test]
    fn a_taken_orb_flies_into_the_player_over_three_ticks() {
        let start = [0.0, 64.0, 0.0];
        let mut pickups = vec![OrbPickup::new(
            orb(3, start, start),
            DVec3::new(3.0, 65.0, 0.0),
        )];
        assert_eq!(pickups[0].position(0.0), DVec3::from_array(start));
        tick_pickups(&mut pickups, DVec3::new(6.0, 65.0, 0.0));
        let halfway = pickups[0].position(0.5);
        assert!(
            (halfway - DVec3::new(1.125, 64.25, 0.0)).length() < 1e-9,
            "{halfway}"
        );
        tick_pickups(&mut pickups, DVec3::new(6.0, 65.0, 0.0));
        assert_eq!(pickups.len(), 1);
        tick_pickups(&mut pickups, DVec3::new(6.0, 65.0, 0.0));
        assert!(pickups.is_empty());
    }

    #[test]
    fn a_taken_orb_keeps_the_look_it_had_when_taken() {
        let atlas = orb_atlas(true);
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![9], vec![3]);
        let taken = ServerOrb {
            tick_count: 6,
            ..orb(7, [0.5, 64.0, 0.5], [0.5, 64.0, 0.5])
        };
        let mut pickups = vec![OrbPickup::new(taken, DVec3::new(5.5, 64.0, 0.5))];
        tick_pickups(&mut pickups, DVec3::new(5.5, 64.0, 0.5));
        tick_pickups(&mut pickups, DVec3::new(5.5, 64.0, 0.5));
        let mut mesh = ChunkMesh::default();
        append_orbs(&mut mesh, &[], &pickups, 0.75, &atlas, Vec3::Z, &light);
        assert_eq!(mesh.vertices.len(), 4);
        let at = pickups[0].position(0.75);
        assert!(at.x > 3.0, "{at}");
        let first = Vec3::from_array(mesh.vertices[0].position);
        let expected = at.as_vec3() + Vec3::new(0.15, 0.025, 0.0);
        assert!((first - expected).length() < 1e-4, "{first}");
        for vertex in &mesh.vertices {
            assert_eq!(vertex.color, colour(7.0));
            assert_eq!((vertex.sky_light, vertex.block_light), (9.0, 10.0));
        }
    }
}
