# The scene language

A single language, in plain English, for films and their recipes. This file is its
reference. Each word is described in the table generated from the vocabulary (`film/FILMS.md`,
"All the words").

The text of a film is read only once, by the `language` crate:

```
text ─lexer─▶ tokens ─parser─▶ tree (each node has its span)                language/src
     ─written─▶ the lines the words play, each with its number             film_core/src/text/front/
     ─words─▶ the engine's commands ─▶ engine
```

The language service (`film/src/service.rs`) answers the Workshop, `scene_player analyze` and
agents: what is wrong (range, fixes), the outline, the folds, where a name is defined, its
references, renaming it, what a word is, what can be written here.

Every error is a `Diagnostic`: a code (`E0102`), a severity (error, remark, detail), a
span (file, line, start and end column), a message, notes, and fixes given as edits to the
text. The reader does not stop at the first one: it reports them all.

## The rules

The vocabulary (`film_core/src/vocabulary.rs`) follows these rules:

1. **Who acts comes first**, like a spoken line (`pikachu (happy): Hi`): `pikachu walk to piplup`, `pikachu hug piplup`, `pikachu emote shock`. A line where nobody acts starts with its word: `music beach`, `wait 2s`, `frame pikachu piplup`, `transition fade`. One order only: a line written the word first (`walk pikachu to piplup`) is an error, `E-ORDER`, its fix writing it who first. The compiler reads it from the words' forms (a first part `who:pokemon`, `who:code`, `who:word`, a team of one: `film_core/src/text/written.rs`), never from a list of words, and from the words' own declarations for those written word first (`Word::word_first`).
2. **One word per idea**: variants are options of the word.
3. **`off` only for a state that lasts** (sleep, weather, music, an emotion, a chase); an action with its own verb keeps it (`drop`, `close`, `vanish`).
4. **Plain English verbs**, an underscore only when nothing else will do (`shake_head`).
5. **The game's assets first** (the sprites' animations, the emotes and their sound, on by default, the moves' effects, the status icons, the weather's pictures); a drawing of one's own only where the game has none, pixel-crisp.
6. **A film only stages**: no dungeon, journey or save; that is the `.game`'s.
7. **The quality gate of each word**: no remark, its picture looked at, a Wiki page whose example plays (`CONTRIBUTING.md`).

The families: Structure, Speech, Presence, Formations, Movement, Turning, Body, Emotions, Camera, Screen, Sound, Effects, Objects, Fights, Time, Story; the optional Dungeon, Guild and Fishing, after `uses dungeon`, `uses guild`, `uses fishing` (their built-in libraries); Advanced, folded.

## 1. A file

A `.film` file is a sequence of lines. A line is an instruction, a header, a comment or
nothing. Indentation has no meaning: the opening words and `end` make the blocks. The
formatter (`scene_player format`, Ctrl+Shift+F in the Workshop) tidies it up, two spaces per
level.

```
title: Fishing for two           # a header
uses welcome                     # the recipes of another file
pinchy = krabby                  # a name for a Pokémon

scene: beach_evening pikachu piplup
moment: the bet
pikachu (happy): Want to bet?
piplup walk to right &
the_end
```

- **Comment**: `#` to the end of the line, except inside a spoken line or a text in
  quotes. The comment just above a `define` is its documentation.
- **Headers** (at the start of a line, followed by `:`): `title:`, `scene:`, `moment:`. A `scene:`
  lasts until the next one, with no `end`. Its lines are not indented.
- **Before the first scene**: `title:`, `uses`, `include`, `for ds`, the names (`name = species`), the
  film's `define`s, `let`s and `keep`s.
- **`for ds`**: the film is written for the DS (Explorers of Sky, `scene_player export-ds`),
  played as a new game starts; `for ds scene`, in place of a scene of a game going on. What
  the DS can't play is then told (`E-DS-…`, `R-DS-…`, film/FILMS.md).

## 2. Lines

| Form | Example |
|---|---|
| spoken line | `pikachu (shouting, fast): No!` |
| words | `piplup walk to right` |
| variable | `let n = 3`, `n = n + 1`, `keep saw_kyogre = true` |
| block | `if n > 2` … `end` |
| return | `return catches` (in a recipe) |

After any line that takes time:

- `&`: it starts and the film goes on;
- `& as wind`: the same, and you can wait for it afterwards (`wait wind`).

### Spoken lines

```
who [(manners)]: text
```

- `who`: a Pokémon of the scene, `narrator`, or one of the crowd (`marill3`).
- `manners`: a face (`happy`, `shouting`…) and manners (`fast`, `slow`, `top`), separated
  by commas.
- In the text: `{expr}` is replaced by its value (`{who}`: the name as the game writes it; `{money}`,
  a flag, a counter: what the game gives the film), `[1s]` is a pause, `*word*` is emphasised.
- `{if test: words | other}`: the words the test chooses as the film plays (`Piece::Choose`; none
  after the `|`: nothing).
- A line's id, written last: `pikachu: Hello! | fr: Bonjour ! #k3f9` (`Said::id`, a letter then
  letters, digits or `_`): its own name whatever its words become, given by `scene_player ids`.
- Other languages after the text, each after a `|`, its code (two or three letters) and `:`:
  `pikachu: Hello! | fr: Bonjour ! | es: ¡Hola!` (`Said::languages`). The film picks its own
  (`DUNGEON_FILM_LANGUAGE`), English otherwise.
- A reserved word (`if`, `when`, `scene`…) is never a `who`. That is what tells a
  spoken line apart from a header or a block.

### Words

```
word argument argument …
```

A word is a verb of the vocabulary or a recipe. Its arguments are read according to its form (§5):

- **names**: a Pokémon, a place, a thing, a spot (`piplup`, `beach`, `Apple`, `throne`);
  several words if need be (`Iron Tail`);
- **numbers**: `3`, `-12`, `+20`, `0.5`;
- **durations**: `1.5s`, `6f` (frames, 60 a second);
- **points**: `480 108` (two numbers where a place is expected);
- **texts**: `"in quotes"`, or the remaining words for a `words...`;
- **a calculation**: `{expr}`, for example `a put {b.x + 16} {b.y}`.

Outside braces, there are no operators: `-12` is a number, not a subtraction.

## 3. Blocks

Each block opens with its word and closes with `end`.

| Block | What it does |
|---|---|
| `if expr` … `else if expr` … `else` … `end` | a branch |
| `repeat expr` … `end` | n times |
| `for each x in expr` … `end` | for each one (`for each x, i in l`: with its rank, from 1) |
| `while expr` … `end` | as long as it is true (at most 10,000 turns per frame) |
| `together` … `end` | everything starts at the same time; the film waits for the longest |
| `beside [name]` … `end` | a thread, alongside the film; `wait name` waits for it |
| `when event` … `end` | alongside, as soon as the event happens (§6) |
| `at mark [± duration]` … `end` | alongside, that long after a `mark` of the scene |
| `every duration` … `end` | alongside, every so often |
| `after duration` … `end` | alongside, once, in that long |
| `choice` / `option text` … / `end` | the player chooses; one branch per `option`; `option "text" [if test] [once]`: offered only when the test holds, only until taken |
| `fight directed a b against c d [level n] [winner a]` … `end` | a fight; its lines are turns (§7) |
| `define form [-> type]` … `end` | a recipe (§5) |

- **Short form**: `when`, `at`, `every` and `after` can also be written on one line, with no
  `end`: `when piplup arrives : emote pikachu question`.
- **A name**: a block alongside (`beside`, `when`, `at`, `every`, `after`) takes `as name` at
  the end of its opening (`every random(3s, 4s) as noises : sound click`). `wait name`
  waits for it, `cease name` stops it. `beside name` is the short form of `beside as name`.
- **Inside a loop**: `break` leaves the loop, `next` goes on to the next turn.

## 4. Values, calculations, variables

A calculation is written after `let x =`, `x =`, `if`, `while`, `repeat`, `return`, `for each … in`,
between the parentheses of a call, and between braces everywhere else.

- **Types**:
  - `number` (whole or decimal), `text`, `bool` (`true`, `false`), `duration`;
  - `pokemon`, `place` (a point, a spot, someone), `thing`, `face`, `music`, `sound`;
  - `list<T>`, and `none` (an optional argument not given).
- **Literals**:
  - `3`, `0.5`, `"text"`, `true`, `1.5s`, `6f`;
  - `[a, b, c]` (a list);
  - `(480, 108)` (a point).
- **Operators**, from the tightest to the loosest:
  - `.` (`a.x`, `a.hp`, `l.length`) and `[i]` (`l[1]`, from 1);
  - `not`, `-`;
  - `* / %`;
  - `+ -` (`+` also joins two texts or two lists);
  - `< <= > >= = !=`;
  - `and`;
  - `or`.
- **Functions**: `chance(50)` (true one time in two), `random(1, 6)`, `div(7, 2)` (whole-number
  division: 3; `7 / 2` is 3.5), `distance(a, b)`,
  `health(a)` (their HP, as a percentage), `position(a)`, `abs`, `min`, `max`, `round`. A recipe
  that returns a value can also be called this way: `let n = long_cast(pikachu, (480, 108))`.
- **What the game says** (read on every frame): `clock`, `answer` (the last option chosen),
  `timer(name)`, `controlled`, `target`.
- **What the story played**: `seen(name)`, how many times that moment (or scene, by its place) was
  played; `go moment name` and `go scene place` go there (`film_core/src/groups/story.rs`; the names counted, `film_core/src/plot.rs`).
- **Scopes**:
  - `let` in a block or a recipe: until its `end`;
  - `let` in a scene: until the end of the scene;
  - `let` before the first scene: the whole film;
  - `keep x = …`: the game's (a flag, a counter): the game playing the film gives it (1 up, 0
    down), the film's value only taken when the game gives none (0); what the film leaves in it, and
    what `set x` and `set x off` set, the game keeps once the film ends (`film/FILMS.md`, “A game”).
- **Reserved names**:
  - `there`: the last place given;
  - `team`: the last team named;
  - `hero`, `partner`, `narrator`.

Types are checked before playing: `wait 30` (a number where a duration is expected)
is an error, with its fix `wait 0.5s`.

## 5. Forms and recipes

The same notation describes the form of every word of the vocabulary and the header of a recipe.

| Element | Meaning |
|---|---|
| `who:pokemon` | a typed argument |
| `to`, `off`, `against` | that very word, as it is |
| `[ … ]` | optional |
| `a | b` | one or the other |
| `turns:number=4` | optional, with its value; can be given by its name: `turns=6` |
| `*who:pokemon` | one or more (a `list<pokemon>`) |
| `words...` | the remaining words (a `text`) |

```
# They cast several times and count their catches.
define long_cast who:pokemon to where:place turns:number=3 -> number
  let catches = 0
  repeat turns
    throw who line to where
    if chance(50)
      catches = catches + 1
    end
  end
  return catches
end

long_cast pikachu to 480 108                    # as a word
let n = long_cast(pikachu, (480, 108), 5)       # as a function
```

- **Arguments** are always typed: a word on its own (`to`, `off`) is a literal word, never an argument. They are typed values. Their variables live only inside the recipe. A word on its own that the recipe's lines then use as a name (`define cheer who`, then `who emote joy` or `{who}`) is told at once, with how to write it: `who:word`.
- **Several recipes with the same name**: the first one whose form fits is taken. If none fits,
  the diagnostic says why each one does not.
- **A recipe is checked where it is written**, once, even if it is never called: each line a
  known word with a form that fits, each argument standing for a value of its type (an untyped
  one could be any word). What is wrong is told on the recipe's own line; a line that only fails
  with what a call gives it is told at the call, with the recipe line (``in recipe `stare` (line 2)``).
- **An optional argument** not given is `none`, and `if other` is true if it is given. A word that
  may be left out, or one of a choice (`[miss]`, `[slowly|fast]`, `right|left`), is one too: `if miss`
  … `else` … `end` tells whether the call said it.
- **A reckoning of numbers known at the call** (written, or given: `sum - div(sum * 9, 10)`, with
  `+ - * /`, `div`, `min`, `max`, `abs`, `round`) is worked out once, there; the others as the film plays.
- **Called with `&`**, a recipe plays alongside the film.
- **`uses name`**: the recipes of `library/films/recipes/name.film`, or of
  `film/recipes/…`. `uses dungeon`, `uses guild`, `uses fishing`: the built-in library of an optional family
  (all of `film/recipes/fishing/`…), whose words a film says only after it. That file contains only `define`s and `let`s.
- **`include name`**: all the scenes of that film, in this place.

## 6. Time

- **A line waits to be finished.** A movement waits for its end; an effect (`sound`, `effect`,
  `emote`, `shake`) is not waited for. The vocabulary says so for each word.
- **`&`** sends it off alongside. `wait name`, `wait a b` (until they have arrived),
  `wait all` (everything that moves and everything playing alongside), `wait 2s`.
- **A thread** (`beside`, `&`, `when`, `every`…) has its own stack and its own variables: it can
  say lines, ask choices and call recipes. The scene's and the film's variables are
  shared.
- **`when` events**:
  - `when a arrives`: a has finished their path;
  - `when a leaves`: a starts moving;
  - `when a is ko`: in a fight, no health left (like `when health(a) <= 0`);
  - `when expr`: as soon as the calculation becomes true, once (`when health(kyogre) <= 50`).
- **Marks**: `mark name` notes the clock. Then `at name + 2s : line`.
- **The pace**: `pace slow`, `pace brisk`, `pace 1.5` make the waits said after it
  (`wait 2s`) longer or shorter, a word's own time staying its own; `pace normal` as written.
  `pace lines 2s`: each line said stays that long, then goes on alone; `pace lines off`.
- **Timers**: `timer name 30s`; `timer(name)` gives what is left. You can have as
  many as you like.
- **Randomness** is drawn from the scene's seed: a film always replays the same way.

## 7. The directed fight

In a `fight directed` block, each line is a turn or an ordinary line:

```
fight directed pikachu piplup against kyogre level 70 winner pikachu
  hp kyogre 640
  hud pikachu kyogre
  when health(kyogre) <= 50 : kyogre (angry): The whole sea obeys me!
  kyogre Hydro Pump all -40
  pikachu Thunder Shock kyogre -110 crit
end
```

- **A turn**: `user Move target [misses] [crit] [ko] [-30] [protect] [slow] [slowmo 30]
  [intercepts d]`.
- **The fight's words** (`hp`, `level`, `knows`, `hud`, `change`, `log`, `number of
  turns`, `random until the end`, `uses`, `throws`) exist only inside this block.
- A fight plays on the film's own course, never alongside.

## 8. Words that change

Some words used to have two meanings, one for films and one for the engine. They now have only one:

| Word | Its only meaning | The other one becomes |
|---|---|---|
| `when` | the event block (§6) | `when <test> <scene>` → `if test` … `gotoscene name` … `end` |
| `choice` | the choice block | the command `answer` → `option` |
| `end` | closes a block | the command `end` → `the_end` |
| `answer` | the last option chosen (a value) | — |
| `repeat` | the block | — |
| `jump` | jumping (a Pokémon) | `goto` → no jumps: loops, recipes, `gotoscene` |
| `exit` | going out (a Pokémon) | `break` (the command) → `break` (the loop word) |

## 9. Old forms

The reader recognises them and gives the new form, with its fix.

| Before | Now |
|---|---|
| `if … endif` | `if …` … `end` |
| `$px` (a number of this call) | `let px = …` in the recipe |
| `if other:` … `end` (recipe) | `if other` … `end` |
| `for who: line`, `{rank}` | `for each q, rank in who` … `end` |
| `position a $x $y`, then `{$x}` | `{a.x}`, `{a.y}`: where they stand when the line plays |
| `choice A | B` / `answer 1` … / `end` | `choice` / `option A` … / `option B` … / `end` |
| `when c 50 | line | line` (fight) | `when health(c) <= 50` … `end` |
| `> command` | the command's word |
| `during`, `after` (at the start), `at 3s:`, `pause` | `&` and `wait` |
| `every 3s~4s sound click`, `every off` | `every random(3s, 4s) as noises : sound click`, `cease noises` |
| `define f who where:place` (an untyped argument) | `define f who:pokemon where:place` |
| a word of the old vocabulary (`fadein`, `turn a left`, `camera on a`) | the form now (`transition fade in`, `a face left`, `frame a snap`): `E-OLD-WORD`, `film_core/src/vocabulary.rs`, `REPLACED` |
| the word first, said of someone (`walk a to b`) | who first (`a walk to b`): `E-ORDER` |

## 10. Not there yet

What the language already reads, but the engine does not play yet (the reader says so):

- **Numbers are decimal** (`let x = 7 / 2` is 3.5, written `3.5` in a spoken line; a
  whole number is written with no decimals). The arguments of a film's words (pixels,
  numbers of times) are rounded. Texts are kept (`let name = "word"`) but cannot be
  calculated with. Lists are only used by `for each`, over a written list or over those a recipe
  receives (`*who`).
- **`health(a)`** can only be read in a fight: `when health(c) <= 50`. **`position(a)`** is written
  `a.x` and `a.y`.
- **Threads alongside** (`beside`, `when`, `every`) do not ask `choice`s, as before.
- **Recipes** are kept as the language reads them, and written out at each call with what
  they are given (calculations done, `if a` and `for each x in *who` decided at the call,
  the call's own numbers); a recipe that calls itself stops at a depth of eight calls,
  and `return` goes at the end.
- **Durations**: a bare number where a duration is expected is an error in the language's
  blocks (`after`, `every`); in the engine's commands, it is only noted by
  `check`.
- **The game's story** (`data/rom/story.txt`, extracted by `extractor` from your copy of the game)
  is still in the engine's commands
  (`engine/docs/COMMANDS.md`): it goes through `story::lowering::read_text`, films do not.

## 11. Grammar

```
file         = { line } ;
line         = [ instruction ] [ comment ] end_of_line ;
instruction  = header | species_name | spoken | variable | block | return | call [ suffix ] ;
header       = ("title" | "scene" | "moment") ":" rest ;
species_name = NAME "=" NAME ;
spoken       = who [ "(" manner { "," manner } ")" ] ":" text [ "#" ID ] [ suffix ] ;
variable     = ("let" | "keep") NAME "=" expr | NAME "=" expr ;
return       = "return" [ expr ] ;
call         = WORD { argument } ;
argument     = WORD | NUMBER | DURATION | TEXT | "{" expr "}" ;
suffix       = "&" [ "as" NAME ] ;
block        = opening end_of_line { line | middle } "end"
             | short_opening ":" instruction ;
opening      = "if" expr | "repeat" expr | "for" "each" NAME [ "," NAME ] "in" expr
             | "while" expr | "together" | "beside" [ NAME ] | "when" event
             | "at" NAME [ ("+" | "-") DURATION ] | "every" expr | "after" expr
             | "choice" | "fight" "directed" { argument } | "define" form [ "->" type ] ;
short_opening = "when" event | "at" NAME [ ("+" | "-") DURATION ] | "every" expr | "after" expr ;
                                   (an opening alongside can end with "as" NAME)
middle       = "else" [ "if" expr ] | "option" ( rest | TEXT [ "if" expr ] [ "once" ] ) ;
event        = NAME ("arrives" | "arrive" | "leaves" | "is" "ko") | expr ;
expr         = or ;
or           = and { "or" and } ;
and          = cmp { "and" cmp } ;
cmp          = sum [ ("<" | "<=" | ">" | ">=" | "=" | "!=") sum ] ;
sum          = product { ("+" | "-") product } ;
product      = unary { ("*" | "/" | "%") unary } ;
unary        = ("not" | "-") unary | postfix ;
postfix      = atom { "." NAME | "[" expr "]" | "(" [ expr { "," expr } ] ")" } ;
atom         = NUMBER | DURATION | TEXT | "true" | "false" | "none" | NAME
             | "[" [ expr { "," expr } ] "]" | "(" expr [ "," expr ] ")" ;
form         = WORD { element } ;
element      = NAME ":" type [ "=" argument ] | "*" NAME [ ":" type ] | NAME "..." [ "=" argument ]
             | WORD (a literal word) | "[" { element } "]" | element "|" element ;
type         = NAME [ "<" type ">" ] ;
```

`WORD` is a sequence of letters (accents and capitals included), digits, `_`, `'` and
`-` inside. `NAME` is a lower-case `WORD` that starts with a letter.

**Reserved words** are reserved only at the start of a line. There, they are neither a word to
call nor a `who`. In arguments, they are ordinary words (`left_of a`, `a walk to cave`).

```
title scene moment end if else repeat for while together beside when every after choice
option fight define return let keep break next uses
```

- **`at`** is a mark only if it is followed by the name of a mark. `at: Yes.` and
  `at (joyous): Yes.` remain spoken lines.
- **`and`, `or`, `not`, `true`, `false`, `none`** are reserved in calculations only;
  **`as`** only after `&`.
- **`name = species`**: before the first scene, `NAME = NAME` (a single name on the right) gives a name
  to a Pokémon. Everywhere else, it is an assignment.

## 12. The games and the places

The same language reads a game's file (`<name>.game`, `game/src/game_file.rs`): a head of
`key: value` lines (`title:`, `author:`, `team:`, `home:`), then chapters. A chapter opens at the
start of a line, `chapter 2 "Its name"`, and closes with `end`; `chapter 3` indented is the
action that goes on to chapter 3. In a chapter: actions, one a line (`play`, `set`, `clear`,
`add`, `chapter`, `go`, `dungeon`, `fight`, `open`, `give`, `take`, `money`, `shop`, `map`,
`finish`), `if … else if … else … end` with the tests of the films (`chapter >= 2 and not met`:
the flags are 1 or 0, `won` how the last dungeon ended), and rules: `on enter`, `on talk
<resident>`, `on zone <name>`, maybe `if <test>`, their actions, `end`.

A place's `on … end` (after the `[life]` of its decor) holds the same actions; its `resident … if
<test>` and `says if <test>: …` the same tests.

```
game         = { head_line } { chapter } ;
head_line    = NAME ":" rest ;
chapter      = "chapter" NUMBER [ TEXT ] (at the start of a line) { action | rule | if } "end" ;
rule         = "on" ( "enter" | "talk" NAME | "zone" NAME ) [ "if" expr ] { action | if } "end" ;
action       = WORD { argument } ;
```
