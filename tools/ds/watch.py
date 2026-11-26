"""A film on DS in one command: exported, put in the cached base copy of
your ROM (`ds_cache.py`), played from the ready savestate, a contact sheet
written.

    python tools/ds/watch.py <film> [--scene n] [--headless] [--frames N]
        [--every N] [--period N] [--out <folder>] [--rom <rom.nds>]
        [--core <libretro core> | --desmume]
    python tools/ds/watch.py <film> --prepare [--scene n] [--core <core>]

- `<film>` an `.exps` (a script written by hand, as `export-ds` writes
  them): played as it is, not exported.
- `--scene n`: from the film's n-th scene on (`ds_cache.py --scene`).
- `--headless`: no window: `--frames` frames (1200, 20 s of game) played
  from the ready state, A every `--period` (40), a shot every `--every`
  (30); the shots and `sheet.png` go to `--out` (`target/ds/watch/`).
  Played by the Workshop's libretro core (its settings' `libretro_core`, or
  `--core`) through the `emulator` program, about 380 frames a second
  with melonDS; else (`--desmume`, or no core) by py-desmume, about 140.
- Neither: the Workshop opens on its Emulator space and plays the film
  there (Play This Film on DS, from the same cache).
- `--prepare`: the export, the copy and the core's ready state, then a line
  `ds-play <copy> <ready state>` (the copy's `.pokes` beside it): what the
  Workshop's Play This Film on DS runs.
- `--rom`: your Explorers of Sky; by default the Workshop's (its settings'
  `ds_rom`), as is armips (`armips`) unless `SKYTEMPLE_ARMIPS_EXEC` is set.

The first run makes the base (4 s, and about 8 s a Pokémon of
SpriteCollab's the first time one is cast) and the core's ready state (3 s
with melonDS, 7 with py-desmume); then a run is the export (about 1 s), the
copy (about 1.5 s) and the film from its first frame. The copy is
`target/ds/watch/watch.nds` (`workshop_play.nds` for the Workshop), written
over each time: never commit it.

A ready state is a core's own: melonDS's is taken at frame 1200 with A every
90 frames until 1050 (`MELON_PRESS`), py-desmume's at 1060
(`ds_cache.BOOT_PRESSES`); both on the New Game menu.
"""
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import ds_cache  # noqa: E402

ROOT = ds_cache.ROOT
EXE = '.exe' if os.name == 'nt' else ''
# The DS's main memory.
MAIN_RAM, MAIN_RAM_SIZE = 0x2000000, 0x400000


def settings():
    """The Workshop's settings (`key = value`), as `workshop_base::settings` keeps them."""
    if os.name == 'nt':
        folder = os.path.join(os.environ.get('APPDATA', ''), 'dungeon_workshop')
    else:
        folder = os.path.join(os.environ.get('XDG_CONFIG_HOME') or os.path.expanduser('~/.config'), 'dungeon_workshop')
    found = {}
    try:
        with open(os.path.join(folder, 'settings.txt'), encoding='utf-8') as file:
            for line in file:
                if '=' in line and not line.lstrip().startswith('#'):
                    key, value = line.split('=', 1)
                    found[key.strip()] = value.strip()
    except OSError:
        pass
    return found


def program(name):
    """A program of the workspace's, the newest build of it."""
    built = [os.path.join(ROOT, 'target', kind, name + EXE) for kind in ('release', 'debug')]
    built = [path for path in built if os.path.isfile(path)]
    if not built:
        raise SystemExit(f'{name}: not built (cargo build -p {name})')
    return max(built, key=os.path.getmtime)


def export(film, exps):
    command = [program('scene_player'), 'export-ds', film, '--out', exps, '--partial']
    done = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding='utf-8', errors='replace')
    for line in (done.stdout + done.stderr).splitlines():
        if line.strip():
            print(f'  {line}')
    if done.returncode != 0:
        raise SystemExit('the export failed')


def ready_state(paths, tell=print):
    """The ready savestate of the base (py-desmume's): made once, from
    power on with no save file, on the New Game menu (the controls on the
    top screen), before the game reads the new game's event."""
    if os.path.isfile(paths.ready):
        return paths.ready
    started = time.time()
    # (In a process of its own: py-desmume runs one emulator a process.)
    subprocess.run([sys.executable, __file__, '--ready', paths.base, paths.ready + '.tmp'], check=True, capture_output=True)
    os.replace(paths.ready + '.tmp', paths.ready)
    tell(f'  ready state: {paths.ready} ({time.time() - started:.1f} s)')
    return paths.ready


def make_ready(base, out):
    from record import play, start
    emu = start(base)
    steps = [(frame, frame + 2, 'A') for frame in ds_cache.BOOT_PRESSES]
    play(emu, steps, ds_cache.READY_FRAME, lambda frame: None)
    emu.savestate.save_file(out)


def pokes_of(path):
    """A `.pokes` file of ds_cache.py: (the mark, [(spare actor, species)])."""
    mark, actors = None, []
    with open(path, encoding='utf-8') as file:
        for line in file:
            words = line.split()
            if words[:1] == ['mark']:
                mark = int(words[1])
            elif words[:1] == ['actor']:
                actors.append((int(words[1]), int(words[2])))
    return mark, actors


def actor_table(memory, mark):
    """Where the actor list is in the DS's main memory (from 0x2000000):
    the first spare entry, found by its species (mark, mark + 1, mark + 2,
    an entry of 12 bytes apart). None if not there."""
    pattern = b''.join(re.escape((mark + n).to_bytes(2, 'little')) + (b'.{10}' if n < 2 else b'') for n in range(3))
    found = re.compile(b'(?s)..' + pattern).search(memory)
    return None if found is None else found.start()


def poke(emu, path):
    """The film's spare actors' species written in the ready state's
    memory (the game read the base's list at power on)."""
    if not os.path.isfile(path):
        return
    mark, actors = pokes_of(path)
    memory = bytes(emu.memory.unsigned[MAIN_RAM:MAIN_RAM + MAIN_RAM_SIZE])
    at = actor_table(memory, mark)
    if at is None:
        raise SystemExit('the ready state holds no actor list of the base: remove it to make it again')
    for spare, species in actors:
        address = MAIN_RAM + at + spare * 12 + 2
        for offset, value in enumerate(species.to_bytes(2, 'little')):
            emu.memory.unsigned[address + offset] = value


def watch(rom, state, out, frames, every, period):
    """The film played from the ready state (A every `period` frames: the
    New Game menu, then each line), a shot every `every`; the sheet."""
    from record import play, start
    os.makedirs(out, exist_ok=True)
    for old in os.listdir(out):
        if old.startswith('shot_') and old.endswith('.png'):
            os.remove(os.path.join(out, old))
    emu = start(rom, state)
    poke(emu, rom + '.pokes')
    steps = [(frame, frame + 2, 'A') for frame in range(1, frames + 1, period)]
    prefix = os.path.join(out, 'shot')

    def on_frame(frame):
        if frame % every == 0:
            emu.screenshot().save(f'{prefix}_{frame}.png')

    play(emu, steps, frames, on_frame)
    sheet = os.path.join(out, 'sheet.png')
    subprocess.run([sys.executable, os.path.join(HERE, 'sheet.py'), sheet, prefix, '10'], check=True)
    return sheet


# melonDS's boot, by the `emulator` program: A every 90 frames until 1050
# (the logos, the title), the New Game menu waiting from about 1125.
MELON_PRESS = 'A:90:1050'
MELON_READY = 1200


def core_ready(paths, core, tell=print):
    """A libretro core's ready state of the base (its own savestate), made
    once by the `emulator` program."""
    stem = os.path.splitext(os.path.basename(core))[0]
    state = os.path.join(paths.base_dir, f'ready.{stem}.state')
    if os.path.isfile(state):
        return state
    started = time.time()
    shot = os.path.join(paths.base_dir, 'ready.png')
    command = [program('emulator'), core, paths.base, str(MELON_READY), shot, '--press', MELON_PRESS, '--state-out', state + '.tmp']
    subprocess.run(command, cwd=ROOT, check=True, capture_output=True)
    os.replace(state + '.tmp', state)
    os.remove(shot)
    tell(f'  ready state: {state} ({time.time() - started:.1f} s)')
    return state


def core_watch(core, rom, state, out, frames, every, period):
    """As `watch`, played by a libretro core through the `emulator` program."""
    os.makedirs(out, exist_ok=True)
    for old in os.listdir(out):
        if old.startswith('shot') and old.endswith('.png'):
            os.remove(os.path.join(out, old))
    forget_saves(rom)
    prefix = os.path.join(out, 'shot')
    command = [program('emulator'), core, rom, str(frames), prefix + '.png', '--press', f'A:{period}', '--every', str(every),
               '--state-in', state, '--pokes', rom + '.pokes']
    done = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding='utf-8', errors='replace')
    if done.returncode != 0:
        raise SystemExit(f'the emulator failed: {done.stderr.strip()[-400:]}')
    os.remove(prefix + '.png')
    sheet = os.path.join(out, 'sheet.png')
    subprocess.run([sys.executable, os.path.join(HERE, 'sheet.py'), sheet, prefix, '10'], check=True)
    return sheet


def forget_saves(rom):
    """The emulator program's save files of that copy: a copy starts a new game."""
    saves = os.path.join(ROOT, 'target', 'emulator', 'saves')
    stem = os.path.splitext(os.path.basename(rom))[0]
    for name in os.listdir(saves) if os.path.isdir(saves) else []:
        if os.path.splitext(name)[0] == stem:
            os.remove(os.path.join(saves, name))


def main():
    args = sys.argv[1:]
    if args[:1] == ['--ready'] and len(args) == 3:
        make_ready(args[1], args[2])
        return 0
    options = {'--scene': None, '--frames': '1200', '--every': '30', '--period': '40', '--out': None, '--rom': None, '--core': None, '--copy': None}
    for name in list(options):
        if name in args:
            at = args.index(name)
            options[name] = args[at + 1]
            del args[at:at + 2]
    flags = {flag: flag in args for flag in ('--headless', '--prepare', '--desmume')}
    args = [arg for arg in args if arg not in flags]
    if len(args) != 1:
        print(__doc__)
        return 2
    film = os.path.abspath(args[0])
    known = settings()
    rom = options['--rom'] or known.get('ds_rom')
    if not rom or not os.path.isfile(rom):
        raise SystemExit('no ROM: give --rom <your Explorers of Sky .nds> (or choose it once in the Workshop)')
    if not os.environ.get('SKYTEMPLE_ARMIPS_EXEC') and known.get('armips'):
        os.environ['SKYTEMPLE_ARMIPS_EXEC'] = known['armips']
    scene = int(options['--scene']) if options['--scene'] else None
    core = None if flags['--desmume'] else (options['--core'] or known.get('libretro_core'))
    if core and not os.path.isfile(core):
        core = None
    if not flags['--headless'] and not flags['--prepare']:
        # The Workshop does the same from the same cache, and plays it in its window.
        environment = dict(os.environ, WORKSHOP_EMULATOR_FILM=film)
        if scene:
            environment['WORKSHOP_EMULATOR_SCENE'] = str(scene)
        subprocess.Popen([program('workshop')], cwd=ROOT, env=environment)
        print(f'the Workshop opens on its Emulator space with {os.path.basename(film)}')
        return 0
    folder = os.path.join(ROOT, 'target', 'ds', 'watch')
    os.makedirs(folder, exist_ok=True)
    stem = os.path.splitext(os.path.basename(film))[0]
    copy = options['--copy'] or os.path.join(folder, 'watch.nds')
    exps = os.path.join(os.path.dirname(copy), f'{stem}.exps')
    times = []
    started = time.time()
    if film.endswith('.exps'):
        # (A script of one's own, as patch_rom.py reads it: played as it is.)
        exps = film
    else:
        export(film, exps)
    times.append(('export', time.time() - started))
    lap = time.time()
    paths = ds_cache.film_rom(rom, exps, copy, scene, tell=lambda line: print(f'  {line}'))
    times.append(('copy', time.time() - lap))
    lap = time.time()
    state = core_ready(paths, core) if core else ready_state(paths)
    times.append(('ready state', time.time() - lap))
    if flags['--prepare']:
        print(f'ds-play {copy} {state}')
        return 0
    lap = time.time()
    shots = options['--out'] or folder
    frames, every, period = int(options['--frames']), int(options['--every']), int(options['--period'])
    if core:
        sheet = core_watch(core, copy, state, shots, frames, every, period)
    else:
        sheet = watch(copy, state, shots, frames, every, period)
    times.append(('played', time.time() - lap))
    print(' · '.join(f'{what} {seconds:.1f} s' for what, seconds in times) + f' · total {time.time() - started:.1f} s')
    print(sheet)
    return 0


if __name__ == '__main__':
    sys.exit(main())
