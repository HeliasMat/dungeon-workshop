//! What is drawn over the film's picture: the place's grid, the camera's
//! frame, who is there (pointed, chosen, named by the line pointed in the
//! Script), the spots the film names, the point under the pointer, what the
//! mouse holds.

use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, vec2};

use super::super::{Films, Frame, Held};
use super::script::spots_of;
use super::{Place, View};
use crate::script::Scripts;
use crate::theme;

/// All the marks over the picture; the place's point under the pointer.
pub(super) fn over(ui: &egui::Ui, films: &Films, scripts: &mut Scripts, drawn: &Frame, view: &View, head: Rect, picture: &egui::Response) -> Option<[f32; 2]> {
    let painter = ui.painter_at(view.seen);
    let place = Place::of(view, drawn);
    if films.grid {
        grid(&painter, &place, view, drawn.close);
    }
    if let Some(camera) = drawn.camera {
        let frame = view.frame_of(camera);
        painter.rect_stroke(frame, 0.0, Stroke::new(1.5_f32, theme::FILM), StrokeKind::Inside);
        painter.text(frame.left_top() + vec2(4.0, 3.0), egui::Align2::LEFT_TOP, "camera", FontId::proportional(theme::SMALL), theme::FILM);
    }
    let pointer = picture.hover_pos();
    bodies(&painter, films, scripts, drawn, view, pointer);
    // (The places the film names: `endroit name x y`.)
    if films.names
        && let Some(open) = scripts.open.as_ref()
    {
        spots(&painter, &place, view, &open.text);
    }
    let place_point = pointer.map(|at| pointed(ui, drawn, &place, view, head, at));
    held(&painter, films, drawn, &place, view, pointer.or(ui.input(|input| input.pointer.hover_pos())));
    place_point
}

/// The grid's step in the place: 8, doubled while its lines would be closer than 6 points.
pub(super) fn grid_step(close: f32, scale: f32) -> f32 {
    let mut step = 8.0;
    while step * close * scale < 6.0 && step < 512.0 {
        step *= 2.0;
    }
    step
}

/// The place's grid, over the whole picture.
fn grid(painter: &Painter, place: &Place, view: &View, close: f32) {
    let step = grid_step(close, view.scale);
    let [x0, y0] = place.to_place(view.shown.min);
    let [x1, y1] = place.to_place(view.shown.max);
    let stroke = Stroke::new(1.0_f32, Color32::from_white_alpha(26));
    let mut x = (x0 / step).ceil() * step;
    while x <= x1 {
        painter.vline(place.to_screen([x, 0.0]).x, view.shown.y_range(), stroke);
        x += step;
    }
    let mut y = (y0 / step).ceil() * step;
    while y <= y1 {
        painter.hline(view.shown.x_range(), place.to_screen([0.0, y]).y, stroke);
        y += step;
    }
}

/// Who is there: the one chosen, those the line pointed in the Script names
/// (or the Timeline lights), all of them with Names, the one pointed (lit in the Script).
fn bodies(painter: &Painter, films: &Films, scripts: &mut Scripts, drawn: &Frame, view: &View, pointer: Option<Pos2>) {
    let named: std::collections::HashSet<String> = scripts
        .hovered_line
        .and_then(|line| scripts.open.as_ref().and_then(|open| open.text.split('\n').nth(line - 1).map(str::to_string)))
        .map(|line| line.split(|letter: char| !letter.is_alphanumeric() && letter != '_').map(str::to_string).collect())
        .unwrap_or_default();
    for (name, body) in &drawn.bodies {
        let frame = view.frame_of(*body);
        let pointed = pointer.is_some_and(|at| frame.contains(at));
        if pointed {
            scripts.lit_who_scene = Some(name.clone());
        }
        if !pointed && films.focus_who.as_ref() == Some(name) {
            painter.rect_stroke(frame, 0.0, Stroke::new(2.0_f32, theme::FILM), StrokeKind::Outside);
            crate::decors::tag(painter, frame.left_top() - vec2(0.0, 2.0), name, theme::FILM);
        } else if !pointed && (named.contains(name) || scripts.lit_who_track.as_ref() == Some(name)) {
            painter.rect_stroke(frame, 0.0, Stroke::new(2.0_f32, theme::POINTED), StrokeKind::Outside);
            crate::decors::tag(painter, frame.left_top() - vec2(0.0, 2.0), name, theme::POINTED);
        } else if films.names || pointed {
            painter.rect_stroke(frame, 0.0, Stroke::new(1.0_f32, if pointed { Color32::WHITE } else { theme::DECOR }), StrokeKind::Outside);
            crate::decors::tag(painter, frame.left_top() - vec2(0.0, 2.0), name, if pointed { Color32::WHITE } else { theme::DECOR });
        }
    }
}

/// The spots the film names, each a small diamond and its name.
fn spots(painter: &Painter, place: &Place, view: &View, text: &str) {
    for (name, point) in spots_of(text) {
        let at = place.to_screen(point);
        if view.seen.contains(at) {
            let mark = [at + vec2(0.0, -4.0), at + vec2(4.0, 0.0), at + vec2(0.0, 4.0), at + vec2(-4.0, 0.0)];
            painter.add(egui::Shape::convex_polygon(mark.to_vec(), theme::FILM, Stroke::new(1.0_f32, Color32::BLACK)));
            painter.text(at + vec2(6.0, 0.0), egui::Align2::LEFT_CENTER, &name, FontId::proportional(theme::SMALL - 1.0), theme::FILM);
        }
    }
}

/// The place's point under the pointer (and who stands there, where), said
/// at the head's right; that point, rounded.
fn pointed(ui: &egui::Ui, drawn: &Frame, place: &Place, view: &View, head: Rect, at: Pos2) -> [f32; 2] {
    let point = place.to_place(at).map(f32::round);
    let mut said = format!("{} {}", point[0], point[1]);
    if let Some(name) = view.body_at(&drawn.bodies, at)
        && let Some([x, y]) = drawn.feet.get(name)
    {
        said = format!("{name} at {x} {y}   ·   {said}");
    }
    ui.painter().text(head.right_center() - vec2(10.0, 0.0), egui::Align2::RIGHT_CENTER, &said, FontId::monospace(theme::SMALL + 1.0), Color32::WHITE);
    point
}

/// What the mouse holds: someone, where they would go; a measure.
fn held(painter: &Painter, films: &Films, drawn: &Frame, place: &Place, view: &View, at: Option<Pos2>) {
    match (&films.held, at) {
        (Some(Held::Someone(name, offset)), Some(at)) => {
            let to = place.to_place(at);
            let feet = [(to[0] + offset[0]).round(), (to[1] + offset[1]).round()];
            if let Some((_, body)) = drawn.bodies.iter().find(|(known, _)| known == name)
                && let Some(from) = drawn.feet.get(name)
            {
                let shift = place.to_screen(feet) - place.to_screen(*from);
                let frame = view.frame_of(*body).translate(shift);
                painter.rect_stroke(frame, 0.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Outside);
                painter.line_segment([place.to_screen(*from), place.to_screen(feet)], Stroke::new(1.0_f32, Color32::from_white_alpha(140)));
            }
            painter.circle_filled(place.to_screen(feet), 3.0, Color32::WHITE);
            crate::decors::tag(painter, place.to_screen(feet) + vec2(6.0, 4.0), &format!("{name} {} {}", feet[0], feet[1]), Color32::WHITE);
        }
        (Some(Held::Measure(from)), Some(at)) => {
            let to = place.to_place(at);
            let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
            let far = (dx * dx + dy * dy).sqrt();
            painter.line_segment([place.to_screen(*from), at], Stroke::new(1.5_f32, theme::FILM));
            painter.circle_filled(place.to_screen(*from), 2.5, theme::FILM);
            crate::decors::tag(painter, at + vec2(8.0, 4.0), &format!("{:.0} px · {:.1} tiles  ({:+.0} {:+.0})", far, far / 8.0, dx, dy), theme::FILM);
        }
        _ => {}
    }
}
