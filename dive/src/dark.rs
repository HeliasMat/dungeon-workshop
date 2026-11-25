//! What lies over a floor: the dark around the team, the weather.

use std::rc::Rc;

use glam::Vec2;
use rules::dungeon::weather::{self, FOG, SANDSTORM};
use rules::dungeon::{Floor, Pos, Tile};
use rules::run::{Book, Dungeon};
use shell::art::{Art, WeatherLayer};
use shell::images::console_colours;
use shell::{Laid, Screen};
use stage_view::ui::{SCREEN_HEIGHT, SCREEN_WIDTH};

use super::TILE;

/// Tiles of the dark around the leader, each way; how dark it is.
const AROUND: (i32, i32) = (13, 9);
const DARK: u8 = 235;

/// On a dark floor (its visibility range above 0) only the leader's room
/// is seen, or from a hallway the tiles that close to it; the rest is in
/// the dark (`floor_properties::visibility_range`).
#[derive(Default)]
pub struct Dark {
    drawn: Option<(u32, u32, Pos, bool)>,
    /// Those of the leader's room and the walls around it, around the leader.
    lit: Vec<bool>,
}

impl Dark {
    /// `dropeye`: the leader's room is as dark as the rest.
    fn lit_tiles(&mut self, floor: &Floor, at: Pos, dropeye: bool) {
        let room = match floor.tile(at) {
            Tile::Floor { room: Some(room) } if !dropeye => Some(room),
            _ => None,
        };
        let in_room =
            |tile: Pos| room.is_some() && (-1..=1).any(|dy| (-1..=1).any(|dx| floor.tile(Pos::new(tile.x + dx, tile.y + dy)) == Tile::Floor { room }));
        self.lit.clear();
        for y in -AROUND.1..=AROUND.1 {
            self.lit.extend((-AROUND.0..=AROUND.0).map(|x| in_room(Pos::new(at.x + x, at.y + y))));
        }
    }

    /// The dark, in spans of rows: `leader` shows at the middle of the
    /// screen. (A hard edge, as all of the game's pixels.) A leader under
    /// Dropeye sees only the tiles next to it, even in a room.
    pub fn draw(&mut self, world: &mut Screen, dungeon: &Dungeon, book: &Book) {
        let dropeye = dungeon.leader().status.dropeye;
        let range = match (dropeye, dungeon.lit) {
            (true, _) => 1,
            (false, true) => 0,
            (false, false) => dungeon.spec(book).map_or(0, |floor| floor.visibility),
        };
        if range <= 0 {
            return;
        }
        let at = dungeon.leader().at;
        let key = (dungeon.generation, dungeon.repaints, at, dropeye);
        if self.drawn != Some(key) {
            self.lit_tiles(&dungeon.floor, at, dropeye);
            self.drawn = Some(key);
        }
        let scale = world.scale as i32;
        let (wide, high) = (SCREEN_WIDTH as i32, SCREEN_HEIGHT as i32);
        let radius = ((range as f32 + 0.5) * TILE as f32 - TILE as f32 / 4.0) as i32;
        let lit = |column: i32, row: i32| {
            let (x, y) = (column + AROUND.0, row + AROUND.1);
            (0..=AROUND.0 * 2).contains(&x) && (0..=AROUND.1 * 2).contains(&y) && self.lit[(y * (AROUND.0 * 2 + 1) + x) as usize]
        };
        let dark = [0, 0, 0, DARK];
        for y in 0..high {
            let from_middle = y - high / 2;
            let row = (from_middle + TILE / 2).div_euclid(TILE);
            // (The part of the round in this row.)
            let across = radius * radius - from_middle * from_middle;
            let (seen_from, seen_to) = match across > 0 {
                true => {
                    let half = (across as f32).sqrt() as i32;
                    (wide / 2 - half, wide / 2 + half)
                }
                false => (wide / 2, wide / 2),
            };
            let mut span: Option<(i32, i32)> = None;
            let mut lay = |from: i32, to: i32| {
                if to <= from {
                    return;
                }
                match &mut span {
                    Some((_, end)) if *end == from => *end = to,
                    _ => {
                        if let Some((start, end)) = span {
                            world.fill(start * scale, y * scale, (end - start) * scale, scale, dark);
                        }
                        span = Some((from, to));
                    }
                }
            };
            let mut x = 0;
            while x < wide {
                let column = (x - wide / 2 + TILE / 2).div_euclid(TILE);
                let end = ((column * TILE + wide / 2 + TILE / 2).min(wide)).max(x + 1);
                if !lit(column, row) {
                    lay(x, end.min(seen_from));
                    lay(x.max(seen_to), end);
                }
                x = end;
            }
            if let Some((start, end)) = span {
                world.fill(start * scale, y * scale, (end - start) * scale, scale, dark);
            }
        }
    }
}

/// How much of a drifting texture shows, of 255, and how fast it goes, in
/// pixels a frame.
const SEEN: u32 = 115;
const DRIFT: [(u8, Vec2); 2] = [(SANDSTORM, Vec2::new(-3.0, -1.0)), (FOG, Vec2::new(-0.3, -0.1))];

/// What is drawn over the view for the weather: rain falling, sand or fog
/// drifting, as the original does (it draws the last two with its 3D
/// engine; here they are see-through pictures). (Hail and snow have their
/// colours only: the game loads no texture for them.)
#[derive(Default)]
pub struct Sky {
    weather: Option<u8>,
    layer: Option<Rc<WeatherLayer>>,
    /// The texture laid as many times as the view takes, one over, and one
    /// of its sides.
    drifting: Vec<u8>,
    laid: (usize, usize),
    side: (usize, usize),
}

/// How a weather shows, if it does.
fn over_the_floor(kind: u8) -> Option<shell::art::Sky> {
    use shell::art::Sky::{Fog, Rain, Sandstorm};
    match kind {
        weather::SANDSTORM => Some(Sandstorm),
        weather::FOG => Some(Fog),
        weather::RAIN => Some(Rain),
        _ => None,
    }
}

impl Sky {
    fn change(&mut self, weather: u8, art: &Art) {
        self.weather = Some(weather);
        self.layer = over_the_floor(weather).and_then(|shown| art.weather(shown));
        self.drifting.clear();
        let Some((wide, high, rgba)) = self.layer.as_ref().filter(|layer| layer.drifts).and_then(|layer| layer.frames.first()) else {
            return;
        };
        let (wide, high) = ((*wide).max(1), (*high).max(1));
        self.side = (wide, high);
        self.laid = ((SCREEN_WIDTH / wide + 2) * wide, (SCREEN_HEIGHT / high + 2) * high);
        self.drifting.reserve(self.laid.0 * self.laid.1 * 4);
        for y in 0..self.laid.1 {
            for x in 0..self.laid.0 {
                let from = ((y % high) * wide + x % wide) * 4;
                let [red, green, blue, alpha] = [rgba[from], rgba[from + 1], rgba[from + 2], rgba[from + 3]];
                self.drifting.extend([red, green, blue, (alpha as u32 * SEEN / 255) as u8]);
            }
        }
        console_colours(&mut self.drifting);
    }

    /// `camera`: what the view's middle shows of the floor; `clock`: frames.
    pub fn draw(&mut self, world: &mut Screen, weather: u8, art: &Art, camera: Vec2, clock: u64) {
        if self.weather != Some(weather) {
            self.change(weather, art);
        }
        let Some(layer) = self.layer.clone() else {
            return;
        };
        let close = world.scale as f32;
        let half = Vec2::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32) / 2.0;
        if !layer.drifts {
            if let Some((wide, high, rgba)) = layer.frames.get((clock as f32 / layer.ticks.max(1.0)) as usize % layer.frames.len().max(1)) {
                world.layer(rgba, *wide, *high, Vec2::new(*wide as f32, *high as f32) / 2.0, close, Laid::SeeThrough);
            }
            return;
        }
        let drift = DRIFT.iter().find(|(kind, _)| *kind == weather).map_or(Vec2::ZERO, |(_, drift)| *drift) * clock as f32;
        let side = Vec2::new(self.side.0 as f32, self.side.1 as f32);
        let from = (camera + drift).rem_euclid(side).floor();
        world.layer(&self.drifting, self.laid.0, self.laid.1, half + from, close, Laid::SeeThrough);
    }
}
