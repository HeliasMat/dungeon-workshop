//! Game text: `MESSAGE/text_<language>.str` files.
//!
//! A file is a table of 32-bit offsets followed by null-terminated strings in
//! a single-byte encoding close to Windows-1252. All languages share the
//! same string numbering.

use super::nds::{NdsRom, RomError};
use crate::bytes::offset_at;

pub struct Strings {
    strings: Vec<String>,
}

impl Strings {
    pub fn parse(data: &[u8]) -> Result<Self, RomError> {
        // The first offset points just past the table, giving its length.
        let table_len = offset_at(data, 0)?;
        let mut strings = Vec::with_capacity(table_len / 4);
        for entry in (0..table_len).step_by(4) {
            let start = offset_at(data, entry)?;
            let text = data.get(start..).ok_or(RomError::Truncated)?;
            let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
            strings.push(decode_text(&text[..end]));
        }
        Ok(Self { strings })
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&str> {
        self.strings.get(index).map(String::as_str)
    }

    /// Index of the last string `matches` accepts.
    pub fn rposition(&self, matches: impl Fn(&str) -> bool) -> Option<usize> {
        self.strings.iter().rposition(|text| matches(text))
    }

    /// Index of the first string equal to `text`.
    pub fn find(&self, text: &str) -> Option<usize> {
        self.strings.iter().position(|other| other == text)
    }

    /// Index of the first of two consecutive strings.
    pub(crate) fn find_pair(&self, first: &str, second: &str) -> Option<usize> {
        self.strings.windows(2).position(|pair| pair[0] == first && pair[1] == second)
    }
}

/// The characters of two bytes the font has (`FONT/kanji_rd.dat`, table
/// 0x81), by their second byte: a wide space, a tilde, arrows, a note.
pub use media::font::WIDE;

/// The game's text as characters: one byte each (see `decode`), but for
/// those of two bytes, which start with 0x81.
pub(crate) fn decode_text(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len());
    let mut bytes = bytes.iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        let wide = (byte == 0x81).then(|| bytes.peek().and_then(|second| WIDE.iter().find(|(code, _)| code == second))).flatten();
        match wide {
            Some(&(_, character)) => {
                bytes.next();
                text.push(character);
            }
            None => text.push(decode(byte)),
        }
    }
    text
}

/// Windows-1252, as far as the game's Western text needs it; but the
/// game's font has the signs of the male and the female where ½ and ¾ are
/// (Nidoran♂ is 0xBD, Nidoran♀ 0xBE).
fn decode(byte: u8) -> char {
    match byte {
        0xBD => '♂',
        0xBE => '♀',
        0x85 => '…',
        0x8C => 'Œ',
        0x91 => '‘',
        0x92 => '’',
        0x9C => 'œ',
        _ => byte as char,
    }
}

/// Names of moves, items and species in one language, indexed by id.
#[derive(Clone, Default)]
pub struct Names {
    pub moves: Vec<String>,
    pub items: Vec<String>,
    /// What each item does, in a line, with the game's markup.
    pub item_descriptions: Vec<String>,
    pub monsters: Vec<String>,
    pub dungeons: Vec<String>,
    /// The IQ skills, by number (see `dungeon::iq`).
    pub iq_skills: Vec<String>,
    /// The tactics of team members, and what each does (`enum
    /// tactic_id` order).
    pub tactics: Vec<String>,
    /// The team's exploration ranks.
    pub explorer_ranks: Vec<String>,
    pub tactic_descriptions: Vec<String>,
    /// Names shown for overworld levels, indexed by the level's place id
    /// (see `PLACE_COUNT`).
    pub places: Vec<String>,
}

/// Place ids below this are dungeons, named by the block the game's
/// `GetGroundNameId` reads; the others index the block of place names.
const FIRST_PLACE: usize = 180;
const PLACE_COUNT: usize = 311;
/// How far before the dungeon block the place names of ids 180 and up are
/// (checked on the European ROM: strings 0x3F80 + id and 0x41BA + id).
const PLACE_BLOCK_OFFSET: usize = 0x41BA - 0x3F80;

impl Names {
    /// `language` is the file's letter: e, f, g, i or s in the EU ROM.
    ///
    /// The numbering of each name block differs between regions, so the
    /// blocks are located in the English file by two known consecutive
    /// names, and then read from the requested language.
    pub fn load(rom: &NdsRom, language: char) -> Result<Self, RomError> {
        let file = |letter: char| {
            let data = rom.file(&format!("MESSAGE/text_{letter}.str")).ok_or(RomError::Truncated)?;
            Strings::parse(data)
        };
        let english = file('e')?;
        let text = if language == 'e' { None } else { file(language).ok() };
        let text = text.as_ref().unwrap_or(&english);
        // (names of ids `first_id` and `first_id + 1`, first_id, how many)
        let block = |first: &str, second: &str, first_id: usize, count: usize| {
            let base = english.find_pair(first, second).ok_or(RomError::Truncated)? - first_id;
            Ok::<_, RomError>((0..count).map(|id| strip_tags(text.get(base + id).unwrap_or_default())).collect())
        };
        // In this block ids 0 and 1 are both Beach Cave, unlike the main one.
        let grounds = english.find_pair("Beach Cave", "Drenched Bluff").ok_or(RomError::Truncated)?;
        let grounds = grounds.checked_sub(1).ok_or(RomError::Truncated)?;
        let places = (0..PLACE_COUNT)
            .map(|id| {
                let index = if id < FIRST_PLACE { Some(grounds + id) } else { (grounds + id).checked_sub(PLACE_BLOCK_OFFSET) };
                let name = index.and_then(|index| text.get(index)).unwrap_or_default();
                // Unused entries are tagged or left as question marks.
                if name.starts_with('[') || name.starts_with('?') { String::new() } else { name.to_string() }
            })
            .collect();
        Ok(Self {
            places,
            moves: block("Iron Tail", "Ice Ball", 1, 559)?,
            items: block("Oran Berry", "Sitrus Berry", 70, 1400)?,
            // Found by the Sleep Seed's (item 84); kept with their tags.
            item_descriptions: match english.find("Induces sleep.").and_then(|at| at.checked_sub(84)) {
                Some(base) => (0..1400).map(|id| text.get(base + id).unwrap_or_default().to_string()).collect(),
                None => Vec::new(),
            },
            monsters: block("Bulbasaur", "Ivysaur", 1, 600)?,
            dungeons: block("Beach Cave", "Beach Cave Pit", 1, 180)?,
            iq_skills: block("Type-Advantage Master", "Item Catcher", 1, 69).unwrap_or_default(),
            tactics: block("Let's go together", "You go the other way", 0, 11).unwrap_or_default(),
            explorer_ranks: block("Normal Rank", "Bronze Rank", 0, 13).unwrap_or_default(),
            // (Right after the names.)
            tactic_descriptions: block("Let's go together", "You go the other way", 0, 22).map(|both: Vec<String>| both[11..].to_vec()).unwrap_or_default(),
        })
    }
}

/// A name without the game's formatting tags (`[M:S0]`, `[CS:K]`...),
/// which stand for symbols and colours this engine does not draw.
fn strip_tags(name: &str) -> String {
    let mut plain = String::new();
    let mut tag = false;
    for c in name.chars() {
        match c {
            '[' => tag = true,
            ']' if tag => tag = false,
            c if !tag => plain.push(c),
            _ => {}
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_stripped_from_names() {
        assert_eq!(strip_tags("[M:S0]Poké"), "Poké");
        assert_eq!(strip_tags("[CS:K]Oran[CR] Berry"), "Oran Berry");
        assert_eq!(strip_tags("Apple"), "Apple");
    }

    #[test]
    fn two_byte_characters_are_read() {
        assert_eq!(decode_text(b"demain! \x81\xF4"), "demain! ♪");
        assert_eq!(decode_text(b"\x81\xA8 l\x85"), "→ l…");
        // Not one the font has: left as it is.
        assert_eq!(decode_text(b"\x81A").chars().count(), 2);
        // The signs of the male and the female, where Windows-1252 has ½ and ¾.
        assert_eq!(decode_text(b"Nidoran\xBE"), "Nidoran♀");
        assert_eq!(decode_text(b"Nidoran\xBD"), "Nidoran♂");
    }

    #[test]
    fn parses_offsets_and_latin_text() {
        let data = [&[8u8, 0, 0, 0, 11, 0, 0, 0][..], b"ab\0", &[0xE9, b'!', 0]].concat();
        let strings = Strings::parse(&data).unwrap();
        assert_eq!(strings.len(), 2);
        assert_eq!(strings.get(0), Some("ab"));
        assert_eq!(strings.get(1), Some("é!"));
        assert_eq!(strings.find_pair("ab", "é!"), Some(0));
    }

    /// Needs a real dump: `DUNGEON_ROM=path cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn real_rom_name_blocks_line_up_across_languages() {
        let rom = crate::nds::test_rom();
        let english = Names::load(&rom, 'e').unwrap();
        assert_eq!(english.moves[1], "Iron Tail");
        assert_eq!(english.items[70], "Oran Berry");
        assert_eq!(english.monsters[1], "Bulbasaur");
        // Further ids from pmdsky-debug's enums: the blocks stay aligned.
        assert_eq!(english.moves[3], "Yawn");
        assert_eq!(english.items[109], "Apple");
        assert_eq!(english.items[110], "Big Apple");
        assert_eq!(english.monsters[25], "Pikachu");
        assert_eq!(english.dungeons[0], "Test Dungeon");
        assert_eq!(english.dungeons[3], "Drenched Bluff");

        // Place ids of levels: dungeons first, then places.
        assert_eq!(english.places[2], "Drenched Bluff");
        assert_eq!(english.places[185], "");

        let french = Names::load(&rom, 'f').unwrap();
        // No formatting tag is left in a name shown to the player.
        let tagged: Vec<&String> = [&french.items, &french.moves, &french.monsters].into_iter().flatten().filter(|name| name.contains('[')).collect();
        assert!(tagged.is_empty(), "{tagged:?}");
        println!("item 183: {:?}, item 1: {:?}", french.items[183], french.items[1]);
        assert_eq!(french.places[1], "Grotte Littorale");
        assert_eq!(french.places[196], "Plage");
        assert_eq!(french.places[277], "Guilde de Grodoudou");
        assert_eq!(french.places[303], "Bourg-Trésor");
        assert_eq!(french.items[109], "Pomme");
        assert_eq!(french.item_descriptions[84], "Provoque le sommeil.");
        assert!(french.item_descriptions[70].starts_with("Rend des "));
        assert_eq!(french.monsters[1], "Bulbizarre");
        // (Unused moves past 400 are named by a tag alone.)
        assert!(french.moves.iter().skip(1).take(400).all(|name| !name.is_empty()));
    }
}
