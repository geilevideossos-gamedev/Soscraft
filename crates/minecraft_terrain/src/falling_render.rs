use crate::mesh::{MovingBlock, ShadowCaster};
use crate::scene::{Block, BlockPos, Scene};
use crate::server::{ServerFallingBlock, ServerTnt};
use glam::DVec3;
use std::collections::HashSet;

const BOX_HEIGHT: f64 = 0.98;
const SHADOW_RADIUS: f32 = 0.5;
const SHADOW_STRENGTH: f32 = 1.0;
const LANDING_REACH: i32 = 2;
pub const LANDED_HOLD_TICKS: u64 = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct Landed {
    pub cell: BlockPos,
    pub until: u64,
}

#[derive(Clone, Debug, Default)]
pub struct FallingBlocks {
    pub falling: Vec<ServerFallingBlock>,
    pub primed: Vec<ServerTnt>,
    pub landed: Vec<Landed>,
}

impl FallingBlocks {
    pub fn receive(
        &mut self,
        falling: Vec<ServerFallingBlock>,
        primed: Vec<ServerTnt>,
        changes: &[(BlockPos, Option<Block>)],
        tick: u64,
    ) {
        let alive: HashSet<i32> = falling.iter().map(|f| f.id).collect();
        for gone in self.falling.iter().filter(|f| !alive.contains(&f.id)) {
            if let Some(cell) = landing(gone, changes) {
                self.landed.retain(|held| held.cell != cell);
                self.landed.push(Landed {
                    cell,
                    until: tick + LANDED_HOLD_TICKS,
                });
            }
        }
        self.falling = falling;
        self.primed = primed;
    }

    pub fn tick_sent(&mut self) {
        for falling in &mut self.falling {
            falling.previous_position = falling.position;
        }
        for primed in &mut self.primed {
            primed.previous_position = primed.position;
            primed.fuse = (primed.fuse - 1).max(0);
        }
    }

    pub fn settle<S: Scene>(
        &mut self,
        scene: &S,
        cell_settled: impl Fn(BlockPos) -> bool,
        tick: u64,
    ) {
        self.landed.retain(|held| {
            !cell_settled(held.cell) && tick < held.until && scene.block(held.cell).is_some()
        });
    }

    pub fn moving_blocks<S: Scene>(&self, scene: &S, partial: f32) -> Vec<MovingBlock> {
        let partial = f64::from(partial.clamp(0.0, 1.0));
        let falling = self
            .falling
            .iter()
            .filter(|f| shown(f, scene))
            .map(|f| MovingBlock {
                feet: interpolated(f.previous_position, f.position, partial),
                block: f.block.clone(),
                seed: f.start,
            });
        let landed = self.landed.iter().filter_map(|held| {
            Some(MovingBlock {
                feet: DVec3::new(
                    f64::from(held.cell.0) + 0.5,
                    f64::from(held.cell.1),
                    f64::from(held.cell.2) + 0.5,
                ),
                block: scene.block(held.cell)?.clone(),
                seed: held.cell,
            })
        });
        falling.chain(landed).collect()
    }

    pub fn primed_tnt(&self, partial: f32) -> Vec<(glam::Vec3, f32)> {
        let partial = partial.clamp(0.0, 1.0);
        self.primed
            .iter()
            .map(|t| {
                let at = interpolated(t.previous_position, t.position, f64::from(partial));
                (at.as_vec3(), t.fuse as f32 - partial + 1.0)
            })
            .collect()
    }

    pub fn shadow_casters<S: Scene>(
        &self,
        scene: &S,
        camera: DVec3,
        partial: f32,
    ) -> Vec<ShadowCaster> {
        let partial = f64::from(partial.clamp(0.0, 1.0));
        self.falling
            .iter()
            .filter(|f| shown(f, scene))
            .map(|f| ShadowCaster {
                position: interpolated(f.previous_position, f.position, partial),
                distance_squared: DVec3::from_array(f.position).distance_squared(camera),
                radius: SHADOW_RADIUS,
                strength: SHADOW_STRENGTH,
            })
            .collect()
    }
}

fn interpolated(previous: [f64; 3], position: [f64; 3], partial: f64) -> DVec3 {
    DVec3::from_array(previous).lerp(DVec3::from_array(position), partial)
}

fn shown<S: Scene>(falling: &ServerFallingBlock, scene: &S) -> bool {
    let [x, y, z] = falling.position.map(|c| c.floor() as i32);
    scene.block((x, y, z)) != Some(&falling.block)
}

fn landing(gone: &ServerFallingBlock, changes: &[(BlockPos, Option<Block>)]) -> Option<BlockPos> {
    let [x, feet, z] = gone.position.map(|c| c.floor() as i32);
    let top = (gone.position[1] + BOX_HEIGHT).floor() as i32;
    changes
        .iter()
        .filter_map(|(pos, block)| Some((*pos, block.as_ref()?)))
        .filter(|(pos, _)| {
            pos.0 == x && pos.2 == z && pos.1 <= top && pos.1 >= feet - LANDING_REACH
        })
        .max_by_key(|(pos, block)| (block.id == gone.block.id, pos.1))
        .map(|(pos, _)| pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::ChunkPos;
    use std::collections::HashMap;

    #[derive(Default)]
    struct Blocks(HashMap<BlockPos, Block>);

    impl Scene for Blocks {
        fn block(&self, pos: BlockPos) -> Option<&Block> {
            self.0.get(&pos)
        }
        fn chunks(&self) -> Vec<ChunkPos> {
            Vec::new()
        }
        fn vertical_range(&self) -> std::ops::Range<i32> {
            -64..320
        }
        fn revision(&self) -> u64 {
            0
        }
    }

    fn sand() -> Block {
        Block::new("minecraft:sand")
    }

    fn falling(id: i32, previous: [f64; 3], position: [f64; 3]) -> ServerFallingBlock {
        ServerFallingBlock {
            id,
            position,
            previous_position: previous,
            block: sand(),
            start: (4, 70, 4),
        }
    }

    fn tnt(previous: [f64; 3], position: [f64; 3]) -> ServerTnt {
        ServerTnt {
            position,
            previous_position: previous,
            fuse: 40,
        }
    }

    #[test]
    fn falling_blocks_are_kept_between_snapshots_and_hold_still_until_the_next() {
        let mut view = FallingBlocks::default();
        let air = Blocks::default();
        view.receive(
            vec![falling(7, [4.5, 70.0, 4.5], [4.5, 69.96, 4.5])],
            vec![tnt([1.5, 64.0, 1.5], [1.5, 63.9, 1.5])],
            &[],
            1,
        );
        let drawn = view.moving_blocks(&air, 0.5);
        assert_eq!(drawn.len(), 1);
        assert!(
            (drawn[0].feet.y - 69.98).abs() < 1e-9,
            "{:?}",
            drawn[0].feet
        );
        assert_eq!(drawn[0].seed, (4, 70, 4));
        let fuse_before = view.primed_tnt(1.0)[0].1;
        view.tick_sent();
        let fuse_after = view.primed_tnt(0.0)[0].1;
        assert!(
            (fuse_after - fuse_before).abs() < 1e-6,
            "{fuse_before} {fuse_after}"
        );
        for partial in [0.0, 0.3, 1.0] {
            let drawn = view.moving_blocks(&air, partial);
            assert_eq!(drawn.len(), 1, "drawn until the next snapshot");
            assert!(
                (drawn[0].feet.y - 69.96).abs() < 1e-9,
                "{:?}",
                drawn[0].feet
            );
            let primed = view.primed_tnt(partial);
            assert_eq!(primed.len(), 1);
            assert!((primed[0].0.y - 63.9).abs() < 1e-5, "{primed:?}");
        }
        view.receive(
            vec![falling(7, [4.5, 69.96, 4.5], [4.5, 69.88, 4.5])],
            Vec::new(),
            &[],
            2,
        );
        assert!((view.moving_blocks(&air, 0.0)[0].feet.y - 69.96).abs() < 1e-9);
        assert!(view.primed_tnt(0.0).is_empty());
    }

    #[test]
    fn a_landed_block_is_drawn_in_its_cell_until_the_terrain_shows_it() {
        let mut view = FallingBlocks::default();
        view.receive(
            vec![falling(7, [4.5, 66.2, 4.5], [4.5, 65.4, 4.5])],
            Vec::new(),
            &[],
            1,
        );
        let mut scene = Blocks::default();
        scene.0.insert((4, 65, 4), sand());
        let changes = [
            ((4, 63, 4), Some(Block::new("minecraft:stone"))),
            ((4, 65, 4), Some(sand())),
            ((9, 65, 9), Some(sand())),
        ];
        view.receive(Vec::new(), Vec::new(), &changes, 2);
        assert_eq!(
            view.landed,
            vec![Landed {
                cell: (4, 65, 4),
                until: 2 + LANDED_HOLD_TICKS
            }]
        );
        view.settle(&scene, |_| false, 3);
        let drawn = view.moving_blocks(&scene, 0.4);
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].feet, DVec3::new(4.5, 65.0, 4.5));
        assert_eq!(
            (drawn[0].block.clone(), drawn[0].seed),
            (sand(), (4, 65, 4)),
            "the placed block as the terrain will show it"
        );
        view.settle(&scene, |cell| cell != (4, 65, 4), 3);
        assert_eq!(view.landed.len(), 1, "only its own cell matters");
        view.settle(&scene, |cell| cell == (4, 65, 4), 3);
        assert!(
            view.moving_blocks(&scene, 0.4).is_empty(),
            "the terrain shows it now"
        );
        view.landed.push(Landed {
            cell: (4, 65, 4),
            until: 5,
        });
        view.settle(&scene, |_| false, 5);
        assert!(view.landed.is_empty(), "held at most a few ticks");
        view.landed.push(Landed {
            cell: (4, 65, 4),
            until: 5,
        });
        view.settle(&Blocks::default(), |_| false, 3);
        assert!(view.landed.is_empty(), "the block is gone again");
    }

    #[test]
    fn a_block_that_turns_into_another_on_landing_is_still_held() {
        let mut view = FallingBlocks::default();
        view.receive(
            vec![falling(7, [4.5, 66.2, 4.5], [4.5, 65.4, 4.5])],
            Vec::new(),
            &[],
            1,
        );
        let concrete = Block::new("minecraft:white_concrete");
        let changes = [((4, 65, 4), Some(concrete.clone()))];
        view.receive(Vec::new(), Vec::new(), &changes, 2);
        assert_eq!(view.landed.len(), 1);
        assert_eq!(view.landed[0].cell, (4, 65, 4));
        let mut scene = Blocks::default();
        scene.0.insert((4, 65, 4), concrete.clone());
        let drawn = view.moving_blocks(&scene, 0.5);
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].block, concrete);
    }

    #[test]
    fn a_falling_block_that_breaks_on_landing_leaves_nothing_behind() {
        let mut view = FallingBlocks::default();
        view.receive(
            vec![falling(7, [4.5, 66.2, 4.5], [4.5, 65.4, 4.5])],
            Vec::new(),
            &[],
            1,
        );
        view.receive(Vec::new(), Vec::new(), &[((4, 65, 4), None)], 2);
        assert!(view.landed.is_empty());
        assert!(view.moving_blocks(&Blocks::default(), 0.5).is_empty());
    }

    #[test]
    fn a_falling_block_over_its_own_placed_state_is_not_drawn() {
        let mut view = FallingBlocks::default();
        view.receive(
            vec![falling(7, [4.5, 70.0, 4.5], [4.5, 70.0, 4.5])],
            Vec::new(),
            &[],
            1,
        );
        let mut scene = Blocks::default();
        scene.0.insert((4, 70, 4), sand());
        assert!(view.moving_blocks(&scene, 0.5).is_empty());
        assert!(view.shadow_casters(&scene, DVec3::ZERO, 0.5).is_empty());
        let casters = view.shadow_casters(&Blocks::default(), DVec3::ZERO, 0.5);
        assert_eq!(casters.len(), 1);
        assert_eq!((casters[0].radius, casters[0].strength), (0.5, 1.0));
    }
}
