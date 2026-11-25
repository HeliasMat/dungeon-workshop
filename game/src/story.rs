//! Where the story of a game stands, the one truth a save keeps: its
//! chapter, its flags and counters, the place the team is in (and by which
//! entrance), the dungeons finished, the places and dungeons opened on the
//! map. With the rules' `Run` (the team, the bag, the money: `Playthrough`)
//! it is the whole game.
//!
//! A save is written where the team stands, never in the middle of a film:
//! changing a film breaks nothing. It goes through the rules' save
//! (`Run::save_text` and its `Place`): `event` is the chapter, the flags and
//! counters its numbers, the place and the rest its words.

use std::collections::{BTreeMap, BTreeSet};

use rules::run::saves::Place;

/// How a flag is kept among the save's numbers (`flag.met_kecleon:1`).
const FLAG: &str = "flag.";
/// How the last dungeon's end is kept.
const WON: &str = "won";
/// How the jobs done are counted (`if jobs_done >= 3`).
const JOBS_DONE: &str = "jobs_done";

/// The names a test reads that are the game's own, never a flag or a
/// counter: the chapter, how the last dungeon ended, the jobs done; the
/// team's exploration rank (1 the first, Normal), its money, its recruits;
/// the day (1 the first) and whether it is evening (1: back from a dungeon,
/// not slept since).
pub const OWN_NAMES: [&str; 8] = ["chapter", WON, JOBS_DONE, "rank", "money", "recruits", "day", "evening"];

/// What the team is now, as tests read it (not saved: the rules' game is).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TeamNumbers {
    pub rank: u32,
    pub money: i32,
    pub recruits: u32,
    pub day: u32,
    pub evening: bool,
}

/// Where the story stands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Story {
    pub chapter: u16,
    pub flags: BTreeSet<String>,
    pub counters: BTreeMap<String, i32>,
    /// The place the team is in (a decor's word), the entrance it came by,
    /// where it stood when saved.
    pub place: Option<String>,
    pub entrance: Option<String>,
    pub at: Option<(i32, i32)>,
    pub finished_dungeons: BTreeSet<String>,
    pub opened: BTreeSet<String>,
    /// How the last dungeon or fight ended (`if won`).
    pub won: bool,
    /// The jobs paid so far.
    pub jobs_done: u32,
    /// The team, as the rules' game has it now.
    pub team: TeamNumbers,
}

impl Story {
    /// What a name of a test reads: the chapter, how the last dungeon
    /// ended, the jobs done, the team's rank, money and recruits, a flag
    /// (1 up, 0 down), a counter; any other, 0.
    pub fn value(&self, name: &str) -> f64 {
        match name {
            "chapter" => f64::from(self.chapter),
            WON => f64::from(u8::from(self.won)),
            JOBS_DONE => f64::from(self.jobs_done),
            "rank" => f64::from(self.team.rank),
            "money" => f64::from(self.team.money),
            "recruits" => f64::from(self.team.recruits),
            "day" => f64::from(self.team.day),
            "evening" => f64::from(u8::from(self.team.evening)),
            name if self.flags.contains(name) => 1.0,
            name => self.counters.get(name).map_or(0.0, |count| f64::from(*count)),
        }
    }

    /// The numbers a film is given: the chapter, `won`, each flag up (and
    /// those of `names` down, at 0), each counter.
    pub fn numbers(&self, names: &BTreeSet<String>) -> Vec<(String, f64)> {
        let mut numbers = vec![("chapter".to_string(), f64::from(self.chapter)), (WON.to_string(), f64::from(u8::from(self.won)))];
        for name in names.iter().chain(&self.flags).chain(self.counters.keys()) {
            numbers.push((name.clone(), self.value(name)));
        }
        numbers
    }

    /// What a film gave back, by the names the game knows: a counter's
    /// number, a flag up or down.
    pub fn take_back(&mut self, numbers: &BTreeMap<String, f64>, names: &BTreeSet<String>, counters: &BTreeSet<String>) {
        for name in names {
            let Some(number) = numbers.get(name) else {
                continue;
            };
            if counters.contains(name) || self.counters.contains_key(name) {
                self.counters.insert(name.clone(), number.round() as i32);
            } else if *number != 0.0 {
                self.flags.insert(name.clone());
            } else {
                self.flags.remove(name);
            }
        }
    }

    /// As the rules' save has it.
    pub fn place_saved(&self) -> Place {
        let mut numbers: BTreeMap<String, f64> = self.counters.iter().map(|(name, count)| (name.clone(), f64::from(*count))).collect();
        numbers.extend(self.flags.iter().map(|flag| (format!("{FLAG}{flag}"), 1.0)));
        numbers.insert(WON.to_string(), f64::from(u8::from(self.won)));
        numbers.insert(JOBS_DONE.to_string(), f64::from(self.jobs_done));
        let mut texts = BTreeMap::new();
        let list = |set: &BTreeSet<String>| set.iter().cloned().collect::<Vec<_>>().join(",");
        if let Some(place) = &self.place {
            texts.insert("place".to_string(), place.clone());
        }
        if let Some(entrance) = &self.entrance {
            texts.insert("entrance".to_string(), entrance.clone());
        }
        if let Some((x, y)) = self.at {
            texts.insert("at".to_string(), format!("{x} {y}"));
        }
        texts.insert("finished".to_string(), list(&self.finished_dungeons));
        texts.insert("opened".to_string(), list(&self.opened));
        Place { event: Some(usize::from(self.chapter)), town: None, numbers, texts, checkpoint: None }
    }

    /// From the rules' save.
    pub fn of_place(place: &Place) -> Story {
        let text = |name: &str| place.texts.get(name).filter(|said| !said.trim().is_empty()).map(|said| said.trim().to_string());
        let list = |name: &str| text(name).map_or_else(BTreeSet::new, |said| said.split(',').map(str::to_string).filter(|word| !word.is_empty()).collect());
        let mut story = Story {
            chapter: place.event.map_or(1, |event| event as u16),
            place: text("place"),
            entrance: text("entrance"),
            at: text("at").and_then(|said| {
                let (x, y) = said.split_once(' ')?;
                Some((x.parse().ok()?, y.parse().ok()?))
            }),
            finished_dungeons: list("finished"),
            opened: list("opened"),
            ..Story::default()
        };
        for (name, number) in &place.numbers {
            match name.strip_prefix(FLAG) {
                Some(flag) if *number != 0.0 => {
                    story.flags.insert(flag.to_string());
                }
                Some(_) => {}
                None if name == WON => story.won = *number != 0.0,
                None if name == JOBS_DONE => story.jobs_done = number.max(0.0).round() as u32,
                None => {
                    story.counters.insert(name.clone(), number.round() as i32);
                }
            }
        }
        story
    }

    /// As a save writes it without the rules' game (no data): its chapter,
    /// its numbers, its words, as `Run::save_text` writes them.
    pub fn written_alone(&self) -> String {
        let place = self.place_saved();
        let event = place.event.map_or_else(|| "-".to_string(), |event| event.to_string());
        let numbers: Vec<String> = place.numbers.iter().map(|(name, number)| format!("{name}:{number}")).collect();
        let mut text = format!("event {event}\ntown -\nvars={}", numbers.join(","));
        for (name, words) in &place.texts {
            text.push_str(&format!("\ntext {name}={}", words.replace('\n', " ")));
        }
        text
    }

    /// From a save's text (the rules' or `written_alone`'s).
    pub fn read(text: &str) -> Story {
        let field = |name: &str| rules::run::saves::field(text, name);
        let numbers = field("vars=")
            .unwrap_or_default()
            .split(',')
            .filter_map(|pair| pair.split_once(':'))
            .filter_map(|(name, value)| Some((name.to_string(), value.parse().ok()?)))
            .collect();
        let texts =
            text.lines().filter_map(|line| line.strip_prefix("text ")?.split_once('=')).map(|(name, words)| (name.to_string(), words.to_string())).collect();
        let event = field("event ").and_then(|event| event.trim().parse().ok());
        Story::of_place(&Place { event, town: None, numbers, texts, checkpoint: None })
    }
}

/// Where a game is saved: `<folder>/<its word>.txt` (`saves/`, in the folder it is played from).
pub fn save_file(folder: &std::path::Path, game: &str) -> std::path::PathBuf {
    folder.join(format!("{game}.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_story_saved_is_read_back_the_same() {
        let mut story =
            Story { chapter: 2, place: Some("town".to_string()), entrance: Some("south".to_string()), at: Some((120, 88)), won: true, ..Story::default() };
        story.flags.insert("met_kecleon".to_string());
        story.counters.insert("apples".to_string(), 3);
        story.finished_dungeons.insert("test_forest".to_string());
        story.opened.insert("beach".to_string());
        story.jobs_done = 4;
        assert_eq!(Story::read(&story.written_alone()), story);
        assert_eq!(Story::of_place(&story.place_saved()), story);
        assert_eq!((story.value("chapter"), story.value("met_kecleon"), story.value("apples"), story.value("nothing")), (2.0, 1.0, 3.0, 0.0));
        story.team = TeamNumbers { rank: 2, money: 350, recruits: 1, day: 3, evening: true };
        assert_eq!([story.value("jobs_done"), story.value("rank"), story.value("money"), story.value("recruits")], [4.0, 2.0, 350.0, 1.0]);
        assert_eq!([story.value("day"), story.value("evening")], [3.0, 1.0]);
    }
}
