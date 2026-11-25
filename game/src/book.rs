//! The rules' book (`dungeons.bin`), as the game, its dungeons and its
//! checks read it from the folder of the game's files.

use rules::run::Book;

/// The game's book of rules, read from the folder of its files.
pub fn read(data: &str) -> Result<Book, crate::Error> {
    let bytes = shell::data::read(data, "dungeons.bin").ok_or(crate::Error::NoBook)?;
    let mut book = Book::read(&bytes).ok_or(crate::Error::BookUnread)?;
    book.strings = stage_view::ui::UiArt::load(data).map(|ui| ui.strings.clone());
    Ok(book)
}
