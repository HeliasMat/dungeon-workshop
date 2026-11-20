//! The words scene files use, by what they mean.

use super::*;

/// What is in the air over a scene, by a name.
pub fn weather_named(name: &str) -> Option<Weather> {
    Some(match name {
        "none" => Weather::Clear,
        "rain" => Weather::Rain,
        "snow" => Weather::Snow,
        "petals" => Weather::Petals,
        "sparkles" => Weather::Sparkles,
        "leaves" => Weather::Leaves,
        "hail" => Weather::Hail,
        "sand" => Weather::Sand,
        "mist" => Weather::Mist,
        "sun" => Weather::Sun,
        _ => return None,
    })
}

/// A colour of the game, by what code writes (`Colour::Green`); a film
/// names one by its word (`colour_named`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Colour {
    White,
    Red,
    Yellow,
    Blue,
    Green,
    Black,
    Pink,
    Orange,
    Purple,
    Cyan,
    Gold,
    Grey,
    Brown,
    Sand,
    Turquoise,
}

impl Colour {
    /// All of them, as their words are listed.
    pub const ALL: [Colour; 15] = [
        Colour::White,
        Colour::Red,
        Colour::Yellow,
        Colour::Blue,
        Colour::Green,
        Colour::Black,
        Colour::Pink,
        Colour::Orange,
        Colour::Purple,
        Colour::Cyan,
        Colour::Gold,
        Colour::Grey,
        Colour::Brown,
        Colour::Sand,
        Colour::Turquoise,
    ];

    /// Its word, as the files write it.
    pub fn name(self) -> &'static str {
        match self {
            Colour::White => "white",
            Colour::Red => "red",
            Colour::Yellow => "yellow",
            Colour::Blue => "blue",
            Colour::Green => "green",
            Colour::Black => "black",
            Colour::Pink => "pink",
            Colour::Orange => "orange",
            Colour::Purple => "purple",
            Colour::Cyan => "cyan",
            Colour::Gold => "gold",
            Colour::Grey => "grey",
            Colour::Brown => "brown",
            Colour::Sand => "sand",
            Colour::Turquoise => "turquoise",
        }
    }

    /// The colour of a word, if it is one.
    pub fn of(name: &str) -> Option<Colour> {
        Colour::ALL.into_iter().find(|colour| colour.name() == name)
    }

    /// Its red, green and blue, as a flash has them.
    pub fn rgb(self) -> [f32; 3] {
        match self {
            Colour::White => [255.0, 255.0, 255.0],
            Colour::Red => [255.0, 40.0, 40.0],
            Colour::Yellow => [255.0, 230.0, 80.0],
            Colour::Blue => [80.0, 140.0, 255.0],
            Colour::Green => [80.0, 230.0, 100.0],
            Colour::Black => [0.0, 0.0, 0.0],
            Colour::Pink => [255.0, 150.0, 200.0],
            Colour::Orange => [255.0, 150.0, 40.0],
            Colour::Purple => [170.0, 90.0, 255.0],
            Colour::Cyan => [80.0, 230.0, 255.0],
            Colour::Gold => [255.0, 200.0, 60.0],
            Colour::Grey => [140.0, 140.0, 140.0],
            Colour::Brown => [140.0, 90.0, 50.0],
            Colour::Sand => [231.0, 175.0, 119.0],
            Colour::Turquoise => [60.0, 200.0, 180.0],
        }
    }

    /// Its red, green and blue as bytes, as a popup has them.
    pub fn bytes(self) -> [u8; 3] {
        self.rgb().map(|part| part as u8)
    }
}

/// A flash's colour, by a name.
pub fn colour_named(name: &str) -> Option<[f32; 3]> {
    Colour::of(name).map(Colour::rgb)
}

/// The light of a moment, by what code writes (`Daylight::Night`); a film
/// names one by its word (`light_named`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Daylight {
    Normal,
    Dawn,
    Evening,
    Night,
    Cave,
    Storm,
    Dream,
    Fire,
}

impl Daylight {
    /// Its tint: red, green, blue and how strong (see `ScreenBeat::Tint`).
    pub fn tint(self) -> [f32; 4] {
        match self {
            Daylight::Normal => [0.0, 0.0, 0.0, 0.0],
            Daylight::Dawn => [255.0, 190.0, 150.0, 50.0],
            Daylight::Evening => [200.0, 80.0, 50.0, 80.0],
            Daylight::Night => [20.0, 30.0, 90.0, 120.0],
            Daylight::Cave => [0.0, 0.0, 0.0, 110.0],
            Daylight::Storm => [40.0, 40.0, 60.0, 100.0],
            Daylight::Dream => [200.0, 150.0, 255.0, 60.0],
            Daylight::Fire => [255.0, 80.0, 20.0, 60.0],
        }
    }

    /// The light of a word, if it is one (`day` is `normal`).
    pub fn of(name: &str) -> Option<Daylight> {
        Some(match name {
            "normal" | "day" => Daylight::Normal,
            "dawn" => Daylight::Dawn,
            "evening" => Daylight::Evening,
            "night" => Daylight::Night,
            "cave" => Daylight::Cave,
            "storm" => Daylight::Storm,
            "dream" => Daylight::Dream,
            "fire" => Daylight::Fire,
            _ => return None,
        })
    }
}

/// The light of a moment, by a name (see `ScreenBeat::Tint`).
pub fn light_named(name: &str) -> Option<[f32; 4]> {
    Daylight::of(name).map(Daylight::tint)
}

/// A note of the scale, by what code writes (`Pitch::E`); a film names one
/// by its word (`note_named`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pitch {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Pitch {
    /// The note, half tones over `c`.
    pub fn half_tones(self) -> i32 {
        match self {
            Pitch::C => 0,
            Pitch::D => 2,
            Pitch::E => 4,
            Pitch::F => 5,
            Pitch::G => 7,
            Pitch::A => 9,
            Pitch::B => 11,
        }
    }

    /// The note an octave up (`e2`), half tones over `c`.
    pub fn high(self) -> i32 {
        self.half_tones() + 12
    }
}

/// A note by its name (`c`, `d`, `e`, `f`, `g`, `a`, `b`, then
/// the octave above with a 2: `c2`), or by half tones: how many half
/// tones over `c`.
pub fn note_named(name: &str) -> Option<i32> {
    const NOTES: [(&str, Pitch); 7] = [("c", Pitch::C), ("d", Pitch::D), ("e", Pitch::E), ("f", Pitch::F), ("g", Pitch::G), ("a", Pitch::A), ("b", Pitch::B)];
    if let Ok(half_tones) = name.parse() {
        return Some(half_tones);
    }
    let (note, octave) = match name.strip_suffix('2') {
        Some(note) => (note, 12),
        None => (name, 0),
    };
    NOTES.iter().find(|(named, _)| *named == note).map(|(_, known)| known.half_tones() + octave)
}

/// The particles a scene shows, by their name (see `ScreenBeat::Particles`):
/// fireworks bursting and falling, sparks thrown up, fireflies drifting
/// and blinking, embers rising, smoke, bubbles, dust coming down, drops
/// flung about, a healing glow, rings widening on water.
pub(crate) const PARTICLES: [(&str, ParticleKind); 10] = [
    ("firework", ParticleKind::Fireworks),
    ("sparks", ParticleKind::Sparks),
    ("fireflies", ParticleKind::Fireflies),
    ("embers", ParticleKind::Embers),
    ("smoke", ParticleKind::Smoke),
    ("bubbles", ParticleKind::Bubbles),
    ("dust", ParticleKind::Dust),
    ("drops", ParticleKind::Drops),
    ("healing", ParticleKind::Healing),
    ("rings", ParticleKind::Rings),
];

/// A kind of particles by its name (`dust`, `sparks`, `bubbles`…).
pub fn particle_kind(name: &str) -> Option<ParticleKind> {
    PARTICLES.iter().find(|(known, _)| *known == name).map(|(_, kind)| *kind)
}

/// The particles by their names, one for each kind.
pub fn particle_names() -> impl Iterator<Item = &'static str> {
    PARTICLES.iter().enumerate().filter(|(at, (_, kind))| PARTICLES[..*at].iter().all(|(_, before)| before != kind)).map(|(_, (name, _))| *name)
}

/// Where words kept on screen go (`hud <place> <words>`): on the left, in
/// the middle, on the right.
pub(crate) const HUD_PLACES: [(&str, usize); 4] = [("left", 0), ("middle", 1), ("center", 1), ("right", 2)];

/// A place of words kept on screen by its name (see `HUD_PLACES`).
pub fn hud_place(name: &str) -> Option<usize> {
    HUD_PLACES.iter().find(|(named, _)| *named == name).map(|(_, at)| *at)
}

#[cfg(test)]
mod typed_names {
    use super::*;

    #[test]
    fn each_typed_name_has_its_word() {
        for colour in Colour::ALL {
            assert_eq!(Colour::of(colour.name()), Some(colour));
        }
        assert_eq!(light_named("night"), Some(Daylight::Night.tint()));
        assert_eq!(note_named("e2"), Some(Pitch::E.high()));
        assert_eq!(note_named("g"), Some(Pitch::G.half_tones()));
    }
}
