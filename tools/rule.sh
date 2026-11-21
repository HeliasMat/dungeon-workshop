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
check spritecollab "assets "
check rules "diagnostics "
check scenes "engine rules "
check story "engine "
check rom_formats "media rules "
check stage_view "assets engine media rules scenes shell "
check film "assets engine language rules scenes story "
check preview "assets engine film media rules scenes shell stage_view "
check game "assets decor dive engine film language preview rules scenes shell stage_view "
check scene_player "engine film game language preview rules scenes shell stage_view "
check decor "assets diagnostics media "
check emulator ""
check dungeon_lsp "assets decor film game language "
check extractor "engine rom_formats rules shell stage_view "
check dive "rules scenes shell stage_view "
check workshop "assets decor dive engine film game media preview rules scenes shell spritecollab stage_view "
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
