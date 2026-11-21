//! Where a scene goes: ifs, loops, answers, passages played meanwhile.

use super::*;

impl Director {
    /// The beat just past a label of the scene `at` is in.
    pub(super) fn past_label(&self, at: Mark, label: &str) -> Option<usize> {
        self.ways(at).past_label(label)
    }

    /// The ways of the scene `at` is in, worked out once.
    fn ways(&self, at: Mark) -> std::sync::Arc<Ways> {
        let scene = &self.events[at.event].scenes[at.scene];
        scene.ways.of(&scene.beats)
    }

    fn beats_long(&self, at: Mark) -> usize {
        self.events[at.event].scenes[at.scene].beats.len()
    }

    /// Plays a passage played meanwhile up to what it waits for. Whether
    /// it goes on (not at its `Return`, its scene's end, a line or
    /// anything else of the story's course: those are the scene's own).
    pub(super) fn play_parallel(&mut self, parallel: &mut Parallel, stage: &mut Stage, frames: f32, focus: Vec2, calls: &mut Vec<Call>) -> bool {
        if parallel.pause > 0.0 {
            parallel.pause -= frames;
            if parallel.pause > 0.0 {
                return true;
            }
        }
        pub(super) const AT_ONCE: usize = 4096;
        for _ in 0..AT_ONCE {
            let Some(beat) = self.beat(parallel.at).cloned() else {
                return false;
            };
            let beat = self.drawn(beat);
            parallel.at.beat += 1;
            let hurrying = self.hurrying();
            let waits = match self.way_on(&beat, &mut parallel.at, stage) {
                Some(waits) => waits,
                None => match beat {
                    // (A line said meanwhile: once no other is on screen, it goes
                    // by itself after its time; played at once, not said.)
                    Beat::Course(Course::Say { .. }) if hurrying => false,
                    Beat::Course(Course::Say { .. }) if self.reading || self.hold.is_some() => true,
                    Beat::Course(Course::Say { entity, face, text, name, .. }) => {
                        self.say(entity, face, &text, name, stage, focus, calls);
                        self.said_meanwhile = true;
                        self.timed = Some(line_frames("", &text, false) as f32);
                        false
                    }
                    Beat::Speech(SpeechBeat::Said) => self.reading && !hurrying,
                    Beat::Flow(FlowBeat::Return) | Beat::Course(_) => return false,
                    Beat::Flow(FlowBeat::Wait(_)) if hurrying => false,
                    Beat::Flow(FlowBeat::Wait(frames)) => {
                        parallel.pause += frames.0;
                        if parallel.pause > 0.0 {
                            return true;
                        }
                        false
                    }
                    // (What only the scene's own course does.)
                    Beat::Flow(FlowBeat::Label(_)) | Beat::Flow(FlowBeat::GotoScene(_)) => false,
                    beat => {
                        self.happen(beat, stage, focus, calls);
                        false
                    }
                },
            };
            if waits {
                parallel.at.beat -= 1;
                return true;
            }
        }
        true
    }

    /// What a beat of the scene's ways does, wherever it is played (the
    /// scene's own course, a passage meanwhile): waits, ifs, loops, gotos
    /// (`at`: past it, moved where the scene goes on). Whether it waits;
    /// none for any other beat.
    pub(super) fn way_on(&mut self, beat: &Beat, at: &mut Mark, stage: &Stage) -> Option<bool> {
        let hurrying = self.hurrying();
        Some(match beat {
            Beat::Flow(FlowBeat::WaitCam) => stage.view.busy() && !hurrying,
            Beat::Flow(FlowBeat::WaitFade) => stage.light_busy() && !hurrying,
            Beat::Flow(FlowBeat::WaitBubbles) => !stage.words.bubbles.is_empty() && !hurrying,
            Beat::Flow(FlowBeat::WaitUntil(condition)) => !self.holds(condition) && !hurrying,
            Beat::Flow(FlowBeat::WaitAll) => stage.busy() && !hurrying,
            Beat::Flow(FlowBeat::WaitFor(who)) => (stage.cast.get(who).is_some_and(|actor| actor.busy()) || stage.tracked(&NodeRef::Who(*who))) && !hurrying,
            Beat::Flow(FlowBeat::WaitTrack(node)) => stage.tracked(node) && !hurrying,
            Beat::Flow(FlowBeat::WaitMoves(who)) => !stage.cast.get(who).is_some_and(|actor| actor.busy()) && !hurrying,
            Beat::Flow(FlowBeat::WaitNear { who, at: there, radius }) => {
                stage.cast.get(who).is_some_and(|actor| actor.position.distance(there.vec2()) > *radius) && !hurrying
            }
            Beat::Flow(FlowBeat::If(test)) => {
                if !self.holds(test) {
                    at.beat = self.next_way(*at);
                }
                false
            }
            // (Reached from the way that held: past the others.)
            Beat::Flow(FlowBeat::ElseIf(_)) | Beat::Flow(FlowBeat::Else) => {
                at.beat = self.past_if(*at);
                false
            }
            Beat::Flow(FlowBeat::EndIf) => false,
            Beat::Flow(FlowBeat::Goto(label)) => {
                if let Some(past) = self.past_label(*at, label) {
                    at.beat = past;
                }
                false
            }
            // (Played at once up to a label, a loop waiting for the player,
            // who plays nothing meanwhile, is left after so many rounds: as
            // if done.)
            Beat::Flow(FlowBeat::While(condition)) => {
                let stuck = hurrying && {
                    self.schedule.hurried_rounds += 1;
                    self.schedule.hurried_rounds > HURRIED_ROUNDS
                };
                if stuck || !self.holds(condition) {
                    self.schedule.hurried_rounds = 0;
                    at.beat = self.past_loop(*at);
                }
                false
            }
            // (Back to its `While`, to be asked again.)
            Beat::Flow(FlowBeat::EndWhile) | Beat::Flow(FlowBeat::Continue) => {
                if let Some(start) = self.loop_start(*at) {
                    at.beat = start;
                }
                false
            }
            Beat::Flow(FlowBeat::Break) => {
                at.beat = self.past_loop(*at);
                false
            }
            _ => return None,
        })
    }

    /// Past the `EndWhile` of the `While` just played (`at`: after it).
    pub(super) fn past_loop(&self, at: Mark) -> usize {
        self.ways(at).end_of_loop(at.beat.saturating_sub(1)).map_or(self.beats_long(at), |end| end + 1)
    }

    /// The `While` of the `EndWhile` just played (`at`: after it).
    pub(super) fn loop_start(&self, at: Mark) -> Option<usize> {
        self.ways(at).start_of_loop(at.beat.checked_sub(1)?)
    }

    /// Where the scene at a mark goes on past an `If` (or an `ElseIf`)
    /// that does not hold: into the next way that does (an `ElseIf` that
    /// holds, the `Else`), else past the `EndIf`.
    pub(super) fn next_way(&self, at: Mark) -> usize {
        let beats = &self.events[at.event].scenes[at.scene].beats;
        let ways = self.ways(at);
        let mut part = at.beat.checked_sub(1).and_then(|here| ways.next_part(here));
        while let Some(place) = part {
            match &beats[place] {
                Beat::Flow(FlowBeat::ElseIf(test)) if !self.holds(test) => part = ways.next_part(place),
                _ => return place + 1,
            }
        }
        beats.len()
    }

    /// Where the scene at a mark goes on once the way that held is played
    /// (at its `ElseIf` or `Else`): past the `EndIf`.
    pub(super) fn past_if(&self, at: Mark) -> usize {
        at.beat.checked_sub(1).and_then(|here| self.ways(at).end_if(here)).map_or(self.beats_long(at), |end| end + 1)
    }

    /// Where the way on of an answer to the question at a mark starts (the
    /// first answer's when that one has none): as [`Beat::branch`], but a
    /// way may hold whole scenes, the next answer being some scenes on.
    pub(super) fn branch(&self, at: Mark, chosen: usize) -> Mark {
        let (mut depth, mut first) = (0, None);
        for (scene, one) in self.events[at.event].scenes.iter().enumerate().skip(at.scene) {
            let from = if scene == at.scene { at.beat + 1 } else { 0 };
            for (beat, one) in one.beats.iter().enumerate().skip(from) {
                match one {
                    Beat::Course(Course::Ask(_)) => depth += 1,
                    Beat::Course(Course::End | Course::Rejoin) if depth == 0 => return first.unwrap_or(Mark { beat: at.beat + 1, ..at }),
                    Beat::Course(Course::End | Course::Rejoin) => depth -= 1,
                    Beat::Course(Course::Answer(way)) if depth == 0 => {
                        let way_on = Mark { scene, beat: beat + 1, ..at };
                        if *way as usize == chosen {
                            return way_on;
                        }
                        first = first.or(Some(way_on));
                    }
                    _ => {}
                }
            }
        }
        first.unwrap_or(Mark { beat: at.beat + 1, ..at })
    }

    /// Where the story goes on when a way ends at a mark (an `Answer`, or
    /// the `End` of the last question in it): past the scenes of the other
    /// answers, to the next scene after the end of the question, and of
    /// any question around it whose way ended with it. The game's scripts
    /// sometimes end none: then a question asked in the scene ends there,
    /// and an answer come from further back has the event's end for its own.
    pub(super) fn past_way(&self, at: Mark) -> Mark {
        let scenes = &self.events[at.event].scenes;
        let (mut depth, mut closed, mut answer) = (0, matches!(self.beat(at), Some(Beat::Course(Course::End))), at);
        for (scene, one) in scenes.iter().enumerate().skip(at.scene) {
            let from = if scene == at.scene { at.beat + 1 } else { 0 };
            for (beat, one) in one.beats.iter().enumerate().skip(from) {
                match one {
                    Beat::Course(Course::End) if closed => {}
                    Beat::Course(Course::Answer(_)) if closed => {
                        (closed, answer) = (false, Mark { scene, beat, ..at });
                    }
                    _ if closed => return Mark { event: at.event, scene: scene + 1, beat: 0 },
                    Beat::Course(Course::Ask(_)) => depth += 1,
                    Beat::Course(Course::End | Course::Rejoin) if depth > 0 => depth -= 1,
                    Beat::Course(Course::End) => closed = true,
                    _ => {}
                }
            }
            if closed {
                return Mark { event: at.event, scene: scene + 1, beat: 0 };
            }
        }
        let scene = if self.asked_in_scene(answer) { answer.scene + 1 } else { scenes.len() };
        Mark { event: at.event, scene, beat: 0 }
    }

    /// Whether the question of the answer at a mark was asked in its scene.
    pub(super) fn asked_in_scene(&self, answer: Mark) -> bool {
        let mut depth = 0;
        for beat in self.events[answer.event].scenes[answer.scene].beats[..answer.beat].iter().rev() {
            match beat {
                Beat::Course(Course::End | Course::Rejoin) => depth += 1,
                Beat::Course(Course::Ask(_)) if depth == 0 => return true,
                Beat::Course(Course::Ask(_)) => depth -= 1,
                _ => {}
            }
        }
        false
    }

    /// Past the `endask` that ends the question whose answer's way ends
    /// at a mark (an `Answer`): none when the question ends with `End`.
    pub(super) fn past_choice(&self, at: Mark) -> Option<usize> {
        let beats = &self.events[at.event].scenes[at.scene].beats;
        let mut depth = 0;
        for (place, beat) in beats.iter().enumerate().skip(at.beat + 1) {
            match beat {
                Beat::Course(Course::Ask(_)) => depth += 1,
                Beat::Course(Course::End | Course::Rejoin) if depth > 0 => depth -= 1,
                Beat::Course(Course::Rejoin) => return Some(place + 1),
                Beat::Course(Course::End) => return None,
                _ => {}
            }
        }
        None
    }
}
