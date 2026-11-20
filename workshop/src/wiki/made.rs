//! What the wiki makes of a word's forms by itself: each form written as an
//! author reads it (`who walk to where [in d]`), each of its parts with what
//! it means, and a first example, a little film around a line of the word
//! with sample values (Pikachu and Piplup on the beach, a place, a second…).

use film::grammar::{Kind, Piece};

/// A part of a form: its name, what it means, whether it may be left out, what it is then.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub name: String,
    pub means: String,
    pub optional: bool,
    pub default: Option<String>,
}

/// A form as it is shown: written (who first if it is said of someone), its parts.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub shown: String,
    pub parts: Vec<Part>,
    /// It starts with someone, written first (`who walk to where`).
    pub someone_first: bool,
}

/// Those in the examples made: the first two of the scene.
pub const CAST: [&str; 2] = ["pikachu", "piplup"];

/// The names a recipe gives to whom it is said of (`who`, `other`, `a`…): someone.
const PEOPLE: [&str; 15] = ["who", "other", "a", "b", "c", "target", "friend", "him", "leader", "one", "from", "with", "prey", "winner", "to"];

/// The names of words said (`text...`): a few words are written there.
const SAID: [&str; 5] = ["text", "title", "words", "said", "what"];

/// What a part's name says it is, before its kind does.
const NAMES: [(&str, &str); 40] = [
    ("who", "who does it (someone of the scene)"),
    ("other", "the other one (someone of the scene)"),
    ("target", "who, or where, it is aimed at"),
    ("where", "where: a place (x y, someone, left_of b, front_of b, a spot)"),
    ("to", "where to"),
    ("from", "where from"),
    ("d", "how long (1.5s, 30f)"),
    ("duration", "how long (1.5s, 30f)"),
    ("time", "how long (1.5s, 30f)"),
    ("n", "how many, or how much"),
    ("text", "the words, as written"),
    ("thing", "the thing, by its name (Apple, Oran Berry)"),
    ("what", "the thing, by its name"),
    ("colour", "a colour (white, red, pink, blue…)"),
    ("kind", "which kind (its word)"),
    ("species", "a species (pikachu, krabby…)"),
    ("group", "the name of a group"),
    ("groups", "groups, or those of the scene"),
    ("members", "those of the scene"),
    ("x", "across, in pixels"),
    ("y", "down, in pixels"),
    ("dx", "how far across, in pixels"),
    ("dy", "how far down, in pixels"),
    ("turns", "how many turns"),
    ("side", "a side or an entrance (left, east…)"),
    ("via", "the way taken"),
    ("entrance", "an entrance of the place (left, east, south…)"),
    ("way", "a direction (left, right, up, down)"),
    ("name", "its name"),
    ("hp", "health points"),
    ("level", "a level"),
    ("lv", "a level"),
    ("strength", "how strong"),
    ("radius", "how wide, in pixels"),
    ("move", "a move of the game (Thunderbolt, Surf…)"),
    ("image", "a picture, by its name"),
    ("flag", "a flag of the game, by its name"),
    ("moment", "a time of day (dawn, day, evening, night)"),
    ("tiles", "how many tiles"),
    ("rest", "the rest of the line, as the word reads it"),
];

/// What a part means, by its name, else by its kind.
fn meaning(name: &str, kind: Kind) -> String {
    NAMES.iter().find(|(known, _)| *known == name).map_or_else(|| kind.told().1.to_string(), |(_, said)| said.to_string())
}

/// A form of a word as an author reads it, and its parts: `who walk to where [in d]`.
pub fn form(word: &str, form: &str, subject: bool) -> Form {
    let pieces = film::grammar::read(form);
    let mut parts = Vec::new();
    collect(&pieces, false, &mut parts);
    let written = names_only(&pieces);
    let first = subject && someone_first(&pieces);
    let shown = match (first, written.split_once(' ')) {
        (true, Some((who, rest))) => format!("{who} {word} {rest}"),
        (true, None) if !written.is_empty() => format!("{written} {word}"),
        _ => format!("{word} {written}").trim_end().to_string(),
    };
    Form { shown, parts, someone_first: first }
}

/// The parts of all the forms, each name once (its first meaning).
pub fn all_parts(forms: &[Form]) -> Vec<Part> {
    let mut all: Vec<Part> = Vec::new();
    for part in forms.iter().flat_map(|form| &form.parts) {
        if !all.iter().any(|known| known.name == part.name) {
            all.push(part.clone());
        }
    }
    all
}

/// Whether a form starts with someone (it is then written who first: `pikachu walk to right`).
fn someone_first(pieces: &[Piece]) -> bool {
    match pieces.first() {
        Some(Piece::Team(_)) | Some(Piece::One { kind: Kind::Someone | Kind::Code, .. }) => true,
        Some(Piece::One { kind: Kind::Word, name, default: None }) => PEOPLE.contains(&name.as_str()),
        _ => false,
    }
}

/// The pieces written by their names only: `who to where [in d]`, `*who` as `who…`.
fn names_only(pieces: &[Piece]) -> String {
    let words: Vec<String> = pieces
        .iter()
        .map(|piece| match piece {
            Piece::One { name, default: Some(_), .. } | Piece::Words { name, default: Some(_), .. } => format!("[{name}]"),
            Piece::One { name, .. } => name.clone(),
            Piece::Team(name) => format!("{name}…"),
            Piece::Words { name, .. } => format!("{name}…"),
            Piece::Choice(words) => words.join("|"),
            Piece::Maybe(inner) => format!("[{}]", names_only(inner)),
        })
        .collect();
    words.join(" ")
}

/// The parts of the pieces, those in `[…]` optional.
fn collect(pieces: &[Piece], optional: bool, parts: &mut Vec<Part>) {
    for piece in pieces {
        let part = |name: &str, means: String, default: &Option<Vec<String>>| Part {
            name: name.to_string(),
            means,
            optional: optional || default.is_some(),
            default: default.as_ref().map(|words| words.join(" ")),
        };
        match piece {
            Piece::One { name, kind, default } => parts.push(part(name, meaning(name, *kind), default)),
            Piece::Words { name, kind, default, .. } => parts.push(part(name, format!("{} (one word or more)", meaning(name, *kind)), default)),
            Piece::Team(name) => parts.push(part(name, "one or more of the scene".to_string(), &None)),
            Piece::Choice(words) if words.len() > 1 => {
                parts.push(Part { name: words.join("|"), means: format!("one of: {}", words.join(", ")), optional, default: None })
            }
            Piece::Choice(_) => {}
            Piece::Maybe(inner) => collect(inner, true, parts),
        }
    }
}

/// A value for a part, by its name, else by its kind; someone: the next of the cast.
fn value(name: &str, kind: Kind, people: &mut impl Iterator<Item = &'static str>) -> String {
    let by_name = match name {
        "colour" | "color" => Some("white"),
        "kind" | "emote" => Some("joy"),
        "species" => Some("krabby"),
        "x" | "x1" | "x0" => Some("200"),
        "y" | "y1" | "y0" => Some("150"),
        "x2" => Some("260"),
        "y2" => Some("150"),
        "dx" => Some("40"),
        "dy" => Some("0"),
        "hp" | "hp2" => Some("30"),
        "level" | "lv" => Some("5"),
        "move" | "blow" => Some("Thunderbolt"),
        "side" | "via" | "entrance" => Some("left"),
        "way" => Some("right"),
        "flag" => Some("met_piplup"),
        "moment" => Some("evening"),
        "group" => Some("friends"),
        "name" if kind == Kind::Word => Some("friends"),
        "turns" | "tiles" => Some("2"),
        "strength" => Some("3"),
        "radius" => Some("40"),
        _ => None,
    };
    if let Some(said) = by_name.filter(|_| kind != Kind::Someone) {
        return said.to_string();
    }
    if kind == Kind::Word && PEOPLE.contains(&name) {
        return people.next().unwrap_or(CAST[0]).to_string();
    }
    match kind {
        Kind::Someone | Kind::Code => people.next().unwrap_or(CAST[0]),
        Kind::Place => "right",
        Kind::Time => "1s",
        Kind::Number => "2",
        Kind::Coordinate => "200",
        Kind::Thing => "Apple",
        Kind::Property => "opacity",
        Kind::Word if SAID.contains(&name) => "What a day!",
        Kind::Word => "Hello!",
    }
    .to_string()
}

/// The words of a form, filled in: who first if it is said of someone; an
/// optional part left out, a choice its first word.
pub fn line(word: &str, form: &str, subject: bool) -> String {
    let mut people = CAST.into_iter().cycle();
    let mut words: Vec<String> = Vec::new();
    let pieces = film::grammar::read(form);
    let subject = subject && someone_first(&pieces);
    for piece in pieces {
        match piece {
            Piece::One { default: Some(_), .. } | Piece::Words { default: Some(_), .. } | Piece::Maybe(_) => {}
            // (All the rest of a line, which may be nothing: nothing, but for words said.)
            Piece::Words { least: 0, kind: Kind::Word, name, .. } if !SAID.contains(&name.as_str()) => {}
            Piece::One { name, kind, .. } | Piece::Words { name, kind, .. } => words.push(value(&name, kind, &mut people)),
            Piece::Team(_) => words.extend(CAST.iter().map(|who| who.to_string())),
            Piece::Choice(choices) => words.extend(choices.first().cloned()),
        }
    }
    match (subject, words.split_first()) {
        (true, Some((who, rest))) => {
            std::iter::once(who.clone()).chain(std::iter::once(word.to_string())).chain(rest.iter().cloned()).collect::<Vec<_>>().join(" ")
        }
        _ => std::iter::once(word.to_string()).chain(words).collect::<Vec<_>>().join(" "),
    }
}

/// The little film made around a line: the beach, Pikachu and Piplup side
/// by side, the line, a moment after; and the line's number in it.
pub fn film(line: &str) -> (String, usize) {
    let [a, b] = CAST;
    (format!("scene: beach {a} {b}\n{a} put center\n{b} put right_of {a}\n{line}\nwait 1s\n"), 4)
}

/// The first example of a word: a film around a line of its first form
/// (said of someone: of its first form that starts with someone).
pub fn example(word: &str, forms: &[String], subject: bool) -> (String, usize) {
    let first = forms.iter().find(|form| subject && someone_first(&film::grammar::read(form))).or(forms.first());
    film(&line(word, first.map_or("", String::as_str), subject))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_form_is_shown_by_its_names_its_parts_explained() {
        let walk = form("walk", "who:pokemon [to] where:place [slowly|fast] [in time:duration]", true);
        assert_eq!(walk.shown, "who walk [to] where [slowly|fast] [in time]");
        let names: Vec<&str> = walk.parts.iter().map(|part| part.name.as_str()).collect();
        assert_eq!(names, ["who", "where", "slowly|fast", "time"]);
        assert!(!walk.parts[0].optional && walk.parts[2].optional && walk.parts[3].optional);
        assert!(walk.parts[1].means.starts_with("where"), "{}", walk.parts[1].means);
        let tornado = form("tornado", "who:pokemon where:place [turns:number=4]", true);
        assert_eq!(tornado.parts[2].default.as_deref(), Some("4"));
        assert!(tornado.parts[2].optional);
        assert_eq!(form("sky", "kind:word", false).shown, "sky kind");
    }

    #[test]
    fn an_example_is_made_from_the_first_form() {
        assert_eq!(line("walk", "who:pokemon [to] where:place [in time:duration]", true), "pikachu walk right");
        assert_eq!(line("emote", "who:pokemon kind:word", true), "pikachu emote joy");
        assert_eq!(line("look", "who:pokemon to:word", true), "pikachu look piplup");
        assert_eq!(line("box", "top|bottom", false), "box top");
        assert_eq!(line("hug", "a:word b:word", true), "pikachu hug piplup");
        assert_eq!(line("regroup", "*who [where:place]", true), "pikachu regroup piplup");
        assert_eq!(line("wait", "sound name:thing...", true), "wait sound Apple", "a form not said of someone: the word first");
        assert_eq!(line("ask", "[text...]", false), "ask");
        let (film, _) = example("walk", &["row|column *who".to_string(), "who:pokemon [to] where:place".to_string()], true);
        assert!(
            film.contains(
                "
pikachu walk right
"
            ),
            "the form said of someone: {film}"
        );
        let (film, at) = example("jump", &["who:pokemon".to_string()], true);
        assert_eq!(film.lines().nth(at - 1), Some("pikachu jump"));
        assert!(film.starts_with("scene: beach pikachu piplup\n"));
    }
}
