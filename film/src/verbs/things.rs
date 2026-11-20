//! Things of a scene: where they stand, what they do. A thing is an item of
//! the game, one of its props (`library/tables/objects.txt`), or a picture of the
//! film's own; the words are the same for all (see `names::things`).

use engine::{Num, Pose, Spot};

use super::{Line, Word};
use crate::emit;
use crate::film::{Film, Kind, Stuff};
use crate::names::things::{Thing, leading_thing, thing_said};
use crate::spots::routes::found;
use crate::spots::somewhere;
use crate::words::{plain, spot};

pub const WORDS: &[Word] = &[
    Word::new("put", &["object what:thing..."], "put object: find, put, thing").does(put_on_stage),
    Word::new("shift", &["thing:word [to] where:place [in d:duration]"], "find, go").does(slid),
    Word::new("open", &["thing:word"], "anim, sound").maybe(worked),
    Word::new("close", &["thing:word"], "pose image 1, sound").maybe(worked),
];

/// The things laid just where they are said, under who stands on them: in
/// the water, on an islet one cannot walk to, flat in the ground.
const EXACTLY: [&str; 3] = ["stepping_stone", "egg", "quicksand"];

/// How long the embers (or the smoke) come off a flame lit (or put out).
const PUFF: f32 = 24.0;

/// One more of a thing, by who it is on the stage. (A prop's first copy is
/// the place's own, if the place has one.)
pub fn thing_out(film: &mut Film, thing: &Thing) -> String {
    let copy = *film.things.copies.entry(thing.key.clone()).and_modify(|copy| *copy += 1).or_insert(0);
    let code = match &thing.kind {
        Kind::Item(item) if copy > 0 => format!("i{item}.{copy}"),
        Kind::Item(item) => format!("i{item}"),
        Kind::Prop(object) => format!("o{object}.{}", copy + 1),
        Kind::Drawn(file, cells) => {
            let at = film.pictures.iter().position(|known| known == file).unwrap_or_else(|| {
                film.pictures.push(file.clone());
                film.pictures.len() - 1
            });
            let code = if copy > 0 { format!("d{at}.{copy}") } else { format!("d{at}") };
            let ticks = film.rhythms.get(file).cloned().unwrap_or_default();
            // (No picture in it is read as one.)
            let cells = (*cells).max(1);
            film.on(&code, |who| emit::drawing(who, file.as_str(), cells, ticks.clone()));
            code
        }
    };
    // (Its own name again once the one that had it is gone: a rocket shot,
    // then another, both `rocket`.)
    let taken = |word: &str| film.things.stuff.iter().any(|stuff| stuff.word == word);
    let word = if copy > 0 && taken(&thing.key) { format!("{}{}", thing.key, copy + 1) } else { thing.key.clone() };
    film.thing_words.insert(word.clone());
    film.give_name(word.clone(), code.clone());
    film.things.stuff.push(Stuff { key: thing.key.clone(), word, code: code.clone(), owner: None, takeable: !matches!(thing.kind, Kind::Prop(_)) });
    code
}

/// What a thing is told once it is on the stage: still, until it is told
/// to play.
pub fn arrives(film: &mut Film, code: &str, thing: &Thing) {
    film.on(code, |who| emit::thing_name(who, thing.name.as_str()));
    if matches!(thing.kind, Kind::Prop(_)) {
        film.on(code, |who| emit::held_step(who, engine::AnimId(0), 0));
    } else {
        film.on(code, |who| emit::takeable(who, true));
    }
}

/// One more of a thing, shown once and gone: not one of the scene's things.
pub fn thing_apart(film: &mut Film, thing: &Thing) -> String {
    let code = thing_out(film, thing);
    film.things.stuff.retain(|stuff| stuff.code != code);
    code
}

/// One more of a thing, put where the words say and still there.
fn set_down(film: &mut Film, thing: &Thing, words: &[String], number: usize) -> String {
    let code = thing_out(film, thing);
    let spot = somewhere(film, words, number);
    // (On the stage first, at the point said: the free ground is sought from
    // it, as for someone; one not there yet is sought from the middle.)
    if let Spot::Written(at) | Spot::Word(at) | Spot::Exact(at) | Spot::Near(at) = &spot
        && let (Num::Lit(x), Num::Lit(y)) = (&at.x, &at.y)
    {
        let (x, y) = (*x, *y);
        film.on(&code, |who| emit::put(who, x, y, engine::Direction::Down));
    }
    // (A stepping stone, an egg on an islet: just where it is said, a part of
    // the place, no thing of anyone's.)
    if EXACTLY.contains(&thing.key.as_str()) {
        let at = found(film, &code, spot, &["exact"]);
        // (Where it goes, found as it plays.)
        film.put_point_at(&code, at);
        film.on(&code, emit::below);
        film.on(&code, |who| emit::shadow(who, false));
        return code;
    }
    let at = found(film, &code, spot, &[]);
    film.put_point_at(&code, at);
    arrives(film, &code, thing);
    code
}

/// A thing of the scene by its word, the last one if several.
fn placed(film: &mut Film, word: &str, line: &Line) -> Option<usize> {
    let key = plain(word);
    let at = film.things.stuff.iter().rposition(|stuff| stuff.word == key).or_else(|| film.things.stuff.iter().rposition(|stuff| stuff.key == key));
    if at.is_none() {
        film.wrong(line.number, format!("“{word}” is not in this scene: put object {word} ...  “{}”", line.text));
    }
    at
}

/// `thing letter image letter.png [images] [pace 40,2,3]`: a picture of the
/// film's own; with a rhythm, the 60ths of a second of each of its pictures.
pub fn own_picture(film: &mut Film, words: &[String], number: usize) {
    let (name, file) = (plain(&words[0]), words[2].clone());
    let cells: u16 = words.get(3).and_then(|cells| cells.parse().ok()).unwrap_or(1);
    if let Some(at) = words.iter().position(|word| plain(word) == "pace") {
        let ticks: Option<Vec<u32>> = words.get(at + 1).and_then(|ticks| ticks.split(',').map(|tick| tick.parse().ok().filter(|tick| *tick > 0)).collect());
        match ticks {
            Some(ticks) if ticks.len() == cells as usize => _ = film.rhythms.insert(file.clone(), ticks),
            Some(ticks) => film.wrong(number, format!("pace of {} pictures for {cells} pictures: a number a picture, 40,2,3...", ticks.len())),
            None => film.wrong(number, "pace wrongly said: numbers of frames (60 a second) separated by commas, pace 40,2,3".to_string()),
        }
    }
    let images = film.tables.project.folders("images");
    if !images.iter().any(|folder| folder.join(&file).exists()) && !std::path::Path::new(&film.tables.data).join("hand").join(&file).exists() {
        film.wrong(number, format!("no image {file} in library/images nor data/hand"));
    }
    film.own.insert(name, (file, cells));
}

/// `put object chest front_of a`: there from the scene's start.
fn put_on_stage(film: &mut Film, line: &Line) {
    let said = line.args.words("what").to_vec();
    let Some((used, thing)) = leading_thing(film, &said) else {
        thing_said(film, &said[..said.len().min(1)], line.number);
        return;
    };
    if film.staging.started {
        film.remark(
            line.number,
            format!("{} bursts in mid-scene: let it be there from the start, carried, thrown, or let it fall (thing {} falls)", thing.name, thing.key),
        );
    }
    set_down(film, &thing, &said[used..], line.number);
}

/// `shift letter to b in 2s`: it slides there, no one carrying it.
fn slid(film: &mut Film, line: &Line) {
    let Some(at) = placed(film, line.args.word("thing").unwrap_or_default(), line) else {
        return;
    };
    let code = film.things.stuff[at].code.clone();
    let words = line.args.words("where").to_vec();
    let frames = line.args.frames("d").unwrap_or(60);
    let spot = match words.as_slice() {
        [one] if film.actor(one).is_some() && spot(&plain(one), film.middle).is_none() => film.spot_by(one, 0.0, 6.0),
        _ => Some(somewhere(film, &words, line.number)),
    };
    let Some(spot) = spot else {
        return;
    };
    if film.things.stuff[at].owner.take().is_some() {
        film.on(&code, |who| emit::carry(who, None, true));
    }
    let to = found(film, &code, spot, &[]);
    // (Where it goes, found as it plays.)
    film.on_at(&code, to, |who, at| emit::go_point(who, at, frames as f32, true));
    film.wait_for(&code, line.wait);
}

/// `open chest`: its animation, up to its end; `close chest`: its first
/// picture. `switch_on`, `switch_off`: its flame.
fn worked(film: &mut Film, line: &Line) -> bool {
    let number = line.number;
    let said = line.args.word("thing").unwrap_or_default().to_string();
    let word = plain(&said);
    if !film.things.stuff.iter().any(|stuff| stuff.word == word || stuff.key == word) {
        return false;
    }
    let Some(at) = placed(film, &said, line) else {
        return true;
    };
    let code = film.things.stuff[at].code.clone();
    let drawn = code.starts_with('d');
    let object = code.strip_prefix('o').and_then(|code| code.split('.').next()).and_then(|object| object.parse::<u16>().ok());
    match line.verb.as_str() {
        "open" => {
            film.on(&code, |who| emit::anim(who, engine::AnimId(0), false, false));
            film.sound("click");
            film.wait_frames(object.and_then(|object| film.tables.prop_frames.get(&object).copied()).unwrap_or(30).min(120), line.wait);
        }
        "close" => {
            film.on(&code, |who| emit::held_step(who, engine::AnimId(0), 0));
            film.sound("click");
        }
        "switch_on" => {
            film.on(&code, |who| emit::opacity(who, 1.0, 0.0));
            film.on(&code, |who| emit::anim(who, engine::AnimId(0), true, false));
            film.on(&code, |who| emit::emitter(engine::ParticleKind::Embers, who, 40.0, Some(emit::colour(engine::Colour::Red)), Some(PUFF)));
            film.sound("click");
        }
        "switch_off" if drawn => {
            film.on(&code, |who| emit::pose(who, Pose::Idle));
            film.on(&code, |who| emit::emitter(engine::ParticleKind::Smoke, who, 40.0, Some(emit::colour(engine::Colour::White)), Some(PUFF)));
        }
        "switch_off" => {
            film.on(&code, |who| emit::emitter(engine::ParticleKind::Smoke, who, 40.0, Some(emit::colour(engine::Colour::White)), Some(PUFF)));
            film.on(&code, |who| emit::opacity(who, 0.0, 0.0));
        }
        _ => film.wrong(number, format!("wrongly said: {}", line.text)),
    }
    true
}

/// The things of the scene at hand, nothing at the next one.
#[derive(Default)]
pub struct ThingsState {
    /// The things, how many of each; the faces said.
    pub stuff: Vec<crate::film::Stuff>,
    pub copies: std::collections::HashMap<String, u32>,
}
