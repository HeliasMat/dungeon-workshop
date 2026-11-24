//! The instruments and the drum kit music is played with.

use super::{Mix, Sound, Synth, Voice};

/// The instruments a song's track may begin from (their pitch: middle C).
pub const INSTRUMENTS: [&str; 12] =
    ["Electric Piano", "Bass", "Synth Bass", "Square Lead", "Super Saw", "Pad", "Strings", "Organ", "Bells", "Plucked String", "Flute", "Chiptune"];

/// An instrument of that kind: its synth, its pitch middle C (a note's half tones over it).
pub fn instrument(kind: &str) -> Sound {
    const C4: f32 = 261.63;
    const C2: f32 = 65.41;
    let base = Synth { pitch: C4, seed: 1, ..Synth::default() };
    let routes = |list: &[[f32; 3]]| {
        let mut mods = [[0.0; 3]; 6];
        for (at, route) in list.iter().enumerate() {
            mods[at] = *route;
        }
        mods
    };
    let mut voices = vec![];
    let synth = match kind {
        "Electric Piano" => Synth {
            wave: 2,
            fm_ratio: 1.0,
            fm_depth: 1.6,
            attack: 0.003,
            decay: 1.1,
            punch: 0.3,
            menv_decay: 0.6,
            mods: routes(&[[3.0, 5.0, 1.5]]),
            volume: 0.55,
            ..base
        },
        "Bass" => Synth { wave: 3, pitch: C2, harmonics: 1.0, attack: 0.004, decay: 0.25, lowpass: 1400.0, volume: 0.85, ..base },
        "Synth Bass" => Synth {
            wave: 1,
            pitch: C2,
            lowpass: 500.0,
            resonance: 0.5,
            menv_decay: 0.25,
            mods: routes(&[[3.0, 2.0, 2.5]]),
            drive: 0.3,
            attack: 0.003,
            decay: 0.15,
            volume: 0.6,
            ..base
        },
        "Square Lead" => Synth {
            wave: 0,
            duty: 0.3,
            unison: 2.0,
            detune: 8.0,
            spread: 0.4,
            vibrato: 0.02,
            vibrato_hz: 5.5,
            attack: 0.01,
            decay: 0.2,
            chorus: 0.2,
            volume: 0.45,
            ..base
        },
        "Super Saw" => Synth { wave: 1, unison: 7.0, detune: 25.0, spread: 0.9, attack: 0.02, decay: 0.4, lowpass: 5000.0, chorus: 0.3, volume: 0.4, ..base },
        "Pad" => Synth {
            wave: 1,
            unison: 5.0,
            detune: 12.0,
            spread: 1.0,
            attack: 0.6,
            decay: 1.2,
            lowpass: 1800.0,
            lfo2_shape: 1.0,
            lfo2_hz: 0.3,
            mods: routes(&[[2.0, 2.0, 1.0]]),
            chorus: 0.5,
            volume: 0.4,
            ..base
        },
        "Strings" => Synth {
            wave: 1,
            unison: 6.0,
            detune: 10.0,
            spread: 0.8,
            attack: 0.25,
            decay: 0.6,
            lowpass: 3000.0,
            vibrato: 0.01,
            vibrato_hz: 5.0,
            chorus: 0.4,
            volume: 0.4,
            ..base
        },
        "Organ" => Synth { wave: 2, harmonics: 5.0, attack: 0.01, decay: 0.08, chorus: 0.3, chorus_hz: 5.0, volume: 0.45, ..base },
        "Bells" => Synth { wave: 2, fm_ratio: 3.5, fm_depth: 2.2, attack: 0.002, decay: 1.8, volume: 0.45, ..base },
        "Plucked String" => Synth { wave: 8, decay: 0.3, string_damp: 0.25, string_bright: 0.6, volume: 0.8, ..base },
        "Flute" => {
            voices.push(Voice {
                gain: 0.25,
                ..Voice::of(
                    "breath",
                    Synth { wave: 5, filter_mode: 1.0, lowpass: 1800.0, resonance: 0.3, attack: 0.05, decay: 0.12, volume: 0.5, ..base }.tidy(),
                )
            });
            Synth { wave: 2, vibrato: 0.012, vibrato_hz: 5.0, attack: 0.06, decay: 0.15, volume: 0.6, ..base }
        }
        _ => Synth { wave: 0, duty: 0.25, attack: 0.0, decay: 0.05, volume: 0.4, ..base },
    };
    voices.insert(0, Voice::of(kind, synth.tidy()));
    let room_mix = match kind {
        "Pad" | "Strings" | "Bells" | "Flute" => 0.3,
        "Electric Piano" | "Organ" => 0.15,
        _ => 0.0,
    };
    Sound { voices, mix: Mix { room_mix, ..Mix::default() } }
}

/// A drum kit, synthesized: each piece by its name (a drum track's rows, from the top).
pub fn drum_kit() -> Vec<(String, Sound)> {
    let base = Synth { seed: 3, ..Synth::default() };
    let piece = |name: &str, voices: Vec<Voice>| (name.to_string(), Sound { voices, mix: Mix::default() });
    let voice = |name: &str, synth: Synth, offset: f32, gain: f32| Voice { offset, gain, ..Voice::of(name, synth.tidy()) };
    vec![
        piece(
            "Kick",
            vec![
                voice(
                    "body",
                    Synth { wave: 2, pitch: 160.0, slide: -5.0, sustain: 0.03, decay: 0.32, punch: 0.6, drive: 0.25, volume: 0.95, ..base },
                    0.0,
                    1.0,
                ),
                voice("click", Synth { wave: 4, pitch: 6000.0, sustain: 0.0, decay: 0.012, highpass: 2000.0, volume: 0.5, ..base }, 0.0, 0.6),
            ],
        ),
        piece(
            "Snare",
            vec![
                voice("skin", Synth { wave: 3, pitch: 190.0, slide: -1.0, sustain: 0.01, decay: 0.1, volume: 0.6, ..base }, 0.0, 1.0),
                voice("snare", Synth { wave: 4, pitch: 6000.0, sustain: 0.01, decay: 0.18, highpass: 1200.0, volume: 0.7, ..base }, 0.0, 1.0),
            ],
        ),
        piece(
            "Clap",
            vec![
                voice(
                    "1",
                    Synth { wave: 4, pitch: 5000.0, filter_mode: 1.0, lowpass: 1200.0, resonance: 0.3, sustain: 0.0, decay: 0.02, volume: 0.8, ..base },
                    0.0,
                    1.0,
                ),
                voice(
                    "2",
                    Synth {
                        wave: 4,
                        pitch: 5000.0,
                        filter_mode: 1.0,
                        lowpass: 1200.0,
                        resonance: 0.3,
                        sustain: 0.0,
                        decay: 0.02,
                        volume: 0.8,
                        seed: 4,
                        ..base
                    },
                    0.012,
                    1.0,
                ),
                voice(
                    "tail",
                    Synth {
                        wave: 4,
                        pitch: 5000.0,
                        filter_mode: 1.0,
                        lowpass: 1200.0,
                        resonance: 0.3,
                        sustain: 0.0,
                        decay: 0.18,
                        volume: 0.8,
                        seed: 5,
                        ..base
                    },
                    0.024,
                    1.0,
                ),
            ],
        ),
        piece(
            "Closed Hi-hat",
            vec![voice("hi-hat", Synth { wave: 4, pitch: 9000.0, highpass: 7000.0, sustain: 0.0, decay: 0.045, volume: 0.5, ..base }, 0.0, 1.0)],
        ),
        piece(
            "Open Hi-hat",
            vec![voice("hi-hat", Synth { wave: 4, pitch: 9000.0, highpass: 6000.0, sustain: 0.02, decay: 0.35, volume: 0.45, ..base }, 0.0, 1.0)],
        ),
        piece(
            "Low Tom",
            vec![voice("tom", Synth { wave: 2, pitch: 120.0, slide: -1.2, sustain: 0.02, decay: 0.35, punch: 0.3, volume: 0.8, ..base }, 0.0, 1.0)],
        ),
        piece(
            "High Tom",
            vec![voice("tom", Synth { wave: 2, pitch: 200.0, slide: -1.2, sustain: 0.02, decay: 0.3, punch: 0.3, volume: 0.8, ..base }, 0.0, 1.0)],
        ),
        piece(
            "Cymbal",
            vec![voice(
                "cymbal",
                Synth { wave: 4, pitch: 9000.0, highpass: 4500.0, sustain: 0.02, decay: 1.4, ring_hz: 380.0, ring_mix: 0.3, volume: 0.45, ..base },
                0.0,
                1.0,
            )],
        ),
        piece(
            "Cowbell",
            vec![
                voice("high", Synth { wave: 0, pitch: 800.0, sustain: 0.0, decay: 0.12, filter_mode: 1.0, lowpass: 900.0, volume: 0.5, ..base }, 0.0, 1.0),
                voice("low", Synth { wave: 0, pitch: 540.0, sustain: 0.0, decay: 0.12, filter_mode: 1.0, lowpass: 900.0, volume: 0.5, ..base }, 0.0, 1.0),
            ],
        ),
        piece(
            "Rim",
            vec![
                voice("wood", Synth { wave: 3, pitch: 1700.0, sustain: 0.0, decay: 0.03, volume: 0.6, ..base }, 0.0, 1.0),
                voice("noise", Synth { wave: 4, pitch: 7000.0, sustain: 0.0, decay: 0.02, highpass: 3000.0, volume: 0.4, ..base }, 0.0, 1.0),
            ],
        ),
    ]
}
