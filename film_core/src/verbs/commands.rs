//! Every command of the engine a word of the language: those the language
//! gives no word of their own (`engine.rs`, the groups) are words here,
//! each with the forms the engine declares for it (`engine::forms`, next to
//! its beat's reading in `engine/src/beat/`) and what it does. Nothing is
//! read from the docs: `engine/docs/COMMANDS.md` has its table of forms
//! written from the engine, and a test here checks its rows by theme say
//! the same.

use super::{Group, Lexicon, Word};

/// The commands with no word of their own that last until who goes gets
/// there (their time in the registry: the others are instant).
const WALKS: [&str; 1] = ["runto"];

/// The commands that keep a form of their own though a word of the films
/// has their name, told apart by their forms (`fade 30`, not `fade music`).
const KEPT: [&str; 7] = ["banner", "fade", "flee", "follow", "ride", "shake", "thing"];

/// The engine's commands no word of the films names: each its forms as the
/// engine declares them and what it writes (`name: what it does`). `runto`
/// lasts until who goes gets there; the others are instant. Read once by
/// the lexicon (`Lexicon::commands`): the words of its groups are taken.
pub(super) fn group(lexicon: &'static Lexicon) -> Group {
    let taken = |own: &str| !KEPT.contains(&own) && lexicon.all_groups_but_commands().any(|group| group.words.iter().any(|known| known.word == own));
    let words: Vec<Word> = engine::commands()
        .iter()
        .filter(|own| !taken(own))
        .map(|&own| {
            let form = engine::form_of(own).unwrap_or_else(|| panic!("“{own}”: the engine declares no form for it"));
            let writes: &'static str = if form.does.is_empty() { own } else { Box::leak(format!("{own}: {}", form.does).into_boxed_str()) };
            let word = Word::new(own, form.forms, writes);
            if WALKS.contains(&own) { word.until() } else { word }
        })
        .collect();
    Group::new("commands", Box::leak(words.into_boxed_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCS: &str = include_str!("../../../engine/docs/COMMANDS.md");

    /// Each command of the docs' tables, its forms and what it writes,
    /// as the docs write them.
    fn from_docs() -> Vec<(&'static str, Vec<String>, String)> {
        let mut forms: Vec<(&'static str, Vec<String>, String)> = Vec::new();
        // (The rows by theme: not the table of forms the engine writes.)
        let by_theme = DOCS.split("<!-- forms:").next().unwrap_or(DOCS);
        for row in by_theme.lines().filter(|row| row.starts_with("| `")) {
            let cells: Vec<&str> = row.split(" | ").collect();
            let first = cells[0].trim_start_matches("| ");
            let said = cells.get(1).map_or("", |cell| cell.trim_end_matches(" |").trim());
            for written in first.split(" / ") {
                let written = written.trim().trim_matches('`');
                let mut tokens = written.split_whitespace();
                let Some(own) = tokens.next().and_then(|first| engine::commands().iter().find(|own| **own == first).copied()) else { continue };
                for one in either(written[written.find(' ').unwrap_or(written.len())..].trim()) {
                    let form = form_of(&one);
                    match forms.iter_mut().find(|(known, ..)| *known == own) {
                        Some((_, known, _)) => {
                            if !known.contains(&form) {
                                known.push(form);
                            }
                        }
                        None => forms.push((own, vec![form], said.to_string())),
                    }
                }
            }
        }
        forms
    }

    /// A row of the docs that says someone or a point, `<who|@ x y>`
    /// (`lightning`, `icon`): one form for someone, one for the point
    /// (`<who>`, `@ <x> <y>`); any other as it is.
    fn either(written: &str) -> Vec<String> {
        let written = written.replace("\\|@ x y>", "|@ x y>");
        let Some((start, end)) = written.find("|@ x y>").and_then(|end| written[..end].rfind('<').map(|start| (start, end))) else {
            return vec![written];
        };
        let (before, name, after) = (&written[..start], &written[start + 1..end], &written[end + "|@ x y>".len()..]);
        either(&format!("{before}<{name}>{after}")).into_iter().chain(either(&format!("{before}@ <x> <y>{after}"))).collect()
    }

    /// What a bare word between brackets of the docs stands for, when it is
    /// no word written as it is but a value that may be left out (`[height]`,
    /// `[frames]`, `[x y]`): its kind (`...`: the words left). A word not
    /// here is written as it is (`[loop]`, `[stop]`, `[plus]`).
    fn left_out_kind(name: &str) -> Option<&'static str> {
        if let Some(kind) = kind_of(name) {
            return Some(kind);
        }
        Some(match name {
            "distance" | "laps" | "level" | "angle" | "offset" | "v" | "p" => "number",
            "side" | "place" | "property" | "rhythm" | "colour" | "entrance" | "number" | "words" => "word",
            _ => return None,
        })
    }

    /// A form of the language from a row of the docs: `<who> <x> [<n>]` →
    /// `who:code x:coordinate [n:number]`, `[height]` → `[height:number]`
    /// (a value left out, `left_out_kind`), `…` the words left, a word as it is.
    fn form_of(written: &str) -> String {
        let spaced = written.replace('[', " [ ").replace(']', " ] ");
        let mut names: Vec<String> = Vec::new();
        let mut out: Vec<String> = Vec::new();
        let mut depth = 0_usize;
        for token in spaced.split_whitespace() {
            if token == "[" || token == "]" {
                depth = if token == "[" { depth + 1 } else { depth.saturating_sub(1) };
                out.push(token.to_string());
                continue;
            }
            // (Between brackets, a value's name left out (`[height]`): typed, never that very word.)
            if let Some(kind) = left_out_kind(token).filter(|_| depth > 0 && token.chars().all(|letter| letter.is_ascii_lowercase() || letter == '_')) {
                let name = name_once(&mut names, token);
                if kind == "..." {
                    out.push(name + "...");
                    break;
                }
                out.push(format!("{name}:{kind}"));
                continue;
            }
            if token.contains('…') || token == "..." {
                out.push(name_once(&mut names, "rest") + "...");
                continue;
            }
            if let Some(inner) = token.strip_prefix('<').and_then(|inner| inner.split('>').next()) {
                let name = name_once(&mut names, inner);
                match kind_of(&crate::words::plain(inner)) {
                    // (Words with spaces, a test, a command: all the words left.)
                    Some("...") => {
                        out.push(name + "...");
                        break;
                    }
                    Some(kind) => out.push(format!("{name}:{kind}")),
                    None => out.push(name + ":word"),
                }
                continue;
            }
            // (A word as it is; what is not one, `{x}`, `"…"`: the words left.)
            if token.chars().all(|letter| letter.is_alphanumeric() || matches!(letter, '|' | '_' | '-' | '@' | '!' | '?')) {
                out.push(token.to_string());
            } else {
                out.push(name_once(&mut names, "rest") + "...");
                break;
            }
        }
        // (A `[` never closed is closed at the end.)
        let opened = out.iter().filter(|token| *token == "[").count();
        let closed = out.iter().filter(|token| *token == "]").count();
        out.extend((closed..opened).map(|_| "]".to_string()));
        out.join(" ").replace("[ ", "[").replace(" ]", "]")
    }

    /// What a name of the docs' forms is: a duration (`1.5s`, `6f`, frames), a
    /// number, all the words left; else a word (someone, `x` that may be a
    /// spot `@name`, a name of the game's).
    fn kind_of(name: &str) -> Option<&'static str> {
        Some(match name {
            "frames" | "time" | "duration" | "delay" => "duration",
            "n" | "times" | "turns" | "strength" | "radius" | "height" | "pixels" | "speed" | "value" | "size" | "scale" | "zoom" | "dx" | "dy" | "gap"
            | "r" | "g" | "b" | "width" => "number",
            "text" | "lines" | "words" | "command" | "condition" | "sequence" => "...",
            "who" | "other" | "leader" => "code",
            "x" | "y" | "x1" | "y1" | "x2" | "y2" => "coordinate",
            _ => return None,
        })
    }

    /// A name of a form, its letters plain, once; a list of two words or
    /// more named by them, as the engine names it (`walk|run` →
    /// `walk_or_run`, `name|n|-` → `name_or_n`; `-|x0.5|s2` stays `x05s2`).
    fn name_once(names: &mut Vec<String>, written: &str) -> String {
        let unescaped = written.replace("\\|", "|");
        let words: Vec<&str> = unescaped.split('|').filter(|word| !word.is_empty() && word.chars().all(|letter| letter.is_ascii_lowercase())).collect();
        let written = if words.len() > 1 { words.join("_or_") } else { written.to_string() };
        let plain: String = crate::words::plain(&written).chars().filter(|letter| letter.is_ascii_alphanumeric() || *letter == '_').collect();
        let base = if plain.is_empty() || plain.starts_with(|letter: char| letter.is_ascii_digit()) { "v".to_string() } else { plain };
        let mut name = base.clone();
        let mut rank = 2;
        while names.contains(&name) {
            name = format!("{base}{rank}");
            rank += 1;
        }
        names.push(name.clone());
        name
    }

    /// The commands group should be, from the docs: one line of Rust a word.
    fn table_from_docs() -> Vec<String> {
        let documented = from_docs();
        let taken =
            |word: &str| !KEPT.contains(&word) && crate::verbs::BARE.all_groups_but_commands().any(|group| group.words.iter().any(|known| known.word == word));
        engine::commands()
            .iter()
            .filter(|own| !taken(own))
            .map(|&own| {
                let (shapes, said) = match documented.iter().find(|(known, ..)| *known == own) {
                    Some((_, shapes, said)) => (shapes.clone(), said.clone()),
                    None => (vec!["[text...]".to_string()], String::new()),
                };
                let writes = if said.is_empty() { own.to_string() } else { format!("{own}: {said}") };
                let forms: Vec<String> = shapes.iter().map(|form| format!("{form:?}")).collect();
                let until = if WALKS.contains(&own) { ".until()" } else { "" };
                format!("Word::new({own:?}, &[{}], {writes:?}){until}", forms.join(", "))
            })
            .collect()
    }

    /// The commands group is, one line of Rust a word.
    fn table_of_code() -> Vec<String> {
        crate::verbs::BARE
            .commands()
            .words
            .iter()
            .map(|word| {
                let forms: Vec<String> = word.forms.iter().map(|form| format!("{form:?}")).collect();
                let until = if matches!(word.time, crate::time::WordTime::Until) { ".until()" } else { "" };
                format!("Word::new({:?}, &[{}], {:?}){until}", word.word, forms.join(", "), word.writes)
            })
            .collect()
    }

    /// `COMMANDS.md`'s rows by theme write each command's forms and what it
    /// does as the engine declares them: the docs are read here only, never
    /// to read a line.
    #[test]
    fn the_docs_say_the_commands_as_the_code_declares_them() {
        let (docs, code) = (table_from_docs(), table_of_code());
        let differ: Vec<String> = docs
            .iter()
            .filter(|line| !code.contains(line))
            .map(|line| format!("the docs: {line}"))
            .chain(code.iter().filter(|line| !docs.contains(line)).map(|line| format!("the code: {line}")))
            .collect();
        assert!(differ.is_empty(), "engine/docs/COMMANDS.md and the commands group (film_core/src/verbs/commands.rs) differ:\n{}", differ.join("\n"));
    }

    /// Every command's rows by theme, those the groups have a word for
    /// too, say the forms and what it does the engine declares.
    #[test]
    fn the_docs_say_every_command_as_the_engine_declares_it() {
        let documented = from_docs();
        for own in engine::commands() {
            let form = engine::form_of(own).expect("a form");
            let forms: Vec<&str> = form.forms.to_vec();
            match documented.iter().find(|(known, ..)| known == own) {
                Some((_, shapes, said)) => {
                    assert_eq!((shapes.iter().map(String::as_str).collect::<Vec<_>>(), said.as_str()), (forms, form.does), "“{own}”")
                }
                None => assert_eq!((forms, form.does), (vec!["[text...]"], ""), "“{own}”: no row in COMMANDS.md"),
            }
        }
    }

    #[test]
    fn every_command_of_the_engine_has_a_word() {
        let group = crate::verbs::BARE.commands();
        assert!(group.words.len() > 100, "{}", group.words.len());
        for own in engine::commands() {
            assert!(crate::verbs::BARE.all_groups().any(|group| group.words.iter().any(|word| word.word == *own)), "“{own}” has no word");
        }
        for word in group.words {
            assert!(engine::commands().contains(&word.word), "“{}” is no command of the engine", word.word);
        }
        assert_eq!(form_of("<who> <x> <y> [<time>]"), "who:code x:coordinate y:coordinate [time:duration]");
        assert_eq!(form_of("<who> <strength> <text>"), "who:code strength:number text...");
        assert_eq!(form_of("<who> on|off"), "who:code on|off");
        assert_eq!(form_of("<a> <a> …"), "a:word a2:word rest...");
        // (A value left out is typed, a flag stays its very word.)
        assert_eq!(form_of("<who> [height] [frames]"), "who:code [height:number] [frames:duration]");
        assert_eq!(form_of("<who> <name> [loop] [frozen]"), "who:code name:word [loop] [frozen]");
        assert_eq!(form_of("<sound> [volume v] [pitch n]"), "sound:word [volume v:number] [pitch n:number]");
        assert_eq!(either("<key> <who\\|@ x y> [time]"), ["<key> <who> [time]", "<key> @ <x> <y> [time]"]);
        // (Each form reads.)
        for word in group.words {
            for form in word.forms {
                let _ = crate::grammar::read(form);
            }
        }
    }
}
