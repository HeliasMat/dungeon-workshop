//! Who is in a scene: its pair, the voices near a line, who answers, its
//! cast, someone here.

use super::highlight::speaker;
use super::text::spot_of;

/// The first two of the scene a place is in (else Pikachu and Piplup): a model's `a` and `b`.
pub(super) fn scene_pair(text: &str, at: usize) -> (String, String) {
    let cast = scene_cast(text, spot_of(text, at).line);
    (cast.first().cloned().unwrap_or_else(|| "pikachu".to_string()), cast.get(1).cloned().unwrap_or_else(|| "piplup".to_string()))
}

/// Those who may speak in the scene a line is in (from 0): those who speak in it, as they first do, then the
/// others of its `scene:` line.
pub(super) fn voices_near(text: &str, line: usize) -> Vec<String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let opens = |at: usize| lines.get(at).is_some_and(|line| line.trim_start().starts_with("scene:"));
    let first = (0..=line.min(lines.len().saturating_sub(1))).rev().find(|at| opens(*at)).unwrap_or(0);
    let last = (line + 1..lines.len()).find(|at| opens(*at)).unwrap_or(lines.len());
    let mut voices: Vec<String> = Vec::new();
    for said in &lines[first..last] {
        if let Some((who, _)) = speaker(said)
            && !voices.iter().any(|voice| voice == who)
        {
            voices.push(who.to_string());
        }
    }
    for who in scene_cast(text, line + 1) {
        if !voices.contains(&who) {
            voices.push(who);
        }
    }
    voices
}

/// Who answers someone on a line (from 0): the last other who spoke before it in the scene, else the first other.
pub(super) fn other_voice(text: &str, line: usize, who: &str, voices: &[String]) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    for said in lines[..line.min(lines.len())].iter().rev() {
        if said.trim_start().starts_with("scene:") {
            break;
        }
        if let Some((other, _)) = speaker(said)
            && other != who
            && other != "narrator"
        {
            return other.to_string();
        }
    }
    voices.iter().find(|voice| *voice != who && *voice != "narrator").cloned().unwrap_or_else(|| who.to_string())
}

/// Those named on the `scene:` line a line is under.
pub(super) fn scene_cast(text: &str, line: usize) -> Vec<String> {
    text.split('\n')
        .take(line)
        .filter_map(|words| words.trim_start().strip_prefix("scene:"))
        .last()
        .map(|cast| cast.split_whitespace().skip(1).map(str::to_string).collect())
        .unwrap_or_default()
}

/// A model's line with `a` and `b` made those of the scene (whole words).
pub(super) fn with_cast(line: &str, a: &str, b: &str) -> String {
    line.split(' ')
        .map(|word| {
            let (core, rest) = word.split_at(word.find(|letter: char| !(letter.is_alphanumeric() || letter == '_')).unwrap_or(word.len()));
            match core {
                "a" => format!("{a}{rest}"),
                "b" => format!("{b}{rest}"),
                _ => word.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Someone of the scene where the cursor is (the first named on its `scene:` line), to begin a line with.
pub(super) fn someone_here(text: &str, cursor: usize) -> String {
    let line = spot_of(text, cursor).line;
    text.split('\n')
        .take(line)
        .filter_map(|words| words.trim_start().strip_prefix("scene:"))
        .last()
        .and_then(|cast| cast.split_whitespace().nth(1))
        .unwrap_or("who")
        .to_string()
}
