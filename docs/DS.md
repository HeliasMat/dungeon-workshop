# Films on a real DS game

A film of the Workshop can be played by Pokémon Mystery Dungeon: Explorers
of Sky itself, as one of its own cutscenes: exported to the game's script
language (`scene_player export-ds`), put in a copy of your own ROM
(`tools/ds/patch_rom.py`), and played in an emulator. What exports and what
does not: `tools/ds/LIMITS.md`.

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
2. In the Emulator space, click **Play This Film on DS**. The first time, it
   asks for your Explorers of Sky `.nds` (only read, never changed) and for
   armips (`SKYTEMPLE_ARMIPS_EXEC`), which SkyTemple's patch for more actors
   is assembled with; both are remembered.
3. It runs `scene_player export-ds <film> --partial` (what does not export
   is left out, and said), then `python tools/ds/patch_rom.py <rom> <exps>
   --boot target/ds/workshop_play.nds --more-actors` (Python 3 with
   `skytemple_files`), each line they say shown under the screens.
4. The copy is loaded from power on with no save file, and A is pressed by
   itself every 90 frames through the title, the New Game menu and the controls screen; the
   film starts after about 25 seconds. Then the keys are yours (X is A:
   it closes each line of dialogue).

The patched copy, its savestates and screenshots are your game's: never
commit them (`target/` and the settings folder are out of the repository).

## Without a window

`emulator <core> <rom> [frames] [out.png] [--press A:90] [--every N]` runs a game with no
window and writes its last frame (both screens) as a PNG, with the core's
options listed: the quickest check that a core works. The Python tools
(`tools/ds/boot_and_watch.py`, `record.py`) do the same with py-desmume and
make contact sheets and videos.

## The quality gate

`python tools/ds/gate.py <rom> [family...]` checks every word of the
language on the real game at once. For each family of the plan
(`film/src/plan.rs`: the core ones, the optional ones, the advanced
words), it takes each word's example from the Wiki (`workshop/wiki/*.txt`;
a line said, `option`, `{if}`, `{value}`, `seen()`, `#id` and `keep`, which
have no page, get a short one of the script's own), exports it with
`scene_player export-ds --partial`, makes the examples that export the
scenes of one film (the word said on black before each), patches it into a
copy of your ROM (`patch_rom.py --boot --more-actors`) and plays it from
power on in py-desmume, A pressed every 60 frames. It writes, under
`target/ds/gate/`:

- `<family>_sheet.png`: a contact sheet of both screens, a shot every 30
  frames (`--every N`), the frame above each;
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
