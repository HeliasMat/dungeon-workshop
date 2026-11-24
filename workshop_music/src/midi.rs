//! Standard MIDI files (as every music program reads and writes them): a song's notes written out, and read in.

/// A track of a MIDI file: its name, whether it is drums (channel 10), its notes (start and length in beats,
/// key, force).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MidiTrack {
    pub(crate) name: String,
    pub(crate) drums: bool,
    pub(crate) notes: Vec<(f32, f32, u8, u8)>,
}

/// The General MIDI keys of the drum kit's pieces, row by row.
pub(crate) const DRUM_KEYS: [u8; 10] = [36, 38, 39, 42, 46, 45, 50, 49, 56, 37];

/// Ticks a beat.
const DIVISION: u32 = 480;

fn variable(mut value: u32, out: &mut Vec<u8>) {
    let mut bytes = vec![(value & 0x7F) as u8];
    value >>= 7;
    while value > 0 {
        bytes.push(((value & 0x7F) as u8) | 0x80);
        value >>= 7;
    }
    bytes.reverse();
    out.extend(bytes);
}

fn chunk(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = kind.to_vec();
    out.extend((body.len() as u32).to_be_bytes());
    out.extend(body);
    out
}

/// A song as a MIDI file (format 1): its tempo and bar, then a track each.
pub(crate) fn write(tempo: f32, beats_per_bar: f32, tracks: &[MidiTrack]) -> Vec<u8> {
    let mut out = chunk(b"MThd", &[[0, 1], ((tracks.len() + 1) as u16).to_be_bytes(), (DIVISION as u16).to_be_bytes()].concat());
    let mut head = Vec::new();
    let micro = (60_000_000.0 / tempo.max(1.0)) as u32;
    head.extend([0, 0xFF, 0x51, 3]);
    head.extend(&micro.to_be_bytes()[1..]);
    head.extend([0, 0xFF, 0x58, 4, beats_per_bar.round().clamp(1.0, 32.0) as u8, 2, 24, 8]);
    head.extend([0, 0xFF, 0x2F, 0]);
    out.extend(chunk(b"MTrk", &head));
    for (at, track) in tracks.iter().enumerate() {
        let channel = if track.drums { 9 } else { [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15][at % 15] };
        // (Each note's start and end, ends first where they meet.)
        let mut events: Vec<(u32, u8, u8, u8)> = Vec::new();
        for (start, length, key, velocity) in &track.notes {
            let on = (start.max(0.0) * DIVISION as f32).round() as u32;
            let off = on + ((length.max(0.01)) * DIVISION as f32).round().max(1.0) as u32;
            events.push((on, 1, *key, (*velocity).max(1)));
            events.push((off, 0, *key, 0));
        }
        events.sort_by_key(|(tick, on, ..)| (*tick, *on));
        let mut body = Vec::new();
        let name = track.name.as_bytes();
        body.extend([0, 0xFF, 0x03]);
        variable(name.len() as u32, &mut body);
        body.extend(name);
        let mut last = 0;
        for (tick, on, key, velocity) in events {
            variable(tick - last, &mut body);
            last = tick;
            body.extend([if on == 1 { 0x90 } else { 0x80 } | channel, key.min(127), velocity.min(127)]);
        }
        body.extend([0, 0xFF, 0x2F, 0]);
        out.extend(chunk(b"MTrk", &body));
    }
    out
}

/// A MIDI file read: its first tempo (none: 120), its tracks with notes (split by channel).
pub(crate) fn read(bytes: &[u8]) -> Result<(f32, Vec<MidiTrack>), String> {
    let wrong = || "not a readable MIDI file".to_string();
    if bytes.len() < 14 || &bytes[0..4] != b"MThd" {
        return Err(wrong());
    }
    let division = u16::from_be_bytes([bytes[12], bytes[13]]);
    if division & 0x8000 != 0 || division == 0 {
        return Err("a MIDI file in SMPTE time: not read".to_string());
    }
    let ticks = f32::from(division);
    let mut tempo = None;
    let mut tracks = Vec::new();
    let mut at = 8 + u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    while at + 8 <= bytes.len() {
        let length = u32::from_be_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let body = bytes.get(at + 8..at + 8 + length).ok_or_else(wrong)?;
        let kind = &bytes[at..at + 4];
        at += 8 + length;
        if kind != b"MTrk" {
            continue;
        }
        let (mut place, mut tick, mut status, mut name) = (0_usize, 0_u32, 0_u8, String::new());
        let mut open: Vec<(u8, u8, u32, u8)> = Vec::new();
        let mut by_channel: std::collections::BTreeMap<u8, Vec<(f32, f32, u8, u8)>> = std::collections::BTreeMap::new();
        let read_variable = |place: &mut usize| -> Option<u32> {
            let mut value = 0_u32;
            for _ in 0..4 {
                let byte = *body.get(*place)?;
                *place += 1;
                value = (value << 7) | u32::from(byte & 0x7F);
                if byte & 0x80 == 0 {
                    return Some(value);
                }
            }
            Some(value)
        };
        while place < body.len() {
            tick += read_variable(&mut place).ok_or_else(wrong)?;
            let mut byte = *body.get(place).ok_or_else(wrong)?;
            if byte & 0x80 != 0 {
                status = byte;
                place += 1;
            } else {
                byte = status;
            }
            match byte {
                0xFF => {
                    let meta = *body.get(place).ok_or_else(wrong)?;
                    place += 1;
                    let size = read_variable(&mut place).ok_or_else(wrong)? as usize;
                    let data = body.get(place..place + size).ok_or_else(wrong)?;
                    place += size;
                    match meta {
                        0x51 if size == 3 && tempo.is_none() => {
                            let micro = u32::from_be_bytes([0, data[0], data[1], data[2]]);
                            tempo = Some(60_000_000.0 / micro.max(1) as f32);
                        }
                        0x03 => name = String::from_utf8_lossy(data).into_owned(),
                        0x2F => break,
                        _ => {}
                    }
                }
                0xF0 | 0xF7 => {
                    let size = read_variable(&mut place).ok_or_else(wrong)? as usize;
                    place += size;
                }
                _ => {
                    let channel = byte & 0x0F;
                    let data_size = if matches!(byte & 0xF0, 0xC0 | 0xD0) { 1 } else { 2 };
                    let data = body.get(place..place + data_size).ok_or_else(wrong)?;
                    place += data_size;
                    let (key, velocity) = (data[0], data.get(1).copied().unwrap_or(0));
                    match byte & 0xF0 {
                        0x90 if velocity > 0 => open.push((channel, key, tick, velocity)),
                        0x80 | 0x90 => {
                            if let Some(found) = open.iter().position(|(on, held, ..)| *on == channel && *held == key) {
                                let (_, _, start, velocity) = open.remove(found);
                                by_channel.entry(channel).or_default().push((start as f32 / ticks, (tick - start) as f32 / ticks, key, velocity));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for (channel, mut notes) in by_channel {
            notes.sort_by(|a, b| a.0.total_cmp(&b.0));
            let named = if name.trim().is_empty() { format!("MIDI {}", tracks.len() + 1) } else { name.trim().to_string() };
            tracks.push(MidiTrack { name: named, drums: channel == 9, notes });
        }
    }
    Ok((tempo.unwrap_or(120.0), tracks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_midi_file_written_is_read_back() {
        let tracks = vec![
            MidiTrack { name: "Piano".to_string(), drums: false, notes: vec![(0.0, 1.0, 60, 100), (1.0, 0.5, 64, 90), (1.0, 2.0, 67, 80)] },
            MidiTrack { name: "Batterie".to_string(), drums: true, notes: vec![(0.0, 0.25, 36, 127), (1.0, 0.25, 38, 110)] },
        ];
        let (tempo, read) = read(&write(128.0, 4.0, &tracks)).expect("lu");
        assert!((tempo - 128.0).abs() < 0.01, "{tempo}");
        assert_eq!(read, tracks);
    }
}
