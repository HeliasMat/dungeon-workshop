//! How a computer-controlled monster picks its attack.
//!
//! The monster looks at each of its moves and at its regular attack, keeps
//! those that would reach someone it wants to hit (or help), and draws one
//! of them by its AI weight. With nothing in reach it walks instead. Its IQ
//! skills change what it looks at, as in the game's `ChooseAiMove`.

use super::combat::{AROUND, MoveSlot, TYPE_COUNT, matchup, range};
use super::floor::{Floor, Mobility, Pos, Tile};
use super::iq;
use super::rng::DungeonRng;
use super::status::problem_of;

/// The eight directions: down first, then counterclockwise on screen.
pub const DIRECTIONS: [(i32, i32); 8] = AROUND;

/// A move's range nibble for "on the user itself".
const RANGE_SELF: u8 = 7;
/// How far the long line moves fly.
const LONG_LINE: i32 = 10;
/// Weak-Type Picker: what the regular attack weighs, and the most a move
/// weighs before the one added (the game's).
const PICKER_REGULAR_WEIGHT: i32 = 2;
const PICKER_TOP_WEIGHT: i32 = 3;

/// Who a move is meant for, as the low nibble of `ai_target_range`.
mod aim {
    pub(crate) const FOES: u8 = 0;
    pub(crate) const SIDE: u8 = 1;
    pub(crate) const EVERYONE: u8 = 2;
    pub(crate) const SELF: u8 = 3;
    pub(crate) const FOES_CHARGED: u8 = 4;
    pub(crate) const OTHERS: u8 = 5;
    pub(crate) const ALLIES: u8 = 6;
}

/// When a move is worth using, as the third nibble of `ai_target_range`.
mod when {
    /// Only now and then: `ai_chance` in 100.
    pub(crate) const SOMETIMES: u8 = 1;
    /// Only once its user is hurt.
    pub(crate) const HURT: u8 = 2;
}

/// The monster choosing its attack.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AiUser {
    pub pos: Pos,
    /// Index in [`DIRECTIONS`] of where it faces.
    pub facing: usize,
    pub mobility: Mobility,
    pub full_hp: bool,
    /// Its IQ skills, as bits by skill (see [`iq`]).
    pub skills: u128,
    /// Its types, for the moves of its own type (Weak-Type Picker).
    pub types: (u8, u8),
}

impl AiUser {
    fn has(&self, skill: u8) -> bool {
        self.skills & (1 << skill) != 0
    }
}

/// Another monster on the floor, as the user sees it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Candidate {
    pub pos: Pos,
    /// An enemy of the user, rather than one of its allies.
    pub hostile: bool,
    /// Its status problems, as `status::problem` bits (Status Checker).
    pub problems: u16,
    /// How much the user would rather strike it of all its foes (Exp.
    /// Go-Getter: its worth; Efficiency Expert: the less HP the more).
    pub worth: i32,
    /// How a move of each type fares against it (Weak-Type Picker).
    pub matchups: [u8; TYPE_COUNT],
}

impl Candidate {
    /// One with nothing that sets it apart.
    pub(crate) fn new(pos: Pos, hostile: bool) -> Self {
        Self { pos, hostile, problems: 0, worth: 1, matchups: [matchup::NEUTRAL; TYPE_COUNT] }
    }
}

/// What the AI settled on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AiChoice {
    /// The move slot to use; `None` for the regular attack.
    pub slot: Option<usize>,
    /// Index in [`DIRECTIONS`] of where to use it.
    pub direction: usize,
}

/// The tile `distance` steps away from `from` that way.
fn ahead(from: Pos, direction: usize, distance: i32) -> Pos {
    let (dx, dy) = DIRECTIONS[direction];
    Pos::new(from.x + dx * distance, from.y + dy * distance)
}

/// Whether `to` is in sight from `from`. In a room, the room and the ring
/// of tiles around it are; in a hallway, a small square around the viewer.
pub fn can_see(floor: &Floor, from: Pos, to: Pos) -> bool {
    match floor.tile(from).room() {
        Some(room) => touches_room(floor, to, room),
        None => (to.x - from.x).abs().max((to.y - from.y).abs()) <= floor.hallway_sight,
    }
}

/// Whether a tile is in that room or right next to it.
fn touches_room(floor: &Floor, p: Pos, room: u8) -> bool {
    std::iter::once((0, 0)).chain(DIRECTIONS).any(|(dx, dy)| floor.tile(Pos::new(p.x + dx, p.y + dy)).room() == Some(room))
}

/// Whether a blow may go that way: it crosses water and chasms like a
/// hovering monster, but not walls nor their corners.
fn can_strike(floor: &Floor, user: &AiUser, direction: usize) -> bool {
    let mobility = if matches!(user.mobility, Mobility::Normal | Mobility::Secondary) { Mobility::Hovering } else { user.mobility };
    let (dx, dy) = DIRECTIONS[direction];
    floor.can_step_as(user.pos, dx, dy, mobility)
}

/// Whether a move meant for `aim` would be wasted on nobody: `who` is
/// `None` for the user itself.
fn wants(aim: u8, who: Option<&Candidate>) -> bool {
    match (aim, who) {
        (aim::SELF, who) => who.is_none(),
        (aim::SIDE, who) => who.is_none_or(|other| !other.hostile),
        (aim::ALLIES, who) => who.is_some_and(|other| !other.hostile),
        // Moves that strike everyone are aimed at foes all the same.
        (aim::FOES | aim::FOES_CHARGED | aim::EVERYONE | aim::OTHERS, who) => who.is_some_and(|other| other.hostile),
        _ => false,
    }
}

/// Status Checker: the move would give its target a status problem it
/// already has.
fn wasted(user: &AiUser, slot: &MoveSlot, who: &Candidate) -> bool {
    user.has(iq::STATUS_CHECKER) && slot.effect().is_some_and(|effect| problem_of(effect) & who.problems != 0)
}

/// The monster a line from the user reaches, stopped by walls: with Course
/// Checker the first one in the way, without it the first worth aiming at
/// (the move flies past the others).
fn first_in_line<'a>(floor: &Floor, user: &AiUser, direction: usize, reach: i32, aim: u8, others: &'a [Candidate]) -> Option<&'a Candidate> {
    let mut line = (1..=reach)
        .map(|distance| ahead(user.pos, direction, distance))
        .take_while(|&tile| floor.tile(tile) != Tile::Wall)
        .filter_map(|tile| others.iter().find(|other| other.pos == tile));
    if user.has(iq::COURSE_CHECKER) { line.next() } else { line.find(|other| wants(aim, Some(other))) }
}

/// Exp. Go-Getter and Efficiency Expert: the user picks its foe by worth.
fn picky(user: &AiUser) -> bool {
    user.has(iq::EXP_GO_GETTER) || user.has(iq::EFFICIENCY_EXPERT)
}

/// The directions a directional move would reach a wanted target in, with
/// that target.
fn aimed<'a>(floor: &Floor, user: &AiUser, slot: &MoveSlot, others: &'a [Candidate]) -> Vec<(usize, &'a Candidate)> {
    let aim = (slot.ai_target_range & 0xF) as u8;
    let (reach, cuts_corners) = match (slot.ai_target_range >> 4 & 0xF) as u8 {
        range::FRONT => (1, false),
        range::FRONT_AND_SIDES | range::NEARBY | range::FRONT_WITH_CORNER_CUTTING => (1, true),
        range::FRONT_2 => (2, false),
        range::FRONT_2_WITH_CORNER_CUTTING => (2, true),
        range::FRONT_10 => (LONG_LINE, false),
        _ => return Vec::new(),
    };
    (0..8)
        .filter(|&direction| cuts_corners || can_strike(floor, user, direction))
        .filter_map(|direction| {
            let target = first_in_line(floor, user, direction, reach, aim, others)?;
            let seen = can_see(floor, user.pos, target.pos);
            (seen && wants(aim, Some(target)) && !wasted(user, slot, target)).then_some((direction, target))
        })
        .collect()
}

/// Where to use a move, and on whom, if it would reach someone worth it:
/// the nearest target for a directional move (a draw among the nearest; the
/// worthiest first for a picky user), where the user faces for a move on
/// the room, the floor or itself.
fn consider<'a>(rng: &mut DungeonRng, floor: &Floor, user: &AiUser, slot: &MoveSlot, others: &'a [Candidate]) -> Option<(usize, Option<&'a Candidate>)> {
    let aim = (slot.ai_target_range & 0xF) as u8;
    let move_range = (slot.ai_target_range >> 4 & 0xF) as u8;
    match (slot.ai_target_range >> 8 & 0xF) as u8 {
        when::HURT if user.full_hp => return None,
        when::SOMETIMES if rng.rand_int(100) >= slot.ai_chance => return None,
        _ => {}
    }
    let fits = |other: &&Candidate| wants(aim, Some(other)) && !wasted(user, slot, other);
    let worth = match move_range {
        RANGE_SELF => wants(aim, None),
        range::ROOM => wants(aim, None) || others.iter().filter(fits).any(|other| can_see(floor, user.pos, other.pos)),
        range::FLOOR => wants(aim, None) || others.iter().any(|other| fits(&other)),
        _ => {
            let aimed = aimed(floor, user, slot, others);
            let distance = |target: &Candidate| (target.pos.x - user.pos.x).abs().max((target.pos.y - user.pos.y).abs());
            let rank = |target: &Candidate| (if picky(user) && target.hostile { -target.worth } else { 0 }, distance(target));
            let best = aimed.iter().map(|&(_, target)| rank(target)).min()?;
            let closest: Vec<(usize, &Candidate)> = aimed.into_iter().filter(|&(_, target)| rank(target) == best).collect();
            let (direction, target) = closest[rng.rand_int(closest.len() as i32) as usize];
            return Some((direction, Some(target)));
        }
    };
    worth.then_some((user.facing, None))
}

/// Where to use a move, if it would reach someone worth it (see
/// [`consider`]).
#[cfg(test)]
pub(crate) fn consider_move(rng: &mut DungeonRng, floor: &Floor, user: &AiUser, slot: &MoveSlot, others: &[Candidate]) -> Option<usize> {
    consider(rng, floor, user, slot, others).map(|(direction, _)| direction)
}

/// Where the regular attack would hit a foe next to the user: straight
/// ahead if one stands there, else the first one found turning round; for a
/// picky user, the worthiest of them. Status Checker leaves the frozen be.
pub(crate) fn target_regular_attack(rng: &mut DungeonRng, floor: &Floor, user: &AiUser, others: &[Candidate]) -> Option<usize> {
    use super::status::problem::FROZEN;
    let foe_at = |direction: usize| {
        let tile = ahead(user.pos, direction, 1);
        let spared = |other: &Candidate| user.has(iq::STATUS_CHECKER) && other.problems & FROZEN != 0;
        let foe = others.iter().find(|other| other.hostile && other.pos == tile && !spared(other));
        foe.filter(|_| can_strike(floor, user, direction))
    };
    let mut found = (0..8).map(|turn| (user.facing + turn) % 8).filter_map(|direction| Some((direction, foe_at(direction)?.worth)));
    if !picky(user) {
        return found.next().map(|(direction, _)| direction);
    }
    let found: Vec<(usize, i32)> = found.collect();
    let best = found.iter().map(|&(_, worth)| worth).max()?;
    let best: Vec<usize> = found.into_iter().filter(|&(_, worth)| worth == best).map(|(direction, _)| direction).collect();
    Some(best[rng.rand_int(best.len() as i32) as usize])
}

/// Weak-Type Picker: what a move is worth on its target, by how its type
/// fares there (twice for a move of the user's own type, up to a cap), one
/// more for any.
fn picked_weight(user: &AiUser, slot: &MoveSlot, target: Option<&Candidate>) -> i32 {
    let fares = match target {
        Some(target) if slot.power > 0 => match target.matchups.get(slot.type_id as usize).copied().unwrap_or(matchup::NEUTRAL) {
            matchup::IMMUNE => 0,
            matchup::SUPER_EFFECTIVE => 2,
            _ => 1,
        },
        _ => 1,
    };
    let own = slot.type_id != 0 && (user.types.0 == slot.type_id || user.types.1 == slot.type_id);
    (fares * if own { 2 } else { 1 }).min(PICKER_TOP_WEIGHT) + 1
}

/// Exclusive Move-User: the moves of a linked chain one of which is about
/// to run out (a PP or less), which would come apart if used.
fn about_to_delink(moves: &[MoveSlot]) -> Vec<bool> {
    let mut spared = vec![false; moves.len()];
    let mut start = 0;
    for end in 1..=moves.len() {
        if end < moves.len() && moves[end].linked {
            continue;
        }
        let chain = &moves[start..end];
        if chain.len() > 1 && chain.iter().any(|slot| slot.pp < 2) {
            spared[start..end].iter_mut().for_each(|spared| *spared = true);
        }
        start = end;
    }
    spared
}

/// The attack a monster uses this turn, if any; without one, it moves.
///
/// Every move with PP that would reach a target, and the regular attack if
/// a foe stands next to the user, take part in a draw by weight; the
/// regular attack weighs `regular_weights[number of moves known]`. With
/// Exclusive Move-User the regular attack is left out (but for a monster
/// out of PP, which struggles), and so are linked moves about to come
/// apart; with Weak-Type Picker the weights go by how well the moves fare,
/// and only the heaviest are drawn.
// (Out of PP, a monster struggles: see the caller. Moves the rules cannot
// carry out yet (`MoveSlot::usable`) stay out of the draw.)
pub(crate) fn choose_move(
    rng: &mut DungeonRng,
    floor: &Floor,
    regular_weights: &[i32; 5],
    user: &AiUser,
    moves: &[MoveSlot],
    others: &[Candidate],
) -> Option<AiChoice> {
    let exclusive = user.has(iq::EXCLUSIVE_MOVE_USER);
    let picker = user.has(iq::WEAK_TYPE_PICKER);
    let delinking = if exclusive { about_to_delink(moves) } else { vec![false; moves.len()] };
    let mut options: Vec<(i32, AiChoice)> = moves
        .iter()
        .enumerate()
        .filter(|&(index, slot)| slot.usable() && slot.ai_weight > 0 && !delinking[index])
        .filter_map(|(index, slot)| {
            let (direction, target) = consider(rng, floor, user, slot, others)?;
            let weight = if picker { picked_weight(user, slot, target) } else { slot.ai_weight };
            Some((weight, AiChoice { slot: Some(index), direction }))
        })
        .collect();
    let out_of_pp = !moves.is_empty() && moves.iter().all(|slot| slot.pp <= 0);
    if (!exclusive || out_of_pp)
        && let Some(direction) = target_regular_attack(rng, floor, user, others)
    {
        let weight = if picker { PICKER_REGULAR_WEIGHT } else { regular_weights[moves.len().min(regular_weights.len() - 1)].max(1) };
        options.push((weight, AiChoice { slot: None, direction }));
    }
    if picker {
        let top = options.iter().map(|&(weight, _)| weight).max().unwrap_or(0);
        options.retain(|&(weight, _)| weight == top);
    }
    draw(rng, &options)
}

/// One of the options, each as likely as its weight.
fn draw(rng: &mut DungeonRng, options: &[(i32, AiChoice)]) -> Option<AiChoice> {
    let total: i32 = options.iter().map(|&(weight, _)| weight).sum();
    if total <= 0 {
        return None;
    }
    let mut roll = rng.rand_int(total);
    for &(weight, choice) in options {
        if roll < weight {
            return Some(choice);
        }
        roll -= weight;
    }
    None
}

/// The first step of a shortest way from `from` to a tile next to `to`,
/// around walls and the tiles `blocked` tells; `None` when already there
/// or when there is no way.
pub(crate) fn step_towards(floor: &Floor, from: Pos, to: Pos, mobility: Mobility, blocked: impl Fn(Pos) -> bool) -> Option<(i32, i32)> {
    use super::floor::{FLOOR_HEIGHT, FLOOR_WIDTH};
    let index = |p: Pos| (p.y * FLOOR_WIDTH + p.x) as usize;
    let near = |p: Pos| (p.x - to.x).abs() <= 1 && (p.y - to.y).abs() <= 1;
    if near(from) {
        return None;
    }
    // Breadth first; each tile remembers the first step that led to it.
    let mut first: Vec<Option<(i32, i32)>> = vec![None; (FLOOR_WIDTH * FLOOR_HEIGHT) as usize];
    let mut queue = std::collections::VecDeque::from([from]);
    first[index(from)] = Some((0, 0));
    while let Some(at) = queue.pop_front() {
        for &(dx, dy) in &DIRECTIONS {
            let next = Pos::new(at.x + dx, at.y + dy);
            if !Floor::in_bounds(next) || first[index(next)].is_some() || !floor.can_step_as(at, dx, dy, mobility) || blocked(next) {
                continue;
            }
            let step = if at == from { (dx, dy) } else { first[index(at)]? };
            if near(next) {
                return Some(step);
            }
            first[index(next)] = Some(step);
            queue.push_back(next);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEIGHTS: [i32; 5] = [100, 20, 30, 40, 50];

    /// A room with a hallway leaving it to the right.
    fn floor() -> Floor {
        let mut floor = Floor::new_walled();
        for y in 5..12 {
            for x in 5..16 {
                floor.set(Pos::new(x, y), Tile::Floor { room: Some(0) });
            }
        }
        for x in 16..24 {
            floor.set(Pos::new(x, 8), Tile::Floor { room: None });
        }
        floor
    }

    fn user(x: i32, y: i32) -> AiUser {
        AiUser { pos: Pos::new(x, y), facing: 0, mobility: Mobility::Normal, full_hp: true, skills: 1 << iq::COURSE_CHECKER, types: (0, 0) }
    }

    fn foe(x: i32, y: i32) -> Candidate {
        Candidate::new(Pos::new(x, y), true)
    }

    fn friend(x: i32, y: i32) -> Candidate {
        Candidate::new(Pos::new(x, y), false)
    }

    /// A damaging move with the given AI weight and AI target/range.
    fn slot(ai_weight: i32, ai_target_range: u16) -> MoveSlot {
        MoveSlot { id: 1, power: 40, pp: 10, max_pp: 10, ai_weight, ai_target_range, ..MoveSlot::fallback_regular_attack() }
    }

    #[test]
    fn the_way_back_goes_around_walls() {
        let mut floor = floor();
        // A wall across the room, open at the bottom only.
        for y in 5..11 {
            floor.set(Pos::new(10, y), Tile::Wall);
        }
        let (from, to) = (Pos::new(8, 6), Pos::new(12, 6));
        let walk = |blocked: &dyn Fn(Pos) -> bool| {
            let (mut at, mut steps) = (from, 0);
            while let Some((dx, dy)) = step_towards(&floor, at, to, Mobility::Normal, blocked) {
                at = Pos::new(at.x + dx, at.y + dy);
                steps += 1;
                assert!(steps < 50 && floor.tile(at).is_walkable() && !blocked(at));
            }
            (at, steps)
        };
        let (end, steps) = walk(&|_| false);
        assert!((end.x - to.x).abs() <= 1 && (end.y - to.y).abs() <= 1);
        assert!(steps > 6, "{steps}");
        let gap = Pos::new(10, 11);
        assert_eq!(step_towards(&floor, from, to, Mobility::Normal, |p| p == gap), None);
    }

    #[test]
    fn sight_covers_the_room_and_its_rim() {
        let floor = floor();
        assert!(can_see(&floor, Pos::new(6, 6), Pos::new(14, 10)));
        assert!(can_see(&floor, Pos::new(6, 6), Pos::new(16, 8)));
        assert!(!can_see(&floor, Pos::new(6, 6), Pos::new(17, 8)));
        // In the hallway, two tiles each way.
        assert!(can_see(&floor, Pos::new(20, 8), Pos::new(18, 8)));
        assert!(!can_see(&floor, Pos::new(20, 8), Pos::new(17, 8)));
    }

    #[test]
    fn regular_attack_needs_an_adjacent_enemy() {
        let floor = floor();
        assert_eq!(target_regular_attack(&mut DungeonRng::new(1), &floor, &user(8, 8), &[friend(9, 8)]), None);
        assert_eq!(target_regular_attack(&mut DungeonRng::new(1), &floor, &user(8, 8), &[foe(10, 8)]), None);
        assert_eq!(target_regular_attack(&mut DungeonRng::new(1), &floor, &user(8, 8), &[foe(9, 8)]), Some(2));
        // The one in front first.
        assert_eq!(target_regular_attack(&mut DungeonRng::new(1), &floor, &user(8, 8), &[foe(9, 8), foe(8, 9)]), Some(0));
        // Not around the corner of the hallway's mouth.
        assert_eq!(target_regular_attack(&mut DungeonRng::new(1), &floor, &user(16, 8), &[foe(15, 7)]), None);
    }

    #[test]
    fn line_moves_hit_the_first_in_the_way() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let line = slot(10, 0x50);
        let front = slot(10, 0x00);
        let me = user(6, 8);
        assert_eq!(consider_move(&mut rng, &floor, &me, &line, &[foe(12, 8)]), Some(2));
        assert_eq!(consider_move(&mut rng, &floor, &me, &front, &[foe(12, 8)]), None);
        assert_eq!(consider_move(&mut rng, &floor, &me, &line, &[foe(8, 10)]), Some(1));
        assert_eq!(consider_move(&mut rng, &floor, &me, &line, &[foe(12, 9)]), None);
        // A friend in the way shields the foe behind.
        assert_eq!(consider_move(&mut rng, &floor, &me, &line, &[friend(9, 8), foe(12, 8)]), None);
        // The nearest of two foes is aimed at.
        assert_eq!(consider_move(&mut rng, &floor, &me, &line, &[foe(6, 11), foe(12, 8)]), Some(0));
        // From a hallway only the nearby tiles are seen.
        let lurker = user(22, 8);
        assert_eq!(consider_move(&mut rng, &floor, &lurker, &line, &[foe(18, 8)]), None);
        assert_eq!(consider_move(&mut rng, &floor, &lurker, &line, &[foe(20, 8)]), Some(6));
    }

    #[test]
    fn room_moves_reach_the_whole_room() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let room = slot(10, 0x30);
        let me = user(6, 6);
        assert_eq!(consider_move(&mut rng, &floor, &me, &room, &[foe(14, 10)]), Some(0));
        assert_eq!(consider_move(&mut rng, &floor, &me, &room, &[foe(16, 8)]), Some(0));
        assert_eq!(consider_move(&mut rng, &floor, &me, &room, &[foe(19, 8)]), None);
        // Nobody to hit: kept for later.
        assert_eq!(consider_move(&mut rng, &floor, &me, &room, &[friend(8, 8)]), None);
    }

    #[test]
    fn moves_for_allies_go_to_allies() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let help = slot(10, 0x06);
        let me = user(8, 8);
        assert_eq!(consider_move(&mut rng, &floor, &me, &help, &[foe(9, 8)]), None);
        assert_eq!(consider_move(&mut rng, &floor, &me, &help, &[friend(9, 8)]), Some(2));
    }

    #[test]
    fn self_heals_wait_for_lost_hp() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let heal = slot(10, 0x273);
        let mut me = user(8, 8);
        assert_eq!(consider_move(&mut rng, &floor, &me, &heal, &[]), None);
        me.full_hp = false;
        assert_eq!(consider_move(&mut rng, &floor, &me, &heal, &[]), Some(0));
    }

    #[test]
    fn sometimes_follows_its_chance() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let sometimes = MoveSlot { ai_chance: 30, ..slot(10, 0x100) };
        let me = user(8, 8);
        let used = (0..2000).filter(|_| consider_move(&mut rng, &floor, &me, &sometimes, &[foe(9, 8)]).is_some()).count();
        assert!((500..700).contains(&used), "{used}");
    }

    #[test]
    fn moves_are_drawn_by_weight() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let me = user(8, 8);
        let others = [foe(9, 8)];
        // Weights 60 and 20, and 30 for the regular attack with two moves.
        let moves = [slot(60, 0x00), slot(20, 0x00)];
        let mut counts = [0; 3];
        for _ in 0..11000 {
            let choice = choose_move(&mut rng, &floor, &WEIGHTS, &me, &moves, &others).unwrap();
            assert_eq!(choice.direction, 2);
            counts[choice.slot.unwrap_or(2)] += 1;
        }
        assert!((5700..6300).contains(&counts[0]), "{counts:?}");
        assert!((1700..2300).contains(&counts[1]), "{counts:?}");
        assert!((2700..3300).contains(&counts[2]), "{counts:?}");
    }

    #[test]
    fn only_what_reaches_is_drawn() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let me = user(6, 8);
        let moves = [slot(60, 0x00), slot(20, 0x50)];
        for _ in 0..200 {
            assert_eq!(choose_move(&mut rng, &floor, &WEIGHTS, &me, &moves, &[foe(12, 9)]), None);
        }
        // On the row, the line move is all that reaches.
        for _ in 0..200 {
            assert_eq!(choose_move(&mut rng, &floor, &WEIGHTS, &me, &moves, &[foe(12, 8)]), Some(AiChoice { slot: Some(1), direction: 2 }));
        }
        // Out of PP, the regular attack is all that is left.
        let spent = [MoveSlot { pp: 0, ..moves[0] }];
        let choice = choose_move(&mut rng, &floor, &WEIGHTS, &me, &spent, &[foe(7, 9)]);
        assert_eq!(choice, Some(AiChoice { slot: None, direction: 1 }));
    }

    fn with(mut me: AiUser, skill: u8) -> AiUser {
        me.skills |= 1 << skill;
        me
    }

    #[test]
    fn course_checker_and_status_checker_spare_wasted_moves() {
        use crate::dungeon::status::problem;
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let line = slot(10, 0x50);
        // Without Course Checker the friend in the way is flown past.
        let blind = AiUser { skills: 0, ..user(6, 8) };
        assert_eq!(consider_move(&mut rng, &floor, &blind, &line, &[friend(9, 8), foe(12, 8)]), Some(2));
        // A sleep move on a sleeper, the regular attack on the frozen.
        let hypnosis = MoveSlot { id: 4, power: 0, target: 0, ..slot(10, 0x00) };
        let sleeper = Candidate { problems: problem::ASLEEP, ..foe(9, 8) };
        let me = user(8, 8);
        assert_eq!(consider_move(&mut rng, &floor, &me, &hypnosis, &[sleeper]), Some(2));
        let checker = with(me, iq::STATUS_CHECKER);
        assert_eq!(consider_move(&mut rng, &floor, &checker, &hypnosis, &[sleeper]), None);
        let frozen = Candidate { problems: problem::FROZEN, ..foe(9, 8) };
        assert_eq!(target_regular_attack(&mut rng, &floor, &checker, &[frozen, foe(8, 9)]), Some(0));
    }

    #[test]
    fn picky_users_go_for_the_worthiest() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let me = with(user(8, 8), iq::EFFICIENCY_EXPERT);
        let weak = Candidate { worth: 400, ..foe(9, 8) };
        for _ in 0..20 {
            assert_eq!(target_regular_attack(&mut rng, &floor, &me, &[foe(8, 9), weak]), Some(2));
        }
        // A line move: the worthiest before the nearest.
        let line = slot(10, 0x50);
        assert_eq!(consider_move(&mut rng, &floor, &me, &line, &[foe(8, 9), Candidate { worth: 400, ..foe(12, 8) }]), Some(2));
    }

    #[test]
    fn exclusive_move_users_and_weak_type_pickers_choose_their_way() {
        let floor = floor();
        let mut rng = DungeonRng::new(7);
        let me = with(user(8, 8), iq::EXCLUSIVE_MOVE_USER);
        let others = [foe(9, 8)];
        // Only moves, never the regular attack; none in reach, nothing.
        let moves = [slot(60, 0x00)];
        for _ in 0..50 {
            assert_eq!(choose_move(&mut rng, &floor, &WEIGHTS, &me, &moves, &others), Some(AiChoice { slot: Some(0), direction: 2 }));
        }
        assert_eq!(choose_move(&mut rng, &floor, &WEIGHTS, &me, &[slot(60, 0x50)], &[foe(10, 9)]), None);
        // A linked chain about to come apart is kept back.
        let chain = [MoveSlot { pp: 1, ..slot(60, 0x00) }, MoveSlot { linked: true, ..slot(60, 0x00) }, slot(5, 0x00)];
        for _ in 0..50 {
            assert_eq!(choose_move(&mut rng, &floor, &WEIGHTS, &me, &chain, &others).and_then(|choice| choice.slot), Some(2));
        }
        // Out of PP, it struggles all the same.
        let spent = [MoveSlot { pp: 0, ..slot(60, 0x00) }];
        assert_eq!(choose_move(&mut rng, &floor, &WEIGHTS, &me, &spent, &others), Some(AiChoice { slot: None, direction: 2 }));
        // Weak-Type Picker: the move that fares best, always.
        const WATER: u8 = 11;
        let picker = with(user(8, 8), iq::WEAK_TYPE_PICKER);
        let mut weak = foe(9, 8);
        weak.matchups[WATER as usize] = matchup::SUPER_EFFECTIVE;
        let moves = [slot(60, 0x00), MoveSlot { type_id: WATER, ..slot(1, 0x00) }];
        for _ in 0..50 {
            assert_eq!(choose_move(&mut rng, &floor, &WEIGHTS, &picker, &moves, &[weak]).and_then(|choice| choice.slot), Some(1));
        }
    }
}
