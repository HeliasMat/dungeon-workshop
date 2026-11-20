use super::super::tests::rules;
use super::*;
use crate::vocabulary::POKEMON;
use engine::{read_events, read_scene};

/// A scene played to its end, a fight in it, the first of the menu taken.
fn played(line: &str, numbers: &[&str]) -> Vec<i32> {
    staged(line, numbers).0
}

/// The same, and the stage as it ended (what it noted wrong).
fn staged(line: &str, numbers: &[&str]) -> (Vec<i32>, Stage) {
    let text = format!(
        "event\t0\tE\t1\t1\nscene\t1\ta\nput\tm1\t100\t100\tdown\nput\tm2\t220\t100\tdown\ngoto\tpast\nlabel\tmoment\nset\tseen\t1\nreturn\nlabel\tpast\n{line}\nset\tdone\t1\n"
    );
    let script = read_scene(&text, None, &POKEMON);
    assert!(script.problems.is_empty(), "{:?}", script.problems);
    let events = read_events(script.lines.into_iter().map(|(_, line)| line), &POKEMON);
    let (mut director, mut stage, mut fight) = (Director::new(events, 0, &POKEMON), Stage::default(), None::<Fighting>);
    let other = |_: Who| 0;
    for _ in 0..200_000 {
        let mut calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
        if let Some(going) = &mut fight {
            going.tick(1.0, &mut director, &mut stage, Vec2::ZERO, &mut calls);
            if going.menu().is_some() {
                going.choose(0);
            }
            if going.over() {
                fight = None;
            }
        }
        for call in calls {
            match call {
                Call::Host { fields, .. } => called(&mut fight, &fields, &|_| Some(rules()), &|_| Ground::default(), &other, &mut director, &mut stage),
                Call::Over => return (numbers.iter().map(|name| director.numbers().get(*name).map_or(-1, |value| value.round() as i32)).collect(), stage),
                _ => {}
            }
        }
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    }
    panic!("the scene does not end");
}

#[test]
fn a_played_fight_ends_and_the_scene_goes_on() {
    let [done, left] = played("fight\tyour_turn\tseed\t3\tside\tm1\tside\tm2", &["done", "hp2"])[..] else { panic!() };
    assert_eq!(done, 1);
    assert!(left == 0 || left == 1);
}

#[test]
fn a_crowd_named_like_a_word_of_the_fight_leaves_it_whole() {
    let (seen, stage) = staged("group\tside\tm1,m2\nfight\tyour_turn\tseed\t3\tside\tm1\tside\tm2", &["done", "hp2"]);
    assert_eq!(seen[0], 1);
    assert!(seen[1] == 0 || seen[1] == 1, "{seen:?}");
    assert!(stage.place.notes.iter().all(|note| !note.wrong));
}

#[test]
fn a_fight_it_cannot_read_is_told() {
    let (seen, stage) = staged("fight\tyour_turn\tlevel\tx\tside\tm1\tside\tm2", &["done"]);
    assert_eq!(seen, [1]);
    assert!(stage.place.notes.iter().any(|note| note.wrong && note.text.contains("unreadable fight")));
}

#[test]
fn a_passage_plays_at_its_moment_and_the_fight_goes_on() {
    let seen = played("fight\talone\tseed\t3\tside\tm1\tside\tm2\twhen\tm2\t60\tmoment\twinner\tm1", &["seen", "done", "hp2"]);
    assert_eq!(seen, [1, 1, 0]);
}

#[test]
fn a_tournament_says_how_it_ended() {
    let seen = played("fight\ttournament\tseed\t3\tside\tm1\tside\tm2\tm2.1", &["champion", "done"]);
    assert!(seen[0] == 0 || seen[0] == 1);
    assert_eq!(seen[1], 1);
}

#[test]
fn lines_of_a_film_show_a_fight_by_hand() {
    let lines = [
        "fight\topen\tside\tm1\tside\tm2\tlead\tm1\tbar\tm1\t30\tbar\tm2\t20",
        "fight\tblow\tside\tm1\tside\tm2\tpower\t154\ttakes\t7\tcrit\tlead\tm1\tbar\tm1\t30\tbar\tm2\t20",
        "fight\tblow\tside\tm2\tside\tm1\tpower\t154\tmiss\tlead\tm1\tbar\tm1\t30\tbar\tm2\t20",
        "fight\tstat\tside\tm1\tup\tspeed",
        "fight\tberry\tside\tm1\theals\t5",
        "fight\tend\tlead\tm1\tbar\tm1\t30\tbar\tm2\t20",
    ];
    let seen = played(&lines.join("\n"), &["hp", "hp2", "summary_blows", "summary_misses", "summary_crits", "summary_hp", "done"]);
    assert_eq!(seen, [30, 13, 1, 0, 1, 30, 1]);
}

#[test]
fn a_line_of_its_own_in_a_passage_plays_and_the_fight_goes_on() {
    let text = "fight\talone\tseed\t3\tside\tm1\tside\tm2\twhen\tm2\t60\tmoment\twinner\tm1";
    let seen = played(text, &["seen", "done"]);
    assert_eq!(seen, [1, 1]);
}

#[test]
fn a_fight_stated_in_advance_is_read_and_played() {
    let line: Vec<String> = "fight directed level 60 seed 2 side m1 m2 side m3 hp m3 320 turns 8 rest winner m2 act m1 154 m3 -30 act m3 52 all misses act m2 154 critical act m1 154 m3 ko act m1 154 protect".split(' ').map(str::to_string).collect();
    let asked = Asked::read(&line).unwrap();
    assert_eq!(asked.lives[&Who::Monster(3)], (320, 320));
    assert_eq!((asked.direction.turns, asked.direction.goes_on, asked.direction.acts.len()), (Some(8), true, 5));
    let acts = &asked.direction.acts;
    let act = |at: usize| match &acts[at] {
        Stated::Act(act) => act.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(act(0), Act { targets: vec![Who::Monster(3)], told: Told::Takes(30), ..Act::of(Who::Monster(1), 154) });
    assert!(act(1).all);
    assert_eq!((act(1).told, act(2).told, act(3).told, act(4).told), (Told::Misses, Told::Critical, Told::Falls, Told::Shielded));
    let text = "fight\tdirected\tseed\t3\tside\tm1\tside\tm2\twinner\tm1\tact\tm1\t154\tm2\tko";
    assert_eq!(played(text, &["hp2", "done"]), [0, 1]);
}

#[test]
fn a_stated_fight_says_all_it_can() {
    let line: Vec<String> = "fight directed side m1 m2 side m3 m4 reserve m2 great_bars m1 m3 act m1 154 m3 critical -60 slow slowmo 30 intercepts m4 scene told pause 60 note It_falls change m1 m2".split(' ').map(str::to_string).collect();
    let asked = Asked::read(&line).unwrap();
    assert_eq!(asked.great_bars, Some((Who::Monster(1), Who::Monster(3))));
    assert_eq!(asked.waiting, [Who::Monster(2)]);
    let acts = &asked.direction.acts;
    assert_eq!(
        acts[0],
        Stated::Act(Act {
            targets: vec![Who::Monster(3)],
            told: Told::Takes(60),
            critical: true,
            slow: true,
            slowed_hit: Some(30),
            taken_by: Some(Who::Monster(4)),
            ..Act::of(Who::Monster(1), 154)
        })
    );
    assert_eq!(acts[1], Stated::Passage("told".to_string()));
    assert_eq!(acts[2], Stated::Pause(60));
    assert_eq!(acts[3], Stated::Log("It_falls".to_string()));
    assert_eq!(acts[4], Stated::Change { out: Who::Monster(1), comes: Who::Monster(2) });
}

#[test]
fn a_line_said_between_two_blows_plays_and_the_fight_goes_on() {
    let text = "fight\tdirected\tseed\t3\tside\tm1\tside\tm2\twinner\tm1\tact\tm1\t154\tm2\t-1\tscene\tmoment\tact\tm1\t154\tm2\tko";
    assert_eq!(played(text, &["seen", "hp2", "done"]), [1, 0, 1]);
}

#[test]
fn a_line_is_read_whole() {
    let line: Vec<String> = "fight alone level 40 seed 9 place 240 side m1 m4 side m2 keep bar m1 60 lv m1 12 knows m1 154 52 species m4 2 reviver m1 support m9 audience m7 m8 winner m1".split(' ').map(str::to_string).collect();
    let mut asked = Asked::read(&line).unwrap();
    asked.lives_from(&[("hp".to_string(), 10.0)].into());
    assert_eq!((asked.level, asked.seed, asked.place, asked.kept), (40, 9, 240, true));
    assert_eq!(asked.sides[0], [Who::Monster(1), Who::Monster(4)]);
    assert_eq!(asked.lives[&Who::Monster(1)], (10, 60));
    assert_eq!(asked.moves[&Who::Monster(1)], [154, 52]);
    assert_eq!(asked.winner, Some(Who::Monster(1)));
    assert_eq!(asked.public.len(), 2);
}

#[test]
fn a_line_reads_its_english_words() {
    let read = |text: &str| Asked::read(&text.split(' ').map(str::to_string).collect::<Vec<String>>()).unwrap();
    let new = read("fight your_turn level 40 seed 9 place 240 side m1 side m2 support m9 when m2 60 moment");
    assert_eq!((new.sort, new.level, new.seed, new.place), (Sort::Played, 40, 9, 240));
    let new = read("fight directed side m1 m2 side m3 hp m3 320 winner m2 act m1 154 m3 critical slow uses m1 i1 name:Apple 5 present m9");
    assert_eq!((new.sort, new.direction.acts.len()), (Sort::Directed, 2));
    assert!(Asked::read(&"fight joue niveau 40".split(' ').map(str::to_string).collect::<Vec<String>>()).is_none(), "the old French words are gone");
    let berry = read("fight berry side m1 ailment burn up speed");
    assert_eq!((berry.sort, berry.written.ailment, berry.written.stat), (Sort::Hand(hand::Line::Berry), Some(Ailment::Burnt), Some((Stat::Speed, true))));
}
