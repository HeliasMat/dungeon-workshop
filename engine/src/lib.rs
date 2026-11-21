//! Scenes written as text, played frame by frame, the same each time.
//! Nothing is drawn here and no game is known: whoever shows them reads the
//! [`Stage`], and plays what the [`Director`] hands over ([`Call`]).
//!
//! - `language`: lines kept as fields between tabs (`story.txt`) read
//!   ([`read_fields`], [`read_scene`]); a film comes already as lines;
//! - `lint`: what is wrong in them ([`Checker`]), told as [`Diagnostic`]s;
//! - `rehearsal`: a scene played through without anything shown, for what
//!   goes wrong only when it plays ([`rehearse`]);
//! - `beat`: what a scene does ([`Beat`]), by family, read from a line
//!   ([`Beat::parse`]);
//! - `director`: the story's events played beat after beat ([`Director`]);
//! - `stage`: the stage frame by frame ([`Stage`]): who stands on it, the
//!   view, the light, the words;
//! - `props`: the registry of every kind of node and every property, and
//!   the one value grammar ([`props::PROPS`], [`props::read`]);
//! - `who`: who is on the stage ([`Who`]);
//! - `vocabulary`: the words of a game, which whoever uses the engine gives
//!   it: the engine knows none.

#![forbid(unsafe_code)]

mod beat;
mod director;
mod language;
mod lint;
mod problem;
pub mod props;
mod rehearsal;
mod stage;
mod vocabulary;
mod who;

pub use beat::{
    AnimId, Arg, Beat, BodyBeat, Cam, Colour, Compare, Condition, Course, CrowdForm, CrowdShape, DataBeat, Daylight, Direction, EmoteId, Exit, FlowBeat,
    Formation, Frames, Gait, GameAsk, GaugeOf, Key, KeyTime, LineEnd, Miss, MotionBeat, MoveId, NodeBeat, NodeRef, Num, Pace, ParticleKind, PathShape, Pitch,
    PlaceBeat, Play, PlayBeat, Played, Point, Pose, Role, ScreenBeat, SheetAnim, Side, SongId, Sort, SoundBeat, SoundId, SoundOf, SpeechBeat, Spot, Stacked,
    Test, TrackMode, Value, ViewBeat, Voice, Weather, colour_named, commands, hud_place, is_command, light_named, note_named, pan_of, particle_kind,
    particle_names, played_filled, weather_named,
};
pub use beat::{Form, form_of, forms};
pub use diagnostics::{Diagnostic, Mention, Said, Severity};
pub use director::{
    Author, Call, Director, Hold, LINE, LineMark, MUSIC_CLOCK, Mark, MusicClock, SOUND_FALLBACK, Scene, StoryEvent, emphasis, names_in, number_text,
    read_events,
};
pub use director::{LETTERS_A_FRAME, SHORTEST_LINE, least_shown, letters, line_frames, read_after, units, writing_frames};
pub use language::{Lines, Script, read_fields, read_scene, speaker};
pub use lint::Checker;
pub use problem::{ParseError, codes};
pub use rehearsal::{Rehearsal, Seen, Watch, rehearse};
pub use stage::framed;
pub use stage::node;
pub use stage::tracks;
pub use stage::{
    Actor, Bolt, Bubble, Camera, Caption, Credits, Cue, Cutaway, Drawn, Ease, EaseFamily, Effect, Emotes, Fade, Flash, Gauge, Glance, Ground, HEAD, Halo,
    LIFT, Layer, Letterbox, Light, Lit, LogLine, Mix, Note, Orbit, Pan, Particle, PlacedPicture, Placing, PlayState, Popup, Quake, Rect, SHEETS, SHOT_FRAMES,
    SetError, Shake, Sheet, Spotlight, Stage, Swing, TILE, Thing, Tint, Train, Tween, Viewport, WALK, Wipe, Words, Zoom, attribute, towards, z_table,
};
pub use vocabulary::Vocabulary;
pub use who::Who;
