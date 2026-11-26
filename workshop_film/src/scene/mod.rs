//! The Scene panel: the frame as the game shows it (the staging view: its
//! screen's effects off), the bar that plays it, someone dragged to a new
//! spot, the handles that write the film's lines.
//!
//! Here the order of its parts and the picture's view (where, how grown);
//! `marks.rs` what is drawn over the picture (the grid, the camera, who is
//! there, the spots, what the mouse holds; the names kept clear of each
//! other), `handles.rs` the handles (the camera's frame, someone's way,
//! their walks, the spots), `mouse.rs` what the mouse does on it (a field's
//! point, someone moved, a handle let go, a measure, a line written, the
//! menu), `drop.rs` an asset of the Assets panel dropped on it, `bar.rs`
//! the line played under it, the bar and the keys, `script.rs` the film's
//! text read and written again (someone moved to a point, the view framed, a
//! way, a walk's point, the spots, someone dropped there).

use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, Sense, Vec2, pos2, vec2};

use super::{Films, Frame, SECOND};
use workshop_base::theme;
use workshop_script::Scripts;

mod bar;
mod drop;
mod handles;
mod marks;
mod mouse;
mod script;
#[cfg(test)]
mod tests;

pub(super) use bar::toggle;

/// The bar's height under the picture, the played line's over it.
const BAR_HEIGHT: f32 = 64.0;
const SUBTITLE: f32 = 22.0;

/// The picture on the panel: where it is shown whole, how many of the
/// panel's points a pixel of it takes, the part of it seen.
#[derive(Clone, Copy)]
struct View {
    shown: Rect,
    scale: f32,
    seen: Rect,
}

impl View {
    /// A box of the picture's pixels, on the panel.
    fn frame_of(&self, [x0, y0, x1, y1]: [f32; 4]) -> Rect {
        Rect::from_min_max(self.shown.min + vec2(x0, y0) * self.scale, self.shown.min + vec2(x1, y1) * self.scale)
    }

    /// Who is at that point of the panel (the one drawn last, over the others).
    fn body_at<'a>(&self, bodies: &'a [(String, [f32; 4])], at: Pos2) -> Option<&'a String> {
        bodies.iter().rev().find(|(_, frame)| self.frame_of(*frame).contains(at)).map(|(name, _)| name)
    }
}

/// How the place lies on the panel: a pixel of the picture a point of the place, and back.
#[derive(Clone, Copy)]
struct Place {
    min: Pos2,
    scale: f32,
    centre: [f32; 2],
    close: f32,
    wide: f32,
    high: f32,
}

impl Place {
    fn of(view: &View, drawn: &Frame) -> Place {
        Place {
            min: view.shown.min,
            scale: view.scale,
            centre: drawn.centre,
            close: drawn.close,
            wide: (drawn.wide / drawn.scale) as f32,
            high: (drawn.high / drawn.scale) as f32,
        }
    }

    fn to_place(self, at: Pos2) -> [f32; 2] {
        let pixel = (at - self.min) / self.scale;
        [(pixel.x - self.wide / 2.0) / self.close + self.centre[0], (pixel.y - self.high / 2.0) / self.close + self.centre[1]]
    }

    fn to_screen(self, point: [f32; 2]) -> Pos2 {
        self.min + vec2((point[0] - self.centre[0]) * self.close + self.wide / 2.0, (point[1] - self.centre[1]) * self.close + self.high / 2.0) * self.scale
    }
}

pub fn scene(ui: &mut egui::Ui, films: &mut Films, scripts: &mut Scripts) {
    let full = ui.available_rect_before_wrap();
    ui.painter().rect_filled(full, 0.0, Color32::from_rgb(0x14, 0x14, 0x14));
    if scripts.open.is_none() {
        ui.painter().text(full.center(), egui::Align2::CENTER_CENTER, "No film open", FontId::proportional(theme::BODY), theme::WEAK);
        return;
    }
    scripts.lit_who_scene = None;
    let (head, capture) = head(ui, films, &mut scripts.ds_preview, full);
    let room = Rect::from_min_max(pos2(full.min.x + 8.0, head.max.y + 8.0), pos2(full.max.x - 8.0, full.max.y - BAR_HEIGHT - SUBTITLE));
    let view = view(ui, films, room);
    let picture = ui.interact(view.seen, ui.id().with("film image"), Sense::click_and_drag());
    if picture.double_clicked() {
        (films.look, films.pan) = (1.0, Vec2::ZERO);
    }
    let handles = films
        .drawn
        .as_ref()
        .zip(scripts.open.as_ref())
        .map_or_else(handles::Handles::default, |(drawn, open)| handles::handles(films, &open.text, films.line_here(open.cursor_line()), drawn, &view));
    let place_point = picture_drawn(ui, films, scripts, &view, room, head, &picture, &handles);
    if let Some((line, said)) = films.wrong.first() {
        let band = Rect::from_min_size(room.min, vec2(room.width(), 22.0));
        ui.painter().rect_filled(band, 0.0, Color32::from_rgb(0x5A, 0x1D, 0x1D));
        ui.painter().text(
            band.left_center() + vec2(8.0, 0.0),
            egui::Align2::LEFT_CENTER,
            format!("line {line}: {said}"),
            FontId::proportional(theme::SMALL),
            Color32::WHITE,
        );
    }
    mouse::on_picture(ui, films, scripts, &view, picture, place_point, &handles);
    drop::dropping(ui, films, scripts, &view);
    if capture && let Some(drawn) = &films.drawn {
        films.said = Some(written_out(drawn, scripts));
    }
    let under = Rect::from_min_size(pos2(full.min.x + 10.0, full.max.y - BAR_HEIGHT - SUBTITLE), vec2(full.width() - 20.0, SUBTITLE));
    bar::line_played(ui, films, scripts, under);
    bar::bar(ui, films, Rect::from_min_max(pos2(full.min.x, full.max.y - BAR_HEIGHT), full.max));
    bar::keys(ui, films, full);
    ui.allocate_rect(full, Sense::hover());
}

/// Over the picture: what is shown of it; its place, whether it is asked written out.
fn head(ui: &mut egui::Ui, films: &mut Films, ds_preview: &mut bool, full: Rect) -> (Rect, bool) {
    let head = Rect::from_min_size(full.min, vec2(full.width(), 30.0));
    ui.painter().rect_filled(head, 0.0, theme::TOOLS);
    let mut top = ui.new_child(egui::UiBuilder::new().max_rect(head.shrink2(vec2(8.0, 3.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    let before_whole = films.whole;
    top.selectable_value(&mut films.whole, false, "Film View").on_hover_text("what the camera shows");
    top.selectable_value(&mut films.whole, true, "Whole Place").on_hover_text("the whole place, the camera's frame on it");
    top.separator();
    top.checkbox(&mut films.names, "Names").on_hover_text("each one, its frame and its name");
    top.checkbox(&mut films.grid, "Grid").on_hover_text("the place's tiles of 8 pixels");
    let capture = top.button("Image").on_hover_text("this image written as a PNG (target/films/<film>/)").clicked();
    top.separator();
    top.checkbox(&mut films.looped, "Loop").on_hover_text("played again from the start at the end");
    top.checkbox(&mut films.follow, "Follow the Script").on_hover_text("the film goes where the Script's cursor is");
    top.separator();
    top.checkbox(ds_preview, "DS Preview").on_hover_text(
        "the film as the DS would show it: what it leaves out (items, a zoom, a word it lacks) not played, the bubbles said in the box (the DS Report: what each line loses)",
    );
    if films.staged && !films.playing {
        top.separator();
        let staging = top
            .selectable_label(true, "Staging")
            .on_hover_text("the staging view: the scene as it stands at the Script's line, the screen's effects off (fades, title, iris…); a click shows them");
        if staging.clicked() {
            films.staged = false;
        }
    }
    if films.look != 1.0 || films.pan != Vec2::ZERO {
        top.separator();
        top.label(RichText::new(format!("×{:.1}", films.look)).color(theme::WEAK));
        if top.button("Fit").on_hover_text("the whole image in the panel (double-click on the image)").clicked() {
            (films.look, films.pan) = (1.0, Vec2::ZERO);
        }
    }
    if films.whole != before_whole {
        films.asked = None;
        (films.look, films.pan) = (1.0, Vec2::ZERO);
    }
    (head, capture)
}

/// How much a picture of that size is grown to fit the room: by whole
/// pixels when it fits so.
fn fitted_scale(room: Vec2, size: Vec2) -> f32 {
    let fit = (room.x / size.x).min(room.y / size.y);
    if fit >= 1.0 { fit.floor() } else { fit }
}

/// The look closer (the wheel up) or farther, around the pointer: how close, the picture's shift.
fn wheel_look(look: f32, pan: Vec2, wheel: f32, at: Pos2, middle: Pos2) -> (f32, Vec2) {
    let closer = (look * if wheel > 0.0 { 1.25 } else { 0.8 }).clamp(0.5, 16.0);
    let centre = middle + pan;
    let moved = at - (at - centre) * (closer / look);
    (closer, moved - middle)
}

/// The picture's view in the room, the wheel looking closer around the pointer.
fn view(ui: &egui::Ui, films: &mut Films, room: Rect) -> View {
    let size = films.picture_size;
    let fitted = fitted_scale(room.size(), size);
    if let Some(at) = ui.input(|input| input.pointer.hover_pos()).filter(|at| room.contains(*at)) {
        let wheel = ui.input(|input| input.raw_scroll_delta.y);
        if wheel != 0.0 {
            (films.look, films.pan) = wheel_look(films.look, films.pan, wheel, at, room.center());
        }
    }
    let scale = fitted * films.look;
    let shown = Rect::from_center_size(room.center() + films.pan, size * scale);
    View { shown, scale, seen: shown.intersect(room) }
}

/// The picture, its marks, whether it is old; the place's point under the pointer.
#[allow(clippy::too_many_arguments)]
fn picture_drawn(
    ui: &egui::Ui,
    films: &Films,
    scripts: &mut Scripts,
    view: &View,
    room: Rect,
    head: Rect,
    picture: &egui::Response,
    handles: &handles::Handles,
) -> Option<[f32; 2]> {
    let Some(texture) = &films.texture else {
        let said = films.failed.clone().unwrap_or_else(|| "the film is playing…".to_string());
        ui.painter().text(room.center(), egui::Align2::CENTER_CENTER, said, FontId::proportional(theme::BODY), theme::WEAK);
        return None;
    };
    ui.painter_at(room).image(texture.id(), view.shown, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
    let place_point = films.drawn.as_ref().and_then(|drawn| marks::over(ui, films, scripts, drawn, view, head, picture, handles));
    if films.shows_staged() {
        workshop_decor::tag(&ui.painter_at(room), view.seen.left_bottom() + vec2(4.0, -4.0), "staging · effects off", theme::FILM);
    }
    if !films.fresh(scripts) {
        ui.painter().text(
            view.shown.right_top() + vec2(-6.0, 6.0),
            egui::Align2::RIGHT_TOP,
            "the film plays again…",
            FontId::proportional(theme::SMALL),
            Color32::WHITE,
        );
    }
    place_point
}

/// The picture written out as a PNG, in `target/films/<film>/`; said.
fn written_out(drawn: &Frame, scripts: &Scripts) -> String {
    let stem =
        scripts.open.as_ref().and_then(|open| open.path.file_stem().map(|stem| stem.to_string_lossy().into_owned())).unwrap_or_else(|| "film".to_string());
    let folder = scripts.session.root().join("target").join("films").join(&stem);
    let path = folder.join(format!("image_{:.1}s.png", drawn.frame as f64 / SECOND));
    let made = std::fs::create_dir_all(&folder).map_err(|why| why.to_string()).and_then(|()| {
        image::RgbaImage::from_raw(drawn.wide as u32, drawn.high as u32, drawn.rgba.clone())
            .ok_or_else(|| "wrong size".to_string())?
            .save(&path)
            .map_err(|why| why.to_string())
    });
    match made {
        Ok(()) => format!("image written: {}", path.display()),
        Err(why) => why,
    }
}
