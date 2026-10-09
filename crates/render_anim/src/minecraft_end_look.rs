use std::sync::Arc;

use image::RgbaImage;
use minecraft_terrain::pack::{PackStack, ResourceId};

pub const PORTAL_COLORS: usize = 16;
const GAME_TIME_PERIOD: f64 = 24_000.0;

pub struct EndLook {
    pub end_sky: Option<Arc<RgbaImage>>,
    pub end_portal: Option<Arc<RgbaImage>>,
    pub portal_colors: [[f32; 4]; PORTAL_COLORS],
}

#[derive(Default)]
pub struct EndFrame {
    pub look: Option<Arc<EndLook>>,
    pub portal_time: f32,
}

impl EndLook {
    pub(crate) fn load(packs: &PackStack) -> Self {
        let texture = |name: &str| -> Option<Arc<RgbaImage>> {
            let id = ResourceId::parse(name).ok()?;
            let bytes = packs.texture(&id).ok()??;
            let image =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
            Some(Arc::new(image.to_rgba8()))
        };
        let parsed = ResourceId::parse("minecraft:end_portal")
            .ok()
            .and_then(|id| {
                packs
                    .get(&id, crate::minecraft_loading::END_PORTAL_SHADER)
                    .ok()
                    .flatten()
            })
            .map_or_else(Vec::new, |source| {
                crate::minecraft_loading::end_portal_colors(&String::from_utf8_lossy(&source))
            });
        if parsed.len() < PORTAL_COLORS {
            diag::warn!(
                World,
                "Minecraft end portal colours: {} of {PORTAL_COLORS} read from {}, the rest are black",
                parsed.len(),
                crate::minecraft_loading::END_PORTAL_SHADER
            );
        }
        Self {
            end_sky: texture("minecraft:environment/end_sky"),
            end_portal: texture("minecraft:entity/end_portal/end_portal"),
            portal_colors: portal_colors(&parsed),
        }
    }
}

pub(crate) fn portal_colors(parsed: &[[f32; 3]]) -> [[f32; 4]; PORTAL_COLORS] {
    let mut colors = [[0.0; 4]; PORTAL_COLORS];
    for (slot, [r, g, b]) in colors.iter_mut().zip(parsed) {
        *slot = [*r, *g, *b, 1.0];
    }
    colors
}

pub(crate) fn portal_time(game_ticks: f64) -> f32 {
    (game_ticks.rem_euclid(GAME_TIME_PERIOD) / GAME_TIME_PERIOD) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_portal_colours_fill_sixteen_slots_and_pad_with_black() {
        let colors = portal_colors(&[[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]);
        assert_eq!(colors[0], [0.1, 0.2, 0.3, 1.0]);
        assert_eq!(colors[1], [0.4, 0.5, 0.6, 1.0]);
        assert!(colors[2..].iter().all(|color| *color == [0.0; 4]));
        let many = vec![[1.0, 1.0, 1.0]; 20];
        assert!(portal_colors(&many).iter().all(|color| *color == [1.0; 4]));
    }

    #[test]
    fn the_portal_time_is_the_game_time_shader_uniform() {
        assert_eq!(portal_time(0.0), 0.0);
        assert_eq!(portal_time(6_000.0), 0.25);
        assert_eq!(portal_time(30_000.0), 0.25);
        assert!((portal_time(23_999.5) - 23_999.5 / 24_000.0).abs() < 1e-6);
    }

    #[test]
    fn the_vanilla_shader_lists_sixteen_portal_colours() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let look = EndLook::load(&packs);
        assert!(look.end_sky.is_some() && look.end_portal.is_some());
        assert_eq!(look.portal_colors[0], [0.022087, 0.098399, 0.110818, 1.0]);
        assert_eq!(look.portal_colors[15], [0.080955, 0.314821, 0.661491, 1.0]);
    }
}
