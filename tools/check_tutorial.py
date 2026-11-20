"""Checks every snippet of docs/TUTORIAL.md for real.

    python tools/check_tutorial.py            (from the repository's root, after
                                               cargo build --release -p scene_player)

Each fenced block of the tutorial is written where the tutorial says, under
target/tutorial_check/games/my_first_game/: the films (a block with `scene:`),
the place (a block with `decor:`, its [life] included),
the dungeon (`floors:`), the game (`chapter`). Then:
  - each film:  scene_player check --all <film>   0 problems, 0 remarks
  - the game:   scene_player check-game <game>    0 problems
  - the block of what `play --headless` tells:     the same lines, in order
Shell blocks are commands, not checked. Exits 1 if anything fails.
"""

import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TUTORIAL = os.path.join(ROOT, "docs", "TUTORIAL.md")
OUT = os.path.join(ROOT, "target", "tutorial_check")
GAME = "my_first_game"
FOLDER = os.path.join(OUT, "games", GAME)
EXE = ".exe" if os.name == "nt" else ""
PLAYER = os.path.join(ROOT, "target", "release", "scene_player" + EXE)


def blocks(text):
    """Each fenced block: (its line, its language, its text, the prose before it)."""
    found, lines, i = [], text.split("\n"), 0
    while i < len(lines):
        fence = re.match(r"^\s*```(\w*)\s*$", lines[i])
        if not fence:
            i += 1
            continue
        start, body = i, []
        i += 1
        while i < len(lines) and not re.match(r"^\s*```\s*$", lines[i]):
            body.append(lines[i])
            i += 1
        indent = min((len(l) - len(l.lstrip()) for l in body if l.strip()), default=0)
        body = [l[indent:] for l in body]
        found.append((start + 1, fence.group(1), "\n".join(body) + "\n", "\n".join(lines[max(0, start - 8):start])))
        i += 1
    return found


def named(before, ext):
    """The file the prose just before a block names (`games/my_first_game/films/x.film`)."""
    names = re.findall(r"`([\w/.]+\." + ext + r")`", before)
    return os.path.basename(names[-1]) if names else None


def run(*args):
    done = subprocess.run([PLAYER, *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    return done.returncode, (done.stdout + done.stderr).strip()


def main():
    if not os.path.exists(PLAYER):
        sys.exit(f"build it first: cargo build --release -p scene_player ({PLAYER} not found)")
    text = open(TUTORIAL, encoding="utf-8").read()
    shutil.rmtree(OUT, ignore_errors=True)
    for sub in ("films", "decors", "dungeons"):
        os.makedirs(os.path.join(FOLDER, sub))
    films, game, headless, failed, checked, skipped = [], None, None, 0, 0, 0
    for line, language, body, before in blocks(text):
        first = body.lstrip().split("\n", 1)[0]
        if language in ("sh", "bash"):
            skipped += 1
        elif re.search(r"^scene:", body, re.M):
            name = named(before, "film") or f"block_{line}.film"
            path = os.path.join(FOLDER, "films", name)
            films.append((line, path))
        elif re.search(r"^decor: \w+", body, re.M):
            decor = re.search(r"^decor: (\w+)", body, re.M).group(1)
            path = os.path.join(FOLDER, "decors", decor + ".decor")
        elif re.search(r"^floors:", body, re.M):
            path = os.path.join(FOLDER, "dungeons", named(before, "dungeon") or f"block_{line}.dungeon")
        elif re.search(r"^chapter \d", body, re.M) and re.search(r"^title:", body, re.M):
            path = game = os.path.join(FOLDER, GAME + ".game")
        elif first.startswith("chapter 1"):
            headless = (line, [l.strip() for l in body.split("\n") if l.strip()])
            continue
        else:
            print(f"TUTORIAL.md:{line}: a block of unknown kind, not checked")
            skipped += 1
            continue
        if language in ("sh", "bash"):
            continue
        with open(path, "w", encoding="utf-8", newline="\n") as file:
            file.write(body)
    for line, path in films:
        checked += 1
        code, said = run("check", "--all", path)
        counts = re.search(r"\b(\d+) problems?, (\d+) remarks?\b", said)
        clean = code == 0 and counts is not None and counts.groups() == ("0", "0")
        print(f"{'ok  ' if clean else 'FAIL'} TUTORIAL.md:{line}: {os.path.relpath(path, ROOT)}")
        if not clean:
            failed += 1
            print("     " + said.replace("\n", "\n     "))
    if game:
        checked += 1
        code, said = run("check-game", game)
        clean = code == 0 and re.search(r"\b0 problems\b", said)
        print(f"{'ok  ' if clean else 'FAIL'} check-game {os.path.relpath(game, ROOT)}")
        if not clean:
            failed += 1
            print("     " + said.replace("\n", "\n     "))
        if headless:
            checked += 1
            line, expected = headless
            code, said = run("play", game, "--headless", "--new")
            told = [l.strip() for l in said.split("\n") if l.strip()]
            same = code == 0 and told == expected
            print(f"{'ok  ' if same else 'FAIL'} TUTORIAL.md:{line}: play --headless tells what the tutorial says")
            if not same:
                failed += 1
                print("     expected: " + " / ".join(expected))
                print("     told:     " + " / ".join(told))
    print(f"{checked} checked, {skipped} commands not checked, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
