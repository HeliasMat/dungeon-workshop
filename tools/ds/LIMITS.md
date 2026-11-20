# How far a film goes on DS

What Explorers of Sky's own script engine can show of a Dungeon Workshop
film, what needs code (ASM), and what each tool would unlock. Everything
marked **seen** was checked in the original game in the emulator
(`tools/ds/boot_and_watch.py`, contact sheets under `target/ds/`).

## Watching the film

Three ways were weighed:

| Way | Verdict |
|---|---|
| Play the film when a new game starts | **Chosen** (`patch_rom.py --boot`). The new game's first event, `EVENT_M00A_01` in `SCRIPT/COMMON/unionall.ssb`, plays the personality quiz as an acting scene; the film's scenes are played just before it with `supervision_ExecuteActingSub`, as the story chains its scenes. Script only, no code. From power on with no save file: A on the title, A on the controls screen, and the film plays (about 20 s after power on). The game then goes on to the quiz as usual. **Seen.** |
| A debug menu entry | `unionall` has `DEBUG_SCENE` and `DEBUG_SCRIPT` (menus `MENU_SCENE_DEBUG`, `MENU_SCRIPT_DEBUG`), but nothing reaches them on a retail cartridge: a script would have to jump to them, which is no shorter than the chosen way, and the menus list the story's scenes, not ours. |
| Hook the title screen or "Continue" | The title menu is code (overlay), not script: needs ASM. "Continue" resumes the saved event, so it only works for a save made on the patched ROM. |

With a save file on the cartridge, the player picks "New Game" on the
title menu (the old adventure is deleted on confirming, as in the game).

`unionall` is decompiled and compiled back as SkyTemple's script editor
does; the result is not byte-identical (the compiler lays branches out
anew) but plays: the story's opening and quiz were seen to run after the
film.

## Several scenes

Each `// ds-export level` line of the export starts a scene; each scene
becomes an acting scene of its own (`SCRIPT/<place>/dwscen01.ssa/.ssb`,
`dwscen02`, ...), files added to the ROM next to the story's (none
replaced), and the event plays them in turn. Places may differ. **Seen**:
two scenes in two places, and five scenes in a row.

For the exporter: write one block per film scene, each with its own
`// ds-export level`, its `// ds-export actor` lines, then its
`def 0 { ... end; }`.

With `<MAP/scene>` (replacing a story scene), the first scene replaces it
and the others are inserted after the line playing it in `unionall`. Not
seen in game: a savestate holds the old `unionall` in memory, and reaching
chapter 2 without one is a whole playthrough.

## Actors

- **45 free actors** without code: the 10 test actors (placed only in the
  debug map `T00P01`) and 35 story NPC actors of a fixed species that no
  layout of the ROM places and no script names (`FREE_ACTORS` in
  `patch_rom.py`). Their species is changed in the actor table (2 bytes
  each in arm9). A Pokémon the game has an actor for keeps it (Krabby is
  `NPC_KURABU`). The exporter may name the free ones with any placeholder
  (`ACTOR_FREE07`); `patch_rom.py` gives each a free actor and renames it.
  Limit: 45 Pokémon without an actor of their own per film (one species per
  actor for the whole film).
- **Per scene: 24 actors at once** (`ground_mode.h`: `struct live_actor
  actors[24]`). **Seen**: 24 different species in one scene. One run with
  20 actors *and* a ground drawn on the top screen hung: keep the top
  screen empty in crowd scenes.
- **What armips unlocks**: SkyTemple's `ActorAndLevelLoader` (psy_commando's
  `pmd2_asm_mods/src/eu/actorlistloader_arm9.asm` and
  `..._overlay11.asm`, about 450 lines of ARM assembly patching a dozen call
  sites) moves the actor table to `BALANCE/actor_list.bin`, so actors can be
  added without limit, and the level table to `BALANCE/level_list.bin`, so
  new places (levels) can be added. Writing its machine code by hand is
  possible in principle but means writing an ARM assembler for those files:
  not done. With armips installed, `skytemple_files`' patcher applies it as
  is.

## The 12 words judged impossible

| Word | Verdict | How (script operations, as Sky's scripts use them) |
|---|---|---|
| crowd | **Fake works** (seen) | Up to 24 actors in a scene; free actors give the species. `ExecuteCommon<actor X>(CORO_JUMP_HAPPY_FUNC_SERIES, 0)` on each without waiting makes the whole crowd cheer at once. |
| follow | **Fake works** (seen) | `MovePositionMark<actor A>(1, mark); Wait(16); MovePositionLives<actor B>(1, ACTOR_A);` B walks to where A is (actions given to an actor run alongside the script; `WaitExecuteLives(ACTOR_B)` waits). A longer walk: a few legs. |
| chase | **Fake works** (seen) | The same with speed 3 for both (`MovePositionMark<actor A>(3, …)`, `MovePositionLives<actor B>(3, ACTOR_A)`). |
| wander | **Fake works** (seen) | `ExecuteCommon<actor X>(CORO_MOVE_RANDOM, 0)`: Sky's own random stroll (turn, step, wait 48 frames, for ever, until the scene ends or another action is given). |
| daytime | **Fake works** (seen) | Evening: `screen_FlushOut(1, 30, 4, 255, 128, 64)`; night: `screen_FlushOut(1, 30, 6, 0, 32, 128)` (dims to a blue dusk); back to day: `screen_FlushIn` with the same numbers. The tint stays until then. Mode 3 fills the screen with the colour and `FlushIn` did not undo it: avoid. Sky also has evening variants of some places as levels of their own (`LEVEL_D01P11B` is the beach at sunset): the exporter can pick the level when the film's light says evening. |
| status | **Fake works** (seen) | Effects on the actor: `EFFECT_CONFUSE` (stars), `EFFECT_POISION_BUBBLE` (sic), `EFFECT_GIANT_Z` (asleep), `EFFECT_TRAPPED_IN_ICE` (frozen), `EFFECT_STATS_DOWN_EFFECT`, `EFFECT_STATS_UP_EFFECT` and `EFFECT_YAWN` (not tried); with poses below (asleep: animation 25 or 33; shivering: 27 or 47). |
| attack | **Fake works** (parts seen) | Wind-up `SetAnimation<actor A>(28)`, strike `SetAnimation<actor A>(48)` (a lunge), then on the target an effect: `EFFECT_PUNCH_ATTACK`, `EFFECT_LIGHTNING_STRIKE` (a bolt from the sky), `EFFECT_FIRE_ATTACK` (a tall flame), `EFFECT_SLASH_ATTACK`, `EFFECT_EXPLOSION_SMALL` (seen; `EFFECT_WATER_EXPLOSION`, `EFFECT_AIR_BLADES` and others of the table not tried); the target's hurt pose `SetAnimation(22)` or `(36)`; a shake `camera_SetEffect(2, 1, 1.0)` ... `camera_SetEffect(0, 0, 0)`; back to standing `SetAnimation(2)`. Real move animations (Thunderbolt as in a dungeon) need code: the dungeon's effect system is not loaded in ground mode. |
| fight | **Fake works** | Attacks traded (above), with steps back (`MovePositionOffset`) and the loser's faint: `SetAnimation(34)` (falls) then `(35)` (lies). |
| hold | **Partly** | No generic held-item sprite in ground mode. `EFFECT_APPLE_ON_HEAD` shows an apple above the actor (seen); `EFFECT_EATING` for eating (seen). Any other item needs a sprite (below). |
| give | **Partly** | Walk to the other (`MovePositionLives`), the apple effect passes from one to the other, the jingle (`me_Play`) and a line. |
| drop | **Needs a sprite** | Objects (`<object>` in a layout) are the only free-standing sprites, and Sky's are place-specific props (`OBJECT_D01P11B2_12`...). |
| throw | **Needs a sprite** | Same; once an item object exists: `MoveHeight` and `SlidePositionOffset<object>` make the arc. |

### Item sprites, without armips

An object entry is a sprite file (`.wan`) in `GROUND/`. Repurposing an
object no scene uses, with its `.wan` replaced by one made from an item
picture (`skytemple_files.graphics.chara_wan` imports sprite sheets), would
give "an item" that can be placed, moved and lifted. Not done: it means
choosing the object, drawing the sheet and checking the game loads it in
any place.

## Poses (`SetAnimation<actor X>(n)`)

Seen on Pikachu, facing down. The numbers are the game's ground animation
ids, not the sprite's groups; 50 to 62 repeat 30 to 42.

| n | pose | n | pose |
|---|---|---|---|
| 1, 4 | hop | 28 | crouch (wind-up) |
| 2 | stand (back to normal) | 30, 24, 26, 66 | face front, arms up (cheer) |
| 8, 11, 14, 39 | strain (lines above the head) | 32 | bow, head down |
| 22, 29, 36, 42, 49, 76 | knocked over (hurt) | 33, 25 | curled up (asleep) |
| 27, 47 | shiver (sweat drops) | 34 | falls down (faint) |
| 38 | jump in the air | 35 | lying on the ground |
| 45 | spin | 48 | lunge to the side (strike) |
| 68 | lying flat, facing sideways | 40, 41, 43, 46 | look aside / around |

## The top screen

Sky's scenes fill it with a place: `back2_SetMode(4);
back2_SetGround(LEVEL_…); camera2_SetPositionMark(Position<'m', x, y>);
screen2_FadeIn(1, 30);` (and `screen2_FadeOut` at the end). Seen working;
a place with animated parts (the guild's torches) showed blank squares in
their place, so a still place reads better.

## Sound and video

`record.py` writes silent mp4s: py-desmume's library exports no audio
samples (only `desmume_volume_get/set`; its sound goes straight to the
SDL device). Sound would need a DeSmuME build with WAV recording (the
desktop DeSmuME's "Record WAV" menu) or the music rendered from the ROM's
SMD/SWD files.

## What code (ASM) would add

Needs armips (SkyTemple's patches) or an ARM gcc + make (c-of-time):

- more actors and new places (ActorAndLevelLoader, above);
- new script operations through c-of-time's custom `ProcessSpecial`s:
  e.g. a dungeon move's animation in a cutscene, items drawn by the item
  code, a custom title-screen entry;
- a title-screen hook ("Watch the film" next to "New Game").
