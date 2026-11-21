//! A beat played at once, or as far as the story goes without waiting: its
//! flow, what it asks of the player, a course taken.

use super::play::Step;
use super::*;

impl Director {
    /// Plays all that happens at once, up to what takes time or waits:
    /// where the story is then, unless it stays there.
    pub(super) fn play_at_once(&mut self, mut at: Mark, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) -> Option<Mark> {
        // What happens at once does, one thing after the other, up to what
        // takes time or waits.
        // (No more than so many at once: a scene going round in circles
        // without a wait goes on in the next frame.)
        const AT_ONCE: usize = 4096;
        let mut played = 0;
        while let Some(beat) = self.beat(at).filter(|beat| beat.staged()).cloned() {
            self.came_back(at, stage);
            played += 1;
            if played > AT_ONCE {
                return None;
            }
            // (A command at a time, once let go.)
            if self.author.stepping && !self.hurrying() {
                if self.author.steps == 0 {
                    return None;
                }
                self.author.steps -= 1;
            }
            if let Some(trace) = &mut self.author.trace {
                trace.push(format!("{}:{}:{} {beat:?}", at.event, at.scene, at.beat));
            }
            let beat = self.drawn(beat);
            at.beat += 1;
            self.at = Some(at);
            // (Waits, ifs, loops and gotos: as a passage meanwhile plays
            // them, see `way_on`.)
            match self.play_one(beat, &mut at, stage, focus, calls) {
                Step::On => {}
                Step::Holds => {
                    at.beat -= 1;
                    self.at = Some(at);
                    return None;
                }
                Step::Leave => return None,
            }
        }
        self.came_back(at, stage);
        Some(at)
    }

    /// The place a save kept is reached (see `keep_place`): the event plays
    /// as usual from here.
    fn came_back(&mut self, at: Mark, stage: &mut Stage) {
        if self.author.skip_to.as_deref() == Some(place_label(at).as_str()) {
            self.author.skip_to = None;
            stage.settle();
            // (A game taken up again: its numbers and words as they were.)
            if let Some((numbers, texts)) = self.author.restore.take() {
                self.vars.extend(numbers);
                self.texts.extend(texts);
                self.roamed.resume_at = self.author.restore_at.take();
            }
        }
    }

    /// Plays a beat of the scene's record.
    fn play_one(&mut self, beat: Beat, at: &mut Mark, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) -> Step {
        if let Some(waits) = self.way_on(&beat, at, stage) {
            self.at = Some(*at);
            return Step::of(waits);
        }
        match beat {
            Beat::Flow(beat) => self.play_flow(beat, at, stage, focus, calls),
            // (The line on screen is not read yet.)
            Beat::Speech(SpeechBeat::Said) => Step::of(self.reading),
            beat => {
                self.happen(beat, stage, focus, calls);
                Step::On
            }
        }
    }

    /// The player roams (see `FlowBeat::Roam`): a press before someone who may
    /// be talked to plays their passage, then comes back here; a way out
    /// reached goes to its scene; else the scene waits.
    fn roaming(&mut self, at: &mut Mark, stage: &mut Stage, calls: &mut Vec<Call>) -> Step {
        let Some(me) = stage.play.controlled else {
            return Step::On;
        };
        // (A game taken up again: where it stood when saved.)
        if let Some(saved) = self.roamed.resume_at.take()
            && let Some(actor) = stage.cast.get_mut(&me)
        {
            actor.position = saved;
            (actor.way.to, actor.way.left) = (actor.position, 0.0);
            self.roam.armed = false;
        }
        // (Back from a passage: the presses made while it played (between its
        // lines, to read them) are not one more talk to whoever was last before
        // the player.)
        let presses = self.vars.get(PRESSES).copied().unwrap_or(0.0);
        if std::mem::take(&mut self.roam.away) {
            self.roam.presses = presses;
        }
        // (The menu asked for: its passage, then back here.)
        if std::mem::take(&mut self.roam.menu_asked)
            && let Some(past) = self.roam.menu.clone().and_then(|label| self.past_label(*at, &label))
        {
            self.schedule.returns.push(Mark { beat: at.beat - 1, ..*at });
            (at.beat, self.roam.away) = (past, true);
            self.at = Some(*at);
            return Step::On;
        }
        if presses != self.roam.presses {
            self.roam.presses = presses;
            let talks = self.vars.get(TALKS).is_some_and(|talks| *talks != 0.0);
            let target = self.vars.get(TARGET).copied().unwrap_or(-1.0);
            let passage = self.roam.talks.iter().find(|(who, _)| talks && who.number() as f64 == target).map(|(_, label)| label.clone());
            if let Some(past) = passage.and_then(|label| self.past_label(*at, &label)) {
                // (Back to the roaming itself, once said.)
                self.schedule.returns.push(Mark { beat: at.beat - 1, ..*at });
                (at.beat, self.roam.away) = (past, true);
                self.at = Some(*at);
                return Step::On;
            }
        }
        if let Some(position) = stage.cast.get(&me).map(|actor| actor.position) {
            let near = self.roam.doors.iter().find(|(_, _, there, reach)| reach.holds(position - *there)).cloned();
            match near {
                Some((scene, arrive, ..)) if self.roam.armed => {
                    if let Some(number) = self.events[at.event].scenes.iter().position(|one| one.name == scene) {
                        self.roamed.arrival = Some(arrive);
                        (self.at, self.opened) = (Some(Mark { event: at.event, scene: number, beat: 0 }), false);
                        return Step::Leave;
                    }
                    // (A way of the game's own, `game go town south`: told to
                    // the game, which takes it; roaming goes on meanwhile.)
                    let words: Option<Vec<String>> = scene.strip_prefix("game ").map(|asked| asked.split_whitespace().map(str::to_string).collect());
                    if let Some(ask) = words.as_deref().and_then(GameAsk::of_words) {
                        self.roam.armed = false;
                        calls.push(Call::Game { ask, from: *at });
                    }
                }
                Some(_) => {}
                None => self.roam.armed = true,
            }
        }
        Step::Holds
    }

    /// A beat of the flow that does not depend on a test.
    fn play_flow(&mut self, beat: FlowBeat, at: &mut Mark, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) -> Step {
        let waits = match beat {
            FlowBeat::Label(label) => {
                if self.author.skip_to.as_ref() == Some(&label) {
                    self.author.skip_to = None;
                    stage.settle();
                }
                self.author.last_label = Some(label);
                false
            }
            // (Another scene of the event, from its start.)
            FlowBeat::GotoScene(name) => {
                if let Some(scene) = self.events[at.event].scenes.iter().position(|scene| scene.name == name) {
                    (self.at, self.opened) = (Some(Mark { event: at.event, scene, beat: 0 }), false);
                    return Step::Leave;
                }
                false
            }
            // (Back past where the passage was started from, of this scene only.)
            FlowBeat::Return => {
                if let Some(back) = self.schedule.returns.pop().filter(|back| (back.event, back.scene) == (at.event, at.scene)) {
                    *at = back;
                    self.at = Some(*at);
                }
                false
            }
            FlowBeat::Wait(_) if self.hurrying() => false,
            FlowBeat::Wait(frames) => {
                self.pause += frames.0;
                if self.pause > 0.0 {
                    return Step::Leave;
                }
                false
            }
            FlowBeat::OnTalk { who, label } => {
                self.roam.talks.retain(|(one, _)| *one != who);
                self.roam.talks.push((who, label));
                false
            }
            FlowBeat::Door { scene, arrive, at: there, radius, wide, high } => {
                let reach = if wide > 0.0 && high > 0.0 { Reach::Rect(Vec2::new(wide, high) / 2.0) } else { Reach::Circle(radius) };
                self.roam.doors.push((scene, arrive, there.vec2(), reach));
                false
            }
            FlowBeat::OnMenu(label) => {
                self.roam.menu = Some(label);
                false
            }
            FlowBeat::SaveGame => {
                // (Where it roams, if played from roaming; else here.)
                let place = self.schedule.returns.last().copied().filter(|back| (back.event, back.scene) == (at.event, at.scene)).unwrap_or(*at);
                if let Some(standing) = stage.play.controlled.and_then(|who| stage.cast.get(&who)).map(|actor| actor.position) {
                    self.roamed.saved_at = Some(standing);
                }
                self.checkpoint = Some(place_label(place));
                calls.push(Call::Checkpoint);
                false
            }
            FlowBeat::Roam if self.hurrying() || self.author.nobody_plays => false,
            FlowBeat::Roam => return self.roaming(at, stage, calls),
            beat => {
                self.happen(Beat::Flow(beat), stage, focus, calls);
                false
            }
        };
        Step::of(waits)
    }

    /// On to what the scene does next; past its end, the next scene.
    pub(super) fn take_course(&mut self, at: Mark, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) {
        // (What comes next waits for the line on screen to be read; but a
        // question right after a line not waited for is asked under it,
        // as the original asks: see `question` in the files.)
        if self.reading {
            if !matches!(self.beat(at), Some(Beat::Course(Course::Ask(_)))) {
                return;
            }
            (self.reading, self.timed) = (false, None);
        }
        // On to what the scene does next; past its end, the next scene.
        let next = Some(Mark { beat: at.beat + 1, ..at });
        let course = match self.beat(at).cloned() {
            Some(Beat::Course(course)) => {
                if let Some(trace) = &mut self.author.trace {
                    trace.push(format!("{}:{}:{} {course:?}", at.event, at.scene, at.beat));
                }
                course
            }
            // (Past the scene's end: the next scene.)
            None => {
                (self.at, self.opened) = (Some(Mark { event: at.event, scene: at.scene + 1, beat: 0 }), false);
                return;
            }
            // (Beats of the scene's record are played as it goes.)
            Some(_) => {
                self.at = next;
                return;
            }
        };
        match course {
            // (An answer's way on ends where the next one starts.)
            // (Unless the question's answers end with `endask`: past it, in
            // the same scene.)
            Course::Answer(_) | Course::End => match matches!(course, Course::Answer(_)).then(|| self.past_choice(at)).flatten() {
                Some(beat) => self.at = Some(Mark { beat, ..at }),
                None => (self.at, self.opened) = (Some(self.past_way(at)), false),
            },
            Course::Rejoin => self.at = next,
            Course::Host(fields) => {
                match self.words.host_holds.contains(&fields[0].as_str()) {
                    true => self.hold = Some(Hold::Host),
                    false => self.at = next,
                }
                self.host(fields, at, calls);
            }
            Course::Game(ask) => {
                match ask.holds() {
                    true => self.hold = Some(Hold::Host),
                    false => self.at = next,
                }
                calls.push(Call::Game { ask, from: at });
            }
            // (Skipped: as read at once; the first answer.)
            Course::Say { .. } | Course::Ask(_) if self.author.skip_to.is_some() => {
                let way = match &course {
                    Course::Ask(answers) => self.branch(at, self.offered(answers).first().map_or(0, |(place, _)| *place)),
                    _ => Mark { beat: at.beat + 1, ..at },
                };
                self.opened &= way.scene == at.scene;
                self.at = Some(way);
            }
            Course::Say { entity, face, text, name, timed } => {
                self.at = next;
                self.say(entity, face, &text, name, stage, focus, calls);
                // (Its time, when it has one, is known as it shows: the box
                // goes by itself from its first frame, keys or none.)
                (self.timed, self.said_meanwhile) = (timed.map(Frames::get), false);
            }
            Course::Ask(answers) => {
                self.hold = Some(Hold::Question);
                let offered = self.offered(&answers);
                self.asking.offered = offered.iter().map(|(place, _)| *place).collect();
                calls.push(Call::Ask(offered.iter().map(|(_, answer)| emphasis(&self.fill(answer))).collect()));
            }
        }
    }
}
