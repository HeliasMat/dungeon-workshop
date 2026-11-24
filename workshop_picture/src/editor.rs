//! The picture's editor: laid out, its keys, saved.

use eframe::egui::{self, Rect, RichText, Sense, pos2, vec2};

use super::canvas::canvas;
use super::panels::{side_panel, tools_panel};
use super::simple::paint_simply;
use super::steps::steps_panel;
use super::{Painting, Tool};
use workshop_base::done::Done;
use workshop_base::theme;

/// The painting: its bar, its tools, its canvas, its colours and layers, its steps.
pub fn paint(ui: &mut egui::Ui, painting: &mut Painting) -> Option<Done> {
    let mut done = None;
    let context = ui.ctx().clone();
    // --- The mode: simply, or as an expert ---
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        if ui
            .selectable_label(!painting.expert, RichText::new("Easy").strong())
            .on_hover_text("a few tools, shapes to begin with, ready-made animations")
            .clicked()
        {
            painting.set_expert(false);
        }
        if ui.selectable_label(painting.expert, RichText::new("Expert").strong()).on_hover_text("all the tools, the layers, no limits").clicked() {
            painting.set_expert(true);
        }
    });
    if !painting.expert {
        return paint_simply(ui, painting, &context);
    }
    // --- The bar ---
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.label(theme::text("Word", theme::SMALL, theme::WEAK));
        ui.add(egui::TextEdit::singleline(&mut painting.word).desired_width(140.0));
        ui.separator();
        let (mut wide, mut high) = painting.size;
        ui.add(egui::DragValue::new(&mut wide).range(1..=1024).suffix(" px"));
        ui.label("×");
        ui.add(egui::DragValue::new(&mut high).range(1..=1024).suffix(" px"));
        painting.size = (wide, high);
        let now = (painting.wide() as u32, painting.high() as u32);
        if painting.size != now && ui.button("Change the Size").on_hover_text("what is painted is kept from the top left").clicked() {
            painting.resized(painting.size);
        }
        ui.separator();
        let undo_said = painting.undone.last().map_or("nothing to undo".to_string(), |kept| format!("undo: {} (Ctrl+Z)", kept.said));
        if ui.add_enabled(!painting.undone.is_empty(), egui::Button::new("Undo")).on_hover_text(undo_said).clicked() {
            painting.undo();
        }
        let redo_said = painting.redone.last().map_or("nothing to redo".to_string(), |kept| format!("redo: {} (Ctrl+Y)", kept.said));
        if ui.add_enabled(!painting.redone.is_empty(), egui::Button::new("Redo")).on_hover_text(redo_said).clicked() {
            painting.redo();
        }
        ui.separator();
        ui.menu_button("Image", |ui| {
            if ui.button("Flip ↔").clicked() {
                painting.transformed("flip", image::imageops::flip_horizontal);
                ui.close();
            }
            if ui.button("Flip ↕").clicked() {
                painting.transformed("flip", image::imageops::flip_vertical);
                ui.close();
            }
            if ui.button("Rotate a Quarter Turn").clicked() {
                painting.transformed("rotate", image::imageops::rotate90);
                ui.close();
            }
            if ui.button("Outline (Primary Colour)").on_hover_text("a stroke around what is painted, like the game's sprites").clicked() {
                painting.outline();
                ui.close();
            }
            if ui.button("Replace the Primary with the Secondary").on_hover_text("on all the layer's images").clicked() {
                painting.replace_colour(painting.primary, painting.secondary);
                ui.close();
            }
            ui.separator();
            if ui.button("Select All (Ctrl+A)").clicked() {
                painting.drop_floating();
                painting.selection = Some([0, 0, painting.wide() - 1, painting.high() - 1]);
                ui.close();
            }
            if ui.button("Clear (Delete)").clicked() {
                painting.keep("clear");
                painting.clear_chosen();
                ui.close();
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(6.0);
            if ui.button("Close").clicked() {
                done = Some(Done::Closed);
            }
            let said = if painting.saved { "✔ Saved" } else { "Save" };
            if ui
                .add(egui::Button::new(RichText::new(said).strong().color(theme::GROUND)).fill(theme::ASSETS))
                .on_hover_text("Ctrl+S: into the drawn objects")
                .clicked()
            {
                done = save_painting(painting);
            }
            if let Some(said) = &painting.said {
                ui.label(theme::text(said, theme::SMALL, theme::FAINT));
            }
        });
    });
    keys(ui, painting, &mut done);
    let room = ui.available_rect_before_wrap();
    let tools = Rect::from_min_size(room.min + vec2(6.0, 4.0), vec2(112.0, room.height() - 8.0));
    let side = Rect::from_min_max(pos2(room.max.x - 250.0, room.min.y + 4.0), room.max - vec2(6.0, 4.0));
    let strip_room = Rect::from_min_max(pos2(tools.max.x + 8.0, room.max.y - 160.0), pos2(side.min.x - 8.0, room.max.y - 4.0));
    let canvas_room = Rect::from_min_max(pos2(tools.max.x + 8.0, room.min.y + 4.0), pos2(side.min.x - 8.0, strip_room.min.y - 6.0));
    let mut column = ui.new_child(egui::UiBuilder::new().max_rect(tools).layout(egui::Layout::top_down_justified(egui::Align::Min)));
    tools_panel(&mut column, painting);
    canvas(ui, painting, canvas_room, &context);
    let mut steps = ui.new_child(egui::UiBuilder::new().max_rect(strip_room).layout(egui::Layout::top_down(egui::Align::Min)));
    steps_panel(&mut steps, painting, &context);
    let mut right = ui.new_child(egui::UiBuilder::new().max_rect(side).layout(egui::Layout::top_down(egui::Align::Min)));
    egui::ScrollArea::vertical().id_salt("painting on the right").auto_shrink(false).show(&mut right, |ui| side_panel(ui, painting, &context));
    ui.allocate_rect(room, Sense::hover());
    done
}

pub(super) fn save_painting(painting: &mut Painting) -> Option<Done> {
    match painting.save() {
        Ok(word) => {
            painting.said = Some("saved".to_string());
            Some(Done::Picture(word))
        }
        Err(why) => {
            painting.said = Some(why);
            None
        }
    }
}

/// The keys of a painting studio.
fn keys(ui: &mut egui::Ui, painting: &mut Painting, done: &mut Option<Done>) {
    if ui.ctx().wants_keyboard_input() || !ui.ui_contains_pointer() {
        return;
    }
    use egui::{Key, Modifiers};
    let consume = |ui: &mut egui::Ui, modifiers: Modifiers, key: Key| ui.input_mut(|input| input.consume_key(modifiers, key));
    if consume(ui, Modifiers::COMMAND, Key::Z) {
        painting.undo();
    }
    if consume(ui, Modifiers::COMMAND, Key::Y) || consume(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) {
        painting.redo();
    }
    if consume(ui, Modifiers::COMMAND, Key::S) {
        *done = save_painting(painting);
    }
    if consume(ui, Modifiers::COMMAND, Key::C) {
        painting.copy(false);
    }
    if consume(ui, Modifiers::COMMAND, Key::X) {
        painting.copy(true);
    }
    if consume(ui, Modifiers::COMMAND, Key::V)
        && let Some(picture) = painting.clipboard.clone()
    {
        painting.paste(picture);
    }
    if consume(ui, Modifiers::COMMAND, Key::A) {
        painting.drop_floating();
        painting.selection = Some([0, 0, painting.wide() - 1, painting.high() - 1]);
    }
    if consume(ui, Modifiers::COMMAND, Key::D) {
        painting.drop_floating();
        painting.selection = None;
    }
    // (Copy and paste as the system says them, too.)
    let events: Vec<egui::Event> = ui.input(|input| input.events.clone());
    for event in events {
        match event {
            egui::Event::Copy => painting.copy(false),
            egui::Event::Cut => painting.copy(true),
            _ => {}
        }
    }
    for (tool, _, key, _) in Tool::ALL {
        if consume(ui, Modifiers::NONE, key) {
            painting.drop_floating();
            painting.tool = tool;
        }
    }
    if consume(ui, Modifiers::NONE, Key::X) {
        std::mem::swap(&mut painting.primary, &mut painting.secondary);
    }
    if consume(ui, Modifiers::NONE, Key::OpenBracket) {
        painting.brush = painting.brush.saturating_sub(1).max(1);
    }
    if consume(ui, Modifiers::NONE, Key::CloseBracket) {
        painting.brush = (painting.brush + 1).min(16);
    }
    if consume(ui, Modifiers::NONE, Key::Delete) || consume(ui, Modifiers::NONE, Key::Backspace) {
        if painting.floating.take().is_some() {
            painting.floating_texture = None;
        } else {
            painting.keep("clear");
            painting.clear_chosen();
        }
    }
    if consume(ui, Modifiers::NONE, Key::Escape) {
        painting.drop_floating();
        painting.selection = None;
    }
    if consume(ui, Modifiers::NONE, Key::Enter) {
        painting.drop_floating();
    }
    // (The arrows: the move tool shifts by a pixel (Maj: by 8); else another step, another layer.)
    let shift = ui.input(|input| input.modifiers.shift);
    let step = if shift { 8 } else { 1 };
    let arrows = [(Key::ArrowLeft, (-step, 0)), (Key::ArrowRight, (step, 0)), (Key::ArrowUp, (0, -step)), (Key::ArrowDown, (0, step))];
    for (key, (dx, dy)) in arrows {
        if !ui.input(|input| input.key_pressed(key)) {
            continue;
        }
        if painting.tool == Tool::Move {
            if painting.floating.is_none() {
                painting.keep("move");
                painting.lift();
            }
            if let Some((_, at)) = &mut painting.floating {
                *at = (at.0 + dx, at.1 + dy);
            }
        } else if dx != 0 {
            let count = painting.count();
            painting.at = if dx < 0 { (painting.at + count - 1) % count } else { (painting.at + 1) % count };
        } else {
            let count = painting.layers.len();
            painting.layer = if dy < 0 { (painting.layer + 1).min(count - 1) } else { painting.layer.saturating_sub(1) };
        }
    }
    if consume(ui, Modifiers::NONE, Key::Plus) || consume(ui, Modifiers::NONE, Key::Equals) {
        painting.zoom = (painting.zoom * 1.25).min(64.0);
        painting.zoom_fits = false;
    }
    if consume(ui, Modifiers::NONE, Key::Minus) {
        painting.zoom = (painting.zoom * 0.8).max(1.0);
        painting.zoom_fits = false;
    }
}
