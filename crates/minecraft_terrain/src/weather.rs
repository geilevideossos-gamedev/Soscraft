use crate::pack::{PackStack, ResourceId};
use crate::rain::Precipitation;
use crate::scene::BlockPos;
use anyhow::Result;
use glam::DVec3;
use image::RgbaImage;
use minecraftoss_player::rng::LegacyRandom;

pub const RAIN_ALPHA: f32 = 1.0;
pub const SNOW_ALPHA: f32 = 0.8;
pub const EDGE_ALPHA: f32 = 0.5;
pub const RAIN_TEXTURE: &str = "minecraft:environment/rain";
pub const SNOW_TEXTURE: &str = "minecraft:environment/snow";
const RAIN_TIME_MASK: i64 = 131071;
const SNOW_TIME_MASK: i64 = 511;
const V_PER_BLOCK: f32 = 0.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column {
    pub x: i32,
    pub z: i32,
    pub bottom: i32,
    pub top: i32,
    pub u_offset: f32,
    pub v_offset: f32,
    pub sky: u8,
    pub block: u8,
    pub snow: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WeatherVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub alpha: f32,
    pub sky: f32,
    pub block: f32,
    pub snow: f32,
}

pub fn column_seed(x: i32, z: i32) -> i32 {
    let a = x
        .wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45238971));
    let b = z
        .wrapping_mul(z)
        .wrapping_mul(418711)
        .wrapping_add(z.wrapping_mul(13761));
    a ^ b
}

fn rain_offset(x: i32, z: i32) -> i32 {
    x.wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45238971))
        .wrapping_add(z.wrapping_mul(z).wrapping_mul(418711))
        .wrapping_add(z.wrapping_mul(13761))
        & 255
}

pub fn rain_column_offsets(x: i32, z: i32, game_time: i64, partial: f32) -> (f32, f32) {
    let mut random = LegacyRandom::new(i64::from(column_seed(x, z)) as u64);
    let time = (game_time & RAIN_TIME_MASK) as i32;
    let speed = 3.0 + random.next_float();
    let v = -((time + rain_offset(x, z)) as f32 + partial) / 32.0 * speed;
    (0.0, v % 32.0)
}

pub fn snow_column_offsets(x: i32, z: i32, game_time: i64, partial: f32) -> (f32, f32) {
    let mut random = LegacyRandom::new(i64::from(column_seed(x, z)) as u64);
    let time = (game_time & RAIN_TIME_MASK) as i32 as f32 + partial;
    let u_base = random.next_double();
    let u = (u_base + f64::from(time * 0.01 * random.next_gaussian() as f32)) as f32;
    let v_base = random.next_double();
    let jitter = (v_base + f64::from(time * random.next_gaussian() as f32 * 0.001)) as f32;
    let v = -((game_time & SNOW_TIME_MASK) as f32 + partial) / 512.0;
    (u, v + jitter)
}

pub fn snow_light(level: u8) -> u8 {
    (level * 3 + 15) / 4
}

pub fn columns(
    camera: DVec3,
    radius: i32,
    game_time: i64,
    partial: f32,
    height: impl Fn(i32, i32) -> i32,
    precipitation: impl Fn(BlockPos) -> Precipitation,
    light: impl Fn(BlockPos) -> (u8, u8),
) -> Vec<Column> {
    let (cx, cy, cz) = (
        camera.x.floor() as i32,
        camera.y.floor() as i32,
        camera.z.floor() as i32,
    );
    let mut rain = Vec::new();
    let mut snow = Vec::new();
    for z in cz - radius..=cz + radius {
        for x in cx - radius..=cx + radius {
            let h = height(x, z);
            let bottom = (cy - radius).max(h);
            let top = (cy + radius).max(h);
            if top == bottom {
                continue;
            }
            let kind = precipitation((x, cy, z));
            if kind == Precipitation::None {
                continue;
            }
            let (sky, block) = light((x, cy.max(h), z));
            let column =
                |(u_offset, v_offset): (f32, f32), sky: u8, block: u8, snow: bool| Column {
                    x,
                    z,
                    bottom,
                    top,
                    u_offset,
                    v_offset,
                    sky,
                    block,
                    snow,
                };
            if kind == Precipitation::Rain {
                rain.push(column(
                    rain_column_offsets(x, z, game_time, partial),
                    sky,
                    block,
                    false,
                ));
            } else {
                snow.push(column(
                    snow_column_offsets(x, z, game_time, partial),
                    snow_light(sky),
                    snow_light(block),
                    true,
                ));
            }
        }
    }
    rain.extend(snow);
    rain
}

pub fn half_widths(dx: i32, dz: i32) -> Option<(f32, f32)> {
    if dx == 0 && dz == 0 {
        return None;
    }
    let (dx, dz) = (dx as f32, dz as f32);
    let length = (dx * dx + dz * dz).sqrt();
    Some((-dz / length / 2.0, dx / length / 2.0))
}

pub fn column_alpha(camera: DVec3, x: i32, z: i32, radius: i32, base: f32, rain_level: f32) -> f32 {
    let dx = (f64::from(x) + 0.5 - camera.x) as f32;
    let dz = (f64::from(z) + 0.5 - camera.z) as f32;
    let reach = (radius * radius) as f32;
    let t = ((dx * dx + dz * dz) / reach).min(1.0);
    (base + t * (EDGE_ALPHA - base)) * rain_level
}

pub fn mesh(
    columns: &[Column],
    camera: DVec3,
    radius: i32,
    rain_level: f32,
) -> (Vec<WeatherVertex>, Vec<u32>) {
    let (cx, cz) = (camera.x.floor() as i32, camera.z.floor() as i32);
    let mut vertices = Vec::with_capacity(columns.len() * 4);
    let mut indices = Vec::with_capacity(columns.len() * 6);
    for column in columns {
        let Some((sx, sz)) = half_widths(column.x - cx, column.z - cz) else {
            continue;
        };
        let base = if column.snow { SNOW_ALPHA } else { RAIN_ALPHA };
        let alpha = column_alpha(camera, column.x, column.z, radius, base, rain_level);
        let (x, z) = (column.x as f32 + 0.5, column.z as f32 + 0.5);
        let (top, bottom) = (column.top as f32, column.bottom as f32);
        let (u0, u1) = (column.u_offset, column.u_offset + 1.0);
        let v_top = bottom * V_PER_BLOCK + column.v_offset;
        let v_bottom = top * V_PER_BLOCK + column.v_offset;
        let first = vertices.len() as u32;
        let corner = |position: [f32; 3], uv: [f32; 2]| WeatherVertex {
            position,
            uv,
            alpha,
            sky: f32::from(column.sky),
            block: f32::from(column.block),
            snow: if column.snow { 1.0 } else { 0.0 },
        };
        vertices.push(corner([x - sx, top, z - sz], [u0, v_top]));
        vertices.push(corner([x + sx, top, z + sz], [u1, v_top]));
        vertices.push(corner([x + sx, bottom, z + sz], [u1, v_bottom]));
        vertices.push(corner([x - sx, bottom, z - sz], [u0, v_bottom]));
        indices.extend([first, first + 1, first + 2, first + 2, first + 3, first]);
    }
    (vertices, indices)
}

pub fn side_by_side(rain: &RgbaImage, snow: &RgbaImage) -> RgbaImage {
    let snow = if snow.dimensions() == rain.dimensions() {
        snow.clone()
    } else {
        image::imageops::resize(
            snow,
            rain.width(),
            rain.height(),
            image::imageops::FilterType::Nearest,
        )
    };
    let mut out = RgbaImage::new(rain.width() * 2, rain.height());
    image::imageops::replace(&mut out, rain, 0, 0);
    image::imageops::replace(&mut out, &snow, i64::from(rain.width()), 0);
    out
}

pub fn weather_texture(packs: &PackStack) -> Result<Option<RgbaImage>> {
    let load = |name: &str| -> Result<Option<RgbaImage>> {
        let id = ResourceId::parse(name)?;
        packs
            .texture(&id)?
            .map(|bytes| {
                Ok(
                    image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?
                        .to_rgba8(),
                )
            })
            .transpose()
    };
    match (load(RAIN_TEXTURE)?, load(SNOW_TEXTURE)?) {
        (Some(rain), Some(snow)) => Ok(Some(side_by_side(&rain, &snow))),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-4 * b.abs().max(1.0)
    }

    #[test]
    fn rain_columns_scroll_at_their_seeded_speed() {
        assert_eq!(column_seed(5, -3), 222591356);
        assert_eq!(column_seed(-12, 7), -560868638);
        assert_eq!(rain_offset(5, -3), 124);
        assert_eq!(rain_offset(-12, 7), 250);
        let mut random = LegacyRandom::new(i64::from(column_seed(5, -3)) as u64);
        assert!(close(3.0 + random.next_float(), 3.7006378));
        let (u, v) = rain_column_offsets(5, -3, 1000, 0.25);
        assert_eq!(u, 0.0);
        assert!(close(v, -2.0138092), "{v}");
        let (_, v) = rain_column_offsets(-12, 7, 200000, 0.0);
        assert!(close(v, -29.427246), "{v}");
        let (_, later) = rain_column_offsets(5, -3, 1001, 0.25);
        let step = 3.7006378 / 32.0;
        assert!(close(later, v_after(-2.0138092, step)), "{later}");
    }

    fn v_after(v: f32, step: f32) -> f32 {
        (v - step) % 32.0
    }

    #[test]
    fn snow_columns_drift_by_gaussians() {
        let (u, v) = snow_column_offsets(-7, 12, 123456, 0.5);
        assert!(close(u, -1068.1278), "{u}");
        assert!(close(v, -250.92334), "{v}");
        let (u, v) = snow_column_offsets(3, 3, 40, 0.0);
        assert!(close(u, 0.6006236), "{u}");
        assert!(close(v, 0.68376994), "{v}");
        assert_eq!(snow_light(15), 15);
        assert_eq!(snow_light(0), 3);
        assert_eq!(snow_light(7), 9);
    }

    #[test]
    fn quads_face_the_camera_and_fade_to_half_at_the_edge() {
        assert_eq!(half_widths(0, 0), None);
        let (sx, sz) = half_widths(3, 0).unwrap();
        assert!(close(sx, 0.0) && close(sz, 0.5));
        let (sx, sz) = half_widths(0, -4).unwrap();
        assert!(close(sx, 0.5) && close(sz, 0.0));
        let (sx, sz) = half_widths(3, 4).unwrap();
        assert!(close(sx, -0.4) && close(sz, 0.3));
        assert!(close(sx * 3.0 + sz * 4.0, 0.0));
        let camera = DVec3::new(0.5, 70.0, 0.5);
        assert!(close(column_alpha(camera, 0, 0, 10, RAIN_ALPHA, 1.0), 1.0));
        assert!(close(column_alpha(camera, 0, 0, 10, SNOW_ALPHA, 1.0), 0.8));
        assert!(close(column_alpha(camera, 10, 0, 10, RAIN_ALPHA, 1.0), 0.5));
        assert!(close(
            column_alpha(camera, 10, 10, 10, SNOW_ALPHA, 1.0),
            0.5
        ));
        assert!(close(
            column_alpha(camera, 5, 0, 10, RAIN_ALPHA, 0.5),
            0.4375
        ));
        let column = Column {
            x: 3,
            z: 4,
            bottom: 60,
            top: 80,
            u_offset: 0.25,
            v_offset: -1.5,
            sky: 15,
            block: 2,
            snow: false,
        };
        let (vertices, indices) = mesh(&[column], camera, 10, 1.0);
        assert_eq!(indices, [0, 1, 2, 2, 3, 0]);
        assert_eq!(vertices.len(), 4);
        let [a, b, c, d] = [vertices[0], vertices[1], vertices[2], vertices[3]];
        assert!(close(a.position[0], 3.5 + 0.4) && close(a.position[2], 4.5 - 0.3));
        assert!(close(b.position[0], 3.5 - 0.4) && close(b.position[2], 4.5 + 0.3));
        assert_eq!(
            [a.position[1], b.position[1], c.position[1], d.position[1]],
            [80.0, 80.0, 60.0, 60.0]
        );
        assert_eq!(
            [a.uv, b.uv, c.uv, d.uv],
            [[0.25, 13.5], [1.25, 13.5], [1.25, 18.5], [0.25, 18.5]]
        );
        assert!(close(a.alpha, 1.0 - 0.5 * 25.0 / 100.0));
        assert_eq!((a.sky, a.block, a.snow), (15.0, 2.0, 0.0));
        let at_camera = Column {
            x: 0,
            z: 0,
            ..column
        };
        assert!(mesh(&[at_camera], camera, 10, 1.0).0.is_empty());
        assert_eq!(std::mem::size_of::<WeatherVertex>(), 36);
    }

    #[test]
    fn roofed_columns_draw_nothing() {
        let camera = DVec3::new(0.5, 64.2, 0.5);
        let height = |x: i32, _: i32| if x == 2 { 90 } else { 63 };
        let built = columns(
            camera,
            3,
            0,
            0.0,
            height,
            |_| Precipitation::Rain,
            |_| (15, 0),
        );
        assert_eq!(built.len(), 7 * 7 - 7);
        assert!(built.iter().all(|column| column.x != 2));
        assert!(
            built
                .iter()
                .all(|column| (column.bottom, column.top) == (63, 67))
        );
        let lit = columns(
            camera,
            3,
            0,
            0.0,
            |_, _| 66,
            |_| Precipitation::Rain,
            |(_, y, _)| (y as u8, 0),
        );
        assert!(
            lit.iter()
                .all(|column| column.sky == 66 && column.bottom == 66)
        );
        let low = columns(
            camera,
            3,
            0,
            0.0,
            |_, _| 10,
            |_| Precipitation::Rain,
            |_| (15, 0),
        );
        assert!(
            low.iter()
                .all(|column| (column.bottom, column.top) == (61, 67))
        );
        let dry = columns(
            camera,
            3,
            0,
            0.0,
            |_, _| 10,
            |_| Precipitation::None,
            |_| (15, 0),
        );
        assert!(dry.is_empty());
    }

    #[test]
    fn only_cold_biomes_snow() {
        assert_eq!(
            crate::rain::precipitation_of(false, -0.5),
            Precipitation::None
        );
        assert_eq!(
            crate::rain::precipitation_of(true, 0.149),
            Precipitation::Snow
        );
        assert_eq!(
            crate::rain::precipitation_of(true, 0.15),
            Precipitation::Rain
        );
        assert_eq!(
            crate::rain::precipitation_of(true, 0.8),
            Precipitation::Rain
        );
        let camera = DVec3::new(0.5, 64.0, 0.5);
        let built = columns(
            camera,
            2,
            0,
            0.0,
            |_, _| 0,
            |(x, _, _)| {
                if x < 0 {
                    Precipitation::Snow
                } else {
                    Precipitation::Rain
                }
            },
            |_| (12, 4),
        );
        let first_snow = built.iter().position(|column| column.snow).unwrap();
        assert!(
            built[..first_snow]
                .iter()
                .all(|column| !column.snow && column.x >= 0)
        );
        assert!(
            built[first_snow..]
                .iter()
                .all(|column| column.snow && column.x < 0)
        );
        assert!(
            built[first_snow..]
                .iter()
                .all(|column| (column.sky, column.block) == (12, 6))
        );
        assert!(
            built[..first_snow]
                .iter()
                .all(|column| (column.sky, column.block) == (12, 4))
        );
    }

    #[test]
    fn the_texture_puts_snow_to_the_right_of_rain() {
        let rain = RgbaImage::from_pixel(2, 4, image::Rgba([1, 2, 3, 4]));
        let snow = RgbaImage::from_pixel(1, 2, image::Rgba([9, 9, 9, 9]));
        let both = side_by_side(&rain, &snow);
        assert_eq!(both.dimensions(), (4, 4));
        assert_eq!(both.get_pixel(1, 3).0, [1, 2, 3, 4]);
        assert_eq!(both.get_pixel(2, 0).0, [9, 9, 9, 9]);
        assert_eq!(both.get_pixel(3, 3).0, [9, 9, 9, 9]);
    }

    #[test]
    fn the_vanilla_rain_and_snow_make_one_128_by_256_texture() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let both = weather_texture(&packs).unwrap().unwrap();
        assert_eq!(both.dimensions(), (128, 256));
        assert!(both.pixels().any(|p| p[3] == 0));
        assert!(both.pixels().any(|p| p[3] > 0));
    }
}
