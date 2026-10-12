use glam::{Mat4, Vec3};

use crate::armor_render::{ArmorPanel, cube_posed_slice};
use crate::mesh::{Atlas, ChunkMesh};
use crate::pack::ResourceId;

pub const WINGS_TEXTURE: &str = "minecraft:entity/equipment/wings/elytra";
pub const ELYTRA: &str = "minecraft:elytra";
const TICK_SECONDS: f32 = 0.05;
const EASE: f32 = 0.3;
const MAX_TICKS_A_FRAME: u32 = 100;
const TICK_SLACK: f32 = 1.0e-4;
const ROLL_THRESHOLD: f64 = 1.0e-5;
const WING_UV: [f32; 2] = [22.0, 0.0];
const WING_SIZE: [f32; 3] = [10.0, 20.0, 2.0];
const WING_GROW: f32 = 1.0;
const WING_PIVOT_X: f32 = 5.0;
const WINGS_BACK: f32 = 2.0;
const WHITE: [f32; 3] = [1.0; 3];

pub fn wings_texture(item: &str) -> Option<ResourceId> {
    if item != ELYTRA {
        return None;
    }
    ResourceId::parse(WINGS_TEXTURE).ok()
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WingAngles {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl WingAngles {
    pub const REST: Self = Self {
        x: 0.2617994,
        y: 0.0,
        z: -0.2617994,
    };
    pub const CROUCH: Self = Self {
        x: 0.6981317,
        y: 0.08726646,
        z: -0.7853982,
    };
    pub const SPREAD: Self = Self {
        x: 0.34906584,
        y: 0.0,
        z: -1.5707964,
    };

    pub fn target(gliding: bool, crouching: bool, motion: [f64; 3]) -> Self {
        if gliding {
            let k = dive_openness(motion);
            Self {
                x: lerp(k, Self::REST.x, Self::SPREAD.x),
                y: 0.0,
                z: lerp(k, Self::REST.z, Self::SPREAD.z),
            }
        } else if crouching {
            Self::CROUCH
        } else {
            Self::REST
        }
    }

    pub fn lerp(self, to: Self, t: f32) -> Self {
        Self {
            x: lerp(t, self.x, to.x),
            y: lerp(t, self.y, to.y),
            z: lerp(t, self.z, to.z),
        }
    }

    fn eased(self, target: Self) -> Self {
        Self {
            x: self.x + (target.x - self.x) * EASE,
            y: self.y + (target.y - self.y) * EASE,
            z: self.z + (target.z - self.z) * EASE,
        }
    }
}

fn lerp(t: f32, from: f32, to: f32) -> f32 {
    from + t * (to - from)
}

fn dive_openness(motion: [f64; 3]) -> f32 {
    if motion[2] >= 0.0 {
        return 1.0;
    }
    let length = (motion[0] * motion[0] + motion[1] * motion[1] + motion[2] * motion[2]).sqrt();
    if length < ROLL_THRESHOLD {
        return 1.0;
    }
    1.0 - (-motion[2] / length).powf(1.5) as f32
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ElytraAnim {
    pub old: WingAngles,
    pub now: WingAngles,
    pub fall_fly_ticks: u32,
    pub partial: f32,
    pub gliding: bool,
}

impl ElytraAnim {
    pub fn advance(&mut self, seconds: f32, gliding: bool, crouching: bool, motion: [f64; 3]) {
        self.gliding = gliding;
        let ticks = self.partial + (seconds.max(0.0) / TICK_SECONDS);
        if !ticks.is_finite() {
            return;
        }
        let whole = (ticks + TICK_SLACK).floor();
        self.partial = (ticks - whole).max(0.0);
        for _ in 0..(whole as u32).min(MAX_TICKS_A_FRAME) {
            self.tick(gliding, crouching, motion);
        }
    }

    pub fn tick(&mut self, gliding: bool, crouching: bool, motion: [f64; 3]) {
        self.fall_fly_ticks = if gliding {
            self.fall_fly_ticks.saturating_add(1)
        } else {
            0
        };
        self.old = self.now;
        self.now = self
            .now
            .eased(WingAngles::target(gliding, crouching, motion));
    }

    pub fn wings(&self) -> WingAngles {
        self.old.lerp(self.now, self.partial)
    }

    pub fn scale(&self) -> f32 {
        if self.gliding {
            fall_flying_scale(self.fall_fly_ticks as f32 + self.partial)
        } else {
            0.0
        }
    }
}

pub fn fall_flying_scale(ticks: f32) -> f32 {
    (ticks * ticks / 100.0).clamp(0.0, 1.0)
}

pub fn flying_roll(look: [f64; 3], motion: [f64; 3]) -> Option<f32> {
    let along = motion[0] * motion[0] + motion[1] * motion[1];
    let view = look[0] * look[0] + look[1] * look[1];
    if along <= ROLL_THRESHOLD || view <= ROLL_THRESHOLD {
        return None;
    }
    let dot = (motion[0] * look[0] + motion[1] * look[1]) / (along * view).sqrt();
    let side = look[0] * motion[1] - look[1] * motion[0];
    Some((side.signum() * dot.abs().min(1.0).acos()) as f32)
}

fn wing_frame(frame: Mat4, side: f32, wings: WingAngles) -> Mat4 {
    frame
        * Mat4::from_translation(Vec3::new(WING_PIVOT_X * side, 0.0, WINGS_BACK))
        * Mat4::from_rotation_z(wings.z * side)
        * Mat4::from_rotation_y(wings.y * side)
        * Mat4::from_rotation_x(wings.x)
}

pub fn append_elytra(
    mesh: &mut ChunkMesh,
    atlas: &Atlas,
    frame: Mat4,
    wings: WingAngles,
    glint: bool,
    sky: f32,
    block: f32,
) {
    let Ok(id) = ResourceId::parse(WINGS_TEXTURE) else {
        return;
    };
    if !atlas.contains(&id) {
        return;
    }
    let panel = ArmorPanel {
        region: atlas.entity_region(&id),
        tint: WHITE,
        sky,
        block,
    };
    let start = mesh.vertices.len() as u32;
    let whole = (f32::NEG_INFINITY, f32::INFINITY);
    cube_posed_slice(
        mesh,
        wing_frame(frame, 1.0, wings),
        &panel,
        [-WING_SIZE[0], 0.0, 0.0],
        WING_SIZE,
        WING_GROW,
        WING_UV,
        whole,
        false,
    );
    cube_posed_slice(
        mesh,
        wing_frame(frame, -1.0, wings),
        &panel,
        [0.0, 0.0, 0.0],
        WING_SIZE,
        WING_GROW,
        WING_UV,
        whole,
        true,
    );
    let end = mesh.vertices.len() as u32;
    if glint && end > start {
        mesh.armor_glint.push(start..end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    fn angles_close(a: WingAngles, b: WingAngles) -> bool {
        close(a.x, b.x) && close(a.y, b.y) && close(a.z, b.z)
    }

    #[test]
    fn only_the_elytra_has_wings() {
        assert_eq!(
            wings_texture("minecraft:elytra"),
            Some(ResourceId::parse(WINGS_TEXTURE).unwrap())
        );
        assert_eq!(wings_texture("minecraft:diamond_chestplate"), None);
        assert_eq!(wings_texture("minecraft:feather"), None);
        assert_eq!(wings_texture("elytra"), None);
    }

    #[test]
    fn the_wings_fold_at_15_degrees_spread_to_90_and_crouch_at_40_and_45() {
        assert!(angles_close(
            WingAngles::target(false, false, [0.0; 3]),
            WingAngles::REST
        ));
        assert!(close(WingAngles::REST.x.to_degrees(), 15.0));
        assert!(angles_close(
            WingAngles::target(false, true, [0.0, 0.0, -1.0]),
            WingAngles::CROUCH
        ));
        assert!(close(WingAngles::CROUCH.x.to_degrees(), 40.0));
        assert!(close(WingAngles::CROUCH.z.to_degrees(), -45.0));
        assert!(close(WingAngles::CROUCH.y.to_degrees(), 5.0));
        let level = WingAngles::target(true, false, [1.5, 0.0, 0.0]);
        assert!(angles_close(level, WingAngles::SPREAD));
        assert!(close(level.z.to_degrees(), -90.0));
        assert!(angles_close(
            WingAngles::target(true, true, [1.0, 0.5, 0.8]),
            WingAngles::SPREAD
        ));
        assert!(angles_close(
            WingAngles::target(true, false, [0.0, 0.0, -3.0]),
            WingAngles::REST
        ));
        let k = 1.0 - 0.5f32.sqrt().powf(1.5);
        assert!(close(k, 0.4054), "{k}");
        let dive = WingAngles::target(true, false, [1.0, 0.0, -1.0]);
        assert!(close(dive.x, 0.2617994 + k * (0.34906584 - 0.2617994)));
        assert!(close(dive.z, -0.2617994 + k * (-1.5707964 + 0.2617994)));
        assert_eq!(dive.y, 0.0);
        assert!(angles_close(
            WingAngles::target(true, false, [0.0, 0.0, -1.0e-7]),
            WingAngles::SPREAD
        ));
    }

    #[test]
    fn the_wings_ease_three_tenths_a_tick_from_zero() {
        let mut anim = ElytraAnim::default();
        assert_eq!(anim.wings(), WingAngles::default());
        anim.advance(0.05, false, false, [0.0; 3]);
        assert!(close(anim.now.x, 0.0785398), "{:?}", anim.now);
        assert!(close(anim.now.z, -0.0785398));
        assert!(close(anim.partial, 0.0));
        assert!(angles_close(anim.wings(), anim.old));
        anim.advance(0.025, false, false, [0.0; 3]);
        let half = anim.wings();
        assert!(close(half.x, 0.5 * 0.0785398), "{half:?}");
        let mut whole = ElytraAnim::default();
        let mut split = ElytraAnim::default();
        for _ in 0..4 {
            whole.advance(0.05, true, false, [1.0, 0.0, 0.0]);
            split.advance(0.03, true, false, [1.0, 0.0, 0.0]);
            split.advance(0.02, true, false, [1.0, 0.0, 0.0]);
        }
        assert!(angles_close(whole.now, split.now), "{whole:?} {split:?}");
        assert!(angles_close(whole.wings(), split.wings()));
        assert_eq!(whole.fall_fly_ticks, split.fall_fly_ticks);
        let mut long = ElytraAnim::default();
        long.advance(1.0e9, true, false, [1.0, 0.0, 0.0]);
        assert!(angles_close(long.now, WingAngles::SPREAD));
        long.advance(f32::NAN, true, false, [1.0, 0.0, 0.0]);
        assert!(angles_close(long.now, WingAngles::SPREAD));
    }

    #[test]
    fn fall_flying_scale_is_ticks_squared_over_100() {
        assert!(close(fall_flying_scale(5.0), 0.25));
        assert!(close(fall_flying_scale(10.0), 1.0));
        assert!(close(fall_flying_scale(20.0), 1.0));
        assert_eq!(fall_flying_scale(0.0), 0.0);
        let mut anim = ElytraAnim::default();
        for _ in 0..5 {
            anim.advance(0.05, true, false, [1.0, 0.0, 0.0]);
        }
        assert_eq!(anim.fall_fly_ticks, 5);
        assert!(close(anim.scale(), 0.25));
        anim.advance(0.025, true, false, [1.0, 0.0, 0.0]);
        assert!(close(anim.scale(), 5.5 * 5.5 / 100.0));
        for _ in 0..20 {
            anim.advance(0.05, true, false, [1.0, 0.0, 0.0]);
        }
        assert!(close(anim.scale(), 1.0));
        anim.advance(0.01, false, false, [1.0, 0.0, 0.0]);
        assert_eq!(anim.scale(), 0.0);
        anim.advance(0.05, false, false, [1.0, 0.0, 0.0]);
        assert_eq!(anim.fall_fly_ticks, 0);
    }

    #[test]
    fn the_flying_roll_is_the_signed_angle_between_motion_and_look() {
        let look = [1.0, 0.0, -0.3];
        let at = |degrees: f64| {
            let radians = degrees.to_radians();
            [radians.cos(), radians.sin(), -0.5]
        };
        let roll = |motion| flying_roll(look, motion).unwrap();
        assert!(close(roll(at(30.0)), 0.5235988), "{}", roll(at(30.0)));
        assert!(close(roll(at(-30.0)), -0.5235988));
        assert!(close(roll(at(90.0)), 1.5707964));
        assert!(close(roll(at(-90.0)), -1.5707964));
        assert!(close(roll(at(180.0)).abs(), 0.0));
        assert!(close(roll(at(150.0)), 0.5235988));
        assert_eq!(flying_roll(look, [0.0, 0.0, -2.0]), None);
        assert_eq!(flying_roll([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]), None);
    }

    fn wings_atlas() -> (crate::test_dir::TestDir, Atlas) {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let path = root.join("assets/minecraft/textures/entity/equipment/wings/elytra.png");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbaImage::from_pixel(64, 32, image::Rgba([150, 150, 160, 255]))
            .save(path)
            .unwrap();
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        let atlas =
            crate::mesh::make_atlas(&packs, vec![ResourceId::parse(WINGS_TEXTURE).unwrap()])
                .unwrap();
        (temp, atlas)
    }

    fn bounds(mesh: &ChunkMesh, range: std::ops::Range<usize>) -> (Vec3, Vec3) {
        mesh.vertices[range].iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(lo, hi), vertex| {
                let at = Vec3::from_array(vertex.position);
                (lo.min(at), hi.max(at))
            },
        )
    }

    #[test]
    fn an_elytra_is_two_mirrored_wings_on_the_back() {
        let (_temp, atlas) = wings_atlas();
        let zero = WingAngles::default();
        let mut mesh = ChunkMesh::default();
        append_elytra(&mut mesh, &atlas, Mat4::IDENTITY, zero, false, 15.0, 4.0);
        assert_eq!(mesh.faces, 12);
        assert!(mesh.armor_glint.is_empty() && mesh.glint.is_empty());
        let region = atlas.entity_region(&ResourceId::parse(WINGS_TEXTURE).unwrap());
        let (u0, u1) = (
            region[0] + (region[2] - region[0]) * 22.0 / 64.0,
            region[0] + (region[2] - region[0]) * 46.0 / 64.0,
        );
        let (v0, v1) = (region[1], region[1] + (region[3] - region[1]) * 22.0 / 32.0);
        assert!(mesh.vertices.iter().all(|vertex| {
            vertex.uv[0] >= u0 - 1e-6
                && vertex.uv[0] <= u1 + 1e-6
                && vertex.uv[1] >= v0 - 1e-6
                && vertex.uv[1] <= v1 + 1e-6
        }));
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| (vertex.sky_light, vertex.block_light) == (15.0, 4.0))
        );
        let half = mesh.vertices.len() / 2;
        for range in [0..half, half..mesh.vertices.len()] {
            let (lo, hi) = bounds(&mesh, range);
            assert!(lo.abs_diff_eq(Vec3::new(-6.0, -1.0, 1.0), 1e-5), "{lo}");
            assert!(hi.abs_diff_eq(Vec3::new(6.0, 21.0, 5.0), 1e-5), "{hi}");
        }
        for right in &mesh.vertices[half..] {
            let mirrored = Vec3::new(-right.position[0], right.position[1], right.position[2]);
            assert!(
                mesh.vertices[..half].iter().any(|left| {
                    Vec3::from_array(left.position).abs_diff_eq(mirrored, 1e-5)
                        && (left.uv[0] - right.uv[0]).abs() < 1e-6
                        && (left.uv[1] - right.uv[1]).abs() < 1e-6
                }),
                "the right wing is the left one mirrored, texture and all"
            );
        }
        let centre = Vec3::new(0.0, 10.0, 3.0);
        for face in 0..12 {
            let corner = |i: usize| {
                Vec3::from_array(mesh.vertices[mesh.indices[face * 6 + i] as usize].position)
            };
            let middle = (0..4)
                .map(|i| Vec3::from_array(mesh.vertices[face * 4 + i].position))
                .sum::<Vec3>()
                / 4.0;
            let normal = (corner(1) - corner(0)).cross(corner(2) - corner(0));
            assert!(normal.dot(middle - centre) > 0.0, "face {face} faces out");
        }
    }

    #[test]
    fn the_wings_turn_out_from_their_shoulder_pivots() {
        let (_temp, atlas) = wings_atlas();
        let mut mesh = ChunkMesh::default();
        append_elytra(
            &mut mesh,
            &atlas,
            Mat4::IDENTITY,
            WingAngles::SPREAD,
            false,
            15.0,
            0.0,
        );
        let half = mesh.vertices.len() / 2;
        let (left_lo, left_hi) = bounds(&mesh, 0..half);
        let (right_lo, right_hi) = bounds(&mesh, half..mesh.vertices.len());
        assert!(left_hi.x > 20.0 && left_lo.x > 0.0, "{left_lo} {left_hi}");
        assert!(
            right_lo.x < -20.0 && right_hi.x < 0.0,
            "{right_lo} {right_hi}"
        );
        assert!(close(left_hi.x, -right_lo.x) && close(left_lo.y, right_lo.y));
        let mut rest = ChunkMesh::default();
        append_elytra(
            &mut rest,
            &atlas,
            Mat4::IDENTITY,
            WingAngles::REST,
            false,
            15.0,
            0.0,
        );
        let (lo, hi) = bounds(&rest, 0..rest.vertices.len());
        assert!(hi.x > 6.0 && lo.x < -6.0, "slightly open at the bottom");
        assert!(hi.z > 5.0, "the bottom swings back");
        let moved = Mat4::from_translation(Vec3::new(10.0, 64.0, 3.0));
        let mut placed = ChunkMesh::default();
        append_elytra(
            &mut placed,
            &atlas,
            moved,
            WingAngles::REST,
            false,
            15.0,
            0.0,
        );
        assert!(placed.vertices.iter().zip(&rest.vertices).all(|(a, b)| {
            (Vec3::from_array(a.position) - Vec3::from_array(b.position))
                .abs_diff_eq(Vec3::new(10.0, 64.0, 3.0), 1e-4)
        }));
    }

    #[test]
    fn an_enchanted_elytra_shimmers_with_the_armour_glint() {
        let (temp, atlas) = wings_atlas();
        let mut mesh = ChunkMesh::default();
        let draw = |mesh: &mut ChunkMesh, atlas: &Atlas, glint: bool| {
            append_elytra(
                mesh,
                atlas,
                Mat4::IDENTITY,
                WingAngles::REST,
                glint,
                15.0,
                0.0,
            )
        };
        draw(&mut mesh, &atlas, false);
        let plain = mesh.vertices.len() as u32;
        draw(&mut mesh, &atlas, true);
        assert_eq!(mesh.armor_glint, vec![plain..2 * plain]);
        assert!(mesh.glint.is_empty());
        let packs = crate::pack::PackStack::open(vec![temp.path().into()]).unwrap();
        let without = crate::mesh::make_atlas(&packs, Vec::new()).unwrap();
        let mut bare = ChunkMesh::default();
        draw(&mut bare, &without, true);
        assert_eq!(bare.faces, 0);
        assert!(bare.armor_glint.is_empty());
    }
}
