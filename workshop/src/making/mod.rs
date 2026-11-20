//! Making a resource in the Resources space, as the best tools of their
//! kind make them.
//!
//! **Pictures and animations**, painted as in a pixel-art studio:
//! - layers (seen or not, how opaque, moved, doubled, merged down), each with
//!   a picture for each step of the animation (a cel);
//! - tools: pencil (its size, pixel-perfect strokes), eraser, fill (what
//!   touches, or all of a colour), picker, line, rectangle and ellipse
//!   (outlined or filled), selection, move, shade (darker, lighter), dither;
//!   mirrors on both axes; a primary and a secondary colour (left and right
//!   buttons, X swaps them), the colours used last, a palette of one's own,
//!   the game's;
//! - a selection: cut, copied, pasted (a picture dropped on the window too),
//!   moved, flipped, turned, cleared; painting stays inside it;
//! - the canvas: zoomed around the pointer, moved (middle button, or Space
//!   held), its grid, the steps before and after seen through (onion skin),
//!   the brush shown where it would paint;
//! - the steps: a strip of them under the canvas, each its time, played
//!   round or back and forth, faster or slower; seen at their true size;
//! - every change undone and redone by its name;
//! - written as the things drawn are (`name.png`, `name.4.png`: four steps
//!   side by side), its layers kept beside to be painted again.
//!
//! **Sounds**, made as the best sound makers of games make theirs (sfxr,
//! Bfxr, ChipTone), then mixed as a sound editor would:
//! - voices (up to four, mixed): each its synth, when it comes in, how loud,
//!   from which side, muted or alone;
//! - a voice: a wave (square and its duty, saw, sine, triangle, white, pink
//!   and 8-bit noises) and its harmonics, an envelope (attack, sustain and its
//!   punch, decay: dragged by its handles on the wave too), a pitch that
//!   slides and slides faster down to a floor, a vibrato, an arpeggio, a
//!   repeat, low-pass (resonant) and high-pass filters that sweep, a
//!   flanger, a coarser grain;
//! - the mix: an echo, a room (reverb), faster or slower, backwards, faded in
//!   and out, made as loud as it may;
//! - made by kind (coin, laser, explosion…), by chance, a little changed,
//!   the groups locked kept; variants written at once (`step_1`, `step_2`…);
//! - seen: its wave (left and right), the voice's pitch, its spectrum; how
//!   loud (peak, mean, clipping); a sound of the game to compare;
//! - every version kept, undone, redone; heard once or round and round, the
//!   playhead on its wave; written where the films find their own sounds
//!   (`sound name`).
//!
//! Each in its folder: `picture/` (the painting, its tools, panels and
//! files), `sound/` (the synth, its voices and mix, the instruments music
//! plays with, the editor).

mod picture;
mod sound;

pub(crate) use picture::fitted;
pub use picture::{Painting, paint};
pub(crate) use sound::{
    Biquad, INSTRUMENTS, Mix, Note, OwnSounds, Sound, SoundMaking, drum_kit, instrument, make_sound, mix_editor, mix_read, mix_written, mixed, note_audio,
    one_shot, open_audio, piano_roll, sample_of,
};

/// What is being made.
pub enum Maker {
    Picture(Box<Painting>),
    Sound(Box<SoundMaking>),
    Music(Box<crate::music::MusicMaking>),
    Dungeon(Box<crate::dungeon::DungeonMaking>),
}

impl Maker {
    /// A new one of that kind: "image", "animation", "sound", "music", "dungeon", "fight".
    pub fn new_of(session: &std::sync::Arc<crate::session::Session>, kind: &str) -> Maker {
        match kind {
            "animation" => Maker::Picture(Box::new(Painting::new(session, "new_animation", 32, 32, 4))),
            "sound" => Maker::Sound(Box::new(SoundMaking::new(session, "new_sound"))),
            "music" => Maker::Music(Box::new(crate::music::MusicMaking::new(session, "new_music"))),
            "dungeon" => Maker::Dungeon(Box::new(crate::dungeon::DungeonMaking::new(session, "new_dungeon"))),
            _ if kind.starts_with("dungeon:") => Maker::Dungeon(Box::new(crate::dungeon::DungeonMaking::open(session, &kind["dungeon:".len()..]))),
            "fight" => Maker::Dungeon(Box::new(crate::dungeon::DungeonMaking::new_fight(session, "new_fight"))),
            _ if kind.starts_with("fight:") => Maker::Dungeon(Box::new(crate::dungeon::DungeonMaking::open_fight(session, &kind["fight:".len()..]))),
            _ => Maker::Picture(Box::new(Painting::new(session, "new_image", 32, 32, 1))),
        }
    }

    /// Its word (as it will be written).
    pub fn word(&self) -> &str {
        match self {
            Maker::Picture(painting) => &painting.word,
            Maker::Sound(making) => &making.word,
            Maker::Music(music) => &music.word,
            Maker::Dungeon(dungeon) => &dungeon.word,
        }
    }

    /// What it is: an image, an animation (several steps), a sound.
    pub fn kind(&self) -> &'static str {
        match self {
            Maker::Picture(painting) if painting.count() > 1 => "Animation",
            Maker::Picture(_) => "Image",
            Maker::Sound(_) => "Sound",
            Maker::Music(_) => "Music",
            Maker::Dungeon(dungeon) if dungeon.fight => "Fight",
            Maker::Dungeon(_) => "Dungeon",
        }
    }

    /// Written as it is now.
    pub fn saved(&self) -> bool {
        match self {
            Maker::Picture(painting) => painting.saved,
            Maker::Sound(making) => making.saved,
            Maker::Music(music) => music.saved,
            Maker::Dungeon(dungeon) => dungeon.saved,
        }
    }
}

/// What making asks of the Resources: a file written (to be read again and chosen), or done.
pub enum Done {
    Picture(String),
    Sound(String),
    Closed,
    /// A boss fight to open in its editor (none named: a new one).
    Fight(String),
}

/// A little random number maker (the same seed, the same numbers).
struct Chance(u32);

impl Chance {
    fn new(seed: u32) -> Chance {
        Chance(seed.max(1))
    }

    /// From 0 to 1.
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next()
    }
}

fn now_seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |since| since.subsec_nanos() ^ since.as_secs() as u32) | 1
}
