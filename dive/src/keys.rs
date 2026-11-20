//! What the player's keys ask this frame. A question or a menu open takes
//! them; otherwise they move the leader.

use rules::run::{Input, Run};
use shell::Keys;

use super::{bag, pause, talk};
use crate::Dive;
use crate::Turn;

/// The arrows held, or the letters that go with them.
fn way(keys: &Keys) -> (i32, i32) {
    let held = |name: &str| keys.held.iter().any(|key| key == name);
    let x = keys.toward.x as i32 + held("d") as i32 - held("a") as i32;
    let y = keys.toward.y as i32 + held("s") as i32 - held("w") as i32;
    (x.signum(), y.signum())
}

fn held(keys: &Keys, name: &str) -> bool {
    keys.held.iter().any(|key| key == name)
}

fn named(keys: &Keys, name: &str) -> bool {
    keys.named.iter().any(|key| key == name)
}

/// Turning in place: Y held (as the games of the kind have it), or Shift.
fn turning(keys: &Keys) -> bool {
    keys.run || held(keys, "y")
}

/// The leader: arrows walk (with Y or Shift held they only turn it), Space is the
/// regular attack, 1 to 4 the moves, R passes the turn. All of them held
/// go on; `Run` drops what comes too soon.
fn leader(keys: &Keys) -> Input {
    if let Some(slot) = ["1", "2", "3", "4"].iter().position(|key| held(keys, key)) {
        return Input::UseMove(slot);
    }
    if keys.hurry {
        return Input::Attack;
    }
    let toward = way(keys);
    if toward != (0, 0) {
        return if turning(keys) { Input::Face(toward) } else { Input::Go { toward } };
    }
    if held(keys, "r") {
        return Input::Wait;
    }
    Input::Idle
}

fn quickload(run: &mut Run, path: &str) -> Input {
    match std::fs::read_to_string(path) {
        Ok(text) => Input::Quickload(text),
        Err(_) => {
            run.say("No quicksave.".to_string());
            Input::Idle
        }
    }
}

pub fn read(game: &mut Turn<'_, Dive>, keys: &Keys) -> Input {
    let in_menu = menus(game, keys);
    if let Some(input) = in_menu {
        game.here.let_go = true;
        return input;
    }
    game.here.turning = turning(keys);
    // (The key that closed a menu does not act on the floor as well.)
    let dive = &mut *game.here;
    dive.let_go = dive.let_go && (keys.hurry || held(keys, "1") || held(keys, "2") || held(keys, "3") || held(keys, "4"));
    if dive.let_go {
        return Input::Idle;
    }
    if keys.escape {
        dive.pause.open();
        return Input::Menu;
    }
    if named(keys, "i") {
        bag::open(game);
        return Input::Menu;
    }
    if named(keys, "f5") {
        return Input::Quicksave;
    }
    if named(keys, "f9") {
        return quickload(game.run, &game.show.quicksave_path());
    }
    if named(keys, "f10") {
        return Input::GiveUp;
    }
    // (Tab: the next of the team is the one moved, where the rules let it.)
    if keys.tab && game.run.dungeon().is_some_and(|dungeon| dungeon.rules.options.team_control) {
        return Input::SwitchControl;
    }
    leader(keys)
}

/// What a question or an open menu asks; none when there is none.
fn menus(game: &mut Turn<'_, Dive>, keys: &Keys) -> Option<Input> {
    let dungeon = game.run.dungeon()?;
    if !dungeon.learning.is_empty() {
        return Some(talk::learn(game, keys));
    }
    if dungeon.ask.is_some() {
        return Some(talk::ask(game, keys));
    }
    if let Some(talking) = dungeon.shop.as_ref().and_then(|shop| shop.talk) {
        return Some(talk::shop(game, keys, talking));
    }
    game.here.talk.done();
    if game.here.bag.is_open() {
        return Some(bag::read(game, keys));
    }
    if game.here.pause.is_open() {
        return Some(pause::read(game, keys));
    }
    None
}
