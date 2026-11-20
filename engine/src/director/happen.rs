//! What happens at once.

use super::*;

impl Director {
    /// Plays a beat that happens at once, without waiting or going
    /// elsewhere in the scene: on the stage, on the director's numbers, or
    /// for the game to show.
    pub(super) fn happen(&mut self, beat: Beat, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) {
        let beat = self.drawn(beat);
        let beat = self.filled(beat);
        let beat = self.keys_timed(beat);
        // (A line turns who is painted in some ways only, until it walks or
        // stands as usual again: see `Figure`.)
        match &beat {
            Beat::Body(
                BodyBeat::Turn { who, .. }
                | BodyBeat::Face { who, .. }
                | BodyBeat::LookAt { who, .. }
                | BodyBeat::TurnAround(who)
                | BodyBeat::TurnBy { who, .. }
                | BodyBeat::TurnSlow { who, .. }
                | BodyBeat::TurnAway { who, .. },
            ) => stage.turned_by_a_line(*who, true),
            Beat::Body(BodyBeat::Pose { who, pose: Pose::Idle }) => stage.turned_by_a_line(*who, false),
            _ => {}
        }
        let Some(beat) = stage.direct(beat, focus) else {
            return;
        };
        match beat {
            Beat::Flow(beat) => self.happen_flow(beat, calls),
            Beat::Speech(SpeechBeat::BoxSide { top }) => {
                stage.words.box_top = top;
                (self.manner.box_raised, self.manner.box_kept) = (false, true);
            }
            Beat::Speech(SpeechBeat::BoxAuto) => {
                stage.words.box_top = false;
                (self.manner.box_raised, self.manner.box_kept) = (false, false);
            }
            Beat::Speech(beat) => self.happen_speech(beat, calls),
            Beat::Body(beat) => self.happen_body(beat, stage, calls),
            Beat::Play(beat) => self.happen_play(beat, calls),
            Beat::Place(beat) => self.happen_place(beat, stage, focus, calls),
            Beat::Sound(beat) => self.happen_sound(beat, stage, focus, calls),
            Beat::Data(beat) => self.happen_data(beat, stage),
            Beat::Motion(beat) => self.happen_motion(beat, stage, focus, calls),
            Beat::Screen(beat) => self.happen_screen(beat, stage, calls),
            Beat::View(beat) => self.happen_view(beat, calls),
            // (Played by the stage.)
            Beat::Node(_) => {}
            Beat::Host(fields) => self.host(fields, self.at.unwrap_or_default(), calls),
            Beat::Course(_) => {}
        }
    }

    /// A track's keys timed by the music or by a mark of the scene, as so
    /// many frames from now: when the music will have played so much, so
    /// long after the mark (`mark name`: `_mark_name`, by the scene's
    /// clock). One already past is now; a mark never made, now too.
    pub(super) fn keys_timed(&self, beat: Beat) -> Beat {
        let Beat::Node(NodeBeat::Animate { node, prop, mut keys, mode, relative, exact }) = beat else {
            return beat;
        };
        let var = |name: &str| self.vars.get(name).copied().unwrap_or(0.0) as f32;
        for key in &mut keys {
            let from_now = match &key.at {
                KeyTime::After(_) => continue,
                KeyTime::Music(frames) => frames.0 - var(MUSIC_CLOCK),
                KeyTime::Mark { name, plus } => match self.vars.get(&format!("_mark_{name}")) {
                    Some(mark) => *mark as f32 + plus.0 - var(names::CLOCK),
                    None => 0.0,
                },
            };
            key.at = KeyTime::After(Frames(from_now.max(0.0)));
        }
        Beat::Node(NodeBeat::Animate { node, prop, keys, mode, relative, exact })
    }

    fn happen_flow(&mut self, beat: FlowBeat, calls: &mut Vec<Call>) {
        match beat {
            FlowBeat::After { frames, more, beat } => {
                let frames = frames.0 + self.chance(more.0 as u32 + 1) as f32;
                self.schedule.later.push(Later { left: frames, beat: *beat });
            }
            FlowBeat::Every { frames, more, beat } => {
                let first = frames.0 + self.chance(more.0 as u32 + 1) as f32;
                self.schedule.every.push(Again { left: first, between: frames.0, more: more.0, beat: *beat });
            }
            FlowBeat::EveryOff => self.schedule.every.clear(),
            FlowBeat::Parallel(None) => {
                self.schedule.parallels.clear();
                self.lines_end(|open| open.thread.is_some() && open.end == LineEnd::Next);
            }
            FlowBeat::Line { number, end } => self.line_starts(number, end),
            FlowBeat::Checkpoint(label) => {
                self.checkpoint = Some(label);
                calls.push(Call::Checkpoint);
            }
            FlowBeat::Parallel(Some(label)) => {
                if let Some(at) = self.at
                    && let Some(past) = self.past_label(at, &label)
                {
                    let thread = self.thread_named(&label);
                    self.schedule.parallels.push(Parallel { at: Mark { beat: past, ..at }, pause: 0.0, thread });
                }
            }
            // (What waits or goes elsewhere means nothing here: the course's.)
            beat if beat.steers() => {}
            beat => calls.push(Call::Show(beat.into())),
        }
    }

    fn happen_speech(&mut self, beat: SpeechBeat, calls: &mut Vec<Call>) {
        match beat {
            SpeechBeat::PortraitSide { who, right } => match right {
                true => _ = self.manner.right.insert(who),
                false => _ = self.manner.right.remove(&who),
            },
            SpeechBeat::TextSpeed(speed) => self.manner.text_speed = speed,
            SpeechBeat::AutoFace(on) => self.manner.auto_face = on,
            SpeechBeat::Audience(who) => self.manner.audience = who,
            SpeechBeat::HoldGaze { who, held } => match held {
                true => _ = self.manner.held_gaze.insert(who),
                false => _ = self.manner.held_gaze.remove(&who),
            },
            SpeechBeat::React(on) => self.manner.react = on,
            SpeechBeat::Mood { who, face } => match face {
                Some(face) => _ = self.manner.faces.insert(who, face),
                None => _ = self.manner.faces.remove(&who),
            },
            SpeechBeat::Typing { who, sound } => match sound {
                Some(sound) => _ = self.manner.voices.insert(who, sound.0),
                None => _ = self.manner.voices.remove(&who),
            },
            // (What waits or goes elsewhere means nothing here.)
            SpeechBeat::Said | SpeechBeat::BoxSide { .. } | SpeechBeat::BoxAuto => {}
            beat => calls.push(Call::Show(beat.into())),
        }
    }

    fn happen_body(&mut self, beat: BodyBeat, stage: &mut Stage, calls: &mut Vec<Call>) {
        match beat {
            // (Shown as another: the game shows its picture.)
            BodyBeat::Disguise { who, like } => {
                if let Some(actor) = stage.cast.get_mut(&who) {
                    actor.looks.looks_like = like;
                }
                calls.push(Call::Show(Beat::Body(BodyBeat::Disguise { who, like })));
            }
            BodyBeat::PutNear { who, other, side } => {
                if let Some(there) = stage.cast.get(&other).map(|actor| actor.position) {
                    let at = there + crate::stage::step_of(side.number()).normalize_or_zero() * 26.0;
                    let direction = crate::stage::towards(at, there).unwrap_or(1);
                    stage.put(who, at, direction);
                    calls.push(Call::Show(Beat::Body(BodyBeat::Put { who, at: at.into(), direction: Direction::from_number(direction) })));
                }
            }
            BodyBeat::Put { who, ref at, direction } => {
                stage.put(who, at.vec2(), direction.number());
                calls.push(Call::Show(beat.into()));
            }
            // (Shown as put where the giver stands.)
            BodyBeat::Give { what, from, to, frames, keeps } => {
                let there = stage.cast.contains_key(&what);
                if let Some(start) = stage.give(what, from, to, frames.0, keeps).filter(|_| !(keeps && there)) {
                    calls.push(Call::Show(Beat::Body(BodyBeat::Put { who: what, at: start.into(), direction: Direction::Down })));
                }
            }
            BodyBeat::Gone(who) => stage.remove(who),
            beat => calls.push(Call::Show(beat.into())),
        }
    }

    fn happen_play(&mut self, beat: PlayBeat, calls: &mut Vec<Call>) {
        match beat {
            PlayBeat::Hud { at, text } => self.kept.hud[at.min(2)] = text,
            PlayBeat::Gauge { at, gauge } => self.kept.gauges[at.min(2)] = gauge,
            PlayBeat::AskTimer { frames, default } => self.asking.ask_timer = Some(Timed { left: frames.0.max(1.0), answer: default }),
            beat => calls.push(Call::Show(beat.into())),
        }
    }

    fn happen_sound(&mut self, beat: SoundBeat, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) {
        // (The music's clock: a song starts it again, from where it starts.)
        match &beat {
            SoundBeat::Music(song) | SoundBeat::MusicIn { song, .. } => self.clocks.music.start(song.is_some(), 0.0),
            SoundBeat::MusicFrom { song, seconds } => self.clocks.music.start(song.is_some(), *seconds),
            SoundBeat::MusicBack => self.clocks.music.start(true, 0.0),
            SoundBeat::MusicOwn { seconds, .. } => self.clocks.music.start(true, *seconds),
            SoundBeat::MusicSpeed(speed) => self.clocks.music.speed = *speed,
            _ => {}
        }
        if let Some(played) = self.clocks.music.reading() {
            self.vars.insert(MUSIC_CLOCK.to_string(), played);
        }
        match beat {
            // (Heard from the side of who is hit.)
            SoundBeat::OnHit { who, sound } => {
                let hits = stage.cast.get(&who).map_or(0, |actor| actor.hits);
                let from = Side::From { who, softer: false };
                let sound = match *sound {
                    SoundBeat::Sound(id) => SoundBeat::Play(Played { pan: from, ..Played::of(SoundOf::Game(id)) }),
                    SoundBeat::MoveSound(id) => SoundBeat::Play(Played { pan: from, ..Played::of(SoundOf::Move(id)) }),
                    SoundBeat::Play(played) if played.pan == Side::Middle => SoundBeat::Play(Played { pan: from, ..played }),
                    sound => sound,
                };
                self.schedule.on_hit.push((who, hits, Beat::Sound(sound)));
            }
            // (Softer the further from where the view looks, from his side.)
            SoundBeat::SoundFrom { id, from } => {
                let (volume, pan) = heard_from(stage, from, focus, true);
                calls.push(Call::Show(Beat::Sound(SoundBeat::SoundAt { id, volume, pan })));
            }
            SoundBeat::Play(mut played) => {
                if let Side::From { who, softer } = played.pan {
                    let (volume, pan) = heard_from(stage, who, focus, softer);
                    (played.volume, played.pan) = (played.volume * volume, Side::At(pan));
                }
                calls.push(Call::Show(Beat::Sound(SoundBeat::Play(played))));
            }
            beat => calls.push(Call::Show(beat.into())),
        }
    }

    fn happen_data(&mut self, beat: DataBeat, stage: &mut Stage) {
        match beat {
            DataBeat::SetText { var, text } => {
                let text = self.fill(&text);
                self.texts.insert(var, text);
            }
            DataBeat::Calc { var, a, op, b } => {
                let (a, b) = (self.value(&a), self.value(&b));
                let value = match op.as_str() {
                    // (Decimals: 7 / 2 is 3.5; a division by 0 gives 0.)
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" if b != 0.0 => a / b,
                    "%" if b != 0.0 => a.rem_euclid(b),
                    // (`div`: the whole part of the division, 7 div 2 is 3.)
                    "div" if b != 0.0 => (a / b).trunc(),
                    "/" | "%" | "div" => 0.0,
                    "min" => a.min(b),
                    _ => a.max(b),
                };
                self.vars.insert(var, value);
            }
            DataBeat::Timer { var, frames } => {
                self.vars.insert(var.clone(), frames.map_or(0.0, |frames| (frames.0 / 60.0).ceil() as f64));
                self.clocks.timers.retain(|(other, _)| *other != var);
                if let Some(frames) = frames {
                    self.clocks.timers.push((var, frames.0));
                }
            }
            DataBeat::Set { var, value } => {
                let value = self.value(&value);
                if let Some(lines) = self.author.lines.as_mut().filter(|_| var == super::LINE) {
                    lines.push(value as i32);
                }
                self.vars.insert(var, value);
            }
            DataBeat::Add { var, value } => {
                let value = self.value(&value);
                *self.vars.entry(var).or_insert(0.0) += value;
            }
            DataBeat::Random { var, from, to } => {
                let drawn = from + self.chance((to - from + 1).max(1) as u32) as i32;
                self.vars.insert(var, drawn.into());
            }
            // (Put as any other, where the other stands.)
            DataBeat::Sees { var, who, other, distance, angle } => {
                let seen = stage.sees(who, other, distance, angle);
                self.vars.insert(var, f64::from(u8::from(seen)));
            }
            DataBeat::Facing { x, y, who, far, goer } => {
                if let Some(at) = stage.before(who, far, goer) {
                    self.vars.insert(x, at.x.round() as f64);
                    self.vars.insert(y, at.y.round() as f64);
                }
            }
            DataBeat::Position { who, x, y } => {
                if let Some(at) = stage.cast.get(&who).map(|actor| actor.position) {
                    self.vars.insert(x, at.x.round() as f64);
                    self.vars.insert(y, at.y.round() as f64);
                }
            }
            DataBeat::Measure { var, a, b } => {
                if let (Some(a), Some(b)) = (stage.cast.get(&a), stage.cast.get(&b)) {
                    self.vars.insert(var, a.position.distance(b.position).round() as f64);
                }
            }
        }
    }

    fn happen_motion(&mut self, beat: MotionBeat, stage: &mut Stage, focus: Vec2, calls: &mut Vec<Call>) {
        match beat {
            // (Each put in its place, facing the point.)
            MotionBeat::Ring { who, at, radius } => {
                let count = who.len().max(1) as f32;
                for (place, who) in who.into_iter().enumerate() {
                    let angle = std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * place as f32 / count;
                    let slot = at + Vec2::from_angle(angle) * radius;
                    let direction = crate::stage::towards(slot, at).unwrap_or(1);
                    self.happen(Beat::Body(BodyBeat::Put { who, at: slot.into(), direction: Direction::from_number(direction) }), stage, focus, calls);
                }
            }
            beat => calls.push(Call::Show(beat.into())),
        }
    }

    fn happen_screen(&mut self, beat: ScreenBeat, stage: &mut Stage, calls: &mut Vec<Call>) {
        match beat {
            // (Another place coming over: as a backdrop, everyone staying.)
            ScreenBeat::Crossfade { level, frames } => {
                stage.new_place();
                calls.push(Call::Show(Beat::Screen(ScreenBeat::Crossfade { level, frames })));
            }
            ScreenBeat::Banner(_) => {
                stage.cast.clear();
                stage.emotes.statuses.clear();
                calls.push(Call::Show(beat.into()));
            }
            ScreenBeat::Backdrop(_, cleared, kept) => {
                if cleared {
                    stage.cast.retain(|who, _| Some(*who) == kept);
                    stage.emotes.statuses.retain(|who, _| Some(*who) == kept);
                    stage.light.halos.clear();
                    stage.light.cutaway = None;
                }
                stage.new_place();
                calls.push(Call::Show(beat.into()));
            }
            beat => calls.push(Call::Show(beat.into())),
        }
    }

    fn happen_view(&mut self, beat: ViewBeat, calls: &mut Vec<Call>) {
        match beat {
            ViewBeat::KeepSpeaker(on) => self.manner.keep_speaker = on,
            beat => calls.push(Call::Show(beat.into())),
        }
    }

    /// A line only the game can play, handed over with the numbers it
    /// names as they are now.
    pub(super) fn host(&mut self, mut fields: Vec<String>, from: Mark, calls: &mut Vec<Call>) {
        let word = fields[0].clone();
        if let Some(field) = self.words.host_numbers.iter().find(|(named, _)| *named == word).and_then(|(_, at)| fields.get_mut(*at)) {
            let name = field.replace(['{', '}'], "");
            let (name, sign) = name.strip_prefix('-').map_or((name.as_str(), 1.0), |name| (name, -1.0));
            if name.parse::<i32>().is_err() {
                // (The game counts in whole numbers: money, items.)
                *field = super::numbers::number_text((self.vars.get(name).copied().unwrap_or(0.0) * sign).round());
            }
        }
        if let (Some((_, var)), Some(change)) =
            (self.words.host_counts.iter().find(|(named, _)| *named == word), fields.get(1).and_then(|field| field.parse::<i32>().ok()))
            && let Some(count) = self.vars.get_mut(*var)
        {
            *count = (*count + f64::from(change)).max(0.0);
        }
        // (`target`: whoever Enter found last.)
        if let Some(target) = self.target() {
            for field in fields.iter_mut().filter(|field| *field == "target") {
                *field = target.to_string();
            }
        }
        // (What it asks of the game, typed: handed over as it is, not as a line.)
        match (self.words.host_asks)(&fields) {
            Some(ask) => calls.push(Call::Game { ask, from }),
            None => calls.push(Call::Host { fields, from }),
        }
    }
}

/// How far from where the view looks a sound from someone is still heard.
const HEARD: f32 = 240.0;

/// How loud a sound from someone is (`softer`: the further from where the
/// view looks, the softer), and from which side (`pan_of`).
pub(super) fn heard_from(stage: &Stage, who: Who, focus: Vec2, softer: bool) -> (f32, f32) {
    let Some(actor) = stage.cast.get(&who) else {
        return (1.0, 0.0);
    };
    let volume = if softer { (1.0 - actor.position.distance(focus) / HEARD).clamp(0.15, 1.0) } else { 1.0 };
    (volume, crate::pan_of(actor.position.x - focus.x))
}
