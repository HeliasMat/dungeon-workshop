//! A line said, in its parts. The lines the
//! Workshop works on by their fields, read here once, by the crate that
//! compiles them: a test sees that what one writes the other reads.

use crate::doc::{Kind, kind_of};

/// A line said, in its parts: its indent, who says it, what is in its
/// brackets (a face, how it is said), what is said.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Said {
    pub lead: String,
    pub who: String,
    pub marks: Vec<String>,
    pub text: String,
}

impl Said {
    /// A line said, read in its parts; none if it is not one.
    pub fn read(line: &str) -> Option<Said> {
        let Kind::Said { .. } = kind_of(line) else {
            return None;
        };
        let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
        let rest = &line[lead.len()..];
        let name_end = rest.find(|letter: char| !(letter.is_alphanumeric() || letter == '_' || letter == '-' || letter == '?')).unwrap_or(rest.len());
        let who = rest[..name_end].to_string();
        let mut after = rest[name_end..].trim_start();
        let mut marks = Vec::new();
        if let Some(inside) = after.strip_prefix('(')
            && let Some((brackets, rest)) = inside.split_once(')')
        {
            marks = brackets.split(',').map(|mark| mark.trim().to_string()).filter(|mark| !mark.is_empty()).collect();
            after = rest.trim_start();
        }
        let text = after.strip_prefix(':').unwrap_or(after).trim_start().to_string();
        Some(Said { lead, who, marks, text })
    }

    /// The line it makes.
    pub fn line(&self) -> String {
        let marks = if self.marks.is_empty() { String::new() } else { format!(" ({})", self.marks.join(", ")) };
        format!("{}{}{marks}: {}", self.lead, self.who, self.text)
    }

    /// Its face, if one of its marks is one.
    pub fn face(&self) -> Option<&str> {
        self.marks.iter().map(String::as_str).find(|mark| crate::words::FACES.iter().any(|(face, _)| face == mark))
    }

    /// The same, with that face (none: without one), its other marks kept.
    pub fn with_face(&self, face: Option<&str>) -> Said {
        let mut marks: Vec<String> = self.marks.iter().filter(|mark| Some(mark.as_str()) != self.face()).cloned().collect();
        if let Some(face) = face {
            marks.insert(0, face.to_string());
        }
        Said { marks, ..self.clone() }
    }

    /// The same, with that way of saying it there or not.
    pub fn with_mark(&self, mark: &str, on: bool) -> Said {
        let mut marks: Vec<String> = self.marks.iter().filter(|known| *known != mark).cloned().collect();
        if on {
            marks.push(mark.to_string());
        }
        Said { marks, ..self.clone() }
    }
}

/// The places a film's scenes are in (`scene: beach pikachu`: `beach`).
pub fn places_of(film: &str) -> Vec<String> {
    let mut places: Vec<String> = film
        .lines()
        .filter_map(|line| match kind_of(line) {
            Kind::Scene { place, .. } => Some(place),
            _ => None,
        })
        .collect();
    places.sort();
    places.dedup();
    places
}

/// A film's text with its scenes in one place now in another, if it has any.
pub fn scenes_moved(film: &str, from: &str, to: &str) -> Option<String> {
    let mut changed = false;
    let mut lines: Vec<String> = film
        .lines()
        .map(|line| {
            let Kind::Scene { place, .. } = kind_of(line) else {
                return line.to_string();
            };
            let lead = &line[..line.len() - line.trim_start().len()];
            let Some(rest) = line.trim_start().strip_prefix("scene:") else {
                return line.to_string();
            };
            let mut words: Vec<&str> = rest.split_whitespace().collect();
            if place != from || words.first() != Some(&from) {
                return line.to_string();
            }
            changed = true;
            words[0] = to;
            format!("{lead}scene: {}", words.join(" "))
        })
        .collect();
    if film.ends_with('\n') {
        lines.push(String::new());
    }
    changed.then(|| lines.join("\n"))
}

/// Seconds as a film writes them: `2s`, `1.5s`.
pub fn seconds_said(seconds: f32) -> String {
    let tenths = (seconds * 10.0).round() / 10.0;
    if tenths.fract() == 0.0 { format!("{tenths:.0}s") } else { format!("{tenths:.1}s") }
}

/// The mark a line sets (`mark quarrel`), if it is one.
pub fn mark_of(line: &str) -> Option<String> {
    match line.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["mark", name] => Some(crate::words::plain(name.trim_end_matches(':'))),
        _ => None,
    }
}

/// The marks a text sets, each with its line, in order.
pub fn marks(text: &str) -> Vec<(usize, String)> {
    text.lines().enumerate().filter_map(|(at, line)| Some((at + 1, mark_of(line)?))).collect()
}

/// A name for a new mark, none of the text's: `m1`, `m2`...
pub fn new_mark(text: &str) -> String {
    let known: Vec<String> = marks(text).into_iter().map(|(_, name)| name).collect();
    (1..).map(|at| format!("m{at}")).find(|name| !known.contains(name)).unwrap_or_default()
}

/// The text with a mark set just before that line (lines from 1), at its indent.
pub fn with_mark_before(text: &str, number: usize, name: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let at = number.saturating_sub(1).min(lines.len());
    let lead: String = lines.get(at).map(|line| line.chars().take_while(|letter| letter.is_whitespace()).collect()).unwrap_or_default();
    lines.insert(at, format!("{lead}mark {name}"));
    let mut made = lines.join("\n");
    if text.ends_with('\n') {
        made.push('\n');
    }
    made
}

/// A film's text with a name (someone, a mark, a spot) made another
/// everywhere it names: its lines' words, who says a line, a scene's cast,
/// `name = species`; not in the words said, nor in the comments.
pub fn renamed(text: &str, old: &str, new: &str) -> String {
    let old = crate::words::plain(old);
    let swap = |word: &str| -> String {
        let core = word.trim_end_matches([':', ',']);
        if !core.is_empty() && crate::words::plain(core) == old { format!("{new}{}", &word[core.len()..]) } else { word.to_string() }
    };
    let swapped = |part: &str| -> String {
        let mut out = String::new();
        let mut word = String::new();
        for letter in part.chars() {
            if letter.is_whitespace() {
                out.push_str(&swap(&word));
                word.clear();
                out.push(letter);
            } else {
                word.push(letter);
            }
        }
        out.push_str(&swap(&word));
        out
    };
    let said_by = |line: &str| match line.split_once(':') {
        Some((who, said)) => format!("{}:{said}", swapped(who)),
        None => line.to_string(),
    };
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            if line.trim_start().starts_with('#') || line.trim_start().starts_with('>') {
                return line.to_string();
            }
            if matches!(kind_of(line), Kind::Said { .. }) {
                // (A line said: who says it, and its brackets; not its words.)
                return said_by(line);
            }
            // (A line anchored, saying a line: the same, after its own colon.)
            let body = crate::time::played_words(line);
            let saying = body != line.trim() && !body.is_empty() && matches!(kind_of(&body), Kind::Said { .. });
            match line.rfind(body.as_str()).filter(|_| saying) {
                Some(at) => {
                    let (head, rest) = line.split_at(at);
                    let (body, tail) = rest.split_at(body.len());
                    format!("{}{}{tail}", swapped(head), said_by(body))
                }
                None => swapped(line),
            }
        })
        .collect();
    if text.ends_with('\n') {
        lines.push(String::new());
    }
    lines.join("\n")
}

/// A film's text with its blocks in step, as the language writes it
/// (`language::format`): each line inside a block two spaces further in than
/// the one that opens it, its `end` back with it; a scene's line at the
/// margin; one blank line at most; the words of each line as they were.
pub fn formatted(text: &str) -> String {
    language::formatted_with(text, &holds)
}

/// The lines of a text, and whether it ended its last one.
fn split_lines(text: &str) -> (Vec<String>, bool) {
    (text.lines().map(str::to_string).collect(), text.ends_with('\n'))
}

fn joined(lines: Vec<String>, ended: bool) -> String {
    let mut made = lines.join("\n");
    if ended {
        made.push('\n');
    }
    made
}

/// The text with those lines (from 1, both included) inside a block: the
/// opening line put before them at their indent, they two spaces further
/// in, its `end` after them.
pub fn wrapped(text: &str, from: usize, to: usize, opening: &str) -> String {
    let (mut lines, ended) = split_lines(text);
    let (from, to) = (from.max(1), to.min(lines.len()));
    if from > to {
        return text.to_string();
    }
    let lead: String = lines[from - 1].chars().take_while(|letter| letter.is_whitespace()).collect();
    for line in &mut lines[from - 1..to] {
        if !line.trim().is_empty() {
            *line = format!("  {line}");
        }
    }
    lines.insert(to, format!("{lead}end"));
    lines.insert(from - 1, format!("{lead}{opening}"));
    joined(lines, ended)
}

/// The text with those lines played beside the film (`&` at their end),
/// or in its course again if they all were.
pub fn beside(text: &str, from: usize, to: usize) -> String {
    let (mut lines, ended) = split_lines(text);
    let (from, to) = (from.max(1), to.min(lines.len()));
    let playing = |line: &String| !line.trim().is_empty() && !line.trim_start().starts_with('#') && !line.trim_start().starts_with("scene:");
    let with = |line: &String| line.trim_end().ends_with(" &");
    let all = (from..=to).filter(|at| playing(&lines[at - 1])).all(|at| with(&lines[at - 1]));
    for at in from..=to {
        let line = &lines[at - 1];
        if playing(line) {
            lines[at - 1] = match (all, with(line)) {
                (true, _) => line.trim_end().strip_suffix(" &").unwrap_or(line).trim_end().to_string(),
                (false, false) => format!("{} &", line.trim_end()),
                (false, true) => line.clone(),
            };
        }
    }
    joined(lines, ended)
}

/// The text with those lines made a recipe of that name (`define` …
/// `end`, before the first scene), called where they were.
pub fn extracted(text: &str, from: usize, to: usize, name: &str) -> String {
    let (mut lines, ended) = split_lines(text);
    let (from, to) = (from.max(1), to.min(lines.len()));
    if from > to {
        return text.to_string();
    }
    let lead: String = lines[from - 1].chars().take_while(|letter| letter.is_whitespace()).collect();
    let taken: Vec<String> = lines.drain(from - 1..to).collect();
    lines.insert(from - 1, format!("{lead}{name}"));
    let first_scene = lines.iter().position(|line| line.trim_start().starts_with("scene:")).unwrap_or(0);
    let mut recipe = vec![format!("define {name}")];
    recipe.extend(taken.iter().map(|line| format!("  {}", line.trim_start())));
    recipe.push("end".to_string());
    recipe.push(String::new());
    for (n, line) in recipe.into_iter().enumerate() {
        lines.insert(first_scene + n, line);
    }
    joined(lines, ended)
}

/// Whether a word's line holds the lines up to its `end`: a memory, a
/// cry through time, a picture shown with no time, a fight the player plays;
/// who first too (`pikachu dimensional_scream tower`, `pikachu keys angle`).
pub fn holds(call: &language::ast::Call) -> bool {
    if call.args.first().is_some_and(|second| matches!(second.text.as_str(), "dimensional_scream" | "keys")) {
        return true;
    }
    match call.word.text.as_str() {
        "memory" | "dimensional_scream" | "keys" | "dialogue" => true,
        "shot" => call.args.len() == 1 && !matches!(call.args[0].text.as_str(), "wide" | "medium" | "close" | "together"),
        // (`fight a vs b`: a fight played whole, no block.)
        "fight" => !call.args.is_empty() && !call.args.iter().any(|arg| arg.text == "vs"),
        _ => false,
    }
}
