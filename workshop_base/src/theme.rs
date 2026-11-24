//! The Workshop's look, in one place: a dark studio (as Unity or Blender),
//! dense, its panels a step lighter than what is between them, one accent
//! a space, a blue bar of state.

use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, TextStyle};

/// Between the panels, under all.
pub const GAP: Color32 = Color32::from_rgb(0x11, 0x11, 0x11);
/// The window's ground; a panel; a panel's tabs; the bars.
pub const GROUND: Color32 = Color32::from_rgb(0x1D, 0x1D, 0x1D);
pub const PANEL: Color32 = Color32::from_rgb(0x25, 0x25, 0x25);
pub const TABS: Color32 = Color32::from_rgb(0x1F, 0x1F, 0x1F);
pub const MENU: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
pub const TOOLS: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
/// A field, a well: darker than its panel.
pub const FIELD: Color32 = Color32::from_rgb(0x1D, 0x1D, 0x1D);
/// What is hovered; what is chosen.
pub const HOVERED: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
pub const CHOSEN: Color32 = Color32::from_rgb(0x2C, 0x5D, 0x87);
/// The words: as they are, strong, weak, weaker.
pub const TEXT: Color32 = Color32::from_rgb(0xD4, 0xD4, 0xD4);
pub const STRONG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const WEAK: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x8A);
pub const FAINT: Color32 = Color32::from_rgb(0x6E, 0x6E, 0x6E);
/// The bar of state.
pub const STATE: Color32 = Color32::from_rgb(0x00, 0x7A, 0xCC);
/// Each space's accent.
pub const HOME: Color32 = Color32::from_rgb(0x8A, 0xA4, 0xFF);
pub const FILM: Color32 = Color32::from_rgb(0xE0, 0xA9, 0x3A);
pub const DECOR: Color32 = Color32::from_rgb(0x3B, 0xA7, 0xD9);
pub const ASSETS: Color32 = Color32::from_rgb(0xA7, 0x8B, 0xFA);
pub const DUNGEON: Color32 = Color32::from_rgb(0x5C, 0xC9, 0x7B);
pub const GAME: Color32 = Color32::from_rgb(0xF4, 0x72, 0xB6);
pub const EMULATOR: Color32 = Color32::from_rgb(0x2D, 0xD4, 0xBF);
/// What is wrong.
/// What is pointed at in one panel, shown in the others.
pub const POINTED: Color32 = Color32::from_rgb(0x4F, 0xC1, 0xFF);
pub const WRONG: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71);

/// The sizes of the words: the interface's, small, a title's.
pub const BODY: f32 = 12.5;
pub const SMALL: f32 = 11.0;
pub const TITLE: f32 = 15.0;

/// The look laid on the window: its colours, its sizes, its fonts.
pub fn apply(context: &egui::Context) {
    fonts(context);
    context.style_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::new(SMALL, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(BODY, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(BODY, FontFamily::Proportional)),
            (TextStyle::Heading, FontId::new(TITLE, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(BODY, FontFamily::Monospace)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        style.spacing.interact_size.y = 22.0;
        let visuals = &mut style.visuals;
        *visuals = egui::Visuals::dark();
        visuals.panel_fill = PANEL;
        visuals.window_fill = TOOLS;
        visuals.extreme_bg_color = FIELD;
        visuals.faint_bg_color = TABS;
        visuals.override_text_color = None;
        visuals.selection.bg_fill = CHOSEN;
        visuals.selection.stroke = Stroke::new(1.0_f32, STRONG);
        visuals.window_stroke = Stroke::new(1.0_f32, GAP);
        visuals.window_corner_radius = egui::CornerRadius::same(4);
        visuals.menu_corner_radius = egui::CornerRadius::same(3);
        for (widget, fill) in [
            (&mut visuals.widgets.noninteractive, PANEL),
            (&mut visuals.widgets.inactive, FIELD),
            (&mut visuals.widgets.hovered, HOVERED),
            (&mut visuals.widgets.active, CHOSEN),
            (&mut visuals.widgets.open, HOVERED),
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.corner_radius = egui::CornerRadius::same(3);
            widget.bg_stroke = Stroke::NONE;
        }
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, STRONG);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, STRONG);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, GAP);
    });
}

/// The system's own font for the words, if it has a clean one; egui's
/// stays behind it, for its symbols and the monospace.
fn fonts(context: &egui::Context) {
    let found = ["C:/Windows/Fonts/segoeui.ttf", "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"].iter().find_map(|path| std::fs::read(path).ok());
    let Some(bytes) = found else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("interface".to_string(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "interface".to_string());
    context.set_fonts(fonts);
}

/// Words of a size and colour.
pub fn text(said: impl Into<String>, size: f32, colour: Color32) -> egui::RichText {
    egui::RichText::new(said).size(size).color(colour)
}

/// Weak words, smaller.
pub fn weak(said: impl Into<String>) -> egui::RichText {
    text(said, SMALL, WEAK)
}
