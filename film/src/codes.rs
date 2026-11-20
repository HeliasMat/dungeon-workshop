//! The films' own codes of what is wrong (those of the engine's lines are
//! `engine::codes`, those of the language `language::diag::codes`): the
//! same rule, `E-` an error, `R-` a remark, never renamed nor given another
//! meaning.

/// What reading a film found wrong, with no code of its own yet.
pub const READ_WRONG: &str = "E-READ";
/// What reading a film found to look at, with no code of its own yet.
pub const READ_REMARK: &str = "R-READ";

/// A line written the word first though its word is said of someone
/// (`walk pikachu to b`): who acts comes first (`pikachu walk to b`). (It
/// was a remark, `R-ORDER`, while the films were written again.)
pub const OLD_ORDER: &str = "E-ORDER";
/// A word of the old vocabulary (`fadein`, `turn`): the plan's form that
/// says it now (`film::plan::REPLACED`).
pub const OLD_WORD: &str = "E-OLD-WORD";

/// A word of an optional family (`bite`, `stairs`, `job`) in a film that
/// does not take its library in (`uses fishing`).
pub const USES: &str = "R-USES";

/// What the words of a film say, with no code of their own yet, as the
/// language's service tells it (`crate::service::diagnostics`).
pub const WORDS: &str = "film";

/// The code of a problem read: its own, or what it is.
pub fn of(problem: &engine::Diagnostic) -> &'static str {
    problem.code_or(if problem.is_remark() { READ_REMARK } else { READ_WRONG })
}

/// Every code, with what it means.
pub const ALL: &[(&str, &str)] = &[
    (READ_WRONG, "what reading finds wrong, with no code of its own yet"),
    (READ_REMARK, "what reading notes, with no code of its own yet"),
    (OLD_ORDER, "a line written the word first, said of someone: who acts comes first"),
    (OLD_WORD, "a word of the old vocabulary: the form of the plan that says it now"),
    (USES, "a word of an optional family in a film that does not take its library in (uses dungeon, guild, fishing)"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_problem_read_has_a_code() {
        assert_eq!(of(&engine::Diagnostic::at(1, "no").as_remark()), READ_REMARK);
        assert_eq!(of(&engine::Diagnostic::at(1, "no")), READ_WRONG);
        assert_eq!(of(&engine::Diagnostic::at(1, "no").coded(engine::codes::NODE_UNKNOWN)), "E-NODE-UNKNOWN");
        let every: Vec<&str> = engine::codes::every().iter().map(|(code, _)| *code).chain(ALL.iter().map(|(code, _)| *code)).collect();
        for (at, code) in every.iter().enumerate() {
            assert!(!every[..at].contains(code), "{code} twice");
        }
    }
}
