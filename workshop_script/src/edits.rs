//! The text changed: a range replaced, lines changed each, marks shifted,
//! lines wrapped in a block or made a recipe, what is typed at each cursor;
//! what the keys do to lines (gone, chosen, commented, doubled, moved,
//! indented, cut, a new one begun).

use std::collections::BTreeSet;
use std::ops::Range;

use film::service::Spot;

use super::completion::wants_fin;
use super::text::{byte_of, index_of, line_at};

/// The text with letters `range` made `with`; where the cursor goes.
pub(super) fn replaced(text: &str, range: Range<usize>, with: &str) -> (String, usize) {
    let (start, end) = (byte_of(text, range.start), byte_of(text, range.end));
    (format!("{}{with}{}", &text[..start], &text[end..]), range.start + with.chars().count())
}

/// The lines of a selection (from 0), the first and the last.
pub(super) fn lines_of(text: &str, selection: (usize, usize)) -> (usize, usize) {
    let (a, b) = (selection.0.min(selection.1), selection.0.max(selection.1));
    (line_at(text, a).0, line_at(text, b).0)
}

/// Those lines changed each by `change`; the selection kept on them.
pub(super) fn each_line(text: &str, (first, last): (usize, usize), change: impl Fn(&str) -> String) -> String {
    text.split('\n').enumerate().map(|(at, line)| if (first..=last).contains(&at) { change(line) } else { line.to_string() }).collect::<Vec<_>>().join("\n")
}

/// Lines marked (from 1) moved as the text changed: those after what changed
/// by as many lines as came or went; those inside it kept if they still are.
pub(super) fn shifted(old: &str, new: &str, marks: &mut BTreeSet<usize>) {
    if marks.is_empty() {
        return;
    }
    let (before, after): (Vec<&str>, Vec<&str>) = (old.split('\n').collect(), new.split('\n').collect());
    let same_start = before.iter().zip(&after).take_while(|(a, b)| a == b).count();
    let same_end = before.iter().rev().zip(after.iter().rev()).take_while(|(a, b)| a == b).count().min(before.len().min(after.len()) - same_start);
    let shift = after.len() as i64 - before.len() as i64;
    *marks = marks
        .iter()
        .filter_map(|line| {
            if *line <= same_start {
                Some(*line)
            } else if *line > before.len() - same_end {
                Some((*line as i64 + shift).max(1) as usize)
            } else {
                (*line <= after.len() - same_end).then_some(*line)
            }
        })
        .collect();
}

/// The lines (0-based, from first to last) put in a block: its word over them, `end` under.
pub(super) fn wrapped_in(text: &str, (first, last): (usize, usize), opener: &str) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let last = last.min(lines.len() - 1);
    let lead: String = lines[first].chars().take_while(|letter| letter.is_whitespace()).collect();
    for line in &mut lines[first..=last] {
        if !line.trim().is_empty() {
            *line = format!("  {line}");
        }
    }
    lines.insert(last + 1, format!("{lead}end"));
    lines.insert(first, format!("{lead}{opener}"));
    lines.join("\n")
}

/// The lines made a recipe of their own (`define recipeN` before the first
/// scene), called where they were.
pub(super) fn made_recipe(text: &str, (first, last): (usize, usize), name: &str) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let last = last.min(lines.len() - 1);
    let body: Vec<String> = lines[first..=last].to_vec();
    let lead: String = lines[first].chars().take_while(|letter| letter.is_whitespace()).collect();
    lines.splice(first..=last, [format!("{lead}{name}")]);
    let common = body.iter().filter(|line| !line.trim().is_empty()).map(|line| line.len() - line.trim_start().len()).min().unwrap_or(0);
    let mut block = vec![format!("define {name}")];
    block.extend(body.iter().map(|line| if line.trim().is_empty() { String::new() } else { format!("  {}", &line[common..]) }));
    block.push("end".to_string());
    block.push(String::new());
    let at = lines.iter().position(|line| film::lines::scene_head(line).is_some()).unwrap_or(0);
    lines.splice(at..at, block);
    lines.join("\n")
}

/// The film with `uses <name>` before its first scene (after its other
/// `uses` and its title), if it has no such line: the text, the letter the
/// line was put at, how many letters it took.
pub(super) fn with_uses(text: &str, name: &str) -> Option<(String, usize, usize)> {
    let uses = format!("uses {name}");
    let mut film: Vec<&str> = text.split('\n').collect();
    if film.iter().any(|line| line.trim() == uses) {
        return None;
    }
    let first_scene = film.iter().position(|line| film::lines::scene_head(line).is_some()).unwrap_or(0);
    let after_head =
        film[..first_scene].iter().rposition(|line| line.trim_start().starts_with("uses ") || line.trim_start().starts_with("title")).map_or(0, |at| at + 1);
    let at: usize = film[..after_head].iter().map(|line| line.chars().count() + 1).sum();
    film.insert(after_head, &uses);
    Some((film.join("\n"), at, uses.chars().count() + 1))
}

/// The lines moved to a recipe file of the library: the film with them
/// called where they were and `uses name` at its head; the recipe file's
/// text, its example the scene they were in, calling it.
pub(super) fn library_recipe(text: &str, (first, last): (usize, usize), name: &str, film_name: &str) -> (String, String) {
    let lines: Vec<&str> = text.split('\n').collect();
    let last = last.min(lines.len() - 1);
    let body = &lines[first..=last];
    let common = body.iter().filter(|line| !line.trim().is_empty()).map(|line| line.len() - line.trim_start().len()).min().unwrap_or(0);
    let scene = lines[..=first]
        .iter()
        .rev()
        .find_map(|line| film::lines::scene_head(line))
        .map(|rest| rest.split('#').next().unwrap_or_default().trim())
        .filter(|rest| !rest.is_empty())
        .unwrap_or("town");
    let mut recipe = format!("# Lines {} to {} of {film_name}.\ndefine {name}\n", first + 1, last + 1);
    for line in body {
        recipe.push_str(&if line.trim().is_empty() { String::new() } else { format!("  {}", &line[common..]) });
        recipe.push('\n');
    }
    recipe.push_str(&format!("end\n\nexample: {scene}\n{name}\n"));
    // (The film: the lines called by the recipe's name, the file used before the first scene.)
    let mut film: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    let lead: String = lines[first].chars().take_while(|letter| letter.is_whitespace()).collect();
    film.splice(first..=last, [format!("{lead}{name}")]);
    let film = film.join("\n");
    (with_uses(&film, name).map_or(film, |(made, ..)| made), recipe)
}

/// What is typed at each cursor: words (a letter, what is pasted, a new line), or a letter taken back.
pub(super) enum Typed {
    Words(String),
    Back,
}

/// The text with that typed at each cursor; the cursors where they are then (in the same order).
pub(super) fn typed_at_each(text: &str, cursors: &[usize], typed: &Typed) -> (String, Vec<usize>) {
    let letters: Vec<char> = text.chars().collect();
    let mut order: Vec<usize> = (0..cursors.len()).collect();
    order.sort_by_key(|at| cursors[*at]);
    let mut made = String::new();
    let mut moved = vec![0; cursors.len()];
    let mut from = 0;
    let mut shift: i64 = 0;
    for at in order {
        let cursor = cursors[at].min(letters.len());
        if cursor < from {
            moved[at] = (cursor as i64 + shift).max(0) as usize;
            continue;
        }
        match typed {
            Typed::Words(words) => {
                made.extend(&letters[from..cursor]);
                made.push_str(words);
                shift += words.chars().count() as i64;
                from = cursor;
            }
            Typed::Back => {
                let back = cursor.saturating_sub(1).max(from);
                made.extend(&letters[from..back]);
                shift -= (cursor - back) as i64;
                from = cursor;
            }
        }
        moved[at] = (cursor as i64 + shift).max(0) as usize;
    }
    made.extend(&letters[from.min(letters.len())..]);
    (made, moved)
}

/// The letters chosen (between the two ends of a selection, in either order).
pub(super) fn chosen_letters(text: &str, selection: (usize, usize)) -> String {
    text.chars().skip(selection.0.min(selection.1)).take(selection.0.abs_diff(selection.1)).collect()
}

/// The lines from `first` to `last` (from 0) gone (Ctrl+Shift+K); the cursor at the start of the one after.
pub(super) fn lines_gone(text: &str, (first, last): (usize, usize)) -> (String, usize) {
    let mut lines: Vec<&str> = text.split('\n').collect();
    lines.drain(first..=last.min(lines.len() - 1));
    let changed = lines.join("\n");
    let at = index_of(&changed, Spot { line: first + 1, column: 1 });
    (changed, at)
}

/// Those lines chosen whole (Ctrl+L): the selection from after the last to the start of the first.
pub(super) fn lines_chosen(text: &str, (first, last): (usize, usize)) -> (usize, usize) {
    let start = index_of(text, Spot { line: first + 1, column: 1 });
    let (_, line_start, line) = line_at(text, index_of(text, Spot { line: last + 1, column: 1 }));
    let end = (line_start + line.chars().count() + 1).min(text.chars().count());
    (end, start)
}

/// Those lines commented out (`# ` after their indent), or taken back out of
/// their comment if they all were (Ctrl+/); the empty ones kept.
pub(super) fn comments_toggled(text: &str, (first, last): (usize, usize)) -> String {
    let all_commented = text.split('\n').skip(first).take(last - first + 1).all(|line| line.trim_start().starts_with('#') || line.trim().is_empty());
    each_line(text, (first, last), |line| {
        if line.trim().is_empty() {
            line.to_string()
        } else if all_commented {
            line.replacen("# ", "", 1).replacen('#', "", 1)
        } else {
            let lead = line.len() - line.trim_start().len();
            format!("{}# {}", &line[..lead], &line[lead..])
        }
    })
}

/// The cursor's line written again under it (Ctrl+D); the cursor gone down with it.
pub(super) fn line_doubled(text: &str, cursor: usize) -> (String, usize) {
    let (_, start, line) = line_at(text, cursor);
    let at = start + line.chars().count();
    let (changed, _) = replaced(text, at..at, &format!("\n{line}"));
    (changed, cursor + line.chars().count() + 1)
}

/// Those lines moved one up or down, past their neighbour (Alt+↑ ↓); the
/// selection moved with them; nothing if they are at the edge.
pub(super) fn lines_moved(text: &str, (first, last): (usize, usize), up: bool, selection: (usize, usize)) -> Option<(String, (usize, usize))> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    let (from, to) = if up { (first.checked_sub(1), last) } else { (Some(first), last + 1) };
    let from = from.filter(|_| to < lines.len())?;
    let shift = if up { -1i64 } else { 1 };
    let block: Vec<&str> = lines[first..=last].to_vec();
    let other = if up { lines[from] } else { lines[to] };
    let mut moved: Vec<&str> = Vec::new();
    if up {
        moved.extend(&block);
        moved.push(other);
    } else {
        moved.push(other);
        moved.extend(&block);
    }
    lines.splice(from.min(first)..=to.max(last), moved);
    let changed = lines.join("\n");
    let step = |at: usize| (at as i64 + shift * (other.chars().count() as i64 + 1)).max(0) as usize;
    Some((changed, (step(selection.0), step(selection.1))))
}

/// Tab: two spaces at the cursor, or before each of the lines chosen; the selection after them.
pub(super) fn tabbed(text: &str, (first, last): (usize, usize), selection: (usize, usize)) -> (String, (usize, usize)) {
    if first == last && selection.0 == selection.1 {
        let (changed, at) = replaced(text, selection.0..selection.0, "  ");
        (changed, (at, at))
    } else {
        (each_line(text, (first, last), |line| format!("  {line}")), (selection.0 + 2, selection.1 + 2))
    }
}

/// Shift+Tab: two spaces (or one) taken from before each of the lines chosen.
pub(super) fn untabbed(text: &str, (first, last): (usize, usize), selection: (usize, usize)) -> (String, (usize, usize)) {
    let changed = each_line(text, (first, last), |line| line.strip_prefix("  ").or_else(|| line.strip_prefix(' ')).unwrap_or(line).to_string());
    let less = |at: usize| at.saturating_sub(2);
    (changed, (less(selection.0), less(selection.1)))
}

/// The words that open a block: a line after them goes further in.
const OPENING: [&str; 12] = ["together", "beside", "repeat", "define", "choice", "option", "when", "if", "else", "while", "group", "fight"];

/// Enter: a new line in place of what is chosen, as far in as the cursor's; further in after a line that
/// opens a block, its `end` under it if it wants one; the cursor where the next words go.
pub(super) fn new_line(text: &str, selection: (usize, usize)) -> (String, usize) {
    let cursor = selection.0;
    let (number, start, line) = line_at(text, cursor);
    let lead: String = line.chars().take_while(|letter| *letter == ' ').collect();
    let column = cursor - start;
    let before: String = line.chars().take(column).collect();
    let opens =
        OPENING.contains(&before.split_whitespace().next().unwrap_or_default()) && !before.trim().is_empty() && column >= line.trim_end().chars().count();
    let deeper = if opens { "  " } else { "" };
    let closes = opens && wants_fin(text, number, &lead, &before);
    let from = selection.0.min(selection.1);
    if closes {
        let (changed, _) = replaced(text, from..selection.0.max(selection.1), &format!("\n{lead}{deeper}\n{lead}end"));
        (changed, from + 1 + lead.chars().count() + 2)
    } else {
        replaced(text, from..selection.0.max(selection.1), &format!("\n{lead}{deeper}"))
    }
}

/// The line `first` (from 0) cut whole (Ctrl+X with nothing chosen); the cursor at the start of the one after.
pub(super) fn line_cut(text: &str, first: usize) -> (String, usize) {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.len() > 1 {
        lines.remove(first);
    } else {
        lines[0] = "";
    }
    let changed = lines.join("\n");
    let at = index_of(&changed, Spot { line: (first + 1).min(lines.len()), column: 1 });
    (changed, at)
}
