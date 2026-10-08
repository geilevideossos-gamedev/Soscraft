pub const INPUT_COMMAND_NAMES: [&str; 84] = [
    "",
    "+attack",
    "-attack",
    "+melee",
    "-melee",
    "+frag",
    "-frag",
    "+smoke",
    "-smoke",
    "+breath_sprint",
    "-breath_sprint",
    "+usereload",
    "-usereload",
    "+speed_throw",
    "-speed_throw",
    "+actionslot 1",
    "-actionslot 1",
    "+actionslot 2",
    "-actionslot 2",
    "+actionslot 3",
    "-actionslot 3",
    "+actionslot 4",
    "-actionslot 4",
    "+stance",
    "-stance",
    "+gostand",
    "-gostand",
    "+forward",
    "-forward",
    "+back",
    "-back",
    "+moveleft",
    "-moveleft",
    "+moveright",
    "-moveright",
    "+movedown",
    "-movedown",
    "+left",
    "-left",
    "+right",
    "-right",
    "+lookup",
    "-lookup",
    "+lookdown",
    "-lookdown",
    "+strafe",
    "-strafe",
    "+holdbreath",
    "-holdbreath",
    "+activate",
    "-activate",
    "+reload",
    "-reload",
    "+prone",
    "-prone",
    "+mlook",
    "-mlook",
    "+toggleads_throw",
    "-toggleads_throw",
    "+sprint",
    "-sprint",
    "+scores",
    "-scores",
    "+talk",
    "-talk",
    "togglemenu",
    "weapnext",
    "pause",
    "chatmodepublic",
    "chatmodeteam",
    "weapprev",
    "centerview",
    "togglecrouch",
    "toggleprone",
    "goprone",
    "gocrouch",
    "toggleads",
    "leaveads",
    "mc_inventory",
    "mc_drop",
    "mc_swaphands",
    "mc_chat",
    "mc_command",
    "mc_perspective",
];

pub const HOLD_PAIR_LIMIT: u32 = 0x41;

pub const MINECRAFT_COMMAND_FIRST: u32 = 78;

pub const SCRIPT_KEYNUM: i32 = 0x400;

pub fn command_id_from_name(name: &str) -> Option<u32> {
    INPUT_COMMAND_NAMES
        .iter()
        .position(|&s| s == name)
        .map(|i| i as u32)
        .filter(|&id| id != 0)
}

pub fn command_id_lookup(name: &str) -> Option<u32> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut buf = [0u8; 32];
    let n = trimmed.len().min(buf.len());
    for (i, b) in trimmed.as_bytes()[..n].iter().enumerate() {
        buf[i] = b.to_ascii_lowercase();
    }
    let folded = core::str::from_utf8(&buf[..n]).ok()?;
    if let Some(id) = command_id_from_name(folded) {
        return Some(id);
    }
    if folded.starts_with('+') || folded.starts_with('-') {
        return None;
    }
    if n + 1 > buf.len() {
        return None;
    }
    buf.copy_within(..n, 1);
    buf[0] = b'+';
    let plus = core::str::from_utf8(&buf[..n + 1]).ok()?;
    command_id_from_name(plus)
}

pub fn command_name(id: u32) -> Option<&'static str> {
    INPUT_COMMAND_NAMES
        .get(id as usize)
        .copied()
        .filter(|s| !s.is_empty())
}

pub fn is_minecraft_command(id: u32) -> bool {
    id >= MINECRAFT_COMMAND_FIRST && (id as usize) < INPUT_COMMAND_NAMES.len()
}

pub fn key_up_command_id(binding: u32) -> Option<u32> {
    if binding == 0 {
        None
    } else if binding < HOLD_PAIR_LIMIT && binding % 2 == 1 {
        Some(binding + 1)
    } else {
        None
    }
}

pub fn command_names() -> impl Iterator<Item = &'static str> {
    INPUT_COMMAND_NAMES
        .iter()
        .copied()
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod minecraft_command_tests {
    use super::*;
    use crate::{ClientInput, input_cmd};

    #[test]
    fn minecraft_commands_follow_the_mw2_ones() {
        assert_eq!(command_id_lookup("mc_inventory"), Some(78));
        assert_eq!(command_id_lookup("mc_perspective"), Some(83));
        assert_eq!(command_id_lookup("MC_SwapHands"), Some(80));
        assert_eq!(command_name(81), Some("mc_chat"));
        assert!(!is_minecraft_command(77));
        assert!((78..=83).all(is_minecraft_command));
        assert!(!is_minecraft_command(84));
        assert!(!is_minecraft_command(0));
    }

    #[test]
    fn minecraft_commands_do_nothing_to_mw2_input() {
        for id in 78..=83 {
            assert_eq!(key_up_command_id(id), None);
            let mut client = ClientInput::default();
            let before = alloc::format!("{client:?}");
            input_cmd(&mut client, id, 5, 0, 16);
            assert_eq!(alloc::format!("{client:?}"), before);
        }
    }
}
