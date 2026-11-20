//! The pictures of the game's files, as they are shown: sprites whose
//! animations are read as they come to be seen, effects, places with their
//! colours that change, what stands before who walks there.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::art_files::{Cell, GroundFile, Sheets, animation, effect_file, ground_file};

/// A picture: `(width, height, RGBA)`.
pub type Picture = (usize, usize, Vec<u8>);

/// A picture from a file.
pub fn picture(path: &str) -> Option<Picture> {
    let picture = image::open(path).ok()?.to_rgba8();
    Some((picture.width() as usize, picture.height() as usize, picture.into_raw()))
}

/// A picture in the console's colours: five bits a part, as its screens show
/// them.
pub fn console_colours(rgba: &mut [u8]) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
        for part in &mut pixel[..3] {
            *part &= 0xF8;
        }
    }
}

/// A picture of a sprite.
#[derive(Clone)]
pub struct Frame {
    pub wide: usize,
    pub high: usize,
    pub rgba: Vec<u8>,
    /// Its 60ths of a second.
    pub ticks: u32,
    /// Where its centre is, in the picture, and where its shadow lies from
    /// there.
    pub origin: (i32, i32),
    pub shadow: (i32, i32),
}

impl Frame {
    /// A picture that stays.
    pub fn still(wide: usize, high: usize, origin: (i32, i32), rgba: Vec<u8>) -> Self {
        Frame { wide, high, rgba, ticks: 60, origin, shadow: (0, 0) }
    }

    /// How far above its place it is drawn.
    pub fn lift(&self) -> i32 {
        self.origin.1 - self.high as i32 / 2
    }
}

impl From<Cell> for Frame {
    fn from(cell: Cell) -> Self {
        Frame {
            wide: cell.wide as usize,
            high: cell.high as usize,
            origin: (cell.wide as i32 / 2, cell.high as i32 / 2),
            rgba: cell.rgba,
            ticks: cell.ticks as u32,
            shadow: cell.shadow,
        }
    }
}

/// The pictures of an animation, by way faced and step.
pub type Ways = Vec<Vec<Frame>>;
/// An animation read, by the number it was given.
type Read = (usize, Ways);

pub fn ways_of(steps: Vec<Vec<Cell>>) -> Ways {
    steps.into_iter().map(|cells| cells.into_iter().map(Frame::from).collect()).collect()
}

/// Where the animations of a sprite are, when they come to be shown.
#[derive(Clone)]
struct Source {
    folder: Arc<str>,
    kind: &'static str,
    sheets: Arc<Sheets>,
    number: u16,
}

impl Source {
    fn has(&self, animation: usize) -> bool {
        self.sheets.contains_key(&(self.number, animation as u8))
    }

    /// (One who has no walk stands instead, one who has no way of standing
    /// walks.)
    fn read(&self, at: usize) -> Ways {
        let read = |at: usize| animation(&self.folder, self.kind, &self.sheets, self.number, at as u8);
        let found = match at {
            0 | 1 => read(at).or_else(|| read(1 - at)),
            _ => read(at),
        };
        found.map_or_else(Vec::new, ways_of)
    }
}

/// The animations of a monster in dungeons, by their place among its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anim {
    Walk = 0,
    Idle = 1,
    Hurt = 2,
    Sleep = 3,
    Attack = 4,
}

/// Pictures by animation, way faced and step. (Of a monster: walking,
/// standing, hurt, asleep, attacking, then its own.) Those of a monster are
/// read when they come to be shown: a crowd stands, and each of them has
/// dozens of animations.
pub struct Sprite {
    source: Option<Source>,
    animations: Vec<OnceCell<Ways>>,
}

impl Sprite {
    /// One whose pictures are all there.
    pub fn whole(animations: Vec<Ways>) -> Self {
        Sprite { source: None, animations: animations.into_iter().map(OnceCell::from).collect() }
    }

    /// One of those of the sheets, none yet read. (One with neither a walk
    /// nor a way of standing is no sprite.)
    pub fn of_sheets(folder: &str, kind: &'static str, sheets: Arc<Sheets>, number: u16, count: usize) -> Option<Self> {
        let source = Source { folder: Arc::from(folder), kind, sheets, number };
        (source.has(0) || source.has(1)).then(|| Sprite { source: Some(source), animations: (0..count.max(5)).map(|_| OnceCell::new()).collect() })
    }

    pub fn ways(&self, at: usize) -> &[Vec<Frame>] {
        let read = || self.source.as_ref().map_or_else(Vec::new, |source| source.read(at));
        self.animations.get(at).map_or(&[], |ways| ways.get_or_init(read))
    }

    /// The steps of an animation that way: its own, or the walk's where it
    /// has none. Which animation that is, and the steps. Ways start facing
    /// down and turn counter-clockwise.
    pub fn steps(&self, anim: Anim, way: usize) -> (Anim, &[Frame]) {
        let steps = |anim: Anim| self.ways(anim as usize).get(way).map_or(&[][..], Vec::as_slice);
        match steps(anim) {
            [] => (Anim::Walk, steps(Anim::Walk)),
            found => (anim, found),
        }
    }

    /// Whether it has that animation, without reading it.
    pub fn has(&self, at: usize) -> bool {
        at < self.animations.len()
            && match &self.source {
                Some(source) => at < 2 || source.has(at),
                None => self.ways(at).iter().any(|steps| !steps.is_empty()),
            }
    }

    fn to_read(&self, at: usize) -> Option<Source> {
        self.source.clone().filter(|_| self.has(at) && self.animations[at].get().is_none())
    }
}

/// Those animations read at once, on all the machine has: a crowd's take a
/// second on one alone.
pub fn read_together<'a>(wanted: impl Iterator<Item = (&'a Sprite, usize)>) {
    let mut wanted: Vec<(&Sprite, usize, Source)> = wanted.filter_map(|(sprite, at)| Some((sprite, at, sprite.to_read(at)?))).collect();
    wanted.sort_by_key(|(sprite, at, _)| (*sprite as *const Sprite, *at));
    wanted.dedup_by_key(|(sprite, at, _)| (*sprite as *const Sprite, *at));
    if wanted.is_empty() {
        return;
    }
    let hands = hands();
    let jobs: Vec<(usize, Source)> = wanted.iter().map(|(_, at, source)| (*at, source.clone())).collect();
    let read: Vec<Ways> = std::thread::scope(|scope| {
        let reading = jobs.chunks(jobs.len().div_ceil(hands));
        let hands: Vec<_> = reading.map(|jobs| scope.spawn(move || jobs.iter().map(|(at, source)| source.read(*at)).collect::<Vec<Ways>>())).collect();
        hands.into_iter().flat_map(|hand| hand.join().expect("reading a picture")).collect()
    });
    for ((sprite, at, _), ways) in wanted.into_iter().zip(read) {
        let _ = sprite.animations[at].set(ways);
    }
}

/// How many can read at once on this machine.
fn hands() -> usize {
    std::thread::available_parallelism().map_or(1, |hands| hands.get())
}

/// Animations being read on the side.
pub struct Behind {
    /// Those on their way, by the number each was given.
    waiting: HashMap<usize, (Rc<Sprite>, usize)>,
    done: (Sender<Read>, Receiver<Read>),
}

impl Default for Behind {
    fn default() -> Self {
        Behind { waiting: HashMap::new(), done: channel() }
    }
}

impl Behind {
    /// Animations that will be wanted, read on the side: all goes on
    /// meanwhile, and does not stop when a whole crowd comes to them.
    pub fn read(&mut self, wanted: Vec<(Rc<Sprite>, usize)>) {
        let first = self.waiting.keys().max().map_or(0, |last| last + 1);
        let mut jobs = Vec::new();
        for (sprite, at) in wanted {
            if let Some(source) = sprite.to_read(at) {
                let number = first + jobs.len();
                jobs.push((number, at, source));
                self.waiting.insert(number, (sprite, at));
            }
        }
        if jobs.is_empty() {
            return;
        }
        let (jobs, next) = (Arc::new(jobs), Arc::new(AtomicUsize::new(0)));
        for _ in 0..hands().saturating_sub(1).max(1) {
            let (jobs, next, send) = (jobs.clone(), next.clone(), self.done.0.clone());
            std::thread::spawn(move || {
                while let Some((number, at, source)) = jobs.get(next.fetch_add(1, Ordering::Relaxed)) {
                    if send.send((*number, source.read(*at))).is_err() {
                        break;
                    }
                }
            });
        }
    }

    /// Those read on the side since the last time, put in their place.
    pub fn take(&mut self) {
        for (number, ways) in self.done.1.try_iter() {
            if let Some((sprite, at)) = self.waiting.remove(&number) {
                let _ = sprite.animations[at].set(ways);
            }
        }
    }
}

/// An effect of the game (an emote, a move's sparks).
pub struct Effect {
    pub sprite: Sprite,
    pub ticks: f32,
    pub repeats: bool,
    /// It covers the screen, whatever the view shows.
    pub screen: bool,
    pub sound: u16,
    /// Where it shows on someone (see `Art::point`).
    pub point: u8,
}

/// A place: its pictures, those drawn over who walks there.
pub struct Ground {
    pub wide: usize,
    pub high: usize,
    pub frames: Vec<Vec<u8>>,
    pub upper: Vec<Vec<u8>>,
    pub frame_seconds: f32,
    /// Which palette and colour each pixel of those shows, a byte each.
    indices: Vec<Vec<u8>>,
    upper_indices: Vec<Vec<u8>>,
    /// The palettes whose colours change with time, and where each is.
    cycles: Vec<Cycle>,
    steps: Vec<usize>,
    /// Where those are in each layer: all a change has to paint.
    moving: Vec<Vec<u32>>,
    pub cover: Cover,
}

/// The upper layer as something to stand behind: for each of its solid
/// tiles, where what it belongs to meets the ground. Who stands above that
/// line is behind it. (0: it hides no one: much of the layer is paths and
/// doormats, that one walks on.)
#[derive(Default)]
pub struct Cover {
    pub tiles_wide: usize,
    pub ground_line: Vec<u16>,
}

impl Cover {
    pub const TILE: usize = 8;

    pub fn of(solid: &[Vec<bool>], upper: Option<&Vec<u8>>, wide: usize, high: usize) -> Cover {
        let Some(upper) = upper else {
            return Cover::default();
        };
        let (tiles_wide, tiles_high) = (wide / Self::TILE, high / Self::TILE);
        let shown = |column: usize, row: usize| {
            (0..Self::TILE * Self::TILE).any(|at| {
                let (x, y) = (column * Self::TILE + at % Self::TILE, row * Self::TILE + at / Self::TILE);
                upper.get((y * wide + x) * 4 + 3).is_some_and(|alpha| *alpha > 0)
            })
        };
        let solid_at = |column: usize, row: usize| solid.get(row).and_then(|tiles| tiles.get(column)).copied().unwrap_or(false);
        let line = |column: usize, row: usize| -> u16 {
            match solid_at(column, row) && shown(column, row) {
                true => ((row..).find(|down| !solid_at(column, *down)).unwrap_or(row) * Self::TILE) as u16,
                false => 0,
            }
        };
        let ground_line = (0..tiles_high).flat_map(|row| (0..tiles_wide).map(move |column| (column, row))).map(|(column, row)| line(column, row)).collect();
        Cover { tiles_wide, ground_line }
    }
}

/// Sixteen colours.
type Palette = [[u8; 3]; 16];

/// A palette changing with time: which, the 60ths of a second a step, its
/// colours at each step.
struct Cycle {
    palette: usize,
    ticks: u16,
    colours: Vec<Palette>,
}

impl Ground {
    /// What changes colour at that moment is painted anew.
    pub fn advance(&mut self, frames: f32) {
        let now = frames.max(0.0) as usize;
        let steps: Vec<usize> = self.cycles.iter().map(|cycle| now / cycle.ticks.max(1) as usize % cycle.colours.len().max(1)).collect();
        if steps == self.steps {
            return;
        }
        let mut shown: [Option<&[[u8; 3]; 16]>; 16] = [None; 16];
        for (cycle, &step) in self.cycles.iter().zip(&steps) {
            if let Some(slot) = shown.get_mut(cycle.palette) {
                *slot = cycle.colours.get(step);
            }
        }
        let layers = self.indices.iter().zip(&mut self.frames).chain(self.upper_indices.iter().zip(&mut self.upper));
        for ((indices, picture), moving) in layers.zip(&self.moving) {
            let pixels = picture.as_chunks_mut::<4>().0;
            for &at in moving {
                let index = indices[at as usize];
                if let Some(colours) = shown[(index >> 4) as usize] {
                    pixels[at as usize][..3].copy_from_slice(&colours[(index & 0xF) as usize]);
                }
            }
        }
        self.steps = steps;
    }
}

pub fn ground_of(folder: &str, level: u16) -> Option<Ground> {
    if let Some(file) = ground_file(folder, level).filter(|file| !file.pictures[0].is_empty()) {
        let GroundFile { info, pictures: [frames, upper, indices, upper_indices, _] } = file;
        let cycles: Vec<Cycle> = info.cycles.into_iter().map(|(palette, ticks, colours)| Cycle { palette, ticks, colours }).collect();
        let steps = vec![0; cycles.len()];
        let cycling = |index: u8| index & 0xF != 0 && cycles.iter().any(|cycle| cycle.palette == (index >> 4) as usize);
        let moving_in = |layer: &Vec<u8>| layer.iter().enumerate().filter(|(_, index)| cycling(**index)).map(|(at, _)| at as u32).collect();
        // (Layers of as many pictures as their indices: none where none cycles.)
        let moving = if cycles.is_empty() { Vec::new() } else { indices.iter().chain(&upper_indices).map(moving_in).collect() };
        let cover = Cover::of(&crate::art_files::solid_rows(folder, level), upper.first(), info.width, info.height);
        return Some(Ground {
            wide: info.width,
            high: info.height,
            frames,
            upper,
            frame_seconds: info.frame_seconds,
            indices,
            upper_indices,
            cycles,
            steps,
            moving,
            cover,
        });
    }
    let (wide, high, pixels) = picture(&assets::data::path(folder, &format!("backdrops/{level}.png")))?;
    Some(Ground {
        wide,
        high,
        frames: vec![pixels],
        upper: Vec::new(),
        frame_seconds: 1.0,
        indices: Vec::new(),
        upper_indices: Vec::new(),
        cycles: Vec::new(),
        steps: Vec::new(),
        moving: Vec::new(),
        cover: Cover::default(),
    })
}

pub fn effect_of(folder: &str, effect: &assets::tables::EffectRow) -> Option<Effect> {
    let file = effect_file(folder, effect)?;
    Some(Effect {
        sprite: Sprite::whole(vec![ways_of(file.ways)]),
        ticks: file.ticks,
        repeats: file.repeats,
        screen: file.screen,
        sound: file.sound,
        point: file.point,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_console_has_five_bits_a_part_and_keeps_what_shows() {
        let mut pixel = [0xFF, 0x0F, 0x88, 0x7F];
        console_colours(&mut pixel);
        assert_eq!(pixel, [0xF8, 0x08, 0x88, 0x7F]);
    }

    #[test]
    fn a_picture_raised_says_how_far() {
        let frame = Frame { wide: 16, high: 16, rgba: Vec::new(), ticks: 60, origin: (8, 14), shadow: (0, 0) };
        assert_eq!(frame.lift(), 6);
    }
}
