"""Put a film exported by `scene_player export-ds` into the user's own
Explorers of Sky ROM, and write the patched copy.

    python tools/ds/patch_rom.py <rom> <film.exps> --boot <out.nds> [--patch out.bps]
    python tools/ds/patch_rom.py <rom> <film.exps> <MAP/scene> <out.nds> [--patch out.bps]

- `--boot`: the film plays as soon as a new game starts (after the title,
  before the personality quiz), and the game then goes on as usual. Each
  scene of the film becomes an acting scene of its own, added to the ROM
  (`SCRIPT/<place>/dwscen01.ssa/.ssb`, ...), and the new game's first
  event (`EVENT_M00A_01` of `SCRIPT/COMMON/unionall.ssb`) plays them in
  turn with `supervision_ExecuteActingSub`, the way the story chains its
  scenes. No savestate needed: `tools/ds/boot_and_watch.py` shows it.
- `<MAP/scene>`: the film's first scene replaces that story scene, like
  `G01P01B/m02a0101` (the first scene of chapter 2); further scenes are
  added after it, in the same event.
- The film's own lines (`// ds-export ...`) say what each scene is: a
  `// ds-export level` line starts a scene, then its `// ds-export actor`
  lines and its `def 0 { ... }`. Who stands in it is written into the
  scene's layout (`.ssa`).
- A Pokémon the game has no actor for (`species` given) takes a free actor:
  the ten test actors and the story's actors that no scene places nor any
  script names (FREE_ACTORS, 45 in all), its species changed in the game's
  actor table. The exporter may name such an actor `ACTOR_NPC_TEST001` or any
  placeholder (`ACTOR_FREE07`): it is renamed here.
- `--patch`: also a BPS patch from the ROM to the copy, to share without
  sharing the game.

Never run it on the original cartridge dump: it only reads `<rom>` and
writes `<out.nds>`. Nothing it writes may be committed.
"""
import contextlib
import logging
import os
import re
import sys
import warnings

from ndspy.rom import NintendoDSRom
from skytemple_files.common import string_codec
from skytemple_files.common.types.file_types import FileType
from skytemple_files.common.util import create_file_in_rom, create_folder_in_rom, get_ppmdu_config_for_rom
from skytemple_files.script.ssa_sse_sss.actor import SsaActor
from skytemple_files.script.ssa_sse_sss.layer import SsaLayer
from skytemple_files.script.ssa_sse_sss.performer import SsaPerformer
from skytemple_files.script.ssa_sse_sss.model import Ssa
from skytemple_files.script.ssa_sse_sss.position import SsaPosition
from skytemple_files.script.ssb.script_compiler import ScriptCompiler

from bps import apply_bps, bps

# The script engine's actor table in arm9 (pmdsky-debug's ENTITIES): 386
# entries of 12 bytes (kind, species, name, two unknowns).
# Keyed by the game code's last letter: P Europe, E North America, J Japan.
ENTITIES = {'P': 0x20A8890, 'E': 0x20A7FF0, 'J': 0x20A9438}
ENTRY = 12

# Actors whose species may change without touching the story: the test
# actors (placed only in the debug map T00P01), then the story's NPC actors
# of a fixed species that no layout (.ssa/.sse/.sss) places and no script
# names (found by scanning every layout of the ROM and every decompiled
# script). The names are SkyTemple's (pmd2data), not the game's text.
FREE_ACTORS = [f'NPC_TEST{n:03d}' for n in range(1, 11)] + [
    'NPC_USOKKII', 'NPC_RUKUSHIO1', 'NPC_RUKUSHIO2', 'NPC_RENTORAA', 'NPC_MANENE',
    'NPC_BURAKKII', 'NPC_BASHAAMO', 'NPC_ENPERUTO', 'NPC_DERUBIRU_A', 'NPC_DERUBIRU_B',
    'NPC_DERUBIRU_C', 'NPC_RUKUSHIO3', 'NPC_RUKUSHIO4', 'NPC_RUKUSHIO5', 'NPC_RUKUSHIO6',
    'NPC_RUKUSHIO7', 'NPC_RUKUSHIO8', 'NPC_RUGIA', 'NPC_HOUOU', 'NPC_DOOTAKUN5',
    'NPC_DOOTAKUN6', 'NPC_SAWAMURAA5', 'NPC_SAWAMURAA6', 'NPC_TSUBOTSUBO', 'NPC_RUNPAPPA2',
    'NPC_RUNPAPPA3', 'NPC_RUNPAPPA4', 'NPC_RUNPAPPA5', 'NPC_NASSHII', 'NPC_KOIKINGU',
    'NPC_HINBASU', 'NPC_MASUKIPPA6', 'NPC_MERIIPU', 'NPC_OTACHI',
]

# The new game's first event: the personality quiz, played right after the
# title. --boot plays the film before it.
BOOT_EVENT = 'coro EVENT_M00A_01 {\n    bgm_Stop();\n'


class Scene:
    """One scene of the film: its place, who stands in it, its script."""

    def __init__(self, level):
        self.level = level
        self.actors = []  # (name, fields)
        self.performers = []  # fields: type, x, y (the camera, the weather)
        self.lines = []

    @property
    def source(self):
        return '\n'.join(self.lines) + '\n'


def scenes_of(source):
    """The film's scenes: each starts at a `// ds-export level` line."""
    scenes, head = [], []
    for line in source.splitlines():
        words = line.split()
        if words[:3] == ['//', 'ds-export', 'level']:
            scenes.append(Scene(words[3]))
            scenes[-1].lines = head + [line]
            head = []
            continue
        if words[:3] == ['//', 'ds-export', 'performer'] and scenes:
            scenes[-1].performers.append(dict(zip(words[4::2], words[5::2])))
        if words[:3] == ['//', 'ds-export', 'actor'] and scenes:
            fields = dict(zip(words[4::2], words[5::2]))
            scenes[-1].actors.append((words[3].removeprefix('ACTOR_'), fields))
        (scenes[-1].lines if scenes else head).append(line)
    if not scenes:
        raise SystemExit('no `// ds-export level` line: not an export of scene_player export-ds')
    return scenes


def tile(value):
    """A position in tiles of 8 pixels, halves allowed: (tiles, offset)."""
    whole = int(float(value))
    return whole, (2 if float(value) - whole >= 0.5 else 0)


def cast(config, scenes):
    """Give each Pokémon without an actor a free one, the same in every
    scene; rename it in the scripts. Returns {actor name: species}."""
    entities = config.script_data.level_entities__by_name
    wanted = {}
    for scene in scenes:
        for name, fields in scene.actors:
            if 'species' in fields:
                species = int(fields['species'])
                if wanted.setdefault(name, species) != species:
                    raise SystemExit(f'{name}: two species ({wanted[name]}, {species}) in one film')
    taken = {name for name in wanted if name in FREE_ACTORS}
    spare = [name for name in FREE_ACTORS if name not in taken]
    renamed = {}
    for name in wanted:
        if name in FREE_ACTORS:
            continue
        if name in entities and name not in FREE_ACTORS:
            raise SystemExit(f'{name}: a story actor; its species is not changed (use a free one)')
        if not spare:
            raise SystemExit(f'more than {len(FREE_ACTORS)} Pokémon without an actor of their own')
        renamed[name] = spare.pop(0)
    for scene in scenes:
        text = scene.source
        for old, new in renamed.items():
            text = re.sub(rf'\bACTOR_{old}\b', f'ACTOR_{new}', text)
        scene.lines = text.splitlines()
        scene.actors = [(renamed.get(name, name), fields) for name, fields in scene.actors]
    return {renamed.get(name, name): species for name, species in wanted.items()}


def set_species(rom, config, region, actor_id, species):
    """The actor of that number is now of that species, in the actor table."""
    arm9 = bytearray(rom.arm9)
    at = ENTITIES[region] - rom.arm9RamAddress + actor_id * ENTRY + 2
    arm9[at:at + 2] = int(species).to_bytes(2, 'little')
    rom.arm9 = bytes(arm9)
    entity = config.script_data.level_entities__by_id[actor_id]
    entity.entid = int(species)
    # A savestate made before the patch holds the old table in memory: the
    # same change, for emu.py.
    address = ENTITIES[region] + actor_id * ENTRY + 2
    return f'poke={address:X}={int(species).to_bytes(2, "little").hex()}'


def layer(config, actors, performers=()):
    """The scene's only layer: the film's actors, where they first stand."""
    data = config.script_data
    directions = {d.name.upper(): d.ssa_id for d in data.directions.values()}
    placed = []
    for name, fields in actors:
        actor = data.level_entities__by_name[name]
        x, x_offset = tile(fields['x'])
        y, y_offset = tile(fields['y'])
        facing = directions[fields.get('facing', 'DIR_DOWN').removeprefix('DIR_').upper()]
        position = SsaPosition(data, x, y, x_offset, y_offset, facing)
        placed.append(SsaActor(data, actor.id, position, -1, -1))
    shown = []
    for fields in performers:
        x, x_offset = tile(fields['x'])
        y, y_offset = tile(fields['y'])
        position = SsaPosition(data, x, y, x_offset, y_offset, directions['DOWN'])
        shown.append(SsaPerformer(int(fields.get('type', 0)), 1, 1, position, -1, -1))
    return SsaLayer.new(placed, [], shown, [], [])


def replaced_layout(rom, config, path, actors, performers=()):
    """A story scene's layout with only the film's actors."""
    ssa = FileType.SSA.deserialize(rom.getFileByName(path), config.script_data)
    ssa.layer_list = [layer(config, actors, performers)]
    return FileType.SSA.serialize(ssa)


def new_layout(config, actors, performers=()):
    return FileType.SSA.serialize(Ssa.new(config.script_data, [layer(config, actors, performers)], [], []))


def compile_script(config, source, path):
    string_codec.init()
    ssb, _ = ScriptCompiler(config).compile_explorerscript(source, path)
    return FileType.SSB.serialize(ssb, config)


def put_file(rom, path, data):
    """Write a file of the ROM, adding it (and its folder) if new."""
    if rom.filenames.idOf(path) is not None:
        rom.setFileByName(path, data)
        return
    folder = path.rsplit('/', 1)[0]
    if rom.filenames.subfolder(folder) is None:
        create_folder_in_rom(rom, folder)
    create_file_in_rom(rom, path, data)


def add_scenes(rom, config, scenes, film_path):
    """Each scene as an acting scene of its own; the lines playing them."""
    plays = []
    for number, scene in enumerate(scenes, 1):
        place = scene.level.removeprefix('LEVEL_')
        name = f'dwscen{number:02d}'
        base = f'SCRIPT/{place}/{name}'
        put_file(rom, base + '.ssa', new_layout(config, scene.actors, scene.performers))
        put_file(rom, base + '.ssb', compile_script(config, scene.source, film_path))
        plays.append(f"    supervision_ExecuteActingSub({scene.level}, '{name.upper()}', 0);\n")
        print(f'  scene {number}: {base} ({len(scene.actors)} actors)')
    return plays


def edit_unionall(rom, config, edit):
    """Decompile the game's common script, change its text, compile it back
    (as SkyTemple's script editor does)."""
    path = 'SCRIPT/COMMON/unionall.ssb'
    string_codec.init()
    # The decompiler talks a lot about the story's tangled branches: quiet.
    logging.disable(logging.WARNING)
    with open(os.devnull, 'w') as quiet, contextlib.redirect_stdout(quiet), contextlib.redirect_stderr(quiet), warnings.catch_warnings():
        warnings.simplefilter('ignore')
        ssb = FileType.SSB.deserialize(rom.getFileByName(path), config)
        source, _ = ssb.to_explorerscript()
    logging.disable(logging.NOTSET)
    rom.setFileByName(path, compile_script(config, edit(source), 'unionall.exps'))


def main():
    args = sys.argv[1:]
    patch = None
    if '--patch' in args:
        at = args.index('--patch')
        patch = args[at + 1]
        del args[at:at + 2]
    boot = '--boot' in args
    if boot:
        args.remove('--boot')
    if len(args) != (3 if boot else 4):
        print(__doc__)
        return 2
    rom_path, film_path, out = args[0], args[1], args[-1]
    with open(rom_path, 'rb') as file:
        original = file.read()
    rom = NintendoDSRom(original)
    config = get_ppmdu_config_for_rom(rom)
    region = original[0xF:0x10].decode('ascii')
    with open(film_path, encoding='utf-8') as file:
        scenes = scenes_of(file.read())
    for name, species in cast(config, scenes).items():
        actor = config.script_data.level_entities__by_name[name]
        poke = set_species(rom, config, region, actor.id, species)
        print(f'ACTOR_{name}: species {species} (for a savestate: emu.py "1:{poke}")')
    if boot:
        plays = add_scenes(rom, config, scenes, film_path)

        def edit(source):
            if BOOT_EVENT not in source:
                raise SystemExit('unionall: the new game event was not found')
            return source.replace(BOOT_EVENT, BOOT_EVENT + ''.join(plays), 1)

        edit_unionall(rom, config, edit)
        what = 'plays when a new game starts'
    else:
        map_name, scene_name = args[2].split('/')
        base = f'SCRIPT/{map_name}/{scene_name}'
        first, rest = scenes[0], scenes[1:]
        rom.setFileByName(base + '.ssa', replaced_layout(rom, config, base + '.ssa', first.actors, first.performers))
        rom.setFileByName(base + '.ssb', compile_script(config, first.source, film_path))
        if rest:
            plays = add_scenes(rom, config, rest, film_path)
            after = re.compile(rf"( *)supervision_ExecuteActingSub\(LEVEL_\w+, '{scene_name.upper()}', 0\);\n")

            def edit(source):
                found = after.search(source)
                if not found:
                    raise SystemExit(f'unionall: no event plays {scene_name}')
                return after.sub(lambda m: m.group(0) + ''.join(plays), source)

            edit_unionall(rom, config, edit)
        what = f'replaces {args[2]}'
    rom.saveToFile(out)
    print(f'{out}: the film ({len(scenes)} scenes) {what}')
    if patch:
        with open(out, 'rb') as file:
            target = file.read()
        made = bps(original, target)
        if apply_bps(original, made) != target:
            print(f'{patch}: the patch made does not give the ROM back; not written')
            return 1
        with open(patch, 'wb') as file:
            file.write(made)
        print(f'{patch}: {len(made)} bytes, checked')
    return 0


if __name__ == '__main__':
    sys.exit(main())
