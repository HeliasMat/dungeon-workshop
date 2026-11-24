//! The tests of the Scene's sums: the picture fitted and looked at closer,
//! a pixel a point of the place and back, the grid's step, a measure, the
//! spots read, someone moved in the text.

use eframe::egui::{Rect, Vec2, pos2, vec2};

use super::marks::grid_step;
use super::mouse::measure_said;
use super::script::{moved_to, spots_of};
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
    assert_eq!(spots_of("spot well 10 20\nspot bad x 2\nriolu put 1 2"), vec![("well".to_string(), [10.0, 20.0])]);
    let text = "scene: beach riolu\n  riolu put 1 2\n  riolu walk to 5 6\n  riolu: hi";
    assert_eq!(moved_to(text, 4, "riolu", [7.0, 8.0]), Ok(("scene: beach riolu\n  riolu put 1 2\n  riolu walk to 7 8\n  riolu: hi".to_string(), 3)));
    assert_eq!(moved_to("scene: beach\n  a: hi", 2, "riolu", [7.0, 8.0]), Ok(("scene: beach\n  riolu put 7 8\n  a: hi".to_string(), 2)));
    assert!(moved_to("scene: beach\nriolu put here", 2, "riolu", [7.0, 8.0]).is_err());
}
