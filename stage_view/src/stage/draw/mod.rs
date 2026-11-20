//! The stage drawn, the same for a film and for the game: sharp pixels, the
//! game's digits. It goes through what the engine draws, in its order
//! ([`Stage::drawing_order`](engine::Stage::drawing_order): the
//! sheets, each body, each picture, each halo, by their `z`), and draws
//! each; the order is not decided here. A node turned, sized, slanted or
//! told how it looks goes through the engine's affine blit (`shell::blit`).
//!
//! - `place`: the place, its light, a thing shown large, a picture instead;
//! - `bodies`: those of the stage, their lines, what moves throw;
//! - `emotes`: statuses, effects, digits over heads;
//! - `overlay` (beside `stage`): what goes over it in the game's pixels.

use engine::props::Blend;
use engine::{Drawn, Sheet, Viewport};
use glam::Vec2;

use super::Play;
use crate::overlay::{Look, Over, draw_over};
use crate::ui::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH, TILE, UiArt};
use shell::Screen;
use shell::art::Art;

mod bodies;
mod emotes;
mod place;

/// Where the lower screen starts on a canvas.
const BELOW: i32 = SCREEN_HEIGHT as i32;

/// How a node's blend is mixed by the blit.
pub(crate) fn mix_of(blend: Blend) -> shell::Mix {
    match blend {
        Blend::Normal => shell::Mix::Normal,
        Blend::Add => shell::Mix::Add,
        Blend::Multiply => shell::Mix::Multiply,
        Blend::Screen => shell::Mix::Screen,
        Blend::Overlay => shell::Mix::Overlay,
        Blend::Lighten => shell::Mix::Lighten,
        Blend::Darken => shell::Mix::Darken,
        Blend::Difference => shell::Mix::Difference,
    }
}

/// How a node is laid by the blit: multiplied by `tint` (of 255 each),
/// `alpha` of it shown, and as its looks say (a colour added, one filling
/// it, its mix, smoothed).
pub(crate) fn paint_of(looks: &engine::node::Looks, tint: [f32; 3], alpha: f32) -> shell::Paint {
    shell::Paint { tint, alpha, add: looks.add, fill: looks.fill, mix: mix_of(looks.blend), smooth: looks.smooth, below: None, fx: shell::Fx::default() }
}

/// What a node's effects do to each pixel (`seconds`: the time gone, for a
/// wave), and what they lay about it (`close`: the dest's pixels to one of
/// the scene, for their widths and offsets).
pub(crate) fn effects_of(effects: &engine::node::Effects, close: f32, seconds: f32) -> (shell::Fx, shell::Around) {
    let rgb = |[red, green, blue, _]: [u8; 4]| [red, green, blue];
    let fx = shell::Fx {
        dissolve: effects.dissolve,
        seed: effects.seed as i64 as u32,
        wave: effects.wave,
        wave_length: effects.wave_length,
        wave_phase: std::f32::consts::TAU * effects.wave_speed * seconds,
        swap: effects.swap_from.zip(effects.swap_to).map(|(from, to)| (rgb(from), rgb(to))),
        flash: effects.flash.map(|colour| (rgb(colour), effects.flash_amount * colour[3] as f32 / 255.0)),
    };
    let around = shell::Around {
        shadow: effects.shadow.map(|colour| (colour, effects.shadow_offset * close)),
        glow: effects.glow.filter(|_| effects.glow_width > 0.0).map(|colour| (colour, effects.glow_width * close)),
        outline: effects.outline.filter(|_| effects.outline_width > 0.0).map(|colour| (colour, effects.outline_width * close)),
    };
    (fx, around)
}

/// What the words on screen take of it, for bubbles to keep clear of: a line
/// is there, and a portrait at that column.
#[derive(Clone, Copy, Default)]
pub struct Clear {
    pub page: bool,
    pub portrait: Option<usize>,
    /// How bubbles look (see `Look::style`).
    pub bubbles: u8,
}

/// What a frame is drawn with: the view on the canvas and on the screen
/// (`large`), how the screen sways, the seconds gone, the light's colour on
/// the place and on the cast, how bubbles look.
struct Brush {
    view: Viewport,
    large: Viewport,
    sway: Vec2,
    seconds: f32,
    on_place: [u8; 4],
    on_cast: [u8; 4],
    look: Look,
}

impl Brush {
    /// Where a point of the place is on the canvas.
    fn on_screen(&self, at: Vec2) -> (i32, i32) {
        let at = self.view.on_screen(at);
        (at.x as i32, BELOW + at.y as i32)
    }

    /// Where a point of the place is on the screen, in all its pixels.
    fn in_world(&self, at: Vec2) -> (i32, i32) {
        let at = self.large.on_screen(at);
        (at.x as i32, at.y as i32)
    }
}

impl Play {
    /// The lower screen as it is now, on `world`, the canvas left with what
    /// goes over all of it (a caption): the words go there.
    pub fn draw(&mut self, canvas: &mut Canvas, world: &mut Screen, art: &Art, ui: &UiArt, clear: Clear) {
        self.draw_with_box(canvas, world, art, ui, clear, &mut |_| {});
    }

    /// The same, the dialogue box drawn by `the_box` on the canvas at its
    /// rank (`Sheet::Box`): under what is told a higher `z`.
    pub fn draw_with_box(&mut self, canvas: &mut Canvas, world: &mut Screen, art: &Art, ui: &UiArt, clear: Clear, the_box: &mut dyn FnMut(&mut Canvas)) {
        // (The one picture read as it is drawn, kept: then nothing changes.)
        if let Some(who) = self.sim.stage.words.large {
            let cast = &self.sim.cast;
            self.large_sprites.entry(who).or_insert_with(|| cast.large(art, who));
        }
        self.drawn(canvas, world, art, ui, clear, the_box);
    }

    /// The picture, from the scene as it is.
    fn drawn(&self, canvas: &mut Canvas, world: &mut Screen, art: &Art, ui: &UiArt, clear: Clear, the_box: &mut dyn FnMut(&mut Canvas)) {
        canvas.data.fill(0);
        world.clear();
        let brush = self.brush(world, clear);
        let mut on_canvas = false;
        let flat = |who| self.sim.shown.get(&who).is_some_and(|shown| shown.flat);
        for (_, drawn) in self.sim.stage.drawing_order(flat) {
            let in_game_pixels = match drawn {
                Drawn::Sheet(sheet) => sheet.in_game_pixels(),
                Drawn::Body(_) => false,
                Drawn::Picture(_) | Drawn::Halo(_) => true,
            };
            // (What was drawn in the game's pixels goes over the screen
            // before what is drawn in all of them.)
            if on_canvas && !in_game_pixels {
                world.over(&canvas.data);
                canvas.data.fill(0);
            }
            on_canvas = in_game_pixels;
            match drawn {
                Drawn::Sheet(Sheet::Box) => the_box(canvas),
                Drawn::Sheet(sheet) => self.draw_sheet(sheet, &brush, canvas, world, art, ui),
                Drawn::Body(who) => self.draw_body(world, &brush, who),
                one => self.with_over(&brush, canvas, art, ui, |over| crate::overlay::draw_one_over(one, over)),
            }
        }
    }

    /// What is drawn over the scene in the game's pixels, drawn by `draw`.
    fn with_over(&self, brush: &Brush, canvas: &mut Canvas, art: &Art, ui: &UiArt, draw: impl FnOnce(&mut Over)) {
        let digits = |text: &str| art.digits().is_some() && emotes::number_cells(text).is_some();
        let on_screen = |at: Vec2| brush.on_screen(at);
        let (close, seconds) = (brush.view.close, brush.seconds);
        // (Sand, fog and rain in the game's own pictures, as on a floor.)
        let shown = match self.sim.stage.light.weather {
            engine::Weather::Sand => Some(shell::art::Sky::Sandstorm),
            engine::Weather::Mist => Some(shell::art::Sky::Fog),
            engine::Weather::Rain => Some(shell::art::Sky::Rain),
            _ => None,
        };
        let layer = shown.and_then(|shown| art.weather(shown));
        let sky = layer.as_deref();
        let mut over =
            Over { canvas, art: ui, stage: &self.sim.stage, look: brush.look, below: BELOW, close, seconds, on_screen: &on_screen, digits: &digits, sky };
        draw(&mut over);
    }

    /// How this frame is drawn.
    fn brush(&self, world: &Screen, clear: Clear) -> Brush {
        let place = self.place_size();
        let screen = Vec2::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
        // (The screen rippling, see `ScreenBeat::Wave`: the view sways.)
        let seconds = self.sim.clock / 60.0;
        let sway = Vec2::new((seconds * 2.1).sin(), (seconds * 1.3).cos() * 0.6) * self.sim.stage.light.wave * 4.0;
        let view = Viewport::of(self.sim.focus.map(|focus| focus + sway), self.sim.stage.view.zoom.now, place, screen);
        // (The light's colour, more on the place than on the cast: who is in
        // the dark can still be seen.)
        const ON_CAST: f32 = 0.55;
        let [red, green, blue, much] = self.sim.stage.light.tint.now;
        let much = (much / 255.0).clamp(0.0, 1.0);
        let on_cast = much * ON_CAST;
        let on_place = 1.0 - (1.0 - much) / (1.0 - on_cast);
        let colour = |part: f32| [red as u8, green as u8, blue as u8, (part * 255.0) as u8];
        // (Bubbles keep clear of the line on screen and of its portrait.)
        let below_box = clear.page && !self.sim.stage.words.box_top;
        let corner = clear.portrait.filter(|_| below_box).map(|x| ((x * TILE) as i32, ((x + 6) * TILE) as i32, BELOW + (11 * TILE) as i32));
        let look = Look { style: clear.bubbles, floor: below_box.then_some(BELOW + (17 * TILE) as i32), corner, ..Look::default() };
        Brush { view, large: view.scaled(world.scale as f32), sway, seconds, on_place: colour(on_place), on_cast: colour(on_cast), look }
    }

    fn draw_sheet(&self, sheet: Sheet, brush: &Brush, canvas: &mut Canvas, world: &mut Screen, art: &Art, ui: &UiArt) {
        let whole = |world: &mut Screen, colour: [u8; 4]| world.fill(0, 0, world.wide as i32, world.high as i32, colour);
        match sheet {
            Sheet::Place => self.draw_place(world, brush, false),
            Sheet::PlaceUpper => self.draw_place(world, brush, true),
            Sheet::PlaceLight => whole(world, brush.on_place),
            Sheet::Bodies => self.draw_bodies(world, brush),
            // (Drawn by whoever has the words, at its rank.)
            Sheet::Box => {}
            Sheet::Lines => self.draw_lines(world, &brush.large, art),
            Sheet::Shots => self.draw_shots(world, brush, art),
            Sheet::Statuses => self.draw_statuses(world, brush, art),
            Sheet::Effects => self.draw_effects(world, brush, art),
            Sheet::CastLight => whole(world, brush.on_cast),
            Sheet::Large => self.draw_large(world),
            Sheet::Cutaway => self.draw_cutaway(world, brush.sway),
            Sheet::Banner => self.draw_banner(canvas, ui),
            Sheet::Digits => self.draw_digits(canvas, brush, art),
            Sheet::Glows => self.draw_glows(world, brush),
            // (Without colours, then the light's own laid over again: a
            // memory's sepia is grey under a warm brown, as an old picture.)
            Sheet::Grey if self.sim.stage.light.grey => {
                world.grey();
                let [red, green, blue, much] = self.sim.stage.light.tint.now;
                if much >= 1.0 {
                    whole(world, [red as u8, green as u8, blue as u8, much.min(255.0) as u8]);
                }
            }
            Sheet::Grey => {}
            // (The fade over all of that, but for a caption: a title on a dark screen.)
            Sheet::Fade => {
                let fade = self.sim.stage.light.fade;
                let shade = if fade.white { 255 } else { 0 };
                whole(world, [shade, shade, shade, (fade.shown() * 255.0) as u8]);
            }
            _ => self.with_over(brush, canvas, art, ui, |over| draw_over(sheet, over)),
        }
    }
}
