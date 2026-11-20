//! Who a scene has on its stage ([`Who`]): a character of the game's
//! scripts, an unseen spot, any creature, a thing of the scenes, an item.

use std::fmt;

/// Someone or something of a scene's stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Who {
    /// A character of the game's scripts, by its number (`0` the hero,
    /// `10` the partner...).
    Character(u16),
    /// An unseen spot (`p<n>` in the files): something to look at or walk
    /// to, with no picture.
    Spot(u16),
    /// Any creature, by its species (`m<species>`): a scene of this
    /// project's own may bring in whoever it likes, with its name and
    /// portraits.
    Monster(u16),
    /// Another creature of a species (`m<species>.<n>`, `n` from 1): three
    /// Krabby on a beach, each its own.
    Another { species: u16, copy: u16 },
    /// A thing of the scenes (a prop), by its object (`o<object>`), and
    /// which of them (`o<object>.<n>`: as many as wanted).
    Thing { object: u16, copy: u16 },
    /// An item of the game, shown as its picture (`i<item>`; `i<item>.<n>`
    /// for another of it, `copy` telling them apart).
    Item { item: u16, copy: u16 },
    /// Whoever Enter found last (`target` in the files; see
    /// `Director::press`): who it is, the director tells when the beat
    /// is played.
    Target,
    /// A picture of a film's own, by its number there (`d<n>`; `d<n>.<c>`
    /// for another of it).
    Drawn { picture: u16, copy: u16 },
}

/// How the story files the game's scripts are read into number them:
/// unseen spots below 0, then characters, any creature from `MONSTERS`,
/// things from `THINGS`, items from `ITEMS` (each copy `ITEM_COPY`
/// further). Those that do not fit there (a species past 4095, a thing's
/// copy...) are numbered from `WIDE`: a kind, a copy, an id, each whole.
const MONSTERS: i64 = 0x3000;
const THINGS: i64 = 0x4000;
const ITEMS: i64 = 0x8000;
const ITEM_COPY: i64 = 0x1000;
/// The other creatures of a species: from here, each copy `ITEM_COPY`
/// further (past any item's copy a file would write).
const OTHERS: i64 = 0x0100_0000;
/// The pictures of a film's own: past all the others.
const DRAWN: i64 = 0x2000_0000;
/// Past all the numbers of the files: a kind (`<< KIND`), a copy (`<<
/// 16`), an id; under 2^53, so a number of the scene (an `f64`) keeps it.
const WIDE: i64 = 1 << 40;
const KIND: u32 = 36;

impl Who {
    /// Someone as the files write it: a number (as the story files read
    /// from the game's scripts have it, see `Who::from_number`), `p<n>`,
    /// `m<species>`, `o<object>`, `i<item>`, `d<picture>`, each with `.<n>`
    /// for another of it.
    pub fn parse(code: &str) -> Option<Who> {
        if code == "target" {
            return Some(Who::Target);
        }
        if let Some(spot) = code.strip_prefix('p') {
            return Some(Who::Spot(spot.parse().ok()?));
        }
        // (The id, and which of them: `66`, `66.1`.)
        let copied = |code: &str| -> Option<(u16, u16)> {
            let (id, copy) = code.split_once('.').unwrap_or((code, "0"));
            Some((id.parse().ok()?, copy.parse().ok()?))
        };
        if let Some(species) = code.strip_prefix('m') {
            return Some(match copied(species)? {
                (species, 0) => Who::Monster(species),
                (species, copy) => Who::Another { species, copy },
            });
        }
        if let Some(object) = code.strip_prefix('o') {
            let (object, copy) = copied(object)?;
            return Some(Who::Thing { object, copy });
        }
        if let Some(picture) = code.strip_prefix('d') {
            let (picture, copy) = copied(picture)?;
            return Some(Who::Drawn { picture, copy });
        }
        if let Some(item) = code.strip_prefix('i') {
            let (item, copy) = copied(item)?;
            return Some(Who::Item { item, copy });
        }
        Who::from_number(code.parse().ok()?)
    }

    /// The number the story files give someone (see `Who::from_number`).
    pub fn number(self) -> i64 {
        let wide = |kind: i64, copy: u16, id: u16| WIDE + (kind << KIND) + (i64::from(copy) << 16) + i64::from(id);
        match self {
            Who::Spot(spot) => -1 - i64::from(spot),
            Who::Character(number) => i64::from(number),
            Who::Monster(species) if i64::from(species) < THINGS - MONSTERS => MONSTERS + i64::from(species),
            Who::Monster(species) => wide(1, 0, species),
            Who::Another { species, copy } if i64::from(species) < ITEM_COPY => OTHERS + i64::from(copy) * ITEM_COPY + i64::from(species),
            Who::Another { species, copy } => wide(2, copy, species),
            Who::Thing { object, copy: 0 } if i64::from(object) < ITEMS - THINGS => THINGS + i64::from(object),
            Who::Thing { object, copy } => wide(3, copy, object),
            Who::Item { item, copy } if i64::from(item) < ITEM_COPY && ITEMS + (i64::from(copy) + 1) * ITEM_COPY <= OTHERS => {
                ITEMS + i64::from(copy) * ITEM_COPY + i64::from(item)
            }
            Who::Item { item, copy } => wide(4, copy, item),
            Who::Drawn { picture, copy } if i64::from(picture) < ITEM_COPY => DRAWN + i64::from(copy) * ITEM_COPY + i64::from(picture),
            Who::Drawn { picture, copy } => wide(5, copy, picture),
            // (No one the files number.)
            Who::Target => i64::from(i32::MIN),
        }
    }

    /// Someone by the number the story files give it.
    pub(crate) fn from_number(number: i64) -> Option<Who> {
        let small = |value: i64| u16::try_from(value).ok();
        if number >= WIDE {
            let (kind, copy, id) = ((number - WIDE) >> KIND, small(((number - WIDE) >> 16) & 0xffff)?, small(number & 0xffff)?);
            return Some(match kind {
                1 if copy == 0 => Who::Monster(id),
                2 => Who::Another { species: id, copy },
                3 => Who::Thing { object: id, copy },
                4 => Who::Item { item: id, copy },
                5 => Who::Drawn { picture: id, copy },
                _ => return None,
            });
        }
        Some(match number {
            number if number < 0 => Who::Spot(small(-1 - number)?),
            number if number >= DRAWN => Who::Drawn { picture: small((number - DRAWN) % ITEM_COPY)?, copy: small((number - DRAWN) / ITEM_COPY)? },
            number if number >= OTHERS => Who::Another { species: small((number - OTHERS) % ITEM_COPY)?, copy: small((number - OTHERS) / ITEM_COPY)? },
            number if number >= ITEMS => Who::Item { item: small((number - ITEMS) % ITEM_COPY)?, copy: small((number - ITEMS) / ITEM_COPY)? },
            number if number >= THINGS => Who::Thing { object: small(number - THINGS)?, copy: 0 },
            number if number >= MONSTERS => Who::Monster(small(number - MONSTERS)?),
            number => Who::Character(small(number)?),
        })
    }

    /// Whether it is a thing: an item, a prop, a picture of a film's own.
    pub fn is_thing(self) -> bool {
        matches!(self, Who::Thing { .. } | Who::Item { .. } | Who::Drawn { .. })
    }
}

/// As the files write it.
impl fmt::Display for Who {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Who::Character(number) => write!(out, "{number}"),
            Who::Spot(spot) => write!(out, "p{spot}"),
            Who::Monster(species) => write!(out, "m{species}"),
            Who::Another { species, copy } => write!(out, "m{species}.{copy}"),
            Who::Thing { object, copy: 0 } => write!(out, "o{object}"),
            Who::Thing { object, copy } => write!(out, "o{object}.{copy}"),
            Who::Drawn { picture, copy: 0 } => write!(out, "d{picture}"),
            Who::Drawn { picture, copy } => write!(out, "d{picture}.{copy}"),
            Who::Item { item, copy: 0 } => write!(out, "i{item}"),
            Who::Item { item, copy } => write!(out, "i{item}.{copy}"),
            Who::Target => write!(out, "target"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everyone_is_read_as_written() {
        for (code, who) in [
            ("10", Who::Character(10)),
            ("p2", Who::Spot(2)),
            ("-3", Who::Spot(2)),
            ("m25", Who::Monster(25)),
            ("m98.2", Who::Another { species: 98, copy: 2 }),
            ("m98.0", Who::Monster(98)),
            ("12288", Who::Monster(0)),
            ("16390", Who::Thing { object: 6, copy: 0 }),
            ("o66", Who::Thing { object: 66, copy: 0 }),
            ("o66.1", Who::Thing { object: 66, copy: 1 }),
            ("i464", Who::Item { item: 464, copy: 0 }),
            ("i70.2", Who::Item { item: 70, copy: 2 }),
            ("d3", Who::Drawn { picture: 3, copy: 0 }),
            ("d3.2", Who::Drawn { picture: 3, copy: 2 }),
        ] {
            assert_eq!(Who::parse(code), Some(who), "{code}");
            assert_eq!(Who::parse(&who.to_string()), Some(who));
        }
        assert_eq!(Who::parse("m99999"), None);
        assert_eq!(Who::parse("x"), None);
    }

    /// No cap on copies nor on ids, but what a `u16` holds: a thing's
    /// fiftieth copy, a species past 4095; each numbered and back.
    #[test]
    fn copies_and_ids_have_room() {
        for who in [
            Who::Thing { object: 66, copy: 50 },
            Who::Thing { object: 0x7fff, copy: 0 },
            Who::Thing { object: u16::MAX, copy: u16::MAX },
            Who::Monster(9000),
            Who::Another { species: 9000, copy: 300 },
            Who::Item { item: 70, copy: 4000 },
            Who::Item { item: 5000, copy: 1 },
            Who::Drawn { picture: 6000, copy: 2 },
            Who::Monster(133),
            Who::Spot(3),
            Who::Character(10),
        ] {
            assert_eq!(Who::from_number(who.number()), Some(who), "{who}");
            assert_eq!(Who::parse(&who.to_string()), Some(who), "{who}");
            // (A number of the scene keeps it whole.)
            assert_eq!(who.number() as f64 as i64, who.number());
        }
        // (As the files numbered them before.)
        assert_eq!(Who::Monster(133).number(), 0x3000 + 133);
        assert_eq!(Who::Thing { object: 6, copy: 0 }.number(), 16390);
    }
}
