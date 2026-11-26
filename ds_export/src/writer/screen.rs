//! The screen: the place shown (its level, the top screen), fades and
//! flashes, the light, the weather, captions.

use engine::{Beat, Frames, ScreenBeat, Weather};

use super::Writer;
use crate::notes::{Fate, Kind};
use crate::sky::{DECOR_LEVEL, TOP_SCREENS, light, quoted};

impl Writer<'_> {
    /// A screen beat: the place, a fade, a flash, the light, a caption.
    pub(super) fn screen(&mut self, number: i32, word: Option<&str>, beat: &ScreenBeat) {
        match beat {
            // The water thrown up, its rings, bubbles, a wake: Sky's scripts have no
            // particles of their own (R-DS-LOSS).
            ScreenBeat::Particles { .. } | ScreenBeat::Emitter { .. } | ScreenBeat::EmitterOff(_)
                if matches!(word, Some("swim" | "dive" | "plunge" | "surface" | "get_out" | "burrow" | "pop_out")) => {}
            // An aura of a type (`aura on`): cinders, a zap, a drip, leaves; Sky's
            // scripts have no particles of their own (R-DS-LOSS).
            ScreenBeat::Particles {
                kind: engine::ParticleKind::Cinders | engine::ParticleKind::Zap | engine::ParticleKind::Drip | engine::ParticleKind::Leaves,
                ..
            }
            | ScreenBeat::Emitter { kind: engine::ParticleKind::Cinders, .. } => {}
            ScreenBeat::EmitterOff(_) if word == Some("aura") => {}
            // The engine's evolution (slowed, shining, growing): Sky's white flash instead (`body`, `evolve`).
            ScreenBeat::Particles { .. } if matches!(word, Some("evolve" | "transform")) => {}
            ScreenBeat::Grey(_) if !self.started => {}
            // The light: Sky's screen_FlushOut, evening orange or night blue
            // (the memory's sepia is its own, below).
            // As a scene opens: the light from the start (the screen tinted before it
            // fades in, as seen), unless the place is one of Sky's own evening or
            // night levels (lettered past A: `D01P11B` is the beach at sunset).
            ScreenBeat::Tint { colour, .. } if !self.started => {
                let own = self.level.as_deref().and_then(|level| level.chars().last()).is_some_and(|last| last.is_ascii_alphabetic() && last != 'A');
                self.tint = light(colour).filter(|_| !own);
            }
            ScreenBeat::Tint { colour, frames } if !self.memory => self.light_to(colour, frames),
            ScreenBeat::Tint { .. } => {}
            ScreenBeat::Grey(on) => self.grey(*on),
            ScreenBeat::Backdrop(id, ..) => self.backdrop(number, *id),
            ScreenBeat::Weather(weather) => self.weather(number, *weather),
            ScreenBeat::Fade { out: false, white, frames } => self.fade_in(*white, *frames),
            ScreenBeat::Fade { out: true, white, frames } => {
                if !self.faded_out {
                    let fade = if *white { "screen_WhiteOut" } else { "screen_FadeOut" };
                    self.line(&format!("{fade}(1, {});", frames.0.round() as i32));
                }
                self.faded_out = true;
            }
            // (A wipe: Sky fades.)
            ScreenBeat::Wipe { out, frames, .. } => self.screen(number, word, &ScreenBeat::Fade { out: *out, white: false, frames: *frames }),
            ScreenBeat::Flash { colour, frames } => self.flash(colour, *frames),
            // (The title again, as the engine shows it with each scene: once is enough.)
            ScreenBeat::Caption { text, .. } if self.title.as_ref() == Some(text) => {}
            ScreenBeat::Caption { text, .. } => {
                if self.started {
                    // Said over the scene, as Sky narrates.
                    self.line(&format!("message_Narration(30, \"{}\");", quoted(text)));
                    self.line("message_Close();");
                    self.said_over = true;
                } else if self.title.is_none() {
                    self.title = Some(text.clone());
                }
            }
            ScreenBeat::Banner(text) => {
                self.line(&format!("back_SetTitleBanner(56, \"{}\");", quoted(text)));
            }
            other => self.unexported(number, word, &Beat::Screen(other.clone())),
        }
    }

    /// Sky's memories and visions: the screen in sepia, and back.
    fn grey(&mut self, on: bool) {
        if on && !self.memory {
            self.line("screen_FlushOut(1, 0, 8, 0, 0, 0);");
        } else if !on && self.memory {
            self.line("screen_FlushIn(1, 0, 1, 0, 0, 0);");
            // (The sepia took the light's place: the light again.)
            if let Some(tint) = self.tint {
                self.flush("Out", 0, tint);
            }
        }
        self.memory = on;
    }

    /// The place shown: its level, and the top screen's picture. (A decor of
    /// the Workshop's: a map of its own, `LEVEL_DECOR_<n>` until patch_rom.py
    /// gives it a level.)
    fn backdrop(&mut self, number: i32, id: u16) {
        self.scene_line = number;
        if !self.game.levels.contains_key(&id)
            && let Some(name) = self.game.decors.get(&id).cloned()
        {
            self.note(number, Kind::Decor, Fate::Changed, format!("{name}: a decor of your own, made a map of Sky's"), ::film::ds::DECOR_LOSS);
        }
        let level = self
            .game
            .levels
            .get(&id)
            .cloned()
            .or_else(|| self.game.decors.contains_key(&id).then(|| format!("{}{id}", DECOR_LEVEL.trim_start_matches("LEVEL_"))));
        let Some(level) = level else {
            self.problem(number, Kind::Logic, Fate::Impossible, format!("no level {id} in the game's data"), "left out");
            return;
        };
        self.top = TOP_SCREENS.iter().find(|(place, _)| *place == level).map(|(_, top)| *top);
        self.size = self.game.size(id);
        // (`--tall`: the place itself on the top screen, a screen above, where it is tall enough.)
        // (A place Sky pairs with a picture of its own keeps it: the beach's is its sky and
        // rock, where the place above the sand is only sea, its foam blank on the top screen.)
        self.tall = self.tall_asked && self.top.is_none() && self.size.is_some_and(|(_, high)| high >= 384.0);
        if self.tall_asked && !self.tall {
            let why = if self.top.is_some() { "Sky's own top picture for it kept" } else { "too short for two screens (384 pixels)" };
            self.note(number, Kind::Screen, Fate::Changed, format!("{level}: no tall view"), why);
        }
        if self.tall {
            self.top = None;
            self.line("back2_SetMode(4);");
            self.line(&format!("back2_SetGround(LEVEL_{level});"));
        } else if let Some(top) = self.top {
            self.line("back2_SetMode(4);");
            self.line(&format!("back2_SetGround(LEVEL_{top});"));
        }
        let level = format!("LEVEL_{level}");
        self.line(&format!("back_SetGround({level});"));
        self.line("supervision_Acting(0);");
        self.level = Some(level);
    }

    /// The screen faded in; the first time, the top screen too, and the
    /// light the scene opens with.
    fn fade_in(&mut self, white: bool, frames: Frames) {
        let first = !self.started;
        self.started = true;
        self.faded_out = false;
        let frames = frames.0.round() as i32;
        if first && (self.top.is_some() || self.tall) {
            self.line(&format!("screen2_FadeIn(0, {frames});"));
        }
        if let Some(tint) = self.tint.filter(|_| first) {
            self.flush("Out", 0, tint);
        }
        // (Sky comes back from white with a fade in: it has no WhiteIn of its own.)
        let fade = "screen_FadeIn";
        let _ = white;
        self.line(&format!("{fade}(1, {frames});"));
    }

    /// A flash: Sky's own (white: `screen_WhiteOut` and back; a colour: `screen_FlushOut`).
    fn flash(&mut self, colour: &[f32; 3], frames: Frames) {
        let half = (frames.0 / 2.0).round().max(1.0) as i32;
        if colour.iter().all(|part| *part >= 250.0) {
            self.line("screen_WhiteOut(1, 2);");
            self.line("Wait(2);");
            self.line(&format!("screen_FadeIn(1, {});", frames.0.round() as i32));
        } else {
            let [red, green, blue] = colour.map(|part| part.round() as i32);
            self.line(&format!("screen_FlushOut(1, {half}, 2, {red}, {green}, {blue});"));
            self.line(&format!("Wait({half});"));
            self.line(&format!("screen_FlushIn(1, {half}, 2, {red}, {green}, {blue});"));
        }
    }

    /// The light on the screen. (Not on the top one: `screen2_Flush…` at
    /// the same time fights it, both screens flickering, as seen.)
    pub(super) fn flush(&mut self, way: &str, frames: i32, tint: &str) {
        self.line(&format!("screen_Flush{way}(1, {frames}, {tint});"));
    }

    /// The weather: Sky's own falling effects on a performer (3) over the
    /// whole screen.
    pub(super) fn weather(&mut self, number: i32, weather: Weather) {
        let effect = match weather {
            Weather::Clear => {
                if self.weather {
                    self.line("SetEffect<performer 3>(EFFECT_NONE, 3);");
                }
                return;
            }
            // (Sky's own rain in its scenes: its 16 shows nothing seen over a place.)
            Weather::Rain => 698,
            Weather::Hail | Weather::Snow => 20,
            other => {
                self.note(
                    number,
                    Kind::Weather,
                    Fate::LeftOut,
                    format!("the weather {}: Sky has no such effect", format!("{other:?}").to_lowercase()),
                    "nothing",
                );
                return;
            }
        };
        self.weather = true;
        self.line(&format!("SetEffect<performer 3>({effect}, 3);"));
    }

    /// The light changed in the scene (`screen_FlushOut`, Sky's mode 4), or taken away.
    fn light_to(&mut self, colour: &[f32; 4], frames: &Frames) {
        let frames = if self.started { frames.0.round() as i32 } else { 0 };
        // (Mode 4 multiplies the screen by the colour, as Sky's own warm tint
        // `128, 96, 64` does; the colours are those its evening and night
        // levels of the guild's crossroads have against the day's.)
        let Some(tint) = light(colour) else {
            if let Some(tint) = self.tint.take() {
                self.flush("In", frames, tint);
            }
            return;
        };
        // (The same light again, as after the rain: kept; FlushIn then FlushOut would flash.)
        if self.tint == Some(tint) {
            return;
        }
        if let Some(old) = self.tint.take() {
            self.flush("In", 0, old);
        }
        self.flush("Out", frames, tint);
        self.tint = Some(tint);
    }
}
