//! The Timeline's tracks: one for each of the cast (their lines said, their
//! gestures, their moves), then the camera, the sound and the music, the
//! screen, and the rest (waits, objects…). Which track a line goes on, and
//! the words its bar shows there.

use super::edits::is_beside;
use super::lines::said_parts;
use super::timeline::{kind_of_line, speaker_of};

/// A track of the Timeline.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum Lane {
    Who(String),
    Camera,
    Sound,
    Screen,
    Other,
}

impl Lane {
    /// The tracks that are not of the cast, in their order.
    pub(super) const OTHERS: [Lane; 4] = [Lane::Camera, Lane::Sound, Lane::Screen, Lane::Other];

    /// Its name on the left.
    pub(super) fn name(&self) -> String {
        match self {
            Lane::Who(who) => who.clone(),
            Lane::Camera => "Camera".to_string(),
            Lane::Sound => "Sound · Music".to_string(),
            Lane::Screen => "Screen".to_string(),
            Lane::Other => "Waits · Other".to_string(),
        }
    }

    /// Who acts on it, if it is one of the cast's.
    pub(super) fn who(&self) -> Option<&str> {
        match self {
            Lane::Who(who) => Some(who),
            _ => None,
        }
    }
}

/// The first words of lines that change what the screen shows.
const SCREEN: [&str; 14] =
    ["transition", "title", "curtain", "iris", "wipe", "flash", "white", "black", "tint", "letterbox", "screen", "weather", "darken", "credits"];

/// Who does a line: who says it, or the one of the cast it is written of
/// first (`pikachu hop`), or the `who` of the form it was read as (`walk pikachu to …`).
pub(super) fn performer_of(line: &str, read: Option<&film::ReadAs>, cast: &[String]) -> Option<String> {
    let known = |word: &str| cast.iter().any(|one| one == word);
    if let Some(who) = speaker_of(line).filter(|who| known(who)) {
        return Some(who);
    }
    let first: String = line.trim_start().chars().take_while(|letter| letter.is_alphanumeric() || *letter == '_').collect();
    if known(&first) {
        return Some(first);
    }
    let read = read?;
    read.values
        .iter()
        .filter(|value| value.name == "who" || value.kind == film::grammar::Kind::Someone)
        .flat_map(|value| value.words.first())
        .find(|word| known(word))
        .cloned()
}

/// The track a line goes on.
pub(super) fn lane_of(line: &str, read: Option<&film::ReadAs>, cast: &[String]) -> Lane {
    if let Some(who) = performer_of(line, read, cast) {
        return Lane::Who(who);
    }
    let first = line.split_whitespace().next().unwrap_or_default();
    match kind_of_line(line) {
        3 => Lane::Camera,
        2 => Lane::Sound,
        _ if SCREEN.contains(&first) => Lane::Screen,
        _ => Lane::Other,
    }
}

/// What a line's bar shows on its track: without the name of whose track it
/// is (`hop happy`, `“Whoa...”`), `∥` in front when it runs beside the film.
pub(super) fn bar_words(line: &str, lane: &Lane) -> String {
    let line = line.trim();
    let shown = match (lane, said_parts(line)) {
        (Lane::Who(_), Some((_, face, _, said))) => {
            let said = said.trim_end_matches('&').trim_end();
            if face.is_empty() { format!("“{said}”") } else { format!("({face}) “{said}”") }
        }
        (Lane::Who(who), None) => {
            let plain = if is_beside(line) { line.trim_end_matches('&').trim_end() } else { line };
            plain.strip_prefix(who.as_str()).map(str::trim_start).filter(|rest| !rest.is_empty()).unwrap_or(plain).to_string()
        }
        _ => line.trim_end_matches('&').trim_end().to_string(),
    };
    if is_beside(line) { format!("∥ {shown}") } else { shown }
}

/// The tracks of the cast, in order: the film's cast, then whoever else comes on the stage or does a line.
pub(super) fn cast_lanes(cast: &[String], on_stage: &[String], doing: &[String]) -> Vec<String> {
    let mut lanes: Vec<String> = Vec::new();
    for who in cast.iter().chain(on_stage).chain(doing) {
        if !lanes.contains(who) {
            lanes.push(who.clone());
        }
    }
    lanes
}
