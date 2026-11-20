#!/bin/sh
# Puts together Dungeon Workshop, to give to someone without this source
# tree: the optimised programs (the workshop, and beside it what it calls:
# scene_player, extractor for the extraction, decor), the films, places,
# dungeons and fights of the project, our pictures and sounds. No data of a
# game: the workshop reads it out of the user's own copy the first time it
# opens. It goes to dist/Workshop/ (never committed).
#
#     sh tools/workshop.sh
set -e
cd "$(dirname "$0")/.."
# (Its own target folder, so that it does not wait on the debug builds.)
CARGO_TARGET_DIR=target/release-build cargo build --offline --release -p workshop -p scene_player -p extractor -p decor -j 2
out="dist/Workshop"
rm -rf "$out"
mkdir -p "$out/film"
for program in workshop scene_player extractor decor; do
    cp "target/release-build/release/$program.exe" "$out/" 2>/dev/null || cp "target/release-build/release/$program" "$out/"
done
# (What the programs read as they go, the root's layout (`assets::project`):
# the shared library (films, places, dungeons, fights, pictures, sounds,
# music, tilesets, the places' songs, `tables/`: the pieces places are made
# of, the grounds traced by hand, the names), the games, the films' recipes.)
cp -r library games "$out/"
cp -r film/recipes "$out/film/"
rm -f "$out"/library/films/*.txt
cp LICENSE CREDITS.md "$out/"
cat > "$out/README.txt" <<'EOF'
Dungeon Workshop: make your own dungeon-exploring games.

1. Launch workshop (workshop.exe on Windows).
2. The first time, it asks for your own copy of the Pokémon Mystery Dungeon:
   Explorers of Sky cartridge (a .nds file you made yourself): it takes the
   resources out of it into the data/ folder (a few minutes, only once). They
   come from your cartridge: do not share them.
3. Open a film, a decor, a dungeon, or create one.

The films are in library/films/, the places in library/decors/,
the dungeons and boss fights in library/dungeons/ and fights/: they
are text files, which the workshop edits and which can be read too.

A fan project, not affiliated with Nintendo, The Pokémon Company, Game Freak,
Creatures or Spike Chunsoft. Licence GPL-3.0: see LICENSE and CREDITS.md.
EOF
echo "$out: ready"
