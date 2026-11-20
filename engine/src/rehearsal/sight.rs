//! Who speaks out of sight as its line shows: off the screen, or behind
//! the dialogue box.

use super::*;

/// Why who speaks is not seen as the line shows, if it is not: off the
/// screen, or behind the dialogue box. Nothing for who is not on the
/// stage, hidden, or a dark screen; nor before the scene has a view.
pub(super) fn out_of_sight(stage: &Stage, focus: Option<Vec2>, who: Who, text: &str) -> Option<diagnostics::Said> {
    let focus = focus?;
    let actor = stage.cast.get(&who).filter(|actor| actor.looks.opacity > 0.1)?;
    if stage.light.fade.level > 0.5 {
        return None;
    }
    let zoom = stage.view.closeness();
    let on_screen = (actor.position - focus) * zoom;
    let words: String = without_tags(text).chars().filter(|c| *c != '\n').take(30).collect();
    if on_screen.x.abs() > HALF.x + 8.0 || on_screen.y < -HALF.y + HEAD || on_screen.y > HALF.y + 8.0 {
        let side = super::side(actor.position, focus, zoom);
        return Some(told!("{} speaks out of the picture, {} (“{}”) → camera on {}", who, side, words.trim(), who));
    }
    // (Its feet low enough that the box hides most of it, wherever the box is.)
    let hidden = match stage.words.box_top {
        true => on_screen.y < -HALF.y + crate::stage::BOX_HIGH + HEAD_IN_THE_BOX,
        false => on_screen.y > HALF.y - crate::stage::BOX_HIGH + FEET_IN_THE_BOX,
    };
    if hidden {
        return Some(told!("{} speaks hidden by the dialogue box (“{}”) → box top, or move them up in the picture", who, words.trim()));
    }
    None
}
