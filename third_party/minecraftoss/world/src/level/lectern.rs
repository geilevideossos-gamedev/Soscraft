use super::burning::LevelSound;
use super::redstone::Kind;
use super::{Level, update};
use minecraftoss_core::block_entity::{lectern_book_has_content, lectern_book_pages};
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::pos::Direction;
use minecraftoss_core::{BlockPos, BlockStateId};

pub const PAGE_TURN_EVENT: i32 = 1043;
pub const BOOK_PUT: &str = "minecraft:item.book.put";
pub const PULSE_TICKS: i32 = 2;
pub const BOOKS: [&str; 2] = ["minecraft:written_book", "minecraft:writable_book"];

pub fn signal(book: Option<&Tag>, page: i32) -> i32 {
    let Some(book) = book else {
        return 0;
    };
    let count = lectern_book_pages(book);
    let progress = if count > 1 {
        page as f32 / (count as f32 - 1.0)
    } else {
        1.0
    };
    (progress * 14.0).floor() as i32 + i32::from(lectern_book_has_content(book))
}

pub fn drop_point(pos: BlockPos, facing: Direction) -> [f64; 3] {
    let (dx, dz) = match facing {
        Direction::North => (0, -1),
        Direction::South => (0, 1),
        Direction::West => (-1, 0),
        Direction::East => (1, 0),
        _ => (0, 0),
    };
    [
        f64::from(pos.x) + 0.5 + f64::from(0.25_f32 * dx as f32),
        f64::from(pos.y + 1),
        f64::from(pos.z) + 0.5 + f64::from(0.25_f32 * dz as f32),
    ]
}

impl Level<'_> {
    pub fn is_lectern(&self, state: BlockStateId) -> bool {
        self.redstone_kind(state) == Some(Kind::Lectern)
    }

    fn lectern_flag(&self, state: BlockStateId, name: &str) -> bool {
        self.registries().blocks.property(state, name) == Some("true")
    }

    fn lectern_facing(&self, state: BlockStateId) -> Direction {
        self.registries()
            .blocks
            .property(state, "facing")
            .and_then(Direction::from_name)
            .unwrap_or(Direction::North)
    }

    pub fn lectern_view(&self, pos: BlockPos) -> Option<(Tag, i32)> {
        if !self.is_lectern(self.block(pos)) {
            return None;
        }
        let entity = self.block_entity(pos)?;
        let book = entity.get("Book")?.clone();
        let page = entity.get("Page").and_then(Tag::as_i64).unwrap_or(0) as i32;
        Some((book, page))
    }

    pub(super) fn lectern_signal(&self, pos: BlockPos) -> i32 {
        if !self.lectern_flag(self.block(pos), "has_book") {
            return 0;
        }
        match self.lectern_view(pos) {
            Some((book, page)) => signal(Some(&book), page),
            None => 0,
        }
    }

    pub fn lectern_place_book(&mut self, pos: BlockPos, book: Tag) -> bool {
        let state = self.block(pos);
        if !self.is_lectern(state) || self.lectern_flag(state, "has_book") {
            return false;
        }
        if self.block_entity(pos).is_none() {
            self.ensure_block_entity(pos, state);
        }
        if self.block_entity(pos).is_none() {
            return false;
        }
        let block = self.block_id(state);
        self.set_block_entity_value(pos, "Book", book);
        self.set_block_entity_value(pos, "Page", Tag::Int(0));
        self.update_neighbour_for_output_signal(pos, block);
        self.reset_book_state(pos, state, true);
        self.sounds.push(LevelSound {
            event: BOOK_PUT,
            pos: [
                f64::from(pos.x) + 0.5,
                f64::from(pos.y) + 0.5,
                f64::from(pos.z) + 0.5,
            ],
            volume: 1.0,
            pitch: 1.0,
        });
        true
    }

    pub fn lectern_set_page(&mut self, pos: BlockPos, page: i32) -> bool {
        let Some((book, current)) = self.lectern_view(pos) else {
            return false;
        };
        let page = page.clamp(0, (lectern_book_pages(&book) - 1).max(0));
        if page == current {
            return false;
        }
        let state = self.block(pos);
        let block = self.block_id(state);
        self.set_block_entity_value(pos, "Page", Tag::Int(page));
        self.update_neighbour_for_output_signal(pos, block);
        let powered = self.with(state, "powered", "true");
        self.set_block(pos, powered, update::ALL, update::LIMIT);
        self.update_neighbors_at(pos.below(), block);
        self.schedule_block_tick(pos, state, PULSE_TICKS);
        self.level_events.push((PAGE_TURN_EVENT, pos, 0));
        true
    }

    pub fn lectern_take_book(&mut self, pos: BlockPos) -> Option<Tag> {
        let (book, _) = self.lectern_view(pos)?;
        let state = self.block(pos);
        if let Some(Tag::Compound(map)) = self.chunks.get_mut(&pos.chunk()).and_then(|chunk| {
            chunk
                .block_entities
                .entities
                .get_mut(&(pos.x, pos.y, pos.z))
        }) {
            map.remove("Book");
            map.remove("Page");
            self.changed_block_entities.insert((pos.x, pos.y, pos.z));
        }
        let block = self.block_id(state);
        self.update_neighbour_for_output_signal(pos, block);
        self.reset_book_state(pos, state, false);
        Some(book)
    }

    fn reset_book_state(&mut self, pos: BlockPos, state: BlockStateId, has_book: bool) {
        let block = self.block_id(state);
        let unpowered = self.with(state, "powered", "false");
        let next = self.with(
            unpowered,
            "has_book",
            if has_book { "true" } else { "false" },
        );
        self.set_block(pos, next, update::ALL, update::LIMIT);
        self.update_neighbors_at(pos.below(), block);
    }

    pub(super) fn pop_lectern_book(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        if !self.is_lectern(state) || !self.lectern_flag(state, "has_book") {
            return false;
        }
        let Some(book) = self
            .block_entity(pos)
            .and_then(|entity| entity.get("Book"))
            .cloned()
        else {
            return false;
        };
        let id = book
            .get("id")
            .and_then(Tag::as_str)
            .unwrap_or("minecraft:air");
        let count = book.get("count").and_then(Tag::as_i64).unwrap_or(1) as i32;
        let mut stack = super::container::Stack::new(id, count);
        stack.components = book
            .get("components")
            .filter(|components| components.as_compound().is_some_and(|map| !map.is_empty()))
            .cloned();
        if !stack.is_empty() {
            let at = drop_point(pos, self.lectern_facing(state));
            self.spawn_at_location(at, stack);
        }
        true
    }
}
