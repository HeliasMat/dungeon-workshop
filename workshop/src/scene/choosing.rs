//! The tool Choisir on the Scene: things taken by a drag (with all that is
//! chosen, put on the guides of others) or a rectangle drawn; chosen by a
//! click (Ctrl or Maj: one more); a piece of the Library dropped; the right
//! button's menu; the keys (arrows, Suppr, Ctrl+D, Ctrl+A, Ctrl+C, Ctrl+X,
//! Ctrl+V, Esc). What is asked, done at the end.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2, pos2, vec2};

use super::moving::Camera;
use super::{Element, Read, View};
use crate::decors::{Carry, Decors, tag};
use crate::edit::{self, Kind};
use workshop_base::theme;

/// What the tool asks, done once all is seen: the text made, what to
/// choose, what is carried, the rectangle being drawn, the view.
struct Asked {
    made: Option<String>,
    chose: Option<Vec<Element>>,
    carry: Option<Carry>,
    marquee: Option<[f32; 2]>,
    camera: Camera,
}

/// How what is laid or carried lands: on the grid of that step, or on the pixel.
#[derive(Clone, Copy)]
pub(super) struct Snap {
    pub step: f32,
    pub on: bool,
}

impl Snap {
    /// A point of the decor where it lands.
    pub(super) fn of(self, point: [f32; 2]) -> [f32; 2] {
        crate::placing::snap(point, self.step, self.on)
    }
}

/// The nearest guide for a box being carried: an edge or a middle of it near
/// one of another's (within `reach`), across (x) or down (y): how far to go, and where the guide is.
pub(super) fn nearest_guide(moved: Rect, others: &[Rect], reach: f32, across: bool) -> Option<(f32, f32)> {
    let lines_of = |area: &Rect| if across { [area.min.x, area.center().x, area.max.x] } else { [area.min.y, area.center().y, area.max.y] };
    let mut best: Option<(f32, f32)> = None;
    for other in others {
        for theirs in lines_of(other) {
            for mine in lines_of(&moved) {
                let gap = theirs - mine;
                if gap.abs() < reach && best.is_none_or(|(known, _)| gap.abs() < known.abs()) {
                    best = Some((gap, theirs));
                }
            }
        }
    }
    best
}

/// What is chosen once a thing is clicked: it alone; with Ctrl or Maj, it added first, or taken out if it was chosen.
pub(super) fn chosen_on_click(selection: &[Element], key: Element, toggling: bool) -> Vec<Element> {
    if !toggling {
        return vec![key];
    }
    let mut now = selection.to_vec();
    match now.iter().position(|one| *one == key) {
        Some(found) => _ = now.remove(found),
        None => now.insert(0, key),
    }
    now
}

/// The first `spotN` name no spot has yet.
pub(super) fn new_spot_name(names: &[String]) -> String {
    (1..).map(|at| format!("spot{at}")).find(|name| !names.contains(name)).unwrap_or_default()
}

/// The tool Choisir, the pointer and the keys over the Scene; what it asked, done.
#[allow(clippy::too_many_arguments)]
pub(super) fn choose(
    ui: &egui::Ui,
    decors: &mut Decors,
    response: &egui::Response,
    painter: &egui::Painter,
    view: View,
    read: &Read,
    under: Option<usize>,
    space: bool,
    middle: [f32; 2],
    camera: Camera,
) {
    let mut asked = Asked { made: None, chose: None, carry: read.carry, marquee: read.marquee, camera };
    let (alt, toggling) = ui.input(|input| (input.modifiers.alt, input.modifiers.command || input.modifiers.shift));
    let snap = Snap { step: super::grid_step(&read.text), on: crate::placing::snapping(decors.snap, alt) };
    if response.drag_started_by(egui::PointerButton::Primary)
        && !space
        && let Some(pointed) = response.interact_pointer_pos()
    {
        drag_begun(read, view, under, pointed, toggling, &mut asked);
    }
    let chosen_now = asked.chose.clone().unwrap_or_else(|| read.selection.clone());
    if let Some(taken) = asked.carry.as_mut()
        && let Some(pointed) = response.interact_pointer_pos()
    {
        carried(painter, decors, read, view, &chosen_now, taken, pointed, snap);
    }
    if response.drag_stopped() {
        drag_ended(response, read, view, &chosen_now, toggling, &mut asked);
    }
    let stamping = decors.stamp.is_some();
    if stamping {
        stamp(decors, response, painter, read, view, snap, space, &mut asked);
    } else if response.clicked() {
        asked.chose = Some(match under {
            Some(at) => chosen_on_click(&read.selection, (read.items[at].line, read.items[at].at), toggling),
            None if toggling => read.selection.clone(),
            None => Vec::new(),
        });
    }
    dropped(decors, response, painter, read, view, snap, &mut asked);
    // (The right button: the stamp put down; else what may be done to what is under, or there.)
    if stamping {
        if response.secondary_clicked() {
            decors.stamp = None;
        }
    } else if response.secondary_clicked() {
        let point = response.hover_pos().map_or(middle, |at| view.to_place(at));
        let key = under.map(|at| (read.items[at].line, read.items[at].at));
        if let Some(key) = key
            && !read.selection.contains(&key)
        {
            asked.chose = Some(vec![key]);
        }
        decors.scene_menu = Some((key, point));
    }
    let menu_target = decors.scene_menu;
    let menu_chosen = asked.chose.clone().unwrap_or_else(|| read.selection.clone());
    if !stamping {
        response.context_menu(|ui| {
            let Some((key, point)) = menu_target else { return };
            match key {
                Some(_) => chosen_menu(ui, decors, read, &menu_chosen, &mut asked),
                None => empty_menu(ui, decors, read, point.map(f32::round), &mut asked),
            }
        });
    }
    if response.hovered() && !ui.ctx().wants_keyboard_input() {
        let here = response.hover_pos().map_or(middle, |at| view.to_place(at));
        keys(ui, decors, read, &chosen_now, here, snap, &mut asked);
    }
    done(ui, decors, response, view, middle, under, space, asked);
}

/// A drag begun: on a thing, it taken with all that is chosen (looked at
/// first); on the empty, a rectangle begun (what is chosen let go, unless Ctrl or Maj).
fn drag_begun(read: &Read, view: View, under: Option<usize>, pointed: Pos2, toggling: bool, asked: &mut Asked) {
    let selection = &read.selection;
    match under {
        Some(at) => {
            let key = (read.items[at].line, read.items[at].at);
            if !selection.contains(&key) {
                let mut now = if toggling { selection.clone() } else { Vec::new() };
                now.insert(0, key);
                asked.chose = Some(now);
            } else if selection.first() != Some(&key) {
                let mut now: Vec<Element> = selection.iter().copied().filter(|one| *one != key).collect();
                now.insert(0, key);
                asked.chose = Some(now);
            }
            asked.carry = Some(Carry { from: view.to_place(pointed), by: [0.0, 0.0] });
        }
        None => {
            asked.marquee = Some(view.to_place(pointed));
            if !toggling {
                asked.chose = Some(Vec::new());
            }
        }
    }
}

/// What is carried, following the pointer: the one looked at on the pixel
/// (or the grid), the others moved as much; put on a guide of another (or of
/// the decor) within a few points of the screen, the guide drawn; where it goes, said.
#[allow(clippy::too_many_arguments)]
fn carried(painter: &egui::Painter, decors: &Decors, read: &Read, view: View, chosen_now: &[Element], taken: &mut Carry, pointed: Pos2, snap: Snap) {
    let [px, py] = view.to_place(pointed);
    let still = edit::items(&read.text);
    let Some(first) = chosen_now.first().and_then(|key| still.iter().find(|one| (one.line, one.at) == *key)) else { return };
    let to = snap.of([first.corner[0] + px - taken.from[0], first.corner[1] + py - taken.from[1]]);
    taken.by = [to[0] - first.corner[0], to[1] - first.corner[1]];
    // (On the grid, no guides: the grid is the guide.)
    if !snap.on {
        let moving = still.iter().filter(|one| chosen_now.contains(&(one.line, one.at))).map(|one| decors.item_box(one)).reduce(|a, b| a.union(b));
        let mut others: Vec<Rect> =
            still.iter().filter(|one| !chosen_now.contains(&(one.line, one.at)) && !read.hidden.contains(one.layer)).map(|one| decors.item_box(one)).collect();
        others.push(Rect::from_min_size(Pos2::ZERO, view.size));
        if let Some(moving) = moving {
            let moved = moving.translate(vec2(taken.by[0], taken.by[1]));
            for across in [true, false] {
                let Some((gap, at)) = nearest_guide(moved, &others, 6.0 / view.scale, across) else { continue };
                let gap = gap.round();
                let seen = view.seen();
                let pink = Stroke::new(1.0_f32, Color32::from_rgb(0xFF, 0x4F, 0xD8));
                if across {
                    taken.by[0] += gap;
                    let x = view.to_screen([at, 0.0]).x;
                    painter.line_segment([pos2(x, seen.min.y), pos2(x, seen.max.y)], pink);
                } else {
                    taken.by[1] += gap;
                    let y = view.to_screen([0.0, at]).y;
                    painter.line_segment([pos2(seen.min.x, y), pos2(seen.max.x, y)], pink);
                }
            }
        }
    }
    let to = [first.corner[0] + taken.by[0], first.corner[1] + taken.by[1]];
    tag(painter, pointed + vec2(14.0, 0.0), &format!("{} {}", to[0], to[1]), Color32::WHITE);
}

/// A drag ended: what was carried moved there; the rectangle drawn, what it touches chosen (added to, with Ctrl or Maj).
fn drag_ended(response: &egui::Response, read: &Read, view: View, chosen_now: &[Element], toggling: bool, asked: &mut Asked) {
    if let Some(taken) = asked.carry.take()
        && taken.by != [0.0, 0.0]
    {
        asked.made = Some(edit::moved(&read.text, chosen_now, (taken.by[0], taken.by[1])));
    }
    if let (Some(start), Some(end)) = (asked.marquee.take(), response.interact_pointer_pos()) {
        let [x, y] = view.to_place(end);
        let area = Rect::from_two_pos(pos2(start[0], start[1]), pos2(x, y));
        let mut now = if toggling { chosen_now.to_vec() } else { Vec::new() };
        for (at, one) in read.items.iter().enumerate() {
            if read.takeable(one) && read.boxes[at].intersects(area) && !now.contains(&(one.line, one.at)) {
                now.push((one.line, one.at));
            }
        }
        asked.chose = Some(now);
    }
}

/// A card dragged over it: where it would land (the ghost: exactly there,
/// on the grid if snapping); let go: laid there, its middle under the
/// pointer, chosen (a model: its lines pasted there; a backdrop: the decor laid on it).
#[allow(clippy::too_many_arguments)]
fn dropped(decors: &mut Decors, response: &egui::Response, painter: &egui::Painter, read: &Read, view: View, snap: Snap, asked: &mut Asked) {
    if let Some(at) = response.hover_pos() {
        crate::library::ghost(decors, painter, response, &|point| view.to_screen(point), view.to_place(at), snap.step, snap.on);
    }
    let (Some(name), Some(at)) = (response.dnd_release_payload::<String>(), response.hover_pos()) else { return };
    let point = view.to_place(at);
    if let Some(level) = name.strip_prefix("backdrop:").and_then(|level| level.parse::<i64>().ok()) {
        let elements = crate::library::place_elements(decors, level);
        asked.made =
            crate::library::backdrop_text(&read.text, crate::library::BackdropAsked::Use { level, resized: false, bare: false }, &decors.backdrops, &elements);
        return;
    }
    match name.strip_prefix("model:") {
        Some(model) => {
            if let Some((_, lines)) = edit::models(decors.session.library()).into_iter().find(|(known, _)| known == model) {
                let (text, now) = edit::pasted(&read.text, &lines, snap.of(point));
                asked.made = Some(text);
                asked.chose = Some(now);
            }
        }
        None => laid_at(decors, read, &name, crate::placing::landing(point, decors.size_of(&name), snap.step, snap.on), asked),
    }
}

/// A piece laid with its corner there (one step), noted among those laid lately, chosen.
fn laid_at(decors: &mut Decors, read: &Read, name: &str, corner: [f32; 2], asked: &mut Asked) {
    crate::library::note_laid(decors, name);
    let line = decors.pose_line(name, &format!("{} {}", corner[0], corner[1]));
    let made = edit::with_line_added(&read.text, &line, false);
    let laid = edit::items(&made).into_iter().rev().find(|one| made.lines().nth(one.line - 1).is_some_and(|known| known.trim() == line.trim()));
    asked.chose = Some(laid.map(|one| vec![(one.line, one.at)]).unwrap_or_default());
    asked.made = Some(made);
}

/// The stamp in hand: its ghost under the pointer, exactly where it would
/// land; each click lays it there (a step each); Esc puts it down (`keys`).
#[allow(clippy::too_many_arguments)]
fn stamp(decors: &mut Decors, response: &egui::Response, painter: &egui::Painter, read: &Read, view: View, snap: Snap, space: bool, asked: &mut Asked) {
    let Some(stamp) = decors.stamp.clone() else { return };
    let Some(at) = response.hover_pos().filter(|_| response.dnd_hover_payload::<String>().is_none() && !space) else { return };
    let corner = crate::placing::landing(view.to_place(at), decors.size_of(&stamp.name), snap.step, snap.on);
    crate::library::paint_ghost(decors, painter, &stamp.name, corner, &|point| view.to_screen(point), "click: here · ");
    if response.clicked() {
        laid_at(decors, read, &stamp.name, corner, asked);
        decors.stamp = Some(crate::placing::stamped(&stamp));
    }
}

/// The menu of what is chosen: copied, doubled, brought to front or sent back, on a layer, grouped, removed.
fn chosen_menu(ui: &mut egui::Ui, decors: &mut Decors, read: &Read, menu_chosen: &[Element], asked: &mut Asked) {
    let text = &read.text;
    let mut lines: Vec<usize> = menu_chosen.iter().map(|(line, _)| *line).collect();
    lines.sort_unstable();
    lines.dedup();
    if ui.button("Copy").clicked() {
        decors.clipboard = edit::copied(text, menu_chosen);
        ui.ctx().copy_text(decors.clipboard.join("\n"));
        ui.close();
    }
    if ui.button("Duplicate").clicked() {
        if let Some(area) = read.box_of(menu_chosen) {
            let (pasted, now) = edit::pasted(text, &edit::copied(text, menu_chosen), [area.min.x + area.width(), area.min.y]);
            asked.made = Some(pasted);
            asked.chose = Some(now);
        }
        ui.close();
    }
    if lines.len() == 1 {
        ui.separator();
        if ui.button("Bring to Front").clicked() {
            asked.made = edit::to_front(text, lines[0], true);
            ui.close();
        }
        if ui.button("Send to Back").clicked() {
            asked.made = edit::to_front(text, lines[0], false);
            ui.close();
        }
    }
    ui.menu_button("Layer", |ui| {
        for (word, name) in [("ground", "Ground"), ("objects", "Objects"), ("over", "Over")] {
            if ui.button(name).clicked() {
                let mut now = text.clone();
                for line in &lines {
                    if let Some(moved) = edit::on_layer(&now, *line, word) {
                        now = moved;
                    }
                }
                asked.made = Some(now);
                ui.close();
            }
        }
    });
    if lines.len() > 1 && ui.button("Group").clicked() {
        let groups = edit::groups(text);
        let name = (1..).map(|at| format!("group{at}")).find(|name| !groups.iter().any(|(known, _, _)| known == name)).unwrap_or_default();
        if let Ok(grouped) = edit::grouped(text, &lines, &name) {
            asked.made = Some(grouped);
        }
        ui.close();
    }
    ui.separator();
    if ui.button(egui::RichText::new("Remove").color(theme::WRONG)).clicked() {
        asked.made = Some(edit::removed(text, menu_chosen));
        asked.chose = Some(Vec::new());
        ui.close();
    }
}

/// The menu of an empty place: what was copied pasted there; a spot, an
/// entrance, a resident, the piece chosen put there; the whole decor seen.
fn empty_menu(ui: &mut egui::Ui, decors: &mut Decors, read: &Read, [x, y]: [f32; 2], asked: &mut Asked) {
    let text = &read.text;
    if !decors.clipboard.is_empty() && ui.button("Paste Here").clicked() {
        let (pasted, now) = edit::pasted(text, &decors.clipboard, [x, y]);
        asked.made = Some(pasted);
        asked.chose = Some(now);
        ui.close();
    }
    let mut added: Option<String> = None;
    if ui.button("A Spot Here").clicked() {
        let names: Vec<String> = read.items.iter().filter(|one| one.kind == Kind::Spot).map(|one| one.name.clone()).collect();
        added = Some(format!("spot {} {x} {y}", new_spot_name(&names)));
    }
    if ui.button("An Entrance Here").clicked() {
        added = Some(format!("entrance {x} {y}"));
    }
    if ui.button(format!("A Resident Here ({})", decors.resident.trim())).clicked() && !decors.resident.trim().is_empty() {
        added = Some(format!("resident {} {x} {y}", decors.resident.trim()));
    }
    if let Some(chosen_piece) = decors.lib.chosen.clone()
        && ui.button(format!("Put \"{}\" Here", decors.shown_name(&chosen_piece))).clicked()
    {
        let [w, h] = decors.size_of(&chosen_piece);
        added = Some(decors.pose_line(&chosen_piece, &format!("{} {}", (x - w / 2.0).round(), (y - h / 2.0).round())));
        crate::library::note_laid(decors, &chosen_piece);
    }
    if let Some(line) = added {
        let now = edit::with_line_added(text, &line, edit::is_life(&line));
        let new = edit::items(&now).into_iter().rev().find(|one| now.lines().nth(one.line - 1).is_some_and(|known| known.trim() == line.trim()));
        asked.chose = Some(new.map(|one| vec![(one.line, one.at)]).unwrap_or_default());
        asked.made = Some(now);
        ui.close();
    }
    ui.separator();
    if ui.button("The Whole Decor").clicked() {
        asked.camera = (None, Vec2::ZERO);
        ui.close();
    }
}

/// The keys pressed over the Scene, read once.
struct Pressed {
    arrows: Option<[f32; 2]>,
    delete: bool,
    duplicate: bool,
    all: bool,
    escape: bool,
    copy: bool,
    cut: bool,
    paste: Option<String>,
    flip: bool,
    /// `]` forward (true), `[` back (false).
    order: Option<bool>,
    layer: bool,
}

impl Pressed {
    fn read(input: &egui::InputState) -> Pressed {
        let plain = !input.modifiers.command && !input.modifiers.alt;
        let arrows = [(egui::Key::ArrowLeft, [-1, 0]), (egui::Key::ArrowRight, [1, 0]), (egui::Key::ArrowUp, [0, -1]), (egui::Key::ArrowDown, [0, 1])]
            .into_iter()
            .find(|(key, _)| input.key_pressed(*key))
            .map(|(_, way)| crate::placing::nudge(way, input.modifiers.shift));
        let order = if plain && input.key_pressed(egui::Key::CloseBracket) {
            Some(true)
        } else if plain && input.key_pressed(egui::Key::OpenBracket) {
            Some(false)
        } else {
            None
        };
        Pressed {
            arrows,
            delete: input.key_pressed(egui::Key::Delete) || input.key_pressed(egui::Key::Backspace),
            duplicate: input.modifiers.command && input.key_pressed(egui::Key::D),
            all: input.modifiers.command && input.key_pressed(egui::Key::A),
            escape: input.key_pressed(egui::Key::Escape),
            copy: input.events.iter().any(|event| matches!(event, egui::Event::Copy)),
            cut: input.events.iter().any(|event| matches!(event, egui::Event::Cut)),
            paste: input.events.iter().find_map(|event| match event {
                egui::Event::Paste(said) => Some(said.clone()),
                _ => None,
            }),
            flip: plain && !input.modifiers.shift && input.key_pressed(egui::Key::H),
            order,
            layer: plain && !input.modifiers.shift && input.key_pressed(egui::Key::L),
        }
    }
}

/// The keys, the pointer over it: arrows a pixel (Shift: 8), Delete or
/// Backspace, Ctrl+D, Ctrl+A, Ctrl+C, Ctrl+X, Ctrl+V (`here`: where the
/// pointer is), H turned over, [ ] back and forward, L the next layer; Esc
/// the stamp put down, else nothing chosen.
fn keys(ui: &egui::Ui, decors: &mut Decors, read: &Read, chosen_now: &[Element], here: [f32; 2], snap: Snap, asked: &mut Asked) {
    let text = &read.text;
    let Pressed { arrows, delete, duplicate, all, escape, copy, cut, paste, flip, order, layer } = ui.input(Pressed::read);
    if !chosen_now.is_empty() {
        let mut lines: Vec<usize> = chosen_now.iter().map(|(line, _)| *line).collect();
        lines.sort_unstable();
        lines.dedup();
        if let Some([dx, dy]) = arrows {
            asked.made = Some(edit::moved(text, chosen_now, (dx, dy)));
        }
        if flip {
            asked.made = Some(edit::flipped(text, &lines));
        }
        if let (Some(front), [line]) = (order, lines.as_slice()) {
            asked.made = edit::to_front(text, *line, front);
            asked.chose = Some(Vec::new());
        }
        if layer {
            asked.made = Some(layered(text, &lines));
        }
        if delete {
            asked.made = Some(edit::removed(text, chosen_now));
            asked.chose = Some(Vec::new());
        }
        if copy || cut {
            decors.clipboard = edit::copied(text, chosen_now);
            ui.ctx().copy_text(decors.clipboard.join("\n"));
        }
        if cut {
            asked.made = Some(edit::removed(text, chosen_now));
            asked.chose = Some(Vec::new());
        }
        if duplicate && let Some(area) = read.box_of(chosen_now) {
            let (pasted, now) = edit::pasted(text, &edit::copied(text, chosen_now), [area.min.x + area.width().max(8.0), area.min.y]);
            asked.made = Some(pasted);
            asked.chose = Some(now);
        }
    }
    if let Some(said) = paste {
        // (Lines of a decor copied, from here or elsewhere; else what was copied here.)
        let said: Vec<String> = said.lines().map(str::to_string).filter(|line| !edit::items(line).is_empty()).collect();
        let lines = if said.is_empty() { decors.clipboard.clone() } else { said };
        if !lines.is_empty() {
            let (pasted, now) = edit::pasted(text, &lines, snap.of(here));
            asked.made = Some(pasted);
            asked.chose = Some(now);
        }
    }
    if all {
        asked.chose = Some(read.items.iter().filter(|one| read.takeable(one)).map(|one| (one.line, one.at)).collect());
    }
    if escape {
        // (Esc: the stamp put down first; then nothing chosen.)
        if decors.stamp.take().is_none() {
            asked.chose = Some(Vec::new());
        }
    }
}

/// Those lines' pieces each on the next layer (ground, objects, over, ground…), as the first of them goes.
pub(super) fn layered(text: &str, lines: &[usize]) -> String {
    let first = lines.iter().find_map(|line| text.lines().nth(line - 1).and_then(decor::doc::layer_of_line));
    let next = crate::placing::next_layer(first.unwrap_or("objects"));
    let mut made = text.to_string();
    for line in lines {
        if let Some(moved) = edit::on_layer(&made, *line, next) {
            made = moved;
        }
    }
    made
}

/// What was asked, done: the view kept, what is carried and drawn, chosen, the text changed; the pointer's look.
#[allow(clippy::too_many_arguments)]
fn done(ui: &egui::Ui, decors: &mut Decors, response: &egui::Response, view: View, middle: [f32; 2], under: Option<usize>, space: bool, asked: Asked) {
    let open = decors.open.as_mut().expect("open");
    (open.scale, open.pan) = asked.camera;
    open.seen_scale = asked.camera.0.unwrap_or(view.fit);
    open.middle = middle;
    open.view = view.rect.size();
    open.carried = asked.carry;
    open.marquee = asked.marquee;
    if let Some(chose) = asked.chose {
        open.choose(chose);
    }
    if let Some(made) = asked.made {
        open.change(made);
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(if space {
            egui::CursorIcon::Grab
        } else if open.carried.is_some() {
            egui::CursorIcon::Grabbing
        } else if under.is_some() {
            egui::CursorIcon::Grab
        } else {
            egui::CursorIcon::Default
        });
    }
    if open.carried.is_some() || open.marquee.is_some() {
        ui.ctx().request_repaint();
    }
}
