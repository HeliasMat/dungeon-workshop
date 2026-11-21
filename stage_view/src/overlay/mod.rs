//! What a scene shows over its lower screen besides its characters, in the
//! game's own pixels: its light and what falls over it, flashes, bubbles over
//! heads (in the look the player chose: `Look::style`), film bands; over
//! the fade, captions and what a scene keeps on screen. Each is a `Sheet`
//! of the engine's, drawn when its turn comes (`draw_over`).
//!
//! - `marks`: the light, what falls, lights, halos, pictures, bolts, keys;
//! - `bubbles`: bubbles, windows and captions;
//! - `over_fade`: what is drawn over the fade.

use std::collections::HashMap;
use std::sync::Arc;

use glam::Vec2;

use crate::ui::{self, Canvas, UiArt};
use engine::Weather as Sky;
use engine::*;

mod bubbles;
mod marks;
mod over_fade;
#[cfg(test)]
mod tests;

use bubbles::*;
use marks::*;

/// Lines of text are this many pixels apart in bubbles and captions.
const PITCH: i32 = 13;

/// How a scene's view is drawn.
#[derive(Clone, Copy, Debug, Default)]
pub struct Look {
    /// How bubbles are drawn, as the player chose: 0 plain, 1 framed as the
    /// game's windows are.
    pub style: u8,
    /// With the light's colour; else whoever calls lays it.
    pub tinted: bool,
    /// What the screen shows over the scene, for bubbles to keep clear of:
    /// the top of a box as wide as the screen; a box of a portrait, its
    /// left, right and top.
    pub floor: Option<i32>,
    pub corner: Option<(i32, i32, i32)>,
}

/// What the sheets over the scene are drawn with: the stage, how they look,
/// where the lower screen starts, how near the view is, the seconds gone,
/// and where a point of the place is on the canvas.
pub(crate) struct Over<'a> {
    pub canvas: &'a mut Canvas,
    pub art: &'a UiArt,
    pub stage: &'a Stage,
    pub look: Look,
    pub below: i32,
    pub close: f32,
    pub seconds: f32,
    pub on_screen: &'a dyn Fn(Vec2) -> (i32, i32),
    /// Words rising that are numbers are drawn in the game's digits.
    pub digits: &'a dyn Fn(&str) -> bool,
    /// The game's own picture of the weather, when it has one (sand and
    /// fog drifting, rain falling): drawn instead of the engine's strokes.
    pub sky: Option<&'a shell::art::WeatherLayer>,
}

/// Draws a sheet over the scene, if it is one of these.
pub(crate) fn draw_over(sheet: Sheet, over: &mut Over) {
    let (art, stage, below, close, seconds, look) = (over.art, over.stage, over.below, over.close, over.seconds, over.look);
    let (on_screen, sky) = (over.on_screen, over.sky);
    let canvas = &mut *over.canvas;
    match sheet {
        Sheet::Weather => draw_air(canvas, stage, below, seconds, look.tinted, sky),
        Sheet::Lights if stage.light.lights.is_empty() => draw_spotlight(canvas, &stage.light.spotlight, below, close, &on_screen),
        Sheet::Lights => draw_lights(canvas, stage, below, close, &on_screen),
        Sheet::Halos => draw_halos(canvas, stage, below, close, &on_screen),
        Sheet::Pictures => draw_pictures(canvas, stage, close, seconds, &on_screen),
        Sheet::Bolts => draw_bolts(canvas, stage, below, close, &on_screen),
        Sheet::Ripples => draw_ripples(canvas, stage, below, seconds),
        Sheet::Particles => draw_particles(canvas, stage, close, on_screen),
        Sheet::Bars => draw_bars(canvas, stage, close, on_screen),
        Sheet::Popups => draw_popups(canvas, art, stage, on_screen, over.digits),
        Sheet::Bubbles => draw_bubbles(canvas, art, stage, look, below, close, on_screen),
        Sheet::Bands => draw_bands(canvas, stage, below),
        Sheet::Wipe => over_fade::draw_wipe(canvas, stage, below),
        Sheet::Credits => over_fade::draw_credits(canvas, art, stage, below),
        Sheet::Caption => over_fade::draw_caption(canvas, art, stage, below),
        Sheet::Hud => over_fade::draw_hud(canvas, art, stage, below),
        Sheet::Log => over_fade::draw_log(canvas, art, stage, below),
        Sheet::Gauges => over_fade::draw_gauges(canvas, stage, below),
        Sheet::Minimap => over_fade::draw_minimap(canvas, stage, below),
        _ => {}
    }
}

/// Draws one of those drawn each at its own rank (a picture, a halo; see
/// `Stage::drawing_order`).
pub(crate) fn draw_one_over(drawn: Drawn, over: &mut Over) {
    let (stage, below, close, seconds) = (over.stage, over.below, over.close, over.seconds);
    let on_screen = over.on_screen;
    match drawn {
        Drawn::Picture(at) => draw_picture(over.canvas, stage, at, close, seconds, &on_screen),
        Drawn::Halo(at) => draw_halo(over.canvas, stage, at, below, close, &on_screen),
        Drawn::Sheet(sheet) => draw_over(sheet, over),
        Drawn::Body(_) => {}
    }
}

/// How far the film bands show, in pixels.
fn band(stage: &Stage) -> i32 {
    (stage.words.letterbox.shown * 20.0) as i32
}

/// The screen rippling: pale bands going down it, see `Beat::Wave`.
fn draw_ripples(canvas: &mut Canvas, stage: &Stage, below: i32, seconds: f32) {
    if stage.light.wave > 0.01 {
        for row in (0..ui::SCREEN_HEIGHT as i32).step_by(2) {
            let alpha = stage.light.wave * 40.0 * (0.5 + 0.5 * (row as f32 * 0.15 - seconds * 3.0).sin());
            canvas.fill(0, below + row, ui::SCREEN_WIDTH, 1, [255, 255, 255, alpha as u8]);
        }
    }
}

/// The particles in the game's pixels: small squares of their colour, fading
/// as they die (fireflies blinking), rings widening flat on the ground. Those
/// that glow are drawn later.
fn draw_particles(canvas: &mut Canvas, stage: &Stage, close: f32, on_screen: &dyn Fn(Vec2) -> (i32, i32)) {
    let size = (1.5 * close).round().max(1.0) as usize;
    for particle in stage.light.particles.iter().filter(|particle| particle.sharp) {
        let (x, y) = on_screen(particle.at);
        let mut alpha = (particle.left / particle.life).clamp(0.0, 1.0);
        if particle.blinks {
            alpha *= 0.5 + 0.5 * (particle.left * 0.15).sin();
        }
        let [red, green, blue] = particle.colour;
        let colour = [red, green, blue, (alpha * 255.0) as u8];
        if particle.ring > 0.0 {
            // (Seen from above at a slant: wide, and flat.)
            let wide = (2.0 + (1.0 - particle.left / particle.life) * particle.ring) * close;
            let steps = (wide * 5.0).max(12.0) as i32;
            for step in 0..steps {
                let (sin, cos) = (step as f32 / steps as f32 * std::f32::consts::TAU).sin_cos();
                canvas.fill(x + (cos * wide).round() as i32, y + (sin * wide * 0.4).round() as i32, size, 1, colour);
            }
            continue;
        }
        canvas.fill(x, y, size, size, colour);
    }
}

/// The little bars of those in a fight (see `BodyBeat::Bar`).
fn draw_bars(canvas: &mut Canvas, stage: &Stage, close: f32, on_screen: &dyn Fn(Vec2) -> (i32, i32)) {
    for actor in stage.cast.values().filter(|actor| actor.attributes & engine::attribute::HIDDEN == 0) {
        let Some(share) = actor.bar else {
            continue;
        };
        // (No wider than who it is over, seen from far.)
        let wide = (14.0 * close.clamp(0.6, 1.0)) as i32;
        let (x, y) = on_screen(actor.shown_at());
        let top = y - ((HEAD * actor.scale() + actor.shown_height()) * close) as i32;
        let colour = match share {
            0.5.. => [80, 220, 100, 255],
            0.2.. => [240, 200, 60, 255],
            _ => [240, 60, 50, 255],
        };
        canvas.fill(x - wide / 2 - 1, top - 1, wide as usize + 2, 4, [0, 0, 0, 220]);
        canvas.fill(x - wide / 2, top, (wide as f32 * share).ceil() as usize, 2, colour);
    }
}

/// The words rising from someone (see `Beat::Popup`), but numbers drawn in
/// the game's digits.
fn draw_popups(canvas: &mut Canvas, art: &UiArt, stage: &Stage, on_screen: &dyn Fn(Vec2) -> (i32, i32), digits: &dyn Fn(&str) -> bool) {
    for popup in stage.emotes.popups.iter().filter(|popup| !digits(&popup.text)) {
        let (x, y) = on_screen(popup.at);
        let wide = art.font.width(&popup.text) as i32 + 6;
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            centred_lines(canvas, art, &[popup.text.as_str()], x - wide / 2 + dx, y + 1 + dy, wide, [0, 0, 0]);
        }
        centred_lines(canvas, art, &[popup.text.as_str()], x - wide / 2, y + 1, wide, popup.colour);
    }
}

/// Each bubble over its head; those that would cover one already there are
/// moved aside (or up, against an edge of the screen).
fn draw_bubbles(canvas: &mut Canvas, art: &UiArt, stage: &Stage, look: Look, below: i32, close: f32, on_screen: &dyn Fn(Vec2) -> (i32, i32)) {
    let Look { style, floor, corner, .. } = look;
    let mut placed: Vec<Placed> = Vec::new();
    // (Under the band, and under a caption.)
    let boxed = if stage.words.boxed && stage.words.box_top { 48 } else { 0 };
    let ceiling = below + band(stage) + boxed + stage.words.caption.as_ref().map_or(0, |caption| 18 + caption.text.lines().count() as i32 * PITCH);
    for bubble in &stage.words.bubbles {
        let Some(actor) = stage.cast.get(&bubble.who) else {
            continue;
        };
        // (Over its head: about the height of a sprite, and its jump.)
        let (x, y) = on_screen(actor.position);
        // (Someone out of view says nothing to be seen: a bubble pushed to
        // a corner would point at no one.)
        const MARGIN: i32 = 16;
        if !(-MARGIN..ui::SCREEN_WIDTH as i32 + MARGIN).contains(&x) || !(below - MARGIN..below + ui::SCREEN_HEIGHT as i32 + 60).contains(&y) {
            continue;
        }
        let tip = y - ((HEAD * actor.scale() + actor.shown_height()) * close) as i32;
        let lines: Vec<&str> = bubble.text.split('\n').collect();
        let shape = Shape::of(art, &lines, style, bubble.voice);
        let mut place = shape.over((x, tip), ceiling);
        place.clear_of(&placed, ceiling);
        place.keep_off(floor, corner);
        match style {
            1 => window_bubble(canvas, art, &lines, bubble.voice, &shape, &place, x),
            _ => classic_bubble(canvas, art, &lines, bubble.voice, &shape, &place, x),
        }
        placed.push(place);
    }
}

/// The film bands, at the top and the bottom.
fn draw_bands(canvas: &mut Canvas, stage: &Stage, below: i32) {
    let band = band(stage);
    if band > 0 {
        canvas.fill(0, below, ui::SCREEN_WIDTH, band as usize, [0, 0, 0, 255]);
        canvas.fill(0, below + ui::SCREEN_HEIGHT as i32 - band, ui::SCREEN_WIDTH, band as usize, [0, 0, 0, 255]);
    }
}

/// The lines said lately (L), the last at the bottom, over the lower
/// screen.
pub fn draw_backlog(canvas: &mut Canvas, art: &UiArt, said: &[String], below: i32) {
    canvas.fill(0, below, ui::SCREEN_WIDTH, ui::SCREEN_HEIGHT, [0, 0, 0, 215]);
    let rows: Vec<&str> = said.iter().flat_map(|line| line.split('\n')).collect();
    let fit = ((ui::SCREEN_HEIGHT as i32 - 8) / PITCH) as usize;
    for (row, line) in rows[rows.len().saturating_sub(fit)..].iter().enumerate() {
        canvas.markup(art, 6, below + 4 + row as i32 * PITCH, ui::SCREEN_WIDTH - 12, line);
    }
}
