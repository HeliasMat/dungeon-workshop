//! What the Scene draws over the decor: what is chosen (framed, the one
//! looked at with its handles), what is pointed at, the rectangle being
//! drawn, the box of several; with Alt, how far another is; very close, the
//! pixels' grid; outside the decor darker, its edge and size; the rulers.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};

use super::{Read, View};
use crate::decors::{Decors, tag};
use crate::theme;

/// The gaps between two boxes, across and down, where they do not overlap:
/// each from where to where (at the height, or along the line, of their middles) and how wide.
pub(super) fn gaps_between(from: Rect, to: Rect) -> Vec<(Pos2, Pos2, f32)> {
    let mut gaps = Vec::new();
    let y = (from.center().y + to.center().y) / 2.0;
    let x = (from.center().x + to.center().x) / 2.0;
    if to.min.x >= from.max.x {
        gaps.push((pos2(from.max.x, y), pos2(to.min.x, y), to.min.x - from.max.x));
    } else if from.min.x >= to.max.x {
        gaps.push((pos2(to.max.x, y), pos2(from.min.x, y), from.min.x - to.max.x));
    }
    if to.min.y >= from.max.y {
        gaps.push((pos2(x, from.max.y), pos2(x, to.min.y), to.min.y - from.max.y));
    } else if from.min.y >= to.max.y {
        gaps.push((pos2(x, to.max.y), pos2(x, from.min.y), from.min.y - to.max.y));
    }
    gaps
}

/// The rulers' step: one that reads at this closeness (its marks 56 points apart at least).
pub(super) fn ruler_step(scale: f32) -> f32 {
    [8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0].into_iter().find(|step: &f32| step * scale >= 56.0).unwrap_or(2048.0)
}

/// What is chosen framed (the one looked at with its handles and its name);
/// the one pointed at, or pointed in the Hierarchy; the rectangle being
/// drawn; several chosen, their box together and its size. That box given back.
pub(super) fn chosen_framed(painter: &egui::Painter, decors: &Decors, read: &Read, view: View, under: Option<usize>, pointed: Option<Pos2>) -> Option<Rect> {
    let selection = &read.selection;
    for (at, one) in read.items.iter().enumerate() {
        let frame = view.on_screen(read.boxes[at]);
        let key = (one.line, one.at);
        if selection.contains(&key) {
            painter.rect_stroke(frame, 0.0, Stroke::new(1.5_f32, theme::DECOR), StrokeKind::Outside);
            if selection.first() == Some(&key) {
                for corner in [frame.left_top(), frame.right_top(), frame.left_bottom(), frame.right_bottom()] {
                    painter.rect_filled(Rect::from_center_size(corner, vec2(6.0, 6.0)), 0.0, Color32::WHITE);
                    painter.rect_stroke(Rect::from_center_size(corner, vec2(6.0, 6.0)), 0.0, Stroke::new(1.0_f32, theme::DECOR), StrokeKind::Inside);
                }
                let many = if selection.len() > 1 { format!(" (+{})", selection.len() - 1) } else { String::new() };
                tag(
                    painter,
                    frame.left_top() - vec2(0.0, 4.0),
                    &format!("{} · {} {}{many}", decors.item_name(one), one.corner[0], one.corner[1]),
                    theme::DECOR,
                );
            }
        } else if (under == Some(at) && read.carry.is_none()) || decors.hier_hovered.contains(&key) {
            painter.rect_stroke(frame, 0.0, Stroke::new(1.0_f32, theme::DECOR.gamma_multiply(0.8)), StrokeKind::Outside);
        }
    }
    if let (Some(start), Some(now)) = (read.marquee, pointed) {
        let drawn = Rect::from_two_pos(view.to_screen(start), now);
        painter.rect_filled(drawn, 0.0, theme::DECOR.gamma_multiply(0.12));
        painter.rect_stroke(drawn, 0.0, Stroke::new(1.0_f32, theme::DECOR), StrokeKind::Inside);
    }
    let chosen_boxes: Vec<Rect> =
        read.items.iter().enumerate().filter(|(_, one)| selection.contains(&(one.line, one.at))).map(|(at, _)| read.boxes[at]).collect();
    let together = chosen_boxes.iter().copied().reduce(|a, b| a.union(b));
    if let Some(together) = together.filter(|_| chosen_boxes.len() > 1) {
        let frame = view.on_screen(together);
        painter.rect_stroke(frame.expand(3.0), 0.0, Stroke::new(1.0_f32, Color32::WHITE.gamma_multiply(0.7)), StrokeKind::Outside);
        tag(painter, frame.left_bottom() + vec2(0.0, 22.0), &format!("{} × {}", together.width().round(), together.height().round()), Color32::WHITE);
    }
    together
}

/// Alt, pointing at another than what is chosen: how far it is from it, across and down.
pub(super) fn measured(painter: &egui::Painter, read: &Read, view: View, together: Option<Rect>, pointer: Option<Pos2>) {
    let other = pointer.and_then(|at| {
        (0..read.items.len())
            .rev()
            .find(|at_one| view.on_screen(read.boxes[*at_one]).contains(at) && !read.selection.contains(&(read.items[*at_one].line, read.items[*at_one].at)))
    });
    let (Some(from), Some(other)) = (together, other) else { return };
    let to = read.boxes[other];
    let pink = Color32::from_rgb(0xFF, 0x4F, 0xD8);
    painter.rect_stroke(view.on_screen(to), 0.0, Stroke::new(1.0_f32, pink), StrokeKind::Outside);
    for (a, b, gap) in gaps_between(from, to) {
        painter.line_segment([view.to_screen([a.x, a.y]), view.to_screen([b.x, b.y])], Stroke::new(1.0_f32, pink));
        let middle = view.to_screen([(a.x + b.x) / 2.0, (a.y + b.y) / 2.0]);
        tag(painter, middle + vec2(4.0, 8.0), &format!("{}", gap.round()), pink);
    }
}

/// Around the decor: very close, the pixels' grid; outside it, darker; its edge, its size.
pub(super) fn around(painter: &egui::Painter, view: View) {
    let (rect, shown, seen) = (view.rect, view.shown, view.seen());
    if view.scale >= 8.0 {
        let stroke = Stroke::new(1.0_f32, Color32::from_black_alpha(40));
        let [x0, y0] = view.to_place(seen.min);
        let [x1, y1] = view.to_place(seen.max);
        for x in x0.ceil() as i64..=x1.floor() as i64 {
            let at = view.to_screen([x as f32, 0.0]).x;
            painter.line_segment([pos2(at, seen.min.y), pos2(at, seen.max.y)], stroke);
        }
        for y in y0.ceil() as i64..=y1.floor() as i64 {
            let at = view.to_screen([0.0, y as f32]).y;
            painter.line_segment([pos2(seen.min.x, at), pos2(seen.max.x, at)], stroke);
        }
    }
    let dark = Color32::from_black_alpha(90);
    for outside in [
        Rect::from_min_max(rect.min, pos2(rect.max.x, shown.min.y)),
        Rect::from_min_max(pos2(rect.min.x, shown.max.y), rect.max),
        Rect::from_min_max(pos2(rect.min.x, shown.min.y), pos2(shown.min.x, shown.max.y)),
        Rect::from_min_max(pos2(shown.max.x, shown.min.y), pos2(rect.max.x, shown.max.y)),
    ] {
        let outside = outside.intersect(rect);
        if outside.is_positive() {
            painter.rect_filled(outside, 0.0, dark);
        }
    }
    painter.rect_stroke(shown, 0.0, Stroke::new(1.0_f32, Color32::from_white_alpha(40)), StrokeKind::Outside);
    if rect.contains(shown.right_bottom() + vec2(-4.0, 14.0)) {
        painter.text(
            shown.right_bottom() + vec2(0.0, 4.0),
            egui::Align2::RIGHT_TOP,
            format!("{} × {}", view.size.x, view.size.y),
            egui::FontId::proportional(theme::SMALL),
            theme::FAINT,
        );
    }
}

/// The rulers, top and left, in pixels of the decor: a step that reads at
/// this closeness, the pointer marked.
pub(super) fn rulers(painter: &egui::Painter, view: View, pointer: Option<Pos2>) {
    const SIDE: f32 = 18.0;
    let rect = view.rect;
    let step = ruler_step(view.scale);
    let top = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + SIDE));
    let left = Rect::from_min_max(rect.min, pos2(rect.min.x + SIDE, rect.max.y));
    for band in [top, left] {
        painter.rect_filled(band, 0.0, Color32::from_rgb(0x22, 0x22, 0x22));
    }
    painter.hline(top.x_range(), top.max.y, Stroke::new(1.0_f32, theme::GAP));
    painter.vline(left.max.x, left.y_range(), Stroke::new(1.0_f32, theme::GAP));
    let font = egui::FontId::proportional(theme::SMALL - 2.0);
    let stroke = Stroke::new(1.0_f32, theme::FAINT);
    let [x0, y0] = view.to_place(pos2(left.max.x, top.max.y));
    let [x1, y1] = view.to_place(rect.max);
    let small = step / 4.0;
    let mut x = (x0 / small).floor() * small;
    while x <= x1 {
        let at = view.to_screen([x, 0.0]).x;
        if at > left.max.x {
            let whole = (x / step).round() * step == x;
            painter.vline(at, (top.max.y - if whole { 8.0 } else { 3.0 })..=top.max.y, stroke);
            if whole {
                painter.text(pos2(at + 2.0, top.min.y + 1.0), egui::Align2::LEFT_TOP, format!("{x}"), font.clone(), theme::WEAK);
            }
        }
        x += small;
    }
    let mut y = (y0 / small).floor() * small;
    while y <= y1 {
        let at = view.to_screen([0.0, y]).y;
        if at > top.max.y {
            let whole = (y / step).round() * step == y;
            painter.hline((left.max.x - if whole { 8.0 } else { 3.0 })..=left.max.x, at, stroke);
            if whole {
                painter.text(pos2(left.min.x + 2.0, at + 2.0), egui::Align2::LEFT_TOP, format!("{y}"), font.clone(), theme::WEAK);
            }
        }
        y += small;
    }
    if let Some(at) = pointer {
        let mark = Stroke::new(1.0_f32, theme::DECOR);
        if at.x > left.max.x {
            painter.vline(at.x, top.y_range(), mark);
        }
        if at.y > top.max.y {
            painter.hline(left.x_range(), at.y, mark);
        }
    }
    painter.rect_filled(Rect::from_min_size(rect.min, vec2(SIDE, SIDE)), 0.0, Color32::from_rgb(0x22, 0x22, 0x22));
}
