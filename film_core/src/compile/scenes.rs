//! A film's scenes told apart, by what each line is: what comes before them
//! (`title:`, `pinchy = krabby`, a lasting `spot`, `thing x image`), then
//! each `scene:` with its place, those it names, how it opens, its lines.

use crate::film::Film;
use crate::ir::{Instr, Shape};
use crate::plot::{StoryMap, slug, uses_story};
use crate::tables::SOMEWHERE;
use crate::text::recipes::unfold;
use crate::words::{digits, plain, tile};

/// A scene: its place, those it names, its lines, how it opens.
pub struct SceneOf {
    pub place: (u16, i32, i32),
    /// Its place as written (`beach`): the scene's name for `go scene`, `seen(beach)`.
    pub word: String,
    pub listed: Vec<String>,
    pub lines: Vec<Instr>,
    pub transition: String,
}

/// The fade a scene opens with when it says none.
const FADE: &str = "fade";

/// The ways a scene closes on the next, as `transition` says them: a fade to
/// black, to white, the boss's flashes, a wipe, the circle closing, a cut.
pub const TRANSITIONS: [&str; 6] = ["fade", "white", "boss", "wipe", "iris", "cut"];

/// `transition iris`, alone on its line (no `in`/`out`): how the next scene
/// opens, as `transition: iris` says it.
fn transition_given(instr: &Instr) -> Option<String> {
    match instr.words() {
        [kind] if instr.verb() == Some("transition") => Some(plain(kind)),
        _ => None,
    }
}

/// `spot throne 400 200`, `spot throne tile 12 7`: before any scene.
fn spot_given(instr: &Instr) -> Option<(String, i32, i32)> {
    let words = instr.words();
    let on_tile = words.len() == 4 && words[1] == "tile";
    if instr.verb() != Some("spot") || !(words.len() == 3 || on_tile) || !digits(&words[words.len() - 2]) || !digits(&words[words.len() - 1]) {
        return None;
    }
    let (x, y) = (words[words.len() - 2].parse().ok()?, words[words.len() - 1].parse().ok()?);
    Some(if on_tile { (plain(&words[0]), tile(x), tile(y)) } else { (plain(&words[0]), x, y) })
}

/// `thing letter image letter.png`: before any scene, the words after `thing`.
fn picture_given(instr: &Instr) -> Option<Vec<String>> {
    let words = instr.words();
    (instr.verb() == Some("thing") && words.len() >= 3 && plain(&words[1]) == "image").then(|| words.to_vec())
}

/// `scene: beach piplup pinchy`.
fn scene_said(film: &mut Film, number: usize, place: &str, listed: &[String], transition: String) -> SceneOf {
    let place = if place.is_empty() {
        film.wrong(number, "a scene says where it is: \"scene: beach\"".to_string());
        "town"
    } else {
        place
    };
    let listed: Vec<String> = listed.iter().map(|word| word.trim_end_matches(',').to_string()).collect();
    // (`residents`: who lives in the place, named by the place itself.)
    for word in listed.iter().filter(|word| plain(word) != "residents") {
        if film.actor(word).is_none() {
            film.unknown_someone(number, word);
        }
    }
    let word = place.to_string();
    let place = film.tables.place_of(place).unwrap_or_else(|problem| {
        film.wrong(number, problem);
        SOMEWHERE
    });
    SceneOf { place, word, listed, lines: Vec::new(), transition }
}

/// The film's scenes: a scene is a place and its lines, with their numbers.
pub fn read(text: &str, film: &mut Film) -> Vec<SceneOf> {
    let mut scenes: Vec<SceneOf> = Vec::new();
    let mut transition = FADE.to_string();
    for instr in unfold(text, film) {
        let number = instr.number;
        match &instr.shape {
            Shape::Transition(said) => transition = plain(said.trim()),
            _ if transition_given(&instr).is_some() => {
                let kind = transition_given(&instr).unwrap_or_default();
                match TRANSITIONS.contains(&kind.as_str()) {
                    true => transition = kind,
                    false => film.unknown(number, "unknown transition", &kind, TRANSITIONS.iter().copied(), &TRANSITIONS.join(", ")),
                }
            }
            Shape::Title(said) => film.title = said.trim().to_string(),
            // (`title The Big Swim`, as `title: The Big Swim`.)
            _ if scenes.is_empty() && instr.verb() == Some("title") && !instr.words().is_empty() => film.title = instr.words().join(" "),
            Shape::Naming { name, species } if scenes.is_empty() => match film.actor(species) {
                Some(code) => film.give_name(plain(name), code),
                None => film.unknown_someone(number, species),
            },
            Shape::Opens { place, listed } => {
                let scene = scene_said(film, number, place, listed, std::mem::replace(&mut transition, FADE.to_string()));
                scenes.push(scene);
            }
            _ => match scenes.last_mut() {
                Some(scene) => scene.lines.push(instr),
                // (`collisions on`: for each scene of the film.)
                None if instr.verb() == Some("collisions") && matches!(instr.words(), [on] if on == "on" || on == "off") => {
                    film.collisions = instr.words()[0] == "on";
                }
                None => match (spot_given(&instr), picture_given(&instr)) {
                    (Some(spot), _) => film.lasting_spots.push(spot),
                    (_, Some(words)) => crate::groups::things::own_picture(film, &words, number),
                    _ => film.wrong(number, "before the first \"scene: <place>\"".to_string()),
                },
            },
        }
    }
    scenes
}

/// The film's map of its moments and scenes (`plot`), from its scenes.
impl StoryMap {
    /// A film's, from its scenes.
    pub fn of(scenes: &[SceneOf], text: &str) -> StoryMap {
        let mut map = StoryMap { used: uses_story(text), ..StoryMap::default() };
        if !map.used {
            return map;
        }
        fn each<'a>(lines: &'a [Instr], into: &mut Vec<&'a Instr>) {
            for line in lines {
                into.push(line);
                if let Some(inner) = &line.inner {
                    each(inner, into);
                }
            }
        }
        for (at, scene) in scenes.iter().enumerate() {
            map.scenes.push(slug(&scene.word));
            let mut lines = Vec::new();
            each(&scene.lines, &mut lines);
            for line in lines.iter().filter(|line| line.verb() == Some("moment")) {
                map.moments.push((slug(&line.words().join(" ")), at));
            }
        }
        for (at, scene) in scenes.iter().enumerate() {
            let mut lines = Vec::new();
            each(&scene.lines, &mut lines);
            for line in lines.iter().filter(|line| line.verb() == Some("go") && line.words().first().is_some_and(|word| word == "moment")) {
                let name = slug(&line.words()[1..].join(" "));
                if let Some(rank) = map.moments.iter().position(|(known, scene)| *known == name && *scene != at)
                    && !map.reached_from_afar.contains(&rank)
                {
                    map.reached_from_afar.push(rank);
                }
            }
        }
        map
    }
}
