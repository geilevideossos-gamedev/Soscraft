use minecraft_net::Dimension;
use minecraftoss_core::block::FluidKind;
use minecraftoss_core::{BlockStateId, Registries};

use crate::dimension::{BedRule, DimensionRules};
use crate::level::Level;
use crate::player::Player;
use crate::player_nbt::{GlobalPos, Place, Respawn};
use crate::portal_forcer::ChunkWorld;
use crate::server::Globals;

pub type Pos = (i32, i32, i32);
type StandTry<'a> = (Pos, &'a [(i32, i32)], bool);

pub const BAD_RESPAWN_POINT_POWER: f32 = 5.0;
pub const STRAW_BED_BREAK_SOUND: &str = "minecraft:block.straw_bed.break_leave";
pub const TOO_FAR_AWAY: &str = "block.minecraft.bed.too_far_away";
pub const OBSTRUCTED: &str = "block.minecraft.bed.obstructed";
pub const OCCUPIED: &str = "block.minecraft.bed.occupied";
pub const NOT_SAFE: &str = "block.minecraft.bed.not_safe";
pub const SPAWN_SET: &str = "block.minecraft.set_spawn";
pub const GLOWSTONE: &str = "minecraft:glowstone";
pub const MAX_CHARGES: u8 = 4;
pub const ANCHOR_CHARGE_SOUND: &str = "minecraft:block.respawn_anchor.charge";
pub const ANCHOR_SET_SPAWN_SOUND: &str = "minecraft:block.respawn_anchor.set_spawn";
pub const ANCHOR_DEPLETE_SOUND: &str = "minecraft:block.respawn_anchor.deplete";

const HORIZONTAL: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
const ANCHOR_SIDES: [(i32, i32); 8] = [
    (0, -1),
    (-1, 0),
    (0, 1),
    (1, 0),
    (-1, -1),
    (1, -1),
    (-1, 1),
    (1, 1),
];
const PLAYER_HALF_WIDTH: f64 = 0.3;
const PLAYER_HEIGHT: f64 = 1.8;
const COLLISION_EPSILON: f64 = 1.0e-7;
const DANGEROUS: [&str; 7] = [
    "minecraft:lava",
    "minecraft:magma_block",
    "minecraft:lava_cauldron",
    "minecraft:wither_rose",
    "minecraft:sweet_berry_bush",
    "minecraft:cactus",
    "minecraft:powder_snow",
];

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
    Occupied(Pos),
    Sleep(Bed),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sleeper {
    pub feet: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub dimension: Dimension,
    pub dark: bool,
    pub creative: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Sleep,
    Problem(Option<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rest {
    pub respawn: Option<Respawn>,
    pub outcome: Outcome,
}

impl Rest {
    fn problem(key: &str) -> Self {
        Self {
            respawn: None,
            outcome: Outcome::Problem(Some(key.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fuel {
    pub main: bool,
    pub held: bool,
    pub offhand: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnchorUse {
    Charge { pos: Pos, state: BlockStateId },
    SetSpawn(Respawn),
    Explode { pos: Pos, shielded: bool },
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
        return Some(BedUse::Occupied(head));
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
    monsters: impl FnOnce([f64; 6]) -> bool,
) -> Rest {
    if !reachable(sleeper.feet, bed.head) && !reachable(sleeper.feet, bed.foot()) {
        return Rest::problem(TOO_FAR_AWAY);
    }
    let above = (bed.head.0, bed.head.1 + 1, bed.head.2);
    let beside = step(above, (-bed.facing.0, -bed.facing.1));
    if suffocates(world, registries, above) || suffocates(world, registries, beside) {
        return Rest::problem(OBSTRUCTED);
    }
    let rule = bed.rule(rules);
    let respawn = rule.can_set_spawn.holds(sleeper.dark).then_some(Respawn {
        at: GlobalPos {
            dimension: sleeper.dimension,
            pos: bed.head,
        },
        yaw: sleeper.yaw,
        pitch: sleeper.pitch,
        forced: false,
    });
    let outcome = if !rule.can_sleep.holds(sleeper.dark) {
        Outcome::Problem(rule.error_message.clone())
    } else if !sleeper.creative && monsters(crate::sleep::monster_box(bed.head)) {
        Outcome::Problem(Some(NOT_SAFE.to_owned()))
    } else {
        Outcome::Sleep
    };
    Rest { respawn, outcome }
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
            use_bed(&mut world, &registries, &overworld, (5, 70, 5)),
            Some(BedUse::Occupied((5, 70, 4)))
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
            creative: false,
        }
    }

    fn rest(
        world: &mut Blocks,
        rules: &DimensionRules,
        bed: &Bed,
        sleeper: &Sleeper,
        monsters: bool,
    ) -> Rest {
        let registries = world.registries;
        rest_in_bed(world, registries, rules, bed, sleeper, |_| monsters)
    }

    fn problem(key: &str) -> Outcome {
        Outcome::Problem(Some(key.to_owned()))
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
            rest(
                &mut world,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, -2.0], false),
                false
            ),
            Rest {
                respawn: Some(point),
                outcome: problem("block.minecraft.bed.no_sleep"),
            },
            "three blocks from the foot is in reach"
        );
        assert_eq!(
            rest(
                &mut world,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, 1.5], true),
                false
            ),
            Rest {
                respawn: Some(point),
                outcome: Outcome::Sleep,
            }
        );
        for far in [[0.5, 64.0, -2.6], [4.6, 64.0, 1.5], [0.5, 66.5, 1.5]] {
            assert_eq!(
                rest(&mut world, &rules, &bed, &sleeper(far, true), false),
                Rest::problem(TOO_FAR_AWAY),
                "{far:?}"
            );
        }
        world.set((0, 65, 0), "minecraft:stone");
        assert_eq!(
            rest(
                &mut world,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, 2.5], true),
                false
            ),
            Rest::problem(OBSTRUCTED)
        );
        world.set((0, 65, 0), "minecraft:glass");
        assert_eq!(
            rest(
                &mut world,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, 2.5], true),
                false
            )
            .outcome,
            Outcome::Sleep,
            "glass does not suffocate"
        );
        let straw = Bed { straw: true, ..bed };
        assert_eq!(
            rest(
                &mut world,
                &rules,
                &straw,
                &sleeper([0.5, 64.0, 2.5], true),
                false
            )
            .respawn,
            None,
            "a straw bed never sets the spawn"
        );
    }

    #[test]
    fn the_rest_checks_run_in_vanilla_order() {
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
        world.set((0, 65, 1), "minecraft:stone");
        assert_eq!(
            rest(
                &mut world,
                &rules,
                &bed,
                &sleeper([0.5, 64.0, 9.5], true),
                true
            ),
            Rest::problem(TOO_FAR_AWAY),
            "too far beats obstructed"
        );
        world.placed.remove(&(0, 65, 1));
        let night = rest(
            &mut world,
            &rules,
            &bed,
            &sleeper([0.5, 64.0, 2.5], true),
            true,
        );
        assert!(night.respawn.is_some(), "the spawn is set with monsters near");
        assert_eq!(night.outcome, problem(NOT_SAFE));
        let day = rest(
            &mut world,
            &rules,
            &bed,
            &sleeper([0.5, 64.0, 2.5], false),
            true,
        );
        assert!(day.respawn.is_some(), "the spawn is set by day");
        assert_eq!(
            day.outcome,
            problem("block.minecraft.bed.no_sleep"),
            "the night comes before the monsters"
        );
        let creative = Sleeper {
            creative: true,
            ..sleeper([0.5, 64.0, 2.5], true)
        };
        let mut asked = false;
        let rested = rest_in_bed(&mut world, &registries, &rules, &bed, &creative, |_| {
            asked = true;
            true
        });
        assert_eq!(rested.outcome, Outcome::Sleep, "creative ignores monsters");
        assert!(!asked);
        let mut area = None;
        rest_in_bed(
            &mut world,
            &registries,
            &rules,
            &bed,
            &sleeper([0.5, 64.0, 2.5], true),
            |seen| {
                area = Some(seen);
                false
            },
        );
        assert_eq!(area, Some([-7.5, 59.0, -6.5, 8.5, 69.0, 9.5]));
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

pub fn is_anchor(registries: &Registries, state: BlockStateId) -> bool {
    let blocks = &registries.blocks;
    blocks
        .block(blocks.block_of(state))
        .is_a("RespawnAnchorBlock")
}

pub fn charges(registries: &Registries, state: BlockStateId) -> u8 {
    registries
        .blocks
        .property(state, "charges")
        .and_then(|charges| charges.parse().ok())
        .unwrap_or(0)
}

pub fn with_charges(
    registries: &Registries,
    state: BlockStateId,
    charges: u8,
) -> Option<BlockStateId> {
    registries
        .blocks
        .with_property(state, "charges", &charges.to_string())
}

fn water_would_flow(world: &mut impl BlockView, registries: &Registries, pos: Pos) -> bool {
    let water = |state: BlockStateId| {
        registries
            .blocks
            .state(state)
            .fluid
            .filter(|fluid| fluid.kind == FluidKind::Water)
    };
    let Some(fluid) = water(world.state_at(pos)) else {
        return false;
    };
    if fluid.source {
        return true;
    }
    fluid.amount >= 2 && water(world.state_at((pos.0, pos.1 - 1, pos.2))).is_none()
}

pub fn shielded_by_water(world: &mut impl BlockView, registries: &Registries, pos: Pos) -> bool {
    let water_above = registries
        .blocks
        .state(world.state_at((pos.0, pos.1 + 1, pos.2)))
        .fluid
        .is_some_and(|fluid| fluid.kind == FluidKind::Water);
    HORIZONTAL
        .iter()
        .any(|&side| water_would_flow(world, registries, step(pos, side)))
        || water_above
}

pub fn use_anchor(
    world: &mut impl BlockView,
    registries: &Registries,
    rules: &DimensionRules,
    pos: Pos,
    dimension: Dimension,
    fuel: Fuel,
) -> Option<AnchorUse> {
    let state = world.state_at(pos);
    if !is_anchor(registries, state) {
        return None;
    }
    let charge = charges(registries, state);
    let chargeable = charge < MAX_CHARGES;
    if fuel.held && chargeable {
        return Some(AnchorUse::Charge {
            pos,
            state: with_charges(registries, state, charge + 1)?,
        });
    }
    if !fuel.main || (fuel.offhand && chargeable) || charge == 0 {
        return None;
    }
    if rules.respawn_anchor_works {
        return Some(AnchorUse::SetSpawn(Respawn {
            at: GlobalPos { dimension, pos },
            yaw: 0.0,
            pitch: 0.0,
            forced: false,
        }));
    }
    Some(AnchorUse::Explode {
        pos,
        shielded: shielded_by_water(world, registries, pos),
    })
}

#[cfg(test)]
mod anchor_tests {
    use super::tests::{Blocks, registries};
    use super::*;

    const MAIN_GLOWSTONE: Fuel = Fuel {
        main: true,
        held: true,
        offhand: false,
    };
    const EMPTY_HAND: Fuel = Fuel {
        main: true,
        held: false,
        offhand: false,
    };

    fn anchor(world: &mut Blocks, pos: Pos, charges: u8) {
        world.set(pos, &format!("minecraft:respawn_anchor[charges={charges}]"));
    }

    fn used(world: &mut Blocks, dimension: Dimension, fuel: Fuel) -> Option<AnchorUse> {
        let registries = world.registries;
        let rules = DimensionRules::load(registries, dimension).unwrap();
        use_anchor(world, registries, &rules, (0, 64, 0), dimension, fuel)
    }

    #[test]
    fn glowstone_charges_an_anchor_up_to_four_from_either_hand() {
        let Some(registries) = registries() else {
            return;
        };
        let mut world = Blocks::new(&registries);
        for charge in 0..MAX_CHARGES {
            anchor(&mut world, (0, 64, 0), charge);
            let Some(AnchorUse::Charge { pos, state }) =
                used(&mut world, Dimension::Overworld, MAIN_GLOWSTONE)
            else {
                panic!("charge {charge} takes glowstone");
            };
            assert_eq!(pos, (0, 64, 0));
            assert_eq!(charges(&registries, state), charge + 1);
        }
        anchor(&mut world, (0, 64, 0), 2);
        let offhand = Fuel {
            main: false,
            held: true,
            offhand: true,
        };
        assert!(matches!(
            used(&mut world, Dimension::Nether, offhand),
            Some(AnchorUse::Charge { .. })
        ));
        let main_empty_offhand_glowstone = Fuel {
            main: true,
            held: false,
            offhand: true,
        };
        assert_eq!(
            used(&mut world, Dimension::Nether, main_empty_offhand_glowstone),
            None,
            "the main hand passes to the offhand's glowstone"
        );
        anchor(&mut world, (0, 64, 0), 4);
        assert!(
            matches!(
                used(&mut world, Dimension::Nether, MAIN_GLOWSTONE),
                Some(AnchorUse::SetSpawn(_))
            ),
            "a full anchor is used as with an empty hand"
        );
    }

    #[test]
    fn a_charged_anchor_sets_the_spawn_in_the_nether_and_explodes_elsewhere() {
        let Some(registries) = registries() else {
            return;
        };
        let mut world = Blocks::new(&registries);
        anchor(&mut world, (0, 64, 0), 0);
        for dimension in [Dimension::Nether, Dimension::Overworld] {
            assert_eq!(
                used(&mut world, dimension, EMPTY_HAND),
                None,
                "an empty anchor does nothing"
            );
        }
        anchor(&mut world, (0, 64, 0), 1);
        assert_eq!(
            used(&mut world, Dimension::Nether, EMPTY_HAND),
            Some(AnchorUse::SetSpawn(Respawn {
                at: GlobalPos {
                    dimension: Dimension::Nether,
                    pos: (0, 64, 0),
                },
                yaw: 0.0,
                pitch: 0.0,
                forced: false,
            }))
        );
        let offhand_use = Fuel {
            main: false,
            held: false,
            offhand: false,
        };
        assert_eq!(used(&mut world, Dimension::Nether, offhand_use), None);
        for dimension in [Dimension::Overworld, Dimension::End] {
            assert_eq!(
                used(&mut world, dimension, EMPTY_HAND),
                Some(AnchorUse::Explode {
                    pos: (0, 64, 0),
                    shielded: false,
                })
            );
        }
    }

    #[test]
    fn flowing_water_beside_or_any_water_above_keeps_the_blocks_around_an_anchor() {
        let Some(registries) = registries() else {
            return;
        };
        let shielded = |placed: &[(Pos, &str)]| {
            let mut world = Blocks::new(&registries);
            anchor(&mut world, (0, 64, 0), 1);
            for &(pos, text) in placed {
                world.set(pos, text);
            }
            shielded_by_water(&mut world, &registries, (0, 64, 0))
        };
        assert!(!shielded(&[]));
        assert!(shielded(&[((1, 64, 0), "minecraft:water[level=0]")]));
        assert!(shielded(&[((0, 64, -1), "minecraft:water[level=6]")]));
        assert!(
            !shielded(&[((0, 64, -1), "minecraft:water[level=7]")]),
            "the thinnest flowing water would not flow on"
        );
        assert!(shielded(&[(
            (0, 64, 1),
            "minecraft:oak_slab[type=bottom,waterlogged=true]"
        )]));
        assert!(
            !shielded(&[
                ((0, 64, 1), "minecraft:water[level=3]"),
                ((0, 63, 1), "minecraft:water[level=0]")
            ]),
            "flowing water over water would not flow"
        );
        assert!(
            shielded(&[((0, 64, 1), "minecraft:water[level=3]")]),
            "flowing water over air would"
        );
        assert!(shielded(&[((0, 65, 0), "minecraft:water[level=6]")]));
        assert!(!shielded(&[((1, 64, 0), "minecraft:lava[level=0]")]));
        assert!(
            !shielded(&[((1, 65, 0), "minecraft:water[level=0]")]),
            "only straight above counts"
        );
    }
}

impl BlockView for ChunkWorld<'_> {
    fn state_at(&mut self, pos: Pos) -> BlockStateId {
        self.state(pos)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnBlock {
    pub feet: [f64; 3],
    pub yaw: f32,
    pub depleted: Option<(Pos, BlockStateId)>,
}

fn tagged(registries: &Registries, state: BlockStateId, tag: &str) -> bool {
    registries
        .block_tags
        .id(tag)
        .is_some_and(|tag| registries.block_in_tag(state, tag))
}

fn dangerous(registries: &Registries, state: BlockStateId) -> bool {
    let blocks = &registries.blocks;
    let name = blocks.block(blocks.block_of(state)).name.as_str();
    tagged(registries, state, "minecraft:fire")
        || DANGEROUS.contains(&name)
        || (tagged(registries, state, "minecraft:campfires")
            && blocks.property(state, "lit") == Some("true"))
}

fn non_climbable_top(world: &mut impl BlockView, registries: &Registries, pos: Pos) -> f64 {
    let blocks = &registries.blocks;
    let state = world.state_at(pos);
    let open_trapdoor = blocks.block(blocks.block_of(state)).is_a("TrapDoorBlock")
        && blocks.property(state, "open") == Some("true");
    if tagged(registries, state, "minecraft:climbable") || open_trapdoor {
        return f64::NEG_INFINITY;
    }
    blocks
        .collision_boxes(state)
        .iter()
        .map(|b| b[4])
        .fold(f64::NEG_INFINITY, f64::max)
}

fn floor_height(world: &mut impl BlockView, registries: &Registries, pos: Pos) -> f64 {
    let top = non_climbable_top(world, registries, pos);
    if top.is_finite() {
        return top;
    }
    let below = non_climbable_top(world, registries, (pos.0, pos.1 - 1, pos.2));
    if below >= 1.0 {
        below - 1.0
    } else {
        f64::NEG_INFINITY
    }
}

fn collides(world: &mut impl BlockView, registries: &Registries, body: [f64; 6]) -> bool {
    let span = |min: f64, max: f64| {
        (min - COLLISION_EPSILON).floor() as i32 - 1..=(max + COLLISION_EPSILON).floor() as i32 + 1
    };
    for x in span(body[0], body[3]) {
        for y in span(body[1], body[4]) {
            for z in span(body[2], body[5]) {
                let state = world.state_at((x, y, z));
                let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
                let hit = registries
                    .blocks
                    .collision_boxes(state)
                    .iter()
                    .map(|b| [b[0] + x, b[1] + y, b[2] + z, b[3] + x, b[4] + y, b[5] + z])
                    .any(|b| {
                        (0..3).all(|i| {
                            b[i] < body[i + 3] - COLLISION_EPSILON
                                && b[i + 3] > body[i] + COLLISION_EPSILON
                        })
                    });
                if hit {
                    return true;
                }
            }
        }
    }
    false
}

pub fn safe_dismount(
    world: &mut impl BlockView,
    registries: &Registries,
    pos: Pos,
    check_dangerous: bool,
) -> Option<[f64; 3]> {
    if check_dangerous && dangerous(registries, world.state_at(pos)) {
        return None;
    }
    let floor = floor_height(world, registries, pos);
    if !floor.is_finite() || floor >= 1.0 {
        return None;
    }
    let below = (pos.0, pos.1 - 1, pos.2);
    if check_dangerous && floor <= 0.0 && dangerous(registries, world.state_at(below)) {
        return None;
    }
    let feet = [
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + floor,
        f64::from(pos.2) + 0.5,
    ];
    let body = [
        feet[0] - PLAYER_HALF_WIDTH,
        feet[1],
        feet[2] - PLAYER_HALF_WIDTH,
        feet[0] + PLAYER_HALF_WIDTH,
        feet[1] + PLAYER_HEIGHT,
        feet[2] + PLAYER_HALF_WIDTH,
    ];
    if collides(world, registries, body) {
        return None;
    }
    let above = (pos.0, pos.1 + 1, pos.2);
    if tagged(
        registries,
        world.state_at(pos),
        "minecraft:invalid_spawn_inside",
    ) || tagged(
        registries,
        world.state_at(above),
        "minecraft:invalid_spawn_inside",
    ) {
        return None;
    }
    let inside = crate::portal::within_border(body[0], body[2])
        && crate::portal::within_border(body[3], body[5]);
    inside.then_some(feet)
}

pub fn anchor_stand_up(
    world: &mut impl BlockView,
    registries: &Registries,
    pos: Pos,
) -> Option<[f64; 3]> {
    let offsets: Vec<(i32, i32, i32)> = ANCHOR_SIDES
        .iter()
        .map(|&(dx, dz)| (dx, 0, dz))
        .chain(ANCHOR_SIDES.iter().map(|&(dx, dz)| (dx, -1, dz)))
        .chain(ANCHOR_SIDES.iter().map(|&(dx, dz)| (dx, 1, dz)))
        .chain(std::iter::once((0, 1, 0)))
        .collect();
    [true, false].into_iter().find_map(|check_dangerous| {
        offsets.iter().find_map(|&(dx, dy, dz)| {
            safe_dismount(
                world,
                registries,
                (pos.0 + dx, pos.1 + dy, pos.2 + dz),
                check_dangerous,
            )
        })
    })
}

fn clockwise((dx, dz): (i32, i32)) -> (i32, i32) {
    (-dz, dx)
}

fn facing_angle((dx, dz): (i32, i32), yaw: f32) -> bool {
    let radians = yaw * (std::f32::consts::PI / 180.0);
    dx as f32 * -radians.sin() + dz as f32 * radians.cos() > 0.0
}

fn bed_surround(forward: (i32, i32), side: (i32, i32)) -> [(i32, i32); 10] {
    let (fx, fz) = forward;
    let (sx, sz) = side;
    [
        (sx, sz),
        (sx - fx, sz - fz),
        (sx - fx * 2, sz - fz * 2),
        (-fx * 2, -fz * 2),
        (-sx - fx * 2, -sz - fz * 2),
        (-sx - fx, -sz - fz),
        (-sx, -sz),
        (-sx + fx, -sz + fz),
        (fx, fz),
        (sx + fx, sz + fz),
    ]
}

fn bed_above(forward: (i32, i32)) -> [(i32, i32); 2] {
    [(0, 0), (-forward.0, -forward.1)]
}

fn stand_at(
    world: &mut impl BlockView,
    registries: &Registries,
    pos: Pos,
    offsets: &[(i32, i32)],
    check_dangerous: bool,
) -> Option<[f64; 3]> {
    offsets.iter().find_map(|&(dx, dz)| {
        safe_dismount(
            world,
            registries,
            (pos.0 + dx, pos.1, pos.2 + dz),
            check_dangerous,
        )
    })
}

pub fn bed_stand_up(
    world: &mut impl BlockView,
    registries: &Registries,
    head: Pos,
    forward: (i32, i32),
    yaw: f32,
) -> Option<[f64; 3]> {
    let right = clockwise(forward);
    let side = if facing_angle(right, yaw) {
        (-right.0, -right.1)
    } else {
        right
    };
    let surround = bed_surround(forward, side);
    let above = bed_above(forward);
    let below = (head.0, head.1 - 1, head.2);
    if is_bed(registries, world.state_at(below)) {
        let tries: [StandTry; 6] = [
            (head, &surround, true),
            (below, &surround, true),
            (head, &above, true),
            (head, &surround, false),
            (below, &surround, false),
            (head, &above, false),
        ];
        return tries
            .into_iter()
            .find_map(|(pos, offsets, check)| stand_at(world, registries, pos, offsets, check));
    }
    let offsets: Vec<(i32, i32)> = surround.iter().chain(above.iter()).copied().collect();
    stand_at(world, registries, head, &offsets, true)
        .or_else(|| stand_at(world, registries, head, &offsets, false))
}

pub fn look_at_yaw(feet: [f64; 3], (x, _, z): Pos) -> f32 {
    let dx = f64::from(x) + 0.5 - feet[0];
    let dz = f64::from(z) + 0.5 - feet[2];
    let length = (dx * dx + dz * dz).sqrt();
    let (dx, dz) = if length < 1.0e-4 {
        (0.0, 0.0)
    } else {
        (dx / length, dz / length)
    };
    let mut yaw = (dz.atan2(dx) * 57.295_776_367_187_5 - 90.0) % 360.0;
    if yaw >= 180.0 {
        yaw -= 360.0;
    }
    if yaw < -180.0 {
        yaw += 360.0;
    }
    yaw as f32
}

pub fn spawn_block(
    world: &mut impl BlockView,
    registries: &Registries,
    rules: &DimensionRules,
    respawn: &Respawn,
    dark: bool,
    consume: bool,
) -> Option<SpawnBlock> {
    let pos = respawn.at.pos;
    let state = world.state_at(pos);
    let blocks = &registries.blocks;
    if is_anchor(registries, state) {
        let charge = charges(registries, state);
        if (!respawn.forced && charge == 0) || !rules.respawn_anchor_works {
            return None;
        }
        let feet = anchor_stand_up(world, registries, pos)?;
        let depleted = (!respawn.forced && consume)
            .then(|| with_charges(registries, state, charge - 1).map(|state| (pos, state)))
            .flatten();
        return Some(SpawnBlock {
            feet,
            yaw: look_at_yaw(feet, pos),
            depleted,
        });
    }
    if is_bed(registries, state) {
        let straw = blocks.block(blocks.block_of(state)).is_a("StrawBedBlock");
        let rule = if straw {
            &rules.straw_bed_rule
        } else {
            &rules.bed_rule
        };
        if !rule.can_set_spawn.holds(dark) {
            return None;
        }
        let forward = horizontal(blocks.property(state, "facing")?)?;
        let feet = bed_stand_up(world, registries, pos, forward, respawn.yaw)?;
        return Some(SpawnBlock {
            feet,
            yaw: look_at_yaw(feet, pos),
            depleted: None,
        });
    }
    None
}

pub fn respawn_at_block(globals: &Globals, level: &mut Level, player: &mut Player) -> bool {
    return_to_block(globals, level, player, true)
}

pub fn return_to_block(
    globals: &Globals,
    level: &mut Level,
    player: &mut Player,
    consume: bool,
) -> bool {
    let dark = level.dark_outside(globals);
    let found = crate::respawn::respawn_point(player)
        .filter(|point| point.at.dimension == level.dimension)
        .and_then(|point| {
            let mut world = ChunkWorld::new(&mut level.terrain.stream);
            spawn_block(
                &mut world,
                &globals.registries,
                &level.terrain.rules,
                &point,
                dark,
                consume,
            )
        });
    let dimension = level.dimension;
    let place = found.map(|block| Place::standing(dimension, block.feet, block.yaw, 0.0));
    let missing = crate::respawn::respawn_in(globals, level, player, |_| place);
    if let Some((pos, state)) = found.and_then(|block| block.depleted) {
        deplete(level, pos, state);
    }
    missing
}

fn deplete(level: &mut Level, pos: Pos, state: BlockStateId) {
    let terrain = &mut level.terrain;
    let block = terrain.stream.states.block(state).cloned();
    match level.entities.as_mut() {
        Some(entities) => {
            crate::level::write_blocks(terrain, entities, vec![(pos, block)]);
            let center = centre(pos);
            entities.sounds.push((
                ANCHOR_DEPLETE_SOUND.to_owned(),
                glam::DVec3::from_array(center),
                1.0,
                1.0,
            ));
        }
        None => {
            terrain.scene.set(pos, block);
            terrain.stream.record_edits(&terrain.scene, &[pos]);
            terrain.stream.mark_edited(&terrain.scene, &[pos]);
        }
    }
}

#[cfg(test)]
mod spawn_tests {
    use super::tests::{Blocks, registries};
    use super::*;

    fn floor(world: &mut Blocks, y: i32) {
        for x in -4..=4 {
            for z in -4..=4 {
                world.set((x, y, z), "minecraft:stone");
            }
        }
    }

    fn point(dimension: Dimension, pos: Pos, yaw: f32, forced: bool) -> Respawn {
        Respawn {
            at: GlobalPos { dimension, pos },
            yaw,
            pitch: 0.0,
            forced,
        }
    }

    #[test]
    fn an_anchor_respawn_stands_beside_it_facing_it_and_spends_a_charge() {
        let Some(registries) = registries() else {
            return;
        };
        let nether = DimensionRules::load(&registries, Dimension::Nether).unwrap();
        let mut world = Blocks::new(&registries);
        floor(&mut world, 63);
        world.set((0, 64, 0), "minecraft:respawn_anchor[charges=4]");
        let anchor = point(Dimension::Nether, (0, 64, 0), 0.0, false);
        let found = spawn_block(&mut world, &registries, &nether, &anchor, true, true).unwrap();
        assert_eq!(found.feet, [0.5, 64.0, -0.5], "north of it first");
        assert!(found.yaw.abs() < 1.0e-4, "facing south, at the anchor");
        let (pos, state) = found.depleted.unwrap();
        assert_eq!((pos, charges(&registries, state)), ((0, 64, 0), 3));
        let forced = point(Dimension::Nether, (0, 64, 0), 0.0, true);
        assert_eq!(
            spawn_block(&mut world, &registries, &nether, &forced, true, true)
                .unwrap()
                .depleted,
            None,
            "a forced point keeps the charge"
        );
        world.set((0, 64, -1), "minecraft:stone");
        world.set((-1, 64, 0), "minecraft:lava[level=0]");
        let found = spawn_block(&mut world, &registries, &nether, &anchor, true, true).unwrap();
        assert_eq!(
            found.feet,
            [0.5, 64.0, 1.5],
            "past stone and lava to the south"
        );
        assert!((found.yaw + 180.0).abs() < 1.0e-4);
        world.set((0, 64, 0), "minecraft:respawn_anchor[charges=0]");
        assert_eq!(
            spawn_block(&mut world, &registries, &nether, &anchor, true, true),
            None,
            "an empty anchor is no spawn"
        );
        world.set((0, 64, 0), "minecraft:respawn_anchor[charges=2]");
        let overworld = DimensionRules::load(&registries, Dimension::Overworld).unwrap();
        assert_eq!(
            spawn_block(&mut world, &registries, &overworld, &anchor, false, true),
            None,
            "anchors do not work outside the Nether"
        );
    }

    #[test]
    fn an_anchor_walled_in_on_its_level_lets_the_player_up_or_down() {
        let Some(registries) = registries() else {
            return;
        };
        let nether = DimensionRules::load(&registries, Dimension::Nether).unwrap();
        let mut world = Blocks::new(&registries);
        floor(&mut world, 63);
        for &(dx, dz) in &ANCHOR_SIDES {
            world.set((dx, 64, dz), "minecraft:stone");
            world.set((dx, 65, dz), "minecraft:stone");
        }
        world.set((0, 64, 0), "minecraft:respawn_anchor[charges=1]");
        let anchor = point(Dimension::Nether, (0, 64, 0), 0.0, false);
        let found = spawn_block(&mut world, &registries, &nether, &anchor, true, true).unwrap();
        assert_eq!(found.feet, [0.5, 65.0, 0.5], "on top of it, last");
        for &(dx, dz) in &ANCHOR_SIDES {
            world.set((dx, 66, dz), "minecraft:stone");
            world.set((dx, 67, dz), "minecraft:stone");
        }
        world.set((0, 65, 0), "minecraft:stone");
        assert_eq!(
            spawn_block(&mut world, &registries, &nether, &anchor, true, true),
            None
        );
    }

    #[test]
    fn a_bed_respawn_stands_on_the_side_away_from_the_player_s_look_or_around_it() {
        let Some(registries) = registries() else {
            return;
        };
        let overworld = DimensionRules::load(&registries, Dimension::Overworld).unwrap();
        let mut world = Blocks::new(&registries);
        floor(&mut world, 63);
        world.set(
            (0, 64, 0),
            "minecraft:red_bed[facing=north,part=foot,occupied=false]",
        );
        world.set(
            (0, 64, -1),
            "minecraft:red_bed[facing=north,part=head,occupied=false]",
        );
        let facing_east = point(Dimension::Overworld, (0, 64, -1), -90.0, false);
        let found = spawn_block(
            &mut world,
            &registries,
            &overworld,
            &facing_east,
            false,
            true,
        )
        .unwrap();
        assert_eq!(found.feet, [-0.5, 64.0, -0.5], "west of the head");
        assert_eq!(found.depleted, None);
        let facing_west = point(Dimension::Overworld, (0, 64, -1), 90.0, false);
        let found = spawn_block(
            &mut world,
            &registries,
            &overworld,
            &facing_west,
            false,
            true,
        )
        .unwrap();
        assert_eq!(found.feet, [1.5, 64.0, -0.5], "east of the head");
        world.set((1, 64, -1), "minecraft:stone");
        world.set((1, 64, 0), "minecraft:stone");
        world.set((1, 64, 1), "minecraft:stone");
        let found = spawn_block(
            &mut world,
            &registries,
            &overworld,
            &facing_west,
            false,
            true,
        )
        .unwrap();
        assert_eq!(
            found.feet,
            [0.5, 64.0, 1.5],
            "past the foot when the side is walled"
        );
        let nether = DimensionRules::load(&registries, Dimension::Nether).unwrap();
        assert_eq!(
            spawn_block(&mut world, &registries, &nether, &facing_west, true, true),
            None,
            "a bed is no spawn where it cannot set one"
        );
        world.set((0, 64, -1), "minecraft:stone");
        assert_eq!(
            spawn_block(
                &mut world,
                &registries,
                &overworld,
                &facing_west,
                false,
                true
            ),
            None
        );
    }

    #[test]
    fn the_respawn_looks_at_the_block() {
        let near = |feet: [f64; 3], yaw: f32| (look_at_yaw(feet, (0, 64, 0)) - yaw).abs() < 1.0e-4;
        assert!(near([0.5, 64.0, -0.5], 0.0));
        assert!(near([0.5, 64.0, 1.5], -180.0));
        assert!(near([1.5, 64.0, 0.5], 90.0));
        assert!(near([-0.5, 64.0, 0.5], -90.0));
        assert!(near([1.5, 64.0, -0.5], 45.0));
    }
}
