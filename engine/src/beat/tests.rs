use super::*;
use crate::vocabulary::SAMPLE;

#[test]
fn walls_mirrors_sounds_and_holds_are_read() {
    let line = |words: &[&str]| Beat::parse(&words.iter().map(|word| word.to_string()).collect::<Vec<String>>(), &SAMPLE).ok();
    assert_eq!(
        line(&["wall", "door", "1", "2", "3", "4"]),
        Some(Beat::Play(PlayBeat::NamedWall { name: "door".to_string(), wall: Some([1.0, 2.0, 3.0, 4.0].into()) }))
    );
    assert_eq!(line(&["wall", "door", "off"]), Some(Beat::Play(PlayBeat::NamedWall { name: "door".to_string(), wall: None })));
    assert_eq!(line(&["wall", "1", "2", "3", "4"]), Some(Beat::Play(PlayBeat::Wall(Some([1.0, 2.0, 3.0, 4.0].into())))));
    assert_eq!(line(&["mirror", "m25", "0"]), Some(Beat::Motion(MotionBeat::Mirror { who: Who::Monster(25), model: Some(Who::Character(0)) })));
    assert_eq!(line(&["sound", "8962", "m94"]), Some(Beat::Sound(SoundBeat::SoundFrom { id: SoundId(8962), from: Who::Monster(94) })));
    assert_eq!(
        line(&["hold", "i109", "m25"]),
        Some(Beat::Body(BodyBeat::Carry { what: Who::Item { item: 109, copy: 0 }, by: Some(Who::Monster(25)), hands: true }))
    );
    assert_eq!(line(&["hold", "i109", "off"]), Some(Beat::Body(BodyBeat::Carry { what: Who::Item { item: 109, copy: 0 }, by: None, hands: true })));
    assert_eq!(
        line(&["popup", "m25", "red", "-12"]),
        Some(Beat::Body(BodyBeat::Popup { who: Who::Monster(25), text: "-12".to_string(), colour: [255, 40, 40] }))
    );
    assert_eq!(line(&["weather", "hail"]), Some(Beat::Screen(ScreenBeat::Weather(Weather::Hail))));
    assert_eq!(line(&["endask"]), Some(Beat::Course(Course::Rejoin)));
}

#[test]
fn a_fishing_line() {
    let line = |text: &str| Beat::parse(&text.split(' ').map(str::to_string).collect::<Vec<String>>(), &SAMPLE).ok();
    let (a, thing) = (Who::Monster(25), Who::Drawn { picture: 3, copy: 0 });
    assert_eq!(line("fishline m25 d3"), Some(Beat::Body(BodyBeat::FishLine { who: a, to: Some(thing), at: None, cast: true })));
    assert_eq!(
        line("fishline m25 d3 300 100"),
        Some(Beat::Body(BodyBeat::FishLine { who: a, to: Some(thing), at: Some(Vec2::new(300.0, 100.0)), cast: true }))
    );
    assert_eq!(line("fishline m25 off"), Some(Beat::Body(BodyBeat::FishLine { who: a, to: None, at: None, cast: true })));
    assert_eq!(line("fishline m25 d3 holds"), Some(Beat::Body(BodyBeat::FishLine { who: a, to: Some(thing), at: None, cast: false })));
    assert_eq!(line("minimap m25 m393"), Some(Beat::Screen(ScreenBeat::Minimap { team: Some(vec![a, Who::Monster(393)]), discover: false })));
    assert_eq!(line("minimap discover m25"), Some(Beat::Screen(ScreenBeat::Minimap { team: Some(vec![a]), discover: true })));
    assert_eq!(line("minimap off"), Some(Beat::Screen(ScreenBeat::Minimap { team: None, discover: false })));
    let drawn = |ticks: Vec<u32>| Some(Beat::Body(BodyBeat::Drawing { who: thing, file: "x.3.png".to_string(), cells: 3, ticks }));
    assert_eq!(line("drawing d3 x.3.png 3"), drawn(Vec::new()));
    assert_eq!(line("drawing d3 x.3.png 3 40,2,3"), drawn(vec![40, 2, 3]));
    assert_eq!(line("drawing d3 x.3.png 3 40,0,3"), None);
}

#[test]
fn the_box_goes_up_and_down() {
    let line = |text: &str| Beat::parse(&text.split(' ').map(str::to_string).collect::<Vec<String>>(), &SAMPLE).ok();
    assert_eq!(line("boxside top"), Some(Beat::Speech(SpeechBeat::BoxSide { top: true })));
    assert_eq!(line("boxside bottom"), Some(Beat::Speech(SpeechBeat::BoxSide { top: false })));
    assert_eq!(line("boxside middle"), None);
    // (A frame never closer than told.)
    let (a, b) = (Who::Monster(25), Who::Monster(4));
    assert_eq!(line("frame 40 m25 m4 max:0.5"), Some(Beat::View(ViewBeat::Frame { who: vec![a, b], at: Vec::new(), frames: Frames(40.0), most: Some(0.5) })));
    assert_eq!(line("frame 40 m25 m4"), Some(Beat::View(ViewBeat::Frame { who: vec![a, b], at: Vec::new(), frames: Frames(40.0), most: None })));
}

#[test]
fn sounds_and_effects_in_step() {
    let line = |text: &str| Beat::parse(&text.split(' ').map(str::to_string).collect::<Vec<String>>(), &SAMPLE).ok();
    let a = Who::Monster(25);
    assert_eq!(line("movesound 44"), Some(Beat::Sound(SoundBeat::MoveSound(MoveId(44)))));
    assert_eq!(line("onhit m25 movesound 44"), Some(Beat::Sound(SoundBeat::OnHit { who: a, sound: Box::new(SoundBeat::MoveSound(MoveId(44))) })));
    assert_eq!(line("volume 0.5"), Some(Beat::Sound(SoundBeat::Volume(0.5))));
    assert_eq!(line("musicfrom 3 12s"), Some(Beat::Sound(SoundBeat::MusicFrom { song: Some(SongId(3)), seconds: 12.0 })));
    assert_eq!(line("moveeffect m25 44"), Some(Beat::Body(BodyBeat::MoveEffect { who: a, id: MoveId(44) })));
    assert_eq!(
        line("moveshot m25 0 44 40"),
        Some(Beat::Body(BodyBeat::MoveShot { from: a, to: Who::Character(0), id: MoveId(44), frames: Some(Frames(40.0)) }))
    );
    assert_eq!(
        line("emit embers m25 4 red 3s"),
        Some(Beat::Screen(ScreenBeat::Emitter {
            kind: ParticleKind::Embers,
            who: a,
            each_second: 4.0,
            colour: Some([255, 40, 40]),
            frames: Some(Frames(180.0))
        }))
    );
    assert_eq!(line("emit off m25"), Some(Beat::Screen(ScreenBeat::EmitterOff(a))));
}

#[test]
fn bodies_are_told_exactly() {
    let line = |text: &str| Beat::parse(&text.split(' ').map(str::to_string).collect::<Vec<String>>(), &SAMPLE).ok();
    let a = Who::Monster(25);
    assert_eq!(line("pose m25 1 image 3"), Some(Beat::Body(BodyBeat::Anim { who: a, anim: AnimId(1), play: crate::beat::Play::Hold(2) })));
    assert_eq!(line("anim m25 0 2 times"), Some(Beat::Body(BodyBeat::Anim { who: a, anim: AnimId(0), play: crate::beat::Play::Times(2) })));
    assert_eq!(line("textspeed 2"), Some(Beat::Speech(SpeechBeat::TextSpeed(Some(2.0)))));
    assert_eq!(line("still m25"), Some(Beat::Body(BodyBeat::Freeze { who: a, on: true })));
    assert_eq!(line("still on"), Some(Beat::Screen(ScreenBeat::Still(true))));
    assert_eq!(line("unfreeze m25"), Some(Beat::Body(BodyBeat::Freeze { who: a, on: false })));
    assert_eq!(line("height m25 12"), Some(Beat::Body(BodyBeat::Rise { who: a, height: 12.0, frames: Frames(1.0) })));
    assert_eq!(line("tremble m25 2 30"), Some(Beat::Body(BodyBeat::Tremble { who: a, frames: Frames(30.0), strength: 2.0 })));
    assert_eq!(line("tremble m25 30"), Some(Beat::Body(BodyBeat::Tremble { who: a, frames: Frames(30.0), strength: 1.0 })));
    assert_eq!(line("infront m25 0"), Some(Beat::Body(BodyBeat::InFront { who: a, of: Some(Who::Character(0)) })));
    assert_eq!(line("infront m25 off"), Some(Beat::Body(BodyBeat::InFront { who: a, of: None })));
    assert_eq!(line("depth m25 3"), Some(Beat::Body(BodyBeat::Depth { who: a, depth: 3.0 })));
    assert_eq!(line("resume m25"), Some(Beat::Motion(MotionBeat::Resume(a))));
    assert_eq!(
        line("orbit m25 0 2 90"),
        Some(Beat::Motion(MotionBeat::Orbit { who: a, at: glam::Vec2::new(0.0, 2.0).into(), turns: 90.0, frames: Frames(120.0) }))
    );
    assert_eq!(line("orbit m25 m4 2 90"), Some(Beat::Motion(MotionBeat::OrbitAround { who: a, other: Who::Monster(4), turns: 2.0, frames: Frames(90.0) })));
}

#[test]
fn commands_have_one_name_told_in_the_docs() {
    let line = |words: &[&str]| words.iter().map(|word| word.to_string()).collect::<Vec<String>>();
    assert!(matches!(Beat::parse(&line(&["put", "0", "10", "20", "down"]), &SAMPLE).ok(), Some(Beat::Body(BodyBeat::Put { .. }))));
    assert!(matches!(Beat::parse(&line(&["after", "20", "jump", "0"]), &SAMPLE).ok(), Some(Beat::Flow(FlowBeat::After { .. }))));
    let docs = include_str!("../../docs/COMMANDS.md");
    for own in commands() {
        assert!(docs.contains(&format!("`{own}")), "“{own}” is not in COMMANDS.md");
    }
}

/// An answer leads its own way, questions inside questions included.
#[test]
fn animations_by_name() {
    let line: Vec<String> = ["anim", "0", "Sit", "1"].map(str::to_string).to_vec();
    assert_eq!(Beat::parse(&line, &SAMPLE).ok(), Some(Beat::Body(BodyBeat::Anim { who: Who::Character(0), anim: AnimId(23), play: crate::beat::Play::Loop })));
    assert_eq!(SAMPLE.animation("sit"), Some(23));
    assert_eq!(SAMPLE.animation("eventsleep"), Some(13));
    assert_eq!(SAMPLE.animation("dance"), None);
}

/// Each word a kind reads by hand is one of its declared commands.
#[test]
fn the_commands_listed_are_those_read() {
    let sources = [
        (body::WORDS, include_str!("body/read.rs")),
        (motion::WORDS, include_str!("motion.rs")),
        (view::WORDS, include_str!("view.rs")),
        (screen::WORDS, include_str!("screen.rs")),
        (sound::WORDS, include_str!("sound.rs")),
        (speech::WORDS, include_str!("speech.rs")),
        (flow::WORDS, include_str!("flow.rs")),
        (data::WORDS, include_str!("data.rs")),
        (play::WORDS, include_str!("play.rs")),
        (place::WORDS, include_str!("place.rs")),
        (node::WORDS, include_str!("node.rs")),
        (parse::WORDS, include_str!("parse.rs")),
    ];
    let read = |line: &&str| line.strip_prefix("        \"").is_some_and(|rest| rest.contains("=>"));
    for (words, source) in sources {
        // (The arms of its `read`, and of the course's.)
        let from = source.find(" fn read(").or_else(|| source.find("fn course(")).unwrap_or(0);
        let body = &source[from..];
        let body = &body[..body.find("\n}\n").unwrap_or(body.len())];
        for line in body.lines().filter(read) {
            let line = line.trim_start();
            let words_read = &line[..line.find("=>").unwrap_or(line.len())];
            for word in words_read[..words_read.find(" if ").unwrap_or(words_read.len())].split(" | ").map(|word| word.trim().split('"').nth(1).unwrap_or("")) {
                assert!(words.iter().any(|command| command.own == word || command.also.contains(&word)), "{word}");
            }
        }
    }
}

#[test]
fn a_line_that_cannot_be_read_says_why() {
    let parse = |words: &[&str]| Beat::parse(&words.iter().map(|word| word.to_string()).collect::<Vec<String>>(), &SAMPLE);
    assert_eq!(parse(&[]), Err(ParseError::Empty));
    assert_eq!(parse(&["jmup", "0"]), Err(ParseError::Unknown { word: "jmup".to_string(), close: Some("jump") }));
    assert_eq!(parse(&["jump"]), Err(ParseError::Malformed { word: "jump".to_string() }));
    assert_eq!(parse(&["zzzzzzzz"]), Err(ParseError::Unknown { word: "zzzzzzzz".to_string(), close: None }));
}

#[test]
fn a_way_to_face_is_a_name_never_a_number() {
    assert_eq!(Direction::of("upleft"), Some(Direction::UpLeft));
    assert_eq!(Direction::UpLeft.number(), 6);
    assert_eq!(Direction::from_number(6).name(), "upleft");
    assert!(Direction::ALL.iter().all(|way| Direction::of(way.name()) == Some(*way) && Direction::from_number(way.number()) == *way));
    // (A number is someone: `alllook 3` looks at the third.)
    assert_eq!(Direction::of("6"), None);
    assert!(Beat::parse(&["turn".into(), "0".into(), "6".into()], &SAMPLE).is_err());
}

#[test]
fn a_beat_names_each_one_once_with_what_it_does() {
    let beat = |words: &str| Beat::parse(&words.split(' ').map(String::from).collect::<Vec<_>>(), &SAMPLE).expect(words);
    let (a, b) = (Who::Character(1), Who::Character(2));
    // (Put beside someone: the one put comes, the other is there already.)
    let near = beat("putnear 1 2");
    assert_eq!((near.naming(Role::Comes), near.naming(Role::Acts)), (vec![a], vec![b]));
    // (`target` replaced wherever it is named, in a beat played later too.)
    for line in ["depth target 3", "after 10 depth target 3", "face 1 target"] {
        let mut beat = beat(line);
        for who in beat.whos_mut().into_iter().filter(|who| **who == Who::Target) {
            *who = b;
        }
        assert!(beat.naming(Role::Acts).contains(&b), "{line}");
    }
}

#[test]
fn a_glide_is_told_how_to_go() {
    let line = |text: &str| Beat::parse(&text.split(' ').map(str::to_string).collect::<Vec<String>>(), &SAMPLE).ok();
    assert_eq!(line("ease linear"), Some(Beat::View(ViewBeat::Ease(crate::stage::Ease::Linear))));
    assert_eq!(line("ease out"), Some(Beat::View(ViewBeat::Ease(crate::stage::Ease::Out(crate::stage::EaseFamily::Quad)))));
    assert_eq!(line("ease smooth"), Some(Beat::View(ViewBeat::Ease(crate::stage::Ease::Smooth))));
    assert_eq!(line("ease weird"), None);
}

#[test]
fn numbers_found_as_the_scene_plays_are_typed() {
    let line = |words: &[&str]| words.iter().map(|word| word.to_string()).collect::<Vec<String>>();
    let var = |name: &str| Num::Var(name.to_string());
    let put = Beat::Body(BodyBeat::Put { who: Who::Monster(25), at: Point { x: var("_place3_x"), y: Num::Lit(-2.5) }, direction: Direction::Down });
    let written = put.fields(&SAMPLE).expect("a line");
    assert_eq!(written, line(&["put", "m25", "{_place3_x}", "-2.5", "down"]));
    assert_eq!(Beat::played(written, &SAMPLE), Some(put));
    // (A spot's numbers too, and a beat's after a while.)
    let find = Beat::played(line(&["find", "_l", "m25", "w:{a}:{b}", "apart"]), &SAMPLE).expect("a beat");
    assert_eq!(find.vars(), ["a", "b"]);
    let after = Beat::played(line(&["after", "11", "orbit", "m3", "{x}", "{y}", "3", "111"]), &SAMPLE).expect("a beat");
    assert_eq!(after.vars(), ["x", "y"]);
    // (A wait found as it plays; a battle's log, its words filled as shown.)
    assert_eq!(Beat::played(line(&["t", "{steps}"]), &SAMPLE), Some(Beat::Flow(FlowBeat::WaitFound(var("steps")))));
    assert_eq!(Beat::played(line(&["battlelog", "90", "{n} coins!"]), &SAMPLE), None);
    // (Names built as it plays: the line, filled then.)
    let set = line(&["set", "note", "n{i}"]);
    assert_eq!(Beat::played(set.clone(), &SAMPLE), Some(Beat::Flow(FlowBeat::Filled(set))));
    let spotlight = line(&["spotlight", "0", "{_r}"]);
    assert_eq!(Beat::played(spotlight.clone(), &SAMPLE), Some(Beat::Flow(FlowBeat::Filled(spotlight))));
}
