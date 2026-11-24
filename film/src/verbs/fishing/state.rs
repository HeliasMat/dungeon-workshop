//! What fishing keeps as a film goes: what is in the water and who swims
//! in the scene at hand (gone at the next one), what was caught and the
//! records (the whole film).

use std::collections::{BTreeMap, BTreeSet};

use crate::film::Kept;

/// One thing a fisher pulled out of the water: who or what it is (its
/// code, or who), its name as said, its word (the Pokémon's, or the thing's
/// key); its size once measured; in his bucket or let go (still caught,
/// told so at the end of the day); where it came out.
#[derive(Clone, Debug)]
pub struct Caught {
    pub code: String,
    pub name: String,
    pub word: String,
    pub pokemon: bool,
    pub size: Option<i32>,
    pub kept: bool,
    pub released: bool,
    pub water: Option<(i32, i32)>,
    pub landed: Option<(i32, i32)>,
}

#[derive(Default)]
pub struct FishingState {
    /// Who has a line in the water: the code of what floats there, and the
    /// point of the water it was cast to, if one was told.
    pub fishing: BTreeMap<String, (String, Option<(String, String)>)>,
    /// Who cast last: the line a fish bites (`bite`).
    pub last_cast: Option<String>,
    /// What each fisher put on his hook (`bait`), until a fish takes it.
    pub bait: BTreeMap<String, String>,
    /// A fish's shadow under someone's bobber (`shadow`): its code.
    pub shadows: BTreeMap<String, String>,
    /// Whose line's tension is shown (`struggle a 3 tension`), till it is in.
    pub tension: Option<String>,
    /// Those watching the fishing (`spectators`): they cheer a catch, groan
    /// at one lost.
    pub fans: Vec<String>,
    /// Those pulling behind each fisher (`chain`), one behind the other.
    pub chains: BTreeMap<String, Vec<String>>,
    /// The glowing fish round each fisher's lantern (`lantern`).
    pub lanterns: BTreeMap<String, Vec<String>>,
    /// Where those swimming were last sent (`swim`).
    pub swum: BTreeMap<String, (i32, i32)>,
    /// The net basket each swimmer tows (`swim ... with net`): its code.
    pub nets: BTreeMap<String, String>,
    /// The baskets with fish in them, and those towed to the right (their
    /// pennant streaming left).
    pub full_nets: BTreeSet<String>,
    pub net_right: BTreeMap<String, bool>,
    /// Those swimming the way they were last sent, to the right or not;
    /// the birds wheeling over the sea (`gulls`).
    pub swims_right: BTreeMap<String, bool>,
    pub gulls: Vec<String>,
    /// What each fisher pulled out of the water (`haul`), the last at the end.
    pub catches: BTreeMap<String, Vec<Caught>>,
    /// The biggest of each kind measured (`measure`), in centimetres.
    pub records: BTreeMap<String, i32>,
}

impl Kept for FishingState {
    /// A new scene: nothing of the last one's water, what was caught kept.
    fn new_scene(&mut self) -> Option<engine::Beat> {
        *self = FishingState { catches: std::mem::take(&mut self.catches), records: std::mem::take(&mut self.records), ..FishingState::default() };
        None
    }
}
