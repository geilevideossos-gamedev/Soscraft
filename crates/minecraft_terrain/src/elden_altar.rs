use crate::scene::{Block, BlockPos};
use elden_boss::shape::yaw_to;
use glam::DVec3;

pub const BOSS: &str = elden_boss::margit::ID;
pub const LANTERN: &str = "minecraft:lantern";
pub const GOLD: &str = "minecraft:gold_block";
pub const HANGING: &str = "hanging";
pub const PILLAR: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Altar {
    pub lantern: BlockPos,
}

impl Altar {
    pub fn blocks(&self) -> [BlockPos; 3] {
        let (x, y, z) = self.lantern;
        [(x, y, z), (x, y - 1, z), (x, y - PILLAR, z)]
    }

    pub fn feet(&self) -> DVec3 {
        let (x, y, z) = self.lantern;
        DVec3::new(
            f64::from(x) + 0.5,
            f64::from(y - PILLAR),
            f64::from(z) + 0.5,
        )
    }
}

pub fn standing_lantern(block: &Block) -> bool {
    block.id.key() == LANTERN && block.properties.get(HANGING).map(String::as_str) != Some("true")
}

fn gold(block: &Block) -> bool {
    block.id.key() == GOLD
}

pub fn find<'a>(
    placed: BlockPos,
    block_at: impl Fn(BlockPos) -> Option<&'a Block>,
) -> Option<Altar> {
    let (x, y, z) = placed;
    let lantern = block_at(placed).is_some_and(standing_lantern);
    let pillar = (1..=PILLAR).all(|below| block_at((x, y - below, z)).is_some_and(gold));
    (lantern && pillar).then_some(Altar { lantern: placed })
}

pub fn facing(feet: DVec3, players: &[DVec3]) -> f32 {
    players
        .iter()
        .min_by(|a, b| {
            a.distance_squared(feet)
                .total_cmp(&b.distance_squared(feet))
        })
        .and_then(|player| yaw_to(feet, *player))
        .map_or(0.0, |yaw| yaw as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn standing() -> Block {
        Block::new(LANTERN).with(HANGING, "false")
    }

    fn pillar(top: Block) -> HashMap<BlockPos, Block> {
        HashMap::from([
            ((4, 65, 4), top),
            ((4, 64, 4), Block::new(GOLD)),
            ((4, 63, 4), Block::new(GOLD)),
        ])
    }

    fn found(blocks: &HashMap<BlockPos, Block>, placed: BlockPos) -> Option<Altar> {
        find(placed, |pos| blocks.get(&pos))
    }

    #[test]
    fn a_standing_lantern_on_two_gold_blocks_is_an_altar() {
        let blocks = pillar(standing());
        let altar = found(&blocks, (4, 65, 4)).expect("an altar");
        assert_eq!(altar.blocks(), [(4, 65, 4), (4, 64, 4), (4, 63, 4)]);
        assert_eq!(
            altar.feet(),
            DVec3::new(4.5, 63.0, 4.5),
            "the pillar's base"
        );
        let plain = pillar(Block::new(LANTERN));
        assert_eq!(
            found(&plain, (4, 65, 4)),
            Some(altar),
            "a lantern's default state stands"
        );
        let mut taller = pillar(standing());
        taller.insert((4, 62, 4), Block::new(GOLD));
        assert_eq!(
            found(&taller, (4, 65, 4)),
            Some(altar),
            "the top two of a taller pillar"
        );
        let mut low = pillar(standing());
        low.insert((4, 66, 4), Block::new("minecraft:stone"));
        assert_eq!(found(&low, (4, 65, 4)), Some(altar), "under a roof too");
    }

    #[test]
    fn a_hanging_lantern_or_one_gold_block_is_not() {
        let hanging = pillar(Block::new(LANTERN).with(HANGING, "true"));
        assert_eq!(found(&hanging, (4, 65, 4)), None, "a hanging lantern");
        let mut one = pillar(standing());
        one.insert((4, 63, 4), Block::new("minecraft:stone"));
        assert_eq!(found(&one, (4, 65, 4)), None, "one gold block");
        one.remove(&(4, 63, 4));
        assert_eq!(found(&one, (4, 65, 4)), None, "one gold block over air");
        let soul = pillar(Block::new("minecraft:soul_lantern").with(HANGING, "false"));
        assert_eq!(found(&soul, (4, 65, 4)), None, "a soul lantern");
        let mut iron = pillar(standing());
        for y in [63, 64] {
            iron.insert((4, y, 4), Block::new("minecraft:iron_block"));
        }
        assert_eq!(found(&iron, (4, 65, 4)), None, "iron blocks");
        let blocks = pillar(standing());
        assert_eq!(
            found(&blocks, (4, 64, 4)),
            None,
            "only the placed lantern is checked"
        );
        let mut beside = pillar(Block::new(GOLD));
        beside.insert((5, 64, 4), standing());
        assert_eq!(
            found(&beside, (5, 64, 4)),
            None,
            "a lantern beside the pillar"
        );
    }

    #[test]
    fn margit_rises_facing_the_nearest_player() {
        let feet = DVec3::new(4.5, 63.0, 4.5);
        let south = DVec3::new(4.5, 64.0, 8.5);
        let east = DVec3::new(20.5, 63.0, 4.5);
        let west = DVec3::new(1.5, 63.0, 4.5);
        assert_eq!(facing(feet, &[east, south]), 0.0);
        assert_eq!(facing(feet, &[east]), -90.0);
        assert_eq!(facing(feet, &[east, west, south]), 90.0);
        assert_eq!(facing(feet, &[]), 0.0, "nobody");
        assert_eq!(facing(feet, &[feet]), 0.0, "on his spot");
    }
}
