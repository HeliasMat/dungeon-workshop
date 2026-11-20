"""Print what an acting scene of the ROM holds: its actors, objects,
performers and position marks (the .ssa), for choosing which scene a film
replaces.

    python tools/ds/inspect_scene.py <rom> <map> <scene>
    python tools/ds/inspect_scene.py <rom> --actors <word>

Example: `inspect_scene.py rom.nds G01P01B m02a0101`. Prints to the
terminal only; nothing it prints may be committed (it is the game's data).
"""
import sys

from ndspy.rom import NintendoDSRom
from skytemple_files.common.util import get_ppmdu_config_for_rom
from skytemple_files.common.types.file_types import FileType


def main():
    rom = NintendoDSRom.fromFile(sys.argv[1])
    config = get_ppmdu_config_for_rom(rom)
    if sys.argv[2] == '--actors':
        word = sys.argv[3].upper()
        for entity in config.script_data.level_entities:
            if word in entity.name.upper():
                print(entity.id, entity.name, 'entid', entity.entid, 'type', entity.type)
        return 0
    level, scene = sys.argv[2], sys.argv[3]
    path = f'SCRIPT/{level}/{scene}.ssa'
    ssa = FileType.SSA.deserialize(rom.getFileByName(path), config.script_data)
    for index, layer in enumerate(ssa.layer_list):
        print('layer', index)
        for actor in layer.actors:
            print('  actor', actor.actor.id, actor.actor.name, 'at', actor.pos.x_relative, actor.pos.y_relative,
                  actor.pos.x_offset, actor.pos.y_offset, actor.pos.direction.name if actor.pos.direction else None,
                  'script', actor.script_id)
        for obj in layer.objects:
            print('  object', obj.object.name, 'at', obj.pos.x_relative, obj.pos.y_relative, 'script', obj.script_id)
        for performer in layer.performers:
            print('  performer type', performer.type, 'at', performer.pos.x_relative, performer.pos.y_relative)
        for event in layer.events:
            print('  event trigger', event.trigger_id, 'at', event.pos.x_relative, event.pos.y_relative)
    for mark in ssa.position_markers:
        print('mark at', mark.pos.x_relative, mark.pos.y_relative)
    return 0


if __name__ == '__main__':
    sys.exit(main())
