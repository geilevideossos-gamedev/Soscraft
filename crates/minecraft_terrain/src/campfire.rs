use glam::{DVec3, Mat4, Vec3};
use minecraftoss_player::inventory::ItemStack;

use crate::mesh::{HeldDisplay, HeldItem};
use crate::scene::{Block, BlockPos};

pub const SLOTS: usize = 4;
const COOL_SPEED: u32 = 2;
const ITEM_HEIGHT: f32 = 0.449_218_75;
const ITEM_OFFSET: f32 = 0.3125;
const ITEM_SCALE: f32 = 0.375;
const VIEW_DISTANCE: f64 = 64.0;

pub fn is_campfire(block: &Block) -> bool {
    block.id.namespace == "minecraft"
        && matches!(block.id.path.as_str(), "campfire" | "soul_campfire")
}

pub fn is_lit(block: &Block) -> bool {
    is_campfire(block) && block.properties.get("lit").map(String::as_str) == Some("true")
}

pub fn shown(pos: BlockPos, camera: DVec3) -> bool {
    let centre = DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    );
    centre.distance_squared(camera) < VIEW_DISTANCE * VIEW_DISTANCE
}

fn horizontal_index(facing: &str) -> usize {
    match facing {
        "west" => 1,
        "north" => 2,
        "east" => 3,
        _ => 0,
    }
}

pub fn item_pose(pos: BlockPos, facing: &str, slot: usize) -> Mat4 {
    let direction = (slot + horizontal_index(facing)) % 4;
    let yaw = -(direction as f32 * 90.0);
    let centre = Vec3::new(
        pos.0 as f32 + 0.5,
        pos.1 as f32 + ITEM_HEIGHT,
        pos.2 as f32 + 0.5,
    );
    Mat4::from_translation(centre)
        * Mat4::from_rotation_y(yaw.to_radians())
        * Mat4::from_rotation_x(90f32.to_radians())
        * Mat4::from_translation(Vec3::new(-ITEM_OFFSET, -ITEM_OFFSET, 0.0))
        * Mat4::from_scale(Vec3::splat(ITEM_SCALE))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Campfire {
    pub items: [Option<ItemStack>; SLOTS],
    progress: [u32; SLOTS],
    total: [u32; SLOTS],
}

impl Campfire {
    pub fn place(&mut self, stack: &ItemStack, ticks: u32) -> bool {
        let Some(slot) = self.items.iter().position(Option::is_none) else {
            return false;
        };
        self.items[slot] = Some(ItemStack {
            count: 1,
            ..stack.clone()
        });
        self.progress[slot] = 0;
        self.total[slot] = ticks;
        true
    }

    pub fn is_empty(&self) -> bool {
        self.items.iter().all(Option::is_none)
    }

    pub fn progress(&self, slot: usize) -> u32 {
        self.progress[slot]
    }

    pub fn tick(
        &mut self,
        lit: bool,
        mut cooked: impl FnMut(&ItemStack) -> Option<ItemStack>,
    ) -> Vec<ItemStack> {
        let mut done = Vec::new();
        for slot in 0..SLOTS {
            if !lit {
                if self.progress[slot] > 0 {
                    self.progress[slot] = self.progress[slot]
                        .saturating_sub(COOL_SPEED)
                        .min(self.total[slot]);
                }
                continue;
            }
            let Some(item) = self.items[slot].as_ref() else {
                continue;
            };
            self.progress[slot] += 1;
            if self.progress[slot] >= self.total[slot] {
                done.push(cooked(item).unwrap_or_else(|| item.clone()));
                self.items[slot] = None;
            }
        }
        done
    }

    pub fn take_contents(self) -> Vec<ItemStack> {
        self.items.into_iter().flatten().collect()
    }

    pub fn held_items(&self, pos: BlockPos, facing: &str) -> Vec<HeldItem> {
        let light = Vec3::new(pos.0 as f32 + 0.5, pos.1 as f32 + 0.5, pos.2 as f32 + 0.5);
        self.items
            .iter()
            .enumerate()
            .filter_map(|(slot, item)| {
                Some(HeldItem {
                    pose: item_pose(pos, facing, slot),
                    light,
                    id: item.as_ref()?.id.clone(),
                    display: HeldDisplay::Fixed,
                    first_tint: None,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEEF: &str = "minecraft:beef";
    const COOKED_BEEF: &str = "minecraft:cooked_beef";

    fn cooked(item: &ItemStack) -> Option<ItemStack> {
        (item.id == BEEF).then(|| ItemStack::new(COOKED_BEEF, 1))
    }

    fn run(campfire: &mut Campfire, lit: bool, ticks: u32) -> Vec<ItemStack> {
        (0..ticks)
            .flat_map(|_| campfire.tick(lit, cooked))
            .collect()
    }

    fn fixed_item_centre(pose: Mat4) -> Vec3 {
        let fixed =
            Mat4::from_rotation_y(180f32.to_radians()) * Mat4::from_translation(Vec3::splat(-0.5));
        (pose * fixed).transform_point3(Vec3::splat(0.5))
    }

    #[test]
    fn campfires_and_their_flames_come_from_their_blocks() {
        assert!(is_campfire(&Block::new("minecraft:campfire")));
        assert!(is_campfire(&Block::new("minecraft:soul_campfire")));
        assert!(!is_campfire(&Block::new("minecraft:furnace")));
        assert!(is_lit(
            &Block::new("minecraft:campfire").with("lit", "true")
        ));
        assert!(!is_lit(
            &Block::new("minecraft:campfire").with("lit", "false")
        ));
        assert!(!is_lit(&Block::new("minecraft:candle").with("lit", "true")));
    }

    #[test]
    fn food_takes_the_first_free_of_four_slots_one_at_a_time() {
        let mut campfire = Campfire::default();
        let mut stack = ItemStack::new(BEEF, 5);
        stack.components = Some(serde_json::json!({"minecraft:custom_name": "Dinner"}));
        for _ in 0..SLOTS {
            assert!(campfire.place(&stack, 600));
        }
        assert!(!campfire.place(&stack, 600));
        assert!(
            campfire
                .items
                .iter()
                .flatten()
                .all(|item| item.count == 1 && item.components == stack.components)
        );
        campfire.items[1] = None;
        assert!(campfire.place(&ItemStack::new("minecraft:cod", 1), 600));
        assert_eq!(
            campfire.items[1].as_ref().map(|i| i.id.as_str()),
            Some("minecraft:cod")
        );
    }

    #[test]
    fn a_lit_campfire_pops_the_cooked_item_on_the_last_tick() {
        let mut campfire = Campfire::default();
        campfire.place(&ItemStack::new(BEEF, 1), 600);
        assert!(run(&mut campfire, true, 599).is_empty());
        let popped = run(&mut campfire, true, 1);
        assert_eq!(popped, vec![ItemStack::new(COOKED_BEEF, 1)]);
        assert!(campfire.is_empty());
    }

    #[test]
    fn an_item_without_a_recipe_at_the_end_pops_out_raw() {
        let mut campfire = Campfire::default();
        let stick = ItemStack::new("minecraft:stick", 1);
        campfire.place(&stick, 3);
        assert_eq!(run(&mut campfire, true, 3), vec![stick]);
    }

    #[test]
    fn an_unlit_campfire_keeps_its_food_and_cools_two_per_tick() {
        let mut campfire = Campfire::default();
        campfire.place(&ItemStack::new(BEEF, 1), 600);
        assert!(run(&mut campfire, false, 1000).is_empty());
        assert_eq!(campfire.progress(0), 0);
        assert!(run(&mut campfire, true, 300).is_empty());
        assert_eq!(campfire.progress(0), 300);
        assert!(run(&mut campfire, false, 100).is_empty());
        assert_eq!(campfire.progress(0), 100);
        assert!(run(&mut campfire, true, 499).is_empty());
        assert_eq!(run(&mut campfire, true, 1).len(), 1);
    }

    #[test]
    fn breaking_takes_the_food_still_on_it() {
        let mut campfire = Campfire::default();
        campfire.place(&ItemStack::new(BEEF, 1), 600);
        campfire.place(&ItemStack::new("minecraft:kelp", 1), 600);
        let ids: Vec<_> = campfire.take_contents().into_iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![BEEF, "minecraft:kelp"]);
    }

    #[test]
    fn items_lie_flat_on_the_four_corners_like_the_smoke() {
        let origin = (10, 64, -3);
        let at = |x: f32, z: f32| Vec3::new(10.0 + x, 64.0 + ITEM_HEIGHT, -3.0 + z);
        let spots = [
            at(0.1875, 0.1875),
            at(0.8125, 0.1875),
            at(0.8125, 0.8125),
            at(0.1875, 0.8125),
        ];
        for (slot, spot) in spots.iter().enumerate() {
            let centre = fixed_item_centre(item_pose(origin, "south", slot));
            assert!(centre.abs_diff_eq(*spot, 1e-4), "slot {slot}: {centre}");
        }
        let east = fixed_item_centre(item_pose(origin, "east", 0));
        assert!(east.abs_diff_eq(spots[3], 1e-4), "{east}");
        let north = fixed_item_centre(item_pose(origin, "north", 1));
        assert!(north.abs_diff_eq(spots[3], 1e-4), "{north}");
        let thickness = item_pose(origin, "west", 2).transform_vector3(Vec3::Z);
        assert!(
            thickness.normalize().abs_diff_eq(Vec3::NEG_Y, 1e-5),
            "{thickness}"
        );
        assert!((thickness.length() - ITEM_SCALE).abs() < 1e-6);
    }

    #[test]
    fn only_slots_with_food_are_drawn_in_the_fixed_display() {
        let mut campfire = Campfire::default();
        campfire.place(&ItemStack::new(BEEF, 1), 600);
        campfire.place(&ItemStack::new("minecraft:cod", 1), 600);
        campfire.items[0] = None;
        let held = campfire.held_items((0, 0, 0), "south");
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].id, "minecraft:cod");
        assert_eq!(held[0].display, HeldDisplay::Fixed);
        assert_eq!(held[0].pose, item_pose((0, 0, 0), "south", 1));
        assert_eq!(held[0].light, Vec3::splat(0.5));
    }

    #[test]
    fn campfire_items_are_drawn_within_sixty_four_blocks() {
        assert!(shown((0, 0, 0), DVec3::new(63.0, 0.5, 0.5)));
        assert!(!shown((0, 0, 0), DVec3::new(64.5, 0.5, 0.5)));
    }

    #[test]
    fn vanilla_raw_beef_lies_flat_at_the_slot_spot() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = crate::pack::PackStack::open(vec![pack]).unwrap();
        let beef = crate::pack::ResourceId::parse(BEEF).unwrap();
        let fixed = crate::model::item_display_transform(&packs, &beef, "fixed").unwrap();
        let pose = item_pose((0, 0, 0), "south", 2) * fixed;
        let centre = pose.transform_point3(Vec3::splat(0.5));
        assert!(
            centre.abs_diff_eq(Vec3::new(0.8125, ITEM_HEIGHT, 0.8125), 1e-4),
            "{centre}"
        );
        let up = pose.transform_vector3(Vec3::Z).normalize();
        assert!(up.y.abs() > 0.9999, "{up}");
    }
}
