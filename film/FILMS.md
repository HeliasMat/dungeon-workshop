# Making a film

A **film** is a series of **scenes**. You write it in plain English in `library/films/<name>.film`. `scene_player` (`target/debug/scene_player`, `cargo build -p scene_player`) reads it, checks it and plays it, without the game.

```bash
scene_player play picnic
```

## The workflow

| Command | What it does |
|---|---|
| `scene_player search krabby beach` | finds a Pokémon or a place |
| `scene_player place beach` | the spots of a place |
| `scene_player check <film> [films...] [--all]` | translates, checks, and plays the film in one go, without drawing: its length, then scene by scene the remarks, numbered, with their line, their moment and the fix. A remark is a flaw you can see on screen: someone speaking off screen or hidden by the box; the one being spoken to (whoever spoke just before) or named in the line, off screen for more than a second; a Pokémon that is K.O. or hidden and speaks; with `collisions on` only, a body inside the decor, two bodies on top of each other at rest, a blocked walk, someone treading on the spot or walking through the decor; a line gone before it could be read; a Pokémon that appears or vanishes on screen all of a sudden, with no fade, flash, smoke or vision to justify it (someone who blinks out as they fall, as in the game, or who is replaced by a picture at the same spot, gives none). `???:` and `umbreon?:` (not seen yet) give none, nor does someone being carried or riding on a back. Everything else is counted as details, which `--all` shows: a position adjusted, a variant turned where it is not painted, a large body cut off by the edge, the crowd in front of someone or on a fight's arena, whoever a shot leaves out of the picture, what reading the film notices (a point that has a name, a duration with no unit...), more of the same remark. Fails if there is a problem |
| `scene_player try <film> [moments] [--on who]` | the checking of `check`, then those moments (eight along the film if no moment is given) on a single contact sheet, each with its second and the line playing; `--on`: each one close up on them |
| `scene_player check <files> --no-play` | only reads them again, without playing them: faster, also for `.txt` scenes, but without the remarks about the picture |
| `scene_player check-game <game>` | a whole game checked without a window (a name: `games/<name>/<name>.game`, then `library`; or a `.game`): its file, each film it plays, each place, dungeon and fight; the names not found (film, place, entrance, dungeon, fight, resident, zone, item, Pokémon, chapter), with the nearest; the flags set and never read or read and never set; what it never reaches. Each problem as `file:line: message`, then how many. Fails if there is a problem |
| `scene_player timing <film> [40s] [50s]` | the film's time, without pictures: each line from its start to its end as the engine played it, its track, its thread, why it starts there; at a moment, what is playing then, who is moving and until when, the next line; between two moments, everything that plays there; `line 12`: that line, every time; `--why 12`: the chain that places it; `--no-play`: the plan without playing ("≥") |
| `scene_player state <film> 32s` | that moment as text, without pictures: the view, the zoom, the line, the bars, and for everyone their position, their gaze, their pose, their line, their HP |
| `scene_player view <film> [--scene N] [--every S]` | plays without a window and writes a contact sheet of the whole film or of one scene to `target/films/<film>/sheet.png`, at screen size: the numbers are readable on it |
| `scene_player image <film> 32s line 90 scene 3 end` | without a window, those moments (in a single pass for seconds), one large picture each, in `target/films/<film>/image_<moment>.png` |
| `scene_player play <film> [--auto]` | opens the film in its window; a game: `play <name>.game` (`--new`, `--headless`: nobody at the keys, no window, what happens in order) |
| `scene_player analyze <film> [--json]` | what the language makes of the film, without playing it: each error with its line, its column and its fixes, then its plan (scenes, moments, recipes, values, marks, threads); `--json` for a program |
| `scene_player cast <film>` | scene by scene, who is in it: each by name, their lines, the beats where they act, then the whole film, the most present first; anyone on the `scene:` line or brought in without saying or doing anything is pointed out |
| `scene_player usages <word> [--max N]` | every line of the films and recipes that uses this word, `film:line` and its text, film by film (40 at most); none: the closest words |
| `scene_player rename <film> <old> <new> [--try]` | renames a character through the language's own reading: who speaks, the `scene:` line, the words that name them, `x = species`, never what is said nor the stories; refuses a name already taken, writes nothing if the language finds more errors in the result; `--try`: only says what it would do |
| `scene_player diff <film> [--from <rev>]` | what the engine gets from the film now against its git version (`HEAD`): each beat by its line, scene by scene, removed `-` and added `+`, then the count |
| `scene_player format [films]` | tidies up (two spaces per block, at most one blank line), like Ctrl+Shift+F in the Workshop |
| `scene_player ids <films>` | gives each line said that has none an id of its own, written last (`#k3f9`): its name whatever its words become, for translations; the ids there are kept |
| `scene_player ground <place>`, `grid <place>`, `illustrations [category]` | what the decor allows, a picture with its measurements (in `target/films/<place>/`), the game's pictures |
| `scene_player items [word]` | the objects for films, and their contact sheet in `target/films/items/sheet.png`: our own pictures, the game's decor pieces (`items chest`, `items 186`), the game's items (`items apple`) |
| `scene_player video <film>` | writes `target/films/<film>/<film>.mp4`, picture and sound (needs `ffmpeg`); it contains data from the ROM |
| `scene_player pack <film>` | makes a zip in `dist/`, to keep among friends: it contains data from the ROM |
| `scene_player properties [kind] [--json]` | the kinds of nodes and their properties, from the registry: type, unit, default, advised range, animatable |
| `scene_player schema --json` | the language for a program: each word and its forms, the engine commands, the grammar of values, the codes |
| `scene_player resources [kind] [--json]` | what a film can use (pictures, sounds, music, places, decors...), by the name it writes, with its size or its length |
| `scene_player server` | stays open for an agent: one JSON request per line, one answer per line ("For agents") |
| `scene_player help [command]` | all the commands, or the details of one |

`--json` on `check`, `analyze`, `state`, `timing`, `cast`, `usages`, `diff`: the same thing for a program, on one line ("For agents").

All these times are the ones you see, slow motion included: a moment given by `check` or `timing` is the same as in `image` and `state`.

In the window: `P` pauses, then `.` steps one frame; `Page Up` and `Page Down` go back and forward 10 seconds, `F3` and `F4` one scene, `Home` goes back to the start. The title bar shows the film's current moment.

**To debug**, as SkyTemple's script debugger does (`preview/src/debug.rs`):

| Key | `DUNGEON_FILM_DEBUG` | What it does |
|---|---|---|
| `F6` | `view` | draws over the scene each one (their box, their name in the film, their place in pixels; blue: the player, yellow: someone you can talk to, orange: an object), the tiles one cannot walk on (red), the walls (`wall`, orange) and the player's bounds (blue), the ways out (green, with their scene, listed at the top even when out of the picture), the place's entrances and the spots the film names (crosses); at the top, the scene and the film's line playing, with its text; at the bottom, the options |
| `F7` | `text` | lines are written at once: one press passes them |
| `F8` | `no_fade` | no more fades: the scene is always seen |
| `F9` | `floors` | in a dungeon, the next floor a moment after each arrival, up to the last (the boss stays); inside a dungeon, `F9` goes down at once |
| `Ctrl` + digit | `scene=3` | goes to the start of that scene (a new game, without the title screen) |

`DUNGEON_FILM_DEBUG` takes these words separated by commas (`all`: the first four) and also holds without a window: `DUNGEON_FILM_DEBUG=view scene_player image <film> 12s` gives the picture with the debug view (`try` and `view` too); `no_fade` makes the film shorter, its moments change.

**Errors at their line**: `check` (and `try`, `view`, `image`) tells each problem as a compiler does, `path:line:column: message`, the column that of the word in question, that word after it in “ ” when the message does not quote it; the remarks seen while playing, `path:line:column (12.5 s): …`. What goes wrong while playing with no known line (the engine stopping, a scene without an end) takes the film's line playing then (`_line`), and says after where the scene was. The JSON also gives `word`.

## A film

**The rules of the language** (the v2.1 vocabulary it is being rebuilt to, family by family: `film/src/plan.rs`):

1. **Who acts comes first**, like a line said (`pikachu (happy): Hi!`): `pikachu walk to piplup`, `eevee jump`, `piplup emote joy`, `pikachu exit via left`. When nobody acts, the line starts with what happens: `music beach`, `wait 2s`, `frame pikachu piplup`, `transition fade`. One order only: a line written the word first (`walk pikachu to piplup`) is wrong (`E-ORDER`), its fix writing it who first (the Workshop and the language server apply it); the turns of a directed fight (`pikachu Thunder Shock krabby`) stay as they are. Which words are said of someone is read from their forms (a first part `who:pokemon`, `who:code`, a team of one) and from the plan, which writes some word first (`fight a vs b`, `wait`, `open`). The Workshop suggests and writes who first; typed `pikachu `, it offers what Pikachu can do.
2. **One word per idea**: its variants are options of the word.
3. **`off` only for a state that lasts**: sleep, the weather, the music, an emotion, a chase (`pikachu sleep off`, `music off`). An action with a verb of its own keeps it: `drop`, `close`, `vanish`.
4. **Plain English verbs**, an underscore only where nothing else will do (`shake_head`).
5. **The game's assets first**: the sprites' 35 animations, the emotes and their sounds (on by default), the moves' effects, the status icons, the weather's pictures. A drawing of one's own only where the game has nothing, always pixel-crisp.
6. **A film only stages**: it starts no dungeon, no journey, no save: that is the `.game`'s part.
7. **Each word's quality gate**: no remark, a picture looked at, a page in the Wiki whose example plays (CONTRIBUTING.md, "The quality gate of a word").

The words come in families: Structure, Speech, Presence, Formations, Movement, Turning, Body, Emotions, Camera, Screen, Sound, Effects, Objects, Fights, Time, Story; Dungeon, Guild and Fishing come with `uses dungeon`, `uses guild`, `uses fishing` (their words without it: a remark `R-USES`, its fix the `uses` line); the Advanced words (the engine's commands) are folded. A word of the old vocabulary is refused (`E-OLD-WORD`), with the form that says it now.

**Written for the DS**: a line `for ds` before the first scene says the film is to play in Explorers of Sky (`scene_player export-ds`, docs/DS.md), as a new game starts; `for ds scene`, in place of a scene of a game going on. Then what the DS can't play is told, by the table of `film/src/ds.rs` (`export-ds --table`): a word it has nothing for (`E-DS-WORD`, with how to write it another way: `fight` as blows of `attack`), a word that exports with a loss (`R-DS-LOSS`, what is lost, once a word), more than 21 Pokémon at once in a scene (`E-DS-ACTORS`, who the export leaves out), a place that is neither a map of Sky's nor a decor of yours (`E-DS-PLACE`; a decor exports as a map of its own, with a loss), the hero or the partner before the player chose them (`E-DS-HERO`, as a new game starts), a zoom or a frame wider than the screen (`R-DS-ZOOM`: Sky has no zoom). Without the line, nothing of it is said.

**The Story family**, the logic of a story told:

| Written | What it does |
|---|---|
| `option "Give an apple" if apples > 0 once` | in a `choice`: an answer offered only when its test holds, never again once taken (`once`); `option Leave` as before |
| `piplup: {if met: You again! \| Hello!}` | in a line said: the words the test chooses as the film plays (none after the `\|`: nothing) |
| `partner: I have {apples} apples, and {money} Poké.` | a value put in: a flag, a counter, the money the game gives the film |
| `go moment the offer`, `go scene town` | on to another moment of the film, back or ahead (in another scene: that scene opens, set, then goes on from the moment), or another scene from its start; an unknown one is told |
| `if seen(the_offer) < 3` | how many times a moment (or a scene, by its place) was played: counted for the run of a film alone, kept by a game's story from one play to the next |
| `pikachu: Hello! \| fr: Bonjour ! #k3f9` | the line's id, written last, its own name whatever its words become, for those who translate it: `scene_player ids <film>` gives one to each line that has none, the ids there kept |


```text
# a comment
title: Snack time
pinchy = krabby

scene: beach piplup pinchy
music peace
piplup (happy): Hi!
pinchy: Hey!
piplup walk to pinchy
piplup hug pinchy
wait 2s

transition: white
scene: town
```

A **scene** is a place and what happens there. Its header says who is there from the start: `scene: beach piplup pinchy`. They are placed on their own, spread out and turned towards each other. Someone a line brings in or makes appear in the middle of the scene (`enter`, `appear`) is only there from that line on, even if named in the header. Someone brought by a vision or a memory leaves when it ends, in the white. Someone not named there comes in through a line that says so (`enter`, `put`, `appear`, a formation); otherwise `check` says so, and nothing in the scene hides inside a line further down. The objects the story always puts in this place, like the guild's totems, are placed on their own too. In a lived-in place (`resident` in its `.decor`, see `DECORS.md`), the word `residents` in the header also brings in the people who live there, each in their place: `scene: my_harbour pikachu residents`.

The lines at the start of a scene that set it up (`put`, `zoom`, `frame`, `tint`, `music`, `crowd`, `pose`, `status`, `every`, `halo`) are played before the fade: you never see the decor empty. The light goes back to daytime at each scene.

Whoever speaks stays on screen: if they are out of frame, the camera glides to them. The music follows on by itself: a scene that keeps the same `music` lets it go on, another one fades the old one out during the fade and brings the new one in. A scene that says nothing about music, in a different place from the previous one, plays that place's own music, as the game plays it there (`library/place_songs.txt`); a built place plays the one its `music` line names (otherwise that of the first place of its `like`), a dungeon room plays its dungeon's; in the same place, whatever is playing goes on. All the music: `scene_player songs [word]`.

**Several languages**: a line can also be said in other languages, after it, each after a `|` and its code (two or three small letters): `pikachu (happy): Hello! | fr: Bonjour ! | es: ¡Hola!`. The film plays in the language `DUNGEON_FILM_LANGUAGE` names (`DUNGEON_FILM_LANGUAGE=fr scene_player play <film>`, and `check`, `image`... too), in English without it; a line that does not give that language is said in English. Each language has its own `{values}` and `[1s]`. A `|` followed by anything other than a code and `:` stays in the text.

**Transitions**, just before `scene:`: `white`, `wipe`, `cut`, `iris` (the circle closes on the last one who spoke). Otherwise, a fade to black. The `title:` shows on its own on black, before the first scene.

## What happens on its own, and how to turn it off

A film does many things without you writing them. Here they all are, with what to write to take control back. A switch can be written anywhere in the scene, holds until the end of that scene, and must be written again in the next one: every scene turns everything back on.

**On every line**

| What happens on its own | When | To turn it off |
|---|---|---|
| Acting from whoever speaks: a hop, an emotion bubble and a sound when their face changes (`joyous`, `surprised`, `angry`, `worried`...), a little hop on "?!", a shake of the head on "No..." | always | write the face you want: `pikachu: No.` acts nothing |
| The speaker and whoever spoke before turn towards each other | always | `autoface off`; for just one: `holdgaze pikachu` |
| Everyone in the scene looks at whoever speaks | always | `audience off`; for just one: `holdgaze pikachu` |
| The camera glides to whoever speaks if they are off screen, and stops there | always | `keepspeaker off` |
| The speaker is on screen but under the box: the box moves to the top for their line, the camera does not move; it comes back down on the next line | always (except after `box top`, which keeps it at the top) | `box bottom` puts it back at the bottom for good |
| The reading time of each line | always | `pikachu (fast): ...`, `(slow)` |
| The names of Pokémon and items take their colour in the lines | always | cannot be turned off |

Listeners do not copy the speaker's emotion: `react on` does that, if you want it.

**On every scene**

| What happens on its own | When | To take it back |
|---|---|---|
| Those named by `scene:` and placed by nothing are spread out and turned towards each other | at the start | `a put x y`, `a face b` |
| A `put` in the middle of a scene brings someone in on foot from the nearest edge. The header of a scene (before its first action or `moment:`) places them, in any order with `weather rain`, `music`, `daytime`, `collisions`, `face`; `a put left facing b` turns a to b once all are put, whatever the order | mid-scene | `a appear spot smoke` (in a puff of smoke) |

| No collisions: everyone goes straight to where they are sent, through walls and through each other, and stands exactly where it is written; a crowd takes all the room it needs | always | `collisions on` in the scene; before the first `scene:`, for the whole film (see "Walls and ground: an option") |
| With `collisions on`: someone who lands on a wall or on someone else is put on the nearest free tile (and `check` says so) | with `collisions on` | choose another position (`scene_player ground`), or `collisions off` |
| With `collisions on`: those walking at the same time avoid each other | with `collisions on` | `spacing off` |
| The light, the grey and the weather go back to daytime | at the start | `daytime`, `daytime night stars`, `weather` in the scene |
| The music goes on if the scene keeps the same one, otherwise it fades from one to the other | at the start | `music x`, `music off fade 3s` |
| With `frame`, the view leaves room below for the dialogue box | on every framing | `boxroom off`, to write again after each `frame` |
| The scene opens on its first framing: a `frame` right after the lines that set the scene (its `moment:` before it, if any) is a cut made before the fade, not a glide | at the start | a line before it (`wait 1s`), then `frame` glides |
| A large body (Gyarados, Groudon) is measured so it fits in the framings | always | cannot be turned off |

**Fights** (`fight a vs b`, `fight directed`)

| What happens on its own | To take it back |
|---|---|
| The view frames the whole field once at the start, then stays still | `frame` before the fight; the fight reframes at the start |
| The recoil, the blinking, the numbers, "It's super effective!", a slow motion on a big hit, cheers from the allies on a critical hit, the fall of a K.O., the winner's joy | cannot be turned off; a hit written by hand (`a Thunder Shock b -30`) shows only what it says for the damage |

**Characters**

| What happens on its own | To take it back |
|---|---|
| After `eevee evolve umbreon`, the line `eevee:` shows Umbreon's name and portrait | always write `eevee` for them |
| A variant painted facing front only (`views down` in its table) stays facing front | cannot be turned off: turning it shows its old picture, and `check` says so |
| `pikachu_pecheur` walks and turns every way, fishes facing any side, and keeps its side while its line is in the water; it sleeps facing front | `pikachu_pecheur reel` frees it (by hand: `pikachu_pecheur holdgaze off`); `pikachu_pecheur sleep off` wakes it up |

## What you can write

The words of a film, family by family (`film/src/plan.rs`); each has its page in the Workshop's Wiki, with an example that plays, and its forms in the table "All the words" below.

| Family | Words |
|---|---|
| Structure | `title`, `scene:`, `moment:`, `transition fade\|white\|boss\|wipe\|iris [in\|out] [d]`, `banner chapter\|place\|dungeon text`, `define … end`, `repeat n … end`, `if … else … end`, `choice … option … end`, `set flag [value]`, `set flag off` |
| Speech | `who (face): text`, `who think (face): text`, `narrate [box\|screen\|sign]: text`, `box top\|bottom\|auto`, `who explain`, `who whisper other` |
| Presence | `who put where [facing side]`, `spot name x y`, `who enter [from side] [to where]`, `who exit [via side]`, `who appear [where] smoke\|fade\|flash\|vision`, `who vanish smoke\|fade\|flash`, `crowd species n area` |
| Formations | `form row\|column\|ring\|lineup who... [to where] [walk\|run]` |
| Movement | `who walk to where [slow\|brisk] [backward] [via a b...]`, `who run to where`, `who shift to\|by where`, `who jump [to where] [height h]`, `who hop [happy\|surprised\|angry] [times n]`, `who follow other`, `who chase other`, `who flee from other [to where]`, `who wander [area]` (each `off` to stop) |
| Turning | `who face side\|other\|away [fast\|slow]`, `who look around\|left\|right\|up\|down`, `who nod [times n]`, `who shake_head` |
| Body | `who pose animation [once\|loop\|hold]`, `who sleep`, `who eat [item]`, `who hurt [hard]`, `who tremble`, `who fall [over\|back\|trip]`, `who faint`, `who cower [from other]`, `who hug other`, `who comfort other`, `who cheer` |
| Emotions | `who emote exclaim\|shock\|sweat\|joy\|laugh\|question\|notice\|anger\|munch [silent]`, `who react joy\|surprise\|anger\|fear\|doubt\|relief`, `who sigh` |
| Camera | `frame who... [where] [snap\|follows]`, `glide to where\|who [in d]`, `shake [soft\|hard]` |
| Screen | `flash [white\|colour]`, `memory … end` |
| Sound | `music track [fade d] [volume v]`, `music off [fade d]`, `sound name [on who] [loop]`, `jingle name` |
| Effects | `effect name on who\|at where\|from a to b`, `weather rain\|snow\|sand\|fog\|hail\|sun [heavy]`, `daytime morning\|day\|evening\|night [stars]`, `who status state\|stat up\|down` |
| Objects | `who hold item`, `who drop [where]`, `who give item to other`, `who throw item at other\|where`, `open thing`, `close thing` |
| Fights | `fight who... vs who... [duel\|double\|relay] [boss] [level n]`, `who attack Move [at target]`, `who evolve species`, `who join [team]` |
| Time | `wait d\|who\|all\|sound\|music`, `together … end`, `mark name`, `at name + d: …`, `pace slow\|brisk\|normal\|n`, `pace lines d\|off` |
| Story | `option "text" [if test] [once]`, `{if test: text \| other}`, `{value}`, `go moment name`, `go scene name`, `seen(name)`, `who: text #id`, `keep flag = value` |
| Dungeon (`uses dungeon`) | `stairs [where]`, `stairs take who...`, `trap kind [on who]`, `ambush who... on other`, `who shop item price...`, `who shop buy item`, `who steal item` |
| Guild (`uses guild`) | `job take\|done\|fail name`, `rank team rank`, `morning_call`, `dinner` |
| Fishing (`uses fishing`) | `who cast [at where]`, `bite [strong\|nibble]`, `who reel [catch what\|snap\|lose]`, `who release item` |

The **advanced words**, folded in the Workshop, are the engine's own commands (`layer`, `tint`, `zoom`, `node`, `prop`, `attach`, `timer`, `every`, `while`, `random`, `tempo`, `halo`...: `engine/docs/COMMANDS.md`), and the lines of a place's scene the game makes (`player`, `dialogue`, `explore`, `game`, `way`).

**The nodes' properties** (`prop who property value`):

<!-- properties: written by film/src/tests/docs.rs, from engine/src/props/ -->
| Property | Type | Unit | Default | Advised | Animates | Kinds | What it is |
|---|---|---|---|---|---|---|---|
| `position` | point | px | `0 0` |  | yes | all | where it is (a body: its feet), in pixels of the scene |
| `x` | number | px | `0` |  | yes | all | its position, from left to right |
| `y` | number | px | `0` |  | yes | all | its position, from top to bottom |
| `angle` | number | deg, turn | `0` |  | yes | all | turned by so many degrees, around its pivot (clockwise) |
| `scale` | number | x, % | `1` | 0.05 to 8 | yes | all | its size (1: its own), in width and height |
| `scale_x` | number | x, % | `1` | 0.05 to 8 | yes | all | its size in width |
| `scale_y` | number | x, % | `1` | 0.05 to 8 | yes | all | its size in height |
| `skew_x` | number | deg, turn | `0` | -80 to 80 | yes | all | slanted by so many degrees, sideways |
| `skew_y` | number | deg, turn | `0` | -80 to 80 | yes | all | slanted by so many degrees, up and down |
| `pivot` | point | px | `0 0` |  | yes | all | the point it turns and grows around, from its position |
| `flip_x` | yes/no |  | `off` |  | no | all | flipped from left to right |
| `flip_y` | yes/no |  | `off` |  | no | all | flipped upside down |
| `opacity` | number | 0..1, % | `1` | 0 to 1 | yes | all | how much of it shows (1: all, 0: nothing) |
| `tint` | colour |  | `#ffffff` |  | yes | all | the colour multiplying its own (white: its own, black: a silhouette) |
| `add` | colour |  | `#000000` |  | yes | all | the colour added to its own (black: nothing), a flash |
| `fill` | colour |  | `none` |  | yes | all | painted all in one colour, its shape kept (none: no) |
| `blend` | choice: normal, add, multiply, screen, overlay, lighten, darken, difference |  | `normal` |  | no | all | how it blends with what is under it |
| `smooth` | yes/no |  | `off` |  | no | body, thing, picture | turned or grown, smoothed (bilinear) rather than in sharp pixels |
| `z` | number |  | `none` |  | yes | all | its rank, in front of those of smaller z (none: as its kind, bodies by their feet; the “z ranks” table) |
| `visible` | yes/no |  | `on` |  | no | all | whether it shows |
| `height` | number | px | `0` | -64 to 400 | yes | body, thing | so many pixels above the ground |
| `sink` | number | px | `0` | 0 to 64 | yes | body, thing | sunk so deep in water (or sand), the part below hidden |
| `depth` | number | px | `0` |  | yes | body, thing | drawn as if it were so much lower, among the others |
| `slowmo` | number | x, % | `1` | 0.05 to 8 | yes | body, thing | its own time (1: the scene's, 0.3: slowed down) |
| `animspeed` | number | x, % | `1` | 0.05 to 8 | yes | body, thing | the speed of its frames (1: their own) |
| `shadow` | yes/no |  | `on` |  | no | body, thing | whether it has its shadow |
| `facing` | choice: down, downright, right, upright, up, upleft, left, downleft |  | `down` |  | no | body, thing | where it looks, one of the eight directions |
| `radius` | number | px | `40` | 1 to 400 | yes | halo | its width, from the middle to the edge |
| `colour` | colour |  | `#ffffff` |  | yes | halo, emitter | its colour (a body: tint) |
| `rate` | number | /s | `4` | 0.1 to 60 | no | emitter | so many particles a second |
| `outline` | colour |  | `none` |  | yes | body, thing, picture | an outline of this colour around its shape (none: no) |
| `outline_size` | number | px | `1` | 0.5 to 8 | yes | body, thing, picture | the outline's width, in pixels of the scene |
| `glow` | colour |  | `none` |  | yes | body, thing, picture | a glow of this colour around it, in three sharp bands (none: no) |
| `glow_size` | number | px | `3` | 1 to 16 | yes | body, thing, picture | the glow's width, in pixels of the scene |
| `drop_shadow` | colour |  | `none` |  | yes | body, thing, picture | a shadow of its shape, of this colour, offset (none: no) |
| `shadow_offset` | point | px | `2 2` |  | yes | body, thing, picture | how far the drop shadow is offset, in pixels of the scene |
| `dissolve` | number | 0..1, % | `0` | 0 to 1 | yes | body, thing, picture | the share of its pixels gone (0: none, 1: all), drawn by its seed |
| `seed` | number |  | `0` |  | no | body, thing, picture | the seed of its dissolve: same seed, same pixels |
| `wave` | number | px | `0` | -16 to 16 | yes | body, thing, picture | its lines wave by so many pixels of its picture, sideways (0: no) |
| `wave_length` | number | px | `16` | 1 to 256 | yes | body, thing, picture | one wave every so many lines of its picture |
| `wave_speed` | number | /s | `1` | -20 to 20 | yes | body, thing, picture | so many waves a second (0: still) |
| `swap_from` | colour |  | `none` |  | yes | body, thing, picture | a colour of its picture, changed into swap_to (none: no) |
| `swap_to` | colour |  | `none` |  | yes | body, thing, picture | the colour swap_from takes |
| `flash` | colour |  | `none` |  | yes | body, thing, picture | it turns to this colour, by flash_amount (none: no); a silhouette: fill |
| `flash_amount` | number | 0..1, % | `1` | 0 to 1 | yes | body, thing, picture | how far it turns to the flash's colour (1: all) |
<!-- end of properties -->

**The z ranks**, from the back to the front:

<!-- z ranks: written by film/src/tests/docs.rs, from engine/src/stage/layers.rs -->
| z | What is drawn |
|---|---|
| -30 | the place |
| -20 | the top of the place (roofs, trees) |
| -10 | the light on the place |
| 0 | bodies and things, by their feet (on the ground -1, among 0, over 1) |
| 10 | fishing lines |
| 20 | what moves throw |
| 30 | statuses |
| 40 | emotes and effects |
| 50 | the light on bodies |
| 60 | a thing shown large |
| 70 | a picture in place of the scene (shot) |
| 80 | a dungeon's name |
| 90 | the weather |
| 100 | the spotlight, the lights |
| 110 | halos |
| 120 | pictures (node … picture) |
| 130 | lightning bolts |
| 140 | ripples |
| 150 | particles |
| 160 | health bars |
| 170 | words rising |
| 180 | bubbles |
| 190 | film bands |
| 200 | the digits of blows |
| 210 | glowing particles |
| 220 | the screen without colours |
| 230 | the fade |
| 240 | the wipe |
| 250 | the credits |
| 260 | the caption |
| 270 | words kept on screen |
| 280 | a fight's log |
| 290 | gauges |
| 300 | a dungeon's map |
| 310 | the dialogue box and its choices |
<!-- end of z ranks -->

**How long the engine's commands last:**

<!-- engine times: written by film/src/tests/docs.rs -->
| Command | Time |
|---|---|
| `vanish` | lasts |
| `opacity` | lasts |
| `jump` | lasts |
| `go` | lasts |
| `hop` | lasts |
| `spin` | lasts |
| `fall` | lasts |
| `step` | until |
| `walkto` | until |
| `rise` | lasts |
| `approach` | lasts |
| `runto` | until |
<!-- end of engine times -->

**A word of the old vocabulary** (`fadein`, `turn`, `camera`, `gathering`...) is wrong (`E-OLD-WORD`): `check` says the form that replaces it, and writes the line again when it can (`fadein 30`: `transition fade in 30`; `turn pikachu left`: `pikachu face left`). The table of them is `REPLACED` in `film/src/plan.rs`.

## A game: chapters, places, cutscenes

A film is a cutscene: it plays, ends, and that is all. What one *plays* is a **game**: a
`.game` file says its story by chapters; the places one walks about are decors, their
`[life]` says who lives there and what happens; the films are the cutscenes it plays; the
dungeons are its own. They only know each other by their names. A game is a folder of its own,
`games/<name>/` (its `<name>.game`, `films/`, `decors/`, `dungeons/`, `fights/`, `music/`, `sounds/`,
`images/`, `tilesets/`; see `games/README.md`); each name is looked for in that folder, then in
the shared library `library/`.

```text
# games/my_game/my_game.game
title: The Little Village
author: Dungeon Workshop
team: pikachu piplup                  # the hero, the partner
home: my_market south            # where a team K.O. in a dungeon comes back to

chapter 1 "The Quiet Fireflies"
  play the_arrival                    # a film of films/
  go my_market south             # into that place, by that entrance
  on talk pidgeot                     # a rule: talking to Pidgeot…
    dungeon my_forest
    if won
      set fireflies_back
      chapter 2
    end
  end
end

chapter 2 "The Glowing Market"
  play the_return
  on talk kecleon if not thanked
    set thanked
    play the_thanks
    finish
  end
end
```

- **The head**: `title:`, `author:`, `team:` (two Pokémon), `home:` (a place and maybe an entrance).
- **A chapter** opens at the start of a line, `chapter 2 "Its name"`, and closes with `end`.
  What it says plays when it starts, in order; a film, a dungeon or a screen is waited for.
  `chapter 3` *indented* is the action that goes on to chapter 3.
- **The actions** (in a chapter, in a rule, in a place's `on … end`): `play <film>`,
  `set <flag>`, `clear <flag>`, `add <counter> <n>`, `chapter <n>`, `go <place> [entrance]`,
  `dungeon <name>` and `fight <name>` (then `if won`), `open <place|dungeon>` (on the map),
  `give [n] <item>`, `take [n] <item>`, `money <n>`, `recruit <pokemon>` (it joins the team's recruits, as the rules make one of its species, and stays in the save), `shop <shop|market|bank|storage|jobs|recruits>`,
  `map`, `finish` (the game's end card, “The End” and its title, until Enter goes back to the
  title screen; the same once the last chapter has nothing left to play; headless, `the end`);
  and `if … else if … else … end`, tests as in films:
  `if chapter >= 2 and not met_kecleon`.
- **The rules**: `on talk <resident>`, `on enter`, `on zone <name>`, each maybe `if <test>`,
  then actions, then `end`. Those of the place where the team is play first, then the chapter's.
- **The places** are decors (`decors/<place>.decor`, the game's, else the library's; one not built yet is built
  as the game first shows it, into `data/built/`, and said once in the log); after `[life]`:

```text
resident kecleon 180 200 down
says if not fireflies_back: The fireflies have gone quiet.
says if fireflies_back: The market glows again, thanks to you!
talk shop                             # then its shop (talk film <film>: a film)
resident pidgeot 100 200 right if not fireflies_back
exit 560 224 16 32 to my_harbour left
zone pier 400 200 64 32               # walked into: `on zone pier`
on talk krabby if not got_apple
  give Golden Apple
  set got_apple
end
```

  The game makes a scene of the place (its residents put, the team at the entrance it came by,
  the player at the keys: arrows, Enter to talk, Shift to run, M or X for the menu: Team, Bag,
  Save, Close), and makes it again when the story changes what it shows.
- **A film reads the game's flags** by `keep`: `keep fireflies_back = false` before its first
  scene, then `if fireflies_back`. The game gives every flag (1 up, 0 down), every counter and
  `chapter` to the film it plays; a `keep` only takes its value when the game gives none.
  **A film sets a flag** with `set <flag>` (`clear <flag>`), in a `choice`'s option or anywhere:
  once it ends, the game keeps it.
- **Saving**: in a place, from the menu, into `saves/<game>.txt`: the chapter, the flags and
  counters, the place, the entrance, where the hero stood, the team, the bag, the money. Never
  in the middle of a film: changing a film breaks no save.
- **K.O. in a dungeon**: back at `home:` (else the place it left from), what was lost told.
- `scene_player play my_game` (a name: `games/<name>/<name>.game`; or a path): its title screen (Continue, New
  game, Quit); `--new` from its start; `--headless` with nobody at the keys and no window,
  what happens told in order (films go by themselves, dungeons count as won, each resident
  who tells the game something talked to once a chapter).
- `scene_player check-game my_game` (a name, or the path of a `.game`) checks the whole game
  without playing it: the game file, each film it plays (as `check` reads them), each place (its
  decor), each dungeon and fight (as the Workshop's "Check the dungeon"); then, walking from the
  first chapter through its rules, actions and the places they lead to: names not found (a film, a
  place, an entrance, a dungeon, a fight, a resident, a zone, an item, a Pokémon, a chapter), with
  the nearest; a flag set and never read, or read and never set (`set`, `add`, a place's tests, a
  film's `keep`, `if`, `set`); a chapter never reached or with no way to end, a place nothing
  leads to, a film or a dungeon only named where the game never goes, a place opened on a map never
  shown, a dungeon and no `home:`. Each problem as `file:line: message`, then how many; it fails if
  there are any. A chapter's reach is told chapter by chapter: a `chapter n` or a `finish` counts
  for a chapter only where its rule can play then: a chapter's own rules in that chapter, a place's
  rules in the chapters where the place may be reached (the home, where the chapter goes, where the
  chapter before left the team, and what their exits lead to) and whose `if` does not rule the
  chapter out (tests on `chapter` alone, with `=`, `<`, `>=`, `and`, `or`, `not`, are worked out;
  one that reads a flag may hold). A fight's problems are on their own lines.
- **The words of before** (`player`, `dialogue`, `passage`, `explore`, `dungeon`, `screen`,
  `map`, `village`, `earn`, `pay`, `stow`, `take_out`) are no longer words of a film: each
  says what to write instead. `player`, `dialogue` and `explore` only live on in the scenes
  the game makes of its places (with `game …` and `way …`, which only it writes).

### A dungeon of your own

```text
# in a chapter of a .game, or a place's `on … end`
dungeon my_forest
if won
  set forest_done
  chapter 2
end
```

- `dungeon <name>`: the team of the game (its hero, its partner, their levels, its recruits, its bag, its money) goes through the dungeon in `dungeons/<name>.dungeon` (the game's, else the library's `library/dungeons/`), floor by floor, turn by turn, like the game's dungeons: wild Pokémon, items on the ground, traps, the stairs, the bag, the menu (Esc). Then the game goes on: `won` is 1 if the team left by the stairs of the last floor (and beat its boss), 0 if it was K.O. or gave up; K.O., it comes back to the game's `home:`, what it lost told. `fight <name>`: a boss fight of `fights/`, alone.
- The team goes in as the game has it (a level written in the dungeon, `level:`, brings it there); what it finds comes back in the game's bag.
- A dungeon takes a few lines to write (`games/my_game/dungeons/my_forest.dungeon`):

```text
name: Firefly Forest
floors: 3
look: Apple Woods                # the decor and shape of the floors, from one of the game's dungeons (and one floor: "Beach Cave 3")
tileset: example_meadow          # optional: tiles of one's own, library/tilesets/<word>.png drawn on the template of 47 cells; the floors keep the look's shape
music: forest                    # optional: the word of a piece of music, or its number; otherwise the look's
pokemon: Caterpie 3 x3 floors 1-2, Pidgey 4 x2, Spearow 6 floor 3   # species, level, how often, on which floors
items: Apple x3, Oran Berry x3, Plain Seed floor 3
recruiting: yes
upward: yes                      # floors are counted going up
level: 5                         # optional: the team's level on the way in
rules: fairy, push_allies        # optional: rule options, each off otherwise (below)
pokemon_type: Clefairy fairy     # optional: a species' types there, one or two
move_type: Charm fairy           # optional: a move's type there
boss: Butterfree 12              # optional: a boss fight at the end (one more floor)
boss_guards: Metapod 8, Kakuna 8
boss_hp: x3                      # its HP times that much
boss_arena: 15x9                 # the size of the arena
boss_shape: pillars              # room, pillars, pool (water along the sides) or cross
boss_map: ......./.X...X./.......   # (instead of the shape) the arena drawn out, one row between each /: . ground, X wall, ~ water
boss_music: battle               # optional: otherwise the dungeon's
boss_at: 10 6                    # optional: the boss's tile (from 0,0 at the arena's top left)
boss_team_at: 10 9               # optional: the tile where the team arrives
boss_allies: Lapras 30 @9,10, Tentacool 25   # optional: they fight alongside the team (on their tile, otherwise near it)
boss_hero: Eevee                 # optional: the fight played alone is played by this team (boss_partner: …, boss_level: …); at the end of a dungeon, by the team that came
boss_floor_items: Max Elixir @3,11, Blast Seed @6,9      # items lying in the arena
boss_traps: spikes @8,7, slumber @10,8 hidden            # traps (hidden: invisible until stepped on)
boss_says: Nobody crosses my forest that easily!        # optional: at the start of the fight
boss_falls: The fireflies... let you through...          # as it falls
boss_item: Golden Apple          # what it holds and drops when beaten
boss_moves: Gust, Sleep Powder, Confusion, Stun Spore   # four at most; otherwise its species'
boss_phase: 60% | says You're starting to annoy me... | anger +2   # under 60% of its HP: it says so, its attack goes up 2 stages
boss_phase: 40% | says To me, my little ones! | reinforcements Caterpie 6, Weedle 6 | weather rain
boss_phase: 20% | heal 30% | moves Gust, Silver Wind, Psychic   # it gets back 30% of its HP and fights differently
boss_recruit: yes                # once beaten, it asks to join the team
```

- **A boss fight is written separately**, in `library/fights/<word>.fight` (the Workshop's Dungeon space › + New Boss Fight): its name (`name:`), the look of its arena (`boss_look: Miracle Sea`) and its `boss…` lines. A dungeon names it with `fight: <word>` (several dungeons can share the same one); `dungeon <word>` in a film also plays a fight on its own. `boss…` lines written in a `.dungeon` are still read.
- **Rule options** (`rules:`, after SkyTemple's patches; `rules/src/rule_options.rs`), each off unless named, also `rules=…` in the settings file for every dungeon: `fairy` (an 18th type, Fairy, with the later games' matchups, given by `pokemon_type:` and `move_type:`), `team_control` (Tab changes the team member you lead; the others follow), `push_allies` (walking into an ally pushes it one tile on if it can go there, otherwise you swap places), `exp_share` (the recruits left at the base get the Exp. too, as the team leaves), `lasting_weather` (the weather a move or an orb brings stays until another comes), `disarmed_houses` (a floor of one single room is never a Monster House). The Workshop's dungeon editor has a checkbox for each, under "Its rules". The team page of the dungeon menu shows the leader's types.
- **The boss fight**: after the last floor, an arena; the team arrives at the bottom, the boss waits three tiles above, its guards (six at most) at its sides. The fight is won, and the dungeon finished, when none of them is left standing. `floors: 0`: the fight alone.
- **Tilesets of one's own**: a PNG of 432×192 drawn on the template of 47 cells (three parts side by side, wall, water or lava, floor, each of 6×8 cells of 24 pixels; in each cell the tile in white, its neighbours alike in black, different in blue; the 256 ways of the neighbours are brought back to those 47), with optional versions `<word>.1.png` and `<word>.2.png`. `decor tileset template <out.png>` writes the blank template, `decor tileset export <n> <out.png>` a tileset of your copy of the game drawn on it to start from, `decor tileset import <png> <word> [dither]` makes a drawing a tileset (tiles of 8, 12 palettes of 16 colours) in `library/tilesets/<word>.png`, read by `tileset: <word>`. `decor tileset example` draws one by code (`example_meadow`, used by `library/dungeons/meadow_path.dungeon`). `decor tilequant <png> [palettes] [dither]` makes any picture keep to the console's rules (`<name>_quant.png`); in the Workshop: Tools › Convert an Image (Tilequant)…, and in the dungeon editor, under its look: its tiles, Export the Template, Export the Look's, Import a Tileset…
- **Placing everyone**: `boss_at`, `boss_team_at`, and `@x,y` after a guard (`boss_guards: Krabby 7 @5,3`); without them, the arena places them (the team at the bottom, the boss three tiles above, its guards at its sides). Wherever someone or something is placed is always ground. Three tiles between the team and the boss: you see it as you come in.
- **Traps**: mud, sticky, grimy, summon, gust, spin, slumber, slow, seal, poison, selfdestruct, explosion, pp_zero, chestnut, wonder, spikes, stealth_rock, toxic_spikes, trip, grudge (no trapdoor or warp, which would take you out of the arena).
- **The staging**:
  - as it enters, a banner with its name and title (`boss_title: The Elder of the Abyss`); if it lets out a cry (`boss_cry:`), the screen shakes;
  - each phase makes a white flash; `shake` in a phase also shakes the screen;
  - on its gauge, a marker for each phase, dimmed once passed;
  - when beaten: a "VICTORY!" banner, a flash and the mission-accomplished music, for a moment, before the film resumes.
- During the fight, the boss's name and HP gauge sit at the bottom of the screen, on a strip below the message box (they hide neither the field nor the minimap). What it says goes into the dungeon's message box.
- **Its phases** (`boss_phase:`, as many as you like): each happens once, when its HP drops below its share, with whatever you put in it, separated by `|`:
  - `says …`: what it says;
  - `anger +n`: its Attack and Special Attack go up n stages (its gauge turns orange);
  - `reinforcements A 6, B 6`: they come up next to it (six at most);
  - `heal 30%`: it gets back that much of its HP;
  - `weather rain`: clear, sunny, sandstorm, rain, hail or fog;
  - `moves X, Y`: from then on it fights with those (four at most);
  - `cry <sound>`: it lets out this cry (a sound of ours);
  - `shake`: the screen shakes.
- The lines `boss_anger: +2 …` (a phase at 50%), `boss_reinforcements: 40% … : …` and `boss_heal: 25% 30%` are still read, as phases.

- The floors are made anew each time: only their look (the decor, the size, the rooms) comes from the named game dungeon. A Pokémon in the dungeon fights like those of its species in the game's dungeons, brought to the level written; a species never met there is refused.
- `floors 1-2` (or `floor 3`) after a Pokémon or an item: only on those floors; without it, on all of them. Every floor must have someone.
- The Workshop has a dungeon editor (its Dungeon space › + New Dungeon), which tries them out right away.

### Sharing a game

```sh
scene_player pack-game my_game        # writes dist/my_game.dwpack
scene_player play dist/my_game.dwpack # a friend plays it, with their own data/
```

- A `.dwpack` is a zip of the game's own files (a whole game folder is its root; what it takes from the shared library `library/` goes in too, at the same place, so the pack plays alone): the `.game`, the places it reaches (and those their exits reach), the films, dungeons, fights and recipe files they name, its own music, sounds, pictures and tilesets, and a manifest (`dwpack.txt`: format, title, author, the files).
- Nothing of `data/` goes in. A place or a film may name the game's Pokémon, places, songs and sounds by their names (whoever plays reads them from their own copy); a picture or a sound taken from `data/` (a path into it, or a name only `data/hand` has) is refused, the file and the word named.
- What goes in is what is named where it is read, never a word that merely spells a file's name (a line of dialogue saying `the_ending` packs no film): the `.game`'s `home:` and actions (`play`, `go`, `dungeon`, `fight`, `open`); a place's exits, `talk film`, `on … end` actions and `image` lines; a film's `uses`/`include` files of the project, and what it plays once compiled: the pictures of one's own it draws, those its words pick by themselves too (the fishing bobber, the rifle's crosshair, the footprints: `film::OWN_PICTURES`), its own sounds and music (`sounds/<name>.wav`, with the `music/<name>.music` it was made from); a dungeon's or a fight's `fight:` and `tileset:`. `library/place_songs.txt` stays out: it is the repository's, read from there, and only says the songs of the game's own places.
- `play x.dwpack` unpacks it into `target/packs/<name>/` and plays it from there (`--headless` too); its save is `saves/<game>.txt`, as for any game.

## Values, tests, loops and events

The whole language is in `language/LANGUAGE.md` (its grammar, its types). The text is read only once: every error has its line, its column and its fix; `scene_player format <film>` tidies a film up (two spaces per block).

```text
let tries = 0                         # a number of the scene (keep x = …: kept in the save)
while tries < 3 and not caught
  throw pikachu_pecheur line to 480 108
  tries = tries + 1
  if chance(40)
    caught = 1
  else if tries = 3
    pikachu_pecheur (sad): Nothing today...
  end
end
pikachu_pecheur: {tries} casts!       # a value in a line
for each q, i in [piplup, eevee]
  jump q
end
when piplup arrives: emote pikachu joy
every random(2s, 4s) as noises: sound click
after 5s: cease noises
when clock > 600: narrator: Time goes by.
```

- **Arithmetic**: `+ - * / %`, `< <= > >= = !=`, `and`, `or`, `not`; `chance(40)`, `random(1, 6)`, `min`, `max`, `abs`, `distance(a, b)`, `a.x`, `a.y`; inside a line of words, between braces: `put a {b.x + 16} {b.y}`.
- **Blocks**: `if` / `else if` / `else`, `while`, `repeat n` (n written out: the lines are copied; a calculation: a loop), `for each x in [..]`, `break`, `next`; `when <test>`, `every`, `after`: running beside the film (`as name`, `cease name`), on one line (`: line`) or as a block until `end`.
- **Choices**: `choice`, one `option …` line per answer, then `end`.

## For agents

An agent does not see the window: it reads numbers. Every command that says something about a film also says it in **JSON**, on **one line**, with `--json`.

**The loop**:
1. write the film (`library/films/<name>.film`);
2. `scene_player analyze <name> --json`: what the language makes of it, without playing it (range, code, message, corrections as changes to the text), and its outline;
3. `scene_player check <name> --json`: what is wrong, read then played (`--no-play`: read only);
4. `scene_player state <name> 12s 30s line 40 --json` at the chosen moments: where everyone is, what can be seen of them, the camera, the box, the sounds;
5. fix it, and start again. Several questions about the same film: `scene_player server` (below), which keeps the film played.

`scene_player properties [kind] --json` (the registry), `scene_player schema --json` (each word and its forms, the grammar of values, the codes) and `scene_player resources [kind] --json` (images, items, game props, sounds, songs, fonts, Pokémon, variants, places, decors, each by the name a film writes, with its size or its length) say what you can write.

### The JSON shapes

Each answer is an object: `command` (its word), `shape` (the version of these shapes: `1`; it goes up only if a field goes away or changes meaning, an added field leaves it as it is), then its fields, **keys sorted in alphabetical order**. Seconds are seconds as seen (slow motion included), to the hundredth; a point is `[x, y]`; a colour `#rrggbb` (`#rrggbbaa` if it is not fully opaque); nothing is `null`. Paths start from the project root, with `/`.

- **A problem** (`check`): `{code, severity (error|remark|detail), source (read|play), line, column, end_column, message, closest [names], correction (the corrected line, or null), advice (what the message suggests after "→"), second, scene}`. Columns start at 1 and count letters.
- **`check`**: `{films: [{film, path, version, plays, ok, duration_s, scenes: [{number, start_s, end_s}], problems: [..], remarks: [..], details: [..]}]}`; it fails (exit code 1) if a film has a problem.
- **`analyze`**: `{file, version, diagnostics: [{range: {start: {line, column}, end}, severity, code, message, notes, corrections: [{title, changes: [{range, text}]}]}], plan: [{kind, name, detail, line, end}]}`.
- **`state`**: `{film, duration_s, moments: [M]}`, and each moment `M`: `{asked, image, second, scene, line, place, cameras: [{name, center, zoom, screen, follows, kept, glides}], box: {shown, upper, line_said}, bars, sound: {music, ambient, sounds: [{sound, name, volume, pan, loop}], recent: [{sound, volume, pan, since_s}]}, tracks: [P], nodes: [N]}`, and each track `P`: `{node, property, t, duration, value, keys, mode (once, loop, pingpong), route, ends}` (`t`, `duration` in seconds).
  Each node `N`: `{ref (m25, node:name, halo:0…), name, kind, properties: {name: value, … every property of its kind, resolved by the registry}, screen: {rect [x, y, w, h], in_frame, visible, covered, covered_by_nodes, covered_by_box, size [w, h], edges {left, upper, right, lower}}, body: {pose, walk, runs, to, statuses, hp}, parent, inherits, world: {position, angle, scale, opacity}, effects: [name]}` (`body` for a body or a thing; `parent` and `inherits`: what it hangs from and what it takes from it, `null` otherwise; `world`: as it is drawn, its parents counted; `effects`: `outline`, `glow`...).
  **What is on screen**: `rect` is in screen pixels (256×192 today), the rectangle holding what is drawn (a body: its given size, on its feet, raised by its height); `visible`: in frame, not hidden, with an opacity above 0; `covered`: the share of what is in frame that is hidden by the nodes in front of it (bodies, things and clearly visible images, in today's order: bodies by their feet, then images) or by the dialogue box, counted on a grid of points; `edges`: the distance to each edge (negative: it goes past it). The caption has no `rect`.
- **`timing`**: `{film, duration_s, without_playing, from_s, at_s, lines: [{line, start_s, end_s, start_frame, end_frame, at_least, track, thread, place: {kind: after|with|at|when, to}, duration: {kind: known|event, frames, waits?}, cause, text}], moves: [{who, how, start_s, end_s}], then}`; with no moment, the whole film; one or two moments, what plays there; `--why N`: `{film, why, chain: [{line, start_s, end_s, at_least, cause}]}`. The server answers the same.
- **`cast`**: `{film, scenes: [{number, place, roles: [{name, lines_said, beats}], crowd, story, silent}], whole_film: [{name, lines_said, beats}]}`.
- **`usages`**: `{word, total, files: [{path, film, lines: [{line, text}]}]}`.
- **`diff`**: `{film, since, state (new|same_text|same_engine|changed), scenes: [{number, place, state, removed, added}]}`.
- **`properties`**: `{kinds: [{name, properties}], properties: [{name, type, choices, unit, unit_written, default, default_written, advised, floor, animatable, kinds, doc}]}`.
- **`schema`**: `{language_version, header, reserves, words: [{word, family, forms, written}], engine: [{name, forms}], genres: [{name, says}], values: {types, units, durations, distances, angles, factors, parts, per_second, colours, points, relative, words, merges}, old_forms: [{old, new, since_version, why}], codes: [{code, says}]}`.
- **`resources`**: `{kinds: {images: [{name, file, size, images}], sounds: [{name, number, when, duration_s, own}], …}}`; the game's sounds are measured only when asked for alone (`resources sounds`).

### The server

`scene_player server` stays open: **one JSON request per line** on its input, **one JSON answer per line** on its output (a request can also be written as words: `state 32s`). The loaded film is compiled and played once, and kept as it stands every 5 seconds: a moment asked for starts again from the last one kept before it, not from the start.

```text
{"command":"load","film":"example"}                  a name, or a path
{"command":"load","text":"scene: beach pikachu\n…","name":"trial"}
{"command":"analyze"}      {"command":"check"}       {"command":"check","play":false}
{"command":"state","t":"32s"}                            t: "32s", 32, "line 90", "scene 3 end", or a list
{"command":"timing"}       {"command":"timing","t":["30s","40s"]}
{"command":"image","t":"32s","path":"target/films/x.png"}
{"command":"properties","kind":"halo"}   {"command":"schema"}   {"command":"resources","kind":"sounds"}
{"command":"quit"}
```

Each answer is the command's own `--json` answer, with `"ok": true`; a refused request: `{"command", "shape", "ok": false, "code", "message"}` (`E-SERVER-REQUEST`, `E-SERVER-COMMAND`, `E-SERVER-NO-FILM`, `E-SERVER-FAILED`). An `id` given in the request is given back in the answer. `load` answers `{film, path, duration_s, scenes, points_kept}`, `image` `{path, second, size}`.

### One language

The language has one version, today's: a word renamed is renamed everywhere, no old form is read. A word not known is told with the nearest names.

### Values, nodes, properties

The grammar of values (durations, pixels, angles, factors, shares, colours, points, relative values) is in "Nodes and properties" above, and `schema --json` gives it (`values`). Every node and every property is in the registry (`engine/src/props/`): the "Properties" table above, `properties --json`, and the `properties` of each node in `state --json` are drawn from it.

### The codes

Every problem has its code, a stable one: `E-…` an error (the film does not play as written), `R-…` a remark (it plays as asked, worth a look). A code is never renamed nor changes meaning; whatever has no code of its own yet gets `E-READ` or `R-READ` (read); whatever the game sees, `R-PLAY-<kind>`.

<!-- codes: written by scene_player/src/commands/server.rs, from each block's codes -->
| Code | What it says |
|---|---|
| `E-PROP-UNKNOWN` | no property has this name (the nearest are given) |
| `E-PROP-KIND` | this kind of node does not have this property |
| `E-VALUE-MISSING` | a property without its value |
| `E-VALUE-TYPE` | a value that is not of the property's type |
| `E-VALUE-IMPOSSIBLE` | a value no node can take: not a finite number, a negative size |
| `E-NODE-UNKNOWN` | no node of the scene has this name |
| `E-NODE-KIND` | no kind of node has this name |
| `E-NODE-TAKEN` | this name is already someone's or something's in the scene |
| `E-ATTACH-KIND` | this node does not attach (the caption, an emitter, a crowd) |
| `E-ATTACH-LOOP` | a node attached to itself, or to a node already hanging on it |
| `E-RIG-FILE` | a rig's file not found or not readable |
| `E-RIG-LINE` | a wrong line in a rig's file (its line is given) |
| `E-CHARACTER-SHEET` | a character's sheet not found, or its cells wrong |
| `E-CHARACTER-ANIM` | a character's animation wrong: its cells, its pace, its row |
| `E-TRACK-PROP-STILL` | a track of a property that does not animate (set sets it at once) |
| `E-EASE-UNKNOWN` | an unknown ease (the “Eases” list in film/FILMS.md) |
| `R-TRACK-ENDLESS` | a looping track never ends: the film does not wait for it (“&”, “stop anim”) |
| `E-TRACK-FORM` | a badly formed anim or keys line: <prop> [from] -> <to> in 2s |
| `E-TRACK-KEY` | an unreadable key of a keys block: <t> <value> [ease] |
| `E-PATH-UNKNOWN` | a path no “path” line made |
| `E-PATH-POINTS` | the points of a path: x y, x y, … (two at least) |
| `R-OUT-OF-RANGE` | a value outside what the registry advises, played as it is |
| `E-READ` | what reading finds wrong, with no code of its own yet |
| `R-READ` | what reading notes, with no code of its own yet |
| `E-ORDER` | a line written the word first, said of someone: who acts comes first |
| `E-OLD-WORD` | a word of the old vocabulary: the form of the plan that says it now |
| `R-USES` | a word of an optional family in a film that does not take its library in (uses dungeon, guild, fishing) |
| `E-DS-WORD` | in a film for ds: a word the DS has nothing for, with how to write it another way |
| `R-DS-LOSS` | in a film for ds: a word that exports with less than the film shows, what it loses |
| `E-DS-ACTORS` | in a film for ds: more than 21 Pokémon at once in a scene, who would be left out |
| `E-DS-PLACE` | in a film for ds: a place that is neither a map of Explorers of Sky's nor a decor of yours |
| `E-DS-HERO` | in a film for ds: the hero or the partner, before the player chose them |
| `R-DS-ZOOM` | in a film for ds: a zoom, or a frame wider than the screen: Sky has no zoom |
| `E-SERVER-REQUEST` | server: a line that is not a request |
| `E-SERVER-COMMAND` | server: no command of this name |
| `E-SERVER-NO-FILM` | server: no film loaded (load one first) |
| `E-SERVER-FAILED` | server: the request failed (its message says why) |
| `R-PLAY-VIEW` | who speaks is out of the picture, or hidden by the box |
| `R-PLAY-ABSENT` | the one spoken to is not in the picture |
| `R-PLAY-WALL` | a body at rest inside the backdrop |
| `R-PLAY-ON` | two bodies at rest on one another |
| `R-PLAY-STUCK` | a walk that does not move forward |
| `R-PLAY-THROUGH` | a walk through the backdrop |
| `R-PLAY-FAST` | a line gone before it could be read |
| `R-PLAY-POPS` | someone appears or disappears all at once, in the picture |
| `R-PLAY-SOUND` | a sound waited for whose length is not known |
| `R-PLAY-FRAME` | the shot or the fight keeps someone who stays out of the picture |
| `R-PLAY-FIELD` | someone of the crowd on a fight's field |
| `R-PLAY-THING` | a thing put inside the backdrop |
| `R-PLAY-CUT` | a large body cut by the edge of the picture |
| `R-PLAY-CROWD` | the crowd, placed otherwise than written |
| `R-PLAY-LINE` | what the game notes about a line |
| `R-PLAY-ITEM` | an item held or put otherwise than written |
| `R-PLAY-PAINTED` | a variant shown from a side it is not painted from |
| `R-PLAY-PLACE` | a place found otherwise than written |
| `E-PLAY-PLACE` | a place the scene does not find as written |
| `R-PLAY-OTHER` | what the game sees, of another kind |
<!-- end of codes -->

## Making it look good
- The view stays above the dialogue box, but a character on the left can end up under the portrait: look at the contact sheet, and place them by hand if needed.
- Few colour veils: keep `tint` for real nights.
- No zooming in: wide views look better.
- Short, natural lines, as between friends, with reactions rather than set phrases.

## All the words

Every word of a film, its forms, the file that reads it (`film/src/verbs/<file>.rs`, `verbs/moving/body.rs` for `body`, `film/src/text/` for `text`, `engine`: an engine command, by its name) and what it writes for the engine (see `engine/docs/COMMANDS.md`).

**The forms**: each word has its own in its line of `WORDS`, in its family's file (`film/src/verbs/`, the engine commands in `engine.rs`), and a single reader, `film/src/grammar.rs`. Each line of a film must fit one of its word's forms, otherwise `check` says which ones and what is missing ("“pikachoo walk to 300 120”: “pikachoo” is not someone of the scene"). When several families share a word, the form decides: `ride pikachu lapras` (fishing), `rise pikachu 24 0.3s` (the engine). An engine command is written in a film only if the language has its form; `>` in front of a line, for debugging only. The notation: `who:pokemon` (someone of the scene), `*who` (one or more), `where:place` (`x y`, someone, `front_of b`, `left_of b`, `between a b`, `3 tiles from b`, `tile 12 7`, a spot), `d:duration` (`1.5s`, `6f`), `n:number`, `word`, `thing...` (one word or more), `text...` (the rest), `rest...` (the end that the word reads itself), `a|b` (one of these words), `[...]` (optional). Recipes are written in the same notation, with default values on top (`turns=4`).

Tests (`film/src/tests/docs.rs`) check that this table is the code's, that each word has its forms, that the `film` skill mentions every word, and that `COMMANDS.md` mentions every command a film writes.

<!-- words: written by film/src/tests/docs.rs -->
| Word | Forms | Read in | Time | Written for the engine |
|---|---|---|---|---|
| `title` | `title text...` | `text` | instant | typecaption before the first scene |
| `scene` | `scene place:word text...` | `text` | instant | scene, backdrop, ground, spread |
| `transition` | `transition kind:fade\|white\|boss\|wipe\|iris\|cut` | `text` | instant | how the next scene comes: whitein, iris, wipe, fadein (and their way out); with in or out, now (verbs/pictures.rs) |
| `define` | `define name:word text...` | `text` | instant | nothing: its lines in the place of its name |
| `repeat` | `repeat n:number` | `text` | instant | nothing: its lines so many times |
| `else` | `else` | `text` | instant | else: the other branch of if … endif (engine commands) |
| `end` | `end text...` | `text` | instant | closes a block (define, repeat, fight directed), or the last one open: a picture's shot, dimensional_scream, memory, choice, fight |
| `go` | `go moment name:text...`; `go scene name:word` | `story` | by the line | on to another moment of the film (its label; in another scene, that scene, then the moment), or another scene from its start (gotoscene) |
| `think` | `who:pokemon think text...` | `acting` | by the line | a line thought, in the box, its portrait (face), its words in brackets |
| `narrate` | `narrate text...` | `acting` | by the line | narrate: words of no one's in the box; narrate screen: words on the screen, centred; narrate sign: a sign read, centred in the box |
| `box` | `box auto` | `acting` | by the line | boxside auto: where it hides no speaker, as the game does: at the foot of the screen, at the top for a line of one standing low |
| `vanish` | `who:pokemon vanish [smoke\|fade\|flash\|vision]` | `acting` | by the line | gone in the game's smoke, a fade (by default), a flash or a vision |
| `shift` | `who:pokemon shift by dx:number dy:number [in d:duration]`; `who:pokemon shift [to] where:place [in d:duration]` | `acting` | by the line | slides there, still looking the same way: go, sliding |
| `jump` | `who:pokemon jump [to] where:place [height h:number]`; `who:pokemon jump height h:number`; `who:pokemon jump` | `acting` | by the line | a leap of his body, higher than a hop: on the spot, or to a place, so high |
| `hop` | `who:pokemon hop [happy\|surprised\|angry] [times n:number]` | `acting` | by the line | the game's hop: his Hop animation; happy, surprised, angry: its emote and its sound (angry: two) |
| `follow` | `who:pokemon follow off` | `acting` | by the line | follow -: stops following |
| `chase` | `who:pokemon chase off` | `acting` | by the line | stop: stops chasing |
| `flee` | `who:pokemon flee from other:pokemon [to where:place]` | `acting` | by the line | runs away from the other: to a place, or a little way |
| `wander` | `who:pokemon wander off` | `acting` | by the line | roams 0: stops wandering |
| `face` | `who:pokemon face away [fast\|slow]`; `who:pokemon face to:word [fast\|slow]` | `acting` | by the line | turns that way, to someone, or round (away); fast: at once; slow: through each way in between, a step every few frames (turnslow) |
| `look` | `who:pokemon look around` | `acting` | by the line | the game's look around: a glance one way, the other, back |
| `nod` | `who:pokemon nod [times n:number]` | `acting` | by the line | his Nod animation, so many times |
| `shake_head` | `who:pokemon shake_head` | `acting` | by the line | the game's head shake: quick turns of the head, one way, the other, back |
| `pose` | `who:pokemon pose off`; `who:pokemon pose name:word [once\|loop\|hold]` | `acting` | by the line | one of his sprite's animations by a clear name: once, again and again, or held at its end; off: standing |
| `sleep` | `who:pokemon sleep off` | `acting` | by the line | awake: his Wake animation, the sleep's mark gone |
| `hurt` | `who:pokemon hurt [hard] [back n:number]` | `acting` | by the line | his Pain animation, knocked back the way he looks; hard: further, the view shaking |
| `tremble` | `who:pokemon tremble off`; `who:pokemon tremble` | `acting` | by the line | trembles until told off |
| `fall` | `who:pokemon fall [over\|back\|trip]` | `acting` | by the line | falls and stays down: his Tumble (over), TumbleBack (back) or Trip animation |
| `faint` | `who:pokemon faint off`; `who:pokemon faint` | `acting` | by the line | his Faint animation, held, until he gets up (off: his Wake animation) |
| `emote` | `who:pokemon emote off`; `who:pokemon emote kind:word [silent]` | `acting` | by the line | an emote balloon of the game over him, with its sound (silent: none); off: none |
| `set` | `set flag:word off`; `set flag:word value:number`; `set flag:word` | `game` | by the line | the game's flag is up (1): the game keeps it once the film ends; in a choice's option, the choice told to the game: set |
| `player` | `player who:pokemon [followed by friend:pokemon]` | `game` | instant | only in a place's scene the game makes: the player steers him with the arrows (Enter: talk, Shift: run), the camera follows him, the friend follows him; M or X: the game's menu: control, collisions, camera, follow |
| `dialogue` | `dialogue who:pokemon` | `game` | instant | only in a place's scene the game makes: dialogue who … end, what plays when the player talks to him (Enter in front of him), then exploring goes on: ontalk, label, return |
| `explore` | `explore` | `game` | by the line | only in a place's scene the game makes: the player explores, the scene waits until he talks to someone or takes a way: roam |
| `game` | `game words...` | `game` | by the line | only in a place's scene the game makes: the scene holds and the game is told (game talk kecleon, game menu) |
| `way` | `way x:number y:number width:number height:number words...` | `game` | instant | only in a place's scene the game makes: walking into that rectangle (its corner x y, its width and height, as a decor's exit or zone) tells the game (way 600 240 32 16 go port east): door |
| `bite` | `bite [strong\|nibble]` | `fishing` | by the line | a fish takes the bait of the line in the water (the last cast): the recipe bite <who> (strong: yanked under; nibble: it only touches) |
| `cast` | `who:pokemon cast at target:place`; `who:pokemon cast` | `fishing` | by the line | his cast (Swing, on a fisher's variant), the line flies out, the bobber lands on the water there (or before him) |
| `release` | `who:pokemon release [catch:word] [x:number y:number]` | `fishing` | by the line | popup, hop of the catch to the water, particles drops and rings, gone, emote |
| `stairs` | `stairs take *who [floor n:number [up]]`; `stairs [where:place]` | `dungeon` | by the line | the game's stairs laid there (on the right if not said): put, under the bodies; take: the team goes up them, the floor ends |
| `trap` | `trap off`; `trap kind:word on who:pokemon`; `trap who:pokemon kind:word` | `dungeon` | by the line | the game's trap under him: put, appear, sound, battlelog, and what it does |
| `ambush` | `ambush *foes on who:pokemon` | `dungeon` | by the line | battlelog, sound, quake; each: put around him, falls from above in its dust, emote |
| `shop` | `who:pokemon shop buy thing...`; `who:pokemon shop goods...` | `dungeon` | by the line | the items laid out in rows before the shopkeeper: put, thing; battlelog |
| `steal` | `who:pokemon steal thing...` | `dungeon` | by the line | picks up without paying; the shopkeeper: emote, tremble, quake, popup, sound, battlelog |
| `job` | `job take\|done\|fail name:text...` | `guild` | by the line | take: cutaway job_board, narrate, jingle; done: music mission_accomplished, narrate; fail: narrate |
| `put` | `put object what:thing...` | `things` | by the line | put object: find, put, thing |
| `shift` | `shift thing:word [to] where:place [in d:duration]` | `things` | by the line | find, go |
| `open` | `open thing:word` | `things` | by the line | anim, sound |
| `close` | `close thing:word` | `things` | by the line | pose image 1, sound |
| `sound` | `sound name:word off`; `sound name:thing... at d:duration`; `sound name:thing... on who:pokemon`; `sound name:thing...` | `heard` | by the line | sound, movesound, playsound; after (at 12), onhit (on a); [left|right|pan n] [from who] [volume n] [pitch n] [loop] [as name] [in d] |
| `music` | `music off [fade d:duration]`; `music name:word fade d:duration [volume v:number]`; `music name:word volume v:number`; `music name:word from d:duration`; `music name:word [d:duration]` | `heard` | by the line | music, musicfrom (from 12s), fademusic (off fade 2s), a song coming in (fade 2s), its volume; a sound of ours: music own: |
| `jingle` | `jingle name:word` | `heard` | by the line | jingle: a little tune of the game by its name (item, treasure, reward, place, episode, news) or number |
| `frame` | `frame free`; `frame members... snap`; `frame members... [follows]` | `view` | by the line | frame, waitcam; keepframe (follows); camera (snap: at once); a place: pan |
| `glide` | `glide to where:place [in d:duration]`; `glide where:word [d:duration]` | `view` | by the line | pan to someone or a place, slowly at its ends |
| `moment` | `moment title:text...` | `view` | by the line | nothing for the engine: a moment of the scene (moment: title), and one framing alone in it |
| `daytime` | `daytime moment:word [stars] [d:duration]` | `sky` | by the line | light (morning, day, evening, night; dawn); tint (18h30); stars: the stars twinkling (weather sparkles) |
| `status` | `who:pokemon status off`; `who:pokemon status stat:word up\|down`; `who:pokemon status kind:word` | `sky` | by the line | status: the game's status icon over him (sleep, burn, poison, frozen, confused, cringe…), off: none; a stat up or down: the game's own arrows (effect) |
| `weather` | `weather off`; `weather kind:rain\|snow\|sand\|fog\|hail\|sun [heavy]` | `sky` | by the line | weather: the game's weather over the scene (heavy: under a storm's light), off: fair again |
| `effect` | `effect move:thing... from source:place to target:place [in d:duration]`; `effect move:thing... on target:place`; `effect move:thing... at target:place`; `effect who:pokemon [n:number]` | `effects` | by the line | moveeffect, bolt (on a); moveshot (from a to b) |
| `flash` | `flash colour:word at after:duration`; `flash colour:word [d:duration]`; `flash` | `effects` | by the line | flash; after (at 12); alone, white with the game's shock |
| `shake` | `shake off`; `shake [soft\|hard] [d:duration]` | `effects` | instant | shake: the view shakes up and down until shake off (or for so long) |
| `tremble` | `tremble screen strength:number d:duration [weak\|strong]`; `tremble thing... strength:number [d:duration]` | `effects` | by the line | tremble screen: quake |
| `fight` | `fight *side_team vs *others [duel\|double\|relay] [boss] [level] [n:number] [winner wins:pokemon]`; `fight directed *side_team against *others [level] [n:number] [winner wins:pokemon]`; `fight directed rest...`; `fight who:pokemon lv:number hp:number against other:pokemon hp2:number`; `fight who:pokemon lv:number hp:number [floor:word]` | `fights` | by the line | fight open (bars, log); fight directed: fight directed |
| `join` | `who:pokemon join [team]` | `fights` | by the line | emote, jump, the game's sentence |
| `attack` | `who:pokemon attack move:thing... at target:pokemon`; `who:pokemon attack move:thing...` | `fights` | by the line | face, move (the move's own animation and effects, its sound), recoil and the Hurt pose of the target |
| `evolve` | `who:pokemon evolve into:pokemon` | `fights` | by the line | do evolution, the game's sentence |
| `roar` | `who:pokemon roar [hard]` | `fights` | by the line | the game's roar, the view shaking (hard: harder), the sprite's own Roar animation, else its Charge and a rise |
| `charge` | `who:pokemon charge move:thing...`; `who:pokemon charge` | `fights` | by the line | his Charge animation, the game's stats-up shine and its sound, held a beat; a move told: his next attack with it lands with a flash and a hard shake |
| `dodge` | `who:pokemon dodge [left\|right\|back]` | `fights` | by the line | a quick hop aside (back by default) with his Hop animation; right after an attack at him, or right before one, it misses: no knock back, "Missed!" |
| `give` | `from:pokemon give thing... to other:pokemon`; `give thing... from:pokemon to:pokemon` | `items` | by the line | give |
| `hold` | `who:pokemon hold thing...` | `items` | by the line | putnear, thing, hold (or approach and hold if it lies on the ground) |
| `drop` | `who:pokemon drop at what:place`; `who:pokemon drop [thing...]` | `items` | by the line | hold off, fall (at a place: set there, a little hop) |
| `eat` | `who:pokemon eat [thing...]` | `items` | by the line | anim Eat, scale, gone, emote |
| `throw` | `who:pokemon throw line [to\|on] [target:place]`; `who:pokemon throw water [to] target:place`; `who:pokemon throw thing... at target:place [straight]`; `who:pokemon throw thing... [to] target:pokemon [straight]` | `items` | by the line | throw a Apple to b: face, hold off, throw, then catch or miss; `straight`: tight and fast, at the height of the hands (a shot); throw a line to x y (or on b), the fisher; throw a water to b, a swimmer: jump, particles drops |
| `put` | `who:pokemon put [where:place] facing way:word`; `who:pokemon put [where:place]`; `put rest...` | `moving` | by the line | find, put (before the scene) or arrives (in the middle of the scene); facing: turned that way, or to someone |
| `walk` | `who:pokemon walk [to] where:place [slow\|brisk\|slowly\|fast] backward [in time:duration]`; `who:pokemon walk [to] where:place [slow\|brisk\|slowly\|fast] [in time:duration] [arrives arrival:duration]`; `who:pokemon walk [to] where:place [slow\|brisk\|slowly\|fast] via path...`; `who:pokemon walk via path... [slow\|brisk\|slowly\|fast] [in time:duration] [arrives arrival:duration]`; `walk rest...` | `moving` | by the line | find, goes; approach (to someone); leaves (to offleft); slow, brisk: its pace; backward: its back to where it goes, stepping; via a b: by those places first |
| `run` | `who:pokemon run [to] where:place [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `who:pokemon run via path... [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `run rest...` | `moving` | by the line | find, goes; chase (to someone) |
| `enter` | `enter row\|column\|triangle *who [where:place] [look to:word] [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `enter face to face *who [where:place] [look to:word] [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `*who enter from side:word [to where:place] [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `*who enter [via side:word] [to where:place] [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `enter rest...` | `moving` | by the line | find, arrives enter [via:<entrance>] |
| `exit` | `*who exit [via side:word]` | `moving` | by the line | leaves, waitfor, gone |
| `appear` | `appear row\|column\|triangle *who [where:place] [look to:word] [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `appear face to face *who [where:place] [look to:word] [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `who:pokemon appear [where:place] smoke\|fade\|flash\|vision\|light`; `who:pokemon appear [where:place]`; `appear rest...` | `moving` | by the line | find, put; in the game's smoke (by default), a fade, a flash, a vision, or the light that gathers into a ring (light) |
| `face` | `face to face *who [where:place] [look to:word] [slowly\|fast] [in time:duration] [arrives arrival:duration]`; `who:pokemon face other:pokemon` | `moving` | by the line | face to face: find, put or arrives, turn |
| `form` | `form row\|column\|ring\|lineup *who [to] [where:place] [walk\|run] [look looks:word] [slowly\|fast] [in time:duration] [arrives arrival:duration]` | `moving` | by the line | find, put or arrives, turn: side by side (row), one behind the other (column), round the place facing it (ring), side by side facing us (lineup); put there before the scene shows, walked to after; walk, run: walked or run to it, even before |
| `spot` | `spot name:word where:place` | `moving` | by the line | spot throne 400 200: setspot, a spot of the film's, by its name |
| `look` | `who:pokemon look to:word` | `body` | by the line | turn (a direction) or face (someone) |
| `jump` | `jump school x:number y:number [n:number] [left]`; `who:pokemon jump water x:number y:number`; `who:pokemon jump water [rest...]`; `who:pokemon jump wave`; `who:word jump [rest...]` | `body` | by the line | jump; jump a water x y: a fish leaps out of the water; jump school x y [n] [left]: a school; jump a wave: a swimmer over a wave |
| `wait` | `wait sound name:thing...`; `wait music d:duration`; `wait d:duration`; `wait all`; `wait *who`; `wait name:word`; `wait [text...]` | `body` | until | wait 2s: t (at the film's pace); wait a b: waitfor each; wait all: waitall, and what plays beside; wait sound name: sound, waitsound (as long as it lasts); wait music 12s: waituntil music_clock |
| `wander` | `who:pokemon wander [n:number]` | `body` | by the line | roams |
| `memory` | `memory` | `pictures` | by the line | whiteout, a sepia tint, whitein; at its end, the same back |
| `transition` | `transition kind:fade\|white\|boss\|wipe\|iris\|cut in\|out [d:duration]` | `pictures` | by the line | fadeout/fadein, whiteout/whitein, iris, wipe: the screen goes, or comes back, now |
| `banner` | `banner kind:chapter\|place\|dungeon text...` | `pictures` | by the line | typecaption on the dark (chapter, dungeon), caption over the scene (place) |
| `sleep` | `who:pokemon sleep` | `rituals` | by the line | pose sleep, status sleep |
| `morning_call` | `morning_call leader:pokemon *who` | `rituals` | by the line | shout, quake, status off, jump; then rank |
| `dinner` | `dinner *who` | `rituals` | by the line | seats, lookat, anim Eat, emote |
| `rank` | `rank *who`; `rank team:word rank:thing...` | `rituals` | by the line | find, goes, turn down (they line up); a team and a rank: jingle, the game's sentence |
| `crowd` | `crowd species:word n:number [rest...]` | `crowds` | by the line | crowd, fidget: crowd <species> <n> [where: a spot, x y] then, in any order, [radius 40] [tight|dense] [shape ring|flock|line|heap] [from east|west|north|south] [fast] |
| `bubble` | `who:pokemon bubble text...` | `said` | by the line | bubble |
| `choice` | `choice text...` | `said` | by the line | ask |
| `box` | `box top\|bottom` | `said` | by the line | boxside |
| `emote` | `who:pokemon emote kind:word`; `who:word emote kind:word d:duration` | `said` | by the line | emote, sound |
| `at` | `at text...` | `timing` | instant | at name [+ 2s]: …: the line (or the block up to end) played beside, so long after the mark (calc, waituntil clock); at 12s: …: so long after the scene's start (waituntil clock); at music 12s: …: once the music has played so long (waituntil music_clock) |
| `when` | `when *who arrives\|arrive [text...]` | `timing` | instant | when a arrives: …: the line (or the block up to end) played beside the film as soon as a has got where it was going (parallel, waitfor) |
| `mark` | `mark name:word` | `timing` | instant | mark name: a mark at that instant of the scene (set _mark_name clock), for “at name + 2s:” |
| `beside` | `beside [name:word]` | `timing` | instant | beside [name] … end: its lines played beside the film, one after the other (parallel); “wait name” waits for it; a line ended by “& as name” plays in it |
| `pace` | `pace lines d:duration`; `pace lines off`; `pace slow\|brisk\|normal`; `pace n:number` | `timing` | instant | pace slow|brisk|normal or a number: the waits said that follow (wait 2s), longer or shorter, a word's own time its own; pace lines 2s: each line said stays that long, then goes on alone; pace lines off: each as long as it is read |
| `together` | `together` | `timing` | until | together … end: each line starts at the same time (parallel), and the film waits for the longest (waituntil) |
| `prop` | `prop who:word rest:text...` | `nodes` | by the line | prop: prop <who> <property> <value>…, a property of the registry or several on the line (prop a angle 45 scale 1.5 opacity 50%) |
| `node` | `node name:word kind:word [file:word] [x:coordinate y:coordinate]` | `nodes` | by the line | node: node <name> group [x y], node <name> image <file> [x y], node <name> off; a node of one's own, set afterwards by prop |
| `attach` | `who:word attach parent:word [dx:number dy:number] [inherit flags...]` | `nodes` | by the line | attach: attach <who> <parent> [dx dy] [inherit pos angle scale opacity colour]; it follows its parent (turned, grown, faded with it), at dx dy from its position (else where it is) |
| `detach` | `who:word detach` | `nodes` | by the line | detach: detach <who>; it stays where it is |
| `hug` |  | `gestures` | by the line | `hug who:pokemon other:pokemon`:  |
| `whisper` |  | `gestures` | by the line | `whisper who:pokemon other:pokemon`:  |
| `comfort` |  | `gestures` | by the line | `comfort who:pokemon other:pokemon`:  |
| `explain` |  | `gestures` | by the line | `explain who:pokemon`:  |
| `cower` |  | `gestures` | by the line | `cower who:pokemon from other:pokemon`:  |
| `cheer` |  | `gestures` | by the line | `cheer who:pokemon`:  |
| `sigh` |  | `gestures` | by the line | `sigh who:pokemon`:  |
| `react` |  | `gestures` | by the line | `react who:pokemon joy`:  |
| `ghost` |  | `gestures` | by the line | `ghost who:pokemon`: He turns half see-through and floats, as a ghost does. |
| `finish` |  | `fights` | by the line | `finish who:pokemon target:pokemon move...`: The finishing blow: time slowed, the screen white, the move; the one struck faints, and the music falls silent a moment over them. |
| `reel` |  | `fishing/line` | by the line | `reel who:pokemon catch what:word`: He reels in what bit: it flies out of the water and lands beside him. |
<!-- end of words -->

## How it is built

`film/` (a single source of truth: the engine and the films' words are in Rust), one home per subject:

| Folder | Role |
|---|---|
| `text/` | reads the text through the language (`front/`): what each line is (`ir.rs`), a recipe kept as written and written out at each call; recipes, `repeat`, `uses`, `include` played (`recipes.rs`), whom each scene names (`cast.rs`) |
| `names/` | names: someone, a Pokémon, a variant, a crowd (`mod.rs`), an object (`things.rs`), a place, an entrance, a picture, a move (`places.rs`) |
| `spots/` | where a line says: a word of the place, pixels, a tile, `front_of a` (`mod.rs`), getting there and at what pace (`routes.rs`), formations (`formations.rs`) |
| `verbs/` | the words, one file per theme, each with its `WORDS` list (the word, what it writes): `acting`, `fishing/`, `dungeon/`, `things`, `heard`, `view`, `sky`, `effects`, `fights/`, `items`, `moving/`, `pictures`, `rituals`, `crowds`, `said`, `timing`, `nodes`, `game`, `guild`; the engine's commands in `engine.rs` and `commands.rs` |
| `emit/` | the `Beat`s the words make, one file per kind (as in the engine); points found while playing (`Point`, `film.on_point`); what the film says is wrong (`film.not_a`, `film.unread`); `adapter.rs`: engine commands written in a film, read as beats |
| `compile/` | the film's scenes (`scenes.rs`), a scene from its opening to its fade (`scene.rs`), a line (`said.rs`), the whole film and its events (`mod.rs`) |
| `tables/` | the game's files, read once: Pokémon and variants, places and grounds, pictures, objects, what the story's scenes say |
| `ds.rs` | what each word becomes on the DS: the table `scene_player export-ds` writes by, and `--table` prints |
| `words.rs`, `screen.rs`, `check.rs` | the little words (faces, emotions, spots), the words on screen (box, bubble, colours), what is wrong, with the film's line and the closest names |
| `tests/` | what each word gives, where it leaves everyone on the real scene |

What depends on the scene is not decided here: `front_of a`, `to b`, formations, who comes in, crowds, the path, the free tile are written as they are (`find`, `goes`, `arrives`, `crowd`... see `engine/src/beat/place.rs`) and are worked out on the `Stage` when the line plays (`stage/placing/`, `stage/ground.rs`). The same goes for the view: `view.rs` only translates the words into camera beats (`engine/src/stage/camera/`), which frames, picks the distance and glides. A `.film` goes through no text file: `.txt` and the engine's language remain for `story.txt`, `>` lines and packs.

**The view's words** are a single family: `frame` (`snap`, `follows`, `free`, people and a place together) and `glide`.

**When something is badly written**, each problem carries its line of the film, and the words it quotes between “ ” are pointed out in place (the Workshop and `analyze` underline them); an unknown name is always told the same way: `unknown item “aple”. Closest: apple (scene_player items)`.

To add a word, add it to the `WORDS` list of its theme's file in `film/src/verbs/`, with its forms, what it makes play and its function, which receives its values already read and makes its `Beat`s:

```rust
Word::new("greet", &["who:pokemon other:pokemon"], "face, jump").does(greets),

fn greets(film: &mut Film, line: &Line) {
    let (who, other) = (line.args.word("who").unwrap_or_default(), line.args.word("other").unwrap_or_default());
    film.face(who, other);
    film.on(who, |who| emit::jump(who, 6.0, 10.0));
}
```

If its meaning depends on the scene, write a `Spot` or a new `PlaceBeat`, never a guess.

A word whose line brings someone onto the stage in the middle of a scene says so on its own line, by the names of its form: `.brings(&["catch"])` (new, unless the scene already named him: what a rod pulls out of the water), `.puts(&["who"])` (put there: `put`, a formation), `.arrives(&["who"])` (coming in, not there before even if named at the head: `enter`, `appear`); `.when("water")` only when the form read that word (`jump a water`); `.brings_with(names)` for names its function makes of the values (`gulls wingull 3`). Those need no place at the scene's start (`text/cast.rs` binds the line to the word's forms and reads those values; it knows no word by name).
