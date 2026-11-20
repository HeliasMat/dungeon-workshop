//! The film's plan: besides the beats, one entry a line, its track, its
//! thread, how it is placed and how long it lasts (known, or until a handle
//! the engine watches). Made as the film is lowered (`Film::play` opens a
//! line, each beat written tells it what it waits for), read without
//! playing ([`Plan::resolve`]) or with the engine's real starts and ends.

use engine::{Beat, Course, FlowBeat, SpeechBeat};

/// Where a line is shown, the Workshop's track: the film's, someone's, the
/// camera's, the music's, the sounds', the scenes'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lane {
    Film,
    Who(String),
    Camera,
    Music,
    Sounds,
    Scenes,
}

impl Lane {
    pub fn said(&self) -> String {
        match self {
            Lane::Film => "film".to_string(),
            Lane::Who(who) => who.clone(),
            Lane::Camera => "camera".to_string(),
            Lane::Music => "music".to_string(),
            Lane::Sounds => "sounds".to_string(),
            Lane::Scenes => "scenes".to_string(),
        }
    }
}

/// What a line placed at a time counts from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clock {
    /// The scene's start (`at 12s`).
    Scene,
    /// The music playing (`at music 12s`).
    Music,
    /// A mark of the scene (`at m + 2s`).
    Mark(String),
    /// Here, where the cursor is (`at +2s`).
    Here,
}

/// How a line is placed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Placement {
    /// After the line before: the cursor waits for its end.
    After,
    /// `&`: at the same instant, the cursor goes on.
    With,
    /// At a clock and so many frames after it, beside the cursor.
    At { clock: Clock, offset: i32 },
    /// At an event (`a arrives`, `a leaves`, a test), beside the cursor.
    When(String),
}

impl Placement {
    /// Its kind, as the JSON says it.
    pub fn kind(&self) -> &'static str {
        match self {
            Placement::After => "after",
            Placement::With => "with",
            Placement::At { .. } => "at",
            Placement::When(_) => "when",
        }
    }
}

/// What a line waits for, when its time is not known before it plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Handle {
    /// Someone gets where it goes (its code).
    Who(String),
    /// Someone sets off.
    Departs(String),
    /// A sound ends.
    Sound(String),
    /// A node's tracks end.
    Tracks(String),
    /// A line said is read.
    Reading,
    /// Nobody moves, nothing beside the film plays.
    All,
    /// Threads beside the film end (`thread3`), the longest.
    Threads(Vec<String>),
    /// A test holds.
    Test(String),
    Camera,
    Light,
    Bubbles,
    Key,
    /// A while drawn by chance.
    Chance,
}

impl Handle {
    pub fn said(&self) -> String {
        match self {
            Handle::Who(who) => format!("{who} arrives"),
            Handle::Departs(who) => format!("{who} leaves"),
            Handle::Sound(sound) => format!("sound {sound}"),
            Handle::Tracks(node) => format!("tracks of {node}"),
            Handle::Reading => "reading".to_string(),
            Handle::All => "all".to_string(),
            Handle::Threads(names) => names.join(" "),
            Handle::Test(test) => test.clone(),
            Handle::Camera => "camera".to_string(),
            Handle::Light => "light".to_string(),
            Handle::Bubbles => "bubbles".to_string(),
            Handle::Key => "key".to_string(),
            Handle::Chance => "random".to_string(),
        }
    }
}

/// How long a line lasts: so many frames, or until an event (so many at
/// least).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Length {
    Known(i32),
    Event { handle: Handle, at_least: i32 },
}

impl Length {
    pub fn kind(&self) -> &'static str {
        match self {
            Length::Known(_) => "known",
            Length::Event { .. } => "event",
        }
    }
}

/// A line of the plan.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanLine {
    pub number: usize,
    pub lane: Lane,
    /// Its thread: 0 the film's course, else the n-th beside it.
    pub thread: usize,
    pub placement: Placement,
    pub length: Length,
    /// Its thread waits for its end (not a line ended by `&`).
    pub waits: bool,
    /// Where its beats start among what the film writes.
    pub written: usize,
    pub scene: usize,
    /// Only the bearer of a thread whose first line is its own: told by
    /// that line.
    pub relay: bool,
}

/// A thread beside the film: its label (`thread3`), the thread it starts
/// from, how many lines the plan had then.
#[derive(Clone, Debug, PartialEq)]
pub struct Thread {
    pub name: String,
    pub parent: usize,
    pub since: usize,
}

/// Why a line starts where it does.
#[derive(Clone, Debug, PartialEq)]
pub enum Cause {
    /// The scene's start.
    Start,
    /// The end of that line (of the plan).
    After(usize),
    /// `&`, where the cursor is: past that line, if any.
    With(Option<usize>),
    /// A clock, so long after.
    Clock { clock: Clock, offset: i32, since: Option<usize> },
    /// An event, waited for from past that line.
    Event { what: String, since: Option<usize> },
}

/// A line placed without playing: its start and end in frames; `at_least`:
/// they wait on an event (≥).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub start: i32,
    pub end: i32,
    pub at_least: bool,
}

/// The plan of a film.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub lines: Vec<PlanLine>,
    /// The threads beside the film, `threads[n - 1]` the n-th.
    pub threads: Vec<Thread>,
    /// The marks (`marque m`), each by the line that makes it.
    pub marks: Vec<(String, usize)>,
    open: Vec<Option<usize>>,
    on: Vec<usize>,
    paused: usize,
    next: Option<Placement>,
    scene: usize,
    read: i32,
}

impl Plan {
    /// The thread lines go to now.
    pub fn thread(&self) -> usize {
        self.on.last().copied().unwrap_or(0)
    }

    /// The line beats go to now.
    fn at(&self) -> Option<usize> {
        self.open.last().copied().flatten().filter(|_| self.paused == 0)
    }

    /// A line of the text starts being lowered (`wait`: not ended by `&`);
    /// `written`: how much the film has written. Closed by [`Plan::end`].
    pub fn begin(&mut self, number: usize, lane: Lane, wait: bool, written: usize) {
        if self.paused > 0 || number == 0 {
            self.open.push(None);
            return;
        }
        let thread = self.thread();
        // (The same line again, in a word that plays others, or the lines a
        // block of the text writes for it: one line.)
        if let Some(at) = self.at().filter(|at| self.lines[*at].number == number && self.lines[*at].thread == thread) {
            self.open.push(Some(at));
            return;
        }
        let last = self.lines.len().checked_sub(1);
        if self.next.is_none()
            && let Some(at) = last.filter(|at| self.lines[*at].number == number && self.lines[*at].thread == thread && self.lines[*at].waits == wait)
        {
            self.open.push(Some(at));
            return;
        }
        let (placement, waits) = match self.next.take() {
            Some(placement) => (placement, true),
            None if wait => (Placement::After, true),
            None => (Placement::With, false),
        };
        let line = PlanLine { number, lane, thread, placement, length: Length::Known(0), waits, written, scene: self.scene, relay: false };
        self.lines.push(line);
        self.open.push(Some(self.lines.len() - 1));
    }

    /// The line at hand is lowered.
    pub fn end(&mut self) {
        self.open.pop();
    }

    /// What the film writes from here is not a line's (the waits that place
    /// a line beside the film), up to [`Plan::resume`].
    pub fn pause(&mut self) {
        self.paused += 1;
    }

    pub fn resume(&mut self) {
        self.paused = self.paused.saturating_sub(1);
    }

    /// A scene opens.
    pub fn scene_opens(&mut self) {
        self.scene += 1;
    }

    /// A mark of the scene, made by the line at hand.
    pub fn mark(&mut self, name: &str) {
        if let Some(at) = self.at() {
            self.marks.push((name.to_string(), at));
        }
    }

    /// A thread beside the film starts, by that label: its first line so
    /// placed. Closed by [`Plan::close_thread`].
    pub fn open_thread(&mut self, name: &str, placement: Placement) {
        let parent = self.thread();
        self.threads.push(Thread { name: name.to_string(), parent, since: self.lines.len() });
        self.on.push(self.threads.len());
        self.next = Some(placement);
    }

    /// The thread at hand is written. Its bearer, a line whose only line
    /// is that thread's first (`pikachu: Oh &`), is only its bearer.
    pub fn close_thread(&mut self) {
        let Some(thread) = self.on.pop() else { return };
        self.next = None;
        let since = self.threads[thread - 1].since;
        let first = self.lines[since..].iter().find(|line| line.thread == thread).map(|line| line.number);
        if let Some(bearer) = self.at().filter(|at| *at < since && Some(self.lines[*at].number) == first) {
            self.lines[bearer].relay = true;
        }
    }

    /// What the line at hand lasts beside the film, ended by `&`: its
    /// handle, the first told.
    pub fn handle(&mut self, length: Length) {
        let Some(at) = self.at() else { return };
        let line = &mut self.lines[at];
        if !line.waits && line.length == Length::Known(0) {
            line.length = length;
        }
    }

    /// A beat written for the line at hand: what it waits for.
    pub fn saw(&mut self, beat: &Beat) {
        let Some(at) = self.at() else { return };
        let event = |handle: Handle| Some(handle);
        let (frames, handle) = match beat {
            Beat::Flow(FlowBeat::Wait(frames)) => (frames.0.round() as i32, None),
            Beat::Flow(FlowBeat::WaitBetween { from, .. }) => (from.0.round() as i32, event(Handle::Chance)),
            Beat::Flow(FlowBeat::WaitFor(who)) | Beat::Flow(FlowBeat::WaitNear { who, .. }) => (0, event(Handle::Who(who.to_string()))),
            Beat::Flow(FlowBeat::WaitMoves(who)) => (0, event(Handle::Departs(who.to_string()))),
            Beat::Flow(FlowBeat::WaitTrack(node)) => (0, event(Handle::Tracks(node.to_string()))),
            Beat::Flow(FlowBeat::WaitSound { of, .. }) => (0, event(Handle::Sound(of.to_string()))),
            Beat::Flow(FlowBeat::WaitAll) => (0, event(Handle::All)),
            Beat::Flow(FlowBeat::WaitCam) => (0, event(Handle::Camera)),
            Beat::Flow(FlowBeat::WaitFade) => (0, event(Handle::Light)),
            Beat::Flow(FlowBeat::WaitBubbles) => (0, event(Handle::Bubbles)),
            Beat::Flow(FlowBeat::WaitUntil(test)) => {
                let fields = beat.fields(&scenes::vocabulary::POKEMON).unwrap_or_default();
                let thread = match test.any.as_slice() {
                    [all] => match all.as_slice() {
                        [condition] => condition.var.strip_suffix("_done").filter(|name| name.starts_with("thread")).map(str::to_string),
                        _ => None,
                    },
                    _ => None,
                };
                (0, event(thread.map_or_else(|| Handle::Test(fields[1..].join(" ")), |name| Handle::Threads(vec![name]))))
            }
            Beat::Course(Course::Say { timed, .. }) => {
                self.read = timed.map_or(0, |frames| frames.0.round() as i32);
                return;
            }
            Beat::Speech(SpeechBeat::Said) => (std::mem::take(&mut self.read), event(Handle::Reading)),
            // (A move of a line ended by `&`: it lasts until who moves gets there.)
            Beat::Motion(_) | Beat::Place(_) => {
                let fields = beat.fields(&scenes::vocabulary::POKEMON).unwrap_or_default();
                if let Some(who) = fields.get(1).and_then(|who| engine::Who::parse(who)) {
                    self.handle(Length::Event { handle: Handle::Who(who.to_string()), at_least: 0 });
                }
                return;
            }
            _ => return,
        };
        let line = &mut self.lines[at];
        line.length = match (std::mem::replace(&mut line.length, Length::Known(0)), handle) {
            (Length::Known(known), None) => Length::Known(known.saturating_add(frames)),
            (Length::Known(known), Some(handle)) => Length::Event { handle, at_least: known.saturating_add(frames) },
            (Length::Event { handle: Handle::Threads(mut names), at_least }, Some(Handle::Threads(more))) => {
                names.extend(more);
                Length::Event { handle: Handle::Threads(names), at_least: at_least.saturating_add(frames) }
            }
            (Length::Event { handle, at_least }, _) => Length::Event { handle, at_least: at_least.saturating_add(frames) },
        };
    }

    /// The lines of the plan, without the bearers of threads: each with its
    /// index.
    pub fn shown(&self) -> impl Iterator<Item = (usize, &PlanLine)> {
        self.lines.iter().enumerate().filter(|(_, line)| !line.relay)
    }

    /// Lines numbered anew (those of a file taken in, told as the line that
    /// takes it in), what the film wrote moved (the bodies put first).
    pub fn renumbered(&mut self, told: impl Fn(usize) -> usize, moved_from: usize, by: usize) {
        for line in &mut self.lines {
            line.number = told(line.number);
            if line.written >= moved_from {
                line.written += by;
            }
        }
    }

    /// The line the cursor of a line's thread stood past when it was
    /// placed: the last waited for of its thread before it, or of the
    /// thread it started from.
    pub fn before(&self, index: usize) -> Option<usize> {
        let (mut thread, mut upto) = (self.lines[index].thread, index);
        loop {
            let found = (0..upto).rev().find(|at| self.lines[*at].thread == thread && self.lines[*at].waits && !self.lines[*at].relay);
            if found.is_some() || thread == 0 {
                return found;
            }
            let strand = &self.threads[thread - 1];
            (thread, upto) = (strand.parent, strand.since.min(upto));
        }
    }

    /// Why a line starts where it does.
    pub fn cause(&self, index: usize) -> Cause {
        let before = self.before(index);
        match &self.lines[index].placement {
            Placement::After => before.map_or(Cause::Start, Cause::After),
            Placement::With => Cause::With(before),
            Placement::At { clock, offset } => Cause::Clock { clock: clock.clone(), offset: *offset, since: before },
            Placement::When(what) => Cause::Event { what: what.clone(), since: before },
        }
    }

    /// Each line placed without playing (by index; a bearer too), from the
    /// known times; one after an event is placed at least so far (≥). A
    /// line waiting for threads beside the film (`together`) ends with the
    /// longest, once they are placed.
    pub fn resolve(&self) -> Vec<Place> {
        let mut cursor: Vec<(i32, bool)> = vec![(0, false); self.threads.len() + 1];
        let mut scenes: Vec<(usize, i32)> = Vec::new();
        let mut out: Vec<Place> = Vec::with_capacity(self.lines.len());
        let mut started = vec![false; self.threads.len() + 1];
        let mut pending: Vec<usize> = Vec::new();
        let ended = |this: &Self, at: usize, out: &mut Vec<Place>, cursor: &mut Vec<(i32, bool)>| {
            let line = &this.lines[at];
            let Length::Event { handle: Handle::Threads(names), at_least: least } = &line.length else { return };
            let (start, mut at_least) = (out[at].start, out[at].at_least);
            let mut longest = *least;
            for name in names {
                match this.threads.iter().position(|thread| thread.name == *name) {
                    Some(thread) => {
                        at_least |= cursor[thread + 1].1;
                        longest = longest.max(cursor[thread + 1].0 - start);
                    }
                    None => at_least = true,
                }
            }
            out[at] = Place { start, end: start + longest.max(0), at_least };
            if line.waits {
                cursor[line.thread] = (out[at].end, at_least);
            }
        };
        for (index, line) in self.lines.iter().enumerate() {
            // (A thread starts where the one it comes from is.)
            for (at, thread) in self.threads.iter().enumerate() {
                if !started[at + 1] && thread.since <= index {
                    started[at + 1] = true;
                    cursor[at + 1] = cursor[thread.parent];
                }
            }
            // (What waits for threads of its own, before the next line of its thread.)
            for at in std::mem::take(&mut pending) {
                match self.lines[at].thread == line.thread {
                    true => ended(self, at, &mut out, &mut cursor),
                    false => pending.push(at),
                }
            }
            if scenes.last().is_none_or(|(scene, _)| *scene != line.scene) {
                scenes.push((line.scene, cursor[0].0));
            }
            let (here, here_unsure) = cursor[line.thread];
            let scene_start = scenes.last().map_or(0, |(_, start)| *start);
            let (start, mut at_least) = match &line.placement {
                Placement::After | Placement::With => (here, here_unsure),
                Placement::At { clock: Clock::Scene, offset } => ((scene_start + offset).max(here), here_unsure),
                Placement::At { clock: Clock::Here, offset } => (here + offset, here_unsure),
                Placement::At { clock: Clock::Mark(name), offset } => {
                    match self.marks.iter().rev().find(|(mark, at)| mark == name && *at < index).and_then(|(_, at)| out.get(*at)) {
                        Some(mark) => ((mark.start + offset).max(here), here_unsure || mark.at_least),
                        None => (here, true),
                    }
                }
                Placement::At { clock: Clock::Music, .. } | Placement::When(_) => (here, true),
            };
            let lasts = match &line.length {
                Length::Known(frames) => *frames,
                Length::Event { handle: Handle::Threads(_), at_least: least } => {
                    pending.push(index);
                    *least
                }
                Length::Event { at_least: least, .. } => {
                    at_least = true;
                    *least
                }
            };
            let end = start + lasts.max(0);
            if line.waits && !pending.contains(&index) {
                cursor[line.thread] = (end, at_least);
            }
            out.push(Place { start, end, at_least });
        }
        for at in pending {
            ended(self, at, &mut out, &mut cursor);
        }
        out
    }
}
