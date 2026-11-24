//! What the Scene draws of the decor: the lines' things in order (the
//! terrains, the grounds' patterns, the pieces' layers), those who live
//! there, the upper layers; the walls, the terrains' over, the landmarks, the grid.

use decor::doc::Pose;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind, Vec2, pos2, vec2};

use super::{Drawn, Read, View, grid_step};
use crate::decors::{Decors, paint_art, tag};
use crate::edit::{self, Item, Kind};
use crate::terrains::Tool;
use workshop_base::theme;

/// The decor's things in the order they are drawn: under those who live
/// there, those who live there, over them; the pieces asked for (not yet
/// known); the terrains' art (their walls drawn over); the moment they are drawn at.
pub(super) struct Things {
    pub(super) lower: Vec<Drawn>,
    pub(super) upper: Vec<Drawn>,
    pub(super) lives: Vec<Item>,
    pub(super) asked: Vec<String>,
    pub(super) terrains: Vec<crate::terrains::TerrainArt>,
    pub(super) seconds: f64,
}

/// Where a ground's pattern is laid: its zone, or the whole decor (none: the zone does not read).
pub(super) fn ground_area(rest: &[&str], size: Vec2) -> Option<Rect> {
    match rest {
        ["zone", x0, y0, x1, y1, ..] => {
            let corners: Vec<f32> = [x0, y0, x1, y1].iter().filter_map(|word| word.parse().ok()).collect();
            (corners.len() == 4).then(|| Rect::from_min_max(pos2(corners[0], corners[1]), pos2(corners[2], corners[3])))
        }
        _ => Some(Rect::from_min_size(Pos2::ZERO, size)),
    }
}

/// The tiles of a pattern laid over an area, row by row from its corner.
pub(super) fn tiles_over(area: Rect, tile: Vec2) -> Vec<Rect> {
    let mut tiles = Vec::new();
    let mut y = area.min.y;
    while y < area.max.y {
        let mut x = area.min.x;
        while x < area.max.x {
            tiles.push(Rect::from_min_size(pos2(x, y), tile));
            x += tile.x;
        }
        y += tile.y;
    }
    tiles
}

/// The decor's things, line after line, as they are to be drawn (those on a hidden layer left out).
pub(super) fn things(decors: &mut Decors, read: &Read, size: Vec2, seconds: f64) -> Things {
    let mut things = Things { lower: Vec::new(), upper: Vec::new(), lives: Vec::new(), asked: Vec::new(), terrains: Vec::new(), seconds };
    let hidden = &read.hidden;
    if !hidden.contains("terrains")
        && let Some((mut drawn, made)) = crate::terrains::drawn_new(decors)
    {
        drawn.tint = decors.tint_of("terrains");
        things.lower.push(drawn);
        things.terrains.push(made);
    }
    let mut by_line: std::collections::BTreeMap<usize, Vec<&Item>> = std::collections::BTreeMap::new();
    for item in &read.items {
        by_line.entry(item.line).or_default().push(item);
    }
    for (at, line) in read.text.lines().enumerate() {
        let number = at + 1;
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["terrain", ..] => {
                if !hidden.contains("terrains")
                    && let Some((mut drawn, made)) = crate::terrains::drawn(decors, number, line)
                {
                    drawn.tint = decors.tint_of("terrains");
                    things.lower.push(drawn);
                    things.terrains.push(made);
                }
                continue;
            }
            ["ground", name, rest @ ..] => {
                if !hidden.contains("ground") {
                    ground_laid(decors, &mut things, name, rest, size);
                }
                continue;
            }
            _ => {}
        }
        if let Some(here) = by_line.get(&number) {
            line_things(decors, &mut things, read, line, here);
        }
    }
    things
}

/// A ground's pattern, tile after tile over its zone (its art asked for if not known yet).
fn ground_laid(decors: &Decors, things: &mut Things, name: &str, rest: &[&str], size: Vec2) {
    let Some(art) = decors.art_known(name) else {
        things.asked.push(name.to_string());
        return;
    };
    let Some(area) = ground_area(rest, size) else { return };
    let Some(first) = art.lower.first() else { return };
    for rect in tiles_over(area, first.size_vec2()) {
        things.lower.push(Drawn { art: art.clone(), rect, layers: (true, true), turned: false, tint: decors.tint_of("ground") });
    }
}

/// A line's things: those who live there, apart; a piece's layers (on the
/// ground: all under; over: all over; else its bottom under, its top over).
fn line_things(decors: &Decors, things: &mut Things, read: &Read, line: &str, here: &[&Item]) {
    let pose = Pose::read(line);
    for one in here {
        if read.hidden.contains(one.layer) {
            continue;
        }
        match one.kind {
            Kind::Lives => things.lives.push((*one).clone()),
            Kind::Piece => {
                let Some(art) = decors.art_known(&one.name) else {
                    things.asked.push(one.name.clone());
                    continue;
                };
                let rect = decors.item_box(one);
                let turned = pose.as_ref().is_some_and(|pose| pose.turned);
                let tint = decors.tint_of(one.layer);
                match one.layer {
                    "ground" => things.lower.push(Drawn { art, rect, layers: (true, true), turned, tint }),
                    "over" => things.upper.push(Drawn { art, rect, layers: (true, true), turned, tint }),
                    _ => {
                        things.lower.push(Drawn { art: art.clone(), rect, layers: (true, false), turned, tint });
                        things.upper.push(Drawn { art, rect, layers: (false, true), turned, tint });
                    }
                }
            }
            _ => {}
        }
    }
}

/// Things drawn where `place` puts them (those out of sight left), in their
/// own tint (or in that one, if it is not white).
pub(super) fn draw_all(painter: &egui::Painter, drawn: &[Drawn], place: &dyn Fn(Rect) -> Rect, tint: Color32, seconds: f64) {
    for one in drawn {
        let at = place(one.rect);
        if at.intersects(painter.clip_rect()) {
            paint_art(painter, &one.art, at, one.layers, one.art.frame(seconds), one.turned, if tint == Color32::WHITE { one.tint } else { tint });
        }
    }
}

/// The decor drawn: under, those who live there, over; the walls, the
/// terrains' over, the landmarks, the grid. The bodies asked for, given back.
pub(super) fn draw(painter: &egui::Painter, decors: &Decors, read: &Read, things: &Things, view: View) -> Vec<String> {
    let seconds = things.seconds;
    let clipped = painter.with_clip_rect(view.seen());
    let on_screen = |area: Rect| view.on_screen(area);
    draw_all(&clipped, &things.lower, &on_screen, Color32::WHITE, seconds);
    let bodies_asked = residents(&clipped, decors, &things.lives, view, seconds);
    draw_all(&clipped, &things.upper, &on_screen, Color32::WHITE, seconds);
    if !read.hidden.contains("walls") || matches!(read.tool, Tool::Walls | Tool::Walk) {
        walls(&clipped, decors, &read.text, view);
    }
    crate::terrains::walls_over(&clipped, decors, &things.terrains, &on_screen);
    crate::terrains::stroke_over(&clipped, decors, &on_screen);
    if !read.hidden.contains("markers") {
        landmarks(&clipped, decors, &read.items, view);
    }
    if decors.grid {
        grid(painter, grid_step(&read.text), view);
    }
    bodies_asked
}

/// Those who live there: their sprite waiting, the way they face (one not
/// found: a red dot and its name). The bodies not yet known, given back.
fn residents(painter: &egui::Painter, decors: &Decors, lives: &[Item], view: View, seconds: f64) -> Vec<String> {
    let mut bodies_asked: Vec<String> = Vec::new();
    for one in lives {
        let key = crate::decors::body_key(one);
        match decors.body_known(&key) {
            Some(Some(body)) => {
                if let Some((texture, around)) = body.at(one.facing, if one.still { 0.0 } else { seconds }) {
                    let place = view.on_screen(around.translate(vec2(one.corner[0], one.corner[1])));
                    painter.image(texture.id(), place, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), decors.tint_of("residents"));
                }
            }
            Some(None) => {
                let feet = view.to_screen(one.corner);
                painter.circle_filled(feet - vec2(0.0, 10.0 * view.scale), (8.0 * view.scale).max(3.0), theme::WRONG);
                painter.text(feet + vec2(0.0, 2.0), egui::Align2::CENTER_TOP, &one.name, egui::FontId::proportional(theme::SMALL), Color32::WHITE);
            }
            None => bodies_asked.push(key),
        }
    }
    bodies_asked
}

/// The walls, as baked; those painted by hand over them (walled in red, free in blue).
fn walls(painter: &egui::Painter, decors: &Decors, text: &str, view: View) {
    if let Some(walls) = &decors.walls {
        let [w, h] = [walls.size()[0] as f32 * 8.0, walls.size()[1] as f32 * 8.0];
        painter.image(walls.id(), view.on_screen(Rect::from_min_size(Pos2::ZERO, vec2(w, h))), Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
    }
    let (walled, free) = edit::painted_walls(text);
    for (tiles, colour) in [(&walled, theme::WRONG), (&free, Color32::from_rgb(0x5A, 0xC8, 0xFA))] {
        for (column, row) in tiles {
            let tile = view.on_screen(Rect::from_min_size(pos2(*column as f32 * 8.0, *row as f32 * 8.0), vec2(8.0, 8.0)));
            painter.rect_stroke(tile.shrink(0.5), 0.0, Stroke::new(1.0_f32, colour), StrokeKind::Inside);
        }
    }
}

/// The landmarks: spots and entrances as diamonds, exits and zones as areas; each named.
fn landmarks(painter: &egui::Painter, decors: &Decors, items: &[Item], view: View) {
    for one in items.iter().filter(|one| one.layer == "markers") {
        let place = view.on_screen(decors.item_box(one));
        let (colour, word) = match one.kind {
            Kind::Spot => (theme::DECOR, one.name.clone()),
            Kind::Entrance => (Color32::from_rgb(0x4A, 0xDE, 0x80), format!("entrance {}", one.name)),
            Kind::Zone => (theme::GAME, format!("zone {}", one.name)),
            _ => (theme::FILM, format!("exit {}", one.name)),
        };
        if matches!(one.kind, Kind::Exit | Kind::Zone) {
            painter.rect_filled(place, 0.0, colour.gamma_multiply(0.25));
            painter.rect_stroke(place, 0.0, Stroke::new(1.5_f32, colour), StrokeKind::Inside);
        } else {
            let centre = place.center();
            let points = vec![centre + vec2(0.0, -6.0), centre + vec2(6.0, 0.0), centre + vec2(0.0, 6.0), centre + vec2(-6.0, 0.0)];
            painter.add(egui::Shape::convex_polygon(points, colour, Stroke::new(1.0_f32, Color32::BLACK)));
        }
        tag(painter, place.left_top() + vec2(0.0, -2.0), &word, colour);
    }
}

/// The grid, when its squares are large enough to see.
fn grid(painter: &egui::Painter, step: f32, view: View) {
    if step * view.scale < 6.0 {
        return;
    }
    let visible = view.seen();
    let stroke = Stroke::new(1.0_f32, Color32::from_white_alpha(28));
    let [x0, y0] = view.to_place(visible.min);
    let [x1, y1] = view.to_place(visible.max);
    let mut x = (x0 / step).ceil() * step;
    while x <= x1 {
        let at = view.to_screen([x, 0.0]).x;
        painter.line_segment([pos2(at, visible.min.y), pos2(at, visible.max.y)], stroke);
        x += step;
    }
    let mut y = (y0 / step).ceil() * step;
    while y <= y1 {
        let at = view.to_screen([0.0, y]).y;
        painter.line_segment([pos2(visible.min.x, at), pos2(visible.max.x, at)], stroke);
        y += step;
    }
}
