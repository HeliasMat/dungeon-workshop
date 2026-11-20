"""Put a film exported by `scene_player export-ds` in place of a cutscene of
the user's own Explorers of Sky ROM, and write the patched copy.

    python tools/ds/patch_rom.py <rom> <film.exps> <MAP/scene> <out.nds> [--patch out.bps]

- `<MAP/scene>`: the acting scene replaced, like `G01P01B/m02a0101` (the
  first scene of chapter 2, played right after the chapter's banner);
- the film's own lines (`// ds-export ...`) say who stands in it: those are
  written into the scene's layout (its `.ssa`), and a Pokémon of the film the
  game has no actor for takes one of the unused test actors (`NPC_TEST001`
  to `NPC_TEST010`), its species changed in the game's actor table;
- `--patch`: also a BPS patch from the ROM to the copy (a writer of our own,
  below), to share without sharing the game.

Never run it on the original cartridge dump: it only reads `<rom>` and
writes `<out.nds>`. Nothing it writes may be committed.
"""
import sys
import zlib

from ndspy.rom import NintendoDSRom
from skytemple_files.common import string_codec
from skytemple_files.common.types.file_types import FileType
from skytemple_files.common.util import get_ppmdu_config_for_rom
from skytemple_files.script.ssa_sse_sss.actor import SsaActor
from skytemple_files.script.ssa_sse_sss.layer import SsaLayer
from skytemple_files.script.ssa_sse_sss.position import SsaPosition
from skytemple_files.script.ssb.script_compiler import ScriptCompiler

# The script engine's actor table in arm9 (pmdsky-debug's ENTITIES): 386
# entries of 12 bytes (kind, species, name, two unknowns).
# Keyed by the game code's last letter: P Europe, E North America, J Japan.
ENTITIES = {'P': 0x20A8890, 'E': 0x20A7FF0, 'J': 0x20A9438}
ENTRY = 12


def directives(source):
    """The `// ds-export` lines of the film: the place, then each actor."""
    level, actors = None, []
    for line in source.splitlines():
        words = line.split()
        if words[:2] != ['//', 'ds-export']:
            continue
        if words[2] == 'level':
            level = words[3]
        elif words[2] == 'actor':
            fields = dict(zip(words[4::2], words[5::2]))
            actors.append((words[3], fields))
    return level, actors


def tile(value):
    """A position in tiles of 8 pixels, halves allowed: (tiles, offset)."""
    whole = int(float(value))
    return whole, (2 if float(value) - whole >= 0.5 else 0)


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


def layout(rom, config, path, actors):
    """The scene's layout with only the film's actors, on its first layer."""
    data = config.script_data
    ssa = FileType.SSA.deserialize(rom.getFileByName(path), data)
    directions = {d.name.upper(): d.ssa_id for d in data.directions.values()}
    placed = []
    for name, fields in actors:
        actor = data.level_entities__by_name[name.removeprefix('ACTOR_')]
        x, x_offset = tile(fields['x'])
        y, y_offset = tile(fields['y'])
        facing = directions[fields.get('facing', 'DIR_DOWN').removeprefix('DIR_').upper()]
        position = SsaPosition(data, x, y, x_offset, y_offset, facing)
        placed.append(SsaActor(data, actor.id, position, -1, -1))
    ssa.layer_list = [SsaLayer.new(placed, [], [], [], [])]
    return FileType.SSA.serialize(ssa)


def compile_film(config, source, path):
    string_codec.init()
    ssb, _ = ScriptCompiler(config).compile_explorerscript(source, path)
    return FileType.SSB.serialize(ssb, config)


# --- BPS (beat's patch format): a writer of our own, for a ROM rebuilt with
# its files moved by a few blocks. Unchanged stretches are found where they
# were, or 512-byte blocks further (ndspy aligns files so).

def number(value):
    out = bytearray()
    while True:
        part = value & 0x7F
        value >>= 7
        if value == 0:
            out.append(0x80 | part)
            return bytes(out)
        out.append(part)
        value -= 1


def same_run(source, target, s, t, limit):
    """How many bytes match from source[s] and target[t]."""
    run, step = 0, 4096
    while run < limit:
        size = min(step, limit - run)
        if source[s + run:s + run + size] == target[t + run:t + run + size]:
            run += size
            continue
        if size == 1:
            break
        step = max(1, size // 2)
    return run


def bps(source, target):
    source, target = memoryview(source), memoryview(target)
    index = {}
    for at in range(0, len(source) - 64, 512):
        index.setdefault(bytes(source[at:at + 64]), at)
    out = bytearray(b'BPS1' + number(len(source)) + number(len(target)) + number(0))
    t, literal_from, source_at, copy_at = 0, None, 0, 0

    def flush_literal(end):
        nonlocal literal_from
        if literal_from is not None:
            out.extend(number(((end - literal_from) - 1) << 2 | 1))
            out.extend(target[literal_from:end])
            literal_from = None

    while t < len(target):
        best, where = 0, None
        # Where it was, as the last copy went on, or at the same place.
        for s in (source_at, t):
            if 0 <= s < len(source):
                limit = min(len(source) - s, len(target) - t)
                run = same_run(source, target, s, t, limit)
                if run > best:
                    best, where = run, s
        if best < 32 and t % 512 == 0 and t + 64 <= len(target):
            s = index.get(bytes(target[t:t + 64]))
            if s is not None:
                run = same_run(source, target, s, t, min(len(source) - s, len(target) - t))
                if run > best:
                    best, where = run, s
        if best >= 32:
            flush_literal(t)
            if where == t:
                out.extend(number((best - 1) << 2 | 0))  # SourceRead
            else:
                relative = where - copy_at
                out.extend(number((best - 1) << 2 | 2))  # SourceCopy
                out.extend(number((abs(relative) << 1) | (1 if relative < 0 else 0)))
                copy_at = where + best
            source_at = where + best
            t += best
            continue
        if literal_from is None:
            literal_from = t
        # On to the next block boundary, where a moved file may start.
        t = min(len(target), (t // 512 + 1) * 512) if best == 0 else t + max(best, 1)
    flush_literal(len(target))
    out.extend(zlib.crc32(source).to_bytes(4, 'little'))
    out.extend(zlib.crc32(target).to_bytes(4, 'little'))
    out.extend(zlib.crc32(out).to_bytes(4, 'little'))
    return bytes(out)


def main():
    args = sys.argv[1:]
    patch = None
    if '--patch' in args:
        at = args.index('--patch')
        patch = args[at + 1]
        del args[at:at + 2]
    if len(args) != 4:
        print(__doc__)
        return 2
    rom_path, film_path, scene, out = args
    with open(rom_path, 'rb') as file:
        original = file.read()
    rom = NintendoDSRom(original)
    config = get_ppmdu_config_for_rom(rom)
    region = original[0xF:0x10].decode('ascii')
    with open(film_path, encoding='utf-8') as file:
        source = file.read()
    level, actors = directives(source)
    for name, fields in actors:
        if 'species' in fields:
            actor = config.script_data.level_entities__by_name[name.removeprefix('ACTOR_')]
            poke = set_species(rom, config, region, actor.id, fields['species'])
            print(f'{name}: species {fields["species"]} (for a savestate: emu.py "1:{poke}")')
    map_name, scene_name = scene.split('/')
    base = f'SCRIPT/{map_name}/{scene_name}'
    rom.setFileByName(base + '.ssa', layout(rom, config, base + '.ssa', actors))
    rom.setFileByName(base + '.ssb', compile_film(config, source, film_path))
    rom.saveToFile(out)
    print(f'{out}: {scene} replaced ({len(actors)} actors, place {level})')
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


def apply_bps(source, patch):
    """The patched file: to check what `bps` writes."""
    at = 4

    def read_number():
        nonlocal at
        value, shift = 0, 1
        while True:
            byte = patch[at]
            at += 1
            value += (byte & 0x7F) * shift
            if byte & 0x80:
                return value
            shift <<= 7
            value += shift

    read_number()
    target = bytearray(read_number())
    metadata = read_number()
    at += metadata
    out, source_at, target_at = 0, 0, 0
    while at < len(patch) - 12:
        data = read_number()
        kind, length = data & 3, (data >> 2) + 1
        if kind == 0:
            target[out:out + length] = source[out:out + length]
        elif kind == 1:
            target[out:out + length] = patch[at:at + length]
            at += length
        elif kind == 2:
            offset = read_number()
            source_at += -(offset >> 1) if offset & 1 else offset >> 1
            target[out:out + length] = source[source_at:source_at + length]
            source_at += length
        else:
            offset = read_number()
            target_at += -(offset >> 1) if offset & 1 else offset >> 1
            for i in range(length):
                target[out + i] = target[target_at + i]
            target_at += length
        out += length
    return bytes(target)


if __name__ == '__main__':
    sys.exit(main())
