//! The words of the game the scenes are for: its faces, songs, sounds,
//! animations, statuses, and the commands only it can play. The engine
//! reads no game's words: whoever uses it gives it its own (see `POKEMON` in
//! `rules`), a value made of tables; a table left empty is words a
//! scene cannot use.

/// A list of names and what each stands for.
type Named<T> = &'static [(&'static str, T)];

#[derive(Debug)]
pub struct Vocabulary {
    /// The lines only the game can play: at once, or as the story's course
    /// (once the line on screen is read). The engine hands them over as
    /// they are written (see `Call::Host`); those it holds the story for,
    /// until [`crate::Director::menu_done`].
    pub host_now: &'static [&'static str],
    pub host_course: &'static [&'static str],
    pub host_holds: &'static [&'static str],
    /// The field of a host line that may name a number, handed over as its
    /// value: `getmoney price`, `getitem 109 count`.
    pub host_numbers: Named<usize>,
    /// The host line that changes a number at once, for what follows it in
    /// the same frame to read it: `getmoney` the money.
    pub host_counts: Named<&'static str>,
    /// What a line of the game's own asks of it, typed as it plays (its
    /// numbers filled): `getmoney 50` the money, `getitem 109 2` an item.
    /// The engine hands it over as it is (`Call::Game`), not as a line.
    pub host_asks: fn(&[String]) -> Option<crate::beat::GameAsk>,
    /// What is offered last when an item is asked for, and how the items
    /// offered are written.
    pub no_item: &'static str,
    pub item_name: (&'static str, &'static str),
    /// How the numbers telling how many of an item the team has begin.
    pub item_numbers: &'static str,
    /// The faces of the portraits by a name (`say pika joyous ...`).
    pub faces: Named<u16>,
    /// The emote a face goes with; the faces listeners jump at (see
    /// `Director::listen`).
    pub face_emotes: &'static [(u16, u16)],
    pub leaping_faces: &'static [u16],
    /// The sound an emote goes with.
    pub emote_sounds: &'static [(u16, u16)],
    /// The emotes scenes show most, by a name.
    pub emotes: Named<u16>,
    /// Songs and sounds heard around, by a name.
    pub songs: Named<u16>,
    /// The game's sounds by a name.
    pub sounds: Named<u16>,
    /// The groups of a sprite's animations: by name, and which.
    pub animations: Named<usize>,
    /// The status icons, by their pictures' names.
    pub status_icons: &'static [&'static str],
    /// The numbers the game gives scenes, always up to date (and those
    /// named by these beginnings: `item_109`, `recruit_25`).
    pub given_numbers: &'static [&'static str],
    pub given_beginnings: &'static [&'static str],
}

fn find<T: Copy>(table: Named<T>, name: &str) -> Option<T> {
    table.iter().find(|(known, _)| *known == name).map(|(_, value)| *value)
}

impl Vocabulary {
    /// No game at all.
    pub const NONE: Vocabulary = Vocabulary {
        host_now: &[],
        host_course: &[],
        host_holds: &[],
        host_numbers: &[],
        host_counts: &[],
        host_asks: |_| None,
        no_item: "",
        item_name: ("", ""),
        item_numbers: "",
        faces: &[],
        face_emotes: &[],
        leaping_faces: &[],
        emote_sounds: &[],
        emotes: &[],
        songs: &[],
        sounds: &[],
        animations: &[],
        status_icons: &[],
        given_numbers: &[],
        given_beginnings: &[],
    };

    pub fn face(&self, name: &str) -> Option<u16> {
        find(self.faces, name)
    }

    pub fn song(&self, name: &str) -> Option<u16> {
        find(self.songs, name)
    }

    pub fn sound(&self, name: &str) -> Option<u16> {
        find(self.sounds, name)
    }

    pub fn emote(&self, name: &str) -> Option<u16> {
        find(self.emotes, name)
    }

    /// The emote a portrait's face goes with, if any.
    pub(crate) fn emote_of_face(&self, face: u16) -> Option<u16> {
        self.face_emotes.iter().find(|(known, _)| *known == face).map(|(_, emote)| *emote)
    }

    /// The sound the emote of a face goes with, if any.
    pub(crate) fn sound_of_face(&self, face: u16) -> Option<u16> {
        let emote = self.emote_of_face(face)?;
        self.emote_sounds.iter().find(|(known, _)| *known == emote).map(|(_, sound)| *sound)
    }

    /// An animation's group by its name, whatever the case.
    pub fn animation(&self, name: &str) -> Option<usize> {
        let name = name.to_lowercase();
        self.animations.iter().find(|(one, _)| one.to_lowercase() == name).map(|(_, group)| *group)
    }

    /// A status icon's picture by its name.
    pub(crate) fn status_icon(&self, name: &str) -> Option<&'static str> {
        self.status_icons.iter().find(|own| **own == name).copied()
    }

    /// Whether that word is a line only the game can play.
    pub fn host_line(&self, word: &str) -> bool {
        self.host_now.contains(&word) || self.host_course.contains(&word)
    }
}

impl Default for &'static Vocabulary {
    fn default() -> Self {
        &Vocabulary::NONE
    }
}

/// A small one, for the engine's own tests.
#[cfg(test)]
pub(crate) static SAMPLE: Vocabulary = Vocabulary {
    host_now: &["getmoney", "getitem", "takeitem", "recruit"],
    host_course: &["dungeon", "ground", "menu", "open", "unlock"],
    host_holds: &["menu"],
    host_numbers: &[("getmoney", 1), ("getitem", 2), ("takeitem", 2)],
    host_counts: &[("getmoney", "money")],
    host_asks: |_| None,
    no_item: "Nothing",
    item_name: ("{item:", "}"),
    item_numbers: "item_",
    faces: &[("normal", 0), ("happy", 1), ("angry", 3), ("sad", 5), ("determined", 9), ("joyous", 10), ("surprised", 12)],
    face_emotes: &[(3, 91), (5, 171), (1, 93), (10, 93), (12, 88)],
    leaping_faces: &[10, 12],
    emote_sounds: &[(88, 8964)],
    emotes: &[("!", 88), ("?", 92), ("joy", 93), ("none", 0)],
    songs: &[("jolly_master", 73), ("rain", 106)],
    sounds: &[("exclamation", 8964), ("hop", 8974)],
    animations: &[("Walk", 0), ("Sit", 23), ("EventSleep", 13)],
    status_icons: &["sleep"],
    given_numbers: &[
        "money",
        "items",
        "answer",
        "recruits",
        "hero",
        "partner",
        "target",
        "clock",
        "music_clock",
        "timer",
        "time",
        "talking",
        "presses",
        "too_late",
        "cancelled",
    ],
    given_beginnings: &["item_", "recruit_"],
};
