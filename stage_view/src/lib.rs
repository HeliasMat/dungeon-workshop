//! All that is drawn, in one place: a scene on screen (`stage`), the game's
//! interface (`ui`), what a scene shows over itself (`overlay`). `shell` is
//! what it draws on; the rules are `rules`'s.

/// What a scene shows over itself, drawn on a canvas.
pub mod overlay;
pub mod stage;
/// Drawing the interface (windows, text, menus).
pub mod ui;
