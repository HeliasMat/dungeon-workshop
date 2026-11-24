//! A film written `for ds`: what the DS can't play of it, told as it is
//! read (`E-DS-…` wrong, `R-DS-…` to look at). The words by the table
//! (their `OnDs`), the rest by the scenes the film made: who is there at
//! once, the places, the hero and partner, the view.

use std::collections::{BTreeMap, HashMap};

use engine::{Beat, BodyBeat, Diagnostic, FlowBeat, MotionBeat, Num, PlaceBeat, Point, Scene, ScreenBeat, Spot, ViewBeat, Who};

use crate::codes;
use crate::ds::{AT_ONCE, DECOR_LOSS, Export, Mode};
use crate::film::Film;

/// What the DS can't play of the film, added to its problems.
pub(crate) fn tell(film: &mut Film, mode: Mode, scenes: &[Scene]) {
    let source = film.source.clone();
    let lines: Vec<(usize, &str)> = line_starts(&source);
    words(film, &lines);
    if mode.boot {
        heroes(film, &lines);
    }
    for scene in scenes {
        place(film, scene, &lines);
        at_once(film, scene);
        view(film, scene, &lines);
    }
}

/// Each line of the text, where it starts.
fn line_starts(source: &str) -> Vec<(usize, &str)> {
    let mut at = 0;
    source
        .split('\n')
        .map(|line| {
            let start = at;
            at += line.len() + 1;
            (start, line)
        })
        .collect()
}

/// Where that word is in the line `number` (1 the first), in bytes of the text.
fn word_at(lines: &[(usize, &str)], number: usize, word: &str) -> Option<(usize, usize)> {
    let (start, line) = lines.get(number.checked_sub(1)?)?;
    let at = line.match_indices(word).find(|(at, _)| {
        let before = line[..*at].chars().next_back();
        let after = line[at + word.len()..].chars().next();
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_') && !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
    })?;
    Some((start + at.0, start + at.0 + word.len()))
}

/// The word a line of the text is of: as it was read, else the word a
/// block opens with (`beside`, `fight directed`, `while`).
fn word_of(film: &Film, number: usize, line: &str) -> Option<String> {
    if let Some(read) = film.read_as.get(&number) {
        return Some(read.word.clone());
    }
    let first = line.split_whitespace().next()?;
    film.lexicon.planned(first).is_some().then(|| first.to_string())
}

/// `E-DS-WORD` for a word the DS lacks, `R-DS-LOSS` once for each word that
/// exports with less (`R-DS-ZOOM` for a zoom).
fn words(film: &mut Film, lines: &[(usize, &str)]) {
    let mut losses: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (at, (_, line)) in lines.iter().enumerate() {
        let number = at + 1;
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(word) = word_of(film, number, trimmed) else { continue };
        let bytes = word_at(lines, number, &word);
        match film.lexicon.export_of(&word) {
            _ if word == "zoom" => {
                let message = "“zoom”: Sky's camera has no zoom, the view keeps its size on DS (left out of the export)";
                film.problems.push(Diagnostic::at(number, message).as_remark().on(bytes).coded(codes::DS_ZOOM));
            }
            Some(Export::Direct) => {}
            Some(Export::Close) => losses.entry(word).or_default().push(number),
            // (Not a word of the vocabulary: an engine's command, or a recipe of the film's own, checked by its words.)
            None => {}
            _ => film.problems.push(Diagnostic::at(number, film.lexicon.ds_refused(&word)).on(bytes).coded(codes::DS_WORD)),
        }
    }
    for (word, numbers) in losses {
        let more = match numbers.len() - 1 {
            0 => String::new(),
            1 => " (and on 1 more line)".to_string(),
            many => format!(" (and on {many} more lines)"),
        };
        let message =
            format!("“{word}” exports to DS with a loss: {}{more}", film.lexicon.on_ds(&word).and_then(|ds| ds.lost).unwrap_or("less than the film shows"));
        let bytes = word_at(lines, numbers[0], &word);
        film.problems.push(Diagnostic::at(numbers[0], message).as_remark().on(bytes).coded(codes::DS_LOSS));
    }
}

/// `E-DS-HERO`: the hero and the partner named in a film that plays as a
/// new game starts, before the player has chosen them.
fn heroes(film: &mut Film, lines: &[(usize, &str)]) {
    for (at, (_, line)) in lines.iter().enumerate() {
        let number = at + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        // (Who speaks, not what is said; a line's words, not those quoted.)
        let said = trimmed.find(':').filter(|colon| !trimmed[..*colon].contains('"'));
        let written = match said.map(|colon| (trimmed[..colon].trim(), &trimmed[colon + 1..])) {
            // (`scene: place who…`: those it names; `title:`, `moment:`: a name.)
            Some(("scene", cast)) => cast,
            Some(("title" | "moment", _)) => continue,
            Some((who, _)) => who,
            None => trimmed,
        };
        let unquoted: String = written.split('"').step_by(2).collect::<Vec<_>>().join(" ");
        let named = unquoted.split(|c: char| !(c.is_alphanumeric() || c == '_')).find(|word| matches!(*word, "hero" | "partner"));
        if let Some(who) = named {
            let message = format!(
                "“{who}”: on DS the film plays as a new game starts, before the player chose the {who}: give them a species of the film's own (“pip = piplup”), or write “for ds scene”"
            );
            film.problems.push(Diagnostic::at(number, message).on(word_at(lines, number, who)).coded(codes::DS_HERO));
        }
    }
}

/// The scene's first line of the film.
fn first_line(scene: &Scene) -> usize {
    scene
        .beats
        .iter()
        .find_map(|beat| match beat {
            Beat::Flow(FlowBeat::Line { number, .. }) if *number > 0 => usize::try_from(*number).ok(),
            _ => None,
        })
        .unwrap_or(1)
}

/// The line that opens the scene (`scene: …`), at or before its first line.
fn header_line(scene: &Scene, lines: &[(usize, &str)]) -> usize {
    let first = first_line(scene);
    (1..=first.min(lines.len())).rev().find(|number| lines[number - 1].1.trim_start().starts_with("scene:")).unwrap_or(first)
}

/// `E-DS-PLACE`: a place that is neither a map of Sky's nor a decor of the
/// Workshop's: the DS has nothing to show it. A decor exports as a map of
/// its own (`export-ds`, `patch_rom.py`), with a loss (`R-DS-LOSS`).
fn place(film: &mut Film, scene: &Scene, lines: &[(usize, &str)]) {
    let sky = &film.tables.sky_levels;
    // (No data: what the game has is not known.)
    if sky.is_empty() {
        return;
    }
    let shown = scene.beats.iter().find_map(|beat| match beat {
        Beat::Screen(ScreenBeat::Backdrop(level, ..)) => Some(*level),
        _ => None,
    });
    let level = shown.unwrap_or(scene.level);
    if sky.contains(&level) {
        return;
    }
    // (A decor of the Workshop's exports as a map of its own, with less.)
    if let Some(name) = film.tables.decors.iter().find(|(_, (own, ..))| *own == level).map(|(name, _)| name.clone()) {
        let message = format!("“{name}”, a place of your own, exports to DS with a loss: {}", DECOR_LOSS);
        film.problems.push(Diagnostic::at(header_line(scene, lines), message).as_remark().coded(codes::DS_LOSS));
        return;
    }
    let message = format!(
        "this place is no map of Explorers of Sky's nor a decor of yours (level {level}): the DS has nothing to show; choose a place of the game's or a decor"
    );
    film.problems.push(Diagnostic::at(header_line(scene, lines), message).coded(codes::DS_PLACE));
}

/// The film's Pokémon by their codes (`m25`, `m25.1`): those the game
/// counts at once (not the hero and the partner, kept apart).
fn pokemon(film: &Film) -> HashMap<String, String> {
    film.actors
        .iter()
        .filter(|(_, code)| matches!(Who::parse(code), Some(Who::Monster(_) | Who::Another { .. })))
        .map(|(name, code)| (code.clone(), name.clone()))
        .collect()
}

/// The words a line is of, by the beats' lines.
fn line_words(film: &Film, scene: &Scene) -> Vec<Option<String>> {
    let mut word = None;
    scene
        .beats
        .iter()
        .map(|beat| {
            if let Beat::Flow(FlowBeat::Line { number, .. }) = beat {
                word = usize::try_from(*number).ok().and_then(|line| film.read_as.get(&line)).map(|read| read.word.clone());
            }
            word.clone()
        })
        .collect()
}

/// One of the scene: its first beat, its last, whether that one hides it, whether it acts.
type Seen = (usize, usize, bool, bool);

/// A frame of more than that many is a crowd's, not of those it shows.
const FEW: usize = 3;

/// `E-DS-ACTORS`: more Pokémon at once than Sky shows (`AT_ONCE`). The
/// export makes room as `scene_player export-ds` does: who comes in later
/// takes the place of who has left, and the crowd's idle ones are left
/// out (those farthest from its middle); never one who speaks, acts or is
/// framed.
fn at_once(film: &mut Film, scene: &Scene) {
    let codes_of = pokemon(film);
    let words = line_words(film, scene);
    // Each one: its first beat, its last, whether that one hides it, whether it acts.
    let mut seen: BTreeMap<String, Seen> = BTreeMap::new();
    // (And the first line of the film it is named on.)
    let mut named_on: HashMap<String, usize> = HashMap::new();
    let mut number = 0;
    for (at, beat) in scene.beats.iter().enumerate() {
        if let Beat::Flow(FlowBeat::Line { number: line, .. }) = beat {
            number = usize::try_from(*line).unwrap_or(0);
        }
        // (A body's size, told for the framings of every scene at the film's
        // start: not someone there.)
        if matches!(beat, Beat::View(ViewBeat::Body { .. }) | Beat::Place(PlaceBeat::Footing { .. })) {
            continue;
        }
        let Some(fields) = beat.fields(&scenes::vocabulary::POKEMON) else { continue };
        let hides = matches!(beat, Beat::Place(PlaceBeat::Leaves { .. }) | Beat::Body(BodyBeat::Gone(_)) | Beat::Body(BodyBeat::Opacity { to: 0.0, .. }));
        // (Setting them there, not acting: put, a crowd, a frame of many; who speaks acts.)
        let placed = matches!(
            beat,
            Beat::Place(
                PlaceBeat::Crowd { .. }
                    | PlaceBeat::Gathers { .. }
                    | PlaceBeat::Spread { .. }
                    | PlaceBeat::Flies(_)
                    | PlaceBeat::Lands(_)
                    | PlaceBeat::Swims { .. }
                    | PlaceBeat::Ghosts(_)
                    | PlaceBeat::Hovers(_)
                    | PlaceBeat::Find { .. }
            ) | Beat::View(ViewBeat::Body { .. })
                | Beat::Body(BodyBeat::Put { .. })
        );
        let crowd_framed = matches!(beat, Beat::View(ViewBeat::Frame { who, .. }) if who.len() > FEW);
        let setting_line = matches!(words[at].as_deref(), None | Some("crowd" | "put" | "scene"));
        let setting = !matches!(beat, Beat::Speech(_)) && (placed || crowd_framed || setting_line);
        for field in fields.iter().filter(|field| codes_of.contains_key(*field)) {
            let one = seen.entry(field.clone()).or_insert((at, at, false, false));
            one.1 = at;
            one.2 = hides;
            one.3 |= !setting;
            if number > 0 {
                named_on.entry(field.clone()).or_insert(number);
            }
        }
    }
    let end = scene.beats.len();
    let span = |one: &Seen| (one.0, if one.2 { one.1 } else { end });
    // The beat most are there at once.
    let (peak, most) = (0..end.max(1))
        .map(|at| (at, seen.values().filter(|one| (span(one).0..=span(one).1).contains(&at)).count()))
        .max_by_key(|(at, count)| (*count, std::cmp::Reverse(*at)))
        .unwrap_or((0, 0));
    if most <= AT_ONCE {
        return;
    }
    let mut there: Vec<(&String, &Seen)> = seen.iter().filter(|(_, one)| (span(one).0..=span(one).1).contains(&peak)).collect();
    // (In the order they come in, then of their names: `pidgey2` before `pidgey10`.)
    there.sort_by_key(|(code, one)| (one.0, codes_of[*code].len(), codes_of[*code].clone()));
    let idle: Vec<&str> = there.iter().filter(|(_, one)| !one.3).map(|(code, _)| codes_of[*code].as_str()).collect();
    let over = most - AT_ONCE;
    let left = over.min(idle.len());
    let mut message = format!("{most} Pokémon at once in this scene, Sky shows {AT_ONCE}");
    if left > 0 {
        let among = match idle.as_slice() {
            [first, second, .., last] if idle.len() > 4 => format!("{first}, {second} … {last}"),
            all => all.join(", "),
        };
        message.push_str(&format!(
            ": the export leaves out {left} of the {} idle ones ({among}), those farthest from the crowd's middle as it stands",
            idle.len()
        ));
    }
    if over > left {
        message.push_str(&format!(
            "; still {} too many, each speaking, acting or framed: the game won't show them. Let some leave first (“exit”, “vanish”)",
            over - left
        ));
    }
    // (Told on the line that brought in the last of them.)
    let number = there.iter().filter_map(|(code, _)| named_on.get(*code).copied()).max().unwrap_or_else(|| first_line(scene));
    film.problems.push(Diagnostic::at(number, message).coded(codes::DS_ACTORS));
}

/// Where a point is, as known before the film plays.
fn point_of(places: &HashMap<String, (f32, f32)>, point: &Point) -> Option<(f32, f32)> {
    let num = |num: &Num| match num {
        Num::Lit(value) => Some(*value),
        Num::Var(name) => {
            let (place, axis) = name.rsplit_once('_')?;
            let (x, y) = *places.get(place)?;
            match axis {
                "x" => Some(x),
                "y" => Some(y),
                _ => None,
            }
        }
    };
    Some((num(&point.x)?, num(&point.y)?))
}

/// `R-DS-ZOOM`: a frame of those wider or taller than the DS's screen,
/// where the view would step back: Sky has no zoom, the export pans
/// across them.
fn view(film: &mut Film, scene: &Scene, lines: &[(usize, &str)]) {
    let mut places: HashMap<String, (f32, f32)> = HashMap::new();
    let mut at: HashMap<Who, (f32, f32)> = HashMap::new();
    let mut number = 0;
    for beat in &scene.beats {
        match beat {
            Beat::Flow(FlowBeat::Line { number: line, .. }) => number = usize::try_from(*line).unwrap_or(0),
            Beat::Place(PlaceBeat::Find { var, spot: Spot::Written(point) | Spot::Word(point) | Spot::Exact(point) | Spot::Near(point), .. }) => {
                if let Some(point) = point_of(&places, point) {
                    places.insert(var.clone(), point);
                }
            }
            Beat::Body(BodyBeat::Put { who, at: point, .. }) | Beat::Motion(MotionBeat::Go { who, at: point, .. }) => {
                if let Some(point) = point_of(&places, point) {
                    at.insert(*who, point);
                }
            }
            Beat::Place(PlaceBeat::Arrives { who, to, .. }) => {
                if let Some(point) = point_of(&places, to) {
                    at.insert(*who, point);
                }
            }
            Beat::View(ViewBeat::Frame { who, at: points, .. }) if number > 0 => {
                let mut shown: Vec<(f32, f32)> = who.iter().filter_map(|one| at.get(one).copied()).collect();
                shown.extend(points.iter().map(|point| (point.x, point.y)));
                if crate::ds::wider_than_the_screen(&shown) {
                    let message = "“frame”: they don't fit the DS's screen and Sky has no zoom to step back: the export pans across them";
                    film.problems.push(Diagnostic::at(number, message).as_remark().on(word_at(lines, number, "frame")).coded(codes::DS_ZOOM));
                }
            }
            _ => {}
        }
    }
}
