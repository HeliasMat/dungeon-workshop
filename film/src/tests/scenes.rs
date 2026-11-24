//! What is said, the game's pictures and rituals, crowds.

use super::*;

#[test]
fn the_games_colours() {
    let lines = clean(&scene(&["krabby: I seek Piplup and the Apple."]));
    let said: Vec<&String> = lines.iter().flatten().filter(|field| field.contains("I seek")).collect();
    assert_eq!(said, [" I seek [CS:N]Piplup[CR] and the [CS:I]Apple[CR]."]);
    let lines = clean(&scene(&["krabby: Do you remember what we saw in Beach Cave?"]));
    assert!(lines.concat().join("").contains("Cave[CR]"));
}

#[test]
fn one_not_known_yet_and_rituals_of_words() {
    let lines = clean(&scene(&["???: Who goes there?", "krabby?: You."]));
    let said: Vec<&str> = lines.iter().filter(|fields| starts(fields, &["say"])).map(|fields| fields[1].as_str()).collect();
    let krabby = format!("m{}:???", crate::tests::tables().monsters["krabby"].0);
    assert_eq!(said, ["-:???", krabby.as_str()]);
    wrote(&scene(&["banner chapter Long, long ago"]), &[&["fadeout", "24"], &["fadein", "24"]]);
    wrote(&scene(&["memory", "end"]), &[&["grey", "on"], &["grey", "off"]]);
    wrote(&scene(&["krabby sleep", "krabby sleep off"]), &[&["pose", "krabby", "sleep"]]);
    wrote(&scene(&["weather snow", "daytime night stars"]), &[&["weather", "snow"], &["weather", "sparkles"]]);
}

#[test]
fn a_new_face_is_acted_first_and_what_is_said_is_acted() {
    let lines = clean(&scene(&["piplup (surprised): What?", "piplup (surprised): Again?"]));
    assert_eq!(lines.iter().filter(|fields| starts(fields, &["emote", "piplup"])).count(), 1);
    let lines = clean(&scene(&["krabby: No, never.", "krabby: What?!"]));
    assert_eq!(lines.iter().filter(|fields| starts(fields, &["turn", "krabby"])).count(), 3);
    // (A way by its name: a number is someone.)
    assert!(lines.contains(&line(&["turn", "krabby", "left"])));
    assert!(lines.contains(&line(&["jump", "krabby", "6", "6"])));
}

#[test]
fn how_a_film_opens_and_closes() {
    let (lines, _) = engine_lines("title: The film\nscene: town krabby\nkrabby: Yes.\ntransition: iris\nscene: beach krabby\n");
    for wanted in [&["typecaption", "150", "The film"][..], &["iris", "out", "30", "krabby"], &["iris", "in", "30"]] {
        assert!(lines.contains(&line(wanted)), "{wanted:?}");
    }
}

#[test]
fn no_one_comes_out_of_nowhere() {
    let arrives = |lines: Vec<Vec<String>>| lines.iter().any(|fields| starts(fields, &["arrives", "pidgey"]));
    assert!(arrives(clean(&scene(&["krabby: Yes.", "pidgey put left"]))));
    assert!(!arrives(clean(&scene(&["pidgey put left", "krabby: Yes."]))));
    assert!(!arrives(clean(&scene(&["krabby: Yes.", "memory", "pidgey put left", "end"]))));
    // (In the game's puff of smoke, its effect 99.)
    assert!(
        clean(&scene(&["krabby: Yes.", "pidgey appear left"]))
            .iter()
            .any(|fields| fields.contains(&"pidgey".to_string()) && fields.contains(&"99".to_string()))
    );
    clean(&scene(&["krabby: Yes.", "pidgey enter from left"]));
}

#[test]
fn a_crowd_keeps_clear_and_walks_in() {
    // (With collisions: a film has none unless it says.)
    let film = played(&format!(
        "collisions on
{}",
        scene(&["crowd pidgey 4 left", "krabby: Yes."])
    ));
    let krabby = point_of(&film, "krabby");
    for name in ["pidgey", "pidgey2", "pidgey3", "pidgey4"] {
        let at = point_of(&film, name);
        assert!((at.x - krabby.x).abs() > 20.0 || (at.y - krabby.y).abs() > 12.0, "{name} {at}");
    }
    let lines = clean(&scene(&["krabby: Yes.", "crowd pidgey 4 left"]));
    assert!(lines.iter().any(|fields| starts(fields, &["crowd", "walk"])));
}

#[test]
fn the_ground() {
    let lac = &tables().grounds[&36];
    assert!(!lac.rows()[29][37], "the lake's sand");
    assert!(lac.rows()[60][10], "the rocks");
    let film = played(&format!(
        "collisions on
{}",
        scene(&["pidgey put 10 10"])
    ));
    assert!(notes(&film).iter().any(|note| note.contains(": put")), "{:?}", notes(&film));
    // (With none, a film's way: just there, and nothing said.)
    let film = played(&scene(&["pidgey put 10 10"]));
    assert_eq!(point_of(&film, "pidgey"), Vec2::new(10.0, 10.0));
    assert!(!notes(&film).iter().any(|note| note.contains(": put")), "{:?}", notes(&film));
    let film = played(&scene(&["krabby wander 400"]));
    assert!(point_of(&film, "krabby").distance(Vec2::new(350.0, 212.0)) < 400.0);
}

#[test]
fn the_games_pictures() {
    let lines = clean("scene: sunset\nnarrator: The evening.\n");
    let level = tables().pictures["sunset"].level.to_string();
    assert!(lines.iter().find(|fields| fields[0] == "scene").expect("a scene").contains(&level));
    assert_eq!(engine_lines("scene: logos\n").1.len(), 1);
    let (_, stage) = played("scene: eclipse_3\n");
    assert_eq!(stage.view.camera, Some(engine::Cam::at(Vec2::new(780.0, 120.0))));
}

#[test]
fn the_games_rituals() {
    let lines = clean(&format!("uses guild\n{}", scene(&["banner dungeon Beach Cave", "dinner krabby piplup", "morning_call krabby piplup"])));
    assert!(lines.contains(&line(&["shout", "krabby", "-", "WAKE UP!!!"])));
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn a_meal_at_the_table() {
    let film = played("scene: mess_hall krabby piplup\ndinner krabby piplup\n");
    let seats = &tables().usual_spots["210"];
    for name in ["krabby", "piplup"] {
        let at = point_of(&film, name);
        assert!(seats.iter().any(|(x, y)| (*x as f32 - at.x).abs() < 2.0 && (*y as f32 - at.y).abs() < 2.0), "{name} {at}");
    }
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn every_pokemon_of_the_game_is_there() {
    assert!(tables().with_sprites.len() > 400);
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn the_game_s_generations() {
    let sizes: Vec<(usize, usize)> = tables().generations.iter().map(|(generation, names)| (*generation, names.len())).collect();
    // (The game has four generations; Pokémon of the later ones, added by hand, make more.)
    assert_eq!(&sizes[..3], [(1, 151), (2, 100), (3, 135)]);
    assert!(sizes[3].1 >= 100, "{sizes:?}");
}

#[test]
fn questions_gestures_and_ways() {
    wrote(&scene(&["choice", "option Let's go", "option We stay", "end"]), &[&["ask", "Let's go|We stay"], &["answer", "0"], &["answer", "1"], &["endask"]]);
    wrote(&scene(&["krabby bubble Hi you"]), &[&["bubble", "krabby", "-", "Hi you"]]);
    has(&scene(&["krabby think: Hi"]), "say");
    has(&scene(&["krabby (joyous): Hi!"]), "say");
    told(&scene(&["krabby (furious): Hi!"]), "unknown face");
    wrote(&scene(&["krabby jump"]), &[&["jump", "krabby", "16", "16"]]);
    // (The game's Joyous balloon, effect 90.)
    wrote(&scene(&["krabby emote joy"]), &[&["emote", "krabby", "joy", "60"]]);
    told(&scene(&["krabby emote furious"]), "unknown emote");
    wrote(&scene(&["krabby hug piplup"]), &[&["chase", "krabby", "piplup"]]);
    told(&scene(&["weather tornado"]), "tornado");
    told(&scene(&["> quake 3 40"]), "no longer passes");
}

#[test]
fn moving() {
    assert_eq!(point_of(&played(&scene(&["krabby put 300 212"])), "krabby"), Vec2::new(300.0, 212.0));
    has(&scene(&["krabby walk to right"]), "goes");
    wrote(&scene(&["krabby walk to piplup"]), &[&["approach", "krabby", "piplup", "40"]]);
    wrote(&scene(&["krabby run to piplup"]), &[&["chase", "krabby", "piplup"]]);
    wrote(&scene(&["krabby exit via right"]), &[&["gone", "krabby"]]);
    has("scene: town\nkrabby enter to right\n", "arrives");
    told(&scene(&["krabby walk to nowhere-at-all"]), "nowhere-at-all");
    told(&scene(&["stranger walk to right"]), "unknown Pokémon “stranger”");
    assert!(!clean(&scene(&["krabby walk to right &", "wait 1s"])).contains(&line(&["waitfor", "krabby"])));
    wrote(&scene(&["krabby walk to right &", "wait all"]), &[&["waitall"]]);
    let film = played(&scene(&["krabby: Yes.", "krabby exit via right"]));
    assert_eq!(film.1.bodies_here().len(), 1, "gone once out of sight");
}

#[test]
fn a_plan_is_set_before_it_shows() {
    let lines = clean("scene: town\nkrabby put 300 212\nzoom 0.5 1\nkrabby: Hi!\n");
    assert!(at_of(&lines, &["zoom"]) < at_of(&lines, &["fadein"]));
    assert!(at_of(&lines, &["light"]) < at_of(&lines, &["zoom"]));
}

#[test]
#[ignore = "needs the extracted data/ (run with --ignored)"]
fn what_stands_in_a_place_is_put_there() {
    let (lines, _) = engine_lines("scene: guild_entrance\nkrabby: Hi!\n");
    assert!(lines.contains(&line(&["put", "16576", "336", "180", "down"])));
}

/// The spots a film says are found for whom the words name: someone by the
/// code the film gives, a place as its words say it.
#[test]
fn the_spots_said_name_who_they_are_found_from() {
    use engine::{Spot, Who};
    use film_core::spots::{Goal, goal_of};
    let mut film = crate::tests::film();
    film.give_name("krabby".to_string(), "m98".to_string());
    film.give_name("apple".to_string(), "0".to_string());
    let krabby = Who::Monster(98);
    assert_eq!(film.spot_of("krabby"), Some(Spot::Of(krabby)));
    assert_eq!(film.spot_beside("krabby"), Some(Spot::Beside(krabby)));
    assert_eq!(film.spot_by("krabby", 12.0, -8.0), Some(Spot::By { of: krabby, dx: 12.0, dy: -8.0 }));
    let mut goal = |said: &str| match goal_of(&mut film, &said.split(' ').map(str::to_string).collect::<Vec<_>>(), 1) {
        Goal::Spot(spot) => spot,
        _ => panic!("{said}"),
    };
    assert_eq!(goal("12 8"), Spot::Written(glam::Vec2::new(12.0, 8.0).into()));
    assert_eq!(goal("between krabby apple"), Spot::Between(krabby, Who::Character(0)));
    assert_eq!(goal("behind krabby"), Spot::Before { of: krabby, back: true });
    assert_eq!(goal("3 tiles from krabby"), Spot::Tiles { of: krabby, count: 3.0 });
}

/// Who is there from the start is named at the head of the scene; the others
/// come by a line that says so. A sound with a Pokémon's name is no one.
#[test]
fn the_head_of_a_scene_names_who_is_there() {
    told("scene: town\nkrabby: Oh!\n", "krabby is there from the start");
    assert!(engine_lines("scene: town krabby\nkrabby: Oh!\n").1.is_empty());
    assert!(engine_lines("scene: town\nkrabby enter via left to center\nkrabby: Oh!\n").1.is_empty());
    assert!(engine_lines("scene: town\nto face face krabby piplup center\nkrabby: Oh!\n").1.is_empty());
    let lines = clean("scene: town krabby\nsound click\nkrabby think: Will Piplup come?\n");
    let named =
        |who: &str| lines.iter().any(|fields| !matches!(fields[0].as_str(), "sound" | "do" | "think" | "body") && fields.iter().any(|field| field == who));
    assert!(!named("click") && !named("piplup"), "{lines:?}");
}

/// One language in a film: an engine's command by its own name, among the
/// language's blocks.
#[test]
fn the_engine_s_commands_by_their_own_names() {
    let lines = clean(&scene(&["tempo 2", "if x = 1", "krabby jump", "else", "piplup jump", "end"]));
    assert!(lines.contains(&line(&["tempo", "2"])) && lines.contains(&line(&["else"])));
}

/// A sound named like a Pokémon (`sound click`, Click) is no one: no body, no
/// one placed, alone or played every so often.
#[test]
fn a_sound_named_like_a_pokemon_is_no_one() {
    // (Click would be a Pokémon of a later generation, added by hand: without it, nothing to try.)
    let Some((number, _)) = crate::tests::tables().monsters.get("click").cloned() else {
        return;
    };
    let clic = format!("m{number}");
    for film in ["scene: beach pikachu\nsound click\n", "scene: beach pikachu\nevery 2s : sound click\n"] {
        let written = crate::tests::compile(film);
        assert!(!written.lines.iter().any(|(_, fields)| fields.contains(&clic)), "{film}");
    }
}

#[test]
fn a_crowd_in_its_form_from_a_side_and_going_together() {
    // (Where said, as tight as said, in the shape said.)
    let lines = clean(&scene(&["crowd pidgey 6 300 200 shape flock tight radius 30"]));
    let crowd = lines.iter().find(|fields| starts(fields, &["crowd", "-"])).expect("crowd");
    for field in ["w:300:200", "shape:flock", "radius:30", "tight:1"] {
        assert!(crowd.contains(&field.to_string()), "{field} {crowd:?}");
    }
    // (From a side said: they walk in, before the scene shows or not.)
    let lines = clean(&scene(&["crowd pidgey 4 center shape heap from east fast"]));
    let crowd = lines.iter().find(|fields| starts(fields, &["crowd", "walk"])).expect("crowd walks in");
    assert!(crowd.contains(&"from:right".to_string()) && crowd.contains(&"run:1".to_string()), "{crowd:?}");
    told(&scene(&["crowd pidgey 4 from far"]), "from where");
    // (All of a crowd goes, keeping its form.)
    has(&scene(&["crowd pidgey 4 left shape flock", "krabby: Oh!", "pidgeys run to 300 200"]), "flock");
    let film = played(&format!("collisions off\n{}", scene(&["crowd pidgey 5 200 200 shape flock", "krabby: Oh!", "pidgeys walk to 300 220"])));
    let points: Vec<Vec2> = ["pidgey", "pidgey2", "pidgey3", "pidgey4", "pidgey5"].iter().map(|name| point_of(&film, name)).collect();
    let middle = points.iter().copied().sum::<Vec2>() / points.len() as f32;
    assert!(middle.distance(Vec2::new(300.0, 220.0)) < 12.0, "{middle}");
    assert!(points.iter().all(|at| at.distance(middle) < 60.0), "{points:?}");
}

#[test]
fn a_flock_from_the_east_comes_from_the_east() {
    let film = played(&format!("collisions off\n{}", scene(&["krabby: Oh!", "crowd pidgey 6 300 200 shape flock from east", "wait pidgey6"])));
    for name in ["pidgey", "pidgey3", "pidgey6"] {
        let at = point_of(&film, name);
        assert!(at.distance(Vec2::new(300.0, 200.0)) < 60.0, "{name} {at}");
    }
}

#[test]
fn a_place_after_the_weather_still_places() {
    // (The head of a scene, whatever its order: placed, not walking in.)
    let lines = clean(&scene(&["weather rain", "music worry", "krabby look piplup", "krabby put 200 200", "krabby: Oh!"]));
    assert!(!lines.iter().any(|fields| starts(fields, &["arrives"])), "{:?}", lines.iter().filter(|fields| starts(fields, &["arrives"])).collect::<Vec<_>>());
    // (After a line of the scene itself, one walks in.)
    let lines = clean(&scene(&["krabby: Oh!", "piplup put 200 200"]));
    assert!(lines.iter().any(|fields| starts(fields, &["arrives"])));
}

#[test]
fn one_who_faints_stays_lying() {
    let lines = clean(&scene(&["krabby faint", "piplup: Krabby?"]));
    let fainted = at_of(&lines, &["anim", "krabby", "faint"]);
    // (Nothing stands it up again before someone moves it.)
    assert!(!lines[fainted + 1..].iter().any(|fields| starts(fields, &["pose", "krabby"]) || starts(fields, &["anim", "krabby"])));
}

#[test]
fn the_engines_short_forms_are_written_out_by_the_film() {
    wrote(&scene(&["wait krabby piplup"]), &[&["waitfor", "krabby"], &["waitfor", "piplup"]]);
    wrote(&scene(&["krabby cheer piplup"]), &[&["after", "0~20", "emote", "krabby", "joy", "40"], &["after", "0~20", "jump", "piplup", "8", "12"]]);
    wrote(&scene(&["crowd krabby 3 center", "each krabbys jump $ 8 12"]), &[&["jump", "krabby2", "8", "12"]]);
}
