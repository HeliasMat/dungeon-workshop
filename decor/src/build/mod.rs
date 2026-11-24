//! Building a place from its file with the pieces of the catalogue: grounds,
//! bands and paths woven, things laid, trees round it, line after line, on a
//! `Place` (`place`); and what is remarked once all is laid (`remarks`).

pub mod compose;

use std::rc::Rc;

use crate::cut::cut;
use crate::fault::{Diagnostic, Location};
use crate::lang::{Decor, Line, closest, numbers, points_of, read_path, said, zone_of};
use crate::piece::Cut;
use crate::pixels::{Grid, Mask, TILE, grown};
use crate::random::Rng;
use crate::weave::{Band, FRINGE, Loom, Woven, as_tiles, path_shape, rounded, weave, weave_columns};
use crate::workshop::Workshop;

use crate::place::{Place, lcm};

/// The most frames a place of ours has.
const MOST_FRAMES: usize = 24;
/// A tile woven with a seam costing more than this is a seam.
const SEAM: f64 = 120.0;

/// The CRC-32 of a name, as zlib makes it: a place's chance, by default.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { crc >> 1 ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// What a line makes of a piece.
#[derive(Clone, Copy, PartialEq)]
enum How {
    Loom,
    Cut,
}

/// Builds a place from its file.
pub struct Builder<'a> {
    workshop: &'a Workshop,
    decor: Decor,
    seed: u32,
}

impl<'a> Builder<'a> {
    pub fn new(workshop: &'a Workshop, decor: Decor) -> Builder<'a> {
        let seed = decor.seed.map_or_else(|| crc32(decor.name.as_bytes()), |seed| seed as u32);
        Builder { workshop, decor, seed }
    }

    fn rng(&self, line: &Line) -> Rng {
        Rng::new(&[self.seed as u64, line.number as u64])
    }

    fn at(&self, line: &Line) -> Location {
        Location::in_file(self.decor.file.clone(), line.number)
    }

    fn uses(verb: &str) -> (How, &'static [&'static str]) {
        match verb {
            "ground" => (How::Loom, &["ground"]),
            "edge" => (How::Loom, &["edge"]),
            "path" => (How::Loom, &["path"]),
            "water" => (How::Cut, &["water", "object"]),
            "ring" => (How::Cut, &["tree"]),
            "put" => (How::Cut, &["object", "water", "tree"]),
            _ => (How::Cut, &["object"]),
        }
    }

    fn piece(&self, line: &Line, name: &str, kinds: &[&str]) -> Result<crate::catalogue::Piece, Diagnostic> {
        let catalogue = &self.workshop.catalogue;
        let Some(piece) = catalogue.get(name) else {
            let fitting: Vec<&str> = catalogue.pieces.iter().filter(|piece| kinds.contains(&piece.kind.as_str())).map(|piece| piece.name.as_str()).collect();
            let near = closest(name, fitting.iter().copied());
            let near = if near.is_empty() { closest(name, catalogue.names()) } else { near };
            return Err(self.at(line).wrong(format!("unknown piece \"{name}\".{near} (decor pieces)")));
        };
        if !kinds.contains(&piece.kind.as_str()) {
            let mut fitting: Vec<&str> =
                catalogue.pieces.iter().filter(|piece| kinds.contains(&piece.kind.as_str())).map(|piece| piece.name.as_str()).collect();
            fitting.sort_unstable();
            let some: Vec<&str> = fitting.into_iter().take(8).collect();
            return Err(self.at(line).wrong(format!("\"{name}\" is a {}, \"{}\" wants: {} ({}...)", piece.kind, line.verb, kinds.join(", "), some.join(", "))));
        }
        Ok(piece.clone())
    }

    fn loom(&self, line: &Line, name: &str, kinds: &[&str]) -> Result<Rc<Loom>, Diagnostic> {
        let piece = self.piece(line, name, kinds)?;
        self.workshop.loom(&piece).map_err(|why| why.within(self.at(line)))
    }

    /// The piece as this line lays it: turned over, without its shadow.
    fn cut(&mut self, line: &Line, name: &str, kinds: &[&str]) -> Result<Cut, Diagnostic> {
        let piece = self.piece(line, name, kinds)?;
        // (Cut once, kept by the workshop from one build to the next.)
        let known = self.workshop.cuts.borrow().get(name).cloned();
        let made = match known {
            Some(made) => made,
            None => {
                let mut made = cut(self.workshop, &piece).map_err(|why| why.within(self.at(line)))?;
                if piece.has("rest") {
                    made = self.rest_of(&piece, made);
                }
                let made = Rc::new(made);
                self.workshop.cuts.borrow_mut().insert(name.to_string(), made.clone());
                made
            }
        };
        let mut made = (*made).clone();
        if line.mirrored {
            made = made.mirrored();
        }
        if !line.shadow {
            made = made.unshaded();
        }
        // (Still, though the game animates it: its first picture only, its colours as they are.)
        if line.still {
            made.lower.truncate(1);
            made.upper.truncate(1);
            (made.lower_index, made.upper_index) = (None, None);
        }
        if let Some((kind, pace)) = &line.animation {
            made = made.living(kind, pace.as_deref());
        }
        Ok(made)
    }

    pub fn build(mut self) -> Result<Place, Diagnostic> {
        // (Any size: laid on whole tiles of 8, cut to its size at the end.)
        let said = (self.decor.width, self.decor.height);
        (self.decor.width, self.decor.height) = (said.0.next_multiple_of(TILE), said.1.next_multiple_of(TILE));
        // (Every piece first: how many frames the place has.)
        let lines = self.decor.lines.clone();
        let (mut frames, mut seconds) = (Vec::new(), Vec::new());
        // (Elements living as we make them live, at their own pace: not the game's.)
        let mut living = false;
        // (Walls said by hand, our own pictures: no pieces.)
        for line in lines.iter().filter(|line| line.verb == "image" && line.animation.is_some()) {
            frames.push(crate::animate::frames(line.animation.as_ref().and_then(|(_, pace)| pace.as_deref())));
            living = true;
        }
        for line in lines.iter().filter(|line| line.verb == "terrain") {
            let made = compose::terrain_cut(self.workshop, line, &self.at(line))?;
            frames.push(made.lower.len());
            seconds.push((made.lower.len(), made.seconds));
        }
        for line in lines.iter().filter(|line| !matches!(line.verb.as_str(), "wall" | "free" | "image" | "terrain")) {
            let (how, kinds) = Self::uses(&line.verb);
            let (count, speed) = match how {
                How::Loom => {
                    let loom = self.loom(line, &line.words[0], kinds)?;
                    (loom.frames(), loom.seconds)
                }
                How::Cut => {
                    let made = self.cut(line, &line.words[0], kinds)?;
                    (made.lower.len(), made.seconds)
                }
            };
            frames.push(count);
            if line.animation.is_some() {
                living = true;
            } else {
                seconds.push((count, speed));
            }
        }
        // (A place of the game under all: its frames too.)
        if let Some(crate::lang::Base::Level { level, .. }) = &self.decor.base {
            let source = self.workshop.source(*level)?;
            frames.push(source.frames.len());
            seconds.push((source.frames.len(), source.info.seconds as f64));
        }
        let mut place = Place::new(self.decor.clone(), lcm(if frames.is_empty() { &[1] } else { &frames }).min(MOST_FRAMES));
        self.base(&mut place)?;
        // (The speed of the most frames, as `max` of pairs has it: the first of the greatest.)
        let mut fastest: Option<(usize, f64)> = None;
        for pair in &seconds {
            if fastest.is_none_or(|best| pair.0 > best.0 || (pair.0 == best.0 && pair.1 > best.1)) {
                fastest = Some(*pair);
            }
        }
        place.seconds = match fastest {
            Some((count, speed)) if count > 1 || !living => speed,
            _ if living => crate::animate::SECONDS,
            _ => 1.0 / 6.0,
        };
        let mut moving: Vec<f64> = seconds.iter().filter(|(count, _)| *count > 1).map(|(_, speed)| *speed).collect();
        moving.sort_by(f64::total_cmp);
        moving.dedup();
        if moving.len() > 1 {
            let told: Vec<String> = moving.iter().map(|speed| format!("{speed:.3}")).collect();
            place.remark(0, format!("images of different speeds ({} s): the most animated wins", told.join(", ")));
        }
        // (Laid on from the place half laid last time, if it began the same;
        // kept anew just before the first line changed since then.)
        let mark = |said: String| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            said.hash(&mut hasher);
            hasher.finish()
        };
        let begun = mark(format!(
            "{:?} {:?} {:?} {:?} {} {} {:?}",
            (self.decor.width, self.decor.height),
            self.seed,
            self.decor.base,
            self.decor.plan,
            place.lower.len(),
            place.seconds,
            place.levels
        ));
        let marks: Vec<u64> = lines.iter().map(|line| mark(format!("{line:?}"))).collect();
        let before = self.workshop.laid_lines.replace(marks.clone());
        let changed = match before.is_empty() {
            true => marks.len(),
            false => before.iter().zip(&marks).position(|(was, is)| was != is).unwrap_or(before.len().min(marks.len())),
        };
        let mut from = 0;
        if let Some((known, laid, half)) = self.workshop.half_laid.borrow().as_ref()
            && *known == begun
            && marks.starts_with(laid)
        {
            // (Its own file's words as now; the spots its lines laid named, kept.)
            let fresh = place.decor.clone();
            let numbers: Vec<usize> = lines[..laid.len()].iter().map(|line| line.number).collect();
            place = (**half).clone();
            let named: Vec<_> = place.decor.entrances.iter().filter(|spot| numbers.contains(&spot.0) && !fresh.entrances.contains(spot)).cloned().collect();
            place.decor = fresh;
            place.decor.entrances.extend(named);
            from = laid.len();
        }
        for (at, line) in lines.iter().enumerate().skip(from) {
            if at == changed && at > from {
                *self.workshop.half_laid.borrow_mut() = Some((begun, marks[..at].to_vec(), Rc::new(place.clone())));
            }
            match line.verb.as_str() {
                "ground" => self.ground(&mut place, line)?,
                "edge" => self.edge(&mut place, line)?,
                "path" => self.path_line(&mut place, line)?,
                "water" => self.pose(&mut place, line, &["water", "object"])?,
                "put" => self.pose(&mut place, line, &["object", "water", "tree"])?,
                "ring" => self.ring(&mut place, line)?,
                // (Tiles painted by hand: the last word, after all is laid.)
                "wall" | "free" if line.words.first().is_some_and(|word| word == "tiles") => {}
                "wall" | "free" => walled(&mut place, line, &self.at(line))?,
                "image" => self.image(&mut place, line)?,
                "terrain" => compose::lay_terrain(self.workshop, &mut place, line, &self.at(line))?,
                _ => self.strew(&mut place, line)?,
            }
        }
        if changed == lines.len() && changed > from {
            *self.workshop.half_laid.borrow_mut() = Some((begun, marks.clone(), Rc::new(place.clone())));
        }
        for line in lines.iter().filter(|line| matches!(line.verb.as_str(), "wall" | "free") && line.words.first().is_some_and(|word| word == "tiles")) {
            walled(&mut place, line, &self.at(line))?;
        }
        crate::remarks::remark_all(self.workshop, &mut place)?;
        place.cut_to(said.0, said.1);
        Ok(place)
    }

    /// What is under all (`base`): a place of the game, or a dungeon's floor
    /// made of the plan (its spots, and the name it shows if it says none).
    fn base(&mut self, place: &mut Place) -> Result<(), Diagnostic> {
        match self.decor.base.clone() {
            None => {
                if let Some((line, _)) = self.decor.plan.first() {
                    return Err(Location::in_file(self.decor.file.clone(), *line).wrong("a layout without \"base dungeon <dungeon> <floor>\""));
                }
            }
            Some(crate::lang::Base::Level { level, x, y, hollow, .. }) => {
                place.lay_level(&self.workshop.game, level, x, y)?;
                if hollow {
                    self.hollow(place, level, x, y);
                }
            }
            Some(crate::lang::Base::Dungeon { dungeon, floor, line }) => {
                let mut rows = self.decor.plan.clone();
                crate::room::squared(&mut rows);
                let made = crate::room::laid(&self.workshop.game.folder, &dungeon, floor, &rows, &self.decor.file, line)?;
                made.lay_on(place);
                place.decor.entrances.extend(made.spots.iter().cloned());
                place.decor.shown.get_or_insert(made.shown.clone());
            }
        }
        Ok(())
    }

    /// A place of the game under all, its elements taken out (`base level n
    /// creuse`): where each laid its pixels, its ground again, tile by tile
    /// from the nearest of its own; what each had of the upper layer, gone.
    fn hollow(&mut self, place: &mut Place, level: i64, x: i64, y: i64) {
        let (holes, cleared) = self.holes_of(level, place.lower[0].w, place.lower[0].h, x, y);
        for frame in place.upper.iter_mut() {
            for (rgba, gone) in frame.cells.iter_mut().zip(&cleared.cells) {
                if *gone {
                    *rgba = [0; 4];
                }
            }
        }
        place.fill_holes(&holes);
    }

    /// Where a place's elements are, laid at x, y on a picture so large: the
    /// pixels to give back to the ground (those of their lower layer, and
    /// those their upper layer covers in every frame), and those of the
    /// upper layer that are theirs.
    fn holes_of(&mut self, level: i64, w: usize, h: usize, x: i64, y: i64) -> (Mask, Mask) {
        let pieces: Vec<crate::catalogue::Piece> = crate::elements_of(&self.workshop.catalogue, level).into_iter().cloned().collect();
        let (mut holes, mut cleared) = (Mask::new(w, h), Mask::new(w, h));
        for piece in pieces {
            let known = self.workshop.cuts.borrow().get(&piece.name).cloned();
            let made = match known {
                Some(made) => made,
                None => match crate::cut::cut(self.workshop, &piece) {
                    Ok(made) => {
                        let made = Rc::new(made);
                        self.workshop.cuts.borrow_mut().insert(piece.name.clone(), made.clone());
                        made
                    }
                    Err(_) => continue,
                },
            };
            let (x0, y0) = (x + piece.rect.0, y + piece.rect.1);
            let (pw, ph) = made.size();
            for cy in 0..ph {
                for cx in 0..pw {
                    let (px, py) = (x0 + cx as i64, y0 + cy as i64);
                    if !(0..w as i64).contains(&px) || !(0..h as i64).contains(&py) {
                        continue;
                    }
                    let (px, py) = (px as usize, py as usize);
                    // (Its own pixels; and under its upper layer, where the game draws
                    // nothing worth seeing, but what a turning blade lets be seen.)
                    if made.lower.iter().any(|frame| frame.get(cx, cy)[3] > 0) || made.upper.iter().all(|frame| frame.get(cx, cy)[3] > 0) {
                        holes.set(px, py, true);
                    }
                    if made.upper.iter().any(|frame| frame.get(cx, cy)[3] > 0) {
                        cleared.set(px, py, true);
                    }
                }
            }
        }
        (holes, cleared)
    }

    /// The rest of a place of the game (`rest`): the place cut whole, its
    /// elements taken out of it, their room given back to its ground; what
    /// is laid under them, a piece of its own.
    pub(crate) fn rest_of(&mut self, piece: &crate::catalogue::Piece, mut made: Cut) -> Cut {
        let crate::catalogue::Level::Place(level) = piece.level else {
            return made;
        };
        let (w, h) = made.size();
        let (holes, cleared) = self.holes_of(level, w, h, -piece.rect.0, -piece.rect.1);
        for frame in made.upper.iter_mut() {
            for (rgba, gone) in frame.cells.iter_mut().zip(&cleared.cells) {
                if *gone {
                    *rgba = [0; 4];
                }
            }
        }
        let mut index = made.lower_index.take().unwrap_or_default();
        crate::place::fill_holes(&mut made.lower, &mut index, &holes);
        made.lower_index = (!index.is_empty()).then_some(index);
        made
    }

    // --- Woven: grounds, bands, paths ---
    fn weave_into(&self, place: &mut Place, loom: &Loom, line: &Line, walls: Option<&str>, made: Woven) -> Result<(), Diagnostic> {
        let (chosen, costs) = made;
        let level = loom.piece.place().unwrap_or(-1);
        let source = self.workshop.source(level)?;
        let with_index = loom.tiles.first().is_some_and(|tile| tile.indices.is_some());
        let table = if with_index { Some(place.table(&self.workshop.game, level)?) } else { None };
        let laying = place.new_laying();
        let frames = place.lower.len();
        for row in 0..chosen.h {
            for column in 0..chosen.w {
                let at = chosen.get(column, row);
                if at < 0 {
                    continue;
                }
                let at = at as usize;
                let (x, y) = (column * TILE, row * TILE);
                let tile = &loom.tiles[at];
                place.shade(&self.workshop.ramps, &loom.tile_shade(at), x as i64, y as i64);
                let opaque: Vec<bool> = (0..TILE * TILE).map(|cell| tile.frames.iter().any(|frame| frame[cell][3] > 0)).collect();
                for frame in 0..frames {
                    let pixels = &tile.frames[frame % tile.frames.len()];
                    for cell in (0..TILE * TILE).filter(|cell| opaque[*cell]) {
                        let (px, py) = (x + cell % TILE, y + cell / TILE);
                        place.lower[frame].set(px, py, pixels[cell]);
                        let shown = match (&table, &tile.indices) {
                            (Some(table), Some(indices)) => table[indices[frame % tile.frames.len() % indices.len()][cell] as usize],
                            _ => 0,
                        };
                        place.lower_index[frame].set(px, py, shown);
                    }
                }
                for cell in (0..TILE * TILE).filter(|cell| opaque[*cell]) {
                    let (px, py) = (x + cell % TILE, y + cell / TILE);
                    for frame in place.upper.iter_mut() {
                        frame.set(px, py, [0; 4]);
                    }
                    place.origin.set(px, py, laying);
                }
                if walls.is_some() || opaque.iter().all(|cell| *cell) {
                    let (sx, sy, _) = &tile.place;
                    let walled = walls == Some("full")
                        || (walls == Some("game") && source.walls.as_ref().is_some_and(|grid| grid.get(*sx as usize / TILE, *sy as usize / TILE)));
                    place.walls.set(column, row, walled);
                }
                let cost = costs.get(column, row);
                if cost > SEAM {
                    place.seams.push((line.number, x, y, cost as i64));
                }
            }
        }
        place.levels.insert(level);
        Ok(())
    }

    fn ground(&self, place: &mut Place, line: &Line) -> Result<(), Diagnostic> {
        let loom = self.loom(line, &line.words[0], &["ground"])?;
        let (rows, columns) = (place.walls.h, place.walls.w);
        let mut wanted: Grid<i64> = Grid::new(columns, rows);
        if line.words.get(1).is_some_and(|word| word == "tiles") {
            wanted = painted_tiles(&self.at(line), &line.words[2..], rows, columns)?;
        } else if line.words.len() > 1 {
            let zone = zone_of(&self.at(line), &line.words[1..], said("ground"))?;
            wanted = ragged_zone(rows, columns, zone, &mut self.rng(line));
        }
        // (Woven once for all it is woven of; again only if one of them changes.)
        let key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            (&line.words[0], wanted.w, &wanted.cells, place.lower[0].w, &place.lower[0].cells, self.seed, line.number).hash(&mut hasher);
            hasher.finish()
        };
        let known = self.workshop.woven.borrow().get(&key).cloned();
        let made = match known {
            Some(made) => (*made).clone(),
            None => {
                let made = weave(&loom, &wanted, &place.lower[0], &mut self.rng(line), None);
                let mut woven = self.workshop.woven.borrow_mut();
                if woven.len() >= 32 {
                    woven.clear();
                }
                woven.insert(key, Rc::new(made.clone()));
                made
            }
        };
        let walls = loom.piece.option("wall").unwrap_or("none").to_string();
        self.weave_into(place, &loom, line, Some(&walls), made)
    }

    fn edge(&self, place: &mut Place, line: &Line) -> Result<(), Diagnostic> {
        let loom = self.loom(line, &line.words[0], &["edge"])?;
        let side = line.words.get(1).map_or("top", String::as_str);
        let digits = !side.is_empty() && side.chars().all(|letter| letter.is_ascii_digit());
        if side != "top" && side != "bottom" && !digits {
            return Err(self.at(line).wrong(said("edge")));
        }
        let rows = loom.tiles.iter().map(|tile| tile.mark).max().unwrap_or(0) as usize + 1;
        let height = place.walls.h;
        let top = match side {
            "top" => 0i64,
            "bottom" => height as i64 - rows as i64,
            _ => {
                let said: i64 = side.parse().unwrap_or(0);
                if said % TILE as i64 != 0 {
                    return Err(self.at(line).wrong("an edge starts on the grid of 8"));
                }
                said / TILE as i64
            }
        };
        if top < 0 || top as usize + rows > height {
            return Err(self.at(line).wrong(format!("\"{}\" goes out of the place ({} pixels high)", line.words[0], rows * TILE)));
        }
        let top = top as usize;
        let above = top > 0 && place.banded[top - 1];
        let below = top + rows < place.banded.len() && place.banded[top + rows];
        let made = weave_columns(&loom, Band { top, rows, columns: place.walls.w, above, below }, &place.lower[0], &mut self.rng(line))
            .map_err(|why| why.within(self.at(line)))?;
        for band in &mut place.banded[top..top + rows] {
            *band = true;
        }
        let walls = loom.piece.option("wall").unwrap_or("game").to_string();
        self.weave_into(place, &loom, line, Some(&walls), made)
    }

    fn path_line(&self, place: &mut Place, line: &Line) -> Result<(), Diagnostic> {
        let loom = self.loom(line, &line.words[0], &["path"])?;
        let (points, width) = read_path(&self.at(line), &line.words[1..])?;
        let (w, h) = (self.decor.width, self.decor.height);
        let inside = path_shape(&rounded(&points), width, w, h);
        if !inside.any() {
            return Err(self.at(line).wrong("this path does not go through the place"));
        }
        let fringe = grown(&inside, FRINGE, 1).minus(&inside);
        let shape = (as_tiles(&inside), as_tiles(&fringe));
        let (rows, columns) = (place.walls.h, place.walls.w);
        let wanted: Grid<i64> = Grid {
            w: columns,
            h: rows,
            cells: (0..rows * columns)
                .map(|at| if shape.0.cells[at].iter().any(|cell| *cell) || shape.1.cells[at].iter().any(|cell| *cell) { 0 } else { -1 })
                .collect(),
        };
        let made = weave(&loom, &wanted, &place.lower[0], &mut self.rng(line), Some(&shape));
        self.weave_into(place, &loom, line, None, made)?;
        place.path = place.path.or(&inside);
        // (Its tiles inside the path are free to walk.)
        for at in 0..rows * columns {
            if shape.0.cells[at].iter().filter(|cell| **cell).count() * 2 > TILE * TILE {
                place.walls.cells[at] = false;
            }
        }
        place.paths.insert(line.number, inside);
        for (x, y) in [points[0], points[points.len() - 1]] {
            place.ends.push((line.number, x, y));
        }
        Ok(())
    }

    // --- Laid: things, water, trees (at the pixel said) ---
    fn pose(&mut self, place: &mut Place, line: &Line, kinds: &[&str]) -> Result<(), Diagnostic> {
        let name = line.words[0].clone();
        let made = self.cut(line, &name, kinds)?;
        // (A thing found by the inventory goes by a number another inventory may give another.)
        let found_name = name
            .strip_prefix('l')
            .and_then(|rest| rest.split_once('_'))
            .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && a.chars().all(|c| c.is_ascii_digit()) && b.chars().all(|c| c.is_ascii_digit()));
        // (A place of the game remade of its elements: its own, by their numbers, as meant.)
        let rest = |word: &String| word.strip_prefix('r').is_some_and(|level| !level.is_empty() && level.bytes().all(|byte| byte.is_ascii_digit()));
        let remade = matches!(&self.decor.base, Some(crate::lang::Base::Level { hollow: true, .. }))
            || self.decor.lines.iter().any(|line| line.verb == "put" && line.words.first().is_some_and(rest));
        if found_name && !remade {
            place.remark(
                line.number,
                format!("\"{name}\" is a thing found on its own: its number may change at the next inventory; name it in pieces.txt (decor element {name})"),
            );
        }
        let tree = self.workshop.catalogue.get(&name).is_some_and(|piece| piece.kind == "tree");
        // (Living and moving: so much room around it, its corner as said.)
        let room = line.animation.as_ref().filter(|(kind, _)| crate::animate::moves(kind)).map_or(0, |_| crate::animate::MARGIN as i64);
        for (x, y) in points_of(&self.at(line), &line.words[1..])? {
            // (Every piece by its top left corner, a tree too; by its foot if said.)
            let (x, y) = if line.foot { (x - made.foot.0, y - made.foot.1) } else { (x - room, y - room) };
            if tree {
                self.tree(place, &made, x + made.foot.0, y + made.foot.1, line)?;
                continue;
            }

            self.check_overlap(place, line, &made, x, y, &name);
            self.lay_thing(place, &made, x, y, &name, line)?;
            let (w, h) = made.size();
            let (x, y, w, h) = (x + room, y + room, w as i64 - 2 * room, h as i64 - 2 * room);
            if x < 0 || y < 0 || x + w > self.decor.width as i64 || y + h > self.decor.height as i64 {
                place.remark(line.number, format!("\"{name}\" goes past the place at {x} {y}"));
            }
            // (Named: a spot of the films at its foot, as an entrance is.)
            if let Some(spot) = &line.name {
                place.decor.entrances.push((line.number, x + w / 2, y + h + TILE as i64, spot.clone()));
            }
        }
        Ok(())
    }

    /// One of our own pictures (`image <png> x y`), at each of its points.
    fn image(&mut self, place: &mut Place, line: &Line) -> Result<(), Diagnostic> {
        let file = line.words[0].clone();
        let (project, data) = (&self.workshop.project, &self.workshop.game.folder);
        let mut picture = own_picture(project, data, &file).ok_or_else(|| {
            let known = own_pictures(project, data);
            self.at(line).wrong(format!("no image \"{file}\" in {}.{}", project.images().display(), closest(&file, known.iter().map(String::as_str))))
        })?;
        if line.mirrored {
            picture = picture.mirrored();
        }
        // (Living: its frames, room kept around it if it moves.)
        let (pictures, room) = match &line.animation {
            Some((kind, pace)) => {
                let room = if crate::animate::moves(kind) { crate::animate::MARGIN } else { 0 };
                (crate::animate::animated(&[crate::animate::padded(&picture, room)], kind, crate::animate::frames(pace.as_deref())), room as i64)
            }
            None => (vec![picture.clone()], 0),
        };
        for (x, y) in points_of(&self.at(line), &line.words[1..])? {
            let owner = place.names.len() as i32;
            place.names.push(file.clone());
            place.lay_picture(&self.workshop.game, &pictures, x - room, y - room, owner, line.layer.as_deref())?;
            if let Some(spot) = &line.name {
                place.decor.entrances.push((line.number, x + picture.w as i64 / 2, y + picture.h as i64 + TILE as i64, spot.clone()));
            }
        }
        Ok(())
    }

    fn lay_thing(&self, place: &mut Place, made: &Cut, x: i64, y: i64, name: &str, line: &Line) -> Result<(), Diagnostic> {
        let owner = place.names.len() as i32;
        place.names.push(name.to_string());
        place.lay_cut((&self.workshop.game, &self.workshop.ramps), made, x, y, owner, line.layer.as_deref())
    }

    fn check_overlap(&self, place: &mut Place, line: &Line, made: &Cut, x: i64, y: i64, name: &str) {
        let (w, h) = (made.solid.w, made.solid.h);
        let Some((x0, y0, x1, y1)) = place.clip(x, y, w, h) else {
            return;
        };
        let seen = made.seen();
        let (rx, ry) = ((x0 as i64 - x) as usize, (y0 as i64 - y) as usize);
        let mut hit_owners: Vec<i32> = Vec::new();
        let (mut hits, mut under_trees, mut on_path) = (0, 0, 0);
        for py in y0..y1 {
            for px in x0..x1 {
                let (cx, cy) = (rx + px - x0, ry + py - y0);
                if !seen.get(cx, cy) {
                    continue;
                }
                let under = place.owner.get(px, py);
                let mine = made.lower[0].get(cx, cy);
                let there = place.lower[0].get(px, py);
                if under > 0 && mine[..3] != there[..3] {
                    hits += 1;
                    if !hit_owners.contains(&under) {
                        hit_owners.push(under);
                    }
                }
                if place.tree_walls.get(px / TILE, py / TILE) && under == 0 && !made.walked {
                    under_trees += 1;
                }
                if place.path.get(px, py) {
                    on_path += 1;
                }
            }
        }
        if hits > 4 {
            let mut others: Vec<String> = hit_owners.iter().map(|owner| place.names[*owner as usize].clone()).collect();
            others.sort();
            others.dedup();
            // (Water meeting water, two stretches of a river: no remark.)
            let catalogue = &self.workshop.catalogue;
            if catalogue.get(name).is_some_and(|piece| piece.kind == "water") {
                others.retain(|other| catalogue.get(other).is_none_or(|piece| piece.kind != "water"));
            }
            // (Over the rest of a place of the game: where it is made to be.)
            others.retain(|other| catalogue.get(other).is_none_or(|piece| !piece.has("rest")));
            if !others.is_empty() {
                place.remark(line.number, format!("\"{name}\" at {x} {y} overlaps {}", others.join(", ")));
            }
        }
        if under_trees > 32 {
            place.remark(line.number, format!("\"{name}\" at {x} {y} is laid under a tree"));
        }
        if on_path > 8 && !made.walked {
            place.remark(line.number, format!("\"{name}\" at {x} {y} is on a path"));
        }
    }

    /// A tree, its foot at fx, fy: its shadow on the ground, it on the upper layer.
    fn tree(&self, place: &mut Place, made: &Cut, fx: i64, fy: i64, line: &Line) -> Result<(), Diagnostic> {
        let before = place.walls.clone();
        place.lay_cut((&self.workshop.game, &self.workshop.ramps), made, fx - made.foot.0, fy - made.foot.1, 0, line.layer.as_deref())?;
        place.tree_walls = place.tree_walls.or(&place.walls.minus(&before));
        place.trees.push((line.number, fx, fy));
        Ok(())
    }

    fn ring(&mut self, place: &mut Place, line: &Line) -> Result<(), Diagnostic> {
        let name = line.words[0].clone();
        let made = self.cut(line, &name, &["tree"])?;
        let piece = self.piece(line, &name, &["tree"])?;
        let (w, h) = (self.decor.width as f64, self.decor.height as f64);
        let (mut cx, mut cy, mut rx, mut ry) = (w / 2.0, h / 2.0, w / 2.0 - 64.0, h / 2.0 - 64.0);
        let rest = &line.words[1..];
        if !rest.is_empty() {
            if rest[0] != "oval" || rest.len() != 5 {
                return Err(self.at(line).wrong(said("ring")));
            }
            let oval = numbers(&self.at(line), &rest[1..], Some(4))?;
            (cx, cy, rx, ry) = (oval[0] as f64, oval[1] as f64, oval[2] as f64, oval[3] as f64);
        }
        let grid: Vec<i64> = piece.option("grid").unwrap_or("64,32,0,0").split(',').filter_map(|word| word.parse().ok()).collect();
        let [step_x, step_y, phase_x, phase_y] = grid.as_slice() else {
            return Err(piece.where_().wrong("grid=step_x,step_y,x,y"));
        };
        let crown_seen: Vec<(usize, usize)> =
            (0..made.upper[0].cells.len()).filter(|at| made.upper[0].cells[*at][3] > 0).map(|at| (at % made.upper[0].w, at / made.upper[0].w)).collect();
        let (xs, ys): (Vec<i64>, Vec<i64>) = crown_seen.iter().map(|(x, y)| (*x as i64, *y as i64)).unzip();
        let (fx0, fy0) = made.foot;
        let crown = (
            xs.iter().min().copied().unwrap_or(0) - fx0,
            ys.iter().min().copied().unwrap_or(0) - fy0,
            xs.iter().max().copied().unwrap_or(0) - fx0,
            ys.iter().max().copied().unwrap_or(0) - fy0,
        );
        let near_path = grown(&place.path, 2, 2);
        let (wi, hi) = (self.decor.width as i64, self.decor.height as i64);
        let mut found = Vec::new();
        for j in -4..hi / step_y + 5 {
            for i in -4..wi / step_x + 5 {
                if (i + j).rem_euclid(2) != 0 {
                    continue;
                }
                let (fx, fy) = (phase_x + step_x * i, phase_y + step_y * j);
                if ((fx as f64 - cx) / rx).abs().powf(3.0) + ((fy as f64 - cy) / ry).abs().powf(3.0) < 1.0 {
                    continue;
                }
                if fx + crown.2 < 0 || fx + crown.0 > wi || fy + crown.1 > hi || fy + crown.3 < -8 {
                    continue;
                }
                let crown_box = place.clip(fx + crown.0, fy + crown.1, (crown.2 - crown.0 + 1) as usize, (crown.3 - crown.1 + 1) as usize);
                if let Some((x0, y0, x1, y1)) = crown_box
                    && (y0..y1).any(|y| (x0..x1).any(|x| near_path.get(x, y)))
                {
                    continue;
                }
                found.push((fx, fy));
            }
        }
        found.sort_by_key(|(x, y)| (*y, *x));
        for (fx, fy) in found {
            self.tree(place, &made, fx, fy, line)?;
        }
        Ok(())
    }

    fn strew(&mut self, place: &mut Place, line: &Line) -> Result<(), Diagnostic> {
        let name = line.words[0].clone();
        let made = self.cut(line, &name, &["object"])?;
        let at = self.at(line);
        let count = numbers(&at, &line.words[1..line.words.len().min(2)], Some(1))?[0];
        let (width, height) = (self.decor.width as i64, self.decor.height as i64);
        let zone = if line.words.len() > 2 { zone_of(&at, &line.words[2..], said("scatter"))? } else { (0, 0, width, height) };
        let mut rng = self.rng(line);
        let (w, h) = made.size();
        let (w, h) = (w as i64, h as i64);
        let tile = TILE as i64;
        let (pw, ph) = (self.decor.width, self.decor.height);
        let mut free = Mask {
            w: pw,
            h: ph,
            cells: (0..pw * ph)
                .map(|at| {
                    let (x, y) = (at % pw, at / pw);
                    !place.walls.get(x / TILE, y / TILE) && !place.path.get(x, y) && place.owner.get(x, y) == 0 && place.upper[0].get(x, y)[3] == 0
                })
                .collect(),
        };
        let (mut laid, mut tries): (Vec<(i64, i64)>, i64) = (Vec::new(), 0);
        while (laid.len() as i64) < count && tries < 400 * count {
            tries += 1;
            let x = rng.integers(zone.0.div_euclid(tile), (zone.0.div_euclid(tile) + 1).max((zone.2 - w).div_euclid(tile) + 1)) * tile;
            let y = rng.integers(zone.1.div_euclid(tile), (zone.1.div_euclid(tile) + 1).max((zone.3 - h).div_euclid(tile) + 1)) * tile;
            if x < 0 || y < 0 || x + w > width || y + h > height {
                continue;
            }
            let (left, top) = ((x - 4).max(0) as usize, (y - 4).max(0) as usize);
            let (right, bottom) = ((x + w + 4).min(width) as usize, (y + h + 4).min(height) as usize);
            if !(top..bottom).all(|py| (left..right).all(|px| free.get(px, py))) {
                continue;
            }
            if laid.iter().any(|(ox, oy)| (x - ox).abs() < 24 && (y - oy).abs() < 24) {
                continue;
            }
            laid.push((x, y));
            self.lay_thing(place, &made, x, y, &name, line)?;
            for py in y as usize..(y + h) as usize {
                for px in x as usize..(x + w) as usize {
                    free.set(px, py, false);
                }
            }
        }
        if (laid.len() as i64) < count {
            place.remark(line.number, format!("\"{name}\": {} laid of {count}, no free room left", laid.len()));
        }
        Ok(())
    }
}

/// Where our own pictures are: those the films draw too, the project's
/// (then the shared library's), then the data's `hand/`.
pub fn own_folders(project: &assets::project::Project, data: &str) -> Vec<std::path::PathBuf> {
    let mut folders = project.folders("images");
    folders.push(std::path::Path::new(data).join("hand"));
    folders
}

/// Our own pictures, by their files' names.
pub fn own_pictures(project: &assets::project::Project, data: &str) -> Vec<String> {
    let mut found: Vec<String> = own_folders(project, data)
        .iter()
        .flat_map(|folder| std::fs::read_dir(folder).into_iter().flatten().flatten())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".png"))
        .collect();
    found.sort();
    found.dedup();
    found
}

/// One of our own pictures (`rowboat.png`; `bouee_eau.2.png`, two side by
/// side: its first), as it is.
pub fn own_picture(project: &assets::project::Project, data: &str, file: &str) -> Option<crate::pixels::Picture> {
    let path = own_folders(project, data).into_iter().map(|folder| folder.join(file)).find(|path| path.is_file())?;
    let read = image::open(path).ok()?.to_rgba8();
    let cells: u32 = file.strip_suffix(".png")?.rsplit_once('.').and_then(|(_, cells)| cells.parse().ok()).unwrap_or(1).max(1);
    let (w, h) = ((read.width() / cells) as usize, read.height() as usize);
    Some(crate::pixels::Grid { w, h, cells: (0..w * h).map(|at| read.get_pixel((at % w) as u32, (at / w) as u32).0).collect() })
}

/// The tiles of a zone (pixels), its edge ragged: a tile on its border is in
/// it one time in two, so two grounds meet without a straight line.
/// The tiles a ground is painted on (`tiles c,r c,r ...`: column, row).
pub(crate) fn painted_tiles(at: &crate::fault::Location, words: &[String], rows: usize, columns: usize) -> Result<Grid<i64>, Diagnostic> {
    let mut wanted = Grid { w: columns, h: rows, cells: vec![-1; rows * columns] };
    for word in words {
        let tile = word.split_once(',').and_then(|(column, row)| Some((column.parse::<i64>().ok()?, row.parse::<i64>().ok()?)));
        let Some((column, row)) = tile else {
            return Err(at.wrong(format!("\"{word}\": a tile is column,row (tiles 3,4 5,4)")));
        };
        // (A tile out of the place: none of it.)
        if (0..columns as i64).contains(&column) && (0..rows as i64).contains(&row) {
            wanted.cells[row as usize * columns + column as usize] = 0;
        }
    }
    Ok(wanted)
}

/// A rectangle's tiles walled (`wall`) or let walk on (`free`), whatever was
/// laid there; or those tiles (`wall tiles 3,4 5,4`, painted in the Workshop).
fn walled(place: &mut Place, line: &Line, at: &crate::fault::Location) -> Result<(), Diagnostic> {
    if line.words.first().is_some_and(|word| word == "tiles") {
        let tiles = painted_tiles(at, &line.words[1..], place.walls.h, place.walls.w)?;
        for row in 0..tiles.h {
            for column in 0..tiles.w {
                if tiles.get(column, row) == 0 {
                    place.walls.set(column, row, line.verb == "wall");
                }
            }
        }
        return Ok(());
    }
    let said = crate::lang::numbers(at, &line.words, Some(4))?;
    let tile = TILE as i64;
    let (x0, y0, x1, y1) = (said[0].min(said[2]), said[1].min(said[3]), said[0].max(said[2]), said[1].max(said[3]));
    for row in y0.div_euclid(tile).max(0)..y1.div_euclid(tile).min(place.walls.h as i64) {
        for column in x0.div_euclid(tile).max(0)..x1.div_euclid(tile).min(place.walls.w as i64) {
            place.walls.set(column as usize, row as usize, line.verb == "wall");
        }
    }
    Ok(())
}

pub(crate) fn ragged_zone(rows: usize, columns: usize, (x0, y0, x1, y1): (i64, i64, i64, i64), rng: &mut Rng) -> Grid<i64> {
    let tile = TILE as i64;
    let (x0, y0, x1, y1) = (x0.div_euclid(tile), y0.div_euclid(tile), x1.div_euclid(tile), y1.div_euclid(tile));
    let mut inside = Mask::new(columns, rows);
    for row in y0.max(0)..y1.clamp(0, rows as i64) {
        for column in x0.max(0)..x1.clamp(0, columns as i64) {
            inside.set(column as usize, row as usize, true);
        }
    }
    let edge = grown(&inside, 1, 1).minus(&inside).or(&inside.and(&grown(&inside.not(), 1, 1)));
    let drawn = rng.randoms(rows * columns);
    Grid {
        w: columns,
        h: rows,
        cells: (0..rows * columns).map(|at| if if edge.cells[at] { drawn[at] < 0.5 } else { inside.cells[at] } { 0 } else { -1 }).collect(),
    }
}
