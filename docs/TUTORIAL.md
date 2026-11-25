# Your first game in 30 minutes

From an empty Workshop to a `.dwpack` your friend can play: a game of two chapters, a place with
two residents whose lines change, a cutscene where two friends talk, a little dungeon with a boss.
Every file is given whole; type it or copy it.

What you need: the Dungeon Workshop (a release zip, or built from the sources: `cargo build --release`)
and your own dump of *Pokémon Mystery Dungeon: Explorers of Sky* (a `.nds` file, North American or
European release). Commands are written for the folder of the Workshop (where `library/` and `games/`
are); from the sources, the programs are in `target/release/`.

The game we make: **My First Game**. Cyndaquil arrives at the orchard, where Chikorita waits.
Sunflora has lost her sun hat in the cave behind the orchard; the two friends go and get it back from
Kricketune.

---

## 1. The first launch (5 minutes, once)

1. Open the Workshop (`workshop`, or `workshop.exe` on Windows).
2. **You see** "Welcome! One thing to do before anything else.", and why: the Workshop contains nothing
   of the game, it reads your own copy once, into a `data/` folder beside it (about 2.5 GB).
3. Click **Choose Your .nds File…** and pick your copy (or drop it on the window).
   **You see** in green: "✓ Explorers of Sky, North America (C2SE): the tool reads this one."
   (In red instead, it says what is wrong: another game, the Japanese release, not a DS dump.)
4. Click **Extract the Resources**. **You see** four steps, one after the other, each ticked with how
   long it took, and below them the last lines of what the step is doing:
   reading your cartridge (the longest, a few minutes), the dungeons' floors, the pieces of the places,
   the places built. Keep the window open.
5. When it is done, the Workshop opens on the **Game** space, empty: no game yet, it is yours to make.
   A small **What to Try First** panel says where to begin; **Close** it.

Without a window, the same thing is:

```sh
extractor my_copy.nds
scene_player dungeons
decor inventory
decor all
```

## 2. Make the game's folder (1 minute)

1. In the **Game** space, on the left under **GAMES**, type `My First Game` in the box (it says
   "The Big Quest" in grey), then click **+ New Game**.
2. **You see** `My First Game · my_first_game.game` in the list, open: its card (Title, Author, Team,
   Home), its file below, one chapter box in the middle ("Chapter 1 · The Beginning", "■ the end").

It made the folder `games/my_first_game/` with `my_first_game.game`, and the empty folders `films/`,
`decors/`, `dungeons/`, `fights/`, `music/`, `sounds/`, `images/`, `tilesets/`.
(By hand: make those folders, and the file with the text below.)

3. On the card, type `Me` in **Author**, `cyndaquil` and `chikorita` in **Team** (the hero, the partner).
   We set **Home** in step 3, once the place exists.

## 3. A place, with two residents whose lines change (5 minutes)

A place is a decor: what it looks like, then who lives there. Make
`games/my_first_game/decors/orchard.decor` (in the **Decor** space you can draw one with the mouse
instead, and add its life after):

```text
# The orchard: grass, a path to the cave, a few trees and flowers.
decor: orchard 512 384
name The Orchard
center 256 220
ground herbe_village
path terre_bourg from 256 384 to 256 232 to 120 232 width 32
entrance 256 376 down

put e156_73 40 40, 400 24
put e156_74 40 40, 400 24
put e156_15 216 152, 336 248, 440 200

[life]
# Who lives in the orchard, and what they say: before the hat is found (chapter 1), after (chapter 2).
resident sunflora 300 268 left
says if chapter = 1: My sun hat! The wind blew it into the cave behind the orchard...
says if chapter = 1: Could you get it back for me? Please?
says if chapter >= 2: My hat! You found it! I feel the sun on my petals again.

resident bidoof 200 196 down
says if chapter = 1: The cave? Yup yup, it's dark in there. Take an Oran Berry or two.
says if chapter >= 2: Yup yup! Heroes of the orchard, that's what you are now!
```

Above `[life]` is what the place looks like: its size, its ground, a path, the entrance the team
comes in by, and elements of the game's places laid where you say (`put <element> x y`; the Decor
space's **Elements** shows them all, or `decor inventory`). Below, who lives there:
`resident <pokemon> x y facing`, then its lines. `says if chapter = 1:` is only said in chapter 1.
The place is built the first time it is needed (or now: `decor build orchard`).

Back in the Game space, on the card, **Home** › `orchard`, and its entrance › `down`.
   The team comes back there if it faints in a dungeon.

## 4. A cutscene: two friends talking (5 minutes)

Make `games/my_first_game/films/a_new_friend.film`:

```text
# My First Game, chapter 1: Cyndaquil arrives at the orchard, where Chikorita waits.
title: A New Friend

scene: orchard chikorita
chikorita put 256 220 facing down
chikorita look around
cyndaquil enter via down to 256 300
chikorita (happy): There you are! I was starting to think you got lost.
cyndaquil (sigh): I did get lost. Twice.
chikorita walk to cyndaquil
chikorita hop happy
chikorita (joyous): Then today is the perfect day to start a rescue team!
cyndaquil (surprised): A rescue team? Us?
chikorita nod
chikorita (determined): Sunflora lost her hat in the cave. Someone has to find it.
cyndaquil face up
wait 1s
cyndaquil (determined): ...All right. Let's go and ask her!
```

Every line starts with who acts, then what they do: `chikorita walk to cyndaquil`, `cyndaquil face up`.
`scene:` names the place and who is there from the start; `enter via down` brings Cyndaquil in by the
orchard's `down` entrance. `who (face): words` is a line said with that face, and the face acts on its
own: `(surprised)` makes Cyndaquil jump with a "!" over the head.

And the one for chapter 2, `games/my_first_game/films/the_hat_returns.film`:

```text
# My First Game, chapter 2: back from the cave with the hat.
title: The Hat Returns

scene: orchard cyndaquil chikorita sunflora
sunflora put 300 268 facing left
cyndaquil put 230 300 facing sunflora
chikorita put 260 320 facing sunflora
sunflora (surprised): Is that... my hat?
chikorita (happy): Kricketune was using it as a stage!
sunflora hop happy times 2
sunflora (joyous): Thank you, thank you! Come back any time!
cyndaquil face chikorita
cyndaquil (happy): Our first rescue...
chikorita (joyous): ...and not our last!
```

**Every word has a page.** In the Workshop's Film space, put the cursor on a word (`hop`, `enter`...) and
press **F1**: the **Wiki** opens in a window of its own, with what the word does, how it is written and
a little example that plays. A mistake is underlined as you type, with its fix one click away: write
`walk chikorita to cyndaquil` (the word first, the old order) and the bulb in the margin offers
`chikorita walk to cyndaquil`. Every word, family by family:
[film/FILMS.md, "What you can write"](../film/FILMS.md#what-you-can-write).

**See it**: `scene_player try games/my_first_game/films/a_new_friend.film` checks it and writes a sheet
of pictures (`target/films/a_new_friend/try.png`): Chikorita waiting, Cyndaquil walking in, the lines
with their faces. Or open it in the Workshop (step 7, a click on its chip) and watch it in the Film space.

## 5. A short dungeon and its boss (5 minutes)

Make `games/my_first_game/dungeons/orchard_cave.dungeon`:

```text
# The cave behind the orchard: two floors, then Kricketune and the hat.
name: Orchard Cave
floors: 2
look: Beach Cave
pokemon: Shellos 3 x3, Kabuto 4 x2, Corsola 5 floor 2
items: Oran Berry x2, Apple
recruiting: no
level: 5
boss: Kricketune 8
boss_arena: 11x9
boss_shape: room
boss_says: This hat is mine now! It sings so well in the wind!
boss_falls: Fine... take your hat...
```

`look:` borrows the shape and tiles of one of the game's dungeons; `pokemon:` who you meet (species,
level, how often, on which floors); after the last floor, the boss in its arena. Every word is in
[film/FILMS.md, "A dungeon of your own"](../film/FILMS.md#a-dungeon-of-your-own).

## 6. The chapters: one leads to the second (3 minutes)

In the Game space, the **FILE** box under the card is the game's file: replace its text with this
(or edit `games/my_first_game/my_first_game.game` in a text editor), then **Save** (Ctrl+S):

```text
# My First Game: a game of one's own, its folder a whole: its films in films/, its places in decors/, its dungeons in dungeons/.
title: My First Game
author: Me
team: cyndaquil chikorita
home: orchard down

chapter 1 "The Lost Hat"
  play a_new_friend
  go orchard down
  on talk sunflora if not hat_found
    dungeon orchard_cave
    if won
      set hat_found
      chapter 2
    end
  end
end

chapter 2 "A Hat in the Sun"
  play the_hat_returns
  go orchard down
  on talk sunflora
    finish
  end
end
```

A chapter plays its lines in order (the film, then the orchard), then waits for its rules:
`on talk sunflora` — talking to Sunflora sends the team into the cave; won, it sets a flag and goes on
to chapter 2. In chapter 2, talking to her ends the game (`finish`).

Several dungeons to choose from, as at a crossroads? `open orchard_cave` (one line per dungeon) in a
chapter; in the place's decor a zone and its rule, `zone trails 400 160 24 48` then `on zone trails`,
`map dungeons`, `end` (walking there opens the list of the dungeons opened); and in the chapter, for
each dungeon, what winning it does: `on won orchard_cave` … `end` (`on lost orchard_cave` too).

**You see** two boxes in the middle: *Chapter 1 · The Lost Hat* with the chips `play a_new_friend`,
`go orchard`, `dungeon orchard_cave` and "→ chapter 2"; *Chapter 2 · A Hat in the Sun*, "■ the end".
A chip in red is a name not found: check its spelling.

## 7. Check it (2 minutes)

- In the Game space, on the right, **CHECK THE GAME** › **Check Again**. **You see**, in green, "nothing to review: it plays".
  A problem shows as `file:line · what`: a click opens that file at that line.
- Click the chips: `play a_new_friend` opens the film in the Film space, `go orchard` the place in
  the Decor space, `dungeon orchard_cave` the dungeon in the Dungeon space (marked **game**: the
  game's own; **▶ Try** plays its first floor at once, **Save** writes it back into the game's folder).
- Without a window:
  ```sh
  scene_player check-game my_first_game
  ```
  **You see** `games/my_first_game/my_first_game.game: 0 problems`.

## 8. Play it (5 minutes)

- In the Game space: **▶ Play from Chapter 1** (a window of its own). The title screen: **New Game**.
  Read the lines with Enter; in the orchard walk with the arrows, Enter in front of Bidoof and Sunflora;
  Sunflora sends you into the cave; at its end, Kricketune. Back in the orchard, Bidoof says something
  else now. Talk to Sunflora: **The End**.
- From any chapter: choose it beside the Play button (or `scene_player play my_first_game --chapter 2`).
- Without a window: `scene_player play my_first_game --headless` tells what happens, in order
  (films go by themselves, dungeons count as won):
  ```text
  chapter 1
  film a_new_friend
  film a_new_friend ended
  go orchard
  talk sunflora
  dungeon orchard_cave
  dungeon won
  chapter 2
  film the_hat_returns
  film the_hat_returns ended
  go orchard
  talk sunflora
  the end
  ```

## 9. Send it to a friend (1 minute)

- In the Game space: **Export the Game…**, keep `my_first_game.dwpack`, Save. **You see**
  "exported: … (“My First Game”, 5 files, nothing of the cartridge)".
- Or: `scene_player pack-game my_first_game` writes `dist/my_first_game.dwpack`.

The pack holds your game's files (and what it uses of the library), never anything of `data/`.
Your friend, with their own Workshop and their own extracted copy of the game:

```sh
scene_player play my_first_game.dwpack
```

---

## Where next

- More places: a new one drawn in the **Decor** space, or a place of the game taken as it is and made
  your own (`decor import <level> <name>`; `scene_player search <word>` finds places and Pokémon).
  `exit x y width height to <place> <entrance>` in a `[life]` leads from one place to another.
- A shop, a bank, a job board: `talk shop` under a resident, or `shop jobs` in a rule.
- Bigger fights: `fight: <word>` and a `.fight` file made in the **Dungeon** space (+ New Boss Fight).
- How a game's folder is laid out: [games/README.md](../games/README.md).
- Every word of a film, a game, a dungeon: [film/FILMS.md](../film/FILMS.md); of a decor:
  [decor/DECORS.md](../decor/DECORS.md).
