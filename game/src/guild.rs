//! The guild's extras, as the original has them: sentry duty (`sentry`:
//! whose footprints these are, among four names, ten visitors, the faster
//! the more points; what the guild gives by how it went, once a day),
//! Marowak's Dojo (`dojo`: a training maze by type, gone through as a short
//! dungeon, the bag left at its door, nothing lost fainting), Pelipper's
//! SOS (`rescue`: the SOS of a team that fainted sent, a friendly team's
//! A-OK Mail read after an exploration, what was lost given back), and the
//! personality test that makes the hero and lets its partner be chosen
//! (`team: quiz`). What they do is the rules' (`rules::sentry`,
//! `rules::dojo`, `rules::quiz`, `rules::run::Run`); here, a key at a time
//! and what is shown, the words naming no resident. The footprints are not
//! the game's pictures (they are not extracted): the visitor's portrait, in
//! shadow, stands for them.

use rules::dojo;
use rules::quiz::Quiz;
use rules::run::Run;
use rules::run::town::Draw;
use rules::sentry::Sentry;
use rules::services::Input;
use stage_view::ui::{ListCursor, Picked};

use crate::assembly::Shown;

/// A portrait shown beside the box: whose, and whether in shadow.
pub type Face = (u16, bool);

/// A second of the game, in frames: a screen's frame.
const FRAME: f32 = 1.0 / 60.0;

fn yes_no() -> Vec<String> {
    vec!["Yes".to_string(), "No".to_string()]
}

/// The items a text of the rules names by number (`{item:12}`), by their names.
fn named(text: &str, name: impl Fn(u16) -> String) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find("{item:") {
        out.push_str(&rest[..at]);
        let after = &rest[at + "{item:".len()..];
        match after.find('}').and_then(|end| Some((after[..end].parse::<u16>().ok()?, end))) {
            Some((item, end)) => {
                out.push_str(&name(item));
                rest = &after[end + 1..];
            }
            None => {
                out.push_str("{item:");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Sentry duty open: asked, on duty, or over (what was said, till a key).
pub struct SentryDuty {
    step: Duty,
    draw: Draw,
}

enum Duty {
    Asked(ListCursor),
    Standing(Sentry),
    Over(String),
}

impl SentryDuty {
    /// Once a day: stood already, it is said so.
    pub fn new(run: &Run, draw: Draw) -> Self {
        let step = match run.sentry_free() {
            true => Duty::Asked(ListCursor::default()),
            false => Duty::Over("You've stood sentry duty today! Come back tomorrow.".to_string()),
        };
        SentryDuty { step, draw }
    }

    /// Whose footprints are on show (in shadow until told).
    pub fn face(&self) -> Option<Face> {
        match &self.step {
            Duty::Standing(sentry) => sentry.shown(),
            _ => None,
        }
    }

    /// A frame: the key pressed (time passes on duty); what is shown;
    /// whether it is left.
    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        let name = |monster: u16| run.book.species_name(monster);
        match &mut self.step {
            Duty::Asked(cursor) => match cursor.press(pressed, 2) {
                Picked::Chosen(0) => {
                    let draw = &mut self.draw;
                    self.step = Duty::Standing(Sentry::new(&run.book.tables.sentry, |below| draw.below(below)));
                    self.play(None, run)
                }
                Picked::Chosen(_) | Picked::Left => (true, (String::new(), Vec::new(), 0)),
                _ => {
                    let asked = format!(
                        "Sentry duty! {} visitors come to the guild. Look at the footprints and say whose they are: the faster, the more points. Ready?",
                        rules::sentry::ROUNDS
                    );
                    (false, (asked, yes_no(), cursor.at))
                }
            },
            Duty::Standing(sentry) => {
                sentry.tick(FRAME);
                if let Some(pressed) = pressed {
                    sentry.press(pressed);
                }
                if !sentry.over() {
                    let view = sentry.view(name);
                    return (false, (view.message, view.options, view.cursor));
                }
                let (points, rating) = (sentry.points, sentry.rating());
                let reward = run.stand_sentry(points, rating, &mut self.draw);
                let judged = match rating {
                    3 => "Perfect! Not one visitor missed!",
                    2 => "Great sentry duty!",
                    1 => "Not bad... You can do better.",
                    _ => "Too many visitors missed! No reward today.",
                };
                let reward = reward.map(|reward| format!("\n{}", named(&reward, |item| run.book.item_name(item)))).unwrap_or_default();
                self.step = Duty::Over(format!("Duty over! {points} points.\n{judged}{reward}"));
                self.play(None, run)
            }
            Duty::Over(said) => (matches!(pressed, Some(Input::Confirm | Input::Cancel)), (said.clone(), Vec::new(), 0)),
        }
    }
}

/// Marowak's Dojo open: its mazes, the one chosen to go into once left.
#[derive(Default)]
pub struct Dojo {
    cursor: ListCursor,
    pub chosen: Option<u16>,
}

impl Dojo {
    /// The mazes open: one by type, the Final Maze once all of them are
    /// gone through (those the rules' tables have).
    fn mazes(run: &Run) -> Vec<u16> {
        dojo::open(run.mazes_done()).into_iter().filter(|&maze| (maze as usize) < run.book.tables.dungeons.len()).collect()
    }

    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        let mazes = Self::mazes(run);
        if mazes.is_empty() {
            let left = matches!(pressed, Some(Input::Confirm | Input::Cancel));
            return (left, ("The Dojo is closed: its mazes are not in the game's data.".to_string(), Vec::new(), 0));
        }
        let left = match self.cursor.press(pressed, mazes.len() + 1) {
            Picked::Chosen(row) if row < mazes.len() => {
                self.chosen = Some(mazes[row]);
                true
            }
            Picked::Chosen(_) | Picked::Left => true,
            _ => false,
        };
        let done = &run.team().progress.mazes;
        let row = |&maze: &u16| match done.contains(&maze) {
            true => format!("{}  cleared", dojo::name(maze)),
            false => dojo::name(maze),
        };
        let options = mazes.iter().map(row).chain(["Done".to_string()]).collect();
        let message = "Welcome to the Dojo! Which maze will you train in? Your bag waits at the door, and fainting costs nothing.".to_string();
        (left, (message, options, self.cursor.at))
    }
}

/// Pelipper's SOS open: the last SOS sent, its A-OK Mail read.
#[derive(Default)]
pub struct Rescue {
    cursor: ListCursor,
    said: Option<String>,
}

impl Rescue {
    pub fn play(&mut self, pressed: Option<Input>, run: &mut Run) -> (bool, Shown) {
        if let Some(said) = self.said.clone() {
            let left = matches!(pressed, Some(Input::Confirm | Input::Cancel));
            return (left, (said, Vec::new(), 0));
        }
        let Some(sos) = run.sos().cloned() else {
            let left = matches!(pressed, Some(Input::Confirm | Input::Cancel));
            let message = "No SOS to send. If your team faints in a dungeon, a friendly team may be asked for help here.";
            return (left, (message.to_string(), Vec::new(), 0));
        };
        let place = format!("{} (floor {})", run.book.dungeon_name(sos.dungeon), sos.floor);
        match (sos.sent, sos.trips) {
            (false, _) => {
                match self.cursor.press(pressed, 2) {
                    Picked::Chosen(0) => self.said = Some(run.send_sos()),
                    Picked::Chosen(_) | Picked::Left => return (true, (String::new(), Vec::new(), 0)),
                    _ => {}
                }
                if let Some(said) = &self.said {
                    return (false, (said.clone(), Vec::new(), 0));
                }
                let lost = format!("{} items and {} Poké", sos.items.len(), sos.money);
                let message = format!("Your team fainted at {place} and lost {lost}. Send an SOS? A friendly team may come to the rescue.");
                (false, (message, yes_no(), self.cursor.at))
            }
            (true, 0) => {
                match self.cursor.press(pressed, 2) {
                    Picked::Chosen(0) => {
                        self.said = Some({
                            let read = run.read_aok();
                            named(&read, |item| run.book.item_name(item))
                        })
                    }
                    Picked::Chosen(_) | Picked::Left => return (true, (String::new(), Vec::new(), 0)),
                    _ => {}
                }
                if let Some(said) = &self.said {
                    return (false, (said.clone(), Vec::new(), 0));
                }
                let message = format!("A-OK Mail has come! A friendly team answered your SOS from {place}. Read it?");
                (false, (message, yes_no(), self.cursor.at))
            }
            (true, _) => {
                let left = matches!(pressed, Some(Input::Confirm | Input::Cancel));
                let message = format!("Your SOS from {place} is out. A friendly team will answer it after your next exploration.");
                (left, (message, Vec::new(), 0))
            }
        }
    }
}

/// The personality test open: the hero and its partner, once chosen.
pub struct QuizScreen {
    quiz: Quiz,
    draw: Draw,
    pub chosen: Option<(u16, u16)>,
}

impl QuizScreen {
    /// The test, if the game's data has it.
    pub fn new(run: &Run, mut draw: Draw) -> Option<Self> {
        let data = run.book.tables.quiz.as_ref()?;
        let quiz = Quiz::new(data, |below| draw.below(below));
        Some(QuizScreen { quiz, draw, chosen: None })
    }

    /// The partner on show while one is chosen.
    pub fn face(&self) -> Option<Face> {
        self.quiz.partner_shown().map(|partner| (partner, false))
    }

    pub fn play(&mut self, pressed: Option<Input>, run: &Run) -> (bool, Shown) {
        let tables = &run.book.tables;
        let Some(data) = tables.quiz.as_ref() else {
            return (true, (String::new(), Vec::new(), 0));
        };
        if let Some(pressed) = pressed {
            let types = |monster: u16| tables.species.get(monster as usize).map_or((0, 0), |species| species.types);
            let draw = &mut self.draw;
            self.chosen = self.quiz.press(pressed, data, &types, |below| draw.below(below));
        }
        let view = self.quiz.view(data, |monster| run.book.species_name(monster));
        (self.chosen.is_some(), (view.message, view.options, view.cursor))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn items_named_by_number_are_told_by_their_names() {
        let name = |item: u16| if item == 70 { "Oran Berry".to_string() } else { format!("#{item}") };
        assert_eq!(super::named("Got {item:70} + {item:9}!", name), "Got Oran Berry + #9!");
        assert_eq!(super::named("{item:x} stays", name), "{item:x} stays");
    }
}
