//! The Decor's brushes: the terrains painted (grass, paths, sand, water… of
//! a place of the game's own tiles, each tile found by its four corners
//! and its neighbours in the game: `decor::terrain`), and things strewn
//! (a piece laid here and there under the brush, never one on another).
//!
//! A terrain is a line of the decor (`terrain <niveau> x y …`): painted
//! here corner by corner, solved by the workshop while one paints, written
//! as one change when the stroke ends. Its walls come from its terrains
//! (the water, the cliffs), shown in red while one paints, said otherwise
//! in the Inspector. A terrain is painted by a brush, a rectangle, or
//! filled where it is the same; Alt+clic takes the terrain under the pointer.
//!
//! The walls are painted too, tile by tile (`mur cases`, `libre cases`): the
//! left button walls, the right lets walk on, Maj gives the tile back to
//! what is laid there.

use std::collections::{BTreeSet, HashMap, HashSet};

use decor::terrain::{NONE, Painted};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::Drawn;
use crate::workshop::{Ask, Solved, TerrainInfo};
use crate::{Decors, PieceArt, made_art, section, texture};
use workshop_base::theme;

/// What the mouse does on the scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Chooses and carries the elements.
    Choose,
    /// Paints terrains.
    Terrain,
    /// Strews a piece.
    Strew,
    /// Paints the walls, tile by tile.
    Walls,
    /// Walks a Pokémon, to try the walls.
    Walk,
}

/// How a terrain is painted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Brush,
    Rectangle,
    Fill,
}

/// A place's terrains, each with its patch made a texture.
pub type Terrains = Vec<(TerrainInfo, Option<egui::TextureHandle>)>;

/// A terrain as the editor draws it: its art, its walls, where it is.
#[derive(Clone)]
pub struct TerrainArt {
    pub art: PieceArt,
    pub walls: egui::TextureHandle,
    pub corner: [f32; 2],
    pub size: [f32; 2],
}

/// A stroke being painted: on which line (none: a new one), the terrain as it is now.
pub struct Painting {
    pub line: Option<usize>,
    pub painted: Painted,
    pub last: Option<(i64, i64)>,
    /// A rectangle's first corner, and its terrain.
    pub anchor: Option<(i64, i64)>,
    pub terrain: u8,
}

/// The brushes' state.
pub struct Brushes {
    pub tool: Tool,
    /// The place whose tiles are painted with; the terrain painted; how wide (in corners, around).
    pub level: i64,
    pub terrain: u8,
    pub size: i64,
    pub shape: Shape,
    /// Each place's terrains (none yet: asked).
    pub tilesets: HashMap<i64, Option<Terrains>>,
    /// Each terrain line solved, by its text; the one being solved.
    pub solved: HashMap<String, Option<TerrainArt>>,
    pub solving: Option<String>,
    /// What each terrain line showed last (`usize::MAX`: the new one): drawn while it is solved anew.
    pub last_drawn: HashMap<usize, TerrainArt>,
    pub painting: Option<Painting>,
    /// The walls shown while painting.
    pub walls_shown: bool,
    /// The piece strewn, how wide the brush, how far apart its pieces; the stroke being strewn.
    pub strewn: Option<String>,
    pub reach: f32,
    pub apart: f32,
    pub strewing: Option<Vec<[f32; 2]>>,
    chance: u64,
    /// A place's ground asked, to begin from.
    pub ground_asked: Option<i64>,
    /// The terrains whose changing colours were asked (once each).
    pub living_asked: HashSet<String>,
    /// The walls' brush: how wide (in tiles, around); the stroke: walled,
    /// let walk on, or given back (none), its tiles.
    pub wall_size: i64,
    pub walls_stroke: Option<(Option<bool>, crate::edit::Tiles)>,
}

impl Default for Brushes {
    fn default() -> Brushes {
        Brushes {
            tool: Tool::Choose,
            level: 323,
            terrain: 0,
            size: 1,
            tilesets: HashMap::new(),
            solved: HashMap::new(),
            solving: None,
            last_drawn: HashMap::new(),
            painting: None,
            walls_shown: true,
            strewn: None,
            reach: 32.0,
            apart: 1.0,
            strewing: None,
            chance: 0x9E37_79B9_7F4A_7C15,
            ground_asked: None,
            living_asked: HashSet::new(),
            wall_size: 0,
            walls_stroke: None,
            shape: Shape::Brush,
        }
    }
}

impl Brushes {
    /// A number at random, 0 to 1.
    fn random(&mut self) -> f32 {
        self.chance ^= self.chance << 13;
        self.chance ^= self.chance >> 7;
        self.chance ^= self.chance << 17;
        (self.chance >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// A place's terrains answered: their patches made textures.
pub fn tileset_answered(context: &egui::Context, decors: &mut Decors, level: i64, terrains: Vec<TerrainInfo>) {
    let made = terrains
        .into_iter()
        .map(|info| {
            let patch = info.patch.as_ref().map(|picture| texture(context, &format!("terrain {level} {}", info.letter), picture, None));
            (info, patch)
        })
        .collect();
    decors.brushes.tilesets.insert(level, Some(made));
}

/// A terrain line solved: made textures, kept by its text.
pub fn terrain_answered(context: &egui::Context, decors: &mut Decors, line: String, solved: Option<Solved>) {
    if decors.brushes.solving.as_ref() == Some(&line) {
        decors.brushes.solving = None;
    }
    let Some(solved) = solved else {
        // (Painted further since: the last one stays drawn.)
        decors.brushes.solved.entry(line).or_insert(None);
        return;
    };
    let words: Vec<String> = line.split_whitespace().skip(1).map(str::to_string).collect();
    let Ok(painted) = Painted::read(&words) else {
        return;
    };
    let art = made_art(context, "terrain", &solved.art);
    let walls = texture(context, "terrain walls", &solved.walls, None);
    let size = [solved.walls.w as f32, solved.walls.h as f32];
    let made = TerrainArt { art, walls, corner: [painted.x as f32, painted.y as f32], size };
    // (Kept: those of the decor now, the one painted.)
    if decors.brushes.solved.len() > 24
        && let Some(open) = &decors.open
    {
        let text = open.text.clone();
        decors.brushes.solved.retain(|known, _| text.contains(known.as_str()));
    }
    // (Its changing colours asked once it is painted: slower, in the background.)
    let painted_now = decors.brushes.painting.is_some();
    let in_text = decors.open.as_ref().is_some_and(|open| open.text.lines().any(|known| known.trim() == line));
    if !painted_now && in_text && decors.brushes.living_asked.insert(line.clone()) {
        decors.ask(Ask::TerrainLiving(line.clone()));
    }
    decors.brushes.solved.insert(line, Some(made));
}

/// The terrain of a line, to draw: as solved, or the last solved of it (or
/// of the stroke painted on it) while the new one is being solved.
pub fn drawn(decors: &mut Decors, number: usize, line: &str) -> Option<(Drawn, TerrainArt)> {
    let painted_now = decors.brushes.painting.as_ref().filter(|painting| painting.line == Some(number)).map(|painting| painting.painted.line());
    let wanted = painted_now.unwrap_or_else(|| line.trim().to_string());
    let made = match solved_or_asked(decors, &wanted) {
        Some(made) => {
            decors.brushes.last_drawn.insert(number, made.clone());
            made
        }
        None => decors.brushes.last_drawn.get(&number).cloned()?,
    };
    let rect = Rect::from_min_size(pos2(made.corner[0], made.corner[1]), vec2(made.size[0], made.size[1]));
    Some((Drawn { art: made.art.clone(), rect, layers: (true, true), turned: false, tint: Color32::WHITE }, made))
}

/// The stroke being painted on a new line, to draw.
pub fn drawn_new(decors: &mut Decors) -> Option<(Drawn, TerrainArt)> {
    let painting = decors.brushes.painting.as_ref().filter(|painting| painting.line.is_none())?;
    let wanted = painting.painted.line();
    let made = match solved_or_asked(decors, &wanted) {
        Some(made) => {
            decors.brushes.last_drawn.insert(usize::MAX, made.clone());
            made
        }
        None => decors.brushes.last_drawn.get(&usize::MAX).cloned()?,
    };
    let rect = Rect::from_min_size(pos2(made.corner[0], made.corner[1]), vec2(made.size[0], made.size[1]));
    Some((Drawn { art: made.art.clone(), rect, layers: (true, true), turned: false, tint: Color32::WHITE }, made))
}

/// A line's terrain if solved; asked if not (one at a time: the one painted last).
fn solved_or_asked(decors: &mut Decors, line: &str) -> Option<TerrainArt> {
    match decors.brushes.solved.get(line) {
        Some(Some(made)) => Some(made.clone()),
        Some(None) if decors.brushes.solving.is_some() => None,
        _ => {
            if decors.brushes.solving.is_none() {
                decors.brushes.solving = Some(line.to_string());
                decors.ask(Ask::Terrain(line.to_string()));
            }
            None
        }
    }
}

/// A place's terrains, asked if not there yet.
fn terrains_of(decors: &mut Decors, level: i64) -> Option<Terrains> {
    match decors.brushes.tilesets.get(&level) {
        Some(made) => made.clone(),
        None => {
            decors.brushes.tilesets.insert(level, None);
            decors.ask(Ask::Tileset(level));
            None
        }
    }
}

/// The lines of the decor that are terrains: their numbers and what they say.
pub fn terrain_lines(text: &str) -> Vec<(usize, Painted)> {
    text.lines()
        .enumerate()
        .filter_map(|(at, line)| {
            let words: Vec<String> = line.split('#').next().unwrap_or_default().split_whitespace().map(str::to_string).collect();
            (words.first()? == "terrain").then_some(())?;
            Some((at + 1, Painted::read(&words[1..]).ok()?))
        })
        .collect()
}

/// Where a new terrain goes: after the decor's first lines (its name, its
/// size, its number), under all that is laid.
pub fn with_terrain_first(text: &str, line: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let heads = ["decor:", "name", "level", "grid", "seed", "like", "center"];
    let mut at = 0;
    while at < lines.len() {
        let first = lines[at].split_whitespace().next().unwrap_or("");
        if !(first.is_empty() || first.starts_with('#') || heads.contains(&first)) {
            break;
        }
        at += 1;
    }
    let mut made: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    made.insert(at, line.to_string());
    made.join("\n") + "\n"
}

/// The scene under a brush: painted or strewn on with the left button,
/// rubbed out with the right; what it touches shown.
#[allow(clippy::too_many_arguments)]
pub fn interact(
    ui: &egui::Ui,
    decors: &mut Decors,
    response: &egui::Response,
    painter: &egui::Painter,
    to_screen: &dyn Fn([f32; 2]) -> Pos2,
    to_place: &dyn Fn(Pos2) -> [f32; 2],
    scale: f32,
) {
    match decors.brushes.tool {
        Tool::Choose => {}
        Tool::Terrain => paint(ui, decors, response, painter, to_screen, to_place, scale),
        Tool::Strew => strew(ui, decors, response, painter, to_screen, to_place, scale),
        Tool::Walls => walls(ui, decors, response, painter, to_screen, to_place),
        Tool::Walk => crate::walk::walk(ui, decors, response, painter, to_screen, to_place, scale),
    }
}

fn paint(
    ui: &egui::Ui,
    decors: &mut Decors,
    response: &egui::Response,
    painter: &egui::Painter,
    to_screen: &dyn Fn([f32; 2]) -> Pos2,
    to_place: &dyn Fn(Pos2) -> [f32; 2],
    scale: f32,
) {
    let Some(open) = &decors.open else { return };
    let alt = ui.input(|input| input.modifiers.alt);
    let corner_of = |at: Pos2| {
        let [x, y] = to_place(at);
        ((x / 8.0).round() as i64, (y / 8.0).round() as i64)
    };
    // (The corner under the pointer, and what the brush will touch.)
    if let Some(at) = response.hover_pos().or(response.interact_pointer_pos()) {
        brush_shown(painter, &decors.brushes, to_screen(corner_place(corner_of(at))), alt, scale);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    // (Alt+clic: the terrain under the pointer taken, of the terrain laid last over it.)
    if alt {
        if response.clicked()
            && let Some(at) = response.interact_pointer_pos()
            && let Some((level, terrain)) = terrain_under(&open.text, corner_of(at))
        {
            (decors.brushes.level, decors.brushes.terrain) = (level, terrain);
        }
        return;
    }
    let pressed = response.dragged_by(egui::PointerButton::Primary)
        || response.dragged_by(egui::PointerButton::Secondary)
        || response.clicked()
        || response.secondary_clicked();
    let erase = response.dragged_by(egui::PointerButton::Secondary) || response.secondary_clicked();
    let [wide, high] = open.size();
    let edges = ((wide / 8.0).ceil() as i64, (high / 8.0).ceil() as i64);
    if pressed && let Some(at) = response.interact_pointer_pos() {
        let corner = corner_of(at);
        let brushes = &mut decors.brushes;
        if brushes.painting.is_none() {
            brushes.painting = Some(painting_begun(brushes, open, corner, erase));
        }
        let (size, shape) = (brushes.size, brushes.shape);
        let painting = brushes.painting.as_mut().expect("painting");
        match shape {
            Shape::Brush => painting.brushed(corner, size),
            Shape::Rectangle => _ = painting.anchor.get_or_insert(corner),
            Shape::Fill if painting.last.is_none() => painting.filled(corner, edges),
            Shape::Fill => {}
        }
        painting.last = Some(corner);
        ui.ctx().request_repaint();
    }
    // (A rectangle being drawn: shown from its first corner to the pointer.)
    if let Some(painting) = &decors.brushes.painting
        && let (Some(anchor), Some(last)) = (painting.anchor, painting.last)
    {
        let area = Rect::from_two_pos(to_screen(corner_place(anchor)), to_screen(corner_place(last)));
        painter.rect_stroke(area, 0.0, Stroke::new(1.5_f32, if painting.terrain == NONE { theme::WRONG } else { Color32::WHITE }), StrokeKind::Inside);
    }
    // (The stroke ended: one change of the decor.)
    let ended = !response.is_pointer_button_down_on() && decors.brushes.painting.is_some();
    if ended && let Some(painting) = decors.brushes.painting.take() {
        stroke_ended(decors, painting);
    }
}

/// Where a corner is in the place.
fn corner_place((cx, cy): (i64, i64)) -> [f32; 2] {
    [cx as f32 * 8.0, cy as f32 * 8.0]
}

/// What the brush will touch, around the corner under the pointer (Alt: the eyedropper).
fn brush_shown(painter: &egui::Painter, brushes: &Brushes, centre: Pos2, alt: bool, scale: f32) {
    let colour = brushes
        .tilesets
        .get(&brushes.level)
        .cloned()
        .flatten()
        .and_then(|terrains| terrains.get(brushes.terrain as usize).map(|(info, _)| info.colour))
        .unwrap_or([255, 255, 255]);
    let colour = Color32::from_rgb(colour[0], colour[1], colour[2]);
    if alt {
        painter.rect_stroke(Rect::from_center_size(centre, vec2(10.0, 10.0)), 0.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Outside);
        crate::tag(painter, centre + vec2(10.0, -6.0), "eyedropper", Color32::WHITE);
        return;
    }
    match brushes.shape {
        Shape::Brush => {
            let reach = (brushes.size as f32 + 0.5) * 8.0;
            painter.circle_stroke(centre, reach * scale, Stroke::new(1.5_f32, colour));
            painter.circle_stroke(centre, reach * scale + 1.5, Stroke::new(1.0_f32, Color32::from_black_alpha(160)));
        }
        Shape::Rectangle => _ = painter.circle_filled(centre, 3.0, colour),
        Shape::Fill => {
            painter.circle_filled(centre, 3.0, colour);
            crate::tag(painter, centre + vec2(10.0, -6.0), "fill", colour);
        }
    }
}

/// The terrain at a corner, of the terrain line laid last over it: its place, its terrain.
fn terrain_under(text: &str, (cx, cy): (i64, i64)) -> Option<(i64, u8)> {
    terrain_lines(text).into_iter().rev().find_map(|(_, painted)| {
        let (vx, vy) = (cx - painted.x.div_euclid(8), cy - painted.y.div_euclid(8));
        let inside = vx >= 0 && vy >= 0 && (vx as usize) < painted.corners.w && (vy as usize) < painted.corners.h;
        let terrain = if inside { painted.corners.get(vx as usize, vy as usize) } else { NONE };
        (terrain != NONE).then_some((painted.level, terrain))
    })
}

/// A stroke begun: on the terrain chosen, else the first of that place, else a new one.
fn painting_begun(brushes: &mut Brushes, open: &crate::Open, (cx, cy): (i64, i64), erase: bool) -> Painting {
    let level = brushes.level;
    let lines = terrain_lines(&open.text);
    let chosen = open.chosen.and_then(|(line, _)| lines.iter().find(|(number, painted)| *number == line && painted.level == level));
    let found = chosen.or_else(|| lines.iter().find(|(_, painted)| painted.level == level));
    let (line, painted) = match found {
        Some((number, painted)) => (Some(*number), painted.clone()),
        None => (None, Painted { level, x: cx * 8, y: cy * 8, corners: decor::pixels::Grid::filled(1, 1, NONE), walls: None }),
    };
    if line.is_none() {
        brushes.last_drawn.remove(&usize::MAX);
    }
    let terrain = if erase { NONE } else { brushes.terrain };
    Painting { line, painted, last: None, anchor: None, terrain }
}

impl Painting {
    /// The brush from the last corner painted to this one: no gap when the mouse goes fast.
    fn brushed(&mut self, (cx, cy): (i64, i64), size: i64) {
        let (fx, fy) = self.last.unwrap_or((cx, cy));
        let steps = (cx - fx).abs().max((cy - fy).abs()).max(1);
        for step in 0..=steps {
            let (px, py) = (fx + (cx - fx) * step / steps, fy + (cy - fy) * step / steps);
            self.painted = self.painted.holding((px - size, py - size), (px + size, py + size));
            let (ox, oy) = (self.painted.x.div_euclid(8), self.painted.y.div_euclid(8));
            for dy in -size..=size {
                for dx in -size..=size {
                    if dx * dx + dy * dy <= size * size + size {
                        self.painted.corners.set((px + dx - ox) as usize, (py + dy - oy) as usize, self.terrain);
                    }
                }
            }
        }
    }

    /// The whole decor held (`edges`: its last corner); the corners alike to the one clicked, touching it, made
    /// the terrain.
    fn filled(&mut self, (cx, cy): (i64, i64), (columns, rows): (i64, i64)) {
        let terrain = self.terrain;
        self.painted = self.painted.holding((0, 0), (columns, rows)).holding((cx, cy), (cx, cy));
        let (ox, oy) = (self.painted.x.div_euclid(8), self.painted.y.div_euclid(8));
        let grid = &mut self.painted.corners;
        let start = ((cx - ox) as usize, (cy - oy) as usize);
        let alike = grid.get(start.0, start.1);
        if alike == terrain {
            return;
        }
        let mut waiting = vec![start];
        while let Some((vx, vy)) = waiting.pop() {
            if grid.get(vx, vy) != alike {
                continue;
            }
            grid.set(vx, vy, terrain);
            let (x, y) = (vx as i64 + ox, vy as i64 + oy);
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                let (gx, gy) = (nx - ox, ny - oy);
                // (Not past the decor's edges.)
                if (0..=columns).contains(&nx) && (0..=rows).contains(&ny) && gx >= 0 && gy >= 0 && (gx as usize) < grid.w && (gy as usize) < grid.h {
                    waiting.push((gx as usize, gy as usize));
                }
            }
        }
    }

    /// A rectangle drawn: from its first corner to the last, the terrain.
    fn rectangle_laid(&mut self) {
        if let (Some((ax, ay)), Some((cx, cy))) = (self.anchor, self.last) {
            let (x0, y0, x1, y1) = (ax.min(cx), ay.min(cy), ax.max(cx), ay.max(cy));
            self.painted = self.painted.holding((x0, y0), (x1, y1));
            let (ox, oy) = (self.painted.x.div_euclid(8), self.painted.y.div_euclid(8));
            for vy in y0..=y1 {
                for vx in x0..=x1 {
                    self.painted.corners.set((vx - ox) as usize, (vy - oy) as usize, self.terrain);
                }
            }
        }
    }
}

/// The stroke ended: one change of the decor, its terrain line chosen.
fn stroke_ended(decors: &mut Decors, mut painting: Painting) {
    painting.rectangle_laid();
    let made = painting.painted.trimmed();
    let Some(open) = &mut decors.open else { return };
    let text = match (painting.line, made.is_empty()) {
        (Some(number), true) => open.with_line(number, None),
        (Some(number), false) => open.with_line(number, Some(made.line())),
        (None, true) => open.text.clone(),
        (None, false) => with_terrain_first(&open.text, &made.line()),
    };
    open.change(text);
    if !made.is_empty() {
        let number = terrain_lines(&open.text).iter().find(|(_, painted)| *painted == made).map(|(number, _)| *number);
        open.choose(number.map(|number| vec![(number, 0)]).unwrap_or_default());
    }
}

/// The walls painted under the pointer: left walls, right lets walk on, Maj gives back.
fn walls(
    ui: &egui::Ui,
    decors: &mut Decors,
    response: &egui::Response,
    painter: &egui::Painter,
    to_screen: &dyn Fn([f32; 2]) -> Pos2,
    to_place: &dyn Fn(Pos2) -> [f32; 2],
) {
    let size = decors.brushes.wall_size;
    let shift = ui.input(|input| input.modifiers.shift);
    let tile_of = |at: Pos2| {
        let [x, y] = to_place(at);
        ((x / 8.0).floor() as i64, (y / 8.0).floor() as i64)
    };
    if let Some(at) = response.hover_pos().or(response.interact_pointer_pos()) {
        let (column, row) = tile_of(at);
        let corner = |column: i64, row: i64| to_screen([column as f32 * 8.0, row as f32 * 8.0]);
        let area = Rect::from_min_max(corner(column - size, row - size), corner(column + size + 1, row + size + 1));
        let colour = if shift { Color32::WHITE } else { theme::WRONG };
        painter.rect_stroke(area, 0.0, Stroke::new(1.5_f32, colour), StrokeKind::Inside);
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    let left = response.dragged_by(egui::PointerButton::Primary) || response.clicked();
    let right = response.dragged_by(egui::PointerButton::Secondary) || response.secondary_clicked();
    if (left || right)
        && let Some(at) = response.interact_pointer_pos()
    {
        let how = if shift { None } else { Some(left) };
        let (column, row) = tile_of(at);
        let stroke = decors.brushes.walls_stroke.get_or_insert_with(|| (how, BTreeSet::new()));
        for dy in -size..=size {
            for dx in -size..=size {
                if column + dx >= 0 && row + dy >= 0 {
                    stroke.1.insert((column + dx, row + dy));
                }
            }
        }
        ui.ctx().request_repaint();
    }
    if !response.is_pointer_button_down_on()
        && let Some((how, tiles)) = decors.brushes.walls_stroke.take()
        && let Some(open) = &mut decors.open
    {
        let made = crate::edit::with_walls(&open.text, &tiles, how);
        open.change(made);
    }
}

/// The walls' stroke being painted, over the scene.
pub fn stroke_over(painter: &egui::Painter, decors: &Decors, on_screen: &dyn Fn(Rect) -> Rect) {
    let Some((how, tiles)) = &decors.brushes.walls_stroke else { return };
    let colour = match how {
        Some(true) => theme::WRONG.gamma_multiply(0.6),
        Some(false) => Color32::from_rgb(0x5A, 0xC8, 0xFA).gamma_multiply(0.6),
        None => Color32::from_white_alpha(90),
    };
    for (column, row) in tiles {
        painter.rect_filled(on_screen(Rect::from_min_size(pos2(*column as f32 * 8.0, *row as f32 * 8.0), vec2(8.0, 8.0))), 0.0, colour);
    }
}

/// The walls' brush: how wide; the walls painted by hand given back.
pub fn walls_panel(ui: &mut egui::Ui, decors: &mut Decors) {
    section(ui, "Walls");
    ui.add_space(4.0);
    ui.label(theme::weak("Left-click: wall · right-click: walkable · Shift+click: as what is laid there"));
    ui.add_space(6.0);
    egui::Grid::new("walls").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(theme::weak("Size"));
        ui.add(egui::Slider::new(&mut decors.brushes.wall_size, 0..=6).suffix(" tile(s) around"));
        ui.end_row();
    });
    ui.add_space(6.0);
    let Some(open) = &decors.open else { return };
    let (walled, free) = crate::edit::painted_walls(&open.text);
    ui.label(theme::weak(format!(
        "{} tiles walled, {} free by hand; the rest comes from what is laid (in pale red, at the last bake).",
        walled.len(),
        free.len()
    )));
    ui.add_space(6.0);
    if ui.button("Give Back All the Tiles").on_hover_text("the walls painted by hand taken out: those of what is laid stay").clicked() {
        let all: BTreeSet<(i64, i64)> = walled.union(&free).copied().collect();
        if let Some(open) = &mut decors.open {
            let made = crate::edit::with_walls(&open.text, &all, None);
            open.change(made);
        }
    }
}

/// A put line on that layer.
fn decors_line(name: &str, points: &str, layer: &str) -> String {
    match layer {
        "ground" | "over" => format!("put {name} {points} layer {layer}"),
        _ => format!("put {name} {points}"),
    }
}

fn strew(
    ui: &egui::Ui,
    decors: &mut Decors,
    response: &egui::Response,
    painter: &egui::Painter,
    to_screen: &dyn Fn([f32; 2]) -> Pos2,
    to_place: &dyn Fn(Pos2) -> [f32; 2],
    scale: f32,
) {
    let Some(name) = decors.brushes.strewn.clone() else {
        if let Some(at) = response.hover_pos() {
            painter.text(
                at + vec2(14.0, 0.0),
                egui::Align2::LEFT_CENTER,
                "choose a piece in the Library",
                egui::FontId::proportional(theme::SMALL),
                Color32::WHITE,
            );
        }
        return;
    };
    let [w, h] = decors.size_of(&name);
    let reach = decors.brushes.reach;
    let layer = decors.active_layer;
    if let Some(at) = response.hover_pos().or(response.interact_pointer_pos()) {
        painter.circle_stroke(at, reach * scale, Stroke::new(1.5_f32, theme::DECOR));
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    let art = decors.art_now(&name);
    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
        if let Some(at) = response.interact_pointer_pos() {
            let [x, y] = to_place(at);
            let apart = w.max(h) * decors.brushes.apart;
            let laid = decors.open.as_ref().map(|open| strewn_points(&open.text, &name)).unwrap_or_default();
            let mut points = decors.brushes.strewing.take().unwrap_or_default();
            // (A few tries a frame: one where there is room.)
            for _ in 0..4 {
                let (angle, far) = (decors.brushes.random() * std::f32::consts::TAU, decors.brushes.random().sqrt() * reach);
                let point = [(x + angle.cos() * far - w / 2.0).round(), (y + angle.sin() * far - h / 2.0).round()];
                let room = points.iter().chain(&laid).all(|other: &[f32; 2]| (other[0] - point[0]).hypot(other[1] - point[1]) >= apart);
                if room {
                    points.push(point);
                    break;
                }
            }
            decors.brushes.strewing = Some(points);
            ui.ctx().request_repaint();
        }
    } else if let Some(points) = decors.brushes.strewing.take()
        && !points.is_empty()
        && let Some(open) = &mut decors.open
    {
        let said: Vec<String> = points.iter().map(|[x, y]| format!("{x} {y}")).collect();
        let line = decors_line(&name, &said.join(", "), layer);
        let made = open.with_added(&line);
        open.change(made);
    }
    // (What is being strewn, shown where it goes.)
    if let (Some(points), Some(art)) = (&decors.brushes.strewing, art) {
        for point in points {
            let rect = Rect::from_min_max(to_screen(*point), to_screen([point[0] + w, point[1] + h]));
            crate::paint_art(painter, &art, rect, (true, true), 0, false, Color32::from_white_alpha(220));
        }
    }
}

/// The points where a piece is laid in the decor already.
fn strewn_points(text: &str, name: &str) -> Vec<[f32; 2]> {
    decor::doc::laid_in(text).into_iter().filter(|one| one.name == name).map(|one| one.corner).collect()
}

/// Walls shown over the terrains while painting.
pub fn walls_over(painter: &egui::Painter, decors: &Decors, arts: &[TerrainArt], on_screen: &dyn Fn(Rect) -> Rect) {
    if decors.brushes.tool != Tool::Terrain || !decors.brushes.walls_shown {
        return;
    }
    for made in arts {
        let rect = on_screen(Rect::from_min_size(pos2(made.corner[0], made.corner[1]), vec2(made.size[0], made.size[1])));
        painter.image(made.walls.id(), rect, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
    }
}

// --- The Inspector, under a brush ---

/// The terrains of a place to paint with, the brush's size; a place's own
/// ground to begin from.
pub fn terrain_panel(ui: &mut egui::Ui, decors: &mut Decors) {
    section(ui, "Terrain Brush");
    ui.add_space(4.0);
    // (The places whose tiles are painted with.)
    let mut places: Vec<(i64, String)> = Vec::new();
    for piece in &decors.pieces {
        if let Some((level, name)) = &piece.place
            && !places.iter().any(|(known, _)| known == level)
        {
            places.push((*level, name.clone()));
        }
    }
    places.sort_by(|a, b| a.1.cmp(&b.1));
    let level = decors.brushes.level;
    let named = places.iter().find(|(known, _)| *known == level).map_or(format!("place {level}"), |(_, name)| name.clone());
    egui::Grid::new("brush").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(theme::weak("Tiles of"));
        egui::ComboBox::from_id_salt("terrain place").selected_text(&named).width(170.0).show_ui(ui, |ui| {
            for (known, name) in &places {
                if ui.selectable_label(*known == level, format!("{name} ({known})")).clicked() {
                    decors.brushes.level = *known;
                    decors.brushes.terrain = 0;
                }
            }
        });
        ui.end_row();
        ui.label(theme::weak("Shape"));
        ui.horizontal(|ui| {
            for (shape, word) in [(Shape::Brush, "Brush"), (Shape::Rectangle, "Rectangle"), (Shape::Fill, "Fill")] {
                if ui.selectable_label(decors.brushes.shape == shape, word).clicked() {
                    decors.brushes.shape = shape;
                }
            }
        });
        ui.end_row();
        ui.label(theme::weak("Size"));
        ui.add(egui::Slider::new(&mut decors.brushes.size, 0..=8).suffix(" corner(s)"));
        ui.end_row();
        ui.label(theme::weak("Walls"));
        ui.checkbox(&mut decors.brushes.walls_shown, "shown in red");
        ui.end_row();
    });
    ui.add_space(6.0);
    ui.label(theme::weak("Left-click: paint · right-click: erase · Alt+click: eyedropper"));
    ui.add_space(6.0);
    let Some(terrains) = terrains_of(decors, decors.brushes.level) else {
        ui.label(theme::weak("learning the place's terrains…"));
        return;
    };
    if terrains.is_empty() {
        ui.label(theme::weak("this place has no tiles to paint"));
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for (at, (info, patch)) in terrains.iter().enumerate() {
            let (rect, response) = ui.allocate_exact_size(vec2(52.0, 64.0), Sense::click());
            let chosen = decors.brushes.terrain as usize == at;
            ui.painter().rect_filled(
                rect,
                2.0,
                if chosen {
                    theme::CHOSEN
                } else if response.hovered() {
                    theme::HOVERED
                } else {
                    theme::FIELD
                },
            );
            let square = Rect::from_min_size(rect.min + vec2(2.0, 2.0), vec2(48.0, 48.0));
            match patch {
                Some(patch) => _ = ui.painter().image(patch.id(), square, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE),
                None => _ = ui.painter().rect_filled(square, 0.0, Color32::from_rgb(info.colour[0], info.colour[1], info.colour[2])),
            }
            if info.wall {
                ui.painter().rect_stroke(square, 0.0, Stroke::new(1.0_f32, theme::WRONG), StrokeKind::Inside);
            }
            ui.painter().text(
                pos2(rect.center().x, rect.bottom() - 7.0),
                egui::Align2::CENTER_CENTER,
                format!("{}  {:.0}%", info.letter, info.share * 100.0),
                egui::FontId::proportional(theme::SMALL - 1.0),
                theme::TEXT,
            );
            let wall = if info.wall { "\nwall: not walkable" } else { "" };
            if response.on_hover_text(format!("terrain {} · {:.1} % of the place{wall}", info.letter, info.share * 100.0)).clicked() {
                decors.brushes.terrain = at as u8;
            }
        }
    });
    ui.add_space(10.0);
    let begun = ui
        .button("Start from the Place's Ground")
        .on_hover_text("the whole ground of this game place, as a terrain to repaint (a new terrain at 0 0)")
        .clicked();
    if begun {
        decors.brushes.ground_asked = Some(decors.brushes.level);
        decors.ask(Ask::PlaceGround(decors.brushes.level));
    }
}

/// A place's ground answered: a new terrain line of it.
pub fn ground_answered(decors: &mut Decors, level: i64, line: Option<String>) {
    if decors.brushes.ground_asked != Some(level) {
        return;
    }
    decors.brushes.ground_asked = None;
    if let (Some(line), Some(open)) = (line, &mut decors.open) {
        let made = with_terrain_first(&open.text, &line);
        open.change(made);
        let number = terrain_lines(&open.text).iter().find(|(_, painted)| painted.line() == line).map(|(number, _)| *number);
        open.choose(number.map(|number| vec![(number, 0)]).unwrap_or_default());
    }
}

/// The strewing brush: its piece, how wide, how far apart.
pub fn strew_panel(ui: &mut egui::Ui, decors: &mut Decors) {
    section(ui, "Scatter");
    ui.add_space(4.0);
    let name = decors.brushes.strewn.clone();
    ui.horizontal(|ui| {
        let (square, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
        ui.painter().rect_filled(square, 2.0, theme::FIELD);
        if let Some(name) = &name
            && let Some(art) = decors.art_now(name)
        {
            crate::paint_thumb(ui.painter(), &art, square.shrink(3.0));
        }
        ui.label(match &name {
            Some(name) => theme::text(decors.shown_name(name), theme::BODY, theme::STRONG),
            None => theme::weak("a piece: click it in the Library"),
        });
    });
    ui.add_space(6.0);
    egui::Grid::new("strew").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(theme::weak("Radius"));
        ui.add(egui::Slider::new(&mut decors.brushes.reach, 4.0..=160.0).suffix(" px"));
        ui.end_row();
        ui.label(theme::weak("Spacing"));
        ui.add(egui::Slider::new(&mut decors.brushes.apart, 0.5..=3.0).suffix(" × its size"));
        ui.end_row();
    });
    ui.add_space(6.0);
    ui.label(theme::weak("Drag on the scene: each stroke, one \"put\" line of all its points."));
}

/// A terrain line looked at: its place, its terrains and which are walls
/// (said here, the line's `walls`), its size; what is done to it.
pub fn terrain_inspector(ui: &mut egui::Ui, decors: &mut Decors, number: usize) {
    let Some(text) = decors.open.as_ref().map(|open| open.text.clone()) else { return };
    let Some((_, painted)) = terrain_lines(&text).into_iter().find(|(line, _)| *line == number) else {
        return;
    };
    let place = decors.pieces.iter().find_map(|piece| piece.place.as_ref().filter(|(level, _)| *level == painted.level).map(|(_, name)| name.clone()));
    ui.label(theme::text(format!("Terrain · {}", place.unwrap_or_else(|| format!("place {}", painted.level))), theme::BODY, theme::STRONG).strong());
    ui.label(
        RichText::new(format!("line {number} · {} × {} tiles · at {} {}", painted.columns(), painted.rows(), painted.x, painted.y))
            .monospace()
            .size(theme::SMALL - 1.0)
            .color(theme::FAINT),
    );
    ui.add_space(6.0);
    section(ui, "Where No One Walks");
    ui.add_space(4.0);
    let Some(terrains) = terrains_of(decors, painted.level) else {
        ui.label(theme::weak("learning the place's terrains…"));
        return;
    };
    let used: Vec<u8> = {
        let mut seen: Vec<u8> = painted.corners.cells.iter().copied().filter(|terrain| *terrain != NONE).collect();
        seen.sort_unstable();
        seen.dedup();
        seen
    };
    let walls_now: Vec<u8> =
        painted.walls.clone().unwrap_or_else(|| terrains.iter().enumerate().filter(|(_, (info, _))| info.wall).map(|(at, _)| at as u8).collect());
    let mut walls = walls_now.clone();
    for terrain in &used {
        let Some((info, patch)) = terrains.get(*terrain as usize) else { continue };
        ui.horizontal(|ui| {
            let (square, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
            match patch {
                Some(patch) => _ = ui.painter().image(patch.id(), square, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE),
                None => _ = ui.painter().rect_filled(square, 0.0, Color32::from_rgb(info.colour[0], info.colour[1], info.colour[2])),
            }
            let mut wall = walls.contains(terrain);
            if ui.checkbox(&mut wall, format!("{}  wall", info.letter)).changed() {
                if wall {
                    walls.push(*terrain);
                } else {
                    walls.retain(|known| known != terrain);
                }
            }
        });
    }
    let Some(open) = &decors.open else { return };
    let mut made: Option<String> = None;
    if walls != walls_now {
        walls.sort_unstable();
        let made_line = Painted { walls: Some(walls), ..painted.clone() }.line();
        made = Some(open.with_line(number, Some(made_line)));
    }
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        if ui.button("Paint").on_hover_text("the terrain brush on it (B)").clicked() {
            decors.brushes.tool = Tool::Terrain;
            decors.brushes.level = painted.level;
        }
        if ui.button("To Front").clicked() {
            made = Some(open.moved_line(number, true));
        }
        if ui.button("To Back").clicked() {
            made = Some(open.moved_line(number, false));
        }
        if ui.button(RichText::new("Remove").color(theme::WRONG)).clicked() {
            made = Some(open.with_line(number, None));
        }
    });
    if let (Some(made), Some(open)) = (made, &mut decors.open) {
        open.change(made);
        if terrain_lines(&open.text).iter().all(|(line, _)| *line != number) {
            open.choose(Vec::new());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn painting(terrain: u8) -> Painting {
        let painted = Painted { level: 1, x: 0, y: 0, corners: decor::pixels::Grid::filled(1, 1, NONE), walls: None };
        Painting { line: None, painted, last: None, anchor: None, terrain }
    }

    fn at(painting: &Painting, (x, y): (i64, i64)) -> u8 {
        let (ox, oy) = (painting.painted.x.div_euclid(8), painting.painted.y.div_euclid(8));
        painting.painted.corners.get((x - ox) as usize, (y - oy) as usize)
    }

    #[test]
    fn a_fill_takes_the_whole_decor_and_a_rectangle_its_corners() {
        let mut filled = painting(2);
        filled.filled((1, 1), (2, 2));
        assert!((0..=2).all(|y| (0..=2).all(|x| at(&filled, (x, y)) == 2)));
        let mut laid = painting(3);
        (laid.anchor, laid.last) = (Some((1, 1)), Some((2, 3)));
        laid.rectangle_laid();
        assert_eq!(at(&laid, (2, 3)), 3);
        assert_eq!(at(&laid, (1, 2)), 3);
        assert_eq!(at(&laid, (0, 0)), NONE);
    }
}
