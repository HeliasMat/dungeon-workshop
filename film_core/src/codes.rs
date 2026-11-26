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
/// A word of the old vocabulary (`fadein`, `turn`): the form that
/// says it now (`film::vocabulary::REPLACED`).
pub const OLD_WORD: &str = "E-OLD-WORD";

/// A word of an optional family (`bite`, `stairs`, `job`) in a film that
/// does not take its library in (`uses fishing`).
pub const USES: &str = "R-USES";

/// In a film `for ds`: a word the DS has nothing for (`film::ds`), with
/// how to write it another way where there is one.
pub const DS_WORD: &str = "E-DS-WORD";
/// In a film `for ds`: a word that exports with less than the film shows,
/// what it loses said (once a word).
pub const DS_LOSS: &str = "R-DS-LOSS";
/// In a film `for ds`: more Pokémon at once in a scene than Sky shows (21),
/// who would be left out said.
pub const DS_ACTORS: &str = "E-DS-ACTORS";
/// In a film `for ds`: a place that is no map of Explorers of Sky's nor a decor of
/// the Workshop's (a decor exports as a map of its own, `R-DS-LOSS`).
pub const DS_PLACE: &str = "E-DS-PLACE";
/// In a film `for ds` (played as a new game starts): the hero or the
/// partner, not chosen yet.
pub const DS_HERO: &str = "E-DS-HERO";
/// In a film `for ds`: a zoom, or a frame wider than the screen, where the
/// view would step back: Sky has no zoom.
pub const DS_ZOOM: &str = "R-DS-ZOOM";
/// In a film `for ds`: a Pokémon or a form the ROM has no sprite for, and
/// no sheets in the data to put in it (one SpriteCollab adds): drawn as its
/// base species, or as a stand-in.
pub const DS_SPECIES: &str = "R-DS-SPECIES";
/// In a film `for ds`: what the export leaves out, the DS drawing nothing
/// of it yet: an item, an engine's command with no DS operation.
pub const DS_LEFT: &str = "R-DS-LEFT";

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
    (OLD_WORD, "a word of the old vocabulary: the form that says it now"),
    (USES, "a word of an optional family in a film that does not take its library in (uses dungeon, guild, fishing)"),
    (DS_WORD, "in a film for ds: a word the DS has nothing for, with how to write it another way"),
    (DS_LOSS, "in a film for ds: a word that exports with less than the film shows, what it loses"),
    (DS_ACTORS, "in a film for ds: more than 21 Pokémon at once in a scene, who would be left out"),
    (DS_PLACE, "in a film for ds: a place that is neither a map of Explorers of Sky's nor a decor of yours"),
    (DS_HERO, "in a film for ds: the hero or the partner, before the player chose them"),
    (DS_ZOOM, "in a film for ds: a zoom, or a frame wider than the screen: Sky has no zoom"),
    (DS_SPECIES, "in a film for ds: a Pokémon or form SpriteCollab adds with no sheets in the data to put in the ROM: drawn as its base species or a stand-in"),
    (DS_LEFT, "in a film for ds: what the export leaves out: an engine's command with no DS operation (not on an item, which is shown)"),
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
