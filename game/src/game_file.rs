//! The game's file (`<name>.game`) and what it says: its head (`title:`,
//! `team:`, `home:`…), its chapters, each with what happens when it starts
//! and the rules it holds (`on talk kecleon` … `end`); and the actions, one
//! small vocabulary the chapters and the places' life share (`play`, `set`,
//! `chapter`, `go`, `dungeon`…). Read by the language's parser
//! (`language::parse_with`): `if`, `else` and `end` as in the films.
//!
//! A chapter opens at the start of a line (`chapter 2 "The Forest"`); the
//! same word indented is the action that goes to that chapter.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use language::Diagnostic;
pub use language::ast::Expr;
use language::ast::{BinOp, Block, ExprKind, Kind, Opening, PartKind, Stmt, UnOp};

/// The words of the actions, and what each does.
pub const ACTIONS: &[(&str, &str)] = &[
    ("play", "play <film>: the film plays (from the project's films/), then the game goes on"),
    ("set", "set <flag>: the flag is up (a film reads it as 1)"),
    ("clear", "clear <flag>: the flag is down"),
    ("add", "add <counter> <n>: the counter goes up by n (down, below 0)"),
    ("chapter", "chapter <n>: the story goes on to that chapter (what it does at its start plays)"),
    ("go", "go <place> [entrance]: the team is in that place (a decor of the project's), by that entrance"),
    ("dungeon", "dungeon <name>: the team goes through that dungeon of one's own; then `if won`"),
    ("fight", "fight <name>: that boss fight of one's own, alone; then `if won`"),
    ("open", "open <place|dungeon>: on the world map from now on"),
    ("give", "give [n] <item>: into the bag"),
    ("take", "take [n] <item>: out of the bag"),
    ("money", "money <n>: so many Poké given (taken, below 0)"),
    ("recruit", "recruit <pokemon>: that Pokémon joins the team's recruits (as the rules make one of its species), kept in the save"),
    ("shop", "shop <shop|market|bank|storage|jobs|recruits|appraise|cafe>: that screen of the game"),
    ("map", "map [dungeons]: the world map, the places and dungeons opened (`map dungeons`: the dungeons only); the one chosen is gone to"),
    ("finish", "finish: the game ends (The End)"),
];

/// What makes a rule play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum When {
    /// The team comes into the place.
    Enter,
    /// The player talks to that one.
    Talk(String),
    /// The player walks into that zone of the place.
    Zone(String),
    /// That dungeon (or fight) was cleared, however it was gone into (`dungeon`, the map).
    Won(String),
    /// The team fell in that dungeon (or fight), and was taken home.
    Lost(String),
}

/// `on talk kecleon [if …]` … `end`: what plays when that happens.
#[derive(Clone, Debug)]
pub struct Rule {
    pub when: When,
    pub test: Option<Expr>,
    pub actions: Vec<Action>,
}

/// One thing the game does.
#[derive(Clone, Debug)]
pub enum Action {
    Play(String),
    Set(String),
    Clear(String),
    Add(String, i32),
    Chapter(u16),
    Go(String, Option<String>),
    Dungeon(String),
    Fight(String),
    Open(String),
    Give(i32, String),
    Money(i32),
    Recruit(String),
    Shop(String),
    /// `map`, `map dungeons`: the world map, of the dungeons only when it says so.
    Map {
        dungeons_only: bool,
    },
    Finish,
    /// `if … else if … else … end`: each test and what it plays (none: `else`).
    If(Vec<(Option<Expr>, Vec<Action>)>),
}

/// A chapter of the story: its number, its name, what plays as it starts,
/// its rules.
#[derive(Clone, Debug, Default)]
pub struct Chapter {
    pub number: u16,
    pub title: String,
    pub actions: Vec<Action>,
    pub rules: Vec<Rule>,
}

/// A game, as its file says it.
#[derive(Clone, Debug, Default)]
pub struct GameFile {
    /// Its word (its file's name), the folder it is in (its films, decors…).
    pub name: String,
    pub folder: PathBuf,
    pub title: String,
    pub author: String,
    /// The hero and the partner, by their names.
    pub team: Vec<String>,
    /// Where the team comes back to, K.O. in a dungeon: a place, an entrance.
    pub home: Option<(String, Option<String>)>,
    pub chapters: Vec<Chapter>,
    /// What does not read, a line each (`line n: …`).
    pub problems: Vec<Diagnostic>,
}

/// A `key: value` of the head, as the language reads it (a said line).
fn head_value(stmt: &Stmt) -> Option<(String, String)> {
    match &stmt.kind {
        Kind::Title(title) => Some(("title".to_string(), title.trim().to_string())),
        Kind::Said(said) => Some((said.who.text.clone(), said.text.trim().to_string())),
        _ => None,
    }
}

/// A place and maybe an entrance: `town south`.
fn place_said(value: &str) -> Option<(String, Option<String>)> {
    let mut words = value.split_whitespace();
    let place = words.next()?.to_string();
    Some((place, words.next().map(str::to_string)))
}

/// The line a byte is on, from 1.
fn line_of(text: &str, at: usize) -> usize {
    text[..at.min(text.len())].matches('\n').count() + 1
}

/// Whether a line opens a block of the game's: `on …` anywhere, `chapter …`
/// at the start of a line.
fn holds(text: &str) -> impl Fn(&language::ast::Call) -> bool + '_ {
    move |call| match call.word.text.as_str() {
        "on" => true,
        "chapter" => call.word.span.start == 0 || text[..call.word.span.start].ends_with('\n'),
        _ => false,
    }
}

impl GameFile {
    /// The game of that file.
    pub fn open(path: &Path) -> Result<GameFile, crate::Error> {
        let text = std::fs::read_to_string(path).map_err(|why| crate::Error::file(path, why))?;
        let mut game = GameFile::read(&text);
        game.name = path.file_stem().map_or_else(|| "game".to_string(), |stem| stem.to_string_lossy().into_owned());
        game.folder = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
        Ok(game)
    }

    /// A game from its text.
    pub fn read(text: &str) -> GameFile {
        let parsed = language::parse_with(text, &holds(text));
        let mut game = GameFile::default();
        for diagnostic in &parsed.diagnostics {
            game.problems.push(Diagnostic {
                severity: language::Severity::Error,
                span: language::Location::line(line_of(text, diagnostic.bytes().start)),
                ..diagnostic.clone()
            });
        }
        for stmt in &parsed.file.head {
            if let Some((key, value)) = head_value(stmt) {
                match key.as_str() {
                    "title" => game.title = value,
                    "author" => game.author = value,
                    "team" => game.team = value.split_whitespace().map(str::to_string).collect(),
                    "home" => game.home = place_said(&value),
                    _ => game
                        .problems
                        .push(Diagnostic::at(line_of(text, stmt.span.start), format!("“{key}:” is not a line of a game's head (title, author, team, home)"))),
                }
                continue;
            }
            match &stmt.kind {
                Kind::Blank | Kind::Comment(_) => {}
                Kind::Block(block) if matches!(&block.opening, Opening::Word(call) if call.word.text == "chapter") => {
                    let Opening::Word(call) = &block.opening else { unreachable!() };
                    let number = call.args.first().and_then(|arg| arg.text.parse::<u16>().ok());
                    let Some(number) = number else {
                        game.problems.push(Diagnostic::at(line_of(text, stmt.span.start), "chapter <number> [\"its name\"]"));
                        continue;
                    };
                    let title = call.args[1..]
                        .iter()
                        .map(|arg| match &arg.kind {
                            language::ast::ArgKind::Text(said) => said.clone(),
                            _ => arg.text.clone(),
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let mut chapter = Chapter { number, title, ..Chapter::default() };
                    let (actions, rules) = read_body(text, &block.body, &mut game.problems);
                    (chapter.actions, chapter.rules) = (actions, rules);
                    game.chapters.push(chapter);
                }
                _ => game.problems.push(Diagnostic::at(
                    line_of(text, stmt.span.start),
                    "before the chapters, only the head (title:, author:, team:, home:); then “chapter 1” … “end”",
                )),
            }
        }
        if !parsed.file.scenes.is_empty() {
            game.problems.push(Diagnostic::said("a game has chapters, not scenes: “chapter 1” … “end”"));
        }
        game
    }

    /// The chapter of that number.
    pub fn chapter(&self, number: u16) -> Option<&Chapter> {
        self.chapters.iter().find(|chapter| chapter.number == number)
    }

    /// The first chapter.
    pub fn first(&self) -> Option<&Chapter> {
        self.chapters.iter().min_by_key(|chapter| chapter.number)
    }

    /// Every name its rules and actions read or set (the flags, the
    /// counters): what a film is given and gives back.
    pub fn names(&self) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for chapter in &self.chapters {
            names_of(&chapter.actions, &mut names);
            for rule in &chapter.rules {
                names_in_rule(rule, &mut names);
            }
        }
        names
    }

    /// Where its films are.
    pub fn films(&self) -> PathBuf {
        self.folder.join("films")
    }

    /// Where its places are.
    pub fn decors(&self) -> PathBuf {
        self.folder.join("decors")
    }
}

/// The names a rule reads and sets.
pub fn names_in_rule(rule: &Rule, names: &mut BTreeSet<String>) {
    if let Some(test) = &rule.test {
        expr_names(test, names);
    }
    names_of(&rule.actions, names);
}

/// The names actions read and set.
pub fn names_of(actions: &[Action], names: &mut BTreeSet<String>) {
    for action in actions {
        match action {
            Action::Set(name) | Action::Clear(name) | Action::Add(name, _) => {
                names.insert(name.clone());
            }
            Action::If(branches) => {
                for (test, actions) in branches {
                    if let Some(test) = test {
                        expr_names(test, names);
                    }
                    names_of(actions, names);
                }
            }
            _ => {}
        }
    }
}

/// The names a test reads.
pub fn expr_names(expr: &Expr, names: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name(name) if !["chapter", "won"].contains(&name.as_str()) => {
            names.insert(name.clone());
        }
        ExprKind::Unary(_, inner) => expr_names(inner, names),
        ExprKind::Binary(_, left, right) => {
            expr_names(left, names);
            expr_names(right, names);
        }
        _ => {}
    }
}

/// The actions and rules of a text of them alone (a place's `on … end`).
pub fn read_actions(text: &str, problems: &mut Vec<Diagnostic>) -> (Vec<Action>, Vec<Rule>) {
    let parsed = language::parse_with(text, &holds(text));
    for diagnostic in &parsed.diagnostics {
        problems.push(Diagnostic {
            severity: language::Severity::Error,
            span: language::Location::line(line_of(text, diagnostic.bytes().start)),
            ..diagnostic.clone()
        });
    }
    read_body(text, &parsed.file.head, problems)
}

/// A test as written (`chapter >= 2 and not met`).
pub fn test_of(written: &str) -> Result<Expr, String> {
    language::expr::expression(written.trim(), 0).map_err(|wrong| format!("“{}”: {}", written.trim(), wrong.message))
}

/// The lines of a block: its actions, its rules.
fn read_body(text: &str, body: &[Stmt], problems: &mut Vec<Diagnostic>) -> (Vec<Action>, Vec<Rule>) {
    let (mut actions, mut rules) = (Vec::new(), Vec::new());
    for stmt in body {
        let line = line_of(text, stmt.span.start);
        match &stmt.kind {
            Kind::Blank | Kind::Comment(_) => {}
            Kind::Call(call) => match action(call) {
                Ok(action) => actions.push(action),
                Err(why) => problems.push(Diagnostic::at(line, why)),
            },
            Kind::Block(block) => match &block.opening {
                Opening::If(_) => actions.push(Action::If(branches(text, block, problems))),
                Opening::Word(call) if call.word.text == "on" => match rule(text, stmt, block, problems) {
                    Ok(rule) => rules.push(rule),
                    Err(why) => problems.push(Diagnostic::at(line, why)),
                },
                Opening::Word(call) if call.word.text == "chapter" => {
                    problems.push(Diagnostic::at(line, "a chapter in a chapter: close the first with “end” (to go to a chapter, indent “chapter n”)"))
                }
                _ => problems.push(Diagnostic::at(line, format!("“{}” is not a block of a game (if, on … end)", block.opening.word()))),
            },
            _ => problems
                .push(Diagnostic::at(line, format!("not an action of a game ({})", ACTIONS.iter().map(|(word, _)| *word).collect::<Vec<_>>().join(", ")))),
        }
    }
    (actions, rules)
}

/// `if … else … end`, as tests and actions.
fn branches(text: &str, block: &Block, problems: &mut Vec<Diagnostic>) -> Vec<(Option<Expr>, Vec<Action>)> {
    let Opening::If(test) = &block.opening else {
        return Vec::new();
    };
    let mut said = vec![(Some(test.clone()), read_body(text, &block.body, problems).0)];
    for part in &block.parts {
        let test = match &part.kind {
            PartKind::ElseIf(test) => Some(test.clone()),
            _ => None,
        };
        said.push((test, read_body(text, &part.body, problems).0));
    }
    said
}

/// `on enter`, `on talk kecleon`, `on zone pier`, `on won forest`, `on lost forest` (`if …` after) … `end`.
fn rule(text: &str, stmt: &Stmt, block: &Block, problems: &mut Vec<Diagnostic>) -> Result<Rule, String> {
    let written = &text[block.opening_span.start..block.opening_span.end.min(text.len())];
    let (head, test) = match written.split_once(" if ") {
        Some((head, test)) => (head, Some(test_of(test)?)),
        None => (written, None),
    };
    let words: Vec<&str> = head.split_whitespace().skip(1).collect();
    let when = match words.as_slice() {
        ["enter"] => When::Enter,
        ["talk", who] => When::Talk(who.to_string()),
        ["zone", name] => When::Zone(name.to_string()),
        ["won", dungeon] => When::Won(dungeon.to_string()),
        ["lost", dungeon] => When::Lost(dungeon.to_string()),
        _ => return Err("on enter | on talk <resident> | on zone <name> | on won <dungeon> | on lost <dungeon> [if …] … end".to_string()),
    };
    let _ = stmt;
    let (actions, inner) = read_body(text, &block.body, problems);
    if !inner.is_empty() {
        problems.push(Diagnostic::at(line_of(text, block.opening_span.start), "a rule in a rule: “on” goes in a chapter or a place"));
    }
    Ok(Rule { when, test, actions })
}

/// A word of a line, as written (a text without its quotes).
fn arg_words(call: &language::ast::Call) -> Vec<String> {
    call.args
        .iter()
        .map(|arg| match &arg.kind {
            language::ast::ArgKind::Text(said) => said.clone(),
            _ => arg.text.clone(),
        })
        .collect()
}

/// One action line.
fn action(call: &language::ast::Call) -> Result<Action, String> {
    let words = arg_words(call);
    let word = call.word.text.as_str();
    let one = || words.first().cloned().ok_or_else(|| usage(word));
    let number = |at: usize| words.get(at).and_then(|said| said.trim_start_matches('+').parse::<i32>().ok());
    Ok(match word {
        "play" => Action::Play(one()?),
        "set" => Action::Set(one()?),
        "clear" => Action::Clear(one()?),
        "add" => Action::Add(one()?, number(1).unwrap_or(1)),
        "chapter" => Action::Chapter(words.first().and_then(|said| said.parse().ok()).ok_or_else(|| usage(word))?),
        "go" => Action::Go(one()?, words.get(1).cloned()),
        "dungeon" => Action::Dungeon(one()?),
        "fight" => Action::Fight(one()?),
        "open" => Action::Open(one()?),
        "give" | "take" => {
            let (count, at) = match number(0) {
                Some(count) => (count, 1),
                None => (1, 0),
            };
            let name = words[at.min(words.len())..].join(" ");
            if name.is_empty() {
                return Err(usage(word));
            }
            Action::Give(if word == "take" { -count } else { count }, name)
        }
        "money" => Action::Money(number(0).ok_or_else(|| usage(word))?),
        "recruit" => Action::Recruit(one()?),
        "shop" => Action::Shop(one()?),
        "map" => match words.first().map(String::as_str) {
            None => Action::Map { dungeons_only: false },
            Some("dungeons") => Action::Map { dungeons_only: true },
            Some(_) => return Err(usage(word)),
        },
        "finish" => Action::Finish,
        _ => {
            let known: Vec<&str> = ACTIONS.iter().map(|(word, _)| *word).collect();
            return Err(format!("“{word}” is not an action of a game ({})", known.join(", ")));
        }
    })
}

/// How an action is written.
fn usage(word: &str) -> String {
    ACTIONS.iter().find(|(known, _)| *known == word).map_or_else(|| word.to_string(), |(_, said)| said.to_string())
}

/// What a test comes to, its names read by `get` (a truth: 1 or 0).
pub fn value(expr: &Expr, get: &dyn Fn(&str) -> f64) -> f64 {
    let truth = |yes: bool| if yes { 1.0 } else { 0.0 };
    match &expr.kind {
        ExprKind::Number(number) => *number,
        ExprKind::Bool(yes) => truth(*yes),
        ExprKind::Name(name) => get(name),
        ExprKind::Unary(UnOp::Not, inner) => truth(value(inner, get) == 0.0),
        ExprKind::Unary(UnOp::Minus, inner) => -value(inner, get),
        ExprKind::Binary(op, left, right) => {
            let (a, b) = (value(left, get), value(right, get));
            match op {
                BinOp::Or => truth(a != 0.0 || b != 0.0),
                BinOp::And => truth(a != 0.0 && b != 0.0),
                BinOp::Less => truth(a < b),
                BinOp::LessEq => truth(a <= b),
                BinOp::More => truth(a > b),
                BinOp::MoreEq => truth(a >= b),
                BinOp::Equal => truth(a == b),
                BinOp::NotEqual => truth(a != b),
                BinOp::Plus => a + b,
                BinOp::Minus => a - b,
                BinOp::Times => a * b,
                BinOp::Divide if b != 0.0 => a / b,
                BinOp::Rest if b != 0.0 => a % b,
                _ => 0.0,
            }
        }
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: &str = "title: Trial\nteam: pikachu piplup\nhome: town south\n\nchapter 1 \"The start\"\n  play intro\n  dungeon forest\n  if won\n    set forest_done\n    chapter 2\n  end\nend\n\nchapter 2\n  go town south\n  on talk kecleon if not met\n    set met\n  end\nend\n";

    #[test]
    fn a_game_reads_its_head_and_its_chapters() {
        let game = GameFile::read(GAME);
        assert!(game.problems.is_empty(), "{:?}", game.problems);
        assert_eq!((game.title.as_str(), game.team.len()), ("Trial", 2));
        assert_eq!(game.home, Some(("town".to_string(), Some("south".to_string()))));
        assert_eq!(game.chapters.len(), 2);
        let first = game.chapter(1).expect("chapter 1");
        assert_eq!(first.title, "The start");
        assert!(matches!(first.actions.as_slice(), [Action::Play(_), Action::Dungeon(_), Action::If(_)]), "{:?}", first.actions);
        let second = game.chapter(2).expect("chapter 2");
        assert_eq!(second.rules.len(), 1);
        assert_eq!(second.rules[0].when, When::Talk("kecleon".to_string()));
        let met = |yes: f64| move |name: &str| if name == "met" { yes } else { 0.0 };
        let test = second.rules[0].test.as_ref().expect("a test");
        assert_eq!((value(test, &met(0.0)), value(test, &met(1.0))), (1.0, 0.0));
        assert_eq!(game.names().into_iter().collect::<Vec<_>>(), ["forest_done", "met"]);
    }

    #[test]
    fn a_dungeon_won_or_lost_has_its_rules_and_the_map_its_dungeons() {
        let game = GameFile::read(
            "title: X\nchapter 1\n  open forest\n  on zone trails\n    map dungeons\n  end\n  on won forest if not done\n    set done\n  end\n  on lost forest\n    map\n  end\nend\n",
        );
        assert!(game.problems.is_empty(), "{:?}", game.problems);
        let rules = &game.chapter(1).expect("chapter 1").rules;
        let whens: Vec<&When> = rules.iter().map(|rule| &rule.when).collect();
        assert_eq!(whens, [&When::Zone("trails".into()), &When::Won("forest".into()), &When::Lost("forest".into())]);
        assert!(matches!(rules[0].actions.as_slice(), [Action::Map { dungeons_only: true }]), "{:?}", rules[0].actions);
        assert!(matches!(rules[2].actions.as_slice(), [Action::Map { dungeons_only: false }]), "{:?}", rules[2].actions);
        assert!(rules[1].test.is_some());
        assert!(!GameFile::read("title: X\nchapter 1\n  map everywhere\nend\n").problems.is_empty());
    }

    #[test]
    fn what_does_not_read_is_said_with_its_line() {
        let game = GameFile::read("title: X\nchapter 1\n  dance wildly\nend\n");
        assert!(game.problems.iter().any(|problem| problem.to_string().starts_with("line 3:") && problem.says("dance")), "{:?}", game.problems);
    }
}
