//! A dungeon of one's own made in the Resources: its name, its floors, the
//! look of a game dungeon's floors, its music, the Pokémon met there and what
//! lies there, written as `library/dungeons/<word>.dungeon` (see
//! `rules::own`); and tried at once, played in the editor with the keys.
//!
//! A crate of Dungeon Workshop: the editor of one dungeon or boss fight
//! (`DungeonMaking`, drawn by `make_dungeon`), the Dungeon space that lists
//! them all (`dungeons`), a species exported as CSV (`level_csv`). It uses
//! `workshop_base` (the session, the look, safe saves) and the game's crates
//! (`rules`, `game`, `dive`...).

use std::collections::HashMap;
use std::time::Instant;

use eframe::egui::{self, Color32, Rect, RichText, Sense, TextureHandle, pos2, vec2};
use game::dungeon_check::{Floors, Foe, Lying};
use rules::own::plain;
use rules::run::Book;
use shell::Screen;
use shell::sound::Hears;

use workshop_base::done::Done;
use workshop_base::theme;

mod arena;
mod check;
mod choosers;
mod dungeon_columns;
pub mod dungeons;
mod editor;
mod fight_columns;
mod floor;
mod form;
mod layout;
pub mod level_csv;
mod lists;
mod state;
#[cfg(test)]
mod tests;

pub use editor::make_dungeon;
pub use lists::{Summary, fight_summaries, fight_summaries_in, summaries, summaries_in};

/// What the game's tables offer: the dungeons whose floors can be taken,
/// the Pokémon met in its dungeons, the items, its songs.
pub struct Offered {
    looks: Vec<(u16, String, usize)>,
    species: Vec<(u16, String)>,
    items: Vec<String>,
    /// The game's moves, by their names.
    moves: Vec<String>,
    /// Who the tried team is at first: Pikachu and Piplup.
    team: (u16, u16),
    /// Who can be of a team: those the game starts with or its dungeons meet.
    able: Vec<(u16, String)>,
    /// The game's tables, for what the rules say of a dungeon; what they offer, for its check.
    pub book: Book,
    known: game::dungeon_check::Known,
}

/// What the editor offers, read from the session's data once (none without `dungeons.bin`).
pub fn offered(session: &workshop_base::session::Session) -> Option<std::sync::Arc<Offered>> {
    let data = session.data().to_string();
    (*session.kept.get(|| read_offered(&data).map(std::sync::Arc::new))).clone()
}

fn read_offered(data: &str) -> Option<Offered> {
    {
        {
            let book = Book::read(&shell::data::read(data, "dungeons.bin")?)?;
            let tables = &book.tables;
            let known = game::dungeon_check::Known::of(&book);
            let (looks, species, items) = (known.looks.clone(), known.species.clone(), known.items.clone());
            let named = |wanted: &str| tables.species.iter().position(|kind| plain(&kind.name) == wanted).map_or(25, |at| at as u16);
            let team = (named("pikachu"), named("piplup"));
            // (Any who starts the game, or whom its dungeons meet: the game knows how it fights.)
            let mut able: Vec<(u16, String)> = (0..tables.species.len() as u16)
                .filter(|id| book.member_at(*id, false, Some(5)).is_some())
                .map(|id| (id, tables.species[id as usize].name.clone()))
                .filter(|(_, name)| !name.is_empty())
                .collect();
            able.sort_by_key(|(_, name)| plain(name));
            able.dedup_by(|a, b| plain(&a.1) == plain(&b.1));
            let mut moves: Vec<String> = tables.moves.iter().filter(|name| !name.is_empty() && !name.starts_with('[')).cloned().collect();
            moves.sort_by_key(|name| plain(name));
            moves.dedup();
            Some(Offered { looks, species, items, moves, team, able, book, known })
        }
    }
}

/// ` floors 2-3`, ` floor 4`, or nothing for all of them.
fn floors_said(floors: Floors) -> String {
    match floors {
        Some((low, high)) if low == high => format!(" floor {low}"),
        Some((low, high)) => format!(" floors {low}-{high}"),
        None => String::new(),
    }
}

/// What a `.dungeon` says, as the editor shows it.
#[derive(Clone, PartialEq)]
struct Form {
    /// Its comments at the top, kept as they are.
    head: Vec<String>,
    name: String,
    floors: usize,
    look: String,
    look_floor: usize,
    /// Its tiles drawn by one (`library/tilesets/<word>.png`; empty: the look's).
    tileset: String,
    music: String,
    foes: Vec<Foe>,
    items: Vec<Lying>,
    recruit: bool,
    up: bool,
    /// The options of its rules (`rules:`), and the types it gives
    /// (`pokemon_type:`, `move_type:` lines, kept as written).
    options: rules::rule_options::RuleOptions,
    retyped: Vec<String>,
    /// The level the team comes in at (none: as it is).
    team_level: Option<i32>,
    /// The jobs' rank there, as written (`rank: E floors 1-2, D`; empty:
    /// from the levels met there).
    rank: String,
    /// The boss fight at the end, written in it (as dungeons were first
    /// written), if there is one.
    boss: Option<BossForm>,
    /// The boss fight at the end, a file of its own (`fight: <word>`).
    fight: String,
}

/// The boss fight: who and at what level, its guards, how much more life,
/// its room (how large, what shape), its music (empty: the dungeon's).
#[derive(Clone, PartialEq)]
struct BossForm {
    species: String,
    level: i32,
    /// Its guards: species, level, where (none: where the arena puts it).
    guards: Vec<(String, i32, Option<Tile>)>,
    /// Those on the team's side: species, level, where (none: by the team).
    allies: Vec<(String, i32, Option<Tile>)>,
    /// The team of the fight played alone (empty: the one that comes), its level.
    hero: String,
    partner: String,
    team_level: Option<i32>,
    /// Where it stands, where the team comes in (none: where the arena puts
    /// them); the items lying there; its traps (kind, where, hidden).
    place: Option<Tile>,
    team: Option<Tile>,
    lying: Vec<(String, Tile)>,
    traps: Vec<(String, Tile, bool)>,
    life: f32,
    room: (usize, usize),
    shape: String,
    music: String,
    /// What it says as the fight begins; as it falls. What it lets fall
    /// (empty: nothing). The moves it knows (none: its own). Its phases.
    /// Whether it asks to join, beaten.
    begins: String,
    falls: String,
    drop: String,
    moves: Vec<String>,
    phases: Vec<PhaseForm>,
    joins: bool,
    /// A sound of ours it cries as the fight begins (empty: none).
    cry: String,
    /// What it is called under its name, as the fight begins.
    title: String,
    /// The arena's own look: a game dungeon, and a floor of it (empty: the dungeon's).
    look: String,
    look_floor: usize,
    /// The arena painted tile by tile (rows of 0 ground, 1 wall, 2 water; none: its shape).
    map: Option<Vec<Vec<u8>>>,
}

/// A phase of the fight: under so much of its life (hundredths), what it
/// says, its anger (stages, 0: none), who comes to help it, how much of its
/// life it gets back (0: none), the weather it brings (empty: none), the
/// moves it takes up (none: the same).
#[derive(Clone, Debug, Default, PartialEq)]
struct PhaseForm {
    share: i32,
    says: String,
    fiercer: i32,
    help: Vec<(String, i32)>,
    heal: i32,
    weather: String,
    moves: Vec<String>,
    /// A sound of ours it cries then (empty: none); the screen shaken then.
    cry: String,
    shake: bool,
}

/// The colour of the team's allies in the arena.
const ALLY: Color32 = Color32::from_rgb(0x40, 0xD0, 0xC0);

/// A tile of the arena, from its top left corner.
type Tile = (usize, usize);

/// What a click on the arena puts there.
#[derive(Clone, Copy, PartialEq)]
enum ArenaTool {
    Team,
    Boss,
    Guard(usize),
    Ally(usize),
    Item,
    Trap,
    Erase,
    /// The ground painted: 0 ground, 1 wall, 2 water.
    Paint(u8),
}

/// The dungeon being tried: played in the editor, the keys its own while the
/// pointer is over it.
struct Trial {
    diving: game::dive::Diving,
    screen: Screen,
    texture: Option<TextureHandle>,
    last: Instant,
}

/// A dungeon being made.
pub struct DungeonMaking {
    /// What the Workshop works with (`session`).
    pub session: std::sync::Arc<workshop_base::session::Session>,
    /// The project it is written in: the open game's folder, or the shared library.
    pub project: assets::project::Project,
    pub word: String,
    pub saved: bool,
    form: Form,
    /// The form as last written.
    written: Form,
    /// What is wrong with it, for the text it was found in.
    checked: (String, Vec<check::Problem>),
    sought_foe: String,
    sought_item: String,
    trial: Option<Trial>,
    said: Option<String>,
    portraits: HashMap<u16, Option<TextureHandle>>,
    /// The song heard (its word).
    heard: Option<String>,
    /// A boss fight of its own (`library/fights/<word>.fight`) rather than a dungeon.
    pub fight: bool,
    /// How much of its life the boss has when its fight is tried alone (in hundredths).
    boss_life: i32,
    sought_move: String,
    /// The arena's tool, the item and the trap it puts, the trap hidden or not.
    arena_tool: ArenaTool,
    arena_item: String,
    arena_trap: String,
    trap_hidden: bool,
    /// What is sought for each phase: a help, a move.
    phase_sought: Vec<(String, String)>,
    sought_guard: String,
    sought_ally: String,
    /// The team of the trial, chosen, and its level (none: Pikachu and Piplup, 5).
    team: Option<(u16, u16, i32)>,
    /// The fight in numbers, for the text it was read from.
    numbers: (String, Option<layout::Numbers>),
    /// The middle shows the trial (else the arena).
    trying_shown: bool,
    /// The floor shown as the game lays it out.
    floor_shown: floor::FloorShown,
    /// A tileset being imported: the path of its picture, dithered or not.
    tile_import: Option<(String, bool)>,
    /// Where a problem of the checker was clicked, and since when (shown a while).
    pointed: Option<(check::Place, Instant)>,
    /// The checker to be scrolled to (its count clicked in the head).
    check_sought: bool,
}
