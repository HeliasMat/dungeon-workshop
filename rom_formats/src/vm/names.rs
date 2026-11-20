//! The numbers scripts go by.

/// Variables of the game (pmdsky-debug's `script_var_id`): the story's
/// progress; the dungeon chosen and the one entered; how the dungeon left
/// ended, twice (`DUNGEON_CLEARED` or anything else).
pub const SCENARIO_MAIN: u16 = 3;

/// The level the leader is in, and the one it came from.
pub const GROUND_ENTER: u16 = 28;

/// Which special episode is played; [`NO_EPISODE`] in the main story.
pub const EXECUTE_SPECIAL_EPISODE_TYPE: u16 = 73;

pub const NO_EPISODE: i32 = -1;

/// The level of `supervision_ExecuteEnter` standing for "this one".
pub(super) const HERE: u16 = 0x7FFF;

/// Name of a level's own scene: the script entering it for free roaming
/// (`enter00.ssb`) and the characters and areas it places (`enter.sse`).
pub const ENTER: &str = "enter";

pub const DUNGEON_SELECT: u16 = 38;

/// The `message_Menu`s asking for a name: the hero's, the partner's, the
/// team's.
/// How much of the world map the clouds have left (1 to 8).
pub const WORLD_MAP_LEVEL: u16 = 87;

/// Places of the world map scripts name besides the table's: where the
/// team is, and the dungeon chosen.
pub const MARKER_HERE: u16 = 1000;

pub const MARKER_CHOSEN: u16 = 1001;

pub(crate) const DUNGEON_ENTER: u16 = 39;

pub(super) const DUNGEON_ENTER_INDEX: u16 = 41;

#[cfg(test)]
pub(crate) const DUNGEON_RESULT: u16 = 43;

#[cfg(test)]
pub(crate) const GROUND_START_MODE: u16 = 44;

#[cfg(test)]
pub(crate) const DUNGEON_CLEARED: i32 = 8;

/// The dungeon of `main_EnterDungeon` standing for "the one chosen".
pub(super) const DUNGEON_CHOSEN: u16 = 0x7FFF;

/// The special process asking whether a job was done in the dungeon left
/// (pmdsky-debug's `SPECIAL_PROC_DUNGEON_HAD_REQUEST_DONE`), the variable
/// this engine keeps the answer in, and the game's count of the jobs done
/// in the day, which the story waits on.
pub(super) const JOB_DONE: u16 = 7;

pub(super) const TEAM_IS_THE_TWO: u16 = 47;

pub(super) const JUMP_TO_TITLE: u16 = 25;

pub(super) const SCENARIO_MAIN_BIT_FLAG: u16 = 17;

pub(super) const SAVING: u16 = 6;

pub(super) const SAVE_ASKED: u16 = 7;

pub(crate) const JOB_DONE_THERE: u16 = 0xFFFC;

pub const REQUEST_CLEAR_COUNT: u16 = 51;

/// The mode of the dungeon entered: how the story has it at that point.
pub(crate) const DUNGEON_ENTER_MODE: u16 = 40;

/// Variable id standing for the modes of the dungeons, by dungeon.
pub(crate) const DUNGEON_MODES: u16 = 0xFFFD;

/// Variable id standing for which variant ("sector") of itself the scene
/// last started is to play: scenes ask (`sector()`) and end differently.
// Known gap: one for all scenes: a scene that asks after one it played inside
// itself would read that one's.
pub(crate) const SECTOR: u16 = 0xFFFB;

/// Dungeons of the game.
pub(super) const DUNGEONS: u16 = 180;

/// The shared routine entering the dungeon chosen at the crossroads.
pub(super) const ENTER_FREE_DUNGEON: usize = 670;

/// The one entering a dungeon the story has opened and not finished with:
/// the story's scenes at its entrance start from there.
// (Taken from the dungeon's mode, as `getout_routine` is, not from the
// game's code; springs go by `ENTER_REQUEST_DUNGEON`.)
pub(super) const ENTER_SCENARIO_DUNGEON: usize = 666;

pub(super) const ENTER_REQUEST_DUNGEON: usize = 667;

pub(super) const MODE_OPEN: i32 = 1;

/// Places chosen like dungeons that are none (a village, a shelter, the
/// springs): the first two are entered by the story's routine, the springs
/// by the jobs'.
pub(crate) const PLACES: std::ops::RangeInclusive<u16> = 175..=178;

pub(super) const SPRINGS: std::ops::RangeInclusive<i32> = 177..=178;

/// The number the story gives the Hot Spring while it sends the team
/// there (chapter 22): no dungeon either; the routine for leaving a
/// dungeon of the story is what takes the team to it.
pub(crate) const WAY_TO_SPRING: u16 = 212;

/// What `main_EnterAdventure` is asked instead of a dungeon: to let the
/// player choose one, or to leave for the one chosen.
pub(super) const CHOOSE_DUNGEON: u16 = 0x7FFF;

pub(super) const LEAVE: u16 = 250;

pub(super) const LEAVE_TOO: u16 = 251;

/// Modes of a dungeon the story has finished with: open for jobs, or for
/// jobs and visits.
#[cfg(test)]
pub(super) const MODE_REQUEST: i32 = 2;

#[cfg(test)]
pub(super) const MODE_OPEN_AND_REQUEST: i32 = 3;

/// Shared routines the game itself runs: to play the story from where its
/// progress stands, and when a dungeon is left, one of the story's or one
/// gone back to.
#[cfg(test)]
pub(crate) const EVENT_DIVIDE: usize = 60;

pub(crate) const GETOUT_SCENARIO_DUNGEON: usize = 672;

#[cfg(test)]
pub(crate) const GETOUT_REQUEST_DUNGEON: usize = 673;

/// The routine the game runs of its own as the team leaves for a
/// dungeon: the way there over the world map.
pub const MOVE_WORLD_MAP: usize = 678;

/// Variable id standing for the list of "performance" flags.
pub(super) const PERFORMANCE: u16 = 0xFFFE;
