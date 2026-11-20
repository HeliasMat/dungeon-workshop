//! A picture laid on another through any affine map ([`blit`]): turned,
//! sized each way, slanted, flipped, about any point. Row by row: for each
//! row of what is drawn on, the span the picture covers is worked out
//! first, then walked one step at a time (principle 9: the renderer is
//! software). Nearest by default, bilinear if asked (`Paint::smooth`);
//! multiplied by a colour, a colour added, a colour filling its shape, and
//! mixed with what is under it as a [`Mix`] says; each pixel as an [`Fx`]
//! says (dissolved, swayed, a colour swapped, flashed). [`blit_with`] lays
//! about it first what an [`Around`] asks (a drop shadow, a glow, an
//! outline), worked out on the region it covers only.

use glam::{Affine2, Vec2};

/// How a picture mixes with what is under it: what each part of a colour
/// becomes, then laid by its alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mix {
    #[default]
    Normal,
    Add,
    Multiply,
    Screen,
    Overlay,
    Lighten,
    Darken,
    Difference,
}

impl Mix {
    /// A part of the colour over (`over`) mixed with the part under it,
    /// both of 255.
    #[inline(always)]
    pub fn of(self, over: f32, under: f32) -> f32 {
        match self {
            Mix::Normal => over,
            Mix::Add => (over + under).min(255.0),
            Mix::Multiply => over * under / 255.0,
            Mix::Screen => 255.0 - (255.0 - over) * (255.0 - under) / 255.0,
            Mix::Overlay if under < 128.0 => 2.0 * over * under / 255.0,
            Mix::Overlay => 255.0 - 2.0 * (255.0 - over) * (255.0 - under) / 255.0,
            Mix::Lighten => over.max(under),
            Mix::Darken => over.min(under),
            Mix::Difference => (over - under).abs(),
        }
    }
}

/// How a picture is laid: its colours multiplied by `tint` (of 255 each:
/// 255 as it is), `add` added, all of it `fill`ed with a colour by that
/// colour's alpha (its shape kept), `alpha` of it shown, mixed as `mix`
/// says, smoothed or not; nothing of it drawn from the row `below` down;
/// each of its pixels as `fx` says.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub tint: [f32; 3],
    pub alpha: f32,
    pub add: [u8; 3],
    pub fill: Option<[u8; 4]>,
    pub mix: Mix,
    pub smooth: bool,
    pub below: Option<i32>,
    pub fx: Fx,
}

impl Default for Paint {
    /// As it is.
    fn default() -> Self {
        Paint { tint: [255.0; 3], alpha: 1.0, add: [0; 3], fill: None, mix: Mix::Normal, smooth: false, below: None, fx: Fx::default() }
    }
}

/// What is done to each pixel of a picture as it is laid: so much of it
/// gone (`dissolve`, 0 to 1, each pixel by a draw of `seed`, the same
/// pixels as it grows), its rows swayed sideways (`wave` of its own
/// pixels, a sway every `wave_length` rows, `wave_phase` radians into it),
/// one colour of it swapped for another (`swap`), all of it gone towards a
/// colour (`flash`: the colour, how much, 0 to 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fx {
    pub dissolve: f32,
    pub seed: u32,
    pub wave: f32,
    pub wave_length: f32,
    pub wave_phase: f32,
    pub swap: Option<([u8; 3], [u8; 3])>,
    pub flash: Option<([u8; 3], f32)>,
}

impl Default for Fx {
    /// Nothing done.
    fn default() -> Self {
        Fx { dissolve: 0.0, seed: 0, wave: 0.0, wave_length: 16.0, wave_phase: 0.0, swap: None, flash: None }
    }
}

impl Fx {
    /// Nothing done to any pixel.
    pub fn is_none(&self) -> bool {
        self.dissolve <= 0.0 && self.wave == 0.0 && self.swap.is_none() && self.flash.is_none()
    }

    /// How far a row of the picture (at `y`, of its pixels) sways.
    #[inline(always)]
    fn sway(&self, y: f32) -> f32 {
        self.wave * (std::f32::consts::TAU * y / self.wave_length.max(f32::MIN_POSITIVE) + self.wave_phase).sin()
    }

    /// Whether the dissolve has taken that pixel of the picture: a draw of
    /// the seed and the pixel, under how much is gone.
    #[inline(always)]
    fn gone(&self, x: i32, y: i32) -> bool {
        if self.dissolve <= 0.0 {
            return false;
        }
        let mut hash = self.seed ^ (x as u32).wrapping_mul(0x9e37_79b1) ^ (y as u32).wrapping_mul(0x85eb_ca77);
        hash ^= hash >> 15;
        hash = hash.wrapping_mul(0x2c1b_3c6d);
        hash ^= hash >> 12;
        hash = hash.wrapping_mul(0x297a_2d39);
        hash ^= hash >> 15;
        (hash as f32 / u32::MAX as f32) < self.dissolve
    }
}

/// What is laid about a picture, under it: a drop shadow (its colour, so
/// many of the dest's pixels off), a glow (its colour, so wide: three hard
/// bands, fainter outwards), an outline (its colour, so wide), each of the
/// picture's shape.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Around {
    pub shadow: Option<([u8; 4], Vec2)>,
    pub glow: Option<([u8; 4], f32)>,
    pub outline: Option<([u8; 4], f32)>,
}

impl Around {
    pub fn is_none(&self) -> bool {
        self.shadow.is_none() && self.glow.is_none() && self.outline.is_none()
    }
}

/// A colour laid on a pixel, `shows` of it (0 to 1), mixed: where the dest
/// is clear, as it is with that alpha.
#[inline(always)]
fn lay(pixel: &mut [u8; 4], [red, green, blue]: [f32; 3], shows: f32, mix: Mix) {
    let shows = shows.min(1.0);
    let over_alpha = (shows * 255.0).round() as u8;
    if pixel[3] == 0 {
        let [red, green, blue] = [red, green, blue].map(|part| part.round().clamp(0.0, 255.0) as u8);
        *pixel = [red, green, blue, over_alpha];
        return;
    }
    pixel[3] = pixel[3].max(over_alpha);
    for (part, over) in pixel.iter_mut().zip([red.min(255.0), green.min(255.0), blue.min(255.0)]) {
        let under = *part as f32;
        *part = (under + (mix.of(over, under) - under) * shows).round().clamp(0.0, 255.0) as u8;
    }
}

/// The largest of each pixel's neighbours so far about (a square), a row's
/// way then a column's: a shape grown by that many pixels.
fn grown(mask: &[u8], (wide, high): (usize, usize), by: usize) -> Vec<u8> {
    if by == 0 {
        return mask.to_vec();
    }
    let mut across = vec![0u8; mask.len()];
    for y in 0..high {
        let row = &mask[y * wide..(y + 1) * wide];
        for x in 0..wide {
            let (from, to) = (x.saturating_sub(by), (x + by + 1).min(wide));
            across[y * wide + x] = row[from..to].iter().copied().max().unwrap_or(0);
        }
    }
    let mut out = vec![0u8; mask.len()];
    for x in 0..wide {
        for y in 0..high {
            let (from, to) = (y.saturating_sub(by), (y + by + 1).min(high));
            out[y * wide + x] = (from..to).map(|at| across[at * wide + x]).max().unwrap_or(0);
        }
    }
    out
}

/// Lays a picture as [`blit`] does, with what `around` asks laid under it
/// first: its shape drawn once on the region it covers (grown by the widest
/// of them), then grown, moved and laid in their colours.
#[allow(clippy::too_many_arguments)]
pub fn blit_with(dest: &mut [u8], (dest_wide, dest_high): (usize, usize), source: &[u8], size: (usize, usize), map: Affine2, paint: &Paint, around: &Around) {
    if !around.is_none() && dest.len() >= dest_wide * dest_high * 4 {
        let reach = around.glow.map_or(0.0, |(_, wide)| wide).max(around.outline.map_or(0.0, |(_, wide)| wide)).ceil();
        let off = around.shadow.map_or(Vec2::ZERO, |(_, by)| by);
        let (low, high) = widened_bounds(&map, size, paint.fx.wave);
        let margin = reach + 1.0;
        let low = (low - Vec2::splat(margin) + off.min(Vec2::ZERO)).floor().max(Vec2::ZERO);
        let high = (high + Vec2::splat(margin) + off.max(Vec2::ZERO)).ceil().min(Vec2::new(dest_wide as f32, dest_high as f32));
        if high.x > low.x && high.y > low.y {
            let (left, top) = (low.x as usize, low.y as usize);
            let (wide, tall) = ((high.x - low.x) as usize, (high.y - low.y) as usize);
            // (Its shape alone, on the region.)
            let mut shape = vec![0u8; wide * tall * 4];
            let alone = Paint { tint: [255.0; 3], alpha: 1.0, add: [0; 3], fill: None, mix: Mix::Normal, below: None, ..*paint };
            blit(&mut shape, (wide, tall), source, size, Affine2::from_translation(-low) * map, &alone);
            let mask: Vec<u8> = shape.as_chunks::<4>().0.iter().map(|pixel| pixel[3]).collect();
            let floor = paint.below.map_or(dest_high as i32, |below| below.min(dest_high as i32));
            let dest = dest.as_chunks_mut::<4>().0;
            let mut laid = |mask: &[u8], (dx, dy): (i32, i32), [red, green, blue, alpha]: [u8; 4], much: f32| {
                let colour = [red as f32, green as f32, blue as f32];
                for y in 0..tall as i32 {
                    let (from, to) = (y - dy, top as i32 + y);
                    if from < 0 || from >= tall as i32 || to >= floor {
                        continue;
                    }
                    for x in 0..wide as i32 {
                        let from_x = x - dx;
                        if from_x < 0 || from_x >= wide as i32 {
                            continue;
                        }
                        let shown = mask[from as usize * wide + from_x as usize];
                        if shown == 0 {
                            continue;
                        }
                        let shows = shown as f32 / 255.0 * alpha as f32 / 255.0 * much * paint.alpha;
                        lay(&mut dest[to as usize * dest_wide + left + x as usize], colour, shows, Mix::Normal);
                    }
                }
            };
            if let Some((colour, by)) = around.shadow {
                laid(&mask, (by.x.round() as i32, by.y.round() as i32), colour, 1.0);
            }
            // (A glow in three hard bands, the widest the faintest.)
            if let Some((colour, far)) = around.glow {
                for (share, much) in [(1.0, 0.25), (2.0 / 3.0, 0.35), (1.0 / 3.0, 0.5)] {
                    let band = grown(&mask, (wide, tall), (far * share).round().max(1.0) as usize);
                    laid(&band, (0, 0), colour, much);
                }
            }
            if let Some((colour, by)) = around.outline {
                let ring = grown(&mask, (wide, tall), by.round().max(1.0) as usize);
                laid(&ring, (0, 0), colour, 1.0);
            }
        }
    }
    blit(dest, (dest_wide, dest_high), source, size, map, paint);
}

/// The box that holds a picture through a map, its rows swayed so far
/// either way (of its own pixels) by a wave.
fn widened_bounds(map: &Affine2, (wide, high): (usize, usize), wave: f32) -> (Vec2, Vec2) {
    let sway = wave.abs();
    let (low, high) = bounds(&(*map * Affine2::from_translation(Vec2::new(-sway, 0.0))), (wide as f32 + 2.0 * sway, high as f32));
    (low, high)
}

/// Where a picture so wide and high goes through a map: its low and high
/// corners (the box that holds it).
pub fn bounds(map: &Affine2, (wide, high): (f32, f32)) -> (Vec2, Vec2) {
    let corners = [Vec2::ZERO, Vec2::new(wide, 0.0), Vec2::new(0.0, high), Vec2::new(wide, high)].map(|corner| map.transform_point2(corner));
    let low = corners.iter().fold(Vec2::splat(f32::INFINITY), |low, corner| low.min(*corner));
    let high = corners.iter().fold(Vec2::splat(f32::NEG_INFINITY), |high, corner| high.max(*corner));
    (low, high)
}

/// The pixel of a picture at a column and a row, nothing outside it.
#[inline(always)]
fn texel(source: &[[u8; 4]], wide: usize, high: usize, x: i32, y: i32) -> [u8; 4] {
    if x < 0 || y < 0 || x as usize >= wide || y as usize >= high { [0; 4] } else { source[y as usize * wide + x as usize] }
}

/// The colour at a point of the picture, between its four nearest pixels,
/// each by its alpha.
#[inline(always)]
fn between(source: &[[u8; 4]], wide: usize, high: usize, at: Vec2) -> [f32; 4] {
    let (x, y) = (at.x - 0.5, at.y - 0.5);
    let (left, top) = (x.floor(), y.floor());
    let (across, down) = (x - left, y - top);
    let (left, top) = (left as i32, top as i32);
    let mut sum = [0.0f32; 4];
    for (dx, dy, share) in [(0, 0, (1.0 - across) * (1.0 - down)), (1, 0, across * (1.0 - down)), (0, 1, (1.0 - across) * down), (1, 1, across * down)] {
        let [red, green, blue, alpha] = texel(source, wide, high, left + dx, top + dy);
        let weight = share * alpha as f32;
        sum[0] += red as f32 * weight;
        sum[1] += green as f32 * weight;
        sum[2] += blue as f32 * weight;
        sum[3] += weight;
    }
    if sum[3] <= 0.0 {
        return [0.0; 4];
    }
    [sum[0] / sum[3], sum[1] / sum[3], sum[2] / sum[3], sum[3]]
}

/// Lays `source` (so wide and high, RGBA) on `dest` (so wide and high,
/// RGBA) through `map`, which takes a point of the source's pixels to one
/// of the dest's. Cut to the dest's edges; a map that squashes it flat
/// draws nothing. Where the dest is wholly clear (a canvas laid over
/// later), the colour goes there as it is, with what shows of it for its
/// alpha; elsewhere it is mixed, and the dest's alpha is the higher of
/// the two (a screen stays whole).
pub fn blit(dest: &mut [u8], (dest_wide, dest_high): (usize, usize), source: &[u8], (wide, high): (usize, usize), map: Affine2, paint: &Paint) {
    if wide == 0 || high == 0 || paint.alpha <= 0.0 || source.len() < wide * high * 4 || dest.len() < dest_wide * dest_high * 4 {
        return;
    }
    let determinant = map.matrix2.determinant();
    if !determinant.is_finite() || determinant.abs() < 1e-9 {
        return;
    }
    let back = map.inverse();
    let fx = paint.fx;
    let (low, top_right) = widened_bounds(&map, (wide, high), fx.wave);
    let floor = paint.below.map_or(dest_high as i32, |below| below.min(dest_high as i32));
    let (left, right) = ((low.x.floor() as i32).max(0), (top_right.x.ceil() as i32).min(dest_wide as i32));
    let (top, bottom) = ((low.y.floor() as i32).max(0), (top_right.y.ceil() as i32).min(floor));
    if right <= left || bottom <= top {
        return;
    }
    let source = &source.as_chunks::<4>().0[..wide * high];
    let dest = dest.as_chunks_mut::<4>().0;
    // (A step right on the dest, a step down: so much on the source.)
    let (along, _) = (back.matrix2.x_axis, back.matrix2.y_axis);
    // (Smoothed, a half pixel more about it shows.)
    let margin = if paint.smooth { 0.5 } else { 0.0 };
    let limits = [(-margin, wide as f32 + margin), (-margin, high as f32 + margin)];
    // (Swayed, a row's span is sought that much wider, each point then tried.)
    let sway = fx.wave.abs();
    let sought = [(limits[0].0 - sway, limits[0].1 + sway), limits[1]];
    let fill = paint.fill.map(|[red, green, blue, alpha]| ([red as f32, green as f32, blue as f32], alpha as f32 / 255.0));
    let flash = fx.flash.map(|([red, green, blue], much)| ([red as f32, green as f32, blue as f32], much.clamp(0.0, 1.0)));
    let plain_colour = paint.tint == [255.0; 3] && paint.add == [0; 3] && fill.is_none() && flash.is_none();
    let plain_fx = fx.is_none();
    for y in top..bottom {
        let start = back.transform_point2(Vec2::new(left as f32 + 0.5, y as f32 + 0.5));
        // (The steps of this row whose point falls in the picture.)
        let (mut first, mut last) = (0.0f32, (right - left - 1) as f32);
        let mut none = false;
        for (from, step, (least, most)) in [(start.x, along.x, sought[0]), (start.y, along.y, sought[1])] {
            if step.abs() < 1e-12 {
                none |= from < least || from >= most;
            } else {
                let (a, b) = ((least - from) / step, (most - from) / step);
                first = first.max(a.min(b).floor());
                last = last.min(a.max(b).ceil());
            }
        }
        if none || last < first {
            continue;
        }
        let row = y as usize * dest_wide;
        for step in first as i32..=last as i32 {
            let mut at = start + along * step as f32;
            if !plain_fx && sway > 0.0 {
                at.x += fx.sway(at.y);
            }
            if at.x < limits[0].0 || at.x >= limits[0].1 || at.y < limits[1].0 || at.y >= limits[1].1 {
                continue;
            }
            if !plain_fx && fx.gone(at.x.floor() as i32, at.y.floor() as i32) {
                continue;
            }
            let [mut red, mut green, mut blue, alpha] = match paint.smooth {
                true => between(source, wide, high, at),
                false => texel(source, wide, high, at.x as i32, at.y as i32).map(f32::from),
            };
            if let Some((from, to)) = fx.swap
                && [red, green, blue] == from.map(f32::from)
            {
                [red, green, blue] = to.map(f32::from);
            }
            let shows = alpha / 255.0 * paint.alpha;
            if shows <= 0.0 {
                continue;
            }
            if !plain_colour {
                red = red * paint.tint[0] / 255.0 + paint.add[0] as f32;
                green = green * paint.tint[1] / 255.0 + paint.add[1] as f32;
                blue = blue * paint.tint[2] / 255.0 + paint.add[2] as f32;
                for (colour, much) in [fill, flash].into_iter().flatten() {
                    red += (colour[0] - red) * much;
                    green += (colour[1] - green) * much;
                    blue += (colour[2] - blue) * much;
                }
            }
            lay(&mut dest[row + (left + step) as usize], [red, green, blue], shows, paint.mix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Mat2;

    /// A picture of 2x2: red, green / blue, white, all whole.
    fn four() -> Vec<u8> {
        [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255]].concat()
    }

    fn pixel(dest: &[u8], wide: usize, x: usize, y: usize) -> [u8; 4] {
        let at = (y * wide + x) * 4;
        [dest[at], dest[at + 1], dest[at + 2], dest[at + 3]]
    }

    fn black(wide: usize, high: usize) -> Vec<u8> {
        [0, 0, 0, 255].repeat(wide * high)
    }

    #[test]
    fn as_it_is_it_is_copied_where_it_goes() {
        let mut dest = black(4, 4);
        blit(&mut dest, (4, 4), &four(), (2, 2), Affine2::from_translation(Vec2::new(1.0, 1.0)), &Paint::default());
        assert_eq!(pixel(&dest, 4, 1, 1), [255, 0, 0, 255]);
        assert_eq!(pixel(&dest, 4, 2, 1), [0, 255, 0, 255]);
        assert_eq!(pixel(&dest, 4, 1, 2), [0, 0, 255, 255]);
        assert_eq!(pixel(&dest, 4, 2, 2), [255, 255, 255, 255]);
        assert_eq!(pixel(&dest, 4, 0, 0), [0, 0, 0, 255]);
        assert_eq!(pixel(&dest, 4, 3, 3), [0, 0, 0, 255]);
    }

    #[test]
    fn twice_as_big_each_pixel_is_a_square_of_four() {
        let mut dest = black(4, 4);
        blit(&mut dest, (4, 4), &four(), (2, 2), Affine2::from_scale(Vec2::splat(2.0)), &Paint::default());
        for (x, y, colour) in [(0, 0, [255, 0, 0]), (1, 1, [255, 0, 0]), (3, 0, [0, 255, 0]), (0, 3, [0, 0, 255]), (3, 3, [255, 255, 255])] {
            assert_eq!(pixel(&dest, 4, x, y)[..3], colour, "{x} {y}");
        }
    }

    #[test]
    fn a_quarter_turn_turns_its_corners() {
        // (Clockwise on the screen, about the picture's middle, put at 1 1.)
        let mut dest = black(2, 2);
        let turn = Affine2::from_translation(Vec2::ONE) * Affine2::from_angle(90f32.to_radians()) * Affine2::from_translation(-Vec2::ONE);
        blit(&mut dest, (2, 2), &four(), (2, 2), turn, &Paint::default());
        // (Red, at the top left, goes to the top right.)
        assert_eq!(pixel(&dest, 2, 1, 0)[..3], [255, 0, 0]);
        assert_eq!(pixel(&dest, 2, 1, 1)[..3], [0, 255, 0]);
        assert_eq!(pixel(&dest, 2, 0, 0)[..3], [0, 0, 255]);
        assert_eq!(pixel(&dest, 2, 0, 1)[..3], [255, 255, 255]);
    }

    #[test]
    fn a_map_and_its_inverse_come_back() {
        let map = Affine2::from_mat2_translation(Mat2::from_cols(Vec2::new(1.5, 0.4), Vec2::new(-0.7, 2.0)), Vec2::new(30.0, -12.0));
        for at in [Vec2::ZERO, Vec2::new(3.0, 9.0), Vec2::new(-40.0, 17.5)] {
            assert!(map.inverse().transform_point2(map.transform_point2(at)).distance(at) < 1e-4);
        }
        let (low, high) = bounds(&Affine2::from_angle(45f32.to_radians()), (2.0, 2.0));
        assert!((high.x - low.x - 8f32.sqrt()).abs() < 1e-4 && (high.y - low.y - 8f32.sqrt()).abs() < 1e-4, "{low} {high}");
    }

    #[test]
    fn past_the_edges_or_flat_nothing_breaks() {
        let mut dest = black(4, 4);
        let kept = dest.clone();
        for map in [
            Affine2::from_translation(Vec2::new(-10.0, 2.0)),
            Affine2::from_translation(Vec2::new(3.5, 3.5)) * Affine2::from_angle(0.7) * Affine2::from_scale(Vec2::splat(9.0)),
            Affine2::from_translation(Vec2::new(1e9, -1e9)),
            Affine2::from_scale(Vec2::new(0.0, 1.0)),
            Affine2::from_scale(Vec2::splat(f32::NAN)),
        ] {
            blit(&mut dest, (4, 4), &four(), (2, 2), map, &Paint { smooth: true, ..Paint::default() });
            blit(&mut dest, (4, 4), &four(), (2, 2), map, &Paint::default());
        }
        // (Half off the left: only its right column shows, at the edge.)
        let mut half = kept.clone();
        blit(&mut half, (4, 4), &four(), (2, 2), Affine2::from_translation(Vec2::new(-1.0, 0.0)), &Paint::default());
        assert_eq!(pixel(&half, 4, 0, 0)[..3], [0, 255, 0]);
        assert_eq!(pixel(&half, 4, 1, 0)[..3], [0, 0, 0]);
        // (Nothing under the row it stops at.)
        let mut cut = kept;
        blit(&mut cut, (4, 4), &four(), (2, 2), Affine2::IDENTITY, &Paint { below: Some(1), ..Paint::default() });
        assert_eq!((pixel(&cut, 4, 0, 0)[..3].to_vec(), pixel(&cut, 4, 0, 1)[..3].to_vec()), (vec![255, 0, 0], vec![0, 0, 0]));
    }

    #[test]
    fn its_colours_are_tinted_added_filled_and_mixed() {
        let grey = [[100, 100, 100, 255]].concat();
        let laid = |paint: Paint, under: u8| {
            let mut dest = vec![under, under, under, 255];
            blit(&mut dest, (1, 1), &grey, (1, 1), Affine2::IDENTITY, &paint);
            dest[0]
        };
        assert_eq!(laid(Paint::default(), 0), 100);
        assert_eq!(laid(Paint { tint: [127.5; 3], ..Paint::default() }, 0), 50);
        assert_eq!(laid(Paint { add: [60; 3], ..Paint::default() }, 0), 160);
        assert_eq!(laid(Paint { fill: Some([255, 255, 255, 255]), ..Paint::default() }, 0), 255);
        assert_eq!(laid(Paint { alpha: 0.5, ..Paint::default() }, 200), 150);
        assert_eq!(laid(Paint { mix: Mix::Add, ..Paint::default() }, 200), 255);
        assert_eq!(laid(Paint { mix: Mix::Multiply, ..Paint::default() }, 255), 100);
        assert_eq!(laid(Paint { mix: Mix::Screen, ..Paint::default() }, 0), 100);
        assert_eq!(laid(Paint { mix: Mix::Darken, ..Paint::default() }, 50), 50);
        assert_eq!(laid(Paint { mix: Mix::Difference, ..Paint::default() }, 30), 70);
    }

    #[test]
    fn on_a_clear_canvas_the_colour_goes_with_what_shows_of_it() {
        let mut dest = vec![0u8; 4];
        blit(&mut dest, (1, 1), &[200, 100, 50, 255], (1, 1), Affine2::IDENTITY, &Paint { alpha: 0.5, mix: Mix::Multiply, ..Paint::default() });
        assert_eq!(dest, [200, 100, 50, 128]);
    }

    /// A picture of 3x3, whole and red in its middle only.
    fn dot() -> Vec<u8> {
        let mut picture = vec![0u8; 9 * 4];
        picture[16..20].copy_from_slice(&[255, 0, 0, 255]);
        picture
    }

    #[test]
    fn an_outline_a_shadow_and_a_glow_are_laid_about_its_shape() {
        let at = Affine2::from_translation(Vec2::new(3.0, 3.0));
        let outline = Around { outline: Some(([0, 0, 0, 255], 1.0)), ..Around::default() };
        let mut dest = [255, 255, 255, 255].repeat(9 * 9);
        blit_with(&mut dest, (9, 9), &dot(), (3, 3), at, &Paint::default(), &outline);
        // (The dot at 4 4, its ring of eight about it, nothing further.)
        assert_eq!(pixel(&dest, 9, 4, 4), [255, 0, 0, 255]);
        for (x, y) in [(3, 3), (4, 3), (5, 5), (3, 5)] {
            assert_eq!(pixel(&dest, 9, x, y), [0, 0, 0, 255], "{x} {y}");
        }
        assert_eq!(pixel(&dest, 9, 2, 4), [255, 255, 255, 255]);
        let shadow = Around { shadow: Some(([0, 0, 0, 255], Vec2::new(2.0, 1.0))), ..Around::default() };
        let mut dest = [255, 255, 255, 255].repeat(9 * 9);
        blit_with(&mut dest, (9, 9), &dot(), (3, 3), at, &Paint::default(), &shadow);
        assert_eq!((pixel(&dest, 9, 6, 5), pixel(&dest, 9, 4, 4), pixel(&dest, 9, 5, 5)), ([0, 0, 0, 255], [255, 0, 0, 255], [255; 4]));
        // (A glow: hard bands, fainter outwards, none past it.)
        let glow = Around { glow: Some(([0, 0, 0, 255], 3.0)), ..Around::default() };
        let mut dest = [255, 255, 255, 255].repeat(9 * 9);
        blit_with(&mut dest, (9, 9), &dot(), (3, 3), at, &Paint::default(), &glow);
        let (near, far) = (pixel(&dest, 9, 5, 4)[0], pixel(&dest, 9, 7, 4)[0]);
        assert!(near < far && far < 255, "{near} {far}");
        assert_eq!(pixel(&dest, 9, 8, 4)[0], 255);
        // (Past the edges of the dest, nothing breaks.)
        let mut small = [255, 255, 255, 255].repeat(2 * 2);
        blit_with(&mut small, (2, 2), &dot(), (3, 3), Affine2::from_translation(Vec2::new(-1.0, -1.0)), &Paint::default(), &glow);
        assert_eq!(pixel(&small, 2, 0, 0), [255, 0, 0, 255]);
    }

    #[test]
    fn each_pixel_dissolves_sways_swaps_and_flashes() {
        let red = [255, 0, 0, 255].repeat(16 * 16);
        let laid = |fx: Fx| {
            let mut dest = [0, 0, 0, 255].repeat(16 * 16);
            blit(&mut dest, (16, 16), &red, (16, 16), Affine2::IDENTITY, &Paint { fx, ..Paint::default() });
            dest
        };
        let count = |dest: &[u8]| dest.chunks(4).filter(|pixel| pixel[0] == 255).count();
        // (Half dissolved: about half; the same seed, the same pixels; all of it: none.)
        let half = laid(Fx { dissolve: 0.5, seed: 42, ..Fx::default() });
        assert!((90..=166).contains(&count(&half)), "{}", count(&half));
        assert_eq!(half, laid(Fx { dissolve: 0.5, seed: 42, ..Fx::default() }));
        assert_ne!(half, laid(Fx { dissolve: 0.5, seed: 7, ..Fx::default() }));
        assert_eq!(count(&laid(Fx { dissolve: 1.0, ..Fx::default() })), 0);
        // (Swayed by 3: its rows move, so many pixels each side at most.)
        let swayed = laid(Fx { wave: 3.0, wave_length: 8.0, ..Fx::default() });
        assert_ne!(swayed, laid(Fx::default()));
        let swap = laid(Fx { swap: Some(([255, 0, 0], [0, 0, 255])), ..Fx::default() });
        assert_eq!(pixel(&swap, 16, 5, 5), [0, 0, 255, 255]);
        let flash = laid(Fx { flash: Some(([255, 255, 255], 0.5)), ..Fx::default() });
        assert_eq!(pixel(&flash, 16, 5, 5), [255, 128, 128, 255]);
    }

    #[test]
    fn smoothed_a_point_between_two_pixels_is_between_their_colours() {
        let two = [[0, 0, 0, 255], [200, 200, 200, 255]].concat();
        // (Four times as wide: the middle columns fall between the two.)
        let mut dest = vec![0u8; 8 * 4];
        blit(&mut dest, (8, 1), &two, (2, 1), Affine2::from_scale(Vec2::new(4.0, 1.0)), &Paint { smooth: true, ..Paint::default() });
        // (The middle columns: a quarter, then three, five, seven eighths of the way.)
        let row: Vec<u8> = (2..6).map(|x| dest[x * 4]).collect();
        assert_eq!(row, [25, 75, 125, 175]);
    }
}
