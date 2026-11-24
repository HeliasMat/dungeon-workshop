# scene_player

A small program that plays a scene by itself, without the game: a window, the sprites, the scenery, the texts and the sounds.

`main.rs` chooses the command, `commands/` does them all, `window.rs` holds the window, `moment.rs` reads moments, `pack.rs` makes a pack. The film itself comes from `preview`, the same as for the Workshop: compiling it (`film`), playing it frame by frame or at once, its images; its drawing, from `stage_view`.

It reads the game's files in `data/` (`rom/`, and what the tools add in `extra/`). So it needs `film` and `preview` for the films, `game` for a game (a `.game`, a `.dwpack`, and a film played alone as a game of one cutscene), `rules` for the tables, `shell` and `stage_view` to show, `ds_export` for `export-ds`.

```bash
cargo build --release -p scene_player
```

```bash
target/release/scene_player.exe play my_game
```

Enter reads a line, the arrows choose an answer, Esc quits.

Space held speeds up, F5 reads the file again and replays it. F2, or `play <film> --auto`, makes the lines go by themselves: the film can be watched without touching the keyboard, except to answer a question. In a playable scene, the arrows walk the character, Shift runs, Enter talks, Tab goes to the next one.

## The commands

`scene_player help` gives them all, `scene_player help <command>` the details of one: see also `film/FILMS.md`. They all read the same way: the word, what it works on (`<film>`: a name in `library/films/`, else in a game's `games/<game>/films/`, or a path; `<game>`: a name in `games/`, or a path), then the options. What they write goes into `target/films/<name>/`. A failure says what to do and ends with a non-zero code.

| Option | Effect |
|---|---|
| `--size <1 to 8>` | `play`, `video`, `measure`: the resolution of the rendering; at 4, 1024 × 768, and the sprites stay sharp even zoomed out |
| `--data <folder>` | all: where the game's files are |
| `--auto`, `--from <label>` | `play`: the lines go by themselves; start from a label |

`scene_player my_film.film` is `play my_film.film`.

## Sending a film

```bash
scene_player pack my_film
```

The result is `dist/my_film.zip`: the player, the whole film, and only the film's images. Unzip it and launch `scene_player.exe`.

This folder holds images and sounds taken from the ROM, and PMDCollab sprites under the CC BY-NC licence: to keep among friends, never to publish. That is why `dist/` is not in git.

## Sharing a game

```bash
scene_player pack-game my_game
scene_player play dist/my_game.dwpack
```

`pack-game` writes `dist/<game>.dwpack`: the game's own files (the `.game`, its places, films, dungeons, fights, recipes, music, sounds, pictures, tilesets) and a manifest, nothing of `data/`; it refuses a picture or a sound that comes from there. Unlike `pack`, it can be published. `play` unpacks a `.dwpack` into `target/packs/<name>/` and plays it with the player's own `data/`. The work is in `game::pack`.

## Limits
- Only the bottom screen is shown.
- The player cannot direct a character, so the minigames (`control`) stay in the game.
- The game's menus and dungeons are skipped: the scene goes on.
