//! The Music and Sounds shelves as cards: a song's title, its word, how long
//! it plays once through, ↻ if it loops, its moods; a sound's word and when
//! it is heard. ▶ heard on a click (■ hushes it), heard after a moment's rest
//! of the pointer and hushed when it leaves, as the Film's Assets do. Only
//! the rows in sight drawn.

use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use super::picker::{HEARD_AFTER, hear_now, hush};
use super::{Item, Resources, Shelf};
use workshop_base::theme;

/// A card's size.
pub const CARD: egui::Vec2 = egui::vec2(260.0, 76.0);

/// How long a song plays once through: `2:05`, `↻` if it loops.
pub fn length_said(seconds: f32, loops: bool) -> String {
    let whole = seconds.round() as u32;
    format!("{}:{:02}{}", whole / 60, whole % 60, if loops { " ↻" } else { "" })
}

/// What the cards were done: one clicked (twice?).
#[derive(Default)]
pub struct Did {
    pub clicked: Option<(usize, bool)>,
}

/// The cards, in rows; `chosen`: the one chosen's place.
pub fn cards(ui: &mut egui::Ui, resources: &mut Resources, items: &[Item], chosen: Option<usize>, scroll_to: Option<f32>) -> Did {
    let mut did = Did::default();
    if items.first().is_some_and(|item| item.shelf == Shelf::Songs) {
        resources.song_lengths();
    }
    let columns = ((ui.available_width() - 10.0) / (CARD.x + 8.0)).floor().max(1.0) as usize;
    let rows = items.len().div_ceil(columns);
    let mut hovered_any = false;
    let mut area = egui::ScrollArea::vertical().id_salt(("music cards", resources.shelf)).auto_shrink(false);
    if let Some(offset) = scroll_to {
        area = area.vertical_scroll_offset(offset);
    }
    ui.spacing_mut().item_spacing.y = 0.0;
    area.show_rows(ui, CARD.y + 8.0, rows, |ui, range| {
        for row in range {
            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), CARD.y + 8.0), Sense::hover());
            for column in 0..columns {
                let at = row * columns + column;
                let Some(item) = items.get(at) else { break };
                let rect = Rect::from_min_size(line.min + vec2(8.0 + column as f32 * (CARD.x + 8.0), 4.0), CARD);
                let response = ui.interact(rect, ui.id().with(("music card", at)), Sense::click());
                card(ui, resources, item, rect, &response, chosen == Some(at));
                if response.hovered() {
                    hovered_any = true;
                    rested_on(ui, resources, item);
                }
                if response.clicked() || response.double_clicked() {
                    did.clicked = Some((at, response.double_clicked()));
                    clicked(resources, item);
                }
            }
        }
    });
    // (Heard on a hover: hushed when the pointer leaves the cards.)
    if !hovered_any {
        if resources.picker.resting.as_ref().is_some_and(|(key, _)| key.starts_with("song:") || key.starts_with("sound:")) {
            resources.picker.resting = None;
        }
        if resources.picker.hover_heard.take().is_some_and(|key| resources.heard.as_deref() == Some(key.as_str())) {
            hush(resources);
        }
    }
    did
}

/// A card: ▶ or ■, the title, the word, the moods or when it is heard, how long.
fn card(ui: &egui::Ui, resources: &Resources, item: &Item, rect: Rect, response: &egui::Response, chosen: bool) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let heard = resources.heard.as_deref() == Some(item.key().as_str());
    let painter = ui.painter_at(rect.expand(1.0));
    let fill = if chosen {
        theme::CHOSEN
    } else if response.hovered() {
        theme::HOVERED
    } else {
        theme::GROUND
    };
    painter.rect_filled(rect, 4.0, fill);
    if heard {
        painter.rect_stroke(rect, 4.0, Stroke::new(1.5_f32, theme::ASSETS), StrokeKind::Inside);
    }
    // (The button: ▶, ■ while heard; its ring filling while the pointer rests before it is heard.)
    let play = Rect::from_center_size(pos2(rect.min.x + 26.0, rect.center().y), vec2(34.0, 34.0));
    painter.circle_filled(play.center(), 17.0, if heard { theme::ASSETS } else { theme::TOOLS });
    painter.text(
        play.center() + vec2(1.0, 0.0),
        egui::Align2::CENTER_CENTER,
        if heard { "■" } else { "▶" },
        FontId::proportional(15.0),
        if heard { theme::GROUND } else { theme::FILM },
    );
    if response.hovered()
        && !heard
        && let Some((key, since)) = &resources.picker.resting
        && *key == item.key()
    {
        let part = (since.elapsed().as_secs_f64() / HEARD_AFTER).min(1.0) as f32;
        let points: Vec<egui::Pos2> = (0..=24)
            .map(|step| {
                let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * part * step as f32 / 24.0;
                play.center() + vec2(angle.cos(), angle.sin()) * 17.0
            })
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(2.0_f32, theme::FILM)));
    }
    let text = rect.min.x + 52.0;
    let wide = rect.max.x - text - 8.0;
    let line = |said: &str, font: FontId, colour: Color32, y: f32| {
        let mut job = egui::text::LayoutJob::simple_singleline(said.to_string(), font, colour);
        job.wrap = egui::text::TextWrapping { max_width: wide, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
        painter.galley(pos2(text, rect.min.y + y), painter.layout_job(job), colour);
    };
    let (title, under) = match item.shelf {
        Shelf::Songs => (item.label.clone(), item.about.clone()),
        _ => (item.word.clone(), item.about.clone()),
    };
    line(&title, FontId::proportional(theme::BODY + 0.5), theme::STRONG, 8.0);
    line(&item.word, FontId::monospace(theme::SMALL - 0.5), Color32::from_rgb(0xDC, 0xDC, 0xAA), 27.0);
    line(&under, FontId::proportional(theme::SMALL - 0.5), theme::WEAK, 45.0);
    if item.shelf == Shelf::Songs
        && let Some((seconds, loops)) = resources.lengths.get(&(item.number as u16)).copied()
    {
        let said = length_said(seconds, loops);
        let galley = painter.layout_no_wrap(said, FontId::proportional(theme::SMALL), theme::TEXT);
        let at = pos2(rect.max.x - galley.size().x - 8.0, rect.min.y + 29.0);
        painter.rect_filled(Rect::from_min_size(at - vec2(4.0, 1.0), galley.size() + vec2(8.0, 2.0)), 5.0, theme::PANEL);
        painter.galley(at, galley, theme::TEXT);
    }
}

/// The pointer resting on a card: heard after a moment (hushed when it leaves).
fn rested_on(ui: &egui::Ui, resources: &mut Resources, item: &Item) {
    let key = item.key();
    let heard = resources.heard.as_deref() == Some(key.as_str());
    let started = match &resources.picker.resting {
        Some((known, when)) if *known == key => *when,
        _ => {
            resources.picker.resting = Some((key.clone(), Instant::now()));
            if let Some(was) = resources.picker.hover_heard.take()
                && resources.heard.as_deref() == Some(was.as_str())
            {
                hush(resources);
            }
            Instant::now()
        }
    };
    let rested = started.elapsed().as_secs_f64();
    if rested >= HEARD_AFTER && resources.picker.hover_heard.as_deref() != Some(key.as_str()) && !heard {
        hear_now(resources, item);
        resources.picker.hover_heard = Some(key);
    } else if rested < HEARD_AFTER {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
    }
}

/// A click: heard on (kept when the pointer leaves), or hushed.
fn clicked(resources: &mut Resources, item: &Item) {
    let key = item.key();
    let heard = resources.heard.as_deref() == Some(key.as_str());
    if heard && resources.picker.hover_heard.as_deref() != Some(key.as_str()) {
        hush(resources);
    } else {
        if !heard {
            hear_now(resources, item);
        }
        resources.picker.hover_heard = None;
    }
}
