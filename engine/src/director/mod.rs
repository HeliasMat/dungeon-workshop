//! The story's director: the events and their scenes read from a story
//! file ([`read_events`]), played beat after beat ([`Director::tick`]).
//! It plays on the [`Stage`] what changes it, waits where a scene waits
//! (time passing, a line to read, someone still walking, a question, a
//! menu) and tells the game, by [`Call`]s, what only the game can do:
//! show who comes on the stage, play sounds, say lines, ask questions,
//! open its menus, go to a dungeon or a place.

use std::collections::{BTreeMap, HashMap, HashSet};

use glam::Vec2;

use crate::beat::*;
use crate::stage::{Gauge, Stage};
use crate::vocabulary::Vocabulary;
use crate::who::Who;

mod beats;
mod events;
mod happen;
mod input;
mod lines;
mod names;
mod numbers;
pub use numbers::number_text;
mod placing;
mod play;
mod reading;
mod speech;
mod story;
#[cfg(test)]
mod tests;
mod ways;

pub use names::MUSIC_CLOCK;
pub(crate) use names::*;
pub use placing::LINE;
pub use reading::*;
pub use speech::*;
pub use story::*;

/// Where the story is: an event, a scene of it, a beat of the scene.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mark {
    pub event: usize,
    pub scene: usize,
    pub beat: usize,
}

/// What holds the story until the game says it is done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// A question is asked (see [`Director::answer`]).
    Question,
    /// The game has a line of its own to play out (see
    /// [`Director::menu_done`]).
    Host,
}

/// What the director asks of the game.
#[derive(Clone, Debug, PartialEq)]
pub enum Call {
    /// The scene at hand opens: an event's `first`; `cleared`, on an
    /// empty stage and a dark screen (the stage is already so: the game
    /// takes away what it showed).
    Opens { first: bool, cleared: bool },
    /// A beat for the game to show or play: who was put on the stage, a
    /// backdrop, music, sounds, a move, a bubble, the top screen... The
    /// stage already has what the beat changes on it.
    Show(Beat),
    /// A line said: on screen until the player reads it (see
    /// [`Director::read`]); the scene goes on meanwhile.
    /// (`right`: the portrait on the right; `speed`: how many times as
    /// fast as usual the line writes itself out.)
    Say { entity: Option<Who>, face: Option<u16>, text: String, name: Option<String>, voice: Option<u16>, right: bool, speed: Option<f32> },
    /// A question and its answers (see [`Director::answer`]).
    Ask(Vec<String>),
    /// The line on screen went by itself.
    LineGone,
    /// A line only the game can play, as it is written, the numbers it
    /// names as they are now (`from`: where the story asks for it, to come
    /// back to). The story goes on meanwhile, unless the line is one of
    /// `Vocabulary::host_holds` (see [`Director::menu_done`]).
    Host { fields: Vec<String>, from: Mark },
    /// What the scene asks of the game that plays it, typed as the beat was
    /// read (see [`GameAsk`]): the story holds if it asks to, until
    /// [`Director::menu_done`].
    Game { ask: GameAsk, from: Mark },
    /// Past the last event: the story is over.
    Over,
    /// The story is to be saved now (see `FlowBeat::Checkpoint`).
    Checkpoint,
    /// The player is to write words, these to begin with (see
    /// [`Director::text_given`]).
    AskText(String),
}

/// A passage of the scene played along with the rest (see
/// `FlowBeat::Parallel`): where it is, the frames it still waits.
#[derive(Clone, Copy, Debug)]
struct Parallel {
    at: Mark,
    pause: f32,
    /// Which passage it is, for the film's lines it plays (`FilmLines::names`).
    thread: u32,
}

/// A line of the film started or ended, on its course (none: the scene's
/// own; else the label of the passage meanwhile): see `FlowBeat::Line`.
#[derive(Clone, Debug, PartialEq)]
pub struct LineMark {
    pub line: i32,
    pub thread: Option<String>,
    pub starts: bool,
}

/// A line of the film started and not ended yet.
#[derive(Clone, Debug)]
struct OpenLine {
    line: i32,
    thread: Option<u32>,
    end: LineEnd,
    left: f32,
    /// Who it waits for was seen on its way; it ends now.
    seen: bool,
    done: bool,
}

/// The film's lines as they play: those open, the passages' labels, the
/// passage playing now.
#[derive(Clone, Debug, Default)]
struct FilmLines {
    open: Vec<OpenLine>,
    names: Vec<String>,
    playing: Option<u32>,
}

/// The questions at hand.
#[derive(Clone, Debug, Default)]
struct Asking {
    /// The time the next question is given, and the answer then taken;
    /// the time left to the one asked (see `PlayBeat::AskTimer`).
    ask_timer: Option<Timed>,
    ask_left: Option<Timed>,
    /// The answers of the question asked that are offered, by their
    /// places among all of them (see `answer_test`).
    offered: Vec<usize>,
    /// A number being chosen: its name, and the numbers offered; the name
    /// the words being written go to.
    number_asked: Option<NumberAsked>,
    text_asked: Option<String>,
}

/// What is to happen other than next.
#[derive(Clone, Debug, Default)]
struct Schedule {
    /// What is to happen later (see `FlowBeat::After`): the frames left,
    /// and the beat.
    later: Vec<Later>,
    /// What happens again and again (see `FlowBeat::Every`): the frames
    /// left before the next time, those between two, and the beat.
    every: Vec<Again>,
    /// Where each `Gosub` not yet returned from goes back to.
    returns: Vec<Mark>,
    /// The passages played meanwhile (see `FlowBeat::Parallel`).
    parallels: Vec<Parallel>,
    /// The rounds of a loop gone through at once (see `HURRIED_ROUNDS`).
    hurried_rounds: u32,
    /// What is heard when someone is next hit: who, how many hits it had
    /// then, the beat (see `SoundBeat::OnHit`).
    on_hit: Vec<(Who, u32, Beat)>,
}

/// The time gone, in frames.
#[derive(Clone, Debug, Default)]
struct Clocks {
    /// Since the story began (for chance), and since the scene at hand
    /// opened (see `CLOCK`).
    clock: f32,
    scene_clock: f32,
    /// The countdowns going, each its number and its frames left (see
    /// `DataBeat::Timer`).
    timers: Vec<(String, f32)>,
    /// How far the music playing now has played (see `MUSIC_CLOCK`).
    music: MusicClock,
}

/// How far the music playing now has played, in frames of the song: from
/// its `music` line (or as far in as `musicfrom` starts it), as fast as
/// `musicspeed` has it play. A new song starts it again.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicClock {
    /// The frames of the song played (none: no music yet, or none any
    /// more).
    pub played: Option<f32>,
    /// Whether a song played and was stopped.
    pub stopped: bool,
    /// How fast the music plays (1: as it is).
    pub speed: f32,
}

impl Default for MusicClock {
    fn default() -> Self {
        MusicClock { played: None, stopped: false, speed: 1.0 }
    }
}

impl MusicClock {
    /// A song starts, that many seconds into it (none: the music stops).
    pub fn start(&mut self, song: bool, seconds: f32) {
        (self.played, self.stopped) = if song { (Some(seconds * 60.0), false) } else { (None, true) };
    }

    /// So many frames of the scene go by.
    pub fn pass(&mut self, frames: f32) {
        if let Some(played) = &mut self.played {
            *played += frames * self.speed;
        }
    }

    /// What the scene's number reads: the frames played; none before any
    /// music; once it stopped, so far that whatever waits for it waits no
    /// more.
    pub fn reading(&self) -> Option<f64> {
        match (self.played, self.stopped) {
            (Some(played), _) => Some(played as f64),
            (None, true) => Some(1e9),
            (None, false) => None,
        }
    }
}

/// How long each sound lasts, in frames, as whoever plays the story knows
/// it (the engine has no sounds of its own: see [`Director::hear_sounds`]).
#[derive(Clone, Default)]
pub struct SoundLengths(Option<std::sync::Arc<LengthOf>>);

/// How long a sound lasts, in frames, if it is known.
type LengthOf = dyn Fn(&SoundOf) -> Option<f32> + Send + Sync;

impl std::fmt::Debug for SoundLengths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() { "SoundLengths(given)" } else { "SoundLengths(none)" })
    }
}

/// The frames waited for a sound whose length is not known.
pub const SOUND_FALLBACK: f32 = 30.0;

/// How lines are said.
#[derive(Clone, Debug, Default)]
pub(crate) struct Manner {
    /// Each speaker's face when a line gives none, and the sound of its
    /// lines.
    faces: HashMap<Who, u16>,
    voices: HashMap<Who, u16>,
    /// Those whose portrait shows on the right; how fast lines write
    /// themselves out.
    right: HashSet<Who>,
    text_speed: Option<f32>,
    /// What is done at each line by itself, and who spoke last.
    auto_face: bool,
    pub(crate) keep_speaker: bool,
    pub(crate) last_speaker: Option<Who>,
    /// The box put at the top for a line by itself, its speaker low on the
    /// screen (not by `box top`): back at the foot once no one is.
    pub(crate) box_raised: bool,
    /// The box kept where the film put it (`boxside top|bottom`), not
    /// raised for anyone (`boxside auto`: raised again).
    pub(crate) box_kept: bool,
    /// Who listens to the lines, whether they answer them, and how many
    /// lines have gone by (they answer in turn).
    audience: Vec<Who>,
    react: bool,
    /// Whom a line does not turn.
    pub(crate) held_gaze: HashSet<Who>,
    lines_heard: usize,
}

/// The frames left to a question, and the answer then taken.
#[derive(Clone, Copy, Debug)]
struct Timed {
    left: f32,
    answer: usize,
}

/// A number being chosen: its name, and the numbers offered.
#[derive(Clone, Debug)]
struct NumberAsked {
    var: String,
    values: Vec<i32>,
}

/// What is to happen later: the frames left, and the beat.
#[derive(Clone, Debug)]
struct Later {
    left: f32,
    beat: Beat,
}

/// What happens again and again: the frames left before the next time,
/// those between two, as many more drawn by chance at most, and the beat.
#[derive(Clone, Debug)]
struct Again {
    left: f32,
    between: f32,
    more: f32,
    beat: Beat,
}

/// An event played now: where it is among the events, and where the story
/// goes back to after it (none: it was over).
#[derive(Clone, Copy, Debug)]
struct Interlude {
    at: usize,
    back: Option<Mark>,
}

/// What a scene keeps by name or on screen.
#[derive(Clone, Debug, Default)]
struct Kept {
    /// The words kept on screen, as written; the gauges, their number,
    /// most and colour.
    hud: [Option<String>; 3],
    gauges: [Option<GaugeOf>; 3],
}

/// What helps whoever writes a scene.
#[derive(Clone, Debug, Default)]
pub struct Author {
    /// No player at the keys (a check, a picture of the film): roaming goes
    /// on at once (see `FlowBeat::Roam`), to the scene after.
    pub nobody_plays: bool,
    /// A game taken up again: its numbers and words, put back once the
    /// place it was saved at is reached (`skip_to`).
    pub restore: Option<(BTreeMap<String, f64>, HashMap<String, String>)>,
    /// With it, where the one roaming stood when it was saved (see
    /// [`Director::saved_at`]).
    pub restore_at: Option<Vec2>,
    /// Its course played a command at a time, so many let go; what else
    /// plays meanwhile goes on.
    pub stepping: bool,
    pub steps: u32,
    /// The last label the scene went past.
    pub last_label: Option<String>,
    /// Each beat played, where it is in the story, once asked for (`Some`);
    /// the game shows them and empties it.
    pub trace: Option<Vec<String>>,
    /// The scene is played at once up to this label, without its waits,
    /// lines or questions (the first answer taken); there everything gets
    /// where it was going (see `Stage::settle`).
    pub skip_to: Option<String>,
    /// The film's lines as they play, once asked for (`Some`), every one of
    /// them, however many a frame; whoever watches empties it.
    pub lines: Option<Vec<i32>>,
    /// Each line of the film starting and ending, on its course, once
    /// asked for (`Some`); whoever watches empties it.
    pub spans: Option<Vec<LineMark>>,
}

/// How near a way out takes: within a circle of that radius, or within a
/// rectangle of those half sides, round its middle (`FlowBeat::Door`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Reach {
    Circle(f32),
    Rect(Vec2),
}

impl Reach {
    /// Whether that far from the middle is in.
    pub(crate) fn holds(self, from_middle: Vec2) -> bool {
        match self {
            Reach::Circle(radius) => from_middle.length() <= radius,
            Reach::Rect(half) => from_middle.x.abs() <= half.x && from_middle.y.abs() <= half.y,
        }
    }

    /// How far it goes each way.
    pub(crate) fn half(self) -> Vec2 {
        match self {
            Reach::Circle(radius) => Vec2::splat(radius),
            Reach::Rect(half) => half,
        }
    }
}

/// What roaming knows of the scene (see `FlowBeat::Roam`): who may be talked
/// to and the label of their passage; the ways out (their scene, the number
/// it comes in by, where, how near); the presses already seen; whether the
/// one roaming has been out of every way since it came (not sent back by the
/// way it came in).
#[derive(Clone, Debug, Default)]
pub(crate) struct Roaming {
    talks: Vec<(Who, String)>,
    /// Each its scene, its arrival, its middle, its reach (a circle's
    /// radius, or a rectangle's half width and half height: `Door::wide`).
    doors: Vec<(String, f32, Vec2, Reach)>,
    /// The menu's passage; the menu key pressed, not yet played.
    menu: Option<String>,
    menu_asked: bool,
    presses: f64,
    /// Gone to a passage (a talk, the menu) and not back yet.
    away: bool,
    armed: bool,
}

/// What the roaming keeps from one scene to the next: where the one roaming
/// stood at the last save (`FlowBeat::SaveGame`), where to put it back once
/// a game saved is taken up again, by which way it came into the scene a
/// way out led to (`FlowBeat::Door`'s `arrive`).
#[derive(Clone, Debug, Default)]
struct Roamed {
    saved_at: Option<Vec2>,
    resume_at: Option<Vec2>,
    arrival: Option<f32>,
}

/// Plays the story's events.
#[derive(Clone, Debug, Default)]
pub struct Director {
    pub(crate) events: Vec<StoryEvent>,
    /// Where the story is (`None`: nowhere, or over).
    pub(crate) at: Option<Mark>,
    /// A line said is on screen.
    pub(crate) reading: bool,
    pub(crate) hold: Option<Hold>,
    /// Scenes are played without their waits (the game's own checks).
    pub(crate) hurried: bool,
    /// The numbers scenes keep by name, for as long as the story is
    /// played (the last answer chosen: `answer`); those named `_name`
    /// for their event only (never saved). Words, likewise.
    pub(crate) vars: BTreeMap<String, f64>,
    pub(crate) texts: HashMap<String, String>,
    /// The label the story was last saved at in the event at hand: a save
    /// keeps it, and reopened the event plays at once up to it.
    pub(crate) checkpoint: Option<String>,
    pub author: Author,
    /// The film's lines open, as they play (see `FlowBeat::Line`).
    film_lines: FilmLines,
    /// The words of the game.
    words: &'static Vocabulary,
    asking: Asking,
    /// While the player roams: who may be talked to, the ways out.
    roam: Roaming,
    /// Where the roaming stands, from one scene to the next (see `Roamed`).
    roamed: Roamed,
    schedule: Schedule,
    clocks: Clocks,
    pub(crate) manner: Manner,
    kept: Kept,
    seed: u32,
    /// An event played now (see [`Director::interlude`]): where it is
    /// among the events, and where the story goes back to after it (none:
    /// it was over).
    interlude: Option<Interlude>,
    /// The event whose own numbers (`_name`) are kept.
    own_numbers_of: Option<usize>,
    /// The line on screen was said by a passage played meanwhile.
    said_meanwhile: bool,
    /// The scene at hand has opened (see [`Call::Opens`]); it opens on a
    /// cleared stage.
    opened: bool,
    wipe: bool,
    /// The frames to let pass before what follows; those after which the
    /// line on screen goes by itself.
    pause: f32,
    timed: Option<f32>,
    /// How long the game's sounds last; the sounds waited for whose length
    /// was not known (see `FlowBeat::WaitSound`).
    sound_lengths: SoundLengths,
    unknown_sounds: Vec<SoundOf>,
}

impl Director {
    /// Whether another story is this one up to where this one stands (its
    /// events, scenes and beats before, and the beat at hand): played on
    /// with the other, nothing played so far would have been other.
    pub fn same_story_so_far(&self, other: &Director) -> bool {
        let Some(Mark { event, scene, beat }) = self.at else {
            return false;
        };
        let (mine, theirs) = (&self.events, &other.events);
        let (Some(mine_now), Some(theirs_now)) = (mine.get(event), theirs.get(event)) else {
            return false;
        };
        let (Some(mine_scene), Some(theirs_scene)) = (mine_now.scenes.get(scene), theirs_now.scenes.get(scene)) else {
            return false;
        };
        let upto = (beat + 1).min(mine_scene.beats.len());
        mine[..event] == theirs[..event]
            && (mine_now.number, &mine_now.name, mine_now.moment) == (theirs_now.number, &theirs_now.name, theirs_now.moment)
            && mine_now.scenes[..scene] == theirs_now.scenes[..scene]
            && (mine_scene.level, &mine_scene.name) == (theirs_scene.level, &theirs_scene.name)
            && theirs_scene.beats.len() >= upto
            && mine_scene.beats[..upto] == theirs_scene.beats[..upto]
    }

    /// The other's story played on from where this one stands (see
    /// [`Director::same_story_so_far`]).
    pub fn take_story(&mut self, other: &Director) {
        self.events = other.events.clone();
    }

    /// The story's events, from the first of that number (or the first).
    pub fn new(events: Vec<StoryEvent>, number: usize, words: &'static Vocabulary) -> Self {
        let first = events.iter().position(|event| event.number == number && !event.scenes.is_empty()).unwrap_or(0);
        let at = (!events.is_empty()).then_some(Mark { event: first, scene: 0, beat: 0 });
        Director { events, at, words, ..Director::default() }
    }

    /// Where the story is (`None`: nowhere, or over).
    pub fn at(&self) -> Option<Mark> {
        self.at
    }

    /// A save made now comes back here, in the middle of the event.
    pub fn keep_place(&mut self) {
        self.checkpoint = self.at.map(place_label);
    }

    /// Puts the story there.
    pub fn go_to(&mut self, at: Option<Mark>) {
        self.at = at;
    }

    pub fn events(&self) -> &[StoryEvent] {
        &self.events
    }

    /// Whether a line said is on screen.
    pub fn reading(&self) -> bool {
        self.reading
    }

    /// What the story waits for.
    pub fn hold(&self) -> Option<Hold> {
        self.hold
    }

    /// Whether the scene goes on without its waits.
    pub fn hurried(&self) -> bool {
        self.hurrying()
    }

    /// Plays scenes without their waits, or as they are written.
    pub fn hurry(&mut self, hurried: bool) {
        self.hurried = hurried;
    }

    /// How long each sound lasts, in frames (none: not known), for `attends
    /// son`: the game's, a move's, a file of our own; given by whoever plays
    /// the story.
    pub fn hear_sounds(&mut self, lengths: impl Fn(&SoundOf) -> Option<f32> + Send + Sync + 'static) {
        self.sound_lengths = SoundLengths(Some(std::sync::Arc::new(lengths)));
    }

    /// The frames a sound lasts, as far as it is known (else
    /// `SOUND_FALLBACK`, and it is kept among `unknown_sounds`).
    pub(crate) fn sound_frames(&mut self, of: &SoundOf) -> f32 {
        match self.sound_lengths.0.as_ref().and_then(|lengths| lengths(of)) {
            Some(frames) => frames.max(1.0),
            None => {
                if !self.unknown_sounds.contains(of) {
                    self.unknown_sounds.push(of.clone());
                }
                SOUND_FALLBACK
            }
        }
    }

    /// The sounds waited for whose length was not known, in the order met.
    pub fn unknown_sounds(&self) -> &[SoundOf] {
        &self.unknown_sounds
    }

    /// The numbers scenes keep by name.
    pub fn numbers(&self) -> &BTreeMap<String, f64> {
        &self.vars
    }

    pub fn numbers_mut(&mut self) -> &mut BTreeMap<String, f64> {
        &mut self.vars
    }

    pub fn set_numbers(&mut self, numbers: BTreeMap<String, f64>) {
        self.vars = numbers;
    }

    pub fn set_number(&mut self, name: String, value: impl Into<f64>) {
        self.vars.insert(name, value.into());
    }

    /// The words scenes keep by name.
    pub fn texts(&self) -> &HashMap<String, String> {
        &self.texts
    }

    pub fn set_texts(&mut self, texts: HashMap<String, String>) {
        self.texts = texts;
    }

    /// The label the story was last saved at in the event at hand.
    pub fn checkpoint(&self) -> Option<&String> {
        self.checkpoint.as_ref()
    }

    /// The name of the scene it plays (empty: none named, or over).
    pub fn scene_name(&self) -> &str {
        self.at.and_then(|at| self.events.get(at.event)?.scenes.get(at.scene)).map_or("", |scene| scene.name.as_str())
    }

    /// What roaming knows, to be shown (the debug view): who may be talked
    /// to; the ways out, each its scene, its middle and how far it reaches
    /// each way (a circle's: its radius both ways).
    pub fn roaming_known(&self) -> (Vec<Who>, Vec<(String, Vec2, Vec2)>) {
        let talks = self.roam.talks.iter().map(|(who, _)| *who).collect();
        let doors = self.roam.doors.iter().map(|(scene, _, at, reach)| (scene.clone(), *at, reach.half())).collect();
        (talks, doors)
    }

    /// The beat at a mark.
    pub fn beat(&self, at: Mark) -> Option<&Beat> {
        self.events.get(at.event)?.scenes.get(at.scene)?.beats.get(at.beat)
    }

    /// The events played so far (all once the story is over).
    pub fn played(&self) -> &[StoryEvent] {
        let upto = self.at.map_or(self.events.len(), |at| at.event).min(self.events.len());
        &self.events[..upto]
    }

    /// The moment the story is at: where the last event played left it.
    pub fn moment(&self) -> (i32, i32) {
        self.played().last().map_or((0, 0), |event| event.moment)
    }

    /// Whether the line on screen goes by itself.
    pub fn line_timed(&self) -> bool {
        self.timed.is_some()
    }

    /// Whether the line on screen was said by a passage played meanwhile:
    /// it goes by itself, and the player goes on walking about meanwhile.
    pub fn line_meanwhile(&self) -> bool {
        self.reading && self.said_meanwhile
    }

    /// Whether the scene goes on without its waits: hurried, or skipped up
    /// to a label.
    fn hurrying(&self) -> bool {
        self.hurried || self.author.skip_to.is_some()
    }
}
