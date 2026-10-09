use minecraft_net::Dimension;
use minecraftoss_core::{BlockStateId, Registries};

use crate::dimension::{BedRule, DimensionRules};
use crate::player_nbt::{GlobalPos, Respawn};

pub type Pos = (i32, i32, i32);

pub const BAD_RESPAWN_POINT_POWER: f32 = 5.0;
pub const STRAW_BED_BREAK_SOUND: &str = "minecraft:block.straw_bed.break_leave";
pub const TOO_FAR_AWAY: &str = "block.minecraft.bed.too_far_away";
pub const OBSTRUCTED: &str = "block.minecraft.bed.obstructed";
pub const OCCUPIED: &str = "block.minecraft.bed.occupied";
pub const SPAWN_SET: &str = "block.minecraft.set_spawn";

pub trait BlockView {
    fn state_at(&mut self, pos: Pos) -> BlockStateId;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bed {
    pub head: Pos,
    pub facing: (i32, i32),
    pub occupied: bool,
    pub straw: bool,
}

impl Bed {
    pub fn foot(&self) -> Pos {
        step(self.head, (-self.facing.0, -self.facing.1))
    }

    pub fn rule<'a>(&self, rules: &'a DimensionRules) -> &'a BedRule {
        if self.straw {
            &rules.straw_bed_rule
        } else {
            &rules.bed_rule
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BedUse {
    Incomplete,
    Destroy {
        removed: Vec<Pos>,
        blast: Option<[f64; 3]>,
        sound: Option<&'static str>,
        message: Option<String>,
    },
    Occupied,
    Sleep(Bed),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sleeper {
    pub feet: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub dimension: Dimension,
    pub dark: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rest {
    pub respawn: Option<Respawn>,
    pub problem: Option<String>,
}

pub fn horizontal(facing: &str) -> Option<(i32, i32)> {
    match facing {
        "north" => Some((0, -1)),
        "south" => Some((0, 1)),
        "west" => Some((-1, 0)),
        "east" => Some((1, 0)),
        _ => None,
    }
}

pub fn step((x, y, z): Pos, (dx, dz): (i32, i32)) -> Pos {
    (x + dx, y, z + dz)
}

pub fn centre((x, y, z): Pos) -> [f64; 3] {
    [f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5]
}

pub fn is_bed(registries: &Registries, state: BlockStateId) -> bool {
    let blocks = &registries.blocks;
    blocks
        .block(blocks.block_of(state))
        .is_a("AbstractBedBlock")
}

pub fn use_bed(
    world: &mut impl BlockView,
    registries: &Registries,
    rules: &DimensionRules,
    pos: Pos,
) -> Option<BedUse> {
    let blocks = &registries.blocks;
    let state = world.state_at(pos);
    if !is_bed(registries, state) {
        return None;
    }
    let facing = horizontal(blocks.property(state, "facing")?)?;
    let part = blocks.property(state, "part");
    let is_head = part == Some("head");
    let toward_other = if is_head {
        (-facing.0, -facing.1)
    } else {
        facing
    };
    let other_pos = step(pos, toward_other);
    let other = world.state_at(other_pos);
    if blocks.block_of(other) != blocks.block_of(state) || blocks.property(other, "part") == part {
        return Some(BedUse::Incomplete);
    }
    let (head, head_state) = if is_head {
        (pos, state)
    } else {
        (other_pos, other)
    };
    let bed = Bed {
        head,
        facing,
        occupied: blocks.property(head_state, "occupied") == Some("true"),
        straw: blocks
            .block(blocks.block_of(head_state))
            .is_a("StrawBedBlock"),
    };
    let rule = bed.rule(rules);
    if rule.destroy_on_use {
        let mut removed = vec![head];
        let foot = bed.foot();
        if blocks.block_of(world.state_at(foot)) == blocks.block_of(head_state) {
            removed.push(foot);
        }
        return Some(BedUse::Destroy {
            removed,
            blast: (!bed.straw).then(|| centre(head)),
            sound: bed.straw.then_some(STRAW_BED_BREAK_SOUND),
            message: rule.error_message.clone(),
        });
    }
    if bed.occupied {
        return Some(BedUse::Occupied);
    }
    Some(BedUse::Sleep(bed))
}

fn reachable(feet: [f64; 3], (x, y, z): Pos) -> bool {
    (feet[0] - (f64::from(x) + 0.5)).abs() <= 3.0
        && (feet[1] - f64::from(y)).abs() <= 2.0
        && (feet[2] - (f64::from(z) + 0.5)).abs() <= 3.0
}

fn suffocates(world: &mut impl BlockView, registries: &Registries, pos: Pos) -> bool {
    registries.blocks.state(world.state_at(pos)).suffocating
}

pub fn rest_in_bed(
    world: &mut impl BlockView,
    registries: &Registries,
    rules: &DimensionRules,
    bed: &Bed,
    sleeper: &Sleeper,
) -> Result<Rest, &'static str> {
    if !reachable(sleeper.feet, bed.head) && !reachable(sleeper.feet, bed.foot()) {
        return Err(TOO_FAR_AWAY);
    }
    let above = (bed.head.0, bed.head.1 + 1, bed.head.2);
    let beside = step(above, (-bed.facing.0, -bed.facing.1));
    if suffocates(world, registries, above) || suffocates(world, registries, beside) {
        return Err(OBSTRUCTED);
    }
    let rule = bed.rule(rules);
    Ok(Rest {
        respawn: rule.can_set_spawn.holds(sleeper.dark).then_some(Respawn {
            at: GlobalPos {
                dimension: sleeper.dimension,
                pos: bed.head,
            },
            yaw: sleeper.yaw,
            pitch: sleeper.pitch,
            forced: false,
        }),
        problem: if rule.can_sleep.holds(sleeper.dark) {
            None
        } else {
            rule.error_message.clone()
        },
    })
}

pub fn set_respawn(point: &mut Option<Respawn>, respawn: Respawn) -> bool {
    let moved = point.is_none_or(|old| old.at != respawn.at);
    *point = Some(respawn);
    moved
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    pub(crate) struct Blocks<'a> {
        pub(crate) registries: &'a Registries,
        pub(crate) placed: HashMap<Pos, BlockStateId>,
    }

    impl<'a> Blocks<'a> {
        pub(crate) fn new(registries: &'a Registries) -> Self {
            Self {
                registries,
                placed: HashMap::new(),
            }
        }

        pub(crate) fn set(&mut self, pos: Pos, text: &str) {
            let state = self.registries.blocks.parse_state(text).unwrap();
            self.placed.insert(pos, state);
        }
    }

    impl BlockView for Blocks<'_> {
        fn state_at(&mut self, pos: Pos) -> BlockStateId {
            self.placed.get(&pos).copied().unwrap_or(BlockStateId::AIR)
        }
    }

    pub(crate) fn registries() -> Option<Registries> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        Registries::load(&paths).ok()
    }

    fn bed(world: &mut Blocks, id: &str, foot: Pos, facing: &str, occupied: bool) {
        let (dx, dz) = horizontal(facing).unwrap();
        world.set(
            foot,
            &format!("{id}[facing={facing},part=foot,occupied={occupied}]"),
        );
        world.set(
            step(foot, (dx, dz)),
            &format!("{id}[facing={facing},part=head,occupied={occupied}]"),
        );
    }

    #[test]
    fn a_bed_explodes_from_its_head_in_the_nether_and_the_end_but_not_the_overworld() {
        let Some(registries) = registries() else {
            return;
        };
        for dimension in [Dimension::Nether, Dimension::End] {
            let rules = DimensionRules::load(&registries, dimension).unwrap();
            let mut world = Blocks::new(&registries);
            bed(&mut world, "minecraft:red_bed", (0, 64, 0), "east", false);
            for clicked in [(0, 64, 0), (1, 64, 0)] {
                assert_eq!(
                    use_bed(&mut world, &registries, &rules, clicked),
                    Some(BedUse::Destroy {
                        removed: vec![(1, 64, 0), (0, 64, 0)],
                        blast: Some([1.5, 64.5, 0.5]),
                        sound: None,
                        message: None,
                    }),
                    "{dimension:?} from {clicked:?}"
                );
            }
        }
        let overworld = DimensionRules::load(&registries, Dimension::Overworld).unwrap();
        let mut world = Blocks::new(&registries);
        bed(&mut world, "minecraft:blue_bed", (5, 70, 5), "north", false);
        assert_eq!(
            use_bed(&mut world, &registries, &overworld, (5, 70, 5)),
            Some(BedUse::Sleep(Bed {
                head: (5, 70, 4),
                facing: (0, -1),
                occupied: false,
                straw: false,
            }))
        );
        bed(&mut world, "minecraft:blue_bed", (5, 70, 5), "north", true);
        assert_eq!(
            use_bed(&mut world, &registries, &overworld, (5, 70, 4)),
            Some(BedUse::Occupied)
        );
    }

    #[test]
    fn a_straw_bed_just_breaks_and_half_a_bed_does_nothing() {
        let Some(registries) = registries() else {
            return;
        };
        let nether = DimensionRules::load(&registries, Dimension::Nether).unwrap();
        let mut world = Blocks::new(&registries);
        bed(
            &mut world,
            "minecraft:straw_bed",
            (0, 40, 0),
            "south",
            false,
        );
        assert_eq!(
            use_bed(&mut world, &registries, &nether, (0, 40, 0)),
            Some(BedUse::Destroy {
                removed: vec![(0, 40, 1), (0, 40, 0)],
                blast: None,
                sound: Some(STRAW_BED_BREAK_SOUND),
                message: None,
            })
        );
        world.placed.remove(&(0, 40, 1));
        assert_eq!(
            use_bed(&mut world, &registries, &nether, (0, 40, 0)),
            Some(BedUse::Incomplete)
        );
        bed(&mut world, "minecraft:red_bed", (3, 40, 0), "south", false);
        world.set(
            (3, 40, 1),
            "minecraft:blue_bed[facing=south,part=head,occupied=false]",
        );
        assert_eq!(
            use_bed(&mut world, &registries, &nether, (3, 40, 0)),
            Some(BedUse::Incomplete),
            "the other half is another bed"
        );
        world.set((6, 40, 0), "minecraft:stone");
        assert_eq!(use_bed(&mut world, &registries, &nether, (6, 40, 0)), None);
    }
}

#[cfg(test)]
mod rest_tests {
    use super::tests::{Blocks, registries};
    use super::*;

    fn sleeper(feet: [f64; 3], dark: bool) -> Sleeper {
        Sleeper {
            feet,
            yaw: 30.0,
            pitch: 10.0,
            dimension: Dimension::Overworld,
            dark,
        }
    }

    #[test]
    fn a_bed_in_reach_sets_the_spawn_by_day_and_says_you_can_sleep_only_at_night() {
        let Some(registries) = registries() else {
            return;
        };
        let rules = DimensionRules::load(&registries, Dimension::Overworld).unwrap();
        let mut world = Blocks::new(&registries);
        let bed = Bed {
            head: (0, 64, 1),
            facing: (0, 1),
            occupied: false,
            straw: false,
        };
        let point = Respawn {
            at: GlobalPos {
                dimension: Dimension::Overworld,
                pos: (0, 64, 1),
            },
            yaw: 30.0,
            pitch: 10.0,
            forced: false,
        };
        assert_eq!(
            rest_in_bed(
                &mut world,
                &registries,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, -2.0], false)
            ),
            Ok(Rest {
                respawn: Some(point),
                problem: Some("block.minecraft.bed.no_sleep".to_owned()),
            }),
            "three blocks from the foot is in reach"
        );
        assert_eq!(
            rest_in_bed(
                &mut world,
                &registries,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, 1.5], true)
            ),
            Ok(Rest {
                respawn: Some(point),
                problem: None,
            })
        );
        for far in [[0.5, 64.0, -2.6], [4.6, 64.0, 1.5], [0.5, 66.5, 1.5]] {
            assert_eq!(
                rest_in_bed(&mut world, &registries, &rules, &bed, &sleeper(far, true)),
                Err(TOO_FAR_AWAY),
                "{far:?}"
            );
        }
        world.set((0, 65, 0), "minecraft:stone");
        assert_eq!(
            rest_in_bed(
                &mut world,
                &registries,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, 2.5], true)
            ),
            Err(OBSTRUCTED)
        );
        world.set((0, 65, 0), "minecraft:glass");
        assert!(
            rest_in_bed(
                &mut world,
                &registries,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, 2.5], true)
            )
            .is_ok(),
            "glass does not suffocate"
        );
        let straw = Bed { straw: true, ..bed };
        assert_eq!(
            rest_in_bed(
                &mut world,
                &registries,
                &rules,
                &straw,
                &sleeper([0.5, 64.0, 2.5], true)
            )
            .map(|rest| rest.respawn),
            Ok(None),
            "a straw bed never sets the spawn"
        );
    }

    #[test]
    fn the_spawn_message_comes_only_when_the_point_moves() {
        let at = |pos| Respawn {
            at: GlobalPos {
                dimension: Dimension::Overworld,
                pos,
            },
            yaw: 0.0,
            pitch: 0.0,
            forced: false,
        };
        let mut point = None;
        assert!(set_respawn(&mut point, at((1, 64, 1))));
        assert!(!set_respawn(&mut point, at((1, 64, 1))));
        assert!(set_respawn(&mut point, at((2, 64, 1))));
        assert_eq!(point, Some(at((2, 64, 1))));
    }
}
