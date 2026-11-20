//! What is said in a film, as it is well written: two spaces, a word twice, two dots, straight quotes, a space
//! before a comma, none after it, a small letter to begin; and, if asked, a space before ! ? : ; as French
//! typesetting has it. Each told with its fix, as the language's remarks are (a detail, its code [`CODE`]).

use film::service::{Diagnostic, Edit, Fix, Location, Severity, Spot};

/// The code of what typography finds.
pub const CODE: &str = "typo";

/// What is said on a line: where it begins (byte), if the line is said by someone.
fn said_from(line: &str) -> Option<usize> {
    crate::script::speaker(line).map(|(_, at)| at)
}

/// One finding: on that line (from 1), from a column to another (from 1, its end past it), said so, fixed so.
fn told(line: usize, from: usize, to: usize, message: String, title: &str, with: String) -> Diagnostic {
    let span = Location::between(Spot { line, column: from }, Spot { line, column: to });
    Diagnostic {
        span: span.clone(),
        severity: Severity::Detail,
        code: Some(CODE),
        message,
        notes: Vec::new(),
        mentions: Vec::new(),
        fixes: vec![Fix { title: title.to_string(), edits: vec![Edit { span, text: with }] }],
    }
}

/// Every finding of the text's lines said.
pub fn checked(text: &str, french_spaces: bool) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for (number, line) in text.split('\n').enumerate() {
        let line_number = number + 1;
        let Some(start) = said_from(line) else { continue };
        if line.trim_start().starts_with('#') {
            continue;
        }
        // (The said part as letters, with the column (from 1) of each.)
        let first_column = line[..start].chars().count() + 1;
        let said: Vec<char> = line[start..].chars().collect();
        let column = |at: usize| first_column + at;
        let mut at = 0;
        // (Its spaces at its start: none of them counts.)
        while at < said.len() && said[at] == ' ' {
            at += 1;
        }
        let begins = at;
        if let Some(letter) = said.get(begins).filter(|letter| letter.is_lowercase()) {
            let capital: String = letter.to_uppercase().collect();
            found.push(told(line_number, column(begins), column(begins) + 1, "a line begins with a capital letter".to_string(), "Capital", capital));
        }
        let mut index = begins;
        while index < said.len() {
            let letter = said[index];
            // (Two spaces or more, inside.)
            if letter == ' ' && said.get(index + 1) == Some(&' ') {
                let end = (index..said.len()).find(|at| said[*at] != ' ').unwrap_or(said.len());
                if end < said.len() {
                    found.push(told(line_number, column(index), column(end), "two spaces".to_string(), "One space", " ".to_string()));
                }
                index = end;
                continue;
            }
            // (Two dots: three, as dots that trail.)
            if letter == '.' && said.get(index + 1) == Some(&'.') && said.get(index + 2) != Some(&'.') && (index == 0 || said[index - 1] != '.') {
                found.push(told(line_number, column(index), column(index + 2), "two dots in a row: an ellipsis has three".to_string(), "…", "...".to_string()));
            }
            // (A space before a comma, or before a dot that ends.)
            if letter == ' ' && matches!(said.get(index + 1), Some(',')) {
                found.push(told(line_number, column(index), column(index + 1), "no space before a comma".to_string(), "No space", String::new()));
            }
            // (No space after a comma or a semicolon, before a letter.)
            if matches!(letter, ',' | ';') && said.get(index + 1).is_some_and(|next| next.is_alphabetic()) {
                found.push(told(line_number, column(index + 1), column(index + 1), format!("a space after \"{letter}\""), "One space", " ".to_string()));
            }
            // (French typesetting: a space before ! ? : ;, if asked.)
            if french_spaces && matches!(letter, '!' | '?' | ':' | ';') && index > begins && said[index - 1].is_alphanumeric() {
                found.push(told(
                    line_number,
                    column(index),
                    column(index),
                    format!("a space before \"{letter}\" (French typesetting)"),
                    "One space",
                    " ".to_string(),
                ));
            }
            index += 1;
        }
        // (Straight quotes, by pairs: curly ones.)
        let quotes: Vec<usize> = (begins..said.len()).filter(|at| said[*at] == '"').collect();
        for pair in quotes.chunks(2).filter(|pair| pair.len() == 2) {
            let inner: String = said[pair[0] + 1..pair[1]].iter().collect();
            found.push(told(
                line_number,
                column(pair[0]),
                column(pair[1] + 1),
                "curly quotes".to_string(),
                "\u{201C} \u{201D}",
                format!("\u{201C}{}\u{201D}", inner.trim()),
            ));
        }
        // (A word twice: "the the".)
        let words: Vec<(usize, String)> = {
            let mut words = Vec::new();
            let mut current = String::new();
            let mut from = 0;
            for (at, letter) in said.iter().enumerate() {
                if letter.is_alphabetic() || *letter == '\'' {
                    if current.is_empty() {
                        from = at;
                    }
                    current.push(*letter);
                } else {
                    if !current.is_empty() {
                        words.push((from, std::mem::take(&mut current)));
                    }
                    if *letter != ' ' {
                        words.push((at, String::new()));
                    }
                }
            }
            if !current.is_empty() {
                words.push((from, current));
            }
            words
        };
        for pair in words.windows(2) {
            let ((_, first), (second_at, second)) = (&pair[0], &pair[1]);
            if !first.is_empty() && first.chars().count() > 1 && first.to_lowercase() == second.to_lowercase() {
                let gone_from = said[..*second_at].iter().rposition(|letter| *letter != ' ').map_or(*second_at, |at| at + 1);
                found.push(told(
                    line_number,
                    column(gone_from),
                    column(second_at + second.chars().count()),
                    format!("\"{first}\" twice"),
                    "Once",
                    String::new(),
                ));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(text: &str, french: bool) -> String {
        let mut text = text.to_string();
        // (One fix at a time, the last first, so the others stay where they are.)
        loop {
            let found = checked(&text, french);
            let Some(last) = found.iter().max_by_key(|told| (told.span.start.line, told.span.start.column)) else { return text };
            text = film::service::applied(&text, &last.fixes[0].edits);
        }
    }

    #[test]
    fn what_is_said_is_put_as_it_is_well_written() {
        assert_eq!(
            fixed("pikachu (joyous): hello  you..how are ,you? the the cat \"yes\"", false),
            "pikachu (joyous): Hello you...how are, you? the cat \u{201C}yes\u{201D}"
        );
        assert_eq!(fixed("piplup: Hey! Wait: no?", true), "piplup: Hey ! Wait : no ?");
        assert!(checked("scene: beach a b\nwalk a to right\n# a comment", true).is_empty(), "nothing is said");
        assert!(checked("a: Well... really.", false).is_empty(), "three dots are right");
    }
}
