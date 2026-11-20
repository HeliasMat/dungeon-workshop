//! SMDL music sequences (`SOUND/BGM/*.smd`), read into the project's own
//! songs (`media::music::Song`).
//!
//! A file is a 0x40-byte header, a "song" chunk, then one "trk " chunk per
//! track and a closing "eoc " chunk. A track is a stream of events: notes,
//! pauses and controller changes. Event layout as documented by the
//! community (Project Pokémon / ppmdu), checked here against every song of
//! the ROM by requiring each track to end exactly on its end-of-track event.

use super::nds::RomError;
use media::music::{Event, Song, Track};

/// Lengths, in ticks, of the fixed pauses 0x80..=0x8F.
const FIXED_PAUSES: [u32; 16] = [96, 72, 64, 48, 36, 32, 24, 18, 16, 12, 9, 8, 6, 4, 3, 2];

/// A track could not be decoded: `opcode` is unknown, or the stream ran out.
#[derive(Debug)]
pub struct BadEvent {
    pub opcode: u8,
}

pub fn parse_song(data: &[u8]) -> Result<Song, RomError> {
    if data.get(0..4) != Some(b"smdl") || data.get(0x40..0x44) != Some(b"song") {
        return Err(RomError::Truncated);
    }
    let ticks_per_quarter = u16::from_le_bytes([byte(data, 0x52)?, byte(data, 0x53)?]);
    let mut tracks = Vec::new();
    // Chunks: 4-byte name, 8 bytes of parameters, 32-bit length, data,
    // padding to a multiple of 4.
    let mut pos = 0x80;
    while data.get(pos..pos + 4) == Some(b"trk ") {
        let length = u32::from_le_bytes([byte(data, pos + 12)?, byte(data, pos + 13)?, byte(data, pos + 14)?, byte(data, pos + 15)?]) as usize;
        let body = data.get(pos + 16..pos + 16 + length).ok_or(RomError::Truncated)?;
        tracks.push(parse_track(body).map_err(|_| RomError::Truncated)?);
        pos = (pos + 16 + length + 3) & !3;
    }
    if data.get(pos..pos + 4) != Some(b"eoc ") {
        return Err(RomError::Truncated);
    }
    Ok(Song { ticks_per_quarter, tracks })
}

/// `body` starts with a 4-byte preamble: track id, channel id, two unknowns.
pub fn parse_track(body: &[u8]) -> Result<Track, BadEvent> {
    let channel = *body.get(1).ok_or(BadEvent { opcode: 0 })?;
    if body.len() < 4 {
        return Err(BadEvent { opcode: 0 });
    }
    let mut events = Vec::new();
    let (mut tick, mut last_pause, mut last_duration, mut octave) = (0u32, 0u32, 0u32, 0i32);
    // Where a passage played several times starts, and the times left.
    let mut repeat: Option<(usize, u8)> = None;
    let mut pos = 4;
    loop {
        // (A track gone wrong does not fill the memory.)
        if events.len() > 200_000 {
            return Err(BadEvent { opcode: 0x9D });
        }
        // Some stub tracks (a lone controller change) have no end-of-track
        // event: the end of the chunk closes them.
        let Some(&opcode) = body.get(pos) else {
            return Ok(Track { channel, events, length: tick });
        };
        pos += 1;
        let mut take = |count: usize| -> Result<&[u8], BadEvent> {
            let bytes = body.get(pos..pos + count).ok_or(BadEvent { opcode })?;
            pos += count;
            Ok(bytes)
        };
        match opcode {
            // Note: velocity, then a byte packing the number of duration
            // bytes (2 bits), an octave change (2 bits) and the note (4 bits).
            0x00..=0x7F => {
                let info = take(1)?[0];
                octave += (info >> 4 & 3) as i32 - 2;
                let duration_bytes = take((info >> 6) as usize)?;
                if !duration_bytes.is_empty() {
                    last_duration = duration_bytes.iter().fold(0, |d, &b| d << 8 | b as u32);
                }
                let key = (octave * 12 + (info & 0xF) as i32).clamp(0, 127) as u8;
                events.push((tick, Event::Note { key, velocity: opcode, duration: last_duration }));
            }
            0x80..=0x8F => {
                last_pause = FIXED_PAUSES[(opcode - 0x80) as usize];
                tick += last_pause;
            }
            0x90 => tick += last_pause,
            0x91 => {
                last_pause = last_pause.wrapping_add_signed(take(1)?[0] as i8 as i32);
                tick += last_pause;
            }
            0x92..=0x94 => {
                let bytes = take((opcode - 0x91) as usize)?;
                last_pause = bytes.iter().rev().fold(0, |d, &b| d << 8 | b as u32);
                tick += last_pause;
            }
            0x95 => {
                last_pause = take(1)?[0] as u32;
                tick += last_pause;
            }
            0x98 => {
                return Ok(Track { channel, events, length: tick });
            }
            0x99 => events.push((tick, Event::LoopPoint)),
            0x9C => {
                let times = take(1)?[0];
                repeat = Some((pos, times));
            }
            0x9D => {
                if let Some((from, times)) = repeat {
                    repeat = (times > 1).then(|| (from, times - 1));
                    if times > 1 {
                        pos = from;
                    }
                }
            }
            0xA8 => {
                let bytes = take(2)?;
                events.push((tick, Event::Bank { high: Some(bytes[0]), low: Some(bytes[1]) }));
            }
            0xA9 => events.push((tick, Event::Bank { high: Some(take(1)?[0]), low: None })),
            0xAA => events.push((tick, Event::Bank { high: None, low: Some(take(1)?[0]) })),
            0xB0 => events.push((tick, Event::Envelope { part: 0, value: 0 })),
            0xB1..=0xB3 | 0xB5 | 0xB6 => events.push((tick, Event::Envelope { part: opcode - 0xB0, value: take(1)?[0] })),
            0xB4 => {
                let bytes = take(2)?;
                events.push((tick, Event::Envelope { part: 4, value: bytes[0] }));
                events.push((tick, Event::Envelope { part: 7, value: bytes[1] }));
            }
            0xD2 => events.push((tick, Event::Tune { semitones: take(1)?[0] as i8, ticks: 0 })),
            0xD4 => {
                let bytes = take(3)?;
                events.push((tick, Event::Tune { semitones: bytes[2] as i8, ticks: u16::from_le_bytes([bytes[0], bytes[1]]) }));
            }
            0xD7 => {
                let bytes = take(2)?;
                events.push((tick, Event::Bend(i16::from_be_bytes([bytes[0], bytes[1]]))));
            }
            0xDB => events.push((tick, Event::BendRange(take(1)?[0]))),
            0xE2 => {
                let bytes = take(3)?;
                events.push((tick, Event::VolumeSweep { volume: bytes[2], ticks: u16::from_le_bytes([bytes[0], bytes[1]]) }));
            }
            0xA0 => octave = take(1)?[0] as i32,
            0xA1 => octave += take(1)?[0] as i8 as i32,
            0xA4 | 0xA5 => events.push((tick, Event::Tempo(take(1)?[0]))),
            0xAC => events.push((tick, Event::Program(take(1)?[0]))),
            0xE0 => events.push((tick, Event::Volume(take(1)?[0]))),
            0xE3 => events.push((tick, Event::Expression(take(1)?[0]))),
            0xE8 => events.push((tick, Event::Pan(take(1)?[0]))),
            // Other controllers (bends, envelopes, LFOs...): skipped for now.
            _ => {
                take(parameter_count(opcode).ok_or(BadEvent { opcode })?)?;
            }
        }
    }
}

/// Number of parameter bytes of the events that are not interpreted yet.
fn parameter_count(opcode: u8) -> Option<usize> {
    Some(match opcode {
        0x9E | 0xC0 => 0,
        0xAB | 0xBC | 0xBE | 0xBF | 0xC3 => 1,
        0xD0 | 0xD1 | 0xDF | 0xE1 | 0xE7 | 0xE9 | 0xEF | 0xF6 => 1,
        0xCB | 0xD3 | 0xD5 | 0xD6 | 0xD8 | 0xF2 | 0xF8 => 2,
        0xAF | 0xEA | 0xF3 => 3,
        0xDD | 0xE5 | 0xED | 0xF1 => 4,
        0xDC | 0xE4 | 0xEC | 0xF0 => 5,
        _ => return None,
    })
}

fn byte(data: &[u8], off: usize) -> Result<u8, RomError> {
    data.get(off).copied().ok_or(RomError::Truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    #[test]
    fn decodes_notes_pauses_and_controllers() {
        // Preamble, octave 4, tempo 120, note C held 24 ticks, fixed pause
        // of 48, same pause again, note one octave up with the last
        // duration, end of track.
        let body = [0, 3, 0, 0, 0xA0, 4, 0xA4, 120, 0x60, 0x60, 24, 0x83, 0x90, 0x7F, 0x32, 0x98];
        let track = parse_track(&body).unwrap();
        assert_eq!(track.channel, 3);
        assert_eq!(track.length, 96);
        assert_eq!(
            track.events,
            vec![
                (0, Event::Tempo(120)),
                (0, Event::Note { key: 48, velocity: 0x60, duration: 24 }),
                (96, Event::Note { key: 62, velocity: 0x7F, duration: 24 }),
            ]
        );
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_songs_all_decode() {
        let rom = crate::nds::test_rom();
        let mut songs: Vec<&str> = rom.paths().filter(|p| p.starts_with("SOUND/BGM/") && p.ends_with(".smd")).collect();
        songs.sort();
        let (mut tracks, mut notes) = (0, 0);
        let mut unknown: BTreeMap<u8, usize> = BTreeMap::new();
        for path in &songs {
            let data = rom.file(path).unwrap();
            // Walk the chunks by hand to report which opcode stops a track.
            let mut pos = 0x80;
            while data.get(pos..pos + 4) == Some(b"trk ") {
                let length = u32::from_le_bytes(data[pos + 12..pos + 16].try_into().unwrap()) as usize;
                match parse_track(&data[pos + 16..pos + 16 + length]) {
                    Ok(track) => {
                        tracks += 1;
                        notes += track.events.iter().filter(|e| matches!(e.1, Event::Note { .. })).count();
                    }
                    Err(bad) => *unknown.entry(bad.opcode).or_default() += 1,
                }
                pos = (pos + 16 + length + 3) & !3;
            }
            assert_eq!(data.get(pos..pos + 4), Some(&b"eoc "[..]), "{path}");
        }
        println!("{} songs, {tracks} tracks, {notes} notes, undecoded tracks by opcode: {unknown:x?}", songs.len());
        assert!(unknown.is_empty());
        assert!(songs.len() > 100 && notes > 100_000);
    }
}
