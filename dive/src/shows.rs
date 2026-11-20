//! What the dungeon shows for a moment: moves' animations, what flies and
//! is lobbed, the numbers over monsters. The rules only tell it happened.

use std::rc::Rc;

use glam::Vec2;
use rules::dungeon::Pos;
use rules::dungeon::stack;
use rules::dungeon::status::item::ROCKS;
use rules::run::{Dungeon, MonsterId};
use rules::tables::Tables;
use scenes::commands::MoveShown;
use shell::Screen;
use shell::art::Art;
use shell::images::{Effect, Frame};

use super::actors::{Actor, looks_of, point_on, way_of};
use super::floor::Looks;
use super::tile_middle;

/// Frames from an attack starting to its blow, at the least (by eye).
const HIT: f32 = 10.0;
/// Pixels a frame of a thrown item (the original's fastest projectiles).
const THROWN: f32 = 6.0;
/// How high what is lobbed flies, in pixels (by eye).
const LOB_HEIGHT: f32 = 24.0;
/// The frames a number shows.
const NUMBER_SHOWN: f32 = 48.0;

/// The way of something flying: from where, and how high it goes.
#[derive(Clone, Copy)]
struct Lob {
    start: Vec2,
    height: f32,
}

impl Lob {
    /// How high it is, at `at` on its way to `to`.
    fn rise(&self, at: Vec2, to: Vec2) -> f32 {
        let whole = (to - self.start).length().max(1.0);
        let done = (1.0 - (to - at).length() / whole).clamp(0.0, 1.0);
        self.height * 4.0 * done * (1.0 - done)
    }
}

/// An effect on the floor.
struct Shown {
    art: Rc<Effect>,
    /// The frames before it starts, those it has shown, those it lasts.
    delay: f32,
    clock: u32,
    left: f32,
    /// Heard as it starts, if it has to wait.
    sound: Option<u16>,
    way: usize,
    at: Vec2,
    to: Vec2,
    /// Pixels a frame on the way there (none: it is there).
    speed: Option<f32>,
    lob: Option<Lob>,
}

/// An item in the air.
struct Thrown {
    item: u16,
    at: Vec2,
    to: Vec2,
    lob: Option<Lob>,
}

/// A number over a monster (damage, HP back) or "MISS", as the original's
/// `DisplayAnimatedNumbers`.
struct Number {
    owner: MonsterId,
    picture: Frame,
    at: Vec2,
    delay: f32,
    age: f32,
}

#[derive(Default)]
pub struct Shows {
    shown: Vec<Shown>,
    thrown: Vec<Thrown>,
    numbers: Vec<Number>,
    /// When blows land on the tiles of the moves just told.
    landing: Vec<(Pos, f32)>,
}

/// Where effects are asked for: what is needed to place them.
pub struct Place<'a> {
    pub dungeon: &'a Dungeon,
    pub art: &'a Art,
    pub tables: &'a Tables,
    /// Sounds to be heard now.
    pub heard: &'a mut Vec<u16>,
}

fn way_for(art: &Effect, facing: (i32, i32)) -> usize {
    if art.sprite.ways(0).len() == 8 { way_of(facing) } else { 0 }
}

impl Shows {
    pub fn clear(&mut self) {
        *self = Shows::default();
    }

    /// The sights of another frame: what was landing is told.
    pub fn new_sights(&mut self) {
        self.landing.clear();
    }

    /// The frames until a blow lands on that tile.
    pub fn landing_on(&self, tile: Pos) -> Option<f32> {
        self.landing.iter().find(|(at, _)| *at == tile).map(|(_, frames)| *frames)
    }

    /// An effect on who stands on a tile, at the point it names.
    pub fn effect(&mut self, id: u16, at: Pos, place: &mut Place) {
        let Some(art) = place.art.effect(id) else {
            return;
        };
        place.heard.push(art.sound);
        let to = tile_middle(at) + point_on(place.dungeon, place.art, at, art.point);
        let left = art.ticks;
        self.shown.push(Shown { way: 0, art, delay: 0.0, clock: 0, left, sound: None, at: to, to, speed: None, lob: None });
    }

    pub fn thrown(&mut self, item: u16, from: Pos, to: Pos) {
        let lob = ROCKS.contains(&stack::id(item)).then(|| Lob { start: tile_middle(from), height: LOB_HEIGHT });
        self.thrown.push(Thrown { item, at: tile_middle(from), to: tile_middle(to), lob });
    }

    /// A move used: on its user, flying to each target, then on the target.
    /// How long all of it lasts, in frames.
    pub fn used(
        &mut self,
        move_id: u16,
        user: Option<&mut Actor>,
        facing: (i32, i32),
        species: Option<u16>,
        from: Pos,
        targets: &[Pos],
        place: &mut Place,
    ) -> f32 {
        let shown = MoveShown::of(place.tables, move_id, species);
        let [before, strike, on_target, flying] = shown.effects;
        // Its blow lands as its user reaches out: about halfway through.
        let mut hit = HIT;
        if let Some(user) = user {
            user.start_motion(shown.group);
            hit = hit.max(user.playing_for() as f32 / 2.0);
        }
        place.heard.push(shown.sound);
        let art = |effect: u16| place.art.effect(effect);
        let wait = art(before).map_or(0.0, |art| art.ticks);
        let effect_point = |effect: u16| art(effect).map_or(3, |art| art.point);
        let user_point = |effect: u16| if shown.point == 255 { effect_point(effect) } else { shown.point };
        let flies = art(flying).is_some();
        let flight = shown.flight;
        let landing = |tile: Pos| {
            let travel = (tile_middle(tile) - tile_middle(from)).length() / flight;
            wait + if flies { hit.max(travel) } else { hit }
        };
        let mut longest = 0f32;
        let mut heard: Vec<u16> = Vec::new();
        let (dungeon, game_art) = (place.dungeon, place.art);
        let mut sounds = Vec::new();
        let mut show = |shows: &mut Vec<Shown>, effect: u16, at: Pos, start: Option<Pos>, delay: f32, point: u8| {
            let Some(art) = game_art.effect(effect) else {
                return;
            };
            longest = longest.max(delay + art.ticks);
            let sound = (!heard.contains(&effect)).then_some(art.sound);
            heard.push(effect);
            let begins = start.unwrap_or(at);
            let begin = tile_middle(begins) + point_on(dungeon, game_art, begins, point);
            let to = tile_middle(at) + point_on(dungeon, game_art, at, point);
            let lob = (start.is_some() && shown.lob).then_some(Lob { start: begin, height: LOB_HEIGHT });
            if delay <= 0.0 {
                sounds.extend(sound);
            }
            let (way, left) = (way_for(&art, facing), art.ticks);
            shows.push(Shown { art, delay, clock: 0, left, sound: sound.filter(|_| delay > 0.0), way, at: begin, to, speed: start.map(|_| flight), lob });
        };
        show(&mut self.shown, before, from, None, 0.0, user_point(before));
        show(&mut self.shown, strike, from, None, wait, user_point(strike));
        for &tile in targets {
            show(&mut self.shown, flying, tile, Some(from), 0.0, user_point(flying));
            show(&mut self.shown, on_target, tile, None, landing(tile), effect_point(on_target));
            self.landing.push((tile, landing(tile)));
        }
        place.heard.append(&mut sounds);
        longest
    }

    /// A number over a monster, as its blow lands; a new one over it takes
    /// the place of the old one then.
    pub fn number(&mut self, owner: MonsterId, picture: Frame, at: Vec2, delay: f32) {
        self.numbers.push(Number { owner, picture, at, delay, age: 0.0 });
    }

    /// A frame goes by; what is to be heard as it starts is told.
    pub fn advance(&mut self, heard: &mut Vec<u16>) {
        for thrown in &mut self.thrown {
            // (An arc's height is only how it is drawn.)
            let way = thrown.to - thrown.at;
            let far = way.length();
            thrown.at = if far <= THROWN { thrown.to } else { thrown.at + way / far * THROWN };
        }
        self.thrown.retain(|thrown| thrown.at != thrown.to);
        for shown in &mut self.shown {
            if shown.delay > 0.0 {
                shown.delay -= 1.0;
                if shown.delay <= 0.0 {
                    heard.extend(shown.sound.take());
                }
                continue;
            }
            shown.clock += 1;
            shown.left -= 1.0;
            if let Some(speed) = shown.speed {
                let way = shown.to - shown.at;
                let far = way.length();
                shown.at = if far <= speed { shown.to } else { shown.at + way / far * speed };
            }
        }
        self.shown.retain(|shown| shown.left > 0.0);
        // Who gets a new number now: the older ones over it go.
        let newest: Vec<MonsterId> = self.numbers.iter().filter(|number| number.delay <= 1.0 && number.age == 0.0).map(|number| number.owner).collect();
        self.numbers.retain(|number| number.age == 0.0 || !newest.contains(&number.owner));
        for number in &mut self.numbers {
            if number.delay > 0.0 {
                number.delay -= 1.0;
                continue;
            }
            // (Up for a third of its time, then still.)
            if number.age < NUMBER_SHOWN / 3.0 {
                number.at.y -= 0.5;
            }
            number.age += 1.0;
        }
        self.numbers.retain(|number| number.age < NUMBER_SHOWN);
    }

    /// What is in the air and on tiles, over those who stand.
    pub fn draw(&self, world: &mut Screen, looks: &mut Looks, tables: &Tables, seen: impl Fn(Vec2) -> (i32, i32), middle: Vec2) {
        for shown in self.shown.iter().filter(|shown| shown.delay <= 0.0) {
            let ways = shown.art.sprite.ways(0);
            let steps = ways.get(shown.way).or(ways.first()).map_or(&[][..], Vec::as_slice);
            let whole: u32 = steps.iter().map(|frame| frame.ticks.max(1)).sum();
            let mut left = shown.clock % whole.max(1);
            let frame = steps.iter().find(|frame| {
                let here = left < frame.ticks.max(1);
                left = left.saturating_sub(frame.ticks.max(1));
                here
            });
            if let Some(frame) = frame {
                let rise = shown.lob.map_or(0.0, |lob| lob.rise(shown.at, shown.to));
                let at = if shown.art.screen { middle } else { shown.at - Vec2::new(0.0, rise) };
                world.sprite(looks.frame(frame), seen(at), world_size(), [255.0; 3], 1.0);
            }
        }
        for thrown in &self.thrown {
            if let Some(picture) = looks.item(thrown.item, tables) {
                let rise = thrown.lob.map_or(0.0, |lob| lob.rise(thrown.at, thrown.to));
                world.sprite(picture, seen(thrown.at - Vec2::new(0.0, rise)), world_size(), [255.0; 3], 1.0);
            }
        }
    }

    pub fn draw_numbers(&self, world: &mut Screen, seen: impl Fn(Vec2) -> (i32, i32)) {
        for number in self.numbers.iter().filter(|number| number.delay <= 0.0) {
            world.sprite(&number.picture, seen(number.at), world_size(), [255.0; 3], 1.0);
        }
    }
}

fn world_size() -> f32 {
    crate::SCALE as f32
}

/// Colours of the rows of the game's digits.
#[derive(Clone, Copy)]
pub enum Digits {
    Yellow = 0,
    Red = 1,
    Green = 2,
}

/// The game's digits, 8 pixels square side by side (MISS four of them), in
/// rows of yellow, red and green; plus and minus after the digits.
pub fn number_picture(art: &Art, text: &str, row: Digits) -> Option<Frame> {
    const SIDE: usize = 8;
    let (sheet_wide, _, sheet) = art.digits()?;
    let cells: Vec<(usize, usize)> = match text {
        "MISS" => vec![(96, 32)],
        _ => text.chars().map(|c| (c.to_digit(10).map_or(if c == '+' { 80 } else { 88 }, |digit| digit as usize * SIDE), SIDE)).collect(),
    };
    let wide: usize = cells.iter().map(|(_, wide)| wide).sum();
    let mut rgba = vec![0u8; wide * SIDE * 4];
    let mut left = 0;
    for (from, cell) in cells {
        for y in 0..SIDE {
            let source = ((row as usize * SIDE + y) * sheet_wide + from) * 4;
            let to = (y * wide + left) * 4;
            if let Some(pixels) = sheet.get(source..source + cell * 4) {
                rgba[to..to + cell * 4].copy_from_slice(pixels);
            }
        }
        left += cell;
    }
    Some(Frame::still(wide, SIDE, (wide as i32 / 2, SIDE as i32 / 2), rgba))
}

/// Where a number shows over who stands there.
pub fn over(dungeon: &Dungeon, art: &Art, who: MonsterId, shown_at: Vec2) -> Vec2 {
    let head = dungeon.monster(who).and_then(looks_of).map_or((0, -16), |looks| art.point(looks, 0));
    shown_at + Vec2::new(head.0 as f32, head.1 as f32 - 10.0)
}
