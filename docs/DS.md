# Films on a real DS game

A film of the Workshop can be played by Pokémon Mystery Dungeon: Explorers
of Sky itself, as one of its own cutscenes: exported to the game's script
language (`scene_player export-ds`), put in a copy of your own ROM
(`tools/ds/patch_rom.py`), and played in an emulator. What exports and what
does not: `tools/ds/LIMITS.md`.

The export is the library `ds_export` (`ds_export/src/lib.rs`: `film`,
`dungeon`, `table`), which `scene_player export-ds` calls; what each word
becomes on the DS is declared with the word (`film_core/src/ds.rs`), and
the Python side, what puts it in the ROM, is `tools/ds/`.

## Writing a film for the DS

A line `for ds` before the first scene (the Script's **DS Mode** writes it)
says the film is for the DS, played as a new game starts; `for ds scene`, in
place of a scene of a game going on (the hero and partner chosen). Then what
the DS can't play is told as you write, from the same table `scene_player
export-ds --table` prints (`film_core/src/ds.rs`): a word the DS has nothing for
(`E-DS-WORD`, with how to write it: a `fight` as blows of `attack`), a word
that exports with a loss (`R-DS-LOSS`: what is lost), more than 21 Pokémon at
once in a scene (`E-DS-ACTORS`: who the export leaves out), a place that is
neither a map of Sky's nor a decor of yours (`E-DS-PLACE`; a decor is a
`R-DS-LOSS`, below), the hero or the partner before the player
chose them (`E-DS-HERO`), a zoom or a frame wider than the screen
(`R-DS-ZOOM`: Sky has no zoom), a Pokémon or form the ROM has no sprite for
(`R-DS-SPECIES`), what the export leaves out (`R-DS-LEFT`: items, an engine's
command with no DS operation). In DS Mode the completion offers only what
exports, and the Wiki shows each word's DS badge.

Nothing is lost in silence, and no Pokémon becomes another:

- **A line said in a bubble** (`who (bubble[, face]): …`, `bubble`, `think`,
  `shout`) is a line of Sky's box, its face as the brackets say, closing by
  itself after the bubble's time (`message_SetWaitMode(frames, 0)`, as Sky's
  endings time theirs). One at a time: no two bubbles side by side.
  `chatter`: Sky's emotes over the crowd (!, ?, laughing, joy, sweat).
- **A Pokémon or form SpriteCollab adds** (numbered past Sky's 1,155: 2000 +
  its Pokédex number, variants from 5000) **is put in the ROM** when the data
  has its sheets (`data/extra/sprites`, `sprites.txt`, its faces in
  `data/extra/portraits`): a monster of the game's own, its name, its sprite
  and its portraits (below, "Pokémon of SpriteCollab's"). Without its sheets,
  or where putting it in fails, a form is drawn as its base species
  (`pichu_spiky` as Pichu), a Pokémon Sky lacks as a stand-in, Unown “?”, its
  name said in its lines (no portrait); told by the export, and by DS Mode
  (`R-DS-SPECIES`) when the data has no sheets. `patch_rom.py` refuses a
  species neither in the ROM's `monster.md` nor put in, instead of letting
  it alias.
- **A pose a species' Sky sprite lacks** (Nod, Sit, Rotate...): the nearest
  one every sprite has (as the film words' `nearest`: a hop, its sleep, its
  attack), told once a species and pose.
- **More than 21 at once** from a scene's start: the idle are left out first,
  then those only acting where they stand, then those moving, never one who
  speaks or is framed among a few; their lines go with them, and the export
  names each one left out.

## Pokémon of SpriteCollab's

A film may cast a Pokémon of a later generation (`grookey`) or a form
(`pichu_spiky`) that SpriteCollab adds and the data has imported: on DS it
is itself. `export-ds` writes its sheets in SpriteBot's layout beside the
export (`x.exps.monster<species>/sprite/AnimData.xml`, `<Anim>-Anim.png`,
`<Anim>-Shadow.png`, `portrait/<Face>.png`: `spritecollab::export`) and a
line `// ds-export monster <species> <instead> <folder> <name>`; then
`patch_rom.py` (`tools/ds/add_monsters.py`, with skytemple_files):

- takes one of the monster list's 34 spare entries (537 to 551, 581 to
  599: never shown by the game, each with a name of its own), gives it the
  numbers of its base species or Pikachu's, and its name in every language;
- makes its sprite from the sheets as SkyTemple's sprite import does
  (`import_sheets`, `split_wan`), adds the three parts to
  `MONSTER/monster.bin`, `m_ground.bin` and `m_attack.bin`, its sizes in
  the game's table;
- puts each face in the portraits (`FONT/kaomado.kao`), so its lines show it;
- casts the film's actors of it as that monster.

Its poses are the ones its sheets have (each by its name: a pose they lack
plays the nearest, as for Sky's own). One that can't be put in (no spare
entry left, sheets SkyTemple can't read) is told and drawn as `<instead>`,
its base species or the stand-in. Only the species the film casts are put
in, so the ROM stays small. What it costs and how far it goes:
`tools/ds/LIMITS.md` ("Species the ROM lacks").


## A place of your own

A scene may be in a decor made with the Workshop (a `.decor`, built): the
export makes it a map of Sky's own. Beside `x.exps`, `export-ds` writes
`x.exps.decor<level>.lower.png` and `.upper.png` (8-bit indexed: index =
palette × 16 + colour, colour 0 clear, each tile of 8 pixels in one palette
of 16 colours, 16 palettes shared by both layers), `.walls.txt` (its walls,
a tile a character), and a line `// ds-export map LEVEL_DECOR_<level> ...`;
the decor is padded right and down to whole chunks (3 × 3 tiles, 24
pixels). `patch_rom.py` makes the game's files of them (`MAP_BG/dwmap0.bpc`,
`.bpl`, `.bma`, with skytemple_files), points a level no story scene plays
(`T00P04A`, then `T00P04A2`, `T00P03`, `T00P02`: four decors a film) at
them, and the scenes' `LEVEL_DECOR_<level>` becomes that level. Positions
are the decor's pixels, as in the film.

What is lost (`R-DS-LOSS` on the scene's line): the decor is still, its
animated tiles and changing colours as they first are; a tile of more
colours than its palette holds (a shadow, a blend) takes its palette's
nearest; the top screen is black (`--tall` should show the place there, a screen
above, when it is 384 pixels tall or more: not tried with a decor). Too many different tiles for
the DS (1024 a layer) and `patch_rom.py` stops, and says it.

## A core for the Emulator space

The Workshop's **Emulator** space runs DS games through a **libretro core**:
a small library that is the emulator itself (`emulator/`, a minimal libretro
frontend, loads it). No core comes with the Workshop; you choose and fetch
one yourself:

1. Go to libretro's build server, `https://buildbot.libretro.com/nightly/`,
   then your system (`windows/x86_64`, `linux/x86_64`, `apple/osx/...`),
   `latest/`.
2. Take `melonds_libretro` (`.dll.zip`, `.so.zip` or `.dylib.zip`), or
   `desmume_libretro`. melonDS is the one tried: it plays Explorers of Sky
   without BIOS or firmware files (it uses its own FreeBIOS and boots the
   game directly).
3. Unzip it anywhere of yours, and in the Emulator space click **Choose a
   Core…** (or start the Workshop with `WORKSHOP_LIBRETRO_CORE=<the file>`).
   It is remembered in the Workshop's settings
   (`%APPDATA%\dungeon_workshop\settings.txt`, `~/.config/dungeon_workshop/`
   elsewhere).

Both are GPL-licensed programs of their own; they are never bundled with or
committed to this project.

## Playing a film on DS

1. Open the film in the Film space.
2. In the Emulator space, click **Play This Film on DS**, or **From Scene
   n** to start at the scene the Script's cursor is in. The first time, it
   asks for your Explorers of Sky `.nds` (only read, never changed) and for
   armips (`SKYTEMPLE_ARMIPS_EXEC`), which SkyTemple's patch for more actors
   is assembled with; both are remembered.
3. It runs `python tools/ds/watch.py <film> --prepare` (Python 3 with
   `skytemple_files`): the export (`scene_player export-ds --partial`: what
   does not export is left out, and said), then the film put in a copy of
   the cached base, `target/ds/workshop_play.nds`. The Script's text is
   exported as it is now, saved or not.
4. The copy is loaded, the core's ready state over it (the New Game menu,
   waiting), the film's Pokémon written in its memory, and A pressed for
   New Game: the film starts at once. Then the keys are yours (X is A: it
   closes each line of dialogue).

The first click makes the base and the core's ready state (about 10 s, more
for each new Pokémon of SpriteCollab's); then a click takes about 3 s.

### The loop, in one command

    python tools/ds/watch.py <film> [--scene n] --headless

exports the film, puts it in the cached base, plays 20 s of it from the
ready state without a window (melonDS through the `emulator` program, else
py-desmume) and writes `target/ds/watch/sheet.png` (a shot every 30 frames);
about 6 to 9 s in all. Without `--headless`, the Workshop opens and plays it
in its Emulator space. `--frames`, `--every`, `--period` (A every n
frames), `--out` and `--rom` as its help says.

How the base and the ready state work (`tools/ds/ds_cache.py`):

- **The base** (`target/ds/cache/<rom hash>/base/base.nds`) is your ROM with
  what every film needs, done once: SkyTemple's ActorAndLevelLoader, 64
  spare actors, the new game's event playing 64 scene slots
  (`SCRIPT/T00P01/dwscen01...`) after a jump, the decor maps' files, and
  the Pokémon SpriteCollab adds that the films played so far cast (each
  sprite made once and kept). A film casting one more makes it again.
- **A film's copy** is the base with only the film's files written over, in
  place (its scenes in the last slots, the jump pointing at the first of
  them, its decors, the actor list): about 1.5 s. Nothing else of the
  cartridge moves.
- **The ready state** is taken once on the base, on the New Game menu,
  before the game reads the event; it plays any film's copy. The game read
  the actor list at power on, so the copy's `.pokes` says which species
  the film's Pokémon are, written in memory after the state is loaded.

The cache is made again when the ROM or these tools change. What it can't
hold (more than 64 scenes, very large scenes or decors) is refused, with
the size: `patch_rom.py --boot` has no such limit and still makes a ROM to
share: `python tools/ds/patch_rom.py <rom> <exps> --boot <out.nds>
--more-actors [--patch out.bps]`. Limits and measured times:
`tools/ds/LIMITS.md` ("The fast loop").

The copies, the cache, its savestates and screenshots are your game's:
never commit them (`target/` and the settings folder are out of the
repository).

## A film that leads into its dungeon (a trial)

A dungeon of the Workshop (`.dungeon`) and the boss fight it ends with
(`.fight`) can follow the film on DS, in place of Beach Cave:

    scene_player export-ds games/<game>/dungeons/<name>.dungeon --out target/ds/<name>.json
    scene_player export-ds <film> --out target/ds/<film>.exps --partial
    python tools/ds/patch_rom.py <rom> target/ds/<film>.exps --boot target/ds/<name>.nds --dungeon target/ds/<name>.json

- `export-ds <file.dungeon>` writes the dungeon as the rules read it (JSON):
  its floors (Pokémon, levels, weights, items, tileset, song), the arena,
  what the boss says, and the team its `.game` names.
- `--dungeon`: a new game plays the film, makes the game's team (the
  `.game`'s `team:`, at the dungeon's `level:`) and goes into the dungeon.
  Its floors are Beach Cave's, renamed; at its end, the boss says its
  `boss_says` line in the film's last place, the fight is Beach Cave Pit
  (its arena, the boss and guards placed, the boss's life multiplied), and
  the boss says its `boss_falls` line after it. Then back to the title.
- `--boss`: straight into the fight, for a quick look (the film is `-` to
  play none).

`python tools/ds/boot_and_watch.py target/ds/<name>.nds` plays it from power
on. What is lost is in `tools/ds/LIMITS.md` ("Dungeons and boss fights"),
how Sky keeps its dungeons in `tools/ds/DUNGEONS.md`.

## Without a window

`emulator <core> <rom> [frames] [out.png] [--press A:90] [--every N]` runs a game with no
window and writes its last frame (both screens) as a PNG, with the core's
options listed: the quickest check that a core works. `--state-in <file>`
starts from a savestate (`--pokes <copy>.pokes` then writes a film's
actors in memory), `--state-out <file>` keeps the last frame's: what
`watch.py` plays with. The Python tools
(`tools/ds/boot_and_watch.py`, `record.py`) do the same with py-desmume and
make contact sheets and videos.

## The quality gate

`python tools/ds/gate.py <rom> [family...]` checks every word of the
language on the real game at once. For each family of the vocabulary
(`export-ds --table`: the core ones, the optional ones, the advanced
words), it takes each word's example from the Wiki (`workshop_wiki/wiki/*.txt`;
a line said, `option`, `{if}`, `{value}`, `seen()`, `#id` and `keep`, which
have no page, get a short one of the script's own), exports it with
`scene_player export-ds --partial`, makes the examples that export the
scenes of one film (the word said on black before each), patches it into a
copy of your ROM (`patch_rom.py --boot --more-actors`) and plays it from
power on in py-desmume, A pressed every 60 frames. It writes, under
`target/ds/gate/`:

- `<family>_sheet.png`: a contact sheet of both screens, a shot every 30
  frames (`--every N`), the frame above each (96 shots a sheet, then `_2`, `_3`...);
- `<family>.txt`: each word, whether it exported, and what the export left
  out or changed;
- `report.txt`: a line a family.

Give family names to run only those (`gate.py <rom> fights time`);
`--no-play` stops after the export and the patch (a minute for all). The
whole gate takes about twenty minutes. It needs a release build of
`scene_player`, Python 3 with `skytemple_files` and py-desmume, and armips
(`SKYTEMPLE_ARMIPS_EXEC`). Everything it writes is made from your game:
never commit it. What the sheets showed, family by family, is in
`tools/ds/LIMITS.md` ("The gate's sheets").
