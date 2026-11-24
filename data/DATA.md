# The game's data (`data/`)

The game, the player, the Workshop and the tools read only these files, never
the ROM. The whole folder is ignored by git (it comes from each person's own
copy of the game), except this file.

## The layers

| Layer | Who writes it | Contents |
|---|---|---|
| `data/rom/` | `extractor <rom>` alone (`extractor/`), which overwrites it | everything the game reads from the ROM |
| `data/extra/` | the tools, **one writer per file** (see below) | what gets added: built places, sprites from elsewhere, variants, grounds |
| `data/hand/` | by hand, or `scene_player dungeons` | portraits and pictures of the variants, traps, stairs |
| `data/built/` | the game alone (`game`), a cache | the places a game built on demand, their decor not built yet: written as `decor` writes `extra/` (`decors.txt`, `decor_<list>`, `grounds/`, `backdrops/`) |

A file is looked for in `hand/`, `extra/`, `built/`, then `rom/` (`assets::data::path`).
The **lists** (`assets/src/indexes.txt`: the file and how many of its first
fields make up the key; `0`: blocks, each starting with `place <level>`) are
read merged: the one in `rom/`, then the places built on demand
(`built/<list>`, `built/decor_<list>`), then the one in `extra/` on top,
then `extra/decor_<list>` (what `decor` writes) on top of all (`decors.txt`
too, by its name); a line from above
replaces the one below with the same key. The three readers of this merge:
`assets/src/data.rs` (`text`, `text_kept`),
`film/src/tables/files.rs` (`game_lines`).

All the lists are UTF-8 text, one line per entry, fields separated by tabs;
`#` at the start: a comment. In spoken text, `\n` is a line break.

## `data/rom/` (written by `extractor`)

| File | Written by (`extractor/src/`) | Columns, meaning | Read by |
|---|---|---|---|
| `story.txt` | `story.rs` | the story: `event <n> <name> <chapter> <step>`, then `scene <level> <name>`, `actor`, `backdrop`, `say <who> <face> <text>`, `ask <answers \| >`, `dungeon <n>`, `ground <level>`... (see `lib.rs`) | `story` (the game, `game/src/zone/story.rs`), `film/src/tables/story.rs`, `preview/src/film.rs` |
| `town.txt` (list, blocks) | `town.rs` | the places you walk around: `place <level> <name> <tiles wide> <tiles high>`, `music <song>`, one `solid <0/1...>` line per row of 8 px tiles (`1`: you cannot pass), `area <x> <y> <w> <h> <examined>`, `actor <entity> <x> <y> <direction> <leader>` and what it does, `menu <n>`, `moment <chapter> <step>` | `game/src/zone/` (`begin.rs`, `places.rs`), `assets::tables` (`solid_rows`: `shell`; `actors`: the residents of `film`), `decor game_grounds` |
| `place_things.txt` | `town.rs` | what is set down in these places: `place <level>`, `moment <chapter> <step>`, `thing <object> <x> <y>` | `game/src/zone/places.rs` |
| `backdrops.txt` (list, key 1) | `pictures.rs` | level, name, width, height (px) of each backdrop | `film/src/tables/` (`places.rs`, `story.rs`), `game/src/zone/story.rs`, `preview/src/check.rs`, `decor` (the free levels) |
| `backdrops/<level>.png` | `pictures.rs` | the whole backdrop (its two layers, first frame) | `shell` (`art.rs`, `images.rs`), `decor`, `scene_player place`, `export`, `decor game_grounds` |
| `grounds/info/<level>.bin` | `pictures.rs` | `media::art_files::GroundInfo` (bincode): size, length of a frame, how many of each kind, palettes, colour cycles | `media/src/art_files.rs`, `decor/src/source.rs`, `export` |
| `grounds/pictures/<level>_<f\|u\|i\|v><frame>.png` | `pictures.rs` | each picture of the place: `f` the ground, `u` the layer you pass under, `i` and `v` their palette indices (colours that cycle) | same |
| `monsters.txt` (list, key 1) | `pictures.rs` | number, name (as the copy of the game writes it) | `assets::tables::numbered` (`film`, `stage_view`), `decor/src/write.rs` |
| `sprites.txt` (list, key 2) | `pictures.rs` | Pokémon, animation (0 walk, 1 idle, 2 hurt, 3 sleep, 4 attack, then those of the scenes), width and height of a cell, frame durations (in 60ths, `,`), shadow x, y | `assets::tables::sheets` (`shell`, `film`, `studio`), `export` |
| `sprites/<pokémon>_<animation>.png` | `pictures.rs` | the sheet: one row per direction (facing down, then anticlockwise), one cell per frame | `shell`, `export` |
| `points.txt` | `pictures.rs` | Pokémon, then `x,y` of the head, the hands, the middle | `assets::tables::points` (`shell`) |
| `sprites/<pokémon>.txt` | `pictures.rs` (`extractor <rom> --steps-only` alone) | animation, direction, a marker each frame (2 the blow lands, 1 back in place), each frame's head, hands and middle `x,y;x,y;x,y;x,y` | `assets::tables::steps` (`media`: `Frame::points`, `Frame::mark`) |
| `portraits/<pokémon>_<face>.png` | `pictures.rs` | the faces, 40x40 | `shell/src/art.rs`, `export` |
| `props.txt` | `pictures.rs` | object, animation, width, height, durations, 0, 0, lying flat (1: you walk over it) | `assets::tables::sheets` (`shell`, `film`, `preview`), `decor` |
| `props/<object>_<animation>.png` | `pictures.rs` | like the sprites | same |
| `entities.txt` | `pictures.rs` | number, script name, Pokémon | `stage_view/src/stage/cast.rs` |
| `dungeons.txt` | `pictures.rs` | number, name, floors | (the tools) |
| `items.txt`, `moves.txt` | `pictures.rs` | number, name | `film/src/tables/mod.rs`, `stage_view/src/stage/cast.rs` |
| `dex.txt` (list, key 1) | `lib.rs` | Pokémon, national Pokédex number | `assets::tables::numbered` (`film`), sprites added by hand in `data/extra/` |
| `talk.txt` | `people.rs` (`rom_formats::talk`, `MESSAGE/tbl_talk.tlk`) | what team members say by personality: `monster <n> <name> <group>` (each Pokémon's group), `special <n> <group>` (the story's 0x24 own characters: partners, Grovyle, Chatot...), `say <group> <kind> <n> <string> <text>` (kind: `healthy`, `half_life`, `pinch`, `level_up`, `wait`, `ground_wait`; string: its number in `text_e.str`; text in English, with the game's tags) | (nothing yet) |
| `guests.txt` | `people.rs` (`rom_formats::guests`) | the guests of the story's dungeons: `guest <n> <Pokémon> <name> <level> <HP> <Attack> <Sp. Atk> <Defense> <Sp. Def> <IQ> <exp> <moves ,> <dungeon where it joined> <floor>`, then `dungeon <n> <first guest> <second> <flags ,>` for each dungeon that adds some or has rules (`-`: none; `hidden_land`: Hidden Land's rules (no sending home, no leader change, out if the partner faints) on the first visit, `hidden_land_cleared`: after, `no_recruiting`, `before_side01_boss2nd`) | (nothing yet) |
| `dungeons.bin` | `tables.rs` | `rules::tables::Tables` (bincode): dungeons, floors, items, species, constants | `game/src/game.rs`, `preview/src/art.rs`, `scene_player dungeons`, `shell::data::folder` (the folder that has it) |
| `tilesets/<n>.png`, `_i.png`, `.cycles`, `.bin` | `tables.rs` | a dungeon's tiles, their palette indices, their cycles, the table of pieces | `shell/src/art.rs`, `decor/src/room/` |
| `interface.bin` | `lib.rs` (`UiArt::pack`) | the interface's pictures, font and words | `game`, `preview/src/art.rs`, `export` |
| `sound.bin` | `textures.rs` | `shell::music::Library` (bincode): songs, jingles, noises | `game/src/main.rs`, `shell`, `scene_player sounds` |
| `effects.txt` | `textures.rs` | id, width, height, durations, time, repeats, full screen, sound, point, directions | `assets::tables::effects` (`shell`), `export` |
| `effects/<id>.png` | `textures.rs` | an effect's frames side by side, one row per direction | same |
| `status.txt`, `status/<name>.png` | `textures.rs` | name, width, height, frames; the strip of icons | `assets::tables::statuses` (`shell`), `export` |
| `weather/<name>.png` | `textures.rs` | what crosses the floor in a storm or fog | `shell/src/art.rs` |
| `numbers.png` | `textures.rs` | the damage numbers | `shell/src/art.rs` |

## `data/extra/` (one writer per file)

| File | Only writer | Columns, meaning | Read by |
|---|---|---|---|
| `decors.txt` | `decor` (`decor/src/write.rs`) | name of the built place, level, middle x, y, name shown, entrances `x,y,name;...` | `film/src/tables/places.rs` (`Tables::decors`), `decor` (the levels), `decor game_grounds` |
| `decor_town.txt` (over `town.txt`) | `decor` | the block of each built place, like those of `town.txt` (`place`, `music`, `departure`, `solid`, `area` + `ground`, `link`, `actor`, `say`) | everything that reads `town.txt` |
| `decor_backdrops.txt` (over `backdrops.txt`) | `decor` | the line of each built place: level, NAME, width, height | everything that reads `backdrops.txt` |
| `decor_ground.txt` (over `ground.txt`) | `decor` | the ground of each built place, like `ground.txt` (`jeu`) | everything that reads `ground.txt` |
| `backdrops/`, `grounds/` (built levels) | `decor` | like those of `data/rom/`, at its own levels | like those of `data/rom/` |
| `ground.txt` (list, key 1) | `decor all` (or `decor game_grounds`) | level, `jeu` (the game's grid) or `main` (traced by hand, `library/tables/grounds.txt`), rows of `0/1` separated by `/`; the game's places only | `assets::tables::grounds` (`stage_view` for the fights of `scenes`, `film`, `decor`) |
| `dungeon_floors.txt` | `scene_player dungeons` (`scene_player/src/commands/dungeons.rs`) | dungeon, name, floor, tileset, secondary | `decor/src/room/` |
| `monsters.txt`, `dex.txt`, `sprites.txt`, `sprites/`, `portraits/`, the credits for these sprites (`spritecollab_credits.txt`) | sprites added by hand in `data/extra/`, or by the Workshop (`spritecollab/src/lib.rs`): Tools › SpriteCollab… (the PMDCollab community's sprites, fetched by git into `sources/spritecollab/`; Pokémon past the game's get 2000 + their Pokédex number), and a Pokémon's Inspector › Import from SpriteBot… (a folder or zip in SpriteBot's layout; a variant gets 5000 and on) | as in `data/rom/`; the credits: free text | as in `data/rom/`; the credits: `scene_player pack` |
| `variants.txt` | by hand, or the Workshop's Import from SpriteBot… as a variant (no variant is provided) | variant number, species, word | `assets::tables::variants` (`shell`: `Art::kin`; `film`) |
| `rod_tips.txt` | by hand, with the variants | Pokémon, animation, direction, `x,y` of the rod's tip on each frame (`-`: none) | `assets::tables::tips` (`shell`: `Art::rod_tip`; `film`) |
| `frame_points.txt`, `worn/` | (nobody any more) | leftovers of old tools, read by nothing | nothing |

`scene_player/src/pack.rs` (`scene_player pack`) copies each file a film shows
(the list: `assets::data::film_files`) into its layer, as it is, and the merge
is done again on reading.

## The levels of built places

The game has its levels up to 458. `decor` gives its own to each place it
builds (`level_of`, `decor/src/write.rs`): the one its file states
(`level 459` in the `.decor`), otherwise the one it had in `decors.txt` (if
no other file claims it), otherwise the first free one from `FIRST_LEVEL`
(459): neither a level of the game (merged `backdrops.txt`), nor one of
`decors.txt`, nor one another file states. It is kept in `decors.txt`, and it
is by this level that `decor_town.txt`, `decor_backdrops.txt`,
`decor_ground.txt`, `backdrops/` and `grounds/` are filed.
