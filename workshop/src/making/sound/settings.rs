//! The expert's settings of a layer, section after section: its wave (and
//! the wave's own), the six groups, modulation and notes (a wave drawn by
//! hand), unison and colour, the filter's kind and a vowel, free modulation
//! (its routes), another layer as a source, a score; the sound as text; the
//! mix of all the layers. Each says whether a setting was taken or let go.

use eframe::egui::{self, Rect, RichText, Sense, Stroke, pos2, vec2};

use super::controls::{Row, choice, group, piano_roll, setting, steps_editor};
use super::{FILTER_MODES, GROUPS, LFO_SHAPES, MOD_SOURCES, MOD_TARGETS, Mix, Note, Sound, SoundMaking, Synth, VOWELS, Voice, WAVES, sine_drawn};
use crate::theme;

/// Whether a setting was taken (what the sound was is to be kept) and let go
/// (a version to be made, if it moved).
#[derive(Default, Clone, Copy)]
pub(super) struct Touched {
    pub(super) started: bool,
    pub(super) let_go: bool,
}

impl Touched {
    /// A pair said by a control: taken, let go.
    pub(super) fn add(&mut self, (taken, gone): (bool, bool)) {
        self.started |= taken;
        self.let_go |= gone;
    }

    /// Changed at once (a click): taken and let go.
    pub(super) fn both(&mut self) {
        self.add((true, true));
    }

    /// What a dragged value said: taken as its drag starts (or changed without
    /// a drag), let go as it stops.
    pub(super) fn dragged(&mut self, response: &egui::Response) {
        self.add(dragged_pair(response.drag_started(), response.drag_stopped(), response.changed(), response.dragged()));
    }
}

/// Taken and let go, for a value dragged: its drag starting, stopping; a change not dragged (typed, a click) both.
pub(super) fn dragged_pair(drag_started: bool, drag_stopped: bool, changed: bool, dragged: bool) -> (bool, bool) {
    let at_once = changed && !dragged;
    (drag_started || at_once, drag_stopped || at_once)
}

/// The drawn wave's models: a square, a saw.
pub(super) fn square_drawn() -> [f32; 32] {
    std::array::from_fn(|at| if at < 16 { 0.8 } else { -0.8 })
}

pub(super) fn saw_drawn() -> [f32; 32] {
    std::array::from_fn(|at| 1.0 - 2.0 * at as f32 / 31.0)
}

/// Each step of a drawn period brought toward its neighbours (going round).
pub(super) fn smoothed(before: [f32; 32]) -> [f32; 32] {
    std::array::from_fn(|at| (before[(at + 31) % 32] + 2.0 * before[at] + before[(at + 1) % 32]) / 4.0)
}

/// The curve's models: flat, rising, falling, a bump.
pub(super) fn curve_models() -> [(&'static str, [f32; 32]); 4] {
    [
        ("Flat", [0.0_f32; 32]),
        ("Rises", std::array::from_fn(|at| at as f32 / 31.0 * 2.0 - 1.0)),
        ("Falls", std::array::from_fn(|at| 1.0 - at as f32 / 31.0 * 2.0)),
        ("Bump", std::array::from_fn(|at| (std::f32::consts::PI * at as f32 / 31.0).sin())),
    ]
}

/// The score made of the layer's simple notes, one after the other, each as long as they are (in beats at that tempo).
pub(super) fn score_of_notes(synth: &Synth, tempo: f32) -> Vec<Note> {
    let beat = 60.0 / tempo.max(1.0);
    let length = synth.note_time / beat;
    let count = synth.notes_len.round().clamp(1.0, 8.0) as usize;
    (0..count).map(|at| Note { start: at as f32 * length, length, tone: synth.notes[at], velocity: 1.0 }).collect()
}

/// The score again, after itself (from the whole beat after its end).
pub(super) fn score_doubled(notes: &mut Vec<Note>) {
    let end = notes.iter().map(|note| note.start + note.length).fold(0.0_f32, f32::max).ceil();
    let again: Vec<Note> = notes.iter().map(|note| Note { start: note.start + end, ..*note }).collect();
    notes.extend(again);
}

/// Settings in a grid of two columns, each with its name.
fn rows(ui: &mut egui::Ui, id: &str, rows: Vec<Row<'_>>, touched: &mut Touched) {
    egui::Grid::new(id).num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
        for (name, value, range, suffix, help) in rows {
            touched.add(setting(ui, name, value, range, suffix, help));
        }
    });
}

/// The settings of the layer chosen, and the mix, in their scrolling; a sound
/// asked by its text, given back.
pub(super) fn layer_settings(ui: &mut egui::Ui, making: &mut SoundMaking, touched: &mut Touched) -> Option<(Sound, &'static str)> {
    let mut versioned = None;
    egui::ScrollArea::vertical().id_salt("sound settings").auto_shrink(false).show(ui, |ui| {
        wave_choice(ui, &mut making.sound.voices[making.voice].synth, touched);
        wave_own(ui, &making.session, &mut making.sound.voices[making.voice], touched);
        six_groups(ui, &mut making.locks, &mut making.sound.voices[making.voice].synth, touched);
        modulation_and_notes(ui, &mut making.sound.voices[making.voice].synth, touched);
        deeper(ui, making, touched);
        versioned = sound_text(ui, making);
        mix_settings(ui, &mut making.sound.mix, touched);
    });
    versioned
}

/// The wave, among all.
fn wave_choice(ui: &mut egui::Ui, synth: &mut Synth, touched: &mut Touched) {
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::text("Wave", theme::SMALL, theme::WEAK));
        for (at, name) in WAVES.iter().enumerate() {
            if ui.selectable_label(synth.wave == at, *name).clicked() && synth.wave != at {
                touched.both();
                synth.wave = at;
            }
        }
    });
}

/// The wave's own settings: the plucked string's, the sample's (one of the sounds made, its pitch, its loop, its start).
fn wave_own(ui: &mut egui::Ui, session: &crate::session::Session, voice: &mut Voice, touched: &mut Touched) {
    match voice.synth.wave {
        8 => rows(
            ui,
            "string",
            vec![
                ("Damping", &mut voice.synth.string_damp, 0.0..=1.0, "", "the string dies fast (1) or rings long (0)"),
                ("Brightness", &mut voice.synth.string_bright, 0.0..=1.0, "", "dull (0), like nylon, or bright (1), like metal"),
            ],
            touched,
        ),
        9 => {
            let own = shell::own_sounds::names_in(&session.library().sounds());
            ui.horizontal(|ui| {
                ui.label(theme::text("Sound Played", theme::SMALL, theme::WEAK));
                let shown = if voice.sample.is_empty() { "(choose one of your sounds)".to_string() } else { voice.sample.clone() };
                egui::ComboBox::from_id_salt("sample").selected_text(shown).width(180.0).show_ui(ui, |ui| {
                    for name in &own {
                        if ui.selectable_label(voice.sample == *name, name).clicked() {
                            voice.sample = name.clone();
                            touched.both();
                        }
                    }
                });
                if ui.button("Original Pitch").on_hover_text("played at its own speed: the pitch at 440 Hz (higher: faster and higher)").clicked() {
                    voice.synth.pitch = 440.0;
                    touched.both();
                }
                let mut looped = voice.synth.sample_loop > 0.5;
                if ui.checkbox(&mut looped, "Loop").on_hover_text("taken again from its (set) start as long as the layer lasts").changed() {
                    voice.synth.sample_loop = f32::from(u8::from(looped));
                    touched.both();
                }
            });
            rows(ui, "sample settings", vec![("Start", &mut voice.synth.sample_start, 0.0..=4.0, " s", "where it is played from (and looped from)")], touched);
            if own.is_empty() {
                ui.label(theme::text("no sound of yours yet: save one, it will be playable here", theme::SMALL, theme::FAINT));
            }
        }
        _ => {}
    }
}

/// The six groups in two columns, each lockable (kept when the sound is made again by kind, by chance).
fn six_groups(ui: &mut egui::Ui, locks: &mut [bool; 6], synth: &mut Synth, touched: &mut Touched) {
    ui.columns(2, |columns| {
        let [lock_envelope, lock_pitch, lock_vibrato, lock_wave, lock_filters, lock_effects] = locks;
        touched.add(group(
            &mut columns[0],
            GROUPS[0],
            lock_envelope,
            vec![
                ("Attack", &mut synth.attack, 0.0..=2.0, " s", "the time to rise (on the wave: the 1st handle)"),
                ("Sustain", &mut synth.sustain, 0.0..=2.0, " s", "the time held (the 2nd handle)"),
                ("Punch", &mut synth.punch, 0.0..=1.0, "", "louder at the start of the sustain (the top handle)"),
                ("Decay", &mut synth.decay, 0.0..=3.0, " s", "the time to die away (the 3rd handle)"),
            ],
        ));
        touched.add(group(
            &mut columns[0],
            GROUPS[1],
            lock_pitch,
            vec![
                ("Pitch", &mut synth.pitch, 30.0..=4000.0, " Hz", "the note at the start"),
                ("Floor", &mut synth.min_pitch, 0.0..=1000.0, " Hz", "the sound stops below it (0: never)"),
                ("Slide", &mut synth.slide, -16.0..=16.0, " oct/s", "rises or falls, in octaves per second"),
                ("Speeds Up", &mut synth.delta_slide, -16.0..=16.0, " oct/s²", "the slide that speeds up"),
            ],
        ));
        touched.add(group(
            &mut columns[0],
            GROUPS[2],
            lock_vibrato,
            vec![
                ("Vibrato", &mut synth.vibrato, 0.0..=1.0, "", "how much the note trembles"),
                ("Speed", &mut synth.vibrato_hz, 0.5..=40.0, " Hz", "how fast"),
                ("Arpeggio", &mut synth.arp_mult, 0.25..=4.0, "×", "the note multiplied once (1: nothing)"),
                ("When", &mut synth.arp_time, 0.0..=1.0, " s", "when the note jumps (0: never)"),
            ],
        ));
        touched.add(group(
            &mut columns[1],
            GROUPS[3],
            lock_wave,
            vec![
                ("Harmonics", &mut synth.harmonics, 0.0..=6.0, "", "higher notes on top, each softer (the waves that are not noises)"),
                ("Duty", &mut synth.duty, 0.05..=0.95, "", "the high part of the square (0.5: round, lower: nasal)"),
                ("Duty Sweep", &mut synth.duty_sweep, -1.0..=1.0, "/s", "the duty cycle changing"),
                ("Repeat", &mut synth.repeat, 0.0..=1.0, " s", "the pitch starts over every… (0: never)"),
            ],
        ));
        touched.add(group(
            &mut columns[1],
            GROUPS[4],
            lock_filters,
            vec![
                ("Low-pass", &mut synth.lowpass, 100.0..=16000.0, " Hz", "the highs cut above it (16000: nothing)"),
                ("Sweep", &mut synth.lowpass_sweep, -8.0..=8.0, " oct/s", "the low-pass moving"),
                ("Resonance", &mut synth.resonance, 0.0..=0.95, "", "the peak at the low-pass"),
                ("High-pass", &mut synth.highpass, 0.0..=4000.0, " Hz", "the lows cut below it (0: nothing)"),
                ("Sweep", &mut synth.highpass_sweep, -8.0..=8.0, " oct/s", "the high-pass moving"),
            ],
        ));
        touched.add(group(
            &mut columns[1],
            GROUPS[5],
            lock_effects,
            vec![
                ("Flanger", &mut synth.flanger, 0.0..=10.0, " ms", "itself a little later, on top"),
                ("Sweep", &mut synth.flanger_sweep, -10.0..=10.0, " ms/s", "the flanger moving"),
                ("Crush", &mut synth.crush, 0.0..=16000.0, " Hz", "coarser (0: nothing)"),
                ("Volume", &mut synth.volume, 0.05..=1.0, "", "loud or soft"),
            ],
        ));
    });
}

/// Beyond: modulation (FM, AM, drive), notes one after the other, a wave drawn by hand.
fn modulation_and_notes(ui: &mut egui::Ui, synth: &mut Synth, touched: &mut Touched) {
    egui::CollapsingHeader::new(RichText::new("Modulation and Notes").strong()).id_salt("modulation").default_open(false).show(ui, |ui| {
        ui.columns(2, |columns| {
            rows(
                &mut columns[0],
                "modulation 1",
                vec![
                    ("FM Ratio", &mut synth.fm_ratio, 0.0..=16.0, "×", "the speed of the modulating wave (FM), against the note"),
                    ("FM Depth", &mut synth.fm_depth, 0.0..=10.0, "", "how much it bends the wave (0: nothing): bells, metal, strange voices"),
                    ("AM Speed", &mut synth.am_hz, 0.0..=200.0, " Hz", "how fast the volume trembles"),
                    ("AM Depth", &mut synth.am_depth, 0.0..=1.0, "", "how much (0: nothing)"),
                    ("Drive", &mut synth.drive, 0.0..=1.0, "", "a warm distortion"),
                ],
                touched,
            );
            let column = &mut columns[1];
            rows(
                column,
                "notes",
                vec![
                    ("Notes", &mut synth.notes_len, 0.0..=8.0, "", "how many notes one after another (0: none): fanfares, jingles"),
                    ("Each", &mut synth.note_time, 0.01..=1.0, " s", "the time of each note"),
                ],
                touched,
            );
            column.horizontal_wrapped(|ui| {
                ui.label(theme::text("semitones", theme::SMALL, theme::WEAK));
                for at in 0..synth.notes_len.round().clamp(0.0, 8.0) as usize {
                    touched.dragged(&ui.add(egui::DragValue::new(&mut synth.notes[at]).speed(0.1).range(-48.0..=48.0).fixed_decimals(0)));
                }
            });
        });
        drawn_wave(ui, synth, touched);
    });
}

/// The drawn wave: one period, drawn with the pointer; its models, smoothed.
fn drawn_wave(ui: &mut egui::Ui, synth: &mut Synth, touched: &mut Touched) {
    ui.label(theme::text("Drawn wave (the \"Drawn\" wave; drag to draw one period)", theme::SMALL, theme::WEAK));
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width().min(520.0), 90.0), Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::GROUND);
    painter.hline(rect.x_range(), rect.center().y, Stroke::new(1.0_f32, theme::GAP));
    let wide = rect.width() / 32.0;
    for (at, value) in synth.drawn.iter().enumerate() {
        let x = rect.min.x + at as f32 * wide;
        let y = rect.center().y - value.clamp(-1.0, 1.0) * rect.height() * 0.45;
        let bar = Rect::from_two_pos(pos2(x + 1.0, rect.center().y), pos2(x + wide - 1.0, y));
        painter.rect_filled(bar, 0.0, if synth.wave == 7 { theme::ASSETS } else { theme::FAINT });
    }
    if let Some(at) = response.interact_pointer_pos().filter(|_| response.dragged() || response.clicked()) {
        let step = (((at.x - rect.min.x) / wide) as usize).min(31);
        synth.drawn[step] = ((rect.center().y - at.y) / (rect.height() * 0.45)).clamp(-1.0, 1.0);
        if response.drag_started() || response.clicked() {
            touched.started = true;
        }
    }
    if response.drag_stopped() || response.clicked() {
        touched.let_go = true;
    }
    ui.horizontal(|ui| {
        for (name, made) in [("Sine", sine_drawn()), ("Square", square_drawn()), ("Saw", saw_drawn())] {
            if ui.small_button(name).clicked() {
                synth.drawn = made;
                touched.both();
            }
        }
        if ui.small_button("Smooth").on_hover_text("each step toward its neighbours").clicked() {
            synth.drawn = smoothed(synth.drawn);
            touched.both();
        }
    });
}

/// Deeper: unison and colour, the filter's kind and a vowel, free modulation, another layer as a source, a score.
fn deeper(ui: &mut egui::Ui, making: &mut SoundMaking, touched: &mut Touched) {
    let place = making.voice;
    let names: Vec<String> =
        std::iter::once("none".to_string()).chain(making.sound.voices.iter().enumerate().map(|(at, voice)| format!("{} {}", at + 1, voice.name))).collect();
    let Sound { voices, mix } = &mut making.sound;
    let voice = &mut voices[place];
    unison_and_colour(ui, &mut voice.synth, touched);
    filter_and_vowel(ui, &mut voice.synth, touched);
    free_modulation(ui, &mut voice.synth, touched);
    routing(ui, &mut voice.synth, &names, touched);
    score(ui, voice, mix, place, touched);
}

/// Unison and colour: copies together, folded, ring, chorus, phaser.
fn unison_and_colour(ui: &mut egui::Ui, synth: &mut Synth, touched: &mut Touched) {
    egui::CollapsingHeader::new(RichText::new("Unison and Colour").strong()).id_salt("unison").default_open(false).show(ui, |ui| {
        ui.columns(2, |columns| {
            rows(
                &mut columns[0],
                "unison 1",
                vec![
                    ("Unison", &mut synth.unison, 1.0..=16.0, "", "so many copies of it played together (1: alone): a super saw, a choir"),
                    ("Detune", &mut synth.detune, 0.0..=100.0, " cents", "how far the copies stray from the note"),
                    ("Width", &mut synth.spread, 0.0..=1.0, "", "the copies spread from left to right"),
                    ("Fold", &mut synth.fold, 0.0..=1.0, "", "the wave pushed beyond its edges folds back: rich, metallic harmonics"),
                ],
                touched,
            );
            rows(
                &mut columns[1],
                "unison 2",
                vec![
                    ("Ring", &mut synth.ring_hz, 1.0..=2000.0, " Hz", "multiplied by a wave (ring modulation): off-key bells, robots"),
                    ("Ring Mix", &mut synth.ring_mix, 0.0..=1.0, "", "how much (0: nothing)"),
                    ("Chorus", &mut synth.chorus, 0.0..=1.0, "", "three copies a little late, wavering: wider, softer"),
                    ("Chorus Speed", &mut synth.chorus_hz, 0.05..=8.0, " Hz", "how fast it wavers"),
                    ("Phaser", &mut synth.phaser, 0.0..=1.0, "", "notches sweeping the sound: a turning whoosh"),
                    ("Phaser Speed", &mut synth.phaser_hz, 0.05..=8.0, " Hz", "how fast they sweep"),
                ],
                touched,
            );
        });
    });
}

/// The filter's kind, its peak, a vowel.
fn filter_and_vowel(ui: &mut egui::Ui, synth: &mut Synth, touched: &mut Touched) {
    egui::CollapsingHeader::new(RichText::new("Filter: Kind and Vowel").strong()).id_salt("filter and vowel").default_open(false).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(theme::text("Kind", theme::SMALL, theme::WEAK));
            if choice(ui, "filter kind", &mut synth.filter_mode, &FILTER_MODES) {
                touched.both();
            }
            ui.label(theme::text("its frequency, its resonance, its sweep: \"Filters\"", theme::SMALL - 1.0, theme::FAINT));
        });
        rows(
            ui,
            "vowel",
            vec![
                ("Peak Gain", &mut synth.filter_gain, -24.0..=24.0, " dB", "the peak: its frequency raised or dug out"),
                (
                    "Vowel",
                    &mut synth.formant,
                    0.0..=5.0,
                    "",
                    "0: none; 1 a, 2 e, 3 i, 4 o, 5 oo, and in between: the sound speaks (move it with a route: \"wah-wah\")",
                ),
                ("Vowel Mix", &mut synth.formant_mix, 0.0..=1.0, "", "how much it is heard"),
            ],
            touched,
        );
        let vowel = synth.formant.round().clamp(0.0, 5.0) as usize;
        ui.label(theme::text(format!("vowel: {}", VOWELS[vowel]), theme::SMALL - 1.0, theme::FAINT));
    });
}

/// Free modulation: two LFOs, a second envelope, a curve drawn, the routes from a source to what it moves.
fn free_modulation(ui: &mut egui::Ui, synth: &mut Synth, touched: &mut Touched) {
    egui::CollapsingHeader::new(RichText::new("Free Modulation").strong()).id_salt("free modulation").default_open(false).show(ui, |ui| {
        ui.columns(2, |columns| {
            for (at, (shape, hz)) in [(&mut synth.lfo1_shape, &mut synth.lfo1_hz), (&mut synth.lfo2_shape, &mut synth.lfo2_hz)].into_iter().enumerate() {
                columns[0].horizontal(|ui| {
                    ui.label(theme::text(format!("LFO {}", at + 1), theme::SMALL, theme::WEAK));
                    if choice(ui, ("lfo shape", at), shape, &LFO_SHAPES) {
                        touched.both();
                    }
                    touched.dragged(&ui.add(egui::DragValue::new(hz).speed(0.05).range(0.0..=200.0).suffix(" Hz")).on_hover_text("how fast it comes and goes"));
                });
            }
            columns[0].label(theme::text("Random: a number drawn for each sound (the variants differ by it)", theme::SMALL - 1.0, theme::FAINT));
            rows(
                &mut columns[1],
                "envelope 2",
                vec![
                    ("Env. 2 Attack", &mut synth.menv_attack, 0.0..=3.0, " s", "the second envelope rises from 0 to 1"),
                    ("Env. 2 Decay", &mut synth.menv_decay, 0.0..=3.0, " s", "then falls…"),
                    ("Env. 2 Level", &mut synth.menv_sustain, 0.0..=1.0, "", "…down to this level, held"),
                ],
                touched,
            );
        });
        ui.label(theme::text("Curve: drawn along the sound, from -1 to 1 (a source for the routes)", theme::SMALL, theme::WEAK));
        touched.add(steps_editor(ui, &mut synth.curve, synth.mods.iter().any(|route| route[0].round() as i32 == 4)));
        ui.horizontal(|ui| {
            for (name, made) in curve_models() {
                if ui.small_button(name).clicked() {
                    synth.curve = made;
                    touched.both();
                }
            }
        });
        ui.add_space(4.0);
        routes(ui, synth, touched);
    });
}

/// The routes: a source, what it moves, how much (negative: the other way).
fn routes(ui: &mut egui::Ui, synth: &mut Synth, touched: &mut Touched) {
    ui.label(theme::text("Routes: a source, what it moves, how much (negative: the other way)", theme::SMALL, theme::WEAK));
    let targets: Vec<&str> = MOD_TARGETS.iter().map(|(name, _)| *name).collect();
    egui::Grid::new("routes").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
        for (at, route) in synth.mods.iter_mut().enumerate() {
            ui.label(theme::text(format!("{}", at + 1), theme::SMALL, theme::FAINT));
            let [source, target, amount] = route;
            if choice(ui, ("source", at), source, &MOD_SOURCES) {
                touched.both();
            }
            if choice(ui, ("target", at), target, &targets) {
                touched.both();
            }
            let unit = MOD_TARGETS[(target.round().max(0.0) as usize).min(MOD_TARGETS.len() - 1)].1;
            touched.dragged(&ui.add(egui::DragValue::new(amount).speed(0.02).max_decimals(2)).on_hover_text(unit));
            ui.end_row();
        }
    });
}

/// Another layer as a source: the one that bends or multiplies this one.
fn routing(ui: &mut egui::Ui, synth: &mut Synth, names: &[String], touched: &mut Touched) {
    egui::CollapsingHeader::new(RichText::new("Routing: Another Layer").strong()).id_salt("routing").default_open(false).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(theme::text("Modulated by", theme::SMALL, theme::WEAK));
            let shown: Vec<&str> = names.iter().map(String::as_str).collect();
            if choice(ui, "modulated by", &mut synth.from_voice, &shown) {
                touched.both();
            }
            ui.label(theme::text("(itself: nothing); its wave, as it is, is the source", theme::SMALL - 1.0, theme::FAINT));
        });
        rows(
            ui,
            "routing",
            vec![
                ("FM by It", &mut synth.from_fm, 0.0..=20.0, "", "the other layer bends this one: FM timbres of several operators"),
                ("Ring by It", &mut synth.from_ring, 0.0..=1.0, "", "this one multiplied by the other"),
            ],
            touched,
        );
    });
}

/// The score (piano roll): its tempo, made of the simple notes, an octave up or down, doubled, cleared; the notes drawn.
fn score(ui: &mut egui::Ui, voice: &mut Voice, mix: &mut Mix, place: usize, touched: &mut Touched) {
    egui::CollapsingHeader::new(RichText::new("Score (Piano Roll)").strong()).id_salt("score").default_open(false).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(theme::text("Tempo", theme::SMALL, theme::WEAK));
            touched.dragged(&ui.add(egui::DragValue::new(&mut mix.tempo).speed(0.5).range(20.0..=400.0).suffix(" beats/min")));
            ui.separator();
            let mut changed = false;
            if ui.small_button("Simple Notes").on_hover_text("the score made of the layer's \"Notes\"").clicked() {
                voice.notes = score_of_notes(&voice.synth, mix.tempo);
                changed = true;
            }
            if ui.small_button("Octave +").clicked() {
                voice.notes.iter_mut().for_each(|note| note.tone += 12.0);
                changed = true;
            }
            if ui.small_button("Octave −").clicked() {
                voice.notes.iter_mut().for_each(|note| note.tone -= 12.0);
                changed = true;
            }
            if ui.small_button("Duplicate").on_hover_text("the score again, after itself").clicked() {
                score_doubled(&mut voice.notes);
                changed = true;
            }
            if ui.small_button("Clear").clicked() {
                voice.notes.clear();
                changed = true;
            }
            touched.add((changed, changed));
        });
        ui.label(theme::text(
            "click: a note; drag: move it; its right edge: its length; right-click: remove it. Each note plays the layer, its pitch shifted, held its length; empty: the layer played once",
            theme::SMALL - 1.0,
            theme::FAINT,
        ));
        touched.add(piano_roll(ui, &mut voice.notes, egui::Id::new(("score", place)), 16.0, false, None));
    });
}

/// The sound as text: anything written, read back ("Apply": the sound asked, given back).
fn sound_text(ui: &mut egui::Ui, making: &mut SoundMaking) -> Option<(Sound, &'static str)> {
    let mut versioned = None;
    egui::CollapsingHeader::new(RichText::new("The Sound's Text").strong()).id_salt("sound text").default_open(false).show(ui, |ui| {
        ui.label(theme::text("every setting of every layer, written; changed here then \"Apply\" (with no bounds at all)", theme::SMALL - 1.0, theme::FAINT));
        let text = making.text.get_or_insert_with(|| making.sound.written());
        ui.add(egui::TextEdit::multiline(text).code_editor().desired_rows(10).desired_width(f32::INFINITY));
        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                versioned = Some((Sound::read(text), "text"));
            }
            if ui.button("Reread the Sound").on_hover_text("the text made again from the sound").clicked() {
                *text = making.sound.written();
            }
        });
    });
    versioned
}

/// The mix: what is done to all the layers together (reversed, normalized, echo, room, speed, fades, the bands, compression, width).
fn mix_settings(ui: &mut egui::Ui, mix: &mut Mix, touched: &mut Touched) {
    egui::CollapsingHeader::new(RichText::new("Mix (All Layers)").strong()).id_salt("mix").default_open(true).show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut mix.reverse, "Reverse").changed() {
                touched.both();
            }
            if ui.checkbox(&mut mix.normalize, "Normalize").on_hover_text("as loud as it can be without clipping (-1 dB)").changed() {
                touched.both();
            }
        });
        ui.columns(2, |columns| {
            rows(
                &mut columns[0],
                "mix 1",
                vec![
                    ("Echo", &mut mix.echo, 0.02..=1.0, " s", "the time before it comes back"),
                    ("Feedback", &mut mix.feedback, 0.0..=0.95, "", "how much each echo comes back again"),
                    ("Echo Mix", &mut mix.echo_mix, 0.0..=1.0, "", "how much it is heard (0: no echo)"),
                    ("Speed", &mut mix.speed, 0.25..=4.0, "×", "faster and higher, or slower and lower"),
                ],
                touched,
            );
            rows(
                &mut columns[1],
                "mix 2",
                vec![
                    ("Room", &mut mix.room, 0.0..=1.0, "", "the size of the place (reverb)"),
                    ("Room Mix", &mut mix.room_mix, 0.0..=1.0, "", "how much it is heard (0: none)"),
                    ("Fade In", &mut mix.fade_in, 0.0..=2.0, " s", "it comes in softly"),
                    ("Fade Out", &mut mix.fade_out, 0.0..=2.0, " s", "it goes out softly"),
                ],
                touched,
            );
        });
        ui.horizontal(|ui| {
            if ui.checkbox(&mut mix.ping_pong, "Ping-pong Echo").on_hover_text("each echo on the other side").changed() {
                touched.both();
            }
            if ui.checkbox(&mut mix.limit, "Limiter").on_hover_text("never above -1 dB: it no longer clips").changed() {
                touched.both();
            }
        });
        ui.columns(2, |columns| {
            rows(
                &mut columns[0],
                "mix 3",
                vec![
                    ("Lows", &mut mix.low, -18.0..=18.0, " dB", "below 200 Hz, raised or lowered"),
                    ("Mids", &mut mix.middle, -18.0..=18.0, " dB", "around 1000 Hz"),
                    ("Highs", &mut mix.high, -18.0..=18.0, " dB", "above 5000 Hz"),
                ],
                touched,
            );
            rows(
                &mut columns[1],
                "mix 4",
                vec![
                    ("Compression", &mut mix.compress, 0.0..=1.0, "", "what is loud brought down, all raised: denser, more present"),
                    ("Width", &mut mix.width, 0.0..=2.0, "", "0: in the middle; 1: as it is; 2: very wide"),
                ],
                touched,
            );
        });
    });
}
