//! The dungeon's top line: the floor, the level, the health and its gauge,
//! kept up to date as the team is hurt and healed.

use super::*;
use engine::GaugeOf;

/// Where the top line is kept on screen (`hud left`).
const LEFT: usize = 0;

/// A floor as told on the black: `B3F`, or `3F` going up.
pub(in crate::verbs) fn floor_said(film: &Film, floor: i32) -> String {
    match film.dungeon.going_up {
        false => format!("B{floor}F"),
        true => format!("{floor}F"),
    }
}

/// The floor in the top line: `B3F`, or `3F` going up.
pub(super) fn floor_short(film: &Film) -> String {
    format!("{}{}F", if film.dungeon.going_up { "" } else { "B" }, film.dungeon.floor)
}

/// The number that holds someone's health, for the top line and its gauge.
pub(super) fn health(who: &str) -> String {
    format!("hp_{who}")
}

/// The top line written again, as it is now.
pub(super) fn shown_top(film: &mut Film) {
    let Some((who, level, most)) = film.dungeon.dungeon_hud.clone() else {
        return;
    };
    let var = health(&who);
    let text = format!("{}  Lv.{level}  HP {{{var}}}/{most}", floor_short(film));
    film.beat(emit::hud(LEFT, Some(text)));
    film.beat(emit::gauge(LEFT, Some(GaugeOf { var, most: emit::number(most), colour: emit::colour(engine::Colour::Green) })));
}

/// The top line and its gauge taken away.
pub(super) fn top_line_off(film: &mut Film) {
    film.beat(emit::hud(LEFT, None));
    film.beat(emit::gauge(LEFT, None));
}

/// Whether the top line follows him.
pub(super) fn followed(film: &Film, who: &str) -> bool {
    film.dungeon.dungeon_hud.as_ref().is_some_and(|(followed, ..)| followed == who)
}

/// What happens to the top line's health when he is hurt or healed.
pub(super) fn health_changes(film: &mut Film, who: &str, by: i32) {
    if followed(film, who) {
        let var = health(who);
        film.beat(emit::add_to(&var, emit::number(by)));
        film.beat(emit::calc(&var, emit::var(&var), "max", emit::number(0)));
        film.beat(emit::calc(&var, emit::var(&var), "min", emit::var(format!("{var}_max"))));
    }
}

/// A fight begins on the floor: its own top line takes the place of the
/// dungeon's, which is put by until it ends.
pub fn fight_begins(film: &mut Film) {
    if film.dungeon.dungeon_hud.is_some() && !film.dungeon.hud_put_by {
        top_line_off(film);
        film.dungeon.hud_put_by = true;
    }
}

/// The fight over: the dungeon's top line again, less what its one lost in
/// it (a directed fight's blows say so).
pub fn fight_ends(film: &mut Film, lost: i32) {
    if film.dungeon.hud_put_by {
        film.dungeon.hud_put_by = false;
        shown_top(film);
        if let Some((who, ..)) = film.dungeon.dungeon_hud.clone().filter(|_| lost > 0) {
            health_changes(film, &who, -lost);
        }
    }
}

/// What the one the top line follows loses in a directed fight: each blow
/// at him that says so (`krabby Crabhammer pikachu -6`).
pub fn lost_in_a_fight(film: &Film, blows: &[crate::ir::Instr]) -> i32 {
    let Some((who, ..)) = film.dungeon.dungeon_hud.as_ref() else {
        return 0;
    };
    blows
        .iter()
        .map(|blow| blow.verb().into_iter().map(str::to_string).chain(blow.words().iter().map(|word| plain(word))).collect::<Vec<String>>())
        .filter(|words| words.iter().skip(1).any(|word| word == who) && words.first() != Some(who))
        .filter_map(|words| words.iter().rev().find_map(|word| word.strip_prefix('-')?.parse::<i32>().ok()))
        .sum()
}

/// The floor a fight's top line says, on a floor whose own top line shows.
pub fn floor_for_a_fight(film: &Film) -> Option<i32> {
    film.dungeon.dungeon_hud.as_ref().map(|_| film.dungeon.floor)
}
