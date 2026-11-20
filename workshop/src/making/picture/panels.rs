//! The panels beside the canvas: the tools, the colours, the layers, the
//! expert's settings, the picture's lines.

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use image::RgbaImage;

use super::canvas::{checkered, fitted};
use super::pixels::composite;
use super::{BLENDS, Layer, PALETTE, Painting, Tool};
use crate::theme;

/// A colour's swatch: clicked (left: primary; right: secondary).
pub(super) fn swatch(ui: &mut egui::Ui, colour: Color32, chosen: bool, side: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    if colour.a() < 255 {
        checkered(ui.painter(), rect, side / 2.0);
    }
    ui.painter().rect_filled(rect, 2.0, colour);
    if chosen {
        ui.painter().rect_stroke(rect, 2.0, Stroke::new(2.0_f32, Color32::WHITE), StrokeKind::Outside);
    }
    let [r, g, b, _] = colour.to_srgba_unmultiplied();
    response.on_hover_text(format!("#{r:02X}{g:02X}{b:02X} — click: primary; right-click: secondary"))
}

pub(super) fn hex_of(colour: Color32) -> String {
    let [r, g, b, _] = colour.to_srgba_unmultiplied();
    format!("{r:02X}{g:02X}{b:02X}")
}

pub(super) fn colour_of_hex(said: &str) -> Option<Color32> {
    let said = said.trim().trim_start_matches('#');
    if said.len() != 6 {
        return None;
    }
    let part = |at: usize| u8::from_str_radix(&said[at..at + 2], 16).ok();
    Some(Color32::from_rgb(part(0)?, part(2)?, part(4)?))
}

/// The tools, and what each takes.
pub(super) fn tools_panel(ui: &mut egui::Ui, painting: &mut Painting) {
    ui.spacing_mut().item_spacing.y = 2.0;
    for (tool, said, _, key) in Tool::ALL {
        let label = RichText::new(format!("{said}  {key}")).size(theme::SMALL + 0.5);
        if ui.selectable_label(painting.tool == tool, label).on_hover_text(tool.about()).clicked() {
            painting.drop_floating();
            painting.tool = tool;
        }
    }
    ui.add_space(6.0);
    ui.separator();
    match painting.tool {
        Tool::Pencil | Tool::Eraser | Tool::Shade | Tool::Dither => {
            ui.label(theme::text("Size  [ ]", theme::SMALL, theme::WEAK));
            ui.add(egui::Slider::new(&mut painting.brush, 1..=16).suffix(" px"));
            if painting.tool == Tool::Pencil {
                ui.checkbox(&mut painting.perfect, "Pixel Perfect").on_hover_text("the L-shaped corners of strokes taken out (size 1)");
            }
        }
        Tool::Rectangle | Tool::Ellipse => {
            ui.checkbox(&mut painting.filled, "Filled");
        }
        Tool::Fill => {
            ui.checkbox(&mut painting.everywhere, "Everywhere").on_hover_text("all of that colour, not only what touches");
        }
        _ => {}
    }
    ui.add_space(4.0);
    ui.checkbox(&mut painting.mirror_x, "Mirror ↔");
    ui.checkbox(&mut painting.mirror_y, "Mirror ↕");
    ui.checkbox(&mut painting.grid, "Grid");
    ui.checkbox(&mut painting.onion, "Onion Skin").on_hover_text("the images before (red) and after (green), faintly, underneath");
    if painting.onion {
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut painting.onion_range).range(1..=4).prefix("±"));
            ui.add(egui::DragValue::new(&mut painting.onion_strength).range(0.05..=1.0).speed(0.01));
        });
    }
    ui.checkbox(&mut painting.tiled, "Tiled").on_hover_text("the image repeated around, to see a pattern join up");
    ui.add_space(6.0);
    ui.label(theme::text(format!("Zoom ×{:.0}", painting.zoom), theme::SMALL, theme::WEAK));
    ui.horizontal(|ui| {
        if ui.small_button("−").clicked() {
            painting.zoom = (painting.zoom * 0.8).max(1.0);
            painting.zoom_fits = false;
        }
        if ui.small_button("+").clicked() {
            painting.zoom = (painting.zoom * 1.25).min(64.0);
            painting.zoom_fits = false;
        }
        if ui.small_button("Fit").clicked() {
            painting.zoom = 0.0;
            painting.pan = Vec2::ZERO;
        }
    });
    ui.add_space(6.0);
    ui.label(theme::text("Wheel: zoom\nMiddle, Space: pan\nX: swap\nAlt+click: eyedropper", theme::SMALL - 1.0, theme::FAINT));
}

/// The colours, the true size, the layers, how a film writes it.
pub(super) fn side_panel(ui: &mut egui::Ui, painting: &mut Painting, context: &egui::Context) {
    colours(ui, painting);
    ui.add_space(6.0);
    ui.separator();
    // --- At its true size ---
    ui.label(theme::text("ACTUAL SIZE", theme::SMALL, theme::WEAK));
    ui.horizontal(|ui| {
        let at = if painting.playing { painting.playing_step() } else { painting.at };
        let texture = painting.composite_texture(context, at);
        for scale in [1.0, 2.0] {
            let size = texture.size_vec2() * scale;
            let (rect, _) = ui.allocate_exact_size(size.max(vec2(8.0, 8.0)), Sense::hover());
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(0x48, 0x70, 0x48));
            ui.painter().image(texture.id(), Rect::from_min_size(rect.min, size), Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
        }
    });
    ui.add_space(6.0);
    ui.separator();
    // --- The layers, the top one first ---
    ui.horizontal(|ui| layers_head(ui, painting));
    for layer in (0..painting.layers.len()).rev() {
        ui.horizontal(|ui| layer_row(ui, painting, context, layer));
    }
    ui.horizontal(|ui| blend_row(ui, painting));
    ui.label(theme::text("double-click: rename it", theme::SMALL - 1.5, theme::FAINT));
    ui.add_space(6.0);
    ui.separator();
    experts(ui, painting);
    ui.add_space(6.0);
    ui.separator();
    ui.label(theme::text("IN A FILM", theme::SMALL, theme::WEAK));
    for line in painting.lines() {
        ui.add(egui::Label::new(RichText::new(line).monospace().size(theme::SMALL - 1.0).color(Color32::from_rgb(0xDC, 0xDC, 0xAA))).selectable(true).wrap());
    }
}

/// The colours: the primary and the secondary (by a picker, by their hex), the recent ones, the palette, those of the image.
fn colours(ui: &mut egui::Ui, painting: &mut Painting) {
    ui.label(theme::text("COLOURS", theme::SMALL, theme::WEAK));
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::hover());
        let back = Rect::from_min_size(rect.min + vec2(16.0, 16.0), vec2(28.0, 28.0));
        let front = Rect::from_min_size(rect.min, vec2(28.0, 28.0));
        ui.painter().rect_filled(back, 2.0, painting.secondary);
        ui.painter().rect_stroke(back, 2.0, Stroke::new(1.0_f32, theme::GAP), StrokeKind::Outside);
        ui.painter().rect_filled(front, 2.0, painting.primary);
        ui.painter().rect_stroke(front, 2.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Outside);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                egui::color_picker::color_edit_button_srgba(ui, &mut painting.primary, egui::color_picker::Alpha::OnlyBlend);
                let mut said = hex_of(painting.primary);
                if ui.add(egui::TextEdit::singleline(&mut said).desired_width(60.0).font(egui::TextStyle::Monospace)).changed()
                    && let Some(colour) = colour_of_hex(&said)
                {
                    painting.primary = colour;
                }
            });
            ui.horizontal(|ui| {
                egui::color_picker::color_edit_button_srgba(ui, &mut painting.secondary, egui::color_picker::Alpha::OnlyBlend);
                if ui.small_button("Swap  X").on_hover_text("swap the primary and the secondary").clicked() {
                    std::mem::swap(&mut painting.primary, &mut painting.secondary);
                }
            });
        });
    });
    if !painting.recent.is_empty() {
        ui.label(theme::text("Recent", theme::SMALL - 1.0, theme::FAINT));
        let recent = painting.recent.clone();
        picked_among(ui, painting, recent);
    }
    ui.horizontal(|ui| {
        ui.label(theme::text("Palette", theme::SMALL - 1.0, theme::FAINT));
        if ui.small_button("+").on_hover_text("add the primary").clicked() && !painting.palette.contains(&painting.primary) {
            painting.palette.push(painting.primary);
        }
        if ui.small_button("−").on_hover_text("remove the primary").clicked() {
            let primary = painting.primary;
            painting.palette.retain(|colour| *colour != primary);
        }
        if ui.small_button("Game").on_hover_text("back to the game's colours").clicked() {
            painting.palette = PALETTE.iter().map(|[r, g, b]| Color32::from_rgb(*r, *g, *b)).collect();
        }
    });
    let palette = painting.palette.clone();
    picked_among(ui, painting, palette);
    let own = painting.own_colours();
    if !own.is_empty() {
        ui.label(theme::text("In the image", theme::SMALL - 1.0, theme::FAINT));
        picked_among(ui, painting, own);
    }
}

/// Colours as small swatches: a click makes one the primary, a right click the secondary.
fn picked_among(ui: &mut egui::Ui, painting: &mut Painting, colours: Vec<Color32>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
        for colour in colours {
            let response = swatch(ui, colour, colour == painting.primary, 17.0);
            if response.clicked() {
                painting.primary = colour;
            }
            if response.secondary_clicked() {
                painting.secondary = colour;
            }
        }
    });
}

/// The layers' head: one more, duplicated, merged down, taken out.
fn layers_head(ui: &mut egui::Ui, painting: &mut Painting) {
    ui.label(theme::text("LAYERS", theme::SMALL, theme::WEAK));
    if ui.small_button("+").on_hover_text("an empty layer above").clicked() {
        painting.keep("new layer");
        let (wide, high, count) = (painting.wide() as u32, painting.high() as u32, painting.count());
        let name = format!("Layer {}", painting.layers.len() + 1);
        painting.layers.insert(painting.layer + 1, Layer { name, visible: true, opacity: 255, blend: 0, cels: vec![RgbaImage::new(wide, high); count] });
        painting.layer += 1;
        painting.all_changed();
    }
    if ui.small_button("Duplicate").clicked() {
        painting.keep("duplicate the layer");
        let mut copy = painting.layers[painting.layer].clone();
        copy.name = format!("{} (copy)", copy.name);
        painting.layers.insert(painting.layer + 1, copy);
        painting.layer += 1;
        painting.all_changed();
    }
    if painting.layer > 0 && ui.small_button("Merge ↓").on_hover_text("into the one below").clicked() {
        merged_down(painting);
    }
    if painting.layers.len() > 1 && ui.small_button("Remove").clicked() {
        painting.keep("remove the layer");
        painting.layers.remove(painting.layer);
        painting.settle();
    }
}

/// The layer chosen merged into the one below, each image.
fn merged_down(painting: &mut Painting) {
    painting.keep("merge");
    let (top, below) = (painting.layer, painting.layer - 1);
    for at in 0..painting.count() {
        let merged = composite(&[Layer { visible: true, ..painting.layers[below].clone() }, Layer { visible: true, ..painting.layers[top].clone() }], at);
        painting.layers[below].cels[at] = merged;
    }
    painting.layers[below].opacity = 255;
    painting.layers.remove(top);
    painting.layer = below;
    painting.all_changed();
}

/// A layer's row: seen or not, its picture, its name (renamed by a double
/// click); chosen: how much it covers, moved up or down.
fn layer_row(ui: &mut egui::Ui, painting: &mut Painting, context: &egui::Context, layer: usize) {
    let chosen = layer == painting.layer;
    let (eye, eye_response) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    crate::icons::eye(ui.painter(), eye, painting.layers[layer].visible, if painting.layers[layer].visible { theme::TEXT } else { theme::FAINT });
    if eye_response.on_hover_text("show it or not").clicked() {
        painting.layers[layer].visible = !painting.layers[layer].visible;
        painting.composites.clear();
    }
    let texture = painting.texture(context, layer, painting.at);
    let (thumb, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
    checkered(ui.painter(), thumb, 4.0);
    ui.painter().image(texture.id(), fitted(texture.size_vec2(), thumb), Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
    if painting.naming_layer == Some(layer) {
        let field = ui.add(egui::TextEdit::singleline(&mut painting.layers[layer].name).desired_width(90.0));
        if field.lost_focus() {
            painting.naming_layer = None;
        } else {
            field.request_focus();
        }
    } else {
        let name = ui.selectable_label(chosen, RichText::new(&painting.layers[layer].name).size(theme::SMALL));
        if name.clicked() {
            painting.drop_floating();
            painting.layer = layer;
        }
        if name.double_clicked() {
            painting.naming_layer = Some(layer);
        }
    }
    if chosen {
        chosen_layer(ui, painting, layer);
    }
}

/// The chosen layer's own: how much it covers, moved up or down.
fn chosen_layer(ui: &mut egui::Ui, painting: &mut Painting, layer: usize) {
    let mut opacity = painting.layers[layer].opacity;
    ui.spacing_mut().slider_width = 50.0;
    if ui.add(egui::Slider::new(&mut opacity, 0..=255).show_value(false)).on_hover_text("how much it covers").changed() {
        painting.layers[layer].opacity = opacity;
        painting.composites.clear();
        painting.saved = false;
    }
    if layer + 1 < painting.layers.len() && ui.small_button("↑").clicked() {
        painting.keep("move the layer up");
        painting.layers.swap(layer, layer + 1);
        painting.layer = layer + 1;
        painting.all_changed();
    }
    if layer > 0 && ui.small_button("↓").clicked() {
        painting.keep("move the layer down");
        painting.layers.swap(layer, layer - 1);
        painting.layer = layer - 1;
        painting.all_changed();
    }
}

/// How the chosen layer is laid over those below.
fn blend_row(ui: &mut egui::Ui, painting: &mut Painting) {
    ui.label(theme::text("Blend", theme::SMALL, theme::WEAK));
    let layer = painting.layer;
    let mut blend = painting.layers[layer].blend;
    egui::ComboBox::from_id_salt("layer blend").selected_text(BLENDS[blend as usize % BLENDS.len()]).show_ui(ui, |ui| {
        for (at, name) in BLENDS.iter().enumerate() {
            ui.selectable_value(&mut blend, at as u8, *name);
        }
    });
    if blend != painting.layers[layer].blend {
        painting.keep("layer blend");
        painting.layers[layer].blend = blend;
        painting.all_changed();
    }
}

/// An expert's: colours adjusted, turned and scaled, fewer colours, the palette from the picture.
fn experts(ui: &mut egui::Ui, painting: &mut Painting) {
    egui::CollapsingHeader::new(theme::text("ADJUST THE COLOURS", theme::SMALL, theme::WEAK)).id_salt("adjust").show(ui, |ui| {
        egui::Grid::new("adjustments").num_columns(2).spacing([6.0, 3.0]).show(ui, |ui| {
            for (name, at, range) in [("Hue", 0, -180.0..=180.0), ("Saturation", 1, -1.0..=1.0), ("Lightness", 2, -1.0..=1.0), ("Contrast", 3, -1.0..=1.0)] {
                ui.label(theme::text(name, theme::SMALL, theme::WEAK));
                ui.spacing_mut().slider_width = 120.0;
                ui.add(egui::Slider::new(&mut painting.adjust[at], range).max_decimals(2));
                ui.end_row();
            }
        });
        ui.checkbox(&mut painting.adjust_all, "all the layer's images");
        if ui.button("Apply").clicked() {
            painting.adjusted();
        }
    });
    egui::CollapsingHeader::new(theme::text("ROTATE, SCALE", theme::SMALL, theme::WEAK)).id_salt("rotate").show(ui, |ui| {
        egui::Grid::new("rotate and scale").num_columns(2).spacing([6.0, 3.0]).show(ui, |ui| {
            ui.label(theme::text("Angle", theme::SMALL, theme::WEAK));
            ui.add(egui::DragValue::new(&mut painting.turn).range(-360.0..=360.0).suffix("°"));
            ui.end_row();
            ui.label(theme::text("Scale", theme::SMALL, theme::WEAK));
            ui.add(egui::DragValue::new(&mut painting.scale).range(0.05..=16.0).speed(0.01).suffix("×"));
            ui.end_row();
        });
        ui.label(theme::text("the selection, else the image; pixels kept whole", theme::SMALL - 1.5, theme::FAINT));
        if ui.button("Apply").clicked() {
            painting.turned();
        }
    });
    egui::CollapsingHeader::new(theme::text("THE IMAGE'S PALETTE", theme::SMALL, theme::WEAK)).id_salt("the image's palette").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(theme::text("Reduce to", theme::SMALL, theme::WEAK));
            ui.add(egui::DragValue::new(&mut painting.colours_at_most).range(2..=64));
            if ui.button("colours").on_hover_text("the layer's colours brought down to so few (the nearest of the most used)").clicked() {
                painting.fewer_colours();
            }
        });
        if ui.button("Take the Image's Palette").on_hover_text("its colours become the palette").clicked() {
            painting.palette = painting.own_colours();
        }
    });
}
