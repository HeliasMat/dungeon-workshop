//! A character renamed in a film (`rename`): every word that names it, by
//! the language's own reading of each line, never the words said.

use std::collections::BTreeMap;

use ::film::grammar::Kind as ValueKind;
use ::film::{Context, ReadAs};
use language::Span;
use language::ast::{ArgKind, Call, Event, Expr, ExprKind, Kind, Opening};
use language::line::{Line, Read};

/// Each line of a text as the language reads it alone: its number (from
/// 1), where it starts, what it is.
pub(super) fn lines_read(text: &str) -> Vec<(usize, usize, Line)> {
    let (mut start, mut before_scenes, mut read) = (0, true, Vec::new());
    for (at, raw) in text.split_inclusive('\n').enumerate() {
        let body = raw.trim_end_matches(['\n', '\r']);
        let line = language::line::line(body, start, before_scenes, &mut Vec::new());
        if matches!(line.read, Read::Scene(_)) {
            before_scenes = false;
        }
        read.push((at + 1, start, line));
        start += raw.len();
    }
    read
}

/// The word a line calls, as written (a block's word too).
pub(super) fn called(line: &Line) -> Option<&str> {
    match &line.read {
        Read::Stmt(Kind::Call(call), _) | Read::Opens { opening: Opening::Word(call), .. } => Some(call.word.text.as_str()),
        Read::Opens { short: Some((Kind::Call(call), _, _)), .. } => Some(call.word.text.as_str()),
        Read::Scene(_) => Some("scene"),
        _ => None,
    }
}

/// Where a text names `name`: each place with its line, and whether it
/// gives it its name (`name = species`).
#[derive(Debug, Default)]
pub(super) struct Found {
    pub places: Vec<(usize, Span)>,
    pub named: bool,
    /// Where the first scene starts.
    pub first_scene: Option<(usize, usize)>,
}

/// Every place `name` is named: the speaker of a said line, a word given
/// to a word (never one of a text: `story …`, `text…`, a thing), the
/// `scene:` cast, the name of `x = species`, a name in a calculation, a
/// `when x arrives`. Never in what is said, a caption, a comment.
pub(super) fn places(text: &str, name: &str, read_as: &BTreeMap<usize, ReadAs>) -> Found {
    let mut found = Found::default();
    for (number, start, line) in lines_read(text) {
        let mut here = Vec::new();
        match &line.read {
            Read::Stmt(kind, _) => in_kind(kind, name, read_as.get(&number), &mut here, &mut found.named),
            Read::Scene(args) => {
                found.first_scene.get_or_insert((number, start));
                here.extend(args.iter().skip(1).filter(|arg| matches!(arg.kind, ArgKind::Word) && arg.text == name).map(|arg| arg.span));
            }
            Read::Opens { opening, short, .. } => {
                match opening {
                    Opening::If(expr) | Opening::Repeat(expr) | Opening::While(expr) | Opening::Every(expr) | Opening::After(expr) => {
                        in_expr(expr, name, &mut here)
                    }
                    Opening::ForEach { list, .. } => in_expr(list, name, &mut here),
                    Opening::When(Event::Arrives(who) | Event::Leaves(who) | Event::Knocked(who)) if who.text == name => here.push(who.span),
                    Opening::When(Event::Becomes(expr)) => in_expr(expr, name, &mut here),
                    Opening::At { offset: Some((_, expr)), .. } => in_expr(expr, name, &mut here),
                    Opening::Fight(args) => here.extend(args.iter().filter(|arg| matches!(arg.kind, ArgKind::Word) && arg.text == name).map(|arg| arg.span)),
                    Opening::Word(call) => in_call(call, name, read_as.get(&number), &mut here),
                    _ => {}
                }
                if let Some((kind, _, _)) = short {
                    in_kind(kind, name, read_as.get(&number), &mut here, &mut found.named);
                }
            }
            Read::ElseIf(expr) => in_expr(expr, name, &mut here),
            _ => {}
        }
        found.places.extend(here.into_iter().map(|span| (number, span)));
    }
    found.places.sort_by_key(|(_, span)| span.start);
    found.places.dedup_by_key(|(_, span)| span.start);
    found
}

fn in_kind(kind: &Kind, name: &str, read: Option<&ReadAs>, here: &mut Vec<Span>, named: &mut bool) {
    match kind {
        // (Its speaker only: `kecleon?:` too.)
        Kind::Said(said) if said.who.text.trim_end_matches('?') == name => here.push(Span::new(said.who.span.start, said.who.span.start + name.len())),
        Kind::Naming { name: given, .. } if given.text == name => {
            *named = true;
            here.push(given.span);
        }
        Kind::Call(call) => in_call(call, name, read, here),
        Kind::Let { value, .. } | Kind::Set { value, .. } | Kind::Return(Some(value)) => in_expr(value, name, here),
        _ => {}
    }
}

/// A word's arguments that are that name, but those its form takes as a
/// text or a thing: as the line was read, or else by the word's forms.
fn in_call(call: &Call, name: &str, read: Option<&ReadAs>, here: &mut Vec<Span>) {
    // (Who acts first: `kecleon jump`, its first word is the one named.)
    if call.word.text == name && ::film::subject_first(name, call.args.first().map(|arg| arg.text.as_str()), "") {
        here.push(call.word.span);
    }
    let wordy = |kind: ValueKind, value: &str| matches!(kind, ValueKind::Thing) || value == "text";
    let text_from = match read {
        Some(_) => None,
        None => ::film::every_word().find(|(_, word, ..)| *word == call.word.text).and_then(|(_, _, forms, _)| {
            forms.iter().find_map(|form| form.split_whitespace().position(|element| element.starts_with("text") || element.starts_with("thing")))
        }),
    };
    for (at, arg) in call.args.iter().enumerate() {
        if !matches!(arg.kind, ArgKind::Word) || arg.text != name {
            continue;
        }
        let kept = match read {
            Some(read) => read.values.iter().any(|value| {
                let inside = match (value.span, value.at) {
                    (Some(span), _) => span.start <= arg.span.start && arg.span.end <= span.end,
                    (None, Some(first)) => (first..first + value.words.len()).contains(&at),
                    (None, None) => false,
                };
                inside && wordy(value.kind, &value.name)
            }),
            None => text_from.is_some_and(|from| at >= from),
        };
        if !kept {
            here.push(arg.span);
        }
    }
}

fn in_expr(expr: &Expr, name: &str, here: &mut Vec<Span>) {
    match &expr.kind {
        ExprKind::Name(said) if said == name => here.push(expr.span),
        ExprKind::List(all) => all.iter().for_each(|one| in_expr(one, name, here)),
        ExprKind::Point(a, b) | ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            in_expr(a, name, here);
            in_expr(b, name, here);
        }
        ExprKind::Unary(_, a) | ExprKind::Field(a, _) => in_expr(a, name, here),
        ExprKind::Call(_, all) => all.iter().for_each(|one| in_expr(one, name, here)),
        _ => {}
    }
}

/// How many errors the language finds in a text.
fn errors(context: &Context, text: &str) -> usize {
    ::film::service::diagnostics(text, context.tables()).iter().filter(|one| one.is_error()).count()
}

/// `rename <film> <old> <new> [--try]`.
pub fn rename(context: &Context, name: &str, old: &str, new: &str, trial: bool) -> Result<(), String> {
    let path = ::film::locate(&context.library, name).ok_or_else(|| format!("film not found: {name}"))?;
    let text = std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    if old == new {
        return Err(format!("“{old}” and “{new}” are the same name: nothing to change"));
    }
    if new.is_empty() || !new.chars().all(language::lexer::in_name) {
        return Err(format!("“{new}” is not a name (letters, digits, _)"));
    }
    let written = ::film::compile_with(&text, context.tables());
    let found = places(&text, old, &written.read_as);
    if found.places.is_empty() {
        let names: Vec<&str> = written.names.iter().map(|(_, name)| name.as_str()).collect();
        return Err(format!("“{old}” names no one in this film{}", ::film::check::nearby(old, names.into_iter())));
    }
    if written.names.iter().any(|(_, name)| name == new) || !places(&text, new, &written.read_as).places.is_empty() {
        return Err(format!("“{new}” is already a name in this film: choose another one"));
    }
    let mut made = text.clone();
    for (_, span) in found.places.iter().rev() {
        made.replace_range(span.start..span.end, new);
    }
    // (Its species was its name: it gets a name of its own, before the first
    // scene.)
    let added = match (found.named, found.first_scene) {
        (false, Some((line, start))) => {
            made.insert_str(start, &format!("{new} = {old}\n"));
            Some(line)
        }
        _ => None,
    };
    let mut lines: Vec<usize> = found.places.iter().map(|(line, _)| *line).collect();
    lines.dedup();
    let shown_path = path.strip_prefix(context.root()).unwrap_or(&path).display().to_string().replace('\\', "/");
    println!("{shown_path}: “{old}” becomes “{new}” in {} place(s), lines {}", found.places.len(), joined(&lines));
    if let Some(line) = added {
        println!("  + line {line}: {new} = {old}  (its own name, before the first scene)");
    }
    let shown = 12;
    for line in lines.iter().take(shown) {
        let was = text.lines().nth(line - 1).unwrap_or("").trim();
        let now = made.lines().nth(line - 1 + usize::from(added.is_some_and(|at| at <= *line))).unwrap_or("").trim();
        println!("  {line:>4}  {was}\n     →  {now}");
    }
    if lines.len() > shown {
        println!("  … and {} more lines", lines.len() - shown);
    }
    let (before, after) = (errors(context, &text), errors(context, &made));
    if after > before {
        return Err(format!("after the change, the language finds {after} error(s) (before: {before}): nothing written"));
    }
    if trial {
        println!("--try: nothing written");
        return Ok(());
    }
    std::fs::write(&path, &made).map_err(|error| format!("{}: {error}", path.display()))?;
    println!("written; the language finds no more errors in it ({after})");
    Ok(())
}

fn joined(numbers: &[usize]) -> String {
    numbers.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_words_that_name_it() {
        let text = "title: Trial\nscene: beach pikachu kecleon\nkecleon (joyous): Hello kecleon!\nnarrate: kecleon sleeps.\nframe pikachu kecleon # kecleon\nif target = kecleon\n  kecleon jump\nend\n";
        let found = places(text, "kecleon", &BTreeMap::new());
        let lines: Vec<usize> = found.places.iter().map(|(line, _)| *line).collect();
        assert_eq!(lines, [2, 3, 5, 6, 7]);
        assert!(found.places.iter().all(|(_, span)| span.of(text) == "kecleon"));
        assert!(!found.named);
        assert_eq!(found.first_scene.map(|(line, _)| line), Some(2));
        // (Its name given, it is found there too.)
        let found = places("kec = kecleon\nscene: beach kec\nkec: Yes.\n", "kec", &BTreeMap::new());
        assert!(found.named);
        assert_eq!(found.places.len(), 3);
    }
}
