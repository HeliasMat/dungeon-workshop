//! A panic in the Workshop is a bug, never what a film or a game is told by:
//! what they are given is checked and told as diagnostics. Should one happen
//! all the same, the one net of the program, set first thing in `main`, tells
//! it as a bug, which thread, where and why, before the usual report.

/// The Workshop's panics told as bugs on stderr, then as Rust tells them.
pub fn told_as_bugs() {
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("a worker").to_string();
        let at = info.location().map(|at| at.to_string()).unwrap_or_default();
        eprintln!("Dungeon Workshop: a bug (in {thread}, at {at}); please report it with what you were doing.");
        before(info);
    }));
}
