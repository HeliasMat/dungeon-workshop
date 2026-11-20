//! The Library's backdrops: each place of the game, which a decor is laid
//! on. A double-click, Enter or a drag onto the scene lays the decor on it:
//! its rest (`r<level>`, the place without its elements) as the backdrop,
//! and each of its elements (houses, trees…) where the game has it, each one
//! to move or take out (the backdrop it had changed, its elements with it:
//! one step, undone by Ctrl+Z). Its menu: the ground alone, the decor made
//! its size too, the place's elements in the Library, the backdrop taken out.

use eframe::egui::{self, Color32, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use super::Source;
use super::cards::{CHOSEN_FILL, CHOSEN_LINE, card_words, paint_checks, paint_framed};
use crate::decors::Decors;
use crate::edit;
use crate::theme;
use crate::workshop::BackdropInfo;

/// What was asked of a backdrop: the decor laid on it (its size made the
/// backdrop's, or kept; `bare`: its ground alone, without its elements), taken off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Asked {
    Use { level: i64, resized: bool, bare: bool },
    TakenOff,
}

/// The `put` lines of a place's elements, where the game lays them (none if
/// the place cannot be read).
pub fn place_elements(decors: &Decors, level: i64) -> Vec<String> {
    let session = &decors.session;
    let Ok(text) = decor::recreated(session.library(), session.data(), level, "place", "place") else { return Vec::new() };
    text.lines()
        .filter(|line| line.starts_with("put ") && edit::rest_level(line.split_whitespace().nth(1).unwrap_or_default()).is_none())
        .map(str::to_string)
        .collect()
}

/// The decor's text once that is asked (`elements`: the place's own, laid
/// with it unless `bare`): none if nothing changes.
pub fn asked_text(text: &str, asked: Asked, backdrops: &[BackdropInfo], elements: &[String]) -> Option<String> {
    let made = match asked {
        Asked::Use { level, resized, bare } => {
            let laid = if bare { edit::with_backdrop(text, level) } else { edit::with_place(text, level, elements) };
            match backdrops.iter().find(|backdrop| backdrop.level == level).filter(|_| resized) {
                Some(backdrop) => edit::with_size(&laid, backdrop.size[0] as i64, backdrop.size[1] as i64),
                None => laid,
            }
        }
        Asked::TakenOff => edit::without_backdrop(text),
    };
    (made != text).then_some(made)
}

/// What was done to the backdrops' cards: one chosen, one asked.
#[derive(Default)]
pub(super) struct Done {
    pub chosen: Option<i64>,
    pub asked: Option<Asked>,
    pub elements_of: Option<i64>,
}

/// The backdrops' cards, wrapped: the one the decor is laid on marked.
pub(super) fn backdrop_cards(ui: &mut egui::Ui, decors: &mut Decors, shown: &[BackdropInfo], done: &mut Done) {
    let used = decors.open.as_ref().and_then(|open| edit::backdrop_of(&open.text)).map(|(_, level)| level);
    let has_elements: std::collections::HashSet<i64> = decors.pieces.iter().filter_map(|piece| piece.place.as_ref().map(|(level, _)| *level)).collect();
    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
    ui.horizontal_wrapped(|ui| {
        for backdrop in shown {
            card(ui, decors, backdrop, used == Some(backdrop.level), has_elements.contains(&backdrop.level), done);
        }
    });
}

/// A backdrop's card: what one gets, its place's name, its size; chosen by a
/// click, used by a double-click or a drag; its menu.
fn card(ui: &mut egui::Ui, decors: &mut Decors, backdrop: &BackdropInfo, used: bool, has_elements: bool, done: &mut Done) {
    let wide = (decors.lib.size * 1.8).max(120.0);
    let (rect, response) = ui.allocate_exact_size(vec2(wide, wide * 0.75 + 30.0), Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let chosen = decors.lib.backdrop == Some(backdrop.level);
    ui.painter().rect_filled(
        rect,
        3.0,
        if chosen {
            CHOSEN_FILL
        } else if response.hovered() {
            theme::HOVERED
        } else {
            theme::FIELD
        },
    );
    if chosen || used {
        let line = if used { Stroke::new(2.0_f32, theme::DECOR) } else { Stroke::new(1.0_f32, CHOSEN_LINE) };
        ui.painter().rect_stroke(rect, 3.0, line, StrokeKind::Inside);
    }
    let room = Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(wide - 8.0, wide * 0.75 - 8.0));
    picture(ui, decors, backdrop.level, room, used || chosen);
    if used {
        crate::decors::tag(ui.painter(), room.left_bottom() + vec2(2.0, -2.0), "this decor's backdrop", theme::DECOR);
    }
    card_words(ui.painter(), rect, &backdrop.place, &format!("{} · {} × {}", backdrop.kind.shown(), backdrop.size[0], backdrop.size[1]));
    let response = response.on_hover_ui(|ui| larger(ui, decors, backdrop));
    if response.drag_started() {
        response.dnd_set_drag_payload(format!("backdrop:{}", backdrop.level));
    }
    if response.clicked() {
        done.chosen = Some(backdrop.level);
    }
    if response.double_clicked() {
        done.asked = Some(Asked::Use { level: backdrop.level, resized: false, bare: false });
    }
    response.context_menu(|ui| {
        if ui
            .button("Use as the Backdrop")
            .on_hover_text("the decor laid on it with all its elements, each one to move; its size kept (Ctrl+Z undoes it)")
            .clicked()
        {
            done.asked = Some(Asked::Use { level: backdrop.level, resized: false, bare: false });
            ui.close();
        }
        let said = format!("Use as the Backdrop, the Decor {} × {}", backdrop.size[0], backdrop.size[1]);
        if ui.button(said).on_hover_text("the decor laid on it with all its elements, and made its size").clicked() {
            done.asked = Some(Asked::Use { level: backdrop.level, resized: true, bare: false });
            ui.close();
        }
        if ui.button("Use Its Ground Only").on_hover_text("the place without its elements (houses, trees…), to lay one's own").clicked() {
            done.asked = Some(Asked::Use { level: backdrop.level, resized: false, bare: true });
            ui.close();
        }
        if has_elements && ui.button("Its Elements").on_hover_text("the elements the game lays on this place, in the Library").clicked() {
            done.elements_of = Some(backdrop.level);
            ui.close();
        }
        if used && ui.button(egui::RichText::new("Take the Backdrop Off").color(theme::WRONG)).clicked() {
            done.asked = Some(Asked::TakenOff);
            ui.close();
        }
    });
}

/// What a backdrop gives, in a room: its rest as cut, if known (asked only
/// when `ask`: a whole place is long to cut), else the place as the game shows it.
fn picture(ui: &egui::Ui, decors: &mut Decors, level: i64, room: Rect, ask: bool) {
    if !ui.is_rect_visible(room) {
        return;
    }
    ui.painter().rect_filled(room, 2.0, theme::GROUND);
    let rest = format!("r{level}");
    let art = if ask { decors.art_now(&rest) } else { decors.art_known(&rest) };
    if let Some(art) = art {
        paint_framed(ui.painter(), &art, room, 0);
        return;
    }
    if !decors.lib.pictures.contains_key(&level) {
        let made = super::place_thumb(ui.ctx(), decors.session.data(), level, 240);
        decors.lib.pictures.insert(level, made);
    }
    if let Some(Some(texture)) = decors.lib.pictures.get(&level) {
        let size = texture.size_vec2();
        let scale = (room.width() / size.x).min(room.height() / size.y);
        ui.painter().image(
            texture.id(),
            Rect::from_center_size(room.center(), size * scale),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    if ask {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
    }
}

/// Pointed at: larger, what it is, what a click does.
fn larger(ui: &mut egui::Ui, decors: &mut Decors, backdrop: &BackdropInfo) {
    ui.set_max_width(400.0);
    let [w, h] = backdrop.size;
    let scale = (384.0 / w.max(1.0)).min(288.0 / h.max(1.0)).min(1.0);
    let (room, _) = ui.allocate_exact_size(vec2(w * scale, h * scale), Sense::hover());
    paint_checks(ui.painter(), room);
    picture(ui, decors, backdrop.level, room, true);
    ui.label(theme::text(&backdrop.place, theme::BODY, theme::STRONG));
    ui.label(theme::weak(format!("{} · {} × {} pixels · place {} of the game", backdrop.kind.shown(), w, h, backdrop.level)));
    ui.label(theme::weak(
        "laid with its elements (houses, trees…), each one to move or take out; right click › Use Its Ground Only for the place without them",
    ));
    ui.label(theme::text(
        "double-click, Enter or drag onto the scene: the decor laid on it · right click: its size, its ground alone, its elements",
        theme::SMALL,
        theme::FAINT,
    ));
}

/// What is asked of the backdrops, done on the decor open (one step, undone by Ctrl+Z).
pub(super) fn apply(decors: &mut Decors, done: Done) {
    if let Some(level) = done.chosen {
        decors.lib.backdrop = Some(level);
    }
    if let Some(level) = done.elements_of {
        decors.lib.source = Source::Place(level);
        decors.lib.sought.clear();
    }
    if let Some(asked) = done.asked {
        use_asked(decors, asked);
    }
}

/// A backdrop asked, done on the decor open.
fn use_asked(decors: &mut Decors, asked: Asked) {
    let backdrops = decors.backdrops.clone();
    let elements = match asked {
        Asked::Use { level, bare: false, .. } => place_elements(decors, level),
        _ => Vec::new(),
    };
    let Some(open) = &mut decors.open else { return };
    if let Some(made) = asked_text(&open.text, asked, &backdrops, &elements) {
        open.change(made);
    }
}

#[cfg(test)]
mod tests {
    use super::{Asked, asked_text};
    use crate::workshop::BackdropInfo;
    use decor::tags::PlaceKind;

    /// A backdrop's ground used (the decor made its size if asked), changed, taken off; nothing when nothing changes.
    #[test]
    fn a_backdrop_is_used_resized_and_taken_off() {
        let backdrops = vec![BackdropInfo { level: 11, place: "Beach".to_string(), kind: PlaceKind::Beach, size: [512.0, 384.0] }];
        let text = "decor: a 256 192\nput e1_1 8 8\n";
        let bare = |resized| Asked::Use { level: 11, resized, bare: true };
        let used = asked_text(text, bare(false), &backdrops, &[]).expect("used");
        assert_eq!(used, "decor: a 256 192\nput r11 0 0 shadow off layer ground locked\nput e1_1 8 8\n");
        let resized = asked_text(text, bare(true), &backdrops, &[]).expect("resized");
        assert!(resized.starts_with("decor: a 512 384\nput r11 0 0"));
        assert_eq!(asked_text(&used, bare(false), &backdrops, &[]), None, "the same: nothing");
        assert_eq!(asked_text(&used, Asked::TakenOff, &backdrops, &[]).as_deref(), Some(text));
        assert_eq!(asked_text(text, Asked::TakenOff, &backdrops, &[]), None);
    }

    /// A backdrop used with its place's elements, laid over it; changed for
    /// another place, the first place's elements go with its backdrop, one's own stay.
    #[test]
    fn a_backdrop_brings_its_elements_and_takes_them_away() {
        let backdrops = Vec::new();
        let text = "decor: a 256 192\nput rocher 40 40\n";
        let with = |level| Asked::Use { level, resized: false, bare: false };
        let eleven = ["put e11_1 10 20 shadow off".to_string(), "put a11_2 0 0 shadow off".to_string()];
        let used = asked_text(text, with(11), &backdrops, &eleven).expect("used");
        assert_eq!(
            used,
            "decor: a 256 192\nput r11 0 0 shadow off layer ground locked\nput e11_1 10 20 shadow off\nput a11_2 0 0 shadow off\nput rocher 40 40\n"
        );
        let other = ["put e12_5 30 30 shadow off".to_string()];
        let changed = asked_text(&used, with(12), &backdrops, &other).expect("changed");
        assert_eq!(changed, "decor: a 256 192\nput r12 0 0 shadow off layer ground locked\nput e12_5 30 30 shadow off\nput rocher 40 40\n");
    }
}
