//! The interface's words, in the game's language: data the rules tell
//! their log with, and whoever draws shows.

/// The interface's texts, in the game's language.
#[derive(Default)]
pub struct UiStrings {
    /// The block of dungeon menu texts, see [`text`].
    pub menu: Vec<String>,
    /// Weather names, by weather id.
    pub weather: Vec<String>,
    pub bag_title: String,
    /// "`[string:0]`'s moves".
    pub moves_title: String,
    /// The controls shown on the top screen, with the game's markup.
    pub controls: String,
    /// The two lines under the moves list, with markup.
    pub moves_hint: String,
    /// "There is nothing at `[string:0]`'s feet."
    pub nothing_here: String,
    pub leader: String,
    /// The team's name: the one the game proposes, until teams can be
    /// named.
    pub team_name: String,
    /// The controls of menus and of the overworld, with markup.
    pub menu_controls: String,
    pub town_controls: String,
    /// The dungeon's messages, by [`log_text`] (empty where not found).
    pub log: Vec<String>,
    /// The hints of dungeons, then of the town: eight titles, then their
    /// eight texts, each (empty where not found).
    pub hints: Vec<String>,
}

/// The eight hints of dungeons (`UiStrings::hints`), then of the town.
pub const HINTS: usize = 8;

/// The dungeon messages taken from the game's text, each found by its
/// English wording.
pub mod log_text {
    pub(crate) const USED_MOVE: usize = 0;
    pub(crate) const TOOK_DAMAGE: usize = 1;
    pub(crate) const MISSED: usize = 2;
    pub(crate) const FAINTED: usize = 3;
    pub(crate) const GREW_LEVEL: usize = 4;
    pub(crate) const LEARNED: usize = 5;
    pub(crate) const SUPER_EFFECTIVE: usize = 6;
    pub(crate) const NOT_VERY_EFFECTIVE: usize = 7;
    pub(crate) const CRITICAL: usize = 8;
    pub(crate) const PICKED_UP: usize = 9;
    pub(crate) const STAT_ROSE: usize = 10;
    pub(crate) const STAT_FELL: usize = 11;
    pub(crate) const STAT_SHARPLY_ROSE: usize = 12;
    pub(crate) const STAT_HARSHLY_FELL: usize = 13;
    pub(crate) const POISONED: usize = 14;
    pub(crate) const BADLY_POISONED: usize = 15;
    pub(crate) const PARALYZED: usize = 16;
    pub(crate) const CONFUSED: usize = 17;
    pub(crate) const FELL_ASLEEP: usize = 18;
    pub(crate) const FROZEN: usize = 19;
    pub(crate) const CRINGED: usize = 20;
    pub(crate) const WOKE_UP: usize = 21;
    pub(crate) const GAINED_EXP: usize = 22;
    /// After the messages, the names of the stats as they put them:
    /// accuracy, evasiveness, defense, special defense, attack, special
    /// attack (found by the first two).
    pub const STATS: usize = ENGLISH.len();
    pub(crate) const ACCURACY: usize = STATS;
    pub(crate) const EVASION: usize = STATS + 1;
    pub(crate) const DEFENSE: usize = STATS + 2;
    pub(crate) const SP_DEFENSE: usize = STATS + 3;
    pub const ATTACK: usize = STATS + 4;
    pub(crate) const SP_ATTACK: usize = STATS + 5;
    pub const ENGLISH: [&str; 23] = [
        "[string:0] used [move:0]!",
        "[string:1] took [digits_c:0] damage!",
        "The move missed [string:1]!",
        "[string:1] fainted!",
        "[string:0] grew to Level [digits_c:0]!",
        "[string:0] learned [move:0]!",
        "It's super effective!",
        "It's not very effective...",
        "A critical hit!",
        "[string:0] picked up [item:0].",
        "[string:0]'s [string:1] rose!",
        "[string:0]'s [string:1] fell!",
        "[string:0]'s [string:1] sharply rose!",
        "[string:0]'s [string:1] harshly fell!",
        "[string:0] was poisoned!",
        "[string:0] was badly poisoned!",
        "[string:0] is paralyzed!",
        "[string:0] became confused!",
        "[string:0] fell asleep!",
        "[string:0] is frozen solid!",
        "[string:0] cringed!",
        "[string:0] woke up!",
        "[string:0] gained [CS:E][digits_c:0][CR] Exp. Points!",
    ];
}

/// Indices in [`UiStrings::menu`]: a block of consecutive game strings.
pub mod text {
    /// The seven entries of the main menu are not consecutive in the game's
    /// text: moves, items, team, others, ground, rest, exit.
    pub const MAIN: [usize; 7] = [0, 1, 2, 3, 4, 6, 7];
    pub const BELLY: usize = 5;
    pub const MONEY: usize = 8;
    pub const WEATHER: usize = 9;
    pub const PLAY_TIME: usize = 10;
    pub const OTHERS_TITLE: usize = 12;
    /// Options, window, map, message log, objectives, recruitment, help.
    pub const OTHERS: std::ops::Range<usize> = 13..20;
    pub const REST_TITLE: usize = 21;
    /// Suspend, give up.
    pub const REST: std::ops::Range<usize> = 22..24;
    pub const COUNT: usize = 24;
}
