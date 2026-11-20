//! Plays the story's events once, at extraction, the way the original
//! does (its scene player, `stage`, with every routine of a scene running
//! side by side), and writes down what is seen: who is put where, who
//! walks where and for how long, who turns, which animation each plays,
//! where the view looks, the fades, the music and the sounds, between the
//! lines said. The game replays that record; nothing of the original's
//! script engine is needed to play it.
//!
//! The record is lines of the story's file (see `extract`), in the order
//! things happen: `t <frames>` lets that many 60ths of a second pass
//! before what follows; `put <who> <x> <y> <direction>` (a way by its name: `down`, `upleft`...) has a character
//! there (`<who>`: its number among the scripts' characters, or `p<n>` for
//! an unseen spot the view can follow); `go <who> <x> <y> <frames>
//! <slides>` walks it there in that time (sliding: without turning to
//! where it goes); `turn <who> <direction>`; `anim <who> <group> <loops>
//! <frozen>` one of its sprite's scene animations; `rise <who> <height>
//! <frames>`; `attr <who> <bits>` how it is drawn; `gone <who>`; `camera
//! <who>`, `camera @ <x> <y>` or `camera -` (the leader); `fade <out>
//! <white> <frames>`; `music <song>` (`-`: none), `ambient <song>` (what
//! is heard around, on a channel of its own), `sound`, `jingle`,
//! `effect <who> <effect>`; `backdrop <level> <cleared> <kept>` (cleared:
//! the characters leave with the old one, but for the one kept, the
//! leader); `scene <level> <name>` marks where a cutscene starts. `fademusic <channel> <frames>` (0 the music, 1 what
//! is heard around, 2 both); `shake <axis> <pixels>` the view (0: still);
//! `banner <name>` a dungeon's name on a dark screen, until the next
//! backdrop; `timed <frames>` after a line said: it goes by itself; `dim
//! <white> <frames> <light>` the screen to a part of its light (of 256).
//! A line said stays on screen a while (the time to read it): what goes on
//! meanwhile is written after it, up to `said`, where the scene waits for
//! the line to be read. `top <level>` (`-`: nothing) and `topcam <x> <y>`
//! are the picture the top screen shows and where it looks. A thing of a
//! scene is a character numbered from 16384 on (16384 + its object).
//! The world map of the top screen: `map show <level>` (the level whose
//! picture it is) and `map hide`; `map mode <n>` (1: the team travels);
//! `map level <n>` how much of it the clouds leave (1 to 8); `map look
//! <x> <y> <slides>` the point its middle shows; `map arrow <x> <y>` (`-`:
//! none); `map mark <x> <y> <blinks>`; `map name <text>` (`-`: none).
//! Leaving for a dungeon that is on the map, the team is first seen on
//! its way there (the original's own routine for it).

use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

use glam::Vec2;

use rom_formats::vm::{self, Cast, Look, Place, Who};

use super::rom_data::{RomData, SceneArt, SpriteArt};

mod play;
pub use play::event;

/// The eight directions as steps, starting facing down and turning
/// counter-clockwise (direction 1 is the first).
const AROUND: [(i32, i32); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];

fn facing_of(direction: u16) -> (i32, i32) {
    AROUND[(direction.max(1) as usize - 1) % 8]
}

fn direction_of(facing: (i32, i32)) -> u16 {
    AROUND.iter().position(|&step| step == facing).unwrap_or(0) as u16 + 1
}

/// The direction closest to a way across the level.
fn facing_towards(delta: Vec2) -> Option<(i32, i32)> {
    let way = delta.try_normalize()? * 1.3;
    Some((way.x.round() as i32, way.y.round() as i32))
}

/// Things are kept as characters are, numbered from here on.
const THINGS: u16 = 0x4000;

fn as_character(who: Who) -> Who {
    match who {
        Who::Object(object) => Who::Actor(THINGS.wrapping_add(object)),
        other => other,
    }
}

/// How a character is written in the record.
fn code(who: Who) -> String {
    match who {
        Who::Actor(entity) => entity.to_string(),
        Who::Performer(number) => format!("p{number}"),
        Who::Object(object) => format!("o{object}"),
    }
}

/// A character or a spot of the scene.
#[derive(Clone)]
struct Body {
    who: Who,
    position: Vec2,
    initial: Vec2,
    facing: (i32, i32),
    path: VecDeque<Vec2>,
    /// Pixels per frame.
    speed: f32,
    slide: bool,
    /// The point of the path its walk there has been written for.
    heading: Option<Vec2>,
    /// The direction to end facing, the frames each step takes, and those
    /// spent on the current one.
    turn: Option<(u16, u16, f32)>,
    height: f32,
    rise: Option<(f32, f32)>,
    attributes: u16,
    /// Frames an animation that ends still lasts.
    animating: f32,
    /// Frames an effect that ends still plays on it.
    effect: f32,
    /// Brought by a cutscene, and leaving with it.
    acting: bool,
    /// The layer of a scene that put it there.
    origin: Option<(Cast, u16)>,
    sprite: Option<Arc<SpriteArt>>,
}

impl Body {
    fn busy(&self) -> bool {
        !self.path.is_empty() || self.turn.is_some() || self.rise.is_some()
    }
}

/// The scene as it stands, and what has been seen of it since it was last
/// read.
#[derive(Clone)]
struct Sim<'a> {
    data: &'a RomData,
    bodies: Vec<Body>,
    leader: Option<Who>,
    fade: f32,
    fade_to: f32,
    fade_rate: f32,
    acting: Option<Rc<SceneArt>>,
    station: Option<Rc<SceneArt>>,
    common: Option<Rc<SceneArt>>,
    level: u16,
    /// The level whose folder holds the scene played.
    scene_level: u16,
    /// A line was read with nothing going on meanwhile (see `Sim::emit`).
    line_open: bool,
    seed: u32,
    /// Frames passed since the last line written.
    waited: u32,
    log: Vec<String>,
    /// The cutscenes played so far.
    seen: Vec<String>,
    /// The world map: whether it shows, the level whose picture it is,
    /// how much of it the clouds leave (0: as the story has it).
    map_shown: bool,
    map: u16,
    map_level: u16,
    /// Of the story's variables, what the map goes by: the dungeon chosen
    /// and how much of the map the story has out of the clouds; and what
    /// a scene sets the latter to, for good.
    chosen: i32,
    clouds: i32,
    keep_clouds: Option<u16>,
    /// The dungeon the team is on its way to over the map.
    travel: Option<u16>,
    /// The view is on the leader (as it starts), not where a scene set it;
    /// who it follows instead.
    on_leader: bool,
    followed: Option<Who>,
    /// The sounds and jingles still heard (jingle, number, frames left),
    /// and the frames the top screen's fade still takes.
    sounds: Vec<(bool, u16, f32)>,
    top_fade: f32,
    /// The frames the music, then what is heard around, still take to
    /// fade away.
    music_fade: [f32; 2],
    /// Lines go by themselves after that many frames (0 or less: they
    /// wait for a key).
    wait_mode: f32,
}

/// The level of the main world map, and the guild's place on it (where
/// the team leaves from).
const MAIN_MAP: u16 = 268;
const GUILD: u16 = 181;

impl Sim<'_> {
    fn emit(&mut self, line: String) {
        // (After a line nothing went on during: what comes next waits for
        // it to be read, as the original closes it first. A question stays
        // under it, a line follows it.)
        if std::mem::take(&mut self.line_open) && !line.starts_with("ask") && !line.starts_with("say") {
            self.log.push("said".to_string());
        }
        if self.waited > 0 {
            self.log.push(format!("t\t{}", self.waited));
            self.waited = 0;
        }
        self.log.push(line);
    }

    fn body(&self, who: Who) -> Option<&Body> {
        self.bodies.iter().find(|body| body.who == who)
    }

    fn body_mut(&mut self, who: Who) -> Option<&mut Body> {
        self.bodies.iter_mut().find(|body| body.who == who)
    }

    fn place(&self, who: Who, from: Vec2, initial: Vec2, place: Place) -> Option<Vec2> {
        match place {
            Place::At(x, y) => Some(Vec2::new(x, y)),
            Place::By(x, y) => Some(from + Vec2::new(x, y)),
            Place::Actor(entity) if Who::Actor(entity) == who => Some(from),
            Place::Actor(entity) => self.body(Who::Actor(entity)).map(|body| body.position),
            Place::Initial => Some(initial),
        }
    }

    fn look(&self, from: Vec2, facing: (i32, i32), look: Look) -> Option<(i32, i32)> {
        match look {
            Look::Around(eighths) => Some(facing_of((direction_of(facing) - 1 + eighths) % 8 + 1)),
            Look::Direction(direction) => Some(facing_of(direction)),
            Look::Actor(entity) => facing_towards(self.body(Who::Actor(entity))?.position - from),
            Look::Talker => facing_towards(self.body(self.leader?)?.position - from),
        }
    }

    /// Where a place scripts name is on the world map: its map's level
    /// and the point.
    fn map_place(&self, place: u16) -> Option<(u16, Vec2)> {
        let marker = match place {
            vm::MARKER_HERE => GUILD,
            vm::MARKER_CHOSEN => self.chosen.max(0) as u16,
            other => other,
        };
        let (level, x, y) = self.data.map_marker(marker)?;
        Some((level, Vec2::new(x, y)))
    }

    fn map_show(&mut self, map: u16) {
        if !self.map_shown || self.map != map {
            (self.map_shown, self.map) = (true, map);
            self.emit(format!("map\tshow\t{map}"));
        }
        // (As much of it as the story has out of the clouds: said only
        // where the event itself has set it; the game knows it otherwise.)
        if self.map_level == 0 && self.clouds > 0 {
            self.map_level = self.clouds.clamp(1, 8) as u16;
            self.emit(format!("map\tlevel\t{}", self.map_level));
        }
    }

    fn map_act(&mut self, asked: vm::MapAct) {
        use vm::MapAct;
        match asked {
            MapAct::Mode(mode) => self.emit(format!("map\tmode\t{mode}")),
            MapAct::Show(true) => self.map_show(MAIN_MAP),
            MapAct::Show(false) => {
                (self.map_shown, self.map_level) = (false, 0);
                self.emit("map\thide".to_string());
            }
            MapAct::Level { level, kept } => {
                self.map_level = level.clamp(1, 8);
                self.emit(format!("map\tlevel\t{}", self.map_level));
                if kept {
                    (self.keep_clouds, self.clouds) = (Some(level), level as i32);
                }
            }
            MapAct::Camera { place, slides } => {
                if let Some((map, spot)) = self.map_place(place) {
                    let slides = slides && self.map_shown && self.map == map;
                    self.map_show(map);
                    self.emit(format!("map\tlook\t{}\t{}\t{}", spot.x, spot.y, slides as u8));
                }
            }
            MapAct::Arrow(place) => match place.and_then(|place| self.map_place(place)) {
                Some((_, spot)) => self.emit(format!("map\tarrow\t{}\t{}", spot.x, spot.y)),
                None => self.emit("map\tarrow\t-".to_string()),
            },
            MapAct::Mark { place, blinks } => {
                if let Some((_, spot)) = self.map_place(place) {
                    self.emit(format!("map\tmark\t{}\t{}\t{}", spot.x, spot.y, blinks as u8));
                }
            }
            // (Places are dungeons by their numbers, first.)
            MapAct::Name(place) => {
                let dungeon = place.map(|place| if place == vm::MARKER_CHOSEN { self.chosen.max(0) as u16 } else { place });
                let name = dungeon.map_or("-".to_string(), |dungeon| super::text(&self.data.dungeon_name(dungeon)));
                self.emit(format!("map\tname\t{name}"));
            }
        }
    }

    /// A sound starts: it is heard for as long as it lasts.
    fn heard(&mut self, jingle: bool, id: u16) {
        let lasts = self.data.sound_frames(id, jingle);
        self.sounds.retain(|sound| (sound.0, sound.1) != (jingle, id));
        self.sounds.push((jingle, id, lasts));
    }

    /// Puts a layer of a scene's characters and spots in the level.
    fn layer(&mut self, cast: Cast, layer: usize, hidden: bool) {
        /// Of how a character is drawn: not at all.
        const HIDDEN: u16 = 128;
        let origin = Some((cast, layer as u16));
        let data = self.data;
        let scene = match cast {
            Cast::Acting => &mut self.acting,
            Cast::Station => &mut self.station,
            Cast::Common => &mut self.common,
        };
        if scene.is_none() && cast != Cast::Acting {
            let name = if cast == Cast::Common { vm::ENTER } else { "" };
            // (A map of its own may have no folder: `G01P01B2` is shown,
            // the entrance's things are `G01P01B`'s, the scene's own.)
            *scene = data.scene(self.level, name, true).or_else(|| data.scene(self.scene_level, name, true)).map(Rc::new);
        }
        let Some(scene) = scene.clone() else {
            return;
        };
        for &(number, x, y) in scene.performers.get(layer).into_iter().flatten() {
            let who = Who::Performer(number);
            self.put(who, Vec2::new(x as f32, y as f32), 1, false, None);
            if let Some(body) = self.body_mut(who) {
                body.origin = origin;
            }
        }
        // (Things: kept as characters are.)
        for &(object, x, y) in scene.things.get(layer).into_iter().flatten() {
            let sprite = data.prop(object).map(|prop| prop.0);
            let who = Who::Actor(THINGS.wrapping_add(object));
            self.put(who, Vec2::new(x as f32, y as f32), 1, cast == Cast::Acting, sprite);
            if let Some(body) = self.body_mut(who) {
                body.origin = origin;
            }
            if hidden {
                self.emit(format!("attr\t{}\t{HIDDEN}", code(who)));
            }
        }
        for actor in scene.layers.get(layer).into_iter().flatten() {
            let who = Who::Actor(actor.entity);
            self.put(who, Vec2::new(actor.x as f32, actor.y as f32), actor.direction, cast == Cast::Acting, actor.sprite.clone());
            if let Some(body) = self.body_mut(who) {
                body.origin = origin;
                body.attributes = if hidden { HIDDEN } else { 0 };
            }
            if hidden {
                self.emit(format!("attr\t{}\t{HIDDEN}", code(who)));
            }
            if actor.leader {
                self.leader = Some(who);
                // (On the leader as it starts, or once who it followed is
                // gone: the original's view falls back on the leader.)
                let lost = self.followed.is_some_and(|followed| self.body(followed).is_none());
                if self.on_leader || lost {
                    (self.on_leader, self.followed) = (true, None);
                    self.emit(format!("camera\t{}", code(who)));
                }
            }
        }
    }

    fn put(&mut self, who: Who, position: Vec2, direction: u16, acting: bool, sprite: Option<Arc<SpriteArt>>) {
        self.bodies.retain(|body| body.who != who);
        self.bodies.push(Body {
            who,
            position,
            initial: position,
            facing: facing_of(direction),
            path: VecDeque::new(),
            speed: 0.0,
            slide: false,
            heading: None,
            turn: None,
            height: 0.0,
            rise: None,
            attributes: 0,
            animating: 0.0,
            effect: 0.0,
            acting,
            origin: None,
            sprite,
        });
        self.emit(format!("put\t{}\t{}\t{}\t{}", code(who), position.x, position.y, engine::Direction::from_number(direction).name()));
    }

    /// A frame passes: walks, turns, rises, animations, effects, the fade.
    fn advance(&mut self) {
        let mut seen = Vec::new();
        for body in &mut self.bodies {
            if let Some((to, speed)) = body.rise {
                match (to - body.height).abs() <= speed {
                    true => (body.height, body.rise) = (to, None),
                    false => body.height += speed * (to - body.height).signum(),
                }
            }
            if let Some(&next) = body.path.front() {
                let delta = next - body.position;
                if body.heading != Some(next) {
                    body.heading = Some(next);
                    let frames = (delta.length() / body.speed.max(0.01)).ceil().max(1.0);
                    seen.push(format!("go\t{}\t{}\t{}\t{frames}\t{}", code(body.who), next.x, next.y, body.slide as u8));
                }
                if !body.slide
                    && let Some(towards) = facing_towards(delta)
                {
                    body.facing = towards;
                }
                if delta.length() <= body.speed {
                    body.position = next;
                    body.path.pop_front();
                    body.heading = None;
                } else {
                    body.position += delta / delta.length() * body.speed;
                }
            } else if let Some((target, per_step, waited)) = &mut body.turn {
                *waited += 1.0;
                let current = direction_of(body.facing);
                if current == *target {
                    body.turn = None;
                } else if *waited >= *per_step as f32 {
                    *waited = 0.0;
                    // The short way round.
                    let ahead = (*target + 8 - current) % 8;
                    let next = if ahead <= 4 { current % 8 + 1 } else { (current + 6) % 8 + 1 };
                    body.facing = facing_of(next);
                    seen.push(format!("turn\t{}\t{}", code(body.who), engine::Direction::from_number(next).name()));
                }
            }
            body.animating = (body.animating - 1.0).max(0.0);
            if body.effect.is_finite() {
                body.effect = (body.effect - 1.0).max(0.0);
            }
        }
        if self.fade != self.fade_to {
            let step = self.fade_rate.max(0.001);
            self.fade = match (self.fade_to - self.fade).abs() <= step {
                true => self.fade_to,
                false => self.fade + step * (self.fade_to - self.fade).signum(),
            };
        }
        for sound in &mut self.sounds {
            sound.2 -= 1.0;
        }
        self.sounds.retain(|sound| sound.2 > 0.0);
        self.top_fade = (self.top_fade - 1.0).max(0.0);
        self.music_fade = self.music_fade.map(|left| (left - 1.0).max(0.0));
        for line in seen {
            self.emit(line);
        }
        self.waited += 1;
    }
}
