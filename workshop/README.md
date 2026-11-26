# Dungeon Workshop

```
cargo run --offline -p workshop [file.film | file.decor]
```

One window, seven spaces (Home, Game, Film, Decor, Dungeon, Resources, Emulator), each with its panels docked as in a studio (the Game's columns and the Dungeon's editors take the whole window).

**Made for films and decors first.** The Workshop opens on the Home (after the first launch, with a welcome panel: **+ New Film**, **+ New Decor**, the last files opened, the first film's steps, [docs/FIRST_FILM.md](../docs/FIRST_FILM.md)). The game-making spaces (Game, Dungeon, Emulator) show only when **Settings › Show the Game-Making Spaces** is on (`game_spaces = on` in the settings file): off for someone new, on by default for whoever already used them (a game in `games/`, or any setting already written: the emulator's paths or keys). The space looked at always shows.

## The whole window

- **The menus work in every space**: **File › Save** and **Edit › Undo / Redo** (and their keys) go to the space looked at: the Film's Script (its undo is the text's own, the Scene's and the Timeline's edits included), the Decor, what is being made in the Resources, the Dungeon and the Game (Save). **File › Export Video…** (the Film space) makes the film open a video in the background: `scene_player video <film>` (beside the Workshop), a window with how far it is (seconds made of the film's length), then where it is written (`target/films/<film>/<film>.mp4`), **Play It**, **Open the Folder**. It needs **ffmpeg** in the PATH: without it, the window says so (how to install it) and nothing is run.
- **The command palette** (**Ctrl+Shift+P**, or View / Help › Command Palette…): every action of the menus, the spaces (Go to: Film…), the panels of the space looked at (View › Open / Close Panel: Timeline…), the last files (Open: …), each with its key; a few letters of its name find it (each word sought, the start of a word first; what can't be done now greyed, last); ↑ ↓, Enter or a click does it, Esc closes (`workshop/src/commands.rs`).
- **Help › Keyboard Shortcuts…**: every key of each space, searched, read from this file's tables of keys (a table whose head starts with "Keys", under its heading: a key added here shows there, `workshop/src/shortcuts.rs`).
- **The bar of state** says the file open from the project's folder (`library/films/the_juggler.film`), • if it is not written yet.

| Keys | |
|---|---|
| Ctrl+Shift+P | the command palette |
| Ctrl+S | save what is open in the space looked at |
| Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z | undo, redo in the space looked at |

## Game

A game of one's own (a folder of its own, `games/<name>/<name>.game`, see `games/README.md`; `workshop_game/`), in three columns (**+ New Game** makes its folder, its `.game` and its `films/`, `decors/`, `dungeons/`… folders; a dungeon of the game's own opens in the Dungeon space only if the library has one of that name):
- **On the left**: the project's games (a click opens one; a title and **+ New Game** makes one); its **card**: title, author, team (hero, partner), home (a place and its entrance: where the team comes back to, K.O.), each written into the file as soon as it is let go, its comments and its order kept; under it, **the file** itself, to write the chapters (Save, Ctrl+S; • not written yet).
- **In the middle**: the **chapters**, a box each: what it plays (`film`), the places it goes to (`place`), the dungeons and fights it sends the team to, what it opens on the map; a click opens it in its space (in red: not found in the project); its rules (`on talk kecleon if …`); **→ chapter n** (the chapter it leads to), **■ the end**, or, in red, a chapter that leads nowhere. A click on a chapter's title chooses it to play from and shows its line in the file.
- **On the right**: **▶ Play from Chapter n** (`scene_player play <game>.game --chapter n`, a new game from that chapter at its home, in a window of its own: the file is saved first); **Export the Game…** (a `.dwpack` where one says, `dist/` at first: the game's own files, nothing of the cartridge; it says where it went or why it was refused); **Check the Game**: what `scene_player check-game` finds (names not found, flags set and never read, what the game never reaches), read again at each save; a click shows where (a line of the file, a film at its line, a decor with its line chosen, a dungeon in its editor).

The places' life (who lives there, what they say and when, what talking to them opens, their ways out, their zones, their `on … end`) is written in the **Decor** space: `decor/DECORS.md`.

## Film › Script

The film's text in an editor, like VS Code.
- **The colours**: who speaks, their face, what they say, the words of the language, the names, the times and numbers, the comments.
- **The margin** numbers the lines and marks those the language says something about (red: error, yellow: remark); those words are underlined.
- **Problems**: what the language says about the film (`film::service::diagnostics`), read again a moment after each change on a thread of its own. A panel of its own, beside the Timeline (its tab says how many, an error in red; **View › Add Panel › Problems**), so the Script keeps its height; closed, they go back under the text, folded to their head while nothing is wrong (a click on the head opens or closes them). Those said alike are **one line**, with how many (`9 × …`) and their lines (each a click to go there); the errors first; a click on one goes to its first line; a fix of one, or of all of a kind at once; **Fix All**; **remarks** lists them or not.
- **Typing** offers the words possible there (↑ ↓, Enter or Tab, Esc; Ctrl+Space to ask for them).
- **Hovering over a word** says what it is, and what is wrong there; a word of the language, in a line, what it does, and **F1** for its page in the Wiki.
- **At the top**: where the cursor is ("Scene 2 · the place as the game names it › moment › line › its word", each piece a click to go there), then a few: **Find**, the story view (**Text ▾**), **View** (word wrap, hints, letter size), **…** (the project's films: search, open, beside, versions, a note; **Format**, **DS Mode**, **Dialogue**, the **Layout**), **Wiki**.
- **A film opened on a line** (from the Home, the Timeline, a problem, `WORKSHOP_LINE`) brings that line into sight, in the middle.
- **DS Mode** (in the Script's head) adds the line `for ds` to the film, or takes it off: the film is written for Explorers of Sky on DS (docs/DS.md). Then what the DS can't play is told as the film is written (`E-DS-WORD`, `R-DS-LOSS`, `E-DS-ACTORS`, `E-DS-PLACE`, `E-DS-HERO`, `R-DS-ZOOM`, film/FILMS.md), the completion offers only the words that export (those with a loss marked "DS with loss: …"), and the Wiki shows each word's DS badge (**DS direct**, **DS with loss**, **not on DS**, what is lost or why on hover and on its page) with **DS only** to list the words that export. The statuses are those of `scene_player export-ds --table` (`film_core/src/ds.rs`).
- **The Layout** (on the left): the scenes, their place, how many lines are wrong in them (⚠), their moments; a click goes there; Fold, Unfold all.
- **The Wiki** (docked in the Film space, beside the Assets; opened there by **Wiki** in the Script's head or **F1**, docked again if its tab was closed; elsewhere a window of its own): every word of the language (its words, the library's recipes, the engine's commands), searched by its name, its family or what it does (the very word first, then those it begins, then its family, then what they do), by family otherwise. A page for each: what it does, said for an author; how it is written, each form with what each part means, what may be left out and what it is then; an example, a little film (written by hand in `workshop_wiki/wiki/*.txt`, or made from the word's form), coloured as the Script colours it, the word's line lit: **Play / Stop the Example** (on the right, in a loop), **Insert at Cursor** (its line under the cursor's, the scene's own two in place of Pikachu and Piplup), **Copy Example**; **See Also**, the words it goes with; a recipe's **How it's made**, its own lines. A word a film does not take as written (the game's now, an engine command a film's words write) says what to write instead. **F1** opens the page of the word at the cursor (or pointed at). A page's example plays at once, large, on the right of its text, the whole scene seen; Insert at Cursor and F1 go to the film being written.
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
  - With **Follow the Script**, the film goes to the line where the cursor is, shown as the **staging view**: the stage as it stands at that line with the screen's effects off (fades, the title, a curtain, an iris, white or black screens), so the `put` lines of a scene's start show the place, not black (**Staging** above the picture, and a tag on it; a click on **Staging** shows the effects). Playing, or going elsewhere with the bar or the Timeline, shows them again.
  - A film opens on its first frame seen (no fade or title over it), unless the Script's cursor is on a later line.
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
  - with **Names** (and in **Whole Place**), the spots of the film (`spot name x y`) marked with a diamond: **dragging one** writes its `spot` line again;
  - **handles**, each drag one change to undo, only the words it must written again (the line's spacing kept); the line they write from is the Script's cursor's in the staging view, else the one playing:
    - **the camera's frame** (Whole Place, its corners marked): dragging it moves the view, a corner grows or shrinks it (the zoom). The moment's last framing line up to the line (`frame …`, `glide [to] …`) gets the point (`frame` keeps `snap` and `max:`, drops `follows`), else `frame x y snap` is added after the moment's (or scene's) first line; a zoom goes on the moment's last `zoom` line, else `zoom n 1f` after the framing line;
    - **the way the one chosen faces** (a click on them): eight dots around their feet, the way they face lit; a click on a dot (or a drag from one, toward the way) writes it: their last line of the scene up to the line, its `facing …` or `look …` written again, `facing way` added to a `put`, else `who look way` after the line;
    - **their walks** in the moment (`who walk|run [to] x y [via …]`): a dotted line from where they stood (blue walking, orange running), each point a dot: **dragging one** writes that point of its line again;
  - the names keep clear of each other: one that would cover another goes up (or down), a thin line to where it belongs.
- **The Assets** (on the right, over the Inspector and the State): what a film is made with, to drag onto the Scene or the Script, or to double-click (written at the Script's cursor). Each is one change to undo.
  - **Tabs**: **Pokémon** (the film's cast first, then the Resources' one catalog: every species with its forms under it, sought by name, number, type or generation as on the Pokémon shelf; SpriteCollab's not imported yet badged, imported in the background when dragged or written), **Faces** and **Gestures** of the Pokémon chosen (a click on its card, or someone chosen on the Scene: followed), **Music** (its title, its word, how long once through, ↻ if it loops), **Sounds**, **Things** (items, held; the game's objects, put down; the drawn ones, held), **Places** (your decors, the game's places), **Effects** (the moves, by their effects). A search over the tab.
  - **On the hover**: a song is heard after a moment and hushed when the pointer leaves (a click keeps it on, **■ Stop** hushes it); a sound is heard; a gesture or an effect plays.
  - **Dropped on the Scene**: a Pokémon joins the scene's cast (its `scene:` line) and is put there, on free ground and off the others, facing the nearest (`who put x y facing other`, after the line playing; one of the cast already: their line written again, as a drag); a thing on someone is held (`who hold thing`), elsewhere put down (`put object thing x y`); a face or a gesture on someone is theirs; the rest is written at the line playing.
  - **Dropped on the Script** (a line under the line pointed at shows where): a Pokémon, its word where it is let go; a face, on a spoken line (`pikachu (happy): …`, in place of the face it had, `bubble` and the rest kept), else a new line to say; a gesture `who sit` (`who pose look_up`, `who anim Swing` for those without a word of their own); `music word`; `sound word`; a thing, held by who is chosen (or by the one the line is of); a place, a new `scene:` with the cast of the one before; a move, `who use Move on other`. A blank line is written over; the line goes as far in as the lines around it.
- **Panels**: **View › Add Panel** opens or closes those of the space (the Assets, the Inspector, the State, the Timeline, the Wiki docked…; the Script and the Scene are always there), and each tab has its ×; **View › Default Layout** puts them back. The panels each space shows are kept in the settings (`panels_film = …`) and open so the next time.
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
- **The Timeline** (under the Scene, across the window; its border dragged makes it taller) shows, from top to bottom:
  - **the scenes**, named by their place, then **the moments** (diamonds, their name);
  - **a track for each of the cast**, in the film's order: their lines said, gestures and moves, a bar each from its start to its end, coloured by its kind; when they are on stage (faint), when they move (a blue line under it). A line goes on the track of who does it: who says it, the one it is written of first (`pikachu hop`), or the `who` of the form it was read as (`walk pikachu to …`). Two lines of theirs at the same time sit one under the other;
  - then **Camera**, **Sound · Music** (each piece from the line that asks for it to the next, the sounds as strokes, over their lines' bars), **Screen** (transitions, titles…) and **Waits · Other**;
  - a bar's words leave out whose track it is (`hop happy`, `“Whoa...”`), run on to the next bar so the verb reads even on a short one, cut with … where they must; a line beside the film (`&`, `beside`) is paler, outlined, marked ∥; the cursor's, lighter. Hovered: the whole line, when, how long, why it starts there, and what a drag does;
  - **▾** before a track's name folds it to a thin strip (▸ opens it); the panel scrolls down when the tracks are more than it holds;
  - at the very bottom, **the whole film** small, the frame of what is seen of it: a click or a drag goes there.
- **The kinds** of the legend: a click hides or shows them.
- **Editing on the Timeline** (each change one undo step, the Script, the Scene and the Timeline following; what was written said under the Scene):
  - **drag a bar** left or right (to a tenth of a second): the `wait` before it is made longer or shorter (taken away at nothing; `wait 0.5s` written if there was none), and the `wait` after it the other way, so the lines after it stay where they are; **Shift+drag**: they move with it. A line beside the film (`… &`) is taken from where it is and written where it then starts: before the line starting then, or in a `beside` block that waits until then;
  - **drag a bar onto another's track**: they do it (`pikachu hop` → `piplup hop`, `pikachu: Hi` → `piplup: Hi`, `walk pikachu to …` by its form's `who`), with its new time if moved across as well;
  - **Alt+drag** a bar (or **Run Alongside (&)** / **Back in Sequence** in its right-click menu): `&` put at its end, or taken away (`& as name` too);
  - **pull the end** of a bar that says its length (`wait 2s`, `wait 18f`, `… in 1.5s`, a walk): its time is written again, in frames if it was in frames (`in Ns` added to a walk that had none);
  - **double-click on a track** at a time: a small window to write a line there, for that track (one of the cast: a gesture, what they say, a walk from where they stand; the camera: glide here, zoom in or back, shake; the screen: fade out or in; any line typed; `wait 1s`). It is written after the line playing then, a `wait` to reach it (a wait playing then split in two round it); **Beside the film (&)** writes it so that nothing after it moves. Esc or a click outside closes it;
  - **an asset let go on a track** (from the Assets panel) is written so at that time: a gesture, a face (`who (face): ...`), a thing held, a move for whose track it is; a song or a sound on any track;
  - right click on a bar: go to the line, play it in a loop, add `wait 1s` after it, alongside or in sequence, duplicate it, delete it.
- **On the ruler**:
  - a click or a drag goes there; Shift+drag chooses a stretch played in a loop (right click, or "✕ loop", to remove it);
  - the lines with a problem are marked in red;
  - when hovered, the time pointed at.
- **Moving around**:
  - a click on a bar goes there, and puts the Script's cursor on its line (the Scene shows it); on a scene or a moment, at its start and on its line; a double-click on a scene shows it across the whole width; a click elsewhere goes to that moment; a click on a name chooses them (as on the Scene);
  - **drag the empty space** (or drag with the middle button anywhere) to move along the film; the wheel scrolls the tracks, Shift+wheel moves along the film; **Ctrl+wheel** zooms in or out, around the pointer;
  - **See All** shows the whole film; **Follow the Playhead** keeps it in sight;
  - the pointer over it: Space plays or stops, ← → from a line's start to the next one, Home, End.
- **The Inspector** edits the cursor's line (‹ ›: the line before, the next one; its kind in colour), the line as written above in a box, wrapped. Its fields in two columns: the name (made readable: `walk_or_run` reads `walk | run`, `_` as spaces; wrapped), the value; a field of one word of a few (`lines | catmull | bezier`) is a row of buttons, a long value (points…) a field on lines of its own. A comment has no **When It Plays**.
  - **A line said**: who speaks (the film's cast), their face (the game's), their manner (fast, slow, top…), what they say.
  - **Another line**: each field of the form that read it when the film played, to change according to its kind. Someone is chosen from the cast, a number is dragged, the rest is written; a place can also be chosen with a click on the Scene ("on the scene", Esc: no). A field not written says its default value.
  - **The word**: its forms and what it does.
  - **When it plays** (a button to go there), and who is on stage at that moment.
  - **What is said about it**, with the fixes offered, in one click.

| Keys (pointer over the Scene or the Timeline) | |
|---|---|
| Space, K | play, stop |
| ← →, J L | a second before, after (the Timeline: from a line's start to the next) |
| Shift+← →, `,` `.` | a frame before, after |
| Home, End | the start, the end |
| F6, F10 | step by line or not; on to the next line |
| Ctrl+wheel | the Timeline closer or further (the Scene: the wheel) |
| Shift+wheel | along the film, on the Timeline |
| Shift+drag | measure on the Scene; on a bar: the lines after it move too |
| Alt+drag | a bar beside the film (`&`) or back in sequence |
| Ctrl+click | the point (or who is there) written at the Script's cursor |
| Esc | no more choosing a point on the Scene |

## Decor

The whole Decor space (Scene, Hierarchy, Inspector, Library, terrains, walls, walking): `decor/DECORS.md`. Wherever a decor is chosen (Home's Decors shelf, the Game space's home, `scene:` in the Script), the decors and places come grouped by kind of place (Town, Beach & sea, Forest & grove, Cave & mountain, Guild & interiors, Dungeon entrances, Other: `library/tables/place_kinds.txt`), each by its name.

- **The Library** (under the Scene, `workshop_decor/src/library/`):
  - **one search at the top**, always there (Ctrl+F over the Library or the Scene, or `/` anywhere in the space; Esc empties it): elements, backdrops, models and the project's decors at once, each kind in its own group, the best first (by name, tag, place; a decor found opens with a click);
  - on the left: **This Decor's Place** (the elements of the place it is made from, `like`, its backdrop or its `level`; a decor that has one opens the Library on it), Favourites, Recently Laid, All Elements, **Backdrops**, Models, the categories, the game's places by kind;
  - pointed at, an element shows **at its own size** (×2, ×3… while it fits), playing, on a checkerboard.
- **Backdrops**: each place of the game without its elements (its rest, `r<level>`). A double-click, Enter or a drag onto the scene lays the decor on it: `put r<level> 0 0 shadow off layer ground locked`, before the first piece (under them all; locked, so a click takes what is over it); a decor that has one has it changed where it is. Right click: the decor made the backdrop's size too, its place's elements, the backdrop taken off. One step each, undone by Ctrl+Z. (`base level` is left to `decor import`: the Workshop does not draw it.)
- **Laying**: a click on a card takes it as a **stamp**: its ghost follows the pointer exactly where it would land, each click lays one (each its own undo step), Esc or a right click puts it down; a double-click lays it in the middle; a drag lays it where it is let go, its ghost showing where. **Snap** (S, or the tools bar) lays and carries on the decor's grid (`grid n`, else 8); Alt held turns it the other way for a moment. **Lays on** (tools bar): the layer what is laid goes on (Ground: under the characters; Objects: as the game, its top in front of them; Over: in front of everything).
- **Walls**: the **Walls** tool (M) paints them (left: walled, right: freed, Shift: given back to what is laid: `wall tiles`, `free tiles`); **Show Walls** (C, or the tools bar) shows where the characters cannot walk, in red. The Walls layer counts the tiles walled as the decor was baked last (its tip: those painted by hand).
- **The marker tools** (tools bar, `workshop_decor/src/markers.rs`): **Resident** (K), **Spot** (T), **Entrance** (E), **Exit** (X), **Zone** (Z). A click on the scene lays one there (one undo step), a click or a drag on one of its kind takes it and moves it (the others of the decor are left alone); each of its kind has a ring on the scene. A small window opens beside the one laid or taken, its essentials written as they are filled:
  - a resident: its Pokémon, sought in the Pokémon catalog (name, number, type; Enter takes the first), and the first line it says: `resident piplup 200 120 down` / `says Welcome to the beach!` (after `[life]`);
  - a spot, a zone: its name (`spot spot1 120 80`, `zone zone1 84 84 32 32`);
  - an entrance: its name (`entrance 160 236 north_gate`);
  - an exit: where it leads, a decor of yours (its menu) or a game place's number (`exit 284 112 32 16 to my_harbour left`).
  Esc or Done closes it; the Inspector has the rest (facing, wandering, size, `on zone`…).
- **The Terrain brush** starts on the tiles of the decor's own place; each swatch is named by what it looks like (grass, sand, water, earth, cliff, rock, snow, lava…), its picture large on hover.
- **The Inspector, nothing chosen**: each setting not said (Grid, Level, Music, Seed) says what it is then and what to tick; a marker's empty name says what to type (`by_the_well`, `north_gate`).
- **▶ Try in a Film** (tools bar, once the decor is baked): a scratch film in this decor, made on a thread (`workshop/src/try_film.rs`, `workshop_home::new_film::tried`): the Walk tool's Pokémon and a friend put on its ground by the scene itself, framed, the first saying where they are, walking to each of its entrances (else across it), the second following. It is written in `target/workshop/try/<decor>.film`, never the library, and opens playing in the Film space; there **Keep in Library** copies it into your films (`try_<decor>.film`).
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
| K T E X Z | Resident, Spot, Entrance, Exit, Zone |

## Home

The project at a glance (`workshop_home/`). In its bar: **+ New Film**, **+ New Decor**, **Your First Film in 5 Minutes** (the welcome panel again; Help has it too), **Games** (with the game-making spaces shown).

**New Film** (`workshop_home/src/new_film/`): four steps on one window, Back and Next under them, a step done clicked to go back to it:
1. **Place**: your decors (their picture), the places films use most, then every place of the game by kind (`library/tables/place_kinds.txt`), searched by name; a double-click goes on.
2. **Cast**: the Pokémon with a body (the game's, those imported from SpriteCollab, the variants), their portraits, searched by name or Pokédex number; two to six, numbered in order (the first two lead), a click on a name takes it out. Searched, SpriteCollab's Pokémon not imported yet are offered too: a click opens SpriteCollab on it.
3. **Start**: A Conversation, An Arrival, A Chase, Two Moments, Empty Stage; beside them, the lines it writes with this cast.
4. **Title** (one offered), the file it will be, **Create the Film**.

The film plays at once: `scene:` with its cast, a `put x y facing …` for each one (where the scene's own placing stands them on its ground, turned to one another: the scene played once, on a thread, to know), the place's `music`, a first `moment:` with its `frame`, the start's lines. It opens in the Film space, on its staging.

**New Decor** (`workshop_home/src/new_decor.rs`): a title, then what it starts from: **A Place of the Game** (made again whole, each element to move or take out: `like <level>`), **Its Ground Only** (the place's picture without its elements, locked under all one lays), or **An Empty Page** (256 × 192, 512 × 384, 768 × 576). A decor made like a place (or laid on its backdrop) opens with the Terrain brush on that place's own tiles.
- **The Project** (on the left): Recent, ★ Favourites, Films, Decors, Drawn Objects (a click: in the Resources), Shared Recipes (`uses`, `include`), Bin; how many of each.
- **The Files** (in the middle):
  - **Open…**: all the films and decors, by their title; **Search the project** (Ctrl+K): the titles, the files, the places, those who play in them, and the words of their text;
  - a **List** (a click on a column sorts it, again: reversed) or a **Grid**; each file with the picture of its place (a film's first scene, a decor itself), made once on a thread and kept in `target/workshop/pictures` (a decor's made again when it changes); while a decor's is made (or if it cannot be), the backdrop of the place it is made like (`like 11`);
  - a click chooses, a double-click opens; ↑ ↓ (← → in the grid) another, Enter opens it, Delete: the bin.
- **The Preview** (on the right):
  - the picture of the place; ▶ opens the film and plays it;
  - the **Details**: the kind, the scenes, the places (as the game names them), the characters, the **Lines Said** (spoken lines), the **Lines of Text** (all), the **State** (what the language says of the film: no problems, remarks, errors), when it changed; of a decor, its size and the films set there;
  - **Open**, **Duplicate** ("_copy", the title too), **Rename** (a decor: also in the films set there, if asked), **Bin** (in `target/workshop/bin`), ★ Favourite, Show in Explorer;
  - in the **Bin**: **Put Back** (in its folder) or **Delete Permanently** (asked once more).

## Resources

Everything a film and a decor are made of, like a studio's asset browser (`workshop_resources/`).
- **The Library** (on the left), groups › shelves, how many on each; a group folds:
  - **Characters**: **Pokémon** (the one catalog, below), **Faces** (each face of each Pokémon its own card, named as a film names it: `pikachu (happy)`, the community's special ones `special_one` to `special_four`; in the films' order: normal, happy… sigh, stunned, then the special ones);
  - **Objects**: Drawn, The Game's, **Items** (the game's items, by their names, with their 16×16 icon of the dungeon floors, from `dungeons.bin`);
  - **Audio**: Music, Sounds;
  - **Effects**: **Attacks** (each move a film names, by its name, with the effects it shows: before it, as it strikes, on the target, what flies, as `dungeons.bin` has them, the very table `effect <Move> on …` plays), **Status Effects** (the game's 25 icons, `data/rom/status`), **Weather** (the sky's: rain, sun, hail, snow, sandstorm, mist, petals, leaves, clear; each card the beach under it, played), **Other Effects** (the game's effects no move shows: the traps', the items', the scenes');
  - **Places**: **Backdrops** (each place or picture of the game a film opens, `scene: <word>`), **Tilesets** (the game's dungeon tilesets, by the first dungeon floor that has each);
  - **Dungeons**: Dungeons, Boss Fights; Decor (elements); and ★ Favourites.
- **The Pokémon catalog** (`catalog.rs`, the Pokémon shelf; the Film's Assets show the same): every Pokémon the project can draw, the game's, those imported from SpriteCollab (the species past the game's, their forms) and the variants painted by hand, **each species once** in the Pokédex's order, **its forms under it** (Eevee's Alternate and Gigantamax, Pikachu's caps, the story's Primal Dialga; Mama, the game's own, under Wigglytuff). The game's doubles a film cannot name (`eevee_733`, a word ending in a number being read as a second Eevee) are left out.
  - **Cards** (`pokemon_cards.rs`): the portrait, the body standing (its idle playing on hover), the name, **the film word** (what to type), `#0133`, a **community** badge (or **hand-made**); **n forms** on a species' card opens its forms after it, on a band of their own (a click again shuts them).
  - **Search** (on the Pokémon shelf the search box seeks the catalog; **Search All Shelves** for the rest): a name, accents, case, `_` and spaces left out (`mr mime`, `Éevée`), a Pokédex number (`133`, `#40`), a type (`fire`), a generation (`gen 8`, `g8`), together (`fire gen 3`); the name itself first, then those beginning with it. A species found shows all its forms when few species are found; a form found alone (`alternate two`, `alola`) shows under its species.
  - **SpriteCollab merged in**: the clone's folders not imported yet (`sources/spritecollab`, looked for on a thread) are cards with a **community · not imported** badge; dragged, double-clicked, written in a film or **Import Now** on its page, SpriteCollab is imported in the background (what is not yet, `spritecollab::import_all`) and everything read again. Tools › SpriteCollab… stays for the rest (a folder looked through, imported again, exported).
  - **Speed**: only the rows in sight are drawn; the cards' pictures are made on a thread, the bodies kept in `target/workshop/pictures/catalog/` (made again when their sheet is newer).
- **The Resources** (in the middle):
  - **Music** and **Sounds** as cards (`music_cards.rs`): a song's title, its word, how long once through, ↻ if it loops, its moods; a sound's word and when it is heard; ▶ on a click (■ hushes it), heard after a moment's rest of the pointer and hushed when it leaves, as the Film's Assets do;
  - a grid of pictures (a Pokémon's portrait, or its body if it has none; an object's first picture; an effect, a status, an attack playing over and over; a wave for a sound), or a **List**;
  - on each picture, **how many films** use it; **Used**: only those; **Sort: usage**: the most used first;
  - **filters**: the Pokémon's generations, the music's moods (calm, mystery, battle…);
  - when hovered, the picture large, what it is, in how many films;
  - **Search everywhere**: in all the shelves at once (the word, the name, what it is); **Sort** by number (the Pokémon in Pokédex order, the effects, music and sounds in the game's) or by name (`Effect 2` before `Effect 10`); **Size** of the pictures;
  - a click chooses, a double-click copies the line of a film that uses it (or plays a sound); ← → ↑ ↓ choose another, Enter as the double-click; right click: copy the word or the line, favourite, listen, open to paint;
  - **+ Create**: **an image**, **an animation**, **a sound**, made in the Workshop (see below); **Import an Image**: a PNG dropped on the window joins the drawn objects under the word of its name (`name.4.png`: four pictures side by side).
- **The Inspector** (on the right):
  - of a Pokémon, **its page** (`inspector/page.rs`): its name, `#0133 · Gen. 1`, its types, its film word (**Copy**), where its pictures come from, **+ Add to the Film** (its word at the Script's cursor); then **Animations** (the game's, then the community's: each a tile playing on hover, cut to what is drawn, named by the words a film says it with, `sit`, `pose look_up`, `anim Swing`; a click plays it in large in the 8 directions, a double click or **Use This Animation** writes `pikachu sit`), **Faces** (each by its film name, shown in large on hover; a double click or **Use This Face** writes `pikachu (happy): …`), **Forms** (the species and its forms, a click shows one's page), **Credits** (SpriteCollab's authors of its sprite and its portraits, from `data/extra/spritecollab_credits.txt` and the clone's `credit_names.txt`: who, when, what, a contact; the game's own said so; **SpriteCollab…** opens Tools › SpriteCollab on it);
  - of a sound, of a music: **Listen When Chosen** (as soon as it is chosen), and the volume;
  - of a drawn object: **Rename…** (the file, and the films that write it if asked), **Delete…** (into the Workshop's bin, `target/workshop/bin`, from where it is taken back by hand);
  - large on a checkerboard, its pixels whole: the pictures one by one (1, 2, 3…) or played (▶ ⏸); of a Pokémon, its faces (normal, joyous, sad…); of a sound, ▶ Listen, ■ Stop;
  - what it is: its word, its name, its size, its pictures and their length, its file;
  - **In a film**: the line that uses it, **Copy Film Line** or **written in the open film** under the cursor's line: `pikachu hold torch`, `music peace`, `krabby walk to right`, `pikachu (happy): ...`, `pikachu hold Oran Berry`, `effect Thunderbolt on pikachu`, `pikachu status burn`, `weather rain`, `effect pikachu 12`, `scene: beach`; a tileset's is a `.dungeon` line (**In a Dungeon**: `look: Beach Cave 2`); of a Pokémon, a portrait, an item, an attack, a status, a weather, an effect, **Watch It Play** plays it on the beach (an attack: Pikachu uses it on a Krabby); each line is checked by a test of `resources/tests.rs`;
  - of an attack, **Its Effects**: each of them side by side, playing, with what it is and its number (a click copies `effect pikachu <number>`);
  - **Used in** n films and m decors, and which (a click opens it in its space);
  - ★ Favourite (kept in `target/workshop/resource_favourites.txt`), **Paint** (the picture opened in the drawing program), **Duplicate**, **In the Folder**, **Copy the Word**.

### The Dungeon space (`workshop_dungeon/src/dungeons.rs`)

Between Decor and Resources, the **Dungeon** space gathers the dungeon editor and the boss fight editor. **All Dungeons** lists, side by side, every dungeon (its floors, its look, how many Pokémon, its boss) and every boss fight (its boss, its level, its guards, its phases); a click on one opens it, **+ New Dungeon** and **+ New Boss Fight** make one. Each one open is a tab in the tools bar (• not saved yet; × or a middle click closes it; Ctrl+Tab the next); they stay open while the list or another space is shown.

### Making a dungeon (`workshop_dungeon/`, its layout: `layout.rs`)

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

### Making an image, an animation, a sound (`workshop_picture/`, `workshop_sound/`)

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

A DS game run inside the Workshop, to see a film as the real game plays it (`workshop_emulator/`, the `emulator` crate). It needs a **libretro core**, the emulator itself as a small library, which the Workshop never comes with: get the melonDS core (or DeSmuME's) from libretro's site (buildbot.libretro.com, nightly, your system, latest: `melonds_libretro`), unzip it anywhere of yours, then **Choose a Core…** in the space (or set `WORKSHOP_LIBRETRO_CORE`). melonDS needs no BIOS or firmware: it uses its own FreeBIOS. See `docs/DS.md`.

- **Load ROM…**: a `.nds` of yours. Both screens are drawn one above the other, crisp, as big as fits; the sound plays at 1×.
- **Keys**: by default the arrows are the D-pad, X the A button, Z the B button, S the X button, A the Y button, Q/W L/R, Enter Start, Backspace (or Shift) Select; the mouse held on the bottom screen is the stylus. **Keys…** opens a window where each DS button gets another key: click it, press the key you want (Escape keeps it); **Back to the Defaults** undoes them all. The keys are kept in the settings (`key_a = K`…).
- **Pause/Play**, **Reset**, **Step** (one frame, paused), **1× 2× 4×**, **Save State** / **Load State** (one slot a game), **Screenshot** (a PNG): the states and pictures go in the settings folder's `emulator/`.
- **Play This Film on DS**: the film open in the Film space exported (`scene_player export-ds --partial`) and put in a copy of your Explorers of Sky (`tools/ds/patch_rom.py --boot --more-actors`, Python with `skytemple_files`) as `target/ds/workshop_play.nds`, then played from power on, A pressed by itself through the title until the film starts (about 25 s). The ROM and armips are asked the first time and remembered; what the steps say, and why one failed, is shown under the screens.

The Workshop's **settings** (`workshop_base/src/settings.rs`) are a small `key = value` file of the user's own: `%APPDATA%\dungeon_workshop\settings.txt` (`~/.config/dungeon_workshop/` elsewhere, or `WORKSHOP_SETTINGS`), holding the core (`libretro_core`), the ROM (`ds_rom`) and armips (`armips`); the emulator's saves, states and screenshots sit beside it in `emulator/`. Nothing of them goes in the project.

## The code

The Workshop is several crates, a part each, so that a change to one part builds only that part and the window over it. In each, a module a subject; a big one is a folder, its `mod.rs` (or the crate's `lib.rs`) holding its state and what its files share. Each crate's `lib.rs` says what it holds and what it uses; `tools/rule.sh` (the building-block rule) lists the crates each may use.

```text
workshop                       the window: its menus, the spaces' docks, what the parts ask
├── workshop_home              the Home
├── workshop_game              the Game space
├── workshop_film              the Film space played
│   ├── workshop_script        the Script
│   │   └── workshop_wiki      the Wiki, the words' catalogue
│   └── workshop_decor         the Decor space
├── workshop_resources         the Resources space and its makings
│   ├── workshop_picture       the painter
│   ├── workshop_music         the music maker
│   │   └── workshop_sound     the sound maker, the instruments
│   └── workshop_dungeon       the dungeon and boss fight editor, the Dungeon space
├── workshop_emulator          the Emulator space
│   └── workshop_first         the first launch, the programs beside the Workshop
├── workshop_tools             the Tools' windows: SpriteCollab, an image converted
└── workshop_base              what all of them share: the session, the look, the project's files
```

Adding a panel, a space or a part: [docs/ADD_A_WORKSHOP_PANEL.md](../docs/ADD_A_WORKSHOP_PANEL.md).

| Crate, module | What it holds |
|---|---|
| `workshop/`: `main.rs`, `app.rs` | the window: its menus, the spaces, their docked panels, the bar of state; what a menu, the palette or a key asks done in the space looked at (`App::run`) |
| `workshop/`: `commands.rs`, `shortcuts.rs`, `video.rs` | the command palette (`Command`, what it finds), Help › Keyboard Shortcuts (read from this README's tables of keys), File › Export Video (`scene_player video` in the background) |
| `workshop_home/` | the Home: its shelves, the files as a table or a grid, the Preview and what is done with a file there (`open` asks the window for its space); New Film (`new_film/`: its text in `mod.rs`, its window in `ui.rs`) and New Decor (`new_decor.rs`) |
| `workshop_first/` | the first launch: the user's copy of the game checked (`check_rom`), the resources extracted step by step (`Phase`, `Event`), what goes wrong said for a person; the programs beside the Workshop (`program_beside`) |
| `workshop_game/` | the Game space: `lib.rs` (the game open, its three columns), `game.rs` (a `.game` file as the space reads and writes it: its card, its chapters as boxes) |
| `workshop_tools/` | the Tools' windows: `sprites.rs` (SpriteCollab looked through, imported), `tilequant.rs` (an image converted to the console's rules) |
| `workshop_emulator/` | the Emulator space: `lib.rs` (a libretro core through the `emulator` crate, its sound through `shell::stream`, a film put on DS), `settings.rs` (the Workshop's own settings) |
| `workshop/`: `capture.rs`, `bug.rs` | a picture of the window without a mouse; a panic told as a bug |
| `workshop_base/`: `session.rs`, `launch.rs` | what every space works with (`Session`: the library, the data, what is read of them and kept, `reload`), how the Workshop was launched (`LaunchOptions`) |
| `workshop_base/`: `theme.rs`, `icons.rs` | the look: colours, sizes, icons drawn with lines |
| `workshop_base/`: `project.rs`, `safe.rs`, `pick.rs` | the project's files (and the kinds of places, the library's films by their words), safe saves, the system's dialogs |
| `workshop_base/`: `ask.rs` | the spaces, and what a part asks of the window (`Ask`: a file opened in its space, the film played (from a line), a file moved, the things drawn shown, a space, a window, a word said), pushed with `Session::ask` and done by `App::answer` once a frame |
| `workshop_base/`: `example.rs`, `tool_run.rs`, `own_sounds.rs`, `done.rs` | a line played as an example, a long piece of work on its thread, the own sounds read as samples, what a making asks of the Resources |
| `workshop_script/` | the Film's Script: `lib.rs` (the film open, its state, the language's thread), `body.rs` (the text drawn and typed in, part after part), `head.rs` (the tabs, the head, the story view), `keys.rs` (the keys taken before the editor), `margin.rs` (the margin, the ends of lines, the ruler), `offers.rs` (what is offered as one types), `popups.rs` (the small windows over the text), `problems.rs` (the problems, the line's box, the status), `highlight.rs` (each piece in its colour), `completion.rs` (what may be typed), `knowledge.rs` (what is known of a word, portraits), `cast.rs` (who is in a scene), `text.rs` and `edits.rs` (where things are, the text changed), `assets.rs` (an asset of the Assets panel written), `plan.rs` (the film's plan), `wiki.rs` (the Wiki beside the text), `windows.rs` (the project's windows), `editor.rs` (laid out), `heard.rs` (sounds named); beside it `story.rs` (the film as a story), `typography.rs` (what is well written), `workspace.rs` (the whole project's films), `debug.rs` (a film looked into) |
| `workshop_wiki/` | the Wiki's pages, a word each: `lib.rs` (a page made of a word and what is written of it, the words it goes with, a recipe's source), `written.rs` (the pages written by hand, `workshop_wiki/wiki/*.txt`: what a word does, its example), `made.rs` (a form as an author reads it, its parts, an example made from it), `search.rs` (the pages found, the best first), `catalogue.rs` (the words by family) |
| `workshop_film/` | the Film played: `lib.rs` (its state), `playing.rs` (its thread), `scene/` (the Scene: `mod.rs` the order of its parts and the view, `marks.rs` what is drawn over the picture, `mouse.rs` what the mouse does on it, `handles.rs` its handles, `drop.rs` an asset dropped on it, `bar.rs` the line played, the bar, the keys, `script.rs` the text read and written again), `timeline.rs` (the Timeline: its axis, the keys, the edits asked made), `bands.rs` (its bands: the tracks drawn, what a click, a drag, a double click or an asset let go asks), `lanes.rs` (which track a line goes on, its bar's words), `edits.rs` (what each Timeline edit writes in the text), `adding.rs` (the window to write a line at a time), `inspector.rs`, `state.rs` (the panels), `lines.rs` (lines written again) |
| `workshop_decor/` | the Decor space: `lib.rs` (the decor open), `scene/` (the Scene: `mod.rs` the view, `drawn.rs`, `marks.rs`, `moving.rs`, `choosing.rs`, `hints.rs` the hint bar), `edit.rs` (its text changed by what is done there), `placing.rs` (where things land: snap, stamp, nudge, layers), `terrains.rs` (the brushes), `library/` (`mod.rs` its parts, `search.rs` what is shown and found, `cards.rs`, `backdrops.rs`), `walk.rs` (a Pokémon walked), `workshop.rs` (the decors' thread); `panels/`, the Decor's panels: `hierarchy.rs` (the Elements' tree), `layers.rs` (the Hierarchy's layers), `history.rs`, `inspector.rs`, `piece.rs` (a piece), `resident.rs` (someone who lives there), `landmark.rs`, `fields.rs` |
| `workshop_dungeon/` | a dungeon or a boss fight made: `lib.rs` (its forms and state), `form.rs` (read, written), `lists.rs`, `state.rs`, `editor.rs`, `layout.rs` (the three columns, the fight in numbers), `fight_columns.rs` (a boss fight's), `dungeon_columns.rs` (a dungeon's, its floors' grid), `arena.rs`, `choosers.rs`, `check.rs`, `floor.rs` (a floor as the game makes it); `dungeons.rs`, the Dungeon space: all the dungeons and fights; `level_csv.rs`, a species exported as CSV |
| `workshop_resources/` | the Resources space: `lib.rs` (shelves, their groups, things, state), `maker.rs` (what is being made: `Maker`), `shelves.rs`, `game_shelves.rs` (the game's attacks, statuses, weathers, effects, portraits, items, backdrops, tilesets), `catalog.rs` (the one Pokémon catalog: built, sought), `pokemon_cards.rs` (its cards, in rows, only those in sight drawn), `thumbs.rs` (the cards' pictures, made on a thread, kept), `music_cards.rs` (the songs' and sounds' cards), `pictures.rs`, `tree.rs` (the Library), `grid.rs` (a shelf's cards), `toolbar.rs` (the tools, the head, the filters over them), `inspector/` (the Inspector: `mod.rs` the order of its parts, `large.rs` the one chosen in large, `about.rs` what it is, how a film writes it, where it is used, `actions.rs` what to do with it, `page.rs` a Pokémon's page, `pokemon.rs` its own actions), `studio.rs` (what is being made, in tabs), `dropped.rs`; `picker.rs`, the Film space's Assets panel |
| `workshop_picture/` | the painting: `canvas.rs`, `pointer.rs` (what the pointer does with each tool), `panels.rs`, `editor.rs`, `simple.rs`, `strokes.rs`, `selection.rs`, `effects.rs`, `steps.rs`, `pixels.rs`, `files.rs` |
| `workshop_sound/` | the sound maker: `synth.rs`, `render.rs`, `voices.rs`, `mix.rs`, `presets.rs`, `instruments.rs`, `versions.rs`, `controls.rs`, `views.rs`, `editor.rs` (the expert's), `settings.rs` (a layer's settings, the mix), `simple.rs` |
| `workshop_music/` | the music maker: `lib.rs` (its state), `project.rs` (the song rendered), `midi.rs`, `editor.rs`, `transport.rs`, `tracks.rs`, `notes.rs`, `mixer.rs`, `keys.rs`, `wave.rs` |

## Launch options

The Workshop's `main` reads, once, what it is launched with (`workshop_base/src/launch.rs`, `LaunchOptions`) and hands it to every space through the session (`Session::launch`); nothing else in the Workshop reads an environment variable.

| Option | Effect |
|---|---|
| `workshop <file>` | that film or decor opened in its space |
| `DUNGEON_DATA=<folder>` | the game's files read from there (else the root's `data/`) |
| `DUNGEON_SPRITECOLLAB=<folder>` | the clone of SpriteCollab the Tools read (else `<root>/sources/spritecollab`) |
| `DUNGEON_FILM_LANGUAGE=fr` | the films said in that language (a line that does not give it: in English) |
| `WORKSHOP_SETTINGS=<file>` | the Workshop's settings there (else `dungeon_workshop/settings.txt` in the system's settings folder): the emulator's paths and keys, `game_spaces = on\|off` |
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
| `WORKSHOP_TOOL=terrain\|scatter\|walls\|walk\|resident\|spot\|entrance\|exit\|zone` | the tool in hand (a marker tool with `WORKSHOP_CHOSEN=n`: the window beside that marker open) |
| `WORKSHOP_MARKER=resident:400:250` | that marker laid there, its tool in hand, its window open (a picture never writes the decor: what it changes is neither saved nor baked) |
| `WORKSHOP_LIBRARY=backdrops\|models\|search:<words>` | the Decor's Library on that shelf, or searching that |
| `WORKSHOP_CHOSEN=n` | line n chosen |
| `WORKSHOP_FOLDED="Transform,Resident"` | those components of the Inspector folded |
| `WORKSHOP_INSTANT=seconds` | the film at that moment |
| `WORKSHOP_SCENE=whole,names,grid` | the Film's Scene shown so |
| `WORKSHOP_TIMELINE=all,insert:pikachu:3.5,fold:camera` | the Film's Timeline showing the whole film, its window to write a line open on that track (`camera`, `sound`, `screen`, `other`, or one of the cast) at that time, a track folded |
| `WORKSHOP_PICKER=gestures:pikachu` (`<tab>[:<pokemon>[:<search>]]`) | the Film's Assets on that tab (`pokemon`, `faces`, `gestures`, `music`, `sounds`, `things`, `places`, `effects`), for that Pokémon, searching that |
| `WORKSHOP_STEP=1` | the Film stepping by line (Step on), playing from the start (or `WORKSHOP_INSTANT`): it stops at the next line |
| `WORKSHOP_LINE=n` | the Script's cursor on line n (brought into sight) |
| `WORKSHOP_PALETTE=<words>` | the command palette open, those words sought |
| `WORKSHOP_SHORTCUTS=1` | Help › Keyboard Shortcuts open |
| `WORKSHOP_WORDS=family` | the words by family open on that one |
| `WORKSHOP_CREATE=image\|animation\|sound\|music\|dungeon\|dungeon:<word>` | the Resources making one (`dungeon:<word>`: that one open) |
| `WORKSHOP_TRY=1` | the open dungeon tried at once; in the Decor space, Try in a Film at once |
| `WORKSHOP_CREATE=fight:<word>` | that boss fight open in its editor (with `WORKSHOP_TRY=1`: tried) |
| `WORKSHOP_BOSS_HP=20` | the fight alone tried with the boss at 20 % of its HP |
| `WORKSHOP_FILE=my_film` | that file chosen in the Home |
| `WORKSHOP_NEW=film:3:beach:pikachu,piplup:chase` | the New Film window open at that step (1 to 4), that place, that cast, that start (`talk`, `arrival`, `chase`, `moments`, `empty`) chosen; `decor[:<place>]`: the New Decor window |
| `WORKSHOP_WELCOME=1` | the welcome panel, as after the first launch |
| `WORKSHOP_PANELS=layout,wiki` | the Layout, the Wiki open beside the Script |
| `WORKSHOP_WIKI=walk\|walk:play\|joy:source\|search:rain\|all` | the Script's Wiki open on that page (its example played, its recipe's source shown), searching that, or on all the words (`all:ds`: DS only, in DS Mode); in a picture it shows over the window |
| `WORKSHOP_COMPLETE=1` | with `WORKSHOP_LINE=n`: the cursor at the end of that line, what may be typed there offered |
| `WORKSHOP_PROJECT=search:word\|beside:film\|versions\|open` | the Script with the search in the project, a film beside, the versions or "Open" |
| `WORKSHOP_FOCUS=1` | the Script in focus |
| `WORKSHOP_RESOURCE=drawing:torch` (`…:body`: a Pokémon's body; `pokemon:eevee:faces`: that part of its page, `animations`, `faces`, `forms`, `credits`; `pokemon?eevee`: that sought) | the Resources shelf (`pokemon`, `portrait`, `drawing`, `prop`, `item`, `song`, `sound`, `attack`, `status`, `weather`, `effect`, `backdrop`, `tileset`, `piece`, `dungeon`, `boss`) and the one chosen |
| `WORKSHOP_SPRITECOLLAB=25` | the SpriteCollab window open on that number |
| `WORKSHOP_SEED=n`, `WORKSHOP_DRAWN=1` | the dungeon's floor laid out with that seed, drawn |
| `WORKSHOP_MUSIC=1:instrument` | that track of the song, that view (`notes`, `instrument`, `mixer`, `automation`, `scale`) |
| `WORKSHOP_TILEQUANT=<file.png>` | the image converter open on that picture |
