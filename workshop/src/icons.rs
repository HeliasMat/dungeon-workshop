//! Small icons drawn with lines, sharp at any size (the font has not all
//! the signs a studio needs): a chevron, an eye, a lock, a magnifier, a
//! plus, folding all, a folder.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Sense, Stroke, pos2, vec2};

use crate::theme;

/// A chevron: pointing right (folded) or down (open).
pub fn chevron(painter: &Painter, centre: Pos2, open: bool, colour: Color32) {
    let stroke = Stroke::new(1.4_f32, colour);
    let points = if open {
        vec![centre + vec2(-3.5, -1.5), centre + vec2(0.0, 2.0), centre + vec2(3.5, -1.5)]
    } else {
        vec![centre + vec2(-1.5, -3.5), centre + vec2(2.0, 0.0), centre + vec2(-1.5, 3.5)]
    };
    painter.add(egui::Shape::line(points, stroke));
}

/// An eye, open or struck through.
pub fn eye(painter: &Painter, rect: Rect, open: bool, colour: Color32) {
    let centre = rect.center();
    let stroke = Stroke::new(1.2_f32, colour);
    let wide = rect.width() * 0.42;
    let top: Vec<Pos2> = (0..=8)
        .map(|at| {
            let t = at as f32 / 8.0 * std::f32::consts::PI;
            pos2(centre.x - wide * t.cos(), centre.y - wide * 0.55 * t.sin())
        })
        .collect();
    let bottom: Vec<Pos2> = top.iter().map(|point| pos2(point.x, 2.0 * centre.y - point.y)).collect();
    painter.add(egui::Shape::line(top, stroke));
    painter.add(egui::Shape::line(bottom, stroke));
    painter.circle_filled(centre, 1.8, colour);
    if !open {
        painter.line_segment([centre + vec2(-wide, wide), centre + vec2(wide, -wide)], stroke);
    }
}

/// A lock, shut or open.
pub fn lock(painter: &Painter, rect: Rect, shut: bool, colour: Color32) {
    let centre = rect.center();
    let stroke = Stroke::new(1.2_f32, colour);
    let body = Rect::from_center_size(centre + vec2(0.0, 2.0), vec2(8.0, 6.0));
    painter.rect_stroke(body, 1.0, stroke, egui::StrokeKind::Inside);
    let lift = if shut { 0.0 } else { 2.5 };
    let arc: Vec<Pos2> = (0..=8)
        .map(|at| {
            let t = at as f32 / 8.0 * std::f32::consts::PI;
            pos2(centre.x - 2.5 * t.cos() + lift, body.top() - 3.5 * t.sin())
        })
        .collect();
    painter.add(egui::Shape::line(arc, stroke));
}

/// A magnifier.
pub fn search(painter: &Painter, rect: Rect, colour: Color32) {
    let centre = rect.center() + vec2(-1.5, -1.5);
    let stroke = Stroke::new(1.3_f32, colour);
    painter.circle_stroke(centre, 3.6, stroke);
    painter.line_segment([centre + vec2(2.6, 2.6), centre + vec2(5.5, 5.5)], stroke);
}

/// A plus.
pub fn plus(painter: &Painter, rect: Rect, colour: Color32) {
    let centre = rect.center();
    let stroke = Stroke::new(1.3_f32, colour);
    painter.line_segment([centre + vec2(-4.5, 0.0), centre + vec2(4.5, 0.0)], stroke);
    painter.line_segment([centre + vec2(0.0, -4.5), centre + vec2(0.0, 4.5)], stroke);
}

/// Folding all: two boxes, a minus.
pub fn fold_all(painter: &Painter, rect: Rect, colour: Color32) {
    let centre = rect.center();
    let stroke = Stroke::new(1.1_f32, colour);
    painter.rect_stroke(Rect::from_center_size(centre + vec2(1.5, 1.5), vec2(9.0, 9.0)), 1.0, stroke, egui::StrokeKind::Inside);
    painter.line_segment([centre + vec2(-1.5, 1.5), centre + vec2(4.5, 1.5)], stroke);
    painter.line_segment([centre + vec2(-4.5, -4.5), centre + vec2(3.0, -4.5)], stroke);
    painter.line_segment([centre + vec2(-4.5, -4.5), centre + vec2(-4.5, 3.0)], stroke);
}

/// A folder (a group), open or shut.
pub fn folder(painter: &Painter, rect: Rect, open: bool, colour: Color32) {
    let base = Rect::from_center_size(rect.center() + vec2(0.0, 1.0), vec2(12.0, 8.5));
    let tab = Rect::from_min_size(base.left_top() + vec2(0.0, -2.0), vec2(5.0, 2.5));
    painter.rect_filled(tab, 1.0, colour);
    if open {
        painter.rect_stroke(base, 1.0, Stroke::new(1.2_f32, colour), egui::StrokeKind::Inside);
        painter.rect_filled(Rect::from_min_max(base.left_top() + vec2(2.0, 3.0), base.right_bottom()), 1.0, colour.gamma_multiply(0.7));
    } else {
        painter.rect_filled(base, 1.0, colour);
    }
}

/// A small button with an icon on it; whether it was clicked.
pub fn button(ui: &mut egui::Ui, rect: Rect, said: &str, draw: impl Fn(&Painter, Rect, Color32)) -> bool {
    let response = ui.interact(rect, ui.id().with(("icon", said, rect.min.x as i32, rect.min.y as i32)), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 3.0, Color32::from_white_alpha(22));
    }
    draw(ui.painter(), rect, if response.hovered() { theme::STRONG } else { theme::TEXT });
    response.on_hover_text(said).clicked()
}

/// Back to what it is by default: an arrow turning back.
pub fn reset(painter: &Painter, rect: Rect, colour: Color32) {
    let centre = rect.center();
    let stroke = Stroke::new(1.2_f32, colour);
    let arc: Vec<Pos2> = (0..=12)
        .map(|at| {
            let t = -0.6 + at as f32 / 12.0 * 4.6;
            centre + vec2(t.cos() * 4.5, t.sin() * 4.5)
        })
        .collect();
    let end = arc[0];
    painter.add(egui::Shape::line(arc, stroke));
    painter.line_segment([end, end + vec2(-3.0, -0.5)], stroke);
    painter.line_segment([end, end + vec2(0.5, -3.0)], stroke);
}

/// An arrow from the middle of a square toward one of the eight ways
/// (`decor::lang::FACING`: down, downright, right, …).
pub fn way(painter: &Painter, rect: Rect, facing: usize, colour: Color32) {
    let angle = std::f32::consts::FRAC_PI_2 - facing as f32 * std::f32::consts::FRAC_PI_4;
    let to = vec2(angle.cos(), angle.sin());
    let centre = rect.center();
    let stroke = Stroke::new(1.5_f32, colour);
    let tip = centre + to * 5.0;
    painter.line_segment([centre - to * 4.0, tip], stroke);
    let side = vec2(-to.y, to.x);
    painter.line_segment([tip, tip - to * 3.0 + side * 2.5], stroke);
    painter.line_segment([tip, tip - to * 3.0 - side * 2.5], stroke);
}

/// A cross: taken out, closed.
pub fn cross(painter: &Painter, rect: Rect, colour: Color32) {
    let centre = rect.center();
    let stroke = Stroke::new(1.3_f32, colour);
    painter.line_segment([centre + vec2(-3.5, -3.5), centre + vec2(3.5, 3.5)], stroke);
    painter.line_segment([centre + vec2(-3.5, 3.5), centre + vec2(3.5, -3.5)], stroke);
}

/// A star, filled (a favourite) or drawn.
pub fn star(painter: &Painter, rect: Rect, filled: bool, colour: Color32) {
    let centre = rect.center();
    let points: Vec<Pos2> = (0..10)
        .map(|at| {
            let angle = -std::f32::consts::FRAC_PI_2 + at as f32 * std::f32::consts::PI / 5.0;
            let far = if at % 2 == 0 { 6.5 } else { 2.8 };
            centre + vec2(angle.cos() * far, angle.sin() * far)
        })
        .collect();
    if filled {
        // (Drawn as triangles from the middle: a star is not convex.)
        for at in 0..10 {
            painter.add(egui::Shape::convex_polygon(vec![centre, points[at], points[(at + 1) % 10]], colour, Stroke::NONE));
        }
    } else {
        let mut closed = points.clone();
        closed.push(points[0]);
        painter.add(egui::Shape::line(closed, Stroke::new(1.2_f32, colour)));
    }
}

/// A wave: it lives (the game animates it).
pub fn wave(painter: &Painter, rect: Rect, colour: Color32) {
    let points: Vec<Pos2> = (0..=12)
        .map(|at| {
            let t = at as f32 / 12.0;
            pos2(rect.min.x + t * rect.width(), rect.center().y - (t * std::f32::consts::TAU).sin() * rect.height() * 0.35)
        })
        .collect();
    painter.add(egui::Shape::line(points, Stroke::new(1.3_f32, colour)));
}
