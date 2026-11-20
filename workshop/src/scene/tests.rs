//! The tests of the Scene: the view (fitting, closer where the pointer is,
//! framed), a pattern's tiles, the gaps measured, the guides, a click's choice.

use eframe::egui::{Rect, pos2, vec2};

use super::choosing::{Snap, chosen_on_click, layered, nearest_guide, new_spot_name};
use super::drawn::{ground_area, tiles_over};
use super::marks::{gaps_between, ruler_step};
use super::moving::{framed_on, map_size, zoomed, zoomed_at};
use super::{View, fit_scale, grid_step};

/// The view: fitted in whole steps when larger than the decor's pixels; a point there and back; closer
/// where the pointer is (that point kept under it); framed on a box; its closeness kept within bounds.
#[test]
fn the_view_fits_zooms_and_frames() {
    assert_eq!(fit_scale(vec2(1000.0, 500.0), vec2(200.0, 100.0)), 4.0);
    assert!((fit_scale(vec2(100.0, 100.0), vec2(200.0, 100.0)) - 0.48).abs() < 1e-6);
    let view = View::of(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 500.0)), vec2(200.0, 100.0), None, vec2(0.0, 0.0));
    assert_eq!(view.scale, 4.0);
    assert_eq!(view.to_place(view.to_screen([10.0, 20.0])), [10.0, 20.0]);
    let at = pos2(300.0, 200.0);
    let (scale, pan) = zoomed_at(view, at, 2.0);
    let closer = View::of(view.rect, view.size, scale, pan);
    let [u, v] = closer.to_place(at);
    let [was_u, was_v] = view.to_place(at);
    assert!((u - was_u).abs() < 1e-3 && (v - was_v).abs() < 1e-3, "the point under the pointer stays");
    assert_eq!(zoomed(view, (None, vec2(10.0, 0.0)), 100.0), (Some(16.0), vec2(40.0, 0.0)), "no closer than 16, the pan in step");
    let (frame, _) = framed_on(vec2(600.0, 600.0), vec2(200.0, 100.0), Rect::from_min_size(pos2(0.0, 0.0), vec2(50.0, 50.0)));
    assert_eq!(frame, Some(4.0));
    assert_eq!(map_size(200.0, vec2(400.0, 100.0)), vec2(200.0, 50.0));
    assert_eq!(map_size(200.0, vec2(100.0, 400.0)), vec2(50.0, 200.0));
}

/// A ground's zone and the tiles laid over it; the grid's step; the rulers' step.
#[test]
fn patterns_grids_and_rulers() {
    assert_eq!(ground_area(&["zone", "0", "0", "16", "8"], vec2(64.0, 64.0)), Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(16.0, 8.0))));
    assert_eq!(ground_area(&["zone", "0", "x", "16", "8"], vec2(64.0, 64.0)), None);
    assert_eq!(ground_area(&[], vec2(64.0, 32.0)), Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(64.0, 32.0))));
    let tiles = tiles_over(Rect::from_min_max(pos2(0.0, 0.0), pos2(20.0, 8.0)), vec2(8.0, 8.0));
    assert_eq!(tiles.iter().map(|tile| tile.min.x).collect::<Vec<_>>(), vec![0.0, 8.0, 16.0], "the last one over the edge");
    assert_eq!(grid_step("size 64 64\ngrid 16\n"), 16.0);
    assert_eq!(grid_step("size 64 64\n"), 8.0);
    assert_eq!(ruler_step(1.0), 64.0);
    assert_eq!(ruler_step(8.0), 8.0);
}

/// Measured with Alt: the gaps across and down; the guides a box is put on; a click's choice; snapping; a spot's name.
#[test]
fn gaps_guides_and_choices() {
    let from = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 10.0));
    let gaps = gaps_between(from, Rect::from_min_max(pos2(30.0, 20.0), pos2(40.0, 30.0)));
    assert_eq!(gaps.iter().map(|(_, _, gap)| *gap).collect::<Vec<_>>(), vec![20.0, 10.0]);
    assert!(gaps_between(from, Rect::from_min_max(pos2(5.0, 5.0), pos2(8.0, 8.0))).is_empty(), "inside: no gap");
    let others = [Rect::from_min_max(pos2(50.0, 0.0), pos2(60.0, 10.0))];
    assert_eq!(nearest_guide(Rect::from_min_max(pos2(38.0, 30.0), pos2(48.0, 40.0)), &others, 3.0, true), Some((2.0, 50.0)), "its right edge on their left");
    assert_eq!(nearest_guide(Rect::from_min_max(pos2(0.0, 30.0), pos2(10.0, 40.0)), &others, 3.0, true), None);
    assert_eq!(chosen_on_click(&[(1, 0)], (2, 0), false), vec![(2, 0)]);
    assert_eq!(chosen_on_click(&[(1, 0)], (2, 0), true), vec![(2, 0), (1, 0)]);
    assert_eq!(chosen_on_click(&[(1, 0), (2, 0)], (1, 0), true), vec![(2, 0)]);
    assert_eq!(Snap { step: 8.0, on: false }.of([13.4, 4.6]), [13.0, 5.0]);
    assert_eq!(Snap { step: 8.0, on: true }.of([13.4, 4.6]), [16.0, 8.0]);
    assert_eq!(new_spot_name(&["spot1".to_string()]), "spot2");
}

/// L: the pieces chosen on the next layer, as the first of them goes; someone who lives there left as he is.
#[test]
fn the_layer_key_goes_round() {
    let text = "decor: a 64 64\nput e1_1 8 8\nput e1_2 16 16 layer over\n[life]\nresident pikachu 50 60 down\n";
    let once = layered(text, &[2, 3]);
    assert_eq!(once, "decor: a 64 64\nput e1_1 8 8 layer over\nput e1_2 16 16 layer over\n[life]\nresident pikachu 50 60 down\n");
    let twice = layered(&once, &[2]);
    assert!(twice.contains("put e1_1 8 8 layer ground\n"));
    assert_eq!(layered(&layered(&twice, &[2]), &[2, 5]).lines().nth(4), Some("resident pikachu 50 60 down"));
}
