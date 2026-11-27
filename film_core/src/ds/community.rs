//! A species of Sky's drawn on DS from the community's sheet (SpriteCollab,
//! `sources/spritecollab/sprite/<dex>/`) when the film poses it with an
//! animation its Sky sprite can't play and that sheet has (Pikachu's
//! Double: Sky's table plays no group 8, 9 or 12, nor the community's own
//! past 34). Nothing to write in the film: the export sees it, puts the
//! sheet in the ROM as a monster of its own (`tools/ds/add_monsters.py`),
//! and casts the film's actors of that species as it.
//!
//! Only the animations the film needs are kept (walking, standing, the
//! seven every sprite has, those it poses), each in a group Sky's
//! `SetAnimation` plays: its own where it can, else a free one of 13 to 32.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use engine::{Beat, BodyBeat, Play, Scene, Who};

use super::sprites::{PerData, PoseOnDs, SKY_SPECIES, animation, group_name, instead, kept, pose_on_ds};

/// The number the export gives such a species' sprite (its actors' species
/// until `patch_rom.py` gives them the ROM's monster): this + its Pokédex
/// number (none of the data's species: 2000 + a Pokédex number, variants
/// from 5000).
pub const COMMUNITY: u16 = 4000;

/// The groups every sprite has (`sprites::common_group`): kept, for the
/// engine walks, stands and the nearest poses fall back on them.
const COMMON: [&str; 7] = ["Walk", "Attack", "Sleep", "Hurt", "Idle", "Hop", "Charge"];

/// A Sky species drawn from the community's sheet in this film.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Community {
    /// Sky's species.
    pub species: u16,
    /// Its Pokédex number (the sheet's folder).
    pub dex: u16,
    /// The export's number for it (`COMMUNITY` + `dex`).
    pub number: u16,
    /// Each animation kept: the group it has in the sprite put in the ROM,
    /// its name in the sheet (`Double`).
    pub groups: Vec<(u16, String)>,
}

impl Community {
    /// The pose the DS plays for the engine's group: the sheet's own of
    /// that name, else the nearest it keeps (as `pose_on_ds`).
    pub fn pose(&self, group: u16, looped: bool) -> Option<PoseOnDs> {
        let slot = |name: &str| self.groups.iter().find(|(_, own)| own == name).map(|(at, _)| *at);
        let name = group_name(group);
        if let Some(set) = slot(name).and_then(|at| animation(at, looped)) {
            return Some(PoseOnDs { set, instead: None });
        }
        if group == 7 {
            return None;
        }
        let tried = instead(name).iter().chain(instead(instead(name).first().copied().unwrap_or(""))).chain(&["Hop"]);
        tried.filter(|nearest| **nearest != "Idle" && **nearest != name).find_map(|nearest| {
            let set = slot(nearest).filter(|at| *at != 7).and_then(|at| animation(at, looped))?;
            Some(PoseOnDs { set, instead: Some(nearest) })
        })
    }
}

/// The clone of SpriteCollab: `DUNGEON_SPRITECOLLAB`, else
/// `<root>/sources/spritecollab` (the root: the data folder's).
pub fn community_clone(data: &str) -> PathBuf {
    std::env::var_os("DUNGEON_SPRITECOLLAB")
        .map_or_else(|| std::path::Path::new(data).parent().unwrap_or(std::path::Path::new("")).join("sources").join("spritecollab"), PathBuf::from)
}

/// The Pokédex number of a species of Sky's (`dex.txt`).
fn dex_of(data: &str, species: u16) -> Option<u16> {
    static CACHE: PerData<BTreeMap<u16, u16>> = OnceLock::new();
    let table = kept(&CACHE, data, || {
        assets::data::text(data, "dex.txt")
            .lines()
            .filter_map(|line| {
                let (species, dex) = line.split_once('\t')?;
                Some((species.trim().parse().ok()?, dex.trim().parse().ok()?))
            })
            .collect()
    });
    table.get(&species).copied().filter(|dex| *dex > 0)
}

/// The animations the community's sheet of that Pokédex number has (its
/// `AnimData.xml`'s names); none without the clone.
fn sheet_names(data: &str, dex: u16) -> Arc<Vec<String>> {
    static CACHE: PerData<Vec<String>> = OnceLock::new();
    kept(&CACHE, &format!("{data}\u{0}{dex}"), || {
        let xml = std::fs::read_to_string(community_clone(data).join("sprite").join(format!("{dex:04}")).join("AnimData.xml")).unwrap_or_default();
        xml.split("<Name>").skip(1).filter_map(|rest| Some(rest.split_once("</Name>")?.0.trim().to_string())).collect()
    })
}

/// Each species of Sky's the scenes pose with an animation its Sky sprite
/// can't play and the community's sheet has: drawn from that sheet, by
/// Sky's species. (A hop is Sky's own jump; the hero and the partner,
/// whose species the player chose, keep theirs.)
pub fn community_sheets<'a>(data: &str, scenes: impl IntoIterator<Item = &'a Scene>) -> BTreeMap<u16, Community> {
    let mut asked: BTreeMap<u16, BTreeSet<(u16, bool)>> = BTreeMap::new();
    for beat in scenes.into_iter().flat_map(|scene| &scene.beats) {
        if let Beat::Body(BodyBeat::Anim { who: Who::Monster(species) | Who::Another { species, .. }, anim, play }) = beat
            && *species > 0
            && *species < SKY_SPECIES
        {
            asked.entry(*species).or_default().insert((anim.0, matches!(play, Play::Loop)));
        }
    }
    let mut chosen = BTreeMap::new();
    for (species, poses) in asked {
        let Some(dex) = dex_of(data, species) else { continue };
        let names = sheet_names(data, dex);
        let has = |name: &str| names.iter().any(|own| own == name);
        if !has("Walk") || !has("Idle") {
            continue;
        }
        let lacking = poses.iter().any(|(group, looped)| {
            *group != 10 && has(group_name(*group)) && pose_on_ds(data, species, *group, *looped).is_some_and(|pose| pose.instead.is_some())
        });
        if !lacking {
            continue;
        }
        let mut wanted: Vec<&str> = COMMON.iter().copied().filter(|name| has(name)).collect();
        for (group, _) in &poses {
            let name = group_name(*group);
            let kept = if has(name) { Some(name) } else { instead(name).iter().copied().find(|nearest| has(nearest)) };
            if let Some(name) = kept.filter(|name| !wanted.contains(name)) {
                wanted.push(name);
            }
        }
        chosen.insert(species, Community { species, dex, number: COMMUNITY + dex, groups: laid_out(&wanted) });
    }
    chosen
}

/// The animations, each in a group Sky's table plays: its own where it
/// can (its number in `ANIMATION_GROUPS`), else a free one of 13 to 32
/// (both played once and looped); past them, left out.
fn laid_out(names: &[&str]) -> Vec<(u16, String)> {
    let own = |name: &str| assets::tables::ANIMATION_GROUPS.iter().find(|(one, _)| *one == name).map(|(_, at)| *at as u16);
    let playable = |at: u16| animation(at, false).is_some() || animation(at, true).is_some();
    let mut groups: Vec<(u16, String)> = Vec::new();
    let mut moved = Vec::new();
    for name in names {
        match own(name).filter(|at| playable(*at) && !groups.iter().any(|(taken, _)| taken == at)) {
            Some(at) => groups.push((at, name.to_string())),
            None => moved.push(*name),
        }
    }
    let mut free = (13..=32).filter(|at| !groups.iter().any(|(taken, _)| taken == at)).collect::<Vec<u16>>().into_iter();
    for name in moved {
        let Some(at) = free.next() else { break };
        groups.push((at, name.to_string()));
    }
    groups.sort();
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Double (9) and the community's own (QuickStrike, 64) go where Sky's table plays them.
    #[test]
    fn what_sky_cant_play_moves_to_a_free_group() {
        let groups = laid_out(&["Walk", "Idle", "Double", "Shoot", "LostBalance", "QuickStrike"]);
        assert_eq!(
            groups,
            vec![(0, "Walk".into()), (3, "Shoot".into()), (7, "Idle".into()), (13, "Double".into()), (14, "QuickStrike".into()), (31, "LostBalance".into())]
        );
        let pikachu = Community { species: 25, dex: 25, number: 4025, groups };
        assert_eq!(pikachu.pose(9, true), Some(PoseOnDs { set: 42, instead: None }));
        // (A nod, with neither it nor a hop kept: it stands.)
        assert_eq!(pikachu.pose(22, false), None);
    }
}
