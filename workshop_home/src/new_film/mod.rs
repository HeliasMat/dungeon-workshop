//! A new film, as the New Film window makes it: a place, a cast, a start
//! (an empty stage, two who talk, one who arrives, a chase, two moments),
//! a title. Its text plays at once: the scene with its cast, each one put
//! where the scene's own placing puts them (on its ground, turned to one
//! another, found by playing the scene once), the place's music, a first
//! moment with its framing, then the start's lines.
//!
//! What is written lives here, apart from the window (`ui`), so that the
//! tests read it as the film checker does.

pub mod ui;

use std::collections::HashMap;
use std::sync::Arc;

/// How a new film begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// Everyone placed and framed; nothing said yet.
    Empty,
    /// The first two talk; the others answer.
    Talk,
    /// One is waiting; the last of the cast walks in.
    Arrival,
    /// The second chases the first, round the place.
    Chase,
    /// A morning, then the evening: two moments.
    TwoMoments,
}

impl Start {
    pub const ALL: [Start; 5] = [Start::Talk, Start::Arrival, Start::Chase, Start::TwoMoments, Start::Empty];

    pub fn name(self) -> &'static str {
        match self {
            Start::Empty => "Empty Stage",
            Start::Talk => "A Conversation",
            Start::Arrival => "An Arrival",
            Start::Chase => "A Chase",
            Start::TwoMoments => "Two Moments",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Start::Empty => "everyone placed and framed, nothing said: the page is yours",
            Start::Talk => "the first two talk, the others answer",
            Start::Arrival => "they wait; the last one walks in, late",
            Start::Chase => "a dare, then the second chases the first",
            Start::TwoMoments => "a morning, then the evening falls: two moments",
        }
    }

    /// Who comes in by a line (not put at the start): the last one, in an arrival.
    fn arriving(self, cast: usize) -> Option<usize> {
        (self == Start::Arrival && cast >= 2).then(|| cast - 1)
    }
}

/// What the window chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub title: String,
    /// The place's word, as `scene:` takes it (a place of the game, a decor).
    pub place: String,
    /// The cast's words (`pikachu`), in the order chosen, and the names their lines say (`Pikachu`).
    pub cast: Vec<(String, String)>,
    pub start: Start,
}

/// Someone put at the start: where their feet are, the way they face.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub word: String,
    pub at: [i32; 2],
    pub facing: String,
}

/// The fewest and the most a new film's cast holds.
pub const FEWEST: usize = 2;
pub const MOST: usize = 6;

/// The film's lines after its setup: its moments.
fn moments(plan: &Plan) -> Vec<String> {
    let words: Vec<&str> = plan.cast.iter().map(|(word, _)| word.as_str()).collect();
    let names: Vec<&str> = plan.cast.iter().map(|(_, name)| name.as_str()).collect();
    let (a, b) = (words[0], words.get(1).copied().unwrap_or(words[0]));
    let (name_a, name_b) = (names[0], names.get(1).copied().unwrap_or(names[0]));
    let others = &words[2.min(words.len())..];
    let all = words.join(" ");
    let mut lines: Vec<String> = Vec::new();
    let mut add = |line: String| lines.push(line);
    match plan.start {
        Start::Empty => {
            add("moment: the start".to_string());
            add(format!("frame {all}"));
            add("wait 1s".to_string());
            add(format!("# Your film begins here: `{a} (happy): Hello!`, `{b} walk to {a}`... (Ctrl+Space offers what comes next)"));
        }
        Start::Talk => {
            add("moment: hello".to_string());
            add(format!("frame {all}"));
            add(format!("{a} (happy): Hey, {name_b}! There you are."));
            add(format!("{b} (surprised): Oh! {name_a}! I didn't see you."));
            add(format!("{a}: Something strange is going on around here..."));
            add(format!("{b} (worried): Strange? What do you mean?"));
            for other in others {
                add(format!("{other} hop surprised"));
            }
            add(format!("{a} (determined): Come on. I'll show you!"));
            add(format!("{b} nod"));
        }
        Start::Arrival => {
            let arriving = words[words.len() - 1];
            let arriving_name = names[names.len() - 1];
            let waiting: Vec<&str> = words[..words.len() - 1].to_vec();
            add("moment: waiting".to_string());
            add(format!("frame {}", waiting.join(" ")));
            add(format!("{a} look around"));
            add(format!("{a} (worried): Where is {arriving_name}? It's getting late..."));
            add("wait 1s".to_string());
            add("moment: here at last".to_string());
            add(format!("{arriving} enter from right"));
            add(format!("{arriving} walk to {a}"));
            add(format!("frame {all}"));
            add(format!("{arriving} (happy): Sorry, sorry! I'm here!"));
            add(format!("{a} hop happy"));
            add(format!("{a} (joyous): {arriving_name}! You made it!"));
        }
        Start::Chase => {
            add("moment: the dare".to_string());
            add(format!("frame {all}"));
            add(format!("{a} (happy): Bet you can't catch me, {name_b}!"));
            add(format!("{b} (determined): Oh yeah? We'll see about that!"));
            add("moment: the chase".to_string());
            add(format!("frame {a} {b} follows"));
            add(format!("{b} chase {a}"));
            for other in others {
                add(format!("{other} emote joy"));
            }
            add("wait 3s".to_string());
            add(format!("{b} chase off"));
            add(format!("{a} (joyous): Ha ha! Too slow!"));
        }
        Start::TwoMoments => {
            add("moment: morning".to_string());
            add(format!("frame {all}"));
            add(format!("{a} (happy): What a beautiful morning."));
            add(format!("{b}: A perfect day for an adventure!"));
            add("moment: evening".to_string());
            add("daytime evening".to_string());
            add(format!("frame {all}"));
            add(format!("{b} (sad): The day went by so fast..."));
            add(format!("{a} nod"));
            add(format!("{a}: Let's come back tomorrow, {name_b}."));
        }
    }
    lines
}

/// The scene's head as the placing reads it: the place and those put at the start.
fn put_at_start(plan: &Plan) -> Vec<&str> {
    let arriving = plan.start.arriving(plan.cast.len());
    plan.cast.iter().enumerate().filter(|(at, _)| Some(*at) != arriving).map(|(_, (word, _))| word.as_str()).collect()
}

/// A little film of the scene alone, its cast placed by the scene itself:
/// what is played once to know where each one stands, and read for the place's music.
pub fn sketch(plan: &Plan) -> String {
    format!("scene: {} {}\nwait 2s\n", plan.place, put_at_start(plan).join(" "))
}

/// The new film's text: its title, its scene with its cast (each put where
/// `placed` says, if it says), its music, its moments.
pub fn film_text(plan: &Plan, music: Option<&str>, placed: &[Placed]) -> String {
    let title = plan.title.trim();
    let mut text = format!("# {title}: begun with New Film. Ctrl+Enter plays from the cursor's line; drag someone on the Scene to move them.\n");
    text.push_str(&format!("title: {title}\n\n"));
    let cast: Vec<&str> = plan.cast.iter().map(|(word, _)| word.as_str()).collect();
    text.push_str(&format!("scene: {} {}\n", plan.place, cast.join(" ")));
    if let Some(music) = music {
        text.push_str(&format!("music {music}\n"));
    }
    for word in put_at_start(plan) {
        if let Some(one) = placed.iter().find(|one| one.word == word) {
            text.push_str(&format!("{word} put {} {} facing {}\n", one.at[0], one.at[1], one.facing));
        }
    }
    text.push('\n');
    for line in moments(plan) {
        if line.starts_with("moment:") && !text.ends_with("\n\n") {
            text.push('\n');
        }
        text.push_str(&line);
        text.push('\n');
    }
    text
}

/// The place's own music, as a scene there plays it when it names none.
pub fn music_of(plan: &Plan, tables: Arc<film::tables::Tables>) -> Option<String> {
    film::compile_with(&sketch(plan), tables).songs.into_iter().next().map(|(_, song)| song)
}

/// Where the scene puts each one at its start: its sketch played once (the
/// game's files read for it: a second or two), looked at on its first frame seen.
pub fn placed(plan: &Plan, data: &str, library: &assets::project::Project, tables: Arc<film::tables::Tables>) -> Result<Vec<Placed>, String> {
    let art = preview::art::Art::open_in(data, library.clone())?;
    let mut session = preview::session::Session::new(&sketch(plan), art, tables);
    let report = session.play().clone();
    if let Some(wrong) = report.read.iter().find(|problem| !problem.is_remark()) {
        return Err(wrong.message.clone());
    }
    let names: HashMap<_, String> = report.names.iter().cloned().collect();
    let frame = session.first_seen(10 * 60).unwrap_or(1);
    let film = session.seek(frame).ok_or("the scene does not play")?;
    let moment = preview::state::moment(film, &|who| names.get(&who).cloned().unwrap_or_default());
    Ok(moment
        .bodies
        .into_iter()
        .filter(|body| !body.hidden && plan.cast.iter().any(|(word, _)| *word == body.name))
        .map(|body| Placed { word: body.name, at: [body.at.x.round() as i32, body.at.y.round() as i32], facing: body.faces.to_string() })
        .collect())
}

/// The whole film made: the place's music read, the cast placed (if the game's
/// files are there; else the scene places them itself as it plays), the text.
pub fn made(plan: &Plan, data: &str, library: &assets::project::Project, tables: Arc<film::tables::Tables>) -> String {
    let music = music_of(plan, tables.clone());
    let placed = placed(plan, data, library, tables).unwrap_or_default();
    film_text(plan, music.as_deref(), &placed)
}

/// The most places a scratch film walks to.
const MOST_STOPS: usize = 4;

/// Where a scratch film in a decor walks: to each of its entrances (the
/// first four), else across it, from a quarter to three quarters of its width.
pub fn stops(decor_text: &str) -> Vec<[i32; 2]> {
    let entrances: Vec<[i32; 2]> =
        decor::doc::entrances(decor_text).into_iter().take(MOST_STOPS).map(|([x, y], _)| [x.round() as i32, y.round() as i32]).collect();
    if !entrances.is_empty() {
        return entrances;
    }
    let size = decor_text.lines().find_map(|line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
        ["decor:", _, wide, high, ..] => Some([wide.parse::<i32>().ok()?, high.parse::<i32>().ok()?]),
        _ => None,
    });
    let [wide, high] = size.unwrap_or([512, 384]);
    vec![[wide / 4, high / 2], [wide * 3 / 4, high / 2]]
}

/// A scratch film in a decor (Try in a Film): its cast put where the scene
/// puts them (on its ground), framed; the first says where they are, walks to
/// each stop, the second following; then they meet again.
pub fn tour_text(plan: &Plan, music: Option<&str>, placed: &[Placed], stops: &[[i32; 2]]) -> String {
    let words: Vec<&str> = plan.cast.iter().map(|(word, _)| word.as_str()).collect();
    let (a, b) = (words[0], words.get(1).copied().unwrap_or(words[0]));
    let title = plan.title.trim();
    let mut text = format!("# Try in a Film: a scratch film in {title}, kept in target/ (not your library); Keep in Library keeps it.\n");
    text.push_str(&format!("scene: {} {}\n", plan.place, words.join(" ")));
    if let Some(music) = music {
        text.push_str(&format!("music {music}\n"));
    }
    for word in &words {
        if let Some(one) = placed.iter().find(|one| one.word == *word) {
            text.push_str(&format!("{word} put {} {} facing {}\n", one.at[0], one.at[1], one.facing));
        }
    }
    text.push_str("\nmoment: a look around\n");
    text.push_str(&format!("frame {}\n", words.join(" ")));
    text.push_str(&format!("{a} (happy): So this is {title}!\n"));
    if a != b {
        text.push_str(&format!("{b} nod\n"));
    }
    text.push_str("\nmoment: the way round\n");
    text.push_str(&format!("frame {a} follows\n"));
    if a != b {
        text.push_str(&format!("{b} follow {a}\n"));
    }
    for [x, y] in stops {
        text.push_str(&format!("{a} walk to {x} {y}\n"));
    }
    if a != b {
        text.push_str("\nmoment: together again\n");
        text.push_str(&format!("{b} follow off\n"));
        text.push_str(&format!("{b} walk to {a}\n"));
        text.push_str(&format!("frame {a} {b}\n"));
    }
    text.push_str(&format!("{a} (joyous): What a place!\n"));
    text
}

/// A scratch film in that decor, made whole: the two placed by playing its
/// scene once (the game's files read; else the scene places them as it
/// plays), the place's music, its stops.
#[allow(clippy::too_many_arguments)]
pub fn tried(
    name: &str,
    title: &str,
    decor_text: &str,
    cast: &[&str],
    data: &str,
    library: &assets::project::Project,
    tables: Arc<film::tables::Tables>,
) -> String {
    let plan = Plan {
        title: title.to_string(),
        place: name.to_string(),
        cast: cast.iter().map(|word| (word.to_string(), shown_name(&tables, word))).collect(),
        start: Start::Empty,
    };
    let music = music_of(&plan, tables.clone());
    let placed = placed(&plan, data, library, tables).unwrap_or_default();
    tour_text(&plan, music.as_deref(), &placed, &stops(decor_text))
}

/// The name a Pokémon's lines call it by: the game's (`Pikachu`), else its word made a name.
pub fn shown_name(tables: &film::tables::Tables, word: &str) -> String {
    tables.monsters.get(word).map(|(_, name)| name.clone()).filter(|name| !name.is_empty() && !name.starts_with('?')).unwrap_or_else(|| {
        let said = word.replace('_', " ");
        let mut letters = said.chars();
        letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect())
    })
}

#[cfg(test)]
mod tests;
