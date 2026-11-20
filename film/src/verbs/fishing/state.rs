//! What fishing keeps as a film goes: what is in the water and who swims
//! in the scene at hand (gone at the next one), what was caught and the
//! records (the whole film).

use std::collections::{BTreeMap, BTreeSet};

use crate::film::Caught;

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
    /// Who rides on whose back (`ride`), and where those swimming were last
    /// sent (`swim`).
    pub mounts: BTreeMap<String, String>,
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

impl FishingState {
    /// A new scene: nothing of the last one's water, what was caught kept.
    pub fn new_scene(&mut self) {
        *self = FishingState { catches: std::mem::take(&mut self.catches), records: std::mem::take(&mut self.records), ..FishingState::default() };
    }
}
