//! The floor itself: the team onto it, its log, the stairs found and taken,
//! the traps, the items on the ground, the map, the team led in file, the
//! bottom reached or the team beaten.

use super::*;

/// The stairs laid on the floor, if a line laid them.
pub(super) fn stairs(film: &Film) -> Option<String> {
    film.things.stuff.iter().rev().find(|stuff| stuff.key == STAIRS).map(|stuff| stuff.code.clone())
}

/// `stairs spot` (or `x y`): the game's stairs laid there, under all.
pub(super) fn lay_stairs(film: &mut Film, line: &Line) {
    // (`stairs take pikachu piplup`: the team takes them, as `take_stairs`.)
    if line.args.has("take") {
        takes_stairs(film, line);
        return;
    }
    let Some(thing) = thing_named(film, STAIRS) else {
        film.wrong(line.number, "no stairs image: scene_player dungeons".to_string());
        return;
    };
    let said = line.args.words("where").to_vec();
    let said = if said.is_empty() { vec!["right".to_string()] } else { said };
    let spot = somewhere(film, &said, line.number);
    // (Known on the floor, for `find_stairs` and `take_stairs`.)
    let code = thing_out(film, &thing);
    let at = found(film, &code, spot, &["exact"]);
    film.put_point_at(&code, at);
    film.on(&code, |who| emit::shadow(who, false));
    film.on(&code, emit::below);
    arrives(film, &code, &thing);
}

/// `take_stairs a b c [floor 4]`: each goes onto the stairs and is gone,
/// one after another; the stairs' sound; the floor fades to black, and the
/// next floor's name is told on it (the next scene comes out of it).
pub(super) fn takes_stairs(film: &mut Film, line: &Line) {
    let Some(code) = stairs(film) else {
        film.wrong(line.number, "no stairs on the floor: stairs <spot> first".to_string());
        return;
    };
    let those: Vec<String> = line.args.words("who").to_vec();
    for who in &those {
        // (Onto the stairs, and off the floor once gone: the next one walks
        // onto them too, not beside who stood there.)
        if let Some(who_of) = film.one(&code) {
            go(film, who, Spot::Of(who_of), "walk", true, "-", false);
        }
        film.on(who, |who| emit::opacity(who, 0.0, 10.0));
        film.wait(i64::from(ONE_BY_ONE.max(10)));
        film.on(who, emit::gone);
    }
    film.sound(STAIR_SOUND.to_string());
    floor_left(film);
    let floor = line.args.number("n").map(|floor| floor as i32);
    // (A dungeon that goes up: its floors are floors, not basements.)
    if line.args.has("up") {
        film.kept::<DungeonState>().going_up = true;
    }
    film.beat(emit::fade(true, false, 30.0));
    film.beat(emit::wait_fade());
    if let Some(floor) = floor {
        let name = film.kept::<DungeonState>().dungeon.clone().unwrap_or("Dungeon".to_string());
        film.kept::<DungeonState>().floor = floor;
        let under = floor_said(film, floor);
        let shown = format!("{}{}{under}", crate::screen::on_screen(&name), crate::screen::BREAK);
        film.caption(120.0, &shown, true);
        film.wait(120);
    }
}

/// The traps as the log names them, by their word.
pub(super) const TRAPS: [(&str, &str); 24] = [
    ("mud", "Mud Trap"),
    ("sticky", "Sticky Trap"),
    ("grimy", "Grimy Trap"),
    ("summon", "Summon Trap"),
    ("trapdoor", "Trapdoor"),
    ("warp", "Warp Trap"),
    ("gust", "Gust Trap"),
    ("spin", "Spin Trap"),
    ("slumber", "Slumber Trap"),
    ("slow", "Slow Trap"),
    ("seal", "Seal Trap"),
    ("poison", "Poison Trap"),
    ("selfdestruct", "Selfdestruct Trap"),
    ("explosion", "Explosion Trap"),
    ("pp_zero", "PP-Zero Trap"),
    ("chestnut", "Chestnut Trap"),
    ("wonder", "Wonder Tile"),
    ("pokemon", "Pokémon Trap"),
    ("spikes", "Spiked Tile"),
    ("stealth_rock", "Stealth Rock"),
    ("toxic_spikes", "Toxic Spikes"),
    ("trip", "Trip Trap"),
    ("random", "Random Trap"),
    ("grudge", "Grudge Trap"),
];

/// `trap a slumber` (any trap of `TRAPS`): he steps on it: it shows
/// under him, clicks, the log tells it, and it does what it does.
pub(super) fn trapped(film: &mut Film, line: &Line) {
    // (`trap off`: the traps laid fade from the floor.)
    if line.args.has("off") {
        for code in std::mem::take(&mut film.kept::<DungeonState>().traps) {
            film.on(&code, |thing| emit::opacity(thing, 0.0, 20.0));
            film.on(&code, |thing| emit::after(20.0, 0.0, emit::gone(thing)));
        }
        film.wait(20);
        return;
    }
    let who = crate::verbs::who_said(line);
    let who = who.as_str();
    let kind = line.args.word("kind").map(|word| plain(word).trim_start_matches("trap_").to_string()).unwrap_or_default();
    let Some((_, called_trap)) = TRAPS.iter().find(|(word, _)| *word == kind) else {
        let words: Vec<&str> = TRAPS.iter().map(|(word, _)| *word).collect();
        film.wrong(line.number, format!("trap a <kind>: {}", words.join(", ")));
        return;
    };
    let Some(thing) = thing_named(film, &format!("trap_{kind}")) else {
        film.wrong(line.number, format!("no trap_{kind} image: scene_player dungeons"));
        return;
    };
    let code = thing_apart(film, &thing);
    film.kept::<DungeonState>().traps.push(code.clone());
    let Some(under) = film.spot_of(who) else {
        return;
    };
    let at = found(film, &code, under, &["exact"]);
    film.put_point_at(&code, at);
    film.on(&code, |who| emit::shadow(who, false));
    film.on(&code, emit::below);
    arrives(film, &code, &thing);
    film.on(&code, |who| emit::opacity(who, 0.0, 0.0));
    film.on(&code, |who| emit::opacity(who, 1.0, 10.0));
    film.sound("click");
    film.emote(who, "exclaim", Some(30.0));
    film.jump(who, 4.0, 6.0);
    let name = film.called(who);
    logged(film, &format!("{name} stepped on a {called_trap}!"));
    film.wait(30);
    match kind.as_str() {
        "slumber" => {
            film.on(who, |who| emit::particles(engine::ParticleKind::Dust, Cam::Who(who), 20, Some(emit::colour(engine::Colour::Green))));
            film.on(who, |who| emit::pose(who, Pose::Sleep));
            film.on(who, |who| emit::status(who, emit::status_icon("sleep")));
            logged(film, &format!("{name} fell asleep!"));
        }
        "poison" | "toxic_spikes" => {
            film.on(who, |who| emit::particles(engine::ParticleKind::Bubbles, Cam::Who(who), 16, Some(emit::colour(engine::Colour::Purple))));
            film.on(who, |who| emit::status(who, emit::status_icon("poison")));
            logged(film, &format!("{name} was poisoned!"));
        }
        "spin" => {
            film.on(who, |who| emit::spin(who, 2.0, 40.0));
            film.on(who, |who| emit::status(who, emit::status_icon("confused")));
            logged(film, &format!("{name} became confused!"));
        }
        "slow" | "mud" | "pp_zero" | "seal" => {
            let colour = emit::colour(if kind == "mud" { engine::Colour::Brown } else { engine::Colour::Grey });
            film.on(who, |who| emit::particles(engine::ParticleKind::Dust, Cam::Who(who), 14, Some(colour)));
            film.on(who, |who| emit::status(who, emit::status_icon("lowered")));
            let said = match kind.as_str() {
                "slow" => "its Speed fell!",
                "mud" => "its Attack fell!",
                "pp_zero" => "its moves lost their PP!",
                _ => "a move was sealed!",
            };
            logged(film, &format!("{name}: {said}"));
        }
        "gust" => {
            film.on(who, |who| emit::particles(engine::ParticleKind::Dust, Cam::Who(who), 20, Some(emit::colour(engine::Colour::White))));
            film.sound("hop");
            film.on(who, |who| emit::spin(who, 1.0, 30.0));
            if let Some(beside) = film.spot_by(who, 48.0, 0.0) {
                let to = found(film, who, beside, &[]);
                film.on_at(who, to, |who, at| emit::hop_point(who, at, 30.0, 30.0));
            }
            logged(film, &format!("{name} was blown away by the gust!"));
        }
        "trapdoor" => {
            film.sound("shock");
            film.on(who, |who| emit::spin(who, 1.0, 20.0));
            film.on(who, |who| emit::opacity(who, 0.0, 20.0));
            logged(film, &format!("{name} fell to the next floor!"));
        }
        "warp" => {
            film.on(who, |who| emit::particles(engine::ParticleKind::Sparks, Cam::Who(who), 24, Some(emit::colour(engine::Colour::Green))));
            film.on(who, |who| emit::opacity(who, 0.0, 12.0));
            logged(film, &format!("{name} was warped!"));
        }
        "explosion" | "selfdestruct" => {
            film.beat(emit::flash(emit::light(engine::Colour::White), 6.0));
            film.beat(emit::quake(4.0, 30.0));
            film.sound("shock");
            film.on(who, |who| emit::particles(engine::ParticleKind::Smoke, Cam::Who(who), 40, None));
            film.on(who, |who| emit::popup(who, emit::colour(engine::Colour::Red), "-20"));
            health_changes(film, who, -20);
            film.emote(who, "shock", Some(40.0));
            logged(film, &format!("{name} was caught in the explosion!"));
        }
        "chestnut" | "spikes" | "stealth_rock" => {
            film.sound("shock");
            film.on(who, |who| emit::tremble(who, 20.0, 2.0));
            let much = if kind == "stealth_rock" { 12 } else { 8 };
            film.on(who, |who| emit::popup(who, emit::colour(engine::Colour::Red), format!("-{much}")));
            health_changes(film, who, -much);
            logged(film, &format!("{name} was hurt!"));
        }
        "sticky" | "grimy" | "trip" => {
            film.emote(who, "sweat", Some(40.0));
            let said = match kind.as_str() {
                "sticky" => "an item in the bag became sticky!",
                "grimy" => "the food in the bag became grimy!",
                _ => "it tripped!",
            };
            logged(film, &format!("{name}: {said}"));
        }
        _ => {
            film.emote(who, "question", Some(40.0));
            logged(film, "Nothing happened...");
        }
    }
    film.wait_frames(60, line.wait);
}
