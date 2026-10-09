use crate::bucket::{Change, ItemUse};
use crate::server::ItemOnBlock;
use glam::DVec3;
use minecraftoss_generator::feature::end_portal::FRAME;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::{Block, Player, World};

pub const ENDER_EYE: &str = "minecraft:ender_eye";
const USE_REACH: f64 = 4.5;

#[derive(Clone, Debug, PartialEq)]
pub struct EyeThrow {
    pub feet: DVec3,
    pub middle: DVec3,
    pub components: Option<String>,
}

pub fn body_height(player: &Player) -> f64 {
    if player.swimming {
        0.6
    } else if player.crouching {
        1.5
    } else {
        1.8
    }
}

pub fn eye_throw(player: &Player, held: &ItemStack) -> EyeThrow {
    EyeThrow {
        feet: player.pos,
        middle: player.pos + DVec3::Y * (body_height(player) * 0.5),
        components: held.components.as_ref().map(ToString::to_string),
    }
}

pub fn takes_eye(block: &Block) -> bool {
    block.id == FRAME && block.property("eye") == Some("false")
}

pub fn eye_use(player: &Player, world: &impl World, held: &ItemStack) -> Option<ItemUse> {
    if held.id != ENDER_EYE {
        return None;
    }
    let aimed = player
        .target(world, USE_REACH)
        .and_then(|hit| Some((hit.pos, world.block(hit.pos)?)));
    let change = match aimed {
        Some((pos, block)) if takes_eye(&block) => Change::Server(pos, ItemOnBlock::EnderEye),
        Some((_, block)) if block.id == FRAME => return None,
        _ => Change::ThrowEye(eye_throw(player, held)),
    };
    Some(ItemUse {
        change,
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
    fn a_filled_frame_passes_the_eye_on() {
        let eye = ItemStack::new(ENDER_EYE, 3);
        assert_eq!(eye_use(&looking_down(), &below(frame("true")), &eye), None);
        let pearl = ItemStack::new("minecraft:ender_pearl", 3);
        assert_eq!(
            eye_use(&looking_down(), &below(frame("false")), &pearl),
            None
        );
    }

    #[test]
    fn anywhere_else_the_eye_is_thrown_from_the_middle_of_the_body() {
        let mut eye = ItemStack::new(ENDER_EYE, 3);
        let thrown = |player: &Player, world: &Blocks, eye: &ItemStack| {
            let used = eye_use(player, world, eye).expect("the eye is used");
            assert!(used.consumed && used.result.is_none() && used.wear == 0);
            assert!(used.sounds.is_empty(), "the server plays the launch sound");
            match used.change {
                Change::ThrowEye(throw) => throw,
                other => panic!("{other:?}"),
            }
        };
        let standing = looking_down();
        let on_stone = thrown(&standing, &below(Block::new("minecraft:stone")), &eye);
        assert_eq!(on_stone.feet, DVec3::new(0.5, 1.0, 0.5));
        assert_eq!(on_stone.middle, DVec3::new(0.5, 1.9, 0.5));
        assert_eq!(on_stone.components, None);
        let mut crouching = looking_down();
        crouching.crouching = true;
        let in_the_air = thrown(&crouching, &Blocks::default(), &eye);
        assert_eq!(in_the_air.middle, DVec3::new(0.5, 1.75, 0.5));
        eye.components = Some(serde_json::json!({"minecraft:custom_name": "Seeker"}));
        let named = thrown(&standing, &Blocks::default(), &eye);
        assert_eq!(
            named.components.as_deref(),
            Some(r#"{"minecraft:custom_name":"Seeker"}"#)
        );
    }
}
