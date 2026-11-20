//! A film's time, told: when each line starts and ends, on which track and
//! thread, how it is placed and why it starts then, as the engine tells it
//! (`Timeline::spans`, joined to the film's plan: nothing guessed); what
//! plays at a moment, who moves; `--why N`, the chain that places a
//! line; `--no-play`, the plan worked out without playing ("≥" after
//! an event).

use std::collections::BTreeMap;

use engine::Who;
use film::time::{Cause, Clock, Length, Placement, Plan};

use super::Piece;
use super::json::{self, Value, json};
use super::looking::name_of;
use crate::moment::{self, Moment};
use preview::played::{Pass, Timeline, seconds};

/// How far someone goes in a frame, at least, to be moving.
const MOVING: f32 = 0.3;
/// The shortest move told, in frames: not a shuffle.
const SHORTEST_MOVE: usize = 6;
/// How long a line's words are shown, at most.
const SHOWN: usize = 60;

/// What `timing` is asked besides its moments.
#[derive(Clone, Copy, Debug, Default)]
pub struct Asked {
    /// `--why N`: why line N starts where it does.
    pub why: Option<i32>,
    /// `--no-play`: the plan, without playing.
    pub unplayed: bool,
}

/// A line of the film as it played (or as the plan places it): its
/// number, its frames, at least so far (`--no-play`, after an event),
/// its line in the plan.
#[derive(Clone, Debug)]
struct Played {
    line: i32,
    start: usize,
    end: usize,
    at_least: bool,
    thread: usize,
    plan: Option<usize>,
}

/// Someone moving, from a frame to another.
struct Move {
    who: String,
    start: usize,
    end: usize,
    how: String,
}

/// `timing <film> [--json]`, told; see `chrono_run`.
pub fn chrono(piece: &Piece, moments: &[String], as_json: bool, asked: Asked) -> Result<(), String> {
    if let Some(told) = chrono_run(piece, moments, as_json, asked)? {
        json::print(&told);
    }
    Ok(())
}

/// The film's time, for a program: as `timing <film> --json` tells it.
pub fn chrono_value(piece: &Piece, moments: &[String]) -> Result<Value, String> {
    chrono_run(piece, moments, true, Asked::default())?.ok_or_else(|| "timing: nothing".to_string())
}

/// The film played through: its timeline joined to its plan, and who moved.
fn played_through(piece: &Piece, plan: &Plan) -> Result<(Timeline, Vec<Move>), String> {
    let mut pass = Pass::new(piece.film()?);
    let mut last: BTreeMap<Who, (f32, f32, f32)> = BTreeMap::new();
    let mut moving: BTreeMap<Who, (usize, &'static str)> = BTreeMap::new();
    let mut moves: Vec<(Who, usize, usize, &'static str)> = Vec::new();
    // (The tracks, by node and property: when each started; those over.)
    let mut tracking: BTreeMap<(String, &'static str), usize> = BTreeMap::new();
    let mut tracked: Vec<(String, usize, usize, String)> = Vec::new();
    loop {
        let going = pass.step();
        let frame = pass.frame;
        let play = pass.film.play();
        for (who, actor) in &play.sim.stage.cast {
            if !matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. }) {
                continue;
            }
            let now = (actor.position.x, actor.position.y, actor.shown_height());
            let was = last.insert(*who, now).unwrap_or(now);
            let gone = ((now.0 - was.0).powi(2) + (now.1 - was.1).powi(2)).sqrt();
            let how = if actor.walks() {
                Some(if actor.runs() { "runs" } else { "walks" })
            } else if (now.2 - was.2).abs() > MOVING {
                Some(if now.2 > was.2 { "rises" } else { "sinks" })
            } else if gone > MOVING {
                Some("glides")
            } else {
                None
            };
            match (how, moving.get(who).copied()) {
                (Some(how), None) => _ = moving.insert(*who, (frame, how)),
                (None, Some((start, how))) => {
                    moving.remove(who);
                    if frame - start >= SHORTEST_MOVE {
                        moves.push((*who, start, frame, how));
                    }
                }
                _ => {}
            }
        }
        let now: Vec<(String, &'static str)> = play
            .sim
            .stage
            .tracks
            .iter()
            .map(|track| {
                let node = match &track.node {
                    engine::NodeRef::Who(who) => name_of(piece, &pass.film, *who),
                    other => other.name(),
                };
                (node, track.prop.prop().name)
            })
            .collect();
        for key in &now {
            tracking.entry(key.clone()).or_insert(frame);
        }
        for (key, start) in tracking.clone() {
            if !now.contains(&key) {
                tracking.remove(&key);
                tracked.push((key.0, start, frame, format!("anim {}", key.1)));
            }
        }
        if !going {
            break;
        }
    }
    let end = pass.frame;
    for ((node, prop), start) in std::mem::take(&mut tracking) {
        tracked.push((node, start, end, format!("anim {prop}")));
    }
    for (who, (start, how)) in std::mem::take(&mut moving) {
        moves.push((who, start, end, how));
    }
    let film = &pass.film;
    let mut moves: Vec<Move> =
        moves.into_iter().map(|(who, start, end, how)| Move { who: name_of(piece, film, who), start, end, how: how.to_string() }).collect();
    moves.extend(tracked.into_iter().map(|(who, start, end, how)| Move { who, start, end, how }));
    moves.sort_by_key(|one| (one.start, one.end));
    let mut timeline = pass.timeline;
    timeline.join(plan);
    Ok((timeline, moves))
}

/// Why a plan's line starts where it does, in words.
fn cause_said(plan: &Plan, cause: &Cause) -> String {
    let line = |at: usize| plan.lines[at].number;
    let from = |since: &Option<usize>| since.map_or(String::new(), |at| format!(", waited for since the end of line {}", line(at)));
    match cause {
        Cause::Start => "at the start".to_string(),
        Cause::After(at) => format!("after line {}", line(*at)),
        Cause::With(Some(at)) => format!("with “&”, at the end of line {}", line(*at)),
        Cause::With(None) => "with “&”, at the start".to_string(),
        Cause::Clock { clock, offset, since } => {
            let time = seconds(*offset as usize);
            match clock {
                Clock::Scene => format!("at {time} s of the scene"),
                Clock::Music => format!("at {time} s of the music"),
                Clock::Mark(mark) => format!("{time} s after the mark {mark}"),
                Clock::Here => format!("{time} s after the end of line {}", since.map_or(0, line)),
            }
        }
        Cause::Event { what, since } => format!("when {what}{}", from(since)),
    }
}

/// The plan's line a cause comes from, if one.
fn caused_by(cause: &Cause) -> Option<usize> {
    match cause {
        Cause::After(at) | Cause::With(Some(at)) => Some(*at),
        Cause::Clock { since, .. } | Cause::Event { since, .. } => *since,
        Cause::With(None) | Cause::Start => None,
    }
}

/// How a line is placed, for a program: its kind, and what it counts from.
fn place_value(plan: &Plan, at: usize) -> Value {
    let line = &plan.lines[at];
    let before = plan.before(at).map(|before| plan.lines[before].number);
    let towards = match &line.placement {
        Placement::After | Placement::With => json!(before),
        Placement::At { clock, offset } => {
            let clock = match clock {
                Clock::Scene => "scene".to_string(),
                Clock::Music => "music".to_string(),
                Clock::Mark(mark) => format!("mark {mark}"),
                Clock::Here => "here".to_string(),
            };
            json!({"clock": clock, "offset_s": json::seconds(*offset as usize)})
        }
        Placement::When(event) => json!(event),
    };
    json!({"kind": line.placement.kind(), "to": towards})
}

/// How long a line lasts, for a program.
fn length_value(plan: &Plan, at: usize, names: &[(String, String)]) -> Value {
    match &plan.lines[at].length {
        Length::Known(frames) => json!({"kind": "known", "frames": frames}),
        Length::Event { handle, at_least } => {
            let said = match handle {
                film::time::Handle::Who(code) => names.iter().find(|(own, _)| own == code).map_or(handle.said(), |(_, name)| format!("{name} arrives")),
                other => other.said(),
            };
            json!({"kind": "event", "frames": at_least, "waits": said})
        }
    }
}

/// `timing <film>`: every line, when it starts and ends; `timing <film> 40s`:
/// what plays then; `timing <film> 30s 40s`: what plays between; `chrono
/// <film> number 12`: that line, each time, and why.
fn chrono_run(piece: &Piece, moments: &[String], as_json: bool, asked: Asked) -> Result<Option<Value>, String> {
    let written = piece.written.as_ref().ok_or("timing: a film (.film)")?;
    let times = Times::of(piece, &written.plan, &written.names, asked.unplayed)?;
    if let Some(line) = asked.why {
        return why(piece, times.plan, &times.played, line, as_json, &|one| times.told(one));
    }
    let read = match moments.is_empty() {
        true => Vec::new(),
        false => moment::read(moments)?,
    };
    // (A line asked alone: that line, each time it plays, and why.)
    if let [Moment::Line(line)] = read.as_slice()
        && !as_json
    {
        let each: Vec<&Played> = times.played.iter().filter(|one| one.line == *line).collect();
        if each.is_empty() {
            return Err(format!("line {line} does not play"));
        }
        println!("{}: line {line}, {}", piece.name, times.words(*line));
        each.iter().for_each(|one| println!("{}", times.told(one)));
        return Ok(None);
    }
    let frames = times.frames_of(&read)?;
    if as_json {
        return Ok(Some(times.json(&frames, asked.unplayed)));
    }
    match frames.as_slice() {
        [] => times.print_all(asked.unplayed),
        [at] => times.print_at(*at),
        [from, to, ..] => times.print_between(*from.min(to), *from.max(to)),
    }
    Ok(None)
}

/// A film's lines as they played (or as the plan places them), who moved, the timeline, its text.
struct Times<'a> {
    piece: &'a Piece,
    plan: &'a Plan,
    names: &'a [(String, String)],
    played: Vec<Played>,
    moves: Vec<Move>,
    timeline: Option<Timeline>,
    /// Its last frame.
    end: usize,
    source: Vec<&'a str>,
}

impl<'a> Times<'a> {
    fn of(piece: &'a Piece, plan: &'a Plan, names: &'a [(String, String)], unplayed: bool) -> Result<Times<'a>, String> {
        let (played, moves, timeline): (Vec<Played>, Vec<Move>, Option<Timeline>) = if unplayed {
            let places = plan.resolve();
            let played = plan
                .shown()
                .map(|(at, line)| Played {
                    line: line.number as i32,
                    start: places[at].start.max(0) as usize,
                    end: places[at].end.max(0) as usize,
                    at_least: places[at].at_least,
                    thread: line.thread,
                    plan: Some(at),
                })
                .collect();
            (played, Vec::new(), None)
        } else {
            let (timeline, moves) = played_through(piece, plan)?;
            // (The scene's own setting up, line 0, is not a line of the film.)
            let played = timeline
                .spans
                .iter()
                .filter(|span| span.number > 0)
                .map(|span| Played { line: span.number, start: span.start, end: span.end, at_least: false, thread: span.thread, plan: span.plan })
                .collect();
            (played, moves, Some(timeline))
        };
        let end = timeline.as_ref().map_or_else(|| played.iter().map(|one| one.end).max().unwrap_or(0), |timeline| timeline.frames);
        Ok(Times { piece, plan, names, played, moves, timeline, end, source: piece.text.lines().collect() })
    }

    /// A line's words, as written.
    fn said(&self, line: i32) -> &'a str {
        self.source.get((line as usize).wrapping_sub(1)).map_or("", |words| words.trim())
    }

    /// A line's words, cut short.
    fn words(&self, line: i32) -> String {
        shortened(self.said(line))
    }

    /// Whether a line plays beside the film (a thread, or `&`: not waited for).
    fn beside(&self, one: &Played) -> bool {
        one.thread > 0 || one.plan.is_some_and(|at| !self.plan.lines[at].waits)
    }

    fn cause(&self, one: &Played) -> Option<String> {
        one.plan.map(|at| cause_said(self.plan, &self.plan.cause(at)))
    }

    /// A line told: its start and end, its number, its course, its words, why it starts then.
    fn told(&self, one: &Played) -> String {
        let course = match (one.thread > 0, one.plan.is_some_and(|at| !self.plan.lines[at].waits)) {
            (true, _) => "beside ",
            (false, true) => "& ",
            (false, false) => "",
        };
        let least = if one.at_least { "≥" } else { " " };
        let why = self.cause(one).map_or(String::new(), |why| format!("   ({why})"));
        format!("{:>6.1} →{least}{:>6.1} s  line {:<4} {course}{}{why}", seconds(one.start), seconds(one.end), one.line, self.words(one.line))
    }

    /// The frames of the moments asked (a line: its own start, not a moment after it).
    fn frames_of(&self, read: &[Moment]) -> Result<Vec<usize>, String> {
        read.iter()
            .map(|moment| match (moment, &self.timeline) {
                (Moment::Line(line), _) => self.played.iter().find(|one| one.line == *line).map(|one| one.start).ok_or(format!("line {line} does not play")),
                (other, Some(timeline)) => other.frame(timeline),
                (Moment::Second(second), None) => Ok((second * 60.0) as usize),
                (Moment::Scene(..), None) => Err("--no-play: a moment in seconds or a line".to_string()),
            })
            .collect()
    }

    fn line_json(&self, one: &Played) -> Value {
        let at = one.plan;
        json!({
            "line": one.line,
            "start_s": json::seconds(one.start),
            "end_s": json::seconds(one.end),
            "start_frame": one.start,
            "end_frame": one.end,
            "at_least": one.at_least,
            "track": at.map(|at| self.plan.lines[at].lane.said()),
            "thread": one.thread,
            "place": at.map(|at| place_value(self.plan, at)),
            "duration": at.map(|at| length_value(self.plan, at, self.names)),
            "cause": self.cause(one),
            "text": self.said(one.line),
        })
    }

    /// For a program: the lines and moves of the moments asked (all of them, none asked), what plays then.
    fn json(&self, frames: &[usize], unplayed: bool) -> Value {
        let (from, to) = match frames {
            [] => (0, self.end),
            [at] => (*at, *at),
            [from, to, ..] => (*from.min(to), *from.max(to)),
        };
        let lines: Vec<Value> = self
            .played
            .iter()
            .filter(|one| frames.is_empty() || (one.start <= to && (one.end.max(one.start + 1) > from || one.start >= from)))
            .map(|one| self.line_json(one))
            .collect();
        let movers: Vec<Value> = self
            .moves
            .iter()
            .filter(|one| frames.is_empty() || (one.start <= to && one.end > from))
            .map(|one| json!({"who": one.who, "how": one.how, "start_s": json::seconds(one.start), "end_s": json::seconds(one.end)}))
            .collect();
        let next = self.played.iter().find(|one| !frames.is_empty() && one.start > to && !self.beside(one)).map(|one| self.line_json(one));
        json::answer(
            "timing",
            json!({
                "film": self.piece.name,
                "duration_s": json::seconds(self.end),
                "without_playing": unplayed,
                "from_s": (!frames.is_empty()).then(|| json::seconds(from)),
                "at_s": (!frames.is_empty()).then(|| json::seconds(to)),
                "lines": lines,
                "moves": movers,
                "then": next,
            }),
        )
    }

    /// Every line, from its start to its end.
    fn print_all(&self, unplayed: bool) {
        let how = if unplayed { "the plan without playing (“≥”: after an event)" } else { "as the engine played them" };
        println!(
            "{}: {}{} s, each line from its start to its end, {how} (“beside”: a thread beside the film; “&”: the film does not wait for it)",
            self.piece.name,
            if unplayed { "≥ " } else { "" },
            seconds(self.end)
        );
        self.played.iter().for_each(|one| println!("{}", self.told(one)));
    }

    /// What plays at a moment: the film's line, those beside, who moves, what comes then.
    fn print_at(&self, at: usize) {
        let scene = self.timeline.as_ref().map_or(String::new(), |timeline| format!(", scene {}", timeline.scene_at(at)));
        println!("{} at {} s{scene}", self.piece.name, seconds(at));
        let now: Vec<&Played> = self.played.iter().filter(|one| one.start <= at && at < one.end.max(one.start + 1)).collect();
        match now.iter().rev().find(|one| !self.beside(one)) {
            Some(one) => println!("the film: {}", self.told(one)),
            None => println!("the film: nothing plays there"),
        }
        let aside: Vec<&&Played> = now.iter().filter(|one| self.beside(one)).collect();
        if !aside.is_empty() {
            println!("beside:");
            aside.iter().for_each(|one| println!("  {}", self.told(one)));
        }
        self.print_movers(self.moves.iter().filter(|one| one.start <= at && at < one.end).collect());
        if let Some(next) = self.played.iter().find(|one| one.start > at && !self.beside(one)) {
            println!("then: {}", self.told(next));
        }
    }

    /// What plays between two moments, who moves.
    fn print_between(&self, from: usize, to: usize) {
        println!("{} from {} s to {} s", self.piece.name, seconds(from), seconds(to));
        self.played.iter().filter(|one| one.start < to && (one.end > from || one.start >= from)).for_each(|one| println!("{}", self.told(one)));
        self.print_movers(self.moves.iter().filter(|one| one.start < to && one.end > from).collect());
    }

    fn print_movers(&self, movers: Vec<&Move>) {
        if !movers.is_empty() {
            println!("who moves:");
            movers.iter().for_each(|one| println!("  {:>6.1} → {:>6.1} s  {} {}", seconds(one.start), seconds(one.end), one.who, one.how));
        }
    }
}

/// Words cut short, past `SHOWN` letters.
fn shortened(said: &str) -> String {
    let mut shown: String = said.chars().take(SHOWN).collect();
    if said.chars().count() > SHOWN {
        shown.push('…');
    }
    shown
}

/// `--why N`: line N where it first starts, then the line that places
/// it, and so on up to a scene's start.
fn why(piece: &Piece, plan: &Plan, played: &[Played], line: i32, as_json: bool, told: &dyn Fn(&Played) -> String) -> Result<Option<Value>, String> {
    let mut at = played.iter().find(|one| one.line == line && one.plan.is_some()).cloned().ok_or(format!("line {line} does not play"))?;
    let mut chain: Vec<(Played, String)> = Vec::new();
    for _ in 0..64 {
        let cause = plan.cause(at.plan.expect("a line of the plan"));
        chain.push((at.clone(), cause_said(plan, &cause)));
        let Some(from) = caused_by(&cause) else { break };
        // (That line, the last time it started before this one.)
        let Some(before) = played.iter().rfind(|one| one.plan == Some(from) && one.start <= at.start).cloned() else { break };
        at = before;
    }
    if as_json {
        let links: Vec<Value> = chain
            .iter()
            .map(|(one, why)| json!({"line": one.line, "start_s": json::seconds(one.start), "end_s": json::seconds(one.end), "at_least": one.at_least, "cause": why}))
            .collect();
        return Ok(Some(json::answer("chrono", json!({"film": piece.name, "why": line, "chain": links}))));
    }
    println!("{}: why line {line} starts there", piece.name);
    for (depth, (one, _)) in chain.iter().enumerate() {
        println!("{}{}", "  ".repeat(depth), told(one).trim_start());
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use film::time::{Lane, PlanLine};

    fn line(number: usize, thread: usize, placement: Placement) -> PlanLine {
        PlanLine { number, lane: Lane::Film, thread, placement, length: Length::Known(60), waits: true, written: 0, scene: 1, relay: false }
    }

    #[test]
    fn why_a_line_starts_is_said_from_the_plan() {
        let mut plan = Plan::default();
        plan.lines.push(line(3, 0, Placement::After));
        plan.lines.push(line(4, 0, Placement::After));
        plan.lines.push(line(5, 0, Placement::At { clock: Clock::Here, offset: 120 }));
        assert_eq!(cause_said(&plan, &plan.cause(0)), "at the start");
        assert_eq!(cause_said(&plan, &plan.cause(1)), "after line 3");
        assert_eq!(cause_said(&plan, &plan.cause(2)), "2 s after the end of line 4");
        assert_eq!(caused_by(&plan.cause(2)), Some(1));
        assert_eq!(place_value(&plan, 1), json!({"kind": "after", "to": 3}));
    }

    #[test]
    fn long_words_are_cut_short() {
        assert_eq!(shortened("pikachu: Hi."), "pikachu: Hi.");
        let long = "a".repeat(SHOWN + 5);
        assert_eq!(shortened(&long), format!("{}…", "a".repeat(SHOWN)));
    }
}
