//! The pieces worth offering in the Workshop, sorted out of all the catalogue
//! has (`decor tri`): only those the ROM itself tells apart (the upper
//! layer's objects, `e…`; the animated tiles, `a…`; the things the scripts
//! lay, `o…`; and each place's rest, `r…`, backstage); each cut as it is
//! laid, then set aside if it is a speck, of a screen that is no place
//! (a menu, a vision), or the same as one kept already (to the pixel, or
//! near enough: a tent of each of the town's times). Written in `library/tables/set_aside.txt`, a line a piece set aside
//! and why; the catalogue stays whole (a place of the game is remade of all
//! its pieces, and a decor lays any piece by its name).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::catalogue::Piece;
use crate::cut::cut;
use crate::pixels::Picture;
use crate::weave::Workshop;

/// Fewer pixels than this: a speck.
const SPECK: usize = 64;
/// Two pictures this near (bits apart of their 16 × 16 marks), of sizes
/// this near: the same.
const NEAR_BITS: u32 = 12;
const NEAR_SIZE: usize = 4;

/// How a piece looks once cut: its first frame, both layers, cropped to
/// what is drawn; how many frames it has.
pub struct Look {
    picture: Picture,
    frames: usize,
}

/// A piece's first frame, its upper layer over its lower one, cropped to
/// what is drawn; none if nothing is.
fn look_of(lower: &Picture, upper: &Picture, frames: usize) -> Option<Look> {
    let (w, h) = (lower.w, lower.h);
    let seen = |x: usize, y: usize| {
        let over = if upper.w == w && upper.h == h { upper.get(x, y) } else { [0; 4] };
        if over[3] > 0 { over } else { lower.get(x, y) }
    };
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if seen(x, y)[3] > 0 {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    if x1 <= x0 {
        return None;
    }
    let mut picture = Picture::new(x1 - x0, y1 - y0);
    for y in y0..y1 {
        for x in x0..x1 {
            picture.set(x - x0, y - y0, seen(x, y));
        }
    }
    Some(Look { picture, frames })
}

/// The places of the game that are no places (its menus, `S99…`, and its
/// visions, `V…`): their pieces are writing and flashes, not things.
pub fn no_place(code: &str) -> bool {
    code.starts_with("S99") || code.starts_with('V')
}

/// Why a piece is no good to lay, if it is not: a speck.
fn fault_of(full: &Picture) -> Option<String> {
    let count = full.cells.iter().filter(|rgba| rgba[3] > 0).count();
    (count < SPECK).then(|| format!("too small ({count} pixels)"))
}

/// A picture's mark: 16 × 16 cells, each lighter than the mean or not (its
/// clear pixels dark).
fn mark_of(picture: &Picture) -> [bool; 256] {
    let mut cells = [0.0f64; 256];
    for y in 0..picture.h {
        for x in 0..picture.w {
            let [red, green, blue, seen] = picture.get(x, y);
            let light = if seen > 0 { red as f64 * 0.3 + green as f64 * 0.6 + blue as f64 * 0.1 } else { 0.0 };
            cells[(y * 16 / picture.h) * 16 + x * 16 / picture.w] += light;
        }
    }
    let mean = cells.iter().sum::<f64>() / 256.0;
    let mut mark = [false; 256];
    for (at, cell) in cells.iter().enumerate() {
        mark[at] = *cell > mean;
    }
    mark
}

/// What a picture is to the pixel, with how many frames it has: two the same have the same key.
fn key_of(picture: &Picture, frames: usize) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (picture.w, picture.h, frames).hash(&mut hasher);
    picture.cells.hash(&mut hasher);
    hasher.finish()
}

/// A picture turned over, left to right.
fn mirrored(picture: &Picture) -> Picture {
    let mut out = picture.clone();
    for row in out.cells.chunks_mut(picture.w.max(1)) {
        row.reverse();
    }
    out
}

/// The piece a piece set aside is the same as, to the pixel (turned over or
/// not), as `set_aside.txt` says it: `duplicate of e12_3`, `flipped duplicate of o40`.
pub fn twin_of(why: &str) -> Option<&str> {
    why.strip_prefix("duplicate of ").or_else(|| why.strip_prefix("flipped duplicate of ")).map(str::trim).filter(|name| !name.is_empty())
}

/// Whether the Workshop offers a piece, by why it is set aside (none: it is
/// kept): kept, or only almost like another (the tent of another time of day).
pub fn offered(why: Option<&str>) -> bool {
    why.is_none_or(|why| why.starts_with("almost"))
}

/// Each piece offered, the pieces set aside as the same as it, to the pixel
/// (the same tree in other places): one element, all its names.
pub fn twins(aside: &HashMap<String, String>) -> HashMap<String, Vec<String>> {
    let mut found: HashMap<String, Vec<String>> = HashMap::new();
    for (name, why) in aside {
        if let Some(kept) = twin_of(why) {
            found.entry(kept.to_string()).or_default().push(name.clone());
        }
    }
    for names in found.values_mut() {
        names.sort();
    }
    found
}

/// A piece kept, as the near twins are told: its name, size and mark.
type Kept = (String, usize, usize, [bool; 256]);

/// The pieces set aside, each with why: none named by hand, nor a ground,
/// an edge or a path; the others in the order they are offered (the whole
/// elements before the things found alone), the first of the same kept.
pub fn sorted(workshop: &Workshop) -> Vec<(String, String)> {
    let catalogue = &workshop.catalogue;
    let rank = |piece: &Piece| match piece.name.chars().next() {
        _ if !catalogue.found(piece) => 0,
        Some('e') => 1,
        Some('a') => 2,
        Some('o') => 3,
        Some('r') => 4,
        _ => 5,
    };
    // (Grounds, edges and paths are tools to paint with, not elements: kept.)
    let mut pieces: Vec<&Piece> = catalogue.pieces.iter().filter(|piece| !matches!(piece.kind.as_str(), "ground" | "edge" | "path")).collect();
    // (By rank, then place by place: each place read once, then forgotten.)
    let level = |piece: &Piece| match piece.level {
        crate::catalogue::Level::Place(level) => level,
        crate::catalogue::Level::Thing(_) => -1,
    };
    // (Of twins, the one of the place of the larger number kept: the first
    // places are early drafts of later ones, under names ill guessed.)
    pieces.sort_by_key(|piece| (rank(piece), std::cmp::Reverse(level(piece))));
    let mut reading = None;
    let total = pieces.len();
    // (Each place's code, `T01P01A`: its menus and visions are no places.)
    let codes: HashMap<i64, String> =
        workshop.game.rows("backdrops.txt").into_iter().filter_map(|row| Some((row.first()?.parse().ok()?, row.get(1)?.clone()))).collect();
    let mut aside = Vec::new();
    let mut same: HashMap<u64, String> = HashMap::new();
    let mut near: HashMap<(usize, usize), Vec<Kept>> = HashMap::new();
    for (done, piece) in pieces.into_iter().enumerate() {
        if reading != Some((rank(piece), level(piece))) {
            workshop.forget();
            reading = Some((rank(piece), level(piece)));
        }
        if done % 250 == 0 {
            eprintln!("sorting: {done} / {total} {}", piece.name);
        }
        let named = !catalogue.found(piece);
        // (Only those made to remake the game's places: its elements whole
        // and the rest of each place; the pieces of before, all set aside.)
        let exact = (piece.name.starts_with('e') && piece.has("over"))
            || (piece.name.starts_with('a') && piece.has("tiles"))
            || piece.name.starts_with('o')
            || piece.has("rest");
        if !exact {
            aside.push((piece.name.clone(), "not a piece the ROM keeps apart".to_string()));
            continue;
        }
        if codes.get(&level(piece)).is_some_and(|code| no_place(code)) {
            aside.push((piece.name.clone(), "from a screen of the game, not a place (menu, vision)".to_string()));
            continue;
        }
        // (The place whole: a background, not an element.)
        if piece.name.ends_with("_tout") {
            aside.push((piece.name.clone(), "the whole place".to_string()));
            continue;
        }
        let made = match cut(workshop, piece) {
            Ok(made) => made,
            Err(_) if named => continue,
            Err(_) => {
                aside.push((piece.name.clone(), "cannot be cut out".to_string()));
                continue;
            }
        };
        let upper = made.upper.first().cloned().unwrap_or_else(|| Picture::new(0, 0));
        let lower = &made.lower[0];
        let mut full = lower.clone();
        if upper.w == full.w && upper.h == full.h {
            for (cell, over) in full.cells.iter_mut().zip(&upper.cells) {
                if over[3] > 0 {
                    *cell = *over;
                }
            }
        }
        let Some(look) = look_of(lower, &upper, made.lower.len()) else {
            if !named {
                aside.push((piece.name.clone(), "nothing drawn".to_string()));
            }
            continue;
        };
        if !named && let Some(why) = fault_of(&full) {
            aside.push((piece.name.clone(), why));
            continue;
        }
        // (The same to the pixel as one kept, or as one kept turned over.)
        let key = key_of(&look.picture, look.frames);
        if let Some(first) = same.get(&key) {
            if !named {
                aside.push((piece.name.clone(), format!("duplicate of {first}")));
            }
            continue;
        }
        if let Some(first) = same.get(&key_of(&mirrored(&look.picture), look.frames)) {
            if !named {
                aside.push((piece.name.clone(), format!("flipped duplicate of {first}")));
            }
            continue;
        }
        same.insert(key, piece.name.clone());
        // (Near enough to one kept: its size and its marks.)
        let mark = mark_of(&look.picture);
        let (w, h) = (look.picture.w, look.picture.h);
        let bucket = (w / 16, h / 16);
        let mut twin = None;
        for dx in 0..3usize {
            for dy in 0..3usize {
                let (bx, by) = ((bucket.0 + dx).wrapping_sub(1), (bucket.1 + dy).wrapping_sub(1));
                for (other, ow, oh, other_mark) in near.get(&(bx, by)).into_iter().flatten() {
                    let apart = mark.iter().zip(other_mark).filter(|(a, b)| a != b).count() as u32;
                    if w.abs_diff(*ow) <= NEAR_SIZE && h.abs_diff(*oh) <= NEAR_SIZE && apart <= NEAR_BITS {
                        twin = Some(other.clone());
                    }
                }
            }
        }
        match twin {
            Some(other) if !named => aside.push((piece.name.clone(), format!("almost like {other}"))),
            _ => near.entry(bucket).or_default().push((piece.name.clone(), w, h, mark)),
        }
    }
    aside
}

/// For pieces named, how they are judged: their size, squareness, fill and
/// why they would be set aside (`decor sort <piece> ...`).
pub fn measured(workshop: &Workshop, names: &[String]) -> Vec<String> {
    names
        .iter()
        .map(|name| {
            let Some(piece) = workshop.catalogue.get(name) else {
                return format!("{name}: no piece");
            };
            let Ok(made) = cut(workshop, piece) else {
                return format!("{name}: cannot be cut out");
            };
            let upper = made.upper.first().cloned().unwrap_or_else(|| Picture::new(0, 0));
            let mut full = made.lower[0].clone();
            if upper.w == full.w && upper.h == full.h {
                for (cell, over) in full.cells.iter_mut().zip(&upper.cells) {
                    if over[3] > 0 {
                        *cell = *over;
                    }
                }
            }
            let Some(look) = look_of(&made.lower[0], &upper, made.lower.len()) else {
                return format!("{name}: nothing drawn");
            };
            let fault = fault_of(&full).unwrap_or_else(|| "kept".to_string());
            format!("{name}: {} × {}, {} images — {fault}", look.picture.w, look.picture.h, look.frames)
        })
        .collect()
}

/// Each element of the game's places as it is laid, written for a look at
/// them all (`decor apercus <dossier>`): `<dossier>/<level>/<name>.png`, its
/// first frame, both layers, cropped to what it draws; place by place, each
/// forgotten once done. How many written.
pub fn previews(workshop: &Workshop, out: &std::path::Path) -> Result<usize, crate::fault::Diagnostic> {
    let mut pieces: Vec<&Piece> = workshop
        .catalogue
        .pieces
        .iter()
        .filter(|piece| (piece.name.starts_with('e') && piece.has("over")) || (piece.name.starts_with('a') && piece.has("tiles")))
        .collect();
    let level = |piece: &Piece| match piece.level {
        crate::catalogue::Level::Place(level) => level,
        crate::catalogue::Level::Thing(_) => -1,
    };
    pieces.sort_by_key(|piece| level(piece));
    let mut reading = None;
    let mut written = 0;
    for piece in pieces {
        if reading != Some(level(piece)) {
            workshop.forget();
            reading = Some(level(piece));
        }
        let Ok(made) = cut(workshop, piece) else {
            continue;
        };
        let upper = made.upper.first().cloned().unwrap_or_else(|| Picture::new(0, 0));
        let Some(look) = look_of(&made.lower[0], &upper, made.lower.len()) else {
            continue;
        };
        let folder = out.join(level(piece).to_string());
        std::fs::create_dir_all(&folder).map_err(|why| crate::fault::Diagnostic::said(format!("{} : {why}", folder.display())))?;
        let path = folder.join(format!("{}.png", piece.name));
        let bytes: Vec<u8> = look.picture.cells.iter().flatten().copied().collect();
        image::save_buffer(&path, &bytes, look.picture.w as u32, look.picture.h as u32, image::ColorType::Rgba8)
            .map_err(|why| crate::fault::Diagnostic::said(format!("{} : {why}", path.display())))?;
        written += 1;
    }
    Ok(written)
}

/// The pieces set aside, as `library/tables/set_aside.txt` says: name, why.
pub fn set_aside(folder: &std::path::Path) -> HashMap<String, String> {
    std::fs::read_to_string(folder.join("set_aside.txt"))
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let (name, why) = line.split_once(char::is_whitespace)?;
            Some((name.to_string(), why.trim().to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pieces the same as another, turned over or not, are its twins; the almost alike stay offered.
    #[test]
    fn twins_are_one_element() {
        let aside: HashMap<String, String> = [
            ("e1_1", "duplicate of e9_1"),
            ("e2_4", "flipped duplicate of e9_1"),
            ("e3_1", "almost like e9_1"),
            ("e4_1", "too small (12 pixels)"),
            ("o7", "duplicate of o8"),
        ]
        .into_iter()
        .map(|(name, why)| (name.to_string(), why.to_string()))
        .collect();
        let twins = twins(&aside);
        assert_eq!(twins["e9_1"], ["e1_1", "e2_4"]);
        assert_eq!(twins["o8"], ["o7"]);
        assert_eq!(twins.len(), 2);
        assert!(offered(None) && offered(Some("almost like e9_1")));
        assert!(!offered(Some("duplicate of e9_1")) && !offered(Some("too small (12 pixels)")));
        let mut picture = Picture::new(2, 1);
        picture.set(0, 0, [1, 2, 3, 255]);
        assert_eq!(mirrored(&picture).get(1, 0), [1, 2, 3, 255]);
        assert_eq!(key_of(&mirrored(&mirrored(&picture)), 1), key_of(&picture, 1));
    }

    #[test]
    fn a_mark_tells_alike_from_unlike() {
        let mut a = Picture::new(32, 32);
        for y in 0..16 {
            for x in 0..32 {
                a.set(x, y, [250, 250, 250, 255]);
            }
        }
        let mut b = a.clone();
        b.set(0, 31, [250, 250, 250, 255]);
        let mut c = Picture::new(32, 32);
        for y in 16..32 {
            for x in 0..32 {
                c.set(x, y, [250, 250, 250, 255]);
            }
        }
        let apart = |p: &Picture, q: &Picture| mark_of(p).iter().zip(mark_of(q).iter()).filter(|(x, y)| x != y).count() as u32;
        assert!(apart(&a, &b) <= NEAR_BITS);
        assert!(apart(&a, &c) > NEAR_BITS);
    }
}
