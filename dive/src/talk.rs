//! The questions of a dungeon: yes or no, Kecleon's, which move to forget.

use rules::run::dungeon::{Ask, ShopTalk};
use rules::run::{Input, Run};
use shell::Keys;
use stage_view::ui::{BOTTOM, Canvas, ListCursor, Picked, UiArt, YesNo, color};

use crate::Dive;
use crate::Turn;

/// The cursors of the question being asked: its yes or no, its list.
#[derive(Default)]
pub struct Talk {
    answer: YesNo,
    list: ListCursor,
}

impl Talk {
    /// No question is asked: the next starts afresh.
    pub fn done(&mut self) {
        *self = Talk::default();
    }
}

pub fn ask(game: &mut Turn<'_, Dive>, keys: &Keys) -> Input {
    let talk = &mut game.here.talk;
    let Some(yes) = talk.answer.press(keys) else {
        return Input::Menu;
    };
    talk.done();
    Input::Answer(yes)
}

pub fn shop(game: &mut Turn<'_, Dive>, keys: &Keys, talking: ShopTalk) -> Input {
    let talk = &mut game.here.talk;
    if keys.escape {
        talk.done();
        return Input::LeaveShop;
    }
    match talking {
        ShopTalk::Pay => {
            if let Some(yes) = talk.answer.press(keys) {
                talk.done();
                return Input::Pay(yes);
            }
        }
        ShopTalk::Sell => {
            let items = game.run.bag().items.len();
            if let Picked::Chosen(row) = talk.list.keys(keys, items + 1) {
                return if row < items { Input::Sell(row) } else { Input::LeaveShop };
            }
        }
    }
    Input::Menu
}

pub fn learn(game: &mut Turn<'_, Dive>, keys: &Keys) -> Input {
    let known = game.run.dungeon().and_then(|dungeon| dungeon.monster(dungeon.learning.first()?.0)).map_or(0, |monster| monster.moves.len());
    let talk = &mut game.here.talk;
    let forgotten = match talk.list.keys(keys, known + 1) {
        Picked::Chosen(row) => Some(row).filter(|&row| row < known),
        Picked::Left => None,
        _ => return Input::Menu,
    };
    talk.done();
    Input::Forget(forgotten)
}

/// What is being asked, over all the rest.
pub fn draw(canvas: &mut Canvas, ui: &UiArt, run: &Run, talk: &Talk) {
    let Some(dungeon) = run.dungeon() else {
        return;
    };
    let book = &run.book;
    let chosen = Some(talk.answer.cursor());
    if let Some(ask) = &dungeon.ask {
        let title = match ask {
            Ask::Stairs => "Stairs: go on?".to_string(),
            Ask::Recruit(recruit) => format!("Welcome {}?", recruit.name),
            Ask::Leave => "Job done: leave the dungeon?".to_string(),
            Ask::HiddenStairs => "Hidden Stairs: go on?".to_string(),
            Ask::Bazaar(service) => service.question().to_string(),
        };
        canvas.titled_menu(ui, (2, BOTTOM + 2, 26), (&title, color::WHITE), &YesNo::ROWS, 2, chosen, 12);
    }
    if let Some(shop) = &dungeon.shop {
        match shop.talk {
            Some(ShopTalk::Pay) => {
                let total: i32 = shop.owed.iter().map(|(_, price)| price).sum();
                let title = format!("Pay {total} Poké?");
                canvas.titled_menu(ui, (2, BOTTOM + 2, 14), (&title, color::WHITE), &YesNo::ROWS, 2, chosen, 12);
            }
            Some(ShopTalk::Sell) => {
                // (Eight rows at a time: the item, what it sells for.)
                const SHOWN: usize = 8;
                let items = &run.bag().items;
                let mut cursor = talk.list;
                cursor.within(items.len() + 1);
                let first = cursor.first(SHOWN);
                let names: Vec<String> = items.iter().map(|&item| book.item_name(item)).chain(["Nothing".to_string()]).collect();
                let prices: Vec<String> = items.iter().map(|&item| format!("{} P", book.item_prices(item).1)).collect();
                let shown: Vec<(&str, [u8; 3])> = names.iter().skip(first).take(SHOWN).map(|row| (row.as_str(), color::WHITE)).collect();
                let title = format!("Sell?  ({} Poké)", run.bag().money);
                let lines = canvas.titled_menu(ui, (2, BOTTOM + 2, 22), (&title, color::WHITE), &shown, SHOWN, Some(cursor.at - first), 12);
                let prices: Vec<(&str, [u8; 3])> = prices.iter().skip(first).take(SHOWN).map(|price| (price.as_str(), color::CYAN)).collect();
                canvas.right_column(ui, (24 * stage_view::ui::TILE) as i32 - 6, &lines, &prices);
            }
            None => {}
        }
    }
    if let Some((who, slot)) = dungeon.learning.first()
        && let Some(member) = dungeon.monster(*who)
    {
        let title = format!("{}: forget which move?", member.name);
        let mut rows: Vec<String> = member.moves.iter().map(|slot| book.move_name(slot.id)).collect();
        rows.push(format!("Don't learn {}", book.move_name(slot.id)));
        let rows: Vec<(&str, [u8; 3])> =
            rows.iter().enumerate().map(|(at, row)| (row.as_str(), if at < member.moves.len() { color::GREEN } else { color::WHITE })).collect();
        let mut cursor = talk.list;
        cursor.within(rows.len());
        canvas.titled_menu(ui, (2, BOTTOM + 2, 26), (&title, color::WHITE), &rows, 5, Some(cursor.at), 16);
    }
}
