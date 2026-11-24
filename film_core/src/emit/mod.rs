//! The beats a word makes, typed (`engine::Beat`): what the film
//! hands the engine. A file by the kind of beat, as the engine has them
//! (`engine/src/beat/`): what a word makes, by a constructor with typed
//! values (`emit::jump(who, height, frames)`), handed over by
//! `film.beat(..)`; and the film's own side of it, by `Film` (someone by the
//! name the film gives, a wait at the film's pace, a sound by its name).
//!
//! A point found as the scene plays is a `Point` of `Num::Var`s
//! (`film.on_point`, `emit::found_at`); what is wrong in a line is told by
//! the film (`film.not_a`, `film.unread`), never handed to the engine for it
//! to say. The engine's commands a film writes itself (`quake 3 0.67s`, `>`)
//! are made from their words typed by their form (`command.rs`), by the
//! engine's typed reading of a command (`Beat::build`), never from text.

use scenes::vocabulary::POKEMON;
use std::fmt::Display;

use engine::{Beat, Exit, Gait, Num, Point, Who};

use crate::film::{Film, Sortie, Staging};

mod body;
mod command;
mod course;
mod data;
mod flow;
mod motion;
mod node;
mod place;
mod play;
mod screen;
mod sound;
mod speech;
mod view;

pub use body::{
    among, anim, anim_times, below, blink, bounce, carry, drawing, emote, face, fall, fidget, fishline, flip, float, give, gone, held_step, hop, hop_point,
    in_front, jump, large, look_at, look_at_point, mimic, move_effect, move_shot, on_top, opacity, popup, pose, put, put_near, put_point, recoil, recolour,
    reset, ride, rise, scale, shadow, shine, sink, spin, status, status_icon, stop, takeable, thing_name, throw, tremble, turn, uses_move, views,
};
pub use course::{answer, ask, host, rejoin, said, say};
pub use data::{add_to, calc, number, position, set, var};
pub use flow::{
    after, back, film_line, frames_read, goto, label, otherwise, parallel, wait, wait_all, wait_cam, wait_fade, wait_for, wait_near, wait_reached, wait_sound,
    wait_until,
};
pub use motion::{approach, chase, follow, go, go_point, orbit, orbit_around, orbit_point, spacing, speed, step, train, walk, walk_point, wander};
pub use node::{animate, attach, detach, follow_path, make, set_prop, stop_tracks, unmake, wait_track};
pub use place::{
    advance, all_look, all_look_way, arrives, crowd, entrances, exit, find, flies, flock, gathers, ghosts, goes, goes_by, ground, hovers, lands, leaves,
    name_spot, roams, round, seats, shot, spread, swims,
};
pub use play::{collisions, gauge, hud, named_wall};
pub use screen::{
    backdrop, battle_log, caption, credits, cutaway, cutaway_off, cutaway_pan, emitter, emitter_off, fade, flash, grey, halo, halo_off, lightning, minimap,
    particles, picture, picture_point, still, tint, wave, weather, wipe,
};
pub use sound::{NOTE_SOUND, jingle, move_sound, music, music_fade, music_from, note, on_hit, sound};
pub use speech::{audience, auto_face, box_auto, box_side, bubble, hold_gaze, shout, text_speed};
pub use view::{box_room, camera, ease, frame, frame_with, glance, iris, keep_frame, keep_speaker, pan, quake, shake, spotlight, spotlight_add, spotlight_off};

// (The names of the game a word writes in its code are typed:
// `Colour::Green`, `ParticleKind::Dust`, `Direction::Down`,
// `emote::SWEAT`, `noise::CLICK`, so one the game has not is a mistake the
// compiler finds. A name a film gives is looked up by the word, which says
// when it is none.)

/// A colour of the game, as a popup has it.
pub fn colour(colour: engine::Colour) -> [u8; 3] {
    colour.bytes()
}

/// A flash's colour, as the flash has it.
pub fn light(colour: engine::Colour) -> [f32; 3] {
    colour.rgb()
}

/// The light of a time of day, as a tint has it.
pub fn light_of(light: engine::Daylight) -> [f32; 4] {
    light.tint()
}

/// An animation of the game by its name (`Sit`, `Attack`), if it has one.
pub fn anim_of(name: &str) -> Option<engine::AnimId> {
    POKEMON.animation(name).and_then(|group| u16::try_from(group).ok()).map(engine::AnimId)
}

impl Film {
    /// A beat for the engine, of the film's line at hand.
    pub fn beat(&mut self, beat: Beat) {
        // (A beat the engine could not write as a line, made of what a line
        // gave: told, never handed over.)
        if beat.fields(&POKEMON).is_none() {
            self.wrong(self.number, format!("this line asks for what the engine cannot play ({beat:?})"));
            return;
        }
        self.plan.saw(&beat);
        self.written.push((self.number, Sortie::Beat(beat)));
    }

    /// A beat for each one a word names: someone by the name the film gives
    /// or a code, a crowd as each of its members (`krabbys`), the beat made
    /// for each by `beat`. A word that is no one is told as wrong.
    pub fn on(&mut self, word: &str, beat: impl Fn(Who) -> Beat) {
        for one in self.everyone(word) {
            match self.someone(&one) {
                Some(who) => self.beat(beat(who)),
                None => self.wrong(self.number, format!("\"{one}\" is no one of the scene")),
            }
        }
    }

    /// Someone a beat names besides who it is for (`give` its giver and
    /// taker, `face a b` its b), by the name the film gives or a code. None
    /// for a crowd or no one.
    pub fn other(&self, word: &str) -> Option<Who> {
        self.someone(word)
    }

    /// Who the engine is to move for a word of the film's lines: a name
    /// given (`cast`), or a code the engine reads (`m25`, `0`, `target`).
    /// None for a crowd's name, which is all of them (`film.everyone`), or
    /// a word that is no one.
    pub fn someone(&self, word: &str) -> Option<Who> {
        if self.groups.contains_key(word) {
            return None;
        }
        match self.actors.get(word) {
            Some(code) => Who::parse(code),
            None => Who::parse(word),
        }
    }

    /// Those a word names: a crowd or a band (`krabbys`, `team`) as each
    /// of its members, a group of groups as all of theirs; anyone else as
    /// themselves. By the names the film gives them.
    pub fn everyone(&self, word: &str) -> Vec<String> {
        fn members(film: &Film, word: &str, depth: usize, into: &mut Vec<String>) {
            match film.groups.get(word).filter(|_| depth < 8) {
                Some(group) => group.iter().for_each(|member| members(film, member, depth + 1, into)),
                None => into.push(word.to_string()),
            }
        }
        let mut all = Vec::new();
        members(self, word, 0, &mut all);
        all
    }

    /// Words said with someone's name in them (`{name:pikachu}`), as the
    /// game reads them: by their code (`{name:m25}`), the game putting in
    /// the name.
    pub fn names_in(&self, text: &str) -> String {
        let mut text = text.to_string();
        if !text.contains("{name:") {
            return text;
        }
        for (name, code) in &self.actors {
            let mark = format!("{{name:{name}}}");
            if text.contains(&mark) {
                text = text.replace(&mark, &format!("{{name:{code}}}"));
            }
        }
        text
    }
}

/// A point as a word writes it, each number as written or one found as the
/// scene plays (`{_place3_x}`); none if either is neither.
pub fn point(x: &str, y: &str) -> Option<Point> {
    Some(Point { x: Num::read(x)?, y: Num::read(y)? })
}

/// The point a spot found as the scene plays gives (`emit::find`): its two
/// numbers by their names, `<var>_x`, `<var>_y`.
pub fn found_at(var: &str) -> Point {
    Point { x: Num::Var(format!("{var}_x")), y: Num::Var(format!("{var}_y")) }
}

impl Film {
    /// A word of a line that is not what it should be (`a duration`, `a sound
    /// of the game`), told as wrong.
    pub fn not_a(&mut self, word: impl Display, what: &str) {
        self.wrong(self.number, format!("\"{word}\" is not {what}"));
    }

    /// What a line says that no beat can be made of, told as wrong.
    pub fn unread(&mut self, what: impl Display) {
        self.wrong(self.number, format!("\"{what}\" does not read"));
    }

    /// Someone else a line names, one of the scene by the name the film
    /// gives or a code; a crowd or no one: told as wrong.
    pub fn one(&mut self, word: &str) -> Option<Who> {
        let who = self.someone(word);
        if who.is_none() {
            self.not_a(word, "a Pokémon of the scene");
        }
        who
    }

    /// A point a word writes (`emit::point`); one that is none: told.
    pub fn point_at(&mut self, x: impl Display, y: impl Display) -> Option<Point> {
        let (x, y) = (x.to_string(), y.to_string());
        let at = point(&x, &y);
        if at.is_none() {
            self.not_a(format!("{x} {y}"), "a point");
        }
        at
    }

    /// A beat at a point a word writes, for each one a word names.
    pub fn on_point(&mut self, who: &str, x: impl Display, y: impl Display, beat: impl Fn(Who, Point) -> Beat) {
        if let Some(at) = self.point_at(x, y) {
            self.on_at(who, at, beat);
        }
    }

    /// A beat at a point, for each one a word names.
    pub fn on_at(&mut self, who: &str, at: Point, beat: impl Fn(Who, Point) -> Beat) {
        self.on(who, |who| beat(who, at.clone()));
    }

    /// Someone put at a point a word writes, looking down.
    pub fn put_at(&mut self, who: &str, x: impl Display, y: impl Display) {
        if let Some(at) = self.point_at(x, y) {
            self.put_point_at(who, at);
        }
    }

    /// Someone put at a point, looking down.
    pub fn put_point_at(&mut self, who: &str, at: Point) {
        self.on_at(who, at, |who, at| put_point(who, at, engine::Direction::Down));
    }

    /// How someone goes, as a word writes it (`walk` or `run`, then its
    /// pace: `-`, `x1.5`, `s3`); one that is none: told.
    pub fn gait_of(&mut self, gait: &str, pace: &str) -> Option<Gait> {
        let read = Gait::parse(gait, pace);
        if read.is_none() {
            self.not_a(pace, "a pace");
        }
        read
    }

    /// Someone walks in to a point (see `emit::arrives`), by that way if
    /// told.
    pub fn arrives(&mut self, who: &str, gait: &str, pace: &str, to: Point, placing: bool, from: Option<String>) {
        let Some(gait) = self.gait_of(gait, pace) else { return };
        let from = from.as_deref().map(Exit::parse);
        self.on_at(who, to, |who, to| arrives(who, to, gait, placing, from.clone()));
    }

    /// Someone goes by points a word writes, two numbers each, round the
    /// walls and those standing there.
    pub fn goes_by(&mut self, who: &str, gait: &str, pace: &str, points: &[String]) {
        let read: Option<Vec<Point>> = points.chunks(2).map(|pair| point(pair.first()?, pair.get(1)?)).collect();
        match read.filter(|read| !read.is_empty()) {
            Some(read) => self.goes_through(who, gait, pace, read),
            None => self.not_a(points.join(" "), "a run of points"),
        }
    }

    /// Someone goes by these points, round the walls and those standing
    /// there.
    pub fn goes_through(&mut self, who: &str, gait: &str, pace: &str, points: Vec<Point>) {
        let Some(gait) = self.gait_of(gait, pace) else { return };
        self.on(who, |who| goes_by(who, points.clone(), gait));
    }

    /// Words told centred in the box, then the box closes (`narrate`).
    pub fn narrate(&mut self, text: &str) {
        let centred = format!("[CN]{}", text.trim_start().replace("\\n", "\\n[CN]"));
        let text = self.names_in(&centred).replace("\\n", "\n");
        self.beat(say(None, None, text, None, None));
        self.beat(said());
    }
}

impl Staging {
    /// The light of the scene's hour, as `daytime` last laid it.
    pub fn daylight(&self) -> [f32; 4] {
        self.daylight.unwrap_or_else(|| light_of(engine::Daylight::Normal))
    }
}

impl Film {
    /// The film waits for someone to get where it goes (each of a crowd),
    /// if the line says so; else the line lasts as long, beside the film.
    pub fn wait_for(&mut self, who: &str, wait: bool) {
        if wait {
            self.on(who, wait_for);
        } else if let Some(code) = self.everyone(who).first().and_then(|one| self.someone(one)) {
            self.plan.handle(crate::time::Length::Event { handle: crate::time::Handle::Who(code.to_string()), at_least: 0 });
        }
    }

    /// The film waits that many frames (`30`, `1.5s`, `6f`), at its pace,
    /// if the line says so (else the line lasts as long, beside the film);
    /// a time it cannot read, kept for the engine to tell.
    pub fn wait_frames(&mut self, frames: impl ToString, wait: bool) {
        let frames = frames.to_string();
        if !wait {
            if let Some(read) = crate::words::frames_of(&frames) {
                self.plan.handle(crate::time::Length::Known(read));
            }
            return;
        }
        match crate::words::frames_of(&frames) {
            Some(read) => self.wait(i64::from(read)),
            None => self.not_a(&frames, "a duration"),
        }
    }
}
