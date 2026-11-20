//! That each line is one the engine can play, on someone who is there.

use super::*;
use crate::beat::Role;
use crate::who::Who;

/// The words of the lines read by what plays the story: one of them that
/// does not read is told by its own rule (`shape`), or there.
const OWN_LINES: [&str; 9] = ["event", "scene", "actor", "say", "timed", "ask", "answer", "end", "uses"];

/// What is wrong in the shape of a line that has its own rule.
fn shape(fields: &[String], words: &Vocabulary) -> Option<String> {
    let numeric = |at: usize| fields.get(at).is_some_and(|field| field.parse::<f32>().is_ok());
    Some(match fields[0].as_str() {
        "event" if !(numeric(1) && fields.len() >= 3) => "“event” wants a number and a name".to_string(),
        "scene" if !numeric(1) => "“scene” wants the number of a place".to_string(),
        // (Read into the line said before it, as the story is read.)
        "timed" if !numeric(1) => "“timed” wants a number of frames".to_string(),
        "say" if fields.len() < 4 => "“say” wants <who> <face> <text> (“-” for none)".to_string(),
        "say" if fields[2] != "-" && fields[2].parse::<u16>().is_err() && words.face(&fields[2]).is_none() => {
            format!("unknown face: “{}” (see the names in COMMANDS.md)", fields[2])
        }
        "ask" if fields.len() < 2 || !fields[1].contains('|') => "“ask” wants at least two answers separated by |".to_string(),
        "music" | "ambient" if fields.get(1).is_some_and(|name| !known_song(name, words)) => {
            format!("unknown music: “{}” (see the names in COMMANDS.md)", fields[1])
        }
        _ => return None,
    })
}

fn known_song(name: &str, words: &Vocabulary) -> bool {
    matches!(name, "-" | "back") || name.starts_with("own:") || name.parse::<u16>().is_ok() || words.song(name).is_some()
}

/// Who comes on the stage by a beat.
fn comes_on(beat: &Beat) -> Vec<Who> {
    beat.naming(Role::Comes)
}

/// Each line readable, each `said` after a line, everyone on the stage
/// before it acts.
pub(super) fn commands(reads: &[Read], words: &'static Vocabulary) -> Vec<Diagnostic> {
    let (mut put, mut said_line, mut problems) = (HashSet::new(), false, Vec::new());
    for read in reads {
        let number = read.number;
        let line = read.fields.join(" ");
        if let Some(message) = shape(&read.fields, words) {
            problems.push(Diagnostic::at(number, message));
        } else {
            match &read.is {
                // (What the story reads itself, a line said, a question and
                // its answers: read elsewhere.)
                Is::Event | Is::Scene(_) | Is::Story | Is::Beat(Beat::Course(_)) => {}
                Is::Wrong(_) if OWN_LINES.contains(&read.fields[0].as_str()) => {}
                Is::Wrong(error) | Is::Filled(Err(error)) => problems.push(Diagnostic::at(number, format!("{error}: {line}"))),
                // (Filled when played: who comes on by it, its numbers unknown yet.)
                Is::Filled(Ok(beat)) => put.extend(comes_on(beat).into_iter().filter(|_| !matches!(beat, Beat::Motion(_)))),
                Is::Beat(beat) => {
                    put.extend(comes_on(beat));
                    if matches!(beat, Beat::Speech(SpeechBeat::Said)) && !said_line {
                        problems.push(Diagnostic::at(number, "“said” without a line before".to_string()));
                    }
                    for who in beat.naming(Role::Acts).into_iter().filter(|who| !matches!(who, Who::Spot(_) | Who::Target) && !put.contains(who)) {
                        problems.push(Diagnostic::at(number, "").naming(name_of(who).words(" is not on stage yet (bring them in before: put, enter, appear)")));
                    }
                }
            }
        }
        said_line = match &read.is {
            Is::Beat(Beat::Course(Course::Say { .. })) => true,
            Is::Beat(Beat::Speech(SpeechBeat::Said)) | Is::Scene(_) => false,
            _ => said_line,
        };
    }
    problems
}

/// Someone of a scene file, as told in what is wrong (a Pokémon named typed:
/// `problem::telling`).
fn name_of(who: Who) -> diagnostics::Said {
    match who {
        Who::Item { item, .. } => format!("item i{item}").into(),
        Who::Thing { .. } => format!("thing {who}").into(),
        Who::Monster(_) | Who::Another { .. } => crate::problem::told!("{}", who),
        _ => format!("character {who}").into(),
    }
}
