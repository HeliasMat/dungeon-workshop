//! A film turned into lines for the engine, each with the film's own line.
//!
//! - `scene`: a scene opened, cast, set, played and closed;
//! - `said`: a line said;
//! - here: the film as a whole, and a line sent where it is played.

use std::collections::HashSet;

use ::scenes::vocabulary::POKEMON;
use engine::{Beat, FlowBeat, Scene, Script, StoryEvent, ViewBeat, Who};

use crate::film::{Film, Sortie};
use crate::ir::{Instr, Shape};
use crate::text::told;
use crate::verbs::command;
use crate::{emit, fields};
use scenes::read;

mod said;
pub use said::Speaking;
pub(crate) mod scene;
pub(crate) mod scenes;

/// The event a film opens, for the engine (its first chapter, first step).
const EVENT: &str = "EVENT_M01_01";

impl Film {
    /// A line of a scene.
    pub fn play(&mut self, item: &Instr) {
        let kept = std::mem::replace(&mut self.number, item.number);
        let shown = !matches!(item.shape, Shape::Blank);
        if shown {
            let lane = self.piste_of(item);
            let wait = !matches!(item.shape, Shape::Word { wait: false, .. }) && !item.text.ends_with(" &");
            self.plan.begin(item.number, lane, wait, self.written.len());
        }
        self.played(item);
        if shown {
            self.plan.end();
        }
        self.number = kept;
    }

    /// The track a line is shown on: someone's, the camera's, the music's,
    /// the sounds', the scenes', or the film's.
    fn piste_of(&mut self, item: &Instr) -> crate::time::Lane {
        use crate::time::Lane;
        const CAMERA: [&str; 9] = ["camera", "view", "zoom", "pan", "dolly", "scroll", "frame", "follow", "shot"];
        const MUSIC: [&str; 3] = ["music", "musicspeed", "silence"];
        const SOUNDS: [&str; 3] = ["sound", "ambient", "noise"];
        match &item.shape {
            Shape::Said { who, .. } if self.actor(who).is_some() => Lane::Who(crate::words::plain(who)),
            Shape::Opens { .. } | Shape::Transition(_) => Lane::Scenes,
            Shape::Word { verb, words, .. } => {
                if CAMERA.contains(&verb.as_str()) {
                    Lane::Camera
                } else if MUSIC.contains(&verb.as_str()) || (verb == "wait" && words.first().is_some_and(|word| word == "music")) {
                    Lane::Music
                } else if SOUNDS.contains(&verb.as_str()) || (verb == "wait" && words.first().is_some_and(|word| word == "sound")) {
                    Lane::Sounds
                } else {
                    words.iter().find(|word| self.actor(word).is_some()).map_or(Lane::Film, |who| Lane::Who(crate::words::plain(who)))
                }
            }
            _ => Lane::Film,
        }
    }

    /// A line said by someone, as written in a line that does more.
    pub fn play_said(&mut self, who: &str, words: &str, number: usize) {
        self.said(who, None, words, number);
    }

    fn played(&mut self, item: &Instr) {
        let number = item.number;
        if !matches!(item.verb(), Some("catch" | "miss")) {
            self.throw_lands();
        }
        if let Shape::Said { who, brackets: None, words, .. } = &item.shape
            && said::unknown_speaker(self, who)
        {
            self.said_unknown(who, words);
            return;
        }
        match &item.shape {
            // (`who: words &`: said beside the film, which goes on.)
            Shape::Said { who, beside: true, .. } if told(who) || self.actor(who).is_some() => _ = crate::verbs::timing::said_meanwhile(self, item),
            Shape::Said { who, brackets, words, .. } if told(who) || self.actor(who).is_some() => self.said(who, brackets.as_deref(), words, number),
            Shape::Said { who, .. } if who != "walk" => self.unknown_someone(number, who),
            _ => command(self, item),
        }
    }
}

/// A film as it comes out: lines for the engine, what is wrong, who is who.
pub struct Written {
    /// What the lines use: names, groups, the event they open.
    pub head: String,
    pub lines: engine::Lines,
    pub problems: Vec<engine::Diagnostic>,
    /// Each code, by the name the film gives it.
    pub names: Vec<(String, String)>,
    /// The codes of those of the film's crowds and bands.
    pub crowd: Vec<String>,
    /// What the engine plays, written out by the film itself (`out`):
    /// its lines, each one beat, and what was wrong in writing them.
    pub engine: engine::Script,
    /// Those lines read as the engine plays them: its events, each scene
    /// its beats.
    pub events: Vec<engine::StoryEvent>,
    /// The lines the words still hand over as fields, not as beats
    /// (`Sortie::Champs`), each with its line of the film: what is left to
    /// make typed (`scene_player fingerprint --fields`).
    pub left_as_fields: Vec<(usize, Vec<String>)>,
    /// How lines were read, by line: told when asked (`check --all`).
    pub notes: Vec<(usize, String)>,
    /// What each line of the text was read as (a word's line, not one a
    /// recipe or a word wrote for it).
    pub read_as: std::collections::BTreeMap<usize, ReadAs>,
    /// The songs the film chose itself for scenes that ask none, their
    /// decor's (`place_songs`): each by the line it plays from.
    pub songs: Vec<(usize, String)>,
    /// Its plan: a line each, its thread, placing and time (`crate::time`).
    pub plan: crate::time::Plan,
    /// The pictures of one's own it draws, by their files (`footprints.png`,
    /// `bobber.4.png`): its things, and those its words pick by themselves
    /// (`crate::OWN_PICTURES`). What a game's pack takes of its `images/`.
    pub pictures: Vec<String>,
}

/// A line of the text as it was read: its word, the family whose form fit
/// it, what that form gave each of its names, and where in the line.
#[derive(Clone, Debug)]
pub struct ReadAs {
    pub word: String,
    pub family: &'static str,
    pub values: Vec<crate::grammar::Value>,
}

impl ReadAs {
    /// Where the line's word stands among the words written there: 0 when
    /// written first (`music calm`), 1 when after who acts (`pikachu walk
    /// to piplup`); none if the line now written is another word's.
    pub fn word_at(&self, written: &str) -> Option<usize> {
        written.split_whitespace().take(2).position(|said| crate::words::plain(said) == self.word)
    }

    /// The edit that makes that field of the line, as it was read, these
    /// words; why not, if the line now written is another word's, or has
    /// no such field said. Written who first, the field is found where it
    /// is written: who before the word, the rest after it.
    pub fn edit(&self, number: usize, written: &str, name: &str, words: &[String]) -> Result<crate::doc::Edit, String> {
        let changed = || format!("line {number}: changed since the film played");
        let word_at = self.word_at(written).ok_or_else(changed)?;
        let value = self.values.iter().find(|value| value.name == name).ok_or_else(|| format!("line {number}: \"{}\" has no field \"{name}\"", self.word))?;
        let at = value.at.ok_or_else(|| format!("line {number}: \"{name}\" is not written there"))?;
        let long = value.words.len();
        if word_at == 0 {
            return Ok(crate::doc::Edit::Words { number, at, long, with: words.to_vec() });
        }
        // (Its words among those written: those before the word, then the
        // word, then the rest; a field never across the word.)
        let tokens: Vec<(usize, usize)> = tokens(written);
        let placed = |read: usize| if read < word_at { read } else { read + 1 };
        if long == 0 || (at < word_at && at + long > word_at) {
            return Err(format!("line {number}: \"{name}\" is not written there"));
        }
        let (first, last) = (placed(at), placed(at + long - 1));
        let (Some(&(start, _)), Some(&(_, end))) = (tokens.get(first), tokens.get(last)) else { return Err(changed()) };
        let text = format!("{}{}{}", &written[..start], words.join(" "), &written[end..]);
        Ok(crate::doc::Edit::Line { number, text })
    }
}

/// Where each word of a line is, in bytes.
fn tokens(written: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut start = None;
    for (at, letter) in written.char_indices().chain(std::iter::once((written.len(), ' '))) {
        match (letter.is_whitespace(), start) {
            (false, None) => start = Some(at),
            (true, Some(from)) => {
                found.push((from, at));
                start = None;
            }
            _ => {}
        }
    }
    found
}

/// A film's text turned into lines, with these tables (its project's, its
/// data's, its language).
pub fn compile_with(text: &str, tables: std::sync::Arc<crate::tables::Tables>) -> Written {
    compile_as(text, tables, false)
}

/// A scene the game makes of a place (`game::roam`), turned into
/// lines with those tables: the words only the game writes (`player`,
/// `dialogue`, `explore`, `game`, `way`) are known there.
pub fn compile_place_with(text: &str, tables: std::sync::Arc<crate::tables::Tables>) -> Written {
    compile_as(text, tables, true)
}

fn compile_as(text: &str, tables: std::sync::Arc<crate::tables::Tables>, made_by_the_game: bool) -> Written {
    let mut film = Film::with(tables);
    film.game.made_by_the_game = made_by_the_game;
    film.source = std::sync::Arc::from(text);
    let mut scenes = read(text, &mut film);
    film.story = crate::plot::StoryMap::of(&scenes, text);
    let songs = scene::place_songs(&film, &mut scenes);
    let (mut song, mut body_at) = (None, None);
    for at in 0..scenes.len() {
        film.one_scene(at, &scenes, &mut song, &mut body_at);
    }
    // (How big they are, told once: the view and the places go by it.)
    let at = body_at.unwrap_or(0);
    let bodies = bodies(&film);
    let moved = bodies.len();
    film.written.splice(at..at, bodies.into_iter().map(|beat| (0, Sortie::Beat(beat))));
    // (Its recipes and those it uses, each checked once, alone, where it is
    // written: those the film names known by then.)
    crate::text::checked::check(&mut film);
    // (The lines of the files taken in, told as the line that takes them in.)
    let included = std::mem::take(&mut film.included);
    let told = |number: usize| crate::text::recipes::told_line(&included, number);
    film.plan.renumbered(|number| told(number).0, at, moved);
    for (number, _) in &mut film.written {
        *number = told(*number).0;
    }
    for problem in &mut film.problems {
        if problem.line() > 0
            && let (line, Some((name, own))) = told(problem.line())
        {
            (problem.span.start.line, problem.span.end.line) = (line, line);
            *problem = std::mem::take(problem).led_by(&format!("in {name}, line {own}: "));
        }
    }
    for (number, _) in &mut film.notes {
        *number = told(*number).0;
    }
    film.read_as.retain(|number, _| *number < crate::text::recipes::INCLUDED);
    let left_as_fields: Vec<(usize, Vec<String>)> = film
        .written
        .iter()
        .filter_map(|(number, made)| match made {
            Sortie::Champs(fields) => Some((*number, fields.clone())),
            Sortie::Beat(_) | Sortie::Scene { .. } => None,
        })
        .collect();
    // (A line no beat reads: told, as the engine would not play it.)
    for (number, fields) in &left_as_fields {
        let why = Beat::parse(fields, &POKEMON).err().map_or_else(String::new, |error| format!(" ({error})"));
        film.wrong(*number, format!("\"{}\": the engine does not read this line{why}", fields.join(" ")));
    }
    let played = with_lines(std::mem::take(&mut film.written), &film.plan);
    let events = vec![StoryEvent { number: 0, name: EVENT.to_string(), moment: (1, 1), scenes: scenes_of(&played), film: true }];
    // (As lines, for a file the engine reads and for those who look at them.)
    let lines: engine::Lines = played.iter().filter(|(_, made)| !matches!(made, Sortie::Champs(_))).map(|(number, made)| (*number, made.fields())).collect();
    let event = (0, fields!["event", 0, EVENT, 1, 1]);
    let engine = Script { lines: std::iter::once(event).chain(lines.iter().cloned()).collect(), problems: Vec::new() };
    let head = format!("event\t0\t{EVENT}\t1\t1\n");
    // (Those of a crowd, by their codes: no one speaks of them by name.)
    let crowd = film.groups.values().flatten().filter_map(|name| film.actors.get(name).cloned()).collect();
    let names = film.names.iter().map(|name| (film.actors[name].clone(), name.clone())).collect();
    let pictures = std::mem::take(&mut film.pictures);
    Written {
        head,
        lines,
        problems: film.problems,
        names,
        crowd,
        engine,
        events,
        left_as_fields,
        notes: film.notes,
        read_as: film.read_as,
        songs,
        plan: film.plan,
        pictures,
    }
}

/// The size of each one the film names, once.
fn bodies(film: &Film) -> Vec<Beat> {
    let mut seen = HashSet::new();
    let mut bodies = Vec::new();
    for name in &film.names {
        let code = &film.actors[name];
        if let (Some((wide, high)), Some(who)) = (film.body_of(code).filter(|_| seen.insert(code.clone())), Who::parse(code)) {
            bodies.push(Beat::View(ViewBeat::Body { who, size: glam::Vec2::new(wide as f32, high as f32) }));
        }
    }
    bodies
}

/// What the film writes, each line of its plan starting where its beats do
/// (`filmline <n>`, with how it ends), each line gone back to after another
/// told again (`filmline <n> again`), but at a scene's opening and a label.
pub(crate) fn with_lines(written: Vec<(usize, Sortie)>, plan: &crate::time::Plan) -> Vec<(usize, Sortie)> {
    let mut starts: std::collections::BTreeMap<usize, Vec<&crate::time::PlanLine>> = std::collections::BTreeMap::new();
    for (_, line) in plan.shown() {
        starts.entry(line.written).or_default().push(line);
    }
    let mut out = Vec::with_capacity(written.len() * 3 / 2);
    let mut last = None;
    let mut waiting: Vec<&crate::time::PlanLine> = Vec::new();
    for (at, (number, made)) in written.into_iter().enumerate() {
        let mut fresh = std::mem::take(&mut waiting);
        fresh.extend(starts.remove(&at).unwrap_or_default());
        if matches!(made, Sortie::Scene { .. } | Sortie::Beat(Beat::Flow(FlowBeat::Label(_)))) {
            waiting = fresh;
            out.push((number, made));
            continue;
        }
        if fresh.is_empty() && Some(number) != last {
            out.push((number, Sortie::Beat(emit::film_line(number as i32, engine::LineEnd::Again))));
        }
        for line in fresh {
            out.push((line.number, Sortie::Beat(emit::film_line(line.number as i32, line_end(line)))));
        }
        last = Some(number);
        out.push((number, made));
    }
    out
}

/// How a line of the plan ends, for the engine to tell: with the next of
/// its course if its course waits for it, else by its handle.
fn line_end(line: &crate::time::PlanLine) -> engine::LineEnd {
    use crate::time::{Handle, Length};
    use engine::LineEnd;
    if line.waits {
        return LineEnd::Next;
    }
    match &line.length {
        Length::Known(frames) if *frames > 0 => LineEnd::Frames(engine::Frames(*frames as f32)),
        Length::Event { handle: Handle::Who(code), .. } => Who::parse(code).map_or(LineEnd::Next, LineEnd::Who),
        Length::Event { handle: Handle::Tracks(node), .. } => engine::NodeRef::parse(node).map_or(LineEnd::Next, LineEnd::Track),
        _ => LineEnd::Next,
    }
}

/// The film's scenes, each its beats, as the engine plays them.
fn scenes_of(played: &[(usize, Sortie)]) -> Vec<Scene> {
    let mut scenes: Vec<Scene> = Vec::new();
    for (_, made) in played {
        match made {
            Sortie::Scene { level, name } => scenes.push(Scene { level: *level, name: name.clone(), ..Scene::default() }),
            Sortie::Beat(beat) => {
                if scenes.is_empty() {
                    scenes.push(Scene::default());
                }
                scenes.last_mut().expect("a scene").beats.push(beat.clone());
            }
            Sortie::Champs(_) => {}
        }
    }
    // (A way out to a place: to the first scene there, `passage`.)
    let named: Vec<(u16, String)> = scenes.iter().map(|scene| (scene.level, scene.name.clone())).collect();
    for beat in scenes.iter_mut().flat_map(|scene| scene.beats.iter_mut()) {
        if let Beat::Flow(FlowBeat::Door { scene, .. } | FlowBeat::GotoScene(scene)) = beat
            && let Some(level) = scene.strip_prefix("level:").and_then(|level| level.parse::<u16>().ok())
            && let Some((_, name)) = named.iter().find(|(there, _)| *there == level)
        {
            *scene = name.clone();
        }
    }
    scenes
}
