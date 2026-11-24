//! The hint bar along the top of the Scene: what is on (the stamp in hand,
//! snapping, the walls shown, the layer pieces go on), then the keys of the
//! tool in hand as it is now. Small, see-through, never in the way: no mode
//! without its mark.

use eframe::egui::{self, Color32, Rect, pos2, vec2};

use crate::decors::Decors;
use crate::terrains::Tool;
use workshop_base::theme;

/// What the hints depend on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Now {
    pub tool: Tool,
    pub stamping: bool,
    pub chosen: bool,
}

/// The keys of the tool in hand, as it is now: each its key, what it does.
pub(super) fn hints(now: Now) -> &'static [(&'static str, &'static str)] {
    match now.tool {
        Tool::Choose if now.stamping => &[("click", "lay it"), ("Alt", "snap the other way"), ("Esc / right click", "put the stamp down")],
        Tool::Choose if now.chosen => &[
            ("arrows", "nudge (Shift: 8)"),
            ("H", "flip"),
            ("[ ]", "back, forward"),
            ("L", "layer"),
            ("Ctrl+D", "duplicate"),
            ("Del", "remove"),
            ("F", "frame"),
            ("Esc", "let go"),
        ],
        Tool::Choose => &[
            ("click", "choose"),
            ("drag", "a rectangle"),
            ("a card", "click: stamp · drag: drop here"),
            ("Ctrl+F or /", "search"),
            ("S", "snap"),
            ("C", "walls"),
            ("Space+drag", "pan"),
        ],
        Tool::Terrain => &[("drag", "paint"), ("right drag", "erase"), ("V", "back to choosing")],
        Tool::Strew => &[("drag", "scatter the piece chosen in the Library"), ("V", "back to choosing")],
        Tool::Walls => &[("drag", "wall"), ("right drag", "free"), ("V", "back to choosing")],
        Tool::Walk => &[("arrows / WASD", "walk"), ("V", "back to choosing")],
    }
}

/// What is on, each a word and its colour: the stamp, snapping, the walls shown, the layer pieces go on.
pub(super) fn chips(decors: &Decors, step: f32) -> Vec<(String, Color32)> {
    let mut chips = Vec::new();
    if let Some(stamp) = &decors.stamp {
        let laid = if stamp.laid > 0 { format!(" ×{}", stamp.laid) } else { String::new() };
        chips.push((format!("STAMP · {}{laid}", decors.shown_name(&stamp.name)), theme::DECOR));
    }
    if decors.snap {
        chips.push((format!("SNAP {step:.0}"), theme::GAME));
    }
    if !decors.is_hidden("walls") {
        chips.push(("WALLS".to_string(), theme::WRONG));
    }
    chips.push((format!("lays on {}", crate::placing::layer_name(decors.active_layer)), theme::WEAK));
    chips
}

/// The bar drawn along the top of the room.
pub(super) fn bar(painter: &egui::Painter, room: Rect, decors: &Decors, now: Now, step: f32) {
    let font = egui::FontId::proportional(theme::SMALL);
    let strip = Rect::from_min_size(room.min, vec2(room.width(), 20.0));
    painter.rect_filled(strip, 0.0, Color32::from_black_alpha(150));
    let painter = painter.with_clip_rect(strip);
    let mut x = strip.min.x + 6.0;
    for (word, colour) in chips(decors, step) {
        let galley = painter.layout_no_wrap(word, font.clone(), colour);
        let chip = Rect::from_min_size(pos2(x, strip.min.y + 3.0), galley.size() + vec2(8.0, 0.0)).with_max_y(strip.max.y - 3.0);
        painter.rect_stroke(chip, 3.0, egui::Stroke::new(1.0_f32, colour.gamma_multiply(0.8)), egui::StrokeKind::Inside);
        painter.galley(pos2(chip.min.x + 4.0, strip.center().y - galley.size().y / 2.0), galley, colour);
        x = chip.max.x + 6.0;
    }
    x += 6.0;
    for (key, what) in hints(now) {
        let key_galley = painter.layout_no_wrap(key.to_string(), font.clone(), theme::STRONG);
        let what_galley = painter.layout_no_wrap(what.to_string(), font.clone(), theme::WEAK);
        let y = strip.center().y - key_galley.size().y / 2.0;
        let key_wide = key_galley.size().x;
        let what_wide = what_galley.size().x;
        painter.galley(pos2(x, y), key_galley, theme::STRONG);
        painter.galley(pos2(x + key_wide + 4.0, y), what_galley, theme::WEAK);
        x += key_wide + what_wide + 16.0;
    }
}

#[cfg(test)]
mod tests {
    use super::{Now, hints};
    use crate::terrains::Tool;

    /// Each tool has its keys; the stamp says how to stop it; what is chosen, how to move it.
    #[test]
    fn each_tool_says_its_keys() {
        for tool in [Tool::Choose, Tool::Terrain, Tool::Strew, Tool::Walls, Tool::Walk] {
            assert!(!hints(Now { tool, stamping: false, chosen: false }).is_empty());
        }
        let stamping = hints(Now { tool: Tool::Choose, stamping: true, chosen: true });
        assert!(stamping.iter().any(|(key, _)| key.contains("Esc")));
        let chosen = hints(Now { tool: Tool::Choose, stamping: false, chosen: true });
        assert!(chosen.iter().any(|(key, _)| *key == "H") && chosen.iter().any(|(key, _)| *key == "arrows"));
    }
}
