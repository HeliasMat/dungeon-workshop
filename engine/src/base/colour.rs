//! The game's colours, by what code writes and by their words.

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
