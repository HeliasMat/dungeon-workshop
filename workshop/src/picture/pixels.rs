//! Pictures as pixels: layers laid on each other, moved, turned, steps side
//! by side, the points of a line, a rectangle, an ellipse, a fill.

use std::collections::HashSet;

use eframe::egui::Color32;
use image::{Rgba, RgbaImage};

use super::Layer;

pub(super) fn rgba(colour: Color32) -> Rgba<u8> {
    Rgba(colour.to_srgba_unmultiplied())
}

/// A picture moved, scaled (negative: turned over), made fainter, about its middle (its pixels kept whole).
pub(super) fn moved(picture: &RgbaImage, dx: f32, dy: f32, sx: f32, sy: f32, alpha: f32) -> RgbaImage {
    let (wide, high) = (picture.width(), picture.height());
    let (cx, cy) = (wide as f32 / 2.0, high as f32 / 2.0);
    let mut made = RgbaImage::new(wide, high);
    for y in 0..high {
        for x in 0..wide {
            let from_x = (x as f32 + 0.5 - cx - dx) / if sx.abs() < 0.05 { 0.05_f32.copysign(sx) } else { sx } + cx;
            let from_y = (y as f32 + 0.5 - cy - dy) / sy.max(0.05) + cy;
            if from_x >= 0.0 && from_y >= 0.0 && (from_x as u32) < wide && (from_y as u32) < high {
                let mut pixel = *picture.get_pixel(from_x as u32, from_y as u32);
                pixel[3] = (f32::from(pixel[3]) * alpha.clamp(0.0, 1.0)) as u8;
                made.put_pixel(x, y, pixel);
            }
        }
    }
    made
}

/// A picture turned (degrees) and scaled about its middle, its pixels kept whole (as big as it was).
pub(super) fn rotated(picture: &RgbaImage, degrees: f32, scale: f32) -> RgbaImage {
    let (wide, high) = (picture.width(), picture.height());
    let (cx, cy) = (wide as f32 / 2.0, high as f32 / 2.0);
    let (sin, cos) = (-degrees.to_radians()).sin_cos();
    let scale = scale.max(0.05);
    let mut made = RgbaImage::new(wide, high);
    for y in 0..high {
        for x in 0..wide {
            let (px, py) = ((x as f32 + 0.5 - cx) / scale, (y as f32 + 0.5 - cy) / scale);
            let (from_x, from_y) = (px * cos - py * sin + cx, px * sin + py * cos + cy);
            if from_x >= 0.0 && from_y >= 0.0 && (from_x as u32) < wide && (from_y as u32) < high {
                made.put_pixel(x, y, *picture.get_pixel(from_x as u32, from_y as u32));
            }
        }
    }
    made
}

/// The layers seen, laid one over the other (each as opaque as it is, as it is laid), at a step.
pub(super) fn composite(layers: &[Layer], at: usize) -> RgbaImage {
    let (wide, high) = (layers[0].cels[at].width(), layers[0].cels[at].height());
    let mut made = RgbaImage::new(wide, high);
    for layer in layers.iter().filter(|layer| layer.visible && layer.opacity > 0) {
        for (x, y, pixel) in layer.cels[at].enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            let alpha = f32::from(a) / 255.0 * f32::from(layer.opacity) / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let under = made.get_pixel(x, y).0;
            let under_alpha = f32::from(under[3]) / 255.0;
            // (Its colour as its way of laying makes it, over what is under it (where something is).)
            let blended = |over: u8, below: u8| -> f32 {
                let (over, below) = (f32::from(over) / 255.0, f32::from(below) / 255.0);
                let made = match layer.blend {
                    1 => over * below,
                    2 => (over + below).min(1.0),
                    3 => 1.0 - (1.0 - over) * (1.0 - below),
                    _ => over,
                };
                (over + (made - over) * under_alpha) * 255.0
            };
            let out_alpha = alpha + under_alpha * (1.0 - alpha);
            let mix = |over: u8, below: u8| {
                ((blended(over, below) * alpha + f32::from(below) * under_alpha * (1.0 - alpha)) / out_alpha.max(1e-6)).round().clamp(0.0, 255.0) as u8
            };
            made.put_pixel(x, y, Rgba([mix(r, under[0]), mix(g, under[1]), mix(b, under[2]), (out_alpha * 255.0).round() as u8]));
        }
    }
    made
}

/// Steps side by side, as the game's sheets are.
pub(super) fn strip(steps: &[RgbaImage]) -> RgbaImage {
    let (wide, high) = (steps[0].width(), steps[0].height());
    let mut made = RgbaImage::new(wide * steps.len() as u32, high);
    for (at, step) in steps.iter().enumerate() {
        image::imageops::overlay(&mut made, step, i64::from(wide) * at as i64, 0);
    }
    made
}

/// The pixels from one to another, none missed.
pub(super) fn line_points((mut x, mut y): (i32, i32), (to_x, to_y): (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = ((to_x - x).abs(), -(to_y - y).abs());
    let (sx, sy) = (if x < to_x { 1 } else { -1 }, if y < to_y { 1 } else { -1 });
    let mut error = dx + dy;
    let mut points = Vec::new();
    loop {
        points.push((x, y));
        if x == to_x && y == to_y {
            return points;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
}

/// The pixels of a rectangle: its edge, or all of it.
pub(super) fn rectangle_points((x0, y0): (i32, i32), (x1, y1): (i32, i32), filled: bool) -> Vec<(i32, i32)> {
    let (left, right, top, bottom) = (x0.min(x1), x0.max(x1), y0.min(y1), y0.max(y1));
    let mut points = Vec::new();
    for y in top..=bottom {
        for x in left..=right {
            if filled || x == left || x == right || y == top || y == bottom {
                points.push((x, y));
            }
        }
    }
    points
}

/// The pixels of an ellipse within a rectangle: its edge, or all of it.
pub(super) fn ellipse_points((x0, y0): (i32, i32), (x1, y1): (i32, i32), filled: bool) -> Vec<(i32, i32)> {
    let (left, right, top, bottom) = (x0.min(x1), x0.max(x1), y0.min(y1), y0.max(y1));
    let (cx, cy) = ((left + right) as f32 / 2.0, (top + bottom) as f32 / 2.0);
    let (rx, ry) = (((right - left) as f32 / 2.0).max(0.5), ((bottom - top) as f32 / 2.0).max(0.5));
    let mut points = HashSet::new();
    for y in top..=bottom {
        let along = ((y as f32 - cy) / ry).clamp(-1.0, 1.0);
        let half = rx * (1.0 - along * along).max(0.0).sqrt();
        let (from, to) = ((cx - half).round() as i32, (cx + half).round() as i32);
        if filled {
            points.extend((from..=to).map(|x| (x, y)));
        } else {
            points.insert((from, y));
            points.insert((to, y));
        }
    }
    if !filled {
        for x in left..=right {
            let along = ((x as f32 - cx) / rx).clamp(-1.0, 1.0);
            let half = ry * (1.0 - along * along).max(0.0).sqrt();
            points.insert((x, (cy - half).round() as i32));
            points.insert((x, (cy + half).round() as i32));
        }
    }
    points.into_iter().collect()
}

/// The pixels of the same colour as that one, touching it (or anywhere), made another.
pub(super) fn filled(frame: &mut RgbaImage, (x, y): (i32, i32), colour: Rgba<u8>, everywhere: bool, inside: &dyn Fn((i32, i32)) -> bool) {
    let (wide, high) = (frame.width() as i32, frame.height() as i32);
    if !(0..wide).contains(&x) || !(0..high).contains(&y) {
        return;
    }
    let was = *frame.get_pixel(x as u32, y as u32);
    if was == colour {
        return;
    }
    if everywhere {
        for y in 0..high {
            for x in 0..wide {
                if *frame.get_pixel(x as u32, y as u32) == was && inside((x, y)) {
                    frame.put_pixel(x as u32, y as u32, colour);
                }
            }
        }
        return;
    }
    let mut waiting = vec![(x, y)];
    while let Some((x, y)) = waiting.pop() {
        if !(0..wide).contains(&x) || !(0..high).contains(&y) || *frame.get_pixel(x as u32, y as u32) != was || !inside((x, y)) {
            continue;
        }
        frame.put_pixel(x as u32, y as u32, colour);
        waiting.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
    }
}
