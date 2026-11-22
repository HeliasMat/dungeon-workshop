# How Explorers of Sky keeps its dungeons

What `tools/ds/dungeon.py` writes, and why it writes it there. Read from the
game with `skytemple_files` (its readers: `MappaBin`, `FixedBin`,
`HardcodedDungeons`, `HardcodedFixedFloorTables`, `HardcodedDungeonMusic`,
`HardcodedDefaultStarters`) and from the game's decompiled scripts
(`SCRIPT/COMMON/unionall`, `D01P11B/m01a0301`, `D01P41A/m01a0401`). The
addresses are pmdsky-debug's, through SkyTemple's `pmd2data.xml`.

## The dungeon list (arm9)

One entry a dungeon (180): its number of floors, the floor list of
`mappa_s.bin` it takes them from (`mappa_index`), the floor of that list it
starts after (`start_after`), and how many floors the whole group has.
Two dungeons may share a list: Beach Cave (dungeon 1) is floors 0–3 of list
1, Beach Cave Pit (dungeon 2) is floor 4 of the same list, one floor that is
a fixed room. Sky's story bosses are made this way: the boss's floor is a
dungeon of its own, after the one before it.

Beside it, the **restrictions** (arm9, one a dungeon): floors counted up or
down, recruiting, experience, level reset, money, IQ, how many items and team
members come in, the turn limit (the wind).

## Floors: `BALANCE/mappa_s.bin`

A list of floor lists (100). A floor is:

- a **layout**: structure (how rooms are laid), densities (rooms, items,
  traps, buried items, water, extra hallways), the chances of a Kecleon shop
  and a monster house, the **tileset** (0–169, the dungeon's look), the
  **music** (an index into the dungeon music list of overlay 10, 170 entries,
  each a song or a random group; songs 21+ are the dungeon tracks), weather,
  darkness, enemy IQ, the most money, and `fixed_floor_id` (0: generated;
  else a fixed room of `fixed.bin`);
- the **monster spawn list**: species (md index: gen 1–3 as the Pokédex, then
  shifted by the Unown forms), **level**, and the spawn weights as thresholds
  out of 10,000 (each over the one before, by md index), the monster-house
  weights the same way. A weight of 0 keeps a species out (Kecleon, 383, is
  listed with 0 on every floor for the shops);
- a **trap list** (25 thresholds);
- **item lists** (floor, shop, monster house, buried, two unknown):
  categories by number, thresholds out of 10,000; the items of each category
  by number, thresholds within their category.

`mappa_gs.bin` is made from it (tileset and fixed room of each floor, for the
ground side); SkyTemple writes both, and so does `dungeon.py`.

## Fixed rooms: `BALANCE/fixed.bin` and its tables

256 rooms, each a grid (width × height) of actions: a tile (floor, wall,
impassable wall, secondary terrain/water, the leader's start, the
attendants' starts, warp zone, key doors) or an **entity** (number − 16).
An entity is a row of the entity table of overlay 29 (269 rows): an item,
a **monster** and a tile (trap). A monster row (120) is a species, a **stats
row** and a kind (6: a strong enemy, its stats from the row; 10: an ally;
...). The stats rows (overlay 10, 99) give level, **HP**, attack, special
attack, defence, special defence: a boss's life is set there, as large as
wanted (up to 9,999). The
properties of each room (overlay 10): its **music** (an index into the
dungeon music list), moves and orbs allowed, whether the floor ends when all
enemies are down. Many entity rows are used by no room: free to take.

## Entering a dungeon from a script

The story enters Beach Cave like this (`m01a0301`):

    main_EnterDungeon(1, 0);
    switch ( message_Menu(MENU_DUNGEON_INITIALIZE_TEAM) ) { }
    main_EnterDungeon(-1, 0);
    hold;

The team is the one the personality quiz made (`ProcessSpecial(
PROCESS_SPECIAL_INIT_MAIN_TEAM_AFTER_QUIZ)`); with no quiz, of the game's
**default hero and partner** (arm9, Charmander and Bulbasaur, and their
starting levels), which can be changed.

When a dungeon ends, the ground starts `GETOUT_SCENARIO_DUNGEON`
(unionall): cleared (`$GROUND_START_MODE` 8), it goes by `$DUNGEON_ENTER`
(the dungeon) and the story's progress (`$SCENARIO_MAIN`) to the next event.
Beach Cave cleared at chapter [2, 2] plays `M01A0401` (in the pit's ground,
`D01P41A`: the boss scene), which ends with `CORO_BOSS_WIPE_FUNC` and
`main_EnterDungeon(2, 30)`: the boss fight. The pit cleared plays the scene
after it. Lost, the team goes back by `CORO_EVENT_DIVIDE`.

So **a boss fight in Sky is a fixed room on a dungeon of its own, with a
ground scene before it and one after it**, played from `GETOUT`. The boss
says nothing during the fight: no script runs in a dungeon (a few fixed rooms
have their lines in the dungeon code itself, overlay 29).

## What `dungeon.py` takes

- Dungeon 1 (Beach Cave) and 2 (Beach Cave Pit), floor list 1, fixed room 1:
  safe early in a new game, as the film starts the game before the story
  ever goes there. Both are renamed in every language's text (the four
  "Dungeon Names" blocks).
- Free entity, monster and stats rows of the fixed-room tables: those no
  other room reaches.
- The default hero and partner, and their level.
- `GETOUT_SCENARIO_DUNGEON`'s cases for dungeons 1 and 2, only while
  `$SCENARIO_MAIN` is under [2, 0] (a new game before the quiz): Sky's own
  story is untouched.
