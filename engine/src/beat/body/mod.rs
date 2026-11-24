//! Who stands on the stage and how it shows.

use super::*;

mod read;

pub(super) use read::{FORMS, WORDS, read};

/// Who stands on the stage and how it shows.
#[derive(Clone, Debug, PartialEq)]
pub enum BodyBeat {
    Put {
        who: Who,
        at: Point,
        direction: Direction,
    },
    /// Someone put beside someone else (on that side),
    /// turned to them.
    PutNear {
        who: Who,
        other: Who,
        side: Direction,
    },
    Gone(Who),
    Turn {
        who: Who,
        direction: Direction,
    },
    /// Someone plays one of its animations, so (see `Play`).
    Anim {
        who: Who,
        anim: AnimId,
        play: Play,
    },
    /// Someone takes a pose of its own: asleep, hurt, attacking, standing.
    Pose {
        who: Who,
        pose: Pose,
    },
    Rise {
        who: Who,
        height: f32,
        frames: Frames,
    },
    Attr {
        who: Who,
        bits: u16,
    },
    /// Someone drawn under everyone, among the others, or over everyone.
    Layer {
        who: Who,
        place: Stacked,
    },
    /// Someone showing and hiding in turn, or no longer.
    Blink {
        who: Who,
        on: bool,
    },
    /// What a move shows on what it strikes, on someone, with no blow.
    MoveEffect {
        who: Who,
        id: MoveId,
    },
    /// What a move throws, from someone to someone else, in so many frames,
    /// with no blow (none: at the move's own speed).
    MoveShot {
        from: Who,
        to: Who,
        id: MoveId,
        frames: Option<Frames>,
    },
    /// Someone trembles for that many frames, that many pixels to and fro.
    Tremble {
        who: Who,
        frames: Frames,
        strength: f32,
    },
    /// How fast someone's own time goes (1: as the scene's; 0.3: slowed
    /// down): its moves, its pictures, what shows on it; the rest of the
    /// scene as it goes. It multiplies the scene's (`ViewBeat::Tempo`).
    Slow {
        who: Who,
        by: f32,
    },
    /// Someone stays as it is, in the middle of a step (or goes on).
    Freeze {
        who: Who,
        on: bool,
    },
    /// Someone is drawn in front of another, wherever they stand (none: as
    /// where it stands has it).
    InFront {
        who: Who,
        of: Option<Who>,
    },
    /// Someone jumps on the spot, that high, in that many frames.
    Jump {
        who: Who,
        height: f32,
        frames: Frames,
    },
    /// Someone jumps to a place in an arc that high.
    Hop {
        who: Who,
        at: Point,
        height: f32,
        frames: Frames,
    },
    /// Someone spins round that many times: by the eight ways it faces,
    /// or (`rotate`) its very picture turning.
    Spin {
        who: Who,
        turns: f32,
        frames: Frames,
        rotate: bool,
    },
    /// Someone shows that much (1 wholly, 0 not at all), in that many
    /// frames: appearing, vanishing.
    Opacity {
        who: Who,
        to: f32,
        frames: Frames,
    },
    /// Someone grows or shrinks to that size (1: its own), in that many
    /// frames.
    Scale {
        who: Who,
        to: f32,
        frames: Frames,
    },
    /// Someone casts its shadow, or not.
    Shadow {
        who: Who,
        on: bool,
    },
    /// Someone's picture drawn the other way round, left to right, or not.
    Flip {
        who: Who,
        on: bool,
    },
    /// Someone sunk in the water so many pixels: drawn that much lower, what
    /// is under the surface hidden (0: out of it); in sand, if `sand`.
    Sink {
        who: Who,
        depth: f32,
        sand: bool,
    },
    /// Someone as it was put: its own size, colours, all of it showing,
    /// among the others, with its shadow, no name, doing nothing more.
    Reset(Who),
    /// A name shown under someone's feet (none: gone).
    /// A little bar of life over someone: how much of it is left; none: gone.
    Bar {
        who: Who,
        share: Option<f32>,
    },
    /// Someone looks about now and then, as it stands (or stops).
    Fidget {
        who: Who,
        on: bool,
    },
    /// Someone hops on the spot again and again, that high, a hop taking
    /// that many frames (0 high: stops).
    Bounce {
        who: Who,
        height: f32,
        frames: Frames,
    },
    /// A dungeon's status icon over someone (see `status_icon`), in the
    /// place of those it had, or with them (`add`); none: takes them off.
    Status {
        who: Who,
        icon: Option<&'static str>,
        add: bool,
    },
    /// Something carried by someone, wherever it goes: over its head, or
    /// in its hands before it (none: put down where it is).
    Carry {
        what: Who,
        by: Option<Who>,
        hands: bool,
    },
    /// Someone on another's back, wherever it goes, up and down with it:
    /// that high over its feet, that far to its side (none: left where it is).
    Ride {
        who: Who,
        on: Option<Who>,
        high: f32,
        beside: f32,
    },
    /// A thing (an item) handed from someone to someone else, in an arc.
    /// It leaves the stage there, or (`keeps`) shows over the giver first
    /// and stays in the other's hands.
    Give {
        what: Who,
        from: Who,
        to: Who,
        frames: Frames,
        keeps: bool,
    },
    /// What a thing is called, as it is told (see `stage::Thing`).
    ThingName {
        who: Who,
        name: String,
    },
    /// The file of a picture of a film's own: its name, how many pictures
    /// are side by side in it, the 60ths of a second each shows when it
    /// plays (none: the usual pace, the first picture kept for standing).
    /// For who shows the stage.
    Drawing {
        who: Who,
        file: String,
        cells: u16,
        ticks: Vec<u32>,
    },
    /// A character of a film's own drawn from a sprite sheet (`character`):
    /// its file, the size of a cell, how many ways it faces (1, 4 or 8
    /// rows), its animations. For who shows the stage.
    Sheet {
        who: Who,
        file: String,
        cell: (u16, u16),
        ways: u8,
        anims: Vec<SheetAnim>,
    },
    /// Whether a thing can be picked up.
    Takeable {
        who: Who,
        on: bool,
    },
    /// Whether a thing glints.
    Shine {
        who: Who,
        on: bool,
    },
    /// A thing shown large over the scene (none: no longer).
    Large(Option<Who>),
    /// A fishing line from someone's rod to something (none: no line), thrown
    /// to a point if one is told (see `Stage::lines`); or only tied to what is
    /// already at its end, where it is (a catch, `fishline a b holds`).
    FishLine {
        who: Who,
        to: Option<Who>,
        at: Option<Vec2>,
        cast: bool,
    },
    /// Someone thrown in an arc to where another stands now.
    Throw {
        who: Who,
        at: Who,
        height: f32,
        frames: Frames,
    },
    /// Someone falling from that high to the ground, over so many frames.
    Fall {
        who: Who,
        height: f32,
        frames: Frames,
    },
    /// Someone floats up and down, that high, a swing taking that many
    /// frames (0 high: stops).
    Float {
        who: Who,
        height: f32,
        frames: Frames,
    },
    /// What someone's picture is multiplied by (red, green, blue, of 255:
    /// 255 255 255 as it is, 0 0 0 a silhouette), in that many frames.
    Colour {
        who: Who,
        colour: [f32; 3],
        frames: Frames,
    },
    /// Someone takes a blow from someone else: thrown back a little, hurt,
    /// trembling, turned to who struck.
    Recoil {
        who: Who,
        from: Who,
        distance: f32,
    },
    /// Someone turns to look at a point.
    LookAt {
        who: Who,
        at: Point,
    },
    /// Someone turns to face someone else.
    Face {
        who: Who,
        other: Who,
    },
    /// Someone turns round, its back to where it looked.
    TurnAround(Who),
    /// Someone turns so many eighths from where it looks, at once:
    /// counter-clockwise as the directions go, clockwise below 0.
    TurnBy {
        who: Who,
        eighths: f32,
    },
    /// Someone's next turn goes the slow way, as the game's slow turns: it
    /// goes through each way in between, one every so many frames.
    TurnSlow {
        who: Who,
        every: Frames,
    },
    /// An effect on someone (0: none), for that many frames when given,
    /// in the place of those it showed, or with them (`add`).
    Effect {
        who: Who,
        id: EmoteId,
        frames: Option<Frames>,
        add: bool,
    },
    /// Someone shown as another: its picture, its name and its face in
    /// what it says (none: itself again).
    Disguise {
        who: Who,
        like: Option<Who>,
    },
    /// Someone shown with another's picture, still itself in what it says
    /// (none: its own picture again).
    Mimic {
        who: Who,
        like: Option<Who>,
    },
    /// A few words rising from someone and fading: a blow's number, a
    /// status taken.
    Popup {
        who: Who,
        text: String,
        colour: [u8; 3],
    },
    /// A move used by someone, on someone else or no one.
    Move {
        who: Who,
        id: MoveId,
        target: Option<Who>,
    },
    /// The only ways someone is painted in (a variant's): it faces the first
    /// again once it stands, unless a line turns it (none: all of them).
    Views {
        who: Who,
        ways: Vec<Direction>,
    },
    /// Someone stops whatever it does: walking, going round, wandering,
    /// following, watching, floating, spinning, hopping, looking about.
    Stop(Who),
}

/// Where someone is drawn among the others (`layer`): under everyone, among
/// them as they stand, or over everyone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stacked {
    Under,
    Among,
    Over,
}

/// How someone plays an animation (`BodyBeat::Anim`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Once, then it stays on its last picture (with no such animation: as
    /// it is).
    Once,
    /// Again and again (with no such animation: it stands).
    Loop,
    /// So many times, then it stands again.
    Times(u16),
    /// That step of it, and it stays on it (0: its first).
    Hold(usize),
    /// On its first picture, not moving; with no such animation, it stands
    /// (`or_stand`) or stays as it is.
    Still { or_stand: bool },
}

impl Play {
    /// As the game's two flags have it: looped, frozen.
    pub fn of(looped: bool, frozen: bool) -> Play {
        match (looped, frozen) {
            (false, false) => Play::Once,
            (true, false) => Play::Loop,
            (or_stand, true) => Play::Still { or_stand },
        }
    }
}

/// An animation of a character's sprite sheet (`BodyBeat::Sheet`): which
/// of its sprite's animations it is (0 walk, 1 idle, 2 hurt, 3 sleep, 4
/// attack, 5 + n its own group n), its cells along a row (from the left),
/// the 60ths of a second each shows (empty: 8 each; fewer than the cells:
/// the last kept), the row of the sheet its first way is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetAnim {
    pub animation: u16,
    pub cells: Vec<u16>,
    pub ticks: Vec<u32>,
    pub row: u16,
}

impl SheetAnim {
    /// As a line of the engine writes it: `animation:cells:ticks:row`,
    /// the cells and ticks with commas (`-`: no ticks).
    pub fn written(&self) -> String {
        let list = |all: Vec<String>| if all.is_empty() { "-".to_string() } else { all.join(",") };
        format!(
            "{}:{}:{}:{}",
            self.animation,
            list(self.cells.iter().map(u16::to_string).collect()),
            list(self.ticks.iter().map(u32::to_string).collect()),
            self.row
        )
    }

    /// One read back from [`SheetAnim::written`].
    pub fn read(field: &str) -> Option<SheetAnim> {
        let parts: Vec<&str> = field.split(':').collect();
        let [animation, cells, ticks, row] = parts.as_slice() else { return None };
        let list = |field: &str| -> Option<Vec<u32>> {
            match field {
                "-" => Some(Vec::new()),
                field => field.split(',').map(|one| one.parse().ok()).collect(),
            }
        };
        let cells: Vec<u16> = list(cells)?.into_iter().map(u16::try_from).collect::<Result<_, _>>().ok()?;
        let ticks = list(ticks)?;
        (!cells.is_empty() && ticks.iter().all(|tick| *tick > 0)).then_some(())?;
        Some(SheetAnim { animation: animation.parse().ok()?, cells, ticks, row: row.parse().ok()? })
    }
}
