//! The first launch: no game data yet. Dungeon Workshop holds none of a game's
//! pictures, sounds or texts; it reads them from the user's own copy of
//! *Explorers of Sky*, once extracted. This screen says so, asks for that
//! copy (chosen in a dialog, dropped on the window, or its path written),
//! checks it is one the tool reads, then makes the data folder, step by step,
//! each shown as it goes: the cartridge read by the `extractor` program
//! beside the Workshop, then (in the Workshop itself) the dungeons' floors,
//! the pieces of the game's places, and every place of the library and of
//! the games built. Once done, the Workshop opens on an example game.
//!
//! What it goes through is a small state machine of its own (`Phase`,
//! `Event`), and what can go wrong is said for a person (`Wrong`,
//! `explained`, `missing_program`): all tested without a window.

use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

use eframe::egui::{self, RichText};

use crate::session::Session;
use crate::theme;

/// Whether the game's data is there to work with.
pub fn has_data(session: &Session) -> bool {
    assets::data::has(session.data(), "dungeons.bin")
}

/// About how much room the data takes on the disk, said before it starts.
const ROOM: &str = "about 2.5 GB";

// ----- the cartridge -----

/// The releases of *Explorers of Sky* the tool reads (its texts are read in English).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Region {
    NorthAmerica,
    Europe,
}

impl Region {
    pub fn said(self) -> &'static str {
        match self {
            Region::NorthAmerica => "Explorers of Sky, North America (C2SE)",
            Region::Europe => "Explorers of Sky, Europe (C2SP)",
        }
    }
}

/// Why a file is not a cartridge the tool can read.
#[derive(Clone, Debug, PartialEq)]
pub enum Wrong {
    /// No file there.
    Missing,
    /// Not a Nintendo DS cartridge's dump at all.
    NotNds,
    /// A DS game, but another one.
    OtherGame { title: String, code: String },
    /// Explorers of Sky, the Japanese release.
    Japanese,
    /// It could not be read.
    Unreadable(String),
}

impl Wrong {
    /// What it is, and what to do, for a person.
    pub fn said(&self) -> String {
        match self {
            Wrong::Missing => "There is no file there. Choose your .nds file again.".to_string(),
            Wrong::NotNds => {
                "This is not a Nintendo DS cartridge dump. Choose the .nds file you made from your own Explorers of Sky cartridge (a file of about 64 MB)."
                    .to_string()
            }
            Wrong::OtherGame { title, code } if title.starts_with("POKEDUN") => format!(
                "This is a Pokémon Mystery Dungeon game, but not Explorers of Sky (its code is {code}). Explorers of Time and Explorers of Darkness are not read: only Explorers of Sky is."
            ),
            Wrong::OtherGame { title, code } => format!("This is another DS game (\"{title}\", code {code}), not Pokémon Mystery Dungeon: Explorers of Sky."),
            Wrong::Japanese => {
                "This is the Japanese release of Explorers of Sky (C2SJ). The tool reads the game's texts in English: use the North American (C2SE) or European (C2SP) release."
                    .to_string()
            }
            Wrong::Unreadable(why) => format!("The file could not be read ({why}). Is it on a drive that is still connected?"),
        }
    }
}

/// What a DS cartridge's first bytes say: Explorers of Sky of a region the
/// tool reads, or why not. (Its title at 0, 12 bytes; its game code at 12, 4.)
pub fn read_header(bytes: &[u8]) -> Result<Region, Wrong> {
    if bytes.len() < 0x200 {
        return Err(Wrong::NotNds);
    }
    let printable = |part: &[u8]| part.iter().all(|byte| *byte == 0 || (0x20..0x7f).contains(byte));
    let (title, code) = (&bytes[0..12], &bytes[12..16]);
    if !printable(title) || !code.iter().all(u8::is_ascii_alphanumeric) {
        return Err(Wrong::NotNds);
    }
    let code = String::from_utf8_lossy(code).into_owned();
    match code.as_str() {
        "C2SE" => Ok(Region::NorthAmerica),
        "C2SP" => Ok(Region::Europe),
        "C2SJ" => Err(Wrong::Japanese),
        _ => Err(Wrong::OtherGame { title: String::from_utf8_lossy(title).trim_end_matches('\0').trim().to_string(), code }),
    }
}

/// That file's header read and checked.
pub fn check_rom(path: &Path) -> Result<Region, Wrong> {
    if !path.is_file() {
        return Err(Wrong::Missing);
    }
    let mut first = Vec::with_capacity(0x200);
    let file = std::fs::File::open(path).map_err(|why| Wrong::Unreadable(why.to_string()))?;
    file.take(0x200).read_to_end(&mut first).map_err(|why| Wrong::Unreadable(why.to_string()))?;
    read_header(&first)
}

// ----- the steps -----

/// What the first launch does, in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// The cartridge read into `data/rom/` (the `extractor` program).
    Extract,
    /// Each dungeon floor's tileset, the stairs and the traps as pictures (`scene_player dungeons`).
    Dungeons,
    /// The pieces of the game's places found (`decor inventory`).
    Pieces,
    /// Every place of the library and of the games built (`decor all`).
    Places,
}

impl Step {
    pub const ALL: [Step; 4] = [Step::Extract, Step::Dungeons, Step::Pieces, Step::Places];

    /// What it does, as shown.
    pub fn doing(self) -> &'static str {
        match self {
            Step::Extract => "Reading your cartridge: pictures, sounds, music, texts, maps (the longest step)",
            Step::Dungeons => "Getting the dungeons' floors ready",
            Step::Pieces => "Finding the pieces of the game's places",
            Step::Places => "Building the places of the films and of the example games",
        }
    }

    fn after(self) -> Option<Step> {
        Step::ALL.iter().position(|step| *step == self).and_then(|at| Step::ALL.get(at + 1)).copied()
    }
}

/// An error a step met, said for a person: the disk full, a folder that cannot be written, else as it is.
pub fn explained(step: Step, error: &str) -> String {
    let lower = error.to_lowercase();
    let full = ["os error 112", "os error 28", "no space left", "not enough space", "disk full", "storage full"];
    let denied = ["os error 5)", "os error 13", "permission denied", "access is denied"];
    if full.iter().any(|said| lower.contains(said)) {
        return format!(
            "Not enough space on the disk: the data takes {ROOM}. Free some space (or set DUNGEON_DATA to a folder on another disk), then click Try Again.\n({error})"
        );
    }
    if denied.iter().any(|said| lower.contains(said)) {
        return format!(
            "The data folder cannot be written here. Move the Workshop's folder somewhere of your own (your Documents, not Program Files), or set DUNGEON_DATA to a folder you can write to, then click Try Again.\n({error})"
        );
    }
    format!("{} failed:\n{error}", step.doing())
}

/// A program the first launch runs beside the Workshop, not there: why, and how to get it.
pub fn missing_program(path: &Path) -> String {
    let program = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
    match folder.components().any(|part| part.as_os_str() == "target") {
        // (From the sources: `cargo run -p workshop` builds the Workshop alone.)
        true => format!(
            "The {program} program is not built yet ({} is missing). Build it, then click Try Again:\n    cargo build {}-p {program}",
            path.display(),
            if folder.ends_with("release") { "--release " } else { "" }
        ),
        false => format!(
            "The {program} program is missing beside the Workshop ({}). It comes in the same zip as the Workshop: unzip the whole folder again, then click Try Again.",
            path.display()
        ),
    }
}

// ----- the state machine -----

/// Where the first launch is.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// No cartridge chosen yet, or one that will not do (why).
    Choosing(Option<Wrong>),
    /// A cartridge the tool reads, chosen: Extract can be clicked.
    Ready(PathBuf, Region),
    /// A step running, what it said last (a few lines).
    Running { step: Step, said: Vec<String> },
    /// A step failed: why, said for a person; the cartridge kept, to try again.
    Failed { rom: Option<PathBuf>, said: String },
    /// Everything made: the Workshop opens.
    Done,
}

/// What happens to it.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A file chosen, and what its header said.
    Chosen(PathBuf, Result<Region, Wrong>),
    /// A step begins.
    Started(Step),
    /// A line of what the step is doing.
    Said(String),
    /// A step went wrong (its error as it came).
    Failed(Step, String),
    /// A step done.
    Finished(Step),
}

/// How many lines of what a step says are kept on screen.
const SAID_KEPT: usize = 6;

impl Phase {
    /// The phase after that event (`rom`: the cartridge being read, kept if a step fails).
    pub fn next(self, event: Event, rom: Option<&Path>) -> Phase {
        match (self, event) {
            (Phase::Done, _) => Phase::Done,
            // (A cartridge dropped while it runs: the run goes on.)
            (running @ Phase::Running { .. }, Event::Chosen(..)) => running,
            (_, Event::Chosen(path, Ok(region))) => Phase::Ready(path, region),
            (_, Event::Chosen(_, Err(wrong))) => Phase::Choosing(Some(wrong)),
            (_, Event::Started(step)) => Phase::Running { step, said: Vec::new() },
            (Phase::Running { step, mut said }, Event::Said(line)) => {
                said.push(line);
                let over = said.len().saturating_sub(SAID_KEPT);
                said.drain(..over);
                Phase::Running { step, said }
            }
            (phase, Event::Said(_)) => phase,
            (_, Event::Failed(step, error)) => Phase::Failed { rom: rom.map(Path::to_path_buf), said: explained(step, &error) },
            (_, Event::Finished(step)) => match step.after() {
                Some(next) => Phase::Running { step: next, said: Vec::new() },
                None => Phase::Done,
            },
        }
    }

    /// Whether a step is running.
    pub fn running(&self) -> bool {
        matches!(self, Phase::Running { .. })
    }
}

// ----- the screen -----

/// The first launch, as it goes: the copy's path as written, the phase, the run's events.
pub struct First {
    pub session: Arc<Session>,
    rom: String,
    phase: Phase,
    /// The cartridge chosen, kept to try again.
    chosen: Option<PathBuf>,
    run: Option<(Receiver<Event>, Instant)>,
    /// How long each step done took, in seconds; when the one running began.
    took: Vec<(Step, u64)>,
    step_since: Instant,
}

impl First {
    pub fn new(session: Arc<Session>) -> First {
        First { session, rom: String::new(), phase: Phase::Choosing(None), chosen: None, run: None, took: Vec::new(), step_since: Instant::now() }
    }

    /// The phase after that event.
    fn happen(&mut self, event: Event) {
        let rom = self.chosen.clone();
        self.phase = std::mem::replace(&mut self.phase, Phase::Done).next(event, rom.as_deref());
    }

    /// The file at that path chosen: checked.
    fn choose(&mut self, path: PathBuf) {
        self.rom = path.display().to_string();
        let checked = check_rom(&path);
        self.chosen = checked.is_ok().then(|| path.clone());
        self.happen(Event::Chosen(path, checked));
    }

    /// The extraction started on its own thread (its steps told as events).
    fn start(&mut self, rom: PathBuf) {
        let (tell, events) = channel();
        let session = self.session.clone();
        self.chosen = Some(rom.clone());
        std::thread::spawn(move || run_steps(&session, &rom, &tell));
        (self.run, self.took, self.step_since) = (Some((events, Instant::now())), Vec::new(), Instant::now());
        self.happen(Event::Started(Step::Extract));
    }

    /// A frame of it; true once the data is ready (the Workshop opens).
    pub fn ui(&mut self, context: &egui::Context) -> bool {
        // (A copy dropped on the window: checked.)
        if !self.phase.running()
            && let Some(dropped) = context.input(|input| input.raw.dropped_files.iter().find_map(|file| file.path.clone()))
        {
            self.choose(dropped);
        }
        let mut events = Vec::new();
        if let Some((told, _)) = &self.run {
            events.extend(told.try_iter());
            context.request_repaint_after(std::time::Duration::from_millis(200));
        }
        for event in events {
            if let Event::Finished(step) = &event {
                self.took.push((*step, self.step_since.elapsed().as_secs()));
                self.step_since = Instant::now();
            }
            if matches!(event, Event::Failed(..)) {
                self.run = None;
            }
            self.happen(event);
        }
        if self.phase == Phase::Done {
            // (What was read before the data was there, read again.)
            self.session.reload();
            if has_data(&self.session) {
                return true;
            }
            self.phase = Phase::Failed { rom: self.chosen.clone(), said: format!("The extraction ended, but {} has no dungeons.bin.", self.session.data()) };
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::same(40))).show(context, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.label(RichText::new("Dungeon Workshop").size(30.0).strong().color(theme::STRONG));
                    ui.add_space(6.0);
                    ui.label(theme::text("Welcome! One thing to do before anything else.", theme::BODY + 2.0, theme::WEAK));
                    ui.add_space(20.0);
                });
                ui.vertical(|ui| {
                    ui.set_max_width(700.0);
                    self.explanation(ui);
                    ui.add_space(16.0);
                    match self.phase.clone() {
                        Phase::Running { step, said } => self.progress(ui, step, &said),
                        phase => self.choosing(ui, &phase),
                    }
                });
            });
        });
        false
    }

    /// What this is about: the user's own cartridge, never shipped.
    fn explanation(&self, ui: &mut egui::Ui) {
        let line = |ui: &mut egui::Ui, said: &str| ui.label(theme::text(said, theme::BODY, theme::TEXT));
        line(
            ui,
            "Dungeon Workshop contains no picture, sound, music or text of any published game. It reads them from your own copy of Pokémon Mystery Dungeon: Explorers of Sky: a .nds file you dumped yourself from a cartridge you own. Nothing of the game comes with the Workshop, and nothing is ever sent anywhere.",
        );
        ui.add_space(6.0);
        line(
            ui,
            &format!(
                "The Workshop reads that file once and writes what it needs into a data folder ({ROOM}): {}. It takes a few minutes. Your .nds file is only read, never changed.",
                Path::new(self.session.data()).display()
            ),
        );
    }

    /// Choosing the cartridge, then Extract; what went wrong.
    fn choosing(&mut self, ui: &mut egui::Ui, phase: &Phase) {
        ui.label(theme::text("1. Your cartridge", theme::TITLE, theme::STRONG));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Choose Your .nds File…").strong()).clicked()
                && let Some(path) = crate::pick::file("Your copy of Explorers of Sky", &[("Nintendo DS cartridge", &["nds"]), ("Any file", &["*"])], None)
            {
                self.choose(path);
            }
            ui.label(theme::weak("or drop it on this window, or write its path and press Enter:"));
        });
        let written = ui.add(egui::TextEdit::singleline(&mut self.rom).hint_text("C:\\Games\\explorers_of_sky.nds").desired_width(660.0));
        if written.lost_focus() && !self.rom.trim().is_empty() {
            let path = PathBuf::from(self.rom.trim().trim_matches('"'));
            if self.chosen.as_ref() != Some(&path) {
                self.choose(path);
            }
        }
        ui.add_space(8.0);
        match phase {
            Phase::Choosing(Some(wrong)) => _ = ui.label(RichText::new(wrong.said()).color(theme::WRONG).size(theme::BODY)),
            Phase::Ready(_, region) => _ = ui.label(theme::text(format!("✓ {}: the tool reads this one.", region.said()), theme::BODY, theme::DUNGEON)),
            _ => {}
        }
        ui.add_space(14.0);
        ui.label(theme::text("2. Extract", theme::TITLE, theme::STRONG));
        ui.add_space(4.0);
        let rom = match phase {
            Phase::Ready(path, _) => Some(path.clone()),
            Phase::Failed { rom, .. } => rom.clone(),
            _ => None,
        };
        let label = if matches!(phase, Phase::Failed { .. }) { "Try Again" } else { "Extract the Resources" };
        if ui.add_enabled(rom.is_some(), egui::Button::new(RichText::new(label).strong().color(theme::ASSETS))).clicked()
            && let Some(rom) = rom
        {
            match program_beside("extractor") {
                Ok(_) => self.start(rom),
                Err(why) => self.phase = Phase::Failed { rom: Some(rom), said: why },
            }
        }
        if let Phase::Failed { said, .. } = phase {
            ui.add_space(8.0);
            egui::Frame::new().fill(theme::GROUND).corner_radius(4).inner_margin(10).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(said).color(theme::WRONG).size(theme::BODY));
            });
        }
    }

    /// The steps: done (how long), running (what it says), to come.
    fn progress(&self, ui: &mut egui::Ui, now: Step, said: &[String]) {
        ui.label(theme::text("Extracting… keep this window open.", theme::TITLE, theme::STRONG));
        ui.add_space(8.0);
        for step in Step::ALL {
            let done = self.took.iter().find(|(one, _)| *one == step);
            ui.horizontal(|ui| match (done, step == now) {
                (Some((_, seconds)), _) => {
                    ui.label(theme::text("✓", theme::BODY, theme::DUNGEON));
                    ui.label(theme::text(format!("{} ({seconds} s)", step.doing()), theme::BODY, theme::WEAK));
                }
                (None, true) => {
                    ui.add(egui::Spinner::new());
                    ui.label(theme::text(format!("{}… {} s", step.doing(), self.step_since.elapsed().as_secs()), theme::BODY, theme::STRONG));
                }
                (None, false) => {
                    ui.label(theme::text("○", theme::BODY, theme::FAINT));
                    ui.label(theme::text(step.doing(), theme::BODY, theme::FAINT));
                }
            });
        }
        let at = Step::ALL.iter().position(|step| *step == now).unwrap_or(0);
        ui.add_space(6.0);
        ui.add(egui::ProgressBar::new(at as f32 / Step::ALL.len() as f32).desired_width(660.0).text(format!("step {} of {}", at + 1, Step::ALL.len())));
        ui.add_space(8.0);
        egui::Frame::new().fill(theme::GROUND).corner_radius(4).inner_margin(10).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if said.is_empty() {
                ui.label(theme::text("…", theme::SMALL, theme::FAINT));
            }
            for line in said {
                ui.label(RichText::new(line).monospace().size(theme::SMALL).color(theme::WEAK));
            }
        });
        if let Some((_, since)) = &self.run {
            ui.label(theme::text(format!("{} s in all", since.elapsed().as_secs()), theme::SMALL, theme::FAINT));
        }
    }
}

/// One of the programs that come with the Workshop, beside it: its path, or why it is not there.
pub fn program_beside(program: &str) -> Result<PathBuf, String> {
    let folder = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)).unwrap_or_default();
    let path = folder.join(format!("{program}{}", std::env::consts::EXE_SUFFIX));
    if path.is_file() { Ok(path) } else { Err(missing_program(&path)) }
}

/// The steps, one after the other, each told (started, its lines, done or why not).
fn run_steps(session: &Session, rom: &Path, tell: &Sender<Event>) {
    let (root, data) = (session.root().to_path_buf(), session.data().to_string());
    let library = session.library().clone();
    let said = |line: String| _ = tell.send(Event::Said(line));
    for step in Step::ALL {
        _ = tell.send(Event::Started(step));
        let done = match step {
            Step::Extract => extract(&root, &data, rom, &said),
            Step::Dungeons => game::setup::prepare_dungeons(&data).map(|lines| lines.into_iter().for_each(said)).map_err(String::from),
            Step::Pieces => pieces(&library, &data).map(said),
            Step::Places => decor::build_all(&library, &data, &decor::write::extra_of(&data), &mut |told| {
                for line in told.unwrap_or_else(|why| why).lines().filter(|line| !line.trim().is_empty()) {
                    said(line.to_string());
                }
            })
            .map(|failed| {
                if !failed.is_empty() {
                    said(format!("{} place(s) not built: {}", failed.len(), failed.join(", ")));
                }
            }),
        };
        match done {
            Ok(()) => _ = tell.send(Event::Finished(step)),
            Err(why) => {
                _ = tell.send(Event::Failed(step, why));
                return;
            }
        }
    }
}

/// The cartridge read by `extractor <rom> <data>`, beside the Workshop, from the root, each line it says told.
fn extract(root: &Path, data: &str, rom: &Path, said: &dyn Fn(String)) -> Result<(), String> {
    let program = program_beside("extractor")?;
    // (Into data/rom: the layer the extraction alone writes, made first.)
    let into = Path::new(data).join("rom");
    std::fs::create_dir_all(&into).map_err(|why| format!("{}: {why}", into.display()))?;
    let mut child = std::process::Command::new(&program)
        .arg(rom)
        .arg(data)
        .current_dir(root)
        .env("DUNGEON_DATA", data)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|why| format!("{}: {why}", program.display()))?;
    // (What it says on its errors' side, kept to tell why it failed.)
    let errors = child.stderr.take().map(|stderr| {
        std::thread::spawn(move || {
            let mut text = String::new();
            _ = std::io::BufReader::new(stderr).read_to_string(&mut text);
            text
        })
    });
    if let Some(stdout) = child.stdout.take() {
        for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
            said(line);
        }
    }
    let status = child.wait().map_err(|why| why.to_string())?;
    let errors = errors.and_then(|thread| thread.join().ok()).unwrap_or_default();
    for line in errors.lines().filter(|line| !line.trim().is_empty()) {
        said(line.to_string());
    }
    match status.success() && into.join("dungeons.bin").is_file() {
        true => Ok(()),
        false if errors.trim().is_empty() => Err(format!("extractor stopped ({status})")),
        false => Err(errors.trim().to_string()),
    }
}

/// The pieces of the game's places found (`decor inventory`): what it said.
fn pieces(library: &assets::project::Project, data: &str) -> Result<String, String> {
    let workshop = decor::workshop(library, data).map_err(|why| why.to_string())?;
    let out = decor::game_pieces(data);
    if let Some(folder) = out.parent() {
        std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
    }
    decor::inventory::inventory(&workshop, &[], &out).map_err(|why| why.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first 512 bytes of a cartridge of that title and game code.
    fn header(title: &[u8], code: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x200];
        bytes[..title.len()].copy_from_slice(title);
        bytes[12..16].copy_from_slice(code);
        bytes
    }

    #[test]
    fn a_cartridge_is_read_by_its_header() {
        assert_eq!(read_header(&header(b"POKEDUN SORA", b"C2SE")), Ok(Region::NorthAmerica));
        assert_eq!(read_header(&header(b"POKEDUN SORA", b"C2SP")), Ok(Region::Europe));
        assert_eq!(read_header(&header(b"POKEDUN SORA", b"C2SJ")), Err(Wrong::Japanese));
        assert!(matches!(read_header(&header(b"POKEDUN DARK", b"YFYE")), Err(Wrong::OtherGame { .. })));
        assert!(matches!(read_header(&header(b"MARIO KART", b"AMCE")), Err(Wrong::OtherGame { .. })));
        // (A picture, a file of noise, one too short: not a cartridge.)
        assert_eq!(read_header(b"\x89PNG\r\n\x1a\n"), Err(Wrong::NotNds));
        assert_eq!(read_header(&[0xff; 0x200]), Err(Wrong::NotNds));
        let said = Wrong::OtherGame { title: "POKEDUN DARK".into(), code: "YFYE".into() }.said();
        assert!(said.contains("Explorers of Time") && said.contains("only Explorers of Sky"), "{said}");
        assert!(Wrong::Japanese.said().contains("C2SE"));
        assert_eq!(check_rom(Path::new("no/such/file.nds")), Err(Wrong::Missing));
    }

    #[test]
    fn what_goes_wrong_is_said_for_a_person() {
        let full = explained(Step::Extract, "data/rom/sprites.bin: There is not enough space on the disk. (os error 112)");
        assert!(full.starts_with("Not enough space") && full.contains(ROOM), "{full}");
        assert!(explained(Step::Places, "No space left on device (os error 28)").starts_with("Not enough space"));
        assert!(explained(Step::Extract, "C:\\Program Files\\dw\\data\\rom: Access is denied. (os error 5)").contains("cannot be written"));
        let other = explained(Step::Pieces, "no level 323");
        assert!(other.contains(Step::Pieces.doing()) && other.contains("no level 323"), "{other}");
        // (A program missing: from the sources, how to build it; from the zip, unzip it again.)
        let built = missing_program(Path::new("C:/dw/target/debug/extractor.exe"));
        assert!(built.contains("cargo build -p extractor"), "{built}");
        assert!(missing_program(Path::new("C:/dw/target/release/extractor.exe")).contains("cargo build --release -p extractor"));
        assert!(missing_program(Path::new("C:/Games/DungeonWorkshop/extractor.exe")).contains("unzip"));
    }

    #[test]
    fn the_first_launch_goes_from_the_cartridge_to_the_workshop() {
        let rom = PathBuf::from("sky.nds");
        let mut phase = Phase::Choosing(None);
        phase = phase.next(Event::Chosen(rom.clone(), Err(Wrong::NotNds)), None);
        assert_eq!(phase, Phase::Choosing(Some(Wrong::NotNds)));
        phase = phase.next(Event::Chosen(rom.clone(), Ok(Region::Europe)), None);
        assert_eq!(phase, Phase::Ready(rom.clone(), Region::Europe));
        phase = phase.next(Event::Started(Step::Extract), Some(&rom));
        for line in 0..10 {
            phase = phase.next(Event::Said(format!("line {line}")), Some(&rom));
        }
        match &phase {
            Phase::Running { step: Step::Extract, said } => assert_eq!((said.len(), said[SAID_KEPT - 1].as_str()), (SAID_KEPT, "line 9")),
            other => panic!("{other:?}"),
        }
        // (A cartridge dropped meanwhile changes nothing.)
        phase = phase.next(Event::Chosen(PathBuf::from("other.nds"), Ok(Region::NorthAmerica)), Some(&rom));
        assert!(phase.running());
        phase = phase.next(Event::Finished(Step::Extract), Some(&rom));
        assert_eq!(phase, Phase::Running { step: Step::Dungeons, said: Vec::new() });
        // (A step failed: why, and the cartridge kept to try again.)
        let failed = phase.clone().next(Event::Failed(Step::Dungeons, "(os error 112)".into()), Some(&rom));
        assert!(matches!(&failed, Phase::Failed { rom: Some(kept), said } if *kept == rom && said.starts_with("Not enough space")), "{failed:?}");
        for step in [Step::Dungeons, Step::Pieces, Step::Places] {
            phase = phase.next(Event::Finished(step), Some(&rom));
        }
        assert_eq!(phase, Phase::Done);
    }
}
