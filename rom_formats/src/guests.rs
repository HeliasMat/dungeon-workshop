//! The guests: monsters that join the team for a story dungeon (Bidoof,
//! Grovyle, Chatot...), and which dungeons add which.
//!
//! `GUEST_MONSTER_DATA` (ARM9, pmdsky-debug's `struct guest_monster`) is 18
//! entries of 36 bytes: a pointer to a name (null in every entry), the monster, the dungeon and
//! floor it joined at, four moves, its HP, level, IQ, Attack, Sp. Atk,
//! Defense, Sp. Def (one byte each, two apart) and experience.
//!
//! Which dungeon adds which guests is code in the original game. SkyTemple's
//! EditGuestPokemon patch (Frostbyte, GPL-3) turns it into a table of 2
//! bytes per dungeon (`EXTRA_DUNGEON_DATA`) and adds more guests after
//! (`GUEST_MONSTER_DATA2`); on a patched ROM both are read, on the others
//! the table the patch starts from, which does what the original's code
//! does (`EoS-ASM-Hacks/src/EditGuestPokemon.asm`). (Both also keep one
//! check of their own: no guest in Brine Cave, 0x23, once
//! `PERFOMANCE_PROGRESS_LIST[25]` is set.) Layouts from
//! skytemple-files (`hardcoded/guest_pokemon.py`, GPL-3).

use super::nds::{NdsRom, RomError};
use crate::bytes::{u16_le, u32_le};
use crate::sym;

const ENTRY: usize = 36;
/// Guests of the original game, in `GUEST_MONSTER_DATA`.
const FIRST_TABLE: usize = 18;
/// Dungeons in `EXTRA_DUNGEON_DATA`.
pub const DUNGEON_COUNT: usize = 0xB4;
/// Where the patch writes over the original's code, in the ARM9 binary by
/// region (EU, NA, JP), and the instruction there before (skytemple-files'
/// `patch/handler/edit_guest_pokemon.py`).
const PATCH_AT: [usize; 3] = [0x4EF00, 0x4EBC8, 0x4EF20];
const ORIGINAL_INSTRUCTION: u32 = 0xE356_0044;
/// The patch's table as it starts, the original game's choice: (dungeon,
/// entry), the dungeons not listed adding no guest.
const ORIGINAL_DUNGEONS: [(usize, u16); 39] = [
    (8, 0x0003),
    (9, 0x0010),
    (10, 0x000F),
    (11, 0x0005),
    (32, 0x0006),
    (33, 0x0B0C),
    (34, 0x0009),
    (35, 0x000A),
    (38, 0x404D),
    (39, 0x404D),
    (40, 0x404D),
    (41, 0x4080),
    (42, 0x4080),
    (43, 0x4080),
    (62, 0x0080),
    (63, 0x0080),
    (64, 0x0080),
    (65, 0x0080),
    (66, 0x0080),
    (67, 0x008E),
    (68, 0x008E),
    (69, 0x008E),
    (111, 0x0011),
    (112, 0x0011),
    (113, 0x0011),
    (114, 0x0011),
    (115, 0x0011),
    (116, 0x0011),
    (117, 0x0011),
    (118, 0x0011),
    (119, 0x0011),
    (120, 0x0011),
    (121, 0x0011),
    (122, 0x0011),
    (123, 0x8012),
    (124, 0x8004),
    (128, 0x0201),
    (135, 0x0008),
    (164, 0x0007),
];

#[derive(Clone, Debug)]
pub struct Guest {
    pub monster: u16,
    /// The dungeon and floor it is said to have joined at.
    pub joined_at: u8,
    pub joined_floor: u8,
    pub moves: [u16; 4],
    pub hp: i16,
    pub level: u8,
    pub iq: i16,
    pub attack: u8,
    pub sp_attack: u8,
    pub defense: u8,
    pub sp_defense: u8,
    pub exp: i32,
}

/// What a dungeon does on the first visit.
#[derive(Clone, Copy, Debug, Default)]
pub struct DungeonGuests {
    /// The guests added to the team (indexes in the guest list).
    pub guests: [Option<usize>; 2],
    /// Hidden Land's rules (no sending home, no change of leader, out when
    /// the partner faints) until the dungeon is cleared, and after.
    pub hidden_land_rules: bool,
    pub hidden_land_rules_cleared: bool,
    /// No recruiting there.
    pub no_recruiting: bool,
    /// The guests join only while the story's `SIDE01_BOSS2ND` is 0.
    pub only_before_side01_boss2nd: bool,
}

impl DungeonGuests {
    fn parse(entry: u16) -> Self {
        let guest = |bits: u16| (bits as usize).checked_sub(1);
        Self {
            guests: [guest(entry & 0x3F), guest((entry >> 8) & 0x3F)],
            hidden_land_rules: entry & 0x80 != 0,
            no_recruiting: entry & 0x40 != 0,
            only_before_side01_boss2nd: entry & 0x8000 != 0,
            hidden_land_rules_cleared: entry & 0x4000 != 0,
        }
    }
}

pub struct Guests {
    pub guests: Vec<Guest>,
    /// By dungeon.
    pub dungeons: Vec<DungeonGuests>,
    /// Whether the ROM has the EditGuestPokemon patch.
    pub patched: bool,
}

pub fn load(rom: &NdsRom) -> Result<Guests, RomError> {
    let region = rom.region()?;
    let (base, arm9) = rom.arm9().ok_or(RomError::Truncated)?;
    let table = |symbol: sym::Symbol| -> Result<&[u8], RomError> {
        let start = symbol.address[region].checked_sub(base).ok_or(RomError::Truncated)? as usize;
        arm9.get(start..start + symbol.length[region] as usize).ok_or(RomError::Truncated)
    };
    let patched = u32_le(arm9, PATCH_AT[region])? != ORIGINAL_INSTRUCTION;
    let guest = |e: &[u8; ENTRY]| -> Result<Guest, RomError> {
        let half = |at: usize| u16_le(e, at);
        Ok(Guest {
            monster: half(4)?,
            joined_at: e[6],
            joined_floor: e[7],
            moves: [half(8)?, half(10)?, half(12)?, half(14)?],
            hp: half(0x10)? as i16,
            level: e[0x12],
            iq: half(0x14)? as i16,
            attack: e[0x16],
            sp_attack: e[0x18],
            defense: e[0x1A],
            sp_defense: e[0x1C],
            exp: u32_le(e, 0x20)? as i32,
        })
    };
    // An entry of zeros ends the list.
    let mut entries: Vec<&[u8; ENTRY]> = table(sym::GUEST_MONSTER_DATA)?.as_chunks::<ENTRY>().0.iter().take(FIRST_TABLE).collect();
    if patched && entries.iter().all(|e| e.iter().any(|&b| b != 0)) {
        entries.extend(table(sym::GUEST_MONSTER_DATA2)?.as_chunks::<ENTRY>().0);
    }
    let guests = entries.into_iter().take_while(|e| e.iter().any(|&b| b != 0)).map(guest).collect::<Result<_, _>>()?;
    let dungeons = if patched {
        let data = table(sym::EXTRA_DUNGEON_DATA)?;
        (0..DUNGEON_COUNT).map(|d| u16_le(data, d * 2).map(DungeonGuests::parse)).collect::<Result<_, _>>()?
    } else {
        let mut dungeons = vec![DungeonGuests::default(); DUNGEON_COUNT];
        for (dungeon, entry) in ORIGINAL_DUNGEONS {
            dungeons[dungeon] = DungeonGuests::parse(entry);
        }
        dungeons
    };
    Ok(Guests { guests, dungeons, patched })
}
