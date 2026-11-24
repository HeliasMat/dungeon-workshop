use super::*;
use crate::read_events;
use crate::read_scene;
use crate::vocabulary::SAMPLE;

/// How many of that kind were told.
fn told(watch: &Watch, kind: &str) -> usize {
    watch.remarks.iter().filter(|seen| seen.kind == kind).count()
}

/// Whether that kind was told as a remark (not a detail).
fn remarked(watch: &Watch, kind: &str) -> bool {
    watch.remarks.iter().any(|seen| seen.kind == kind && !seen.small)
}

/// Whether that kind was told, as a detail only.
fn detailed(watch: &Watch, kind: &str) -> bool {
    told(watch, kind) > 0 && !remarked(watch, kind)
}

#[test]
fn a_scene_is_played_through_and_who_speaks_out_of_sight_is_told() {
    let text = "event\t0\tE\t1\t1\nscene\t1\ta\nput\t0\t100\t100\tdown\nput\t10\t500\t100\tdown\ncamera\t@\t100\t100\nfadein\t1\nwaitfade\nsay\t0\t-\t Here!\nsaid\nsay\t10\t-\t Over there!\nsaid\nset\tn\t0\nwhile\tpresses < 3\nwait\t1\nendwhile\nask\tA|B\nanswer\t0\nset\tn\t1\nanswer\t1\nset\tn\t2\nend\n";
    let rehearsal = rehearse(read_events(read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line), &SAMPLE), &SAMPLE);
    assert!(rehearsal.problems.is_empty(), "{:?}", rehearsal.problems);
    assert_eq!(rehearsal.remarks.len(), 1, "{:?}", rehearsal.remarks);
    assert!(rehearsal.remarks[0].says("out of the picture"));
}

#[test]
fn what_is_seen_as_a_film_goes_by_is_told_with_its_line() {
    let (mut watch, mut stage, mut director) = (Watch::default(), Stage::default(), Director::default());
    let (a, b) = (Who::Monster(1), Who::Monster(2));
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    stage.put(b, Vec2::new(104.0, 100.0), 1);
    stage.place_walls([[90.0, 90.0, 110.0, 110.0]]);
    director.set_number(LINE.to_string(), 12);
    for _ in 0..60 {
        watch.look(&director, &stage, Vec2::new(100.0, 100.0), None);
    }
    assert!(watch.remarks.iter().any(|seen| seen.line == 12 && seen.kind == "wall" && !seen.small), "{:?}", watch.remarks);
    assert!(remarked(&watch, "on"), "{:?}", watch.remarks);
    // (Two who fight may touch: nothing more is told of them.)
    let before = told(&watch, "on");
    watch.remarks.clear();
    watch.told.clear();
    watch.fighters = stage.cast.keys().copied().collect();
    for _ in 0..60 {
        watch.look(&director, &stage, Vec2::new(100.0, 100.0), None);
    }
    assert!(before > 0 && told(&watch, "on") == 0, "{:?}", watch.remarks);
    watch.fighters.clear();
    // (A line gone at once; told once.)
    watch.look(&director, &stage, Vec2::ZERO, Some("A rather long line, truly."));
    watch.look(&director, &stage, Vec2::ZERO, None);
    assert!(remarked(&watch, "fast"), "{:?}", watch.remarks);
    // (Who speaks far from the view.)
    director.manner.last_speaker = Some(a);
    for _ in 0..50 {
        watch.look(&director, &stage, Vec2::new(900.0, 100.0), Some("[CS:N]A[CR]: Ici."));
    }
    assert!(remarked(&watch, "view"), "{:?}", watch.remarks);
    assert_eq!(told(&watch, "wall"), 2, "each told once");
}

/// The ground of a place: open, with a rock over the tiles 3 and 4 of its row 3.
fn rock_ground() -> crate::base::Ground {
    let rows: Vec<Vec<bool>> = (0..8).map(|y| (0..16).map(|x| y == 3 && (3..=4).contains(&x)).collect()).collect();
    crate::base::Ground::new(rows)
}

#[test]
fn the_ground_tells_a_body_on_a_rock_and_a_walk_through_it() {
    let (mut watch, mut stage, director) = (Watch::default(), Stage::default(), Director::default());
    stage.place.ground = Some(rock_ground());
    stage.place.spots.insert("rock".to_string(), Vec2::new(32.0, 28.0));
    stage.put(Who::Monster(1), Vec2::new(28.0, 28.0), 1);
    for _ in 0..60 {
        watch.look(&director, &stage, Vec2::new(28.0, 28.0), None);
    }
    assert!(remarked(&watch, "wall"), "{:?}", watch.remarks);
    // (Said by the name of the spot there.)
    assert!(watch.remarks.iter().any(|seen| seen.kind == "wall" && seen.text.contains("near “rock”")), "{:?}", watch.remarks);
    let (mut watch, mut stage) = (Watch::default(), Stage::default());
    stage.place.ground = Some(rock_ground());
    stage.put(Who::Monster(2), Vec2::new(4.0, 28.0), 1);
    stage.cast.get_mut(&Who::Monster(2)).expect("there").go(Vec2::new(100.0, 28.0), 80.0, false);
    for _ in 0..40 {
        stage.advance(1.0, 0.0, Some(Vec2::ZERO));
        watch.look(&director, &stage, Vec2::new(28.0, 28.0), None);
    }
    assert!(remarked(&watch, "through"), "{:?}", watch.remarks);
}

#[test]
fn a_walk_that_goes_nowhere_is_told() {
    let (mut watch, mut stage, director) = (Watch::default(), Stage::default(), Director::default());
    stage.put(Who::Monster(1), Vec2::new(40.0, 40.0), 1);
    stage.cast.get_mut(&Who::Monster(1)).expect("there").go(Vec2::new(100.0, 40.0), 500.0, false);
    for _ in 0..80 {
        watch.look(&director, &stage, Vec2::new(40.0, 40.0), None);
    }
    assert!(remarked(&watch, "stuck"), "{:?}", watch.remarks);
}

#[test]
fn a_crowd_among_fighters_and_a_fighter_left_out_are_told() {
    let (mut watch, mut stage, director) = (Watch::default(), Stage::default(), Director::default());
    let (a, b, c, d) = (Who::Monster(1), Who::Monster(2), Who::Monster(3), Who::Monster(4));
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    stage.put(b, Vec2::new(200.0, 100.0), 1);
    stage.put(c, Vec2::new(150.0, 100.0), 1);
    stage.put(d, Vec2::new(900.0, 100.0), 1);
    watch.fighters = vec![a, b, d];
    for _ in 0..60 {
        watch.look(&director, &stage, Vec2::new(150.0, 100.0), None);
    }
    assert!(detailed(&watch, "field") && detailed(&watch, "frame"), "{:?}", watch.remarks);
}

#[test]
fn who_speaks_hidden_a_thing_on_a_wall_and_one_out_of_frame_are_told() {
    let (mut watch, mut stage, mut director) = (Watch::default(), Stage::default(), Director::default());
    let (a, far, wall) = (Who::Monster(1), Who::Item { item: 70, copy: 0 }, Who::Drawn { picture: 0, copy: 0 });
    stage.place.ground = Some(rock_ground());
    stage.put(a, Vec2::new(40.0, 40.0), 1);
    stage.put(far, Vec2::new(900.0, 40.0), 1);
    stage.put(wall, Vec2::new(28.0, 28.0), 1);
    stage.direct(Beat::Body(BodyBeat::ThingName { who: far, name: "Oran Berry".to_string() }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::ThingName { who: wall, name: "letter".to_string() }), Vec2::ZERO);
    stage.direct(Beat::Body(BodyBeat::Opacity { who: a, to: 0.0, frames: Frames(0.0) }), Vec2::ZERO);
    director.manner.last_speaker = Some(a);
    for _ in 0..50 {
        watch.look(&director, &stage, Vec2::new(40.0, 40.0), Some("[CS:N]A[CR]: My oran berry!"));
    }
    assert!(remarked(&watch, "absent") && detailed(&watch, "thing") && detailed(&watch, "item"), "{:?}", watch.remarks);
}

#[test]
fn a_line_on_the_ground_is_told() {
    let (mut watch, mut stage, director) = (Watch::default(), Stage::default(), Director::default());
    let (a, on_sand) = (Who::Monster(1), Who::Drawn { picture: 0, copy: 0 });
    stage.place.ground = Some(rock_ground());
    stage.put(a, Vec2::new(4.0, 4.0), 1);
    stage.put(on_sand, Vec2::new(4.0, 40.0), 1);
    stage.lines.insert(a, on_sand);
    for _ in 0..12 {
        watch.look(&director, &stage, Vec2::new(4.0, 4.0), None);
    }
    assert!(detailed(&watch, "line"), "{:?}", watch.remarks);
    let mut watch = Watch::default();
    stage.cast.get_mut(&on_sand).expect("there").position = Vec2::new(28.0, 28.0);
    for _ in 0..12 {
        watch.look(&director, &stage, Vec2::new(4.0, 4.0), None);
    }
    assert_eq!(told(&watch, "line"), 0, "{:?}", watch.remarks);
}

/// Who is spoken of or to out of the frame; as details, a variant turned a
/// way it is not painted in, a big body cut by the frame, the crowd before
/// someone; a walker shuffling on the spot. A tint is no remark at all.
#[test]
fn what_the_films_showed_wrong_is_told() {
    let (mut watch, mut stage, mut director) = (Watch::default(), Stage::default(), Director::default());
    let (a, b, far, big, hunter, crowd) = (Who::Monster(1), Who::Monster(2), Who::Monster(3), Who::Monster(143), Who::Monster(4001), Who::Monster(183));
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    stage.put(b, Vec2::new(130.0, 100.0), 1);
    stage.put(far, Vec2::new(600.0, 100.0), 1);
    stage.put(hunter, Vec2::new(80.0, 100.0), 3);
    stage.put(crowd, Vec2::new(130.0, 110.0), 1);
    stage.put(big, Vec2::new(100.0, 250.0), 1);
    stage.view.bodies.insert(big, Vec2::new(64.0, 64.0));
    stage.direct(Beat::Body(BodyBeat::Views { who: hunter, ways: vec![Direction::Down] }), Vec2::ZERO);
    stage.light.tint.to = [200.0, 80.0, 50.0, 80.0];
    watch.names = HashMap::from([(a, "Pikachu".to_string()), (b, "Piplup".to_string()), (far, "Snorlax".to_string()), (crowd, "Marill".to_string())]);
    watch.crowd.insert(crowd);
    // (b asks a something; a answers and speaks of Snorlax, far off: to b,
    // of Snorlax.)
    director.manner.last_speaker = Some(b);
    for _ in 0..50 {
        watch.look(&director, &stage, Vec2::new(110.0, 100.0), Some("[CS:N]B[CR]: Pikachu, do you see him?"));
    }
    stage.put(b, Vec2::new(700.0, 100.0), 1);
    director.manner.last_speaker = Some(a);
    for _ in 0..140 {
        watch.look(&director, &stage, Vec2::new(110.0, 100.0), Some("[CS:N]A[CR]: Where is Snorlax?"));
    }
    let says = |words: &str| watch.remarks.iter().filter(|seen| seen.kind == "absent" && seen.text.contains(words)).count();
    assert_eq!(says("speaks of them"), 1, "{:?}", watch.remarks);
    assert_eq!(says("speaks to them"), 1, "{:?}", watch.remarks);
    assert!(remarked(&watch, "absent"), "{:?}", watch.remarks);
    assert!(detailed(&watch, "painted") && detailed(&watch, "cut") && detailed(&watch, "crowd"), "{:?}", watch.remarks);
    assert!(watch.remarks.iter().all(|seen| !seen.text.contains("veil")), "{:?}", watch.remarks);
    // (Aiming is its own, painted every way.)
    let mut watch = Watch::default();
    director.manner.held_gaze.insert(hunter);
    for _ in 0..60 {
        watch.look(&director, &stage, Vec2::new(110.0, 100.0), None);
    }
    assert_eq!(told(&watch, "painted"), 0, "{:?}", watch.remarks);
    // (A walker that gets no nearer, pushed back each frame.)
    let mut watch = Watch::default();
    let mut stage = Stage::default();
    stage.put(a, Vec2::new(40.0, 40.0), 1);
    for _ in 0..120 {
        stage.cast.get_mut(&a).expect("there").go(Vec2::new(200.0, 40.0), 500.0, false);
        stage.cast.get_mut(&a).expect("there").position.x = 40.0 + (watch.frame % 3) as f32 * 2.0;
        watch.look(&director, &stage, Vec2::new(40.0, 40.0), None);
    }
    assert!(remarked(&watch, "stuck"), "{:?}", watch.remarks);
}

#[test]
fn a_scene_that_never_ends_is_told() {
    let text = "event\t0\tE\t1\t1\nscene\t1\ta\nlabel\ttour\nwait\t1\ngoto\ttour\n";
    let rehearsal = rehearse_for(read_events(read_scene(text, None, &SAMPLE).lines.into_iter().map(|(_, line)| line), &SAMPLE), 3000, &SAMPLE);
    assert!(rehearsal.problems.iter().any(|problem| problem.says("does not end")), "{:?}", rehearsal.problems);
}

/// Who spoke before is not spoken to by a line that does not answer it.
#[test]
fn who_spoke_before_is_spoken_to_only_when_answered() {
    let (mut watch, mut stage, mut director) = (Watch::default(), Stage::default(), Director::default());
    let (a, b) = (Who::Monster(1), Who::Monster(2));
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    stage.put(b, Vec2::new(130.0, 100.0), 1);
    watch.names = HashMap::from([(a, "Pikachu".to_string()), (b, "Piplup".to_string())]);
    director.manner.last_speaker = Some(b);
    for _ in 0..50 {
        watch.look(&director, &stage, Vec2::new(110.0, 100.0), Some("[CS:N]B[CR]: The village greets you!"));
    }
    stage.put(b, Vec2::new(700.0, 100.0), 1);
    director.manner.last_speaker = Some(a);
    for _ in 0..140 {
        watch.look(&director, &stage, Vec2::new(110.0, 100.0), Some("[CS:N]A[CR]: And now, the party."));
    }
    assert_eq!(told(&watch, "absent"), 0, "{:?}", watch.remarks);
}

/// Someone there at once in sight, with nothing to say why, is told; in a
/// puff of smoke, or blinking out as he falls, he is not.
#[test]
fn who_is_there_or_gone_at_once_is_told() {
    let (mut watch, mut stage, director) = (Watch::default(), Stage::default(), Director::default());
    let (a, b) = (Who::Monster(1), Who::Monster(2));
    for _ in 0..70 {
        watch.look(&director, &stage, Vec2::new(100.0, 100.0), None);
    }
    stage.put(a, Vec2::new(100.0, 100.0), 1);
    watch.look(&director, &stage, Vec2::new(100.0, 100.0), None);
    assert!(remarked(&watch, "pops"), "{:?}", watch.remarks);
    let mut watch = Watch::default();
    for _ in 0..70 {
        watch.look(&director, &stage, Vec2::new(100.0, 100.0), None);
    }
    stage.light.particles.push(crate::stage::Particle {
        at: Vec2::new(120.0, 100.0),
        speed: Vec2::ZERO,
        fall: 0.0,
        left: 30.0,
        life: 30.0,
        colour: [200, 200, 200],
        blinks: false,
        sharp: false,
        ring: 0.0,
    });
    stage.put(b, Vec2::new(120.0, 100.0), 1);
    watch.look(&director, &stage, Vec2::new(100.0, 100.0), None);
    assert_eq!(told(&watch, "pops"), 0, "{:?}", watch.remarks);
}
