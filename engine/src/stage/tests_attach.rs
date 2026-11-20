//! Nodes hung on others (`attache`): chained transforms, what each takes,
//! loops refused, bodies going with what bears them; and the effects of a
//! node, by their properties.

use super::node::{Inherit, NodeEffect};
use super::*;
use crate::props::{Given, PropId, Value};

fn group(stage: &mut Stage, name: &str, at: Vec2) -> NodeRef {
    stage.direct(Beat::Node(NodeBeat::Make { name: name.to_string(), sort: Sort::Group, at: Point::at(at) }), Vec2::ZERO);
    NodeRef::Named(name.to_string())
}

fn set(stage: &mut Stage, node: &NodeRef, prop: PropId, value: Value) {
    assert_eq!(stage.set(node, prop, &Given::Is(value)), Ok(()), "{prop:?}");
}

fn at(stage: &Stage, node: &NodeRef) -> Vec2 {
    stage.node(node).expect("there").transform.pos
}

fn near(a: Vec2, b: Vec2) -> bool {
    a.distance(b) < 1e-3
}

/// A child of a child turns with both: the arm with the body, the hand
/// with the arm; each keeps its own angle, drawn with theirs.
#[test]
fn a_chain_of_nodes_turns_together() {
    let mut stage = Stage::default();
    let body = group(&mut stage, "body", Vec2::new(100.0, 100.0));
    let arm = group(&mut stage, "bras", Vec2::new(110.0, 100.0));
    let hand = group(&mut stage, "main", Vec2::new(120.0, 100.0));
    assert!(stage.attach(&arm, &body, None, Inherit::default()));
    assert!(stage.attach(&hand, &arm, None, Inherit::default()));
    assert_eq!(stage.nodes[&arm].offset, Vec2::new(10.0, 0.0), "where it was");
    assert_eq!((stage.depth_of(&hand), stage.depth_of(&body)), (2, 0));
    // (A quarter turn, clockwise on the screen.)
    set(&mut stage, &body, PropId::Angle, Value::Number(90.0));
    stage.attached_along();
    assert!(near(at(&stage, &arm), Vec2::new(100.0, 110.0)), "{}", at(&stage, &arm));
    assert!(near(at(&stage, &hand), Vec2::new(100.0, 120.0)), "{}", at(&stage, &hand));
    set(&mut stage, &arm, PropId::Angle, Value::Number(-90.0));
    stage.attached_along();
    assert!(near(at(&stage, &hand), Vec2::new(110.0, 110.0)), "{}", at(&stage, &hand));
    assert_eq!(stage.node(&hand).map(|node| node.transform.angle_deg), Some(0.0));
    assert_eq!(stage.get(&arm, PropId::Angle), Some(Value::Number(-90.0)), "its own");
    // (Grown: the chain grows with it.)
    set(&mut stage, &body, PropId::Scale, Value::Number(2.0));
    stage.attached_along();
    assert!(near(at(&stage, &arm), Vec2::new(100.0, 120.0)), "{}", at(&stage, &arm));
    assert_eq!(stage.node(&hand).map(|node| node.transform.scale), Some(Vec2::splat(2.0)));
    // (A loop is refused: the body on its own hand.)
    assert!(!stage.attach(&body, &hand, None, Inherit::default()));
    assert!(!stage.attach(&body, &body, None, Inherit::default()));
    // (Let go: it stays where it is.)
    let there = at(&stage, &hand);
    stage.detach(&hand);
    set(&mut stage, &body, PropId::Angle, Value::Number(0.0));
    stage.attached_along();
    assert_eq!(at(&stage, &hand), there);
    assert_eq!(stage.node(&hand).and_then(|node| node.parent), None);
}

/// A pivot is kept where it hangs: a part turned about its own joint
/// turns there, its parent's turn taking the joint along.
#[test]
fn a_part_hangs_by_its_pivot() {
    let mut stage = Stage::default();
    let body = group(&mut stage, "body", Vec2::new(50.0, 50.0));
    let arm = group(&mut stage, "bras", Vec2::ZERO);
    set(&mut stage, &arm, PropId::Pivot, Value::Point(Vec2::new(0.0, -4.0)));
    assert!(stage.attach(&arm, &body, Some(Vec2::new(10.0, 4.0)), Inherit::default()));
    // (Its joint, 4 over its position, at 10 0 from the body.)
    assert!(near(at(&stage, &arm) + Vec2::new(0.0, -4.0), Vec2::new(60.0, 50.0)));
    set(&mut stage, &body, PropId::Angle, Value::Number(180.0));
    stage.attached_along();
    assert!(near(at(&stage, &arm) + Vec2::new(0.0, -4.0), Vec2::new(40.0, 50.0)), "{}", at(&stage, &arm));
}

/// Taking only its place, it does not turn nor fade with its parent; all
/// of it, it does.
#[test]
fn what_a_node_takes_from_its_parent() {
    let mut stage = Stage::default();
    let raft = group(&mut stage, "radeau", Vec2::new(100.0, 100.0));
    stage.put(Who::Monster(25), Vec2::new(100.0, 80.0), 1);
    let pika = NodeRef::Who(Who::Monster(25));
    assert!(stage.attach(&pika, &raft, None, Inherit::default()));
    set(&mut stage, &raft, PropId::Opacity, Value::Number(0.5));
    set(&mut stage, &raft, PropId::Tint, Value::Colour([255, 0, 0, 255]));
    set(&mut stage, &raft, PropId::Position, Value::Point(Vec2::new(150.0, 100.0)));
    run_frames(&mut stage, 1);
    assert_eq!(stage.cast[&Who::Monster(25)].position, Vec2::new(150.0, 80.0), "goes with it");
    let shown = stage.node(&pika).expect("there");
    assert_eq!((shown.looks.opacity, shown.looks.tint), (0.5, [255, 0, 0, 255]));
    // (Put elsewhere while hung: it hangs there now.)
    set(&mut stage, &pika, PropId::Position, Value::Point(Vec2::new(160.0, 90.0)));
    assert_eq!(stage.nodes[&pika].offset, Vec2::new(10.0, -10.0));
    let moon = group(&mut stage, "moon", Vec2::new(0.0, 0.0));
    assert!(stage.attach(&moon, &raft, Some(Vec2::ZERO), Inherit { pos: true, ..Inherit::NONE }));
    set(&mut stage, &raft, PropId::Angle, Value::Number(45.0));
    let alone = stage.node(&moon).expect("there");
    assert_eq!((alone.transform.angle_deg, alone.looks.opacity, alone.transform.pos), (0.0, 1.0, Vec2::new(150.0, 100.0)));
    // (Hung on someone, drawn just in front of them, with no `z` of its own.)
    stage.put(Who::Monster(7), Vec2::new(100.0, 120.0), 1);
    stage.put(Who::Monster(4), Vec2::new(100.0, 60.0), 1);
    assert!(stage.attach(&NodeRef::Who(Who::Monster(4)), &NodeRef::Who(Who::Monster(7)), None, Inherit::default()));
    let order: Vec<Who> = stage.in_drawing_order(|_| false).into_iter().map(|(who, _)| who).collect();
    let rank = |who: Who| order.iter().position(|one| *one == who).expect("drawn");
    assert_eq!(rank(Who::Monster(4)), rank(Who::Monster(7)) + 1, "{order:?}");
    // (One carried is let go of, to hang.)
    stage.with(Who::Monster(25), |actor| actor.habits.carried_by = Some(Who::Monster(7)));
    assert!(stage.attach(&pika, &raft, None, Inherit::default()));
    assert!(!stage.cast[&Who::Monster(25)].carried());
}

fn run_frames(stage: &mut Stage, frames: usize) {
    for _ in 0..frames {
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    }
}

/// The effects are properties: set, read back, listed as the node has
/// them; none until a colour or an amount turns one on.
#[test]
fn effects_are_properties_of_a_drawn_node() {
    let mut stage = Stage::default();
    stage.put(Who::Monster(25), Vec2::new(100.0, 80.0), 1);
    let pika = NodeRef::Who(Who::Monster(25));
    assert!(stage.node(&pika).is_some_and(|node| node.looks.effects.list().is_empty() && node.looks.is_plain()));
    set(&mut stage, &pika, PropId::Outline, Value::Colour([0, 0, 0, 255]));
    set(&mut stage, &pika, PropId::OutlineSize, Value::Number(2.0));
    set(&mut stage, &pika, PropId::Dissolve, Value::Number(0.4));
    set(&mut stage, &pika, PropId::Seed, Value::Number(42.0));
    set(&mut stage, &pika, PropId::Wave, Value::Number(3.0));
    set(&mut stage, &pika, PropId::SwapFrom, Value::Colour([255, 255, 0, 255]));
    set(&mut stage, &pika, PropId::SwapTo, Value::Colour([0, 128, 255, 255]));
    set(&mut stage, &pika, PropId::Flash, Value::Colour([255, 255, 255, 255]));
    set(&mut stage, &pika, PropId::FlashAmount, Value::Number(0.5));
    set(&mut stage, &pika, PropId::DropShadow, Value::Colour([0, 0, 0, 128]));
    set(&mut stage, &pika, PropId::ShadowOffset, Value::Point(Vec2::new(3.0, 1.0)));
    set(&mut stage, &pika, PropId::Glow, Value::Colour([255, 200, 0, 255]));
    let node = stage.node(&pika).expect("there");
    assert!(!node.looks.is_plain());
    let names: Vec<&str> = node.looks.effects.list().iter().map(NodeEffect::name).collect();
    assert_eq!(names, ["drop_shadow", "glow", "outline", "dissolve", "wave", "palette", "flash"]);
    assert_eq!(stage.get(&pika, PropId::OutlineSize), Some(Value::Number(2.0)));
    assert_eq!(stage.get(&pika, PropId::ShadowOffset), Some(Value::Point(Vec2::new(3.0, 1.0))));
    assert_eq!(stage.get(&pika, PropId::WaveLength), Some(Value::Number(16.0)), "its default");
    // (Off again: no colour.)
    set(&mut stage, &pika, PropId::Outline, Value::Nothing);
    assert_eq!(stage.get(&pika, PropId::Outline), Some(Value::Nothing));
    // (Not a halo's: nor any kind that is not drawn from a picture.)
    stage.light.halos.push(Halo { on: Cam::Who(Who::Monster(25)), radius: 40.0, colour: [255, 255, 255] });
    assert!(stage.set(&NodeRef::Halo(0), PropId::Outline, &Given::Is(Value::Colour([0, 0, 0, 255]))).is_err());
    assert!(stage.set(&pika, PropId::Dissolve, &Given::Is(Value::Number(-1.0))).is_err(), "impossible");
}
