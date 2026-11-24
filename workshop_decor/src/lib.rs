//! The Decor space: the decor open (its text the one truth, each change
//! undone in one step, written by itself a moment after, baked for the
//! films in the background), what is drawn of it (each piece as laid, those
//! who live there), and the Library (the pieces to lay, place by
//! place; the things of the scenes; the models). The Scene: `scene`; the
//! Hierarchy and the Inspector: `panels`; the brushes: `terrains`; the
//! text changed by what is done: `edit`.
//!
//! A crate of Dungeon Workshop: the decor open (`Decors`), its Scene (`scene`),
//! its panels (`panels`), brushes (`terrains`), the Library (`library`), where
//! things land (`placing`); inside, its text changed by what is done (`edit`),
//! a Pokémon walked in it (`walk`), the decors' tool on its thread
//! (`workshop`). It uses `workshop_base` (the session, the look, safe saves)
//! and `decor` (the decors' language and tool).

mod edit;
pub mod library;
pub mod panels;
pub mod placing;
pub mod scene;
pub mod terrains;
mod walk;
mod workshop;

use std::sync::Arc;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use workshop_base::session::Session;

use decor::doc::Pose;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Vec2, pos2, vec2};

use crate::edit::{Item, Kind};
use crate::terrains::Brushes;
use crate::workshop::{Answer, Art, Ask, Body, Picture, PieceInfo, Said, Workshop};
use workshop_base::theme;

/// How long after a change it is written out (and baked for the films).
const WRITTEN_AFTER: Duration = Duration::from_millis(800);
/// How often a living decor is drawn again.
const FRAME_EVERY: Duration = Duration::from_millis(33);

/// The decor open.
pub struct Open {
    pub path: PathBuf,
    pub text: String,
    saved: String,
    /// Its text's revision; the one asked baked; the one baked.
    revision: u64,
    asked: u64,
    pub baked: u64,
    /// The texts before each change, and what each change did; those undone.
    undo: Vec<(String, String)>,
    redo: Vec<(String, String)>,
    changed: Option<Instant>,
    /// What baking it said.
    pub said: Vec<Said>,
    /// The line looked at (from 1), and which of its corners; the others chosen with it.
    pub chosen: Option<(usize, usize)>,
    pub more: Vec<(usize, usize)>,
    /// How close (none: all of it in sight), how far moved.
    pub scale: Option<f32>,
    pub pan: Vec2,
    pub seen_scale: f32,
    /// The point of the decor in the middle of the scene: where a piece clicked goes.
    pub middle: [f32; 2],
    /// What is chosen being carried: from where the pointer took it, by how much.
    pub carried: Option<Carry>,
    /// A rectangle being drawn to choose: from that point of the decor.
    pub marquee: Option<[f32; 2]>,
    /// How large the scene is on the screen.
    pub view: Vec2,
}

/// What is chosen, carried: from where the pointer took it, by how much (on the grid if asked).
#[derive(Clone, Copy, Debug)]
pub struct Carry {
    pub from: [f32; 2],
    pub by: [f32; 2],
}

impl Open {
    fn read(session: &Session, path: PathBuf) -> Open {
        workshop_base::safe::check(session, &path);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let mut open = Open {
            path,
            saved: text.clone(),
            text,
            revision: 1,
            asked: 0,
            baked: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            changed: None,
            said: Vec::new(),
            chosen: None,
            more: Vec::new(),
            scale: None,
            pan: Vec2::ZERO,
            seen_scale: 1.0,
            middle: [128.0, 96.0],
            carried: None,
            marquee: None,
            view: vec2(800.0, 600.0),
        };
        // (`WORKSHOP_VIEW="x y zoom"`: that point of the decor in the middle, so close; for a picture.)
        let asked: Vec<f32> = session.launch.view.clone().unwrap_or_default().split_whitespace().filter_map(|word| word.parse().ok()).collect();
        if let [x, y, close] = asked[..] {
            let [w, h] = open.size();
            (open.scale, open.pan) = (Some(close), vec2(w / 2.0 - x, h / 2.0 - y) * close);
        }
        open
    }

    /// Its text made this: one change, to undo.
    pub fn change(&mut self, made: String) {
        if made == self.text {
            return;
        }
        let said = crate::edit::described(&self.text, &made);
        // (The same thing moved or changed again within a second (a value dragged, an arrow held): one step.)
        let again = self.undo.last().is_some_and(|(_, last)| merges(last, &said))
            && self.changed.is_some_and(|when| when.elapsed() < Duration::from_secs(1))
            && self.redo.is_empty();
        if again {
            self.text = made;
        } else {
            self.undo.push((std::mem::replace(&mut self.text, made), said));
        }
        self.redo.clear();
        self.revision += 1;
        self.changed = Some(Instant::now());
    }

    pub fn undo(&mut self) {
        if let Some((before, said)) = self.undo.pop() {
            self.redo.push((std::mem::replace(&mut self.text, before), said));
            self.revision += 1;
            self.changed = Some(Instant::now());
        }
    }

    pub fn redo(&mut self) {
        if let Some((after, said)) = self.redo.pop() {
            self.undo.push((std::mem::replace(&mut self.text, after), said));
            self.revision += 1;
            self.changed = Some(Instant::now());
        }
    }

    /// What was done, the first first; and what was undone, the next first.
    pub fn history(&self) -> (Vec<String>, Vec<String>) {
        (self.undo.iter().map(|(_, said)| said.clone()).collect(), self.redo.iter().rev().map(|(_, said)| said.clone()).collect())
    }

    pub fn saved(&self) -> bool {
        self.text == self.saved
    }

    /// Written out now.
    pub fn save(&mut self) -> Result<(), String> {
        workshop_base::safe::write(&self.path, &self.text).map_err(|why| format!("{}: {why}", self.path.display()))?;
        self.saved = self.text.clone();
        Ok(())
    }

    /// Its size, as its first line says it (`decor: name 512 384`).
    pub fn size(&self) -> [f32; 2] {
        self.text
            .lines()
            .find_map(|line| {
                let words: Vec<&str> = line.split_whitespace().collect();
                match words.as_slice() {
                    ["decor:", _, wide, high, ..] => Some([wide.parse().ok()?, high.parse().ok()?]),
                    _ => None,
                }
            })
            .unwrap_or([512.0, 384.0])
    }

    /// Whether it is baked as it is now (for the films).
    pub fn is_baked(&self) -> bool {
        self.baked == self.revision
    }

    /// Its file's name, as the decors' tool knows it.
    fn name(&self) -> String {
        self.path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
    }

    /// Line `number` (from 1) made another, or taken out.
    pub(crate) fn with_line(&self, number: usize, made: Option<String>) -> String {
        let mut lines: Vec<String> = self.text.lines().map(str::to_string).collect();
        if number == 0 || number > lines.len() {
            return self.text.clone();
        }
        match made {
            Some(made) => lines[number - 1] = made,
            None => _ = lines.remove(number - 1),
        }
        lines.join("\n") + "\n"
    }

    /// A line put at the end of what is seen (before `[vie]`), or of the life of the place.
    pub(crate) fn with_added(&self, line: &str) -> String {
        crate::edit::with_line_added(&self.text, line, crate::edit::is_life(line))
    }

    /// That part of the decor in the middle of the scene, close enough to see it well.
    pub fn frame_on(&mut self, area: Rect) {
        let size = Vec2::from(self.size());
        let scale = (self.view.x / (area.width().max(24.0) * 3.0)).min(self.view.y / (area.height().max(24.0) * 3.0)).clamp(0.25, 6.0);
        self.scale = Some(scale);
        self.pan = (size / 2.0 - area.center().to_vec2()) * scale;
    }

    /// All that is chosen: the one looked at first.
    pub fn selection(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = self.chosen.into_iter().collect();
        for one in &self.more {
            if !out.contains(one) {
                out.push(*one);
            }
        }
        out
    }

    /// Those chosen, the first looked at.
    pub fn choose(&mut self, chosen: Vec<(usize, usize)>) {
        self.chosen = chosen.first().copied();
        self.more = chosen.into_iter().skip(1).collect();
    }

    /// The pose of line `number`.
    pub(crate) fn pose(&self, number: usize) -> Option<Pose> {
        self.text.lines().nth(number.checked_sub(1)?).and_then(Pose::read)
    }

    /// Line `number` moved one step before (later: it shows over) or after.
    pub(crate) fn moved_line(&self, number: usize, later: bool) -> String {
        let mut lines: Vec<String> = self.text.lines().map(str::to_string).collect();
        let at = number - 1;
        let to = if later { at + 1 } else { at.wrapping_sub(1) };
        if to < lines.len() {
            lines.swap(at, to);
        }
        lines.join("\n") + "\n"
    }
}

/// Whether a change goes with the one before it into one step: the same
/// thing moved or changed again; never a thing added or taken out (each
/// piece stamped, each one removed, is a step of its own).
pub(crate) fn merges(last: &str, said: &str) -> bool {
    last == said && (said.starts_with("moved:") || said.starts_with("changed:"))
}

/// One thing to draw: a piece somewhere, which of its layers.
pub struct Drawn {
    pub art: PieceArt,
    pub rect: Rect,
    pub layers: (bool, bool),
    pub turned: bool,
    /// How much it is seen (its layer's opacity).
    pub tint: Color32,
}

/// A piece as the editor draws it: its pictures on the lower layer and on
/// the upper one, frame by frame, how long a frame lasts.
#[derive(Clone)]
pub struct PieceArt {
    pub lower: Vec<egui::TextureHandle>,
    pub upper: Vec<egui::TextureHandle>,
    pub seconds: f64,
}

impl PieceArt {
    /// The frame it is at, so many seconds in.
    pub(crate) fn frame(&self, seconds: f64) -> usize {
        let count = self.lower.len().max(1);
        if self.seconds <= 0.0 { 0 } else { (seconds / self.seconds) as usize % count }
    }
}

/// Someone who lives there, as the editor draws him: for each way he faces,
/// his steps (a picture, its 60ths); where his feet are from a picture's middle.
#[derive(Clone)]
pub struct BodyArt {
    pub ways: Vec<Vec<(egui::TextureHandle, u8)>>,
    pub shadow: (i32, i32),
    /// For each way, where it is seen (its pixels not see-through) in any of
    /// its steps, from his feet: his box, tight and still, as he waits.
    pub seen: Vec<Option<Rect>>,
}

impl BodyArt {
    /// The picture of that way, so many seconds in, and where it is from his feet.
    pub(crate) fn at(&self, way: usize, seconds: f64) -> Option<(&egui::TextureHandle, Rect)> {
        let steps = self.ways.get(way).filter(|steps| !steps.is_empty()).or_else(|| self.ways.iter().find(|steps| !steps.is_empty()))?;
        let total: u64 = steps.iter().map(|(_, ticks)| u64::from((*ticks).max(1))).sum();
        let mut left = (seconds * 60.0) as u64 % total.max(1);
        let mut step = &steps[0];
        for one in steps {
            let ticks = u64::from(one.1.max(1));
            if left < ticks {
                step = one;
                break;
            }
            left -= ticks;
        }
        let size = step.0.size_vec2();
        let middle = pos2(-self.shadow.0 as f32, -self.shadow.1 as f32);
        Some((&step.0, Rect::from_center_size(middle, size)))
    }
}

/// A Pokémon's animations: each its number, its film name, its own.
pub type Anims = Vec<(u8, String, String)>;

/// What the scene's menu was opened on (a thing, or none), and where.
pub type SceneMenu = (Option<(usize, usize)>, [f32; 2]);

/// All the Decor space works with.
pub struct Decors {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    pub open: Option<Open>,
    workshop: Workshop,
    pub pieces: Vec<PieceInfo>,
    by_name: HashMap<String, usize>,
    arts: HashMap<String, Option<PieceArt>>,
    asked: HashSet<String>,
    bodies: HashMap<String, Option<BodyArt>>,
    /// Each Pokémon's animations (none yet: asked).
    anims: HashMap<String, Option<Anims>>,
    /// The Library: where one looks, what is sought, how it is shown, the favourites.
    pub lib: crate::library::Library,
    /// The decor living (its animations played).
    pub living: bool,
    /// The brushes: which tool the mouse is, the terrains, the semis.
    pub brushes: Brushes,
    /// The layers hidden, locked (`edit::LAYERS`); the grid shown.
    pub hidden: HashSet<&'static str>,
    pub locked: HashSet<&'static str>,
    /// The layer what is laid goes on (`sol`, `objets`, `dessus`); one shown alone;
    /// how much each is seen (1: wholly); the one the Hierarchy lists alone.
    pub active_layer: &'static str,
    pub solo: Option<&'static str>,
    pub opacity: HashMap<&'static str, f32>,
    pub layer_filter: Option<&'static str>,
    pub grid: bool,
    /// What was copied, as lines from 0 0.
    pub clipboard: Vec<String>,
    /// Where one does not walk, as the decor was baked last (a pixel a tile).
    pub walls: Option<egui::TextureHandle>,
    /// The models there are (read again when their source is opened); one's name being typed.
    pub models: Option<Vec<(String, Vec<String>)>>,
    pub model_name: String,
    /// Someone to add, by his word.
    pub resident: String,
    /// What was said of the last model kept (or group made).
    pub model_said: Option<String>,
    /// The groups folded in the Hierarchy; one's name being typed.
    pub folded: HashSet<String>,
    pub group_name: String,
    /// The Hierarchy: what is sought there, the things under the pointer
    /// there (shown on the scene), the choice it last saw, what it refused.
    pub hier_sought: String,
    pub hier_hovered: Vec<(usize, usize)>,
    pub hier_seen: Vec<(usize, usize)>,
    pub hier_said: Option<String>,
    /// Its sections open (Layers, Elements); its search shown, to be focused;
    /// a line being renamed (its number, the name typed); whether it has the keys.
    pub sections: (bool, bool),
    pub search_shown: bool,
    pub search_focus: bool,
    pub renaming: Option<(usize, String)>,
    pub hier_focus: bool,
    /// The Inspector: its components folded; the line written by hand (its
    /// number, as typed, as it was), what reading it said; the size and grid typed.
    pub folded_parts: HashSet<String>,
    pub raw_line: Option<(usize, String, String)>,
    pub raw_said: Option<String>,
    pub size_edit: Option<[i64; 2]>,
    /// The scene: its rulers shown; what its menu was opened on (a thing, or none) and where.
    pub rulers: bool,
    pub scene_menu: Option<SceneMenu>,
    /// The walls as baked last, by tiles; someone walked to try them, by his word.
    pub walls_grid: Option<crate::workshop::Walls>,
    pub walker: Option<crate::walk::Walker>,
    pub walker_word: String,
    /// The backdrops there are (places of the game without their elements).
    pub backdrops: Vec<crate::workshop::BackdropInfo>,
    /// What is laid lands on the decor's grid (S; Alt held: the other way).
    pub snap: bool,
    /// The stamp: a piece of the Library laid at each click on the scene, until Esc.
    pub stamp: Option<crate::placing::Stamp>,
}

impl Decors {
    pub fn new(session: &Arc<Session>, context: &egui::Context) -> Decors {
        let waker = context.clone();
        Decors {
            session: session.clone(),
            open: None,
            workshop: Workshop::start(session, move || waker.request_repaint()),
            pieces: Vec::new(),
            by_name: HashMap::new(),
            arts: HashMap::new(),
            asked: HashSet::new(),
            bodies: HashMap::new(),
            anims: HashMap::new(),
            lib: crate::library::Library::asked(session.launch.library.as_deref()),
            living: true,
            brushes: Brushes::default(),
            hidden: HashSet::from(["walls"]),
            locked: HashSet::new(),
            active_layer: "objects",
            solo: None,
            opacity: HashMap::new(),
            layer_filter: None,
            grid: false,
            clipboard: Vec::new(),
            walls: None,
            models: None,
            model_name: String::new(),
            resident: "pikachu".to_string(),
            model_said: None,
            folded: HashSet::new(),
            group_name: String::new(),
            hier_sought: String::new(),
            hier_hovered: Vec::new(),
            hier_seen: Vec::new(),
            hier_said: None,
            sections: (true, true),
            search_shown: false,
            search_focus: false,
            renaming: None,
            hier_focus: false,
            folded_parts: HashSet::from(["Line".to_string()]),
            raw_line: None,
            raw_said: None,
            size_edit: None,
            rulers: true,
            scene_menu: None,
            walls_grid: None,
            walker: None,
            walker_word: "pikachu".to_string(),
            backdrops: Vec::new(),
            snap: false,
            stamp: None,
        }
    }

    /// Whether a layer is hidden (by its eye, or another shown alone).
    pub fn is_hidden(&self, layer: &str) -> bool {
        self.solo.map_or_else(|| self.hidden.contains(layer), |alone| alone != layer)
    }

    /// The tint a layer is drawn with: how much it is seen.
    pub fn tint_of(&self, layer: &str) -> Color32 {
        let seen = self.opacity.get(layer).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        Color32::from_white_alpha((seen * 255.0).round() as u8)
    }

    /// The line laying a piece at these points, on the layer chosen.
    pub fn pose_line(&self, name: &str, points: &str) -> String {
        match self.active_layer {
            "ground" => format!("put {name} {points} layer ground"),
            "over" => format!("put {name} {points} layer over"),
            _ => format!("put {name} {points}"),
        }
    }

    /// Something asked of the workshop.
    pub(crate) fn ask(&self, ask: Ask) {
        self.workshop.ask(ask);
    }

    /// A piece as it is laid, asked if it is not there yet.
    pub fn art_now(&mut self, name: &str) -> Option<PieceArt> {
        self.art(name)
    }

    /// The animations a Pokémon has, asked if not yet.
    pub(crate) fn anims_now(&mut self, word: &str) -> Option<Anims> {
        match self.anims.get(word) {
            Some(made) => made.clone(),
            None => {
                self.anims.insert(word.to_string(), None);
                self.workshop.ask(Ask::Anims(word.to_string()));
                None
            }
        }
    }

    /// Someone who lives there, as drawn (asked if not yet).
    pub(crate) fn body_now(&mut self, word: &str) -> Option<BodyArt> {
        self.body(word)
    }

    /// A piece's art if it is there (never asked).
    pub(crate) fn art_known(&self, name: &str) -> Option<PieceArt> {
        self.arts.get(name).cloned().flatten()
    }

    /// Someone's art if it is there; whether it was answered.
    pub(crate) fn body_known(&self, word: &str) -> Option<Option<BodyArt>> {
        self.bodies.get(word).cloned()
    }

    /// That decor open (if it is not yet).
    pub fn open(&mut self, path: &PathBuf) {
        if self.open.as_ref().is_none_or(|open| &open.path != path) {
            self.open = Some(Open::read(&self.session, path.clone()));
        }
    }

    /// Each frame: what the workshop answered taken; the decor written out
    /// a moment after its last change, then baked for the films; drawn
    /// again while it lives.
    pub fn tick(&mut self, context: &egui::Context) -> Option<String> {
        let mut told = None;
        while let Ok(answer) = self.workshop.answers.try_recv() {
            match answer {
                Answer::Pieces(pieces) => {
                    // (A twin's name, as old decors lay it, is the element it is the same as.)
                    self.by_name = pieces
                        .iter()
                        .enumerate()
                        .flat_map(|(at, piece)| std::iter::once(&piece.name).chain(&piece.twins).map(move |name| (name.clone(), at)))
                        .collect();
                    self.pieces = pieces;
                }
                Answer::Backdrops(backdrops) => self.backdrops = backdrops,
                Answer::Baked { revision, said, walls } => {
                    if let Some(open) = &mut self.open {
                        open.baked = revision;
                        open.said = said;
                    }
                    if let Some(walls) = walls {
                        let rgba = walls.tiles.iter().flat_map(|wall| if *wall { [0xE0, 0x30, 0x30, 0x78] } else { [0, 0, 0, 0] }).collect();
                        let picture = Picture { w: walls.columns.max(1), h: walls.rows.max(1), rgba };
                        if picture.rgba.len() == picture.w * picture.h * 4 {
                            self.walls = Some(texture(context, "decor walls", &picture, self.walls.take()));
                        }
                        self.walls_grid = Some(walls);
                    }
                }
                Answer::Art(name, art) => {
                    let made = art.map(|art| made_art(context, &name, &art));
                    self.arts.insert(name, made);
                }
                Answer::Body(word, body) => {
                    let made = body.map(|body| made_body(context, &word, body));
                    self.bodies.insert(word, made);
                }
                Answer::Anims(word, anims) => {
                    self.anims.insert(word, Some(anims));
                }
                Answer::Tileset(level, terrains) => crate::terrains::tileset_answered(context, self, level, terrains),
                Answer::Terrain(line, solved) => crate::terrains::terrain_answered(context, self, line, solved),
                Answer::PlaceGround(level, line) => crate::terrains::ground_answered(self, level, line),
            }
        }
        let Some(open) = &mut self.open else {
            return None;
        };
        if open.changed.is_some_and(|changed| changed.elapsed() >= WRITTEN_AFTER) {
            open.changed = None;
            if let Err(why) = open.save() {
                told = Some(why);
            }
        }
        // (Baked once written, nothing being carried.)
        if open.asked != open.revision && open.changed.is_none() && open.carried.is_none() {
            self.workshop.ask(Ask::Bake { revision: open.revision, name: open.name(), text: open.text.clone() });
            open.asked = open.revision;
        }
        if self.living {
            context.request_repaint_after(FRAME_EVERY);
        } else if open.changed.is_some() {
            context.request_repaint_after(Duration::from_millis(200));
        }
        told
    }

    /// A piece as it is laid, asked if it is not there yet.
    fn art(&mut self, name: &str) -> Option<PieceArt> {
        match self.arts.get(name) {
            Some(made) => made.clone(),
            None => {
                if self.asked.insert(name.to_string()) {
                    self.workshop.ask(Ask::Art(name.to_string()));
                }
                None
            }
        }
    }

    /// Someone who lives there, asked if he is not there yet.
    fn body(&mut self, word: &str) -> Option<BodyArt> {
        match self.bodies.get(word) {
            Some(made) => made.clone(),
            None => {
                if self.asked.insert(format!("@{word}")) {
                    self.workshop.ask(Ask::Body(word.to_string()));
                }
                None
            }
        }
    }

    fn piece(&self, name: &str) -> Option<&PieceInfo> {
        self.by_name.get(name).map(|at| &self.pieces[*at])
    }

    /// How large an element is laid: its pictures' size, else its piece's.
    pub(crate) fn size_of(&self, name: &str) -> [f32; 2] {
        if let Some(Some(art)) = self.arts.get(name)
            && let Some(first) = art.lower.first()
        {
            return [first.size_vec2().x, first.size_vec2().y];
        }
        self.piece(name).map_or([24.0, 24.0], |piece| piece.size)
    }

    /// Where a thing shows, in the decor's pixels.
    pub(crate) fn item_box(&self, one: &Item) -> Rect {
        let [x, y] = one.corner;
        match one.kind {
            Kind::Spot | Kind::Entrance => return Rect::from_center_size(pos2(x, y), vec2(12.0, 12.0)),
            Kind::Exit | Kind::Zone => return Rect::from_min_size(pos2(x, y), vec2(one.size[0].max(4.0), one.size[1].max(4.0))),
            Kind::Piece | Kind::Lives => {}
        }
        if one.kind == Kind::Lives {
            // (As his sprite stands, still: its first step.)
            if let Some(Some(body)) = self.bodies.get(&body_key(one))
                && let Some((_, around)) = body.at(one.facing, 0.0)
            {
                let way = if body.ways.get(one.facing).is_some_and(|steps| !steps.is_empty()) {
                    one.facing
                } else {
                    body.ways.iter().position(|steps| !steps.is_empty()).unwrap_or(0)
                };
                let tight = body.seen.get(way).copied().flatten().unwrap_or(around);
                return tight.translate(vec2(x, y));
            }
            return Rect::from_min_max(pos2(x - 10.0, y - 26.0), pos2(x + 10.0, y));
        }
        let [w, h] = self.size_of(&one.name);
        Rect::from_min_size(pos2(x, y), vec2(w, h))
    }

    /// The name a thing is shown by.
    pub fn item_name(&self, one: &Item) -> String {
        match one.kind {
            Kind::Piece => self.shown_name(&one.name),
            Kind::Lives => format!("Resident · {}", one.name),
            Kind::Spot => format!("Spot · {}", one.name),
            Kind::Entrance => format!("Entrance · {}", if one.name.is_empty() { "unnamed" } else { &one.name }),
            Kind::Exit => format!("Exit · {}", one.name),
            Kind::Zone => format!("Zone · {}", one.name),
        }
    }

    /// The name an element is shown by: its place and what it is.
    pub fn shown_name(&self, name: &str) -> String {
        match self.piece(name) {
            // (As our eyes saw it, and where it comes from.)
            Some(piece) if piece.label.is_some() => {
                let title = crate::library::title_of(piece);
                piece.place.as_ref().map_or(title.clone(), |(_, place)| format!("{title} · {place}"))
            }
            Some(PieceInfo { place: Some((_, place)), kind, .. }) => {
                let number = name.rsplit('_').next().unwrap_or_default();
                let what = if *kind == 'a' { "animated" } else { "object" };
                format!("{place} · {what} {number}")
            }
            Some(PieceInfo { place: None, kind: 'o', .. }) => format!("scene object {}", name.trim_start_matches('o')),
            // (A place's rest: by the place, as its pieces name it.)
            _ if name.strip_prefix('r').is_some_and(|level| !level.is_empty() && level.bytes().all(|byte| byte.is_ascii_digit())) => {
                let level: i64 = name[1..].parse().unwrap_or_default();
                let place = self.pieces.iter().find_map(|piece| piece.place.as_ref().filter(|(known, _)| *known == level).map(|(_, place)| place.clone()));
                format!("{} · rest of the place", place.unwrap_or_else(|| format!("place {level}")))
            }
            _ => name.to_string(),
        }
    }
}

/// A piece's pictures made textures.
pub(crate) fn made_art(context: &egui::Context, name: &str, art: &Art) -> PieceArt {
    let made = |layer: &str, pictures: &[Picture]| -> Vec<egui::TextureHandle> {
        pictures.iter().enumerate().map(|(at, picture)| texture(context, &format!("{name} {layer} {at}"), picture, None)).collect()
    };
    PieceArt { lower: made("lower", &art.lower), upper: made("upper", &art.upper), seconds: art.seconds }
}

/// Someone's pictures made textures.
pub(crate) fn made_body(context: &egui::Context, word: &str, body: Body) -> BodyArt {
    let ways = body
        .ways
        .iter()
        .enumerate()
        .map(|(way, steps)| {
            steps.iter().enumerate().map(|(at, (picture, ticks))| (texture(context, &format!("{word} {way} {at}"), picture, None), *ticks)).collect()
        })
        .collect();
    // (Where each way shows something, over all its steps: his box, tight, as in the films.)
    let middle = pos2(-body.shadow.0 as f32, -body.shadow.1 as f32);
    let seen = body
        .ways
        .iter()
        .map(|steps| {
            let mut bounds: Option<Rect> = None;
            for (picture, _) in steps {
                let corner = Rect::from_center_size(middle, vec2(picture.w as f32, picture.h as f32)).min;
                for (at, pixel) in picture.rgba.as_chunks::<4>().0.iter().enumerate() {
                    if pixel[3] >= 200 {
                        let one = Rect::from_min_size(corner + vec2((at % picture.w) as f32, (at / picture.w) as f32), vec2(1.0, 1.0));
                        bounds = Some(bounds.map_or(one, |known| known.union(one)));
                    }
                }
            }
            bounds
        })
        .collect();
    BodyArt { ways, shadow: body.shadow, seen }
}

/// A piece drawn in a rectangle of the screen: its lower layer, its upper
/// one, or both, at the frame it is at; turned over or not.
pub(crate) fn paint_art(painter: &egui::Painter, art: &PieceArt, rect: Rect, layers: (bool, bool), frame: usize, turned: bool, tint: Color32) {
    let uv = if turned { Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0)) } else { Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)) };
    if layers.0
        && let Some(lower) = art.lower.get(frame % art.lower.len().max(1))
    {
        painter.image(lower.id(), rect, uv, tint);
    }
    if layers.1
        && let Some(upper) = art.upper.get(frame % art.upper.len().max(1))
    {
        painter.image(upper.id(), rect, uv, tint);
    }
}

/// A piece drawn whole in a square (a tile of the Library, the Inspector).
pub fn paint_thumb(painter: &egui::Painter, art: &PieceArt, room: Rect) {
    if let Some(first) = art.lower.first() {
        paint_art(painter, art, fitted(first.size_vec2(), room), (true, true), 0, false, Color32::WHITE);
    }
}

/// A picture made (or remade) a texture: pixels whole when larger, smoothed when smaller.
pub(crate) fn texture(context: &egui::Context, name: &str, picture: &Picture, before: Option<egui::TextureHandle>) -> egui::TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied([picture.w, picture.h], &picture.rgba);
    let options = egui::TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Linear,
        mipmap_mode: Some(egui::TextureFilter::Linear),
        ..egui::TextureOptions::NEAREST
    };
    match before {
        Some(mut texture) => {
            texture.set(image, options);
            texture
        }
        None => context.load_texture(name, image, options),
    }
}

/// A picture drawn whole in a square, its pixels whole if it is smaller.
fn fitted(size: Vec2, room: Rect) -> Rect {
    let scale = (room.width() / size.x.max(1.0)).min(room.height() / size.y.max(1.0));
    let scale = if scale >= 1.0 { scale.floor() } else { scale };
    Rect::from_center_size(room.center(), size * scale)
}

/// A label on a dark tag, its bottom left there.
pub fn tag(painter: &egui::Painter, at: Pos2, said: &str, colour: Color32) {
    let galley = painter.layout_no_wrap(said.to_string(), egui::FontId::proportional(theme::SMALL), colour);
    let room = Rect::from_min_size(at - vec2(0.0, galley.size().y + 4.0), galley.size() + vec2(8.0, 4.0));
    painter.rect_filled(room, 2.0, Color32::from_black_alpha(200));
    painter.galley(room.min + vec2(4.0, 2.0), galley, colour);
}

/// The decor seen whole again.
pub fn fit(decors: &mut Decors) {
    if let Some(open) = &mut decors.open {
        (open.scale, open.pan) = (None, Vec2::ZERO);
    }
}

/// A section's title, across the panel.
pub fn section(ui: &mut egui::Ui, title: &str) {
    egui::Frame::new().fill(theme::TOOLS).inner_margin(egui::Margin::symmetric(6, 3)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).strong());
    });
}

/// The sprite someone who lives there is drawn with: as he waits, or doing
/// his animation (`pikachu#23`).
pub(crate) fn body_key(one: &Item) -> String {
    match one.anim.as_deref().and_then(|name| scenes::vocabulary::POKEMON.animation(name)) {
        Some(group) => format!("{}#{group}", one.name),
        None => one.name.clone(),
    }
}

/// A decor's kind of place: its own line in `place_kinds.txt`, else the place of the game it is built on (`level`).
pub fn decor_kind(kinds: &decor::tags::PlaceKinds, name: &str, text: &str) -> decor::tags::PlaceKind {
    let level = text.lines().find_map(|line| line.trim().strip_prefix("level ").and_then(|level| level.trim().parse().ok()));
    kinds.of_decor(name, level)
}

/// A decor's name as it says it (`name …`), else its file's.
pub fn decor_title(name: &str, text: &str) -> String {
    text.lines()
        .find_map(|line| line.trim().strip_prefix("name ").map(|title| title.trim().to_string()))
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::merges;

    /// A piece moved or changed again goes into the same step; each piece laid (each stamp) or removed is a step of its own, undone one by one.
    #[test]
    fn what_is_laid_is_undone_one_by_one() {
        assert!(merges("moved: put e1_1", "moved: put e1_1"));
        assert!(merges("changed: put e1_1", "changed: put e1_1"));
        assert!(!merges("added: put e1_1", "added: put e1_1"));
        assert!(!merges("removed: put e1_1", "removed: put e1_1"));
        assert!(!merges("moved: put e1_1", "moved: put e1_2"));
    }
}
