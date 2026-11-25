//! A scene on screen, for all who play one: the scene as it goes (`Sim`),
//! the pictures it is drawn with, and what it would have heard played. The
//! words on screen are said as `Call`s; who plays it shows them in the one
//! message box, `ui::message`.
//!
//! - `sim`: the scene as it goes, with no window, no sound, no picture;
//! - `cast`: who is who, their names and pictures;
//! - `pose`: which animation someone shows;
//! - `heard`: what is heard of it;
//! - `said`: a line said, as the message box (`ui::message`) shows it;
//! - `draw`: the picture, from the scene as it is: it changes nothing of it.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, sync_channel};

use crate::ui::{Picture, UiArt};
use engine::{Beat, Call, Director, Stage, Who};
use glam::Vec2;
use rules::tables::Tables;

use shell::Keys;
use shell::art::Art;
use shell::images::{Behind, Ground, Sprite, ground_of};
use shell::sound::Hears;

pub mod cast;
mod draw;
pub(crate) use draw::{effects_of, paint_of};
mod heard;
mod pose;
mod said;
mod sim;

pub use cast::Cast;
pub use draw::Clear;
pub use heard::sound_lengths;
pub use sim::{Facts, Heard, Sim};

/// How the first of the numbers of a sprite's own animations follows the
/// five every monster has (see `BodyBeat::Anim`).
pub const SCENE_ANIMATIONS: usize = 5;

/// How many places wait, read, for their scene to come.
const PLACES_AHEAD: usize = 2;

/// What a stage is shown with, held by who plays it.
pub struct Means<'a> {
    pub art: &'a Art,
    pub ui: &'a UiArt,
    pub tables: &'a Tables,
    pub audio: &'a mut Option<Box<dyn Hears>>,
    /// How loud music is (1: all it can; 0: not at all).
    pub music: f32,
    /// How loud sounds are.
    pub sounds: f32,
}

impl Means<'_> {
    /// What the scene reads of the pictures and tables.
    pub fn facts(&self) -> Facts<'_> {
        Facts { art: self.art, tables: self.tables }
    }
}

/// A scene being played: the scene as it goes, and the pictures it is drawn
/// with, read beforehand and kept (none of it changes the scene).
pub struct Play {
    pub sim: Sim,
    /// The place shown, read, and which it is.
    ground: Option<(u16, Ground)>,
    /// The next places of the film, read beforehand.
    coming: Option<Receiver<(u16, Ground)>>,
    /// Animations read on the side before they are shown.
    behind: Behind,
    /// The pictures a scene shows instead of itself, by their level.
    pictures: HashMap<u16, Option<Picture>>,
    /// The pictures of a film's own for when they are shown large.
    large_sprites: HashMap<Who, Option<Rc<Sprite>>>,
}

impl Play {
    pub fn new(director: Director, cast: Cast) -> Self {
        Play { sim: Sim::new(director, cast), ground: None, coming: None, behind: Behind::default(), pictures: HashMap::new(), large_sprites: HashMap::new() }
    }

    /// The places a film goes to, in order, read beforehand on the side: no
    /// scene waits for its place. (But for the first, which is wanted at once.)
    pub fn read_ahead(&mut self, folder: &str, levels: Vec<u16>) {
        let (send, receive) = sync_channel(PLACES_AHEAD);
        let folder = folder.to_string();
        std::thread::spawn(move || {
            for level in levels.into_iter().skip(1) {
                if ground_of(&folder, level).is_some_and(|ground| send.send((level, ground)).is_err()) {
                    break;
                }
            }
        });
        self.coming = Some(receive);
    }

    /// The pictures of the place the scene shows: those already there when a
    /// scene stays in its place, else those read beforehand (those read for
    /// scenes the film went past are left behind), else read now.
    fn place(&mut self, art: &Art) {
        let Some(level) = self.sim.place else {
            self.ground = None;
            return;
        };
        if self.ground.as_ref().is_some_and(|(read, _)| *read == level) {
            return;
        }
        let waiting = self.coming.iter().flat_map(|coming| coming.try_iter());
        let found = waiting.into_iter().find(|(read, _)| *read == level).map(|(_, ground)| ground).or_else(|| art.ground(level));
        self.ground = found.map(|ground| (level, ground));
    }

    /// The place shown, read.
    pub(super) fn ground(&self) -> Option<&Ground> {
        self.ground.as_ref().map(|(_, ground)| ground)
    }

    /// Puts a place on the screen, whatever a scene asked.
    pub fn set_ground(&mut self, art: &Art, level: u16) -> bool {
        self.sim.place = Some(level);
        self.place(art);
        self.ground.is_some()
    }

    /// The size of the place shown, in pixels.
    pub fn place_size(&self) -> Vec2 {
        self.ground().map_or(Vec2::ZERO, |ground| Vec2::new(ground.wide as f32, ground.high as f32))
    }

    /// What comes next: the director plays on, the fight going on with it.
    /// Gives what it asks, and the choice a fight offers, if it does now.
    pub fn direct(&mut self, frames: f32) -> (Vec<Call>, Option<Vec<String>>) {
        self.sim.direct(frames)
    }

    /// A frame of the stage goes by, and what it shows is read; what it
    /// would have heard, played.
    pub fn pass(&mut self, frames: f32, means: &mut Means) {
        self.behind.take();
        self.sim.pass(frames, means.facts());
        self.behind.read(std::mem::take(&mut self.sim.arriving));
        let wanted = self.sim.stage.light.cutaway.iter().flat_map(|cutaway| [Some(cutaway.level), cutaway.before.as_ref().map(|before| before.level)]);
        for level in wanted.flatten().collect::<Vec<u16>>() {
            self.pictures.entry(level).or_insert_with(|| means.art.backdrop(level));
        }
        self.place(means.art);
        if let Some((_, ground)) = &mut self.ground {
            ground.advance(self.sim.clock);
        }
        self.hear(means);
    }

    /// A scene opens (`Call::Opens`): its start is kept; `cleared`, on an
    /// empty stage and a dark screen.
    pub fn opens(&mut self, cleared: bool, means: &mut Means) {
        self.sim.opens(cleared);
        self.place(means.art);
        self.hear(means);
    }

    /// The scene opens on an empty stage and a dark screen.
    pub fn clear(&mut self, means: &mut Means) {
        self.sim.clear();
        self.place(means.art);
        self.hear(means);
    }

    /// Someone comes on the stage, shown as `looks`.
    pub fn show(&mut self, who: Who, looks: Who, means: &mut Means) {
        self.sim.show(who, looks, means.facts());
    }

    /// The line of a fight.
    pub fn fought(&mut self, fields: &[String], means: &Means) {
        self.sim.fought(fields, means.facts());
    }

    /// A beat of the stage only the one showing it can play.
    pub fn beat(&mut self, beat: Beat, means: &mut Means) {
        self.sim.beat(beat, means.facts());
        self.place(means.art);
        self.hear(means);
    }

    /// The song of the scene, as the music goes on after it was skipped to.
    pub fn song_again(&mut self, means: &mut Means) {
        self.sim.song_again();
        self.hear(means);
    }

    /// What the scene would have heard, played as loud as the one playing it
    /// has music and sounds; nothing, with no sound.
    fn hear(&mut self, means: &mut Means) {
        let heard = std::mem::take(&mut self.sim.heard);
        let Some(audio) = means.audio else {
            return;
        };
        for heard in heard {
            match heard {
                Heard::Song { channel, song, level, step } => audio.song(channel, song, level * means.music, step * means.music),
                Heard::SongFrom { channel, song, level, seconds } => audio.song_from(channel, song, level * means.music, seconds),
                Heard::Volume { channel, level } => audio.volume(channel, level * means.music),
                Heard::Sound { id, volume, pitch } => audio.sound(id, volume * means.sounds, pitch),
                Heard::OwnSong { channel, name, level, step, seconds } => audio.own_song(channel, &name, level * means.music, step * means.music, seconds),
                Heard::Play(voice) => {
                    let loud = if matches!(voice.source, shell::sound::Source::Song(_)) { means.music } else { means.sounds };
                    audio.play(shell::sound::Voice { volume: voice.volume * loud, ..voice });
                }
                Heard::FadeVoice { name, level, frames, stop } => audio.fade_voice(&name, level * means.sounds, frames, stop),
                Heard::Hush => audio.hush(),
                Heard::Jingle(id) => audio.jingle(id),
                Heard::Fade { channel, frames } => audio.fade(channel, frames),
                Heard::Speed(speed) => audio.speed(speed),
            }
        }
    }
}

/// How many frames go by in one of a scene: its own pace, or as many as
/// holding Space asks.
pub fn scene_frames(stage: &Stage, hurry: bool) -> f32 {
    const HURRIED: f32 = 4.0;
    if hurry { HURRIED } else { stage.tempo.unwrap_or(1.0) }
}

/// What the keys do while a scene plays: someone walked about. Not while a
/// line waits to be read.
pub fn scene_keys(director: &mut Director, stage: &mut Stage, keys: &Keys) {
    let free = (!director.reading() || director.line_meanwhile()) && director.hold().is_none();
    if !free {
        return;
    }
    if keys.tab {
        stage.swap_control();
    }
    if stage.play.controlled.is_none() && !stage.play.listening {
        return;
    }
    stage.steer(keys.toward, keys.run);
    if keys.enter {
        director.press(stage);
    }
    // (M or X: the menu of a film one plays, see `FlowBeat::OnMenu`.)
    if keys.named.iter().any(|name| name == "m" || name == "x") {
        director.open_menu();
    }
    if stage.play.controlled.is_none() {
        for (pressed, arrow) in [(keys.up, 1), (keys.down, 2), (keys.left, 3), (keys.right, 4)] {
            if pressed {
                director.arrow(arrow);
            }
        }
    }
}
