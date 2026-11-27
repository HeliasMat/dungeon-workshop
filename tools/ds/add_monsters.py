"""Put the Pokémon SpriteCollab adds, that a film casts, in a copy of the
user's own Explorers of Sky ROM: each a monster of the ROM's own, its name,
its sprite made from SpriteCollab's sheets, its portraits.

`scene_player export-ds` writes, beside the film's `.exps`, each such
Pokémon's sheets in SpriteBot's layout (`<film>.exps.monster<species>/`:
`sprite/AnimData.xml`, `sprite/<Anim>-Anim.png` and `-Shadow.png`,
`portrait/<Face>.png`) and a line `// ds-export monster <species>
<instead> <folder> <name>`; `patch_rom.py` calls `add` with them.

How, with SkyTemple's own code (skytemple_files, GPL-3):

- the room: the monster list's spare entries, which the game never shows
  and which have names of their own: 537 to 551 (named "??????????") and
  581 to 599 (`reserve_27` ... `reserve_45`), 34 in all; an entry is taken
  only while its name is still that one. (SkyTemple's ExpandPokeList,
  which grows the list to 2,048, is far too slow for each patch: see
  tools/ds/LIMITS.md.)
- a monster: that entry of `BALANCE/monster.md`, given the numbers of its
  base species (a form) or Pikachu's (a Pokémon of its own), its name in
  every language's text (`MESSAGE/text_*.str`);
- its sprite: SpriteCollab's sheets read as SkyTemple's monster sprite
  import does (`FileType.WAN.CHARA.import_sheets`, then `split_wan`), the
  three parts added to `MONSTER/monster.bin`, `m_ground.bin` (the scenes')
  and `m_attack.bin`, its sizes in the game's table
  (`check_and_correct_monster_sprite_size`); its scenes' part's size told
  to `data/extra/ds_ground_sizes.txt` (`ground_sizes.record`), for the
  export to count it in a scene's room;
- its portraits: each face, 40x40, in `FONT/kaomado.kao` (brought to 16
  colours as SkyTemple does).

A Pokémon that can't be put in (sheets that don't read, no spare entry
left) is told and drawn as `<instead>`: its base species, or the stand-in.

    python tools/ds/add_monsters.py <rom> <film.exps> <out.nds>

only puts them in (to look at them in SkyTemple); patch_rom.py does it
with the film. Work on a COPY: it only reads <rom>.
"""
import os
import sys

from PIL import Image

import ground_sizes

from skytemple_files.common.sprite_util import check_and_correct_monster_sprite_size
from skytemple_files.common.types.file_types import FileType
from skytemple_files.common.util import MONSTER_BIN, get_binary_from_rom, get_files_from_rom_with_extension, set_binary_in_rom
from skytemple_files.hardcoded.monster_sprite_data_table import HardcodedMonsterSpriteDataTable

GROUND_BIN = 'MONSTER/m_ground.bin'
ATTACK_BIN = 'MONSTER/m_attack.bin'
MD = 'BALANCE/monster.md'
KAO = 'FONT/kaomado.kao'

# The monster list's spare entries: never shown by the game, a name of their own.
SPARE = list(range(537, 552)) + list(range(581, 600))
# A Pokémon of its own is given Pikachu's numbers (speed, size, gender...).
LIKE = 25
STAND_IN = 228
# The faces of the portraits, in the ROM's order (each face's own at 2n, its mirror at 2n + 1).
FACES = ['Normal', 'Happy', 'Pain', 'Angry', 'Worried', 'Sad', 'Crying', 'Shouting', 'Teary-Eyed', 'Determined',
         'Joyous', 'Inspired', 'Surprised', 'Dizzy', 'Special0', 'Special1', 'Sigh', 'Stunned', 'Special2', 'Special3']
# Each monster entry's fields (but its place in the list and its own number).
FIELDS = ['unk31', 'national_pokedex_number', 'base_movement_speed', 'gender', 'body_size', 'type_primary',
          'type_secondary', 'movement_type', 'iq_group', 'ability_primary', 'ability_secondary', 'bitfield1_0',
          'bitfield1_1', 'bitfield1_2', 'bitfield1_3', 'bitfield1_5', 'can_evolve', 'can_move', 'item_required_for_spawning',
          'exp_yield', 'recruit_rate1', 'base_hp', 'recruit_rate2', 'base_atk', 'base_sp_atk', 'base_def',
          'base_sp_def', 'weight', 'size', 'shadow_size', 'chance_spawn_asleep', 'hp_regeneration', 'unk21_h']


class Monster:
    """A Pokémon of the export's: its number there, who it is drawn as
    where it can't be put in, its sheets' folder, its name."""

    def __init__(self, species, instead, folder, name):
        self.species, self.instead, self.folder, self.name = int(species), int(instead), folder, name


def monsters_of(source, beside):
    """The export's `// ds-export monster` lines; `beside`: the export's folder."""
    found = []
    for line in source.splitlines():
        words = line.split(maxsplit=6)
        if words[:3] == ['//', 'ds-export', 'monster'] and len(words) == 7:
            found.append(Monster(words[3], words[4], os.path.join(beside, words[5]), words[6]))
    return found


def texts(rom, config):
    """Every language's text: {path: its strings}."""
    return {path: FileType.STR.deserialize(rom.getFileByName(path), string_encoding=config.string_encoding)
            for path in get_files_from_rom_with_extension(rom, 'str')}


def spare(config, strings):
    """The spare entries still free (their name still the game's placeholder)."""
    names = config.string_index_data.string_blocks['Pokemon Names'].begin
    english = next((text for path, text in strings.items() if path.endswith('_e.str')), next(iter(strings.values())))
    placeholder = lambda name: name.startswith('reserve_') or set(name) <= {'?'}
    return [number for number in SPARE if placeholder(english.strings[names + number])]


def with_offsets(folder):
    """SkyTemple reads an `<Anim>-Offsets.png` beside each sheet (where the
    head and the hands are): an empty one where the export wrote none
    (each frame's middle is then taken for them)."""
    sprite = os.path.join(folder, 'sprite')
    for file in os.listdir(sprite):
        if not file.endswith('-Anim.png'):
            continue
        offsets = os.path.join(sprite, file.replace('-Anim.png', '-Offsets.png'))
        if not os.path.exists(offsets):
            Image.new('RGBA', Image.open(os.path.join(sprite, file)).size, (0, 0, 0, 0)).save(offsets)
    return sprite


def made_sprite(folder):
    """The sheets as the three parts of a sprite of the ROM's (the
    dungeons', the scenes', the attacks'), ready for their packs."""
    wan = FileType.WAN.CHARA.import_sheets(with_offsets(folder))
    monster, ground, attack = FileType.WAN.CHARA.split_wan(wan)
    compressed = lambda part: FileType.PKDPX.serialize(FileType.PKDPX.compress(FileType.WAN.CHARA.serialize(part)))
    return compressed(monster), FileType.WAN.CHARA.serialize(ground), compressed(attack)


def put_faces(kao, folder, number):
    """Each face of the folder's at the monster's place in the portraits. How many."""
    put = 0
    for face, name in enumerate(FACES):
        path = os.path.join(folder, 'portrait', f'{name}.png')
        if os.path.exists(path):
            kao.set_from_img(number - 1, face * 2, Image.open(path).convert('RGB').crop((0, 0, 40, 40)))
            put += 1
    return put


def add(rom, config, monsters, tell=print, made=made_sprite):
    """Each monster put in the ROM: {its export's number: the ROM's}, the
    one drawn instead for those that couldn't be. `made`: a sheets'
    folder made a sprite (`ds_cache.py` keeps each one made)."""
    if not monsters:
        return {}
    strings = texts(rom, config)
    free = spare(config, strings)
    md = FileType.MD.deserialize(rom.getFileByName(MD))
    kao = FileType.KAO.deserialize(rom.getFileByName(KAO))
    packs = {path: FileType.BIN_PACK.deserialize(rom.getFileByName(path)) for path in (MONSTER_BIN, GROUND_BIN, ATTACK_BIN)}
    arm9 = bytearray(get_binary_from_rom(rom, config.bin_sections.arm9))
    sizes = HardcodedMonsterSpriteDataTable.get(arm9, config)
    names = config.string_index_data.string_blocks['Pokemon Names'].begin
    categories = config.string_index_data.string_blocks['Pokemon Categories'].begin
    given = {}
    for one in monsters:
        try:
            if not free:
                raise RuntimeError(f'no spare monster entry left ({len(SPARE)} a ROM)')
            if not os.path.isdir(os.path.join(one.folder, 'sprite')):
                raise RuntimeError(f'no sheets at {one.folder}')
            parts = made(one.folder)
            # (Its scenes' sprite as the game loads it: what the export counts in a scene's room.)
            ground_sizes.record(one.species, len(parts[1]))
            number = free.pop(0)
            # (Packs of unequal lengths, as SkyTemple mends them: empty sprites.)
            sprite = max(len(pack) for pack in packs.values())
            for (path, pack), part in zip(packs.items(), parts):
                while len(pack) < sprite:
                    pack.append(b'')
                pack.append(part)
            entry = md.entries[number]
            like = md.entries[one.instead if one.instead not in (0, STAND_IN) else LIKE]
            for field in FIELDS:
                if hasattr(like, field):
                    setattr(entry, field, getattr(like, field))
            # (Its own number: the game finds its name and its faces by it; no evolution.)
            entry.entid = number
            entry.base_form_index = number
            entry.pre_evo_index = 0
            entry.evo_method = 0
            entry.sprite_index = sprite
            check_and_correct_monster_sprite_size(entry, md_gender1=entry, md_gender2=None, monster_bin=packs[MONSTER_BIN],
                                                  m_attack_bin=packs[ATTACK_BIN], sprite_size_table=sizes)
            faces = put_faces(kao, one.folder, number)
            for text in strings.values():
                text.strings[names + number] = one.name[:10]
                text.strings[categories + number] = text.strings[categories + (like.md_index % 600)]
            given[one.species] = number
            size = sum(len(part) for part in parts)
            tell(f'  {one.name} (species {one.species}): monster {number} of the ROM, sprite {sprite} ({size // 1024} KB), {faces} faces')
        except Exception as why:  # (Its sheets, SkyTemple's import: drawn instead, told.)
            tell(f'  {one.name} (species {one.species}) could not be put in ({type(why).__name__}: {why}): drawn as species {one.instead} instead')
            given[one.species] = one.instead
    for path, pack in packs.items():
        rom.setFileByName(path, FileType.BIN_PACK.serialize(pack))
    for path, text in strings.items():
        rom.setFileByName(path, FileType.STR.serialize(text))
    HardcodedMonsterSpriteDataTable.set(sizes, arm9, config)
    set_binary_in_rom(rom, config.bin_sections.arm9, bytes(arm9))
    rom.setFileByName(MD, FileType.MD.serialize(md))
    rom.setFileByName(KAO, FileType.KAO.serialize(kao))
    return given


def main():
    if len(sys.argv) != 4:
        print(__doc__)
        return 2
    from ndspy.rom import NintendoDSRom
    from skytemple_files.common.util import get_ppmdu_config_for_rom
    rom_path, film_path, out = sys.argv[1:]
    rom = NintendoDSRom.fromFile(rom_path)
    config = get_ppmdu_config_for_rom(rom)
    with open(film_path, encoding='utf-8') as file:
        monsters = monsters_of(file.read(), os.path.dirname(os.path.abspath(film_path)))
    for species, number in add(rom, config, monsters).items():
        print(f'species {species}: {number}')
    rom.saveToFile(out)
    return 0


if __name__ == '__main__':
    sys.exit(main())
