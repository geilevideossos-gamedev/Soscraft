# Track A log: controls and chat

Branch `track-a`, worktree `C:\Dev\mc-wt\track-a`. Briefs: `a1-grenade-keys.md`, `a2-chat.md`.

A commit can't name its own hash, so each entry is written with its commit and its hash is filled in by the next commit; the last entry's hash is in `git log` under its subject.

## Group: grenade keys

### 1. Minecraft keys never take a grenade key outside a Minecraft screen

Commit: (filled in by the next commit)

- **Changed:** `console/src/binds.rs minecraft_presses`. The old rule let the swap-hands key keep its MW2 binding only while a gun was selected (`holding_item`), so with a Minecraft item or an empty hand F swapped hands instead of throwing. Now any Minecraft key that is also bound to `+frag` or `+smoke` in the MW2 layer keeps its throw while no Minecraft screen is open, whatever is in hand. Inside a screen the Minecraft action still wins (F swaps the hovered slot like vanilla). `holding_item` is gone from `MinecraftKeyState` and from `console/src/plugin/mod.rs`.
- **Tests:** `a_grenade_key_keeps_its_throw_outside_minecraft_screens` replaces `swap_hands_takes_its_key_only_while_a_minecraft_item_is_in_hand`; `a_rebound_swap_hands_key_follows_the_same_rule` and `only_just_pressed_keys_fire_their_action` updated.
- **Decision:** the throw wins over every Minecraft action on the same key, not only swap hands, because the owner wants the grenade to work whatever is held. A key bound only to a Minecraft action still takes the key as before.
- **Test steps** (pick a class with Frag and Flash):
  1. Select cobblestone and press G: a frag flies, nothing is placed or swapped.
  2. Select an empty slot and press G: a frag flies.
  3. Open the inventory (E), hover the cobblestone and press F: it goes to the offhand. Press G: nothing is thrown while the inventory is open.
- **Kit:**

```
IW4L_MINECRAFT_KIT=minecraft:cobblestone*64
```
