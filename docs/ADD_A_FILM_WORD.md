# Add a film word

A checklist, in the order you will do it. `hop` is the example throughout: its declaration and its page are each one place you can grep.

A word is declared once, on its `Word`: its forms, what it writes, how it plays, its family and its DS status. Everything else is derived from it: the vocabulary (`film_core/src/vocabulary.rs`, `Lexicon::vocabulary`), the DS table (`scene_player export-ds --table`), the catalogue, the Wiki's gate and the "All the words" table of `film/FILMS.md`.

Two words to keep apart:

- a **family** is what an author sees: `Movement`, `Speech`, `Fishing`... (`film_core::vocabulary::FAMILIES`);
- a **group** is a list of words in the code (`WORDS` and its `GROUP`, `film_core::verbs::Group`), the lists a line is offered to, the narrower first.

## 1. Pick its group and its crate

- **The core leans on it** (things, sounds, the view, the sky, effects, items, pictures, crowds, lines said, timing): a file of `film_core/src/verbs/`, for example `film_core/src/verbs/sky.rs`.
- **Everything else** goes in a group's crate, which uses `film_core` and nothing above it:
  - `film_acting/src/lib.rs`: what someone says and does where they stand (`hop`, `think`, `narrate`...).
  - `film_moving/src/`: moving, swimming, flying, riding, contact, hiding, body.
  - `film_fishing/src/`: fishing.
  - `film_dungeon/src/`: the dungeon and the guild.
  - `film_fights/src/`: fights.
- **A new group**: put its `GROUP` (`Group::new("name", WORDS)`) in `GROUPS` in `film/src/lib.rs`, and change the array's length. The order matters: the narrower groups come first, and the first form that fits wins. If the group keeps state from one line to the next, give it a type of its own, kept by the film (`Film::kept`, `film_core::film::Kept`; `film_fishing/src/state.rs` is an example).
- **A new group crate**: add it to `members` in `Cargo.toml` and as a dependency of `film/Cargo.toml`, then add its rows to the crate tables of `ARCHITECTURE.md`. `sh tools/rule.sh` fails until the table and the manifests agree.

## 2. Write the `Word`

In the group's `WORDS` (see `film_core/src/verbs/mod.rs` for the builder and the form notation):

```rust
Word::new("hop", &["who:pokemon [happy|surprised|angry] [times n:number]"], "what it writes, for the catalogue")
    .family("Movement")
    .ds_direct("ExecuteCommon<actor>(CORO_JUMP_HAPPY|SURPRISE|ANGRY_FUNC_SERIES, 0), WaitExecuteLives")
    .maybe(hopped),
```

- **Forms**: one string per way of writing the line, the narrower first, in the notation of `film_core/src/verbs/mod.rs`: `who:pokemon`, `*who`, `where:place`, `d:duration`, `n:number`, `a|b`, `[...]`, `text...`, `rest...`. A single reader, `film_core/src/grammar.rs`, checks every line against them. A form that starts with who acts is written who first (`pikachu hop`); add `.word_first()` if the word comes first all the same (`fight a vs b`, `wait pikachu`).
- **Its family**: `.family("Movement")`, one of `FAMILIES` in `film_core/src/vocabulary.rs` (`"Advanced"` for a word for those who want to control everything). The catalogue, the Wiki and the completion show the word there. A word of an optional family (`Dungeon`, `Guild`, `Fishing`) asks its film for `uses dungeon`, `uses guild` or `uses fishing`.
- **Its DS status**, always with the family:
  - `.ds_direct(how)`: written as the game's own operations;
  - `.ds_close(how, lost)`: written with less, and what is lost (the remark `R-DS-LOSS` of a film `for ds`);
  - `.ds_work(how)`: the game could, the export does not write it yet;
  - `.ds_none(how)`: the game has nothing for it; add `.ds_fix("say it in a line")` to tell how to write it another way.
- **How it plays**:
  - `.does(f)`: `fn(&mut Film, &Line)`, the word always plays its line.
  - `.maybe(f)`: `fn(&mut Film, &Line) -> bool`. Returning `false` hands the line to the next group, or to the engine's command of that name.
  - In either one, build typed beats with the constructors of `film_core/src/emit/` (`emit::jump`, `emit::music`...). Never write text for the engine to read again.
- **Who it brings on stage**, if its line puts someone there mid-scene: `.brings(&["who"])`, `.puts(..)`, `.arrives(..)` or `.brings_with(f)`, plus `.when("word")` if only one form brings someone.
- **How long its line lasts**: `.lasts()`, `.until()` or `.instant()` (`film_core/src/time/`).
- **A word several groups read** (`jump`: acting's, fishing's `jump a water`, body's): its family and DS status go on one declaration only; the others are plain `Word::new(...)` that add their forms, in the narrower groups.
- **A word made of other words**: write it as a recipe in `film/recipes/` instead, and give it its line in `OTHERS` (`film_core/src/vocabulary.rs`: `recipe("Body", "hug", "who hug other").ds_close(..)`). `OTHERS` also holds the ways of writing of the language itself (`syntax(...)`) and the engine's commands shown in a family (`command(...)`): what has no `Word` of its own.
- **Advanced words** have the engine's own form (`film_core/src/verbs/engine.rs`). Only write one when the word means more than the command does; an engine's form never carries a family.

## 3. Its Wiki page

In the family's file of `workshop_wiki/wiki/` (`movement.txt` for `hop`; the format is described at the top of `workshop_wiki/src/written.rs`):

```text
== hop
> One paragraph, for an author: what the word does.
scene: beach pikachu piplup
pikachu hop happy times 2
```

The example is a small film. It must read with no problem and no remark, and play with nothing to say. That is the gate (`workshop_wiki/src/tests.rs`, one `gate_<family>` test per family).

## 4. FILMS.md

- **The family table** near the top of `film/FILMS.md` ("Family | Words") is written by hand: add the word's form there.
- **"All the words"**, between its `words` marks, is generated from the code. Rewrite it with:

```bash
GOLDEN=write cargo test -p film docs
```

## 5. Look at it

Copy the Wiki example into a film of your own in `library/films/` (that folder is yours and is never committed), then:

```bash
cargo run -p scene_player -- check my_trial
cargo run -p scene_player -- try my_trial 2s line 4    # writes target/films/my_trial/try.png
```

Look at the sheet against the person's half of the gate (CONTRIBUTING.md, "The quality gate of a word"): the game's own assets, pixel-crisp, nothing moving by itself.

## 6. Run

```bash
cargo check -p film_acting                    # or the group's crate
cargo nextest run -p film -p film_core -p workshop_wiki
sh tools/verify.sh                            # once, at the end
```

`film/src/tests/vocabulary.rs` tells a word declared twice, a family that does not exist, or a family without its DS status. To list the recipes and commands of `OTHERS` still to build: `cargo test -p workshop_wiki gate -- --nocapture`.
