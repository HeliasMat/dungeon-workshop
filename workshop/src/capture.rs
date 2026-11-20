//! A picture of the window, to look at it without sitting before it:
//! `WORKSHOP_CAPTURE=path.png` (and `WORKSHOP_SPACE=film|decor|resources`,
//! `WORKSHOP_AFTER=seconds`) takes it once it is time, writes it, closes.

use eframe::egui;

/// A picture asked: where, from when, after how long, whether asked yet.
pub struct Capture {
    path: std::path::PathBuf,
    since: std::time::Instant,
    after: std::time::Duration,
    asked: bool,
}

impl Capture {
    /// The one the Workshop was launched with, if one is (`WORKSHOP_CAPTURE`, `WORKSHOP_AFTER`).
    pub fn asked(launch: &crate::launch::LaunchOptions) -> Option<Capture> {
        let path = launch.capture.clone()?;
        let seconds = launch.capture_after.unwrap_or(3);
        Some(Capture { path, since: std::time::Instant::now(), after: std::time::Duration::from_secs(seconds), asked: false })
    }

    /// Asked once it is time; written once it comes, and the window closed.
    pub fn take(&mut self, context: &egui::Context) {
        let taken = context.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = taken {
            let pixels: Vec<u8> = image.pixels.iter().flat_map(|colour| colour.to_array()).collect();
            let [wide, high] = image.size.map(|side| side as u32);
            if let Some(picture) = image::RgbaImage::from_raw(wide, high, pixels)
                && let Err(why) = picture.save(&self.path)
            {
                eprintln!("{}: {why}", self.path.display());
            }
            context.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !self.asked && self.since.elapsed() >= self.after {
            context.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            self.asked = true;
        }
        context.request_repaint();
    }
}
