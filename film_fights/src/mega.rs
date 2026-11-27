//! `charizard mega_evolve x`: his stone glints, two rainbow strands wind
//! round him, a flash of a few frames, a burst, and he is his mega form;
//! still himself, his name and his lines his own, his mega form's portraits.
//! `charizard mega_evolve back`: white, a flash, himself again.

use film_core::emit;
use film_core::film::Film;
use film_core::names::species;
use film_core::verbs::Line;
use film_core::words::plain;

pub(crate) fn mega_evolved(film: &mut Film, line: &Line) {
    let who = plain(line.args.word("who").unwrap_or_default());
    let number = line.number;
    if line.args.has("back") {
        if !film.staging.megas.remove(&who) {
            film.remark(number, format!("{who} mega evolves back: he was not mega evolved (played all the same)"));
        }
        film.call(number, "_unmega", &[&who]);
        film.on(&who, |one| emit::disguise(one, None));
        film.call(number, "_unmega_done", &[&who]);
        return;
    }
    if film.staging.megas.contains(&who) {
        film.remark(number, format!("{who} is mega evolved already (played all the same)"));
    }
    let side = ["x", "y"].into_iter().find(|side| line.args.has(side));
    let Some(form) = mega_form(film, &who, side, number) else { return };
    let Some(like) = film.coded(&form) else { return };
    film.call(number, "_mega", &[&who]);
    film.on(&who, |one| emit::disguise(one, Some(like)));
    film.staging.megas.insert(who.clone());
    film.call(number, "_mega_done", &[&who]);
}

/// The word of someone's mega form (`charizard_mega_x`), as SpriteCollab
/// names them: `<species>_mega`, `_mega_x`, `_mega_y`; one with a sprite
/// (many have only a portrait). None: told.
fn mega_form(film: &mut Film, who: &str, side: Option<&str>, number: usize) -> Option<String> {
    let code = crate::lines::code_now(film, who);
    let base = species(&code).and_then(|kind| film.word_of_number(kind)).unwrap_or_default();
    // (A form the tables know, and whether it has a sprite.)
    let form = |film: &Film, end: &str| {
        let word = format!("{base}_mega{end}");
        let row = film.tables.monsters.get(&word)?;
        Some((word, film.tables.sprite_groups.contains_key(&row.0)))
    };
    let asked = side.map_or(String::new(), |side| format!("_{side}"));
    let forms: Vec<(String, bool)> = [asked.as_str(), "", "_x", "_y"].into_iter().filter_map(|end| form(film, end)).collect();
    let Some((first, _)) = forms.first().cloned() else {
        let side = side.map_or(String::new(), |side| format!(" {side}"));
        film.remark(number, format!("{who} has no mega form{side} (a {base}): left out; the species with one are those of `scene_player search mega`"));
        return None;
    };
    let Some((drawn, _)) = forms.iter().find(|(_, sprite)| *sprite).cloned() else {
        film.remark(number, format!("{who}'s mega form {first} has no sprite in data/extra (a portrait only): left out"));
        return None;
    };
    let wanted = match side {
        Some(side) => format!("{base}_mega_{side}"),
        None => drawn.clone(),
    };
    if drawn != wanted {
        let why = if forms.iter().any(|(word, _)| *word == wanted) { "has no sprite" } else { "is not a form of his" };
        film.remark(number, format!("{who}: {wanted} {why}, played as {drawn}"));
    } else if side.is_none() && forms.iter().any(|(word, _)| word.ends_with("_y")) && drawn.ends_with("_x") {
        film.remark(number, format!("{who} has two mega forms: say x or y (played as x)"));
    }
    Some(drawn)
}
