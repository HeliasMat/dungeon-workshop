//! Sounds made by kind and by chance, and moved by a beginner's words.

use super::{Mix, Note, Sound, Synth, Voice};
use crate::making::Chance;

/// The kinds of sounds made by chance, as sfxr makes them.
pub(super) const KINDS: [&str; 9] = ["Coin", "Laser", "Explosion", "Power-up", "Hit", "Jump", "Blip", "Magic", "Step"];

/// The kinds a beginner chooses among, by what they are for: sfxr's and more (several voices some).
pub(super) const EASY_KINDS: [(&str, &[&str]); 5] = [
    ("Game", &["Coin", "Jump", "Hit", "Explosion", "Laser", "Power-up", "Step"]),
    ("Menus", &["Blip", "Confirm", "Error", "Victory", "Alarm"]),
    ("Nature", &["Splash", "Wind", "Bell", "Cry"]),
    ("Magic", &["Magic", "Twinkle", "Teleport"]),
    ("Synth", &["Super Saw", "String", "Robot Voice", "Wobble", "Melody", "Pad"]),
];

/// A sound of that kind, made by chance (the old games' kinds, and some of several voices).
pub(super) fn preset(kind: &str, seed: u32) -> Sound {
    if KINDS.contains(&kind) {
        return Sound::of(Synth::of_kind(kind, seed));
    }
    let mut chance = Chance::new(seed);
    let voice = |name: &str, synth: Synth| Voice::of(name, synth.tidy());
    let base = Synth { seed, ..Synth::default() };
    let voices = match kind {
        "Bell" => {
            let pitch = chance.between(500.0, 1400.0);
            let body = Synth { wave: 2, harmonics: 3.0, pitch, sustain: 0.02, decay: chance.between(0.6, 1.4), punch: 0.4, ..base };
            let shine = Synth { wave: 2, pitch: pitch * 2.76, sustain: 0.0, decay: 0.5, volume: 0.3, ..base };
            vec![voice("bell", body), voice("shine", shine)]
        }
        "Splash" => {
            let drop = Synth { wave: 2, pitch: chance.between(250.0, 450.0), slide: chance.between(3.0, 6.0), sustain: 0.04, decay: 0.12, ..base };
            let splash = Synth { wave: 4, pitch: 2000.0, sustain: 0.02, decay: chance.between(0.1, 0.25), lowpass: 2500.0, volume: 0.35, ..base };
            vec![voice("drop", drop), Voice { offset: 0.03, ..voice("splash", splash) }]
        }
        "Alarm" => vec![voice(
            "alarm",
            Synth {
                wave: 0,
                duty: 0.4,
                pitch: chance.between(700.0, 1100.0),
                vibrato: 0.5,
                vibrato_hz: chance.between(3.0, 7.0),
                sustain: 0.6,
                decay: 0.05,
                ..base
            },
        )],
        "Error" => {
            let low = Synth { wave: 0, duty: 0.5, pitch: chance.between(150.0, 220.0), sustain: 0.18, decay: 0.06, volume: 0.45, ..base };
            vec![voice("low", low), voice("beat", Synth { pitch: low.pitch * 1.06, ..low })]
        }
        "Confirm" => {
            vec![voice("yes", Synth { wave: 3, pitch: chance.between(600.0, 900.0), arp_mult: 1.5, arp_time: 0.06, sustain: 0.1, decay: 0.12, ..base })]
        }
        "Victory" => {
            let step = chance.between(0.08, 0.12);
            vec![voice(
                "fanfare",
                Synth {
                    wave: 0,
                    duty: 0.3,
                    pitch: chance.between(440.0, 600.0),
                    notes_len: 4.0,
                    note_time: step,
                    notes: [0.0, 4.0, 7.0, 12.0, 0.0, 0.0, 0.0, 0.0],
                    sustain: step * 4.0,
                    decay: 0.35,
                    punch: 0.3,
                    ..base
                },
            )]
        }
        "Wind" => vec![voice(
            "wind",
            Synth {
                wave: 5,
                pitch: 1200.0,
                attack: 0.4,
                sustain: chance.between(0.6, 1.2),
                decay: 0.6,
                lowpass: chance.between(500.0, 1200.0),
                lowpass_sweep: chance.between(-0.8, 0.8),
                resonance: 0.6,
                volume: 0.7,
                ..base
            },
        )],
        "Cry" => vec![voice(
            "cry",
            Synth {
                wave: 1,
                pitch: chance.between(450.0, 800.0),
                slide: -chance.between(0.5, 1.5),
                vibrato: 0.3,
                vibrato_hz: chance.between(7.0, 11.0),
                attack: 0.02,
                sustain: 0.25,
                decay: 0.15,
                lowpass: 5000.0,
                ..base
            },
        )],
        "Twinkle" => vec![voice(
            "sparkles",
            Synth {
                wave: 2,
                harmonics: 2.0,
                pitch: chance.between(1200.0, 2000.0),
                notes_len: 6.0,
                note_time: 0.05,
                notes: [0.0, 7.0, 12.0, 19.0, 12.0, 24.0, 0.0, 0.0],
                sustain: 0.3,
                decay: 0.3,
                ..base
            },
        )],
        "Teleport" => vec![voice(
            "teleport",
            Synth {
                wave: 2,
                pitch: chance.between(200.0, 400.0),
                slide: 5.0,
                fm_ratio: 3.0,
                fm_depth: chance.between(1.0, 3.0),
                sustain: 0.3,
                decay: 0.2,
                flanger: 2.0,
                flanger_sweep: 4.0,
                ..base
            },
        )],
        "Super Saw" => vec![voice(
            "super saw",
            Synth {
                wave: 1,
                pitch: chance.between(180.0, 300.0),
                unison: 7.0,
                detune: chance.between(18.0, 35.0),
                spread: 0.9,
                attack: 0.03,
                sustain: 0.7,
                decay: 0.5,
                lowpass: 4000.0,
                chorus: 0.3,
                volume: 0.5,
                ..base
            },
        )],
        "String" => vec![voice(
            "string",
            Synth {
                wave: 8,
                pitch: chance.between(180.0, 520.0),
                sustain: 1.4,
                decay: 0.3,
                string_damp: chance.between(0.05, 0.4),
                string_bright: chance.between(0.3, 0.8),
                volume: 0.8,
                ..base
            },
        )],
        "Robot Voice" => {
            let mut mods = [[0.0; 3]; 6];
            mods[0] = [1.0, 9.0, 2.0];
            vec![voice(
                "voice",
                Synth {
                    wave: 1,
                    pitch: chance.between(100.0, 180.0),
                    formant: 3.0,
                    formant_mix: 1.0,
                    lfo1_shape: 4.0,
                    lfo1_hz: chance.between(3.0, 7.0),
                    mods,
                    ring_hz: 40.0,
                    ring_mix: 0.25,
                    sustain: 1.0,
                    decay: 0.2,
                    volume: 0.8,
                    ..base
                },
            )]
        }
        "Wobble" => {
            let mut mods = [[0.0; 3]; 6];
            mods[0] = [1.0, 2.0, 2.5];
            vec![voice(
                "wobble",
                Synth {
                    wave: 1,
                    pitch: chance.between(45.0, 70.0),
                    unison: 3.0,
                    detune: 10.0,
                    spread: 0.3,
                    lowpass: 380.0,
                    resonance: 0.7,
                    lfo1_hz: [2.0, 4.0, 6.0][(chance.next() * 3.0) as usize % 3],
                    mods,
                    drive: 0.35,
                    sustain: 1.6,
                    decay: 0.2,
                    volume: 0.6,
                    ..base
                },
            )]
        }
        "Melody" => {
            let mut melody =
                Voice::of("melody", Synth { wave: 0, duty: 0.25, pitch: 523.25, attack: 0.005, decay: 0.08, punch: 0.2, volume: 0.5, ..base }.tidy());
            let tones = [0.0, 4.0, 7.0, 12.0, 7.0, 4.0, 0.0, 7.0];
            melody.notes = tones
                .iter()
                .enumerate()
                .map(|(at, tone)| Note { start: at as f32 * 0.5, length: 0.45, tone: *tone, velocity: if at % 2 == 0 { 1.0 } else { 0.7 } })
                .collect();
            let mut bass = Voice::of("bass", Synth { wave: 3, pitch: 130.81, attack: 0.005, decay: 0.1, volume: 0.6, ..base }.tidy());
            bass.notes = [0.0, 5.0, 7.0, 0.0].iter().enumerate().map(|(at, tone)| Note { start: at as f32, length: 0.9, tone: *tone, velocity: 0.9 }).collect();
            return Sound {
                voices: vec![melody, bass],
                mix: Mix { tempo: chance.between(120.0, 160.0).round(), echo_mix: 0.2, echo: 0.21, ping_pong: true, ..Mix::default() },
            };
        }
        "Pad" => {
            let mut mods = [[0.0; 3]; 6];
            mods[0] = [2.0, 2.0, 1.2];
            vec![voice(
                "pad",
                Synth {
                    wave: 1,
                    pitch: chance.between(160.0, 260.0),
                    unison: 5.0,
                    detune: 12.0,
                    spread: 1.0,
                    attack: 0.8,
                    sustain: 1.6,
                    decay: 1.4,
                    lowpass: 1600.0,
                    lfo2_shape: 1.0,
                    lfo2_hz: 0.3,
                    mods,
                    chorus: 0.5,
                    volume: 0.5,
                    ..base
                },
            )]
        }
        _ => vec![voice("Layer 1", Synth::of_kind("Blip", seed))],
    };
    let room_mix = match kind {
        "Bell" => 0.25,
        "Pad" | "String" => 0.4,
        _ => 0.0,
    };
    Sound { voices, mix: Mix { room_mix, ..Mix::default() } }
}

/// The words a beginner moves a sound with: each from one side (-1) to the other (1).
pub(super) const EASY: [(&str, &str, &str); 7] = [
    ("Low", "High", "lower or higher"),
    ("Short", "Long", "shorter or longer"),
    ("Soft", "Hard", "coming in softly, or striking"),
    ("Clean", "Worn", "clear, or crackling like an old game"),
    ("Falls", "Rises", "the note falling, or rising"),
    ("Dry", "Distant", "close by, or in a large hall"),
    ("Quiet", "Loud", "the volume"),
];

/// A sound moved by a beginner's words (each from -1 to 1, 0: as it was).
pub(super) fn eased(base: &Sound, words: &[f32; 7]) -> Sound {
    let [pitch, length, hardness, dirt, motion, space, loud] = *words;
    let mut sound = base.clone();
    let stretch = 2f32.powf(length * 1.3);
    for voice in &mut sound.voices {
        let synth = &mut voice.synth;
        synth.pitch = (synth.pitch * 2f32.powf(pitch * 1.5)).clamp(20.0, 8000.0);
        synth.min_pitch *= 2f32.powf(pitch * 1.5);
        for time in [&mut synth.attack, &mut synth.sustain, &mut synth.decay, &mut synth.arp_time, &mut synth.repeat, &mut synth.note_time] {
            *time *= stretch;
        }
        synth.slide /= stretch;
        synth.delta_slide /= stretch * stretch;
        voice.offset *= stretch;
        if hardness > 0.0 {
            synth.punch = (synth.punch + hardness * 0.6).min(1.0);
            synth.attack *= 1.0 - hardness * 0.95;
        } else {
            synth.attack += -hardness * 0.12;
            synth.lowpass = synth.lowpass.min(16000.0 * 2f32.powf(hardness * 4.0));
            synth.punch *= 1.0 + hardness;
        }
        if dirt > 0.0 {
            synth.crush = 12000.0 * 2f32.powf(-dirt * 3.5);
            synth.drive = synth.drive.max(dirt * 0.6);
        } else {
            synth.lowpass = synth.lowpass.min(16000.0 * 2f32.powf(dirt * 2.5));
        }
        synth.slide += motion * 5.0;
        voice.gain *= 2f32.powf(loud);
        *synth = synth.tidy();
    }
    if space > 0.0 {
        sound.mix.room_mix = sound.mix.room_mix.max(space * 0.6);
        sound.mix.room = 0.4 + space * 0.6;
        sound.mix.echo_mix = sound.mix.echo_mix.max(space * 0.25);
    } else {
        sound.mix.room_mix *= 1.0 + space;
        sound.mix.echo_mix *= 1.0 + space;
    }
    sound
}
