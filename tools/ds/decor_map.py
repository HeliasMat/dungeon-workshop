"""A place of the Workshop's own (a decor) as a map of Explorers of Sky's,
for `patch_rom.py`.

`scene_player export-ds` writes each decor a film's scenes are in beside
the export: its lower and upper layers as 8-bit indexed PNGs (index =
palette * 16 + colour, colour 0 clear, each tile of 8 of one palette, the
16 palettes shared by both layers), its walls by tiles, and a line

    // ds-export map LEVEL_DECOR_<n> <name> <width> <height> <lower.png> <upper.png or -> <walls.txt>

Here the game's files are made of them with skytemple_files (BPC: the
tiles and chunks; BPL: the palettes; BMA: the chunks laid, the collision),
added under `MAP_BG/` (`DWMAP0.bpc`, ...), and a level the story never
plays (`SPARE_LEVELS`) is pointed at them in `MAP_BG/bg_list.dat`: the
scenes' `LEVEL_DECOR_<n>` becomes that level. Nothing of the story's
places is changed.
"""
import os

from PIL import Image
from skytemple_files.common.types.file_types import FileType
from skytemple_files.graphics.bg_list_dat.handler import BgListDatHandler

# Levels of the game no story scene plays, each with a map of its own (no
# other level shares its bg_list entry) and two layers (map type 6, as the
# story's towns): the debug levels T00P04A and T00P04A2 (no script names
# them), then T00P03 and T00P02, only named by the debug maps' own scripts.
SPARE_LEVELS = ['T00P04A', 'T00P04A2', 'T00P03', 'T00P02']

# What the game's maps hold at most (the most found in Sky's own): tiles of
# a BPC layer (a tile's number is 10 bits on the DS, the null tile with
# them), chunks of a layer (more than any map of Sky's: unproven), size.
MAX_TILES = 1024
MAX_CHUNKS = 917
MAX_SIZE_CHUNKS = 60 * 40
TILES_IN_CHUNK = 9
CHUNK = 24


def maps_of(source):
    """The export's `// ds-export map` lines: placeholder, name, width, height, files."""
    maps = []
    for line in source.splitlines():
        words = line.split()
        if words[:3] == ['//', 'ds-export', 'map']:
            placeholder, name, width, height, lower, upper, walls = words[3:10]
            maps.append({'placeholder': placeholder, 'name': name, 'width': int(width), 'height': int(height),
                         'lower': lower, 'upper': None if upper == '-' else upper, 'walls': walls})
    return maps


def picture(folder, name):
    """An indexed picture of the export, its 256 colours kept as they are."""
    image = Image.open(os.path.join(folder, name))
    if image.mode != 'P':
        raise SystemExit(f'{name}: not an indexed picture (export it again with scene_player export-ds)')
    colours = (image.getpalette() or [])[:256 * 3]
    image.putpalette(colours + [0] * (256 * 3 - len(colours)))
    return image


def made(rom, config, number, made, folder):
    """One decor's map made the game's files, for the `number`-th spare
    level: {path in the ROM: its bytes} (`MAP_BG/dwmap<number>.bpc`, .bpl,
    .bma). The level's own map is the start (its layers' count, its
    chunks' size)."""
    level = {level.name: level for level in config.script_data.level_list}[SPARE_LEVELS[number]]
    bg = BgListDatHandler.deserialize(rom.getFileByName('MAP_BG/bg_list.dat'))
    width, height = made['width'], made['height']
    if width % CHUNK or height % CHUNK:
        raise SystemExit(f"{made['name']}: {width}x{height} pixels, not whole chunks of {CHUNK} (export it again)")
    wide, high = width // CHUNK, height // CHUNK
    if wide * high > MAX_SIZE_CHUNKS:
        raise SystemExit(f"{made['name']}: {wide}x{high} chunks, larger than any map of Sky's ({MAX_SIZE_CHUNKS} chunks, 1440x960 pixels)")
    old = bg.level[level.mapid]
    bma, bpc, bpl = old.get_bma(rom), old.get_bpc(rom), old.get_bpl(rom)
    lower = picture(folder, made['lower'])
    upper = picture(folder, made['upper']) if made['upper'] else None
    if upper is None:
        bma.remove_upper_layer()
        bpc.remove_upper_layer()
    else:
        bma.add_upper_layer()
        bpc.add_upper_layer()
    # (No animated tiles: their counts in each layer gone.)
    for layer in bpc.layers:
        layer.bpas = [0, 0, 0, 0]
    bma.resize(wide, high, wide * 3, high * 3)
    bma.from_pil(bpc, bpl, lower, upper, force_import=True)
    if bpl.has_palette_animation:
        bpl.has_palette_animation = False
    for at, layer in enumerate(bpc.layers):
        which = 'upper' if upper is not None and at == 0 else 'lower'
        if len(layer.tiles) > MAX_TILES:
            raise SystemExit(f"{made['name']}: {len(layer.tiles)} different tiles on its {which} layer, the DS has room for {MAX_TILES}: "
                             'make the decor smaller or of more repeated pieces')
        chunks = layer.chunk_tilemap_len
        if chunks > MAX_CHUNKS:
            raise SystemExit(f"{made['name']}: {chunks} different chunks on its {which} layer, more than any map of Sky's ({MAX_CHUNKS})")
        print(f"  {made['name']}: {which} layer, {len(layer.tiles)} tiles, {chunks} chunks")
    with open(os.path.join(folder, made['walls']), encoding='utf-8') as file:
        rows = [row.strip() for row in file if row.strip()]
    walls = [cell == '1' for row in rows for cell in row]
    if len(walls) != wide * 3 * high * 3:
        raise SystemExit(f"{made['walls']}: {len(walls)} tiles, the map has {wide * 3 * high * 3}")
    bma.number_of_collision_layers = 1
    bma.collision = walls
    bma.collision2 = None
    name = f'dwmap{number}'
    print(f"  {made['name']}: level {level.name}, {wide}x{high} chunks, {bpl.number_palettes} palettes (MAP_BG/{name}.*)")
    return {f'MAP_BG/{name}.bpc': FileType.BPC.serialize(bpc), f'MAP_BG/{name}.bpl': FileType.BPL.serialize(bpl),
            f'MAP_BG/{name}.bma': FileType.BMA.serialize(bma)}


def put(rom, config, maps, folder):
    """Each map into the ROM, on a spare level of its own: {placeholder: LEVEL_x}."""
    if not maps:
        return {}
    if len(maps) > len(SPARE_LEVELS):
        raise SystemExit(f'{len(maps)} decors in one film: the DS export has {len(SPARE_LEVELS)} spare levels for them')
    from skytemple_files.common.util import create_file_in_rom
    levels = {level.name: level for level in config.script_data.level_list}
    entry_of = BgListDatHandler.get_entry_model_cls()
    renamed = {}
    for number, one in enumerate(maps):
        files = made(rom, config, number, one, folder)
        for path, data in files.items():
            if rom.filenames.idOf(path) is None:
                create_file_in_rom(rom, path, data)
            else:
                rom.setFileByName(path, data)
        level = levels[SPARE_LEVELS[number]]
        bg = BgListDatHandler.deserialize(rom.getFileByName('MAP_BG/bg_list.dat'))
        name = f'DWMAP{number}'
        bg.set_level(level.mapid, entry_of(name, name, name, [None] * 8))
        rom.setFileByName('MAP_BG/bg_list.dat', BgListDatHandler.serialize(bg))
        renamed[one['placeholder']] = f'LEVEL_{level.name}'
    return renamed
