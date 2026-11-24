//! The pieces a place is made of, cut from the game's own places: things cut
//! out of their ground with their shadow (`matte`), the game's things of the
//! scenes, the tree of a wood. Grounds, paths and bands are woven (`weave`).

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::catalogue::{Level, Piece};
use crate::fault::Diagnostic;
use crate::matte::{Shade, matte, thing_shadow};
use crate::piece::Cut;
use crate::pixels::{Grid, Mask, Picture, TILE, border, colour, component, components, counted, grown, has, key, key_of, light, rgb_of, unique};
use crate::source::{Source, picture_at, tile_at};
use crate::weave::{Crumbs, colours_said};
use crate::workshop::{Workshop, grounds_of, seen_in};

/// How deep `peel=` peels, in pixels.
const PEEL_DEEP: usize = 6;
/// What is left of a small thing erased, as many pixels at most.
const DUST: usize = 12;

/// Only the piece of what is seen (either layer, any frame) that holds the
/// pixel `seed`, pieces a few pixels apart being one: the rest made clear.
/// A tile of a place seen so many times in it at least is a ground's (a
/// building's tiles, one of a kind, are seen once, twice, three times).
const GROUND_SEEN: usize = 5;

pub fn keep_element(
    lower: &mut [Picture],
    upper: &mut [Picture],
    shade: &mut Shade,
    seeds: &[(usize, usize)],
    lower_tiles: &[(usize, usize)],
    (place_upper, origin): (&[Picture], (usize, usize)),
    ground_tile: &dyn Fn(usize, usize) -> bool,
) {
    let (w, h) = (lower[0].w, lower[0].h);
    // (The colours of its ground: those of the tile round it, which is ground
    // by how it is found, where the place's tile is seen again elsewhere (a
    // building is one of a kind, a ground is not) and nothing of the upper
    // layer is over it; none of them kept of its lower layer.)
    let mut ground = std::collections::HashSet::new();
    for row in 0..h {
        for column in 0..w {
            let ring = row < TILE || column < TILE || row + TILE >= h || column + TILE >= w;
            let rgba = lower[0].cells[row * w + column];
            let (px, py) = (origin.0 + column, origin.1 + row);
            let covered = place_upper.iter().any(|frame| px < frame.w && py < frame.h && frame.get(px, py)[3] > 0);
            if ring && rgba[3] > 0 && !covered && !lower_tiles.contains(&(column / TILE, row / TILE)) && ground_tile(px / TILE, py / TILE) {
                ground.insert([rgba[0], rgba[1], rgba[2]]);
            }
        }
    }
    for frame in lower.iter_mut() {
        for rgba in frame.cells.iter_mut().filter(|rgba| ground.contains(&[rgba[0], rgba[1], rgba[2]])) {
            rgba[3] = 0;
        }
    }
    // (Of its lower layer, only its own tiles (`lower`): never its ground.)
    for row in 0..h {
        for column in 0..w {
            if !lower_tiles.contains(&(column / TILE, row / TILE)) {
                for frame in lower.iter_mut() {
                    frame.cells[row * w + column][3] = 0;
                }
                shade.factor.cells[row * w + column] = 0.0;
            }
        }
    }
    let mut seen = Mask::new(w, h);
    for frame in lower.iter().chain(upper.iter()) {
        for (cell, rgba) in seen.cells.iter_mut().zip(&frame.cells) {
            *cell |= rgba[3] > 0;
        }
    }
    let seed = seeds[0];
    if seed.0 >= w || seed.1 >= h || !seen.get(seed.0, seed.1) {
        return;
    }
    // (Of its upper layer, the object it is, found in the whole place as the
    // inventory finds it: those stuck to it, and any other, out.)
    let objects: Vec<Mask> = seeds.iter().filter_map(|seed| object_of(place_upper, (origin.0 + seed.0, origin.1 + seed.1))).collect();
    if let Some(object) = objects.into_iter().reduce(|all, one| all.or(&one)) {
        for y in 0..h {
            for x in 0..w {
                if object.get(origin.0 + x, origin.1 + y) {
                    continue;
                }
                for frame in upper.iter_mut() {
                    frame.cells[y * w + x][3] = 0;
                }
                seen.cells[y * w + x] = lower.iter().any(|frame| frame.cells[y * w + x][3] > 0);
            }
        }
    }
    let joined = crate::pixels::grown(&seen, 3, 1);
    let mine = crate::pixels::component(&joined, seeds);
    for at in (0..w * h).filter(|at| !mine.cells[*at]) {
        for frame in lower.iter_mut().chain(upper.iter_mut()) {
            frame.cells[at][3] = 0;
        }
        shade.factor.cells[at] = 0.0;
    }
}

/// The object of a place's upper layer (any of its frames) that holds a
/// pixel, as the inventory finds it: its piece (near pixels together),
/// those stuck to it apart (`pixels::objects`). None if the pixel shows
/// nothing there.
pub fn object_of(upper: &[Picture], (x, y): (usize, usize)) -> Option<Mask> {
    let first = upper.first()?;
    let (w, h) = (first.w, first.h);
    let mut seen = Mask::new(w, h);
    for frame in upper {
        for (cell, rgba) in seen.cells.iter_mut().zip(&frame.cells) {
            *cell |= rgba[3] > 0;
        }
    }
    if x >= w || y >= h || !seen.get(x, y) {
        return None;
    }
    let piece = component(&grown(&seen, 3, 1), &[(x, y)]).and(&seen);
    let colour = |x: usize, y: usize| {
        let shown = upper.iter().map(|frame| frame.get(x, y)).find(|rgba| rgba[3] > 0).unwrap_or_default();
        [shown[0], shown[1], shown[2]]
    };
    // (Only the one holding the pixel: none other made.)
    let (label, _) = crate::pixels::object_labels(&piece, &colour);
    let mine = label.get(x, y);
    (mine > 0).then(|| Mask { w, h, cells: label.cells.iter().map(|at| *at == mine).collect() })
}

/// `0,2;1,2`: tiles by column and row.
fn tiles_list(text: &str) -> Vec<(i64, i64)> {
    let mut found = Vec::new();
    for part in text.split(';').filter(|part| !part.is_empty()) {
        if let Some((column, row)) = part.split_once(',')
            && let (Ok(column), Ok(row)) = (column.parse(), row.parse())
            && !found.contains(&(column, row))
        {
            found.push((column, row));
        }
    }
    found
}

/// The colours of the ground round a rectangle: those most seen there; and
/// those most seen along its own edge, if near those.
fn ring_palette(source: &Source, (x0, y0, x1, y1): (i64, i64, i64, i64)) -> Vec<i64> {
    let picture = source.picture();
    let ring = 3 * TILE as i64;
    let (left, top) = ((x0 - ring).max(0), (y0 - ring).max(0));
    let (right, bottom) = ((x1 + ring).min(picture.w as i64), (y1 + ring).min(picture.h as i64));
    let mut round_it = Vec::new();
    for y in top..bottom {
        for x in left..right {
            if !(x0 <= x && x < x1 && y0 <= y && y < y1) {
                round_it.push(key(source.rgb(x as usize, y as usize)));
            }
        }
    }
    let total = round_it.len().max(1);
    let common: Vec<i64> = counted(round_it).into_iter().filter(|(_, count)| count * 16 >= total).map(|(colour, _)| colour).collect();
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let edge: Vec<i64> = border(w, h).into_iter().map(|(x, y)| key(source.rgb(x0 as usize + x, y0 as usize + y))).collect();
    let length = edge.len();
    let near: Vec<i64> = counted(edge)
        .into_iter()
        .filter(|(seen, count)| {
            let rgb = rgb_of(*seen);
            count * 10 >= length
                && !common.is_empty()
                && common.iter().map(|other| (0..3).map(|part| (rgb[part] as i64 - rgb_of(*other)[part] as i64).abs()).sum::<i64>()).min().unwrap_or(i64::MAX)
                    <= 24
        })
        .map(|(seen, _)| seen)
        .collect();
    unique(common.into_iter().chain(near))
}

/// The rectangles an option says (`x0,y0,x1,y1;...`, in the piece), as a mask.
fn rects_mask(piece: &Piece, option: &str, w: usize, h: usize) -> Result<Mask, Diagnostic> {
    let mut mask = Mask::new(w, h);
    for part in piece.option(option).unwrap_or_default().split(';').filter(|part| !part.is_empty()) {
        let numbers: Vec<i64> = part.split(',').filter_map(|word| word.parse().ok()).collect();
        let [x0, y0, x1, y1] = numbers.as_slice() else {
            return Err(piece.where_().wrong(format!("{option}=x0,y0,x1,y1;... in numbers (\"{part}\")")));
        };
        for y in (*y0).max(0) as usize..(*y1).clamp(0, h as i64) as usize {
            for x in (*x0).max(0) as usize..(*x1).clamp(0, w as i64) as usize {
                mask.set(x, y, true);
            }
        }
    }
    Ok(mask)
}

/// What is the thing itself in a rectangle that is its heart and a tile round
/// it (`alone`): the parts that reach the heart.
fn alone(thing: &Mask) -> Mask {
    let mut heart = Mask::new(thing.w, thing.h);
    for y in TILE..thing.h.saturating_sub(TILE) {
        for x in TILE..thing.w.saturating_sub(TILE) {
            heart.set(x, y, true);
        }
    }
    let mut kept = Mask::new(thing.w, thing.h);
    for part in components(thing) {
        if part.and(&heart).any() {
            kept = kept.or(&part);
        }
    }
    kept
}

/// What of a piece's rectangle is the thing, and its shade: cut from its
/// grounds, then as its options say.
fn cut_out(workshop: &Workshop, piece: &Piece, source: &Source) -> Result<(Mask, Shade), Diagnostic> {
    let (tiles, palette, mut shadows) = grounds_of(workshop, piece)?;
    if let Some(own) = piece.option("shadow").filter(|own| !own.is_empty()) {
        shadows = unique(shadows.into_iter().chain(colours_said(own)));
    }
    let palette = match palette {
        Some(palette) if piece.has("ground_only") => palette,
        Some(palette) => unique(palette.into_iter().chain(ring_palette(source, piece.rect))),
        None => ring_palette(source, piece.rect),
    };
    let (mut thing, mut shade) = matte(source, piece.rect, &tiles, &palette, &shadows, piece.has("whole"));
    let (w, h) = (thing.w, thing.h);
    if piece.option("keep").is_some_and(|said| !said.is_empty()) {
        thing = thing.or(&rects_mask(piece, "keep", w, h)?);
    }
    if piece.option("in").is_some_and(|said| !said.is_empty()) {
        let within = rects_mask(piece, "in", w, h)?;
        thing = thing.and(&within);
        for at in 0..w * h {
            if !within.cells[at] {
                shade.factor.cells[at] = 0.0;
            }
        }
    }
    if piece.has("alone") {
        thing = alone(&thing);
        let near = grown(&thing, 2, 1);
        for at in 0..w * h {
            if !near.cells[at] {
                shade.factor.cells[at] = 0.0;
            }
        }
    }
    if let Some(peeled) = piece.option("peel").filter(|said| !said.is_empty()) {
        let peeled = unique(colours_said(peeled));
        let (x0, y0, _, _) = piece.rect;
        let drawn = Grid { w, h, cells: (0..w * h).map(|at| has(&peeled, key(source.rgb(x0 as usize + at % w, y0 as usize + at / w)))).collect() };
        for _ in 0..PEEL_DEEP {
            let rim = thing.and(&grown(&component(&thing.not(), &border(w, h)), 1, 1)).and(&drawn);
            if !rim.any() {
                break;
            }
            thing = thing.minus(&rim);
        }
    }
    if piece.has("full") {
        thing = thing.or(&component(&thing.not(), &border(w, h)).not());
        for at in 0..w * h {
            if thing.cells[at] {
                shade.factor.cells[at] = 0.0;
            }
        }
    }
    Ok((thing, shade))
}

/// The small things of a place's grounds (`crumb`), each as the colours it
/// shows and where: named there or in a place whose grounds it shares.
fn crumbs_of(workshop: &Workshop, level: i64) -> Result<Crumbs, Diagnostic> {
    if let Some(known) = workshop.crumbs.borrow().get(&level) {
        return Ok(known.clone());
    }
    let mut levels = vec![level];
    for ground in seen_in(workshop, level)?.iter() {
        if let Some(other) = ground.place() {
            levels.push(other);
        }
    }
    let mut found = Vec::new();
    for crumb in &workshop.catalogue.pieces {
        let Some(own) = crumb.place().filter(|own| crumb.has("crumb") && levels.contains(own)) else {
            continue;
        };
        let made = cut(workshop, crumb)?;
        let source = workshop.source(own)?;
        let (x0, y0, x1, y1) = crumb.rect;
        let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let colours = Grid { w, h, cells: (0..w * h).map(|at| source.rgb(x0 as usize + at % w, y0 as usize + at / w)).collect() };
        found.push((colours, made.lower[0].map(|rgba| rgba[3] > 0)));
    }
    let found = Rc::new(found);
    workshop.crumbs.borrow_mut().insert(level, found.clone());
    Ok(found)
}

/// Where the small things of its ground are seen whole in a piece's rectangle.
fn crumbs_in(workshop: &Workshop, piece: &Piece, source: &Source) -> Result<Vec<Mask>, Diagnostic> {
    let (x0, y0, x1, y1) = piece.rect;
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let seen = |x: usize, y: usize| source.rgb(x0 as usize + x, y0 as usize + y);
    let mut found = Vec::new();
    for (colours, mask) in crumbs_of(workshop, piece.place().unwrap_or_default())?.iter() {
        let (tw, th) = (mask.w, mask.h);
        if th > h || tw > w || mask.count() * 4 > h * w {
            continue;
        }
        let Some((fx, fy)) = mask.first() else {
            continue;
        };
        let want = colours.get(fx, fy);
        for dy in 0..=h - th {
            for dx in 0..=w - tw {
                if seen(dx + fx, dy + fy) != want {
                    continue;
                }
                let whole = (0..tw * th).filter(|at| mask.cells[*at]).all(|at| seen(dx + at % tw, dy + at / tw) == colours.cells[at]);
                if whole {
                    let mut at = Mask::new(w, h);
                    for cell in (0..tw * th).filter(|cell| mask.cells[*cell]) {
                        at.set(dx + cell % tw, dy + cell / tw, true);
                    }
                    found.push(at);
                }
            }
        }
    }
    Ok(found)
}

/// The small things of its ground drawn the same each time, leaning on a
/// piece: wherever one is seen whole, it goes, and the dust of its shadow.
fn erase_crumbs(workshop: &Workshop, piece: &Piece, source: &Source, lower: &mut [Picture], upper: &mut [Picture]) -> Result<(), Diagnostic> {
    let crumbs = crumbs_in(workshop, piece, source)?;
    for crumb in &crumbs {
        clear(lower, upper, crumb);
    }
    if !crumbs.is_empty() {
        let (w, h) = (lower[0].w, lower[0].h);
        let shown = Grid { w, h, cells: (0..w * h).map(|at| lower[0].cells[at][3] > 0 || upper[0].cells[at][3] > 0).collect() };
        for part in components(&shown) {
            if part.count() <= DUST {
                clear(lower, upper, &part);
            }
        }
    }
    Ok(())
}

/// Nothing seen of those pixels, on either layer, any frame.
fn clear(lower: &mut [Picture], upper: &mut [Picture], mask: &Mask) {
    for frame in lower.iter_mut().chain(upper.iter_mut()) {
        for (rgba, gone) in frame.cells.iter_mut().zip(&mask.cells) {
            if *gone {
                rgba[3] = 0;
            }
        }
    }
}

/// Whether one walks over a piece.
pub fn walks(piece: &Piece) -> bool {
    piece.has("walk") || piece.option("wall") == Some("none")
}

fn foot_of(piece: &Piece, w: usize, h: usize) -> (i64, i64) {
    if let Some((x, y)) = piece.option("foot").and_then(|said| said.split_once(',')).and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?))) {
        return (x, y);
    }
    ((w / 2) as i64, h as i64 - 4)
}

/// Which pixels of a piece are walls: the game's grid (by default where the
/// place has one), all it shows, or none.
fn walls_of(piece: &Piece, source: &Source, lower: &[Picture], upper: &[Picture]) -> Result<Mask, Diagnostic> {
    let (x0, y0, _, _) = piece.rect;
    let (w, h) = (lower[0].w, lower[0].h);
    let mut seen = Mask::new(w, h);
    for frame in lower.iter().chain(upper) {
        for (cell, rgba) in seen.cells.iter_mut().zip(&frame.cells) {
            *cell |= rgba[3] > 0;
        }
    }
    let by_default = if source.walls.is_some() && piece.kind == "object" { "game" } else { "full" };
    let walls = if walks(piece) { "none" } else { piece.option("wall").unwrap_or(by_default) };
    match walls {
        "game" => {
            let Some(grid) = &source.walls else {
                return Err(piece.where_().wrong(format!("wall=game, but level {} has no walls", source.level)));
            };
            if x0 % TILE as i64 != 0 || y0 % TILE as i64 != 0 {
                return Err(piece.where_().wrong("wall=game asks for a rectangle on the grid of 8"));
            }
            let (column, row) = ((x0 as usize) / TILE, (y0 as usize) / TILE);
            Ok(Grid {
                w,
                h,
                cells: (0..w * h)
                    .map(|at| {
                        seen.cells[at]
                            && grid.cells.get((row + at / w / TILE) * grid.w + column + at % w / TILE).copied().unwrap_or(false)
                            && column + at % w / TILE < grid.w
                    })
                    .collect(),
            })
        }
        "full" => Ok(seen),
        "none" => Ok(Mask::new(w, h)),
        other => Err(piece.where_().wrong(format!("wall=\"{other}\": game, full or none"))),
    }
}

/// The piece, cut from its place as the catalogue says.
pub fn cut(workshop: &Workshop, piece: &Piece) -> Result<Cut, Diagnostic> {
    if let Some(maker) = piece.option("extract") {
        return match maker {
            "arbre_bois" => wood_tree(workshop, piece),
            other => Err(piece.where_().wrong(format!("unknown extract \"{other}\" (arbre_bois)"))),
        };
    }
    let level = match piece.level {
        Level::Thing(thing) => return thing_cut(workshop, piece, thing),
        Level::Place(level) => level,
    };
    let source = workshop.source(level)?;
    let (x0, y0, x1, y1) = piece.rect;
    let (width, height) = (source.frames[0].w as i64, source.frames[0].h as i64);
    if !(0 <= x0 && x0 < x1 && x1 <= width && 0 <= y0 && y0 < y1 && y1 <= height) {
        return Err(piece.where_().wrong(format!("\"{}\" goes out of its place ({width}x{height})", piece.name)));
    }
    let (ox, oy, ex, ey) = (x0 as usize, y0 as usize, x1 as usize, y1 as usize);
    let (w, h) = (ex - ox, ey - oy);
    let takes_upper = piece.has("upper");
    let crop = |frames: &Vec<Picture>| -> Vec<Picture> { frames.iter().map(|frame| frame.crop(ox, oy, ex, ey)).collect() };
    let crop_index = |frames: &Option<Vec<Grid<u8>>>| frames.as_ref().map(|all| all.iter().map(|frame| frame.crop(ox, oy, ex, ey)).collect::<Vec<_>>());
    let mut lower = crop(if takes_upper { &source.frames } else { &source.merged });
    let mut upper = if takes_upper { crop(&source.upper) } else { vec![Picture::new(w, h)] };
    let lower_index = crop_index(if takes_upper { &source.indices } else { &source.merged_index });
    let upper_index = if takes_upper { crop_index(&source.upper_indices) } else { None };
    let mut shade = Shade::none(w, h);
    // (Tiles of the lower layer, those only: nothing of the upper one.)
    if let Some(said) = piece.option("tiles") {
        let tiles: Vec<(usize, usize)> =
            said.split(';').filter_map(|tile| tile.split_once(',').and_then(|(column, row)| Some((column.parse().ok()?, row.parse().ok()?)))).collect();
        for at in 0..w * h {
            let kept = tiles.contains(&((at % w) / TILE, (at / w) / TILE));
            for frame in lower.iter_mut().filter(|_| !kept) {
                frame.cells[at][3] = 0;
            }
            for frame in upper.iter_mut() {
                frame.cells[at][3] = 0;
            }
        }
    }
    if !piece.has("base") && !piece.has("over") && ["object", "water", "tree"].contains(&piece.kind.as_str()) {
        let (is_thing, made) = cut_out(workshop, piece, &source)?;
        shade = made;
        clear(&mut lower, &mut upper, &is_thing.not());
        if !piece.has("crumb") && !piece.has("whole") {
            erase_crumbs(workshop, piece, &source, &mut lower, &mut upper)?;
        }
    }
    // (An element whole: that piece of what is seen only, its neighbours left out.)
    let seeds: Vec<(usize, usize)> = piece
        .option("element")
        .unwrap_or_default()
        .split(';')
        .filter_map(|seed| seed.split_once(',').and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?))))
        .collect();
    if !seeds.is_empty() {
        let lower_tiles: Vec<(usize, usize)> = piece
            .option("lower")
            .unwrap_or_default()
            .split(';')
            .filter_map(|tile| tile.split_once(',').and_then(|(column, row)| Some((column.parse().ok()?, row.parse().ok()?))))
            .collect();
        // (A tile of the place seen again elsewhere in it: of a ground.)
        let counts = source.tile_counts();
        let picture = source.picture();
        let ground_tile = |column: usize, row: usize| {
            (column + 1) * TILE <= picture.w
                && (row + 1) * TILE <= picture.h
                && counts.get(&tile_at(picture, column * TILE, row * TILE)).unwrap_or(0) >= GROUND_SEEN
        };
        keep_element(&mut lower, &mut upper, &mut shade, &seeds, &lower_tiles, (&source.upper, (ox, oy)), &ground_tile);
    }
    let left_out = tiles_list(piece.option("without").unwrap_or_default());
    let mut top_only: Vec<(i64, i64)> = Vec::new();
    if let Some(rows) = piece.option("upper").filter(|rows| !rows.is_empty()) {
        let (first, last) = rows.split_once('-').unwrap_or((rows, ""));
        let first: i64 = first.parse().unwrap_or(0);
        let last: i64 = if last.is_empty() { first } else { last.parse().unwrap_or(first) };
        for row in first..=last {
            for column in 0..=(w / TILE) as i64 {
                top_only.push((column, row));
            }
        }
    }
    let mut blanked: Vec<(i64, i64)> = left_out.clone();
    for tile in top_only {
        if !blanked.contains(&tile) {
            blanked.push(tile);
        }
    }
    for (column, row) in blanked {
        let whole = left_out.contains(&(column, row));
        for y in (row * TILE as i64).max(0)..((row + 1) * TILE as i64).min(h as i64) {
            for x in (column * TILE as i64).max(0)..((column + 1) * TILE as i64).min(w as i64) {
                let at = y as usize * w + x as usize;
                for frame in lower.iter_mut() {
                    frame.cells[at][3] = 0;
                }
                shade.factor.cells[at] = 0.0;
                if whole {
                    for frame in upper.iter_mut() {
                        frame.cells[at][3] = 0;
                    }
                }
            }
        }
    }
    // (On both layers: a colour changed, from the row `replace_lower=` says down.)
    let from_row: usize = piece.option("replace_lower").and_then(|row| row.parse().ok()).unwrap_or(0);
    for pair in piece.option("replace").unwrap_or_default().split(',').filter(|pair| !pair.is_empty()) {
        let (old, new) = pair.split_once('>').unwrap_or((pair, ""));
        let Some(old) = colour(old) else {
            continue;
        };
        for frame in lower.iter_mut().chain(upper.iter_mut()) {
            for y in from_row..frame.h {
                for x in 0..frame.w {
                    let rgba = frame.at(x, y);
                    if key_of(*rgba) == key(old) {
                        match colour(new) {
                            Some(made) if new != "-" => (rgba[0], rgba[1], rgba[2]) = (made[0], made[1], made[2]),
                            _ => rgba[3] = 0,
                        }
                    }
                }
            }
        }
    }
    let solid = walls_of(piece, &source, &lower, &upper)?;
    let made = Cut {
        lower,
        upper,
        solid,
        level,
        lower_index,
        upper_index,
        shade,
        foot: foot_of(piece, w, h),
        walked: walks(piece),
        seconds: source.info.seconds as f64,
    };
    Ok(if piece.option("flip").unwrap_or_default().contains('h') { made.mirrored() } else { made })
}

/// A piece of one of the game's things: its first picture, facing down; a
/// wall where it is seen, unless walked on; its shadow (`shadow=c1,c2@row`).
fn thing_cut(workshop: &Workshop, piece: &Piece, thing: u32) -> Result<Cut, Diagnostic> {
    let picture = picture_at(&assets::data::path(&workshop.game.folder, &format!("props/{thing}_0.png")))
        .ok_or_else(|| piece.where_().wrong(format!("no image of object {thing}")))?;
    let (x0, y0, x1, y1) = piece.rect;
    if !(0 <= x0 && x0 < x1 && x1 <= picture.w as i64 && 0 <= y0 && y0 < y1 && y1 <= picture.h as i64) {
        return Err(piece.where_().wrong(format!("\"{}\" goes out of its image ({}x{})", piece.name, picture.w, picture.h)));
    }
    let mut part = picture.crop(x0 as usize, y0 as usize, x1 as usize, y1 as usize);
    let (w, h) = (part.w, part.h);
    let mut shadow = Mask::new(w, h);
    let said = piece.option("shadow").unwrap_or_default();
    let (colours, row) = said.split_once('@').unwrap_or((said, ""));
    let colours = colours_said(colours);
    if let Some(first) = colours.first() {
        let row: usize = row.parse().unwrap_or(0);
        let others = unique(colours[1..].iter().copied());
        let opaque = |at: usize| part.cells[at][3] > 0;
        let below = |at: usize| opaque(at) && at / w >= row;
        let keyed = |at: usize| key_of(part.cells[at]);
        for at in 0..w * h {
            shadow.cells[at] = below(at) && keyed(at) == *first;
        }
        let other: Vec<bool> = (0..w * h).map(|at| below(at) && has(&others, keyed(at))).collect();
        let thing_pixels: Vec<bool> = (0..w * h).map(|at| opaque(at) && !shadow.cells[at] && !other[at]).collect();
        let touches = |at: usize| {
            let (x, y) = (at % w, at / w);
            (y > 0 && thing_pixels[at - w]) || (y + 1 < h && thing_pixels[at + w]) || (x > 0 && thing_pixels[at - 1]) || (x + 1 < w && thing_pixels[at + 1])
        };
        for (at, (seen, shadowed)) in other.iter().zip(&mut shadow.cells).enumerate() {
            if *seen && !touches(at) {
                *shadowed = true;
            }
        }
        for at in 0..w * h {
            if shadow.cells[at] {
                part.cells[at][3] = 0;
            }
        }
    }
    let walked = walks(piece);
    let solid = if walked { Mask::new(w, h) } else { part.map(|rgba| rgba[3] > 0) };
    Ok(Cut {
        lower: vec![part],
        upper: vec![Picture::new(w, h)],
        solid,
        level: -1,
        lower_index: None,
        upper_index: None,
        shade: thing_shadow(&shadow),
        foot: foot_of(piece, w, h),
        walked,
        seconds: 0.0,
    })
}

// --- The tree of the beach path (225) -------------------------------------
// Its wood is one tree on a grid of 64 by 32 pixels, each in front of others:
// the tree is what is tree on every one of them, in the colours it shows where
// nothing stands in front of it; its shadow, that of the tree of the mirrored
// wood with none in front of it.
const FOREST: i64 = 225;
const CROWN_PATCH: (i64, i64, i64, i64) = (238, 228, 24, 16);
const ANCHOR: (i64, i64) = (242, 264);
const GRID: (i64, i64, i64, i64) = (50, 360, 64, 32);
const BOX: (i64, i64, i64, i64) = (-56, -64, 52, 40);
const SHADE_BOX: (i64, i64, i64, i64) = (-48, -24, 56, 32);
const SHADE_OF: (i64, i64) = (370, 136);
const TREE_COLOURS: [[u8; 3]; 13] = [
    [71, 135, 31],
    [55, 103, 31],
    [47, 87, 31],
    [87, 167, 31],
    [111, 223, 31],
    [63, 103, 31],
    [215, 255, 151],
    [103, 183, 31],
    [95, 167, 39],
    [103, 223, 31],
    [143, 79, 31],
    [103, 63, 31],
    [199, 111, 31],
];
const TREE_SHADOW: [[u8; 3]; 4] = [[135, 119, 95], [135, 111, 95], [175, 183, 47], [135, 143, 39]];
const GRASS: [[u8; 3]; 3] = [[207, 223, 87], [191, 215, 71], [223, 231, 111]];
const CROWN_MIDDLE: i64 = 6;
const CROWN_FOOT: i64 = -3;

/// The wood's picture, and the same turned over.
fn wood(workshop: &Workshop) -> Result<[Grid<[u8; 3]>; 2], Diagnostic> {
    let source = workshop.source(FOREST)?;
    let picture = source.picture().map(|rgba| [rgba[0], rgba[1], rgba[2]]);
    let mirrored = picture.mirrored();
    Ok([picture, mirrored])
}

/// Where the trees of a picture of the wood stand: those whose crown is the
/// wood's own, on its grid.
fn trees_in(picture: &Grid<[u8; 3]>, wood: &Grid<[u8; 3]>) -> Vec<(i64, i64)> {
    let (x0, y0, w, h) = CROWN_PATCH;
    let mut found = Vec::new();
    for j in -20..20i64 {
        for i in -2..9i64 {
            if (i + j).rem_euclid(2) != 0 {
                continue;
            }
            let (x, y) = (GRID.0 + GRID.2 * i, GRID.1 + GRID.3 * j);
            let (px, py) = (x + x0 - ANCHOR.0, y + y0 - ANCHOR.1);
            if px < 0 || py < 0 || px + w > picture.w as i64 || py + h > picture.h as i64 {
                continue;
            }
            let same =
                (0..h).all(|dy| (0..w).all(|dx| picture.get((px + dx) as usize, (py + dy) as usize) == wood.get((x0 + dx) as usize, (y0 + dy) as usize)));
            if same {
                found.push((x, y));
            }
        }
    }
    found
}

/// Where the tree is on every tree of the wood, filled.
fn tree_mask(pictures: [&Grid<[u8; 3]>; 2], wood: &Grid<[u8; 3]>) -> Mask {
    let (x0, y0, x1, y1) = BOX;
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let tree = unique(TREE_COLOURS.iter().map(|rgb| key(*rgb)));
    let mut possible = Mask::filled(w, h, true);
    for picture in pictures {
        for (x, y) in trees_in(picture, wood) {
            for dy in y0..y1 {
                for dx in x0..x1 {
                    let (px, py) = (x + dx, y + dy);
                    if px >= 0 && py >= 0 && px < picture.w as i64 && py < picture.h as i64 && !has(&tree, key(picture.get(px as usize, py as usize))) {
                        possible.set((dx - x0) as usize, (dy - y0) as usize, false);
                    }
                }
            }
        }
    }
    let mask = component(&possible, &[((-x0) as usize, (-40 - y0) as usize)]);
    let mut padded = Mask::filled(w + 2, h + 2, true);
    for y in 0..h {
        for x in 0..w {
            padded.set(x + 1, y + 1, !mask.get(x, y));
        }
    }
    let outside = component(&padded, &[(0, 0)]).crop(1, 1, w + 1, h + 1);
    outside.not()
}

/// The tree, in the colours it shows where nothing stands in front of it.
fn wood_picture(workshop: &Workshop) -> Result<Picture, Diagnostic> {
    let (x0, y0, x1, y1) = BOX;
    let [picture, mirrored] = wood(workshop)?;
    let mask = tree_mask([&picture, &mirrored], &picture);
    let (w, h) = (mask.w, mask.h);
    let mut votes: BTreeMap<(usize, usize), Vec<[u8; 3]>> = BTreeMap::new();
    for one in [&picture, &mirrored] {
        let anchors = trees_in(one, &picture);
        for &(x, y) in &anchors {
            let fronts: Vec<(i64, i64)> = anchors.iter().copied().filter(|(fx, fy)| *fy > y && (fx - x).abs() < 130 && fy - y < 110).collect();
            for at in (0..w * h).filter(|at| mask.cells[*at]) {
                let (dx, dy) = ((at % w) as i64, (at / w) as i64);
                let (px, py) = (x + dx + x0, y + dy + y0);
                if px < 0 || py < 0 || px >= one.w as i64 || py >= one.h as i64 {
                    continue;
                }
                let hidden = fronts.iter().any(|(fx, fy)| {
                    let (mx, my) = (px - fx - x0, py - fy - y0);
                    (0..x1 - x0).contains(&mx) && (0..y1 - y0).contains(&my) && mask.get(mx as usize, my as usize)
                });
                if !hidden {
                    votes.entry((dy as usize, dx as usize)).or_default().push(one.get(px as usize, py as usize));
                }
            }
        }
    }
    let mut out = Picture::new(w, h);
    for ((dy, dx), seen) in &votes {
        // (The colour seen most; of those as often, the first in order.)
        let mut kinds: Vec<[u8; 3]> = seen.clone();
        kinds.sort_unstable();
        kinds.dedup();
        let count = |colour: &[u8; 3]| seen.iter().filter(|one| *one == colour).count();
        let best = kinds.iter().fold(kinds[0], |best, colour| if count(colour) > count(&best) { *colour } else { best });
        out.set(*dx, *dy, [best[0], best[1], best[2], 255]);
    }
    // (Its right side is never seen against the grass: the crown is as wide
    // on both sides of its middle.)
    for row in 0..h {
        if row as i64 + y0 >= CROWN_FOOT {
            break;
        }
        if let Some(first) = (0..w).find(|x| out.get(*x, row)[3] > 0) {
            let furthest = 2 * CROWN_MIDDLE - (first as i64 + x0);
            for x in (furthest - x0 + 1).max(0) as usize..w {
                out.at(x, row)[3] = 0;
            }
        }
    }
    Ok(out)
}

/// The tree's shadow, the colours the wood shows.
fn wood_shadow(workshop: &Workshop, tree: &Picture) -> Result<Picture, Diagnostic> {
    let (x0, y0, x1, y1) = SHADE_BOX;
    let [picture, mirrored] = wood(workshop)?;
    let mut covered = Mask::new(mirrored.w, mirrored.h);
    let (bx, by) = (BOX.0, BOX.1);
    for (x, y) in trees_in(&mirrored, &picture) {
        for at in (0..tree.cells.len()).filter(|at| tree.cells[*at][3] > 0) {
            let (px, py) = (x + (at % tree.w) as i64 + bx, y + (at / tree.w) as i64 + by);
            if px >= 0 && py >= 0 && px < covered.w as i64 && py < covered.h as i64 {
                covered.set(px as usize, py as usize, true);
            }
        }
    }
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let mut out = Picture::new(w, h);
    let (x, y) = SHADE_OF;
    for dy in y0..y1 {
        for dx in x0..x1 {
            let (px, py) = ((x + dx) as usize, (y + dy) as usize);
            let rgb = mirrored.get(px, py);
            if !covered.get(px, py) && TREE_SHADOW.contains(&rgb) {
                out.set((dx - x0) as usize, (dy - y0) as usize, [rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }
    let parts = components(&out.map(|rgba| rgba[3] > 0));
    if let Some(largest) = parts.first() {
        for at in 0..w * h {
            if !largest.cells[at] {
                out.cells[at] = [0; 4];
            }
        }
    }
    Ok(out)
}

/// The tree of the wood of 225: the whole tree on the upper layer, its
/// shadow made of the ground under it.
fn wood_tree(workshop: &Workshop, piece: &Piece) -> Result<Cut, Diagnostic> {
    let tree = wood_picture(workshop)?;
    let shadow = wood_shadow(workshop, &tree)?;
    let (bx, by, bx1, by1) = BOX;
    let (sx, sy, sx1, sy1) = SHADE_BOX;
    let (ux, uy) = (bx.min(sx), by.min(sy));
    let (w, h) = ((bx1.max(sx1) - ux) as usize, (by1.max(sy1) - uy) as usize);
    let mut upper = Picture::new(w, h);
    for y in 0..tree.h {
        for x in 0..tree.w {
            upper.set(x + (bx - ux) as usize, y + (by - uy) as usize, tree.get(x, y));
        }
    }
    let mut shade = Shade::none(w, h);
    let grass = light(GRASS[0]);
    for y in 0..shadow.h {
        for x in 0..shadow.w {
            let rgba = shadow.get(x, y);
            if rgba[3] > 0 {
                let (px, py) = (x + (sx - ux) as usize, y + (sy - uy) as usize);
                shade.colour.set(px, py, [rgba[0], rgba[1], rgba[2]]);
                shade.factor.set(px, py, (light([rgba[0], rgba[1], rgba[2]]) / grass) as f32);
            }
        }
    }
    shade.grounds = unique(GRASS.iter().map(|rgb| key(*rgb)));
    let solid = upper.map(|rgba| rgba[3] > 0);
    let seconds = match piece.level {
        Level::Place(level) => workshop.source(level)?.info.seconds as f64,
        Level::Thing(_) => 0.0,
    };
    let level = piece.place().unwrap_or(-1);
    Ok(Cut {
        lower: vec![Picture::new(w, h)],
        upper: vec![upper],
        solid,
        level,
        lower_index: None,
        upper_index: None,
        shade,
        foot: (7 - ux, 15 - uy),
        walked: false,
        seconds,
    })
}
