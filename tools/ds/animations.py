"""Write the table of each species' ground animations that `scene_player
export-ds` reads to pick, for every Pokémon, the group of its own sprite
that shows a pose (`data/extra/ds_animations.txt`, never committed).

    python tools/ds/animations.py <rom> [out.txt]

In ground mode Sky plays a sprite's animation *group* by its number
(`SetAnimation(n)` goes through overlay 11's SETANIMATION_TABLE to a
group, with no per-species lookup: `ChangeActorAnimation`). The groups
past 12 are each sprite's own: the starters and a few others share one
order (13 EventSleep, 14 Wake ... 33 Faint, 34 HitGround), but most story
characters have their own (Dusknoir's faint is 25, Grovyle lies down with
20, Wigglytuff's 13 is a roll). A group the sprite lacks shows it standing.

So, for every species whose groups 13-34 are not the shared order, one
line: `species<TAB>group=Name,group=Name...` with the groups its sprite
has (in m_ground.bin, monster.bin or m_attack.bin: the game merges
them), named after SkyTemple's `pmd2spritedata.xml` (GPL-3). A species
with the shared order has no line. Numbers only: no game text.
"""
import os
import sys
import xml.etree.ElementTree as ET

from ndspy.rom import NintendoDSRom
import skytemple_files
from skytemple_files.common.types.file_types import FileType
from skytemple_files.container.sir0.handler import Sir0Handler
from skytemple_files.graphics.chara_wan.model import WanFile

SHARED = ('EventSleep Wake Eat Tumble Pose Pull Pain Float DeepBreath Nod Sit LookUp Sink Trip '
          'Laying LeapForth Head Cringe LostBalance TumbleBack Faint HitGround').split()


def names():
    path = os.path.join(os.path.dirname(skytemple_files.__file__), '_resources', 'ppmdu_config', 'pmd2spritedata.xml')
    game = ET.parse(path).getroot().find('AnimationNames/Game')
    return {int(sprite.get('id')): {int(index.get('id')): [name.text for name in index.findall('Name')]
                                    for index in sprite.findall('Index')}
            for sprite in game.findall('Sprite')}


def filled(pack, index):
    try:
        wan = Sir0Handler.unwrap_obj(Sir0Handler.deserialize(pack[index]), WanFile)
    except Exception:
        return set()
    return {group for group, sequences in enumerate(wan.animGroupData) if len(sequences) > 0}


def main():
    if len(sys.argv) not in (2, 3):
        print(__doc__)
        return 2
    # (Read from one's own ROM: it goes to one's data, never to the repository.)
    data = os.environ.get('DUNGEON_DATA') or os.path.join(os.path.dirname(__file__), '..', '..', 'data')
    out = sys.argv[2] if len(sys.argv) == 3 else os.path.join(data, 'extra', 'ds_animations.txt')
    rom = NintendoDSRom.fromFile(sys.argv[1])
    md = FileType.MD.deserialize(rom.getFileByName('BALANCE/monster.md'))
    packs = [FileType.BIN_PACK.deserialize(rom.getFileByName(f'MONSTER/{name}.bin'))
             for name in ('m_ground', 'monster', 'm_attack')]
    named = names()
    lines, seen = [], {}
    for species, entry in enumerate(md.entries):
        sprite = entry.sprite_index
        if sprite not in seen:
            has = set().union(*(filled(pack, sprite) for pack in packs if sprite < len(pack)))
            own = named.get(sprite, {})
            special = {group: own.get(group, ['?'])[0] for group in sorted(has) if group >= 13}
            shared = all(special.get(13 + at) == name for at, name in enumerate(SHARED))
            seen[sprite] = None if shared else special
        special = seen[sprite]
        if special is not None:
            lines.append(f'{species}\t' + ','.join(f'{group}={name}' for group, name in special.items()))
    with open(out, 'w', newline='\n') as file:
        file.write('# species<TAB>group=name,...: tools/ds/animations.py (names: SkyTemple, GPL-3)\n')
        file.write('\n'.join(lines) + '\n')
    print(f'{out}: {len(lines)} species with a sprite of their own order, of {len(md.entries)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
