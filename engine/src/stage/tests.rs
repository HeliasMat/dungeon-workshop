use super::*;

fn run(stage: &mut Stage, frames: usize) -> Option<Vec2> {
    let mut focus = Some(Vec2::ZERO);
    for _ in 0..frames {
        focus = stage.advance(1.0, 0.0, focus);
    }
    focus
}

#[test]
fn someone_walks_where_sent_turned_that_way() {
    let mut stage = Stage::default();
    stage.put(Who::Character(1), Vec2::new(0.0, 0.0), 1);
    assert_eq!(
        stage.direct(
            Beat::Motion(MotionBeat::Go { who: Who::Character(1), at: glam::Vec2::new(30.0, 0.0).into(), frames: Frames(10.0), slide: false }),
            Vec2::ZERO
        ),
        None
    );
    run(&mut stage, 5);
    let actor = &stage.cast[&Who::Character(1)];
    assert_eq!(actor.position, Vec2::new(15.0, 0.0));
    assert!(actor.walks() && stage.busy());
    assert_eq!(actor.direction, 3);
    run(&mut stage, 5);
    assert_eq!(stage.cast[&Who::Character(1)].position, Vec2::new(30.0, 0.0));
    assert!(!stage.busy());
}

#[test]
fn doors_and_reflections() {
    let (me, double) = (Who::Character(0), Who::Monster(25));
    let mut stage = Stage::default();
    stage.direct(Beat::Play(PlayBeat::Wall(Some([0.0, 0.0, 10.0, 10.0].into()))), Vec2::ZERO);
    stage.direct(Beat::Play(PlayBeat::NamedWall { name: "carry".to_string(), wall: Some([100.0, 0.0, 110.0, 200.0].into()) }), Vec2::ZERO);
    stage.direct(Beat::Play(PlayBeat::NamedWall { name: "carry".to_string(), wall: None }), Vec2::ZERO);
    assert_eq!(stage.play.walls, vec![[0.0, 0.0, 10.0, 10.0].into()]);
    // (The double goes left as the leader goes right, up as it goes up.)
    stage.put(me, Vec2::new(50.0, 100.0), 3);
    stage.put(double, Vec2::new(150.0, 100.0), 3);
    stage.direct(Beat::Motion(MotionBeat::Mirror { who: double, model: Some(me) }), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::Go { who: me, at: glam::Vec2::new(70.0, 90.0).into(), frames: Frames(10.0), slide: false }), Vec2::ZERO);
    run(&mut stage, 20);
    assert!(stage.cast[&double].position.distance(Vec2::new(130.0, 90.0)) < 1.5, "{:?}", stage.cast[&double].position);
    stage.direct(Beat::Motion(MotionBeat::Mirror { who: double, model: None }), Vec2::ZERO);
}

#[test]
fn a_line_walks_where_its_leader_walked_and_tab_passes_the_hand() {
    let (a, b, c) = (Who::Character(1), Who::Character(2), Who::Character(3));
    let mut stage = Stage::default();
    stage.put(a, Vec2::new(40.0, 0.0), 3);
    stage.put(b, Vec2::new(20.0, 0.0), 3);
    stage.put(c, Vec2::new(0.0, 0.0), 3);
    stage.direct(Beat::Motion(MotionBeat::Train { gap: 20.0, who: vec![a, b, c] }), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::Go { who: a, at: glam::Vec2::new(40.0, 60.0).into(), frames: Frames(30.0), slide: false }), Vec2::ZERO);
    run(&mut stage, 200);
    // (Down the way the leader took, not straight to it.)
    assert!(stage.cast[&b].position.distance(Vec2::new(40.0, 40.0)) < 1.5, "{:?}", stage.cast[&b].position);
    assert!(stage.cast[&c].position.distance(Vec2::new(40.0, 20.0)) < 1.5, "{:?}", stage.cast[&c].position);
    stage.direct(Beat::Play(PlayBeat::ControlTeam(vec![a, b])), Vec2::ZERO);
    assert_eq!(stage.play.controlled, Some(a));
    assert!(stage.swap_control() && stage.play.controlled == Some(b));
    assert!(stage.swap_control() && stage.play.controlled == Some(a));
}

#[test]
fn a_jump_lands_where_it_started() {
    let mut stage = Stage::default();
    stage.put(Who::Character(1), Vec2::new(5.0, 5.0), 1);
    stage.direct(Beat::Body(BodyBeat::Jump { who: Who::Character(1), height: 10.0, frames: Frames(10.0) }), Vec2::ZERO);
    run(&mut stage, 5);
    assert_eq!(stage.cast[&Who::Character(1)].air.height, 10.0);
    run(&mut stage, 5);
    let actor = &stage.cast[&Who::Character(1)];
    assert_eq!((actor.position, actor.air.height, actor.air.arc), (Vec2::new(5.0, 5.0), 0.0, None));
}

#[test]
fn a_thing_given_leaves_once_there() {
    let mut stage = Stage::default();
    stage.put(Who::Character(1), Vec2::new(0.0, 0.0), 1);
    stage.put(Who::Character(2), Vec2::new(40.0, 0.0), 1);
    assert_eq!(stage.give(Who::Character(9), Who::Character(1), Who::Character(2), 8.0, false), Some(Vec2::new(0.0, 2.0)));
    assert!(stage.give(Who::Character(9), Who::Character(1), Who::Character(3), 8.0, false).is_none());
    run(&mut stage, 4);
    assert!(stage.cast[&Who::Character(9)].air.height > 14.0);
    run(&mut stage, 4);
    assert!(!stage.cast.contains_key(&Who::Character(9)));
}

#[test]
fn a_follower_stops_a_step_behind() {
    let mut stage = Stage::default();
    stage.put(Who::Character(1), Vec2::new(100.0, 0.0), 1);
    stage.put(Who::Character(2), Vec2::new(0.0, 0.0), 1);
    stage.direct(Beat::Motion(MotionBeat::Follow { who: Who::Character(2), leader: Some(Who::Character(1)), gap: None }), Vec2::ZERO);
    run(&mut stage, 120);
    let gap = stage.cast[&Who::Character(1)].position.distance(stage.cast[&Who::Character(2)].position);
    assert!((20.0..=26.0).contains(&gap), "{gap}");
}

#[test]
fn the_strongest_cue_wins() {
    let mut actor = Actor::at(Vec2::ZERO, 1);
    actor.cue(Cue::Pose(Pose::Sleep));
    actor.cue(Cue::Hurt);
    assert_eq!(actor.cue, Some(Cue::Pose(Pose::Sleep)));
    actor.cue = None;
    actor.cue(Cue::Hurt);
    actor.cue(Cue::Motion(3));
    assert_eq!(actor.cue, Some(Cue::Motion(3)));
}

#[test]
fn the_view_follows_pans_and_fades() {
    let mut stage = Stage::dark();
    stage.put(Who::Character(1), Vec2::new(50.0, 50.0), 1);
    stage.direct(Beat::View(ViewBeat::Camera(Some(Cam::Who(Who::Character(1))))), Vec2::ZERO);
    assert_eq!(run(&mut stage, 1), Some(Vec2::new(50.0, 50.0)));
    stage.direct(Beat::View(ViewBeat::Pan { to: Cam::at(Vec2::new(150.0, 50.0)), frames: Frames(10.0) }), Vec2::new(50.0, 50.0));
    stage.direct(Beat::Screen(ScreenBeat::Fade { out: false, white: false, frames: Frames(10.0) }), Vec2::ZERO);
    let mut focus = Some(Vec2::new(50.0, 50.0));
    for _ in 0..5 {
        focus = stage.advance(1.0, 0.0, focus);
    }
    assert_eq!(focus, Some(Vec2::new(100.0, 50.0)));
    assert!((stage.light.fade.level - 0.5).abs() < 1e-4);
    for _ in 0..5 {
        focus = stage.advance(1.0, 0.0, focus);
    }
    assert_eq!((focus, stage.view.camera, stage.light.fade.level), (Some(Vec2::new(150.0, 50.0)), Some(Cam::at(Vec2::new(150.0, 50.0))), 0.0));
}

#[test]
fn a_glide_goes_as_its_ease_was_told_when_it_began() {
    // (A quarter of the way in time: evenly, a quarter of the way; quickly at first, further.)
    for (ease, at) in [(Ease::Linear, 75.0), (Ease::Out(EaseFamily::Quad), 93.75), (Ease::Smooth, 65.625)] {
        let mut stage = Stage::dark();
        stage.direct(Beat::View(ViewBeat::Ease(ease)), Vec2::ZERO);
        stage.direct(Beat::View(ViewBeat::Pan { to: Cam::at(Vec2::new(150.0, 50.0)), frames: Frames(8.0) }), Vec2::new(50.0, 50.0));
        // (Told another way after it began: it goes on as it was.)
        stage.direct(Beat::View(ViewBeat::Ease(Ease::Smooth)), Vec2::ZERO);
        let mut focus = Some(Vec2::new(50.0, 50.0));
        for _ in 0..2 {
            focus = stage.advance(1.0, 0.0, focus);
        }
        assert_eq!(focus, Some(Vec2::new(at, 50.0)), "{ease:?}");
    }
}

#[test]
fn effects_last_their_time_and_leave_with_their_bearer() {
    let mut stage = Stage::default();
    stage.put(Who::Character(1), Vec2::ZERO, 1);
    stage.put(Who::Character(2), Vec2::ZERO, 1);
    stage.direct(Beat::Body(BodyBeat::Effect { who: Who::Character(1), id: EmoteId(88), frames: Some(Frames(3.0)), add: false }), Vec2::ZERO);
    stage.effect(Who::Character(2), 93, None);
    stage.effect(Who::Character(3), 93, None);
    stage.direct(Beat::Body(BodyBeat::Status { who: Who::Character(2), icon: Some("sleep"), add: false }), Vec2::ZERO);
    assert_eq!(stage.emotes.statuses_of(Who::Character(2)), ["sleep"]);
    stage.direct(Beat::Body(BodyBeat::Status { who: Who::Character(2), icon: None, add: false }), Vec2::ZERO);
    assert!(stage.emotes.statuses.is_empty());
    assert_eq!(stage.emotes.effects.len(), 2);
    run(&mut stage, 3);
    assert_eq!(stage.emotes.effects.keys().collect::<Vec<_>>(), [&Who::Character(2)]);
    stage.remove(Who::Character(2));
    assert!(stage.emotes.effects.is_empty());
}

#[test]
fn a_path_is_walked_point_after_point() {
    let mut stage = Stage::default();
    stage.put(Who::Character(1), Vec2::ZERO, 1);
    stage.direct(Beat::Motion(MotionBeat::Speed { who: Who::Character(1), pace: 2.0 }), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::Path { who: Who::Character(1), points: vec![Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)], run: false }), Vec2::ZERO);
    run(&mut stage, 5);
    assert_eq!(stage.cast[&Who::Character(1)].position, Vec2::new(10.0, 0.0));
    assert!(stage.busy());
    run(&mut stage, 6);
    assert_eq!(stage.cast[&Who::Character(1)].position, Vec2::new(10.0, 10.0));
    assert!(!stage.busy());
}

#[test]
fn wanderers_stay_near() {
    let mut stage = Stage::default();
    stage.put(Who::Character(1), Vec2::new(100.0, 100.0), 1);
    stage.put(Who::Character(2), Vec2::new(0.0, 100.0), 1);
    stage.direct(Beat::Motion(MotionBeat::Wander { who: Who::Character(1), radius: 20.0 }), Vec2::ZERO);
    let mut moved = false;
    for _ in 0..600 {
        stage.advance(1.0, 0.0, None);
        let at = stage.cast[&Who::Character(1)].position;
        assert!(at.distance(Vec2::new(100.0, 100.0)) <= 20.5);
        moved |= at != Vec2::new(100.0, 100.0);
    }
    assert!(moved, "it wandered");
    assert!(!stage.busy(), "wandering is no business to wait for");
}

#[test]
fn someone_fades_and_the_view_quakes_less_and_less() {
    let mut stage = Stage::default();
    stage.put(Who::Character(1), Vec2::ZERO, 1);
    stage.direct(Beat::Body(BodyBeat::Opacity { who: Who::Character(1), to: 0.0, frames: Frames(0.0) }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Opacity { who: Who::Character(1), to: 1.0, frames: Frames(10.0) }), Vec2::ZERO);
    run(&mut stage, 5);
    assert!((stage.cast[&Who::Character(1)].looks.opacity - 0.5).abs() < 1e-4);
    stage.direct(Beat::View(ViewBeat::Quake { strength: 8.0, frames: Frames(4.0) }), Vec2::ZERO);
    let first = stage.advance(1.0, 0.0, Some(Vec2::ZERO)).unwrap();
    assert!(first.length() > 0.0);
    run(&mut stage, 4);
    assert_eq!(stage.view.quake, None);
}

#[test]
fn someone_goes_round_and_takes_a_blow() {
    let mut stage = Stage::default();
    let (a, b) = (Who::Character(1), Who::Character(2));
    stage.put(a, Vec2::new(10.0, 0.0), 1);
    stage.put(b, Vec2::new(-20.0, 0.0), 1);
    stage.direct(Beat::Motion(MotionBeat::Orbit { who: a, at: glam::Vec2::new(0.0, 0.0).into(), turns: 0.5, frames: Frames(10.0) }), Vec2::ZERO);
    run(&mut stage, 5);
    assert!((stage.cast[&a].position - Vec2::new(0.0, 10.0)).length() < 0.01, "{:?}", stage.cast[&a].position);
    run(&mut stage, 5);
    assert!((stage.cast[&a].position - Vec2::new(-10.0, 0.0)).length() < 0.01);
    assert!(!stage.busy());
    stage.direct(Beat::Body(BodyBeat::Recoil { who: a, from: b, distance: 16.0 }), Vec2::ZERO);
    assert_eq!(stage.cast[&a].cue, Some(Cue::Hurt));
    run(&mut stage, 10);
    assert!((stage.cast[&a].position - Vec2::new(6.0, 0.0)).length() < 0.01);
    assert_eq!(stage.cast[&a].direction, 7, "turned to who struck");
}

#[test]
fn the_view_frames_everyone_and_a_spotlight_follows() {
    let mut stage = Stage::default();
    let (a, b) = (Who::Character(1), Who::Character(2));
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    stage.put(b, Vec2::new(160.0, 100.0), 1);
    stage.direct(Beat::View(ViewBeat::Frame { who: vec![a, b], at: Vec::new(), frames: Frames(10.0), most: None }), Vec2::ZERO);
    let mut focus = Some(Vec2::ZERO);
    for _ in 0..10 {
        focus = stage.advance(1.0, 0.0, focus);
    }
    // (Their middle, and room for the box below them from the start.)
    assert_eq!(focus, Some(Vec2::new(130.0, 92.0 + crate::stage::BOX_HIGH / 2.0)));
    assert_eq!(stage.view.zoom.now, 1.0, "as the original shows its scenes");
    stage.direct(Beat::View(ViewBeat::Spotlight(Some(Lit { on: Cam::Who(a), radius: 30.0 }))), Vec2::ZERO);
    run(&mut stage, 20);
    assert!((stage.light.spotlight.shown - 1.0).abs() < 1e-4);
    assert_eq!(stage.light.spotlight.at, Vec2::new(100.0, 88.0));
    stage.direct(Beat::View(ViewBeat::Spotlight(None)), Vec2::ZERO);
    run(&mut stage, 20);
    assert!(stage.light.spotlight.shown < 1e-4);
}

#[test]
fn steps_floats_and_colours() {
    let mut stage = Stage::default();
    let a = Who::Character(1);
    stage.put(a, Vec2::new(10.0, 10.0), 1);
    stage.direct(Beat::Motion(MotionBeat::Step { who: a, dx: 12.0, dy: 0.0, run: false }), Vec2::ZERO);
    run(&mut stage, 10);
    assert!((stage.cast[&a].position - Vec2::new(22.0, 10.0)).length() < 0.01);
    stage.direct(Beat::Body(BodyBeat::Float { who: a, height: 6.0, frames: Frames(20.0) }), Vec2::ZERO);
    run(&mut stage, 10);
    assert!((stage.cast[&a].shown_height() - 6.0).abs() < 1e-3);
    assert!(!stage.busy(), "floating is no business to wait for");
    stage.direct(Beat::Body(BodyBeat::Colour { who: a, colour: [0.0, 0.0, 0.0], frames: Frames(4.0) }), Vec2::ZERO);
    run(&mut stage, 2);
    assert!((stage.cast[&a].colour[0] - 127.5).abs() < 1e-3);
    run(&mut stage, 2);
    assert_eq!(stage.cast[&a].colour, [0.0; 3]);
}

#[test]
fn groups_stand_in_a_row() {
    let mut stage = Stage::default();
    let (a, b, c) = (Who::Character(1), Who::Character(2), Who::Character(3));
    for (who, x) in [(a, 0.0), (b, 50.0), (c, 100.0)] {
        stage.put(who, Vec2::new(x, 0.0), 5);
    }
    stage.direct(Beat::Motion(MotionBeat::LineUp { who: vec![a, b, c], at: Vec2::new(0.0, 40.0), gap: Vec2::new(30.0, 0.0) }), Vec2::ZERO);
    stage.settle();
    assert_eq!(stage.cast[&c].position, Vec2::new(60.0, 40.0));
    assert_eq!(stage.cast[&b].direction, 1, "facing down");
    stage.direct(Beat::Motion(MotionBeat::Flee { who: c, from: a, distance: 30.0 }), Vec2::ZERO);
    stage.settle();
    assert!(!stage.busy());
}

#[test]
fn an_iris_closes_and_opens_and_a_shake_stops() {
    let mut stage = Stage::default();
    stage.direct(Beat::View(ViewBeat::Iris { open: false, frames: Frames(10.0), on: None }), Vec2::new(5.0, 5.0));
    assert!(stage.light_busy());
    run(&mut stage, 10);
    assert_eq!((stage.light.spotlight.radius, stage.light.spotlight.black, stage.light.spotlight.shown), (0.0, true, 1.0));
    assert!(!stage.light_busy());
    stage.direct(Beat::View(ViewBeat::Iris { open: true, frames: Frames(10.0), on: None }), Vec2::ZERO);
    run(&mut stage, 10);
    assert_eq!(stage.light.spotlight.on, None);
    assert!(!stage.light.spotlight.black);
    stage.direct(Beat::View(ViewBeat::Shake { axis: 1, strength: 3.0, frames: Some(Frames(4.0)) }), Vec2::ZERO);
    run(&mut stage, 4);
    assert_eq!(stage.view.shake, Shake::default());
    stage.direct(Beat::View(ViewBeat::Camera(Some(Cam::at(Vec2::new(50.0, 50.0))))), Vec2::ZERO);
    assert_eq!(run(&mut stage, 1), Some(Vec2::new(50.0, 50.0)));
}

#[test]
fn what_is_held_stays_before_who_has_it() {
    let mut stage = Stage::default();
    let (me, apple) = (Who::Character(1), Who::Item { item: 109, copy: 0 });
    stage.put(me, Vec2::new(50.0, 50.0), 3);
    stage.put(apple, Vec2::ZERO, 1);
    stage.direct(Beat::Body(BodyBeat::Carry { what: apple, by: Some(me), hands: true }), Vec2::ZERO);
    run(&mut stage, 1);
    assert!(stage.cast[&apple].position.distance(Vec2::new(61.0, 50.0)) < 0.1, "on the right, where it faces");
    stage.direct(Beat::Body(BodyBeat::Carry { what: apple, by: None, hands: true }), Vec2::ZERO);
    assert_eq!(stage.cast[&apple].air.height, 0.0);
    run(&mut stage, 1);
    assert!(stage.cast[&apple].position.distance(Vec2::new(61.0, 50.0)) < 0.1, "left where it was");
}

#[test]
fn words_rise_and_go() {
    let mut stage = Stage::default();
    let me = Who::Character(1);
    stage.put(me, Vec2::new(50.0, 50.0), 1);
    stage.direct(Beat::Body(BodyBeat::Popup { who: me, text: "-12".to_string(), colour: [255, 40, 40] }), Vec2::ZERO);
    let start = stage.emotes.popups[0].at.y;
    run(&mut stage, 10);
    assert!(stage.emotes.popups[0].at.y < start);
    run(&mut stage, 80);
    assert!(stage.emotes.popups.is_empty());
}

#[test]
fn who_walks_steps_round_who_it_meets() {
    let mut stage = Stage::default();
    let (a, b) = (Who::Character(1), Who::Character(2));
    stage.put(a, Vec2::new(0.0, 0.0), 1);
    stage.put(b, Vec2::new(100.0, 0.0), 1);
    stage.direct(Beat::Motion(MotionBeat::Spacing(true)), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::Walk { who: a, at: glam::Vec2::new(100.0, 0.0).into(), run: false }), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::Walk { who: b, at: glam::Vec2::new(0.0, 0.0).into(), run: false }), Vec2::ZERO);
    let mut nearest = f32::MAX;
    for _ in 0..120 {
        run(&mut stage, 1);
        nearest = nearest.min(stage.cast[&a].position.distance(stage.cast[&b].position));
    }
    assert!(nearest > 8.0, "they went through each other: {nearest}");
    assert!(stage.cast[&a].position.distance(Vec2::new(100.0, 0.0)) < 1.0, "it got there");
    assert!(stage.cast[&b].position.distance(Vec2::ZERO) < 1.0);
}

#[test]
fn the_view_goes_back_for_big_bodies_and_never_comes_closer() {
    let mut stage = Stage::default();
    let (small, big) = (Who::Character(1), Who::Character(2));
    stage.put(small, Vec2::new(100.0, 200.0), 1);
    stage.put(big, Vec2::new(160.0, 200.0), 1);
    stage.direct(Beat::View(ViewBeat::KeepFrame(Some(vec![small, big]))), Vec2::ZERO);
    run(&mut stage, 200);
    assert_eq!(stage.view.zoom.now, 1.0, "two of the usual size");
    stage.direct(Beat::View(ViewBeat::Body { who: big, size: Vec2::new(70.0, 180.0) }), Vec2::ZERO);
    let focus = run(&mut stage, 200).unwrap();
    assert!((stage.view.zoom.now - 136.0 / 220.0).abs() < 0.01, "{}", stage.view.zoom.now);
    assert!((focus.y - 130.0).abs() < 0.5, "the body in sight, up to its head: {focus}");
    stage.direct(Beat::View(ViewBeat::KeepFrame(Some(vec![small]))), Vec2::ZERO);
    run(&mut stage, 200);
    assert!(stage.view.zoom.now <= 1.0);
}

#[test]
fn the_view_keeps_them_in_sight() {
    let mut stage = Stage::default();
    let (a, b) = (Who::Character(1), Who::Character(2));
    stage.put(a, Vec2::new(0.0, 0.0), 1);
    stage.put(b, Vec2::new(40.0, 0.0), 1);
    stage.direct(Beat::View(ViewBeat::Camera(Some(Cam::at(Vec2::new(-500.0, 0.0))))), Vec2::ZERO);
    stage.direct(Beat::View(ViewBeat::KeepFrame(Some(vec![a, b]))), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::Walk { who: b, at: glam::Vec2::new(200.0, 0.0).into(), run: true }), Vec2::ZERO);
    let focus = run(&mut stage, 200).unwrap();
    assert!((focus - Vec2::new(100.0, -8.0)).length() < 0.5, "{focus}");
    assert!(stage.view.zoom.now < 1.01, "further as they spread: {}", stage.view.zoom.now);
    stage.direct(Beat::View(ViewBeat::KeepFrame(None)), Vec2::ZERO);
    assert!(stage.view.framing.is_empty());
}

/// (After a fight's framing, or a `vue`: `zoom 0.5 1` once did nothing.)
#[test]
fn told_how_close_or_where_the_view_lets_go_of_who_it_kept() {
    let mut stage = Stage::default();
    let (a, b) = (Who::Character(1), Who::Character(2));
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    stage.put(b, Vec2::new(160.0, 100.0), 1);
    stage.direct(Beat::View(ViewBeat::KeepFrame(Some(vec![a, b]))), Vec2::ZERO);
    run(&mut stage, 200);
    assert_eq!(stage.view.zoom.now, 1.0);
    stage.direct(Beat::View(ViewBeat::Zoom { to: 0.5, frames: Frames(1.0) }), Vec2::ZERO);
    run(&mut stage, 60);
    assert_eq!(stage.view.zoom.now, 0.5, "it does not go back to them");
    assert!(stage.view.framing.is_empty());
    stage.direct(Beat::View(ViewBeat::KeepFrame(Some(vec![a, b]))), Vec2::ZERO);
    stage.direct(Beat::View(ViewBeat::Camera(Some(Cam::at(Vec2::new(300.0, 100.0))))), Vec2::ZERO);
    assert_eq!(run(&mut stage, 60), Some(Vec2::new(300.0, 100.0)));
}

#[test]
fn the_player_walks_someone_about() {
    let mut stage = Stage::default();
    let a = Who::Character(1);
    stage.put(a, Vec2::ZERO, 1);
    stage.direct(Beat::Play(PlayBeat::Control(Some(a))), Vec2::ZERO);
    for _ in 0..10 {
        stage.steer(Vec2::X, false);
        stage.advance(1.0, 0.0, None);
    }
    let actor = &stage.cast[&a];
    assert!(actor.position.x > 5.0 && actor.walks(), "{}", actor.position);
    stage.steer(Vec2::ZERO, false);
    assert!(!stage.cast[&a].walks(), "stops when let go");
    // (A walk the scene asks for goes on, keys or not.)
    stage.direct(Beat::Motion(MotionBeat::Walk { who: a, at: glam::Vec2::new(100.0, 0.0).into(), run: false }), Vec2::ZERO);
    stage.steer(Vec2::ZERO, false);
    assert!(stage.cast[&a].walks());
    // (Kept in its box.)
    stage.direct(Beat::Play(PlayBeat::Bounds(Some([-10.0, -10.0, 10.0, 10.0].into()))), Vec2::ZERO);
    stage.steer(Vec2::X, false);
    assert!(stage.cast[&a].way.to.x <= 10.0);
    stage.direct(Beat::Play(PlayBeat::Control(None)), Vec2::ZERO);
    let to = stage.cast[&a].way.to;
    stage.steer(Vec2::Y, false);
    assert_eq!(stage.cast[&a].way.to, to, "no one walked about any more");
}

#[test]
fn one_stopped_on_its_way_goes_on_along_it() {
    let mut stage = Stage::default();
    let (a, b) = (Who::Character(1), Who::Character(2));
    stage.put(a, Vec2::ZERO, 1);
    stage.direct(Beat::Motion(MotionBeat::Path { who: a, points: vec![Vec2::new(60.0, 0.0), Vec2::new(60.0, 60.0)], run: false }), Vec2::ZERO);
    run(&mut stage, 10);
    stage.direct(Beat::Body(BodyBeat::Stop(a)), Vec2::ZERO);
    let stopped = stage.cast[&a].position;
    run(&mut stage, 20);
    assert_eq!(stage.cast[&a].position, stopped);
    assert!(!stage.busy());
    stage.direct(Beat::Motion(MotionBeat::Resume(a)), Vec2::ZERO);
    run(&mut stage, 200);
    assert_eq!(stage.cast[&a].position, Vec2::new(60.0, 60.0));
    // (Round someone, who is followed as it moves.)
    stage.put(b, Vec2::new(60.0, 100.0), 1);
    stage.direct(Beat::Motion(MotionBeat::OrbitAround { who: a, other: b, turns: 1.0, frames: Frames(60.0) }), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::Go { who: b, at: glam::Vec2::new(160.0, 100.0).into(), frames: Frames(60.0), slide: true }), Vec2::ZERO);
    run(&mut stage, 60);
    assert!(stage.cast[&a].position.distance(Vec2::new(160.0, 100.0)) < 42.0, "{}", stage.cast[&a].position);
}

#[test]
fn the_view_leaves_room_for_the_box_and_a_glide_may_stay() {
    let mut stage = Stage::default();
    let a = Who::Character(1);
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    stage.direct(Beat::View(ViewBeat::KeepFrame(Some(vec![a]))), Vec2::ZERO);
    let mut focus = Some(Vec2::new(100.0, 92.0));
    for _ in 0..60 {
        focus = stage.advance(1.0, 0.0, focus);
    }
    let free = focus.unwrap().y;
    stage.words.boxed = true;
    for _ in 0..120 {
        focus = stage.advance(1.0, 0.0, focus);
    }
    assert!((focus.unwrap().y - free - 28.0).abs() < 1.0, "{free} {focus:?}");
    stage.words.boxed = false;
    stage.direct(Beat::View(ViewBeat::KeepFrame(None)), Vec2::ZERO);
    // (Up to where someone was; not after it.)
    stage.direct(Beat::View(ViewBeat::PanStays { to: Cam::Who(a), frames: Frames(5.0) }), Vec2::ZERO);
    for _ in 0..6 {
        focus = stage.advance(1.0, 0.0, focus);
    }
    assert_eq!(stage.view.camera, Some(Cam::at(Vec2::new(100.0, 100.0))));
    stage.put(a, Vec2::new(300.0, 100.0), 1);
    stage.direct(Beat::View(ViewBeat::Pan { to: Cam::Who(a), frames: Frames(5.0) }), Vec2::ZERO);
    for _ in 0..6 {
        stage.advance(1.0, 0.0, focus);
    }
    assert_eq!(stage.view.camera, Some(Cam::Who(a)));
}

#[test]
fn a_body_held_trembles_by_what_is_told_and_goes_nowhere() {
    let mut stage = Stage::default();
    let a = Who::Character(1);
    stage.put(a, Vec2::ZERO, 1);
    stage.direct(Beat::Body(BodyBeat::Tremble { who: a, frames: Frames(20.0), strength: 3.0 }), Vec2::ZERO);
    run(&mut stage, 1);
    assert_eq!(stage.cast[&a].shown_at().x.abs(), 3.0);
    stage.direct(Beat::Motion(MotionBeat::Walk { who: a, at: glam::Vec2::new(50.0, 0.0).into(), run: false }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Freeze { who: a, on: true }), Vec2::ZERO);
    run(&mut stage, 30);
    assert_eq!((stage.cast[&a].position, stage.cast[&a].picture_speed(), stage.busy()), (Vec2::ZERO, 0.0, false));
    stage.direct(Beat::Body(BodyBeat::Freeze { who: a, on: false }), Vec2::ZERO);
    run(&mut stage, 200);
    assert_eq!((stage.cast[&a].position.x, stage.cast[&a].picture_speed()), (50.0, 1.0));
}

#[test]
fn particles_come_off_someone_as_it_goes_and_stop() {
    let mut stage = Stage::default();
    let a = Who::Character(1);
    stage.put(a, Vec2::new(500.0, 500.0), 1);
    stage.direct(
        Beat::Screen(ScreenBeat::Emitter { kind: ParticleKind::Embers, who: a, each_second: 30.0, colour: None, frames: Some(Frames(40.0)) }),
        Vec2::ZERO,
    );
    run(&mut stage, 20);
    assert!(stage.light.particles.len() >= 8 && stage.light.particles.iter().all(|particle| particle.sharp));
    assert!(stage.light.particles.iter().all(|particle| particle.at.distance(Vec2::new(500.0, 484.0)) < 80.0));
    run(&mut stage, 30);
    assert!(stage.light.emitters.is_empty());
}

#[test]
fn particles_credits_and_wipes() {
    let mut stage = Stage::default();
    stage.direct(Beat::Screen(ScreenBeat::Particles { kind: ParticleKind::Fireworks, on: Cam::at(Vec2::ZERO), count: 30, colour: None }), Vec2::ZERO);
    assert_eq!(stage.light.particles.len(), 30);
    run(&mut stage, 120);
    assert!(stage.light.particles.is_empty(), "burnt out");
    stage.direct(Beat::Screen(ScreenBeat::Credits { frames: Frames(60.0), lines: vec!["The End".to_string()] }), Vec2::ZERO);
    stage.direct(Beat::Screen(ScreenBeat::Wipe { out: true, side: 0, frames: Frames(20.0) }), Vec2::ZERO);
    assert!(stage.light_busy());
    run(&mut stage, 20);
    assert!((stage.light.wipe.shown - 1.0).abs() < 1e-4, "covered");
    run(&mut stage, 40);
    assert!(stage.words.credits.is_none() && !stage.light_busy());
}

#[test]
fn walls_solids_pushing_and_eyes() {
    let mut stage = Stage::default();
    let (me, guard, rock) = (Who::Character(0), Who::Monster(322), Who::Monster(74));
    stage.put(me, Vec2::new(0.0, 0.0), 3);
    stage.direct(Beat::Play(PlayBeat::Control(Some(me))), Vec2::ZERO);
    stage.direct(Beat::Play(PlayBeat::Wall(Some([20.0, -50.0, 40.0, 50.0].into()))), Vec2::ZERO);
    for _ in 0..40 {
        stage.steer(Vec2::X, false);
        stage.advance(1.0, 0.0, None);
    }
    assert!(stage.cast[&me].position.x < 16.0, "stopped by the wall: {}", stage.cast[&me].position);
    // (Seen in front, not through the wall nor from behind.)
    stage.put(guard, Vec2::new(0.0, 40.0), 5);
    assert!(stage.sees(guard, me, 64.0, 90.0));
    stage.put(guard, Vec2::new(70.0, 0.0), 7);
    assert!(!stage.sees(guard, me, 100.0, 90.0), "the wall between");
    stage.put(guard, Vec2::new(0.0, 40.0), 1);
    assert!(!stage.sees(guard, me, 64.0, 90.0), "it looks away");
    // (Out of a wall one was put in; the walls gone with their place.)
    stage.put(me, Vec2::new(30.0, 0.0), 3);
    stage.steer(Vec2::X, false);
    assert!(stage.cast[&me].walks(), "gets out");
    stage.new_place();
    assert!(stage.play.walls.is_empty());
    // (A rock pushed a step.)
    stage.direct(Beat::Play(PlayBeat::Wall(None)), Vec2::ZERO);
    stage.put(rock, stage.cast[&me].position + Vec2::new(14.0, 0.0), 1);
    stage.cast.get_mut(&rock).unwrap().habits.pushable = true;
    let before = stage.cast[&rock].position;
    stage.steer(Vec2::X, false);
    assert_eq!(stage.cast[&rock].way.to, before + Vec2::new(16.0, 0.0));
}

#[test]
fn walked_round_a_wall() {
    let mut stage = Stage::default();
    let who = Who::Character(0);
    stage.put(who, Vec2::new(0.0, 0.0), 1);
    stage.direct(Beat::Play(PlayBeat::Wall(Some([40.0, -40.0, 60.0, 40.0].into()))), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::WalkTo { who, at: glam::Vec2::new(100.0, 0.0).into(), run: false }), Vec2::ZERO);
    for _ in 0..400 {
        stage.advance(1.0, 0.0, None);
        assert!(!walled(&stage.play.walls, stage.cast[&who].position), "through the wall at {}", stage.cast[&who].position);
    }
    assert!(stage.cast[&who].position.distance(Vec2::new(100.0, 0.0)) < 1.0, "got there: {}", stage.cast[&who].position);
    // (No way: shut in.)
    assert_eq!(
        way_round(
            &[[-100.0, -100.0, 200.0, -10.0], [-100.0, 10.0, 200.0, 100.0], [-100.0, -100.0, -10.0, 100.0], [10.0, -100.0, 200.0, 100.0]].map(Rect::from),
            &[],
            Vec2::ZERO,
            Vec2::new(150.0, 0.0)
        ),
        None
    );
}

#[test]
fn thrown_falling_and_more_light() {
    let mut stage = Stage::default();
    let (rock, me) = (Who::Monster(74), Who::Character(0));
    stage.put(rock, Vec2::ZERO, 1);
    stage.put(me, Vec2::new(60.0, 20.0), 1);
    stage.direct(Beat::Body(BodyBeat::Throw { who: rock, at: me, height: 20.0, frames: Frames(10.0) }), Vec2::ZERO);
    run(&mut stage, 10);
    assert_eq!(stage.cast[&rock].position, Vec2::new(60.0, 20.0));
    stage.direct(Beat::Body(BodyBeat::Fall { who: rock, height: 80.0, frames: Frames(8.0) }), Vec2::ZERO);
    assert_eq!(stage.cast[&rock].air.height, 80.0);
    run(&mut stage, 8);
    assert!(stage.cast[&rock].air.height.abs() < 0.01, "on the ground");
    stage.direct(Beat::View(ViewBeat::Spotlight(Some(Lit { on: Cam::Who(me), radius: 30.0 }))), Vec2::ZERO);
    stage.direct(Beat::View(ViewBeat::SpotlightAdd(Cam::at(Vec2::new(5.0, 5.0)), 20.0)), Vec2::ZERO);
    stage.direct(Beat::View(ViewBeat::SpotlightAdd(Cam::at(Vec2::new(5.0, 5.0)), 25.0)), Vec2::ZERO);
    assert_eq!(stage.light.lights, vec![Lit { on: Cam::at(Vec2::new(5.0, 5.0)), radius: 25.0 }]);
    stage.direct(Beat::View(ViewBeat::Spotlight(None)), Vec2::ZERO);
    assert!(stage.light.lights.is_empty());
}

#[test]
fn layers_sizes_and_shadows() {
    let mut stage = Stage::default();
    let a = Who::Character(1);
    stage.put(a, Vec2::ZERO, 1);
    stage.direct(Beat::Body(BodyBeat::Attr { who: a, bits: 32 }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Layer { who: a, place: crate::beat::Stacked::Over }), Vec2::ZERO);
    assert_eq!(stage.cast[&a].attributes, 36);
    stage.direct(Beat::Body(BodyBeat::Blink { who: a, on: false }), Vec2::ZERO);
    assert_eq!(stage.cast[&a].attributes, 4);
    stage.direct(Beat::Body(BodyBeat::Scale { who: a, to: 2.0, frames: Frames(4.0) }), Vec2::ZERO);
    run(&mut stage, 2);
    assert!((stage.cast[&a].looks.scale - 1.5).abs() < 1e-4);
    run(&mut stage, 2);
    assert_eq!(stage.cast[&a].looks.scale, 2.0);
    stage.direct(Beat::Body(BodyBeat::Shadow { who: a, on: false }), Vec2::ZERO);
    assert!(stage.cast[&a].shadowless);
}

#[test]
fn hops_glances_and_looking_about() {
    let mut stage = Stage::default();
    let a = Who::Character(1);
    stage.put(a, Vec2::ZERO, 1);
    stage.direct(Beat::Body(BodyBeat::Bounce { who: a, height: 4.0, frames: Frames(10.0) }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Fidget { who: a, on: true }), Vec2::ZERO);
    run(&mut stage, 5);
    assert!((stage.cast[&a].shown_height() - 4.0).abs() < 1e-3);
    assert!(!stage.busy(), "hopping and looking about are no business to wait for");
    let mut turned = false;
    for _ in 0..600 {
        stage.advance(1.0, 0.0, None);
        turned |= stage.cast[&a].direction != 1;
    }
    assert!(turned, "it looked about");
    stage.direct(Beat::View(ViewBeat::Camera(Some(Cam::at(Vec2::new(0.0, 0.0))))), Vec2::ZERO);
    stage.direct(Beat::View(ViewBeat::Glance { to: Cam::at(Vec2::new(100.0, 0.0)), frames: Frames(10.0), hold: 5.0 }), Vec2::ZERO);
    let mut focus = Some(Vec2::ZERO);
    let mut farthest: f32 = 0.0;
    for _ in 0..40 {
        assert!(stage.view.busy() || focus == Some(Vec2::ZERO));
        focus = stage.advance(1.0, 0.0, focus);
        farthest = farthest.max(focus.unwrap().x);
    }
    assert_eq!((farthest, focus), (100.0, Some(Vec2::ZERO)), "there and back");
    assert!(!stage.view.busy());
}

#[test]
fn captions_write_themselves_out_and_someone_turns_round() {
    let mut stage = Stage::default();
    stage.direct(Beat::Screen(ScreenBeat::Caption { frames: Frames(100.0), text: "Hello".to_string(), typed: true }), Vec2::ZERO);
    run(&mut stage, 4);
    assert_eq!(stage.words.caption.as_ref().map(Caption::shown).as_deref(), Some("He"));
    run(&mut stage, 20);
    assert_eq!(stage.words.caption.as_ref().map(Caption::shown).as_deref(), Some("Hello"));
    let a = Who::Character(1);
    for (from, to) in [(1, 5), (3, 7), (6, 2), (8, 4)] {
        stage.put(a, Vec2::ZERO, from);
        stage.direct(Beat::Body(BodyBeat::TurnAround(a)), Vec2::ZERO);
        assert_eq!(stage.cast[&a].direction, to, "from {from}");
    }
}

#[test]
fn someone_and_the_view_put_back_as_usual() {
    let mut stage = Stage::default();
    let a = Who::Character(1);
    stage.put(a, Vec2::new(5.0, 5.0), 3);
    stage.direct(Beat::Body(BodyBeat::Scale { who: a, to: 2.0, frames: Frames(0.0) }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Colour { who: a, colour: [0.0; 3], frames: Frames(0.0) }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Effect { who: a, id: EmoteId(88), frames: None, add: false }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Reset(a)), Vec2::ZERO);
    assert_eq!(stage.cast[&a], Actor::at(Vec2::new(5.0, 5.0), 3));
    stage.direct(Beat::View(ViewBeat::Zoom { to: 2.0, frames: Frames(1.0) }), Vec2::ZERO);
    stage.direct(Beat::View(ViewBeat::Tempo(0.5)), Vec2::ZERO);
    stage.direct(Beat::View(ViewBeat::Letterbox(true)), Vec2::ZERO);
    assert_eq!((stage.view.zoom.to, stage.tempo, stage.words.letterbox.to), (2.0, Some(0.5), 1.0));
}

#[test]
fn beats_off_the_stage_are_given_back() {
    let mut stage = Stage::default();
    assert_eq!(stage.direct(Beat::Flow(FlowBeat::Wait(Frames(3.0))), Vec2::ZERO), Some(Beat::Flow(FlowBeat::Wait(Frames(3.0)))));
    assert_eq!(stage.direct(Beat::Speech(SpeechBeat::Said), Vec2::ZERO), Some(Beat::Speech(SpeechBeat::Said)));
    // (Someone not on the stage: nothing happens.)
    assert_eq!(stage.direct(Beat::Body(BodyBeat::Turn { who: Who::Character(7), direction: Direction::DownRight }), Vec2::ZERO), None);
    assert_eq!(towards(Vec2::ZERO, Vec2::new(0.0, 10.0)), Some(1));
    assert_eq!(towards(Vec2::ZERO, Vec2::ZERO), None);
}

#[test]
fn a_picture_glides_and_comes_over_the_one_shown() {
    let mut stage = Stage::default();
    let cut = |level, frames| Beat::Screen(ScreenBeat::Cutaway { level, x: 0.0, y: 0.0, over: false, frames });
    stage.direct(cut(1, Frames(0.0)), Vec2::ZERO);
    stage.direct(Beat::Screen(ScreenBeat::CutawayPan { x: 10.0, y: 0.0, frames: Frames(10.0) }), Vec2::ZERO);
    stage.advance(5.0, 0.0, None);
    assert_eq!(stage.light.cutaway.as_ref().map(|cutaway| cutaway.at.x), Some(5.0));
    stage.direct(cut(2, Frames(10.0)), Vec2::ZERO);
    stage.advance(5.0, 0.0, None);
    let cutaway = stage.light.cutaway.as_ref().expect("shown");
    assert_eq!((cutaway.level, cutaway.shown()), (2, 0.5));
    assert_eq!(cutaway.before.as_ref().map(|before| before.at.x), Some(10.0));
    stage.advance(5.0, 0.0, None);
    assert!(stage.light.cutaway.as_ref().expect("shown").before.is_none());
    stage.direct(Beat::Screen(ScreenBeat::CutawayOff), Vec2::ZERO);
    assert!(stage.light.cutaway.is_none());
}

#[test]
fn a_shot_hurts_and_shows_when_it_lands_not_before() {
    let (thrower, target) = (Who::Character(0), Who::Character(1));
    let mut stage = Stage::default();
    stage.put(thrower, Vec2::new(0.0, 0.0), 1);
    stage.put(target, Vec2::new(32.0, 0.0), 1);
    stage.shoot(thrower, target, 7, 9);
    stage.advance(SHOT_FRAMES - 1.0, 0.0, None);
    assert_eq!(stage.shots.len(), 1);
    assert_eq!(stage.shots[0].at(stage.cast[&target].position), Vec2::new(30.0, 0.0));
    assert_eq!(stage.cast[&target].cue, None);
    assert!(stage.emotes.effects.is_empty());
    stage.advance(1.0, 0.0, None);
    assert!(stage.shots.is_empty());
    assert_eq!(stage.cast[&target].cue, Some(Cue::Hurt));
    assert_eq!(stage.emotes.effects[&target][0].id, 9);
}

#[test]
fn no_one_stays_when_the_curtain_falls() {
    let mut stage = Stage::default();
    stage.put(Who::Character(0), Vec2::ZERO, 1);
    stage.curtain("The End");
    assert!(stage.cast.is_empty());
    assert_eq!(stage.words.caption.as_ref().map(|caption| caption.text.as_str()), Some("The End"));
}

#[test]
fn a_shot_that_does_not_hurt_only_shows() {
    let (thrower, target) = (Who::Character(0), Who::Character(1));
    let mut stage = Stage::default();
    stage.put(thrower, Vec2::new(0.0, 0.0), 1);
    stage.put(target, Vec2::new(32.0, 0.0), 1);
    stage.shoot_as(thrower, target, 7, 9, 4.0, false, false);
    stage.advance(4.0, 0.0, None);
    assert!(stage.shots.is_empty());
    assert_eq!(stage.cast[&target].cue, None);
    assert_eq!(stage.emotes.effects[&target][0].id, 9);
}

#[test]
fn a_lob_rises_on_its_way_and_a_straight_shot_does_not() {
    let shot = |gone, lob| Shot { effect: 7, from: Vec2::ZERO, target: Who::Character(1), hit: 0, gone, whole: 16.0, lob, hurts: false };
    assert_eq!(shot(0.0, true).rise(), 0.0);
    assert_eq!(shot(16.0, true).rise(), 0.0);
    assert_eq!(shot(8.0, true).rise(), LOB_HEIGHT);
    assert!(shot(4.0, true).rise() < LOB_HEIGHT);
    assert_eq!(shot(8.0, false).rise(), 0.0);
}

#[test]
fn a_thing_in_the_hands_goes_with_who_holds_it_in_front_or_behind() {
    let mut stage = Stage::default();
    let (who, first, second) = (Who::Monster(1), Who::Item { item: 70, copy: 0 }, Who::Drawn { picture: 1, copy: 0 });
    stage.put(who, Vec2::new(100.0, 100.0), 1);
    stage.put(first, Vec2::new(100.0, 100.0), 1);
    stage.put(second, Vec2::new(100.0, 100.0), 1);
    for thing in [first, second] {
        stage.direct(Beat::Body(BodyBeat::Carry { what: thing, by: Some(who), hands: true }), Vec2::ZERO);
    }
    stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    let order = |stage: &Stage| stage.in_drawing_order(|_| false).iter().map(|(one, _)| *one).collect::<Vec<Who>>();
    assert_eq!(order(&stage)[0], who, "looking down, what it holds is before it");
    let told = stage.things();
    assert_eq!(told.len(), 2);
    assert!(told.iter().all(|thing| thing.owner == Some(who)));
    assert_ne!(told[0].place, told[1].place, "side by side");
    stage.direct(Beat::Body(BodyBeat::Turn { who, direction: Direction::Up }), Vec2::ZERO);
    stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    assert_eq!(*order(&stage).last().expect("someone"), who, "looking away, it hides what it holds");
    // (Up with a jump, hidden with who has it.)
    stage.direct(Beat::Body(BodyBeat::Jump { who, height: 20.0, frames: Frames(20.0) }), Vec2::ZERO);
    for _ in 0..10 {
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    }
    assert!(stage.thing(first).expect("there").height > 20.0);
    stage.direct(Beat::Body(BodyBeat::Attr { who, bits: attribute::HIDDEN }), Vec2::ZERO);
    stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    assert_eq!(stage.cast[&first].attributes & attribute::HIDDEN, attribute::HIDDEN);
    stage.remove(who);
    assert!(stage.things().is_empty(), "who leaves takes what it holds");
}

#[test]
fn a_thing_given_shows_over_the_giver_and_lands_in_the_other_hands() {
    let mut stage = Stage::default();
    let (from, to, what) = (Who::Monster(1), Who::Monster(2), Who::Drawn { picture: 1, copy: 0 });
    stage.put(from, Vec2::new(100.0, 100.0), 3);
    stage.put(to, Vec2::new(200.0, 100.0), 7);
    stage.give(what, from, to, 30.0, true);
    stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    assert!(stage.thing(what).expect("there").height >= 22.0 && stage.thing(what).expect("there").owner == Some(from));
    for _ in 0..30 {
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    }
    let flying = stage.thing(what).expect("there");
    assert!(flying.owner.is_none() && flying.place.x > 110.0 && flying.place.x < 195.0, "{flying:?}");
    for _ in 0..30 {
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    }
    assert_eq!(stage.thing(what).expect("there").owner, Some(to));
}

#[test]
fn a_thing_of_the_game_is_a_thing_too() {
    let mut stage = Stage::default();
    stage.put(Who::Thing { object: 66, copy: 0 }, Vec2::new(10.0, 20.0), 1);
    stage.direct(Beat::Body(BodyBeat::ThingName { who: Who::Thing { object: 66, copy: 0 }, name: "chest".to_string() }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Anim { who: Who::Thing { object: 66, copy: 0 }, anim: AnimId(0), play: crate::beat::Play::Hold(3) }), Vec2::ZERO);
    let thing = stage.thing(Who::Thing { object: 66, copy: 0 }).expect("there");
    assert_eq!((thing.picture, thing.frame, thing.name.as_str()), (Picture::Prop(66), Some(3), "chest"));
    assert!(stage.thing(Who::Monster(1)).is_none());
}

#[test]
fn a_line_flies_where_it_is_told_and_who_holds_it_keeps_an_eye_on_it() {
    let mut stage = Stage::default();
    let (a, bobber) = (Who::Monster(25), Who::Drawn { picture: 0, copy: 0 });
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    stage.put(bobber, Vec2::new(100.0, 100.0), 1);
    stage.direct(Beat::Body(BodyBeat::FishLine { who: a, to: Some(bobber), at: Some(Vec2::new(160.0, 40.0)), cast: true }), Vec2::ZERO);
    assert_eq!(stage.lines[&a], bobber);
    for _ in 0..40 {
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    }
    assert_eq!(stage.cast[&bobber].position, Vec2::new(160.0, 40.0));
    assert!(stage.cast[&bobber].shown_height() < 1.0);
    stage.direct(Beat::Body(BodyBeat::Turn { who: a, direction: Direction::Left }), Vec2::ZERO);
    stage.advance(1.0, 0.0, Some(Vec2::ZERO));
    assert_eq!(stage.cast[&a].direction, towards(Vec2::new(100.0, 100.0), Vec2::new(160.0, 40.0)).expect("a way"));
    stage.direct(Beat::Body(BodyBeat::FishLine { who: a, to: None, at: None, cast: true }), Vec2::ZERO);
    assert!(stage.lines.is_empty());
    // (Told no point: before it.)
    stage.direct(Beat::Body(BodyBeat::Turn { who: a, direction: Direction::Down }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::FishLine { who: a, to: Some(bobber), at: None, cast: true }), Vec2::ZERO);
    assert!(stage.cast[&bobber].goal().y > 100.0);
    // (Tied to a catch, out on the water: it stays where it comes out.)
    let catch = Who::Monster(129);
    stage.put(catch, Vec2::new(160.0, 40.0), 1);
    stage.direct(Beat::Body(BodyBeat::FishLine { who: a, to: Some(catch), at: None, cast: false }), Vec2::ZERO);
    assert_eq!((stage.lines[&a], stage.cast[&catch].position, stage.cast[&catch].goal()), (catch, Vec2::new(160.0, 40.0), Vec2::new(160.0, 40.0)));
}

/// Two sent out the same way go in file, never one on the other, and are gone
/// once out; one comes in by a way of the place, from off it.
#[test]
fn who_goes_out_goes_in_file_and_is_gone() {
    let (a, b, c) = (Who::Monster(1), Who::Monster(2), Who::Monster(3));
    let mut stage = Stage::default();
    // (An open place with a wall across its right, a gap at the bottom.)
    let rows: Vec<String> = (0..20).map(|y| (0..40).map(|x| if x == 30 && y < 15 { '1' } else { '0' }).collect()).collect();
    stage.place.ground = Ground::parse(&rows.join("/"));
    stage.place.middle = Vec2::new(160.0, 80.0);
    stage.put(a, Vec2::new(160.0, 80.0), 1);
    stage.put(b, Vec2::new(160.0, 100.0), 1);
    assert!(stage.leave(a, &Exit::Side { right: true }, false, Vec2::ZERO) && stage.leave(b, &Exit::Side { right: true }, false, Vec2::ZERO));
    let mut nearest = f32::MAX;
    for _ in 0..800 {
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
        if let (Some(first), Some(second)) = (stage.cast.get(&a), stage.cast.get(&b)) {
            nearest = nearest.min(first.position.distance(second.position));
        }
    }
    assert!(!stage.cast.contains_key(&a) && !stage.cast.contains_key(&b), "gone");
    assert!(nearest >= 14.0, "{nearest}");
    // (In by a way of the place's own: from past its edge.)
    stage.place.entrances = vec![("south".to_string(), Vec2::new(100.0, 156.0))];
    stage.put(c, Vec2::new(100.0, 156.0), 1);
    let (start, door) = stage.way_in(c, Vec2::new(100.0, 80.0), &Exit::Named("south".to_string())).expect("a way in");
    assert!(start.y > 160.0 && door.y < 160.0, "{start} {door}");
    assert!(stage.exit_of(c, Vec2::new(100.0, 80.0), &Exit::Named("north".to_string())).is_none());
    assert!(stage.place.notes.iter().any(|note| note.text.contains("is not an entrance")));
}

/// Who is painted facing down only faces down again once it stands, unless a
/// line turned it; walking, it faces where it goes.
#[test]
fn who_is_painted_one_way_keeps_facing_it() {
    let a = Who::Monster(4001);
    let mut stage = Stage::default();
    stage.put(a, Vec2::new(100.0, 100.0), 3);
    stage.direct(Beat::Body(BodyBeat::Views { who: a, ways: vec![Direction::Down] }), Vec2::ZERO);
    run(&mut stage, 1);
    assert_eq!(stage.cast[&a].direction, 1);
    stage.turned_by_a_line(a, true);
    stage.direct(Beat::Body(BodyBeat::Turn { who: a, direction: Direction::Right }), Vec2::ZERO);
    run(&mut stage, 5);
    assert_eq!(stage.cast[&a].direction, 3, "turned by a line");
    stage.direct(Beat::Motion(MotionBeat::Go { who: a, at: glam::Vec2::new(60.0, 100.0).into(), frames: Frames(20.0), slide: false }), Vec2::ZERO);
    run(&mut stage, 10);
    assert_eq!(stage.cast[&a].direction, 7, "walking left");
    run(&mut stage, 20);
    assert_eq!(stage.cast[&a].direction, 1, "standing again");
    stage.direct(Beat::Body(BodyBeat::Views { who: a, ways: Vec::new() }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Turn { who: a, direction: Direction::Right }), Vec2::ZERO);
    run(&mut stage, 5);
    assert_eq!(stage.cast[&a].direction, 3, "all ways again");
}

#[test]
fn a_rider_goes_on_the_back_of_its_mount() {
    let mut stage = Stage::default();
    let (rider, mount) = (Who::Character(1), Who::Character(2));
    stage.put(rider, Vec2::new(10.0, 10.0), 1);
    stage.put(mount, Vec2::new(100.0, 80.0), 1);
    stage.direct(Beat::Body(BodyBeat::Ride { who: rider, on: Some(mount), high: 14.0, beside: 0.0 }), Vec2::ZERO);
    stage.direct(Beat::Motion(MotionBeat::Go { who: mount, at: glam::Vec2::new(160.0, 60.0).into(), frames: Frames(10.0), slide: true }), Vec2::ZERO);
    run(&mut stage, 20);
    let (on, under) = (&stage.cast[&rider], &stage.cast[&mount]);
    assert_eq!(under.position, Vec2::new(160.0, 60.0));
    // (Its mount facing us: behind its neck, drawn behind it, higher on it.)
    assert_eq!(on.position, under.shown_at() + Vec2::new(0.0, -1.0));
    assert!((on.air.height - 14.0 - super::thing::BEHIND_THE_NECK).abs() < 0.01, "{}", on.air.height);
    // (Off its back: left where it is, on the ground.)
    stage.direct(Beat::Body(BodyBeat::Ride { who: rider, on: None, high: 0.0, beside: 0.0 }), Vec2::ZERO);
    run(&mut stage, 1);
    assert_eq!(stage.cast[&rider].air.height, 0.0);
}

#[test]
fn a_way_to_face_turns_round_and_looks_its_way() {
    use super::character::{facing, turned};
    assert_eq!((turned(1, 4), turned(8, 1), turned(3, 0), turned(2, 15)), (5, 1, 3, 1));
    // (Down is towards us, right is right.)
    assert!((facing(1) - Vec2::new(0.0, 1.0)).length() < 1e-6 && (facing(3) - Vec2::new(1.0, 0.0)).length() < 1e-6);
}

#[test]
fn one_slowed_down_goes_in_its_own_time_and_the_others_in_the_scene_s() {
    let mut stage = Stage::default();
    let (slow, other) = (Who::Character(1), Who::Character(2));
    stage.put(slow, Vec2::ZERO, 1);
    stage.put(other, Vec2::new(0.0, 40.0), 1);
    stage.direct(Beat::Body(BodyBeat::Slow { who: slow, by: 0.25 }), Vec2::ZERO);
    stage.cast.get_mut(&slow).unwrap().looks.speed = 2.0;
    for who in [slow, other] {
        let to = stage.cast[&who].position + Vec2::new(40.0, 0.0);
        stage.cast.get_mut(&who).unwrap().go(to, 40.0, false);
    }
    run(&mut stage, 40);
    // (The other is there, the slowed one a quarter of the way; its pictures
    // go by at its own speed, slowed down too.)
    assert_eq!((stage.cast[&other].position.x, stage.cast[&other].busy()), (40.0, false));
    assert_eq!((stage.cast[&slow].position.x, stage.cast[&slow].busy(), stage.cast[&slow].picture_speed()), (10.0, true, 0.5));
    stage.direct(Beat::Body(BodyBeat::Slow { who: slow, by: 1.0 }), Vec2::ZERO);
    run(&mut stage, 30);
    assert_eq!((stage.cast[&slow].position.x, stage.cast[&slow].busy()), (40.0, false));
}

/// With no collisions, a walk goes straight to where it is sent, through a
/// wall; with them, round it.
#[test]
fn with_no_collisions_a_walk_goes_straight_through_a_wall() {
    let who = Who::Character(0);
    let walk = |collisions: bool| {
        let mut stage = Stage::default();
        stage.put(who, Vec2::ZERO, 1);
        stage.direct(Beat::Play(PlayBeat::Collisions(collisions)), Vec2::ZERO);
        stage.direct(Beat::Play(PlayBeat::Wall(Some([40.0, -40.0, 60.0, 40.0].into()))), Vec2::ZERO);
        stage.direct(Beat::Motion(MotionBeat::WalkTo { who, at: Vec2::new(100.0, 0.0).into(), run: false }), Vec2::ZERO);
        let mut through = false;
        for _ in 0..400 {
            stage.advance(1.0, 0.0, None);
            through |= walled(&stage.play.walls, stage.cast[&who].position);
        }
        assert!(stage.cast[&who].position.distance(Vec2::new(100.0, 0.0)) < 1.0);
        through
    };
    assert!(walk(false), "straight, through the wall");
    assert!(!walk(true), "round the wall");
}

/// With collisions, one put on a wall is put on the nearest free ground;
/// with none, just where it is written.
#[test]
fn a_place_on_a_wall_is_moved_only_with_collisions() {
    let who = Who::Character(0);
    let rows: Vec<String> = (0..20).map(|y| (0..40).map(|x| if (10..20).contains(&x) && (5..15).contains(&y) { '1' } else { '0' }).collect()).collect();
    let mut stage = Stage::default();
    stage.place.ground = Ground::parse(&rows.join("/"));
    let wanted = Vec2::new(120.0, 80.0);
    let on = stage.find(Some(who), &Spot::Written(wanted.into()), false, false);
    assert!(on.distance(wanted) > 8.0 && !stage.place.ground.as_ref().unwrap().blocked(on, 0), "moved off the wall: {on}");
    stage.direct(Beat::Play(PlayBeat::Collisions(false)), Vec2::ZERO);
    assert_eq!(stage.find(Some(who), &Spot::Written(wanted.into()), false, false), wanted);
}

/// With no collisions, a crowd takes all the room it needs, walls or not.
#[test]
fn with_no_collisions_a_crowd_is_placed_whole() {
    let mut stage = Stage::default();
    stage.place.ground = Ground::parse("000/000/000");
    let step = Vec2::new(16.0, 16.0);
    assert!(stage.spots_around(Vec2::new(12.0, 12.0), 120, step).len() < 120);
    stage.no_collisions = true;
    assert_eq!(stage.spots_around(Vec2::new(12.0, 12.0), 120, step).len(), 120);
}
