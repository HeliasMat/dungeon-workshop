//! Going somewhere: walking, following, chasing, fleeing.

use engine::{Beat, MotionBeat};

use super::{BESIDE, Writer};
use crate::sky::speed;

impl Writer<'_> {
    /// A motion beat: who goes where, after whom.
    pub(super) fn motion(&mut self, number: i32, word: Option<&str>, beat: &MotionBeat) {
        match beat {
            MotionBeat::Go { who, at, frames, slide } => {
                let (Some(actor), Some(to)) = (self.who(number, *who), self.point(at)) else {
                    self.problems.push(format!("line {number}: where {who:?} goes is not known before the film plays"));
                    return;
                };
                let from = self.at.get(who).copied().unwrap_or(to);
                let distance = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
                self.go(*who, &actor, to, &speed(distance, frames.0), *slide);
            }
            // Following and chasing, as LIMITS found Sky can: the follower
            // walks to where the leader is (`MovePositionLives`) as it goes.
            MotionBeat::Follow { who, leader, .. } => match leader {
                Some(leader) => {
                    self.followers.retain(|(one, _)| one != who);
                    self.followers.push((*who, *leader));
                }
                None => self.followers.retain(|(one, _)| one != who),
            },
            MotionBeat::Chase { who, other } if word == Some("chase") => {
                let (Some(actor), Some(other_actor)) = (self.who(number, *who), self.who(number, *other)) else { return };
                self.line(&format!("MovePositionLives<actor {actor}>(3, {other_actor});"));
                self.moving(&actor);
                if let Some(&at) = self.at.get(other) {
                    self.at.insert(*who, at);
                }
            }
            MotionBeat::Approach { who, other, .. } | MotionBeat::Chase { who, other } => {
                let (Some(actor), Some(&to), Some(&from)) = (self.who(number, *who), self.at.get(other), self.at.get(who)) else { return };
                let side = if from.0 <= to.0 { -BESIDE } else { BESIDE };
                self.go(*who, &actor, (to.0 + side, to.1), "1", false);
            }
            MotionBeat::Flee { who, from, distance } => {
                let (Some(actor), Some(&at), Some(&away)) = (self.who(number, *who), self.at.get(who), self.at.get(from)) else { return };
                let way = if at.0 >= away.0 { 1.0 } else { -1.0 };
                self.calm();
                self.go(*who, &actor, (at.0 + way * distance, at.1), "2", false);
            }
            other => self.unexported(number, word, &Beat::Motion(other.clone())),
        }
    }
}
