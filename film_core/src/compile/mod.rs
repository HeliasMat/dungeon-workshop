//! A film turned into lines for the engine, each with the film's own line.
//!
//! - `scene`: a scene opened, cast, set, played and closed;
//! - `said`: a line said;
//! - here: the film as a whole, and a line sent where it is played.

use std::collections::HashSet;

use ::scenes::vocabulary::POKEMON;
use engine::{Beat, FlowBeat, Scene, Script, StoryEvent, ViewBeat, Who};

use crate::film::{Film, ReadAs, Sortie};
use crate::ir::{Instr, Shape};
use crate::verbs::command;
use crate::words::told;
use crate::{emit, fields};
use scenes::read;

mod for_ds;
mod said;

pub(crate) mod scene;
pub mod scenes;

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
    /// (`Lexicon::own_pictures`). What a game's pack takes of its `images/`.
    pub pictures: Vec<String>,
}

/// A film's text turned into lines, with these tables (its project's, its
/// data's, its language).
pub fn compile_with(lexicon: &'static crate::verbs::Lexicon, text: &str, tables: std::sync::Arc<crate::tables::Tables>) -> Written {
    compile_as(lexicon, text, tables, false)
}

/// A scene the game makes of a place (`game::roam`), turned into
/// lines with those tables: the words only the game writes (`player`,
/// `dialogue`, `explore`, `game`, `way`) are known there.
pub fn compile_place_with(lexicon: &'static crate::verbs::Lexicon, text: &str, tables: std::sync::Arc<crate::tables::Tables>) -> Written {
    compile_as(lexicon, text, tables, true)
}

fn compile_as(lexicon: &'static crate::verbs::Lexicon, text: &str, tables: std::sync::Arc<crate::tables::Tables>, made_by_the_game: bool) -> Written {
    let mut film = Film::with(lexicon, tables);
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
    // (Written for the DS: what it can't play, told.)
    if let Some(mode) = film.for_ds {
        for_ds::tell(&mut film, mode, &events[0].scenes);
    }
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

/// How wide one stands on the ground, by its shadow (small, middling, big).
const FOOTINGS: [f32; 3] = [16.0, 24.0, 40.0];

/// The size of each one the film names, once.
fn bodies(film: &Film) -> Vec<Beat> {
    let mut seen = HashSet::new();
    let mut bodies = Vec::new();
    for name in &film.names {
        let code = &film.actors[name];
        if !seen.insert(code.clone()) {
            continue;
        }
        if let (Some((wide, high)), Some(who)) = (film.body_of(code), Who::parse(code)) {
            bodies.push(Beat::View(ViewBeat::Body { who, size: glam::Vec2::new(wide as f32, high as f32) }));
        }
        // (How wide each stands on the ground, by its shadow: how far the
        // others keep from it.)
        if let (Some(shadow), Some(who)) = (film.kind_of(code).and_then(|kind| kind.shadow), Who::parse(code)) {
            bodies.push(Beat::Place(engine::PlaceBeat::Footing { who, wide: FOOTINGS[usize::from(shadow.min(2))] }));
        }
    }
    bodies
}

/// What the film writes, each line of its plan starting where its beats do
/// (`filmline <n>`, with how it ends), each line gone back to after another
/// told again (`filmline <n> again`), but at a scene's opening and a label.
pub fn with_lines(written: Vec<(usize, Sortie)>, plan: &crate::time::Plan) -> Vec<(usize, Sortie)> {
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
