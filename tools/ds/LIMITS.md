# How far a film goes on DS

What Explorers of Sky's own script engine can show of a Dungeon Workshop
film, what needs code (ASM), and what each tool would unlock. Everything
marked **seen** was checked in the original game in the emulator
(`tools/ds/boot_and_watch.py`, contact sheets under `target/ds/`).

## Watching the film

In the Workshop: the Emulator space's **Play This Film on DS** exports the film open, patches it in with `--boot --more-actors` and plays it from power on in a libretro core of the user's (melonDS); how to get one: `docs/DS.md`.

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

- **45 free actors** without code (any number with `--more-actors`, below): the 10 test actors (placed only in the
  debug map `T00P01`) and 35 story NPC actors of a fixed species that no
  layout of the ROM places and no script names (`FREE_ACTORS` in
  `patch_rom.py`). Their species is changed in the actor table (2 bytes
  each in arm9). A Pokémon the game has an actor for keeps it (Krabby is
  `NPC_KURABU`). The exporter may name the free ones with any placeholder
  (`ACTOR_FREE07`); `patch_rom.py` gives each a free actor and renames it.
  Limit: 45 Pokémon without an actor of their own per film (one species per
  actor for the whole film).
- **Per scene: 21 actors at once**, not 24. The ground state has 24 live
  actors (`ground_mode.h`: `struct live_actor actors[24]`, allocated on
  the heap: `MemAlloc(0x3780)` in
  `AllocAndInitPartnerFollowDataAndLiveActorList`), but slots 0-2 are kept
  for the hero, the partner and a third party member: the loader
  (pret/pmd-sky `ov11_022F7E78`, NA) puts those three in their own slot and
  looks for a free one for anyone else **from the fourth** (`mov r1, #3`).
  **Seen** (reading the list in memory, `target/ds/labtools/probe.py`): a
  layout of 24 loads its first 21, the last 3 silently not; with 21 loaded,
  `supervision_Acting(1)` loads none of layer 1's. Objects (16) and
  performers (16, the camera and the weather are performers) have pools of
  their own and don't count. One run with 20 actors *and* a ground drawn on
  the top screen once hung (not again with 24 and the tall view).
- **More over a scene's course** (seen): `Destroy<actor X>()` (Sky's scenes
  put away who is done with it) frees a slot, and `supervision_Acting(n)`
  loads layer `n` of the layout into the free slots, mid-scene: two
  destroyed, two of layer 1 shown in their place, on the beach. The
  exporter does it (`Writer::fit`): in a scene over 21, who comes in later
  is in a layer of its own, loaded on its first line, after destroying who
  has left (hidden for good); seen: 22 on the beach, two walk out, two walk
  in, one of them speaks (`target/ds/lab/room_s.png`). Too many from the
  start: the crowd's idle
  ones (no line names them, no frame of a few) farthest from its middle are
  left out, and the export says who.
- **24 or more with code** (armips): two ways. (1) Let ordinary actors use
  the three kept slots: one instruction (`mov r1, #3` to `#0` in that
  loader), 24 at once, but a hero or partner loaded after them takes slot
  0-2 by force and replaces whoever is there: safe only in scenes without
  them, and the patch is global (story scenes put the hero in their layout
  too, usually first, so likely fine, but unproven). (2) A larger array:
  the allocation size and every loop bound over it (`cmp rX, #0x18`: 16 in
  `overlay_11_022EE5E4.s`, 4 more elsewhere in overlay 11, each to check it
  is about actors) and the 0x250 stride, plus room on the heap (arena 6).
  Then the other limits come in: 128 hardware sprites (OAM) for the bodies,
  shadows and emotes, and the 96 loaded sprite files (`wan_table`). Doable
  with armips, a day's work and a long test; not built.
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
| crowd | **Fake works** (seen) | Up to 21 actors at once in a scene; free actors give the species. `ExecuteCommon<actor X>(CORO_JUMP_HAPPY_FUNC_SERIES, 0)` on each without waiting makes the whole crowd cheer at once. |
| follow | **Fake works** (seen) | `MovePositionMark<actor A>(1, mark); Wait(16); MovePositionLives<actor B>(1, ACTOR_A);` B walks to where A is (actions given to an actor run alongside the script; `WaitExecuteLives(ACTOR_B)` waits). A longer walk: a few legs. |
| chase | **Fake works** (seen) | The same with speed 3 for both (`MovePositionMark<actor A>(3, …)`, `MovePositionLives<actor B>(3, ACTOR_A)`). |
| wander | **Fake works** (seen) | `ExecuteCommon<actor X>(CORO_MOVE_RANDOM, 0)`: Sky's own random stroll (turn, step, wait 48 frames, for ever, until the scene ends or another action is given). |
| daytime | **Fake works** (seen) | Sky's scripts never tint for dusk: they switch to the place's evening or night level (`G01P01B`, `G01P01C`; `LEVEL_D01P11B` is the beach at sunset), and dim a room at bedtime with `screen_FadeChange(1, 60, 0, 192)`. Their only warm tint, `screen_FlushOut(1, 4, 4, 128, 96, 64)`, shows mode 4 multiplies the screen by the colour (seen: 255, 128, 64 turned the sand orange and the sea green: far too strong). So mid-scene: evening `screen_FlushOut(1, 30, 4, 224, 176, 136)`, night `screen_FlushOut(1, 30, 4, 80, 112, 144)`, the factors measured between the guild's crossroads by day and its evening and night levels (**seen** side by side: `target/ds/lab/tint2_grid.png`); back to day: `screen_FlushIn` with the same numbers. The tint stays until then. Mode 3 fills the screen with the colour and `FlushIn` did not undo it: avoid. Sky also has evening variants of some places as levels of their own (`LEVEL_D01P11B` is the beach at sunset): the exporter can pick the level when the film's light says evening. |
| status | **Fake works** (seen) | Effects on the actor: `EFFECT_CONFUSE` (stars), `EFFECT_POISION_BUBBLE` (sic), `EFFECT_GIANT_Z` (asleep), `EFFECT_TRAPPED_IN_ICE` (frozen), `EFFECT_STATS_DOWN_EFFECT`, `EFFECT_STATS_UP_EFFECT` and `EFFECT_YAWN` (not tried); with poses below (asleep: animation 25 or 33; shivering: 27 or 47). |
| attack | **Fake works** (parts seen) | Wind-up `SetAnimation<actor A>(28)`, strike `SetAnimation<actor A>(48)` (a lunge), then on the target an effect: `EFFECT_PUNCH_ATTACK`, `EFFECT_LIGHTNING_STRIKE` (a bolt from the sky), `EFFECT_FIRE_ATTACK` (a tall flame), `EFFECT_SLASH_ATTACK`, `EFFECT_EXPLOSION_SMALL` (seen; `EFFECT_WATER_EXPLOSION`, `EFFECT_AIR_BLADES` and others of the table not tried); the target's hurt pose `SetAnimation(22)` or `(36)`; a shake `camera_SetEffect(2, 1, 1.0)` ... `camera_SetEffect(0, 0, 0)`; back to standing `SetAnimation(2)`. Real move animations (Thunderbolt as in a dungeon) need code: the dungeon's effect system is not loaded in ground mode. |
| fight | **Exported** (seen) | Each blow the fights' rules played (a directed fight's as written: `scenes::fight::host::played`, the same battle the engine plays) staged as `attack`: the fighter turns to its target, its wind-up and lunge, the move type's effect on the target, a shake; the target slides back (`SlidePositionMark`, further and the view shaking hard for a critical), its pain once, and back to its place; missed: the target hops. A knock out: its faint. The lines said in the fight's moments play where they come; no bars, no log, no "Missed!". Below, "Routines, floating, fights". |
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

## Routines, floating, fights (`beside`, `float`, `fight`, recoils)

- **Lines beside the film** (`beside … end`, `at …:`, a line ended by `&`,
  seen): as Sky's scenes run their actors at once. Each actor's lines of
  the block go to a routine of its own, `def n for actor X { … }` after
  `def 0` (Sky's `m01a0301` wakes the hero and the partner that way). The
  routine starts when the scene loads and waits on a lock (`Lock(n)`); the
  script starts it where the block is (`WaitLockLives(n, X); Unlock(n);`)
  and goes on; `wait name` (or the scene's end) joins it
  (`WaitLockLives(m, X)`: X waits on its next lock once its lines are
  done). Inside, the lines are Sky's targetless ones (`SetAnimation(28)`,
  `MovePositionMark(…)`), each waiting for the last, and Sky's common
  routines are called (`CallCommon(CORO_JUMP_ANGRY_FUNC)`) instead of run
  (`ExecuteCommon<actor>(…_SERIES, 0)`). Locks: one where all routines end,
  one per block for those starting there, one after each part another
  follows; Sky's scripts use 0 to 19. `together` stays as it was (each line
  told at once: Sky's lives operations already run alongside the script).
  - **Seen, in the game**: an order given to an actor (`<actor X>`) replaces
    its routine (`target/ds/p1b`: a `SetDirection` before the `Unlock`, and
    the routine never ran; `WaitLockLives` on it did not hang). So an actor
    the script tells anything before its last block plays its lines in the
    script instead, all at once, waited for at the join (as before), and
    one hidden from the start hides in its routine (`SetOutputAttribute`
    before its first `Lock`).
  - **Not possible**: `with (actor X) { jump @label; }` to start a routine
    on demand: the script itself jumped (seen: the scene skipped to its end).
  - A block falls back to the script, at once, when a line in it is said,
    moves the camera, tells two actors in one line, or runs a routine Sky
    calls no other way (`CORO_MOVE_RANDOM`, `wander`).
  - `at mark + 2s:` waits 2 s from where the line is written, not from the
    mark nor the scene's start (the export has no clock of the scene).
  - `when a arrives:` does not wait for `a` in the routine: its lines start
    at once.
- **float** (seen: Dragonite and Altaria over the tower's floor, their
  shadows below): a routine of its own, `forever { MoveHeight(pace, 8);
  MoveHeight(pace, 0); }`, 8 pixels at least (the engine's 4 hardly shows on
  Sky's sprites). `float 0`, or any order the script gives it later, lands it
  first (`MoveHeight<actor X>(1, 0)`).
- **Recoils** (`hurt`, a blow): a slide back from who struck (from oneself:
  back the way it looks), `SlidePositionMark`, facing as it was, over 10
  frames.
- **A plain `fight a vs b`** (by the rules) is played to its end when the
  film is exported, with the seed the line has, so the blows are those the
  engine plays (with clear weather and no one choosing: a `Played` fight's
  menu is never asked). A fight its rules can't play (a crowd, a line of
  one's own) becomes a short exchange of two blows, and it is said.
- **Seen** (`a_world_full_of_pokemon`, `--partial`, all ten scenes from a
  cold boot, 16 000 frames, then the quiz as usual): the tower's two
  `beside` blocks as nine routines (the young dragons' emote shown while
  Kyogre's Water Pulse lands), the lake's floating Latias and Latios, and the
  amphitheater's directed final: six blows, the lines said between them,
  Hitmonlee's faint.

## Poses (`SetAnimation<actor X>(n)`)

`n` goes through overlay 11's `SETANIMATION_TABLE` to a group of the
actor's sprite (`ChangeActorAnimation`), with no lookup by species: 22-41
play groups 13-32 once, 42-61 loop them, 81 and 82 play 33 and 34; 2 is
the species' standing. Groups past 12 are each sprite's own. The starters
(Pikachu, Piplup...: 42 species) share one order (13 EventSleep ... 33
Faint, 34 HitGround); the other 1113 order theirs their own way (Dusknoir
lies down with its 27, Grovyle with its 20, Wigglytuff's 13 is a roll,
Snorlax has none). A group the sprite lacks shows it standing; the groups
of `monster.bin` and `m_attack.bin` show too (Wigglytuff sleeps with 5).
So the exporter picks each species' own group by name, or a close one
(`own_group` in `ds.rs`, the table `data/extra/ds_animations.txt` (never committed) written by
`tools/ds/animations.py` with SkyTemple's names). **Seen**: faint, waking,
hurt, trembling and sleep on Piplup, Pikachu and Dusknoir
(`target/ds/lab/poses_sheet.png`).

The table below was seen on Pikachu, facing down, in that light; its
pictures are the starters' order.

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

## Seeing more of a scene: no zoom, a tall view, pans

- **No zoom in Sky's ground mode.** Its places are text BGs at a fixed
  scale and its Pokémon are plain OAM sprites; no script operation scales
  the view (the camera operations move it, `camera_SetEffect` shakes it),
  and pmdsky-debug knows no memory value that does.
- **Tall view** (`export-ds --tall`, seen): the top screen shows the same
  place a screen above the bottom one, `back2_SetMode(4);
  back2_SetGround(<the same level>); camera2_SetPositionMark(...)`, and
  `camera2_MovePositionMark` glides it with performer 0 at the same pace,
  as Sky's own two-screen sky scene does (`S13P04B/t02a0203`). The ground
  joins across the screens where the place is tall enough (the bottom
  camera at least 288 pixels down). But **the top screen draws no
  actors**: the beach's crowd standing in the top screen's part showed on
  neither screen (`target/ds/lab/s2.png`). Sky's top-screen things are
  objects flagged for it (the object table's third field is 1:
  `v22p01a1`...), never Pokémon. So the bottom camera stays on the film's
  subject and the top shows the scenery above; where the subject is near
  the top of the place the two screens show some rows twice (the game
  stops the top camera at the edge). A place Sky pairs with a picture of
  its own (`TOP_SCREENS`: the beach's sky and rock) keeps that picture: the
  beach above its sand is only sea, its animated foam blank on the top
  screen.
- **Pans** (seen): a `frame` of people wider or taller than the screen
  (with room for their sprites) glides to one end, pans across at 0.75
  pixels a frame, and comes back to their middle; less than 24 pixels out,
  their middle is enough.
- **A true zoom**: the place's BGs would have to become affine (rotation /
  scaling) BGs, which take only 8-bit maps of 256-colour tiles (Sky's
  places are 4-bit text BGs, up to three layers, palette animation), and
  each Pokémon an affine sprite (32 matrices for the screen, double-size
  boxes, so fewer fit in the 128 OAM entries). That rewrites the ground
  renderer: C code (c-of-time) hooking overlay 11's BG and sprite setup,
  weeks of work, not realistic.
- **A frozen wide shot by display capture**: the DS's display capture
  writes what engine A shows (BGs and sprites, 256x192 at most, no scaling
  while capturing) to a VRAM bank; shown again as a 16-bit bitmap BG
  (affine), it can be scaled down. One capture holds only the screen
  already seen, so a 2x wide shot means four captures at four camera
  positions, shrunk and pasted by the CPU into one 256x192 picture, shown
  still while the script waits. It needs code (c-of-time: a special
  process called from the script, the capture register, a free 128 KB VRAM
  bank, while ground mode uses the banks for its BGs and sprites, so their
  mapping saved and restored), and the picture is still: the Pokémon
  frozen. Possible for a one-off establishing shot, a few days with
  c-of-time and an ARM gcc; not built.

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

## Dungeons and boss fights (`patch_rom.py --dungeon`, seen 2026-11-22)

Seen from a cold boot (`tests/games/test_rescue`'s Test Woods): the film,
then "Test Woods B1F" in Mystifying Forest's tileset, Chikorita and
Totodile at level 6, Hoothoot and Spinarak met on floor 1; with `--boss`,
Ursaring and its two Teddiursa in the arena, Ursaring's life doubled (88),
and once it falls its `boss_falls` line, then the title. The scene before
the fight (Beach Cave's end) is the same code path, not reached by the key
script (it would need the four floors walked).

What holds: the floor count, each floor's tileset, song, layout figures
(the look's), Pokémon (species, level, weights from the counts), traps and
items (weights from the counts, by Sky's categories), recruiting and the
other rules Sky's restrictions have, the team and its level, the dungeon's
name (every language), the arena's size and walls/water, the boss and its
guards where the arena puts them, their level and stats, the boss's life
multiplied, the fight's song, the lines before and after the fight.

What is lost:

- **One slot**: the dungeon replaces Beach Cave and its pit, so one dungeon
  (and one fight) a ROM; the story's own Beach Cave is gone (harmless while
  the film starts the game). The game's dungeon is entered at once after the
  film; the `.game`'s other steps (chapters, talks, shop, recruit) are not
  played.
- **Floors counted up** (`goes_up`), a dungeon of 0 floors (a fight alone):
  not done.
- **One level a species a floor**: Sky lists a species once a floor.
- **The boss's moves** (`boss_moves`): fixed-room Pokémon know their
  species' level-up moves. **Its item** (`boss_item`): they hold nothing.
  **Its title and cry**: the title is said in its first line; no cry.
- **The arena's allies, watchers, items and traps**: not placed (only the
  boss and its guards).
- **Phases** (`boss_phase`: lines, `anger`, `shake`, weather, reinforcements,
  heals at HP thresholds): Sky has nothing for them; no script runs during a
  dungeon. Lost.
- **The boss joining the team** after the fight: not done.
- **Lost fight**: the game's own way back (`CORO_EVENT_DIVIDE`), which,
  before the quiz, starts the new game (and the film) again.
- The hero and partner have Sky's default names (no quiz, no naming).

What code would add (c-of-time or armips):

- **Phases**: a hook in the dungeon's turn loop (c-of-time's
  `CustomScriptSpecialProcessCall` is for ground scripts; dungeons need an
  overlay 29 patch, as SkyTemple's "dungeon interrupts" do): read the boss's
  HP each turn, and at a threshold show a message (`LogMessage` /
  `DisplayMessage`), raise its stats (`BoostOffensiveStat`), call
  `SpawnMonster` for reinforcements, `TryIncreaseHp`, `TryActivateWeather`,
  shake the screen. All exist as functions in pmdsky-debug's overlay 29;
  none is reachable from a script.
- **Boss moves and items**: set the boss's moves and held item after it
  spawns (a fixed-room hook, same patch).
- **More dungeon slots**: SkyTemple's dungeon list can grow (its "Add
  dungeon" patches the list's size in arm9 with armips), so a game's every
  dungeon could get one of its own instead of Beach Cave.
- **New tilesets**: a tileset of the Workshop's own (`tileset:` a picture)
  needs SkyTemple's dungeon tileset import (`dungeon.bin`'s DMA/DPC/DPCI/DPL
  files), doable in Python without code, but not done here: the look of a
  Sky dungeon only.

## Things, joining, evolving, marks

- **open / close** (seen): a thing put in the film (`put object chest`) is
  an object of the game in the scene's layout (`// ds-export object
  OBJECT_N<number>`, renamed by `patch_rom.py` to the game's name).
  Objects have a table of their own (`ov11_02321DC8`, EU `0x23228F8`):
  1-8 loop groups 0-7, 9-16 hold their first picture, 17-24 play them
  once. Open: `SetAnimation<object>(17)`; close: `(9)`. A chest opened
  and closed on the beach. One of each object per scene (a script names
  an object by its kind).
- **join** (seen): as Sky's recruitment scenes, `me_Play(5);
  message_Mail("[CN]… joined the team!"); WaitMe(5)`.
- **evolve** (seen): no script operation changes an actor's species, so
  the evolved one is an actor of its own, hidden until then: a stats-up
  shine, `screen_WhiteOut`, the old one hidden, the new one put in its
  place and shown, `screen_FadeIn`. The film's later lines naming the old
  one go to the new one. Pikachu became Raichu on the beach.
- **mark**: a `@label`; what the film times from it is already written as
  waits.

## A demo made for the DS (seen, cold boot)

Three places (beach, forest, beach), 110 s from the title to the last
line: evening light, a sepia memory, rain and night in the forest, emotes,
Sky's own routines (looking around, shaking the head), two attacks with
their effects, a faint, an evolution, the beach's top-screen picture.
What it taught the exporter:

- a light said as a scene opens is put on before the screen fades in
  (`screen_FlushOut(1, 0, …)` then `screen_FadeIn`: tinted from its
  first frame; after the fade in, one plain frame shows first), except on
  Sky's own evening and night levels (lettered past A);
- the same light again (after the rain) is kept: `FlushIn` then
  `FlushOut` flashed the plain scene;
- the memory's sepia replaces the light: the light is put back after it;
- rain is Sky's `EFFECT_VERY_HEAVY_RAIN` (698), as its scenes have it on
  performer 3; the effect named Rain (16) showed nothing;
- the top screen cannot share the light: `screen2_FlushOut` with
  `screen_FlushOut` made both screens flicker.

## More actors with armips (`patch_rom.py --more-actors`, seen)

With armips (`SKYTEMPLE_ARMIPS_EXEC` naming armips.exe), `--more-actors`
applies SkyTemple's ActorAndLevelLoader to the copy: the actor table
moves to `BALANCE/actor_list.bin`, and every Pokémon of the film without
an actor of its own gets a new actor there (`NPC_DW386`, ...), none of
the story's changed. Past the 45 free ones the exporter names them
`ACTOR_NPC_FREE045`, ... (without the option `patch_rom.py` refuses them).
**Seen**: 48 species in two scenes of 24 from a cold boot (at most 21 of each can have loaded, as found later), the last ones
(Clefairy, Golbat, Parasect, Golduck) shown right
(`target/ds/lab/many_grid.png`). It unlocks any number of species in a
film (still 21 at once in a scene), and the level list moves too
(`BALANCE/level_list.bin`): new places become possible, not done yet.

## The gate's sheets (`tools/ds/gate.py`, 2026-11-21)

Every word's Wiki example, exported (`--partial`), patched and played
from a cold boot, a sheet a family (`target/ds/gate/`). Every family
played to its end and on to the personality quiz: no scene hung. What
differs from the engine, family by family (the words not named play as
the table says):

| Family | What the sheets show apart from the engine |
|---|---|
| Structure | `if chance(50)`: Sky's `switch ( random(100) )`, in tenths (seen: "Lucky!"). Transitions: fades only. |
| Speech | `box top`: only the portrait goes up. `whisper`: normal text. |
| Presence | `appear`/`vanish`: a smoke puff, then at once. `crowd`: still. |
| Formations | As the engine. |
| Movement | `jump` to a place: a run (no arc). `follow`/`chase`: catch-ups after each move. |
| Turning | `face … slowly`: turns at Sky's pace (the engine's slow turn is left out). |
| Body | `eat`: the eating effect, no apple (items: none). `hurt`: the slide back, its pain. |
| Emotions | As the engine (Sky's emotes are the engine's numbers). |
| Camera | As the engine; a frame wider than the screen pans across. |
| Screen | `flash` of a colour: Sky's FlushOut and FlushIn; `memory`: Sky's sepia. |
| Sound | A sound played in a loop by name (`chirp`) and faded: left out. |
| Effects | `effect … from a to b`: the effect on b (no shot flying). `weather sparkles`, sand, fog, sun: nothing. `daytime`: Sky's evening / night tints. |
| Objects | `hold`, `give`, `drop`, `throw`: no item drawn (no item sprite in ground mode). `open`/`close`: as the engine. |
| Fights | `fight` by the rules: its blows as attacks (the Wiki's: one Thunder, Piplup faints, as the engine plays it). `dodge`: no "Missed!". `finish`: no slow motion. |
| Time | `at 2s:` and `at mark + 1s:`: from where the line is written. `together`: at once. |
| Story | `{value}`: the number as last set ("I have 3 apples."). `seen()`, `keep`: the scene's own variables. Menus as Sky's. |
| Dungeon | `stairs`, `trap`, `ambush`, `shop`, `steal`: the Pokémon's part only; stairs, traps, items, the log and "-50 P" left out. |
| Guild | `job`, `rank`, `morning_call`, `dinner`: the lines said; the cutaway, Loudred's bubble and the menus left out. |
| Fishing | `cast`, `bite`, `reel`, `release`: poses and lines; the rod, line and bobber left out (pictures of one's own). |
| Advanced | `beside`, `when`, `at`: routines (above). `float`: hovering. `layer`, `ghost`, `zoom`, `bubble`, `light`, `flip`, nodes, `every`, `while` (a variable set from another), `timer`, `spin`: left out; `player`, `dialogue`, `explore`, `game`, `way`: a cutscene has none. |
