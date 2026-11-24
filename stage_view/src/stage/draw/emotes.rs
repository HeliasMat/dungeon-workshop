//! What shows over those of the stage, as the engine's `Emotes` has it:
//! statuses over heads, emotes and effects at their point on a body, the
//! numbers of blows in the game's digits.

use engine::{Emotes, Who, attribute};
use glam::Vec2;

use super::{Brush, Play};
use crate::stage::pose::way_of;
use crate::ui::Canvas;
use shell::Screen;
use shell::art::Art;

/// The side of a digit of the game's.
const DIGIT: usize = 8;

/// A number of a blow in the game's digits, if it is one: its row (yellow
/// lost, red past 999, green gained or missed), where each cell starts in
/// the row and how wide it is.
pub(super) fn number_cells(text: &str) -> Option<(usize, Vec<(usize, usize)>)> {
    if matches!(text, "Miss!" | "MISS") {
        return Some((2, vec![(96, 32)]));
    }
    let digits = text.strip_prefix(['-', '+'])?;
    let worth: u32 = digits.parse().ok()?;
    let row = if text.starts_with('+') { 2 } else { usize::from(worth > 999) };
    let cell = |letter: char| letter.to_digit(10).map_or(if letter == '+' { 80 } else { 88 }, |digit| digit as usize * DIGIT);
    Some((row, text.chars().map(|letter| (cell(letter), DIGIT)).collect()))
}

/// How far apart, side by side, the statuses over one head show.
const SIDE_BY_SIDE: f32 = 14.0;

impl Play {
    /// The statuses over heads, in the order of who has them.
    pub(super) fn draw_statuses(&self, world: &mut Screen, brush: &Brush, art: &Art) {
        let (view, clock) = (&brush.large, self.sim.clock as usize);
        // (Several over one, side by side.)
        let mut icons: Vec<(Who, &'static str, f32)> = self
            .sim
            .stage
            .emotes
            .statuses
            .iter()
            .flat_map(|(who, icons)| icons.iter().enumerate().map(move |(at, icon)| (*who, *icon, (at as f32 - (icons.len() - 1) as f32 / 2.0) * SIDE_BY_SIDE)))
            .collect();
        icons.sort_by_key(|(who, _, _)| who.number());
        for (who, icon, aside) in icons {
            let (Some(actor), Some(frames)) = (self.sim.stage.cast.get(&who), art.status(icon)) else {
                continue;
            };
            // (Not over one who is not seen: hidden in a bush, gone.)
            if actor.attributes & attribute::HIDDEN != 0 || actor.shown_opacity() < 0.1 || frames.is_empty() {
                continue;
            }
            let frame = &frames[clock / 4 % frames.len()];
            let at = view.on_screen(Emotes::status_point(actor, icon) + Vec2::new(aside, 0.0));
            world.sprite(frame, (at.x as i32, at.y as i32), view.close, [255.0; 3], 1.0);
        }
    }

    /// The emotes and effects, each at its point on the body.
    pub(super) fn draw_effects(&self, world: &mut Screen, brush: &Brush, art: &Art) {
        let view = &brush.large;
        for ((who, _), shown) in &self.sim.effects {
            let Some(actor) = self.sim.stage.cast.get(who) else {
                continue;
            };
            let ways = shown.art.sprite.ways(0);
            let steps = ways.get(if ways.len() == 8 { way_of(actor.direction) } else { 0 });
            let Some(steps) = steps.filter(|steps| !steps.is_empty()) else {
                continue;
            };
            let whole: u32 = steps.iter().map(|frame| frame.ticks).sum();
            let mut left = shown.gone as u32 % whole.max(1);
            let frame = steps.iter().find(|frame| {
                let here = left < frame.ticks;
                left = left.saturating_sub(frame.ticks);
                here
            });
            if let Some(frame) = frame {
                // (Its head, a hand, its middle on the step it shows: a move
                // leaves from its mouth as it opens it.)
                let now = actor.body.and_then(|body| body.now.get(shown.point as usize).copied()).map(|at| (at.x as i32, at.y as i32));
                let (x, y) = now.unwrap_or_else(|| art.point(self.sim.cast.who(*who).0, shown.point));
                let on = view.on_screen(actor.shown_at() + Vec2::new(x as f32, y as f32 - actor.shown_height()));
                world.sprite(frame, (on.x as i32, on.y as i32), view.close, [255.0; 3], 1.0);
            }
        }
    }

    /// The numbers of blows rising, in the game's own digits.
    pub(super) fn draw_digits(&self, canvas: &mut Canvas, brush: &Brush, art: &Art) {
        let Some((sheet_wide, _, sheet)) = art.digits() else {
            return;
        };
        for popup in &self.sim.stage.emotes.popups {
            let Some((row, cells)) = number_cells(&popup.text) else {
                continue;
            };
            let wide: usize = cells.iter().map(|(_, wide)| wide).sum();
            let (x, y) = brush.on_screen(popup.at);
            let mut left = x - wide as i32 / 2;
            for (from, cell) in cells {
                let rows = (0..DIGIT)
                    .filter_map(|line| sheet.get(((row * DIGIT + line) * sheet_wide + from) * 4..((row * DIGIT + line) * sheet_wide + from + cell) * 4));
                let picture: Vec<u8> = rows.flatten().copied().collect();
                canvas.blit(left, y, cell, DIGIT, &picture);
                left += cell as i32;
            }
        }
    }
}
