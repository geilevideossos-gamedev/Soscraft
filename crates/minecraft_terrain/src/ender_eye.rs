use crate::bucket::{Change, ItemUse};
use crate::server::ItemOnBlock;
use minecraftoss_generator::feature::end_portal::FRAME;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::{Block, Player, World};

pub const ENDER_EYE: &str = "minecraft:ender_eye";
const USE_REACH: f64 = 4.5;

pub fn takes_eye(block: &Block) -> bool {
    block.id == FRAME && block.property("eye") == Some("false")
}

pub fn eye_use(player: &Player, world: &impl World, held: &ItemStack) -> Option<ItemUse> {
    if held.id != ENDER_EYE {
        return None;
    }
    let hit = player.target(world, USE_REACH)?;
    if !world.block(hit.pos).as_ref().is_some_and(takes_eye) {
        return None;
    }
    Some(ItemUse {
        change: Change::Server(hit.pos, ItemOnBlock::EnderEye),
        result: None,
        wear: 0,
        consumed: true,
        sounds: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use minecraftoss_player::Pos;
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

    fn frame(eye: &str) -> Block {
        Block::new(FRAME).with("facing", "north").with("eye", eye)
    }

    fn below(block: Block) -> Blocks {
        let mut world = Blocks::default();
        world.set_block((0, 0, 0), Some(block));
        world
    }

    fn looking_down() -> Player {
        let mut player = Player::new(DVec3::new(0.5, 1.0, 0.5));
        player.pitch = 90.0;
        player
    }

    #[test]
    fn an_eye_on_an_empty_frame_asks_the_server_and_is_spent() {
        let eye = ItemStack::new(ENDER_EYE, 3);
        let used =
            eye_use(&looking_down(), &below(frame("false")), &eye).expect("aimed at the frame");
        assert_eq!(
            used.change,
            Change::Server((0, 0, 0), ItemOnBlock::EnderEye)
        );
        assert!(used.consumed && used.result.is_none() && used.wear == 0);
        assert!(used.sounds.is_empty(), "the server plays the fill sound");
    }

    #[test]
    fn a_filled_frame_or_another_block_passes_the_eye_on() {
        let eye = ItemStack::new(ENDER_EYE, 3);
        assert_eq!(eye_use(&looking_down(), &below(frame("true")), &eye), None);
        assert_eq!(
            eye_use(&looking_down(), &below(Block::new("minecraft:stone")), &eye),
            None
        );
        assert_eq!(eye_use(&looking_down(), &Blocks::default(), &eye), None);
        let pearl = ItemStack::new("minecraft:ender_pearl", 3);
        assert_eq!(
            eye_use(&looking_down(), &below(frame("false")), &pearl),
            None
        );
    }
}
