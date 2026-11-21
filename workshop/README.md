# Dungeon Workshop

```
cargo run --offline -p workshop [file.film | file.decor]
```

One window, seven spaces (Home, Game, Film, Decor, Dungeon, Resources, Emulator), each with its panels docked as in a studio (the Game's columns and the Dungeon's editors take the whole window).

## Game

A game of one's own (a folder of its own, `games/<name>/<name>.game`, see `games/README.md`; `workshop/src/games.rs`), in three columns (**+ New Game** makes its folder, its `.game` and its `films/`, `decors/`, `dungeons/`… folders; a dungeon of the game's own opens in the Dungeon space only if the library has one of that name):
- **On the left**: the project's games (a click opens one; a title and **+ New Game** makes one); its **card**: title, author, team (hero, partner), home (a place and its entrance: where the team comes back to, K.O.), each written into the file as soon as it is let go, its comments and its order kept; under it, **the file** itself, to write the chapters (Save, Ctrl+S; • not written yet).
- **In the middle**: the **chapters**, a box each: what it plays (`film`), the places it goes to (`place`), the dungeons and fights it sends the team to, what it opens on the map; a click opens it in its space (in red: not found in the project); its rules (`on talk kecleon if …`); **→ chapter n** (the chapter it leads to), **■ the end**, or, in red, a chapter that leads nowhere. A click on a chapter's title chooses it to play from and shows its line in the file.
- **On the right**: **▶ Play from Chapter n** (`scene_player play <game>.game --chapter n`, a new game from that chapter at its home, in a window of its own: the file is saved first); **Export the Game…** (a `.dwpack` where one says, `dist/` at first: the game's own files, nothing of the cartridge; it says where it went or why it was refused); **Check the Game**: what `scene_player check-game` finds (names not found, flags set and never read, what the game never reaches), read again at each save; a click shows where (a line of the file, a film at its line, a decor with its line chosen, a dungeon in its editor).

The places' life (who lives there, what they say and when, what talking to them opens, their ways out, their zones, their `on … end`) is written in the **Decor** space: `decor/DECORS.md`.

## Film › Script

The film's text in an editor, like VS Code.
- **The colours**: who speaks, their face, what they say, the words of the language, the names, the times and numbers, the comments.
- **The margin** numbers the lines and marks those the language says something about (red: error, yellow: remark); those words are underlined.
- **PROBLEMS**, under the text, lists what the language says about the film (`film::service::diagnostics`), read again a moment after each change on a thread of its own. A click goes to the line; a button applies a fix offered.
- **Typing** offers the words possible there (↑ ↓, Enter or Tab, Esc; Ctrl+Space to ask for them).
- **Hovering over a word** says what it is, and what is wrong there; a word of the language, in a line, what it does, and **F1** for its page in the Wiki.
- **At the top**: where the cursor is ("Scene 2 · the place as the game names it › moment › line › its word", each piece a click to go there), **Layout**, **Wiki**, **Find**, **View** (word wrap, hints, letter size), **Format**, **DS Mode**.
- **DS Mode** (in the Script's head) adds the line `for ds` to the film, or takes it off: the film is written for Explorers of Sky on DS (docs/DS.md). Then what the DS can't play is told as the film is written (`E-DS-WORD`, `R-DS-LOSS`, `E-DS-ACTORS`, `E-DS-PLACE`, `E-DS-HERO`, `R-DS-ZOOM`, film/FILMS.md), the completion offers only the words that export (those with a loss marked "DS with loss: …"), and the Wiki shows each word's DS badge (**DS direct**, **DS with loss**, **not on DS**, what is lost or why on hover and on its page) with **DS only** to list the words that export. The statuses are those of `scene_player export-ds --table` (`film/src/ds.rs`).
- **The Layout** (on the left): the scenes, their place, how many lines are wrong in them (⚠), their moments; a click goes there; Fold, Unfold all.
- **The Wiki** (a window of its own, opened by **Wiki** in the Script's head or **F1**, to read beside the Workshop while writing): every word of the language (its words, the library's recipes, the engine's commands), searched by its name, its family or what it does (the very word first, then those it begins, then its family, then what they do), by family otherwise. A page for each: what it does, said for an author; how it is written, each form with what each part means, what may be left out and what it is then; an example, a little film (written by hand in `workshop/wiki/*.txt`, or made from the word's form), coloured as the Script colours it, the word's line lit: **Play / Stop the Example** (on the right, in a loop), **Insert at Cursor** (its line under the cursor's, the scene's own two in place of Pikachu and Piplup), **Copy Example**; **See Also**, the words it goes with; a recipe's **How it's made**, its own lines. A word a film does not take as written (the game's now, an engine command a film's words write) says what to write instead. **F1** opens the page of the word at the cursor (or pointed at). A page's example plays at once, large, on the right of its text, the whole scene seen; Insert at Cursor and F1 go to the film being written.
- **At the bottom**: the line said at the cursor in its box, cut as the game cuts it (34 letters, 3 lines; what goes beyond in red); the status bar (line, column, lines, scenes, the film's length; the problems, a click: the next one).
- **The film is written by itself** a moment after the last change.

| Keys | |
|---|---|
| Ctrl+S | write now |
| Ctrl+Z, Ctrl+Y | undo, redo |
| Ctrl+Space | offer the words |
| F1 | the word's page in the Wiki |
| F12 | go to where this name is given |
| F2 | rename everywhere |
| Ctrl+/ | comment out the line (or the lines chosen) |
| Ctrl+D | duplicate the line |
| Alt+↑ ↓ | move the line |
| Tab, Shift+Tab | indent, unindent |
| Enter | a line at the same indent (further in after a block that opens) |
| Shift+Alt+F | format |
| Ctrl+F, Ctrl+H | find, replace (Enter: the next one, Shift+Enter: the one before, Aa: match case) |
| Ctrl+G | go to a line |
| Ctrl+= Ctrl+- Ctrl+0 | letters larger, smaller, as before |
| Ctrl+Shift+[ Ctrl+Shift+] | fold, unfold what holds the cursor |
| Ctrl+Enter | the film plays from this line |
| Ctrl+. | the line's fixes |
| F8, Shift+F8 | the next problem, the one before |
| Ctrl+B, F4, Shift+F4 | mark the line (a bookmark), go to the next one, the one before |
| Ctrl+C, Ctrl+X with nothing chosen | copy, cut the whole line |
| Ctrl+Shift+K, Ctrl+L | remove the line, choose it |
| F12, Ctrl+click | go to where this name is given; on `uses`, `include` or a recipe of a shared file: that file opened |
| Ctrl+wheel | letters larger, smaller |
| Alt+click, Ctrl+Alt+↑ ↓ | one more cursor (what is typed is written at each; Esc: just one) |

And also:
- **models**: at the start of a line, `scene`, `dialogue`, `fight`, `choice`, `together`, `beside`, `when`, `define`, `repeat`, `fishing` offer ready-made lines (Tab unfolds them, at the line's indent);
- **the word under the cursor** is lit wherever it is written;
- **the ruler on the right** shows the errors, the remarks, what is found and the cursor; a click goes there;
- **above the line being written**, the forms of its word and what it wants where the cursor is;
- **the line the film plays** has a green arrow in the margin; a click on a number takes the film there;
- **Word Wrap** wraps long lines or not; the **remarks** box shows them in PROBLEMS or not;
- **fold** the scenes, the moments and the blocks: a chevron in the margin, "⋯ n lines" after a folded line, **Fold / Unfold** all in the Layout; the cursor going in unfolds them;
- **the line of the scene** (and of the moment) stays at the top while what follows is read; a click goes there;
- **a bulb** in the margin where a fix is offered: a click, the fixes; **Fix All (n)** in PROBLEMS applies the first of each, if they do not get in each other's way;
- **at the end of each line** (**Hints**), when it plays, for how long, and what is wrong with it;
- **the words by family** (Ctrl+Space on an empty line): the families on the left (Speech, Movement… Advanced: the engine's commands), their words in the middle, on the right the word chosen, its form, what it does, an example and **that example played** (Pikachu and Eevee on the beach, looping); ← → a family, ↑ ↓ a word, Enter writes it (or "Write the Example");
- **what follows is offered by itself**: a word taken (Tab, Enter) is written with its space and what comes next is offered at once (someone, a place, a word of the form), a space typed too; in grey after the cursor, the rest of the word chosen;
- **moves**: after `move <who> `, the game's moves, found without spaces or accents (`irontai`: "Iron Tail"); the one chosen **plays** under the list (Pikachu uses it on a Krabby);
- **who first, in one go**: `pik` then Tab writes `pikachu ` and at once offers what they may do (the most common first: walk, run, look…), Tab again writes the action and offers what follows (`to`, the places, those of the scene…); the word's forms and "here:" are read on a line that starts with someone too;
- **who first**, as the language writes it (`pikachu walk to right`): at the start of a line, those of the film are offered ("does something", or `name:` for a line said); after a name, the words said of someone; such a word taken from the words by family is written after someone of the scene;
- **Enter after an open block** (`together`, `beside`, `repeat`, `define`, `choice`, `when`, `fight directed`) also writes its `end`;
- **right click on the lines chosen**: play from here, start them together (`together`), beside the film (`beside`), don't wait for them (`&`), make a recipe of them (`define` at the top, called in their place), comment them out, duplicate them;
- **in the margin**: ▶ when hovered plays from the line, the bookmarks in blue; when hovered, what is said about the line;
- **guides**: a thin line at each indent of the blocks; the line the film plays on a green background, the lines found on a yellow one;
- **the ruler on the right** also shows the scenes (blue), the moments, the line played (green);
- **while the film plays** and nothing is being written, the Script keeps the line played in sight;
- **hovering over a name** shows the portrait of its species and its name in the game; **over a place**, its picture and its name;
- **the models** take the first two of the scene in place of `a` and `b`;
- **what is offered** also comes from the game and the project: the places after `scene:`, the faces after `(`, the shared files after `uses`, `include`, the marks after `at`, the species from 3 letters on, the items where a thing is wanted; at the top of the list, the forms of the line's word; "N in all" when there are more; a move plays with the species of the one using it;
- **right click**: also "When … Arrives" (each one of the scene), and "Make a Recipe of Them…" asks for its name;
- the folds and the bookmarks follow their lines when lines are added or removed above them;
- **the panels answer each other**, in blue: a line hovered in the Script frames its bar in the Timeline and those it names on the Scene; a bar hovered lights its line; someone hovered on the Scene (or their name in the Timeline) marks their lines in the margin;
- **a model** unfolded chooses its first `...`, to be written at once; the whole description of the word offered is under the list.

## Film › Scene, Timeline, Inspector

- **The film plays** on a thread of its own, as `scene_player check` plays it: a moment after each change of the Script, only from the line changed.
- **The Scene** shows the game's picture, its pixels whole, at the moment one is at.
  - Under it, a bar: the time (to drag), ⏮ ▶ ⏭ (next scene), the speed (0.5×, 1×, 2×), **Sound**, **Follow the Script**.
  - With **Follow the Script**, the film goes to the line where the cursor is.
  - On the keyboard, the pointer over it: Space plays, ← → by a second (Shift: by a frame), Home.
  - What keeps the film from playing shows in red at the top.
- **Above the picture**:
  - **Film View** (what the camera shows) or **Whole Place** (the whole place, the camera's frame over it);
  - **Names**: the frame and the name of each one (when hovered, the one pointed at);
  - **Grid**: tiles of 8 pixels of the place, wider when they would be too tight;
  - **Image**: this one written as PNG in `target/films/<film>/`;
  - **Loop**, **Follow the Script**;
  - on the right, the point of the place under the pointer.
- **On the picture**:
  - Ctrl+click writes that point (`x y`), or the name of the one pointed at, where the Script's cursor is;
  - right click: a line written at once (`who put x y`, `who walk to x y`, `spot here x y`, `frame x y snap`);
  - **dragging someone** puts them there: the last line of the scene that sets them on a point (`put`, `walk … to x y`…) up to the one playing is written again, else a `put` after `scene:`;
  - dragging elsewhere (or with the middle button) moves the picture, the wheel looks closer around the pointer, a double-click (or **Fit**) puts it back whole;
  - **Shift+drag** measures: pixels, tiles, gap in x and in y;
  - hovering over someone, at the top right, where their feet are in the place;
  - with **Names**, the spots of the film (`spot name x y`) marked with a diamond.
- **Step by line** (for the author who wants to see each line do its work):
  - **Step** in the bar (F6) on: as the film plays, it stops at the start of each new line of its own course, the line lit in the Script (outlined in the Film's colour, kept in sight). Lines played beside the film (`&`) and those of a recipe do not stop it. Several lines starting on the same frame stop it one after the other, the film standing still.
  - **Next Line** (F10, wherever the pointer is): played until the next line starts, stopped there, Step on or not. Space plays on (stopping at the next line again with Step on).
  - A breakpoint (F9 in the Script) stops the film at its line, Step or not.
- **The State** (under the Inspector): the scene at the moment shown, as `scene_player state` tells it (both read `preview::state`):
  - the time and the line playing (or stopped at: a click, it in the Script);
  - **the camera**: its centre, its zoom, who it follows, who it keeps in frame, whether it glides; the bars, the line in the box;
  - **each character**: where they stand, then which way they face, where they walk to, their pose, how high, hidden, their statuses, their HP (a bar too), out of frame;
  - **the objects** the film put: where, held by whom, their image, takeable, shining.
  - A click on a name chooses them, as a click on them on the Scene does: their frame in the Film's colour on the Scene, their lines brought out in the Timeline; a click again: no longer. Hovered, their lines are marked in the Script's margin.
- **Under the picture**, the line playing (a click: it in the Script); J a second before, K plays or stops, L a second after, `,` `.` by a frame; on the time bar, in orange, where each scene starts; ‹ › by a frame; 0.25× too.
- **The Timeline** shows, from top to bottom:
  - **the scenes**, named by their place, then **the moments** (diamonds, their name);
  - **the lines** as bars, from their start to their end, their text inside, coloured by their kind: lines said, waits, sound, camera, actions. A line beside the film is paler and outlined; the cursor's, lighter. When hovered, the line, when, how long, and why it starts there;
  - **the music**: each piece, from the line that asks for it to the next one; the sounds, strokes;
  - **the cast**: each one, when they are on stage, when they move, when they speak; a click on a name brings out their lines ("✕ name" at the top: all);
  - at the very bottom, **the whole film** small, the frame of what is seen of it: a click or a drag goes there.
- **The kinds** of the legend: a click hides or shows them.
- **Changing a line from its bar**:
  - pulling the end of a bar that says its length (`wait 2s`, `… in 1.5s`, a walk) lengthens or shortens it: the time is written again (`in Ns` added to a walk that had none);
  - right click: go to the line, play it in a loop, add `wait 1s` after it, duplicate it, delete it.
- **On the ruler**:
  - a click or a drag goes there; Shift+drag chooses a stretch played in a loop (right click, or "✕ loop", to remove it);
  - the lines with a problem are marked in red;
  - when hovered, the time pointed at.
- **Moving around**:
  - a click on a bar goes there, and puts the Script's cursor on its line; on a scene or a moment, at its start and on its line; a double-click on a scene shows it across the whole width; a click elsewhere goes to that moment;
  - the wheel (or Shift+wheel) scrolls; Ctrl+wheel zooms in or out, around the pointer;
  - **See All** shows the whole film; **Follow Playback** keeps the playhead in sight;
  - Space plays, Home, End; ← → from a line's start to the next one.
- **The Inspector** edits the cursor's line (‹ ›: the line before, the next one; its kind in colour).
  - **A line said**: who speaks (the film's cast), their face (the game's), their manner (fast, slow, top…), what they say.
  - **Another line**: each field of the form that read it when the film played, to change according to its kind. Someone is chosen from the cast, a number is dragged, the rest is written; a place can also be chosen with a click on the Scene ("on the scene", Esc: no). A field not written says its default value.
  - **The word**: its forms and what it does.
  - **When it plays** (a button to go there), and who is on stage at that moment.
  - **What is said about it**, with the fixes offered, in one click.

## Decor

The whole Decor space (Scene, Hierarchy, Inspector, Library, terrains, walls, walking): `decor/DECORS.md`. Wherever a decor is chosen (Home's Decors shelf, the Game space's home, `scene:` in the Script), the decors and places come grouped by kind of place (Town, Beach & sea, Forest & grove, Cave & mountain, Guild & interiors, Dungeon entrances, Other: `library/tables/place_kinds.txt`), each by its name.

- **The Library** (under the Scene, `workshop/src/library/`):
  - **one search at the top**, always there (Ctrl+F over the Library or the Scene, or `/` anywhere in the space; Esc empties it): elements, backdrops, models and the project's decors at once, each kind in its own group, the best first (by name, tag, place; a decor found opens with a click);
  - on the left: **This Decor's Place** (the elements of the place it is made from), Favourites, Recently Laid, All Elements, **Backdrops**, Models, the categories, the game's places by kind;
  - pointed at, an element shows **at its own size** (×2, ×3… while it fits), playing, on a checkerboard.
- **Backdrops**: each place of the game without its elements (its rest, `r<level>`). A double-click, Enter or a drag onto the scene lays the decor on it: `put r<level> 0 0 shadow off layer ground locked`, before the first piece (under them all; locked, so a click takes what is over it); a decor that has one has it changed where it is. Right click: the decor made the backdrop's size too, its place's elements, the backdrop taken off. One step each, undone by Ctrl+Z. (`base level` is left to `decor import`: the Workshop does not draw it.)
- **Laying**: a click on a card takes it as a **stamp**: its ghost follows the pointer exactly where it would land, each click lays one (each its own undo step), Esc or a right click puts it down; a double-click lays it in the middle; a drag lays it where it is let go, its ghost showing where. **Snap** (S, or the tools bar) lays and carries on the decor's grid (`grid n`, else 8); Alt held turns it the other way for a moment. **Lays on** (tools bar): the layer what is laid goes on (Ground: under the characters; Objects: as the game, its top in front of them; Over: in front of everything). **Walls** (C, or the tools bar): where the characters cannot walk, in red.
- **The hint bar** along the top of the Scene: what is on (the stamp, snap, walls, the layer) and the keys of the tool in hand.

| Keys (pointer over the Scene, Choose) | |
|---|---|
| arrows, Shift+arrows | nudge what is chosen 1 pixel, 8 |
| H | flip |
| [ ] | back, forward (one element) |
| L | the elements chosen on the next layer |
| Ctrl+D, Delete or Backspace | duplicate, remove |
| Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z | undo, redo |
| Esc | the stamp put down, else nothing chosen |
| S, C, G, R | snap, walls, grid, rulers |
| Ctrl+F, / | the Library's search |
| V B N M P | Choose, Terrain, Scatter, Walls, Walk |

## Home

The project at a glance (`workshop/src/home.rs`).
- **The Project** (on the left): Recent, ★ Favourites, Films, Decors, Drawn Objects (a click: in the Resources), Shared Recipes (`uses`, `include`), Bin; how many of each.
- **The Files** (in the middle):
  - **Open…**: all the films and decors, by their title; **Search the project** (Ctrl+K): the titles, the files, the places, those who play in them, and the words of their text;
  - a **List** (a click on a column sorts it, again: reversed) or a **Grid**; each file with the picture of its place (a film's first scene, a decor itself);
  - a click chooses, a double-click opens; ↑ ↓ (← → in the grid) another, Enter opens it, Delete: the bin.
- **The Preview** (on the right):
  - the picture of the place; ▶ opens the film and plays it;
  - the **Details**: the kind, the scenes, the places (as the game names them), the characters, the lines said, the lines, the **State** (what the language says of the film: no problems, remarks, errors), when it changed; of a decor, its size and the films set there;
  - **Open**, **Duplicate** ("_copy", the title too), **Rename** (a decor: also in the films set there, if asked), **Bin** (in `target/workshop/bin`), ★ Favourite, Show in Explorer;
  - in the **Bin**: **Put Back** (in its folder) or **Delete Permanently** (asked once more).

## Resources

Everything a film and a decor are made of, like a studio's asset browser (`workshop/src/resources/`).
- **The Library** (on the left), groups › shelves, how many on each; a group folds:
  - **Characters**: Pokémon, Variants, **Portraits** (each face of each Pokémon its own card, named as a film names it: `pikachu (happy)`; the faces 14 and 18, which no film word names, left out);
  - **Objects**: Drawn, The Game's, **Items** (the game's items, by their names, with their 16×16 icon of the dungeon floors, from `dungeons.bin`);
  - **Audio**: Music, Sounds;
  - **Effects**: **Attacks** (each move a film names, by its name, with the effects it shows: before it, as it strikes, on the target, what flies, as `dungeons.bin` has them, the very table `effect <Move> on …` plays), **Status Effects** (the game's 25 icons, `data/rom/status`), **Weather** (the sky's: rain, sun, hail, snow, sandstorm, mist, petals, leaves, clear; each card the beach under it, played), **Other Effects** (the game's effects no move shows: the traps', the items', the scenes');
  - **Places**: **Backdrops** (each place or picture of the game a film opens, `scene: <word>`), **Tilesets** (the game's dungeon tilesets, by the first dungeon floor that has each);
  - **Dungeons**: Dungeons, Boss Fights; Decor (elements); and ★ Favourites.
- **The Resources** (in the middle):
  - a grid of pictures (a Pokémon's portrait, or its body if it has none; an object's first picture; an effect, a status, an attack playing over and over; a wave for a sound), or a **List**;
  - on each picture, **how many films** use it; **Used**: only those; **Sort: usage**: the most used first;
  - **filters**: the Pokémon's generations, the music's moods (calm, mystery, battle…);
  - when hovered, the picture large, what it is, in how many films;
  - **Search everywhere**: in all the shelves at once (the word, the name, what it is); **Sort** by number (the Pokémon in Pokédex order, the effects, music and sounds in the game's) or by name (`Effect 2` before `Effect 10`); **Size** of the pictures;
  - a click chooses, a double-click copies the line of a film that uses it (or plays a sound); ← → ↑ ↓ choose another, Enter as the double-click; right click: copy the word or the line, favourite, listen, open to paint;
  - **+ Create**: **an image**, **an animation**, **a sound**, made in the Workshop (see below); **Import an Image**: a PNG dropped on the window joins the drawn objects under the word of its name (`name.4.png`: four pictures side by side).
- **The Inspector** (on the right):
  - of a Pokémon (or of a variant), two sections: **Faces** (normal, joyous, sad… a click shows it) and **Animations** (each of those of its body: Walk, Idle, Attack, Sleep, Hop… in the 8 directions; a click plays it, and its line follows: `anim pikachu Attack`);
  - of a sound, of a music: **Listen When Chosen** (as soon as it is chosen), and the volume;
  - of a drawn object: **Rename…** (the file, and the films that write it if asked), **Delete…** (into the Workshop's bin, `target/workshop/bin`, from where it is taken back by hand);
  - large on a checkerboard, its pixels whole: the pictures one by one (1, 2, 3…) or played (▶ ⏸); of a Pokémon, its faces (normal, joyous, sad…); of a sound, ▶ Listen, ■ Stop;
  - what it is: its word, its name, its size, its pictures and their length, its file;
  - **In a film**: the line that uses it, **Copy Film Line** or **written in the open film** under the cursor's line: `pikachu hold torch`, `music peace`, `krabby walk to right`, `pikachu (happy): ...`, `pikachu hold Oran Berry`, `effect Thunderbolt on pikachu`, `pikachu status burn`, `weather rain`, `effect pikachu 12`, `scene: beach`; a tileset's is a `.dungeon` line (**In a Dungeon**: `look: Beach Cave 2`); of a Pokémon, a portrait, an item, an attack, a status, a weather, an effect, **Watch It Play** plays it on the beach (an attack: Pikachu uses it on a Krabby); each line is checked by a test of `resources/tests.rs`;
  - of an attack, **Its Effects**: each of them side by side, playing, with what it is and its number (a click copies `effect pikachu <number>`);
  - **Used in** n films and m decors, and which (a click opens it in its space);
  - ★ Favourite (kept in `target/workshop/resource_favourites.txt`), **Paint** (the picture opened in the drawing program), **Duplicate**, **In the Folder**, **Copy the Word**.

### The Dungeon space (`workshop/src/dungeons.rs`)

Between Decor and Resources, the **Dungeon** space gathers the dungeon editor and the boss fight editor. **All Dungeons** lists, side by side, every dungeon (its floors, its look, how many Pokémon, its boss) and every boss fight (its boss, its level, its guards, its phases); a click on one opens it, **+ New Dungeon** and **+ New Boss Fight** make one. Each one open is a tab in the tools bar (• not saved yet; × or a middle click closes it; Ctrl+Tab the next); they stay open while the list or another space is shown.

### Making a dungeon (`workshop/src/dungeon/`, its layout: `dungeon/layout.rs`)

In the Library, the **Dungeons** group has two shelves: **Dungeons** (each `.dungeon`, with its floors, its look, its boss, those met there; the portrait of its boss or of its first Pokémon) and **Boss Fights** (each fight written apart, by its boss: its level, its title, its guards, its phases). In the Inspector, **Edit** and **Edit the Fight** open it in the **Dungeon** space; "In a film" gives the line `dungeon <word>`.

In the **Dungeon** space, **+ New Dungeon** or a dungeon of the list (or, from the Resources, **+ Create › A Dungeon…**) opens the editor over the whole window, in three columns. A film uses it with `dungeon <word>` after a `player` (see "A dungeon of your own" in `film/FILMS.md`).
- **On the left, what it is**: its **name**; its **floors** (how many, counted going up or not, whether one may recruit there, the level the team enters at); its **look** (the decor and the shape of the floors, taken from a game dungeon and one of its floors); its **music**, to listen to (▶); the **boss fight at the end**: a fight written apart (or none), **Edit This Fight** opens it in its editor, **+ New Fight** makes one, and under it its boss, its level, its HP and whether it looks easy or hard; a dungeon that keeps its fight in its own file offers **Make It a Fight of Its Own**.
- **In the middle, its floors as a grid**: a row for each Pokémon met there (its portrait, its level, "often") and for each item on the ground, a column for each floor, and the boss's at the end; **a click on a tile** adds that floor (or removes it, at the edge of those it has), **a right click**: all the floors; hovering over a floor's number, who is met there and in what share; a floor where no one is met is said in red. "+ a Pokémon…", "+ an item…" search among those of the game.
- **On the right, the trial**: **the trial team** (who leads and who follows, their portrait, their level, unless the dungeon says its own; any Pokémon the game starts with or its dungeons meet; kept from one dungeon to another in `target/workshop/trial_team.txt`), then **▶ Try** plays it there, as written: the keys go to the dungeon while the mouse is over it (arrows, Y or Shift held to turn in place, Space, 1 to 4, Enter, Esc or Backspace for the menu, R); under it, **what is wrong** (read again at each change) and **its file**. **Save** (Ctrl+S) writes it in `library/dungeons/<word>.dungeon`.

### Making a boss fight (the same files)

In the **Dungeon** space, **+ New Boss Fight** or a fight of the list (or **Edit the Fight** in the Library) opens its editor, apart from the dungeons', over the whole window in three columns; it writes `library/fights/<word>.fight`. While playing, the boss's name and its HP gauge are at the bottom of the screen.
- **On the left, the boss**: its portrait, its species, its level, its HP times so much, whether it asks to join the team once beaten; **its strength and the team's**: level, HP, Attack, Defense, Special Attack and Defense of the boss, of its guards and of the trial team, as the game computes them, with the reinforcements of its phases, and **at a guess**: easy, balanced, hard or very hard (when hovered, the sum: the time the team needs to bring them down against the time they need to bring it down, without items, moves or types); **its guards** (six at most, their portrait, their level, their tile); **the team that fights it** when it is played alone (the trial, `dungeon <word>` in a film; at the end of a dungeon, it is the one that came there): who leads, who follows, at what level (none: the one that comes); **its allies** (six at most) who fight on the team's side, near it or on the tile where they are put (turquoise in the arena), counted in its strength and in "at a guess"; **its moves** (four at most; none: those of its species); **what it drops**, beaten.
- **In the middle, the arena or the trial**:
  - **The arena**, large: its width, its height, its starting shape (a room, four pillars, water on the sides, a cross); the team (green), the boss (red), its guards (orange), the items laid (blue), the traps (purple, pale if hidden). **Everything is laid there with the mouse**: the tool (the team, the boss, each guard, each ally, an item of the game, a trap and whether it is hidden, the eraser) then a click on a tile; right click: take away what is there; when hovered, the tile and what is on it; the list of the items and traps laid (×: take it away); "put each back in its first place". **Paint** the ground, a wall or water, tile by tile by clicking or dragging (writes `boss_map:`); "erase the paint" goes back to the starting shape.
  - **The trial**: **▶ Try the Fight** plays the arena at once with the trial team (chosen above); "the boss at … % of its HP" starts it already hurt, to see its phases at once.
- **On the right, its staging**: its **name** and **its arena's look** (a game dungeon and one of its floors; none: that of the dungeon that names it); **its entrance**: its title (under its name, in the banner), its cry (the screen shakes), what it says at the start and as it falls; **its music**, to listen to (▶); **its phases** on a band of its HP (full on the left, each phase marked where it comes, orange from its first anger on), then a card for each phase: under what share of its HP, what it says, its anger, its heal, the weather, its cry, the shake, its reinforcements (six at most), new moves; **+ a phase**; then **what is wrong** and **its file**.

### Making an image, an animation, a sound (`workshop/src/making/`)

In place of the grid, until **Close**. Thought out like the best tools of their kind: Aseprite for pixel art, sfxr for sound effects.

**Making, over the whole window**: what is being made takes the whole window (no Library nor Inspector), each making its **tab** in the top bar: Image, Animation, Sound or Music, its word, `•` while it is not saved. A click shows one, **Ctrl+Tab** the next (Ctrl+Shift+Tab the one before), **×** or the middle click closes it (not saved: a second click, "close without saving?"), **+ New** opens another. **◀ Library** goes back to the resources, the makings stay open (**▶ Resume**), also when going through the other spaces. The canvas fits itself to the room it has, until a zoom by hand.

**An image, an animation** (+ Create, or **Paint Here** on a drawn object), in two ways (at the top; the last one chosen is kept):

**Easy** — for someone who knows nothing about it, in three steps:
1. **Start**: an empty image (16, 32 or 64 pixels), or a **shape** (Round, Box, Star, Drop) in the colour chosen, its outline around it.
2. **Draw**: Pencil (size 1, 2, 4), Eraser, Fill, Picker; **Mirror** (a face, a vase from the front); the game's colours large; **Outline** (a dark line, like the game's objects), **Relief** (light at the top left, shadow at the bottom right), Clear All; Undo, Redo. The image for real, ×1 ×2 ×3.
3. **Animate for Me** (from the first picture): **Bounce**, **Blink**, **Shake**, **Grow**, **Spin**; Slow–Fast; ▶ Play; + Copy the picture to make one by hand. Then **name it and save it**.

**Expert** — everything, without limits:
- **Tools** (their letter): Pencil B (size 1 to 16, `[` `]`; **pixel perfect**: the L corners of the strokes removed), Eraser E, Bucket G (**everywhere**: all of that colour), Picker I (or Alt+click), Line L, Rectangle R and Ellipse O (**filled** or not; Shift: square, circle, 45°), Selection M, Move V, Shade S (click: darker; right click: lighter), Dither D (a checkerboard of the two colours); **Mirror** ↔ and ↕.
- **Two colours**: the main one (click) and the second one (right click), X swaps them; their colour picker, their code; the **latest** used, a **palette** of one's own (+ −; Game: the game's), those **in the image**.
- **Selection**: painting stays inside; Ctrl+C, Ctrl+X, Ctrl+V (an image dropped on the window too); Move lifts it and moves it (arrows: by a pixel, Shift: by 8; Enter lays it, Esc forgets it); Ctrl+A all, Ctrl+D none, Delete empties it.
- **Image** (menu): flip ↔ ↕, turn a quarter, **outline** in the main colour (like the game's sprites), replace the main colour with the second one.
- **The canvas**: the wheel zooms around the pointer, the middle button or Space moves it, Fit; the grid; the **onion skin** (the pictures before in red, those after in green); the brush shown where it would paint; the point and the size of the selection.
- **Layers**: shown or not, opacity, up, down, duplicate, **merge down**, remove; double-click: rename.
- **Frames** (the strip under the canvas): + empty, Duplicate, Remove, ◀ ▶; each its time (sixtieths), All: the same; ▶ Play on the canvas, in a **loop** or **ping-pong**, ×0.5 ×1 ×2; ← → another, ↑ ↓ another layer.
- **Without limits**: the canvas from 1 to 1024 pixels; each layer its **blend** (Normal, Multiply, Add, Screen); **Adjust the Colours** (hue, saturation, light, contrast; this picture or all those of the layer); **Rotate, Scale** by any angle and scale (the selection, else the picture; pixels kept whole); **Reduce to N Colours** (the nearest of the most used); **Take the Image's Palette**; the **onion skin** adjustable (±1 to ±4 pictures, its strength); the **tiling** (the picture repeated around it, for a pattern that joins up).
- **Real size** ×1 and ×2; Undo, Redo (Ctrl+Z, Ctrl+Y), each change by its name.
- **Save** (Ctrl+S): the layers merged, in `library/images/name.png` (`name.4.png`: four pictures side by side); the layers and the times kept in `target/workshop/paintings/name/`, to paint again; **In a film**: `put object name front_of pikachu`, `object name image name.4.png 4 rhythm 8,8,8,8`.

**A music** (+ Create → A Song, or + New; "Open a Song": those already made): a whole music studio, which starts with a four-track piece (drums; piano in the reverb; bass ducked by the kick; melody in the echo) in a loop.
- **Transport**: ▶ Play / Stop (Space; the piece is computed apart, track by track at the same time, then heard; **changed while playing, it is heard again from where it is**), Start (Origin), **Rec.** (what is played on the keyboard while playing is written in the chosen track), **Mic** (records the microphone from the playhead, the piece played at the same time; the take becomes a sound of the project, laid on an audio track), **Loop** (Shift + drag on the ruler to place it), **Metronome**, the bar and the beat, the **tempo**, the beats per bar, the length, the **swing**; Undo, Redo (Ctrl+Z, Ctrl+Y).
- **Tracks**: each its name, its colour, **M**ute, **S**olo, its volume, its pan, its level; **+ Instr.** (Electric Piano, Bass, Synth Bass, Square Lead, Supersaw, Pad, Strings, Organ, Bells, Plucked String, Flute, Chiptune), **+ Drums** (kick, snare, clap, closed and open hi-hats, toms, cymbal, cowbell, rim: all synthesized), **+ Audio** (sounds as they are), × removes it.
- **Arrangement**: the bars, **regions** in each track; a double-click makes one, dragging moves it (to the beat; Shift: to the sixteenth), its right edge lengthens it, **Alt** + drag makes a **linked copy** of it (a pattern: changing one changes the others; "Make Unique" unlinks it), right click (or Delete) removes it, Ctrl+D duplicates it (linked); an audio region shows the shape of its sound; the playhead (a click on the ruler), which follows playback; the wheel: the tracks, Shift: along the piece, Ctrl: closer.
- **Notes** (the region chosen): a **piano roll** (the names of the notes, C3 to C6; a click lays a note, dragging moves it, its edge lengthens it, right click removes it; the note chosen: start, length, semitone, velocity), the **scale** lit (major, minor, harmonic, pentatonics, blues, dorian, mixolydian; its tonic) and **In the Scale**, **As Chords** (major, minor, 7th, major 7, minor 7, sus4, sus2, diminished, augmented), an **arpeggiator** (up, down, up-down, random; 1/4 to 1/32), **Strum** (the notes of a chord shifted like a guitar), **Humanize**, **Legato**, transpose (±1, ±12), Quantize, and the **velocities** of each note as bars, dragged; for the drums, a **step sequencer** (a row per drum, a cell per sixteenth; Shift: softer, dragging lays them in a row); for an audio region, its **sound** (which, from where, its volume, its pitch, its fades, reversed; the region as long as it); Linked Copy, ▶ Listen to the Region.
- **Instrument**: the whole **Workshop sound** (Easy or Expert: its layers, its waves, its modulation, its effects) for the track's instrument, its pitch being middle C; Change Instrument; for the drums, each drum (Original Kit).
- **Mixing**: a strip per track (volume, pan, M, S, its level; a low-pass **filter** and its resonance; its **sends** to two shared buses; a **sidechain**: ducked when another track hits, by so much) and its **effects** (lows, mids, highs, compression, echo and ping-pong, room, width, limiter); **bus A** (a reverb), **bus B** (an echo), adjustable; the **master** and its volume.
- **Automation**: drawn along the piece, for each track, its **volume**, its **pan**, its **filter**, its **sends** (a click: a point, dragging moves it, right click removes it; the value when hovered).
- **The keyboard plays**: A W S E D F T G Y H U J K O L P ; (C, C#, D…: two rows like a piano, on a QWERTY keyboard), Z and X: an octave lower, higher; for the drums, each key a drum.
- **Import**: a **MIDI** file (`.mid`, from another program: each track becomes a track, the drums on their drums) or a **sound** (`.wav`, kept among the project's and laid on an audio track) dropped on the window; or from Import: the `.mid` files of `library/music/`, the project's sounds.
- **Export**: the piece as heard in `library/sounds/<word>.wav` (**in a film: `music <word>`**), and the piece itself in `library/music/<word>.music`, to take up again; **More…**: the **separate tracks** (each its `.wav`), the **MIDI** (`<word>.mid`, for another program), the piece alone.

**A sound** (+ Create, or **Edit** on a sound of one's own), in two ways (at the top; the last one chosen is kept):

**Easy** — for someone who knows nothing about it, in three steps:
1. **Choose a kind of sound**, by what it is for: *Game* (Coin, Jump, Hit, Explosion, Laser, Power-up, Step), *Menus* (Beep, Confirm, Error, Victory, Alarm), *Nature* (Splash, Wind, Bell, Cry), *Magic* (Magic, Sparkle, Teleport). It is heard at once; again: another of the same kind.
2. **Listen, adjust** with words: Low–High, Short–Long, Soft–Hard, Clean–Damaged, Falls–Rises, Dry–Distant, Quiet–Loud (a double-click: in the middle); **Another Like This**, **A Little Different**, **Surprise!**, **Undo**.
3. **Name it and save it**; it is written `sound name` in a film.

**Expert** — everything, without limits (the same sound, with all its settings; each value can also be typed beyond the bounds of its slider, with a double-click):
- **Layers** (up to 8, keys 1 to 8): each its sound, its **start**, its **gain**, its **pan** (left, right); **mute**, **solo**; +, Duplicate, Remove.
- **Make** (the layer chosen): Coin, Laser, Explosion, Power-up, Hit, Jump, Beep, Magic, Step (random within its kind); **Random**; **Mutate** (the same, a little changed). A **locked** group is not touched.
- **Wave**: square (and its **duty cycle**, which slides), saw, sine, triangle, noise, **pink noise**, **8-bit noise** (like the NES), **drawn**, **string** (a plucked string, Karplus-Strong: its damping, its brightness), **sample** (one of your sounds read at any pitch, from a start, looping or not; "Original Pitch"); **harmonics** on top.
- **Envelope**: attack, sustain, **punch**, decay — or its **handles** dragged on the wave. **Pitch**: the note, its **floor**, its **slide** and its **acceleration**. **Vibrato**, **arpeggio**. **Repeat**. **Filters**: low-pass (and its **resonance**), high-pass, which slide. **Flanger**, **grain**, volume.
- **Modulation and notes**: **FM** (ratio, depth: bells, metal, strange voices), **AM** (speed, depth), **drive** (a warm distortion); **notes** one after another (up to 8, in semitones, each its time: fanfares, jingles); a **wave drawn** by hand (32 steps; Sine, Square, Saw to start with, Smooth).
- **Sound text**: each setting of each layer written out, changed by hand then **Apply**.
- **More…**: the kinds of the Easy mode (some of several layers), for the whole sound.
- **Unison and colour**: up to 16 copies played together, **detuned** (cents) and **spread** from left to right (supersaws, choirs, pads); **fold** (the wave folded onto itself); **ring** (multiplied by a wave: robots, false bells); **chorus** and **phaser** (how much, how fast).
- **Filter: kind and vowel**: low-pass, band-pass, notch, **peak** (its gain), high-pass, all resonant; a **vowel** (a, e, i, o, u, and between two: three formants) that makes the sound speak.
- **Free modulation**: two **LFOs** (sine, triangle, square, saw, random; up to 200 Hz), a **second envelope** (attack, decay, level), a **curve** drawn along the sound, a **random** value drawn at each sound; **six routes**: each source to the pitch, the cutoff, the volume, the pan, the FM, the duty cycle, the resonance, the drive, the vowel, the ring or the fold, by an amount (negative: reversed). A wobble: LFO 1 → Cutoff; a voice that says "wah-wah": LFO → Vowel.
- **Routing**: a layer **modulated by another**, by FM (timbres of several operators) or by ring.
- **Score** (a piano roll): as many notes as wanted, each its start, its length, its semitone, its velocity; a click lays one, dragging moves it, its right edge lengthens it, a right click removes it; the **tempo**; Simple Notes, Octave ±, Duplicate, Clear. Each note plays the whole layer (unison, routes, effects included): melodies, chords, basses.
- **Mix** (all the layers): **echo** (time, feedback, share; **ping-pong**, from one side to the other), **room** (a reverb: size, share), **speed** (faster and higher), **reversed**, **fades** in and out, **lows, mids, highs** (an equalizer), **compression**, **width**, **limiter** (never beyond -1 dB), **normalize** (as loud as it can, at -1 dB).
- **Synth** (in More… and in Easy mode): Supersaw, String, Robot Voice, Wobble, Melody (two layers in a score), Pad.
- **See**: the mix's wave (left at the top, right at the bottom), the layer's envelope and the **playhead**; the layer's **pitch** over time; the mix's **spectrum**.
- **Meters**: length, **peak** and **average** (dB), a VU meter; **clips** says when to lower or normalize.
- **Compare**: a sound of the game, heard beside it.
- **Variants**: the sound and N slightly changed variants written at once (`word_1`, `word_2`…: steps, hits never twice the same).
- ▶ Listen (Space), **in a loop**, **play while changing** (as soon as a setting is let go); **Versions** (a click takes one back), Undo, Redo (Ctrl+Z, Ctrl+Y).
- **Save** (Ctrl+S): `library/sounds/name.wav`, in stereo (its layers and settings kept in `target/workshop/sounds/`); **In a film**: `sound name`.
- The sounds of one's own are on the **Sounds** shelf, first.

### Writing the lines
- **Dialogue mode** (the **Dialogue** button, Ctrl+Shift+D): Enter at the end of a line said begins the other's; Tab on a line begun goes to someone else of the scene.
- **The faces** are offered as portraits of the one speaking; a click takes one.
- **Typography** (View › Typography): what is said is read again as typography wants (two spaces, a word twice, `..`, straight quotes, spaces around commas, the capital; View › "Spaces Before ! ? : ;" for the French way). It is underlined in blue, with a fix for each, and all can be fixed at once.
- **The reading time** of the cursor's line is under its box; the Layout gives the length of each scene.

### The film seen as a story (**Text ▾** at the top)
- **Screenplay**: the film to read, each line said with the portrait of its face, what is done in italics, each scene a heading with its length; it is also written as an HTML page to print (in `target/workshop/screenplays/`).
- **Board**: a card per scene (its place, who is there, its summary, how much is said there, its length); dropped on another, the scene moves in the text.
- **Characters**: who speaks in which scene and how much; each one's sheet is kept in `library/characters/` and shown when hovering over their name in the text.
- **Graph**: the scenes in order, their moments, their choices, the passages from one place to another.

Everywhere, a click goes back to the line in the text.

### Watching a playable film
- **F9** sets a breakpoint: a red dot in the margin and on the ruler. The film stops at that line as it plays. **F10** plays on to the next line (see "Step by line").
- **VARIABLES**, in the Layout, gives each number of the film: where it is declared, given, read (a click goes there), and what it is at the moment the Scene shows.
  - The numbers declared (`let`) can be changed: **▶ Play from Here with These Values** plays again from the cursor without touching the text.
  - **The Text's Values** plays the film again as it is written.

### The project
- **Search All the Films** (Ctrl+Shift+F, or **Project ▾**): the films and the recipes, whole word or not, matching case or not. A click opens the film at the line. **Replace Everywhere** is asked twice, and renames someone in the whole project.
- **Open a Film or a Recipe** by a few letters of its name (Ctrl+P).
- **The films open** are tabs above the text (×: out of the list).
- **Another Film Beside** (Project ▾), read in colours. A double-click on one of its lines opens it there.
- **Versions** (Project ▾):
  - the film is kept while it is written (a version every two minutes at most, sixty in all, in `target/workshop/versions/`);
  - the last commit can be taken too;
  - each version is compared line by line (red: removed, green: added) and taken back (Ctrl+Z undoes it).
- **The notes in the margin** (Ctrl+Shift+N) are not part of the film. They are yellow squares (green once done), kept in `library/notes/`. The Layout lists them under **NOTES AND TO DO**, with the comments `# TODO`.

### Listening, adjusting, focusing
- **▶ at the end of a line `sound x` or `music x`** plays that sound or that music: the game's, or ours from `library/sounds/`. ■ stops it.
- **Alt + dragging a number** raises it to the right and lowers it to the left, by steps of 1, or of 0.1 for `1.5s`. The Scene follows. Alt+click without dragging still adds a cursor.
- **Focus** (F11, or View › Focus) shows the text alone over the whole window, in a column, the line being written in the middle. F11 or Esc to go back.

| Keys | |
|---|---|
| Ctrl+Shift+D | dialogue mode |
| F9, F10, F6 | a breakpoint; on to the next line; step by line or not |
| Ctrl+Shift+F | search all the films |
| Ctrl+P | open a film by its name |
| Ctrl+Shift+N | a note on the line |
| Alt + drag | change a number |
| F11 | focus |

## Emulator

A DS game run inside the Workshop, to see a film as the real game plays it (`workshop/src/emulate.rs`, the `emulator` crate). It needs a **libretro core**, the emulator itself as a small library, which the Workshop never comes with: get the melonDS core (or DeSmuME's) from libretro's site (buildbot.libretro.com, nightly, your system, latest: `melonds_libretro`), unzip it anywhere of yours, then **Choose a Core…** in the space (or set `WORKSHOP_LIBRETRO_CORE`). melonDS needs no BIOS or firmware: it uses its own FreeBIOS. See `docs/DS.md`.

- **Load ROM…**: a `.nds` of yours. Both screens are drawn one above the other, crisp, as big as fits; the sound plays at 1×.
- **Keys**: arrows the D-pad, X the A button, Z the B button, S the X button, A the Y button, Q/W L/R, Enter Start, Shift Select; the mouse held on the bottom screen is the stylus.
- **Pause/Play**, **Reset**, **Step** (one frame, paused), **1× 2× 4×**, **Save State** / **Load State** (one slot a game), **Screenshot** (a PNG): the states and pictures go in the settings folder's `emulator/`.
- **Play This Film on DS**: the film open in the Film space exported (`scene_player export-ds --partial`) and put in a copy of your Explorers of Sky (`tools/ds/patch_rom.py --boot --more-actors`, Python with `skytemple_files`) as `target/ds/workshop_play.nds`, then played from power on, A pressed by itself through the title until the film starts (about 25 s). The ROM and armips are asked the first time and remembered; what the steps say, and why one failed, is shown under the screens.

The Workshop's **settings** (`settings.rs`) are a small `key = value` file of the user's own: `%APPDATA%\dungeon_workshop\settings.txt` (`~/.config/dungeon_workshop/` elsewhere, or `WORKSHOP_SETTINGS`), holding the core (`libretro_core`), the ROM (`ds_rom`) and armips (`armips`); the emulator's saves, states and screenshots sit beside it in `emulator/`. Nothing of them goes in the project.

## The code

`workshop/src/`, a module a subject; a big one is a folder, its `mod.rs` holding its state and what its files share. Under the building-block rule (`tools/rule.sh`): it uses `assets`, `decor`, `emulator`, `rules`, `dive`, `game`, `scenes`, `film`, `preview`, `engine`, `shell` and `stage_view`, by their public side.

| Module | What it holds |
|---|---|
| `main.rs`, `app.rs` | the window: its menus, the spaces, their docked panels, the bar of state |
| `theme.rs`, `icons.rs` | the look: colours, sizes, icons drawn with lines |
| `home.rs`, `first.rs`, `project.rs`, `safe.rs`, `pick.rs` | the Home, the first launch (the resources extracted), the project's files, safe saves, the system's dialogs |
| `games.rs`, `game.rs` | the Game space, and the `.game` file it reads and writes |
| `script/` | the Film's Script: `mod.rs` (the film open, its state, the language's thread), `body.rs` (the text drawn and typed in, part after part), `head.rs` (the tabs, the head, the story view), `keys.rs` (the keys taken before the editor), `margin.rs` (the margin, the ends of lines, the ruler), `offers.rs` (what is offered as one types), `popups.rs` (the small windows over the text), `problems.rs` (the problems, the line's box, the status), `highlight.rs` (each piece in its colour), `completion.rs` (what may be typed), `knowledge.rs` (what is known of a word, portraits), `cast.rs` (who is in a scene), `text.rs` and `edits.rs` (where things are, the text changed), `plan.rs` (the film's plan), `wiki.rs` (the Wiki beside the text), `windows.rs` (the project's windows), `editor.rs` (laid out), `heard.rs` (sounds named) |
| `films/` | the Film played: `mod.rs` (its state), `playing.rs` (its thread), `scene/` (the Scene: `mod.rs` the order of its parts and the view, `marks.rs` what is drawn over the picture, `mouse.rs` what the mouse does on it, `bar.rs` the line played, the bar, the keys, `script.rs` the text read and written again), `timeline.rs` (its axis), `bands.rs` (the timeline's bands), `inspector.rs`, `state.rs` (the panels), `lines.rs` (lines written again) |
| `story.rs`, `catalogue.rs`, `example.rs`, `typography.rs`, `workspace.rs`, `debug.rs` | beside the Script: the film as a story, the words by family, a line played as an example, what is well written, the whole project's films, a film looked into |
| `wiki/` | the Wiki's pages, a word each: `mod.rs` (a page made of a word and what is written of it, the words it goes with, a recipe's source), `written.rs` (the pages written by hand, `workshop/wiki/*.txt`: what a word does, its example), `made.rs` (a form as an author reads it, its parts, an example made from it), `search.rs` (the pages found, the best first) |
| `decors.rs`, `scene/` (the Scene: `mod.rs` the view, `drawn.rs`, `marks.rs`, `moving.rs`, `choosing.rs`, `hints.rs` the hint bar), `edit.rs`, `placing.rs`, `terrains.rs`, `library/` (`mod.rs` its parts, `search.rs` what is shown and found, `cards.rs`, `backdrops.rs`), `walk.rs`, `workshop.rs` | the Decor space: the decor open, its Scene, its text changed by what is done there, where things land (snap, stamp, nudge, layers), the brushes, the Library, a Pokémon walked, the decors' thread |
| `panels/` | the Decor's panels: `hierarchy.rs` (the Elements' tree), `layers.rs` (the Hierarchy's layers), `history.rs`, `inspector.rs`, `piece.rs` (a piece), `resident.rs` (someone who lives there), `landmark.rs`, `fields.rs` |
| `dungeons.rs` | the Dungeon space: all the dungeons and fights |
| `dungeon/` | a dungeon or a boss fight made: `mod.rs` (its forms and state), `form.rs` (read, written), `lists.rs`, `state.rs`, `editor.rs`, `layout.rs` (the three columns, the fight in numbers), `fight_columns.rs` (a boss fight's), `dungeon_columns.rs` (a dungeon's, its floors' grid), `arena.rs`, `choosers.rs`, `check.rs`, `floor.rs` (a floor as the game makes it) |
| `resources/` | the Resources space: `mod.rs` (shelves, their groups, things, state), `shelves.rs`, `game_shelves.rs` (the game's attacks, statuses, weathers, effects, portraits, items, backdrops, tilesets), `pictures.rs`, `tree.rs` (the Library), `grid.rs` (a shelf's cards), `toolbar.rs` (the tools, the head, the filters over them), `inspector/` (the Inspector: `mod.rs` the order of its parts, `large.rs` the one chosen in large, `about.rs` what it is, how a film writes it, where it is used, `actions.rs` what to do with it, `pokemon.rs` a Pokémon's own actions), `studio.rs` (what is being made, in tabs), `dropped.rs` |
| `making/` | what is made in the Resources: `mod.rs` (`Maker`), `picture/` (the painting: `canvas.rs`, `pointer.rs` (what the pointer does with each tool), `panels.rs`, `editor.rs`, `simple.rs`, `strokes.rs`, `selection.rs`, `effects.rs`, `steps.rs`, `pixels.rs`, `files.rs`), `sound/` (the sound maker: `synth.rs`, `render.rs`, `voices.rs`, `mix.rs`, `presets.rs`, `instruments.rs`, `versions.rs`, `controls.rs`, `views.rs`, `editor.rs` (the expert's), `settings.rs` (a layer's settings, the mix), `simple.rs`) |
| `music/` | the music maker: `mod.rs` (its state), `project.rs` (the song rendered), `midi.rs`, `editor.rs`, `transport.rs`, `tracks.rs`, `notes.rs`, `mixer.rs`, `keys.rs`, `wave.rs` |
| `spritecollab.rs`, `quant_tool.rs`, `level_csv.rs`, `tool_run.rs` | the Tools: SpriteCollab, an image converted, a species exported as CSV, a long piece of work on its thread |
| `emulate.rs`, `settings.rs` | the Emulator space (a libretro core through the `emulator` crate, its sound through `shell::stream`, a film put on DS), the Workshop's own settings |
| `capture.rs` | a picture of the window without a mouse |
| `session.rs`, `launch.rs` | what every space works with (`Session`: the library, the data, what is read of them, `reload`), how the Workshop was launched (`LaunchOptions`) |

## Launch options

The Workshop's `main` reads, once, what it is launched with (`workshop/src/launch.rs`, `LaunchOptions`) and hands it to every space through the session (`Session::launch`); nothing else in the Workshop reads an environment variable.

| Option | Effect |
|---|---|
| `workshop <file>` | that film or decor opened in its space |
| `DUNGEON_DATA=<folder>` | the game's files read from there (else the root's `data/`) |
| `DUNGEON_SPRITECOLLAB=<folder>` | the clone of SpriteCollab the Tools read (else `<root>/sources/spritecollab`) |
| `DUNGEON_FILM_LANGUAGE=fr` | the films said in that language (a line that does not give it: in English) |
| `WORKSHOP_SETTINGS=<file>` | the Workshop's settings there (else `dungeon_workshop/settings.txt` in the system's settings folder) |
| `WORKSHOP_LIBRETRO_CORE=<file>` | the libretro core the Emulator runs (before the settings' own) |
| `SKYTEMPLE_ARMIPS_EXEC=<armips>` | what Play This Film on DS assembles with (before the settings' own) |

The root (`library/`, `games/`, `film/recipes/`) is the first folder up from where it runs, else up from the program, that has `library/films`. What its spaces make (films, decors, dungeons, fights, pictures, sounds, music, tilesets) goes into `library/`; a new game into `games/<name>/`; its own settings and versions into `<root>/target/workshop/`.

### For a picture without a mouse

`WORKSHOP_CAPTURE=x.png` takes a picture of the window then closes it. The other variables set what is seen in it:

| Variable | Effect |
|---|---|
| `WORKSHOP_AFTER=seconds` | when the picture is taken |
| `WORKSHOP_SPACE=game\|film\|decor\|resources\|emulator` | the space shown |
| `WORKSHOP_EMULATOR_ROM=<file.nds>`, `WORKSHOP_EMULATOR_FRAMES=n` | that game loaded in the Emulator, run n frames at once (past its logos) |
| `WORKSHOP_EMULATOR_FILM=<file.film>` | that film put on DS at once (Play This Film on DS; the settings' ROM and armips) |
| `WORKSHOP_GAME=my_game` | that game open in the Game space |
| `WORKSHOP_VIEW="x y zoom"` | the view of the scene (in the Film: `Screenplay`, `Board`, `Characters`, `Graph`: the Script seen so) |
| `WORKSHOP_TOOL=terrain\|scatter\|walls\|walk` | the tool in hand |
| `WORKSHOP_LIBRARY=backdrops\|models\|search:<words>` | the Decor's Library on that shelf, or searching that |
| `WORKSHOP_CHOSEN=n` | line n chosen |
| `WORKSHOP_FOLDED="Transform,Resident"` | those components of the Inspector folded |
| `WORKSHOP_INSTANT=seconds` | the film at that moment |
| `WORKSHOP_SCENE=whole,names,grid` | the Film's Scene shown so |
| `WORKSHOP_STEP=1` | the Film stepping by line (Step on), playing from the start (or `WORKSHOP_INSTANT`): it stops at the next line |
| `WORKSHOP_LINE=n` | the Script's cursor on line n |
| `WORKSHOP_WORDS=family` | the words by family open on that one |
| `WORKSHOP_CREATE=image\|animation\|sound\|music\|dungeon\|dungeon:<word>` | the Resources making one (`dungeon:<word>`: that one open) |
| `WORKSHOP_TRY=1` | the open dungeon tried at once |
| `WORKSHOP_CREATE=fight:<word>` | that boss fight open in its editor (with `WORKSHOP_TRY=1`: tried) |
| `WORKSHOP_BOSS_HP=20` | the fight alone tried with the boss at 20 % of its HP |
| `WORKSHOP_FILE=my_film` | that file chosen in the Home |
| `WORKSHOP_PANELS=layout,wiki` | the Layout, the Wiki open beside the Script |
| `WORKSHOP_WIKI=walk\|walk:play\|joy:source\|search:rain\|all` | the Script's Wiki open on that page (its example played, its recipe's source shown), searching that, or on all the words (`all:ds`: DS only, in DS Mode); in a picture it shows over the window |
| `WORKSHOP_COMPLETE=1` | with `WORKSHOP_LINE=n`: the cursor at the end of that line, what may be typed there offered |
| `WORKSHOP_PROJECT=search:word\|beside:film\|versions\|open` | the Script with the search in the project, a film beside, the versions or "Open" |
| `WORKSHOP_FOCUS=1` | the Script in focus |
| `WORKSHOP_RESOURCE=drawing:torch` (`…:body`: a Pokémon's body) | the Resources shelf (`pokemon`, `variant`, `portrait`, `drawing`, `prop`, `item`, `song`, `sound`, `attack`, `status`, `weather`, `effect`, `backdrop`, `tileset`, `piece`, `dungeon`, `boss`) and the one chosen |
| `WORKSHOP_SPRITECOLLAB=25` | the SpriteCollab window open on that number |
| `WORKSHOP_SEED=n`, `WORKSHOP_DRAWN=1` | the dungeon's floor laid out with that seed, drawn |
| `WORKSHOP_MUSIC=1:instrument` | that track of the song, that view (`notes`, `instrument`, `mixer`, `automation`, `scale`) |
| `WORKSHOP_TILEQUANT=<file.png>` | the image converter open on that picture |
