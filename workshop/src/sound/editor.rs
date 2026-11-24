//! The sound's editor, as an expert has it: the mode, the bar, the keys, the
//! layers, the layer chosen, made by kind or chance; its wave, pitch and
//! spectrum; its settings (`settings.rs`); beside it, the side panel.

use eframe::egui::{self, Rect, RichText, Sense, pos2, vec2};
use shell::sound::Hears;

use super::now_seed;
use super::presets::{EASY_KINDS, KINDS, preset};
use super::settings::{Touched, layer_settings};
use super::simple::make_sound_simply;
use super::versions::{hear, redo_sound, save_sound, undo_sound};
use super::views::{draw_pitch, draw_spectrum, draw_wave, side_sound};
use super::{Sound, SoundMaking, Synth, Voice};
use workshop_base::done::Done;
use workshop_base::theme;

/// Where the sound goes out: the card, how loud.
struct Out<'a> {
    audio: &'a mut Option<Box<dyn Hears>>,
    volume: f32,
}

impl Out<'_> {
    /// The sound heard as it is now.
    fn hear(&mut self, making: &mut SoundMaking) {
        hear(making, self.audio, self.volume);
    }
}

/// The keys that choose a layer, from the first.
const LAYER_KEYS: [egui::Key; 8] =
    [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7, egui::Key::Num8];

/// What a layer's button says: its number, its name, muted, alone.
pub(super) fn layer_said(at: usize, voice: &Voice) -> String {
    let mut said = format!("{}  {}", at + 1, voice.name);
    if voice.muted {
        said.push_str(" (muted)");
    }
    if voice.solo {
        said.push_str(" (solo)");
    }
    said
}

/// The sound maker.
pub fn make_sound(ui: &mut egui::Ui, making: &mut SoundMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) -> Option<Done> {
    let mut done = None;
    // (The voice chosen is one the sound has, whatever replaced it: undone, redone, a version, a kind.)
    making.voice = making.voice.min(making.sound.voices.len().saturating_sub(1));
    mode(ui, making);
    if !making.expert {
        return make_sound_simply(ui, making, audio, volume);
    }
    let mut out = Out { audio, volume };
    bar(ui, making, &mut out, &mut done);
    keys(ui, making, &mut out, &mut done);
    keep_looping(ui, making, &mut out);
    let before = making.sound.clone();
    let mut touched = Touched::default();
    // (A voice added, doubled or taken out: the sound made so at once, before its voice is shown.)
    if let Some((sound, said)) = layers(ui, making) {
        making.set(sound, said);
    }
    making.voice = making.voice.min(making.sound.voices.len().saturating_sub(1));
    layer_row(ui, making, &mut touched);
    made_by_kind(ui, making, &mut out);
    ui.add_space(4.0);
    let room = ui.available_rect_before_wrap();
    let side_room = Rect::from_min_max(pos2(room.max.x - 210.0, room.min.y), room.max - vec2(6.0, 4.0));
    let main = Rect::from_min_max(room.min + vec2(6.0, 0.0), pos2(side_room.min.x - 10.0, room.max.y - 4.0));
    let mut left = ui.new_child(egui::UiBuilder::new().max_rect(main).layout(egui::Layout::top_down(egui::Align::Min)));
    seen(&mut left, making, main, &mut touched);
    let versioned = layer_settings(&mut left, making, &mut touched);
    settled(making, &mut out, &before, touched, versioned);
    // --- Beside: how loud, a sound of the game to compare, variants, versions ---
    let mut right = ui.new_child(egui::UiBuilder::new().max_rect(side_room).layout(egui::Layout::top_down(egui::Align::Min)));
    side_sound(&mut right, making, out.audio, volume, &mut done);
    ui.allocate_rect(room, Sense::hover());
    done
}

/// The mode: simply, or as an expert.
fn mode(ui: &mut egui::Ui, making: &mut SoundMaking) {
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        let mut expert = making.expert;
        if ui.selectable_label(!expert, RichText::new("Easy").strong()).on_hover_text("choose a kind of sound, adjust it with words").clicked() {
            expert = false;
        }
        if ui.selectable_label(expert, RichText::new("Expert").strong()).on_hover_text("all that makes a sound, no limits").clicked() {
            expert = true;
        }
        making.set_expert(expert);
    });
}

/// The bar: the word, Listen, Loop, Play on Change, Undo, Redo; Close, Save, what was said.
fn bar(ui: &mut egui::Ui, making: &mut SoundMaking, out: &mut Out, done: &mut Option<Done>) {
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        if !making.embedded {
            ui.label(theme::text("Word", theme::SMALL, theme::WEAK));
            ui.add(egui::TextEdit::singleline(&mut making.word).desired_width(140.0));
            ui.separator();
        }
        if ui.add(egui::Button::new(RichText::new("▶ Listen").strong().color(theme::GROUND)).fill(theme::ASSETS)).on_hover_text("Space").clicked() {
            out.hear(making);
        }
        ui.checkbox(&mut making.looped, "Loop");
        ui.checkbox(&mut making.play_on_change, "Play on Change");
        ui.separator();
        if ui.add_enabled(!making.undone.is_empty(), egui::Button::new("Undo")).on_hover_text("Ctrl+Z").clicked() {
            undo_sound(making);
        }
        if ui.add_enabled(!making.redone.is_empty(), egui::Button::new("Redo")).on_hover_text("Ctrl+Y").clicked() {
            redo_sound(making);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(6.0);
            if !making.embedded && ui.button("Close").clicked() {
                *done = Some(Done::Closed);
            }
            let said = if making.saved { "✔ Saved" } else { "Save" };
            if !making.embedded
                && ui
                    .add(egui::Button::new(RichText::new(said).strong().color(theme::GROUND)).fill(theme::ASSETS))
                    .on_hover_text("Ctrl+S: library/sounds/<word>.wav; in a film: sound <word>")
                    .clicked()
            {
                *done = save_sound(making);
            }
            if let Some(said) = &making.said {
                ui.label(theme::text(said, theme::SMALL, theme::FAINT));
            }
        });
    });
}

/// The keys, the pointer over the maker: Space heard, Ctrl+Z undone, Ctrl+Y redone, Ctrl+S saved, 1 to 8 a layer.
fn keys(ui: &mut egui::Ui, making: &mut SoundMaking, out: &mut Out, done: &mut Option<Done>) {
    if !ui.ui_contains_pointer() || ui.ctx().wants_keyboard_input() {
        return;
    }
    let (space, undo, redo, save) = ui.input_mut(|input| {
        (
            input.key_pressed(egui::Key::Space),
            input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z),
            input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y),
            input.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
        )
    });
    if space {
        out.hear(making);
    }
    if undo {
        undo_sound(making);
    }
    if redo {
        redo_sound(making);
    }
    if save && !making.embedded {
        *done = save_sound(making);
    }
    for (at, key) in LAYER_KEYS.into_iter().enumerate() {
        if ui.input(|input| input.key_pressed(key)) && at < making.sound.voices.len() {
            making.voice = at;
        }
    }
}

/// Round and round: heard again once it has ended (looped), else done.
fn keep_looping(ui: &egui::Ui, making: &mut SoundMaking, out: &mut Out) {
    let Some((since, seconds)) = making.played else { return };
    if since.elapsed().as_secs_f32() >= seconds {
        if making.looped {
            out.hear(making);
        } else {
            making.played = None;
        }
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
}

/// The layers: each one's button, one more, doubled, removed (the sound so made given back).
fn layers(ui: &mut egui::Ui, making: &mut SoundMaking) -> Option<(Sound, &'static str)> {
    let mut versioned = None;
    ui.horizontal_wrapped(|ui| {
        ui.add_space(6.0);
        ui.label(theme::text("Layers", theme::SMALL, theme::WEAK));
        for at in 0..making.sound.voices.len() {
            if ui.selectable_label(making.voice == at, layer_said(at, &making.sound.voices[at])).on_hover_text("keys 1 to 4").clicked() {
                making.voice = at;
            }
        }
        if making.sound.voices.len() < 8 && ui.small_button("+").on_hover_text("one more layer, mixed with the others").clicked() {
            let mut sound = making.sound.clone();
            let number = sound.voices.len() + 1;
            sound.voices.push(Voice::of(&format!("Layer {number}"), Synth::of_kind("Blip", now_seed())));
            making.voice = sound.voices.len() - 1;
            versioned = Some((sound, "new layer"));
        }
        if making.sound.voices.len() < 8 && ui.small_button("Duplicate").clicked() {
            let mut sound = making.sound.clone();
            let mut copy = sound.voices[making.voice].clone();
            copy.name = format!("{} (copy)", copy.name);
            sound.voices.insert(making.voice + 1, copy);
            making.voice += 1;
            versioned = Some((sound, "duplicate the layer"));
        }
        if making.sound.voices.len() > 1 && ui.small_button("Remove").clicked() {
            let mut sound = making.sound.clone();
            sound.voices.remove(making.voice);
            making.voice = making.voice.min(sound.voices.len() - 1);
            versioned = Some((sound, "remove the layer"));
        }
    });
    versioned
}

/// The layer chosen: its name, muted, alone, when it starts, its gain, its pan.
fn layer_row(ui: &mut egui::Ui, making: &mut SoundMaking, touched: &mut Touched) {
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        let voice = &mut making.sound.voices[making.voice];
        ui.add(egui::TextEdit::singleline(&mut voice.name).desired_width(110.0));
        if ui.selectable_label(voice.muted, "Mute").clicked() {
            voice.muted = !voice.muted;
            touched.both();
        }
        if ui.selectable_label(voice.solo, "Solo").on_hover_text("hear only this one").clicked() {
            voice.solo = !voice.solo;
            touched.both();
        }
        ui.spacing_mut().slider_width = 110.0;
        for (name, value, range, suffix, help) in [
            ("Start", &mut voice.offset, 0.0..=2.0, " s", "when it begins"),
            ("Gain", &mut voice.gain, 0.0..=2.0, "", "how much it counts in the mix"),
            ("Pan", &mut voice.pan, -1.0..=1.0, "", "left (-1), middle, right (1)"),
        ] {
            ui.label(theme::text(name, theme::SMALL, theme::WEAK));
            touched.dragged(&ui.add(egui::Slider::new(value, range).suffix(suffix).max_decimals(2)).on_hover_text(help));
        }
    });
}

/// Made by kind, by chance, a little changed (the locked groups kept); heard at once.
fn made_by_kind(ui: &mut egui::Ui, making: &mut SoundMaking, out: &mut Out) {
    ui.horizontal_wrapped(|ui| {
        ui.add_space(6.0);
        ui.label(theme::text("Make", theme::SMALL, theme::WEAK));
        for kind in KINDS {
            if ui.button(kind).on_hover_text("a sound of this kind, at random (again: another); the chosen layer").clicked() {
                making.set_synth(Synth::of_kind(kind, now_seed()), kind);
                out.hear(making);
            }
        }
        ui.menu_button("More…", |ui| {
            for (family, kinds) in EASY_KINDS {
                ui.label(theme::text(family, theme::SMALL, theme::WEAK));
                for kind in kinds.iter().filter(|kind| !KINDS.contains(kind)) {
                    if ui.button(*kind).on_hover_text("the whole sound (its layers) replaced").clicked() {
                        making.set(preset(kind, now_seed()), kind);
                        out.hear(making);
                        ui.close();
                    }
                }
            }
        });
        ui.separator();
        if ui.button("Random").clicked() {
            making.set_synth(Synth::random(now_seed()), "random");
            out.hear(making);
        }
        if ui.button("Mutate").on_hover_text("the same, changed a little").clicked() {
            let mutated = making.synth().mutated(now_seed());
            making.set_synth(mutated, "mutated");
            out.hear(making);
        }
    });
}

/// The sound seen: its wave (the handles of the layer's envelope on it), its pitch, its spectrum.
fn seen(left: &mut egui::Ui, making: &mut SoundMaking, main: Rect, touched: &mut Touched) {
    let wave_rect = Rect::from_min_size(left.cursor().min, vec2(main.width(), 120.0));
    let wave_response = left.allocate_rect(wave_rect, Sense::click_and_drag());
    touched.add(draw_wave(left, making, wave_rect, &wave_response));
    left.add_space(4.0);
    let lower = Rect::from_min_size(left.cursor().min, vec2(main.width(), 64.0));
    left.allocate_rect(lower, Sense::hover());
    let pitch_rect = Rect::from_min_max(lower.min, pos2(lower.center().x - 4.0, lower.max.y));
    let spectrum_rect = Rect::from_min_max(pos2(lower.center().x + 4.0, lower.min.y), lower.max);
    draw_pitch(left, making, pitch_rect);
    draw_spectrum(left, making, spectrum_rect);
    left.add_space(6.0);
}

/// A setting taken: what the sound was, kept; let go: if it moved, a version
/// (undone later), heard. A sound asked by its text, made; the text written again.
fn settled(making: &mut SoundMaking, out: &mut Out, before: &Sound, touched: Touched, versioned: Option<(Sound, &'static str)>) {
    if touched.started && making.taken.is_none() {
        making.taken = Some(before.clone());
    }
    if touched.let_go
        && let Some(was) = making.taken.take()
        && was != making.sound
    {
        making.undone.push(was);
        making.redone.clear();
        making.saved = false;
        let sound = making.sound.clone();
        making.versions.insert(0, ("set".to_string(), sound));
        making.versions.truncate(30);
        if making.play_on_change {
            out.hear(making);
        }
    }
    if let Some((sound, said)) = versioned {
        making.set(sound, said);
    }
    if making.text.is_some() && touched.let_go {
        making.text = Some(making.sound.written());
    }
}
