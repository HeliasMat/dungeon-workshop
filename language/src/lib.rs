//! The one language of the scenes (`LANGUAGE.md`), read once.
//!
//! - `span`: where a piece was written (bytes, lines, columns);
//! - `diag`: what is wrong, where, and its fixes as edits;
//! - `lexer`, `expr`: the calculations;
//! - `line`: what each line is (a said line, words, a block's opening…);
//! - `parser`: a whole text, its blocks from their opening to their `end`;
//! - `ast`: the tree made;
//! - `format`: the text written back in order.
//!
//! Nothing here knows the game: names are only names, an argument only what
//! was written. The vocabulary and the meaning come after.

#![forbid(unsafe_code)]

pub mod ast;
pub mod diag;
pub mod expr;
pub mod format;
pub mod lexer;
pub mod line;
pub mod parser;
pub mod span;

#[cfg(test)]
mod tests;

pub use diag::{Diagnostic, Edit, Fix, Location, Severity};
pub use diagnostics::text;
pub use parser::{Parsed, parse, parse_with};
pub use span::{Position, Source, Span};

/// The text written back in order (`format`), whatever is wrong with it.
pub fn formatted(text: &str) -> String {
    format::formatted(&parse(text).file)
}

/// The same, the words of a vocabulary that hold lines told by `holds`.
pub fn formatted_with(text: &str, holds: &dyn Fn(&ast::Call) -> bool) -> String {
    format::formatted(&parse_with(text, holds).file)
}
