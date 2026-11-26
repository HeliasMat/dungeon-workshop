//! The tests of the Scene's sums: the picture fitted and looked at closer,
//! a pixel a point of the place and back, the grid's step, a measure, the
//! spots read, someone moved in the text.

use eframe::egui::{Rect, Vec2, pos2, vec2};

use super::handles::facing_dots;
use super::marks::Labels;
use super::marks::grid_step;
use super::mouse::measure_said;
use super::script::{
    Move, WAYS, dropped_in, faced, framed_at, free_spot, moment_of, moved_to, moves_of, nearest, number, spot_lines, way_of, with_cast, with_point, with_words,
};
use super::{Place, View, fitted_scale, wheel_look};

/// By whole pixels when it fits so, else smaller.
#[test]
fn the_picture_is_fitted() {
    assert_eq!(fitted_scale(vec2(530.0, 400.0), vec2(256.0, 192.0)), 2.0);
    assert_eq!(fitted_scale(vec2(128.0, 400.0), vec2(256.0, 192.0)), 0.5);
}

/// The wheel looks closer around the pointer: the point under it stays.
#[test]
fn the_wheel_looks_closer_around_the_pointer() {
    let (look, pan) = wheel_look(1.0, Vec2::ZERO, 1.0, pos2(110.0, 100.0), pos2(100.0, 100.0));
    assert_eq!(look, 1.25);
    assert!((pan.x - -2.5).abs() < 1e-4 && pan.y.abs() < 1e-4, "{pan:?}");
    assert_eq!(wheel_look(16.0, Vec2::ZERO, 1.0, pos2(0.0, 0.0), pos2(0.0, 0.0)).0, 16.0, "no closer than 16");
    assert_eq!(wheel_look(1.0, Vec2::ZERO, -1.0, pos2(0.0, 0.0), pos2(0.0, 0.0)).0, 0.8);
}

/// A pixel of the picture is a point of the place, and back; a box is on the panel.
#[test]
fn a_pixel_is_a_point_of_the_place() {
    let view = View { shown: Rect::from_min_size(pos2(10.0, 20.0), vec2(512.0, 384.0)), scale: 2.0, seen: Rect::EVERYTHING };
    let place = Place { min: view.shown.min, scale: 2.0, centre: [100.0, 50.0], close: 1.0, wide: 256.0, high: 192.0 };
    assert_eq!(place.to_place(pos2(10.0 + 256.0, 20.0 + 192.0)), [100.0, 50.0], "the middle");
    assert_eq!(place.to_screen([100.0, 50.0]), pos2(266.0, 212.0));
    assert_eq!(view.frame_of([1.0, 2.0, 3.0, 4.0]), Rect::from_min_max(pos2(12.0, 24.0), pos2(16.0, 28.0)));
    let bodies = vec![("riolu".to_string(), [0.0, 0.0, 10.0, 10.0]), ("piplup".to_string(), [5.0, 5.0, 20.0, 20.0])];
    assert_eq!(view.body_at(&bodies, pos2(24.0, 34.0)).map(String::as_str), Some("piplup"), "the one drawn last");
    assert_eq!(view.body_at(&bodies, pos2(12.0, 22.0)).map(String::as_str), Some("riolu"));
    assert_eq!(view.body_at(&bodies, pos2(500.0, 22.0)), None);
}

/// The grid doubles while its lines would be too close, up to 512.
#[test]
fn the_grid_steps_out() {
    assert_eq!(grid_step(1.0, 1.0), 8.0);
    assert_eq!(grid_step(0.25, 1.0), 32.0);
    assert_eq!(grid_step(0.0, 1.0), 512.0);
    assert_eq!(measure_said([0.0, 0.0], [0.0, 12.0]), "from 0 0 to 0 12: 12 px, 1.5 tiles");
}

/// The spots, and someone moved: their last line that puts them at a point, or a `put` after the scene's line.
#[test]
fn the_text_is_read_and_written_again() {
    assert_eq!(spot_lines("spot well 10 20\nspot bad x 2\nriolu put 1 2"), vec![("well".to_string(), 1, [10.0, 20.0])]);
    let text = "scene: beach riolu\n  riolu put 1 2\n  riolu walk to 5 6\n  riolu: hi";
    assert_eq!(moved_to(text, 4, "riolu", [7.0, 8.0]), Ok(("scene: beach riolu\n  riolu put 1 2\n  riolu walk to 7 8\n  riolu: hi".to_string(), 3)));
    assert_eq!(moved_to("scene: beach\n  a: hi", 2, "riolu", [7.0, 8.0]), Ok(("scene: beach\n  riolu put 7 8\n  a: hi".to_string(), 2)));
    assert!(moved_to("scene: beach\nriolu put here", 2, "riolu", [7.0, 8.0]).is_err());
}

/// Words of a line made others, the rest as it was (its spaces, its indent); numbers written short.
#[test]
fn words_are_made_others_in_place() {
    assert_eq!(with_words("a\n  riolu  walk to 1 2  # x", 2, 3, 2, "5 6"), "a\n  riolu  walk to 5 6  # x");
    assert_eq!(with_point("spot well 10 20", 1, 2, [3.0, 4.5]), "spot well 3 4.5");
    assert_eq!(with_words("one", 3, 0, 1, "two"), "one", "no such line: as it was");
    assert_eq!((number(2.0), number(1.256), number(0.5)), ("2".to_string(), "1.26".to_string(), "0.5".to_string()));
}

/// The moment of a line: from its `moment:` (or `scene:`) line to the next one.
#[test]
fn a_moment_has_its_lines() {
    let text = "scene: beach riolu\nriolu put 1 2\nmoment: a\nriolu: hi\nmoment: b\nriolu: bye";
    assert_eq!(moment_of(text, 2), (1, 3));
    assert_eq!(moment_of(text, 4), (3, 5));
    assert_eq!(moment_of(text, 6), (5, 7));
}

/// The camera dragged: the moment's framing line written again with the point, else a `frame … snap` added; its zoom.
#[test]
fn the_camera_frames_a_point() {
    let text = "scene: town riolu\nriolu put 1 2\n\nmoment: m\nglide to 300 212 in 1f\nriolu: hi";
    assert_eq!(framed_at(text, 6, [310.0, 200.0], None), ("scene: town riolu\nriolu put 1 2\n\nmoment: m\nglide to 310 200 in 1f\nriolu: hi".to_string(), 5));
    let named = "scene: town riolu\nframe riolu piplup follows\nriolu: hi";
    assert_eq!(framed_at(named, 3, [5.0, 6.0], None).0, "scene: town riolu\nframe 5 6\nriolu: hi", "a point stays: follows no longer");
    assert_eq!(framed_at("scene: town riolu\nframe riolu snap\nriolu: hi", 3, [5.0, 6.0], None).0, "scene: town riolu\nframe 5 6 snap\nriolu: hi");
    assert_eq!(framed_at("scene: town riolu\nglide riolu 2s", 2, [5.0, 6.0], None).0, "scene: town riolu\nglide 5 6 2s");
    assert_eq!(framed_at("scene: town riolu\n  riolu: hi", 2, [5.0, 6.0], None), ("scene: town riolu\n  frame 5 6 snap\n  riolu: hi".to_string(), 2));
    assert_eq!(framed_at("scene: town riolu\n  riolu: hi", 2, [5.0, 6.0], Some(1.5)).0, "scene: town riolu\n  frame 5 6 snap\n  zoom 1.5 1f\n  riolu: hi");
    assert_eq!(
        framed_at("scene: town riolu\nzoom 2 1f\nframe 1 1 snap\nriolu: hi", 4, [5.0, 6.0], Some(0.5)).0,
        "scene: town riolu\nzoom 0.5 1f\nframe 5 6 snap\nriolu: hi"
    );
}

/// Someone's way: from the screen's direction; their line's `facing` (or `look`) written again, added to a `put`, else a `look`.
#[test]
fn someone_faces_a_way() {
    assert_eq!((way_of(1.0, 0.0), way_of(0.0, 1.0), way_of(-1.0, -1.0), way_of(1.0, -0.9)), ("right", "down", "upleft", "upright"));
    for (at, dot) in facing_dots(eframe::egui::Pos2::ZERO).iter().enumerate() {
        assert_eq!(way_of(dot.x, dot.y), WAYS[at], "each dot its way");
    }
    let text = "scene: town riolu\nriolu put 1 2 facing up\nriolu: hi";
    assert_eq!(faced(text, 3, "riolu", "left"), ("scene: town riolu\nriolu put 1 2 facing left\nriolu: hi".to_string(), 2));
    assert_eq!(faced("scene: town riolu\nriolu put 1 2\nriolu: hi", 3, "riolu", "left").0, "scene: town riolu\nriolu put 1 2 facing left\nriolu: hi");
    assert_eq!(faced("scene: town riolu\nriolu look up", 2, "riolu", "down").0, "scene: town riolu\nriolu look down");
    assert_eq!(
        faced("scene: town riolu\nriolu jump\npiplup: hi", 3, "riolu", "down"),
        ("scene: town riolu\nriolu jump\npiplup: hi\nriolu look down".to_string(), 4)
    );
}

/// Someone's walks in the moment: their points in the order walked (`via` first), where they stood before.
#[test]
fn the_walks_of_a_moment() {
    let text =
        "scene: town riolu\nriolu put 1 2\nmoment: m\nriolu walk to 10 20\nriolu run to 30 40 via 5 5 6 6\npiplup walk to 0 0\nmoment: n\nriolu walk to 9 9";
    let (before, moves) = moves_of(text, 4, "riolu");
    assert_eq!(before, Some([1.0, 2.0]));
    assert_eq!(
        moves,
        vec![
            Move { line: 4, points: vec![(3, [10.0, 20.0])], runs: false },
            Move { line: 5, points: vec![(6, [5.0, 5.0]), (8, [6.0, 6.0]), (3, [30.0, 40.0])], runs: true },
        ]
    );
    assert_eq!(with_point(text, 5, 8, [7.0, 7.0]).split('\n').nth(4), Some("riolu run to 30 40 via 5 5 7 7"));
}

/// Names laid on the picture keep clear of each other, moved up first.
#[test]
fn names_do_not_cover_each_other() {
    let mut labels = Labels::default();
    let first = labels.spread(Rect::from_min_size(pos2(0.0, 100.0), vec2(40.0, 10.0)));
    let second = labels.spread(Rect::from_min_size(pos2(10.0, 100.0), vec2(40.0, 10.0)));
    assert_eq!(first.min, pos2(0.0, 100.0));
    assert!(!first.intersects(second), "{first:?} {second:?}");
    assert!(second.min.y < first.min.y, "moved up");
    assert_eq!(labels.spread(Rect::from_min_size(pos2(200.0, 0.0), vec2(5.0, 5.0))).min, pos2(200.0, 0.0), "free: where asked");
}

/// A Pokémon dropped on the Scene: one of the scene's cast, put there facing the nearest, after the line playing; one
/// already there, moved; no scene, said.
#[test]
fn a_pokemon_dropped_joins_the_cast_and_stands_there() {
    let text = "title: T\n\nscene: town pikachu munchlax\npikachu put 300 242 facing down\nmunchlax put 262 262\n\nmoment: m\npikachu: Hi!";
    let (made, line) = dropped_in(text, 5, "piplup", [320.0, 250.0], Some("pikachu")).unwrap();
    assert_eq!(
        made,
        "title: T\n\nscene: town pikachu munchlax piplup\npikachu put 300 242 facing down\nmunchlax put 262 262\npiplup put 320 250 facing pikachu\n\nmoment: m\npikachu: Hi!"
    );
    assert_eq!(line, 6);
    // (Dropped later: after the line playing, as far in.)
    let (made, line) = dropped_in("scene: beach riolu\n  riolu: hi", 2, "eevee", [10.0, 20.0], None).unwrap();
    assert_eq!((made.as_str(), line), ("scene: beach riolu eevee\n  riolu: hi\n  eevee put 10 20", 3));
    // (One of the cast already: their line written again.)
    let (made, _) = dropped_in(text, 8, "munchlax", [100.0, 110.0], Some("pikachu")).unwrap();
    assert!(made.contains("munchlax put 100 110\n") && made.contains("scene: town pikachu munchlax\n"), "{made}");
    assert!(dropped_in("title: T\npikachu: hi", 2, "eevee", [1.0, 2.0], None).is_err(), "no scene");
    assert_eq!(with_cast("scene: beach a # the beach", "b"), "scene: beach a b # the beach");
}

/// Where someone dropped stands: on free ground, off the others; whom they face, the nearest.
#[test]
fn someone_dropped_stands_on_free_ground() {
    assert_eq!(free_spot(None, &[], [10.4, 20.6]), [10.0, 21.0]);
    assert_eq!(free_spot(None, &[[10.0, 20.0]], [12.0, 21.0]), [-24.0, 21.0], "beside the one there");
    // (A wall on the left half, rows of 8 pixels: put on the nearest free tile.)
    let ground = engine::Ground::new(vec![vec![true, true, false, false, false, false, false, false]; 6]);
    let [x, y] = free_spot(Some(&ground), &[], [4.0, 12.0]);
    assert!(!ground.blocked(glam::Vec2::new(x, y), 1) && x >= 16.0, "{x} {y}");
    let feet: std::collections::BTreeMap<String, [f32; 2]> = [("a".to_string(), [0.0, 0.0]), ("b".to_string(), [50.0, 0.0])].into_iter().collect();
    assert_eq!(nearest(feet.iter(), [40.0, 0.0], "c").map(String::as_str), Some("b"));
    assert_eq!(nearest(feet.iter(), [40.0, 0.0], "b").map(String::as_str), Some("a"), "not themselves");
}
