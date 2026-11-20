//! Who and what a line speaks of, or to, out of the frame.

use super::watch::*;
use super::*;

impl Watch {
    /// Who a line speaks of by name, or speaks to (who spoke before), out of the
    /// frame for more than a second of it.
    pub(super) fn spoken_of(&mut self, stage: &Stage, centre: Vec2, speaker: Option<Who>, text: &str, line: i32) {
        let words = without_tags(text).to_lowercase();
        // (A name two of them go by says no one; a crowd is not spoken of by name.)
        let is_named = |who: &Who, name: &String| {
            !self.crowd.contains(who)
                && name.chars().count() >= 3
                && words.contains(&name.to_lowercase())
                && self.names.iter().filter(|(other, known)| *known == name && !self.crowd.contains(other)).count() == 1
        };
        let mut concerned: Vec<(Who, bool)> =
            self.names.iter().filter(|(who, name)| Some(**who) != speaker && is_named(who, name)).map(|(who, _)| (*who, true)).collect();
        // (Who spoke before is spoken to only if this line answers it: it
        // asked something, or called this one by name.)
        let called = |said: &str| {
            speaker.and_then(|speaker| self.names.get(&speaker)).is_some_and(|name| name.chars().count() >= 3 && said.contains(&name.to_lowercase()))
        };
        let answered = self.before.as_ref().filter(|(other, said)| {
            speaker.is_some_and(|speaker| speaker != *other) && (said.contains('?') || called(said)) && !concerned.iter().any(|(who, _)| who == other)
        });
        if let Some((other, _)) = answered {
            concerned.push((*other, false));
        }
        let zoom = stage.view.closeness();
        for (who, by_name) in concerned {
            let Some(actor) = stage.cast.get(&who).filter(|actor| actor.attributes & attribute::HIDDEN == 0 && actor.shown_opacity() > 0.1) else {
                continue;
            };
            let shown = (actor.position - centre) * zoom;
            let out = shown.x.abs() > HALF.x || shown.y > HALF.y + 8.0 || shown.y < -HALF.y + HEAD;
            let frames = self.away.entry(who).or_insert(0);
            *frames = if out { *frames + 1 } else { 0 };
            if *frames == 60 {
                let words: String = without_tags(text).chars().filter(|letter| *letter != '\n').take(30).collect();
                let (side, words) = (super::side(actor.position, centre, zoom), words.trim().to_string());
                let said = match (speaker, by_name) {
                    (Some(speaker), true) => told!("{} is {}, while {} speaks of them (“{}”) → frame {} {}", who, side, speaker, words, speaker, who),
                    (Some(speaker), false) => told!("{} is {}, while {} speaks to them (“{}”) → frame {} {}", who, side, speaker, words, speaker, who),
                    (None, _) => told!("{} is {}, while they are spoken of (“{}”) → camera on {}", who, side, words, who),
                };
                self.tell(line, format!("absent {who} {line}"), said);
            }
        }
    }

    /// A thing a line names, out of the frame.
    pub(super) fn things_spoken_of(&mut self, stage: &Stage, centre: Vec2, text: &str, line: i32) {
        let words = without_tags(text).to_lowercase();
        let zoom = stage.view.closeness();
        for thing in stage.things().into_iter().filter(|thing| thing.name.len() >= 3 && words.contains(&thing.name.to_lowercase())) {
            let shown = (thing.place - centre) * zoom;
            if shown.x.abs() > HALF.x || shown.y.abs() > HALF.y {
                let told = format!("{} is spoken of, and is {}", thing.name, super::side(thing.place, centre, zoom));
                self.tell(line, format!("item {line} {}", thing.id), told);
            }
        }
    }
}
