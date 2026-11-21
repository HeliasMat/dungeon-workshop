//! The words shown over the scene ([`Words`]): bubbles, a caption, the
//! film bands, the credits, gauges, keys, pictures put about, the words kept
//! on screen, a fight's log, the dialogue box.

use super::*;

/// What is shown over the scene.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Words {
    pub bubbles: Vec<Bubble>,
    pub letterbox: Letterbox,
    pub caption: Option<Caption>,
    /// The credits going up the screen: their lines, the frames left and
    /// those they take in all (see `ScreenBeat::Credits`).
    pub credits: Option<Credits>,
    /// The gauges kept on screen, at the top on the left, in the middle,
    /// on the right (see `PlayBeat::Gauge`): how full, of what colour.
    pub gauges: [Option<Gauge>; 3],
    /// The pictures put about the scene: by their name, their file, their
    /// point (see `ScreenBeat::PlacedPicture`).
    pub pictures: Vec<PlacedPicture>,
    /// The thing shown large over the scene (see `BodyBeat::Large`).
    pub large: Option<Who>,
    /// The words kept on screen as they show now (see `PlayBeat::Hud`).
    pub hud: [Option<String>; 3],
    /// The last lines of a fight's log (see `ScreenBeat::BattleLog`).
    pub log: Vec<LogLine>,
    /// A dungeon's map in a corner, and who is of the team on it (see
    /// `ScreenBeat::Minimap`); the tiles the team has seen, if it shows only
    /// those, by row (none: all of the place).
    pub minimap: Option<Vec<Who>>,
    pub minimap_seen: Option<Vec<Vec<bool>>>,
    /// A line shows in the dialogue box; the box is at the top.
    pub boxed: bool,
    pub box_top: bool,
}

/// The credits going up the screen: their lines, the frames left of those
/// they take.
#[derive(Clone, Debug, PartialEq)]
pub struct Credits {
    pub lines: Vec<String>,
    pub left: f32,
    pub whole: f32,
}

/// A gauge kept on screen: how full (0 to 1), of what colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gauge {
    pub full: f32,
    pub colour: [u8; 3],
}

/// A picture from a file put on a point of the scene, by a name.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedPicture {
    pub name: String,
    pub file: String,
    pub at: Vec2,
}

/// A bubble over someone's head: whose, the frames it still stays, its
/// words and how they are said.
#[derive(Clone, Debug, PartialEq)]
pub struct Bubble {
    pub who: Who,
    pub left: f32,
    pub text: String,
    pub voice: Voice,
}

/// A line of a fight's log: its words, the frames left.
#[derive(Clone, Debug, PartialEq)]
pub struct LogLine {
    pub text: String,
    pub left: f32,
}

impl LogLine {
    /// How many stay on screen.
    pub const HELD: usize = 3;
}

/// Film bands: how far they show (0 to 1), and where they go.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Letterbox {
    pub shown: f32,
    pub to: f32,
}

/// A few words over the scene, and the frames they stay.
#[derive(Clone, Debug, PartialEq)]
pub struct Caption {
    pub left: f32,
    pub text: String,
    /// How many of its letters are written out so far (none: all).
    pub typed: Option<f32>,
}

impl Caption {
    /// What shows of it now.
    pub fn shown(&self) -> String {
        match self.typed {
            Some(letters) => self.text.chars().take(letters as usize).collect(),
            None => self.text.clone(),
        }
    }
}

impl Words {
    /// A frame of the words: bubbles, a fight's log, a caption, the keys
    /// shown for a while and the credits go once their time is up; a
    /// caption is written out; the film bands come and go.
    pub(super) fn pass(&mut self, frames: f32) {
        self.bubbles.retain_mut(|bubble| {
            bubble.left -= frames;
            bubble.left > 0.0
        });
        self.log.retain_mut(|line| {
            line.left -= frames;
            line.left > 0.0
        });
        if let Some(caption) = &mut self.caption {
            caption.left -= frames;
            // (A letter about every two frames.)
            if let Some(letters) = &mut caption.typed {
                *letters += frames * 0.5;
            }
        }
        self.caption = self.caption.take().filter(|caption| caption.left > 0.0);
        let letterbox = &mut self.letterbox;
        letterbox.shown += (letterbox.to - letterbox.shown).clamp(-frames / 20.0, frames / 20.0);
        if let Some(credits) = &mut self.credits {
            credits.left -= frames;
            if credits.left <= 0.0 {
                self.credits = None;
            }
        }
    }
}

impl Stage {
    /// A bubble over someone's head, in the place of the one it had.
    pub fn bubble(&mut self, who: Who, frames: f32, text: String, voice: Voice) {
        self.words.bubbles.retain(|bubble| bubble.who != who);
        self.words.bubbles.push(Bubble { who, left: frames.max(1.0), text, voice });
    }
}
