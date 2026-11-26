//! What the mouse does on the film's picture: a field of the Inspector given
//! a point, someone dragged (their line in the Script written again), the
//! picture moved, a measure, a point or a name written (Ctrl+click), someone
//! chosen, a line written from the menu.

use eframe::egui::{self, FontId, Pos2, vec2};

use super::super::lines::with_field;
use super::super::{Films, Held};
use super::handles::{Handles, camera_moved};
use super::script::{faced, framed_at, moved_to, number, way_of, with_point};
use super::{Place, View};
use workshop_base::theme;
use workshop_script::Scripts;

/// What a measure from a point to another says.
pub(super) fn measure_said(from: [f32; 2], to: [f32; 2]) -> String {
    let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
    format!("from {} {} to {} {}: {:.0} px, {:.1} tiles", from[0], from[1], to[0], to[1], (dx * dx + dy * dy).sqrt(), (dx * dx + dy * dy).sqrt() / 8.0)
}

/// All the mouse does on the picture, in its order.
#[allow(clippy::too_many_arguments)]
pub(super) fn on_picture(
    ui: &egui::Ui,
    films: &mut Films,
    scripts: &mut Scripts,
    view: &View,
    picture: egui::Response,
    place_point: Option<[f32; 2]>,
    handles: &Handles,
) {
    let body_at = |films: &Films, at: Pos2| films.drawn.as_ref().and_then(|drawn| view.body_at(&drawn.bodies, at).cloned());
    let command = ui.input(|input| input.modifiers.command);
    picking(ui, films, scripts, view, &picture, place_point);
    // (Over a handle: the pointer says it can be taken.)
    if films.held.is_none()
        && films.picking.is_none()
        && let Some(at) = picture.hover_pos()
        && let Some(icon) = handles.cursor(at, body_at(films, at).is_some())
    {
        ui.ctx().set_cursor_icon(icon);
    }
    // (A drag: a handle, someone moved (their line in the Script written again), the picture moved, Maj: a measure.)
    if picture.drag_started()
        && let Some(at) = picture.interact_pointer_pos()
    {
        let shift = ui.input(|input| input.modifiers.shift);
        let middle = picture.dragged_by(egui::PointerButton::Middle);
        let handle = place_point.filter(|_| !shift && !middle && films.picking.is_none()).and_then(|point| handles.at(at, point, body_at(films, at).is_some()));
        films.held = Some(match (shift, middle, place_point, body_at(films, at)) {
            _ if handle.is_some() => handle.unwrap_or(Held::Picture),
            (true, false, Some(point), _) => Held::Measure(point),
            (false, false, Some(point), Some(name)) if films.drawn.as_ref().is_some_and(|drawn| drawn.feet.contains_key(&name)) => {
                let feet = films.drawn.as_ref().and_then(|drawn| drawn.feet.get(&name).copied()).unwrap_or(point);
                Held::Someone(name, [feet[0] - point[0], feet[1] - point[1]])
            }
            _ => Held::Picture,
        });
    }
    if picture.dragged() && matches!(films.held, Some(Held::Picture)) {
        films.pan += picture.drag_delta();
    }
    if picture.drag_stopped() {
        let at = ui.input(|input| input.pointer.latest_pos());
        let_go(films, scripts, view, handles, place_point, at);
    }
    // (A click on one of the dots around the one chosen: they face that way.)
    if picture.clicked()
        && films.picking.is_none()
        && !command
        && let Some((who, way)) = picture.interact_pointer_pos().and_then(|at| handles.way_clicked(at))
    {
        facing(films, scripts, &who, way);
        return;
    }
    // (Ctrl+clic on the picture: its point written in the Script, or the name of who is there.)
    if picture.clicked()
        && films.picking.is_none()
        && command
        && let (Some(point), Some(at), Some(open)) = (place_point, picture.interact_pointer_pos(), scripts.open.as_mut())
    {
        let words = body_at(films, at).unwrap_or_else(|| format!("{} {}", point[0], point[1]));
        open.insert(&words);
        films.said = Some(format!("written in the Script: {words}"));
    }
    // (A click on someone: they are chosen (their lines brought out in the Timeline, the State), or no longer.)
    let chosen = (picture.clicked() && films.picking.is_none() && !command).then(|| picture.interact_pointer_pos().and_then(|at| body_at(films, at))).flatten();
    if let Some(point) = place_point {
        let who = picture.hover_pos().and_then(|at| body_at(films, at));
        menu(&picture, scripts, point, who);
    }
    if let Some(name) = chosen {
        films.choose(name);
    }
    if films.held.is_none() {
        picture.on_hover_text(
            "drag someone: their line rewritten · click someone: chosen, their way (dots) and walks (points) to drag · Whole Place: drag the camera's frame, a corner zooms · drag a spot · drag elsewhere: the image · Shift+drag: measure · wheel: closer · Ctrl+click: this point written · right-click: a line",
        );
    }
}

/// A field of the Inspector being chosen here: a click gives it this point (Esc: no).
fn picking(ui: &egui::Ui, films: &mut Films, scripts: &mut Scripts, view: &View, picture: &egui::Response, place_point: Option<[f32; 2]>) {
    let Some((line, name)) = films.picking.clone() else {
        return;
    };
    ui.painter().text(
        view.shown.center_top() + vec2(0.0, 8.0),
        egui::Align2::CENTER_TOP,
        format!("a click: \"{name}\" of line {line}  (Esc: no)"),
        FontId::proportional(theme::SMALL),
        theme::FILM,
    );
    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        films.picking = None;
    }
    if picture.clicked()
        && let (Some(point), Some(open), Some(read)) = (place_point, scripts.open.as_mut(), films.fields.get(&line))
    {
        let words = vec![format!("{}", point[0]), format!("{}", point[1])];
        match with_field(&open.text, line, read, &name, &words) {
            Ok(made) => open.set_text(made),
            Err(why) => films.said = Some(why),
        }
        films.picking = None;
    }
}

/// Someone facing that way: their line written again (`faced`), said.
fn facing(films: &mut Films, scripts: &mut Scripts, who: &str, way: &str) {
    if let Some(open) = scripts.open.as_mut() {
        let (made, line) = faced(&open.text, films.line_here(open.cursor_line()), who, way);
        open.set_text(made);
        films.said = Some(format!("line {line}: {who} facing {way}"));
    }
}

/// What the mouse held, let go (the pointer there): someone put there
/// (their line written again), a handle's line written again (the camera's
/// framing, someone's way, a walk's or a spot's point), a measure said.
fn let_go(films: &mut Films, scripts: &mut Scripts, view: &View, handles: &Handles, place_point: Option<[f32; 2]>, at: Option<Pos2>) {
    let place = films.drawn.as_ref().map(|drawn| Place::of(view, drawn));
    let handled = place.zip(at).map(|(place, at)| (place, at, place.to_place(at).map(f32::round)));
    match films.held.take() {
        Some(Held::Camera { from, corner }) => {
            if let (Some((place, at, _)), Some(camera), Some(open), Some(drawn)) = (handled, handles.camera, scripts.open.as_mut(), films.drawn.as_ref()) {
                let moved = camera_moved(camera, from, &place, at, corner);
                let centre = place.to_place(moved.center()).map(f32::round);
                let close = corner.then(|| (drawn.state.camera.zoom * camera.width() / moved.width().max(1.0)).clamp(0.25, 4.0));
                let (made, line) = framed_at(&open.text, films.line_here(open.cursor_line()), centre, close);
                open.set_text(made);
                let zoom = close.map(|close| format!(", zoom {}", number(close))).unwrap_or_default();
                films.said = Some(format!("line {line}: the camera on {} {}{zoom}", centre[0], centre[1]));
            }
        }
        Some(Held::Facing(who)) => {
            if let (Some((_, at, _)), Some((_, feet, _))) = (handled, handles.facing.as_ref())
                && feet.distance(at) > 4.0
            {
                let way = way_of(at.x - feet.x, at.y - feet.y);
                facing(films, scripts, &who, way);
            }
        }
        Some(Held::Point(line, word)) => {
            if let (Some((_, _, point)), Some(open)) = (handled, scripts.open.as_mut()) {
                open.set_text(with_point(&open.text, line, word, point));
                films.said = Some(format!("line {line}: the point made {} {}", point[0], point[1]));
            }
        }
        Some(Held::Someone(name, offset)) => {
            if let (Some(point), Some(open)) = (place_point.or(handled.map(|(_, _, point)| point)), scripts.open.as_mut()) {
                let feet = [(point[0] + offset[0]).round(), (point[1] + offset[1]).round()];
                let playing = films.line_here(open.cursor_line());
                match moved_to(&open.text, playing, &name, feet) {
                    Ok((made, line)) => {
                        open.set_text(made);
                        films.said = Some(format!("line {line}: {name} at {} {}", feet[0], feet[1]));
                    }
                    Err(why) => films.said = Some(why),
                }
            }
        }
        Some(Held::Measure(from)) => {
            if let Some(to) = place_point {
                films.said = Some(measure_said(from, to));
            }
        }
        _ => {}
    }
}

/// The right click's menu: lines with this point (and who is there) written in the Script.
fn menu(picture: &egui::Response, scripts: &mut Scripts, point: [f32; 2], who: Option<String>) {
    let picture_menu = picture.clone();
    picture_menu.context_menu(|ui| {
        let [x, y] = point;
        let name = who.clone().unwrap_or_else(|| "who".to_string());
        for line in
            [format!("{x} {y}"), format!("{name} put {x} {y}"), format!("{name} walk to {x} {y}"), format!("spot here {x} {y}"), format!("frame {x} {y} snap")]
        {
            if ui.button(format!("Write \"{line}\"")).clicked() {
                if let Some(open) = scripts.open.as_mut() {
                    open.insert(&line);
                }
                ui.close();
            }
        }
        if let Some(who) = &who
            && ui.button(format!("Write \"{who}\"")).clicked()
        {
            if let Some(open) = scripts.open.as_mut() {
                open.insert(who);
            }
            ui.close();
        }
    });
}
