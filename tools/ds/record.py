"""Record what the original game shows in the emulator (py-desmume, no
window) as an mp4, to share what plays on DS.

    python tools/ds/record.py <rom> <keys> <out.mp4> [--screens both|bottom|top]
        [--from N] [--to N] [--state savestate.ds] [--scale 2]

- `<keys>`: the key presses, in emu.py's form: `frame:Button` or
  `first-last:Button`, comma-separated, and `mashA:<period>` to press A
  every <period> frames (dialogue needs A). Example: `mashA:90`.
- `--from`/`--to`: the frames kept (the frames before `--from` are still
  played, unrecorded). `--to` defaults to the last frame named in `<keys>`
  (needed with `mashA`).
- `--screens`: both screens (top above bottom, 256x384), or one (256x192).
- `--scale`: whole-pixel enlargement (2 by default), sharp like the DS.
- `--state`: a savestate to start from; without it, a cold boot.
- No sound: py-desmume's library exposes no audio samples (only a volume
  setting; its sound goes to the SDL device), so the video is silent.

ffmpeg is looked for in $FFMPEG, then C:/ffmpeg-*/bin, then the PATH. Work on
a COPY of the ROM; the video shows the user's game: never commit it.
"""
import glob
import os
import shutil
import subprocess
import sys

from desmume.controls import keymask
from desmume.emulator import DeSmuME, Language

from emu import BUTTONS, parse

FPS = 60  # the DS draws 59.83 frames a second


def ffmpeg():
    found = os.environ.get('FFMPEG') or next(iter(sorted(glob.glob('C:/ffmpeg-*/bin/ffmpeg.exe'))), None)
    return found or shutil.which('ffmpeg') or sys.exit('ffmpeg not found: set FFMPEG')


def start(rom, state=None):
    emu = DeSmuME()
    emu.set_language(getattr(Language, os.environ.get('EMU_LANG', 'ENGLISH')))
    emu.open(rom)
    if state:
        emu.savestate.load_file(state)
    emu.volume_set(0)
    return emu


def play(emu, steps, last, on_frame):
    """Run frames 1..last, holding the buttons of `steps`; on_frame(frame)
    after each."""
    for frame in range(1, last + 1):
        held = {BUTTONS[action] for first, end, action in steps if first <= frame < end and action in BUTTONS}
        for key in BUTTONS.values():
            (emu.input.keypad_add_key if key in held else emu.input.keypad_rm_key)(keymask(key))
        emu.cycle(False)
        on_frame(frame)


def record(rom, keys, out, screens='both', first=1, last=None, state=None, scale=2):
    steps = parse(keys)
    if not last:
        if 'mashA' in keys:
            sys.exit('mashA presses A for ever: give --to')
        last = max(end for _, end, _ in steps)
    crop = {'both': (0, 0, 256, 384), 'top': (0, 0, 256, 192), 'bottom': (0, 192, 256, 384)}[screens]
    size = (crop[2] - crop[0], crop[3] - crop[1])
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    command = [ffmpeg(), '-y', '-loglevel', 'error', '-f', 'rawvideo', '-pix_fmt', 'rgb24',
               '-s', f'{size[0]}x{size[1]}', '-r', str(FPS), '-i', '-',
               '-vf', f'scale=iw*{scale}:ih*{scale}:flags=neighbor', '-c:v', 'libx264',
               '-pix_fmt', 'yuv420p', '-crf', '18', out]
    encoder = subprocess.Popen(command, stdin=subprocess.PIPE)
    emu = start(rom, state)

    def on_frame(frame):
        if frame >= first:
            encoder.stdin.write(emu.screenshot().crop(crop).convert('RGB').tobytes())

    play(emu, steps, last, on_frame)
    encoder.stdin.close()
    encoder.wait()
    print(f'{out}: frames {first}-{last}, {(last - first + 1) / FPS:.1f}s, {screens} screen(s), silent')
    return encoder.returncode


def main():
    args = sys.argv[1:]
    options = {'--screens': 'both', '--from': '1', '--to': None, '--state': None, '--scale': '2'}
    for name in list(options):
        if name in args:
            at = args.index(name)
            options[name] = args[at + 1]
            del args[at:at + 2]
    if len(args) != 3:
        print(__doc__)
        return 2
    rom, keys, out = args
    return record(rom, keys, out, options['--screens'], int(options['--from']),
                  int(options['--to']) if options['--to'] else None, options['--state'],
                  int(options['--scale']))


if __name__ == '__main__':
    sys.exit(main())
