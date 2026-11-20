//! Time in a film, one model. A cursor goes on; each line starts and ends,
//! and is placed one of four ways:
//!
//! - after (`line`): at the end of the line before, the cursor waits for
//!   its end;
//! - with (`line &`, `& as x`): at the same instant, the cursor does not
//!   wait;
//! - at a time (`at 12s :`, `at music 12s :`, `at m + 2s :`, `at +2s :`): the
//!   clock and so long after, beside the cursor;
//! - at an event (`when a arrives :`, `when a leaves :`, `when <test> :`):
//!   when it happens, beside the cursor.
//!
//! `wait` is the only way back to the cursor. Each word says its time in
//! the registry ([`WordTime`]). The lowering makes, besides the beats, the
//! film's [`Plan`]: a line each, with its thread, its placing and how long
//! it lasts, known or until a handle. The engine tells when each really
//! starts and ends (`FlowBeat::Line`).

pub mod plan;
pub mod replace;

pub use plan::*;
pub use replace::{Change, played_words, replace, replaced_line, seconds_said};

/// How long a word's line lasts, as the registry says (the "Time" column
/// of FILMS.md): the one rule of who waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordTime {
    /// Its `d`: the film waits so long, unless `&`.
    Lasts,
    /// Until an event (who walks gets there, a sound ends): the film waits
    /// for it, unless `&`.
    Until,
    /// No time of the film's: the film goes on at once (what it starts may
    /// last, beside it).
    Instant,
    /// The word's own play says, by the beats it writes (a walk waits for
    /// who walks, a line said for its reading).
    ByLine,
}

impl WordTime {
    /// As FILMS.md says it.
    pub fn said(self) -> &'static str {
        match self {
            WordTime::Lasts => "lasts",
            WordTime::Until => "until",
            WordTime::Instant => "instant",
            WordTime::ByLine => "by the line",
        }
    }
}

#[cfg(test)]
mod tests;
