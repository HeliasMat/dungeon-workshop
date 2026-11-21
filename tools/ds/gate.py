"""The DS quality gate: every word of the plan, as its Wiki page shows it,
exported, patched into a copy of the user's own ROM and played in the
real game, one contact sheet per family.

    python tools/ds/gate.py <rom> [family...] [--every N] [--no-play]

For each family of `film/src/plan.rs` (the core ones, the optional ones,
the advanced words), each word's example in the Wiki (`workshop/wiki/*.txt`,
under `== word`) is exported with `scene_player export-ds --partial`; the
examples that export become the scenes of one film (its name shown on black
before each, `message_Narration`), patched with `patch_rom.py --boot
--more-actors` into `target/ds/gate/<family>.nds`, played from power on
(A every 60 frames, as `boot_and_watch.py`) and shot every `--every` frames
(30). Written under `target/ds/gate/`:

- `<family>_sheet.png`: the shots, both screens, each under its frame (the
  word is said on black before its scene);
- `<family>.txt`: each word, whether it exported, and what the export
  left out or changed;
- `report.txt`: all families, in short.

`--no-play` stops after the export and the patch. The ROM is only
read; nothing written here may be committed (captures of the game, a
patched ROM). Needs a release build of scene_player
(`cargo build --release -p scene_player`), py-desmume, skytemple_files and,
for `--more-actors`, armips (`SKYTEMPLE_ARMIPS_EXEC`).
"""
import os
import re
import subprocess
import sys

from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..'))
OUT = os.path.join(ROOT, 'target', 'ds', 'gate')
PLAYER = os.path.join(ROOT, 'target', 'release', 'scene_player' + ('.exe' if os.name == 'nt' else ''))

# From power on: the title (A), the controls (A), then the film (frame ~1130).
FILM_STARTS = 1130
# Frames each scene is given beyond three quarters of its length in the engine
# (fades; A, pressed every second, closes lines sooner than the engine).
SCENE_MORE = 240


# The ways of writing with no page of their own in the Wiki (a line said,
# a choice's options, values in a text, the story's numbers): an example
# each, as the Wiki would write it.
SYNTAX = {
    '(line)': 'scene: beach pikachu piplup\npikachu: Hello there.\npiplup (happy): Hi!\n',
    'option': 'scene: beach pikachu piplup\npikachu: Swim or rest?\nchoice\n  option "Swim"\n    piplup hop\n  option "Rest"\n    piplup sleep\nend\n',
    '{if}': 'scene: beach pikachu piplup\nset apples 3\npikachu: {if apples > 2: So many apples! | Hardly any apples.}\n',
    '{value}': 'scene: beach pikachu piplup\nset apples 3\npikachu: I have {apples} apples.\n',
    'seen()': 'scene: beach pikachu piplup\nmoment: again\npikachu: Once more...\nif seen(again) < 2\n  go moment again\nend\npiplup: Done.\n',
    '#id': 'scene: beach pikachu piplup\npikachu: Hello there. #greeting\n',
    'keep': 'scene: beach pikachu piplup\nkeep met_piplup = 1\npikachu: Nice to meet you.\n',
}


def families():
    """The plan's families in its order, each with its words (`plan.rs`)."""
    text = open(os.path.join(ROOT, 'film', 'src', 'plan.rs'), encoding='utf-8').read()
    found = {}
    for family, word in re.findall(r'(?:word|recipe|syntax)\((ADVANCED|"[^"]+"), "([^"]+)"', text):
        family = 'Advanced' if family == 'ADVANCED' else family.strip('"')
        found.setdefault(family, []).append(word)
    return found


def examples():
    """Each word's example in the Wiki: its lines, by word."""
    pages = {}
    folder = os.path.join(ROOT, 'workshop', 'wiki')
    for name in sorted(os.listdir(folder)):
        if not name.endswith('.txt'):
            continue
        word, lines = None, []
        for line in open(os.path.join(folder, name), encoding='utf-8').read().splitlines() + ['']:
            if line.startswith('== '):
                word, lines = line[3:].strip(), []
                continue
            if word is None or line.startswith('>') or line.startswith('#'):
                continue
            if line.startswith('!'):
                # (`!retired`: an old word, no example.)
                word, lines = None, []
            elif line.strip():
                lines.append(line)
            elif lines:
                pages.setdefault(word, '\n'.join(lines) + '\n')
                word, lines = None, []
    for word, film in SYNTAX.items():
        pages.setdefault(word, film)
    return pages


def export(word, film, folder):
    """The word's example exported: (its scenes, its seconds, what it told, ok)."""
    stem = re.sub(r'[^a-z0-9_]', '_', word.lower()).strip('_') or 'word'
    path = os.path.join(folder, f'{stem}.film')
    with open(path, 'w', encoding='utf-8', newline='\n') as file:
        file.write(film)
    out = os.path.join(folder, f'{stem}.exps')
    done = subprocess.run([PLAYER, 'export-ds', path, '--out', out, '--partial'], capture_output=True, text=True, encoding='utf-8', cwd=ROOT)
    told = [line.strip() for line in (done.stderr + done.stdout).splitlines() if line.strip() and not line.startswith(out)]
    if done.returncode != 0 or not os.path.exists(out):
        return None, 0, told, False
    checked = subprocess.run([PLAYER, 'check', path], capture_output=True, text=True, encoding='utf-8', cwd=ROOT)
    seconds = re.search(r'([\d.]+) s,', checked.stdout + checked.stderr)
    return open(out, encoding='utf-8').read(), float(seconds.group(1)) if seconds else 10.0, told, True


def free_actor(at):
    """The exporter's name for the film's `at`th Pokémon with no actor of its own."""
    if at < 10:
        return f'NPC_TEST{at + 1:03d}'
    return f'FREE{at + 1}' if at < 45 else f'NPC_FREE{at:03d}'


def labelled(exps, word, species):
    """The export's scenes, the word said on black before each. Each
    example was exported alone, its free actors named from the first: they
    are named anew for the whole film, one for each species (`species`, the
    film's so far)."""
    said = word.replace('"', "'")
    names = {}
    for name, kind in re.findall(r'^// ds-export actor ACTOR_(\S+) species (\d+)', exps, re.M):
        if kind not in species:
            species.append(kind)
        names[name] = free_actor(species.index(kind))
    if names:
        exps = re.sub(r'\bACTOR_(\w+)', lambda found: 'ACTOR_' + names.get(found.group(1), found.group(1)), exps)
    lines = []
    for line in exps.splitlines():
        if line.startswith('// The film'):
            continue
        lines.append(line)
        if line.startswith('def 0 {'):
            lines.append(f'    message_Narration(30, "[CN]{said}");')
            lines.append('    message_Close();')
    return '\n'.join(lines) + '\n'


def play(rom, prefix, last, every):
    """The film played in a process of its own (py-desmume starts once a
    process): the shots it took, in frame order."""
    subprocess.run([sys.executable, os.path.abspath(__file__), '--shoot', rom, prefix, str(last), str(every)], capture_output=True)
    shots = []
    for name in os.listdir(os.path.dirname(prefix)):
        found = re.fullmatch(re.escape(os.path.basename(prefix)) + r'_(\d+)\.png', name)
        if found:
            shots.append((int(found.group(1)), os.path.join(os.path.dirname(prefix), name)))
    return sorted(shots)


def shoot(rom, prefix, last, every):
    """From power on, A every 60 frames; a shot every `every` frames once the film starts."""
    sys.path.insert(0, HERE)
    from record import play as run, start
    emu = start(rom)
    steps = [(f, f + 2, 'A') for f in range(1, last + 1, 60)]
    shots = []

    def on_frame(frame):
        if frame >= FILM_STARTS and frame % every == 0:
            path = f'{prefix}_{frame}.png'
            emu.screenshot().save(path)
            shots.append((frame, path))

    run(emu, steps, last, on_frame)
    return shots


def sheet(out, shots, columns=12, scale=0.5, page=96):
    """The shots side by side, both screens, each under its frame; past
    `page` shots, more sheets (`_2`, `_3`...), each readable whole."""
    if len(shots) > page:
        for at in range(0, len(shots), page):
            number = at // page + 1
            sheet(out if number == 1 else out.replace('.png', f'_{number}.png'), shots[at:at + page], columns, scale, page)
        return
    if not shots:
        return
    w, h = int(256 * scale), int(384 * scale)
    rows = (len(shots) + columns - 1) // columns
    image = Image.new('RGB', (w * columns, (h + 12) * rows), (24, 24, 24))
    draw = ImageDraw.Draw(image)
    for at, (frame, path) in enumerate(shots):
        x, y = (at % columns) * w, (at // columns) * (h + 12)
        image.paste(Image.open(path).resize((w, h), Image.BILINEAR), (x, y + 12))
        draw.text((x + 2, y), str(frame), fill=(255, 255, 0))
    image.save(out)


def main():
    args = sys.argv[1:]
    if args[:1] == ['--shoot']:
        shoot(args[1], args[2], int(args[3]), int(args[4]))
        return 0
    every = 30
    if '--every' in args:
        at = args.index('--every')
        every = int(args[at + 1])
        del args[at:at + 2]
    playing = '--no-play' not in args
    args = [arg for arg in args if arg != '--no-play']
    if not args:
        print(__doc__)
        return 2
    rom, asked = args[0], [arg.lower() for arg in args[1:]]
    if not os.path.exists(PLAYER):
        print(f'{PLAYER}: build it first (cargo build --release -p scene_player)')
        return 2
    os.makedirs(OUT, exist_ok=True)
    pages = examples()
    report = []
    for family, words in families().items():
        if asked and family.lower() not in asked:
            continue
        folder = os.path.join(OUT, family.lower())
        os.makedirs(folder, exist_ok=True)
        lines, scenes, species = [f'# {family}'], [], []
        frame = FILM_STARTS
        for word in words:
            film = pages.get(word)
            if film is None:
                lines.append(f'{word}: no example in the Wiki')
                continue
            exps, seconds, told, ok = export(word, film, folder)
            lines.append(f'{word}: {"exported" if ok else "NOT EXPORTED"}')
            lines.extend(f'    {line}' for line in told)
            if ok:
                scenes.append(labelled(exps, word, species))
                count = exps.count('// ds-export level')
                length = int(seconds * 45) + SCENE_MORE * max(count, 1) + 60
                frame += length
        exported = len(scenes)
        summary = f'{family}: {exported} of {len(words)} words exported'
        if scenes and playing:
            film = os.path.join(OUT, f'{family.lower()}.exps')
            with open(film, 'w', encoding='utf-8', newline='\n') as file:
                file.write(f'// The DS gate: the family {family}, one scene a word.\n' + ''.join(scenes))
            nds = os.path.join(OUT, f'{family.lower()}.nds')
            patched = subprocess.run([sys.executable, os.path.join(HERE, 'patch_rom.py'), rom, film, '--boot', '--more-actors', nds],
                                     capture_output=True, text=True, encoding='utf-8')
            if patched.returncode != 0:
                lines.append('patch failed:')
                lines.extend(f'    {line}' for line in (patched.stdout + patched.stderr).splitlines()[-12:])
                summary += ', patch FAILED'
            else:
                shots_folder = os.path.join(folder, 'shots')
                os.makedirs(shots_folder, exist_ok=True)
                for old in os.listdir(shots_folder):
                    os.remove(os.path.join(shots_folder, old))
                shots = play(nds, os.path.join(shots_folder, 's'), frame + 600, every)
                sheet(os.path.join(OUT, f"{family.lower()}_sheet.png"), shots)
                summary += f', played to frame {frame + 600}'
        with open(os.path.join(OUT, f'{family.lower()}.txt'), 'w', encoding='utf-8', newline='\n') as file:
            file.write('\n'.join(lines) + '\n')
        report.append(summary)
        print(summary, flush=True)
    with open(os.path.join(OUT, 'report.txt'), 'w', encoding='utf-8', newline='\n') as file:
        file.write('\n'.join(report) + '\n')
    return 0


if __name__ == '__main__':
    sys.exit(main())
