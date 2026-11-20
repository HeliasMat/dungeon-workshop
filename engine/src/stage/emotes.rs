//! What shows over someone of the stage, the one home of it: an emote or a
//! move's effect (joy, surprise, sweat, sparks...), a status (asleep,
//! confused...), words and digits rising. Their timings are kept here, and
//! where on a body they show; whoever draws them reads [`Emotes`].

use std::collections::BTreeMap;

use glam::Vec2;

use super::Actor;
use crate::who::Who;

/// How high over its feet a usual body's head is: where words rise from,
/// where a bubble points.
pub const HEAD: f32 = 30.0;

/// The frames words rise for, and how far a frame.
const POPUP_FRAMES: f32 = 70.0;
const POPUP_RISE: f32 = 0.3;

/// How high over its feet a status shows (a frozen one: on the body).
const STATUS_OVER: f32 = 26.0;
const FROZEN_OVER: f32 = 12.0;

/// An effect shown on someone (an emote, a move's sparks...): which of
/// the game's, and the frames it still lasts (`None`: as long as the
/// game's own lasts, for ever for one that repeats).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effect {
    pub id: u16,
    pub left: Option<f32>,
}

/// A few words rising and fading (see `BodyBeat::Popup`): where they are, of
/// what colour, and the frames they have left.
#[derive(Clone, Debug, PartialEq)]
pub struct Popup {
    pub at: Vec2,
    pub text: String,
    pub colour: [u8; 3],
    pub left: f32,
}

/// All that shows over those of the stage.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Emotes {
    /// The effects each one shows, the first given first (as many as
    /// given: see `BodyBeat::Effect`'s `add`).
    pub effects: BTreeMap<Who, Vec<Effect>>,
    /// The status icons shown over someone, the first given first (see
    /// `BodyBeat::Status`).
    pub statuses: BTreeMap<Who, Vec<&'static str>>,
    /// The words rising from those of the scene (see `BodyBeat::Popup`).
    pub popups: Vec<Popup>,
}

impl Emotes {
    /// Someone shows that effect for so many frames (none: its own time),
    /// in the place of those it showed, or with them (`add`); the same
    /// effect again starts over. 0, or someone not there: none.
    pub(crate) fn effect(&mut self, who: Who, id: u16, frames: Option<f32>, here: bool, add: bool) {
        if id == 0 || !here {
            self.effects.remove(&who);
            return;
        }
        let shown = self.effects.entry(who).or_default();
        if !add {
            shown.clear();
        }
        shown.retain(|effect| effect.id != id);
        shown.push(Effect { id, left: frames });
    }

    /// An effect someone shows is over (its time as the game has it is
    /// up), the others it shows staying.
    pub fn effect_over(&mut self, who: Who, id: u16) {
        if let Some(shown) = self.effects.get_mut(&who) {
            shown.retain(|effect| effect.id != id);
            if shown.is_empty() {
                self.effects.remove(&who);
            }
        }
    }

    /// The effects someone shows.
    pub fn effects_of(&self, who: Who) -> &[Effect] {
        self.effects.get(&who).map_or(&[], Vec::as_slice)
    }

    /// A status over someone there, in the place of those it had, or with
    /// them (`add`); none: all taken away.
    pub(crate) fn status(&mut self, who: Who, icon: Option<&'static str>, here: bool, add: bool) {
        match icon.filter(|_| here) {
            Some(icon) => {
                let shown = self.statuses.entry(who).or_default();
                if !add {
                    shown.clear();
                }
                if !shown.contains(&icon) {
                    shown.push(icon);
                }
            }
            None => _ = self.statuses.remove(&who),
        }
    }

    /// The statuses over someone.
    pub fn statuses_of(&self, who: Who) -> &[&'static str] {
        self.statuses.get(&who).map_or(&[], Vec::as_slice)
    }

    /// Words rising from over someone's head.
    pub(crate) fn popup(&mut self, actor: &Actor, text: String, colour: [u8; 3]) {
        self.popups.push(Popup { at: actor.over_head(HEAD), text, colour, left: POPUP_FRAMES });
    }

    /// The words rise and fade.
    pub(crate) fn rise(&mut self, frames: f32) {
        for popup in &mut self.popups {
            popup.at.y -= POPUP_RISE * frames;
            popup.left -= frames;
        }
        self.popups.retain(|popup| popup.left > 0.0);
    }

    /// Effects go once their time is up; statuses and effects of who left go.
    pub(crate) fn pass(&mut self, frames: f32, cast: &BTreeMap<Who, Actor>) {
        self.statuses.retain(|who, _| cast.contains_key(who));
        self.effects.retain(|who, shown| {
            let time = cast.get(who).map_or(1.0, |actor| actor.looks.time);
            shown.retain_mut(|effect| {
                // (In the time of who it shows on.)
                if let Some(left) = &mut effect.left {
                    *left -= frames * time;
                }
                effect.left.is_none_or(|left| left > 0.0)
            });
            cast.contains_key(who) && !shown.is_empty()
        });
    }

    /// Where a status shows over that one.
    pub fn status_point(actor: &Actor, icon: &str) -> Vec2 {
        actor.over_head(if icon == "frozen" { FROZEN_OVER } else { STATUS_OVER })
    }
}

impl Actor {
    /// The point so high over its feet, as it shows (its height and its
    /// trembling with it).
    pub fn over_head(&self, rise: f32) -> Vec2 {
        self.shown_at() - Vec2::new(0.0, rise + self.shown_height())
    }
}
