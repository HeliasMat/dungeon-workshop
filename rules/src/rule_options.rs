//! Rule options, each off unless asked for: ideas from SkyTemple's patch
//! catalogue (`skytemple_files/patch/handler`), done here in the engine's
//! own way. A dungeon of one's own turns them on with a `rules:` line, the
//! settings file with `rules=`; both are merged as a dungeon is entered.
//!
//! ```text
//! rules: fairy, team_control, push_allies, exp_share, lasting_weather, disarmed_houses, enemies_evolve
//! ```

/// The options of a dungeon's rules (the words of the `rules:` line).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuleOptions {
    /// `fairy`: the Fairy type, an 18th, may be given to Pokémon and moves.
    pub fairy: bool,
    /// `team_control`: Tab gives the player another member of the team to
    /// move; the others follow on their own.
    pub team_control: bool,
    /// `push_allies`: walking into an ally pushes it one tile on, where it
    /// can go; else the two trade places as usual.
    pub push_allies: bool,
    /// `exp_share`: the members waiting at the base get the Exp. too.
    pub exp_share: bool,
    /// `lasting_weather`: the weather a move or an orb brings stays until
    /// another comes.
    pub lasting_weather: bool,
    /// `disarmed_houses`: a floor made of one single room is no Monster
    /// House.
    pub disarmed_houses: bool,
    /// `enemies_evolve`: a foe that defeats someone evolves, as the game's
    /// flag for it has it (`Restrictions::enemies_evolve`, which none of
    /// its dungeons sets).
    pub enemies_evolve: bool,
}

/// Each option: its word, and what the Workshop says of it.
pub const WORDS: [(&str, &str); 7] = [
    ("fairy", "Fairy type: an 18th type, for its Pokémon and its moves"),
    ("team_control", "control of the whole team: Tab changes the Pokémon you lead"),
    ("push_allies", "push your allies: walking into an ally pushes it one tile on"),
    ("exp_share", "Exp. share: those left at the base gain some too"),
    ("lasting_weather", "the weather does not stop: a move's weather stays"),
    ("disarmed_houses", "no Monster House on a floor of one single room"),
    ("enemies_evolve", "foes evolve: one that defeats someone evolves"),
];

impl RuleOptions {
    /// The option of that word, to set or read (with or without its `_`).
    pub fn flag(&mut self, word: &str) -> Option<&mut bool> {
        Some(match word {
            "fairy" => &mut self.fairy,
            "teamcontrol" | "team_control" => &mut self.team_control,
            "pushallies" | "push_allies" => &mut self.push_allies,
            "expshare" | "exp_share" => &mut self.exp_share,
            "lastingweather" | "lasting_weather" => &mut self.lasting_weather,
            "disarmedhouses" | "disarmed_houses" => &mut self.disarmed_houses,
            "enemiesevolve" | "enemies_evolve" => &mut self.enemies_evolve,
            _ => return None,
        })
    }

    /// The options a list of words turns on (`fairy, push_allies`), and the
    /// words not known.
    pub fn read(said: &str) -> (Self, Vec<String>) {
        let mut options = Self::default();
        let mut unknown = Vec::new();
        for word in said.split([',', ' ']).map(str::trim).filter(|word| !word.is_empty()) {
            match options.flag(&crate::own::plain(word)) {
                Some(flag) => *flag = true,
                None => unknown.push(word.to_string()),
            }
        }
        (options, unknown)
    }

    /// The words of the options on, as `read` takes them.
    pub fn words(&self) -> Vec<&'static str> {
        let mut copy = *self;
        WORDS.iter().map(|(word, _)| *word).filter(|word| copy.flag(word).is_some_and(|on| *on)).collect()
    }

    /// Those of either.
    pub fn or(self, other: Self) -> Self {
        Self {
            fairy: self.fairy || other.fairy,
            team_control: self.team_control || other.team_control,
            push_allies: self.push_allies || other.push_allies,
            exp_share: self.exp_share || other.exp_share,
            lasting_weather: self.lasting_weather || other.lasting_weather,
            disarmed_houses: self.disarmed_houses || other.disarmed_houses,
            enemies_evolve: self.enemies_evolve || other.enemies_evolve,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_are_read_from_their_words_and_written_back() {
        let (options, unknown) = RuleOptions::read("Fairy, Push_Allies lasting_weather, nothing");
        assert!(options.fairy && options.push_allies && options.lasting_weather && !options.exp_share);
        assert_eq!(unknown, ["nothing"]);
        assert_eq!(options.words(), ["fairy", "push_allies", "lasting_weather"]);
        assert_eq!(RuleOptions::read(&options.words().join(", ")).0, options);
        assert_eq!(RuleOptions::default().words(), Vec::<&str>::new());
    }
}
