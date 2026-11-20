//! Places found on the stage as it stands: free ground, ways round whoever
//! stands there, spots said in relation to someone. What is seen on the way
//! is noted, for whoever watches the scene.

use std::collections::HashSet;

use crate::problem::{Arg, point, told};

use super::*;

mod ways;

/// The least between two who stand side by side, then two who face each other,
/// then two in a column.
const LEAST_GAP: f32 = 28.0;
const FACING_GAP: f32 = 72.0;
const COLUMN_GAP: f32 = 36.0;
/// The smallest ring a formation stands in.
const RING_RADIUS: f32 = 30.0;
/// How far from the middle of a place the view goes, about.
const UNSEEN: Vec2 = Vec2::new(136.0, 104.0);
/// How far from the middle what is out of sight stands where there is no ground.
const OFF: f32 = 150.0;
/// The notes kept at most.
const MOST_NOTES: usize = 200;

/// What was seen as a place was found: the line it was written on (the
/// number the film gives, 0 if none), what, and whether it is wrong or a remark.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub line: i32,
    pub text: String,
    pub wrong: bool,
    /// Only a place moved a little, as was to be expected.
    pub small: bool,
    /// Those its text names (`diagnostics::Mention`).
    pub mentions: Vec<diagnostics::Mention>,
}

/// What the stage knows of the ground it stands on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Placing {
    pub ground: Option<Ground>,
    pub middle: Vec2,
    /// Those who walk at a pace of their own, until their next walk.
    pub paced: HashSet<Who>,
    pub flying: HashSet<Who>,
    pub spots: HashMap<String, Vec2>,
    /// The place's ways in and out, by name (see `PlaceBeat::Entrances`).
    pub entrances: Vec<(String, Vec2)>,
    /// Who goes out or comes in now, and by which way.
    pub leaving: BTreeMap<Who, Vec2>,
    pub entering: BTreeMap<Who, Vec2>,
    /// The line at hand, and what was seen.
    pub line: i32,
    pub notes: Vec<Note>,
}

/// How near an edge a way out is to lead off the place; how far past the edge
/// one walks before it is gone; how far behind each other two go in file.
const NEAR_EDGE: f32 = 48.0;
const PAST: f32 = 32.0;
const IN_FILE: f32 = 26.0;

fn tiles(at: Vec2) -> String {
    format!("{} {}", (at.x / TILE).floor(), (at.y / TILE).floor())
}

/// A number between -1 and 1 that seems drawn by chance, the same for a
/// place each time (`salt`: which of its numbers).
pub(crate) fn scatter(row: i32, column: i32, salt: u32) -> f32 {
    let mut mixed = (row as u32).wrapping_mul(0x9E37_79B1) ^ (column as u32).wrapping_mul(0x85EB_CA77) ^ salt.wrapping_mul(0xC2B2_AE3D);
    mixed ^= mixed >> 15;
    mixed = mixed.wrapping_mul(0x2C1B_3C6D);
    mixed ^= mixed >> 12;
    (mixed % 2001) as f32 / 1000.0 - 1.0
}

impl Stage {
    /// Something the host could not play as written, told as the place's are.
    pub fn wrong(&mut self, text: String) {
        self.note(text, true);
    }

    fn note(&mut self, said: impl Into<diagnostics::Said>, wrong: bool) {
        if self.place.notes.len() < MOST_NOTES {
            let said = said.into();
            self.place.notes.push(Note { line: self.place.line, text: said.text, wrong, small: false, mentions: said.mentions });
        }
    }

    /// A place moved a little: told, but only to who asks for all of it.
    fn moved(&mut self, text: diagnostics::Said) {
        self.note(text, false);
        if let Some(note) = self.place.notes.last_mut() {
            note.small = true;
        }
    }

    /// How far apart two stand, by the size of their bodies.
    fn gap(&self, who: Option<Who>, other: Who) -> f32 {
        let wide = |one: Option<Who>| one.map_or(USUAL_BODY.x, |one| self.view.body(one).x);
        (((wide(who) + wide(Some(other))) as i32 / 2 + 6) as f32).max(LEAST_GAP)
    }

    /// Whether someone takes up room on the ground.
    fn stands(&self, who: Who, actor: &Actor) -> bool {
        matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. } | Who::Thing { .. })
            && !self.place.flying.contains(&who)
            && actor.air.height == 0.0
            && actor.habits.carried_by.is_none()
    }

    /// Those who stand on the ground and do not go anywhere.
    fn standing(&self, who: Option<Who>) -> impl Iterator<Item = (Who, Vec2)> {
        self.cast
            .iter()
            .filter(move |(one, actor)| Some(**one) != who && self.stands(**one, actor) && !actor.walks())
            .map(|(one, actor)| (*one, actor.position))
    }

    /// The walls, and those standing there as walls too (`by_body`: each as
    /// wide as it shows, to stand clear of it); none with no collisions,
    /// nothing being in the way.
    pub(crate) fn crowded(&self, who: Option<Who>, by_body: bool) -> Option<Ground> {
        let ground = self.place.ground.as_ref().filter(|_| !self.no_collisions)?;
        let reach = |other: Who| {
            let wide = match who.filter(|_| by_body) {
                Some(who) => ((self.view.body(other).x + self.view.body(who).x) as i32 / 2 - 8) as f32,
                None => 16.0,
            }
            .max(16.0);
            (wide, (wide as i32 / 2) as f32)
        };
        Some(ground.with(self.standing(who).map(|(other, at)| {
            let (wide, high) = reach(other);
            (at, wide, high.max(16.0))
        })))
    }

    /// The point of a spot, as things stand.
    pub(crate) fn point(&mut self, who: Option<Who>, spot: &Spot) -> Option<Vec2> {
        let there = |stage: &Stage, of: Who| stage.cast.get(&of).map(|actor| actor.position);
        let missing = |stage: &mut Stage, of: Who| {
            stage.note(told!("{} is not on stage", of), true);
            None
        };
        // (From where someone comes, a way of its own if it stands there.)
        let from = |stage: &Stage, other: Vec2| {
            let start = who.and_then(|who| there(stage, who)).unwrap_or(stage.place.middle);
            if start.distance(other) < 1.0 { other - Vec2::X } else { start }
        };
        macro_rules! of {
            ($who:expr) => {
                match there(self, *$who) {
                    Some(at) => at,
                    None => return missing(self, *$who),
                }
            };
        }
        Some(match spot {
            Spot::Written(at) | Spot::Word(at) | Spot::Exact(at) | Spot::Near(at) => at.vec2(),
            Spot::Named(name) => match self.place.spots.get(name) {
                Some(at) => *at,
                None => {
                    self.note(format!("“{name}” is not a spot here"), true);
                    return None;
                }
            },
            Spot::Of(of) => of!(of),
            Spot::Before { of, back } => {
                let at = of!(of);
                let far = self.gap(who, *of) * if *back { -1.0 } else { 1.0 };
                at + step_of(self.cast[of].direction).normalize_or_zero() * far
            }
            Spot::Side { of, right } => of!(of) + Vec2::X * self.gap(who, *of) * if *right { 1.0 } else { -1.0 },
            Spot::Between(a, b) => (of!(a) + of!(b)) / 2.0,
            Spot::Tiles { of, count } => {
                let at = of!(of);
                let start = from(self, at);
                at + (start - at).normalize() * count * TILE
            }
            Spot::Beside(of) => {
                let at = of!(of);
                let start = from(self, at);
                at + (start - at).normalize() * self.gap(who, *of)
            }
            // (Who a crowd had no room for is not told again.)
            Spot::By { of, dx, dy } => there(self, *of)? + Vec2::new(*dx, *dy),
            Spot::Shape { kind, who: those, at, round } => {
                let middle = self.point(those.first().copied(), round)?;
                self.formation(*kind, those, middle).get(*at).copied()?
            }
        })
    }

    /// Where each of a formation stands, round a point.
    fn formation(&self, kind: Formation, who: &[Who], middle: Vec2) -> Vec<Vec2> {
        let gap = |first: Who, second: Who| self.gap(Some(first), second);
        match kind {
            Formation::Facing => {
                let half = (FACING_GAP.max(gap(who[0], who[1])) as i32 / 2) as f32;
                vec![middle - Vec2::X * half, middle + Vec2::X * half]
            }
            Formation::Triangle => {
                let far = gap(who[0], who[1]).max(gap(who[1], who[2]));
                vec![middle - Vec2::Y * (far / 2.0).floor(), middle + Vec2::new(-far, (far / 2.0).floor()), middle + Vec2::new(far, (far / 2.0).floor())]
            }
            Formation::Ring => {
                // (Round enough for each to keep the gap to the next.)
                let gaps: f32 = who.iter().zip(who.iter().cycle().skip(1)).map(|(a, b)| gap(*a, *b)).sum();
                let radius = (gaps / std::f32::consts::TAU).max(RING_RADIUS).round();
                let many = who.len() as f32;
                (0..who.len())
                    .map(|at| {
                        let angle = std::f32::consts::TAU * at as f32 / many;
                        middle + (Vec2::new(angle.sin(), -angle.cos()) * radius).round()
                    })
                    .collect()
            }
            Formation::Line | Formation::Column => {
                let mut along = vec![0.0_f32];
                for pair in who.windows(2) {
                    let step = match kind {
                        Formation::Line => gap(pair[0], pair[1]),
                        _ => COLUMN_GAP.max(((self.view.body(pair[0]).y + self.view.body(pair[1]).y) as i32 / 3) as f32),
                    };
                    along.push(along[along.len() - 1] + step);
                }
                let shift = (along[along.len() - 1] as i32 / 2) as f32;
                let way = if kind == Formation::Line { Vec2::X } else { Vec2::Y };
                along.into_iter().map(|at| middle + way * (at - shift)).collect()
            }
        }
    }

    /// A place found as a line says it: on free ground, unless it says it
    /// is exact or who is meant flies. The middle if it is not known.
    pub(crate) fn find(&mut self, who: Option<Who>, spot: &Spot, apart: bool, exact: bool) -> Vec2 {
        let Some(at) = self.point(who, spot) else {
            return self.place.middle;
        };
        let flying = who.is_some_and(|who| self.place.flying.contains(&who));
        if exact || who.is_none() || flying || matches!(spot, Spot::Exact(_)) {
            return at;
        }
        // (No collisions: just there; only two sent to the same word, or to
        // someone, kept from standing on each other.)
        if self.no_collisions {
            return if apart && !matches!(spot, Spot::Written(_)) { self.apart(who, at) } else { at };
        }
        let who_said = || who.map_or(Arg::from(""), Arg::from);
        let there = match spot {
            Spot::Written(_) | Spot::Word(_) => {
                let there = self.crowded(who, false).map_or(at, |ground| ground.free(at));
                if matches!(spot, Spot::Written(_)) && there.distance(at) > 24.0 {
                    self.moved(told!("{} cannot be at {}: put at {}", who_said(), point(at), point(there)));
                }
                there
            }
            _ => {
                let there = self.crowded(who, true).map_or(at, |ground| ground.free(at));
                if there.distance(at) > TILE {
                    self.moved(told!("{} cannot be on tile {} (a wall or someone): put on tile {}", who_said(), tiles(at), tiles(there)));
                }
                there
            }
        };
        if apart { self.apart(who, there) } else { there }
    }

    /// The nearest free spot to that point with no one else going there.
    fn apart(&self, who: Option<Who>, at: Vec2) -> Vec2 {
        const AROUND: [(f32, f32); 9] =
            [(0.0, 0.0), (-36.0, 0.0), (36.0, 0.0), (0.0, 22.0), (0.0, -22.0), (-72.0, 0.0), (72.0, 0.0), (-36.0, 22.0), (36.0, 22.0)];
        let taken: Vec<Vec2> = self
            .cast
            .iter()
            .filter(|(one, actor)| Some(**one) != who && self.stands(**one, actor))
            .map(|(_, actor)| if actor.walks() { actor.goal() } else { actor.position })
            .collect();
        let crowded = self.crowded(who, false);
        for (dx, dy) in AROUND {
            let near = at + Vec2::new(dx, dy);
            let spot = crowded.as_ref().map_or(near, |ground| ground.free(near));
            if !taken.iter().any(|there| (spot.x - there.x).abs() < 24.0 && (spot.y - there.y).abs() < 14.0) {
                return spot;
            }
        }
        at
    }
    /// What is in the way of someone going round a point is told.
    pub(crate) fn round_check(&mut self, who: Who, around: Vec2) {
        let Some(here) = self.cast.get(&who).map(|actor| actor.position) else {
            return;
        };
        let reach = here.distance(around);
        // (Those on the same circle go round with it.)
        let Some(ground) = self.place.ground.clone().filter(|_| !self.no_collisions) else {
            return;
        };
        let others = self.standing(Some(who)).filter(|(_, at)| (at.distance(around) - reach).abs() >= 8.0).map(|(_, at)| (at, 16.0, 16.0));
        let crowded = ground.with(others);
        for step in 0..36 {
            let angle = step as f32 * std::f32::consts::PI / 18.0;
            let at = around + Vec2::from_angle(angle) * reach;
            if crowded.blocked(at, 0) {
                self.note(told!("{} turns through a wall or someone, towards {}", who, point(at)), false);
                return;
            }
        }
    }

    /// As wide as the ground leaves someone to stroll; none where it has no
    /// room.
    pub(crate) fn room_to_roam(&mut self, who: Who, mut wide: f32) -> Option<f32> {
        let (Some(ground), Some(home)) = (self.place.ground.clone().filter(|_| !self.no_collisions), self.cast.get(&who).map(|actor| actor.position)) else {
            return Some(wide);
        };
        let ring = |far: f32| (0..16).any(|at| ground.blocked(home + Vec2::from_angle(at as f32 * std::f32::consts::PI / 8.0) * far, 0));
        while wide > 8.0 && ring(wide) {
            wide -= 8.0;
        }
        if ring(wide) {
            self.note(told!("{} has no room to stroll here", who), false);
            return None;
        }
        Some(wide)
    }

    /// Free ground about a point, the nearest first, a place for each.
    pub(crate) fn spots_around(&self, at: Vec2, count: usize, step: Vec2) -> Vec<Vec2> {
        let mut found: Vec<Vec2> = Vec::with_capacity(count);
        let taken: Vec<Vec2> = self.standing(None).map(|(_, at)| at).collect();
        let free = |spot: Vec2, found: &[Vec2]| {
            taken.iter().chain(found).all(|there| (spot.x - there.x).abs() >= step.x * 0.7 || (spot.y - there.y).abs() >= step.y * 0.6)
        };
        // (A crowd, not a parade: each a little off its place on a grid, the
        // nearest to the middle first, a ragged edge, wider than deep as the
        // screen is over its dialogue box.)
        let mut reach = 2_i32;
        while ((2 * reach + 1) * (2 * reach + 1)) < (count as i32) * 4 {
            reach += 1;
        }
        let mut spots: Vec<(f32, Vec2)> = Vec::new();
        for row in -reach..=reach {
            for column in -reach..=reach {
                let (off_x, off_y, edge) = (scatter(row, column, 1), scatter(row, column, 2), scatter(row, column, 3));
                let aside = row.rem_euclid(2) as f32 * 0.5;
                let spot = at + Vec2::new((column as f32 + aside + off_x * 0.4) * step.x, (row as f32 + off_y * 0.35) * step.y);
                let far = ((column as f32 + aside).powi(2) + (row as f32 * 1.7).powi(2)).sqrt() + (edge + 1.0) * 0.6;
                spots.push((far, spot));
            }
        }
        spots.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, spot) in spots {
            if found.len() == count {
                break;
            }
            if free(spot, &found) && (self.no_collisions || self.place.ground.as_ref().is_none_or(|ground| !ground.blocked(spot, 1))) {
                found.push(spot);
            }
        }
        found
    }

    /// A crowd's middle and how wide it stands: away from those who act.
    pub(crate) fn crowd_round(&self, who: &[Who], at: Vec2) -> (Vec2, f32) {
        let middle = self.place.middle;
        let wide = 30.0 + who.len() as f32 * 5.0;
        let away = if at.x < middle.x { -1.0 } else { 1.0 };
        let others: Vec<Vec2> = self.standing(None).filter(|(one, _)| !who.contains(one) && !matches!(one, Who::Thing { .. })).map(|(_, at)| at).collect();
        let mut at = at;
        while others.iter().any(|there| at.distance(*there) < wide + 12.0) && (at.x - middle.x).abs() < 200.0 {
            at.x += away * 12.0;
        }
        (at, wide)
    }

    /// Where those of a crowd stand about a point, in its form but a ring
    /// (`crowd_round`): a flock and a heap on free ground, the nearest to
    /// the middle first; a row side by side.
    pub(crate) fn crowd_shape(&self, at: Vec2, count: usize, form: &crate::beat::CrowdForm) -> Vec<Vec2> {
        use crate::beat::CrowdShape;
        let tight = if form.tight { 0.75 } else { 1.0 };
        if form.shape == CrowdShape::Line {
            let step = match form.wide {
                Some(wide) if count > 1 => 2.0 * wide / (count - 1) as f32,
                _ => 26.0 * tight,
            };
            let left = at.x - step * (count.saturating_sub(1)) as f32 / 2.0;
            return (0..count).map(|place| Vec2::new(left + step * place as f32, at.y)).collect();
        }
        let usual = if form.shape == CrowdShape::Heap { Vec2::new(16.0, 12.0) } else { Vec2::new(22.0, 16.0) };
        let step = match form.wide {
            // (So many a row that the furthest stand that far.)
            Some(wide) => {
                let across = (2.0 * wide / (count as f32).sqrt().ceil().max(1.0)).clamp(8.0, 64.0);
                Vec2::new(across, across * 0.7)
            }
            None => usual,
        } * tight;
        let mut spots = self.spots_around(at, count, step);
        // (No room left for some: on top of the others rather than nowhere.)
        while spots.len() < count {
            spots.push(at + Vec2::new(scatter(spots.len() as i32, 3, 1), scatter(spots.len() as i32, 3, 2)) * step);
        }
        spots
    }

    /// All who stand on the stage, the view's bodies among them, for a word
    /// that means them all.
    pub fn bodies_here(&self) -> Vec<Who> {
        self.cast
            .iter()
            .filter(|(who, actor)| matches!(who, Who::Character(_) | Who::Monster(_) | Who::Another { .. }) && actor.attributes & attribute::HIDDEN == 0)
            .map(|(who, _)| *who)
            .collect()
    }
}

impl Stage {
    /// There was not room for all of a gathering.
    pub(crate) fn note_room(&mut self, found: usize, asked: usize) {
        self.note(format!("not enough room: {found} out of {asked}"), false);
    }
}
