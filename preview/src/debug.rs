//! What a film shows and lets do for whoever writes it, as SkyTemple's
//! script debugger does: over the scene, who stands where and their box,
//! where the player may talk or go out, the places the film names, the
//! cells one cannot walk on, the film's line and scene; and ways to go
//! faster (lines written at once, no fades, a dungeon's floors gone
//! through by themselves, a scene to start at).
//!
//! `DUNGEON_FILM_DEBUG` turns them on, words between commas: `view`, `text`,
//! `no_fade`, `floors`, `all` (the four), `scene=3`; a program's `main`
//! reads it ([`Debug::read`]) and hands it down (none: all off). In the
//! window F6 to F9 turn each on and off, Ctrl and a digit go to that scene.

use engine::{TILE, Who};
use glam::Vec2;
use media::font::Font;
use shell::Screen;

use crate::film::Film;

/// The variable that turns them on, read by the programs' `main`.
pub const VARIABLE: &str = "DUNGEON_FILM_DEBUG";

/// The frames a dungeon's floor is seen before `floors` goes on to the next.
const ON_A_FLOOR: u64 = 90;

/// The developer's options of a film played.
#[derive(Clone, Debug, Default)]
pub struct Debug {
    /// What the scene holds drawn over it (`view`, F6).
    pub shown: bool,
    /// Lines written at once: one press passes each (`text`, F7).
    pub fast_text: bool,
    /// No fade: the scene always seen (`no_fade`, F8).
    pub no_fades: bool,
    /// In a dungeon, on to the next floor a moment after reaching each, up
    /// to the last (`floors`, F9; in a dungeon F9 goes down at once).
    pub floors: bool,
    /// The scene to start at, from 1 (`scene=3`).
    pub scene: Option<usize>,
    /// The film's names for those it shows; its text, a line each.
    pub names: Vec<(Who, String)>,
    pub source: Vec<String>,
    /// The floor a dungeon was on, and the frame of the dungeon it was reached at.
    floor_seen: Option<(usize, u64)>,
}

impl Debug {
    /// As those words say (`view,text`), any not known left.
    pub fn read(words: &str) -> Self {
        let mut debug = Debug::default();
        for word in words.split([',', ' ', ';']).map(str::trim).filter(|word| !word.is_empty()) {
            match word {
                "view" | "1" => debug.shown = true,
                "text" => debug.fast_text = true,
                "no_fade" | "no_fades" => debug.no_fades = true,
                "floors" => debug.floors = true,
                "all" => (debug.shown, debug.fast_text, debug.no_fades, debug.floors) = (true, true, true, true),
                _ => {
                    if let Some(number) = word.strip_prefix("scene=") {
                        debug.scene = number.parse().ok().filter(|scene| *scene > 0);
                    }
                }
            }
        }
        debug
    }

    /// The film's names (`Written::names`: each code and its name) and its text.
    pub fn know(&mut self, names: &[(String, String)], text: &str) {
        self.names = names.iter().filter_map(|(code, name)| Some((Who::parse(code)?, name.clone()))).collect();
        self.source = text.lines().map(str::to_string).collect();
    }

    /// What is on, shortly, each with its key.
    pub fn said(&self) -> String {
        let on = |on: bool| if on { "on" } else { "off" };
        format!(
            "F6 view {}  -  F7 quick text {}  -  F8 no fades {}  -  F9 floors {}  -  Ctrl+n: scene n",
            on(self.shown),
            on(self.fast_text),
            on(self.no_fades),
            on(self.floors)
        )
    }

    /// Whether `floors` goes on from that floor now, the dungeon at that frame.
    pub fn floor_done(&mut self, floor: Option<(usize, usize)>, frame: u64) -> bool {
        let Some((at, of)) = floor.filter(|_| self.floors) else {
            return false;
        };
        match self.floor_seen {
            Some((seen, since)) if seen == at => at < of && frame >= since + ON_A_FLOOR,
            _ => {
                self.floor_seen = Some((at, frame));
                false
            }
        }
    }
}

const RED: [u8; 3] = [255, 70, 70];
const ORANGE: [u8; 3] = [255, 160, 40];
const YELLOW: [u8; 3] = [255, 230, 60];
const GREEN: [u8; 3] = [90, 240, 110];
const CYAN: [u8; 3] = [80, 220, 255];
const WHITE: [u8; 3] = [255, 255, 255];
const PINK: [u8; 3] = [255, 120, 220];

/// A line of text on a dark band, sharp; how wide it is.
fn text(world: &mut Screen, font: &Font, (x, y): (i32, i32), words: &str, colour: [u8; 3]) -> i32 {
    let (wide, high, rgba) = font.render_colored(words, colour);
    world.fill(x - 1, y - 1, wide as i32 + 2, high as i32 + 1, [0, 0, 0, 170]);
    for (at, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
        if pixel[3] > 0 {
            world.fill(x + (at % wide.max(1)) as i32, y + (at / wide.max(1)) as i32, 1, 1, *pixel);
        }
    }
    wide as i32
}

/// The edges of a box, a pixel of the screen thick.
fn outline(world: &mut Screen, (left, top): (i32, i32), (right, bottom): (i32, i32), [red, green, blue]: [u8; 3]) {
    let colour = [red, green, blue, 230];
    let (wide, high) = ((right - left).max(1), (bottom - top).max(1));
    world.fill(left, top, wide, 1, colour);
    world.fill(left, bottom, wide + 1, 1, colour);
    world.fill(left, top, 1, high, colour);
    world.fill(right, top, 1, high, colour);
}

/// A small cross at a point.
fn cross(world: &mut Screen, (x, y): (i32, i32), [red, green, blue]: [u8; 3]) {
    let colour = [red, green, blue, 255];
    world.fill(x - 3, y, 7, 1, colour);
    world.fill(x, y - 3, 1, 7, colour);
}

/// The scene as the film holds it, drawn over what is shown of it.
pub fn draw(film: &Film, world: &mut Screen) {
    let scale = world.scale as f32;
    let view = film.view().scaled(scale);
    let on = |at: Vec2| {
        let at = view.on_screen(at);
        (at.x.round() as i32, at.y.round() as i32)
    };
    let font = &film.art.ui.font;
    let sim = &film.play().sim;
    let stage = &sim.stage;
    let labels = scale >= 2.0;
    // (The cells one cannot walk on.)
    if let Some(ground) = &stage.place.ground {
        let tile = TILE * view.close;
        for (row, cells) in ground.rows().iter().enumerate() {
            for (column, solid) in cells.iter().enumerate() {
                if *solid {
                    let (x, y) = on(Vec2::new(column as f32, row as f32) * TILE);
                    let [red, green, blue] = RED;
                    world.fill(x, y, tile.ceil() as i32, tile.ceil() as i32, [red, green, blue, 70]);
                }
            }
        }
    }
    // (The scene's own walls, `wall`; the ground's; where the player may go.)
    for wall in stage.play.walls() {
        outline(world, on(Vec2::new(wall.left, wall.top)), on(Vec2::new(wall.right, wall.bottom)), ORANGE);
    }
    if let Some(bounds) = &stage.play.bounds {
        outline(world, on(Vec2::new(bounds.left, bounds.top)), on(Vec2::new(bounds.right, bounds.bottom)), CYAN);
    }
    // (The ways out and in, the points the film names.)
    let (talkers, doors) = sim.director.roaming_known();
    for (scene, at, half) in &doors {
        let (x, y) = on(*at);
        let (wide, high) = ((half.x * view.close).round() as i32, (half.y * view.close).round() as i32);
        outline(world, (x - wide, y - high), (x + wide, y + high), GREEN);
        if labels {
            text(world, font, (x - wide, y - high - 13), &format!("way out -> {scene}"), GREEN);
        }
    }
    for (name, at) in &stage.place.entrances {
        let (x, y) = on(*at);
        cross(world, (x, y), GREEN);
        if labels {
            text(world, font, (x + 4, y + 2), &format!("entrance {name}"), GREEN);
        }
    }
    let mut spots: Vec<(&String, &Vec2)> = stage.place.spots.iter().collect();
    spots.sort_by(|a, b| a.0.cmp(b.0));
    for (name, at) in spots {
        let (x, y) = on(*at);
        cross(world, (x, y), PINK);
        if labels {
            text(world, font, (x + 4, y + 2), name, PINK);
        }
    }
    // (Each one on the stage: its box, its name, where it stands.)
    for (who, actor) in &stage.cast {
        let size = stage.view.bodies.get(who).copied().unwrap_or(Vec2::new(24.0, 24.0));
        let (x, y) = on(actor.position);
        let half = (size * view.close / 2.0).round();
        let colour = match () {
            _ if stage.play.controlled == Some(*who) => CYAN,
            _ if talkers.contains(who) => YELLOW,
            _ if who.is_thing() => ORANGE,
            _ => WHITE,
        };
        outline(world, (x - half.x as i32, y - (half.y * 2.0) as i32), (x + half.x as i32, y), colour);
        cross(world, (x, y), colour);
        if labels {
            let name = film.debug.names.iter().find(|(one, _)| one == who).map_or_else(|| sim.cast.who(*who).1, |(_, name)| name.clone());
            let name = if name.is_empty() { who.to_string() } else { name };
            let talk = if talkers.contains(who) { " (talk)" } else { "" };
            let words = format!("{name} {:.0},{:.0}{talk}", actor.position.x, actor.position.y);
            text(world, font, (x - half.x as i32, y + 2), &words, colour);
        }
    }
}

/// Over all, the box too: above, the film's line now, its scene, the ways
/// out; below, what is on.
pub fn draw_told(film: &Film, world: &mut Screen) {
    let (font, sim) = (&film.art.ui.font, &film.play().sim);
    let labels = world.scale >= 2;
    let (_, doors) = sim.director.roaming_known();
    let line = sim.director.numbers().get(engine::LINE).copied().unwrap_or(0.0) as usize;
    let written = film.debug.source.get(line.wrapping_sub(1)).map_or(String::new(), |words| format!(": {}", words.trim()));
    let written: String = written.chars().take(90).collect();
    let scene = sim.director.scene_name();
    let number = film.scenes().len();
    text(world, font, (4, 3), &format!("scene {number} \"{scene}\"  -  line {line}{written}"), YELLOW);
    if labels {
        // (The ways out, even those out of the view.)
        let ways: Vec<String> = doors.iter().map(|(scene, at, _)| format!("{scene} ({:.0},{:.0})", at.x, at.y)).collect();
        if !ways.is_empty() {
            text(world, font, (4, 17), &format!("ways out: {}", ways.join(", ")), GREEN);
        }
        text(world, font, (4, world.high as i32 - 15), &film.debug.said(), WHITE);
    }
}

/// The same in a dungeon: its floor, and what is on.
pub fn draw_dive(floor: Option<(usize, usize)>, debug: &Debug, font: &Font, world: &mut Screen) {
    let floor = floor.map_or(String::new(), |(at, of)| format!("  -  floor {at}/{of}"));
    text(world, font, (4, 3), &format!("dungeon{floor}  -  F9: next floor"), YELLOW);
    if world.scale >= 2 {
        text(world, font, (4, world.high as i32 - 15), &debug.said(), WHITE);
    }
}
