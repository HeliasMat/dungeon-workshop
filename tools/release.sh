#!/bin/sh
# Ready-to-use programs, zipped: builds the release programs and makes
#     dist/dungeon-workshop-<version>-<target>.zip
# with the programs at its top, beside what they read where they are run
# (the "installed" layout: the same folders as the repository):
#   games/                where the games made go, a folder each (empty but its README)
#   library/              the shared library: pictures, sounds, music, tilesets (and the places,
#                         films and dungeons one makes)
#                         (the programs find their root by it: the folder that has library/films)
#   film/recipes/         the films' recipes, as files
#   README.md, LICENSE, CREDITS.md, docs/TUTORIAL.md, START_HERE.txt
# Never data/ (it is extracted from each one's own cartridge, beside the
# programs, on first launch), nor anything from the ROM. Only files git
# tracks go in: nothing of one's own work in progress.
#     sh tools/release.sh            the host's target
#     TARGET=x86_64-pc-windows-msvc sh tools/release.sh
#     CARGO_FLAGS=--offline sh tools/release.sh
cd "$(dirname "$0")/.." || exit 2
set -e

# (The one version of all the crates: [workspace.package] in Cargo.toml.)
version=$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
target=${TARGET:-$(rustc -vV | sed -n 's/^host: //p')}
programs="scene_player workshop dungeon_lsp extractor decor"
case "$target" in
*windows*) exe=.exe ;;
*) exe= ;;
esac
name="dungeon-workshop-$version-$target"
stage="target/release_stage/$name"
zip="dist/$name.zip"

echo "building $name"
packages=""
for program in $programs; do
    packages="$packages -p $program"
done
# shellcheck disable=SC2086 # (each word its own argument)
if [ -n "$TARGET" ]; then
    cargo build --release $CARGO_FLAGS --target "$target" $packages
    built="target/$target/release"
else
    cargo build --release $CARGO_FLAGS $packages
    built="target/release"
fi

rm -rf "$stage"
mkdir -p "$stage" dist
for program in $programs; do
    cp "$built/$program$exe" "$stage/"
done

# (What git tracks of those folders, as it is now.)
tracked() {
    if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        git -c core.quotepath=off ls-files -- "$@"
    else
        find "$@" -type f
    fi
}
tracked games library film/recipes README.md LICENSE CREDITS.md docs/TUTORIAL.md |
    while IFS= read -r file; do
        [ -f "$file" ] || continue
        mkdir -p "$stage/$(dirname "$file")"
        cp "$file" "$stage/$file"
    done
# (Nothing of a cartridge, ever.)
if find "$stage" -type f | grep -qE '\.(nds|bin|wan|sir0)$|/data/'; then
    echo "a file of a game's data was about to go in: stopped"
    exit 1
fi

cat >"$stage/START_HERE.txt" <<EOF
Dungeon Workshop $version ($target)
A workshop for making your own dungeon-crawling games.

1. Extract the resources from YOUR OWN copy of the game, once.
   Nothing of the game comes with this folder: the pictures, music and
   sounds are read from a cartridge you own and dumped yourself.
     - Open the Workshop (workshop$exe): click "Choose Your .nds File...",
       then "Extract the Resources". A few minutes, only once.
     - Or, without a window, from this folder:
         ./extractor$exe my_copy.nds
         ./scene_player$exe dungeons
         ./decor$exe inventory
         ./decor$exe all
   It all goes into data/, beside the programs. (Elsewhere: set the
   DUNGEON_DATA environment variable to that folder.)

2. Make your own: docs/TUTORIAL.md, "Your first game in 30 minutes",
   step by step. Or open the Workshop, the Game space, "+ New Game".
   Each game is a folder of its own in games/ (see games/README.md);
   copy the folder to share it, or: ./scene_player$exe pack-game <name>

Read README.md for everything else. library/ is the shared library
every game can use (pictures, sounds, music, tilesets, and the places and
films made in the Workshop). Play a game: ./scene_player$exe play <name>
(arrows: walk, Enter: talk, Shift: run, M: the menu).
EOF

rm -f "$zip"
here=$(pwd)
if command -v zip >/dev/null 2>&1; then
    (cd "target/release_stage" && zip -q -r -X "$here/$zip" "$name")
elif [ -x /c/Windows/System32/tar.exe ]; then
    # (Windows' own tar writes zips; Git's does not.)
    /c/Windows/System32/tar.exe -a -c -f "$zip" -C "target/release_stage" "$name"
elif tar --version 2>/dev/null | grep -q bsdtar; then
    tar -a -c -f "$zip" -C "target/release_stage" "$name"
else
    echo "no zip, nor a tar that writes zips: the folder is in $stage"
    exit 1
fi
echo "$zip"
