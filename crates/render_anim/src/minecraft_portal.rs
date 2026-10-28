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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FriendPortal {
    effect: portal::PortalEffect,
    clock: f64,
    age: u64,
}

pub(crate) fn friend_inside(
    feet: Option<[f64; 3]>,
    spectator: bool,
    axis_at: impl Fn((i32, i32, i32)) -> Option<portal::PortalAxis>,
) -> bool {
    !spectator && feet.is_some_and(|feet| portal::touched_portal(feet, axis_at).is_some())
}

pub(crate) fn friend_portal_tick(
    friend: &mut FriendPortal,
    seconds: f64,
    inside: bool,
) -> (bool, Option<McPortalOverlay>) {
    friend.clock += seconds.max(0.0);
    let ticks = (friend.clock / portal::TICK_SECONDS).floor();
    friend.clock -= ticks * portal::TICK_SECONDS;
    let ticks = ticks.min(f64::from(u32::MAX)) as u32;
    for _ in 0..ticks {
        friend.age += 1;
        friend.effect.tick(inside);
    }
    let partial = (friend.clock / portal::TICK_SECONDS) as f32;
    let shown = friend.effect.shown(partial);
    let overlay = (shown > 0.0).then(|| McPortalOverlay {
        alpha: portal::overlay_alpha(shown),
        frame: friend.age,
    });
    (inside && ticks > 0, overlay)
}

pub(crate) const FREEZE_OUTLINE: &str = "minecraft:misc/powder_snow_outline";
pub(crate) const SPYGLASS_SCOPE: &str = "minecraft:misc/spyglass_scope";
pub(crate) const PUMPKIN_BLUR: &str = "minecraft:misc/pumpkinblur";

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

pub(crate) fn blurred(
    packs: &PackStack,
    images: &mut Assets<Image>,
    sprite: &str,
) -> Option<Handle<Image>> {
    let (handle, _) = strip(packs, images, sprite)?;
    images.get_mut(&handle)?.sampler = bevy::image::ImageSampler::linear();
    Some(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICK: f64 = portal::TICK_SECONDS;

    fn ticks(friend: &mut FriendPortal, count: usize, inside: bool) -> (bool, Option<McPortalOverlay>) {
        let mut last = (false, None);
        for _ in 0..count {
            last = friend_portal_tick(friend, TICK, inside);
        }
        last
    }

    fn close(overlay: Option<McPortalOverlay>, intensity: f32) -> bool {
        overlay.is_some_and(|shown| (shown.alpha - portal::overlay_alpha(intensity)).abs() < 1.0e-4)
    }

    #[test]
    fn a_friends_portal_overlay_rises_inside_falls_outside_and_restarts_on_a_new_level() {
        let mut friend = FriendPortal::default();
        assert_eq!(friend_portal_tick(&mut friend, TICK / 2.0, true), (false, None));
        let (closes, overlay) = friend_portal_tick(&mut friend, TICK / 2.0, true);
        assert!(closes, "the first tick inside closes the inventory");
        assert_eq!(overlay, None, "drawn from the tick before");
        let (closes, overlay) = friend_portal_tick(&mut friend, TICK / 2.0, true);
        assert!(!closes && close(overlay, 0.00625), "it is drawn between ticks");
        assert_eq!(overlay.map(|shown| shown.frame), Some(1));
        friend_portal_tick(&mut friend, TICK / 2.0, true);
        let (closes, overlay) = ticks(&mut friend, 18, true);
        assert!(closes && close(overlay, 0.2375), "{overlay:?}");
        assert_eq!(overlay.map(|shown| shown.frame), Some(20));
        let (_, overlay) = ticks(&mut friend, 100, true);
        assert_eq!(overlay.map(|shown| shown.alpha), Some(1.0));
        let (closes, overlay) = ticks(&mut friend, 10, false);
        assert!(!closes && close(overlay, 0.55), "{overlay:?}");
        let (_, overlay) = ticks(&mut friend, 15, false);
        assert_eq!(overlay, None, "it is gone a second after stepping out");
        ticks(&mut friend, 40, true);
        friend = FriendPortal::default();
        let (closes, overlay) = ticks(&mut friend, 2, true);
        assert!(closes && close(overlay, 0.0125), "a new level starts it again");
    }

    #[test]
    fn a_friend_is_inside_a_portal_only_alive_and_not_a_spectator() {
        let axis = |pos: (i32, i32, i32)| (pos == (0, 64, 0)).then_some(portal::PortalAxis::Z);
        assert!(friend_inside(Some([0.5, 64.0, 0.5]), false, axis));
        assert!(!friend_inside(Some([0.5, 64.0, 0.5]), true, axis), "a spectator");
        assert!(!friend_inside(None, false, axis), "dead");
        assert!(!friend_inside(Some([5.5, 64.0, 5.5]), false, axis));
        let mut friend = FriendPortal::default();
        let inside = friend_inside(Some([0.5, 64.0, 0.5]), true, axis);
        assert_eq!(ticks(&mut friend, 40, inside), (false, None), "a spectator sees no overlay");
    }
}
