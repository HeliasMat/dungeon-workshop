//! What befalls the team on a floor: a Monster House, a wild one, the wind,
//! the orbs, Poké, the client found, enemies beaten or joining, one of the
//! team knocked out, hunger.

use super::*;

/// `ambush zubat koffing on pikachu`: foes leap down round him from
/// above, as wild Pokémon ambush a team in the game: the alarm, the ground
/// shaking, each falling in turn in its dust, then face to face.
pub(super) fn ambush(film: &mut Film, line: &Line) {
    let who = film_core::verbs::who_said(line);
    let enemies: Vec<String> = line.args.words("foes").to_vec();
    logged(film, "Wild Pokémon ambushed the team!");
    drop_around(film, &who, &enemies);
    for enemy in &enemies {
        film.emote(enemy, "anger", Some(40.0));
    }
    film.wait_frames(40, line.wait);
}

/// They drop from the ceiling round him, one after another.
fn drop_around(film: &mut Film, who: &str, enemies: &[String]) {
    film.sound("alert");
    film.beat(emit::quake(3.0, 40.0));
    film.emote(who, "exclaim", Some(40.0));
    for (at, enemy) in enemies.iter().enumerate() {
        let angle = at as f32 / enemies.len() as f32 * std::f32::consts::TAU;
        let (dx, dy) = ((angle.cos() * 60.0) as i32, (angle.sin() * 38.0) as i32);
        let Some(by) = film.spot_by(who, dx as f32, dy as f32) else {
            continue;
        };
        let to = found(film, enemy, by, &["apart"]);
        film.put_point_at(enemy, to);
        film.face(enemy, who);
        film.on(enemy, |who| emit::rise(who, 120.0, 1.0));
        film.on(enemy, |who| emit::after((at * 8) as f32, 0.0, emit::rise(who, 0.0, 18.0)));
        film.on(enemy, |who| emit::after((at * 8 + 18) as f32, 0.0, emit::particles(engine::ParticleKind::Dust, Cam::Who(who), 8, None)));
    }
    film.beat(emit::wait((enemies.len() * 8 + 20) as u32));
    film.sound("shock");
}
