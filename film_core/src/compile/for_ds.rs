//! A film written `for ds`: what the DS can't play of it, told as it is
//! read (`E-DS-…` wrong, `R-DS-…` to look at). The words by the table
//! (their `OnDs`), the rest by the scenes the film made: who is there at
//! once, the places, the hero and partner, the view.

use std::collections::{BTreeMap, HashMap};

use engine::{Beat, BodyBeat, Course, Diagnostic, FlowBeat, MotionBeat, Num, PlaceBeat, Play, Point, Scene, ScreenBeat, SpeechBeat, Spot, ViewBeat, Who};

use crate::codes;
use crate::ds::{AT_ONCE, DECOR_LOSS, Drawn, Export, Mode, drawn_on_ds, group_name, pose_on_ds, room};
use crate::film::Film;

/// What the DS can't play of the film, added to its problems.
pub(crate) fn tell(film: &mut Film, mode: Mode, scenes: &[Scene]) {
    let source = film.source.clone();
    let lines: Vec<(usize, &str)> = line_starts(&source);
    words(film, &lines);
    if mode.boot {
        heroes(film, &lines);
    }
    species(film, &lines);
    let mut poses_told = BTreeMap::new();
    for scene in scenes {
        place(film, scene, &lines);
        at_once(film, scene);
        sprites(film, scene, &lines);
        view(film, scene, &lines);
        poses(film, scene, &mut poses_told);
    }
    for ((who, asked, shown), numbers) in poses_told {
        let message = format!("{who}'s Sky sprite has no {asked}: {shown} instead on DS{}", more_lines(numbers.len()));
        let bytes = word_at(&lines, numbers[0], &who);
        film.problems.push(Diagnostic::at(numbers[0], message).as_remark().on(bytes).coded(codes::DS_LOSS));
    }
}

/// ` (and on 2 more lines)`, after what is told of the first of so many.
fn more_lines(count: usize) -> String {
    match count.saturating_sub(1) {
        0 => String::new(),
        1 => " (and on 1 more line)".to_string(),
        many => format!(" (and on {many} more lines)"),
    }
}

/// Whether a line is said in a bubble (`pichu (bubble, happy): …`, or
/// over another, `(top)`): the DS says it in its box.
fn in_a_bubble(line: &str) -> bool {
    let Some((head, _)) = line.split_once(':') else { return false };
    let Some(brackets) = head.trim_end().strip_suffix(')').and_then(|head| head.split_once('(')).map(|(_, inside)| inside) else { return false };
    brackets.split(',').map(crate::words::plain).any(|word| word == "bubble" || word == "top")
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
    // (The film's own words, `define greet`: their lines are its words'.)
    let source = film.source.clone();
    let defined: Vec<&str> = source.lines().filter_map(|line| line.trim().strip_prefix("define ")?.split_whitespace().next()).collect();
    for (at, (_, line)) in lines.iter().enumerate() {
        let number = at + 1;
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if in_a_bubble(trimmed) {
            losses.entry(BUBBLE_LINE.to_string()).or_default().push(number);
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
            // (A recipe of the film's own: checked by its words.)
            None if defined.contains(&word.as_str()) => {}
            // (An engine's command on an item, `trackpath i109 ...`: its object's on DS.)
            None if on_an_item(trimmed) => {}
            // (Not a word of the vocabulary: an engine's command, which the export leaves out.)
            None => {
                let what = if crate::words::engine_word(&word) { "an engine's command" } else { "not a word of the vocabulary" };
                let message = format!("“{word}”, {what}: the DS export has nothing for it and leaves it out");
                film.problems.push(Diagnostic::at(number, message).as_remark().on(bytes).coded(codes::DS_LEFT));
            }
            _ => film.problems.push(Diagnostic::at(number, film.lexicon.ds_refused(&word)).on(bytes).coded(codes::DS_WORD)),
        }
    }
    for (word, numbers) in losses {
        let more = more_lines(numbers.len());
        let lost = film.lexicon.on_ds(&word).and_then(|ds| ds.lost).unwrap_or("less than the film shows");
        let message = if word == BUBBLE_LINE {
            format!("a line said in a bubble exports to DS as a line of Sky's box, closing by itself: {lost}{more}")
        } else {
            format!("“{word}” exports to DS with a loss: {lost}{more}")
        };
        let bytes =
            if word == BUBBLE_LINE { word_at(lines, numbers[0], "bubble").or(word_at(lines, numbers[0], "top")) } else { word_at(lines, numbers[0], &word) };
        film.problems.push(Diagnostic::at(numbers[0], message).as_remark().on(bytes).coded(codes::DS_LOSS));
    }
}

/// Whether an engine's command is on an item or a picture of the film's
/// own (`trackpath i109 ...`, `gone i109.2`): the DS export makes it an
/// object of the scene's and plays it.
fn on_an_item(line: &str) -> bool {
    let mut words = line.split_whitespace();
    let command = words.next().unwrap_or_default();
    ["trackpath", "untrack", "waittrack", "gone", "shadow"].contains(&command)
        && words.next().and_then(Who::parse).is_some_and(|who| matches!(who, Who::Item { .. } | Who::Drawn { .. }))
}

/// The vocabulary's entry of a line said in a bubble (`film_core::vocabulary::OTHERS`).
const BUBBLE_LINE: &str = "(bubble)";

/// `R-DS-SPECIES`: a Pokémon or a form the ROM has no sprite for (one
/// SpriteCollab adds, numbered past Sky's, with no sheets in the data to put
/// in the ROM): drawn as its base species, or as a stand-in. One whose
/// sheets the data has is put in the ROM by `patch_rom.py`: nothing told.
/// Told on the first line that names it.
fn species(film: &mut Film, lines: &[(usize, &str)]) {
    let mut actors: Vec<(String, String)> = film.actors.iter().map(|(name, code)| (name.clone(), code.clone())).collect();
    actors.sort();
    for (name, code) in actors {
        let Some(Who::Monster(species) | Who::Another { species, .. }) = Who::parse(&code) else { continue };
        let drawn = drawn_on_ds(&film.tables.data, species);
        let shown = match drawn {
            Drawn::Own | Drawn::Added(_) => continue,
            Drawn::Base(base) => format!("a form SpriteCollab adds: drawn as {}, its base species", monster_name(film, base)),
            Drawn::StandIn => {
                format!("a Pokémon Sky doesn't have: drawn as a stand-in, {} “?”, its name said in its lines", monster_name(film, crate::ds::STAND_IN))
            }
        };
        let first = (1..=lines.len()).find_map(|number| {
            let trimmed = lines[number - 1].1.trim_start();
            (!trimmed.starts_with('#')).then(|| word_at(lines, number, &name).map(|bytes| (number, bytes))).flatten()
        });
        let Some((number, bytes)) = first else { continue };
        let message = format!("“{name}” (species {species}) on DS: {shown}, the data having no sheets of its own to put in the ROM");
        film.problems.push(Diagnostic::at(number, message).as_remark().on(Some(bytes)).coded(codes::DS_SPECIES));
    }
}

/// A species' name, as the game's table has it.
fn monster_name(film: &Film, species: u16) -> String {
    let code = species.to_string();
    film.tables.monsters.values().find(|named| named.0 == code).map_or_else(|| format!("species {species}"), |named| named.1.clone())
}

/// `R-DS-LOSS`: a pose a species' Sky sprite lacks (`pikachu pose nod` on
/// a sprite with no Nod): the nearest every sprite has instead, as the
/// export plays it; told once a Pokémon and pose.
fn poses(film: &Film, scene: &Scene, told: &mut BTreeMap<(String, &'static str, &'static str), Vec<usize>>) {
    let names: HashMap<&String, &String> = film.actors.iter().map(|(name, code)| (code, name)).collect();
    let mut number = 0;
    for beat in &scene.beats {
        if let Beat::Flow(FlowBeat::Line { number: line, .. }) = beat {
            number = usize::try_from(*line).unwrap_or(0);
        }
        // (Hops are Sky's own jumps.)
        let Beat::Body(BodyBeat::Anim { who, anim, play }) = beat else { continue };
        let (Some(species), true) = (
            match who {
                Who::Monster(species) | Who::Another { species, .. } => Some(*species),
                _ => None,
            },
            number > 0 && anim.0 != 10,
        ) else {
            continue;
        };
        let species = drawn_on_ds(&film.tables.data, species).species(species);
        let Some(shown) = pose_on_ds(&film.tables.data, species, anim.0, matches!(play, Play::Loop)).and_then(|pose| pose.instead) else { continue };
        let Some(name) = names.get(&who.to_string()) else { continue };
        let asked = group_name(anim.0);
        let lines = told.entry(((*name).clone(), asked, shown)).or_default();
        if !lines.contains(&number) {
            lines.push(number);
        }
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

/// `R-DS-SPRITES`: more species in a scene than their sprites have room
/// for on DS (`room`: each species' sprite loaded once as the scene starts,
/// in Sky's arena and, GroundSpriteRoom on, the hook's; past it a magenta
/// square). Told with the species, their bytes and the room's, what the
/// export does (it splits the scene at a moment when both parts fit, else
/// leaves out the least needed) and how to keep them all.
fn sprites(film: &mut Film, scene: &Scene, lines: &[(usize, &str)]) {
    let codes_of = pokemon(film);
    let data = film.tables.data.clone();
    let moments: Vec<usize> = lines.iter().enumerate().filter(|(_, (_, line))| line.trim_start().starts_with("moment:")).map(|(at, _)| at + 1).collect();
    // Each species as the DS shows it, and the film's names of its Pokémon.
    let mut names: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    let usage = room::usage_of(&scene.beats, &moments, |field| {
        let name = codes_of.get(field)?;
        let (Who::Monster(species) | Who::Another { species, .. }) = Who::parse(field)? else { return None };
        let shown = drawn_on_ds(&data, species).species(species);
        let named = names.entry(shown).or_default();
        if !named.contains(name) {
            named.push(name.clone());
        }
        Some(shown)
    });
    let all = usage.all();
    let room = room::Room::current();
    if room::fits(&data, &room, &all, 0) {
        return;
    }
    let named = |kinds: &[u16]| -> String {
        kinds.iter().map(|kind| names.get(kind).map_or_else(|| kind.to_string(), |names| names.join(", "))).collect::<Vec<_>>().join(", ")
    };
    let what = match room::split_at(&data, &room, &usage, &[], 0) {
        Some(at) => {
            let line = usage.lines[at];
            let moment = lines.get(line.wrapping_sub(1)).and_then(|(_, text)| text.trim_start().strip_prefix("moment:")).unwrap_or("").trim();
            let (first, second) = (usage.part(0..at), usage.part(at..usage.moments.len()));
            format!(
                "the export splits it into two DS scenes at the moment “{moment}” (line {line}): {} species, about {} KB, then {}, about {} KB",
                first.len(),
                room::kb(room::scene_bytes(&data, &first, 0)),
                second.len(),
                room::kb(room::scene_bytes(&data, &second, 0))
            )
        }
        None => {
            let mut least: Vec<u16> = all.clone();
            least.sort_by_key(|kind| usage.uses.get(kind).copied().unwrap_or(0));
            let left = room::leave_out(&data, &room, &all, &least, 0);
            format!(
                "no moment splits it so that both parts fit: the export leaves out the least needed, about {} ({} KB)",
                named(&left),
                room::kb(room::scene_bytes(&data, &left, 0))
            )
        }
    };
    let message = format!(
        "{} species in this scene ({}): their sprites take about {} KB, {}; past it they are drawn as magenta squares. {what}. To keep them all as they are: split the scene, have some leave in one scene and the others arrive in the next, or use fewer new species (Sky's own take about 40 KB each, SpriteCollab's 60-77 KB)",
        all.len(),
        named(&all),
        room::kb(room::scene_bytes(&data, &all, 0)),
        room.said(),
    );
    film.problems.push(Diagnostic::at(header_line(scene, lines), message).as_remark().coded(codes::DS_SPRITES));
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

/// One of the scene: its first beat, its last, whether that one hides it,
/// whether it acts, moves, speaks (or is framed among a few).
#[derive(Clone, Copy)]
struct Seen {
    first: usize,
    last: usize,
    hides: bool,
    acts: bool,
    moves: bool,
    speaks: bool,
}

impl Seen {
    /// What the export keeps first: who speaks, then who moves, then who acts.
    fn rank(&self) -> u8 {
        if self.speaks {
            3
        } else if self.moves {
            2
        } else {
            u8::from(self.acts)
        }
    }
}

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
        let speaks = matches!(beat, Beat::Course(Course::Say { .. }) | Beat::Speech(SpeechBeat::Bubble { .. }))
            || matches!(beat, Beat::View(ViewBeat::Frame { who, .. }) if who.len() <= FEW);
        let moves = matches!(beat, Beat::Motion(_)) && !setting;
        for field in fields.iter().filter(|field| codes_of.contains_key(*field)) {
            let one = seen.entry(field.clone()).or_insert(Seen { first: at, last: at, hides: false, acts: false, moves: false, speaks: false });
            one.last = at;
            one.hides = hides;
            one.acts |= !setting;
            one.moves |= moves;
            one.speaks |= speaks;
            if number > 0 {
                named_on.entry(field.clone()).or_insert(number);
            }
        }
    }
    let end = scene.beats.len();
    let span = |one: &Seen| (one.first, if one.hides { one.last } else { end });
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
    there.sort_by_key(|(code, one)| (one.first, codes_of[*code].len(), codes_of[*code].clone()));
    let over = most - AT_ONCE;
    let mut message = format!("{most} Pokémon at once in this scene, Sky shows {AT_ONCE}");
    // (The idle first, then those only acting, then those moving; never one who speaks.)
    let mut left = 0;
    for (rank, what) in [(0, "idle"), (1, "only acting where they stand"), (2, "moving, never speaking")] {
        let these: Vec<&str> = there.iter().filter(|(_, one)| one.rank() == rank).map(|(code, _)| codes_of[*code].as_str()).collect();
        let taken = (over - left).min(these.len());
        if taken == 0 {
            continue;
        }
        let among = match these.as_slice() {
            [first, second, .., last] if these.len() > 4 => format!("{first}, {second} … {last}"),
            all => all.join(", "),
        };
        message.push_str(&format!(
            "{} the export leaves out {taken} of the {} {what} ({among}), those farthest from the crowd's middle as it stands",
            if left == 0 { ":" } else { ";" },
            these.len()
        ));
        left += taken;
    }
    if over > left {
        message
            .push_str(&format!("; still {} too many, each speaking or framed: the game won't show them. Let some leave first (“exit”, “vanish”)", over - left));
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
