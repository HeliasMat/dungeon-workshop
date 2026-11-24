//! A film being written: its words so far, and what the words alone say. Who
//! stands where is not here: the stage knows it when the film plays.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use engine::{Beat, Diagnostic, Ground};

use crate::tables::Tables;

/// Fields for a line of the engine, of anything written.
#[macro_export]
macro_rules! fields {
    ($($field:expr),* $(,)?) => { vec![$($field.to_string()),*] };
}

/// What a word hands the engine: a beat, typed; or a line of fields no beat
/// reads (a spot `@` by its name, `define`/`do`…), told as wrong once the
/// film is compiled.
#[derive(Clone, Debug, PartialEq)]
pub enum Sortie {
    Beat(Beat),
    /// A scene of the film opens: in that place of the game, by that name.
    Scene {
        level: u16,
        name: String,
    },
    Champs(Vec<String>),
}

impl Sortie {
    /// Its line of fields, as the engine's passes read it.
    pub fn fields(&self) -> Vec<String> {
        match self {
            Sortie::Beat(beat) => beat.fields(&scenes::vocabulary::POKEMON).unwrap_or_else(|| unreachable!("a beat made by the film has its line: {beat:?}")),
            Sortie::Scene { level, name } => vec!["scene".to_string(), level.to_string(), name.clone()],
            Sortie::Champs(fields) => fields.clone(),
        }
    }

    /// Its engine's word (`walk`, `fight`, `label`…).
    pub fn verb(&self) -> String {
        match self {
            Sortie::Champs(fields) => fields[0].clone(),
            beat => beat.fields().swap_remove(0),
        }
    }
}

/// What a thing is: an item, a prop of the game, a picture of the film's own
/// (its file, how many pictures in it).
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Item(String),
    Prop(u16),
    Drawn(String, u16),
}

/// A thing of the scene at hand: what kind by its word (`key`), the word for
/// this one, who it is on the stage, who has it.
#[derive(Clone, Debug)]
pub struct Stuff {
    pub key: String,
    pub word: String,
    pub code: String,
    pub owner: Option<String>,
    pub takeable: bool,
}

/// A thing in the air: the next line says how it lands.
pub struct Flying {
    pub code: String,
    pub target: String,
    pub wait: bool,
    pub thrower: String,
}

/// What a family keeps from one line to the next, of its own type, in the
/// film (`Film::kept`): the film knows none of them, only that each may
/// change at a new scene and may show a log.
pub trait Kept: std::any::Any {
    /// A new scene opens: what goes with the last one goes; a beat the new
    /// scene opens with, if any.
    fn new_scene(&mut self) -> Option<Beat> {
        None
    }

    /// Whether the lines said now go to a log on the screen (a fight's, a
    /// dungeon floor's), not to a box.
    fn logs(&self) -> bool {
        false
    }
}

/// What the stage knows of the scene at hand, gone at the next one: who
/// came on by a line, a vision, who evolved or was renamed. (Each family
/// keeps its own: `fishing`, `dungeon`, `speaking`, `things`.)
#[derive(Default)]
pub struct Staging {
    pub started: bool,
    pub vision: bool,
    /// Who a line of the scene brings on (not there from its start); those
    /// of them a vision or a memory brought, gone when it ends.
    pub brought: Vec<String>,
    pub dreamt: Vec<String>,
    /// (Who has evolved: from what, and what it is called now.)
    pub evolved: BTreeMap<String, String>,
    pub renamed: HashMap<String, String>,
    /// The nodes of the film's own (`node`), by name, and their kind.
    pub nodes: BTreeMap<String, engine::props::NodeKind>,
    /// Who hangs on whom (`attach`): each node's parents, the nearest
    /// last, for a loop to be told before it plays.
    pub parents: BTreeMap<String, Vec<String>>,
    /// The light of the hour the scene said (`daytime evening`): what a
    /// weather gone or a memory ended comes back to. None: by day.
    pub daylight: Option<[f32; 4]>,
    /// Who the scene's head puts facing someone (`a put left facing b`):
    /// turned once all of it is put, whatever the order of its lines.
    pub facing_later: Vec<(String, String)>,
    /// Who is in the water (`swim`), and who is under it (`dive`); who
    /// flies, how high (`fly`).
    pub swimming: BTreeSet<String>,
    pub under: BTreeSet<String>,
    pub flying: BTreeMap<String, f32>,
    /// Who hides (`hide`), and who is under the ground (`burrow`).
    pub hidden: BTreeSet<String>,
    pub buried: BTreeSet<String>,
    /// Who rides on whose back (`ride`); how many pixels each mount has
    /// sunk under its riders (by weight).
    pub mounts: BTreeMap<String, String>,
    pub sunk: BTreeMap<String, f32>,
    /// Who shows an aura of his type (`aura on`): the game's type.
    pub auras: BTreeMap<String, u8>,
}

pub struct Film {
    /// The words it is read with: every family of the language.
    pub lexicon: &'static crate::verbs::Lexicon,
    pub tables: Arc<Tables>,
    /// Those known by a name, in the order they came; their codes.
    pub names: Vec<String>,
    pub actors: HashMap<String, String>,
    pub problems: Vec<Diagnostic>,
    /// The film's own text; the line at hand, its words each where it is
    /// written there (`Film::written_words`), for a problem to say which.
    pub source: Arc<str>,
    pub here: (usize, Vec<(String, language::Span)>),
    pub title: String,
    pub groups: BTreeMap<String, Vec<String>>,
    /// The ground of the scene at hand, and the point it is measured from.
    pub rows: Option<Ground>,
    pub middle: (i32, i32),
    pub level: u16,
    /// What the film knows of the scene at hand, nothing at each new one.
    pub staging: Staging,
    /// What a line said keeps: the faces said, who spoke last (`compile/said.rs`).
    pub speaking: Speaking,
    /// The things of the scene at hand, how many of each (`groups/things.rs`).
    pub things: ThingsState,
    /// The film's time: threads, marks, what the scene waits for, what is open (`groups/timing.rs`).
    pub timing: TimingState,
    /// The film's plan: a line each, its thread, placing and time (`time::plan`).
    pub plan: crate::time::Plan,

    /// A film one plays: who the player walks (`groups/game.rs`).
    pub game: Game,
    /// What each family keeps of its own (`Kept`), by its type: fishing's,
    /// a dungeon's, a fight's.
    kept: BTreeMap<std::any::TypeId, Box<dyn Kept>>,
    pub flying: Option<Flying>,
    /// The pictures the film calls its own, by name; their numbers, by file;
    /// the words that are things, not someone.
    pub own: BTreeMap<String, (String, u16)>,
    pub pictures: Vec<String>,
    /// The 60ths of a second of each picture of a file of ours, if the film says.
    pub rhythms: BTreeMap<String, Vec<u32>>,
    pub thing_words: HashSet<String>,
    /// The paths of the film's own (`route`), by name: their points and
    /// the way they go along them.
    pub paths: BTreeMap<String, (Vec<glam::Vec2>, engine::PathShape)>,
    pub acting: bool,
    /// `collisions on` before the first scene: each of its scenes opens
    /// with them (a film's are without).
    pub collisions: bool,
    pub clouds: i32,
    /// (The same film, the same fight.)
    pub chance: u64,
    pub seeing: bool,
    /// The last point a line was given, for `there`, and the last team one
    /// named, for `team` (`text/recipes.rs`): one line goes on where the one
    /// before left off.
    pub last_spot: Option<(i32, i32)>,
    pub last_team: Vec<String>,
    /// How much longer (more than 1) or shorter each `wait 2s` of the film
    /// is, from `pace` on (only those).
    pub pace: f32,
    /// The film's spots, by name; those before any scene last.
    pub spots: HashSet<String>,
    pub lasting_spots: Vec<(String, i32, i32)>,
    /// The ways in and out of the place at hand, by name.
    pub entrances: Vec<String>,
    /// What is written so far, each with the film's line.
    pub written: Vec<(usize, Sortie)>,
    /// The box put somewhere by the scene (`box top|bottom`): where it
    /// hides no one again at the next.
    pub box_kept: bool,
    /// How many places were found, unseen points effects were put on,
    /// passages written; those the scene waits for before it closes.
    pub found: usize,
    /// The numbers words keep for a moment as the scene plays, each its own.
    pub temps: crate::temps::Temps,
    /// The files taken in (`include`): each the line that takes it, its name.
    /// Their lines are numbered `INCLUDED × (rank + 1) + their own line`
    /// while the film is read, then told as the line that takes them in.
    pub included: Vec<(usize, String)>,
    /// The places given to recipes in two numbers, by line (`tornado a 300
    /// 160`): told if the place has a name for them (`verbs::named_near`).
    pub points_said: HashMap<usize, Vec<(i32, i32)>>,
    /// The moment of the scene at hand (`moment: the gust`), and whether its
    /// view was framed already.
    pub moment: Option<(String, bool)>,
    /// Its moments and scenes as the story family goes to them and counts
    /// them (`go moment`, `seen(name)`): `crate::plot`.
    pub story: crate::plot::StoryMap,
    /// The libraries it takes in (`uses fishing`): an optional family's words with them.
    pub uses: Vec<String>,
    /// What it is written for (`for ds`): told what the DS can't play.
    pub for_ds: Option<crate::ds::Mode>,
    /// What the film read a line as, to be told when asked (`scene_player check --all`):
    /// what `there` and `team` stood for.
    pub notes: Vec<(usize, String)>,
    /// How many recipes were called: each call's own numbers (`$px`).
    pub recipe_calls: usize,
    /// The recipes the film writes and uses, each once: checked alone
    /// once the film is read (`text::checked`).
    pub defined: Vec<crate::verbs::Defined>,
    /// The line at hand.
    pub number: usize,
    /// What each of the text's own lines was read as.
    pub read_as: std::collections::BTreeMap<usize, ReadAs>,
}

impl Film {
    /// A film compiled with these tables.
    pub fn with(lexicon: &'static crate::verbs::Lexicon, tables: Arc<Tables>) -> Self {
        Film {
            lexicon,
            tables,
            names: vec!["hero".to_string(), "partner".to_string()],
            actors: HashMap::from([("hero".to_string(), "0".to_string()), ("partner".to_string(), "10".to_string())]),
            problems: Vec::new(),
            source: Arc::from(""),
            here: (0, Vec::new()),
            title: String::new(),
            groups: BTreeMap::new(),
            rows: None,
            middle: (0, 0),
            level: 0,
            staging: Staging::default(),
            speaking: Default::default(),
            things: Default::default(),
            timing: Default::default(),
            plan: Default::default(),
            game: Default::default(),
            kept: BTreeMap::new(),
            flying: None,
            own: BTreeMap::new(),
            pictures: Vec::new(),
            rhythms: BTreeMap::new(),
            thing_words: HashSet::new(),
            paths: BTreeMap::new(),
            acting: true,
            collisions: false,
            clouds: 8,
            chance: 7,
            seeing: false,
            last_spot: None,
            last_team: Vec::new(),
            pace: 1.0,
            spots: HashSet::new(),
            lasting_spots: Vec::new(),
            entrances: Vec::new(),
            written: Vec::new(),
            box_kept: false,
            found: 0,
            temps: crate::temps::Temps::default(),
            included: Vec::new(),
            points_said: HashMap::new(),
            moment: None,
            story: Default::default(),
            uses: Vec::new(),
            for_ds: None,
            notes: Vec::new(),
            recipe_calls: 0,
            defined: Vec::new(),
            read_as: Default::default(),
            number: 0,
        }
    }

    /// A number of its own for a place found when the film plays.
    pub fn found_var(&mut self) -> String {
        self.found += 1;
        format!("_place{}", self.found)
    }

    /// What a family keeps of its own type, made the first time it is asked.
    pub fn kept<T: Kept + Default>(&mut self) -> &mut T {
        let kept = self.kept.entry(std::any::TypeId::of::<T>()).or_insert_with(|| Box::new(T::default()));
        (&mut **kept as &mut dyn std::any::Any).downcast_mut().expect("kept by its type")
    }

    /// What a family keeps of its own type, if it kept anything yet.
    pub fn kept_now<T: Kept>(&self) -> Option<&T> {
        self.kept.get(&std::any::TypeId::of::<T>()).and_then(|kept| (&**kept as &dyn std::any::Any).downcast_ref())
    }

    /// Whether a family shows a log now (a fight's, a dungeon floor's): the
    /// lines said go there.
    pub fn logs(&self) -> bool {
        self.kept.values().any(|kept| kept.logs())
    }

    /// A new scene opens: each family lets go of what goes with the last one;
    /// the beats the new scene opens with.
    pub fn kept_new_scene(&mut self) -> Vec<Beat> {
        self.kept.values_mut().filter_map(|kept| kept.new_scene()).collect()
    }

    /// A number drawn: the same film, the same draws.
    pub fn draw(&mut self) -> u64 {
        self.chance = self.chance.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.chance >> 34) % ((1 << 30) - 1) + 1
    }
}

/// A line of the text as it was read: its word, the family whose form fit
/// it, what that form gave each of its names, and where in the line.
#[derive(Clone, Debug)]
pub struct ReadAs {
    pub word: String,
    pub group: &'static str,
    pub values: Vec<crate::grammar::Value>,
}

impl ReadAs {
    /// Where the line's word stands among the words written there: 0 when
    /// written first (`music calm`), 1 when after who acts (`pikachu walk
    /// to piplup`); none if the line now written is another word's.
    pub fn word_at(&self, written: &str) -> Option<usize> {
        written.split_whitespace().take(2).position(|said| crate::words::plain(said) == self.word)
    }

    /// The edit that makes that field of the line, as it was read, these
    /// words; why not, if the line now written is another word's, or has
    /// no such field said. Written who first, the field is found where it
    /// is written: who before the word, the rest after it.
    pub fn edit(&self, number: usize, written: &str, name: &str, words: &[String]) -> Result<crate::doc::Edit, String> {
        let changed = || format!("line {number}: changed since the film played");
        let word_at = self.word_at(written).ok_or_else(changed)?;
        let value = self.values.iter().find(|value| value.name == name).ok_or_else(|| format!("line {number}: \"{}\" has no field \"{name}\"", self.word))?;
        let at = value.at.ok_or_else(|| format!("line {number}: \"{name}\" is not written there"))?;
        let long = value.words.len();
        if word_at == 0 {
            return Ok(crate::doc::Edit::Words { number, at, long, with: words.to_vec() });
        }
        // (Its words among those written: those before the word, then the
        // word, then the rest; a field never across the word.)
        let tokens: Vec<(usize, usize)> = tokens(written);
        let placed = |read: usize| if read < word_at { read } else { read + 1 };
        if long == 0 || (at < word_at && at + long > word_at) {
            return Err(format!("line {number}: \"{name}\" is not written there"));
        }
        let (first, last) = (placed(at), placed(at + long - 1));
        let (Some(&(start, _)), Some(&(_, end))) = (tokens.get(first), tokens.get(last)) else { return Err(changed()) };
        let text = format!("{}{}{}", &written[..start], words.join(" "), &written[end..]);
        Ok(crate::doc::Edit::Line { number, text })
    }
}

/// What a line said keeps in the scene at hand, nothing at the next one.
#[derive(Default)]
pub struct Speaking {
    pub faces: std::collections::HashMap<String, String>,
    pub last_speaker: Option<String>,
}

/// The things of the scene at hand, nothing at the next one.
#[derive(Default)]
pub struct ThingsState {
    /// The things, how many of each; the faces said.
    pub stuff: Vec<crate::film::Stuff>,
    pub copies: std::collections::HashMap<String, u32>,
}

/// The film's time, the whole film: how long lines stay, the marks, the
/// threads beside it, what the scene waits for before it closes, the
/// instants marked, what is open (what `end` plays to close it).
#[derive(Default)]
pub struct TimingState {
    /// (How long lines stay, if the film says; the box at the top.)
    pub line_time: Option<i32>,
    pub marks: usize,
    pub threads: usize,
    pub awaited: Vec<String>,
    /// What is played beside the film under a name (`& as wind`), by
    /// that name: its thread.
    pub named: std::collections::HashMap<String, String>,
    /// The instants marked in the scene at hand (`mark name`).
    pub time_marks: Vec<String>,
    /// A music was given, somewhere above in the film (`music …`): its
    /// clock goes (`at music 12s`, `wait music 12s`).
    pub music: bool,
    /// The plain ambient is a sound of ours (`ambient my_wind`): the
    /// game's, coming after, stops it.
    pub ambiance_own: bool,
    /// What is open (a shot of a picture, a vision, a memory, a choice, a
    /// fight on the floor), the last last: what `end` plays to close it.
    pub opened: Vec<fn(&mut crate::film::Film, &crate::verbs::Line)>,
}

/// What a scene the game makes knows: who the player walks, how many talks so far.
#[derive(Default)]
pub struct Game {
    /// A scene the game makes of a place (`film::compile_place`): the words
    /// only the game writes are known.
    pub made_by_the_game: bool,
    pub player: Option<String>,
    pub(crate) talks: usize,
    pub(crate) menus: usize,
    /// The dialogues open, up to their `end`: who, its label, past it.
    pub(crate) open: Vec<(engine::Who, String, String)>,
}

/// Where each word of a line is, in bytes.
fn tokens(written: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut start = None;
    for (at, letter) in written.char_indices().chain(std::iter::once((written.len(), ' '))) {
        match (letter.is_whitespace(), start) {
            (false, None) => start = Some(at),
            (true, Some(from)) => {
                found.push((from, at));
                start = None;
            }
            _ => {}
        }
    }
    found
}
