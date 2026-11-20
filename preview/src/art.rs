//! What a film shows, read from the folder of the game's files: the
//! interface, the tables, who is who. (The pictures themselves are the
//! shell's `Art`: this one reads through to it.)

use std::ops::Deref;

use rules::tables::Tables;
use stage_view::stage::Cast;
use stage_view::ui::UiArt;

pub struct Art {
    files: shell::art::Art,
    pub ui: UiArt,
    pub tables: Tables,
    pub cast: Cast,
}

impl Deref for Art {
    type Target = shell::art::Art;

    fn deref(&self) -> &shell::art::Art {
        &self.files
    }
}

impl Art {
    /// The folder's files, or what is wrong with them; that project's own
    /// pictures, sounds and recipes (`project`, through the shell's art).
    pub fn open_in(folder: &str, project: assets::project::Project) -> Result<Self, String> {
        let read = |file: &str| shell::data::read(folder, file).ok_or(format!("{folder}/{file} : introuvable"));
        let ui = UiArt::unpack(&read("interface.bin")?).ok_or("interface.bin unreadable")?;
        let tables = bincode::deserialize(&read("dungeons.bin")?).map_err(|error| format!("dungeons.bin unreadable: {error}"))?;
        Ok(Art { files: shell::art::Art::open_in(folder, project), ui, tables, cast: Cast::open(folder) })
    }
}
