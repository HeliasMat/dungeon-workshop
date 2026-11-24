//! What is being made in the Resources space: a picture, a sound, a music,
//! a dungeon or a boss fight, each in the editor of its part.

use crate::sound::SoundMaking;
use workshop_picture::Painting;

/// What is being made.
pub enum Maker {
    Picture(Box<Painting>),
    Sound(Box<SoundMaking>),
    Music(Box<crate::music::MusicMaking>),
    Dungeon(Box<crate::dungeon::DungeonMaking>),
}

impl Maker {
    /// A new one of that kind: "image", "animation", "sound", "music", "dungeon", "fight".
    pub fn new_of(session: &std::sync::Arc<workshop_base::session::Session>, kind: &str) -> Maker {
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
