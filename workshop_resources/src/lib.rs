//! The Resources space: all a film and a decor are made with, looked
//! through as in a studio's asset browser.
//! - the Library: the shelves (the Pokémon catalog, `catalog.rs`: every
//!   species, its forms and variants under it; their faces; the things drawn
//!   and the game's, the songs and the sounds, the effects, the pieces of the
//!   decors), how many on each, the favourites;
//! - the Resources: a shelf's things in a grid (or a list), their pictures,
//!   sought by a few letters everywhere at once (on the Pokémon shelf: the
//!   catalog, by name, number, type, generation), sorted, made larger or
//!   smaller; the Pokémon's and the songs' cards their own (`pokemon_cards.rs`,
//!   `music_cards.rs`), only those in sight drawn; a PNG dropped on the window
//!   joins the things drawn;
//! - the Inspector: the one chosen in large, its pictures one by one or
//!   playing, heard if it is heard, what it is, how a film writes it (written
//!   in the film open in a click), the films and decors it is in; a Pokémon's
//!   page (`inspector/page.rs`: its animations, faces, forms, credits);
//! - the Assets (`picker`), a panel of the Film space: the same shelves made
//!   small, beside the film, to drag onto its Scene or its Script.
//!
//! A crate of Dungeon Workshop: the Resources space (`Resources`) and its
//! makings (`Maker`: a picture of `workshop_picture`, a sound of
//! `workshop_sound`, a song of `workshop_music`, a dungeon or a fight of
//! `workshop_dungeon`). It uses `workshop_base` (the session, the look),
//! `workshop_decor` (the decor open, the pieces' pictures) and
//! `workshop_script` (a word written in the film open).

use std::sync::Arc;

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::time::Instant;
use workshop_base::session::Session;

use eframe::egui::{self, Rect, Vec2, pos2, vec2};
use shell::sound::Hears;

use pictures::sheet_of;
use shelves::{favourites_file, filter_words, made_list};
use workshop_decor::Decors;

mod catalog;
mod dropped;
mod game_shelves;
mod grid;
mod inspector;
mod maker;
mod music_cards;
mod picker;
mod pictures;
mod pokemon_cards;
mod shelves;
mod studio;
#[cfg(test)]
mod tests;
mod thumbs;
mod toolbar;
mod tree;

pub use grid::grid;
pub use inspector::inspector;
pub use maker::Maker;
pub use picker::{Tab as PickerTab, picker};
pub use studio::{making_tabs, studio};
pub use tree::tree;

/// New pictures read in one frame, at most (the others the next ones).
const READ_EACH_FRAME: usize = 14;

/// A shelf of the Library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Shelf {
    Pokemon,
    /// Each Pokémon's faces, one card each.
    Portraits,
    Drawn,
    Props,
    /// The game's items, with their icons.
    Items,
    Songs,
    Sounds,
    /// The moves, by the effects they show; the statuses; the skies.
    Attacks,
    Statuses,
    Weather,
    /// The game's effects no move shows.
    Effects,
    /// The places' and pictures' backdrops; the dungeons' tilesets.
    Backdrops,
    Tilesets,
    Pieces,
    /// The dungeons written (`library/dungeons`), and those with a boss fight.
    Dungeons,
    Bosses,
    Favourites,
}

impl Shelf {
    const ALL: [Shelf; 16] = [
        Shelf::Pokemon,
        Shelf::Portraits,
        Shelf::Drawn,
        Shelf::Props,
        Shelf::Items,
        Shelf::Songs,
        Shelf::Sounds,
        Shelf::Attacks,
        Shelf::Statuses,
        Shelf::Weather,
        Shelf::Effects,
        Shelf::Backdrops,
        Shelf::Tilesets,
        Shelf::Pieces,
        Shelf::Dungeons,
        Shelf::Bosses,
    ];

    /// The groups of the Library, each its shelves (none: a shelf alone).
    const GROUPS: [(&'static str, &'static [Shelf]); 7] = [
        ("Characters", &[Shelf::Pokemon, Shelf::Portraits]),
        ("Objects", &[Shelf::Drawn, Shelf::Props, Shelf::Items]),
        ("Audio", &[Shelf::Songs, Shelf::Sounds]),
        ("Effects", &[Shelf::Attacks, Shelf::Statuses, Shelf::Weather, Shelf::Effects]),
        ("Places", &[Shelf::Backdrops, Shelf::Tilesets]),
        ("Dungeons", &[Shelf::Dungeons, Shelf::Bosses]),
        ("", &[Shelf::Pieces]),
    ];

    fn name(self) -> &'static str {
        match self {
            Shelf::Pokemon => "Pokémon",
            Shelf::Portraits => "Faces",
            Shelf::Drawn => "Drawn",
            Shelf::Props => "The Game's",
            Shelf::Items => "Items",
            Shelf::Songs => "Music",
            Shelf::Sounds => "Sounds",
            Shelf::Attacks => "Attacks",
            Shelf::Statuses => "Status Effects",
            Shelf::Weather => "Weather",
            Shelf::Effects => "Other Effects",
            Shelf::Backdrops => "Backdrops",
            Shelf::Tilesets => "Tilesets",
            Shelf::Pieces => "Decor (Elements)",
            Shelf::Dungeons => "Dungeons",
            Shelf::Bosses => "Boss Fights",
            Shelf::Favourites => "Favourites",
        }
    }

    /// Its group's name, and its own (`Objects › Drawn`).
    fn path(self) -> String {
        match Shelf::GROUPS.iter().find(|(group, shelves)| !group.is_empty() && shelves.contains(&self)) {
            Some((group, _)) => format!("{group} › {}", self.name()),
            None => self.name().to_string(),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Shelf::Pokemon => "pokemon",
            Shelf::Portraits => "portrait",
            Shelf::Drawn => "drawing",
            Shelf::Props => "prop",
            Shelf::Items => "item",
            Shelf::Songs => "song",
            Shelf::Sounds => "sound",
            Shelf::Attacks => "attack",
            Shelf::Statuses => "status",
            Shelf::Weather => "weather",
            Shelf::Effects => "effect",
            Shelf::Backdrops => "backdrop",
            Shelf::Tilesets => "tileset",
            Shelf::Pieces => "piece",
            Shelf::Dungeons => "dungeon",
            Shelf::Bosses => "boss",
            Shelf::Favourites => "favourite",
        }
    }

    fn heard(self) -> bool {
        matches!(self, Shelf::Songs | Shelf::Sounds)
    }

    /// Its things play over and over on their cards (the game's effects).
    fn animated(self) -> bool {
        matches!(self, Shelf::Attacks | Shelf::Statuses | Shelf::Effects)
    }

    /// A line of it is shown played, on the beach, in the Inspector.
    fn watched(self) -> bool {
        matches!(self, Shelf::Pokemon | Shelf::Portraits | Shelf::Items | Shelf::Attacks | Shelf::Statuses | Shelf::Weather | Shelf::Effects)
    }
}

/// A thing of a shelf: its word (as a film writes it), its name shown, what
/// it is, its number (to sort by), its file.
#[derive(Clone, Debug)]
struct Item {
    shelf: Shelf,
    word: String,
    label: String,
    about: String,
    number: i64,
    file: Option<PathBuf>,
    /// Where it comes when sorted by number: a Pokémon's in the Pokédex, else the game's (none: by name).
    order: i64,
}

impl Item {
    fn key(&self) -> String {
        format!("{}:{}", self.shelf.key(), self.word)
    }

    /// How a film (or a decor, or a dungeon) writes it: the line "Copy Film Line" gives.
    fn line(&self) -> Option<String> {
        let word = &self.word;
        match self.shelf {
            Shelf::Pokemon => Some(format!("{word} walk to right")),
            Shelf::Portraits => Some(format!("{word}: ...")),
            Shelf::Drawn => Some(format!("pikachu hold {word}")),
            Shelf::Props => Some(format!("object put {word} front_of pikachu")),
            Shelf::Items => Some(format!("pikachu hold {}", self.label)),
            Shelf::Songs => Some(format!("music {word}")),
            Shelf::Sounds => Some(format!("sound {word}")),
            Shelf::Attacks => Some(format!("effect {} on pikachu", self.label)),
            Shelf::Statuses => Some(format!("pikachu status {word}")),
            Shelf::Weather => Some(format!("weather {word}")),
            Shelf::Effects => Some(format!("effect pikachu {word}")),
            Shelf::Backdrops => Some(format!("scene: {word}")),
            Shelf::Tilesets => word.parse::<u16>().is_err().then(|| format!("look: {word}")),
            Shelf::Pieces => Some(format!("put {word}")),
            Shelf::Dungeons | Shelf::Bosses => Some(format!("dungeon {word}")),
            Shelf::Favourites => None,
        }
    }

    /// The little film its line is watched played in: on the beach, beside someone.
    fn watched_film(&self) -> Option<String> {
        let line = self.line()?;
        match self.shelf {
            Shelf::Attacks => Some(workshop_base::example::film_of_move("pikachu", &self.label)),
            Shelf::Portraits => {
                let who = self.word.split_once(' ').map_or(self.word.as_str(), |(who, _)| who);
                let other = if who == "eevee" { "pikachu" } else { "eevee" };
                Some(format!(
                    "scene: beach {who} {other}
{who} put center facing {other}
{other} put right_of {who} facing {who}
frame {who} {other}
wait 0.4s
{}: Hello!
wait 1s
",
                    self.word
                ))
            }
            shelf if shelf.watched() => Some(workshop_base::example::film_of(&line)),
            _ => None,
        }
    }
}

/// A picture, cut in steps: its texture, a step's size, how many, each one's 60ths.
#[derive(Clone)]
struct Sheet {
    texture: egui::TextureHandle,
    cell: Vec2,
    count: usize,
    ticks: Vec<u8>,
}

impl Sheet {
    /// The part of the texture a step is.
    fn uv(&self, step: usize) -> Rect {
        let size = self.texture.size_vec2();
        let step = step % self.count.max(1);
        let x = step as f32 * self.cell.x / size.x;
        Rect::from_min_max(pos2(x, 0.0), pos2(x + self.cell.x / size.x, self.cell.y / size.y))
    }

    /// The part of the texture a step is, on a row (the way a body faces, from below: 0).
    fn uv_at(&self, step: usize, row: usize) -> Rect {
        let first = self.uv(step);
        let high = self.cell.y / self.texture.size_vec2().y;
        let row = if (row + 1) as f32 * high <= 1.001 { row } else { 0 };
        first.translate(vec2(0.0, row as f32 * high))
    }

    /// The step at that moment, the steps playing round.
    fn step_at(&self, seconds: f64) -> usize {
        let ticks: Vec<u32> = (0..self.count).map(|step| u32::from(*self.ticks.get(step).unwrap_or(&6)).max(1)).collect();
        let total: u32 = ticks.iter().sum();
        let mut now = ((seconds * 60.0) as u64 % u64::from(total.max(1))) as u32;
        for (step, length) in ticks.iter().enumerate() {
            if now < *length {
                return step;
            }
            now -= length;
        }
        0
    }
}

/// The Resources space as the window keeps it.
pub struct Resources {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    shelf: Shelf,
    chosen: Option<String>,
    sought: String,
    size: f32,
    listed: bool,
    /// Sorted by number (0), by name (1), by how many films use it (2).
    sort: u8,
    /// Only those used in a film; a generation of Pokémon, a mood of songs.
    only_used: bool,
    filter: Option<String>,
    /// The grid to be scrolled to the one chosen (or to its top), once.
    scroll_wanted: bool,
    folded: BTreeSet<&'static str>,
    favourites: BTreeSet<String>,
    lists: HashMap<Shelf, Vec<Item>>,
    pictures: HashMap<String, Option<Sheet>>,
    read_now: usize,
    /// The Inspector: the step shown, playing or not; a face of a Pokémon shown.
    step: usize,
    playing: bool,
    face: usize,
    watching: bool,
    /// A Pokémon's body shown (not its faces): which animation (its file), the way it faces.
    body: Option<bool>,
    anim: u8,
    way: usize,
    /// Heard as soon as it is chosen; how loud.
    auto_hear: bool,
    volume: f32,
    /// A thing drawn being named anew (and whether in the films too); being put away.
    renaming: Option<(String, bool)>,
    trashing: bool,
    /// The pictures, animations and sounds being made (each its tab), the one shown; shown on the whole window or not
    /// (back to the Library, they stay open); one closed while not written, asked once more.
    makings: Vec<Maker>,
    making_at: usize,
    making_full: bool,
    closing: Option<usize>,
    began: Instant,
    /// When the dungeons were last read.
    dungeons_read: Option<Instant>,
    /// What is heard, by its key.
    audio: Option<Box<dyn Hears>>,
    heard: Option<String>,
    /// The films and decors, read to say where each thing is used (read again after a while).
    corpus: Option<(Instant, Vec<(PathBuf, String)>)>,
    /// Each word of them, and the files it is in.
    index: HashMap<String, BTreeSet<usize>>,
    examples: workshop_base::example::Examples,
    pub said: Option<String>,
    /// Work going on for the Pokémon chosen (its sheets written or read); a SpriteBot folder being imported.
    run: Option<workshop_base::tool_run::Run>,
    sheet_import: Option<SheetImport>,
    /// The Film space's Assets panel (`picker`).
    picker: picker::Picker,
    /// The Pokémon catalog (made when first asked, again when SpriteCollab's folders not imported are known).
    catalog: Option<Arc<catalog::Catalog>>,
    missing: Option<Vec<spritecollab::Missing>>,
    missing_reading: Option<std::sync::mpsc::Receiver<Vec<spritecollab::Missing>>>,
    /// The Pokémon's cards' pictures, made on a thread.
    thumbs: thumbs::Thumbs,
    /// The species whose forms were opened, or shut, by their word.
    open_species: std::collections::HashSet<String>,
    shut_species: std::collections::HashSet<String>,
    /// On the Pokémon shelf, a search through every shelf asked (else through the catalog).
    everywhere: bool,
    /// The part of a Pokémon's page shown.
    page: inspector::Page,
    /// SpriteCollab's credits and its forms' folders, read when a page first shows them.
    credits: Option<Arc<HashMap<spritecollab::Unit, Vec<spritecollab::Credit>>>>,
    form_units: Option<Arc<HashMap<String, spritecollab::Unit>>>,
    /// The songs' lengths (seconds, whether it loops), read on a thread.
    lengths: HashMap<u16, (f32, bool)>,
    lengths_reading: Option<std::sync::mpsc::Receiver<HashMap<u16, (f32, bool)>>>,
    /// What is drawn of each body's animation facing down (`pictures::body_bounds`), by its picture's key.
    bounds: HashMap<String, Option<Rect>>,
}

/// A folder (or zip) of SpriteBot's layout to import: in place of the Pokémon chosen, or as a variant of it.
struct SheetImport {
    source: String,
    variant: bool,
    word: String,
}

impl Resources {
    pub fn new(session: &Arc<Session>, context: &egui::Context) -> Resources {
        let favourites = std::fs::read_to_string(favourites_file(session))
            .map(|text| text.lines().map(str::to_string).filter(|line| !line.is_empty()).collect())
            .unwrap_or_default();
        // (`WORKSHOP_RESOURCE=drawing:bouee`: that shelf, that one chosen, for a picture; `pokemon?eevee`: that
        // shelf, that sought; `pokemon:eevee:faces`: that part of its page.)
        let (asked, sought) = match session.launch.resource.as_deref().map(|asked| asked.split_once('?').unwrap_or((asked, ""))) {
            Some((asked, sought)) => (Some(asked.to_string()), sought.to_string()),
            None => (None, String::new()),
        };
        let shelf = asked.as_ref().and_then(|asked| Shelf::ALL.into_iter().find(|shelf| asked.starts_with(shelf.key()))).unwrap_or(Shelf::Pokemon);
        let (asked, page) = match asked.as_deref().and_then(|asked| asked.rsplit_once(':')).and_then(|(rest, page)| Some((rest, inspector::Page::of(page)?))) {
            Some((rest, page)) => (Some(rest.to_string()), page),
            None => (asked, inspector::Page::Animations),
        };
        Resources {
            session: session.clone(),
            shelf,
            chosen: asked.as_ref().filter(|asked| asked.contains(':')).map(|asked| asked.trim_end_matches(":body").to_string()),
            sought,
            size: 96.0,
            listed: false,
            sort: 0,
            only_used: false,
            filter: None,
            scroll_wanted: true,
            folded: BTreeSet::new(),
            favourites,
            lists: HashMap::new(),
            pictures: HashMap::new(),
            read_now: 0,
            step: 0,
            playing: true,
            face: 0,
            watching: false,
            body: asked.as_ref().filter(|asked| asked.ends_with(":body")).map(|_| true),
            anim: 5,
            way: 0,
            auto_hear: true,
            volume: 0.8,
            renaming: None,
            trashing: false,
            // (`WORKSHOP_CREATE=image|animation|sound`: that being made, for a picture.)
            makings: match session.launch.create.as_deref() {
                // (Dungeons and fights are the Dungeon space's.)
                Some(kinds) => kinds
                    .split(',')
                    .filter(|kind| !kind.starts_with("dungeon") && !kind.starts_with("fight"))
                    .map(|kind| Maker::new_of(session, kind))
                    .collect(),
                None => Vec::new(),
            },
            making_at: 0,
            making_full: true,
            closing: None,
            began: Instant::now(),
            dungeons_read: None,
            audio: None,
            heard: None,
            corpus: None,
            index: HashMap::new(),
            examples: workshop_base::example::Examples::new(session, context),
            said: None,
            run: None,
            sheet_import: None,
            picker: picker::Picker::new(session.launch.picker.as_deref()),
            catalog: None,
            missing: None,
            missing_reading: None,
            thumbs: thumbs::Thumbs::new(),
            open_species: std::collections::HashSet::new(),
            shut_species: std::collections::HashSet::new(),
            everywhere: false,
            page,
            credits: None,
            form_units: None,
            lengths: HashMap::new(),
            lengths_reading: None,
            bounds: HashMap::new(),
        }
    }

    /// Everything read again (new Pokémon, new animations written by a tool).
    pub fn forget(&mut self) {
        self.lists.clear();
        self.pictures.clear();
        self.catalog = None;
        self.thumbs.forget();
        (self.credits, self.form_units) = (None, None);
    }

    /// The Pokémon catalog: made once (SpriteCollab's folders not imported looked for on a thread, then added).
    fn catalog(&mut self) -> Arc<catalog::Catalog> {
        if let Some(reading) = &self.missing_reading
            && let Ok(missing) = reading.try_recv()
        {
            self.missing = Some(missing);
            self.missing_reading = None;
            self.catalog = None;
            self.lists.remove(&Shelf::Pokemon);
            self.lists.remove(&Shelf::Portraits);
        }
        if self.missing.is_none() && self.missing_reading.is_none() && self.session.spritecollab.join("sprite").is_dir() {
            let (send, receive) = std::sync::mpsc::channel();
            let places = workshop_base::tool_run::places(&self.session);
            std::thread::spawn(move || _ = send.send(spritecollab::not_imported(&places)));
            self.missing_reading = Some(receive);
        }
        if self.catalog.is_none() {
            let missing = self.missing.clone().unwrap_or_default();
            self.catalog = Some(Arc::new(catalog::of_session(&self.session, &missing)));
        }
        self.catalog.clone().unwrap_or_default()
    }

    /// Whether a species' forms are shown (`by_itself`: as the search would have it).
    fn species_open(&self, species: &catalog::Species, by_itself: bool) -> bool {
        pokemon_cards::is_open(&self.open_species, &self.shut_species, species, by_itself)
    }

    /// A species' forms opened, or shut.
    fn toggle_species(&mut self, species: &catalog::Species, now_open: bool) {
        let word = species.entries.first().map(|entry| entry.word.clone()).unwrap_or_default();
        if now_open {
            self.open_species.remove(&word);
            self.shut_species.insert(word);
        } else {
            self.shut_species.remove(&word);
            self.open_species.insert(word);
        }
    }

    /// A Pokémon of SpriteCollab not imported yet, used: SpriteCollab imported (what is not yet) in the background;
    /// then everything read again. Whether it was needed.
    fn import_if_missing(&mut self, word: &str) -> bool {
        let catalog = self.catalog();
        let Some((_, entry)) = catalog.entry(word) else { return false };
        if entry.source != catalog::Source::Missing {
            return false;
        }
        if self.run.is_none() {
            let places = workshop_base::tool_run::places(&self.session);
            self.run = Some(workshop_base::tool_run::Run::spawn("import community", move || {
                spritecollab::import_all(&places, spritecollab::Over::Ours, &mut |_| {}).map(|totals| format!("imported from SpriteCollab: {totals}"))
            }));
            self.said = Some(format!("{word}: importing it from SpriteCollab, in the background…"));
        }
        true
    }

    /// The work on its thread told when done; an import: everything read again.
    fn work_done(&mut self) {
        if let Some(run) = self.run.as_mut()
            && let Some(done) = run.poll().cloned()
        {
            let doing = run.doing.clone();
            self.run = None;
            self.said = Some(match done {
                Ok(said) => {
                    if doing.starts_with("import") {
                        self.session.reload();
                        self.missing = None;
                        self.forget();
                    }
                    said
                }
                Err(why) => why,
            });
        }
    }

    /// The songs' lengths, read on a thread the first time they are asked.
    fn song_lengths(&mut self) -> &HashMap<u16, (f32, bool)> {
        if let Some(reading) = &self.lengths_reading
            && let Ok(lengths) = reading.try_recv()
        {
            self.lengths = lengths;
            self.lengths_reading = None;
        }
        if self.lengths.is_empty() && self.lengths_reading.is_none() {
            let (send, receive) = std::sync::mpsc::channel();
            let data = self.session.data().to_string();
            std::thread::spawn(move || _ = send.send(shell::sound::song_lengths(&data)));
            self.lengths_reading = Some(receive);
        }
        &self.lengths
    }

    /// The keys of the things open and not written yet (`dungeon:word`…), as the shelves know them.
    pub fn unsaved_keys(&self) -> std::collections::HashSet<String> {
        let mut keys = std::collections::HashSet::new();
        for maker in self.makings.iter().filter(|maker| !maker.saved()) {
            let word = maker.word();
            let shelves: &[Shelf] = match maker {
                Maker::Picture(_) => &[Shelf::Drawn],
                Maker::Sound(_) => &[Shelf::Sounds],
                Maker::Music(_) => &[Shelf::Songs, Shelf::Sounds],
                Maker::Dungeon(_) => &[Shelf::Dungeons, Shelf::Bosses],
            };
            keys.extend(shelves.iter().map(|shelf| format!("{}:{word}", shelf.key())));
        }
        keys
    }

    /// How many open things are not written yet.
    pub fn unsaved_count(&self) -> usize {
        self.makings.iter().filter(|maker| !maker.saved()).count()
    }

    /// A file restored (`workshop_base::safe`): the open things written in it read again.
    pub fn reload(&mut self, path: &std::path::Path) {
        for maker in &mut self.makings {
            let reopened = match maker {
                Maker::Music(music) if workshop_music::song_file(&self.session, &music.word) == path => {
                    Some(Maker::Music(Box::new(workshop_music::MusicMaking::open(&self.session, &music.word))))
                }
                _ => None,
            };
            if let Some(reopened) = reopened {
                *maker = reopened;
            }
        }
        self.lists.remove(&Shelf::Dungeons);
        self.lists.remove(&Shelf::Bosses);
    }

    /// The things drawn shown (asked from the Home), read again.
    pub fn show_drawn(&mut self) {
        self.lists.remove(&Shelf::Drawn);
        (self.shelf, self.scroll_wanted) = (Shelf::Drawn, true);
        self.sought.clear();
    }

    /// A shelf's things (read once; the things drawn and the decor's pieces read again when asked).
    fn list(&mut self, shelf: Shelf, decors: &Decors) -> Vec<Item> {
        if shelf == Shelf::Pieces && self.lists.get(&shelf).is_some_and(|list| list.len() != decors.pieces.len()) {
            self.lists.remove(&shelf);
        }
        // (The dungeons written, read again every few seconds: one just written shows.)
        if matches!(shelf, Shelf::Dungeons | Shelf::Bosses) && self.dungeons_read.is_none_or(|when| when.elapsed().as_secs() >= 3) {
            self.lists.remove(&Shelf::Dungeons);
            self.lists.remove(&Shelf::Bosses);
            self.dungeons_read = Some(Instant::now());
        }
        if shelf == Shelf::Favourites {
            let wanted = self.favourites.clone();
            return Shelf::ALL.iter().flat_map(|shelf| self.list(*shelf, decors)).filter(|item| wanted.contains(&item.key())).collect();
        }
        if shelf == Shelf::Pokemon && !self.lists.contains_key(&shelf) {
            let catalog = self.catalog();
            self.lists.insert(shelf, catalog::items(&catalog));
        }
        self.lists.entry(shelf).or_insert_with(|| made_list(&self.session, shelf, decors)).clone()
    }

    /// Whether the search is the catalog's (on the Pokémon shelf, not asked through every shelf).
    fn catalog_sought(&self) -> bool {
        self.shelf == Shelf::Pokemon && !self.everywhere && !self.sought.trim().is_empty()
    }

    /// What the grid shows: the shelf's, or, something being sought, all those that answer it.
    fn shown(&mut self, decors: &Decors) -> Vec<Item> {
        let sought = film::words::plain(self.sought.trim());
        // (On the Pokémon shelf: the catalog's search, by name, number, type, generation; the closest first.)
        if self.catalog_sought() {
            let catalog = self.catalog();
            let all = self.list(Shelf::Pokemon, decors);
            let by_word: HashMap<&str, &Item> = all.iter().map(|item| (item.word.as_str(), item)).collect();
            return catalog
                .search(&self.sought)
                .iter()
                .flat_map(|found| found.entries.iter().map(move |entry| (found.species, *entry)))
                .filter_map(|(species, entry)| by_word.get(catalog.species[species].entries[entry].word.as_str()).map(|item| (*item).clone()))
                .collect();
        }
        let mut items: Vec<Item> = if sought.is_empty() {
            self.list(self.shelf, decors)
        } else {
            Shelf::ALL
                .iter()
                .flat_map(|shelf| self.list(*shelf, decors))
                .filter(|item| [&item.word, &item.label, &item.about].iter().any(|said| film::words::plain(said).contains(&sought)))
                .collect()
        };
        // (A generation, a mood; those used in a film only.)
        if let Some(filter) = self.filter.clone().filter(|_| sought.is_empty()) {
            if self.shelf == Shelf::Pokemon {
                let catalog = self.catalog();
                let generation: u8 = filter.parse().unwrap_or(0);
                items.retain(|item| catalog.entry(&item.word).is_some_and(|(species, _)| species.generation == generation));
            } else {
                let wanted = filter_words(self.shelf, &filter);
                items.retain(|item| wanted.contains(&item.word));
            }
        }
        self.read_corpus();
        if self.only_used {
            items.retain(|item| self.films_using(&item.word) > 0);
        }
        match self.sort {
            0 => items.sort_by_key(|item| (item.shelf, item.order, film::words::plain(&item.label))),
            // (`Effect 2` before `Effect 10`: the words, then the number.)
            1 => items.sort_by_key(|item| {
                (
                    item.shelf,
                    film::words::plain(item.label.trim_end_matches(|letter: char| letter.is_ascii_digit())),
                    item.order,
                    film::words::plain(&item.label),
                )
            }),
            _ => {
                let mut counted: Vec<(usize, Item)> = items.into_iter().map(|item| (self.films_using(&item.word), item)).collect();
                counted.sort_by_key(|(count, item)| (item.shelf, std::cmp::Reverse(*count), item.order));
                items = counted.into_iter().map(|(_, item)| item).collect();
            }
        }
        items
    }

    fn chosen_item(&mut self, decors: &Decors) -> Option<Item> {
        let key = self.chosen.clone()?;
        let (shelf, word) = key.split_once(':')?;
        let shelf = Shelf::ALL.into_iter().find(|known| known.key() == shelf)?;
        self.list(shelf, decors).into_iter().find(|item| item.word == word)
    }

    fn favourite(&mut self, key: &str) {
        if !self.favourites.remove(key) {
            self.favourites.insert(key.to_string());
        }
        let said: Vec<&str> = self.favourites.iter().map(String::as_str).collect();
        if let Some(folder) = favourites_file(&self.session).parent() {
            _ = std::fs::create_dir_all(folder);
        }
        _ = std::fs::write(favourites_file(&self.session), said.join("\n"));
    }

    /// A thing's picture, read when it is first asked (so many each frame).
    fn picture(&mut self, context: &egui::Context, item: &Item) -> Option<Sheet> {
        let key = item.key();
        if let Some(known) = self.pictures.get(&key) {
            return known.clone();
        }
        if self.read_now >= READ_EACH_FRAME {
            context.request_repaint();
            return None;
        }
        self.read_now += 1;
        let made = sheet_of(&self.session, context, item);
        self.pictures.insert(key, made.clone());
        made
    }

    /// Heard, or no more.
    fn hear(&mut self, item: &Item) {
        if self.audio.is_none() {
            self.audio = shell::sound::Audio::open_in(self.session.data(), self.session.library().clone()).map(|audio| Box::new(audio) as Box<dyn Hears>);
        }
        let Some(audio) = self.audio.as_mut() else {
            self.said = Some("no sound: the sound card or the game's sounds are missing".to_string());
            return;
        };
        let key = item.key();
        match item.shelf {
            Shelf::Songs if self.heard.as_deref() == Some(key.as_str()) => {
                audio.song(0, None, 0.0, 0.0);
                self.heard = None;
            }
            Shelf::Songs => {
                audio.song_from(0, Some(item.number as u16), self.volume, 0.0);
                audio.volume(0, self.volume);
                self.heard = Some(key);
            }
            // (One of our own: read as it is now, heard.)
            Shelf::Sounds if item.number < 0 => {
                if let Some(stereo) = shell::own_sounds::read(self.session.data(), self.session.library(), &item.word) {
                    audio.samples(stereo, self.volume);
                }
            }
            Shelf::Sounds => audio.sound(item.number as u16, self.volume, 1.0),
            _ => {}
        }
    }

    /// The films and decors read (again after a while), each of their words indexed.
    fn read_corpus(&mut self) {
        if self.corpus.as_ref().is_none_or(|(when, _)| when.elapsed().as_secs() > 20) {
            let mut files = Vec::new();
            for (folder, end) in [(self.session.library().films(), "film"), (self.session.library().decors(), "decor")] {
                for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|own| own == end)
                        && let Ok(text) = std::fs::read_to_string(&path)
                    {
                        files.push((path, text));
                    }
                }
            }
            self.index.clear();
            for (at, (_, text)) in files.iter().enumerate() {
                for piece in text.split(|letter: char| !(letter.is_alphanumeric() || letter == '_')).filter(|piece| !piece.is_empty()) {
                    self.index.entry(film::words::plain(piece)).or_default().insert(at);
                }
            }
            self.corpus = Some((Instant::now(), files));
        }
    }

    /// The films and decors a word is in (as a whole word).
    fn used_in(&mut self, word: &str) -> Vec<PathBuf> {
        self.read_corpus();
        let Some((_, files)) = &self.corpus else { return Vec::new() };
        self.index.get(&film::words::plain(word)).map(|found| found.iter().map(|at| files[*at].0.clone()).collect()).unwrap_or_default()
    }

    /// How many films use a word.
    fn films_using(&self, word: &str) -> usize {
        let Some((_, files)) = &self.corpus else { return 0 };
        self.index
            .get(&film::words::plain(word))
            .map_or(0, |found| found.iter().filter(|at| files[**at].0.extension().is_some_and(|own| own == "film")).count())
    }

    /// Chosen; heard at once if it is heard and that is asked.
    fn choose(&mut self, item: &Item) {
        let fresh = self.chosen.as_deref() != Some(item.key().as_str());
        self.chosen = Some(item.key());
        if fresh {
            (self.step, self.face, self.watching, self.body, self.renaming, self.trashing) = (0, 0, false, None, None, false);
            if self.auto_hear && item.shelf.heard() && self.heard.as_deref() != Some(item.key().as_str()) {
                self.hear(item);
            }
        }
    }
}

impl Resources {
    /// A making opened, shown on the whole window (the same word and kind already open: that one, shown).
    fn open_making(&mut self, maker: Maker) {
        let same = self.makings.iter().position(|open| open.kind() == maker.kind() && open.word() == maker.word() && open.saved());
        self.making_at = same.unwrap_or_else(|| {
            self.makings.push(maker);
            self.makings.len() - 1
        });
        self.making_full = true;
        self.closing = None;
    }

    /// Something is being made, shown on the whole window.
    pub fn creating(&self) -> bool {
        self.making_full && !self.makings.is_empty()
    }

    fn close_making(&mut self, at: usize) {
        if at < self.makings.len() {
            self.makings.remove(at);
        }
        self.making_at = self.making_at.min(self.makings.len().saturating_sub(1));
        self.closing = None;
        if self.makings.is_empty() {
            self.making_full = false;
            self.scroll_wanted = true;
        }
    }
}
