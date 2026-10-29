use std::collections::{BTreeMap, BTreeSet};

use crate::containers::{Station, chest_partner, chest_sound};
use crate::lids::SET_OPEN_COUNT;
use crate::scene::{Block, BlockPos};

pub const RECHECK_TICKS: u64 = 5;
pub const SOUND_VOLUME: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Opened,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenerEvent {
    pub pos: BlockPos,
    pub count: u32,
    pub change: Option<Change>,
}

impl OpenerEvent {
    pub fn block_event(&self) -> (BlockPos, u8, u32) {
        (self.pos, SET_OPEN_COUNT, self.count)
    }

    pub fn game_event(&self) -> Option<&'static str> {
        self.change.map(|change| match change {
            Change::Opened => crate::game_events::CONTAINER_OPEN,
            Change::Closed => crate::game_events::CONTAINER_CLOSE,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpenerSound {
    pub event: &'static str,
    pub position: [f64; 3],
    pub volume: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Openers {
    cells: BTreeMap<BlockPos, BTreeSet<u128>>,
}

impl Openers {
    pub fn counted(block: &Block) -> bool {
        matches!(
            Station::of(block),
            Some(Station::Chest | Station::Ender | Station::Shulker | Station::Barrel)
        )
    }

    pub fn count(&self, pos: BlockPos) -> u32 {
        self.cells
            .get(&pos)
            .map_or(0, |players| players.len() as u32)
    }

    pub fn open(&mut self, pos: BlockPos, player: u128, spectator: bool) -> Option<OpenerEvent> {
        if spectator {
            return None;
        }
        let players = self.cells.entry(pos).or_default();
        if !players.insert(player) {
            return None;
        }
        let count = players.len() as u32;
        let change = (count == 1).then_some(Change::Opened);
        Some(OpenerEvent { pos, count, change })
    }

    pub fn close(&mut self, pos: BlockPos, player: u128) -> Option<OpenerEvent> {
        let players = self.cells.get_mut(&pos)?;
        if !players.remove(&player) {
            return None;
        }
        let count = players.len() as u32;
        if count == 0 {
            self.cells.remove(&pos);
        }
        let change = (count == 0).then_some(Change::Closed);
        Some(OpenerEvent { pos, count, change })
    }

    pub fn recheck(&mut self, has_open: impl Fn(u128, BlockPos) -> bool) -> Vec<OpenerEvent> {
        let mut events = Vec::new();
        self.cells.retain(|&pos, players| {
            let before = players.len();
            players.retain(|&player| has_open(player, pos));
            if players.len() != before {
                let count = players.len() as u32;
                let change = (count == 0).then_some(Change::Closed);
                events.push(OpenerEvent { pos, count, change });
            }
            !players.is_empty()
        });
        events
    }

    pub fn leave(&mut self, player: u128) -> Vec<OpenerEvent> {
        self.recheck(|other, _| other != player)
    }

    pub fn removed(&mut self, pos: BlockPos) {
        self.cells.remove(&pos);
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }
}

pub fn sound(block: &Block, pos: BlockPos, change: Change) -> Option<OpenerSound> {
    let open = change == Change::Opened;
    let centre = [
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    ];
    let property = |name: &str| block.properties.get(name).map(String::as_str);
    let (event, position) = match Station::of(block)? {
        Station::Chest => {
            let event = chest_sound(&block.id.path, open);
            match property("type") {
                Some("left") => return None,
                Some("right") => {
                    let (x, _, z) = chest_partner(pos, block)?;
                    let step = [f64::from(x - pos.0), 0.0, f64::from(z - pos.2)];
                    (event, std::array::from_fn(|k| centre[k] + step[k] * 0.5))
                }
                _ => (event, centre),
            }
        }
        Station::Ender if open => ("minecraft:block.ender_chest.open", centre),
        Station::Ender => ("minecraft:block.ender_chest.close", centre),
        Station::Shulker if open => ("minecraft:block.shulker_box.open", centre),
        Station::Shulker => ("minecraft:block.shulker_box.close", centre),
        Station::Barrel => {
            let step = match property("facing").unwrap_or("north") {
                "up" => [0.0, 1.0, 0.0],
                "down" => [0.0, -1.0, 0.0],
                "south" => [0.0, 0.0, 1.0],
                "west" => [-1.0, 0.0, 0.0],
                "east" => [1.0, 0.0, 0.0],
                _ => [0.0, 0.0, -1.0],
            };
            let event = if open {
                "minecraft:block.barrel.open"
            } else {
                "minecraft:block.barrel.close"
            };
            (event, std::array::from_fn(|k| centre[k] + step[k] / 2.0))
        }
        Station::Oven(_) | Station::Brewer => return None,
    };
    Some(OpenerSound {
        event,
        position,
        volume: SOUND_VOLUME,
    })
}

pub fn pitch(random: f32) -> f32 {
    random * 0.1 + 0.9
}

pub fn opened_state(block: &Block, change: Change) -> Option<Block> {
    let barrel =
        Station::of(block) == Some(Station::Barrel) && block.properties.contains_key("open");
    barrel.then(|| {
        let open = if change == Change::Opened {
            "true"
        } else {
            "false"
        };
        block.clone().with("open", open)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEVE: u128 = 0x1234;
    const ALEX: u128 = 0x5678;

    fn event(pos: BlockPos, count: u32, change: Option<Change>) -> Option<OpenerEvent> {
        Some(OpenerEvent { pos, count, change })
    }

    #[test]
    fn the_first_opener_opens_it_and_the_last_one_shuts_it() {
        let pos = (4, 70, -9);
        let mut openers = Openers::default();
        assert_eq!(
            openers.open(pos, STEVE, false),
            event(pos, 1, Some(Change::Opened))
        );
        assert_eq!(openers.open(pos, ALEX, false), event(pos, 2, None));
        assert_eq!(
            openers.open(pos, STEVE, false),
            None,
            "counted once per player"
        );
        assert_eq!(openers.count(pos), 2);
        assert_eq!(openers.close(pos, STEVE), event(pos, 1, None));
        assert_eq!(openers.close(pos, STEVE), None);
        assert_eq!(
            openers.close(pos, ALEX),
            event(pos, 0, Some(Change::Closed))
        );
        assert_eq!(openers.count(pos), 0);
        assert_eq!(
            event(pos, 2, None).unwrap().block_event(),
            (pos, SET_OPEN_COUNT, 2)
        );
    }

    #[test]
    fn opening_a_chest_posts_container_open_once_and_closing_the_last_viewer_posts_close() {
        let pos = (4, 70, -9);
        let mut openers = Openers::default();
        let posted: Vec<Option<&str>> = [
            openers.open(pos, STEVE, false),
            openers.open(pos, ALEX, false),
            openers.close(pos, STEVE),
            openers.close(pos, ALEX),
        ]
        .iter()
        .map(|event| event.and_then(|event| event.game_event()))
        .collect();
        assert_eq!(
            posted,
            vec![
                Some(crate::game_events::CONTAINER_OPEN),
                None,
                None,
                Some(crate::game_events::CONTAINER_CLOSE)
            ]
        );
        openers.open(pos, STEVE, false);
        let left = openers.leave(STEVE);
        assert_eq!(
            left.iter().map(OpenerEvent::game_event).collect::<Vec<_>>(),
            vec![Some(crate::game_events::CONTAINER_CLOSE)],
            "a player who leaves closes it"
        );
    }

    #[test]
    fn spectators_look_inside_without_opening_it() {
        let pos = (0, 64, 0);
        let mut openers = Openers::default();
        assert_eq!(openers.open(pos, STEVE, true), None);
        assert_eq!(openers.count(pos), 0);
        assert_eq!(openers.close(pos, STEVE), None);
    }

    #[test]
    fn a_player_who_leaves_shuts_what_only_it_had_open() {
        let (left, right, other) = ((0, 64, 0), (1, 64, 0), (5, 64, 5));
        let mut openers = Openers::default();
        openers.open(left, STEVE, false);
        openers.open(right, STEVE, false);
        openers.open(right, ALEX, false);
        openers.open(other, ALEX, false);
        let events = openers.leave(STEVE);
        assert_eq!(
            events,
            vec![
                OpenerEvent {
                    pos: left,
                    count: 0,
                    change: Some(Change::Closed)
                },
                OpenerEvent {
                    pos: right,
                    count: 1,
                    change: None
                },
            ]
        );
        assert_eq!((openers.count(left), openers.count(right)), (0, 1));
        assert_eq!(openers.count(other), 1);
    }

    #[test]
    fn a_recheck_forgets_players_who_no_longer_have_it_open() {
        let pos = (0, 64, 0);
        let mut openers = Openers::default();
        openers.open(pos, STEVE, false);
        openers.open(pos, ALEX, false);
        assert_eq!(openers.recheck(|_, _| true), Vec::new());
        let events = openers.recheck(|player, at| player == ALEX && at == pos);
        assert_eq!(events, vec![event(pos, 1, None).unwrap()]);
        let events = openers.recheck(|_, _| false);
        assert_eq!(events, vec![event(pos, 0, Some(Change::Closed)).unwrap()]);
        assert_eq!(RECHECK_TICKS, 5);
    }

    #[test]
    fn a_broken_container_forgets_its_openers_without_a_sound() {
        let pos = (0, 64, 0);
        let mut openers = Openers::default();
        openers.open(pos, STEVE, false);
        openers.removed(pos);
        assert_eq!(openers.close(pos, STEVE), None);
        openers.open(pos, ALEX, false);
        openers.clear();
        assert_eq!(openers.count(pos), 0);
    }

    #[test]
    fn opening_sounds_come_from_where_vanilla_plays_them() {
        let pos = (10, 64, -3);
        let centre = [10.5, 64.5, -2.5];
        let at = |block: Block, change| sound(&block, pos, change).map(|s| (s.event, s.position));
        let shulker = Block::new("minecraft:blue_shulker_box").with("facing", "down");
        assert_eq!(
            at(shulker.clone(), Change::Opened),
            Some(("minecraft:block.shulker_box.open", centre))
        );
        assert_eq!(
            at(shulker, Change::Closed),
            Some(("minecraft:block.shulker_box.close", centre))
        );
        let chest = |side: &str| {
            Block::new("minecraft:chest")
                .with("facing", "north")
                .with("type", side)
        };
        assert_eq!(
            at(chest("single"), Change::Opened),
            Some(("minecraft:block.chest.open", centre))
        );
        assert_eq!(at(chest("left"), Change::Opened), None, "one sound a pair");
        assert_eq!(
            at(chest("right"), Change::Closed),
            Some(("minecraft:block.chest.close", [10.0, 64.5, -2.5]))
        );
        let copper = Block::new("minecraft:waxed_weathered_copper_chest").with("type", "single");
        assert_eq!(
            at(copper, Change::Opened),
            Some(("minecraft:block.copper_chest_weathered.open", centre))
        );
        let ender = Block::new("minecraft:ender_chest").with("facing", "east");
        assert_eq!(
            at(ender, Change::Closed),
            Some(("minecraft:block.ender_chest.close", centre))
        );
        let barrel = Block::new("minecraft:barrel").with("facing", "up");
        assert_eq!(
            at(barrel, Change::Opened),
            Some(("minecraft:block.barrel.open", [10.5, 65.0, -2.5]))
        );
        let barrel = Block::new("minecraft:barrel").with("facing", "west");
        assert_eq!(
            at(barrel, Change::Closed),
            Some(("minecraft:block.barrel.close", [10.0, 64.5, -2.5]))
        );
        assert_eq!(at(Block::new("minecraft:furnace"), Change::Opened), None);
        let volume = sound(&chest("single"), pos, Change::Opened).unwrap().volume;
        assert_eq!(volume, 0.5);
        assert!((pitch(0.0) - 0.9).abs() < 1e-6 && (pitch(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_barrel_shows_open_while_someone_has_it_open() {
        let barrel = Block::new("minecraft:barrel")
            .with("facing", "up")
            .with("open", "false");
        let open = opened_state(&barrel, Change::Opened).unwrap();
        assert_eq!(
            open.properties.get("open").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            open.properties.get("facing").map(String::as_str),
            Some("up")
        );
        let shut = opened_state(&open, Change::Closed).unwrap();
        assert_eq!(
            shut.properties.get("open").map(String::as_str),
            Some("false")
        );
        let chest = Block::new("minecraft:chest");
        assert_eq!(opened_state(&chest, Change::Opened), None);
        assert!(Openers::counted(&barrel) && Openers::counted(&chest));
        assert!(Openers::counted(&Block::new("minecraft:ender_chest")));
        assert!(Openers::counted(&Block::new("minecraft:shulker_box")));
        assert!(!Openers::counted(&Block::new("minecraft:furnace")));
        assert!(!Openers::counted(&Block::new("minecraft:brewing_stand")));
    }
}
