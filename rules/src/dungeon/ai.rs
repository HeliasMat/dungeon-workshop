//! How a computer-controlled monster picks its attack.
//!
//! The monster looks at each of its moves and at its regular attack, keeps
//! those that would reach someone it wants to hit (or help), and draws one
//! of them by its AI weight. With nothing in reach it walks instead.

use super::combat::{AROUND, MoveSlot, range};
use super::floor::{Floor, Mobility, Pos, Tile};
use super::rng::DungeonRng;

/// The eight directions: down first, then counterclockwise on screen.
pub const DIRECTIONS: [(i32, i32); 8] = AROUND;

/// A move's range nibble for "on the user itself".
const RANGE_SELF: u8 = 7;
/// How far the long line moves fly.
const LONG_LINE: i32 = 10;

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
}

/// Another monster on the floor, as the user sees it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Candidate {
    pub pos: Pos,
    /// An enemy of the user, rather than one of its allies.
    pub hostile: bool,
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

/// The first monster along a line from the user, stopped by walls.
fn first_in_line<'a>(floor: &Floor, user: &AiUser, direction: usize, reach: i32, others: &'a [Candidate]) -> Option<&'a Candidate> {
    (1..=reach)
        .map(|distance| ahead(user.pos, direction, distance))
        .take_while(|&tile| floor.tile(tile) != Tile::Wall)
        .find_map(|tile| others.iter().find(|other| other.pos == tile))
}

/// The directions a directional move would reach a wanted target in, with
/// how far that target stands.
fn aimed(floor: &Floor, user: &AiUser, move_range: u8, aim: u8, others: &[Candidate]) -> Vec<(usize, i32)> {
    let (reach, cuts_corners) = match move_range {
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
            let target = first_in_line(floor, user, direction, reach, others)?;
            let seen = can_see(floor, user.pos, target.pos);
            (seen && wants(aim, Some(target))).then(|| (direction, (target.pos.x - user.pos.x).abs().max((target.pos.y - user.pos.y).abs())))
        })
        .collect()
}

/// Where to use a move, if it would reach someone worth it: the nearest
/// target for a directional move (a draw among the nearest), where the
/// user faces for a move on the room, the floor or itself.
pub(crate) fn consider_move(rng: &mut DungeonRng, floor: &Floor, user: &AiUser, slot: &MoveSlot, others: &[Candidate]) -> Option<usize> {
    let aim = (slot.ai_target_range & 0xF) as u8;
    let move_range = (slot.ai_target_range >> 4 & 0xF) as u8;
    match (slot.ai_target_range >> 8 & 0xF) as u8 {
        when::HURT if user.full_hp => return None,
        when::SOMETIMES if rng.rand_int(100) >= slot.ai_chance => return None,
        _ => {}
    }
    let worth = match move_range {
        RANGE_SELF => wants(aim, None),
        range::ROOM => wants(aim, None) || others.iter().any(|other| can_see(floor, user.pos, other.pos) && wants(aim, Some(other))),
        range::FLOOR => wants(aim, None) || others.iter().any(|other| wants(aim, Some(other))),
        _ => {
            let aimed = aimed(floor, user, move_range, aim, others);
            let nearest = aimed.iter().map(|&(_, distance)| distance).min()?;
            let closest: Vec<usize> = aimed.iter().filter(|&&(_, distance)| distance == nearest).map(|&(direction, _)| direction).collect();
            return Some(closest[rng.rand_int(closest.len() as i32) as usize]);
        }
    };
    worth.then_some(user.facing)
}

/// Where the regular attack would hit a foe next to the user: straight
/// ahead if one stands there, else the first one found turning round.
pub(crate) fn target_regular_attack(floor: &Floor, user: &AiUser, others: &[Candidate]) -> Option<usize> {
    let foe_at = |direction: usize| {
        let tile = ahead(user.pos, direction, 1);
        others.iter().any(|other| other.hostile && other.pos == tile) && can_strike(floor, user, direction)
    };
    (0..8).map(|turn| (user.facing + turn) % 8).find(|&direction| foe_at(direction))
}

/// The attack a monster uses this turn, if any; without one, it moves.
///
/// Every move with PP that would reach a target, and the regular attack if
/// a foe stands next to the user, take part in a draw by weight; the
/// regular attack weighs `regular_weights[number of moves known]`.
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
    let mut options: Vec<(i32, AiChoice)> = moves
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.usable() && slot.ai_weight > 0)
        .filter_map(|(index, slot)| {
            let direction = consider_move(rng, floor, user, slot, others)?;
            Some((slot.ai_weight, AiChoice { slot: Some(index), direction }))
        })
        .collect();
    if let Some(direction) = target_regular_attack(floor, user, others) {
        let weight = regular_weights[moves.len().min(regular_weights.len() - 1)].max(1);
        options.push((weight, AiChoice { slot: None, direction }));
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
        AiUser { pos: Pos::new(x, y), facing: 0, mobility: Mobility::Normal, full_hp: true }
    }

    fn foe(x: i32, y: i32) -> Candidate {
        Candidate { pos: Pos::new(x, y), hostile: true }
    }

    fn friend(x: i32, y: i32) -> Candidate {
        Candidate { pos: Pos::new(x, y), hostile: false }
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
        assert_eq!(target_regular_attack(&floor, &user(8, 8), &[friend(9, 8)]), None);
        assert_eq!(target_regular_attack(&floor, &user(8, 8), &[foe(10, 8)]), None);
        assert_eq!(target_regular_attack(&floor, &user(8, 8), &[foe(9, 8)]), Some(2));
        // The one in front first.
        assert_eq!(target_regular_attack(&floor, &user(8, 8), &[foe(9, 8), foe(8, 9)]), Some(0));
        // Not around the corner of the hallway's mouth.
        assert_eq!(target_regular_attack(&floor, &user(16, 8), &[foe(15, 7)]), None);
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
}
