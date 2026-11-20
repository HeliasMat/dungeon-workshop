//! How lines are said.

use super::declare::{Dash, Face, IfAbsent, OnOff, Sound, declare};
use super::parse::Fields;
use super::*;

/// How lines are said.
#[derive(Clone, Debug, PartialEq)]
pub enum SpeechBeat {
    /// A few words in a bubble over someone's head (said, thought or
    /// shouted), for that many frames; the scene goes on meanwhile.
    Bubble { who: Who, frames: Frames, text: String, voice: Voice },
    /// The line said just before is read by here: the scene waits for it.
    Said,
    /// Someone's portrait shown on the right of the box (or on the left
    /// again), as the original has a second speaker's.
    PortraitSide { who: Who, right: bool },
    /// How fast the lines that follow write themselves out: so many times
    /// as fast as usual (none: as usual).
    TextSpeed(Option<f32>),
    /// The dialogue box kept at the top of the screen, or at its bottom.
    BoxSide { top: bool },
    /// The dialogue box where it hides no speaker, as the game has it: at
    /// the foot of the screen, at the top for a line of one standing low.
    BoxAuto,
    /// The face someone's lines show when they give none (none: no
    /// portrait).
    Mood { who: Who, face: Option<u16> },
    /// The sound someone's lines make as they write themselves out (none:
    /// the game's own).
    Typing { who: Who, sound: Option<SoundId> },
    /// From now on (or no more), at each line: its speaker and the one
    /// who spoke before turn to each other; the speaker shows the emote
    /// of its face; the view glides to the speaker.
    AutoFace(bool),
    /// Those listening: at each line they look at who speaks. Empty: nobody.
    Audience(Vec<Who>),
    /// From now on (or no more) lines do not turn that one: it keeps looking
    /// where it looks. Only asked for, it turns.
    HoldGaze { who: Who, held: bool },
    /// Those listening answer the lines' faces, in turn: the same emote,
    /// and a little jump at a surprise or a joy.
    React(bool),
}

declare! {
    SpeechBeat;
    hand: "audience", "boxside", "bubble", "portrait", "shout",
        "textspeed", "think";
    "said" => Said {},
    "mood" => Mood { who: Who, face: Dash<Face> },
    "voice" => Typing { who: Who, sound: Dash<Sound> },
    "holdgaze" => HoldGaze { who: Who, held: IfAbsent<OnOff> = true },
    "autoface" => AutoFace(on: IfAbsent<OnOff> = true),
    "react" => React(on: IfAbsent<OnOff> = true),
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new("audience", &["rest...", "off"], "those look at the speaker, at each line"),
    Form::new("autoface", &["[rest...]"], "the speaker and the one before turn towards each other"),
    Form::new(
        "boxside",
        &["rest..."],
        "the dialogue box kept at the top of the screen, its portrait below, or at the bottom; auto: where it hides no speaker, as the game has it (at the foot, at the top for a line of one standing low)",
    ),
    Form::new("bubble", &["who:code frames:word text..."], "a bubble above their head (the scene goes on); `-`: the time to read it"),
    Form::new("holdgaze", &["who:code [rest...]"], "lines no longer turn them, they keep their gaze; `turn` and `lookat` still turn them"),
    Form::new("mood", &["who:code face:word", "who:code -"], "the face of their lines that give none (`-`)"),
    Form::new("portrait", &["who:code rest..."], "their portrait on the right of the box (the one who answers), or on the left"),
    Form::new(
        "react",
        &["[rest...]"],
        "those in the `audience` answer the speaker's faces, one in two and in turn: the same emote, and a little jump at surprise or joy; films do not turn it on",
    ),
    Form::new("said", &["[text...]"], ""),
    Form::new("shout", &["rest..."], "a thought (little bubbles) / a shout (spiky bubble)"),
    Form::new("textspeed", &["times:number", "normal"], "the following lines type so many times faster (`0.4` slowly, `3` fast)"),
    Form::new("think", &["rest..."], "a thought (little bubbles) / a shout (spiky bubble)"),
    Form::new("voice", &["who:code sound:word", "who:code -", "who:code namen:word"], "the sound of their lines as they type"),
];

pub(super) fn read(fields: &Fields) -> Option<SpeechBeat> {
    if let Some(beat) = SpeechBeat::declared(fields) {
        return Some(beat);
    }
    let verb = fields.verb()?;
    Some(match verb {
        "portrait" => SpeechBeat::PortraitSide {
            who: fields.who(1)?,
            right: fields.lookup(2, "right or left", |side| match side {
                "right" => Some(true),
                "left" => Some(false),
                _ => None,
            })?,
        },
        "textspeed" => SpeechBeat::TextSpeed(match fields.word(1) {
            Some("normal") => None,
            _ => Some(fields.real(1)?.clamp(0.1, 8.0)),
        }),
        "boxside" if fields.is(1, "auto") => SpeechBeat::BoxAuto,
        "boxside" => SpeechBeat::BoxSide {
            top: fields.lookup(1, "top or bottom", |side| match side {
                "top" => Some(true),
                "bottom" => Some(false),
                _ => None,
            })?,
        },
        "audience" if fields.is(1, "off") => SpeechBeat::Audience(Vec::new()),
        "audience" => SpeechBeat::Audience(fields.group(1)?),
        "bubble" | "think" | "shout" => {
            let text = fields.named(3, "words")?;
            SpeechBeat::Bubble {
                who: fields.who(1)?,
                // (`-`: long enough to be read, by its length.)
                frames: match fields.word(2) {
                    Some("-") => Frames((40.0 + 4.0 * text.chars().count() as f32).min(360.0)),
                    _ => Frames(fields.real(2)?),
                },
                text: text.replace("\\n", "\n"),
                voice: match verb {
                    "think" => Voice::Thought,
                    "shout" => Voice::Shouted,
                    _ => Voice::Said,
                },
            }
        }
        _ => return None,
    })
}
