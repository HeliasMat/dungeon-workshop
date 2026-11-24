//! The line: baited, cast, something coming near, fought, pulled in (what
//! it nibbles, bites, snaps on are recipes: `film/recipes/fishing/line.film`).

use super::*;

/// `throw a line [x y]`: to that point of the water; none, before it. He
/// stays at the end of his cast, the rod out, until he does something else.
pub(super) fn cast(film: &mut Film, line: &Line) {
    let who = who_of(line);
    has_a_rod(film, &who, line);
    if line.args.has("on") {
        hooks_a_friend(film, line, &who, line.args.word("target").map(plain));
        return;
    }
    let Some(thing) = thing_named(film, BOBBER_PICTURE) else {
        film.wrong(line.number, "no bobber image in library/images".to_string());
        return;
    };
    let point = line.args.point("target").map(|(x, y)| (x.to_string(), y.to_string()));
    // (Cast again with his line still out: it is reeled in first, its bobber
    // gone from the water, not left floating there.)
    if let Some((old, _)) = film.kept::<FishingState>().fishing.remove(&who) {
        shadow_dives(film, &who);
        tension_off(film, &who, false);
        film.on(&who, |who| emit::fishline(who, None, None, true));
        film.on(&old, emit::gone);
    }
    // (Cast to a point: he turns to it first, whichever way it is, and casts
    // that way; none told, he casts the way he faces.)
    if let Some((x, y)) = &point {
        match numbers(x, y) {
            Some((x, y)) => film.on(&who, |who| emit::look_at(who, x, y)),
            None => film.on_point(&who, x, y, emit::look_at_point),
        }
    }
    keeps_his_way(film, &who);
    film.anim(&who, "Swing", false);
    film.wait(i64::from(SWUNG));
    let code = thing_apart(film, &thing);
    with_other(film, &code, &who, |code, who| emit::put_near(code, who, engine::Direction::Down));
    // (On the water, it has its rings for a shadow.)
    film.on(&code, |code| emit::shadow(code, false));
    // (Cast off a back out at sea: on open water, wherever it falls.)
    if film.staging.mounts.contains_key(&who) {
        film.on(&code, |code| emit::flies(vec![code]));
    }
    film.sound("hop");
    let at = point.as_ref().map(|(x, y)| numbers(x, y));
    match (at, film.other(&code)) {
        (Some(Some((x, y))), Some(to)) => film.on(&who, |who| emit::fishline(who, Some(to), Some(Vec2::new(x, y)), true)),
        (None, Some(to)) => film.on(&who, |who| emit::fishline(who, Some(to), None, true)),
        _ => film.unread(format!("{who} {code}")),
    }
    film.kept::<FishingState>().fishing.insert(who.clone(), (code.clone(), point));
    film.kept::<FishingState>().last_cast = Some(who.clone());
    film.wait(i64::from(CAST));
    film.sound("click");
    drops(film, &code, engine::ParticleKind::Drops, 12, Some(engine::Colour::Blue));
    drops(film, &code, engine::ParticleKind::Rings, 2, None);
    film.on(&code, |code| emit::float(code, 1.0, 70.0));
    film.on(&code, |code| emit::emitter(engine::ParticleKind::Rings, code, RESTING_RINGS, Some(emit::colour(engine::Colour::White)), None));
    film.wait_frames(20, line.wait);
}

/// `throw a line on b`: his cast goes wrong: the hook catches the friend
/// behind him, who yelps; the line comes off, nothing is cast.
fn hooks_a_friend(film: &mut Film, line: &Line, who: &str, friend: Option<String>) {
    let Some(friend) = friend.filter(|friend| film.actor(friend).is_some()) else {
        film.wrong(line.number, format!("throw {who} line on <who>: whom he hooks"));
        return;
    };
    keeps_his_way(film, who);
    film.anim(who, "Swing", false);
    film.wait(i64::from(SWUNG));
    film.sound("hop");
    with_other(film, who, &friend, |who, friend| emit::fishline(who, Some(friend), None, false));
    film.wait(10);
    film.sound("shock");
    film.jump(&friend, 10.0, 10.0);
    film.on(&friend, |friend| emit::tremble(friend, 30.0, 2.0));
    floating(film, &friend, engine::Colour::Red, "Ouch!");
    film.emote(&friend, "anger", Some(50.0));
    film.wait(30);
    film.on(who, |who| emit::fishline(who, None, None, true));
    film.on(who, |who| emit::pose(who, Pose::Idle));
    film.emote(who, "sweat", Some(40.0));
    free_to_turn(film, who);
    film.wait_frames(20, line.wait);
}

/// Where a catch comes out: on the bobber, at the point it was cast to;
/// cast to no point, just by it.
pub(super) fn out_of_the_water(film: &mut Film, caught: &str, code: &str, at: &Option<(String, String)>) {
    match at {
        Some((x, y)) => match numbers(x, y) {
            Some((x, y)) => film.on(caught, |caught| emit::put(caught, x, y, engine::Direction::Down)),
            None => film.put_at(caught, x, y),
        },
        None => with_other(film, caught, code, |caught, code| emit::put_near(caught, code, engine::Direction::Down)),
    }
}

/// What comes out of the water with a pull, where the bobber was: someone, or
/// a thing; where it lands.
pub(super) fn catch(film: &mut Film, line: &Line, word: Option<String>, who: &str, code: &str, at: &Option<(String, String)>) -> Option<(String, Point)> {
    let word = word?;
    // (Where it lands: on free ground by the point told after it, or beside him.)
    let beside = match point_of(line) {
        Some((x, y)) => engine::Spot::Written(crate::spots::xy(x, y)),
        None => film.spot_by(who, 34.0, 12.0)?,
    };
    if film.actor(&word).is_some() {
        // (Out at sea on a back, a point told is on the water: there it lands,
        // and swims, not carried off to the nearest shore.)
        if let (true, Some((x, y))) = (film.staging.mounts.contains_key(who), point_of(line)) {
            out_of_the_water(film, &word, code, at);
            film.on(&word, |word| emit::flies(vec![word]));
            return Some((word, crate::spots::xy(x, y)));
        }
        let lands = found(film, &word, beside, &["apart"]);
        out_of_the_water(film, &word, code, at);
        return Some((word, lands));
    }
    let Some(thing) = known_thing(film, std::slice::from_ref(&word)) else {
        film.wrong(line.number, format!("what comes out of the water? \"{word}\": someone or an item"));
        return None;
    };
    let caught = thing_out(film, &thing);
    out_of_the_water(film, &caught, code, at);
    arrives(film, &caught, &thing);
    set_owner(film, &caught, None);
    let lands = found(film, &caught, beside, &[]);
    Some((caught, lands))
}

/// `haul a [krabby | Apple | random] [x y] [heavy]`: fought, the rod bent in his pull, then what was
/// on the line flies up out of the water and lands beside; or nothing comes,
/// and the bobber does.
pub(super) fn pulled(film: &mut Film, line: &Line) {
    let who = who_of(line);
    let Some(code) = line_of(film, &who, line) else {
        return;
    };
    keeps_his_way(film, &who);
    shadow_dives(film, &who);
    let heavy = line.args.has("heavy");
    let slips = line.args.has("unhooks");
    // (By chance: the same for the same line of the same film.)
    let word = line.args.word("catch").map(plain).and_then(|word| match word.as_str() {
        "random" => {
            // (What bites by chance: by his bait, if he put one on.)
            let bait = film.kept::<FishingState>().bait.get(&who).map(|bait| plain(bait)).unwrap_or_default();
            let table: &[&str] = BAITED.iter().find(|(kind, _)| bait.contains(kind)).map_or(&CATCHES, |(_, table)| table);
            let drawn = table[(line.number * 7 + film.kept::<FishingState>().bait.len() * 3) % table.len()];
            film.remark(line.number, format!("{who} fishes at random: {}", if drawn == "-" { "nothing" } else { drawn }));
            (drawn != "-").then(|| drawn.to_string())
        }
        _ => Some(word),
    });
    let fought = if heavy { HEAVY } else { FOUGHT };
    film.anim(&who, "Pull", false);
    let strength = if heavy { 2.0 } else { 1.0 };
    film.on(&who, |who| emit::tremble(who, fought as f32, strength));
    // (The bobber shaken on the water, what is under fighting.)
    film.on(&code, |code| emit::tremble(code, fought as f32, 2.0));
    film.sound("hop");
    drops(film, &code, engine::ParticleKind::Rings, 2, None);
    if heavy {
        // (A heavy one: the ground shakes, he sweats, the water boils twice.)
        film.beat(emit::quake(3.0, fought as f32));
        film.emote(&who, "sweat", Some(60.0));
        let (bubbles, splash) = ((engine::ParticleKind::Bubbles, None), (engine::ParticleKind::Drops, Some(emit::colour(engine::Colour::Blue))));
        film.on(&code, |code| emit::after((fought / 3) as f32, 0.0, emit::particles(bubbles.0, Cam::Who(code), 10, bubbles.1)));
        film.on(&code, |code| emit::after((fought * 2 / 3) as f32, 0.0, emit::particles(splash.0, Cam::Who(code), 20, splash.1)));
        // (Riding, the fish tows his mount out toward it, by the water.)
        if let Some(mount) = film.staging.mounts.get(&who).cloned() {
            let from = film.kept::<FishingState>().swum.get(&mount).copied();
            for at in 1..=3 {
                let when = (fought * at / 4) as f32;
                if let Some((x, y)) = from {
                    film.on(&mount, |mount| emit::after(when, 0.0, emit::go(mount, x as f32, (y - TOWED * at) as f32, 12.0, true)));
                }
                film.on(&mount, |mount| emit::after(when, 0.0, emit::particles(splash.0, Cam::Who(mount), 12, splash.1)));
            }
            if let Some((x, y)) = from {
                film.kept::<FishingState>().swum.insert(mount.clone(), (x, y - TOWED * 3));
            }
            floating(film, &mount, engine::Colour::White, "It's pulling us!");
        }
    }
    film.wait(i64::from(fought / 2));
    drops(film, &code, engine::ParticleKind::Drops, 14, Some(engine::Colour::Blue));
    drops(film, &code, engine::ParticleKind::Rings, 2, None);
    film.wait(i64::from(fought / 2));
    // (The splash of what comes out.)
    drops(film, &code, engine::ParticleKind::Drops, 40, Some(engine::Colour::Blue));
    drops(film, &code, engine::ParticleKind::Bubbles, 12, None);
    drops(film, &code, engine::ParticleKind::Rings, 4, None);
    let at = film.kept::<FishingState>().fishing.remove(&who).and_then(|(_, at)| at);
    let held = catch(film, line, word, &who, &code, &at);
    let bait = film.kept::<FishingState>().bait.remove(&who);
    tension_off(film, &who, false);
    film.on(&code, emit::gone);
    film.sound("shock");
    film.on(&who, |who| emit::pose(who, Pose::Idle));
    let water = at.as_ref().and_then(|(x, y)| Some((x.parse::<i32>().ok()?, y.parse::<i32>().ok()?)));
    match held {
        // (Off the hook in the air: it falls back into the water, by where it
        // came out, and is gone.)
        Some((caught, _)) if slips => {
            let (x, y) = water.unwrap_or((0, 0));
            let (x, y) = (x + 14, y + 4);
            with_other(film, &who, &caught, |who, caught| emit::fishline(who, Some(caught), None, false));
            film.on(&caught, |caught| emit::hop(caught, x as f32, y as f32, (CATCH_HEIGHT / 2) as f32, CATCH_FRAMES as f32));
            film.on(&caught, |caught| emit::tremble(caught, CATCH_FRAMES as f32, 3.0));
            film.wait(i64::from(CATCH_FRAMES / 3));
            film.on(&who, |who| emit::fishline(who, None, None, true));
            film.emote(&who, "shock", Some(50.0));
            film.wait(i64::from(CATCH_FRAMES * 2 / 3));
            film.beat(drops_at(x, y, engine::ParticleKind::Drops, 24, Some(engine::Colour::Blue)));
            film.beat(drops_at(x, y, engine::ParticleKind::Rings, 3, None));
            film.sound("click");
            film.on(&caught, emit::gone);
            floating(film, &who, engine::Colour::White, "It got away...");
            cheer(film, false);
            if water.is_none() {
                film.remark(line.number, format!("haul {who} ... unhooks: throw {who} line x y, for the catch to fall back there"));
            }
        }
        Some((caught, lands)) => {
            let (name, word, pokemon) = caught_name(film, &caught);
            // (Where it lands, when the line says: the rest is found as the film plays.)
            let landed = point_of(line).or_else(|| written(&lands).map(|(x, y)| (x as i32, y as i32)));
            film.kept::<FishingState>().catches.entry(who.clone()).or_default().push(Caught {
                code: caught.clone(),
                name,
                word,
                pokemon,
                size: None,
                kept: false,
                released: false,
                water,
                landed,
            });
            cheer(film, true);
            // (The line tied to it where it comes out, not cast: it flies
            // from the water to beside him.)
            with_other(film, &who, &caught, |who, caught| emit::fishline(who, Some(caught), None, false));
            let on_back = film.staging.mounts.get(&who).cloned().filter(|_| point_of(line).is_none());
            match (&on_back, written(&lands)) {
                // (Riding, and no point told: onto the back he rides, behind him.)
                (Some(mount), _) => with_other(film, &caught, mount, |caught, mount| emit::throw(caught, mount, CATCH_HEIGHT as f32, CATCH_FRAMES as f32)),
                (None, Some((x, y))) => film.on(&caught, |caught| emit::hop(caught, x, y, CATCH_HEIGHT as f32, CATCH_FRAMES as f32)),
                (None, None) => film.on_at(&caught, lands.clone(), |who, at| emit::hop_point(who, at, CATCH_HEIGHT as f32, CATCH_FRAMES as f32)),
            }
            // (It wriggles on the line as it flies.)
            film.on(&caught, |caught| emit::tremble(caught, CATCH_FRAMES as f32, 2.0));
            // (The line let go just before it lands: never lying on the ground.)
            film.wait(i64::from(CATCH_FRAMES - 4));
            film.on(&who, |who| emit::fishline(who, None, None, true));
            film.wait(4);
            if let Some(mount) = &on_back {
                with_other(film, &caught, mount, |caught, mount| emit::ride(caught, Some(mount), BACK_CATCH_HIGH as f32, BACK_CATCH_SIDE as f32));
            }
            film.sound("shock");
            // (It drips a while.)
            let dripping = emit::frames_read("2s").map(|frames| frames as f32);
            film.on(&caught, |caught| emit::emitter(engine::ParticleKind::Drops, caught, 6.0, Some(emit::colour(engine::Colour::Blue)), dripping));
        }
        None => {
            film.on(&who, |who| emit::fishline(who, None, None, true));
            // (His bait gone with what ate it.)
            if bait.is_some() {
                floating(film, &who, engine::Colour::White, "The bait is gone...");
            }
        }
    }
    free_to_turn(film, &who);
    film.wait_frames(20, line.wait);
}

/// What bites by chance on each bait: an apple, the fish that like it; a
/// berry, those of the reefs; a gummi, the rare ones; a seed, the small.
const BAITED: [(&str, &[&str]); 4] = [
    ("apple", &["magikarp", "magikarp", "remoraid", "seaking", "Apple", "-"]),
    ("berry", &["staryu", "shellder", "qwilfish", "krabby", "oranberry", "-"]),
    ("gummi", &["lapras", "gyarados", "seaking", "chinchou", "barboach", "-"]),
    ("seed", &["poliwag", "chinchou", "magikarp", "-"]),
];

/// How far a heavy one tows the mount he rides, at each pull.
const TOWED: i32 = 6;

/// Where a catch sits on the back he rides: how high, how far behind its middle
/// (behind him).
const BACK_CATCH_HIGH: i32 = 8;
const BACK_CATCH_SIDE: i32 = 20;

/// What came out of the water: its name as said, its word, whether a Pokémon.
fn caught_name(film: &mut Film, caught: &str) -> (String, String, bool) {
    if film.actor(caught).is_some() {
        return (crate::verbs::dungeon::called_by(film, caught), caught.to_string(), true);
    }
    let key = film.things.stuff.iter().find(|stuff| stuff.code == caught).map_or(caught.to_string(), |stuff| stuff.key.clone());
    let name = crate::screen::capitalized(&known_thing(film, std::slice::from_ref(&key)).map_or(key.clone(), |thing| thing.name));
    (name, key, false)
}
