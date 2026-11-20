//! Weaving grounds, paths and bands from the game's own tiles.
//!
//! A ground (`ground`) and a band (`edge`) are woven tile by tile, each tile the
//! one of the source whose edges best meet what is round it, going on from
//! its neighbour as in the game where it can. A path (`path`) is woven by
//! its shape: each tile of the source knows what of it is inside the path and
//! what is its fringe; each tile of the path wanted takes the source tile of
//! the most alike shape, then of the best edges.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::catalogue::{Catalogue, Level, Piece};
use crate::fault::Diagnostic;
use crate::matte::{Ramps, Shade, Tile, matte};
use crate::pixels::{Grid, Mask, Rgba, TILE, border, colour, component, counted, grown, has, key, key_of, light, mean, unique};
use crate::random::Rng;
use crate::source::{Game, Source, tile_at};

/// How much a seam costs against going on as in the source.
pub const GOING_ON: f64 = 40.0;
/// What a pixel of a path's shape costs, wrong inside or wrong in its fringe.
const SHAPE_INSIDE: f64 = 40.0;
const SHAPE_FRINGE: f64 = 4.0;
/// A tile seen so often in a place may be one of its grounds; with none named
/// there, those seen most until this share of the place.
const PLACE_GROUND: usize = 8;
const MOST_OF_PLACE: f64 = 0.6;
/// How wide the fringe of a path is, in pixels.
pub const FRINGE: usize = 3;
/// The ways a tile of a path is turned (`flippable`).
const TURNS: [&str; 8] = ["", "h", "v", "hv", "t", "th", "tv", "thv"];
/// The pixels of a tile.
const CELLS: usize = TILE * TILE;

/// The small things of a place's grounds: the colours each shows, where.
pub type Crumbs = Rc<Vec<(Grid<[u8; 3]>, Mask)>>;

/// A place half laid: a mark of how it began, a mark of each line laid, itself.
pub(crate) type HalfLaid = (u64, Vec<u64>, Rc<crate::build::Place>);

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
    pub(crate) cuts: RefCell<HashMap<String, Rc<crate::cut::Cut>>>,
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
    pub(crate) tilesets: RefCell<HashMap<i64, Rc<crate::terrain::Tileset>>>,
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
}

/// A tile of a loom: its frames (alpha 0 where its ground is cut out), the
/// palette colours of each frame, its shade, where it is in its source (x, y,
/// turn), its row in a band, and for a path what of it is inside and fringe.
#[derive(Clone, Debug)]
pub struct LoomTile {
    pub frames: Vec<Vec<Rgba>>,
    pub indices: Option<Vec<Vec<u8>>>,
    pub place: (i64, i64, String),
    pub mark: i64,
    pub shade: Vec<f32>,
    pub shade_colour: Vec<[u8; 3]>,
    pub inside: Vec<bool>,
    pub fringe: Vec<bool>,
}

impl LoomTile {
    /// Its colours, frame 0, row by row.
    pub fn rgb(&self) -> Tile {
        self.frames[0].iter().flat_map(|rgba| [rgba[0], rgba[1], rgba[2]]).collect()
    }
}

/// The tiles a ground, a path or a band is woven from.
pub struct Loom {
    pub piece: Piece,
    pub tiles: Vec<LoomTile>,
    pub by_place: HashMap<(i64, i64, String), usize>,
    pub seconds: f64,
    pub grounds: Vec<i64>,
}

impl Loom {
    /// Whether all its tiles are whole and shade nothing.
    pub fn opaque(&self) -> bool {
        self.tiles.iter().all(|tile| tile.frames[0].iter().all(|rgba| rgba[3] > 0))
            && !self.tiles.iter().any(|tile| tile.shade.iter().any(|factor| *factor != 0.0))
    }

    pub fn frames(&self) -> usize {
        self.tiles.first().map_or(1, |tile| tile.frames.len())
    }

    pub fn tile_shade(&self, at: usize) -> Shade {
        let tile = &self.tiles[at];
        Shade {
            factor: Grid { w: TILE, h: TILE, cells: tile.shade.clone() },
            colour: Grid { w: TILE, h: TILE, cells: tile.shade_colour.clone() },
            grounds: self.grounds.clone(),
        }
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

/// The colours a piece's option says (`rrggbb,...`).
pub fn colours_said(text: &str) -> Vec<i64> {
    text.split(',').filter(|word| !word.is_empty()).filter_map(colour).map(key).collect()
}

/// The colours of a rectangle of a place's picture, as keys.
pub(crate) fn rect_keys(source: &Source, (x0, y0, x1, y1): (i64, i64, i64, i64)) -> Vec<i64> {
    let mut out = Vec::new();
    for y in y0.max(0)..y1.min(source.picture().h as i64) {
        for x in x0.max(0)..x1.min(source.picture().w as i64) {
            out.push(key(source.rgb(x as usize, y as usize)));
        }
    }
    out
}

/// Those of a rectangle's colours seen in at least one part out of `of`.
pub(crate) fn common(every: Vec<i64>, of: usize) -> Vec<i64> {
    let total = every.len();
    counted(every).into_iter().filter(|(_, count)| count * of >= total).map(|(colour, _)| colour).collect()
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

/// The colours of the shadows the game draws on a ground (`shadow=`).
pub(crate) fn shadow_colours(piece: &Piece) -> Vec<i64> {
    colours_said(piece.option("shadow").unwrap_or_default())
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

/// The grounds a piece stands on: their tiles, colours (none: it names
/// none) and shadow colours.
pub type Grounds = (Vec<Tile>, Option<Vec<i64>>, Vec<i64>);

/// A path's shape, tile by tile: what of each is inside it, what its fringe.
pub type Shape = (Grid<Vec<bool>>, Grid<Vec<bool>>);

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

/// A tile's cells (8 by 8) turned as `turn` says: across (t), left to right
/// (h), upside down (v), in that order.
pub fn turned<T: Copy>(cells: &[T], turn: &str) -> Vec<T> {
    let mut out = cells.to_vec();
    let at = |x: usize, y: usize| y * TILE + x;
    if turn.contains('t') {
        out = (0..CELLS).map(|cell| out[at(cell / TILE, cell % TILE)]).collect();
    }
    if turn.contains('h') {
        out = (0..CELLS).map(|cell| out[at(TILE - 1 - cell % TILE, cell / TILE)]).collect();
    }
    if turn.contains('v') {
        out = (0..CELLS).map(|cell| out[at(cell % TILE, TILE - 1 - cell / TILE)]).collect();
    }
    out
}

/// The tiles of a sol, edge or path_line piece.
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

/// A line through points, its turns rounded (cut at a quarter, again and again).
pub fn rounded(points: &[(i64, i64)]) -> Vec<(f64, f64)> {
    let mut points: Vec<(f64, f64)> = points.iter().map(|(x, y)| (*x as f64, *y as f64)).collect();
    for _ in 0..3 {
        if points.len() < 3 {
            break;
        }
        let mut out = vec![points[0]];
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            out.push((a.0 * 0.75 + b.0 * 0.25, a.1 * 0.75 + b.1 * 0.25));
            out.push((a.0 * 0.25 + b.0 * 0.75, a.1 * 0.25 + b.1 * 0.75));
        }
        out.push(points[points.len() - 1]);
        // (The ends stay where they are said: the first and last cut go.)
        let mut kept = vec![out[0]];
        kept.extend_from_slice(&out[2..out.len() - 2]);
        kept.push(out[out.len() - 1]);
        points = kept;
    }
    points
}

/// Which pixels of a place are inside a path along these points.
pub fn path_shape(points: &[(f64, f64)], width: i64, w: usize, h: usize) -> Mask {
    let mut inside = Mask::new(w, h);
    let reach = (width as f64 / 2.0) * (width as f64 / 2.0);
    for pair in points.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let length = (dx * dx + dy * dy).max(1e-9);
        for y in 0..h {
            for x in 0..w {
                let (px, py) = (x as f64, y as f64);
                let t = (((px - ax) * dx + (py - ay) * dy) / length).clamp(0.0, 1.0);
                let (ex, ey) = (px - ax - t * dx, py - ay - t * dy);
                if ex * ex + ey * ey <= reach {
                    inside.set(x, y, true);
                }
            }
        }
    }
    inside
}

/// A mask of a place as its tiles: each one's 64 cells, row by row.
pub fn as_tiles(mask: &Mask) -> Grid<Vec<bool>> {
    let (columns, rows) = (mask.w / TILE, mask.h / TILE);
    let mut out: Grid<Vec<bool>> = Grid { w: columns, h: rows, cells: vec![Vec::new(); columns * rows] };
    for row in 0..rows {
        for column in 0..columns {
            out.cells[row * columns + column] = (0..CELLS).map(|cell| mask.get(column * TILE + cell % TILE, row * TILE + cell / TILE)).collect();
        }
    }
    out
}

/// The colours of a picture, as numbers (frame 0 of a place, as woven over).
type Canvas = Grid<[i32; 3]>;

/// What a tile looks like over a ground, roughly: where it is cut out, the
/// ground, shaded as it says.
fn laid_over(tile: &LoomTile, under: &[[i32; 3]]) -> Vec<[i32; 3]> {
    (0..CELLS)
        .map(|at| {
            let rgba = tile.frames[0][at];
            if rgba[3] > 0 {
                return [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32];
            }
            let factor = tile.shade[at];
            let factor = if factor > 0.0 { factor as f64 } else { 1.0 };
            under[at].map(|part| (part as f64 * factor) as i32)
        })
        .collect()
}

fn block(picture: &Canvas, x: usize, y: usize) -> Vec<[i32; 3]> {
    (0..CELLS).map(|at| picture.get(x + at % TILE, y + at / TILE)).collect()
}

fn difference(a: [i32; 3], b: [i32; 3]) -> i64 {
    (0..3).map(|part| (a[part] - b[part]).abs() as i64).sum()
}

/// The choice of each tile of a weaving (-1: none), and what each cost.
pub type Woven = (Grid<i64>, Grid<f64>);

/// Chooses a tile of the loom for each tile of `wanted` (-1 none, else the
/// mark it must have): the one whose edges best meet what is round it
/// already (`canvas`, frame 0 of the place) and going on from its neighbours
/// as in the game; for a path, `shape` (inside, fringe) says each tile's shape.
pub fn weave(loom: &Loom, wanted: &Grid<i64>, canvas: &Grid<Rgba>, rng: &mut Rng, shape: Option<&Shape>) -> Woven {
    let (rows, columns) = (wanted.h, wanted.w);
    let mut chosen: Grid<i64> = Grid::filled(columns, rows, -1);
    let mut costs: Grid<f64> = Grid::new(columns, rows);
    let tiles: Vec<Vec<[i32; 3]>> =
        loom.tiles.iter().map(|tile| tile.frames[0].iter().map(|rgba| [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32]).collect()).collect();
    let clear = loom.opaque();
    let mut by_mark: Vec<(i64, Vec<usize>)> = Vec::new();
    for (at, tile) in loom.tiles.iter().enumerate() {
        match by_mark.iter_mut().find(|(mark, _)| *mark == tile.mark) {
            Some((_, found)) => found.push(at),
            None => by_mark.push((tile.mark, vec![at])),
        }
    }
    let everything: Vec<usize> = (0..tiles.len()).collect();
    let mut picture: Canvas = canvas.map(|rgba| [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32]);
    for row in 0..rows {
        for column in 0..columns {
            let mark = wanted.get(column, row);
            if mark < 0 {
                continue;
            }
            let candidates: &Vec<usize> = match shape {
                Some(_) => &everything,
                None => match by_mark.iter().find(|(known, _)| *known == mark) {
                    Some((_, found)) => found,
                    // (No tile of that kind in the source: the nearest kind.)
                    None => &by_mark.iter().min_by_key(|(other, _)| ((other ^ mark).count_ones(), *other)).expect("a kind").1,
                },
            };
            let (y, x) = (row * TILE, column * TILE);
            let under = block(&picture, x, y);
            let seen: Vec<Vec<[i32; 3]>> = candidates.iter().map(|at| if clear { tiles[*at].clone() } else { laid_over(&loom.tiles[*at], &under) }).collect();
            let mut sums = vec![0i64; candidates.len()];
            let mut sides = 0;
            // (Left, above: laid by now; right, below: only what is there and not to be woven.)
            let edge = |sums: &mut Vec<i64>, cells: &dyn Fn(usize) -> usize, there: &dyn Fn(usize) -> [i32; 3]| {
                for (slot, one) in sums.iter_mut().zip(&seen) {
                    *slot += (0..TILE).map(|at| difference(one[cells(at)], there(at))).sum::<i64>();
                }
            };
            if column > 0 && (chosen.get(column - 1, row) >= 0 || wanted.get(column - 1, row) < 0) {
                edge(&mut sums, &|at| at * TILE, &|at| picture.get(x - 1, y + at));
                sides += 1;
            }
            if row > 0 && (chosen.get(column, row - 1) >= 0 || wanted.get(column, row - 1) < 0) {
                edge(&mut sums, &|at| at, &|at| picture.get(x + at, y - 1));
                sides += 1;
            }
            if column + 1 < columns && wanted.get(column + 1, row) < 0 {
                edge(&mut sums, &|at| at * TILE + TILE - 1, &|at| picture.get(x + TILE, y + at));
                sides += 1;
            }
            if row + 1 < rows && wanted.get(column, row + 1) < 0 {
                edge(&mut sums, &|at| (TILE - 1) * TILE + at, &|at| picture.get(x + at, y + TILE));
                sides += 1;
            }
            let mut cost: Vec<f64> = sums.iter().map(|sum| *sum as f64 / sides.max(1) as f64 / TILE as f64).collect();
            let raw = cost.clone();
            if let Some((inside, fringe)) = shape {
                let (inside, fringe) = (&inside.cells[row * columns + column], &fringe.cells[row * columns + column]);
                for (slot, at) in cost.iter_mut().zip(candidates) {
                    let tile = &loom.tiles[*at];
                    *slot += SHAPE_INSIDE * (0..CELLS).filter(|cell| tile.inside[*cell] != inside[*cell]).count() as f64;
                    *slot += SHAPE_FRINGE * (0..CELLS).filter(|cell| tile.fringe[*cell] != fringe[*cell]).count() as f64;
                }
            }
            for (dr, dc, dx, dy) in [(0i64, -1i64, TILE as i64, 0i64), (-1, 0, 0, TILE as i64)] {
                let (r, c) = (row as i64 + dr, column as i64 + dc);
                if r < 0 || c < 0 || chosen.get(c as usize, r as usize) < 0 {
                    continue;
                }
                let (px, py, turn) = &loom.tiles[chosen.get(c as usize, r as usize) as usize].place;
                if turn.contains('t') {
                    continue;
                }
                let next = (if turn.contains('h') { px - dx } else { px + dx }, if turn.contains('v') { py - dy } else { py + dy }, turn.clone());
                if let Some(following) = loom.by_place.get(&next) {
                    for (slot, at) in cost.iter_mut().zip(candidates) {
                        if at == following {
                            *slot -= GOING_ON;
                        }
                    }
                }
            }
            for (slot, drawn) in cost.iter_mut().zip(rng.randoms(candidates.len())) {
                *slot += drawn * 0.5;
            }
            let best = (0..cost.len()).fold(0, |best, at| if cost[at] < cost[best] { at } else { best });
            chosen.set(column, row, candidates[best] as i64);
            costs.set(column, row, raw[best]);
            for (at, rgb) in seen[best].iter().enumerate() {
                picture.set(x + at % TILE, y + at / TILE, *rgb);
            }
        }
    }
    (chosen, costs)
}

/// Where a band lies, in tiles: its first row, how many rows, how many
/// columns; whether it has a ground above it, below it, to meet.
#[derive(Clone, Copy, Debug)]
pub struct Band {
    pub top: usize,
    pub rows: usize,
    pub columns: usize,
    pub above: bool,
    pub below: bool,
}

/// A band: whole columns of the source side by side, each the one whose left
/// edge best meets the column before it, going on as in the game where it
/// can, and whose edges best meet the bands above and below it.
pub fn weave_columns(loom: &Loom, band: Band, canvas: &Grid<Rgba>, rng: &mut Rng) -> Result<Woven, Diagnostic> {
    let Band { top, rows, columns, above, below } = band;
    let mut by_column: Vec<(i64, Vec<(i64, usize)>)> = Vec::new();
    for (at, tile) in loom.tiles.iter().enumerate() {
        let x = tile.place.0;
        let marks = match by_column.iter_mut().find(|(known, _)| *known == x) {
            Some((_, marks)) => marks,
            None => {
                by_column.push((x, Vec::new()));
                &mut by_column.last_mut().expect("just put").1
            }
        };
        match marks.iter_mut().find(|(mark, _)| *mark == tile.mark) {
            Some((_, kept)) => *kept = at,
            None => marks.push((tile.mark, at)),
        }
    }
    let mut xs: Vec<i64> = by_column.iter().filter(|(_, marks)| marks.len() == rows).map(|(x, _)| *x).collect();
    xs.sort_unstable();
    if xs.is_empty() {
        return Err(loom.piece.where_().wrong(format!("\"{}\" has no whole column", loom.piece.name)));
    }
    let stacks: Vec<Vec<usize>> = xs
        .iter()
        .map(|x| {
            let marks = &by_column.iter().find(|(known, _)| known == x).expect("a column").1;
            (0..rows).map(|row| marks.iter().find(|(mark, _)| *mark == row as i64).map_or(0, |(_, at)| *at)).collect()
        })
        .collect();
    let rgb = |at: usize, cell: usize| -> [i32; 3] {
        let rgba = loom.tiles[at].frames[0][cell];
        [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32]
    };
    let picture: Canvas = canvas.map(|rgba| [rgba[0] as i32, rgba[1] as i32, rgba[2] as i32]);
    let height = canvas.h / TILE;
    let mut chosen: Grid<i64> = Grid::filled(columns, height, -1);
    let mut costs: Grid<f64> = Grid::new(columns, height);
    let mut previous: Option<usize> = None;
    for column in 0..columns {
        let x = column * TILE;
        let mut cost = vec![0.0f64; xs.len()];
        for (slot, stack) in cost.iter_mut().zip(&stacks) {
            if above {
                let sum: i64 = (0..TILE).map(|at| difference(rgb(stack[0], at), picture.get(x + at, top * TILE - 1))).sum();
                *slot += sum as f64 / TILE as f64;
            }
            if below {
                let sum: i64 = (0..TILE).map(|at| difference(rgb(stack[rows - 1], (TILE - 1) * TILE + at), picture.get(x + at, (top + rows) * TILE))).sum();
                *slot += sum as f64 / TILE as f64;
            }
        }
        let mut raw = vec![0.0f64; xs.len()];
        if let Some(before) = previous {
            for (at, stack) in stacks.iter().enumerate() {
                let sum: i64 = (0..rows)
                    .flat_map(|row| (0..TILE).map(move |y| (row, y)))
                    .map(|(row, y)| difference(rgb(stack[row], y * TILE), rgb(stacks[before][row], y * TILE + TILE - 1)))
                    .sum();
                raw[at] = sum as f64 / (rows * TILE) as f64;
                cost[at] += raw[at];
            }
            if let Some(next) = xs.iter().position(|x| *x == xs[before] + TILE as i64) {
                cost[next] -= GOING_ON;
            }
        }
        for (slot, drawn) in cost.iter_mut().zip(rng.randoms(xs.len())) {
            *slot += drawn * 0.5;
        }
        let best = (0..cost.len()).fold(0, |best, at| if cost[at] < cost[best] { at } else { best });
        for (row, stacked) in stacks[best].iter().take(rows).enumerate() {
            chosen.set(column, top + row, *stacked as i64);
            costs.set(column, top + row, raw[best]);
        }
        previous = Some(best);
    }
    Ok((chosen, costs))
}
