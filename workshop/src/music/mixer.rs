//! An instrument's settings, the mixer and its strips, the automation.

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, pos2, vec2};
use shell::sound::Hears;

use super::MusicMaking;
use super::editor::colour;
use super::project::{Kind, Target};
use super::wave::pcm;
use crate::sound::{self, SoundMaking, open_audio};
use workshop_base::theme;

/// The track's instrument, made with the sound maker (a drum kit: each drum); another chosen.
pub(super) fn instrument_view(ui: &mut egui::Ui, making: &mut MusicMaking, place: usize, audio: &mut Option<Box<dyn Hears>>, volume: f32) {
    let Some(track) = making.project.tracks.get(place) else { return };
    let kind = track.kind;
    if kind == Kind::Audio {
        ui.label(theme::text("An audio track has no instrument: its regions play sounds as they are (Notes: the region's sound).", theme::SMALL, theme::FAINT));
        return;
    }
    let names: Vec<String> = track.kit.iter().map(|(name, _)| name.clone()).collect();
    ui.horizontal_wrapped(|ui| match kind {
        Kind::Drums => {
            ui.label(theme::text("Drum", theme::SMALL, theme::WEAK));
            for (at, name) in names.iter().enumerate() {
                if ui.selectable_label(making.piece == at, name).clicked() {
                    making.piece = at;
                    making.editor = None;
                    let hit = sound::one_shot(&making.project.tracks[place].kit[at].1, 1.0, &making.session.own_sounds);
                    open_audio(&making.session, audio);
                    if let Some(audio) = audio.as_mut() {
                        audio.samples(pcm(&hit.0, &hit.1), volume);
                    }
                }
            }
            if ui.button("Original Kit").clicked() {
                making.project.tracks[place].kit = sound::drum_kit();
                making.editor = None;
            }
        }
        _ => {
            ui.menu_button("Change Instrument", |ui| {
                for kind in sound::INSTRUMENTS {
                    if ui.button(kind).clicked() {
                        making.project.tracks[place].instrument = sound::instrument(kind);
                        making.editor = None;
                        ui.close();
                    }
                }
            });
            ui.label(theme::text(
                "the whole Workshop sound, for this track: its pitch is middle C (the notes in semitones above it)",
                theme::SMALL - 1.0,
                theme::FAINT,
            ));
        }
    });
    let piece = making.piece.min(names.len().saturating_sub(1));
    let wanted = (place, if kind == Kind::Drums { piece } else { 0 });
    if making.editor.as_ref().is_none_or(|(track, piece, _)| (*track, *piece) != wanted) {
        let sound = match kind {
            Kind::Drums => match making.project.tracks[place].kit.get(piece) {
                Some((_, sound)) => sound.clone(),
                None => return,
            },
            _ => making.project.tracks[place].instrument.clone(),
        };
        making.editor = Some((wanted.0, wanted.1, SoundMaking::editing(&making.session, sound)));
    }
    let Some((_, _, editor)) = making.editor.as_mut() else { return };
    sound::make_sound(ui, editor, audio, volume);
    let made = editor.sound().clone();
    let track = &mut making.project.tracks[place];
    match kind {
        Kind::Drums => {
            if let Some((_, sound)) = track.kit.get_mut(piece).filter(|(_, sound)| *sound != made) {
                *sound = made;
            }
        }
        _ if track.instrument != made => track.instrument = made,
        _ => {}
    }
}

/// The mixer: each track's strip (how loud, from which side, muted or alone, its level, its filter, its sends,
/// what squeezes it, its effects), the two buses', the master's.
pub(super) fn mixer_view(ui: &mut egui::Ui, making: &mut MusicMaking) {
    let names: Vec<String> = std::iter::once("none".to_string()).chain(making.project.tracks.iter().map(|track| track.name.clone())).collect();
    egui::ScrollArea::both().id_salt("mixing desk").show(ui, |ui| {
        ui.horizontal_top(|ui| {
            let count = making.project.tracks.len();
            for at in 0..count {
                let peak = making.peaks.get(at).copied().unwrap_or(0.0);
                let track = &mut making.project.tracks[at];
                strip(ui, &track.name.clone(), colour(track.colour), peak, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut track.volume, 0.0..=1.5).vertical().show_value(false));
                        ui.vertical(|ui| {
                            ui.label(theme::text(format!("{:+.1} dB", 20.0 * track.volume.max(1e-4).log10()), theme::SMALL - 1.0, theme::WEAK));
                            ui.add(egui::DragValue::new(&mut track.pan).range(-1.0..=1.0).speed(0.01).prefix("pan ").max_decimals(2));
                            ui.horizontal(|ui| {
                                ui.toggle_value(&mut track.muted, "M");
                                ui.toggle_value(&mut track.solo, "S");
                            });
                        });
                    });
                    egui::Grid::new(("track settings", at)).num_columns(2).spacing([6.0, 3.0]).show(ui, |ui| {
                        ui.spacing_mut().slider_width = 110.0;
                        ui.label(theme::text("Filter", theme::SMALL - 1.0, theme::WEAK)).on_hover_text("a low-pass: the highs cut above it");
                        ui.add(egui::Slider::new(&mut track.cutoff, 20.0..=20_000.0).logarithmic(true).suffix(" Hz").max_decimals(0));
                        ui.end_row();
                        ui.label(theme::text("Resonance", theme::SMALL - 1.0, theme::WEAK));
                        ui.add(egui::Slider::new(&mut track.resonance, 0.0..=0.95).max_decimals(2));
                        ui.end_row();
                        ui.label(theme::text("Reverb Send", theme::SMALL - 1.0, theme::WEAK)).on_hover_text("to bus A, a shared reverb");
                        ui.add(egui::Slider::new(&mut track.send_a, 0.0..=1.0).max_decimals(2));
                        ui.end_row();
                        ui.label(theme::text("Echo Send", theme::SMALL - 1.0, theme::WEAK)).on_hover_text("to bus B, a shared echo");
                        ui.add(egui::Slider::new(&mut track.send_b, 0.0..=1.0).max_decimals(2));
                        ui.end_row();
                        ui.label(theme::text("Sidechain", theme::SMALL - 1.0, theme::WEAK))
                            .on_hover_text("ducked when another track strikes (the kick: it pumps)");
                        let shown = names.get(track.sidechain).cloned().unwrap_or_default();
                        egui::ComboBox::from_id_salt(("sidechain", at)).selected_text(shown).width(110.0).show_ui(ui, |ui| {
                            for (number, name) in names.iter().enumerate().filter(|(number, _)| *number != at + 1) {
                                ui.selectable_value(&mut track.sidechain, number, name);
                            }
                        });
                        ui.end_row();
                        ui.label(theme::text("Ducking", theme::SMALL - 1.0, theme::WEAK));
                        ui.add(egui::Slider::new(&mut track.duck, 0.0..=1.0).max_decimals(2));
                        ui.end_row();
                    });
                    sound::mix_editor(ui, &mut track.fx, &format!("track {at}"));
                });
            }
            let project = &mut making.project;
            strip(ui, "Bus A · reverb", theme::DECOR, 0.0, |ui| {
                sound::mix_editor(ui, &mut project.bus_a, "bus a");
            });
            strip(ui, "Bus B · echo", theme::DECOR, 0.0, |ui| {
                sound::mix_editor(ui, &mut project.bus_b, "bus b");
            });
            strip(ui, "Master", theme::STRONG, 0.0, |ui| {
                ui.horizontal(|ui| {
                    ui.label(theme::text("Volume", theme::SMALL - 1.0, theme::WEAK));
                    ui.add(egui::Slider::new(&mut project.master_volume, 0.0..=2.0).max_decimals(2));
                });
                sound::mix_editor(ui, &mut project.master, "master");
            });
        });
    });
}

/// A strip of the mixer: its name, its colour, its level, what it holds.
fn strip(ui: &mut egui::Ui, name: &str, shade: Color32, peak: f32, inside: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new().fill(theme::PANEL).corner_radius(4).inner_margin(6).show(ui, |ui| {
        ui.set_width(250.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                ui.painter().rect_filled(dot, 2.0, shade);
                ui.label(RichText::new(name).strong());
            });
            let (meter, _) = ui.allocate_exact_size(vec2(236.0, 5.0), Sense::hover());
            ui.painter().rect_filled(meter, 1.0, theme::GAP);
            ui.painter().rect_filled(
                Rect::from_min_size(meter.min, vec2(meter.width() * peak.min(1.0), meter.height())),
                1.0,
                if peak > 0.98 { theme::WRONG } else { Color32::from_rgb(0x6C, 0xC6, 0x6C) },
            );
            inside(ui);
        });
    });
}

/// A track's setting drawn along the song (volume, side, filter, sends): a click puts a point, dragging moves it,
/// a right click takes it off.
pub(super) fn automation_view(ui: &mut egui::Ui, making: &mut MusicMaking) {
    let end = making.project.end().max(4.0);
    let per_bar = making.project.beats_per_bar.max(1.0);
    let target = &mut making.lane;
    let Some(track) = making.project.tracks.get_mut(making.track) else { return };
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::text("Draw", theme::SMALL, theme::WEAK));
        for each in Target::ALL {
            let drawn = !track.lane(each).is_empty();
            let said = if drawn { format!("{} •", each.name()) } else { each.name().to_string() };
            ui.selectable_value(target, each, said);
        }
        if ui.button("Clear").clicked() {
            track.lane_mut(*target).clear();
        }
        ui.label(theme::text("along the song; no point: the mixing desk's setting", theme::SMALL - 1.0, theme::FAINT));
    });
    let target = *target;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width().min(1200.0), 180.0), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    let x_of = |beat: f32| rect.min.x + beat / end * rect.width();
    let y_of = |value: f32| rect.max.y - value * rect.height();
    let beat_of = |x: f32| ((x - rect.min.x) / rect.width() * end).clamp(0.0, end);
    let value_of = |y: f32| ((rect.max.y - y) / rect.height()).clamp(0.0, 1.0);
    painter.hline(rect.x_range(), y_of(0.5), Stroke::new(1.0_f32, theme::HOVERED));
    for bar in 0..=(end / per_bar) as i32 {
        painter.vline(x_of(bar as f32 * per_bar), rect.y_range(), Stroke::new(1.0_f32, theme::PANEL));
    }
    let shade = colour(track.colour);
    let lane = track.lane_mut(target);
    let points: Vec<Pos2> = lane.iter().map(|(beat, value)| pos2(x_of(*beat), y_of(*value))).collect();
    if points.len() > 1 {
        painter.add(egui::Shape::line(points.clone(), Stroke::new(2.0_f32, shade)));
    }
    for (point, (_, value)) in points.iter().zip(lane.iter()) {
        painter.circle_filled(*point, 4.0, Color32::WHITE);
        if response.hovered() && ui.input(|input| input.pointer.hover_pos()).is_some_and(|at| at.distance(*point) < 10.0) {
            painter.text(
                *point + vec2(6.0, -10.0),
                egui::Align2::LEFT_BOTTOM,
                target.said(*value),
                egui::FontId::proportional(theme::SMALL - 1.0),
                theme::STRONG,
            );
        }
    }
    let dragged_id = egui::Id::new("dragged automation point");
    if let Some(at) = response.interact_pointer_pos() {
        let near = points.iter().position(|point| point.distance(at) < 8.0);
        if response.secondary_clicked() {
            if let Some(near) = near {
                lane.remove(near);
            }
        } else if response.drag_started() || response.clicked() {
            let taken = match near {
                Some(near) => near,
                None => {
                    lane.push(((beat_of(at.x) * 4.0).round() / 4.0, value_of(at.y)));
                    lane.len() - 1
                }
            };
            ui.data_mut(|data| data.insert_temp(dragged_id, taken));
        }
        if response.dragged()
            && let Some(taken) = ui.data(|data| data.get_temp::<usize>(dragged_id))
            && let Some(point) = lane.get_mut(taken)
        {
            *point = ((beat_of(at.x) * 4.0).round() / 4.0, value_of(at.y));
        }
    }
    if response.drag_stopped() || response.clicked() {
        lane.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    // (A lane looked at but not drawn: none, the song unchanged.)
    track.lanes.retain(|(_, points)| !points.is_empty());
}
