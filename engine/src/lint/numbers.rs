//! The numbers a file asks for that nothing gives.

use super::*;

/// The commands that give a number, by the name after them.
const GIVING: [&str; 7] = ["set", "add", "random", "calc", "measure", "sees", "settext"];

/// The words of the tests.
const WORDS: [&str; 8] = ["and", "or", "==", "!=", "<", "<=", ">", ">="];

/// A number's name: not a number, not someone as written, not a word of
/// tests. (One made of numbers, `n{i}`, is known when played only.)
fn named(token: &str) -> bool {
    let someone = token.starts_with(['m', 'i', 'p', 'o']) && token[1..].starts_with(|c: char| c.is_ascii_digit());
    token.starts_with(|c: char| c.is_alphabetic() || c == '_') && !someone && !WORDS.contains(&token) && token.parse::<i32>().is_err() && !token.contains('{')
}

/// Where a line gives a number: the place of its command, its own or the
/// one `after` or `every` plays.
fn giving_at(fields: &[String]) -> Option<usize> {
    fields.iter().take(3).position(|field| GIVING.contains(&field.as_str()))
}

/// The numbers a line gives.
fn given_by(fields: &[String]) -> Vec<String> {
    match fields[0].as_str() {
        // (Where someone stands, as two numbers.)
        "position" => fields.iter().skip(2).take(2).cloned().collect(),
        "facing" => fields.iter().skip(1).take(2).cloned().collect(),
        // (Where a place was found.)
        "find" | "seats" => fields.get(1).map(|var| vec![format!("{var}_x"), format!("{var}_y")]).unwrap_or_default(),
        "gathers" => (0..fields.len().saturating_sub(5)).flat_map(|at| [format!("{}{at}_x", fields[1]), format!("{}{at}_y", fields[1])]).collect(),
        // (Those another scene gives.)
        "uses" => fields[1..].iter().flat_map(|names| names.split(',')).map(str::to_string).collect(),
        _ => giving_at(fields).and_then(|at| fields.get(at + 1)).cloned().into_iter().collect(),
    }
}

/// The numbers a command takes by name.
fn commands_ask(fields: &[String], words: &Vocabulary) -> Vec<String> {
    let mut asked = Vec::new();
    if let Some(at) = giving_at(fields) {
        let values = match fields[at].as_str() {
            "calc" => vec![at + 2, at + 4],
            "set" | "add" => vec![at + 2],
            _ => Vec::new(),
        };
        asked.extend(values.into_iter().filter_map(|value| fields.get(value)).filter(|value| named(value)).cloned());
    }
    let by_name = match fields[0].as_str() {
        word if words.host_numbers.iter().any(|(named, _)| *named == word) => {
            words.host_numbers.iter().find(|(named, _)| *named == word).and_then(|(_, at)| fields.get(*at))
        }
        // (A gauge: its number, after its place if it has one; its most
        // may be a name too.)
        "gauge" => {
            let from = if fields.get(1).is_some_and(|place| crate::beat::hud_place(place).is_some()) { 2 } else { 1 };
            asked.extend(fields.get(from + 1).filter(|most| named(most)).cloned());
            fields.get(from).filter(|var| var.as_str() != "off")
        }
        _ => None,
    };
    asked.extend(by_name.map(|name| name.replace(['{', '}', '-'], "")).filter(|name| named(name)));
    asked
}

/// The numbers a test asks, not the words kept that it compares.
fn tests_ask(fields: &[String], texts: &HashSet<&String>) -> Vec<String> {
    if !matches!(fields[0].as_str(), "if" | "elif" | "while" | "waituntil") {
        return Vec::new();
    }
    let compared = |at: usize| at >= 3 && texts.contains(&fields[at - 2]);
    fields.iter().enumerate().skip(1).filter(|(at, token)| named(token) && !compared(*at) && !texts.contains(token)).map(|(_, token)| token.clone()).collect()
}

/// The numbers named in words (`{name}`), and in an answer offered only
/// so (`[if ...]`).
fn words_ask(fields: &[String]) -> Vec<String> {
    let mut asked = Vec::new();
    for field in fields {
        for (start, _) in field.match_indices('{') {
            if let Some(length) = field[start + 1..].find('}') {
                let name = &field[start + 1..start + 1 + length];
                if !name.contains(':') && named(name) {
                    asked.push(name.to_string());
                }
            }
        }
        for answer in field.split('|') {
            if let (Some(test), _) = crate::beat::answer_test(answer) {
                asked.extend(test.any.iter().flatten().map(|condition| condition.var.clone()));
            }
        }
    }
    asked
}

/// Numbers named as a word of the tests: they would read wrong.
fn reserved(lines: &[(usize, Vec<String>)]) -> Vec<Diagnostic> {
    const RESERVED: [&str; 2] = ["and", "or"];
    let named = lines.iter().filter_map(|(number, fields)| Some((number, fields.get(giving_at(fields)? + 1)?)));
    named
        .filter(|(_, var)| RESERVED.contains(&var.as_str()))
        .map(|(number, var)| Diagnostic::at(number, format!("“{var}” is a word of the conditions (and, or): give this number another name")))
        .collect()
}

pub(super) fn numbers_never_given(lines: &[(usize, Vec<String>)], words: &Vocabulary) -> Vec<Diagnostic> {
    // (Words kept by a name: compared with a word, `if answer == pikachu`.)
    let texts: HashSet<&String> =
        lines.iter().filter(|(_, fields)| fields[0] == "settext").filter_map(|(_, fields)| fields.get(1)).collect();
    let given: HashSet<String> = lines.iter().flat_map(|(_, fields)| given_by(fields)).collect();
    let known =
        |name: &str| given.contains(name) || words.given_numbers.contains(&name) || words.given_beginnings.iter().any(|beginning| name.starts_with(beginning));
    let mut told = HashSet::new();
    let mut problems = reserved(lines);
    for (number, fields) in lines.iter().filter(|(_, fields)| fields[0] != "uses") {
        let asked = [commands_ask(fields, words), tests_ask(fields, &texts), words_ask(fields)].concat();
        for name in asked.into_iter().filter(|name| !known(name)) {
            if told.insert(name.clone()) {
                problems.push(Diagnostic::at(number, format!("the number “{name}” is never given (set, add, random, calc, measure): a typo?")));
            }
        }
    }
    problems
}

/// A field with what its braces name as 1 (`{x}`, `n{i}`: `1`, `n1`).
pub(crate) fn braces_as_one(field: &str) -> String {
    let mut field = field.to_string();
    while let Some(close) = field.find('}') {
        let Some(open) = field[..close].rfind('{') else {
            break;
        };
        field.replace_range(open..=close, "1");
    }
    field
}
