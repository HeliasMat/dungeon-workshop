//! A line a scene says, as the message box shows it: the same for the game
//! and a film.

use engine::{Call, emphasis};

use super::Play;
use crate::ui::message::Said;

impl Play {
    /// The line of a `Call::Say`, the names in it put in; `timed`: it goes
    /// when the scene says, not with a key.
    pub fn said(&self, call: &Call, timed: bool) -> Option<Said> {
        let Call::Say { entity, face, text, name, voice, right, speed } = call else {
            return None;
        };
        let (monster, own) = entity.map_or((0, String::new()), |who| self.sim.cast.who(who));
        // (A name shown instead of its own may name someone: `{name:10}`.)
        let name = name.as_ref().map_or(own, |name| self.sim.cast.names_in(name));
        let said = emphasis(&self.sim.cast.names_in(text));
        let page = if name.is_empty() { said } else { format!("[CS:N]{name}[CR]: {said}") };
        Some(Said { pages: vec![page], portrait: face.map(|face| (monster, face, *right)), voice: *voice, pace: *speed, timed, ..Said::default() })
    }
}
