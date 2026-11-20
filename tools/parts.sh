#!/bin/sh
# The parts of the project (the crates), read from their Cargo.toml: each
# one's folder, its package, the parts it uses (its tests' too), the parts
# that use it, near or far.
#     sh tools/parts.sh           the table
#     sh tools/parts.sh <part>    the packages a change to that part can break:
#                                 it and all that use it (by folder or package)
cd "$(dirname "$0")/.." || exit 2
folders=$(sed -n 's/^members = \[\(.*\)\]/\1/p' Cargo.toml | tr -d '",')
tomls=$(for folder in $folders; do printf '%s/Cargo.toml ' "$folder"; done)
# (One awk for all of it: a program started is slow on Windows.)
# shellcheck disable=SC2086
awk -v asked="$1" '
    FNR == 1 { folder = FILENAME; sub(/\/Cargo\.toml$/, "", folder); order[++count] = folder }
    /^name = "/ && !(folder in package) { name = $0; gsub(/^name = "|"$/, "", name); package[folder] = name }
    /^[a-z_]+ = \{ path = "\.\.\// { used = $0; sub(/.*path = "\.\.\//, "", used); sub(/".*/, "", used); uses[folder] = uses[folder] " " used }
    # A part and all those that use it, near or far.
    function users(part,    found, grown, i, other, list, n, j) {
        found = " " part " "
        do {
            grown = 0
            for (i = 1; i <= count; i++) {
                other = order[i]
                if (index(found, " " other " ")) continue
                n = split(uses[other], list, " ")
                for (j = 1; j <= n; j++) if (index(found, " " list[j] " ")) { found = found other " "; grown = 1; break }
            }
        } while (grown)
        return found
    }
    END {
        if (asked != "") {
            for (i = 1; i <= count; i++) if (order[i] == asked || package[order[i]] == asked) part = order[i]
            if (part == "") { print "no part \"" asked "\"" > "/dev/stderr"; exit 2 }
            n = split(users(part), list, " ")
            line = ""
            for (j = 1; j <= n; j++) line = line (j > 1 ? " " : "") package[list[j]]
            print line
            exit 0
        }
        printf "%-12s %-13s %-52s %s\n", "folder", "package", "uses", "used by (near or far)"
        for (i = 1; i <= count; i++) {
            folder = order[i]
            others = users(folder); sub("^ " folder " ", "", others); sub(/ $/, "", others)
            used = uses[folder]; sub(/^ /, "", used)
            printf "%-12s %-13s %-52s %s\n", folder, package[folder], used, (others == "" ? "-" : others)
        }
    }
' $tomls
