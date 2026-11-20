//! The decors' workshop, on a thread of its own: each piece as it is laid
//! (its pictures, both its layers, for the editor to draw itself, at once);
//! a decor baked from its text for the films (the decors' tool, seconds on
//! a large place: never on the window's thread); the pieces there are.

use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, Sender, channel};

use decor::catalogue::Level;

/// A picture: its size, its pixels (RGBA).
#[derive(Clone)]
pub struct Picture {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
}

/// A piece there is to lay: its name, the place of the game it is of (its
/// number, its name), what it is (`e` an object of the upper layer, `a`
/// animated tiles, `o` a thing of the scenes), its size, whether the game
/// animates it.
#[derive(Clone, Debug)]
pub struct PieceInfo {
    pub name: String,
    pub place: Option<(i64, String)>,
    /// The kind of its place (`library/tables/place_kinds.txt`; Other for the scenes' things).
    pub place_kind: decor::tags::PlaceKind,
    pub kind: char,
    pub size: [f32; 2],
    pub moves: bool,
    /// The same to the pixel elsewhere (turned over or not), offered as this one: their names, the places they are in.
    pub twins: Vec<String>,
    pub also: Vec<String>,
    /// What our eyes say of it: its category, its name one reads, its tags.
    pub category: decor::tags::Category,
    pub label: Option<String>,
    pub tags: Vec<String>,
}

/// A backdrop a decor may be laid on: a place of the game without its
/// elements (its rest, `r<level>`, laid at 0 0), its name, its kind, its size.
#[derive(Clone, Debug)]
pub struct BackdropInfo {
    pub level: i64,
    pub place: String,
    pub kind: decor::tags::PlaceKind,
    pub size: [f32; 2],
}

/// What is said of a decor built: on a line (0: the decor), wrong or remarked.
#[derive(Clone, Debug)]
pub struct Said {
    pub line: usize,
    pub text: String,
    pub wrong: bool,
}

/// A piece as it is laid: its pictures on the lower layer (under who
/// passes) and on the upper one (over), frame by frame, how long a frame lasts.
pub struct Art {
    pub lower: Vec<Picture>,
    pub upper: Vec<Picture>,
    pub seconds: f64,
}

/// Someone who lives in a decor, as the game shows him waiting: for each
/// way he faces (as `decor::lang::FACING` lists them), his steps, each a
/// picture and its 60ths of a second; where his feet are from a picture's middle.
pub struct Body {
    pub ways: Vec<Vec<(Picture, u8)>>,
    pub shadow: (i32, i32),
}

/// A terrain of a place, to paint with: its letter, its colour, how much
/// of the place it is, whether one walks on it, a patch of it (24 by 24).
#[derive(Clone)]
pub struct TerrainInfo {
    pub letter: char,
    pub colour: [u8; 3],
    pub share: f64,
    pub wall: bool,
    pub patch: Option<Picture>,
}

/// A terrain line solved: its tiles' frames, how long a frame lasts, where
/// one does not walk (red where a wall is).
pub struct Solved {
    pub art: Art,
    pub walls: Picture,
}

/// Where one does not walk in a decor baked: its tiles of 8, a row after another.
pub struct Walls {
    pub columns: usize,
    pub rows: usize,
    pub tiles: Vec<bool>,
}

/// What the window asks.
pub enum Ask {
    /// A decor baked from its text (its file's name), of that revision.
    Bake { revision: u64, name: String, text: String },
    /// A piece as it is laid.
    Art(String),
    /// A Pokémon as it waits, by its word (`pikachu`; `pikachu@walk` as it
    /// walks, `pikachu#23` doing that animation).
    Body(String),
    /// The animations a Pokémon has.
    Anims(String),
    /// A place's terrains, to paint with.
    Tileset(i64),
    /// A terrain line, its tiles solved (the line as written).
    Terrain(String),
    /// The same, its colours that change with time sampled too (slower: once it is painted).
    TerrainLiving(String),
    /// A whole place's own ground, as a terrain line at 0 0.
    PlaceGround(i64),
}

/// What the workshop answers.
pub enum Answer {
    Pieces(Vec<PieceInfo>),
    Backdrops(Vec<BackdropInfo>),
    Baked {
        revision: u64,
        said: Vec<Said>,
        walls: Option<Walls>,
    },
    Art(String, Option<Art>),
    Body(String, Option<Body>),
    /// Each animation a Pokémon has: its number, its name, its own.
    Anims(String, Vec<(u8, String, String)>),
    Tileset(i64, Vec<TerrainInfo>),
    Terrain(String, Option<Solved>),
    PlaceGround(i64, Option<String>),
}

/// The workshop's end of the line, from the window.
pub struct Workshop {
    asks: Sender<Ask>,
    pub answers: Receiver<Answer>,
}

impl Workshop {
    /// Started on its own thread; `woken` called when it has answered.
    pub fn start(session: &std::sync::Arc<crate::session::Session>, woken: impl Fn() + Send + 'static) -> Workshop {
        let (asks, asked) = channel::<Ask>();
        let (answer, answers) = channel::<Answer>();
        let session = session.clone();
        std::thread::spawn(move || {
            let send = |reply| {
                _ = answer.send(reply);
                woken();
            };
            work(&session, &asked, &send);
        });
        Workshop { asks, answers }
    }

    pub fn ask(&self, ask: Ask) {
        _ = self.asks.send(ask);
    }
}

/// The game's places by their numbers, as the game names them (`town.txt`);
/// its maps for trying (`T00…`) by their code.
fn place_names(data: &str) -> BTreeMap<i64, String> {
    let text = std::fs::read_to_string(format!("{data}/rom/town.txt")).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            (parts.next() == Some("place")).then_some(())?;
            let level = parts.next()?.parse().ok()?;
            let named = parts.next()?;
            let (said, code) = named.split_once(" (").unwrap_or((named, ""));
            let code = code.trim_end_matches(')');
            let name = if code.starts_with("T00") { format!("Test Map {code}") } else { said.trim().to_string() };
            Some((level, name))
        })
        .collect()
}

/// The pieces there are to lay: those the ROM itself tells apart, none set
/// aside (`decor sort`) but those almost like another; each with its twins
/// (the same to the pixel, turned over or not: not offered again) and the
/// places they are in; its category, its name and tags as our eyes saw it
/// (`library/tables/piece_tags.txt`).
fn pieces(workshop: &decor::weave::Workshop, data: &str) -> Vec<PieceInfo> {
    let tables = decor::pieces_folder(&workshop.project);
    let kinds = decor::tags::PlaceKinds::of(&tables);
    // (A place the game leaves unnamed, `P05P04A`: by our name for it.)
    let mut names = place_names(data);
    for (level, name) in names.iter_mut() {
        if let Some(ours) = kinds.names.get(&level.to_string()).filter(|_| !name.contains(' ')) {
            *name = ours.clone();
        }
    }
    let aside = decor::sort::set_aside(&tables);
    let twins = decor::sort::twins(&aside);
    let labels = decor::tags::labels(&tables);
    let place_of = |level: &Level| match level {
        Level::Place(level) => Some((*level, names.get(level).cloned().unwrap_or_else(|| format!("place {level}")))),
        Level::Thing(_) => None,
    };
    decor::offered(&workshop.catalogue, &aside)
        .into_iter()
        .map(|piece| {
            let place = place_of(&piece.level);
            let twins = twins.get(&piece.name).cloned().unwrap_or_default();
            let mut also: Vec<String> = twins
                .iter()
                .filter_map(|twin| workshop.catalogue.get(twin))
                .filter_map(|twin| place_of(&twin.level))
                .map(|(_, name)| name)
                .filter(|name| place.as_ref().is_none_or(|(_, own)| own != name))
                .collect();
            also.sort();
            also.dedup();
            let (x0, y0, x1, y1) = piece.rect;
            let moves = piece.has("moves");
            let label = decor::tags::label_of(&piece.name, &labels, &aside);
            PieceInfo {
                name: piece.name.clone(),
                place_kind: place.as_ref().map_or(decor::tags::PlaceKind::Other, |(level, _)| kinds.of_level(*level)),
                place,
                kind: piece.name.chars().next().unwrap_or('?'),
                size: [(x1 - x0) as f32, (y1 - y0) as f32],
                moves,
                twins,
                also,
                category: decor::tags::category_of(&piece.name, moves, label),
                label: label.map(|label| label.name.clone()),
                tags: label.map(|label| label.tags.clone()).unwrap_or_default(),
            }
        })
        .collect()
}

/// The backdrops there are, by kind of place, then by name: each place of the game whose rest the catalogue
/// has (`decor inventory`).
fn backdrops(workshop: &decor::weave::Workshop, data: &str) -> Vec<BackdropInfo> {
    let kinds = decor::tags::PlaceKinds::of(&decor::pieces_folder(&workshop.project));
    let names = place_names(data);
    let mut out: Vec<BackdropInfo> = workshop
        .catalogue
        .pieces
        .iter()
        .filter_map(|piece| {
            let level = crate::edit::rest_level(&piece.name)?;
            let (x0, y0, x1, y1) = piece.rect;
            // (A place the game leaves unnamed, `P05P04A`: by our name for it, as the pieces say it.)
            let ours = kinds.names.get(&level.to_string());
            let place = match (names.get(&level), ours) {
                (Some(game), Some(ours)) if !game.contains(' ') => ours.clone(),
                (Some(game), _) => game.clone(),
                (None, Some(ours)) => ours.clone(),
                (None, None) => format!("place {level}"),
            };
            Some(BackdropInfo { level, place, kind: kinds.of_level(level), size: [(x1 - x0) as f32, (y1 - y0) as f32] })
        })
        .collect();
    out.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.place.cmp(&b.place)).then(a.level.cmp(&b.level)));
    out
}

/// A monster's sprite has five animations of its own first (walking,
/// standing, hurt, asleep, attacking); the game's animation n is its file n + 5
/// (`stage_view::stage::SCENE_ANIMATIONS`).
const SCENE_ANIMATIONS: u8 = 5;

/// The sprites' sheets, by Pokémon and animation.
type Sheets = std::collections::HashMap<(u16, u8), shell::art_files::Sheet>;

/// The most pictures a piece is drawn with (its frames and its colours that
/// change with time, sampled together).
const MOST_SAMPLES: u64 = 48;

/// A piece as it is laid: each of its frames, both its layers (the upper
/// one left out where it shows nothing). Its colours that change with time
/// (water shimmering) too: the piece laid alone, sampled as often as its
/// fastest change, over the time its frames and colours take to come back.
fn art(workshop: &decor::weave::Workshop, name: &str) -> Option<Art> {
    let cut = decor::laid_cut(workshop, name).ok()?;
    let picture = |frame: &decor::pixels::Picture| Picture { w: frame.w, h: frame.h, rgba: frame.cells.iter().flatten().copied().collect() };
    let shows = |frames: &[decor::pixels::Picture]| frames.iter().any(|frame| frame.cells.iter().any(|rgba| rgba[3] > 0));
    let plain = || {
        let upper = if shows(&cut.upper) { cut.upper.iter().map(picture).collect() } else { Vec::new() };
        Some(Art { lower: cut.lower.iter().map(picture).collect(), upper, seconds: cut.seconds })
    };
    if cut.lower_index.is_none() && cut.upper_index.is_none() {
        return plain();
    }
    let (w, h) = cut.size();
    sampled(workshop, &format!("put {name} 0 0"), (w, h)).or_else(plain)
}

/// One line laid alone at 0 0, sampled as often as its fastest change
/// (its frames, its colours that change with time) over the time all take
/// to come back; none if no colour of it changes.
fn sampled(workshop: &decor::weave::Workshop, line: &str, (w, h): (usize, usize)) -> Option<Art> {
    let text = format!("decor: piece {} {}\n{line}\n", w.next_multiple_of(8).max(16), h.next_multiple_of(8).max(16));
    let place = decor::read("piece", &text).and_then(|read| decor::built(workshop, read)).ok()?;
    if place.cycles.is_empty() {
        return None;
    }
    // (In 60ths: a frame, each palette's step; how often, how long till all come back.)
    let gcd = |mut a: u64, mut b: u64| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let frame_ticks = ((place.seconds * 60.0).round() as u64).max(1);
    let mut every = if place.lower.len() > 1 { frame_ticks } else { u64::MAX };
    let mut period = if place.lower.len() > 1 { frame_ticks * place.lower.len() as u64 } else { 1 };
    for (_, (ticks, steps)) in &place.cycles {
        let ticks = u64::from(*ticks).max(1);
        every = every.min(ticks);
        let whole = ticks * steps.len().max(1) as u64;
        period = period / gcd(period, whole) * whole;
    }
    let every = every.max(1);
    let count = (period / every).clamp(1, MOST_SAMPLES);
    let (mut lower, mut upper) = (Vec::new(), Vec::new());
    for sample in 0..count {
        let (full_w, full_h, below, above) = place.layers_at((sample * every) as f64 / 60.0);
        let (w, h) = (full_w.min(w), full_h.min(h));
        let crop = |rgba: &[u8], full: usize| -> Vec<u8> { (0..h).flat_map(|y| rgba[y * full * 4..(y * full + w) * 4].iter().copied()).collect() };
        let full = place.lower[0].w;
        lower.push(Picture { w, h, rgba: crop(&below, full) });
        upper.push(Picture { w, h, rgba: crop(&above, full) });
    }
    if !upper.iter().any(|picture| picture.rgba.chunks(4).any(|pixel| pixel[3] > 0)) {
        upper.clear();
    }
    Some(Art { lower, upper, seconds: every as f64 / 60.0 })
}

/// A place's terrains, each with a patch of it.
fn tileset(workshop: &decor::weave::Workshop, level: i64) -> Vec<TerrainInfo> {
    let Ok(set) = decor::terrain::tileset(workshop, level) else {
        return Vec::new();
    };
    set.terrains
        .iter()
        .enumerate()
        .map(|(at, terrain)| {
            let painted = decor::terrain::Painted { level, x: 0, y: 0, corners: decor::pixels::Grid::filled(4, 4, at as u8), walls: None };
            let cut = decor::terrain::cut_of(&set, &painted);
            let patch = cut.lower.first().map(picture_of);
            TerrainInfo { letter: terrain.letter, colour: terrain.colour, share: terrain.share, wall: terrain.wall, patch }
        })
        .collect()
}

fn picture_of(frame: &decor::pixels::Picture) -> Picture {
    Picture { w: frame.w, h: frame.h, rgba: frame.cells.iter().flatten().copied().collect() }
}

/// A terrain line solved: its frames (its colours that change seen as in
/// the first), its walls.
fn terrain(workshop: &decor::weave::Workshop, line: &str) -> Option<Solved> {
    let words: Vec<String> = line.split_whitespace().skip(1).map(str::to_string).collect();
    let painted = decor::terrain::Painted::read(&words).ok()?;
    let cut = decor::terrain::cut(workshop, &painted).ok()?;
    let walls = Picture {
        w: cut.solid.w,
        h: cut.solid.h,
        rgba: cut.solid.cells.iter().flat_map(|solid| if *solid { [0xE0, 0x30, 0x30, 0x70] } else { [0, 0, 0, 0] }).collect(),
    };
    Some(Solved { art: Art { lower: cut.lower.iter().map(picture_of).collect(), upper: Vec::new(), seconds: cut.seconds }, walls })
}

/// A terrain line's colours that change with time, sampled (the line laid at 0 0).
fn terrain_living(workshop: &decor::weave::Workshop, line: &str) -> Option<Solved> {
    let mut solved = terrain(workshop, line)?;
    let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    if words.len() > 3 {
        (words[2], words[3]) = ("0".to_string(), "0".to_string());
    }
    if let Some(art) = sampled(workshop, &words.join(" "), (solved.walls.w, solved.walls.h)) {
        solved.art.lower = art.lower;
        solved.art.seconds = art.seconds;
    }
    Some(solved)
}

/// A place's own ground as a terrain line, at 0 0.
fn place_ground(workshop: &decor::weave::Workshop, level: i64) -> Option<String> {
    let set = decor::terrain::tileset(workshop, level).ok()?;
    let (columns, rows) = (set.corners.w - 1, set.corners.h - 1);
    Some(decor::terrain::of_the_place(&set, 0, 0, (0, 0, columns, rows)).line())
}

/// A Pokémon as it waits (its sprite's first animation after standing: the
/// game's Idle), each way it faces.
fn body(data: &str, sheets: &Sheets, tables: &film::tables::Tables, word: &str) -> Option<Body> {
    // (`pikachu@marche`: as it walks, its sprite's first animation; `pikachu#23`:
    // the game's animation 23, its sprite's file after the five of every monster.)
    let (word, files) = match (word.split_once('@'), word.split_once('#')) {
        (Some((word, _)), _) => (word, vec![0u8, 1]),
        (_, Some((word, group))) => (word, vec![group.parse::<u8>().map_or(1, |group| group.saturating_add(SCENE_ANIMATIONS))]),
        _ => (word, vec![1u8, 0]),
    };
    let number: u16 = tables.monsters.get(word)?.0.parse().ok()?;
    let ways = files
        .iter()
        .find_map(|file| shell::art_files::animation(data, "sprites", sheets, number, *file).filter(|ways| ways.iter().any(|steps| !steps.is_empty())))?;
    let shadow = ways.iter().flatten().next().map_or((0, 0), |cell| cell.shadow);
    let ways = ways
        .into_iter()
        .map(|steps| steps.into_iter().map(|cell| (Picture { w: cell.wide as usize, h: cell.high as usize, rgba: cell.rgba }, cell.ticks)).collect())
        .collect();
    Some(Body { ways, shadow })
}

/// The workshop at work: what is asked, answered; of several bakes asked
/// while one was being made, only the last.
fn work(session: &crate::session::Session, asked: &Receiver<Ask>, send: &dyn Fn(Answer)) {
    let data = session.data().to_string();
    let workshop = match decor::workshop(session.library(), &data) {
        Ok(workshop) => workshop,
        Err(why) => {
            send(Answer::Baked { revision: 0, said: vec![Said { line: 0, text: why.to_string(), wrong: true }], walls: None });
            return;
        }
    };
    send(Answer::Pieces(pieces(&workshop, &data)));
    send(Answer::Backdrops(backdrops(&workshop, &data)));
    let extra = decor::write::extra_of(&data);
    // (The Pokémon's sprites and names, read the first time one is asked.)
    let mut bodies: Option<(Sheets, film::tables::Tables)> = None;
    while let Ok(first) = asked.recv() {
        // (Those waiting too: the pieces first (the editor waits on them), of
        // the bakes the last one only, of the terrains being painted too.)
        let mut waiting = vec![first];
        waiting.extend(asked.try_iter());
        waiting.sort_by_key(|ask| matches!(ask, Ask::Bake { .. }));
        let last_bake = waiting.iter().rposition(|ask| matches!(ask, Ask::Bake { .. }));
        let last_terrain = waiting.iter().rposition(|ask| matches!(ask, Ask::Terrain(_)));
        for (at, ask) in waiting.into_iter().enumerate() {
            match ask {
                Ask::Art(name) => {
                    let made = art(&workshop, &name);
                    send(Answer::Art(name, made));
                }
                Ask::Body(word) => {
                    let (sheets, tables) =
                        bodies.get_or_insert_with(|| (shell::art_files::sheets(&data), film::tables::Tables::read(session.library(), &data)));
                    let made = body(&data, sheets, tables, &word);
                    send(Answer::Body(word, made));
                }
                Ask::Bake { revision, name, text } if Some(at) == last_bake => {
                    let made = decor::read(&name, &text).and_then(|read| decor::built(&workshop, read));
                    let walls = made.as_ref().ok().map(|place| Walls { columns: place.walls.w, rows: place.walls.h, tiles: place.walls.cells.clone() });
                    let said = match made {
                        Ok(place) => {
                            let mut said: Vec<Said> =
                                place.remarks.iter().map(|remark| Said { line: remark.line(), text: remark.message.clone(), wrong: false }).collect();
                            if let Err(why) = decor::written(&workshop, &place, &extra) {
                                said.push(Said { line: 0, text: why.to_string(), wrong: true });
                            }
                            said
                        }
                        Err(why) => vec![Said { line: why.line(), text: why.message.clone(), wrong: true }],
                    };
                    send(Answer::Baked { revision, said, walls });
                }
                Ask::Bake { .. } => {}
                Ask::Anims(word) => {
                    let (sheets, tables) =
                        bodies.get_or_insert_with(|| (shell::art_files::sheets(&data), film::tables::Tables::read(session.library(), &data)));
                    let number: Option<u16> = tables.monsters.get(&word).and_then(|(number, _)| number.parse().ok());
                    let mut made: Vec<(u8, String, String)> = scenes::vocabulary::POKEMON
                        .animations
                        .iter()
                        .filter_map(|(own, group)| {
                            let group = u8::try_from(*group).ok()?;
                            sheets.contains_key(&(number?, group.saturating_add(SCENE_ANIMATIONS))).then(|| (group, own.replace('_', " "), own.to_string()))
                        })
                        .collect();
                    made.sort_by_key(|(group, ..)| *group);
                    send(Answer::Anims(word, made));
                }
                Ask::Tileset(level) => {
                    let made = tileset(&workshop, level);
                    send(Answer::Tileset(level, made));
                }
                Ask::Terrain(line) if Some(at) == last_terrain => {
                    let made = terrain(&workshop, &line);
                    send(Answer::Terrain(line, made));
                }
                // (A terrain painted further since: answered empty, the last one drawn.)
                Ask::Terrain(line) => send(Answer::Terrain(line, None)),
                Ask::TerrainLiving(line) => {
                    let made = terrain_living(&workshop, &line);
                    send(Answer::Terrain(line, made));
                }
                Ask::PlaceGround(level) => {
                    let made = place_ground(&workshop, level);
                    send(Answer::PlaceGround(level, made));
                }
            }
        }
    }
}
