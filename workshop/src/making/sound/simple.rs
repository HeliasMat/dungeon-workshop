//! A sound made simply, as a beginner would.

use eframe::egui::{self, Color32, RichText, Sense, vec2};
use shell::sound::Hears;

use super::SoundMaking;
use super::presets::{EASY, EASY_KINDS, eased, preset};
use super::versions::{hear, save_sound};
use super::views::draw_wave;
use crate::making::{Done, now_seed};
use crate::theme;

/// The maker for a beginner: choose a kind, move it with words, hear it, write it.
pub(super) fn make_sound_simply(ui: &mut egui::Ui, making: &mut SoundMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) -> Option<Done> {
    let mut done = None;
    if let Some((since, seconds)) = making.played {
        if since.elapsed().as_secs_f32() >= seconds {
            making.played = None;
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }
    if ui.ui_contains_pointer() && !ui.ctx().wants_keyboard_input() && ui.input(|input| input.key_pressed(egui::Key::Space)) {
        hear(making, audio, volume);
    }
    egui::ScrollArea::vertical().id_salt("easy sound").auto_shrink(false).show(ui, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(theme::text("1. Choose a kind of sound", theme::TITLE, theme::STRONG));
            ui.label(theme::text("(again: another of the same kind)", theme::SMALL, theme::FAINT));
        });
        kinds(ui, making, audio, volume);
        ui.add_space(14.0);
        step(ui, "2. Listen, adjust");
        listen_row(ui, making, audio, volume);
        ui.add_space(8.0);
        // (Its wave, as it is now.)
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            let (rect, response) = ui.allocate_exact_size(vec2((ui.available_width() - 20.0).min(720.0), 90.0), Sense::hover());
            draw_wave(ui, making, rect, &response);
        });
        ui.add_space(8.0);
        words(ui, making, audio, volume);
        ui.add_space(14.0);
        step(ui, "3. Give it a name and save it");
        done = save_row(ui, making);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.label(theme::text("To go further, \"Expert\" at the top: the same sound, with all its settings.", theme::SMALL, theme::FAINT));
        });
    });
    done
}

/// A step's title.
fn step(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(theme::text(title, theme::TITLE, theme::STRONG));
    });
}

/// The kinds of sound, by family: one clicked is drawn anew and heard.
fn kinds(ui: &mut egui::Ui, making: &mut SoundMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) {
    for (family, kinds) in EASY_KINDS {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(10.0);
            ui.add_sized([70.0, 40.0], egui::Label::new(theme::text(family, theme::SMALL, theme::WEAK)));
            for kind in kinds {
                let chosen = making.kind == *kind;
                let button = egui::Button::new(RichText::new(*kind).size(theme::BODY + 1.0).strong().color(if chosen { theme::GROUND } else { theme::TEXT }))
                    .fill(if chosen { theme::ASSETS } else { theme::FIELD })
                    .min_size(vec2(108.0, 40.0));
                if ui.add(button).clicked() {
                    making.ease(Some((preset(kind, now_seed()), kind.to_string())), [0.0; 7]);
                    hear(making, audio, volume);
                }
            }
        });
    }
}

/// Any kind of sound at all, by a seed.
fn surprise_kind(seed: u32) -> &'static str {
    let kinds: Vec<&str> = EASY_KINDS.iter().flat_map(|(_, kinds)| kinds.iter().copied()).collect();
    kinds[seed as usize % kinds.len()]
}

/// Listen, another like this, a little different, a surprise, undo.
fn listen_row(ui: &mut egui::Ui, making: &mut SoundMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) {
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        if ui
            .add(
                egui::Button::new(RichText::new("▶  Listen").size(theme::BODY + 2.0).strong().color(theme::GROUND))
                    .fill(theme::ASSETS)
                    .min_size(vec2(150.0, 42.0)),
            )
            .on_hover_text("Space")
            .clicked()
        {
            hear(making, audio, volume);
        }
        if ui
            .add(egui::Button::new(RichText::new("Another Like This").size(theme::BODY + 1.0)).min_size(vec2(150.0, 42.0)))
            .on_hover_text("the same kind, drawn again (the settings kept)")
            .clicked()
        {
            let kind = making.kind.clone();
            let words = making.words;
            making.ease(Some((preset(&kind, now_seed()), kind)), words);
            hear(making, audio, volume);
        }
        if ui
            .add(egui::Button::new(RichText::new("A Little Different").size(theme::BODY + 1.0)).min_size(vec2(150.0, 42.0)))
            .on_hover_text("the same, changed a little")
            .clicked()
        {
            let mut base = making.base.clone();
            for (at, voice) in base.voices.iter_mut().enumerate() {
                voice.synth = voice.synth.mutated(now_seed().wrapping_add(at as u32 * 31));
            }
            let kind = making.kind.clone();
            let words = making.words;
            making.ease(Some((base, kind)), words);
            hear(making, audio, volume);
        }
        if ui.add(egui::Button::new(RichText::new("Surprise!").size(theme::BODY + 1.0)).min_size(vec2(130.0, 42.0))).on_hover_text("any sound at all").clicked()
        {
            let seed = now_seed();
            let kind = surprise_kind(seed);
            making.ease(Some((preset(kind, seed), kind.to_string())), [0.0; 7]);
            hear(making, audio, volume);
        }
        if ui
            .add_enabled(!making.easy_undone.is_empty(), egui::Button::new(RichText::new("Undo").size(theme::BODY + 1.0)).min_size(vec2(100.0, 42.0)))
            .on_hover_text("back to the sound before")
            .clicked()
            && let Some((base, kind, words)) = making.easy_undone.pop()
        {
            (making.base, making.kind, making.words) = (base, kind, words);
            let sound = eased(&making.base, &making.words);
            making.undone.push(std::mem::replace(&mut making.sound, sound));
            making.saved = false;
            hear(making, audio, volume);
        }
    });
}

/// The sound's words, each a slider between two (a double-click: the middle); heard when one is let go.
fn words(ui: &mut egui::Ui, making: &mut SoundMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) {
    let mut words = making.words;
    let mut let_go = false;
    egui::Grid::new("the sound's words").num_columns(3).spacing([12.0, 10.0]).show(ui, |ui| {
        for (at, (low, high, help)) in EASY.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.add_sized([70.0, 20.0], egui::Label::new(theme::text(*low, theme::BODY, theme::TEXT)));
            });
            ui.spacing_mut().slider_width = 360.0;
            let response = ui.add(egui::Slider::new(&mut words[at], -1.0..=1.0).show_value(false)).on_hover_text(*help);
            if response.double_clicked() {
                words[at] = 0.0;
                let_go = true;
            }
            let_go |= response.drag_stopped() || (response.changed() && !response.dragged());
            ui.label(theme::text(*high, theme::BODY, theme::TEXT));
            ui.end_row();
        }
    });
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.label(theme::text("a double-click on a slider puts it back in the middle", theme::SMALL - 1.0, theme::FAINT));
    });
    if let_go && words != making.words {
        making.ease(None, words);
        hear(making, audio, volume);
    } else {
        making.words = words;
    }
}

/// Its name, saved; how a film writes it.
fn save_row(ui: &mut egui::Ui, making: &mut SoundMaking) -> Option<Done> {
    let mut done = None;
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        ui.add(egui::TextEdit::singleline(&mut making.word).desired_width(220.0).font(egui::TextStyle::Heading).hint_text("my_sound"));
        let said = if making.saved { "✔ Saved" } else { "Save" };
        if !making.embedded
            && ui
                .add(
                    egui::Button::new(RichText::new(said).size(theme::BODY + 2.0).strong().color(theme::GROUND))
                        .fill(theme::ASSETS)
                        .min_size(vec2(150.0, 36.0)),
                )
                .clicked()
        {
            done = save_sound(making);
        }
        if !making.embedded && ui.button("Close").clicked() {
            done = Some(Done::Closed);
        }
    });
    ui.horizontal(|ui| {
        ui.add_space(10.0);
        let word = film::words::plain(&making.word);
        ui.label(theme::text("In a film, it is written:", theme::SMALL, theme::WEAK));
        ui.add(egui::Label::new(RichText::new(format!("sound {word}")).monospace().color(Color32::from_rgb(0xDC, 0xDC, 0xAA))).selectable(true));
        if let Some(said) = &making.said {
            ui.label(theme::text(said, theme::SMALL, theme::FAINT));
        }
    });
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_surprise_is_one_of_the_kinds() {
        let all: Vec<&str> = EASY_KINDS.iter().flat_map(|(_, kinds)| kinds.iter().copied()).collect();
        for seed in [0, 1, 7, u32::MAX] {
            assert!(all.contains(&surprise_kind(seed)));
        }
        assert_eq!(surprise_kind(0), all[0]);
    }
}
