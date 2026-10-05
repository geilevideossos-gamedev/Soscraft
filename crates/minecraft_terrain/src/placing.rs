use minecraftoss_player::{Block, Pos, World};

pub struct Staged<'a, W> {
    world: &'a W,
    writes: Vec<(Pos, Option<Block>)>,
}

impl<'a, W: World> Staged<'a, W> {
    pub fn new(world: &'a W) -> Self {
        Self {
            world,
            writes: Vec::new(),
        }
    }

    pub fn into_writes(self) -> Vec<(Pos, Option<Block>)> {
        let mut out: Vec<(Pos, Option<Block>)> = Vec::new();
        for (pos, block) in self.writes {
            out.retain(|(p, _)| *p != pos);
            out.push((pos, block));
        }
        out
    }

    fn staged(&self, pos: Pos) -> Option<&Option<Block>> {
        self.writes
            .iter()
            .rev()
            .find(|(p, _)| *p == pos)
            .map(|(_, b)| b)
    }
}

impl<W: World> World for Staged<'_, W> {
    fn block(&self, pos: Pos) -> Option<Block> {
        match self.staged(pos) {
            Some(block) => block.clone(),
            None => self.world.block(pos),
        }
    }

    fn set_block(&mut self, pos: Pos, block: Option<Block>) {
        self.writes.push((pos, block));
    }

    fn collision_boxes(&self, pos: Pos) -> Vec<[f64; 6]> {
        match self.staged(pos) {
            Some(block) => block
                .as_ref()
                .map_or_else(Vec::new, minecraftoss_player::authored_collision_boxes),
            None => self.world.collision_boxes(pos),
        }
    }

    fn min_y(&self) -> i32 {
        self.world.min_y()
    }

    fn max_y(&self) -> i32 {
        self.world.max_y()
    }

    fn replaceable(&self, block: &Block) -> bool {
        self.world.replaceable(block)
    }
}

pub fn obstructed(pos: Pos, boxes: &[[f64; 6]], bodies: &[[f64; 6]]) -> bool {
    let at = [f64::from(pos.0), f64::from(pos.1), f64::from(pos.2)];
    boxes.iter().any(|b| {
        let placed: [f64; 6] = std::array::from_fn(|k| at[k % 3] + b[k]);
        bodies
            .iter()
            .any(|body| (0..3).all(|k| placed[k] < body[k + 3] && body[k] < placed[k + 3]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Blocks(HashMap<Pos, Block>);

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.0.get(&pos).cloned()
        }

        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            match block {
                Some(block) => self.0.insert(pos, block),
                None => self.0.remove(&pos),
            };
        }
    }

    const FULL: [f64; 6] = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    const FENCE: [f64; 6] = [0.375, 0.0, 0.375, 0.625, 1.5, 0.625];

    fn hull(feet: [f64; 3], height: f64) -> [f64; 6] {
        let half = 15.0 / 36.0;
        [
            feet[0] - half,
            feet[1],
            feet[2] - half,
            feet[0] + half,
            feet[1] + height,
            feet[2] + half,
        ]
    }

    #[test]
    fn staged_writes_are_read_back_but_leave_the_world_alone() {
        let mut world = Blocks::default();
        world.set_block((0, 0, 0), Some(Block::new("minecraft:stone")));
        let mut staged = Staged::new(&world);
        staged.set_block((0, 1, 0), Some(Block::new("minecraft:dirt")));
        staged.set_block((0, 0, 0), None);
        assert_eq!(
            staged.block((0, 1, 0)).map(|b| b.id).as_deref(),
            Some("minecraft:dirt")
        );
        assert_eq!(staged.block((0, 0, 0)), None);
        assert!(staged.collision_boxes((0, 0, 0)).is_empty());
        let writes = staged.into_writes();
        assert_eq!(writes.len(), 2);
        assert!(world.block((0, 1, 0)).is_none());
        assert!(world.block((0, 0, 0)).is_some());
    }

    #[test]
    fn a_position_written_twice_keeps_its_last_block() {
        let world = Blocks::default();
        let mut staged = Staged::new(&world);
        staged.set_block((1, 2, 3), Some(Block::new("minecraft:dirt")));
        staged.set_block((1, 2, 3), Some(Block::new("minecraft:stone")));
        let writes = staged.into_writes();
        assert_eq!(writes.len(), 1);
        assert_eq!(
            writes[0].1.as_ref().map(|b| b.id.as_str()),
            Some("minecraft:stone")
        );
    }

    #[test]
    fn a_block_under_a_jumping_player_is_free_once_the_feet_clear_it() {
        let height = 70.0 / 36.0;
        assert!(obstructed(
            (0, 0, 0),
            &[FULL],
            &[hull([0.5, 0.95, 0.5], height)]
        ));
        assert!(!obstructed(
            (0, 0, 0),
            &[FULL],
            &[hull([0.5, 1.0, 0.5], height)]
        ));
        assert!(!obstructed(
            (0, 0, 0),
            &[FULL],
            &[hull([0.5, 1.01, 0.5], height)]
        ));
    }

    #[test]
    fn the_whole_mw2_hull_counts_not_just_a_vanilla_player() {
        let height = 70.0 / 36.0;
        let beside = hull([1.4, 0.0, 0.5], height);
        assert!(obstructed((0, 0, 0), &[FULL], &[beside]));
        let overhead = hull([0.5, -1.9, 0.5], height);
        assert!(obstructed((0, 0, 0), &[FULL], &[overhead]));
    }

    #[test]
    fn a_fence_reaches_into_the_block_above_it() {
        let height = 70.0 / 36.0;
        assert!(obstructed(
            (0, 0, 0),
            &[FENCE],
            &[hull([0.5, 1.2, 0.5], height)]
        ));
        assert!(!obstructed(
            (0, 0, 0),
            &[FENCE],
            &[hull([0.5, 1.5, 0.5], height)]
        ));
    }

    #[test]
    fn any_body_in_the_way_blocks_and_blocks_without_collision_never_do() {
        let height = 70.0 / 36.0;
        let far = hull([10.5, 0.0, 10.5], height);
        let near = hull([0.5, 0.5, 0.5], height);
        assert!(obstructed((0, 0, 0), &[FULL], &[far, near]));
        assert!(!obstructed((0, 0, 0), &[], &[near]));
    }
}
