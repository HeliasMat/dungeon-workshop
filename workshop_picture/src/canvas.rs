//! The canvas: the picture drawn zoomed, painted with the pointer, the
//! selection's dashes, the checkerboard behind. What the pointer does with
//! each tool: `pointer.rs`.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};

use super::pointer::{pixel_at, pointer};
use super::{Painting, Tool};
use workshop_base::theme;

/// A checkered ground under a picture (what is not painted shows so).
pub(super) fn checkered(painter: &egui::Painter, room: Rect, square: f32) {
    painter.rect_filled(room, 0.0, Color32::from_rgb(0x2E, 0x2E, 0x2E));
    let square = square.max(4.0);
    let (columns, rows) = ((room.width() / square).ceil() as i32, (room.height() / square).ceil() as i32);
    if columns * rows > 40_000 {
        return;
    }
    for row in 0..rows {
        for column in (0..columns).filter(|column| (row + column) % 2 == 0) {
            let cell = Rect::from_min_size(room.min + vec2(column as f32 * square, row as f32 * square), vec2(square, square)).intersect(room);
            painter.rect_filled(cell, 0.0, Color32::from_rgb(0x26, 0x26, 0x26));
        }
    }
}

/// A picture in a room, its pixels whole when it fits so.
pub fn fitted(size: Vec2, room: Rect) -> Rect {
    let fit = (room.width() / size.x).min(room.height() / size.y);
    let scale = if fit >= 1.0 { fit.floor() } else { fit };
    Rect::from_center_size(room.center(), size * scale)
}

/// The zoom after a turn of the wheel, around the pointer: how close, the picture's shift.
pub(super) fn wheel_zoom(zoom: f32, pan: Vec2, wheel: f32, at: Pos2, middle: Pos2) -> (f32, Vec2) {
    let closer = (zoom * if wheel > 0.0 { 1.25 } else { 0.8 }).clamp(1.0, 64.0);
    let centre = middle + pan;
    let moved = at - (at - centre) * (closer / zoom);
    (closer, moved - middle)
}

/// The canvas: the layers seen, painted on by the pointer.
pub(super) fn canvas(ui: &mut egui::Ui, painting: &mut Painting, room: Rect, context: &egui::Context) {
    let painter = ui.painter_at(room);
    painter.rect_filled(room, 0.0, Color32::from_rgb(0x1A, 0x1A, 0x1A));
    let response = ui.interact(room, ui.id().with("canvas"), Sense::click_and_drag());
    let panning = viewed(ui, painting, &response, room);
    let size = vec2(painting.wide() as f32, painting.high() as f32);
    let shown = Rect::from_center_size(room.center() + painting.pan, size * painting.zoom);
    checkered(&painter, shown, (painting.zoom * 4.0).max(8.0));
    let full = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    if painting.playing && painting.count() > 1 {
        let at = painting.playing_step();
        let texture = painting.composite_texture(context, at);
        painter.image(texture.id(), shown, full, Color32::WHITE);
        context.request_repaint_after(std::time::Duration::from_millis(16));
    } else {
        // (Tiled: the picture round it, faint, to see a pattern join.)
        if painting.tiled {
            let texture = painting.composite_texture(context, painting.at);
            for (dx, dy) in [(-1.0, -1.0), (0.0, -1.0), (1.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0)] {
                painter.image(texture.id(), shown.translate(vec2(dx * shown.width(), dy * shown.height())), full, Color32::from_white_alpha(120));
            }
        }
        if painting.onion && painting.count() > 1 {
            onion(&painter, painting, context, shown);
        }
        layers(&painter, painting, context, shown);
        floating(ui, &painter, painting, context, shown);
    }
    if painting.grid && painting.zoom >= 6.0 {
        let stroke = Stroke::new(1.0_f32, Color32::from_white_alpha(16));
        for x in 0..=size.x as i32 {
            painter.vline(shown.min.x + x as f32 * painting.zoom, shown.y_range(), stroke);
        }
        for y in 0..=size.y as i32 {
            painter.hline(shown.x_range(), shown.min.y + y as f32 * painting.zoom, stroke);
        }
    }
    painter.rect_stroke(shown, 0.0, Stroke::new(1.0_f32, theme::GAP), StrokeKind::Outside);
    if let Some([x0, y0, x1, y1]) = painting.selection.filter(|_| painting.floating.is_none()) {
        let place =
            Rect::from_min_max(shown.min + vec2(x0 as f32, y0 as f32) * painting.zoom, shown.min + vec2((x1 + 1) as f32, (y1 + 1) as f32) * painting.zoom);
        dashed(&painter, place, ui.input(|input| input.time));
        context.request_repaint_after(std::time::Duration::from_millis(60));
    }
    if let Some(at) = response.hover_pos() {
        brush_shown(ui, &painter, painting, shown, room, at, panning);
    }
    if panning || painting.playing {
        return;
    }
    pointer(ui, painting, &response, &painter, shown);
}

/// The view: fitted, closer by the wheel (around the pointer), moved by the
/// middle button or Space held; whether it is being moved.
fn viewed(ui: &egui::Ui, painting: &mut Painting, response: &egui::Response, room: Rect) -> bool {
    let size = vec2(painting.wide() as f32, painting.high() as f32);
    if painting.zoom <= 0.0 {
        painting.zoom_fits = true;
    }
    if painting.zoom_fits {
        painting.zoom = ((room.width() - 40.0) / size.x).min((room.height() - 40.0) / size.y).floor().clamp(1.0, 64.0);
        painting.pan = Vec2::ZERO;
    }
    if let Some(at) = response.hover_pos() {
        let wheel = ui.input(|input| input.raw_scroll_delta.y);
        if wheel != 0.0 {
            (painting.zoom, painting.pan) = wheel_zoom(painting.zoom, painting.pan, wheel, at, room.center());
            painting.zoom_fits = false;
        }
    }
    let panning = ui.input(|input| input.pointer.middle_down() || input.key_down(egui::Key::Space));
    if panning && response.dragged() {
        painting.pan += response.drag_delta();
        painting.zoom_fits = false;
    }
    panning
}

/// The steps before and after, faint and tinted: the onion's skin.
fn onion(painter: &egui::Painter, painting: &mut Painting, context: &egui::Context, shown: Rect) {
    let full = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    let count = painting.count();
    let range = painting.onion_range.clamp(1, 4).min(count - 1);
    for distance in (1..=range).rev() {
        let faint = (painting.onion_strength.clamp(0.0, 1.0) * 255.0 / distance as f32) as u8;
        for (other, tint) in [
            ((painting.at + count * 4 - distance) % count, Color32::from_rgba_unmultiplied(255, 90, 90, faint)),
            ((painting.at + distance) % count, Color32::from_rgba_unmultiplied(90, 255, 120, faint)),
        ] {
            if other != painting.at {
                let texture = painting.composite_texture(context, other);
                painter.image(texture.id(), shown, full, tint);
            }
        }
    }
}

/// The layers seen, each at its opacity (laid otherwise than as they are: as they make the picture).
fn layers(painter: &egui::Painter, painting: &mut Painting, context: &egui::Context, shown: Rect) {
    let full = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    if painting.layers.iter().any(|layer| layer.blend != 0) {
        let texture = painting.composite_texture(context, painting.at);
        painter.image(texture.id(), shown, full, Color32::WHITE);
        return;
    }
    for layer in 0..painting.layers.len() {
        if !painting.layers[layer].visible {
            continue;
        }
        let opacity = painting.layers[layer].opacity;
        let texture = painting.texture(context, layer, painting.at);
        painter.image(texture.id(), shown, full, Color32::from_white_alpha(opacity));
    }
}

/// What is lifted, over it all, in marching dashes.
fn floating(ui: &egui::Ui, painter: &egui::Painter, painting: &mut Painting, context: &egui::Context, shown: Rect) {
    let Some((lifted, (x, y))) = &painting.floating else {
        return;
    };
    if painting.floating_texture.is_none() {
        let image = egui::ColorImage::from_rgba_unmultiplied([lifted.width() as usize, lifted.height() as usize], lifted.as_raw());
        painting.floating_texture = Some(context.load_texture("floating", image, egui::TextureOptions::NEAREST));
    }
    if let Some(texture) = &painting.floating_texture {
        let place =
            Rect::from_min_size(shown.min + vec2(*x as f32, *y as f32) * painting.zoom, vec2(lifted.width() as f32, lifted.height() as f32) * painting.zoom);
        painter.image(texture.id(), place, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
        dashed(painter, place, ui.input(|input| input.time));
    }
}

/// Where the pointer is: the brush shown, its place (and the selection's size) said.
fn brush_shown(ui: &egui::Ui, painter: &egui::Painter, painting: &Painting, shown: Rect, room: Rect, at: Pos2, panning: bool) {
    let (x, y) = pixel_at(shown.min, painting.zoom, at);
    let brush = if matches!(painting.tool, Tool::Pencil | Tool::Eraser | Tool::Shade | Tool::Dither) { painting.brush as i32 } else { 1 };
    let start = -(brush - 1) / 2;
    let cell = Rect::from_min_size(shown.min + vec2((x + start) as f32, (y + start) as f32) * painting.zoom, Vec2::splat(brush as f32 * painting.zoom));
    painter.rect_stroke(cell, 0.0, Stroke::new(1.0_f32, Color32::WHITE), StrokeKind::Inside);
    let mut said = format!("{x}, {y}");
    if let Some([x0, y0, x1, y1]) = painting.selection {
        said.push_str(&format!("   selection {}×{}", x1 - x0 + 1, y1 - y0 + 1));
    }
    painter.text(room.right_bottom() - vec2(6.0, 6.0), egui::Align2::RIGHT_BOTTOM, said, FontId::monospace(theme::SMALL), theme::WEAK);
    if panning {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
}

/// A rectangle's edge in marching dashes.
fn dashed(painter: &egui::Painter, rect: Rect, time: f64) {
    let shift = (time * 12.0) as f32 % 8.0;
    let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];
    for pair in corners.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let length = from.distance(to);
        let direction = (to - from) / length.max(1.0);
        let mut along = -shift;
        while along < length {
            let (a, b) = (along.max(0.0), (along + 4.0).min(length));
            if b > a {
                painter.line_segment([from + direction * a, from + direction * b], Stroke::new(1.0_f32, Color32::WHITE));
            }
            let (c, d) = ((along + 4.0).max(0.0), (along + 8.0).min(length));
            if d > c {
                painter.line_segment([from + direction * c, from + direction * d], Stroke::new(1.0_f32, Color32::BLACK));
            }
            along += 8.0;
        }
    }
}
