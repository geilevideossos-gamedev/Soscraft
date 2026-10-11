use std::collections::BTreeMap;

use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{Chunk, ChunkPos};
use minecraftoss_world::level::Level;

pub type Pos = (i32, i32, i32);

pub const END_GATEWAY: &str = "minecraft:end_gateway";
pub const SPAWN_TIME: i64 = 200;
pub const COOLDOWN_TIME: i32 = 40;
pub const ATTENTION_INTERVAL: i64 = 2400;
pub const PEARL_SIZE: f64 = 0.25;
const SPAWNABLE_HEIGHT: i32 = 20_000_000;
const WORLD_BORDER: i32 = 30_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gateway {
    pub age: i64,
    pub cooldown: i32,
    pub exit: Option<Pos>,
    pub exact: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GatewayView {
    pub pos: Pos,
    pub age: i64,
    pub cooldown: i32,
    pub exit: Option<Pos>,
    pub exact: bool,
}

impl GatewayView {
    pub fn is_spawning(&self) -> bool {
        self.age < SPAWN_TIME
    }

    pub fn is_cooling_down(&self) -> bool {
        self.cooldown > 0
    }
}

fn in_spawnable_bounds((x, y, z): Pos) -> bool {
    (-SPAWNABLE_HEIGHT..SPAWNABLE_HEIGHT).contains(&y)
        && (-WORLD_BORDER..WORLD_BORDER).contains(&x)
        && (-WORLD_BORDER..WORLD_BORDER).contains(&z)
}

impl Gateway {
    pub fn from_tag(tag: &Tag) -> Option<Self> {
        if tag.get("id").and_then(Tag::as_str) != Some(END_GATEWAY) {
            return None;
        }
        let exit = tag
            .get("exit_portal")
            .and_then(Tag::as_ints)
            .filter(|exit| exit.len() == 3)
            .map(|exit| (exit[0], exit[1], exit[2]))
            .filter(|&exit| in_spawnable_bounds(exit));
        Some(Self {
            age: tag.get("Age").and_then(Tag::as_i64).unwrap_or(0),
            cooldown: 0,
            exit,
            exact: tag
                .get("ExactTeleport")
                .and_then(Tag::as_i64)
                .is_some_and(|exact| exact != 0),
        })
    }

    pub fn is_spawning(&self) -> bool {
        self.age < SPAWN_TIME
    }

    pub fn is_cooling_down(&self) -> bool {
        self.cooldown > 0
    }

    fn tick(&mut self) -> bool {
        let (spawning, cooling) = (self.is_spawning(), self.is_cooling_down());
        self.age += 1;
        let mut changed = false;
        if cooling {
            self.cooldown -= 1;
        } else if self.age % ATTENTION_INTERVAL == 0 {
            self.cooldown = COOLDOWN_TIME;
            changed = true;
        }
        changed || spawning != self.is_spawning() || cooling != self.is_cooling_down()
    }
}

fn chunk_of((x, _, z): Pos) -> (i32, i32) {
    (x >> 4, z >> 4)
}

#[derive(Clone, Debug, Default)]
pub struct Gateways {
    table: BTreeMap<Pos, Gateway>,
}

impl Gateways {
    pub fn get(&self, pos: Pos) -> Option<&Gateway> {
        self.table.get(&pos)
    }

    pub fn sync(&mut self, pos: Pos, tag: Option<&Tag>) {
        match tag.and_then(Gateway::from_tag) {
            Some(fresh) => {
                let known = self.table.entry(pos).or_insert(fresh);
                known.exit = fresh.exit;
                known.exact = fresh.exact;
            }
            None => {
                self.table.remove(&pos);
            }
        }
    }

    pub fn load_chunk(&mut self, chunk: &Chunk) {
        let mut tags: BTreeMap<Pos, &Tag> = chunk
            .block_entities
            .pending
            .iter()
            .map(|(&pos, tag)| (pos, tag))
            .collect();
        tags.extend(
            chunk
                .block_entities
                .entities
                .iter()
                .map(|(&pos, tag)| (pos, tag)),
        );
        let here = (chunk.pos.x, chunk.pos.z);
        self.table.retain(|&pos, _| {
            chunk_of(pos) != here
                || tags
                    .get(&pos)
                    .and_then(|tag| Gateway::from_tag(tag))
                    .is_some()
        });
        for (pos, tag) in tags {
            if chunk_of(pos) == here {
                self.sync(pos, Some(tag));
            }
        }
    }

    pub fn unload_chunk(&mut self, chunk: ChunkPos) {
        self.table
            .retain(|&pos, _| chunk_of(pos) != (chunk.x, chunk.z));
    }

    pub fn tick(&mut self) -> Vec<(Pos, i64)> {
        let mut changed = Vec::new();
        for (&pos, gateway) in &mut self.table {
            if gateway.tick() {
                changed.push((pos, gateway.age));
            }
        }
        changed
    }

    pub fn trigger_cooldown(&mut self, pos: Pos) -> Option<i64> {
        let gateway = self
            .table
            .get_mut(&pos)
            .filter(|gateway| !gateway.is_cooling_down())?;
        gateway.cooldown = COOLDOWN_TIME;
        Some(gateway.age)
    }

    pub fn set_exit(&mut self, pos: Pos, exit: Pos, exact: bool) -> Option<i64> {
        let gateway = self.table.get_mut(&pos)?;
        gateway.exit = Some(exit);
        gateway.exact = exact;
        Some(gateway.age)
    }

    pub fn views(&self) -> Vec<GatewayView> {
        self.table
            .iter()
            .map(|(&pos, gateway)| GatewayView {
                pos,
                age: gateway.age,
                cooldown: gateway.cooldown,
                exit: gateway.exit,
                exact: gateway.exact,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ClientGateways {
    views: Vec<GatewayView>,
    marks: Vec<(Pos, u64)>,
}

impl ClientGateways {
    pub fn receive(&mut self, views: Vec<GatewayView>, ticks: u64) {
        self.views = views;
        let views = &self.views;
        self.marks.retain(|&(pos, since)| {
            ticks < since + COOLDOWN_TIME as u64
                && views
                    .iter()
                    .any(|view| view.pos == pos && !view.is_cooling_down())
        });
    }

    pub fn views(&self) -> &[GatewayView] {
        &self.views
    }

    pub fn get(&self, pos: Pos) -> Option<&GatewayView> {
        self.views.iter().find(|view| view.pos == pos)
    }

    pub fn mark_cooling(&mut self, pos: Pos, ticks: u64) {
        self.marks.push((pos, ticks));
    }

    pub fn is_cooling(&self, pos: Pos, ticks: u64) -> bool {
        self.get(pos).is_some_and(GatewayView::is_cooling_down)
            || self
                .marks
                .iter()
                .any(|&(at, since)| at == pos && ticks < since + COOLDOWN_TIME as u64)
    }
}

pub fn pearl_gateway(
    from: [f64; 3],
    to: [f64; 3],
    mut enter: impl FnMut(Pos) -> bool,
) -> Option<Pos> {
    let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    let steps = (length / (PEARL_SIZE / 2.0)).ceil().max(1.0) as u32;
    let half = PEARL_SIZE / 2.0 - 1.0e-5;
    let mut seen = Vec::new();
    for step in 0..=steps {
        let t = f64::from(step) / f64::from(steps);
        let at = [
            from[0] + delta[0] * t,
            from[1] + delta[1] * t,
            from[2] + delta[2] * t,
        ];
        let low = [at[0] - half, at[1] + 1.0e-5, at[2] - half];
        let high = [at[0] + half, at[1] + PEARL_SIZE - 1.0e-5, at[2] + half];
        let range = |axis: usize| low[axis].floor() as i32..=high[axis].floor() as i32;
        for z in range(2) {
            for y in range(1) {
                for x in range(0) {
                    if seen.contains(&(x, y, z)) {
                        continue;
                    }
                    seen.push((x, y, z));
                    if enter((x, y, z)) {
                        return Some((x, y, z));
                    }
                }
            }
        }
    }
    None
}

pub fn tag_at<'l>(level: &'l Level<'_>, pos: Pos) -> Option<&'l Tag> {
    level
        .chunk(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2).chunk())?
        .block_entities
        .entities
        .get(&pos)
}

pub fn write_age(level: &mut Level<'_>, pos: Pos, age: i64) {
    level.set_block_entity_value(
        minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2),
        "Age",
        Tag::Long(age),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pearl_sweeps_its_small_box_along_its_flight() {
        let gateway = |pos: Pos| pos == (0, 64, 0);
        assert_eq!(
            pearl_gateway([0.5, 66.0, -3.0], [0.5, 64.2, 3.0], gateway),
            Some((0, 64, 0))
        );
        assert_eq!(
            pearl_gateway([0.5, 65.0, 0.5], [0.5, 65.0, 0.5], gateway),
            None
        );
        assert_eq!(
            pearl_gateway([0.5, 64.76, 0.5], [0.5, 64.76, 0.5], gateway),
            Some((0, 64, 0)),
            "the box is a quarter high"
        );
        assert_eq!(
            pearl_gateway([1.13, 64.5, 0.5], [1.13, 64.5, 0.5], gateway),
            None,
            "and a quarter wide"
        );
        let mut asked = Vec::new();
        let first = pearl_gateway([0.5, 64.5, 5.5], [0.5, 64.5, -2.5], |pos| {
            asked.push(pos);
            pos == (0, 64, 3) || pos == (0, 64, 0)
        });
        assert_eq!(first, Some((0, 64, 3)), "the first gateway on the way");
        assert_eq!(asked, vec![(0, 64, 5), (0, 64, 4), (0, 64, 3)]);
    }

    fn tag(entries: &[(&str, Tag)]) -> Tag {
        Tag::Compound(
            entries
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
        )
    }

    fn gateway_tag(age: i64) -> Tag {
        tag(&[
            ("id", Tag::String(END_GATEWAY.to_owned())),
            ("Age", Tag::Long(age)),
        ])
    }

    fn one(age: i64) -> (Gateways, Pos) {
        let pos = (96, 75, 0);
        let mut gateways = Gateways::default();
        gateways.sync(pos, Some(&gateway_tag(age)));
        (gateways, pos)
    }

    #[test]
    fn a_gateway_spawns_for_200_ticks_and_flashes_every_2400() {
        let (mut gateways, pos) = one(0);
        assert!(gateways.get(pos).unwrap().is_spawning());
        let mut reports = Vec::new();
        for _ in 0..5000 {
            reports.extend(gateways.tick());
        }
        assert_eq!(
            reports,
            [
                (pos, 200),
                (pos, 2400),
                (pos, 2440),
                (pos, 4800),
                (pos, 4840)
            ]
        );
        let gateway = gateways.get(pos).unwrap();
        assert_eq!((gateway.age, gateway.cooldown), (5000, 0));
        assert!(!gateway.is_spawning());
        let (mut flashing, pos) = one(2399);
        assert_eq!(flashing.tick(), [(pos, 2400)]);
        assert_eq!(flashing.get(pos).unwrap().cooldown, COOLDOWN_TIME);
        assert!(flashing.views()[0].is_cooling_down());
        assert!(flashing.tick().is_empty());
        assert_eq!(flashing.get(pos).unwrap().cooldown, COOLDOWN_TIME - 1);
    }

    #[test]
    fn a_triggered_gateway_cools_for_40_ticks_and_ignores_a_second_trigger() {
        let (mut gateways, pos) = one(500);
        assert_eq!(gateways.trigger_cooldown(pos), Some(500));
        assert_eq!(gateways.trigger_cooldown(pos), None);
        assert_eq!(gateways.trigger_cooldown((0, 0, 0)), None);
        for tick in 1..=COOLDOWN_TIME {
            assert!(gateways.get(pos).unwrap().is_cooling_down());
            let reports = gateways.tick();
            assert_eq!(reports.is_empty(), tick < COOLDOWN_TIME, "tick {tick}");
        }
        assert!(!gateways.get(pos).unwrap().is_cooling_down());
        assert_eq!(gateways.trigger_cooldown(pos), Some(540));
    }

    #[test]
    fn exits_are_read_with_vanilla_keys_and_bounds() {
        let read = |entries: &[(&str, Tag)]| {
            let mut all = vec![("id", Tag::String(END_GATEWAY.to_owned()))];
            all.extend(entries.iter().cloned());
            Gateway::from_tag(&tag(&all)).unwrap()
        };
        assert_eq!(read(&[]), Gateway::default());
        let gateway = read(&[
            ("Age", Tag::Long(150)),
            ("exit_portal", Tag::IntArray(vec![100, 50, 0])),
            ("ExactTeleport", Tag::Byte(1)),
        ]);
        assert_eq!(
            gateway,
            Gateway {
                age: 150,
                cooldown: 0,
                exit: Some((100, 50, 0)),
                exact: true
            }
        );
        assert_eq!(
            read(&[("exit_portal", Tag::IntArray(vec![1, 2]))]).exit,
            None
        );
        assert_eq!(
            read(&[("exit_portal", Tag::IntArray(vec![30_000_000, 50, 0]))]).exit,
            None
        );
        assert_eq!(
            read(&[(
                "exit_portal",
                Tag::IntArray(vec![29_999_999, -19_999_999, -30_000_000])
            )])
            .exit,
            Some((29_999_999, -19_999_999, -30_000_000))
        );
        assert_eq!(
            read(&[("exit_portal", Tag::IntArray(vec![0, 20_000_000, 0]))]).exit,
            None
        );
        assert_eq!(
            Gateway::from_tag(&tag(&[(
                "id",
                Tag::String("minecraft:end_portal".to_owned())
            )])),
            None
        );
    }

    #[test]
    fn a_synced_gateway_keeps_its_age_and_cooldown_and_takes_the_new_exit() {
        let (mut gateways, pos) = one(300);
        gateways.trigger_cooldown(pos);
        let mut moved = gateway_tag(0);
        if let Tag::Compound(map) = &mut moved {
            map.insert("exit_portal".to_owned(), Tag::IntArray(vec![1, 2, 3]));
        }
        gateways.sync(pos, Some(&moved));
        assert_eq!(
            gateways.get(pos),
            Some(&Gateway {
                age: 300,
                cooldown: COOLDOWN_TIME,
                exit: Some((1, 2, 3)),
                exact: false
            })
        );
        assert_eq!(gateways.set_exit(pos, (4, 5, 6), true), Some(300));
        assert_eq!(
            gateways.views(),
            [GatewayView {
                pos,
                age: 300,
                cooldown: COOLDOWN_TIME,
                exit: Some((4, 5, 6)),
                exact: true
            }]
        );
        assert_eq!(gateways.set_exit((0, 0, 0), (4, 5, 6), true), None);
        gateways.sync(pos, None);
        assert!(gateways.views().is_empty());
    }

    #[test]
    fn a_marked_gateway_counts_as_cooling_until_the_server_shows_it() {
        let pos = (96, 75, 0);
        let open = GatewayView {
            pos,
            age: 900,
            cooldown: 0,
            exit: None,
            exact: false,
        };
        let mut client = ClientGateways::default();
        client.receive(vec![open], 10);
        assert_eq!(client.get(pos), Some(&open));
        assert!(!client.is_cooling(pos, 10));
        client.mark_cooling(pos, 10);
        assert!(client.is_cooling(pos, 10));
        client.receive(vec![open], 11);
        assert!(
            client.is_cooling(pos, 11),
            "a snapshot from before the trigger keeps the mark"
        );
        client.receive(
            vec![GatewayView {
                cooldown: COOLDOWN_TIME,
                ..open
            }],
            12,
        );
        assert!(client.is_cooling(pos, 12));
        client.receive(
            vec![GatewayView {
                cooldown: 0,
                ..open
            }],
            13,
        );
        assert!(
            !client.is_cooling(pos, 13),
            "the server's cooldown replaced the mark"
        );
        client.mark_cooling(pos, 20);
        assert!(client.is_cooling(pos, 59));
        assert!(
            !client.is_cooling(pos, 60),
            "a mark the server never confirms runs out"
        );
        client.receive(vec![open], 60);
        assert!(!client.is_cooling(pos, 60));
        client.mark_cooling(pos, 70);
        client.receive(Vec::new(), 71);
        assert!(
            !client.is_cooling(pos, 71),
            "a gateway that is gone takes its mark with it"
        );
        assert!(client.views().is_empty());
    }
}
