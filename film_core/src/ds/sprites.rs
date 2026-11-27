//! Sky's sprites, as the export and the DS mode read them: which species
//! the game has (`drawn_on_ds`: one SpriteCollab adds is put in the ROM
//! from its sheets; without them a form is drawn as its base species, a
//! Pokémon of a later generation as a stand-in), and which
//! pose each species' own sprite shows for a group of the engine's
//! (`pose_on_ds`: its own group of that name, else the nearest one every
//! sprite has).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::tables::game_file;

/// The species Sky's sprites have: `monster.md`'s entries (0 to 1154).
/// Past them, the Pokémon and forms SpriteCollab adds (2000 + their
/// Pokédex number; variants from 5000), which the ROM has no sprite for.
pub const SKY_SPECIES: u16 = 1155;

/// Who stands in for a Pokémon Sky doesn't have at all: Unown “?”, an
/// odd one no film mistakes for the one it means.
pub const STAND_IN: u16 = 228;

/// How a species is drawn on the DS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drawn {
    /// Sky has it.
    Own,
    /// One SpriteCollab adds, whose sheets the data has: put in the ROM
    /// from them (`tools/ds/add_monsters.py`), drawn as itself, or as the
    /// species given (its base, or the stand-in) where that fails.
    Added(u16),
    /// A form SpriteCollab adds (`pichu_spiky`): as its base species.
    Base(u16),
    /// A Pokémon Sky doesn't have (a later generation's): as [`STAND_IN`].
    StandIn,
}

impl Drawn {
    /// The species the DS shows.
    pub fn species(self, asked: u16) -> u16 {
        match self {
            Drawn::Own | Drawn::Added(_) => asked,
            Drawn::Base(base) => base,
            Drawn::StandIn => STAND_IN,
        }
    }
}

/// A cache of tables, one a data folder.
pub(super) type PerData<T> = OnceLock<Mutex<HashMap<String, Arc<T>>>>;

/// A table of the data's, read once a data folder.
pub(super) fn kept<T: Send + Sync + 'static>(cache: &'static PerData<T>, data: &str, read: impl FnOnce() -> T) -> Arc<T> {
    let mut kept = cache.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    kept.entry(data.to_string()).or_insert_with(|| Arc::new(read())).clone()
}

/// The variants' bases (`variants.txt`: number, whose it is), by number.
fn bases(data: &str) -> Arc<HashMap<u16, u16>> {
    static CACHE: PerData<HashMap<u16, u16>> = OnceLock::new();
    kept(&CACHE, data, || assets::tables::variants(data).into_iter().map(|variant| (variant.number, variant.of)).collect())
}

/// How that species is drawn on the DS (`data`: the data folder, its
/// sheets and `variants.txt`): itself (Sky's, or added from its sheets),
/// its base species, or the stand-in.
pub fn drawn_on_ds(data: &str, species: u16) -> Drawn {
    match without_sheets(data, species) {
        Drawn::Own => Drawn::Own,
        instead if added_groups(data).contains_key(&species) => Drawn::Added(instead.species(species)),
        instead => instead,
    }
}

/// How that species is drawn where its sheets aren't put in the ROM: Sky's
/// own, its base species, or the stand-in.
pub fn without_sheets(data: &str, species: u16) -> Drawn {
    if species < SKY_SPECIES {
        return Drawn::Own;
    }
    match bases(data).get(&species) {
        Some(&base) if base > 0 && base < SKY_SPECIES => Drawn::Base(base),
        _ => Drawn::StandIn,
    }
}

/// The groups past 12 in the order the starters' sprites share (the
/// engine's `AnimId`s are these): 13 EventSleep ... 34 HitGround.
const SHARED: &str =
    "EventSleep Wake Eat Tumble Pose Pull Pain Float DeepBreath Nod Sit LookUp Sink Trip Laying LeapForth Head Cringe LostBalance TumbleBack Faint HitGround";

/// Where each species whose sprite orders its groups past 12 its own way is
/// told: the groups it has and their names, read from one's own ROM by
/// `tools/ds/animations.py` into the data (never in the repository; the
/// names are SkyTemple's). Without it, every species shares the starters'
/// order.
pub const OWN_ORDER: &str = "extra/ds_animations.txt";

type Groups = HashMap<u16, Vec<(u16, String)>>;

/// The groups of the sheets the data has for the species SpriteCollab adds
/// (`sprites.txt`'s rows past Sky's species), as their sprite put in the ROM
/// numbers them: a scene's sheet is its group past the dungeons' five, the
/// dungeons' own their group (as `spritecollab::export` writes them).
fn added_groups(data: &str) -> Arc<Groups> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Groups>>>> = OnceLock::new();
    const DUNGEON: [u16; 5] = [0, 7, 6, 5, 1];
    kept(&CACHE, data, || {
        let mut groups: Groups = HashMap::new();
        for line in assets::data::text(data, "sprites.txt").lines() {
            let mut fields = line.split('\t');
            let (Some(Ok(species)), Some(Ok(index))) = (fields.next().map(str::parse::<u16>), fields.next().map(str::parse::<u16>)) else { continue };
            if species < SKY_SPECIES {
                continue;
            }
            let group = index.checked_sub(5).unwrap_or(DUNGEON[usize::from(index.min(4))]);
            let own = groups.entry(species).or_default();
            if !own.iter().any(|(at, _)| *at == group) {
                own.push((group, group_name(group).to_string()));
            }
        }
        // (Walking and standing, or no sprite the game can show.)
        groups.retain(|_, own| own.iter().any(|(at, _)| *at == 0) && own.iter().any(|(at, _)| *at == 7));
        groups
    })
}

/// Each species' own groups and their names (`OWN_ORDER`, and the sheets of
/// the species added from SpriteCollab's).
fn own_groups(data: &str) -> Arc<Groups> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Groups>>>> = OnceLock::new();
    kept(&CACHE, data, || {
        let told = std::fs::read_to_string(game_file(data, OWN_ORDER)).unwrap_or_default();
        let mut own: Groups = told
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let (species, groups) = line.split_once('\t')?;
                let groups =
                    groups.split(',').filter_map(|pair| pair.split_once('=')).filter_map(|(at, name)| Some((at.parse().ok()?, name.to_string()))).collect();
                Some((species.parse().ok()?, groups))
            })
            .collect();
        own.extend(added_groups(data).iter().map(|(species, groups)| (*species, groups.clone())));
        own
    })
}

/// What shows a pose when a sprite has no group of that name, in turn
/// (a name of the first thirteen groups, which every sprite has, ends it).
pub(super) fn instead(name: &str) -> &'static [&'static str] {
    match name {
        "EventSleep" => &["Sleep"],
        "Wake" => &["StandingUp", "Hop"],
        "Tumble" | "TumbleBack" | "Trip" => &["Tumble", "TumbleBack", "Trip", "Hurt"],
        "Pose" | "Float" | "LeapForth" => &["Hop"],
        "Pull" => &["Charge"],
        "Pain" | "Cringe" | "LostBalance" => &["Pain", "Hurt"],
        "Laying" => &["Fainted", "Sleep"],
        // (Their own "Faint" may rise again, as Dusknoir's 25: their lying one, held, reads better.)
        "Sink" | "Faint" | "HitGround" => &["Fainted", "Laying", "Sleep"],
        "Sit" => &["Laying", "Sleep"],
        "Nod" | "Rotate" => &["Hop"],
        "LookUp" | "DeepBreath" => &["Charge"],
        "Eat" | "Head" | "Double" | "Swing" => &["Attack"],
        // (The community's own, past Sky's groups.)
        "Bow" => &["Nod", "Hop"],
        "Yawn" | "Swell" | "Sing" | "Sound" => &["DeepBreath", "Charge"],
        "Dance" | "Twirl" | "Appeal" | "Wave" | "RaiseArms" | "Shake" | "Wiggle" | "RearUp" | "FlapAround" | "Hover" => &["Pose", "Hop"],
        "Withdraw" | "Injured" | "Cry" => &["Cringe", "Pain", "Hurt"],
        "StandingUp" => &["Wake", "Hop"],
        "SpAttack" | "Emit" | "Shock" | "Gas" | "Roar" | "Rumble" => &["Charge"],
        "Stomp" | "DigOut" => &["HitGround", "Hop"],
        "DigIn" => &["Sink", "Hurt"],
        "Fainted" => &["Laying", "Sleep"],
        "CarefulWalk" => &["Walk"],
        "Punch" | "Jab" | "Uppercut" | "TailWhip" | "Lick" | "Kick" | "Strike" | "QuickStrike" | "MultiStrike" | "Slam" | "Bite" | "Scratch"
        | "MultiScratch" | "Slice" | "Chop" | "Slap" | "Ricochet" => &["Attack"],
        _ => &["Idle"],
    }
}

/// The first thirteen groups, the same in every sprite.
fn common_group(name: &str) -> Option<u16> {
    Some(match name {
        "Walk" => 0,
        "Attack" => 1,
        "Sleep" => 5,
        "Hurt" => 6,
        "Idle" => 7,
        "Hop" => 10,
        "Charge" => 11,
        _ => return None,
    })
}

/// The name of an engine's group (`Nod`), as the sprites' sheets name it.
pub fn group_name(group: u16) -> &'static str {
    assets::tables::ANIMATION_GROUPS.iter().find(|(_, at)| *at == usize::from(group)).map_or("", |(name, _)| *name)
}

/// The group of that species' own sprite named so (`Roar`), if it has one
/// Sky's table reaches.
pub fn own_named(data: &str, species: u16, name: &str) -> Option<u16> {
    own_groups(data).get(&species)?.iter().find(|(at, own)| own == name && *at <= 34).map(|(at, _)| *at)
}

/// The group of that species' own sprite that shows what the engine's
/// group shows (the starters' order): itself for most of the story's
/// heroes, another one, or a close one, for the sprites ordered their own
/// way (Dusknoir's faint is its 25). `None`: it would only stand.
pub fn own_group(data: &str, species: u16, group: u16) -> Option<u16> {
    let table = own_groups(data);
    // (One of the community's own, past Sky's groups: the species' own group
    // of that name where it has one, else the nearest Sky's sprites share.)
    if usize::from(group) >= assets::tables::NAMED_GROUPS {
        let name = group_name(group);
        let own = table.get(&species).filter(|_| species != 0);
        let named = |name: &str| own.and_then(|own| own.iter().find(|(at, own)| *own == name && *at <= 34).map(|(at, _)| *at));
        return named(name).or_else(|| instead(name).iter().find_map(|name| common_group(name).or_else(|| named(name)))).filter(|at| *at != 7);
    }
    let Some(own) = table.get(&species).filter(|_| species != 0) else { return Some(group) };
    let Some(name) = group.checked_sub(13).and_then(|at| SHARED.split(' ').nth(usize::from(at))) else { return Some(group) };
    // (Sky's table reaches groups 0 to 34 only.)
    let named = |name: &str| own.iter().find(|(at, own)| *own == name && *at <= 34).map(|(at, _)| *at);
    if let Some(at) = named(name).filter(|_| name != "Faint") {
        return Some(at);
    }
    instead(name).iter().find_map(|name| common_group(name).or_else(|| named(name))).filter(|at| *at != 7)
}

/// The sprite's animation groups (the engine's `AnimId`) as Sky's
/// `SetAnimation` numbers (its table `SETANIMATION_TABLE`): played once
/// and kept, or looped.
pub fn animation(group: u16, looped: bool) -> Option<u16> {
    let once = match group {
        0 => Some(83),
        1 => Some(13),
        2 => Some(12),
        3 => Some(18),
        4 => Some(19),
        6 => Some(11),
        10 => Some(17),
        13..=32 => Some(22 + group - 13),
        33 => Some(81),
        34 => Some(82),
        _ => None,
    };
    let looping = match group {
        0 => Some(5),
        3 => Some(20),
        4 => Some(21),
        5 => Some(6),
        6 => Some(14),
        7 => Some(4),
        11 => Some(9),
        13..=32 => Some(42 + group - 13),
        _ => None,
    };
    if looped { looping.or(once) } else { once.or(looping) }
}

/// A pose as the DS plays it on that species' own sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoseOnDs {
    /// Sky's `SetAnimation` number.
    pub set: u16,
    /// The pose shown instead, when the species' sprite has none for the
    /// one asked (`Nod` asked, `Hop` shown): told, never left out.
    pub instead: Option<&'static str>,
}

/// The pose the DS plays for the engine's group on that species' sprite
/// (looped or once): its own (`own_group`), else, when its sprite shows
/// none, the nearest one every sprite has, in turn, a hop at last (as the
/// film words' `nearest`). `None` only for the standing pose itself.
pub fn pose_on_ds(data: &str, species: u16, group: u16, looped: bool) -> Option<PoseOnDs> {
    if let Some(set) = own_group(data, species, group).and_then(|own| animation(own, looped)) {
        return Some(PoseOnDs { set, instead: None });
    }
    if group == 7 {
        return None;
    }
    let name = group_name(group);
    let own = own_groups(data);
    let own = own.get(&species).filter(|_| species != 0);
    let named = |name: &str| own.and_then(|own| own.iter().find(|(at, one)| one == name && *at <= 34).map(|(at, _)| *at));
    let tried = instead(name).iter().chain(instead(instead(name).first().copied().unwrap_or(""))).chain(&["Hop"]);
    for nearest in tried.filter(|nearest| **nearest != "Idle" && **nearest != name) {
        let group = common_group(nearest).or_else(|| named(nearest)).or_else(|| {
            // (A group of the starters' order: where the species has no table of its own.)
            let at = SHARED.split(' ').position(|one| one == *nearest)?;
            own.is_none().then_some(13 + at as u16)
        });
        if let Some(set) = group.filter(|group| *group != 7).and_then(|group| animation(group, looped)) {
            return Some(PoseOnDs { set, instead: Some(nearest) });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animations_go_through_skys_table() {
        // Sleep looped (42, the same entry as Sky's 62) and waking once (23, as its 63).
        assert_eq!(animation(13, true), Some(42));
        assert_eq!(animation(14, false), Some(23));
        // A group with no looped entry plays once.
        assert_eq!(animation(33, true), Some(81));
        assert_eq!(animation(12, false), None);
    }

    /// One of the community's own animations, which Sky's sprites lack: the nearest they share.
    #[test]
    fn the_community_s_animations_play_the_nearest_sky_has() {
        let group = |name: &str| assets::tables::ANIMATION_GROUPS.iter().find(|(one, _)| *one == name).map(|(_, at)| *at as u16).unwrap();
        assert_eq!(own_group("", 0, group("Bow")), Some(10));
        assert_eq!(own_group("", 0, group("Punch")), Some(1));
        assert_eq!(own_group("", 0, group("Swell")), Some(11));
    }

    /// A pose no Sky sprite plays (Rotate, 12) or one a species lacks: the nearest every sprite has, told.
    #[test]
    fn a_pose_the_sprite_lacks_plays_the_nearest_every_sprite_has() {
        assert_eq!(pose_on_ds("", 25, 12, false), Some(PoseOnDs { set: 17, instead: Some("Hop") }));
        assert_eq!(pose_on_ds("", 25, 9, false), Some(PoseOnDs { set: 13, instead: Some("Attack") }));
        // (Its own: no change told.)
        assert_eq!(pose_on_ds("", 25, 22, false), Some(PoseOnDs { set: 31, instead: None }));
        // Standing is standing.
        assert_eq!(pose_on_ds("", 25, 7, false), Some(PoseOnDs { set: 4, instead: None }));
    }

    #[test]
    fn a_form_is_drawn_as_its_base_and_a_later_pokemon_by_a_stand_in() {
        assert_eq!(drawn_on_ds("", 172), Drawn::Own);
        // (No variants known: a stand-in, never another Pokémon by chance.)
        assert_eq!(drawn_on_ds("", 5257), Drawn::StandIn);
        assert_eq!(drawn_on_ds("", 2495).species(2495), STAND_IN);
        assert_eq!(Drawn::Base(172).species(5257), 172);
        assert_eq!(Drawn::Added(228).species(2810), 2810);
    }
}
