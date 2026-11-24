//! Scenes written as text, played frame by frame, the same each time.
//! Nothing is drawn here and no game is known: whoever shows them reads the
//! [`Stage`], and plays what the [`Director`] hands over ([`Call`]).
//!
//! Its modules go one way, each using only those below it:
//!
//! - `base`, `who`, `vocabulary`: the values a beat says and the stage
//!   keeps ([`Ease`], [`Frames`], [`Ground`], [`Rect`], [`Colour`]...);
//!   who is on the stage ([`Who`]); the words of a game, which whoever uses
//!   the engine gives it (the engine knows none), and what a scene asks of
//!   that game ([`GameAsk`]);
//! - `problem`, `language`: a line that does not read ([`ParseError`]);
//!   lines kept as fields between tabs (`story.txt`) read ([`read_fields`],
//!   [`read_scene`]); a film comes already as lines;
//! - `props`: the registry of every kind of node and every property, and
//!   the one value grammar ([`props::PROPS`], [`props::read`]);
//! - `beat`: what a scene does ([`Beat`]), by family, read from a line
//!   ([`Beat::parse`]);
//! - `stage`: the stage frame by frame ([`Stage`]): who stands on it, the
//!   view, the light, the words;
//! - `director`: the story's events played beat after beat ([`Director`]);
//! - `rehearsal`: a scene played through without anything shown, for what
//!   goes wrong only when it plays ([`rehearse`]);
//! - `lint`: what is wrong in the lines ([`Checker`]), told as
//!   [`Diagnostic`]s.

mod base;
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

pub use base::{AnimId, Colour, Direction, Ease, EaseFamily, EmoteId, Frames, Ground, MoveId, Rect, SongId, SoundId, TILE, colour_named};
pub use beat::{
    Arg, Beat, BodyBeat, Cam, Compare, Condition, Course, CrowdForm, CrowdShape, DataBeat, Daylight, Exit, FlowBeat, Formation, Gait, GaugeOf, Halo, Key,
    KeyTime, LineEnd, Lit, Miss, MotionBeat, NodeBeat, NodeRef, Num, Pace, ParticleKind, PathShape, Pitch, PlaceBeat, Play, PlayBeat, Played, Point, Pose,
    Role, ScreenBeat, SheetAnim, Side, Sort, SoundBeat, SoundOf, SpeechBeat, Spot, Stacked, Test, TrackMode, Value, ViewBeat, Voice, Weather, commands,
    hud_place, is_command, light_named, note_named, pan_of, particle_kind, particle_names, played_filled, weather_named,
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
    Actor, Body, Bolt, Bubble, Camera, Caption, Credits, Cue, Cutaway, Drawn, Effect, Emotes, Fade, Flash, Gauge, Glance, HEAD, LIFT, Layer, Letterbox, Light,
    LogLine, Mix, Note, Orbit, Pan, Particle, PlacedPicture, Placing, PlayState, Popup, Quake, SHEETS, SHOT_FRAMES, SetError, Shake, Sheet, Spotlight, Stage,
    Swing, Thing, Tint, Train, Tween, Viewport, WALK, Wipe, Words, Zoom, attribute, towards, z_table,
};
pub use vocabulary::{GameAsk, Vocabulary};
pub use who::Who;
