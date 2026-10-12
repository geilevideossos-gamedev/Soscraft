use std::collections::{BTreeMap, BTreeSet};

use glam::DVec3;
use minecraft_net::messages::{Blob, Nbt};
use minecraft_net::{Dimension, Lane, StateFrame, ToClient};
use minecraft_terrain::client_mobs::ServerMob;
use minecraft_terrain::mob_tracking::{MobEvent, tracking_range, tracks};
use minecraft_terrain::server::EntitySnapshot;
use minecraftoss_core::nbt::Tag;

pub type ChunkXz = (i32, i32);

const ITEMS: &str = "minecraft:item";
const ORBS: &str = "minecraft:experience_orb";
const TNT: &str = "minecraft:tnt";
const FALLING: &str = "minecraft:falling_block";
const THROWN: &str = "minecraft:snowball";
const CLOUDS: &str = "minecraft:area_effect_cloud";
const CRYSTALS: &str = "minecraft:end_crystal";
const FRAMES: &str = "minecraft:item_frame";

#[derive(Default)]
pub struct MobSender {
    level: Option<Dimension>,
    sent: BTreeSet<u64>,
    held: BTreeMap<u64, Tag>,
    tick: u32,
}

impl MobSender {
    pub fn follow(&mut self, level: Option<Dimension>) {
        if self.level != level {
            self.level = level;
            self.sent.clear();
            self.held.clear();
        }
    }

    pub fn sent(&self) -> &BTreeSet<u64> {
        &self.sent
    }

    pub fn held(&self) -> impl Iterator<Item = u64> + '_ {
        self.held.keys().copied()
    }

    pub fn events<'a>(
        &mut self,
        events: impl IntoIterator<Item = &'a MobEvent>,
        moves: Option<&[ServerMob]>,
        has_chunk: impl Fn(ChunkXz) -> bool,
        out: &mut Vec<ToClient>,
    ) {
        let at = |id: u64, tag: &Tag| {
            moves
                .and_then(|moves| moves.iter().find(|mob| mob.id == id))
                .map(|mob| mob.position)
                .or_else(|| position(tag))
        };
        let shown = |id: u64, tag: &Tag| at(id, tag).is_some_and(|at| has_chunk(chunk_of(at)));
        for event in events {
            match event {
                MobEvent::Add(id, tag) => {
                    if shown(*id, tag) {
                        self.held.remove(id);
                        self.sent.insert(*id);
                        out.push(ToClient::MobAdd {
                            id: *id,
                            tag: Nbt(minecraftoss_core::nbt::write(tag, "")),
                        });
                    } else {
                        self.held.insert(*id, tag.clone());
                    }
                }
                MobEvent::Look(id, tag) => {
                    if self.sent.contains(id) {
                        out.push(ToClient::MobLook {
                            id: *id,
                            look: Blob(minecraftoss_core::nbt::write(tag, "")),
                        });
                    } else if let Some(held) = self.held.get_mut(id) {
                        *held = tag.clone();
                    }
                }
                MobEvent::Remove(ids) => {
                    let gone: Vec<u64> = ids
                        .iter()
                        .copied()
                        .filter(|id| {
                            self.held.remove(id);
                            self.sent.remove(id)
                        })
                        .collect();
                    if !gone.is_empty() {
                        out.push(ToClient::MobRemove { ids: gone });
                    }
                }
            }
        }
        let ready: Vec<u64> = self
            .held
            .iter()
            .filter(|(id, tag)| shown(**id, tag))
            .map(|(id, _)| *id)
            .collect();
        for id in ready {
            if let Some(tag) = self.held.remove(&id) {
                self.sent.insert(id);
                out.push(ToClient::MobAdd {
                    id,
                    tag: Nbt(minecraftoss_core::nbt::write(&tag, "")),
                });
            }
        }
    }

    pub fn frame(
        &mut self,
        epoch: u32,
        moves: Option<&[ServerMob]>,
        snapshot: Option<&EntitySnapshot>,
        around: (DVec3, i32),
        has_chunk: impl Fn(ChunkXz) -> bool,
    ) -> Option<StateFrame> {
        let moves = moves?;
        let mobs: Vec<ServerMob> = moves
            .iter()
            .filter(|mob| self.sent.contains(&mob.id))
            .copied()
            .collect();
        let mobs = minecraft_net::encode(Lane::State, &mobs).ok()?;
        let mut entities = Vec::new();
        if let Some(bytes) = snapshot
            .map(|snapshot| near(snapshot, around, has_chunk))
            .and_then(|near| minecraft_net::encode(Lane::State, &near).ok())
        {
            entities = bytes;
        }
        self.tick = self.tick.wrapping_add(1);
        Some(StateFrame {
            tick: self.tick,
            epoch,
            mobs: Blob(mobs),
            entities: Blob(entities),
        })
    }
}

fn position(tag: &Tag) -> Option<DVec3> {
    let pos = tag.get("Pos")?.as_list()?;
    let axis = |i: usize| pos.get(i).and_then(Tag::as_f64);
    Some(DVec3::new(axis(0)?, axis(1)?, axis(2)?))
}

fn chunk_of(at: DVec3) -> ChunkXz {
    ((at.x.floor() as i32) >> 4, (at.z.floor() as i32) >> 4)
}

pub fn near(
    snapshot: &EntitySnapshot,
    (feet, view_distance): (DVec3, i32),
    has_chunk: impl Fn(ChunkXz) -> bool,
) -> EntitySnapshot {
    let keeps = |kind: &str, at: DVec3| {
        has_chunk(chunk_of(at)) && tracks(tracking_range(kind), view_distance, feet.to_array(), at)
    };
    let array = DVec3::from_array;
    let block = |pos: [i32; 3]| DVec3::new(f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2]));
    let mut eyes = snapshot.eyes.clone();
    eyes.views.retain(|eye| keeps(THROWN, eye.position));
    eyes.shattered.retain(|pos| keeps(THROWN, block(*pos)));
    EntitySnapshot {
        items: filtered(&snapshot.items, |item| keeps(ITEMS, array(item.position))),
        tnt: filtered(&snapshot.tnt, |tnt| keeps(TNT, array(tnt.position))),
        falling: filtered(&snapshot.falling, |falling| {
            keeps(FALLING, array(falling.position))
        }),
        orbs: filtered(&snapshot.orbs, |orb| keeps(ORBS, array(orb.position))),
        thrown_views: filtered(&snapshot.thrown_views, |thrown| {
            keeps(THROWN, thrown.position)
        }),
        clouds: filtered(&snapshot.clouds, |cloud| keeps(CLOUDS, cloud.position)),
        tridents: filtered(&snapshot.tridents, |trident| {
            keeps(THROWN, trident.position)
        }),
        eyes,
        crystals: filtered(&snapshot.crystals, |crystal| {
            keeps(CRYSTALS, crystal.position)
        }),
        dragons: snapshot.dragons.clone(),
        dragon_fireballs: snapshot.dragon_fireballs.clone(),
        breath_bursts: snapshot.breath_bursts.clone(),
        gateways: snapshot.gateways.clone(),
        frames: filtered(&snapshot.frames, |frame| keeps(FRAMES, frame.position)),
        dragon_breath: snapshot.dragon_breath.clone(),
        dragon_explosions: snapshot.dragon_explosions.clone(),
    }
}

fn filtered<T: Clone>(all: &[T], keep: impl Fn(&T) -> bool) -> Vec<T> {
    all.iter().filter(|value| keep(value)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::server::ServerItem;

    fn tag_at(x: f64, z: f64) -> Tag {
        minecraftoss_core::snbt::parse_compound(&format!(
            "{{id:\"minecraft:cow\",Pos:[{x}d,64.0d,{z}d]}}"
        ))
        .expect("the tag")
    }

    fn mob(id: u64, x: f64, z: f64) -> ServerMob {
        ServerMob {
            id,
            position: DVec3::new(x, 64.0, z),
            y_rot: 0.0,
            x_rot: 0.0,
            y_head_rot: 0.0,
            on_ground: true,
            baby: false,
            eye_height: 1.3,
            max_head_y_rot: 75.0,
            update_interval: 3,
            sync: Default::default(),
            resting: false,
            hurts: 0,
            dead: false,
            swings: 0,
            size: (0.9, 1.4),
            vehicle: None,
            show: Default::default(),
        }
    }

    fn kinds(out: &[ToClient]) -> Vec<String> {
        out.iter()
            .map(|message| match message {
                ToClient::MobAdd { id, .. } => format!("add {id}"),
                ToClient::MobLook { id, .. } => format!("look {id}"),
                ToClient::MobRemove { ids } => format!("remove {ids:?}"),
                other => format!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn an_add_waits_for_its_chunk_and_a_remove_cancels_it() {
        let mut sender = MobSender::default();
        let mut out = Vec::new();
        let events = [
            MobEvent::Add(7, tag_at(80.5, 0.5)),
            MobEvent::Add(8, tag_at(2.5, 0.5)),
        ];
        sender.events(&events, None, |chunk| chunk == (0, 0), &mut out);
        assert_eq!(kinds(&out), vec!["add 8"]);
        assert_eq!(sender.held().collect::<Vec<_>>(), vec![7]);
        out.clear();
        let moved = [mob(7, 3.5, 0.5)];
        sender.events(&[], Some(&moved), |chunk| chunk == (0, 0), &mut out);
        assert_eq!(
            kinds(&out),
            vec!["add 7"],
            "the mob walked into a chunk the friend has"
        );
        out.clear();
        let events = [
            MobEvent::Add(9, tag_at(80.5, 0.5)),
            MobEvent::Remove(vec![9, 8]),
        ];
        sender.events(&events, None, |chunk| chunk == (0, 0), &mut out);
        assert_eq!(kinds(&out), vec!["remove [8]"], "the held add is dropped");
        assert_eq!(sender.held().count(), 0);
        assert_eq!(sender.sent().iter().copied().collect::<Vec<_>>(), vec![7]);
    }

    #[test]
    fn a_look_follows_its_add_and_a_held_add_takes_the_newer_look() {
        let mut sender = MobSender::default();
        let mut out = Vec::new();
        let events = [
            MobEvent::Look(3, tag_at(1.5, 1.5)),
            MobEvent::Add(4, tag_at(40.5, 1.5)),
            MobEvent::Look(4, tag_at(41.5, 1.5)),
        ];
        sender.events(&events, None, |chunk| chunk == (0, 0), &mut out);
        assert!(out.is_empty(), "{:?}", kinds(&out));
        sender.events(&[], None, |_| true, &mut out);
        let [ToClient::MobAdd { id: 4, tag }] = out.as_slice() else {
            panic!("{:?}", kinds(&out));
        };
        let shown = minecraftoss_core::nbt::parse(&tag.0).expect("the tag");
        assert_eq!(shown, tag_at(41.5, 1.5));
        out.clear();
        sender.events(
            &[MobEvent::Look(4, tag_at(42.5, 1.5))],
            None,
            |_| true,
            &mut out,
        );
        assert_eq!(kinds(&out), vec!["look 4"]);
    }

    #[test]
    fn a_mob_removed_and_added_again_in_one_frame_is_removed_first() {
        let mut sender = MobSender::default();
        let mut out = Vec::new();
        sender.events(
            &[MobEvent::Add(5, tag_at(1.5, 1.5))],
            None,
            |_| true,
            &mut out,
        );
        out.clear();
        let events = [
            MobEvent::Remove(vec![5]),
            MobEvent::Add(5, tag_at(1.5, 1.5)),
        ];
        sender.events(&events, None, |_| true, &mut out);
        assert_eq!(kinds(&out), vec!["remove [5]", "add 5"]);
        assert!(sender.sent().contains(&5));
    }

    #[test]
    fn a_new_level_forgets_what_was_sent() {
        let mut sender = MobSender::default();
        sender.follow(Some(Dimension::Overworld));
        let mut out = Vec::new();
        sender.events(
            &[MobEvent::Add(5, tag_at(1.5, 1.5))],
            None,
            |_| true,
            &mut out,
        );
        sender.follow(Some(Dimension::Overworld));
        assert!(sender.sent().contains(&5));
        sender.follow(Some(Dimension::Nether));
        assert!(sender.sent().is_empty());
    }

    fn item(id: i32, x: f64) -> ServerItem {
        ServerItem {
            id,
            item: "minecraft:dirt".to_owned(),
            count: 1,
            components: None,
            position: [x, 64.0, 0.5],
            previous_position: [x, 64.0, 0.5],
            velocity: [0.0; 3],
            age: 0,
            pickup_delay: 0,
            on_ground: true,
            on_fire: false,
        }
    }

    #[test]
    fn the_state_frame_holds_only_sent_mobs_and_entities_in_range() {
        let mut sender = MobSender::default();
        let mut out = Vec::new();
        sender.events(
            &[MobEvent::Add(7, tag_at(1.5, 0.5))],
            None,
            |_| true,
            &mut out,
        );
        let moves = [mob(7, 1.5, 0.5), mob(9, 2.5, 0.5)];
        let snapshot = EntitySnapshot {
            items: vec![item(1, 10.5), item(2, 120.5), item(3, 40.5)],
            ..EntitySnapshot::default()
        };
        let around = (DVec3::new(0.5, 64.0, 0.5), 8);
        assert!(
            sender
                .frame(0, None, Some(&snapshot), around, |_| true)
                .is_none()
        );
        let frame = sender
            .frame(0, Some(&moves), Some(&snapshot), around, |chunk| {
                chunk.0 < 2
            })
            .expect("a frame");
        let first = frame.tick;
        let mobs: Vec<ServerMob> = minecraft_net::decode(Lane::State, &frame.mobs.0).unwrap();
        assert_eq!(mobs.iter().map(|mob| mob.id).collect::<Vec<_>>(), vec![7]);
        let near: EntitySnapshot = minecraft_net::decode(Lane::State, &frame.entities.0).unwrap();
        assert_eq!(
            near.items.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![1],
            "items go 6 chunks and only in chunks the friend has"
        );
        let next = sender
            .frame(0, Some(&moves), None, around, |_| true)
            .expect("a frame");
        assert!(next.tick > first);
        assert!(next.entities.0.is_empty());
    }
}
