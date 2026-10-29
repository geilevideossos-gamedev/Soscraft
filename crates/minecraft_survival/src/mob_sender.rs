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
const OMINOUS_ITEMS: &str = "minecraft:ominous_item_spawner";
const ORBS: &str = "minecraft:experience_orb";
const TNT: &str = "minecraft:tnt";
const FALLING: &str = "minecraft:falling_block";
const THROWN: &str = "minecraft:snowball";
const PARTICLE_RANGE: f64 = 32.0;
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
            skaters: Blob::default(),
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
        ominous_items: filtered(&snapshot.ominous_items, |item| {
            keeps(OMINOUS_ITEMS, array(item.position))
        }),
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
        vibrations: filtered(&snapshot.vibrations, |vibration| {
            has_chunk(chunk_of(vibration.from)) && vibration.from.distance_squared(feet) < PARTICLE_RANGE * PARTICLE_RANGE
        }),
        sculk_souls: filtered(&snapshot.sculk_souls, |at| {
            has_chunk(chunk_of(*at)) && at.distance_squared(feet) < PARTICLE_RANGE * PARTICLE_RANGE
        }),
        sonic_booms: filtered(&snapshot.sonic_booms, |at| {
            has_chunk(chunk_of(*at)) && at.distance_squared(feet) < PARTICLE_RANGE * PARTICLE_RANGE
        }),
        gateways: snapshot.gateways.clone(),
        frames: filtered(&snapshot.frames, |frame| keeps(FRAMES, frame.position)),
        dragon_breath: snapshot.dragon_breath.clone(),
        dragon_explosions: snapshot.dragon_explosions.clone(),
        swirls: filtered(&snapshot.swirls, |swirl| {
            has_chunk(chunk_of(swirl.feet))
                && tracks(swirl.range, view_distance, feet.to_array(), swirl.feet)
        }),
        firework_bursts: filtered(&snapshot.firework_bursts, |burst| {
            keeps(THROWN, burst.position)
        }),
        mob_puffs: filtered(&snapshot.mob_puffs, |puff| {
            has_chunk(chunk_of(puff.feet))
                && tracks(puff.range, view_distance, feet.to_array(), puff.feet)
        }),
    }
}

pub fn take_bursts(snapshot: &mut EntitySnapshot) -> EntitySnapshot {
    let mut bursts = EntitySnapshot {
        breath_bursts: std::mem::take(&mut snapshot.breath_bursts),
        vibrations: std::mem::take(&mut snapshot.vibrations),
        sculk_souls: std::mem::take(&mut snapshot.sculk_souls),
        sonic_booms: std::mem::take(&mut snapshot.sonic_booms),
        dragon_breath: std::mem::take(&mut snapshot.dragon_breath),
        dragon_explosions: std::mem::take(&mut snapshot.dragon_explosions),
        firework_bursts: std::mem::take(&mut snapshot.firework_bursts),
        mob_puffs: std::mem::take(&mut snapshot.mob_puffs),
        ..EntitySnapshot::default()
    };
    bursts.eyes.shattered = std::mem::take(&mut snapshot.eyes.shattered);
    bursts
}

fn add_bursts(into: &mut EntitySnapshot, from: EntitySnapshot) {
    into.eyes.shattered.extend(from.eyes.shattered);
    into.breath_bursts.extend(from.breath_bursts);
    into.vibrations.extend(from.vibrations);
    into.sculk_souls.extend(from.sculk_souls);
    into.sonic_booms.extend(from.sonic_booms);
    into.dragon_breath.extend(from.dragon_breath);
    into.dragon_explosions.extend(from.dragon_explosions);
    into.firework_bursts.extend(from.firework_bursts);
    into.mob_puffs.extend(from.mob_puffs);
}

pub fn keep_bursts(older: &mut EntitySnapshot, newer: &mut EntitySnapshot) {
    let mut kept = take_bursts(older);
    add_bursts(&mut kept, take_bursts(newer));
    add_bursts(newer, kept);
}

pub fn has_bursts(snapshot: &EntitySnapshot) -> bool {
    !(snapshot.eyes.shattered.is_empty()
        && snapshot.breath_bursts.is_empty()
        && snapshot.vibrations.is_empty()
        && snapshot.sculk_souls.is_empty()
        && snapshot.sonic_booms.is_empty()
        && snapshot.dragon_breath.is_empty()
        && snapshot.dragon_explosions.is_empty()
        && snapshot.firework_bursts.is_empty()
        && snapshot.mob_puffs.is_empty())
}

pub fn bursts_message(
    bursts: &EntitySnapshot,
    around: (DVec3, i32),
    has_chunk: impl Fn(ChunkXz) -> bool,
) -> Option<ToClient> {
    let near = near(bursts, around, has_chunk);
    if !has_bursts(&near) {
        return None;
    }
    let entities = minecraft_net::encode(Lane::Reliable, &near).ok()?;
    Some(ToClient::Bursts {
        entities: Blob(entities),
    })
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

    #[test]
    fn a_registered_mobs_show_crosses_the_state_lane() {
        use minecraft_terrain::families::FamilyShow;
        let mut shown = mob(7, 1.5, 0.5);
        shown.show.family = Some(FamilyShow { flags: 0b101, values: [1, -2, 3, 0, 0, 0, 40, i32::MAX] });
        shown.show.fire = true;
        let plain = mob(9, 2.5, 0.5);
        let bytes = minecraft_net::encode(Lane::State, &vec![shown, plain]).unwrap();
        let back: Vec<ServerMob> = minecraft_net::decode(Lane::State, &bytes).unwrap();
        assert_eq!(back.iter().map(|mob| (mob.id, mob.show)).collect::<Vec<_>>(), vec![(7, shown.show), (9, plain.show)]);
        assert_eq!(back[1].show.family, None, "other mobs carry no family show");
        let alone = minecraft_net::encode(Lane::State, &vec![plain]).unwrap();
        let shows = minecraft_net::encode(Lane::State, &vec![shown]).unwrap();
        assert!(shows.len() > alone.len(), "the family show travels with its mob");
    }

    #[test]
    fn ominous_items_reach_friends_eight_chunks_away_with_their_tint_and_age() {
        let mut sender = MobSender::default();
        let ominous = |id: i32, x: f64| minecraft_terrain::trial_spawner::OminousItemView {
            id,
            position: [x, 70.5, 0.5],
            item: "minecraft:lingering_potion".to_owned(),
            tint: Some(0x8889ba),
            age: 42,
        };
        let snapshot = EntitySnapshot {
            ominous_items: vec![ominous(1, 100.5), ominous(2, 140.5)],
            ..EntitySnapshot::default()
        };
        let around = (DVec3::new(0.5, 64.0, 0.5), 12);
        let frame = sender
            .frame(0, Some(&[]), Some(&snapshot), around, |_| true)
            .expect("a frame");
        let near: EntitySnapshot = minecraft_net::decode(Lane::State, &frame.entities.0).unwrap();
        assert_eq!(near.ominous_items, vec![ominous(1, 100.5)]);
    }

    #[test]
    fn effect_swirls_go_only_as_far_as_their_body_is_tracked() {
        let swirl = |x: f64, range: i32| minecraft_terrain::spell_particles::SwirlView {
            feet: DVec3::new(x, 64.0, 0.5),
            width: 0.9,
            height: 1.4,
            colors: vec![0xFF87_A363],
            ambient: false,
            invisible: false,
            range,
        };
        let snapshot = EntitySnapshot {
            swirls: vec![
                swirl(100.5, 10),
                swirl(170.5, 10),
                swirl(140.5, 8),
                swirl(300.5, 32),
                swirl(20.5, 8),
            ],
            ..EntitySnapshot::default()
        };
        let kept = |view_distance: i32, has_chunk: fn(ChunkXz) -> bool| {
            near(
                &snapshot,
                (DVec3::new(0.5, 64.0, 0.5), view_distance),
                has_chunk,
            )
            .swirls
            .iter()
            .map(|swirl| swirl.feet.x)
            .collect::<Vec<_>>()
        };
        assert_eq!(kept(32, |_| true), vec![100.5, 300.5, 20.5]);
        assert_eq!(kept(32, |chunk| chunk.0 < 2), vec![20.5]);
        assert_eq!(kept(16, |_| true), vec![100.5, 20.5]);
        assert_eq!(kept(4, |_| true), vec![20.5]);
    }

    #[test]
    fn a_vibration_burst_reaches_friends_near_it() {
        use minecraft_terrain::vibration_particles::VibrationView;
        let ripple = |x: f64| VibrationView {
            from: DVec3::new(x, 64.0, 0.5),
            to: DVec3::new(0.5, 64.5, 0.5),
            ticks: 5,
        };
        let mut older = EntitySnapshot::default();
        older.vibrations.push(ripple(10.5));
        let mut newer = EntitySnapshot::default();
        newer.vibrations.push(ripple(40.5));
        keep_bursts(&mut older, &mut newer);
        assert!(!has_bursts(&older));
        assert_eq!(newer.vibrations.len(), 2, "kept until the next snapshot is sent");
        let bursts = take_bursts(&mut newer);
        assert!(newer.vibrations.is_empty());
        let here = (DVec3::new(0.5, 64.0, 0.5), 8);
        let Some(ToClient::Bursts { entities }) = bursts_message(&bursts, here, |_| true) else {
            panic!("the ripples go on the reliable stream");
        };
        let sent: EntitySnapshot = minecraft_net::decode(Lane::Reliable, &entities.0).unwrap();
        assert_eq!(sent.vibrations, vec![ripple(10.5)], "a ripple 40 blocks away is out of particle range");
        assert_eq!(bursts_message(&bursts, here, |_| false), None, "nor in a chunk the friend lacks");
    }

    #[test]
    fn a_catalyst_s_souls_reach_friends_near_it() {
        let mut older = EntitySnapshot::default();
        older.sculk_souls.push(DVec3::new(8.5, 65.15, 0.5));
        let mut newer = EntitySnapshot::default();
        newer.sculk_souls.push(DVec3::new(40.5, 65.15, 0.5));
        keep_bursts(&mut older, &mut newer);
        assert!(!has_bursts(&older));
        assert_eq!(newer.sculk_souls.len(), 2, "kept until the next snapshot is sent");
        let bursts = take_bursts(&mut newer);
        assert!(newer.sculk_souls.is_empty());
        let here = (DVec3::new(0.5, 64.0, 0.5), 8);
        let Some(ToClient::Bursts { entities }) = bursts_message(&bursts, here, |_| true) else {
            panic!("the souls go on the reliable stream");
        };
        let sent: EntitySnapshot = minecraft_net::decode(Lane::Reliable, &entities.0).unwrap();
        assert_eq!(sent.sculk_souls, vec![DVec3::new(8.5, 65.15, 0.5)], "40 blocks away is out of particle range");
    }

    #[test]
    fn a_sonic_boom_s_rings_reach_friends_near_them() {
        let mut older = EntitySnapshot::default();
        older.sonic_booms.extend([DVec3::new(2.5, 65.6, 0.5), DVec3::new(31.5, 65.6, 0.5)]);
        let mut newer = EntitySnapshot::default();
        newer.sonic_booms.push(DVec3::new(32.6, 65.6, 0.5));
        keep_bursts(&mut older, &mut newer);
        assert!(!has_bursts(&older));
        assert_eq!(newer.sonic_booms.len(), 3, "kept until the next snapshot is sent");
        let bursts = take_bursts(&mut newer);
        assert!(newer.sonic_booms.is_empty());
        let here = (DVec3::new(0.5, 64.0, 0.5), 8);
        let Some(ToClient::Bursts { entities }) = bursts_message(&bursts, here, |_| true) else {
            panic!("the rings go on the reliable stream");
        };
        let sent: EntitySnapshot = minecraft_net::decode(Lane::Reliable, &entities.0).unwrap();
        assert_eq!(sent.sonic_booms, vec![DVec3::new(2.5, 65.6, 0.5), DVec3::new(31.5, 65.6, 0.5)], "a ring past 32 blocks is out of particle range");
    }

    #[test]
    fn the_dragons_one_shot_effects_leave_the_state_frame_and_are_kept_until_sent() {
        let mut older = EntitySnapshot::default();
        older.breath_bursts.push(DVec3::new(1.0, 60.0, 1.0));
        older.eyes.shattered.push([0, 64, 0]);
        let mut newer = EntitySnapshot::default();
        newer.breath_bursts.push(DVec3::new(2.0, 60.0, 2.0));
        newer.dragon_explosions.push((DVec3::new(0.0, 70.0, 0.0), true));
        keep_bursts(&mut older, &mut newer);
        assert!(!has_bursts(&older));
        assert_eq!(
            newer.breath_bursts,
            vec![DVec3::new(1.0, 60.0, 1.0), DVec3::new(2.0, 60.0, 2.0)],
            "an earlier tick's bursts are not lost to a later snapshot"
        );
        let bursts = take_bursts(&mut newer);
        assert!(!has_bursts(&newer), "the droppable state frame carries none");
        let here = (DVec3::new(0.0, 64.0, 0.0), 8);
        let Some(ToClient::Bursts { entities }) = bursts_message(&bursts, here, |_| true) else {
            panic!("the bursts go on the reliable stream");
        };
        let sent: EntitySnapshot = minecraft_net::decode(Lane::Reliable, &entities.0).unwrap();
        assert_eq!(sent.breath_bursts.len(), 2);
        assert_eq!(sent.dragon_explosions.len(), 1);
        assert_eq!(sent.eyes.shattered, vec![[0, 64, 0]]);
        let far = (DVec3::new(5000.0, 64.0, 0.0), 8);
        let Some(ToClient::Bursts { entities }) = bursts_message(&bursts, far, |_| true) else {
            panic!("the dragon's effects reach the whole End");
        };
        let sent: EntitySnapshot = minecraft_net::decode(Lane::Reliable, &entities.0).unwrap();
        assert!(sent.eyes.shattered.is_empty(), "a far eye is not");
        let mut eye = EntitySnapshot::default();
        eye.eyes.shattered.push([0, 64, 0]);
        assert_eq!(bursts_message(&eye, far, |_| true), None);
        assert_eq!(bursts_message(&eye, here, |_| false), None, "not before its chunk");
    }

    #[test]
    fn mob_puffs_go_on_the_reliable_stream_and_stay_with_their_range() {
        use minecraft_terrain::mob_particles::{MobPuff, PuffKind};
        let puff = |x: f64, range: i32| MobPuff {
            kind: PuffKind::Heart,
            count: 7,
            feet: DVec3::new(x, 64.0, 0.5),
            width: 0.6,
            height: 0.85,
            lift: 0.5,
            range,
        };
        let mut older = EntitySnapshot {
            mob_puffs: vec![puff(0.5, 10)],
            ..EntitySnapshot::default()
        };
        let mut newer = EntitySnapshot {
            mob_puffs: vec![puff(140.5, 8), puff(150.5, 10)],
            ..EntitySnapshot::default()
        };
        keep_bursts(&mut older, &mut newer);
        assert!(!has_bursts(&older));
        assert_eq!(newer.mob_puffs.len(), 3, "an earlier tick's puffs are kept until sent");
        let bursts = take_bursts(&mut newer);
        assert!(!has_bursts(&newer), "the droppable state frame carries none");
        let here = (DVec3::new(0.5, 64.0, 0.5), 32);
        let Some(ToClient::Bursts { entities }) = bursts_message(&bursts, here, |_| true) else {
            panic!("the puffs go on the reliable stream");
        };
        let sent: EntitySnapshot = minecraft_net::decode(Lane::Reliable, &entities.0).unwrap();
        assert_eq!(
            sent.mob_puffs,
            vec![puff(0.5, 10), puff(150.5, 10)],
            "a cat's puff goes 8 chunks, a wolf's 10"
        );
        let far = (DVec3::new(1000.5, 64.0, 0.5), 32);
        assert_eq!(bursts_message(&bursts, far, |_| true), None, "none out of range");
        assert_eq!(bursts_message(&bursts, here, |_| false), None, "not before its chunk");
    }
}
