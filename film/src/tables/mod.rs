//! The game's files as a film reads them: Pokémon, moves, items, places, their
//! ground, what stands in them. Read by whoever compiles films, for a
//! project and a data folder it names (`Tables::read`), kept and handed down
//! by it, read again when it says so (places made); none of it changes as a
//! film plays.
//!
//! - `files`: where they are, the lists merged;
//! - `monsters`: the Pokémon, variants, generations;
//! - `places`: grounds, places of our own, pictures, props, drawings;
//! - `story`: what the story's scenes say of the places.

use std::collections::{BTreeMap, HashMap, HashSet};

use assets::project::Project;

use engine::Ground;

mod files;
mod monsters;
mod places;
mod story;

pub use files::{OWN_SOUNDS, game_file};

/// The places worth knowing: level, where a group stands. Those made by
/// the `decor` builder are in `decors.txt` (`Tables::decors`).
pub const PLACES: [(&str, (u16, i32, i32)); 18] = [
    ("beach", (11, 400, 205)),
    ("beach_evening", (12, 400, 205)),
    ("town", (323, 350, 212)),
    ("guild", (203, 250, 200)),
    ("guild_entrance", (197, 240, 196)),
    ("mess_hall", (210, 256, 184)),
    ("crew_quarters", (211, 180, 182)),
    ("forest", (52, 276, 220)),
    ("map", (268, 252, 168)),
    ("waterfall_valley", (229, 372, 800)),
    ("lava_field", (148, 456, 228)),
    ("amphitheater", (240, 300, 420)),
    ("rocky_desert", (147, 228, 264)),
    ("vast_plain", (174, 720, 480)),
    ("ruins", (66, 250, 235)),
    ("lake", (36, 300, 232)),
    ("cafe", (227, 350, 232)),
    ("cave", (28, 300, 350)),
];

/// The pictures not for films, of `library/tables/images.txt`.
pub const NOT_FOR_FILMS: [&str; 1] = ["screen"];
/// The place a scene goes to when its own is not known.
pub const SOMEWHERE: (u16, i32, i32) = PLACES[2].1;
/// The world map's level: a picture, not a place to walk.
pub const MAP_LEVEL: u16 = 268;
/// The world map: the size of its pictures, where the team's town is on it.
pub const MAP_PICTURE: (i32, i32) = (504, 336);
pub const HOME: (i32, i32) = (212, 180);

/// A name of the game: its number, how it is written.
pub type Named = (String, String);

/// One who lives in a place (`resident` in its `.decor`).
#[derive(Clone, Debug)]
pub struct Resident {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub facing: String,
    pub wanders: i32,
    /// What it does over and over (its animation's name); held on its first picture.
    pub animation: Option<String>,
    pub still: bool,
}

/// A picture of the game: its level, category, what the view is on.
#[derive(Clone, Debug)]
pub struct Picture {
    pub level: u16,
    pub category: String,
    pub x: i32,
    pub y: i32,
}

/// An animation of the game: the frames a case stays, the middles of its cases.
#[derive(Clone, Debug)]
pub struct Animation {
    pub frames: i32,
    pub cases: Vec<(i32, i32)>,
}

/// A level by its name: its number, usual middle, name, size.
#[derive(Clone, Debug)]
pub struct Level {
    pub level: u16,
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub size: (i32, i32),
}

/// How a species goes about by nature, as the game's tables say
/// (`rules::tables::Species::movement`): it walks, floats over everything,
/// passes through walls, swims, or walks on lava too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    Walks,
    Hovers,
    Ghost,
    Lava,
    Swims,
}

impl Movement {
    /// The game's movement type: 2 hovering, 3 intangible, 4 lava, 5 water;
    /// the others walk.
    pub fn of(kind: u8) -> Movement {
        match kind {
            2 => Movement::Hovers,
            3 => Movement::Ghost,
            4 => Movement::Lava,
            5 => Movement::Swims,
            _ => Movement::Walks,
        }
    }
}

/// What the game's tables say of a monster (`rules::tables::Species`),
/// for the films: its two types (0: none), its shadow (0 small, 1
/// middling, 2 big), its family, the monster it evolves from (0: none),
/// the moves it learns by level and those machines teach it.
#[derive(Clone, Debug, Default)]
pub struct Kind {
    pub types: (u8, u8),
    pub shadow: Option<u8>,
    pub family: u16,
    pub from: u16,
    pub learnset: Vec<(u8, u16)>,
    pub machines: Vec<u16>,
}

/// All of the game's files a film needs.
#[derive(Debug, Default)]
pub struct Tables {
    pub monsters: BTreeMap<String, Named>,
    pub moves: BTreeMap<String, Named>,
    pub items: BTreeMap<String, Named>,
    /// How big each monster shows, by its number.
    pub bodies: HashMap<String, (i32, i32)>,
    /// The animations each monster's sprite has, by its number: their
    /// groups (`Sit` is 23), as `sprites.txt` lists its sheets.
    pub sprite_groups: HashMap<String, HashSet<u16>>,
    /// Where each monster's head, hands and middle are (`points.txt`), by
    /// its number: from where it stands, y downwards, facing us.
    pub points: HashMap<String, [(i32, i32); 4]>,
    /// How each monster goes about by nature, by its number: the game's
    /// movement types (`Movement`), from its tables (`dungeons.bin`); none
    /// told, it walks.
    pub movements: HashMap<String, Movement>,
    /// How heavy each monster is, by its number, from its tables
    /// (`dungeons.bin`): who wins a shove (`push`).
    pub weights: HashMap<String, u16>,
    /// What else the game's tables say of each monster, by its number
    /// (`dungeons.bin`): its types, shadow, family, what it evolves from,
    /// the moves it learns.
    pub kinds: HashMap<String, Kind>,
    /// Each move's type and power, by its number (`dungeons.bin`).
    pub move_kinds: Vec<(u8, i32)>,
    /// The faces each monster's portraits have, by its number (`happy` is
    /// 1), as `portraits/` holds them.
    pub portraits: HashMap<String, HashSet<u16>>,
    /// Those with a sprite, in the order of their numbers.
    pub with_sprites: Vec<String>,
    /// The group of each species' own sprite that shows it roaring, by its
    /// number, where the sprite has one (`roar`): read from one's own game
    /// (`extra/ds_animations.txt`, `tools/ds/animations.py`), none without.
    pub roars: HashMap<String, u16>,
    /// Those with a portrait alone, and the file.
    pub faces_alone: BTreeMap<String, String>,
    pub generations: BTreeMap<usize, Vec<String>>,
    pub pictures: BTreeMap<String, Picture>,
    pub animations: BTreeMap<String, Animation>,
    pub picture_levels: HashSet<u16>,
    pub grounds: HashMap<u16, Ground>,
    pub things: HashMap<String, Vec<(String, i32, i32)>>,
    pub usual_spots: HashMap<String, Vec<(i32, i32)>>,
    pub map_spots: HashMap<String, (i32, i32)>,
    pub levels: BTreeMap<String, Level>,
    /// Every level of Explorers of Sky's, by its number (`levels.txt`): what
    /// a film `for ds` can show.
    pub sky_levels: HashSet<u16>,
    /// The places made by the `decor` builder, by their names: level, where a
    /// group stands (`data/extra/decors.txt`).
    pub decors: BTreeMap<String, (u16, i32, i32)>,
    /// Where one comes in and goes out of each of those places, by level:
    /// its name and point (`entrance x y name`).
    pub entrances: HashMap<u16, Vec<(String, i32, i32)>>,
    /// Who lives in each place of this project's, by level (`town.txt`):
    /// their names, places, the way each faces, how far it walks about.
    pub residents: HashMap<u16, Vec<Resident>>,
    /// The game's place names of two words or more, the longest first.
    pub place_names: Vec<String>,
    /// The props by their words (`library/tables/objects.txt`, the game's own names);
    /// the frames of each one's animation.
    pub props: BTreeMap<String, u16>,
    pub prop_frames: BTreeMap<u16, i32>,
    /// The pictures that are ours, by their words: their files, how many
    /// pictures in each (`torch.4.png`: 4).
    pub drawings: BTreeMap<String, (String, u16)>,
    /// The monsters with a fishing rod drawn in their pictures, by number.
    pub fishing: HashSet<String>,
    /// The monsters with a rifle drawn in their frames, by number.
    pub aiming: HashSet<String>,
    /// The variants of each species, by its number: their words, in order
    /// (`25`: `pikachu_chasseur`, `pikachu_pecheur`...).
    pub variants: BTreeMap<String, Vec<String>>,
    /// The variants painted by hand in only some ways, by number: those ways
    /// (`views down` in their table), which they keep facing by themselves.
    pub views: HashMap<String, Vec<String>>,
    /// The song of each place of the game, by its number, as its scenes play
    /// it most (`library/place_songs.txt`).
    pub place_songs: HashMap<u16, u16>,
    /// The project they were read for: where its films, recipes, pictures
    /// and sounds of its own are (`compile_with` looks there).
    pub project: Project,
    /// The names of the project's own sounds (`<name>.wav`), then the
    /// shared library's, in order.
    pub own_sounds: Vec<String>,
    /// The data folder they were read from (`data/`: its layers).
    pub data: String,
    /// The language the films' lines are said in (`en`, `fr`…): a line
    /// that does not give it is said in English. Chosen by the program
    /// (`with_language`), English unless it says otherwise.
    pub language: String,
}

/// The language a film's lines are said in when none is chosen.
pub const ENGLISH: &str = "en";

impl Tables {
    /// The game's files in that data folder read now, for that project's
    /// films (its own pictures, sounds, recipes, then the shared library's).
    pub fn read(project: &Project, data: &str) -> Tables {
        read_tables(project, data)
    }

    /// The same tables, the lines said in that language (`fr`; empty: English).
    pub fn with_language(mut self, code: &str) -> Tables {
        let code = code.trim().to_lowercase();
        self.language = if code.is_empty() { ENGLISH.to_string() } else { code };
        self
    }
}

fn read_tables(project: &Project, data: &str) -> Tables {
    let mut tables = Tables {
        moves: files::named(data, "moves.txt"),
        items: files::named(data, "items.txt"),
        project: project.clone(),
        own_sounds: files::own_sounds_of(project),
        data: data.to_string(),
        language: ENGLISH.to_string(),
        ..Tables::default()
    };
    monsters::read_monsters(&mut tables);
    places::read_grounds(&mut tables);
    places::read_catalogue(&mut tables);
    places::read_things(&mut tables);
    story::read_story(&mut tables);
    places::read_residents(&mut tables);
    let songs = files::lines_of(&project.shared().join("place_songs.txt"));
    tables.place_songs =
        songs.iter().filter_map(|line| line.split_once('\t')).filter_map(|(level, song)| Some((level.parse().ok()?, song.trim().parse().ok()?))).collect();
    tables
}
