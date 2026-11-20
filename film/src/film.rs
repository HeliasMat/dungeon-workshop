//! A film being written: its words so far, and what the words alone say. Who
//! stands where is not here: the stage knows it when the film plays.

use std::collections::{BTreeMap, HashMap, HashSet};
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

/// One thing a fisher pulled out of the water: who or what it is (its
/// code, or who), its name as said, its word (the Pokémon's, or the thing's
/// key); its size once measured; in his bucket or let go (still caught,
/// told so at the end of the day); where it came out.
#[derive(Clone, Debug)]
pub struct Caught {
    pub code: String,
    pub name: String,
    pub word: String,
    pub pokemon: bool,
    pub size: Option<i32>,
    pub kept: bool,
    pub released: bool,
    pub water: Option<(i32, i32)>,
    pub landed: Option<(i32, i32)>,
}

/// A thing in the air: the next line says how it lands.
pub struct Flying {
    pub code: String,
    pub target: String,
    pub wait: bool,
    pub thrower: String,
}

/// A shop: its keeper, and each thing laid out (its word, its code, its price).
pub type Shop = (String, Vec<(String, String, i32)>);

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
}

impl Staging {
    /// The light of the scene's hour, as `daytime` last laid it.
    pub fn daylight(&self) -> [f32; 4] {
        self.daylight.unwrap_or_else(|| crate::emit::light_of(engine::Daylight::Normal))
    }
}

pub struct Film {
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
    pub speaking: crate::compile::Speaking,
    /// The things of the scene at hand, how many of each (`verbs/things.rs`).
    pub things: crate::verbs::things::ThingsState,
    /// The film's time: threads, marks, what the scene waits for, what is open (`verbs/timing.rs`).
    pub timing: crate::verbs::timing::TimingState,
    /// The film's plan: a line each, its thread, placing and time (`time::plan`).
    pub plan: crate::time::Plan,

    /// Fishing's own: what is in the water, who swims, what was caught.
    pub fishing: crate::verbs::fishing::FishingState,
    /// A film one plays: who the player walks (`verbs/game.rs`).
    pub game: crate::verbs::game::Game,
    /// A dungeon's own: its floor, its team, what lies on the floor at hand.
    pub dungeon: crate::verbs::dungeon::DungeonState,
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
    pub combat: crate::verbs::fights::Combat,
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
    /// What the film read a line as, to be told when asked (`verifie --tout`):
    /// what `there` and `team` stood for.
    pub notes: Vec<(usize, String)>,
    /// How many recipes were called: each call's own numbers (`$px`).
    pub recipe_calls: usize,
    /// The recipes the film writes and uses, each once: checked alone
    /// once the film is read (`text::checked`).
    pub defined: Vec<crate::text::checked::Defined>,
    /// The line at hand.
    pub number: usize,
    /// What each of the text's own lines was read as.
    pub read_as: std::collections::BTreeMap<usize, crate::compile::ReadAs>,
}

impl Film {
    /// A film compiled with the repository's tables, for the tests.
    #[cfg(test)]
    pub fn new() -> Self {
        Film::with(crate::tests::tables())
    }

    /// A film compiled with these tables.
    pub fn with(tables: Arc<Tables>) -> Self {
        Film {
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
            fishing: Default::default(),
            game: Default::default(),
            dungeon: Default::default(),
            flying: None,
            own: BTreeMap::new(),
            pictures: Vec::new(),
            rhythms: BTreeMap::new(),
            thing_words: HashSet::new(),
            paths: BTreeMap::new(),
            acting: true,
            collisions: false,
            clouds: 8,
            combat: Default::default(),
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
            notes: Vec::new(),
            recipe_calls: 0,
            defined: Vec::new(),
            read_as: Default::default(),
            number: 0,
        }
    }

    /// The film waits for someone to get where it goes (each of a crowd),
    /// if the line says so; else the line lasts as long, beside the film.
    pub fn wait_for(&mut self, who: &str, wait: bool) {
        if wait {
            self.on(who, crate::emit::wait_for);
        } else if let Some(code) = self.everyone(who).first().and_then(|one| self.someone(one)) {
            self.plan.handle(crate::time::Length::Event { handle: crate::time::Handle::Who(code.to_string()), at_least: 0 });
        }
    }

    /// The film waits that many frames (`30`, `1.5s`, `6f`), at its pace,
    /// if the line says so (else the line lasts as long, beside the film);
    /// a time it cannot read, kept for the engine to tell.
    pub fn wait_frames(&mut self, frames: impl ToString, wait: bool) {
        let frames = frames.to_string();
        if !wait {
            if let Some(read) = crate::words::frames_of(&frames) {
                self.plan.handle(crate::time::Length::Known(read));
            }
            return;
        }
        match crate::words::frames_of(&frames) {
            Some(read) => self.wait(i64::from(read)),
            None => self.not_a(&frames, "a duration"),
        }
    }

    /// A number of its own for a place found when the film plays.
    pub fn found_var(&mut self) -> String {
        self.found += 1;
        format!("_place{}", self.found)
    }

    /// A number drawn: the same film, the same draws.
    pub fn draw(&mut self) -> u64 {
        self.chance = self.chance.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.chance >> 34) % ((1 << 30) - 1) + 1
    }
}

impl Default for Film {
    /// A film with empty tables (it knows no Pokémon, no place).
    fn default() -> Self {
        Film::with(Arc::new(Tables::default()))
    }
}
