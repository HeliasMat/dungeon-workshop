//! The game: what conducts the rest. It holds the one truth (`Story`, and
//! the rules' `Run` in `Playthrough`), and goes from mode to mode: the title
//! screen, a place walked about (`roam`), a film (a cutscene), a dungeon, a
//! screen of the game (a shop, the bank…). The films, the places, the
//! dungeons only know each other by their names; the game's file
//! (`game_file::GameFile`) and the places' life say what follows what.
//!
//! A film asks the game for what only it does by a `GameRequest`; the game
//! answers, and goes on. A film played alone is a game too: one with no
//! file, the film its only cutscene (`Game::of_film`).
//!
//! The films themselves are played and drawn by `preview`: the game uses it,
//! never the other way.
//!
//! - `story`: where the story stands (its chapter, its flags, its place);
//! - `game_file`: the game's file (`<name>.game`): its chapters and their rules;
//! - `roam`: a place walked about, made a scene of its decor's life;
//! - `playthrough`: the rules' game (the team, its recruits, its bag, its
//!   money), kept from one scene, one dungeon, one save to the next;
//! - `dive`: a dungeon of one's own gone through (`.dungeon`, `.fight`);
//! - `game_screens`: the game's screens (shops, bank, storage, jobs,
//!   recruits, the world map), over the scene;
//! - `dungeon_check`, `project_check`: what is wrong in a dungeon, in a whole game;
//! - `pack`: a game shared as a `.dwpack`, its manifest in `manifest`;
//! - `book`: the rules' book, read from the data;
//! - `drive`: a game played by a script at the keys, with no window (the end-to-end tests).

pub mod book;
pub mod dive;
#[cfg(test)]
mod dive_tests;
pub mod drive;
pub mod dungeon_check;
#[cfg(test)]
mod dungeon_rules_tests;
#[cfg(test)]
mod e2e_tests;
mod error;
pub mod game_file;
pub mod game_screens;
pub mod manifest;
pub mod pack;
pub mod playthrough;
pub mod project_check;
pub mod roam;
pub mod setup;
pub mod story;
#[cfg(test)]
mod testing;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use preview::art::Art;
use preview::film::Film;
use shell::sound::Hears;
use shell::{Keys, Screen};
use stage_view::ui::message::Words;
use stage_view::ui::{Align, Canvas, ListCursor, Picked, color};

use crate::dive::Diving;
use crate::game_screens::GameScreen;
use crate::playthrough::Playthrough;
use game_file::{Action, GameFile, Rule, When};
use roam::Arrival;
use story::Story;

pub use assets::project::Project;
pub use engine::GameAsk;
pub use error::{Error, PackWrong};
pub use preview::film::GameRequest;

/// What the game is doing now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Title,
    Roam,
    Cutscene,
    Dungeon,
    Screen,
    /// The game finished: its end card, until a key goes back to the title screen.
    End,
    Over,
}

/// A game's title screen: its name, the game saved if there is one, the choice under the cursor.
pub struct Title {
    pub name: String,
    /// The game saved, if there is one.
    pub saved: Option<String>,
    pub cursor: ListCursor,
}

impl Title {
    /// Its choices: a new game, the one saved (if there is one), leaving.
    fn options(&self) -> Vec<&'static str> {
        match self.saved.is_some() {
            true => vec!["Continue", "New game", "Quit"],
            false => vec!["New game", "Quit"],
        }
    }

    /// The title screen drawn: its name above, its choices below.
    pub fn draw(&self, canvas: &mut Canvas, ui: &stage_view::ui::UiArt, world: &mut Screen) {
        // (On the lower screen, the one a film shows: its name in a window, the choices beside.)
        card(canvas, ui, 5, &[(&self.name, color::YELLOW)]);
        let options: Vec<String> = self.options().into_iter().map(str::to_string).collect();
        canvas.choice_box(ui, &options, self.cursor, false);
        world.rgba.fill(0);
        world.over(&canvas.data);
    }
}

/// A screen of the game's own over its backdrop (the title, the end card):
/// a window on the lower screen, `high` tiles, its lines centred.
fn card(canvas: &mut Canvas, ui: &stage_view::ui::UiArt, high: usize, lines: &[(&str, [u8; 3])]) {
    use stage_view::ui::{SCREEN_HEIGHT, TILE};
    canvas.clear();
    canvas.backdrop_rows(ui, 0..SCREEN_HEIGHT * 2);
    canvas.text_panel(ui, (2, SCREEN_HEIGHT / TILE + 3, 28, high), lines, (15, 16), Align::Centred);
}

/// The place walked about now: its word, its scene (none while a film has
/// what it plays with), the text it was made of, whether it holds for the
/// game (a talk).
pub struct Place {
    pub word: String,
    pub film: Option<Film>,
    pub text: String,
    held: bool,
    /// Where the player stood when its scene was put away.
    at: Option<(i32, i32)>,
    /// What to say as it opens next (a K.O.'s cost).
    told: Vec<String>,
    /// How its scene was made: the way the team came in by, whether back where it stood.
    arrival: Arrival,
    back: bool,
}

/// Actions being played: those, and the next.
struct Running {
    actions: Arc<Vec<Action>>,
    next: usize,
}

/// The game being played.
pub struct Game {
    /// Where its films, places, dungeons, fights, pictures and sounds are
    /// (the folder of its `.game`; a film alone: its art's).
    pub folders: Project,
    /// The game's file (none: a film alone).
    pub project: Option<GameFile>,
    pub story: Story,
    /// The rules' game (the team, the bag, the money), once it is made;
    /// whether it could not be (no data).
    pub playthrough: Option<Playthrough>,
    playthrough_failed: bool,
    /// The folder of the game's files.
    data: String,
    /// What the program works with: the library, the data, the tables read.
    context: Arc<film::Context>,
    /// What films play with, while no film has it.
    art: Option<Art>,
    audio: Option<Box<dyn Hears>>,
    pub title: Option<Title>,
    /// The film playing (a cutscene), and its word.
    pub cutscene: Option<(Film, String)>,
    pub place: Option<Place>,
    pub diving: Option<Diving>,
    pub screen: Option<(GameScreen, Words)>,
    running: Vec<Running>,
    /// Nobody at the keys: films go by themselves, dungeons as cleared, a
    /// place's talks once each, then its ways out.
    pub nobody: bool,
    talked: BTreeSet<(u16, String, String)>,
    /// Nobody at the keys: the ways out walked in each chapter (from where, which), and the places seen.
    walked: BTreeSet<(u16, String, usize)>,
    seen: BTreeSet<(u16, String)>,
    /// Nobody at the keys: the zones walked into in each chapter (where, which).
    zoned: BTreeSet<(u16, String, String)>,
    /// The dungeon (or fight) the team is in, by its word, for its `on won` / `on lost`.
    dived: Option<String>,
    /// The words of the ways the map open now shows, in its order.
    map_ways: Vec<String>,
    /// Nothing changed since the place was last made as the story has it.
    settled: bool,
    /// Nobody at the keys, and nothing left to do in the place: the game rests there.
    pub rested: bool,
    /// Where games are saved (`saves/`, in the folder it is played from).
    pub saves: PathBuf,
    /// What happened, in order (`chapter 1`, `film intro`, `dungeon forest won`…).
    pub log: Vec<String>,
    /// What to tell whoever plays (a save written, why something could not be).
    pub said: Option<String>,
    /// The game finished (`finish`, its last chapter played out): its end
    /// card shown until a key; nobody at the keys, it is over at once.
    pub ended: bool,
    pub over: bool,
    draw: rules::run::town::Draw,
    pub debug: preview::debug::Debug,
    /// The auto of the films (lines read by themselves).
    pub auto: bool,
    /// The places built on demand (or tried), each once.
    built: BTreeSet<String>,
}

impl Game {
    /// A game of nothing yet, with what it plays with.
    fn bare(context: &Arc<film::Context>, folders: Project, art: Option<Art>, audio: Option<Box<dyn Hears>>) -> Game {
        let data = context.data.as_str();
        Game {
            context: context.clone(),
            folders,
            project: None,
            story: Story::default(),
            playthrough: None,
            playthrough_failed: false,
            data: data.to_string(),
            art,
            audio,
            title: None,
            cutscene: None,
            place: None,
            diving: None,
            screen: None,
            running: Vec::new(),
            nobody: false,
            talked: BTreeSet::new(),
            walked: BTreeSet::new(),
            seen: BTreeSet::new(),
            zoned: BTreeSet::new(),
            dived: None,
            map_ways: Vec::new(),
            settled: false,
            rested: false,
            saves: PathBuf::from("saves"),
            log: Vec::new(),
            said: None,
            ended: false,
            over: false,
            draw: rules::run::town::Draw(seed()),
            debug: preview::debug::Debug::default(),
            auto: false,
            built: BTreeSet::new(),
        }
    }

    /// A film played alone, as a game: what it asks done as it asks it.
    pub fn of_film(mut film: Film, context: &Arc<film::Context>) -> Game {
        film.hosted = true;
        let mut game = Game::bare(context, film.art.project.clone(), None, None);
        game.data = film.art.folder.clone();
        (game.debug, game.auto) = (film.debug.clone(), film.auto);
        let word = film
            .path
            .as_deref()
            .map_or_else(String::new, |path| Path::new(path).file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned()));
        game.cutscene = Some((film, word));
        game
    }

    /// The game of that file (`<name>.game`), on its title screen.
    /// (Its places, films and dungeons its own, then the context's library's.)
    pub fn open(context: &Arc<film::Context>, path: &Path, audio: Option<Box<dyn Hears>>) -> Result<Game, Error> {
        let project = GameFile::open(path)?;
        if let Some(problem) = project.problems.first() {
            return Err(Error::Wrong { file: path.display().to_string(), first: problem.clone() });
        }
        let folders = context.library.of_game(path);
        let art = Art::open_in(&context.data, folders.clone())?;
        let mut game = Game::bare(context, folders, Some(art), audio);
        let saved = std::fs::read_to_string(story::save_file(Path::new("saves"), &project.name)).ok();
        game.title =
            Some(Title { name: if project.title.is_empty() { project.name.clone() } else { project.title.clone() }, saved, cursor: ListCursor::default() });
        game.project = Some(project);
        Ok(game)
    }

    /// Past the title screen, a new game.
    pub fn skip_title(&mut self) {
        if self.title.take().is_some() && self.project.is_some() {
            self.new_game();
        }
    }

    /// What the game is doing now.
    pub fn mode(&self) -> Mode {
        match () {
            _ if self.title.is_some() => Mode::Title,
            _ if self.ended => Mode::End,
            _ if self.diving.is_some() => Mode::Dungeon,
            _ if self.screen.is_some() => Mode::Screen,
            _ if self.cutscene.is_some() && !(self.project.is_some() && self.cutscene.as_ref().is_some_and(|(film, _)| film.over)) => Mode::Cutscene,
            _ if self.over => Mode::Over,
            _ if self.place.is_some() => Mode::Roam,
            _ => Mode::Over,
        }
    }

    /// The film playing (the cutscene, else the place's scene).
    pub fn film(&self) -> Option<&Film> {
        self.cutscene.as_ref().map(|(film, _)| film).or_else(|| self.place.as_ref().and_then(|place| place.film.as_ref()))
    }

    /// The same, to change it.
    pub fn film_mut(&mut self) -> Option<&mut Film> {
        match &mut self.cutscene {
            Some((film, _)) => Some(film),
            None => self.place.as_mut().and_then(|place| place.film.as_mut()),
        }
    }

    /// The box of the film playing.
    pub fn words(&self) -> Option<&Words> {
        self.film()?.words()
    }

    /// A number of the film playing.
    pub fn number(&self, name: &str) -> Option<f64> {
        self.film()?.number(name)
    }

    /// Where the one the player walks stands, in the film playing.
    pub fn walked_at(&self) -> Option<glam::Vec2> {
        self.film()?.walked_at()
    }

    /// Where each scene of the film playing started.
    pub fn scenes(&self) -> &[f32] {
        self.film().map_or(&[], Film::scenes)
    }

    /// How far the film playing has gone, in frames.
    pub fn played(&self) -> f32 {
        self.film().map_or(0.0, Film::played)
    }

    /// The film alone, given back (to be played again from elsewhere).
    pub fn into_film(self) -> Option<Film> {
        self.cutscene.map(|(film, _)| film)
    }

    /// The film alone, out of the game for a moment (to go elsewhere in it), and put back.
    pub fn take_film(&mut self) -> Option<Film> {
        self.cutscene.take().map(|(film, _)| film)
    }

    pub fn put_film(&mut self, mut film: Film) {
        film.hosted = true;
        let word = film.path.clone().unwrap_or_default();
        self.cutscene = Some((film, word));
    }

    // ----- the game played -----

    /// A frame goes by.
    pub fn tick(&mut self, keys: Keys, frames: f32) {
        if self.title.is_some() {
            self.title_keys(&keys);
            return;
        }
        if self.ended {
            if keys.enter || keys.escape {
                self.back_to_title();
            }
            return;
        }
        // (A dungeon gone through: it has the keys; over, the game goes on knowing how.)
        if let Some(diving) = &mut self.diving {
            diving.play(&keys, frames);
            // (`floors`: on to the next floor a moment after each is reached.)
            if self.debug.floor_done(diving.floor(), diving.clock()) {
                diving.skip_floor();
            }
            if diving.ended.is_some() {
                self.dive_over();
            }
            return;
        }
        // (A screen of the game open: it has the keys; left, what asked for it goes on.)
        if let Some((mut screen, _)) = self.screen.take() {
            let Some(playthrough) = self.playthrough.as_mut() else {
                self.screen_left(0);
                return;
            };
            let (left, shown) = screen.play(&keys, playthrough);
            // (Saved from the menu: written now, said on it.)
            if let GameScreen::Menu { save, said, .. } = &mut screen
                && std::mem::take(save)
            {
                self.screen = Some((GameScreen::Menu { cursor: ListCursor::at(2), said: None, save: false }, Words::default()));
                let told = match self.save() {
                    Ok(_) => "Game saved.".to_string(),
                    Err(why) => why.to_string(),
                };
                self.screen = None;
                *said = Some(told);
            }
            match left {
                true => self.screen_left(screen.chosen()),
                false => self.screen = Some((screen, shown)),
            }
            return;
        }
        if let Some((film, _)) = &mut self.cutscene {
            let keys = if self.nobody { Keys { enter: film.asking() || film.page().is_some(), ..Keys::default() } } else { keys };
            film.tick(keys, frames);
            for request in film.requests() {
                self.request(request);
            }
            if self.project.is_some() && self.cutscene.as_ref().is_some_and(|(film, _)| film.over) {
                self.cutscene_over();
            }
            self.over |= self.project.is_none() && self.cutscene.as_ref().is_some_and(|(film, _)| film.over);
        } else if let Some(film) = self.place.as_mut().and_then(|place| place.film.as_mut()) {
            film.tick(keys, frames);
            for request in film.requests() {
                self.request(request);
            }
        }
        self.advance();
    }

    /// Played with nobody at the keys, so many frames at most, or to its end.
    pub fn play_alone(&mut self, most: usize) {
        self.nobody = true;
        self.skip_title();
        for _ in 0..most {
            if self.over || self.rested || self.ended {
                return;
            }
            self.tick(Keys::default(), 1.0);
        }
    }

    /// The keys of the title screen: a choice made, the game begins, from the game saved or anew.
    fn title_keys(&mut self, keys: &Keys) {
        let Some(title) = &mut self.title else {
            return;
        };
        let options = title.options();
        let Picked::Chosen(row) = title.cursor.keys(keys, options.len()) else {
            return;
        };
        let chosen = options[row];
        let Some(title) = self.title.take() else {
            return;
        };
        match (chosen, title.saved) {
            ("Quit", _) => self.over = true,
            ("Continue", Some(saved)) => self.load(&saved),
            _ if self.project.is_some() => self.new_game(),
            _ => {}
        }
    }

    /// The game from its start: its first chapter.
    pub fn new_game(&mut self) {
        let Some(first) = self.project.as_ref().and_then(GameFile::first).map(|chapter| chapter.number) else {
            self.over = true;
            return;
        };
        self.story = Story::default();
        self.make_team();
        self.chapter(first);
    }

    /// A new game from that chapter on (to try it from there): past the
    /// title screen, the team made, at the game's home, the chapter's start
    /// played. The flags the chapters before would have set are not.
    pub fn new_game_at(&mut self, number: u16) -> Result<(), Error> {
        let Some(project) = &self.project else {
            return Err(Error::NotNow("a film alone has no chapters"));
        };
        if project.chapter(number).is_none() {
            let known: Vec<String> = project.chapters.iter().map(|chapter| chapter.number.to_string()).collect();
            return Err(Error::NoChapter { game: project.name.clone(), number, known });
        }
        let home = project.home.clone();
        self.title = None;
        self.story = Story::default();
        self.make_team();
        if let Some((place, entrance)) = home {
            self.story.entrance = entrance.clone();
            self.enter(&place, Arrival::Entrance(entrance), false);
        }
        self.chapter(number);
        Ok(())
    }

    /// The team of the game's file, in its game.
    fn make_team(&mut self) {
        let team = self.project.as_ref().map(|project| project.team.clone()).unwrap_or_default();
        let tables = self.tables();
        let species = |name: &str| tables.monsters.get(name).and_then(|(number, _)| number.parse::<u16>().ok());
        let (Some(hero), partner) = (team.first().and_then(|name| species(name)), team.get(1).and_then(|name| species(name))) else {
            return;
        };
        if let Some(playthrough) = self.playthrough_mut() {
            playthrough.team(hero, partner.unwrap_or(hero));
        }
    }

    /// A game saved, taken up: its story, its team and bag, where it stood.
    pub fn load(&mut self, saved: &str) {
        self.story = Story::read(saved);
        if let Some(playthrough) = self.playthrough_mut() {
            playthrough.read(saved);
        }
        self.log.push(format!("loaded chapter {}", self.story.chapter));
        if let Some(place) = self.story.place.clone() {
            let arrival = match self.story.at {
                Some((x, y)) => Arrival::At(x, y),
                None => Arrival::Entrance(self.story.entrance.clone()),
            };
            self.enter(&place, arrival, false);
        }
    }

    /// Saved where the team stands now (not in a film, a dungeon, a screen): where it was written, or why not.
    pub fn save(&mut self) -> Result<PathBuf, Error> {
        let Some(project) = &self.project else {
            return Err(Error::NotNow("a film alone is saved by its own menu"));
        };
        if self.mode() != Mode::Roam && !matches!(self.screen, Some((GameScreen::Menu { .. }, _))) {
            return Err(Error::NotNow("a game is saved in a place, not in a film"));
        }
        if let Some(at) = self.place.as_ref().and_then(|place| place.film.as_ref()).and_then(Film::walked_at) {
            self.story.at = Some((at.x as i32, at.y as i32));
        }
        let place = self.story.place_saved();
        let text = match self.playthrough.as_ref().filter(|playthrough| playthrough.run().is_some()) {
            Some(playthrough) => playthrough.written_at(&place),
            None => self.story.written_alone(),
        };
        let file = story::save_file(&self.saves, &project.name);
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(&file, text).map_err(|why| Error::file(&file, why))?;
        self.log.push(format!("saved chapter {}", self.story.chapter));
        Ok(file)
    }

    /// The text of the save as it would be written now.
    pub fn saved_text(&self) -> String {
        let place = self.story.place_saved();
        match self.playthrough.as_ref().filter(|playthrough| playthrough.run().is_some()) {
            Some(playthrough) => playthrough.written_at(&place),
            None => self.story.written_alone(),
        }
    }

    /// The game played through it, made when it is not yet (none: it cannot
    /// be, no data).
    fn playthrough_mut(&mut self) -> Option<&mut Playthrough> {
        if self.playthrough.is_none() && !self.playthrough_failed {
            match Playthrough::new(&self.data, &self.folders) {
                Ok(playthrough) => self.playthrough = Some(playthrough),
                Err(_) => self.playthrough_failed = true,
            }
        }
        self.playthrough.as_mut()
    }

    // ----- what films ask -----

    /// A film's request, answered.
    fn request(&mut self, request: GameRequest) {
        if !matches!(request, GameRequest::Ask(GameAsk::Money(_) | GameAsk::Item { .. })) {
            self.settled = false;
        }
        let ask = match request {
            GameRequest::Checkpoint => return self.checkpoint(),
            GameRequest::Ask(ask) => ask,
        };
        match ask {
            // (The engine counts the money and items itself: the game, if there is one yet, too.)
            GameAsk::Money(amount) => {
                if let Some(playthrough) = self.playthrough.as_mut() {
                    playthrough.money(amount);
                    self.tell_film();
                }
            }
            GameAsk::Item { item, count, take } => {
                if let Some(playthrough) = self.playthrough.as_mut() {
                    playthrough.give_item(item, count, take);
                    self.tell_film();
                }
            }
            GameAsk::Go { place, entrance } => {
                self.story.entrance = entrance.clone();
                self.log.push(format!("go {place}"));
                self.enter(&place, Arrival::Entrance(entrance), true);
            }
            GameAsk::Zone(name) => self.rules_of(&When::Zone(name)),
            GameAsk::Talk(who) => {
                if let Some(place) = &mut self.place {
                    place.held = true;
                }
                self.talk(&who);
            }
            GameAsk::Menu => {
                if let Some(place) = &mut self.place {
                    place.held = true;
                }
                self.open_screen("menu", Vec::new());
            }
        }
    }

    /// The film playing told what its game has now (its bag, its money, its team).
    fn tell_film(&mut self) {
        let playthrough = self.playthrough.as_ref();
        if let Some((film, _)) = &mut self.cutscene {
            told(film, playthrough);
        } else if let Some(film) = self.place.as_mut().and_then(|place| place.film.as_mut()) {
            told(film, playthrough);
        }
    }

    /// A save asked for by a story's own scene (`checkpoint`): a game, where it stands.
    fn checkpoint(&mut self) {
        self.said = Some(match self.project.is_some() {
            true => match self.save() {
                Ok(file) => format!("game saved: {}", file.display()),
                Err(why) => why.to_string(),
            },
            false => "a film alone is not saved: a game is (a .game)".to_string(),
        });
    }

    // ----- dungeons -----

    /// Into a dungeon (or a boss fight) of one's own, by its word, gone
    /// through by the game's own team, lent; one that cannot be read is
    /// said, and the story goes on as if it failed.
    fn dive(&mut self, word: &str) {
        self.dived = Some(word.to_string());
        // (Nobody at the keys: passed, as cleared, to see what follows.)
        if self.nobody {
            self.dive_told(true, String::new());
            return;
        }
        // (The game is always the one that goes in: what the team becomes there is kept.)
        let (folder, folders) = (self.data.clone(), self.folders.clone());
        let guild_cut = self.project.as_ref().is_some_and(|project| game_file::guild_cuts(project, &self.story.flags));
        let Some(playthrough) = self.playthrough_mut() else {
            self.said = Some("no data: the dungeons cannot be gone through".to_string());
            self.dive_told(false, String::new());
            return;
        };
        let at = playthrough.dungeon_id(word);
        // (The guild keeps its cut of the jobs' rewards only if the game says, till the team graduates.)
        if let Some(run) = playthrough.run_mut() {
            run.set_chapter(if guild_cut { 0 } else { rules::jobs::GRADUATION });
        }
        let Some(run) = playthrough.lend() else {
            self.said = Some("the game's team is already in a dungeon".to_string());
            self.dive_told(false, String::new());
            return;
        };
        let audio = self.take_audio();
        match Diving::start_with(&folder, &folders, word, run, None, at) {
            Ok(mut diving) => {
                diving.frame.audio = audio;
                self.words_gone();
                self.diving = Some(diving);
            }
            Err((why, run)) => {
                self.give_audio(audio);
                if let Some(playthrough) = self.playthrough.as_mut() {
                    playthrough.put_back(*run);
                }
                self.said = Some(why);
                self.dive_told(false, String::new());
            }
        }
    }

    /// The box of the film playing emptied (a dungeon or a screen over it).
    fn words_gone(&mut self) {
        if let Some(film) = self.film_mut() {
            film.words_gone();
        }
    }

    /// The dungeon over: its game given back, what asked for it told.
    fn dive_over(&mut self) {
        let Some(mut diving) = self.diving.take() else {
            return;
        };
        let audio = diving.frame.audio.take();
        let cleared = diving.ended.unwrap_or(false);
        let left_said = diving.left_said.clone();
        // (The game back: what the team has become, its recruits, its bag, its money.)
        if let (Some(run), Some(playthrough)) = (diving.give_back(), self.playthrough.as_mut()) {
            playthrough.put_back(run);
        }
        self.give_audio(audio);
        self.tell_film();
        self.dive_told(cleared, left_said);
        if let Some(film) = self.film_mut() {
            film.music_again();
        }
    }

    /// How the dungeon ended, told to the story.
    fn dive_told(&mut self, cleared: bool, left_said: String) {
        let told = match (cleared, left_said.trim()) {
            (false, "") => "The team fainted: it goes home.".to_string(),
            (_, said) => said.to_string(),
        };
        self.settled = false;
        self.story.won = cleared;
        self.log.push(format!("dungeon {}", if cleared { "won" } else { "lost" }));
        if !cleared {
            self.home(told);
        } else if !told.is_empty() {
            self.said = Some(told);
        }
        // (Its `on won <it>` / `on lost <it>`, played before what follows the `dungeon` that went in, if one did.)
        if let Some(word) = self.dived.take() {
            self.rules_of(&if cleared { When::Won(word) } else { When::Lost(word) });
        }
    }

    /// K.O.: back to the game's home (else the place it was in), what was lost said.
    fn home(&mut self, told: String) {
        let home = self
            .project
            .as_ref()
            .and_then(|project| project.home.clone())
            .or_else(|| self.story.place.clone().map(|place| (place, self.story.entrance.clone())));
        if let Some((place, entrance)) = home {
            self.story.entrance = entrance.clone();
            self.enter(&place, Arrival::Entrance(entrance), false);
            if let Some(place) = &mut self.place {
                place.told.push(told);
            }
        }
    }

    // ----- screens -----

    /// One of the game's screens (`shop`, `map …`), over the scene; nobody
    /// at the keys, or no game: passed at once.
    fn open_screen(&mut self, word: &str, ways: Vec<String>) {
        let mut draw = self.draw;
        let jobs_in = self.job_dungeons();
        let opened = match self.nobody {
            true => None,
            false => self.playthrough_mut().and_then(|playthrough| GameScreen::open(word, ways, playthrough, &mut draw, jobs_in.as_ref())),
        };
        self.draw = draw;
        match opened {
            Some(screen) => {
                self.words_gone();
                self.screen = Some((screen, Words::default()));
            }
            None => self.screen_left(0),
        }
    }

    /// The dungeons the boards offer jobs in (`game_file::job_dungeons`).
    fn job_dungeons(&self) -> Option<BTreeSet<String>> {
        game_file::job_dungeons(self.project.as_ref()?, &self.story.opened, |word| self.folders.dungeon(word).is_file())
    }

    /// The world map (`map`, `map dungeons`): the ways opened (the dungeons
    /// only, when it says so), by their names; nobody at the keys, the one
    /// the story waits for gone to at once (`nobody_maps`).
    fn open_map(&mut self, dungeons_only: bool) {
        let ways: Vec<String> = self.story.opened.iter().filter(|word| !dungeons_only || self.is_dive(word)).cloned().collect();
        if self.nobody {
            let chosen = self.nobody_maps(&ways);
            self.map_ways = ways;
            return self.screen_left(chosen);
        }
        let names = ways.iter().map(|word| self.way_name(word)).collect();
        self.map_ways = ways;
        self.open_screen("map", names);
    }

    /// Whether that word is a dungeon or a fight of the game's (else a place).
    fn is_dive(&self, word: &str) -> bool {
        self.folders.dungeon(word).is_file() || self.folders.fight(word).is_file()
    }

    /// A way of the map as it is shown: the dungeon's `name:`, the place's
    /// `name`; else its word.
    fn way_name(&self, word: &str) -> String {
        let named = match self.is_dive(word) {
            true => [self.folders.dungeon(word), self.folders.fight(word)].iter().find_map(|file| {
                let text = std::fs::read_to_string(file).ok()?;
                text.lines().find_map(|line| line.trim().strip_prefix("name:").map(|name| name.trim().to_string()))
            }),
            false => roam::decor_of(&self.folders.decor(word)).ok().map(|decor| decor.name),
        };
        named.filter(|name| !name.is_empty()).unwrap_or_else(|| word.to_string())
    }

    /// Nobody at the keys, on the map: the way whose `on won` moves the
    /// story on (a chapter, the end), else one whose `on won` plays, else
    /// the first (from 1; 0: none).
    fn nobody_maps(&self, ways: &[String]) -> usize {
        let rules = self.rules_now();
        let story = &self.story;
        let holds = |rule: &Rule| rule.test.as_ref().is_none_or(|test| game_file::value(test, &|name| story.value(name)) != 0.0);
        let won = |way: &String| rules.iter().filter(|rule| rule.when == When::Won(way.clone()) && holds(rule)).collect::<Vec<_>>();
        let moves = ways.iter().position(|way| won(way).iter().any(|rule| moves_on(&rule.actions)));
        let plays = ways.iter().position(|way| !won(way).is_empty());
        moves.or(plays).map_or(usize::from(!ways.is_empty()), |at| at + 1)
    }

    /// The screen left (the way chosen on a map, from 1): the story goes on.
    fn screen_left(&mut self, chosen: usize) {
        (self.screen, self.settled) = (None, false);
        self.tell_film();
        let way = chosen.checked_sub(1).and_then(|at| self.map_ways.get(at).cloned());
        match way {
            Some(word) if self.is_dive(&word) => {
                self.log.push(format!("map: {word}"));
                self.push(vec![Action::Dungeon(word)]);
            }
            Some(word) => {
                self.log.push(format!("map: {word}"));
                self.push(vec![Action::Go(word, None)]);
            }
            None => {}
        }
    }

    // ----- the story's actions -----

    /// These actions, to play next (before what was playing).
    fn push(&mut self, actions: Vec<Action>) {
        if !actions.is_empty() {
            self.running.push(Running { actions: Arc::new(actions), next: 0 });
        }
    }

    /// Whether the game waits for something to end (a film, a dungeon, a screen).
    fn waiting(&self) -> bool {
        self.title.is_some() || self.diving.is_some() || self.screen.is_some() || self.cutscene.is_some()
    }

    /// The actions played, up to one that waits; nothing left, the place goes on.
    fn advance(&mut self) {
        if self.project.is_none() || self.over {
            return;
        }
        while !self.waiting() && !self.over {
            let Some(top) = self.running.last_mut() else {
                break;
            };
            let Some(action) = top.actions.get(top.next).cloned() else {
                self.running.pop();
                continue;
            };
            top.next += 1;
            self.act(action);
        }
        if self.waiting() || self.over {
            return;
        }
        // (Nothing left to play: the place, as the story has it now.)
        match &self.place {
            Some(_) if !std::mem::replace(&mut self.settled, true) => self.place_goes_on(),
            Some(_) => {}
            None => self.finished(),
        }
    }

    /// The game ends (it cannot go on).
    fn the_end(&mut self) {
        if !self.over {
            self.log.push("the end".to_string());
        }
        self.over = true;
    }

    /// The game finished, as its story says: its end card, someone at the
    /// keys; else over, as when it cannot go on.
    fn finished(&mut self) {
        if self.nobody || self.project.is_none() {
            return self.the_end();
        }
        if !self.ended {
            self.log.push("the end".to_string());
        }
        (self.ended, self.running) = (true, Vec::new());
        self.words_gone();
    }

    /// From the end card back to the title screen: what films play with
    /// taken back, the story and the game as before a new one.
    fn back_to_title(&mut self) {
        if let Some((film, _)) = self.cutscene.take() {
            let (art, audio) = film.parts();
            (self.art, self.audio) = (Some(art), audio.or(self.audio.take()));
        }
        if let Some(film) = self.place.take().and_then(|place| place.film) {
            let (art, audio) = film.parts();
            (self.art, self.audio) = (Some(art), audio.or(self.audio.take()));
        }
        let Some(project) = &self.project else {
            self.over = true;
            return;
        };
        let saved = std::fs::read_to_string(story::save_file(&self.saves, &project.name)).ok();
        let name = if project.title.is_empty() { project.name.clone() } else { project.title.clone() };
        self.title = Some(Title { name, saved, cursor: ListCursor::default() });
        self.story = Story::default();
        (self.playthrough, self.playthrough_failed) = (None, false);
        (self.ended, self.settled, self.rested) = (false, false, false);
        self.running.clear();
        self.talked.clear();
        (self.walked, self.seen, self.zoned) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
        (self.dived, self.map_ways) = (None, Vec::new());
        self.log.push("title".to_string());
    }

    /// One action played.
    fn act(&mut self, action: Action) {
        self.settled = false;
        match action {
            Action::Play(word) => self.play_film(&word),
            Action::Set(flag) => {
                self.story.flags.insert(flag);
            }
            Action::Clear(flag) => {
                self.story.flags.remove(&flag);
            }
            Action::Add(counter, count) => *self.story.counters.entry(counter).or_insert(0) += count,
            Action::Chapter(number) => {
                self.running.clear();
                self.chapter(number);
            }
            Action::Go(place, entrance) => {
                self.story.entrance = entrance.clone();
                self.log.push(format!("go {place}"));
                self.enter(&place, Arrival::Entrance(entrance), true);
            }
            Action::Dungeon(word) | Action::Fight(word) => {
                self.log.push(format!("dungeon {word}"));
                self.dive(&word);
            }
            Action::Open(word) => {
                self.story.opened.insert(word);
            }
            Action::Give(count, name) => {
                if let Some(playthrough) = self.playthrough_mut() {
                    playthrough.give(&name, count);
                }
            }
            Action::Money(amount) => {
                if let Some(playthrough) = self.playthrough_mut() {
                    playthrough.money(amount);
                }
            }
            Action::Recruit(name) => {
                let species = self.tables().monsters.get(name.as_str()).and_then(|(number, _)| number.parse::<u16>().ok());
                match (species, self.playthrough_mut()) {
                    (Some(species), Some(playthrough)) => {
                        playthrough.recruit(species);
                        self.log.push(format!("recruit {name}"));
                    }
                    (None, _) => self.said = Some(format!("recruit {name}: no such Pokémon")),
                    (_, None) => self.said = Some(format!("recruit {name}: no data, no team to join")),
                }
            }
            Action::Shop(word) => self.open_screen(&word, Vec::new()),
            Action::Map { dungeons_only } => self.open_map(dungeons_only),
            Action::Finish => self.finished(),
            Action::If(branches) => {
                let story = &self.story;
                let chosen = branches.into_iter().find(|(test, _)| test.as_ref().is_none_or(|test| game_file::value(test, &|name| story.value(name)) != 0.0));
                if let Some((_, actions)) = chosen {
                    self.push(actions);
                }
            }
        }
    }

    /// On to that chapter: what it does as it starts.
    fn chapter(&mut self, number: u16) {
        self.story.chapter = number;
        self.log.push(format!("chapter {number}"));
        let actions = self.project.as_ref().and_then(|project| project.chapter(number)).map(|chapter| chapter.actions.clone()).unwrap_or_default();
        self.push(actions);
    }

    /// The names a film is given and gives back: the game's own, and those it sets.
    fn names(&self) -> (BTreeSet<String>, BTreeSet<String>) {
        let mut names = self.project.as_ref().map(GameFile::names).unwrap_or_default();
        names.extend(self.story.flags.iter().cloned());
        names.extend(self.story.counters.keys().cloned());
        let mut counters: BTreeSet<String> = self.story.counters.keys().cloned().collect();
        if let Some(project) = &self.project {
            for chapter in &project.chapters {
                counters_of(&chapter.actions, &mut counters);
            }
        }
        (names, counters)
    }

    /// A film of the game's (its `films/<word>.film`), played now.
    fn play_film(&mut self, word: &str) {
        self.log.push(format!("film {word}"));
        if self.project.is_none() {
            return;
        }
        let path = self.folders.film(word);
        let Some(art) = self.take_art() else {
            self.said = Some("no data: the films cannot play".to_string());
            return;
        };
        let audio = self.take_audio();
        match Film::open(&path.to_string_lossy(), art, audio, None, self.tables()) {
            Ok(mut film) => {
                film.hosted = true;
                film.curtain = false;
                (film.auto, film.debug) = (self.auto || self.nobody, self.debug.clone());
                if self.nobody {
                    film.nobody_plays();
                }
                let (names, _) = self.names();
                let director = film.director_mut();
                for (name, number) in self.story.numbers(&names) {
                    director.set_number(name, number);
                }
                told(&mut film, self.playthrough.as_ref());
                self.cutscene = Some((film, word.to_string()));
            }
            Err(why) => {
                self.said = Some(why);
                self.art = Art::open_in(&self.data, self.folders.clone()).ok();
            }
        }
    }

    /// The film over: what it set kept, what it played with back, the game goes on.
    fn cutscene_over(&mut self) {
        let Some((film, word)) = self.cutscene.take() else {
            return;
        };
        self.settled = false;
        let (mut names, mut counters) = self.names();
        names.extend(film.path.as_deref().map(films_flags).unwrap_or_default());
        // (How many times each moment and scene of it was played: counters the story keeps, `seen(name)`.)
        let seen = film.path.as_deref().map(|path| ::film::plot::counters(&std::fs::read_to_string(path).unwrap_or_default())).unwrap_or_default();
        names.extend(seen.iter().cloned());
        counters.extend(seen);
        let numbers = film.director().numbers().clone();
        self.story.take_back(&numbers, &names, &counters);
        self.log.push(format!("film {word} ended"));
        let (art, audio) = film.parts();
        self.art = Some(art);
        self.audio = audio;
        if let Some(film) = self.film_mut() {
            film.music_again();
        }
    }

    /// What films play with: the game's, else the place's scene's (put away meanwhile).
    fn take_art(&mut self) -> Option<Art> {
        if let Some(art) = self.art.take() {
            return Some(art);
        }
        let place = self.place.as_mut()?;
        let film = place.film.take()?;
        place.at = film.walked_at().map(|at| (at.x as i32, at.y as i32));
        let (art, audio) = film.parts();
        if audio.is_some() {
            self.audio = audio;
        }
        Some(art)
    }

    /// What films are heard by, taken from whichever has it.
    fn take_audio(&mut self) -> Option<Box<dyn Hears>> {
        if let Some(audio) = self.audio.take() {
            return Some(audio);
        }
        self.film_mut().and_then(|film| film.audio.take())
    }

    /// Given back, to the film playing or kept.
    fn give_audio(&mut self, audio: Option<Box<dyn Hears>>) {
        match self.film_mut() {
            Some(film) => film.audio = audio,
            None => self.audio = audio,
        }
    }

    // ----- places -----

    /// Into a place, by that way: its scene made; `new`: come in (its
    /// `on enter` plays), else back where it was.
    fn enter(&mut self, word: &str, arrival: Arrival, new: bool) {
        self.settled = false;
        self.story.place = Some(word.to_string());
        self.story.at = match arrival {
            Arrival::At(x, y) => Some((x, y)),
            Arrival::Entrance(_) => None,
        };
        // (The scene the place had, put away: what it plays with back.)
        if let Some(film) = self.place.take().and_then(|place| place.film) {
            let (art, audio) = film.parts();
            self.art = Some(art);
            if audio.is_some() {
                self.audio = audio;
            }
        }
        let arrival_made = arrival.clone();
        self.place = Some(Place {
            word: word.to_string(),
            film: None,
            text: String::new(),
            held: false,
            at: None,
            told: Vec::new(),
            arrival: arrival_made,
            back: false,
        });
        if new {
            self.rules_of(&When::Enter);
        }
    }

    /// The place's scene as the story has it now: made again if it changed
    /// (or was put away), held talks let go; nobody at the keys, its talks
    /// played once each, then the game rests.
    fn place_goes_on(&mut self) {
        let Some(place) = &self.place else {
            return;
        };
        let word = place.word.clone();
        // (Its scene as it was made, made again as the story has it now: the same, it goes on.)
        if place.film.is_some() {
            let (arrival, back) = (place.arrival.clone(), place.back);
            if self.place_text(&word, &arrival, back).is_ok_and(|again| self.place.as_ref().is_some_and(|place| place.text == again)) {
                let place = self.place.as_mut().expect("a place");
                if std::mem::take(&mut place.held)
                    && let Some(film) = &mut place.film
                {
                    film.release();
                }
                if self.nobody {
                    self.nobody_talks();
                }
                return;
            }
        }
        let place = self.place.as_ref().expect("a place");
        let arrival = match (place.film.as_ref().and_then(Film::walked_at), place.at) {
            (Some(at), _) => Arrival::At(at.x as i32, at.y as i32),
            (None, Some((x, y))) => Arrival::At(x, y),
            (None, None) => Arrival::Entrance(self.story.entrance.clone()),
        };
        let back = place.film.is_some() || place.at.is_some();
        if let Err(why) = self.built_on_demand(&word) {
            self.said = Some(why);
            self.the_end();
            return;
        }
        let text = match self.place_text(&word, &arrival, back) {
            Ok(text) => text,
            Err(why) => {
                self.said = Some(why);
                self.the_end();
                return;
            }
        };
        if let Some(place) = &mut self.place {
            (place.arrival, place.back) = (arrival, back);
        }
        self.make_scene(text);
        if self.nobody {
            self.nobody_talks();
        }
    }

    /// What the places' scenes are compiled with.
    fn tables(&self) -> Arc<film::tables::Tables> {
        self.context.tables_for(&self.folders)
    }

    /// A place whose decor has no level built yet, built now (into the
    /// data's `built/` layer, `roam::build_decor`), once, and said in the
    /// log; the tables read again to know it.
    fn built_on_demand(&mut self, word: &str) -> Result<(), String> {
        if self.tables().decors.contains_key(word) || !self.built.insert(word.to_string()) {
            return Ok(());
        }
        if self.project.is_none() {
            return Ok(());
        }
        let file = self.folders.decor(word);
        let cache = assets::data::built_folder(&self.data);
        let level = roam::build_decor(&self.folders, &self.data, &file, &cache)?;
        self.log.push(format!("built {word} (level {level}): its decor was not built yet"));
        self.context.reload_for(&self.folders);
        Ok(())
    }

    /// The text of the place's scene, as the story has it now.
    fn place_text(&self, word: &str, arrival: &Arrival, back: bool) -> Result<String, String> {
        let project = self.project.as_ref().ok_or("no game")?;
        let decor = roam::decor_of(&self.folders.decor(word))?;
        let story = &self.story;
        let talking = |who: &str| self.talks_to_the_game(who);
        let holds =
            |test: Option<&str>| test.is_none_or(|test| game_file::test_of(test).is_ok_and(|test| game_file::value(&test, &|name| story.value(name)) != 0.0));
        let told = self.place.as_ref().map(|place| place.told.clone()).unwrap_or_default();
        let making = roam::Making { team: &project.team, arrival: arrival.clone(), talking: &talking, holds: &holds, told: &told, cut: back };
        Ok(roam::scene_text(word, &decor, &making))
    }

    /// Whether talking to that one tells the game (a rule, a service, a film).
    fn talks_to_the_game(&self, who: &str) -> bool {
        let talk = When::Talk(who.to_string());
        self.rules_now().iter().any(|rule| rule.when == talk) || self.resident(who).is_some_and(|resident| resident.talk.is_some())
    }

    /// The place's decor, as its file says it now.
    fn place_decor(&self) -> Option<decor::lang::Decor> {
        self.project.as_ref()?;
        roam::decor_of(&self.folders.decor(&self.place.as_ref()?.word)).ok()
    }

    /// Whether a test of the place's holds now (none: it does).
    fn holds(&self, test: Option<&str>) -> bool {
        test.is_none_or(|test| game_file::test_of(test).is_ok_and(|test| game_file::value(&test, &|name| self.story.value(name)) != 0.0))
    }

    /// One who lives in the place and is there now.
    fn resident(&self, who: &str) -> Option<decor::lang::Resident> {
        self.place_decor()?.residents.into_iter().find(|resident| resident.name == who && self.holds(resident.condition.as_deref()))
    }

    /// The rules that may play now: the place's (its `on … end`), then the chapter's.
    fn rules_now(&self) -> Vec<Rule> {
        let mut found = Vec::new();
        if let Some(decor) = self.place_decor() {
            for rule in &decor.rules {
                let mut problems = Vec::new();
                let (_, read) = game_file::read_actions(&format!("{}\n{}end\n", rule.head, rule.body), &mut problems);
                found.extend(read);
            }
        }
        let chapter = self.project.as_ref().and_then(|project| project.chapter(self.story.chapter));
        found.extend(chapter.map(|chapter| chapter.rules.clone()).unwrap_or_default());
        found
    }

    /// The rules of that moment whose test holds, played (the place's first, then the chapter's).
    fn rules_of(&mut self, when: &When) {
        let story = &self.story;
        let holds = |rule: &Rule| rule.test.as_ref().is_none_or(|test| game_file::value(test, &|name| story.value(name)) != 0.0);
        let chosen: Vec<Rule> = self.rules_now().into_iter().filter(|rule| rule.when == *when && holds(rule)).collect();
        for rule in chosen.into_iter().rev() {
            self.push(rule.actions);
        }
    }

    /// The player talked to that one: what it does (a film, a screen), after the rules of that talk.
    fn talk(&mut self, who: &str) {
        self.log.push(format!("talk {who}"));
        let talk = self.resident(who).and_then(|resident| resident.talk);
        match talk.as_deref().map(|talk| talk.split_once(' ').unwrap_or((talk, ""))) {
            Some(("film", word)) => self.push(vec![Action::Play(word.to_string())]),
            Some((screen, _)) => self.push(vec![Action::Shop(screen.to_string())]),
            None => {}
        }
        self.rules_of(&When::Talk(who.to_string()));
    }

    /// Nobody at the keys: each one of the place who tells the game when
    /// talked to, talked to once in this chapter; none left, each zone with
    /// an `on zone` walked into, once (a map there: `nobody_maps`); then a way out of
    /// the place walked, once in this chapter (to a place not seen yet
    /// first); none left either, the game rests.
    fn nobody_talks(&mut self) {
        let Some(word) = self.place.as_ref().map(|place| place.word.clone()) else {
            return;
        };
        self.seen.insert((self.story.chapter, word.clone()));
        let here: Vec<String> = self.place_decor().map_or(Vec::new(), |decor| {
            decor.residents.iter().filter(|resident| self.holds(resident.condition.as_deref())).map(|resident| resident.name.clone()).collect()
        });
        let next = here.into_iter().find(|who| self.talks_to_the_game(who) && !self.talked.contains(&(self.story.chapter, word.clone(), who.clone())));
        let zone = if next.is_none() { self.zone_not_walked(&word) } else { None };
        match (next, zone) {
            (Some(who), _) => {
                self.talked.insert((self.story.chapter, word, who.clone()));
                self.talk(&who);
                // (A talk whose rules do not hold now plays nothing: the place goes on all the same.)
                self.settled = false;
                self.advance();
            }
            (None, Some(zone)) => {
                self.zoned.insert((self.story.chapter, word, zone.clone()));
                self.log.push(format!("zone {zone}"));
                self.request(GameRequest::Ask(GameAsk::Zone(zone)));
                self.advance();
            }
            (None, None) => match self.way_not_walked(&word) {
                Some((at, place, entrance)) => {
                    self.walked.insert((self.story.chapter, word, at));
                    // (Played on the next frame: the story goes on from the place's scene.)
                    self.request(GameRequest::Ask(GameAsk::Go { place, entrance }));
                }
                None => {
                    self.rested = true;
                    self.log.push(format!("rests in {word}"));
                }
            },
        }
    }

    /// A zone of that place with an `on zone` that plays now, not walked
    /// into yet in this chapter.
    fn zone_not_walked(&self, word: &str) -> Option<String> {
        let chapter = self.story.chapter;
        let zones = self.place_decor().map(|decor| decor.zones).unwrap_or_default();
        let rules = self.rules_now();
        let story = &self.story;
        let plays = |zone: &str| {
            let when = When::Zone(zone.to_string());
            rules.iter().any(|rule| rule.when == when && rule.test.as_ref().is_none_or(|test| game_file::value(test, &|name| story.value(name)) != 0.0))
        };
        zones.into_iter().map(|zone| zone.name).find(|zone| !self.zoned.contains(&(chapter, word.to_string(), zone.clone())) && plays(zone))
    }

    /// A way out of that place not walked yet in this chapter, to a place of
    /// the game's (one not seen yet in it first): which, where to, by which entrance.
    fn way_not_walked(&self, word: &str) -> Option<(usize, String, Option<String>)> {
        let chapter = self.story.chapter;
        let ways = self.place_decor().map(|decor| decor.ways_out).unwrap_or_default();
        let open: Vec<(usize, &decor::lang::WayOut)> = ways
            .iter()
            .enumerate()
            .filter(|(at, way)| !self.walked.contains(&(chapter, word.to_string(), *at)) && self.folders.decor(&way.to).is_file())
            .collect();
        let new = open.iter().find(|(_, way)| !self.seen.contains(&(chapter, way.to.clone())));
        new.or(open.first()).map(|(at, way)| (*at, way.to.clone(), way.entrance.clone()))
    }

    /// The place's scene made of that text, played.
    fn make_scene(&mut self, text: String) {
        let Some(art) = self.take_art() else {
            self.said = Some("no data: the places cannot be shown".to_string());
            self.the_end();
            return;
        };
        let written = film::compile_place_with(&text, self.tables());
        if let Some(problem) = written.problems.iter().find(|problem| !problem.is_remark()) {
            self.said = Some(format!("the scene of {}: {problem}\n{text}", self.story.place.clone().unwrap_or_default()));
            self.art = Some(art);
            self.the_end();
            return;
        }
        match Film::of_written(&written, art) {
            Ok(mut film) => {
                film.hosted = true;
                film.curtain = false;
                (film.auto, film.debug) = (self.auto, self.debug.clone());
                film.audio = self.audio.take();
                if self.nobody {
                    film.nobody_plays();
                }
                let (names, _) = self.names();
                for (name, number) in self.story.numbers(&names) {
                    film.director_mut().set_number(name, number);
                }
                told(&mut film, self.playthrough.as_ref());
                if let Some(place) = &mut self.place {
                    (place.film, place.text, place.held, place.at) = (Some(film), text, false, None);
                    place.told.clear();
                }
            }
            Err(failed) => {
                let (art, why) = *failed;
                self.art = Some(art);
                self.said = Some(why);
                self.the_end();
            }
        }
    }

    // ----- drawn -----

    /// The lower screen as it is now.
    pub fn draw(&mut self, canvas: &mut Canvas, world: &mut Screen) {
        if self.ended {
            let ui = self.art.as_ref().map(|art| &art.ui).or_else(|| self.film().map(|film| &film.art.ui));
            if let (Some(ui), Some(project)) = (ui, &self.project) {
                let name = if project.title.is_empty() { project.name.as_str() } else { project.title.as_str() };
                end_card(name, canvas, ui, world);
            }
            return;
        }
        if let Some(title) = &self.title {
            let ui = self.art.as_ref().map(|art| &art.ui).or_else(|| self.film().map(|film| &film.art.ui));
            if let Some(ui) = ui {
                title.draw(canvas, ui, world);
            }
            return;
        }
        if let Some(diving) = &self.diving {
            diving.draw(world);
            if self.debug.shown
                && let Some(film) = self.film()
            {
                preview::debug::draw_dive(diving.floor(), &self.debug, &film.art.ui.font, world);
            }
            return;
        }
        let (screen, playthrough) = (self.screen.as_ref(), self.playthrough.as_ref());
        let film = match &mut self.cutscene {
            Some((film, _)) => Some(film),
            None => self.place.as_mut().and_then(|place| place.film.as_mut()),
        };
        let Some(film) = film else {
            canvas.clear();
            world.rgba.fill(0);
            return;
        };
        match (screen, playthrough) {
            (Some((screen, shown)), Some(playthrough)) => {
                let mut boxed = |canvas: &mut Canvas, ui: &stage_view::ui::UiArt| screen.draw(canvas, ui, shown, playthrough);
                film.draw_with(canvas, world, Some(&mut boxed));
            }
            _ => film.draw(canvas, world),
        }
    }
}

/// A game's end card: “The End”, its title, the key that goes back.
fn end_card(name: &str, canvas: &mut Canvas, ui: &stage_view::ui::UiArt, world: &mut Screen) {
    let back = "Enter: back to the title screen";
    card(canvas, ui, 8, &[("The End", color::YELLOW), (name, color::WHITE), ("", color::WHITE), (back, color::WHITE)]);
    world.rgba.fill(0);
    world.over(&canvas.data);
}

/// Whether these actions move the story on: a chapter, the end.
fn moves_on(actions: &[Action]) -> bool {
    actions.iter().any(|action| match action {
        Action::Chapter(_) | Action::Finish => true,
        Action::If(branches) => branches.iter().any(|(_, inner)| moves_on(inner)),
        _ => false,
    })
}

/// The counters actions add to.
fn counters_of(actions: &[Action], counters: &mut BTreeSet<String>) {
    for action in actions {
        match action {
            Action::Add(name, _) => {
                counters.insert(name.clone());
            }
            Action::If(branches) => branches.iter().for_each(|(_, actions)| counters_of(actions, counters)),
            _ => {}
        }
    }
}

/// A film's numbers and words as its game has them.
fn told(film: &mut Film, playthrough: Option<&Playthrough>) {
    let Some(playthrough) = playthrough else {
        return;
    };
    let director = film.director_mut();
    let (mut numbers, mut texts) = (director.numbers().clone(), director.texts().clone());
    playthrough.into_film(&mut numbers, &mut texts);
    director.set_numbers(numbers);
    director.set_texts(texts);
}

/// A seed for what the game draws at random (the boards, the shops' stock).
fn seed() -> u32 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |time| time.subsec_nanos()) | 1
}

/// The flags a film sets or reads as the game's (`set x`, `clear x`, `keep x = …`), by its file.
fn films_flags(path: &str) -> BTreeSet<String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            match (words.next(), words.next()) {
                (Some("set" | "clear" | "keep"), Some(name)) => Some(name.trim_end_matches('=').to_string()),
                _ => None,
            }
        })
        .collect()
}
