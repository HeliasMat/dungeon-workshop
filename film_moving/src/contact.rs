//! Two bodies that meet: a shove of sumo (`push`), which only picks who
//! wins; the rest is the library's recipes (`film/recipes/contact.film`:
//! `_sumo`, `tug`, `tickle`, `chomp`, `high_five`, `boost`, `lift`).

use film_core::film::Film;
use film_core::verbs::{Line, Word};
use film_core::words::plain;

pub const WORDS: &[Word] = &[Word::new(
    "push",
    &["who:pokemon other:pokemon [wins winner:pokemon]"],
    "the recipe _sumo, the winner first: the heavier by the game's tables (none known: who pushes), or who wins says",
)
.family("Body")
.ds_close("its words: both pull and tremble, the loser hurt (no knock back) and tumbling", "no knock back: the loser tumbles where he stands")
.does(pushed)];

/// Its words, a group of the language (`Lexicon`).
pub const GROUP: film_core::verbs::Group = film_core::verbs::Group::new("contact", WORDS);

/// `a push b [wins b]`: they lock and strain, and one throws the other
/// back: who the line says, else the heavier, else a.
fn pushed(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let other = plain(line.args.word("other").unwrap_or_default());
    let winner = match line.args.word("winner").map(plain) {
        Some(winner) if winner == who || winner == other => winner,
        Some(winner) => {
            film.wrong(line.number, format!("“{winner}” wins: he is not in the shove ({who} or {other})"));
            who.clone()
        }
        None => {
            let weight = |film: &mut Film, one: &str| film.actor(one).and_then(|code| film.weight_of(&code));
            match (weight(film, &who), weight(film, &other)) {
                (Some(mine), Some(theirs)) if theirs > mine => other.clone(),
                _ => who.clone(),
            }
        }
    };
    let loser = if winner == who { other } else { who };
    film.call(line.number, "_sumo", &[&winner, &loser]);
}
