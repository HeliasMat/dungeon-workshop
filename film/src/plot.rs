//! The Story family: the logic of a story told, as Ink and Yarn Spinner
//! have it. Read by the language (`language/`: `option "text" [if test]
//! [once]`, `{if test: words | other}`, `{value}`, a line's `#id`) and
//! written by `text/front/`; here what goes from one part of a film to
//! another and counts what was played:
//!
//! - `go moment <name>`, `go scene <place>`: on to that moment (its label,
//!   `_moment_<name>`; in another scene, that scene opened, then on to it
//!   once it is set), or that scene from its start;
//! - `seen(<name>)` in a test: how many times a moment or a scene was
//!   played, the number `seen_<name>` (counted for the run of a film alone;
//!   kept by the game's story across plays when a game plays it:
//!   [`counters`], `game/src/lib.rs`).
//!
//! A film that does none of it is written as before: no label, no count.

use std::collections::{BTreeSet, HashSet};

use engine::{Beat, Compare, Condition, FlowBeat, Test, Value};

use crate::compile::scenes::SceneOf;
use crate::emit;
use crate::film::Film;
use crate::ir::Instr;
use crate::verbs::{Line, Word};
use crate::words::plain;

/// The words of the family: `go`.
pub const WORDS: &[Word] = &[Word::new(
    "go",
    &["moment name:text...", "scene name:word"],
    "on to another moment of the film (its label; in another scene, that scene, then the moment), or another scene from its start (gotoscene)",
)
.maybe(went)];

/// The number a film holds which moment of another scene it goes to.
const GOING: &str = "_go_moment";

/// A moment's or a scene's name as compared: its words plain, joined by `_`
/// (`moment: The bet` is `the_bet`).
pub fn slug(name: &str) -> String {
    name.split_whitespace().map(plain).filter(|word| !word.is_empty()).collect::<Vec<_>>().join("_")
}

/// The number that counts how many times that moment or scene was played.
pub fn seen_var(name: &str) -> String {
    format!("seen_{}", slug(name))
}

/// The label a moment starts at.
fn label(slug: &str) -> String {
    format!("_moment_{slug}")
}

/// Whether a film's text goes from one part to another or counts them.
pub fn uses_story(text: &str) -> bool {
    text.lines().map(str::trim_start).any(|line| line.starts_with("go moment ") || line.starts_with("go scene ") || line.contains("seen("))
}

/// The numbers a film counts its moments and scenes by (`seen_the_bet`), if
/// it counts them: those a game keeps for it from one play to the next.
pub fn counters(text: &str) -> BTreeSet<String> {
    if !uses_story(text) {
        return BTreeSet::new();
    }
    names(text).into_iter().map(|name| format!("seen_{name}")).collect()
}

/// A film's moments and scenes, by name (as `slug` writes them), read by the
/// language alone.
pub fn names(text: &str) -> Vec<String> {
    names_of(&language::parse(text).file)
}

/// The same, of a text read.
pub fn names_of(file: &language::ast::File) -> Vec<String> {
    use language::ast::{Kind, Stmt};
    fn moments(stmts: &[Stmt], into: &mut Vec<String>) {
        for stmt in stmts {
            match &stmt.kind {
                Kind::Moment(name) => into.push(slug(name)),
                Kind::Block(block) => {
                    moments(&block.body, into);
                    for part in &block.parts {
                        moments(&part.body, into);
                    }
                }
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    for scene in &file.scenes {
        if let Some(place) = scene.args.first() {
            found.push(slug(&place.text));
        }
        moments(&scene.body, &mut found);
    }
    let mut seen = HashSet::new();
    found.retain(|name| seen.insert(name.clone()));
    found
}

/// What the film knows of its moments and scenes, to go to them and count them.
#[derive(Clone, Debug, Default)]
pub struct StoryMap {
    /// Whether the film goes from one to another or counts them.
    pub used: bool,
    /// Each moment by name, the scene it is in (its rank).
    pub moments: Vec<(String, usize)>,
    /// Each scene's place, by rank.
    pub scenes: Vec<String>,
    /// The moments gone to from another scene: asked for as their scene opens.
    pub reached_from_afar: Vec<usize>,
    /// The scene at hand.
    pub at: usize,
}

impl StoryMap {
    /// A film's, from its scenes.
    pub fn of(scenes: &[SceneOf], text: &str) -> StoryMap {
        let mut map = StoryMap { used: uses_story(text), ..StoryMap::default() };
        if !map.used {
            return map;
        }
        fn each<'a>(lines: &'a [Instr], into: &mut Vec<&'a Instr>) {
            for line in lines {
                into.push(line);
                if let Some(inner) = &line.inner {
                    each(inner, into);
                }
            }
        }
        for (at, scene) in scenes.iter().enumerate() {
            map.scenes.push(slug(&scene.word));
            let mut lines = Vec::new();
            each(&scene.lines, &mut lines);
            for line in lines.iter().filter(|line| line.verb() == Some("moment")) {
                map.moments.push((slug(&line.words().join(" ")), at));
            }
        }
        for (at, scene) in scenes.iter().enumerate() {
            let mut lines = Vec::new();
            each(&scene.lines, &mut lines);
            for line in lines.iter().filter(|line| line.verb() == Some("go") && line.words().first().is_some_and(|word| word == "moment")) {
                let name = slug(&line.words()[1..].join(" "));
                if let Some(rank) = map.moments.iter().position(|(known, scene)| *known == name && *scene != at)
                    && !map.reached_from_afar.contains(&rank)
                {
                    map.reached_from_afar.push(rank);
                }
            }
        }
        map
    }

    /// The moment of that name: its rank, its scene.
    fn moment(&self, name: &str) -> Option<(usize, usize)> {
        self.moments.iter().position(|(known, _)| known == name).map(|rank| (rank, self.moments[rank].1))
    }
}

/// `go moment the bet`, `go scene beach`.
fn went(film: &mut Film, line: &Line) -> bool {
    let name = slug(&line.args.words("name").join(" "));
    if line.args.has("moment") {
        match film.story.moment(&name) {
            Some((_, scene)) if scene == film.story.at => film.beat(emit::goto(label(&name))),
            Some((rank, scene)) => {
                film.beat(emit::set(GOING, Value::Number(rank as f64 + 1.0)));
                leave_for(film, scene);
            }
            None => {
                let known: Vec<String> = film.story.moments.iter().map(|(name, _)| name.clone()).collect();
                let said = if known.is_empty() { "the film has none".to_string() } else { known.join(", ") };
                film.wrong(line.number, format!("“{}”: no moment “{name}” in the film (its moments: {said})", line.text));
            }
        }
        return true;
    }
    match film.story.scenes.iter().position(|place| *place == name) {
        Some(scene) => {
            if film.story.scenes.iter().filter(|place| **place == name).count() > 1 {
                film.remark(line.number, format!("“{}”: several scenes are in {name}: on to the first", line.text));
            }
            leave_for(film, scene);
        }
        None => film.wrong(line.number, format!("“{}”: no scene in “{name}” in the film (its scenes: {})", line.text, film.story.scenes.join(", "))),
    }
    true
}

/// The scene at hand fades out; that one opens.
fn leave_for(film: &mut Film, scene: usize) {
    film.beat(emit::fade(true, false, 30.0));
    film.beat(emit::wait_fade());
    film.beat(Beat::Flow(FlowBeat::GotoScene(format!("scene{}", scene + 1))));
}

/// A scene shows, set, faded in: counted; on to the moment of it another
/// scene went to, if one did.
pub(crate) fn scene_shows(film: &mut Film, at: usize) {
    film.story.at = at;
    if !film.story.used {
        return;
    }
    let var = seen_var(&film.story.scenes[at]);
    film.beat(emit::calc(var.clone(), Value::Var(var), "+", Value::Number(1.0)));
    let gone_to: Vec<usize> = film.story.reached_from_afar.iter().copied().filter(|rank| film.story.moments[*rank].1 == at).collect();
    for rank in gone_to {
        let asked = Test { any: vec![vec![Condition { var: GOING.to_string(), compare: Compare::Equal, value: Value::Number(rank as f64 + 1.0) }]] };
        film.beat(Beat::Flow(FlowBeat::If(asked)));
        film.beat(emit::set(GOING, Value::Number(0.0)));
        film.beat(emit::goto(label(&film.story.moments[rank].0)));
        film.beat(Beat::Flow(FlowBeat::EndIf));
    }
}

/// A moment starts (`moment: the bet`): its label, counted.
pub(crate) fn moment_starts(film: &mut Film, title: &str) {
    if !film.story.used {
        return;
    }
    let name = slug(title);
    film.beat(emit::label(label(&name)));
    let var = seen_var(&name);
    film.beat(emit::calc(var.clone(), Value::Var(var), "+", Value::Number(1.0)));
}
