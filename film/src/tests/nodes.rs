//! The words of the nodes: `prop` sets any property of the registry,
//! several on a line; `node` makes a node of the film's own; what is
//! wrong comes with its stable code.

use engine::NodeRef;
use engine::props::{PropId, Value};

use super::*;

const TOWN: &str = "scene: town pikachu eevee\n";

/// The codes of a film's problems.
fn codes(film: &str) -> Vec<&'static str> {
    compile(film).problems.iter().filter_map(|problem| problem.code).collect()
}

#[test]
fn prop_sets_several_properties_on_a_line() {
    let lines = clean(&format!("{TOWN}prop pikachu angle 45 scale 1.5 opacity 50% tint #ff8800 position @eevee+10,-4\n"));
    for wanted in [
        &["prop", "pikachu", "angle", "45"][..],
        &["prop", "pikachu", "scale", "1.5"],
        &["prop", "pikachu", "opacity", "0.5"],
        &["prop", "pikachu", "tint", "#ff8800"],
    ] {
        assert!(lines.contains(&line(wanted)), "{wanted:?} in {lines:?}");
    }
    assert!(lines.iter().any(|fields| starts(fields, &["prop", "pikachu", "position"])), "{lines:?}");
    // (Relative, multiplied, the default.)
    let lines = clean(&format!("{TOWN}prop pikachu angle +30 scale x2 opacity default\n"));
    assert!(lines.contains(&line(&["prop", "pikachu", "angle", "+30"])) && lines.contains(&line(&["prop", "pikachu", "scale", "x2"])));
}

#[test]
fn what_prop_sets_shows_on_the_stage() {
    let (written, stage) = played(&format!("{TOWN}prop pikachu angle 45 scale 2 sink 100 slowmo 0.5\nwait 1s\n"));
    let code = &written.names.iter().find(|(_, name)| name == "pikachu").expect("pikachu").0;
    let who = Who::parse(code).expect("someone");
    let node = NodeRef::Who(who);
    assert_eq!(stage.get(&node, PropId::Angle), Some(Value::Number(45.0)));
    assert_eq!(stage.get(&node, PropId::Scale), Some(Value::Number(2.0)));
    // (No clamp: 100 deep, as written.)
    assert_eq!(stage.get(&node, PropId::Sink), Some(Value::Number(100.0)));
    assert_eq!(stage.cast[&who].own_time(), 0.5);
}

#[test]
fn node_makes_a_node_prop_sets_it() {
    let (_, stage) = played(&format!("{TOWN}node arm group 40 30\nprop arm angle 90 z 3\nnode moon image moon.png 10 20\nprop moon opacity 0.3\nwait 1s\n"));
    let arm = NodeRef::Named("arm".to_string());
    assert_eq!(stage.node(&arm).map(|node| (node.transform.pos, node.transform.angle_deg, node.z)), Some((Vec2::new(40.0, 30.0), 90.0, Some(3.0))));
    let moon = NodeRef::Named("moon".to_string());
    assert_eq!(stage.get(&moon, PropId::Opacity), Some(Value::Number(0.3)));
    assert_eq!(stage.words.pictures.iter().map(|picture| picture.file.as_str()).collect::<Vec<_>>(), ["moon.png"]);
}

#[test]
fn what_is_wrong_has_its_code() {
    assert_eq!(codes(&format!("{TOWN}prop pikachu opcity 1\n")), ["E-PROP-UNKNOWN"]);
    told(&format!("{TOWN}prop pikachu opcity 1\n"), "Near: opacity");
    assert_eq!(codes(&format!("{TOWN}prop pikachu angle red\n")), ["E-VALUE-TYPE"]);
    assert_eq!(codes(&format!("{TOWN}prop pikachu scale -1\n")), ["E-VALUE-IMPOSSIBLE"]);
    assert_eq!(codes(&format!("{TOWN}prop pikachu radius 3\n")), ["E-PROP-KIND"]);
    assert_eq!(codes(&format!("{TOWN}prop pikachu angle\n")), ["E-VALUE-MISSING"]);
    assert_eq!(codes(&format!("{TOWN}prop nobody angle 3\n")), ["E-NODE-UNKNOWN"]);
    assert_eq!(codes(&format!("{TOWN}node arm ring\n")), ["E-NODE-KIND"]);
    assert_eq!(codes(&format!("{TOWN}node pikachu group\n")), ["E-NODE-TAKEN"]);
}

/// Past what is advised: it plays as written, and the checker remarks it.
#[test]
fn past_the_advised_values_a_remark() {
    let written = compile(&format!("{TOWN}prop pikachu scale 20\n"));
    assert!(written.problems.iter().all(|problem| problem.is_remark()), "{:?}", written.problems);
    let checked = engine::Checker::new(&POKEMON).check_script(written.script());
    let remark = checked.iter().find(|problem| problem.code == Some("R-OUT-OF-RANGE")).expect("a remark");
    assert!(remark.is_remark() && remark.says("scale 20"), "{remark}");
}

/// No cap on a prop's copies: the eighth chest is one more.
#[test]
fn a_thing_has_as_many_copies_as_wanted() {
    let mut film = String::from("scene: town pikachu\n");
    for _ in 0..9 {
        film.push_str("object put chest\n");
    }
    let (_, problems) = engine_lines(&film);
    assert!(problems.iter().all(|problem| !problem.contains("at most")), "{problems:?}");
}

/// `attach` hangs a node on another, `detach` lets it go; a loop and a
/// caption are refused with their codes.
#[test]
fn attach_hangs_a_node_on_another() {
    let lines = clean(&format!("{TOWN}node raft group 100 100\npikachu attach raft 0 -8 inherit pos angle\npikachu detach\n"));
    assert!(lines.contains(&line(&["attach", "pikachu", "node:raft", "0", "-8", "inherit", "pos", "angle"])), "{lines:?}");
    assert!(lines.contains(&line(&["detach", "pikachu"])), "{lines:?}");
    let (_, stage) = played(&format!("{TOWN}node raft group 100 100\npikachu attach raft 0 -8\nprop raft x 150\nwait 1s\n"));
    let hung = stage.nodes.iter().find(|(_, kept)| kept.parent.is_some()).expect("hung");
    assert_eq!(hung.1.parent, Some(NodeRef::Named("raft".to_string())));
    assert_eq!(stage.node(hung.0).map(|node| node.transform.pos), Some(Vec2::new(150.0, 92.0)));
    assert_eq!(codes(&format!("{TOWN}node a group\nnode b group\na attach b\nb attach a\n")), ["E-ATTACH-LOOP"]);
    assert_eq!(codes(&format!("{TOWN}node a group\na attach a\n")), ["E-ATTACH-LOOP"]);
    assert_eq!(codes(&format!("{TOWN}attach caption pikachu\n")), ["E-ATTACH-KIND"]);
    assert_eq!(codes(&format!("{TOWN}pikachu attach eevee inherit speed\n")), ["E-VALUE-TYPE"]);
}

/// `attach`'s offset is read by its form: what is not a number is told by
/// its name there, not guessed from the line's words.
#[test]
fn attach_tells_an_offset_that_is_not_a_number() {
    let film = compile(&format!("{TOWN}attach pikachu eevee 0 far\n"));
    let problems: Vec<&str> = film.problems.iter().filter(|problem| !problem.is_remark()).map(|problem| problem.message.as_str()).collect();
    assert!(problems.iter().any(|problem| problem.contains("“far” is not a number (dy)")), "{problems:?}");
}
