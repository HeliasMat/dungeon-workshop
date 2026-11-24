//! The sound editor's controls: a setting, a group of them, a choice, steps
//! drawn, a piano roll.

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use super::Note;
use workshop_base::theme;

/// A row of a sound's setting: its name, its slider (its help); whether it was taken, let go.
pub(super) fn setting(ui: &mut egui::Ui, name: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>, suffix: &str, help: &str) -> (bool, bool) {
    ui.label(theme::text(name, theme::SMALL, theme::WEAK)).on_hover_text(help);
    ui.spacing_mut().slider_width = 200.0;
    let response = ui
        .add(egui::Slider::new(value, range).suffix(suffix).max_decimals(3).clamping(egui::SliderClamping::Never))
        .on_hover_text(format!("{help}\n(the value can be typed too, beyond the bounds: a double-click)"));
    if !value.is_finite() {
        *value = 0.0;
    }
    ui.end_row();
    (response.drag_started() || (response.changed() && !response.dragged()), response.drag_stopped() || (response.changed() && !response.dragged()))
}

/// A group of settings, folded or not, locked or not: whether one was taken, let go.
pub(super) type Row<'a> = (&'a str, &'a mut f32, std::ops::RangeInclusive<f32>, &'a str, &'a str);
pub(super) fn group(ui: &mut egui::Ui, title: &str, lock: &mut bool, rows: Vec<Row<'_>>) -> (bool, bool) {
    let mut moved = (false, false);
    let header = if *lock { format!("{title}  · locked") } else { title.to_string() };
    egui::CollapsingHeader::new(RichText::new(header).strong()).id_salt(title).default_open(true).show(ui, |ui| {
        ui.checkbox(lock, theme::text("locked (Make, Random and Mutate leave it alone)", theme::SMALL - 1.0, theme::FAINT));
        egui::Grid::new(("settings", title)).num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
            for (name, value, range, suffix, help) in rows {
                let (taken, let_go) = setting(ui, name, value, range, suffix, help);
                moved = (moved.0 | taken, moved.1 | let_go);
            }
        });
    });
    moved
}

/// A choice among names, kept as a number (`filter_mode`, a route's source…): whether it changed.
pub(super) fn choice(ui: &mut egui::Ui, id: impl std::hash::Hash, value: &mut f32, names: &[&str]) -> bool {
    let mut at = (value.round().max(0.0) as usize).min(names.len() - 1);
    let before = at;
    egui::ComboBox::from_id_salt(id).selected_text(names[at]).width(110.0).show_ui(ui, |ui| {
        for (place, name) in names.iter().enumerate() {
            ui.selectable_value(&mut at, place, *name);
        }
    });
    if at != before {
        *value = at as f32;
    }
    at != before
}

/// 32 steps drawn with the pointer (a wave, a curve), from -1 to 1: whether taken, let go.
pub(super) fn steps_editor(ui: &mut egui::Ui, values: &mut [f32; 32], lit: bool) -> (bool, bool) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width().min(520.0), 80.0), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    painter.hline(rect.x_range(), rect.center().y, Stroke::new(1.0_f32, theme::GAP));
    let wide = rect.width() / 32.0;
    for (at, value) in values.iter().enumerate() {
        let x = rect.min.x + at as f32 * wide;
        let y = rect.center().y - value.clamp(-1.0, 1.0) * rect.height() * 0.45;
        painter.rect_filled(Rect::from_two_pos(pos2(x + 1.0, rect.center().y), pos2(x + wide - 1.0, y)), 0.0, if lit { theme::ASSETS } else { theme::FAINT });
    }
    let mut moved = (false, false);
    if let Some(at) = response.interact_pointer_pos().filter(|_| response.dragged() || response.clicked()) {
        let step = (((at.x - rect.min.x) / wide).max(0.0) as usize).min(31);
        values[step] = ((rect.center().y - at.y) / (rect.height() * 0.45)).clamp(-1.0, 1.0);
        moved.0 |= response.drag_started() || response.clicked();
    }
    moved.1 |= response.drag_stopped() || response.clicked();
    moved
}

/// A voice's score as a piano roll: a click adds a note, dragging moves it, its right edge lengthens it,
/// a right click takes it off; the chosen one's numbers below. Whether taken, let go.
pub(crate) fn piano_roll(ui: &mut egui::Ui, notes: &mut Vec<Note>, id: egui::Id, beats: f32, named: bool, scale: Option<[bool; 12]>) -> (bool, bool) {
    const LOW: f32 = -12.0;
    const HIGH: f32 = 24.0;
    const SNAP: f32 = 0.25;
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let beats = beats.max(1.0);
    let rows = HIGH - LOW + 1.0;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width().min(900.0), 300.0), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    let grid = Rect::from_min_max(rect.min + vec2(30.0, 0.0), rect.max);
    let (row_high, beat_wide) = (grid.height() / rows, grid.width() / beats);
    let y_of = |tone: f32| grid.min.y + (HIGH - tone) * row_high;
    let x_of = |beat: f32| grid.min.x + beat * beat_wide;
    for row in 0..rows as i32 {
        let tone = HIGH - row as f32;
        let semitone = (tone as i32).rem_euclid(12);
        let band = Rect::from_min_size(pos2(grid.min.x, y_of(tone)), vec2(grid.width(), row_high));
        if matches!(semitone, 1 | 3 | 6 | 8 | 10) {
            painter.rect_filled(band, 0.0, theme::GAP);
        }
        // (A scale chosen: the rows out of it, darker; its notes, lit.)
        if let Some(scale) = scale {
            if scale[semitone as usize] {
                painter.rect_filled(band, 0.0, theme::ASSETS.gamma_multiply(0.12));
            } else {
                painter.rect_filled(band, 0.0, Color32::from_black_alpha(150));
            }
        }
        if semitone == 0 {
            painter.hline(grid.x_range(), band.max.y, Stroke::new(1.0_f32, theme::FAINT));
        }
        if named || semitone == 0 {
            let said = if named { format!("{}{}", NAMES[semitone as usize], 4 + (tone as i32).div_euclid(12)) } else { format!("{:+}", tone as i32) };
            if !named || matches!(semitone, 0 | 4 | 7) {
                painter.text(pos2(rect.min.x + 3.0, band.center().y), egui::Align2::LEFT_CENTER, said, egui::FontId::monospace(9.0), theme::WEAK);
            }
        }
    }
    for quarter in 0..=(beats / SNAP) as i32 {
        let colour = if quarter % 16 == 0 {
            theme::FAINT
        } else if quarter % 4 == 0 {
            theme::HOVERED
        } else {
            theme::PANEL
        };
        painter.vline(x_of(quarter as f32 * SNAP), grid.y_range(), Stroke::new(1.0_f32, colour));
    }
    let note_rect =
        |note: &Note| Rect::from_min_max(pos2(x_of(note.start), y_of(note.tone) + 1.0), pos2(x_of(note.start + note.length), y_of(note.tone) + row_high - 1.0));
    let chosen_id = id.with("chosen");
    let dragged_id = id.with("dragged");
    let mut chosen: Option<usize> = ui.data(|data| data.get_temp::<Option<usize>>(chosen_id)).flatten().filter(|at| *at < notes.len());
    for (at, note) in notes.iter().enumerate() {
        let shown = note_rect(note);
        painter.rect_filled(shown, 2.0, theme::ASSETS.gamma_multiply(0.35 + 0.65 * note.velocity.clamp(0.0, 1.0)));
        if chosen == Some(at) {
            painter.rect_stroke(shown, 2.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Outside);
        }
    }
    let snap = |value: f32| (value / SNAP).round() * SNAP;
    let place_of = |at: Pos2| (((at.x - grid.min.x) / beat_wide).max(0.0), (HIGH - ((at.y - grid.min.y) / row_high).floor()).clamp(LOW, HIGH));
    let hit = |at: Pos2, notes: &[Note]| notes.iter().rposition(|note| note_rect(note).expand(1.0).contains(at));
    let mut moved = (false, false);
    if let Some(at) = response.interact_pointer_pos() {
        if response.secondary_clicked() {
            if let Some(gone) = hit(at, notes) {
                notes.remove(gone);
                chosen = None;
                moved = (true, true);
            }
        } else if response.drag_started() || response.clicked() {
            match hit(at, notes) {
                Some(found) => {
                    let note = notes[found];
                    let edge = (at.x - x_of(note.start + note.length)).abs() < 6.0;
                    ui.data_mut(|data| data.insert_temp(dragged_id, (found, edge, place_of(at).0 - note.start)));
                    chosen = Some(found);
                    moved.0 = true;
                }
                None if grid.contains(at) => {
                    let (beat, tone) = place_of(at);
                    let length = chosen.and_then(|was| notes.get(was)).map_or(1.0, |note| note.length);
                    notes.push(Note { start: (beat / SNAP).floor() * SNAP, length, tone, velocity: 1.0 });
                    chosen = Some(notes.len() - 1);
                    ui.data_mut(|data| data.insert_temp(dragged_id, (notes.len() - 1, true, 0.0_f32)));
                    moved = (true, response.clicked());
                }
                None => {}
            }
        }
        if response.dragged()
            && let Some((dragged, edge, grab)) = ui.data(|data| data.get_temp::<(usize, bool, f32)>(dragged_id))
            && let Some(note) = notes.get_mut(dragged)
        {
            let (beat, tone) = place_of(at);
            if edge {
                note.length = (snap(beat) - note.start).max(SNAP);
            } else {
                (note.start, note.tone) = (snap(beat - grab).max(0.0), tone);
            }
        }
    }
    if response.drag_stopped() {
        ui.data_mut(|data| data.remove::<(usize, bool, f32)>(dragged_id));
        moved.1 = true;
    }
    ui.data_mut(|data| data.insert_temp(chosen_id, chosen));
    if let Some(note) = chosen.and_then(|at| notes.get_mut(at)) {
        ui.horizontal(|ui| {
            ui.label(theme::text("the chosen note:", theme::SMALL, theme::WEAK));
            for (name, value, speed) in [
                ("start", &mut note.start, 0.05),
                ("length", &mut note.length, 0.05),
                ("semitone", &mut note.tone, 0.2),
                ("velocity", &mut note.velocity, 0.01),
            ] {
                ui.label(theme::text(name, theme::SMALL, theme::FAINT));
                let response = ui.add(egui::DragValue::new(value).speed(speed).max_decimals(2));
                moved.0 |= response.drag_started() || (response.changed() && !response.dragged());
                moved.1 |= response.drag_stopped() || (response.changed() && !response.dragged());
            }
        });
    }
    moved
}
