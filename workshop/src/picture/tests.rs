//! The tests of the pictures: lines, shapes, fills, layers laid, a beginner's
//! shapes and animations, an expert's blends and colours.

use super::pixels::{ellipse_points, filled, line_points, rectangle_points, rotated};
use super::*;

#[test]
fn lines_shapes_fills_and_layers_are_made() {
    assert_eq!(line_points((0, 0), (3, 1)), vec![(0, 0), (1, 0), (2, 1), (3, 1)]);
    assert_eq!(rectangle_points((0, 0), (2, 2), false).len(), 8);
    assert_eq!(rectangle_points((0, 0), (2, 2), true).len(), 9);
    assert!(ellipse_points((0, 0), (8, 4), true).len() > ellipse_points((0, 0), (8, 4), false).len());
    let mut frame = RgbaImage::new(4, 4);
    for (x, y) in [(1, 0), (1, 1), (0, 2), (1, 2)] {
        frame.put_pixel(x, y, Rgba([0, 0, 0, 255]));
    }
    filled(&mut frame, (0, 0), Rgba([255, 0, 0, 255]), false, &|_| true);
    assert_eq!(frame.get_pixel(0, 1).0, [255, 0, 0, 255]);
    assert_eq!(frame.get_pixel(3, 3).0, [0, 0, 0, 0], "behind the wall: not filled");
    // (Two layers: the top one, half opaque, over the other.)
    let mut below = RgbaImage::new(1, 1);
    below.put_pixel(0, 0, Rgba([0, 0, 255, 255]));
    let mut above = RgbaImage::new(1, 1);
    above.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    let layers = [
        Layer { name: "a".into(), visible: true, opacity: 255, blend: 0, cels: vec![below] },
        Layer { name: "b".into(), visible: true, opacity: 128, blend: 0, cels: vec![above] },
    ];
    let made = composite(&layers, 0).get_pixel(0, 0).0;
    assert!(made[0] > 100 && made[2] > 100 && made[3] == 255, "{made:?}");
}

#[test]
fn a_beginner_shapes_and_animates_an_expert_blends_and_reduces() {
    let mut painting = Painting::new(&std::sync::Arc::new(crate::session::Session::for_tests()), "easy_test", 32, 32, 1);
    painting.begin(32);
    for kind in ["Round", "Box", "Star", "Drop"] {
        painting.layers[0].cels[0] = RgbaImage::new(32, 32);
        painting.shape(kind);
        let drawn = painting.layers[0].cels[0].pixels().filter(|pixel| pixel[3] > 0).count();
        assert!(drawn > 100 && drawn < 32 * 32, "{kind}: {drawn}");
    }
    painting.relief();
    for (kind, steps) in [("Bounce", 6), ("Blink", 4), ("Shake", 4), ("Grow", 6), ("Spin", 8)] {
        painting.animate(kind);
        assert_eq!((painting.count(), painting.ticks.len()), (steps, steps), "{kind}");
        painting.undo();
    }
    // (Multiplied: white over blue stays blue.)
    let mut below = RgbaImage::new(1, 1);
    below.put_pixel(0, 0, Rgba([0, 0, 255, 255]));
    let mut above = RgbaImage::new(1, 1);
    above.put_pixel(0, 0, Rgba([255, 255, 255, 255]));
    let layers = [
        Layer { name: "a".into(), visible: true, opacity: 255, blend: 0, cels: vec![below] },
        Layer { name: "b".into(), visible: true, opacity: 255, blend: 1, cels: vec![above] },
    ];
    assert_eq!(composite(&layers, 0).get_pixel(0, 0).0, [0, 0, 255, 255]);
    // (Fewer colours: no more than asked.)
    let mut many = RgbaImage::new(16, 16);
    for (x, y, pixel) in many.enumerate_pixels_mut() {
        *pixel = Rgba([(x * 16) as u8, (y * 16) as u8, 80, 255]);
    }
    painting.layers[0].cels = vec![many];
    painting.colours_at_most = 4;
    painting.fewer_colours();
    let kinds: std::collections::HashSet<_> = painting.layers[0].cels[0].pixels().map(|pixel| pixel.0).collect();
    assert!(kinds.len() <= 4, "{}", kinds.len());
    // (Turned a half turn and back: the same.)
    let picture = painting.layers[0].cels[0].clone();
    assert_eq!(rotated(&rotated(&picture, 180.0, 1.0), 180.0, 1.0), picture);
}

/// The canvas's sums: a point a pixel, the wheel around the pointer, a shape held square with Shift.
#[test]
fn the_canvas_reads_the_pointer() {
    use super::canvas::wheel_zoom;
    use super::pointer::{held_square, pixel_at};
    use eframe::egui::{Vec2, pos2};
    assert_eq!(pixel_at(pos2(10.0, 10.0), 4.0, pos2(17.9, 9.0)), (1, -1));
    let (zoom, pan) = wheel_zoom(4.0, Vec2::ZERO, 1.0, pos2(100.0, 50.0), pos2(100.0, 50.0));
    assert_eq!((zoom, pan), (5.0, Vec2::ZERO), "around the pointer at the middle: no shift");
    assert_eq!(wheel_zoom(1.0, Vec2::ZERO, -1.0, pos2(0.0, 0.0), pos2(0.0, 0.0)).0, 1.0, "no farther than 1");
    assert_eq!(held_square((0, 0), (5, 2), false), (5, 5), "a square");
    assert_eq!(held_square((0, 0), (-5, 2), false), (-5, 5));
    assert_eq!(held_square((0, 0), (5, 1), true), (5, 0), "a line flat");
    assert_eq!(held_square((0, 0), (1, 5), true), (0, 5), "upright");
    assert_eq!(held_square((0, 0), (4, 3), true), (4, 4), "at 45°");
}

/// The beginner's parts lie round the canvas: the tools left, the colours right, the rows under.
#[test]
fn the_simple_painting_lays_out_its_parts() {
    use super::simple::Rooms;
    use eframe::egui::{Rect, pos2};
    let rooms = Rooms::of(Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 600.0)));
    assert_eq!(rooms.tools, Rect::from_min_max(pos2(10.0, 0.0), pos2(160.0, 470.0)));
    assert_eq!(rooms.side, Rect::from_min_max(pos2(740.0, 0.0), pos2(992.0, 470.0)));
    assert_eq!(rooms.canvas, Rect::from_min_max(pos2(170.0, 0.0), pos2(730.0, 470.0)));
    assert_eq!(rooms.bottom, Rect::from_min_max(pos2(10.0, 476.0), pos2(992.0, 598.0)));
}

/// A colour written in hex and read back; what is not one.
#[test]
fn a_colour_is_written_in_hex() {
    use super::panels::{colour_of_hex, hex_of};
    use eframe::egui::Color32;
    assert_eq!(hex_of(Color32::from_rgb(0x18, 0xA0, 0xFF)), "18A0FF");
    assert_eq!(colour_of_hex(" #18a0ff"), Some(Color32::from_rgb(0x18, 0xA0, 0xFF)));
    assert_eq!((colour_of_hex("18A0F"), colour_of_hex("GG0000")), (None, None));
}
