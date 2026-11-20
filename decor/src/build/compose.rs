//! A decor made of the game's own pieces, built simply: each line laid in
//! its order, nothing guessed. The pieces are those the ROM tells apart
//! (the upper layer's objects, the animated tiles, the things of the
//! scenes, a place's rest), each cut once and kept; a ground is a piece
//! repeated over its zone; walls painted by hand the last word.
//!
//! - `put <piece> x y[, x y…]` (turned, still, without its shadow, on a
//!   layer, named for the films): laid as the game draws it, both its
//!   layers, every frame of it, its colours that change with time, its walls.
//! - `ground <piece> [zone x0 y0 x1 y1]`: that piece repeated, tile after tile.
//! - `terrain <level> x y …`: terrains painted of a place's own tiles
//!   (`crate::terrain`), their walls with them.
//! - `wall …`, `free …`: walls said by hand.
//!
//! A decor saying anything else (a place of the game under all, edges,
//! paths, water, strewn things, our own pictures, an element living as we
//! make it live) is built by the full builder (`Builder`).

use std::rc::Rc;

use super::place::{Place, lcm};
use super::{MOST_FRAMES, walled};
use crate::cut::Cut;
use crate::fault::{Diagnostic, Location};
use crate::lang::{Decor, Line, points_of, zone_of};
use crate::pixels::TILE;
use crate::weave::Workshop;

/// Whether a decor says only what this builder lays.
pub fn simple(decor: &Decor) -> bool {
    decor.base.is_none()
        && decor.plan.is_empty()
        && decor.lines.iter().all(|line| match line.verb.as_str() {
            "put" => line.animation.is_none(),
            "ground" => line.words.len() == 1 || line.words.get(1).is_some_and(|word| word == "zone"),
            "wall" | "free" | "terrain" => true,
            _ => false,
        })
}

/// A terrain line's piece: its tiles solved.
pub(crate) fn terrain_cut(workshop: &Workshop, line: &Line, at: &Location) -> Result<Rc<Cut>, Diagnostic> {
    let painted = crate::terrain::Painted::read(&line.words).map_err(|why| at.wrong(why))?;
    Ok(Rc::new(crate::terrain::cut(workshop, &painted).map_err(|why| why.within(at.clone()))?))
}

/// A terrain laid at its corner, on the ground, its walls with it.
pub(crate) fn lay_terrain(workshop: &Workshop, place: &mut Place, line: &Line, at: &Location) -> Result<(), Diagnostic> {
    let painted = crate::terrain::Painted::read(&line.words).map_err(|why| at.wrong(why))?;
    let cut = terrain_cut(workshop, line, at)?;
    place.lay_cut(workshop, &cut, painted.x, painted.y, 0, None)
}

/// A piece as this line lays it: turned over, without its shadow, still.
fn as_laid(workshop: &Workshop, line: &Line, at: &Location) -> Result<Rc<Cut>, Diagnostic> {
    let name = &line.words[0];
    let cut = crate::laid_cut(workshop, name).map_err(|why| why.within(at.clone()))?;
    if !line.mirrored && line.shadow && !line.still {
        return Ok(cut);
    }
    let mut made = (*cut).clone();
    if line.mirrored {
        made = made.mirrored();
    }
    if !line.shadow {
        made = made.unshaded();
    }
    if line.still {
        made.lower.truncate(1);
        made.upper.truncate(1);
        (made.lower_index, made.upper_index) = (None, None);
    }
    Ok(Rc::new(made))
}

/// The decor built: its lines laid in order, its walls, cut to its size.
pub fn composed(workshop: &Workshop, decor: Decor) -> Result<Place, Diagnostic> {
    // (Laid on whole tiles of 8, cut to its size at the end.)
    let said = (decor.width, decor.height);
    let mut decor = decor;
    (decor.width, decor.height) = (said.0.next_multiple_of(TILE), said.1.next_multiple_of(TILE));
    let at = |line: &Line| Location::in_file(decor.file.clone(), line.number);
    // (Every piece first: how many frames the place has, and how fast.)
    let mut cuts: Vec<Option<Rc<Cut>>> = Vec::new();
    for line in &decor.lines {
        let made = match line.verb.as_str() {
            "put" | "ground" => Some(as_laid(workshop, line, &at(line))?),
            "terrain" => Some(terrain_cut(workshop, line, &at(line))?),
            _ => None,
        };
        cuts.push(made);
    }
    let frames: Vec<usize> = cuts.iter().flatten().map(|cut| cut.lower.len().max(1)).collect();
    let mut place = Place::new(decor.clone(), lcm(if frames.is_empty() { &[1] } else { &frames }).min(MOST_FRAMES));
    // (As fast as the piece of the most frames.)
    place.seconds = cuts.iter().flatten().filter(|cut| cut.lower.len() > 1).max_by_key(|cut| cut.lower.len()).map_or(1.0 / 6.0, |cut| cut.seconds);
    for (line, cut) in decor.lines.iter().zip(&cuts) {
        let Some(cut) = cut else {
            if line.words.first().is_none_or(|word| word != "tiles") {
                walled(&mut place, line, &at(line))?;
            }
            continue;
        };
        let name = &line.words[0];
        match line.verb.as_str() {
            "terrain" => lay_terrain(workshop, &mut place, line, &at(line))?,
            "ground" => {
                let (x0, y0, x1, y1) = match line.words.len() {
                    1 => (0, 0, decor.width as i64, decor.height as i64),
                    _ => zone_of(&at(line), &line.words[1..], "ground <piece> [zone x0 y0 x1 y1]")?,
                };
                let (w, h) = cut.size();
                let (w, h) = (w.max(1) as i64, h.max(1) as i64);
                let mut y = y0;
                while y < y1 {
                    let mut x = x0;
                    while x < x1 {
                        place.lay_cut(workshop, cut, x, y, 0, line.layer.as_deref())?;
                        x += w;
                    }
                    y += h;
                }
            }
            _ => {
                for (x, y) in points_of(&at(line), &line.words[1..])? {
                    let (x, y) = if line.foot { (x - cut.foot.0, y - cut.foot.1) } else { (x, y) };
                    let owner = place.names.len() as i32;
                    place.names.push(name.clone());
                    place.lay_cut(workshop, cut, x, y, owner, line.layer.as_deref())?;
                    let (w, h) = cut.size();
                    if x < 0 || y < 0 || x + w as i64 > decor.width as i64 || y + h as i64 > decor.height as i64 {
                        place.remark(line.number, format!("\"{name}\" goes past the place at {x} {y}"));
                    }
                    // (Named: a spot of the films at its foot.)
                    if let Some(spot) = &line.name {
                        place.decor.entrances.push((line.number, x + w as i64 / 2, y + h as i64 + TILE as i64, spot.clone()));
                    }
                }
            }
        }
    }
    // (Tiles painted by hand: the last word.)
    for line in decor.lines.iter().filter(|line| matches!(line.verb.as_str(), "wall" | "free") && line.words.first().is_some_and(|word| word == "tiles")) {
        walled(&mut place, line, &at(line))?;
    }
    place.cut_to(said.0, said.1);
    Ok(place)
}
