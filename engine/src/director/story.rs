//! The story as a file tells it: its events and their scenes.

use super::*;

/// A scene of an event: where it is set, and what it does in turn.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub level: u16,
    /// Its name, as its file gives it (see `FlowBeat::GotoScene`).
    pub name: String,
    pub beats: Vec<Beat>,
    /// Where its blocks end, worked out the first time it is asked.
    pub ways: WaysOf,
}

/// A scene's ways, worked out once from its beats (`Ways`): never told
/// apart in comparing two scenes, which their beats are.
#[derive(Clone, Debug, Default)]
pub struct WaysOf(std::sync::OnceLock<std::sync::Arc<Ways>>);

impl PartialEq for WaysOf {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl WaysOf {
    /// Its ways; worked out again if beats were added since (a scene the
    /// game lengthens before it plays).
    pub(super) fn of(&self, beats: &[Beat]) -> std::sync::Arc<Ways> {
        let made = self.0.get_or_init(|| std::sync::Arc::new(Ways::of(beats)));
        if made.long == beats.len() { made.clone() } else { std::sync::Arc::new(Ways::of(beats)) }
    }
}

/// Where a scene goes, by beat: each label's place, each way of an `If`
/// its next, its `EndIf`, each `While` its `EndWhile` and back, the loop
/// each beat is in. (`NONE`: none.)
#[derive(Debug, Default)]
pub struct Ways {
    long: usize,
    labels: std::collections::HashMap<String, usize>,
    next_part: Vec<usize>,
    end_if: Vec<usize>,
    end_while: Vec<usize>,
    start_while: Vec<usize>,
    in_loop: Vec<usize>,
}

const NONE: usize = usize::MAX;

impl Ways {
    fn of(beats: &[Beat]) -> Ways {
        let long = beats.len();
        let mut ways = Ways {
            long,
            labels: std::collections::HashMap::new(),
            next_part: vec![NONE; long],
            end_if: vec![NONE; long],
            end_while: vec![NONE; long],
            start_while: vec![NONE; long],
            in_loop: vec![NONE; long],
        };
        // (Each `If` open: its parts so far.)
        let mut ifs: Vec<Vec<usize>> = Vec::new();
        let mut loops: Vec<usize> = Vec::new();
        for (at, beat) in beats.iter().enumerate() {
            ways.in_loop[at] = loops.last().copied().unwrap_or(NONE);
            match beat {
                Beat::Flow(FlowBeat::Label(name)) => {
                    ways.labels.entry(name.clone()).or_insert(at);
                }
                Beat::Flow(FlowBeat::If(_)) => ifs.push(vec![at]),
                Beat::Flow(FlowBeat::ElseIf(_) | FlowBeat::Else) => {
                    if let Some(parts) = ifs.last_mut() {
                        if let Some(last) = parts.last() {
                            ways.next_part[*last] = at;
                        }
                        parts.push(at);
                    }
                }
                Beat::Flow(FlowBeat::EndIf) => {
                    if let Some(parts) = ifs.pop() {
                        if let Some(last) = parts.last() {
                            ways.next_part[*last] = at;
                        }
                        for part in parts {
                            ways.end_if[part] = at;
                        }
                    }
                }
                Beat::Flow(FlowBeat::While(_)) => loops.push(at),
                Beat::Flow(FlowBeat::EndWhile) => {
                    if let Some(start) = loops.pop() {
                        ways.end_while[start] = at;
                        ways.start_while[at] = start;
                    }
                }
                _ => {}
            }
        }
        ways
    }

    /// The beat just past a label.
    pub(super) fn past_label(&self, label: &str) -> Option<usize> {
        self.labels.get(label).map(|at| at + 1)
    }

    /// The next way of the `If`, `ElseIf` or `Else` at that beat.
    pub(super) fn next_part(&self, at: usize) -> Option<usize> {
        self.next_part.get(at).copied().filter(|next| *next != NONE)
    }

    /// The `EndIf` of a way.
    pub(super) fn end_if(&self, at: usize) -> Option<usize> {
        self.end_if.get(at).copied().filter(|end| *end != NONE)
    }

    /// The `EndWhile` of the loop at that beat (its `While`, or in it).
    pub(super) fn end_of_loop(&self, at: usize) -> Option<usize> {
        let start = if self.end_while.get(at).is_some_and(|end| *end != NONE) { at } else { *self.in_loop.get(at)? };
        self.end_while.get(start).copied().filter(|end| *end != NONE)
    }

    /// The `While` of the loop at that beat (its `EndWhile`, or in it).
    pub(super) fn start_of_loop(&self, at: usize) -> Option<usize> {
        match self.start_while.get(at).copied().filter(|start| *start != NONE) {
            Some(start) => Some(start),
            None => self.in_loop.get(at).copied().filter(|start| *start != NONE),
        }
    }
}

/// An event of the story: its number (0: one the game plays of its own),
/// its name, the moment it leaves the story at (chapter and step, as the
/// original's scripts count them), and its scenes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StoryEvent {
    pub number: usize,
    pub name: String,
    pub moment: (i32, i32),
    pub scenes: Vec<Scene>,
    /// Made by a film (`film::compile`): its scenes open with no
    /// collisions, the game's with them (see `PlayBeat::Collisions`).
    pub film: bool,
}

/// The events of a story file's lines (as `read_scene` gives them):
/// `event <number> <name> <chapter> <step>`, then `scene <level> <name>`
/// and what each scene does. What an event shows outside its scenes is a
/// scene of its own. (`actor` lines, who stands in a scene of the game's
/// scripts as it opens, are left: the scene's record puts them.)
pub fn read_events(lines: impl IntoIterator<Item = Vec<String>>, words: &'static Vocabulary) -> Vec<StoryEvent> {
    let mut events: Vec<StoryEvent> = Vec::new();
    for line in lines {
        let number = |at: usize| line.get(at).and_then(|field| field.parse::<i32>().ok());
        if line[0] == "event" {
            if let Some(event) = number(1) {
                let moment = (number(3).unwrap_or(0), number(4).unwrap_or(0));
                events.push(StoryEvent { number: event as usize, name: line.get(2).cloned().unwrap_or_default(), moment, scenes: Vec::new(), film: false });
            }
            continue;
        }
        let Some(scenes) = events.last_mut().map(|event| &mut event.scenes) else {
            continue;
        };
        match line[0].as_str() {
            "scene" => scenes.extend(number(1).map(|level| Scene { level: level as u16, name: line.get(2).cloned().unwrap_or_default(), ..Scene::default() })),
            "actor" => {}
            // (The time of the line said just before.)
            "timed" => {
                if let Some(Beat::Course(Course::Say { timed: timed @ None, .. })) = scenes.last_mut().and_then(|scene| scene.beats.last_mut())
                    && let Some(frames) = line.get(1).and_then(|field| field.parse::<f32>().ok())
                {
                    *timed = Some(Frames(frames));
                }
            }
            _ => {
                if scenes.is_empty() {
                    scenes.push(Scene::default());
                }
                // (A line with numbers put in when it is played: its typed
                // beat, or kept as it is written, see `Beat::played`.)
                let beat = match crate::beat::played_filled(&line) {
                    true => Some(Beat::played(line.clone(), words).unwrap_or(Beat::Flow(FlowBeat::Filled(line)))),
                    false => Beat::parse(&line, words).ok(),
                };
                scenes.last_mut().expect("a scene").beats.extend(beat);
            }
        }
    }
    events
}
