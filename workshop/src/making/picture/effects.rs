//! What changes a whole picture: a beginner's shapes, relief and
//! animations, an expert's outline, colours adjusted, turned, fewer colours,
//! resized.

use std::collections::HashMap;
use std::time::Instant;

use eframe::egui::{self, Color32};
use image::{Rgba, RgbaImage};

use super::files::picture_expert_file;
use super::pixels::{moved, rgba, rotated};
use super::{Layer, Painting, Tool};

impl Painting {
    /// A line of the primary colour around what is painted (on the cel).
    pub(super) fn outline(&mut self) {
        self.keep("outline");
        let colour = rgba(self.primary);
        let before = self.cel().clone();
        let (wide, high) = (self.wide(), self.high());
        for y in 0..high {
            for x in 0..wide {
                if before.get_pixel(x as u32, y as u32)[3] != 0 || !self.paintable((x, y)) {
                    continue;
                }
                let touches = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    (0..wide).contains(&nx) && (0..high).contains(&ny) && before.get_pixel(nx as u32, ny as u32)[3] != 0
                });
                if touches {
                    self.cel().put_pixel(x as u32, y as u32, colour);
                }
            }
        }
        let (layer, at) = (self.layer, self.at);
        self.changed(layer, at);
    }

    /// Painted as an expert or simply (kept).
    pub(super) fn set_expert(&mut self, expert: bool) {
        self.expert = expert;
        if !expert && !matches!(self.tool, Tool::Pencil | Tool::Eraser | Tool::Fill | Tool::Picker) {
            self.drop_floating();
            self.tool = Tool::Pencil;
        }
        if let Some(folder) = picture_expert_file(self.session.root()).parent() {
            _ = std::fs::create_dir_all(folder);
        }
        _ = std::fs::write(picture_expert_file(self.session.root()), if expert { "1" } else { "0" });
    }

    /// A beginner's start: an empty picture of that size (one step, one layer).
    pub(super) fn begin(&mut self, side: u32) {
        self.keep("new image");
        self.layers = vec![Layer { name: "Layer 1".to_string(), visible: true, opacity: 255, blend: 0, cels: vec![RgbaImage::new(side, side)] }];
        self.ticks = vec![8];
        (self.layer, self.at, self.selection, self.floating) = (0, 0, None, None);
        self.zoom = 0.0;
        self.settle();
    }

    /// A shape laid in the primary colour, its outline round it: to begin from.
    pub(super) fn shape(&mut self, kind: &str) {
        self.keep(kind);
        let (wide, high) = (self.wide(), self.high());
        let colour = rgba(self.primary);
        let (cx, cy) = (wide as f32 / 2.0 - 0.5, high as f32 / 2.0 - 0.5);
        let (rx, ry) = (wide as f32 * 0.38, high as f32 * 0.38);
        let inside = |x: f32, y: f32| -> bool {
            let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
            match kind {
                "Round" => dx.abs().powf(2.4) + dy.abs().powf(2.4) <= 1.0,
                "Box" => dx.abs() <= 0.95 && dy.abs() <= 0.8,
                "Drop" => {
                    let below = dx * dx + ((y - cy - ry * 0.25) / (ry * 0.75)).powi(2) <= 1.0;
                    let top = dy < -0.05 && dx.abs() <= (dy + 1.0).max(0.0) * 0.7;
                    below || top
                }
                _ => {
                    // (A star of five points.)
                    let angle = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
                    let reach = 0.45 + 0.55 * ((angle * 5.0 / 2.0).cos().abs()).powf(3.0);
                    (dx * dx + dy * dy).sqrt() <= reach
                }
            }
        };
        for y in 0..high {
            for x in 0..wide {
                // (A lone tip, of one pixel: not drawn, as a pixel artist would not.)
                let around = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)].iter().filter(|(dx, dy)| inside(x as f32 + dx, y as f32 + dy)).count();
                if inside(x as f32, y as f32) && around >= 2 {
                    self.cel().put_pixel(x as u32, y as u32, colour);
                }
            }
        }
        let primary = self.primary;
        self.primary = Color32::from_rgb(0x18, 0x10, 0x10);
        self.outline();
        self.undone.pop();
        self.primary = primary;
        let (layer, at) = (self.layer, self.at);
        self.changed(layer, at);
    }

    /// Relief as the game's pictures have it: light from the top left, shade at the bottom right.
    pub(super) fn relief(&mut self) {
        self.keep("relief");
        let before = self.cel().clone();
        let (wide, high) = (self.wide(), self.high());
        // (What is drawn, its outline (the darkest) left out: the outline is its edge, as the empty is.)
        let seen = |x: i32, y: i32| {
            (0..wide).contains(&x) && (0..high).contains(&y) && {
                let [r, g, b, a] = before.get_pixel(x as u32, y as u32).0;
                a > 0 && u16::from(r) + u16::from(g) + u16::from(b) >= 90
            }
        };
        for y in 0..high {
            for x in 0..wide {
                if !seen(x, y) {
                    continue;
                }
                let [r, g, b, a] = before.get_pixel(x as u32, y as u32).0;
                let lit = !seen(x, y - 1) || !seen(x - 1, y);
                let shaded = !seen(x, y + 1) || !seen(x + 1, y);
                let made = if lit {
                    [r, g, b].map(|channel| channel.saturating_add(((255 - channel) as f32 * 0.3) as u8))
                } else if shaded {
                    [r, g, b].map(|channel| (channel as f32 * 0.7) as u8)
                } else {
                    continue;
                };
                self.cel().put_pixel(x as u32, y as u32, Rgba([made[0], made[1], made[2], a]));
            }
        }
        let (layer, at) = (self.layer, self.at);
        self.changed(layer, at);
    }

    /// An animation made from the first step: it bounces, blinks, shakes, grows, turns (every layer so).
    pub(super) fn animate(&mut self, kind: &str) {
        self.drop_floating();
        self.keep(kind);
        let high = self.high() as f32;
        let lift = (high / 10.0).round().max(1.0);
        let steps: Vec<(f32, f32, f32, f32, f32)> = match kind {
            "Bounce" => [0.0, -0.5, -1.0, -1.3, -1.0, -0.5].iter().map(|up| (0.0, up * lift, 1.0, 1.0, 1.0)).collect(),
            "Blink" => [1.0, 0.65, 0.25, 0.65].iter().map(|alpha| (0.0, 0.0, 1.0, 1.0, *alpha)).collect(),
            "Shake" => [0.0, 1.0, 0.0, -1.0].iter().map(|dx| (*dx, 0.0, 1.0, 1.0, 1.0)).collect(),
            "Grow" => [1.0, 1.06, 1.12, 1.06, 1.0, 0.94].iter().map(|grow| (0.0, 0.0, *grow, *grow, 1.0)).collect(),
            _ => [1.0, 0.55, 0.1, -0.55, -1.0, -0.55, 0.1, 0.55].iter().map(|width| (0.0, 0.0, *width, 1.0, 1.0)).collect(),
        };
        for layer in &mut self.layers {
            let first = layer.cels[0].clone();
            layer.cels = steps.iter().map(|(dx, dy, sx, sy, alpha)| moved(&first, *dx, *dy, *sx, *sy, *alpha)).collect();
        }
        self.ticks = vec![self.pace_ticks(); steps.len()];
        self.at = 0;
        self.playing = true;
        self.began = Instant::now();
        self.settle();
    }

    /// The time of each step, by a beginner's pace (from slow to fast).
    pub(super) fn pace_ticks(&self) -> u8 {
        (20.0 - self.pace.clamp(0.0, 1.0) * 17.0).round() as u8
    }

    /// Its colours adjusted (hue turned, saturation, light, contrast), on this step or every one of the layer.
    pub(super) fn adjusted(&mut self) {
        self.keep("colours");
        let [hue, saturation, light, contrast] = self.adjust;
        let change = |pixel: &mut Rgba<u8>| {
            if pixel[3] == 0 {
                return;
            }
            let hsva = egui::ecolor::Hsva::from(Color32::from_rgb(pixel[0], pixel[1], pixel[2]));
            let mut made = egui::ecolor::Hsva { h: (hsva.h + hue / 360.0).rem_euclid(1.0), s: (hsva.s * (1.0 + saturation)).clamp(0.0, 1.0), ..hsva };
            made.v = (made.v * (1.0 + light)).clamp(0.0, 1.0);
            let colour = Color32::from(made);
            let stretch = |channel: u8| (((f32::from(channel) / 255.0 - 0.5) * (1.0 + contrast) + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
            *pixel = Rgba([stretch(colour.r()), stretch(colour.g()), stretch(colour.b()), pixel[3]]);
        };
        let (layer, at) = (self.layer, self.at);
        if self.adjust_all {
            for cel in &mut self.layers[layer].cels {
                cel.pixels_mut().for_each(change);
            }
        } else {
            self.layers[layer].cels[at].pixels_mut().for_each(change);
        }
        self.adjust = [0.0; 4];
        self.all_changed();
    }

    /// Turned and scaled about its middle (its pixels kept whole): what is chosen, else the cel.
    pub(super) fn turned(&mut self) {
        self.keep("rotate, scale");
        let (turn, scale) = (self.turn, self.scale);
        let made = |picture: &RgbaImage| rotated(picture, turn, scale);
        if let Some((lifted, at)) = self.floating.take() {
            self.floating = Some((made(&lifted), at));
            self.floating_texture = None;
        } else if self.selection.is_some() {
            self.lift();
            if let Some((lifted, at)) = self.floating.take() {
                self.floating = Some((made(&lifted), at));
            }
            self.drop_floating();
        } else {
            let done = made(self.cel());
            *self.cel() = done;
        }
        (self.turn, self.scale) = (0.0, 1.0);
        self.all_changed();
    }

    /// The layer's colours made so many at most (the nearest of the most used).
    pub(super) fn fewer_colours(&mut self) {
        self.keep("fewer colours");
        let layer = self.layer;
        let mut counted: HashMap<[u8; 3], usize> = HashMap::new();
        for cel in &self.layers[layer].cels {
            for pixel in cel.pixels().filter(|pixel| pixel[3] > 0) {
                *counted.entry([pixel[0], pixel[1], pixel[2]]).or_default() += 1;
            }
        }
        let wanted = self.colours_at_most.max(2);
        if counted.len() <= wanted {
            self.undone.pop();
            self.said = Some(format!("already {} colours or fewer", counted.len()));
            return;
        }
        // (A few rounds of k-means, from the most used ones.)
        let mut colours: Vec<([u8; 3], usize)> = counted.into_iter().collect();
        colours.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        let mut centres: Vec<[f32; 3]> = colours.iter().take(wanted).map(|(colour, _)| colour.map(f32::from)).collect();
        let nearest = |centres: &[[f32; 3]], colour: [f32; 3]| -> usize {
            centres
                .iter()
                .enumerate()
                .map(|(at, centre)| (at, (0..3).map(|channel| (centre[channel] - colour[channel]).powi(2)).sum::<f32>()))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map_or(0, |(at, _)| at)
        };
        for _ in 0..8 {
            let mut sums = vec![([0.0_f32; 3], 0.0_f32); centres.len()];
            for (colour, count) in &colours {
                let colour = colour.map(f32::from);
                let at = nearest(&centres, colour);
                for (sum, channel) in sums[at].0.iter_mut().zip(colour) {
                    *sum += channel * *count as f32;
                }
                sums[at].1 += *count as f32;
            }
            for (centre, (sum, count)) in centres.iter_mut().zip(sums) {
                if count > 0.0 {
                    *centre = sum.map(|value| value / count);
                }
            }
        }
        for cel in &mut self.layers[layer].cels {
            for pixel in cel.pixels_mut().filter(|pixel| pixel[3] > 0) {
                let centre = centres[nearest(&centres, [pixel[0], pixel[1], pixel[2]].map(f32::from))];
                *pixel = Rgba([centre[0].round() as u8, centre[1].round() as u8, centre[2].round() as u8, pixel[3]]);
            }
        }
        self.all_changed();
    }

    /// Every pixel of one colour made another, on every step of the layer.
    pub(super) fn replace_colour(&mut self, from: Color32, to: Color32) {
        self.keep("replace the colour");
        let (from, to) = (rgba(from), rgba(to));
        let layer = self.layer;
        for cel in &mut self.layers[layer].cels {
            for pixel in cel.pixels_mut().filter(|pixel| **pixel == from) {
                *pixel = to;
            }
        }
        self.all_changed();
    }

    /// Made that size, what is painted kept from its top left.
    pub(super) fn resized(&mut self, (wide, high): (u32, u32)) {
        self.keep("size");
        for layer in &mut self.layers {
            for cel in &mut layer.cels {
                let mut made = RgbaImage::new(wide, high);
                image::imageops::overlay(&mut made, cel, 0, 0);
                *cel = made;
            }
        }
        self.selection = None;
        self.floating = None;
        self.all_changed();
    }

    /// The colours the picture has, the most used first.
    pub(super) fn own_colours(&self) -> Vec<Color32> {
        let mut counted: HashMap<[u8; 4], usize> = HashMap::new();
        for layer in &self.layers {
            for cel in &layer.cels {
                for pixel in cel.pixels().filter(|pixel| pixel[3] > 0) {
                    *counted.entry(pixel.0).or_default() += 1;
                }
            }
        }
        let mut colours: Vec<([u8; 4], usize)> = counted.into_iter().collect();
        colours.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        colours.into_iter().take(32).map(|([r, g, b, a], _)| Color32::from_rgba_unmultiplied(r, g, b, a)).collect()
    }
}
