//! The film's typed beats written as Explorers of Sky's operations. The
//! `Writer` takes the beats one by one (`beat`): first as the script's
//! course (`course`: labels, tests, variables, choices), then as what they
//! do (`plain`), each kind of beat in its file: `body` and `acting`,
//! `motion`, `place`, `view`, `screen`, `sound`, `talk`, `flow`. `fight`
//! stages a fight as its blows; `scene` puts each scene together, its
//! actors fitted to what Sky loads at once, its routines beside.

mod acting;
mod body;
mod course;
mod fight;
mod flow;
mod motion;
mod place;
mod scene;
mod screen;
mod sound;
mod talk;
#[cfg(test)]
mod tests_ds;
mod things;
mod view;

pub(crate) use fight::fights_played;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

use engine::{Beat, Cam, Course, Direction, FlowBeat, Formation, Num, Point, SpeechBeat, Spot, Who};

use ::film::ds::{Drawn, STAND_IN, drawn_on_ds};

use crate::notes::{Fate, Kind, Note};
use crate::sky::{FREE_ACTORS, Game, HIDDEN, animation, direction, own_group, speed, tiles};
use fight::Played;
use scene::{Capture, Part};
use things::{Carried, Look, Phase, Shown};

/// An actor of the scene: its name in the game, the species given to it
/// (a free test actor), its species, where it stands first, whether it
/// shows from the start.
pub(crate) struct Actor {
    name: String,
    species: Option<u16>,
    kind: u16,
    first: Option<(f32, f32, &'static str)>,
    hidden: bool,
    /// The layer of the scene's layout it is in: 0 from the start, a later
    /// one loaded when it comes in (past Sky's 21 at once).
    layer: usize,
}

pub(crate) struct Writer<'a> {
    game: &'a Game,
    body: String,
    pub(crate) actors: BTreeMap<Who, Actor>,
    /// The places the film found (`PlaceBeat::Find`), by their name.
    places: HashMap<String, (f32, f32)>,
    /// Where each one stands now, and where it faces.
    at: HashMap<Who, (f32, f32)>,
    facing: HashMap<Who, Direction>,
    pub(crate) level: Option<String>,
    top: Option<&'static str>,
    middle: Option<(f32, f32)>,
    /// Where the camera (performer 0) is.
    camera: (f32, f32),
    started: bool,
    faded_out: bool,
    marks: usize,
    talking: bool,
    /// The title, shown as Sky shows its story's title before the scene.
    title: Option<String>,
    /// Those whose emote still shows: the game loops some (joy, laughing) until told none.
    showing: Vec<String>,
    /// Those who played an animation once and stand again once the line is over.
    stand_after: Vec<String>,
    /// Those who were told to move and are not waited for yet.
    busy: Vec<String>,
    /// The line whose other beats Sky's own routine already plays.
    skip: Option<i32>,
    /// In a thread of `together`: the waits kept for its end.
    thread: bool,
    kept: Vec<String>,
    memory: bool,
    weather: bool,
    box_top: bool,
    /// Where the last line's box closed (to ask in it).
    closed: Option<usize>,
    /// The film's variables, as the game's.
    vars: Vec<String>,
    /// The film's actors, the same in every scene: name, species given, species.
    cast: BTreeMap<Who, (String, Option<u16>, u16)>,
    /// The scenes written before this one.
    blocks: Vec<String>,
    titled: bool,
    /// Who follows whom (`follow`): they walk after the leader's moves.
    followers: Vec<(Who, Who)>,
    /// The light the screen is tinted with (`screen_FlushOut`'s mode and colour).
    tint: Option<&'static str>,
    /// The choices open, and the answers each has had; the options of the last asked.
    menus: Vec<usize>,
    options: Vec<String>,
    indent: usize,
    /// After Sky's boss wipe: the whole screen white.
    white_all: bool,
    /// A caption said as narration, waited for by its key.
    said_over: bool,
    /// How long the emotes showing have been waited on.
    shown_for: f32,
    /// The place's things (the game's objects) and where each first stands.
    objects: BTreeMap<u16, (f32, f32)>,
    /// Who has become whom (`evolve`): the later lines' name, the new one's actor.
    became: HashMap<Who, Who>,
    /// The words each line of the film was given, by its number.
    pub(crate) words: HashMap<i32, Vec<String>>,
    /// What keeps the film from exporting (left out, `--partial`).
    pub(crate) problems: Vec<Note>,
    /// What the export changed to fit the DS, or left out, as it exports.
    pub(crate) notes: Vec<Note>,
    /// How it was made to fit, no loss (Sky's routines, the layout's layers): told, no note.
    pub(crate) info: Vec<String>,
    /// The line of the beat being written; the line the scene's place is on;
    /// every line that played.
    pub(crate) at_line: i32,
    pub(crate) scene_line: i32,
    pub(crate) lines_seen: BTreeSet<i32>,
    /// Those a frame of a few named (never left out).
    framed: Vec<Who>,
    /// `--tall`: the top screen shows the place above the bottom one, where it is tall enough.
    pub(crate) tall_asked: bool,
    /// This scene's place shows on both screens, one above the other.
    tall: bool,
    /// The place's size in pixels.
    size: Option<(f32, f32)>,
    /// The blocks played beside the script (`beside`, `float`), each
    /// actor's part of each: Sky's routines (`def n for actor`).
    besides: Vec<Vec<Part>>,
    /// A `beside` being written.
    capture: Option<Capture>,
    /// The film's threads written as such blocks, by name: their number.
    threads: HashMap<String, usize>,
    /// Those hovering (`float`).
    floating: Vec<String>,
    /// The fights as their rules played them, by their line.
    pub(crate) fights: Played,
    /// What a fight's moments say (its passages), by label; the one being written.
    passages: HashMap<String, String>,
    passage: Option<(String, usize)>,
    /// The film's numbers as last written (`{value}` in a line), and when
    /// lines beside it play (`at`), by name.
    values: HashMap<String, f64>,
    /// The poses a species' Sky sprite lacks, the one shown instead, by
    /// the lines they are on: told once each.
    pub(crate) instead_poses: BTreeMap<(String, String, &'static str), Vec<i32>>,
    /// The film's own names of its Pokémon, to tell who is drawn as whom.
    pub(crate) film_names: HashMap<Who, String>,
    /// The face of each line said in a bubble, by its line (the bubble's
    /// beat has none: read from the line's brackets).
    pub(crate) bubble_faces: HashMap<i32, u16>,
    /// The species SpriteCollab adds the film casts, put in the ROM from
    /// their sheets (`patch_rom.py`), and who each is drawn as where that
    /// fails (its base, or the stand-in).
    pub(crate) added: BTreeMap<u16, u16>,
    /// The things the film shows as objects (`things`): the object showing
    /// each now, how many of each look the scene has, the film's slot of the
    /// n-th of a look, the files of its pictures, the objects hidden from the
    /// start.
    things: BTreeMap<Who, Shown>,
    look_count: HashMap<Look, usize>,
    slots: BTreeMap<(Look, usize), u16>,
    drawings: HashMap<Who, String>,
    hidden_objects: Vec<u16>,
    /// Who or what is carried by whom (held, ridden, lifted).
    carried: Vec<Carried>,
    /// The things' paths, as their objects' routines, and the locks they took.
    phases: Vec<Phase>,
    path_locks: u16,
    /// The lines that played a thing as an object.
    pub(crate) item_lines: BTreeSet<i32>,
}

/// A frame of so many at most is of a few: they are never left out.
const FEW: usize = 6;

/// A pan's pace across a crowd too wide for the screen, pixels a frame.
const PAN: f32 = 0.75;

/// Actors at once in a scene: the game's 24, less the hero's, the
/// partner's and a third party member's, kept for them (overlay 11 looks
/// for a free one from the fourth: `ov11_022F7E78`, NA). Seen: 21 load.
const AT_ONCE: usize = ::film::ds::AT_ONCE;

/// The screen, pixels, and how far round someone's feet its sprite shows
/// (sideways, above, below).
const SCREEN: (f32, f32) = (256.0, 192.0);

const SPRITE: (f32, f32, f32) = (16.0, 32.0, 8.0);

/// Where a pan across those starts and ends, when they don't fit the
/// screen (none: they fit).
fn pan(points: &[(f32, f32)]) -> Option<((f32, f32), (f32, f32))> {
    let (left, right) = points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), point| (lo.min(point.0), hi.max(point.0)));
    let (top, bottom) = points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), point| (lo.min(point.1), hi.max(point.1)));
    let wide = right - left + 2.0 * SPRITE.0 > SCREEN.0;
    let high = bottom - top + SPRITE.1 + SPRITE.2 > SCREEN.1;
    if !wide && !high {
        return None;
    }
    let middle = ((left + right) / 2.0, (top + bottom) / 2.0);
    let from = (if wide { left - SPRITE.0 + SCREEN.0 / 2.0 } else { middle.0 }, if high { top - SPRITE.1 + SCREEN.1 / 2.0 } else { middle.1 });
    let to = (if wide { right + SPRITE.0 - SCREEN.0 / 2.0 } else { middle.0 }, if high { bottom + SPRITE.2 - SCREEN.1 / 2.0 } else { middle.1 });
    // (A few pixels out: their middle shows them well enough.)
    let across = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
    (across >= 24.0).then_some((from, to))
}

/// What an engine's beat is, in a few words (its kind's name, `View(Zoom
/// {..})` as "zoom"), and the kind of note it makes.
fn beat_said(beat: &Beat) -> (Kind, String) {
    let debug = format!("{beat:?}");
    let names: Vec<&str> =
        debug.split(|letter: char| !letter.is_alphanumeric()).filter(|name| name.starts_with(|letter: char| letter.is_ascii_uppercase())).take(2).collect();
    let kind = match names.first().copied() {
        Some("View") => Kind::Camera,
        Some("Screen") => Kind::Screen,
        Some("Sound") => Kind::Sound,
        Some("Speech") => Kind::Speech,
        _ => Kind::Engine,
    };
    let name = names.get(1).or(names.first()).copied().unwrap_or("step");
    let mut said = String::new();
    for letter in name.chars() {
        if letter.is_ascii_uppercase() && !said.is_empty() {
            said.push(' ');
        }
        said.push(letter.to_ascii_lowercase());
    }
    if said == "zoom" {
        return (Kind::Camera, "zoom (Sky has none)".to_string());
    }
    (kind, said)
}

/// The way from one point to another, of the eight.
fn way_to(from: (f32, f32), to: (f32, f32)) -> Direction {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    // Counter-clockwise from down, as the engine numbers them (y goes down).
    let angle = dx.atan2(dy).to_degrees();
    let eighth = ((angle + 360.0 + 22.5) / 45.0) as usize % 8;
    Direction::ALL[eighth]
}

/// Whether a script line names that actor (`ACTOR_X`, not `ACTOR_X2`).
fn names(line: &str, actor: &str) -> bool {
    line.match_indices(actor).any(|(at, _)| !line[at + actor.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
}

/// Beside someone, so far (the engine's own distance between two at rest).
const BESIDE: f32 = 24.0;

/// How far from the camera an actor is out of sight (half the screen and more).
const OFF_SCREEN: f32 = 150.0;

impl<'a> Writer<'a> {
    pub(crate) fn new(game: &'a Game) -> Self {
        Writer {
            game,
            body: String::new(),
            actors: BTreeMap::new(),
            places: HashMap::new(),
            at: HashMap::new(),
            facing: HashMap::new(),
            level: None,
            top: None,
            middle: None,
            camera: (128.0, 96.0),
            started: false,
            faded_out: false,
            marks: 0,
            talking: false,
            title: None,
            showing: Vec::new(),
            stand_after: Vec::new(),
            busy: Vec::new(),
            skip: None,
            thread: false,
            kept: Vec::new(),
            memory: false,
            weather: false,
            box_top: false,
            closed: None,
            vars: Vec::new(),
            menus: Vec::new(),
            followers: Vec::new(),
            cast: BTreeMap::new(),
            blocks: Vec::new(),
            titled: false,
            tint: None,
            options: Vec::new(),
            indent: 1,
            white_all: false,
            said_over: false,
            shown_for: 0.0,
            words: HashMap::new(),
            objects: BTreeMap::new(),
            became: HashMap::new(),
            problems: Vec::new(),
            notes: Vec::new(),
            info: Vec::new(),
            at_line: 0,
            scene_line: 0,
            lines_seen: BTreeSet::new(),
            framed: Vec::new(),
            tall_asked: false,
            tall: false,
            size: None,
            besides: Vec::new(),
            capture: None,
            threads: HashMap::new(),
            floating: Vec::new(),
            fights: HashMap::new(),
            passages: HashMap::new(),
            passage: None,
            values: HashMap::new(),
            instead_poses: BTreeMap::new(),
            added: BTreeMap::new(),
            film_names: HashMap::new(),
            bubble_faces: HashMap::new(),
            things: BTreeMap::new(),
            look_count: HashMap::new(),
            slots: BTreeMap::new(),
            drawings: HashMap::new(),
            hidden_objects: Vec::new(),
            carried: Vec::new(),
            phases: Vec::new(),
            path_locks: 0,
            item_lines: BTreeSet::new(),
        }
    }

    pub(crate) fn beat(&mut self, number: i32, word: Option<&str>, beat: &Beat) {
        self.at_line = number;
        if number > 0 {
            self.lines_seen.insert(number);
        }
        if let Beat::Flow(FlowBeat::Line { number: line, .. }) = beat {
            self.stand();
            // An emote shown long enough goes once its line is over (Sky loops some).
            if self.shown_for >= 30.0 {
                self.calm();
            }
            // (In a `beside`: where each of its lines starts.)
            let at = self.body.len();
            if let Some(capture) = self.capture.as_mut() {
                capture.lines.push(at);
            }
            if self.skip != Some(*line) {
                self.skip = None;
            }
            return;
        }
        if self.skip == Some(number) {
            return;
        }
        if self.talking && !matches!(beat, Beat::Speech(SpeechBeat::Said) | Beat::Flow(FlowBeat::Wait(_))) {
            self.talking = false;
        }
        if !matches!(beat, Beat::Course(Course::Ask(_)) | Beat::Flow(FlowBeat::Wait(_))) {
            self.closed = None;
        }
        if self.started && self.routine(number, word, beat) {
            self.skip = Some(number);
            return;
        }
        // (An item, a picture of the film's own: an object of the scene's.)
        if self.thing_beat(number, beat) {
            return;
        }
        self.course(number, word, beat);
    }

    /// A beat as what it does, each kind in its file: `body` (and `acting`),
    /// `motion`, `place`, `view`, `screen`, `sound`, `talk` (`speech`), `flow`.
    fn plain(&mut self, number: i32, word: Option<&str>, beat: &Beat) {
        match beat {
            Beat::Body(body) => self.body(number, word, body),
            Beat::Motion(motion) => self.motion(number, word, motion),
            Beat::Place(place) => self.place(number, word, place),
            Beat::View(view) => self.view(number, word, view),
            Beat::Screen(screen) => self.screen(number, word, screen),
            Beat::Sound(sound) => self.sound(number, word, sound),
            Beat::Speech(speech) => self.speech(number, word, speech),
            Beat::Flow(flow) => self.flow(number, word, flow),
            // A mount sunk under a heavy rider, and back up: not on DS (R-DS-LOSS).
            Beat::Node(engine::NodeBeat::Set { prop: engine::props::PropId::Height, .. })
                if matches!(word, Some("ride" | "get_off" | "dismount" | "carry" | "put_down")) => {}
            Beat::Course(Course::Host(fields)) | Beat::Host(fields) if fields.first().is_some_and(|first| first == "fight") => self.fight(number, fields),
            Beat::Course(Course::Say { entity, face, text, .. }) => self.say(number, word, *entity, *face, text),
            other => self.unexported(number, word, other),
        }
    }

    /// A beat the export has nothing for yet: an error (or, `--partial`, left out).
    fn unexported(&mut self, number: i32, word: Option<&str>, beat: &Beat) {
        let (kind, what) = beat_said(beat);
        let message = match word {
            Some(word) => format!("{word}: its {what} isn't played"),
            None => format!("the engine's {what} isn't played"),
        };
        self.problem(number, kind, Fate::LeftOut, message, "nothing");
    }

    /// Something changed to fit the DS, or left out, on that line.
    pub(crate) fn note(&mut self, line: i32, kind: Kind, fate: Fate, message: impl Into<String>, instead: impl Into<String>) {
        self.notes.push(Note::new(line, kind, fate, message, instead));
    }

    /// What keeps the film from exporting as it is (`--partial`: left out).
    pub(crate) fn problem(&mut self, line: i32, kind: Kind, fate: Fate, message: impl Into<String>, instead: impl Into<String>) {
        self.problems.push(Note::new(line, kind, fate, message, instead));
    }

    /// Sky's `SetAnimation` number for the engine's group on that one's
    /// own sprite (`own_group`).
    fn set(&self, who: Who, group: u16, looped: bool) -> Option<u16> {
        let who = self.became.get(&who).copied().unwrap_or(who);
        let species = self.actors.get(&who).map_or(0, |actor| actor.kind);
        animation(own_group(species, group)?, looped)
    }

    fn line(&mut self, text: &str) {
        // (One hovering told something else lands first: an order ends its routine where it is.)
        if self.capture.is_none() && !text.starts_with("MoveHeight") {
            let landing: Vec<String> = self.floating.iter().filter(|actor| text.contains(&format!("<actor {actor}>"))).cloned().collect();
            for actor in landing {
                self.floating.retain(|one| *one != actor);
                let _ = writeln!(self.body, "{}MoveHeight<actor {actor}>(1, 0);", "    ".repeat(self.indent));
                let _ = writeln!(self.body, "{}WaitExecuteLives({actor});", "    ".repeat(self.indent));
            }
        }
        let _ = writeln!(self.body, "{}{text}", "    ".repeat(self.indent));
    }

    /// Their emotes put away, once the line is said or before they move.
    fn calm(&mut self) {
        for actor in std::mem::take(&mut self.showing) {
            self.line(&format!("SetEffect<actor {actor}>(EFFECT_NONE, 3);"));
        }
    }

    /// Those who played an animation once stand again.
    fn stand(&mut self) {
        for actor in std::mem::take(&mut self.stand_after) {
            self.line(&format!("SetAnimation<actor {actor}>(2);"));
        }
    }

    fn position(&mut self, (x, y): (f32, f32)) -> String {
        self.marks += 1;
        format!("Position<'m{}', {}, {}>", self.marks, tiles(x), tiles(y))
    }

    /// Someone moving on alone, waited for later.
    fn moving(&mut self, actor: &str) {
        if !self.busy.iter().any(|one| one == actor) {
            self.busy.push(actor.to_string());
        }
    }

    /// The script waits for someone (in a thread of `together`: at its end).
    fn wait_for(&mut self, actor: &str) {
        if self.thread {
            if !self.kept.iter().any(|one| one == actor) {
                self.kept.push(actor.to_string());
            }
            return;
        }
        self.busy.retain(|one| one != actor);
        self.line(&format!("WaitExecuteLives({actor});"));
        self.wait_along(actor);
    }

    /// The game's actor for someone of the film: the hero and the partner
    /// as the game has them, a species by the game's own actor of it, else
    /// a free test actor given that species.
    fn actor(&mut self, who: Who) -> Option<String> {
        let who = self.became.get(&who).copied().unwrap_or(who);
        if let Some(actor) = self.actors.get(&who) {
            return Some(format!("ACTOR_{}", actor.name));
        }
        // (Past the free ones, a name `patch_rom.py --more-actors` gives a new actor.)
        let free = |cast: &BTreeMap<Who, (String, Option<u16>, u16)>| {
            let given = cast.values().filter(|one| one.1.is_some()).count();
            Some(FREE_ACTORS.get(given).map_or_else(|| format!("NPC_FREE{given:03}"), |name| name.to_string()))
        };
        let (name, species, kind) = match who {
            _ if self.cast.contains_key(&who) => self.cast[&who].clone(),
            Who::Character(number) => {
                let (name, kind) = self.game.actors.get(usize::from(number))?.clone();
                (name, None, kind)
            }
            Who::Monster(species) if self.drawn(who, species) == Drawn::Own => {
                let own = self.game.actors.iter().find(|(name, of)| *of == species && name.starts_with("NPC_") && !name.starts_with("NPC_TEST"));
                match own {
                    Some((name, _)) => (name.clone(), None, species),
                    None => (free(&self.cast)?, Some(species), species),
                }
            }
            // (A species the ROM has no sprite for: its base, or the stand-in, on a free actor of its own.)
            Who::Monster(species) | Who::Another { species, .. } => {
                let shown = self.drawn(who, species).species(species);
                (free(&self.cast)?, Some(shown), shown)
            }
            _ => return None,
        };
        self.cast.insert(who, (name.clone(), species, kind));
        // Who comes in after the scene opens is hidden until then.
        let hidden = self.started;
        self.actors.insert(who, Actor { name: name.clone(), species, kind, first: None, hidden, layer: 0 });
        Some(format!("ACTOR_{name}"))
    }

    /// How that one's species is drawn on the DS (`film::ds::drawn_on_ds`),
    /// told the first time when it isn't itself: never another Pokémon by
    /// chance, never in silence.
    fn drawn(&mut self, who: Who, species: u16) -> Drawn {
        let drawn = drawn_on_ds(&self.game.data, species);
        if let Drawn::Added(instead) = drawn {
            if self.added.insert(species, instead).is_none() {
                let name = self.film_names.get(&who).cloned().unwrap_or_else(|| self.game.name(species));
                let message = format!("{name} (species {species}): a Pokémon SpriteCollab adds");
                self.note(self.at_line, Kind::Added, Fate::Added, message, "put in the ROM from its sheets, its portraits with it");
            }
        } else if drawn != Drawn::Own {
            let name = self.film_names.get(&who).cloned().unwrap_or_else(|| self.game.name(species));
            let (message, instead) = match drawn {
                Drawn::Base(base) => (
                    format!("{name} (species {species}): a form SpriteCollab adds, with no sheets of its own in the data"),
                    format!("drawn as {}, its base species", self.game.name(base)),
                ),
                _ => (
                    format!("{name} (species {species}): a Pokémon Sky doesn't have, with no sheets in the data"),
                    format!("drawn as a stand-in, {} “?”, its name said in its lines", self.game.name(STAND_IN)),
                ),
            };
            if !self.notes.iter().any(|note| note.kind == Kind::Species && note.message == message) {
                self.note(self.at_line, Kind::Species, Fate::Changed, message, instead);
            }
        }
        drawn
    }

    /// Whether that one is drawn as the stand-in (no portrait of its own).
    fn stand_in(&self, who: Who) -> bool {
        matches!(who, Who::Monster(species) | Who::Another { species, .. } if drawn_on_ds(&self.game.data, species) == Drawn::StandIn)
    }

    fn who(&mut self, number: i32, who: Who) -> Option<String> {
        let actor = self.actor(who);
        if actor.is_none() {
            match who {
                Who::Item { .. } | Who::Thing { .. } => {
                    self.problem(number, Kind::Item, Fate::LeftOut, "the item isn't shown", "nothing: Sky's scenes show no items")
                }
                Who::Drawn { .. } => self.problem(number, Kind::Item, Fate::LeftOut, "a picture of the film's own isn't shown", "nothing"),
                _ => self.problem(
                    number,
                    Kind::Logic,
                    Fate::LeftOut,
                    format!("{who:?} has no actor in the game (only Pokémon, and {} new species at most)", FREE_ACTORS.len()),
                    "left out",
                ),
            }
        }
        actor
    }

    /// A number, written or found by the film before.
    fn num(&self, num: &Num) -> Option<f32> {
        match num {
            Num::Lit(value) => Some(*value),
            Num::Var(name) => {
                let (place, axis) = name.rsplit_once('_')?;
                let (x, y) = *self.places.get(place)?;
                match axis {
                    "x" => Some(x),
                    "y" => Some(y),
                    _ => None,
                }
            }
        }
    }

    fn point(&self, point: &Point) -> Option<(f32, f32)> {
        Some((self.num(&point.x)?, self.num(&point.y)?))
    }

    /// Where a spot is, as the export knows it before the game plays.
    fn spot(&self, spot: &Spot) -> Option<(f32, f32)> {
        match spot {
            Spot::Written(point) | Spot::Word(point) | Spot::Exact(point) | Spot::Near(point) => self.point(point),
            Spot::Of(other) => self.at.get(other).copied(),
            Spot::Side { of, right } => {
                let (x, y) = *self.at.get(of)?;
                Some((if *right { x + BESIDE } else { x - BESIDE }, y))
            }
            Spot::Beside(of) => {
                let (x, y) = *self.at.get(of)?;
                Some((x - BESIDE, y))
            }
            // Before someone as it looks, or behind it (a dodge back).
            Spot::Before { of, back } => {
                let (x, y) = *self.at.get(of)?;
                let (dx, dy) = match self.facing.get(of).copied().unwrap_or(Direction::Down) {
                    Direction::Down => (0.0, 1.0),
                    Direction::DownRight => (0.7, 0.7),
                    Direction::Right => (1.0, 0.0),
                    Direction::UpRight => (0.7, -0.7),
                    Direction::Up => (0.0, -1.0),
                    Direction::UpLeft => (-0.7, -0.7),
                    Direction::Left => (-1.0, 0.0),
                    Direction::DownLeft => (-0.7, 0.7),
                };
                let way = if *back { -BESIDE } else { BESIDE };
                Some((x + dx * way, y + dy * way))
            }
            Spot::By { of, dx, dy } => {
                let (x, y) = *self.at.get(of)?;
                Some((x + dx, y + dy))
            }
            Spot::Tiles { of, count } => {
                let (x, y) = *self.at.get(of)?;
                Some((x + count * 8.0, y))
            }
            Spot::Between(a, b) => {
                let (a, b) = (*self.at.get(a)?, *self.at.get(b)?);
                Some(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0))
            }
            // A formation round a point, spaced as the engine spaces two at rest.
            Spot::Shape { kind, who, at, round } => {
                let (x, y) = self.spot(round)?;
                let many = who.len() as f32;
                let along = (*at as f32 - (many - 1.0) / 2.0) * BESIDE;
                Some(match kind {
                    Formation::Line | Formation::Facing => (x + along, y),
                    Formation::Column => (x, y + along),
                    Formation::Triangle => [(x, y - 16.0), (x - BESIDE, y + 12.0), (x + BESIDE, y + 12.0)].get(*at).copied()?,
                    Formation::Ring => {
                        let radius = (many * BESIDE / std::f32::consts::TAU).max(BESIDE);
                        let angle = std::f32::consts::TAU * *at as f32 / many;
                        (x + angle.sin() * radius, y - angle.cos() * radius)
                    }
                })
            }
            _ => None,
        }
    }

    /// Where the camera looks at, from the film's.
    fn cam(&self, cam: &Cam) -> Option<(f32, f32)> {
        match cam {
            Cam::At(point) => self.point(point),
            Cam::Who(who) => self.at.get(who).copied(),
        }
    }

    /// The camera (performer 0) taken there, in so many frames (none: at once).
    fn camera_to(&mut self, to: (f32, f32), frames: f32) {
        let distance = ((to.0 - self.camera.0).powi(2) + (to.1 - self.camera.1).powi(2)).sqrt();
        self.camera = to;
        let position = self.position(to);
        let above = self.tall.then(|| self.above(to));
        let above = above.map(|above| self.position(above));
        if frames <= 1.0 || distance < 2.0 || !self.started {
            self.line(&format!("SetPositionMark<performer 0>({position});"));
            if let Some(above) = above {
                self.line(&format!("camera2_SetPositionMark({above});"));
            }
        } else {
            let pace = speed(distance, frames);
            self.line(&format!("MovePositionMark<performer 0>({pace}, {position});"));
            // (The top screen's camera glides with it, as Sky's tall views do: S13P04's.)
            if let Some(above) = above {
                self.line(&format!("camera2_MovePositionMark({pace}, {above});"));
            }
        }
    }

    /// Where the top screen's camera looks for the bottom one's there, in
    /// the tall view: a screen above, both kept in the place (the game
    /// stops a camera at its edges; where the bottom one is too high for a
    /// screen above, the two show the same rows).
    fn above(&self, (x, y): (f32, f32)) -> (f32, f32) {
        let (wide, high) = self.size.unwrap_or((256.0, 384.0));
        let x = x.clamp(128.0, (wide - 128.0).max(128.0));
        let y = y.clamp(96.0, (high - 96.0).max(96.0));
        (x, (y - 192.0).max(96.0))
    }

    /// Someone walks (`MovePositionMark`, turned the way it goes) or
    /// slides (`SlidePositionMark`, facing as it was) there, at that speed.
    fn go(&mut self, who: Who, actor: &str, to: (f32, f32), pace: &str, slide: bool) {
        let from = self.at.get(&who).copied();
        self.at.insert(who, to);
        if let (Some(from), false) = (from, slide) {
            self.facing.insert(who, way_to(from, to));
        }
        let position = self.position(to);
        let verb = if slide { "SlidePositionMark" } else { "MovePositionMark" };
        self.line(&format!("{verb}<actor {actor}>({pace}, {position});"));
        self.moving(actor);
        self.carry_along(who, to, pace);
    }

    /// Someone shows there: put, and no longer hidden.
    fn show_at(&mut self, who: Who, actor: &str, at: (f32, f32)) {
        let first = self.actors.get(&who).is_some_and(|one| one.hidden && one.first.is_none());
        if first {
            if let Some(one) = self.actors.get_mut(&who) {
                let facing = self.facing.get(&who).copied().unwrap_or(Direction::Down);
                one.first = Some((at.0, at.1, direction(facing)));
            }
        } else if self.at.get(&who) != Some(&at) {
            let position = self.position(at);
            self.line(&format!("SetPositionMark<actor {actor}>({position});"));
            self.set_along(who, at);
        }
        self.at.insert(who, at);
        self.line(&format!("ResetOutputAttribute<actor {actor}>({HIDDEN});"));
    }

    /// Whether the line's word was given that word (`hop surprised`).
    fn said_with(&self, number: i32, said: &str) -> bool {
        self.words.get(&number).is_some_and(|words| words.iter().any(|word| word == said))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ways_are_the_engines_eight() {
        assert_eq!(way_to((0.0, 0.0), (10.0, 0.0)), Direction::Right);
        assert_eq!(way_to((0.0, 0.0), (0.0, 10.0)), Direction::Down);
        assert_eq!(way_to((0.0, 0.0), (-10.0, -10.0)), Direction::UpLeft);
    }

    #[test]
    fn a_crowd_wider_than_the_screen_is_panned_across() {
        assert_eq!(pan(&[(100.0, 100.0), (300.0, 100.0)]), None);
        let (from, to) = pan(&[(100.0, 100.0), (400.0, 110.0)]).expect("too wide");
        assert_eq!((from.0, to.0), (100.0 - 16.0 + 128.0, 400.0 + 16.0 - 128.0));
        assert_eq!(from.1, to.1);
    }

    #[test]
    fn a_line_names_an_actor_not_a_longer_one() {
        assert!(names("Destroy<actor ACTOR_FREE1>();", "ACTOR_FREE1"));
        assert!(!names("Destroy<actor ACTOR_FREE12>();", "ACTOR_FREE1"));
    }
}
