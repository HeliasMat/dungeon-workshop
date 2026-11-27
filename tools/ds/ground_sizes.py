"""Write the table of each species' ground sprite size that `scene_player
export-ds` reads to know whether a scene's sprites fit the room the DS
has for them (`data/extra/ds_ground_sizes.txt`, never committed: it is
read from one's own ROM).

    python tools/ds/ground_sizes.py <rom> [out.txt]

In ground mode Sky loads each species a scene shows once, its sprite of
`MONSTER/m_ground.bin` decompressed (LoadWanTableEntryFromPack), into its
ground arenas (tools/ds/LIMITS.md, "Species in a scene"). So, for every
species of `BALANCE/monster.md`: `species<TAB>bytes`, the size of its
sprite as loaded (a PKDPX's decompressed size, else the entry's own).
The Pokémon SpriteCollab adds are told by `add_monsters.py` as it makes
their sprites (`record`), and kept when the table is written again.
Numbers only: no game data.
"""
import os
import struct
import sys

from ndspy.rom import NintendoDSRom
from skytemple_files.common.types.file_types import FileType

# The species past Sky's own: those SpriteCollab adds (film_core's SKY_SPECIES).
SKY_SPECIES = 1155
HEADER = '# species<TAB>bytes: each ground sprite as Sky loads it (tools/ds/ground_sizes.py, add_monsters.py)\n'


def table_path():
    """Where the export reads it: the data's `extra/`."""
    data = os.environ.get('DUNGEON_DATA') or os.path.join(os.path.dirname(__file__), '..', '..', 'data')
    return os.path.join(data, 'extra', 'ds_ground_sizes.txt')


def read(path):
    """The table as it is: {species: bytes}."""
    sizes = {}
    if os.path.isfile(path):
        with open(path, encoding='utf-8') as file:
            for line in file:
                fields = line.strip().split('\t')
                if len(fields) == 2 and not line.startswith('#'):
                    sizes[int(fields[0])] = int(fields[1])
    return sizes


def write(path, sizes):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w', newline='\n', encoding='utf-8') as file:
        file.write(HEADER)
        file.write(''.join(f'{species}\t{bytes_}\n' for species, bytes_ in sorted(sizes.items())))


def record(species, bytes_, path=None):
    """A Pokémon SpriteCollab adds: its sprite's size, once it is made."""
    path = path or table_path()
    sizes = read(path)
    if sizes.get(int(species)) != int(bytes_):
        sizes[int(species)] = int(bytes_)
        write(path, sizes)


def loaded(packed):
    """An entry of m_ground.bin as the game loads it: a PKDPX decompressed."""
    if packed[:5] == b'PKDPX':
        return struct.unpack_from('<I', packed, 0x10)[0]
    return len(packed)


def main():
    if len(sys.argv) not in (2, 3):
        print(__doc__)
        return 2
    out = sys.argv[2] if len(sys.argv) == 3 else table_path()
    rom = NintendoDSRom.fromFile(sys.argv[1])
    md = FileType.MD.deserialize(rom.getFileByName('BALANCE/monster.md'))
    raw = rom.getFileByName('MONSTER/m_ground.bin')
    count = struct.unpack_from('<I', raw, 4)[0]
    entries = [struct.unpack_from('<II', raw, 8 + 8 * at) for at in range(count)]
    sizes = {species: bytes_ for species, bytes_ in read(out).items() if species >= SKY_SPECIES}
    for species, entry in enumerate(md.entries):
        if entry.sprite_index < count:
            offset, length = entries[entry.sprite_index]
            if length:
                sizes[species] = loaded(raw[offset:offset + length])
    write(out, sizes)
    print(f'{out}: {len(sizes)} species')
    return 0


if __name__ == '__main__':
    sys.exit(main())
