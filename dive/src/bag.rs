//! The bag in a dungeon: the list, what can be done with an item (asked of
//! `Run::item_actions`), what it does.

use rules::dungeon::stack;
use rules::run::{Input, ItemAction, Run};
use shell::Keys;
use stage_view::ui::{BOTTOM, Canvas, ListCursor, Picked, Rows, SCREEN_HEIGHT, TILE, UiArt, color};

use crate::Dive;
use crate::Turn;

/// Items a page.
const SHOWN: usize = 8;
/// The frames what was refused is told in place of the description.
const TOLD: u32 = 120;

/// The cursor of the bag; of its actions, once an item was chosen.
#[derive(Default)]
pub struct BagMenu {
    open: bool,
    cursor: ListCursor,
    actions: Option<ListCursor>,
    /// What was asked was refused a moment ago: the log says why.
    told: u32,
}

impl BagMenu {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// What was asked was done.
    pub fn close(&mut self) {
        *self = BagMenu::default();
    }
}

/// Opens the bag, if it can be.
pub fn open(game: &mut Turn<'_, Dive>) {
    if game.run.bag_opens() && !game.run.bag().items.is_empty() {
        game.here.bag = BagMenu { open: true, ..BagMenu::default() };
    }
}

pub fn read(game: &mut Turn<'_, Dive>, keys: &Keys) -> Input {
    let (run, dive) = (&mut *game.run, &mut *game.here);
    let menu = &mut dive.bag;
    let count = run.bag().items.len();
    if count == 0 || keys.named.iter().any(|key| key == "i") {
        menu.close();
        return Input::Menu;
    }
    menu.told = menu.told.saturating_sub(1);
    menu.cursor.within(count);
    if let Some(action) = &mut menu.actions {
        let actions = run.item_actions(menu.cursor.at);
        if keys.escape {
            menu.actions = None;
            return Input::Menu;
        }
        let Picked::Chosen(chosen) = action.keys(keys, actions.len()) else {
            return Input::Menu;
        };
        let Some(&chosen) = actions.get(chosen) else {
            return Input::Menu;
        };
        let item = menu.cursor.at;
        menu.actions = None;
        menu.told = TOLD;
        return match chosen {
            ItemAction::Use => Input::UseItem(item),
            ItemAction::Throw => Input::ThrowItem(item),
            ItemAction::Give(to) => Input::Give { item, to },
            ItemAction::PutDown => Input::PutDown(item),
        };
    }
    if keys.escape {
        menu.close();
        return Input::Menu;
    }
    if let Picked::Chosen(_) = menu.cursor.keys(keys, count) {
        menu.actions = Some(ListCursor::default());
    }
    Input::Menu
}

fn action_name(run: &Run, action: ItemAction) -> String {
    match action {
        ItemAction::Use => "Use".to_string(),
        ItemAction::Throw => "Throw".to_string(),
        ItemAction::PutDown => "Put Down".to_string(),
        ItemAction::Give(to) => {
            let who = run.dungeon().and_then(|dungeon| dungeon.monster(to)).map_or("?", |monster| monster.name.as_str());
            format!("Give to {who}")
        }
    }
}

pub fn draw(canvas: &mut Canvas, ui: &UiArt, run: &Run, menu: &BagMenu) {
    if !menu.open {
        return;
    }
    let book = &run.book;
    let items = &run.bag().items;
    let mut cursor = menu.cursor;
    cursor.within(items.len());
    let screen = SCREEN_HEIGHT as i32;
    let (left, top) = ((2 * TILE) as i32, screen + (2 * TILE) as i32);
    canvas.window(ui, 2, BOTTOM + 2, 18, 17);
    canvas.header(ui, left, top, 18 * TILE, (&ui.strings.bag_title, color::WHITE));
    let first = cursor.first(SHOWN);
    let names: Vec<String> = items.iter().skip(first).take(SHOWN).map(|&item| book.item_name(item)).collect();
    let rows: Vec<(&str, [u8; 3])> = names.iter().map(|name| (name.as_str(), color::ITEM)).collect();
    let on = menu.actions.is_none().then_some(cursor.at - first);
    canvas.rows(ui, Rows { left, top: top + 16, pitch: 13, indent: 20 }, &rows, on);
    canvas.foot(ui, left, top + 120, 18 * TILE - 2, true);
    // What the selected item does; or why that could not be done.
    canvas.window(ui, 2, BOTTOM + 20, 26, 2);
    let line = match menu.told > 0 {
        true => run.log.lines.last().cloned(),
        false => items
            .get(cursor.at)
            .and_then(|&item| book.tables.item_descriptions.get(stack::id(item) as usize))
            .map(|text| text.lines().next().unwrap_or("").to_string()),
    };
    if let Some(line) = line {
        canvas.markup(ui, left + 2, screen + 162, 26 * TILE - 4, &line);
    }
    if let Some(action) = menu.actions {
        let actions = run.item_actions(cursor.at);
        canvas.window(ui, 13, BOTTOM + 5, 17, actions.len() * 2);
        let names: Vec<String> = actions.iter().map(|&shown| action_name(run, shown)).collect();
        let rows: Vec<(&str, [u8; 3])> = names.iter().map(|name| (name.as_str(), color::WHITE)).collect();
        canvas.rows(ui, Rows { left: (13 * TILE) as i32, top: screen + (5 * TILE) as i32 + 2, pitch: 16, indent: 14 }, &rows, Some(action.at));
    }
}
