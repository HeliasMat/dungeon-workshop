//! A picture drawn by hand, larger than the game's screen: pictures keep
//! all their pixels, even with the view drawn back.

use glam::Vec2;

use crate::images::{Cover, Frame};

/// One of the console's screens.
pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 192;

/// How a picture is laid on what is there.
#[derive(Clone, Copy)]
pub enum Laid {
    Whole,
    /// As its own alpha has it, over a picture laid just the same way.
    SeeThrough,
    /// That much of it, of 255.
    Faded(u32),
    /// Only its light: what is dark of it shows nothing.
    Light,
}

/// What `Laid::Light` takes for dark.
const DARK: u32 = 40;

/// A picture, so many times as wide and as high as the game's screen.
pub struct Screen {
    pub scale: usize,
    pub wide: usize,
    pub high: usize,
    pub rgba: Vec<u8>,
    /// Which column of a picture each column drawn shows: kept, not to be
    /// made anew for each sprite.
    columns: Vec<usize>,
}

/// A part of a colour (already multiplied by its alpha) over a part of a
/// pixel (by what is left of it): the sum divided by 255, without dividing.
#[inline(always)]
fn mixed(over: u16, under: u8, rest: u16) -> u8 {
    let sum = over + under as u16 * rest;
    ((sum + 1 + (sum >> 8)) >> 8) as u8
}

/// A colour blended over a pixel.
#[inline]
fn blended(under: &mut [u8; 4], colour: [u8; 3], alpha: u32) {
    for (under, over) in under.iter_mut().zip(colour) {
        *under = mixed(over as u16 * alpha as u16, *under, 255 - alpha as u16);
    }
}

/// How a sprite is coloured: each of its colours multiplied by so much (1: as
/// it is), and how much of it is seen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tint {
    pub colour: [f32; 3],
    pub alpha: f32,
}

impl Screen {
    pub fn new(scale: usize) -> Self {
        let (wide, high) = (SCREEN_WIDTH * scale, SCREEN_HEIGHT * scale);
        Screen { scale, wide, high, rgba: vec![0; wide * high * 4], columns: Vec::new() }
    }

    /// One as tall as both of the game's screens.
    pub fn tall(scale: usize) -> Self {
        let (wide, high) = (SCREEN_WIDTH * scale, SCREEN_HEIGHT * 2 * scale);
        Screen { scale, wide, high, rgba: vec![0; wide * high * 4], columns: Vec::new() }
    }

    /// Another screen as big as one of the game's, as the lower of two.
    pub fn lower(&mut self, world: &Screen) {
        let start = (self.high - world.high) * self.wide * 4;
        self.rgba[start..].copy_from_slice(&world.rgba);
    }

    /// A whole canvas over it, both screens.
    pub fn canvas(&mut self, rgba: &[u8]) {
        self.enlarged(rgba);
    }

    /// Pixels of the canvas, each made a square of the scale, from the top.
    fn enlarged(&mut self, rgba: &[u8]) {
        let scale = self.scale;
        for (at, &[red, green, blue, alpha]) in rgba.as_chunks::<4>().0.iter().enumerate() {
            let (x, y) = (at % SCREEN_WIDTH * scale, at / SCREEN_WIDTH * scale);
            match alpha {
                0 => {}
                // (Most of what shows hides what is under it.)
                255 => {
                    for row in y..y + scale {
                        let start = (row * self.wide + x) * 4;
                        for pixel in self.rgba[start..start + scale * 4].as_chunks_mut::<4>().0 {
                            *pixel = [red, green, blue, 255];
                        }
                    }
                }
                _ => self.fill(x as i32, y as i32, scale as i32, scale as i32, [red, green, blue, alpha]),
            }
        }
    }

    /// All black.
    pub fn clear(&mut self) {
        for pixel in self.rgba.as_chunks_mut::<4>().0 {
            *pixel = [0, 0, 0, 255];
        }
    }

    /// All of it without its colours.
    pub fn grey(&mut self) {
        for [red, green, blue, _] in self.rgba.as_chunks_mut::<4>().0 {
            let light = ((*red as u32 * 3 + *green as u32 * 6 + *blue as u32) / 10) as u8;
            (*red, *green, *blue) = (light, light, light);
        }
    }

    /// A box of a colour, blended, cut to the picture.
    pub fn fill(&mut self, x: i32, y: i32, wide: i32, high: i32, [red, green, blue, alpha]: [u8; 4]) {
        let (left, right) = (x.max(0), (x + wide).min(self.wide as i32));
        if alpha == 0 || right <= left {
            return;
        }
        let (left, right) = (left as usize, right as usize);
        // (All four parts of each pixel, its fourth full and staying so, by
        // sixteens: the machine does those together.)
        let colour = [red, green, blue, 255].map(|part| part as u16 * alpha as u16);
        let rest = 255 - alpha as u16;
        let row_of_colour: [u16; 16] = std::array::from_fn(|at| colour[at % 4]);
        for row in y.max(0)..(y + high).min(self.high as i32) {
            let line = row as usize * self.wide;
            let (sixteens, others) = self.rgba[(line + left) * 4..(line + right) * 4].as_chunks_mut::<16>();
            for parts in sixteens {
                for at in 0..16 {
                    parts[at] = mixed(row_of_colour[at], parts[at], rest);
                }
            }
            for (at, part) in others.iter_mut().enumerate() {
                *part = mixed(colour[at % 4], *part, rest);
            }
        }
    }

    /// A shadow on the ground, in the game's own shape: rows of pixels of
    /// the game, each `pixel` wide here, about a point.
    pub fn shadow(&mut self, (x, y): (i32, i32), rows: &[usize], pixel: i32, alpha: u8) {
        let (x, y) = (x - x.rem_euclid(pixel), y - y.rem_euclid(pixel));
        let top = y - rows.len() as i32 * pixel / 2;
        for (at, &wide) in rows.iter().enumerate() {
            let wide = wide as i32 * pixel;
            self.fill(x - wide / 2, top + at as i32 * pixel, wide, pixel, [0, 0, 0, alpha]);
        }
    }

    /// A round light in pixels of the game, of three strengths: the brightest
    /// at its middle.
    pub fn glow(&mut self, (x, y): (i32, i32), radius: f32, [red, green, blue]: [u8; 3], alpha: f32) {
        let pixel = self.scale as i32;
        let (x, y) = (x - x.rem_euclid(pixel), y - y.rem_euclid(pixel));
        let reach = (radius / pixel as f32).ceil() as i32;
        for row in -reach..=reach {
            for column in -reach..=reach {
                let far = ((row * row + column * column) as f32).sqrt() * pixel as f32 / radius.max(1.0);
                if far < 1.0 {
                    let strength = ((1.0 - far) * 3.0).ceil() / 3.0;
                    self.fill(x + column * pixel, y + row * pixel, pixel, pixel, [red, green, blue, (alpha * strength * 255.0).min(255.0) as u8]);
                }
            }
        }
    }

    /// A picture about a point, laid as asked.
    pub fn layer(&mut self, rgba: &[u8], wide: usize, high: usize, centre: Vec2, close: f32, laid: Laid) {
        // (All of it or none: nothing to blend.)
        let laid = match laid {
            Laid::Faded(0) => return,
            Laid::Faded(255) => Laid::Whole,
            laid => laid,
        };
        let across = (0..self.wide).map(|x| centre.x + (x as f32 - self.wide as f32 / 2.0) / close);
        let shown: Vec<(usize, usize)> =
            across.enumerate().filter(|(_, along)| (0.0..wide as f32).contains(along)).map(|(x, along)| (x, along as usize)).collect();
        let (Some(&(first, _)), Some(&(last, _))) = (shown.first(), shown.last()) else {
            return;
        };
        let columns: Vec<usize> = shown.into_iter().map(|(_, column)| column).collect();
        let mut drawn = None;
        for y in 0..self.high {
            let from = centre.y + (y as f32 - self.high as f32 / 2.0) / close;
            if from < 0.0 || from >= high as f32 {
                continue;
            }
            let from = from as usize;
            let Some(source) = rgba.get(from * wide * 4..(from + 1) * wide * 4) else {
                continue;
            };
            let source = source.as_chunks::<4>().0;
            let start = y * self.wide;
            // (A row like the one above it, the picture being larger here.)
            if let (Laid::Whole | Laid::SeeThrough, Some((above, row))) = (laid, drawn)
                && row == from
            {
                self.rgba.copy_within((above + first) * 4..(above + last + 1) * 4, (start + first) * 4);
                continue;
            }
            drawn = Some((start, from));
            let line = self.rgba[(start + first) * 4..(start + last + 1) * 4].as_chunks_mut::<4>().0;
            match laid {
                Laid::Whole => {
                    for (pixel, &column) in line.iter_mut().zip(&columns) {
                        let [red, green, blue, _] = source[column];
                        pixel[..3].copy_from_slice(&[red, green, blue]);
                    }
                }
                Laid::SeeThrough => {
                    for (pixel, &column) in line.iter_mut().zip(&columns) {
                        let [red, green, blue, alpha] = source[column];
                        match alpha {
                            0 => {}
                            255 => pixel[..3].copy_from_slice(&[red, green, blue]),
                            _ => blended(pixel, [red, green, blue], alpha as u32),
                        }
                    }
                }
                Laid::Faded(much) => {
                    for (pixel, &column) in line.iter_mut().zip(&columns) {
                        let [red, green, blue, _] = source[column];
                        blended(pixel, [red, green, blue], much);
                    }
                }
                Laid::Light => {
                    for (pixel, &column) in line.iter_mut().zip(&columns) {
                        let [red, green, blue, _] = source[column];
                        if red as u32 + green as u32 + blue as u32 > DARK {
                            pixel[..3].copy_from_slice(&[red, green, blue]);
                        }
                    }
                }
            }
        }
    }

    /// A sprite's picture centred on a point, at that size, multiplied by a
    /// colour, showing that much.
    pub fn sprite(&mut self, frame: &Frame, at: (i32, i32), size: f32, colour: [f32; 3], alpha: f32) {
        self.sprite_turned(frame, at, size, Tint { colour, alpha }, false, None);
    }

    /// The same, drawn the other way round, left to right, if `flipped`; none
    /// of it from the row `under` down (under the water's surface, or the
    /// sand's), a line of that surface's two colours on it.
    pub fn sprite_turned(&mut self, frame: &Frame, (x, y): (i32, i32), size: f32, tint: Tint, flipped: bool, under: Option<(i32, [[u8; 3]; 2])>) {
        let Tint { colour, alpha } = tint;
        let (under, surface) = (under.map(|(row, _)| row), under.map_or([[0; 3]; 2], |(_, surface)| surface));
        let size = size.max(0.01);
        let (wide, high) = ((frame.wide as f32 * size) as i32, (frame.high as f32 * size) as i32);
        let (left, top) = (x - wide / 2, y - high / 2);
        let (first, end) = ((-left).max(0), wide.min(self.wide as i32 - left));
        if end <= first {
            return;
        }
        self.columns.clear();
        let last = frame.wide.saturating_sub(1);
        self.columns.extend((first..end).map(|column| ((column as f32 / size) as usize).min(last)).map(|column| if flipped { last - column } else { column }));
        // (As it is, most often: nothing to work out for its pixels.)
        let plain = colour == [255.0; 3] && alpha >= 1.0;
        let source = frame.rgba.as_chunks::<4>().0;
        let low = under.map_or(high, |under| high.min(under - top));
        // (Where it goes into the water, a line of foam on it: light, darker,
        // by turns, of whole pixels.)
        if under.is_some() && low > 0 && low < high {
            let from = (low as f32 / size) as usize * frame.wide;
            let source = frame.rgba.as_chunks::<4>().0;
            let pixel = size.round().max(1.0) as i32;
            for (at, &column) in self.columns.clone().iter().enumerate() {
                if source.get(from + column).is_some_and(|colour| colour[3] > 0) {
                    let x = left + first + at as i32;
                    let foam = surface[(column / 3) % 2];
                    for y in (top + low).max(0)..(top + low + pixel).min(self.high as i32) {
                        let start = (y as usize * self.wide + x as usize) * 4;
                        self.rgba[start..start + 3].copy_from_slice(&foam);
                    }
                }
            }
        }
        for row in (-top).max(0)..low.min(self.high as i32 - top) {
            let from = (row as f32 / size) as usize * frame.wide;
            let Some(source) = source.get(from..from + frame.wide) else {
                continue;
            };
            let start = (top + row) as usize * self.wide + (left + first) as usize;
            let line = self.rgba[start * 4..(start + self.columns.len()) * 4].as_chunks_mut::<4>().0;
            for (pixel, &column) in line.iter_mut().zip(&self.columns) {
                let [red, green, blue, shows] = source[column];
                if plain {
                    match shows {
                        0 => {}
                        255 => pixel[..3].copy_from_slice(&[red, green, blue]),
                        _ => blended(pixel, [red, green, blue], shows as u32),
                    }
                    continue;
                }
                let shows = (shows as f32 * alpha) as u32;
                if shows > 0 {
                    let tinted = [red as f32 * colour[0] / 255.0, green as f32 * colour[1] / 255.0, blue as f32 * colour[2] / 255.0].map(|part| part as u8);
                    blended(pixel, tinted, shows);
                }
            }
        }
    }

    /// A sprite's picture laid through any map, from its pixels to the
    /// screen's (see [`crate::blit::blit`]): turned, sized, slanted.
    pub fn blit(&mut self, frame: &Frame, map: glam::Affine2, paint: &crate::blit::Paint) {
        crate::blit::blit(&mut self.rgba, (self.wide, self.high), &frame.rgba, (frame.wide, frame.high), map, paint);
    }

    /// The same, with what `around` asks laid under it (see `blit_with`).
    pub fn blit_with(&mut self, frame: &Frame, map: glam::Affine2, paint: &crate::blit::Paint, around: &crate::blit::Around) {
        crate::blit::blit_with(&mut self.rgba, (self.wide, self.high), &frame.rgba, (frame.wide, frame.high), map, paint, around);
    }

    /// What of the upper layer `rgba` stands before someone whose feet are at
    /// `feet`, painted again over a box of the picture, in the light of the
    /// place. Pictures laid as `layer` lays them.
    pub fn cover(
        &mut self,
        rgba: &[u8],
        (wide, high): (usize, usize),
        cover: &Cover,
        centre: Vec2,
        close: f32,
        feet: f32,
        (left, top, right, bottom): (i32, i32, i32, i32),
        [red, green, blue, tint]: [u8; 4],
    ) {
        let (left, right) = (left.max(0) as usize, right.clamp(0, self.wide as i32) as usize);
        let (top, bottom) = (top.max(0) as usize, bottom.clamp(0, self.high as i32) as usize);
        let place = |pixel: usize, middle: f32, size: usize| middle + (pixel as f32 - size as f32 / 2.0) / close;
        // (Most stand before nothing: look at the tiles first.)
        let tile = |x: f32, y: f32| -> Option<usize> {
            let (column, row) = (x as usize / Cover::TILE, y as usize / Cover::TILE);
            (x >= 0.0 && y >= 0.0 && column < cover.tiles_wide).then(|| row * cover.tiles_wide + column)
        };
        let near = |x: f32, y: f32| tile(x, y).and_then(|tile| cover.ground_line.get(tile)).is_some_and(|line| *line as f32 > feet);
        let (from, to) = (place(left, centre.x, self.wide), place(right, centre.x, self.wide));
        let (above, below) = (place(top, centre.y, self.high), place(bottom, centre.y, self.high));
        let any = (0..=((below - above) / Cover::TILE as f32) as usize + 1).any(|row| {
            (0..=((to - from) / Cover::TILE as f32) as usize + 1).any(|column| near(from + (column * Cover::TILE) as f32, above + (row * Cover::TILE) as f32))
        });
        if !any {
            return;
        }
        for y in top..bottom {
            let from_y = place(y, centre.y, self.high);
            for x in left..right {
                let along = place(x, centre.x, self.wide);
                if near(along, from_y) && along < wide as f32 && from_y < high as f32 {
                    let source = (from_y as usize * wide + along as usize) * 4;
                    if let Some(&[r, g, b, alpha]) = rgba.get(source..source + 4).and_then(|pixel| <&[u8; 4]>::try_from(pixel).ok())
                        && alpha > 0
                    {
                        let pixel: &mut [u8; 4] = (&mut self.rgba[(y * self.wide + x) * 4..][..4]).try_into().expect("four parts");
                        blended(&mut *pixel, [r, g, b], alpha as u32);
                        blended(pixel, [red, green, blue], tint as u32);
                    }
                }
            }
        }
    }

    /// The lower screen of a canvas over it: its words, its light.
    pub fn over(&mut self, rgba: &[u8]) {
        self.enlarged(&rgba[SCREEN_WIDTH * SCREEN_HEIGHT * 4..]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_laid_over_is_mixed_as_its_alpha_says() {
        // (Over every grey, every alpha: the exact blend, rounded, within one.)
        let mut screen = Screen::new(1);
        for under in [0u8, 1, 37, 128, 200, 255] {
            for alpha in [1u8, 64, 128, 200, 254, 255] {
                screen.rgba[..4].copy_from_slice(&[under, under, under, 255]);
                screen.fill(0, 0, 1, 1, [255, 0, 90, alpha]);
                let exact = |over: u8| (over as f32 * alpha as f32 + under as f32 * (255 - alpha) as f32) / 255.0;
                for (part, over) in [255u8, 0, 90].into_iter().enumerate() {
                    assert!((screen.rgba[part] as f32 - exact(over)).abs() <= 1.0, "{under} under {over} at {alpha}");
                }
                assert_eq!(screen.rgba[3], 255);
            }
        }
    }

    #[test]
    fn the_lower_screen_laid_over_is_the_canvas_s_own() {
        // (Its pixels, whole or seen through, each a square of the scale.)
        let mut canvas = vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT * 2 * 4];
        for (at, pixel) in canvas.as_chunks_mut::<4>().0.iter_mut().enumerate().skip(SCREEN_WIDTH * SCREEN_HEIGHT) {
            pixel.copy_from_slice(&[(at % 251) as u8, (at % 97) as u8, 40, [0, 255, 128][at % 3]]);
        }
        let (mut over, mut whole) = (Screen::new(2), Screen::new(2));
        over.over(&canvas);
        whole.canvas(&canvas[SCREEN_WIDTH * SCREEN_HEIGHT * 4..]);
        assert!(over.rgba == whole.rgba);
    }
}
