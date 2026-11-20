//! Tournaments and crowds.

use super::*;
use crate::fight::crowd::HEALTH;

impl Staging {
    /// A round of a tournament is told.
    pub(crate) fn round_told(&mut self, round: usize, rounds: usize, foe: Who, comes_in: bool) {
        if comes_in {
            let at = self.slots[1];
            add!(self, "run", foe, at.x as i32, at.y as i32);
            add!(self, "waitfor", foe);
            self.at.insert(foe, at);
        }
        add!(self, "caption", 90, format!("Fight {} of {rounds}", round + 1));
        add!(self, "t", 100);
    }

    /// Who lost a round of a tournament goes.
    pub(crate) fn round_lost_by(&mut self, foe: Who) {
        self.flown.push(foe);
        add!(self, "vanish", foe, 24);
        add!(self, "t", 40);
    }

    /// The tournament is won.
    pub(crate) fn champion(&mut self, who: Who) {
        add!(self, "jingle", 7);
        add!(self, "flash", "gold", 20);
        add!(self, "emote", who, "joy", 60);
        add!(self, "jump", who, 14, 14);
        let name = self.names.get(&who).cloned().unwrap_or_default();
        add!(self, "caption", 180, format!("{name} wins the tournament!"));
        add!(self, "t", 190);
    }

    /// What happens of two crowds, as lines.
    pub(crate) fn rush(&mut self, rush: &Rush) {
        match rush {
            Rush::Starts => add!(self, "sound", "hop"),
            Rush::Strides { who, to, place } => {
                add!(self, "after", (*place as i32) % GOING, "run", who, to.x as i32, to.y as i32);
                self.at.insert(*who, *to);
            }
            Rush::Strikes { who, other, id, side, place, around } => {
                let delay = strike_delay(*side, *place);
                add!(self, "after", delay, "face", who, other);
                add!(self, "after", delay, "move", who, id, other);
                if *around {
                    add!(self, "after", delay + 6, "shake", 0, 2, 12);
                }
            }
            Rush::Takes { one, by, taken, left, side, place } => self.takes(*one, *by, *taken, *left, *side, *place),
            Rush::Ends => {
                add!(self, "sound", "shock");
                add!(self, "quake", 1, 20);
                add!(self, "t", ROUND);
            }
            Rush::Runs { who, to } => {
                let at = self.roll(16);
                add!(self, "emote", who, "sweat", 40);
                add!(self, "bar", who, "off");
                add!(self, "after", at, "runto", who, to.x as i32, to.y as i32);
                add!(self, "after", FLIGHT - EDGE, "vanish", who, EDGE);
                self.flown.push(*who);
            }
            Rush::Ran => add!(self, "t", FLIGHT),
            Rush::Glad { who } => {
                let (first, second) = (self.roll(20), self.roll(20));
                add!(self, "bar", who, "off");
                add!(self, "after", first, "emote", who, "joy", 50);
                add!(self, "after", second, "jump", who, 10, 12);
            }
        }
    }

    /// A blow gets to someone, who goes if it was the last: it takes its
    /// blow, blinks, and goes, as in dungeons.
    pub(crate) fn takes(&mut self, one: Who, by: Who, taken: i32, left: i32, side: usize, place: usize) {
        let delay = strike_delay(side, place) + HIT_FRAMES;
        self.pushed(delay, one, by, 8);
        if place.is_multiple_of(NUMBERS) {
            add!(self, "after", delay + 2, "popup", one, "yellow", format!("-{taken}"));
        }
        if left > 0 {
            add!(self, "after", delay + 2, "bar", one, format!("{:.2}", left as f32 / HEALTH as f32));
        } else if !self.flown.contains(&one) {
            self.flown.push(one);
            add!(self, "after", delay + 2, "bar", one, "off");
            add!(self, "after", delay + 10, "blink", one, "on");
            add!(self, "after", delay + 10 + FAINT_BLINK, "gone", one);
        }
    }
}
