# Architecture

## The words

| Word | Meaning |
|---|---|
| **Film** | The whole thing: a complete story, written in a `.film` file |
| **Scene** | A part of a film, in one place |
| **Beat** | A single action in a scene: walking, saying a line, jumping |

## The building blocks

```text
7   workshop                   scene_player                   dungeon_lsp
    Dungeon Workshop           a film or a game in a window   a language server for editors
    (its parts: workshop_*)

6   game
    a game: its story, places, dungeons, screens; films as its cutscenes

5   preview                    dive                           extractor
    a film being worked on,    a dungeon on screen            extracts data/rom/ from the ROM, once
    with no window

4   stage_view                 film
    everything that is drawn   the films' language whole; under it a crate a
                               family of words (film_acting, film_moving,
                               film_fishing, film_dungeon, film_fights), each on
                               film_core

3   film_core                  shell                          rom_formats            decor
    what the words say: the    window, sound played,          the ROM's formats      places made for films
    text read, the scenes      Screen, the art kept
    compiled, by the words
    it is given

2   scenes                     story                          media                  spritecollab
    this game on the engine:   the game's story: story.txt    pictures, font, music, the community's sprites
    its words, its commands,                                  as data: no window,    fetched by git and
    its fights                                                no sound device        imported

1   engine                     rules                          assets                 language
    plays the scenes           rules and data, no engine      the files of data/,    the language of
                                                              read and merged        films and scenes

0   diagnostics                                                  emulator
    what is wrong, said one way by every block (`Diagnostic`);   a libretro core run (melonDS):
    words compared (`text`)                                      the Workshop's Emulator space
```

Each block is a crate, its folder named as its package.

### Names

One name per crate: **its folder is its package**, so a name seen in a `use`, a `Cargo.toml`, `cargo -p` or `tools/rule.sh` is also where its code is. A library has a short plain name, what it is (`engine`, `film`, `game`); the game's core is `rules` because `core` is Rust's own crate. A crate that makes a program is named after the program one runs (`scene_player`, `workshop`, `dungeon_lsp`, `extractor`; `decor` is a program and a library): the commands typed and the docs keep those names, and the Workshop finds `extractor`, `scene_player` and `decor` beside itself by them. The Workshop's parts are libraries named `workshop_<part>` (`workshop_script`, `workshop_decor`...), under the program `workshop` that ties them. The ROM's formats are `rom_formats`, not `rom`, so as not to be taken for `data/rom/` (the files extracted). Where a crate has a module named as a crate it uses (`film`'s `compile::scenes`, `workshop_base`'s `game`), that file writes the crate `::scenes`, `::game`.

| Folder and package | What it is |
|---|---|
| `diagnostics` | what is wrong, said one way by every block; words compared |
| `assets` | the files of `data/`, read and merged |
| `language` | the language of films and scenes, read once |
| `engine` | plays scenes frame by frame; a game's words handed to it (see "What the engine knows") |
| `rules` | the game's rules and data, no engine |
| `media` | pictures, font, music, the console's colours, as data: no window, no sound device |
| `shell` | window, sound played, `Screen`, the art read and kept |
| `spritecollab` | the PMDCollab community's sprites fetched by git and imported, SpriteBot's layout |
| `scenes` | this game on the engine: its words, commands, fights |
| `story` | the game's story (`story.txt`) as engine lines |
| `stage_view` | everything that is drawn |
| `film_core` | the films' language by the words it is given: the text read, the scenes compiled, what is written for the engine; the families it leans on |
| `film_fishing` | the films' fishing: the line cast, what bites and comes out, what was caught and measured, those watching |
| `film_fights` | the films' fights: the lines a fight is written in, a directed fight's block, the blows, dodges and evolutions |
| `film_dungeon` | the films' dungeons: a floor played as the game plays it, its log, stairs, traps and shops; the guild's jobs |
| `film_moving` | the films' moving: being put, walking, coming in and going out, swimming, flying, riding, hiding, contact, the body's gestures |
| `film_acting` | the films' acting: what someone says and does where they stand, poses, emotions and gestures played with the game's own sprites |
| `film` | the films' language whole: every family of its words gathered (`LEXICON`), a `.film` turned into what the engine plays, the language's service for editors |
| `rom_formats` | the original game's formats, for `extractor` alone |
| `decor` | places made for films; the `decor` program |
| `preview` | a film being worked on, with no window |
| `dive` | a dungeon on screen |
| `game` | a game: story, places, dungeons, screens; its `.game` file (`game_file`) |
| `extractor` | the program that writes `data/rom/` out of the ROM |
| `scene_player` | the program that plays a film or a game, and its commands |
| `emulator` | a small libretro frontend: a core the user chose loaded, a game run frame by frame (program `emulator`, headless) |
| `dungeon_lsp` | the language server (program `dungeon_lsp`) |
| `workshop_base` | what every part of the Workshop shares: the `Session`, the look, the project's files, safe saves, the system's dialogs, the spaces and what a part asks of the window (`Ask`) |
| `workshop_picture` | the Resources' painter: pictures and animations painted as in a pixel-art studio |
| `workshop_sound` | the Resources' sound maker: sounds made as the sound makers of games make them, then mixed; the instruments music plays with |
| `workshop_music` | the Resources' music maker: a song of tracks, regions and notes, its mixer, MIDI files |
| `workshop_dungeon` | the dungeon editor: dungeons and boss fights made, checked, tried; the Dungeon space that lists them |
| `workshop_wiki` | the Wiki: a page for each word of the films' language, the words' catalogue by family |
| `workshop_script` | the Film space's Script: a film's text edited, the language's service beside it, the whole project's films |
| `workshop_decor` | the Decor space: the decor open, its Scene, Hierarchy and Inspector, its brushes, the Library of pieces, the decors' tool at work |
| `workshop_film` | the Film space played: the film of the Script on its thread, its Scene, Timeline and Inspector |
| `workshop_resources` | the Resources space: the shelves of the game's and the project's things, their Inspector, and what is being made in tabs (`Maker`) |
| `workshop` | Dungeon Workshop, the editor |

A block only uses blocks from the layers below it, and only those that `sh tools/rule.sh` allows; it fails otherwise, and when this table is not its list (path dependencies, the `[dependencies]` of each `Cargo.toml`; test ones are free):

<!-- rule -->
| Block | Uses |
|---|---|
| `diagnostics` | nothing |
| `engine` | `diagnostics` |
| `language` | `diagnostics` |
| `assets` | nothing |
| `rules` | `diagnostics` |
| `media` | `assets` |
| `shell` | `assets`, `media` |
| `spritecollab` | `assets`, `diagnostics` |
| `scenes` | `assets`, `engine`, `rules` |
| `story` | `engine` |
| `rom_formats` | `media`, `rules` |
| `decor` | `assets`, `diagnostics`, `media` |
| `film_core` | `assets`, `engine`, `language`, `rules`, `scenes`, `story` |
| `film_fishing` | `engine`, `film_core`, `rules`, `scenes` |
| `film_fights` | `assets`, `engine`, `film_core`, `film_dungeon`, `rules`, `scenes` |
| `film_dungeon` | `engine`, `film_core`, `scenes` |
| `film_moving` | `engine`, `film_acting`, `film_core` |
| `film_acting` | `engine`, `film_core`, `scenes` |
| `film` | `assets`, `engine`, `film_acting`, `film_core`, `film_dungeon`, `film_fights`, `film_fishing`, `film_moving`, `language`, `scenes`, `story` |
| `stage_view` | `assets`, `engine`, `media`, `rules`, `scenes`, `shell` |
| `preview` | `assets`, `engine`, `film`, `media`, `rules`, `scenes`, `shell`, `stage_view` |
| `dive` | `rules`, `scenes`, `shell`, `stage_view` |
| `extractor` | `engine`, `rom_formats`, `rules`, `shell`, `stage_view` |
| `game` | `assets`, `decor`, `dive`, `engine`, `film`, `language`, `preview`, `rules`, `scenes`, `shell`, `stage_view` |
| `scene_player` | `engine`, `film`, `game`, `language`, `preview`, `rules`, `scenes`, `shell`, `stage_view` |
| `emulator` | nothing |
| `dungeon_lsp` | `assets`, `decor`, `film`, `game`, `language` |
| `workshop_base` | `assets`, `decor`, `film`, `game`, `preview`, `shell`, `spritecollab`, `stage_view` |
| `workshop_decor` | `assets`, `decor`, `film`, `scenes`, `shell`, `workshop_base` |
| `workshop_dungeon` | `assets`, `decor`, `dive`, `film`, `game`, `media`, `rules`, `shell`, `workshop_base` |
| `workshop_film` | `engine`, `film`, `preview`, `shell`, `stage_view`, `workshop_base`, `workshop_decor`, `workshop_script` |
| `workshop_music` | `film`, `rules`, `shell`, `workshop_base`, `workshop_sound` |
| `workshop_picture` | `film`, `workshop_base` |
| `workshop_resources` | `assets`, `film`, `game`, `rules`, `scenes`, `shell`, `spritecollab`, `workshop_base`, `workshop_decor`, `workshop_dungeon`, `workshop_music`, `workshop_picture`, `workshop_script`, `workshop_sound` |
| `workshop_script` | `assets`, `decor`, `film`, `rules`, `scenes`, `shell`, `workshop_base`, `workshop_wiki` |
| `workshop_sound` | `film`, `rules`, `shell`, `workshop_base` |
| `workshop_wiki` | `engine`, `film`, `preview`, `workshop_base` |
| `workshop` | `assets`, `decor`, `emulator`, `engine`, `film`, `game`, `media`, `shell`, `spritecollab`, `workshop_base`, `workshop_decor`, `workshop_dungeon`, `workshop_film`, `workshop_resources`, `workshop_script` |
<!-- end of rule -->

So `preview` plays and draws films and knows no game (no `decor`, `language` nor `dive`); `game` uses `preview`, never the other way; only `extractor` uses `rom_formats`; `assets` reads `data/` for everyone. The engine reads no game's data nor rules: its faces, songs, animations and the commands only a game plays are handed to it (`Vocabulary`). It does know a few things of this kind of game, said exactly in "What the engine knows" below.

| Block | Folder | Role |
|---|---|---|
| **Diagnostics** | `diagnostics/` | What is wrong, said one way by every block: `Diagnostic` (its code, its weight `Severity`, where it is `Location`: a file, a line and columns, the bytes; its message, notes, fixes as `Edit`s of the text; `mentions`: those its message names, typed `Mention`s, a Pokémon of a scene by its species and where its name is in the message, so a film says it by its own name without reading the words: `Said` writes them, `film::Written::by_name` says them), `applied` (a text with its edits made), `span.rs` (`Span`, `Source`, `Position`: bytes, lines, columns). `text.rs`: words as they are compared, one table of accents for every block (`normalise`, by what it keeps: `Kept::Letters` the rules', `Words` the films', `Joined` the places'), `distance`, `similarity`, `alike`, `nearest` (the names nearest one not known). Each block keeps its own codes: `language::diag::codes`, `engine::codes`, `film::codes`, `rules::own::codes`, `game::dungeon_check::codes`, `game::project_check::codes`, the server's (`scene_player`). Knows nothing else; depends on nothing |
| **Language** | `language/` | The language of films and scenes (`language/LANGUAGE.md`), read only once: lexer, expressions, lines, blocks, the tree where each piece knows where it is written (`ast.rs`), what is wrong along with its fixes (`diag.rs`), the text tidied up (`format.rs`). Knows nothing of the game |
| **Engine** | `engine/` | Knows who is where and what is happening, frame by frame. Reads no language: a film comes to it as lines of fields, compiled by `film`; the one text it splits is the game's story, `story.txt`, fields between tabs (`engine/src/language/`: `read_fields`, which `story` calls). Its only dependencies are `glam` and `diagnostics` (what is wrong, said as every block says it) |
| **Rules** | `rules/` | No window, drawing, sound, ROM or engine: `attack/` (a move), `dungeon/`, `run/` (a game: `Run::step(Input) -> Vec<Sight>`), town, jobs, services, `tables.rs`, `strings.rs` (the interface's words) |
| **The game on stage** | `scenes/` | This game on the engine, with the rules of `rules/`: `vocabulary.rs` (`POKEMON`, its words handed to the engine), `commands.rs` (what only the game knows how to play), `fight/` (fights on stage) |
| **Files** | `assets/` | The files of `data/` as everyone reads them (`data/DATA.md`): `data.rs` (where a file is in its layers, the merged lists, `text_kept`), `tables.rs` (the tables read: `backdrops`, `grounds`, `built`, `numbered`, `entities`, `solid_rows`, `sheets`, `effects`, `statuses`, `points`, `variants`, `tips`, `actors`), `data::film_files` (what a film pack copies), `project.rs` (`Project`: where a project's films, decors, dungeons, fights, sounds, images, tilesets, recipes are, and its root; the shared library `library/` is a project, a game's the folder of its `.game` (`games/<name>/`), each file looked for there first, then in the shared library; `find_game`, `games`, `new_game`; `find_root`, the one root finder, called by the programs' `main` only: see "The root, the data, the context"). Nothing but std |
| **Media** | `media/` | What is shown and heard, as data: pictures (`images.rs`), the game's picture files read (`art_files.rs`), tilesets (`tileset.rs`) and the template one's own are drawn on (`template.rs`), the console's colours (`quant.rs`), font (`font.rs`), songs and banks (`music.rs`) and the synthesizer (`audio.rs`), project files (`pack.rs`). No window, no audio device, no program run, no network: what `rom_formats` and `decor` use |
| **Shell** | `shell/` | Window and keyboard (`minifb`), sound played (`rodio`: `sound.rs`, `record.rs`, our own sound files `own_sounds.rs`), a hand-drawn `Screen` (`screen.rs`, `blit.rs`), the game's art read as it is needed and kept (`art.rs`, `sheet.rs`), where to find it (`data`, the one from `assets`); `media`'s `images`, `art_files`, `audio`, `music` reached through it too. Knows neither rules nor scenes |
| **SpriteCollab** | `spritecollab/` | The PMDCollab community's sprites and faces: fetched by a sparse `git clone` into `sources/spritecollab/`, imported into `data/extra/` in the game's own format, a Pokémon's sheets written out and read back in SpriteBot's layout. The one block of the bottom that runs a program and reads the network; only the Workshop uses it |
| **Drawing** | `stage_view/` | `stage/`: `Sim` (`sim.rs`), the scene playing with no window, sound or picture (the director, the `Stage`, the fight, everyone's pose, the music, the clock; what it would hear, listed in `heard`), which can be cloned whole; `Play`, the `Sim` and the pictures read for it; `stage/draw/` draws it sheet by sheet (`SHEETS`), changing nothing in it; `ui/`: `Canvas`, windows (`windows.rs`), text (`writing.rs`), the dialogue box (`message.rs`), the cursor and the top line (`menus.rs`), the widgets every screen of the game and of a dungeon is made of (`widgets.rs`: a header, rows with the cursor, a second column, a titled list, a box of choices, a panel of text; their keys as state: `ListCursor`, `YesNo`), `UiArt` (`packed.rs`); `overlay/`: bubbles, lights, pictures over a scene |
| **ROM** | `rom_formats/` | The original game's formats (`script/`, `wan.rs`...) and its scripts played: `vm/`, one file per family of operations, and `acting.rs`. Only `extractor` uses it |
| **Extraction** | `extractor/` | `extractor <rom> [folder]`: everything the game reads from the ROM, written once into `data/rom/` (`story.rs`, `town.rs`, `tables.rs`, `pictures.rs`, `textures.rs`, `rom_data/`, `record/`); the game does not depend on it. `extractor dump <what> <rom> ...` (`dump.rs`, `extractor --help`): development aids that write some of the ROM's pictures out as raw RGBA, or list a script, to look at by eye (portraits, title sprites, maps, tile entries, menu pictures, objects, effects, emotes, a script, a sprite's offsets); the formats only read and return, the tests that read the ROM (`DUNGEON_ROM`) only check |
| **Preview** | `preview/` | A film being worked on, with no window: `film.rs` plays it frame by frame and keeps it whole at a moment (`Snapshot`); `played.rs` plays it in one go (`Timeline`: when each line plays, who is on stage and who moves when); `check.rs`, what is wrong on reading; `session.rs`, a film being written: its text (`film::doc`), replayed when it changes, a snapshot every two seconds, any moment drawn at once, a field of a line changed by its name; `film.rs` also says what a film asks of a game (`GameRequest`: the engine's typed `GameAsk`, handed over by `Call::Game`, or a save), answered at once when no game plays it |
| **Game** | `game/` | The game, conducting the films: `lib.rs` (`Game`: the title, a place walked about, a cutscene, a dungeon, a screen), `story.rs` (where the story stands, its save), `game_file.rs` (the `.game` file), `roam.rs` (a place's scene made of its decor's life, its ways and zones as rectangles; a decor not built yet built on demand into `data/built/`), `playthrough.rs` (the rules' `Run`: team, bag, money), `dive.rs` (a dungeon or a fight of one's own), `game_screens.rs` (shops, bank, map…), `dungeon_check.rs` and `project_check.rs` (what is wrong in a dungeon, in a whole game), `pack.rs` (a `.dwpack`: the files its game, places, films and dungeons name), `drive.rs` (a game played by a script at the keys, no window: the end-to-end tests, `e2e_tests.rs`) |
| **Player** | `scene_player/` | Plays a film, in a window (`window.rs`) or in one go (through `preview`), and all its commands (`commands/`, `scene_player help`) |
| **Story** | `story/` | Reads the game's story (`story.txt`, tab-separated fields, split by `engine::read_fields`) as engine lines; `lowering/`: `read_text` and the passes that write out in full what engine lines write short. `lines_of`, `read_events`: what the game reads; a `.film` or a `.scene` is refused |
| **Films** | `film_core/`, `film/` | `film_core` reads a `.film` through the language (`text/front/` writes it as lines of words, `ir::Instr` and their `Shape`; a recipe kept as the language read it, written out at each call), by the words it is given (`verbs/lexicon.rs`, `Lexicon`: the families of words in their order, what is read once of them; a film carries its own, `Film::lexicon`). Each word makes its `Beat`s (`emit/`, one kind of beat per file, as in the engine), and the film makes the engine's events itself (`compile/`), with no line read twice. A `.film`: what its words say, a table a family (`verbs/`, each word with its forms, its doc and its function, each family its `FAMILY`; `verbs/engine.rs`, the engine's commands); what a family keeps from one line to the next is its own type, kept by the film (`Film::kept`, `Kept`). What depends on the scene is worked out on the `Stage`, when it plays (`PlaceBeat`, `Spot`). `doc/`: the text as it is written, read back byte for byte, its outline, its edits (`Edit`); `order.rs`: one word order, who acts first; `plan.rs`: the v2.1 vocabulary being built, its families, the words to build; `plot.rs`: the Story family (going to a moment or a scene, `seen`). `film` gathers every family in its order (`LEXICON`) and is what the others use: a film compiled with every word (`compile_with`), `story.txt` through `story` (`read_any_with`, `lines_of`: a `.film` or the story), the language service for the Workshop and agents (`service.rs`), `steps`; its tests compile films with all the words |
| **Places** | `decor/` | Builds the places made for films (`.decor`) from pieces of the game's places, cut out from their ground, and dungeon rooms from the tiles of a floor: `lang.rs` (the language), `cut.rs` and `matte.rs` (cutting, matting, shadows), `weave.rs` (weaving grounds, strips, paths), `build/` (laying out: `place.rs` what gets laid, `mod.rs` line by line), `remarks.rs`, `write.rs` (into `data/extra/`), `room/`, `inventory.rs`, `sheets/` (contact sheets), `fault.rs` (what is wrong, and on which line). A single command, `decor` (`decor/DECORS.md`) |
| **Workshop** | `workshop/`, `workshop_*/` | The editor (`egui`): films, decors, dungeons, boss fights, pictures, sounds, music; the first launch (`first.rs`) extracts the resources from the user's copy of the game; the window that ties its parts, each a crate of its own so that a change builds only that part: `workshop_base/` (the session, the look, the project's files, shared by all), `workshop_script/` (the Script), `workshop_wiki/` (the Wiki), `workshop_film/` (the Film played), `workshop_decor/` (the Decor space), `workshop_dungeon/` (the dungeon and fight editor), `workshop_resources/` (the Resources and their makings: `workshop_picture/`, `workshop_sound/`, `workshop_music/`). See `workshop/README.md` (its map: "The code") |


A `Run` is read through `team()`, `bag()`, `bank()`, `dungeon()`, `options()` and changes only through its methods: the town's rules in `run/town.rs`, what scenes do in `run/story.rs`.

## The root, the data, the context

The programs work in a **root**: `library/` (the shared library: films and recipes, decors, dungeons, fights, images, sounds, music, tilesets, `place_songs.txt`, `tables/`: the pieces places are made of, the grounds traced by hand, the names of pictures and props), `games/` (a folder a game), `film/recipes/`, `data/` (extracted from the user's cartridge, never committed). The repository is one; a release unzipped (`tools/release.sh`) is another, its programs at its top. `tests/` holds only what the tests alone use.

Nothing below a program's `main` looks for any of it. Each `main` finds the root once (`assets::project::find_root`: the first folder up from where it runs, then up from the program, that has `library/films`; `Project::found` says what is wrong when there is none), works out the data folder once (`assets::data::folder(root)`: `DUNGEON_DATA`, else `<root>/data`), and hands them down:

| Program | What its `main` makes and hands down |
|---|---|
| `scene_player` | a `film::Context` (the library, the data, the language `DUNGEON_FILM_LANGUAGE`; `--data` replaces the data folder), given to every command; the developer's options of a film in a window (`DUNGEON_FILM_DEBUG`, `preview::debug::Debug::read`), given to `play`; a packet (`scene_player pack`) is its own root |
| `workshop` | a `Session` (`workshop_base/src/session.rs`): the `film::Context`, how it was launched (`launch.rs`: every variable it reads, `DUNGEON_FILM_DEBUG` too), what is read of the files and kept (sheets, effects, the Pokédex, the words' catalogue, the dungeon editor's offers, the own sounds' samples); given to each space at its making; `reload()` when the Workshop reads its project again |
| `emulator` | nothing: the core, the ROM and the frames named on its command line |
| `dungeon_lsp` | a `film::Context` of the root up from the editor's folder (`initialize`), read again (`reload`) when a file is saved |
| `decor` | the library (`Project`) and the data folder |
| `extractor` | its folder: the one named, else `DUNGEON_DATA`, else `./data` |
| the tests | `assets::testing`: the repository's root, its library, its data folder (through `assets::data::folder`; the fixture `tests/fixture_data/` where that holds no extraction), the same for every block |

`film::Context` (`film_core/src/context.rs`) keeps the film tables of each project read so far (`tables_for`), read again only when told (`reload`, `reload_for` once a game built a place). The tables carry their project, their data folder and the language the lines are said in: what a film is compiled with. The shell's `Art` keeps what it read of the pictures and the sounds' lengths. What stays static is program data that never changes (the vocabulary `POKEMON`, the engine's commands, the properties, the recipes built into the programs) and caches of a file's content by its path and when it changed (`assets::data::text_kept`, the pictures a film shows).

## From a film to the screen

```text
my_film.film             what you write
        │  language/     read only once: the tree (each line its range), what is wrong, at its column
        │  film_core/src/text/    the tree as film lines (`ir::Instr`, `Shape`); a recipe written out at each call with what it is given (`front::called`)
        │  film_core/src/verbs/   each word reads its form (`grammar::bind`, typed values, each where it is written), keeps its family's state, makes its `Beat`s (`emit/`)
beats                    what depends on the scene is left as said: `front_of a`, `arrives`; a number found while playing, by its name (`Num::Var`, `FlowBeat::WaitFound`)
        │  film_core/src/compile/ each scene opened (`Sortie::Scene`), its beats, each line of the film marked: the events, made without reading anything again
events                   `Written::events`: each scene, its `Beat`s (the checker, `engine/src/lint/`, reads these written lines)
        │  director/     plays the beats and waits when it should
        │  stage/        who is where, camera, light
frame after frame
        │  scene_player/ or workshop/
the screen and the sound
```

On each frame, whatever is displaying does three things: `director.tick(...)`, it handles the `Call`s received, then `stage.advance(...)`.

### The bridge from a word to the engine

A word hands the engine typed `Beat`s, never text to be read again: each family of `film_core/src/verbs/` builds them by the constructors of `film_core/src/emit/` (`emit::jump(who, height, frames)`, `emit::music(song)`), and `Film::beat` keeps them for the events. The lines of fields a beat writes (`Beat::fields`, `engine/src/beat/written/`) are only for what shows them: packs, `scene_player fingerprint`, `steps`, the checker; `Beat::written` makes sure each reads back the same beat.

Two ways still go through words, by design:

- **The engine's own commands** a film writes as they are (an Advanced word, a recipe of `film/recipes/`, `> …`): the line's words are typed by the form that fit (`grammar::bind`, the command's forms as the engine declares them: `engine::forms`, each kind's `FORMS`) into `engine::Arg`s, then built by `Beat::build`, the very reading `declare!` gives `story.txt`'s lines (`Film::engine_command`, `film_core/src/emit/command.rs`). A word is typed once, nothing is reparsed from a joined line.
- **Recipes** stay text of the language (`film/recipes/`, the library's): each call is written out and read as the film's own lines.

What is left to type, in order:

1. The three lines the front writes for the engine as text (`film_core/src/text/front/`: `position` in `values.rs`, `measure` in `values.rs`, `waitmoves` in `blocks.rs`), as `Shape::Engine`: they name someone by the film's name, which only the film knows when it plays. A `Shape` carrying the names and a closure making the beat once they are codes would end them.
2. The Advanced words whose form is the engine's (`film_core/src/verbs/engine.rs`, 79 words, and `commands.rs`, every other command): left on purpose. They are already typed end to end: their words bound by the engine's own form (`grammar::bind`), handed over as `engine::Arg`s and built by `Beat::build`, which for a `declare!`d command is the reading generated from its one line (`zoom`, `letterbox`, `float`, `spin`, `jump`...), with the film's waits (`.lasts()`, `.until()`) and its found numbers (`{x}`) handled once in `Film::engine_command`. A constructor in `emit/` for one of them would be a second reading of the same form, to keep in step by hand; one is worth writing only when the word grows a meaning of its own beyond the command (as `frame`, `music` or `pose` did).
3. `Beat::Host` and `Course::Host` (a game's lines of words) and `FlowBeat::Filled` (see "Known debt").

Every core family (Structure to Story) and the optional ones build their beats themselves; since wave 4 the `music` and `pose` words no longer let a plain line (`music peace 1s`, `pose a attack`) fall through to the engine's command.

## The heart of the scene

Each subsystem has one home and one model; beats talk to it through the `Stage`, and `stage_view` draws it with no rules of its own.

**Camera** — `engine/src/stage/camera/`. `Camera` (the field `stage.view`) owns the view: where it looks (`Cam`: a point or someone it follows), how far (`Zoom`), whom it keeps in frame (`framing`, by the size of the bodies, `bodies`), the room left for the dialogue box, its glides (`Pan`), glances, shakes. `shots.rs` plays the view beats (`Camera::direct`) and frames a group (`fit`, never closer than 1; `shot_close` for a `shot`); `follow.rs` moves it on by one frame; `viewport.rs` (`Viewport::of`) says what can be seen of a place, never going past its edges.

**Layers** — `engine/src/stage/layers.rs`. A single ordered list, `SHEETS`: the place (lower layer and its animations), its upper layer (treetops, roofs), the light on the place, bodies by depth (`Layer` then `in_drawing_order`), fishing lines, shots, statuses, emotions, the light on bodies, the thing shown large, the picture in place of the scene (`Cutaway`), then what is drawn at the game's pixels (weather, lights, bubbles, letterbox bars, numbers...), halos, the fade, and on top of the fade the captions. `stage_view/src/stage/draw/` goes through the list and draws each sheet; `overlay/` draws the top ones.

**Characters** — `engine/src/stage/character/`. A body on stage is an `Actor` (where it is, where it is going: `Way`, in the air: `Air`, what it does by itself: `Habits`, how it looks: `Looks`, its next gesture: `Cue`) and a `Figure`, kept from one scene to the next: the directions a variant is painted in (`views`). `habits.rs`: who follows, chases, wanders, steps aside. The species, the sprite and its animation belong to `stage_view/src/stage/` (`Cast`, `pose.rs`). What a body wears is painted into a variant's frames.

**Emotions and statuses** — `engine/src/stage/emotes.rs`. `Emotes` (the field `stage.emotes`) holds the effect each one shows (joy, surprise, sweat, sparkles), the statuses (asleep, confused...), the rising words and numbers; their durations and their point on the body (`HEAD`, `Emotes::status_point`). `stage_view/src/stage/draw/emotes.rs` draws them.

**The rest of the scene** — `light.rs` (`Light`: fade, tint, spotlight, particles, picture in place of the scene), `words.rs` (`Words`: bubbles, caption, letterbox, bars), `shots.rs`, `controls.rs` (whom the player walks, the walls), `placing/` (where one can stand), `direct/` (the beats the scene plays).

## What the engine knows

The engine depends on no other block of the game (`glam` and `diagnostics` only) and reads none of its files, but it is not blank about the kind of game it plays:

- **Whom it can name** (`engine/src/who.rs`, `Who`): `Character(n)`, a character by the number the original game's scripts give it (`0` the hero, `10` the partner: `story.txt` is written so); `Monster(species)` and `Another { species, copy }`, a creature by its species number; `Item`, an item by its number; `Thing`, a prop by its object number; `Drawn`, a picture of a film's own; `Spot`, an unseen point; `Target`. What these numbers mean (names, sprites, portraits) is the game's (`Vocabulary`, the tables): the engine only keeps them apart.
- **What it asks of a game** (`engine/src/beat/game.rs`, `GameAsk`, through `Call::Game`): a way out taken (`Go`), a zone walked into, someone talked to, the menu, money given or taken (`Money`), an item given or taken (`Item`). A game answers them; with none, they are answered at once (`preview`).
- **What it shows of a game's screens**: a line of a fight's log (`ScreenBeat::BattleLog`), the place's banner, the dialogue box with a portrait.

Why: the scenes it plays are this kind of game's (story.txt and the films both name characters, species and items by number), and a typed enum keeps every beat checked where it is read. Making it truly game-neutral is future work (see "Known debt").

## Who decides what

Each rule has a single source. To change it, this is the place, and nowhere else.

| Rule | Where |
|---|---|
| What the camera shows | `engine/src/stage/camera/` (`Camera`, `Viewport`) |
| What is drawn and in what order, who goes in front of whom | `engine/src/stage/layers.rs` (`SHEETS`, `Layer`) |
| The properties of what is shown (name, alias, type, unit, default, advised range, animatable, kinds), the grammar of values, the problem codes | `engine/src/props/` (`PROPS`, `read`); the codes: `engine/src/problem.rs` (`codes`) |
| What is shown as nodes (`Node`, `Transform`, `Looks`, `z`, parent), a property read or set | `engine/src/stage/node.rs` (`Stage::node`, `get`, `set`); its beats: `engine/src/beat/node.rs` (`NodeBeat`) |
| How a value changes over time | `engine/src/stage/motion.rs` (`Tween`, `Swing`) |
| The commands | `engine/src/beat/`, one file per family: `body/`, `motion.rs`, `view.rs`, `screen.rs`, `sound.rs`, `speech.rs`, `flow.rs`, `data.rs`, `play.rs`, `node.rs`, `place.rs`, `game.rs` |
| The game's world: faces, songs, animations | `scenes/src/vocabulary.rs` (`POKEMON`), handed to the engine |
| What only the game knows how to play: dungeons, items, map | `scenes/src/commands.rs`, its words in `scenes/src/vocabulary.rs` |
| What a move does | `rules/src/attack/` (`resolve`, and `foreseen` to estimate it; an explosion on stage goes through it too); the numbers: `rules/src/dungeon/combat/` (damage, accuracy), `rules/src/dungeon/status/` (effects); `run/strike.rs` tells it on the floor, `fight/battle/` shows it on stage |
| A fight announced by a film | `scenes/src/fight/battle/directed.rs`; `film_fights/` only writes its line |
| The size of bodies, for the view and positions | `film_core/src/compile/mod.rs` writes it as `body` lines; `engine/src/stage/camera/shots.rs` (`fit`) makes the framing from it, `stage/placing/` the gaps |
| Where the ground is, the path, the free tile | `engine/src/stage/ground.rs` (`Ground`), on the real scene: `stage/placing/` (`front_of a`, formations, crowds, who comes in) |
| How fast a line is typed out, how long it stays, when it is read too fast | `engine/src/director/reading.rs` (`LETTERS_A_FRAME`, `line_frames`, `read_after`, `least_shown`), for the box, the films, the player and `do` |
| What a fight and a film say alike | `scenes/src/fight/said.txt`, read by `fight/said.rs`, public for `film/` |
| What is noticed about a film as it plays | `engine/src/rehearsal/`, one file per kind of remark (`Watch`, `Seen`), read by `scene_player check`; what a placement found: `Note` |
| When something happens in a film | `preview/src/played.rs` (`Pass`, `Timeline`): in frames as they are seen, slow motion included; `scene_player/src/moment.rs` reads `32s`, `line 90`, `scene 3 end` |
| Reading the game's pictures | `media/src/art_files.rs` (the pictures), from the tables of `assets/src/tables.rs` (`sheets`, `effects`, `statuses`) |
| Bubbles on screen | `stage_view/src/overlay/bubbles.rs` |
| The dialogue box: a line typed out letter by letter, portrait, choices (the game and the player) | `stage_view/src/ui/message.rs` (`Words`, `Said`); a scene's line: `Play::said` (`stage_view/src/stage/said.rs`) |
| Lights, pictures, weather on screen | `stage_view/src/overlay/marks.rs` |
| The films' words | a family a theme, each with its words (`WORDS`) and its `FAMILY`: those the core leans on in `film_core/src/verbs/` (`things`, `heard`, `view`, `sky`, `effects`, `items`, `pictures`, `rituals`, `crowds`, `said`, `timing`, `nodes`, `game`, `aura`), the others a crate each (`film_acting`, `film_moving`: moving, swimming, flying, riding, contact, hiding, body; `film_fishing`, `film_dungeon`: the dungeon and the guild, `film_fights`); their order, the narrower first, in `film/src/lib.rs` (`FAMILIES`, the `LEXICON`); the engine's commands in `engine.rs` and `commands.rs` (the advanced words); the vocabulary, its families and the old words refused in `film_core/src/plan.rs`; all of them in the "All the words" table of `film/FILMS.md`, checked by `film/src/tests/docs.rs`; what is wrong, with the closest name: `film_core/src/check.rs` |
| The game's pictures, their categories, their animations | `library/tables/images.txt` |
| An object on stage: a game item, a decor piece, a picture of our own | `engine/src/stage/thing.rs` (`Thing`); its words: `film_core/src/verbs/things.rs`, `items.rs`; the word for each decor piece: `library/tables/objects.txt`; our pictures: `library/images/` |
| A Pokémon variant | painted by hand in `data/hand/`, its line in `data/extra/variants.txt` (none is provided) |
| What a Pokémon wears | painted into a variant; where its rod's line starts: `pointe` in its table, written by hand in `data/extra/rod_tips.txt`, read by `Art::rod_tip` (`shell/src/art.rs`); its words: `film_fishing/src/` and the recipes in `film/recipes/fishing/`; the small things the words use (bobber, crosshair, footprints): `library/images/` |
| What replaces the scene on screen | `engine/src/stage/light.rs` (`Cutaway`) |
| Where a game file is, the merged lists, the tables read | `assets/src/data.rs` (where, merged) and `assets/src/tables.rs` (read: backgrounds, grounds, built places, names, walls); the keys of the lists: `assets/src/indexes.txt`; each file: `data/DATA.md` |
| Where you can walk in each place | `data/extra/ground.txt`, made by `decor game_grounds`; places traced by hand: `library/tables/grounds.txt` |
| A place made for films | `library/decors/<name>.decor`, built by `decor/` (`cargo run --release -p decor -- build <name>`; language, laying out, matting and shadows, weaving, remarks, contact sheets; also dungeon rooms) from the pieces in `library/tables/pieces.txt`; its name for films: `data/extra/decors.txt` (its walls, its background and its residents: `data/extra/decor_*.txt`), read by `Tables::decors` (`film_core/src/tables/places.rs`) |

## Who writes and who reads `data/`

[data/DATA.md](data/DATA.md): each file of `data/rom/` and `data/extra/`, its only writer, its columns, its meaning and who reads it. In short: `extractor` writes `data/rom/`; in `data/extra/`, each file has one writer (`decor` its own, `decors.txt` and `decor_<list>`, which reading lays over the game's list of the same name; `decor game_grounds` `ground.txt`; `scene_player dungeons` `dungeon_floors.txt`; sprites and variants added by hand, each with its own numbers). A list is read merged by key: `data/rom/`, then `data/extra/`, then `data/extra/decor_<list>` (`assets/src/data.rs`). The levels of built places: `decor` gives them (`level 459` in the file, otherwise the one in `decors.txt`, otherwise the first free one after the game's).

## Where to go

| To... | Go to... |
|---|---|
| Write a film | [film/FILMS.md](film/FILMS.md) |
| All the engine's commands | [engine/docs/COMMANDS.md](engine/docs/COMMANDS.md) |
| The engine in detail | [engine/README.md](engine/README.md) |
| The player and packs | [scene_player/README.md](scene_player/README.md) |
| Make a place | [decor/DECORS.md](decor/DECORS.md) |
| Add a word to films | [docs/ADD_A_FILM_WORD.md](docs/ADD_A_FILM_WORD.md): its family (`film_core/src/verbs/` or a family's crate), its `Word`, its line in the plan (`film_core/src/plan.rs`) and in the DS table (`film_core/src/ds.rs`), its Wiki page, `film/FILMS.md` |
| Add a panel or a part to the Workshop | [docs/ADD_A_WORKSHOP_PANEL.md](docs/ADD_A_WORKSHOP_PANEL.md): its crate, its state with the `Session`, what it asks the window (`workshop_base/src/ask.rs`), its place in `workshop/src/app.rs` |
| Add a command to the engine | its family's file in `beat/`, then `stage/direct/` or `director/happen.rs` |

## Checking

```bash
sh tools/verify.sh
```

It checks the building-block rule (`tools/rule.sh`, the table above included), builds everything, runs every test, clippy (not one warning), the documentation and the format, goes all the way to the end and says what failed; the details are in `target/verify/`.

## Working on one part

Each part (a block, a folder) can be worked on without looking at the rest: you only check it and the blocks that use it, directly or not; the ones it uses have not changed.

```bash
sh tools/parts.sh            # the table: each part, what it uses, who uses it
sh tools/parts.sh decor      # the packages a change to decor touches: decor game workshop dungeon_lsp scene_player
sh tools/verify.sh decor     # checks only those
```

`tools/parts.sh` reads the `Cargo.toml` files (path dependencies, test ones included); a part is named by its folder, which is its package (`rules`, `scenes`, `engine`...). `sh tools/verify.sh <part>` says which packages it checks, then for those alone: the building-block rule, the build, the tests, clippy, the docs and the formatting. With no part, it checks everything.

Who uses whom, near or far, and so what a change to a part also checks: `sh tools/parts.sh` prints it from the `Cargo.toml` files (test dependencies included), so it is never out of date; the rule above is the same thing for the `[dependencies]` alone.

Every test passes without `data/` (as on CI). Where `DUNGEON_DATA` names no folder and `data/` holds no extraction (no `dungeons.bin`), `assets::testing::data` hands the tests a **fixture** instead, `tests/fixture_data/`: a few lists written by hand in the formats of `data/DATA.md`, in its layers (`rom/`: some eighty species and their national numbers, the moves and items the tests name, sprite cells and props of made-up sizes, a backdrop and a ground (walls of our own) for each of `film`'s `PLACES` and the repository's own decors, a few place names, a `story.txt` of four lines putting two of them on the world map; `hand/stairs.png` and `hand/trap_slumber.png`, one of the library's pictures standing in for the stairs and a trap), no picture, sound or text of the game; `DUNGEON_DATA=tests/fixture_data cargo test` runs the tests on it where `data/` is there. A test that needs more than the fixture holds (every species, the story, the pictures and sounds) is marked `#[ignore = "needs the extracted data/ (run with --ignored)"]` and told as ignored, never passed by doing nothing. They are in `assets` (`assets/src/tables/tests.rs`), `decor`, `film` (`film/src/tests/`), `scenes` (`scenes/src/fight/tests.rs`), `preview` (`preview/src/tests.rs`) and `game` (`game/src/tests.rs`, `e2e_tests.rs`, `dive_tests.rs`), and read the data folder through `assets::testing::data`; `sh tools/verify.sh` runs them (`-- --ignored`, step `data-tests`) when `data/rom/dungeons.bin` is there. The other ignored tests are `rom_formats`' (they read a ROM, `DUNGEON_ROM`) and `scene_player`'s check of every film (over two minutes).

## Known debt

What a review found too large to change at once, kept here as the work to come:

- **Typed beats end to end between `film` and the engine**: no `Vec<String>` host lines, no `FlowBeat::Filled` (a line read again once its numbers are found), the written lines (`engine/src/beat/written/`) kept only for `story.txt`.
- **One typed extension point for a game's requests**, instead of `GameAsk`'s fixed list (money, items) and the host's lines of words.
- **The Workshop split** into a model library (its state, its edits, no `egui`) and its `egui` views.
- **A `DataDir` newtype** for the data folder, instead of a `&str` handed everywhere.
- **An engine with no game's notions**: `Who` by the game's own numbers, `GameAsk`'s money and items, `BattleLog` (see "What the engine knows").

## Adding an engine command

A regular command (fields in their place, each read in a single way) is declared **once**, in the `declare!` of its kind of beat (`engine/src/beat/<kind>.rs`, `body/read.rs` for the body), just before its `read`:

```rust
"jump" => Jump { who: Who, height: f32 = 10.0, frames: Frames = Frames(12.0) },
```

its name, its fields in the order of the line, each by the way it is read and written (a `Field` from `engine/src/beat/declare.rs`: `Who`, `f32`, `u16`, `Frames`, `Point`, `Vec2`, `Direction`, `Text`, `Value`, `OnOff`, `Bit`, `Flag`, `Opt<_>`, `OrOff<_>`, `Dash<_>`, `IfAbsent<_> = default`…). From this line come its reading, its writing (read back the same by construction) and the list of commands (`commands()`). What is left: its variant in its kind's enum (the same file, `body/mod.rs` for the body), what it does (`engine/src/stage/direct/`), whom it names if it names someone (`beat/cast.rs`), its form beside its `declare!` (`FORMS`: `Form::new("jump", &["who:code [height:number] [frames:duration]"], "a jump on the spot")`, which the language reads and `engine/docs/COMMANDS.md`'s table of forms is written from: `GOLDEN=write cargo test -p engine forms`), its row in a theme of `COMMANDS.md` (a test checks it says the same), and an example in the round-trip test of `beat/written/<kind>.rs`. A film word that plays it comes on top of that (`film_core/src/verbs/`).

A command with several forms or a reading of its own (`anim … times`, `pose … image`, an `@ x y` or someone, groups) is declared `hand:` (its names only) and keeps its reading in `read` and its writing in `beat/written/<kind>.rs`. `scene_player fingerprint --compare` says whether the films' written lines have changed.
