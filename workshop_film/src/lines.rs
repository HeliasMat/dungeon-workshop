//! A film's lines changed: what a line says split, a line or a field
//! written again.

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

/// The text with its lines changed so (its last line ended as it was).
pub(super) fn edited_lines(text: &str, change: impl FnOnce(&mut Vec<String>)) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    change(&mut lines);
    lines.join("\n")
}

/// The text with line `number` (from 1) made another.
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
