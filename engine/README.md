# engine

A scene engine fed with lines of fields, played frame by frame, the same every time. It draws nothing and reads no game's data: its only dependencies are `glam` and `diagnostics`. What it does know of this kind of game (whom it names by number, what it asks of a game, the fight's log) is said in `../ARCHITECTURE.md`, "What the engine knows".

```
lines of fields (a film compiled by `film`, or read_scene / read_fields)  ->  events (read_events)  ->  Director  ->  Stage
```

## Layout
- `who`: who is on stage.
- `beat/`: what a scene does. `Beat` is split into families, one file each with its beats and how they are read (`body.rs`, `motion.rs`...): `Body`, `Motion`, `View`, `Screen`, `Sound`, `Speech`, `Flow`, `Data`, `Play`, plus `Course` for what waits for the player. Each family declares its commands once (`declare!`, `declare.rs`: names, fields read and written), `parse.rs` reads a line into the family it belongs to, `names.rs` holds the words of the language, `asking.rs` what a scene asks of its numbers, `cast.rs` who a beat concerns.
- `language/`: the one text the engine reads, lines kept as fields between tabs (`story.txt`): `read_fields`, `read_scene`. The film language is `language/` and `film/` at the top of the repository; the passes that write out short forms are `story`'s (`story/src/lowering/`).
- `lint/`: what is wrong in a file, one rule per pass (`commands`, `ways`, `numbers`, `places`). `Checker` is given the sizes of the backdrops and what the host understands, and returns `Diagnostic`s (`diagnostics`, the one shape of every block, with the engine's `codes`). `Beat::parse` says why a line cannot be read (`ParseError`) and suggests the nearest command.
- `stage/`: the scene at a given moment, one subsystem per folder or file, each its own model: `camera/` (the view: `Camera`, `Viewport`), `layers.rs` (what is drawn and in which order: `SHEETS`, `Layer`), `character/` (a body: `Actor`, `Figure`), `emotes.rs` (emotes, statuses, rising words: `Emotes`), `light.rs`, `words.rs`, `shots.rs`, `controls.rs`, `placing/`. `direct/` plays each family there, `advance` makes time go by, `motion.rs` holds `Tween`, `Swing` and `Rect`: the game and the player both go through it.
- `director/`: the course of things. `tick` fits in about thirty lines and says what it does; `play.rs` moves the story on, `beats.rs` plays what happens at once, `happen.rs` what happens on stage, `ways.rs` decides where the scene goes.
- `vocabulary`: the `Vocabulary` type, the words of a game (faces, songs, sounds, animations, statuses, host commands). The engine holds none: whoever uses it gives it their own, when reading and to the `Director`. **For another game, this value is what changes**; the Pokémon ones are in `scenes/src/vocabulary.rs`.
- `rehearsal/`: plays a scene without a display, to find what breaks as it runs; what is seen of a film as it plays (`Watch`), one file per kind of remark (`sight`, `spoken`, `framing`, `bodies`).

## Using it
Each frame:

1. `director.tick(&mut stage, frames, view)` returns `Call`s;
2. handle those `Call`s: show someone, play a sound, show a line, or `Call::Host` for a line only the host understands;
3. `stage.advance(...)`, then draw what `stage` says.

`engine/tests/play_a_scene.rs` shows it in a few lines, `scene_player/` is a complete example. This game translates its `Call::Host` lines in `scenes/src/commands.rs`.

To write a film, start with `../film/FILMS.md` (the film language, which writes these commands). The engine's commands are described in `docs/COMMANDS.md`, summed up in `docs/CHEATSHEET.md`.

## Checking
```bash
sh tools/verify.sh
```

It checks the crates' rule (`tools/rule.sh`), builds everything, runs every test, clippy, the format and the documentation.
