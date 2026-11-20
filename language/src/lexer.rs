//! The tokens of a calculation: numbers, durations, texts, names and signs,
//! each with its span in the whole text.

use crate::ast::Duration;
use crate::diag::{Diagnostic, codes};
use crate::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Number(f64),
    Duration(Duration),
    Text(String),
    Name(String),
    Sign(&'static str),
}

#[derive(Clone, Debug)]
pub struct Lexed {
    pub token: Token,
    pub span: Span,
}

const SIGNS: [&str; 24] = ["<=", ">=", "!=", "->", "...", "(", ")", "[", "]", "{", "}", ",", ":", "+", "-", "*", "/", "%", "<", ">", "=", ".", "&", "|"];

/// Whether that letter goes in a name.
pub fn in_name(letter: char) -> bool {
    letter.is_alphanumeric() || letter == '_'
}

/// The length in bytes of the name that starts the text (letters, digits,
/// `_`; a `'` or a `-` between two letters; a `.` before a digit, as in
/// `m25.1`).
pub fn name_length(text: &str) -> usize {
    let letters: Vec<(usize, char)> = text.char_indices().collect();
    let mut at = 0;
    while at < letters.len() {
        let letter = letters[at].1;
        let next = letters.get(at + 1).map(|(_, next)| *next);
        let joined = match letter {
            '\'' | '-' => at > 0 && next.is_some_and(char::is_alphabetic),
            '.' => at > 0 && next.is_some_and(|next| next.is_ascii_digit()),
            _ => in_name(letter),
        };
        if !joined {
            break;
        }
        at += 1;
    }
    letters.get(at).map_or(text.len(), |(byte, _)| *byte)
}

/// A number written at the start of the text, with its length: digits,
/// maybe a `.` and digits.
pub fn number_length(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() && bytes[at].is_ascii_digit() {
        at += 1;
    }
    if at > 0 && at + 1 < bytes.len() && bytes[at] == b'.' && bytes[at + 1].is_ascii_digit() {
        at += 1;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
    }
    at
}

/// A number or a duration that is the whole of that word (`3`, `-12`,
/// `0.5`, `1.5s`, `6f`).
pub fn number_word(word: &str) -> Option<Token> {
    let (sign, digits) = match word.as_bytes().first() {
        Some(b'-') => (-1.0, &word[1..]),
        Some(b'+') => (1.0, &word[1..]),
        _ => (1.0, word),
    };
    let length = number_length(digits);
    if length == 0 {
        return None;
    }
    let value: f64 = digits[..length].parse().ok()?;
    match &digits[length..] {
        "" => Some(Token::Number(sign * value)),
        "s" => Some(Token::Duration(Duration::Seconds(sign * value))),
        "f" => Some(Token::Duration(Duration::Frames(sign * value))),
        _ => None,
    }
}

/// The tokens of that text, which starts at `base` in the whole text.
pub fn tokens(text: &str, base: usize) -> Result<Vec<Lexed>, Diagnostic> {
    let mut made = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        let letter = rest.chars().next().unwrap_or(' ');
        if letter.is_whitespace() {
            at += letter.len_utf8();
            continue;
        }
        let start = at;
        let token = if letter.is_ascii_digit() {
            let length = number_length(rest);
            let value: f64 = rest[..length].parse().unwrap_or(0.0);
            at += length;
            let unit = rest[length..].chars().next();
            let after = rest[length..].chars().nth(1);
            match unit {
                Some('s') if !after.is_some_and(in_name) => {
                    at += 1;
                    Token::Duration(Duration::Seconds(value))
                }
                Some('f') if !after.is_some_and(in_name) => {
                    at += 1;
                    Token::Duration(Duration::Frames(value))
                }
                _ => Token::Number(value),
            }
        } else if letter == '$' && rest[1..].starts_with(in_name) {
            // (`$1`, `$name`: what a sequence of the engine's is given.)
            let length = 1 + name_length(&rest[1..]);
            at += length;
            Token::Name(rest[..length].to_string())
        } else if in_name(letter) {
            let length = name_length(rest);
            at += length;
            Token::Name(rest[..length].to_string())
        } else if letter == '"' {
            let (said, length) = quoted(rest).ok_or_else(|| {
                Diagnostic::error(codes::UNCLOSED_MARK, Span::new(base + start, base + text.len()), "a quoted text is not closed").fix(
                    "close the text",
                    Span::new(base + text.len(), base + text.len()),
                    "\"",
                )
            })?;
            at += length;
            Token::Text(said)
        } else if rest.starts_with("==") {
            let span = Span::new(base + at, base + at + 2);
            return Err(Diagnostic::error(codes::OLD_FORM, span, "“==” is written “=”").fix("write “=”", span, "="));
        } else if let Some(sign) = SIGNS.iter().find(|sign| rest.starts_with(**sign)) {
            at += sign.len();
            Token::Sign(sign)
        } else {
            let span = Span::new(base + at, base + at + letter.len_utf8());
            return Err(Diagnostic::error(codes::EXPRESSION, span, format!("“{letter}” means nothing in a calculation")));
        };
        made.push(Lexed { token, span: Span::new(base + start, base + at) });
    }
    Ok(made)
}

/// The text between the quotes that start this one, and the length of the
/// whole (`\"` is a quote, `\\` a backslash).
pub fn quoted(text: &str) -> Option<(String, usize)> {
    let mut said = String::new();
    let mut letters = text.char_indices().skip(1);
    while let Some((at, letter)) = letters.next() {
        match letter {
            '"' => return Some((said, at + 1)),
            '\\' => {
                if let Some((_, next)) = letters.next() {
                    said.push(next);
                }
            }
            _ => said.push(letter),
        }
    }
    None
}
