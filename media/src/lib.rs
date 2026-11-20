//! What is shown and heard, as data: pictures (`images`) and the game's
//! picture files (`art_files`), tilesets (`tileset`) and the template one's
//! own are drawn on (`template`), the console's colours (`quant`), the font
//! (`font`), songs and banks (`music`) and the synthesizer making sound of
//! them (`audio`), the project's packed files (`pack`). No window, no audio
//! device, no program run, no network: `shell` shows and plays them, the
//! extraction and the décors' tool write them.

pub mod art_files;
pub mod audio;
pub mod font;
pub mod images;
pub mod music;
pub mod pack;
pub mod quant;
pub mod template;
pub mod tileset;
