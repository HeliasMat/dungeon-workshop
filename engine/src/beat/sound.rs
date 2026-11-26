//! What is heard.

use super::declare::{Song, declare};
use super::parse::Fields;
use super::*;

/// What is heard.
#[derive(Clone, Debug, PartialEq)]
pub enum SoundBeat {
    Music(Option<SongId>),
    /// A song coming in softly, over so many frames (see `SoundBeat::Music`).
    MusicIn {
        song: Option<SongId>,
        frames: Frames,
    },
    /// The music played before the one now comes back.
    MusicBack,
    /// A channel of music fades away (0 the music, 1 what is heard
    /// around, 2 both).
    MusicFade {
        channel: u8,
        frames: Frames,
    },
    /// How fast the music plays (1: as it is; a song faster, slower).
    MusicSpeed(f32),
    /// What is heard around (the sea...), on a channel of its own.
    Ambient(Option<SongId>),
    Sound(SoundId),
    /// A sound heard from someone: the further from the view, the softer
    /// (see `SoundBeat::SoundAt`).
    SoundFrom {
        id: SoundId,
        from: Who,
    },
    /// A sound played at that volume (1: as usual) and from that side
    /// (`Side`), for the game.
    SoundAt {
        id: SoundId,
        volume: f32,
        pan: f32,
    },
    Jingle(u16),
    /// What a move sounds like, alone.
    MoveSound(MoveId),
    /// A sound heard the next time someone is hit.
    OnHit {
        who: Who,
        sound: Box<SoundBeat>,
    },
    /// A song from that far into it, in seconds.
    MusicFrom {
        song: Option<SongId>,
        seconds: f32,
    },
    /// How loud the music is (1: as it is).
    Volume(f32),
    /// A note: a sound played higher or lower by so many half tones.
    Note {
        pitch: i32,
        sound: SoundId,
    },
    /// A sound played as said: how loud, from where, how high, looped,
    /// by a name it is stopped or faded by (see `Played`).
    Play(Played),
    /// A sound or an ambiance playing, by its name, goes to that volume over
    /// so many frames; `stop`: gone once there.
    Fade {
        name: String,
        level: f32,
        frames: Frames,
        stop: bool,
    },
    /// A file of our own as the music (see `SoundOf::Own`), coming in over
    /// so many frames, from that far into it, in seconds.
    MusicOwn {
        name: String,
        frames: Frames,
        seconds: f32,
    },
}

/// Which sound is heard: one of the game's, what a move sounds like, a song
/// of the game (an ambiance), or a file of our own, by its name (whoever
/// plays the story finds it: `library/sounds/<name>.wav`).
#[derive(Clone, Debug, PartialEq)]
pub enum SoundOf {
    Game(SoundId),
    Move(MoveId),
    Song(SongId),
    Own(String),
}

impl SoundOf {
    /// As a line has it: a number (one of the game's sounds), `move:<n>`,
    /// `song:<n>`, `own:<name>`.
    pub fn read(field: &str, words: &Vocabulary) -> Option<SoundOf> {
        let (kind, said) = field.split_once(':').unwrap_or(("", field));
        Some(match kind {
            "move" => SoundOf::Move(MoveId(said.parse().ok()?)),
            "song" => SoundOf::Song(SongId(said.parse().ok().or_else(|| words.song(said))?)),
            "own" if !said.is_empty() => SoundOf::Own(said.to_string()),
            "" => SoundOf::Game(SoundId(said.parse().ok().or_else(|| words.sound(said))?)),
            _ => return None,
        })
    }
}

impl std::fmt::Display for SoundOf {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SoundOf::Game(id) => write!(out, "{id}"),
            SoundOf::Move(id) => write!(out, "move:{id}"),
            SoundOf::Song(id) => write!(out, "song:{id}"),
            SoundOf::Own(name) => write!(out, "own:{name}"),
        }
    }
}

/// Where a sound is heard from, left to right.
#[derive(Clone, Debug, PartialEq)]
pub enum Side {
    Middle,
    /// -1 all to the left, 1 all to the right.
    At(f32),
    /// From where someone is on the screen; `softer`: softer too the further
    /// he is from where the view looks (as `SoundBeat::SoundFrom`).
    From {
        who: Who,
        softer: bool,
    },
}

/// A sound played as said.
#[derive(Clone, Debug, PartialEq)]
pub struct Played {
    pub of: SoundOf,
    /// 1: as loud as it is.
    pub volume: f32,
    pub pan: Side,
    /// Higher (or lower, below 0) by so many half tones: faster (slower) too.
    pub pitch: i32,
    /// Again and again, until it is stopped (`SoundBeat::Fade`).
    pub looped: bool,
    /// The name it is stopped or faded by.
    pub name: Option<String>,
    /// The frames it comes in over, from silence (0: at once).
    pub rise: Frames,
}

impl Played {
    /// A sound as it is: once, in the middle, unnamed.
    pub fn of(of: SoundOf) -> Played {
        Played { of, volume: 1.0, pan: Side::Middle, pitch: 0, looped: false, name: None, rise: Frames::ZERO }
    }

    /// How much faster it plays for its pitch.
    pub fn speed(&self) -> f32 {
        2.0_f32.powf(self.pitch as f32 / 12.0)
    }
}

/// Half the width of the screen, in pixels: someone that far from where the
/// view looks is heard all from that side (see `pan_of`).
const HALF_SCREEN: f32 = 128.0;
/// How far to one side a sound heard from someone goes at most: never all
/// in one ear.
const SIDE_MOST: f32 = 0.8;

/// Where a sound from someone so many pixels right of where the view looks
/// is heard, left (-1) to right (1).
pub fn pan_of(right: f32) -> f32 {
    (right / HALF_SCREEN).clamp(-1.0, 1.0) * SIDE_MOST
}

/// A sound as written after its `playsound`: `[volume v] [pan p] [from
/// <who> [near]] [pitch n] [loop] [as <name>] [rise <time>]`.
fn played(fields: &Fields) -> Option<Played> {
    let words = fields.words;
    let mut played = Played::of(fields.lookup(1, "a sound (its name, move:, song:, own:)", |sound| SoundOf::read(sound, words))?);
    let mut at = 2;
    while let Some(word) = fields.has(at).then(|| fields.named(at, "volume, pan, from, pitch, as, rise or loop")) {
        let value = at + 1;
        at += 2;
        match word? {
            "volume" => played.volume = fields.real(value)?.clamp(0.0, 4.0),
            "pan" => played.pan = Side::At(fields.real(value)?.clamp(-1.0, 1.0)),
            "from" => {
                let near = fields.is(at, "near");
                played.pan = Side::From { who: fields.who(value)?, softer: !near };
                at += usize::from(near);
            }
            "pitch" => played.pitch = fields.whole(value)?,
            "as" => played.name = Some(fields.named(value, "a name")?.to_string()),
            "rise" => played.rise = Frames(fields.frames(value)? as f32),
            "loop" => {
                played.looped = true;
                at -= 1;
            }
            _ => return None,
        }
    }
    Some(played)
}

declare! {
    SoundBeat;
    hand: "fadesound", "music", "musicfrom", "musicspeed", "note", "onhit",
        "playsound", "sound", "volume";
    "ambient" => Ambient(song: Song),
    "jingle" => Jingle(id: u16),
    "movesound" => MoveSound(id: MoveId),
    "fademusic" => MusicFade { channel: u8, frames: Frames },
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new("ambient", &["namen:word"], "an ambient sound"),
    Form::new("fademusic", &["v:word frames:duration"], "fades out the music, the ambience, or both"),
    Form::new(
        "fadesound",
        &["name:word volume:word time:duration [stop]"],
        "a named sound or ambience (`as`) goes to this volume during this time; `stop`: it stops once there (`fadesound rain 0 2s stop`)",
    ),
    Form::new("jingle", &["n:number"], "a sound effect (by its name, below, or its number) / a little tune"),
    Form::new("movesound", &["move:word"], "the sound of a move, alone"),
    Form::new("music", &["namen:word", "name:word time:duration", "back", "rest..."], "the music (`-`: silence)"),
    Form::new("musicfrom", &["namen:word time:duration"], "the music, from this moment of the song"),
    Form::new("musicspeed", &["percent:word"], "the music faster or slower (`120`: a fifth faster, higher; `100`: as it is)"),
    Form::new(
        "note",
        &["note:word [2] [sound]"],
        "a note: the sound (the click by default) played higher, by so many semitones (`c2`: the octave above; or a number of semitones)",
    ),
    Form::new("onhit", &["who:code sound:word rest..."], "this sound, at the next blow who takes"),
    Form::new(
        "playsound",
        &["sound:word [volume v:number] [pan p:number] [from who:code [near]] [pitch n:number] [loop] [as name:word] [rise time:duration]"],
        "a sound played the way you say. `<sound>`: a game sound (name or number), `move:<n>` (a move's), `song:<n>` (a game song, for an ambience), `own:<name>` (a file of ours, `library/sounds/<name>.wav`). `volume` (1: as is, up to 4), `pan` (-1 all the way left, 1 all the way right), `from <who>` (from the side they are on screen, quieter the further they are; `near`: without fading), `pitch` (semitones, higher and faster), `loop` (endless, until its `fadesound … stop`), `as <name>` (its name, to change or stop it; another of the same name stops), `rise` (it rises from silence during this time). As many as you like at the same time",
    ),
    Form::new("sound", &["namen:word", "namen:word who:code"], "a sound effect (by its name, below, or its number) / a little tune"),
    Form::new("volume", &["v:word to rest..."], "the music's volume"),
];

pub(super) fn read(fields: &Fields) -> Option<SoundBeat> {
    if let Some(beat) = SoundBeat::declared(fields) {
        return Some(beat);
    }
    let words = fields.words;
    // (A song, a sound by its number or its name.)
    let song = |at: usize| match fields.word(at) {
        Some(name) if name.parse::<u16>().is_err() => words.song(name),
        _ => fields.number(at),
    };
    let sound = |at: usize| match fields.word(at) {
        Some(name) if name.parse::<u16>().is_err() => fields.lookup(at, "a sound", |name| words.sound(name)),
        _ => fields.number(at),
    };
    let verb = fields.verb()?;
    Some(match verb {
        "playsound" => SoundBeat::Play(played(fields)?),
        // (`fadesound <name> <level> <time> [stop]`.)
        "fadesound" => SoundBeat::Fade {
            name: fields.named(1, "a name")?.to_string(),
            level: fields.real(2)?.clamp(0.0, 4.0),
            frames: Frames(fields.frames(3)? as f32),
            stop: match fields.word(4) {
                None if !fields.has(4) => false,
                Some("stop") => true,
                _ => return None,
            },
        },
        "music" | "musicfrom" if fields.word(1).is_some_and(|name| name.starts_with("own:")) => SoundBeat::MusicOwn {
            name: fields.word(1)?.strip_prefix("own:").filter(|name| !name.is_empty())?.to_string(),
            frames: Frames(if verb == "music" { (if fields.has(2) { fields.frames(2)? } else { 0 }) as f32 } else { 0.0 }),
            seconds: if verb == "musicfrom" { fields.frames(2)? as f32 / 60.0 } else { 0.0 },
        },
        "music" if fields.is(1, "back") => SoundBeat::MusicBack,
        // (`music <song> <time>`: coming in softly.)
        "music" if fields.has(2) && fields.word(2).is_none_or(|time| frames_of(time).is_some()) => {
            SoundBeat::MusicIn { song: fields.has(1).then(|| song(1)).flatten().map(SongId), frames: Frames(fields.frames(2)? as f32) }
        }
        "music" => SoundBeat::Music(fields.has(1).then(|| song(1)).flatten().map(SongId)),
        // (`note <c|d|e|f|g|a|b>[octave] [sound]`, or half tones.)
        "note" => SoundBeat::Note {
            pitch: fields.lookup(1, "a note (c, d, e… or half tones)", note_named)?,
            sound: SoundId(fields.has(2).then(|| sound(2)).flatten().unwrap_or(0x3F02)),
        },
        // (In hundredths: `musicspeed 120`, a fifth faster.)
        "musicspeed" => SoundBeat::MusicSpeed((fields.real(1)? / 100.0).clamp(0.25, 4.0)),
        // (`sound <name> <who>`: heard from someone.)
        "sound" if fields.len() > 2 => SoundBeat::SoundFrom { id: SoundId(sound(1)?), from: fields.who(2)? },
        "sound" => SoundBeat::Sound(SoundId(sound(1)?)),
        "musicfrom" => SoundBeat::MusicFrom { song: fields.has(1).then(|| song(1)).flatten().map(SongId), seconds: fields.frames(2)? as f32 / 60.0 },
        "volume" => SoundBeat::Volume(fields.real(1)?.clamp(0.0, 1.0)),
        // (`onhit <who> sound <name>`, `onhit <who> movesound <move>`.)
        "onhit" => SoundBeat::OnHit {
            who: fields.who(1)?,
            sound: match Beat::read(&fields.from(2)?)? {
                Beat::Sound(sound) => Box::new(sound),
                _ => return None,
            },
        },
        _ => return None,
    })
}
