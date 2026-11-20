//! SEDL sound effects (`SOUND/SE/*.sed`, `SOUND/SYSTEM/se_sys.sed`).
//!
//! A file is a group of sounds: a 0x40-byte header (the group's number at
//! 0x0E, the count of its sounds at 0x30), then chunks as a bank's (4-byte
//! name, 8 bytes of parameters, 32-bit length, data, padding to 16).
//! "seq " starts with a table of 16-bit offsets, one per sound (0: none),
//! each to a 0x30-byte header like a song's followed by its "trk " chunks
//! (see [`super::smdl`]); "bnkl" has the same table to, per sound, a count
//! and the numbers of the banks holding its samples. Worked out on the
//! ROM's files with ppmdu's notes on the DSE formats; "mcrl" is not read.
//!
//! A sound's number in the game is its group's in the high byte and its
//! place in the group in the low one. [`library`] reads the whole `SOUND`
//! folder into the project's own music library.

use super::smdl::parse_track;
use crate::bytes::u16_at;
use media::music::Song;

/// The number of the group of sounds a file holds.
pub fn group(data: &[u8]) -> Option<u8> {
    (data.get(0..4)? == b"sedl").then(|| data.get(0x0E).copied())?
}

/// Sound `index` of a file: its notes, and the numbers of its banks (the
/// two bytes at 0x0E of a bank's file), which its tracks choose among.
pub fn sound(data: &[u8], index: usize) -> Option<(Song, Vec<[u8; 2]>)> {
    if group(data).is_none() || index >= u16_at(data, 0x30)? as usize {
        return None;
    }
    let (mut song, mut bank) = (None, None);
    let mut pos = 0x40;
    while let Some(name) = data.get(pos..pos + 4) {
        let length = u32::from_le_bytes(data.get(pos + 12..pos + 16)?.try_into().ok()?) as usize;
        let body = data.get(pos + 16..pos + 16 + length)?;
        match name {
            b"seq " => {
                let at = u16_at(body, index * 2)? as usize;
                if at == 0 {
                    return None;
                }
                let ticks_per_quarter = u16_at(body, at + 2)?;
                let mut tracks = Vec::new();
                let mut track = at + 0x30;
                while body.get(track..track + 4) == Some(b"trk ") {
                    let length = u32::from_le_bytes(body.get(track + 12..track + 16)?.try_into().ok()?) as usize;
                    tracks.push(parse_track(body.get(track + 16..track + 16 + length)?).ok()?);
                    track = (track + 16 + length + 3) & !3;
                }
                song = Some(Song { ticks_per_quarter, tracks });
            }
            b"bnkl" => {
                let at = u16_at(body, index * 2)? as usize;
                let count = u16_at(body, at)? as usize;
                let numbers = (0..count).filter_map(|n| Some([*body.get(at + 4 + n * 2)?, *body.get(at + 5 + n * 2)?]));
                bank = Some(numbers.collect::<Vec<_>>()).filter(|banks| !banks.is_empty());
            }
            b"eod " => break,
            _ => {}
        }
        pos = (pos + 16 + length + 15) & !15;
    }
    Some((song?, bank?))
}

/// The number of a bank of sounds, by which tracks name it: the one its
/// file bears, or for the numbered files of `SOUND/SWD` their own number
/// (two of them bear the same).
pub(crate) fn bank_number(name: &str, data: &[u8]) -> Option<[u8; 2]> {
    let stem = name.strip_suffix(".swd")?;
    let digits = stem.get(stem.len().checked_sub(3)?..)?;
    match digits.parse::<u8>() {
        Ok(number) if stem.starts_with("SWD") => Some([number, 0x64]),
        _ => Some([*data.get(0x0E)?, *data.get(0x0F)?]),
    }
}

/// The game's songs, short tunes and sounds, read from the files of its
/// `SOUND` folder (`files`: their paths in it, and their contents) into the
/// project's own library.
pub fn library<'a>(files: impl Iterator<Item = (&'a str, &'a [u8])>) -> media::music::Library {
    use media::music::Library;
    let mut files: Vec<(&str, &[u8])> = files.collect();
    // (In the order of their names: the same file wins every time.)
    files.sort();
    let mut library = Library::default();
    let number = |name: &str, prefix: &str, suffix: &str| name.strip_prefix(prefix)?.strip_suffix(suffix)?.parse::<u16>().ok();
    let file = |name: &str| files.iter().find(|(path, _)| *path == name).map(|(_, data)| *data);
    library.song_samples = file("BGM/bgm.swd").and_then(|data| crate::swdl::parse_bank(data).ok()).unwrap_or_default();
    library.jingle_bank = file("ME/me.swd").and_then(|data| crate::swdl::parse_bank(data).ok()).unwrap_or_default();
    for (name, data) in &files {
        if let Some(song) = number(name, "BGM/bgm", ".smd") {
            let bank = file(&format!("BGM/bgm{song:04}.swd")).and_then(|data| crate::swdl::parse_bank(data).ok());
            if let (Ok(notes), Some(bank)) = (crate::smdl::parse_song(data), bank) {
                library.songs.insert(song, (notes, bank));
            }
        } else if let Some(jingle) = number(name, "ME/me", ".smd") {
            if let Ok(notes) = crate::smdl::parse_song(data) {
                library.jingles.insert(jingle, notes);
            }
        } else if name.ends_with(".sed")
            && let Some(group) = group(data)
        {
            for index in 0..=255u8 {
                if let Some(sound) = sound(data, index as usize) {
                    library.sounds.entry(u16::from(group) << 8 | u16::from(index)).or_insert(sound);
                }
            }
        // (The sounds' banks: a song's bank may share a number with one.)
        } else if name.ends_with(".swd")
            && !name.starts_with("BGM/")
            && !name.starts_with("ME/")
            && let (Some(number), Ok(bank)) = (bank_number(name, data), crate::swdl::parse_bank(data))
        {
            library.sound_banks.entry(number).or_insert(bank);
        }
    }
    library
}

#[cfg(test)]
mod tests {
    use super::*;

    use media::music::Bank;
    use media::music::Event;

    /// Needs a real dump.
    #[test]
    #[ignore]
    fn real_rom_sounds_all_decode() {
        let rom = crate::nds::test_rom();
        let banks: Vec<([u8; 2], Bank)> = rom
            .paths()
            .filter(|path| path.ends_with(".swd") && !path.starts_with("SOUND/BGM"))
            .filter_map(|path| {
                let data = rom.file(path).unwrap();
                Some(([data[0x0E], data[0x0F]], crate::swdl::parse_bank(data).ok()?))
            })
            .collect();
        let (mut groups, mut sounds, mut notes, mut bankless, mut silent) = (0, 0, 0, 0, Vec::new());
        for path in rom.paths().filter(|path| path.ends_with(".sed")) {
            let data = rom.file(path).unwrap();
            assert!(group(data).is_some(), "{path}");
            groups += 1;
            let count = u16_at(data, 0x30).unwrap() as usize;
            // The first of a group is no sound.
            assert!(sound(data, 0).is_none() && sound(data, count).is_none(), "{path}");
            for index in 1..count {
                let Some((song, bank)) = sound(data, index) else {
                    println!("{path} {index}: not read");
                    continue;
                };
                sounds += 1;
                notes += song.tracks.iter().flat_map(|track| &track.events).filter(|event| matches!(event.1, Event::Note { .. })).count();
                let Some((_, bank)) = banks.iter().find(|(id, _)| *id == bank[0]) else {
                    bankless += 1;
                    continue;
                };
                // Heard: a sound in seven is rendered.
                if sounds % 7 == 0 {
                    let pcm = media::audio::render(&song, &media::audio::Banks { own: bank, main: bank, named: &Default::default(), gain: 1.0 }, 2.0);
                    if pcm.iter().all(|sample| sample.abs() < 200) {
                        silent.push(format!("{path} {index}"));
                    }
                }
            }
        }
        println!("{groups} groups, {sounds} sounds, {notes} notes, {bankless} without their bank, {} banks", banks.len());
        println!("silent of those rendered: {silent:?}");
        assert!(groups > 40 && sounds > 800 && notes > sounds && bankless == 0);
        assert!(silent.len() < sounds / 7 / 10);
    }
}
