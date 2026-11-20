//! The selection: lifted, dropped, copied, cut, pasted, cleared, changed.

use image::{Rgba, RgbaImage};

use super::{Painting, Tool};

impl Painting {
    /// What is chosen lifted to be moved (taken off its layer).
    pub(super) fn lift(&mut self) {
        if self.floating.is_some() {
            return;
        }
        let [x0, y0, x1, y1] = self.selection.unwrap_or([0, 0, self.wide() - 1, self.high() - 1]);
        let (wide, high) = ((x1 - x0 + 1) as u32, (y1 - y0 + 1) as u32);
        let lifted = image::imageops::crop_imm(self.cel(), x0 as u32, y0 as u32, wide, high).to_image();
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.cel().put_pixel(x as u32, y as u32, Rgba([0, 0, 0, 0]));
            }
        }
        self.floating = Some((lifted, (x0, y0)));
        self.floating_texture = None;
        let (layer, at) = (self.layer, self.at);
        self.changed(layer, at);
    }

    /// What is lifted laid down where it is.
    pub(super) fn drop_floating(&mut self) {
        if let Some((lifted, (x, y))) = self.floating.take() {
            image::imageops::overlay(self.cel(), &lifted, i64::from(x), i64::from(y));
            let (wide, high) = (lifted.width() as i32, lifted.height() as i32);
            self.selection = Some([x.max(0), y.max(0), (x + wide - 1).min(self.wide() - 1), (y + high - 1).min(self.high() - 1)])
                .filter(|[x0, y0, x1, y1]| x0 <= x1 && y0 <= y1);
            self.floating_texture = None;
            let (layer, at) = (self.layer, self.at);
            self.changed(layer, at);
        }
    }

    /// A picture laid over what is painted, to be moved (pasted, dropped on the window).
    pub fn paste(&mut self, picture: RgbaImage) {
        self.drop_floating();
        self.keep("paste");
        self.floating = Some((picture, (0, 0)));
        self.floating_texture = None;
        self.tool = Tool::Move;
    }

    /// What is chosen (or all the cel) copied; cut too if asked.
    pub(super) fn copy(&mut self, cut: bool) {
        if let Some((lifted, _)) = &self.floating {
            self.clipboard = Some(lifted.clone());
            if cut {
                self.keep("cut");
                self.floating = None;
                self.floating_texture = None;
            }
            return;
        }
        let [x0, y0, x1, y1] = self.selection.unwrap_or([0, 0, self.wide() - 1, self.high() - 1]);
        self.clipboard = Some(image::imageops::crop_imm(self.cel(), x0 as u32, y0 as u32, (x1 - x0 + 1) as u32, (y1 - y0 + 1) as u32).to_image());
        if cut {
            self.keep("cut");
            self.clear_chosen();
        }
        self.said = Some(if cut { "cut" } else { "copied" }.to_string());
    }

    pub(super) fn clear_chosen(&mut self) {
        let [x0, y0, x1, y1] = self.selection.unwrap_or([0, 0, self.wide() - 1, self.high() - 1]);
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.cel().put_pixel(x as u32, y as u32, Rgba([0, 0, 0, 0]));
            }
        }
        let (layer, at) = (self.layer, self.at);
        self.changed(layer, at);
    }

    /// Flipped, turned: what is lifted, else what is chosen, else the cel.
    pub(super) fn transformed(&mut self, said: &str, change: fn(&RgbaImage) -> RgbaImage) {
        self.keep(said);
        if let Some((lifted, at)) = self.floating.take() {
            self.floating = Some((change(&lifted), at));
            self.floating_texture = None;
            return;
        }
        if self.selection.is_some() {
            self.lift();
            if let Some((lifted, at)) = self.floating.take() {
                self.floating = Some((change(&lifted), at));
            }
            self.drop_floating();
            return;
        }
        let made = change(self.cel());
        let (wide, high) = (self.wide() as u32, self.high() as u32);
        let mut fitted = RgbaImage::new(wide, high);
        image::imageops::overlay(&mut fitted, &made, 0, 0);
        *self.cel() = fitted;
        let (layer, at) = (self.layer, self.at);
        self.changed(layer, at);
    }
}
