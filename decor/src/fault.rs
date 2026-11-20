//! What goes wrong, and where: a file and its line, kept apart from what is
//! said, for who shows it on that line (the Workshop) as for who prints it:
//! the one shape every block says it in (`diagnostics`). A fault is a
//! [`Diagnostic`] (an error), a remark of a place built one too (a remark),
//! a line of a file a [`Location`] (`Location::in_file`, line 0: the file
//! as a whole).

pub use diagnostics::{Diagnostic, Location};
