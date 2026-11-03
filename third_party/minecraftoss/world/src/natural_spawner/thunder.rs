use super::spawning::SpawnContext;
use super::tick::TickInput;
use super::{CreatureSpawns, SpawnLevel, set};
use minecraftoss_core::chunk::HeightmapKind;
use minecraftoss_core::entity_data::place;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{BlockPos, BlockStateId, ChunkPos};

const SKELETON_HORSE: &str = "minecraft:skeleton_horse";
const LIGHTNING_RODS: &str = "minecraft:lightning_rods";
const OPEN_SKY: i32 = 15;
const REACH: f64 = 3.0;
const PLAYER_HALF_WIDTH: f64 = 0.3;
const PLAYER_HEIGHT: f64 = 1.8;
const TRAP_ODDS: f64 = 0.01;

impl CreatureSpawns {
    pub(super) fn strike_thunder(
        &self,
        level: &mut dyn SpawnLevel,
        context: &SpawnContext,
        input: &TickInput,
        chunk: ChunkPos,
    ) {
        let around = level.block_random_pos(chunk.min_block_x(), 0, chunk.min_block_z());
        let pos = lightning_target(level, input, around);
        if !level.raining_at(pos) {
            return;
        }
        let difficulty = Self::difficulty_at(level, context, pos);
        let trap = input.spawn_mobs
            && level.random().next_f64() < f64::from(difficulty.effective) * TRAP_ODDS
            && !self.is_lightning_rod(level.block(pos.below()));
        if trap {
            if let Some(horse) = self.trap_horse(level, pos) {
                level.add_entity(horse);
            }
        }
        let at = [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y),
            f64::from(pos.z) + 0.5,
        ];
        level.add_lightning(at, trap);
    }

    fn is_lightning_rod(&self, state: BlockStateId) -> bool {
        let rods = self.registries.block_tags.id(LIGHTNING_RODS);
        rods.is_some_and(|rods| self.registries.block_in_tag(state, rods))
    }

    fn trap_horse(&self, level: &mut dyn SpawnLevel, pos: BlockPos) -> Option<Tag> {
        let mut horse = self
            .registries
            .entities
            .as_ref()?
            .default_tag(SKELETON_HORSE)?;
        let uuid = level.next_uuid();
        let corner = [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)];
        place(&mut horse, corner, 0.0, 0.0, uuid);
        set(&mut horse, "SkeletonTrap", Tag::Byte(1));
        set(&mut horse, "Age", Tag::Int(0));
        Some(horse)
    }
}

fn lightning_target(level: &mut dyn SpawnLevel, input: &TickInput, around: BlockPos) -> BlockPos {
    let top = level.height(HeightmapKind::MotionBlocking, around.x, around.z);
    let column = BlockPos::new(around.x, top, around.z);
    if let Some(rod) = level.lightning_rod_near(column) {
        return rod.above();
    }
    let living = living_under_sky(level, input, column);
    if !living.is_empty() {
        let pick = level.random().next_i32_bound(living.len() as i32);
        return living[pick as usize];
    }
    if column.y == level.min_y() - 1 {
        column.offset(0, 2, 0)
    } else {
        column
    }
}

fn living_under_sky(
    level: &mut dyn SpawnLevel,
    input: &TickInput,
    column: BlockPos,
) -> Vec<BlockPos> {
    let low = [
        f64::from(column.x) - REACH,
        f64::from(column.y) - REACH,
        f64::from(column.z) - REACH,
    ];
    let high = [
        f64::from(column.x) + 1.0 + REACH,
        f64::from(level.max_y()) + 2.0 + REACH,
        f64::from(column.z) + 1.0 + REACH,
    ];
    let meets = |bb: [f64; 6]| (0..3).all(|axis| bb[axis] < high[axis] && bb[axis + 3] > low[axis]);
    let mobs = input
        .census
        .iter()
        .filter(|mob| meets(mob.bb))
        .map(|mob| mob.pos);
    let players = input
        .players
        .iter()
        .filter(|player| player.alive && meets(player_box(player.pos)))
        .map(|player| player.pos);
    let candidates: Vec<[f64; 3]> = mobs.chain(players).collect();
    let mut living = Vec::new();
    for pos in candidates {
        let block = BlockPos::new(
            pos[0].floor() as i32,
            pos[1].floor() as i32,
            pos[2].floor() as i32,
        );
        if level.sky_brightness(block) >= OPEN_SKY {
            living.push(block);
        }
    }
    living
}

fn player_box(feet: [f64; 3]) -> [f64; 6] {
    [
        feet[0] - PLAYER_HALF_WIDTH,
        feet[1],
        feet[2] - PLAYER_HALF_WIDTH,
        feet[0] + PLAYER_HALF_WIDTH,
        feet[1] + PLAYER_HEIGHT,
        feet[2] + PLAYER_HALF_WIDTH,
    ]
}

#[cfg(test)]
mod tests;
