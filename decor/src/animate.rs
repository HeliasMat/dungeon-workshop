//! An element's own life (`put rocher 8 8 anim glow slow`): its pictures
//! made again, frame after frame, as the game's places live: water that
//! ripples, a lantern that glows, colours that turn as the game turns its
//! palettes, a waterfall that runs. In whole pixels, as the game draws: a
//! row shifted by a pixel, a colour taken for another, never a blur.

use crate::pixels::{Grid, Picture};

/// The ways an element lives, and what each does.
pub const KINDS: [(&str, &str); 8] = [
    ("wave", "wave: its rows ripple by one pixel, like water or hot air"),
    ("float", "float: it rises and sinks by one pixel, as if laid on water"),
    ("sway", "sway: its top sways, like a tree in the wind"),
    ("glow", "glow: it lights up and darkens, like an ember, lava, a lantern"),
    ("twinkle", "twinkle: points of light pass over it, like a crystal or water in the sun"),
    ("colours", "colours: its colours cycle, like the game's waterfalls and lights"),
    ("scroll", "scroll: what is inside it flows downward, like a waterfall"),
    ("blink", "blink: it appears and disappears"),
];

/// How fast: so many frames for a whole turn, each of `SECONDS`.
pub const PACES: [(&str, usize); 3] = [("slow", 24), ("normal", 12), ("fast", 8)];

/// How long a frame of an element's life shows, as the game's water does.
pub const SECONDS: f64 = 0.1;

/// The room kept around an element that moves (`wave`, `float`,
/// `sway`), not to lose its edges when it goes a pixel beyond them.
pub const MARGIN: usize = 2;

/// Whether a way of living moves its pixels (the element is given room).
pub fn moves(kind: &str) -> bool {
    matches!(kind, "wave" | "float" | "sway")
}

/// So many frames for a whole turn at that pace (`normal` if none or unknown).
pub fn frames(pace: Option<&str>) -> usize {
    PACES.iter().find(|(name, _)| Some(*name) == pace).map_or(12, |(_, count)| *count)
}

/// A picture with `margin` empty pixels around.
pub fn padded<T: Copy + Default>(grid: &Grid<T>, margin: usize) -> Grid<T> {
    let mut out = Grid::new(grid.w + 2 * margin, grid.h + 2 * margin);
    for y in 0..grid.h {
        for x in 0..grid.w {
            out.set(x + margin, y + margin, grid.get(x, y));
        }
    }
    out
}

/// How bright a colour is, 0 to 255.
fn brightness([red, green, blue, _]: [u8; 4]) -> u32 {
    (red as u32 * 3 + green as u32 * 6 + blue as u32) / 10
}

/// A number of a pixel, the same each time, to choose some of them.
fn mixed(x: usize, y: usize, salt: usize) -> usize {
    let mut value = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ (salt as u64).wrapping_mul(0x1656_67B1);
    value ^= value >> 29;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    (value ^ (value >> 32)) as usize
}

/// A picture with each row moved so many pixels across (`by(row)`).
fn rows_shifted(picture: &Picture, by: impl Fn(usize) -> i64) -> Picture {
    let mut out = Picture::new(picture.w, picture.h);
    for y in 0..picture.h {
        let shift = by(y);
        for x in 0..picture.w {
            let from = x as i64 - shift;
            if (0..picture.w as i64).contains(&from) {
                out.set(x, y, picture.get(from as usize, y));
            }
        }
    }
    out
}

/// A picture moved down so many pixels (up if fewer than none).
fn moved_down(picture: &Picture, by: i64) -> Picture {
    let mut out = Picture::new(picture.w, picture.h);
    for y in 0..picture.h {
        let from = y as i64 - by;
        if (0..picture.h as i64).contains(&from) {
            for x in 0..picture.w {
                out.set(x, y, picture.get(x, from as usize));
            }
        }
    }
    out
}

/// The pictures of an element living that way, `count` of them for a whole
/// turn: frame `at` made of its own frame `at` (if it has several, as the
/// game's water has), so many of each.
pub fn animated(pictures: &[Picture], kind: &str, count: usize) -> Vec<Picture> {
    let count = count.max(1);
    // (Its colours, darkest first, the outline's left out: those that turn.)
    let mut turning: Vec<[u8; 4]> = Vec::new();
    if kind == "colours" {
        for pixel in pictures[0].cells.iter().filter(|pixel| pixel[3] > 0 && brightness(**pixel) > 48) {
            let opaque = [pixel[0], pixel[1], pixel[2], 255];
            if !turning.contains(&opaque) {
                turning.push(opaque);
            }
        }
        turning.sort_by_key(|colour| brightness(*colour));
        turning.truncate(16);
    }
    (0..count)
        .map(|at| {
            let picture = &pictures[at % pictures.len()];
            let turn = at as f32 / count as f32 * std::f32::consts::TAU;
            match kind {
                "wave" => rows_shifted(picture, |y| (turn + y as f32 * 0.55).sin().round() as i64),
                "float" => moved_down(picture, turn.sin().round() as i64),
                "sway" => {
                    let high = picture.h.max(1) as f32;
                    // (The top the most, the foot not at all.)
                    rows_shifted(picture, |y| (turn.sin() * 2.0 * (1.0 - y as f32 / high).powf(1.5)).round() as i64)
                }
                "glow" => {
                    let light = 1.0 + 0.2 * turn.sin();
                    picture.map(|[red, green, blue, seen]| {
                        let lit = |part: u8| (part as f32 * light).round().clamp(0.0, 255.0) as u8;
                        [lit(red), lit(green), lit(blue), seen]
                    })
                }
                "twinkle" => {
                    let mut out = picture.clone();
                    for y in 0..picture.h {
                        for x in 0..picture.w {
                            let pixel = picture.get(x, y);
                            if pixel[3] == 0 || brightness(pixel) < 60 || !mixed(x, y, 7).is_multiple_of(9) {
                                continue;
                            }
                            // (Each its moment in the turn: white then, half white just before and after.)
                            let moment = mixed(x, y, 3) % count;
                            let far = (at + count - moment) % count;
                            let lit = match far.min(count - far) {
                                0 => 255,
                                1 => 128,
                                _ => continue,
                            };
                            let white = |part: u8| (part as u32 + (255 - part as u32) * lit / 255) as u8;
                            out.set(x, y, [white(pixel[0]), white(pixel[1]), white(pixel[2]), pixel[3]]);
                        }
                    }
                    out
                }
                "colours" if turning.len() > 1 => {
                    let step = at * turning.len() / count;
                    picture.map(|pixel| {
                        let opaque = [pixel[0], pixel[1], pixel[2], 255];
                        match turning.iter().position(|colour| *colour == opaque) {
                            Some(rank) => {
                                let [red, green, blue, _] = turning[(rank + step) % turning.len()];
                                [red, green, blue, pixel[3]]
                            }
                            None => pixel,
                        }
                    })
                }
                "scroll" => {
                    // (What it has inside, not its outline, runs down its column,
                    // a whole height a turn.)
                    let inside = |pixel: [u8; 4]| pixel[3] > 0 && brightness(pixel) > 48;
                    let mut out = picture.clone();
                    let shift = at * picture.h / count;
                    for x in 0..picture.w {
                        for y in 0..picture.h {
                            if !inside(picture.get(x, y)) {
                                continue;
                            }
                            let from = (y + picture.h - shift % picture.h.max(1)) % picture.h.max(1);
                            let taken = picture.get(x, from);
                            if inside(taken) {
                                out.set(x, y, taken);
                            }
                        }
                    }
                    out
                }
                "blink" if at * 2 >= count => Picture::new(picture.w, picture.h),
                _ => picture.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Picture {
        let mut picture = Picture::new(6, 6);
        for y in 2..4 {
            for x in 2..4 {
                picture.set(x, y, [200, 120, 60, 255]);
            }
        }
        picture.set(1, 1, [90, 160, 230, 255]);
        picture
    }

    #[test]
    fn an_element_lives_a_whole_turn_in_whole_pixels() {
        for (kind, _) in KINDS {
            for (pace, count) in PACES {
                let made = animated(&[square()], kind, frames(Some(pace)));
                assert_eq!(made.len(), count, "{kind} {pace}");
                assert!(made.iter().all(|frame| (frame.w, frame.h) == (6, 6)));
            }
        }
        // (Floating: a pixel up, a pixel down, back where it was.)
        let floating = animated(&[square()], "float", 12);
        assert_eq!(floating[0], square());
        assert!(floating.iter().any(|frame| frame.get(2, 1)[3] > 0) && floating.iter().any(|frame| frame.get(2, 4)[3] > 0));
        // (Glowing: brighter, then darker, its outline as it was.)
        let glowing = animated(&[square()], "glow", 12);
        assert!(glowing.iter().any(|frame| frame.get(2, 2)[0] > 200) && glowing.iter().any(|frame| frame.get(2, 2)[0] < 200));
        // (Blinking: there half the turn.)
        let blinking = animated(&[square()], "blink", 8);
        assert_eq!(blinking.iter().filter(|frame| frame.cells.iter().any(|pixel| pixel[3] > 0)).count(), 4);
        // (Its colours turning: the two it has, each in the other's place.)
        let turning = animated(&[square()], "colours", 12);
        assert!(turning.iter().any(|frame| frame.get(2, 2) == [90, 160, 230, 255]));
    }

    #[test]
    fn room_is_kept_around_what_moves() {
        let padded = padded(&square(), MARGIN);
        assert_eq!((padded.w, padded.h), (6 + 2 * MARGIN, 6 + 2 * MARGIN));
        assert_eq!(padded.get(2 + MARGIN, 2 + MARGIN), [200, 120, 60, 255]);
        assert!(moves("float") && !moves("glow"));
    }
}
