//! Places for films, and their sheets.
//!
//! Exits: 0 done, 1 a place or a piece wrong (its file and line said), 2 the
//! command said wrong.

use std::path::PathBuf;
use std::process::ExitCode;

use decor::catalogue::{Catalogue, Piece};
use decor::fault::Diagnostic;
use decor::lang::{VERBS, closest, help};
use decor::weave::Workshop;

/// What went wrong: the command said wrong (its usage), or a place or a piece.
enum Failed {
    Usage(String),
    Wrong(Diagnostic),
}

impl From<Diagnostic> for Failed {
    fn from(why: Diagnostic) -> Failed {
        Failed::Wrong(why)
    }
}

fn usage(said: &str) -> Failed {
    Failed::Usage(said.to_string())
}

/// The value after a word of the command, taken off it.
fn take(words: &mut Vec<String>, word: &str) -> Result<Option<String>, Failed> {
    let Some(at) = words.iter().position(|said| said == word) else {
        return Ok(None);
    };
    if at + 1 >= words.len() {
        return Err(usage(&format!("{word} <...>")));
    }
    let value = words.remove(at + 1);
    words.remove(at);
    Ok(Some(value))
}

fn numbers(words: &[String]) -> Option<Vec<i64>> {
    words.iter().map(|word| word.parse().ok()).collect()
}

/// The pieces a word picks: by name, kind, note or place; none, all.
fn picked<'a>(catalogue: &'a Catalogue, word: Option<&str>, by: impl Fn(&Piece, &str) -> bool) -> Result<Vec<&'a Piece>, Failed> {
    let chosen: Vec<&Piece> = catalogue.pieces.iter().filter(|piece| word.is_none_or(|word| by(piece, word))).collect();
    match word {
        Some(word) if chosen.is_empty() => Err(Diagnostic::said(format!("no piece for \"{word}\".{}", closest(word, catalogue.names()))).into()),
        _ => Ok(chosen),
    }
}

fn level_of(piece: &Piece) -> String {
    match piece.level {
        decor::catalogue::Level::Place(level) => level.to_string(),
        decor::catalogue::Level::Thing(thing) => format!("o{thing}"),
    }
}

/// What every command may need: the game's data, where what is made goes,
/// the file a place is read from instead of its own, places told as JSON,
/// the pieces' sheets without their backdrops.
struct Asked {
    /// The shared library whose places are built (`library/decors`), its root's tools.
    project: assets::project::Project,
    data: String,
    extra: PathBuf,
    from: Option<PathBuf>,
    json: bool,
    bare: bool,
}

impl Asked {
    fn workshop(&self) -> Result<Workshop, Failed> {
        Ok(decor::workshop(&self.project, &self.data)?)
    }

    /// A dungeon room: built, and its picture if it is to be seen.
    fn room(&self, name: &str, seen: bool) -> Result<(), Failed> {
        let made = decor::room::build(&self.project, name, &self.data, &self.extra)?;
        print!("{}", decor::room::told(name, &made));
        if seen {
            let path = decor::sheets::sheet_path(self.project.root(), &format!("{name}.png"))?;
            decor::sheets::canvas::Canvas::of(made.picture()).save(&path)?;
            println!("{}", path.display());
        }
        Ok(())
    }

    /// A place built and told.
    fn one(&self, name: &str, workshop: &Workshop) -> Result<decor::build::Place, Failed> {
        let (place, level) = decor::make(workshop, decor::load(&self.project, name, self.from.as_deref())?, &self.extra)?;
        if self.json {
            println!("{}", decor::told_json(&place, level));
        } else {
            print!("{}", decor::told(&place, level));
        }
        Ok(place)
    }
}

fn run(mut words: Vec<String>) -> Result<(), Failed> {
    let from = take(&mut words, "--from")?.map(PathBuf::from);
    let out = take(&mut words, "--out")?.map(PathBuf::from);
    let bare = words.iter().any(|word| word == "--backdrops");
    let json = words.iter().any(|word| word == "--json");
    words.retain(|word| word != "--backdrops" && word != "--json");
    let (what, rest) = match words.split_first() {
        Some((what, rest)) => (what.as_str(), rest),
        None => ("help", &[][..]),
    };
    let project = assets::project::Project::found().map_err(|why| Failed::from(Diagnostic::said(why)))?;
    let data = assets::data::folder(project.root());
    let extra = out.unwrap_or_else(|| decor::write::extra_of(&data));
    let asked = Asked { project, data, extra, from, json, bare };
    match what {
        "help" | "-h" | "--help" => print!("{}", help()),
        "new" => new(&asked, rest)?,
        "recreate" => recreate(&asked, rest)?,
        "import" => {
            let [level, name] = rest else {
                return Err(usage("import <level> <name>"));
            };
            let level = level.parse().map_err(|_| usage("import <level> <name>: a place number of the game"))?;
            println!("{}", decor::imported(&asked.project, &asked.data, &asked.extra, level, name)?.display());
            asked.one(name, &asked.workshop()?)?;
        }
        "migrate" if rest.is_empty() => {
            let told = decor::migrate(&asked.project, &asked.data, &asked.extra)?;
            if told.is_empty() {
                println!("nothing to migrate");
            }
            for line in told {
                println!("{line}");
            }
        }
        "resize" => resize(&asked, rest)?,
        "grid" if rest.first().is_some_and(|word| word.parse::<i64>().is_ok()) => {
            let (Some(said), 1 | 5) = (numbers(rest), rest.len()) else {
                return Err(usage("grid <level> [x0 y0 x1 y1]"));
            };
            let rect = (said.len() == 5).then(|| (said[1], said[2], said[3], said[4]));
            println!("{}", decor::sheets::pieces::level_grid(asked.project.root(), &asked.data, said[0], rect)?.display());
        }
        "build" | "view" if rest.len() == 1 && asked.from.is_none() && decor::room::names(&asked.project).contains(&rest[0]) => {
            asked.room(&rest[0], what == "view")?
        }
        "build" | "view" | "grid" => build(&asked, what, rest)?,
        "game_grounds" => game_grounds(&asked)?,
        "all" if rest.is_empty() && asked.from.is_none() => all(&asked)?,
        "dungeons" => println!("{}", decor::room::list(&rest.join(" "), &asked.data)?.join("\n")),
        "tileset" => tileset(&asked, rest)?,
        "tilequant" => tilequant(rest)?,
        "generate" => generate(&asked, rest)?,
        "overview" => overview(&asked, rest)?,
        "pieces" if rest.len() <= 1 => pieces(&asked, rest)?,
        "element" if !rest.is_empty() => println!("{}", decor::sheets::pieces::element(&asked.workshop()?, rest)?.display()),
        "check" if rest.len() <= 1 => check(&asked, rest)?,
        "grounds" if rest.len() <= 1 => grounds(&asked, rest)?,
        "reassemble" => {
            let [level] = rest else {
                return Err(usage("reassemble <level>"));
            };
            let level = level.parse().map_err(|_| usage("reassemble <level>"))?;
            println!("{}", decor::sheets::pieces::put_back_sheet(&asked.workshop()?, level)?.display());
        }
        "previews" => {
            let [folder] = rest else {
                return Err(usage("previews <folder>"));
            };
            let out = std::path::PathBuf::from(folder);
            println!("{}: {} elements", out.display(), decor::sort::previews(&asked.workshop()?, &out)?);
        }
        "sort" if !rest.is_empty() => {
            for said in decor::sort::measured(&asked.workshop()?, rest) {
                println!("{said}");
            }
        }
        "sort" => sort(&asked)?,
        "labels" => labels(&asked, rest)?,
        "terrains" => terrains(&asked, rest)?,
        "inventory" => {
            let levels = numbers(rest).ok_or_else(|| usage("inventory [level ...]"))?;
            let out = decor::game_pieces(&asked.data);
            if let Some(folder) = out.parent() {
                _ = std::fs::create_dir_all(folder);
            }
            println!("{}", decor::inventory::inventory(&asked.workshop()?, &levels, &out)?);
        }
        _ if VERBS.iter().any(|(verb, _)| *verb == what) => return Err(usage(&format!("{what}: decor help"))),
        _ => return Err(usage(&format!("\"{what}\"?{} (decor help)", closest(what, VERBS.iter().map(|(verb, _)| *verb))))),
    }
    Ok(())
}

/// `new <name> <width> <height>`: a starting place, its level free.
fn new(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let [name, wide, high] = rest else {
        return Err(usage("new <name> <width> <height>"));
    };
    let (Ok(wide), Ok(high)) = (wide.parse(), high.parse()) else {
        return Err(usage("new <name> <width> <height>: numbers"));
    };
    let level = decor::write::free_level(&decor::source::Game::new(&asked.data), &asked.extra, &asked.project.decors());
    println!("{}", decor::new_place(&asked.project, name, wide, high, Some(level))?.display());
    Ok(())
}

/// `recreate <level> <name>`: a place of the game written again as a decor.
fn recreate(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let [level, name] = rest else {
        return Err(usage("recreate <level> <name>"));
    };
    let level: i64 = level.parse().map_err(|_| usage("recreate <level> <name>: a place number of the game"))?;
    let path = asked.project.decors().join(format!("{name}.decor"));
    if path.exists() {
        return Err(usage("recreate <level> <name>: this decor already exists"));
    }
    let text = decor::recreated(&asked.project, &asked.data, level, name, name)?;
    std::fs::write(&path, text).map_err(|why| decor::fault::Diagnostic::said(format!("{}: {why}", path.display())))?;
    println!("{}", path.display());
    asked.one(name, &asked.workshop()?)?;
    Ok(())
}

/// `resize <name> <width> <height> [anchor]`.
fn resize(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let said = "resize <name> <width> <height> [anchor]";
    let (name, wide, high, anchor) = match rest {
        [name, wide, high] => (name, wide, high, "top_left"),
        [name, wide, high, anchor] => (name, wide, high, anchor.as_str()),
        _ => return Err(usage(said)),
    };
    let (Ok(wide), Ok(high)) = (wide.parse::<usize>(), high.parse::<usize>()) else {
        return Err(usage(said));
    };
    let Some(anchor) = decor::doc::anchor_of(anchor) else {
        let all: Vec<&str> = decor::doc::ANCHORS.iter().map(|(word, _)| *word).collect();
        return Err(usage(&format!("{said}: the anchor is {}", all.join(", "))));
    };
    let (least, most) = decor::lang::SIDES;
    if ![wide, high].iter().all(|side| (least..=most).contains(&(*side as i64))) {
        return Err(usage(&format!("{said}: from {least} to {most} pixels")));
    }
    let path = asked.project.decors().join(format!("{name}.decor"));
    let text = std::fs::read_to_string(&path)
        .map_err(|_| Diagnostic::said(format!("no place \"{name}\".{}", closest(name, decor::names(&asked.project).iter().map(String::as_str)))))?;
    std::fs::write(&path, decor::doc::resized(&text, [wide, high], anchor)).map_err(|why| Diagnostic::said(format!("{}: {why}", path.display())))?;
    println!("{}", path.display());
    asked.one(name, &asked.workshop()?)?;
    Ok(())
}

/// `build`, `view`, `grid <name>`: a place built (its sheets seen, its grid).
fn build(asked: &Asked, what: &str, rest: &[String]) -> Result<(), Failed> {
    let [name] = rest else {
        return Err(usage(&format!("{what} <name> [--from <file>]")));
    };
    let place = asked.one(name, &asked.workshop()?)?;
    if what == "view" {
        for path in decor::sheets::look(asked.project.root(), &asked.data, &place)? {
            println!("{}", path.display());
        }
    } else if what == "grid" {
        println!("{}", decor::sheets::grid(asked.project.root(), &place)?.display());
    }
    Ok(())
}

/// The ground of the game's own places, over which the built ones lie.
fn game_grounds(asked: &Asked) -> Result<(), Failed> {
    let count = decor::grounds::write(&asked.data, &asked.extra, asked.project.root()).map_err(|why| usage(&why))?;
    println!("{}: the ground of {count} places of the game", asked.extra.join("ground.txt").display());
    Ok(())
}

/// Every place and room built (one that cannot be is told, the others still
/// are); then the ground of the game's own places.
fn all(asked: &Asked) -> Result<(), Failed> {
    let failed = decor::build_all(&asked.project, &asked.data, &asked.extra, &mut |told| match told {
        Ok(said) => print!("{said}"),
        Err(why) => eprintln!("{why}"),
    })
    .map_err(|why| usage(&why))?;
    if !failed.is_empty() {
        eprintln!("{} place(s) not built: {}", failed.len(), failed.join(", "));
    }
    Ok(())
}

/// `tileset template | export | import | example`: the tilesets of one's own.
fn tileset(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    match rest {
        [what, out] if what == "template" => {
            decor::tilesets::write_blank(std::path::Path::new(out))?;
            println!("{out}");
        }
        [what, number, out] if what == "export" => {
            let number = number.parse::<u8>().map_err(|_| usage("tileset export <n> <out.png>"))?;
            decor::tilesets::export_game(&asked.data, number, std::path::Path::new(out))?;
            println!("{out}");
        }
        [what, source, word, more @ ..] if what == "import" => {
            let path = decor::tilesets::import(&asked.project, std::path::Path::new(source), word, more.iter().any(|word| word == "dither"))?;
            println!("{}", path.display());
        }
        [what, more @ ..] if what == "example" => {
            let word = more.first().map_or("example_meadow", String::as_str);
            let made = media::template::quantized(&decor::tilesets::drawn_example(), decor::tilesets::PALETTES, false);
            println!("{}", decor::tilesets::write_made(&asked.project, word, &made)?.display());
        }
        _ => return Err(usage("tileset template <out.png> | export <n> <out.png> | import <png> <word> [dither] | example [word]")),
    }
    Ok(())
}

/// `tilequant <png> [palettes] [dither]`: a picture made a tileset's.
fn tilequant(rest: &[String]) -> Result<(), Failed> {
    let Some((source, more)) = rest.split_first() else {
        return Err(usage("tilequant <png> [palettes] [dither]"));
    };
    let palettes = more.iter().find_map(|word| word.parse::<usize>().ok()).unwrap_or(16);
    let asked = media::quant::Asked { palettes, dither: more.iter().any(|word| word == "dither") };
    let (path, made) = decor::tilesets::convert_file(std::path::Path::new(source), asked)?;
    println!("{}: {}×{}, {} palette(s)", path.display(), made.size.0, made.size.1, made.palettes);
    Ok(())
}

/// `generate …`: a dungeon room made, told, its picture written.
fn generate(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let (path, made) = decor::room::generated(&asked.project, rest, &asked.data, &asked.extra)?;
    let name = path.file_stem().map_or(String::new(), |stem| stem.to_string_lossy().into_owned());
    print!("{}", decor::room::told(&name, &made));
    let picture = decor::sheets::sheet_path(asked.project.root(), &format!("{name}.png"))?;
    decor::sheets::canvas::Canvas::of(made.picture()).save(&picture)?;
    println!("{}\n{}", path.display(), picture.display());
    Ok(())
}

/// `overview [out] [names…]`: places side by side in one picture.
fn overview(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let out = rest.first().map_or("overview", String::as_str);
    let known = decor::names(&asked.project);
    let chosen: Vec<String> = if rest.len() > 1 { rest[1..].to_vec() } else { known.clone() };
    if let Some(name) = chosen.iter().find(|name| !known.contains(name)) {
        return Err(Diagnostic::said(format!("no place \"{name}\".{}", closest(name, known.iter().map(String::as_str)))).into());
    }
    let workshop = asked.workshop()?;
    let mut pictures = Vec::new();
    for name in chosen {
        let (place, _) = decor::make(&workshop, decor::load(&asked.project, &name, None)?, &asked.extra)?;
        pictures.push((name, decor::sheets::whole(&place)));
    }
    println!("{}", decor::sheets::ensemble(asked.project.root(), &pictures, out)?.display());
    Ok(())
}

/// `pieces [word]`: the pieces a word picks, on a contact sheet.
fn pieces(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let workshop = asked.workshop()?;
    let word = rest.first().map(String::as_str);
    let chosen = picked(&workshop.catalogue, word, |piece, word| {
        piece.name.contains(word) || piece.kind == word || piece.note.contains(word) || level_of(piece) == word
    })?;
    let (sheet, lines) = decor::sheets::pieces::contact(&workshop, &chosen, asked.bare);
    let path = decor::sheets::sheet_path(
        asked.project.root(),
        &format!("{}{}.png", word.map_or("pieces".to_string(), |word| format!("pieces_{word}")), if asked.bare { "_backdrops" } else { "" }),
    )?;
    sheet.save(&path)?;
    println!("{}", lines.join("\n"));
    println!("{}", path.display());
    Ok(())
}

/// `check [word]`: the pieces' outlines checked (without a word, those named
/// by hand: the found ones are thousands).
fn check(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let workshop = asked.workshop()?;
    let lines = match rest.first() {
        Some(word) => decor::sheets::pieces::outline_check(
            &workshop,
            &picked(&workshop.catalogue, Some(word), |piece, word| piece.name.contains(word) || level_of(piece) == word)?,
        ),
        None => {
            let named = Catalogue::read_named(&decor::pieces_folder(&asked.project))?;
            let chosen: Vec<&Piece> = named.iter().filter_map(|piece| workshop.catalogue.get(&piece.name)).collect();
            decor::sheets::pieces::outline_check(&workshop, &chosen)
        }
    };
    println!("{}", lines.join("\n"));
    Ok(())
}

/// `grounds [word]`: the grounds' swatches (a word: a place's number, a
/// name's start; none: those named by hand).
fn grounds(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let workshop = asked.workshop()?;
    let grounds = workshop.catalogue.pieces.iter().filter(|piece| piece.kind == "ground");
    let chosen: Vec<String> = match rest.first() {
        Some(word) => grounds.filter(|piece| piece.name.contains(word.as_str()) || level_of(piece) == *word).map(|piece| piece.name.clone()).collect(),
        None => {
            let named = Catalogue::read_named(&decor::pieces_folder(&asked.project))?;
            grounds.filter(|piece| named.iter().any(|known| known.name == piece.name)).map(|piece| piece.name.clone()).collect()
        }
    };
    if chosen.is_empty() {
        let word = rest.first().map_or("", String::as_str);
        let names = workshop.catalogue.pieces.iter().filter(|piece| piece.kind == "ground").map(|piece| piece.name.as_str());
        return Err(Diagnostic::said(format!("no ground for \"{word}\".{}", closest(word, names))).into());
    }
    println!("{}", decor::sheets::pieces::ground_swatches(&workshop, &chosen)?.display());
    Ok(())
}

/// Why a piece is set aside, by its kind: what comes before its numbers,
/// its `of …` and its `like …`.
fn why_kind(why: &str) -> String {
    let kind: String = why.split(|letter: char| letter == '(' || letter.is_ascii_digit()).next().unwrap_or(why).trim().to_string();
    kind.split(" of ").next().unwrap_or(&kind).split(" like ").next().unwrap_or(&kind).to_string()
}

/// `sort`: the pieces the Workshop does not offer, and why, written; counted by why.
fn sort(asked: &Asked) -> Result<(), Failed> {
    let aside = decor::sort::sorted(&asked.workshop()?);
    let mut lines = vec![
        "# The pieces the Workshop does not offer, and why: decor sort writes them, nothing is written by hand.".to_string(),
        "# They stay in the catalogue: a decor lays them by their name, a place of the game is made again of all of them.".to_string(),
    ];
    let mut why_counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (name, why) in &aside {
        lines.push(format!("{name}  {why}"));
        *why_counts.entry(why_kind(why)).or_default() += 1;
    }
    let path = decor::pieces_folder(&asked.project).join("set_aside.txt");
    std::fs::write(&path, lines.join("\n") + "\n").map_err(|why| decor::fault::Diagnostic::said(format!("{}: {why}", path.display())))?;
    println!("{}: {} pieces set aside", path.display(), aside.len());
    for (kind, count) in why_counts {
        println!("  {kind}: {count}");
    }
    Ok(())
}

/// `labels <folder> [all] [e|a|o]`: the elements offered with no line in
/// `piece_tags.txt` (those almost like another go by its line; all of them:
/// `all`; of one kind only: its letter), on sheets to label them by eye.
fn labels(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let said = "labels <folder> [all] [e|a|o]";
    let Some((folder, more)) = rest.split_first() else {
        return Err(usage(said));
    };
    let tables = decor::pieces_folder(&asked.project);
    if more.first().is_some_and(|word| word == "places") {
        return places_sheets(asked, &tables, folder, &more[1..]);
    }
    let all = more.iter().any(|word| word == "all");
    let letter = more.iter().find(|word| word.len() == 1).and_then(|word| word.chars().next());
    let workshop = asked.workshop()?;
    let labels = decor::tags::labels(&tables);
    let aside = decor::sort::set_aside(&tables);
    let mut chosen: Vec<&Piece> = decor::offered(&workshop.catalogue, &aside)
        .into_iter()
        .filter(|piece| all || (decor::tags::label_of(&piece.name, &labels, &aside).is_none() && !aside.contains_key(&piece.name)))
        .filter(|piece| letter.is_none_or(|letter| piece.name.starts_with(letter)))
        .collect();
    chosen.sort_by_key(|piece| (piece.name.chars().next(), level_of(piece).trim_start_matches('o').parse::<i64>().unwrap_or(-1), piece.line));
    let names: Vec<String> = chosen.iter().map(|piece| piece.name.clone()).collect();
    let (sheets, failed) = decor::sheets::pieces::label_sheets(&workshop, &names, std::path::Path::new(folder))?;
    for sheet in &sheets {
        println!("{}", sheet.display());
    }
    if !failed.is_empty() {
        println!("not cut: {}", failed.join(" "));
    }
    println!("{} element(s) on {} sheet(s)", names.len(), sheets.len());
    Ok(())
}

/// `labels <folder> places [level ...]`: the game's places named (none: those
/// with no line in `place_kinds.txt`, menus and visions left out), on sheets
/// to tell their kinds by eye.
fn places_sheets(asked: &Asked, tables: &std::path::Path, folder: &str, levels: &[String]) -> Result<(), Failed> {
    let asked_levels = numbers(levels).ok_or_else(|| usage("labels <folder> places [level ...]"))?;
    let kinds = decor::tags::PlaceKinds::of(tables);
    // (The game's own places: the ROM's list, not the decors built over it.)
    let rom = std::fs::read_to_string(std::path::Path::new(&asked.data).join("rom").join("backdrops.txt")).unwrap_or_default();
    let places: Vec<(i64, String)> = rom
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter_map(|(level, rest)| Some((level.parse::<i64>().ok()?, rest.split('\t').next()?.to_string())))
        .filter(|(level, code)| {
            if asked_levels.is_empty() { !decor::sort::no_place(code) && !kinds.by_key.contains_key(&level.to_string()) } else { asked_levels.contains(level) }
        })
        .collect();
    for sheet in decor::sheets::pieces::place_sheets(&asked.data, &places, std::path::Path::new(folder))? {
        println!("{}", sheet.display());
    }
    println!("{} place(s)", places.len());
    Ok(())
}

/// `terrains <level> [letters]`.
fn terrains(asked: &Asked, rest: &[String]) -> Result<(), Failed> {
    let (level, letters) = match rest {
        [level] => (level, ""),
        [level, letters] => (level, letters.as_str()),
        _ => return Err(usage("terrains <level> [letters]")),
    };
    let level: i64 = level.parse().map_err(|_| usage("terrains <level> [letters]"))?;
    println!("{}", decor::sheets::terrains(&asked.workshop()?, level, letters)?);
    Ok(())
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failed::Usage(said)) => {
            eprintln!("usage: decor {said}");
            ExitCode::from(2)
        }
        Err(Failed::Wrong(why)) => {
            eprintln!("{why}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{numbers, take, why_kind};

    /// A word's value taken off the command; numbers read; why a piece is set aside, by its kind.
    #[test]
    fn the_command_is_read() {
        let mut words: Vec<String> = ["build", "--from", "x.decor", "beach"].map(String::from).to_vec();
        assert_eq!(take(&mut words, "--from").ok().flatten().as_deref(), Some("x.decor"));
        assert_eq!(words, ["build", "beach"]);
        assert!(take(&mut vec!["--out".to_string()], "--out").is_err());
        assert_eq!(numbers(&["3".to_string(), "-4".to_string()]), Some(vec![3, -4]));
        assert_eq!(numbers(&["3".to_string(), "x".to_string()]), None);
        assert_eq!(why_kind("too small (12 pixels)"), "too small");
        assert_eq!(why_kind("a copy of tree_3"), "a copy");
        assert_eq!(why_kind("drawn like tree_3"), "drawn");
    }
}
