"""The paths the docs name are in the repository: every `folder/...` path
written in backticks in a tracked .md, or in a comment of a tracked .rs
(and the `film/recipes/...` a comment names, even bare), whose first folder is one of the repository's, must be a tracked file or
folder (from the repository's root, or from the file's own folder). The
user's own folders and what is built are not tracked by design: they are
let through (ALLOWED). Prints each stale path at its line; exit 1 if any.

    python tools/check_paths.py
"""
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Folders that are the user's or built, never tracked whole: a path under
# them is an example or the user's own.
ALLOWED = ("library/", "games/", "data/", "target/", "dist/", "saves/")

TICKS = re.compile(r"`([^`\n]+)`")
RECIPES = re.compile(r"film/recipes/[A-Za-z0-9_./-]*")
PATH = re.compile(r"[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]*)+")


def main():
    listed = subprocess.run(["git", "-C", ROOT, "ls-files", "-z"], capture_output=True, check=True).stdout.decode("utf-8")
    files = [name for name in listed.split("\0") if name]
    known = set(files)
    for name in files:
        parts = name.split("/")
        known.update("/".join(parts[:end]) for end in range(1, len(parts)))
    tops = {name.split("/")[0] for name in files if "/" in name}

    def exists(path, folder):
        path = path.rstrip("/")
        return any(
            candidate in known or candidate + ".rs" in known
            for candidate in (path, os.path.normpath(os.path.join(folder, path)).replace(os.sep, "/"))
        )

    stale = []
    for name in files:
        if name.endswith(".md"):
            spans = lambda text: ((line, span) for line, row in enumerate(text.split("\n"), 1) for span in TICKS.findall(row))
        elif name.endswith(".rs"):
            # (In a comment: the paths in backticks, and the recipes named bare.)
            spans = lambda text: (
                (line, span)
                for line, row in enumerate(text.split("\n"), 1)
                if "//" in row
                for span in TICKS.findall(row.split("//", 1)[1]) + RECIPES.findall(row.split("//", 1)[1])
            )
        else:
            continue
        with open(os.path.join(ROOT, name), encoding="utf-8", errors="replace") as file:
            text = file.read()
        folder = os.path.dirname(name)
        for line, span in spans(text):
            if any(mark in span for mark in "<>*{}…$") or "..." in span:
                continue
            for path in PATH.findall(span):
                path = re.sub(r":\d+(-\d+)?$", "", path.strip(".,;:"))
                if path.split("/")[0] not in tops or path.startswith(ALLOWED) or "//" in path:
                    continue
                if not exists(path, folder):
                    stale.append(f"{name}:{line}: {path}")
    stale = list(dict.fromkeys(stale))
    for row in stale:
        print(row)
    if stale:
        print(f"{len(stale)} paths named in the docs are not in the repository")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
