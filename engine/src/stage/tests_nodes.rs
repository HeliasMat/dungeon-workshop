//! The nodes: what a body, a thing, a picture, a halo, an emitter and a
//! node of a film's own are as nodes, their properties set by the
//! registry's names, and the stage as the game has it.

use super::node::{Inherit, Transform};
use super::*;
use crate::props::{self, Given, NodeKind, PropId, Value};

fn pika() -> NodeRef {
    NodeRef::Who(Who::Monster(25))
}

fn stage_with_pika() -> Stage {
    let mut stage = Stage::default();
    stage.put(Who::Monster(25), Vec2::new(100.0, 80.0), 1);
    stage
}

fn set(stage: &mut Stage, node: &NodeRef, prop: PropId, value: Value) {
    assert_eq!(stage.set(node, prop, &Given::Is(value)), Ok(()), "{prop:?}");
}

/// A body just put is, as a node, as it always showed: its place, its
/// size, all of it, its colours, no turn, no slant, `z` as today's order;
/// each property of its kind at the registry's default.
#[test]
fn a_body_as_a_node_is_as_it_was() {
    let stage = stage_with_pika();
    let node = stage.node(&pika()).expect("there");
    assert_eq!((node.kind, node.name.as_str()), (NodeKind::Body, "m25"));
    assert_eq!(node.transform, Transform { pos: Vec2::new(100.0, 80.0), ..Transform::default() });
    assert_eq!(node.looks, node::Looks::default());
    assert_eq!((node.z, node.visible, node.parent.clone(), node.inherit), (None, true, None, Inherit::default()));
    for prop in props::of_kind(NodeKind::Body) {
        if matches!(prop.id, PropId::Position | PropId::X | PropId::Y) {
            continue;
        }
        assert_eq!(stage.get(&pika(), prop.id), Some(prop.default), "{}", prop.name);
    }
    assert_eq!(stage.get(&pika(), PropId::Position), Some(Value::Point(Vec2::new(100.0, 80.0))));
    assert_eq!(stage.get(&pika(), PropId::Radius), None, "not a body's");
}

#[test]
fn a_body_s_own_properties_are_set_on_it_with_no_clamp() {
    let mut stage = stage_with_pika();
    let who = Who::Monster(25);
    set(&mut stage, &pika(), PropId::Opacity, Value::Number(0.5));
    set(&mut stage, &pika(), PropId::Scale, Value::Number(20.0));
    set(&mut stage, &pika(), PropId::Sink, Value::Number(100.0));
    set(&mut stage, &pika(), PropId::Slowmo, Value::Number(0.01));
    set(&mut stage, &pika(), PropId::AnimSpeed, Value::Number(12.0));
    set(&mut stage, &pika(), PropId::Height, Value::Number(30.0));
    set(&mut stage, &pika(), PropId::Facing, Value::Choice("left"));
    set(&mut stage, &pika(), PropId::Tint, Value::Colour([255, 0, 0, 128]));
    set(&mut stage, &pika(), PropId::Shadow, Value::Bool(false));
    let actor = &stage.cast[&who];
    assert_eq!((actor.shown_opacity(), actor.scale(), actor.sunk, actor.own_time()), (0.5, 20.0, 100.0, 0.01));
    assert_eq!((actor.picture_speed(), actor.shown_height(), actor.direction), (12.0 * 0.01, 30.0, Direction::Left.number()));
    assert_eq!((actor.colour, actor.shadowless), ([255.0, 0.0, 0.0], true));
    assert_eq!(stage.get(&pika(), PropId::Tint), Some(Value::Colour([255, 0, 0, 128])));
    set(&mut stage, &pika(), PropId::Visible, Value::Bool(false));
    assert!(!stage.cast[&who].seen());
    set(&mut stage, &pika(), PropId::Visible, Value::Bool(true));
    assert!(stage.cast[&who].seen());
}

#[test]
fn what_a_body_has_not_the_node_keeps() {
    let mut stage = stage_with_pika();
    set(&mut stage, &pika(), PropId::Angle, Value::Number(45.0));
    set(&mut stage, &pika(), PropId::SkewX, Value::Number(10.0));
    set(&mut stage, &pika(), PropId::Pivot, Value::Point(Vec2::new(0.0, -12.0)));
    set(&mut stage, &pika(), PropId::Blend, Value::Choice("add"));
    set(&mut stage, &pika(), PropId::Z, Value::Number(5.0));
    set(&mut stage, &pika(), PropId::Fill, Value::Colour([255, 255, 255, 255]));
    let node = stage.node(&pika()).expect("there");
    assert_eq!((node.transform.angle_deg, node.transform.skew, node.transform.pivot), (45.0, Vec2::new(10.0, 0.0), Vec2::new(0.0, -12.0)));
    assert_eq!((node.looks.blend, node.z, node.looks.fill), (props::Blend::Add, Some(5.0), Some([255; 4])));
    // (Its size along one way: over its own, which it keeps.)
    set(&mut stage, &pika(), PropId::Scale, Value::Number(2.0));
    set(&mut stage, &pika(), PropId::ScaleX, Value::Number(3.0));
    assert_eq!(stage.node(&pika()).map(|node| node.transform.scale), Some(Vec2::new(3.0, 2.0)));
    assert_eq!(stage.cast[&Who::Monster(25)].scale(), 2.0);
    set(&mut stage, &pika(), PropId::Scale, Value::Number(1.0));
    assert_eq!(stage.node(&pika()).map(|node| node.transform.scale), Some(Vec2::ONE));
    // (Back to the default; gone with its body.)
    assert_eq!(stage.set(&pika(), PropId::Angle, &Given::Default), Ok(()));
    assert_eq!(stage.get(&pika(), PropId::Angle), Some(Value::Number(0.0)));
    stage.remove(Who::Monster(25));
    assert!(stage.nodes.is_empty());
}

#[test]
fn values_relative_multiplied_and_by_another_node() {
    let mut stage = stage_with_pika();
    stage.put(Who::Monster(7), Vec2::new(10.0, 10.0), 1);
    set(&mut stage, &pika(), PropId::Angle, Value::Number(45.0));
    assert_eq!(stage.set(&pika(), PropId::Angle, &Given::Plus(Value::Number(30.0))), Ok(()));
    assert_eq!(stage.get(&pika(), PropId::Angle), Some(Value::Number(75.0)));
    set(&mut stage, &pika(), PropId::Scale, Value::Number(2.0));
    assert_eq!(stage.set(&pika(), PropId::Scale, &Given::Times(1.5)), Ok(()));
    assert_eq!(stage.cast[&Who::Monster(25)].scale(), 3.0);
    let by = Given::At { node: NodeRef::Who(Who::Monster(7)), by: Vec2::new(10.0, -4.0) };
    assert_eq!(stage.set(&pika(), PropId::Position, &by), Ok(()));
    assert_eq!(stage.cast[&Who::Monster(25)].position, Vec2::new(20.0, 6.0));
    assert_eq!(stage.set(&pika(), PropId::Position, &Given::Plus(Value::Point(Vec2::new(1.0, 1.0)))), Ok(()));
    assert_eq!(stage.cast[&Who::Monster(25)].position, Vec2::new(21.0, 7.0));
}

#[test]
fn what_cannot_be_set_is_said_why() {
    let mut stage = stage_with_pika();
    assert_eq!(stage.set(&pika(), PropId::Radius, &Given::Is(Value::Number(3.0))), Err(SetError::NotOfKind(NodeKind::Body)));
    assert_eq!(stage.set(&NodeRef::Who(Who::Monster(1)), PropId::Angle, &Given::Is(Value::Number(3.0))), Err(SetError::NoNode));
    assert_eq!(stage.set(&pika(), PropId::Scale, &Given::Is(Value::Number(-1.0))), Err(SetError::Impossible));
    assert_eq!(stage.set(&pika(), PropId::Angle, &Given::Is(Value::Bool(true))), Err(SetError::WrongValue));
    assert_eq!(stage.set(&pika(), PropId::Tint, &Given::Times(2.0)), Err(SetError::WrongValue));
    assert_eq!(stage.cast[&Who::Monster(25)].scale(), 1.0);
}

#[test]
fn nodes_of_a_film_s_own_a_picture_a_halo_an_emitter() {
    let mut stage = stage_with_pika();
    let group = NodeRef::Named("group".to_string());
    let make = NodeBeat::Make { name: "group".to_string(), sort: Sort::Group, at: Point::at(Vec2::new(10.0, 20.0)) };
    assert_eq!(stage.direct(Beat::Node(make), Vec2::ZERO), None);
    assert_eq!(stage.kind_of(&group), Some(NodeKind::Group));
    let set_angle = NodeBeat::Set { node: group.clone(), prop: PropId::Angle, value: Given::Is(Value::Number(90.0)) };
    stage.direct(Beat::Node(set_angle), Vec2::ZERO);
    assert_eq!(stage.node(&group).map(|node| (node.transform.pos, node.transform.angle_deg)), Some((Vec2::new(10.0, 20.0), 90.0)));
    assert_eq!(stage.get(&group, PropId::Height), None, "a group stands on nothing");
    // (A picture: the stage's own, its point as a node's.)
    let moon = NodeRef::Named("moon".to_string());
    let make = NodeBeat::Make { name: "moon".to_string(), sort: Sort::Picture("moon.png".to_string()), at: Point::at(Vec2::new(5.0, 5.0)) };
    stage.direct(Beat::Node(make), Vec2::ZERO);
    assert_eq!(stage.kind_of(&moon), Some(NodeKind::Picture));
    set(&mut stage, &moon, PropId::Position, Value::Point(Vec2::new(50.0, 60.0)));
    set(&mut stage, &moon, PropId::Opacity, Value::Number(0.3));
    assert_eq!(stage.words.pictures[0].at, Vec2::new(50.0, 60.0));
    assert_eq!(stage.node(&moon).map(|node| (node.transform.pos, node.looks.opacity)), Some((Vec2::new(50.0, 60.0), 0.3)));
    // (A halo by its rank, an emitter by who has it.)
    stage.light.halos.push(Halo { on: Cam::Who(Who::Monster(25)), radius: 40.0, colour: [255, 255, 255] });
    set(&mut stage, &NodeRef::Halo(0), PropId::Radius, Value::Number(90.0));
    set(&mut stage, &NodeRef::Halo(0), PropId::Colour, Value::Colour([255, 0, 0, 255]));
    assert_eq!((stage.light.halos[0].radius, stage.light.halos[0].colour), (90.0, [255, 0, 0]));
    assert_eq!(stage.node(&NodeRef::Halo(0)).map(|node| node.transform.pos), Some(Vec2::new(100.0, 80.0)));
    stage.light.emitters.push(Emitter { kind: ParticleKind::Sparks, who: Who::Monster(25), between: 15.0, colour: None, left: None, wait: 0.0 });
    let emitter = NodeRef::Emitter(Who::Monster(25));
    assert_eq!(stage.get(&emitter, PropId::Rate), Some(Value::Number(4.0)));
    set(&mut stage, &emitter, PropId::Rate, Value::Number(30.0));
    assert_eq!(stage.light.emitters[0].between, 2.0);
    assert_eq!(stage.node_refs(), vec![pika(), moon.clone(), group.clone(), NodeRef::Halo(0), emitter]);
    stage.direct(Beat::Node(NodeBeat::Unmake("moon".to_string())), Vec2::ZERO);
    assert!(stage.words.pictures.is_empty() && stage.kind_of(&moon).is_none());
}

#[test]
fn a_track_is_played_by_the_stage() {
    let mut stage = stage_with_pika();
    let key = crate::beat::Key { at: crate::beat::KeyTime::After(Frames(60.0)), value: Given::Is(Value::Number(90.0)), ease: Ease::Linear };
    let track = NodeBeat::Animate { node: pika(), prop: PropId::Angle, keys: vec![key], mode: TrackMode::Loop, relative: false, exact: false };
    assert_eq!(stage.direct(Beat::Node(track), Vec2::ZERO), None);
    assert_eq!(stage.tracks.len(), 1);
    run_frames(&mut stage, 30);
    assert!(stage.get(&pika(), PropId::Angle).and_then(|angle| angle.number()).is_some_and(|angle| (angle - 45.0).abs() < 1.0));
    stage.direct(Beat::Node(NodeBeat::Stop { node: pika(), prop: None }), Vec2::ZERO);
    assert!(stage.tracks.is_empty());
}

#[test]
fn several_effects_and_statuses_on_one_body() {
    let mut stage = stage_with_pika();
    let who = Who::Monster(25);
    stage.effect(who, 88, Some(10.0));
    stage.effect_with(who, 93, None, true);
    stage.effect_with(who, 88, Some(3.0), true);
    assert_eq!(stage.emotes.effects_of(who).iter().map(|effect| effect.id).collect::<Vec<_>>(), [93, 88]);
    run_frames(&mut stage, 3);
    assert_eq!(stage.emotes.effects_of(who).iter().map(|effect| effect.id).collect::<Vec<_>>(), [93]);
    stage.effect(who, 12, None);
    assert_eq!(stage.emotes.effects_of(who).iter().map(|effect| effect.id).collect::<Vec<_>>(), [12], "in their place");
    stage.emotes.effect_over(who, 12);
    assert!(stage.emotes.effects.is_empty());
    for (icon, add) in [("sleep", false), ("confused", true), ("sleep", true)] {
        stage.direct(Beat::Body(BodyBeat::Status { who, icon: Some(icon), add }), Vec2::ZERO);
    }
    assert_eq!(stage.emotes.statuses_of(who), ["sleep", "confused"]);
    stage.direct(Beat::Body(BodyBeat::Status { who, icon: None, add: false }), Vec2::ZERO);
    assert!(stage.emotes.statuses_of(who).is_empty());
}

/// A spin that turns its picture: its node's angle goes round, the way it
/// faces stays, and whole turns end upright; the old spin by the eight
/// ways is as it was. An angle told ends a spin that turns.
#[test]
fn a_spin_turns_the_picture_itself_if_asked() {
    let mut stage = stage_with_pika();
    let facing = stage.cast[&Who::Monster(25)].direction;
    stage.direct(Beat::Body(BodyBeat::Spin { who: Who::Monster(25), turns: 1.0, frames: Frames(40.0), rotate: true }), Vec2::ZERO);
    run_frames(&mut stage, 10);
    let angle = |stage: &Stage| stage.get(&pika(), PropId::Angle).and_then(|value| value.number()).expect("an angle");
    assert!((angle(&stage) - 90.0).abs() < 0.01, "{}", angle(&stage));
    assert_eq!(stage.cast[&Who::Monster(25)].direction, facing);
    run_frames(&mut stage, 40);
    assert_eq!(angle(&stage), 0.0);
    // (Half a turn: it stays upside down; an angle told sets it so.)
    stage.direct(Beat::Body(BodyBeat::Spin { who: Who::Monster(25), turns: 0.5, frames: Frames(10.0), rotate: true }), Vec2::ZERO);
    run_frames(&mut stage, 20);
    assert!((angle(&stage) - 180.0).abs() < 0.01);
    set(&mut stage, &pika(), PropId::Angle, Value::Number(30.0));
    assert!((angle(&stage) - 30.0).abs() < 0.01);
    // (By the eight ways: it faces round, its picture upright.)
    stage.direct(Beat::Body(BodyBeat::Spin { who: Who::Monster(25), turns: 1.0, frames: Frames(16.0), rotate: false }), Vec2::ZERO);
    run_frames(&mut stage, 4);
    assert_ne!(stage.cast[&Who::Monster(25)].direction, facing);
    assert!((angle(&stage) - 30.0).abs() < 0.01);
}

/// A node's map: turned about its pivot, slanted, flipped; the rectangle
/// that holds it.
#[test]
fn a_node_turns_about_its_pivot() {
    let turned = Transform { angle_deg: 90.0, pivot: Vec2::new(0.0, -10.0), ..Transform::default() };
    // (Its foot, 10 under its pivot, goes 10 to the left of it.)
    assert!(turned.about_pivot().transform_point2(Vec2::ZERO).distance(Vec2::new(-10.0, -10.0)) < 1e-4);
    assert!(turned.about_pivot().transform_point2(Vec2::new(0.0, -10.0)).distance(Vec2::new(0.0, -10.0)) < 1e-4);
    let flipped = Transform { flip_y: true, ..Transform::default() };
    assert_eq!(flipped.about_pivot().transform_point2(Vec2::new(3.0, 4.0)), Vec2::new(3.0, -4.0));
    let slanted = Transform { skew: Vec2::new(45.0, 0.0), ..Transform::default() };
    assert!(slanted.about_pivot().transform_point2(Vec2::new(0.0, 10.0)).distance(Vec2::new(10.0, 10.0)) < 1e-4);
    let (low, high) = turned.bounds(Vec2::new(-2.0, -4.0), Vec2::new(2.0, 0.0));
    assert!((high - low).distance(Vec2::new(4.0, 4.0)) < 1e-4);
    assert!(Transform::default().is_upright() && !turned.is_upright() && !flipped.is_upright());
    assert!(!Transform { scale: Vec2::new(2.0, 1.0), ..Transform::default() }.is_upright());
}

fn run_frames(stage: &mut Stage, frames: usize) {
    for _ in 0..frames {
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    }
}

/// The game plays on the same stage: its defaults stay as they were. The
/// DS's screen, collisions on, today's order of what is drawn, the view
/// at its usual closeness on no one, no node told anything. (Each
/// workstream that touches the engine checks its own here.)
#[test]
fn the_game_side_defaults() {
    let stage = Stage::default();
    assert!(!stage.no_collisions, "collisions on");
    assert_eq!(camera::SCREEN, Vec2::new(256.0, 192.0), "the DS's screen");
    assert_eq!(stage.view, Camera::default());
    assert_eq!((stage.view.zoom.now, stage.view.zoom.to), (1.0, 1.0));
    assert!(stage.view.camera.is_none() && stage.view.framing.is_empty() && !stage.view.roomy);
    assert!(stage.nodes.is_empty());
    // (The place, the bodies by their feet, what is over them, the words.)
    let at = |sheet: Sheet| SHEETS.iter().position(|known| *known == sheet).expect("a sheet");
    let order = [
        Sheet::Place,
        Sheet::PlaceUpper,
        Sheet::Bodies,
        Sheet::Statuses,
        Sheet::Effects,
        Sheet::Cutaway,
        Sheet::Pictures,
        Sheet::Bubbles,
        Sheet::Fade,
        Sheet::Caption,
    ];
    assert!(order.windows(2).all(|pair| at(pair[0]) < at(pair[1])), "today's order");
    assert_eq!((SHEETS[0], SHEETS[SHEETS.len() - 1]), (Sheet::Place, Sheet::Box));
    assert!(Layer::Ground < Layer::Among && Layer::Among < Layer::Over);
    // (No `z` told: every rank is today's, the box over all the rest, and
    // the bodies by their feet among the sheets, where the bodies' sheet was.)
    let ranks: Vec<f32> = SHEETS.iter().map(|sheet| sheet.z()).collect();
    assert!(ranks.windows(2).all(|pair| pair[0] < pair[1]), "sheets by rank, as they were");
    assert!(Sheet::Place.z() < Layer::Ground.z() && Layer::Over.z() < Sheet::Lines.z() && Sheet::Minimap.z() < Sheet::Box.z());
    // (A body as a node: no `z` of its own, drawn by its feet.)
    let mut stage = stage;
    stage.put(Who::Character(0), Vec2::new(4.0, 9.0), 1);
    assert_eq!(stage.node(&NodeRef::Who(Who::Character(0))).and_then(|node| node.z), None);
    assert_eq!(stage.cast[&Who::Character(0)].drawn_at(), 9.0);
    stage.put(Who::Character(1), Vec2::new(4.0, 3.0), 1);
    let drawn: Vec<Drawn> = stage.drawing_order(|_| false).into_iter().map(|(_, drawn)| drawn).collect();
    let at = |one: Drawn| drawn.iter().position(|known| *known == one).expect("drawn");
    assert!(at(Drawn::Sheet(Sheet::PlaceLight)) < at(Drawn::Body(Who::Character(1))), "the place under them");
    assert!(at(Drawn::Body(Who::Character(1))) < at(Drawn::Body(Who::Character(0))), "by their feet");
    assert!(at(Drawn::Body(Who::Character(0))) < at(Drawn::Sheet(Sheet::Lines)) && at(Drawn::Sheet(Sheet::Caption)) < at(Drawn::Sheet(Sheet::Box)));
    // (Nothing turned, slanted or sized one way: drawn as the game draws.)
    assert!(stage.node(&NodeRef::Who(Who::Character(0))).is_some_and(|node| node.transform.is_upright() && node.looks.is_plain()));
}
