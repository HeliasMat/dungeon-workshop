//! What may be typed, offered as one types: the models of lines, the words
//! that may follow, a choice taken.

use super::Completion;
use super::cast::{scene_pair, with_cast};
use super::edits::replaced;
use super::highlight::{names_of, speaker};
use super::knowledge::{known_here, species_of};
use super::text::{letter_of_word, line_at, spot_of};
use workshop_base::session::Session;

/// The models a word unfolds into (offered with the words, Tab): its word,
/// what it is, its lines (`a`, `b`: two of the scene).
const MODELS: [(&str, &str, &str); 10] = [
    ("scene", "a scene, its music, its first moment", "scene: beach a b\nmusic peace\nmoment: the beginning\na: ...\nb: ..."),
    ("dialogue", "two speaking to each other", "a: ...\nb (surprised): ...\na (joyous): ..."),
    (
        "fight",
        "a directed fight, turn by turn",
        "fight directed a against b level 30\n  hp a 120\n  hp b 90\n  hud a b\n  a Tackle b -30\n  b Tackle a -20\n  a Tackle b ko\nend",
    ),
    ("choice", "a choice of the player's, and its two answers", "choice\noption Yes\n  a (joyous): Yes!\noption No\n  a (sad): No...\nend"),
    ("together", "lines that start at the same time", "together\n  jump a\n  jump b\nend"),
    ("beside", "lines beside the film", "beside\n  walk a to right\n  emote a joy\nend"),
    ("when", "when someone arrives: beside the film", "walk a to center &\nwhen a arrives\n  emote a joy\nend"),
    ("define", "a recipe of one's own", "define greet who\n  jump who\n  emote who joy\nend"),
    ("repeat", "lines played several times", "repeat 3\n  jump a\n  wait 0.5s\nend"),
    ("fishing", "a fishing trip, from the line to the catch", "throw a line to 300 90\nbite a\nstruggle a 2\nhaul a magikarp"),
];

/// Whether the forms of the line's word are not to be shown (a line said, a comment).
pub(super) fn no_forms_here(text: &str, cursor: usize) -> bool {
    let (_, _, line) = line_at(text, cursor);
    let trimmed = line.trim_start();
    trimmed.starts_with('#') || speaker(&line).is_some()
}

/// A word offered, taken in the place of the letters typed: a model unfolded
/// (its lines as far in as the line it is on), else the word and a space.
pub(super) fn taken(text: &str, completion: &Completion, word: &str) -> (String, (usize, usize)) {
    if let Some((_, _, lines)) =
        MODELS.iter().find(|(known, ..)| *known == word).filter(|_| completion.choices.iter().any(|(known, about)| known == word && about.starts_with("model")))
    {
        let (_, start, line) = line_at(text, completion.letters.start);
        let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
        let at_start = completion.letters.start - start == lead.chars().count();
        if at_start {
            let (a, b) = scene_pair(text, completion.letters.start);
            let cast: Vec<String> = lines.split('\n').map(|line| with_cast(line, &a, &b)).collect();
            let unfolded = cast.join(&format!("\n{lead}"));
            let (made, end) = replaced(text, completion.letters.clone(), &unfolded);
            // (Its first `...` chosen, to be written at once.)
            let chosen = unfolded.find("...").map(|byte| {
                let from = completion.letters.start + unfolded[..byte].chars().count();
                (from + 3, from)
            });
            return (made, chosen.unwrap_or((end, end)));
        }
    }
    let spaced = text.chars().nth(completion.letters.end).is_some_and(|letter| letter == ' ');
    let with = if spaced || word.ends_with(':') { word.to_string() } else { format!("{word} ") };
    let (made, end) = replaced(text, completion.letters.clone(), &with);
    (made, (end, end))
}

/// What may be typed at the cursor: the words that begin as the one being
/// typed (all, if asked with nothing typed).
/// A change of the chosen lines, made when asked.
pub(super) type LineMade<'a> = Box<dyn Fn() -> String + 'a>;

/// What follows a word taken: offered at once when there are few enough to choose from.
pub(super) fn chained(session: &Session, text: &str, at: usize) -> Option<Completion> {
    offered(session, text, at, true).filter(|completion| completion.choices.len() <= 80 || someone_then(text, at))
}

/// Whether the line, up to there, is someone of the film and a space: what they do comes next.
fn someone_then(text: &str, at: usize) -> bool {
    let (_, start, line) = line_at(text, at);
    let before: String = line.chars().take(at - start).collect();
    let words: Vec<&str> = before.split_whitespace().collect();
    before.ends_with(' ') && words.len() == 1 && names_of(text).contains(words[0]) && !film::is_word(words[0], text)
}

/// The text with the line at the cursor, said who first (`pikachu walk to …`),
/// written word first (`walk pikachu to …`) as the language's service reads
/// it: its letters as many, the columns after both words the same.
pub(super) fn word_first(text: &str, cursor: usize) -> std::borrow::Cow<'_, str> {
    let (index, _, line) = line_at(text, cursor);
    let lead = line.len() - line.trim_start().len();
    let rest = &line[lead..];
    let Some((who, after)) = rest.split_once(' ') else { return text.into() };
    let verb: String = after.chars().take_while(|letter| letter_of_word(*letter)).collect();
    if verb.is_empty() || after.starts_with(' ') || !names_of(text).contains(who) || film::is_word(who, text) || !film::is_word(&verb, text) {
        return text.into();
    }
    let swapped = format!("{}{verb} {who}{}", &line[..lead], &after[verb.len()..]);
    let mut lines: Vec<&str> = text.split('\n').collect();
    lines[index] = &swapped;
    lines.join("\n").into()
}

/// The word a line is said with: its first, or its second if someone of the film comes first.
pub(super) fn line_word(text: &str, line: &str) -> Option<String> {
    let mut words = line.split_whitespace();
    let first = words.next()?;
    if names_of(text).contains(first) && !film::is_word(first, text) {
        return words.next().map(|word| film::words::plain(word.trim_end_matches(':')));
    }
    Some(film::words::plain(first))
}

/// What follows a space typed after a word of a line (not a reply, not a comment): few enough.
pub(super) fn after_space(session: &Session, text: &str, at: usize) -> Option<Completion> {
    let (_, _, line) = line_at(text, at);
    if line.trim_start().starts_with('#') || speaker(&line).is_some() || line.split_whitespace().count() < 1 {
        return None;
    }
    offered(session, text, at, true).filter(|completion| completion.choices.len() <= 40 || someone_then(text, at))
}

/// Whether a block just opened on that line (0-based) wants its `end`: no
/// body already under it, and no `end` where it would be.
pub(super) fn wants_fin(text: &str, line: usize, lead: &str, before: &str) -> bool {
    let first = before.split_whitespace().next().unwrap_or_default();
    let block = matches!(first, "together" | "beside" | "repeat" | "define" | "choice")
        || (first == "fight" && before.contains("directed"))
        || (first == "when" && !before.contains(':'));
    if !block {
        return false;
    }
    match text.split('\n').skip(line + 1).find(|next| !next.trim().is_empty()) {
        None => true,
        Some(next) => {
            let deeper = next.len() - next.trim_start().len() > lead.len();
            !deeper && !matches!(next.split_whitespace().next(), Some("end" | "option" | "else"))
        }
    }
}

/// What may be typed at the cursor; in a film written for the DS (`for
/// ds`), only the words that export, those with a loss marked so.
pub(super) fn offered(session: &Session, text: &str, cursor: usize, asked: bool) -> Option<Completion> {
    let mut completion = every_offer(session, text, cursor, asked)?;
    if film::ds::written_for_ds(text) {
        completion.choices = for_ds(completion.choices);
    }
    (!completion.choices.is_empty()).then_some(completion)
}

/// The choices a film for the DS keeps: a word of the plan only if it
/// exports (marked when with a loss), a model only if its words do; the
/// rest (names, species, things, lines said) as they are.
pub(super) fn for_ds(choices: Vec<(String, String)>) -> Vec<(String, String)> {
    let planned = |word: &str| film::vocabulary::planned(word).is_some();
    choices
        .into_iter()
        .filter_map(|(word, about)| {
            if about.starts_with("model") {
                let lines = MODELS.iter().find(|(model, ..)| *model == word).map_or("", |(_, _, lines)| *lines);
                let words = lines.lines().flat_map(|line| line.split_whitespace().take(2)).map(film::words::plain);
                return words.filter(|said| planned(said)).all(|said| film::ds::status(&said).exports()).then_some((word, about));
            }
            let plain = film::words::plain(&word);
            if !planned(&plain) {
                return Some((word, about));
            }
            match film::ds::status(&plain) {
                film::ds::Export::Direct => Some((word, about)),
                film::ds::Export::Close => Some((word, format!("{about} · DS with loss: {}", film::ds::lost(&plain).unwrap_or("less")))),
                _ => None,
            }
        })
        .collect()
}

fn every_offer(session: &Session, text: &str, cursor: usize, asked: bool) -> Option<Completion> {
    let letters: Vec<char> = text.chars().collect();
    let mut start = cursor.min(letters.len());
    while start > 0 && letter_of_word(letters[start - 1]) {
        start -= 1;
    }
    let typed: String = letters[start..cursor.min(letters.len())].iter().collect();
    // (After `move <who> `: the game's moves, found by what is typed of them.)
    let (_, line_start, line) = line_at(text, cursor);
    let before: String = line.chars().take(cursor - line_start).collect();
    if let Some((byte, who, typed_move)) = workshop_wiki::catalogue::attack_rest(session, &before) {
        let from = line_start + before[..byte].chars().count();
        let moves: Vec<(String, String)> = workshop_wiki::catalogue::move_choices(session, &typed_move).into_iter().take(200).collect();
        if !moves.is_empty() && !(moves.len() == 1 && moves[0].0 == typed_move) {
            let user = species_of(session, text, &who);
            return Some(Completion { letters: from..cursor, choices: moves, chosen: 0, user: Some(user) });
        }
    }
    if typed.is_empty() && !asked {
        return None;
    }
    let plain = film::words::plain(&typed);
    let head: String = before.chars().take(before.chars().count() - typed.chars().count()).collect();
    if let Some(mut found) = known_here(session, text, &head, &plain) {
        found.retain(|(word, _)| film::words::plain(word) != plain);
        return (!found.is_empty()).then_some(Completion { letters: start..cursor, choices: found, chosen: 0, user: None });
    }
    // (Someone first: what they may do, the words said of someone.)
    let names = names_of(text);
    if let [who] = head.split_whitespace().collect::<Vec<_>>().as_slice()
        && head.ends_with(' ')
        && names.contains(*who)
        && !film::is_word(who, text)
    {
        let mut doing: Vec<(String, String)> = workshop_wiki::catalogue::catalogue(session)
            .iter()
            .filter(|entry| entry.subject && film::words::plain(&entry.word).starts_with(&plain))
            .filter(|entry| entry.family != workshop_wiki::catalogue::ADVANCED || plain.len() >= 3)
            .map(|entry| (entry.word.clone(), format!("{} · {}", workshop_wiki::catalogue::family_shown(entry.family), entry.shown())))
            .collect();
        // (The most used first, then the shortest.)
        const USUAL: [&str; 16] =
            ["walk", "run", "look", "jump", "emote", "move", "enter", "exit", "put", "hold", "give", "throw", "joy", "fear", "shift", "turn"];
        doing.sort_by_key(|(word, _)| (USUAL.iter().position(|usual| usual == word).unwrap_or(USUAL.len()), word.len(), word.clone()));
        return (!doing.is_empty()).then_some(Completion { letters: start..cursor, choices: doing, chosen: 0, user: None });
    }
    let mut choices: Vec<(String, String)> = film::service::completions(&word_first(text, cursor), spot_of(text, cursor), session.library())
        .into_iter()
        .filter(|(word, _)| film::words::plain(word).starts_with(&plain) && film::words::plain(word) != plain)
        .collect();
    // (Those that begin as typed first, the shortest first.)
    choices.sort_by_key(|(word, _)| (word.len(), word.clone()));
    choices.dedup_by(|a, b| a.0 == b.0);
    // (At a line's start: the models of lines that begin so, first.)
    let lead = line.chars().take_while(|letter| letter.is_whitespace()).count();
    // (At a line's start: those of the film, who do something or say something.)
    if start == line_start + lead {
        let mut cast: Vec<&String> = names.iter().filter(|name| film::words::plain(name).starts_with(&plain) && !film::is_word(name, text)).collect();
        cast.sort();
        for name in cast.into_iter().rev() {
            if !choices.iter().any(|(word, _)| word == name) {
                choices.insert(0, (name.clone(), "does something: walk, jump, move…".to_string()));
            }
            let said = format!("{name}:");
            if !choices.iter().any(|(word, _)| *word == said) {
                choices.push((said, "says a line".to_string()));
            }
        }
    }
    if start == line_start + lead && !typed.is_empty() {
        let models: Vec<(String, String)> =
            MODELS.iter().filter(|(word, ..)| word.starts_with(&plain)).map(|(word, about, _)| (word.to_string(), format!("model — {about}"))).collect();
        choices.retain(|(word, _)| !models.iter().any(|(model, _)| model == word));
        choices.splice(0..0, models);
    }
    // (Pokémon by their species, once a few letters are typed; things where a thing is wanted.)
    if start != line_start + lead && plain.chars().count() >= 3 {
        let tables = session.tables();
        let species: Vec<(String, String)> = tables
            .monsters
            .iter()
            .filter(|(word, _)| word.starts_with(&plain))
            .take(30)
            .map(|(word, (_, name))| (word.clone(), format!("Pokémon · {name}")))
            .collect();
        choices.extend(species.into_iter().filter(|(word, _)| !choices.iter().any(|(known, _)| known == word)).collect::<Vec<_>>());
    }
    if choices.iter().any(|(_, about)| about.contains("a thing")) {
        let tables = session.tables();
        let things = tables.items.values().map(|(_, name)| name.clone()).chain(tables.props.keys().cloned()).chain(tables.drawings.keys().cloned());
        let mut found: Vec<(String, String)> =
            things.filter(|name| !name.is_empty() && film::words::plain(name).starts_with(&plain)).map(|name| (name, "object".to_string())).collect();
        found.sort();
        found.dedup();
        choices.extend(found.into_iter().take(80));
    }
    (!choices.is_empty()).then_some(Completion { letters: start..cursor, choices, chosen: 0, user: None })
}
