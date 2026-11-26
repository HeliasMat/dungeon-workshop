//! The Scene of the Decor space: the decor drawn piece by piece in the
//! order of its lines (the terrains and grounds, the pieces' lower layers,
//! those who live there, the upper layers), its walls and landmarks over it
//! if their layers are shown, its grid; a map of all of it when it does not
//! fit. With the tool Choisir: things chosen by a click (Ctrl or Maj: one
//! more), by a rectangle drawn over the empty, carried to the pixel (Alt: on
//! the grid of 8), moved by the arrows, copied, cut, pasted, duplicated,
//! taken out. A marker tool (`markers`): a click on the empty lays one, its
//! window beside it (`window.rs`); those of its kind taken and carried as
//! with Choisir. The other tools: `terrains`.
//!
//! Here the view (how close, where) and the order of it all; `drawn.rs` what
//! is drawn, `marks.rs` what is drawn over it (what is chosen, measures,
//! rulers), `moving.rs` the view moved (the map, the wheel, the keys, its
//! bar), `choosing.rs` the tool Choisir.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Vec2, vec2};

use crate::edit::{self, Item};
use crate::placing::grid_step;
use crate::terrains::Tool;
use crate::{Carry, Decors, Drawn};
use workshop_base::theme;

mod choosing;
mod drawn;
mod hints;
mod marks;
mod moving;
#[cfg(test)]
mod tests;
mod window;

/// A thing of the decor: its line, its place on the line.
type Element = (usize, usize);

/// How the decor is seen: the room of the Scene, where the decor is drawn in
/// it, how close (and how close it would be to fit), its size in pixels.
#[derive(Clone, Copy)]
struct View {
    rect: Rect,
    shown: Rect,
    scale: f32,
    fit: f32,
    size: Vec2,
}

/// How close a decor of that size is seen to fit in that room (whole steps when larger than its pixels).
fn fit_scale(room: Vec2, size: Vec2) -> f32 {
    let fit = (room.x / size.x).min(room.y / size.y) * 0.96;
    if fit >= 1.0 { fit.floor() } else { fit }
}

impl View {
    /// The view of a decor of that size in that room, at that closeness (none: to fit), moved by `pan`.
    fn of(rect: Rect, size: Vec2, scale: Option<f32>, pan: Vec2) -> View {
        let fit = fit_scale(rect.size(), size);
        let scale = scale.unwrap_or(fit);
        View { rect, shown: Rect::from_center_size(rect.center() + pan, size * scale), scale, fit, size }
    }

    /// A point of the decor, on the screen.
    fn to_screen(self, point: [f32; 2]) -> Pos2 {
        self.shown.min + vec2(point[0], point[1]) * self.scale
    }

    /// A point of the screen, in the decor.
    fn to_place(self, at: Pos2) -> [f32; 2] {
        let point = (at - self.shown.min) / self.scale;
        [point.x, point.y]
    }

    /// An area of the decor, on the screen.
    fn on_screen(self, area: Rect) -> Rect {
        Rect::from_min_max(self.to_screen([area.min.x, area.min.y]), self.to_screen([area.max.x, area.max.y]))
    }

    /// What is seen of the decor, on the screen.
    fn seen(self) -> Rect {
        self.shown.intersect(self.rect)
    }

    /// A closeness kept within what the Scene allows.
    fn clamped(self, next: f32) -> f32 {
        next.clamp(self.fit.min(1.0) * 0.5, 16.0)
    }
}

/// What the Scene reads of the decor once: its text, its things (those
/// chosen moved by what is carried) and their boxes, what is chosen,
/// carried, the rectangle being drawn; the layers hidden, locked; the tool in hand.
struct Read {
    text: String,
    items: Vec<Item>,
    boxes: Vec<Rect>,
    selection: Vec<Element>,
    carry: Option<Carry>,
    marquee: Option<[f32; 2]>,
    hidden: std::collections::HashSet<&'static str>,
    locked: std::collections::HashSet<&'static str>,
    tool: Tool,
}

impl Read {
    /// Whether a thing can be taken: not locked, nor on a layer locked or hidden.
    fn takeable(&self, one: &Item) -> bool {
        !one.locked && !self.locked.contains(one.layer) && !self.hidden.contains(one.layer)
    }

    /// The box of those things together.
    fn box_of(&self, chosen: &[Element]) -> Option<Rect> {
        chosen.iter().filter_map(|key| self.items.iter().position(|one| (one.line, one.at) == *key)).map(|at| self.boxes[at]).reduce(|a, b| a.union(b))
    }
}

/// The things of the decor as the Scene draws them: those chosen where they are carried to.
fn items_carried(text: &str, selection: &[Element], carry: Option<Carry>) -> Vec<Item> {
    let mut items = edit::items(text);
    if let Some(carry) = carry {
        for item in items.iter_mut().filter(|item| selection.contains(&(item.line, item.at))) {
            item.corner = [item.corner[0] + carry.by[0], item.corner[1] + carry.by[1]];
        }
    }
    items
}

/// The Scene: the decor drawn, what is over it, the view moved; then the tool in hand.
pub fn scene(ui: &mut egui::Ui, decors: &mut Decors) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(0x30, 0x30, 0x30));
    let Some(open) = &decors.open else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No decor open: File › Open, or the Home",
            egui::FontId::proportional(theme::BODY),
            theme::WEAK,
        );
        return;
    };
    let view = View::of(rect, Vec2::from(open.size()), open.scale, open.pan);
    painter.rect_filled(view.shown, 0.0, Color32::BLACK);
    let seconds = if decors.living { ui.input(|input| input.time) } else { 0.0 };
    let (selection, carry, marquee, text) = (open.selection(), open.carried, open.marquee, open.text.clone());
    let items = items_carried(&text, &selection, carry);
    // (A marker tool in hand: the residents and the markers shown, to take and move.)
    let marker = decors.brushes.tool.marker();
    let mut read = Read {
        hidden: edit::LAYERS
            .iter()
            .map(|(word, _)| *word)
            .filter(|word| decors.is_hidden(word) && !(marker.is_some() && matches!(*word, "residents" | "markers")))
            .collect(),
        locked: decors.locked.clone(),
        tool: decors.brushes.tool,
        boxes: Vec::new(),
        text,
        items,
        selection,
        carry,
        marquee,
    };
    let things = drawn::things(decors, &read, view.size, seconds);
    let bodies_asked = drawn::draw(&painter, decors, &read, &things, view);
    for name in &things.asked {
        decors.art_now(name);
    }
    for word in &bodies_asked {
        decors.body_now(word);
    }
    read.boxes = read.items.iter().map(|one| decors.item_box(one)).collect();
    let choosing = read.tool == Tool::Choose || marker.is_some();
    let pointer = response.hover_pos();
    // (A marker tool takes only its own kind: elsewhere, a click lays one.)
    let under = pointer.filter(|_| choosing).and_then(|at| {
        (0..read.items.len()).rev().find(|at_one| {
            let one = &read.items[*at_one];
            read.takeable(one) && marker.is_none_or(|marker| one.kind == marker.kind()) && view.on_screen(read.boxes[*at_one]).contains(at)
        })
    });
    let together = marks::chosen_framed(&painter, decors, &read, view, under, response.interact_pointer_pos());
    if ui.input(|input| input.modifiers.alt) && read.carry.is_none() {
        marks::measured(&painter, &read, view, together, pointer);
    }
    marks::around(&painter, view);
    if decors.rulers {
        marks::rulers(&painter, view, pointer);
    }
    let mut camera = open_view(decors);
    let space = moving::moved(ui, &response, &painter, view, &things, together, pointer, &mut camera);
    let middle = view.to_place(rect.center());
    // (Ctrl+F over the scene: the Library's search.)
    if response.hovered() && ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::F)) {
        decors.lib.focus = true;
    }
    if let Some(marker) = marker {
        marks::handles(&painter, decors, &read, view, marker, under);
    }
    if choosing {
        choosing::choose(ui, decors, &response, &painter, view, &read, under, space, middle, camera);
    } else {
        let open = decors.open.as_mut().expect("open");
        (open.scale, open.pan) = camera;
        open.seen_scale = camera.0.unwrap_or(view.fit);
        open.middle = middle;
        open.view = rect.size();
        crate::terrains::interact(ui, decors, &response, &painter, &|point| view.to_screen(point), &|at| view.to_place(at), view.scale);
    }
    // (The hint bar, under the rulers: what is on, the keys of the tool in hand.)
    let chosen = decors.open.as_ref().is_some_and(|open| open.chosen.is_some());
    let now = hints::Now { tool: decors.brushes.tool, stamping: decors.stamp.is_some() && read.tool == Tool::Choose, chosen };
    let below_rulers = if decors.rulers { Rect::from_min_max(rect.min + vec2(18.0, 19.0), rect.max) } else { rect };
    hints::bar(&painter, below_rulers, decors, now, grid_step(&read.text));
    // (The window beside the marker laid or taken: its essentials.)
    if let Some(number) = decors.marker_window {
        let items = decors.open.as_ref().map(|open| edit::items(&open.text)).unwrap_or_default();
        match items.iter().find(|one| one.line == number) {
            Some(one) => {
                let place = view.on_screen(decors.item_box(one));
                window::window(ui, decors, number, place.right_top() + vec2(12.0, 0.0));
            }
            None => decors.marker_window = None,
        }
    }
}

/// How close the decor open is seen, and how far moved.
fn open_view(decors: &Decors) -> (Option<f32>, Vec2) {
    decors.open.as_ref().map_or((None, Vec2::ZERO), |open| (open.scale, open.pan))
}
