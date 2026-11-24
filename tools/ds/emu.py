"""Development helper: run the ORIGINAL game in DeSmuME (py-desmume), without
a window, to look at what it really shows.

    python tools/ds/emu.py <rom> <savestate or -> <out prefix> <script>

The script is a comma-separated list of `frame:action` or `first-last:action`
steps: an action is a button (A, B, X, Y, L, R, Start, Select, Up, Down,
Left, Right), `shot` (saves both screens to
`<out prefix>_<frame>.png`, top above bottom), `save` (writes a savestate to
`<out prefix>_<frame>.ds`), `vars` (writes the script engine's variables,
by name, to `<out prefix>_<frame>_vars.txt`: the story's progress, the
dungeon entered, how it ended...), `mem=<hex address>=<length>` (prints that
memory in hexadecimal), `poke=<hex address>=<hex bytes>` (writes them in
memory: what a patched arm9 holds, for a savestate made before the patch)
or `quit`. `mashA:<period>` in place of a frame range presses A every
<period> frames for the whole run.

Work on COPIES of the ROM and savestate (the emulator may write a battery
file next to the ROM). Nothing captured may be committed.
"""
import os
import sys

from desmume.controls import Keys, keymask
from desmume.emulator import DeSmuME, Language

BUTTONS = {
    'A': Keys.KEY_A, 'B': Keys.KEY_B, 'X': Keys.KEY_X, 'Y': Keys.KEY_Y,
    'L': Keys.KEY_L, 'R': Keys.KEY_R, 'Start': Keys.KEY_START, 'Select': Keys.KEY_SELECT,
    'Up': Keys.KEY_UP, 'Down': Keys.KEY_DOWN, 'Left': Keys.KEY_LEFT, 'Right': Keys.KEY_RIGHT,
}


# The table defining the script engine's variables and the one holding their
# values, by region (pmdsky-debug's SCRIPT_VARS and SCRIPT_VARS_VALUES).
SCRIPT_VARS = {'P': (0x209DDF4, 0x22AB9EC), 'E': (0x209D870, 0x22AB0AC), 'J': (0x209EC44, 0x22AC868)}
VARIABLE_COUNT = 115
TAB, NEWLINE = chr(9), chr(10)


def read(emu, address, length):
    return bytes(emu.memory.unsigned[address:address + length])


def number(data, signed=False):
    return int.from_bytes(data, 'little', signed=signed)


def script_variables(emu, region):
    """Every script variable of the running game: (name, [values])."""
    definitions, values = SCRIPT_VARS[region]
    found = []
    for index in range(VARIABLE_COUNT):
        entry = read(emu, definitions + index * 16, 16)
        kind, offset, shift, count = (number(entry[i:i + 2]) for i in (0, 4, 6, 8))
        name = read(emu, number(entry[12:16]), 40).split(bytes(1))[0].decode('ascii', 'replace')
        base = values + offset
        # Kinds: 1 a bit, 2 and 3 a byte, 4 a signed byte, 5 and 6 two bytes,
        # 7 and 8 four; 9 is computed by the game (not read here).
        width = {2: 1, 3: 1, 4: 1, 5: 2, 6: 2, 7: 4, 8: 4}.get(kind)
        if kind == 1:
            bits = [(read(emu, base + ((shift + i) >> 3), 1)[0] >> ((shift + i) & 7)) & 1 for i in range(count)]
            found.append((index, name, bits))
        elif width:
            signed = kind in (4, 6, 8)
            found.append((index, name, [number(read(emu, base + i * width, width), signed) for i in range(count)]))
        else:
            found.append((index, name, ['(computed)']))
    return found


def parse(script):
    steps = []
    for step in script.split(','):
        frames, action = step.strip().split(':')
        if frames == 'mashA':
            steps += [(f, f + 2, 'A') for f in range(1, 200000, int(action))]
            continue
        if '-' in frames:
            first, last = (int(part) for part in frames.split('-'))
        else:
            first, last = int(frames), int(frames) + 1
        steps.append((first, last, action))
    return steps


def main():
    if len(sys.argv) != 5:
        print(__doc__)
        return 2
    rom, state, prefix, script = sys.argv[1:5]
    with open(rom, 'rb') as file:
        region = file.read(0x10)[0xF:0x10].decode('ascii')
    steps = parse(script)
    emu = DeSmuME()
    # EMU_LANG picks the console's language (default French); EMU_SAVE is a
    # DeSmuME battery file (.dsv) to start from, when booting without a
    # savestate.
    emu.set_language(getattr(Language, os.environ.get('EMU_LANG', 'FRENCH')))
    emu.open(rom)
    if os.environ.get('EMU_SAVE'):
        emu.backup.import_file(os.environ['EMU_SAVE'])
        emu.reset()
    if state != '-':
        emu.savestate.load_file(state)
    emu.volume_set(0)
    final = ('shot', 'save', 'quit', 'vars')
    last = max(end for _, end, action in steps if action in final or action.startswith('mem='))
    for frame in range(1, last + 1):
        held = set()
        for first, end, action in steps:
            if not first <= frame < end:
                continue
            if action == 'shot':
                emu.screenshot().save(f'{prefix}_{frame}.png')
            elif action == 'save':
                emu.savestate.save_file(f'{prefix}_{frame}.ds')
            elif action == 'vars':
                with open(f'{prefix}_{frame}_vars.txt', 'w', encoding='utf-8') as file:
                    for index, name, values in script_variables(emu, region):
                        # Long lists of bits: only those set.
                        if len(values) > 8:
                            values = [f'{i}' for i, value in enumerate(values) if value] or ['-']
                            name += ' (indexes set)'
                        file.write(f"{index}" + TAB + name + TAB + " ".join(str(v) for v in values) + NEWLINE)
            elif action.startswith('mem='):
                _, address, length = action.split('=')
                print(f'{frame}: {address} = {read(emu, int(address, 16), int(length)).hex(" ")}')
            elif action.startswith('poke='):
                _, address, data = action.split('=')
                for offset, value in enumerate(bytes.fromhex(data)):
                    emu.memory.unsigned[int(address, 16) + offset] = value
            elif action == 'quit':
                return
            else:
                held.add(BUTTONS[action])
        for name, key in BUTTONS.items():
            if key in held:
                emu.input.keypad_add_key(keymask(key))
            else:
                emu.input.keypad_rm_key(keymask(key))
        emu.cycle(False)


if __name__ == '__main__':
    main()
