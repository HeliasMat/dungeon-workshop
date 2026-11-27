//! The room a scene's sprites have on the DS (tools/ds/LIMITS.md, "Species
//! in a scene"). In ground mode Sky loads each species a scene shows once,
//! its whole `m_ground.bin` sprite decompressed, into
//! `GROUND_MEMORY_ARENA_2` (0xAFCE4 bytes usable, 32 blocks), one after
//! the other in the layout's order. It is bytes, not a count: 17 of Sky's
//! species filled it in the_brawl. Our c-of-time hook GroundSpriteRoom
//! (tools/ds/patches.toml) puts a sprite that no longer fits there in
//! `GROUND_MEMORY_ARENA_1` (0x63B04 bytes, 0xB300 of them the ground
//! effects', 0x20000 kept free for them): 0xE84E8 bytes in all. Past the
//! room a sprite is drawn as FixMemorySoftlock's magenta square.
//!
//! The export never lets that happen in silence: a scene over the room is
//! split at a moment into two DS scenes when each part fits
//! ([`split_at`]), else the species least needed leave it
//! ([`leave_out`]); the film's DS remarks say so first (`R-DS-SPRITES`).

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use engine::{Beat, BodyBeat, FlowBeat, PlaceBeat, ViewBeat};

use super::sprites::SKY_SPECIES;
use crate::tables::game_file;

/// `GROUND_MEMORY_ARENA_2`'s usable bytes (measured in the EU game's memory).
pub const ARENA_2: u32 = 0xAFCE4;
/// Its blocks (32), one taken before the sprites come.
pub const ARENA_2_BLOCKS: usize = 31;
/// `GROUND_MEMORY_ARENA_1`'s bytes, what the ground effects take of it as
/// a scene starts (0xB300 in the films measured), and what the hook keeps
/// free in one block after a sprite (0x20000).
pub const ARENA_1: u32 = 0x63B04;
pub const ARENA_1_EFFECTS: u32 = 0xB300;
pub const ARENA_1_KEPT: u32 = 0x20000;
/// Its blocks (52), 9 taken by the effects.
pub const ARENA_1_BLOCKS: usize = 43;

/// The hook giving arena 1, as tools/ds/patches.toml names it.
pub const HOOK: &str = "GroundSpriteRoom";

/// Each species' ground sprite size, decompressed: Sky's read from one's
/// own ROM by `tools/ds/ground_sizes.py`, SpriteCollab's added as
/// `tools/ds/add_monsters.py` makes them (the data, never the repository).
pub const GROUND_SIZES: &str = "extra/ds_ground_sizes.txt";

/// A species of Sky's without the table: its sprites take 0x3000-0x14000
/// bytes, about 40 KB on average; a little over.
pub const SKY_GUESS: u32 = 56_000;

/// A species SpriteCollab adds, its size not known nor its sheets' cells:
/// 0xE230-0x130D0 seen (57-77 KB), a little over.
pub const ADDED_GUESS: u32 = 80_000;

/// An item shown as an object (its icon's sprite, a few KB).
pub const OBJECT_GUESS: u32 = 4_096;

/// The arenas a scene's sprites go to, in turn: (bytes, blocks) each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
    pub arenas: Vec<(u32, usize)>,
}

impl Room {
    /// Sky's own: arena 2 only.
    pub fn sky() -> Room {
        Room { arenas: vec![(ARENA_2, ARENA_2_BLOCKS)] }
    }

    /// With GroundSpriteRoom: arena 2, then what arena 1 gives.
    pub fn with_hook() -> Room {
        Room { arenas: vec![(ARENA_2, ARENA_2_BLOCKS), (ARENA_1 - ARENA_1_EFFECTS - ARENA_1_KEPT, ARENA_1_BLOCKS)] }
    }

    /// The room the export's base gives: the hook's when patches.toml has
    /// it on, as `DW_PATCHES` changes the choice.
    pub fn current() -> Room {
        if super::patches::Patches::current().has(HOOK) { Room::with_hook() } else { Room::sky() }
    }

    /// Its bytes in all.
    pub fn bytes(&self) -> u32 {
        self.arenas.iter().map(|(bytes, _)| bytes).sum()
    }

    /// Whether the hook's arena is in it.
    pub fn hooked(&self) -> bool {
        self.arenas.len() > 1
    }

    /// How the notes name it.
    pub fn said(&self) -> String {
        if self.hooked() {
            format!("the DS holds {} KB of them (GroundSpriteRoom on)", kb(self.bytes()))
        } else {
            format!("Sky holds {} KB of them (GroundSpriteRoom off)", kb(self.bytes()))
        }
    }

    /// Where each sprite goes, loaded in turn: the first arena it fits in
    /// (each filled from its start, as the game does), else none (a square).
    pub fn place(&self, costs: &[u32]) -> Vec<Option<usize>> {
        let mut left: Vec<(u32, usize)> = self.arenas.clone();
        costs
            .iter()
            .map(|&cost| {
                let at = left.iter().position(|(bytes, blocks)| *bytes >= cost && *blocks > 0)?;
                left[at].0 -= cost;
                left[at].1 -= 1;
                Some(at)
            })
            .collect()
    }

    /// Whether every sprite finds room.
    pub fn holds(&self, costs: &[u32]) -> bool {
        self.place(costs).iter().all(Option::is_some)
    }
}

/// Whether the hook is on: patches.toml's `default`, then `DW_PATCHES`
/// (`+Name` on, `-Name` off, `only:A,B` those alone), as megapatch.py reads them.
pub fn hook_on(patches: &str, choice: &str) -> bool {
    super::patches::Patches::read(patches, choice, false).has(HOOK)
}

/// The sizes of `GROUND_SIZES`, by species, read once a data folder.
fn sizes(data: &str) -> Arc<HashMap<u16, u32>> {
    static CACHE: super::sprites::PerData<HashMap<u16, u32>> = OnceLock::new();
    let mut kept = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    kept.entry(data.to_string())
        .or_insert_with(|| {
            let told = if data.is_empty() { String::new() } else { std::fs::read_to_string(game_file(data, GROUND_SIZES)).unwrap_or_default() };
            Arc::new(read_sizes(&told))
        })
        .clone()
}

/// `species<TAB>bytes` lines, `#` comments.
fn read_sizes(told: &str) -> HashMap<u16, u32> {
    told.lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| {
            let (species, bytes) = line.split_once('\t')?;
            Some((species.trim().parse().ok()?, bytes.trim().parse().ok()?))
        })
        .collect()
}

/// A SpriteCollab species' sprite guessed from its sheets' cells (the
/// data's `sprites.txt`: each animation's cell and frames), before its
/// sprite is made once (`add_monsters.py` then tells its size): its
/// cells' pixels, the 8 ways, over `PIXELS_A_BYTE` (Sky's sprites keep 4
/// bits a pixel, their empty tiles left out, a frame used twice once).
fn added_estimate(data: &str, species: u16) -> Option<u32> {
    if data.is_empty() {
        return None;
    }
    let text = assets::data::text(data, "sprites.txt");
    let wanted = species.to_string();
    let mut pixels = 0u64;
    let mut found = false;
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 7 || fields[0] != wanted {
            continue;
        }
        let number = |at: usize| fields[at].trim().parse::<u64>().unwrap_or(0);
        let frames = fields[4].split(',').filter(|one| !one.trim().is_empty()).count() as u64;
        pixels += number(2) * number(3) * frames.max(1) * 8;
        found = true;
    }
    found.then(|| (pixels / PIXELS_A_BYTE) as u32 + 4_096)
}

/// The sheets' pixels for a byte of sprite (`added_estimate`): 46-64 on
/// the far shore's 15 as made (57-78 KB); the fewest, on the safe side.
const PIXELS_A_BYTE: u64 = 48;

/// The bytes a species' sprite takes in a scene (the species as the DS
/// shows it): the table's, else a guess on the safe side.
pub fn sprite_bytes(data: &str, species: u16) -> u32 {
    let bytes = match sizes(data).get(&species) {
        Some(&bytes) => bytes,
        None if species >= SKY_SPECIES => added_estimate(data, species).unwrap_or(ADDED_GUESS),
        None => SKY_GUESS,
    };
    (bytes + 15) & !15
}

/// Each species once, in the order it first comes.
pub fn once(kinds: &[u16]) -> Vec<u16> {
    let mut seen = Vec::new();
    for kind in kinds {
        if !seen.contains(kind) {
            seen.push(*kind);
        }
    }
    seen
}

/// What each sprite a scene loads costs, in the order they load: each
/// species once, then its objects.
pub fn costs(data: &str, kinds: &[u16], objects: usize) -> Vec<u32> {
    once(kinds).iter().map(|&kind| sprite_bytes(data, kind)).chain(std::iter::repeat_n(OBJECT_GUESS, objects)).collect()
}

/// What a scene's sprites take in all.
pub fn scene_bytes(data: &str, kinds: &[u16], objects: usize) -> u32 {
    costs(data, kinds, objects).iter().sum()
}

/// Whether a scene's species (in the order they load) and objects fit.
pub fn fits(data: &str, room: &Room, kinds: &[u16], objects: usize) -> bool {
    room.holds(&costs(data, kinds, objects))
}

/// The species to leave out for the rest to fit: `load_order` the scene's
/// (their order kept for the rest), `least_needed_first` those that may
/// leave, in turn, until the others fit (past them, what is left is left).
pub fn leave_out(data: &str, room: &Room, load_order: &[u16], least_needed_first: &[u16], objects: usize) -> Vec<u16> {
    let mut left = Vec::new();
    let mut candidates = least_needed_first.iter();
    loop {
        let kept: Vec<u16> = load_order.iter().copied().filter(|kind| !left.contains(kind)).collect();
        if fits(data, room, &kept, objects) {
            return left;
        }
        match candidates.next() {
            Some(kind) => left.push(*kind),
            None => return left,
        }
    }
}

/// A scene's species by its moments: `opening` those put before its
/// first moment and named by no moment, `moments` each moment's (the
/// first: the beats before any), each in the order they come.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub opening: Vec<u16>,
    pub moments: Vec<Vec<u16>>,
    /// The film's line each moment starts at (0: the beats before the first).
    pub lines: Vec<usize>,
    /// How many beats name each species (but those setting the opening).
    pub uses: HashMap<u16, usize>,
}

/// A scene's species by its moments, from its beats: `moment_lines` the
/// film's `moment:` lines (1 the first), `kind` the species a beat's field
/// shows on DS (none: no Pokémon). What comes before the first moment sets
/// the opening (in a scene without moments, putting, turning and finding a
/// place); a body's size (told for every scene at the film's start) is no
/// one there.
pub fn usage_of(beats: &[Beat], moment_lines: &[usize], mut kind: impl FnMut(&str) -> Option<u16>) -> Usage {
    let numbers: Vec<usize> =
        beats.iter().filter_map(|beat| if let Beat::Flow(FlowBeat::Line { number, .. }) = beat { usize::try_from(*number).ok() } else { None }).collect();
    let (first, last) = (numbers.iter().copied().filter(|n| *n > 0).min().unwrap_or(0), numbers.iter().copied().max().unwrap_or(0));
    let starts: Vec<usize> = moment_lines.iter().copied().filter(|line| *line > first && *line < last).collect();
    let mut usage = Usage { moments: vec![Vec::new(); starts.len() + 1], lines: [0].into_iter().chain(starts.iter().copied()).collect(), ..Usage::default() };
    let mut placed = Vec::new();
    let mut number = 0;
    for beat in beats {
        if let Beat::Flow(FlowBeat::Line { number: line, .. }) = beat {
            number = usize::try_from(*line).unwrap_or(0);
        }
        if matches!(beat, Beat::View(ViewBeat::Body { .. }) | Beat::Place(PlaceBeat::Footing { .. })) {
            continue;
        }
        let Some(fields) = beat.fields(&scenes::vocabulary::POKEMON) else { continue };
        let moment = starts.iter().filter(|start| **start < number).count();
        // (Before the first moment, all of it sets the scene: its species go with the first part.)
        let setting = moment == 0
            && (!starts.is_empty() || matches!(beat, Beat::Body(BodyBeat::Put { .. } | BodyBeat::Turn { .. }) | Beat::Place(PlaceBeat::Find { .. })));
        for one in fields.iter().filter_map(|field| kind(field)) {
            if setting {
                placed.push(one);
            } else {
                usage.moments[moment].push(one);
                *usage.uses.entry(one).or_default() += 1;
            }
        }
    }
    for moment in &mut usage.moments {
        *moment = once(moment);
    }
    usage.opening = once(&placed).into_iter().filter(|one| !usage.uses.contains_key(one)).collect();
    usage
}

impl Usage {
    /// The species of the moments `range`, the opening's first when it
    /// starts the scene.
    pub fn part(&self, range: std::ops::Range<usize>) -> Vec<u16> {
        let mut kinds = if range.start == 0 { self.opening.clone() } else { Vec::new() };
        for moment in &self.moments[range] {
            kinds.extend(moment);
        }
        once(&kinds)
    }

    /// Every species of the scene.
    pub fn all(&self) -> Vec<u16> {
        self.part(0..self.moments.len())
    }
}

/// Where to split a scene over the room into two DS scenes, at the start of
/// a moment: the first part the moments before it (with the opening), the
/// second the rest, `always` (the hero, the partner) in both; the split
/// that fits both, the most even. None if none does.
pub fn split_at(data: &str, room: &Room, usage: &Usage, always: &[u16], objects: usize) -> Option<usize> {
    let with = |kinds: Vec<u16>| -> Vec<u16> { once(&[always.to_vec(), kinds].concat()) };
    (1..usage.moments.len())
        .filter_map(|at| {
            let (first, second) = (with(usage.part(0..at)), with(usage.part(at..usage.moments.len())));
            (fits(data, room, &first, objects) && fits(data, room, &second, objects))
                .then(|| (scene_bytes(data, &first, objects).max(scene_bytes(data, &second, objects)), at))
        })
        .min()
        .map(|(_, at)| at)
}

/// A size in KB, as the notes say it.
pub fn kb(bytes: u32) -> u32 {
    bytes.div_ceil(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML: &str = "[[patch]]\nname = \"Other\"\ndefault = \"off\"\n\n[[patch]]\nname = \"GroundSpriteRoom\"\nkind = \"cot\"\ndefault = \"on\"\n";

    #[test]
    fn the_hook_is_on_as_the_patches_and_their_choice_say() {
        assert!(hook_on(TOML, ""));
        assert!(!hook_on(TOML, "-GroundSpriteRoom"));
        assert!(hook_on(TOML, "+Other"));
        assert!(!hook_on(TOML, "only:Other"));
        assert!(hook_on(TOML, "only:Other,GroundSpriteRoom"));
        let off = TOML.replace("\"on\"", "\"off\"");
        assert!(hook_on(&off, "+GroundSpriteRoom"));
        assert!(!hook_on(&off, ""));
        // (The repository's own patches: the hook on.)
        assert!(hook_on(crate::ds::patches::PATCHES, ""));
    }

    #[test]
    fn the_room_is_arena_2_then_what_the_hook_gives() {
        assert_eq!(Room::sky().bytes(), 0xAFCE4);
        assert_eq!(Room::with_hook().bytes(), 0xAFCE4 + 0x38804);
        // Filled in turn: a sprite that no longer fits in arena 2 goes to
        // arena 1, and a smaller one after it may still fit in arena 2.
        let placed = Room::with_hook().place(&[0xA0000, 0x10000, 0x8000, 0x30000, 0x10000]);
        assert_eq!(placed, vec![Some(0), Some(1), Some(0), None, Some(1)]);
        assert!(!Room::sky().holds(&[0xA0000, 0x10000]));
    }

    /// The far shore's 15 SpriteCollab sprites as measured, in their
    /// order: 10 in arena 2 (0xA28F0), 3 in arena 1, the last two squares;
    /// without the hook, 10.
    #[test]
    fn the_far_shores_sprites_place_as_measured() {
        let sizes = [0xEA50, 0xF830, 0xF500, 0x10030, 0x130B0, 0xE230, 0x12DB0, 0x11A20, 0xE2B0, 0x113E0, 0x130D0, 0x116A0, 0x107D0, 0xFEC0, 0x12D40];
        let placed = Room::with_hook().place(&sizes);
        assert_eq!(placed.iter().filter(|at| **at == Some(0)).count(), 10);
        assert_eq!(placed.iter().filter(|at| at.is_some()).count(), 13);
        assert_eq!(placed[13..], [None, None]);
        assert_eq!(Room::sky().place(&sizes).iter().filter(|at| at.is_some()).count(), 10);
    }

    /// Without the table: Sky's at 56 KB, SpriteCollab's at 80 KB (no data).
    #[test]
    fn the_guesses_stay_on_the_safe_side() {
        let room = Room::sky();
        assert!(fits("", &room, &(1..=12).collect::<Vec<u16>>(), 0));
        assert!(!fits("", &room, &(1..=13).collect::<Vec<u16>>(), 0));
        assert!(fits("", &room, &(2001..=2009).collect::<Vec<u16>>(), 0));
        assert!(!fits("", &room, &(2001..=2010).collect::<Vec<u16>>(), 0));
        // With the hook: 11 of SpriteCollab's at the guess.
        assert!(fits("", &Room::with_hook(), &(2001..=2011).collect::<Vec<u16>>(), 0));
        assert!(!fits("", &Room::with_hook(), &(2001..=2012).collect::<Vec<u16>>(), 0));
        // (A species twice is one sprite.)
        assert!(fits("", &room, &[25; 40], 0));
        assert_eq!(scene_bytes("", &[25, 25, 4], 0), 2 * SKY_GUESS);
    }

    #[test]
    fn the_least_needed_leave_first_until_the_rest_fit() {
        let room = Room::sky();
        let order: Vec<u16> = (2001..=2015).collect();
        let ranked: Vec<u16> = (2001..=2015).rev().collect();
        assert_eq!(leave_out("", &room, &order, &ranked, 0), (2010..=2015).rev().collect::<Vec<u16>>());
        assert!(leave_out("", &room, &[1, 4, 7], &[1, 4, 7], 0).is_empty());
        // (Only those that may leave: past them, what is left is left.)
        assert_eq!(leave_out("", &room, &order, &[2001], 0), vec![2001]);
    }

    #[test]
    fn the_objects_share_the_room() {
        let sky: Vec<u16> = (1..=12).collect();
        assert!(fits("", &Room::sky(), &sky, 11));
        assert!(!fits("", &Room::sky(), &sky, 12));
    }

    #[test]
    fn a_scene_splits_where_both_parts_fit_the_most_even() {
        let room = Room::sky();
        // 4 moments of 4 SpriteCollab species each: 16 in all, 9 fit.
        let moments: Vec<Vec<u16>> = (0..4).map(|at| (0..4).map(|one| 2000 + at * 4 + one).collect()).collect();
        let usage = Usage { moments, ..Usage::default() };
        assert!(!fits("", &room, &usage.all(), 0));
        assert_eq!(split_at("", &room, &usage, &[], 0), Some(2));
        // The hero and the partner in both parts: too many for Sky's room, at 2 with the hook's.
        assert_eq!(split_at("", &room, &usage, &[1, 4], 0), None);
        assert_eq!(split_at("", &Room::with_hook(), &usage, &[1, 4], 0), Some(2));
        // One moment holding too many: no split.
        let all = Usage { moments: vec![vec![2001], (2001..=2012).collect()], ..Usage::default() };
        assert_eq!(split_at("", &room, &all, &[], 0), None);
        // Those put at the opening and named by no moment go with the first part.
        let put = Usage { opening: (2100..2104).collect(), moments: vec![(2001..=2004).collect(), (2005..=2009).collect()], ..Usage::default() };
        assert_eq!(put.part(0..1).len(), 8);
        assert_eq!(split_at("", &room, &put, &[], 0), Some(1));
    }
}
