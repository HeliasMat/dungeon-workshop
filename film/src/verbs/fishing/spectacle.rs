//! The sea's big moments: a school leaping, a giant's shadow passing under,
//! a whirlpool, a treasure fished up and opened, gulls, a storm, a chase.

use super::*;

/// A school leaping: how many when the line does not say, how far each
/// leaps, how high, how long, how long after the one before.
const SCHOOL: usize = 8;
const LEAP_ALONG: i32 = 30;
const LEAP_UP: i32 = 30;
const LEAP_TIME: i32 = 36;
const ONE_AFTER: i32 = 7;

/// `jump school x y [n] [left]`: a school of little fish leaps out of the
/// water there, one after the other, each in its arc, a splash going in and
/// coming out; to the right, or the left.
pub(super) fn school_leaps(film: &mut Film, line: &Line) {
    let Some((x, y)) = point_of(line) else {
        film.wrong(line.number, "jump school x y [number] [left]: where the school leaps".to_string());
        return;
    };
    let count = line.args.number("n").map_or(SCHOOL, |count| count as usize).clamp(1, 20);
    let left = line.args.has("left");
    let Some(fish) = thing_named(film, if left { "small_fish" } else { "small_fish_r" }) else {
        film.wrong(line.number, "no small_fish image in library/images".to_string());
        return;
    };
    let way = if left { -1 } else { 1 };
    film.sound("hop");
    for one in 0..count {
        let code = thing_apart(film, &fish);
        let (from_x, from_y) = (x + way * one as i32 * 6, y + [0, 3, -2, 4, -3, 1][one % 6]);
        let to_x = from_x + way * LEAP_ALONG;
        let at = one as i32 * ONE_AFTER + 1;
        // (There from the start, unseen under the water, till its leap.)
        film.on(&code, |code| emit::put(code, from_x as f32, from_y as f32, engine::Direction::Down));
        film.on(&code, |code| emit::shadow(code, false));
        film.on(&code, |code| emit::opacity(code, 0.0, 0.0));
        film.on(&code, |code| emit::anim(code, engine::AnimId(0), true, false));
        let (out, back) = (at as f32, (at + LEAP_TIME) as f32);
        film.on(&code, |code| emit::after(out, 0.0, emit::opacity(code, 1.0, 0.0)));
        film.beat(emit::after(out, 0.0, drops_at(from_x, from_y, engine::ParticleKind::Drops, 8, Some(engine::Colour::Blue))));
        let high = (LEAP_UP + (one as i32 % 3) * 4) as f32;
        film.on(&code, |code| emit::after(out, 0.0, emit::hop(code, to_x as f32, from_y as f32, high, LEAP_TIME as f32)));
        film.beat(emit::after(back, 0.0, drops_at(to_x, from_y, engine::ParticleKind::Drops, 10, Some(engine::Colour::Blue))));
        film.beat(emit::after(back, 0.0, drops_at(to_x, from_y, engine::ParticleKind::Rings, 1, None)));
        film.on(&code, |code| emit::after(back, 0.0, emit::gone(code)));
    }
    cheer(film, true);
    film.wait_frames(count as i32 * ONE_AFTER + LEAP_TIME + 10, line.wait);
}

/// `shadow giant x y to x y [in 4s]`: the shadow of something huge passes
/// under the water there, three times as big as a big fish; the sea shudders,
/// those watching sweat.
pub(super) fn giant_passes(film: &mut Film, line: &Line) {
    let (Some((x, y)), Some((to_x, to_y))) = (point_of(line), point_named(line, "x2", "y2")) else {
        film.wrong(line.number, "shadow giant x y to x y [in 4s]: where it passes from, and how far".to_string());
        return;
    };
    let frames = line.args.frames("d").unwrap_or(240);
    let Some(shadow) = thing_named(film, if to_x > x { "big_shadow_r" } else { "big_shadow" }) else {
        film.wrong(line.number, "no big_shadow image in library/images".to_string());
        return;
    };
    let code = thing_apart(film, &shadow);
    film.on(&code, |code| emit::put(code, x as f32, y as f32, engine::Direction::Down));
    film.on(&code, |code| emit::shadow(code, false));
    film.on(&code, emit::below);
    film.on(&code, |code| emit::scale(code, 3.0, 0.0));
    film.on(&code, |code| emit::anim(code, engine::AnimId(0), true, false));
    film.on(&code, |code| emit::go(code, to_x as f32, to_y as f32, frames as f32, true));
    let third = (frames / 3) as f32;
    film.beat(emit::after(third, 0.0, emit::quake(1.0, 40.0)));
    film.beat(emit::after(third, 0.0, emit::sound(engine::SoundId(rules::songs::noise::STUNNED))));
    for one in film.kept::<FishingState>().fans.clone() {
        film.on(&one, |one| emit::after(third, 0.0, emit::emote(one, scenes::vocabulary::emote::SWEAT, Some(60.0))));
    }
    film.wait(i64::from(frames));
    film.on(&code, emit::gone);
    film.wait_frames(10, line.wait);
}

/// `throw lapras water to b`: it blows a spout of water up over itself, the drops
/// falling round it; on b, who gets soaked.
pub(super) fn spouts(film: &mut Film, line: &Line) {
    let who = who_of(line);
    film.jump(&who, 3.0, 8.0);
    film.sound("hop");
    let (drop, blue, white) = (engine::ParticleKind::Drops, Some(emit::colour(engine::Colour::Blue)), Some(emit::colour(engine::Colour::White)));
    for at in 0..6 {
        let at = (at * 6 + 1) as f32;
        film.on(&who, |who| emit::after(at, 0.0, emit::particles(drop, Cam::Who(who), 14, blue)));
        film.on(&who, |who| emit::after(at, 0.0, emit::particles(drop, Cam::Who(who), 6, white)));
    }
    if let Some(wet) = line.args.word("target").map(plain).filter(|word| film.actor(word).is_some()) {
        film.on(&wet, |wet| emit::after(24.0, 0.0, emit::particles(drop, Cam::Who(wet), 20, blue)));
        film.on(&wet, |wet| emit::after(24.0, 0.0, emit::tremble(wet, 30.0, 2.0)));
        film.on(&wet, |wet| emit::after(26.0, 0.0, emit::popup(wet, emit::colour(engine::Colour::White), "Pshhh!")));
        film.on(&wet, |wet| emit::after(30.0, 0.0, emit::emote(wet, scenes::vocabulary::emote::SWEAT, Some(50.0))));
    }
    film.wait_frames(50, line.wait);
}

/// `jump lapras wave`: it leaps over a wave, ahead the way it faces, those
/// on its back with it; a splash where it comes down.
pub(super) fn leaps_a_wave(film: &mut Film, line: &Line) {
    let who = who_of(line);
    let Some((x, y)) = film.kept::<FishingState>().swum.get(&who).copied() else {
        film.wrong(line.number, format!("jump {who} wave: where is he? (swim {who} to x y, before)"));
        return;
    };
    let ahead = if film.kept::<FishingState>().swims_right.get(&who).copied().unwrap_or(false) { 40 } else { -40 };
    drops(film, &who, engine::ParticleKind::Drops, 16, Some(engine::Colour::Blue));
    film.sound("hop");
    film.on(&who, |who| emit::hop(who, (x + ahead) as f32, y as f32, 30.0, 40.0));
    film.wait(40);
    drops(film, &who, engine::ParticleKind::Drops, 24, Some(engine::Colour::Blue));
    drops(film, &who, engine::ParticleKind::Rings, 3, None);
    film.sound("click");
    film.kept::<FishingState>().swum.insert(who, (x + ahead, y));
    cheer(film, true);
    film.wait_frames(20, line.wait);
}
