//! What the ROM sets up beside its plain tables: the balance, the pictures
//! the tables carry, the interface.

use std::fmt::Display;

use rom_formats::{self, RomError};
use rules::dungeon::{Constants, MoveSlot, TypeChart};
use stage_view::ui::{self, UiArt};

use super::{LANGUAGE, PLAYER_MD_INDEX, RomData};

/// Reports a part of the ROM that could not be read.
fn load<T, E: Display>(what: &str, result: Result<T, E>) -> Option<T> {
    result.inspect_err(|error| eprintln!("{what} unavailable: {error}")).ok()
}

pub(crate) struct Interface {
    /// Without it every hit is neutral.
    pub types: Option<TypeChart>,
    /// Without them effects use zeros.
    pub constants: Constants,
    pub trap_art: Vec<Vec<u8>>,
    pub stairs_tile: usize,
    pub weather_colors: Vec<Vec<u8>>,
    pub item_art: Vec<Vec<u8>>,
    pub regular_attack: Option<MoveSlot>,
    pub money_table: Vec<i32>,
    pub ui_art: Option<UiArt>,
}

impl Interface {
    pub(crate) fn of(data: &RomData) -> Self {
        let mut constants = load("balance constants", rom_formats::types::load_constants(&data.rom)).unwrap_or_default();
        // (The game doubles the species' value.)
        constants.regen_speed = data.monsters.species(PLAYER_MD_INDEX).map_or(0, |species| species.hp_regeneration as i32 * 2);
        let pack = || data.rom.file("DUNGEON/dungeon.bin").ok_or(RomError::Truncated);
        Interface {
            types: load("type matchups", rom_formats::types::load_type_chart(&data.rom)),
            constants,
            trap_art: load("trap graphics", pack().and_then(rom_formats::tileset::load_trap_graphics)).unwrap_or_default(),
            stairs_tile: rom_formats::tileset::STAIRS_TILE,
            weather_colors: load("weather colours", pack().and_then(rom_formats::tileset::load_weather_colors)).unwrap_or_default(),
            item_art: data.item_art(),
            regular_attack: data.regular_attack(),
            money_table: load("money amounts", rom_formats::types::load_money_table(&data.rom)).unwrap_or_default(),
            ui_art: load("interface graphics", ui_art(data)),
        }
    }
}

/// (The original's interface needs all of its pieces, but for the pictures
/// it can do without.)
fn ui_art(data: &RomData) -> Result<UiArt, RomError> {
    let font = data.rom.file("FONT/kanji_rd.dat").ok_or(RomError::Truncated).and_then(rom_formats::font::parse)?;
    let picture = |name: &str| data.rom.file(name).and_then(|file| rom_formats::menus::load_bgp(file).ok()).unwrap_or_default();
    let (cursor, more) = rom_formats::menus::load_cursors(&data.rom)?;
    Ok(ui::UiArt {
        cursor,
        more,
        font,
        frame: rom_formats::menus::load_frame(&data.rom, 0)?,
        frame_green: rom_formats::menus::load_frame(&data.rom, 2)?,
        frame_pink: rom_formats::menus::load_frame(&data.rom, 1)?,
        frame_types: (1..5).flat_map(|kind| [0, 1, 2].map(|variant| rom_formats::menus::load_frame_of(&data.rom, kind, variant).unwrap_or_default())).collect(),
        title_logo: data.title_logo().unwrap_or_default(),
        menu_art: picture("BACK/s09p10a.bgp"),
        menu_backdrop: picture("BACK/expback.bgp"),
        title_art: picture("TOP/title_0.bgp"),
        title_sprites: data.title_sprites(),
        marks: rom_formats::menus::load_marks(&data.rom)?,
        hud: rom_formats::menus::load_hud(&data.rom, LANGUAGE)?,
        text_colors: rom_formats::menus::load_text_colors(&data.rom)?,
        backdrop: rom_formats::menus::load_backdrop(&data.rom)?,
        strings: std::sync::Arc::new(rom_formats::menus::load_strings(&data.rom, LANGUAGE)?),
    })
}
