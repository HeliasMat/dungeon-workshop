//! SWDL sound banks (`SOUND/BGM/*.swd`): instruments and their samples,
//! read into the project's own banks (`media::music::Bank`).
//!
//! A file is a 0x50-byte header followed by chunks: "wavi" (sample
//! descriptions), "prgi" (programs, i.e. instruments made of key splits),
//! "kgrp", "pcmd" (raw sample data) and "eod ". Each song has its own bank
//! describing its programs; the sample data itself sits in the main bank
//! `bgm.swd`. Layout as documented by the community (Project Pokémon /
//! ppmdu), checked against every bank of the ROM in the tests.

use super::nds::RomError;
use crate::bytes::{offset_at, u16_le};
use media::music::{Bank, Envelope, Program, SampleFormat, SampleInfo, Split};

pub(crate) fn parse_bank(data: &[u8]) -> Result<Bank, RomError> {
    if data.get(0..4) != Some(b"swdl") {
        return Err(RomError::Truncated);
    }
    let sample_slots = u16_le(data, 0x46)? as usize;
    let program_slots = u16_le(data, 0x48)? as usize;
    let mut bank = Bank::default();

    // Chunks: 4-byte name, 8 bytes of parameters, 32-bit length, data,
    // padding to a multiple of 16.
    let mut pos = 0x50;
    while let Some(name) = data.get(pos..pos + 4) {
        let length = offset_at(data, pos + 12)?;
        let body = data.get(pos + 16..pos + 16 + length).ok_or(RomError::Truncated)?;
        match name {
            b"wavi" => {
                for offset in pointer_table(body, sample_slots)? {
                    let sample = parse_sample(body.get(offset..).ok_or(RomError::Truncated)?)?;
                    bank.samples.insert(sample.id, sample);
                }
            }
            b"prgi" => {
                for offset in pointer_table(body, program_slots)? {
                    let entry = body.get(offset..).ok_or(RomError::Truncated)?;
                    bank.programs.insert(u16_le(entry, 0)?, parse_program(entry)?);
                }
            }
            b"pcmd" => bank.pcm = body.to_vec(),
            b"eod " => return Ok(bank),
            _ => {}
        }
        pos = (pos + 16 + length + 15) & !15;
    }
    Err(RomError::Truncated)
}

/// The non-zero 16-bit offsets of a chunk's leading pointer table.
fn pointer_table(body: &[u8], slots: usize) -> Result<Vec<usize>, RomError> {
    let mut offsets = Vec::new();
    for slot in 0..slots {
        let offset = u16_le(body, slot * 2)? as usize;
        if offset != 0 {
            offsets.push(offset);
        }
    }
    Ok(offsets)
}

/// 0x40-byte sample description.
fn parse_sample(e: &[u8]) -> Result<SampleInfo, RomError> {
    if e.len() < 0x40 {
        return Err(RomError::Truncated);
    }
    Ok(SampleInfo {
        id: u16_le(e, 0x02)?,
        fine_tune: e[0x04] as i8,
        coarse_tune: e[0x05] as i8,
        root_key: e[0x06],
        volume: e[0x08] as i8,
        format: match u16_le(e, 0x12)? {
            0x0000 => SampleFormat::Pcm8,
            0x0100 => SampleFormat::Pcm16,
            0x0200 => SampleFormat::Adpcm,
            0x0400 => SampleFormat::Noise,
            _ => SampleFormat::Psg,
        },
        duty: e[0x19],
        looped: e[0x15] != 0,
        rate: offset_at(e, 0x20)? as u32,
        position: offset_at(e, 0x24)?,
        // Stored in 32-bit words.
        loop_start: offset_at(e, 0x28)? * 4,
        loop_length: offset_at(e, 0x2C)? * 4,
    })
}

/// 0x10-byte header, 16 bytes per LFO, a 16-byte delimiter, then 0x30 bytes
/// per split.
fn parse_program(e: &[u8]) -> Result<Program, RomError> {
    let split_count = u16_le(e, 0x02)? as usize;
    let lfo_count = *e.get(0x0B).ok_or(RomError::Truncated)? as usize;
    let first = 0x10 + lfo_count * 16 + 16;
    let mut splits = Vec::with_capacity(split_count);
    for i in 0..split_count {
        let s = e.get(first + i * 0x30..first + (i + 1) * 0x30).ok_or(RomError::Truncated)?;
        splits.push(Split {
            low_key: s[0x04],
            high_key: s[0x05],
            sample: u16_le(s, 0x12)?,
            fine_tune: s[0x14] as i8,
            coarse_tune: s[0x15] as i8,
            root_key: s[0x16],
            volume: s[0x18] as i8,
            pan: s[0x19] as i8,
            envelope: Envelope {
                on: s[0x20] != 0,
                multiplier: s[0x21],
                first: s[0x28],
                attack: s[0x29],
                decay: s[0x2A],
                sustain: s[0x2B],
                hold: s[0x2C],
                fade: s[0x2D],
                release: s[0x2E],
            },
        });
    }
    Ok(Program { volume: *e.get(0x04).ok_or(RomError::Truncated)? as i8, splits })
}

#[cfg(test)]
mod tests {
    use super::*;

    use media::music::{Event, decode_adpcm};

    #[test]
    fn adpcm_follows_the_codes() {
        // Start at 0 with the smallest step: +code 7 climbs, code 15 falls back.
        let up = decode_adpcm(&[0, 0, 0, 0, 0x77, 0x77]);
        assert!(up.windows(2).all(|pair| pair[1] > pair[0]) && up[0] > 0);
        let down = decode_adpcm(&[0, 0, 0, 0, 0xFF, 0xFF]);
        assert!(down.windows(2).all(|pair| pair[1] < pair[0]) && down[0] < 0);
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_banks_match_their_songs() {
        let rom = crate::nds::test_rom();
        let main = parse_bank(rom.file("SOUND/BGM/bgm.swd").unwrap()).unwrap();
        println!("main bank: {} samples described, {} bytes of data", main.samples.len(), main.pcm.len());
        assert!(main.pcm.len() > 1_000_000);

        let mut songs: Vec<&str> = rom.paths().filter(|p| p.starts_with("SOUND/BGM/bgm0") && p.ends_with(".smd")).collect();
        songs.sort();
        let (mut programs_used, mut missing_programs, mut undecodable) = (0, 0, 0);
        for path in &songs {
            let song = crate::smdl::parse_song(rom.file(path).unwrap()).unwrap();
            let bank_path = path.replace(".smd", ".swd");
            let bank = parse_bank(rom.file(&bank_path).unwrap_or_else(|| panic!("{bank_path}"))).unwrap_or_else(|e| panic!("{bank_path}: {e}"));
            for track in &song.tracks {
                for (_, event) in &track.events {
                    let Event::Program(id) = event else { continue };
                    programs_used += 1;
                    let Some(program) = bank.programs.get(&(*id as u16)) else {
                        missing_programs += 1;
                        continue;
                    };
                    // Every split names a sample that decodes from the main
                    // bank (the song's own descriptions point into a data
                    // chunk it does not have).
                    for split in &program.splits {
                        match main.decode(split.sample, &main.pcm) {
                            Some((pcm, loop_start)) if !pcm.is_empty() && loop_start <= pcm.len() => {}
                            _ => undecodable += 1,
                        }
                    }
                }
            }
        }
        println!(
            "{} songs, {programs_used} program changes, {missing_programs} without a program, {undecodable} splits without a decodable sample",
            songs.len()
        );
        assert!(programs_used > 1000);
        // A few program changes name programs their bank does not define.
        assert!(missing_programs * 50 < programs_used, "too many missing programs");
        assert_eq!(undecodable, 0);
    }
}
