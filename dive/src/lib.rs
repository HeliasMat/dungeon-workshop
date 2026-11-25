//! A dungeon on screen: the floor and those on it, the interface, the
//! pause menu and the bag. Played by the films one
//! plays (`preview`), each handing it the screens (`Frame`) and the run.
//!
//! The mode itself: the pause menu and the bag. A frame: the keys become an `Input` (`keys`), the
//! rules play it, what they tell is seen to (`sights`), then all is drawn
//! (`world`, `hud`). All of it reads `Run`; what is kept here is only how
//! things look and where menus' cursors are.

mod actors;
mod bag;
mod dark;
mod floor;
mod hud;
mod icons;
mod keys;
pub mod look;
mod messages;
mod pages;
mod pause;
mod shows;
mod sights;
mod talk;
mod world;

/// A floor drawn alone with its tileset, and how large that picture is.
pub use floor::{HIGH as FLOOR_HIGH, WIDE as FLOOR_WIDE, drawn as floor_drawn};

use std::collections::HashMap;

use glam::Vec2;
use rules::dungeon::Pos;
use rules::run::dungeon::Outcome;
use rules::run::{MonsterId, Options};
use shell::Keys;
use shell::images::Behind;
use shell::sound::Hears;

use rules::run::Run;
use rules::run::options::saved;
use shell::Screen;
use shell::art::Art;
use stage_view::ui::{Canvas, UiArt};
use std::sync::Arc;

/// Screen pixels per pixel of the original's screens.
pub const SCALE: usize = 2;

/// What the screens are drawn with: the lower screen (what is walked in or
/// explored), both screens' interface, the pictures, the sound.
pub struct Frame {
    /// The lower screen, what is walked in or explored.
    pub world: Screen,
    /// Both screens' interface, cleared each frame.
    pub canvas: Canvas,
    pub art: Art,
    /// The interface's pictures, font and words; none before `extractor`.
    pub ui: Option<Arc<UiArt>>,
    /// None: no sound to be had, or a check.
    pub audio: Option<Box<dyn Hears>>,
    pub folder: String,
    pub blink: look::Blink,
    /// Saves go to files of their own under `target/`, away from the
    /// player's (checks, and a game started elsewhere than the title).
    pub scratch: bool,
}

impl Frame {
    /// The screens of a game, its pictures read from `folder` (`art`, its
    /// project's own pictures too).
    pub fn new(folder: &str, art: Art, ui: Option<Arc<UiArt>>, audio: Option<Box<dyn Hears>>) -> Self {
        Frame {
            world: Screen::new(SCALE),
            canvas: Canvas::default(),
            art,
            ui,
            audio,
            folder: folder.to_string(),
            blink: look::Blink::default(),
            scratch: false,
        }
    }

    /// Where the dungeon's quicksave is kept.
    pub fn quicksave_path(&self) -> String {
        match self.scratch {
            true => "target/dungeon_test_save.txt".to_string(),
            false => saved("dungeon_save.txt"),
        }
    }

    /// A song, on the music's channel, as the options want it.
    pub fn play_song(&mut self, options: &Options, song: Option<u16>) {
        let (song, level) = (song.filter(|_| options.music && options.volume > 0), options.volume as f32 / 10.0);
        if let Some(audio) = &mut self.audio {
            audio.song(0, song, level, 0.0);
        }
    }
}

/// The dungeon's share of a frame: the run, the screens, its own state, and
/// how it ended (if it did).
pub struct Turn<'a, M> {
    pub run: &'a mut Run,
    pub show: &'a mut Frame,
    pub here: &'a mut M,
    /// What comes after this frame.
    pub next: &'a mut Option<Done>,
}

/// The dungeon over.
pub enum Done {
    /// Left by the pause menu's way to the title.
    ToTitle,
    /// Left: cleared, fainted, abandoned…, and what is said of it.
    Left { outcome: Outcome, message: String },
}

/// One tile of a floor, in pixels of the game.
pub const TILE: i32 = 24;
const SCREEN: Vec2 = Vec2::new(stage_view::ui::SCREEN_WIDTH as f32, stage_view::ui::SCREEN_HEIGHT as f32);

/// The middle of a tile, in pixels of the floor's picture.
fn tile_middle(at: Pos) -> Vec2 {
    Vec2::new((at.x * TILE + TILE / 2) as f32, (at.y * TILE + TILE / 2) as f32)
}

/// A sound of the game, as loud as the options have it.
fn sound(audio: &Option<Box<dyn Hears>>, options: &Options, id: u16) {
    if let Some(audio) = audio.as_ref().filter(|_| options.volume > 0) {
        audio.sound(id, options.volume as f32 / 10.0, 1.0);
    }
}

/// What is kept of a dungeon on screen.
#[derive(Default)]
pub struct Dive {
    /// The team chose this dungeon: its jobs are paid on leaving.
    chosen: bool,
    pause: pause::Pause,
    bag: bag::BagMenu,
    talk: talk::Talk,
    /// A menu just closed: the key held for it does nothing until let go.
    let_go: bool,
    /// What the music is now (it is played again if the options allow it).
    song: Option<u16>,
    picture: floor::FloorPicture,
    looks: floor::Looks,
    actors: HashMap<MonsterId, actors::Actor>,
    shows: shows::Shows,
    dark: dark::Dark,
    sky: dark::Sky,
    map: hud::Map,
    top: hud::Top,
    messages: messages::Messages,
    /// Those of the floor, by depth; a list kept not to be made anew.
    order: Vec<(i32, MonsterId)>,
    behind: Behind,
    /// The leader is turned in place (Y or Shift held): the ways it may face are shown.
    turning: bool,
}

/// Enters a dungeon by its number; `chosen`: the team chose it, its jobs
/// count (not one the story sends to).
pub fn enter(game: &mut Turn<'_, Dive>, id: u16, chosen: bool) {
    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |time| time.subsec_nanos());
    let seed = (game.run.frame() as u32).wrapping_mul(0x9E37_79B1) ^ time;
    match chosen {
        true => game.run.enter_dungeon(id, seed),
        false => game.run.enter_story_dungeon(id, seed),
    }
    *game.here = Dive { chosen, ..Dive::default() };
}

pub fn frame(game: &mut Turn<'_, Dive>, keys: &Keys) {
    if game.run.dungeon().is_none() {
        return leave(game);
    }
    let input = keys::read(game, keys);
    let told = game.run.step(input);
    sights::see(game, told);
    if game.here.pause.leaving {
        *game.next = Some(Done::ToTitle);
        return;
    }
    if game.run.dungeon().is_some_and(|dungeon| dungeon.outcome.is_some()) {
        return leave(game);
    }
    sights::advance(game);
    world::draw(game);
    hud::draw(game);
}

/// The run is over: back where the team came from.
fn leave(game: &mut Turn<'_, Dive>) {
    let chosen = game.here.chosen;
    *game.here = Dive::default();
    game.show.art.forget_sprites();
    let left = game.run.leave_dungeon();
    let (outcome, mut message) = left.map_or((Outcome::Abandoned, String::new()), |left| (left.outcome, left.message));
    // (Items as scenes name them, `{item:12}`: whoever tells it knows.)
    if chosen && let Some(paid) = game.run.pay_jobs() {
        message.push('\n');
        message.push_str(&paid);
    }
    *game.next = Some(Done::Left { outcome, message });
}
