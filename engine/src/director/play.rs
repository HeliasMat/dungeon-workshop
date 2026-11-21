//! The story going on, a frame at a time.

use super::*;

/// What playing a beat comes to: on to the next, or the story stays on
/// this one, or it was sent elsewhere.
pub(super) enum Step {
    On,
    Holds,
    Leave,
}

impl Step {
    pub(super) fn of(waits: bool) -> Self {
        if waits { Step::Holds } else { Step::On }
    }
}

impl Director {
    /// Lets that many frames pass and plays what follows, up to what
    /// waits (`focus`: where the view looks now). Gives what the game is
    /// to do, in order.
    pub fn tick(&mut self, stage: &mut Stage, frames: f32, focus: Vec2) -> Vec<Call> {
        let mut calls = Vec::new();
        let Some(at) = self.at else {
            return calls;
        };
        if self.past_scenes(at, &mut calls) {
            return calls;
        }
        if !self.opened {
            self.open_scene(at, stage, &mut calls);
        }
        self.lines_go_by(stage, frames);
        self.play_scheduled(at, stage, frames, focus, &mut calls);
        if self.question_ran_out(frames, &mut calls) {
            return calls;
        }
        self.run_clocks(stage, frames);
        self.keep_words(stage);
        if self.hold.is_some() {
            return calls;
        }
        self.line_goes_by(frames, &mut calls);
        if !self.waited(frames) {
            return calls;
        }
        if let Some(at) = self.play_at_once(at, stage, focus, &mut calls) {
            self.take_course(at, stage, focus, &mut calls);
        }
        calls
    }

    /// Past an event's last scene, on to the next: whether it was.
    fn past_scenes(&mut self, at: Mark, calls: &mut Vec<Call>) -> bool {
        // Past an event's last scene: the next event, if any; past an event
        // played now, where the story was (see `interlude`).
        if self.events.get(at.event).and_then(|event| event.scenes.get(at.scene)).is_none()
            && let Some(Interlude { back, .. }) = self.interlude.filter(|interlude| interlude.at == at.event)
        {
            (self.interlude, self.at, self.opened) = (None, back, false);
            // (Played after the story's end: over again, as it was.)
            if back.is_none() {
                self.lines_end(|_| true);
                calls.push(Call::Over);
            }
            return true;
        }
        if self.events.get(at.event).and_then(|event| event.scenes.get(at.scene)).is_none() {
            let next = (at.event + 1..self.events.len()).find(|&next| !self.events[next].scenes.is_empty());
            (self.at, self.opened) = (next.map(|event| Mark { event, scene: 0, beat: 0 }), false);
            // (A place of that event not met: the next event plays as usual.)
            if self.author.skip_to.as_deref().is_some_and(|label| label.starts_with(PLACE_LABEL)) {
                self.author.skip_to = None;
            }
            if next.is_none() {
                self.lines_end(|_| true);
                calls.push(Call::Over);
            }
            return true;
        }
        false
    }

    /// The scene at hand opens.
    fn open_scene(&mut self, at: Mark, stage: &mut Stage, calls: &mut Vec<Call>) {
        self.opened = true;
        self.lines_end(|_| true);
        let cleared = std::mem::take(&mut self.wipe) || at.scene == 0;
        if cleared {
            *stage = Stage::dark();
            (self.pause, self.reading, self.timed) = (0.0, false, None);
            (self.schedule.later, self.schedule.returns, self.schedule.every, self.schedule.parallels) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            (self.manner.last_speaker, self.kept.hud, self.kept.gauges) = (None, Default::default(), Default::default());
            (self.asking.ask_timer, self.asking.ask_left, self.clocks.timers, self.asking.number_asked) = (None, None, Vec::new(), None);
        }
        self.clocks.scene_clock = 0.0;
        // (Who may be talked to and the ways out: the scene's own.)
        self.roam = Roaming { presses: self.vars.get(PRESSES).copied().unwrap_or(0.0), ..Roaming::default() };
        // (A film's scene opens with no collisions, the game's with them,
        // until the scene says otherwise.)
        stage.no_collisions = self.events.get(at.event).is_some_and(|event| event.film);
        // (A new event: saved anew from its start.)
        if at.scene == 0 {
            self.checkpoint = None;
        }
        // (The numbers of the event before, `_name`, gone.)
        if self.own_numbers_of != Some(at.event) {
            self.vars.retain(|name, _| !name.starts_with('_'));
            self.own_numbers_of = Some(at.event);
        }
        calls.push(Call::Opens { first: at.scene == 0, cleared });
    }

    /// What was to happen by now does.
    fn play_scheduled(&mut self, at: Mark, stage: &mut Stage, frames: f32, focus: Vec2, calls: &mut Vec<Call>) {
        // What was to happen later, when its time comes (whatever the scene
        // waits for meanwhile).
        self.clocks.clock += frames;
        if !self.schedule.later.is_empty() {
            let mut due = Vec::new();
            self.schedule.later.retain_mut(|later| {
                later.left -= frames;
                if later.left <= 0.0 {
                    due.push(later.beat.clone());
                }
                later.left > 0.0
            });
            for beat in due {
                self.happen(beat, stage, focus, calls);
            }
        }
        // (What was waiting for a blow on someone.)
        let (heard, waiting) =
            std::mem::take(&mut self.schedule.on_hit).into_iter().partition(|(who, hits, _)| stage.cast.get(who).is_some_and(|actor| actor.hits > *hits));
        let heard: Vec<(Who, u32, Beat)> = heard;
        self.schedule.on_hit = waiting;
        for (_, _, beat) in heard {
            self.happen(beat, stage, focus, calls);
        }
        let mut due = Vec::new();
        for (at, again) in self.schedule.every.iter_mut().enumerate() {
            again.left -= frames;
            if again.left <= 0.0 {
                due.push(at);
            }
        }
        // (Again after its while, and as much again drawn by chance.)
        for &at in &due {
            let more = self.chance(self.schedule.every[at].more as u32 + 1) as f32;
            let again = &mut self.schedule.every[at];
            again.left += again.between + more;
        }
        let due: Vec<Beat> = due.into_iter().map(|at| self.schedule.every[at].beat.clone()).collect();
        for beat in due {
            self.happen(beat, stage, focus, calls);
        }
        // (The passages played meanwhile, of the scene at hand only, even
        // while a line is read or a question asked.)
        if !self.schedule.parallels.is_empty() {
            let mut parallels = std::mem::take(&mut self.schedule.parallels);
            parallels.retain_mut(|parallel| {
                self.plays_on(Some(parallel.thread));
                let goes = (parallel.at.event, parallel.at.scene) == (at.event, at.scene) && self.play_parallel(parallel, stage, frames, focus, calls);
                self.plays_on(None);
                if !goes {
                    self.thread_ends(parallel.thread);
                }
                goes
            });
            // (Those started meanwhile after them.)
            parallels.append(&mut self.schedule.parallels);
            self.schedule.parallels = parallels;
        }
    }

    /// A question given its time: whether that time just ran out.
    fn question_ran_out(&mut self, frames: f32, calls: &mut Vec<Call>) -> bool {
        // (A question given its time: the seconds left, then the answer
        // taken for the player.)
        if self.hold == Some(Hold::Question)
            && let Some(Timed { left, answer: default }) = self.asking.ask_left
        {
            let left = left - frames;
            self.vars.insert(TIME_LEFT.to_string(), (left.max(0.0) / 60.0).ceil() as f64);
            self.asking.ask_left = Some(Timed { left, answer: default });
            if left <= 0.0 {
                let shown = self.asking.offered.iter().position(|&place| place == default).unwrap_or(0);
                self.answer(Some(shown));
                self.vars.insert(TOO_LATE.to_string(), 1.0);
                calls.push(Call::LineGone);
                // (On along that answer's way from the next frame.)
                return true;
            }
        }
        false
    }

    fn run_clocks(&mut self, stage: &Stage, frames: f32) {
        // (The scene's clock; who is walked about.)
        self.clocks.scene_clock += frames;
        self.vars.insert(CLOCK.to_string(), self.clocks.scene_clock as f64);
        self.clocks.music.pass(frames);
        if let Some(played) = self.clocks.music.reading() {
            self.vars.insert(MUSIC_CLOCK.to_string(), played);
        }
        match stage.play.controlled {
            Some(who) => _ = self.vars.insert(LED.to_string(), who.number() as f64),
            None => _ = self.vars.remove(LED),
        }
        // (Each countdown: its seconds left; one at 0 stops.)
        for (var, left) in &mut self.clocks.timers {
            *left = (*left - frames).max(0.0);
            self.vars.insert(var.clone(), (*left / 60.0).ceil() as f64);
        }
        self.clocks.timers.retain(|(_, left)| *left > 0.0);
    }

    fn keep_words(&self, stage: &mut Stage) {
        // (A line on screen: what the view leaves room for.)
        stage.words.boxed = self.reading;
        // (The words kept on screen, with their numbers now.)
        stage.words.hud = std::array::from_fn(|at| self.kept.hud[at].as_ref().map(|text| emphasis(&self.fill(text))));
        stage.words.gauges = std::array::from_fn(|at| {
            let GaugeOf { var, most, colour } = self.kept.gauges[at].as_ref()?;
            let (now, most) = (self.vars.get(var).copied().unwrap_or(0.0), self.value(most).max(1.0));
            Some(Gauge { full: (now as f32 / most as f32).clamp(0.0, 1.0), colour: *colour })
        });
    }

    fn line_goes_by(&mut self, frames: f32, calls: &mut Vec<Call>) {
        // (A line that goes by itself, after its time.)
        if let Some(left) = self.timed.filter(|_| self.reading) {
            let left = left - frames;
            self.timed = (left > 0.0).then_some(left);
            if left <= 0.0 {
                self.reading = false;
                calls.push(Call::LineGone);
            }
        }
    }

    /// Whether the wait at hand is over.
    fn waited(&mut self, frames: f32) -> bool {
        if self.pause > 0.0 {
            self.pause -= frames;
            if self.pause > 0.0 {
                return false;
            }
        }
        true
    }
}
