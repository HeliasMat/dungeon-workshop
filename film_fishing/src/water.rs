//! What goes on in the water by itself: a leap; a fish's shadow coming up to
//! a bobber.

use super::*;

/// `jump magikarp water x y`: one leaps out of the water there and dives back
/// in a little farther: out of the water and back into it, never on the shore.
pub(super) fn leaps(film: &mut Film, line: &Line) {
    let who = line.args.word("who").map(plain).filter(|word| film.actor(word).is_some());
    let (Some(who), Some((x, y))) = (who, point_of(line)) else {
        film.wrong(line.number, "jump <pokemon> water x y: who leaps, and where in the water".to_string());
        return;
    };
    let code = who.clone();
    let lands = x + if (x / 8) % 2 == 0 { LEAP } else { -LEAP };
    let way = if lands > x { engine::Direction::Right } else { engine::Direction::Left };
    film.on(&code, |code| emit::put(code, x as f32, y as f32, way));
    film.on(&code, |code| emit::shadow(code, false));
    film.beat(drops_at(x, y, engine::ParticleKind::Drops, 16, Some(engine::Colour::Blue)));
    film.beat(drops_at(x, y, engine::ParticleKind::Rings, 2, None));
    film.sound("hop");
    film.on(&code, |code| emit::hop(code, lands as f32, y as f32, LEAP_HEIGHT as f32, LEAP_FRAMES as f32));
    film.wait(i64::from(LEAP_FRAMES));
    film.beat(drops_at(lands, y, engine::ParticleKind::Drops, 20, Some(engine::Colour::Blue)));
    film.beat(drops_at(lands, y, engine::ParticleKind::Rings, 3, None));
    film.sound("splash");
    film.on(&code, emit::gone);
    film.wait_frames(20, line.wait);
}

/// How a shadow comes up: the frames of its way to the bobber, of its turn.
const SHADOW_COMES: i32 = 90;
const SHADOW_TURNS: i32 = 40;

/// `shadow a [big] [x y]`: a fish's shadow comes up under the water from
/// there (from farther out, none told), past his bobber, turns, and waits
/// under it, its tail beating (`shadow a flees`: it darts off and is gone). It
/// dives when something bites, or the line comes in.
pub(super) fn shadow(film: &mut Film, line: &Line) {
    let who = who_of(line);
    let Some((x, y)) = bobber_at(film, &who) else {
        film.wrong(line.number, format!("shadow {who}: his bobber first, at a point of the water (throw {who} line x y)"));
        return;
    };
    let big = line.args.has("big");
    let from = point_of(line).unwrap_or((x - 120, y - 8));
    // (Its picture by the way it swims: it turns past the bobber, and comes
    // back under it the other way.)
    let rightwards = from.0 < x;
    let picture = |right: bool| format!("ombre_{}{}", if big { "grande" } else { "poisson" }, if right { "_d" } else { "" });
    let (Some(going), Some(back)) = (thing_named(film, &picture(rightwards)), thing_named(film, &picture(!rightwards))) else {
        film.wrong(line.number, "no fish_shadow image in library/images".to_string());
        return;
    };
    let side = if rightwards { 1 } else { -1 };
    let first = thing_apart(film, &going);
    film.on(&first, |first| emit::put(first, from.0 as f32, from.1 as f32, engine::Direction::Down));
    film.on(&first, |first| emit::shadow(first, false));
    film.on(&first, emit::below);
    film.on(&first, |first| emit::anim(first, engine::AnimId(0), true, false));
    film.on(&first, |first| emit::go(first, (x + side * 22) as f32, (y + 5) as f32, SHADOW_COMES as f32, true));
    film.wait(i64::from(SHADOW_COMES));
    film.on(&first, emit::gone);
    let second = thing_apart(film, &back);
    film.on(&second, |second| emit::put(second, (x + side * 22) as f32, (y + 5) as f32, engine::Direction::Down));
    film.on(&second, |second| emit::shadow(second, false));
    film.on(&second, emit::below);
    film.on(&second, |second| emit::anim(second, engine::AnimId(0), true, false));
    film.on(&second, |second| emit::go(second, (x + side * 4) as f32, (y + 4) as f32, SHADOW_TURNS as f32, true));
    film.beat(drops_at(x, y, engine::ParticleKind::Rings, 1, None));
    film.kept::<FishingState>().shadows.insert(who, second);
    film.wait_frames(SHADOW_TURNS, line.wait);
}

/// The shadow under his bobber, if one waits there, darts off and is gone.
pub(super) fn flees(film: &mut Film, who: &str) -> bool {
    let (Some(code), Some((x, y))) = (film.kept::<FishingState>().shadows.get(who).cloned(), bobber_at(film, who)) else {
        return false;
    };
    film.kept::<FishingState>().shadows.remove(who);
    let away = if (x / 8) % 2 == 0 { 130 } else { -130 };
    film.beat(drops_at(x, y, engine::ParticleKind::Rings, 2, None));
    film.on(&code, |code| emit::go(code, (x + away) as f32, (y + 12) as f32, 24.0, true));
    film.wait(24);
    film.on(&code, emit::gone);
    true
}
