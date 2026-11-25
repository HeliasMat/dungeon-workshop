#!/bin/sh
# The rule of the crates: each may use those named here, no other.
cd "$(dirname "$0")/.."
wrong=0
ruled=""
check() {
    ruled="$ruled
$1: $2"
    used=$(sed -n '/^\[dependencies\]/,/^\[/p' "$1/Cargo.toml" | grep -oE '^[a-z_]+ = \{ path' | cut -d' ' -f1 | sort | tr '\n' ' ')
    [ "$used" = "$2" ] || { echo "$1 uses: $used; the rule: $2"; wrong=1; }
}
check diagnostics ""
check engine "diagnostics "
check language "diagnostics "
check assets ""
check media "assets "
check shell "assets media "
check spritecollab "assets diagnostics "
check rules "diagnostics "
check scenes "assets engine rules "
check story "engine "
check rom_formats "media rules "
check stage_view "assets engine media rules scenes shell "
check film_core "assets engine language rules scenes story "
check film_view "engine film_core scenes "
check film_speech "engine film_core scenes "
check film_fishing "engine film_core rules scenes "
check film_fights "assets engine film_core film_dungeon film_speech film_view rules scenes "
check film_dungeon "engine film_core film_speech film_view scenes "
check film_moving "engine film_acting film_core film_speech "
check film_acting "engine film_core film_speech scenes "
check film "assets engine film_acting film_core film_dungeon film_fights film_fishing film_moving film_speech film_view language scenes story "
check preview "assets engine film media rules scenes shell stage_view "
check game "assets decor dive engine film language preview rules scenes shell stage_view "
check ds_export "engine film game preview rules scenes "
check scene_player "ds_export engine film game language preview rules scenes shell stage_view "
check decor "assets diagnostics media "
check emulator ""
check dungeon_lsp "assets decor film game language "
check extractor "engine rom_formats rules shell stage_view "
check dive "rules scenes shell stage_view "
check workshop_base "assets decor film preview shell spritecollab stage_view "
check workshop_decor "assets decor film scenes shell workshop_base "
check workshop_dungeon "assets decor dive film game media rules shell workshop_base "
check workshop_film "engine film preview shell stage_view workshop_base workshop_decor workshop_script "
check workshop_music "film rules shell workshop_base workshop_sound "
check workshop_picture "film workshop_base "
check workshop_resources "assets film game rules scenes shell spritecollab workshop_base workshop_decor workshop_dungeon workshop_music workshop_picture workshop_script workshop_sound "
check workshop_script "assets decor film rules scenes shell workshop_base workshop_wiki "
check workshop_sound "film rules shell workshop_base "
check workshop_wiki "engine film preview workshop_base "
check workshop_game "assets decor engine game workshop_base workshop_decor "
check workshop_first "assets decor game workshop_base "
check workshop_emulator "emulator shell workshop_base workshop_first "
check workshop_home "decor film workshop_base workshop_decor "
check workshop "assets decor engine film game media shell spritecollab workshop_base workshop_decor workshop_dungeon workshop_emulator workshop_film workshop_first workshop_game workshop_home workshop_resources workshop_script "
# (The table of ARCHITECTURE.md, between <!-- rule --> and its end, is this
# list, block for block.)
told=$(sed -n '/^<!-- rule -->/,/^<!-- end of rule -->/p' ARCHITECTURE.md | grep '^| `' | tr -d '`,' | sed 's/^| \([a-z_]*\) | \(.*\) |$/\1: \2 /; s/: nothing $/: /' | sort)
listed=$(printf '%s\n' "$ruled" | grep . | sort)
if [ "$told" != "$listed" ]; then
    echo "ARCHITECTURE.md's table of the rule (<!-- rule -->) is not this list:"
    printf '%s\n' "$listed"
    wrong=1
fi
# (No data of a game in the repository: no copy of a cartridge, nothing extracted from one.)
if git ls-files | grep -qE '\.(nds|bin|wan|sir0)$|^data/[^D]|^rom/'; then
    echo "the repository holds data of a game: it is read from the user's own copy, never committed"
    wrong=1
fi
exit $wrong
