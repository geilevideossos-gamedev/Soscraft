//! Minecraft's sound events on the Minecraft map, resolved as the game
//! resolves them: `sounds.json` names each event's weighted variants (files,
//! or other events) with their own volume and pitch.
use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::Vec3;
use minecraft_terrain::pack::{PackStack, ResourceId};

const DEFAULT_ATTENUATION: f32 = 16.0;

struct Entry {
    name: String,
    volume: f32,
    pitch: f32,
    weight: u32,
    attenuation: f32,
    /// `name` is another event rather than a file.
    event: bool,
}

pub(crate) struct Sounds {
    events: HashMap<String, Vec<Entry>>,
    files: HashMap<String, Option<Arc<[u8]>>>,
    rng: u64,
    unresolved: std::collections::HashSet<String>,
    /// Requests for the audio system this frame.
    pub(crate) queued: Vec<audio::McSoundRequest>,
}

impl Sounds {
    pub(crate) fn load(packs: &PackStack) -> Self {
        let mut events = HashMap::new();
        let json = ResourceId::parse("minecraft:sounds")
            .ok()
            .and_then(|id| packs.json(&id, "sounds.json").ok().flatten());
        if let Some(entries) = json.as_ref().and_then(serde_json::Value::as_object) {
            for (event, value) in entries {
                let list = value["sounds"]
                    .as_array()
                    .map(|sounds| {
                        sounds
                            .iter()
                            .filter_map(|sound| match sound {
                                serde_json::Value::String(name) => Some(Entry {
                                    name: name.clone(),
                                    volume: 1.0,
                                    pitch: 1.0,
                                    weight: 1,
                                    attenuation: DEFAULT_ATTENUATION,
                                    event: false,
                                }),
                                serde_json::Value::Object(o) => Some(Entry {
                                    name: o.get("name")?.as_str()?.to_owned(),
                                    volume: o.get("volume").and_then(serde_json::Value::as_f64).unwrap_or(1.0) as f32,
                                    pitch: o.get("pitch").and_then(serde_json::Value::as_f64).unwrap_or(1.0) as f32,
                                    weight: o.get("weight").and_then(serde_json::Value::as_u64).unwrap_or(1) as u32,
                                    attenuation: o.get("attenuation_distance").and_then(serde_json::Value::as_f64).map_or(DEFAULT_ATTENUATION, |distance| distance as f32),
                                    event: o.get("type").and_then(serde_json::Value::as_str) == Some("event"),
                                }),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                events.insert(event.clone(), list);
            }
        }
        diag::info!(World, "Minecraft sounds: {} events", events.len());
        Self {
            events,
            files: HashMap::new(),
            rng: 0x9e37_79b9_7f4a_7c15,
            unresolved: std::collections::HashSet::new(),
            queued: Vec::new(),
        }
    }

    pub(crate) fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        ((self.rng >> 40) as u32 as f32) / ((1u32 << 24) as f32)
    }

    /// Plays `event` at a map point (or at the listener), picking one of its
    /// variants by weight.
    pub(crate) fn play(&mut self, packs: &PackStack, event: &str, position: Option<Vec3>, volume: f32, pitch: f32) {
        self.play_depth(packs, event, position, volume, pitch, 0);
    }

    pub(crate) fn play_without_falloff(&mut self, packs: &PackStack, event: &str, position: Option<Vec3>, volume: f32, pitch: f32) {
        let queued = self.queued.len();
        self.play(packs, event, position, volume, pitch);
        for request in &mut self.queued[queued..] {
            request.attenuation = f32::INFINITY;
        }
    }

    fn play_depth(&mut self, packs: &PackStack, event: &str, position: Option<Vec3>, volume: f32, pitch: f32, depth: u8) {
        let key = event.strip_prefix("minecraft:").unwrap_or(event);
        let total: u32 = self.events.get(key).map_or(0, |e| e.iter().map(|x| x.weight).sum());
        if total == 0 || depth > 4 {
            if self.unresolved.insert(key.to_owned()) {
                diag::info!(World, "Minecraft sound event `{event}` has no sounds");
            }
            return;
        }
        let mut pick = (self.random() * total as f32) as u32;
        let Some(entries) = self.events.get(key) else { return };
        let Some(entry) = entries.iter().find(|entry| {
            if pick < entry.weight {
                true
            } else {
                pick -= entry.weight;
                false
            }
        }) else {
            return;
        };
        let (name, entry_volume, entry_pitch, is_event, attenuation) = (entry.name.clone(), entry.volume, entry.pitch, entry.event, entry.attenuation);
        if is_event {
            self.play_depth(packs, &name, position, volume * entry_volume, pitch * entry_pitch, depth + 1);
            return;
        }
        let bytes = self
            .files
            .entry(name.clone())
            .or_insert_with(|| {
                let (namespace, path) = name.split_once(':').unwrap_or(("minecraft", &name));
                let id = ResourceId::parse(&format!("{namespace}:{path}")).ok()?;
                packs.get(&id, &format!("sounds/{path}.ogg")).ok().flatten().map(Arc::from)
            })
            .clone();
        let Some(bytes) = bytes else {
            if self.unresolved.insert(name.clone()) {
                diag::info!(World, "Minecraft sound file `{name}` is not in the pack");
            }
            return;
        };
        self.queued.push(audio::McSoundRequest {
            key: name,
            bytes,
            position,
            volume: volume * entry_volume,
            pitch: pitch * entry_pitch,
            attenuation,
            block: sim::voxel::BLOCK,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_portal_hum_fades_out_at_ten_blocks() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let mut sounds = Sounds::load(&packs);
        sounds.play(&packs, "minecraft:block.portal.ambient", Some(Vec3::ZERO), 0.5, 1.0);
        sounds.play(&packs, "minecraft:block.stone.break", Some(Vec3::ZERO), 1.0, 1.0);
        let reach: Vec<f32> = sounds.queued.iter().map(|request| request.attenuation).collect();
        assert_eq!(reach, [10.0, 16.0]);
    }

    #[test]
    fn a_sound_without_falloff_keeps_its_volume_far_away() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let mut sounds = Sounds::load(&packs);
        sounds.play(&packs, "minecraft:block.stone.break", Some(Vec3::ZERO), 1.0, 1.0);
        sounds.play_without_falloff(&packs, "minecraft:weather.end_flash", Some(Vec3::new(5000.0, 0.0, 0.0)), 1.0, 1.0);
        assert_eq!(sounds.queued.len(), 2);
        assert_eq!(sounds.queued[0].attenuation, 16.0);
        let flash = &sounds.queued[1];
        assert!(flash.key.starts_with("ambient/weather/end_flash"), "{}", flash.key);
        assert_eq!((flash.volume, flash.pitch), (1.0, 1.0));
        assert_eq!(flash.attenuation, f32::INFINITY);
        let reach = flash.attenuation * flash.volume.max(1.0) * flash.block;
        assert_eq!((1.0 - 5000.0 / reach).clamp(0.0, 1.0), 1.0);
    }
}
