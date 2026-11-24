//! A film one plays, looked into: its numbers (where each is declared, given, read; what each is at the moment
//! shown), and the text played with other values for those declared (`let x = …`), its lines kept where they are.

use std::collections::BTreeMap;

/// A number of the film: its declaration (its line, from 1, and its value as written), the lines it is given at,
/// and read at.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Variable {
    pub declared: Option<(usize, String)>,
    pub given: Vec<usize>,
    pub read: Vec<usize>,
}

/// The words that open a line reading numbers.
const READERS: [&str; 6] = ["if", "else", "while", "when", "repeat", "wait"];

/// The film's numbers, by their names.
pub fn variables(text: &str) -> BTreeMap<String, Variable> {
    let mut found: BTreeMap<String, Variable> = BTreeMap::new();
    let lines: Vec<&str> = text.split('\n').collect();
    let word = |name: &str| name.chars().all(|letter| letter.is_alphanumeric() || letter == '_') && name.chars().next().is_some_and(char::is_alphabetic);
    for (at, raw) in lines.iter().enumerate() {
        let number = at + 1;
        let line = raw.split('#').next().unwrap_or_default().trim();
        if let Some(rest) = line.strip_prefix("let ")
            && let Some((name, value)) = rest.split_once('=')
            && word(name.trim())
        {
            found.entry(name.trim().to_string()).or_default().declared = Some((number, value.trim().to_string()));
            continue;
        }
        if let Some((name, _)) = line.split_once('=')
            && !line.contains(':')
            && word(name.trim())
            && !name.trim().contains(' ')
        {
            found.entry(name.trim().to_string()).or_default().given.push(number);
            continue;
        }
        let mut words = line.split_whitespace();
        if let Some(first @ ("stow" | "take_out")) = words.next() {
            let thing: Vec<&str> = words.filter(|word| word.parse::<f32>().is_err()).collect();
            if !thing.is_empty() {
                let name = format!("bag_{}", film::words::plain(&thing.join("_")));
                let _ = first;
                found.entry(name).or_default().given.push(number);
            }
        }
    }
    // (Read: in a line that tests, or written in `{…}`.)
    let names: Vec<String> = found.keys().cloned().collect();
    for (at, raw) in lines.iter().enumerate() {
        let number = at + 1;
        let line = raw.split('#').next().unwrap_or_default().trim();
        let first = line.split_whitespace().next().unwrap_or_default();
        let tested = READERS.contains(&first);
        let tokens: Vec<String> =
            line.split(|letter: char| !(letter.is_alphanumeric() || letter == '_')).filter(|token| !token.is_empty()).map(str::to_string).collect();
        for token in &tokens {
            let named = names.contains(token) || (tested && token.starts_with("bag_"));
            if !named {
                continue;
            }
            let braced = line.contains(&format!("{{{token}}}"));
            let entry = found.entry(token.clone()).or_default();
            if (tested || braced)
                && !entry.read.contains(&number)
                && entry.declared.as_ref().is_none_or(|(line, _)| *line != number)
                && !entry.given.contains(&number)
            {
                entry.read.push(number);
            }
        }
    }
    found
}

/// What a number is now, among those the engine holds (a declared one is kept as `_name`).
pub fn value_now(numbers: &BTreeMap<String, f64>, name: &str) -> Option<f64> {
    numbers.get(name).or_else(|| numbers.get(&format!("_{name}"))).copied()
}

/// The text with those declared given other values (the `let` lines rewritten, no line added or taken).
pub fn with_values(text: &str, values: &BTreeMap<String, f64>) -> String {
    text.split('\n')
        .map(|line| {
            let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
            if let Some(rest) = line.trim_start().strip_prefix("let ")
                && let Some((name, _)) = rest.split_once('=')
                && let Some(value) = values.get(name.trim())
            {
                return format!("{lead}let {} = {value}", name.trim());
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILM: &str = "let seen = 0\nscene: beach a b\ndialogue b\n  if seen\n    b: You again, {seen} times!\n  end\n  seen = 1\nend\nstow 2 Golden Apple\nif bag_golden_apple > 0\n  a: Yum\nend\n";

    #[test]
    fn numbers_are_found_where_declared_given_and_read() {
        let found = variables(FILM);
        let seen = &found["seen"];
        assert_eq!(seen.declared, Some((1, "0".to_string())));
        assert_eq!(seen.given, vec![7]);
        assert_eq!(seen.read, vec![4, 5]);
        assert_eq!(found["bag_golden_apple"].given, vec![9]);
        assert_eq!(found["bag_golden_apple"].read, vec![10]);
        let values = BTreeMap::from([("seen".to_string(), 3.0)]);
        let played = with_values(FILM, &values);
        assert!(played.starts_with("let seen = 3\n"));
        assert_eq!(played.split('\n').count(), FILM.split('\n').count(), "not one line more");
        assert_eq!(value_now(&BTreeMap::from([("_seen".to_string(), 1.0)]), "seen"), Some(1.0));
    }
}
