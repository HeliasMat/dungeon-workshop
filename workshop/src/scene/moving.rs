//! The Scene's view moved: a map of all of it when it does not fit (a click
//! there goes there), the middle button, the wheel where the pointer is,
//! Space held, the keys (0, 1, 2, + and -, F), the bar of the view; where the pointer is.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};

use super::View;
use super::drawn::{Things, draw_all};
use workshop_base::theme;

/// How close and where the decor is seen: its closeness (none: to fit), how far it is moved.
pub(super) type Camera = (Option<f32>, Vec2);

/// The camera at another closeness, the middle of the room still on the same point of the decor.
pub(super) fn zoomed(view: View, camera: Camera, next: f32) -> Camera {
    let next = view.clamped(next);
    (Some(next), camera.1 * (next / view.scale))
}

/// The camera closer or further by a factor, the point under the pointer kept under it.
pub(super) fn zoomed_at(view: View, at: Pos2, factor: f32) -> Camera {
    let next = view.clamped(view.scale * factor);
    let [u, v] = view.to_place(at);
    let centre = at + view.size * next / 2.0 - vec2(u, v) * next;
    (Some(next), centre - view.rect.center())
}

/// The camera on that box of the decor, three times as large as it in the room (F).
pub(super) fn framed_on(room: Vec2, size: Vec2, together: Rect) -> Camera {
    let next = (room.x / (together.width().max(24.0) * 3.0)).min(room.y / (together.height().max(24.0) * 3.0)).clamp(0.25, 6.0);
    (Some(next), (size / 2.0 - together.center().to_vec2()) * next)
}

/// The map's size: as wide as asked (or as high, for a decor higher than wide), the decor's shape kept.
pub(super) fn map_size(wide: f32, size: Vec2) -> Vec2 {
    let map_size = vec2(wide, wide * size.y / size.x);
    if map_size.y > wide { vec2(wide * size.x / size.y, wide) } else { map_size }
}

/// The view moved by all that moves it; whether Space is held (a drag then moves the view).
#[allow(clippy::too_many_arguments)]
pub(super) fn moved(
    ui: &egui::Ui,
    response: &egui::Response,
    painter: &egui::Painter,
    view: View,
    things: &Things,
    together: Option<Rect>,
    pointer: Option<Pos2>,
    camera: &mut Camera,
) -> bool {
    if !view.rect.expand(2.0).contains_rect(view.shown) {
        map(ui, painter, view, things, camera);
    }
    // (Moved with the middle button; closer or further with the wheel, where the pointer is.)
    if response.dragged_by(egui::PointerButton::Middle) {
        camera.1 += response.drag_delta();
    }
    if let Some(at) = pointer {
        let (pinch, wheel) = ui.input(|input| (input.zoom_delta(), input.smooth_scroll_delta.y));
        let factor = pinch * (wheel * 0.0025).exp();
        if (factor - 1.0).abs() > 0.0005 {
            *camera = zoomed_at(view, at, factor);
        }
    }
    // (Space held: a drag moves the view.)
    let typing = ui.ctx().wants_keyboard_input();
    let space = !typing && ui.input(|input| input.key_down(egui::Key::Space));
    if space && response.dragged_by(egui::PointerButton::Primary) {
        camera.1 += response.drag_delta();
    }
    if response.hovered() && !typing {
        view_keys(ui, view, together, camera);
    }
    view_bar(ui, painter, view, pointer, camera);
    space
}

/// A map of all of it, bottom right, what is seen framed; a click or a drag there goes there.
fn map(ui: &egui::Ui, painter: &egui::Painter, view: View, things: &Things, camera: &mut Camera) {
    let (rect, size) = (view.rect, view.size);
    let map_size = map_size(200.0_f32.min(rect.width() * 0.3), size);
    let map = Rect::from_min_size(rect.right_bottom() - map_size - vec2(10.0, 10.0), map_size);
    let k = map_size.x / size.x;
    let on_map = move |area: Rect| Rect::from_min_max(map.min + area.min.to_vec2() * k, map.min + area.max.to_vec2() * k);
    let map_painter = painter.with_clip_rect(map);
    map_painter.rect_filled(map, 0.0, Color32::BLACK);
    draw_all(&map_painter, &things.lower, &on_map, Color32::WHITE, things.seconds);
    draw_all(&map_painter, &things.upper, &on_map, Color32::WHITE, things.seconds);
    let seen = view.seen();
    let [vx0, vy0] = view.to_place(seen.min);
    let [vx1, vy1] = view.to_place(seen.max);
    painter.rect_stroke(on_map(Rect::from_min_max(pos2(vx0, vy0), pos2(vx1, vy1))), 0.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Inside);
    painter.rect_stroke(map, 0.0, Stroke::new(1.0_f32, theme::GAP), StrokeKind::Outside);
    let on_it = ui.interact(map, ui.id().with("decor map"), Sense::click_and_drag());
    if (on_it.clicked() || on_it.dragged())
        && let Some(at) = on_it.interact_pointer_pos()
    {
        let point = (at - map.min) / k;
        *camera = (Some(view.scale), (size / 2.0 - point) * view.scale);
    }
}

/// The keys over the scene: 0 all of it, 1 its pixels as they are, 2 twice, + and - closer and further, F what is chosen.
fn view_keys(ui: &egui::Ui, view: View, together: Option<Rect>, camera: &mut Camera) {
    let (all, one, two, closer, further, frame) = ui.input(|input| {
        let plain = !input.modifiers.command;
        (
            plain && input.key_pressed(egui::Key::Num0),
            plain && input.key_pressed(egui::Key::Num1),
            plain && input.key_pressed(egui::Key::Num2),
            input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals),
            input.key_pressed(egui::Key::Minus),
            plain && input.key_pressed(egui::Key::F),
        )
    });
    if all {
        *camera = (None, Vec2::ZERO);
    }
    if one {
        *camera = zoomed(view, *camera, 1.0);
    }
    if two {
        *camera = zoomed(view, *camera, 2.0);
    }
    if closer {
        *camera = zoomed(view, *camera, view.scale * 1.25);
    }
    if further {
        *camera = zoomed(view, *camera, view.scale / 1.25);
    }
    if frame && let Some(together) = together {
        *camera = framed_on(view.rect.size(), view.size, together);
    }
}

/// The bar of the view, bottom left: how close, closer, further, all, 1:1; where the pointer is.
fn view_bar(ui: &egui::Ui, painter: &egui::Painter, view: View, pointer: Option<Pos2>, camera: &mut Camera) {
    let rect = view.rect;
    let bar = Rect::from_min_size(pos2(rect.min.x + 26.0, rect.max.y - 30.0), vec2(250.0, 24.0));
    painter.rect_filled(bar, 4.0, Color32::from_black_alpha(170));
    let mut x = bar.min.x + 4.0;
    for (word, said) in [("−", "further (-)"), ("+", "closer (+)"), ("All", "the whole decor (0)"), ("1:1", "its pixels as they are (1)")] {
        let wide = if word.len() > 2 { 34.0 } else { 22.0 };
        let button = Rect::from_min_size(pos2(x, bar.min.y + 2.0), vec2(wide, 20.0));
        let pressed = ui.interact(button, ui.id().with(("view", word)), Sense::click());
        if pressed.hovered() {
            painter.rect_filled(button, 3.0, Color32::from_white_alpha(30));
        }
        painter.text(button.center(), egui::Align2::CENTER_CENTER, word, egui::FontId::proportional(theme::BODY), theme::STRONG);
        if pressed.on_hover_text(said).clicked() {
            *camera = match word {
                "−" => zoomed(view, *camera, view.scale / 1.25),
                "+" => zoomed(view, *camera, view.scale * 1.25),
                "All" => (None, Vec2::ZERO),
                _ => zoomed(view, *camera, 1.0),
            };
        }
        x += wide + 2.0;
        if word == "+" {
            painter.text(
                pos2(x + 20.0, bar.center().y),
                egui::Align2::CENTER_CENTER,
                format!("{:.0} %", view.scale * 100.0),
                egui::FontId::proportional(theme::SMALL),
                theme::TEXT,
            );
            x += 42.0;
        }
    }
    if let Some(at) = pointer {
        let [px, py] = view.to_place(at);
        let said = format!("{} {} · tile {},{}", px.floor(), py.floor(), (px / 8.0).floor(), (py / 8.0).floor());
        painter.text(pos2(bar.max.x + 8.0, bar.center().y), egui::Align2::LEFT_CENTER, said, egui::FontId::monospace(theme::SMALL), Color32::WHITE);
    }
}
