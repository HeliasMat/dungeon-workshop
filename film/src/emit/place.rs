//! The beats of where everyone stands together.

use engine::{Beat, Direction, Exit, Gait, PlaceBeat, Point, Spot, Who};
use glam::Vec2;

/// Everyone on stage turns to someone.
pub fn all_look(at: Who) -> Beat {
    Beat::Place(PlaceBeat::AllLook { way: None, at: Some(at) })
}

/// Each goes to a seat of those, the place each takes kept under that name.
pub fn seats(var: impl Into<String>, who: Vec<Who>, seats: Vec<Vec2>) -> Beat {
    Beat::Place(PlaceBeat::Seats { var: var.into(), who, seats })
}

/// Everyone on stage turns that way.
pub fn all_look_way(way: Direction) -> Beat {
    Beat::Place(PlaceBeat::AllLook { way: Some(way), at: None })
}

/// Those who fly in this scene: they stand on nothing.
pub fn flies(who: Vec<Who>) -> Beat {
    Beat::Place(PlaceBeat::Flies(who))
}

/// Someone walks out of the place by that way (running, if told), and is gone.
pub fn leaves(who: Who, by: Exit, run: bool) -> Beat {
    Beat::Place(PlaceBeat::Leaves { who, by, run })
}

/// A way out by its name: the nearest on a side (`left`, `right`), or
/// one of the place's own.
pub fn exit(name: &str) -> Exit {
    Exit::parse(name)
}

/// Someone strolls, as wide as the ground leaves it.
pub fn roams(who: Who, wide: f32) -> Beat {
    Beat::Place(PlaceBeat::Roams { who, wide })
}

/// Someone goes by these points, round the walls and those standing there.
pub fn goes(who: Who, points: Vec<Vec2>, gait: Gait) -> Beat {
    Beat::Place(PlaceBeat::Goes { who, points: points.into_iter().map(Point::at).collect(), gait })
}

/// A crowd in its own circle about that point (none: the middle); walking
/// in, or put there.
pub fn crowd(who: Vec<Who>, spot: Option<Spot>, walking: bool, form: engine::CrowdForm) -> Beat {
    Beat::Place(PlaceBeat::Crowd { who, spot, walking, form })
}

/// A crowd going together to a spot, each keeping its place in it.
pub fn flock(who: Vec<Who>, to: Spot, run: bool) -> Beat {
    Beat::Place(PlaceBeat::Flock { who, to, run })
}

/// Joy running through a crowd, from one side to the other.
pub fn wave_through(who: Vec<Who>) -> Beat {
    Beat::Place(PlaceBeat::Wave(who))
}

/// A crowd running that far, staying on the ground.
pub fn advance(who: Vec<Who>, by: Vec2) -> Beat {
    Beat::Place(PlaceBeat::Advance { who, by })
}

impl crate::film::Film {
    /// Who those words name, a crowd as each of its members, each told as
    /// used (`film.other`); none if one of them is no one.
    pub fn bodies(&mut self, words: &[String]) -> Option<Vec<Who>> {
        let members: Vec<String> = words.iter().flat_map(|word| self.everyone(word)).collect();
        members.iter().map(|one| self.other(one)).collect()
    }
}

/// The ground of the place shown (none: none known), and its middle.
pub fn ground(middle: Vec2, ground: Option<engine::Ground>) -> Beat {
    Beat::Place(PlaceBeat::Ground { middle, ground })
}

/// The ways in and out of the place, by name.
pub fn entrances(ways: Vec<(String, Vec2)>) -> Beat {
    Beat::Place(PlaceBeat::Entrances(ways))
}

/// A spot gets a name.
pub fn name_spot(name: impl Into<String>, spot: engine::Spot) -> Beat {
    Beat::Place(PlaceBeat::Name { name: name.into(), spot })
}

/// They are spread out at the scene's start, turned to one another, the
/// view on them (`picture`: on a picture, not a place).
pub fn spread(who: Vec<Who>, picture: bool) -> Beat {
    Beat::Place(PlaceBeat::Spread { who, picture })
}

/// How close a shot comes, less if someone's body asks for it.
pub fn shot(close: f32, who: Option<Who>) -> Beat {
    Beat::Place(PlaceBeat::Shot { close, who })
}

/// Someone goes by these points (found as they play, maybe), round the
/// walls and those standing there.
pub fn goes_by(who: Who, points: Vec<Point>, gait: Gait) -> Beat {
    Beat::Place(PlaceBeat::Goes { who, points, gait })
}

/// Someone walks in to a point (found as it plays, maybe): from where the
/// view does not go, or by that way (`from`); `placing`: who is there walks
/// to it.
pub fn arrives(who: Who, to: Point, gait: Gait, placing: bool, from: Option<Exit>) -> Beat {
    Beat::Place(PlaceBeat::Arrives { who, to, gait, placing, from })
}

/// A spot found as the scene plays, its two numbers kept under that name
/// (`<var>_x`, `<var>_y`): for someone if told, away from the others
/// (`apart`), or just there (`exact`).
pub fn find(var: impl Into<String>, who: Option<Who>, spot: Spot, apart: bool, exact: bool) -> Beat {
    Beat::Place(PlaceBeat::Find { var: var.into(), who, spot, apart, exact })
}

/// What is in the way of someone going round a spot is told.
pub fn round(who: Who, spot: Spot) -> Beat {
    Beat::Place(PlaceBeat::Round { who, spot })
}

/// So many on free ground about a spot, each a step from the last, given
/// two numbers (`<var><n>_x`...); none: a place kept, with no one put.
pub fn gathers(var: impl Into<String>, spot: Spot, step: Vec2, who: Vec<Option<Who>>) -> Beat {
    Beat::Place(PlaceBeat::Gathers { var: var.into(), spot, step, who })
}
