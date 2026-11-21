//! A game played by a script, with no window: keys frame by frame, or held
//! until something holds of the game (a mode, a place, a line of its log),
//! walking up to someone and talking, reading what is said. For tests that
//! go through a whole game as a player would (`e2e_tests`), and anyone who
//! wants to drive one.
//!
//! ```ignore
//! let mut driver = Driver::new(&mut game);
//! driver.press("enter");                          // the title's "New game"
//! driver.read_until(20_000, |game| game.mode() == Mode::Roam)?;
//! driver.talk_to("kecleon", 3_000)?;              // walked up to, Enter
//! driver.keys(&["enter", "down", "enter"]);       // one key a frame
//! ```

use glam::Vec2;
use shell::Keys;

use crate::{Game, Mode};

/// How near someone the player stands to talk to it, in pixels (the
/// engine's reach is 40).
const NEAR: f32 = 26.0;

/// Keys by their names, joined by `+`: `enter`, `escape`, `up`, `down`,
/// `left`, `right` (pressed and held: the way walked too), `tab`, `shift`,
/// `space`; any other is pressed by its name
/// (`m`, `x`, `f10`). Empty: nothing.
pub fn keys(names: &str) -> Keys {
    let mut keys = Keys::default();
    for name in names.split('+').map(str::trim).filter(|name| !name.is_empty()) {
        match name {
            "enter" => keys.enter = true,
            "escape" => keys.escape = true,
            "up" => (keys.up, keys.toward.y) = (true, -1.0),
            "down" => (keys.down, keys.toward.y) = (true, 1.0),
            "left" => (keys.left, keys.toward.x) = (true, -1.0),
            "right" => (keys.right, keys.toward.x) = (true, 1.0),
            "tab" => keys.tab = true,
            "shift" => keys.run = true,
            "space" => {
                keys.hurry = true;
                keys.named.push(name.to_string());
            }
            _ => {
                keys.named.push(name.to_string());
                keys.held.push(name.to_string());
            }
        }
    }
    keys
}

/// Where someone of the scene playing stands (its species' name, any case).
pub fn someone_at(game: &Game, name: &str) -> Option<Vec2> {
    let sim = &game.film()?.play().sim;
    sim.stage.cast.iter().find(|(who, _)| sim.cast.who(**who).1.eq_ignore_ascii_case(name)).map(|(_, actor)| actor.position)
}

/// Who is in the scene playing, by their species' names, and where.
pub fn cast_of(game: &Game) -> Vec<(String, Vec2)> {
    let Some(film) = game.film() else {
        return Vec::new();
    };
    let sim = &film.play().sim;
    sim.stage.cast.iter().map(|(who, actor)| (sim.cast.who(*who).1, actor.position)).collect()
}

/// A game being driven, and the frames it has played.
pub struct Driver<'a> {
    pub game: &'a mut Game,
    pub frames: usize,
}

impl<'a> Driver<'a> {
    pub fn new(game: &'a mut Game) -> Self {
        Driver { game, frames: 0 }
    }

    /// A frame with these keys.
    pub fn frame(&mut self, keys: Keys) {
        self.game.tick(keys, 1.0);
        self.frames += 1;
    }

    /// Keys pressed once (`keys` names them): a frame with them, a frame
    /// without, so the next press is a press again.
    pub fn press(&mut self, names: &str) {
        self.frame(keys(names));
        self.frame(Keys::default());
    }

    /// Each of these pressed in turn (see [`Driver::press`]).
    pub fn keys(&mut self, each: &[&str]) {
        for names in each {
            self.press(names);
        }
    }

    /// Keys held for so many frames.
    pub fn hold(&mut self, names: &str, frames: usize) {
        for _ in 0..frames {
            self.frame(keys(names));
        }
    }

    /// Nothing pressed for so many frames.
    pub fn wait(&mut self, frames: usize) {
        for _ in 0..frames {
            self.frame(Keys::default());
        }
    }

    /// Frames with the keys `keys` gives (the game, the frame counted from
    /// 0) until `done` holds, `most` frames at most: how many it took, or
    /// what the game was at.
    pub fn until(&mut self, most: usize, mut keys: impl FnMut(&Game, usize) -> Keys, done: impl Fn(&Game) -> bool) -> Result<usize, String> {
        for frame in 0..most {
            if done(self.game) {
                return Ok(frame);
            }
            let pressed = keys(self.game, frame);
            self.frame(pressed);
        }
        match done(self.game) {
            true => Ok(most),
            false => Err(self.lost(&format!("not done after {most} frames"))),
        }
    }

    /// Nothing pressed until `done` holds.
    pub fn wait_until(&mut self, most: usize, done: impl Fn(&Game) -> bool) -> Result<usize, String> {
        self.until(most, |_, _| Keys::default(), done)
    }

    /// Enter pressed every other frame until `done` holds: the lines read
    /// as they come, each question answered by its first choice.
    pub fn read_until(&mut self, most: usize, done: impl Fn(&Game) -> bool) -> Result<usize, String> {
        self.until(most, |_, frame| Keys { enter: frame % 2 == 0, ..Keys::default() }, done)
    }

    /// The player walked up to that one (its species' name), within talking
    /// reach, facing it. Stuck against something, it goes round for a moment.
    pub fn walk_to(&mut self, name: &str, most: usize) -> Result<usize, String> {
        // (Who comes to its place walks there first: waited for, standing still a moment.)
        let mut still = (someone_at(self.game, name), 0);
        for _ in 0..most {
            if still.1 >= 20 {
                break;
            }
            self.frame(Keys::default());
            let now = someone_at(self.game, name);
            still = match (now, still.0) {
                (Some(now), Some(before)) if now.distance(before) < 0.1 => (Some(now), still.1 + 1),
                _ => (now, 0),
            };
        }
        let Some(mut last) = self.game.walked_at() else {
            return Err(self.lost("nobody walks"));
        };
        let (mut stuck, mut around, mut detours) = (0, 0usize, 0usize);
        let name = name.to_string();
        let near = |game: &Game| match (game.walked_at(), someone_at(game, &name)) {
            (Some(at), Some(there)) => at.distance(there) <= NEAR,
            _ => false,
        };
        let took = self.until(
            most,
            |game, _| {
                let (Some(at), Some(there)) = (game.walked_at(), someone_at(game, &name)) else {
                    return Keys::default();
                };
                let way = (there - at).normalize_or_zero();
                stuck = if at.distance(last) < 0.25 { stuck + 1 } else { 0 };
                last = at;
                // (Stuck: aside, one side then the other, each time a little
                // longer, until round what is in the way.)
                if stuck > 12 {
                    detours += 1;
                    (stuck, around) = (0, (24 * (1 + detours / 2)).min(120));
                }
                let toward = match around.checked_sub(1) {
                    Some(left) => {
                        around = left;
                        if detours % 2 == 1 { Vec2::new(-way.y, way.x) } else { Vec2::new(way.y, -way.x) }
                    }
                    None => way,
                };
                Keys { toward, ..Keys::default() }
            },
            near,
        )?;
        // (Turned to it: a step toward it, then still.)
        if let (Some(at), Some(there)) = (self.game.walked_at(), someone_at(self.game, &name)) {
            self.frame(Keys { toward: (there - at).normalize_or_zero(), ..Keys::default() });
        }
        self.wait(2);
        Ok(took)
    }

    /// The player walked toward that point of the place (a zone, a way
    /// out) until `done` holds; stuck against something, it goes round for
    /// a moment.
    pub fn walk_until(&mut self, point: Vec2, most: usize, done: impl Fn(&Game) -> bool) -> Result<usize, String> {
        let Some(mut last) = self.game.walked_at() else {
            return Err(self.lost("nobody walks"));
        };
        let (mut stuck, mut around, mut detours) = (0, 0usize, 0usize);
        self.until(
            most,
            |game, _| {
                let Some(at) = game.walked_at() else {
                    return Keys::default();
                };
                let way = (point - at).normalize_or_zero();
                stuck = if at.distance(last) < 0.25 { stuck + 1 } else { 0 };
                last = at;
                // (Stuck: aside, one side then the other, each time a little
                // longer, until round what is in the way.)
                if stuck > 12 {
                    detours += 1;
                    (stuck, around) = (0, (24 * (1 + detours / 2)).min(120));
                }
                let toward = match around.checked_sub(1) {
                    Some(left) => {
                        around = left;
                        if detours % 2 == 1 { Vec2::new(-way.y, way.x) } else { Vec2::new(way.y, -way.x) }
                    }
                    None => way,
                };
                Keys { toward, ..Keys::default() }
            },
            done,
        )
    }

    /// That one walked up to and talked to (Enter), until the game hears of
    /// it (`talk <name>` in its log).
    pub fn talk_to(&mut self, name: &str, most: usize) -> Result<usize, String> {
        let took = self.walk_to(name, most)?;
        let said = format!("talk {name}");
        let before = self.game.log.iter().filter(|line| **line == said).count();
        let talked = move |game: &Game| game.log.iter().filter(|line| **line == said).count() > before;
        Ok(took + self.until(1_500, |_, frame| Keys { enter: frame % 8 == 0, ..Keys::default() }, talked)?)
    }

    /// Played on until the player walks a place again (a film over, a
    /// screen left), the lines read as they come.
    pub fn read_to_roaming(&mut self, most: usize) -> Result<usize, String> {
        let roaming = |game: &Game| game.mode() == Mode::Roam && game.cutscene.is_none() && game.walked_at().is_some() && game.words().is_none();
        let mut took = self.read_until(most, roaming)?;
        // (The place's scene made again as the story has it now, a few frames on: roaming still.)
        self.wait(10);
        took += 10 + self.read_until(most, roaming)?;
        Ok(took)
    }

    /// Why a script could not go on, with what the game was at.
    pub fn lost(&self, why: &str) -> String {
        let game = &*self.game;
        format!(
            "{why} (frame {}): mode {:?}, chapter {}, place {:?}, at {:?}, said {:?}, line {:?}\ncast: {:?}\nlog: {:?}",
            self.frames,
            game.mode(),
            game.story.chapter,
            game.story.place,
            game.walked_at(),
            game.said,
            game.film().and_then(preview::film::Film::page),
            cast_of(game),
            game.log
        )
    }
}
