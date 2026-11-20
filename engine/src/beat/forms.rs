//! Each command's form, declared once, beside the reading of its beat: each
//! kind of beat's file has its `FORMS` next to its `declare!`. A form is
//! written as the film's language reads one (`who:code [height:number]
//! [frames:duration]`): its parts in order, each a word as it is (`off`,
//! `@`) or a name and its kind (`code`, `number`, `duration`, `coordinate`,
//! `word`), a part between brackets that may be left out, `name...` all the
//! words left. With what the command does, in words.
//!
//! The language gives each command no word of its own these forms
//! (`film/src/verbs/commands.rs`); `engine/docs/COMMANDS.md` has their table
//! written from here (`GOLDEN=write cargo test -p engine forms`), and its rows
//! by theme are checked against it.

use super::*;

/// A command's forms and what it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Form {
    pub own: &'static str,
    pub forms: &'static [&'static str],
    /// What it does, in words (none: nothing told yet).
    pub does: &'static str,
}

impl Form {
    pub const fn new(own: &'static str, forms: &'static [&'static str], does: &'static str) -> Form {
        Form { own, forms, does }
    }
}

/// Every command's form, kind by kind.
pub fn forms() -> impl Iterator<Item = &'static Form> {
    [
        body::FORMS,
        motion::FORMS,
        view::FORMS,
        screen::FORMS,
        sound::FORMS,
        speech::FORMS,
        flow::FORMS,
        data::FORMS,
        play::FORMS,
        place::FORMS,
        node::FORMS,
        parse::FORMS,
    ]
    .into_iter()
    .flatten()
}

/// A command's form, by its own name.
pub fn form_of(own: &str) -> Option<&'static Form> {
    forms().find(|form| form.own == own)
}

/// The table of every command and its forms, as `COMMANDS.md` has it
/// between its marks.
#[cfg(test)]
pub fn table() -> String {
    let mut all: Vec<&Form> = forms().collect();
    all.sort_by_key(|form| form.own);
    let mut out = String::from("| Command | Forms |\n|---|---|\n");
    for form in all {
        let shown: Vec<String> = form.forms.iter().map(|written| format!("`{}`", format!("{} {written}", form.own).trim_end().replace('|', "\\|"))).collect();
        out.push_str(&format!("| `{}` | {} |\n", form.own, shown.join(" · ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: &str = "<!-- forms: written by engine/src/beat/forms.rs -->";
    const END: &str = "<!-- end of forms -->";

    /// Each command has its form, once, in the file of the kind that reads it.
    #[test]
    fn every_command_has_one_form_where_it_is_read() {
        for own in commands() {
            assert_eq!(forms().filter(|form| form.own == *own).count(), 1, "“{own}”");
        }
        assert_eq!(forms().count(), commands().len());
        for (kind, words) in [(body::FORMS, body::WORDS), (flow::FORMS, flow::WORDS), (play::FORMS, play::WORDS), (view::FORMS, view::WORDS)] {
            for form in kind {
                assert!(words.iter().any(|command| command.own == form.own || command.also.contains(&form.own)), "“{}”", form.own);
            }
        }
    }

    /// `COMMANDS.md`'s table of forms is the code's (`GOLDEN=write` writes it anew).
    #[test]
    fn the_forms_are_in_commands_md_as_declared() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/COMMANDS.md");
        let text = std::fs::read_to_string(&path).expect("COMMANDS.md").replace("\r\n", "\n");
        let made = format!("{START}\n{}{END}", table());
        let (Some(from), Some(to)) = (text.find(START), text.find(END)) else {
            panic!("COMMANDS.md: no table of forms ({START} ... {END})");
        };
        let kept = &text[from..to + END.len()];
        if std::env::var("GOLDEN").is_ok_and(|asked| asked == "write") {
            std::fs::write(&path, text.replace(kept, &made)).expect("written");
            return;
        }
        assert_eq!(kept, made, "the table of forms of COMMANDS.md is no longer the code's: GOLDEN=write cargo test -p engine forms");
    }
}
