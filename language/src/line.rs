//! One line: what it is (a said line, words, a variable, a block's opening,
//! an `else`, an `end`…), its arguments, its `&`, its comment.

use crate::ast::{Arg, ArgKind, Beside, Call, Duration, Element, Event, Expr, Form, Kind, Language, Name, Opening, PartKind, Piece, Recipe, Said, Type};
use crate::diag::{Diagnostic, codes};
use crate::expr::expression;
use crate::lexer::{Token, in_name, name_length, number_word, quoted};
use crate::span::Span;

/// The words that open, part or close a block, or start a line of their
/// own: at the start of a line they are never a word to call nor a `who`.
pub const RESERVED: [&str; 24] = [
    "title", "scene", "moment", "end", "if", "else", "repeat", "for", "while", "together", "beside", "when", "every", "after", "choice", "option", "fight",
    "define", "return", "let", "keep", "break", "next", "uses",
];

/// What a line is (read once and taken apart at once: its size does not
/// matter).
#[allow(clippy::large_enum_variant)]
pub enum Read {
    Blank,
    Comment(String),
    Stmt(Kind, Option<Beside>),
    Scene(Vec<Arg>),
    /// `extra`: what was written after a word that takes nothing, kept.
    Opens {
        opening: Opening,
        named: Option<Name>,
        short: Option<(Kind, Option<Beside>, Span)>,
        extra: Option<String>,
    },
    ElseIf(Expr),
    Else,
    Option(PartKind),
    End,
}

/// A line read: what it is, its span (without its comment), its comment.
pub struct Line {
    pub read: Read,
    pub span: Span,
    pub comment: Option<String>,
}

/// Reads the line that starts at `base` (`before_scenes`: no scene yet).
pub fn line(text: &str, base: usize, before_scenes: bool, diagnostics: &mut Vec<Diagnostic>) -> Line {
    let lead = text.len() - text.trim_start().len();
    let trimmed = text.trim();
    let start = base + lead;
    if trimmed.is_empty() {
        return Line { read: Read::Blank, span: Span::new(start, start), comment: None };
    }
    if let Some(comment) = trimmed.strip_prefix('#') {
        return Line { read: Read::Comment(comment.trim().to_string()), span: Span::new(start, start + trimmed.len()), comment: None };
    }
    let first = &trimmed[..name_length(trimmed)];
    // A said line keeps its `#`: it is part of what is said.
    let (code, comment) = if first.is_empty() || said_head(trimmed).is_none() { cut_comment(trimmed) } else { (trimmed, None) };
    let span = Span::new(start, start + code.len());
    let read = statement(code, start, before_scenes, diagnostics).unwrap_or_else(|| Read::Stmt(Kind::Unreadable(code.to_string()), None));
    Line { read, span, comment }
}

/// The code of a line and its comment: a `#` at the start of a word, out of
/// texts and braces, but for a colour's (`#ff8800`, see `colour_word`).
fn cut_comment(text: &str) -> (&str, Option<String>) {
    let mut depth = 0usize;
    let mut quoted_now = false;
    let mut before = ' ';
    for (at, letter) in text.char_indices() {
        match letter {
            '"' => quoted_now = !quoted_now,
            '{' | '(' | '[' if !quoted_now => depth += 1,
            '}' | ')' | ']' if !quoted_now => depth = depth.saturating_sub(1),
            '#' if !quoted_now && depth == 0 && before.is_whitespace() && !colour_word(&text[at + 1..]) => {
                return (text[..at].trim_end(), Some(text[at + 1..].trim().to_string()));
            }
            _ => {}
        }
        before = letter;
    }
    (text, None)
}

/// Whether a word after a `#` is a colour's hex digits, the whole of it:
/// 3, 4, 6 or 8 of them (`#f80`, `#ff8800`, `#ff880080`).
pub fn colour_word(after: &str) -> bool {
    let digits = after.bytes().take_while(u8::is_ascii_hexdigit).count();
    matches!(digits, 3 | 4 | 6 | 8) && after[digits..].chars().next().is_none_or(char::is_whitespace)
}

/// Where the `:` of a said line is, if this is one: `who:` or `who (…):`,
/// `who` not a reserved word.
fn said_head(text: &str) -> Option<(usize, Option<(usize, usize)>)> {
    let name = who_length(text);
    if name == 0 || RESERVED.contains(&&text[..name]) || !text[..name].chars().all(|letter| in_name(letter) || matches!(letter, '?' | '-')) {
        return None;
    }
    let after = &text[name..];
    let gap = after.len() - after.trim_start().len();
    let rest = &after[gap..];
    if rest.starts_with(':') {
        return Some((name + gap, None));
    }
    if rest.starts_with('(') {
        let close = rest.find(')')?;
        let past = &rest[close + 1..];
        let colon = past.len() - past.trim_start().len();
        if past[colon..].starts_with(':') {
            let open = name + gap;
            return Some((open + close + 1 + colon, Some((open + 1, open + close))));
        }
    }
    None
}

/// The length of who says a line: a name, maybe hidden (`umbreon?`, `???`).
fn who_length(text: &str) -> usize {
    let name = name_length(text);
    name + text[name..].len() - text[name..].trim_start_matches('?').len()
}

fn name_at(text: &str, start: usize) -> Name {
    Name { text: text.to_string(), span: Span::new(start, start + text.len()) }
}

/// The first word of a text, its rest (trimmed), and where the rest starts.
fn split_word(text: &str) -> (&str, &str, usize) {
    let word = text.split_whitespace().next().unwrap_or("");
    let rest = &text[word.len()..];
    let gap = rest.len() - rest.trim_start().len();
    (word, rest.trim(), word.len() + gap)
}

fn statement(text: &str, start: usize, before_scenes: bool, diagnostics: &mut Vec<Diagnostic>) -> Option<Read> {
    let span = Span::new(start, start + text.len());
    if let Some((colon, manners)) = said_head(text) {
        return Some(said(text, start, colon, manners, diagnostics));
    }
    let (word, rest, rest_at) = split_word(text);
    let rest_start = start + rest_at;
    let header = text.find(':').map(|colon| text[..colon].trim());
    match header {
        Some("title") => return Some(Read::Stmt(Kind::Title(text[text.find(':')? + 1..].trim().to_string()), None)),
        Some("moment") => return Some(Read::Stmt(Kind::Moment(text[text.find(':')? + 1..].trim().to_string()), None)),
        Some("scene") => {
            let colon = text.find(':')? + 1;
            let after = &text[colon..];
            let gap = after.len() - after.trim_start().len();
            return Some(Read::Scene(args(after.trim(), start + colon + gap, diagnostics)));
        }
        _ => {}
    }
    let read = match word {
        "end" if rest.is_empty() => Read::End,
        "endif" if rest.is_empty() => {
            diagnostics.push(old(span, "“endif” has become “end”").fix("write “end”", span, "end"));
            Read::End
        }
        "else" => {
            if rest.is_empty() {
                Read::Else
            } else if let Some(test) = rest.strip_prefix("if ") {
                let at = rest_start + (rest.len() - test.len());
                Read::ElseIf(expr_or_note(test.trim(), at + (test.len() - test.trim_start().len()), diagnostics)?)
            } else {
                diagnostics.push(Diagnostic::error(codes::EXTRA, span, "“else” is followed by “if …” or by nothing"));
                Read::Else
            }
        }
        "option" => Read::Option(option(rest, rest_start, diagnostics)),
        "uses" | "include" => {
            let Some(name) = rest.split_whitespace().next().filter(|_| rest.split_whitespace().count() == 1) else {
                diagnostics.push(Diagnostic::error(codes::NAME, span, format!("“{word}” takes a file name, alone")));
                return None;
            };
            let name = name_at(name, rest_start);
            Read::Stmt(if word == "uses" { Kind::Uses(name) } else { Kind::Includes(name) }, None)
        }
        "let" | "keep" => {
            let Some((name, value)) = assignment(rest, rest_start, diagnostics) else {
                diagnostics.push(
                    Diagnostic::error(codes::UNREADABLE, span, format!("“{word}” is written “{word} name = value”"))
                        .note(format!("for example “{word} n = 3”")),
                );
                return None;
            };
            Read::Stmt(Kind::Let { kept: word == "keep", name, value: value.ok()? }, None)
        }
        "return" => Read::Stmt(Kind::Return(if rest.is_empty() { None } else { Some(expr_or_note(rest, rest_start, diagnostics)?) }), None),
        "break" if rest.is_empty() => Read::Stmt(Kind::Break, None),
        "next" if rest.is_empty() => Read::Stmt(Kind::Continue, None),
        "if" => {
            let test = rest.strip_suffix(':').map(str::trim_end);
            if let Some(test) = test {
                let colon = Span::new(rest_start + test.len(), rest_start + rest.len());
                diagnostics.push(old(colon, "“if …:” is written without “:”").fix("remove the “:”", colon, ""));
            }
            opens(Opening::If(expr_or_nothing(test.unwrap_or(rest), rest_start, diagnostics)))
        }
        "repeat" => opens(Opening::Repeat(expr_or_nothing(rest, rest_start, diagnostics))),
        "while" => opens(Opening::While(expr_or_nothing(rest, rest_start, diagnostics))),
        "for" => for_each(rest, rest_start, span, diagnostics)?,
        "together" | "choice" => {
            if !rest.is_empty() {
                let extra = Span::new(rest_start, rest_start + rest.len());
                let mut told = Diagnostic::error(codes::EXTRA, extra, format!("“{word}” takes nothing after it"));
                if word == "choice" && rest.contains('|') {
                    told = told.code_old().note("today: “choice”, then one line “option …” per answer, then “end”");
                }
                diagnostics.push(told);
            }
            let opening = if word == "choice" { Opening::Choice } else { Opening::Together };
            Read::Opens { opening, named: None, short: None, extra: (!rest.is_empty()).then(|| rest.to_string()) }
        }
        "beside" => {
            let (opening_text, named) = named_end(rest, rest_start);
            let named = named.or_else(|| (!opening_text.is_empty()).then(|| name_at(opening_text, rest_start)));
            Read::Opens { opening: Opening::Beside, named, short: None, extra: None }
        }
        "when" | "every" | "after" | "at" => beside_block(word, rest, rest_start, span, before_scenes, diagnostics)?,
        "fight" if !rest.starts_with("directed") => call_line(text, start, diagnostics)?,
        "fight" => {
            let fight = &rest["directed".len()..];
            let at = rest_start + "directed".len() + (fight.len() - fight.trim_start().len());
            opens(Opening::Fight(args(fight.trim(), at, diagnostics)))
        }
        "define" => {
            let (form_text, returns) = match rest.rfind("->") {
                Some(arrow) => (rest[..arrow].trim_end(), Some((&rest[arrow + 2..], rest_start + arrow + 2))),
                None => (rest, None),
            };
            // A form that does not read still opens its block, under its word.
            let form = form(form_text, rest_start).unwrap_or_else(|wrong| {
                diagnostics.push(wrong);
                let word = form_text.split_whitespace().next().unwrap_or("");
                Form { word: name_at(word, rest_start), elements: vec![Element::Word(name_at(form_text[word.len()..].trim(), rest_start + word.len()))] }
            });
            let returns = match returns {
                Some((ty, at)) => Some(type_of(ty.trim(), at + (ty.len() - ty.trim_start().len())).map_err(|wrong| diagnostics.push(wrong)).ok()?),
                None => None,
            };
            opens(Opening::Recipe(Recipe { form, returns }))
        }
        _ => {
            if let Some((name, value)) = assignment(text, start, diagnostics) {
                if before_scenes && let Ok(Expr { kind: crate::ast::ExprKind::Name(species), span }) = &value {
                    return Some(Read::Stmt(Kind::Naming { name, species: Name { text: species.clone(), span: *span } }, None));
                }
                return Some(Read::Stmt(Kind::Set { name, value: value.ok()? }, None));
            }
            // (`title`, `scene`, `moment` are headers with their `:` only: `scene 11 name` is a word.)
            if RESERVED.contains(&word) && !matches!(word, "title" | "scene" | "moment") {
                diagnostics.push(Diagnostic::error(codes::UNREADABLE, span, format!("“{word}” is not written this way")).note("see language/LANGUAGE.md"));
                return None;
            }
            if word.starts_with('>') {
                let told = old(span, "“>” no longer passes a raw line to the engine: each command has its word");
                diagnostics.push(told);
                return None;
            }
            call_line(text, start, diagnostics)?
        }
    };
    Some(read)
}

/// `word argument…`
fn call_line(text: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> Option<Read> {
    let mut found = args(text, start, diagnostics);
    let beside = beside_end(&mut found);
    if found.is_empty() {
        diagnostics.push(Diagnostic::error(codes::UNREADABLE, Span::new(start, start + text.len()), "a line starts with a word"));
        return None;
    }
    let word = found.remove(0);
    // (A time first: a key of a `keys` block, `1s 90`.)
    let timed = matches!(word.kind, ArgKind::Duration(_));
    if !timed && (!matches!(word.kind, ArgKind::Word) || name_length(&word.text) != word.text.len()) {
        diagnostics.push(Diagnostic::error(codes::UNREADABLE, word.span, format!("a line starts with a word, not with “{}”", word.text)));
        return None;
    }
    Some(Read::Stmt(Kind::Call(Call { word: Name { text: word.text, span: word.span }, args: found }), beside))
}

fn opens(opening: Opening) -> Read {
    Read::Opens { opening, named: None, short: None, extra: None }
}

fn old(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(codes::OLD_FORM, span, message)
}

trait OldCode {
    fn code_old(self) -> Self;
}

impl OldCode for Diagnostic {
    fn code_old(mut self) -> Self {
        self.code = Some(codes::OLD_FORM);
        self
    }
}

/// The calculation, or `none` (and said) if it does not read: its block
/// stays a block, so its `end` is not one too many.
fn expr_or_nothing(text: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> Expr {
    expr_or_note(text, start, diagnostics)
        .unwrap_or(Expr { span: Span::new(start, start + text.len()), kind: crate::ast::ExprKind::Unreadable(text.to_string()) })
}

fn expr_or_note(text: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> Option<Expr> {
    expression(text, start).map_err(|wrong| diagnostics.push(wrong)).ok()
}

/// `name = value` (one `=`, a name before it).
fn assignment(text: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> Option<(Name, Result<Expr, ()>)> {
    let name = name_length(text);
    if name == 0 || !text[..name].chars().next().is_some_and(char::is_alphabetic) {
        return None;
    }
    let after = &text[name..];
    let gap = after.len() - after.trim_start().len();
    let rest = &after[gap..];
    if !rest.starts_with('=') || rest.starts_with("==") {
        return None;
    }
    let value = &rest[1..];
    let at = start + name + gap + 1 + (value.len() - value.trim_start().len());
    let value = expression(value.trim(), at).map_err(|wrong| diagnostics.push(wrong));
    Some((name_at(&text[..name], start), value))
}

/// `for each x[, rank] in list`
fn for_each(rest: &str, start: usize, span: Span, diagnostics: &mut Vec<Diagnostic>) -> Option<Read> {
    let Some(each) = rest.strip_prefix("each ") else {
        let told = if rest.contains(':') {
            old(span, "“for who: line” has become “for each w in who” … “end”")
        } else {
            Diagnostic::error(codes::UNREADABLE, span, "“for” is written “for each x in list”")
        };
        diagnostics.push(told);
        return None;
    };
    let each_at = start + "each ".len();
    let Some(inside) = each.find(" in ") else {
        diagnostics.push(Diagnostic::error(codes::UNREADABLE, span, "“for each x” expects “in” and a list"));
        return None;
    };
    let names = &each[..inside];
    let mut found = names.split(',').map(str::trim);
    let name = found.next().filter(|name| !name.is_empty())?;
    let name_at_byte = each_at + names.find(name).unwrap_or(0);
    let rank = found.next().map(|rank| name_at(rank, each_at + names.rfind(rank).unwrap_or(0)));
    let list = &each[inside + 4..];
    let list_at = each_at + inside + 4 + (list.len() - list.trim_start().len());
    let list = expr_or_note(list.trim(), list_at, diagnostics)?;
    Some(opens(Opening::ForEach { name: name_at(name, name_at_byte), rank, list }))
}

/// `when`, `every`, `after`, `at`: a block beside the film, maybe named,
/// maybe on one line (`: line`).
fn beside_block(word: &str, rest: &str, start: usize, span: Span, before_scenes: bool, diagnostics: &mut Vec<Diagnostic>) -> Option<Read> {
    let (opening_text, short) = match top_colon(rest) {
        // (`when a arrives :` and nothing after: the block of before.)
        Some(colon) if rest[colon + 1..].trim().is_empty() => {
            let sign = Span::new(start + rest[..colon].trim_end().len(), start + rest.len());
            diagnostics.push(old(sign, format!("“{word} … :” opens its block without “:”")).fix("remove the “:”", sign, ""));
            (rest[..colon].trim_end(), None)
        }
        Some(colon) => {
            let inner = &rest[colon + 1..];
            let inner_at = start + colon + 1 + (inner.len() - inner.trim_start().len());
            (rest[..colon].trim_end(), Some((inner.trim(), inner_at)))
        }
        None => (rest, None),
    };
    let (opening_text, named) = named_end(opening_text, start);
    let opening = match word {
        "when" => {
            if opening_text.contains('|') {
                diagnostics.push(old(span, "“when c 50 | line” has become “when health(c) <= 50 : line”"));
                return None;
            }
            Opening::When(event(opening_text, start, diagnostics)?)
        }
        "every" if opening_text == "off" => {
            diagnostics.push(old(span, "“every off” has become “cease name” (the block named by “as name”)"));
            return None;
        }
        "every" | "after" => {
            let Ok(every) = expression(opening_text, start) else {
                let mut told = old(span, format!("“{word}” is written “{word} duration : line”, or as a block up to “end”"));
                let (first, after, _) = split_word(opening_text);
                if short.is_none() && !after.is_empty() {
                    let length = first.len();
                    let first = match first.split_once('~') {
                        Some((low, high)) => format!("random({low}, {high})"),
                        None => first.to_string(),
                    };
                    let whole = Span::new(start, start + length);
                    told = told.fix(format!("write “{word} {first} : {after}”"), whole, format!("{first} :"));
                }
                diagnostics.push(told);
                return None;
            };
            if word == "every" { Opening::Every(every) } else { Opening::After(every) }
        }
        _ => {
            let mark = opening_text.split_whitespace().next().unwrap_or("");
            // (`at 12s`: from the scene's start; `at music 12s`: by the music's clock.)
            let (first, then, then_at) = split_word(opening_text);
            let music = first == "music" && then.starts_with(|letter: char| letter.is_ascii_digit());
            if music || mark.starts_with(|letter: char| letter.is_ascii_digit()) {
                let (time, more, at) = match music {
                    true => {
                        let (time, more, _) = split_word(then);
                        (time, more, start + then_at)
                    }
                    false => (first, then, start),
                };
                if !clock_time(time) || !more.is_empty() {
                    let written = if music { format!("at music {time}") } else { format!("at {time}") };
                    diagnostics.push(Diagnostic::error(
                        codes::UNREADABLE,
                        span,
                        format!("“{written}”: a duration with its unit (“at 12s : line”, “at music 12s : line”, “6f” in frames), nothing after"),
                    ));
                    return None;
                }
                let clock = if music { crate::ast::Clock::Music } else { crate::ast::Clock::Scene };
                if before_scenes {
                    diagnostics.push(Diagnostic::error(codes::UNREADABLE, span, "“at 12s” plays in a scene"));
                    return None;
                }
                let opening = Opening::At { mark: name_at(time, at), offset: None, clock };
                return opened(opening, named, short, diagnostics);
            }
            // (`at +2s`: so long from here.)
            if let Some(time) = mark.strip_prefix('+').filter(|time| clock_time(time)) {
                if before_scenes || opening_text.trim() != mark {
                    diagnostics.push(Diagnostic::error(codes::UNREADABLE, span, "“at +2s” plays in a scene, nothing after the duration (“at +2s : line”)"));
                    return None;
                }
                let opening = Opening::At { mark: name_at(time, start + 1), offset: None, clock: crate::ast::Clock::Here };
                return opened(opening, named, short, diagnostics);
            }
            if before_scenes || mark.is_empty() || name_length(mark) != mark.len() {
                diagnostics.push(Diagnostic::error(codes::UNREADABLE, span, "“at” opens a mark of the scene: “at name + 2s : line”"));
                return None;
            }
            let after = opening_text[mark.len()..].trim();
            let offset = if after.is_empty() {
                None
            } else {
                let minus = after.starts_with('-');
                let Some(by) = after.strip_prefix(['+', '-']) else {
                    diagnostics.push(Diagnostic::error(codes::UNREADABLE, span, "“at mark” is followed by “+ duration” or by “- duration”"));
                    return None;
                };
                let at = start + (opening_text.len() - by.len()) + (by.len() - by.trim_start().len());
                Some((minus, expr_or_note(by.trim(), at, diagnostics)?))
            };
            Opening::At { mark: name_at(mark, start), offset, clock: crate::ast::Clock::Mark }
        }
    };
    opened(opening, named, short, diagnostics)
}

/// A time an `at` line counts to: digits, then `s` (seconds) or `f` (frames).
fn clock_time(word: &str) -> bool {
    word.strip_suffix(['s', 'f'])
        .is_some_and(|number| !number.is_empty() && number.chars().all(|letter| letter.is_ascii_digit() || letter == '.') && number.parse::<f32>().is_ok())
}

/// A block beside the film read, with its line after `:` if it has one.
fn opened(opening: Opening, named: Option<Name>, short: Option<(&str, usize)>, diagnostics: &mut Vec<Diagnostic>) -> Option<Read> {
    let short = match short {
        Some((inner, inner_at)) => {
            let inner_span = Span::new(inner_at, inner_at + inner.len());
            match statement(inner, inner_at, false, diagnostics)? {
                Read::Stmt(kind, beside) => Some((kind, beside, inner_span)),
                _ => {
                    diagnostics.push(Diagnostic::error(codes::UNREADABLE, inner_span, "after “:”, one single simple line (not a block)"));
                    return None;
                }
            }
        }
        None => None,
    };
    Some(Read::Opens { opening, named, short, extra: None })
}

/// What a `when` waits for: `a arrives`, `a leaves`, `a is ko`, or a test.
fn event(text: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> Option<Event> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let plain = |word: &str| name_length(word) == word.len() && !word.is_empty();
    match words.as_slice() {
        [who, "arrives"] if plain(who) => Some(Event::Arrives(name_at(who, start))),
        [who, "leaves"] if plain(who) => Some(Event::Leaves(name_at(who, start))),
        [who, "is", "ko"] if plain(who) => Some(Event::Knocked(name_at(who, start))),
        _ => Some(Event::Becomes(expr_or_note(text, start, diagnostics)?)),
    }
}

/// The first `:` out of brackets and texts.
fn top_colon(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut quoted_now = false;
    for (at, letter) in text.char_indices() {
        match letter {
            '"' => quoted_now = !quoted_now,
            '{' | '(' | '[' if !quoted_now => depth += 1,
            '}' | ')' | ']' if !quoted_now => depth = depth.saturating_sub(1),
            ':' if !quoted_now && depth == 0 => return Some(at),
            _ => {}
        }
    }
    None
}

/// A text without its `as name` end, and that name.
fn named_end(text: &str, start: usize) -> (&str, Option<Name>) {
    let words: Vec<&str> = text.split_whitespace().collect();
    if let [.., "as", name] = words.as_slice()
        && name_length(name) == name.len()
    {
        let at = text.rfind(name).unwrap_or(0);
        let before = text[..at].trim_end().strip_suffix("as").unwrap_or("").trim_end();
        return (before, Some(name_at(name, start + at)));
    }
    (text, None)
}

/// Takes the `&` (or `& as name`) off the end of a line's arguments.
fn beside_end(found: &mut Vec<Arg>) -> Option<Beside> {
    let length = found.len();
    if length >= 3 && found[length - 3].text == "&" && found[length - 2].text == "as" {
        let name = found.pop()?;
        found.pop();
        let sign = found.pop()?;
        return Some(Beside { span: sign.span.to(name.span), named: Some(Name { text: name.text, span: name.span }) });
    }
    if length >= 2 && found[length - 1].text == "&" {
        let sign = found.pop()?;
        return Some(Beside { span: sign.span, named: None });
    }
    None
}

fn said(text: &str, start: usize, colon: usize, manners: Option<(usize, usize)>, diagnostics: &mut Vec<Diagnostic>) -> Read {
    let who = name_at(&text[..who_length(text)], start);
    let manners = manners
        .map(|(from, to)| {
            let inside = &text[from..to];
            let mut made = Vec::new();
            let mut at = 0;
            for manner in inside.split(',') {
                let lead = manner.len() - manner.trim_start().len();
                if !manner.trim().is_empty() {
                    made.push(name_at(manner.trim(), start + from + at + lead));
                }
                at += manner.len() + 1;
            }
            made
        })
        .unwrap_or_default();
    let after = &text[colon + 1..];
    let text_at = start + colon + 1 + (after.len() - after.trim_start().len());
    let mut said_text = after.trim();
    let mut beside = None;
    let words: Vec<&str> = said_text.split_whitespace().collect();
    let tail = match words.as_slice() {
        [.., "&", "as", name] if name_length(name) == name.len() => Some((3, Some(*name))),
        [_, .., "&"] => Some((1, None)),
        _ => None,
    };
    if let Some((count, name)) = tail {
        let mut cut = said_text.len();
        for _ in 0..count {
            cut = said_text[..cut].trim_end().rfind(char::is_whitespace).map_or(0, |space| space + 1);
        }
        let sign_at = text_at + cut;
        let named = name.map(|name| name_at(name, text_at + said_text.rfind(name).unwrap_or(0)));
        beside = Some(Beside { span: Span::new(sign_at, text_at + said_text.len()), named });
        said_text = said_text[..cut].trim_end();
    }
    let text_span = Span::new(text_at, text_at + said_text.len());
    // (Its id, written last: `Hello! #k3f9`; the words before it are what is said.)
    let last = said_text.rfind(char::is_whitespace).map_or(0, |space| space + 1);
    let (id, said_text_alone) = match line_id(&said_text[last..]).filter(|_| last > 0) {
        Some(id) => (Some(name_at(id, text_at + last + 1)), said_text[..last].trim_end()),
        None => (None, said_text),
    };
    // (Its other languages after it: `Hello! | fr: Bonjour ! | es: ¡Hola!`.)
    let cuts = language_cuts(said_text_alone);
    let first = cuts.first().map_or(said_text_alone.len(), |(at, _)| *at);
    let mut languages = Vec::new();
    for (rank, (at, code)) in cuts.iter().enumerate() {
        let end = cuts.get(rank + 1).map_or(said_text_alone.len(), |(next, _)| *next);
        let code_at = said_text_alone[*at..].find(code.as_str()).map_or(*at, |found| at + found);
        let from = code_at + code.len() + 1;
        let inside = &said_text_alone[from..end];
        let lead = inside.len() - inside.trim_start().len();
        let pieces = pieces(inside.trim(), text_at + from + lead, diagnostics);
        languages.push(Language { code: name_at(code, text_at + code_at), pieces });
    }
    let pieces = pieces(said_text_alone[..first].trim_end(), text_at, diagnostics);
    Read::Stmt(Kind::Said(Said { who, manners, text: said_text.to_string(), text_span, pieces, languages, id }), beside)
}

/// Where a said text gives itself in another language (` | fr:`, two or
/// three small letters, outside any `{…}`): each place of its `|`, and the code.
fn language_cuts(text: &str) -> Vec<(usize, String)> {
    let mut cuts = Vec::new();
    let mut depth = 0_usize;
    for (at, letter) in text.char_indices() {
        match letter {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => {
                let rest = text[at + 1..].trim_start();
                let code: String = rest.chars().take_while(char::is_ascii_lowercase).collect();
                if (2..=3).contains(&code.len()) && rest[code.len()..].starts_with(':') {
                    cuts.push((at, code));
                }
            }
            _ => {}
        }
    }
    cuts
}

/// The pieces of a said text: words, `{value}`, `[1s]`.
pub fn pieces(text: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> Vec<Piece> {
    let mut made = Vec::new();
    let mut plain = String::new();
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        let letter = rest.chars().next().unwrap_or(' ');
        if letter == '{' {
            let Some(close) = closing(rest) else {
                let span = Span::new(start + at, start + text.len());
                diagnostics.push(Diagnostic::error(codes::UNCLOSED_MARK, span, "a “{” is not closed").fix("close it", Span::new(span.end, span.end), "}"));
                plain.push_str(rest);
                break;
            };
            let inside = &rest[1..close];
            // (`{if test: words | other}`: words chosen as the film plays.)
            if inside.trim_start().starts_with("if ") {
                if let Some(chosen) = chosen(inside, start + at + 1, diagnostics) {
                    if !plain.is_empty() {
                        made.push(Piece::Text(std::mem::take(&mut plain)));
                    }
                    made.push(chosen);
                }
                at += close + 1;
                continue;
            }
            if let Some(value) = expr_or_note(inside.trim(), start + at + 1 + (inside.len() - inside.trim_start().len()), diagnostics) {
                if !plain.is_empty() {
                    made.push(Piece::Text(std::mem::take(&mut plain)));
                }
                made.push(Piece::Value(value));
            }
            at += close + 1;
            continue;
        }
        if letter == '['
            && let Some(close) = rest.find(']')
            && let Some(Token::Duration(pause)) = number_word(rest[1..close].trim())
        {
            if !plain.is_empty() {
                made.push(Piece::Text(std::mem::take(&mut plain)));
            }
            made.push(Piece::Pause(pause));
            at += close + 1;
            continue;
        }
        plain.push(letter);
        at += letter.len_utf8();
    }
    if !plain.is_empty() {
        made.push(Piece::Text(plain));
    }
    made
}

/// A line's id as written last (`#k3f9`): a letter, then letters, digits or
/// `_`; the id without its `#`.
pub fn line_id(word: &str) -> Option<&str> {
    let id = word.strip_prefix('#')?;
    (id.starts_with(|letter: char| letter.is_ascii_alphabetic()) && id.chars().all(|letter| letter.is_ascii_alphanumeric() || letter == '_')).then_some(id)
}

/// `if test: words | other` (inside its braces, starting at `start`): the
/// test, the words when it holds, the others (none: nothing).
fn chosen(inside: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> Option<Piece> {
    let lead = inside.len() - inside.trim_start().len();
    let body = inside.trim_start().strip_prefix("if ")?;
    let body_at = start + lead + 3;
    let Some(colon) = outside_brackets(body, ':') else {
        let span = Span::new(start, start + inside.len());
        diagnostics.push(Diagnostic::error(codes::EXPRESSION, span, "words chosen are written “{if test: words | other}”"));
        return None;
    };
    let test_text = &body[..colon];
    let test = expr_or_note(test_text.trim(), body_at + (test_text.len() - test_text.trim_start().len()), diagnostics)?;
    let branches = &body[colon + 1..];
    let branches_at = body_at + colon + 1;
    let (then, otherwise) = match outside_brackets(branches, '|') {
        Some(bar) => (&branches[..bar], &branches[bar + 1..]),
        None => (branches, ""),
    };
    let lead_of = |text: &str| text.len() - text.trim_start().len();
    let then_pieces = pieces(then.trim(), branches_at + lead_of(then), diagnostics);
    let otherwise_at = branches_at + then.len() + 1 + lead_of(otherwise);
    let otherwise_pieces = pieces(otherwise.trim(), otherwise_at, diagnostics);
    Some(Piece::Choose { test, then: then_pieces, otherwise: otherwise_pieces })
}

/// Where that letter first is in a text, outside brackets, braces and quotes.
fn outside_brackets(text: &str, wanted: char) -> Option<usize> {
    let (mut depth, mut quoted_now) = (0_i32, false);
    for (at, letter) in text.char_indices() {
        match letter {
            '"' => quoted_now = !quoted_now,
            '(' | '[' | '{' if !quoted_now => depth += 1,
            ')' | ']' | '}' if !quoted_now => depth -= 1,
            letter if letter == wanted && depth == 0 && !quoted_now => return Some(at),
            _ => {}
        }
    }
    None
}

/// `option text`, or `option "text" [if test] [once]`: an answer, offered
/// only when its test holds, only until taken if `once`.
fn option(rest: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> PartKind {
    let plain = |diagnostics: &mut Vec<Diagnostic>| PartKind::Option {
        text: rest.to_string(),
        words: rest.to_string(),
        pieces: pieces(rest, start, diagnostics),
        test: None,
        once: false,
    };
    let Some(quoted) = rest.strip_prefix('"') else {
        return plain(diagnostics);
    };
    let Some(close) = quoted.find('"') else {
        let span = Span::new(start, start + rest.len());
        diagnostics.push(Diagnostic::error(codes::UNCLOSED_MARK, span, "a “\"” is not closed").fix("close it", Span::new(span.end, span.end), "\""));
        return plain(diagnostics);
    };
    let words = &quoted[..close];
    let after_at = start + 1 + close + 1;
    let mut after = quoted[close + 1..].trim_end();
    let once = after == "once" || after.ends_with(" once");
    if once {
        after = after[..after.len() - "once".len()].trim_end();
    }
    let lead = after.len() - after.trim_start().len();
    let test = match after.trim_start() {
        "" => None,
        asked => match asked.strip_prefix("if ") {
            Some(test) => expr_or_note(test.trim(), after_at + lead + 3 + (test.len() - test.trim_start().len()), diagnostics),
            None => {
                let span = Span::new(after_at + lead, after_at + after.len());
                diagnostics.push(Diagnostic::error(codes::EXTRA, span, "after an answer in quotes: “if test”, “once”, or nothing"));
                None
            }
        },
    };
    PartKind::Option { text: rest.to_string(), words: words.to_string(), pieces: pieces(words, start + 1, diagnostics), test, once }
}

/// Where the `}` that closes the `{` starting this text is.
fn closing(text: &str) -> Option<usize> {
    let mut depth = 0;
    let mut quoted_now = false;
    for (at, letter) in text.char_indices() {
        match letter {
            '"' => quoted_now = !quoted_now,
            '{' if !quoted_now => depth += 1,
            '}' if !quoted_now => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
    }
    None
}

/// The arguments of a line, as written: words, numbers, durations, texts,
/// `{calculations}`.
pub fn args(text: &str, start: usize, diagnostics: &mut Vec<Diagnostic>) -> Vec<Arg> {
    let mut made = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        let letter = rest.chars().next().unwrap_or(' ');
        if letter.is_whitespace() {
            at += letter.len_utf8();
            continue;
        }
        let from = start + at;
        let (length, kind) = if letter == '{' {
            match closing(rest) {
                Some(close) => {
                    let inside = &rest[1..close];
                    let value = expr_or_note(inside.trim(), from + 1 + (inside.len() - inside.trim_start().len()), diagnostics);
                    (close + 1, value.map_or(ArgKind::Word, ArgKind::Value))
                }
                None => {
                    let span = Span::new(from, start + text.len());
                    diagnostics.push(Diagnostic::error(codes::UNCLOSED_MARK, span, "a “{” is not closed").fix("close it", Span::new(span.end, span.end), "}"));
                    (rest.len(), ArgKind::Word)
                }
            }
        } else if letter == '"' {
            match quoted(rest) {
                Some((said, length)) => (length, ArgKind::Text(said)),
                None => {
                    let span = Span::new(from, start + text.len());
                    diagnostics.push(Diagnostic::error(codes::UNCLOSED_MARK, span, "a quoted text is not closed").fix(
                        "close the text",
                        Span::new(span.end, span.end),
                        "\"",
                    ));
                    (rest.len(), ArgKind::Word)
                }
            }
        } else {
            let length = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let kind = match number_word(&rest[..length]) {
                Some(Token::Number(number)) => ArgKind::Number(number),
                Some(Token::Duration(duration)) => ArgKind::Duration(duration),
                _ => ArgKind::Word,
            };
            (length, kind)
        };
        made.push(Arg { span: Span::new(from, from + length), text: rest[..length].to_string(), kind });
        at += length;
    }
    made
}

/// A form: its word, then what it takes (`who:pokemon to where:place
/// [turns:number=3]`), as the vocabulary and the recipes write them.
pub fn form(text: &str, start: usize) -> Result<Form, Diagnostic> {
    let pieces = form_pieces(text, start);
    let Some(((word, word_span), rest)) = pieces.split_first() else {
        return Err(Diagnostic::error(codes::FORM, Span::new(start, start), "“define” expects the name of the recipe"));
    };
    if name_length(word) != word.len() {
        return Err(Diagnostic::error(codes::FORM, *word_span, format!("“{word}” is not a recipe name")));
    }
    let mut at = 0;
    let elements = elements(rest, &mut at, false)?;
    Ok(Form { word: Name { text: word.to_string(), span: *word_span }, elements })
}

fn form_pieces(text: &str, start: usize) -> Vec<(String, Span)> {
    let mut made = Vec::new();
    let mut word = String::new();
    let mut word_start = 0;
    for (at, letter) in text.char_indices() {
        if letter.is_whitespace() || matches!(letter, '[' | ']' | '|') {
            if !word.is_empty() {
                made.push((std::mem::take(&mut word), Span::new(start + word_start, start + at)));
            }
            if !letter.is_whitespace() {
                made.push((letter.to_string(), Span::new(start + at, start + at + 1)));
            }
        } else {
            if word.is_empty() {
                word_start = at;
            }
            word.push(letter);
        }
    }
    if !word.is_empty() {
        made.push((word, Span::new(start + word_start, start + text.len())));
    }
    made
}

fn elements(pieces: &[(String, Span)], at: &mut usize, inside: bool) -> Result<Vec<Element>, Diagnostic> {
    let mut runs: Vec<Vec<Element>> = vec![Vec::new()];
    while let Some((piece, span)) = pieces.get(*at) {
        *at += 1;
        match piece.as_str() {
            "[" => {
                let optional = elements(pieces, at, true)?;
                if let Some(run) = runs.last_mut() {
                    run.push(Element::Optional(optional));
                }
            }
            "]" if inside => break,
            "]" => return Err(Diagnostic::error(codes::FORM, *span, "a “]” without its “[”")),
            "|" => runs.push(Vec::new()),
            _ => {
                let element = element(piece, *span)?;
                if let Some(run) = runs.last_mut() {
                    run.push(element);
                }
            }
        }
    }
    if inside && pieces.get(*at - 1).is_none_or(|(piece, _)| piece != "]") {
        return Err(Diagnostic::error(codes::FORM, pieces.last().map(|(_, span)| *span).unwrap_or_default(), "a “[” is not closed"));
    }
    Ok(if runs.len() == 1 { runs.pop().unwrap_or_default() } else { vec![Element::OneOf(runs)] })
}

fn element(piece: &str, span: Span) -> Result<Element, Diagnostic> {
    let (many, bare) = match piece.strip_prefix('*') {
        Some(bare) => (true, bare),
        None => (false, piece),
    };
    let (head, default) = match bare.split_once('=') {
        Some((head, default)) => (head, Some(default.to_string())),
        None => (bare, None),
    };
    let start = span.start + usize::from(many);
    if let Some(name) = head.strip_suffix("...") {
        return Ok(Element::Param { name: Name { text: name.to_string(), span: Span::new(start, start + name.len()) }, ty: None, default, many, rest: true });
    }
    if let Some((name, ty)) = head.split_once(':') {
        let ty = type_of(ty, start + name.len() + 1)?;
        return Ok(Element::Param {
            name: Name { text: name.to_string(), span: Span::new(start, start + name.len()) },
            ty: Some(ty),
            default,
            many,
            rest: false,
        });
    }
    if many || default.is_some() {
        return Err(Diagnostic::error(codes::FORM, span, format!("“{piece}”: an argument has its type, as “{}:pokemon”", head)));
    }
    Ok(Element::Word(Name { text: piece.to_string(), span }))
}

/// `pokemon`, `list<place>`
pub fn type_of(text: &str, start: usize) -> Result<Type, Diagnostic> {
    if let Some((name, inner)) = text.split_once('<') {
        let Some(inner) = inner.strip_suffix('>') else {
            return Err(Diagnostic::error(codes::FORM, Span::new(start, start + text.len()), "a “<” without its “>”"));
        };
        let of = type_of(inner, start + name.len() + 1)?;
        return Ok(Type { name: name.to_string(), of: Some(Box::new(of)) });
    }
    if text.is_empty() || name_length(text) != text.len() {
        return Err(Diagnostic::error(codes::FORM, Span::new(start, start + text.len()), format!("“{text}” is not a type")));
    }
    Ok(Type { name: text.to_string(), of: None })
}

impl Duration {
    /// As written: `1.5s`, `6f`.
    pub fn written(self) -> String {
        match self {
            Duration::Seconds(seconds) => format!("{}s", number_text(seconds)),
            Duration::Frames(frames) => format!("{}f", number_text(frames)),
        }
    }
}

/// A number as the language writes it: `3`, `0.5`, `-12`.
pub fn number_text(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 { format!("{}", number as i64) } else { format!("{number}") }
}
