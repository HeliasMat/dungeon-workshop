//! The town's services drawn, as the original shows them: a list of items
//! read by pages with what each costs and the money carried beside it, and an
//! amount entered digit by digit at the bank. (Used by the game, and by a
//! film one plays: `preview::screens`.)

use rules::services::{self, Service};

use super::message::Words;
use super::{BOTTOM, Canvas, SCREEN_HEIGHT, TILE, UiArt, color, mark, strip_tags, text};

/// An amount entered digit by digit, as the original's bank has it: its own
/// window over the message, the digit being changed in yellow between two
/// arrows, and what there is to take from.
pub fn draw_amount(canvas: &mut Canvas, art: &UiArt, (amount, digit, deposit): (i32, u32, bool), there: (i32, i32)) {
    const WIDE: i32 = 12;
    let below = SCREEN_HEIGHT as i32;
    let digits = Service::DIGITS as i32;
    canvas.window(art, 9, BOTTOM - 7, 14, 6);
    let left = (9 * TILE) as i32 + 12;
    let top = below + (BOTTOM as i32 - 7) * TILE as i32 + 6;
    let shown = format!("{amount:0width$}", width = digits as usize);
    for (at, figure) in shown.chars().enumerate() {
        let x = left + at as i32 * WIDE;
        let chosen = digits - 1 - at as i32 == digit as i32;
        canvas.text(art, x, top + 8, &figure.to_string(), if chosen { color::YELLOW } else { color::WHITE });
        if chosen {
            // (The arrows: a small triangle above, one below.)
            for row in 0..3 {
                canvas.fill(x + 2 - row, top + 2 + row, (1 + row * 2) as usize, 1, [248, 248, 0, 255]);
                canvas.fill(x + 2 - row, top + 26 - row, (1 + row * 2) as usize, 1, [248, 248, 0, 255]);
            }
        }
    }
    canvas.mark(art, left + digits * WIDE + 2, top + 8, mark::POKE);
    let (label, there) = if deposit { ("On hand", there.0) } else { ("In the bank", there.1) };
    canvas.text(art, left - 2, top + 32, label, color::WHITE);
    canvas.text_right(art, (23 * TILE) as i32 - 8, top + 32, &there.to_string(), color::CYAN);
}

/// A service's list of items: a titled list on the left, read by pages,
/// with what each costs; the money carried on the right.
pub fn draw_list(canvas: &mut Canvas, art: &UiArt, title: &str, words: &Words, money: i32) {
    const SHOWN: usize = services::LIST_PAGE;
    let below = SCREEN_HEIGHT as i32;
    let first = words.cursor.page(SHOWN);
    let pages = words.options.len().div_ceil(SHOWN);
    let title = format!("{title}  {}/{pages}", first / SHOWN + 1);
    // (An entry is a name, then maybe what it costs.)
    let entry = |option: &String| match option.split_once("  ") {
        Some((name, price)) => (name.to_string(), strip_tags(price)),
        None => (option.clone(), String::new()),
    };
    let entries: Vec<(String, String)> = words.options.iter().skip(first).take(SHOWN).map(entry).collect();
    let rows: Vec<(&str, [u8; 3])> = entries.iter().map(|(name, _)| (name.as_str(), color::WHITE)).collect();
    let lines = canvas.titled_menu(art, (2, BOTTOM, 20), (&title, color::WHITE), &rows, SHOWN, Some(words.cursor.at - first), 12);
    let prices: Vec<(&str, [u8; 3])> = entries.iter().map(|(_, price)| (price.as_str(), color::CYAN)).collect();
    canvas.right_column(art, (22 * TILE) as i32 - 6, &lines, &prices);
    canvas.window(art, 23, BOTTOM, 7, 4);
    canvas.text(art, (23 * TILE) as i32 + 4, below + 2, art.label(text::MONEY), color::WHITE);
    canvas.text_right(art, (30 * TILE) as i32 - 16, below + 16, &money.to_string(), color::CYAN);
    canvas.mark(art, (30 * TILE) as i32 - 14, below + 16, mark::POKE);
}
