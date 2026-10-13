use std::collections::HashMap;
use std::sync::{Arc, mpsc};

use asset_game::{FamilyKey, LoadoutRules, WeaponRegistry, WeaponSelection};
use minecraft_survival::gun_names::{self, GunContent, GunNames};
use minecraft_survival::guns::{self, Hotbar, Settled, Spent};
use minecraft_terrain::fork_items::{self, Icon, Kind};
use minecraft_terrain::game_mode::PlayerMode;
use minecraft_terrain::item_icon::SpriteClock;
use minecraft_terrain::pack::PackStack;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use playerstate_iw4::PlayerState;

pub(crate) struct RegistryGuns<'a>(pub(crate) &'a WeaponRegistry);

impl GunContent for RegistryGuns<'_> {
    fn configuration(&self, weapon: &str, attachments: &[String]) -> Option<u32> {
        let key = FamilyKey::parse(weapon)?;
        match self.0.weapon_families().find(&key) {
            Some(family) => self
                .0
                .resolve_configuration(
                    &WeaponSelection::with(family.key.clone(), attachments),
                    LoadoutRules { max_attachments: 2 },
                )
                .ok()
                .map(|resolved| resolved.id),
            None if attachments.is_empty() => self.0.resolve_index(weapon).ok().flatten(),
            None => None,
        }
    }

    fn selection(&self, index: u32) -> Option<(String, Vec<String>)> {
        if let Some(selection) = self.0.describe_configuration(index)
            && let Some(family) = &selection.family
        {
            return Some((family.asset_key(), selection.attachments.clone()));
        }
        Some((self.0.namespaced_key_of(index)?, Vec::new()))
    }

    fn clip_size(&self, index: u32) -> Option<i32> {
        self.0
            .facts_of(index)
            .map(|facts| facts.clip_size)
            .filter(|size| *size > 0)
    }
}

type Decoded = Vec<(String, image::RgbaImage)>;

#[derive(Default)]
pub(crate) struct ForkLook {
    registry: Option<Arc<WeaponRegistry>>,
    names: GunNames,
    named: bool,
    pending: Option<mpsc::Receiver<Decoded>>,
    images: Option<HashMap<String, image::RgbaImage>>,
}

impl ForkLook {
    pub(crate) fn set_weapons(&mut self, registry: Option<&Arc<WeaponRegistry>>) {
        let same = match (&self.registry, registry) {
            (Some(old), Some(new)) => Arc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.registry = registry.cloned();
            self.names = GunNames::default();
            self.named = false;
        }
    }

    pub(crate) fn weapon(&mut self, stack: &ItemStack) -> Option<u32> {
        fork_items::get(&stack.id).filter(|row| row.kind == Kind::Gun)?;
        let registry = self.registry.clone()?;
        self.names.weapon_of(&RegistryGuns(&registry), stack)
    }

    pub(crate) fn lines(
        &mut self,
        stack: &ItemStack,
        weapon: Option<u32>,
        language: &HashMap<String, String>,
    ) -> Vec<String> {
        let Some(row) = fork_items::get(&stack.id).filter(|row| row.kind == Kind::Gun) else {
            return Vec::new();
        };
        let name = |id: &str| minecraft_terrain::item_icons::item_name(language, id);
        let attachments: Vec<String> = gun_names::attachments(stack)
            .iter()
            .map(|alias| gun_names::attachment_name(alias, name))
            .collect();
        let size = match (weapon, self.registry.clone()) {
            (Some(weapon), Some(registry)) => {
                self.names.clip_size(&RegistryGuns(&registry), weapon)
            }
            _ => None,
        };
        let clips = (
            gun_names::clip(stack, "clip"),
            gun_names::clip(stack, "clip_left"),
        );
        let ammo = row.ammo.map(name);
        gun_names::tooltip_lines(&attachments, clips, size, ammo.as_deref())
    }

    pub(crate) fn name_items(
        &mut self,
        language: &mut HashMap<String, String>,
        strings: Option<&asset_game::LocalizeCatalog>,
    ) -> bool {
        if self.named {
            return false;
        }
        let (Some(registry), Some(strings)) = (self.registry.clone(), strings) else {
            return false;
        };
        self.named = true;
        let content = RegistryGuns(&registry);
        for row in fork_items::rows() {
            let Some(index) = self.names.weapon(&content, row.id, &[]) else {
                continue;
            };
            let text = registry
                .display_name_key_of(index)
                .and_then(|key| strings.text(key));
            if let Some(text) = text.filter(|text| !text.trim().is_empty()) {
                language.insert(row.language_key(), text.to_owned());
            }
        }
        true
    }

    pub(crate) fn waiting(&mut self, id: &str) -> bool {
        let Some(Icon::Material(_)) = fork_items::get(id).map(|row| row.icon) else {
            return false;
        };
        if self.images.is_some() {
            return false;
        }
        if let Some(pending) = self.pending.as_ref() {
            match pending.try_recv() {
                Ok(decoded) => {
                    self.images = Some(decoded.into_iter().collect());
                    self.pending = None;
                    return false;
                }
                Err(mpsc::TryRecvError::Empty) => return true,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.images = Some(HashMap::new());
                    self.pending = None;
                    return false;
                }
            }
        }
        let stems: Vec<&'static str> = fork_items::rows()
            .iter()
            .filter_map(|row| match row.icon {
                Icon::Material(stem) => Some(stem),
                _ => None,
            })
            .collect();
        let (send, receive) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("minecraft-fork-icons".into())
            .spawn(move || {
                let _ = send.send(decode(&stems));
            });
        if spawned.is_ok() {
            self.pending = Some(receive);
            true
        } else {
            self.images = Some(HashMap::new());
            false
        }
    }

    pub(crate) fn icon(
        &self,
        packs: &PackStack,
        id: &str,
        size: usize,
        clock: &SpriteClock,
    ) -> Option<anyhow::Result<Option<image::RgbaImage>>> {
        let row = fork_items::get(id)?;
        Some(match row.icon {
            Icon::KillIcon => Ok(None),
            Icon::Sprite { item, tint } => {
                minecraft_terrain::item_icons::item_icon_at(packs, item, size, clock)
                    .map(|icon| icon.map(|icon| tinted(icon, tint)))
            }
            Icon::Material(stem) => Ok(self
                .images
                .as_ref()
                .and_then(|images| images.get(stem))
                .map(|image| fitted(image, size as u32))),
        })
    }

    pub(crate) fn settle(
        &mut self,
        inventory: &mut Inventory,
        mode: &PlayerMode,
        client: u32,
        hotbar: Hotbar<'_>,
    ) -> Option<Settled> {
        let registry = self.registry.clone()?;
        let spent = spent(client);
        let content = RegistryGuns(&registry);
        let settled =
            guns::settle_arsenal(inventory, mode, spent, (&mut self.names, &content), hotbar);
        sim::voxel::set_arsenal(client, published(&settled.arsenal));
        Some(settled)
    }
}

pub(crate) fn gun_room(ps: &PlayerState, authority: &sim::SimWorld) -> usize {
    let taken = ps
        .weapons
        .iter()
        .filter_map(|&weapon| u32::try_from(weapon).ok().filter(|&weapon| weapon != 0))
        .filter(|&weapon| {
            authority
                .weapon_combat_row(weapon)
                .is_none_or(|facts| facts.inventory_type != 0)
                || sim::voxel::kept_by_mw2(authority.weapon_script_name(weapon))
        })
        .count();
    guns::WEAPON_SLOTS.saturating_sub(taken)
}

fn published(arsenal: &guns::Arsenal) -> sim::voxel::Arsenal {
    sim::voxel::Arsenal {
        guns: arsenal
            .guns
            .iter()
            .map(|gun| sim::voxel::GunSpec {
                weapon: gun.weapon,
                clip: gun.clip,
                clip_left: gun.clip_left,
                alt_clip: gun.alt_clip,
                ammo: gun.ammo,
                rebind: gun.rebind,
            })
            .collect(),
        selected: arsenal.selected,
        stock: arsenal.stock,
        lethal: None,
        tactical: None,
        endless: arsenal.endless,
    }
}

fn spent(client: u32) -> Vec<Spent> {
    sim::voxel::take_spent(client)
        .into_iter()
        .map(|spent| match spent {
            sim::voxel::Spent::Rounds { ammo, rounds } => Spent::Rounds { ammo, rounds },
            sim::voxel::Spent::Grenade { weapon, delta } => Spent::Grenade { weapon, delta },
            sim::voxel::Spent::Clip {
                weapon,
                clip,
                clip_left,
                alt_clip,
            } => Spent::Clip {
                weapon,
                clip,
                clip_left,
                alt_clip,
            },
            sim::voxel::Spent::Flare { pos } => Spent::Flare { pos },
        })
        .collect()
}

fn decode(stems: &[&'static str]) -> Decoded {
    let Ok(games) = asset_transport::games_root_from_env() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for stem in stems {
        match asset_material::decode_ui_image(&games.0, stem) {
            Ok(Some((width, height, pixels))) => {
                if let Some(image) = image::RgbaImage::from_raw(width, height, pixels) {
                    out.push(((*stem).to_owned(), image));
                }
            }
            Ok(None) => diag::warn!(World, "Minecraft fork item icon `{stem}` not found"),
            Err(error) => diag::warn!(World, "Minecraft fork item icon `{stem}`: {error}"),
        }
    }
    out
}

pub(crate) fn tinted(mut icon: image::RgbaImage, tint: Option<[u8; 3]>) -> image::RgbaImage {
    let Some(tint) = tint else {
        return icon;
    };
    for pixel in icon.pixels_mut() {
        for (channel, factor) in pixel.0.iter_mut().zip(tint) {
            *channel = ((u16::from(*channel) * u16::from(factor) + 127) / 255) as u8;
        }
    }
    icon
}

pub(crate) fn fitted(image: &image::RgbaImage, size: u32) -> image::RgbaImage {
    let (width, height) = image.dimensions();
    let mut out = image::RgbaImage::new(size, size);
    if width == 0 || height == 0 || size == 0 {
        return out;
    }
    let scale = (size as f32 / width as f32).min(size as f32 / height as f32);
    let w = ((width as f32 * scale).round() as u32).clamp(1, size);
    let h = ((height as f32 * scale).round() as u32).clamp(1, size);
    let resized = image::imageops::resize(image, w, h, image::imageops::FilterType::Triangle);
    image::imageops::overlay(
        &mut out,
        &resized,
        i64::from((size - w) / 2),
        i64::from((size - h) / 2),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tint_multiplies_the_sprite_and_keeps_alpha() {
        let icon = image::RgbaImage::from_pixel(2, 1, image::Rgba([255, 128, 0, 200]));
        let out = tinted(icon.clone(), Some([214, 120, 78]));
        assert_eq!(out.get_pixel(0, 0).0, [214, 60, 0, 200]);
        assert_eq!(tinted(icon.clone(), None), icon);
    }

    #[test]
    fn a_wide_image_is_fitted_into_the_square_and_centred() {
        let image = image::RgbaImage::from_pixel(128, 64, image::Rgba([10, 20, 30, 255]));
        let out = fitted(&image, 32);
        assert_eq!(out.dimensions(), (32, 32));
        assert_eq!(out.get_pixel(16, 16).0, [10, 20, 30, 255]);
        assert_eq!(out.get_pixel(16, 4).0[3], 0);
        assert_eq!(out.get_pixel(16, 27).0[3], 0);
    }
}
