//! The Story group: `go moment <name>`, `go scene <place>`, on to that
//! moment (its label, `_moment_<name>`; in another scene, that scene opened,
//! then on to it once it is set) or that scene from its start; and each
//! scene and moment counted as it plays, for `seen(<name>)`. Their names and
//! the film's map of them: `crate::plot`.

use engine::{Beat, Compare, Condition, FlowBeat, Test, Value};

use crate::emit;
use crate::film::Film;
use crate::plot::{StoryMap, seen_var, slug};
use crate::verbs::{Line, Word};

/// The words of the group: `go`.
pub const WORDS: &[Word] = &[Word::new(
    "go",
    &["moment name:text...", "scene name:word"],
    "on to another moment of the film (its label; in another scene, that scene, then the moment), or another scene from its start (gotoscene)",
)
.family("Story")
.ds_close("go moment: jump @label; go scene: one scene a film on DS", "a film is one scene after another on DS: “go scene” is not written")
.word_first()
.maybe(went)];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: crate::verbs::Group = crate::verbs::Group::new("story", WORDS);

/// The number a film holds which moment of another scene it goes to.
const GOING: &str = "_go_moment";

/// The label a moment starts at.
fn label(slug: &str) -> String {
    format!("_moment_{slug}")
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

impl StoryMap {
    /// The moment of that name: its rank, its scene.
    fn moment(&self, name: &str) -> Option<(usize, usize)> {
        self.moments.iter().position(|(known, _)| known == name).map(|rank| (rank, self.moments[rank].1))
    }
}
