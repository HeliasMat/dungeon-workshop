//! What shows of the game, read from its folder as it comes to be needed
//! and kept: sprites, things, effects, status icons, digits, tilesets, the
//! weather, places, faces. (What is heard is `sound`'s; the tables, what is
//! counted with, are `rules::run::Book`'s.)

use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use assets::project::Project;
use assets::tables::{EffectRow, Tips};

use crate::art_files::{Sheets, prop_sheets, sheets, status_icons};
use crate::data;

use crate::images::{Effect, Frame, Ground, Picture, Sprite, effect_of, ground_of, picture};

/// A weather that shows over a floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sky {
    Sandstorm,
    Fog,
    Rain,
}

/// (A tileset is `media`'s, read the same by the décors' tool.)
pub use media::tileset::{CHUNK, CHUNKS_IN_ROW, Tileset, read_tileset};

/// The first number of the tilesets of one's own: the game's stop before it.
pub const OWN_TILESETS: u8 = 200;

/// The pictures of a project's tilesets of one's own, in order: its own,
/// then the shared library's (`<word>.png`, their versions `<word>.1.png`
/// left out), each word once.
pub fn own_tilesets(project: &Project) -> Vec<std::path::PathBuf> {
    let words = project.names("tilesets", "png").into_iter().filter(|word| !word.contains('.'));
    words.map(|word| project.find("tilesets", &format!("{word}.png"))).collect()
}

/// The number a floor knows a tileset of one's own by (its picture drawn on
/// the template, `crate::template`): where it is among the project's
/// ([`own_tilesets`]), the same for whoever asks with that project; none if
/// it is not one of them, or once all the numbers are taken.
pub fn own_tileset(project: &Project, path: &std::path::Path) -> Option<u8> {
    let at = own_tilesets(project).iter().position(|known| known == path)?;
    u8::try_from(at).ok().and_then(|at| OWN_TILESETS.checked_add(at)).filter(|id| *id < u8::MAX)
}

/// The picture of a project's tileset of one's own, by its number.
pub fn own_tileset_path(project: &Project, id: u8) -> Option<std::path::PathBuf> {
    own_tilesets(project).get(id.checked_sub(OWN_TILESETS)? as usize).cloned()
}

/// What is drawn over a floor in a weather.
pub struct WeatherLayer {
    /// A texture that drifts, or frames covering the screen.
    pub drifts: bool,
    pub frames: Vec<Picture>,
    /// The 60ths of a second of a frame.
    pub ticks: f32,
}

/// (The rain is a moves' effect covering the screen, its frames side by
/// side, the first of the two of them.)
const RAIN_EFFECT: u16 = 697;

/// How many animations each has in its sheets.
fn counts(sheets: &Sheets) -> HashMap<u16, usize> {
    let mut counts = HashMap::new();
    for &(number, animation) in sheets.keys() {
        let count = counts.entry(number).or_insert(0);
        *count = (*count).max(animation as usize + 1);
    }
    counts
}

/// A kind of sprites, by number, those seen so far kept.
struct Kind {
    folder: &'static str,
    sheets: Arc<Sheets>,
    counts: HashMap<u16, usize>,
    seen: RefCell<HashMap<u16, Option<Rc<Sprite>>>>,
}

impl Kind {
    fn new(folder: &'static str, sheets: Sheets) -> Self {
        Kind { folder, counts: counts(&sheets), sheets: Arc::new(sheets), seen: RefCell::default() }
    }

    fn sprite(&self, folder: &str, number: u16) -> Option<Rc<Sprite>> {
        let made = || Sprite::of_sheets(folder, self.folder, self.sheets.clone(), number, self.counts.get(&number).copied().unwrap_or(0)).map(Rc::new);
        self.seen.borrow_mut().entry(number).or_insert_with(made).clone()
    }

    fn forget(&self) {
        self.seen.borrow_mut().retain(|_, sprite| sprite.as_ref().is_some_and(|sprite| Rc::strong_count(sprite) > 1));
    }
}

/// Where a picture of our own is: in the data's hand layer (a packet has it
/// there), else among the project's images (then the repository's).
pub fn ours(folder: &str, project: &Project, file: &str) -> std::path::PathBuf {
    let hand = std::path::Path::new(folder).join("hand").join(file);
    if hand.exists() { hand } else { project.find("images", file) }
}

/// The game's folder. Nothing but its lists is read before it is asked for.
pub struct Art {
    pub folder: String,
    /// The project whose own pictures it shows (see `ours`).
    pub project: Project,
    /// The things that lie on the ground: those who stand are seen over them.
    pub flats: HashSet<u16>,
    monsters: Kind,
    things: Kind,
    points: HashMap<u16, [(i32, i32); 4]>,
    /// The variants (`variants.txt`): who each one is of.
    variants: HashMap<u16, u16>,
    tips: Tips,
    effect_rows: HashMap<u16, EffectRow>,
    effects: RefCell<HashMap<u16, Option<Rc<Effect>>>>,
    statuses: OnceCell<HashMap<String, Rc<Vec<Frame>>>>,
    digits: OnceCell<Option<Picture>>,
    tilesets: RefCell<HashMap<u8, Option<Rc<Tileset>>>>,
    weathers: RefCell<HashMap<Sky, Option<Rc<WeatherLayer>>>>,
    /// The game's sounds' lengths, read when first asked (`sound_lengths`).
    sounds: Arc<crate::sound::SoundLengths>,
}

impl Art {
    /// The game's folder, that project's own pictures.
    pub fn open_in(folder: &str, project: Project) -> Self {
        let (things, flats) = prop_sheets(folder);
        Art {
            folder: folder.to_string(),
            project,
            flats,
            monsters: Kind::new("sprites", sheets(folder)),
            things: Kind::new("props", things),
            points: assets::tables::points(folder),
            variants: assets::tables::variants(folder).into_iter().map(|variant| (variant.number, variant.of)).collect(),
            tips: assets::tables::tips(folder),
            effect_rows: assets::tables::effects(folder).into_iter().map(|effect| (effect.id, effect)).collect(),
            effects: RefCell::default(),
            statuses: OnceCell::new(),
            digits: OnceCell::new(),
            tilesets: RefCell::default(),
            weathers: RefCell::default(),
            sounds: Arc::new(crate::sound::SoundLengths::new(folder)),
        }
    }

    /// How long each of the game's sounds lasts, in frames: its library
    /// read the first time one is asked, kept with this art.
    pub fn sound_lengths(&self) -> impl Fn(u16) -> Option<f32> + Send + Sync + 'static {
        let sounds = self.sounds.clone();
        move |id| sounds.of(id)
    }

    /// A monster's sprite, by its number.
    pub fn monster(&self, monster: u16) -> Option<Rc<Sprite>> {
        self.monsters.sprite(&self.folder, monster)
    }

    /// The monster whose stats, moves and points a monster has: its own, or
    /// for a variant, those of the Pokémon it is one of.
    pub fn kin(&self, monster: u16) -> u16 {
        self.variants.get(&monster).copied().unwrap_or(monster)
    }

    /// A thing of the scenes, by its object.
    pub fn thing(&self, object: u16) -> Option<Rc<Sprite>> {
        self.things.sprite(&self.folder, object)
    }

    /// A picture of a film's own, of the hand layer or of the scenes' images:
    /// `cells` pictures side by side, the first as it stands, the others its
    /// animation (one alone: both). With `ticks`, the 60ths of a second of
    /// each, as the game's sprites have them: it plays all of them in turn,
    /// the first among them (a picture without its own keeps the last pace).
    pub fn drawn(&self, file: &str, cells: u16, ticks: &[u32]) -> Option<Rc<Sprite>> {
        const TICKS: u32 = 8;
        let picture = image::open(self.ours(file)).ok()?.to_rgba8();
        let (wide, high) = (picture.width() / cells.max(1) as u32, picture.height());
        // (It stands on what it shows, not on the bottom of its picture.)
        let foot = (0..high).rev().find(|&y| (0..picture.width()).any(|x| picture.get_pixel(x, y).0[3] > 0)).unwrap_or(high - 1) as i32;
        let still = |at: u32, ticks: u32| {
            let rows = (0..high).flat_map(|y| (0..wide).flat_map(move |x| [(x + at * wide, y)]));
            let rgba: Vec<u8> = rows.flat_map(|(x, y)| picture.get_pixel(x, y).0).collect();
            let mut frame = crate::images::Frame::still(wide as usize, high as usize, (wide as i32 / 2, foot - 1), rgba);
            frame.ticks = ticks;
            frame
        };
        let standing = || vec![still(0, 60)];
        let paced = |at: u32| ticks.get(at as usize).or(ticks.last()).copied().unwrap_or(TICKS).max(1);
        let playing = || match ticks.is_empty() {
            true => (1..cells.max(2) as u32).map(|at| still(at.min(cells.max(1) as u32 - 1), TICKS)).collect::<Vec<_>>(),
            false => (0..cells.max(1) as u32).map(|at| still(at, paced(at))).collect(),
        };
        let ways = |steps: &dyn Fn() -> Vec<crate::images::Frame>| (0..8).map(|_| steps()).collect::<Vec<_>>();
        let mut animations = vec![ways(&standing), ways(&standing), Vec::new(), Vec::new(), Vec::new()];
        animations.push(ways(&playing));
        Some(Rc::new(Sprite::whole(animations)))
    }

    /// A character of a film's own, from its sprite sheet (see
    /// `crate::sheet`): a file of ours, or a path as it is.
    pub fn sheet(&self, file: &str, cell: (u16, u16), ways: u8, anims: &[crate::sheet::Cells]) -> Option<Rc<Sprite>> {
        let ours = self.ours(file);
        let path = if ours.exists() { ours } else { std::path::PathBuf::from(file) };
        let picture = image::open(path).ok()?.to_rgba8();
        let size = (picture.width() as usize, picture.height() as usize);
        Some(Rc::new(crate::sheet::sliced(picture.as_raw(), size, (cell.0 as usize, cell.1 as usize), ways, anims)))
    }

    /// Where a file of our own is: the hand layer, or the scenes' images.
    fn ours(&self, file: &str) -> std::path::PathBuf {
        ours(&self.folder, &self.project, file)
    }

    /// Where the line of a rod drawn in a variant's picture starts, from the
    /// middle of its cell (`rod_tips.txt`, from `pointe` in its table): by
    /// monster, animation, way and step.
    pub fn rod_tip(&self, monster: u16, animation: usize, way: usize, step: usize) -> Option<(i32, i32)> {
        *self.tips.get(&(monster, animation, way))?.get(step)?
    }

    /// Lets go of the sprites no one holds any more (at each new place).
    pub fn forget_sprites(&self) {
        self.monsters.forget();
        self.things.forget();
    }

    /// Where a monster's head, hands and middle are, from where it stands
    /// (y downwards).
    pub fn point(&self, monster: u16, point: u8) -> (i32, i32) {
        self.points.get(&self.kin(monster)).and_then(|points| points.get(point as usize)).copied().unwrap_or((0, 0))
    }

    /// An effect, by its number in `effects.txt`.
    pub fn effect(&self, id: u16) -> Option<Rc<Effect>> {
        let made = || self.effect_rows.get(&id).and_then(|effect| effect_of(&self.folder, effect)).map(Rc::new);
        self.effects.borrow_mut().entry(id).or_insert_with(made).clone()
    }

    /// The pictures of a status icon, by its name (`status.txt`).
    pub fn status(&self, name: &str) -> Option<Rc<Vec<Frame>>> {
        let statuses = self.statuses.get_or_init(|| {
            let step = |(wide, high, rgba): (u32, u32, Vec<u8>)| Frame {
                ticks: 4,
                ..Frame::still(wide as usize, high as usize, (wide as i32 / 2, high as i32 / 2), rgba)
            };
            status_icons(&self.folder).into_iter().map(|(name, frames)| (name, Rc::new(frames.into_iter().map(step).collect()))).collect()
        });
        statuses.get(name).cloned()
    }

    /// The game's digits, in rows of yellow, red and green, 8 pixels high:
    /// 0 to 9, plus, minus, "MISS".
    pub fn digits(&self) -> Option<&Picture> {
        self.digits.get_or_init(|| picture(&data::path(&self.folder, "numbers.png"))).as_ref()
    }

    /// A floor's tileset, by `Floor::tileset`: the game's, or one of one's
    /// own (from [`OWN_TILESETS`], see [`own_tileset`]).
    pub fn tileset(&self, id: u8) -> Option<Rc<Tileset>> {
        let made = || match own_tileset_path(&self.project, id) {
            Some(path) => media::template::read(&path).map_err(|why| eprintln!("{why}")).ok().map(Rc::new),
            None => read_tileset(&self.folder, id).map(Rc::new),
        };
        self.tilesets.borrow_mut().entry(id).or_insert_with(made).clone()
    }

    /// What is drawn over a floor in a weather; none for most.
    pub fn weather(&self, weather: Sky) -> Option<Rc<WeatherLayer>> {
        let made = || self.weather_layer(weather).map(Rc::new);
        self.weathers.borrow_mut().entry(weather).or_insert_with(made).clone()
    }

    fn weather_layer(&self, weather: Sky) -> Option<WeatherLayer> {
        match weather {
            Sky::Sandstorm | Sky::Fog => {
                let name = if weather == Sky::Sandstorm { "sandstorm" } else { "fog" };
                let texture = picture(&data::path(&self.folder, &format!("weather/{name}.png")))?;
                Some(WeatherLayer { drifts: true, frames: vec![texture], ticks: 1.0 })
            }
            Sky::Rain => {
                let rain = self.effect_rows.get(&RAIN_EFFECT)?;
                let (wide, high, ticks) = (rain.wide, rain.high, *rain.durations.first()? as f32);
                let strip = image::open(data::path(&self.folder, &format!("effects/{RAIN_EFFECT}.png"))).ok()?.to_rgba8();
                let frame = |frame: u32| (wide as usize, high as usize, image::imageops::crop_imm(&strip, frame * wide, 0, wide, high).to_image().into_raw());
                Some(WeatherLayer { drifts: false, frames: (0..strip.width() / wide.max(1)).map(frame).collect(), ticks })
            }
        }
    }

    /// A place's pictures: its map's, or its backdrop.
    pub fn ground(&self, level: u16) -> Option<Ground> {
        ground_of(&self.folder, level)
    }

    /// The whole picture of a place, all its layers on it.
    pub fn backdrop(&self, level: u16) -> Option<Picture> {
        picture(&data::path(&self.folder, &format!("backdrops/{level}.png")))
    }

    /// A face of a monster: 40 by 40 pixels.
    pub fn portrait(&self, monster: u16, face: u16) -> Option<Vec<u8>> {
        let read = |face: u16| picture(&data::path(&self.folder, &format!("portraits/{monster}_{face}.png"))).map(|(_, _, rgba)| rgba);
        // (A face the game has not painted for it shows its Normal one: Poissoroy
        // has no other, and a variant painted with its Normal face only.)
        read(face).or_else(|| read(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture of ours at a pace of its own: all its pictures in turn, each
    /// its own time; without one, the first stands and the others play.
    #[test]
    fn a_picture_of_ours_plays_at_its_own_pace() {
        let folder = concat!(env!("CARGO_MANIFEST_DIR"), "/../no_data_here");
        let art = Art::open_in(folder, Project::library(concat!(env!("CARGO_MANIFEST_DIR"), "/..")));
        let paced = art.drawn("torch.4.png", 4, &[40, 2, 3, 2]).expect("the torch, among the scenes' images");
        let ticks: Vec<u32> = paced.ways(5)[0].iter().map(|frame| frame.ticks).collect();
        assert_eq!(ticks, [40, 2, 3, 2]);
        let usual = art.drawn("torch.4.png", 4, &[]).expect("the torch");
        assert_eq!(usual.ways(5)[0].iter().map(|frame| frame.ticks).collect::<Vec<_>>(), [8, 8, 8]);
        assert_eq!(usual.ways(1)[0].len(), 1);
    }
}
