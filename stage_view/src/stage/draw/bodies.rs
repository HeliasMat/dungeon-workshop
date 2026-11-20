//! Those of the stage drawn, by depth: each with its shadow, what it wears,
//! a glint if it shines, the place's upper layer over who stands behind it;
//! their fishing lines; what moves throw; the particles that glow.

use engine::NodeRef;
use engine::node::{Looks, Transform};
use engine::{Viewport, Who, attribute, towards};
use glam::{Affine2, Vec2};
use shell::Screen;
use shell::art::Art;
use shell::art_files::SHADOWS;

use super::{Brush, Play};
use crate::stage::pose::way_of;

/// The line where one goes into the water, of whole pixels by turns: light,
/// darker; where one goes into the sand.
const FOAM: [[u8; 3]; 2] = [[248, 252, 255], [176, 224, 248]];
const SAND: [[u8; 3]; 2] = [[231, 175, 119], [183, 103, 79]];

/// A little star of whole pixels, going round the corners of a thing's picture.
fn glint(world: &mut Screen, (x, y): (i32, i32), frame: &shell::images::Frame, close: f32, clock: usize) {
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    let pixel = close.round().max(1.0) as i32;
    let (half_wide, half_high) = (frame.wide as i32 * pixel / 3, frame.high as i32 * pixel / 3);
    let (at, grown) = (clock / 12 % 4, (clock / 6).is_multiple_of(2));
    let (dx, dy) = [(1, -1), (-1, 1), (1, 1), (-1, -1)][at];
    let (x, y) = (x + dx * half_wide, y + dy * half_high);
    world.fill(x, y, pixel, pixel, WHITE);
    if grown {
        for (side_x, side_y) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            world.fill(x + side_x * pixel, y + side_y * pixel, pixel, pixel, WHITE);
        }
    }
}

/// The lowest row of a picture with something drawn in it.
fn lowest_row(frame: &shell::images::Frame) -> usize {
    let pixels = frame.rgba.as_chunks::<4>().0;
    (0..frame.high).rev().find(|&row| pixels[row * frame.wide..(row + 1) * frame.wide].iter().any(|pixel| pixel[3] > 0)).unwrap_or(frame.high.saturating_sub(1))
}

/// Where a point of the place is on the screen, in all its pixels.
fn at_pixel(view: &Viewport, at: Vec2) -> (i32, i32) {
    let at = view.on_screen(at);
    (at.x as i32, at.y as i32)
}

impl Play {
    /// Those of the stage, in the engine's drawing order (all of them at
    /// once: `drawn` draws each at its own rank).
    pub(super) fn draw_bodies(&self, world: &mut Screen, brush: &Brush) {
        let flat = |who: Who| self.sim.shown.get(&who).is_some_and(|shown| shown.flat);
        for (who, _) in self.sim.stage.in_drawing_order(flat) {
            self.draw_body(world, brush, who);
        }
    }

    /// Someone of the stage: its shadow, its picture as its node has it
    /// (turned, sized, slanted about its pivot, flipped, a colour added or
    /// filling it, mixed, smoothed; a spin that turns it), a glint if it
    /// shines, the place's upper layer over it if it stands behind it.
    pub(super) fn draw_body(&self, world: &mut Screen, brush: &Brush, who: Who) {
        let (view, light, seconds) = (&brush.large, brush.on_place, brush.seconds);
        let close = view.close;
        let clock = self.sim.clock as usize;
        {
            let (Some(actor), Some(shown)) = (self.sim.stage.cast.get(&who), self.sim.shown.get(&who)) else {
                return;
            };
            if actor.attributes & attribute::HIDDEN != 0 || (actor.attributes & attribute::BLINKING != 0 && clock % 16 >= 8) {
                return;
            }
            let way = way_of(actor.direction);
            let at = actor.shown_at();
            // (A thing in the air has a shadow, whatever it is; one in someone's
            // hands has none.)
            let thrown = (who.is_thing() && actor.shown_height() > 2.0).then_some(0);
            // (Sunk in the water: no shadow on it.)
            if let (Some(size), false) = (shown.shadow.or(thrown).filter(|_| !actor.shadowless && !actor.carried() && actor.sunk <= 0.0), shown.flat) {
                let lies = shown.picture(way).map_or((0, 0), |frame| frame.shadow);
                let (x, y) = at_pixel(view, at + Vec2::new(lies.0 as f32, lies.1 as f32 + 3.0));
                // (A smaller shape the higher it goes.)
                let size = (size as usize).saturating_sub((actor.shown_height().max(0.0) / 16.0) as usize);
                let alpha = (110.0 * actor.shown_opacity()) as u8;
                world.shadow((x, y), SHADOWS[size.min(2)], close.round().max(1.0) as i32, alpha);
            }
            let Some(frame) = shown.picture(way) else {
                return;
            };
            let alpha = actor.shown_opacity() * if actor.attributes & attribute::SEEN_THROUGH != 0 { 0.5 } else { 1.0 };
            let afloat = at - Vec2::new(0.0, actor.shown_height() + frame.lift() as f32);
            let raised = afloat + Vec2::new(0.0, actor.sunk);
            let size = close * actor.shown_scale();
            // (Sunk in the water: so much lower, and nothing of it under the
            // surface: where the lowest of its pictures, this way, this
            // animation, reached; the same line whatever it does, its flippers
            // going in and out of it.)
            let under = (actor.sunk > 0.0).then(|| {
                let deepest = shown.steps(way).iter().map(|step| lowest_row(step) as i32 + 1 - step.origin.1).max().unwrap_or(0);
                (at_pixel(view, at).1 + (deepest as f32 * size) as i32, if actor.in_sand { SAND } else { FOAM })
            });
            // (What its node keeps of its own: an angle, a slant, a size one
            // way, a pivot, a flip up and down, how it looks.)
            let kept = self.sim.stage.nodes.get(&NodeRef::Who(who));
            let (mut transform, plain_looks) = (kept.map_or(Transform::default(), |kept| kept.transform), Looks::default());
            let looks = kept.map_or(&plain_looks, |kept| &kept.looks);
            // (Hung on someone: its turn, size, opacity and tint too.)
            let hung = match kept.is_some_and(|kept| kept.parent.is_some()) {
                true => self.sim.stage.hung(&NodeRef::Who(who)),
                false => Default::default(),
            };
            transform.angle_deg += hung.angle_deg;
            transform.scale *= hung.scale;
            let colour = [0, 1, 2].map(|at| actor.colour[at] * hung.tint[at]);
            let alpha = alpha * looks.opacity * looks.tint[3] as f32 / 255.0 * hung.opacity * hung.tint[3];
            let turn = actor.shown_turn();
            let centre = at_pixel(view, raised);
            // (As the game draws a sprite, most often.)
            let around = if transform.is_upright() && turn == 0.0 && looks.is_plain() {
                let size = size * transform.scale.x;
                world.sprite_turned(frame, centre, size, shell::Tint { colour, alpha }, actor.flipped ^ transform.flip_x, under);
                let (half_wide, half_high) = ((frame.wide as f32 * size) as i32 / 2 + 1, (frame.high as f32 * size) as i32 / 2 + 1);
                (centre.0 - half_wide, centre.1 - half_high, centre.0 + half_wide, centre.1 + half_high)
            } else {
                // (Its picture where the game puts it; turned by a spin about
                // its middle; then as its node says, about its pivot from its
                // feet, in the air as high as it is.)
                let middle = Vec2::new(centre.0 as f32, centre.1 as f32);
                let feet = at_pixel(view, at - Vec2::new(0.0, actor.shown_height()));
                let feet = Vec2::new(feet.0 as f32, feet.1 as f32);
                let picture = Vec2::new(frame.wide as f32, frame.high as f32);
                let flip = if actor.flipped { -1.0 } else { 1.0 };
                let sprite = Affine2::from_translation(middle) * Affine2::from_scale(Vec2::new(size * flip, size)) * Affine2::from_translation(-picture / 2.0);
                let spun = Affine2::from_translation(middle) * Affine2::from_angle(turn.to_radians()) * Affine2::from_translation(-middle);
                let node = Affine2::from_translation(feet)
                    * Affine2::from_scale(Vec2::splat(close))
                    * transform.about_pivot()
                    * Affine2::from_scale(Vec2::splat(1.0 / close.max(f32::MIN_POSITIVE)))
                    * Affine2::from_translation(-feet);
                let map = node * spun * sprite;
                let (fx, around) = super::effects_of(&looks.effects, close, seconds);
                let paint = shell::Paint { below: under.map(|(row, _)| row), fx, ..super::paint_of(looks, colour, alpha) };
                world.blit_with(frame, map, &paint, &around);
                let (low, high) = shell::blit::bounds(&map, (picture.x, picture.y));
                (low.x as i32 - 1, low.y as i32 - 1, high.x as i32 + 1, high.y as i32 + 1)
            };
            if self.sim.stage.thing(who).is_some_and(|thing| thing.shines) {
                glint(world, centre, frame, close, clock);
            }
            // (Behind a roof, it is what is seen; but for one told a rank
            // over it, who peeks out of the foliage.)
            let over_upper = actor.attributes & attribute::OVER_MAP != 0 || self.sim.stage.over_place_upper(who);
            if let Some(ground) = self.ground().filter(|ground| !ground.upper.is_empty() && !over_upper) {
                let shown = (seconds / ground.frame_seconds.max(0.01)) as usize % ground.upper.len();
                world.cover(&ground.upper[shown], (ground.wide, ground.high), &ground.cover, view.centre, close, at.y, around, light);
            }
        }
    }

    /// The fishing lines: from the rod's tip to what they hold, sagging, of
    /// whole pixels of the game. (Drawn over the cast: nobody stands on them.)
    pub(super) fn draw_lines(&self, world: &mut Screen, view: &Viewport, art: &Art) {
        const COLOUR: [u8; 4] = [235, 245, 255, 255];
        const SAG: f32 = 0.16;
        // (Where it is tied to what it holds: a little over its feet.)
        const TIED: f32 = 8.0;
        let pixel = view.close.round().max(1.0) as i32;
        for (who, to) in &self.sim.stage.lines {
            let (Some(fisher), Some(held), Some(shown)) = (self.sim.stage.cast.get(who), self.sim.stage.cast.get(to), self.sim.shown.get(who)) else {
                continue;
            };
            let way = way_of(fisher.direction);
            // (Where the rod drawn in his picture ends, if it is drawn: a variant's `pointe`.)
            let tip = art.rod_tip(self.sim.cast.who(*who).0, shown.showing(), way, shown.step()).unwrap_or((0, -24));
            let from = fisher.shown_at() + Vec2::new(tip.0 as f32, tip.1 as f32 - fisher.shown_height());
            let end = held.shown_at() - Vec2::new(0.0, held.shown_height() + TIED);
            let steps = from.distance(end).ceil().max(1.0) as usize;
            for step in 0..=steps {
                let part = step as f32 / steps as f32;
                let low = 4.0 * part * (1.0 - part) * SAG * from.distance(end);
                // (Each on the grid of the game's pixels.)
                let on = view.on_screen(from.lerp(end, part) + Vec2::Y * low);
                let (x, y) = ((on.x as i32).div_euclid(pixel) * pixel, (on.y as i32).div_euclid(pixel) * pixel);
                world.fill(x, y, pixel, pixel, COLOUR);
            }
        }
    }

    /// What moves throw, a little off the ground.
    pub(super) fn draw_shots(&self, world: &mut Screen, brush: &Brush, art: &Art) {
        const FLYING: f32 = 12.0;
        let view = &brush.large;
        for shot in self.sim.stage.shots.iter().filter(|shot| shot.effect != 0) {
            let Some(to) = self.sim.stage.cast.get(&shot.target).map(|actor| actor.position) else {
                continue;
            };
            let (at, way, gone) = (shot.at(to) - Vec2::new(0.0, shot.rise()), to - shot.from, shot.gone as u32);
            let Some(thrown) = art.effect(shot.effect) else {
                continue;
            };
            // (Those with a picture for each way go the way they are thrown.)
            let ways = thrown.sprite.ways(0);
            let facing = towards(Vec2::ZERO, way).map_or(0, way_of);
            let steps = ways.get(if ways.len() == 8 { facing } else { 0 }).map_or(&[][..], Vec::as_slice);
            let whole: u32 = steps.iter().map(|frame| frame.ticks).sum();
            let mut left = gone % whole.max(1);
            let frame = steps.iter().find(|frame| {
                let here = left < frame.ticks;
                left = left.saturating_sub(frame.ticks);
                here
            });
            if let Some(frame) = frame {
                world.sprite(frame, at_pixel(view, at - Vec2::new(0.0, FLYING)), view.close, [255.0; 3], 1.0);
            }
        }
    }

    /// The particles that glow, in all the pixels there are, over the light,
    /// with a tail behind them and a glow about them.
    pub(super) fn draw_glows(&self, world: &mut Screen, brush: &Brush) {
        let (scale, close) = (world.scale as f32, brush.view.close);
        for particle in self.sim.stage.light.particles.iter().filter(|particle| !particle.sharp) {
            let mut alpha = (particle.left / particle.life).clamp(0.0, 1.0);
            if particle.blinks {
                alpha *= 0.5 + 0.5 * (particle.left * 0.15).sin();
            }
            let radius = 3.0 * scale * close.max(0.5);
            for (behind, part) in [(9.0, 0.2), (6.0, 0.35), (3.0, 0.55)] {
                world.glow(brush.in_world(particle.at - particle.speed * behind), radius * 0.8, particle.colour, alpha * part);
            }
            world.glow(brush.in_world(particle.at), radius * 2.0, particle.colour, alpha * 0.45);
            world.glow(brush.in_world(particle.at), radius * 0.8, [255, 255, 255], alpha);
        }
    }
}

/// The smallest box around what is drawn of a picture (its pixels that show),
/// in its own pixels: left, top, right, bottom (the last two past the end).
fn drawn_part(frame: &shell::images::Frame) -> Option<[usize; 4]> {
    let pixels = frame.rgba.as_chunks::<4>().0;
    let mut part: Option<[usize; 4]> = None;
    for row in 0..frame.high {
        for column in 0..frame.wide {
            if pixels.get(row * frame.wide + column).is_some_and(|pixel| pixel[3] > 0) {
                let [left, top, right, bottom] = part.get_or_insert([column, row, column + 1, row + 1]);
                (*left, *top, *right, *bottom) = ((*left).min(column), (*top).min(row), (*right).max(column + 1), (*bottom).max(row + 1));
            }
        }
    }
    part
}

impl Play {
    /// Where someone of the stage is drawn on a screen seen through `view`,
    /// to the pixel: the box around the pixels of his picture that show, as
    /// `draw_body` lays them (its picture this way, its size, flipped, in the
    /// air, sunk in the water, its node's turn and size). Left, top, right,
    /// bottom; none if nothing of him is drawn.
    pub fn drawn_box(&self, who: Who, view: &Viewport) -> Option<[f32; 4]> {
        let (actor, shown) = (self.sim.stage.cast.get(&who)?, self.sim.shown.get(&who)?);
        if actor.attributes & attribute::HIDDEN != 0 {
            return None;
        }
        let way = way_of(actor.direction);
        let frame = shown.picture(way)?;
        let [left, top, right, bottom] = drawn_part(frame)?;
        let close = view.close;
        let at = actor.shown_at();
        let raised = at - Vec2::new(0.0, actor.shown_height() + frame.lift() as f32) + Vec2::new(0.0, actor.sunk);
        let size = close * actor.shown_scale();
        // (Sunk: nothing drawn under the line it goes into the water at.)
        let under = (actor.sunk > 0.0).then(|| {
            let deepest = shown.steps(way).iter().map(|step| lowest_row(step) as i32 + 1 - step.origin.1).max().unwrap_or(0);
            (at_pixel(view, at).1 + (deepest as f32 * size) as i32) as f32
        });
        let kept = self.sim.stage.nodes.get(&NodeRef::Who(who));
        let (mut transform, plain_looks) = (kept.map_or(Transform::default(), |kept| kept.transform), Looks::default());
        let looks = kept.map_or(&plain_looks, |kept| &kept.looks);
        let hung = match kept.is_some_and(|kept| kept.parent.is_some()) {
            true => self.sim.stage.hung(&NodeRef::Who(who)),
            false => Default::default(),
        };
        transform.angle_deg += hung.angle_deg;
        transform.scale *= hung.scale;
        let turn = actor.shown_turn();
        let centre = at_pixel(view, raised);
        let [low_x, low_y, high_x, high_y] = if transform.is_upright() && turn == 0.0 && looks.is_plain() {
            // (As `Screen::sprite_turned` lays it: a column of the screen shows
            // the picture's column it falls in, from the left of the whole
            // picture, centred; mirrored if it is flipped.)
            let size = (size * transform.scale.x).max(0.01);
            let (wide, high) = ((frame.wide as f32 * size) as i32, (frame.high as f32 * size) as i32);
            let (picture_left, picture_top) = ((centre.0 - wide / 2) as f32, (centre.1 - high / 2) as f32);
            let (left, right) = match actor.flipped ^ transform.flip_x {
                true => (frame.wide - right, frame.wide - left),
                false => (left, right),
            };
            let edge = |pixel: usize| (pixel as f32 * size).ceil();
            [picture_left + edge(left), picture_top + edge(top), picture_left + edge(right).min(wide as f32), picture_top + edge(bottom).min(high as f32)]
        } else {
            // (Through the same map as `draw_body`'s.)
            let middle = Vec2::new(centre.0 as f32, centre.1 as f32);
            let feet = at_pixel(view, at - Vec2::new(0.0, actor.shown_height()));
            let feet = Vec2::new(feet.0 as f32, feet.1 as f32);
            let picture = Vec2::new(frame.wide as f32, frame.high as f32);
            let flip = if actor.flipped { -1.0 } else { 1.0 };
            let sprite = Affine2::from_translation(middle) * Affine2::from_scale(Vec2::new(size * flip, size)) * Affine2::from_translation(-picture / 2.0);
            let spun = Affine2::from_translation(middle) * Affine2::from_angle(turn.to_radians()) * Affine2::from_translation(-middle);
            let node = Affine2::from_translation(feet)
                * Affine2::from_scale(Vec2::splat(close))
                * transform.about_pivot()
                * Affine2::from_scale(Vec2::splat(1.0 / close.max(f32::MIN_POSITIVE)))
                * Affine2::from_translation(-feet);
            let part = Affine2::from_translation(Vec2::new(left as f32, top as f32));
            let (low, high) = shell::blit::bounds(&(node * spun * sprite * part), ((right - left) as f32, (bottom - top) as f32));
            [low.x, low.y, high.x, high.y]
        };
        let high_y = under.map_or(high_y, |under| high_y.min(under));
        (high_y > low_y && high_x > low_x).then_some([low_x, low_y, high_x, high_y])
    }
}

#[cfg(test)]
mod tests {
    use super::drawn_part;

    #[test]
    fn the_part_drawn_is_only_the_pixels_that_show() {
        let (wide, high) = (6, 5);
        let mut rgba = vec![0; wide * high * 4];
        for (column, row) in [(2, 1), (4, 3), (3, 2)] {
            rgba[(row * wide + column) * 4 + 3] = 255;
        }
        let frame = shell::images::Frame { wide, high, rgba, ticks: 1, origin: (3, 2), shadow: (0, 0) };
        assert_eq!(drawn_part(&frame), Some([2, 1, 5, 4]));
        let empty = shell::images::Frame { rgba: vec![0; wide * high * 4], ..frame };
        assert_eq!(drawn_part(&empty), None);
    }
}
