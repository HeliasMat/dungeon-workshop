//! What all screens share: the colour of window frames, the blinking of
//! menus' cursors and the sounds menus make.

use shell::Keys;
use stage_view::ui::{self, Canvas};

use rules::run::Run;

use crate::Frame;

/// The colour of the frames of windows: green before the hero is chosen,
/// pink in a side story, blue otherwise.
pub fn frame_color(episode: bool, hero: u16) -> ui::Frame {
    match (episode, hero) {
        (true, _) => ui::Frame::Pink,
        (false, 0) => ui::Frame::Green,
        (false, _) => ui::Frame::Blue,
    }
}

/// How many frames ago a cursor last moved or appeared, and where the
/// cursors were on the last picture drawn. The cursor shows 16 frames out
/// of 32, starting again shown each time it moves.
#[derive(Default)]
pub struct Blink {
    frames: u32,
    places: Vec<(i32, i32)>,
    steered: u8,
}

const MOVED: u16 = 0x3F01;
const CHOSEN: u16 = 0x3F02;
const LEFT: u16 = 0x3F03;

/// Readies the canvas of a frame: its frames' colour, the cursor's blink.
pub fn prepare(show: &mut Frame, run: &Run) {
    let canvas: &mut Canvas = &mut show.canvas;
    canvas.cursor_hidden = (show.blink.frames / 16) % 2 == 1;
    canvas.cursors_before = show.blink.places.clone();
    canvas.frame = frame_color(run.team().episode, run.team().species.0);
    canvas.frame_type = run.options().window as usize;
    show.blink.frames += 1;
}

/// Takes note of the cursors just drawn, and makes menus' sounds: one as a
/// cursor moves, one as a choice is made and one as a menu is left.
pub fn settle(show: &mut Frame, keys: &Keys) {
    let blink = &mut show.blink;
    if show.canvas.cursor_moved() {
        blink.frames = 0;
    }
    // (The picture is drawn after the keys are read: a direction counts for
    // a few frames.)
    blink.steered = if keys.up || keys.down || keys.left || keys.right { 3 } else { blink.steered.saturating_sub(1) };
    let sound = if blink.places.is_empty() {
        None
    } else if keys.enter {
        Some(CHOSEN)
    } else if keys.escape {
        Some(LEFT)
    } else if !show.canvas.cursors.is_empty() && blink.places != show.canvas.cursors && blink.steered > 0 {
        Some(MOVED)
    } else {
        None
    };
    if let (Some(sound), Some(audio)) = (sound, &show.audio) {
        audio.sound(sound, 1.0, 1.0);
    }
    if blink.places != show.canvas.cursors {
        blink.places = show.canvas.cursors.clone();
    }
}
