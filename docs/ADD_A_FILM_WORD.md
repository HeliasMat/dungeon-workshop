# Add a film word

A checklist, in the order you will do it. `hop` is the example throughout: its form, its DS status, its page and its code are each one line you can grep.

## 1. Pick its family and its crate

A family is a `WORDS` list plus its `FAMILY` (`film_core::verbs::Family::new("name", WORDS)`).

- **The core leans on it** (things, sounds, the view, the sky, effects, items, pictures, crowds, lines said, timing): a file of `film_core/src/verbs/`, for example `film_core/src/verbs/sky.rs`.
- **Everything else** goes in a family crate, which uses `film_core` and nothing above it:
  - `film_acting/src/lib.rs`: what someone says and does where they stand (`hop`, `think`, `narrate`...).
  - `film_moving/src/`: moving, swimming, flying, riding, contact, hiding, body.
  - `film_fishing/src/`: fishing.
  - `film_dungeon/src/`: the dungeon and the guild.
  - `film_fights/src/`: fights.
- **A new family**: put its `FAMILY` in `FAMILIES` in `film/src/lib.rs`, and change the array's length. The order matters, because the narrower families come first and the first form that fits wins. If the family keeps state from one line to the next, give it a type of its own, kept by the film (`Film::kept`, `film_core::film::Kept`; `film_fishing/src/state.rs` is an example).
- **A new family crate**: add it to `members` in `Cargo.toml` and as a dependency of `film/Cargo.toml`, then add its rows to the crate tables of `ARCHITECTURE.md`. `sh tools/rule.sh` fails until the table and the manifests agree.

## 2. Write the `Word`

In the family's `WORDS` (see `film_core/src/verbs/mod.rs` for the builder and the form notation):

```rust
Word::new("hop", &["who:pokemon [happy|surprised|angry] [times n:number]"], "what it writes, for the catalogue")
    .maybe(hopped),
```

- **Forms**: one string per way of writing the line, the narrower first. Use the notation of `film_core/src/verbs/mod.rs`: `who:pokemon`, `*who`, `where:place`, `d:duration`, `n:number`, `a|b`, `[...]`, `text...`, `rest...`. A single reader, `film_core/src/grammar.rs`, checks every line against them.
- **How it plays**:
  - `.does(f)`: `fn(&mut Film, &Line)`, the word always plays its line.
  - `.maybe(f)`: `fn(&mut Film, &Line) -> bool`. Returning `false` hands the line to the next family, or to the engine's command of that name.
  - In either one, build typed beats with the constructors of `film_core/src/emit/` (`emit::jump`, `emit::music`...). Never write text for the engine to read again.
- **Who it brings on stage**, if its line puts someone there mid-scene: `.brings(&["who"])`, `.puts(..)`, `.arrives(..)` or `.brings_with(f)`, plus `.when("word")` if only one form brings someone.
- **How long its line lasts**: `.lasts()`, `.until()` or `.instant()` (`film_core/src/time/`).
- **A word made of other words**: write it as a recipe in `film/recipes/` instead. It is `recipe(...)` in the plan, not `word(...)`.
- **Advanced words** have the engine's own form (`film_core/src/verbs/engine.rs`). Only write one when the word means more than the command does.

## 3. Its entry in the plan

`film_core/src/plan.rs`, `WORDS`, in its family, with the form as an author reads it:

```rust
word("Movement", "hop", "who hop [happy|surprised|angry] [times n]"),
```

The family name is one of `FAMILIES` or `OPTIONAL` there. The plan's family is where the catalogue, the Wiki and the completion show the word.

## 4. Its DS status

`film_core/src/ds.rs`, `WORDS`, under its family:

```rust
("hop", Export::Direct, "ExecuteCommon<actor>(CORO_JUMP_HAPPY|SURPRISE|ANGRY_FUNC_SERIES, 0), WaitExecuteLives"),
```

Pick the `Export` that fits: `Direct` (the game's own operations), `Close` (written, but with less; then also say what it loses in `LOSSES`, same file), `Work` (possible, not written yet), or `None` (would need a patch of the game's code). Every word of the plan needs a status (`every_word_of_the_plan_has_its_ds_status`).

## 5. Its Wiki page

In the family's file of `workshop_wiki/wiki/` (`movement.txt` for `hop`; the format is described at the top of `workshop_wiki/src/written.rs`):

```text
== hop
> One paragraph, for an author: what the word does.
scene: beach pikachu piplup
pikachu hop happy times 2
```

The example is a small film. It must read with no problem and no remark, and play with nothing to say. That is the gate (`workshop_wiki/src/tests.rs`, one `gate_<family>` test per family).

## 6. FILMS.md

- **The family table** near the top of `film/FILMS.md` ("Family | Words"): add the word's form there by hand.
- **"All the words"**, between its `words` marks, is generated from the code. Rewrite it with:

```bash
GOLDEN=write cargo test -p film docs
```

## 7. Look at it

Copy the Wiki example into a film of your own in `library/films/` (that folder is yours and is never committed), then:

```bash
cargo run -p scene_player -- check my_trial
cargo run -p scene_player -- try my_trial 2s line 4    # writes target/films/my_trial/try.png
```

Look at the sheet against the person's half of the gate (CONTRIBUTING.md, "The quality gate of a word"): the game's own assets, pixel-crisp, nothing moving by itself.

## 8. Run

```bash
cargo check -p film_acting                    # or the family's crate
cargo nextest run -p film -p film_core -p workshop_wiki
sh tools/verify.sh                            # once, at the end
```

To list the plan's words that are still to build: `cargo test -p workshop_wiki gate -- --nocapture`.
