//! Sounds of our own: `.wav` files made for the films, beside the game's
//! (`library/sounds/<name>.wav`; in a packet, `data/hand/sounds/`), read
//! once, at the rate the speakers play, in two channels.

use std::path::{Path, PathBuf};

use assets::project::Project;

use crate::audio::OUTPUT_RATE;

/// Where a sound of our own is: in the data's hand layer (a packet has it
/// there), else among the project's sounds (then the repository's).
pub fn path(folder: &str, project: &Project, name: &str) -> PathBuf {
    let file = format!("{name}.wav");
    let hand = Path::new(folder).join("hand/sounds").join(&file);
    if hand.exists() { hand } else { project.find("sounds", &file) }
}

/// A sound of ours: its samples, left and right one after the other, at the
/// speakers' rate (none: no such file, or not one read here).
pub fn read(folder: &str, project: &Project, name: &str) -> Option<Vec<i16>> {
    let wave = Wave::read(&std::fs::read(path(folder, project, name)).ok()?)?;
    Some(wave.stereo_at(OUTPUT_RATE))
}

/// How long a sound of ours lasts, in seconds, from its file's head alone.
pub fn seconds(folder: &str, project: &Project, name: &str) -> Option<f32> {
    let wave = Wave::read(&std::fs::read(path(folder, project, name)).ok()?)?;
    Some(wave.frames() as f32 / wave.rate as f32)
}

/// The names of our own sounds in a folder (`library/sounds`), in order.
pub fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|file| file.file_name().to_string_lossy().strip_suffix(".wav").map(str::to_string))
        .collect();
    names.sort();
    names
}

/// A WAVE file as read: its samples, -1 to 1, its channels one after the
/// other.
#[derive(Debug, PartialEq)]
pub struct Wave {
    pub channels: usize,
    pub rate: u32,
    pub samples: Vec<f32>,
}

impl Wave {
    /// Plain PCM of 8, 16, 24 or 32 bits, or floats of 32; any rate, any
    /// number of channels.
    pub fn read(bytes: &[u8]) -> Option<Wave> {
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return None;
        }
        let (mut format, mut data) = (None, None);
        let mut at = 12;
        while at + 8 <= bytes.len() {
            let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
            let body = &bytes[at + 8..(at + 8 + size).min(bytes.len())];
            match &bytes[at..at + 4] {
                b"fmt " if body.len() >= 16 => format = Some(body),
                b"data" => data = Some(body),
                _ => {}
            }
            // (Chunks are kept to even sizes.)
            at += 8 + size + size % 2;
        }
        let (format, data) = (format?, data?);
        let word = |at: usize| u16::from_le_bytes([format[at], format[at + 1]]);
        let mut kind = word(0);
        // (Extensible: its kind is the first two bytes of its sub-format.)
        if kind == 0xFFFE && format.len() >= 26 {
            kind = word(24);
        }
        let (channels, rate, bits) = (word(2) as usize, u32::from_le_bytes(format[4..8].try_into().ok()?), word(14));
        if channels == 0 || rate == 0 {
            return None;
        }
        let samples: Vec<f32> = match (kind, bits) {
            (1, 8) => data.iter().map(|byte| (*byte as f32 - 128.0) / 128.0).collect(),
            (1, 16) => data.as_chunks::<2>().0.iter().map(|pair| i16::from_le_bytes(*pair) as f32 / 32768.0).collect(),
            (1, 24) => data.as_chunks::<3>().0.iter().map(|three| i32::from_le_bytes([0, three[0], three[1], three[2]]) as f32 / 2_147_483_648.0).collect(),
            (1, 32) => data.as_chunks::<4>().0.iter().map(|four| i32::from_le_bytes(*four) as f32 / 2_147_483_648.0).collect(),
            (3, 32) => data.as_chunks::<4>().0.iter().map(|four| f32::from_le_bytes(*four)).collect(),
            _ => return None,
        };
        Some(Wave { channels, rate, samples })
    }

    /// How many samples each channel has.
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels
    }

    /// In two channels (one: heard in both; more: the first two), at that
    /// rate, left and right one after the other.
    pub fn stereo_at(&self, rate: u32) -> Vec<i16> {
        let frames = self.frames();
        let side = |frame: usize, right: bool| self.samples[frame * self.channels + usize::from(right && self.channels > 1)];
        let count = (frames as u64 * rate as u64 / self.rate as u64) as usize;
        let mut out = Vec::with_capacity(count * 2);
        for at in 0..count {
            // (Between the two samples it falls between.)
            let from = at as f64 * self.rate as f64 / rate as f64;
            let (first, part) = (from as usize, (from.fract()) as f32);
            let next = (first + 1).min(frames - 1);
            for right in [false, true] {
                let value = side(first, right) * (1.0 - part) + side(next, right) * part;
                out.push((value.clamp(-1.0, 1.0) * 32767.0) as i16);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A WAVE file of 16 bits.
    fn wave(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
        let size = samples.len() as u32 * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + size).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 2 * channels as u32).to_le_bytes());
        bytes.extend_from_slice(&(2 * channels).to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&size.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn a_mono_file_is_heard_in_both_ears_at_the_speakers_rate() {
        let read = Wave::read(&wave(1, 16_000, &[0, 16384, 32767, 0])).expect("read");
        assert_eq!((read.channels, read.rate, read.frames()), (1, 16_000, 4));
        let stereo = read.stereo_at(32_000);
        assert_eq!(stereo.len(), 16);
        assert_eq!(stereo[2], stereo[3]);
        // (Halfway between 0 and a half.)
        assert!((stereo[2] - 8191).abs() < 4, "{}", stereo[2]);
    }

    #[test]
    fn a_stereo_file_keeps_its_sides_and_its_length() {
        let read = Wave::read(&wave(2, 32_000, &[1000, -1000, 2000, -2000])).expect("read");
        assert_eq!(read.frames(), 2);
        let stereo = read.stereo_at(32_000);
        assert!(stereo[0] > 0 && stereo[1] < 0);
        assert!(Wave::read(b"RIFF....WAVE").is_none());
    }

    #[test]
    fn a_sound_of_ours_is_looked_for_in_the_hand_layer_first() {
        let path = path("data", &assets::testing::library(), "soft_wind");
        assert!(path.ends_with("library/sounds/soft_wind.wav"), "{}", path.display());
    }
}
