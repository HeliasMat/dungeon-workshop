//! What a dungeon keeps as a film goes: which, its floor, its team and its
//! mission (the whole film); what lies on the floor at hand (gone at the
//! next scene).

use std::collections::BTreeSet;

use crate::film::Kept;

/// A shop: its keeper, and each thing laid out (its word, its code, its price).
pub type Shop = (String, Vec<(String, String, i32)>);

pub struct DungeonState {
    pub dungeon: Option<String>,
    /// The floor of it the film is on, and whether its floors go up.
    pub floor: i32,
    pub going_up: bool,
    /// Who the dungeon's top line follows (`dungeon_hud`), if anyone, with
    /// his level and his full health.
    pub dungeon_hud: Option<(String, i32, i32)>,
    /// That top line put by for a fight, until it ends.
    pub hud_put_by: bool,
    /// Who joined the team on the floors (`new_recruit`); the team the map shows,
    /// and whether it is found as one goes.
    pub recruits: Vec<String>,
    /// The guild's: the mission the team took (`take_job`).
    pub mission: Option<String>,
    pub map_team: Option<(bool, Vec<String>)>,
    /// The rocks of a cave-in, in the way (`rockslide`, `smash`).
    pub rubble: Vec<String>,
    /// The foes beaten and gone (`foe_ko`): where they lay is kept.
    pub downed: BTreeSet<String>,
    /// The crystals lit so far (`crystal`).
    pub lit: BTreeSet<String>,
    /// The footprints on the ground (`footprints`), until `clear_footprints`.
    pub tracks: Vec<String>,
    /// A shop on the floor (`shop`): its keeper, and what it sells, the
    /// code of each thing laid out and its price.
    pub shop: Option<Shop>,
    /// The floor is dark (`darkness`): only round the team can be seen.
    pub dark: bool,
    /// The traps laid on the floor (`trap`), until `trap off`.
    pub traps: Vec<String>,
}

impl Default for DungeonState {
    fn default() -> Self {
        DungeonState {
            dungeon: None,
            floor: 1,
            going_up: false,
            dungeon_hud: None,
            hud_put_by: false,
            recruits: Vec::new(),
            mission: None,
            map_team: None,
            rubble: Vec::new(),
            downed: BTreeSet::new(),
            lit: BTreeSet::new(),
            tracks: Vec::new(),
            shop: None,
            dark: false,
            traps: Vec::new(),
        }
    }
}

impl Kept for DungeonState {
    /// A new scene: what lies on the floor at hand goes; the lights back on
    /// if it was dark.
    fn new_scene(&mut self) -> Option<engine::Beat> {
        let dark = self.dark;
        *self = DungeonState {
            dungeon: self.dungeon.take(),
            floor: self.floor,
            going_up: self.going_up,
            dungeon_hud: self.dungeon_hud.take(),
            hud_put_by: self.hud_put_by,
            recruits: std::mem::take(&mut self.recruits),
            mission: self.mission.take(),
            map_team: self.map_team.take(),
            ..DungeonState::default()
        };
        dark.then(crate::emit::spotlight_off)
    }

    /// The top line of a dungeon's floor shows its log.
    fn logs(&self) -> bool {
        self.dungeon_hud.is_some()
    }
}
