//! Events played outside the story's order.

use super::*;

impl Director {
    /// Plays an event of the game's own before the one at hand (after the
    /// last when the story is over), from its start.
    pub fn insert(&mut self, event: StoryEvent) {
        // (An event played now and left at its end: over, the story back
        // where it was, not to be played again.)
        if let Some(Interlude { back, .. }) = self.interlude.take() {
            self.at = back;
        }
        let before = self.at.map_or(self.events.len(), |at| at.event).min(self.events.len());
        self.events.insert(before, event);
        self.moved_on(before);
        self.at = Some(Mark { event: before, scene: 0, beat: 0 });
        self.resume();
    }

    /// Plays an event now, wherever the story is (a scene of a place the
    /// team walks into), then goes back to where the story was, as it
    /// was: not to the start of the event it was in, as `insert` does.
    pub fn interlude(&mut self, event: StoryEvent) {
        // (After another left at its end, the team walking again: back
        // where the story was before them both.)
        let (at, back) = match self.interlude {
            Some(Interlude { at, back }) => (at + 1, back),
            None => (self.at.map_or(self.events.len(), |back| back.event).min(self.events.len()), self.at),
        };
        self.events.insert(at, event);
        self.moved_on(at);
        // (The event it was in is one further now.)
        let back = back.map(|back| Mark { event: back.event + 1, ..back });
        self.interlude = Some(Interlude { at, back });
        self.at = Some(Mark { event: at, scene: 0, beat: 0 });
        self.resume();
    }

    /// An event put before those from `from` on: what is kept of where they
    /// were (a `gosub` to go back from, a passage played meanwhile) one
    /// further too.
    fn moved_on(&mut self, from: usize) {
        let on = |mark: &mut Mark| {
            if mark.event >= from {
                mark.event += 1;
            }
        };
        self.schedule.returns.iter_mut().for_each(on);
        self.schedule.parallels.iter_mut().for_each(|parallel| on(&mut parallel.at));
    }

    /// Whether an event played now is not over yet (see `interlude`).
    #[cfg(test)]
    pub(crate) fn in_interlude(&self) -> bool {
        self.interlude.is_some()
    }

    /// Whether the story is over: no event left, though one played now
    /// may have left the director at its end.
    pub fn over(&self) -> bool {
        self.at.is_none() || matches!(self.interlude, Some(Interlude { back: None, .. }))
    }
}
