# Track F: fixes from the interim test of release 1

The owner tested release 1 (4d45e4fe), section A and kit A of `docs/round8/release-1-tests.md`. Screenshots are in `scrennshots/screenshots-4d45e4f/screenshots-4d45e4f/` (untracked, in `C:\Dev\mashup-crafting`); screenshot N is the file starting with N. Track F runs beside track E, from `minecraft-survival`, and is merged before release 2 so its fixes are in that build. More findings from sections B, C and D may follow and are added here.

Working: armour on the MW2 body, eyes of ender, spawner mobs, milk, the End and the Nether, and everything else in kit A.

## F1. The MW2 body and the guns render black
Screenshot 1: third person in the Nether, the MW2 player model and the gun are fully black (the elytra on the back is lit). Find out whether it happens everywhere or only in dark places or certain dimensions, and fix it so the body and guns look like before.

## F2. Spyglass like vanilla
Screenshot 2: the spyglass item icon is wrong. Using it does not zoom. Vanilla: hold right-click to zoom in with the spyglass overlay and its sounds, release to stop.

## F3. Every spawn egg spawns its mob
The cave spider spawn egg does nothing, while the zombie egg works. Check the spawn egg of every mob the game has, and make each one spawn its mob.

## F4. Grenade keys
Final layout: G throws the lethal (frag, semtex and so on), Shift+G throws the tactical (flash, concussion), whatever the player is holding. F only swaps hands. Today no key throws a flash. Both grenade keys can be rebound in the controls menu.

## Rules
One commit per fix, tests, clippy, rustfmt on new files, no code comments, gentle cargo mode, owner steps and a kit line per fix in `docs/round8/track-f-log.md`.
