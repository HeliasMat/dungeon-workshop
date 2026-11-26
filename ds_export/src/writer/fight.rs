//! A fight, staged as Sky stages them: the blows its rules played, as attacks.

use std::collections::HashMap;

use engine::{Beat, BodyBeat, Course, Play, Who};
use scenes::fight::Sight;

use super::Writer;
use crate::notes::{Fate, Kind};
use crate::sky::Game;

/// What the fights' rules played and how it ended, by the fight's line.
pub(crate) type Played = HashMap<Vec<String>, ([Vec<Who>; 2], Vec<Sight>)>;

/// Each fight of the film played by its rules, as the engine plays it (a
/// directed one as written): what the export stages as Sky's attacks.
pub(crate) fn fights_played(written: &::film::Written, art: &preview::art::Art, game: &Game) -> Played {
    let rules = |species: &[u16]| Some(scenes::fight::Rules::of_kin(&art.tables, species, &|id| art.kin(id)));
    let other = |who: Who| match who {
        Who::Character(number) => game.actors.get(usize::from(number)).map_or(0, |(_, species)| *species),
        Who::Monster(species) | Who::Another { species, .. } => species,
        _ => 0,
    };
    let mut played = HashMap::new();
    for beat in written.events.iter().flat_map(|event| &event.scenes).flat_map(|scene| &scene.beats) {
        let (Beat::Host(fields) | Beat::Course(Course::Host(fields))) = beat else { continue };
        if fields.first().is_some_and(|first| first == "fight")
            && fields.get(1).is_some_and(|second| second != "end")
            && let Some(fight) = scenes::fight::host::played(fields, &rules, &other)
        {
            played.insert(fields.clone(), fight);
        }
    }
    played
}

/// A fight's own label (`fight1_said0`, `fight1_past`): its passages.
pub(super) fn fight_label(label: &str) -> bool {
    label.strip_prefix("fight").is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()) && rest.contains('_'))
}

impl Writer<'_> {
    /// A fight, staged as Sky stages them: each blow the rules played as an
    /// attack, the one struck stepping back in pain, a fall its faint, the
    /// lines said in its moments where they come; no bars, no log.
    pub(super) fn fight(&mut self, number: i32, fields: &[String]) {
        if fields.get(1).is_some_and(|second| second == "end") {
            return;
        }
        let Some((sides, sights)) = self.fights.get(fields).cloned() else {
            // (Its blows can't be read: a short exchange between the first of each side.)
            let side: Vec<Who> =
                fields.windows(2).filter(|pair| pair[0] == "side").filter_map(|pair| pair[1].strip_prefix('m')?.parse().ok().map(Who::Monster)).collect();
            if let [first, second, ..] = side[..] {
                for (who, target) in [(first, second), (second, first)] {
                    self.blow(number, who, 0, Some(target), false, false);
                }
            }
            self.note(number, Kind::Fight, Fate::Changed, "the fight's blows, not played by its rules here", "a short exchange between the first of each side");
            return;
        };
        let _ = sides;
        // (`DS_FIGHT=1`: what the rules played, sight by sight.)
        if std::env::var_os("DS_FIGHT").is_some() {
            for sight in &sights {
                eprintln!("{sight:?}");
            }
        }
        for sight in sights {
            match sight {
                Sight::Blow(blow) => {
                    if blow.landings.is_empty() {
                        self.blow(number, blow.who, blow.power.slot.id, None, false, false);
                    }
                    for landing in &blow.landings {
                        self.blow(number, blow.who, blow.power.slot.id, Some(landing.target), landing.missed, landing.critical);
                    }
                }
                Sight::Deed(deed) => self.blow(number, deed.who, deed.power.slot.id, deed.landing.as_ref().map(|landing| landing.target), true, false),
                Sight::Falls { who, .. } => {
                    self.plain(number, Some("fight"), &Beat::Body(BodyBeat::Anim { who, anim: engine::AnimId(33), play: Play::Once }));
                    self.line("Wait(40);");
                }
                Sight::Passage(label) => {
                    if let Some(text) = self.passages.get(&label).cloned() {
                        self.body.push_str(&text);
                    }
                }
                _ => {}
            }
        }
        self.note(number, Kind::Fight, Fate::Changed, "the fight: no bars, no log", "staged as its blows, with Sky's attacks");
    }

    /// A blow of a fight: who strikes turns to its target and attacks
    /// (`attack`'s staging); the one struck steps back in pain, further
    /// for a critical one, the view shaking; missed: it hops aside.
    pub(super) fn blow(&mut self, number: i32, who: Who, id: u16, target: Option<Who>, missed: bool, critical: bool) {
        let word = Some("fight");
        if let Some(target) = target.filter(|target| *target != who) {
            self.plain(number, word, &Beat::Body(BodyBeat::Face { who, other: target }));
        }
        self.plain(number, word, &Beat::Body(BodyBeat::Move { who, id: engine::MoveId(id), target: if missed { None } else { target } }));
        let Some(target) = target.filter(|target| *target != who) else {
            self.line("Wait(20);");
            return;
        };
        if missed {
            self.plain(number, word, &Beat::Body(BodyBeat::Anim { who: target, anim: engine::AnimId(10), play: Play::Times(1) }));
        } else {
            if critical {
                self.line("camera_SetEffect(2, 2, 3.0);");
            }
            let stood = self.at.get(&target).copied();
            self.recoil(number, target, who, if critical { 20.0 } else { 12.0 });
            self.plain(number, word, &Beat::Body(BodyBeat::Anim { who: target, anim: engine::AnimId(19), play: Play::Times(1) }));
            self.line("Wait(24);");
            if critical {
                self.line("camera_SetEffect(0, 0, 0);");
            }
            self.stand();
            // (Back to its place, as the fighters of Sky's scenes keep theirs: a long fight would push them apart.)
            if let (Some(there), Some(actor)) = (stood, self.who(number, target)) {
                self.go(target, &actor, there, "1", true);
                self.wait_for(&actor);
            }
        }
        self.line("Wait(12);");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fights_passages_are_its_own_labels() {
        assert!(fight_label("fight1_said0"));
        assert!(fight_label("fight12_past"));
        assert!(!fight_label("fighters"));
        assert!(!fight_label("fight_club"));
    }
}
