//! The numbers the engine keeps for scenes, by the name scenes read them.

/// The number the last answer chosen is kept in (0 for the first).
pub(crate) const ANSWER: &str = "answer";

/// The numbers of a question given its time (see `PlayBeat::AskTimer`): the
/// seconds left to answer it; whether the time ran out (1) or the player
/// answered (0).
pub(crate) const TIME_LEFT: &str = "time";

/// The seconds a countdown has left (see `DataBeat::Timer`).
pub(crate) const TIMER: &str = "timer";

/// What Enter does while the player walks someone about (see
/// [`Director::press`](crate::Director::press)): the presses counted; whether someone is close
/// enough to talk to, and who (as a number: `if target == eevee`); how
/// close that is, in pixels.
pub(crate) const PRESSES: &str = "presses";

pub(crate) const TALKS: &str = "talking";

pub(crate) const TARGET: &str = "target";

pub(super) const REACH: f32 = 40.0;

/// The frames gone since the scene at hand opened, and when Enter was
/// last pressed by that clock (see [`Director::press`](crate::Director::press)): a game of keys
/// in time.
pub(crate) const CLOCK: &str = "clock";

/// The frames the music playing now has played (see `MusicClock`): what
/// `at music 12s` and `wait music 12s` wait for.
pub const MUSIC_CLOCK: &str = "music_clock";

pub(crate) const PRESSED_AT: &str = "press_clock";

/// The arrows pressed (see [`Director::arrow`](crate::Director::arrow)): how many, the last one
/// (1 up, 2 down, 3 left, 4 right) and when.
pub(crate) const ARROWS: &str = "arrows";

pub(crate) const ARROW: &str = "arrow";

pub(crate) const ARROW_AT: &str = "arrow_clock";

/// Who the player walks about now (see `PlayBeat::ControlTeam`), as a number:
/// `if controlled == partner`.
pub(crate) const LED: &str = "controlled";

/// How many rounds a loop goes through at most while the scene is played
/// at once up to a label (`skipto`, `DUNGEON_FROM`, a checkpoint): one
/// waiting for the player would go round for ever.
pub(super) const HURRIED_ROUNDS: u32 = 64;

/// How the label of a place in an event starts (see `keep_place`): a save
/// made there plays the event at once up to it.
pub(super) const PLACE_LABEL: char = '@';

pub(super) fn place_label(at: super::Mark) -> String {
    format!("{PLACE_LABEL}{}.{}", at.scene, at.beat)
}

pub(crate) const TOO_LATE: &str = "too_late";
