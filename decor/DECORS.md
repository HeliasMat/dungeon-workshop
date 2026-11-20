# Making a place

> **Since 2026-11-15, the Workshop only offers what the ROM itself keeps apart**:
> - the objects of the upper layer `e<level>_<n>` (their transparent pixels really are), with nothing of the ground layer;
> - the animated tiles of the ground layer `a<level>_<n>` (BPA tiles, cycling palettes: windmill, flowers, water);
> - the objects laid by the scenes `o<n>` (chests, signs…), their first frame.
>
> The rest of each place `r<level>` stays in the catalogue, backstage, to make a place of the game again.
>
> The pieces of before were removed, and the sections below that speak of them describe the tool of before:
> - `pieces.txt` is empty;
> - no more grounds, edges or paths to weave;
> - no more things `l…`, whole places `l…_tout` or scene objects `o…`.

A place is written like a film: a short file, `library/decors/<name>.decor`, one idea per line. The tool builds it with the elements of the game's places; no pixel is drawn, no built image goes into git (everything goes into `data/extra/`). Then the place plays in a film by its name: `scene: my_village`.

There is only one kind of place, the `.decor`, and a single editor, the Workshop's Decors tab. Dungeon rooms are decors too (`base dungeon`), and a place of the game can be taken as it is (`base level`, `decor import`).

```
# The village well: what one sees there, where one comes from.
decor: village_well 600 400        # any size, from 16 to 4096 pixels
level 496                          # fixed at its birth (decor new, "New Decor")
grid 16                            # the step of the Workshop's grid
name The Village Well
base level 323                     # underneath, as it is (or: base dungeon Beach Cave 3, then layout)
ground herbe_bourg
put tente_bourg 13 27              # to the pixel; layer objects when nothing is said
put arbre_bourg 300 90 layer over
put sapin_crepuscule 120 200 foot  # by its foot (where it touches the ground): trees in rows
put e224_8 256 216 still           # a windmill of the game that does not turn: its first frame only
image rowboat.png 40 50 layer ground  # an image of mine (library/images/), never enlarged
spot well 120 80                   # a spot for the films, with nothing laid there
entrance 300 384 down
wall 0 0 64 32

[life]                             # the life of the place, after what is seen
music 12
resident eevee 200 220 down wanders 32
says Welcome to the well!
resident snorlax 320 200 anim Sleep   # what it does endlessly: Sleep, Sit, Eat... (still: its first frame)
exit 584 180 16 40 to 224
```

After `[life]`, only `music`, `resident`, `says`, `exit`, `link`, `departure` (before it, they are still read, as in the places of before).

A resident says after its feet, in any order:
- where it faces (`down`, `right`…);
- `wanders n`: it strolls up to n pixels from its place;
- `anim <animation>`: what it does endlessly, by the game's name (`Sleep`, `Sit`, `Eat`…);
- `still`: motionless on the first frame of that animation (lying down, asleep).

The game and the films play it as it is (`actor … <animation> <still>` in `decor_town.txt`).

**Laying**: every piece is laid by its **top left corner**, to the pixel, whatever its kind (a tree like a rock); that is what the Workshop writes. `foot` at the end of the line lays it by its foot (the point where it touches the ground: the piece's `foot=` in `pieces.txt`, otherwise the middle of its bottom), handy to line trees up in rows. Only `ring` lays its trees by their foot, on their grid.

Three layers: `ground` (under everything), `objects` (the piece as it is: its top in front of whoever passes; when nothing is said), `over` (in front of whoever passes). Within a layer, the next line goes in front. Pixel elements are never enlarged; zones and what repeats are.

The game does not keep its decor elements apart: one of its places is a mosaic of 8-pixel tiles on two layers, and nothing in it says where a tent ends. Each element is therefore **cut** out of a place of the game and **matted** from its ground. That is all the tool's work, and what is checked the most.

## Two builders

**The simple builder** (`build/compose.rs`) builds any decor made only of the game's pieces: those the ROM keeps apart (`e…`, `a…`, `o…`) and the rests of places (`r…`).
- **What it reads**: only the lines `put` (flipped, still, without shadow, on a layer, named), `ground <piece> [zone x0 y0 x1 y1]`, `terrain`, `wall` and `free`.
- **What it does**:
  - each line is laid in order, without guessing anything;
  - each piece is cut once and then kept (`laid_cut`: the rest of a place is hollowed of its elements);
  - a ground is its piece **repeated** tile after tile over its zone (no more weaving);
  - walls painted by hand have the last word.
- **What it does not do**: no guessed remark (overlaps, shadows…).
- **Result**: a place of the game made again of its pieces stays identical to the pixel (tested on the Crossroads and Treasure Town). The great Treasure country (4000 × 2000) is built and written in about 6 s, against 19 s before.

**The full builder** (`Builder`) takes the rest: a place of the game as a base, edges, paths, water, scattered things, our own images, an element animated our way, dungeon rooms.

`decor::built` chooses by itself: the simple one if the decor says nothing but those lines, the full one otherwise.

The new Workshop (`workshop/`) draws a decor itself, piece by piece, while it is edited. Building only serves there for baking for the films, done in the background once the text is saved.

## Painted terrains

A terrain is painted as in game editors: grass, path, sand, water… with the very tiles of a place of the game (`decor/src/terrain.rs`).
- **What is learnt from a place**, once, without the ROM:
  - its 8 × 8 tiles, all their frames (animated tiles too), and the palette of each pixel (the colours that change with time);
  - the same tiles flipped, as the game flips its own;
  - its **terrains**: the colours of tile quarters grouped together (a the most seen, b the next…, 16 at most); each tile is filed by the terrain of its four corners;
  - who is next to whom in the game, and how often;
  - the purity of each tile: a tile that carries an object (the border of a rug, a sign) or a hole is almost never chosen;
  - the walls: a terrain where the place almost never lets one walk (its water, its cliffs).
- **What is painted**: the grid of corners, one letter per corner.
- **What is chosen**: for each tile, a tile that has those four corners, drawn according to how often the game lays it after its left neighbour and under the one above (a Markov chain). The draw is the same at each build.
- **Where one walks then**: wherever the terrain allows; `walls` says otherwise.

```
terrain 323 0 0 40 28 a18d4a19/a18d4a19/.../c14a5d4a18 walls ch
```
- **The line**: `terrain <level> x y columns rows <corners> [walls <letters>|none]`.
- **The corners**: (columns + 1) letters per row and (rows + 1) rows, separated by `/`. A letter followed by a number repeats (`a18`: eighteen a). `.` means nothing.
- **Where it is laid**: a terrain is laid on the ground, under what is laid after it.
- **`decor terrains <level> [letters]`** tells the terrains of a place (their letter, their colour, their share, those that are walls). It also writes:
  - `terrains_<level>.png`: the place made again from its own corners, beside the original;
  - with letters, each combination of corners of those letters and its tiles.

**In the Workshop** (`cargo run -p workshop`), Decor space, five tools (toolbar, or the keys V, B, N, M, P):
- **Walk**: a Pokémon (Pikachu when nothing is said) starts from the first entrance. It walks with the arrows (or WASD), stopped by the walls of the last bake, and the scene follows it; a click puts it elsewhere.
- **Dragging** snaps to the edges and middles of the other elements and of the decor (a pink line shows it; Alt: the grid of 8 instead).
- **The Hierarchy** is made like the VS Code explorer:
  - two folding sections, LAYERS and ELEMENTS; hovering ELEMENTS: search (also Ctrl+F), fold all, group what is chosen;
  - a tree: groups as folders (chevron, click: open or folded), their elements indented, with indent lines;
  - after each name, in grey: its position, its number of points, its spot name;
  - the eye and the padlock only show on hover, or when they are on;
  - the list takes the keyboard once clicked: ↑ ↓, ← (fold, up to the group), → (unfold), F2 or Enter (rename in place), Del, F;
  - renaming changes the name of a group, a spot or an entrance, a resident's Pokémon, or the spot name at a piece's foot;
  - a click in the empty space under the list chooses nothing any more.
- What it lists, the most forward at the top:
  - one badge per kind: T terrain, O object, A animated tiles, F rest of a place, C scene thing, H resident, R spot, E entrance, S exit, G group;
  - a search;
  - on hover, the element is framed on the scene; what is chosen on the scene scrolls to its line;
  - double-click or F: the scene frames it; ↑ ↓ go from one line to the other; Del removes;
  - dragging a line onto another puts it just in front (the drawing order), or into a group;
  - right click: Frame, Duplicate, In Front, Behind, Out of the Group, Group the Chosen Elements, Remove.
- **The Inspector** is made of folding components, as in Unity: a column of labels, an arrow that resets the value, and at the top what the bake says of the line (remarks, errors).
  - **A piece**:
    - Transform: each point of the line ("+ a point", a point removed, "on the grid"), its size, flipped;
    - Rendering: layer, animated, shadow, locked, hidden, the spot at its foot;
    - Order: behind, in front, all the way behind, all the way in front;
    - Line: the line itself, written by hand, kept only if the decor still reads (Ctrl+Enter).
  - **A resident**: its feet, its Pokémon, where it faces on a rose of the eight directions, its stroll (`wanders n`), what it says (the `says` lines that follow it, added, changed or removed). Its frame on the scene fits its pixels over its whole idle animation.
    - Animation: each animation its sprite has, animated and turned as it is, chosen with a click ("default": it waits as the game makes it wait); looped or still. The scene shows it so at once.
    - What it says: each line its own "only if…" test (`says if not met: …`), empty: always.
    - In the Game: **There Only If** (`resident … if chapter >= 2`), **Talked To** (only speaks, plays a film, opens the shop, market, bank, storage, jobs or recruits: `talk …`), its **`on talk <name>` … `end`** as a small text field (the actions of a `.game`; written when the field is left, if each `if` has its `end`; emptied: taken out).
  - **A marker**: its name, its place, the size and the destination of an exit: a game place by its number, or a decor by its name and one of its entrances (`exit … to my_harbour left`).
  - **A zone** (`zone name x y w h`, Add › Zone; drawn pink on the scene, dragged like the markers): its name, its place, its size, and its **`on zone <name>` … `end`**.
  - **Nothing chosen**, the **Life Rules**: each `on … end` of the place (`on enter`, `on talk …`, `on zone …`) as a small text field; **+ on enter**.
  - **Several elements**:
    - what they share (layer, flipped, shadow, locked, hidden; a half-ticked box when some have it);
    - align, spread at equal gaps across or along (three at least);
    - group, save as a template.
  - **Nothing chosen**: the decor itself (name, size, grid, level, music, seed), what it holds, what can be added to it.
  - A value one drags makes only one undo step (the changes of the same thing less than a second apart merge).
- **The Scene** is a studio viewport:
  - rulers at the top and on the left, in pixels of the decor, the pointer marked on them (R shows or hides them);
  - the outside of the decor darkened, its edge and its size; very close (800 % and more), the grid of pixels;
  - at the bottom left, a bar: −, +, the zoom, All, 1:1, and the position of the pointer in pixels and in tiles of 8;
  - with the keyboard, the pointer over it: 0 the whole decor, 1 its pixels as they are, 2 twice, + and −, F frames what is chosen; Space held, dragging moves the view (like the middle button);
  - several chosen: their frame together and its size; Alt while pointing at another element: the distance between them, in pixels;
  - right click on an element: Copy, Duplicate, All in Front, All Behind, Layer, Group, Remove; on empty space: Paste Here, a spot, an entrance, a resident or the piece chosen in the Library, laid there, and The Whole Decor.
- **The Library** is a resource browser:
  - one card per element: the same element in several places (to the pixel, turned over or not) is one card, its places said on hover; an old name laid in a decor still lays it;
  - on the left, where one looks: Favourites, Recently Laid, All, then each category with how many (Nature, Water, Buildings, Furniture & props, Animated, Scene objects, Other: `library/tables/piece_tags.txt`), Models; then the game's places grouped by kind (Town, Beach & sea, Forest & grove, Cave & mountain, Guild & interiors, Dungeon entrances, Other: `library/tables/place_kinds.txt`), each folding, each place its picture and its name (ours for those the game leaves unnamed), a search filters them;
  - at the top: where one is, the sizes shown (Small up to 32 pixels, Medium up to 96, Large), a search (name, tag, place or id; the name's start first, then a whole word or tag, then the places), Grid or List, the size of the images;
  - each card says its name (`big oak`) and the place it comes from (`Treasure Town +2`: in two more); the animated ones play;
  - a click chooses a piece (the bottom line names it; Scatter takes it), a double-click or Enter lays it in the middle of the scene, a drag lays it where it is dropped; while it is dragged, the scene shows it in its place;
  - on hover, it shows large and animated, with its category, size, tags, its place and the others it is in; its star puts it in the Favourites;
  - the Favourites and the last pieces laid are kept on this machine (`target/workshop/library.txt`), not in the project.
- **Groups** (`group name` … `end`) show in the Hierarchy: folded or unfolded, shown, hidden or locked at once, chosen with a click, undone ("ungroup"). Several chosen elements are grouped from the Inspector.
- **The History** (a tab beside the Hierarchy) tells each change (added, removed, moved, changed). A click goes back there, or redoes what was undone.
- **Choose**: take and move the elements, the residents and the markers (spots, entrances, exits).
  - Click: one; Ctrl or Shift + click: one more; a rectangle drawn on empty space: all it touches; Ctrl+A: all.
  - Dragging moves all that is chosen, to the pixel (Alt: on the grid of 8); the arrows by one pixel (Shift: 8).
  - Ctrl+C, Ctrl+X, Ctrl+V (at the pointer), Ctrl+D (beside), Del, Esc.
  - Several chosen: the Inspector aligns them on the first (left, center, right, top, middle, bottom), duplicates them, removes them, or **saves them as a template** (`library/decors/templates/<name>.template`, lines laid from 0 0), to lay again from Library › Templates (click: in the middle; drag: where it is dropped).
  - Nothing chosen: the Inspector adds a spot, an entrance, an exit or a resident in the middle of the view; a chosen resident takes its facing there, a marker its name, its size and the place it leads to.
- **The layers** (at the top of the Hierarchy, from below to above): Terrains, Ground, Objects, Residents, Over, Markers, Walls.
  - Each is shown or hidden, locked or not. Alt+click on the eye: it alone. Its opacity can be set (in the Workshop only).
  - The **active layer** (a click on Ground, Objects or Over) receives what is laid from the Library or by scattering (`layer ground`, nothing, `layer over`).
  - Hovered, a layer frames its elements on the scene; double-click: all chosen.
  - Right click: Choose All Its Elements, List Only It (the list filtered), Show Only It, Active Layer, Opacity 100 %.
  - A line of the list dropped on Ground, Objects or Over moves there.
  - Right click on a piece: Layer › Ground / Objects / Over, All in Front (drawn after everything), All Behind (before everything laid, after the terrains).
- **The grid** (Grid button or G) has the decor's `grid n` step. A **map** of the whole decor shows at the bottom right when it does not fit in the scene; a click on it goes there.
- **Walls**: left click walls the tiles, right click frees them, Shift+click gives them back to what is laid (`wall tiles`, `free tiles`); the walls of the last bake are pale red.
- **Terrain**: the Inspector shows the terrains of the chosen place, a square of each (the walls framed in red).
  - Brush, Rectangle or Fill (what is alike and touching, up to the edges of the decor); left click: paint; right click: erase; Alt+click: the eyedropper; the size of the brush is set in corners.
  - The colours that change with time (the water) play once the stroke is finished.
  - The walls show in red while one paints.
  - "Start from the place's ground" turns the whole ground of a place of the game into terrain, to paint over.
  - A terrain chosen in the Hierarchy tells in the Inspector which terrains are walls; a box ticked or unticked writes `walls`.
- **Scatter**: a piece chosen in the Library is scattered under the brush, never one on another; the radius and the gap can be set. Each stroke becomes a `put` line of all its points.

## The commands

`decor` is the builder of places (`decor/`, in Rust): `cargo run --release -p decor -- <command>`, or `target/release/decor.exe` once built.

```bash
# Places
decor help                         # the commands and every word of a .decor (made of the very tables decor/src/lang.rs reads)

decor new my_clearing 512 384      # a starting place to complete (its level fixed)
decor import 323 my_square         # a place of the game under a new place, to make one's own
decor recreate 224 my_crossroads   # a place of the game made again of pieces alone (its rest r224, its elements): identical, each one can be moved
decor resize my_square 640 400 center   # its size, from one of the 9 anchors
decor migrate                      # the places of before written as now, built the same
decor build my_village      # builds it, says what is wrong (--json: one line for an agent)
decor view my_village       # and its sheets: <name>.png, _walls.png, _side_by_side.png, _grid.png
decor grid my_village       # the image with its grid (grid n, otherwise 16), what it lays named
decor all                          # builds again every place and room (after an extractor)
decor overview six_decors          # every place on one sheet
# (The Workshop, cargo run -p workshop, makes them with the mouse too: the Decor space, Elements on the left (the element chosen,
#  its animation, Add, In the Decor, The Place), the scene on the right and its modes Elements, Grounds, Paths,
#  Collisions (painted tile by tile), Markers; it builds them at each change and plays them animated.)

# Elements
decor pieces [word] [--backdrops]  # the catalogue by kind and by place; --backdrops: on black and on white
decor element tente_bourg          # an element, large, on black and on white, its grid of tiles
decor element pots_bourg pont_bourg   # several side by side
decor check [word]                 # what is left of their ground, the worst first
decor grounds [word]               # each ground woven on a square: no word, those named; s52: those of place 52
decor reassemble 323               # the elements of a place of the game put back in their place, on black
decor grid 323 0 160 120 240       # a place of the game (or a rectangle) and its grid
decor inventory [level ...]        # finds again the elements of all the game's places
decor sort                         # what the Workshop offers: the elements made again of quality, one of each, turned over or not (set_aside.txt)
decor labels target/labels e       # the elements offered with no line in library/tables/piece_tags.txt, on sheets to label them by eye
decor labels target/places places  # the game's places with no line in library/tables/place_kinds.txt, on sheets
decor sort e224_8 e323_1           # how these pieces are judged (size, outline, holes, lightness)
```

The sheets go into `target/decors/`. Exit codes: 0 done; 1 a place or a piece at fault, the file and its line said, with the closest names; 2 the command badly said. The tests: `cargo test -p decor`.

## What the Workshop offers: the sort

`decor sort` writes `library/tables/set_aside.txt`: each piece the Workshop does not offer, with the reason. The catalogue stays whole: a decor still lays a piece by its name, and a place of the game is made again of all its pieces.

The Workshop only offers the pieces made to remake the game's places:
- the whole elements `e…`;
- the ground of each place `r…`;
- in addition, the grounds, paths and edges to paint, which are tools.

All the pieces of before are set aside, as are those of the screens that are not places (menus `S99…`, visions `V…`).

Among the pieces made again, the sort sets aside:
- **the scraps**:
  - too small (less than 64 pixels);
  - in crumbs;
  - cut by their frame;
  - squares of ground: an edge on the grid of tiles, or more than 0.68 of a whole tile side per tile around;
  - hollow (less than 30 % of their rectangle drawn);
  - bits of landscape (more than 256 pixels a side);
  - ragged (outline more than 15 times that of a disc of the same area);
  - too dark (mean lightness under 60);
  - holed (holes over more than 6.5 % of the piece);
- **the duplicates**: identical to the pixel (`duplicate of`), or to the pixel once turned over (`flipped duplicate of`): the Workshop offers them as one element, with every place it is in, and a decor that lays one by its old name still lays it (the catalogue stays whole). The first kept is the one of the place with the highest number: the first places are drafts of the next ones.
- **the almost alike** (`almost like`: the same size within 4 pixels, and almost the same 16 × 16 mark, the tent of another time of day) stay offered, labelled as the one they are like.

## What our eyes say of them: categories, names, kinds of places

`library/tables/piece_tags.txt` (written by hand, by looking at `decor labels` sheets): each element offered, `id | category | name | tags`, the categories `nature`, `water`, `buildings`, `furniture`, `animated`, `scene`, `other`. The animated tiles `a…`, the elements the game animates and the scenes' things `o…` go in Animated and Scene objects whatever their line says; an element almost like another goes by that one's line. `library/tables/place_kinds.txt`: each place of the game (by its level) and each decor (by its name), `place | kind [| our name]`, the kinds `town`, `beach`, `forest`, `cave`, `guild`, `dungeon`, `other`; our name for a place the game leaves unnamed. Read by `decor::tags`; `cargo test -p decor -- --ignored every_element_and_place_is_labelled` says what lacks a line, or a line that names nothing.

The sort takes about half a minute: it cuts each piece, place by place, forgetting each place after use. Then `decor sort <piece> ...` tells how precise pieces are judged.

## All the game's elements: the inventory

`decor inventory` finds by itself each element of each of the game's 459 places: the rare tiles, of colours that no ground has nor those the catalogue names in that place (the edge of a path is not a thing), joined, and one tile around. In a place painted in one piece (the inside of the Guild), the blocks of standing walls in what one walks through too. It writes `data/extra/game_pieces.txt` (at extraction, from your own copy), read with `library/tables/pieces.txt`, which wins for the same name: `l<level>_<n>` (the n-th thing of the place, from top to bottom), `s<level>_<n>` (one of its grounds, the largest first), `l<level>_tout` (the whole place), `o<n>` (a scene thing). Each thing there is `alone` and `on=place`.

**The grounds of all places**: the tiles a place shows at least four times, side by side with those that share at least half their colours, make a ground; the patches of one ground cut by a path join up. For each, the largest rectangle that has only it, and a second one (`also=`): a ground to weave, `ground s52_1`. A ground that covers less than a twentieth of the place (a waterfall, an edge), a flat area of almost a single colour (the screen of a vision; a plain earth like that of lava is named by hand) or the black around a place are not grounds. 253 places give 433; `decor grounds s52` shows them woven, to choose. A ground that is used is named in `pieces.txt`, with its shadows (`shadow=`) so that things are matted from it. `inventory 323` only redoes that place.

**A place of the game made again of pieces alone** (`decor recreate <level> <name>`, or in the Workshop: "New Decor", "A place of the game, element by element").
- **What it writes**: no base. First `put r<level> 0 0`, then a line `put <element> x y shadow off` per upper object (`e…`), at the exact place where the game draws it.
- **The rest, `r<level>`**: a piece of the catalogue (`base upper rest`), backstage. It is the place without its upper objects: its whole ground layer, and under what the upper layer covers in all its frames, the ground comes back, tile by tile, from the nearest ground tile.
- **The proof**: the place laid this way is the game's to the pixel, in each frame and on both layers. Each piece is therefore cut exactly: its pixels, its frames, its place, nothing lost or doubled.
  - Checked on 20 places, and by the test `a_place_of_the_game_remade_of_its_pieces_is_the_same` (Crossroads, Treasure Town).
  - That does not say a piece is really "a single object": two neighbours stuck together stay one piece. That is judged by eye (`decor element`).
- **Then**: an element moved or removed leaves ground in its place.
- **Limits**:
  - The game's shadows stay in the rest (`shadow off`) and do not follow a moved element.
  - The walls are those of each piece.
  - A place with no element found only has its rest.
- **`base level n hollow`** does the same with the place of the game as a base rather than a piece.

**What the ROM keeps apart itself** (`decor inventory`, in `data/extra/game_pieces.txt`, made again at extraction, never written by hand; `decor sort` sets aside specks of less than 64 pixels, menus and visions, duplicates to the pixel or almost).
- **The upper objects**, `e<level>_<n>` (`over upper element=x,y [moves]`): each bit of a place's upper layer (what one passes under: a roof, a windmill, flowers), seen in any of its frames; bits within 3 pixels go together, touching objects are separated (`pixels::objects`: cores worn by 2 pixels, each pixel to the nearest, glued back if they touch with the same light colours). Nothing of the ground layer: its transparency is the game's. `element=x,y`: one of its pixels, the cut keeps only that object. `moves`: the game animates it, it is laid with all its frames.
- **The animated tiles**, `a<level>_<n>` (`base upper tiles=c,r;… moves`): the tiles of the ground layer that change from one frame to the next (BPA tiles, cycling palettes), side by side. Only those tiles, with all their frames: a river, a waterfall, flowers.
- **The scene objects**, `o<n>`: each object the scripts lay (`props.txt`), its first frame, cropped to what it draws.
- Larger than 384 pixels a side: a stretch of the place, not offered.
- **In the Workshop**: "Add", chip "Places of the game" (a place opened: its objects and its animated tiles), "Animated", "Scene Objects".

## What the Workshop writes too

- `put rocher_plage 8 8 name big_rock`: a spot for the films at its foot (`put pikachu big_rock`), written with the entrances; `spot big_rock 40 60`: the same, without laying anything.
- `… hidden`: the line kept, the thing not laid; `… locked`: the Workshop's mouse does not take it (nothing for the place).
- `group name` … `end`: lines that go together in the Workshop; nothing for the place.
- `wall x0 y0 x1 y1`, `free x0 y0 x1 y1`: the tiles touched, walled or left free, whatever was laid before (in the order of the lines).
- `wall tiles 3,4 5,4 …`, `free tiles …`: tiles of 8 (column, row) walled or left free, painted in the Collisions mode. It is the last word, after everything laid, whatever the order of the lines.
- `… anim <kind> [slow|normal|fast]` (after `put`, `water`, `image`, `scatter`): the element lives like the game's animated places, in whole pixels.
  - The kinds: `wave` (its rows, by one pixel), `float` (it rises and sinks), `sway` (its top, in the wind), `glow` (it lights up and darkens), `twinkle` (points of light), `colours` (its colours cycle), `scroll` (what is inside it flows down), `blink`.
  - The speed: 24, 12 or 8 frames of 0.1 s per turn. The place takes as many frames as it needs, 24 at most.
  - The element keeps its corner. Those that move have two pixels of margin around them, and the colours the game cycles no longer cycle.
- `ground sable_plage tiles 3,4 4,4 …`: a ground painted tile by tile (column, row of 8 pixels), woven without a seam like the others.

## An element case by case

1. **Find it**: `decor reassemble 323` (the elements of the place on black, with their names) or `pieces l323 --backdrops`.
2. **Note its rectangle**: `decor grid 323 x0 y0 x1 y1`, the game's grid, large (a line every 8 pixels, a magenta one every 32, with its coordinate). The rectangle follows the 8-pixel tiles.
3. **Name it** in `pieces.txt`, in the section of its place, with `on=` the ground of its place (or `on=place`).
4. **Look at it**: `decor element <name>`: large, on black and on white, its grid of tiles numbered. What is left of its ground shows on one background or the other, a hole on both.
5. **Correct it**, and start again:
   - a neighbour caught in the rectangle (a tuft, a rock): `without=` its tiles;
   - a rim of grass or a shadow that stays: its colours in the ground's `shadow=` if they are the ground's everywhere, otherwise in the piece's `shadow=`; `replace=c>-` if they appear nowhere in the thing;
   - a hole in the thing, a part as light as the grass: `keep=`;
   - small detached bits that belong to it: `whole`;
   - a thing caught among others (a river): `in=` its outline.
6. **Check it in a place**: `decor view <place>`, then `cargo test -p decor`.

`decor check` measures, for each named element, the share of its outline made of the colours of its ground and its holes, the worst first. It is a guide, not a judge: a thing whose foliage has the colours of the grass (a bush of the village) looks wrong there; the `element` sheet decides.

## What it remarks

A piece that overlaps another (two waters join without a remark), laid under a tree or on a path, or that goes past the edge (laid at the pixel said, never moved back); an element found on its own laid by its number; a path that ends without meeting anything; an entrance on a wall; free tiles that cannot be reached from the edges or from an entrance (a gap between trees only from 12 tiles); a seam (two woven tiles that touch badly); the **visible joins**: two things laid side by side whose colours never touch in the places they come from, along a line of the grid over 6 pixels at least (a square of another ground, a cut edge), grouped together. `view` marks them: walls in red, seams in cyan, joins in magenta (framed), entrances in yellow.

## How it is made

- **Matting** (`matte.rs`, `cut.rs`): the thing is compared, tile by tile, with the tiles of its grounds (`on=`, and the plain tiles around it). What is that ground, or a colour of that ground's own, goes away; what is one of its shadows (`shadow=`) keeps only how much darker it is; the rest is the thing. Only what touches the edge through ground or shadow goes away: a grass green in the middle of a bush stays the bush. Then its outline in the ground's colours is peeled down to its line, its holes filled if they are not ground, the crumbs of other things caught by the rectangle removed, the small things of the ground (`crumb`) erased where they lean on it. No colour threshold.
- **Shadows**: a shadow always darkens (the lightness of the grass around a thing stays in its place: laid elsewhere, it would be a pale square). It is laid on the ground of the new place, pixel by pixel: on the ground it comes from, with the game's colour; on a ground that has its shadows (`shadow=`), with the one of them that is as dark; elsewhere, the ground darkened by as much, in DS colours.
- **Weaving** (`ground`, `edge`, `weave.rs`): each 8-pixel tile takes a tile of the source place, the one whose edges best meet what surrounds it, preferably the one that follows its neighbour in the game: large pieces of the game are copied over.
- **Paths**: a line rounded at its turns; each source tile knows, pixel by pixel, what is inside and what is its fringe; each tile takes the source tile of the closest shape (turned if need be), then of the best edges. The ground around the path in its source is matted like a thing.
- **Bands** (`edge`): whole columns of the source: the sea keeps its waves, its foam, the wet sand.
- **Frames**: animated places stay so (as many frames as the source, the game's cycling colours with their palette).
- **Walls**: the game's grid under each piece when the game has one, otherwise its pixels; a tile wholly covered takes what covers it says, otherwise it is a wall if a third of it is. Written in `data/extra/decor_town.txt` and `decor_ground.txt`, read over `town.txt` and `ground.txt` (see `data/DATA.md`). The entrances go into `decors.txt` after the name (`x,y,name;...`).

The code, in `decor/src/`: `main.rs` (the commands), `lang.rs` (the language), `build/` (laying: `place.rs`, what is laid; `mod.rs`, line after line), `remarks.rs` (what it remarks), `catalogue.rs` (the catalogue), `cut.rs` (cutting), `matte.rs` (matting, shadows), `weave.rs` (weaving, a place's grounds), `source.rs` (the game's places), `write.rs` (what is written in data/extra), `inventory.rs` (the inventory), `room/` (dungeon rooms, floors made as the game makes them), `sheets/` (the sheets: of places, of elements, the check).

## A dungeon room

A `.decor` whose base is a floor of the game: `base dungeon <dungeon> <floor>` (name or number of the dungeon), then `layout` and its rows, one letter per 24-pixel tile, tiles chosen as the game chooses them (according to the ground and like neighbours): `#` wall, `.` ground, `~` the floor's second ground (water, lava), a small letter for a spot with that name (`put pikachu a`, `stairs e`). The layout goes up to `[life]` or the end; one lays on top as elsewhere (the lines before `layout`).

```
decor: my_cave_room 576 384     # 24 columns, 16 rows of 24 pixels
level 461
name Beach Cave
base dungeon Beach Cave 3
layout
#########n##############
###.......o........#####
```

(The old `.room` files become these `.decor` with `decor migrate`, built the same.)

```bash
target/debug/scene_player.exe dungeons           # once: the tileset of each floor, the stairs and traps as images
decor dungeons cave                              # the dungeons and their floors
decor view my_cave_room                # builds it, and its image in target/decors/
decor all                                        # builds again every place and every room
decor generate bois_etage Apple Woods 6 13       # a whole floor, made as the game does (seed 13)
```

`generate` writes a `.decor` (`base dungeon`) with the layout of a whole floor made as the game makes them: rooms in a grid, a few empty cells, one-tile corridors joining them (and sometimes a loop), sometimes a pool; `s` the arrival in the first room, `e` the stairs in the farthest, `p q r` traps, `o t` items, sometimes `k` a shop (where the shopkeeper stands) and `m` a Monster House, each in its room, `a b c...` the middle of the other rooms. The same seed, the same floor. The layout written can be touched up by hand afterwards. Already made: `foret_brumeuse_etage`, `grotte_cascade_etage`, `plaines_elek_etage`, `falaise_trempee_etage`, `grotte_etuve_etage`, `mt_corne_etage`, `bois_aux_pommes_etage`.

It is written into `data/extra/` like the other places, and plays the same way: `scene: my_cave_room`.

Water and lava move there as in the game: their palettes cycle (palette 10 of each tileset, sometimes 11), according to the colour each pixel shows. The extraction writes them beside each tileset (`data/rom/tilesets/<id>_i.png`, `<id>.cycles`); an older extraction does not have them: `extractor <rom> --tables-only` adds them without changing anything else, then `decor all`.
