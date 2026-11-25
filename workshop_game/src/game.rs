//! A game of one's own, as the Game space reads and writes it: the `.game`
//! files of the project, the card of one (its head: `title:`, `author:`,
//! `team:`, `home:`) written back into its text with the rest kept as it
//! is, and its chapters as boxes: what each one plays, where it goes, the
//! dungeons and fights it sends the team to, its rules, the chapters it
//! leads to. The game itself is read by `game::game_file`.

use std::path::PathBuf;

use assets::project::Project;
use game::game_file::{Action, Chapter, GameFile, Rule, When};

/// The words of a game's head, in the order a game writes them.
pub const HEAD: [&str; 4] = ["title", "author", "team", "home"];

/// Where the games are: `games/`, a folder each (`games/<name>/<name>.game`).
pub fn folder(library: &Project) -> PathBuf {
    library.games_folder()
}

/// The games: those of `games/`, then those left in `library`.
pub fn games(library: &Project) -> Vec<PathBuf> {
    library.games()
}

/// A new game's text: its head, one chapter to begin with.
pub fn new_text(title: &str) -> String {
    format!(
        "# {title}: a game of one's own, its folder a whole: its films in films/, its places in decors/, its dungeons in dungeons/.\ntitle: {title}\nteam: pikachu piplup\n\nchapter 1 \"The Beginning\"\n  # play a_film\n  # go a_place\n  finish\nend\n"
    )
}

/// The word of a game titled so (`the_big_quest`), none without a title.
pub fn word_of(title: &str) -> Option<String> {
    let words: Vec<String> = title
        .split_whitespace()
        .map(|word| word.to_lowercase().chars().filter(|letter| letter.is_alphanumeric()).collect::<String>())
        .filter(|word| !word.is_empty())
        .collect();
    (!words.is_empty()).then(|| words.join("_"))
}

/// The file a new game titled so would be (`games/the_big_quest/the_big_quest.game`), none without a title.
pub fn file_of(library: &Project, title: &str) -> Option<PathBuf> {
    word_of(title).map(|word| folder(library).join(&word).join(format!("{word}.game")))
}

/// A new game titled so made: its folder, `films/`, `decors/`… and its file.
pub fn create(library: &Project, title: &str) -> Result<PathBuf, String> {
    let word = word_of(title).ok_or("a title first")?;
    library.new_game(&word, &new_text(title.trim()))
}

/// Whether a line opens a chapter (`chapter 1 …` at its start).
fn opens_chapter(line: &str) -> bool {
    line.starts_with("chapter ") || line == "chapter"
}

/// What the head says for that word (`team` → `pikachu piplup`), if it says it.
pub fn head(text: &str, key: &str) -> Option<String> {
    text.lines()
        .take_while(|line| !opens_chapter(line))
        .find_map(|line| line.trim().strip_prefix(key).and_then(|rest| rest.strip_prefix(':')).map(|value| value.trim().to_string()))
}

/// The game's text with that word of its head made to say that (empty:
/// taken out); everything else kept, comments and order. A word not said
/// yet goes beside the others, in the order of `HEAD`.
pub fn with_head(text: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let head_end = lines.iter().position(|line| opens_chapter(line)).unwrap_or(lines.len());
    let key_of = |line: &str| HEAD.iter().position(|known| line.trim().strip_prefix(known).is_some_and(|rest| rest.starts_with(':')));
    let value = value.trim();
    let written = format!("{key}: {value}");
    match lines[..head_end].iter().position(|line| key_of(line).is_some_and(|at| HEAD[at] == key)) {
        Some(at) if value.is_empty() => _ = lines.remove(at),
        Some(at) => lines[at] = written,
        None if value.is_empty() => {}
        None => {
            let rank = HEAD.iter().position(|known| *known == key).unwrap_or(HEAD.len());
            let said: Vec<(usize, usize)> = lines[..head_end].iter().enumerate().filter_map(|(at, line)| Some((at, key_of(line)?))).collect();
            let at = match (said.iter().rev().find(|(_, known)| *known < rank), said.iter().find(|(_, known)| *known > rank)) {
                (Some((before, _)), _) => before + 1,
                (None, Some((after, _))) => *after,
                // (No head yet: after the comments the file begins with.)
                (None, None) => lines[..head_end].iter().position(|line| !line.trim().starts_with('#')).unwrap_or(head_end),
            };
            lines.insert(at, written);
        }
    }
    lines.join("\n") + "\n"
}

/// What a chapter touches that has an editor of its own, or another chapter.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Link {
    Film(String),
    Place(String),
    Dungeon(String),
    Fight(String),
    /// Opened on the world map (a place or a dungeon).
    Opened(String),
}

impl Link {
    /// What it is, in a word; its name.
    pub fn said(&self) -> (&'static str, &str) {
        match self {
            Link::Film(name) => ("film", name),
            Link::Place(name) => ("place", name),
            Link::Dungeon(name) => ("dungeon", name),
            Link::Fight(name) => ("fight", name),
            Link::Opened(name) => ("opens", name),
        }
    }
}

/// A chapter as a box of the graph: its number, its name, what it touches
/// (each once, in the order it is met), its rules (their heads), the
/// chapters it leads to, whether it ends the game.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChapterBox {
    pub number: u16,
    pub title: String,
    pub touches: Vec<Link>,
    pub rules: Vec<String>,
    pub leads_to: Vec<u16>,
    pub ends: bool,
}

/// A rule's head as written (`on talk kecleon`, ` if …` when it has a test).
fn rule_head(rule: &Rule) -> String {
    let when = match &rule.when {
        When::Enter => "on enter".to_string(),
        When::Talk(who) => format!("on talk {who}"),
        When::Zone(name) => format!("on zone {name}"),
        When::Won(name) => format!("on won {name}"),
        When::Lost(name) => format!("on lost {name}"),
    };
    if rule.test.is_some() { format!("{when} if …") } else { when }
}

/// What actions touch, put in the box.
fn walked(actions: &[Action], into: &mut ChapterBox) {
    let touch = |into: &mut ChapterBox, link: Link| {
        if !into.touches.contains(&link) {
            into.touches.push(link);
        }
    };
    for action in actions {
        match action {
            Action::Play(name) => touch(into, Link::Film(name.clone())),
            Action::Go(place, _) => touch(into, Link::Place(place.clone())),
            Action::Dungeon(name) => touch(into, Link::Dungeon(name.clone())),
            Action::Fight(name) => touch(into, Link::Fight(name.clone())),
            Action::Open(name) => touch(into, Link::Opened(name.clone())),
            Action::Chapter(number) if !into.leads_to.contains(number) => into.leads_to.push(*number),
            Action::Finish => into.ends = true,
            Action::If(branches) => {
                for (_, inner) in branches {
                    walked(inner, into);
                }
            }
            _ => {}
        }
    }
}

/// A chapter as a box.
pub fn chapter_box(chapter: &Chapter) -> ChapterBox {
    let mut made = ChapterBox { number: chapter.number, title: chapter.title.clone(), ..ChapterBox::default() };
    walked(&chapter.actions, &mut made);
    for rule in &chapter.rules {
        made.rules.push(rule_head(rule));
        walked(&rule.actions, &mut made);
    }
    made
}

/// The game's chapters as boxes, in their order.
pub fn chapter_boxes(game: &GameFile) -> Vec<ChapterBox> {
    let mut boxes: Vec<ChapterBox> = game.chapters.iter().map(chapter_box).collect();
    boxes.sort_by_key(|made| made.number);
    boxes
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = "# The Trial: a comment kept.\ntitle: The Trial\nteam: pikachu piplup\n# where the team comes back to\nhome: town south\n\nchapter 1 \"Start\"\n  # title: not the head\n  finish\nend\n";

    #[test]
    fn the_card_written_back_keeps_the_rest_of_the_file() {
        let made = with_head(GAME, "title", "The Big Trial");
        assert_eq!(made, GAME.replace("title: The Trial", "title: The Big Trial"));
        // (A word not said yet: beside the others, in their order.)
        let made = with_head(&made, "author", "Me");
        assert_eq!(made.lines().nth(2), Some("author: Me"));
        assert_eq!(head(&made, "author").as_deref(), Some("Me"));
        // (Empty: taken out; the comments and the chapters as they were.)
        let made = with_head(&made, "home", "");
        assert!(!made.contains("home:"));
        assert!(made.contains("# where the team comes back to") && made.contains("  # title: not the head"));
        assert_eq!(head(&made, "title").as_deref(), Some("The Big Trial"));
        let read = GameFile::read(&made);
        assert!(read.problems.is_empty(), "{:?}", read.problems);
        assert_eq!((read.title.as_str(), read.author.as_str(), read.home.clone()), ("The Big Trial", "Me", None));
        // (A head of nothing yet: after the first comments.)
        let bare = with_head("# a game\n\nchapter 1\nend\n", "title", "X");
        assert_eq!(bare, "# a game\ntitle: X\n\nchapter 1\nend\n");
    }

    #[test]
    fn the_example_s_chapters_link_what_they_touch() {
        let path = workshop_base::session::Session::for_tests().root().join("tests/games/test_village/test_village.game");
        let game = GameFile::open(&path).expect("the example game");
        let boxes = chapter_boxes(&game);
        assert_eq!(boxes.len(), 2);
        let (first, second) = (&boxes[0], &boxes[1]);
        assert_eq!((first.number, first.title.as_str()), (1, "Chapter 1"));
        assert_eq!(first.touches, [Link::Film("arrival".into()), Link::Place("test_market".into()), Link::Dungeon("test_forest".into())]);
        assert_eq!((first.leads_to.as_slice(), first.ends), ([2].as_slice(), false));
        assert_eq!(first.rules, ["on talk pidgeot"]);
        assert_eq!(second.touches, [Link::Film("back_home".into()), Link::Film("thanks".into())]);
        assert_eq!((second.leads_to.is_empty(), second.ends), (true, true));
        assert_eq!(second.rules, ["on talk kecleon if …"]);
    }

    #[test]
    fn a_new_game_reads() {
        let game = GameFile::read(&new_text("The Big Quest"));
        assert!(game.problems.is_empty(), "{:?}", game.problems);
        assert_eq!(game.title, "The Big Quest");
        assert_eq!(
            file_of(workshop_base::session::Session::for_tests().library(), "The Big Quest!")
                .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
                .as_deref(),
            Some("the_big_quest.game")
        );
    }
}
