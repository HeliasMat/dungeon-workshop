//! What a scene asks of the game that plays it, typed as the scene is
//! made: whoever plays the story takes it as it is, with no line to read
//! again (see `Call::Game`).

/// What a scene asks of the game that plays it: from the scenes a game
/// makes of its places (`game talk kecleon`, `game menu`; a way near which
/// the game is told, `way … go port east`), or a line of the game's own
/// that changes its money or its bag (`Vocabulary::host_asks`).
#[derive(Clone, Debug, PartialEq)]
pub enum GameAsk {
    /// The player took a way out to that place (by that entrance).
    Go { place: String, entrance: Option<String> },
    /// The player walked into that zone.
    Zone(String),
    /// The player talked to that one: the scene holds.
    Talk(String),
    /// The player asked for the game's menu (M, X): the scene holds.
    Menu,
    /// So much money given (taken, below 0).
    Money(i32),
    /// An item of the game by its number, so many given or taken.
    Item { item: u16, count: u16, take: bool },
}

impl GameAsk {
    /// As the words after `game` say it (`talk kecleon`, `go port east`,
    /// `zone pier`, `menu`); none if they ask nothing the game knows.
    pub fn of_words(words: &[String]) -> Option<GameAsk> {
        let word = |at: usize| words.get(at).cloned();
        Some(match words.first()?.as_str() {
            "go" => GameAsk::Go { place: word(1)?, entrance: word(2).filter(|entrance| !entrance.is_empty()) },
            "zone" => GameAsk::Zone(word(1)?),
            "talk" => GameAsk::Talk(word(1)?),
            "menu" => GameAsk::Menu,
            _ => return None,
        })
    }

    /// Its words after `game`, as `of_words` reads them back (none for what
    /// a `game` line never says: the money, the items).
    pub fn words(&self) -> Option<Vec<String>> {
        let word = |word: &str| word.to_string();
        Some(match self {
            GameAsk::Go { place, entrance } => [word("go"), place.clone()].into_iter().chain(entrance.clone()).collect(),
            GameAsk::Zone(zone) => vec![word("zone"), zone.clone()],
            GameAsk::Talk(who) => vec![word("talk"), who.clone()],
            GameAsk::Menu => vec![word("menu")],
            GameAsk::Money(_) | GameAsk::Item { .. } => return None,
        })
    }

    /// Whether the scene holds until the game is done with it (see
    /// [`crate::Director::menu_done`]).
    pub fn holds(&self) -> bool {
        matches!(self, GameAsk::Talk(_) | GameAsk::Menu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_game_line_says_is_read_back_the_same() {
        for ask in [
            GameAsk::Go { place: "port".into(), entrance: Some("east".into()) },
            GameAsk::Go { place: "port".into(), entrance: None },
            GameAsk::Zone("pier".into()),
            GameAsk::Talk("kecleon".into()),
            GameAsk::Menu,
        ] {
            assert_eq!(GameAsk::of_words(&ask.words().expect("its words")), Some(ask));
        }
        assert_eq!(GameAsk::of_words(&["dive".to_string()]), None);
    }
}
