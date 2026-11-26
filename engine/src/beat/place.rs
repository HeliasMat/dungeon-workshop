//! Words whose meaning is the stage's own: where someone stands now, which
//! way it looks, what ground is free. Written by whoever knows no more than
//! the words (a film); found when they are played, on the real stage.

use super::declare::{Opt, declare};
use glam::Vec2;

use super::parse::Fields;
use super::*;
use crate::base::Ground;

/// A place as a line says it (see `Spot::parse`).
#[derive(Clone, Debug, PartialEq)]
pub enum Spot {
    /// Pixels written by hand: put on free ground, told when moved far.
    Written(Point),
    /// A word of the place: on free ground.
    Word(Point),
    /// As it is, whatever stands there.
    Exact(Point),
    /// A tile: on free ground, by the size of the bodies around.
    Near(Point),
    /// A spot named by `setspot`.
    Named(String),
    /// Where someone stands.
    Of(Who),
    /// Before someone as it looks now, or behind it.
    Before {
        of: Who,
        back: bool,
    },
    /// To someone's side.
    Side {
        of: Who,
        right: bool,
    },
    Between(Who, Who),
    /// So many tiles from someone, on the side one comes from.
    Tiles {
        of: Who,
        count: f32,
    },
    /// Beside someone, on the side one comes from.
    Beside(Who),
    /// So far from where someone stands.
    By {
        of: Who,
        dx: f32,
        dy: f32,
    },
    /// A place in a formation of these round a spot.
    Shape {
        kind: Formation,
        who: Vec<Who>,
        at: usize,
        round: Box<Spot>,
    },
}

/// How those of a formation stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formation {
    Line,
    Column,
    Triangle,
    Facing,
    /// In a ring round the point, each facing it.
    Ring,
}

/// A way out of a place: the nearest on a side, or one of its own by name.
#[derive(Clone, Debug, PartialEq)]
pub enum Exit {
    Side { right: bool },
    Named(String),
}

impl Exit {
    /// By its name: the nearest on a side (`left`, `right`), or one of
    /// the place's own.
    pub fn parse(field: &str) -> Exit {
        match field {
            "right" => Exit::Side { right: true },
            "left" => Exit::Side { right: false },
            name => Exit::Named(name.to_string()),
        }
    }
}

/// How fast someone goes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Pace {
    #[default]
    Usual,
    /// So much of a walker's.
    Times(f32),
    /// All the way in so many seconds.
    Seconds(f32),
}

/// How a crowd stands about its middle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrowdShape {
    /// In a ring, all facing its middle.
    #[default]
    Ring,
    /// A tight flock, nearest to the middle first, a ragged edge.
    Flock,
    /// Side by side, in a row.
    Line,
    /// Packed together, tighter than a flock.
    Heap,
}

impl CrowdShape {
    /// The four, with the film's words for them.
    pub const NAMES: [(CrowdShape, &'static str); 4] =
        [(CrowdShape::Ring, "ring"), (CrowdShape::Flock, "flock"), (CrowdShape::Line, "line"), (CrowdShape::Heap, "heap")];

    pub fn of(word: &str) -> Option<CrowdShape> {
        CrowdShape::NAMES.iter().find(|(_, name)| *name == word).map(|(shape, _)| *shape)
    }

    pub fn name(self) -> &'static str {
        CrowdShape::NAMES.iter().find(|(shape, _)| *shape == self).map_or("ring", |(_, name)| name)
    }
}

/// How a crowd stands and comes: its shape, how wide (none: as many as they
/// are), tighter, from which side of the view it walks in (none: from the
/// nearest edge, if it walks in), running.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CrowdForm {
    pub shape: CrowdShape,
    pub wide: Option<f32>,
    pub tight: bool,
    pub from: Option<Direction>,
    pub run: bool,
}

impl CrowdForm {
    /// As fields, each with its name (`shape:flock`, `radius:40`, `tight:1`,
    /// `from:right`, `run:1`); nothing for what is as usual.
    pub fn fields(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.shape != CrowdShape::Ring {
            out.push(format!("shape:{}", self.shape.name()));
        }
        if let Some(wide) = self.wide {
            out.push(format!("radius:{wide}"));
        }
        if self.tight {
            out.push("tight:1".to_string());
        }
        if let Some(way) = self.from {
            out.push(format!("from:{}", way.name()));
        }
        if self.run {
            out.push("run:1".to_string());
        }
        out
    }

    /// Read from such fields; none if one is not one of them.
    fn read(fields: &[String]) -> Option<CrowdForm> {
        let mut form = CrowdForm::default();
        for field in fields {
            let (name, value) = field.split_once(':')?;
            match name {
                "shape" => form.shape = CrowdShape::of(value)?,
                "radius" => form.wide = Some(value.parse().ok()?),
                "tight" => form.tight = true,
                "from" => form.from = Some(Direction::of(value)?),
                "run" => form.run = true,
                _ => return None,
            }
        }
        Some(form)
    }
}

/// How someone goes: running or walking, and how fast.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Gait {
    pub run: bool,
    pub pace: Pace,
}

impl Gait {
    /// A walk, at a walker's pace.
    pub const WALK: Gait = Gait { run: false, pace: Pace::Usual };

    /// As two fields: `run` or `walk`, then the pace.
    pub fn parse(run: &str, pace: &str) -> Option<Gait> {
        Some(Gait { run: run == "run", pace: Pace::parse(pace)? })
    }
}

impl Pace {
    /// As a field: `-` (as usual), `x<times>`, `s<seconds>`.
    pub fn parse(field: &str) -> Option<Pace> {
        Some(match field.split_at_checked(1)? {
            ("-", "") => Pace::Usual,
            ("x", much) => Pace::Times(much.parse().ok()?),
            ("s", seconds) => Pace::Seconds(seconds.parse().ok()?),
            _ => return None,
        })
    }
}

impl Spot {
    /// As a field: its letter, then what it says, with `:` between.
    pub fn parse(field: &str) -> Option<Spot> {
        let mut parts = field.splitn(2, ':');
        let (kind, rest) = (parts.next()?, parts.next().unwrap_or(""));
        let point = |rest: &str| -> Option<Point> {
            let (x, y) = rest.split_once(':')?;
            Some(Point { x: Num::read(x)?, y: Num::read(y)? })
        };
        let who = |rest: &str| Who::parse(rest);
        Some(match kind {
            "w" => Spot::Written(point(rest)?),
            "m" => Spot::Word(point(rest)?),
            "x" => Spot::Exact(point(rest)?),
            "t" => Spot::Near(point(rest)?),
            "n" => Spot::Named(rest.to_string()),
            "o" => Spot::Of(who(rest)?),
            "d" | "b" => Spot::Before { of: who(rest)?, back: kind == "b" },
            "g" | "r" => Spot::Side { of: who(rest)?, right: kind == "r" },
            "e" => {
                let (a, b) = rest.split_once(':')?;
                Spot::Between(who(a)?, who(b)?)
            }
            "c" => {
                let (count, of) = rest.split_once(':')?;
                Spot::Tiles { of: who(of)?, count: count.parse().ok()? }
            }
            "p" => Spot::Beside(who(rest)?),
            "y" => {
                let (of, far) = rest.split_once(':')?;
                let (dx, dy) = far.split_once(':')?;
                Spot::By { of: who(of)?, dx: dx.parse().ok()?, dy: dy.parse().ok()? }
            }
            "f" => {
                let mut parts = rest.splitn(4, ':');
                let kind = match parts.next()? {
                    "row" => Formation::Line,
                    "column" => Formation::Column,
                    "triangle" => Formation::Triangle,
                    "face" => Formation::Facing,
                    "ring" => Formation::Ring,
                    _ => return None,
                };
                let at = parts.next()?.parse().ok()?;
                let who = parts.next()?.split(',').map(Who::parse).collect::<Option<Vec<Who>>>()?;
                // (Face to face takes two, a triangle three, a line at least one.)
                let enough = match kind {
                    Formation::Facing => who.len() == 2,
                    Formation::Triangle => who.len() == 3,
                    Formation::Line | Formation::Column | Formation::Ring => !who.is_empty(),
                };
                if !enough || at >= who.len() {
                    return None;
                }
                Spot::Shape { kind, who, at, round: Box::new(Spot::parse(parts.next()?)?) }
            }
            _ => return None,
        })
    }
}

/// What is found on the stage as it is played.
#[derive(Clone, Debug, PartialEq)]
pub enum PlaceBeat {
    /// The ground of the place shown, and its middle.
    Ground { middle: Vec2, ground: Option<Ground> },
    /// Those who fly in this scene: they stand on nothing.
    Flies(Vec<Who>),
    /// Those who flew (or swam) stand on the ground again: walls hold them.
    Lands(Vec<Who>),
    /// Those who swim by nature: where no one walks (the water), sunk so
    /// deep; on the ground, not.
    Swims { depth: f32, who: Vec<Who> },
    /// Those who pass through walls by nature: seen through while in one.
    Ghosts(Vec<Who>),
    /// Those who hover by nature: a little bob in the air, once on the stage.
    Hovers(Vec<Who>),
    /// A spot gets a name, found now.
    Name { name: String, spot: Spot },
    /// A place found now, put in two numbers (`<var>_x`, `<var>_y`), for whom
    /// (none: for the view); `apart`: where no one else goes; `exact`: as it is.
    Find { var: String, who: Option<Who>, spot: Spot, apart: bool, exact: bool },
    /// They are spread out at the scene's start, turned to one another, the
    /// view on them.
    Spread { who: Vec<Who>, picture: bool },
    /// Someone goes by these points, round the walls and those standing there.
    Goes { who: Who, points: Vec<Point>, gait: Gait },
    /// Someone walks in from where the view does not go, or out of a puff
    /// of smoke where there is no way in; `placing`: who is there walks to it;
    /// `from`: in by that way, from off the place.
    Arrives { who: Who, to: Point, gait: Gait, placing: bool, from: Option<Exit> },
    /// Someone walks out of the place by that way, round what is in the way,
    /// in file behind who goes out there too, and is gone.
    Leaves { who: Who, by: Exit, run: bool },
    /// The ways in and out of the place, by name.
    Entrances(Vec<(String, Vec2)>),
    /// What is in the way of someone going round a spot is told.
    Round { who: Who, spot: Spot },
    /// All those on the stage look that way, or at someone.
    AllLook { way: Option<Direction>, at: Option<Who> },
    /// A crowd about that spot (none: the middle), in its form (a ring away
    /// from those who act, a flock, a row, a heap); `walking`: they walk in
    /// (from the side its form says, keeping their places in it).
    Crowd { who: Vec<Who>, spot: Option<Spot>, walking: bool, form: CrowdForm },
    /// So many on free ground about a spot, each given two numbers
    /// (`<var><n>_x`...); none: a place kept, with no one to put there.
    Gathers { var: String, spot: Spot, step: Vec2, who: Vec<Option<Who>> },
    /// A crowd running that far, staying on the ground.
    Advance { who: Vec<Who>, by: Vec2 },
    /// A crowd going together to a spot, each keeping its place in it (its
    /// middle there); `run`: running.
    Flock { who: Vec<Who>, to: Spot, run: bool },
    /// Each goes to a seat it can get to; the middle of the table in two
    /// numbers.
    Seats { var: String, who: Vec<Who>, seats: Vec<Vec2> },
    /// How close a shot comes, less if someone's body asks for it.
    Shot { close: f32, who: Option<Who> },
    /// Someone strolls, as wide as the ground leaves it.
    Roams { who: Who, wide: f32 },
    /// How wide someone stands on the ground (its shadow, as the game's
    /// tables give it): how far the others keep from it when placed.
    Footing { who: Who, wide: f32 },
}

fn point_of(fields: &Fields, at: usize) -> Option<Vec2> {
    Some(Vec2::new(fields.real(at)?, fields.real(at + 1)?))
}

/// Those from there up to that one (not it), each someone: none if it is
/// past the command's end.
fn group_of(fields: &Fields, from: usize, to: usize) -> Option<Vec<Who>> {
    (from <= fields.len()).then_some(())?;
    (from..to).map(|at| fields.who(at)).collect()
}

fn gait_of(fields: &Fields, at: usize) -> Option<Gait> {
    Gait::parse(fields.named(at, "walk or run")?, fields.named(at + 1, "a pace")?)
}

/// Points, each two numbers, from there up to that one (not it).
fn points_of(fields: &Fields, from: usize, to: usize) -> Option<Vec<Vec2>> {
    (from <= fields.len()).then_some(())?;
    (from..to).step_by(2).map(|at| (at + 1 < to).then(|| point_of(fields, at)).flatten()).collect()
}

/// A spot of the place (`w:12:3`, `a:m25`…) there.
fn spot_at(fields: &Fields, at: usize) -> Option<Spot> {
    fields.lookup(at, "a spot", Spot::parse)
}

declare! {
    PlaceBeat;
    hand: "advance", "alllook", "arrives", "crowd", "entrances", "find", "flies", "flock", "ghosts", "hovers", "lands", "gathers", "goes", "ground", "leaves", "roundcheck",
        "seats", "setspot", "spread", "swims";
    "shot" => Shot { close: f32, who: Opt<Who> },
    "roams" => Roams { who: Who, wide: f32 },
    "footing" => Footing { who: Who, wide: f32 },
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new("advance", &["dx:number dy:number who:code"], "a crowd runs, staying on the ground"),
    Form::new("alllook", &["whoside:word"], "everyone looks"),
    Form::new(
        "arrives",
        &["who:code walk_or_run:word pace:word x:coordinate y:coordinate [rest...]"],
        "comes in from where the view does not go, or in smoke if there is no way; `place`: whoever is there walks there; in a place that has entrances, by the nearest one on that side; `by:left`, `by:south`: by this side or this entrance, from outside the place, in a line behind whoever comes in that way",
    ),
    Form::new(
        "crowd",
        &["walk:word spot:word [rest...]"],
        "a crowd around this place: in a ring, away from those who act; in a flock or in a tight heap on free ground; in a line; `radius`: this wide; `from:right`: they all come in from that side of the view, each keeping their place, straight ahead",
    ),
    Form::new("entrances", &["name:word x:coordinate rest..."], "the place's entrances (`data/extra/decors.txt`): `film/` writes them on its own"),
    Form::new("find", &["var:word who:word spot:word [apart] [exact]"], "finds a place for who, in `{var_x}` and `{var_y}`"),
    Form::new("flies", &["who:code"], "those who fly in this scene: they take up no place"),
    Form::new("lands", &["who:code"], "those who flew or swam walk on the ground again: walls hold them"),
    Form::new("ghosts", &["who:code"], "those who pass through walls by nature: seen through while in one"),
    Form::new("hovers", &["who:code"], "those who hover by nature: a little bob in the air (2 pixels, a second), once on the stage"),
    Form::new("flock", &["spot:word walk_or_run:word who:code"], "a crowd goes to this place, its middle there, each keeping their place (straight ahead)"),
    Form::new("gathers", &["var:word spot:word step:word rest..."], "each on free ground around the place, their place in `{var<n>_x}`"),
    Form::new(
        "goes",
        &["who:code walk_or_run:word x05s2:word x:coordinate y:coordinate"],
        "goes through these points, going around walls and people; a pace: so many times a walker's, or so many seconds",
    ),
    Form::new("ground", &["place:word", "x:coordinate y:coordinate lines:word"], "the story carries on in a place where you walk"),
    Form::new(
        "leaves",
        &["who:code right_or_left_or_entrance:word [run]"],
        "(`run`: running, scared) leaves the place by the nearest entrance on that side (the place's, otherwise its ground nearest the edge) or by that one, going around what is in the way, in a line behind whoever leaves that way, and disappears once outside",
    ),
    Form::new("roams", &["who:code wide:word"], "they stroll around, as wide as the ground allows"),
    Form::new(
        "footing",
        &["who:code wide:word"],
        "how wide someone stands on the ground (its shadow): how far the others keep from it when placed, in a crowd (`film/` writes it on its own)",
    ),
    Form::new("roundcheck", &["who:code spot:word"], "says what is in the way of a round"),
    Form::new("seats", &["var:word who:code rest..."], "each goes to a seat they can reach; the middle of the table in `{var_x}`"),
    Form::new("setspot", &["rest..."], "names a place, found now"),
    Form::new("shot", &["zoom:number who:word"], "this zoom, less if the body needs it"),
    Form::new("spread", &["frame_or_place:word who:code"], "they move apart at the start of a scene, the view on them"),
    Form::new(
        "swims",
        &["pixels:number who:code"],
        "those who swim by nature: sunk so many pixels where no one walks (the water), not on the ground; a `sink` of theirs wins",
    ),
];

pub(super) fn read(fields: &Fields) -> Option<PlaceBeat> {
    if let Some(beat) = PlaceBeat::declared(fields) {
        return Some(beat);
    }
    let end = fields.len();
    Some(match fields.verb()? {
        "ground" => PlaceBeat::Ground { middle: point_of(fields, 1)?, ground: Ground::parse(fields.named(3, "a ground (0101/1100…)")?) },
        "flies" => PlaceBeat::Flies(group_of(fields, 1, end)?),
        "lands" => PlaceBeat::Lands(group_of(fields, 1, end)?),
        "ghosts" => PlaceBeat::Ghosts(group_of(fields, 1, end)?),
        "hovers" => PlaceBeat::Hovers(group_of(fields, 1, end)?),
        "swims" => PlaceBeat::Swims { depth: fields.real(1)?, who: group_of(fields, 2, end)? },
        "setspot" => {
            PlaceBeat::Name { name: fields.lookup(1, "a name (n:…)", |name| name.strip_prefix("n:").map(str::to_string))?, spot: spot_at(fields, 2)? }
        }
        "find" => PlaceBeat::Find {
            var: fields.named(1, "a name")?.to_string(),
            who: fields.who(2),
            spot: spot_at(fields, 3)?,
            apart: fields.find(4, "apart").is_some(),
            exact: fields.find(4, "exact").is_some(),
        },
        "spread" => PlaceBeat::Spread { who: group_of(fields, 2, end)?, picture: fields.named(1, "image or -")? == "image" },
        "goes" => PlaceBeat::Goes {
            who: fields.who(1)?,
            gait: gait_of(fields, 2)?,
            points: {
                (4 <= end).then_some(())?;
                (4..end)
                    .step_by(2)
                    .map(|at| Some(Point { x: fields.num(at)?, y: (at + 1 < end).then(|| fields.num(at + 1)).flatten()? }))
                    .collect::<Option<_>>()?
            },
        },
        "arrives" => PlaceBeat::Arrives {
            who: fields.who(1)?,
            gait: gait_of(fields, 2)?,
            to: fields.point(4)?,
            placing: fields.is(6, "place"),
            // (`via:<way>`: in by that way.)
            from: fields.word(7).and_then(|way| way.strip_prefix("via:")).map(Exit::parse),
        },
        // (`leaves <who> <way> [run]`: running, frightened.)
        "leaves" => PlaceBeat::Leaves { who: fields.who(1)?, by: Exit::parse(fields.named(2, "a way out")?), run: fields.is(3, "run") },
        // (`entrances <name> <x> <y>...`.)
        "entrances" => PlaceBeat::Entrances(
            (1..end)
                .step_by(3)
                .map(|at| {
                    Some((
                        fields.named(at, "a name")?.to_string(),
                        Vec2::new((at + 1 < end).then(|| fields.real(at + 1)).flatten()?, (at + 2 < end).then(|| fields.real(at + 2)).flatten()?),
                    ))
                })
                .collect::<Option<_>>()?,
        ),
        "roundcheck" => PlaceBeat::Round { who: fields.who(1)?, spot: spot_at(fields, 2)? },
        "alllook" => PlaceBeat::AllLook { way: fields.has(1).then(|| fields.word(1).and_then(Direction::of))?, at: fields.who(1) },
        // (`crowd walk|- <spot>|- [shape:flock radius:40 ...] <who>...`.)
        "crowd" => {
            let spot = match fields.named(2, "a spot or -")? {
                "-" => None,
                _ => Some(spot_at(fields, 2)?),
            };
            (3 <= end).then_some(())?;
            let named = (3..end).take_while(|at| fields.word(*at).is_some_and(|field| field.contains(':'))).count();
            let form = CrowdForm::read(&fields.from(3)?.before(named).words_from(0)?)?;
            PlaceBeat::Crowd { walking: fields.named(1, "walk or -")? == "walk", spot, form, who: group_of(fields, 3 + named, end)? }
        }
        "flock" => PlaceBeat::Flock { to: spot_at(fields, 1)?, run: fields.named(2, "walk or run")? == "run", who: group_of(fields, 3, end)? },
        "gathers" => PlaceBeat::Gathers {
            var: fields.named(1, "a name")?.to_string(),
            spot: spot_at(fields, 2)?,
            step: point_of(fields, 3)?,
            who: {
                (5 <= end).then_some(())?;
                (5..end).map(|at| fields.who(at)).collect()
            },
        },
        "advance" => PlaceBeat::Advance { by: point_of(fields, 1)?, who: group_of(fields, 3, end)? },
        "seats" => {
            let split = fields.find(0, ";")?;
            (2 <= split).then_some(())?;
            PlaceBeat::Seats { var: fields.named(1, "a name")?.to_string(), who: group_of(fields, 2, split)?, seats: points_of(fields, split + 1, end)? }
        }
        _ => return None,
    })
}
