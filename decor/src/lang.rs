//! The language of places (`library/decors/<name>.decor`): one idea a
//! line, errors with their line and the closest word. Described in
//! decor/DECORS.md.

use diagnostics::text;

use crate::fault::{Diagnostic, Location};

/// The words of a `.decor`, and what each says.
pub const COMMANDS: [(&str, &str); 33] = [
    (
        "base",
        "base level n [x y] [hollow] | base dungeon <dungeon> <floor>: what is underneath, as it is (a place of the game at this corner, or a dungeon floor made of its layout); hollow: without its elements, their room filled with its ground (they are laid by their own lines)",
    ),
    ("layout", "layout: the rows that follow, one letter per 24-pixel tile, make the dungeon base (# wall, . ground, ~ water or lava, a small letter: a spot)"),
    (
        "ground",
        "ground <ground> [zone x0 y0 x1 y1 | tiles c,r c,r ...]: the ground of the whole place, of a zone, or of these tiles (column, row), woven without a seam",
    ),
    ("edge", "edge <edge> [top|bottom|y]: a band across the whole width, the sea and its foam"),
    ("path", "path <path> from x y to x y [to x y ...] [width n]"),
    ("water", "water <piece> x y: water laid down, where no one walks"),
    (
        "terrain",
        "terrain <level> x y columns rows <corners>: painted terrains (grass, path, sand...) from the very tiles of the game's place n, each tile chosen from its four corners and its neighbours in the game, its walls with it; the corners: (columns+1) letters per row, (rows+1) rows separated by /, a the terrain most seen in the place, b the next..., . nothing, a12 twelve a",
    ),
    ("ring", "ring <tree> [oval cx cy rx ry]: trees all around, on their grid"),
    (
        "put",
        "put <piece> x y[, x y ...] [flip] [shadow off] [layer ground|objects|over] [foot]: its top left corner, to the pixel, whatever its kind (a tree too); foot: x y is its foot (where it touches the ground), for trees in rows",
    ),
    ("image", "image <png> x y[, x y ...] [flip] [layer ground|objects|over]: an image of one's own (library/images/), to the pixel, never enlarged"),
    ("scatter", "scatter <piece> n [zone x0 y0 x1 y1] [flip] [shadow off]: n small things at random, never on a path"),
    ("entrance", "entrance x y [name]: where one comes in, in the films, on a free tile"),
    ("spot", "spot name x y: a spot for the films at this point, without laying anything there (put pikachu name)"),
    ("level", "level n: its number (otherwise the first free one)"),
    ("name", "name words: its name in the game"),
    ("grid", "grid n: the step of its grid in the Workshop and on `decor grid`, in pixels (8 if not said)"),
    ("center", "center x y: where a group stands in the films"),
    ("seed", "seed n: the chance of `scatter`"),
    ("like", "like n [n ...]: the game's places to show beside it in `view`"),
    (
        "resident",
        "resident <pokemon> x y [facing] [wanders n] [anim <animation> [still]]: someone who lives there, standing or strolling up to n pixels from their place; anim: what they do endlessly (Sleep, Sit, Eat…), still: motionless on its first frame",
    ),
    (
        "says",
        "says [if test:] words: what the resident above says when spoken to (several: one after the other); if: only while the test holds (says if not met_kecleon: Welcome!)",
    ),
    (
        "talk",
        "talk film <film> | talk shop|market|bank|storage|jobs|recruits|appraise|cafe: talking to the resident above plays that film of the game, or opens that screen, after what it says",
    ),
    ("zone", "zone name x y width height: a zone of the place; walking into it plays the game's `on zone name`"),
    (
        "on",
        "on enter | on talk <resident> | on zone <name> [if test] … end: what the game does then (play, set, chapter, go, dungeon…: the actions of a .game)",
    ),
    ("music", "music n: the number of the place's music in the game"),
    (
        "exit",
        "exit x y width height to n | to <decor> [entrance]: a zone that leads to the game's place n, or, in a game of one's own, to that place (a decor of the project) by that entrance",
    ),
    ("link", "link n x y width height: from the game's place n, a zone that leads here"),
    ("departure", "departure: one goes there from the Town's crossroads, as into a dungeon (and `exit … to 224` to come back)"),
    ("wall", "wall x0 y0 x1 y1 | wall tiles c,r c,r ...: no one walks there, whatever is there (the tiles it touches, or these 8-pixel tiles: column, row)"),
    ("free", "free x0 y0 x1 y1 | free tiles c,r c,r ...: one walks there, whatever is there"),
    ("[life]", "[life]: what follows is the life of the place, not what is seen there: music, resident, says, talk, exit, zone, on, link, departure"),
    ("group", "group name: the lines up to the \"end\" go together in the Workshop (moved, hidden, locked together)"),
    ("end", "end: the end of a group"),
];
/// What a place's life says (`[life]`, after what is seen).
pub const LIFE: [&str; 9] = ["music", "resident", "says", "talk", "exit", "zone", "on", "link", "departure"];

/// The ways a resident faces.
pub const FACING: [&str; 8] = ["down", "downright", "right", "upright", "up", "upleft", "left", "downleft"];
/// What may follow a piece laid.
pub const MODIFIERS: [(&str, &str); 9] = [
    ("flip", "flip: flipped from left to right"),
    ("shadow", "shadow off: without its shadow"),
    ("layer", "layer ground|objects|over: on the ground, as the piece is (its top in front of whoever passes), or all in front of whoever passes"),
    ("name", "name word: a spot for the films, at its foot (put pikachu word)"),
    ("hidden", "hidden: not laid (the line kept)"),
    ("locked", "locked: the Workshop's mouse does not pick it up"),
    ("still", "still: motionless, its first frame only (a windmill of the game that does not turn)"),
    (
        "anim",
        "anim <wave|float|sway|glow|twinkle|colours|scroll|blink> [slow|normal|fast]: it lives like the game's animated places (rippling water, a glowing lantern, colours that cycle)",
    ),
    (
        "foot",
        "foot: laid by its foot (the point where it touches the ground, the piece's `foot=`, otherwise the middle of its bottom), not by its top left corner",
    ),
];
const MODIFIED: [&str; 5] = ["put", "scatter", "water", "ring", "image"];

/// The three layers of a place, from the ground up: under all, as the
/// piece is (its top in front of whoever passes), in front of all.
pub const LAYERS: [&str; 3] = ["ground", "objects", "over"];

/// A layer as said, as it is kept: none for `objects`, the piece as it is.
pub fn layer_of(word: &str) -> Option<Option<&'static str>> {
    match word {
        "ground" => Some(Some("ground")),
        "over" => Some(Some("over")),
        "objects" => Some(None),
        _ => None,
    }
}

/// The line that lays a piece there: its corner at that pixel of the place.
pub fn laid(piece: &str, x: f32, y: f32) -> String {
    format!("put {piece} {} {}", x.round() as i64, y.round() as i64)
}

/// The least and the most a place's side may be, in pixels.
pub const SIDES: (i64, i64) = (16, 4096);

/// The commands of `decor`, and what each does, as `decor help` shows them.
pub const VERBS: [(&str, &str); 25] = [
    ("help", "help                               this text, and every word of a .decor"),
    ("new", "new <name> <width> <height>        writes a starting place (its level fixed)"),
    (
        "build",
        "build <name> [--from f] [--out d] [--json]\n                                           builds it, says what is wrong; --from: its text read from this\n                                           file (the place being written); --out: writes there;\n                                           --json: one JSON line (size, level, grid, images, walls,\n                                           middle, spots, layers, remarks)",
    ),
    (
        "view",
        "view <name>                        and its sheets in target/decors/: <name>.png, _walls.png,\n                                           _side_by_side.png, _grid.png (its grid, what it lays, named)",
    ),
    (
        "grid",
        "grid <name>                        the image with its grid (grid n, otherwise 16) to read places\n  decor grid <level> [x0 y0 x1 y1]         a place of the game (or a rectangle) and its grid, to note\n                                           a piece's rectangle",
    ),
    (
        "resize",
        "resize <name> <width> <height> [anchor]\n                                           its size changed, what it lays moved so that the anchor stays\n                                           where it is (top_left, top, top_right, left, center,\n                                           right, bottom_left, bottom, bottom_right; top_left with no anchor)",
    ),
    ("all", "all [--out d]                      builds again every place of library/decors/, and those only a game has"),
    ("import", "import <level> <name>              a place of the game under a new decor (base level), to make one's own"),
    (
        "migrate",
        "migrate                            the places written as before, written again as now (their\n                                           level said, their layers ground|objects|over, the life after [life],\n                                           old dungeon rooms as .decor with base dungeon): built the same",
    ),
    ("overview", "overview [out] [name ...]          every (or these) place on one sheet: <out>.png"),
    ("dungeons", "dungeons [word]                    the dungeons and their floors (for base dungeon)"),
    (
        "tileset",
        "tileset template <out.png>          the template of 47 cells of a tileset of one's own (wall, water, floor)
  decor tileset export <n> <out.png>       the game's tileset n drawn on the template, to start from it
  decor tileset import <png> <word> [dither] a drawing on the template made a tileset (12 palettes):
                                           library/tilesets/<word>.png, \"tileset: <word>\" of a .dungeon
  decor tileset example [word]             a tileset drawn by code on the template (stone, water, grass):
                                           library/tilesets/<word>.png (example_meadow with no word)",
    ),
    ("tilequant", "tilequant <png> [palettes] [dither]  a picture to the console's rules (tiles of 8, 16 colours): <name>_quant.png"),
    (
        "generate",
        "generate <name> <dungeon> <floor> [seed] [width height]\n                                           a whole floor made as the game makes them: a .decor (base dungeon, layout)",
    ),
    (
        "pieces",
        "pieces [word] [--backdrops]        the catalogue by kind and by place: pieces[_word].png;\n                                           --backdrops: each thing on black and on white, without shadow",
    ),
    (
        "element",
        "element <name> [name ...]          one piece, large, on black and on white, its grid of tiles\n                                           numbered (for without= and keep=); or several",
    ),
    ("check", "check [word]                       what is left of their ground on the pieces, the worst first"),
    (
        "grounds",
        "grounds [word]                     each ground woven on a square, to choose one (no word:\n                                           those named; s52: those found in place 52; a place number)",
    ),
    ("reassemble", "reassemble <level>                 the things of a place of the game put back in their place, on black"),
    ("inventory", "inventory [level ...]              finds again the things of the game's places (data/extra/game_pieces.txt)"),
    (
        "terrains",
        "terrains <level>                   the terrains learnt from a place of the game (their letters for `terrain`),\n                                           and the place made again from its own corners: terrains_<level>.png",
    ),
    ("sort", "sort                               sets aside from the Workshop the scrap pieces and the duplicates, turned over or not (set_aside.txt)"),
    ("labels", "labels <folder> [all] [e|a|o]       the elements offered with no line in piece_tags.txt, on sheets to label them by eye"),
    ("previews", "previews <folder>                  the image of each element of the game's places, place by place, to look them over"),
    ("recreate", "recreate <level> <name>            makes a place of the game again from pieces alone, identically"),
];

/// `decor help`: the commands, then every word of a `.decor` from the
/// tables the reader reads by (`COMMANDS`, `MODIFIERS`, `LAYERS`, `LIFE`),
/// so that it cannot say other than what is read.
pub fn help() -> String {
    let command = |name: &str| VERBS.iter().find(|(verb, _)| *verb == name).map_or(String::new(), |(_, said)| format!("  decor {said}\n"));
    let commands = |names: &[&str]| names.iter().map(|name| command(name)).collect::<String>();
    let words: String = COMMANDS.iter().map(|(_, said)| format!("  {said}\n")).collect();
    let after: String = MODIFIERS.iter().map(|(_, said)| format!("  {said}\n")).collect();
    format!(
        "The places for the films: library/decors/<name>.decor, made of the pieces of the game's places
(library/tables/pieces.txt), built into data/extra/ (never committed) as the game reads its places.
There is only one kind of place, the .decor; a dungeon room or floor is one too (base dungeon).

Places:
{}
Dungeons:
{}
Pieces:
{}
Exit codes: 0 done, 1 a place or a piece at fault (the file and its line are said), 2 the command
badly said. The language is in decor/DECORS.md. A place is built the same every time: what
is scattered comes from a seed.

The words of a .decor:
{words}
After a piece laid ({}):
{after}
The layers, from the ground up: {} (saying nothing: objects). After [life]: {}.
Each put is by the top left corner of the piece, to the pixel, whatever its kind (a
tree too), except `foot`; ring lays its trees by their foot, on their grid.
",
        commands(&["help", "new", "import", "recreate", "build", "view", "grid", "resize", "migrate", "all", "overview"]),
        commands(&["dungeons", "generate", "tileset", "tilequant"]),
        commands(&["pieces", "element", "check", "grounds", "reassemble", "inventory", "terrains", "sort", "labels", "previews"]),
        MODIFIED.join(", "),
        LAYERS.join(", "),
        LIFE.join(", "),
    )
}

/// What a word says, by the table it is in.
pub fn said(word: &str) -> &'static str {
    COMMANDS.iter().find(|(known, _)| *known == word).map_or("", |(_, said)| *said)
}

/// A name as compared: small letters, no accents, `_` for the rest
/// (`diagnostics::text::Kept::Joined`).
pub(crate) fn plain(word: &str) -> String {
    text::normalise(word, text::Kept::Joined)
}

/// ` Closest: a, b.` for the words most like it (`text::alike`), if any.
pub fn closest<'a>(word: &str, names: impl IntoIterator<Item = &'a str>) -> String {
    let names = text::nearest(names, 3, |name| Some(text::alike(word, name)).filter(|ratio| *ratio >= 0.5));
    if names.is_empty() { String::new() } else { format!(" Closest: {}.", names.join(", ")) }
}

/// A line of a place that lays something.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub number: usize,
    pub verb: String,
    pub words: Vec<String>,
    pub mirrored: bool,
    pub shadow: bool,
    /// None: as the piece is (`objects`); `ground` or `over`.
    pub layer: Option<String>,
    /// A spot of the films at its foot, by that name.
    pub name: Option<String>,
    /// Not laid (`hidden`); not taken by the Workshop's pointer (`locked`).
    pub hidden: bool,
    pub locked: bool,
    /// Its points are its foot (`foot`), not its top left corner.
    pub foot: bool,
    /// How it lives (`anim wave fast`): its way, its pace.
    pub animation: Option<(String, Option<String>)>,
    /// Still, though the game animates it (`still`): its first picture only.
    pub still: bool,
}

/// Who lives in a place.
#[derive(Clone, Debug, PartialEq)]
pub struct Resident {
    pub name: String,
    pub x: i64,
    pub y: i64,
    pub facing: String,
    pub wanders: i64,
    pub lines: Vec<String>,
    pub number: usize,
    /// What he does, by its name (`Sleep`, `Sit`), over and over; held on its first picture.
    pub animation: Option<String>,
    pub still: bool,
    /// Each line's test, if it has one (`says if met: …`), beside `lines`.
    pub tests: Vec<Option<String>>,
    /// Only there while that holds (`resident kecleon 120 80 if chapter >= 2`).
    pub condition: Option<String>,
    /// Talked to, what the game does after the lines: `film <word>`, `shop`, `bank`….
    pub talk: Option<String>,
}

/// What is under a place, as it is (`base`): a place of the game at a
/// corner, or a dungeon's floor made of the place's layout.
#[derive(Clone, Debug, PartialEq)]
pub enum Base {
    /// `hollow`: its elements taken out of it (laid again by its lines), the
    /// holes filled with its own ground.
    Level {
        level: i64,
        x: i64,
        y: i64,
        line: usize,
        hollow: bool,
    },
    Dungeon {
        dungeon: String,
        floor: i64,
        line: usize,
    },
}

/// A place as its file says it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decor {
    pub name: String,
    /// What is under it (`base`), and the layout of a dungeon's floor (its rows, each with its line).
    pub base: Option<Base>,
    pub plan: Vec<(usize, Vec<char>)>,
    pub width: usize,
    pub height: usize,
    pub lines: Vec<Line>,
    pub level: Option<i64>,
    /// The step of its grid, in the Workshop and on its sheets (`grid 16`).
    pub grid: Option<usize>,
    pub shown: Option<String>,
    pub middle: Option<(i64, i64)>,
    pub seed: Option<i64>,
    pub compared: Vec<i64>,
    pub residents: Vec<Resident>,
    pub music: Option<i64>,
    /// Its ways out: x, y, width, height, the place they lead to.
    pub exits: Vec<(i64, i64, i64, i64, i64)>,
    /// Its ways out to the places of a game of one's own, by their names.
    pub ways_out: Vec<WayOut>,
    /// Its zones, by their names.
    pub zones: Vec<Zone>,
    /// What the game does there (`on enter` … `end`): its first line, its lines, where it starts.
    pub rules: Vec<LifeRule>,
    /// Its ways in: from, x, y, width, height.
    pub links: Vec<(i64, i64, i64, i64, i64)>,
    pub departure: bool,
    /// Its entrances: line, x, y, name (its things named too, at their foot).
    pub entrances: Vec<(usize, i64, i64, String)>,
    /// Its groups: name, first line, last line.
    pub groups: Vec<(String, usize, usize)>,
    pub file: String,
}

/// Numbers said (a comma after one let be), so many if `count` says.
pub fn numbers(at: &Location, words: &[String], count: Option<usize>) -> Result<Vec<i64>, Diagnostic> {
    let found: Option<Vec<i64>> = words.iter().map(|word| word.trim_end_matches(',').parse().ok()).collect();
    let Some(found) = found else {
        return Err(at.wrong(format!("numbers expected, not \"{}\"", words.join(" "))));
    };
    if let Some(count) = count.filter(|count| found.len() != *count) {
        return Err(at.wrong(format!("{count} numbers expected, not \"{}\"", words.join(" "))));
    }
    Ok(found)
}

/// `flip`, `shadow off`, `layer ground|objects|over`, `name`, `hidden`,
/// `locked`, `foot` taken off the end of a line.
fn modifiers(at: &Location, line: &mut Line) -> Result<(), Diagnostic> {
    let words = std::mem::take(&mut line.words);
    let mut kept = vec![words[0].clone()];
    let mut index = 1;
    while index < words.len() {
        let word = words[index].as_str();
        if word == "flip" {
            line.mirrored = true;
        } else if word == "hidden" {
            line.hidden = true;
        } else if word == "locked" {
            line.locked = true;
        } else if word == "foot" {
            line.foot = true;
        } else if word == "still" {
            line.still = true;
        } else if word == "anim" {
            let kinds: Vec<&str> = crate::animate::KINDS.iter().map(|(kind, _)| *kind).collect();
            let Some(kind) = words.get(index + 1).filter(|kind| kinds.contains(&kind.as_str())) else {
                let said = words.get(index + 1).map_or("", String::as_str);
                return Err(at.wrong(format!("anim <kind> [slow|normal|fast]: one of {}{}", kinds.join(", "), closest(said, kinds.iter().copied()))));
            };
            index += 1;
            let pace = words.get(index + 1).filter(|pace| crate::animate::PACES.iter().any(|(known, _)| known == pace)).cloned();
            if pace.is_some() {
                index += 1;
            }
            line.animation = Some((kind.clone(), pace));
        } else if word == "name" {
            let Some(name) = words.get(index + 1).filter(|name| name.chars().all(|letter| letter.is_alphanumeric() || letter == '_')) else {
                return Err(at.wrong("name word: one word, letters, digits and _"));
            };
            line.name = Some(name.clone());
            index += 1;
        } else if word == "shadow" || word == "layer" {
            let value = words.get(index + 1).map_or("", String::as_str);
            let told = || at.wrong(MODIFIERS.iter().find(|(known, _)| *known == word).map_or("", |(_, told)| *told));
            if word == "shadow" {
                if !["off", "on"].contains(&value) {
                    return Err(told());
                }
                line.shadow = value == "on";
            } else {
                line.layer = layer_of(value).ok_or_else(told)?.map(str::to_string);
            }
            index += 1;
        } else if word.trim_end_matches(',').trim_start_matches('-').chars().all(|letter| letter.is_ascii_digit())
            && !word.trim_end_matches(',').trim_start_matches('-').is_empty()
            || ["zone", "oval", ","].contains(&word)
        {
            kept.push(word.to_string());
        } else {
            let names = MODIFIERS.iter().map(|(name, _)| *name).chain(["zone", "oval"]);
            let all: Vec<&str> = MODIFIERS.iter().map(|(name, _)| *name).collect();
            return Err(at.wrong(format!("\"{word}\"?{} After the numbers: {}.", closest(word, names), all.join(", "))));
        }
        index += 1;
    }
    line.words = kept;
    Ok(())
}

/// A place's text being read: the place so far (none before its first
/// line), whether a layout's rows are being read, whether `[life]` was
/// said, a rule being read (`on …` up to its `end`) and how many blocks are
/// open in it.
struct Parsing<'a> {
    file: &'a str,
    decor: Option<Decor>,
    planning: bool,
    living: bool,
    rule: Option<(LifeRule, i32)>,
}

/// A line of the place said: where, its verb, the words after it, the line as written, its number.
struct Told<'a> {
    at: Location,
    verb: &'a str,
    rest: &'a [String],
    raw: &'a str,
    number: usize,
}

impl Told<'_> {
    /// How its verb is written, as its fault.
    fn usage(&self) -> Diagnostic {
        self.at.wrong(said(self.verb))
    }
}

/// What a verb does to the place.
type Verb = fn(&mut Decor, &Told<'_>) -> Result<(), Diagnostic>;

/// The verbs that do more than lay a line (the others: `laid_line`).
const DOINGS: &[(&str, Verb)] = &[
    ("level", |decor, told| {
        decor.level = Some(numbers(&told.at, told.rest, Some(1))?[0]);
        Ok(())
    }),
    ("name", |decor, told| {
        decor.shown = Some(told.rest.join(" "));
        Ok(())
    }),
    ("base", base),
    ("grid", |decor, told| {
        let step = numbers(&told.at, told.rest, Some(1))?[0];
        if !(1..=256).contains(&step) {
            return Err(told.at.wrong("grid n: from 1 to 256 pixels"));
        }
        decor.grid = Some(step as usize);
        Ok(())
    }),
    ("center", |decor, told| {
        let point = numbers(&told.at, told.rest, Some(2))?;
        decor.middle = Some((point[0], point[1]));
        Ok(())
    }),
    ("seed", |decor, told| {
        decor.seed = Some(numbers(&told.at, told.rest, Some(1))?[0]);
        Ok(())
    }),
    ("like", |decor, told| {
        decor.compared = numbers(&told.at, told.rest, None)?;
        Ok(())
    }),
    ("resident", resident),
    ("says", says),
    ("talk", |decor, told| {
        let Some(resident) = decor.residents.last_mut() else {
            return Err(told.at.wrong("\"talk\" follows a \"resident\""));
        };
        match told.rest {
            [film, word] if film == "film" => resident.talk = Some(format!("film {word}")),
            [screen] if TALKS[1..].contains(&screen.as_str()) => resident.talk = Some(screen.clone()),
            _ => return Err(told.usage()),
        }
        Ok(())
    }),
    ("zone", |decor, told| {
        let [name, x, y, wide, high] = told.rest else {
            return Err(told.usage());
        };
        let zone = numbers(&told.at, &[x.clone(), y.clone(), wide.clone(), high.clone()], Some(4))?;
        decor.zones.push(Zone { name: name.clone(), x: zone[0], y: zone[1], width: zone[2], height: zone[3] });
        Ok(())
    }),
    ("music", |decor, told| {
        decor.music = Some(numbers(&told.at, told.rest, Some(1))?[0]);
        Ok(())
    }),
    ("exit", exit),
    ("departure", |decor, _| {
        decor.departure = true;
        Ok(())
    }),
    ("link", |decor, told| {
        let link = numbers(&told.at, told.rest, Some(5))?;
        decor.links.push((link[0], link[1], link[2], link[3], link[4]));
        Ok(())
    }),
    ("group", |decor, told| {
        let [name] = told.rest else {
            return Err(told.usage());
        };
        if decor.groups.last().is_some_and(|(_, _, end)| *end == 0) {
            return Err(told.at.wrong("a group in a group: close the first with \"end\""));
        }
        decor.groups.push((name.clone(), told.number, 0));
        Ok(())
    }),
    ("end", |decor, told| match decor.groups.last_mut().filter(|(_, _, end)| *end == 0) {
        Some(group) => {
            group.2 = told.number;
            Ok(())
        }
        None => Err(told.at.wrong("\"end\" without \"group\"")),
    }),
    ("wall", walls),
    ("free", walls),
    ("terrain", |decor, told| {
        crate::terrain::Painted::read(told.rest).map_err(|why| told.at.wrong(why))?;
        decor.lines.push(plain_line(told));
        Ok(())
    }),
    ("spot", |decor, told| {
        let [name, x, y] = told.rest else {
            return Err(told.usage());
        };
        let point = numbers(&told.at, &[x.clone(), y.clone()], Some(2))?;
        decor.entrances.push((told.number, point[0], point[1], name.clone()));
        Ok(())
    }),
    ("entrance", |decor, told| {
        let rest = told.rest;
        if rest.len() < 2 {
            return Err(told.usage());
        }
        let point = numbers(&told.at, &rest[..2], Some(2))?;
        let name = if rest.len() > 2 { rest[2..].join(" ") } else { format!("entrance{}", decor.entrances.len() + 1) };
        decor.entrances.push((told.number, point[0], point[1], name));
        Ok(())
    }),
];

/// A line that lays something, as it is written (no modifier read yet).
fn plain_line(told: &Told<'_>) -> Line {
    Line {
        number: told.number,
        verb: told.verb.to_string(),
        words: told.rest.to_vec(),
        mirrored: false,
        shadow: true,
        layer: None,
        name: None,
        hidden: false,
        locked: false,
        foot: false,
        animation: None,
        still: false,
    }
}

/// `base level n [x y] [hollow]`, `base dungeon name floor`: only one.
fn base(decor: &mut Decor, told: &Told<'_>) -> Result<(), Diagnostic> {
    let (at, rest) = (&told.at, told.rest);
    if decor.base.is_some() {
        return Err(at.wrong("only one base"));
    }
    decor.base = Some(match rest.first().map(String::as_str) {
        Some("level") if [2, 3, 4, 5].contains(&rest.len()) => {
            let hollow = rest.last().is_some_and(|word| word == "hollow");
            let said = numbers(at, &rest[1..rest.len() - usize::from(hollow)], None)?;
            if said.len() != 1 && said.len() != 3 {
                return Err(told.usage());
            }
            Base::Level { level: said[0], x: said.get(1).copied().unwrap_or(0), y: said.get(2).copied().unwrap_or(0), line: told.number, hollow }
        }
        Some("dungeon") if rest.len() > 2 => {
            let floor = numbers(at, &rest[rest.len() - 1..], Some(1))?[0];
            Base::Dungeon { dungeon: rest[1..rest.len() - 1].join(" "), floor, line: told.number }
        }
        _ => return Err(told.usage()),
    });
    Ok(())
}

/// `resident name x y [way] [wanders n] [anim word] [still] [if test]`.
fn resident(decor: &mut Decor, told: &Told<'_>) -> Result<(), Diagnostic> {
    let at = &told.at;
    // (`if …` at its end: there only while it holds.)
    let (rest, condition) = match told.rest.iter().position(|word| word == "if") {
        Some(at_if) => (&told.rest[..at_if], Some(told.rest[at_if + 1..].join(" ")).filter(|test| !test.is_empty())),
        None => (told.rest, None),
    };
    if rest.len() < 3 {
        return Err(told.usage());
    }
    let point = numbers(at, &rest[1..3], Some(2))?;
    // (After his feet, in any order: the way he faces, how far he wanders, what he does.)
    let (mut facing, mut wanders, mut animation, mut still) = ("down".to_string(), 0, None, false);
    let mut left = &rest[3..];
    while let Some(word) = left.first() {
        match word.as_str() {
            way if FACING.contains(&way) => facing = way.to_string(),
            "wanders" => {
                wanders = numbers(at, &left[1..left.len().min(2)], Some(1))?[0];
                left = &left[1..];
            }
            "anim" => {
                animation = Some(left.get(1).ok_or_else(|| told.usage())?.clone());
                left = &left[1..];
            }
            "still" => still = true,
            _ => return Err(told.usage()),
        }
        left = &left[1..];
    }
    decor.residents.push(Resident {
        name: rest[0].clone(),
        x: point[0],
        y: point[1],
        facing,
        wanders,
        lines: Vec::new(),
        number: told.number,
        animation,
        still,
        tests: Vec::new(),
        condition,
        talk: None,
    });
    Ok(())
}

/// `says …` (`says if not met: Welcome!`: only while it holds), after a resident.
fn says(decor: &mut Decor, told: &Told<'_>) -> Result<(), Diagnostic> {
    let Some(resident) = decor.residents.last_mut() else {
        return Err(told.at.wrong("\"says\" follows a \"resident\""));
    };
    let said = told.raw.split_once("says").map_or("", |(_, said)| said).trim();
    let (test, said) = match said.strip_prefix("if ").and_then(|tested| tested.split_once(':')) {
        Some((test, said)) => (Some(test.trim().to_string()), said.trim()),
        None => (None, said),
    };
    resident.lines.push(said.to_string());
    resident.tests.push(test);
    Ok(())
}

/// `exit x y w h to place [entrance]`: to a place of the game by its
/// number; to a place of one's own by its name, and an entrance.
fn exit(decor: &mut Decor, told: &Told<'_>) -> Result<(), Diagnostic> {
    let rest = told.rest;
    if !(6..=7).contains(&rest.len()) || rest[4] != "to" {
        return Err(told.usage());
    }
    let zone = numbers(&told.at, &rest[..4], Some(4))?;
    match rest[5].parse::<i64>() {
        Ok(to) if rest.len() == 6 => decor.exits.push((zone[0], zone[1], zone[2], zone[3], to)),
        Ok(_) => return Err(told.usage()),
        Err(_) => decor.ways_out.push(WayOut { x: zone[0], y: zone[1], width: zone[2], height: zone[3], to: rest[5].clone(), entrance: rest.get(6).cloned() }),
    }
    Ok(())
}

/// `wall x y w h`, `free tiles 3,4 5,4`: walls laid or taken off.
fn walls(decor: &mut Decor, told: &Told<'_>) -> Result<(), Diagnostic> {
    match told.rest.first().map(String::as_str) {
        Some("tiles") => {
            for word in &told.rest[1..] {
                let tile = word.split_once(',').and_then(|(column, row)| Some((column.parse::<i64>().ok()?, row.parse::<i64>().ok()?)));
                if tile.is_none() {
                    return Err(told.at.wrong(format!("\"{word}\": a tile is column,row ({} tiles 3,4 5,4)", told.verb)));
                }
            }
        }
        _ => {
            numbers(&told.at, told.rest, Some(4))?;
        }
    }
    decor.lines.push(plain_line(told));
    Ok(())
}

/// A line that lays a piece (its modifiers read, if it takes them); hidden: kept in the file, not laid.
fn laid_line(decor: &mut Decor, told: &Told<'_>) -> Result<(), Diagnostic> {
    if told.rest.is_empty() {
        return Err(told.usage());
    }
    let mut made = plain_line(told);
    if MODIFIED.contains(&told.verb) {
        modifiers(&told.at, &mut made)?;
    }
    if !made.hidden {
        decor.lines.push(made);
    }
    Ok(())
}

/// The first line, `decor: name width height`: the place begun.
fn first_line(at: &Location, words: &[String], file: &str) -> Result<Decor, Diagnostic> {
    if words[0] != "decor:" || words.len() != 4 {
        return Err(at.wrong("the first line is \"decor: name width height\""));
    }
    let size = numbers(at, &words[2..], Some(2))?;
    let (width, height) = (size[0], size[1]);
    if ![width, height].iter().all(|side| (SIDES.0..=SIDES.1).contains(side)) {
        return Err(at.wrong(format!("width and height from {} to {} pixels", SIDES.0, SIDES.1)));
    }
    Ok(Decor { name: words[1].clone(), width: width as usize, height: height as usize, file: file.to_string(), ..Decor::default() })
}

impl Parsing<'_> {
    /// A line read: a layout's row, a rule's, the first line, or a verb's.
    fn line(&mut self, number: usize, raw: &str) -> Result<(), Diagnostic> {
        // (A layout's rows, read as they are: `#` is a wall there.)
        if self.planning
            && let Some(decor) = self.decor.as_mut()
        {
            if raw.trim() == "[life]" {
                self.planning = false;
            } else {
                if !raw.trim().is_empty() {
                    decor.plan.push((number, raw.trim_end().chars().collect()));
                }
                return Ok(());
            }
        }
        if self.in_rule(raw) {
            return Ok(());
        }
        let words: Vec<String> = raw.split('#').next().unwrap_or_default().split_whitespace().map(str::to_string).collect();
        if words.is_empty() {
            return Ok(());
        }
        let at = Location::in_file(self.file, number);
        let Some(decor) = self.decor.as_mut() else {
            self.decor = Some(first_line(&at, &words, self.file)?);
            return Ok(());
        };
        let (verb, rest) = (words[0].as_str(), &words[1..]);
        if !COMMANDS.iter().any(|(known, _)| *known == verb) {
            return Err(at.wrong(format!("unknown word \"{verb}\".{}", closest(verb, COMMANDS.iter().map(|(word, _)| *word)))));
        }
        if self.living && !LIFE.contains(&verb) {
            return Err(at.wrong(format!("\"{verb}\" after [life]: there, only {}; what is seen goes before", LIFE.join(", "))));
        }
        let told = Told { at, verb, rest, raw, number };
        match verb {
            "[life]" => self.living = true,
            "layout" => self.planning = true,
            "on" => {
                let head = raw.split('#').next().unwrap_or_default().trim().to_string();
                if !matches!(rest.first().map(String::as_str), Some("enter" | "talk" | "zone")) {
                    return Err(told.usage());
                }
                self.rule = Some((LifeRule { head, body: String::new(), line: number }, 1));
            }
            _ => match DOINGS.iter().find(|(known, _)| *known == verb) {
                Some((_, verb)) => verb(decor, &told)?,
                None => laid_line(decor, &told)?,
            },
        }
        Ok(())
    }

    /// What the game does there, `on …` up to its `end`: kept as written, a
    /// game reads it. Whether the line was the rule's.
    fn in_rule(&mut self, raw: &str) -> bool {
        let Some((open, depth)) = self.rule.as_mut() else {
            return false;
        };
        match raw.split_whitespace().next().unwrap_or_default() {
            "if" | "on" | "choice" => *depth += 1,
            "end" => *depth -= 1,
            _ => {}
        }
        if *depth > 0 {
            open.body.push_str(raw);
            open.body.push('\n');
        } else if let (Some((done, _)), Some(decor)) = (self.rule.take(), self.decor.as_mut()) {
            decor.rules.push(done);
        }
        true
    }

    /// The place read whole: no rule nor group left open.
    fn finished(self) -> Result<Decor, Diagnostic> {
        if let Some((open, _)) = self.rule {
            return Err(Location::in_file(self.file, open.line).wrong(format!("\"{}\" has no \"end\"", open.head)));
        }
        if let Some((name, line, _)) = self.decor.as_ref().and_then(|decor| decor.groups.iter().find(|(_, _, end)| *end == 0)) {
            return Err(Location::in_file(self.file, *line).wrong(format!("the group \"{name}\" has no \"end\"")));
        }
        self.decor.ok_or_else(|| Location::in_file(self.file, 0).wrong("empty; the first line is \"decor: name width height\""))
    }
}

/// A place as its file says it; errors with their line.
pub fn parse(text: &str, file: &str) -> Result<Decor, Diagnostic> {
    let mut parsing = Parsing { file, decor: None, planning: false, living: false, rule: None };
    for (number, raw) in text.lines().enumerate().map(|(at, line)| (at + 1, line)) {
        parsing.line(number, raw)?;
    }
    parsing.finished()
}

/// `x y, x y, ...` as points.
pub fn points_of(at: &Location, words: &[String]) -> Result<Vec<(i64, i64)>, Diagnostic> {
    let flat: Vec<String> = words.join(" ").replace(',', " ").split_whitespace().map(str::to_string).collect();
    let flat = numbers(at, &flat, None)?;
    if flat.is_empty() || flat.len() % 2 != 0 {
        return Err(at.wrong("pairs x y"));
    }
    Ok(flat.chunks(2).map(|pair| (pair[0], pair[1])).collect())
}

/// `zone x0 y0 x1 y1`.
pub fn zone_of(at: &Location, words: &[String], usage: &str) -> Result<(i64, i64, i64, i64), Diagnostic> {
    if words.len() != 5 || words[0] != "zone" {
        return Err(at.wrong(usage));
    }
    let zone = numbers(at, &words[1..], Some(4))?;
    if zone[0] >= zone[2] || zone[1] >= zone[3] {
        return Err(at.wrong("a zone goes from its top left corner to its bottom right one"));
    }
    Ok((zone[0], zone[1], zone[2], zone[3]))
}

/// `from x y to x y ... [width n]`.
pub fn read_path(at: &Location, words: &[String]) -> Result<(Vec<(i64, i64)>, i64), Diagnostic> {
    let mut words = words.to_vec();
    let mut width = 24;
    if let Some(index) = words.iter().position(|word| word == "width") {
        width = numbers(at, &words[index + 1..(index + 2).min(words.len())], Some(1))?[0];
        words.drain(index..(index + 2).min(words.len()));
    }
    if words.first().is_none_or(|word| word != "from") {
        return Err(at.wrong(said("path")));
    }
    let (mut points, mut index) = (Vec::new(), 0);
    while index < words.len() {
        if !["from", "to"].contains(&words[index].as_str()) || index + 3 > words.len() || (index == 0) != (words[index] == "from") {
            return Err(at.wrong("\"from x y\" then \"to x y\""));
        }
        let point = numbers(at, &words[index + 1..index + 3], Some(2))?;
        points.push((point[0], point[1]));
        index += 3;
    }
    if points.len() < 2 {
        return Err(at.wrong("a path goes from somewhere to somewhere"));
    }
    Ok((points, width))
}

/// A way out to a place of a game of one's own: `exit x y width height to town south`.
#[derive(Clone, Debug, PartialEq)]
pub struct WayOut {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub to: String,
    pub entrance: Option<String>,
}

/// A zone of a place, by its name: `zone pier 400 200 64 32`.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub name: String,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

/// What the game does in a place: `on talk kecleon` (its head), the lines up
/// to its `end` (as written: a game reads them), the line it starts at.
#[derive(Clone, Debug, PartialEq)]
pub struct LifeRule {
    pub head: String,
    pub body: String,
    pub line: usize,
}

/// The screens and what a resident may do when talked to (`talk …`).
pub const TALKS: [&str; 9] = ["film", "shop", "market", "bank", "storage", "jobs", "recruits", "appraise", "cafe"];
