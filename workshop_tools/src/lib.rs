//! The windows of the Workshop's Tools menu, each over whatever space is
//! looked at:
//!
//! - `sprites`: SpriteCollab, the community's sprites and portraits looked
//!   through and imported into the data (`Browser`);
//! - `tilequant`: any PNG made to the console's rules, written beside it
//!   (`QuantTool`).
//!
//! The window keeps each one while it is open and shows it once a frame.

pub mod sprites;
pub mod tilequant;
