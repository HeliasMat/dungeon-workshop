//! The one chosen in large, on a checkered ground: its steps one by one or
//! playing, a Pokémon's faces and its body's animations, a sound's loudness.

use std::path::PathBuf;

use eframe::egui::{self, Color32, Painter, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use super::super::pictures::{animations_of, body_sheet, face_name, faces_of, paint_item, read_png};
use super::super::{Item, Resources, Sheet, Shelf};
use super::{Shown, section};
use crate::decors::Decors;
use crate::making::fitted;
use crate::theme;

/// What is shown of the one chosen: a person's body (if it has no faces, or
/// its body was asked) or its face, a thing's picture; the step shown.
pub(super) fn shown(resources: &mut Resources, context: &egui::Context, item: &Item) -> Shown {
    let person = matches!(item.shelf, Shelf::Pokemon | Shelf::Variants);
    let has_faces = person && assets::data::has(resources.session.data(), &format!("portraits/{}_0.png", item.number));
    let anims = if person { animations_of(&resources.session, item.number) } else { Vec::new() };
    let body = person && (!has_faces || resources.body.unwrap_or(false)) && !anims.is_empty();
    if body && !anims.iter().any(|(file, _)| *file == resources.anim) {
        resources.anim = anims.iter().find(|(_, word)| word == "Idle").or(anims.first()).map_or(5, |(file, _)| *file);
    }
    let sheet = if body {
        let key = format!("body:{}:{}", item.number, resources.anim);
        if !resources.pictures.contains_key(&key) {
            let made = body_sheet(&resources.session, context, item.number, resources.anim);
            resources.pictures.insert(key.clone(), made);
        }
        resources.pictures.get(&key).cloned().flatten()
    } else if person {
        None
    } else {
        resources.picture(context, item)
    };
    let steps = sheet.as_ref().map_or(1, |sheet| sheet.count);
    let step = match (&sheet, resources.playing) {
        (Some(sheet), true) if sheet.count > 1 => {
            context.request_repaint_after(std::time::Duration::from_millis(33));
            sheet.step_at(resources.began.elapsed().as_secs_f64())
        }
        _ => resources.step.min(steps.saturating_sub(1)),
    };
    Shown { person, has_faces, anims, body, sheet, steps, step }
}

/// The checkered ground, the one chosen drawn on it; how much it is grown, said.
pub(super) fn in_large(ui: &mut egui::Ui, resources: &mut Resources, decors: &mut Decors, context: &egui::Context, item: &Item, shown: &Shown) -> String {
    let room = Rect::from_min_size(ui.cursor().min + vec2(8.0, 6.0), vec2(ui.available_width() - 16.0, 170.0));
    ui.allocate_rect(room.expand2(vec2(0.0, 6.0)), Sense::hover());
    let painter = ui.painter_at(room);
    checkered(&painter, room);
    match item.shelf {
        Shelf::Pokemon | Shelf::Variants if shown.body => match &shown.sheet {
            Some(sheet) => {
                let fit = fitted(sheet.cell, room.shrink(10.0));
                painter.image(sheet.texture.id(), fit, sheet.uv_at(shown.step, resources.way), Color32::WHITE);
                format!("×{:.0}", (fit.width() / sheet.cell.x).max(1.0))
            }
            None => String::new(),
        },
        Shelf::Pokemon | Shelf::Variants => face_in_large(&painter, resources, context, item, room),
        _ => {
            paint_item(&painter, resources, decors, context, item, room.shrink(10.0), shown.step);
            shown.sheet.as_ref().map_or_else(String::new, |sheet| format!("×{:.0}", (fitted(sheet.cell, room.shrink(14.0)).width() / sheet.cell.x).max(1.0)))
        }
    }
}

/// The ground of squares a picture is shown on.
fn checkered(painter: &Painter, room: Rect) {
    let square = 12.0;
    for row in 0..(room.height() / square).ceil() as i32 {
        for column in 0..(room.width() / square).ceil() as i32 {
            let colour = if (row + column) % 2 == 0 { Color32::from_rgb(0x2E, 0x2E, 0x2E) } else { Color32::from_rgb(0x26, 0x26, 0x26) };
            painter.rect_filled(Rect::from_min_size(room.min + vec2(column as f32 * square, row as f32 * square), vec2(square, square)), 0.0, colour);
        }
    }
    painter.rect_stroke(room, 0.0, Stroke::new(1.0_f32, theme::GAP), StrokeKind::Outside);
}

/// A Pokémon's face chosen, in large (read once); how much it is grown, said.
fn face_in_large(painter: &Painter, resources: &mut Resources, context: &egui::Context, item: &Item, room: Rect) -> String {
    let faces = faces_of(&resources.session, item.number);
    let face = faces.get(resources.face).copied().unwrap_or(0);
    let path = PathBuf::from(assets::data::path(resources.session.data(), &format!("portraits/{}_{face}.png", item.number)));
    let key = format!("face:{}:{face}", item.number);
    if !resources.pictures.contains_key(&key) {
        let made = read_png(context, &path, &key, 0).map(|texture| Sheet { cell: texture.size_vec2(), texture, count: 1, ticks: vec![60] });
        resources.pictures.insert(key.clone(), made);
    }
    match resources.pictures.get(&key) {
        Some(Some(face)) => {
            let fit = fitted(face.cell, room.shrink(10.0));
            painter.image(face.texture.id(), fit, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
            format!("×{:.0}", fit.width() / face.cell.x)
        }
        _ => String::new(),
    }
}

/// Its steps, one by one or playing; a sound listened to; how much it is grown.
pub(super) fn steps_row(ui: &mut egui::Ui, resources: &mut Resources, item: &Item, shown: &Shown, scale_said: &str) {
    let (steps, step) = (shown.steps, shown.step);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing.x = 3.0;
        if steps > 1 {
            for at in 0..steps.min(12) {
                if ui.selectable_label(!resources.playing && step == at, format!("{}", at + 1)).clicked() {
                    (resources.step, resources.playing) = (at, false);
                }
            }
            if steps > 12 {
                let mut chosen = resources.step;
                if ui.add(egui::DragValue::new(&mut chosen).range(0..=steps - 1).prefix("image ")).changed() {
                    (resources.step, resources.playing) = (chosen, false);
                }
            }
            if ui.selectable_label(resources.playing, if resources.playing { "⏸" } else { "▶" }).on_hover_text("play, stop").clicked() {
                resources.playing = !resources.playing;
            }
        }
        if item.shelf.heard() {
            let heard = resources.heard.as_deref() == Some(item.key().as_str());
            let said = if heard { "■ Stop" } else { "▶ Listen" };
            if ui.add(egui::Button::new(RichText::new(said).color(theme::GROUND)).fill(theme::ASSETS)).clicked() {
                resources.hear(item);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            ui.label(theme::text(scale_said, theme::SMALL, theme::WEAK));
        });
    });
}

/// A Pokémon's faces; a click shows one.
pub(super) fn faces(ui: &mut egui::Ui, resources: &mut Resources, item: &Item, shown: &Shown) {
    section(ui, "Faces");
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
        for (at, face) in faces_of(&resources.session, item.number).iter().enumerate() {
            if ui.selectable_label(!shown.body && resources.face == at, theme::text(face_name(*face), theme::SMALL, theme::TEXT)).clicked() {
                (resources.face, resources.body) = (at, Some(false));
            }
        }
    });
}

/// Its body's animations, the way it faces; a click plays one.
pub(super) fn animations(ui: &mut egui::Ui, resources: &mut Resources, shown: &Shown) {
    section(ui, "Animations");
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(theme::text("Side", theme::SMALL, theme::WEAK));
        for way in 0..8 {
            way_button(ui, resources, way);
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
        for (file, word) in &shown.anims {
            if ui.selectable_label(shown.body && resources.anim == *file, theme::text(word.replace('_', " "), theme::SMALL, theme::TEXT)).clicked() {
                (resources.anim, resources.playing, resources.body) = (*file, true, Some(true));
            }
        }
    });
}

/// A way it may face, as a small button.
fn way_button(ui: &mut egui::Ui, resources: &mut Resources, way: usize) {
    let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    let on = resources.way == way;
    let fill = if on {
        theme::CHOSEN
    } else if response.hovered() {
        theme::HOVERED
    } else {
        theme::FIELD
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    crate::icons::way(ui.painter(), rect, way, if on { theme::STRONG } else { theme::WEAK });
    if response.on_hover_text("the way it faces").clicked() {
        (resources.way, resources.body) = (way, Some(true));
    }
}

/// A sound: heard as soon as it is chosen, or not; how loud.
pub(super) fn loudness(ui: &mut egui::Ui, resources: &mut Resources) {
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.checkbox(&mut resources.auto_hear, theme::text("Listen on choosing", theme::SMALL, theme::TEXT));
        ui.spacing_mut().slider_width = 80.0;
        if ui.add(egui::Slider::new(&mut resources.volume, 0.0..=1.0).show_value(false)).on_hover_text("the volume").changed()
            && let Some(audio) = resources.audio.as_mut()
        {
            audio.volume(0, resources.volume);
        }
    });
}
