//! Tools › Convert an Image (Tilequant): any PNG made to the console's
//! rules (tiles of 8, each one palette of 16 colours, so many palettes at
//! most, see `media::quant`), written as `<name>_quant.png` beside it,
//! before and after side by side.

use eframe::egui::{self, RichText, TextureHandle};

use workshop_base::theme;

/// The tool's window: the picture's path, what is asked, what it became.
pub struct QuantTool {
    path: String,
    palettes: usize,
    dither: bool,
    said: String,
    /// Before and after, and their size.
    shown: Option<(TextureHandle, TextureHandle, (usize, usize))>,
}

impl QuantTool {
    pub fn new() -> QuantTool {
        QuantTool { path: String::new(), palettes: 16, dither: false, said: String::new(), shown: None }
    }

    /// One asked by the environment (`WORKSHOP_TILEQUANT=path.png`), converted at once, for a picture.
    pub fn asked(context: &egui::Context, launch: &workshop_base::launch::LaunchOptions) -> Option<QuantTool> {
        let path = launch.tilequant.clone()?;
        let mut tool = QuantTool { path, ..QuantTool::new() };
        tool.convert(context);
        Some(tool)
    }

    fn convert(&mut self, context: &egui::Context) {
        let source = std::path::PathBuf::from(self.path.trim());
        let asked = media::quant::Asked { palettes: self.palettes, dither: self.dither };
        match decor::tilesets::convert_file(&source, asked) {
            Ok((written, made)) => {
                let texture = |name: &str, rgba: &[u8]| {
                    context.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([made.size.0, made.size.1], rgba), egui::TextureOptions::NEAREST)
                };
                self.shown = Some((texture("before tilequant", &made.before), texture("after tilequant", &made.after), made.size));
                self.said = format!("written: {} ({} palette(s) of 16 colours)", written.display(), made.palettes);
            }
            Err(why) => {
                self.shown = None;
                self.said = why.to_string();
            }
        }
    }
}

/// The window, while it is open.
pub fn show(context: &egui::Context, tool: &mut Option<QuantTool>) {
    let Some(made) = tool.as_mut() else {
        return;
    };
    let mut open = true;
    if let Some(dropped) = context.input(|input| input.raw.dropped_files.iter().find_map(|file| file.path.clone())) {
        made.path = dropped.display().to_string();
        made.convert(context);
    }
    egui::Window::new("Convert an Image (Tilequant)").open(&mut open).default_width(760.0).show(context, |ui| {
        ui.label(theme::text(
            "to the console's rules: tiles of 8×8, each one palette of 16 colours (the first seen through), crisp pixels",
            theme::SMALL,
            theme::WEAK,
        ));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut made.path).desired_width(300.0).hint_text("path of the PNG (or drop it on the window)"));
            if ui.button("Browse…").clicked()
                && let Some(path) = workshop_base::pick::png("An Image to Convert", Some(std::path::Path::new(made.path.trim())))
            {
                made.path = path.display().to_string();
                made.convert(ui.ctx());
            }
            ui.add(egui::DragValue::new(&mut made.palettes).range(1..=16).suffix(" palettes")).on_hover_text("16 at most; 12 for a dungeon tileset");
            ui.checkbox(&mut made.dither, "dithered").on_hover_text("an ordered dither where colours are missing");
            if ui.button("Convert").clicked() {
                made.convert(ui.ctx());
            }
        });
        if !made.said.is_empty() {
            ui.label(RichText::new(&made.said).size(theme::SMALL).color(theme::TEXT));
        }
        if let Some((before, after, (wide, high))) = &made.shown {
            let room = (ui.available_width() - 12.0) / 2.0;
            let scale = (room / *wide as f32).min(420.0 / *high as f32);
            let size = egui::vec2(*wide as f32 * scale, *high as f32 * scale);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(theme::text("before", theme::SMALL, theme::WEAK));
                    ui.image((before.id(), size));
                });
                ui.vertical(|ui| {
                    ui.label(theme::text("after", theme::SMALL, theme::WEAK));
                    ui.image((after.id(), size));
                });
            });
        }
    });
    if !open {
        *tool = None;
    }
}
