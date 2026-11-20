//! Time in a scene, one way: a line ending with `&` starts and the film goes
//! on; `wait` waits (so long, for someone, for all). A line said with `&`
//! is played beside the film: a passage (`parallel`) the scene waits for
//! before it closes. The words of before (`at 3s:`, `during`, `after`) say
//! how they are written now.
//!
//! Lines anchored to what happens, beside the film too: `when a arrives:
//! …` plays once a has got where it was going; `mark name` marks the
//! instant, and `at name + 2s: …` plays so long after it; `at 12s: …` so
//! long after the scene's start, `at music 12s: …` once the music
//! playing now has played so long. Each on one line, or as a block up to
//! its `end`.

use super::{Line, Word};
use crate::emit;
use crate::film::Film;
use engine::{Beat, Compare, Condition, FlowBeat, Frames, Test, Value};

use crate::ir::{Instr, Shape};
use crate::time::{Clock, Placement};

pub const WORDS: &[Word] = &[
    Word::new("at", &["text..."], "at name [+ 2s]: …: the line (or the block up to end) played beside, so long after the mark (calc, waituntil clock); at 12s: …: so long after the scene's start (waituntil clock); at music 12s: …: once the music has played so long (waituntil music_clock)").maybe(gone_word).instant(),
    Word::new("when", &["*who arrives|arrive [text...]"], "when a arrives: …: the line (or the block up to end) played beside the film as soon as a has got where it was going (parallel, waitfor)").maybe(when_arrives).instant(),
    Word::new("mark", &["name:word"], "mark name: a mark at that instant of the scene (set _mark_name clock), for “at name + 2s:”").maybe(marked).instant(),
    Word::new("beside", &["[name:word]"], "beside [name] … end: its lines played beside the film, one after the other (parallel); “wait name” waits for it; a line ended by “& as name” plays in it").maybe(beside).instant(),
    Word::new(
        "pace",
        &["lines d:duration", "lines off", "slow|brisk|normal", "n:number"],
        "pace slow|brisk|normal or a number: the waits said that follow (wait 2s), longer or shorter, a word's own time its own; pace lines 2s: each line said stays that long, then goes on alone; pace lines off: each as long as it is read",
    )
    .maybe(paced)
    .instant(),
    Word::new("together", &[""], "together … end: each line starts at the same time (parallel), and the film waits for the longest (waituntil)").maybe(together_said).until(),
];

/// What only the scene's own course can play.
const NOT_MEANWHILE: [&str; 5] = ["fight", "ask", "menu", "dungeon", "ground_place"];

/// Lines played beside the film, starting now, one after another: the film
/// goes on meanwhile, `wait all` and the scene's end wait for them. Its
/// name, for `wait`.
pub(crate) fn meanwhile(film: &mut Film, number: usize, lines: Vec<Instr>) -> String {
    meanwhile_placed(film, number, Vec::new(), lines, Placement::With)
}

/// Lines played beside the film, their first so placed (`at 12s`, `when
/// a arrives`): what places it played first (`prelude`), no line's own.
pub(crate) fn meanwhile_placed(film: &mut Film, number: usize, mut prelude: Vec<Instr>, mut lines: Vec<Instr>, placement: Placement) -> String {
    // (A block the text writes beside the film opens with what it waits
    // for, `when a leaves`, `when x > 3`, `after 2s`: its placing.)
    let placement = match (prelude.is_empty(), lines.first().map(|first| (first.number, &first.shape))) {
        (true, Some((at, shape @ (Shape::Engine(_) | Shape::Beat(_))))) if at == number && placement == Placement::With => {
            let placed = match shape {
                Shape::Beat(beat @ Beat::Flow(FlowBeat::WaitUntil(_))) => {
                    Some(Placement::When(beat.fields(&scenes::vocabulary::POKEMON).unwrap_or_default()[1..].join(" ")))
                }
                Shape::Beat(Beat::Flow(FlowBeat::Wait(frames))) => Some(Placement::At { clock: Clock::Here, offset: frames.0.round() as i32 }),
                Shape::Engine(fields) if fields.first().is_some_and(|verb| verb == "waitmoves") => {
                    Some(Placement::When(format!("{} leaves", fields[1..].join(" "))))
                }
                _ => None,
            };
            match placed {
                Some(placed) => {
                    prelude.push(lines.remove(0));
                    placed
                }
                None => placement,
            }
        }
        _ => placement,
    };
    film.timing.threads += 1;
    let thread = film.timing.threads;
    let (name, past) = (format!("thread{thread}"), format!("thread{thread}_past"));
    film.beat(emit::goto(past.clone()));
    film.beat(emit::label(name.clone()));
    film.plan.open_thread(&name, placement);
    let start = film.written.len();
    film.plan.pause();
    for line in &prelude {
        film.play(line);
    }
    film.plan.resume();
    for line in &lines {
        film.play(line);
    }
    film.throw_lands();
    film.plan.close_thread();
    if film.written[start..].iter().any(|(_, made)| NOT_MEANWHILE.contains(&made.verb().as_str())) {
        film.wrong(number, "a fight or a question is not played beside the film: without “&”".to_string());
    }
    film.beat(emit::set(format!("{name}_done"), emit::number(1)));
    film.beat(emit::back());
    film.beat(emit::label(past));
    film.beat(emit::set(format!("{name}_done"), emit::number(0)));
    film.beat(emit::parallel(name.clone()));
    film.timing.awaited.push(name.clone());
    name
}

/// `together` … `end`: each line starts now, all at once, and the film waits
/// for the longest.
fn together(film: &mut Film, number: usize, lines: &[Instr]) {
    let threads: Vec<String> = lines.iter().map(|line| meanwhile(film, number, vec![line.clone()])).collect();
    film.timing.awaited.retain(|name| !threads.contains(name));
    for name in threads {
        film.beat(emit::wait_until(format!("{name}_done"), emit::number(1)));
    }
}

/// Waits for what goes on beside the film to be over.
pub(crate) fn wait_meanwhile(film: &mut Film) {
    for name in std::mem::take(&mut film.timing.awaited) {
        film.beat(emit::wait_until(format!("{name}_done"), emit::number(1)));
    }
}

/// `beside [name]` ... `end`: beside the film, waited for by that name, `wait name`.
fn beside(film: &mut Film, line: &Line) -> bool {
    if line.block.is_empty() {
        return false;
    }
    let thread = meanwhile(film, line.number, line.block.clone());
    if let Some(name) = line.args.word("name") {
        film.timing.named.insert(crate::words::plain(name), thread);
    }
    true
}

/// `together` ... `end`: its lines all at once.
fn together_said(film: &mut Film, line: &Line) -> bool {
    if line.block.is_empty() {
        return false;
    }
    together(film, line.number, &line.block);
    true
}

/// What an anchored line plays: what follows its colon, or its block.
fn anchored_lines(film: &mut Film, line: &Line, then: Option<&Instr>) -> Option<Vec<Instr>> {
    match (then, line.block.is_empty()) {
        (Some(then), _) => Some(vec![then.clone()]),
        (None, false) => Some(line.block.clone()),
        (None, true) => {
            film.wrong(line.number, format!("“{}”: nothing to play (a line after “:”, or a block up to the “end”)", line.text));
            None
        }
    }
}

/// `when a arrives: …`: beside the film, once a has got where it went.
fn when_arrives(film: &mut Film, line: &Line) -> bool {
    let Shape::Arrives { who, then } = &line.shape else {
        return false;
    };
    let Some(lines) = anchored_lines(film, line, then.as_deref()) else {
        return true;
    };
    let prelude = vec![Instr::word(line.number, "wait", who, true)];
    meanwhile_placed(film, line.number, prelude, lines, Placement::When(format!("{} arrives", who.join(" "))));
    true
}

/// `mark name`: the instant marked, in the scene's clock.
fn marked(film: &mut Film, line: &Line) -> bool {
    let Some(name) = line.args.word("name").map(|name| crate::words::plain(name.trim_end_matches(':'))) else {
        return false;
    };
    film.beat(emit::set(format!("_mark_{name}"), emit::var("clock")));
    film.plan.mark(&name);
    if !film.timing.time_marks.contains(&name) {
        film.timing.time_marks.push(name);
    }
    true
}

/// `pace slow`: the waits said from here on, longer or shorter, till
/// another; `pace lines 2s`: each line said stays so long, then goes on.
fn paced(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    if args.has("lines") {
        film.timing.line_time = if args.has("off") { None } else { args.frames("d") };
        return true;
    }
    film.pace = match (args.has("slow"), args.has("brisk"), args.number("n")) {
        (true, ..) => 1.4,
        (_, true, _) => 0.7,
        (.., Some(pace)) if (0.2..=4.0).contains(&pace) => pace,
        (.., Some(_)) => {
            film.wrong(line.number, "pace: a number from 0.2 (brisker) to 4 (slower)".to_string());
            return true;
        }
        _ => 1.0,
    };
    true
}

/// `at name + 2s: …`: beside the film, so long after the mark.
fn after_mark(film: &mut Film, line: &Line) -> bool {
    let Shape::AtMark { name, after, then } = &line.shape else {
        return false;
    };
    if name == crate::ir::SCENE_CLOCK || name == crate::ir::MUSIC_CLOCK {
        return at_clock(film, line, name == crate::ir::MUSIC_CLOCK, *after, then.as_deref());
    }
    if name == crate::ir::HERE_CLOCK {
        return from_here(film, line, *after, then.as_deref());
    }
    if !film.timing.time_marks.contains(name) {
        let known = if film.timing.time_marks.is_empty() { "none in this scene".to_string() } else { film.timing.time_marks.join(", ") };
        film.wrong(line.number, format!("“{name}” is not marked above in the scene (“mark {name}”; marks: {known})"));
        return true;
    }
    let Some(lines) = anchored_lines(film, line, then.as_deref()) else {
        return true;
    };
    let at = format!("_at_{name}_{}", line.number);
    let reached = Condition { var: "clock".to_string(), compare: Compare::GreaterOrEqual, value: Value::Var(at.clone()) };
    let prelude = vec![
        Instr::beat(line.number, emit::calc(&at, Value::Var(format!("_mark_{name}")), "+", Value::Number(f64::from(*after)))),
        Instr::beat(line.number, Beat::Flow(FlowBeat::WaitUntil(Test { any: vec![vec![reached]] }))),
    ];
    meanwhile_placed(film, line.number, prelude, lines, Placement::At { clock: Clock::Mark(name.clone()), offset: *after });
    true
}

/// `at 12s: …`: beside the film, so long after the scene's start (its
/// clock, `clock`); `at music 12s: …`: once the music playing now has
/// played so long (`music_clock`).
fn at_clock(film: &mut Film, line: &Line, music: bool, at: i32, then: Option<&Instr>) -> bool {
    if music && !music_known(film, line.number, "at music") {
        return true;
    }
    let Some(lines) = anchored_lines(film, line, then) else {
        return true;
    };
    let clock = if music { engine::MUSIC_CLOCK } else { "clock" };
    let prelude = vec![Instr::beat(line.number, emit::wait_reached(clock, f64::from(at)))];
    let clock = if music { Clock::Music } else { Clock::Scene };
    meanwhile_placed(film, line.number, prelude, lines, Placement::At { clock, offset: at });
    true
}

/// `at +2s: …`: beside the film, so long from here (where the film is when
/// it reads the line).
fn from_here(film: &mut Film, line: &Line, after: i32, then: Option<&Instr>) -> bool {
    let Some(lines) = anchored_lines(film, line, then) else {
        return true;
    };
    let prelude = vec![Instr::beat(line.number, Beat::Flow(FlowBeat::Wait(Frames(after as f32))))];
    meanwhile_placed(film, line.number, prelude, lines, Placement::At { clock: Clock::Here, offset: after });
    true
}

/// Whether a music was given before this line in the film (`music …`):
/// else what counts by its clock would wait for ever, told.
pub(crate) fn music_known(film: &mut Film, number: usize, said: &str) -> bool {
    if !film.timing.music {
        film.wrong(number, format!("“{said}”: no music above in the film (“music name” first)"));
    }
    film.timing.music
}

/// The ways of time a film had once, and what to write instead (the engine's
/// `after 20 ...` too: one way of time in a film).
fn gone_word(film: &mut Film, line: &Line) -> bool {
    if line.verb == "at" && after_mark(film, line) {
        return true;
    }
    let told = match line.verb.as_str() {
        "at" if line.args.words("text").first().is_some_and(|word| word.ends_with(':')) => {
            "“at 3s: …” is no more: “wait 3s” then the line (ended by “&” if the film does not wait for it)"
        }
        "at" => return false,
        "during" => "“during … end” is no more: write the lines ended by “&”, some “wait 1s” between them, then “wait” as long as it takes",
        _ => "“after” is no more: “wait a” (or “wait 2s”) then the line",
    };
    film.wrong(line.number, told.to_string());
    true
}

/// `who: words &`: a line said, beside the film (see [`meanwhile`]).
pub(crate) fn said_meanwhile(film: &mut Film, item: &Instr) -> bool {
    let Shape::Said { who, brackets, words, beside: true } = &item.shape else {
        return false;
    };
    let said = Instr { shape: Shape::Said { who: who.clone(), brackets: brackets.clone(), words: words.clone(), beside: false }, ..item.clone() };
    meanwhile(film, item.number, vec![said]);
    true
}

/// The film's time, the whole film: how long lines stay, the marks, the
/// threads beside it, what the scene waits for before it closes, the
/// instants marked, what is open (what `end` plays to close it).
#[derive(Default)]
pub struct TimingState {
    /// (How long lines stay, if the film says; the box at the top.)
    pub line_time: Option<i32>,
    pub marks: usize,
    pub threads: usize,
    pub awaited: Vec<String>,
    /// What is played beside the film under a name (`& as wind`), by
    /// that name: its thread.
    pub named: std::collections::HashMap<String, String>,
    /// The instants marked in the scene at hand (`mark name`).
    pub time_marks: Vec<String>,
    /// A music was given, somewhere above in the film (`music …`): its
    /// clock goes (`at music 12s`, `wait music 12s`).
    pub music: bool,
    /// The plain ambient is a sound of ours (`ambient my_wind`): the
    /// game's, coming after, stops it.
    pub ambiance_own: bool,
    /// What is open (a shot of a picture, a vision, a memory, a choice, a
    /// fight on the floor), the last last: what `end` plays to close it.
    pub opened: Vec<fn(&mut crate::film::Film, &crate::verbs::Line)>,
}
