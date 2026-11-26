"""Put the items a film shows (held, given, thrown, eaten, set down,
following a path) in a copy of the user's own Explorers of Sky ROM, as
objects of the game's scenes.

Sky's cutscenes show things as objects (its object table: a name, a
sprite `GROUND/<name>.wan`), placed in a scene's layout and told by the
script (`SetPositionMark<object ...>`, `MovePositionMark`, `MoveHeight`,
`SetOutputAttribute`, routines `def n for object`). An item has no object
of its own: the export (`scene_player export-ds`) casts each item it shows
as one of `SLOTS`, objects of the table no scene of the story places nor
any script names, and writes a line

    // ds-export item OBJECT_N<slot> item <item id>
    // ds-export item OBJECT_N<slot> picture <file beside the export>

for each; `sprites` makes each slot's sprite (`GROUND/<name>.wan`) from
Sky's own item icon (its 16 x 16 picture in `DUNGEON/dungeon.bin`, its
palette by `BALANCE/item_p.bin`) or from a picture of the film's own (a PNG
brought to 16 colours), with SkyTemple's static sprite maker (skytemple_rust's
`pmd_wan.encode_image_to_static_wan_file`, as SkyTemple's "import image"
of an object). The same slot keeps its sprite in every scene of the film.

Never run on the original cartridge dump: only copies are written. Nothing
it makes may be committed.
"""
import os

# Objects of Sky's table (EU) that no scene's layout places nor any script
# names, each with a sprite file of its own: the items' slots. Found by
# reading every layout and script of the game (`GROUND/<name>.wan` exists,
# the name is the entry's alone, and its third field is 0).
SLOTS = (11, 66, 97, 99, 154, 155, 189, 205, 221, 252, 253, 254, 255, 317, 373, 375,
         391, 396, 406, 412, 433, 449, 454, 484, 485, 486, 487, 546)
# The room a slot's sprite is given in the fast copy's base (bytes).
ROOM = 8 * 1024


def slot_file(config, slot):
    """The sprite file of a slot's object."""
    return f'GROUND/{config.script_data.objects__by_id[slot].name}.wan'


def items_of(source):
    """The film's items: [(slot, 'item' | 'picture', id or file)]."""
    found = []
    for line in source.splitlines():
        words = line.split()
        if words[:3] == ['//', 'ds-export', 'item'] and len(words) >= 6:
            slot = int(words[3].removeprefix('OBJECT_N'))
            if slot not in SLOTS:
                raise SystemExit(f'{words[3]}: not one of the items\' slots')
            found.append((slot, words[4], ' '.join(words[5:])))
    return found


def icon(rom, config, item):
    """Sky's icon of an item, a picture of 16 colours (index 0 clear)."""
    from PIL import Image
    from skytemple_files.common.types.file_types import FileType
    items = FileType.ITEM_P.deserialize(rom.getFileByName('BALANCE/item_p.bin'))
    entry = items.item_list[item]
    dungeon = FileType.DUNGEON_BIN.deserialize(rom.getFileByName('DUNGEON/dungeon.bin'), config)
    pictures = dungeon.get('items.itm.img')
    drawn = pictures.to_pil(entry.sprite, entry.palette)
    picture = Image.new('P', drawn.size)
    picture.putdata([index % 16 for index in drawn.getdata()])
    colours = list(pictures.palettes[entry.palette])[:48]
    picture.putpalette(colours + [0] * (768 - len(colours)))
    return picture


def picture_of(path):
    """A picture of the film's own brought to 16 colours, index 0 clear."""
    from PIL import Image
    image = Image.open(path).convert('RGBA')
    if image.width > 64 or image.height > 64:
        image.thumbnail((64, 64), Image.NEAREST)
    solid = Image.new('RGB', image.size, (0, 0, 0))
    solid.paste(image, mask=image.split()[3])
    few = solid.quantize(15, method=Image.Quantize.MEDIANCUT)
    clear = [alpha < 128 for alpha in image.split()[3].getdata()]
    picture = Image.new('P', image.size)
    picture.putdata([0 if hidden else index + 1 for index, hidden in zip(few.getdata(), clear)])
    colours = (few.getpalette() or [])[:45]
    picture.putpalette([0, 0, 0] + colours + [0] * (768 - 3 - len(colours)))
    return picture


def sprite(picture):
    """A still sprite of the game's (a WAN of one picture) of it."""
    from skytemple_rust import pmd_wan
    return bytes(pmd_wan.encode_image_to_static_wan_file(picture))


def sprites(rom, config, items, beside):
    """Each item's slot's sprite file: {path: bytes}."""
    made = {}
    for slot, kind, what in items:
        if kind == 'item':
            picture = icon(rom, config, int(what))
        elif kind == 'picture':
            picture = picture_of(os.path.join(beside, what))
        else:
            raise SystemExit(f'// ds-export item: {kind}? (item or picture)')
        made[slot_file(config, slot)] = sprite(picture)
    return made
