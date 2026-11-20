"""Watch a film patched in with `patch_rom.py --boot`, from a cold boot of
the ROM, no savestate: the keys a player presses, then a contact sheet
(and a video if asked).

    python tools/ds/boot_and_watch.py <patched.nds> [out prefix] [--to N]
        [--every N] [--period N] [--video out.mp4]

What a player does, and what the script presses:
1. power on: the logos, then the title screen ("Press Start") at about
   frame 780; A works as Start;
2. with no save file the game goes straight to a new game: a screen of
   the controls, closed with A (about frame 1040);
3. the new game's first event plays the film's scenes, one after the other
   (from about frame 1130); A closes each line of dialogue;
4. after the film, the personality quiz starts as usual ("Welcome!").

So A every `--period` frames (40 by default: quick; 90 leaves time to
read) is the whole key script. With a save file on the cartridge, the
player picks "New Game" instead of "Continue" on the title menu (and
the game asks to confirm deleting the old adventure): not scripted here.

Shots every `--every` frames (45) from frame 900 to `--to` (3000) go to
`<prefix>_<frame>.png` and the sheet to `<prefix>_sheet.png`
(default prefix target/ds/boot/watch). Work on a COPY of the ROM; nothing
captured may be committed.
"""
import os
import subprocess
import sys

from record import FPS, play, record, start


def main():
    args = sys.argv[1:]
    options = {'--to': '3000', '--every': '45', '--period': '40', '--video': None}
    for name in list(options):
        if name in args:
            at = args.index(name)
            options[name] = args[at + 1]
            del args[at:at + 2]
    if len(args) not in (1, 2):
        print(__doc__)
        return 2
    rom = args[0]
    prefix = args[1] if len(args) == 2 else 'target/ds/boot/watch'
    last, every, period = int(options['--to']), int(options['--every']), int(options['--period'])
    keys = f'mashA:{period}'
    if options['--video']:
        # From the controls screen on: the film and what follows.
        return record(rom, keys, options['--video'], 'both', 1000, last)
    os.makedirs(os.path.dirname(os.path.abspath(prefix)), exist_ok=True)
    emu = start(rom)
    steps = [(f, f + 2, 'A') for f in range(1, last + 1, period)]

    def on_frame(frame):
        if frame >= 900 and frame % every == 0:
            emu.screenshot().save(f'{prefix}_{frame}.png')

    play(emu, steps, last, on_frame)
    sheet = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'sheet.py')
    subprocess.run([sys.executable, sheet, f'{prefix}_sheet.png', prefix, '10'], check=True)
    print(f'{last / FPS:.0f}s of game from power on, A every {period} frames')
    return 0


if __name__ == '__main__':
    sys.exit(main())
