//! What is wrong with a text: where, why, and how to fix it, as data (the
//! one shape every block says it in, `diagnostics`), and the language's own
//! codes.
//!
//! A fix is a list of edits of the text, never found again by reading the
//! message: an editor or an agent applies it as it is.

pub use diagnostics::{Diagnostic, Edit, Fix, Location, Severity, applied};

/// The codes, each with what it means.
pub mod codes {
    /// A line read as nothing the language has.
    pub const UNREADABLE: &str = "E0001";
    /// A block opened and never closed by its `end`.
    pub const UNCLOSED: &str = "E0002";
    /// An `end` with no block to close.
    pub const STRAY_END: &str = "E0003";
    /// An `else` or an `option` outside its block.
    pub const STRAY_MIDDLE: &str = "E0004";
    /// A calculation that does not read.
    pub const EXPRESSION: &str = "E0005";
    /// A `{` or a `"` never closed.
    pub const UNCLOSED_MARK: &str = "E0006";
    /// The form of a word or a recipe that does not read.
    pub const FORM: &str = "E0007";
    /// A way of writing the language had before: the fix gives today's.
    pub const OLD_FORM: &str = "E0008";
    /// A name where one is needed.
    pub const NAME: &str = "E0009";
    /// A block that takes nothing after its word, given something.
    pub const EXTRA: &str = "E0010";
}
