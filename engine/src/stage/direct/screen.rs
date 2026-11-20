//! The beats of the view's light, of the screen and of what the player does,
//! as the stage plays them. (The camera's own are the camera's: see
//! `camera/shots.rs`.)

use super::*;

impl Stage {
    /// A beat of the view: the camera plays its own, the light's are here.
    pub(super) fn direct_view(&mut self, beat: ViewBeat, focus: Vec2) -> Option<ViewBeat> {
        match self.view.direct(beat, focus, &self.cast)? {
            ViewBeat::Tempo(tempo) => self.tempo = ((tempo - 1.0).abs() > 0.01).then_some(tempo),
            ViewBeat::SpotlightAdd(on, radius) => {
                self.light.lights.retain(|lit| lit.on != on);
                self.light.lights.push(Lit { on, radius: radius.max(8.0) });
            }
            ViewBeat::Spotlight(on) => match on {
                Some(Lit { on, radius }) => {
                    (self.light.spotlight.on, self.light.spotlight.radius) = (Some(on), radius.max(8.0));
                    (self.light.spotlight.black, self.light.spotlight.closing) = (false, None);
                }
                None => {
                    self.light.spotlight.on = None;
                    self.light.lights.clear();
                }
            },
            ViewBeat::Iris { open, frames, on } => {
                let on = on.map(Cam::Who).unwrap_or(Cam::at(focus));
                let frames = frames.0.max(1.0);
                let (from, to) = if open { (0.0, WHOLE_SCREEN) } else { (WHOLE_SCREEN, 0.0) };
                // (Where it closes, from the first frame.)
                let at = match on {
                    Cam::At(ref at) => at.vec2(),
                    Cam::Who(who) => self.cast.get(&who).map_or(focus, |actor| actor.position - Vec2::new(0.0, 12.0)),
                };
                self.light.spotlight = Spotlight { on: Some(on), radius: from, at, shown: 1.0, black: true, closing: Some(Tween::new(to, frames)) };
            }
            ViewBeat::Letterbox(on) => self.words.letterbox.to = if on { 1.0 } else { 0.0 },
            other => return Some(other),
        }
        None
    }

    pub(super) fn direct_screen(&mut self, beat: ScreenBeat, _focus: Vec2) -> Option<ScreenBeat> {
        match beat {
            ScreenBeat::Particles { kind, on, count, colour } => {
                let at = match on {
                    Cam::At(at) => Some(at.vec2()),
                    Cam::Who(who) => self.cast.get(&who).map(|actor| match kind {
                        ParticleKind::Rings => actor.position,
                        _ => actor.position - Vec2::new(0.0, 16.0),
                    }),
                };
                for _ in 0..at.map_or(0, |_| count.min(300)) {
                    let particle = self.particle(kind, at.unwrap_or_default(), colour);
                    self.light.particles.push(particle);
                }
            }
            ScreenBeat::Emitter { kind, who, each_second, colour, frames } => {
                self.light.emitters.retain(|emitter| emitter.who != who);
                self.light.emitters.push(Emitter { kind, who, between: 60.0 / each_second, colour, left: frames.map(Frames::get), wait: 0.0 });
            }
            ScreenBeat::EmitterOff(who) => self.light.emitters.retain(|emitter| emitter.who != who),
            ScreenBeat::Minimap { team, discover } => {
                let size = self.place.ground.as_ref().map(|ground| (ground.rows().len(), ground.rows().first().map_or(0, Vec::len)));
                self.words.minimap_seen = size.filter(|_| discover && team.is_some()).map(|(high, wide)| vec![vec![false; wide]; high]);
                self.words.minimap = team;
                self.reveal_map();
            }
            ScreenBeat::Credits { frames, lines } => self.words.credits = Some(Credits { lines, left: frames.0.max(1.0), whole: frames.0.max(1.0) }),
            ScreenBeat::Wipe { out, side, frames } => {
                self.light.wipe = Wipe { to: if out { 1.0 } else { 0.0 }, left: frames.0.max(1.0), side, ..self.light.wipe }
            }
            ScreenBeat::BattleLog { frames, text } => {
                self.words.log.push(LogLine { text, left: frames.0 });
                // (As many as the game's window holds.)
                let over = self.words.log.len().saturating_sub(LogLine::HELD);
                self.words.log.drain(..over);
            }
            ScreenBeat::Cutaway { level, x, y, over, frames } => {
                let at = Vec2::new(x, y);
                match self.light.cutaway.as_mut().filter(|cutaway| cutaway.level == level && frames == Frames(0.0)) {
                    // (The next step of an animation: the picture is there.)
                    Some(cutaway) => (cutaway.at, cutaway.over, cutaway.pan) = (at, over, None),
                    None => {
                        let before = self.light.cutaway.take().filter(|_| frames > Frames(0.0)).map(|old| Box::new(Cutaway { before: None, ..old }));
                        self.light.cutaway = Some(Cutaway { level, at, over, pan: None, before, coming: (frames.0, frames.0.max(1.0)) });
                    }
                }
            }
            ScreenBeat::CutawayPan { x, y, frames } => {
                if let Some(cutaway) = &mut self.light.cutaway {
                    cutaway.pan = Some((Vec2::new(x, y), frames.0.max(1.0)));
                }
            }
            ScreenBeat::CutawayOff => self.light.cutaway = None,
            ScreenBeat::Grey(on) => self.light.grey = on,
            ScreenBeat::Still(on) => self.light.still = on,
            ScreenBeat::Wave { to, frames } => self.light.waving = Some(Tween::new(to, frames.0.max(1.0))),
            ScreenBeat::Lightning(on) => {
                let at = match on {
                    Cam::At(at) => Some(at.vec2()),
                    Cam::Who(who) => self.cast.get(&who).map(|actor| actor.position),
                };
                if let Some(at) = at {
                    let drawn = self.chance(1 << 16);
                    self.light.bolts.push(Bolt { at, left: 14.0, drawn });
                }
            }
            ScreenBeat::Picture { name, file, at } => {
                self.words.pictures.retain(|shown| shown.name != name);
                if let Some(file) = file {
                    self.words.pictures.push(PlacedPicture { name, file, at: at.vec2() });
                }
            }
            ScreenBeat::Halo(None) => self.light.halos.clear(),
            ScreenBeat::Halo(Some(Halo { on, radius, colour })) => {
                self.light.halos.retain(|halo| halo.on != on);
                if radius > 0.0 {
                    self.light.halos.push(Halo { on, radius, colour });
                }
            }
            ScreenBeat::Fade { out, white, frames } => {
                (self.light.fade.to, self.light.fade.speed) = (if out { 1.0 } else { 0.0 }, 1.0 / frames.0.max(1.0));
                if out {
                    self.light.fade.white = white;
                }
            }
            ScreenBeat::Dim { white, frames, to } => {
                let fade = &mut self.light.fade;
                fade.to = 1.0 - (to / 256.0).clamp(0.0, 1.0);
                (fade.speed, fade.white) = (((fade.to - fade.level).abs() / frames.0.max(1.0)).max(0.001), white);
            }
            ScreenBeat::Tint { colour, frames } => self.light.tint = Tint { now: self.light.tint.now, to: colour, left: frames.0.max(1.0) },
            ScreenBeat::Flash { colour, frames } => {
                self.light.flash = Flash { colour: [colour[0], colour[1], colour[2], 255.0], speed: 255.0 / frames.0.max(1.0) }
            }
            ScreenBeat::Weather(kind) => self.light.weather = kind,
            ScreenBeat::Caption { frames, text, typed } => {
                self.words.caption = (!text.is_empty()).then(|| Caption { left: frames.0.max(1.0), text, typed: typed.then_some(0.0) })
            }
            other => return Some(other),
        }
        None
    }

    pub(super) fn direct_play(&mut self, beat: PlayBeat, _focus: Vec2) -> Option<PlayBeat> {
        match beat {
            PlayBeat::Control(who) => (self.play.controlled, self.play.steered) = (who, false),
            // (Walked about, the first of them, unless one of them already
            // is.)
            PlayBeat::ControlTeam(who) => {
                if let Some(first) = who.first().filter(|_| self.play.controlled.is_none_or(|now| !who.contains(&now))) {
                    (self.play.controlled, self.play.steered) = (Some(*first), false);
                }
                self.play.team = who;
            }
            PlayBeat::Listen(on) => self.play.listening = on,
            PlayBeat::Bounds(bounds) => self.play.bounds = bounds,
            PlayBeat::Wall(None) => self.clear_walls(),
            PlayBeat::Wall(Some(wall)) => self.put_wall(wall, None),
            // (A wall of that name in place of the one it had.)
            PlayBeat::NamedWall { name, wall } => {
                self.take_wall(&name);
                if let Some(wall) = wall {
                    self.put_wall(wall, Some(name));
                }
            }
            PlayBeat::Collisions(on) => self.no_collisions = !on,
            other => return Some(other),
        }
        None
    }
}
