//! The Script of the Film space: the film's text in an editor as VS Code
//! has one. Each piece of a line in its colour (who speaks, the face, what
//! is said, the words of the language, the names, the times); the lines
//! numbered, those that are wrong marked in the margin and underlined; what
//! the language says of the film found again a moment after each change, on
//! a thread of its own (`film::service::diagnostics`), listed under the text
//! with its ways to put it right; what may be typed offered as one types
//! (Ctrl+Space too); what a word is said when it is pointed at; the plan of
//! the film (its scenes, its moments, its recipes) over the text, where the
//! cursor is in it. Written out by itself a moment after the last change.
//!
//! Keys, the text having them: Ctrl+S writes it, Ctrl+Space offers, F12
//! goes to where a name is given, F2 renames it everywhere, Ctrl+/ comments
//! the line, Ctrl+D duplicates it, Alt+↑ ↓ moves it, Tab and Shift+Tab indent,
//! Enter keeps the line's indent, Shift+Alt+F puts the blocks in step; Ctrl+F
//! finds, Ctrl+H replaces, Ctrl+G goes to a line, Ctrl+= Ctrl+- Ctrl+0 make
//! the letters larger, smaller, as they were.
//!
//! Besides: models of lines unfolded by a word (`scene`, `fight`…, Tab);
//! the word under the cursor lit wherever it is; a ruler on the right (what
//! is wrong, what is found, where the cursor is: a click goes there); what
//! the line's word wants, over the line, as it is written; the line the film
//! plays now marked in the margin, a click on a number taking the film there.
//!
//! And: scenes, moments and blocks folded (their chevron in the margin;
//! Ctrl+Shift+\[ and \]); the line of the scene (and of the moment) kept at the
//! top while what follows is read; a bulb in the margin where a fix is
//! offered (a click, the fixes), all fixed at once; how long each line
//! plays, faint at its end; a model's first `...` chosen to be written.

use std::sync::Arc;

use crate::session::Session;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32};
use film::service::{Diagnostic, Spot};

use edits::replaced;
use text::{index_of, line_at, spot_of};

mod body;
mod cast;
mod completion;
mod editor;
mod edits;
mod head;
mod heard;
mod highlight;
mod keys;
mod knowledge;
mod margin;
mod offers;
mod plan;
mod popups;
mod problems;
#[cfg(test)]
mod tests;
mod text;
mod wiki;
mod windows;

pub use editor::script;
pub(crate) use highlight::speaker;
pub(crate) use knowledge::species_of;

/// How long after a change it is written out, and looked at again.
const WRITTEN_AFTER: Duration = Duration::from_millis(800);
const LOOKED_AFTER: Duration = Duration::from_millis(350);
/// How long the pointer rests on a word before it is said what it is.
const HOVER_AFTER: Duration = Duration::from_millis(450);
/// The margin's width; the most words offered at once.
const GUTTER: f32 = 68.0;
/// The status line's height, under the Script.
const STATUS: f32 = 20.0;
const OFFERED: usize = 12;

// (The colours of VS Code's dark theme.)
const COMMENT: Color32 = Color32::from_rgb(0x6A, 0x99, 0x55);
const LABEL: Color32 = Color32::from_rgb(0xC5, 0x86, 0xC0);
const KEYWORD: Color32 = Color32::from_rgb(0x56, 0x9C, 0xD6);
const LINK: Color32 = Color32::from_rgb(0x8F, 0xA6, 0xC0);
const NAME: Color32 = Color32::from_rgb(0x4E, 0xC9, 0xB0);
const FACE: Color32 = Color32::from_rgb(0x9C, 0xDC, 0xFE);
const SAID: Color32 = Color32::from_rgb(0xCE, 0x91, 0x78);
const NUMBER: Color32 = Color32::from_rgb(0xB5, 0xCE, 0xA8);
const PLAIN: Color32 = Color32::from_rgb(0xD4, 0xD4, 0xD4);
const SYMBOL: Color32 = Color32::from_rgb(0xD7, 0xBA, 0x7D);
const WRONG: Color32 = Color32::from_rgb(0xF1, 0x4C, 0x4C);
const REMARK: Color32 = Color32::from_rgb(0xCC, 0xA7, 0x00);
const TYPO: Color32 = Color32::from_rgb(0x4F, 0xA8, 0xE0);

/// How a finding is shown: an error, a remark, a matter of typography.
fn severity_colour(told: &Diagnostic) -> Color32 {
    match (told.is_error(), told.code) {
        (true, _) => WRONG,
        (false, Some(crate::typography::CODE)) => TYPO,
        _ => REMARK,
    }
}

/// The colour of the worst said of a line: an error before a remark, a remark before typography.
fn worst(said: &[(Range<usize>, Color32)]) -> Color32 {
    [WRONG, REMARK, TYPO].into_iter().find(|shade| said.iter().any(|(_, own)| own == shade)).unwrap_or(REMARK)
}

/// The film open: its text (the one truth), as written last, its revision,
/// the one looked at, what was found wrong (and of which revision).
pub struct Script {
    pub path: PathBuf,
    pub text: String,
    saved: String,
    revision: u64,
    asked: u64,
    told: Vec<Diagnostic>,
    told_for: u64,
    changed: Option<Instant>,
    /// Where the cursor was (a letter's index); a place to put it next (its selection).
    cursor: usize,
    jump: Option<(usize, usize)>,
    /// The root its versions are kept under (`target/workshop/versions`).
    root: PathBuf,
}

impl Script {
    fn read(session: &Session, path: PathBuf) -> Script {
        crate::safe::check(session, &path);
        let text = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
        Script {
            path,
            saved: text.clone(),
            text,
            revision: next_revision(),
            asked: 0,
            told: Vec::new(),
            told_for: 0,
            changed: None,
            cursor: 0,
            jump: None,
            root: session.root().to_path_buf(),
        }
    }

    pub fn saved(&self) -> bool {
        self.text == self.saved
    }

    pub fn save(&mut self) -> Result<(), String> {
        crate::safe::write(&self.path, &self.text).map_err(|why| format!("{}: {why}", self.path.display()))?;
        self.saved = self.text.clone();
        crate::workspace::keep_version(&self.root, &self.path, &self.text);
        Ok(())
    }

    /// Its text made this (from here: what is typed comes by the editor itself).
    fn change(&mut self, made: String) {
        if made != self.text {
            self.text = made;
            self.touched();
        }
    }

    fn touched(&mut self) {
        self.revision = next_revision();
        self.changed = Some(Instant::now());
    }

    /// Its text's revision.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// How long since it last changed.
    pub fn quiet(&self) -> Duration {
        self.changed.map_or(Duration::MAX, |when| when.elapsed())
    }

    /// The line the cursor is on (from 1).
    pub fn cursor_line(&self) -> usize {
        spot_of(&self.text, self.cursor).line
    }

    /// The cursor put at the start of that line (from 1).
    pub fn go_to_line(&mut self, line: usize) {
        let at = index_of(&self.text, Spot { line, column: 1 });
        self.jump = Some((at, at));
    }

    /// Words written where the cursor is (a space before them if the
    /// letter before is not one), the cursor after them.
    pub fn insert(&mut self, words: &str) {
        let before = if self.cursor > 0 { self.text.chars().nth(self.cursor - 1) } else { None };
        let with = if before.is_some_and(|letter| !letter.is_whitespace()) { format!(" {words}") } else { words.to_string() };
        let (made, at) = replaced(&self.text, self.cursor..self.cursor, &with);
        self.text = made;
        self.touched();
        self.cursor = at;
        self.jump = Some((at, at));
    }

    /// A line added under the cursor's, as far in, the cursor at its end.
    pub fn add_line(&mut self, line: &str) {
        let (_, start, current) = line_at(&self.text, self.cursor);
        let lead: String = current.chars().take_while(|letter| letter.is_whitespace()).collect();
        let end = start + current.chars().count();
        let (made, at) = if current.trim().is_empty() {
            replaced(&self.text, start..end, &format!("{lead}{line}"))
        } else {
            replaced(&self.text, end..end, &format!("\n{lead}{line}"))
        };
        self.change(made);
        self.cursor = at;
        self.jump = Some((at, at));
    }

    /// Its text made this, by what is done elsewhere (one change to undo).
    pub fn set_text(&mut self, made: String) {
        self.change(made);
    }

    /// What the language says of a line, whole (its fixes too).
    pub fn told_full(&self, line: usize) -> Vec<Diagnostic> {
        self.told.iter().filter(|told| told.span.start.line == line).cloned().collect()
    }
}

/// A word being typed, and what it may become.
struct Completion {
    /// The letters it takes (letters' indices), from its start to the cursor.
    letters: Range<usize>,
    choices: Vec<(String, String)>,
    chosen: usize,
    /// Its choices are moves, used by that one (its species).
    user: Option<String>,
}

/// What is sought in the text, and what it is replaced with.
struct Finding {
    sought: String,
    with: String,
    replacing: bool,
    /// Its case as written counts.
    case: bool,
    /// The one found to look at; the field to be given the keys.
    at: usize,
    focus: bool,
}

/// The Script's state: the film open, the thread looking at it, what is
/// offered, what the pointer rests on, a name being renamed; what is
/// sought, a line asked for; how large the letters, whether lines wrap,
/// whether the remarks are listed.
pub struct Scripts {
    /// What the Workshop works with (`session`).
    pub session: Arc<Session>,
    pub open: Option<Script>,
    /// A text to check: its revision, its text, how typography is looked at
    /// (1: as typography wants, 2: with a space before ! ? : ;, the French way).
    asks: Sender<(u64, String, u8)>,
    answers: Receiver<(u64, Vec<Diagnostic>)>,
    completion: Option<Completion>,
    hover: Option<(usize, Instant, Option<String>)>,
    renaming: Option<(Spot, String)>,
    problems_open: bool,
    pub said: Option<String>,
    finding: Option<Finding>,
    going_to: Option<String>,
    size: f32,
    wrap: bool,
    remarks_listed: bool,
    /// The folds folded, by their first line; the bulb opened (its line, where).
    folded: BTreeSet<usize>,
    bulb: Option<(usize, egui::Pos2)>,
    /// The words by family, open (Ctrl+Space on an empty line, "Words").
    browsing: Option<Browse>,
    /// The lines marked (Ctrl+B, F4); what is said at the end of each line, or not.
    bookmarks: BTreeSet<usize>,
    hints: bool,
    /// The film asked to play from that line (Ctrl+Enter, ▶ in the margin).
    pub play_from: Option<usize>,
    /// The panels answering each other: the line pointed here; the line, and
    /// who, pointed in the Timeline or on the Scene.
    pub hovered_line: Option<usize>,
    pub lit_line: Option<usize>,
    pub lit_who_scene: Option<String>,
    pub lit_who_track: Option<String>,
    /// The examples played apart (a word chosen, a move offered).
    examples: crate::example::Examples,
    /// The cursors more than the editor's own (Alt+click): what is typed is written at each.
    extra: Vec<usize>,
    /// The Layout and the Wiki beside the text; the family last looked at; what is sought in the Wiki, the page
    /// open there, its example played, its recipe's source shown.
    plan_shown: bool,
    wiki_shown: bool,
    family: usize,
    wiki_sought: String,
    wiki_page: Option<String>,
    wiki_playing: bool,
    wiki_source: bool,
    /// What the Script says of a word a film does not take, by the word (read once).
    wiki_refused: Option<(String, String)>,
    /// The text the folds and bookmarks were last at (they follow the lines as they move).
    marks_text: String,
    /// The line the film plays, last followed in sight.
    followed: Option<usize>,
    /// A recipe being named (its lines, from 0; its name).
    extracting: Option<(usize, usize, String)>,
    /// Another file asked open (a shared one: F12), and the line to go to there.
    pub open_asked: Option<(PathBuf, usize)>,
    line_asked: Option<usize>,
    /// The portraits of those pointed at, by species (and face: `species:face`).
    portraits: HashMap<String, Option<egui::TextureHandle>>,
    /// Writing a dialogue: Enter after a line said begins the other's; Tab, someone else of the scene.
    dialogue: bool,
    /// What is said looked at as typography wants; with a space before ! ? : ; (the French way).
    typography: bool,
    french_spaces: bool,
    /// The film seen as text, or as a story (a screenplay, a board, its characters, a graph); the card dragged; the
    /// characters' sheets as written.
    view: crate::story::View,
    dragged_scene: Option<usize>,
    sheets: HashMap<String, String>,
    /// The lines the film stops at as it plays (F9); a step asked (F10); the text to play with other values; the
    /// values typed for those declared.
    pub breakpoints: BTreeSet<usize>,
    pub step_asked: bool,
    pub override_text: Option<String>,
    values: BTreeMap<String, f64>,
    /// The project: what is sought in all its films; a film opened by its name (or put beside); the films opened,
    /// as tabs; the one beside; the versions shown (the one chosen); the notes of the margin (one being written).
    project_search: Option<ProjectSearch>,
    quick_open: Option<(String, bool)>,
    pub recent: Vec<PathBuf>,
    beside: Option<PathBuf>,
    versions_shown: bool,
    version_chosen: Option<String>,
    notes: BTreeMap<usize, crate::workspace::Note>,
    note_editing: Option<(usize, String)>,
    /// A sound or a music heard from its line (the line); the sound card; a number dragged with Alt; the text alone
    /// on the screen (F11), the line kept in the middle.
    heard: Option<usize>,
    audio: Option<Box<dyn shell::sound::Hears>>,
    scrub: Option<Scrub>,
    pub concentration: bool,
    centered: Option<(usize, u8)>,
}

/// A number dragged with Alt: where it is (letters), as long, its value when taken, its decimals, where the pointer
/// was, whether it moved yet.
struct Scrub {
    start: usize,
    len: usize,
    value: f64,
    decimals: usize,
    from_x: f32,
    moved: bool,
}

/// Words sought in all the project's films: what, by what replaced, whole words, the case; what was found (and
/// for what); a replacement asked once (to be asked again).
struct ProjectSearch {
    sought: String,
    with: String,
    whole: bool,
    case: bool,
    hits: Vec<crate::workspace::Hit>,
    searched: (String, bool, bool),
    asked: bool,
}

/// What the film is, as the Script shows it: the line it plays (and whether
/// it plays), when each line plays and how long, how long it is.
pub struct Seen {
    /// The film's numbers at the moment shown.
    pub numbers: BTreeMap<String, f64>,
    pub line: Option<usize>,
    /// The line it stopped at, stepping by line (lit, kept in sight).
    pub stepped: Option<usize>,
    pub playing: bool,
    pub timing: BTreeMap<usize, (f64, f64)>,
    pub end: Option<f64>,
}

/// The words by family, as they are browsed: the family, the word chosen;
/// moved by the keys (to be scrolled to); just opened.
struct Browse {
    family: usize,
    chosen: usize,
    moved: bool,
    fresh: bool,
}

impl Scripts {
    pub fn new(session: &Arc<Session>, context: &egui::Context) -> Scripts {
        let (asks, asked) = channel::<(u64, String, u8)>();
        let (answer, answers) = channel::<(u64, Vec<Diagnostic>)>();
        let waker = context.clone();
        let films = session.context.clone();
        std::thread::spawn(move || {
            while let Ok(mut last) = asked.recv() {
                // (Of the texts asked meanwhile, the last only.)
                while let Ok(newer) = asked.try_recv() {
                    last = newer;
                }
                let mut told = film::service::diagnostics(&last.1, films.tables());
                let typography = last.2;
                if typography & 1 != 0 {
                    told.extend(crate::typography::checked(&last.1, typography & 2 != 0));
                }
                if answer.send((last.0, told)).is_err() {
                    break;
                }
                waker.request_repaint();
            }
        });
        Scripts {
            session: session.clone(),
            open: None,
            asks,
            answers,
            completion: None,
            hover: None,
            renaming: None,
            problems_open: true,
            said: None,
            finding: None,
            going_to: None,
            size: 13.0,
            wrap: true,
            remarks_listed: true,
            folded: BTreeSet::new(),
            bulb: None,
            browsing: None,
            bookmarks: BTreeSet::new(),
            hints: true,
            play_from: None,
            hovered_line: None,
            lit_line: None,
            lit_who_scene: None,
            lit_who_track: None,
            examples: crate::example::Examples::new(session, context),
            extra: Vec::new(),
            plan_shown: false,
            // (For a picture: `WORKSHOP_WIKI=walk`, `walk:play`, `joy:source`, `search:rain`, `all`.)
            wiki_shown: session.launch.wiki.is_some(),
            family: 0,
            wiki_sought: session.launch.wiki.as_deref().and_then(|asked| asked.strip_prefix("search:")).unwrap_or_default().to_string(),
            wiki_page: (session.launch.wiki.as_deref())
                .filter(|asked| !asked.starts_with("search:") && *asked != "all" && !asked.is_empty())
                .map(|asked| asked.split(':').next().unwrap_or_default().to_string()),
            wiki_playing: session.launch.wiki.as_deref().is_some_and(|asked| asked.ends_with(":play")),
            wiki_source: session.launch.wiki.as_deref().is_some_and(|asked| asked.ends_with(":source")),
            wiki_refused: None,
            marks_text: String::new(),
            followed: None,
            extracting: None,
            open_asked: None,
            line_asked: None,
            portraits: HashMap::new(),
            dialogue: false,
            typography: true,
            french_spaces: false,
            // (For a picture of it: `WORKSHOP_VIEW=Board`.)
            view: (session.launch.view.clone())
                .and_then(|asked| crate::story::View::ALL.into_iter().find(|view| view.name() == asked))
                .unwrap_or(crate::story::View::Text),
            dragged_scene: None,
            sheets: HashMap::new(),
            breakpoints: BTreeSet::new(),
            step_asked: false,
            override_text: None,
            values: BTreeMap::new(),
            // (For a picture: `WORKSHOP_PROJECT=search:word`, `beside:film`, `versions`, `open`.)
            project_search: session.launch.project.as_deref().and_then(|asked| asked.strip_prefix("search:").map(str::to_string)).map(|sought| ProjectSearch {
                sought,
                with: String::new(),
                whole: true,
                case: false,
                hits: Vec::new(),
                searched: (String::new(), false, false),
                asked: false,
            }),
            quick_open: (session.launch.project.as_deref() == Some("open")).then(|| (String::new(), false)),
            recent: Vec::new(),
            beside: (session.launch.project.as_deref())
                .and_then(|asked| asked.strip_prefix("beside:").map(|name| session.library().films().join(format!("{name}.film")))),
            versions_shown: session.launch.project.as_deref() == Some("versions"),
            version_chosen: (session.launch.project.as_deref() == Some("versions")).then(|| "commit".to_string()),
            notes: BTreeMap::new(),
            note_editing: None,
            heard: None,
            audio: None,
            scrub: None,
            concentration: session.launch.focus,
            centered: None,
        }
    }

    /// The line to go to in the next file opened.
    pub fn ask_line(&mut self, line: usize) {
        self.line_asked = Some(line);
    }

    /// That film open (if it is not yet).
    pub fn open(&mut self, path: &PathBuf) {
        if self.open.as_ref().is_none_or(|open| &open.path != path) {
            self.open = Some(Script::read(&self.session, path.clone()));
            (self.completion, self.hover, self.renaming) = (None, None, None);
            self.folded.clear();
            self.bookmarks.clear();
            self.breakpoints.clear();
            self.recent.retain(|known| known != path);
            self.recent.insert(0, path.clone());
            self.recent.truncate(10);
            self.notes = crate::workspace::read_notes(self.session.library(), path);
            self.note_editing = None;
            self.version_chosen.take_if(|chosen| chosen != "commit");
            self.marks_text.clear();
            self.extra.clear();
            if let (Some(line), Some(open)) = (self.line_asked.take(), self.open.as_mut()) {
                open.go_to_line(line);
            }
            // (`WORKSHOP_LINE=n`: the cursor on that line, for a picture.)
            if let (Some(line), Some(open)) = (self.session.launch.line, self.open.as_mut()) {
                open.go_to_line(line);
                open.cursor = index_of(&open.text, Spot { line, column: 1 });
            }
            // (`WORKSHOP_PANELS=layout,wiki`: those beside the text, for a picture.)
            let beside = self.session.launch.panels.clone().unwrap_or_default();
            (self.plan_shown, self.wiki_shown) = (self.plan_shown || beside.contains("layout"), self.wiki_shown || beside.contains("wiki"));
            // (`WORKSHOP_WORDS=family`: the words by family open, on that family.)
            if let Some(family) = self.session.launch.words.clone() {
                let family = crate::catalogue::FAMILIES.iter().position(|name| film::words::plain(name).starts_with(&film::words::plain(&family))).unwrap_or(0);
                self.browsing = Some(Browse { family, chosen: 0, moved: true, fresh: true });
            }
        }
    }

    /// Each frame: what was found wrong taken; written out a moment after the
    /// last change; looked at again.
    pub fn tick(&mut self, context: &egui::Context) {
        while let Ok((revision, told)) = self.answers.try_recv() {
            if let Some(open) = &mut self.open
                && revision >= open.told_for
            {
                (open.told, open.told_for) = (told, revision);
            }
        }
        let Some(open) = &mut self.open else { return };
        let quiet = open.changed.map_or(Duration::MAX, |when| when.elapsed());
        if open.asked != open.revision && quiet >= LOOKED_AFTER {
            _ = self.asks.send((open.revision, open.text.clone(), u8::from(self.typography) | u8::from(self.french_spaces) << 1));
            open.asked = open.revision;
        }
        if quiet >= WRITTEN_AFTER && !open.saved() {
            open.changed = None;
            if let Err(why) = open.save() {
                self.said = Some(why);
            }
        }
        if open.changed.is_some() || open.asked != open.told_for {
            context.request_repaint_after(Duration::from_millis(120));
        }
    }

    /// How many are wrong, how many remarked upon (of the last look).
    pub fn counts(&self) -> (usize, usize) {
        self.open.as_ref().map_or((0, 0), |open| {
            let wrong = open.told.iter().filter(|told| told.is_error()).count();
            (wrong, open.told.len() - wrong)
        })
    }
}

/// A revision no file has had (two films opened one after the other never share one).
fn next_revision() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
