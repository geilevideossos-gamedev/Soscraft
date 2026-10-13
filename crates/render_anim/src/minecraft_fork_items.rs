use std::collections::HashMap;
use std::sync::{Arc, mpsc};

use asset_game::{FamilyKey, LoadoutRules, WeaponRegistry, WeaponSelection};
use minecraft_survival::gun_crafting;
use minecraft_survival::gun_names::{self, GunContent, GunNames};
use minecraft_net::Dimension;
use minecraft_survival::guns::{self, Hotbar, Settled, Spent};
use minecraft_survival::player_nbt::Place;
use minecraft_terrain::crafting::Grid;
use minecraft_terrain::fork_items::{self, Bucket, Icon, Kind};
use minecraft_terrain::fork_look;
use minecraft_terrain::game_mode::PlayerMode;
use minecraft_terrain::gun_mods::GunRules;
use minecraft_terrain::item_icon::SpriteClock;
use minecraft_terrain::mesh::{Atlas, AtlasPatch};
use minecraft_terrain::pack::{PackStack, ResourceId};
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

    fn bucket(&self, index: u32) -> Option<Bucket> {
        Some(match self.0.facts_of(index)?.weap_class {
            0 => Bucket::AssaultRifle,
            1 => Bucket::Sniper,
            2 => Bucket::Lmg,
            3 => Bucket::Smg,
            weapon_iw4::WEAPCLASS_SPREAD => Bucket::Shotgun,
            weapon_iw4::WEAPCLASS_PISTOL => Bucket::Pistol,
            7 => Bucket::Launcher,
            _ => return None,
        })
    }
}

pub(crate) struct ForkRules(Arc<WeaponRegistry>);

impl GunRules for ForkRules {
    fn accepts(&self, id: &str, attachments: &[String]) -> bool {
        gun_crafting::Rules(&RegistryGuns(&self.0)).accepts(id, attachments)
    }

    fn clip_size(&self, id: &str, attachments: &[String]) -> Option<i32> {
        gun_crafting::Rules(&RegistryGuns(&self.0)).clip_size(id, attachments)
    }
}

type Decoded = Vec<(String, image::RgbaImage)>;
type Sprites = Vec<(&'static str, image::RgbaImage)>;

const GUN_STAND_IN: &str = "minecraft:item/crossbow_standby";
const ITEM_STAND_IN: &str = "minecraft:item/firework_star";

#[derive(Default)]
pub(crate) struct ForkLook {
    registry: Option<Arc<WeaponRegistry>>,
    names: GunNames,
    named: bool,
    pending: Option<mpsc::Receiver<Decoded>>,
    images: Option<HashMap<String, image::RgbaImage>>,
    gun_pending: Option<mpsc::Receiver<Decoded>>,
    gun_icons: Option<HashMap<String, image::RgbaImage>>,
    world: Option<Sprites>,
    patched: Option<(usize, u64)>,
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
            if let Some(registry) = registry {
                let names = registry
                    .weapon_families()
                    .families()
                    .iter()
                    .flat_map(|family| {
                        family.attachments.iter().map(|choice| choice.name.as_str())
                    });
                let missing = gun_crafting::unmapped(names);
                if !missing.is_empty() {
                    diag::warn!(
                        World,
                        "MW2 attachments with no Minecraft item, so they cannot be crafted: {}",
                        missing.join(", ")
                    );
                }
            }
        }
    }

    pub(crate) fn rules(&self) -> Option<ForkRules> {
        self.registry.clone().map(ForkRules)
    }

    pub(crate) fn special(&self, inventory: &Inventory, grid: Grid) -> Option<ItemStack> {
        let rules = self.rules()?;
        gun_crafting::result(inventory, grid, &rules)
    }

    pub(crate) fn weapon(&mut self, stack: &ItemStack) -> Option<u32> {
        fork_items::get(&stack.id).filter(|row| row.kind == Kind::Gun)?;
        let registry = self.registry.clone()?;
        self.names.weapon_of(&RegistryGuns(&registry), stack)
    }

    pub(crate) fn clip_bar(
        &mut self,
        stack: &ItemStack,
        weapon: Option<u32>,
    ) -> Option<(u32, u32)> {
        let registry = self.registry.clone()?;
        let size = self.names.clip_size(&RegistryGuns(&registry), weapon?);
        gun_names::clip_bar(stack, size)
    }

    pub(crate) fn lines(
        &mut self,
        stack: &ItemStack,
        weapon: Option<u32>,
        language: &HashMap<String, String>,
    ) -> Vec<String> {
        let Some(row) = fork_items::get(&stack.id).filter(|row| row.kind == Kind::Gun) else {
            return match fork_items::get(&stack.id) {
                Some(row) if row.kind == Kind::Attachment => {
                    vec![gun_crafting::ATTACH_HINT.to_owned()]
                }
                _ => Vec::new(),
            };
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

    fn gun_icons_ready(&mut self) -> bool {
        if self.gun_icons.is_some() {
            return true;
        }
        if let Some(pending) = self.gun_pending.as_ref() {
            let decoded = match pending.try_recv() {
                Ok(decoded) => decoded,
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => Vec::new(),
            };
            self.gun_icons = Some(decoded.into_iter().collect());
            self.gun_pending = None;
            return true;
        }
        let Some(registry) = self.registry.clone() else {
            return false;
        };
        let content = RegistryGuns(&registry);
        let mut jobs = Vec::new();
        for row in fork_items::rows() {
            if row.icon != Icon::KillIcon {
                continue;
            }
            let Some(index) = self.names.weapon(&content, row.id, &[]) else {
                continue;
            };
            let Some(image) = registry
                .kill_icon_image_of(index)
                .or_else(|| registry.kill_icon_of(index))
            else {
                continue;
            };
            let key = match registry
                .namespace_of(index)
                .map(|namespace| namespace.as_str())
            {
                Some(namespace) if namespace != "iw4" => format!("{namespace}:material/{image}"),
                _ => image.to_owned(),
            };
            let flip = registry
                .facts_of(index)
                .is_some_and(|facts| facts.flip_kill_icon);
            jobs.push((row.id.to_owned(), key, flip));
        }
        let (send, receive) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("minecraft-gun-icons".into())
            .spawn(move || {
                let _ = send.send(decode_guns(&jobs));
            });
        if spawned.is_ok() {
            self.gun_pending = Some(receive);
            false
        } else {
            self.gun_icons = Some(HashMap::new());
            true
        }
    }

    fn world_sprites(&mut self, packs: &PackStack) -> Option<Sprites> {
        let material = fork_items::rows()
            .iter()
            .find(|row| matches!(row.icon, Icon::Material(_)))?;
        if self.waiting(material.id) || !self.gun_icons_ready() {
            return None;
        }
        let stand_in = |texture: &str| {
            let id = ResourceId::parse(texture).ok()?;
            let bytes = packs.texture(&id).ok().flatten()?;
            image::load_from_memory(&bytes)
                .ok()
                .map(|image| image.to_rgba8())
        };
        let (gun_stand_in, item_stand_in) = (stand_in(GUN_STAND_IN), stand_in(ITEM_STAND_IN));
        let mut missing = Vec::new();
        let mut sprites = Vec::new();
        for row in fork_items::rows() {
            let found = match row.icon {
                Icon::KillIcon => self.gun_icons.as_ref().and_then(|icons| icons.get(row.id)),
                Icon::Material(stem) => self.images.as_ref().and_then(|images| images.get(stem)),
                Icon::Sprite { .. } => continue,
            };
            let icon = match found {
                Some(icon) => icon,
                None => {
                    missing.push(row.id);
                    let stand_in = if row.kind == Kind::Gun {
                        &gun_stand_in
                    } else {
                        &item_stand_in
                    };
                    let Some(icon) = stand_in.as_ref() else {
                        continue;
                    };
                    icon
                }
            };
            sprites.push((row.id, fork_look::sprite(icon, false)));
        }
        if !missing.is_empty() {
            diag::warn!(
                World,
                "Minecraft fork items with a stand-in look on the ground: {}",
                missing.join(", ")
            );
        }
        Some(sprites)
    }

    pub(crate) fn dress_world(
        &mut self,
        packs: &mut PackStack,
        atlas: &Arc<Atlas>,
        (generation, active): (u64, bool),
        patches: &mut Vec<AtlasPatch>,
    ) {
        if self.world.is_none() {
            self.world = self.world_sprites(packs);
        }
        let Some(world) = self.world.as_ref() else {
            return;
        };
        let installed = world.first().is_none_or(|(id, _)| {
            ResourceId::parse(id)
                .is_ok_and(|id| packs.item_definition(&id).ok().flatten().is_some())
        });
        if !installed {
            for (id, sprite) in world {
                fork_look::install(packs, id, sprite);
            }
        }
        let key = (Arc::as_ptr(atlas) as usize, generation);
        if active && (!installed || self.patched != Some(key)) {
            patches.extend(
                world
                    .iter()
                    .filter_map(|(id, sprite)| fork_look::patch(atlas, id, sprite)),
            );
        }
        self.patched = active.then_some(key);
    }

    pub(crate) fn first_loadout(
        &mut self,
        player: (&mut Inventory, usize, &mut Vec<String>),
        alive: bool,
        class: Option<minecraft_survival::loadout::Class>,
    ) -> Vec<ItemStack> {
        let Some(registry) = self.registry.clone().filter(|_| alive) else {
            return Vec::new();
        };
        let content = RegistryGuns(&registry);
        let guns = (&mut self.names, &content);
        let Some(given) =
            minecraft_survival::loadout::give_first_loadout(player, class.as_ref(), guns)
        else {
            return Vec::new();
        };
        if !given.stood_in.is_empty() {
            diag::warn!(
                World,
                "Minecraft first-join loadout: no item for {}, gave the first gun of its class",
                given.stood_in.join(", ")
            );
        }
        given.dropped
    }

    pub(crate) fn settle(
        &mut self,
        inventory: &mut Inventory,
        mode: &PlayerMode,
        client: u32,
        hotbar: Hotbar<'_>,
        flare_at: impl Fn([f32; 3]) -> Option<Place>,
    ) -> Option<Settled> {
        let registry = self.registry.clone()?;
        let spent = spent(client, flare_at);
        let content = RegistryGuns(&registry);
        let settled =
            guns::settle_arsenal(inventory, mode, spent, (&mut self.names, &content), hotbar);
        sim::voxel::set_arsenal(client, published(&settled.arsenal));
        Some(settled)
    }
}

pub(crate) fn class_of(
    authority: &sim::SimWorld,
    client: sim::ClientId,
) -> Option<minecraft_survival::loadout::Class> {
    let spec = authority.client_meta(client)?.loadout.as_ref()?;
    Some(minecraft_survival::loadout::Class {
        primary: spec.primary,
        secondary: spec.secondary,
        lethal: spec.lethal,
        tactical: spec.tactical,
    })
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
                alt_ammo: gun.alt_ammo,
            })
            .collect(),
        selected: arsenal.selected,
        stock: arsenal.stock,
        lethal: arsenal.lethal,
        tactical: arsenal.tactical,
        endless: arsenal.endless,
    }
}

pub(crate) fn flare_place(
    dimension: Dimension,
    origin: [f64; 3],
    yaw: f32,
) -> impl Fn([f32; 3]) -> Option<Place> {
    move |pos| {
        minecraft_survival::bands::holds(dimension, f64::from(pos[2]))
            .then(|| Place::standing(dimension, sim::voxel::to_block(origin, pos), yaw, 0.0))
    }
}

fn spent(client: u32, flare_at: impl Fn([f32; 3]) -> Option<Place>) -> Vec<Spent> {
    sim::voxel::take_spent(client)
        .into_iter()
        .filter_map(|spent| match spent {
            sim::voxel::Spent::Rounds { ammo, rounds } => Some(Spent::Rounds { ammo, rounds }),
            sim::voxel::Spent::Grenade { weapon, delta } => Some(Spent::Grenade { weapon, delta }),
            sim::voxel::Spent::Clip {
                weapon,
                clip,
                clip_left,
                alt_clip,
            } => Some(Spent::Clip {
                weapon,
                clip,
                clip_left,
                alt_clip,
            }),
            sim::voxel::Spent::Flare { pos } => {
                let at = flare_at(pos);
                if at.is_none() {
                    diag::warn!(World, "Minecraft tactical insertion planted outside the player's dimension");
                }
                at.map(|at| Spent::Flare { at })
            }
            sim::voxel::Spent::FlareGone => Some(Spent::FlareGone),
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

fn decode_guns(jobs: &[(String, String, bool)]) -> Decoded {
    let Ok(games) = asset_transport::games_root_from_env() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (id, key, flip) in jobs {
        match asset_material::decode_ui_image(&games.0, key) {
            Ok(Some((width, height, pixels))) => {
                if let Some(mut image) = image::RgbaImage::from_raw(width, height, pixels) {
                    if *flip {
                        image::imageops::flip_horizontal_in_place(&mut image);
                    }
                    out.push((id.clone(), image));
                }
            }
            Ok(None) => diag::warn!(World, "Minecraft gun icon `{key}` for `{id}` not found"),
            Err(error) => diag::warn!(World, "Minecraft gun icon `{key}` for `{id}`: {error}"),
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

    #[test]
    fn the_creative_combat_tab_and_search_list_the_fork_items_by_their_mw2_names() {
        use crate::minecraft_creative::{entry_stacks, search_lines};
        use minecraft_terrain::creative_search::{Searched, search};
        use minecraft_terrain::creative_variants::VariantData;
        let mut tabs = minecraft_terrain::creative_tabs::fallback_tabs(Vec::new(), "test");
        assert!(fork_items::add_to_creative(&mut tabs));
        let mut catalog = minecraftoss_player::item_catalog::ItemCatalog::default();
        fork_items::load(&mut catalog);
        let recipes = minecraftoss_player::crafting::RecipeBook::default()
            .with_item_catalog(Arc::new(catalog));
        let variants = VariantData::default();
        let combat = tabs.tab(fork_items::CREATIVE_TAB).unwrap();
        let shown = entry_stacks(
            &combat.displayed().collect::<Vec<_>>(),
            &recipes,
            Some(&variants),
        );
        let ids: Vec<&str> = shown.iter().map(|stack| stack.id.as_str()).collect();
        let table: Vec<&str> = fork_items::rows().iter().map(|row| row.id).collect();
        assert_eq!(ids, table);
        let m4 = shown.iter().find(|stack| stack.id == "iw4l:m4").unwrap();
        assert_eq!(m4.components, Some(fork_items::loaded_gun()));
        assert_eq!(m4.max, 1);
        let shield = shown
            .iter()
            .find(|stack| stack.id == "iw4l:riotshield")
            .unwrap();
        assert_eq!(shield.components, None);
        let rockets = shown
            .iter()
            .find(|stack| stack.id == "iw4l:rocket")
            .unwrap();
        assert_eq!((rockets.components.clone(), rockets.max), (None, 16));
        let mut language = HashMap::new();
        language.insert(
            "item.iw4l.cheytac".to_owned(),
            "Intervention (MW2)".to_owned(),
        );
        fork_items::add_names(&mut language);
        let entries = tabs.search(false);
        let stacks = entry_stacks(
            &entries.iter().collect::<Vec<_>>(),
            &recipes,
            Some(&variants),
        );
        let lines: Vec<Vec<String>> = stacks
            .iter()
            .map(|stack| search_lines(stack, Some(&language), Some(&variants)))
            .collect();
        let searched: Vec<Searched> = stacks
            .iter()
            .zip(&lines)
            .map(|(stack, lines)| Searched {
                id: &stack.id,
                lines,
                tags: &[],
            })
            .collect();
        let found = |query: &str| -> Vec<&str> {
            search(query, &searched)
                .into_iter()
                .map(|index| stacks[index].id.as_str())
                .collect()
        };
        assert!(found("M4A1").contains(&"iw4l:m4"));
        assert!(found("m4a1").contains(&"iw4l:m4"));
        assert_eq!(found("Intervention (MW2)"), ["iw4l:cheytac"]);
        assert!(found("frag").contains(&"iw4l:frag_grenade"));
    }
}
