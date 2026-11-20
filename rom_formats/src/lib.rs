//! The original game's formats, read from a user-supplied ROM dump, and its
//! scripts played: what only `extractor` needs. Everything else reads the
//! project's own files. No game asset is shipped with this project.

/// The game's original scripts played (the scenes as its script engine
/// runs them: see `vm`), to read them into story files.
pub mod acting;
mod bytes;
pub mod dungeons;
pub mod fixed;
pub mod font;
pub mod guests;
pub mod items;
pub mod kao;
pub mod mapbg;
pub mod mappa;
pub mod menus;
pub mod monster;
pub mod moves;
pub mod nds;
pub mod px;
pub mod quiz;
pub mod scene;
pub mod script;
pub mod sedl;
pub mod smdl;
pub mod swdl;
pub mod sym;
pub mod talk;
pub mod text;
pub mod tileset;
pub mod types;
/// The original script engine's virtual machine.
pub mod vm;
pub mod wan;

pub use mappa::Mappa;
pub use monster::MonsterData;
pub use moves::MoveData;
pub use nds::{NdsRom, RomError};
