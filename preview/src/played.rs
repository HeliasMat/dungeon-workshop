//! A film played at once, with no window: when each thing happens, who is
//! on the stage when, who moves when. Every pass goes the same way, so a
//! moment found by one is that of the next.

use std::collections::BTreeMap;

use engine::Who;
use shell::Keys;

use crate::film::Film;

/// Where a film played at once is given up on: half an hour.
pub const MOST_FRAMES: usize = 108_000;

/// The frames of a second.
pub const SECOND: usize = 60;

/// Seconds, to a tenth.
pub fn seconds(frames: usize) -> f32 {
    (frames as f32 / 6.0).round() / 10.0
}

/// Where a scene starts, and when it can be seen (out of its fades).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Span {
    pub start: usize,
    pub seen_from: Option<usize>,
    pub seen_to: usize,
}

/// When things happen in a film, in frames as they show.
#[derive(Clone, Debug, Default)]
pub struct Timeline {
    pub frames: usize,
    pub scenes: Vec<Span>,
    /// The frame each line of the film first plays at.
    pub lines: BTreeMap<i32, usize>,
    /// Each line as it plays, each time, with its frame, for whoever takes
    /// them (`timing`).
    pub plays: Vec<(i32, usize)>,
    /// Each one who comes on the stage: when it is there, when it moves.
    pub tracks: BTreeMap<Who, Track>,
    /// Each line of the film as it played, each time: its real start and
    /// end, as the engine tells them (`FlowBeat::Line`).
    pub spans: Vec<LineSpan>,
    /// (The spans not ended yet, by line and course.)
    pub open: BTreeMap<(i32, Option<String>), std::collections::VecDeque<usize>>,
}

/// A line of the film as it played once: its number, the frames it started
/// and ended at, its course (`label`: the passage beside the film, none:
/// the film's own; `thread`: its thread as the plan numbers them, 0 the
/// film's), its line in the plan and why it started then, once joined to
/// the plan (`Timeline::join`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineSpan {
    pub number: i32,
    pub start: usize,
    pub end: usize,
    pub label: Option<String>,
    pub thread: usize,
    pub plan: Option<usize>,
    pub cause: Option<film::time::Cause>,
}

/// Someone of a film, as it goes: its name, the frames it is on the stage,
/// those it moves, from the first to the last of each.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    pub name: String,
    pub shown: Vec<(usize, usize)>,
    pub moving: Vec<(usize, usize)>,
}

/// A frame where something is, or is not: the frames it is, run together.
fn mark(spans: &mut Vec<(usize, usize)>, frame: usize, on: bool) {
    if !on {
        return;
    }
    match spans.last_mut() {
        Some((_, end)) if *end + 1 == frame => *end = frame,
        _ => spans.push((frame, frame)),
    }
}

impl Timeline {
    /// The scene (from 1) playing at that frame; 0 before the first.
    pub fn scene_at(&self, frame: usize) -> usize {
        self.scenes.iter().filter(|scene| scene.start <= frame).count()
    }

    /// When a line plays: that line, or the one holding it (a line of a
    /// fight's block, a comment).
    pub fn line(&self, line: i32) -> Option<(i32, usize)> {
        self.lines.range(..=line).next_back().map(|(line, frame)| (*line, *frame))
    }

    /// The engine's marks of a frame: spans started, spans ended.
    pub fn marked(&mut self, marks: Vec<engine::LineMark>, frame: usize) {
        for mark in marks {
            let key = (mark.line, mark.thread.clone());
            if mark.starts {
                self.open.entry(key).or_default().push_back(self.spans.len());
                self.spans.push(LineSpan { number: mark.line, start: frame, end: frame, label: mark.thread, ..LineSpan::default() });
            } else if let Some(at) = self.open.get_mut(&key).and_then(std::collections::VecDeque::pop_front) {
                self.spans[at].end = frame;
            }
        }
    }

    /// The spans still open end there (the film is over).
    pub fn close(&mut self, frame: usize) {
        for at in std::mem::take(&mut self.open).into_values().flatten() {
            self.spans[at].end = frame;
        }
    }

    /// Each span given its thread, its line in the plan and its cause.
    pub fn join(&mut self, plan: &film::time::Plan) {
        for span in &mut self.spans {
            span.thread = span.label.as_ref().and_then(|label| plan.threads.iter().position(|thread| thread.name == *label)).map_or(0, |at| at + 1);
            span.plan = plan.shown().find(|(_, line)| line.number as i32 == span.number && line.thread == span.thread).map(|(at, _)| at);
            span.cause = span.plan.map(|at| plan.cause(at));
        }
    }

    /// The end of a scene's frames: where the next starts.
    pub fn end_of(&self, scene: usize) -> usize {
        self.scenes.get(scene).map_or(self.frames, |next| next.start)
    }
}

/// That frame of a film played at once: the same at any pass, from the
/// start or from a moment kept (a choice is made a moment after it shows).
pub fn tick_at(film: &mut Film, frame: usize) {
    let enter = film.asking() && frame % 90 == 89;
    // (Nobody at the keys: who roams goes on, to the next scene.)
    film.nobody_plays();
    film.tick(Keys { enter, ..Keys::default() }, 1.0);
}

/// A film going by, a frame at a time, as a player would read it.
pub struct Pass {
    pub film: Film,
    pub frame: usize,
    pub timeline: Timeline,
}

impl Pass {
    pub fn new(mut film: Film) -> Self {
        film.keep_lines();
        // (Lines are read by themselves once written out and read, as the
        // window's auto mode reads them: `read_after`.)
        film.auto = true;
        Pass { film, frame: 0, timeline: Timeline::default() }
    }

    /// A frame goes by: false once the film is over.
    pub fn step(&mut self) -> bool {
        if self.film.over || self.frame >= MOST_FRAMES {
            return false;
        }
        tick_at(&mut self.film, self.frame);
        self.frame += 1;
        let timeline = &mut self.timeline;
        timeline.frames = self.frame;
        for line in self.film.lines_played() {
            timeline.lines.entry(line).or_insert(self.frame);
            timeline.plays.push((line, self.frame));
        }
        timeline.marked(self.film.spans_played(), self.frame);
        while timeline.scenes.len() < self.film.scenes().len() {
            timeline.scenes.push(Span { start: self.frame, ..Span::default() });
        }
        if let Some(scene) = timeline.scenes.last_mut().filter(|_| self.film.seen()) {
            scene.seen_from = scene.seen_from.or(Some(self.frame));
            scene.seen_to = self.frame;
        }
        let sim = &self.film.play().sim;
        for (who, actor) in sim.stage.cast.iter().filter(|(who, _)| !who.is_thing()) {
            let track = timeline.tracks.entry(*who).or_insert_with(|| Track { name: sim.cast.who(*who).1, ..Track::default() });
            mark(&mut track.shown, self.frame, true);
            mark(&mut track.moving, self.frame, actor.walks());
        }
        if self.film.over || self.frame >= MOST_FRAMES {
            self.timeline.close(self.frame);
        }
        !self.film.over
    }

    /// Played to its end.
    pub fn to_the_end(&mut self) {
        while self.step() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_not_played_is_found_in_the_one_holding_it() {
        let timeline = Timeline { lines: BTreeMap::from([(4, 10), (9, 80)]), ..Timeline::default() };
        assert_eq!(timeline.line(9), Some((9, 80)));
        assert_eq!(timeline.line(7), Some((4, 10)));
        assert_eq!(timeline.line(2), None);
    }

    #[test]
    fn the_engine_s_marks_make_each_line_s_span() {
        let mark = |line: i32, thread: Option<&str>, starts: bool| engine::LineMark { line, thread: thread.map(str::to_string), starts };
        let mut timeline = Timeline::default();
        timeline.marked(vec![mark(3, None, true)], 1);
        timeline.marked(vec![mark(4, Some("thread1"), true), mark(3, None, false), mark(5, None, true)], 10);
        timeline.marked(vec![mark(4, Some("thread1"), false)], 25);
        timeline.close(40);
        let spans: Vec<(i32, usize, usize, Option<&str>)> =
            timeline.spans.iter().map(|span| (span.number, span.start, span.end, span.label.as_deref())).collect();
        assert_eq!(spans, [(3, 1, 10, None), (4, 10, 25, Some("thread1")), (5, 10, 40, None)]);
    }

    #[test]
    fn frames_are_told_in_seconds_and_scenes() {
        let scenes = vec![Span { start: 5, ..Span::default() }, Span { start: 100, ..Span::default() }];
        let timeline = Timeline { frames: 200, scenes, ..Timeline::default() };
        assert_eq!((timeline.scene_at(1), timeline.scene_at(50), timeline.scene_at(100)), (0, 1, 2));
        assert_eq!((timeline.end_of(1), timeline.end_of(2)), (100, 200));
        assert_eq!(seconds(90), 1.5);
    }
}
