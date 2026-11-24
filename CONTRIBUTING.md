# Contributing

## Building

The toolchain is pinned in `rust-toolchain.toml` (`rustup` picks it up by itself).

```bash
cargo build --workspace
cargo run -p workshop             # the editor
cargo run -p scene_player -- help # the player and its commands
```

The game's data is not in the repository: it is extracted from your own copy of the game (`cargo run -p extractor -- <rom>`, or the Workshop's first launch) into `data/`. Every test passes without it: where `data/` holds no extraction, the tests read a small made-up fixture instead (`tests/fixture_data/`: a few species, moves, items, places' sizes and grounds, written by hand); those that need the real data (every species, the story, the pictures and sounds) are `#[ignore]`d and run with `-- --ignored`.

To run the tests as CI does, without the data even where you have it, point them at the fixture:

```bash
DUNGEON_DATA=tests/fixture_data cargo test --workspace
```

A test that fails only there needs a line added to the fixture (in the format of `data/DATA.md`, numbers and sizes of our own, never a picture, a sound or text of the game), or, if it truly needs the game's files, the `#[ignore = "needs the extracted data/ (run with --ignored)"]` the others have.

## Checking

```bash
sh tools/verify.sh          # everything: the crate rule, build, tests, clippy, docs, format
sh tools/verify.sh <part>   # one crate and those that use it
sh tools/rule.sh            # the crate rule alone
python tools/check_paths.py # the paths the docs name are in the repository
python tools/check_tutorial.py  # every snippet of docs/TUTORIAL.md checked for real (after a release build of scene_player)
```

CI runs the same steps (`.github/workflows/ci.yml`). Clippy must give no warning, and every crate is formatted by `cargo fmt` (`rustfmt.toml`). The tests run under cargo-nextest when it is installed (`.config/nextest.toml`), else under `cargo test`.

## How-to guides

- [Add a film word](docs/ADD_A_FILM_WORD.md): its family and crate, the `Word`, the plan, the DS status, the Wiki page, FILMS.md, `scene_player try`.
- [Add a Workshop panel or part](docs/ADD_A_WORKSHOP_PANEL.md): its crate, the session, asking the window (`workshop_base/src/ask.rs`), registering it in `workshop/src/app.rs`.

## The crate rule

Each crate only uses crates of the layers below it, as the table in `ARCHITECTURE.md` lists them; `sh tools/rule.sh` fails when a `Cargo.toml` and that table differ. Adding a dependency between crates means changing both. Read `ARCHITECTURE.md` first: who decides what, where each rule lives.

## Where things go

- `library/`: the shared library every game sees (films, decors, dungeons, fights, images, sounds, music, tilesets, `tables/` the pieces and names places and films are made of).
- `games/<name>/`: a game, its `.game` file and its own films, decors, dungeons; a file is looked for there first, then in `library/`.
- `tests/`: only what tests alone use.
- `data/`: extracted from the user's cartridge and made from it. **Never committed** (only `data/DATA.md`, which says what each file is and who writes it).

## Commits

One commit for one finished change, its message in English: what changed and why, in a sentence or two, the first line standing on its own. Keep the docs (`ARCHITECTURE.md`, `film/FILMS.md`, `engine/docs/COMMANDS.md`, `data/DATA.md`) in step with the code in the same commit; the tables they hold are checked by tests (`GOLDEN=write cargo test -p film docs` writes the words' table anew).

## The quality gate of a word

The film language is its vocabulary (`film_core/src/vocabulary.rs`: the families; each word declared once, on its `Word` or in `OTHERS`). A new word joins its family only once it passes the gate.

The part the tests check (`every_word_passes_the_gate` for the catalogue, then one `gate_<family>` per family for its pages, run side by side; `workshop_wiki/src/tests.rs`): every word of the catalogue is a word of the vocabulary or an advanced one (an engine's command), none of the old vocabulary (`REPLACED`) nor of the library's own (`_bobber`); each has its page in the Wiki (`workshop_wiki/wiki/*.txt`, written by hand for the core's and the optional families' words), with an example that reads with no problem and no remark, and that plays with nothing to say where the game's files are there (as `scene_player check` tells it; with the fixture, `DUNGEON_DATA=tests/fixture_data`, only the reading). The recipes and commands of `OTHERS` not built yet are listed by the test (`cargo test -p workshop_wiki gate -- --nocapture`), not asked.

The part a person checks, for each word before it joins its family:

- **Look at it in a picture**: `scene_player try <film> [moments]` on its Wiki example (copied into a film of `library/films/`), the moments where the word plays: it shows what the word says, where it says it, and nothing else moves by itself.
- **The game's assets first**: the sprites' own animations, the emotes and their sounds, the moves' effects, the status icons, the weather's pictures. A drawing of one's own only where the game has nothing.
- **Pixel-crisp**: like the game, nothing soft: no blur, no smooth gradient, no glow; the pictures at whole pixels.
- **One order**: who acts first (`pikachu walk to piplup`), the word first when nobody acts (`music beach`); `off` only for a state that lasts.
