//! IQ: a monster's IQ points (raised by eating Gummis) open the skills of
//! its species' IQ group, each once its IQ reaches what the skill needs.
//! The tables come from the player's own game data; how they are used is
//! this project's.

use super::combat::Stats;

pub(crate) const TYPE_ADVANTAGE_MASTER: u8 = 1;
pub(crate) const SURE_HIT_ATTACKER: u8 = 4;
pub(crate) const QUICK_DODGER: u8 = 5;
pub(crate) const NONSLEEPER: u8 = 17;
pub const TRAP_SEER: u8 = 19;
pub(crate) const DEDICATED_TRAVELER: u8 = 21;
pub(crate) const ITEM_MASTER: u8 = 22;
pub(crate) const EXCLUSIVE_MOVE_USER: u8 = 23;
pub(crate) const STAIR_SENSOR: u8 = 26;
pub(crate) const EXP_ELITE: u8 = 28;
pub(crate) const SHARPSHOOTER: u8 = 29;
pub(crate) const AGGRESSOR: u8 = 34;
pub(crate) const DEFENDER: u8 = 35;
pub(crate) const CONCENTRATOR: u8 = 48;
pub(crate) const ENERGY_SAVER: u8 = 16;
pub(crate) const ERRATIC_PLAYER: u8 = 59;
pub(crate) const INTIMIDATOR: u8 = 36;
pub(crate) const POWER_PITCHER: u8 = 47;
pub(crate) const WISE_HEALER: u8 = 52;
pub(crate) const PIERCE_HURLER: u8 = 66;
pub(crate) const COUNTER_BASHER: u8 = 49;
pub(crate) const QUICK_HEALER: u8 = 53;
pub(crate) const CLUTCH_PERFORMER: u8 = 63;
pub(crate) const CRITICAL_DODGER: u8 = 64;
pub(crate) const MAP_SURVEYOR: u8 = 65;
pub(crate) const ITEM_CATCHER: u8 = 2;
pub(crate) const COURSE_CHECKER: u8 = 3;
pub(crate) const NONTRAITOR: u8 = 7;
pub(crate) const STATUS_CHECKER: u8 = 8;
pub(crate) const WEAK_TYPE_PICKER: u8 = 11;
pub(crate) const LAVA_EVADER: u8 = 20;
pub(crate) const ESCAPIST: u8 = 24;
pub(crate) const BRICK_TOUGH: u8 = 56;
pub(crate) const MULTITALENT: u8 = 57;
pub(crate) const EXP_GO_GETTER: u8 = 9;
pub(crate) const EFFICIENCY_EXPERT: u8 = 10;
pub(crate) const ALL_TERRAIN_HIKER: u8 = 12;
pub(crate) const ABSOLUTE_MOVER: u8 = 13;
pub(crate) const TRAP_AVOIDER: u8 = 14;
pub(crate) const HOUSE_AVOIDER: u8 = 15;
pub(crate) const SELF_CURER: u8 = 18;
pub(crate) const COIN_WATCHER: u8 = 27;
pub(crate) const FAST_FRIEND: u8 = 30;
pub(crate) const PP_SAVER: u8 = 31;
pub(crate) const BODYGUARD: u8 = 32;
pub(crate) const TRAP_BUSTER: u8 = 38;
pub(crate) const COUNTER_HITTER: u8 = 39;
pub(crate) const EXTRA_STRIKER: u8 = 40;
pub(crate) const HIT_AND_RUNNER: u8 = 41;
pub(crate) const QUICK_STRIKER: u8 = 43;
pub(crate) const NO_CHARGER: u8 = 44;
pub(crate) const WARY_FIGHTER: u8 = 45;
pub(crate) const GAP_PROBER: u8 = 46;
pub(crate) const CHEERLEADER: u8 = 50;
pub(crate) const NATURE_GIFTER: u8 = 51;
pub(crate) const ACUTE_SNIFFER: u8 = 54;
pub(crate) const DEEP_BREATHER: u8 = 55;
pub(crate) const TIME_TRIPPER: u8 = 58;
pub(crate) const PRACTICE_SWINGER: u8 = 60;
pub(crate) const HAGGLER: u8 = 62;

/// What Brick-Tough adds to the max HP and Multitalent to each move's
/// PP while they are on (the game's, read from its arm9).
pub(crate) const BRICK_TOUGH_HP: i32 = 10;
pub(crate) const MULTITALENT_PP: i32 = 5;

/// Below these IQs a wild monster goes without Exclusive Move-User and
/// Item Master (the game's, read from its arm9).
const MIN_WILD_IQ_EXCLUSIVE_MOVE_USER: i32 = 331;
const MIN_WILD_IQ_ITEM_MASTER: i32 = 150;
/// The skills that cannot be switched off, as in the game.
pub(crate) const ALWAYS_ON: [u8; 3] = [ESCAPIST, BRICK_TOUGH, MULTITALENT];

/// How many skill numbers there are (1 to 68; 0 is none).
pub const SKILLS: usize = 69;
/// The first and last Gummis (White Gummi, Wonder Gummi) of the items.
pub const FIRST_GUMMI: u16 = 119;
pub const WONDER_GUMMI: u16 = 136;
/// A team member's IQ at its start, and the most it can have.
pub const STARTING_IQ: i32 = 1;
pub(crate) const MAX_IQ: i32 = 999;
/// Types there are, as the Gummi tables count them.
pub const TYPES: usize = 18;

/// The IQ tables, as read from the game data.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IqTables {
    /// The IQ each skill needs.
    pub required: Vec<i32>,
    /// By IQ group, its skills in order (up to 25).
    pub groups: Vec<Vec<u8>>,
    /// By skill, the group of skills of which only one can be on.
    pub restrictions: Vec<i16>,
    /// By monster type then Gummi type: IQ gained, belly restored.
    pub gummi_iq: Vec<Vec<i16>>,
    pub gummi_belly: Vec<Vec<i16>>,
    /// What a Wonder Gummi gives.
    pub wonder_iq: i32,
}

/// Ends a group's list of skills.
const END_OF_GROUP: u8 = 0xFF;

/// The skills only a team makes use of, and those the game takes from
/// wild monsters (Dedicated Traveler, Lava Evader, Weak-Type Picker).
const TEAM_ONLY: [u8; 7] = [TRAP_SEER, STAIR_SENSOR, MAP_SURVEYOR, EXP_ELITE, DEDICATED_TRAVELER, LAVA_EVADER, WEAK_TYPE_PICKER];

impl IqTables {
    /// The skills a monster of `group` has at `iq`, as bits by skill
    /// number. Of skills that exclude one another, the one needing the most
    /// IQ wins: a newer skill takes the place of an older one.
    pub fn skills(&self, group: u8, iq: i32) -> u128 {
        let needs = |skill: u8| self.required.get(skill as usize).copied();
        let mut open: Vec<u8> = self
            .groups
            .get(group as usize)
            .into_iter()
            .flatten()
            .copied()
            .take_while(|&skill| skill != END_OF_GROUP)
            .filter(|&skill| (skill as usize) < SKILLS && needs(skill).is_some_and(|needed| needed <= iq))
            .collect();
        open.sort_by_key(|&skill| std::cmp::Reverse(needs(skill)));
        let mut skills = 0u128;
        let mut kinds: Vec<i16> = Vec::new();
        for skill in open {
            let kind = self.restrictions.get(skill as usize).copied().unwrap_or(0);
            if kind != 0 && kinds.contains(&kind) {
                continue;
            }
            kinds.push(kind);
            skills |= 1 << skill;
        }
        skills
    }

    /// Those of an enemy at its floor's IQ, less the team's own skills;
    /// Exclusive Move-User and Item Master only past an IQ (the game's
    /// `UpdateIqSkills`).
    pub(crate) fn enemy_skills(&self, group: u8, iq: i32) -> u128 {
        let mut kept_off = TEAM_ONLY.iter().fold(0u128, |bits, &skill| bits | 1 << skill);
        if iq < MIN_WILD_IQ_EXCLUSIVE_MOVE_USER {
            kept_off |= 1 << EXCLUSIVE_MOVE_USER;
        }
        if iq < MIN_WILD_IQ_ITEM_MASTER {
            kept_off |= 1 << ITEM_MASTER;
        }
        self.skills(group, iq) & !kept_off
    }

    /// What eating a Gummi gives a monster of these types: IQ points and
    /// belly, as the table says for the better suited of its two types.
    pub fn gummi(&self, types: (u8, u8), item: u16) -> Option<(i32, i32)> {
        match item {
            WONDER_GUMMI => Some((self.wonder_iq, 0)),
            FIRST_GUMMI..WONDER_GUMMI => {
                let column = (item - FIRST_GUMMI + 1) as usize;
                let cell = |table: &[Vec<i16>], kind: u8| table.get(kind as usize).and_then(|row| row.get(column)).copied().unwrap_or(0) as i32;
                let best = if cell(&self.gummi_iq, types.1) > cell(&self.gummi_iq, types.0) { types.1 } else { types.0 };
                Some((cell(&self.gummi_iq, best), cell(&self.gummi_belly, best)))
            }
            _ => None,
        }
    }
}

/// Whether a monster has a skill on.
pub fn has(stats: &Stats, skill: u8) -> bool {
    (skill as usize) < SKILLS && stats.iq_skills & (1 << skill) != 0
}

/// Brick-Tough's HP and Multitalent's PP, given or taken back as the
/// skills come on or off: `boosted` tells whether the HP are given.
pub(crate) fn keep_boosts(stats: &mut Stats, moves: &mut [super::combat::MoveSlot], boosted: &mut bool) {
    let tough = has(stats, BRICK_TOUGH);
    if tough != *boosted {
        *boosted = tough;
        let by = if tough { BRICK_TOUGH_HP } else { -BRICK_TOUGH_HP };
        stats.max_hp = (stats.max_hp + by).max(1);
        stats.hp = if tough { stats.hp + by } else { stats.hp.min(stats.max_hp) };
    }
    let talented = has(stats, MULTITALENT);
    for slot in moves.iter_mut().filter(|slot| slot.talented != talented) {
        slot.talented = talented;
        let by = if talented { MULTITALENT_PP } else { -MULTITALENT_PP };
        slot.max_pp = (slot.max_pp + by).max(1);
        slot.pp = if talented { slot.pp + by } else { slot.pp.min(slot.max_pp) };
    }
}

/// Self-Curer: a status problem lasts half as long (`extra` the turn it
/// starts on, for those that count it).
pub(crate) fn cure_sooner(stats: &Stats, turns: &mut i32, extra: i32) {
    const LASTING: i32 = 100;
    if has(stats, SELF_CURER) && *turns < LASTING && *turns > extra {
        *turns = (*turns - extra + 1) / 2 + extra;
    }
}

/// The skills a monster has that it did not have before, by number.
pub fn learnt(before: u128, now: u128) -> Vec<u8> {
    (0..SKILLS as u8).filter(|&skill| now & (1 << skill) != 0 && before & (1 << skill) == 0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables() -> IqTables {
        let mut required = vec![0; SKILLS];
        (required[AGGRESSOR as usize], required[DEFENDER as usize], required[SHARPSHOOTER as usize]) = (10, 20, 30);
        let mut restrictions = vec![0; SKILLS];
        (restrictions[AGGRESSOR as usize], restrictions[DEFENDER as usize]) = (1, 1);
        let mut gummi_iq = vec![vec![1; TYPES]; TYPES];
        gummi_iq[2][2] = 5;
        IqTables {
            required,
            groups: vec![vec![AGGRESSOR, DEFENDER, SHARPSHOOTER, 0xFF]],
            restrictions,
            gummi_iq: gummi_iq.clone(),
            gummi_belly: gummi_iq,
            wonder_iq: 10,
        }
    }

    #[test]
    fn skills_open_with_iq_and_the_newer_wins() {
        let tables = tables();
        assert_eq!(tables.skills(0, 5), 0);
        assert_eq!(tables.skills(0, 10), 1 << AGGRESSOR);
        // Defender excludes Aggressor and needs more IQ: it takes its place.
        assert_eq!(tables.skills(0, 20), 1 << DEFENDER);
        assert_eq!(tables.skills(0, 30), 1 << DEFENDER | 1 << SHARPSHOOTER);
        assert_eq!(learnt(1 << DEFENDER, 1 << DEFENDER | 1 << SHARPSHOOTER), vec![SHARPSHOOTER]);
        // Nothing past the group's end, nor for a group that is not there.
        assert_eq!(tables.skills(1, 999), 0);
    }

    #[test]
    fn enemies_go_without_the_team_skills() {
        let mut tables = tables();
        tables.groups[0].insert(0, MAP_SURVEYOR);
        assert_eq!(tables.skills(0, 10), 1 << MAP_SURVEYOR | 1 << AGGRESSOR);
        assert_eq!(tables.enemy_skills(0, 10), 1 << AGGRESSOR);
    }

    #[test]
    fn wild_monsters_need_iq_for_two_skills() {
        let mut tables = tables();
        tables.required.resize(SKILLS, 0);
        (tables.required[ITEM_MASTER as usize], tables.required[EXCLUSIVE_MOVE_USER as usize]) = (-1, -1);
        tables.groups[0] = vec![ITEM_MASTER, EXCLUSIVE_MOVE_USER, LAVA_EVADER, 0xFF];
        assert_eq!(tables.skills(0, 1), 1 << ITEM_MASTER | 1 << EXCLUSIVE_MOVE_USER | 1 << LAVA_EVADER);
        assert_eq!(tables.enemy_skills(0, 149), 0);
        assert_eq!(tables.enemy_skills(0, 150), 1 << ITEM_MASTER);
        assert_eq!(tables.enemy_skills(0, 331), 1 << ITEM_MASTER | 1 << EXCLUSIVE_MOVE_USER);
    }

    #[test]
    fn brick_tough_and_multitalent_give_and_take_back() {
        let mut stats = Stats {
            level: 5,
            max_hp: 30,
            hp: 30,
            attack: 9,
            defense: 9,
            sp_attack: 9,
            sp_defense: 9,
            types: (1, 0),
            abilities: [0, 0],
            held: 0,
            iq: 1,
            iq_group: 0,
            iq_skills: 0,
            female: false,
        };
        let mut moves = [super::super::combat::MoveSlot { pp: 3, max_pp: 10, ..super::super::combat::MoveSlot::fallback_regular_attack() }];
        let (hp, mut tough) = (stats.max_hp, false);
        stats.iq_skills = 1 << BRICK_TOUGH | 1 << MULTITALENT;
        keep_boosts(&mut stats, &mut moves, &mut tough);
        keep_boosts(&mut stats, &mut moves, &mut tough);
        assert_eq!((stats.max_hp, moves[0].max_pp, moves[0].pp), (hp + BRICK_TOUGH_HP, 10 + MULTITALENT_PP, 3 + MULTITALENT_PP));
        stats.iq_skills = 0;
        keep_boosts(&mut stats, &mut moves, &mut tough);
        // (The PP left stay, up to the max.)
        assert_eq!((stats.max_hp, moves[0].max_pp, moves[0].pp), (hp, 10, 8));
        // Self-Curer halves a status problem's turns.
        let mut turns = 9;
        cure_sooner(&stats, &mut turns, 1);
        assert_eq!(turns, 9);
        stats.iq_skills = 1 << SELF_CURER;
        cure_sooner(&stats, &mut turns, 1);
        assert_eq!(turns, 5);
    }

    #[test]
    fn a_gummi_of_ones_type_gives_more() {
        let tables = tables();
        // A Red Gummi (fire) to a fire monster, to a water one.
        assert_eq!(tables.gummi((2, 0), FIRST_GUMMI + 1), Some((5, 5)));
        assert_eq!(tables.gummi((3, 0), FIRST_GUMMI + 1), Some((1, 1)));
        // The better of two types counts.
        assert_eq!(tables.gummi((3, 2), FIRST_GUMMI + 1), Some((5, 5)));
        assert_eq!(tables.gummi((3, 0), WONDER_GUMMI), Some((10, 0)));
        assert_eq!(tables.gummi((3, 0), 70), None);
    }
}
