//! The project's own music: songs (notes, pauses, controller changes on
//! tracks), banks (instruments made of key splits, and their samples),
//! and the library of every song, short tune and sound of the game
//! (`Library`), as written once by the extraction (`data/rom/sound.bin`) and
//! played by the synthesizer (`crate::audio`). Nothing here is read from
//! the ROM: `rom_formats::smdl`, `rom_formats::swdl` and `rom_formats::sedl` read the game's files
//! into it.

use std::collections::HashMap;

use serde::{Deserialize, Serialize, Serializer};

/// A map written in the order of its keys: the same library is always the
/// same bytes (a `HashMap` goes through its keys in no fixed order).
fn sorted<K: Ord + Serialize, V: Serialize, S: Serializer>(map: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_map(map.iter().collect::<std::collections::BTreeMap<_, _>>())
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Note {
        /// Semitones, `octave * 12 + note`.
        key: u8,
        velocity: u8,
        /// How long the key is held, in ticks.
        duration: u32,
    },
    Tempo(u8),
    Program(u8),
    Volume(u8),
    Expression(u8),
    Pan(u8),
    /// The bank the track's programs are taken from: the two bytes of its
    /// number, either or both.
    Bank {
        high: Option<u8>,
        low: Option<u8>,
    },
    /// The pitch bent: 8192 is the whole of the range.
    Bend(i16),
    /// The range of bends, in semitones.
    BendRange(u8),
    /// The track tuned by semitones, at once or reached over some ticks.
    Tune {
        semitones: i8,
        ticks: u16,
    },
    /// The track's volume reached over some ticks.
    VolumeSweep {
        volume: u8,
        ticks: u16,
    },
    /// A part of the volume envelope of the notes to come: the event's
    /// number less 0xB0 (0 none, 1 first level, 2 attack, 3 hold, 4 decay,
    /// 5 fade, 6 release) and 7 for the level held.
    Envelope {
        part: u8,
        value: u8,
    },
    /// Where playback resumes after the end of the track.
    LoopPoint,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub channel: u8,
    /// `(tick, event)`, in playing order.
    pub events: Vec<(u32, Event)>,
    pub length: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub ticks_per_quarter: u16,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleFormat {
    Pcm8,
    Pcm16,
    /// 4-bit IMA ADPCM with a 4-byte header.
    Adpcm,
    /// The console's own square wave: no sample data.
    Psg,
    /// The console's own noise: no sample data.
    Noise,
}

/// How loud a note is as it goes: from a first level up to the whole in
/// `attack`, held there for `hold`, down to the `sustain` level in `decay`,
/// fading from there to nothing in `fade`, and gone in `release` once the
/// note is let go. Levels out of 127, times in seconds (`None`: never).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub on: bool,
    pub multiplier: u8,
    pub first: u8,
    pub attack: u8,
    pub hold: u8,
    pub decay: u8,
    pub sustain: u8,
    pub fade: u8,
    pub release: u8,
}

impl Envelope {
    /// The seconds a time of the envelope stands for (`None`: for ever).
    pub fn seconds(&self, time: u8) -> Option<f32> {
        let time = time.min(127) as usize;
        if time == 127 {
            return None;
        }
        Some(match self.multiplier {
            0 => ENVELOPE_LONG_TIMES[time] as f32 / 1000.0,
            by => ENVELOPE_TIMES[time] as f32 * by as f32 / 1000.0,
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SampleInfo {
    pub id: u16,
    pub format: SampleFormat,
    pub looped: bool,
    pub rate: u32,
    /// Offset of the data in the "pcmd" chunk.
    pub position: usize,
    /// Start and length of the loop, in bytes.
    pub loop_start: usize,
    pub loop_length: usize,
    /// Tuning, in semitones and cents.
    pub coarse_tune: i8,
    pub fine_tune: i8,
    pub root_key: u8,
    pub volume: i8,
    /// Of the console's square wave: the eighths of its time it is up,
    /// less one.
    pub duty: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Split {
    pub low_key: u8,
    pub high_key: u8,
    pub sample: u16,
    pub coarse_tune: i8,
    pub fine_tune: i8,
    pub root_key: u8,
    pub volume: i8,
    pub pan: i8,
    pub envelope: Envelope,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Program {
    pub volume: i8,
    pub splits: Vec<Split>,
}

impl Program {
    /// The split playing `key`.
    pub fn split(&self, key: u8) -> Option<&Split> {
        self.splits.iter().find(|s| (s.low_key..=s.high_key).contains(&key))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Bank {
    #[serde(serialize_with = "sorted")]
    pub samples: HashMap<u16, SampleInfo>,
    #[serde(serialize_with = "sorted")]
    pub programs: HashMap<u16, Program>,
    /// The "pcmd" chunk; empty in banks that rely on the main bank.
    pub pcm: Vec<u8>,
}

impl Bank {
    /// Decodes a sample described by this bank from its data chunk `pcm` to
    /// signed 16-bit PCM. Returns the samples and the loop start, both in
    /// output samples. Song banks carry no data: their samples are decoded
    /// through the main bank, which describes the same sample ids.
    pub fn decode(&self, id: u16, pcm: &[u8]) -> Option<(Vec<i16>, usize)> {
        let info = self.samples.get(&id)?;
        let length = info.loop_start + info.loop_length;
        let raw = pcm.get(info.position..info.position + length)?;
        Some(match info.format {
            SampleFormat::Pcm8 => (raw.iter().map(|&b| (b as i8 as i16) << 8).collect(), info.loop_start),
            SampleFormat::Pcm16 => (raw.as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes([b[0], b[1]])).collect(), info.loop_start / 2),
            // Two samples per byte after the 4-byte header.
            SampleFormat::Adpcm => (decode_adpcm(raw), info.loop_start.saturating_sub(4) * 2),
            SampleFormat::Psg | SampleFormat::Noise => return None,
        })
    }
}

/// The milliseconds of an envelope's times, to be multiplied (ppmdu's
/// tables of the sound driver).
const ENVELOPE_TIMES: [u16; 128] = [
    0x0000, 0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008, 0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x000E, 0x000F, 0x0010, 0x0011, 0x0012,
    0x0013, 0x0014, 0x0015, 0x0016, 0x0017, 0x0018, 0x0019, 0x001A, 0x001B, 0x001C, 0x001D, 0x001E, 0x001F, 0x0020, 0x0023, 0x0028, 0x002D, 0x0033, 0x0039,
    0x0040, 0x0048, 0x0050, 0x0058, 0x0062, 0x006D, 0x0078, 0x0083, 0x0090, 0x009E, 0x00AC, 0x00BC, 0x00CC, 0x00DE, 0x00F0, 0x0104, 0x0119, 0x012F, 0x0147,
    0x0160, 0x017A, 0x0196, 0x01B3, 0x01D2, 0x01F2, 0x0214, 0x0238, 0x025E, 0x0285, 0x02AE, 0x02D9, 0x0307, 0x0336, 0x0367, 0x039B, 0x03D1, 0x0406, 0x0442,
    0x047E, 0x04C4, 0x0500, 0x0546, 0x058C, 0x0622, 0x0672, 0x06CC, 0x071C, 0x0776, 0x07DA, 0x0834, 0x0898, 0x0906, 0x096A, 0x09D8, 0x0A50, 0x0ABE, 0x0B40,
    0x0BB8, 0x0C3A, 0x0CBC, 0x0D48, 0x0DDE, 0x0E6A, 0x0F00, 0x0FA0, 0x1040, 0x10EA, 0x1194, 0x123E, 0x12F2, 0x13B0, 0x146E, 0x1536, 0x15FE, 0x16D0, 0x17A2,
    0x187E, 0x195A, 0x1A40, 0x1B30, 0x1C20, 0x1D1A, 0x1E1E, 0x1F22, 0x2030, 0x2148, 0x2260, 0x2382, 0x2710, 0x7FFF,
];

/// The same without a multiplier.
const ENVELOPE_LONG_TIMES: [u32; 128] = [
    0x00000000, 0x00000004, 0x00000007, 0x0000000A, 0x0000000F, 0x00000015, 0x0000001C, 0x00000024, 0x0000002E, 0x0000003A, 0x00000048, 0x00000057, 0x00000068,
    0x0000007B, 0x00000091, 0x000000A8, 0x00000185, 0x000001BE, 0x000001FC, 0x0000023F, 0x00000288, 0x000002D6, 0x0000032A, 0x00000385, 0x000003E5, 0x0000044C,
    0x000004BA, 0x0000052E, 0x000005A9, 0x0000062C, 0x000006B5, 0x00000746, 0x00000BCF, 0x00000CC0, 0x00000DBD, 0x00000EC6, 0x00000FDC, 0x000010FF, 0x0000122F,
    0x0000136C, 0x000014B6, 0x0000160F, 0x00001775, 0x000018EA, 0x00001A6D, 0x00001BFF, 0x00001DA0, 0x00001F51, 0x00002C16, 0x00002E80, 0x00003100, 0x00003395,
    0x00003641, 0x00003902, 0x00003BDB, 0x00003ECA, 0x000041D0, 0x000044EE, 0x00004824, 0x00004B73, 0x00004ED9, 0x00005259, 0x000055F2, 0x000059A4, 0x000074CC,
    0x000079AB, 0x00007EAC, 0x000083CE, 0x00008911, 0x00008E77, 0x000093FF, 0x000099AA, 0x00009F78, 0x0000A56A, 0x0000AB80, 0x0000B1BB, 0x0000B81A, 0x0000BE9E,
    0x0000C547, 0x0000CC17, 0x0000FD42, 0x000105CB, 0x00010E82, 0x00011768, 0x0001207E, 0x000129C4, 0x0001333B, 0x00013CE2, 0x000146BB, 0x000150C5, 0x00015B02,
    0x00016572, 0x00017015, 0x00017AEB, 0x000185F5, 0x00019133, 0x0001E16D, 0x0001EF07, 0x0001FCE0, 0x00020AF7, 0x0002194F, 0x000227E6, 0x000236BE, 0x000245D7,
    0x00025532, 0x000264CF, 0x000274AE, 0x000284D0, 0x00029536, 0x0002A5E0, 0x0002B6CE, 0x0002C802, 0x000341B0, 0x000355F8, 0x00036A90, 0x00037F79, 0x000394B4,
    0x0003AA41, 0x0003C021, 0x0003D654, 0x0003ECDA, 0x000403B5, 0x00041AE5, 0x0004326A, 0x00044A45, 0x00046277, 0x00047B00, 0x7FFFFFFF,
];

const ADPCM_INDEX: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];
const ADPCM_STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230,
    253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327,
    3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];

/// Standard IMA ADPCM as used by the DS: a header holding the first sample
/// and the step index, then 4-bit codes, low nibble first.
pub fn decode_adpcm(data: &[u8]) -> Vec<i16> {
    if data.len() < 4 {
        return Vec::new();
    }
    let mut sample = i16::from_le_bytes([data[0], data[1]]) as i32;
    let mut index = (data[2] as i32).clamp(0, 88);
    let mut out = Vec::with_capacity((data.len() - 4) * 2);
    for &byte in &data[4..] {
        for code in [byte & 0xF, byte >> 4] {
            let step = ADPCM_STEPS[index as usize];
            let mut diff = step >> 3;
            if code & 1 != 0 {
                diff += step >> 2;
            }
            if code & 2 != 0 {
                diff += step >> 1;
            }
            if code & 4 != 0 {
                diff += step;
            }
            sample = if code & 8 != 0 { sample - diff } else { sample + diff }.clamp(-32768, 32767);
            index = (index + ADPCM_INDEX[(code & 7) as usize]).clamp(0, 88);
            out.push(sample as i16);
        }
    }
    out
}

/// Every song, short tune (jingle) and sound of the game, with the banks
/// they play with.
#[derive(Default, Serialize, Deserialize)]
pub struct Library {
    /// The songs by number, each with its own bank; the bank holding the
    /// samples of songs' banks that have none.
    #[serde(serialize_with = "sorted")]
    pub songs: HashMap<u16, (Song, Bank)>,
    pub song_samples: Bank,
    /// The short tunes by number, and their bank.
    #[serde(serialize_with = "sorted")]
    pub jingles: HashMap<u16, Song>,
    pub jingle_bank: Bank,
    /// The sounds by number (their group's in the high byte, their place
    /// in it in the low one), each with the numbers of the banks it plays
    /// with (its own first); the sounds' banks by number.
    #[serde(serialize_with = "sorted")]
    pub sounds: HashMap<u16, (Song, Vec<[u8; 2]>)>,
    #[serde(serialize_with = "sorted")]
    pub sound_banks: HashMap<[u8; 2], Bank>,
}

impl Library {
    /// The game's library, as the extraction wrote it.
    pub fn open(folder: &str) -> Option<Library> {
        bincode::deserialize(&assets::data::read(folder, "sound.bin")?).ok()
    }

    /// Song `number`, played for `seconds` at most: PCM audio (see
    /// `audio::OUTPUT_RATE`).
    pub fn music(&self, number: u16, seconds: f32) -> Option<Vec<i16>> {
        let (song, bank) = self.songs.get(&number)?;
        let banks = crate::audio::Banks { own: bank, main: &self.song_samples, named: &HashMap::new(), gain: 1.0 };
        Some(crate::audio::render(song, &banks, seconds))
    }

    /// Short tune `number`.
    pub fn jingle(&self, number: u16, seconds: f32) -> Option<Vec<i16>> {
        let song = self.jingles.get(&number)?;
        let banks = crate::audio::Banks { own: &self.jingle_bank, main: &self.jingle_bank, named: &HashMap::new(), gain: 1.0 };
        Some(crate::audio::render(song, &banks, seconds))
    }

    /// How long sound `number` lasts, in seconds, `seconds` at most.
    pub fn sound_seconds(&self, number: u16, seconds: f32) -> Option<f32> {
        let (song, _) = self.sounds.get(&number)?;
        Some(crate::audio::length(song, seconds))
    }

    /// Sound `number`, louder than songs (see `audio::SOUND_GAIN`).
    pub fn sound(&self, number: u16, seconds: f32) -> Option<Vec<i16>> {
        let (song, banks) = self.sounds.get(&number)?;
        let own = self.sound_banks.get(banks.first()?)?;
        let banks = crate::audio::Banks { own, main: own, named: &self.sound_banks, gain: crate::audio::SOUND_GAIN };
        Some(crate::audio::render(song, &banks, seconds))
    }
}
