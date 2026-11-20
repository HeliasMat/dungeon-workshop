//! The mix: echo, room, speed, fades, loudness; written, read, edited; a
//! note or a hit played through it.

use std::collections::HashMap;

use eframe::egui;

use super::render::Biquad;
use super::{Mix, Note, Sound};
use crate::theme;

/// The mix's settings by their names (its switches apart).
pub(super) fn mix_fields(mix: &mut Mix) -> [(&'static str, &mut f32); 14] {
    [
        ("mix.echo", &mut mix.echo),
        ("mix.echo_feedback", &mut mix.feedback),
        ("mix.echo_mix", &mut mix.echo_mix),
        ("mix.room", &mut mix.room),
        ("mix.room_mix", &mut mix.room_mix),
        ("mix.speed", &mut mix.speed),
        ("mix.fade_in", &mut mix.fade_in),
        ("mix.fade_out", &mut mix.fade_out),
        ("mix.tempo", &mut mix.tempo),
        ("mix.low", &mut mix.low),
        ("mix.mid", &mut mix.middle),
        ("mix.high", &mut mix.high),
        ("mix.compression", &mut mix.compress),
        ("mix.width", &mut mix.width),
    ]
}

/// What a mix does to two channels: faster or slower, backwards, an echo, a room, the equaliser, what does not
/// move taken off, compressed, wider, faded, made loud, limited (a sound's, a track's, a song's master).
pub(crate) fn mixed(mix: &Mix, mut left: Vec<f32>, mut right: Vec<f32>) -> (Vec<f32>, Vec<f32>) {
    let rate = shell::audio::OUTPUT_RATE as f32;
    // (Faster or slower: its samples taken further apart or closer, its pitch with it.)
    if (mix.speed - 1.0).abs() > 0.001 && left.len() > 1 {
        let made = (left.len() as f32 / mix.speed.max(0.1)) as usize;
        (left, right) = (resampled(&left, mix.speed, made), resampled(&right, mix.speed, made));
    }
    if mix.reverse {
        left.reverse();
        right.reverse();
    }
    if mix.echo_mix > 0.0 && mix.echo > 0.0 {
        echoed(mix, &mut left, &mut right, rate);
    }
    if mix.room_mix > 0.0 {
        roomed(mix, &mut left, &mut right, rate);
    }
    let equalised = mix.low != 0.0 || mix.middle != 0.0 || mix.high != 0.0;
    if equalised {
        equalise(mix, &mut left, &mut right, rate);
    }
    // (What does not move taken off (a wave lopsided, raised by the low shelf), before it is compressed.)
    if equalised || mix.compress > 0.0 || mix.limit {
        for channel in [&mut left, &mut right] {
            unmoving_taken_off(channel);
        }
    }
    if mix.compress > 0.0 {
        compress(mix, &mut left, &mut right, rate);
    }
    // (How wide, from the middle out.)
    if (mix.width - 1.0).abs() > 0.001 {
        for at in 0..left.len() {
            let (middle, side) = ((left[at] + right[at]) / 2.0, (left[at] - right[at]) / 2.0 * mix.width.max(0.0));
            (left[at], right[at]) = (middle + side, middle - side);
        }
    }
    fade(mix, &mut left, &mut right, rate);
    let peak = left.iter().chain(&right).fold(0.0_f32, |most, sample| most.max(sample.abs()));
    if mix.normalize && peak > 1e-4 {
        let gain = 0.891 / peak;
        for sample in left.iter_mut().chain(right.iter_mut()) {
            *sample *= gain;
        }
    }
    if mix.limit {
        limit(&mut left, &mut right, rate);
    }
    (left, right)
}

/// A channel faster (`speed` above 1) or slower, `made` samples long: its samples taken further apart or closer.
fn resampled(channel: &[f32], speed: f32, made: usize) -> Vec<f32> {
    (0..made)
        .map(|at| {
            let from = at as f32 * speed;
            let below = (from as usize).min(channel.len() - 1);
            let above = (below + 1).min(channel.len() - 1);
            let part = from - below as f32;
            channel[below] * (1.0 - part) + channel[above] * part
        })
        .collect()
}

/// An echo: itself again, softer each time, after a while; with ping-pong, each echo on the side the last was not.
fn echoed(mix: &Mix, left: &mut Vec<f32>, right: &mut Vec<f32>, rate: f32) {
    let delay = (mix.echo * rate).max(1.0) as usize;
    let repeats = if mix.feedback > 0.01 { (0.01_f32.ln() / mix.feedback.min(0.95).ln()).clamp(1.0, 12.0) } else { 1.0 };
    let tail = ((delay as f32 * repeats) as usize).min((4.0 * rate) as usize);
    if mix.ping_pong {
        left.resize(left.len() + tail, 0.0);
        right.resize(right.len() + tail, 0.0);
        let (dry_left, dry_right) = (left.clone(), right.clone());
        let (mut wet_left, mut wet_right) = (vec![0.0_f32; dry_left.len()], vec![0.0_f32; dry_left.len()]);
        for at in delay..dry_left.len() {
            wet_left[at] = dry_right[at - delay] + mix.feedback.min(0.95) * wet_right[at - delay];
            wet_right[at] = dry_left[at - delay] + mix.feedback.min(0.95) * wet_left[at - delay];
        }
        for at in 0..left.len() {
            left[at] += mix.echo_mix * wet_left[at];
            right[at] += mix.echo_mix * wet_right[at];
        }
        return;
    }
    for channel in [left, right] {
        channel.resize(channel.len() + tail, 0.0);
        let dry = channel.clone();
        let mut wet = vec![0.0_f32; dry.len()];
        for at in delay..dry.len() {
            wet[at] = dry[at - delay] + mix.feedback.min(0.95) * wet[at - delay];
        }
        for (sample, echo) in channel.iter_mut().zip(&wet) {
            *sample += mix.echo_mix * echo;
        }
    }
}

/// A room: four combs and two all-pass filters (Schroeder), a little apart on each side.
fn roomed(mix: &Mix, left: &mut Vec<f32>, right: &mut Vec<f32>, rate: f32) {
    let scale = 0.5 + mix.room.clamp(0.0, 1.0) * 1.1;
    let tail = ((0.3 + mix.room * 1.5) * rate) as usize;
    for (side, channel) in [left, right].into_iter().enumerate() {
        channel.resize(channel.len() + tail, 0.0);
        let dry = channel.clone();
        let mut wet = vec![0.0_f32; dry.len()];
        let spread = side as f32 * 1.3;
        let gain = 0.7 + 0.25 * mix.room.clamp(0.0, 1.0);
        for comb in [29.7_f32, 37.1, 41.1, 43.7] {
            let delay = (((comb + spread) * scale / 1000.0) * rate) as usize;
            let mut line = vec![0.0_f32; dry.len()];
            for at in 0..dry.len() {
                line[at] = dry[at] + if at >= delay { gain * line[at - delay] } else { 0.0 };
                wet[at] += line[at] * 0.25;
            }
        }
        for allpass in [5.0_f32, 1.7] {
            let delay = ((allpass / 1000.0) * rate).max(1.0) as usize;
            let input = wet.clone();
            for at in 0..wet.len() {
                let back_in = if at >= delay { input[at - delay] } else { 0.0 };
                let back_out = if at >= delay { wet[at - delay] } else { 0.0 };
                wet[at] = -0.7 * input[at] + back_in + 0.7 * back_out;
            }
        }
        for (sample, room) in channel.iter_mut().zip(&wet) {
            *sample = *sample * (1.0 - mix.room_mix * 0.5) + room * mix.room_mix;
        }
    }
}

/// Low, middle, high: two shelves and a peak.
fn equalise(mix: &Mix, left: &mut [f32], right: &mut [f32], rate: f32) {
    for channel in [left, right] {
        let mut bands = [Biquad::default(); 3];
        bands[0].set(5, 200.0, 0.707, mix.low, rate);
        bands[1].set(3, 1000.0, 0.7, mix.middle, rate);
        bands[2].set(6, 5000.0, 0.707, mix.high, rate);
        for sample in channel.iter_mut() {
            *sample = bands.iter_mut().fold(*sample, |made, band| band.run(made));
        }
    }
}

/// What does not move in a wave taken off (a high pass, very low).
fn unmoving_taken_off(channel: &mut [f32]) {
    let (mut last_in, mut last_out) = (0.0_f32, 0.0_f32);
    for sample in channel.iter_mut() {
        let made = *sample - last_in + 0.995 * last_out;
        (last_in, last_out) = (*sample, made);
        *sample = made;
    }
}

/// Compressed: what is loud brought down, all brought up after.
fn compress(mix: &Mix, left: &mut [f32], right: &mut [f32], rate: f32) {
    let threshold = -6.0 - 30.0 * mix.compress.min(1.0);
    let ratio = 1.0 + 7.0 * mix.compress.min(1.0);
    let (attack, release) = ((-1.0 / (0.003 * rate)).exp(), (-1.0 / (0.12 * rate)).exp());
    let makeup = -threshold * (1.0 - 1.0 / ratio) * 0.5;
    let mut level = 0.0_f32;
    for at in 0..left.len() {
        let now = left[at].abs().max(right[at].abs());
        level = if now > level { attack * level + (1.0 - attack) * now } else { release * level + (1.0 - release) * now };
        let decibels = 20.0 * level.max(1e-6).log10();
        let reduced = if decibels > threshold { (threshold - decibels) * (1.0 - 1.0 / ratio) } else { 0.0 };
        let gain = 10f32.powf((reduced + makeup) / 20.0);
        left[at] *= gain;
        right[at] *= gain;
    }
}

/// Coming in, going out.
fn fade(mix: &Mix, left: &mut [f32], right: &mut [f32], rate: f32) {
    let length = left.len();
    for at in 0..length {
        let time = at as f32 / rate;
        let gone = (length - at) as f32 / rate;
        let mut gain = 1.0;
        if mix.fade_in > 0.0 {
            gain *= (time / mix.fade_in).min(1.0);
        }
        if mix.fade_out > 0.0 {
            gain *= (gone / mix.fade_out).min(1.0);
        }
        left[at] *= gain;
        right[at] *= gain;
    }
}

/// Limited: never past -1 dB, brought down at once, let back slowly.
fn limit(left: &mut [f32], right: &mut [f32], rate: f32) {
    let ceiling = 0.891;
    let release = (-1.0 / (0.05 * rate)).exp();
    let mut gain = 1.0_f32;
    for at in 0..left.len() {
        let peak = left[at].abs().max(right[at].abs());
        let wanted = if peak * gain > ceiling { ceiling / peak } else { 1.0 };
        gain = if wanted < gain { wanted } else { release * gain + (1.0 - release) * wanted.min(1.0) };
        left[at] *= gain;
        right[at] *= gain;
    }
}

/// The mix written as `key=value` lines, under a prefix (`track0.fx.`).
pub(crate) fn mix_written(mix: &Mix, prefix: &str) -> String {
    let mut copy = *mix;
    let mut said = String::new();
    for (key, value) in mix_fields(&mut copy) {
        said.push_str(&format!("{prefix}{key}={value}\n"));
    }
    said.push_str(&format!(
        "{prefix}mix.reverse={}\n{prefix}mix.normalize={}\n{prefix}mix.pingpong={}\n{prefix}mix.limiter={}\n",
        u8::from(mix.reverse),
        u8::from(mix.normalize),
        u8::from(mix.ping_pong),
        u8::from(mix.limit)
    ));
    said
}

/// A mix read back from its lines under a prefix (what is not there, as it is by default).
pub(crate) fn mix_read(pairs: &HashMap<String, String>, prefix: &str) -> Mix {
    let mut mix = Mix::default();
    let number = |key: &str| pairs.get(&format!("{prefix}{key}")).and_then(|value| value.parse::<f32>().ok());
    for (key, value) in mix_fields(&mut mix) {
        if let Some(given) = number(key) {
            *value = given;
        }
    }
    mix.reverse = number("mix.reverse").is_some_and(|value| value > 0.5);
    mix.normalize = number("mix.normalize").is_some_and(|value| value > 0.5);
    mix.ping_pong = number("mix.pingpong").is_some_and(|value| value > 0.5);
    mix.limit = number("mix.limiter").is_some_and(|value| value > 0.5);
    mix
}

/// The mix's settings, to be changed (an effects chain of a track, the master): whether taken, let go.
pub(crate) fn mix_editor(ui: &mut egui::Ui, mix: &mut Mix, id: &str) -> (bool, bool) {
    let mut moved = (false, false);
    ui.horizontal(|ui| {
        for (said, value, help) in [
            ("Ping-pong", &mut mix.ping_pong, "each echo on the other side"),
            ("Limiter", &mut mix.limit, "never above -1 dB"),
            ("Normalize", &mut mix.normalize, "as loud as it can be"),
        ] {
            if ui.checkbox(value, said).on_hover_text(help).changed() {
                moved = (true, true);
            }
        }
    });
    egui::Grid::new(("effects", id)).num_columns(2).spacing([8.0, 3.0]).show(ui, |ui| {
        for (name, value, range, suffix, help) in [
            ("Lows", &mut mix.low, -18.0..=18.0, " dB", "below 200 Hz"),
            ("Mids", &mut mix.middle, -18.0..=18.0, " dB", "around 1000 Hz"),
            ("Highs", &mut mix.high, -18.0..=18.0, " dB", "above 5000 Hz"),
            ("Compression", &mut mix.compress, 0.0..=1.0, "", "denser, more present"),
            ("Echo", &mut mix.echo, 0.02..=1.0, " s", "the time before it comes back"),
            ("Feedback", &mut mix.feedback, 0.0..=0.95, "", "how much each echo comes back"),
            ("Echo Mix", &mut mix.echo_mix, 0.0..=1.0, "", "0: none"),
            ("Room", &mut mix.room, 0.0..=1.0, "", "the size of the place"),
            ("Room Mix", &mut mix.room_mix, 0.0..=1.0, "", "0: none"),
            ("Width", &mut mix.width, 0.0..=2.0, "", "1: as it is"),
        ] {
            ui.label(theme::text(name, theme::SMALL - 1.0, theme::WEAK)).on_hover_text(help);
            ui.spacing_mut().slider_width = 110.0;
            let response = ui.add(egui::Slider::new(value, range).suffix(suffix).max_decimals(2));
            moved.0 |= response.drag_started() || (response.changed() && !response.dragged());
            moved.1 |= response.drag_stopped() || (response.changed() && !response.dragged());
            ui.end_row();
        }
    });
    moved
}

/// A note of an instrument: the sound played that many half tones over its pitch, held so long (s), so hard.
pub(crate) fn note_audio(sound: &Sound, tone: f32, seconds: f32, velocity: f32, own: &super::OwnSounds) -> (Vec<f32>, Vec<f32>) {
    let mut played = sound.clone();
    played.mix.tempo = 60.0;
    for voice in &mut played.voices {
        voice.notes = vec![Note { start: 0.0, length: seconds.max(0.01), tone, velocity }];
    }
    split(&played.render(own).0)
}

/// A sound played once as it is (a drum), so hard.
pub(crate) fn one_shot(sound: &Sound, velocity: f32, own: &super::OwnSounds) -> (Vec<f32>, Vec<f32>) {
    let (mut left, mut right) = split(&sound.render(own).0);
    for sample in left.iter_mut().chain(right.iter_mut()) {
        *sample *= velocity;
    }
    (left, right)
}

/// Two channels, one after the other, apart.
fn split(stereo: &[f32]) -> (Vec<f32>, Vec<f32>) {
    (stereo.iter().step_by(2).copied().collect(), stereo.iter().skip(1).step_by(2).copied().collect())
}

#[cfg(test)]
mod tests {
    use super::{resampled, unmoving_taken_off};

    #[test]
    fn twice_as_fast_takes_every_other_sample_and_a_still_wave_fades_away() {
        assert_eq!(resampled(&[0.0, 1.0, 2.0, 3.0], 2.0, 2), vec![0.0, 2.0]);
        let mut still = vec![0.5_f32; 4000];
        unmoving_taken_off(&mut still);
        assert_eq!(still[0], 0.5);
        assert!(still[3999].abs() < 0.01);
    }
}
