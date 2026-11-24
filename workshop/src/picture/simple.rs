//! A picture painted simply, as a beginner would.

use std::time::Instant;

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use super::canvas::{canvas, checkered, fitted};
use super::editor::save_painting;
use super::panels::swatch;
use super::{PALETTE, Painting, Tool};
use crate::done::Done;
use crate::theme;

/// Where the parts lie under the first row: the tools on the left, the
/// colours on the right, the canvas between, the last rows under them.
#[derive(Debug, PartialEq)]
pub(super) struct Rooms {
    pub tools: Rect,
    pub side: Rect,
    pub canvas: Rect,
    pub bottom: Rect,
}

impl Rooms {
    pub(super) fn of(room: Rect) -> Rooms {
        let tools = Rect::from_min_size(room.min + vec2(10.0, 0.0), vec2(150.0, room.height() - 130.0));
        let side = Rect::from_min_max(pos2(room.max.x - 260.0, room.min.y), pos2(room.max.x - 8.0, room.max.y - 130.0));
        let canvas = Rect::from_min_max(pos2(tools.max.x + 10.0, room.min.y), pos2(side.min.x - 10.0, room.max.y - 130.0));
        let bottom = Rect::from_min_max(pos2(room.min.x + 10.0, room.max.y - 124.0), room.max - vec2(8.0, 2.0));
        Rooms { tools, side, canvas, bottom }
    }
}

/// A large button, filled when chosen.
fn big(said: &str, chosen: bool) -> egui::Button<'static> {
    egui::Button::new(RichText::new(said).size(theme::BODY + 1.0).strong().color(if chosen { theme::GROUND } else { theme::TEXT }))
        .fill(if chosen { theme::ASSETS } else { theme::FIELD })
        .min_size(vec2(96.0, 34.0))
}

/// The painting for a beginner: begin from a size or a shape, draw with a few tools and the game's
/// colours, add an outline and relief, animate it in a click, name it, write it.
pub(super) fn paint_simply(ui: &mut egui::Ui, painting: &mut Painting, context: &egui::Context) -> Option<Done> {
    let mut done = None;
    if !matches!(painting.tool, Tool::Pencil | Tool::Eraser | Tool::Fill | Tool::Picker) {
        painting.tool = Tool::Pencil;
    }
    // (Its keys: undo, redo, written.)
    if ui.ui_contains_pointer() && !ui.ctx().wants_keyboard_input() {
        let (undo, redo, save) = ui.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
            )
        });
        if undo {
            painting.undo();
        }
        if redo {
            painting.redo();
        }
        if save {
            done = save_painting(painting);
        }
    }
    ui.horizontal_wrapped(|ui| start(ui, painting));
    ui.add_space(6.0);
    let room = ui.available_rect_before_wrap();
    let rooms = Rooms::of(room);
    let mut column = ui.new_child(egui::UiBuilder::new().max_rect(rooms.tools).layout(egui::Layout::top_down_justified(egui::Align::Min)));
    draw(&mut column, painting);
    canvas(ui, painting, rooms.canvas, context);
    let mut right = ui.new_child(egui::UiBuilder::new().max_rect(rooms.side).layout(egui::Layout::top_down(egui::Align::Min)));
    colours(&mut right, painting, context);
    // --- 3. Animate, name, write ---
    let mut under = ui.new_child(egui::UiBuilder::new().max_rect(rooms.bottom).layout(egui::Layout::top_down(egui::Align::Min)));
    under.horizontal_wrapped(|ui| animate(ui, painting));
    under.horizontal(|ui| images_and_name(ui, painting, context, &mut done));
    under.horizontal(|ui| {
        ui.label(theme::text("In a film:", theme::SMALL, theme::WEAK));
        for line in painting.lines() {
            ui.add(egui::Label::new(RichText::new(line).monospace().size(theme::SMALL - 1.0).color(Color32::from_rgb(0xDC, 0xDC, 0xAA))).selectable(true));
        }
        if let Some(said) = &painting.said {
            ui.label(theme::text(said, theme::SMALL, theme::FAINT));
        }
        ui.label(theme::text("· all the tools: \"Expert\" at the top", theme::SMALL - 1.0, theme::FAINT));
    });
    ui.allocate_rect(room, Sense::hover());
    done
}

/// 1. Begin: an empty image of a size, or a shape.
fn start(ui: &mut egui::Ui, painting: &mut Painting) {
    ui.add_space(10.0);
    ui.label(theme::text("1. Start", theme::TITLE, theme::STRONG));
    ui.add_space(8.0);
    for (said, side) in [("Small 16×16", 16), ("Medium 32×32", 32), ("Large 64×64", 64)] {
        if ui.add(big(said, false)).on_hover_text("an empty image of that size").clicked() {
            painting.begin(side);
        }
    }
    ui.separator();
    ui.label(theme::text("a shape:", theme::SMALL, theme::WEAK));
    for kind in ["Round", "Box", "Star", "Drop"] {
        if ui.add(big(kind, false)).on_hover_text("drawn in the chosen colour, its outline around it").clicked() {
            painting.shape(kind);
        }
    }
}

/// 2. Draw: a few tools, the pencil's size, the mirror, undone, redone; the finishing.
fn draw(column: &mut egui::Ui, painting: &mut Painting) {
    column.label(theme::text("2. Draw", theme::TITLE, theme::STRONG));
    column.add_space(4.0);
    for (tool, said, help) in [
        (Tool::Pencil, "Pencil", "draws (right-click: the 2nd colour)"),
        (Tool::Eraser, "Eraser", "erases"),
        (Tool::Fill, "Fill", "fills an area with a colour"),
        (Tool::Picker, "Eyedropper", "takes a colour from the image"),
    ] {
        if column.add(big(said, painting.tool == tool)).on_hover_text(help).clicked() {
            painting.tool = tool;
        }
    }
    column.add_space(6.0);
    column.label(theme::text("Pencil size", theme::SMALL, theme::WEAK));
    column.horizontal(|ui| {
        for size in [1, 2, 4] {
            if ui
                .add(
                    egui::Button::new(RichText::new(format!("{size}")).strong())
                        .fill(if painting.brush == size { theme::ASSETS } else { theme::FIELD })
                        .min_size(vec2(40.0, 30.0)),
                )
                .clicked()
            {
                painting.brush = size;
            }
        }
    });
    column.add_space(6.0);
    let mirror = painting.mirror_x;
    if column
        .add(big(if mirror { "Mirror: on" } else { "Mirror: off" }, mirror))
        .on_hover_text("what is drawn is drawn on the other side too: a face, a vase, a Pokémon from the front")
        .clicked()
    {
        painting.mirror_x = !mirror;
    }
    column.add_space(6.0);
    column.horizontal(|ui| {
        if ui
            .add_enabled(!painting.undone.is_empty(), egui::Button::new(RichText::new("Undo").strong()).min_size(vec2(68.0, 30.0)))
            .on_hover_text("Ctrl+Z")
            .clicked()
        {
            painting.undo();
        }
        if ui
            .add_enabled(!painting.redone.is_empty(), egui::Button::new(RichText::new("Redo").strong()).min_size(vec2(68.0, 30.0)))
            .on_hover_text("Ctrl+Y")
            .clicked()
        {
            painting.redo();
        }
    });
    column.add_space(6.0);
    finishing(column, painting);
}

/// An outline, a relief, all erased.
fn finishing(column: &mut egui::Ui, painting: &mut Painting) {
    column.label(theme::text("Finishing", theme::SMALL, theme::WEAK));
    if column.add(big("Outline", false)).on_hover_text("a dark stroke around the drawing, like the game's objects").clicked() {
        let primary = painting.primary;
        painting.primary = Color32::from_rgb(0x18, 0x10, 0x10);
        painting.outline();
        painting.primary = primary;
    }
    if column.add(big("Relief", false)).on_hover_text("the light at the top left, the shadow at the bottom right, as in the game").clicked() {
        painting.relief();
    }
    if column.add(big("Erase All", false)).clicked() {
        painting.keep("erase all");
        painting.selection = None;
        painting.clear_chosen();
    }
}

/// The colours: the one in hand, the game's, large; the picture at its actual size.
fn colours(right: &mut egui::Ui, painting: &mut Painting, context: &egui::Context) {
    right.label(theme::text("Colour", theme::SMALL, theme::WEAK));
    right.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(56.0, 40.0), Sense::hover());
        ui.painter().rect_filled(rect, 4.0, painting.primary);
        ui.painter().rect_stroke(rect, 4.0, Stroke::new(2.0_f32, Color32::WHITE), StrokeKind::Outside);
        ui.vertical(|ui| {
            ui.label(theme::text("click: choose", theme::SMALL - 1.0, theme::FAINT));
            ui.horizontal(|ui| {
                ui.label(theme::text("other:", theme::SMALL - 1.0, theme::FAINT));
                egui::color_picker::color_edit_button_srgba(ui, &mut painting.primary, egui::color_picker::Alpha::Opaque);
            });
        });
    });
    right.add_space(4.0);
    right.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
        for [r, g, b] in PALETTE {
            let colour = Color32::from_rgb(r, g, b);
            let response = swatch(ui, colour, colour == painting.primary, 26.0);
            if response.clicked() {
                painting.primary = colour;
                if painting.tool == Tool::Eraser || painting.tool == Tool::Picker {
                    painting.tool = Tool::Pencil;
                }
            }
            if response.secondary_clicked() {
                painting.secondary = colour;
            }
        }
    });
    right.add_space(8.0);
    right.label(theme::text("Actual Size", theme::SMALL, theme::WEAK));
    right.horizontal(|ui| {
        let at = if painting.playing { painting.playing_step() } else { painting.at };
        let texture = painting.composite_texture(context, at);
        for scale in [1.0, 2.0, 3.0] {
            let size = texture.size_vec2() * scale;
            let (rect, _) = ui.allocate_exact_size(size.max(vec2(8.0, 8.0)), Sense::hover());
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(0x48, 0x70, 0x48));
            ui.painter().image(texture.id(), Rect::from_min_size(rect.min, size), Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
        }
    });
}

/// 3. Animate: a movement in a click, played, how fast.
fn animate(ui: &mut egui::Ui, painting: &mut Painting) {
    ui.label(theme::text("3. Animate", theme::TITLE, theme::STRONG));
    ui.label(theme::text("for me:", theme::SMALL, theme::WEAK));
    for (kind, help) in [
        ("Bounce", "it hops on the spot"),
        ("Blink", "it fades and comes back"),
        ("Shake", "it shakes from left to right"),
        ("Grow", "it swells and shrinks"),
        ("Spin", "it spins round"),
    ] {
        if ui.add(big(kind, false)).on_hover_text(format!("{help} (from the first image)")).clicked() {
            painting.animate(kind);
        }
    }
    ui.separator();
    let count = painting.count();
    if ui.add(egui::Button::new(RichText::new(if painting.playing { "⏸ Stop" } else { "▶ Play" }).strong()).min_size(vec2(90.0, 34.0))).clicked() {
        painting.playing = !painting.playing;
        painting.began = Instant::now();
    }
    ui.label(theme::text("Slow", theme::SMALL, theme::WEAK));
    ui.spacing_mut().slider_width = 100.0;
    if ui.add(egui::Slider::new(&mut painting.pace, 0.0..=1.0).show_value(false)).changed() {
        painting.ticks = vec![painting.pace_ticks(); count];
        painting.saved = false;
    }
    ui.label(theme::text("Fast", theme::SMALL, theme::WEAK));
}

/// Its images (one more, one less), its name; written or closed (what is done).
fn images_and_name(ui: &mut egui::Ui, painting: &mut Painting, context: &egui::Context, done: &mut Option<Done>) {
    ui.label(theme::text(format!("Images: {}", painting.count()), theme::SMALL, theme::WEAK));
    egui::ScrollArea::horizontal().id_salt("easy images").max_width(ui.available_width() - 520.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            for at in 0..painting.count() {
                let (rect, response) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
                checkered(ui.painter(), rect, 6.0);
                let texture = painting.composite_texture(context, at);
                ui.painter().image(texture.id(), fitted(texture.size_vec2(), rect.shrink(2.0)), Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
                if at == painting.at {
                    ui.painter().rect_stroke(rect, 0.0, Stroke::new(2.0_f32, theme::ASSETS), StrokeKind::Outside);
                }
                if response.clicked() {
                    painting.playing = false;
                    painting.at = at;
                }
            }
        });
    });
    if ui.button("+ Copy the Image").on_hover_text("one more image, the same, to change it a little").clicked() {
        painting.add_step(true);
        painting.ticks = vec![painting.pace_ticks(); painting.count()];
    }
    if painting.count() > 1 && ui.button("Remove").clicked() {
        painting.remove_step();
    }
    ui.separator();
    ui.label(theme::text("Name", theme::SMALL, theme::WEAK));
    ui.add(egui::TextEdit::singleline(&mut painting.word).desired_width(150.0).hint_text("my_object"));
    let said = if painting.saved { "✔ Saved" } else { "Save" };
    if ui.add(egui::Button::new(RichText::new(said).strong().color(theme::GROUND)).fill(theme::ASSETS).min_size(vec2(110.0, 30.0))).clicked() {
        *done = save_painting(painting);
    }
    if ui.button("Close").clicked() {
        *done = Some(Done::Closed);
    }
}
