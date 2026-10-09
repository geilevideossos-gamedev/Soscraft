use minecraftoss_player::inventory::ItemStack;

pub const WEAPON_PREFIX: &str = "iw4:weapon/";
pub const HOTBAR: usize = 9;

pub fn weapon_of(stack: &ItemStack) -> Option<u32> {
    stack.id.strip_prefix(WEAPON_PREFIX)?.parse().ok()
}

pub fn weapon_item(weapon: u32) -> ItemStack {
    let mut stack = ItemStack::new(format!("{WEAPON_PREFIX}{weapon}"), 1);
    stack.max = 1;
    stack
}
