//! The State panel: the film at the frame shown, who is where.

use eframe::egui::{self, Rect, RichText, Sense, vec2};

use super::Films;
use super::playing::clock;
use workshop_base::theme;
use workshop_script::Scripts;

/// The State: the scene at the moment shown, as `scene_player state` tells
/// it (`preview::state`): the camera, the bars, the line in the box, each
/// one on stage (a click chooses them, as on the Scene), the objects.
pub fn state(ui: &mut egui::Ui, films: &mut Films, scripts: &mut Scripts) {
    if scripts.open.is_none() {
        ui.label(theme::weak("No film open."));
        return;
    }
    let Some(drawn) = &films.drawn else {
        ui.label(theme::weak(films.failed.clone().unwrap_or_else(|| "the film is playing…".to_string())));
        return;
    };
    let moment = drawn.state.clone();
    let frame = drawn.frame;
    let line = films.stepped_line().or(films.playing_line());
    let mut chosen: Option<String> = None;
    let mut go_line: Option<usize> = None;
    let mut lit: Option<String> = None;
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(theme::text(format!("At {}", clock(frame as f64)), theme::TITLE, theme::STRONG).strong());
            if let Some(line) = line {
                let said = if films.stepped_line().is_some() { format!("stopped at line {line}") } else { format!("line {line}") };
                if ui.link(theme::text(said, theme::SMALL, theme::FILM)).on_hover_text("this line in the Script").clicked() {
                    go_line = Some(line);
                }
            }
        });
        ui.add_space(4.0);
        crate::decors::section(ui, "Camera");
        ui.add_space(4.0);
        let camera = &moment.camera;
        egui::Grid::new("camera now").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            ui.label(theme::weak("Centre"));
            ui.label(RichText::new(format!("{} {}", camera.centre.x.round(), camera.centre.y.round())).monospace());
            ui.end_row();
            ui.label(theme::weak("Zoom"));
            ui.label(RichText::new(format!("{:.2}{}", camera.zoom, if camera.glides { " · glides" } else { "" })).monospace());
            ui.end_row();
            if let Some(who) = &camera.follows {
                ui.label(theme::weak("Follows"));
                ui.label(who);
                ui.end_row();
            }
            if !camera.keeps.is_empty() {
                ui.label(theme::weak("Keeps"));
                ui.label(camera.keeps.join(", "));
                ui.end_row();
            }
        });
        for bar in &moment.bars {
            ui.label(theme::text(format!("bar: {bar}"), theme::SMALL, theme::WEAK));
        }
        if let Some(said) = &moment.said {
            ui.add_space(4.0);
            ui.label(theme::text(format!("« {said} »"), theme::SMALL, theme::TEXT));
        }
        ui.add_space(8.0);
        crate::decors::section(ui, &format!("Characters ({})", moment.bodies.len()));
        ui.add_space(4.0);
        if moment.bodies.is_empty() {
            ui.label(theme::weak("nobody on stage"));
        }
        for body in &moment.bodies {
            ui.horizontal(|ui| {
                let picked = films.focus_who.as_ref() == Some(&body.name);
                let named = RichText::new(&body.name).strong().color(if body.hidden {
                    theme::WEAK
                } else if picked {
                    theme::FILM
                } else {
                    theme::STRONG
                });
                let row = ui.selectable_label(picked, named).on_hover_text("a click: chosen (their lines in the Timeline, their frame on the Scene)");
                if row.clicked() {
                    chosen = Some(body.name.clone());
                }
                if row.hovered() {
                    lit = Some(body.name.clone());
                }
                ui.label(RichText::new(format!("{} {}", body.at.x.round(), body.at.y.round())).monospace().color(theme::TEXT));
                if let Some(hp) = body.hp {
                    let (bar, _) = ui.allocate_exact_size(vec2(40.0, 6.0), Sense::hover());
                    ui.painter().rect_filled(bar, 0.0, theme::FIELD);
                    let fill = if hp > 0.5 {
                        theme::DUNGEON
                    } else if hp > 0.25 {
                        theme::FILM
                    } else {
                        theme::WRONG
                    };
                    ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(bar.width() * hp.clamp(0.0, 1.0), bar.height())), 0.0, fill);
                }
            });
            // (The rest, short: faces, walks, pose, HP…)
            ui.horizontal_wrapped(|ui| {
                ui.add_space(12.0);
                ui.label(theme::text(body.details().join(" · "), theme::SMALL, theme::WEAK));
            });
            ui.add_space(2.0);
        }
        ui.add_space(8.0);
        crate::decors::section(ui, &format!("Objects ({})", moment.objects.len()));
        ui.add_space(4.0);
        if moment.objects.is_empty() {
            ui.label(theme::weak("no object the film put"));
        }
        for object in &moment.objects {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&object.name).strong());
                ui.label(RichText::new(format!("{} {}", object.at.x.round(), object.at.y.round())).monospace().color(theme::TEXT));
                let details = object.details();
                if !details.is_empty() {
                    ui.label(theme::text(details.join(" · "), theme::SMALL, theme::WEAK));
                }
            });
        }
    });
    if let Some(who) = chosen {
        films.choose(who);
    }
    if lit.is_some() {
        scripts.lit_who_scene = lit;
    }
    if let (Some(line), Some(open)) = (go_line, scripts.open.as_mut()) {
        open.go_to_line(line);
    }
}
