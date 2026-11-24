//! A film's lines changed: what a line says split, its seconds stretched, a
//! line or a field written again.

use super::SECOND;
use super::timeline::speaker_of;

/// A line said, in its parts: who, the face, how it is said, what.
pub(super) fn said_parts(line: &str) -> Option<(String, String, String, String)> {
    let who = speaker_of(line)?;
    let rest = line.trim_start()[who.len()..].trim_start();
    let (inside, said) = match rest.strip_prefix('(') {
        Some(after) => {
            let (inside, after) = after.split_once(')')?;
            (inside.to_string(), after.trim_start().strip_prefix(':')?.trim().to_string())
        }
        None => (String::new(), rest.strip_prefix(':')?.trim().to_string()),
    };
    let faces: Vec<&str> = film::words::FACES.iter().map(|(word, _)| *word).collect();
    let words: Vec<&str> = inside.split(',').map(str::trim).filter(|word| !word.is_empty()).collect();
    let face = words.iter().find(|word| faces.contains(word)).map(|word| word.to_string()).unwrap_or_default();
    let manners: Vec<&str> = words.iter().filter(|word| **word != face).copied().collect();
    Some((who, face, manners.join(", "), said))
}

/// The text with line `number` (from 1) made another.
/// Whether a line says how long it lasts, or could (`wait 2s`, `… in 1.5s`, a walk).
pub(super) fn stretchable(words: &str) -> bool {
    seconds_word(words).is_some() || words.split_whitespace().take(2).any(|word| matches!(word, "walk" | "run"))
}

/// Where a line says a time in seconds (`2s`, `1.5s`): its word's index, the last one.
fn seconds_word(words: &str) -> Option<usize> {
    words
        .split_whitespace()
        .collect::<Vec<_>>()
        .iter()
        .rposition(|word| word.strip_suffix('s').is_some_and(|number| !number.is_empty() && number.parse::<f64>().is_ok()))
}

/// The seconds from a frame to another, a tenth at most, at least one.
pub(super) fn seconds_between(start: usize, stop: usize) -> f64 {
    ((stop.saturating_sub(start) as f64 / SECOND * 10.0).round() / 10.0).max(0.1)
}

/// The line lasting that long: its time written again, or `in Ns` added (before a `&`).
pub(super) fn with_seconds(line: &str, seconds: f64) -> String {
    let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
    let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    let said = if seconds.fract() == 0.0 { format!("{seconds:.0}s") } else { format!("{seconds:.1}s") };
    match seconds_word(line) {
        Some(at) => words[at] = said,
        None => {
            let at = words.iter().position(|word| word == "&").unwrap_or(words.len());
            words.insert(at, said);
            words.insert(at, "in".to_string());
        }
    }
    format!("{lead}{}", words.join(" "))
}

/// The text with its lines changed so (its last line ended as it was).
pub(super) fn edited_lines(text: &str, change: impl FnOnce(&mut Vec<String>)) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    change(&mut lines);
    lines.join("\n")
}

pub(super) fn with_line(text: &str, number: usize, made: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if let Some(line) = lines.get_mut(number.wrapping_sub(1)) {
        *line = made.to_string();
    }
    lines.join("\n") + "\n"
}

/// The text with a field of a line, as the film read it, made these words.
pub fn with_field(text: &str, number: usize, read: &film::ReadAs, name: &str, words: &[String]) -> Result<String, String> {
    let line = text.lines().nth(number.wrapping_sub(1)).unwrap_or_default();
    let edit = read.edit(number, line, name, words)?;
    let mut doc = film::doc::Doc::read(text);
    doc.apply(&edit)?;
    Ok(doc.typed())
}
