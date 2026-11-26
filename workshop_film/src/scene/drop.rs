//! An asset of the Assets panel dropped on the picture: a Pokémon put
//! there (one of the scene's cast, on free ground, facing the nearest), a
//! thing put down there (or held by who it is dropped on), anything else
//! written at the line playing for who it is dropped on.

use eframe::egui::{self, Color32, Stroke, vec2};
use workshop_base::asset::Asset;
use workshop_base::theme;
use workshop_script::Scripts;

use super::super::Films;
use super::script::{dropped_in, free_spot, nearest};
use super::{Place, View};

/// While an asset is dragged over the picture: where it would go; let go, written.
pub(super) fn dropping(ui: &egui::Ui, films: &mut Films, scripts: &mut Scripts, view: &View) {
    let Some(asset) = egui::DragAndDrop::payload::<Asset>(ui.ctx()) else {
        return;
    };
    let Some(at) = ui.input(|input| input.pointer.latest_pos()).filter(|at| view.seen.contains(*at)) else {
        return;
    };
    let Some(drawn) = films.drawn.as_ref() else {
        return;
    };
    let place = Place::of(view, drawn);
    let point = place.to_place(at).map(f32::round);
    let under = view.body_at(&drawn.bodies, at).cloned();
    let painter = ui.painter_at(view.seen);
    // (Where it goes: a Pokémon on free ground, its spot ringed; anything else, what it writes.)
    let said = match &*asset {
        Asset::Pokemon { word } => {
            let others: Vec<[f32; 2]> = drawn.feet.iter().filter(|(name, _)| *name != word).map(|(_, feet)| *feet).collect();
            let spot = free_spot(drawn.ground.as_ref(), &others, point);
            let shown = place.to_screen(spot);
            painter.circle_stroke(shown, 10.0, Stroke::new(2.0_f32, theme::FILM));
            painter.line_segment([at, shown], Stroke::new(1.0_f32, Color32::from_white_alpha(120)));
            let facing = nearest(drawn.feet.iter(), spot, word).map(|other| format!(" facing {other}")).unwrap_or_default();
            format!("{word} put {} {}{facing}", spot[0], spot[1])
        }
        Asset::Thing { word, held: true } if under.is_some() => format!("{} hold {word}", under.clone().unwrap_or_default()),
        Asset::Thing { word, .. } => format!("put object {word} {} {}", point[0], point[1]),
        other => match &under {
            Some(who) => format!("{} (for {who})", other.said()),
            None => other.said(),
        },
    };
    if let Some(who) = &under
        && !matches!(&*asset, Asset::Pokemon { .. })
        && let Some((_, frame)) = drawn.bodies.iter().find(|(name, _)| name == who)
    {
        painter.rect_stroke(view.frame_of(*frame), 2.0, Stroke::new(2.0_f32, theme::FILM), egui::StrokeKind::Outside);
    }
    workshop_decor::tag(&painter, at + vec2(14.0, -6.0), &said, theme::FILM);
    ui.ctx().request_repaint();
    if !ui.input(|input| input.pointer.any_released()) {
        return;
    }
    let Some(asset) = egui::DragAndDrop::take_payload::<Asset>(ui.ctx()) else {
        return;
    };
    let Some(open) = scripts.open.as_mut() else {
        return;
    };
    let playing = films.line_here(open.cursor_line());
    films.said = Some(match &*asset {
        Asset::Pokemon { word } => {
            let Some(drawn) = films.drawn.as_ref() else { return };
            let others: Vec<[f32; 2]> = drawn.feet.iter().filter(|(name, _)| *name != word).map(|(_, feet)| *feet).collect();
            let spot = free_spot(drawn.ground.as_ref(), &others, point);
            let facing = nearest(drawn.feet.iter(), spot, word).cloned();
            match dropped_in(&open.text, playing, word, spot, facing.as_deref()) {
                Ok((made, line)) => {
                    open.set_text(made);
                    format!("line {line}: {word} at {} {}", spot[0], spot[1])
                }
                Err(why) => why,
            }
        }
        // (A face, a gesture dropped on someone: theirs.)
        Asset::Face { face, .. } if under.is_some() => {
            open.put_asset(&Asset::Face { who: under.clone().unwrap_or_default(), face: face.clone() }, Some(playing), None, Some(point))
        }
        Asset::Gesture { words, .. } if under.is_some() => {
            open.put_asset(&Asset::Gesture { who: under.clone().unwrap_or_default(), words: words.clone() }, Some(playing), None, Some(point))
        }
        asset => {
            let who = under.as_deref().or(films.chosen());
            let who = if matches!(asset, Asset::Thing { .. }) { under.as_deref() } else { who }.map(str::to_string);
            open.put_asset(asset, Some(playing), who.as_deref(), Some(point))
        }
    });
}
