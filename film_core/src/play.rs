//! A line played: by the groups whose form of its word fits it, the
//! narrower first; what none plays the engine's own command, or what is
//! wrong with it. What a line of the film's own text is told as written
//! (an old word, the old order, a family's word not taken in) is told here
//! too, before it plays (`text::written`).

use crate::film::Film;
use crate::ir::{Instr, Shape};
use crate::verbs::{Group, Lexicon, Line, fitting_forms, internal, spot_named};
use crate::words::{engine_word, plain};

/// A line of a scene other than a line said, as the engine's commands.
/// Ending with `&`: the film goes on without waiting for it.
impl Film {
    /// A word played for another, by the tree: that word and its words, the
    /// library's recipes played out in it (a gesture of
    /// `film/recipes/gestures.film`: `evolution a m133` for `evolve`).
    pub fn call(&mut self, number: usize, verb: &str, words: &[&str]) {
        let words: Vec<String> = words.iter().map(|word| word.to_string()).collect();
        for instr in crate::text::recipes::unfold_word(self, Instr::word(number, verb, &words, true)) {
            command(self, &instr);
        }
    }
}

pub fn command(film: &mut Film, item: &Instr) {
    // (Its words where they are written, for a problem to say which.)
    let here = (item.number, film.written_words(item));
    let kept = std::mem::replace(&mut film.here, here);
    let before = film.problems.len();
    command_here(film, item);
    film.here = kept;
    // (A line a recipe wrote: what is wrong with it says which recipe line.)
    if let Some(from) = &item.from.0 {
        for problem in film.problems[before..].iter_mut().filter(|problem| !problem.is_remark()) {
            problem.message = format!("{} ({})", problem.message, from.told());
        }
    }
}

/// The forms of its word a line fits, by group, with what each read.
type Fitting = Vec<(&'static str, crate::grammar::Bound)>;

fn command_here(film: &mut Film, item: &Instr) {
    // (A beat the language made itself, typed: a test, a number, a loop, a wait.)
    if let Shape::Beat(beat) = &item.shape {
        film.language_beat(beat.clone());
        return;
    }
    // (Fields for the engine: as they are.)
    if let Shape::Engine(fields) = &item.shape {
        if !fields.is_empty() {
            // (Written for the engine as it is: each word typed by the film's names.)
            film.engine_command(&fields[0], &fields[1..], None);
        }
        return;
    }
    let text = item.text.trim_end_matches([' ', '&']).to_string();
    // (Its first word as written: the engine's command, by the name the line gives it.)
    let first = text.split_whitespace().next().unwrap_or_default().to_string();
    let Some(line) = line_of(item, text) else { return };
    // (`end`: what was opened last is closed, as it was told to be.)
    if line.verb == "end" && line.rest.is_empty() {
        match film.timing.opened.pop() {
            Some(close) => close(film, &line),
            None => film.wrong(line.number, "“end” closes nothing here: no block, no image shot, dimensional_scream, memory, choice or fight open".to_string()),
        }
        return;
    }
    if item.from.0.is_none() && item.span.is_some() && refused_as_written(film, &line, &first) {
        return;
    }
    // (The line must fit one of its word's forms; the groups
    // whose form fits play it first; one that only the engine's form fits
    // goes to the engine.)
    let (fitting, why) = fitting_forms(film, &line);
    told_as_read(film, item, &line, &fitting);
    let mut line = line;
    let (fitting, why) = spots_as_points(film, &mut line, fitting, why);
    let fitting_groups: Vec<&str> = fitting.iter().map(|(group, _)| *group).collect();
    if fitting_groups.is_empty() && !why.is_empty() {
        film.wrong(line.number, format!("“{}”: {}", line.text, why.join("; ")));
        return;
    }
    if played_by_a_group(film, &mut line, &fitting, &fitting_groups) {
        return;
    }
    if !engine_word(&first) && !engine_word(&line.verb) {
        film.unknown(line.number, "unknown word", &first, film.lexicon.every_word().map(|(_, word, ..)| word), "film/FILMS.md");
    } else if line.verb == "else" && line.rest.is_empty() {
        film.beat(crate::emit::otherwise());
    } else if fitting_groups.is_empty() {
        // (Only the words the language has a form for: `engine.rs`.)
        film.wrong(
            line.number,
            format!("“{}”: a command of the engine the language does not know (its form in film_core/src/verbs/engine.rs, or `>` to debug)", line.text),
        );
    } else {
        engine_plays(film, &line, &first, &fitting);
    }
}

/// A word: its word and its words as read once; a line anchored or a
/// fight's moment: by its first word, what it is kept with it. None for a
/// blank line.
fn line_of(item: &Instr, text: String) -> Option<Line> {
    let (verb, rest, wait) = match &item.shape {
        Shape::Word { verb, words, wait } => (verb.clone(), words.clone(), *wait),
        Shape::Blank => return None,
        _ => {
            let words: Vec<String> = text.split_whitespace().map(str::to_string).collect();
            (plain(&words[0]), words[1..].to_vec(), !item.text.ends_with(" &"))
        }
    };
    Some(Line {
        verb,
        rest,
        text,
        number: item.number,
        wait,
        block: item.inner.clone().unwrap_or_default(),
        args: Default::default(),
        shape: item.shape.clone(),
    })
}

/// A word of the old vocabulary, or the library's own, in the film's own
/// text: refused, the form now its fix.
fn refused_as_written(film: &mut Film, line: &Line, first: &str) -> bool {
    let written = film.here.1.clone();
    // (Who first, `pikachu nap`: the old word second.)
    let second = written.get(1).map(|(said, _)| plain(said)).filter(|_| !film.lexicon.is_word(&line.verb, &film.source));
    let old = std::iter::once(line.verb.clone()).chain(second).find(|word| film.lexicon.replaced(word).is_some()).unwrap_or_default();
    if crate::text::written::told_replaced(film, line.number, &written, &old) {
        return true;
    }
    if internal(&line.verb) && written.first().is_some_and(|(said, _)| plain(said) == line.verb) {
        film.unknown(line.number, "unknown word", first, film.lexicon.every_word().map(|(_, word, ..)| word), "film/FILMS.md");
        return true;
    }
    false
}

/// What a line of the text tells as written: the library its word asks
/// for, who comes first, and what it was read as.
fn told_as_read(film: &mut Film, item: &Instr, line: &Line, fitting: &Fitting) {
    // (A word of an optional family: its library taken in, `uses fishing`.)
    if item.from.0.is_none() && item.span.is_some() && !fitting.is_empty() {
        crate::text::written::asks_its_library(film, line.number, &line.verb);
    }
    // (Written the word first, said of someone: who first now, `pikachu walk to b`.)
    if item.from.0.is_none()
        && let Some((group, _)) = fitting.first()
        && crate::text::written::said_of_someone(film, group, &line.verb, &line.rest)
    {
        let written = film.here.1.clone();
        crate::text::written::told_who_first(film, line.number, &written, &line.verb);
    }
    // (What a line of the text was read as, by the form that fits first, a
    // spot still by its name: for who edits it by its fields. Not those a
    // recipe, a scene or a word writes with its number.)
    let as_in_the_text = item.span.is_some();
    if let Some((group, bound)) = fitting.first().filter(|_| as_in_the_text) {
        let read = crate::film::ReadAs { word: line.verb.clone(), group, values: bound.values.clone() };
        film.read_as.entry(line.number).or_insert(read);
    }
}

/// A place said in numbers where the place has a name for it: its name
/// reads better, and holds if the place changes (remarked). A place said by
/// its name, a spot of the place's or of the film's: its point, for the
/// words that reckon with numbers, `crystal a b`; the forms fitted again.
fn spots_as_points(film: &mut Film, line: &mut Line, fitting: Fitting, why: Vec<String>) -> (Fitting, Vec<String>) {
    let places: Vec<(usize, usize)> = fitting.first().map(|(_, bound)| bound.places().collect()).unwrap_or_default();
    let mut said: Vec<(i32, i32)> = places
        .iter()
        .filter(|(_, long)| *long == 2)
        .filter_map(|(at, _)| Some((line.rest[*at].parse::<f32>().ok()? as i32, line.rest[*at + 1].parse::<f32>().ok()? as i32)))
        .collect();
    said.extend(film.points_said.remove(&line.number).unwrap_or_default());
    let mut spliced = false;
    for (at, _) in places.iter().filter(|(_, long)| *long == 1).rev() {
        if let Some((x, y)) = spot_named(film, &line.rest[*at]) {
            line.rest.splice(*at..*at + 1, [x.to_string(), y.to_string()]);
            spliced = true;
        }
    }
    let (mut fitting, mut why) = if spliced { fitting_forms(film, line) } else { (fitting, why) };
    // (No form fits: a named spot where a form wants numbers, `release a b
    // offshore`, is its point too.)
    if fitting.is_empty() {
        let mut named = line.as_verb(&line.verb);
        let mut any = false;
        for at in (0..named.rest.len()).rev() {
            if film.actor(&named.rest[at]).is_none()
                && let Some((x, y)) = spot_named(film, &named.rest[at])
            {
                named.rest.splice(at..at + 1, [x.to_string(), y.to_string()]);
                any = true;
            }
        }
        if any {
            let (found, _) = fitting_forms(film, &named);
            if !found.is_empty() {
                (line.rest, fitting, why) = (named.rest, found, Vec::new());
            }
        }
    }
    for (x, y) in said {
        if let Some((name, at_x, at_y)) = named_near(film, x, y) {
            film.remark(line.number, format!("{x} {y} is the place's spot “{name}” ({at_x}, {at_y}): write {name}"));
        }
    }
    (fitting, why)
}

/// The groups that have the word, those whose form fits first, each given
/// what its own form read: true once one plays it. When the engine's form
/// fits too, the groups whose form does not are not asked: what the fitting
/// ones let go is the engine's, `anim krabby DeepBreath` a game animation
/// once no character has it.
fn played_by_a_group(film: &mut Film, line: &mut Line, fitting: &Fitting, fitting_groups: &[&str]) -> bool {
    let verb = line.verb.clone();
    let has = |group: &Group| group.words.iter().any(|word| word.word == verb);
    let by_the_engine_only = !fitting_groups.is_empty() && fitting_groups.iter().all(|group| crate::verbs::ENGINE_GROUPS.contains(group));
    let engine_fits = fitting_groups.iter().any(|group| crate::verbs::ENGINE_GROUPS.contains(group));
    let ordered = film
        .lexicon
        .groups()
        .iter()
        .filter(|group| fitting_groups.contains(&group.name))
        .chain(film.lexicon.groups().iter().filter(|group| !engine_fits && !fitting_groups.contains(&group.name)));
    for group in ordered.into_iter().filter(|group| !by_the_engine_only && has(group)) {
        line.args = fitting.iter().find(|(from, _)| *from == group.name).map(|(_, bound)| bound.clone()).unwrap_or_default();
        if group.plays(film, line) {
            return true;
        }
    }
    false
}

/// The engine's own command: its beat, and how long the film waits for it,
/// as the registry says of the word (its `d`, or until its `who` gets
/// there), unless `&`.
fn engine_plays(film: &mut Film, line: &Line, first: &str, fitting: &Fitting) {
    let (time, bound) = engine_time(film.lexicon, &line.verb, fitting).unwrap_or((crate::time::WordTime::Instant, Default::default()));
    // (A time without its unit: frames, as written before.)
    let said_time = bound.values.iter().filter(|value| value.kind == crate::grammar::Kind::Time).filter_map(|value| value.words.first());
    if let Some(said) = said_time.clone().find(|said| crate::words::digits(said) && *said != "0") {
        film.remark(line.number, format!("“{}”: a duration without its unit, in frames; write {}", line.text, crate::words::in_seconds_said(said)));
    }
    let lasting = bound.word("d").map(|said| in_frames(film, said)).and_then(|frames| frames.parse::<i32>().ok()).filter(|frames| *frames > 0);
    let lasts = lasting.filter(|_| time == crate::time::WordTime::Lasts);
    // (Instant, but lasting so long beside the film: its line's end.)
    if let Some(frames) = lasting.filter(|_| time == crate::time::WordTime::Instant) {
        film.plan.handle(crate::time::Length::Known(frames));
    }
    let walker = (time == crate::time::WordTime::Until).then(|| bound.word("who").or(line.rest.first().map(String::as_str)).map(plain)).flatten();
    // (Its words typed by what its form read them as, made into its beat.)
    film.engine_command(first, &line.rest, Some(&bound));
    // (A move waits for its end, unless the line ends with `&`; a sound,
    // sparks, a shake, a face go on by themselves.)
    if let Some(frames) = lasts {
        film.wait_frames(frames, line.wait);
    }
    if let Some(who) = walker {
        film.wait_for(&who, line.wait);
    }
}

/// How near a point said in numbers must be to a place's named spot to be
/// told to say its name.
const NAMED_NEAR: i32 = 12;

/// The named spot of the place at hand by that point, if one: the place's
/// own (the letters of a room's plan, its ways in), the film's (`spot`).
fn named_near(film: &Film, x: i32, y: i32) -> Option<(String, i32, i32)> {
    let place = film.tables.entrances.get(&film.level).into_iter().flatten().cloned();
    let film_own = film.lasting_spots.iter().cloned();
    place
        .chain(film_own)
        .map(|(name, at_x, at_y)| (name, at_x, at_y, (at_x - x).pow(2) + (at_y - y).pow(2)))
        .filter(|(.., far)| *far <= NAMED_NEAR * NAMED_NEAR)
        .min_by_key(|(.., far)| *far)
        .map(|(name, at_x, at_y, _)| (name, at_x, at_y))
}

/// The time of the engine's word a line goes to, as the registry has it
/// (`engine.rs`, the commands' own table), and what its form read there.
fn engine_time(
    lexicon: &'static Lexicon,
    verb: &str,
    fitting: &[(&'static str, crate::grammar::Bound)],
) -> Option<(crate::time::WordTime, crate::grammar::Bound)> {
    lexicon.engine_groups().into_iter().find_map(|group| {
        let word = group.words.iter().find(|word| word.word == verb)?;
        let bound = fitting.iter().find(|(from, _)| *from == group.name)?.1.clone();
        Some((word.timing(), bound))
    })
}

/// A time in seconds (`1.5s`) or in frames (`6f`), as frames (the film's
/// pace, `pace`, is only for `wait`); any other word as it is.
fn in_frames(_film: &Film, word: &str) -> String {
    match crate::words::frames_of(word) {
        Some(frames) if word.ends_with('s') || word.ends_with('f') => frames.to_string(),
        _ => word.to_string(),
    }
}
