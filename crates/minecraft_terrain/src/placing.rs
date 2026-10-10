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

    fn sturdy_face(&self, pos: Pos, face: minecraftoss_player::Face) -> bool {
        match self.staged(pos) {
            Some(block) => block.as_ref().is_some_and(|block| {
                minecraftoss_player::authored_sturdy_face(block, &self.collision_boxes(pos), face)
            }),
            None => self.world.sturdy_face(pos, face),
        }
    }

    fn sturdy_center(&self, pos: Pos, face: minecraftoss_player::Face) -> bool {
        match self.staged(pos) {
            Some(block) => block.as_ref().is_some_and(|block| {
                minecraftoss_player::authored_sturdy_center(block, &self.collision_boxes(pos), face)
            }),
            None => self.world.sturdy_center(pos, face),
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

pub struct SurviveView<'a> {
    scene: &'a crate::scene::HandcraftedScene,
    light: Option<&'a crate::lighting::SkyLight>,
    random: minecraftoss_core::random::AnyRandom,
}

impl<'a> SurviveView<'a> {
    pub fn new(
        scene: &'a crate::scene::HandcraftedScene,
        light: Option<&'a crate::lighting::SkyLight>,
    ) -> Self {
        Self {
            scene,
            light,
            random: minecraftoss_core::random::AnyRandom::new(true, 0),
        }
    }
}

pub fn raw_brightness(sky: u8, block: u8, darkening: i32) -> (i32, bool) {
    (
        (i32::from(sky) - darkening).max(i32::from(block)),
        sky >= 15,
    )
}

impl minecraftoss_generator::feature::World for SurviveView<'_> {
    fn block_at(&self, x: i32, y: i32, z: i32) -> minecraftoss_core::BlockStateId {
        self.scene
            .state_at((x, y, z))
            .unwrap_or(minecraftoss_core::BlockStateId::AIR)
    }

    fn set_block_with_flags(
        &mut self,
        _lib: &minecraftoss_generator::feature::Library,
        _pos: minecraftoss_core::BlockPos,
        _state: minecraftoss_core::BlockStateId,
        _flags: u32,
    ) -> bool {
        false
    }

    fn schedule(&mut self, _pos: minecraftoss_core::BlockPos, _fluid: bool, _delay: i32) {}

    fn random(&mut self) -> &mut minecraftoss_core::random::AnyRandom {
        &mut self.random
    }

    fn light(&self, pos: minecraftoss_core::BlockPos, darkening: i32) -> Option<(i32, bool)> {
        let at = (pos.x, pos.y, pos.z);
        self.light
            .map(|light| raw_brightness(light.get(at), light.get_block(at), darkening))
    }

    fn world_seed(&self) -> i64 {
        0
    }

    fn min_y(&self) -> i32 {
        crate::scene::Scene::vertical_range(self.scene).start
    }

    fn max_y(&self) -> i32 {
        crate::scene::Scene::vertical_range(self.scene).end - 1
    }

    fn height_at(&self, _kind: minecraftoss_core::chunk::HeightmapKind, _x: i32, _z: i32) -> i32 {
        minecraftoss_generator::feature::World::min_y(self)
    }
}

pub fn may_place(
    library: &minecraftoss_generator::feature::Library,
    view: &SurviveView<'_>,
    pos: Pos,
    block: &crate::scene::Block,
) -> bool {
    if block.id.key() == crate::scaffolding::SCAFFOLDING {
        return true;
    }
    match view.scene.states().and_then(|states| states.state_of(block)) {
        Some(state) => library
            .survival
            .can_survive(&library.registries, view, state, pos),
        None => true,
    }
}

pub fn base_if_covered(
    scene: &crate::scene::HandcraftedScene,
    pos: Pos,
    block: crate::scene::Block,
) -> crate::scene::Block {
    let key = block.id.key();
    let farmland = key == "minecraft:farmland";
    if !farmland && key != "minecraft:dirt_path" {
        return block;
    }
    let Some(states) = scene.states() else {
        return block;
    };
    let registries = states.registries();
    let blocks = &registries.blocks;
    let above = crate::scene::Scene::block(scene, (pos.0, pos.1 + 1, pos.2))
        .and_then(|above| states.state_of(above));
    let covered = above.is_some_and(|state| {
        let keeps = if farmland {
            registries
                .block_tags
                .id("minecraft:maintains_farmland")
                .is_some_and(|tag| registries.block_in_tag(state, tag))
        } else {
            blocks.block(blocks.block_of(state)).is_a("FenceGateBlock")
        };
        blocks.is(state, minecraftoss_core::block::flags::LEGACY_SOLID) && !keeps
    });
    if covered {
        crate::scene::Block::new("minecraft:dirt")
    } else {
        block
    }
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

    #[test]
    fn staged_torches_have_no_collision_and_staged_tops_hold_them_by_the_centre() {
        let mut world = Blocks::default();
        world.set_block((0, 0, 0), Some(Block::new("minecraft:oak_leaves")));
        let mut staged = Staged::new(&world);
        staged.set_block(
            (0, 1, 0),
            Some(Block::new("minecraft:wall_torch").with("facing", "south")),
        );
        staged.set_block(
            (1, 0, 0),
            Some(Block::new("minecraft:stone_slab").with("type", "top")),
        );
        staged.set_block(
            (2, 0, 0),
            Some(Block::new("minecraft:stone_slab").with("type", "bottom")),
        );
        assert!(staged.collision_boxes((0, 1, 0)).is_empty());
        let up = minecraftoss_player::Face::Up;
        assert!(!staged.sturdy_center((0, 0, 0), up));
        assert!(staged.sturdy_center((1, 0, 0), up));
        assert!(!staged.sturdy_center((2, 0, 0), up));
        assert!(!staged.sturdy_center((0, 1, 0), up));
    }

    #[test]
    fn farmland_and_paths_placed_under_a_solid_block_become_dirt() {
        use crate::scene::Block as SceneBlock;
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        let states = crate::terrain::BlockStates::new(std::sync::Arc::new(registries), 0, -64, 384)
            .unwrap();
        let scene = crate::scene::HandcraftedScene::streamed(std::sync::Arc::new(states));
        let placed = |scene: &crate::scene::HandcraftedScene, id: &str, above: Option<&str>| {
            let mut scene = scene.clone();
            scene.set((0, 11, 0), above.map(SceneBlock::new));
            base_if_covered(&scene, (0, 10, 0), SceneBlock::new(id))
                .id
                .key()
        };
        for id in ["minecraft:farmland", "minecraft:dirt_path"] {
            assert_eq!(placed(&scene, id, None), id);
            assert_eq!(placed(&scene, id, Some("minecraft:torch")), id);
            assert_eq!(placed(&scene, id, Some("minecraft:stone")), "minecraft:dirt");
            assert_eq!(placed(&scene, id, Some("minecraft:oak_sign")), "minecraft:dirt");
            assert_eq!(placed(&scene, id, Some("minecraft:oak_fence_gate")), id);
        }
        assert_eq!(placed(&scene, "minecraft:farmland", Some("minecraft:wheat")), "minecraft:farmland");
        assert_eq!(placed(&scene, "minecraft:stone", Some("minecraft:stone")), "minecraft:stone");
    }

    #[test]
    fn survival_light_is_the_brighter_of_sky_and_block_and_only_full_sky_counts_as_open() {
        assert_eq!(raw_brightness(15, 3, 0), (15, true));
        assert_eq!(raw_brightness(4, 9, 0), (9, false));
        assert_eq!(raw_brightness(15, 3, 11), (4, true));
        assert_eq!(raw_brightness(14, 0, 0), (14, false));
    }

    #[test]
    fn the_survival_view_reads_the_client_light_and_answers_none_without_it() {
        use minecraftoss_generator::feature::World as _;
        let scene = crate::scene::HandcraftedScene::default();
        let light = crate::lighting::SkyLight::from_levels(
            (0, 0, 0),
            (1, 2, 1),
            vec![15, 4],
            vec![3, 9],
        );
        let lit = SurviveView::new(&scene, Some(&light));
        let at = |y| minecraftoss_core::BlockPos::new(0, y, 0);
        assert_eq!(lit.light(at(0), 0), Some((15, true)));
        assert_eq!(lit.light(at(1), 0), Some((9, false)));
        assert_eq!(SurviveView::new(&scene, None).light(at(0), 0), None);
        assert_eq!(lit.block_at(0, 0, 0), minecraftoss_core::BlockStateId::AIR);
    }
}
