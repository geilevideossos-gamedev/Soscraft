use image::{Rgba, RgbaImage};

pub const PATH: &str = "iw4l/margit";
pub const TEXTURE: &str = "minecraft:iw4l/margit";
pub const SIZE: u32 = 64;

pub const HEAD: [f32; 2] = [0.0, 0.0];
pub const BEARD: [f32; 2] = [32.0, 0.0];
pub const HORN: [f32; 2] = [44.0, 0.0];
pub const WOOD: [f32; 2] = [52.0, 0.0];
pub const HOOD: [f32; 2] = [0.0, 16.0];
pub const GOLD: [f32; 2] = [40.0, 19.0];
pub const CLOTH: [f32; 2] = [0.0, 32.0];
pub const SKIRT: [f32; 2] = [0.0, 50.0];
pub const ARM: [f32; 2] = [52.0, 40.0];

pub const HEAD_AREA: [u32; 4] = [0, 0, 32, 16];
pub const BEARD_AREA: [u32; 4] = [32, 0, 44, 7];
pub const HORN_AREA: [u32; 4] = [44, 0, 52, 6];
pub const WOOD_AREA: [u32; 4] = [52, 0, 60, 19];
pub const HOOD_AREA: [u32; 4] = [0, 16, 36, 32];
pub const GOLD_AREA: [u32; 4] = [40, 19, 64, 40];
pub const CLOTH_AREA: [u32; 4] = [0, 32, 34, 50];
pub const SKIRT_AREA: [u32; 4] = [0, 50, 50, 64];
pub const ARM_AREA: [u32; 4] = [52, 40, 64, 57];

pub const CLOAK: [u8; 3] = [38, 33, 36];
pub const FUR: [u8; 3] = [96, 84, 70];
pub const BEARD_GREY: [u8; 3] = [170, 167, 160];
pub const PALE: [u8; 3] = [196, 172, 152];
pub const SHADOW: [u8; 3] = [30, 26, 26];
pub const EYE: [u8; 3] = [255, 208, 64];
pub const BONE: [u8; 3] = [214, 204, 176];
pub const CANE: [u8; 3] = [92, 62, 38];
pub const GOLDEN: [u8; 3] = [236, 184, 52];
pub const GLEAM: [u8; 3] = [255, 230, 128];

const FACE: [u32; 4] = [8, 8, 16, 16];
const EYES: [[u32; 2]; 4] = [[9, 11], [10, 11], [13, 11], [14, 11]];
const BROWS: [[u32; 2]; 4] = [[9, 10], [10, 10], [13, 10], [14, 10]];
const HOOD_RIM: [[u32; 4]; 3] = [[8, 23, 18, 24], [7, 24, 8, 32], [18, 24, 19, 32]];
const HOOD_INSIDE: [u32; 4] = [18, 16, 28, 24];
const MANTLE: [u32; 4] = [3, 32, 17, 37];
const HEM: [u32; 4] = [0, 62, 50, 64];

fn hash(x: u32, y: u32, salt: u32) -> u32 {
    let mut h =
        x.wrapping_mul(0x9e37_79b1) ^ y.wrapping_mul(0x85eb_ca77) ^ salt.wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

fn shade(base: [u8; 3], spread: i32, x: u32, y: u32, salt: u32) -> Rgba<u8> {
    let span = (2 * spread + 1) as u32;
    let offset = (hash(x, y, salt) % span) as i32 - spread;
    let channel = |c: u8| (i32::from(c) + offset).clamp(0, 255) as u8;
    Rgba([channel(base[0]), channel(base[1]), channel(base[2]), 255])
}

fn fill(skin: &mut RgbaImage, area: [u32; 4], base: [u8; 3], spread: i32, salt: u32) {
    for y in area[1]..area[3] {
        for x in area[0]..area[2] {
            skin.put_pixel(x, y, shade(base, spread, x, y, salt));
        }
    }
}

fn dot(skin: &mut RgbaImage, at: [u32; 2], colour: [u8; 3]) {
    skin.put_pixel(at[0], at[1], Rgba([colour[0], colour[1], colour[2], 255]));
}

fn darken(skin: &mut RgbaImage, x: u32, y: u32, by: u8) {
    let Rgba([r, g, b, a]) = *skin.get_pixel(x, y);
    skin.put_pixel(
        x,
        y,
        Rgba([
            r.saturating_sub(by),
            g.saturating_sub(by),
            b.saturating_sub(by),
            a,
        ]),
    );
}

pub fn paint() -> RgbaImage {
    let mut skin = RgbaImage::new(SIZE, SIZE);
    fill(&mut skin, HEAD_AREA, SHADOW, 4, 1);
    fill(&mut skin, FACE, PALE, 8, 2);
    for x in FACE[0]..FACE[2] {
        darken(&mut skin, x, FACE[1], 70);
        darken(&mut skin, x, FACE[1] + 1, 40);
    }
    for at in BROWS {
        dot(&mut skin, at, [70, 60, 56]);
    }
    for at in EYES {
        dot(&mut skin, at, EYE);
    }
    for x in 11..13 {
        darken(&mut skin, x, 12, 24);
    }
    fill(&mut skin, BEARD_AREA, BEARD_GREY, 14, 3);
    for y in BEARD_AREA[1]..BEARD_AREA[3] {
        for x in BEARD_AREA[0]..BEARD_AREA[2] {
            if hash(x, 0, 4) % 3 == 0 {
                darken(&mut skin, x, y, 30);
            }
        }
    }
    fill(&mut skin, HORN_AREA, BONE, 10, 5);
    for x in HORN_AREA[0]..HORN_AREA[2] {
        darken(&mut skin, x, HORN_AREA[1], 60);
        darken(&mut skin, x, HORN_AREA[1] + 1, 40);
        darken(&mut skin, x, HORN_AREA[1] + 2, 20);
    }
    fill(&mut skin, WOOD_AREA, CANE, 6, 6);
    for y in WOOD_AREA[1]..WOOD_AREA[3] {
        for x in WOOD_AREA[0]..WOOD_AREA[2] {
            if hash(x, y / 4, 7) % 3 == 0 {
                darken(&mut skin, x, y, 22);
            }
        }
    }
    fill(&mut skin, HOOD_AREA, CLOAK, 6, 8);
    for rim in HOOD_RIM {
        fill(&mut skin, rim, FUR, 14, 9);
    }
    fill(&mut skin, HOOD_INSIDE, SHADOW, 3, 10);
    fill(&mut skin, CLOTH_AREA, CLOAK, 6, 11);
    for y in CLOTH_AREA[1]..CLOTH_AREA[3] {
        for x in CLOTH_AREA[0]..CLOTH_AREA[2] {
            if hash(x, y, 12) % 9 == 0 {
                darken(&mut skin, x, y, 12);
            }
        }
    }
    fill(&mut skin, MANTLE, FUR, 14, 13);
    fill(&mut skin, SKIRT_AREA, CLOAK, 6, 14);
    for x in HEM[0]..HEM[2] {
        if hash(x, HEM[1], 15) % 5 == 0 {
            skin.get_pixel_mut(x, HEM[1]).0[3] = 0;
        }
        if hash(x, HEM[3], 16) % 2 == 0 {
            skin.get_pixel_mut(x, HEM[3] - 1).0[3] = 0;
        }
    }
    fill(&mut skin, GOLD_AREA, GOLDEN, 14, 17);
    for y in GOLD_AREA[1]..GOLD_AREA[3] {
        for x in GOLD_AREA[0]..GOLD_AREA[2] {
            if hash(x, y, 18) % 7 == 0 {
                dot(&mut skin, [x, y], GLEAM);
            }
        }
    }
    fill(&mut skin, ARM_AREA, PALE, 8, 19);
    for y in ARM_AREA[1]..ARM_AREA[3] {
        for x in ARM_AREA[0]..ARM_AREA[2] {
            if hash(x, 0, 20) % 4 == 0 {
                darken(&mut skin, x, y, 26);
            }
        }
    }
    for x in ARM_AREA[0]..ARM_AREA[2] {
        darken(&mut skin, x, ARM_AREA[3] - 1, 30);
        darken(&mut skin, x, ARM_AREA[3] - 2, 18);
    }
    skin
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luma(pixel: &Rgba<u8>) -> u32 {
        (u32::from(pixel[0]) * 3 + u32::from(pixel[1]) * 6 + u32::from(pixel[2])) / 10
    }

    fn pixels(skin: &RgbaImage, area: [u32; 4]) -> Vec<Rgba<u8>> {
        let mut out = Vec::new();
        for y in area[1]..area[3] {
            for x in area[0]..area[2] {
                out.push(*skin.get_pixel(x, y));
            }
        }
        out
    }

    #[test]
    fn the_skin_paints_the_cloak_dark_and_the_beard_grey_in_their_uv_boxes() {
        let skin = paint();
        assert_eq!(skin.dimensions(), (SIZE, SIZE));
        let torso_front = [5, 37, 14, 50];
        for pixel in pixels(&skin, torso_front) {
            assert!(luma(&pixel) < 60 && pixel[3] == 255, "{pixel:?}");
        }
        let beard_front = [33, 1, 38, 7];
        for pixel in pixels(&skin, beard_front) {
            let spread =
                pixel[0].max(pixel[1]).max(pixel[2]) - pixel[0].min(pixel[1]).min(pixel[2]);
            assert!(spread < 16, "grey: {pixel:?}");
            assert!((110..=200).contains(&luma(&pixel)), "{pixel:?}");
        }
        for at in EYES {
            assert_eq!(
                skin.get_pixel(at[0], at[1]).0,
                [EYE[0], EYE[1], EYE[2], 255]
            );
        }
        for pixel in pixels(&skin, [8, 12, 16, 16]) {
            assert!(
                pixel[0] > 150 && pixel[0] > pixel[2],
                "pale skin: {pixel:?}"
            );
        }
        for pixel in pixels(&skin, GOLD_AREA) {
            assert!(
                pixel[0] > 200 && pixel[1] > 150 && pixel[2] < 140,
                "gold: {pixel:?}"
            );
        }
        for pixel in pixels(&skin, WOOD_AREA) {
            assert!(
                pixel[0] > pixel[1] && pixel[1] > pixel[2] && luma(&pixel) < 90,
                "wood: {pixel:?}"
            );
        }
        let hem = pixels(&skin, [0, 63, 50, 64]);
        let torn = hem.iter().filter(|pixel| pixel[3] == 0).count();
        assert!(torn > 10 && torn < 40, "a ragged hem: {torn}");
        assert!(
            pixels(&skin, [0, 50, 50, 62])
                .iter()
                .all(|pixel| pixel[3] == 255)
        );
        assert_eq!(paint(), skin, "the same every time");
    }
}
