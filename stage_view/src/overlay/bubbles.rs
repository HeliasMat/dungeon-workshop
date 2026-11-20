//! Bubbles, windows and captions: the words of a scene.

use super::*;

/// The widest of these lines, without their colour tags.
pub(super) fn widest(art: &UiArt, lines: &[&str]) -> i32 {
    lines.iter().map(|line| art.font.width(&ui::strip_tags(line))).max().unwrap_or(0) as i32
}

/// Writes lines centred in a box `wide` pixels across from `left`: in
/// that colour, or in the lines' own colours where they have tags.
pub(super) fn centred_lines(canvas: &mut Canvas, art: &UiArt, lines: &[&str], left: i32, top: i32, wide: i32, ink: [u8; 3]) {
    for (row, line) in lines.iter().enumerate() {
        let shown = ui::strip_tags(line);
        let x = left + (wide - art.font.width(&shown) as i32) / 2;
        let y = top + row as i32 * PITCH;
        match line.contains("[C") {
            true => canvas.markup(art, x, y, wide as usize, line),
            false => {
                canvas.text(art, x, y, &shown, ink);
            }
        }
    }
}

/// How big a bubble is: its inside (the window's size in tiles in the
/// game's look), the margin its frame takes round it, and its tail's
/// length under it.
pub(super) struct Shape {
    pub(super) wide: i32,
    pub(super) high: i32,
    pub(super) margin: i32,
    pub(super) tail: i32,
    pub(super) tiles: (usize, usize),
}

/// Where a bubble's inside is, and the box it takes with its frame.
#[derive(Clone, Copy)]
pub(super) struct Placed {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) outer: (i32, i32, i32, i32),
}

impl Shape {
    pub(super) fn of(art: &UiArt, lines: &[&str], style: u8, voice: Voice) -> Self {
        let text_high = lines.len() as i32 * PITCH;
        match style {
            1 => {
                use ui::TILE;
                let tiles = (((widest(art, lines) + 6) as usize).div_ceil(TILE).max(2), ((text_high + 2) as usize).div_ceil(TILE));
                let (wide, high) = ((tiles.0 * TILE) as i32, (tiles.1 * TILE) as i32);
                Self { wide, high, margin: 5, tail: if voice == Voice::Thought { 12 } else { 8 }, tiles }
            }
            _ => Self { wide: widest(art, lines) + 10, high: text_high + 4, margin: 2, tail: if voice == Voice::Thought { 9 } else { 5 }, tiles: (0, 0) },
        }
    }

    /// Over the point its tail goes down to, inside the screen and under
    /// the ceiling.
    pub(super) fn over(&self, (x, tip): (i32, i32), ceiling: i32) -> Placed {
        // (One wider than the screen starts at its edge.)
        let left = (x - self.wide / 2).clamp(self.margin, (ui::SCREEN_WIDTH as i32 - self.wide - self.margin).max(self.margin));
        let top = (tip - self.high - self.tail).max(ceiling + self.margin);
        Placed { left, top, outer: (left - self.margin, top - self.margin, self.wide + self.margin * 2, self.high + self.margin * 2) }
    }
}

impl Placed {
    /// Up off a box under it, and aside off a portrait.
    pub(super) fn keep_off(&mut self, floor: Option<i32>, corner: Option<(i32, i32, i32)>) {
        let (left, top, wide, high) = self.outer;
        if let Some(over) = floor.map(|floor| top + high - floor).filter(|over| *over > 0) {
            *self = self.moved(0, -over);
        }
        if let Some((from, to, portrait_top)) = corner
            && self.outer.1 + high > portrait_top
            && left < to
            && left + wide > from
        {
            let aside = if from > ui::SCREEN_WIDTH as i32 / 2 { from - (left + wide) } else { to - left };
            *self = self.moved(aside, 0);
        }
    }

    pub(super) fn overlaps(&self, other: &Placed) -> bool {
        let (a, b) = (self.outer, other.outer);
        a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
    }

    pub(super) fn moved(&self, dx: i32, dy: i32) -> Placed {
        Placed { left: self.left + dx, top: self.top + dy, outer: (self.outer.0 + dx, self.outer.1 + dy, self.outer.2, self.outer.3) }
    }

    /// Out of the way of those already placed: sideways, away from the
    /// one it covers, while the screen has room; else above it.
    pub(super) fn clear_of(&mut self, placed: &[Placed], ceiling: i32) {
        for _ in 0..8 {
            let Some(other) = placed.iter().find(|other| self.overlaps(other)) else {
                return;
            };
            let (mine, theirs) = (self.outer.0 * 2 + self.outer.2, other.outer.0 * 2 + other.outer.2);
            let aside = match mine >= theirs {
                true => other.outer.0 + other.outer.2 - self.outer.0 + 1,
                false => other.outer.0 - (self.outer.0 + self.outer.2) - 1,
            };
            let moved = self.moved(aside, 0);
            let inside = moved.outer.0 >= 0 && moved.outer.0 + moved.outer.2 <= ui::SCREEN_WIDTH as i32;
            *self = match inside {
                true => moved,
                false => self.moved(0, (other.outer.1 - (self.outer.1 + self.outer.3) - 1).max(ceiling - self.outer.1)),
            };
        }
    }
}

/// The classic look: white, rounded, a dark edge; a pointed tail for
/// words said, small round puffs for thoughts, spikes and red for a shout.
pub(super) fn classic_bubble(canvas: &mut Canvas, art: &UiArt, lines: &[&str], voice: Voice, shape: &Shape, place: &Placed, x: i32) {
    const PAPER: [u8; 4] = [248, 248, 248, 255];
    let (ink, edge) = match voice {
        Voice::Shouted => ([200, 32, 32], [200, 32, 32, 255]),
        _ => ([40, 40, 56], [40, 40, 56, 255]),
    };
    let (wide, high, left, top) = (shape.wide, shape.high, place.left, place.top);
    canvas.fill(left + 1, top, (wide - 2) as usize, 1, edge);
    canvas.fill(left + 1, top + high - 1, (wide - 2) as usize, 1, edge);
    canvas.fill(left, top + 1, 1, (high - 2) as usize, edge);
    canvas.fill(left + wide - 1, top + 1, 1, (high - 2) as usize, edge);
    canvas.fill(left + 1, top + 1, (wide - 2) as usize, (high - 2) as usize, PAPER);
    let foot = x.clamp(left + 5, left + wide - 6);
    match voice {
        // (Puffs going down to the head, smaller and smaller.)
        Voice::Thought => {
            for (at, size) in [(2, 4), (6, 2)] {
                let (px, py) = (foot - size / 2 + at / 3, top + high + at - 1);
                canvas.fill(px - 1, py, (size + 2) as usize, size as usize, edge);
                canvas.fill(px, py - 1, size as usize, (size + 2) as usize, edge);
                canvas.fill(px, py, size as usize, size as usize, PAPER);
            }
        }
        _ => {
            for row in 0..4 {
                canvas.fill(foot - 3 + row, top + high - 1 + row, (7 - row * 2).max(1) as usize, 1, PAPER);
                canvas.fill(foot - 4 + row, top + high - 1 + row, 1, 1, edge);
                canvas.fill(foot + 3 - row, top + high - 1 + row, 1, 1, edge);
            }
        }
    }
    // (A shout bristles: little spikes at its corners.)
    if voice == Voice::Shouted {
        for (cx, cy, dx, dy) in [(left, top, -1, -1), (left + wide - 1, top, 1, -1), (left, top + high - 1, -1, 1), (left + wide - 1, top + high - 1, 1, 1)] {
            for step in 1..3 {
                canvas.fill(cx + dx * step, cy + dy * step, 1, 1, edge);
            }
        }
    }
    // (On paper, no shadow: it would read as the words written twice.)
    for (row, line) in lines.iter().enumerate() {
        let shown = ui::strip_tags(line);
        let x = left + (wide - art.font.width(&shown) as i32) / 2;
        canvas.text_plain(art, x, top + 2 + row as i32 * PITCH, &shown, ink);
    }
}

/// The game's own small windows: the dialogue's grey, the portraits' thin
/// frame in the frames' colour and type, white text; a tail opened
/// through the frame for words said, two small framed squares for
/// thoughts, yellow words for a shout.
pub(super) fn window_bubble(canvas: &mut Canvas, art: &UiArt, lines: &[&str], voice: Voice, shape: &Shape, place: &Placed, x: i32) {
    use ui::color;
    const GREY: [u8; 4] = [32, 32, 32, 255];
    // (The frame's ring is 4 pixels round the inside, the tail 4 more.)
    const RING: i32 = 4;
    let (wide, high, left, top) = (shape.wide, shape.high, place.left, place.top);
    let edge = canvas.small_window(art, left, top, shape.tiles.0, shape.tiles.1);
    let foot = x.clamp(left + 6, left + wide - 7);
    let under = top + high;
    match voice {
        Voice::Thought => {
            for (at, size) in [(RING + 1, 4), (RING + 6, 2)] {
                let px = foot - size / 2;
                canvas.fill(px - 1, under + at - 1, (size + 2) as usize, (size + 2) as usize, edge);
                canvas.fill(px, under + at, size as usize, size as usize, GREY);
            }
        }
        _ => {
            for row in 0..RING * 2 {
                let half = if row < RING { 3 } else { 3 - (row - RING) };
                canvas.fill(foot - half, under + row, (half * 2 + 1) as usize, 1, GREY);
                if row >= RING - 1 {
                    canvas.fill(foot - half - 1, under + row, 1, 1, edge);
                    canvas.fill(foot + half + 1, under + row, 1, 1, edge);
                }
            }
            canvas.fill(foot, under + RING * 2, 1, 1, edge);
        }
    }
    let ink = if voice == Voice::Shouted { color::YELLOW } else { color::WHITE };
    centred_lines(canvas, art, lines, left, top + 1, wide, ink);
}

/// A caption near the top, in the chosen look: a dark band, or the game's
/// small window.
pub(super) fn caption(canvas: &mut Canvas, art: &UiArt, lines: &[&str], style: u8, top: i32) {
    use ui::{TILE, color};
    match style {
        1 => {
            let tiles_wide = ((widest(art, lines) + 10) as usize).div_ceil(TILE);
            let tiles_high = (lines.len() * PITCH as usize + 2).div_ceil(TILE);
            let wide = (tiles_wide * TILE) as i32;
            let left = (ui::SCREEN_WIDTH as i32 - wide) / 2;
            canvas.small_window(art, left, top, tiles_wide, tiles_high);
            centred_lines(canvas, art, lines, left, top + 1, wide, color::WHITE);
        }
        _ => {
            let high = lines.len() * PITCH as usize + 6;
            canvas.fill(0, top - 2, ui::SCREEN_WIDTH, high, [0, 0, 0, 150]);
            centred_lines(canvas, art, lines, 0, top + 1, ui::SCREEN_WIDTH as i32, color::WHITE);
        }
    }
}
