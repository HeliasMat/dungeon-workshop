//! What the player does.

use super::*;

impl Director {
    /// The player pressed Enter while walking someone about (see
    /// `PlayBeat::Control`): counted (`presses`), and who is nearest and close
    /// enough to talk to, or to pick up (`talking`, `target`).
    pub fn press(&mut self, stage: &Stage) {
        *self.vars.entry(PRESSES.to_string()).or_insert(0.0) += 1.0;
        self.vars.insert(PRESSED_AT.to_string(), self.clocks.scene_clock as f64);
        let Some((me, at, ahead)) =
            stage.play.controlled.and_then(|who| stage.cast.get(&who).map(|actor| (who, actor.position, crate::stage::step_of(actor.direction))))
        else {
            return;
        };
        // (In front of it, the way it faces: not who stands behind.)
        let in_front = |there: Vec2| (there - at).dot(ahead) >= 0.0 || there.distance(at) < 8.0;
        let near = stage
            .cast
            .iter()
            .filter(|(who, actor)| **who != me && in_front(actor.position))
            .map(|(who, actor)| (*who, actor.position.distance(at)))
            .filter(|(_, distance)| *distance <= REACH);
        let near = near.min_by(|a, b| a.1.total_cmp(&b.1));
        self.vars.insert(TALKS.to_string(), f64::from(u8::from(near.is_some())));
        if let Some((who, _)) = near {
            self.vars.insert(TARGET.to_string(), who.number() as f64);
        }
    }

    /// The menu key, while roaming: its passage plays (see `FlowBeat::OnMenu`).
    pub fn open_menu(&mut self) {
        self.roam.menu_asked = self.roam.menu.is_some();
    }

    /// The numbers and words of the story as they are now, to be saved.
    pub fn numbers_and_words(&self) -> (BTreeMap<String, f64>, HashMap<String, String>) {
        (self.vars.clone(), self.texts.clone())
    }

    /// An arrow pressed while keys are counted with no one walked about
    /// (see `PlayBeat::Listen`): 1 up, 2 down, 3 left, 4 right.
    pub fn arrow(&mut self, arrow: i32) {
        *self.vars.entry(ARROWS.to_string()).or_insert(0.0) += 1.0;
        self.vars.insert(ARROW.to_string(), arrow.into());
        self.vars.insert(ARROW_AT.to_string(), self.clocks.scene_clock as f64);
    }

    /// The player has read the line on screen.
    pub fn read(&mut self) {
        (self.reading, self.timed) = (false, None);
    }

    /// The question asked is answered (`None`: it had no answers to
    /// choose from): on along that answer's way.
    pub fn answer(&mut self, chosen: Option<usize>) {
        let Some(at) = self.at.filter(|_| self.hold == Some(Hold::Question)) else {
            return;
        };
        // (Among those offered: its place among all of them.)
        let chosen = chosen.map(|chosen| self.asking.offered.get(chosen).copied().unwrap_or(chosen));
        let way = chosen.map_or(Mark { beat: at.beat + 1, ..at }, |chosen| self.branch(at, chosen));
        self.opened &= way.scene == at.scene;
        (self.at, self.hold) = (Some(way), None);
        if let Some(chosen) = chosen {
            self.vars.insert(ANSWER.to_string(), chosen as f64);
        }
    }

    /// The answers of a question offered: those without a test, and those
    /// whose test holds; with their places among all of them.
    pub(super) fn offered(&self, answers: &[String]) -> Vec<(usize, String)> {
        let offered = answers.iter().enumerate().filter_map(|(place, answer)| {
            let (test, words) = answer_test(answer);
            test.is_none_or(|test| self.holds(&test)).then(|| (place, words.to_string()))
        });
        offered.collect()
    }

    /// Where the one roaming stood at the last save (`FlowBeat::SaveGame`),
    /// to be written with it (and given back by `Author::restore_at`).
    pub fn saved_at(&self) -> Option<Vec2> {
        self.roamed.saved_at
    }

    /// The way the scene a way out led to was come into by (`FlowBeat::Door`'s
    /// `arrive`), if one was.
    pub fn arrival(&self) -> Option<f32> {
        self.roamed.arrival
    }

    /// The menu open is done with; `said`: it leaves a line on screen to
    /// read.
    pub fn menu_done(&mut self, said: bool) {
        if let Some(at) = self.at.filter(|_| self.hold == Some(Hold::Host)) {
            (self.at, self.hold, self.reading) = (Some(Mark { beat: at.beat + 1, ..at }), None, said);
        }
    }

    /// A beat the game plays at once, the story held: as if the scene had
    /// it there. What the game is to do comes back, as from `tick`.
    pub fn play(&mut self, beat: Beat, stage: &mut Stage, focus: Vec2) -> Vec<Call> {
        let mut calls = Vec::new();
        self.happen(beat, stage, focus, &mut calls);
        calls
    }

    /// The passage of the scene at a label played, the story held: at its
    /// `return` the story comes back to the line that holds, which is played
    /// again. Whether there is such a label.
    pub fn passage(&mut self, label: &str) -> bool {
        let Some(at) = self.at.filter(|_| self.hold == Some(Hold::Host)) else {
            return false;
        };
        let Some(past) = self.past_label(at, label) else {
            return false;
        };
        self.schedule.returns.push(at);
        (self.at, self.hold) = (Some(Mark { beat: past, ..at }), None);
        true
    }

    /// The scene at hand opens again, on a cleared stage: back from
    /// elsewhere (a place walked in, a dungeon).
    pub fn resume(&mut self) {
        (self.opened, self.wipe) = (false, true);
    }
}
