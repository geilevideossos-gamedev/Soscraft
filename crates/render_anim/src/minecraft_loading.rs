use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use frame::{McLevelLoading, McLoadingReason};
use image::RgbaImage;
use minecraft_survival::switch::LoadingReason;
use minecraft_terrain::pack::{PackStack, ResourceId};

const END_PORTAL_LAYERS: usize = 15;
const END_PORTAL_SHADER: &str = "shaders/core/rendertype_end_portal.fsh";
const END_CANVAS_WIDTH: u32 = 960;
const PANORAMA_CANVAS_WIDTH: u32 = 480;
const PANORAMA_FOV_DEGREES: f32 = 85.0;
const PANORAMA_PITCH_DEGREES: f32 = 10.0;
const PANORAMA_SPIN_PER_TICK: f32 = 0.1;
const PANORAMA_FACES: [&str; 6] = ["_1", "_3", "_5", "_4", "_0", "_2"];
const SWAP_FRAMES: u32 = 2;

#[derive(Default)]
pub(crate) struct LoadingBackgrounds {
    portal: Option<(Handle<Image>, u32)>,
    end_sky: Option<RgbaImage>,
    end_portal: Option<RgbaImage>,
    end_colors: Vec<[f32; 3]>,
    panorama: Option<Vec<RgbaImage>>,
    menu: Option<Handle<Image>>,
    canvases: [Option<Handle<Image>>; 2],
    shown: usize,
    pending: Option<(usize, u32)>,
    painted: Option<LoadingReason>,
    clock: f64,
    spin: f32,
    spun: u64,
}

impl LoadingBackgrounds {
    pub(crate) fn load(packs: &PackStack, images: &mut Assets<Image>) -> Self {
        let texture = |name: &str| -> Option<RgbaImage> {
            let id = ResourceId::parse(name).ok()?;
            let bytes = packs.texture(&id).ok()??;
            Some(
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
                    .ok()?
                    .to_rgba8(),
            )
        };
        let portal = texture("minecraft:block/nether_portal").map(|mut strip| {
            for pixel in strip.pixels_mut() {
                pixel.0[3] = 255;
            }
            let frames = (strip.height() / strip.width().max(1)).max(1);
            (images.add(gpu_image(strip, false)), frames)
        });
        let panorama: Option<Vec<RgbaImage>> = PANORAMA_FACES
            .iter()
            .map(|face| texture(&format!("minecraft:gui/title/background/panorama{face}")))
            .collect();
        let menu = texture("minecraft:gui/inworld_menu_background")
            .map(|menu| images.add(gpu_image(menu, false)));
        let end_colors = ResourceId::parse("minecraft:end_portal")
            .ok()
            .and_then(|id| packs.get(&id, END_PORTAL_SHADER).ok().flatten())
            .map(|source| end_portal_colors(&String::from_utf8_lossy(&source)))
            .unwrap_or_default();
        if end_colors.len() < END_PORTAL_LAYERS {
            diag::warn!(
                World,
                "Minecraft end portal colours unreadable from {END_PORTAL_SHADER}: the End loading screen is plain"
            );
        }
        Self {
            portal,
            end_sky: texture("minecraft:environment/end_sky"),
            end_portal: texture("minecraft:entity/end_portal/end_portal"),
            end_colors,
            panorama,
            menu,
            ..Self::default()
        }
    }

    pub(crate) fn frame(
        &mut self,
        reason: Option<LoadingReason>,
        game_time: f64,
        dt: f64,
        aspect: f32,
        images: &mut Assets<Image>,
    ) -> Option<McLevelLoading> {
        let Some(reason) = reason else {
            self.painted = None;
            self.pending = None;
            self.clock = 0.0;
            self.spun = 0;
            return None;
        };
        self.clock += dt;
        let ticks = (self.clock * 20.0) as u64;
        if let Some((index, frames)) = self.pending.as_mut() {
            if *frames == 0 {
                self.shown = *index;
                self.pending = None;
            } else {
                *frames -= 1;
            }
        }
        match reason {
            LoadingReason::NetherPortal => {
                let (portal, frames) = self.portal.clone().unzip();
                Some(McLevelLoading {
                    reason: McLoadingReason::NetherPortal,
                    background: portal,
                    background_st: portal_frame_st(ticks, frames.unwrap_or(1)),
                    menu_background: None,
                })
            }
            LoadingReason::EndPortal => {
                if self.painted != Some(reason) {
                    self.painted = Some(reason);
                    let width = END_CANVAS_WIDTH;
                    let height = canvas_height(width, aspect);
                    let mut canvas = RgbaImage::new(width, height);
                    if let (Some(sky), Some(portal)) = (&self.end_sky, &self.end_portal)
                        && self.end_colors.len() >= END_PORTAL_LAYERS
                    {
                        let time = ((game_time.rem_euclid(24_000.0)) / 24_000.0) as f32;
                        paint_end_portal(&mut canvas, sky, portal, &self.end_colors, time);
                    } else {
                        for pixel in canvas.pixels_mut() {
                            pixel.0 = [0, 0, 0, 255];
                        }
                    }
                    self.paint(canvas, images, true);
                }
                Some(self.canvas_loading(McLoadingReason::EndPortal, None))
            }
            LoadingReason::Other => {
                if self.painted != Some(reason) || (self.pending.is_none() && ticks > self.spun) {
                    self.spin = wrap_degrees(
                        self.spin
                            + (ticks.saturating_sub(self.spun)) as f32 * PANORAMA_SPIN_PER_TICK,
                    );
                    self.spun = ticks;
                    let immediate = self.painted != Some(reason);
                    self.painted = Some(reason);
                    let width = PANORAMA_CANVAS_WIDTH;
                    let mut canvas = RgbaImage::new(width, canvas_height(width, aspect));
                    match &self.panorama {
                        Some(faces) => paint_panorama(&mut canvas, faces, -self.spin),
                        None => {
                            for pixel in canvas.pixels_mut() {
                                pixel.0 = [0, 0, 0, 255];
                            }
                        }
                    }
                    self.paint(canvas, images, immediate);
                }
                let menu = self.menu.clone();
                Some(self.canvas_loading(McLoadingReason::Other, menu))
            }
        }
    }

    fn paint(&mut self, canvas: RgbaImage, images: &mut Assets<Image>, immediate: bool) {
        let index = if immediate {
            self.shown
        } else {
            1 - self.shown
        };
        let image = gpu_image(canvas, true);
        match self.canvases[index].as_ref() {
            Some(handle) => {
                let _ = images.insert(handle.id(), image);
            }
            None => self.canvases[index] = Some(images.add(image)),
        }
        if !immediate {
            self.pending = Some((index, SWAP_FRAMES));
        }
    }

    fn canvas_loading(
        &self,
        reason: McLoadingReason,
        menu: Option<Handle<Image>>,
    ) -> McLevelLoading {
        McLevelLoading {
            reason,
            background: self.canvases[self.shown].clone(),
            background_st: [0.0, 0.0, 1.0, 1.0],
            menu_background: menu,
        }
    }
}

fn gpu_image(image: RgbaImage, smooth: bool) -> Image {
    let filter = if smooth {
        ImageFilterMode::Linear
    } else {
        ImageFilterMode::Nearest
    };
    Image {
        sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            ..default()
        }),
        ..Image::new(
            Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            image.into_raw(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        )
    }
}

fn canvas_height(width: u32, aspect: f32) -> u32 {
    let aspect = if aspect.is_finite() && aspect > 0.1 {
        aspect
    } else {
        16.0 / 9.0
    };
    ((width as f32 / aspect).round() as u32).max(1)
}

fn wrap_degrees(degrees: f32) -> f32 {
    let wrapped = degrees.rem_euclid(360.0);
    if wrapped >= 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }
}

pub(crate) fn portal_frame_st(ticks: u64, frames: u32) -> [f32; 4] {
    let frames = frames.max(1);
    let frame = (ticks % u64::from(frames)) as f32;
    let count = frames as f32;
    [0.0, frame / count, 1.0, (frame + 1.0) / count]
}

pub(crate) fn end_portal_colors(source: &str) -> Vec<[f32; 3]> {
    let Some(start) = source.find("COLORS") else {
        return Vec::new();
    };
    let body = &source[start..];
    let Some(open) = body.find("vec3[](") else {
        return Vec::new();
    };
    let body = &body[open + "vec3[](".len()..];
    let end = body.find(");").unwrap_or(body.len());
    body[..end]
        .split("vec3(")
        .skip(1)
        .filter_map(|entry| {
            let numbers: Vec<f32> = entry
                .split(')')
                .next()?
                .split(',')
                .map(|number| number.trim().parse().ok())
                .collect::<Option<_>>()?;
            match numbers[..] {
                [r, g, b] => Some([r, g, b]),
                _ => None,
            }
        })
        .collect()
}

pub(crate) fn end_portal_uv(layer: f32, game_time: f32, screen: [f32; 2]) -> [f32; 2] {
    let scale = (4.5 - layer / 4.0) * 2.0;
    let angle = ((layer * layer * 4321.0 + layer * 9.0) * 2.0).to_radians();
    let (sin, cos) = angle.sin_cos();
    let [x, y] = screen;
    let turned = [scale * (x * cos - y * sin), scale * (x * sin + y * cos)];
    let moved = [
        turned[0] + 17.0 / layer,
        turned[1] + (2.0 + layer / 1.5) * (game_time * 1.5),
    ];
    [moved[0] * 0.5 + 0.25, moved[1] * 0.5 + 0.25]
}

fn sample_repeat(image: &RgbaImage, [u, v]: [f32; 2]) -> [f32; 3] {
    let (width, height) = (image.width().max(1), image.height().max(1));
    let x = ((u.rem_euclid(1.0) * width as f32) as u32).min(width - 1);
    let y = ((v.rem_euclid(1.0) * height as f32) as u32).min(height - 1);
    let pixel = image.get_pixel(x, y).0;
    [
        f32::from(pixel[0]) / 255.0,
        f32::from(pixel[1]) / 255.0,
        f32::from(pixel[2]) / 255.0,
    ]
}

fn paint_end_portal(
    canvas: &mut RgbaImage,
    sky: &RgbaImage,
    portal: &RgbaImage,
    colors: &[[f32; 3]],
    game_time: f32,
) {
    let (width, height) = (canvas.width(), canvas.height());
    for py in 0..height {
        for px in 0..width {
            let screen = [
                (px as f32 + 0.5) / width as f32,
                1.0 - (py as f32 + 0.5) / height as f32,
            ];
            let base = sample_repeat(sky, screen);
            let mut color = [
                base[0] * colors[0][0],
                base[1] * colors[0][1],
                base[2] * colors[0][2],
            ];
            for (i, tint) in colors.iter().take(END_PORTAL_LAYERS).enumerate() {
                let texel = sample_repeat(portal, end_portal_uv((i + 1) as f32, game_time, screen));
                for channel in 0..3 {
                    color[channel] += texel[channel] * tint[channel];
                }
            }
            let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
            canvas.put_pixel(
                px,
                py,
                image::Rgba([byte(color[0]), byte(color[1]), byte(color[2]), 255]),
            );
        }
    }
}

pub(crate) fn cube_face(direction: Vec3) -> (usize, [f32; 2]) {
    let [x, y, z] = direction.to_array();
    let (ax, ay, az) = (x.abs(), y.abs(), z.abs());
    let (face, sc, tc, major) = if ax >= ay && ax >= az {
        if x > 0.0 {
            (0, -z, -y, ax)
        } else {
            (1, z, -y, ax)
        }
    } else if ay >= az {
        if y > 0.0 {
            (2, x, z, ay)
        } else {
            (3, x, -z, ay)
        }
    } else if z > 0.0 {
        (4, x, -y, az)
    } else {
        (5, -x, -y, az)
    };
    let major = major.max(f32::EPSILON);
    (face, [(sc / major + 1.0) * 0.5, (tc / major + 1.0) * 0.5])
}

pub(crate) fn panorama_direction(screen: [f32; 2], aspect: f32, yaw_degrees: f32) -> Vec3 {
    let half = (PANORAMA_FOV_DEGREES * 0.5).to_radians().tan();
    let view = Vec3::new(
        (screen[0] * 2.0 - 1.0) * half * aspect,
        (1.0 - screen[1] * 2.0) * half,
        -1.0,
    );
    let undo = Mat3::from_rotation_y(-yaw_degrees.to_radians())
        * Mat3::from_rotation_x(-PANORAMA_PITCH_DEGREES.to_radians())
        * Mat3::from_rotation_x(-std::f32::consts::PI);
    undo * view
}

fn paint_panorama(canvas: &mut RgbaImage, faces: &[RgbaImage], yaw_degrees: f32) {
    let (width, height) = (canvas.width(), canvas.height());
    let aspect = width as f32 / height.max(1) as f32;
    for py in 0..height {
        for px in 0..width {
            let screen = [
                (px as f32 + 0.5) / width as f32,
                (py as f32 + 0.5) / height as f32,
            ];
            let (face, uv) = cube_face(panorama_direction(screen, aspect, yaw_degrees));
            let [r, g, b] = faces
                .get(face)
                .map_or([0.0; 3], |face| sample_repeat(face, uv));
            let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
            canvas.put_pixel(px, py, image::Rgba([byte(r), byte(g), byte(b), 255]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
    }

    #[test]
    fn the_end_portal_colours_come_from_the_shader_source() {
        let source = "uniform sampler2D Sampler1;\nconst vec3[] COLORS = vec3[](\n    vec3(0.1, 0.2, 0.3),\n    vec3(1.0, 0.5, 0.25)\n);\nconst mat4 SCALE_TRANSLATE = mat4(0.5);";
        assert_eq!(
            end_portal_colors(source),
            vec![[0.1, 0.2, 0.3], [1.0, 0.5, 0.25]]
        );
        assert!(end_portal_colors("void main() {}").is_empty());
        assert!(end_portal_colors("const vec3[] COLORS = vec3[](vec3(1.0, x, 2.0));").is_empty());
    }

    #[test]
    fn an_end_portal_layer_scales_turns_and_shifts_the_screen_like_the_shader() {
        assert!(near(end_portal_uv(1.0, 0.0, [0.0, 0.0]), [8.75, 0.25]));
        assert!(near(end_portal_uv(4.0, 0.0, [0.0, 0.0]), [2.375, 0.25]));
        let drift = end_portal_uv(3.0, 0.5, [0.0, 0.0])[1] - end_portal_uv(3.0, 0.0, [0.0, 0.0])[1];
        assert!(
            (drift - (2.0 + 3.0 / 1.5) * 0.75 * 0.5).abs() < 1e-4,
            "{drift}"
        );
        let angle = ((4321.0f32 + 9.0) * 2.0).to_radians();
        let corner = end_portal_uv(1.0, 0.0, [1.0, 0.0]);
        assert!(near(
            corner,
            [
                (8.5 * angle.cos() + 17.0) * 0.5 + 0.25,
                8.5 * angle.sin() * 0.5 + 0.25
            ]
        ));
    }

    #[test]
    fn the_nether_portal_background_steps_one_frame_a_tick() {
        assert_eq!(portal_frame_st(0, 32), [0.0, 0.0, 1.0, 1.0 / 32.0]);
        assert_eq!(portal_frame_st(33, 32), [0.0, 1.0 / 32.0, 1.0, 2.0 / 32.0]);
        assert_eq!(portal_frame_st(5, 0), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn a_cube_map_direction_picks_its_face_like_gl() {
        assert_eq!(cube_face(Vec3::X), (0, [0.5, 0.5]));
        assert_eq!(cube_face(Vec3::NEG_X), (1, [0.5, 0.5]));
        assert_eq!(cube_face(Vec3::Y), (2, [0.5, 0.5]));
        assert_eq!(cube_face(Vec3::NEG_Y), (3, [0.5, 0.5]));
        assert_eq!(cube_face(Vec3::Z), (4, [0.5, 0.5]));
        assert_eq!(cube_face(Vec3::NEG_Z), (5, [0.5, 0.5]));
        let (face, uv) = cube_face(Vec3::new(0.5, 0.5, 1.0));
        assert_eq!(face, 4);
        assert!(near(uv, [0.75, 0.25]), "{uv:?}");
    }

    #[test]
    fn the_panorama_looks_from_the_upside_down_cube_tilted_ten_degrees() {
        let ahead = panorama_direction([0.5, 0.5], 16.0 / 9.0, 0.0);
        let pitch = PANORAMA_PITCH_DEGREES.to_radians();
        assert!(
            (ahead - Vec3::new(0.0, pitch.sin(), pitch.cos())).length() < 1e-4,
            "{ahead}"
        );
        assert_eq!(cube_face(ahead).0, 4);
        let turned = panorama_direction([0.5, 0.5], 16.0 / 9.0, 90.0);
        assert_eq!(cube_face(turned).0, 1, "{turned}");
    }

    #[test]
    fn the_canvas_keeps_the_window_shape_and_the_spin_wraps() {
        assert_eq!(canvas_height(960, 16.0 / 9.0), 540);
        assert_eq!(canvas_height(960, f32::NAN), 540);
        assert_eq!(canvas_height(480, 4.0 / 3.0), 360);
        assert_eq!(wrap_degrees(190.0), -170.0);
        assert_eq!(wrap_degrees(-190.0), 170.0);
    }

    #[test]
    fn a_painted_end_portal_is_the_sky_and_the_layers_added_up() {
        let sky = RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 255, 255]));
        let portal = RgbaImage::from_pixel(2, 2, image::Rgba([128, 128, 128, 255]));
        let colors = vec![[0.02, 0.04, 0.06]; END_PORTAL_LAYERS];
        let mut canvas = RgbaImage::new(4, 2);
        paint_end_portal(&mut canvas, &sky, &portal, &colors, 0.25);
        let expected = |c: f32| ((c + 15.0 * c * 128.0 / 255.0) * 255.0).round() as u8;
        for pixel in canvas.pixels() {
            assert_eq!(
                pixel.0,
                [expected(0.02), expected(0.04), expected(0.06), 255]
            );
        }
    }
}
