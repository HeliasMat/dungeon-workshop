//! Fights, by hand and stated in advance.

use super::*;

const FIGHT: &str = "fight directed krabby piplup against eevee pikachu level 40";

fn fight_film(lines: &[&str]) -> String {
    format!("scene: town krabby piplup eevee pikachu marill\n{FIGHT}\n{}\nend\n", lines.join("\n"))
}

fn directed(lines: &[&str]) -> (Vec<String>, Vec<Vec<String>>) {
    let written = clean(&fight_film(lines));
    (written.iter().find(|fields| starts(fields, &["fight", "directed"])).expect("a fight").clone(), written)
}

fn index_of(fields: &[String], word: &str) -> usize {
    fields.iter().position(|field| field == word).unwrap_or_else(|| panic!("{word}"))
}

#[test]
fn several_options_on_a_blow() {
    let (fight, _) = directed(&["krabby Bubble eevee crit -60 slow slowmo 30 intercepts pikachu"]);
    let at = index_of(&fight, "act");
    assert_eq!(fight[at + 3..at + 11], line(&["eevee", "critical", "-60", "intercepts", "pikachu", "slowmo", "30", "slow"]));
}

#[test]
fn a_blow_slowed_on_a_line_of_its_own() {
    let (fight, _) = directed(&["slowmo 1s", "krabby Bubble eevee"]);
    let at = index_of(&fight, "act");
    assert_eq!(fight[at + 4..at + 6], line(&["slowmo", "60"]));
}

#[test]
fn a_miss_is_no_more_than_that_and_who_intercepts_is_of_the_other_side() {
    told(&fight_film(&["krabby Bubble eevee misses crit"]), "misses");
    told(&fight_film(&["krabby Bubble eevee intercepts piplup"]), "intercept");
}

#[test]
fn the_great_bars_and_when() {
    let (fight, written) = directed(&["hud krabby eevee", "when health(eevee) <= 50 : eevee (angry): Enough!", "krabby Bubble eevee"]);
    let at = index_of(&fight, "great_bars");
    assert_eq!(fight[at + 1..at + 3], line(&["krabby", "eevee"]));
    assert!(fight.contains(&"when".to_string()));
    assert!(written.contains(&line(&["label", "fight1_0"])));
}

#[test]
fn what_comes_between_two_blows() {
    let (fight, written) = directed(&["krabby Bubble eevee", "eevee (angry): Ouch!", "wait 1s", "log The ground shakes", "piplup Tackle pikachu"]);
    let at = index_of(&fight, "scene");
    assert_eq!(fight[at..at + 6], line(&["scene", "fight1_said0", "pause", "60", "note", "The ground shakes"]));
    assert!(written.contains(&line(&["label", "fight1_said0"])));
}

#[test]
fn a_change_brings_in_one_who_waits() {
    let (fight, _) = directed(&["change krabby marill", "marill Bubble eevee"]);
    assert!(fight[index_of(&fight, "reserve")..].contains(&"marill".to_string()));
    let at = index_of(&fight, "change");
    assert_eq!(fight[at + 1..at + 3], line(&["krabby", "marill"]));
}

#[test]
fn a_fight_goes_on_the_films_own_course() {
    // (Nothing beside the film may fight: a fight is never a passage.)
    let written =
        clean("scene: town piplup krabby\npiplup put left\nkrabby put right\nwait 2s\nkrabby bubble Come on! &\nwait 3s\nfight piplup vs krabby duel\n");
    let at = at_of(&written, &["fight", "alone"]);
    assert_eq!(written[at - 1], line(&["t", "180"]));
    assert!(!written.iter().any(|fields| fields[0] == "parallel"));
    told("scene: town piplup krabby\npiplup put left\nkrabby put right\nat 3s: fight piplup vs krabby duel\n", "beside the film");
}

#[test]
fn a_fight_stated_in_advance() {
    let film = "scene: lava_field pikachu piplup groudon rayquaza\nfight directed pikachu piplup against groudon rayquaza level 60\nhp pikachu 320\nlevel piplup 40\nknows piplup ThunderShock Iron Defense\nnumber of turns 8\npikachu ThunderShock groudon\ngroudon Earthquake all misses\npiplup Hydro Pump groudon crit\npikachu Iron Tail rayquaza -30\npiplup Last Resort groudon ko\ngroudon Protect\nrandom until the end winner piplup\nend\n";
    let (lines, problems) = engine_lines(film);
    assert_eq!(problems, ["remark: line 12: groudon is already K.O.: this line will not be played"]);
    let fight = lines.iter().find(|fields| starts(fields, &["fight", "directed"])).expect("a fight");
    assert_eq!(fight[index_of(fight, "level") + 1], "60");
    assert_eq!(fight[index_of(fight, "winner") + 1], "piplup");
    assert_eq!(fight[index_of(fight, "hp")..index_of(fight, "hp") + 3], line(&["hp", "pikachu", "320"]));
    assert!(fight.contains(&"rest".to_string()));
    assert_eq!(fight[index_of(fight, "turns") + 1], "8");
    let said = fight.join(" ");
    for wanted in [
        "knows piplup 188 ",
        "act pikachu 188 groudon",
        "act groudon 118 all misses",
        "act piplup 240 groudon critical",
        "rayquaza -30",
        "act piplup 497 groudon ko",
    ] {
        assert!(said.contains(wanted), "{wanted}");
    }
    assert_eq!(lines[at_of(&lines, &["fight", "directed"]) + 1][..2], line(&["fight", "end"]));
}

#[test]
fn a_directed_fight_says_what_is_wrong() {
    let head = "scene: lava_field\nfight directed pikachu against groudon\n";
    for (lines, words) in [
        ("piplup ThunderShock groudon\nend\n", "“piplup” is not in this fight"),
        ("pikachu ThunderShok groudon\nend\n", "ThunderShock"),
        ("pikachu ThunderShock pikachu\nend\n", "not on the other side"),
        ("hp pikachu lots\nend\n", "“lots” is not a number"),
        ("hp mewtwo 3\nend\n", "“mewtwo” is not in this fight"),
        ("pikachu ThunderShock groudon\n", "is not closed by"),
    ] {
        told(&format!("{head}{lines}"), words);
    }
}

#[test]
fn an_evolution() {
    let lines = clean(&scene(&["krabby evolve trapinch"]));
    assert!(lines.iter().any(|fields| starts(fields, &["disguise", "krabby"])));
    let lines = clean(&scene(&["krabby evolve trapinch", "trapinch (joyous): Yes."]));
    assert!(!lines.iter().any(|fields| fields[0] == "spread" && fields.contains(&"trapinch".to_string())));
    assert!(lines.iter().any(|fields| starts(fields, &["say", "krabby"])));
}

fn fight_of(film: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let lines = clean(film);
    let fights: Vec<&Vec<String>> = lines.iter().filter(|fields| fields[0] == "fight" && !matches!(fields[1].as_str(), "end" | "open" | "rage")).collect();
    assert_eq!(fights.len(), 1);
    assert!(!lines.iter().any(|fields| matches!(fields[0].as_str(), "ask" | "while")));
    (fights[0].clone(), lines.clone())
}

#[test]
fn every_fight_is_a_line() {
    for (said, sort) in [
        ("fight pikachu vs krabby duel level 40 winner krabby", "alone"),
        ("fight pikachu eevee vs krabby marill double level 25", "double"),
        ("fight pikachu eevee vs krabby marill relay winner eevee", "alone"),
    ] {
        let film =
            format!("scene: amphitheater pikachu eevee krabby marill\npikachu put left\neevee put 300 200\nkrabby put right\nmarill put 500 200\n{said}\n");
        assert_eq!(fight_of(&film).0[1], sort, "{said}");
    }
}

#[test]
fn a_fight_ends_but_what_lasts_stays() {
    let mut fight = Combat {
        lead: Some("pikachu".to_string()),
        bars: vec![("pikachu".to_string(), 2)],
        written: 3,
        moments: vec![("mewtwo".to_string(), 50, Vec::new())],
        ..Combat::default()
    };
    fight.levels.insert("pikachu".to_string(), 30);
    fight.end();
    assert!(fight.lead.is_none() && fight.bars.is_empty() && fight.moments.is_empty());
    assert_eq!((fight.levels.len(), fight.written), (1, 3));
}
