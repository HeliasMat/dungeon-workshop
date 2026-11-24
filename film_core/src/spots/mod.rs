//! Where a line says: a word of the place, a point, a tile, a spot of the
//! film's, beside someone. A place written here is only what the words say:
//! it is found when it plays, on the stage (see `Spot` in the engine).
//!
//! - `routes`: going there, how fast, by what way;
//! - `formations`: several standing in a shape.

pub mod formations;
pub mod routes;

use crate::check::nearby;
use crate::emit;
use crate::film::Film;
use crate::words::{SPOTS, digits, plain, spot, tile};
use engine::{Point, Spot, Who};
use glam::Vec2;

/// A place a line says.
pub enum Goal {
    /// It says one, for the engine.
    Spot(Spot),
    /// It says one wrong, told.
    Told,
    /// It says none.
    Nothing,
}

/// Where the view may go.
pub enum Aim {
    At(i32, i32),
    Found(Spot),
}

/// A point of whole pixels, as written.
pub fn xy(x: i32, y: i32) -> Point {
    Point::at(Vec2::new(x as f32, y as f32))
}

/// The last two words, when they are a point in pixels (`200 150`).
fn ending_pixels(words: &[String]) -> Option<Point> {
    match words {
        [.., x, y] if digits(x) && digits(y) => emit::point(x, y),
        _ => None,
    }
}

impl Film {
    /// The middle of the scene, as a point; as a spot, a word of the place.
    pub fn middle_at(&self) -> Point {
        xy(self.middle.0, self.middle.1)
    }

    pub fn middle_spot(&self) -> Spot {
        Spot::Word(self.middle_at())
    }

    /// Someone a word names, by the code the film gives (`code`); no one:
    /// told.
    pub fn coded(&mut self, word: &str) -> Option<Who> {
        let who = Who::parse(&self.code(word));
        if who.is_none() {
            self.not_a(word, "a Pokémon of the scene");
        }
        who
    }

    /// Where someone a word names stands; beside them, on free ground; so
    /// far from them. No one: told.
    pub fn spot_of(&mut self, word: &str) -> Option<Spot> {
        self.coded(word).map(Spot::Of)
    }

    pub fn spot_beside(&mut self, word: &str) -> Option<Spot> {
        self.coded(word).map(Spot::Beside)
    }

    pub fn spot_by(&mut self, word: &str, dx: f32, dy: f32) -> Option<Spot> {
        Some(Spot::By { of: self.coded(word)?, dx, dy })
    }

    /// Every word of a place the film knows here: the place's, the film's.
    fn spot_words(&self) -> Vec<String> {
        SPOTS.iter().map(|(name, _)| name.to_string()).chain(self.spots.iter().cloned()).collect()
    }
}

fn said_near(words: &[String]) -> bool {
    match words.first().map(String::as_str) {
        Some("tile" | "front_of" | "behind" | "between" | "left_of" | "right_of") => true,
        Some(count) if digits(count) => words.len() >= 3 && matches!(words[1].as_str(), "tile" | "tiles") && words[2] == "from",
        _ => false,
    }
}

/// What a place is measured from: a spot of the film's, or someone.
enum From {
    Spot(String),
    One(Who),
}

/// Someone or a spot a place is measured from; told when neither.
fn from(film: &mut Film, word: &str, number: usize, said: &str) -> Option<From> {
    if film.spots.contains(word) {
        return Some(From::Spot(word.to_string()));
    }
    if film.actor(word).is_some() {
        return film.coded(word).map(From::One);
    }
    let known: Vec<String> = film.spots.iter().cloned().chain(film.actors.keys().cloned()).collect();
    let close = nearby(word, known.iter().map(String::as_str));
    film.wrong(number, format!("“{word}” is not on the stage: where is “{said}”?{close}"));
    None
}

/// Someone a place is measured from, not a spot; told when not.
fn someone_from(film: &mut Film, word: &str, number: usize, said: &str) -> Option<Who> {
    match from(film, word, number, said)? {
        From::One(who) => Some(who),
        From::Spot(_) => {
            film.wrong(number, format!("“{word}” is a spot, not someone: where is “{said}”?"));
            None
        }
    }
}

/// The place of words said in relation to something; none if wrong, told.
fn near_field(film: &mut Film, words: &[String], number: usize) -> Option<Spot> {
    let said = words.join(" ");
    let text: Vec<&str> = words.iter().map(String::as_str).collect();
    // (`front_of throne`: a spot has no way it looks: the spot itself.)
    let tell = |film: &mut Film, word: &str, kind: &dyn Fn(Who) -> Spot| {
        Some(match from(film, word, number, &said)? {
            From::Spot(name) => Spot::Named(name),
            From::One(who) => kind(who),
        })
    };
    match text.as_slice() {
        ["tile", x, y] if digits(x) && digits(y) => Some(Spot::Near(xy(tile(x.parse().unwrap_or(0)), tile(y.parse().unwrap_or(0))))),
        ["front_of", who] => tell(film, who, &|of| Spot::Before { of, back: false }),
        ["behind", who] => tell(film, who, &|of| Spot::Before { of, back: true }),
        ["between", a, b] => {
            let (a, b) = (someone_from(film, a, number, &said)?, someone_from(film, b, number, &said)?);
            Some(Spot::Between(a, b))
        }
        ["left_of", who] => tell(film, who, &|of| Spot::Side { of, right: false }),
        ["right_of", who] => tell(film, who, &|of| Spot::Side { of, right: true }),
        [count, "tile" | "tiles", "from", who] if digits(count) => {
            let count: f32 = count.parse().unwrap_or(0.0);
            tell(film, who, &|of| Spot::Tiles { of, count })
        }
        _ => {
            film.wrong(number, format!("place badly said “{said}”: front_of a, behind a, left_of a, right_of a, between a b, 3 tiles from a, tile 12 7"));
            None
        }
    }
}

/// A spot of the film's or a place said in relation to something.
fn named_or_near(film: &mut Film, words: &[String], number: usize) -> Goal {
    let mut words: Vec<String> = words.iter().map(|word| plain(word)).collect();
    if words.first().is_some_and(|word| word == "to") {
        words.remove(0);
    }
    if words.first().is_some_and(|word| word == "at") && words.len() == 2 {
        words.remove(0);
    }
    if words.len() == 1 && film.spots.contains(&words[0]) {
        return Goal::Spot(Spot::Named(words[0].clone()));
    }
    if said_near(&words) {
        return near_field(film, &words, number).map_or(Goal::Told, Goal::Spot);
    }
    Goal::Nothing
}

/// The place a line sends someone to (the words after who): a word, a point in
/// pixels, a spot of the film's, a tile, a place said in relation to someone.
pub fn goal_of(film: &mut Film, words: &[String], number: usize) -> Goal {
    match named_or_near(film, words, number) {
        Goal::Nothing => {}
        said => return said,
    }
    let Some(last) = words.last() else {
        return Goal::Nothing;
    };
    if let Some(at) = ending_pixels(words) {
        return Goal::Spot(Spot::Written(at));
    }
    let word = plain(last);
    match spot(&word, film.middle) {
        Some((x, y)) if word.starts_with("off") => Goal::Spot(Spot::Exact(xy(x, y))),
        Some((x, y)) => Goal::Spot(Spot::Word(xy(x, y))),
        None => Goal::Nothing,
    }
}

/// A point or a word of the place at the end of the words, and the words
/// before it: `set_down a Apple 200 150`, `set_down a Apple left`.
pub fn ending_point(film: &Film, words: &[String]) -> (Vec<String>, Option<Spot>) {
    let mut words = words.to_vec();
    if let Some(at) = ending_pixels(&words) {
        words.truncate(words.len() - 2);
        return (words, Some(Spot::Written(at)));
    }
    if let Some((x, y)) = words.last().and_then(|word| spot(&plain(word), film.middle)) {
        words.pop();
        return (words, Some(Spot::Word(xy(x, y))));
    }
    (words, None)
}

/// Where a line puts someone; the middle if it does not say, told if it says
/// wrong.
pub fn somewhere(film: &mut Film, words: &[String], number: usize) -> Spot {
    match goal_of(film, words, number) {
        Goal::Spot(spot) => return spot,
        Goal::Nothing if !words.is_empty() => {
            let names = film.spot_words();
            let close = nearby(&words[words.len() - 1], names.iter().map(String::as_str));
            film.wrong(number, format!("which spot? “{}”{close}", words.join(" ")));
        }
        _ => {}
    }
    film.middle_spot()
}

/// Where the view may go: a point, whoever stands in the way.
pub fn point_said(film: &mut Film, words: &[String], number: usize) -> Option<Aim> {
    let words: Vec<String> = words.iter().map(|word| plain(word)).collect();
    if words.len() == 2 && digits(&words[0]) && digits(&words[1]) {
        return Some(Aim::At(words[0].parse().ok()?, words[1].parse().ok()?));
    }
    if words.len() == 1 && film.spots.contains(&words[0]) {
        return Some(Aim::Found(Spot::Named(words[0].clone())));
    }
    if words.len() == 1
        && let Some((x, y)) = spot(&words[0], film.middle)
    {
        return Some(Aim::At(x, y));
    }
    if said_near(&words) {
        return near_field(film, &words, number).map(Aim::Found);
    }
    None
}

/// Where the view goes, told when it is no one nor a place (see `aim`).
pub fn nowhere(film: &mut Film, words: &[String], number: usize, what: &str) {
    let known = film.spot_words();
    let close = nearby(words.last().map_or("", String::as_str), known.iter().map(String::as_str));
    film.wrong(number, format!("{what} “{}”{close}", words.join(" ")));
}
