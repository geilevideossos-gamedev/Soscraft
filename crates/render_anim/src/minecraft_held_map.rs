use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use frame::minecraft_ui::{McHeldMap, McMapHands};
use minecraft_terrain::map_picture::{self, MapLook};
use minecraft_terrain::maps::{MAP_ID, MapCache};
use minecraft_terrain::pack::PackStack;
use minecraftoss_player::inventory::ItemStack;

use crate::minecraft_hand::{Held, UseShown};

pub(crate) fn map_id(stack: &ItemStack) -> Option<i32> {
    let id = stack.components.as_ref()?.get(MAP_ID)?.as_i64()?;
    i32::try_from(id).ok()
}

pub(crate) fn drawn_as_map(stack: &ItemStack) -> bool {
    map_id(stack).is_some()
}

pub(crate) fn layout(
    main: Option<&ItemStack>,
    offhand: Option<&ItemStack>,
) -> Vec<(McMapHands, i32)> {
    let mut maps = Vec::new();
    if let Some(id) = main.and_then(map_id) {
        let hands = if offhand.is_none() {
            McMapHands::Both
        } else {
            McMapHands::Right
        };
        maps.push((hands, id));
    }
    if let Some(id) = offhand.and_then(map_id) {
        maps.push((McMapHands::Left, id));
    }
    maps
}

struct Picture {
    image: Handle<Image>,
    shows: (i32, Option<u32>),
}

#[derive(Default)]
pub(crate) struct HeldMaps {
    look: Option<MapLook>,
    pictures: [Option<Picture>; 2],
}

fn picture_image(pixels: Vec<u8>) -> Image {
    let side = map_picture::SIDE as u32;
    Image {
        sampler: bevy::image::ImageSampler::nearest(),
        ..Image::new(
            Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }
}

impl HeldMaps {
    pub(crate) fn load(packs: &PackStack, jar: Option<&Path>) -> Self {
        let (look, warnings) = MapLook::load(packs, jar);
        for warning in warnings {
            diag::warn!(World, "Minecraft held maps: {warning}");
        }
        Self {
            look: Some(look),
            pictures: Default::default(),
        }
    }

    pub(crate) fn show(
        &mut self,
        held: &Held<'_>,
        pitch: f32,
        cache: &MapCache,
        images: &mut Assets<Image>,
    ) -> Vec<McHeldMap> {
        let Some(look) = self.look.as_ref() else {
            return Vec::new();
        };
        if held
            .using
            .is_some_and(|using| using.kind == UseShown::Scope)
        {
            return Vec::new();
        }
        let pitch = (pitch + 180.0).rem_euclid(360.0) - 180.0;
        let main = held.main.map(|(_, stack)| stack);
        let mut shown = Vec::new();
        for (hands, id) in layout(main, held.offhand) {
            let offhand = hands == McMapHands::Left;
            let data = cache.get(id);
            let shows = (id, data.map(|map| map.version));
            let picture = &mut self.pictures[usize::from(offhand)];
            let image = match picture.take() {
                Some(old) if old.shows == shows => old.image,
                old => {
                    let image = picture_image(map_picture::compose(look, data));
                    match old {
                        Some(old) => {
                            let _ = images.insert(old.image.id(), image);
                            old.image
                        }
                        None => images.add(image),
                    }
                }
            };
            *picture = Some(Picture {
                image: image.clone(),
                shows,
            });
            shown.push(McHeldMap {
                picture: image,
                hands,
                pitch,
                swing: if held.swing_offhand == offhand {
                    held.swing
                } else {
                    0.0
                },
            });
        }
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft_hand::HeldUse;
    use minecraft_net::messages::{MapPatch, MapUpdate};
    use serde_json::json;

    fn map(item: &str, id: i64) -> ItemStack {
        ItemStack {
            components: Some(json!({ "minecraft:map_id": id })),
            ..ItemStack::new(item, 1)
        }
    }

    fn held<'a>(main: Option<&'a ItemStack>, offhand: Option<&'a ItemStack>) -> Held<'a> {
        Held {
            main: main.map(|stack| (0, stack)),
            offhand,
            swing: 0.25,
            swing_offhand: false,
            using: None,
            partial: 0.0,
            activation: None,
        }
    }

    fn update(id: i32, color: u8) -> MapUpdate {
        MapUpdate {
            id,
            scale: 0,
            locked: false,
            decorations: None,
            patch: Some(MapPatch {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                colors: vec![color],
            }),
        }
    }

    #[test]
    fn the_map_is_held_in_both_hands_only_while_the_offhand_is_empty() {
        let filled = map("minecraft:filled_map", 3);
        let treasure = map("minecraft:buried_treasure_map", 7);
        let torch = ItemStack::new("minecraft:torch", 1);
        let blank = ItemStack::new("minecraft:filled_map", 1);
        let empty = ItemStack::new("minecraft:map", 1);
        assert_eq!(layout(Some(&filled), None), vec![(McMapHands::Both, 3)]);
        assert_eq!(
            layout(Some(&filled), Some(&torch)),
            vec![(McMapHands::Right, 3)]
        );
        assert_eq!(
            layout(Some(&filled), Some(&treasure)),
            vec![(McMapHands::Right, 3), (McMapHands::Left, 7)]
        );
        assert_eq!(
            layout(Some(&torch), Some(&treasure)),
            vec![(McMapHands::Left, 7)]
        );
        assert_eq!(layout(None, Some(&treasure)), vec![(McMapHands::Left, 7)]);
        assert!(layout(Some(&blank), None).is_empty());
        assert!(layout(Some(&empty), None).is_empty());
        assert!(layout(None, None).is_empty());
        assert!(drawn_as_map(&treasure) && !drawn_as_map(&blank) && !drawn_as_map(&torch));
    }

    #[test]
    fn a_held_map_swings_with_its_own_hand_and_is_redrawn_when_its_picture_changes() {
        let mut maps = HeldMaps {
            look: Some(MapLook::default()),
            pictures: Default::default(),
        };
        let mut images = Assets::<Image>::default();
        let mut cache = MapCache::default();
        cache.apply(update(3, 4));
        let filled = map("minecraft:filled_map", 3);
        let shown = maps.show(&held(Some(&filled), None), 400.0, &cache, &mut images);
        assert_eq!(shown.len(), 1);
        assert_eq!(
            (shown[0].hands, shown[0].pitch, shown[0].swing),
            (McMapHands::Both, 40.0, 0.25)
        );
        let first = shown[0].picture.clone();
        assert_eq!(
            maps.pictures[0].as_ref().map(|p| p.shows),
            Some((3, Some(1)))
        );
        let again = maps.show(&held(Some(&filled), None), 0.0, &cache, &mut images);
        assert_eq!(again[0].picture, first);
        cache.apply(update(3, 8));
        let redrawn = maps.show(&held(Some(&filled), None), 0.0, &cache, &mut images);
        assert_eq!(redrawn[0].picture, first);
        assert_eq!(
            maps.pictures[0].as_ref().map(|p| p.shows),
            Some((3, Some(2)))
        );
        let mut offhand_swing = held(None, Some(&filled));
        offhand_swing.swing_offhand = true;
        let left = maps.show(&offhand_swing, 0.0, &cache, &mut images);
        assert_eq!((left[0].hands, left[0].swing), (McMapHands::Left, 0.25));
        assert_ne!(left[0].picture, first);
        let torch = ItemStack::new("minecraft:torch", 1);
        let right = maps.show(&held(Some(&filled), Some(&torch)), 0.0, &cache, &mut images);
        assert_eq!((right[0].hands, right[0].swing), (McMapHands::Right, 0.25));
        let mut scoping = held(Some(&filled), None);
        scoping.using = Some(HeldUse {
            slot: 0,
            kind: UseShown::Scope,
        });
        assert!(maps.show(&scoping, 0.0, &cache, &mut images).is_empty());
    }
}
