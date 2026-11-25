//! A whole game checked, with no window (`scene_player check-game <game>`):
//! its file, and everything it reaches. Each film it plays is checked as a
//! film is (`film::service::diagnostics`), each place as a decor is
//! (`decor::lang::parse`), each dungeon as the Workshop checks it
//! (`dungeon_check`); and what only shows at the scale of the game:
//!
//! - the names: a film, a place, an entrance, a dungeon or a fight, a
//!   resident, a zone, an item, a Pokémon, a chapter that does not exist;
//! - the flags: set and never read, read and never set (by the game's
//!   tests, the places' conditions and rules, the films' `keep`, `if`,
//!   `set`, `clear`);
//! - the reach: a chapter never gone to, or with no way to end it; a place
//!   nothing leads to; a film or a dungeon only named where the game never
//!   goes; a place opened on a map never shown; a dungeon with no `home:`.
//!
//! It is a walk: from the first chapter, its actions and rules, the places
//! they go to, their ways out and rules, the chapters those go on to…; then
//! what was left aside, walked too for its names, and said never reached.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use decor::lang::Decor;
use language::{Diagnostic, Location};
use rules::own::plain;
use rules::run::Book;

use crate::dungeon_check::Known;
use crate::game_file::{self, Action, Expr, GameFile, Rule, When};

/// The codes of what only the check of a whole game finds (a film, a place,
/// a dungeon keep their own).
pub mod codes {
    /// A name, a flag, a reach of the game that is wrong.
    pub const GAME: &str = "E-GAME";
}

/// A problem in a file, on a line (from 1): its file written with `/`
/// (`file:line: message` as it is shown).
fn told(file: &Path, line: usize, message: impl Into<String>) -> Diagnostic {
    site_of(file, line).wrong(message)
}

/// A line of a file, the file written with `/`.
fn site_of(file: &Path, line: usize) -> Location {
    Location::in_file(file.display().to_string().replace('\\', "/"), line)
}

/// The file a problem is in.
pub fn file_of(problem: &Diagnostic) -> PathBuf {
    PathBuf::from(problem.span.file.as_deref().unwrap_or_default())
}

/// What the check found: the problems, and what it could not look at (no data).
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Each in its file, on its line.
    pub problems: Vec<Diagnostic>,
    pub notes: Vec<String>,
}

/// Where something is named: a file, a line.
#[derive(Clone, Debug, PartialEq)]
struct Site {
    file: PathBuf,
    line: usize,
}

/// What a walk goes through next.
#[derive(Clone, Debug, PartialEq)]
enum Step {
    Chapter(u16),
    Place(String),
}

/// Where actions are read: their file, its text, the line to look for them
/// from, whether the game gets there, the place they are of.
#[derive(Clone)]
struct Within<'a> {
    file: &'a Path,
    text: &'a str,
    from: usize,
    reached: bool,
    place: Option<&'a str>,
}

/// A place read: its decor, its text, its file.
struct Read {
    decor: Decor,
    text: String,
    file: PathBuf,
}

/// Something the game names (a film, a place, a dungeon): where first, whether reached.
#[derive(Default)]
struct Named {
    sites: BTreeMap<String, (Site, bool)>,
}

impl Named {
    fn add(&mut self, name: &str, site: Site, reached: bool) -> bool {
        let entry = self.sites.entry(name.to_string()).or_insert((site, false));
        let first = !entry.1 && reached;
        entry.1 |= reached;
        first
    }
}

/// The check under way.
struct Check<'a> {
    game: &'a GameFile,
    /// The game's project (its folder, then the shared library), and its tables.
    project: assets::project::Project,
    tables: std::sync::Arc<film::tables::Tables>,
    path: &'a Path,
    text: &'a str,
    book: Option<&'a Book>,
    known: Option<Known>,
    problems: Vec<Diagnostic>,
    places: BTreeMap<String, Option<Read>>,
    named_places: Named,
    films: Named,
    dungeons: Named,
    chapters: BTreeSet<u16>,
    queue: VecDeque<Step>,
    /// The flags and counters set (by the game, the places, the films), and read.
    sets: BTreeMap<String, Site>,
    reads: BTreeMap<String, Site>,
    /// Names a film reads in its own tests (enough for a flag to be read).
    film_reads: BTreeSet<String>,
    opened: BTreeMap<String, Site>,
    map_shown: bool,
    dives: Option<Site>,
    /// The chapters' rules on talks and zones, checked once the places are known.
    talks: Vec<(String, Site)>,
    zones: Vec<(String, Site)>,
    /// The reached chapters with no way to end, said at the end.
    endless: BTreeSet<u16>,
}

/// The game of that file, checked; its data (the game's tables) from the
/// context's folder, its places and films its own, then the library's.
pub fn check(context: &film::Context, path: &Path) -> Result<Report, crate::Error> {
    let data = context.data.as_str();
    if !path.is_file() {
        return Err(crate::Error::NoGame(shown(context.root(), path)));
    }
    let text = std::fs::read_to_string(path).map_err(|why| crate::Error::File { path: shown(context.root(), path), why })?.replace("\r\n", "\n");
    let mut game = GameFile::read(&text);
    game.name = path.file_stem().map_or_else(|| "game".to_string(), |stem| stem.to_string_lossy().into_owned());
    game.folder = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    let book = crate::book::read(data).ok();
    let mut report = check_game(context, &game, &text, path, book.as_ref());
    if book.is_none() {
        report.notes.push(format!("no data in {data} (the extractor): the items, the Pokémon and the dungeons were not checked"));
    }
    Ok(report)
}

/// A game read from that text (at that path), checked.
pub fn check_game(context: &film::Context, game: &GameFile, text: &str, path: &Path, book: Option<&Book>) -> Report {
    let project = context.library.at(&game.folder);
    let tables = context.tables_for(&project);
    let mut check = Check {
        game,
        project,
        tables,
        path,
        text,
        book,
        known: book.map(Known::of),
        problems: Vec::new(),
        places: BTreeMap::new(),
        named_places: Named::default(),
        films: Named::default(),
        dungeons: Named::default(),
        chapters: BTreeSet::new(),
        queue: VecDeque::new(),
        sets: BTreeMap::new(),
        reads: BTreeMap::new(),
        film_reads: BTreeSet::new(),
        opened: BTreeMap::new(),
        map_shown: false,
        dives: None,
        talks: Vec::new(),
        zones: Vec::new(),
        endless: BTreeSet::new(),
    };
    check.run();
    let mut problems = check.problems;
    let key = |problem: &Diagnostic| (file_of(problem) != path, file_of(problem), problem.line());
    problems.sort_by_key(key);
    problems.dedup();
    Report { problems, notes: Vec::new() }
}

/// A path as said to whoever checks: from the repository's folder (or the
/// one it is checked from), with forward slashes.
fn shown(root: &Path, path: &Path) -> String {
    if path.is_relative() {
        return path.display().to_string().replace('\\', "/");
    }
    let whole = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let from = [root.to_path_buf(), std::env::current_dir().unwrap_or_default()];
    let short = from.iter().find_map(|folder| whole.strip_prefix(folder).ok()).unwrap_or(path);
    short.display().to_string().replace('\\', "/")
}

/// What a test says of the chapter, as written: true or false whatever the
/// rest is, or none if that depends on more (a flag).
fn chapter_holds(test: &Expr, chapter: u16) -> Option<bool> {
    fn known(expr: &Expr, chapter: u16) -> Option<f64> {
        use language::ast::{BinOp, ExprKind, UnOp};
        let truth = |yes: bool| if yes { 1.0 } else { 0.0 };
        match &expr.kind {
            ExprKind::Number(number) => Some(*number),
            ExprKind::Bool(yes) => Some(truth(*yes)),
            ExprKind::Name(name) if name == "chapter" => Some(f64::from(chapter)),
            ExprKind::Unary(UnOp::Not, inner) => known(inner, chapter).map(|value| truth(value == 0.0)),
            ExprKind::Unary(UnOp::Minus, inner) => known(inner, chapter).map(|value| -value),
            ExprKind::Binary(BinOp::And, left, right) => match (known(left, chapter), known(right, chapter)) {
                (Some(one), _) | (_, Some(one)) if one == 0.0 => Some(0.0),
                (Some(_), Some(_)) => Some(1.0),
                _ => None,
            },
            ExprKind::Binary(BinOp::Or, left, right) => match (known(left, chapter), known(right, chapter)) {
                (Some(one), _) | (_, Some(one)) if one != 0.0 => Some(1.0),
                (Some(_), Some(_)) => Some(0.0),
                _ => None,
            },
            ExprKind::Binary(_, left, right) => {
                known(left, chapter)?;
                known(right, chapter)?;
                Some(game_file::value(expr, &|name| if name == "chapter" { f64::from(chapter) } else { 0.0 }))
            }
            _ => None,
        }
    }
    known(test, chapter).map(|value| value != 0.0)
}

/// What actions may do in that chapter (a test that rules the chapter out
/// skipped): the places they go to, the chapters they go on to, whether
/// they end it (another chapter, or the game's end).
#[derive(Default)]
struct Goes {
    places: BTreeSet<String>,
    chapters: BTreeSet<u16>,
    ends: bool,
}

impl Goes {
    fn of(&mut self, actions: &[Action], chapter: u16, is_place: &dyn Fn(&str) -> bool) {
        for action in actions {
            match action {
                Action::Go(place, _) => {
                    self.places.insert(place.clone());
                }
                Action::Open(word) if is_place(word) => {
                    self.places.insert(word.clone());
                }
                Action::Chapter(number) if *number != chapter => {
                    self.chapters.insert(*number);
                    self.ends = true;
                }
                Action::Finish => self.ends = true,
                Action::If(branches) => {
                    for (test, actions) in branches {
                        if test.as_ref().is_none_or(|test| chapter_holds(test, chapter) != Some(false)) {
                            self.of(actions, chapter, is_place);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// A rule's, if its test lets it play in that chapter.
    fn rule(&mut self, rule: &Rule, chapter: u16, is_place: &dyn Fn(&str) -> bool) {
        if rule.test.as_ref().is_none_or(|test| chapter_holds(test, chapter) != Some(false)) {
            self.of(&rule.actions, chapter, is_place);
        }
    }
}

/// What the rules say of a fight played alone (`dive::fight_alone`: its
/// `name:` lines out, a `name:` and `floors: 0` before the rest), at its
/// line in the fight's own file; said with no line, the line of the key it
/// names (`boss_hp`), else the first.
fn fight_line(text: &str, why: &Diagnostic) -> (usize, String) {
    let kept: Vec<usize> = text.lines().enumerate().filter(|(_, line)| !line.trim_start().starts_with("name:")).map(|(at, _)| at + 1).collect();
    let named = text.lines().position(|line| line.trim_start().starts_with("name:")).map(|at| at + 1);
    let said = why.message.as_str();
    if let Some(played) = Some(why.line()).filter(|played| *played > 0) {
        let line = match played {
            1 => named.unwrap_or(1),
            2 => 1,
            played => kept.get(played - 3).copied().unwrap_or(1),
        };
        return (line, said.to_string());
    }
    let keyed = text.lines().position(|line| {
        line.split_once(':').is_some_and(|(key, _)| {
            let key = key.trim();
            !key.is_empty() && !key.contains(' ') && (said.contains(&format!("\"{key}:")) || said.starts_with(&format!("{key}:")))
        })
    });
    (keyed.map_or(1, |at| at + 1), said.to_string())
}

/// The first line from `from` (from 1) whose words are those (the first
/// ones; `None`: any word there); else `from`.
fn locate(text: &str, from: usize, words: &[Option<&str>]) -> usize {
    text.lines()
        .enumerate()
        .skip(from.saturating_sub(1))
        .find(|(_, line)| {
            let said: Vec<&str> = line.split('#').next().unwrap_or_default().split_whitespace().collect();
            said.len() >= words.len() && words.iter().zip(&said).all(|(wanted, word)| wanted.is_none_or(|wanted| wanted == *word))
        })
        .map_or(from.max(1), |(at, _)| at + 1)
}

/// The names in a folder with that extension (their stems).
fn names_in(folder: &Path, extension: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|said| said == extension))
        .filter_map(|path| Some(path.file_stem()?.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    names
}

/// `unknown <what> “word”. Near: …` as the films say it.
fn unknown<'a>(what: &str, word: &str, known: impl Iterator<Item = &'a str>, look: &str) -> String {
    film::check::unknown(what, word, &film::check::closest_of(word, known), look)
}

/// The names of the tests of a test written (`chapter >= 2 and not met`).
fn names_of_test(test: &Expr) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    game_file::expr_names(test, &mut names);
    names
}

/// The words of a test in a film's line (`if met and chapter > 1`): its names.
fn film_test_names(line: &str) -> Vec<String> {
    line.split(|letter: char| !(letter.is_alphanumeric() || letter == '_'))
        .filter(|word| word.chars().next().is_some_and(|first| first.is_alphabetic() || first == '_'))
        .filter(|word| !["if", "else", "while", "until", "and", "or", "not", "true", "false"].contains(word))
        .map(str::to_string)
        .collect()
}

impl Check<'_> {
    fn say(&mut self, site: &Site, message: impl Into<String>) {
        self.problems.push(told(&site.file, site.line, message).coded(codes::GAME));
    }

    fn game_site(&self, line: usize) -> Site {
        Site { file: self.path.to_path_buf(), line }
    }

    /// The line a chapter opens on.
    fn chapter_line(&self, number: u16) -> usize {
        let wanted = number.to_string();
        self.text
            .lines()
            .position(|line| {
                let mut words = line.split_whitespace();
                !line.starts_with([' ', '\t']) && words.next() == Some("chapter") && words.next() == Some(wanted.as_str())
            })
            .map_or(1, |at| at + 1)
    }

    fn run(&mut self) {
        // --- What does not read ---
        for said in self.game.problems.clone() {
            let site = self.game_site(Some(said.line()).filter(|line| *line > 0).unwrap_or(1));
            let code = said.code_or(codes::GAME);
            self.problems.push(told(&site.file, site.line, said.message).coded(code));
        }
        // --- The head ---
        let head = |key: &str| self.text.lines().position(|line| line.trim_start().starts_with(&format!("{key}:"))).map_or(1, |at| at + 1);
        let (team_line, home_line) = (head("team"), head("home"));
        let tables = self.tables.clone();
        let monsters = &tables.monsters;
        // (No data: no Pokémon known, none checked.)
        for member in self.game.team.clone() {
            if !monsters.is_empty() && !monsters.contains_key(member.as_str()) {
                let site = self.game_site(team_line);
                self.say(&site, unknown("unknown Pokémon", &member, monsters.keys().map(String::as_str), "team: the hero, the partner"));
            }
        }
        if let Some((place, entrance)) = self.game.home.clone() {
            self.name_place(&place, entrance.as_deref(), self.game_site(home_line), true);
        }
        let mut numbers = BTreeSet::new();
        for chapter in &self.game.chapters {
            if !numbers.insert(chapter.number) {
                let site = self.game_site(self.chapter_line(chapter.number));
                self.say(&site, format!("two chapters {}: the second is never played", chapter.number));
            }
        }
        // --- The walk, from the first chapter ---
        let Some(first) = self.game.first().map(|chapter| chapter.number) else {
            let site = self.game_site(1);
            self.say(&site, "a game has chapters: “chapter 1” … “end”");
            return;
        };
        self.queue.push_back(Step::Chapter(first));
        loop {
            while let Some(step) = self.queue.pop_front() {
                match step {
                    Step::Chapter(number) => self.walk_chapter(number, true),
                    Step::Place(name) => self.walk_place(&name, true),
                }
            }
            // (The map shown: what was opened on it is reached.)
            let opened: Vec<(String, Site)> = self.opened.iter().map(|(name, site)| (name.clone(), site.clone())).collect();
            let more: Vec<(String, Site)> = opened.into_iter().filter(|(name, _)| self.map_shown && !self.is_reached(name)).collect();
            if more.is_empty() {
                break;
            }
            for (name, site) in more {
                self.open(&name, site, true);
            }
        }
        // --- What the walk never came to: walked for its names, said never reached ---
        let walked = self.chapters.clone();
        let (reached_chapters, ends) = self.chapters_reach(first);
        for chapter in self.game.chapters.clone() {
            if !reached_chapters.contains(&chapter.number) {
                let site = self.game_site(self.chapter_line(chapter.number));
                self.say(
                    &site,
                    format!(
                        "chapter {} is never reached: no “chapter {}” that can play leads to it (the game starts at chapter {first})",
                        chapter.number, chapter.number
                    ),
                );
                if !walked.contains(&chapter.number) {
                    self.walk_chapter(chapter.number, false);
                }
            } else if !ends.contains(&chapter.number) {
                self.endless.insert(chapter.number);
            }
        }
        for (name, (site, reached)) in std::mem::take(&mut self.named_places.sites) {
            if !reached && self.places.get(&name).is_some_and(Option::is_some) {
                match self.opened.get(&name) {
                    Some(at) if !self.map_shown => {
                        let at = at.clone();
                        self.say(&at, format!("“{name}” is opened on the map, but no “map” ever shows it"));
                    }
                    _ => self.say(&site, format!("nothing the game reaches leads to “{name}” (go, exit, open)")),
                }
                self.walk_place(&name, false);
            }
            self.named_places.sites.insert(name, (site, reached));
        }
        // (A place of the folder with a way to one of the game's, which nothing leads to.)
        let folder = self.game.decors();
        for name in names_in(&folder, "decor") {
            if self.places.contains_key(&name) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(folder.join(format!("{name}.decor"))) else {
                continue;
            };
            let Ok(decor) = decor::lang::parse(&text.replace("\r\n", "\n"), &name) else {
                continue;
            };
            if let Some(way) = decor.ways_out.iter().find(|way| self.named_places.sites.get(&way.to).is_some_and(|(_, reached)| *reached)) {
                let line = locate(&text, 1, &[Some("exit"), None, None, None, None, Some("to"), Some(way.to.as_str())]);
                let site = Site { file: folder.join(format!("{name}.decor")), line };
                self.say(&site, format!("“{name}” has a way to “{}”, but nothing the game reaches leads to it (go, exit, open)", way.to));
            }
        }
        self.contents();
        self.ends();
    }

    /// Whether a place or a dungeon was reached.
    fn is_reached(&self, name: &str) -> bool {
        [&self.named_places, &self.dungeons].iter().any(|named| named.sites.get(name).is_some_and(|(_, reached)| *reached))
    }

    // ----- the chapters -----

    /// Each chapter's reach, chapter by chapter: the places the team may be
    /// in during it (the home, those its actions and rules go to, those it
    /// was in when the chapter before went on to it, and what their exits
    /// and rules lead to), the chapters it may go on to, whether it may end.
    /// A place's rule counts for a chapter only if the place may be reached
    /// in it and its `if` does not rule the chapter out. The chapters
    /// reached from the first, and those that may end.
    fn chapters_reach(&mut self, first: u16) -> (BTreeSet<u16>, BTreeSet<u16>) {
        let numbers: Vec<u16> = self.game.chapters.iter().map(|chapter| chapter.number).collect();
        // (Each place of the game's: its exits, its rules.)
        let mut lives: BTreeMap<String, (Vec<String>, Vec<Rule>)> = BTreeMap::new();
        let names: Vec<String> = self.places.keys().cloned().collect();
        for name in names {
            let Some(read) = self.decor(&name) else { continue };
            let exits = read.decor.ways_out.iter().map(|way| way.to.clone()).collect();
            let mut rules = Vec::new();
            for rule in &read.decor.rules {
                rules.extend(game_file::read_actions(&format!("{}\n{}end\n", rule.head, rule.body), &mut Vec::new()).1);
            }
            lives.insert(name, (exits, rules));
        }
        let project = self.project.clone();
        let is_place = |word: &str| project.decor(word).is_file();
        let mut goes: BTreeMap<u16, Goes> = BTreeMap::new();
        for chapter in &self.game.chapters {
            let mut said = Goes::default();
            said.of(&chapter.actions, chapter.number, &is_place);
            for rule in &chapter.rules {
                said.rule(rule, chapter.number, &is_place);
            }
            if let Some((home, _)) = &self.game.home {
                said.places.insert(home.clone());
            }
            goes.entry(chapter.number).or_default().places.extend(said.places);
            let entry = goes.entry(chapter.number).or_default();
            entry.chapters.extend(said.chapters);
            entry.ends |= said.ends;
        }
        // (Until nothing more is found.)
        loop {
            let mut changed = false;
            for &number in &numbers {
                let before = goes.get(&number).map_or((0, 0, false), |said| (said.places.len(), said.chapters.len(), said.ends));
                // (Where the team was as a chapter before went on to this one.)
                let carried: Vec<String> = goes
                    .iter()
                    .filter(|(other, said)| **other != number && said.chapters.contains(&number))
                    .flat_map(|(_, said)| said.places.clone())
                    .collect();
                let mut said = std::mem::take(goes.entry(number).or_default());
                said.places.extend(carried);
                let mut seen = BTreeSet::new();
                let mut next: Vec<String> = said.places.iter().cloned().collect();
                while let Some(place) = next.pop() {
                    if !seen.insert(place.clone()) {
                        continue;
                    }
                    let Some((exits, rules)) = lives.get(&place) else { continue };
                    let mut more = Goes::default();
                    more.places.extend(exits.iter().cloned());
                    for rule in rules {
                        more.rule(rule, number, &is_place);
                    }
                    next.extend(more.places.iter().filter(|place| !said.places.contains(*place)).cloned());
                    said.places.extend(more.places);
                    said.chapters.extend(more.chapters);
                    said.ends |= more.ends;
                }
                changed |= (said.places.len(), said.chapters.len(), said.ends) != before;
                goes.insert(number, said);
            }
            if !changed {
                break;
            }
        }
        let mut reached = BTreeSet::new();
        let mut next = vec![first];
        while let Some(number) = next.pop() {
            if reached.insert(number) {
                next.extend(goes.get(&number).map(|said| said.chapters.iter().copied().collect::<Vec<_>>()).unwrap_or_default());
            }
        }
        let ends = goes.iter().filter(|(_, said)| said.ends).map(|(number, _)| *number).collect();
        (reached, ends)
    }

    fn walk_chapter(&mut self, number: u16, reached: bool) {
        if reached && !self.chapters.insert(number) {
            return;
        }
        let Some(chapter) = self.game.chapter(number).cloned() else {
            return;
        };
        let (path, text) = (self.path, self.text);
        let within = Within { file: path, text, from: self.chapter_line(number), reached, place: None };
        self.actions(&chapter.actions, &within);
        for rule in &chapter.rules {
            self.rule(rule, &within);
        }
    }

    /// A rule: its test read, whom or where it waits for, its actions.
    fn rule(&mut self, rule: &Rule, within: &Within) {
        let head = match &rule.when {
            When::Enter => vec![Some("on"), Some("enter")],
            When::Talk(who) => vec![Some("on"), Some("talk"), Some(who.as_str())],
            When::Zone(zone) => vec![Some("on"), Some("zone"), Some(zone.as_str())],
        };
        let line = locate(within.text, within.from, &head);
        let site = Site { file: within.file.to_path_buf(), line };
        if let Some(test) = &rule.test {
            self.read(names_of_test(test), &site);
        }
        match (&rule.when, within.place) {
            (When::Talk(who), None) => self.talks.push((who.clone(), site.clone())),
            (When::Zone(zone), None) => self.zones.push((zone.clone(), site.clone())),
            (When::Talk(who), Some(place)) => {
                let residents: Vec<String> =
                    self.decor(place).map(|read| read.decor.residents.iter().map(|one| one.name.clone()).collect()).unwrap_or_default();
                if !residents.contains(who) {
                    self.say(&site, unknown("nobody lives here named", who, residents.iter().map(String::as_str), "resident <name> x y"));
                }
            }
            (When::Zone(zone), Some(place)) => {
                let zones: Vec<String> = self.decor(place).map(|read| read.decor.zones.iter().map(|one| one.name.clone()).collect()).unwrap_or_default();
                if !zones.contains(zone) {
                    self.say(&site, unknown("no zone here named", zone, zones.iter().map(String::as_str), "zone <name> x y width height"));
                }
            }
            (When::Enter, _) => {}
        }
        let inner = Within { from: line, ..within.clone() };
        self.actions(&rule.actions, &inner);
    }

    /// Names read by a test, there.
    fn read(&mut self, names: BTreeSet<String>, site: &Site) {
        for name in names {
            self.reads.entry(name).or_insert_with(|| site.clone());
        }
    }

    // ----- the actions -----

    fn actions(&mut self, actions: &[Action], within: &Within) {
        for action in actions {
            self.action(action, within);
        }
    }

    fn action(&mut self, action: &Action, within: &Within) {
        let at = |words: &[Option<&str>]| Site { file: within.file.to_path_buf(), line: locate(within.text, within.from, words) };
        let reached = within.reached;
        match action {
            Action::Play(film) => {
                self.films.add(film, at(&[Some("play"), Some(film)]), reached);
            }
            Action::Set(name) => {
                let site = at(&[Some("set"), Some(name)]);
                self.sets.entry(name.clone()).or_insert(site);
            }
            Action::Add(name, _) => {
                let site = at(&[Some("add"), Some(name)]);
                self.sets.entry(name.clone()).or_insert(site);
            }
            Action::Clear(_) | Action::Money(_) | Action::Finish => {}
            Action::Chapter(number) => {
                let said = number.to_string();
                let site = at(&[Some("chapter"), Some(said.as_str())]);
                if self.game.chapter(*number).is_none() {
                    let known: Vec<String> = self.game.chapters.iter().map(|chapter| chapter.number.to_string()).collect();
                    self.say(&site, format!("there is no chapter {number} (the chapters: {})", known.join(", ")));
                } else if reached && !self.chapters.contains(number) {
                    self.queue.push_back(Step::Chapter(*number));
                }
            }
            Action::Go(place, entrance) => self.name_place(place, entrance.as_deref(), at(&[Some("go"), Some(place)]), reached),
            Action::Dungeon(word) | Action::Fight(word) => {
                let verb = if matches!(action, Action::Fight(_)) { "fight" } else { "dungeon" };
                let site = at(&[Some(verb), Some(word)]);
                if reached && self.dives.is_none() {
                    self.dives = Some(site.clone());
                }
                self.name_dungeon(word, site, reached);
            }
            Action::Open(word) => {
                let site = at(&[Some("open"), Some(word)]);
                self.opened.entry(word.clone()).or_insert_with(|| site.clone());
                self.open(word, site, reached && self.map_shown);
            }
            Action::Give(count, item) => {
                let site = at(&[Some(if *count < 0 { "take" } else { "give" })]);
                self.item(item, &site);
            }
            Action::Recruit(species) => {
                let tables = self.tables.clone();
                let monsters = &tables.monsters;
                if !monsters.is_empty() && !monsters.contains_key(species.as_str()) {
                    let site = at(&[Some("recruit"), Some(species)]);
                    self.say(&site, unknown("unknown Pokémon", species, monsters.keys().map(String::as_str), "recruit <pokemon>"));
                }
            }
            Action::Shop(screen) => {
                if !decor::lang::TALKS[1..].contains(&screen.as_str()) {
                    let site = at(&[Some("shop"), Some(screen)]);
                    self.say(&site, unknown("no screen", screen, decor::lang::TALKS[1..].iter().copied(), "shop|market|bank|storage|jobs|recruits|appraise"));
                }
            }
            Action::Map => self.map_shown |= reached,
            Action::If(branches) => {
                let site = at(&[Some("if")]);
                for (test, actions) in branches {
                    if let Some(test) = test {
                        self.read(names_of_test(test), &site);
                    }
                    self.actions(actions, within);
                }
            }
        }
    }

    /// An item given or taken: the game's, else a thing of one's own (said
    /// only when it is near an item of the game's: likely a slip).
    fn item(&mut self, item: &str, site: &Site) {
        let Some(known) = &self.known else {
            return;
        };
        if known.items.iter().any(|name| plain(name) == plain(item)) {
            return;
        }
        let near = film::check::closest_of(item, known.items.iter().map(String::as_str));
        if !near.is_empty() {
            self.say(site, format!("“{item}” is not an item of the game (it becomes a thing of one's own). Near: {}", near.join(", ")));
        }
    }

    /// `open <place|dungeon>`: whichever it is.
    fn open(&mut self, word: &str, site: Site, reached: bool) {
        if self.decor(word).is_some() {
            self.name_place(word, None, site, reached);
        } else if self.dungeon_file(word).is_some() {
            self.name_dungeon(word, site, reached);
        } else {
            let mut known = self.project.names("decors", "decor");
            known.extend(self.project.names("dungeons", "dungeon"));
            self.say(&site, unknown("no place or dungeon", word, known.iter().map(String::as_str), "decors/<place>.decor, dungeons/<name>.dungeon"));
        }
    }

    // ----- the places -----

    /// A place read (once), if it is there and reads.
    fn decor(&mut self, name: &str) -> Option<&Read> {
        if !self.places.contains_key(name) {
            let file = self.project.decor(name);
            let read = match std::fs::read_to_string(&file) {
                Ok(text) => {
                    let text = text.replace("\r\n", "\n");
                    match decor::lang::parse(&text, &file.display().to_string()) {
                        Ok(decor) => Some(Read { decor, text, file }),
                        Err(fault) => {
                            let line = if fault.span.file.is_some() || fault.line() > 0 { fault.line() } else { 1 };
                            self.problems.push(told(&file, line, fault.message));
                            None
                        }
                    }
                }
                Err(_) => None,
            };
            self.places.insert(name.to_string(), read);
        }
        self.places.get(name).and_then(Option::as_ref)
    }

    /// A place named there (and an entrance): it must be, and have it.
    fn name_place(&mut self, place: &str, entrance: Option<&str>, site: Site, reached: bool) {
        let file = self.project.decor(place);
        if !file.is_file() {
            let known = self.project.names("decors", "decor");
            self.say(&site, unknown("no place", place, known.iter().map(String::as_str), "decors/<place>.decor"));
            return;
        }
        let entrances: Option<Vec<String>> = self.decor(place).map(|read| read.decor.entrances.iter().map(|(.., name)| name.clone()).collect());
        if let (Some(entrance), Some(entrances)) = (entrance, entrances)
            && !entrances.iter().any(|known| known == entrance)
        {
            self.say(&site, unknown(&format!("“{place}” has no entrance"), entrance, entrances.iter().map(String::as_str), "entrance x y <name>"));
        }
        if self.named_places.add(place, site, reached) {
            self.queue.push_back(Step::Place(place.to_string()));
        }
    }

    /// A place's life: who lives there and when, what they say, where its
    /// ways lead, its rules.
    fn walk_place(&mut self, name: &str, reached: bool) {
        let Some(read) = self.decor(name) else {
            return;
        };
        let (decor, text, file) = (read.decor.clone(), read.text.clone(), read.file.clone());
        let site = |line: usize| Site { file: file.clone(), line };
        for resident in &decor.residents {
            if let Some(condition) = &resident.condition {
                self.test(condition, &site(resident.number));
            }
            // (Its `says` lines, up to the next resident: their tests, in order.)
            let says: Vec<usize> = text
                .lines()
                .enumerate()
                .skip(resident.number)
                .take_while(|(_, line)| line.split_whitespace().next() != Some("resident"))
                .filter(|(_, line)| line.split_whitespace().next() == Some("says"))
                .map(|(at, _)| at + 1)
                .collect();
            for (rank, test) in resident.tests.iter().enumerate() {
                if let Some(test) = test {
                    let line = says.get(rank).copied().unwrap_or(resident.number);
                    self.test(test, &site(line));
                }
            }
            if let Some(word) = resident.talk.as_deref().and_then(|talk| talk.strip_prefix("film ")) {
                let line = locate(&text, resident.number, &[Some("talk"), Some("film")]);
                self.films.add(word, site(line), reached);
            }
        }
        for way in &decor.ways_out {
            let line = locate(&text, 1, &[Some("exit"), None, None, None, None, Some("to"), Some(way.to.as_str())]);
            self.name_place(&way.to, way.entrance.as_deref(), site(line), reached);
        }
        for rule in &decor.rules {
            let mut said = Vec::new();
            let (_, read) = game_file::read_actions(&format!("{}\n{}end\n", rule.head, rule.body), &mut said);
            for wrong in said {
                let line = Some(wrong.line()).filter(|line| *line > 0).unwrap_or(1);
                self.say(&site(rule.line + line - 1), wrong.message);
            }
            let within = Within { file: &file, text: &text, from: rule.line, reached, place: Some(name) };
            for rule in &read {
                self.rule(rule, &within);
            }
        }
    }

    /// A test of a place's, written: it must read; its names are read.
    fn test(&mut self, written: &str, site: &Site) {
        match game_file::test_of(written) {
            Ok(test) => self.read(names_of_test(&test), site),
            Err(why) => self.say(site, format!("not a test: {why}")),
        }
    }

    // ----- the dungeons -----

    /// A dungeon's file by its name, else a fight's (as `dive::text_of` reads them).
    fn dungeon_file(&self, word: &str) -> Option<PathBuf> {
        let dungeon = self.project.dungeon(word);
        let fight = self.project.fight(word);
        [dungeon, fight].into_iter().find(|path| path.is_file())
    }

    fn name_dungeon(&mut self, word: &str, site: Site, reached: bool) {
        if self.dungeon_file(word).is_none() {
            let mut known = self.project.names("dungeons", "dungeon");
            known.extend(self.project.names("fights", "fight"));
            self.say(&site, unknown("no dungeon or fight", word, known.iter().map(String::as_str), "dungeons/<name>.dungeon, fights/<name>.fight"));
            return;
        }
        self.dungeons.add(word, site, reached);
    }

    // ----- what the game names, checked -----

    fn contents(&mut self) {
        // --- The films ---
        let films: Vec<(String, (Site, bool))> = self.films.sites.iter().map(|(name, said)| (name.clone(), said.clone())).collect();
        for (name, (site, reached)) in films {
            let file = self.project.film(&name);
            let Ok(text) = std::fs::read_to_string(&file) else {
                let known = self.project.names("films", "film");
                self.say(&site, unknown("no film", &name, known.iter().map(String::as_str), "films/<film>.film"));
                continue;
            };
            let text = text.replace("\r\n", "\n");
            if !reached {
                self.say(&site, format!("the film “{name}” is never played: only from what the game never reaches"));
            }
            for told in film::service::diagnostics(&text, self.tables.clone()) {
                if told.is_error() {
                    let at = site_of(&file, told.span.start.line.max(1));
                    self.problems.push(Diagnostic { span: Location { bytes: told.span.bytes, ..at }, ..told });
                }
            }
            self.film_flags(&text, &file);
        }
        // --- The dungeons and the fights ---
        let project = self.project.clone();
        let dungeons: Vec<(String, (Site, bool))> = self.dungeons.sites.iter().map(|(name, said)| (name.clone(), said.clone())).collect();
        for (name, (site, reached)) in dungeons {
            if !reached {
                self.say(&site, format!("“{name}” is never gone into: only from what the game never reaches"));
            }
            let (Some(file), Some(book)) = (self.dungeon_file(&name), self.book) else {
                continue;
            };
            let known = self.known.as_ref().map(|known| (known, book));
            if file.extension().is_some_and(|said| said == "fight") {
                let text = std::fs::read_to_string(&file).unwrap_or_default().replace("\r\n", "\n");
                let played = crate::dive::resolved_for(&project, &crate::dive::fight_alone(&text));
                if let Err(wrong) = rules::own::read(&played, &book.tables) {
                    for why in wrong {
                        let (line, message) = fight_line(&text, &why);
                        self.problems.push(told(&file, line, message).coded(why.code_or(codes::GAME)));
                    }
                }
                continue;
            }
            match crate::dungeon_check::of_file(&file, known, &project) {
                Ok(found) => {
                    for problem in found {
                        let message =
                            if problem.is_error() { problem.message.clone() } else { format!("{} (it plays, but likely not as meant)", problem.message) };
                        // (Told as a problem of the game whatever its weight.)
                        self.problems.push(
                            Diagnostic { severity: language::Severity::Error, ..told(&file, problem.line(), message) }.coded(problem.code_or(codes::GAME)),
                        );
                    }
                }
                Err(why) => self.say(&site, why),
            }
        }
    }

    /// What a film sets for the game (`set`, `clear`), reads of it (`keep`),
    /// and reads in its own tests.
    fn film_flags(&mut self, text: &str, file: &Path) {
        let assigned = |name: &str| {
            text.lines().any(|line| {
                let words: Vec<&str> = line.split_whitespace().collect();
                matches!(words.as_slice(), [said, "=" | "+=" | "-=", ..] if *said == name) || matches!(words.as_slice(), ["let", said, ..] if *said == name)
            })
        };
        for (at, line) in text.lines().enumerate() {
            let site = Site { file: file.to_path_buf(), line: at + 1 };
            let words: Vec<&str> = line.split('#').next().unwrap_or_default().split_whitespace().collect();
            match words.as_slice() {
                ["set", name, ..] => {
                    self.sets.entry(name.to_string()).or_insert(site);
                }
                ["keep", name, ..] => {
                    let name = name.trim_end_matches('=');
                    if !assigned(name) {
                        self.reads.entry(name.to_string()).or_insert(site);
                    }
                }
                ["if" | "while" | "until", ..] | ["else", "if", ..] => {
                    self.film_reads.extend(film_test_names(line.split('#').next().unwrap_or_default()));
                }
                _ => {}
            }
        }
    }

    // ----- at the end -----

    fn ends(&mut self) {
        // --- Whom and where the chapters' rules wait for ---
        let read: Vec<&Read> = self.places.values().flatten().collect();
        let residents: BTreeSet<String> = read.iter().flat_map(|read| read.decor.residents.iter().map(|one| one.name.clone())).collect();
        let zones: BTreeSet<String> = read.iter().flat_map(|read| read.decor.zones.iter().map(|one| one.name.clone())).collect();
        for (who, site) in self.talks.clone() {
            if !residents.contains(&who) {
                self.say(
                    &site,
                    unknown("nobody in the game's places is named", &who, residents.iter().map(String::as_str), "resident <name> x y, in a decor's [life]"),
                );
            }
        }
        for (zone, site) in self.zones.clone() {
            if !zones.contains(&zone) {
                self.say(&site, unknown("no zone of the game's places is named", &zone, zones.iter().map(String::as_str), "zone <name> x y width height"));
            }
        }
        // --- The flags ---
        let given = ["chapter", "won"];
        for (name, site) in self.sets.clone() {
            if !self.reads.contains_key(&name) && !self.film_reads.contains(&name) {
                self.say(&site, format!("“{name}” is set but never read (by a test of the game, of a place, or of a film)"));
            }
        }
        for (name, site) in self.reads.clone() {
            if !self.sets.contains_key(&name) && !given.contains(&name.as_str()) {
                let near = film::check::closest_of(&name, self.sets.keys().map(String::as_str));
                let near = if near.is_empty() { String::new() } else { format!(". Near: {}", near.join(", ")) };
                self.say(&site, format!("“{name}” is read but never set (set, add, or a film's set){near}"));
            }
        }
        // --- The chapters with no way to end ---
        for number in self.endless.clone() {
            let site = self.game_site(self.chapter_line(number));
            self.say(
                &site,
                format!("chapter {number} has no way to end: no “chapter …” nor “finish” in it, nor in a rule of its places that plays in it (their `if` on the chapter)"),
            );
        }
        // --- A K.O. with nowhere to come back to ---
        if let (Some(site), None) = (self.dives.clone(), &self.game.home) {
            self.say(&site, "a dungeon, and no “home:” in the game's head: a team K.O. there has no place of its own to come back to");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game of that text, with those decors (name, text), in a folder of its own.
    fn checked(name: &str, game: &str, decors: &[(&str, &str)]) -> Vec<String> {
        let folder = std::env::temp_dir().join(format!("dw_project_check_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("decors")).expect("a folder");
        std::fs::create_dir_all(folder.join("films")).expect("a folder");
        for (place, text) in decors {
            std::fs::write(folder.join("decors").join(format!("{place}.decor")), text).expect("written");
        }
        let path = folder.join(format!("{name}.game"));
        std::fs::write(&path, game).expect("written");
        let report = check(&crate::testing::context_on("no data here"), &path).expect("checked");
        let _ = std::fs::remove_dir_all(&folder);
        report.problems.iter().map(|problem| format!("{}:{}", problem.line(), problem.message)).collect()
    }

    const TOWN: &str = "decor: town 256 192\nentrance 10 96 west\n[life]\nresident kecleon 120 80 down\nsays if not met: Welcome!\n";

    #[test]
    fn a_film_not_there_is_said_with_its_line_and_the_nearest() {
        let game = "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town west\n  play the_arival\n  finish\nend\n";
        let problems = checked("film", game, &[("town", TOWN)]);
        assert!(problems.iter().any(|said| said.starts_with("6:") && said.contains("no film “the_arival”")), "{problems:?}");
    }

    #[test]
    fn a_flag_read_and_never_set_is_said() {
        let game = "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town west\n  on talk kecleon\n    finish\n  end\nend\n";
        let problems = checked("flag", game, &[("town", TOWN)]);
        assert!(problems.iter().any(|said| said.starts_with("5:") && said.contains("“met” is read but never set")), "{problems:?}");
        // (Set, it is read: nothing more to say of it.)
        let game = "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town west\n  on talk kecleon\n    set met\n    finish\n  end\nend\n";
        let problems = checked("flag_set", game, &[("town", TOWN)]);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_place_nothing_leads_to_is_said() {
        let beach = "decor: beach 256 192\nentrance 10 96 west\n[life]\nexit 0 80 16 32 to town west\n";
        let game = "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town west\n  finish\nend\n";
        let problems = checked("reach", game, &[("town", "decor: town 256 192\nentrance 10 96 west\n"), ("beach", beach)]);
        assert!(problems.iter().any(|said| said.starts_with("4:") && said.contains("“beach” has a way to “town”, but nothing")), "{problems:?}");
    }

    #[test]
    fn names_chapters_and_ends_are_checked() {
        let game = "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town east\n  on talk kecleo\n    chapter 3\n  end\nend\n\nchapter 2\n  finish\nend\n";
        let problems = checked("names", game, &[("town", TOWN)]);
        let has = |line: &str, said: &str| problems.iter().any(|problem| problem.starts_with(line) && problem.contains(said));
        assert!(has("5:", "“town” has no entrance “east”. Near: west") || has("5:", "has no entrance “east”"), "{problems:?}");
        assert!(has("6:", "“kecleo”. Near: kecleon"), "{problems:?}");
        assert!(has("7:", "there is no chapter 3"), "{problems:?}");
        assert!(has("11:", "chapter 2 is never reached"), "{problems:?}");
        let problems = checked("endless", "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town west\nend\n", &[("town", TOWN)]);
        assert!(problems.iter().any(|said| said.starts_with("4:") && said.contains("chapter 1 has no way to end")), "{problems:?}");
    }

    /// A town whose zone goes on to chapter 2 in chapter 1 only, and whose
    /// Kecleon ends the game in the chapters that test lets through.
    fn town_ending(test: &str) -> String {
        format!(
            "decor: town 256 192\nentrance 10 96 west\n[life]\nresident kecleon 120 80 down\nzone pier 40 40 32 32\non zone pier if chapter = 1\n  chapter 2\nend\non talk kecleon if {test}\n  finish\nend\n"
        )
    }

    #[test]
    fn a_chapter_ends_only_by_the_rules_that_play_in_it() {
        let game = "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town west\nend\n\nchapter 2\n  go town west\nend\n";
        let has = |problems: &[String], said: &str| problems.iter().any(|problem| problem.contains(said));
        // (Kecleon ends it only from chapter 3, which there is not: chapter 2 never ends; chapter 1 by its zone.)
        let problems = checked("reach_none", game, &[("town", &town_ending("chapter >= 3"))]);
        assert!(has(&problems, "chapter 2 has no way to end"), "{problems:?}");
        assert!(!has(&problems, "chapter 1 has no way to end") && !has(&problems, "never reached"), "{problems:?}");
        // (In chapter 2: it ends.)
        let problems = checked("reach_two", game, &[("town", &town_ending("chapter = 2 and not met"))]);
        assert!(!has(&problems, "no way to end"), "{problems:?}");
        // (The zone's way on rules chapter 1 out: chapter 2 is never reached, chapter 1 never ends.)
        let town = town_ending("chapter = 2").replace("if chapter = 1", "if chapter > 1 or not chapter");
        let problems = checked("reach_never", game, &[("town", &town)]);
        assert!(has(&problems, "chapter 2 is never reached") && has(&problems, "chapter 1 has no way to end"), "{problems:?}");
        // (An exit only in another chapter's own rule does not end this one.)
        let game =
            "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town west\n  on talk kecleon\n    chapter 2\n  end\nend\n\nchapter 2\n  go town west\nend\n";
        let problems = checked("reach_other", game, &[("town", "decor: town 256 192\nentrance 10 96 west\n[life]\nresident kecleon 120 80 down\n")]);
        assert!(has(&problems, "chapter 2 has no way to end") && !has(&problems, "chapter 1 has no way"), "{problems:?}");
    }

    #[test]
    fn what_a_fight_alone_says_is_on_its_own_line() {
        let fight = "# A fight.\nname: The Boss\nboss: Butterfree 12\nboss_hp: lots\n";
        // (Played: “name: The Boss”, “floors: 0”, the comment, the boss, its hp.)
        assert_eq!(fight_line(fight, &Diagnostic::at(5, "boss_hp: x2 or 150%")), (4, "boss_hp: x2 or 150%".to_string()));
        assert_eq!(fight_line(fight, &Diagnostic::at(1, "no name")), (2, "no name".to_string()));
        assert_eq!(fight_line(fight, &Diagnostic::said("\"boss_hp: lots\"?")).0, 4);
    }

    #[test]
    fn a_missing_game_is_said_plainly() {
        let said = check(&crate::testing::context_on("no data"), Path::new("library/no_such_game.game")).expect_err("no game");
        assert!(matches!(said, crate::Error::NoGame(_)));
        assert_eq!(said.to_string(), "library/no_such_game.game: no such game (a .game file: its title, team, chapters)");
    }

    #[test]
    fn the_example_is_clean() {
        let path = crate::testing::root().join("tests/games/test_village/test_village.game");
        let report = check(&crate::testing::context(), &path).expect("checked");
        let problems: Vec<String> = report.problems.iter().map(Diagnostic::to_string).collect();
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
