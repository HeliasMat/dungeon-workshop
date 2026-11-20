//! A move as the rules know it, who it reaches, and whether it lands.

use super::*;

/// Move id of the regular (no-PP) attack.
pub const REGULAR_ATTACK_ID: u16 = 355;

impl MoveSlot {
    /// The regular attack when no move data is at hand: a weak, typeless
    /// physical hit that never misses.
    pub fn fallback_regular_attack() -> Self {
        Self {
            id: REGULAR_ATTACK_ID,
            power: 5,
            accuracy: SURE_HIT,
            accuracy2: SURE_HIT,
            strikes: 1,
            pp: 0,
            max_pp: 0,
            type_id: 0,
            category: 0,
            sealed: false,
            crit_chance: 0,
            range: range::FRONT,
            target: 0,
            ai_weight: 0,
            ai_target_range: 0,
            ai_chance: 0,
            linked: false,
        }
    }
}

/// An accuracy above 100 never misses; the move data writes 125.
const SURE_HIT: i32 = 125;

/// Move ranges, as the move data writes them (the high half of a move's
/// target byte).
pub mod range {
    /// The tile in front, not around a wall's corner.
    pub const FRONT: u8 = 0;
    /// The tile in front and the two beside it.
    pub(crate) const FRONT_AND_SIDES: u8 = 1;
    /// The eight tiles around.
    pub(crate) const NEARBY: u8 = 2;
    /// Whoever the user can see: its room, or close by in a hallway.
    pub const ROOM: u8 = 3;
    /// The first one along a line two tiles long, or ten.
    pub(crate) const FRONT_2: u8 = 4;
    pub(crate) const FRONT_10: u8 = 5;
    /// The whole floor.
    pub const FLOOR: u8 = 6;
    /// The tile in front, even around a wall's corner.
    pub(crate) const FRONT_WITH_CORNER_CUTTING: u8 = 8;
    pub(crate) const FRONT_2_WITH_CORNER_CUTTING: u8 = 9;
}

/// Move targets (the low half of the same byte) that strike enemies.
pub(super) const TARGET_ENEMIES: u8 = 0;

pub(super) const TARGET_ENEMIES_AFTER_CHARGING: u8 = 4;

/// Everyone in reach, or everyone but the user: damaging moves, aimed at
/// enemies too.
pub(super) const TARGET_ALL: u8 = 2;

pub(super) const TARGET_ALL_BUT_USER: u8 = 5;

/// The user's side, with the user or without.
pub const TARGET_PARTY: u8 = 1;

pub const TARGET_TEAMMATES: u8 = 6;

/// A move known by a monster, with its remaining PP.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct MoveSlot {
    /// Number of the move in the move data.
    pub id: u16,
    pub power: i32,
    /// Both must pass for the move to hit.
    pub accuracy: i32,
    pub accuracy2: i32,
    /// How many times the move hits; 0 for 2 to 5 at random.
    pub strikes: i32,
    pub pp: i32,
    pub max_pp: i32,
    pub type_id: u8,
    /// 0 physical, 1 special, 2 status.
    pub category: u8,
    /// Sealed moves cannot be used until the floor is left.
    pub sealed: bool,
    /// Base critical hit chance, in percent.
    pub crit_chance: i32,
    /// See [`range`].
    pub range: u8,
    pub target: u8,
    /// What the AI goes by (see `ai`): the move's weight in its random
    /// pick, its target, range and condition, and the chance of the
    /// "random" condition.
    pub ai_weight: i32,
    pub ai_target_range: u16,
    pub ai_chance: i32,
    /// Linked to the move before it: the two go off one after the other in
    /// one turn.
    #[serde(default)]
    pub linked: bool,
}

impl MoveSlot {
    /// Damaging moves aimed at enemies, and the status moves that have an
    /// effect so far.
    pub fn usable(&self) -> bool {
        let damaging = (self.power > 0 || is_fixed_damage(self.id))
            && matches!(self.target, TARGET_ENEMIES | TARGET_ENEMIES_AFTER_CHARGING | TARGET_ALL | TARGET_ALL_BUT_USER);
        // (Metronome, Copycat, Assist, Nature Power and Me First become
        // another move as they are used; Bide and Revenge strike back.)
        const WILDCARDS: [u16; 8] = [330, 525, 229, 119, 61, 340, 227, 465];
        self.pp > 0 && !self.sealed && (damaging || self.effect().is_some() || WILDCARDS.contains(&self.id))
    }

    /// Aimed at everyone (or everyone but its user) around it, not along a
    /// line: its user's allies too are struck where it reaches. Line moves
    /// spare the allies in the way.
    pub(crate) fn hits_allies(&self) -> bool {
        let line = matches!(self.range, range::FRONT_2 | range::FRONT_10 | range::FRONT_2_WITH_CORNER_CUTTING);
        matches!(self.target, TARGET_ALL | TARGET_ALL_BUT_USER) && !line
    }

    /// [`Self::effect`], with weather-dependent amounts settled.
    pub fn effect_in(&self, constants: &Constants, weather: u8, user: &Stats) -> Option<MoveEffect> {
        const GHOST: u8 = 14;
        match self.effect()? {
            // Curse depends on whether its user is a Ghost type.
            MoveEffect::Curse if user.types.0 == GHOST || user.types.1 == GHOST => Some(MoveEffect::Cursed),
            MoveEffect::Curse => Some(MoveEffect::CurseSelf),
            MoveEffect::HealBy(index) => Some(MoveEffect::HealFlat(constants.heal_amount(index, weather))),
            effect => Some(effect),
        }
    }

    /// The effect of a status move; `None` for damaging and unhandled moves.
    pub fn effect(&self) -> Option<MoveEffect> {
        if self.power > 0 || is_fixed_damage(self.id) { None } else { move_effect(self.id) }
    }
}

/// Positions, among `enemies`, hit by a move of the given range used from
/// `from` while facing `facing`.
pub fn targets_in_range(floor: &Floor, from: Pos, facing: (i32, i32), range: u8, enemies: &[Pos]) -> Vec<Pos> {
    let step = |(dx, dy): (i32, i32), distance: i32| Pos::new(from.x + dx * distance, from.y + dy * distance);
    let tiles: Vec<Pos> = match range {
        range::FLOOR => return enemies.to_vec(),
        range::ROOM => return enemies.iter().copied().filter(|&p| crate::dungeon::ai::can_see(floor, from, p)).collect(),
        range::FRONT_2 | range::FRONT_2_WITH_CORNER_CUTTING => return first_along(floor, from, facing, 2, enemies).into_iter().collect(),
        range::FRONT_10 => return first_along(floor, from, facing, 10, enemies).into_iter().collect(),
        range::NEARBY => AROUND.iter().map(|&direction| step(direction, 1)).collect(),
        range::FRONT_AND_SIDES => beside(facing).into_iter().map(|direction| step(direction, 1)).collect(),
        range::FRONT_WITH_CORNER_CUTTING => vec![step(facing, 1)],
        // FRONT and anything unknown: one tile ahead, not around a corner.
        _ if floor.can_step(from, facing.0, facing.1) => vec![step(facing, 1)],
        _ => Vec::new(),
    };
    tiles.into_iter().filter(|tile| enemies.contains(tile)).collect()
}

/// The facing direction and the two next to it, around the circle.
fn beside(facing: (i32, i32)) -> [(i32, i32); 3] {
    let front = AROUND.iter().position(|&direction| direction == facing).unwrap_or(0);
    [AROUND[(front + 7) % 8], AROUND[front], AROUND[(front + 1) % 8]]
}

/// The first enemy at most `length` tiles away straight ahead; a wall
/// stops the line.
fn first_along(floor: &Floor, from: Pos, facing: (i32, i32), length: i32, enemies: &[Pos]) -> Option<Pos> {
    (1..=length)
        .map(|distance| Pos::new(from.x + facing.0 * distance, from.y + facing.1 * distance))
        .take_while(|&tile| floor.tile(tile) != Tile::Wall)
        .find(|tile| enemies.contains(tile))
}

/// How much a stage (0 to 20, 10 neutral) of accuracy or evasion weighs: a
/// quarter more for each stage up, as much less for each stage down.
fn aim_factor(stage: i32) -> f64 {
    let from_neutral = (stage.clamp(0, STAGE_COUNT as i32 - 1) - 10) as f64;
    if from_neutral >= 0.0 { (4.0 + from_neutral) / 4.0 } else { 4.0 / (4.0 - from_neutral) }
}

/// One accuracy check: an accuracy above 100 never misses; otherwise the
/// chance is the accuracy, raised by the attacker's accuracy stage and
/// lowered by the target's evasion stage. The roll is always drawn, so a
/// sure hit does not change what the dice say next.
pub(crate) fn move_hit_check(rng: &mut DungeonRng, accuracy: i32, (accuracy_stage, evasion_stage): (i32, i32)) -> bool {
    let roll = rng.rand_int(100);
    if accuracy > 100 {
        return true;
    }
    let chance = accuracy as f64 * aim_factor(accuracy_stage) / aim_factor(evasion_stage);
    (roll as f64) < chance
}

/// Whether a move lands on a target: both of its accuracy values must
/// pass. Thunder never misses in the rain and aims worse in the sun;
/// Blizzard never misses in hail. (Monsters of either gender aim alike.)
pub fn move_hits(rng: &mut DungeonRng, _constants: &Constants, slot: &MoveSlot, stages: (i32, i32), weather: u8, _females: (bool, bool)) -> bool {
    const THUNDER: u16 = 64;
    const BLIZZARD: u16 = 270;
    let sure = matches!((slot.id, weather), (THUNDER, weather::RAIN) | (BLIZZARD, weather::HAIL));
    let stages = match (slot.id, weather) {
        (THUNDER, weather::SUNNY) => (stages.0 - 2, stages.1),
        _ => stages,
    };
    let first = move_hit_check(rng, slot.accuracy, stages);
    let second = move_hit_check(rng, slot.accuracy2, stages);
    sure || (first && second)
}
