use std::collections::BTreeMap;

use bevy::prelude::*;
use frame::MinecraftUi;
use frame::minecraft_ui::McPortalOverlay;
use minecraft_survival::level::Level;
use minecraft_survival::player::Player;
use minecraft_survival::portal;
use minecraft_survival::server::Globals;
use minecraft_terrain::pack::{PackStack, ResourceId};
use minecraft_terrain::terrain::Dimension;

use crate::minecraft_sounds::Sounds;

const TRIGGER_SOUND: &str = "minecraft:block.portal.trigger";
const TRAVEL_SOUND: &str = "minecraft:block.portal.travel";
const PORTAL_SOUND_VOLUME: f32 = 0.25;
const PORTAL_SPRITE: &str = "minecraft:block/nether_portal";

fn portal_pitch(sounds: &mut Sounds) -> f32 {
    sounds.random() * 0.4 + 0.8
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn step(
    player: &mut Player,
    levels: &mut BTreeMap<Dimension, Level>,
    globals: &Globals,
    feet: Option<[f64; 3]>,
    seconds: f64,
    sounds: Option<&mut Sounds>,
    packs: &PackStack,
    mut ui: Option<&mut MinecraftUi>,
) {
    let Some(level) = levels.get_mut(&player.dimension) else {
        if let Some(ui) = ui {
            ui.portal_overlay = None;
        }
        return;
    };
    let stepped = portal::step(
        player,
        &level.terrain,
        &globals.game_rules,
        feet,
        seconds,
        |pos| minecraft_survival::the_end::gateway::enter(level.entities.as_mut(), pos),
    );
    if stepped.inside
        && let Some(ui) = ui.as_deref_mut()
    {
        ui.inventory_open = false;
    }
    if stepped.triggered
        && let Some(sounds) = sounds
    {
        let pitch = portal_pitch(sounds);
        sounds.play(packs, TRIGGER_SOUND, None, PORTAL_SOUND_VOLUME, pitch);
    }
    let partial = (player.portal.clock / portal::TICK_SECONDS) as f32;
    let shown = player.portal.effect.shown(partial);
    if let Some(ui) = ui.as_deref_mut() {
        ui.portal_overlay = (shown > 0.0).then(|| McPortalOverlay {
            alpha: portal::overlay_alpha(shown),
            frame: player.portal.age,
        });
    }
    let Some(due) = stepped.due else {
        return;
    };
    if due.kind == portal::PortalKind::End {
        crate::minecraft_end::travel(player, level, globals, due.entry);
        if let Some(ui) = ui {
            ui.portal_overlay = None;
        }
        return;
    }
    if due.kind == portal::PortalKind::Gateway {
        crate::minecraft_end::gateway(player, level, due.entry);
        return;
    }
    let Some(travel) = portal::depart(player, &level.terrain, &globals.registries, due) else {
        diag::warn!(World, "Minecraft portal at {:?} leads nowhere", due.entry);
        return;
    };
    if !portal::may_enter(&globals.game_rules, travel.to) {
        return;
    }
    diag::info!(
        World,
        "Minecraft portal at {:?} leads to {} near {:?}",
        due.entry,
        travel.to.dimension_type(),
        travel.target
    );
    portal::set_off(player, travel);
    if let Some(ui) = ui {
        ui.portal_overlay = None;
    }
}

pub(crate) fn arriving(player: &mut Player, sounds: Option<&mut Sounds>, packs: &PackStack) {
    let known = player
        .arrival
        .is_some_and(|arrival| arrival.dimension == player.dimension);
    if !player.portal.travel_sound || !known {
        return;
    }
    player.portal.travel_sound = false;
    if let Some(sounds) = sounds {
        let pitch = portal_pitch(sounds);
        sounds.play(packs, TRAVEL_SOUND, None, PORTAL_SOUND_VOLUME, pitch);
    }
}

pub(crate) const FREEZE_OUTLINE: &str = "minecraft:misc/powder_snow_outline";
pub(crate) const SPYGLASS_SCOPE: &str = "minecraft:misc/spyglass_scope";

pub(crate) fn frames(
    packs: &PackStack,
    images: &mut Assets<Image>,
) -> Option<(Handle<Image>, u32)> {
    strip(packs, images, PORTAL_SPRITE)
}

pub(crate) fn strip(
    packs: &PackStack,
    images: &mut Assets<Image>,
    sprite: &str,
) -> Option<(Handle<Image>, u32)> {
    let id = ResourceId::parse(sprite).ok()?;
    let bytes = packs.texture(&id).ok()??;
    let strip = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    let (width, height) = strip.dimensions();
    let frames = (height / width.max(1)).max(1);
    let image = Image {
        sampler: bevy::image::ImageSampler::nearest(),
        ..Image::new(
            bevy::render::render_resource::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            strip.into_raw(),
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::default(),
        )
    };
    Some((images.add(image), frames))
}
