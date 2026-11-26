//! What the Film space's Assets panel hands to the Scene and the Script: a
//! Pokémon, a face, a gesture, a song, a sound, a thing, a place, a move's
//! effect. Dragged, it is egui's drag-and-drop payload (`egui::DragAndDrop`);
//! double-clicked, it is written at the Script's cursor. The Scene and the
//! Script each say how it is written (`workshop_film`, `workshop_script`).

/// One thing of the Assets panel, as a film writes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asset {
    /// A Pokémon, by its word (`pikachu`).
    Pokemon { word: String },
    /// A face of one (`pikachu`, `happy`): `pikachu (happy): …`.
    Face { who: String, face: String },
    /// A gesture of one: the words after their name (`sit`, `pose look_up`, `anim Attack`).
    Gesture { who: String, words: String },
    /// A song, by its word (`treasure_town`).
    Music { word: String },
    /// A sound, by its word.
    Sound { word: String },
    /// A thing (`Apple`, `torch`): held by someone (`who hold`), else put down (`put object`).
    Thing { word: String, held: bool },
    /// A place, by its word (`beach`, a decor's name).
    Place { word: String },
    /// A move, by its name (`Thunderbolt`): `who use Thunderbolt on …`.
    Effect { name: String },
}

impl Asset {
    /// What it is, in a few words (beside the pointer while it is dragged).
    pub fn said(&self) -> String {
        match self {
            Asset::Pokemon { word } => word.clone(),
            Asset::Face { who, face } => format!("{who} ({face})"),
            Asset::Gesture { who, words } => format!("{who} {words}"),
            Asset::Music { word } => format!("music {word}"),
            Asset::Sound { word } => format!("sound {word}"),
            Asset::Thing { word, held: true } => format!("hold {word}"),
            Asset::Thing { word, held: false } => format!("put object {word}"),
            Asset::Place { word } => format!("scene: {word}"),
            Asset::Effect { name } => format!("use {name}"),
        }
    }
}
