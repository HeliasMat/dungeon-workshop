//! The film's lines as they play (`FlowBeat::Line`): when each starts and
//! ends, on the course it is played on, told to whoever watches
//! (`Author::spans`).

use super::*;

/// How long a line ended by `&` waits for who it moves to set off, in frames.
const SETTING_OFF: f32 = 30.0;

impl Director {
    /// A line of the film starts on the course at hand: the line before it
    /// there ends, if its course waited for it.
    pub(super) fn line_starts(&mut self, number: i32, end: LineEnd) {
        self.vars.insert(LINE.to_string(), f64::from(number));
        if let Some(lines) = &mut self.author.lines {
            lines.push(number);
        }
        if end == LineEnd::Again || self.author.spans.is_none() {
            return;
        }
        let thread = self.film_lines.playing;
        self.lines_end(|open| open.thread == thread && open.end == LineEnd::Next);
        let left = match &end {
            LineEnd::Frames(frames) => frames.0,
            _ => 0.0,
        };
        self.film_lines.open.push(OpenLine { line: number, thread, end, left, seen: false, done: false });
        self.line_marked(number, thread, true);
    }

    fn line_marked(&mut self, line: i32, thread: Option<u32>, starts: bool) {
        let thread = thread.and_then(|thread| self.film_lines.names.get(thread as usize).cloned());
        if let Some(spans) = &mut self.author.spans {
            spans.push(LineMark { line, thread, starts });
        }
    }

    /// Those open lines end now.
    pub(super) fn lines_end(&mut self, ends: impl Fn(&OpenLine) -> bool) {
        if self.film_lines.open.is_empty() {
            return;
        }
        let (gone, kept): (Vec<OpenLine>, Vec<OpenLine>) = std::mem::take(&mut self.film_lines.open).into_iter().partition(|open| ends(open));
        self.film_lines.open = kept;
        for open in gone {
            self.line_marked(open.line, open.thread, false);
        }
    }

    /// The lines that end by themselves (someone arrived, so long gone by,
    /// tracks over) end, as the frames go by.
    pub(super) fn lines_go_by(&mut self, stage: &Stage, frames: f32) {
        for open in &mut self.film_lines.open {
            open.done = match &open.end {
                LineEnd::Frames(_) => {
                    open.left -= frames;
                    open.left <= 0.0
                }
                // (Who sets off a little after, a place found first: given a
                // while to set off, else it was there already.)
                LineEnd::Who(who) => {
                    let busy = stage.cast.get(who).is_some_and(|actor| actor.busy()) || stage.tracked(&NodeRef::Who(*who));
                    open.left += frames;
                    open.seen |= busy;
                    !busy && (open.seen || open.left > SETTING_OFF)
                }
                LineEnd::Track(node) => !stage.tracked(node),
                LineEnd::Next | LineEnd::Again => false,
            };
        }
        self.lines_end(|open| open.done);
    }

    /// A passage meanwhile started from that label: its number.
    pub(super) fn thread_named(&mut self, label: &str) -> u32 {
        self.film_lines.names.push(label.to_string());
        (self.film_lines.names.len() - 1) as u32
    }

    /// A passage meanwhile plays (none: the scene's course).
    pub(super) fn plays_on(&mut self, thread: Option<u32>) {
        self.film_lines.playing = thread;
    }

    /// A passage meanwhile is over: the lines it waited for end.
    pub(super) fn thread_ends(&mut self, thread: u32) {
        self.lines_end(|open| open.thread == Some(thread) && open.end == LineEnd::Next);
    }
}
