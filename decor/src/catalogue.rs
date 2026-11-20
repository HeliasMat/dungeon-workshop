//! The pieces a place is made of (`library/tables/pieces.txt`, then those the
//! game has, `data/extra/game_pieces.txt`), cut from the game's own places, in the order
//! they are written: where a choice goes to the first, it is this order's.
//!
//! ```text
//! name  kind  level  x0 y0 x1 y1  [option=value ...]  # what it is
//! ```

use crate::fault::{Diagnostic, Location};
use std::collections::HashMap;
use std::path::Path;

/// The kinds of pieces.
pub const KINDS: [&str; 6] = ["ground", "edge", "path", "water", "object", "tree"];
/// The options a piece may have.
pub const OPTIONS: [&str; 32] = [
    "on",
    "base",
    "shadow",
    "without",
    "upper",
    "flip",
    "replace",
    "wall",
    "walk",
    "foot",
    "also",
    "inside",
    "flippable",
    "grid",
    "extract",
    "tints",
    "frequent",
    "rare",
    "alone",
    "crumb",
    "whole",
    "keep",
    "in",
    "full",
    "replace_lower",
    "peel",
    "element",
    "lower",
    "moves",
    "rest",
    "over",
    "tiles",
];
/// And one more, `ground_only`, last in the tool's own list.
const GROUND_ONLY: &str = "ground_only";
/// The options a piece's square keeps: how it is laid, not how it is cut.
const SQUARE_KEEPS: [&str; 5] = ["upper", "wall", "walk", "foot", "flip"];

/// Where a piece is cut from: a place of the game, or one of its things (`o394`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    Place(i64),
    Thing(u32),
}

/// A piece of the catalogue.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub name: String,
    pub kind: String,
    pub level: Level,
    pub rect: (i64, i64, i64, i64),
    /// Its options, in the order written.
    pub options: Vec<(String, String)>,
    pub line: usize,
    pub note: String,
}

impl Piece {
    pub fn option(&self, key: &str) -> Option<&str> {
        self.options.iter().find(|(known, _)| known == key).map(|(_, value)| value.as_str())
    }

    pub fn has(&self, key: &str) -> bool {
        self.option(key).is_some()
    }

    /// Its line in pieces.txt.
    pub fn where_(&self) -> Location {
        Location::in_file("pieces.txt", self.line)
    }

    pub fn place(&self) -> Option<i64> {
        match self.level {
            Level::Place(level) => Some(level),
            Level::Thing(_) => None,
        }
    }
}

/// The pieces by name, in the order they come.
#[derive(Clone, Debug, Default)]
pub struct Catalogue {
    pub pieces: Vec<Piece>,
    by_name: HashMap<String, usize>,
    /// How many, first, are named by hand (and their squares): the others
    /// were found by the inventory, their numbers may change.
    named: usize,
}

impl Catalogue {
    pub fn get(&self, name: &str) -> Option<&Piece> {
        self.by_name.get(name).map(|at| &self.pieces[*at])
    }

    fn add(&mut self, piece: Piece) {
        if !self.by_name.contains_key(&piece.name) {
            self.by_name.insert(piece.name.clone(), self.pieces.len());
            self.pieces.push(piece);
        }
    }

    /// Whether it was found by the inventory (`l323_7`), not named by hand.
    pub fn found(&self, piece: &Piece) -> bool {
        self.by_name.get(&piece.name).is_some_and(|at| *at >= self.named)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.pieces.iter().map(|piece| piece.name.as_str())
    }

    /// Those named by hand only (pieces.txt), as written.
    pub fn read_named(folder: &Path) -> Result<Vec<Piece>, Diagnostic> {
        read_one(&folder.join("pieces.txt"))
    }

    /// Those named by hand (pieces.txt), each thing as its square too, then
    /// every thing the game has (`data/extra/game_pieces.txt`), if found.
    pub fn read(folder: &Path, data: &str) -> Result<Catalogue, Diagnostic> {
        let mut catalogue = Catalogue::default();
        let named = read_one(&folder.join("pieces.txt"))?;
        let squares = squared(&named);
        for piece in named.into_iter().chain(squares) {
            catalogue.add(piece);
        }
        catalogue.named = catalogue.pieces.len();
        let found = crate::game_pieces(data);
        if found.exists() {
            for piece in read_one(&found)? {
                catalogue.add(piece);
            }
        }
        Ok(catalogue)
    }
}

/// Each thing named by hand as its square too, `<name>_square`: the whole
/// rectangle of tiles round it, as the game draws it there.
fn squared(pieces: &[Piece]) -> Vec<Piece> {
    let mut found = Vec::new();
    for piece in pieces {
        if !["object", "water", "tree"].contains(&piece.kind.as_str()) || piece.place().is_none() || piece.has("base") || piece.has("extract") {
            continue;
        }
        let mut options: Vec<(String, String)> = piece.options.iter().filter(|(key, _)| SQUARE_KEEPS.contains(&key.as_str())).cloned().collect();
        // (Its upper layer with it, but every row whole: none with that layer only.)
        for (key, value) in options.iter_mut() {
            if key == "upper" {
                value.clear();
            }
        }
        options.retain(|(key, _)| key != "base");
        options.push(("base".to_string(), String::new()));
        let name = format!("{}_square", piece.name);
        found.push(Piece { name, options, note: format!("{} (on the grid)", piece.note), ..piece.clone() });
    }
    found
}

/// The pieces of one file, as written.
fn read_one(path: &Path) -> Result<Vec<Piece>, Diagnostic> {
    let file = path.file_name().map_or(String::new(), |name| name.to_string_lossy().into_owned());
    let text = std::fs::read_to_string(path).map_err(|why| Diagnostic::said(format!("{file} : {why}")))?;
    let mut pieces: Vec<Piece> = Vec::new();
    for (number, line) in text.lines().enumerate().map(|(at, line)| (at + 1, line)) {
        let (said, note) = line.split_once('#').unwrap_or((line, ""));
        let words: Vec<&str> = said.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        let at = Location::in_file(file.clone(), number);
        if words.len() < 7 {
            return Err(at.wrong("name kind level x0 y0 x1 y1 are needed"));
        }
        let (name, kind) = (words[0], words[1]);
        if !KINDS.contains(&kind) {
            return Err(at.wrong(format!("unknown kind \"{kind}\" ({})", KINDS.join(", "))));
        }
        if let Some(known) = pieces.iter().find(|piece| piece.name == name) {
            return Err(at.wrong(format!("\"{name}\" is already line {}", known.line)));
        }
        let level = match words[2].strip_prefix('o').filter(|thing| !thing.is_empty() && thing.chars().all(|letter| letter.is_ascii_digit())) {
            Some(thing) => thing.parse().ok().map(Level::Thing),
            None => words[2].parse().ok().map(Level::Place),
        };
        let rect: Vec<i64> = words[3..7].iter().filter_map(|word| word.parse().ok()).collect();
        let (Some(level), [x0, y0, x1, y1]) = (level, rect.as_slice()) else {
            return Err(at.wrong("level (or o<object>) and rectangle in numbers"));
        };
        let mut options = Vec::new();
        for word in &words[7..] {
            let (key, value) = word.split_once('=').unwrap_or((word, ""));
            if !OPTIONS.contains(&key) && key != GROUND_ONLY {
                return Err(at.wrong(format!("unknown option \"{key}\"")));
            }
            // (Said twice: where it was first, the last value, as a dict keeps it.)
            match options.iter_mut().find(|(known, _): &&mut (String, String)| known == key) {
                Some((_, kept)) => *kept = value.to_string(),
                None => options.push((key.to_string(), value.to_string())),
            }
        }
        pieces.push(Piece {
            name: name.to_string(),
            kind: kind.to_string(),
            level,
            rect: (*x0, *y0, *x1, *y1),
            options,
            line: number,
            note: note.trim().to_string(),
        });
    }
    Ok(pieces)
}
