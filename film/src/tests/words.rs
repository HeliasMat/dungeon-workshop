//! Words of precision: sounds, effects, the hour, a way, the view, lines said.

use super::*;

#[test]
fn sounds_and_music_in_step() {
    let lines = clean(&scene(&["sound Bite at 12", "sound shock at 30", "sound Bite on krabby", "sound Bite"]));
    let chosen = lines.iter().find(|fields| fields[0] == "movesound").expect("a sound").last().expect("a move").clone();
    assert!(lines.contains(&line(&["after", "12", "movesound", &chosen])));
    assert!(lines.contains(&line(&["after", "30", "sound", "shock"])));
    assert!(lines.contains(&line(&["onhit", "krabby", "movesound", &chosen])));
    assert!(clean(&scene(&["sound shock"])).contains(&line(&["sound", "shock"])));
    assert!(clean(&scene(&["sound shock krabby"])).contains(&line(&["sound", "shock", "krabby"])));
    let lines = clean(&scene(&["music peace from 12s", "music off fade 3s"]));
    assert!(lines.contains(&line(&["musicfrom", "peace", "12s"])));
    assert!(lines.contains(&line(&["fademusic", "0", "180"])));
}

#[test]
fn effects_on_a_body_a_point_or_between_two() {
    let lines = clean(&scene(&[
        "effect bolt on krabby",
        "effect Hydro Pump on krabby",
        "effect Hydro Pump from krabby to piplup in 40",
        "effect Iron Tail on 300 200",
    ]));
    assert!(lines.contains(&line(&["lightning", "krabby"])));
    let chosen = lines.iter().find(|fields| starts(fields, &["moveeffect", "krabby"])).expect("an effect")[2].clone();
    assert!(lines.contains(&line(&["moveshot", "krabby", "piplup", &chosen, "40"])));
    assert!(lines.contains(&line(&["put", "p900", "300", "200", "down"])));
    assert!(lines.iter().any(|fields| starts(fields, &["moveeffect", "p900"])));
    assert!(clean(&scene(&["effect krabby 12"])).contains(&line(&["effect", "krabby", "12"])));
    told(&scene(&["effect Whatsit on krabby"]), "unknown effect");
}

#[test]
fn an_hour_exactly_and_no_veil_unless_written() {
    // (18h is the light of the evening, `evening`: its beat is written by that name.)
    assert!(clean(&scene(&["daytime 18h"])).contains(&line(&["light", "evening", "120"])));
    assert_eq!(clean(&scene(&["daytime 12h30 1"])).iter().find(|fields| fields[0] == "tint").expect("a tint")[4], "0");
    // (The last tint: the veil of the hour, after the one put off at the scene's start.)
    assert_eq!(clean(&scene(&["daytime 21h 1s"])).iter().rev().find(|fields| fields[0] == "tint").expect("a tint")[4], "100");
    // (None but the one put off, all its colour nothing.)
    assert!(!clean(&scene(&["krabby: Oh!"])).iter().any(|fields| fields[0] == "tint" && fields[4] != "0"));
    assert!(clean(&scene(&["daytime evening"])).contains(&line(&["light", "evening", "120"])));
}

#[test]
fn a_weather_gone_and_a_memory_ended_keep_the_hour() {
    let last_light = |lines: Vec<Vec<String>>| lines.into_iter().rev().find(|fields| fields[0] == "light" || fields[0] == "tint").expect("a light");
    let evening = last_light(clean(&scene(&["daytime evening"])))[1].clone();
    assert_eq!(last_light(clean(&scene(&["daytime evening", "weather rain heavy", "weather off"])))[1], evening);
    assert_eq!(last_light(clean(&scene(&["daytime evening", "memory", "end"])))[1], evening);
}

#[test]
fn a_way_by_places() {
    let lines = clean(&scene(&["spot throne 330 300", "piplup walk via 300 330, throne, to krabby slowly"]));
    let way = lines.iter().find(|fields| fields[0] == "goes").expect("a way");
    assert_eq!(way[1..4], line(&["piplup", "walk", "x0.5"]));
    assert!(way.len() >= 10, "{way:?}");
    let lines = clean(&scene(&["piplup run via 300 330, tile 40 40 &"]));
    assert!(lines.iter().any(|fields| starts(fields, &["goes", "piplup", "run"])));
    assert!(!lines.contains(&line(&["waitfor", "piplup"])));
}

#[test]
fn someone_stopped_and_going_on() {
    let lines = clean(&scene(&["piplup walk to right &", "wait 1s", "piplup stop", "wait 1s", "piplup resume"]));
    assert!(lines.contains(&line(&["stop", "piplup"])) && lines.contains(&line(&["resume", "piplup"])));
}

#[test]
fn before_someone_is_found_as_it_looks_when_played() {
    let film = "scene: town krabby\nkrabby put 200 212\npiplup put 340 212\npiplup look left\nwait 1s\n";
    let far = |said: &str| {
        let played = played(&format!("{film}pikachu put {said} piplup\nwait all\n"));
        point_of(&played, "pikachu").x - 340.0
    };
    assert!(far("front_of") < 0.0, "it looks left");
    assert!(far("behind") > 0.0);
    assert!(far("left_of") < 0.0);
    assert!(far("right_of") > 0.0);
    let lines = clean(&scene(&["krabby: Oh!", "piplup walk to front_of krabby", "glide to behind krabby"]));
    assert!(lines.iter().any(|fields| fields[0] == "find" && fields[2] == "piplup" && fields[3].starts_with("d:")));
    assert!(lines.iter().any(|fields| fields[0] == "find" && fields[3].starts_with("b:") && fields[4] == "exact"));
}

/// The view on a group, or on a whole crowd by its name; a glide, never a jump.
#[test]
fn the_view_keeps_a_group_or_a_crowd() {
    let crowd = "scene: town krabby piplup\ncrowd marill 3 right\n";
    wrote(&format!("{crowd}frame krabby piplup follows\n"), &[&["keepframe", "krabby", "piplup"]]);
    wrote(&format!("{crowd}frame marills follows\n"), &[&["keepframe", "marill", "marill2", "marill3"]]);
    wrote(&format!("{crowd}frame marills\n"), &[&["frame", "0", "marill", "marill2", "marill3"]]);
}

#[test]
fn a_line_said_fast_slowly_with_a_pause() {
    let said = |lines: &[Vec<String>]| said_lines(lines).into_iter().next().expect("a line");
    // (Pikachu: his portraits have every face; a face one lacks is another, see `face_had`.)
    let of = |manner: &str| clean(&format!("scene: town pikachu\npikachu ({manner}): No, I'm not coming!\n"));
    let (fast, slow, usual) = (of("shouting, fast"), of("slow"), of("normal"));
    assert!(fast.contains(&line(&["textspeed", "1.6"])));
    assert!(slow.contains(&line(&["textspeed", "0.6"])) && slow.contains(&line(&["textspeed", "normal"])));
    assert!(!usual.iter().any(|fields| fields[0] == "textspeed"));
    assert_eq!(said(&fast)[1], POKEMON.face("shouting").expect("a face").to_string());
    let frames = |lines: &[Vec<String>]| said(lines)[3].parse::<i32>().expect("frames");
    assert!(frames(&fast) < frames(&usual) && frames(&slow) > frames(&usual));
    let lines = clean(&scene(&["krabby: Wait...[1s] No.", "krabby: Wait... No."]));
    let both = said_lines(&lines);
    assert!(both[0][2].contains("[W:60]"));
    assert_eq!(both[0][3].parse::<i32>().expect("frames"), both[1][3].parse::<i32>().expect("frames") + 60);
    assert!(said(&clean(&scene(&["krabby: *Never* again."])))[2].contains("*Never*"));
}

/// A face between brackets in a line changes there (`[FACE:n]`, as the
/// game's text writes it), one the species has; the narrator's has none.
#[test]
fn a_face_changes_in_the_middle_of_a_line() {
    let said = |lines: &[Vec<String>]| said_lines(lines).into_iter().next().expect("a line");
    let line = said(&clean("scene: town pikachu\npikachu (happy): We won! [sad] But at what cost...\n"));
    assert!(line[2].contains("We won! [FACE:") && line[2].contains("]But at what cost"), "{line:?}");
    let told = said(&clean("scene: town pikachu\nnarrator: Night fell. [sad] Cold.\n"));
    assert!(told[2].contains("Night fell. Cold.") && !told[2].contains("FACE"), "{told:?}");
}

#[test]
fn the_box_on_top() {
    let film = format!("{}scene: beach\nnarrator: Down below.\n", scene(&["box top", "krabby: Oh!"]));
    let (lines, _) = engine_lines(&film);
    assert!(lines.contains(&line(&["boxside", "top"])) && lines.contains(&line(&["boxside", "auto"])));
}

#[test]
fn a_line_over_another() {
    // (A line said beside the film, another over it a second later.)
    let lines = clean(&scene(&["krabby: I talk for a long time. &", "wait 1s", "piplup (top): Hey!"]));
    assert!(lines.contains(&line(&["bubble", "piplup", "-", "Hey!"])));
}

#[test]
fn bodies() {
    let lines = clean(&scene(&[
        "piplup pose Attack image 3",
        "piplup anim Walk 2 times",
        "piplup still",
        "piplup unfreeze",
        "piplup height 12",
        "piplup tremble 2 30",
        "piplup infront krabby",
    ]));
    for wanted in [
        &["pose", "piplup", "Attack", "image", "3"][..],
        &["anim", "piplup", "Walk", "2", "times"],
        &["still", "piplup"],
        &["unfreeze", "piplup"],
        &["height", "piplup", "12"],
        &["tremble", "piplup", "2", "30"],
        &["infront", "piplup", "krabby"],
    ] {
        assert!(lines.contains(&line(wanted)), "{wanted:?}");
    }
}

// ---- One family of words for the view: old spellings stay.

/// Every gesture of the library (`film/recipes/gestures.film`), played on
/// someone, writes what the engine reads: none is left broken unseen.
#[test]
fn every_gesture_of_the_library_plays() {
    let stand_in = |name: &str| match name.trim_matches(['[', ']']).split(':').next().unwrap_or("") {
        "who" => "krabby",
        "other" => "piplup",
        "threat" => "kecleon",
        "way" => "left",
        "form" => "m133",
        // (A thing on the stage by its code, as `receive` hands it to `got_item`.)
        "item" => "o1",
        "text..." => "Psst, listen",
        "thing..." => "krabby",
        "where" => "300 200",
        // (A word written as it is, `cower who from other`, `react who joy`.)
        literal if !name.contains([':', '.', '[']) => Box::leak(literal.to_string().into_boxed_str()),
        other => panic!("a gesture takes “{other}”: give it an example here"),
    };
    let library = include_str!("../../recipes/gestures.film");
    let mut broken = Vec::new();
    for header in library.lines().filter_map(|line| line.strip_prefix("define ")) {
        let mut words = header.split_whitespace();
        let name = words.next().expect("a name");
        // (The library's own, `_evolution`: no film writes it.)
        if film_core::verbs::internal(name) {
            continue;
        }
        // (A word that may be left out, `back:number=12` or `[in`, is; who acts first.)
        let mut given: Vec<&str> =
            words.filter(|word| !word.contains('=') && !(word.starts_with('[') && !word.contains(':') && !word.contains("..."))).map(stand_in).collect();
        let who = (given.first() == Some(&"krabby")).then(|| given.remove(0));
        let call = who.into_iter().chain(std::iter::once(name)).chain(given).collect::<Vec<_>>().join(" ");
        let (_, problems) = engine_lines(&format!("scene: town krabby piplup kecleon\n{call}\n"));
        broken.extend(problems.into_iter().filter(|problem| !is_remark(problem)).map(|problem| format!("{call}: {problem}")));
    }
    assert!(broken.is_empty(), "{broken:#?}");
}

/// A line said is shown as the box will cut it, its marks out; another is
/// no line said.
#[test]
fn a_line_said_is_cut_as_the_box_shows_it() {
    let lines = film_core::screen::boxed("pikachu (joyous): Wait...[1s] Shall we *really* go to the end of the beach, the two of us, tonight?").expect("said");
    assert_eq!(lines, ["Wait... Shall we really go to the", "end of the beach, the two of us,", "tonight?"]);
    assert!(film_core::screen::boxed("pikachu walk to right").is_none());
}
