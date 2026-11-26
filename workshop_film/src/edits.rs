//! What the Timeline's drags and inserts write in the film's text, each a
//! whole new text (one change to undo, through `Script::set_text`):
//! - a bar moved in time: the `wait` before it longer or shorter (or one
//!   written, or taken away), the one after it the other way so the rest
//!   stays (unless the rest is to move with it); a line beside the film
//!   (`&`) taken from where it is and written where it then starts;
//! - a line made to run alongside (`&`) or back in sequence;
//! - a line's time (`in 2s`, `wait 1s`, `18f`) written again;
//! - who does a line changed;
//! - a line written at a time: after the line playing then, a `wait` to
//!   reach it (a wait playing then split in two), or beside the film.

use super::SECOND;

/// The Timeline's step in time: a tenth of a second, in frames.
pub(super) const STEP: usize = 6;

/// A line of the film's own course as it played the first time: its number
/// (from 1), its start and end, in frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Course {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

/// The film's own course (not the lines beside it), each line once (the first time it played), in the order of the text.
pub(super) fn course_of(timeline: &preview::played::Timeline) -> Vec<Course> {
    let mut course: Vec<Course> = Vec::new();
    for span in timeline.spans.iter().filter(|span| span.label.is_none() && span.thread == 0 && span.number > 0) {
        let line = span.number as usize;
        if !course.iter().any(|known| known.line == line) {
            course.push(Course { line, start: span.start, end: span.end.max(span.start) });
        }
    }
    course.sort_by_key(|known| known.line);
    course
}

/// A time written in a film (`2s`, `1.5s`, `18f`), in frames.
pub(super) fn frames_of(word: &str) -> Option<usize> {
    if let Some(number) = word.strip_suffix('f') {
        return number.parse::<usize>().ok();
    }
    let seconds: f64 = word.strip_suffix('s')?.parse().ok()?;
    (seconds >= 0.0 && seconds.is_finite()).then(|| (seconds * SECOND).round() as usize)
}

/// So many frames as a film writes them: seconds when a whole tenth (`1s`, `1.5s`), else frames (`18f`).
pub(super) fn time_said(frames: usize) -> String {
    if frames % STEP == 0 {
        let seconds = frames as f64 / SECOND;
        if seconds.fract() == 0.0 { format!("{seconds:.0}s") } else { format!("{seconds:.1}s") }
    } else {
        format!("{frames}f")
    }
}

/// So many frames to the nearest step.
pub(super) fn stepped(frames: i64) -> i64 {
    (frames as f64 / STEP as f64).round() as i64 * STEP as i64
}

/// A line's lead (its spaces in front).
fn lead(line: &str) -> String {
    line.chars().take_while(|letter| letter.is_whitespace()).collect()
}

/// Whether a line is blank or a remark.
fn quiet(line: &str) -> bool {
    let line = line.trim();
    line.is_empty() || line.starts_with('#')
}

/// How long a plain `wait 2s` waits, in frames.
pub(super) fn wait_of(line: &str) -> Option<usize> {
    match line.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["wait", time] => frames_of(time),
        _ => None,
    }
}

/// Whether a line opens a scene, a moment or the title.
fn header(line: &str) -> bool {
    let line = line.trim_start();
    ["scene:", "moment:", "title:"].iter().any(|head| line.starts_with(head))
}

/// The line written before that one (by index), past blanks and remarks.
fn written_before(lines: &[String], at: usize) -> Option<usize> {
    (0..at).rev().find(|index| !quiet(&lines[*index]))
}

/// The line written after that one (by index), past blanks and remarks.
fn written_after(lines: &[String], at: usize) -> Option<usize> {
    (at + 1..lines.len()).find(|index| !quiet(&lines[*index]))
}

/// Where a line goes written after that one (by index): past the block it opens (its lines further in, its `end`).
fn after_block(lines: &[String], at: usize) -> usize {
    let own = lead(&lines[at]).len();
    let mut next = at + 1;
    while let Some(index) = written_after(lines, next - 1) {
        if lead(&lines[index]).len() > own {
            next = index + 1;
        } else {
            if next > at + 1 && lines[index].trim() == "end" && lead(&lines[index]).len() == own {
                next = index + 1;
            }
            break;
        }
    }
    next
}

/// The text's lines, and the text again.
fn split(text: &str) -> Vec<String> {
    text.split('\n').map(str::to_string).collect()
}

/// Whether a line runs beside the film (`… &`, `… & as name`).
pub(super) fn is_beside(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    words.last() == Some(&"&") || (words.len() >= 3 && words[words.len() - 3] == "&" && words[words.len() - 2] == "as")
}

/// A line without its `&` (or `& as name`).
fn without_beside(line: &str) -> String {
    let mut words: Vec<&str> = line.split_whitespace().collect();
    if words.last() == Some(&"&") {
        words.pop();
    } else if words.len() >= 3 && words[words.len() - 3] == "&" && words[words.len() - 2] == "as" {
        words.truncate(words.len() - 3);
    }
    format!("{}{}", lead(line), words.join(" "))
}

/// The text with that line (from 1) moved by so many frames (to a step): the
/// `wait` before it longer or shorter (taken away at nothing, written if
/// there was none), the `wait` after it the other way so the lines after it
/// stay where they are (`ripple`: they move with it). The text, the
/// line's number then (a wait written or taken away before it), what was done.
pub(super) fn retimed(text: &str, number: usize, by: i64, ripple: bool) -> Result<(String, usize, String), String> {
    let mut lines = split(text);
    let at = number.checked_sub(1).filter(|at| *at < lines.len() && !quiet(&lines[*at])).ok_or_else(|| format!("line {number}: no line there"))?;
    let by = stepped(by);
    if by == 0 {
        return Err(format!("line {number}: not moved"));
    }
    let own = lead(&lines[at]);
    let same = |index: &usize, lines: &[String]| lead(&lines[*index]) == own && wait_of(&lines[*index]).is_some();
    let before = written_before(&lines, at).filter(|index| same(index, &lines));
    let after = written_after(&lines, at).filter(|index| same(index, &lines));
    let moved = match before {
        Some(index) => by.max(-(wait_of(&lines[index]).unwrap_or(0) as i64)),
        None if by > 0 => by,
        None => return Err(format!("line {number}: no wait before it to shorten")),
    };
    if moved == 0 {
        return Err(format!("line {number}: it starts as early as it can"));
    }
    // (After it first, then before: the indices before stay right.)
    if let (false, Some(index)) = (ripple, after) {
        let left = wait_of(&lines[index]).unwrap_or(0) as i64 - moved;
        if left > 0 {
            lines[index] = format!("{own}wait {}", time_said(left as usize));
        } else {
            lines.remove(index);
        }
    } else if !ripple && moved < 0 {
        // (Earlier, with no wait after it: one, so that the rest stays.)
        lines.insert(at + 1, format!("{own}wait {}", time_said((-moved) as usize)));
    }
    let now = match before {
        Some(index) => {
            let left = wait_of(&lines[index]).unwrap_or(0) as i64 + moved;
            if left > 0 {
                lines[index] = format!("{own}wait {}", time_said(left as usize));
                number
            } else {
                lines.remove(index);
                number - 1
            }
        }
        None => {
            lines.insert(at, format!("{own}wait {}", time_said(moved as usize)));
            number + 1
        }
    };
    let way = if moved > 0 { "later" } else { "earlier" };
    Ok((lines.join("\n"), now, format!("line {now}: {} {way}", time_said(moved.unsigned_abs() as usize))))
}

/// The text with that line made to run beside the film (`&` put at its end),
/// or back in sequence (its `&` taken away). The text, whether it now runs beside.
pub(super) fn toggled_beside(text: &str, number: usize) -> Result<(String, bool), String> {
    let mut lines = split(text);
    let at = number.checked_sub(1).filter(|at| *at < lines.len() && !quiet(&lines[*at])).ok_or_else(|| format!("line {number}: no line there"))?;
    let line = lines[at].clone();
    let first = line.split_whitespace().next().unwrap_or_default();
    if header(&line) || matches!(first, "wait" | "beside" | "end" | "if" | "else" | "while" | "repeat" | "for" | "when" | "every" | "after" | "at" | "let") {
        return Err(format!("line {number}: \"{first}\" runs in sequence only"));
    }
    let beside = is_beside(&line);
    lines[at] = if beside { without_beside(&line) } else { format!("{} &", line.trim_end()) };
    Ok((lines.join("\n"), !beside))
}

/// Where a line says how long it lasts (`2s`, `1.5s`, `18f`): its word's index, the last one.
fn time_word(words: &[&str]) -> Option<usize> {
    words.iter().rposition(|word| frames_of(word).is_some() && word.len() > 1)
}

/// Whether a line says how long it lasts, or could (`wait 2s`, `… in 1.5s`, a walk).
pub(super) fn stretchable(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    time_word(&words).is_some() || words.iter().take(2).any(|word| matches!(*word, "walk" | "run"))
}

/// The line lasting so many frames: its time written again (in frames if it
/// was in frames, else in seconds to a tenth), or `in Ns` added (before a `&`).
pub(super) fn with_duration(line: &str, frames: usize) -> String {
    let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    let borrowed: Vec<&str> = words.iter().map(String::as_str).collect();
    match time_word(&borrowed) {
        Some(at) if words[at].ends_with('f') => words[at] = format!("{}f", frames.max(1)),
        Some(at) => words[at] = time_said(stepped(frames as i64).max(STEP as i64) as usize),
        None => {
            let at = words.iter().position(|word| word == "&").unwrap_or(words.len());
            words.insert(at, time_said(stepped(frames as i64).max(STEP as i64) as usize));
            words.insert(at, "in".to_string());
        }
    }
    format!("{}{}", lead(line), words.join(" "))
}

/// The text with someone else doing that line: the name it starts with (a
/// line said, `who hop`), else the field `who` of the form that read it.
pub(super) fn with_performer(text: &str, number: usize, read: Option<&film::ReadAs>, from: &str, to: &str) -> Result<String, String> {
    let mut lines = split(text);
    let at = number.checked_sub(1).filter(|at| *at < lines.len()).ok_or_else(|| format!("line {number}: no line there"))?;
    let line = lines[at].clone();
    let rest = line.trim_start();
    let first: String = rest.chars().take_while(|letter| letter.is_alphanumeric() || *letter == '_').collect();
    if first == from {
        lines[at] = format!("{}{to}{}", lead(&line), &rest[first.len()..]);
        return Ok(lines.join("\n"));
    }
    if let Some(read) = read
        && read.values.iter().any(|value| value.name == "who" && value.words.iter().any(|word| word == from))
    {
        return super::lines::with_field(text, number, read, "who", &[to.to_string()]);
    }
    Err(format!("line {number}: who does it is not written so that it can change"))
}

/// The course's line playing at that frame (where a line written then goes):
/// the last begun by then (of those beginning together, the last written).
fn playing_at(course: &[Course], at: usize, lines: &[String], headers: bool) -> Option<Course> {
    course
        .iter()
        .filter(|known| known.start <= at && known.line <= lines.len() && !quiet(&lines[known.line - 1]) && (headers || !header(&lines[known.line - 1])))
        .max_by_key(|known| (known.start, known.line))
        .copied()
}

/// The lead of a line written after that one: its own, or the next line's after a header.
fn lead_after(lines: &[String], at: usize) -> String {
    if header(&lines[at]) {
        return written_after(lines, at).filter(|index| !header(&lines[*index])).map(|index| lead(&lines[index])).unwrap_or_default();
    }
    lead(&lines[at])
}

/// The text with a line written to start at that frame, in sequence: after
/// the line playing then (past its block), a `wait` to reach it; a wait
/// playing then split in two around it. The text, the new line's number, what was done.
pub(super) fn inserted_in_sequence(text: &str, course: &[Course], at: usize, words: &str) -> Result<(String, usize, String), String> {
    let mut lines = split(text);
    let playing = playing_at(course, at, &lines, true).ok_or_else(|| "nothing plays there yet".to_string())?;
    let index = playing.line - 1;
    let own = lead_after(&lines, index);
    if let Some(waits) = wait_of(&lines[index]) {
        let first = stepped(at as i64 - playing.start as i64).max(0) as usize;
        if first > 0 && first < waits {
            let made = [format!("{own}wait {}", time_said(first)), format!("{own}{words}"), format!("{own}wait {}", time_said(waits - first))];
            lines.splice(index..=index, made);
            return Ok((lines.join("\n"), index + 2, format!("line {}: {words}, the wait of line {} split", index + 2, playing.line)));
        }
    }
    let after = after_block(&lines, index);
    let gap = stepped(at as i64 - playing.end as i64);
    let mut made = Vec::new();
    if gap > 0 {
        made.push(format!("{own}wait {}", time_said(gap as usize)));
    }
    made.push(format!("{own}{words}"));
    let count = made.len();
    lines.splice(after..after, made);
    let number = after + count;
    let said = if gap > 0 || playing.end <= at + STEP / 2 {
        format!("line {number}: {words}")
    } else {
        format!("line {number}: {words} (when line {} ends)", playing.line)
    };
    Ok((lines.join("\n"), number, said))
}

/// The text with a line written to start at that frame beside the film
/// (nothing else moved): before the line starting then, `… &`; later than a
/// line starts, in a `beside` block that waits until then. The text, the new line's number.
pub(super) fn inserted_beside(text: &str, course: &[Course], at: usize, words: &str) -> Result<(String, usize), String> {
    let mut lines = split(text);
    let playing = playing_at(course, at, &lines, false).ok_or_else(|| "nothing plays there yet".to_string())?;
    let index = playing.line - 1;
    let own = lead(&lines[index]);
    let words = without_beside(words.trim());
    let wait = stepped(at as i64 - playing.start as i64).max(0) as usize;
    if wait == 0 {
        lines.insert(index, format!("{own}{} &", words.trim()));
        return Ok((lines.join("\n"), index + 1));
    }
    let made = [format!("{own}beside"), format!("{own}  wait {}", time_said(wait)), format!("{own}  {}", words.trim()), format!("{own}end")];
    lines.splice(index..index, made);
    Ok((lines.join("\n"), index + 3))
}

/// The text with a line beside the film (`… &`) moved to start at that
/// frame: taken from where it is, written where it then starts (beside).
pub(super) fn moved_beside(text: &str, course: &[Course], number: usize, at: usize) -> Result<(String, usize), String> {
    let mut lines = split(text);
    let index = number.checked_sub(1).filter(|index| *index < lines.len()).ok_or_else(|| format!("line {number}: no line there"))?;
    let line = lines.remove(index);
    let course: Vec<Course> = course
        .iter()
        .filter(|known| known.line != number)
        .map(|known| Course { line: if known.line > number { known.line - 1 } else { known.line }, ..*known })
        .collect();
    inserted_beside(&lines.join("\n"), &course, at, &line)
}

/// A line to write for an asset dropped on someone's track (none: on the track of no one).
pub(super) fn asset_line(asset: &workshop_base::asset::Asset, who: Option<&str>) -> Option<String> {
    use workshop_base::asset::Asset;
    match (asset, who) {
        (Asset::Music { word }, _) => Some(format!("music {word}")),
        (Asset::Sound { word }, _) => Some(format!("sound {word}")),
        (Asset::Gesture { words, .. }, Some(who)) => Some(format!("{who} {words}")),
        (Asset::Face { face, .. }, Some(who)) => Some(format!("{who} ({face}): ...")),
        (Asset::Thing { word, held: true }, Some(who)) => Some(format!("{who} hold {word}")),
        (Asset::Effect { name }, Some(who)) => Some(format!("{who} use {name}")),
        _ => None,
    }
}
