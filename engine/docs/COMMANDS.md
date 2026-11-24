# Scene commands

Every command on one page: `CHEATSHEET.md`. Here, each one explained, sorted by theme.

## Contents
1. The file
2. Events and scenes
3. Placing, showing, removing
4. Moving
5. Gazes
6. Body and animations
7. Appearance
8. Emotes
9. Speech
10. Choices
11. The view
12. Light, colours, time
13. Sounds and music
14. Numbers and logic
15. Little games: the player in control
16. What scenes do to the game
17. Sequences and shorthand
18. Community Pokémon and animations
19. Tools
20. Commands of the original game

## 1. The file
The commands are written in a film (`.film`, `film/FILMS.md`), each by its name, one line per command, the fields separated by spaces, a text that has spaces in quotes (`say 10 1 "Hello!"`); a film's words make most of them for you. A line that starts with `#` is a comment.

The engine reads no author text: the film's language reads the film and hands it its lines. Only the game's story, taken from the ROM (`data/rom/story.txt`), is kept as tab-separated fields.

**Who** (`<who>`):
- a character by their number (`data/rom/entities.txt`: `0` the hero, `10` the partner…);
- `m<species>` any Pokémon (`monsters.txt` in `data/rom` and `data/extra`: `m4` Charmander, `m133` Eevee…), with its name and its portraits; `m98.1`, `m98.2`… others of the same species, each its own;
- `i<item>` or `i<item>.<n>` a game item lying on the ground (`data/rom/items.txt`);
- `o<prop>` or `o<prop>.<n>` a game prop (`data/rom/props/`: `o66` a round stone slab, flat on the ground); `.1`, `.2`… others of the same (up to 7);
- `p<n>` an invisible point (to look at or walk towards something);
- `target`: the one Enter found last (see 15): `face target hero`, `line target - Thanks!`, `{name:target}`.

**Where**: `<x> <y>` in pixels of the backdrop, or `@spot` (given by `spot`), or `@spot+30-10` next to it (30 to the right, 10 above).

**Directions**: by their name, never a digit (a digit is someone): `down`, `downright`, `right`, `upright`, `up`, `upleft`, `left`, `downleft`.

**Durations**: in seconds, `1.5s`; in frames for the very short, `6f` (60 per second); a number alone is read in frames.

| Line | Effect |
|---|---|
| `cast <name> <who>` | a name for someone: `cast eevee m133`, then `put eevee …`, `line eevee …` |
| `spot <name> <x> <y>` | a name for a place: `spot fountain 300 240`, then `put eevee @fountain down` |
| `group <name> <a,b,c>` | a name for several (a group can contain others): `waitfor <group>`, `each <group> …`, `frame 30 <group>`… |
| `include <file>` | takes in another file (path relative to the scene): `include ../lib/my_sequences.txt` |
| `uses <a,b>` | these numbers are given by another scene of the story (the checker does not report them) |
| `#frames <n>` | for the sheet tool: how many frames to play it at most |
| `#keys <frame>:<key>,…` | for the sheet tool: the keys pressed (`600-640:Right` holds it), for scenes the player plays |

Names hold for the whole file; the words of lines are never touched (except `{name:…}`, see Speech).

**Numbers in commands**: a field that contains `{name}` (anywhere but in the words of a line) is filled in when the line plays: `put rock1 {x} {y} down`, `halo @ {x} {y} 16 black`, `walk pika {cx} 200`. Names can be nested: `set note n{i}`, `{n{i}}` (the innermost first). The lines that give the scene its shape (`if`, `while`, `goto`, `label`, `ask`…) are not filled in: do a `set` before.

## 2. Events and scenes
| Command | Effect |
|---|---|
| `event 0 <NAME> 1 1` | opens an event of the story (a scene of this project: `event 0 EVENT_M01_01 1 1`) |
| `scene <place> <name>` | a new scene, in a place (`data/rom/levels.txt`; tool: `scene_player grid <place>`) |
| `backdrop <place> 1 -` | shows the place's backdrop |
| `end` | end of the scene (and of the answers of a choice) |
| `endask` | in place of a choice's `end`: each answer carries on **after it, in the same scene** (instead of going to the next scene): a question in a loop |
| `gotoscene <name>` | the story carries on in another scene of the same event, by its name |
| `banner <text>` | a title on a black screen (the name of a dungeon) |
| `top <place>` / `topcam <x> <y>` | the picture on the top screen |
| `minimap <who...>` / `minimap off` | the map of the place at the top right, one pixel per walkable tile: those (the team) in yellow, the others in red, the stairs in white, the items on the ground in blue |
| `topimage <file>` / `topimage off` | a picture of your own on the top screen (a 256 x 192 PNG, path from the scene) |

## 3. Placing, showing, removing
| Command | Effect |
|---|---|
| `put <who> <x> <y> <dir>` | puts them there, facing that way |
| `putnear <who> <other> [side]` | right next to the other (to the right by default; `left`, `up`, `down`…), facing them |
| `gone <who> [<who>…]` | removes them from the scene |
| `vanish <who> [frames]` / `appear <who> [frames]` | fades out / fades in (20 frames by default) |
| `opacity <who> <0 to 1> [frames]` | half visible, a quarter… (`opacity <who> 0 0` right after a `put`: invisible at first) |
| `give <item> <from> <to> [frames] [keep]` | an item (`i109` an Apple) passes from one's hands to the other's in an arc, then vanishes; with `keep`, it first shows above the giver and stays in the other's hands |
| `reset <who>` | as they were when put, where they are: their size, their colours, visible, among the others, with their shadow, without a name, doing nothing more |
| `carry <item> <who>` / `carry <item> off` | the item (or someone) is carried above their head and follows them everywhere / put down where it is |
| `ride <who> <mount> [height] [offset]` / `ride <who> off` | sitting on the mount's back, so many pixels above its feet (16), so many to the side: they follow it everywhere, go up and down with it / left where they are |
| `hold <item> <who>` / `hold <item> off` | the item (already placed by `put`) in their hands, in front of them, on the side they look, and it follows them everywhere / dropped where it is |
| `thing <item> name:<name>` | the name of an item of the scene, for `state` and `do` |
| `drawing <item> <file> [frames] [rhythm]` | a picture of ours for `d<n>`, taken from `data/hand/` or `library/images/`, to give before its `put`; `frames` side by side: the first at rest, the next its animation; `rhythm` (`40,2,3`): the sixtieths of a second of each frame, and the animation plays them all, the first included |
| `takeable <item> [off]` | it can be picked up, or not |
| `shine <item> [off]` | a little star of crisp pixels turns around it |
| `large <item>` / `large off` | the item large, crisp, above the scene |
| `fishline <who> <item> [x y]` / `fishline <who> <item> holds` / `fishline <who> off` | a fishing line, from the end of their rod (`tip` in their variant's table) to the item, which flies in an arc to the point, or in front of them; `holds`: only tied to what is already there (a catch, where it comes out of the water); they keep an eye on it |
| `image <name> <file> <x> <y>` / `image <name> off` | a picture of your own (a PNG, path from the scene: `../images/sign.png`) put on a point: a sign, a drawing |
| `disguise <who> <other>` / `disguise <who> off` | they have the other's picture, and their name and face in what they say: an illusion (`disguise fake partner`), an evolution |

## 4. Moving
Movements do not wait: the scene goes on. `waitfor <who>` waits until they have finished, `waitall` until no one moves any more.

| Command | Effect |
|---|---|
| `orbit <who> <other> [laps] [frames]` | turns around the other, who may move |
| `resume <who>` | the one a `stop` halted on the way sets off again towards where they were going |
| `walk <who> <x> <y>` / `run <who> <x> <y>` | walks / runs there, at their pace |
| `step <who> <dx> <dy>` / `runstep …` | a few steps from where they are: `step pika -20 0` |
| `path <who> <x1> <y1> <x2> <y2> …` / `runpath …` | through several points in a row |
| `go <who> <x> <y> <frames> <slide 0/1>` | there in so many frames; with `1` they slide (without walking or turning) |
| `hop <who> <x> <y> <height> <frames>` | a jump in an arc to there |
| `throw <who> <to> [height] [frames]` | thrown in an arc to where the other stands at that moment (24 pixels high, 30 frames) |
| `fall <who> [height] [frames]` | falls from high up to the ground (90 pixels, 18 frames) |
| `speed <who> <pixels per frame>` | their pace (a walker: 1.2; running 2.2 times faster) |
| `approach <who> <other> <frames>` | goes and stands next to the other |
| `chase <who> <other>` | runs to the other, stops next to them and looks at them |
| `mirror <who> <model>` / `mirror <who> off` | copies them like a reflection: walks as far, the other way from left to right, the same way from top to bottom, facing the same; walls and the `bounds` rectangle stop them |
| `flee <who> <from> [distance]` | runs away from the other (48 pixels by default) |
| `lineup <x> <y> <dx> <dy> <who…>` | in a row from the point, spaced by (dx, dy), facing down |
| `keeponly <who…>` | all the others leave the scene at once |
| `crossto <right\|left\|up\|down> <place> <x> <y> <who…>` | going to the next place on foot, in one stroke: they leave by that side while the black comes in from that side, the next place shows (the others stay in the place left; `keeponly <who…>` does the same alone), they come in by the opposite side (the black goes back that way) and walk to the point, one behind the other |
| `ring <x> <y> <radius> <who…>` | put at once in a circle around the point, facing it: a crowd already in place |
| `train <gap> <leader> <who…>` / `train off` | a line: each walks where the leader went, at this gap from the one before (runs if too far behind); a parade, a train |
| `follow <who> <leader> [gap]` / `follow <who> -` | follows them everywhere (stops behind, so many pixels away, 22 otherwise, or at an angle if someone is already there) / stops |
| `cheer <who…\|group> [emote]` | each jumps and shows their joy (or this emote: joy, anger, sweat, tears, surprise, question, shock, sparkle), not all at the same time |
| `boo <who…\|group> [emote]` | each shows their anger (or this emote), without jumping |
| `wander <who> [radius]` | strolls around where they are (24 pixels; `0`: stops); does not count for `waitall` |
| `orbit <who> <x> <y> [laps] [frames]` | turns around a point (1 lap in 120 frames; negative laps: the other way) |
| `walkto <who> <x> <y>` / `runto …` | goes there going around the walls (they find their way) |
| `spacing [on\|off]` | those who walk step aside from those they meet: two who face each other go around each other and still arrive on time; the walls of `wall` and those of the backdrop count. `orbit` and slides do not dodge: they push the others |
| `collisions on\|off` | collisions, until the end of the scene. `off`: each goes straight where they are sent (`walkto`, `path`, entrances and exits), through the place's ground and the `wall`s; no one steps aside (`spacing` no longer plays); a place is taken as written, never pushed onto the free tile; a crowd takes all the room it needs; `check` no longer says anything about walls or bodies on top of each other. The ground still serves to choose where one comes in and goes out. `on`: all that comes back. A film's scenes open without (`off`), those of the game and of `.scene` files with |
| `stop <who>` | stops everything they do: walk, path, lap, stroll, follow, gaze, float, spin |
| `waitfor <who> [<who>…]` / `waitall` | waits until they have finished / until no one moves any more |
| `waitmoves <who>` | waits until they set off: `when a leaves` in a film |
| `waitsound <sound> [pitch n]` | waits as long as this sound lasts: a game sound, `move:<n>`, `own:<name>` (as with `playsound`), played `n` semitones higher (shorter); its length is given to the director by whoever plays the story (`Director::hear_sounds`: the game's sound bank, the moves, our files); unknown, half a second, and `check` remarks on it. `wait sound flash` in a film plays it and waits for it |
| `waitnear <who> <x> <y> [radius]` | waits until they are closer than this radius to the point (16 pixels; `@spot` works) |
| `roam` | the player explores: they steer the one of `control` with the arrows, and the scene waits. Enter in front of someone with `ontalk` plays their passage, then you explore again; at a `door`, you go into its scene. Without a player at the keyboard (`check`, a frame of the film) or sped up, it goes straight on to what follows |
| `ontalk <who> <label>` | when the exploring player talks to them (Enter in front of them, closer than 40 pixels), the passage after this label plays, up to `return` |
| `onmenu <label>` | the menu key (M or X), while exploring, plays the passage after this label, up to `return` |
| `game talk <who>` · `game menu` · `game go <place> [entrance]` · `game zone <name>` | (written by the game in the scenes it makes of its places) asks the game, typed as the line is read (`GameAsk`, handed over by `Call::Game`): a talk and the menu hold the scene until the game is done |
| `team <hero> <friend>` | (written by the word `player`) the game's team, by the species of the hero and the friend (0: no friend): changed, it keeps the level the leader had; the same, it stays what it has become |
| `bag <n> <thing…>` | (written by `stow` and `take_out`) so many of this thing in the game's bag (below 0: taken out): a game item by its name, otherwise a thing of your own; the film gets `bag_<word>` |
| `savegame` | the game is saved: where you explore if you come from exploring (a menu, a dialogue), otherwise here; where the one you steer stands (`Director::saved_at`); a game taken up again (`Author::restore`, `restore_at`) puts them back there |
| `door <scene> <arrival> <x> <y> [radius] [width height]` | while exploring, closer than this radius to the point (20 pixels), or inside the rectangle of that width and height round it, you go into this scene, by the way `<arrival>` (`Director::arrival`; a scene `game …` tells the game instead); you do not go back through the door you came from before you have left it |

## 5. Gazes
| Command | Effect |
|---|---|
| `turn <who> <dir>` | turns |
| `face <who> <other>` | turns towards the other |
| `turnaround <who>` | turns right around |
| `turnby <who> <eighths>` | turns so many eighths from where they look, at once (counter-clockwise, clockwise below 0): the game's look around and head shake |
| `turnslow <who> [frames]` | their next turn goes the slow way, through each way in between, one every so many frames (6 otherwise), as the game's slow turns |
| `lookat <who> <x> <y>` | turns towards a point (`lookat eevee @fountain`) |
| `fidget <who> [on\|off]` | looks around from time to time (a lively crowd); does not count for `waitall` |
| `spin <who> <laps> <frames> [rotate]` | spins on the spot: through their eight directions; `rotate`: their picture itself turns, their direction kept |
| `views <who> <dir…>` / `views <who> off` | the only directions they are drawn in (a variant, `views down` in its table): when stopped, they go back to it on their own; lines do not turn them; a `turn`, `face`, `lookat` in the film turns them until their next walk or `pose <who> idle`; `film/` writes it on its own |

## 6. Body and animations
| Command | Effect |
|---|---|
| `jump <who> [height] [frames]` | a jump on the spot (10 pixels, 12 frames) |
| `bounce <who> [height] [frames]` | hops without stopping (4 pixels, 16 frames; `0`: stops); does not count for `waitall` |
| `float <who> [height] [frames]` | floats up and down (4 pixels, 60 frames; `0`: stops); does not count for `waitall` |
| `rise <who> <height> <frames>` | rises (or comes back down) above the ground |
| `tremble <who> [strength] <frames>` | trembles, by so many pixels (1) |
| `recoil <who> <from> [distance]` | takes a blow from the other: backs off (16 pixels), is hurt, trembles, turns towards them |
| `anim <who> <name> [loop] [frozen]` | an animation of their sprite, by its name (table below) |
| `anim <who> <name> <n> times` | this animation n times, then they stand as before |
| `pose <who> <name> image <n>` | this frame of the animation (the first: 1), and they stay on it |
| `slowmo <who> <times\|off>` (`slowmo a 0.3`) | their own time alone, so many times slower (or faster): their steps, their jumps, their laps, their frames, what shows on them; the rest of the scene on the scene's time (`tempo` multiplies them). What waits for them (`waitfor`) waits for their slowed time. `off` (or `1`): like the scene |
| `still <who>`, `unfreeze <who>` | they stay as they are, mid-gesture, and no longer count for `waitall`; then they go on |
| `height <who> <height> [frames]` | at this height above the ground, at once or in so many frames |
| `infront <who> <other\|off>` | drawn in front of the other, wherever they are |
| `move <who> <move> [<target>]` | uses a move (`data/rom/moves.txt`) as in a dungeon: its gesture, its effects, its sound, the target takes it |

Animations by their name, as SpriteCollab names them:

| Name | Name | Name | Name |
|---|---|---|---|
| `Walk` | `Attack` | `Sleep` | `Hurt` |
| `Idle` | `Swing` | `Double` | `Hop` |
| `Charge` | `Rotate` | `EventSleep` | `Wake` |
| `Eat` | `Vise` | `Tumble` | `Pull` |
| `Float` | `Nod` | `Sit` | `LookUp` |
| `Sink` | `Trip` | `Laying` | `LeapForth` |
| `Head` | `Cringe` | `LostBalance` | `TumbleBack` |
| `Faint` | `HitGround` | `Shoot` | `FireRifle` |
| `Pose` | `Pain` | `DeepBreath` | |

An animation a Pokémon does not have: they stay as they are (see 18 to add some).

## 7. Appearance
| Command | Effect |
|---|---|
| `scale <who> <size> [frames]` | grows or shrinks (`scale pika 2 30`; 1: their size) |
| `colour <who> <r> <g> <b> [frames]` | tints them (out of 255: `colour pika 255 120 120` reddens) |
| `uncolour <who> [frames]` | their colours |
| `color ...` / `uncolor ...` | `colour` and `uncolour`, other spelling |
| `layer <who> over\|under\|normal` | drawn over everyone, under, or among the others |
| `blink <who> on\|off` | blinks |
| `shadow <who> off\|on` | without a shadow (someone who flies high) / with one |
| `sink <who> <pixels> [sand]` | sunk in the water by so many pixels: drawn lower, nothing of them below the surface (the line of their feet), those on their back with them; `sand`: in sand, the surface line sand-coloured; 0: out of the water |
| `flip <who> on\|off` | their picture flipped from left to right (a game item painted facing one way only: a wake) / put back |
| `bar <who> <0 to 1>` / `bar <who> off` | a little health bar above them: what they have left |
| `prop <node> <property> <value>` | a property of any node, at once: `prop m25 angle 45`, `prop m25 opacity 50%`, `prop m25 tint #ff8800`, `prop m25 position @m7+10,-4`, `prop m25 scale x1.5`, `prop m25 angle +30`, `prop m25 angle default`. The node: someone (`m25`, `o66.3`), `node:<name>` (a picture, a node made by `node`), `caption`, `halo:<rank>`, `emitter:<who>`. The properties, their types, units, defaults and advised ranges: the "Properties" table of `film/FILMS.md` (the registry, `engine/src/props/`); a value outside the advised range plays as is, with an `R-HORS-PLAGE` remark; an impossible one (a negative size) is an `E-VALEUR-IMPOSSIBLE` error |
| `attach <node> <parent> [dx dy] [inherit pos angle scale opacity colour]` | the node hangs from its parent: it goes where the parent puts it, at dx dy from its position in its turned and scaled frame (without dx dy: where it is), its pivot kept; it takes from it what `inherit` says (without: everything); a parent's parent counts too; neither the caption nor an emitter; a body carried or on a back is let go; a body attached to a body is drawn just in front of it (without a `z` of its own) |
| `detach <node>` | it no longer hangs from anything and stays where it is |
| `track <node> <property> [loop\|pingpong] [relative] [exact] / <t> <value> <curve> / …` | a track: the property goes through its keys (any animatable property of the registry). `<t>`: so many frames after the start (`120`), `music:720` (when the music has played so much), `mark:<name>+30` (so much after `set _mark_<name> clock`); a key already passed: at once. The first key after 0: the track starts from what the property is. `<value>` as for `prop`; `+30`: added to the key before; `relative`: each key added to what it was at the start. `<curve>`: how it comes to this key (`linear`, `smooth`, `in_quad`, `out_back`, `inout_sine`, `steps(4)`, `bezier(0.4,0,0.2,1)`; the list in `film/FILMS.md`). An angle takes the shortest way, unless `exact` or a relative key. `loop`: it starts again; `pingpong`: it comes back, and sets off again. A new track of the same property of the same node takes the other's place. The node's time counts (`slowmo`), and `tempo` |
| `trackpath <node> <frames> <lines\|catmull\|bezier> <curve> [loop\|pingpong] [orient] [relative] / x y / x y …` | the node follows these points at an even speed (by the length of the path), in so many frames, by this curve: in straight lines, in a Catmull-Rom curve through all the points, or in one Bézier curve whose controls they are; `orient`: turned the way it goes (a body: it looks that way); `relative`: the points from where it is |
| `untrack <node> [property]` | its tracks stop (only that property's): it stays as it is |
| `waittrack <node>` | waits until its tracks that end have ended (a looping track does not end: it is not waited for); `waitfor <who>` waits for their tracks too |
| `sheet <who> <file> <W>x<H> <1\|4\|8> [anim:cells:rhythm:row ...]` | a character of your own drawn from a sheet (`character`): cells of W×H, one row per direction (1: all the same; 4: down, left, right, up; 8: down, downright, right, upright, up, upleft, left, downleft); each animation: its number (0 walk, 1 idle, 2 hurt, 3 sleep, 4 attack, 5 + n the n-th of your own), its cells on the row (`0,1,2`), their sixtieths (`6,6,8`, `-`: 8 each), the row of its first direction |
| `node <name> group [x y]` / `node <name> picture <file> [x y]` / `node <name> off` | a node of your own: a group, just its place, to hang other nodes on; a picture from a file, put there; `off`: removes it |

`scale`, `sink` and `slowmo` take any positive number, with no ceiling (beyond what the registry advises: a remark); a colour has each part from 0 to 255.

A floating Pokémon (Celebi…) shows well above its place: keep it low enough in the view.

## 8. Emotes
| Command | Effect |
|---|---|
| `emote <who> <name> [frames] [plus]` | `!` `?` `...` `sweat` `anger` `joy` `tears` `shock` `sparkle` (`none`: removes them); with a duration, goes away on its own; `plus`: with those they already show, instead of replacing them (as many as you like) |
| `status <who> <name> [plus]` / `status <who> off` | the dungeon status icon above them, until `off`, in place of theirs, or with them (`plus`): `sleep`, `burn`, `poison`, `toxic`, `curse`, `frozen` (in its block of ice), `confused`, `cringe`, `protect`, `safeguard`, `reflect`, `light_screen`, `endure`, `low_hp`, `lowered`, `sleepless`, `taunt`, `encore`, `embargo`, `sure_shot`, `set_damage`, `focus_energy`, `cross_eyed`, `grudge`, `heal_block` |
| `popup <who> <colour> <words>` | words that rise from them and go away: `-12`, `Frozen!` |

The library's sequences (17) play whole emotions: `do joy eevee`, `do fear pika`…

## 9. Speech
| Command | Effect |
|---|---|
| `line <who> <face> <text>` | a line, with its portrait, waited for (Enter) |
| `line <who> <face> <page 1>\|\|<page 2>\|\|…` | a speech in several boxes; a page can change face: `\|\|[angry] Enough!` (the next ones keep it) |
| `narrate <text>` | a centred narration, without a portrait, waited for |
| `oneof <who> <face> <a\|b\|c>` | one of the lines, at random |
| `say <who> <face> <text>` then `said` | the line, and further on the wait until it is read (the scene goes on between the two) |
| `line <who>:<name> …` | another name shown: `line eevee:??? - Who goes there?`; a voice without a portrait: `line -:A voice - Psst!`; someone else's name: `line fake:{name:partner} - …` |
| `bubble <who> <frames\|-> <text>` | a bubble above their head (the scene goes on); `-`: the time to read it |
| `think …` / `shout …` | a thought (little bubbles) / a shout (spiky bubble) |
| `chat <who> <words> <who> <words> …` | bubbles one after the other: background chatter |
| `caption <frames> <text>` / `typecaption …` | a caption at the top of the screen / that types itself letter by letter |
| `waitbubbles` | waits until there are no bubbles left |
| `timed <frames>` | the previous line goes by on its own |
| `mood <who> <face>` / `mood <who> -` | the face of their lines that give none (`-`) |
| `voice <who> <sound>` / `voice <who> -` | the sound of their lines as they type |
| `portrait <who> right\|left` | their portrait on the right of the box (the one who answers), or on the left |
| `textspeed <times>` / `textspeed normal` | the following lines type so many times faster (`0.4` slowly, `3` fast) |
| `autoface [on\|off]` | the speaker and the one before turn towards each other |
| `holdgaze <who> [on\|off]` | lines no longer turn them, they keep their gaze; `turn` and `lookat` still turn them |
| `audience <who…>` / `audience off` | those look at the speaker, at each line |
| `react [on\|off]` | those in the `audience` answer the speaker's faces, one in two and in turn: the same emote, and a little jump at surprise or joy; films do not turn it on |
| `keepspeaker [on\|off]` | the view slides to the speaker, only if it does not show them |

**In the words**: `*Apple*` in colour (what matters); `{number}` its value (`You have {berries} berries!`); `{name:<who>}` their name (`Thanks, {name:zorua}!`; the hero and the partner with the name the player gave); `{item:109}` the name of an item ("Apple"); `[W:30]` a pause in the middle of the line, the time to type 30 letters, in silence (`I... [W:40]I like you.`); `\n` a new line. Also in bubbles, captions, choices and `hud` (except pauses).

**Faces**, by their name or their number (not every Pokémon has every face):

| Name | Face | Name | Face |
|---|---|---|---|
| `normal` | 0 | `happy` | 1 |
| `pain` | 2 | `angry` | 3 |
| `worried` | 4 | `sad` | 5 |
| `crying` | 6 | `shouting` | 7 |
| `teary_eyed` | 8 | `determined` | 9 |
| `joyous` | 10 | `inspired` | 11 |
| `surprised` | 12 | `dizzy` | 13 |
| `sigh` | 16 | `stunned` | 17 |

With `mood` and `autoface`, a whole conversation is written with `line <who> - <text>`: each turns, reacts and is shown on their own. A face shows: `angry` anger, `worried` sweat, `sad` / `crying` / `teary_eyed` tears, `happy` / `joyous` notes, `surprised` "!", `dizzy` "?", `sigh` "...", `stunned` the shock.

The bubble style is chosen in the game's options; bubbles that would overlap move apart; a bubble of someone out of view is not shown.

## 10. Choices
```
ask	Yes|No|[if item_109 > 0]Give an Apple
answer	0
…
answer	1
…
answer	2
takeitem	109
…
end
```
- Each answer runs up to the next; a single `end` closes them all, then the next scene begins.
- `question <who> <face> <text> <a|b|c>`: the line and the choice together, the question staying shown above the answers (like the game); then the `answer`s as for `ask`.
- `answer` keeps the place of the answer taken (0 for the first), for later: `if answer == 1`.
- `[if <condition>]` in front of an answer: offered only if it is true (the condition as after `if`). Each `answer` keeps its number, and `answer` the place among all.

## 11. The view
| Command | Effect |
|---|---|
| `camera <who>` / `camera @ <x> <y>` | the view on someone (and follows them) / on a point, at once |
| `pan <who> <frames>` / `pan @ <x> <y> <frames>` | the view slides there smoothly |
| `ease smooth\|linear\|out` | how the `pan`s that start afterwards slide: slowly at the ends (smooth, like the game), at an even pace, or fast then slowing down |
| `zoom <closeness> <frames>` | brings the view closer (2: twice as big; 1: normal). The original game shows its scenes without zoom, and all those in this folder are at 1 |
| `boxroom [on\|off]` | the view moves away from the dialogue box when a line shows, and stays away between two lines; `frame` and `keepframe` do it on their own |
| `pan <who\|@ x y> <frames> fixed` | slides and stays there, without following the one it was going to |
| `frame <frames> <who…> [max:0.5]` | the view slides and shows them all, bodies included (see `body`), with a margin; it moves back if it must, never closer than 1 (never zooming: the view only slides); `max:0.5`: never closer than this zoom |
| `keepframe <who…>` / `keepframe off` | keeps them all in view continuously, wherever they go; the view moves back if their bodies do not fit, never closer than 1 |
| `body <who> <wide> <high>` | the size of a body on screen, in pixels: `frame` and `keepframe` make it fit (`film/` writes it on its own for the big ones) |
| `glance <who> [frames] [wait]` / `glance @ <x> <y> …` | the view goes to take a look, stays there (60 frames), then comes back |
| `shake <axis> <strength> [frames]` | a shake (strength 0: stop; with a duration, stops on its own) |
| `quake <strength> [frames]` | an earthquake that dies down (40 frames) |
| `letterbox 1` / `letterbox 0` | black cinema bars |
| `iris out [frames] [who]` / `iris in …` | the circle that closes onto black (onto someone), then opens again |
| `waitcam` | waits until the view no longer moves |

## 12. Light, colours, time
| Command | Effect |
|---|---|
| `fadein <frames>` / `fadeout <frames>` | fade from / to black |
| `whitein <frames>` / `whiteout <frames>` | fade from / to white |
| `light <ambience> [frames]` | `normal` (`day`), `dawn`, `evening`, `night`, `cave`, `storm`, `dream`, `fire` |
| `tint <r> <g> <b> <strength> <frames>` / `tint <colour> <strength> <frames>` | tints the light (strength out of 255; 0: normal) |
| `flash <colour> [frames]` | a colour by its name (below); or `flash <r> <g> <b> <frames>` |
| `weather <name>` | `none`, `rain`, `snow`, `hail`, `sand`, `mist`, `sun`, `petals`, `sparkles`, `leaves` |
| `spotlight <who> [radius]` / `spotlight @ <x> <y> [radius]` / `spotlight off` | everything darkens except a circle of light, which follows the person |
| `spotlight plus <who\|@ x y> [radius]` | another hole of light in the dark (crystals, lanterns); `spotlight off` puts them all out |
| `halo <who> [radius] [colour]` / `halo @ <x> <y> [radius] [colour]` | a soft glow around someone (which it follows, even if they rise) or on a point (24 pixels, orange by default); several at once: lanterns at night; `halo <who> off` puts it out, `halo off` all of them |
| `lightning <who\|@ x y>` | the engine's command behind `bolt` |
| `textspeed <times>` / `textspeed normal` | see Speech (9) |
| `boxside top\|bottom\|auto` | the dialogue box kept at the top of the screen, its portrait below, or at the bottom; auto: where it hides no speaker, as the game has it (at the foot, at the top for a line of one standing low) |
| `particles <kind> <who\|@ x y> [number] [colour]` | `firework` (bursts into colours and falls back), `sparks` (spurt out), `fireflies` (float and blink around), `embers` (rise), `smoke`, `bubbles`, `dust` (falls), `drops` (thrown out), `healing` (a glow that rises); 40 by default |
| `t <duration>` / `wait <duration>` | waits (`t 30`: 30 frames; `t 1.5s`); `t 1s~3s`: a time drawn at random between the two (a crowd that does not move in step) |
| `tempo <speed>` | the scene's time slower or faster (`0.4` slowed; `1` normal) |
| `wipe out\|in <left\|right\|up\|down> [time]` | the black comes in from one side and covers the screen (`out`), or withdraws (`in`) (`wipe close` / `open` in a film): to go from one place to another |
| `credits <time> <lines>` | credits that rise from the bottom to the top of the screen during this time, lines separated by `\n` |
| `moveeffect <who> <move>` | what a move shows on arrival, on who, without a blow |
| `moveshot <from> <to> <move> [frames]` | what it throws, from one to the other, without a blow |
| `emit <kind> <who> [per second] [colour] [time]`, `emit off <who>` | crisp particles come out of them, wherever they go |
| `bolt <who\|@ x y>` | a lightning bolt strikes there, zigzagging from the top of the screen, with its flash and its thunder a moment later |
| `wave <strength> [time]` | the screen ripples (from 0 to 1; `wave 0`: sharp again): a dream, a vision |
| `still on\|off` | everyone stops mid-gesture: time has stopped |
| `battlelog <frames> <text>` | a line of the battle log, at the bottom of the screen (the last three stay) |
| `cutaway [over] <place> <x> <y> [time]` / `cutaway off` | a picture in place of the scene, which goes on behind; `over`: only its light passes over the scene; `time`: it fades in over the previous one |
| `cutaway pan <x> <y> <time>` | the view slides over the picture shown |
| `grey on\|off` | the screen loses its colours: a vision, a memory |
| `crossfade <place> [time]` | the backdrop of another place comes over in a cross-fade, each staying there |
| `waitfade` | waits for the end of a fade, a light, an iris, a wipe, credits |

Colours by their name (for `flash`, `tint`, `halo`, `particles`, `gauge`): `white`, `black`, `grey`, `red`, `pink`, `orange`, `yellow`, `gold`, `green`, `turquoise`, `cyan`, `blue`, `purple`, `brown`, `sand`.

## 13. Sounds and music
| Command | Effect |
|---|---|
| `music <name\|n\|->` | the music (`-`: silence) |
| `music <name> <time>` | the music comes in smoothly, from silence to its volume, during this time (`music memory 3s`) |
| `music back` | the music from before comes back |
| `musicfrom <name\|n> <time>` | the music, from this moment of the song |
| `volume <0 to 1>` | the music's volume |
| `movesound <move>` | the sound of a move, alone |
| `onhit <who> <sound ...\|movesound ...>` | this sound, at the next blow who takes |
| `musicspeed <percent>` | the music faster or slower (`120`: a fifth faster, higher; `100`: as it is) |
| `ambient <name\|n\|->` | an ambient sound |
| `fademusic <0\|1\|2> <frames>` | fades out the music, the ambience, or both |
| `sound <name\|n>` / `jingle <n>` | a sound effect (by its name, below, or its number) / a little tune |
| `sound <name\|n> <who>` | a sound effect coming from someone: quieter the further they are from what the view looks at, and from the side they are on screen (left, right) |
| `playsound <sound> [volume v] [pan p] [from <who> [near]] [pitch n] [loop] [as <name>] [rise <time>]` | a sound played the way you say. `<sound>`: a game sound (name or number), `move:<n>` (a move's), `song:<n>` (a game song, for an ambience), `own:<name>` (a file of ours, `library/sounds/<name>.wav`). `volume` (1: as is, up to 4), `pan` (-1 all the way left, 1 all the way right), `from <who>` (from the side they are on screen, quieter the further they are; `near`: without fading), `pitch` (semitones, higher and faster), `loop` (endless, until its `fadesound … stop`), `as <name>` (its name, to change or stop it; another of the same name stops), `rise` (it rises from silence during this time). As many as you like at the same time |
| `fadesound <name> <volume> <time> [stop]` | a named sound or ambience (`as`) goes to this volume during this time; `stop`: it stops once there (`fadesound rain 0 2s stop`) |
| `music own:<name> [<time>]`, `musicfrom own:<name> <time>` | a file of ours as music, smoothly during this time; from this moment |
| `voice <who> <name\|n>` | the sound of their lines as they type (see Speech) |
| `note <c\|d\|e\|f\|g\|a\|b>[2] [sound]` | a note: the sound (the click by default) played higher, by so many semitones (`c2`: the octave above; or a number of semitones) |

The game's 193 songs and ambiences each have a name: `scene_player songs` gives them all, with their number, their original title and their moods; `scene_player songs sad` (or `battle`, `calm`, `party`, `tension`, `fear`, `mystery`, `emotion`, `epic`, `funny`, `victory`, `night`, `sea`, `dungeon`, `forest`, `snow`...) those of that mood. The most common: `peace`, `evening_beach`, `guild`, `guild_party`, `treasure_town`, `upbeat`, `worry`, `battle`, `memory`, `compassion`, `comfort`, `unease`, `rising_fear`, `impossible`, `morning_sun`, `end_of_day`, `good_night`. The ambient ones (`scene_player songs ambient`) are lasting sounds, to play with `ambient`: `sea`, `rain`, `storm`, `wind_1`, `fire_1`, `waterfall`, `rumble`, `hot_water`...

Sound effects by their name (`scene_player sounds [word]` gives them, with when the game plays them). The game names none of them: each is named after what its scenes show with it; the others play by their number (`sound 8964`). To hear one, or a song: `scene_player listen <word|number>` writes `target/sounds/<word>.wav`.

| Name | When | Name | When |
|---|---|---|---|
| `exclamation` | "!" | `question` | "?" |
| `sweat` | drops of sweat | `drop` | a drop of embarrassment |
| `shock` | shocked | `stunned` | stunned |
| `startle` | "!" with a start | `alert` | an alert "!" |
| `anger` | a jump of anger | `annoyed` | annoyed |
| `hop` | a jump of surprise | `flash` | a white flash |
| `chirp` | Chatot's cry | `click` | a line read, a choice |
| `quake` | the earth shakes | `jolt` | a blow that shakes the view |
| `roar` | a great roar | `light` | a white light that floods everything |
| `vision` | a vision begins | `darkness` | everything goes black |
| `fall` | they fall on their bottom | `rocks` | falling rocks |
| `confetti` | the party | `laugh` | someone laughs |
| `joy` | amazed | `stomach` | a rumbling stomach |
| `mealtime` | the call to dinner | `announce` | an announcement by the narrator |
| `lookout` | the lookout at the Guild's entrance | `item` | an item shown, given or taken |
| `puzzled` | "?" while looking around | `splash` | a dive into water |
| `torrent` | a rushing torrent | `amazed` | amazed, with a start |
| `eureka` | they found it | `strike` | a blow struck |
| `impact` | a big crash | `wings` | wings flapping |
| `map` | a map being unfolded | `boss` | a boss bursts out |
| `tremor` | the ground trembles | `snicker` | a snicker |
| `hooray` | everyone cheers | `wake_up` | a rousing wake-up |
| `tumble` | they fall from high up | `aura` | an aura that spreads |
| `water_jet` | a jet of water | | |

The library's sequences (`surprise`, `fear`, `anger`, `question`) play their own.

## 14. Numbers and logic
Numbers by name, kept in the story's save (a choice in chapter 2 counts in chapter 5); 0 as long as nothing was given to them. They are decimal: `set x 0.5`, `calc m 7 / 2` is 3.5; in a text, `{m}` is written `3.5`, and a whole number without a decimal (`3`).

| Command | Effect |
|---|---|
| `set <name> <value>` | a value (a number, or another name) |
| `add <name> [value]` | adds (1 by default; `-1` takes away) |
| `random <name> <n>` / `random <name> <a> <b>` | at random from 0 to n-1 / from a to b |
| `calc <name> <a> <op> <b>` | `+` `-` `*` `/` (decimal: 7 / 2 is 3.5) `div` (whole: 7 div 2 is 3) `%` (the remainder) `min` `max` (a division by 0 gives 0) |
| `measure <name> <a> <b>` | the distance between the two, in pixels |
| `position <who> <x> <y>` | two numbers take where they stand |
| `list <name> <a,b,c>` | a list: `name1`, `name2`, `name3`, and `name_count` (how many); read by its rank with `{name{i}}` or `set x name{i}` |
| `sees <name> <watcher> <who> [distance] [angle]` | 1 if the watcher sees them: close enough (64 pixels), in front of them in this cone (90 degrees), with no wall between them |
| `settext <name> <words>` | words kept under this name, put into texts by `{name}`; kept in the save |
| `if <condition>` … `elif <condition>` … `else` … `endif` | the first true case is played |
| `switch <name>` / `case <v>[,<v>…]` / `default` / `endswitch` | one case per value |
| `when <condition> <scene>` … `otherwise <scene>` | a table of what comes next: the story carries on in the scene of the first true condition, otherwise in the last: several endings |
| `once <name>` … `endonce` | played the first time only; the number remembers it |
| `while <condition>` … `endwhile` | played again as long as it is true; `break` leaves it, `continue` goes back to the start |
| `waituntil <condition>` | the scene waits until it is true |
| `label <label>` / `goto <label>` | a label / going there |
| `parallel <label>` / `parallel off` | plays this passage (up to its `return`) **at the same time** as the scene / stops them all. Its lines (`line`) go away on their own after a moment, and let the player move: comments during a trial |
| `filmline <n>` / `filmline <n> until <who>` / `filmline <n> for <time>` / `filmline <n> track <node>` / `filmline <n> again` | line n of the film starts here, on the course where it plays; it ends when the next one of its course starts (or its course ends), when who has arrived, after so much time, when the node's tracks have ended; `again`: the current line goes on (its number said again, `_line`). Written by films, for `timing` and the Workshop: you do not write it by hand |
| `after <time> <command>` | the command later, without blocking: `after 30 emote eevee !`; `after 1s~3s …`: after a time drawn at random (murmurs in a crowd) |
| `timer <time>` / `timer off` | a countdown: `timer` gives the seconds left, 0 at the end (`hud {timer} s`, `waituntil timer == 0`); as many as you like, named: `timer bomb 30s` gives `timer_bomb` (`timer(bomb)` in a film), `timer bomb off` |
| `every <time> <command>` / `every off` | the command every so many frames or seconds, in the background; `every 3s~6s …`: at intervals drawn at random |
| `log <text>` | writes to the console (`{name}` replaced): for debugging |

**Conditions**: `<name> == != < <= > >= <value>`, or `<name>` alone (other than 0), joined by `and` (all) and `or` (one or the other; `and` goes first). With tabs or spaces. Someone is compared by their name: `if target == eevee`. A number must not be called `and` or `or` (the checker reports it). Words kept (`settext`) are compared to a word (even a character's name: `if answer == raichu` compares the word), ignoring capitals, accents and the spaces around: `if answer == pikachu` (`_` for a space: `== treasure_town`).

**Parallel passages** have their own waits, `if`, `while` and `goto` (a merchant pacing up and down while two characters talk); they can make bubbles, but not lines or choices (they stop there).

**An event's numbers**: a number whose name starts with `_` (`_i`, `_tries`) belongs only to its event: it disappears when the story moves on to the next one and is never saved. For counters, loops, anything another scene does not need to read; the other numbers are kept by the whole story.

**Numbers given by the game**, always up to date (not saved): `money`, `items` (in the bag), `item_<n>` (how many of item n), `hero_species`, `partner_species` (their species; also `hero`, `partner`, but these names most often stand for the characters, given by `cast`), `recruits`, `recruit_<species>`, `answer`, `timer`, and for the player in control `presses`, `talking`, `target`, `controlled` (who they steer: `if controlled == partner`); `clock` (the frames since the start of the scene), `music_clock` (the frames of the music playing, since its `music` line, from its start `musicfrom`, at its speed `musicspeed`; a new music resets it to zero; absent before any music, very large once the music is cut), `press_clock` (when Enter was pressed), `arrows`, `arrow` (1 up, 2 down, 3 left, 4 right), `arrow_clock` (with `listen`).

## 15. Little games: the player in control
| Command | Effect |
|---|---|
| `control <who>` / `control off` | the player makes them walk with the arrows (Shift: running), while the scene goes on |
| `bounds <left> <top> <right> <bottom>` / `bounds off` | they stay in this rectangle |
| `swapcontrol <who…>` / `swapcontrol off` | the player can switch from one to the other with **Tab** (the first is steered, unless one of them already is); `controlled` says which |
| `listen on\|off` | Enter and the arrows count without anyone being steered: a game of keys (`presses`, `press_clock`, `arrows`, `arrow`, `arrow_clock`) |
| `wall <left> <top> <right> <bottom>` / `wall off` | a wall of the place: they cannot go into it and slide along it; the gaze (`sees`) does not pass through it; those who flee (`avoid`) and those who look for their way (`walkto`) go around it (several at once; they hold for the place: a new backdrop clears them, like `bounds` and the `image`s) |
| `wall <name> <left> <top> <right> <bottom>` / `wall <name> off` | a wall that has a name: only it is removed, a door that opens (`wall off` removes them all) |
| `gauge [left\|middle\|right] <number> <max> [colour]` / `gauge [place] off` | a bar under the words of this place, filled in proportion to the number (the max: a number or a name; green by default): hit points, flames that fade |
| `hud [left\|middle\|right] <text>` / `hud [place] off` | words kept at the top of the screen (on the right if you do not say where), their numbers always up to date: `hud left Lives: {lives}` and `hud Score: {score}` at the same time |

When the player presses Enter: `presses` goes up by 1; the nearest one in front of them (on the side they look, 40 pixels away at most) is looked at: `talking` is 1 if there is someone or an item, and `target` says who. Then `target` is also used as someone: `face target hero`, `stop target`, `line target - Hello!`, `{name:target}` — a single line for all those that can be found. A line or a choice takes the keys back while it is read.

```
control	pika
hud	Found: {found}/3
set	talking	0
waituntil	talking
if	target == eevee
line	eevee	joyous	 You found me!
endif
```
An invisible point (`p<n>`) put somewhere is also found with Enter: `put p2 @sign down`, then `if target == p2` to read a sign (with `image` to show it).


## 16. What scenes do to the game
| Command | Effect |
|---|---|
| `getitem <item> [number]` | the team receives the item (`getitem 109` or `i109`), in the bag, otherwise in storage; `getitem 109 3`: three; `getitem 109 amount`: as many as this number |
| `takeitem <item> [number]` | the team gives one (from the bag, otherwise from storage), or so many |
| `getmoney <sum\|name>` | money (`-50`: loses some; `getmoney price`: as much as this number; `getmoney -price`: loses it) |
| `recruit <who> [level]` | they join the team as a recruit, with their name (level 5) |
| `dungeon <n>` | the team sets off for a dungeon; the story resumes afterwards |
| `ground <place>` | the story carries on in a place where you walk |
| `menu <n>` | one of the game's menus |
| `fight <sort> ...` | a fight played by the game, the scene waiting (written by `film/`: `your_turn`, `alone`, `double`, `tournament`, `brawl`, `directed`: an announced fight, with `hp <who> <n>`, `turns <n>`, `rest`, and its turns `act <who> <move> [<targets> \| all] [misses \| critical \| ko \| protect \| -30] [slow] [slowmo <frames>] [intercepts <who>]`, `uses`, `throws`, `pause <frames>`, `note <text>`, `scene <label>`, `change <who> <who>`, `reserve`, `great_bars`; or a line written by hand: `open`, `blow`, `stat`, `ko`, `combo`, `recruit`, `berry`, `rage`, `end`. Its other parts: `level`, `seed`, `place`, `side <who...>` (twice), `winner`, `keep`, `lv <who> <n>`, `knows <who> <moves...>`, `species <who> <n>`, `reviver`, `support`, `audience`, `present`, `when <who> <hp> <label>`, `lead`, `bar <who> <most>`, `floor`, `power`, `takes`, `heals`, `crit`, `miss`, `times`, `ailment burn\|freeze\|paralyze\|sleep\|poison\|confuse`, `together`, `up\|down <stat>`. `scenes/src/fight`); it leaves `hp`, `hp2`, `champion`, `summary_hp` |
| `checkpoint <label>` | the story is saved there; reopened, the scene plays at once up to the `label <label>` and resumes (put the `checkpoint` just before its `label`) |

The town's residents play their conversations like scenes: they share the story's numbers and can give items, money and recruits.

## 17. Sequences and shorthand
**Sequences**: lines defined once, played everywhere, with values to fill in (`$1`, `$2`…; `$2?down`: `down` if none is given).

```
define	greet
face	$1	$2
jump	$1	6	8
enddef

do	greet	eevee	pika
```

**A sequence's numbers**: `%name` in a sequence is a number of its own, its own for each call (two calls, or a sequence that calls another, do not tread on each other). To **give back a value**, give it the name of the number to put it in:

```
define	double
calc	%half	$1	*	2
set	$2	%half
enddef

do	double	points	result
```

| Writing | Becomes |
|---|---|
| `line …` | `say` then `said` |
| `repeat <n>` … `endrepeat` | the lines, n times (nestable) |
| `each <a,b\|group> <command with $>` | the command for each, `$` replaced: `each pika,eevee emote $ joy` |
| `each <group> do <sequence> $` | each plays the sequence |

**Gestures** (`joy`, `surprise`, `hug`, `trip`...) are no longer engine sequences: they are recipes of the film language, in `film/recipes/gestures.film` (the "All words" table of `FILMS.md` gives them). Written in commands, they are `emote`, `jump`, `waitfor`.

## 18. Tools
- **Playing a film**: `scene_player <file>`; F5 reloads and replays it, `scene_player play <file> --from <label>` starts from a label.
- **Command names**: each command has one name, the one given in its row, and it works everywhere, also after `after`, `every`, `each`. All of them, A to Z, with their forms: the last table of this page.

- **While it runs**: L shows the last lines said (to read a conversation again), F5 reloads and replays it, F6 from the last label passed, F4 plays it **step by step** (one command at a time; F3: the next; F4 again: resume; the waiting command shows at the top; with F7 you see the numbers change) and `halt` in the file puts it there at that line (no effect outside the game), **F7 shows the backdrop's grid, where everyone stands and the scene's numbers with their value** (to place, and to see why an `if` passes or not), and at the top right the frames per second, how many are on stage and how many particles (in yellow under 50: too heavy a crowd); holding Space speeds it up; at the end, Escape goes back to the menu.
- **Placing**: `scene_player grid <place>` outputs the backdrop with a grid every 50 pixels (`target/films/<place>/grid.png`).
- **Seeing**: `scene_player view <file> [--scene N] [--every S]` writes a sheet of frames without a window (Enter pressed regularly); `scene_player image <file> 32s` one moment in large.
- **Checking**: `scene_player check <files>` reports: unknown or badly written commands, someone used before their `put`, a `put` or a `walk` off the backdrop, `said` without a line, a choice without `answer`, `answer` after the `end`, `if` / `while` / `switch` / `once` badly closed, `break` outside a `while`, missing labels, `gotoscene` to an unknown scene, an `@` spot without `spot`, a name given but never used, an unknown face or music, `do` with too few values, an included file not found, a number used that nothing gives (a typo?). The positions of `lineup`, `ring` and `walkto` are checked like those of `put`. Then, if nothing is wrong, it **plays the scene to the end without the game** (lines read at once, answers taken in turn, and the scene played again with the other answers, up to four times, the words asked for given, a wait for the player let go when the scene no longer moves forward) and reports: the engine stopping dead, a line badly filled by its numbers, a scene that does not end (an endless loop). It also makes **remarks** (`remark:`, which do not make the file wrong): someone who talks off screen or hidden by the dialogue box (once the camera is set).

## 20. Commands of the original game
Kept because the story taken from the game (`data/rom/story.txt`) uses them; in a scene of this project, prefer those in parentheses.

| Command | Effect |
|---|---|
| `fade <out 0/1> <white 0/1> <frames>` | a fade (`fadein`, `fadeout`, `whitein`, `whiteout`) |
| `dim 0 <frames> <light out of 256>` | darkens (`light`, `tint`) |
| `attr <who> <bits>` | 32 blinks, 64 transparent, 128 hidden, 8 on top (`blink`, `opacity`, `layer`) |
| `effect <who> <n> [frames]` | an emote or effect by number: 88 "!", 89 sweat, 90 "…", 91 anger, 92 "?", 93 notes, 171 tears, 642 sparkles (`emote`) |
| `pose <who> sleep\|hurt\|attack\|idle\|walk` | a pose (`anim <who> <name>`) |
| `anim <who> <group> <loop> <frozen>` | an animation by its number (`anim` by its name) |
| `t <frames>` / `wait` | waits |
| `map …` | the world map on the top screen |

## What is on the stage

What `film/` writes for what depends on the stage at the moment the line plays (`beat/place.rs`, `stage/placing/`). A place (`<spot>`) is one field: `w:x:y` (pixels, put on free ground and told if it is far), `m:x:y` (a word of the place), `x:x:y` (as is), `t:x:y` (a tile), `n:name`, `o:who`, `d:who` (in front), `b:who` (behind), `g:who`, `r:who`, `e:a:b` (between), `c:3:who` (3 tiles away), `p:who` (beside), `y:who:dx:dy`, `f:row:0:a,b:<spot>` (a place in a formation).

| Command | Effect |
|---|---|
| `ground <x> <y> <lines\|->` | the place's ground (`0` and `1`, lines separated by `/`) and its middle |
| `flies <who...>` | those who fly in this scene: they take up no place |
| `lands <who...>` | those who flew or swam walk on the ground again: walls hold them |
| `swims <pixels> <who...>` | those who swim by nature: sunk so many pixels where no one walks (the water), not on the ground; a `sink` of theirs wins |
| `ghosts <who...>` | those who pass through walls by nature: seen through while in one |
| `hovers <who...>` | those who hover by nature: a little bob in the air (2 pixels, a second), once on the stage |
| `setspot n:<name> <spot>` | names a place, found now |
| `find <var> <who\|-> <spot> [apart] [exact]` | finds a place for who, in `{var_x}` and `{var_y}` |
| `spread <frame\|place> <who...>` | they move apart at the start of a scene, the view on them |
| `goes <who> <walk\|run> <-\|x0.5\|s2> <x> <y>...` | goes through these points, going around walls and people; a pace: so many times a walker's, or so many seconds |
| `arrives <who> <walk\|run> <pace> <x> <y> [place\|enter] [by:<side\|entrance>]` | comes in from where the view does not go, or in smoke if there is no way; `place`: whoever is there walks there; in a place that has entrances, by the nearest one on that side; `by:left`, `by:south`: by this side or this entrance, from outside the place, in a line behind whoever comes in that way |
| `leaves <who> <right\|left\|entrance> [run]` | (`run`: running, scared) leaves the place by the nearest entrance on that side (the place's, otherwise its ground nearest the edge) or by that one, going around what is in the way, in a line behind whoever leaves that way, and disappears once outside |
| `entrances <name> <x> <y>…` | the place's entrances (`data/extra/decors.txt`): `film/` writes them on its own |
| `roundcheck <who> <spot>` | says what is in the way of a round |
| `alllook <who\|side>` | everyone looks |
| `crowd <walk\|-> <spot\|-> [shape:ring\|flock\|line\|heap] [radius:<px>] [tight:1] [from:<side>] [run:1] <who...>` | a crowd around this place: in a ring, away from those who act; in a flock or in a tight heap on free ground; in a line; `radius`: this wide; `from:right`: they all come in from that side of the view, each keeping their place, straight ahead |
| `gathers <var> <spot> <step x> <step y> <who\|-...>` | each on free ground around the place, their place in `{var<n>_x}` |
| `advance <dx> <dy> <who...>` | a crowd runs, staying on the ground |
| `flock <spot> <walk\|run> <who...>` | a crowd goes to this place, its middle there, each keeping their place (straight ahead) |

| `seats <var> <who...> ; <x> <y>...` | each goes to a seat they can reach; the middle of the table in `{var_x}` |
| `shot <zoom> <who\|->` | this zoom, less if the body needs it |
| `roams <who> <wide>` | they stroll around, as wide as the ground allows |

## Every command and its forms

Each command's forms as the engine declares them (`engine/src/beat/`, each kind's `FORMS`), as a film writes them: a part `name:kind` (`code` someone, `coordinate` a number or a spot, `duration`, `number`, `word`), a part between brackets that may be left out, `name...` all the words left. Written from the code (`GOLDEN=write cargo test -p engine forms`).

<!-- forms: written by engine/src/beat/forms.rs -->
| Command | Forms |
|---|---|
| `add` | `add name:word [value:number]` |
| `advance` | `advance dx:number dy:number who:code` |
| `after` | `after time:duration command...` |
| `alllook` | `alllook whoside:word` |
| `ambient` | `ambient namen:word` |
| `anim` | `anim who:code name:word [loop] [frozen]` · `anim who:code name:word n:number times` · `anim who:code group:word loop:word frozen:word` |
| `answer` | `answer [text...]` |
| `appear` | `appear who:code [frames:duration]` |
| `approach` | `approach who:code other:code frames:duration` |
| `arrives` | `arrives who:code walkrun:word pace:word x:coordinate y:coordinate [rest...]` |
| `ask` | `ask [text...]` |
| `attach` | `attach node:word parent:word [dx:number dy:number] [inherit pos angle:number scale:number opacity colour:word]` |
| `attr` | `attr who:code bits:word` |
| `audience` | `audience rest...` · `audience off` |
| `autoface` | `autoface [rest...]` |
| `backdrop` | `backdrop place:word 1 -` |
| `banner` | `banner text...` |
| `bar` | `bar who:code v:word to rest...` · `bar who:code off` |
| `battlelog` | `battlelog frames:duration text...` |
| `blink` | `blink who:code rest...` |
| `body` | `body who:code wide:word high:word` |
| `bounce` | `bounce who:code [height:number] [frames:duration]` |
| `bounds` | `bounds left:word top:word right:word bottom:word` · `bounds off` |
| `boxroom` | `boxroom [rest...]` |
| `boxside` | `boxside rest...` |
| `break` | `break [text...]` |
| `bubble` | `bubble who:code frames:word text...` |
| `calc` | `calc name:word a:word op:word b:number` |
| `camera` | `camera who:code` · `camera @ x:coordinate y:coordinate` |
| `caption` | `caption frames:duration text...` |
| `carry` | `carry item:word who:code` · `carry item:word off` |
| `chase` | `chase who:code other:code` |
| `checkpoint` | `checkpoint label:word` |
| `collisions` | `collisions rest...` |
| `color` | `color rest...` |
| `colour` | `colour who:code r:number g:number b:number [frames:duration]` |
| `continue` | `continue [text...]` |
| `control` | `control who:code` · `control off` |
| `credits` | `credits time:duration lines...` |
| `crossfade` | `crossfade place:word [time:duration]` |
| `crowd` | `crowd walk:word spot:word [rest...]` |
| `cutaway` | `cutaway [over] place:word x:coordinate y:coordinate [time:duration]` · `cutaway off` · `cutaway pan x:coordinate y:coordinate time:duration` |
| `detach` | `detach node:word` |
| `dim` | `dim 0 frames:duration light:word out of rest...` |
| `disguise` | `disguise who:code other:code` · `disguise who:code off` |
| `door` | `door scene:word arrival:word x:coordinate y:coordinate [radius:number] [width:number height:number]` |
| `drawing` | `drawing item:word file:word [frames:duration] [rhythm:word]` |
| `ease` | `ease rest...` |
| `effect` | `effect who:code n:number [frames:duration]` |
| `elif` | `elif [text...]` |
| `else` | `else [text...]` |
| `emit` | `emit kind:word who:code [per second] [colour:word] [time:duration] rest...` |
| `emote` | `emote who:code name:word [frames:duration] [plus]` |
| `end` | `end` |
| `endask` | `endask` |
| `endif` | `endif [text...]` |
| `endwhile` | `endwhile [text...]` |
| `entrances` | `entrances name:word x:coordinate rest...` |
| `every` | `every time:duration command...` · `every off` |
| `face` | `face who:code other:code` |
| `fade` | `fade out:word rest...` |
| `fadein` | `fadein frames:duration` |
| `fademusic` | `fademusic v:word frames:duration` |
| `fadeout` | `fadeout frames:duration` |
| `fadesound` | `fadesound name:word volume:word time:duration [stop]` |
| `fall` | `fall who:code [height:number] [frames:duration]` · `fall` |
| `fidget` | `fidget who:code [rest...]` |
| `filmline` | `filmline n:number` · `filmline n:number until who:code` · `filmline n:number for time:duration` · `filmline n:number track node:word` · `filmline n:number again` |
| `find` | `find var:word who:word spot:word [apart] [exact]` |
| `fishline` | `fishline who:code item:word [x:coordinate y:coordinate]` · `fishline who:code item:word holds` · `fishline who:code off` |
| `flash` | `flash colour:word [frames:duration]` |
| `flee` | `flee who:code from:word [distance:number]` |
| `flies` | `flies who:code` |
| `flip` | `flip who:code rest...` |
| `float` | `float who:code [height:number] [frames:duration]` |
| `flock` | `flock spot:word walkrun:word who:code` |
| `follow` | `follow who:code leader:code [gap:number]` · `follow who:code -` |
| `frame` | `frame frames:duration rest... [rest2...]` |
| `game` | `game talk who:code rest...` |
| `gathers` | `gathers var:word spot:word step:word rest...` |
| `gauge` | `gauge [rest...]` · `gauge [place:word] off` |
| `ghosts` | `ghosts who:code` |
| `give` | `give item:word from:word to:word [frames:duration] [keep]` |
| `glance` | `glance who:code [frames:duration] [wait]` · `glance @ x:coordinate y:coordinate rest...` |
| `go` | `go who:code x:coordinate y:coordinate frames:duration slide:word rest...` |
| `goes` | `goes who:code walkrun:word x05s2:word x:coordinate y:coordinate` |
| `gone` | `gone who:code [rest...]` |
| `goto` | `goto label:word` |
| `gotoscene` | `gotoscene name:word` |
| `grey` | `grey rest...` |
| `ground` | `ground place:word` · `ground x:coordinate y:coordinate lines:word` |
| `halo` | `halo who:code [radius:number] [colour:word]` · `halo @ x:coordinate y:coordinate [radius:number] [colour:word]` |
| `height` | `height who:code height:number [frames:duration]` |
| `hold` | `hold item:word who:code` · `hold item:word off` |
| `holdgaze` | `holdgaze who:code [rest...]` |
| `hop` | `hop who:code x:coordinate y:coordinate height:number frames:duration` · `hop` |
| `hovers` | `hovers who:code` |
| `hud` | `hud [rest...]` · `hud [place:word] off` |
| `if` | `if condition...` |
| `image` | `image name:word file:word x:coordinate y:coordinate` · `image name:word off` |
| `infront` | `infront who:code otheroff:word` |
| `iris` | `iris out [frames:duration] [who:code]` · `iris in rest...` |
| `jingle` | `jingle n:number` |
| `jump` | `jump who:code [height:number] [frames:duration]` |
| `keepframe` | `keepframe rest...` · `keepframe off` |
| `keeponly` | `keeponly rest...` |
| `keepspeaker` | `keepspeaker [rest...]` |
| `label` | `label label:word` |
| `lands` | `lands who:code` |
| `large` | `large item:word` · `large off` |
| `layer` | `layer who:code rest...` |
| `leaves` | `leaves who:code rightleftentrance:word [run]` |
| `letterbox` | `letterbox 1` · `letterbox 0` |
| `light` | `light ambience:word [frames:duration]` |
| `lightning` | `lightning who:code` · `lightning @ x:coordinate y:coordinate` |
| `lineup` | `lineup x:coordinate y:coordinate dx:number dy:number rest...` |
| `listen` | `listen rest...` |
| `log` | `log text...` |
| `lookat` | `lookat who:code x:coordinate y:coordinate` |
| `measure` | `measure name:word a:word b:number` |
| `minimap` | `minimap who:code` · `minimap off` |
| `mirror` | `mirror who:code model:word` · `mirror who:code off` |
| `mood` | `mood who:code face:word` · `mood who:code -` |
| `move` | `move who:code move:word [target:word]` |
| `moveeffect` | `moveeffect who:code move:word` |
| `moveshot` | `moveshot from:word to:word move:word [frames:duration]` |
| `movesound` | `movesound move:word` |
| `music` | `music namen:word` · `music name:word time:duration` · `music back` · `music rest...` |
| `musicfrom` | `musicfrom namen:word time:duration` |
| `musicspeed` | `musicspeed percent:word` |
| `node` | `node name:word group [x:coordinate y:coordinate]` · `node name:word picture file:word [x:coordinate y:coordinate]` · `node name:word off` |
| `note` | `note cdefgab:word [2] [sound]` |
| `onhit` | `onhit who:code sound:word rest...` |
| `onmenu` | `onmenu label:word` |
| `ontalk` | `ontalk who:code label:word` |
| `opacity` | `opacity who:code v:word to rest...` |
| `orbit` | `orbit who:code other:code [laps:number] [frames:duration]` · `orbit who:code x:coordinate y:coordinate [laps:number] [frames:duration]` |
| `pan` | `pan who:code frames:duration` · `pan @ x:coordinate y:coordinate frames:duration` · `pan who:code frames:duration fixed` · `pan @ x:coordinate y:coordinate frames:duration fixed` |
| `parallel` | `parallel label:word` · `parallel off` |
| `particles` | `particles kind:word who:code [number:word] [colour:word]` · `particles kind:word @ x:coordinate y:coordinate [number:word] [colour:word]` |
| `path` | `path who:code x1:coordinate y1:coordinate x2:coordinate y2:coordinate rest...` |
| `playsound` | `playsound sound:word [volume v:number] [pan p:number] [from who:code [near]] [pitch n:number] [loop] [as name:word] [rise time:duration]` |
| `popup` | `popup who:code colour:word words...` |
| `portrait` | `portrait who:code rest...` |
| `pose` | `pose who:code name:word image n:number` · `pose who:code rest...` |
| `position` | `position who:code x:coordinate y:coordinate` |
| `prop` | `prop node:word property:word value:number` |
| `put` | `put who:code x:coordinate y:coordinate dir:word` |
| `putnear` | `putnear who:code other:code [side:word]` |
| `quake` | `quake strength:number [frames:duration]` · `quake` |
| `random` | `random name:word n:number` · `random name:word a:word b:number` |
| `react` | `react [rest...]` |
| `recoil` | `recoil who:code from:word [distance:number]` |
| `reset` | `reset who:code` |
| `resume` | `resume who:code` |
| `return` | `return [text...]` |
| `ride` | `ride who:code mount:word [height:number] [offset:number]` · `ride who:code off` |
| `ring` | `ring x:coordinate y:coordinate radius:number rest...` |
| `rise` | `rise who:code height:number frames:duration` |
| `roam` | `roam` |
| `roams` | `roams who:code wide:word` |
| `roundcheck` | `roundcheck who:code spot:word` |
| `run` | `run who:code x:coordinate y:coordinate` |
| `runpath` | `runpath rest...` |
| `runstep` | `runstep rest...` |
| `runto` | `runto rest...` |
| `said` | `said [text...]` |
| `savegame` | `savegame` |
| `say` | `say who:code face:word text...` |
| `scale` | `scale who:code size:number [frames:duration]` |
| `seats` | `seats var:word who:code rest...` |
| `sees` | `sees name:word watcher:word who:code [distance:number] [angle:number]` |
| `set` | `set name:word value:number` |
| `setspot` | `setspot rest...` |
| `settext` | `settext name:word words...` |
| `shadow` | `shadow who:code rest...` |
| `shake` | `shake axis:word strength:number [frames:duration]` |
| `sheet` | `sheet who:code file:word w:word v:word [rest...]` |
| `shine` | `shine item:word [off]` |
| `shot` | `shot zoom:number who:word` |
| `shout` | `shout rest...` |
| `sink` | `sink who:code pixels:number [sand]` |
| `slowmo` | `slowmo who:code timesoff:word rest...` |
| `sound` | `sound namen:word` · `sound namen:word who:code` |
| `spacing` | `spacing [rest...]` |
| `speed` | `speed who:code pixels:number per rest...` |
| `spin` | `spin who:code laps:word frames:duration [rotate]` |
| `spotlight` | `spotlight who:code [radius:number]` · `spotlight @ x:coordinate y:coordinate [radius:number]` · `spotlight off` · `spotlight plus who:code [radius:number]` · `spotlight plus @ x:coordinate y:coordinate [radius:number]` |
| `spread` | `spread frameplace:word who:code` |
| `status` | `status who:code name:word [plus]` · `status who:code off` |
| `step` | `step who:code dx:number dy:number` |
| `still` | `still who:code rest...` · `still rest...` |
| `stop` | `stop who:code` |
| `swapcontrol` | `swapcontrol rest...` · `swapcontrol off` |
| `swims` | `swims pixels:number who:code` |
| `t` | `t duration:duration` · `t frames:duration` |
| `takeable` | `takeable item:word [off]` |
| `tempo` | `tempo speed:number` |
| `textspeed` | `textspeed times:number` · `textspeed normal` |
| `thing` | `thing item:word rest...` |
| `think` | `think rest...` |
| `throw` | `throw who:code to:word [height:number] [frames:duration]` |
| `timed` | `timed frames:duration` |
| `timer` | `timer time:duration` · `timer off` |
| `tint` | `tint r:number g:number b:number strength:number frames:duration` · `tint colour:word strength:number frames:duration` |
| `track` | `track node:word property:word [rest...]` |
| `trackpath` | `trackpath node:word frames:duration linescatmullbezier:word curve:word [rest...]` |
| `train` | `train gap:number leader:code rest...` · `train off` |
| `tremble` | `tremble who:code [strength:number] frames:duration` |
| `turn` | `turn who:code dir:word` |
| `turnaround` | `turnaround who:code` |
| `turnby` | `turnby who:code eighths:word` |
| `turnslow` | `turnslow who:code [frames:duration]` |
| `typecaption` | `typecaption rest...` |
| `uncolor` | `uncolor rest...` |
| `uncolour` | `uncolour who:code [frames:duration]` |
| `unfreeze` | `unfreeze [text...]` |
| `untrack` | `untrack node:word [property:word]` |
| `vanish` | `vanish who:code [frames:duration]` |
| `views` | `views who:code rest...` · `views who:code off` |
| `voice` | `voice who:code sound:word` · `voice who:code -` · `voice who:code namen:word` |
| `volume` | `volume v:word to rest...` |
| `wait` | `wait duration:duration` · `wait` |
| `waitall` | `waitall` |
| `waitbubbles` | `waitbubbles` |
| `waitcam` | `waitcam` |
| `waitfade` | `waitfade` |
| `waitfor` | `waitfor who:code [rest...]` |
| `waitmoves` | `waitmoves who:code` |
| `waitnear` | `waitnear who:code x:coordinate y:coordinate [radius:number]` |
| `waitsound` | `waitsound sound:word [pitch n:number]` |
| `waittrack` | `waittrack node:word` |
| `waituntil` | `waituntil condition...` |
| `walk` | `walk who:code x:coordinate y:coordinate` |
| `walkto` | `walkto who:code x:coordinate y:coordinate` |
| `wall` | `wall left:word top:word right:word bottom:word` · `wall off` · `wall name:word left:word top:word right:word bottom:word` · `wall name:word off` |
| `wander` | `wander who:code [radius:number]` |
| `wave` | `wave strength:number [time:duration]` |
| `weather` | `weather name:word` |
| `while` | `while condition...` |
| `whitein` | `whitein frames:duration` |
| `whiteout` | `whiteout frames:duration` |
| `wipe` | `wipe rest...` |
| `zoom` | `zoom closeness:word frames:duration` |
<!-- end of forms -->
