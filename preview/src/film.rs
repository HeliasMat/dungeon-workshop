//! A scene file played: the engine's director and stage, what shows who
//! stands there, the words on screen, and the whole drawn on a canvas; kept
//! whole at a moment (`Snapshot`), to go back to it.
//!
//! What only the game does (its team and bag, a save, a place's ways and
//! talks) a film asks of it by a `GameRequest`; played alone (no game hosts
//! it), each is answered at once, as if it went well.

use engine::{Beat, Call, Director, GameAsk, Hold, ScreenBeat, StoryEvent, Viewport};
use rules::run::options::Options;
use scenes::vocabulary::POKEMON;
use shell::sound::Hears;
use shell::{Keys, Screen};
use stage_view::stage::{Clear, Means, Play, Sim, scene_keys};
use stage_view::ui::message::Words;
use stage_view::ui::{Canvas, SCREEN_HEIGHT, SCREEN_WIDTH, UiArt};

use crate::art::Art;

/// What a film asks of the game that plays it: what the engine asked, typed
/// as the beat was read (`GameAsk`: a way out, a zone, a talk, the menu,
/// money, items), or a save (`checkpoint`).
#[derive(Clone, Debug, PartialEq)]
pub enum GameRequest {
    Ask(GameAsk),
    /// The game is to be saved here.
    Checkpoint,
}

impl GameRequest {
    /// Whether the film holds until the game is done with it.
    pub fn holds(&self) -> bool {
        matches!(self, GameRequest::Ask(ask) if ask.holds())
    }
}

/// A film at a moment, whole: the scene as it goes, the box and what it
/// shows, whether it is over. (The pictures read for it are kept by the film
/// it goes back into: they are the same at any moment.)
#[derive(Clone)]
pub struct Snapshot {
    sim: Sim,
    words: Option<Words>,
    over: bool,
}

impl Snapshot {
    /// How far it had gone, in frames.
    pub fn played(&self) -> f32 {
        self.sim.clock
    }
}

pub struct Film {
    /// Lines are read by themselves.
    pub auto: bool,
    pub art: Art,
    pub audio: Option<Box<dyn Hears>>,
    play: Play,
    /// The message box, as the game's.
    words: Option<Words>,
    pub over: bool,
    /// The file it was read from.
    pub path: Option<String>,
    /// A game plays it (`game::Game`): what it asks is kept for the
    /// game to take (`requests`); else answered at once.
    pub hosted: bool,
    requests: Vec<GameRequest>,
    /// Its end shows “The End” (a film alone, or the last of a game).
    pub curtain: bool,
    /// Whoever writes it: what is drawn over the scene, ways to go faster
    /// (`DUNGEON_FILM_DEBUG`, F6 to F9 in the window; see `crate::debug`).
    pub debug: crate::debug::Debug,
}

/// The places the events go to, in their order: each scene's, and each
/// it shows as it plays (`backdrop`, `crossfade`), to be read ahead.
fn places_of(events: &[StoryEvent]) -> Vec<u16> {
    let mut places = Vec::new();
    for scene in events.iter().flat_map(|event| &event.scenes) {
        // (A scene no line opened, what an event shows outside its own: no place.)
        if !scene.name.is_empty() || scene.level != 0 {
            places.push(scene.level);
        }
        places.extend(scene.beats.iter().filter_map(|beat| match beat {
            Beat::Screen(ScreenBeat::Backdrop(level, ..) | ScreenBeat::Crossfade { level, .. }) => Some(*level),
            _ => None,
        }));
    }
    places.dedup();
    places
}

/// An event of the game's story by its number, written
/// `read:84`.
pub fn story_event(name: &str) -> Option<usize> {
    name.strip_prefix("read:")?.parse().ok()
}

impl Film {
    /// A scene file, or an event of the game's story (`read:84`: played from
    /// its `story.txt` as the game reads it, alone), ready to play.
    /// (`from`: a label; what comes before it is played at once.) A film is
    /// compiled with those tables (its project's).
    pub fn open(
        path: &str,
        art: Art,
        audio: Option<Box<dyn Hears>>,
        from: Option<String>,
        tables: std::sync::Arc<::film::tables::Tables>,
    ) -> Result<Self, String> {
        let event = story_event(path);
        let file = match event {
            Some(_) => shell::data::path(&art.folder, "story.txt"),
            None => path.to_string(),
        };
        let (mut events, problems) = ::film::read_any_with(std::path::Path::new(&file), &POKEMON, tables)?;
        if let Some(number) = event {
            events.retain(|told| told.number == number);
        }
        if let Some(problem) = problems.iter().find(|problem| !problem.is_remark()).filter(|_| event.is_none()) {
            return Err(format!("{path} : {problem}"));
        }
        let mut film = Self::of_events(events, art, audio, from).map_err(|failed| format!("{path} : {}", failed.1))?;
        film.path = event.is_none().then(|| path.to_string());
        Ok(film)
    }

    /// The story's director, as it is now (its numbers, its words, its place).
    pub fn director(&self) -> &Director {
        &self.play.sim.director
    }

    /// The story's director, to give it numbers and words, or a place to go to.
    pub fn director_mut(&mut self) -> &mut Director {
        &mut self.play.sim.director
    }

    /// A number of the story, as it is now.
    pub fn number(&self, name: &str) -> Option<f64> {
        self.play.sim.director.numbers_and_words().0.get(name).copied()
    }

    /// Where the one the player walks stands.
    pub fn walked_at(&self) -> Option<glam::Vec2> {
        let stage = &self.play.sim.stage;
        stage.play.controlled.and_then(|who| stage.cast.get(&who)).map(|actor| actor.position)
    }

    /// What it asked of the game since last asked.
    pub fn requests(&mut self) -> Vec<GameRequest> {
        std::mem::take(&mut self.requests)
    }

    /// What the game was asked is done: the film, held for it, goes on.
    pub fn release(&mut self) {
        self.play.sim.director.menu_done(false);
    }

    /// A film from its words compiled (`film::compile`), ready to play; else
    /// why not, and what it was to play with, given back. (What is wrong in
    /// the words, `check::read_through` tells.)
    pub fn of_written(written: &::film::Written, art: Art) -> Result<Self, Box<(Art, String)>> {
        Self::of_events(written.events.clone(), art, None, None)
    }

    /// The engine's events of a film, ready to play; else why not, and what
    /// it was to play with.
    fn of_events(events: Vec<StoryEvent>, art: Art, audio: Option<Box<dyn Hears>>, from: Option<String>) -> Result<Self, Box<(Art, String)>> {
        let places = places_of(&events);
        if events.is_empty() {
            return Err(Box::new((art, "no event in this file".to_string())));
        }
        let mut director = Director::new(events, 0, &POKEMON);
        director.author.skip_to = from;
        // (How long the game's sounds last, for `wait sound`.)
        director.hear_sounds(stage_view::stage::sound_lengths(&art, &art.tables));
        let mut play = Play::new(director, art.cast.clone());
        play.read_ahead(&art.folder, places);
        Ok(Film {
            auto: false,
            art,
            audio,
            play,
            words: None,
            over: false,
            path: None,
            hosted: false,
            requests: Vec::new(),
            curtain: true,
            debug: crate::debug::Debug::default(),
        })
    }

    /// Its music as it plays now, heard again (after it was gone to a
    /// moment without a sound).
    pub fn music_again(&mut self) {
        let mut means = full(&self.art, &mut self.audio);
        self.play.song_again(&mut means);
    }

    /// Nothing heard of it any more: its music and its sounds stopped.
    pub fn hush(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.song(0, None, 0.0, 0.0);
            audio.song(1, None, 0.0, 0.0);
        }
    }

    /// It whole, as it is now.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot { sim: self.play.sim.clone(), words: self.words.clone(), over: self.over }
    }

    /// Whether its story is that moment's up to where that one stood: played
    /// on from there, it plays as it would have from its start.
    pub fn continues(&self, moment: &Snapshot) -> bool {
        moment.sim.director.same_story_so_far(&self.play.sim.director)
    }

    /// At that moment of another play of a story the same so far, its own
    /// story to play on with.
    pub fn restore_into(&mut self, moment: &Snapshot) {
        let story = self.play.sim.director.clone();
        self.restore(moment);
        self.play.sim.director.take_story(&story);
    }

    /// Back to that moment of it.
    pub fn restore(&mut self, moment: &Snapshot) {
        self.play.sim = moment.sim.clone();
        (self.words, self.over) = (moment.words.clone(), moment.over);
        // (Its place, read again at once: the view and what is drawn go by
        // it before the next frame does.)
        if let Some(level) = self.play.sim.place {
            self.play.set_ground(&self.art, level);
        }
    }

    /// What can be seen of it now is looked at.
    pub fn watched(&self, watch: &mut engine::Watch) {
        watch.fighters = self.play.sim.fight.as_ref().map_or(Vec::new(), |fight| fight.fighters());
        // (The names lines speak of them by.)
        for who in self.play.sim.stage.cast.keys().filter(|who| !who.is_thing()) {
            if !watch.names.contains_key(who) {
                watch.names.insert(*who, self.play().sim.cast.who(*who).1);
            }
        }
        watch.look(&self.play.sim.director, &self.play.sim.stage, self.view().centre, self.page());
    }

    /// The whole place, as the game's screen shows it all (no closer than
    /// as it is, no further than the camera goes).
    pub fn whole_view(&self) -> Viewport {
        let screen = glam::Vec2::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
        let place = self.play.place_size();
        let close = if place.x > 0.0 && place.y > 0.0 { (screen.x / place.x).min(screen.y / place.y).clamp(0.1, 1.0) } else { 1.0 };
        Viewport::of(None, close, place, screen)
    }

    /// The whole place drawn, the stage as it is now (no words over it);
    /// the view the film has, kept as it was.
    pub fn draw_whole(&mut self, canvas: &mut Canvas, world: &mut Screen) {
        let whole = self.whole_view();
        let sim = &mut self.play.sim;
        let (focus, close) = (sim.focus, sim.stage.view.zoom.now);
        (sim.focus, sim.stage.view.zoom.now) = (Some(whole.centre), whole.close);
        let clear = Clear { page: false, portrait: None, bubbles: 0 };
        self.play.draw(canvas, world, &self.art, &self.art.ui, clear);
        world.over(&canvas.data);
        let sim = &mut self.play.sim;
        (sim.focus, sim.stage.view.zoom.now) = (focus, close);
    }

    /// The stage drawn with the camera on that point, as close as it is now
    /// (no words over it), and the view it was drawn with; the view the film
    /// has, kept as it was. For a close look at someone the film's camera
    /// leaves out of its frame.
    pub fn draw_around(&mut self, canvas: &mut Canvas, world: &mut Screen, centre: glam::Vec2) -> Viewport {
        let focus = self.play.sim.focus;
        self.play.sim.focus = Some(centre);
        let view = self.view();
        let clear = Clear { page: false, portrait: None, bubbles: 0 };
        self.play.draw(canvas, world, &self.art, &self.art.ui, clear);
        world.over(&canvas.data);
        self.play.sim.focus = focus;
        view
    }

    /// What the view shows of the place now.
    pub fn view(&self) -> Viewport {
        let screen = glam::Vec2::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
        Viewport::of(self.play.sim.focus, self.play.sim.stage.view.zoom.now, self.play.place_size(), screen)
    }

    /// The scene as it stands, to be told.
    pub fn play(&self) -> &Play {
        &self.play
    }

    /// The line on screen.
    pub fn page(&self) -> Option<&str> {
        self.words.as_ref().and_then(Words::page)
    }

    /// Whether its scene can be seen: no fade over it.
    pub fn seen(&self) -> bool {
        self.play.sim.stage.light.fade.level < 0.05
    }

    /// The box as it is now: its pages, the one shown, the choices.
    pub fn words(&self) -> Option<&Words> {
        self.words.as_ref()
    }

    /// The box emptied (a screen of the game opens over it).
    pub fn words_gone(&mut self) {
        self.words = None;
    }

    /// Nobody at the keys (a check, a picture): who roams goes on, to the next scene.
    pub fn nobody_plays(&mut self) {
        self.play.sim.director.author.nobody_plays = true;
    }

    /// Its lines are noted as they play.
    pub fn keep_lines(&mut self) {
        self.play.sim.director.author.lines = Some(Vec::new());
        self.play.sim.director.author.spans = Some(Vec::new());
    }

    /// The lines started and ended since last asked, each on its course.
    pub fn spans_played(&mut self) -> Vec<engine::LineMark> {
        self.play.sim.director.author.spans.as_mut().map_or(Vec::new(), std::mem::take)
    }

    /// The lines played since last asked.
    pub fn lines_played(&mut self) -> Vec<i32> {
        self.play.sim.director.author.lines.as_mut().map_or(Vec::new(), std::mem::take)
    }

    /// How far it has gone, in frames.
    pub fn played(&self) -> f32 {
        self.play.sim.clock
    }

    /// Those where each scene started, so far.
    pub fn scenes(&self) -> &[f32] {
        &self.play.sim.scene_starts
    }

    /// Whether it waits for an answer.
    pub fn asking(&self) -> bool {
        self.words.as_ref().is_some_and(|words| !words.options.is_empty())
    }

    /// What it plays with, to play it again from the start.
    pub fn parts(self) -> (Art, Option<Box<dyn Hears>>) {
        (self.art, self.audio)
    }

    /// Played on at once and without a sound, as long as asked; then its
    /// music goes on from there.
    pub fn skip_while(&mut self, going: impl Fn(&Film) -> bool) {
        let (audio, auto) = (self.audio.take(), self.auto);
        self.auto = true;
        while going(self) && !self.over {
            let enter = self.asking();
            self.tick(Keys { enter, ..Keys::default() }, 1.0);
        }
        (self.audio, self.auto) = (audio, auto);
        let mut means = full(&self.art, &mut self.audio);
        self.play.song_again(&mut means);
    }

    /// The lower screen as it is now.
    pub fn draw(&mut self, canvas: &mut Canvas, world: &mut Screen) {
        self.draw_with(canvas, world, None);
    }

    /// The lower screen as it is now, the box drawn by that instead (a
    /// screen of the game open over the scene).
    pub fn draw_with(&mut self, canvas: &mut Canvas, world: &mut Screen, mut boxed: Option<DrawBox<'_>>) {
        let words = self.words.as_ref();
        let clear = Clear { page: self.page().is_some(), portrait: words.and_then(|words| words.portrait.as_ref()).map(|portrait| portrait.x), bubbles: 0 };
        // (The box where the scene has it, at its rank; a fight's choices
        // between those who fight: as the game shows them.)
        let shown = &self.play.sim.stage.words;
        let (box_top, fight) = (shown.box_top, shown.hud.iter().any(Option::is_some));
        let (words, ui) = (self.words.as_ref(), &self.art.ui);
        let mut the_box = |canvas: &mut Canvas| {
            if let Some(boxed) = boxed.as_mut() {
                boxed(canvas, ui);
            } else if let Some(words) = words {
                words.draw(canvas, ui, box_top);
                words.draw_choices(canvas, ui, fight);
            }
        };
        self.play.draw_with_box(canvas, world, &self.art, ui, clear, &mut the_box);
        // (What the scene holds under the box, what is told over it.)
        if self.debug.shown {
            crate::debug::draw(self, world);
        }
        world.over(&canvas.data);
        if self.debug.shown {
            crate::debug::draw_told(self, world);
        }
    }

    /// A frame goes by.
    pub fn tick(&mut self, keys: Keys, frames: f32) {
        let frames = frames * stage_view::stage::scene_frames(&self.play.sim.stage, keys.hurry);
        self.play.sim.clock += frames;
        if let Some(audio) = &mut self.audio {
            audio.tick();
        }
        scene_keys(&mut self.play.sim.director, &mut self.play.sim.stage, &keys);
        let enter = keys.enter || self.read_by_itself();
        self.keys(&Keys { enter, ..keys });
        if self.over {
            return;
        }
        let (calls, menu) = self.play.direct(frames);
        if let Some(menu) = menu {
            self.ask(menu);
        }
        for call in calls {
            self.call(call);
        }
        let mut means = full(&self.art, &mut self.audio);
        self.play.pass(frames, &mut means);
        // (`no_fade`: the scene always seen.)
        if self.debug.no_fades {
            self.play.sim.stage.light.fade = engine::Fade::CLEAR;
        }
        // (At the speed the game's options have at first; `text`: at once.)
        if let Some(words) = &mut self.words {
            if self.debug.fast_text {
                words.pace = Some(10_000.0);
            }
            words.write_out(Options::default().text_speed, self.audio.as_deref());
        }
    }

    /// Whether the line on screen has waited long enough to be read, in a
    /// film going on by itself.
    fn read_by_itself(&self) -> bool {
        let waiting = self.words.as_ref().and_then(Words::waiting).filter(|_| self.auto);
        waiting.is_some_and(|(written, waited)| waited as f32 >= engine::read_after(written, false))
    }

    /// The keys of the box: a line read, a choice made, given to who waits
    /// for it as the game gives it.
    fn keys(&mut self, keys: &Keys) {
        let Some(words) = &mut self.words else {
            return;
        };
        // (The click of a line read, as in the game.)
        let Some(chosen) = words.press(keys, self.audio.as_deref().filter(|_| !self.auto)) else {
            return;
        };
        let choice = !words.options.is_empty();
        self.words = None;
        let director = &mut self.play.sim.director;
        match &mut self.play.sim.fight {
            Some(fight) if choice && fight.asking() => fight.choose(chosen),
            _ if director.hold() == Some(Hold::Question) => director.answer(choice.then_some(chosen)),
            _ if director.reading() && !director.line_timed() => director.read(),
            _ => {}
        }
    }

    /// The choices of a question, under the line shown (or alone).
    fn ask(&mut self, options: Vec<String>) {
        let words = self.words.get_or_insert_default();
        (words.options, words.cursor) = (options, stage_view::ui::ListCursor::default());
    }

    fn call(&mut self, call: Call) {
        let mut means = full(&self.art, &mut self.audio);
        match call {
            Call::Opens { cleared, .. } => {
                self.words = None;
                self.play.opens(cleared, &mut means);
            }
            Call::Show(beat) => self.play.beat(beat, &mut means),
            Call::Say { .. } => {
                let said = self.play.said(&call, self.play.sim.director.line_timed());
                self.words = said.map(|said| Words::said(said, &self.art));
            }
            Call::Ask(answers) => {
                let answers = answers.iter().map(|answer| self.play.sim.cast.names_in(answer)).collect();
                self.ask(answers);
            }
            Call::LineGone => self.words = None,
            Call::Host { fields, .. } if fields[0] == "fight" => self.play.fought(&fields, &means),
            // (A line the game holds for and has nothing to do with (a ROM's `menu`): on.)
            Call::Host { fields, .. } if POKEMON.host_holds.contains(&fields[0].as_str()) => self.play.sim.director.menu_done(false),
            Call::Host { .. } => {}
            Call::Game { ask, .. } => self.asked(GameRequest::Ask(ask)),
            Call::Over => {
                self.over = true;
                self.words = None;
                if self.curtain {
                    self.play.sim.stage.curtain("The End   (Escape: quit)");
                }
            }
            Call::Checkpoint => self.asked(GameRequest::Checkpoint),
        }
    }

    /// Something only the game does: kept for it, or (no game) done at once
    /// as if it went well.
    fn asked(&mut self, request: GameRequest) {
        if self.hosted {
            self.requests.push(request);
            return;
        }
        let director = &mut self.play.sim.director;
        // (Nobody at the keys (checks, pictures, the timeline): passed at once.)
        if request.holds() {
            director.menu_done(false);
        }
    }
}

/// What a scene plays with, all of it heard as loud as it can be.
fn full<'a>(art: &'a Art, audio: &'a mut Option<Box<dyn Hears>>) -> Means<'a> {
    Means { art, ui: &art.ui, tables: &art.tables, audio, music: 1.0, sounds: 1.0 }
}

/// What draws the box over the scene, when a screen of the game has it.
pub type DrawBox<'a> = &'a mut dyn FnMut(&mut Canvas, &UiArt);
