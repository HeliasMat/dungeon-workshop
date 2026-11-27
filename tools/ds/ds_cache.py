"""The fast edit -> DS loop: a base copy of the user's ROM made once and
cached, then each film put in a copy of it in about a second.

    python tools/ds/ds_cache.py <rom> <film.exps> <out.nds> [--scene n]
    python tools/ds/ds_cache.py <rom> --base

What `patch_rom.py --boot --more-actors` does to the ROM is mostly the same
for every film: SkyTemple's ActorAndLevelLoader (armips, ~2 s), the new
game's event in `SCRIPT/COMMON/unionall.ssb` (decompiled and compiled back,
~3 s), the Pokémon SpriteCollab adds (~8 s each). The base does all that
once, under `target/ds/cache/<rom>/base/` (keyed by the ROM's hash and these
tools' own text):

- the new game's event plays `SLOTS` acting scenes, `SCRIPT/T00P01/
  dwscen01` to `dwscen64`, after a `jump` to the first one the film uses: a
  film of n scenes is put in the last n slots, and only the jump's target
  changes, two bytes of unionall's compiled script. Every slot's acting
  is on the same level, `SLOT_LEVEL` (a debug map no story scene plays);
  each scene's script sets its own place at once (`back_SetGround`, as the
  export writes it), so it plays where the film says.
- the files a film writes are in the base with room to spare: every
  slot's script and layout, the actor list, the four decor maps
  (`MAP_BG/dwmap0..3`, the spare levels pointed at them). A film's copy is
  the base's bytes with only those files written over, in place, the rest
  of their room zeros: no file of the cartridge moves, and the game's file
  table, which it reads once at power on, stays true.
- `SPARE_ACTORS` actors added to the actor list (`NPC_DW386`...), each of
  species `MARK + n` until a film casts a Pokémon as it.
- the Pokémon SpriteCollab adds: those of every film played so far (34 at
  most), each sprite made once and kept beside its sheets' copy
  (`monsters/<species>-<hash>.sprite`); a film casting one the base lacks
  makes the base again, with it (a second a Pokémon already made).
- a decor map, made once from its pictures (`decors/<hash>.bin`).
- the items' slots' sprites (`add_items.SLOTS`, objects no scene uses), each
  with room for an item's icon or a picture of the film's own.

A ready savestate is taken on the base on the New Game menu of a new
game, before the game reads the event (`watch.py`: py-desmume's
`ready.ds`, a libretro core's `ready.<core>.state`). Loaded over a film's
copy, one press of A plays the film from its first scene. The game read the
actor list at power on, so the film's spare actors' species are written in
memory too: `<out>.pokes` says which (the list found by its marks).

`--scene n`: only the film's scenes from the n-th on (each scene's layout is
where everyone stands at its start, so it plays as in the whole film).

Not for `--dungeon` nor a story scene replaced: `patch_rom.py` does those.
Never on the original cartridge dump: the ROM is only read. Nothing here may
be committed.
"""
import hashlib
import json
import os
import pickle
import re
import shutil
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
CACHE = os.path.join(ROOT, 'target', 'ds', 'cache')

# What the base is made with: a change of these files (or of VERSION) makes it again.
VERSION = 1
TOOLS = ('ds_cache.py', 'patch_rom.py', 'add_monsters.py', 'decor_map.py', 'add_items.py', 'megapatch.py', 'patches.toml', 'cot_build.py')
SLOTS = 64
SLOT_LEVEL = 'T00P01'
SLOT_PREFIX = 'dwscen'
MAX_MONSTERS = 34
# Actors added to the base's list for the films' Pokémon; their species
# until then, which finds the list in a savestate's memory.
SPARE_ACTORS = 64
MARK = 4000
UNIONALL = 'SCRIPT/COMMON/unionall.ssb'
ACTOR_LIST = 'BALANCE/actor_list.bin'
# The room kept for each file a film writes (bytes): a scene's script and
# layout, the actor list (beyond its size), a decor's three files.
ROOM = {'ssb': 48 * 1024, 'ssa': 8 * 1024, 'actors': 16 * 1024, 'bpc': 160 * 1024, 'bpl': 8 * 1024, 'bma': 96 * 1024}

# The controls screen of a new game, in py-desmume, no save file: A at
# these frames (the logos, the title), the state taken at READY_FRAME.
BOOT_PRESSES = range(700, 1000, 40)
READY_FRAME = 1060


def quiet_imports():
    """skytemple_files's decompiler pulls igraph, which pulls matplotlib and
    plotly to draw graphs: none of it is needed here (a second saved)."""
    for name in ('matplotlib', 'plotly'):
        sys.modules.setdefault(name, None)


def file_hash(path):
    digest = hashlib.sha1()
    with open(path, 'rb') as file:
        for block in iter(lambda: file.read(1 << 20), b''):
            digest.update(block)
    return digest.hexdigest()


def rom_key(rom_path):
    """The ROM's hash (12 digits), remembered by its path, size and time."""
    known_path = os.path.join(CACHE, 'roms.json')
    try:
        with open(known_path, encoding='utf-8') as file:
            known = json.load(file)
    except (OSError, ValueError):
        known = {}
    stat = os.stat(rom_path)
    mark = f'{os.path.abspath(rom_path)}|{stat.st_size}|{stat.st_mtime_ns}'
    if mark not in known:
        known[mark] = file_hash(rom_path)[:12]
        os.makedirs(CACHE, exist_ok=True)
        with open(known_path, 'w', encoding='utf-8') as file:
            json.dump(known, file, indent=1)
    return known[mark]


def tools_key():
    import megapatch
    digest = hashlib.sha1(str(VERSION).encode())
    for name in TOOLS:
        with open(os.path.join(HERE, name), 'rb') as file:
            digest.update(file.read())
    # (The mega patch: its choice, DW_PATCHES, and its patches' files.)
    digest.update(megapatch.key().encode())
    return digest.hexdigest()[:10]


def folder_hash(folder):
    """The hash of a folder's files (a Pokémon's sheets)."""
    digest = hashlib.sha1()
    for base, dirs, files in sorted(os.walk(folder)):
        dirs.sort()
        for name in sorted(files):
            if name.endswith('-Offsets.png'):
                continue
            digest.update(os.path.relpath(os.path.join(base, name), folder).encode())
            with open(os.path.join(base, name), 'rb') as file:
                digest.update(file.read())
    return digest.hexdigest()[:12]


class Paths:
    """Where a ROM's cache is."""

    def __init__(self, rom_path):
        self.rom = rom_path
        self.root = os.path.join(CACHE, rom_key(rom_path))
        self.base_dir = os.path.join(self.root, 'base')
        self.base = os.path.join(self.base_dir, 'base.nds')
        self.manifest = os.path.join(self.base_dir, 'base.json')
        self.ready = os.path.join(self.base_dir, 'ready.ds')
        self.monsters = os.path.join(self.root, 'monsters')
        self.decors = os.path.join(self.root, 'decors')

    def read_manifest(self, stale=False):
        """The base's manifest; None if it is stale (made by other tools, its
        copy gone), unless `stale` (the Pokémon it held are kept)."""
        try:
            with open(self.manifest, encoding='utf-8') as file:
                manifest = json.load(file)
        except (OSError, ValueError):
            return None
        if not stale and (manifest.get('tools') != tools_key() or not os.path.isfile(self.base)):
            return None
        return manifest


def keep_sheets(paths, monster):
    """A Pokémon's sheets copied into the cache (an export is written over
    by the next one): the copy's folder."""
    kept = os.path.join(paths.monsters, f'{monster.species}-{folder_hash(monster.folder)}')
    if not os.path.isdir(kept):
        shutil.copytree(monster.folder, kept + '.tmp', dirs_exist_ok=True)
        os.replace(kept + '.tmp', kept)
    return kept


def made_sprite_kept(folder):
    """add_monsters.made_sprite, kept beside the sheets' copy."""
    import add_monsters
    kept = folder.rstrip('/\\') + '.sprite'
    if os.path.isfile(kept):
        with open(kept, 'rb') as file:
            return pickle.load(file)
    parts = add_monsters.made_sprite(folder)
    with open(kept, 'wb') as file:
        pickle.dump(parts, file)
    return parts


def slot_lines():
    """The new game's event, before the quiz: a jump to the film's first
    slot, then every slot in turn, then back to the title screen."""
    lines = ['    jump @dw_slot_end;\n']
    for slot in range(1, SLOTS + 1):
        lines.append(f'    §dw_slot_{slot:02d};\n')
        lines.append(f"    supervision_ExecuteActingSub(LEVEL_{SLOT_LEVEL}, '{SLOT_PREFIX.upper()}{slot:02d}', 0);\n")
    lines.append('    §dw_slot_end;\n')
    import patch_rom
    lines += patch_rom.FILM_END
    return ''.join(lines)


def slot_offsets(data, config):
    """Where, in unionall's compiled script, the jump's target is (a byte
    offset) and the value the jump takes for each slot (and the end)."""
    from skytemple_files.common.types.file_types import FileType
    ssb = FileType.SSB.deserialize(data, config)
    level = next(one.id for one in config.script_data.level_list if one.name == SLOT_LEVEL)
    acting = lambda op: op.op_code.name == 'supervision_ExecuteActingSub' and op.params[0] == level
    for ops in ssb.routine_ops:
        names = [op.op_code.name for op in ops]
        for at, op in enumerate(ops):
            # (The slots: the only actings of that level, one after the other; strings are numbers here.)
            if not all(acting(one) for one in ops[at:at + SLOTS]) or len(ops) < at + SLOTS + 1:
                continue
            jump = ops[at - 1]
            if jump.op_code.name != 'Jump':
                raise SystemExit(f'unionall: no jump before the slots ({names[at - 1]})')
            targets = [ops[at + k].offset for k in range(SLOTS)]
            end = ops[at + SLOTS].offset
            return jump, targets, end
    seen = [repr(op.params) for ops in ssb.routine_ops for op in ops if op.op_code.name == 'supervision_ExecuteActingSub'][-3:]
    raise SystemExit(f'unionall: the slots were not found ({seen})')


def reserved(rom):
    """The files a film writes, and the room each one is given."""
    import add_items
    import decor_map
    from skytemple_files.common.util import get_ppmdu_config_for_rom
    room = {ACTOR_LIST: len(rom.getFileByName(ACTOR_LIST)) + ROOM['actors']}
    # (The items' slots' sprites.)
    config = get_ppmdu_config_for_rom(rom)
    for slot in add_items.SLOTS:
        room[add_items.slot_file(config, slot)] = add_items.ROOM
    for slot in range(1, SLOTS + 1):
        base = f'SCRIPT/{SLOT_LEVEL}/{SLOT_PREFIX}{slot:02d}'
        room[base + '.ssb'] = ROOM['ssb']
        room[base + '.ssa'] = ROOM['ssa']
    for number in range(len(decor_map.SPARE_LEVELS)):
        for kind in ('bpc', 'bpl', 'bma'):
            room[f'MAP_BG/dwmap{number}.{kind}'] = ROOM[kind]
    return room


def build_base(paths, monsters, tell=print):
    """The base copy, its manifest; `monsters`: the add_monsters.Monster
    the base must hold (their sheets kept in the cache)."""
    quiet_imports()
    import add_monsters
    import decor_map
    import patch_rom
    from ndspy.rom import NintendoDSRom
    from skytemple_files.common.types.file_types import FileType
    from skytemple_files.common.util import get_ppmdu_config_for_rom
    from skytemple_files.common.ppmdu_config.script_data import Pmd2ScriptEntity
    from skytemple_files.graphics.bg_list_dat.handler import BgListDatHandler
    from skytemple_files.list.actor.model import ActorListBin
    from skytemple_files.patch.patches import Patcher

    started = time.time()
    if not os.environ.get('SKYTEMPLE_ARMIPS_EXEC'):
        raise SystemExit('the base: set SKYTEMPLE_ARMIPS_EXEC to armips.exe (SkyTemple assembles its patch with it)')
    with open(paths.rom, 'rb') as file:
        rom = NintendoDSRom(file.read())
    config = get_ppmdu_config_for_rom(rom)
    own = patch_rom.own_species(rom)
    patcher = Patcher(rom, config)
    if not patcher.is_applied('ActorAndLevelLoader'):
        patcher.apply('ActorAndLevelLoader')
    # The mega patch: the community's patches for cutscenes (patches.toml).
    import megapatch
    patched = megapatch.apply(rom, config, patcher, tell)
    # The spare actors a film casts its Pokémon as, each of a species no
    # Pokémon has (MARK + n) until a film gives it one.
    actors = FileType.SIR0.unwrap_obj(FileType.SIR0.deserialize(rom.getFileByName(ACTOR_LIST)), ActorListBin)
    like = next(entry for entry in actors.list if entry.name == 'NPC_TEST001')
    first_spare = len(actors.list)
    for spare in range(SPARE_ACTORS):
        actors.list.append(Pmd2ScriptEntity(len(actors.list), MARK + spare, f'NPC_DW{len(actors.list):03d}', like.type, like.unk3, like.unk4))
    rom.setFileByName(ACTOR_LIST, FileType.SIR0.serialize(FileType.SIR0.wrap_obj(actors)))
    config.script_data.level_entities = list(actors.list)
    tell(f'  base: ActorAndLevelLoader ({time.time() - started:.1f} s)')
    # The Pokémon SpriteCollab adds: each from its sheets' copy.
    given = add_monsters.add(rom, config, monsters, tell, made=made_sprite_kept) if monsters else {}
    # The decors' maps: each spare level pointed at a map of its own, as
    # its own map for now (a film's decor only changes what it holds).
    levels = {level.name: level for level in config.script_data.level_list}
    bg = BgListDatHandler.deserialize(rom.getFileByName('MAP_BG/bg_list.dat'))
    entry_of = BgListDatHandler.get_entry_model_cls()
    for number, name in enumerate(decor_map.SPARE_LEVELS):
        level = levels[name]
        old = bg.level[level.mapid]
        new = f'DWMAP{number}'
        for kind, data in (('bpc', FileType.BPC.serialize(old.get_bpc(rom))), ('bpl', FileType.BPL.serialize(old.get_bpl(rom))),
                           ('bma', FileType.BMA.serialize(old.get_bma(rom)))):
            patch_rom.put_file(rom, f'MAP_BG/{new.lower()}.{kind}', data)
        bg.set_level(level.mapid, entry_of(new, new, new, [None] * 8))
    rom.setFileByName('MAP_BG/bg_list.dat', BgListDatHandler.serialize(bg))
    # Every slot, a scene of nothing for now.
    empty_layout = patch_rom.new_layout(config, [])
    empty_script = patch_rom.compile_script(config, 'def 0 {\n    end;\n}\n', 'slot.exps')
    for slot in range(1, SLOTS + 1):
        base = f'SCRIPT/{SLOT_LEVEL}/{SLOT_PREFIX}{slot:02d}'
        patch_rom.put_file(rom, base + '.ssa', empty_layout)
        patch_rom.put_file(rom, base + '.ssb', empty_script)

    def edit(source):
        if patch_rom.BOOT_EVENT not in source:
            raise SystemExit('unionall: the new game event was not found')
        return source.replace(patch_rom.BOOT_EVENT, patch_rom.BOOT_EVENT + slot_lines(), 1)

    patch_rom.edit_unionall(rom, config, edit)
    unionall = rom.getFileByName(UNIONALL)
    jump, targets, end = slot_offsets(unionall, config)
    at = jump_byte(unionall, config, jump, end)
    # The files a film writes, each with room to spare (zeros after it):
    # written in place, nothing else of the cartridge moves.
    room = {}
    for path, size in reserved(rom).items():
        data = rom.getFileByName(path)
        room[path] = max(size, len(data))
        rom.setFileByName(path, data + bytes(room[path] - len(data)))
    os.makedirs(paths.base_dir, exist_ok=True)
    data = rom.save(updateDeviceCapacity=True)
    fat = int.from_bytes(data[0x48:0x4C], 'little')
    files = {}
    for path, size in room.items():
        start = fat + 8 * rom.filenames.idOf(path)
        begin, finish = (int.from_bytes(data[start + i:start + i + 4], 'little') for i in (0, 4))
        if finish - begin != size:
            raise SystemExit(f'{path}: {finish - begin} bytes in the cartridge, {size} written')
        files[path] = [begin, size]
    start = fat + 8 * rom.filenames.idOf(UNIONALL)
    files[UNIONALL] = [int.from_bytes(data[start:start + 4], 'little'), len(unionall)]
    with open(paths.base + '.tmp', 'wb') as file:
        file.write(data)
    os.replace(paths.base + '.tmp', paths.base)
    if os.path.exists(paths.ready):
        os.remove(paths.ready)
    for stale in os.listdir(paths.base_dir):
        if stale.startswith('ready'):
            os.remove(os.path.join(paths.base_dir, stale))
    manifest = {
        'tools': tools_key(), 'own_species': own, 'slots': SLOTS, 'slot_level': SLOT_LEVEL,
        'jump_byte': at, 'slot_targets': targets, 'end_target': end, 'files': files, 'first_spare': first_spare,
        'patches': patched['applied'], 'special_processes': patched['special_processes'],
        'monsters': {str(one.species): {'number': given.get(one.species, one.instead), 'instead': one.instead,
                                        'name': one.name, 'folder': one.folder} for one in monsters},
    }
    with open(paths.manifest, 'w', encoding='utf-8') as file:
        json.dump(manifest, file, indent=1)
    tell(f'  base: {paths.base} ({time.time() - started:.1f} s, {len(monsters)} Pokémon added)')
    return manifest


def jump_byte(data, config, jump, end):
    """The byte offset of the jump's target in unionall's compiled script."""
    value = end.to_bytes(2, 'little')
    found = [at for at in range(0, len(data) - 1, 2) if data[at:at + 2] == value]
    # The jump op: its opcode just before its target.
    opcode = jump.op_code.id.to_bytes(2, 'little')
    candidates = [at for at in found if data[at - 2:at] == opcode]
    if len(candidates) != 1:
        raise SystemExit(f'unionall: the slots\' jump not found once ({len(candidates)} places)')
    return candidates[0]


def ensure_base(rom_path, monsters=(), tell=print):
    """The cache's paths and manifest, the base made (again) if it is
    missing, stale, or lacks one of `monsters`."""
    import add_monsters
    paths = Paths(rom_path)
    manifest = paths.read_manifest()
    wanted = {one.species: one for one in monsters}
    # (The Pokémon a base held, kept when it is made again.)
    held = manifest or paths.read_manifest(stale=True) or {'monsters': {}}
    had = {int(species): add_monsters.Monster(species, one['instead'], one['folder'], one['name'])
           for species, one in held['monsters'].items() if os.path.isdir(one['folder'])}
    if manifest:
        missing = [one for species, one in wanted.items()
                   if species not in had or os.path.basename(had[species].folder) != f'{species}-{folder_hash(one.folder)}']
        if not missing:
            return paths, manifest
        tell(f'  base: made again for {", ".join(one.name for one in missing)}')
    for species, one in wanted.items():
        had[species] = add_monsters.Monster(species, one.instead, keep_sheets(paths, one), one.name)
    chosen = list(had.values())
    if len(chosen) > MAX_MONSTERS:
        chosen = [had[species] for species in wanted]
    return paths, build_base(paths, chosen, tell)


def film_rom(rom_path, film_path, out, scene=None, tell=print):
    """The film put in a copy of the base, written to `out`."""
    quiet_imports()
    started = time.time()
    import add_monsters
    import decor_map
    import patch_rom
    from ndspy.rom import NintendoDSRom
    from skytemple_files.common.util import get_ppmdu_config_for_rom

    with open(film_path, encoding='utf-8') as file:
        source = file.read()
    beside = os.path.dirname(os.path.abspath(film_path))
    monsters = add_monsters.monsters_of(source, beside)
    paths, manifest = ensure_base(rom_path, monsters, tell)
    with open(paths.base, 'rb') as file:
        cartridge = bytearray(file.read())
    rom = NintendoDSRom(cartridge)
    config = get_ppmdu_config_for_rom(rom)
    written = {}
    scenes = patch_rom.scenes_of(source)
    if scene:
        if not 1 <= scene <= len(scenes):
            raise SystemExit(f'--scene {scene}: the film has {len(scenes)} scenes')
        scenes = scenes[scene - 1:]
    if len(scenes) > SLOTS:
        raise SystemExit(f'{len(scenes)} scenes: the fast copy has {SLOTS} (patch_rom.py --boot has no limit)')
    # The decors: their maps made once, kept by their pictures' hash.
    renamed = put_decors(rom, config, paths, decor_map.maps_of(source), beside, written, tell)
    for old, new in renamed.items():
        for one in scenes:
            one.lines = [re.sub(rf'\b{old}\b', new, line) for line in one.lines]
            if one.level == old:
                one.level = new
    # The items shown: each slot's sprite, from Sky's icon or the film's picture.
    import add_items
    written.update(add_items.sprites(rom, config, add_items.items_of(source), beside))
    # The Pokémon SpriteCollab adds: the base's.
    given = {int(species): one['number'] for species, one in manifest['monsters'].items()}
    added = relabel_monsters(scenes, monsters, given)
    patch_rom.check_species(manifest['own_species'], scenes, added)
    cast = cast_spare(rom, config, scenes, manifest)
    for name, (_, species) in cast.items():
        tell(f'ACTOR_{name}: species {species} (a spare actor of the base)')
    written[ACTOR_LIST] = rom.getFileByName(ACTOR_LIST)
    first = SLOTS - len(scenes) + 1
    for number, one in enumerate(scenes):
        patch_rom.object_names(config, one)
        slot = first + number
        base = f'SCRIPT/{SLOT_LEVEL}/{SLOT_PREFIX}{slot:02d}'
        written[base + '.ssa'] = patch_rom.new_layout(config, one.actors, one.performers, one.objects)
        written[base + '.ssb'] = patch_rom.compile_script(config, one.source, film_path)
        tell(f'  scene {number + (scene or 1)}: slot {slot} ({one.level}, {len(one.actors)} actors)')
    # Each file in its place in the cartridge, the rest of its room zeros;
    # the event's jump to the film's first slot.
    for path, data in written.items():
        begin, room = manifest['files'][path]
        if len(data) > room:
            raise SystemExit(f'{path}: {len(data)} bytes, the fast copy has room for {room} (patch_rom.py --boot has no limit)')
        cartridge[begin:begin + room] = data + bytes(room - len(data))
    begin = manifest['files'][UNIONALL][0] + manifest['jump_byte']
    target = manifest['slot_targets'][first - 1] if scenes else manifest['end_target']
    cartridge[begin:begin + 2] = target.to_bytes(2, 'little')
    with open(out, 'wb') as file:
        file.write(cartridge)
    # What a savestate of the base holds in memory that the film changes:
    # the actor list (read once, at power on), told for the players to write.
    with open(out + '.pokes', 'w', encoding='utf-8', newline='\n') as file:
        file.write(f'# The spare actors\' species, for a ready state of the base: the actor list in memory, found by its\n'
                   f'# spare entries\' species {MARK}, {MARK + 1}, ... (12 bytes an entry, the species at byte 2).\n')
        file.write(f'mark {MARK}\n')
        for spare, species in sorted(cast.values()):
            file.write(f'actor {spare} {species}\n')
    tell(f'{out}: the film ({len(scenes)} scenes) in the cached base, {time.time() - started:.1f} s')
    return paths


def cast_spare(rom, config, scenes, manifest):
    """Each Pokémon of the film without an actor of the game's own cast as
    one of the base's spare actors, the same in every scene, renamed in the
    scripts; its species set in the list. {actor: (spare number, species)}."""
    from skytemple_files.common.types.file_types import FileType
    from skytemple_files.list.actor.model import ActorListBin
    actors = FileType.SIR0.unwrap_obj(FileType.SIR0.deserialize(rom.getFileByName(ACTOR_LIST)), ActorListBin)
    wanted = {}
    for one in scenes:
        for name, fields in one.actors:
            if 'species' in fields and wanted.setdefault(name, int(fields['species'])) != int(fields['species']):
                raise SystemExit(f'{name}: two species in one film')
    if len(wanted) > SPARE_ACTORS:
        raise SystemExit(f'{len(wanted)} Pokémon without an actor of their own: the fast copy has {SPARE_ACTORS} (patch_rom.py --boot has no limit)')
    first = manifest['first_spare']
    renamed, cast = {}, {}
    for spare, (name, species) in enumerate(wanted.items()):
        entry = actors.list[first + spare]
        entry.entid = species
        renamed[name] = entry.name
        cast[entry.name] = (spare, species)
    rom.setFileByName(ACTOR_LIST, FileType.SIR0.serialize(FileType.SIR0.wrap_obj(actors)))
    config.script_data.level_entities = list(actors.list)
    for one in scenes:
        text = one.source
        for old, new in renamed.items():
            text = re.sub(rf'\bACTOR_{old}\b', f'ACTOR_{new}', text)
        one.lines = text.splitlines()
        one.actors = [(renamed.get(name, name), fields) for name, fields in one.actors]
    return cast


def relabel_monsters(scenes, monsters, given):
    """The film's actors of a Pokémon the base holds cast as its monster;
    one drawn instead by the stand-in has its name said in its lines. The
    species put in."""
    names = {one.species: one.name for one in monsters}
    for one in scenes:
        text = one.source
        for name, fields in one.actors:
            asked = int(fields.get('species', 0))
            if asked not in given or asked not in names:
                continue
            fields['species'] = str(given[asked])
            if given[asked] == add_monsters_stand_in():
                said = re.compile(rf'message_Set(?:Face|Actor)\(ACTOR_{name}\b[^;]*\);(\s*)message_(Talk|Monologue)\("')
                text = said.sub(lambda m, asked=asked: f'message_ResetActor();{m.group(1)}message_{m.group(2)}(" {names[asked]}:', text)
        one.lines = text.splitlines()
    insteads = {one.species: one.instead for one in monsters}
    return {number for asked, number in given.items() if asked in insteads and number != insteads[asked]}


def add_monsters_stand_in():
    import add_monsters
    return add_monsters.STAND_IN


def put_decors(rom, config, paths, maps, beside, written, tell):
    """Each decor's map put in its spare level's files of the base:
    {placeholder: LEVEL_x}."""
    import decor_map
    if not maps:
        return {}
    if len(maps) > len(decor_map.SPARE_LEVELS):
        raise SystemExit(f'{len(maps)} decors in one film: the DS export has {len(decor_map.SPARE_LEVELS)} spare levels for them')
    renamed = {}
    for number, made in enumerate(maps):
        digest = hashlib.sha1(f'{decor_map.__file__}|{made["width"]}|{made["height"]}|{number}'.encode())
        for name in (made['lower'], made['upper'], made['walls']):
            if name:
                with open(os.path.join(beside, name), 'rb') as file:
                    digest.update(file.read())
        kept = os.path.join(paths.decors, digest.hexdigest()[:16] + '.bin')
        if os.path.isfile(kept):
            with open(kept, 'rb') as file:
                files = pickle.load(file)
            tell(f"  {made['name']}: its map kept from before")
        else:
            files = decor_map.made(rom, config, number, made, beside)
            os.makedirs(paths.decors, exist_ok=True)
            with open(kept, 'wb') as file:
                pickle.dump(files, file)
        written.update(files)
        renamed[made['placeholder']] = f'LEVEL_{decor_map.SPARE_LEVELS[number]}'
    return renamed


def main():
    args = sys.argv[1:]
    scene = None
    if '--scene' in args:
        at = args.index('--scene')
        scene = int(args[at + 1])
        del args[at:at + 2]
    if len(args) == 2 and args[1] == '--base':
        paths, _ = ensure_base(args[0])
        print(paths.base)
        return 0
    if len(args) != 3:
        print(__doc__)
        return 2
    film_rom(args[0], args[1], args[2], scene)
    return 0


if __name__ == '__main__':
    sys.exit(main())
