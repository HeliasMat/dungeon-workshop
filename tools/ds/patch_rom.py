"""Put a film exported by `scene_player export-ds` into the user's own
Explorers of Sky ROM, and write the patched copy.

    python tools/ds/patch_rom.py <rom> <film.exps> --boot <out.nds> [--more-actors] [--patch out.bps]
    python tools/ds/patch_rom.py <rom> <film.exps> <MAP/scene> <out.nds> [--more-actors] [--patch out.bps]

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
  scene's layout (`.ssa`); an actor line ending `layer n` goes in layer n,
  which its script loads later (`supervision_Acting(n)`): Sky has 21 actors
  at once, and a scene shows more over its course this way.
- A Pokémon the game has no actor for (`species` given) takes a free actor:
  the ten test actors and the story's actors that no scene places nor any
  script names (FREE_ACTORS, 45 in all), its species changed in the game's
  actor table. The exporter may name such an actor `ACTOR_NPC_TEST001` or any
  placeholder (`ACTOR_FREE07`): it is renamed here.
- A Pokémon SpriteCollab adds (a form, or one of a later generation: the
  export's `// ds-export monster` lines, its sheets beside the export) is
  put in the ROM as a monster of its own, on one of the list's 34 spare
  entries, its sprite and portraits made from those sheets
  (`tools/ds/add_monsters.py`), and the film's actors of it are of that
  monster. One that can't be is told and drawn as its base species or the
  stand-in (Unown "?", its name said in its lines, no portrait). A species
  neither the ROM has (an entry of `BALANCE/monster.md`) nor put in is
  refused, never let alias another Pokémon.
- The place's things (`// ds-export object OBJECT_N<number> x .. y ..`)
  are objects of the game put in the layout; the script names them by
  number and they are renamed here to the game's names.
- `--more-actors`: any number of Pokémon of their own. SkyTemple's
  ActorAndLevelLoader patch (assembled by armips: `SKYTEMPLE_ARMIPS_EXEC`
  names armips.exe) moves the game's actor table to
  `BALANCE/actor_list.bin`; each Pokémon without an actor then gets a new
  actor there (`NPC_DW<n>`), none of the story's touched. Beyond 45 the
  exporter names them `ACTOR_NPC_FREE<n>`, which only this option casts.
- A scene in a decor of the Workshop's (`// ds-export map LEVEL_DECOR_<n> ...`,
  its pictures and walls beside the export): made a map of the game's and
  put on a level the story never plays (`tools/ds/decor_map.py`), the
  scenes' `LEVEL_DECOR_<n>` renamed to it.
- `--patch`: also a BPS patch from the ROM to the copy, to share without
  sharing the game.
- `--dungeon <dungeon.json>` (with `--boot`): after the film, the game's
  team goes into a dungeon of the Workshop, exported by `scene_player
  export-ds <file.dungeon>`, and its boss fight (`tools/ds/dungeon.py`).
  `--boss`: straight into the fight. The film may be `-` (none).

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
from skytemple_files.common.ppmdu_config.script_data import Pmd2ScriptEntity
from skytemple_files.common.util import create_file_in_rom, create_folder_in_rom, get_ppmdu_config_for_rom
from skytemple_files.list.actor.model import ActorListBin
from skytemple_files.script.ssa_sse_sss.actor import SsaActor
from skytemple_files.script.ssa_sse_sss.layer import SsaLayer
from skytemple_files.script.ssa_sse_sss.performer import SsaPerformer
from skytemple_files.script.ssa_sse_sss.model import Ssa
from skytemple_files.script.ssa_sse_sss.object import SsaObject
from skytemple_files.script.ssa_sse_sss.position import SsaPosition
from skytemple_files.script.ssb.script_compiler import ScriptCompiler

from bps import apply_bps, bps
import add_monsters
import decor_map
import dungeon

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

# Who stands in for a Pokémon that couldn't be put in (Unown "?", as the export's).
STAND_IN = 228

# The new game's first event: the personality quiz, played right after the
# title. --boot plays the film before it.
BOOT_EVENT = 'coro EVENT_M00A_01 {\n    bgm_Stop();\n'


class Scene:
    """One scene of the film: its place, who stands in it, its script."""

    def __init__(self, level):
        self.level = level
        self.actors = []  # (name, fields)
        self.performers = []  # fields: type, x, y (the camera, the weather)
        self.objects = []  # (name, fields): the place's things, objects of the game
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
        if words[:3] == ['//', 'ds-export', 'object'] and scenes:
            scenes[-1].objects.append((words[3], dict(zip(words[4::2], words[5::2]))))
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


def more_actors(rom, config, scenes):
    """SkyTemple's ActorAndLevelLoader, then a new actor of its own for each
    Pokémon without one, renamed in the scripts. Returns {actor: species}."""
    from skytemple_files.patch.patches import Patcher
    if not os.environ.get('SKYTEMPLE_ARMIPS_EXEC'):
        raise SystemExit('--more-actors: set SKYTEMPLE_ARMIPS_EXEC to armips.exe (SkyTemple assembles its patch with it)')
    patcher = Patcher(rom, config)
    if not patcher.is_applied('ActorAndLevelLoader'):
        patcher.apply('ActorAndLevelLoader')
    path = 'BALANCE/actor_list.bin'
    actors = FileType.SIR0.unwrap_obj(FileType.SIR0.deserialize(rom.getFileByName(path)), ActorListBin)
    like = next(entry for entry in actors.list if entry.name == 'NPC_TEST001')
    wanted = {}
    for scene in scenes:
        for name, fields in scene.actors:
            if 'species' in fields:
                if wanted.setdefault(name, int(fields['species'])) != int(fields['species']):
                    raise SystemExit(f'{name}: two species in one film')
    renamed = {}
    for name, species in wanted.items():
        new = f'NPC_DW{len(actors.list):03d}'
        actors.list.append(Pmd2ScriptEntity(len(actors.list), species, new, like.type, like.unk3, like.unk4))
        renamed[name] = new
    rom.setFileByName(path, FileType.SIR0.serialize(FileType.SIR0.wrap_obj(actors)))
    config.script_data.level_entities = list(actors.list)
    for scene in scenes:
        text = scene.source
        for old, new in renamed.items():
            text = re.sub(rf'\bACTOR_{old}\b', f'ACTOR_{new}', text)
        scene.lines = text.splitlines()
        scene.actors = [(renamed.get(name, name), fields) for name, fields in scene.actors]
    return {renamed[name]: species for name, species in wanted.items()}


def own_species(rom):
    """How many monsters the ROM has of its own (`BALANCE/monster.md`'s entries)."""
    return len(FileType.MD.deserialize(rom.getFileByName('BALANCE/monster.md')).entries)


def check_species(known, scenes, added=()):
    """Every species the film gives an actor is one the ROM has (`known`:
    its own entries of `BALANCE/monster.md`, before any were added) or one
    put in (`added`): past them the game would show some other Pokémon in
    its place, so it is refused, saying which."""
    for scene in scenes:
        for name, fields in scene.actors:
            if 'species' in fields and not (0 < int(fields['species']) < known or int(fields['species']) in added):
                raise SystemExit(
                    f'{name}: species {fields["species"]} is not in this ROM (its species are 1 to {known - 1}): '
                    'the game would draw another Pokémon; export the film again (a form is drawn as its base species, '
                    'a Pokémon Sky lacks as a stand-in)')


def put_monsters(rom, config, scenes, monsters):
    """The Pokémon SpriteCollab adds put in the ROM (`add_monsters.add`);
    the actors of each of the ROM's monster made of it, or of the one drawn
    instead (the stand-in's lines then say its name, with no portrait).
    The species put in."""
    given = add_monsters.add(rom, config, monsters)
    names = {one.species: one.name for one in monsters}
    for scene in scenes:
        text = scene.source
        for name, fields in scene.actors:
            asked = int(fields.get('species', 0))
            if asked not in given:
                continue
            fields['species'] = str(given[asked])
            if given[asked] == STAND_IN:
                said = re.compile(rf'message_Set(?:Face|Actor)\(ACTOR_{name}\b[^;]*\);(\s*)message_(Talk|Monologue)\("')
                text = said.sub(lambda m: f'message_ResetActor();{m.group(1)}message_{m.group(2)}(" {names[asked]}:', text)
        scene.lines = text.splitlines()
    return {number for asked, number in given.items() if number != next(one.instead for one in monsters if one.species == asked)}


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


def object_names(config, scene):
    """The exporter names an object by its number (`OBJECT_N186`): the
    game's name for it (pmd2data's, as the script compiler knows them)."""
    objects = config.script_data.objects__by_id
    text = scene.source
    for name, _ in scene.objects:
        number = int(name.removeprefix('OBJECT_N'))
        if number not in objects:
            raise SystemExit(f'{name}: no object {number} in the game')
        text = re.sub(rf'\b{name}\b', 'OBJECT_' + objects[number].unique_name.upper(), text)
    scene.lines = text.splitlines()


def layer(config, actors, performers=(), objects=()):
    """A layer of the scene: the film's actors and things, where they first stand."""
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
    things = []
    for name, fields in objects:
        x, x_offset = tile(fields['x'])
        y, y_offset = tile(fields['y'])
        position = SsaPosition(data, x, y, x_offset, y_offset, directions['DOWN'])
        things.append(SsaObject(data, int(name.removeprefix('OBJECT_N')), 1, 1, position, -1, -1))
    return SsaLayer.new(placed, things, shown, [], [])


def layers(config, actors, performers=(), objects=()):
    """The scene's layers: the first holds the camera, the things and who
    stands there from the start; an actor's `layer n` puts it in layer n,
    which the script loads later (`supervision_Acting(n)`), when a slot of
    the game's 24 is free again."""
    count = 1 + max((int(fields.get('layer', 0)) for _, fields in actors), default=0)
    return [layer(config, [one for one in actors if int(one[1].get('layer', 0)) == n],
                  performers if n == 0 else (), objects if n == 0 else ()) for n in range(count)]


def replaced_layout(rom, config, path, actors, performers=(), objects=()):
    """A story scene's layout with only the film's actors and things."""
    ssa = FileType.SSA.deserialize(rom.getFileByName(path), config.script_data)
    ssa.layer_list = layers(config, actors, performers, objects)
    return FileType.SSA.serialize(ssa)


def new_layout(config, actors, performers=(), objects=()):
    return FileType.SSA.serialize(Ssa.new(config.script_data, layers(config, actors, performers, objects), [], []))


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


def add_scenes(rom, config, scenes, film_path, prefix='dwscen'):
    """Each scene as an acting scene of its own; the lines playing them."""
    plays = []
    for number, scene in enumerate(scenes, 1):
        place = scene.level.removeprefix('LEVEL_')
        name = f'{prefix}{number:02d}'
        base = f'SCRIPT/{place}/{name}'
        put_file(rom, base + '.ssa', new_layout(config, scene.actors, scene.performers, scene.objects))
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


def boss_place(scenes):
    """Where the boss scenes are played: the film's last place, around
    where its view was (else Beach Cave Pit's, where Sky's first boss talks)."""
    if not scenes:
        return 'LEVEL_D01P41A', (28, 22)
    last = scenes[-1]
    spots = [(float(f['x']), float(f['y'])) for f in last.performers] or [(float(f['x']), float(f['y'])) for _, f in last.actors]
    if not spots:
        return last.level, (32, 24)
    return last.level, (round(sum(x for x, _ in spots) / len(spots)), round(sum(y for _, y in spots) / len(spots)))


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
    extra = '--more-actors' in args
    if extra:
        args.remove('--more-actors')
    dungeon_json = None
    if '--dungeon' in args:
        at = args.index('--dungeon')
        dungeon_json = args[at + 1]
        del args[at:at + 2]
    boss_first = '--boss' in args
    if boss_first:
        args.remove('--boss')
    if dungeon_json and not boot:
        raise SystemExit('--dungeon: with --boot only')
    if len(args) != (3 if boot else 4):
        print(__doc__)
        return 2
    rom_path, film_path, out = args[0], args[1], args[-1]
    with open(rom_path, 'rb') as file:
        original = file.read()
    rom = NintendoDSRom(original)
    config = get_ppmdu_config_for_rom(rom)
    region = original[0xF:0x10].decode('ascii')
    if film_path == '-':
        scenes = []
        if not dungeon_json:
            raise SystemExit('no film (-): only with --dungeon')
    else:
        with open(film_path, encoding='utf-8') as file:
            source = file.read()
        scenes = scenes_of(source)
        # The decors of the Workshop's the scenes are in: maps of their own, on spare levels.
        for old, new in decor_map.put(rom, config, decor_map.maps_of(source), os.path.dirname(os.path.abspath(film_path))).items():
            for scene in scenes:
                scene.lines = [re.sub(rf'\b{old}\b', new, line) for line in scene.lines]
                if scene.level == old:
                    scene.level = new
    boss = []
    if dungeon_json:
        dump = dungeon.load(dungeon_json)
        for lost in dungeon.put(rom, config, dump):
            print(f'  lost: {lost}')
        boss = [scene for text in dungeon.boss_scenes(dump, *boss_place(scenes)) for scene in scenes_of(text)]
    known = own_species(rom)
    monsters = add_monsters.monsters_of(source, os.path.dirname(os.path.abspath(film_path))) if scenes else []
    added = set()
    if monsters:
        added = put_monsters(rom, config, scenes + boss, monsters)
    check_species(known, scenes + boss, added)
    if extra:
        for name, species in more_actors(rom, config, scenes + boss).items():
            print(f'ACTOR_{name}: species {species} (a new actor)')
    for name, species in ({} if extra else cast(config, scenes + boss)).items():
        actor = config.script_data.level_entities__by_name[name]
        poke = set_species(rom, config, region, actor.id, species)
        print(f'ACTOR_{name}: species {species} (for a savestate: emu.py "1:{poke}")')
    for scene in scenes:
        object_names(config, scene)
    if boot:
        plays = add_scenes(rom, config, scenes, film_path)
        if dungeon_json:
            fight = [line.strip() for line in add_scenes(rom, config, boss, 'dungeon.py', 'dwboss')]
            before, after = (fight[:1], fight[1:]) if fight else ([], [])
            plays += dungeon.team_lines() + dungeon.enter_lines(dungeon.BOSS_OF if boss_first else dungeon.FLOORS_OF)

        def edit(source):
            if BOOT_EVENT not in source:
                raise SystemExit('unionall: the new game event was not found')
            source = source.replace(BOOT_EVENT, BOOT_EVENT + ''.join(plays), 1)
            if dungeon_json:
                source = dungeon.getout_edit(source, before, after)
            return source

        edit_unionall(rom, config, edit)
        what = 'plays when a new game starts'
    else:
        map_name, scene_name = args[2].split('/')
        base = f'SCRIPT/{map_name}/{scene_name}'
        first, rest = scenes[0], scenes[1:]
        rom.setFileByName(base + '.ssa', replaced_layout(rom, config, base + '.ssa', first.actors, first.performers, first.objects))
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
