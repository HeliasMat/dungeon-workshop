use super::marks::{draw_lights, draw_spotlight};
use super::*;

/// The alpha of a pixel of the canvas's lower screen, the view's middle at (128, 96).
fn alpha(canvas: &Canvas, x: usize, y: usize) -> u8 {
    canvas.data[((ui::SCREEN_HEIGHT + y) * ui::SCREEN_WIDTH + x) * 4 + 3]
}

/// Points of the scene on the canvas: the scene's (0, 0) at the middle of its lower screen.
fn on_screen(at: Vec2) -> (i32, i32) {
    (at.x as i32 + 128, at.y as i32 + 96 + ui::SCREEN_HEIGHT as i32)
}

#[test]
fn a_spotlight_leaves_its_middle_lit_and_all_round_dark() {
    let spotlight = Spotlight { at: Vec2::ZERO, radius: 30.0, shown: 1.0, ..Spotlight::default() };
    let mut canvas = Canvas::default();
    draw_spotlight(&mut canvas, &spotlight, ui::SCREEN_HEIGHT as i32, 1.0, &on_screen);
    assert_eq!(alpha(&canvas, 128, 96), 0, "its middle");
    assert!(alpha(&canvas, 2, 2) > 200 && alpha(&canvas, 250, 180) > 200, "round it");
}

#[test]
fn several_lights_each_leave_their_hole() {
    let mut stage = Stage::default();
    stage.light.spotlight = Spotlight { at: Vec2::new(-80.0, 0.0), radius: 20.0, shown: 1.0, ..Spotlight::default() };
    stage.light.lights.push(Lit { on: Cam::at(Vec2::new(80.0, 0.0)), radius: 20.0 });
    let mut canvas = Canvas::default();
    draw_lights(&mut canvas, &stage, ui::SCREEN_HEIGHT as i32, 1.0, &on_screen);
    assert_eq!((alpha(&canvas, 48, 96), alpha(&canvas, 208, 96)), (0, 0), "both holes");
    assert!(alpha(&canvas, 128, 96) > 200, "dark between them");
    // (Their edge falls one pixel in two, never a soft shade.)
    let edge: Vec<u8> = (68..76).map(|x| alpha(&canvas, x, 96)).collect();
    assert!(edge.iter().all(|alpha| *alpha == 0 || *alpha > 200) && edge.contains(&0) && edge.iter().any(|alpha| *alpha > 200), "{edge:?}");
}
