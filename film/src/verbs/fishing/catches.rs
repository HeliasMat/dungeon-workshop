//! What was caught: measured against the record, let go, put in
//! the bucket, told at the end of the day, snatched by a bird, kept in a
//! photograph; a bottle uncorked; those watching.

use super::*;

/// Which of his catches a line names: the one it says, else his last
/// one still out of the bucket (or in it, `bucketed`).
pub(super) fn which(film: &mut Film, line: &Line, who: &str, asked: &str) -> Option<usize> {
    let named = line.args.word("catch").filter(|word| !digits(word)).map(plain);
    let caught = film.kept::<FishingState>().catches.get(who).and_then(|all| match &named {
        Some(word) => all.iter().rposition(|one| !one.released && (&one.word == word || &one.code == word)),
        None => all.iter().rposition(|one| !one.kept && !one.released),
    });
    if caught.is_none() {
        film.wrong(line.number, format!("{asked} {who} [catch]: nothing caught by him{}", named.map_or(String::new(), |word| format!(" that is \"{word}\""))));
    }
    caught
}

/// `release a [catch] [x y]`: his catch let go: it hops back into the water
/// where it came out (or there), a splash, rings, and it is gone.
pub(super) fn released(film: &mut Film, line: &Line) {
    let who = who_of(line);
    let Some(at) = which(film, line, &who, "release") else {
        return;
    };
    let one = film.kept::<FishingState>().catches[&who][at].clone();
    film.kept::<FishingState>().catches.get_mut(&who).expect("his")[at].released = true;
    let told = point_of(line);
    let Some((x, y)) = told.or(one.water) else {
        film.wrong(line.number, format!("release {who}: where is the water? release {who} x y"));
        return;
    };
    film.face(&who, &one.code);
    if one.pokemon {
        film.emote(&one.code, "joy", Some(40.0));
        floating(film, &one.code, engine::Colour::White, "Thank you!");
        film.wait(30);
    }
    // (Off a back, if it lay there.)
    film.on(&one.code, |code| emit::ride(code, None, 16.0, 0.0));
    film.on(&one.code, |code| emit::hop(code, x as f32, y as f32, 46.0, 44.0));
    film.sound("hop");
    film.wait(44);
    film.beat(drops_at(x, y, engine::ParticleKind::Drops, 22, Some(engine::Colour::Blue)));
    film.beat(drops_at(x, y, engine::ParticleKind::Rings, 3, None));
    film.sound("click");
    film.on(&one.code, emit::gone);
    film.emote(&who, "joy", Some(40.0));
    film.wait_frames(30, line.wait);
}

/// `spectators a b c`: they watch his fishing: glad at each catch (and a
/// leap on the line), sorry at each lost; `spectators off`.
pub(super) fn watched(film: &mut Film, line: &Line) {
    match () {
        _ if line.args.has("joy") => cheer(film, true),
        _ if line.args.has("sorrow") => cheer(film, false),
        _ => film.kept::<FishingState>().fans = line.args.words("who").to_vec(),
    }
}
