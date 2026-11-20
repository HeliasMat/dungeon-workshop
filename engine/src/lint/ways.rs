//! That the ways through a file lead somewhere.

use super::*;

/// What a line opens that an end must close.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Open {
    If,
    While,
}

impl Open {
    fn word(self) -> &'static str {
        match self {
            Open::If => "if",
            Open::While => "while",
        }
    }
}

/// What a scene opens and goes to, as far as it was read.
#[derive(Default)]
struct Scene {
    open: Vec<(usize, Open)>,
    labels: HashSet<String>,
    gotos: Vec<(usize, String)>,
}

impl Scene {
    /// What it left open, or sent nowhere.
    fn end(&mut self, problems: &mut Vec<Diagnostic>) {
        for (number, open) in self.open.drain(..) {
            let kind = open.word();
            problems.push(Diagnostic::at(number, format!("“{kind}” without “end{kind}” in its scene")));
        }
        for (number, label) in self.gotos.drain(..).filter(|(_, label)| !self.labels.contains(label)) {
            problems.push(Diagnostic::at(number, format!("“goto {label}” without “label {label}” in its scene")));
        }
        self.labels.clear();
    }
}

/// Each `if` and `while` closed in its scene, each `goto` to a label of it.
pub(super) fn ways(reads: &[Read]) -> Vec<Diagnostic> {
    let (mut scene, mut problems) = (Scene::default(), Vec::new());
    for read in reads {
        let number = read.number;
        // (The word as written, for what is told.)
        let word = read.fields[0].as_str();
        match read.flow() {
            Some(Flow::Scene | Flow::Event) => scene.end(&mut problems),
            Some(Flow::If) => scene.open.push((number, Open::If)),
            Some(Flow::While) => scene.open.push((number, Open::While)),
            Some(Flow::Else | Flow::ElseIf) if scene.open.last().is_none_or(|(_, open)| *open != Open::If) => {
                problems.push(Diagnostic::at(number, format!("“{word}” without “if” before")));
            }
            Some(Flow::EndIf | Flow::EndWhile) => {
                let wanted = if matches!(read.flow(), Some(Flow::EndIf)) { Open::If } else { Open::While };
                if scene.open.pop().is_none_or(|(_, open)| open != wanted) {
                    problems.push(Diagnostic::at(number, format!("“{word}” without “{}” before", wanted.word())));
                }
            }
            Some(Flow::Label(label)) => _ = scene.labels.insert(label.to_string()),
            Some(Flow::OutOfWhile) if !scene.open.iter().any(|(_, open)| *open == Open::While) => {
                problems.push(Diagnostic::at(number, format!("“{word}” outside a “while”")));
            }
            Some(Flow::GoesTo(label)) => scene.gotos.push((number, label.to_string())),
            _ => {}
        }
    }
    scene.end(&mut problems);
    problems
}

/// Each question followed by its answers, each answer after a question.
pub(super) fn choices(reads: &[Read]) -> Vec<Diagnostic> {
    const NO_ANSWER: &str = "question without “answer” after";
    // (The question last asked, whether it was answered; the choices whose
    // answers may still come: each ask opens one, its end closes it.)
    let (mut asked, mut answered, mut open, mut problems) = (None, false, 0_usize, Vec::new());
    for read in reads {
        match read.flow() {
            Some(Flow::Answer) if open == 0 => problems.push(Diagnostic::at(
                read.number,
                "“answer” with no question open (the answers of an “ask” follow one another, and a single “end” closes them all)".to_string(),
            )),
            Some(Flow::Answer) => answered = true,
            Some(Flow::Ask) => {
                (asked, answered) = (Some(read.number), false);
                open += 1;
            }
            Some(Flow::EndAsk) => open = open.saturating_sub(1),
            Some(Flow::Event) => open = 0,
            Some(Flow::Scene) => {
                if let (Some(asked), false) = (asked, answered) {
                    problems.push(Diagnostic::at(asked, NO_ANSWER.to_string()));
                }
                asked = None;
            }
            _ => {}
        }
    }
    if let (Some(asked), false) = (asked, answered) {
        problems.push(Diagnostic::at(asked, NO_ANSWER.to_string()));
    }
    problems
}

/// Each `gotoscene` to a scene the file has.
pub(super) fn scenes_gone_to(reads: &[Read]) -> Vec<Diagnostic> {
    let names: HashSet<&str> = reads
        .iter()
        .filter_map(|read| match &read.is {
            Is::Scene(name) => name.as_deref(),
            _ => None,
        })
        .collect();
    let asked = reads.iter().filter_map(|read| match read.flow() {
        Some(Flow::GotoScene(name)) => Some((read.number, name)),
        _ => None,
    });
    asked
        // (`level:477`: the first scene of a place, as a film names it till it is compiled.)
        .filter(|(_, name)| !names.contains(name) && !name.starts_with("level:"))
        .map(|(number, name)| Diagnostic::at(number, format!("“gotoscene {name}”: no scene of this name (“scene <place> {name}”)")))
        .collect()
}
