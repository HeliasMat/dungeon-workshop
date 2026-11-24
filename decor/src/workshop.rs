//! What a place is made with (`Workshop`): the game's files, the catalogue,
//! and what is worked out of them once and kept: the grounds each place
//! shows (`seen_in`), a ground's colours and tiles (`grounds_of`), its loom
//! (`Workshop::loom`), the shadows drawn on each colour (the ramps), the
//! tilesets, the pieces cut and grounds woven, the place laid last.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::catalogue::{Catalogue, Level, Piece};
use crate::fault::Diagnostic;
use crate::matte::{Ramps, Shade, Tile, matte};
use crate::pixels::{Grid, Mask, Rgba, TILE, border, component, counted, grown, has, key, key_of, light, mean, unique};
use crate::source::{Game, Source, tile_at};
use crate::terrain::{Tileset, learned};
use crate::weave::{CELLS, Crumbs, FRINGE, Grounds, Loom, LoomTile, Woven, colours_said, common, rect_keys, shadow_colours, turned};

/// A tile seen so often in a place may be one of its grounds; with none named
/// there, those seen most until this share of the place.
const PLACE_GROUND: usize = 8;

const MOST_OF_PLACE: f64 = 0.6;

/// The ways a tile of a path is turned (`flippable`).
const TURNS: [&str; 8] = ["", "h", "v", "hv", "t", "th", "tv", "thv"];

/// A place half laid: a mark of how it began, a mark of each line laid, itself.
pub(crate) type HalfLaid = (u64, Vec<u64>, Rc<crate::place::Place>);

/// What a place is made with: the game's files, the catalogue, and what is
/// worked out of them once.
pub struct Workshop {
    pub game: Game,
    /// The project whose places are built: its pictures, its decors.
    pub project: assets::project::Project,
    pub catalogue: Catalogue,
    pub ramps: Ramps,
    looms: RefCell<HashMap<String, Rc<Loom>>>,
    seen_in: RefCell<HashMap<i64, Rc<Vec<Piece>>>>,
    pub(crate) crumbs: RefCell<HashMap<i64, Crumbs>>,
    /// Each piece cut (a place's rest, hollowed), kept from one build to the
    /// next: a decor changed is built again of the pieces already cut.
    pub(crate) cuts: RefCell<HashMap<String, Rc<crate::piece::Cut>>>,
    /// Each ground woven, by all it was woven of (its ground, where, what was
    /// under it, the decor's chance and its line): woven once, the longest of
    /// a build (a forest over a whole large decor takes seconds).
    pub(crate) woven: RefCell<HashMap<u64, Rc<Woven>>>,
    /// A place as it was half laid, the last time one was built: what it was
    /// laid of (a mark of how it began, a mark of each line laid), and
    /// itself. The next build that begins the same is laid on from there:
    /// one line changed of a large decor, only that one and those after it.
    pub(crate) half_laid: RefCell<Option<HalfLaid>>,
    /// The marks of the lines of the place built last, to tell which changed.
    pub(crate) laid_lines: RefCell<Vec<u64>>,
    /// Each place's tiles learned to paint terrains with.
    pub(crate) tilesets: RefCell<HashMap<i64, Rc<Tileset>>>,
}

impl Workshop {
    pub fn new(game: Game, catalogue: Catalogue, project: assets::project::Project) -> Result<Workshop, Diagnostic> {
        let mut workshop = Workshop {
            game,
            project,
            catalogue,
            ramps: Ramps::default(),
            looms: Default::default(),
            seen_in: Default::default(),
            crumbs: Default::default(),
            cuts: Default::default(),
            woven: Default::default(),
            half_laid: Default::default(),
            laid_lines: Default::default(),
            tilesets: Default::default(),
        };
        workshop.ramps = read_ramps(&workshop)?;
        Ok(workshop)
    }

    pub fn source(&self, level: i64) -> Result<Rc<Source>, Diagnostic> {
        self.game.source(level)
    }

    /// What was worked out of the places read, forgotten with them.
    pub fn forget(&self) {
        self.game.forget();
        self.looms.borrow_mut().clear();
        self.crumbs.borrow_mut().clear();
        self.cuts.borrow_mut().clear();
        self.woven.borrow_mut().clear();
        *self.half_laid.borrow_mut() = None;
        self.laid_lines.borrow_mut().clear();
        self.tilesets.borrow_mut().clear();
    }

    /// A piece's loom, made once.
    pub fn loom(&self, piece: &Piece) -> Result<Rc<Loom>, Diagnostic> {
        if let Some(known) = self.looms.borrow().get(&piece.name) {
            return Ok(known.clone());
        }
        let made = Rc::new(loom_of(self, piece)?);
        self.looms.borrow_mut().insert(piece.name.clone(), made.clone());
        Ok(made)
    }

    /// A place's tileset, learned once.
    pub fn tileset(&self, level: i64) -> Result<Rc<Tileset>, Diagnostic> {
        if let Some(known) = self.tilesets.borrow().get(&level) {
            return Ok(known.clone());
        }
        let made = Rc::new(learned(&*self.source(level)?));
        self.tilesets.borrow_mut().insert(level, made.clone());
        Ok(made)
    }
}

fn rects_of(piece: &Piece) -> Vec<(i64, i64, i64, i64)> {
    let mut rects = vec![piece.rect];
    for part in piece.option("also").unwrap_or_default().split(';').filter(|part| !part.is_empty()) {
        let numbers: Vec<i64> = part.split(',').filter_map(|word| word.parse().ok()).collect();
        if let [x0, y0, x1, y1] = numbers.as_slice() {
            rects.push((*x0, *y0, *x1, *y1));
        }
    }
    rects
}

/// The colours of a ground's tiles, and the common ones of its `tints`.
pub fn ground_colours(workshop: &Workshop, piece: &Piece) -> Result<Vec<i64>, Diagnostic> {
    let loom = workshop.loom(piece)?;
    let mut found: Vec<i64> = loom.tiles.iter().flat_map(|tile| tile.frames[0].iter().map(|rgba| key_of(*rgba)).collect::<Vec<_>>()).collect();
    let tints: Vec<(i64, i64, i64, i64)> = piece
        .option("tints")
        .unwrap_or_default()
        .split(';')
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let numbers: Vec<i64> = part.split(',').filter_map(|word| word.parse().ok()).collect();
            match numbers.as_slice() {
                [x0, y0, x1, y1] => Some((*x0, *y0, *x1, *y1)),
                _ => None,
            }
        })
        .collect();
    if !tints.is_empty()
        && let Some(level) = piece.place()
    {
        let source = workshop.source(level)?;
        let every: Vec<i64> = tints.iter().flat_map(|rect| rect_keys(&source, *rect)).collect();
        found.extend(common(every, 200));
    }
    Ok(unique(found))
}

/// The grounds the catalogue names that a place has: named there, or whose
/// tiles it shows often (those named by hand only).
pub fn seen_in(workshop: &Workshop, level: i64) -> Result<Rc<Vec<Piece>>, Diagnostic> {
    if let Some(known) = workshop.seen_in.borrow().get(&level) {
        return Ok(known.clone());
    }
    let source = workshop.source(level)?;
    let counts = source.tile_counts();
    let mut found = Vec::new();
    for ground in &workshop.catalogue.pieces {
        let inventory = ground.name.strip_prefix('s').is_some_and(|rest| {
            rest.split_once('_')
                .is_some_and(|(a, b)| !a.is_empty() && !b.is_empty() && a.chars().all(|c| c.is_ascii_digit()) && b.chars().all(|c| c.is_ascii_digit()))
        });
        let Some(own_level) = ground.place().filter(|_| ["ground", "edge", "path"].contains(&ground.kind.as_str()) && !inventory) else {
            continue;
        };
        if own_level == level {
            found.push(ground.clone());
            continue;
        }
        let (x0, y0, x1, y1) = ground.rect;
        let picture = workshop.source(own_level)?;
        let mut own: Vec<Tile> = Vec::new();
        let mut y = y0;
        while y + TILE as i64 <= y1 {
            let mut x = x0;
            while x + TILE as i64 <= x1 {
                let tile = tile_at(picture.picture(), x as usize, y as usize);
                if !own.contains(&tile) {
                    own.push(tile);
                }
                x += TILE as i64;
            }
            y += TILE as i64;
        }
        let seen: usize = own.iter().map(|tile| counts.get(tile).unwrap_or(0)).sum();
        if seen >= PLACE_GROUND * own.len().max(1) {
            found.push(ground.clone());
        }
    }
    let found = Rc::new(found);
    workshop.seen_in.borrow_mut().insert(level, found.clone());
    Ok(found)
}

/// The colours of a ground the catalogue names; of a path, those of its
/// inside (`inside=`), else those common in its rectangle.
pub(crate) fn named_colours(workshop: &Workshop, piece: &Piece) -> Result<Vec<i64>, Diagnostic> {
    if piece.kind != "path" {
        return ground_colours(workshop, piece);
    }
    let said = piece.option("inside").unwrap_or_default();
    if !said.is_empty() && !said.starts_with('@') {
        return Ok(colours_said(said));
    }
    let source = workshop.source(piece.place().unwrap_or_default())?;
    Ok(common(rect_keys(&source, piece.rect), 200))
}

/// The shadow colours the game draws on each colour of the catalogue's
/// grounds that say them: the first that says one, for each colour.
fn read_ramps(workshop: &Workshop) -> Result<Ramps, Diagnostic> {
    let mut ramps = Ramps::default();
    for piece in &workshop.catalogue.pieces {
        if !["ground", "edge"].contains(&piece.kind.as_str()) || !piece.has("shadow") {
            continue;
        }
        let shades: Vec<[u8; 3]> = shadow_colours(piece).into_iter().map(|key| [(key >> 16) as u8, (key >> 8 & 255) as u8, (key & 255) as u8]).collect();
        let loom = workshop.loom(piece)?;
        let lights: Vec<f64> =
            loom.tiles.iter().flat_map(|tile| tile.frames[0].iter().map(|rgba| light([rgba[0], rgba[1], rgba[2]])).collect::<Vec<_>>()).collect();
        let whole = mean(&lights).max(1.0);
        let ratios: Vec<f64> = shades.iter().map(|rgb| light(*rgb) / whole).collect();
        for colour in ground_colours(workshop, piece)? {
            ramps.0.entry(colour).or_insert_with(|| (shades.clone(), ratios.clone()));
        }
    }
    Ok(ramps)
}

/// The grounds a piece or a path is cut out of (`on=a+b`): their tiles,
/// colours and shadow colours; colours none when it names none.
pub fn grounds_of(workshop: &Workshop, piece: &Piece) -> Result<Grounds, Diagnostic> {
    let (mut tiles, mut palette, mut shadows): (Vec<Tile>, Vec<Vec<i64>>, Vec<i64>) = (Vec::new(), Vec::new(), Vec::new());
    for name in piece.option("on").unwrap_or_default().split('+').filter(|name| !name.is_empty()) {
        // (`on=place`: the grounds of its own place, named there, and the tiles
        // seen there often that are of their colours only; with none named,
        // the tiles seen most, as much as most of the place.)
        if let Some(level) = piece.place().filter(|_| name == "place") {
            let source = workshop.source(level)?;
            let named = seen_in(workshop, level)?;
            let counts = source.tile_counts();
            let mut seen_often: Vec<(usize, &Tile)> = counts.order.iter().filter(|(_, seen)| *seen >= PLACE_GROUND).map(|(tile, seen)| (*seen, tile)).collect();
            seen_often.sort_by_key(|(seen, _)| std::cmp::Reverse(*seen));
            let often: Vec<Tile> = if named.is_empty() {
                let whole = counts.total();
                let (mut taken, mut often) = (0usize, Vec::new());
                for (seen, tile) in seen_often {
                    if taken as f64 >= MOST_OF_PLACE * whole as f64 {
                        break;
                    }
                    often.push(tile.clone());
                    taken += seen;
                }
                often
            } else {
                let mut allowed = Vec::new();
                for ground in named.iter() {
                    allowed.extend(named_colours(workshop, ground)?);
                }
                for ground in named.iter() {
                    allowed.extend(shadow_colours(ground));
                }
                let allowed = unique(allowed);
                seen_often
                    .into_iter()
                    .filter(|(_, tile)| tile.chunks(3).all(|rgb| has(&allowed, key([rgb[0], rgb[1], rgb[2]]))))
                    .map(|(_, tile)| tile.clone())
                    .collect()
            };
            palette.push(unique(often.iter().flat_map(|tile| tile.chunks(3).map(|rgb| key([rgb[0], rgb[1], rgb[2]])).collect::<Vec<_>>())));
            tiles.extend(often);
            for ground in named.iter() {
                if ground.kind != "path" {
                    tiles.extend(workshop.loom(ground)?.tiles.iter().map(LoomTile::rgb));
                }
                palette.push(named_colours(workshop, ground)?);
                shadows.extend(shadow_colours(ground));
            }
            continue;
        }
        let Some(ground) = workshop.catalogue.get(name).filter(|ground| ["ground", "edge"].contains(&ground.kind.as_str())) else {
            return Err(piece.where_().wrong(format!("on=\"{name}\" is neither a ground nor an edge of the catalogue")));
        };
        tiles.extend(workshop.loom(ground)?.tiles.iter().map(LoomTile::rgb));
        palette.push(ground_colours(workshop, ground)?);
        shadows.extend(shadow_colours(ground));
    }
    if palette.is_empty() {
        return Ok((tiles, None, Vec::new()));
    }
    Ok((tiles, Some(unique(palette.into_iter().flatten())), shadows))
}

/// The colours of the inside of a path (`inside=`: colours, or `@x0,y0,x1,y1`:
/// the common colours of a rectangle inside it).
fn inside_colours(piece: &Piece, source: &Source) -> Result<Vec<i64>, Diagnostic> {
    let said = piece.option("inside").unwrap_or_default();
    let inside = match said.strip_prefix('@') {
        Some(rect) => {
            let numbers: Vec<i64> = rect.split(',').filter_map(|word| word.parse().ok()).collect();
            match numbers.as_slice() {
                [x0, y0, x1, y1] => common(rect_keys(source, (*x0, *y0, *x1, *y1)), 100),
                _ => Vec::new(),
            }
        }
        None => colours_said(said),
    };
    if inside.is_empty() {
        return Err(piece.where_().wrong("a path says its colours: inside=rrggbb,..."));
    }
    Ok(inside)
}

/// The tiles of a ground, edge or path_line piece.
fn loom_of(workshop: &Workshop, piece: &Piece) -> Result<Loom, Diagnostic> {
    let Level::Place(level) = piece.level else {
        return Err(piece.where_().wrong(format!("\"{}\": a ground comes from a place of the game", piece.name)));
    };
    let source = workshop.source(level)?;
    let tile = TILE as i64;
    let mut tiles: Vec<LoomTile> = Vec::new();
    let mut grounds: Vec<i64> = Vec::new();
    for (x0, y0, x1, y1) in rects_of(piece) {
        if x0 % tile != 0 || y0 % tile != 0 || x1 % tile != 0 || y1 % tile != 0 {
            return Err(piece.where_().wrong(format!("\"{}\": a rectangle on the grid of 8", piece.name)));
        }
        let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let (ox, oy) = (x0 as usize, y0 as usize);
        let mut frames: Vec<Grid<Rgba>> = source.merged.iter().map(|frame| frame.crop(ox, oy, ox + w, oy + h)).collect();
        let mut shade = Shade::none(w, h);
        let mut inside = Mask::new(w, h);
        if piece.kind == "path" {
            let colours = unique(inside_colours(piece, &source)?);
            inside = Grid { w, h, cells: (0..w * h).map(|at| has(&colours, key(source.rgb(ox + at % w, oy + at / w)))).collect() };
            // (A stone, a darker spot in the path is still the path.)
            let starts: Vec<(usize, usize)> = border(w, h).into_iter().filter(|(x, y)| !inside.get(*x, *y)).collect();
            inside = inside.or(&component(&inside.not(), &starts).not());
            if piece.has("on") {
                let (ground, palette, shadows) = grounds_of(workshop, piece)?;
                let palette = palette.unwrap_or_default();
                let (is_path, made) = matte(&source, (x0, y0, x1, y1), &ground, &palette, &shadows, false);
                shade = made;
                let near = grown(&inside, FRINGE, 1);
                for at in 0..w * h {
                    if !((is_path.cells[at] && near.cells[at]) || inside.cells[at]) {
                        for frame in &mut frames {
                            frame.cells[at][3] = 0;
                        }
                    }
                    if !near.cells[at] || inside.cells[at] {
                        shade.factor.cells[at] = 0.0;
                    }
                }
                grounds = palette;
            }
        }
        let fringe = grown(&inside, FRINGE, 1).minus(&inside);
        for ty in (0..h).step_by(TILE) {
            for tx in (0..w).step_by(TILE) {
                let cells = |grid_cell: &dyn Fn(usize, usize) -> usize| -> Vec<usize> {
                    (0..CELLS).map(|cell| grid_cell(tx + cell % TILE, ty + cell / TILE)).collect()
                };
                let spots = cells(&|x, y| y * w + x);
                let frames_here: Vec<Vec<Rgba>> = frames.iter().map(|frame| spots.iter().map(|at| frame.cells[*at]).collect()).collect();
                let indices = source
                    .merged_index
                    .as_ref()
                    .map(|all| all.iter().map(|index| (0..CELLS).map(|cell| index.get(ox + tx + cell % TILE, oy + ty + cell / TILE)).collect()).collect());
                tiles.push(LoomTile {
                    frames: frames_here,
                    indices,
                    place: (x0 + tx as i64, y0 + ty as i64, String::new()),
                    mark: if piece.kind == "edge" { (ty / TILE) as i64 } else { 0 },
                    shade: spots.iter().map(|at| shade.factor.cells[*at]).collect(),
                    shade_colour: spots.iter().map(|at| shade.colour.cells[*at]).collect(),
                    inside: spots.iter().map(|at| inside.cells[*at]).collect(),
                    fringe: spots.iter().map(|at| fringe.cells[*at]).collect(),
                });
            }
        }
    }
    // (A tile with a colour seldom seen in the rectangles holds a bit of
    // something else, a flower, a fence: not ground.)
    let rare: f64 = piece.option("rare").unwrap_or(if piece.kind != "edge" { "0.3" } else { "0" }).parse().unwrap_or(0.0);
    if rare > 0.0 {
        let every: Vec<i64> = tiles.iter().flat_map(|tile| tile.frames[0].iter().map(|rgba| key_of(*rgba)).collect::<Vec<_>>()).collect();
        let total = every.len() as f64;
        let seldom: Vec<i64> = counted(every).into_iter().filter(|(_, count)| ((*count * 100) as f64) < rare * total).map(|(colour, _)| colour).collect();
        tiles.retain(|tile| !tile.frames[0].iter().any(|rgba| has(&seldom, key_of(*rgba))));
    }
    // (Of a whole place, only its commonest tiles: its plain ground.)
    if let Some(many) = piece.option("frequent").and_then(|many| many.parse::<usize>().ok()) {
        let made: Vec<Tile> = tiles.iter().map(LoomTile::rgb).collect();
        let mut seen: Vec<(Tile, usize)> = Vec::new();
        for tile in &made {
            match seen.iter_mut().find(|(known, _)| known == tile) {
                Some((_, count)) => *count += 1,
                None => seen.push((tile.clone(), 1)),
            }
        }
        seen.sort_by(|(a, count_a), (b, count_b)| count_b.cmp(count_a).then(a.cmp(b)));
        let kept: Vec<Tile> = seen.into_iter().take(many).map(|(tile, _)| tile).collect();
        let mut at = 0;
        tiles.retain(|_| {
            at += 1;
            kept.contains(&made[at - 1])
        });
    }
    if tiles.is_empty() {
        return Err(piece.where_().wrong(format!("\"{}\" has no tile left", piece.name)));
    }
    if piece.kind == "path" && piece.has("flippable") {
        add_turns(&mut tiles);
    }
    let by_place = tiles.iter().enumerate().map(|(at, tile)| (tile.place.clone(), at)).collect();
    Ok(Loom { piece: piece.clone(), tiles, by_place, seconds: source.info.seconds as f64, grounds })
}

/// The tiles of a path turned every way, each once.
fn add_turns(tiles: &mut Vec<LoomTile>) {
    let bytes = |frame: &[Rgba]| -> Vec<u8> { frame.iter().flat_map(|rgba| *rgba).collect() };
    let mut known: Vec<Vec<u8>> = tiles.iter().map(|tile| bytes(&tile.frames[0])).collect();
    for at in 0..tiles.len() {
        let (tx, ty, _) = tiles[at].place.clone();
        for turn in &TURNS[1..] {
            let frames: Vec<Vec<Rgba>> = tiles[at].frames.iter().map(|frame| turned(frame, turn)).collect();
            let first = bytes(&frames[0]);
            if known.contains(&first) {
                continue;
            }
            known.push(first);
            let base = &tiles[at];
            let made = LoomTile {
                frames,
                indices: base.indices.as_ref().map(|all| all.iter().map(|index| turned(index, turn)).collect()),
                place: (tx, ty, turn.to_string()),
                mark: base.mark,
                shade: turned(&base.shade, turn),
                shade_colour: turned(&base.shade_colour, turn),
                inside: turned(&base.inside, turn),
                fringe: turned(&base.fringe, turn),
            };
            tiles.push(made);
        }
    }
}
