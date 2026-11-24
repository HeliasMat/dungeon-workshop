#!/bin/sh
# All of it checked, in one go: the rule of the crates, every test (those that
# need data/ too when it is there), clippy (not
# one warning), the format, the documentation, the build,
# the game by itself (the films are not played: see below). Goes on after a
# failure and says what failed; each step's output is in target/verify/.
#     sh tools/verify.sh
# One part only (a folder or a package: `decor`, `rules`, `scene_player`...): it
# and the parts that use it (`sh tools/parts.sh`), and nothing else; the
# game by itself only if it is among them.
#     sh tools/verify.sh <part>
cd "$(dirname "$0")/.." || exit 2
logs=target/verify
mkdir -p "$logs"
failed=""

step() {
    name=$1
    shift
    begun=$(date +%s)
    if "$@" >"$logs/$name.txt" 2>&1 </dev/null; then
        echo "ok     $name ($(($(date +%s) - begun)) s)"
    else
        echo "FAILED $name ($(($(date +%s) - begun)) s): $logs/$name.txt"
        grep -E '^(error|test .* FAILED|---- .* stdout)|panicked at|FAILED|^ +FAIL ' "$logs/$name.txt" | head -12 | sed 's/^/       /'
        failed="$failed $name"
    fi
}

# The tests: by cargo-nextest when it is installed (every crate's tests at once,
# in parallel; the doc's examples, which it does not run, by cargo test --doc
# after), else by cargo test. `tests <cargo args> [-- --ignored]`.
tests() {
    if cargo nextest --version >/dev/null 2>&1; then
        ignored=""
        case " $* " in *" --ignored "*) ignored="--run-ignored only" ;; esac
        # shellcheck disable=SC2046
        set -- $(printf '%s
' "$@" | grep -vx -e '--' -e '--ignored')
        # shellcheck disable=SC2086
        cargo nextest run --offline --no-tests=pass --hide-progress-bar $ignored "$@" || return 1
        [ -n "$ignored" ] || cargo test --offline --doc "$@"
    else
        cargo test --offline "$@"
    fi
}

# The places of the tests' games (tests/games/<game>/decors), built into data/extra
# where data/ is extracted: the tests read them as the game's places (without
# data/, the fixture lists them).
test_places() {
    [ -f data/rom/dungeons.bin ] || return 0
    for file in tests/games/*/decors/*.decor; do
        cargo run --offline -q -p decor -- build "$(basename "$file" .decor)" --from "$file" || return 1
    done
}

# (The tests that need the extracted data/ are marked
# `#[ignore = "needs the extracted data/ ..."]`, in these packages: with no
# data/ they are told as ignored; with it, the step data-tests runs them, and
# them alone, `-- --ignored`. The other ignored tests, those of rom_formats
# that read a ROM and scene_player's long check of every film, stay out.)
data_tested="assets decor film scenes preview game"
if [ -f data/rom/dungeons.bin ]; then
    echo "data/ is here: the tests that need it run too (data-tests)"
else
    echo "no data/: the tests that need it are only told as ignored"
fi

# (Formatted as cargo fmt does it: every crate.)
formatted="workshop diagnostics language engine film story dive scene_player preview game decor rom_formats extractor assets media shell spritecollab stage_view dungeon_lsp rules scenes"

if [ -n "$1" ]; then
    packages=$(sh tools/parts.sh "$1") || exit 2
    echo "checked: $packages"
    each=""
    shown=""
    for package in $packages; do
        each="$each -p $package"
        case " $formatted " in *" $package "*) shown="$shown -p $package" ;; esac
    done
    step rule sh tools/rule.sh
    # shellcheck disable=SC2086
    {
        step build cargo build --offline $each
        step test-places test_places
        step tests tests $each
        data_each=""
        for package in $packages; do
            case " $data_tested " in *" $package "*) data_each="$data_each -p $package" ;; esac
        done
        [ -n "$data_each" ] && [ -f data/rom/dungeons.bin ] && step data-tests tests $data_each -- --ignored
        step clippy cargo clippy --offline $each --all-targets -- -D warnings
        step doc env RUSTDOCFLAGS="-D warnings" cargo doc --offline --no-deps $each
        [ -n "$shown" ] && step format cargo fmt --check $shown
    }
else
    step rule sh tools/rule.sh
    step build cargo build --offline --workspace
    step test-places test_places
    step tests tests --workspace
    # shellcheck disable=SC2046
    [ -f data/rom/dungeons.bin ] && step data-tests tests $(for package in $data_tested; do printf -- '-p %s ' "$package"; done) -- --ignored
    step clippy cargo clippy --offline --workspace --all-targets -- -D warnings
    # (The documentation of the code: no link to nothing.)
    step doc env RUSTDOCFLAGS="-D warnings" cargo doc --offline --workspace --no-deps
    # shellcheck disable=SC2086
    step format cargo fmt --check $(for package in $formatted; do printf -- '-p %s ' "$package"; done)
fi
# (The films are no longer played here, as the user asked on 2026-11-09:
# `scene_player check <film>` checks one when it is worked on.)
if [ -n "$failed" ]; then
    echo "failed:$failed"
    exit 1
fi
echo "all pass"
