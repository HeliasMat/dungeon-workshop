//! A scene, in its order: its place opened (ground, ways, spots, what stands
//! there), its cast spread, what sets it played before it shows, the fade in,
//! its lines, what goes on meanwhile seen to its end, the fade out.

use std::collections::HashSet;

use engine::Spot;
use glam::Vec2;

use super::scenes::SceneOf;
use crate::emit;
use crate::film::{Film, Sortie};
use crate::ir::{Instr, Shape};
use crate::names::species;
use crate::tables::MAP_LEVEL;
use crate::tables::Movement;
use crate::text::cast::{placed_by_a_line, puts_someone, who_acts};
use crate::text::first_word;
use crate::verbs::command;
use crate::verbs::timing::wait_meanwhile;
use crate::words::{SETTING, plain, spot};

/// The frames between a scene's last line and its fade out.
const LAST_PAUSE: i32 = 40;
/// The frames of a fade, of a wipe.
const FADE: i32 = 30;
const WIPE: i32 = 24;

/// The song a line of a scene's setting asks for, written plainly.
fn song_asked(item: &Instr) -> Option<String> {
    match (item.verb(), item.words()) {
        (Some("music"), [song]) => Some(song.clone()),
        _ => None,
    }
}

/// Whether some line, in a block or not, says what the music does.
fn asks_a_song(lines: &[Instr]) -> bool {
    lines.iter().any(|item| {
        matches!(item.verb(), Some("music" | "musicfrom" | "fademusic"))
            || (item.verb() == Some("fade") && item.words().first().is_some_and(|word| word == "music"))
            || item.inner.as_deref().is_some_and(asks_a_song)
    })
}

/// A scene that says nothing of the music, in another place than the one
/// before: its place's own song, as the game plays it there (the same place
/// goes on with what plays).
pub(super) fn place_songs(film: &Film, scenes: &mut [SceneOf]) -> Vec<(usize, String)> {
    let (mut before, mut chosen) = (None, Vec::new());
    for scene in scenes.iter_mut() {
        let level = scene.place.0;
        let song =
            film.tables.place_songs.get(&level).and_then(|number| rules::songs::SONG_WORDS.iter().find(|(_, song)| song == number)).map(|(word, _)| *word);
        if let Some(word) = song.filter(|_| before != Some(level) && !asks_a_song(&scene.lines)) {
            let number = scene.lines.first().map_or(0, |item| item.number);
            scene.lines.insert(0, Instr::word(number, "music", &[word.to_string()], true));
            chosen.push((number, word.to_string()));
        }
        before = Some(level);
    }
    chosen
}

/// How many of a scene's lines set it, before it shows.
/// The head of a scene is what comes before its first action: the lines
/// that set it, and among them, in any order, those that change nothing
/// one sees happen (the weather as it is, `weather rain`; who looks where;
/// how one is drawn, `prop`, `anim`, out of the foliage or under it;
/// the sound of the place): a `put` after them still places. The scene's
/// first framing right after them (`frame a b`, its `moment:` before it, if
/// any) is of the head too: the scene opens on it, never on a view that
/// cuts its big bodies off before it lands.
fn setting_lines(lines: &[Instr]) -> usize {
    // (Who flies there from the start, `pidgey fly [height 30]`; not a
    // flight to somewhere, which one sees.)
    let sets = |item: &Instr| {
        item.verb().is_some_and(|verb| SETTING.contains(&verb))
            && !(item.verb() == Some("camera") && item.words().first().is_some_and(|word| word == "from"))
            && !(item.verb() == Some("fly") && !matches!(item.words().len(), 1 | 3))
    };
    let as_it_is = |item: &Instr| match item.verb() {
        // (The weather alone; `weather a rain` is someone's move.)
        Some("weather") => item.words().len() == 1,
        // (Who is in the water from the start, `lapras swim`; who walks
        // whatever his kind, `lapras walk`.)
        Some("swim" | "walk") => item.words().len() == 1,
        Some("look" | "ambient" | "prop" | "anim" | "front_of_leaves" | "under_leaves" | "in_leaves") => true,
        _ => false,
    };
    let head = lines.iter().take_while(|item| sets(item) || as_it_is(item)).count();
    let framing = head + usize::from(lines.get(head).is_some_and(|item| item.verb() == Some("moment")));
    match lines.get(framing) {
        Some(item) if item.verb() == Some("frame") && !item.words().iter().any(|word| word == "follows" || word == "free") => framing + 1,
        _ => head,
    }
}

/// The second word of the lines that start with that one.
fn named_after(scene: &SceneOf, word: &str) -> HashSet<String> {
    scene.lines.iter().filter(|item| item.verb() == Some(word)).filter_map(|item| item.words().first()).map(|named| plain(named)).collect()
}

impl Film {
    /// What the film knows of a scene at hand starts again.
    fn scene_starts(&mut self, level: u16, number: usize) {
        self.rows = self.tables.grounds.get(&level).cloned();
        if self.tables.grounds.is_empty() {
            self.remark(number, "no collision maps: run decor game_grounds".to_string());
        } else if self.rows.is_none() && level != MAP_LEVEL && !self.tables.picture_levels.contains(&level) {
            self.remark(number, "this decor is a picture, not a place to walk: no one is guided there".to_string());
        }
        self.level = level;
        self.spots = self.lasting_spots.iter().map(|(name, _, _)| name.clone()).collect();
        self.moment = None;
        // (A new place: nothing of the last one, its night gone with it.)
        self.staging = crate::film::Staging::default();
        self.speaking = Default::default();
        self.things = Default::default();
        for beat in self.kept_new_scene() {
            self.beat(beat);
        }
    }

    /// The place: its ground, its ways in and out, the film's spots, what
    /// stands there in the game, whoever is in the scene.
    fn place_opens(&mut self, at: usize, (level, x, y): (u16, i32, i32), body_at: &mut Option<usize>) {
        // (The middle of a scene is ground, as all it is measured from.)
        let point = glam::Vec2::new(x as f32, y as f32);
        let free = self.rows.as_ref().map_or(point, |rows| rows.free(point));
        self.middle = (free.x as i32, free.y as i32);
        if std::mem::take(&mut self.box_kept) {
            self.beat(emit::box_auto());
        }
        self.written.push((self.number, Sortie::Scene { level, name: format!("scene{}", at + 1) }));
        self.plan.scene_opens();
        self.beat(emit::label(format!("scene{}", at + 1)));
        self.wait(2);
        self.beat(emit::backdrop(level, true, None));
        if body_at.is_none() {
            *body_at = Some(self.written.len());
        }
        let middle = Vec2::new(self.middle.0 as f32, self.middle.1 as f32);
        self.beat(emit::ground(middle, self.rows.clone()));
        if self.collisions {
            self.beat(emit::collisions(true));
        }
        // (Where one comes in and goes out, for `exit a via south`; each a spot
        // of the scene too, but for the words that already are one.)
        let ways = self.tables.entrances.get(&level).cloned().unwrap_or_default();
        if !ways.is_empty() {
            self.beat(emit::entrances(ways.iter().map(|(name, x, y)| (name.clone(), Vec2::new(*x as f32, *y as f32))).collect()));
        }
        for (name, x, y) in ways.iter().filter(|(name, ..)| spot(name, (0, 0)).is_none()) {
            self.beat(emit::name_spot(name, Spot::Exact(Vec2::new(*x as f32, *y as f32).into())));
            self.spots.insert(name.clone());
        }
        self.entrances = ways.into_iter().map(|(name, ..)| name).collect();
        for (name, x, y) in self.lasting_spots.clone() {
            self.beat(emit::name_spot(name, Spot::Exact(Vec2::new(x as f32, y as f32).into())));
        }
        let tables = self.tables.clone();
        for (thing, x, y) in tables.things.get(&level.to_string()).into_iter().flatten() {
            self.on(thing, |thing| emit::put(thing, *x as f32, *y as f32, engine::Direction::Down));
        }
    }

    /// Those the scene names, then anyone who acts in it, but who a line puts
    /// there itself; and those a line puts there.
    fn cast_of(&mut self, scene: &SceneOf, setting: usize) -> (Vec<String>, Vec<String>) {
        let mut listed: Vec<String> = scene.listed.iter().filter(|word| self.actor(word).is_some()).map(|word| plain(word)).collect();
        // (`residents`: who lives there, each on its own place; see `residents_put`.)
        if scene.listed.iter().any(|word| plain(word) == "residents") {
            for resident in self.tables.residents.get(&scene.place.0).cloned().unwrap_or_default() {
                if self.actor(&resident.name).is_some() && !listed.contains(&resident.name) {
                    listed.push(resident.name);
                }
            }
        }
        let mut cast = listed.clone();
        for who in who_acts(self, &scene.lines) {
            if !cast.contains(&who) {
                cast.push(who);
            }
        }
        // (Who is painted in some ways only keeps facing them by itself.)
        for who in &cast {
            let code = self.code(who);
            if let Some(ways) = species(&code).and_then(|number| self.tables.views.get(number)).cloned() {
                match ways.iter().map(|way| engine::Direction::of(way)).collect::<Option<Vec<_>>>().filter(|ways| !ways.is_empty()) {
                    Some(ways) => self.on(who, |who| emit::views(who, ways.clone())),
                    None => self.not_a(ways.join(" "), "a run of directions"),
                }
            }
        }
        let placed = placed_by_a_line(self, &scene.lines, &listed, setting);
        let mut flying: Vec<String> = named_after(scene, "fly").into_iter().collect();
        if !flying.is_empty() {
            flying.sort();
            match self.bodies(&flying) {
                Some(who) => self.beat(emit::flies(who)),
                None => self.not_a(flying.join(" "), "Pokémon of the scene"),
            }
        }
        self.by_nature(scene, &cast, &placed);
        // (A crowd stands in its own circle, its first one with it; what
        // someone evolves into is that one, not one more; what one
        // transforms into, not named at the head, a look only.)
        let crowds = named_after(scene, "crowd");
        let mut forms: HashSet<String> = scene
            .lines
            .iter()
            .filter(|item| item.text.split_whitespace().count() == 3 && item.text.starts_with("evolve"))
            .filter_map(|item| item.text.split_whitespace().nth(2))
            .map(plain)
            .collect();
        for item in &scene.lines {
            let words: Vec<String> = item.text.split_whitespace().map(plain).collect();
            // (`evolve pikachu`: what he becomes, the next of his family.)
            if let [first, second] = words.as_slice()
                && (first == "evolve" || second == "evolve")
                && let Some(next) = self.actor(if first == "evolve" { second } else { first }).and_then(|code| self.next_form(&code))
            {
                forms.insert(next);
            }
            // (`transform ditto into charizard`, or who first.)
            if let [first, second, into, form] = words.as_slice()
                && (first == "transform" || second == "transform")
                && into == "into"
                && !listed.contains(form)
            {
                forms.insert(form.clone());
            }
        }
        cast.retain(|who| !placed.contains(who) && !crowds.contains(who) && !forms.contains(who));
        // (Who is there from the start is named at the head of the scene; the
        // others come by a line that says so: nothing about the stage hidden
        // in a line far below.)
        // (One of a crowd, `marill3`: the crowd's own.)
        let crowd_kinds: HashSet<String> = scene
            .lines
            .iter()
            .filter(|item| matches!(first_word(&item.text).as_str(), "gathering" | "crowd" | "gang"))
            .flat_map(|item| item.text.split_whitespace().map(plain).collect::<Vec<_>>())
            .collect();
        for who in cast.iter().filter(|who| !listed.contains(who) && !crowd_kinds.contains(who.trim_end_matches(|letter: char| letter.is_ascii_digit()))) {
            let first = scene.lines.iter().find(|item| who_acts(self, std::slice::from_ref(item)).contains(who));
            // (A line that puts someone there or brings him in, as its word says.)
            let comes = first.is_some_and(|item| puts_someone(self, item));
            if !comes {
                let number = first.map_or(0, |item| item.number);
                self.wrong(
                    number,
                    format!(
                        "{who} is there from the start of the scene: name him at the head (\"scene: place … {who}\") or have him come (put, enter, appear)"
                    ),
                );
            }
        }
        (cast, placed)
    }

    /// How each goes about by nature, as the game's tables say, unless a
    /// line of the scene says `who walk`: a hovering kind floats and goes
    /// straight over all (`flies`); a ghost goes through walls, seen through
    /// in them; one of the water, put where no one walks (the water), is in
    /// it (`swims`). The film's own `swim`, `fly`, `land` come after, and
    /// win.
    fn by_nature(&mut self, scene: &SceneOf, cast: &[String], placed: &[String]) {
        let walking: HashSet<String> = scene
            .lines
            .iter()
            .filter(|item| item.verb() == Some("walk") && item.words().len() == 1)
            .filter_map(|item| item.words().first())
            .map(|named| plain(named))
            .collect();
        let mut seen: HashSet<&String> = HashSet::new();
        for who in cast.iter().chain(placed).filter(|who| !walking.contains(*who)) {
            if !seen.insert(who) {
                continue;
            }
            let Some(code) = self.actor(who) else { continue };
            match self.movement_of(&code) {
                Movement::Walks => {}
                Movement::Hovers => {
                    self.on(who, |one| emit::flies(vec![one]));
                    self.on(who, |one| emit::hovers(vec![one]));
                }
                Movement::Ghost => {
                    self.on(who, |one| emit::flies(vec![one]));
                    self.on(who, |one| emit::ghosts(vec![one]));
                }
                // (A place's lava is ground no one walks on, as its walls: not told apart.)
                Movement::Lava => {}
                // (On land it walks round the water as anyone; put in it, or
                // walked into it, it is in it.)
                Movement::Swims => {
                    let depth = crate::verbs::moving::swimming::depth_of(self, who);
                    self.on(who, |one| emit::swims(depth, vec![one]));
                }
            }
        }
    }

    /// What sets a scene, played before it shows: the song that plays goes
    /// on, another comes in softly.
    fn set(&mut self, at: usize, lines: &[Instr], song: &mut Option<String>) {
        for item in lines {
            let heard = song_asked(item);
            if heard.is_some() && heard == *song {
                continue;
            }
            let mut item = item.clone();
            if let Some(heard) = heard {
                *song = Some(heard);
                if at > 0 {
                    // (The song coming in over a second: its word, one more word.)
                    item = match &item.shape {
                        Shape::Word { verb, words, wait } if !item.text.contains('&') => {
                            let words: Vec<String> = words.iter().cloned().chain(["1s".to_string()]).collect();
                            Instr { inner: item.inner.clone(), ..Instr::word(item.number, verb, &words, *wait) }
                        }
                        _ => item,
                    };
                }
            }
            self.number = item.number;
            command(self, &item);
        }
    }

    pub(super) fn one_scene(&mut self, at: usize, scenes: &[SceneOf], song: &mut Option<String>, body_at: &mut Option<usize>) {
        let scene = &scenes[at];
        let number = scene.lines.first().map_or(0, |item| item.number);
        self.number = number;
        self.scene_starts(scene.place.0, number);
        self.place_opens(at, scene.place, body_at);
        let setting = setting_lines(&scene.lines);
        let (mut cast, mut placed) = self.cast_of(scene, setting);
        self.staging.brought = placed.clone();
        // (Who lives there stands on its own place, not spread with the rest.)
        let residents: Vec<_> = match scene.listed.iter().any(|word| plain(word) == "residents") {
            true => self.tables.residents.get(&scene.place.0).cloned().unwrap_or_default(),
            false => Vec::new(),
        };
        cast.retain(|who| !residents.iter().any(|resident| resident.name == *who));
        // Spread out, each turned to the others; the view on them all.
        let ground = if self.tables.picture_levels.contains(&scene.place.0) { "image" } else { "place" };
        match self.bodies(&cast) {
            Some(who) => self.beat(emit::spread(who, ground == "image")),
            None => self.not_a(cast.join(" "), "Pokémon of the scene"),
        }
        for resident in &residents {
            match engine::Direction::of(&resident.facing) {
                Some(way) => self.on(&resident.name, |who| emit::put(who, resident.x as f32, resident.y as f32, way)),
                None => self.not_a(&resident.facing, "a direction"),
            }
            if resident.wanders > 0 {
                self.on(&resident.name, |who| emit::roams(who, resident.wanders as f32));
            }
            // (What it does there, as in the game: over and over, or held.)
            if let Some(group) = resident.animation.as_deref().and_then(emit::anim_of) {
                let still = resident.still;
                self.on(&resident.name, |who| emit::anim(who, group, !still, still));
            }
        }
        // (The light as it is by day, whatever the scene before did.)
        self.beat(emit::tint(emit::light_of(engine::Daylight::Normal), 1.0));
        self.beat(emit::grey(false));
        self.beat(emit::weather(engine::Weather::Clear));
        self.set(at, &scene.lines[..setting], song);
        for (who, other) in std::mem::take(&mut self.staging.facing_later) {
            crate::verbs::acting::faced(self, &who, &other);
        }
        self.number = number;
        // (The film's title, alone on the dark, before all else.)
        if at == 0 && !self.title.is_empty() {
            let title = self.title.clone();
            self.caption(150.0, &title, true);
            self.wait(170);
        }
        match scene.transition.as_str() {
            "white" | "boss" => self.beat(emit::fade(false, true, FADE as f32)),
            "iris" => self.beat(emit::iris(true, FADE as f32, None)),
            "cut" => self.wait(1),
            "wipe" => self.beat(emit::wipe(false, 1, WIPE as f32)),
            _ => self.beat(emit::fade(false, false, FADE as f32)),
        }
        self.staging.started = true;
        self.beat(emit::wait_fade());
        self.beat(emit::spacing(true));
        self.beat(emit::auto_face(true));
        self.beat(emit::keep_speaker(true));
        placed.sort();
        let listening: Vec<String> = cast.into_iter().chain(placed).collect();
        // (They look at who speaks, and keep their own feelings: none takes
        // the speaker's emotion.)
        if !listening.is_empty() {
            match self.bodies(&listening) {
                Some(who) => self.beat(emit::audience(who)),
                None => self.not_a(listening.join(" "), "Pokémon of the scene"),
            }
        }
        crate::plot::scene_shows(self, at);
        self.lines_of(number, &scene.lines[setting..]);
        self.scene_ends(scenes.get(at + 1), song);
    }

    /// The lines of a scene once it shows, in turn.
    fn lines_of(&mut self, number: usize, lines: &[Instr]) {
        // (Marks are of the scene's clock: none from the scene before.)
        self.timing.time_marks.clear();
        let mut last = number;
        for item in lines {
            last = item.number;
            self.play(item);
        }
        self.number = last;
        self.throw_lands();
        // (What was going on meanwhile is seen to its end.)
        wait_meanwhile(self);
    }

    /// The fade out, as the next scene opens; its song coming, this one dies.
    fn scene_ends(&mut self, next: Option<&SceneOf>, song: &Option<String>) {
        self.wait(i64::from(LAST_PAUSE));
        let coming = next.and_then(|next| next.lines.iter().find_map(song_asked));
        if coming.is_some() && coming != *song {
            self.beat(emit::music_fade(0, 60.0));
        }
        match next.map_or("fade", |next| next.transition.as_str()) {
            "white" => self.beat(emit::fade(true, true, FADE as f32)),
            "boss" => boss_flashes(self),
            "iris" => {
                let last = self.speaking.last_speaker.clone();
                match last.as_deref().map(|last| self.other(last)) {
                    None => self.beat(emit::iris(false, FADE as f32, None)),
                    Some(Some(on)) => self.beat(emit::iris(false, FADE as f32, Some(on))),
                    Some(None) => self.not_a(last.unwrap_or_default(), "a Pokémon of the scene"),
                }
            }
            "cut" => self.wait(1),
            "wipe" => self.beat(emit::wipe(true, 1, WIPE as f32)),
            _ => self.beat(emit::fade(true, false, FADE as f32)),
        }
        self.beat(emit::wait_fade());
    }
}

/// Before a boss, as the game does: the screen flashes white three times
/// with the shock's sound, then all goes white.
pub(crate) fn boss_flashes(film: &mut Film) {
    film.sound("boss");
    for _ in 0..3 {
        film.beat(emit::flash(emit::light(engine::Colour::White), 8.0));
        film.wait(12);
    }
    film.beat(emit::quake(1.0, 20.0));
    film.beat(emit::fade(true, true, 20.0));
}
