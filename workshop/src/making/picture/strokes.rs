//! Painting on the picture: where a brush lays, strokes made pixel-perfect,
//! shading, the colours used last.

use eframe::egui::Color32;
use image::Rgba;

use super::pixels::{line_points, rgba};
use super::{Painting, Tool};

impl Painting {
    /// Whether a pixel may be painted: on the picture, inside the selection if there is one.
    pub(super) fn paintable(&self, (x, y): (i32, i32)) -> bool {
        (0..self.wide()).contains(&x)
            && (0..self.high()).contains(&y)
            && self.selection.is_none_or(|[x0, y0, x1, y1]| (x0..=x1).contains(&x) && (y0..=y1).contains(&y))
    }

    /// The points a brush lays there, its mirrors with it.
    fn brushed(&self, (x, y): (i32, i32), size: u32) -> Vec<(i32, i32)> {
        let size = size.max(1) as i32;
        let start = -(size - 1) / 2;
        let mut points = Vec::new();
        for dy in start..start + size {
            for dx in start..start + size {
                let point = (x + dx, y + dy);
                points.push(point);
                if self.mirror_x {
                    points.push((self.wide() - 1 - point.0, point.1));
                }
                if self.mirror_y {
                    points.push((point.0, self.high() - 1 - point.1));
                }
                if self.mirror_x && self.mirror_y {
                    points.push((self.wide() - 1 - point.0, self.high() - 1 - point.1));
                }
            }
        }
        points
    }

    /// A colour laid at a point by the brush (a dither: one of two by the point).
    pub(super) fn lay(&mut self, point: (i32, i32), colour: Rgba<u8>, size: u32) {
        for (x, y) in self.brushed(point, size) {
            if self.paintable((x, y)) {
                let colour =
                    if self.tool == Tool::Dither && (x + y) % 2 != 0 { rgba(if self.secondary_down { self.primary } else { self.secondary }) } else { colour };
                self.cel().put_pixel(x as u32, y as u32, colour);
            }
        }
        let (layer, at) = (self.layer, self.at);
        self.changed(layer, at);
    }

    /// A pencil's pixel, kept pixel-perfect: an L's corner taken back.
    pub(super) fn stroke_to(&mut self, point: (i32, i32), colour: Rgba<u8>) {
        let from = self.last.unwrap_or(point);
        for step in line_points(from, point).into_iter().skip(usize::from(self.last.is_some())) {
            if self.perfect && self.brush == 1 && !self.mirror_x && !self.mirror_y && self.paintable(step) {
                let before = *self.cel().get_pixel(step.0 as u32, step.1 as u32);
                self.trail.push((step, before));
                let length = self.trail.len();
                if length >= 3 {
                    let ((ax, ay), _) = self.trail[length - 3];
                    let ((bx, by), was) = self.trail[length - 2];
                    let (cx, cy) = step;
                    let corner = (ax - cx).abs() == 1 && (ay - cy).abs() == 1 && ((bx == ax && by == cy) || (bx == cx && by == ay));
                    if corner {
                        self.cel().put_pixel(bx as u32, by as u32, was);
                        self.trail.remove(length - 2);
                    }
                }
            }
            self.lay(step, colour, self.brush);
        }
        self.last = Some(point);
    }

    /// A pixel made darker or lighter, once in a stroke.
    pub(super) fn shade(&mut self, point: (i32, i32), lighter: bool) {
        for (x, y) in self.brushed(point, self.brush) {
            if !self.paintable((x, y)) || !self.shaded.insert((x, y)) {
                continue;
            }
            let [r, g, b, a] = self.cel().get_pixel(x as u32, y as u32).0;
            if a == 0 {
                continue;
            }
            let moved = |channel: u8| if lighter { channel.saturating_add(((255 - channel) as f32 * 0.18) as u8 + 6) } else { (channel as f32 * 0.8) as u8 };
            self.cel().put_pixel(x as u32, y as u32, Rgba([moved(r), moved(g), moved(b), a]));
        }
        let (layer, at) = (self.layer, self.at);
        self.changed(layer, at);
    }

    /// A colour used: among the last ones.
    pub(super) fn used(&mut self, colour: Color32) {
        self.recent.retain(|known| *known != colour);
        self.recent.insert(0, colour);
        self.recent.truncate(16);
    }
}
