//! The Library's cards: an element's (a square of the grid or a row of the
//! list, playing if the game animates it, its star), larger when pointed at
//! (at its own size, up to what the tip holds); a model's.

use decor::tags::Size;
use eframe::egui::{self, Color32, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use super::search::{from_said, title_of};
use super::{Picked, colour_of};
use crate::decors::{Decors, PieceArt, paint_art};
use crate::theme;
use crate::workshop::PieceInfo;

/// The colour a card chosen is filled and outlined with.
pub(super) const CHOSEN_FILL: Color32 = Color32::from_rgb(0x04, 0x39, 0x5E);
pub(super) const CHOSEN_LINE: Color32 = Color32::from_rgb(0x00, 0x7F, 0xD4);

/// The largest side a piece is shown at when pointed at.
const LARGEST: f32 = 320.0;

/// The elements' cards, in a list or a wrapped grid, with what was done with them.
pub(super) fn element_cards(ui: &mut egui::Ui, decors: &mut Decors, shown: &[PieceInfo], picked: &mut Picked) {
    let list = decors.lib.list;
    let chosen = decors.lib.chosen.clone();
    let stamping = decors.stamp.as_ref().map(|stamp| stamp.name.clone());
    let mut each = |ui: &mut egui::Ui| {
        for piece in shown {
            let mark = Mark { chosen: chosen.as_deref() == Some(piece.name.as_str()), stamping: stamping.as_deref() == Some(piece.name.as_str()) };
            card(ui, decors, piece, mark, picked);
        }
    };
    if list {
        ui.spacing_mut().item_spacing.y = 0.0;
        each(ui);
    } else {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        ui.horizontal_wrapped(|ui| each(ui));
    }
}

/// Whether a card plays its frames: the game animates it.
fn lives(piece: &PieceInfo) -> bool {
    piece.moves || piece.kind == 'a'
}

/// How a card is marked: chosen, its stamp in hand.
#[derive(Clone, Copy)]
struct Mark {
    chosen: bool,
    stamping: bool,
}

/// A piece's card (a row of the list, or a square of the grid): chosen by a
/// click (its stamp taken), laid by a double-click, dragged; its star; larger when pointed at.
fn card(ui: &mut egui::Ui, decors: &mut Decors, piece: &PieceInfo, mark: Mark, picked: &mut Picked) {
    let (list, size) = (decors.lib.list, decors.lib.size);
    let wanted = if list { vec2(ui.available_width(), 30.0) } else { vec2(size, size + 30.0) };
    let (rect, response) = ui.allocate_exact_size(wanted, Sense::click_and_drag());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let fill = if mark.chosen {
        CHOSEN_FILL
    } else if response.hovered() {
        theme::HOVERED
    } else if list {
        Color32::TRANSPARENT
    } else {
        theme::FIELD
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    if mark.chosen || mark.stamping {
        let line = if mark.stamping { Stroke::new(2.0_f32, theme::DECOR) } else { Stroke::new(1.0_f32, CHOSEN_LINE) };
        ui.painter().rect_stroke(rect, 3.0, line, StrokeKind::Inside);
    }
    let art = decors.art_now(&piece.name);
    // (The animated ones play their frames, as the game does.)
    let frame = match &art {
        Some(art) if lives(piece) && art.lower.len() > 1 => {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
            art.frame(ui.input(|input| input.time))
        }
        _ => 0,
    };
    if list {
        listed_face(ui, piece, art.as_ref(), rect, frame);
    } else {
        square_face(ui, piece, art.as_ref(), rect, size, frame);
    }
    if mark.stamping {
        crate::decors::tag(ui.painter(), rect.left_bottom() + vec2(3.0, -31.0), "stamp", theme::DECOR);
    }
    // (Its star: shown when pointed at, or when it is a favourite.)
    let favourite = decors.lib.favourites.contains(&piece.name);
    let star_at = if list {
        Rect::from_center_size(pos2(rect.max.x - 14.0, rect.center().y), vec2(18.0, 18.0))
    } else {
        Rect::from_center_size(rect.right_top() + vec2(-11.0, 12.0), vec2(18.0, 18.0))
    };
    if (response.hovered() || favourite)
        && crate::icons::button(ui, star_at, if favourite { "remove from the favourites" } else { "add to the favourites" }, |painter, at, tint| {
            crate::icons::star(painter, at, favourite, if favourite { Color32::from_rgb(0xF6, 0xD3, 0x3C) } else { tint })
        })
    {
        picked.star = Some(piece.name.clone());
    }
    let response = response.on_hover_ui(|ui| larger(ui, piece, art.as_ref()));
    if response.drag_started() {
        response.dnd_set_drag_payload(piece.name.clone());
    }
    if response.clicked() {
        picked.clicked = Some(piece.name.clone());
    }
    if response.double_clicked() {
        picked.laid = Some(piece.name.clone());
    }
}

/// A piece drawn whole in a room, at a frame: its pixels whole if it is smaller.
pub(super) fn paint_framed(painter: &egui::Painter, art: &PieceArt, room: Rect, frame: usize) {
    let Some(first) = art.lower.first() else { return };
    let size = first.size_vec2();
    let scale = (room.width() / size.x.max(1.0)).min(room.height() / size.y.max(1.0));
    let scale = if scale >= 1.0 { scale.floor() } else { scale };
    paint_art(painter, art, Rect::from_center_size(room.center(), size * scale), (true, true), frame, false, Color32::WHITE);
}

/// A row of the list: its picture small, its name and place, its category, size and id.
fn listed_face(ui: &egui::Ui, piece: &PieceInfo, art: Option<&PieceArt>, rect: Rect, frame: usize) {
    let square = Rect::from_min_size(rect.min + vec2(4.0, 3.0), vec2(24.0, 24.0));
    if let Some(art) = art {
        paint_framed(ui.painter(), art, square, frame);
    }
    let clip = ui.painter().with_clip_rect(rect.shrink(1.0));
    clip.text(pos2(square.max.x + 8.0, rect.min.y + 9.0), egui::Align2::LEFT_CENTER, title_of(piece), egui::FontId::proportional(theme::BODY), theme::TEXT);
    clip.text(pos2(square.max.x + 8.0, rect.max.y - 8.0), egui::Align2::LEFT_CENTER, from_said(piece), egui::FontId::proportional(theme::SMALL), theme::FAINT);
    let said = format!("{} · {} · {} × {} · {}", piece.category.shown(), Size::of(piece.size).shown(), piece.size[0], piece.size[1], piece.name);
    clip.text(pos2(rect.max.x - 30.0, rect.center().y), egui::Align2::RIGHT_CENTER, said, egui::FontId::proportional(theme::SMALL), theme::FAINT);
}

/// A square of the grid: its picture (playing if it is animated), its
/// category's mark, a wave if it moves, its name and its place under it.
fn square_face(ui: &egui::Ui, piece: &PieceInfo, art: Option<&PieceArt>, rect: Rect, size: f32, frame: usize) {
    let square = Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(size - 8.0, size - 8.0));
    if let Some(art) = art {
        paint_framed(ui.painter(), art, square, frame);
    }
    ui.painter().rect_filled(Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(5.0, 5.0)), 1.0, colour_of(piece.category));
    if lives(piece) {
        crate::icons::wave(ui.painter(), Rect::from_center_size(rect.left_top() + vec2(19.0, 6.5), vec2(12.0, 8.0)), theme::DECOR);
    }
    card_words(ui.painter(), rect, &title_of(piece), &from_said(piece));
}

/// A card's two lines under its picture, cut with "…" when too long: its name, and smaller, where it comes from.
pub(super) fn card_words(painter: &egui::Painter, rect: Rect, name: &str, under: &str) {
    let wide = rect.width() - 8.0;
    elided(painter, pos2(rect.center().x, rect.max.y - 20.0), name, egui::FontId::proportional(theme::SMALL), theme::TEXT, wide);
    elided(painter, pos2(rect.center().x, rect.max.y - 7.0), under, egui::FontId::proportional(theme::SMALL - 1.0), theme::FAINT, wide);
}

/// Words on one line, centred there, cut with "…" past that width.
fn elided(painter: &egui::Painter, centre: egui::Pos2, said: &str, font: egui::FontId, colour: Color32, wide: f32) {
    let mut job = egui::text::LayoutJob::simple_singleline(said.to_string(), font, colour);
    job.wrap = egui::text::TextWrapping::truncate_at_width(wide);
    let galley = painter.layout_job(job);
    let at = egui::Align2::CENTER_CENTER.anchor_size(centre, galley.size());
    painter.galley(at.min, galley, colour);
}

/// How large a piece of that size is shown pointed at: its own size (twice,
/// thrice… while it fits), smaller only if it does not fit; and that scale.
pub(super) fn preview_size(size: [f32; 2], largest: f32) -> ([f32; 2], f32) {
    let fits = (largest / size[0].max(1.0)).min(largest / size[1].max(1.0));
    let scale = if fits >= 1.0 { fits.floor().min(4.0) } else { fits };
    ([size[0] * scale, size[1] * scale], scale)
}

/// Pointed at: at its own size, living, with what it is, its tags and the places it is in.
fn larger(ui: &mut egui::Ui, piece: &PieceInfo, art: Option<&PieceArt>) {
    ui.set_max_width(LARGEST + 16.0);
    let size = art.and_then(|art| art.lower.first()).map_or(piece.size, |first| [first.size_vec2().x, first.size_vec2().y]);
    let ([w, h], scale) = preview_size(size, LARGEST);
    let (square, _) = ui.allocate_exact_size(vec2(w.max(48.0), h.max(48.0)), Sense::hover());
    paint_checks(ui.painter(), square);
    if let Some(art) = art {
        paint_living(ui, art, Rect::from_center_size(square.center(), vec2(w, h)));
    }
    ui.label(theme::text(title_of(piece), theme::BODY, theme::STRONG));
    let shown_at = if scale == 1.0 {
        "its own size".to_string()
    } else if scale > 1.0 {
        format!("shown ×{scale:.0}")
    } else {
        "shown smaller".to_string()
    };
    ui.label(theme::weak(format!(
        "{} · {} × {} pixels ({}, {shown_at}){}",
        piece.category.shown(),
        piece.size[0],
        piece.size[1],
        Size::of(piece.size).shown(),
        if lives(piece) { " · animated" } else { "" }
    )));
    if !piece.tags.is_empty() {
        ui.label(theme::text(piece.tags.join(", "), theme::SMALL, theme::WEAK));
    }
    match &piece.place {
        Some((_, place)) => ui.label(theme::weak(format!("from {place}"))),
        None => ui.label(theme::weak("laid by the scenes")),
    };
    if !piece.also.is_empty() {
        ui.label(theme::weak(format!("also in {}", piece.also.join(", "))));
    }
    ui.label(RichText::new(&piece.name).monospace().size(theme::SMALL).color(theme::FAINT));
    ui.label(theme::text(
        "click: a stamp, a click on the scene lays it (Esc stops) · double-click: in the middle · drag: where dropped",
        theme::SMALL,
        theme::FAINT,
    ));
}

/// A checkerboard under a picture, its see-through pixels seen as such.
pub(super) fn paint_checks(painter: &egui::Painter, room: Rect) {
    painter.rect_filled(room, 2.0, Color32::from_rgb(0x16, 0x16, 0x16));
    let side = 8.0;
    let mut y = room.min.y;
    let mut row = 0;
    while y < room.max.y {
        let mut x = room.min.x + if row % 2 == 0 { 0.0 } else { side };
        while x < room.max.x {
            painter.rect_filled(
                Rect::from_min_max(pos2(x, y), pos2((x + side).min(room.max.x), (y + side).min(room.max.y))),
                0.0,
                Color32::from_rgb(0x22, 0x22, 0x22),
            );
            x += side * 2.0;
        }
        y += side;
        row += 1;
    }
}

/// A piece drawn in a room, living (its frames as they go).
fn paint_living(ui: &egui::Ui, art: &PieceArt, rect: Rect) {
    let seconds = ui.input(|input| input.time);
    paint_art(ui.painter(), art, rect, (true, true), art.frame(seconds), false, Color32::WHITE);
    ui.ctx().request_repaint();
}

/// The models: a card each (its name, how many lines), laid by a double-click
/// in the middle of what is seen, or dragged onto the scene. Those of `shown` only (their places in the list).
pub(super) fn model_cards(ui: &mut egui::Ui, models: &[(String, Vec<String>)], shown: &[usize], laid: &mut Option<Vec<String>>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for (name, lines) in shown.iter().filter_map(|at| models.get(*at)) {
            let (rect, response) = ui.allocate_exact_size(vec2(170.0, 54.0), Sense::click_and_drag());
            ui.painter().rect_filled(rect, 3.0, if response.hovered() { theme::HOVERED } else { theme::FIELD });
            crate::icons::folder(
                ui.painter(),
                Rect::from_center_size(rect.left_center() + vec2(18.0, 0.0), vec2(18.0, 18.0)),
                false,
                Color32::from_rgb(0xDC, 0xB6, 0x7A),
            );
            ui.painter().text(rect.left_center() + vec2(34.0, -8.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(theme::BODY), theme::TEXT);
            ui.painter().text(
                rect.left_center() + vec2(34.0, 9.0),
                egui::Align2::LEFT_CENTER,
                format!("{} lines", lines.len()),
                egui::FontId::proportional(theme::SMALL),
                theme::FAINT,
            );
            let response = response.on_hover_text(format!("{}\n\ndouble-click: in the middle · drag: where dropped", lines.join("\n")));
            if response.drag_started() {
                response.dnd_set_drag_payload(format!("model:{name}"));
            }
            if response.double_clicked() {
                *laid = Some(lines.clone());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::preview_size;

    /// Pointed at, a piece shows at its own size, larger in whole steps while it fits (four times at most), smaller only if it does not fit.
    #[test]
    fn the_preview_shows_a_piece_at_its_size() {
        assert_eq!(preview_size([100.0, 50.0], 320.0), ([300.0, 150.0], 3.0));
        assert_eq!(preview_size([16.0, 16.0], 320.0), ([64.0, 64.0], 4.0));
        assert_eq!(preview_size([320.0, 100.0], 320.0), ([320.0, 100.0], 1.0));
        let ([w, h], scale) = preview_size([640.0, 320.0], 320.0);
        assert_eq!((w, h, scale), (320.0, 160.0, 0.5));
    }
}
