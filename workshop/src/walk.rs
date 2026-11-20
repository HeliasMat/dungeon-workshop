//! A Pokémon walked in the decor, to try where one walks: moved by the
//! arrows (or ZQSD, WASD), stopped by the walls of the decor as it was baked
//! last (by tiles of 8, as the game has them), turned the way it goes,
//! walking as in the game; a click puts it there. The scene follows it.

use eframe::egui::{self, Color32, Pos2, Rect, pos2, vec2};

use crate::decors::{Decors, section, tag};
use crate::edit::{self, Kind};
use crate::theme;

/// How fast one walks, in pixels a second (as the game, about).
const PACE: f32 = 96.0;

/// Someone walked: his feet, the way he faces (`decor::lang::FACING`), whether he walks.
#[derive(Clone, Copy, Debug)]
pub struct Walker {
    pub at: [f32; 2],
    pub facing: usize,
    pub moving: bool,
}

/// Where one does not walk: the decor's tiles walled as baked, its edges.
fn blocked(decors: &Decors, [x, y]: [f32; 2], [w, h]: [f32; 2]) -> bool {
    // (His feet: a small box over the point he stands on.)
    let (x0, x1, y0, y1) = (x - 5.0, x + 5.0, y - 4.0, y);
    if x0 < 0.0 || y0 < 0.0 || x1 >= w || y1 >= h {
        return true;
    }
    let Some(walls) = &decors.walls_grid else {
        return false;
    };
    for row in (y0 / 8.0).floor() as usize..=(y1 / 8.0).floor() as usize {
        for column in (x0 / 8.0).floor() as usize..=(x1 / 8.0).floor() as usize {
            if column < walls.columns && row < walls.rows && walls.tiles[row * walls.columns + column] {
                return true;
            }
        }
    }
    false
}

/// The way one faces going so.
fn facing_of(dx: f32, dy: f32) -> usize {
    match (dx.signum() as i32 * i32::from(dx != 0.0), dy.signum() as i32 * i32::from(dy != 0.0)) {
        (0, 1) => 0,
        (1, 1) => 1,
        (1, 0) => 2,
        (1, -1) => 3,
        (0, -1) => 4,
        (-1, -1) => 5,
        (-1, 0) => 6,
        _ => 7,
    }
}

/// Where one begins: the decor's first entrance, else the middle of what is seen.
fn start(decors: &Decors) -> [f32; 2] {
    let Some(open) = &decors.open else { return [0.0, 0.0] };
    edit::items(&open.text).into_iter().find(|one| one.kind == Kind::Entrance).map_or(open.middle, |one| one.corner)
}

/// The walker under the keys, drawn over the scene; the scene following him.
pub fn walk(
    ui: &egui::Ui,
    decors: &mut Decors,
    response: &egui::Response,
    painter: &egui::Painter,
    to_screen: &dyn Fn([f32; 2]) -> Pos2,
    to_place: &dyn Fn(Pos2) -> [f32; 2],
    scale: f32,
) {
    let Some(open) = &decors.open else { return };
    let size = open.size();
    let mut walker = decors.walker.unwrap_or(Walker { at: start(decors), facing: 0, moving: false });
    // (A click: there, if one may stand there.)
    if response.clicked()
        && let Some(at) = response.interact_pointer_pos()
    {
        let point = to_place(at);
        if !blocked(decors, point, size) {
            walker.at = point;
        }
    }
    let (dx, dy, dt) = ui.input(|input| {
        let down = |keys: &[egui::Key]| keys.iter().any(|key| input.key_down(*key));
        let dx =
            f32::from(u8::from(down(&[egui::Key::ArrowRight, egui::Key::D]))) - f32::from(u8::from(down(&[egui::Key::ArrowLeft, egui::Key::Q, egui::Key::A])));
        let dy =
            f32::from(u8::from(down(&[egui::Key::ArrowDown, egui::Key::S]))) - f32::from(u8::from(down(&[egui::Key::ArrowUp, egui::Key::Z, egui::Key::W])));
        (dx, dy, input.stable_dt.min(0.05))
    });
    let typing = ui.ctx().wants_keyboard_input();
    walker.moving = (dx != 0.0 || dy != 0.0) && !typing;
    if walker.moving {
        walker.facing = facing_of(dx, dy);
        let length = (dx * dx + dy * dy).sqrt();
        let step = [dx / length * PACE * dt, dy / length * PACE * dt];
        // (Against a wall: along it, if one way is free.)
        for tried in [step, [step[0], 0.0], [0.0, step[1]]] {
            let next = [walker.at[0] + tried[0], walker.at[1] + tried[1]];
            if (tried[0] != 0.0 || tried[1] != 0.0) && !blocked(decors, next, size) {
                walker.at = next;
                break;
            }
        }
    }
    // (Drawn, walking or still.)
    let word = decors.walker_word.trim().to_string();
    let seconds = if walker.moving { ui.input(|input| input.time) } else { 0.0 };
    let feet = to_screen(walker.at);
    match decors.body_now(&format!("{word}@walk")) {
        Some(body) => {
            if let Some((texture, around)) = body.at(walker.facing, seconds) {
                let place = Rect::from_min_max(
                    to_screen([walker.at[0] + around.min.x, walker.at[1] + around.min.y]),
                    to_screen([walker.at[0] + around.max.x, walker.at[1] + around.max.y]),
                );
                painter.image(texture.id(), place, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
            }
        }
        None => _ = painter.circle_filled(feet - vec2(0.0, 8.0 * scale), (7.0 * scale).max(3.0), theme::FILM),
    }
    let stuck = blocked(decors, walker.at, size);
    tag(
        painter,
        feet + vec2(10.0, 4.0),
        &format!("{} {}{}", walker.at[0].round(), walker.at[1].round(), if stuck { " · in a wall" } else { "" }),
        if stuck { theme::WRONG } else { Color32::WHITE },
    );
    // (The scene following him: never too near its edges.)
    let room = response.rect.shrink(80.0);
    let clamped = pos2(feet.x.clamp(room.min.x, room.max.x.max(room.min.x)), feet.y.clamp(room.min.y, room.max.y.max(room.min.y)));
    if let Some(open) = &mut decors.open
        && clamped != feet
    {
        if open.scale.is_none() {
            open.scale = Some(open.seen_scale);
        }
        open.pan += clamped - feet;
    }
    decors.walker = Some(walker);
    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    ui.ctx().request_repaint();
}

/// The walker's panel: who walks, from where; what stops him.
pub fn walk_panel(ui: &mut egui::Ui, decors: &mut Decors) {
    section(ui, "Walk");
    ui.add_space(4.0);
    ui.label(theme::weak("Arrows (or WASD): walk · click: go there. The walls are those of the last bake, in pale red."));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(theme::weak("Who"));
        ui.add(egui::TextEdit::singleline(&mut decors.walker_word).hint_text("pikachu").desired_width(120.0));
    });
    ui.add_space(6.0);
    if ui.button("Back to the Entrance").on_hover_text("the decor's first entrance, else the middle of the view").clicked() {
        decors.walker = Some(Walker { at: start(decors), facing: 0, moving: false });
    }
    ui.add_space(6.0);
    let baked = decors.open.as_ref().is_some_and(|open| open.is_baked());
    match (&decors.walls_grid, baked) {
        (None, _) => _ = ui.label(theme::weak("working out the walls…")),
        (Some(_), false) => _ = ui.label(theme::weak("the decor changed: its walls are worked out again…")),
        (Some(walls), true) => {
            _ = ui.label(theme::weak(format!("{} tiles walled out of {}", walls.tiles.iter().filter(|wall| **wall).count(), walls.tiles.len())))
        }
    }
}
