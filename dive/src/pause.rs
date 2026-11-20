//! The dungeon's own menu (moves, items, team, others, ground, rest) while
//! the team explores: what the keys do on its pages. (The entries are the
//! original's. Of "Others": the options, the window's frame, the map shown
//! or not, the message log, the jobs taken, who may be recruited, the
//! dungeon's hints; of "Rest": the quicksave, giving up, the quicksave and
//! back to the title.)

use rules::dungeon::MoveSlot;
use rules::run::options::OPTION_ROWS;
use rules::run::{Input, Monster, Run, Side, tactic};
use shell::Keys;
use stage_view::ui::{HINTS, ListCursor};

use super::{bag, sights};
use crate::Dive;
use crate::Turn;

const MAIN_ROWS: usize = 7;
const OTHERS_ROWS: usize = 7;
const REST_ROWS: usize = 3;
/// Options (of "Others"): the type of window's row.
const WINDOW_ROW: usize = 3;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Closed,
    Main,
    Moves,
    Team,
    Others,
    /// Of that member of the team (1 the first after the leader).
    Tactics(usize),
    /// The IQ skills of that member (0 the leader), switched on or off.
    Iq(usize),
    Log,
    Options,
    Objectives,
    Recruitable,
    Hints,
    Hint(usize),
    Rest,
}

#[derive(Default)]
pub struct Pause {
    pub page: Page,
    pub cursor: ListCursor,
    /// Whose moves are shown: 0 the leader.
    pub member: usize,
    /// Quicksaved: the game goes back to the title.
    pub leaving: bool,
}

/// The leader, then the rest of the team.
pub fn allies(run: &Run) -> Vec<&Monster> {
    run.dungeon().map_or(Vec::new(), |dungeon| dungeon.monsters.iter().filter(|monster| monster.side == Side::Ally).collect())
}

impl Pause {
    pub fn is_open(&self) -> bool {
        self.page != Page::Closed
    }

    pub fn open(&mut self) {
        self.turn_to(Page::Main);
    }

    fn turn_to(&mut self, page: Page) {
        *self = Pause { page, cursor: ListCursor::default(), member: 0, leaving: false };
    }

    fn at(&mut self, page: Page, cursor: usize) {
        *self = Pause { page, cursor: ListCursor::at(cursor), member: 0, leaving: false };
    }
}

/// How many rows a page has.
fn rows(pause: &Pause, run: &Run) -> usize {
    let team = allies(run);
    match pause.page {
        Page::Moves => shown_moves(pause, run).len().max(1),
        Page::Team => team.len() + 1,
        Page::Tactics(member) => tactic::unlocked(team.get(member - 1).map_or(1, |ally| ally.stats.level)).len(),
        Page::Iq(member) => iq_skills(run, member).len().max(1),
        Page::Others => OTHERS_ROWS,
        Page::Log => run.log.lines.len().max(1),
        Page::Rest => REST_ROWS,
        Page::Options => OPTION_ROWS,
        Page::Objectives | Page::Recruitable | Page::Hint(_) => 1,
        Page::Hints => HINTS,
        _ => MAIN_ROWS,
    }
}

/// A member of the team by its row on the Team page (0 the leader).
pub fn member_of(run: &Run, member: usize) -> Option<&Monster> {
    match member {
        0 => run.dungeon().map(|dungeon| dungeon.leader()),
        member => allies(run).get(member - 1).copied(),
    }
}

/// The IQ skills a member has learnt, by number.
pub fn iq_skills(run: &Run, member: usize) -> Vec<u8> {
    let learnt = member_of(run, member).map_or(0, |monster| run.iq_learnt(monster));
    (0..128u8).filter(|&skill| learnt & (1u128 << skill) != 0).collect()
}

/// The monster whose moves are shown, and they.
pub fn shown_monster<'a>(pause: &Pause, run: &'a Run) -> Option<&'a Monster> {
    let dungeon = run.dungeon()?;
    pause.member.checked_sub(1).and_then(|at| allies(run).get(at).copied()).or(Some(dungeon.leader()))
}

fn shown_moves<'a>(pause: &Pause, run: &'a Run) -> &'a [MoveSlot] {
    shown_monster(pause, run).map_or(&[], |monster| &monster.moves)
}

pub fn read(game: &mut Turn<'_, Dive>, keys: &Keys) -> Input {
    let row_count = rows(&game.here.pause, game.run);
    let (run, dive) = (&mut *game.run, &mut *game.here);
    let pause = &mut dive.pause;
    if keys.escape {
        let back = if pause.page == Page::Main { Page::Closed } else { Page::Main };
        pause.turn_to(back);
        return Input::Menu;
    }
    pause.cursor.arrows(keys, row_count);
    // Left and Right change a setting.
    if pause.page == Page::Options && (keys.left || keys.right) {
        change_option(game, keys.right);
        return Input::Menu;
    }
    // Left and Right: the moves of the next member of the team.
    let count = allies(run).len() + 1;
    if pause.page == Page::Moves && count > 1 && (keys.left || keys.right) {
        let turn = if keys.right { 1 } else { count - 1 };
        (pause.member, pause.cursor) = ((pause.member.min(count - 1) + turn) % count, ListCursor::default());
        return Input::Menu;
    }
    // L on a move links it to the one before, or delinks it, with a Link
    // Box in the bag (as the original's).
    if pause.page == Page::Moves && keys.named.iter().any(|key| key == "l") {
        return shown_monster(pause, run).map_or(Input::Menu, |monster| Input::Link { who: monster.id, slot: pause.cursor.at });
    }
    // I on a member: its IQ skills.
    if pause.page == Page::Team && keys.named.iter().any(|key| key == "i") {
        let member = pause.cursor.at;
        pause.turn_to(Page::Iq(member));
        return Input::Menu;
    }
    if !keys.enter {
        return Input::Menu;
    }
    match (pause.page, pause.cursor.at) {
        (Page::Main, 0) => pause.turn_to(Page::Moves),
        (Page::Main, 1) => {
            pause.turn_to(Page::Closed);
            bag::open(game);
        }
        (Page::Main, 2) => pause.turn_to(Page::Team),
        (Page::Main, 3) => pause.turn_to(Page::Others),
        (Page::Main, 4) => {
            let leader = run.dungeon().map_or("", |dungeon| dungeon.leader().name.as_str());
            let nothing = match &run.book.strings {
                Some(strings) => strings.nothing_here.replace("[string:0]", leader),
                None => "There's nothing here.".to_string(),
            };
            run.say(nothing);
            pause.turn_to(Page::Closed);
        }
        (Page::Main, 5) => pause.turn_to(Page::Rest),
        (Page::Main, _) => pause.turn_to(Page::Closed),
        (Page::Rest, 2) => {
            pause.turn_to(Page::Closed);
            pause.leaving = true;
            return Input::Quicksave;
        }
        (Page::Rest, 1) => {
            pause.turn_to(Page::Closed);
            return Input::GiveUp;
        }
        // "Quicksave".
        (Page::Rest, _) => {
            pause.turn_to(Page::Closed);
            return Input::Quicksave;
        }
        // A member of the team, then its tactic (the leader has none).
        (Page::Team, 0) => {}
        (Page::Team, member) => {
            let team = allies(run);
            let (current, level) =
                team.get(member - 1).map_or((0, 1), |ally| (run.team().progress.tactics.get(&ally.name).copied().unwrap_or(0), ally.stats.level));
            let cursor = tactic::unlocked(level).iter().position(|&tactic| tactic == current).unwrap_or(0);
            pause.at(Page::Tactics(member), cursor);
        }
        (Page::Tactics(member), chosen) => {
            let asked =
                allies(run).get(member - 1).and_then(|ally| Some(Input::SetTactic { who: ally.id, tactic: *tactic::unlocked(ally.stats.level).get(chosen)? }));
            pause.turn_to(Page::Team);
            return asked.unwrap_or(Input::Menu);
        }
        (Page::Iq(member), row) => {
            let who = member_of(run, member).map(|monster| monster.id);
            if let (Some(who), Some(&skill)) = (who, iq_skills(run, member).get(row)) {
                return Input::ToggleIq { who, skill };
            }
        }
        (Page::Others, 0) => pause.turn_to(Page::Options),
        // "Window": the settings, on the frames' type.
        (Page::Others, 1) => pause.at(Page::Options, WINDOW_ROW),
        (Page::Others, 2) => {
            let shown = run.toggle_map();
            run.say(if shown { "The map is shown." } else { "The map is hidden." }.to_string());
            pause.turn_to(Page::Closed);
        }
        (Page::Others, 3) => pause.turn_to(Page::Log),
        (Page::Others, 4) => pause.turn_to(Page::Objectives),
        (Page::Others, 5) => pause.turn_to(Page::Recruitable),
        (Page::Others, _) => pause.turn_to(Page::Hints),
        (Page::Hints, hint) => pause.turn_to(Page::Hint(hint)),
        (Page::Hint(hint), _) => pause.at(Page::Hints, hint),
        (Page::Log | Page::Objectives | Page::Recruitable, _) => pause.turn_to(Page::Others),
        (Page::Options, _) => change_option(game, true),
        // (Only the leader's moves are used from here.)
        (Page::Moves, slot) => {
            if pause.member == 0 && run.dungeon().is_some_and(|dungeon| dungeon.leader().moves.get(slot).is_some_and(MoveSlot::usable)) {
                pause.turn_to(Page::Closed);
                return Input::QueueMove(slot);
            }
        }
        _ => pause.turn_to(Page::Main),
    }
    Input::Menu
}

/// The setting under the cursor changed, and kept; the music follows.
fn change_option(game: &mut Turn<'_, Dive>, more: bool) {
    let row = game.here.pause.cursor.at;
    game.run.change_option(row, more);
    game.run.options().save();
    if row == 0 {
        sights::play_song(game);
    }
}
