//! What the pointer does on the canvas, by the tool in hand: the picker
//! (Alt+click), the pencil, the eraser, the dither, the shade, the fill, the
//! line, the rectangle, the ellipse, the selection, the move.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Vec2, vec2};
use image::Rgba;

use super::pixels::{composite, ellipse_points, filled, line_points, rectangle_points, rgba};
use super::{Painting, Tool};

/// The pixel of the picture under a point of the canvas, the picture at `min`, so zoomed.
pub(super) fn pixel_at(min: Pos2, zoom: f32, at: Pos2) -> (i32, i32) {
    (((at.x - min.x) / zoom).floor() as i32, ((at.y - min.y) / zoom).floor() as i32)
}

/// Where a shape drawn from `from` ends with Shift: a square, a circle, a
/// line flat, upright or at 45°.
pub(super) fn held_square(from: (i32, i32), point: (i32, i32), line: bool) -> (i32, i32) {
    let (dx, dy) = (point.0 - from.0, point.1 - from.1);
    let side = dx.abs().max(dy.abs());
    if line && (dx.abs() * 2 < dy.abs() || dy.abs() * 2 < dx.abs()) {
        if dx.abs() > dy.abs() { (point.0, from.1) } else { (from.0, point.1) }
    } else {
        (from.0 + side * dx.signum(), from.1 + side * dy.signum())
    }
}

/// The pointer on the canvas, the picture shown in `shown`.
pub(super) fn pointer(ui: &egui::Ui, painting: &mut Painting, response: &egui::Response, painter: &Painter, shown: Rect) {
    let (pressed_primary, pressed_secondary, alt) = ui.input(|input| (input.pointer.primary_pressed(), input.pointer.secondary_pressed(), input.modifiers.alt));
    let started = response.hovered() && (pressed_primary || pressed_secondary);
    if started {
        painting.secondary_down = pressed_secondary;
    }
    let down = ui.input(|input| input.pointer.primary_down() || input.pointer.secondary_down());
    let Some(at) = response.hover_pos().or(response.interact_pointer_pos()) else { return };
    let point = pixel_at(shown.min, painting.zoom, at);
    let colour =
        if painting.tool == Tool::Eraser { Rgba([0, 0, 0, 0]) } else { rgba(if painting.secondary_down { painting.secondary } else { painting.primary }) };
    // (Alt+clic: the picker, whatever the tool.)
    if started && (alt || painting.tool == Tool::Picker) {
        picked(painting, point);
        return;
    }
    match painting.tool {
        Tool::Pencil | Tool::Eraser | Tool::Dither => freehand(painting, point, colour, started, down),
        Tool::Shade => shade(painting, point, started, down),
        Tool::Fill if started => fill(painting, point, colour),
        Tool::Line | Tool::Rectangle | Tool::Ellipse => {
            if started {
                painting.from = Some(point);
            }
            if let Some(from) = painting.from {
                let shift = ui.input(|input| input.modifiers.shift);
                shape(painting, painter, shown, from, point, colour, shift, down);
            }
        }
        Tool::Select => select(painting, point, started, down, response.clicked()),
        Tool::Move => moved(painting, point, started, down),
        _ => {}
    }
    if !down {
        painting.last = None;
        painting.from = painting.from.filter(|_| false);
    }
}

/// The colour under the point (seen whole, all layers), taken as the one in hand.
fn picked(painting: &mut Painting, point: (i32, i32)) {
    if painting.paintable(point) || (0..painting.wide()).contains(&point.0) && (0..painting.high()).contains(&point.1) {
        let picked =
            composite(&painting.layers, painting.at).get_pixel(point.0.clamp(0, painting.wide() - 1) as u32, point.1.clamp(0, painting.high() - 1) as u32).0;
        if picked[3] > 0 {
            let picked = Color32::from_rgba_unmultiplied(picked[0], picked[1], picked[2], picked[3]);
            if painting.secondary_down {
                painting.secondary = picked;
            } else {
                painting.primary = picked;
            }
        }
    }
}

/// The pencil, the eraser, the dither: a stroke to the point.
fn freehand(painting: &mut Painting, point: (i32, i32), colour: Rgba<u8>, started: bool, down: bool) {
    if started {
        painting.keep(match painting.tool {
            Tool::Eraser => "eraser",
            Tool::Dither => "dither",
            _ => "pencil",
        });
        painting.trail.clear();
        painting.last = None;
        if painting.tool != Tool::Eraser {
            painting.used(if painting.secondary_down { painting.secondary } else { painting.primary });
        }
    }
    if down && (started || painting.last.is_some()) {
        painting.stroke_to(point, colour);
    }
}

/// The shade: each pixel of the way lighter (or darker, the other button).
fn shade(painting: &mut Painting, point: (i32, i32), started: bool, down: bool) {
    if started {
        painting.keep("shade");
        painting.shaded.clear();
        painting.last = None;
    }
    if down && (started || painting.last.is_some()) {
        let from = painting.last.unwrap_or(point);
        for step in line_points(from, point) {
            painting.shade(step, painting.secondary_down);
        }
        painting.last = Some(point);
    }
}

/// The fill, inside the selection (and mirrored, if asked).
fn fill(painting: &mut Painting, point: (i32, i32), colour: Rgba<u8>) {
    painting.keep("fill");
    let (layer, at, everywhere) = (painting.layer, painting.at, painting.everywhere);
    let selection = painting.selection;
    let inside = move |(x, y): (i32, i32)| selection.is_none_or(|[x0, y0, x1, y1]| (x0..=x1).contains(&x) && (y0..=y1).contains(&y));
    filled(&mut painting.layers[layer].cels[at], point, colour, everywhere, &inside);
    if painting.mirror_x {
        let mirrored = (painting.wide() - 1 - point.0, point.1);
        filled(&mut painting.layers[layer].cels[at], mirrored, colour, everywhere, &inside);
    }
    painting.used(if painting.secondary_down { painting.secondary } else { painting.primary });
    painting.changed(layer, at);
}

/// A line, a rectangle or an ellipse: shown while held, laid when let go.
fn shape(painting: &mut Painting, painter: &Painter, shown: Rect, from: (i32, i32), point: (i32, i32), colour: Rgba<u8>, shift: bool, down: bool) {
    let to = if shift { held_square(from, point, painting.tool == Tool::Line) } else { point };
    let points = match painting.tool {
        Tool::Line => line_points(from, to),
        Tool::Rectangle => rectangle_points(from, to, painting.filled),
        _ => ellipse_points(from, to, painting.filled),
    };
    if down {
        let [r, g, b, a] = colour.0;
        for (x, y) in &points {
            let cell = Rect::from_min_size(shown.min + vec2(*x as f32, *y as f32) * painting.zoom, Vec2::splat(painting.zoom));
            painter.rect_filled(cell, 0.0, Color32::from_rgba_unmultiplied(r, g, b, a.max(110)));
        }
        return;
    }
    painting.from = None;
    painting.keep(match painting.tool {
        Tool::Line => "line",
        Tool::Rectangle => "rectangle",
        _ => "ellipse",
    });
    for point in points {
        painting.lay(point, colour, 1);
    }
    painting.used(if painting.secondary_down { painting.secondary } else { painting.primary });
}

/// The selection: a rectangle drawn (a click without moving: none).
fn select(painting: &mut Painting, point: (i32, i32), started: bool, down: bool, clicked: bool) {
    if started {
        painting.drop_floating();
        painting.from = Some(point);
    }
    if let Some(from) = painting.from {
        let clamp = |(x, y): (i32, i32)| (x.clamp(0, painting.wide() - 1), y.clamp(0, painting.high() - 1));
        let (a, b) = (clamp(from), clamp(point));
        painting.selection = Some([a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)]);
        if !down {
            painting.from = None;
            if from == point && clicked {
                painting.selection = None;
            }
        }
    }
}

/// The move: what is chosen lifted, carried.
fn moved(painting: &mut Painting, point: (i32, i32), started: bool, down: bool) {
    if started {
        if painting.floating.is_none() {
            painting.keep("move");
            painting.lift();
        }
        painting.moving = Some(point);
    }
    if let Some(from) = painting.moving {
        if down {
            if let Some((_, at)) = &mut painting.floating {
                *at = (at.0 + point.0 - from.0, at.1 + point.1 - from.1);
            }
            painting.moving = Some(point);
        } else {
            painting.moving = None;
        }
    }
}
