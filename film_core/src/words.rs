//! What the films' language calls things: the tables of its small words, and
//! how a word is read (plain, a number, a time).

use engine::{TILE, is_command};

use scenes::vocabulary::POKEMON;

/// A walker's pace, the engine's.
pub use engine::WALK;

/// The middle of a tile of the game's grid, by its number.
pub fn tile(number: i32) -> i32 {
    number * TILE as i32 + TILE as i32 / 2
}

/// How close each shot goes: never nearer than the view's own.
pub const SHOTS: [(&str, f32); 3] = [("wide", 0.5), ("medium", 0.75), ("close", 1.0)];

/// From the middle of a place. `far`: further up; `near`: nearer the box.
pub const SPOTS: [(&str, (i32, i32)); 9] = [
    ("center", (0, 0)),
    ("left", (-64, 0)),
    ("right", (64, 0)),
    ("far", (0, -30)),
    ("near", (0, 18)),
    ("left-far", (-64, -30)),
    ("right-far", (64, -30)),
    ("offleft", (-150, 0)),
    ("offright", (150, 0)),
];

/// The words of a face; the face.
pub const FACES: [(&str, &str); 20] = [
    ("normal", "normal"),
    ("happy", "happy"),
    ("joyous", "joyous"),
    ("pain", "pain"),
    ("angry", "angry"),
    ("worried", "worried"),
    ("sad", "sad"),
    ("crying", "crying"),
    ("teary_eyed", "teary_eyed"),
    ("shouting", "shouting"),
    ("determined", "determined"),
    ("inspired", "inspired"),
    ("surprised", "surprised"),
    ("dizzy", "dizzy"),
    ("sigh", "sigh"),
    ("stunned", "stunned"),
    ("special_one", "special_one"),
    ("special_two", "special_two"),
    ("special_three", "special_three"),
    ("special_four", "special_four"),
];

pub const EMOTIONS: [(&str, &str); 12] = [
    ("joy", "joy"),
    ("surprise", "exclaim"),
    ("question", "question"),
    ("silence", "drop"),
    ("sweat", "drop"),
    ("anger", "anger"),
    ("tears", "sweat"),
    ("shock", "shock"),
    ("sparkle", "joy"),
    ("eureka", "exclaim"),
    ("dust", "dust"),
    ("smoke", "smoke"),
];

pub const EMOTION_SOUNDS: [(&str, &str); 7] =
    [("surprise", "startle"), ("question", "question"), ("sweat", "drop"), ("silence", "drop"), ("anger", "shock"), ("tears", "sweat"), ("shock", "alert")];

/// A hop (height, frames), an emote and its sound, the frames given.
pub type Acting = (Option<(u32, u32)>, Option<&'static str>, Option<&'static str>, u32);

/// How someone takes on a face before saying what goes with it, as the game's
/// scripts have it most.
pub const ACTING: [(&str, Acting); 13] = [
    ("surprised", (Some((6, 6)), Some("exclaim"), Some("startle"), 24)),
    ("joyous", (Some((10, 10)), Some("joy"), None, 20)),
    ("happy", (None, Some("laugh"), None, 20)),
    ("angry", (Some((6, 6)), Some("anger"), Some("shock"), 20)),
    ("worried", (None, Some("drop"), Some("drop"), 41)),
    ("sad", (None, Some("sweat"), Some("sweat"), 27)),
    ("crying", (None, Some("sweat"), Some("sweat"), 27)),
    ("teary_eyed", (None, Some("sweat"), Some("sweat"), 27)),
    ("pain", (None, Some("sweat"), Some("sweat"), 27)),
    ("determined", (Some((6, 6)), None, None, 10)),
    ("inspired", (Some((6, 6)), Some("exclaim"), Some("exclamation"), 24)),
    ("dizzy", (None, Some("question"), Some("question"), 41)),
    ("shouting", (Some((6, 6)), Some("exclaim"), Some("alert"), 14)),
];

/// The light of the hours of a day, as `engine/src/beat/names.rs` has them: an
/// hour, a colour, how much of it (noon has none: no veil at all).
pub const DAY: [(f32, [f32; 3], f32); 6] = [
    (0.0, [20.0, 30.0, 90.0], 120.0),
    (6.0, [255.0, 190.0, 150.0], 50.0),
    (12.0, [255.0, 190.0, 150.0], 0.0),
    (15.0, [200.0, 80.0, 50.0], 0.0),
    (18.0, [200.0, 80.0, 50.0], 80.0),
    (24.0, [20.0, 30.0, 90.0], 120.0),
];

/// What sets a scene: done before it shows.
pub const SETTING: [&str; 29] = [
    "collisions",
    "put",
    "zoom",
    "camera",
    "light",
    "tint",
    "music",
    "crowd",
    "spot",
    "row",
    "column",
    "triangle",
    "form",
    "pose",
    "status",
    "every",
    "fly",
    "halo",
    "sky",
    "starry_night",
    "daytime",
    "gathering",
    "all_pokemon",
    "generation",
    "height",
    "infront",
    "still",
    "thing",
    "carry",
];

/// What a look is, as a word of the engine.
pub const WAYS: [&str; 8] = ["down", "downright", "right", "upright", "up", "upleft", "left", "downleft"];

/// The value of a word in a table.
pub fn of<T: Copy>(table: &[(&str, T)], word: &str) -> Option<T> {
    table.iter().find(|(name, _)| *name == word).map(|(_, value)| *value)
}

/// A place's word, from its middle.
pub fn spot(word: &str, middle: (i32, i32)) -> Option<(i32, i32)> {
    of(&SPOTS, word).map(|(x, y)| (middle.0 + x, middle.1 + y))
}

/// A word as a film compares it: small, no accent, nothing but letters,
/// numbers, `_` and `-`.
pub fn plain(word: &str) -> String {
    language::text::normalise(word, language::text::Kept::Words)
}

/// Whether the engine reads that word, as a command or the name of a sound.
pub fn engine_word(word: &str) -> bool {
    is_command(word, &POKEMON)
}

/// Whether a word names one of the game's sounds or songs.
pub fn heard_word(word: &str) -> bool {
    POKEMON.sound(word).is_some() || POKEMON.song(word).is_some()
}

/// A word that says a number of frames: `1.5s` (seconds), `6f` (frames, for
/// what is very short), `12` (frames, as written before the units).
pub fn frames_of(word: &str) -> Option<i32> {
    let (number, seconds) = match (word.strip_suffix('s'), word.strip_suffix('f')) {
        (Some(number), _) => (number, true),
        (_, Some(number)) => (number, false),
        _ => (word, false),
    };
    if number.is_empty() || !number.chars().all(|letter| letter.is_ascii_digit() || letter == '.') {
        return None;
    }
    let number: f32 = number.parse().ok()?;
    Some((number * if seconds { 60.0 } else { 1.0 }).round() as i32)
}

/// Whether a word is all digits.
pub fn digits(word: &str) -> bool {
    !word.is_empty() && word.bytes().all(|letter| letter.is_ascii_digit())
}

/// A number of frames as it is best written: in seconds when that falls
/// right (`30` is `0.5s`), in frames else (`7f`).
pub fn in_seconds_said(frames: &str) -> String {
    let Ok(frames) = frames.parse::<i32>() else {
        return frames.to_string();
    };
    let seconds = (frames as f32 / 60.0 * 100.0).round() / 100.0;
    if ((seconds * 60.0).round() as i32) == frames && frames >= 6 { format!("{}s", seconds) } else { format!("{frames}f") }
}

/// A line's first word, and what follows it trimmed.
pub fn head(line: &str) -> (&str, &str) {
    let line = line.trim_start();
    let end = line.find(char::is_whitespace).unwrap_or(line.len());
    (&line[..end], line[end..].trim_start())
}

/// The first word of a line, as compared.
pub fn first_word(line: &str) -> String {
    plain(head(line).0)
}

/// Whether a word is a time in seconds: `3s`, `0.5s`.
pub fn in_seconds(word: &str) -> bool {
    word.strip_suffix('s').is_some_and(|number| !number.is_empty() && number.chars().all(|letter| letter.is_ascii_digit() || letter == '.'))
}

/// `who (face): words` at the start of a line: who, the brackets, the words.
pub fn spoken(line: &str) -> Option<(&str, Option<&str>, &str)> {
    let end = line.find(|letter: char| !(letter.is_alphanumeric() || letter == '_' || letter == '-'))?;
    let (name, rest) = line.split_at(end);
    if name.is_empty() {
        return None;
    }
    let rest = rest.trim_start();
    let (brackets, rest) = match rest.strip_prefix('(') {
        Some(inner) => {
            let (brackets, after) = inner.split_once(')')?;
            (Some(brackets), after.trim_start())
        }
        None => (None, rest),
    };
    Some((name, brackets, rest.strip_prefix(':')?.trim_start()))
}

/// Who says the lines without being on the stage.
pub const TELLERS: [&str; 2] = ["narrator", "voice"];

/// Whether a name is the narrator's.
pub fn told(name: &str) -> bool {
    TELLERS.contains(&plain(name).as_str())
}

/// What is left of a line once its marks (`[W:90]`) are out.
pub fn without_marks(text: &str) -> String {
    let (mut out, mut inside) = (String::new(), false);
    for letter in text.chars() {
        match letter {
            '[' => inside = true,
            ']' if inside => inside = false,
            _ if !inside => out.push(letter),
            _ => {}
        }
    }
    out
}
