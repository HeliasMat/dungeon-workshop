//! A scene file played through without the game, as fast as it goes:
//! every line read at once, the first answers taken in turn, the words
//! asked for given, a wait for the player let go after a while. What it
//! finds: the engine stopping short (a panic), a line its numbers fill
//! wrong, a scene that never ends; and, as remarks, who speaks out of
//! sight or behind the dialogue box.
//!
//! - `watch`: what is seen as a film goes by ([`Watch`]), and the line on screen;
//! - `sight`: who speaks out of sight;
//! - `spoken`: who and what a line speaks of or to, out of the frame;
//! - `framing`: what the frame shows wrong;
//! - `bodies`: the bodies at rest and walking.

use std::collections::{HashMap, HashSet};

use glam::Vec2;

use crate::beat::*;
use crate::director::{Call, Director, LINE, Mark, StoryEvent};
use crate::problem::{point, told};
use crate::stage::{Stage, attribute};
use crate::vocabulary::Vocabulary;
use crate::who::Who;
use diagnostics::Diagnostic;
use diagnostics::Said;

mod bodies;
mod framing;
mod sight;
mod spoken;
#[cfg(test)]
mod tests;
mod watch;

use sight::out_of_sight;
pub use watch::{Seen, Watch};

/// What a scene played through found (see [`rehearse`]).
#[derive(Debug, Default)]
pub struct Rehearsal {
    /// What is wrong: it stops the scene, or plays it wrong.
    pub problems: Vec<Diagnostic>,
    /// What may be meant, but should be looked at (`remarque :`).
    pub remarks: Vec<Diagnostic>,
    /// The frames it took to the end.
    pub frames: u32,
}

/// The frames a scene is played at most before it is said not to end.
const MOST: u32 = 200_000;
/// The frames the director may go no further in its scene (waiting, or
/// going round a loop) before a wait for the player is let go (as if the
/// player had done it): the rest played at once until it goes further.
const STILL: u32 = 600;
/// The times a scene is played through at most, each with other answers.
const WAYS: usize = 4;
/// The frames a line stays before it is read.
const READ_AFTER: u32 = 20;
/// The lower screen (256 by 192): its middle is where the view looks; the
/// dialogue box covers its lowest `BOX_HIGH` pixels (the camera's own), or
/// its highest. Someone is hidden by it once his feet are that far into it
/// (below), his head that far (above).
const HALF: Vec2 = Vec2::new(128.0, 96.0);
const FEET_IN_THE_BOX: f32 = 12.0;
const HEAD_IN_THE_BOX: f32 = 8.0;
/// How high over its feet someone shows, about: its head is in sight
/// only when its feet are that far below the screen's top.
const HEAD: f32 = 18.0;

/// Where a point is off the frame, from where the view looks: the side it
/// went out by, the farther one.
pub(super) fn side(at: Vec2, centre: Vec2, zoom: f32) -> &'static str {
    let off = (at - centre) * zoom / HALF;
    match off.x.abs() >= off.y.abs() {
        true if off.x < 0.0 => "on the left, out of the picture",
        true => "on the right, out of the picture",
        false if off.y < 0.0 => "at the top, out of the picture",
        false => "at the bottom, out of the picture",
    }
}

/// A point as a film reads it best: by the name of a spot near it (the
/// film's `spot`, a way in, a room's letter), its numbers after.
pub(super) fn whereabouts(stage: &Stage, at: Vec2) -> String {
    const NEAR: f32 = 24.0;
    let named = stage.place.spots.iter().map(|(name, there)| (name, *there)).chain(stage.place.entrances.iter().map(|(name, there)| (name, *there)));
    match named.filter(|(_, there)| there.distance(at) <= NEAR).min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at))) {
        Some((name, _)) => format!("near “{name}” {}", point(at)),
        None => format!("at {}", point(at)),
    }
}

/// Plays a scene file's events (as `read_events` gives them) through.
pub fn rehearse(events: Vec<StoryEvent>, words: &'static Vocabulary) -> Rehearsal {
    rehearse_for(events, MOST, words)
}

/// The same, for so many frames at most: played again with other answers
/// (the first ones, then the second ones...), as many times as the
/// question with most answers has, up to `WAYS`; what is found told once.
fn rehearse_for(events: Vec<StoryEvent>, most: u32, words: &'static Vocabulary) -> Rehearsal {
    let (mut rehearsal, mut told) = (Rehearsal::default(), HashSet::new());
    for way in 0..WAYS {
        let answers = play_way(events.clone(), most, way, &mut rehearsal, &mut told, words);
        if way + 1 >= answers {
            break;
        }
    }
    rehearsal
}

/// The scene played through once, the answers taken from that one on in
/// turn (`way`: 0, the first, then the second...). How many answers the
/// question with most of them had.
fn play_way(events: Vec<StoryEvent>, most: u32, way: usize, rehearsal: &mut Rehearsal, told: &mut HashSet<Diagnostic>, words: &'static Vocabulary) -> usize {
    let mut director = Director::new(events, 0, words);
    let mut stage = Stage::default();
    let (mut focus, mut still, mut shown_for, mut asked, mut answers_most) = (None::<Vec2>, 0, 0, 0_usize, 0_usize);
    // (The furthest the director went: event, scene, beat.)
    let mut furthest: Option<Mark> = None;
    // (The line on screen, judged once the view had time to get there:
    // who says it, its words, where the director was, its frames shown.)
    let mut said: Option<(Who, String, (String, i32), u32)> = None;
    for frame in 0..most {
        // (A stop for whoever writes the scene, `halt`, plays on.)
        director.author.stepping = false;
        let calls = director.tick(&mut stage, 1.0, focus.unwrap_or(Vec2::ZERO));
        for call in calls {
            match call {
                Call::Over => {
                    if way == 0 {
                        rehearsal.frames = frame;
                    }
                    return answers_most;
                }
                // (The answers in turn, from this way's on.)
                Call::Ask(answers) => {
                    director.answer((!answers.is_empty()).then(|| (asked + way) % answers.len()));
                    (asked, answers_most) = (asked + 1, answers_most.max(answers.len()));
                }
                Call::AskText(given) => director.text_given(if given.is_empty() { "Pikachu".to_string() } else { given }),
                Call::Host { fields, .. } if words.host_holds.contains(&fields[0].as_str()) => director.menu_done(false),
                Call::Game { ask, .. } if ask.holds() => director.menu_done(false),
                Call::Show(Beat::Flow(FlowBeat::Log(text))) if text.starts_with("line badly filled") => {
                    let problem = played_at(&director, text);
                    if told.insert(problem.clone()) {
                        rehearsal.problems.push(problem);
                    }
                }
                // (Not with `keepspeaker`: the view glides to who speaks.)
                Call::Say { entity: Some(who), text, .. } if !director.manner.keep_speaker => {
                    if let Some((who, text, at, _)) = said.take() {
                        judge(rehearsal, told, &stage, focus, who, &text, &at);
                    }
                    said = Some((who, text, (place(&director), line_of(&director)), 0));
                }
                _ => {}
            }
        }
        // (A line read a moment after it shows, as a player reads it.)
        shown_for = if director.reading && !director.line_timed() { shown_for + 1 } else { 0 };
        if shown_for > READ_AFTER {
            director.read();
        }
        // (Going no further: a wait for the player, let go.)
        let further = match (director.at, furthest) {
            (Some(at), Some(best)) => (at.event, at.scene) != (best.event, best.scene) || at.beat > best.beat,
            (at, None) => at.is_some(),
            (None, Some(_)) => false,
        };
        if further {
            (furthest, still) = (director.at, 0);
        } else {
            still += 1;
        }
        director.hurried = still > STILL;
        focus = stage.advance(1.0, frame as f32 / 60.0, focus).or(focus);
        if let Some((_, _, _, shown)) = &mut said {
            *shown += 1;
        }
        if let Some((who, text, at, shown)) = said.take() {
            match shown >= READ_AFTER || !director.reading {
                true => judge(rehearsal, told, &stage, focus, who, &text, &at),
                false => said = Some((who, text, at, shown)),
            }
        }
    }
    let problem = played_at(&director, format!("the scene does not end in {most} frames (an endless loop?)"));
    if told.insert(problem.clone()) {
        rehearsal.problems.push(problem);
    }
    answers_most
}

/// A line said judged (see [`out_of_sight`]): a remark if its speaker is
/// not seen, told once.
fn judge(rehearsal: &mut Rehearsal, told: &mut HashSet<Diagnostic>, stage: &Stage, focus: Option<Vec2>, who: Who, text: &str, at: &(String, i32)) {
    if let Some(remark) = out_of_sight(stage, focus, who, text).map(|why| told_of(at.0.clone(), at.1, why).as_remark())
        && told.insert(remark.clone())
    {
        rehearsal.remarks.push(remark);
    }
}

/// The film's line the director plays (`LINE`; 0, none: a scene not written as a film).
fn line_of(director: &Director) -> i32 {
    director.numbers().get(LINE).copied().unwrap_or(0.0) as i32
}

/// What went wrong as it played: at the film's line playing then, if it is
/// known (where the director was, said after), else where the director was.
fn told_of(place: String, line: i32, message: impl Into<Said>) -> Diagnostic {
    let said = message.into();
    match usize::try_from(line) {
        Ok(line) if line > 0 => Diagnostic::at(line, "").naming(said.words(&format!(" ({place})"))),
        _ => Diagnostic::played(place, "").naming(said),
    }
}

/// The same, where the director is now.
fn played_at(director: &Director, message: impl Into<Said>) -> Diagnostic {
    told_of(place(director), line_of(director), message)
}

/// Where the director is, for whoever writes the scene: the scene's name
/// and what it plays there.
fn place(director: &Director) -> String {
    let Some(at) = director.at else {
        return "after the end".to_string();
    };
    let name = director.events.get(at.event).and_then(|event| event.scenes.get(at.scene)).map_or("", |scene| scene.name.as_str());
    let beat = director.beat(at).map(|beat| format!("{beat:?}")).unwrap_or_default();
    let beat: String = beat.chars().take(60).collect();
    format!("scene “{name}”, near {beat}")
}

/// Words without what writes them (`[CS:N]`).
pub(crate) fn without_tags(text: &str) -> String {
    let mut in_tag = false;
    text.chars()
        .filter(|letter| {
            in_tag = (in_tag || *letter == '[') && *letter != ']';
            !in_tag && *letter != ']'
        })
        .collect()
}

/// A way one faces, by its name (`down`, `right`...).
fn way_named(way: u16) -> &'static str {
    Direction::ALL.into_iter().find(|known| known.number() == way).map_or("?", Direction::name)
}
