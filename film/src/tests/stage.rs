//! Who goes where, on a real stage.

use super::*;

#[test]
fn a_late_place_keeps_who_is_there() {
    let written = played("scene: town pikachu\npikachu: Hi!\nwait 1s\npikachu put left\n");
    let lines = clean("scene: town pikachu\npikachu: Hi!\nwait 1s\npikachu put left\n");
    assert!(lines.iter().any(|fields| fields[0] == "spread" && fields.contains(&"pikachu".to_string())));
    assert!(lines.iter().any(|fields| starts(fields, &["arrives", "pikachu"]) && fields[fields.len() - 1] == "place"));
    assert!((point_of(&written, "pikachu").x - (350.0 - 64.0)).abs() < 40.0);
}

#[test]
fn a_first_place_is_who_arrives() {
    let lines = clean("scene: town krabby\nkrabby: Hi!\npiplup put left\n");
    assert!(at_of(&lines, &["say"]) < at_of(&lines, &["arrives", "piplup"]));
    assert!(!lines.iter().any(|fields| fields[0] == "spread" && fields.contains(&"piplup".to_string())));
}

/// Out by the place's ways, in file, and gone; in from off the place.
#[test]
fn ways_out_and_in_are_real_routes() {
    let market = "scene: test_market pikachu piplup eevee\n";
    let lines = clean(&format!("{market}pikachu exit via left &\npiplup exit via left\neevee exit via east\nmarill enter via south to center\n"));
    assert!(lines.contains(&line(&["leaves", "pikachu", "left"])) && lines.contains(&line(&["leaves", "eevee", "east"])));
    // (`&`: the film does not wait for who goes out.)
    let pikachu = at_of(&lines, &["leaves", "pikachu"]);
    assert!(!starts(&lines[pikachu + 1], &["waitfor", "pikachu"]), "{:?}", lines[pikachu + 1]);
    assert!(lines.iter().any(|fields| starts(fields, &["arrives", "marill"]) && fields.last().is_some_and(|last| last == "via:south")));
    assert!(lines.iter().any(|fields| starts(fields, &["entrances", "west"])));
    told(&format!("{market}pikachu exit via north\n"), "unknown entrance “north”");
    // (`offleft`: out, not against a wall.)
    has(&format!("{market}pikachu walk to offleft\n"), "leaves");
    // (A way in is a spot of the scene.)
    has(&format!("{market}pikachu put south\n"), "put");
    let played = played(&format!("{market}pikachu exit via left &\npiplup exit via left\neevee exit via east\nmarill enter via south to center\nwait 1s\n"));
    let codes: Vec<&String> = played.0.names.iter().filter(|(_, name)| ["krabby", "piplup"].contains(&name.as_str())).map(|(code, _)| code).collect();
    for code in codes {
        assert!(!played.1.cast.contains_key(&Who::parse(code).expect("someone")), "{code} is gone out");
    }
    let wrong = notes(&played).into_iter().filter(|note| note.contains("blocked") || note.contains("wall")).collect::<Vec<_>>();
    assert!(wrong.is_empty(), "{wrong:?}");
    let marill = point_of(&played, "marill");
    assert!(marill.distance(Vec2::new(288.0, 240.0)) < 40.0, "{marill}");
}

/// What the last films wanted short: several out at once, a row at the start.
#[test]
fn short_ways_for_what_films_often_say() {
    let market = "scene: test_market pikachu piplup eevee\n";
    let lines = clean(&format!("{market}exit pikachu piplup via left\n"));
    let (first, second) = (at_of(&lines, &["leaves", "pikachu", "left"]), at_of(&lines, &["leaves", "piplup", "left"]));
    assert!(first < second && second < at_of(&lines, &["waitfor", "pikachu"]), "together, then waited for");
    let lines = clean(&format!("{market}form row pikachu piplup eevee 240 270\npikachu: Yes.\n"));
    assert_eq!(
        lines
            .iter()
            .filter(|fields| starts(fields, &["put"]) && fields.get(1).is_some_and(|who| ["pikachu", "piplup", "eevee"].contains(&who.as_str())))
            .count(),
        3
    );
}

#[test]
fn a_word_not_known_says_the_nearest() {
    let film = scene(&["wallk krabby to right"]);
    told(&film, "unknown word “wallk”");
    told(&film, "walk");
    told(&film, "line 2");
}

#[test]
fn who_is_not_found_says_the_nearest() {
    told(&scene(&["pikachou walk to right"]), "pikachu");
    told("scene: town pikachou\n", "unknown Pokémon “pikachou”");
    told("pinchy = krabbi\nscene: town\n", "krabby");
    told(&scene(&["krabby attack Thunderbol at piplup"]), "thunderbolt");
    told("scene: beaach\n", "beach");
}

#[test]
fn a_tile_of_the_games_grid() {
    let played = played("scene: town\nkrabby put tile 42 26\n");
    assert_eq!(point_of(&played, "krabby"), Vec2::new(340.0, 212.0));
    // (Pikachu walks: krabby and piplup, of the water by nature, would be
    // put in it, where no one walks.)
    let taken = self::played("collisions on\nscene: town pikachu\npikachu put tile 0 0\n");
    assert!(notes(&taken).iter().any(|note| note.contains("pikachu cannot be on tile 0 0")), "{:?}", notes(&taken));
}

#[test]
fn between_two_and_so_many_tiles_from_someone() {
    let film = played("scene: town pikachu\nkrabby put 300 212\npiplup put 380 212\npikachu: Hi!\npikachu walk between krabby piplup\n");
    assert_eq!(point_of(&film, "pikachu"), Vec2::new(340.0, 212.0));
    let film = played("scene: town\nkrabby put 300 212\npiplup put 400 212\npiplup: There!\npiplup walk 5 tiles from krabby\n");
    assert_eq!(point_of(&film, "piplup"), Vec2::new(340.0, 212.0));
}

#[test]
fn a_big_one_is_given_room() {
    let far = |who: &str| {
        let film = played(&format!("scene: town\n{who} put 340 212\nkrabby put right_of {who}\n"));
        (point_of(&film, "krabby").x - 340.0).abs()
    };
    assert!(far("groudon") > far("pikachu"));
}

#[test]
fn nowhere_near_is_told() {
    told(&scene(&["krabby put front_of nobody"]), "“nobody” is not on the stage");
    told(&scene(&["krabby put seven tiles from piplup"]), "which spot?");
    told(&scene(&["krabby put throne"]), "which spot?");
}

#[test]
fn a_spot_of_the_film() {
    assert_eq!(point_of(&played("scene: town\nspot throne 340 212\nkrabby put throne\n"), "krabby"), Vec2::new(340.0, 212.0));
    assert_eq!(point_of(&played("spot throne tile 42 26\nscene: town\nkrabby put throne\n"), "krabby"), Vec2::new(340.0, 212.0));
    told(&scene(&["spot center 1 2"]), "already a spot");
    told("scene: town\nspot throne 340 212\nscene: beach\nkrabby put throne\n", "which spot?");
}

#[test]
fn those_who_appear_do_not_stack() {
    let film = played(&scene(&["pikachu appear right", "eevee appear right"]));
    assert_ne!(point_of(&film, "pikachu"), point_of(&film, "eevee"));
}

#[test]
fn formations() {
    let film = played("scene: town\nform row pikachu piplup eevee\neevee: Oh!\n");
    let (a, b, c) = (point_of(&film, "pikachu").x, point_of(&film, "piplup").x, point_of(&film, "eevee").x);
    assert!(a < b && b < c, "{a} {b} {c}");
    wrote("scene: town\nform row pikachu piplup eevee\n", &[&["turn", "eevee", "down"]]);
    let film = played("scene: town\nform column pikachu piplup left look right\n");
    assert_ne!(point_of(&film, "pikachu").y, point_of(&film, "piplup").y);
    wrote("scene: town\nform column pikachu piplup left look right\n", &[&["turn", "piplup", "right"]]);
    clean("scene: town\nform ring pikachu piplup eevee\n");
    // (Into the shape, walked or run: after the scene shows, or even at its head.)
    let lines = clean("scene: town pikachu piplup\npikachu: Come!\nform row pikachu piplup to left run look right\n");
    assert!(lines.iter().any(|fields| starts(fields, &["arrives", "piplup", "run"])), "{lines:?}");
    assert!(lines.contains(&line(&["turn", "piplup", "right"])), "{lines:?}");
    let lines = clean("scene: town\nform column pikachu piplup to 300 200 walk\n");
    assert!(lines.iter().any(|fields| starts(fields, &["arrives", "pikachu", "walk"])), "{lines:?}");
    wrote(&scene(&["krabby face piplup"]), &[&["face", "krabby", "piplup"]]);
}

#[test]
fn a_look() {
    wrote(&scene(&["krabby look right"]), &[&["turn", "krabby", "right"]]);
    wrote(&scene(&["krabby look piplup"]), &[&["face", "krabby", "piplup"]]);
}

#[test]
fn a_pace() {
    let lines = clean(&scene(&["krabby: Oh!", "krabby walk to right slowly"]));
    assert!(lines.iter().any(|fields| starts(fields, &["goes", "krabby", "walk", "x0.5"])));
    assert!(lines.contains(&line(&["speed", "krabby", "1.2"])));
    assert!(clean(&scene(&["krabby: Oh!", "krabby run to right fast"])).iter().any(|fields| starts(fields, &["goes", "krabby", "run", "x1.6"])));
    let lines = clean("scene: town\nkrabby put 300 212\nkrabby: Oh!\nkrabby walk to 360 212 in 2s\n");
    assert!(lines.iter().any(|fields| starts(fields, &["goes", "krabby", "walk", "s2"])));
    assert_eq!(clean("scene: town\nkrabby put 300 212\nkrabby: Oh!\nkrabby walk to 360 212 arrives 2s\n"), lines);
    let lines = clean("scene: town\nkrabby put 300 212\npiplup put 400 212\nkrabby: Oh!\nkrabby walk to piplup in 1s\n");
    assert!(!lines.iter().any(|fields| fields[0] == "approach"));
    assert!(lines.iter().any(|fields| fields[0] == "find" && fields[3].starts_with("p:")));
    // (A pace alone on its way is undone at the next, on the stage.)
    let film = played(&scene(&["krabby: Oh!", "krabby walk to right fast &", "wait 1s", "krabby walk to left"]));
    assert!(film.1.place.paced.is_empty());
}

#[test]
fn a_move_given_its_time_takes_it() {
    let film = "scene: town\nkrabby put 300 212\nkrabby: Oh!\n";
    let (written, stage) = played(&format!("{film}krabby walk to 360 212 in 2s &\nwait 1s\nlet there = 1\n"));
    let _ = written;
    assert_eq!(stage.place.paced.len(), 1);
}

fn passage(lines: &[Vec<String>], name: &str) -> Vec<Vec<String>> {
    let start = at_of(lines, &["label", name]);
    let end = (start..lines.len()).find(|at| lines[*at] == line(&["return"])).expect("its end");
    lines[start + 1..end].to_vec()
}

#[test]
fn a_line_said_with_and_is_said_beside_the_film() {
    // (Played beside it: a passage, waited for by `wait all` and at the
    // scene's end.)
    let lines = clean(&scene(&["krabby: I talk for a long time. &", "piplup jump", "wait all", "piplup: Ah."]));
    assert_eq!(passage(&lines, "thread1").last(), Some(&line(&["set", "thread1_done", "1"])));
    assert!(at_of(&lines, &["parallel", "thread1"]) < at_of(&lines, &["jump", "piplup", "16", "16"]));
    assert!(at_of(&lines, &["waituntil", "thread1_done", "==", "1"]) < at_of(&lines, &["say", "piplup"]));
}

#[test]
fn a_line_is_anchored_to_an_arrival_or_a_mark() {
    // (`when a arrives : …`: beside the film, once a has got there.)
    let lines = clean(&scene(&["krabby walk to right &", "when krabby arrives : piplup jump", "piplup: Here it goes."]));
    let thread = passage(&lines, "thread1");
    assert_eq!(thread.first(), Some(&line(&["waitfor", "krabby"])));
    assert!(thread.iter().any(|fields| starts(fields, &["jump", "piplup"])));
    assert!(at_of(&lines, &["parallel", "thread1"]) < at_of(&lines, &["say", "piplup"]), "the film does not wait");
    // (As a block too.)
    let lines = clean(&scene(&["krabby walk to right &", "when krabby arrives", "piplup jump", "krabby emote joy", "end"]));
    assert!(passage(&lines, "thread1").iter().any(|fields| starts(fields, &["emote", "krabby"])));
    // (`mark name`, then `at name + 2s : …`: so long after the mark.)
    let lines = clean(&scene(&["mark quarrel", "piplup: No!", "at quarrel + 2s : krabby jump", "at quarrel", "krabby emote joy", "end"]));
    assert!(lines.contains(&line(&["set", "_mark_quarrel", "clock"])));
    let thread = passage(&lines, "thread1");
    assert!(starts(&thread[0], &["calc"]) && thread[0][2..] == line(&["_mark_quarrel", "+", "120"]), "{thread:?}");
    assert!(starts(&thread[1], &["waituntil", "clock", ">="]));
    assert!(passage(&lines, "thread2").iter().any(|fields| starts(fields, &["emote", "krabby"])));
    told(&scene(&["at quarrel + 1s : krabby jump"]), "is not marked");
    // (`at 3s : …`: so long after the scene's start; `at music 3s : …`, by
    // the music's clock.)
    let lines = clean(&scene(&["at 3s: jump krabby", "piplup: No!"]));
    assert_eq!(passage(&lines, "thread1").first(), Some(&line(&["waituntil", "clock", ">=", "180"])));
    let lines = clean(&scene(&["music peace", "at music 2.5s", "krabby jump", "end", "wait music 4s"]));
    assert_eq!(passage(&lines, "thread1").first(), Some(&line(&["waituntil", "music_clock", ">=", "150"])));
    assert!(lines.contains(&line(&["waituntil", "music_clock", ">=", "240"])));
    told(&scene(&["at 3 : krabby jump"]), "a duration with its unit");
    told(&scene(&["krabby walk to right &", "when krabby arrives", "end"]), "nothing to play");
}

#[test]
fn a_film_uses_the_recipes_of_a_file_or_takes_its_lines() {
    // (`uses welcome`: its recipes, in library/films/recipes.)
    let lines = clean(
        "uses welcome
scene: town krabby piplup
krabby reunion piplup
krabby salute 2
",
    );
    assert!(lines.iter().any(|fields| starts(fields, &["emote", "piplup"])));
    // (Its two greetings, written by its code; the meeting's hug hops with the sprite's Hop.)
    assert_eq!(lines.iter().filter(|fields| starts(fields, &["jump", "krabby"])).count(), 2);
    // (`include`: its lines, as if written there.)
    let lines = clean(
        "include welcome
scene: town krabby piplup
piplup salute
",
    );
    assert!(lines.iter().any(|fields| starts(fields, &["jump", "piplup"])));
    told(
        "uses welcom
scene: town krabby
",
        "welcome",
    );
    // (A file taken in: its lines told as the line that takes it in, what is
    // wrong in them said of the file and its line.)
    let written = compile("include welcome\nscene: town krabby piplup\npiplup salute\n");
    assert!(written.lines.iter().all(|(number, _)| *number < 100), "no line number of an included file is left");
    let included = vec![(1, "welcome".to_string())];
    assert_eq!(crate::text::recipes::told_line(&included, 1_000_004), (1, Some(("welcome".to_string(), 4))));
    assert_eq!(crate::text::recipes::told_line(&included, 7), (7, None));
}

#[test]
fn an_anchored_line_plays_when_it_is_due() {
    // (Played: krabby jumps once there, piplup two seconds after the mark.)
    let film = scene(&["krabby walk to right &", "when krabby arrives : krabby emote joy", "mark there", "at there + 1s : piplup emote anger", "wait 3s"]);
    let (written, _) = played(&film);
    assert!(written.problems.iter().all(|problem| problem.is_remark()), "{:?}", written.problems);
}

#[test]
fn a_line_with_and_goes_on_attends_waits() {
    // (`&`: the line starts and the film goes on; `wait`: so long, for
    // someone, for all.)
    let lines = clean(&scene(&["krabby walk to right &", "piplup jump &", "wait 1s", "wait krabby", "wait all", "krabby: Done!"]));
    let at = |fields: &[&str]| at_of(&lines, fields);
    assert!(at(&["t", "60"]) < at(&["waitfor", "krabby"]) && at(&["waitfor", "krabby"]) < at(&["waitall"]) && at(&["waitall"]) < at(&["say"]));
    told(&scene(&["wait"]), "wait all");
    // (Within a sequence too.)
    let lines = clean("define wriggle a:pokemon\na jump &\nwait 1s\nend\nscene: town krabby\nkrabby wriggle\n");
    assert!(lines.contains(&line(&["t", "60"])));
}

#[test]
fn the_view_is_told_of_bodies() {
    let lines = clean("scene: town groudon krabby\ngroudon: Oh!\n");
    let body = lines.iter().position(|fields| fields[0] == "body" && fields[1] == "groudon").expect("its body");
    assert!(body < at_of(&lines, &["spread"]));
}

#[test]
fn framing_and_the_view() {
    // (At the head of the scene a cut: it opens on its first framing; later a glide.)
    wrote(&scene(&["frame krabby piplup"]), &[&["keepframe", "off"], &["frame", "0", "krabby", "piplup"], &["fadein", "30"]]);
    wrote(&scene(&["krabby: Oh!", "frame krabby piplup"]), &[&["keepframe", "off"], &["frame", "40", "krabby", "piplup"], &["waitcam"]]);
    has(&scene(&["frame 30 krabby piplup"]), "frame");
    wrote(&scene(&["krabby: Oh!", "glide to piplup in 3s"]), &[&["pan", "piplup", "180"], &["waitcam"]]);
    told(&scene(&["frame nowhere"]), "nowhere");
    wrote(&scene(&["frame krabby follows"]), &[&["keepframe", "krabby"]]);
    wrote(&scene(&["frame krabby piplup follows"]), &[&["keepframe", "krabby", "piplup"]]);
    // (People and a place together: the view where they all show.)
    let lines = clean(&scene(&["frame krabby 300 150"]));
    assert!(lines.iter().any(|fields| fields[0] == "frame" && fields.contains(&"@300,150".to_string())), "{lines:?}");
    has(&scene(&["glide right"]), "pan");
}

/// `pace slow`: the waits said after it longer, a word's own time its own;
/// `pace lines 2s`: each line said stays so long, then goes on.
#[test]
fn the_pace_of_what_follows() {
    wrote(&scene(&["pace slow", "wait 1s", "pace 0.5", "wait 0.5s", "pace normal", "wait 1s"]), &[&["t", "84"], &["t", "15"], &["t", "60"]]);
    told(&scene(&["pace 9"]), "pace: a number");
    let said = said_lines(&clean(&scene(&["pace lines 2s", "krabby: A long line that lasts.", "pace lines off", "piplup: No."])));
    assert_eq!(said[0][3], "120");
    assert_ne!(said[1][3], "120");
}
