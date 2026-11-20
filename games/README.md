# Games

Each game is a folder of its own, `games/<name>/`: everything it is made of, and nothing else. Copy the folder to a friend, put it in their `games/`, and they can play it (with their own copy of the cartridge's resources, never sent along).

```text
games/
  my_game/
    my_game.game                 its title, its team, its chapters (the folder's name, the file's name)
    films/                       its cutscenes: play <film>
    decors/                      its places, a .decor each, their [life]: who lives there, what they say, ways out, zones
    dungeons/                    its dungeons: dungeon <name>
    fights/                      its boss fights: fight <name>, or a dungeon's fight: <name>
    music/  sounds/              its own music and sounds (made in the Workshop)
    images/  tilesets/           its own pictures and dungeon tilesets
```

Only the folders a game uses need to be there.

## Where a name is looked for

A game names its films, places, dungeons, fights, pictures and sounds by their words (`play the_arrival`, `go my_market`, `dungeon my_forest`). Each one is looked for:

1. in the game's own folder (`games/<name>/films/the_arrival.film`…);
2. then in the shared library, `library/` (`library/films/`, `library/decors/`, `library/images/`, `library/sounds/`, `library/tilesets/`…), which every game can use;
3. the film recipes (`uses fishing`, `include intro`): those built into the programs (`film/recipes/`), then the game's `films/recipes/`, then the library's.

The Pokémon, the game's places by name (`scene: beach_evening`), its music and its sounds come from the resources extracted from your cartridge (`data/`): they are never in a game's folder.

What the Workshop's Film, Decor, Dungeon and Resources spaces make (films, places, dungeons, fights, pictures, sounds, music, tilesets) is written into the shared library, `library/`, where every game finds it; a game made in its Game space gets its own folder, `games/<name>/`, and what is only that game's is moved or copied there by hand. The tests keep their own files in `tests/`, never read by a game.

The programs find `games/` and `library/` side by side (the root: the first folder that has `library/films`, up from where they run, else up from the programs); the resources extracted from the cartridge are in its `data/` (or the folder `DUNGEON_DATA` names).

A place of the library can be given a life of the game's own: copy its `.decor` into the game's `decors/` (same name) and write its `[life]` there; the game reads its own copy, the films still show the same place.

This folder is empty in a new workshop: the games you make go here (the Workshop's **+ New Game**, or by hand).

## Commands

```sh
scene_player play <name>                 # its title screen, in a window (--new, --chapter 2, --headless)
scene_player check-game <name>           # the whole game checked without a window: 0 problems
scene_player check <film>                # one film: a path, or its name (the library's, then the games')
scene_player pack-game <name>            # dist/<name>.dwpack, to share: its files, and the library's it uses
```

A name is `games/<name>/<name>.game`, then `library/<name>.game`; a path to a `.game` works anywhere.

In the Workshop, the **Game** space lists the games and makes a new one (**+ New Game**: the folder, its `.game` and its folders).

## Making one

Step by step, with every file given whole: [Your first game in 30 minutes](../docs/TUTORIAL.md).

1. **+ New Game** in the Workshop's Game space, or by hand: `games/my_game/my_game.game`.
2. Write its head (`title:`, `author:`, `team:`, `home:`) and its chapters (film/FILMS.md, "A game: chapters, places, cutscenes").
3. Its places: a `.decor` in `decors/` (made in the Workshop's Decor space, or a copy of one of the library's with a life of its own).
4. Its cutscenes in `films/`, its dungeons in `dungeons/`.
5. `scene_player check-game my_game` until 0 problems, then `scene_player play my_game`.
