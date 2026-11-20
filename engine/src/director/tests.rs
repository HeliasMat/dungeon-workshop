use super::*;
use crate::vocabulary::SAMPLE;

fn story(text: &str) -> Director {
    let lines = crate::language::read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line);
    Director::new(read_events(lines, &SAMPLE), 0, &SAMPLE)
}

#[test]
fn before_someone_is_where_it_faces_now_even_turned_by_a_line() {
    // (Told to look down; the line has it turn to who spoke before.)
    let text = "event\t0\tE\t1\t1\nscene\t1\ta\nput\t0\t100\t100\tdown\nput\t10\t200\t100\tdown\nautoface\ton\nsay\t10\t-\tA\ntimed\t2\nsaid\nsay\t0\t-\tB\ntimed\t2\nsaid\nfacing\t_x\t_y\t0\t30\n";
    let (mut director, mut stage) = (story(text), Stage::default());
    for _ in 0..12 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
    }
    assert_eq!(stage.cast[&Who::Character(0)].direction, 3);
    assert_eq!((director.numbers()["_x"], director.numbers()["_y"]), (130.0, 100.0));
}

#[test]
fn before_someone_a_wall_moves_the_point_and_behind_is_the_other_way() {
    let mut stage = Stage::default();
    stage.put(Who::Character(0), Vec2::new(100.0, 100.0), 3);
    assert_eq!(stage.before(Who::Character(0), -30.0, None), Some(Vec2::new(70.0, 100.0)));
    stage.place_walls([[120.0, 60.0, 150.0, 140.0]]);
    assert!(stage.before(Who::Character(0), 30.0, None).unwrap().x < 120.0 || stage.before(Who::Character(0), 30.0, None).unwrap().x > 150.0);
}

#[test]
fn a_sound_waits_for_the_next_hit() {
    let text = "event\t0\tE\t1\t1\nscene\t1\ta\nput\t0\t100\t100\tdown\nput\t10\t200\t100\tdown\nonhit\t0\tmovesound\t44\nt\t5\nrecoil\t0\t10\nt\t20\n";
    let (mut director, mut stage) = (story(text), Stage::default());
    // (Heard from the side of who is hit: right of where the view looks.)
    let heard = |calls: &[Call]| {
        calls.iter().any(|call| {
            matches!(call, Call::Show(Beat::Sound(SoundBeat::Play(Played { of: SoundOf::Move(MoveId(44)), pan: Side::At(pan), volume: 1.0, .. }))) if *pan > 0.5)
        })
    };
    assert!(!heard(&director.tick(&mut stage, 1.0, Vec2::ZERO)));
    let mut times = 0;
    for _ in 0..12 {
        times += usize::from(heard(&director.tick(&mut stage, 1.0, Vec2::ZERO)));
    }
    assert_eq!(times, 1);
}

#[test]
fn events_are_read_with_their_scenes() {
    let events = read_events(
        crate::language::read_scene(
            "event\t5\tEVENT_A\t1\t2\nput\t0\t1\t2\tdown\nscene\t11\tb\nactor\t3\t10\t20\t1\nsay\t3\t0\tHi\nevent\t6\tEVENT_B\n",
            None,
            &SAMPLE,
        )
        .lines
        .into_iter()
        .map(|(_, line)| line),
        &SAMPLE,
    );
    assert_eq!(events.len(), 2);
    assert_eq!((events[0].number, events[0].name.as_str(), events[0].moment), (5, "EVENT_A", (1, 2)));
    assert_eq!(events[0].scenes.len(), 2);
    assert_eq!(events[0].scenes[1].level, 11);
    assert!(matches!(events[0].scenes[1].beats[0], Beat::Course(Course::Say { entity: Some(Who::Character(3)), .. })));
    assert!(events[1].scenes.is_empty());
}

#[test]
fn a_scene_waits_for_time_lines_and_walkers() {
    let mut director =
        story("event\t0\tE\nscene\t11\ta\nput\t0\t0\t0\tdown\ngo\t0\t10\t0\t10\t0\nwaitfor\t0\nwait\t5\nsay\t0\t0\tHello\nput\t1\t5\t5\tdown\nsaid\nend\n");
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(calls[0], Call::Opens { first: true, cleared: true });
    assert!(matches!(calls[1], Call::Show(Beat::Body(BodyBeat::Put { who: Who::Character(0), .. }))));
    assert_eq!(director.at.unwrap().beat, 2, "waits for the walker");
    for _ in 0..10 {
        stage.advance(1.0, 0.0, None);
    }
    assert!(director.tick(&mut stage, 1.0, Vec2::ZERO).is_empty());
    assert_eq!(director.at.unwrap().beat, 4, "then for the time");
    director.tick(&mut stage, 4.0, Vec2::ZERO);
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(matches!(&calls[..], [Call::Say { text, .. }] if text == "Hello"));
    // (The scene goes on while the line is on screen, up to `said`.)
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(matches!(&calls[..], [Call::Show(Beat::Body(BodyBeat::Put { who: Who::Character(1), .. }))]));
    assert!(director.tick(&mut stage, 1.0, Vec2::ZERO).is_empty());
    director.read();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.tick(&mut stage, 1.0, Vec2::ZERO), vec![Call::Over]);
    assert_eq!(director.at, None);
}

#[test]
fn passages_play_meanwhile() {
    let mut director = story(
        "event\t0\tE\nscene\t1\ta\nset\tn\t0\nparallel\tcompte\nwait\t25\nparallel\toff\nwait\t30\nend\nlabel\tcompte\nadd\tn\t1\nwait\t10\ngoto\tcompte\n",
    );
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars["n"], 0.0, "started, not played yet");
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars["n"], 1.0, "played while the scene waits");
    director.tick(&mut stage, 10.0, Vec2::ZERO);
    assert_eq!(director.vars["n"], 2.0, "with its own waits and goto");
    director.tick(&mut stage, 20.0, Vec2::ZERO);
    let stopped = director.vars["n"];
    director.tick(&mut stage, 20.0, Vec2::ZERO);
    assert_eq!(director.vars["n"], stopped, "stopped by parallel off");
}

#[test]
fn answers_offered_when_their_test_holds() {
    let mut director = story(
        "event\t0\tE\nscene\t1\ta\nset\tn\t0\nask\t[if n > 0]Apple|Yes|No\nanswer\t0\nsay\t-\t-\tapple\nanswer\t1\nsay\t-\t-\tyes\nanswer\t2\nsay\t-\t-\tno\nend\n",
    );
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(calls[1], Call::Ask(vec!["Yes".into(), "No".into()]));
    director.answer(Some(1));
    assert_eq!(director.vars[ANSWER], 2.0, "its place among all of them");
    assert!(matches!(&director.tick(&mut stage, 1.0, Vec2::ZERO)[..], [Call::Say { text, .. }] if text == "no"));
}

#[test]
fn words_kept_on_screen_follow_their_numbers() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nset\tn\t3\nhud\tScore : {n}\nhud\tleft\tVies : 3\nwait\t5\nadd\tn\t1\nwait\t5\nhud\toff\nwait\t5\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!((stage.words.hud[2].as_deref(), stage.words.hud[0].as_deref()), (Some("Score : 3"), Some("Vies : 3")));
    director.tick(&mut stage, 5.0, Vec2::ZERO);
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.words.hud[2].as_deref(), Some("Score : 4"));
    director.tick(&mut stage, 5.0, Vec2::ZERO);
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!((stage.words.hud[2].as_deref(), stage.words.hud[0].as_deref()), (None, Some("Vies : 3")));
}

#[test]
fn numbers_are_decimals_and_written_as_such() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nset\tx\t0.5\ncalc\tm\t7\t/\t2\ncalc\ts\tx\t+\t1\nadd\ts\t0.25\nset\tn\t3\nwait\t600\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!((director.vars["x"], director.vars["m"], director.vars["s"]), (0.5, 3.5, 1.75));
    assert_eq!(director.fill_once("{m} and {n}"), "3.5 and 3", "a whole number as always");
    assert_eq!(number_text(1.0 / 3.0), "0.333");
}

#[test]
fn countdowns_named_go_each_their_way() {
    let mut director = story("event\t0\tE\nscene\t1\ta\ntimer\tbombe\t1s\ntimer\t3s\nwait\t600\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!((director.vars["timer_bombe"], director.vars["timer"]), (1.0, 3.0));
    director.tick(&mut stage, 60.0, Vec2::ZERO);
    assert_eq!((director.vars["timer_bombe"], director.vars["timer"]), (0.0, 2.0));
}

#[test]
fn ways_of_an_if_go_to_the_first_that_holds() {
    let mut director = story(
        "event\t0\tE\nscene\t1\ta\nset\tn\t2\nif\tn == 1\nset\tr\t1\nelif\tn == 2\nif\tn > 5\nset\tr\t9\nendif\nset\tr\t2\nelif\tn > 1\nset\tr\t3\nelse\nset\tr\t4\nendif\nlabel\tfin\nwait\t5\n",
    );
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars["r"], 2.0);
}

#[test]
fn loops_are_left_and_gone_round_again() {
    // (Counts to 10, skips the odd ones, stops at 6.)
    let mut director = story(
        "event\t0\tE\nscene\t1\ta\nset\ti\t0\nset\tpairs\t0\nwhile\ti < 10\nadd\ti\t1\nif\ti == 6\nbreak\nendif\nset\tr\t0\nadd\tr\ti\nwhile\tr >= 2\nadd\tr\t-2\nendwhile\nif\tr == 1\ncontinue\nendif\nadd\tpairs\t1\nendwhile\nwait\t5\n",
    );
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!((director.vars["i"], director.vars["pairs"]), (6.0, 2.0));
}

#[test]
fn questions_given_their_time() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nasktimer\t1s\t1\nask\tA|B\nanswer\t0\nsay\t-\t-\ta\nanswer\t1\nsay\t-\t-\tb\nend\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.hold, Some(Hold::Question));
    director.tick(&mut stage, 30.0, Vec2::ZERO);
    assert_eq!(director.vars[TIME_LEFT], 1.0);
    let calls = director.tick(&mut stage, 31.0, Vec2::ZERO);
    assert_eq!(calls, vec![Call::LineGone]);
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(matches!(&calls[..], [Call::Say { text, .. }] if text == "b"), "{calls:?}");
    assert_eq!((director.vars[ANSWER], director.vars[TOO_LATE]), (1.0, 1.0));
}

#[test]
fn enter_talks_to_who_is_near() {
    let mut director = story(
        "event\t0\tE\nscene\t1\ta\nput\t0\t0\t0\tdown\nput\tm133\t20\t0\tdown\nput\ti109\t100\t0\tdown\ncontrol\t0\nwaituntil\ttalking\nif\ttarget == m133\nset\tok\t1\nendif\nwait\t5\n",
    );
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.press(&stage);
    assert_eq!((director.vars[PRESSES], director.vars[TALKS], director.vars[TARGET]), (1.0, 1.0, Who::Monster(133).number() as f64));
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars["ok"], 1.0, "who it is, by the name the file gives");
    // (Turned away from it: not in front any more.)
    stage.cast.get_mut(&Who::Character(0)).unwrap().direction = 7;
    director.press(&stage);
    assert_eq!(director.vars[TALKS], 0.0, "behind it");
    stage.cast.get_mut(&Who::Character(0)).unwrap().direction = 3;
    stage.cast.get_mut(&Who::Monster(133)).unwrap().position.x = 60.0;
    director.press(&stage);
    assert_eq!(director.vars[TALKS], 0.0, "no one close enough");
}

#[test]
fn a_press_while_a_talk_plays_is_not_a_second_talk() {
    let mut director = story(
        "event\t0\tE\nscene\t1\ta\nput\t0\t0\t0\tdown\nput\tm133\t20\t0\tdown\ncontrol\t0\ngoto\tpast\nlabel\ttalk\nadd\tn\nwait\t5\nreturn\nlabel\tpast\nontalk\tm133\ttalk\nroam\n",
    );
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.press(&stage);
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars.get("n"), Some(&1.0), "talked to");
    // (Enter pressed again while its passage plays, as between two of its lines.)
    director.press(&stage);
    for _ in 0..20 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
    }
    assert_eq!(director.vars.get("n"), Some(&1.0), "once, not twice");
    director.press(&stage);
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars.get("n"), Some(&2.0), "a press once back roaming talks again");
}

/// The host's lines among calls, as written.
fn hosted(calls: &[Call]) -> Vec<String> {
    let line = |call: &Call| match call {
        Call::Host { fields, .. } => Some(fields.join(" ")),
        _ => None,
    };
    calls.iter().filter_map(line).collect()
}

#[test]
fn money_given_by_a_number() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nset\tprix\t250\ngetmoney\t{prix}\ngetmoney\t-prix\nset\tn\t2\ngetitem\t109\tn\nwait\t5\n");
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(hosted(&calls), ["getmoney 250", "getmoney -250", "getitem 109 2"]);
}

#[test]
fn portraits_on_the_right_and_lines_slower() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nportrait\tm25\tright\ntextspeed\t0.5\nsay\tm25\t0\tLentement...\nsaid\n");
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(calls.iter().any(|call| matches!(call, Call::Say { right: true, speed: Some(speed), .. } if *speed == 0.5)), "{calls:?}");
}

/// A save kept past a question comes back there, with what it passed played
/// at once.
#[test]
fn a_kept_place_comes_back_past_the_question() {
    let text = "event\t0\tE\nscene\t1\ta\nsay\t-\t-\tfirst\nsaid\nask\tA|B\nanswer\t0\nsay\t-\t-\ta\nsaid\nanswer\t1\nsay\t-\t-\tb\nsaid\nend\nsay\t-\t-\tlast\nsaid\n";
    let mut stage = Stage::default();
    let mut director = story(text);
    for _ in 0..4 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
        director.read();
    }
    assert_eq!(director.hold(), Some(Hold::Question));
    director.answer(Some(0));
    director.keep_place();
    let place = director.checkpoint().cloned();
    let mut again = story(text);
    again.author.skip_to = place;
    let mut said = Vec::new();
    for _ in 0..200 {
        for call in again.tick(&mut stage, 1.0, Vec2::ZERO) {
            if let Call::Say { text, .. } = call {
                said.push(text);
            }
        }
        again.read();
    }
    assert_eq!((said, again.author.skip_to), (vec!["a".to_string()], None));
}

#[test]
fn countdowns_checkpoints_and_questions() {
    let mut director =
        story("event\t0\tE\nscene\t1\ta\ntimer\t2s\nwait\t30\ncheckpoint\tici\nlabel\tici\nask\tA|B\nanswer\t0\nsay\t-\t-\ta\nanswer\t1\nsay\t-\t-\tb\nend\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars[TIMER], 2.0);
    let calls = director.tick(&mut stage, 60.0, Vec2::ZERO);
    assert_eq!(director.vars[TIMER], 1.0);
    assert!(calls.contains(&Call::Checkpoint) && director.checkpoint.as_deref() == Some("ici"), "{calls:?}");
    director.answer(Some(1));
    assert!(matches!(&director.tick(&mut stage, 1.0, Vec2::ZERO)[..], [Call::Say { text, .. }] if text == "b"));
}

#[test]
fn waits_drawn_between_two() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nt\t10~20\nset\tdone\t1\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.tick(&mut stage, 8.0, Vec2::ZERO);
    assert!(!director.vars.contains_key("done"), "not before 10 frames");
    director.tick(&mut stage, 12.0, Vec2::ZERO);
    assert_eq!(director.vars.get("done"), Some(&1.0), "by 20");
}

#[test]
fn a_number_chosen() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nasknumber\tapples\t0\t10\t2\nset\tdone\t1\n");
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(calls[1], Call::Ask(["0", "2", "4", "6", "8", "10"].map(String::from).to_vec()));
    assert!(director.tick(&mut stage, 1.0, Vec2::ZERO).is_empty() && !director.vars.contains_key("done"));
    director.answer(Some(3));
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!((director.vars["apples"], director.vars["done"]), (6.0, 1.0));
}

#[test]
fn a_question_under_its_line() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nsay\tm25\tjoyous\tHow many?\nask\tOne|Two\nanswer\t0\nanswer\t1\nend\n");
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(matches!(&calls[1..], [Call::Say { text, .. }] if text == "How many?"), "{calls:?}");
    // (Asked at once, the line still on screen: not waited for.)
    assert_eq!(director.tick(&mut stage, 1.0, Vec2::ZERO), vec![Call::Ask(vec!["One".into(), "Two".into()])]);
    assert_eq!((director.hold, director.reading), (Some(Hold::Question), false));
}

#[test]
fn whoever_enter_found_acts() {
    let mut director = story(
        "event\t0\tE\nscene\t1\ta\nput\t0\t0\t0\tright\nput\tm133\t20\t0\tdown\ncontrol\t0\nwaituntil\ttalking\nface\ttarget\t0\nsay\ttarget\t-\tThanks, {name:target}!\nsaid\n",
    );
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.press(&stage);
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.cast[&Who::Monster(133)].direction, 7, "turned to the leader");
    assert!(calls.iter().any(|call| matches!(call, Call::Say { entity: Some(Who::Monster(133)), text, .. } if text == "Thanks, {name:m133}!")), "{calls:?}");
}

#[test]
fn gauges_follow_their_numbers() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nset\tvie\t30\nset\tmax\t40\ngauge\tleft\tvie\tmax\tred\nwait\t5\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.words.gauges[0], Some(crate::Gauge { full: 0.75, colour: [255, 40, 40] }));
    assert_eq!(stage.words.gauges[2], None);
}

#[test]
fn played_at_once_a_loop_for_the_player_is_left() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nset\tdone\t0\nwhile\tdone == 0\nwaituntil\ttalking\nendwhile\nlabel\tpast\nset\tpassed\t1\nwait\t5\n");
    director.author.skip_to = Some("after".to_string());
    let mut stage = Stage::default();
    for _ in 0..3 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
    }
    assert_eq!(director.vars.get("passed"), Some(&1.0), "past the loop");
}

#[test]
fn places_kept_and_waited_for() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nput\t0\t10\t20\tdown\nposition\t0\tx\ty\nwaitnear\t0\t100\t20\t8\nset\tarrive\t1\nwait\t5\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!((director.vars["x"], director.vars["y"]), (10.0, 20.0));
    assert!(!director.vars.contains_key("arrive"), "waits to be there");
    stage.cast.get_mut(&Who::Character(0)).unwrap().position = Vec2::new(98.0, 20.0);
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars["arrive"], 1.0);
}

#[test]
fn someone_shown_as_another() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nput\tm2501\t0\t0\tdown\ndisguise\tm2501\tm2502\nsay\tm2501\t0\tI evolved!\nsaid\n");
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(calls.contains(&Call::Show(Beat::Body(BodyBeat::Disguise { who: Who::Monster(2501), like: Some(Who::Monster(2502)) }))), "{calls:?}");
    assert!(calls.iter().any(|call| matches!(call, Call::Say { entity: Some(Who::Monster(2502)), .. })), "says it as the other");
}

#[test]
fn words_written_and_kept() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nsettext\tplace\tthe beach\nasktext\tnickname\tZoroark\nsay\t-\t-\t{nickname}, at {place}!\nsaid\n");
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(calls.last(), Some(&Call::AskText("Zoroark".to_string())));
    director.text_given("Shadow".to_string());
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(matches!(&calls[..], [Call::Say { text, .. }] if text == "Shadow, at the beach!"), "{calls:?}");
}

#[test]
fn words_compared() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nasktext\tmot\nif\tmot == pikachu\nset\tgood\t1\nendif\nwait\t60\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.text_given(" Pikachu ".to_string());
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars["good"], 1.0);
}

#[test]
fn a_line_said_meanwhile_goes_by_itself() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nparallel\tcomment\nwait\t300\nset\tdone\t1\nend\nlabel\tcomment\nsay\t-\t-\tGo!\nsaid\nreturn\n");
    let mut stage = Stage::default();
    let calls: Vec<Call> = (0..2).flat_map(|_| director.tick(&mut stage, 1.0, Vec2::ZERO)).collect();
    assert!(calls.iter().any(|call| matches!(call, Call::Say { text, .. } if text == "Go!")), "{calls:?}");
    assert!(director.reading && director.line_timed());
    let gone = (0..120).any(|_| director.tick(&mut stage, 1.0, Vec2::ZERO).contains(&Call::LineGone));
    assert!(gone && !director.reading);
}

#[test]
fn a_question_in_a_loop() {
    let mut director =
        story("event\t0\tE\nscene\t1\ta\nset\tn\t0\nwhile\tn < 2\nask\tA|B\nanswer\t0\nadd\tn\t1\nanswer\t1\nadd\tn\t10\nendask\nendwhile\nset\tdone\t1\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    let mut answered = |director: &mut Director| {
        director.answer(Some(0));
        for _ in 0..3 {
            director.tick(&mut stage, 1.0, Vec2::ZERO);
        }
    };
    answered(&mut director);
    assert_eq!(director.vars["n"], 1.0);
    answered(&mut director);
    assert_eq!((director.vars["n"], director.vars.get("done")), (2.0, Some(&1.0)));
}

#[test]
fn an_event_played_after_the_end_leaves_the_story_over() {
    let mut director = story("event\t1\tE\nscene\t1\ta\nset\ta\t1\n");
    let mut stage = Stage::default();
    for _ in 0..6 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
    }
    assert!(director.over());
    director.interlude(story("event\t0\tX\nscene\t1\tx\nset\tx\t1\n").events.remove(0));
    assert!(director.over() && director.at.is_some());
    let calls: Vec<Call> = (0..6).flat_map(|_| director.tick(&mut stage, 1.0, Vec2::ZERO)).collect();
    assert_eq!(director.vars.get("x"), Some(&1.0));
    assert!(director.at.is_none() && calls.contains(&Call::Over), "{calls:?}");
}

#[test]
fn an_event_played_now_goes_back_where_the_story_was() {
    let mut director = story("event\t1\tE\nscene\t1\ta\nset\ta\t1\nwait\t10\nset\tb\t1\nevent\t2\tF\nscene\t1\tb\nset\tc\t1\n");
    let mut stage = Stage::default();
    for _ in 0..3 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
    }
    assert!(director.vars.contains_key("a") && !director.vars.contains_key("b"));
    let now = story("event\t0\tX\nscene\t1\tx\nset\tx\t1\n").events.remove(0);
    let then = story("event\t0\tY\nscene\t1\ty\nset\ty\t1\n").events.remove(0);
    director.interlude(now);
    assert!(director.in_interlude());
    // (A second one, once the first is played: back where the story
    // was all the same.)
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.vars.get("x"), Some(&1.0));
    director.interlude(then);
    for _ in 0..30 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
    }
    // (The event it was in goes on from where it was: `a` not set
    // again, `b` set; then the next.)
    assert!(!director.in_interlude());
    assert_eq!((director.vars.get("y"), director.vars.get("b"), director.vars.get("c")), (Some(&1.0), Some(&1.0), Some(&1.0)));
}

#[test]
fn lines_filled_when_played() {
    let mut director = story(
        "event\t0\tE\nscene\t1\ta\nset\tnotes1\t3\nset\tnotes2\t1\nset\tnotes3\t2\nset\tnotes_nombre\t3\nset\ti\t2\nset\tx\t40\nput\tm25\t{x}\t20\tdown\nset\tnote\tnotes{i}\nset\tencore\t{notes{i}}\nwait\t5\n",
    );
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.cast[&Who::Monster(25)].position, Vec2::new(40.0, 20.0));
    assert_eq!((director.vars["note"], director.vars["encore"], director.vars["notes_nombre"]), (1.0, 1.0, 3.0));
}

#[test]
fn numbers_reckoned() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nset\ta\t17\ncalc\tb\ta\t%\t5\ncalc\tc\ta\t/\t0\ncalc\td\tb\tmax\t9\ncalc\te\ta\t*\tb\nwait\t5\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    let vars = &director.vars;
    assert_eq!((vars["b"], vars["c"], vars["d"], vars["e"]), (2.0, 0.0, 9.0, 34.0));
}

#[test]
fn questions_and_menus_hold_the_story() {
    let mut director =
        story("event\t0\tE\nscene\t1\ta\nask\tYes|No\nanswer\t0\nsay\t-\t-\tyes\nanswer\t1\nsay\t-\t-\tno\nend\nscene\t2\tb\nmenu\t7\nground\t9\n");
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(calls[1], Call::Ask(vec!["Yes".into(), "No".into()]));
    assert!(director.tick(&mut stage, 1.0, Vec2::ZERO).is_empty());
    director.answer(Some(1));
    assert!(matches!(&director.tick(&mut stage, 1.0, Vec2::ZERO)[..], [Call::Say { text, .. }] if text == "no"));
    director.read();
    // (Past the answer's way: the next scene.)
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(hosted(&calls), ["menu 7"]);
    assert_eq!(director.tick(&mut stage, 1.0, Vec2::ZERO), vec![]);
    director.menu_done(false);
    assert_eq!(hosted(&director.tick(&mut stage, 1.0, Vec2::ZERO)), ["ground 9"]);
}

/// The lines a director says, up to the end, each read at once.
fn said(director: &mut Director) -> Vec<String> {
    let (mut stage, mut lines) = (Stage::default(), Vec::new());
    for _ in 0..200 {
        for call in director.tick(&mut stage, 1.0, Vec2::ZERO) {
            if let Call::Say { text, .. } = call {
                lines.push(text);
            }
        }
        director.read();
    }
    lines
}

#[test]
fn numbers_choose_the_way_and_loops_go_round() {
    let text = "event\t0\tE\nscene\t1\ta\nset\tn\t0\nlabel\ttop\nadd\tn\nif\tn\t<\t3\nsay\t-\t-\tagain\ngoto\ttop\nelse\nsay\t-\t-\tdone\nendif\nif\tn\t==\tn\nif\tmissing\nsay\t-\t-\tnever\nendif\nsay\t-\t-\tlast\nendif\n";
    let mut director = story(text);
    assert_eq!(said(&mut director), ["again", "again", "done", "last"]);
    assert_eq!(director.vars["n"], 3.0);
}

#[test]
fn a_way_holds_whole_scenes() {
    // (The other way's scenes are not played, either way; the question
    // in the first way ends it, its answer closing the way around it.)
    let text = "event\t0\tE\nscene\t1\ta\nask\tA|B\nanswer\t0\nscene\t1\tb\nsay\t-\t-\tA\nscene\t1\tc\nask\tC|D\nanswer\t0\nanswer\t1\nend\nanswer\t1\nscene\t1\td\nsay\t-\t-\tB\nend\nscene\t1\te\nsay\t-\t-\tlast\n";
    for (chosen, lines) in [(0, ["A", "last"]), (1, ["B", "last"])] {
        let mut director = story(text);
        let mut stage = Stage::default();
        director.tick(&mut stage, 1.0, Vec2::ZERO);
        director.answer(Some(chosen));
        let mut seen = said(&mut director);
        if director.hold == Some(Hold::Question) {
            director.answer(Some(0));
            seen.extend(said(&mut director));
        }
        assert_eq!(seen, lines);
    }
}

#[test]
fn a_question_left_open_ends_with_its_scene() {
    let text = "event\t0\tE\nscene\t1\ta\nask\tA|B\nanswer\t0\nsay\t-\t-\tA\nanswer\t1\nsay\t-\t-\tB\nscene\t1\tb\nsay\t-\t-\tlast\n";
    let mut director = story(text);
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.answer(Some(0));
    assert_eq!(said(&mut director), ["A", "last"]);
}

#[test]
fn the_answer_chosen_is_kept() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nask\tA|B\nanswer\t0\nanswer\t1\nend\nscene\t1\tb\nif\tanswer\t==\t1\nsay\t-\t-\tB\nendif\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.answer(Some(1));
    assert_eq!(director.vars[ANSWER], 1.0);
    assert_eq!(said(&mut director), ["B"]);
}

#[test]
fn a_scene_waits_for_the_view_and_the_light() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nfadein\t10\nwaitfade\nzoom\t2\t10\nwaitcam\nsay\t-\t-\tseen\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    for _ in 0..9 {
        stage.advance(1.0, 0.0, None);
        assert!(director.tick(&mut stage, 1.0, Vec2::ZERO).is_empty());
    }
    let mut calls = Vec::new();
    for _ in 0..12 {
        stage.advance(1.0, 0.0, None);
        calls.extend(director.tick(&mut stage, 1.0, Vec2::ZERO));
    }
    assert!(matches!(&calls[..], [Call::Say { text, .. }] if text == "seen"), "{calls:?}");
}

#[test]
fn later_beats_come_in_their_time_and_numbers_fill_lines() {
    let text = "event\t0\tE\nscene\t1\ta\nput\t0\t0\t0\tdown\nafter\t5\tturn\t0\tright\nset\tberries\t4\nsay\t-\t-\tYou have {berries} berries, {nobody}.\n";
    let mut director = story(text);
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(matches!(calls.last(), Some(Call::Say { text, .. }) if text == "You have 4 berries, {nobody}."), "{calls:?}");
    for _ in 0..4 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
    }
    assert_eq!(stage.cast[&Who::Character(0)].direction, 1, "not yet");
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.cast[&Who::Character(0)].direction, 3);
}

#[test]
fn a_scene_is_skipped_up_to_a_label() {
    let text = "event\t0\tE\nscene\t1\ta\nput\t0\t0\t0\tdown\nsay\t-\t-\tskipped\nask\tA|B\nanswer\t0\nwalk\t0\t60\t0\nwait\t500\nanswer\t1\nend\nscene\t1\tb\nlabel\there\nsay\t-\t-\tseen\n";
    let mut director = story(text);
    director.author.skip_to = Some("here".to_string());
    let mut stage = Stage::default();
    let mut calls = Vec::new();
    for _ in 0..10 {
        calls.extend(director.tick(&mut stage, 1.0, Vec2::ZERO));
    }
    let said: Vec<&Call> = calls.iter().filter(|call| matches!(call, Call::Say { .. })).collect();
    assert!(matches!(&said[..], [Call::Say { text, .. }] if text == "seen"), "{said:?}");
    assert_eq!(director.author.skip_to, None);
    assert_eq!(stage.cast[&Who::Character(0)].position, Vec2::new(60.0, 0.0), "where it was going");
}

#[test]
fn a_loop_goes_round_while_it_holds() {
    let text = "event\t0\tE\nscene\t1\ta\nset\tn\t0\nwhile\tn\t<\t3\nadd\tn\nset\tm\t0\nwhile\tm\t<\t2\nadd\tm\nadd\tinner\nendwhile\nendwhile\nsay\t-\t-\t{n} {inner}\n";
    let mut director = story(text);
    assert_eq!(said(&mut director), ["3 6"]);
}

#[test]
fn beats_come_again_and_again_and_a_scene_waits_for_numbers() {
    let text = "event\t0\tE\nscene\t1\ta\nevery\t2\tadd\tticks\nwaituntil\tticks\t>=\t3\nevery\toff\nsay\t-\t-\t{ticks}\n";
    let mut director = story(text);
    let mut stage = Stage::default();
    let mut said = None;
    for _ in 0..20 {
        for call in director.tick(&mut stage, 1.0, Vec2::ZERO) {
            if let Call::Say { text, .. } = call {
                said = Some(text);
            }
        }
    }
    assert_eq!(said.as_deref(), Some("3"));
    assert_eq!(director.vars["ticks"], 3.0, "no more once off");
}

#[test]
fn numbers_drawn_between_two_and_someone_put_beside() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nput\t0\t100\t100\tdown\nputnear\t10\t0\tleft\nrandom\tn\t5\t7\nsay\t-\t-\t{n}\n");
    director.author.trace = Some(Vec::new());
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.cast[&Who::Character(10)].position, Vec2::new(74.0, 100.0));
    assert_eq!(stage.cast[&Who::Character(10)].direction, 3, "turned to the other");
    assert!(calls.iter().any(|call| matches!(call, Call::Show(Beat::Body(BodyBeat::Put { who: Who::Character(10), .. })))));
    assert!((5.0..=7.0).contains(&director.vars["n"]));
    assert_eq!(director.author.trace.as_ref().map(Vec::len), Some(4), "{:?}", director.author.trace);
}

#[test]
fn speakers_keep_their_mood_and_voice() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nmood\t10\tjoyous\nvoice\t10\t777\nsay\t10\t-\tHi\nsay\t10\t5\tOh\n");
    let mut stage = Stage::default();
    let mut lines = Vec::new();
    for _ in 0..4 {
        for call in director.tick(&mut stage, 1.0, Vec2::ZERO) {
            if let Call::Say { face, voice, .. } = call {
                lines.push((face, voice));
            }
        }
        director.read();
    }
    assert_eq!(lines, [(Some(10), Some(777)), (Some(5), Some(777))]);
}

#[test]
fn ways_are_chosen_by_elif_and_and_or() {
    let text = "event\t0\tE\nscene\t1\ta\nset\ta\t2\nset\tb\t5\nif\ta\t==\t1\nsay\t-\t-\tone\nelif\ta\t==\t2\tand\tb\t<\t3\nsay\t-\t-\ttwo-small\nelif\ta\t==\t9\tor\tb\t>=\t5\nsay\t-\t-\tbig\nelse\nsay\t-\t-\tnone\nendif\nsay\t-\t-\tafter\n";
    let mut director = story(text);
    assert_eq!(said(&mut director), ["big", "after"]);
}

#[test]
fn lines_turn_speakers_to_each_other() {
    let text = "event\t0\tE\nscene\t1\ta\nput\t0\t0\t0\tdown\nput\t10\t40\t0\tdown\nautoface\nsay\t0\t-\tHi\nsay\t10\tsurprised\tOh!\n";
    let mut director = story(text);
    let mut stage = Stage::default();
    for _ in 0..3 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
        director.read();
    }
    assert_eq!(stage.cast[&Who::Character(0)].direction, 3, "turned to the other");
    assert_eq!(stage.cast[&Who::Character(10)].direction, 7);
}

#[test]
fn the_view_goes_to_who_speaks_only_if_it_does_not_show_them() {
    let text = "event\t0\tE\nscene\t1\ta\nput\t0\t0\t0\tdown\nput\t10\t400\t0\tdown\nkeepspeaker\nsay\t0\t-\tIci.\nsay\t10\t-\tLoin.\n";
    let mut director = story(text);
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.view.pan, None, "seen already");
    director.read();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.view.pan.map(|pan| pan.to), Some(Cam::Who(Who::Character(10))));
}

/// One low on the screen, under where the box shows: the box goes up for
/// that line, the view stays; down again for one who stands higher.
#[test]
fn the_box_goes_up_for_who_speaks_low_on_the_screen() {
    let text = "event\t0\tE\nscene\t1\ta\nput\t0\t0\t70\tdown\nput\t10\t0\t0\tdown\nkeepspeaker\nsay\t0\t-\tDown here.\nsay\t10\t-\tIn the middle.\n";
    let mut director = story(text);
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(stage.words.box_top, "the box at the top for who is under it");
    assert_eq!(stage.view.pan, None, "the view stays");
    director.read();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(!stage.words.box_top, "back at the foot");
}

/// A box the film keeps at the foot stays there for one low on the screen;
/// `boxside auto` has it go up again.
#[test]
fn a_box_kept_at_the_foot_stays_there_and_auto_follows_again() {
    let text = "event	0	E
scene	1	a
put	0	0	70	down
boxside	bottom
say	0	-	Down here.
boxside	auto
say	0	-	Still here.
";
    let mut director = story(text);
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(!stage.words.box_top, "kept at the foot");
    director.read();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(stage.words.box_top, "up again for who is under it");
}

#[test]
fn listeners_look_at_who_speaks_and_answer_in_turn() {
    let text = "event\t0\tE\nscene\t1\ta\nput\t0\t0\t0\tdown\nput\t10\t40\t0\tdown\nput\t119\t-40\t0\tdown\naudience\t10\t119\nreact\nsay\t0\tsurprised\tOh!\naudience\toff\nsay\t10\t-\tHein?\n";
    let mut director = story(text);
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.cast[&Who::Character(10)].direction, 7, "turned to the speaker");
    assert_eq!(stage.cast[&Who::Character(119)].direction, 3);
    // (One in two answers.)
    let answered = [10, 119].iter().filter(|one| stage.emotes.effects.contains_key(&Who::Character(**one))).count();
    assert_eq!(answered, 1);
    assert!(!stage.emotes.effects.contains_key(&Who::Character(0)), "not the speaker");
    director.read();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(stage.cast[&Who::Character(0)].direction, 1, "nobody listens any more");
}

#[test]
fn scenes_are_gone_to_by_name() {
    let text = "event\t0\tE\nscene\t1\tstart\nset\tway\t1\nif\tway\t==\t1\ngotoscene\tforest\nendif\nsay\t-\t-\tbeach\nend\nscene\t1\tbeach\nsay\t-\t-\tbeach\nend\nscene\t1\tforest\nsay\t-\t-\tforest\n";
    let mut director = story(text);
    assert_eq!(said(&mut director), ["forest"]);
}

#[test]
fn an_event_put_before_plays_first() {
    let mut director = story("event\t3\tA\nsay\t-\t-\ta\nevent\t4\tB\nsay\t-\t-\tb\n");
    director.insert(StoryEvent {
        scenes: vec![Scene { beats: vec![Beat::Course(Course::Host(vec!["ground".to_string(), "1".to_string()]))], ..Scene::default() }],
        moment: (2, 0),
        ..StoryEvent::default()
    });
    let mut stage = Stage::default();
    let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(hosted(&calls), ["ground 1"]);
    assert_eq!(director.moment(), (0, 0));
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert_eq!(director.at, Some(Mark { event: 1, scene: 0, beat: 0 }));
    assert_eq!(director.moment(), (2, 0));
}

#[test]
fn names_are_put_in_words() {
    let who = |who: Who| if who == Who::Monster(25) { "Pikachu".to_string() } else { String::new() };
    let item = |item: u16| if item == 109 { "Apple".to_string() } else { String::new() };
    assert_eq!(names_in("{name:m25} has an {item:i109.2}, {item:109}!", &SAMPLE, who, item), "Pikachu has an Apple, Apple!");
    assert_eq!(names_in("{name:m25", &SAMPLE, who, item), "{name:m25");
}

#[test]
fn a_timed_line_goes_by_itself_from_the_frame_it_shows() {
    let mut director = story("event\t0\tE\t1\t1\nscene\t1\ta\nput\t10\t200\t100\tdown\nsay\t10\t-\tA\ntimed\t30\nsaid\nset\tdone\t1\n");
    let mut stage = Stage::default();
    for _ in 0..10 {
        let calls = director.tick(&mut stage, 1.0, Vec2::ZERO);
        if calls.iter().any(|call| matches!(call, Call::Say { .. })) {
            assert!(director.line_timed(), "the box must know at once it goes by itself");
            return;
        }
    }
    panic!("the line was never said");
}

#[test]
fn a_crowd_with_no_room_for_all_still_has_a_point_for_each() {
    // (A place of three tiles by three: no room for twelve; each of them has
    // its point, where they gather for those left over.)
    let crowd = ["-"; 12].join("\t");
    let mut director = story(&format!("event\t0\tE\nscene\t1\ta\nground\t12\t12\t000/000/000\ngathers\tg\tx:12:12\t16\t16\t{crowd}\nwait\t5\n"));
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    assert!((0..12).all(|place| director.vars.contains_key(&format!("g{place}_x")) && director.vars.contains_key(&format!("g{place}_y"))));
}

#[test]
fn an_event_put_before_moves_on_what_is_kept_of_the_others() {
    // (A passage not yet returned from, then an event played now: its way
    // back is to the same beat of the same event, one further now.)
    let mut director = story("event\t0\tE\nscene\t1\ta\nwait\t5\nwait\t600\n");
    let mut stage = Stage::default();
    director.tick(&mut stage, 1.0, Vec2::ZERO);
    director.schedule.returns.push(Mark { event: 0, scene: 0, beat: 1 });
    let kept = director.schedule.returns.clone();
    assert_eq!(kept.len(), 1);
    director.interlude(
        read_events(crate::language::read_scene("event\t1\tI\nscene\t1\tb\nwait\t5\n", None, &SAMPLE).lines.into_iter().map(|(_, line)| line), &SAMPLE)
            .remove(0),
    );
    assert_eq!(director.schedule.returns, vec![Mark { event: kept[0].event + 1, ..kept[0] }]);
}

#[test]
fn the_music_s_clock_counts_from_where_the_song_starts_at_its_speed() {
    let mut clock = MusicClock::default();
    assert_eq!(clock.reading(), None, "no music yet: what waits for it waits");
    clock.start(true, 8.0);
    clock.pass(60.0);
    assert_eq!(clock.reading(), Some(540.0));
    clock.speed = 1.5;
    clock.pass(60.0);
    assert_eq!(clock.reading(), Some(630.0));
    // (A new song: from its start, at the speed the music has.)
    clock.start(true, 0.0);
    clock.pass(40.0);
    assert_eq!(clock.reading(), Some(60.0));
    clock.start(false, 0.0);
    assert!(clock.reading().unwrap() > 1e6, "stopped: what waits for it waits no more");
}

#[test]
fn a_sound_is_waited_for_as_long_as_it_lasts() {
    let text = "event\t0\tE\t1\t1\nscene\t1\ta\nmusicfrom\t5\t2s\nwaitsound\t7\nset\tx\t1\nwaituntil\tmusic_clock\t>=\t180\nset\ty\t1\n";
    let (mut director, mut stage) = (story(text), Stage::default());
    director.hear_sounds(|of| (*of == SoundOf::Game(SoundId(7))).then_some(30.0));
    let set = |name: &str, director: &mut Director, stage: &mut Stage| {
        (0..200).position(|_| {
            director.tick(stage, 1.0, Vec2::ZERO);
            director.numbers().contains_key(name)
        })
    };
    assert_eq!(set("x", &mut director, &mut stage), Some(30));
    // (Two seconds into the song as it started, a second more.)
    assert!(set("y", &mut director, &mut stage).is_some_and(|frame| (28..=32).contains(&frame)));
    assert!(director.unknown_sounds().is_empty());
}

#[test]
fn a_sound_from_someone_is_heard_from_his_side_softer_far() {
    let text = "event	0	E	1	1
scene	1	a
put	0	-100	0	down
playsound	own:clac	from	0	volume	0.5
t	5
";
    let (mut director, mut stage) = (story(text), Stage::default());
    let calls: Vec<Call> = (0..3).flat_map(|_| director.tick(&mut stage, 1.0, Vec2::ZERO)).collect();
    let played = calls.iter().find_map(|call| match call {
        Call::Show(Beat::Sound(SoundBeat::Play(played))) => Some(played.clone()),
        _ => None,
    });
    let played = played.expect("heard");
    assert_eq!(played.of, SoundOf::Own("clac".to_string()));
    let Side::At(pan) = played.pan else { panic!("{:?}", played.pan) };
    assert!(pan < -0.5, "{pan}");
    assert!(played.volume < 0.5 && played.volume > 0.1, "{}", played.volume);
    assert_eq!(pan_of(0.0), 0.0);
    assert_eq!(pan_of(1000.0), -pan_of(-1000.0));
    assert!(pan_of(1000.0) < 1.0);
}

#[test]
fn a_sound_waited_for_is_its_length_at_its_pitch_and_an_unknown_one_is_told() {
    let text = "event	0	E	1	1
scene	1	a
waitsound	own:clac	pitch	12
set	x	1
waitsound	move:33
set	y	1
";
    let (mut director, mut stage) = (story(text), Stage::default());
    director.hear_sounds(|of| matches!(of, SoundOf::Own(name) if name == "clac").then_some(40.0));
    let set = |name: &str, director: &mut Director, stage: &mut Stage| {
        (0..200).position(|_| {
            director.tick(stage, 1.0, Vec2::ZERO);
            director.numbers().contains_key(name)
        })
    };
    // (An octave higher: twice as fast.)
    assert_eq!(set("x", &mut director, &mut stage), Some(20));
    assert!(set("y", &mut director, &mut stage).is_some());
    assert_eq!(director.unknown_sounds(), &[SoundOf::Move(MoveId(33))]);
}

#[test]
fn a_film_s_scenes_open_with_no_collisions_the_game_s_with_them() {
    let text = "event\t0\tE\nscene\t1\ta\nwait\t5\n";
    let lines = || crate::language::read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line);
    let mut stage = Stage::default();
    Director::new(read_events(lines(), &SAMPLE), 0, &SAMPLE).tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(!stage.no_collisions, "the game's");
    let mut events = read_events(lines(), &SAMPLE);
    events[0].film = true;
    Director::new(events, 0, &SAMPLE).tick(&mut stage, 1.0, Vec2::ZERO);
    assert!(stage.no_collisions, "a film's");
}

#[test]
fn keys_timed_by_the_music_or_a_mark_are_so_many_frames_from_now() {
    let mut director = story("event\t0\tE\nscene\t1\ta\nwait\t60\n");
    director.vars.insert(MUSIC_CLOCK.to_string(), 300.0);
    director.vars.insert(names::CLOCK.to_string(), 500.0);
    director.vars.insert("_mark_blow".to_string(), 450.0);
    let key = |at| Key { at, value: crate::props::Given::Is(crate::props::Value::Number(1.0)), ease: crate::stage::Ease::Linear };
    let beat = Beat::Node(NodeBeat::Animate {
        node: NodeRef::Who(Who::Monster(25)),
        prop: crate::props::PropId::X,
        keys: vec![
            key(KeyTime::After(Frames(10.0))),
            key(KeyTime::Music(Frames(720.0))),
            key(KeyTime::Mark { name: "blow".to_string(), plus: Frames(120.0) }),
            key(KeyTime::Music(Frames(100.0))),
            key(KeyTime::Mark { name: "jamais".to_string(), plus: Frames(5.0) }),
        ],
        mode: TrackMode::Once,
        relative: false,
        exact: false,
    });
    let Beat::Node(NodeBeat::Animate { keys, .. }) = director.keys_timed(beat) else { panic!() };
    let at: Vec<KeyTime> = keys.into_iter().map(|key| key.at).collect();
    assert_eq!(at, [10.0, 420.0, 70.0, 0.0, 0.0].map(|frames| KeyTime::After(Frames(frames))));
}

/// The film's lines told as they start and end, each on its course: on
/// the scene's at the next line's start, in a passage at its end, one by
/// its frames when they are over.
#[test]
fn the_film_s_lines_start_and_end_on_their_course() {
    let text = "event\t0\tE\nscene\t1\ta\nfilmline\t1\nwait\t10\nfilmline\t2\tfor\t5\nfilmline\t3\nparallel\tfil1\nwait\t20\nfilmline\t5\nend\nlabel\tfil1\nfilmline\t4\nwait\t8\nreturn\n";
    let (mut director, mut stage) = (story(text), Stage::default());
    director.author.spans = Some(Vec::new());
    let mut told: Vec<(usize, i32, Option<String>, bool)> = Vec::new();
    for frame in 0..40 {
        director.tick(&mut stage, 1.0, Vec2::ZERO);
        for mark in director.author.spans.as_mut().map(std::mem::take).unwrap_or_default() {
            told.push((frame, mark.line, mark.thread, mark.starts));
        }
    }
    let when = |line: i32, starts: bool| told.iter().find(|(_, at, _, start)| *at == line && *start == starts).map(|(frame, ..)| *frame);
    assert_eq!(when(1, true), Some(0));
    assert_eq!(when(1, false), when(2, true), "line 1 ends as 2 starts: {told:?}");
    assert_eq!(when(2, true), Some(10));
    assert_eq!(when(2, false).map(|end| end - 10), Some(5), "ended by its frames");
    assert_eq!(when(3, false), when(5, true), "the course's line ends at the next");
    let passage = told.iter().find(|(_, line, ..)| *line == 4).map(|(_, _, thread, _)| thread.clone());
    assert_eq!(passage, Some(Some("fil1".to_string())));
    assert_eq!(when(4, false).zip(when(4, true)).map(|(end, start)| end - start), Some(8), "its passage over: {told:?}");
    assert_eq!(director.numbers()[LINE], 5.0);
}

#[test]
fn a_way_out_reaches_its_circle_or_its_rectangle() {
    use super::Reach;
    let circle = Reach::Circle(10.0);
    assert!(circle.holds(Vec2::new(6.0, 6.0)) && !circle.holds(Vec2::new(8.0, 8.0)));
    // (A long low zone: its corners in, past its height out, though a circle as wide would take it.)
    let zone = Reach::Rect(Vec2::new(32.0, 8.0));
    assert!(zone.holds(Vec2::new(30.0, 7.0)) && !zone.holds(Vec2::new(0.0, 12.0)) && !zone.holds(Vec2::new(33.0, 0.0)));
    assert_eq!((circle.half(), zone.half()), (Vec2::splat(10.0), Vec2::new(32.0, 8.0)));
}
