//! The Emulator space: a DS game run in the Workshop by a libretro core the
//! user chose (melonDS, DeSmuME; never bundled), both screens drawn crisp,
//! its sound on the speakers, the keyboard as its keys and the mouse as its
//! stylus. "Play this film on DS" puts the film open in the Film space into
//! a copy of the user's own Explorers of Sky (`scene_player export-ds`, then
//! `tools/ds/patch_rom.py --boot --more-actors`) and plays it from power on.
//!
//! The core, the ROM and armips are the user's own paths, kept in the
//! Workshop's settings (`settings`), never in the project.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

use eframe::egui::{self, RichText};
use emulator::{Button, Core, Input};

use crate::session::Session;
use crate::settings::{self, Settings};
use crate::theme;

/// The keyboard as the DS's keys: the key, the button, as shown.
const KEYS: [(egui::Key, Button); 12] = [
    (egui::Key::ArrowUp, Button::Up),
    (egui::Key::ArrowDown, Button::Down),
    (egui::Key::ArrowLeft, Button::Left),
    (egui::Key::ArrowRight, Button::Right),
    (egui::Key::X, Button::A),
    (egui::Key::Z, Button::B),
    (egui::Key::S, Button::X),
    (egui::Key::A, Button::Y),
    (egui::Key::Q, Button::L),
    (egui::Key::W, Button::R),
    (egui::Key::Enter, Button::Start),
    (egui::Key::Backspace, Button::Select),
];

/// The key map, in one line.
pub const KEY_MAP: &str = "Arrows: D-pad · X: A · Z: B · S: X · A: Y · Q/W: L/R · Enter: Start · Shift: Select · mouse on the bottom screen: stylus";

/// The patched copy the film is played in, and its film's export, under `<root>/target/ds/`.
const PLAYED: &str = "workshop_play";

/// A is pressed every that many frames from power on, until the film starts (as `tools/ds/boot_and_watch.py` does):
/// in melonDS, the "New Game / Other" menu at about frame 1100, the controls screen at about 1200, the film from about 1450.
const PRESS_EVERY: u64 = 90;
const PRESS_UNTIL: u64 = 1400;

/// What a run of "Play this film on DS" tells as it goes.
enum Told {
    Line(String),
    Done(Result<PathBuf, String>),
}

/// "Play this film on DS" at work: its lines so far, its end.
struct Making {
    film: String,
    lines: Vec<String>,
    told: Receiver<Told>,
    done: Option<Result<(), String>>,
}

pub struct Emulate {
    root: PathBuf,
    data: String,
    settings: Settings,
    /// The core and armips the launch named (before the settings' own).
    launch_core: Option<PathBuf>,
    launch_armips: Option<PathBuf>,
    core: Option<Core>,
    texture: Option<egui::TextureHandle>,
    paused: bool,
    speed: u32,
    /// Frames owed to the clock, and when it was last read.
    owed: f64,
    clock: Instant,
    /// Frames run since the game was loaded.
    frames: u64,
    /// A is pressed by itself until that frame (through the title).
    pressing_until: Option<u64>,
    sound: Option<shell::stream::Stream>,
    /// What was last done, or why not.
    pub said: String,
    making: Option<Making>,
    /// The core's path, as typed.
    core_typed: String,
    /// One frame asked by Step.
    step_asked: bool,
}

impl Emulate {
    pub fn new(session: &Session) -> Emulate {
        let launch = &session.launch;
        let settings = Settings::read(launch.settings.clone());
        let mut emulate = Emulate {
            root: session.root().to_path_buf(),
            data: session.data().to_string(),
            core_typed: String::new(),
            step_asked: false,
            settings,
            launch_core: launch.libretro_core.clone(),
            launch_armips: launch.armips.clone(),
            core: None,
            texture: None,
            paused: false,
            speed: 1,
            owed: 0.0,
            clock: Instant::now(),
            frames: 0,
            pressing_until: None,
            sound: None,
            said: String::new(),
            making: None,
        };
        // (`WORKSHOP_EMULATOR_ROM`, `WORKSHOP_EMULATOR_FRAMES`: a game loaded and run that far, for a picture.)
        if let Some(rom) = launch.emulator_rom.clone() {
            emulate.load(&rom, false);
            if let Some(core) = &mut emulate.core {
                let frames = launch.emulator_frames.unwrap_or(0);
                for _ in 0..frames {
                    core.run(Input::default());
                }
                core.take_audio();
                emulate.frames = frames as u64;
            }
        }
        // (`WORKSHOP_EMULATOR_FILM`: that film put on DS at once, for a picture.)
        if let Some(film) = launch.emulator_film.clone() {
            emulate.play_film(&film);
        }
        emulate
    }

    /// The core to run: the launch's, else the settings'.
    fn core_path(&self) -> Option<PathBuf> {
        self.launch_core.clone().or_else(|| self.settings.path(settings::CORE))
    }

    /// The emulator's own folder, beside the settings: its system files, the games' saves, savestates, pictures.
    fn folder(&self) -> PathBuf {
        self.settings.folder().join("emulator")
    }

    /// That game loaded (`fresh`: its save file removed first, as for a copy just patched).
    fn load(&mut self, rom: &Path, fresh: bool) {
        let Some(core) = self.core_path() else {
            self.said = "no libretro core chosen yet".to_string();
            return;
        };
        // (The game before put away first: one core at a time.)
        self.core = None;
        let folder = self.folder();
        let options = emulator::Options { core, system: folder.join("system"), saves: folder.join("saves"), wanted: emulator::ds_options() };
        if fresh {
            forget_saves(&options.saves, rom);
        }
        match Core::open(&options, rom) {
            Ok(core) => {
                if self.sound.as_ref().is_none_or(|sound| sound.rate() != core.sample_rate()) {
                    self.sound = shell::stream::Stream::open(core.sample_rate());
                }
                self.said = format!("{} {}: {}", core.name, core.version, rom.file_name().unwrap_or_default().to_string_lossy());
                self.core = Some(core);
                (self.frames, self.owed, self.paused, self.pressing_until) = (0, 0.0, false, None);
                self.clock = Instant::now();
            }
            Err(why) => self.said = why,
        }
    }

    /// The savestate's file of the game loaded.
    fn state_file(&self) -> Option<PathBuf> {
        let core = self.core.as_ref()?;
        let stem = core.game.file_stem()?.to_string_lossy().into_owned();
        Some(self.folder().join("states").join(format!("{stem}.state")))
    }

    fn save_state(&mut self) {
        let (Some(file), Some(core)) = (self.state_file(), self.core.as_mut()) else {
            return;
        };
        self.said = match core.save_state().and_then(|bytes| write(&file, &bytes)) {
            Ok(()) => format!("state saved: {}", file.display()),
            Err(why) => why,
        };
    }

    fn load_state(&mut self) {
        let (Some(file), Some(core)) = (self.state_file(), self.core.as_mut()) else {
            return;
        };
        self.said = match std::fs::read(&file).map_err(|why| format!("{}: {why}", file.display())).and_then(|bytes| core.load_state(&bytes)) {
            Ok(()) => {
                self.pressing_until = None;
                "state loaded".to_string()
            }
            Err(why) => why,
        };
    }

    fn screenshot(&mut self) {
        let Some(core) = &self.core else {
            return;
        };
        let stem = core.game.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let file = self.folder().join("screenshots").join(format!("{stem}_{}.png", self.frames));
        self.said = match core.frame().save_png(&file) {
            Ok(()) => format!("screenshot: {}", file.display()),
            Err(why) => why,
        };
    }

    /// A game of the user's chosen and loaded; the first one remembered as their Explorers of Sky.
    fn choose_rom(&mut self) {
        let start = self.settings.path(settings::ROM);
        if let Some(rom) = crate::pick::file("A DS game of yours", &[("Nintendo DS cartridge", &["nds"]), ("Any file", &["*"])], start.as_deref()) {
            if self.settings.get(settings::ROM).is_none() && crate::first::check_rom(&rom).is_ok() {
                self.remember(settings::ROM, &rom);
            }
            self.load(&rom, false);
        }
    }

    fn choose_core(&mut self) {
        let kinds: &[(&str, &[&str])] = &[("libretro core", &["dll", "so", "dylib"]), ("Any file", &["*"])];
        if let Some(core) = crate::pick::file("A libretro core (melonDS or DeSmuME)", kinds, self.core_path().as_deref()) {
            self.remember(settings::CORE, &core);
            self.said = format!("core: {}", core.display());
        }
    }

    fn remember(&mut self, key: &str, path: &Path) {
        if let Err(why) = self.settings.set(key, &path.display().to_string()) {
            self.said = why;
        }
    }

    /// "Play this film on DS": the user's ROM and armips asked once, then the export and the patch on a thread of their own.
    fn play_film(&mut self, film: &Path) {
        let rom = match self.settings.path(settings::ROM).filter(|rom| rom.is_file()) {
            Some(rom) => rom,
            None => match crate::pick::file("Your copy of Explorers of Sky (only read)", &[("Nintendo DS cartridge", &["nds"])], None) {
                Some(rom) => {
                    self.remember(settings::ROM, &rom);
                    rom
                }
                None => return self.said = "no ROM chosen: the film needs your copy of Explorers of Sky".to_string(),
            },
        };
        let armips = match self.launch_armips.clone().or_else(|| self.settings.path(settings::ARMIPS)).filter(|armips| armips.is_file()) {
            Some(armips) => armips,
            None => match crate::pick::file("armips (SkyTemple assembles its patches with it)", &[("armips", &["exe", "*"])], None) {
                Some(armips) => {
                    self.remember(settings::ARMIPS, &armips);
                    armips
                }
                None => return self.said = "no armips chosen: patch_rom.py --more-actors needs it (SKYTEMPLE_ARMIPS_EXEC)".to_string(),
            },
        };
        let (tell, told) = channel();
        let (root, data, film_path) = (self.root.clone(), self.data.clone(), film.to_path_buf());
        std::thread::spawn(move || {
            let done = put_in_rom(&root, &data, &film_path, &rom, &armips, &tell);
            _ = tell.send(Told::Done(done));
        });
        let name = film.file_name().unwrap_or_default().to_string_lossy().into_owned();
        self.said = format!("putting {name} in your ROM…");
        self.making = Some(Making { film: name, lines: Vec::new(), told, done: None });
    }

    /// What the making told since, and its end: the patched copy loaded, A pressed through the title.
    fn hear_making(&mut self) {
        let Some(making) = &mut self.making else {
            return;
        };
        let mut patched = None;
        while let Ok(told) = making.told.try_recv() {
            match told {
                Told::Line(line) => making.lines.push(line),
                Told::Done(Ok(rom)) => {
                    making.done = Some(Ok(()));
                    patched = Some(rom);
                }
                Told::Done(Err(why)) => {
                    self.said = format!("{} could not be put on DS", making.film);
                    making.done = Some(Err(why));
                }
            }
        }
        if let Some(rom) = patched {
            self.load(&rom, true);
            if self.core.is_some() {
                self.pressing_until = Some(PRESS_UNTIL);
                self.said = format!(
                    "{}: from power on, A pressed through the title (the film from about 25 s)",
                    self.making.as_ref().map_or("", |making| &making.film)
                );
            }
        }
    }

    /// The keys held now, as the DS's, and the stylus where the mouse holds the bottom screen.
    fn input(&self, context: &egui::Context, touch: Option<(f32, f32)>) -> Input {
        let mut input = Input { touch, ..Input::default() };
        if !context.wants_keyboard_input() {
            context.input(|keys| {
                for (key, button) in KEYS {
                    if keys.key_down(key) {
                        input = input.with(button);
                    }
                }
                if keys.modifiers.shift {
                    input = input.with(Button::Select);
                }
            });
        }
        if let Some(until) = self.pressing_until
            && self.frames < until
            && self.frames % PRESS_EVERY >= PRESS_EVERY - 3
        {
            input = input.with(Button::A);
        }
        input
    }

    /// The frames owed to the clock run (or one, stepped), their sound played, the last one shown.
    fn run(&mut self, context: &egui::Context, touch: Option<(f32, f32)>, step: bool) {
        let elapsed = self.clock.elapsed().as_secs_f64().min(0.1);
        self.clock = Instant::now();
        let input = self.input(context, touch);
        let Some(core) = &mut self.core else {
            return;
        };
        if !self.paused {
            self.owed += elapsed * core.fps() * self.speed as f64;
        }
        let mut count = self.owed.floor() as u32;
        self.owed -= count as f64;
        if step {
            count = 1;
        }
        // (Never more than a few at once: a slow machine plays slower, it does not stall.)
        for _ in 0..count.min(4 * self.speed) {
            core.run(input);
            self.frames += 1;
        }
        if self.pressing_until.is_some_and(|until| self.frames >= until) {
            self.pressing_until = None;
        }
        let audio = core.take_audio();
        if let Some(sound) = &self.sound {
            match self.speed == 1 && !self.paused {
                true => sound.push(&audio, 0.12),
                false => sound.clear(),
            }
        }
        if let Some(frame) = core.fresh_frame() {
            let image = egui::ColorImage::from_rgba_unmultiplied([frame.width as usize, frame.height as usize], &frame.rgba);
            match &mut self.texture {
                Some(texture) => texture.set(image, egui::TextureOptions::NEAREST),
                None => self.texture = Some(context.load_texture("emulator", image, egui::TextureOptions::NEAREST)),
            }
        }
        if !self.paused {
            context.request_repaint();
        }
    }
}

/// The save files of that game in the saves folder removed: a copy just patched starts a new game.
fn forget_saves(saves: &Path, rom: &Path) {
    let Some(stem) = rom.file_stem() else {
        return;
    };
    for entry in std::fs::read_dir(saves).into_iter().flatten().flatten() {
        if entry.path().file_stem() == Some(stem) {
            _ = std::fs::remove_file(entry.path());
        }
    }
}

fn write(file: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
    }
    std::fs::write(file, bytes).map_err(|why| format!("{}: {why}", file.display()))
}

/// The film exported (`scene_player export-ds`) and put in a copy of the ROM (`patch_rom.py --boot --more-actors`): the copy.
fn put_in_rom(root: &Path, data: &str, film: &Path, rom: &Path, armips: &Path, tell: &Sender<Told>) -> Result<PathBuf, String> {
    let say = |line: String| _ = tell.send(Told::Line(line));
    let folder = root.join("target").join("ds");
    std::fs::create_dir_all(&folder).map_err(|why| format!("{}: {why}", folder.display()))?;
    let (exps, out) = (folder.join(format!("{PLAYED}.exps")), folder.join(format!("{PLAYED}.nds")));
    say(format!("1/2 export: scene_player export-ds {}", film.display()));
    let player = crate::first::program_beside("scene_player")?;
    let mut export = std::process::Command::new(&player);
    export.args(["export-ds".as_ref(), film.as_os_str(), "--out".as_ref(), exps.as_os_str(), "--partial".as_ref()]).env("DUNGEON_DATA", data);
    told_run(export, root, &say).map_err(|why| format!("the export failed: {why}"))?;
    let script = root.join("tools").join("ds").join("patch_rom.py");
    if !script.is_file() {
        return Err(format!("{} is missing: the patch needs the repository's tools", script.display()));
    }
    say(format!("2/2 patch: patch_rom.py --boot --more-actors into {}", out.display()));
    let python = if cfg!(windows) { "python" } else { "python3" };
    let mut patch = std::process::Command::new(python);
    patch.arg(&script).arg(rom).arg(&exps).arg("--boot").arg(&out).arg("--more-actors").env("SKYTEMPLE_ARMIPS_EXEC", armips).env("PYTHONIOENCODING", "utf-8");
    told_run(patch, root, &say).map_err(|why| format!("the patch failed: {why}"))?;
    match out.is_file() {
        true => Ok(out),
        false => Err(format!("{} was not written", out.display())),
    }
}

/// A program run from the root, each line it says told; why it failed (its last lines).
fn told_run(mut command: std::process::Command, root: &Path, say: &dyn Fn(String)) -> Result<(), String> {
    use std::io::BufRead;
    let program = command.get_program().to_string_lossy().into_owned();
    let output = command.current_dir(root).stdin(std::process::Stdio::null()).output().map_err(|why| {
        format!("{program} could not be run ({why}){}", if program.starts_with("python") { ": is Python installed, with skytemple_files?" } else { "" })
    })?;
    let mut lines = Vec::new();
    for line in output.stdout.lines().chain(output.stderr.lines()).map_while(Result::ok).filter(|line| !line.trim().is_empty()) {
        say(format!("  {line}"));
        lines.push(line);
    }
    match output.status.success() {
        true => Ok(()),
        false => Err(lines.iter().rev().take(6).rev().cloned().collect::<Vec<_>>().join("\n")),
    }
}

/// The Emulator's tools: the game, the controls, the film.
pub fn tools(ui: &mut egui::Ui, emulate: &mut Emulate, film: Option<&Path>) {
    if ui.button("Load ROM…").on_hover_text("a DS game of yours (a .nds you dumped)").clicked() {
        emulate.choose_rom();
    }
    ui.separator();
    let has = emulate.core.is_some();
    let word = if emulate.paused { "▶ Play" } else { "⏸ Pause" };
    if ui.add_enabled(has, egui::Button::new(word)).clicked() {
        emulate.paused = !emulate.paused;
    }
    if ui.add_enabled(has, egui::Button::new("Reset")).on_hover_text("as if switched off and on").clicked()
        && let Some(core) = &mut emulate.core
    {
        core.reset();
        emulate.frames = 0;
    }
    let step = ui.add_enabled(has && emulate.paused, egui::Button::new("Step")).on_hover_text("one frame on, paused").clicked();
    if step {
        emulate.step_asked = true;
    }
    for speed in [1, 2, 4] {
        let chosen = emulate.speed == speed;
        let text = RichText::new(format!("{speed}×")).color(if chosen { theme::GROUND } else { theme::TEXT });
        if ui.add_enabled(has, egui::Button::new(text).fill(if chosen { theme::EMULATOR } else { theme::FIELD })).clicked() {
            emulate.speed = speed;
        }
    }
    ui.separator();
    if ui.add_enabled(has, egui::Button::new("Save State")).on_hover_text("one slot a game, in the Workshop's settings folder").clicked() {
        emulate.save_state();
    }
    let saved = emulate.state_file().is_some_and(|file| file.is_file());
    if ui.add_enabled(saved, egui::Button::new("Load State")).clicked() {
        emulate.load_state();
    }
    if ui.add_enabled(has, egui::Button::new("Screenshot")).on_hover_text("both screens as a PNG, in the Workshop's settings folder").clicked() {
        emulate.screenshot();
    }
    ui.separator();
    let busy = emulate.making.as_ref().is_some_and(|making| making.done.is_none());
    let can = film.is_some() && emulate.core_path().is_some() && !busy;
    let hint = match film {
        Some(film) => format!("{} put in a copy of your Explorers of Sky, played from power on", film.file_name().unwrap_or_default().to_string_lossy()),
        None => "open a film in the Film space first".to_string(),
    };
    let play = egui::Button::new(RichText::new("Play This Film on DS").color(theme::GROUND).strong()).fill(theme::EMULATOR);
    if ui.add_enabled(can, play).on_hover_text(hint).on_disabled_hover_text("open a film in the Film space, and choose a core").clicked()
        && let Some(film) = film
    {
        emulate.play_film(film);
    }
}

/// The Emulator's page: both screens, or what to do first; the film's making, the key map.
pub fn page(ui: &mut egui::Ui, emulate: &mut Emulate) {
    emulate.hear_making();
    if emulate.core_path().is_none() {
        return no_core(ui, emulate);
    }
    let room = ui.available_size();
    let making_height = if emulate.making.is_some() { 120.0 } else { 0.0 };
    let screen_room = egui::vec2(room.x, (room.y - making_height - 24.0).max(64.0));
    let (rect, response) = ui.allocate_exact_size(screen_room, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::GAP);
    let mut touch = None;
    let step = std::mem::take(&mut emulate.step_asked);
    match (&emulate.core, &emulate.texture) {
        (Some(_), Some(texture)) => {
            let [wide, high] = texture.size().map(|side| side as f32);
            // (Whole times bigger when there is room: crisp pixels.)
            let fit = (rect.width() / wide).min(rect.height() / high);
            let scale = if fit >= 1.0 { fit.floor() } else { fit };
            let shown = egui::Rect::from_center_size(rect.center(), egui::vec2(wide * scale, high * scale));
            painter.image(texture.id(), shown, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            // (The mouse held on the bottom screen: the stylus there.)
            if response.is_pointer_button_down_on()
                && let Some(at) = response.interact_pointer_pos()
                && shown.contains(at)
            {
                let (x, y) = ((at.x - shown.left()) / scale, (at.y - shown.top()) / scale);
                if y >= high / 2.0 {
                    touch = Some((x, y));
                }
            }
        }
        (Some(_), None) => _ = painter.text(rect.center(), egui::Align2::CENTER_CENTER, "starting…", egui::FontId::proportional(14.0), theme::WEAK),
        (None, _) => {
            _ = painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Load a ROM of yours, or open a film in the Film space and click Play This Film on DS",
                egui::FontId::proportional(14.0),
                theme::WEAK,
            )
        }
    }
    if emulate.core.is_some() {
        response.request_focus();
        emulate.run(ui.ctx(), touch, step);
    }
    ui.horizontal(|ui| {
        ui.label(theme::text(KEY_MAP, theme::SMALL, theme::FAINT));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let state = match &emulate.core {
                Some(_) => format!("frame {}{}", emulate.frames, if emulate.paused { " · paused" } else { "" }),
                None => String::new(),
            };
            ui.label(theme::weak(state));
        });
    });
    if let Some(making) = &emulate.making {
        making_log(ui, making);
    }
}

/// The film being put on DS: what it said, its end.
fn making_log(ui: &mut egui::Ui, making: &Making) {
    let (word, colour) = match &making.done {
        None => (format!("Putting {} on DS…", making.film), theme::TEXT),
        Some(Ok(())) => (format!("{} is on DS: playing from power on", making.film), theme::EMULATOR),
        Some(Err(_)) => (format!("{} could not be put on DS", making.film), theme::WRONG),
    };
    ui.label(theme::text(word, theme::BODY, colour).strong());
    egui::ScrollArea::vertical().max_height(96.0).stick_to_bottom(true).show(ui, |ui| {
        for line in &making.lines {
            ui.label(theme::text(line, theme::SMALL, theme::WEAK));
        }
        if let Some(Err(why)) = &making.done {
            ui.label(theme::text(why, theme::SMALL, theme::WRONG));
        }
    });
}

/// No core yet: what one is, where to get one, how to tell the Workshop.
fn no_core(ui: &mut egui::Ui, emulate: &mut Emulate) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.set_max_width(620.0);
        ui.label(theme::text("The Emulator needs a libretro core", 18.0, theme::STRONG));
        ui.add_space(8.0);
        for line in [
            "The Workshop runs DS games through a libretro core, a small library that is the emulator itself. None comes with the Workshop: choose one yourself.",
            "Get the melonDS (or DeSmuME) core from libretro's site: buildbot.libretro.com, nightly, your system, latest, melonds_libretro (a .zip holding melonds_libretro.dll, .so or .dylib). Unzip it anywhere of yours.",
            "melonDS plays without BIOS or firmware files (it has its own FreeBIOS).",
            "Then choose the file below, or set WORKSHOP_LIBRETRO_CORE to its path before starting the Workshop.",
        ] {
            ui.label(theme::text(line, theme::BODY, theme::TEXT));
            ui.add_space(4.0);
        }
        ui.add_space(8.0);
        if ui.button(RichText::new("Choose a Core…").strong()).clicked() {
            emulate.choose_core();
        }
        ui.add_space(4.0);
        let typed = ui.add(egui::TextEdit::singleline(&mut emulate.core_typed).hint_text("or its path, then Enter").desired_width(480.0));
        if typed.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            let core = PathBuf::from(emulate.core_typed.trim().trim_matches('"'));
            match core.is_file() {
                true => emulate.remember(settings::CORE, &core),
                false => emulate.said = format!("no file at {}", core.display()),
            }
        }
        if let Some(file) = &emulate.settings.file {
            ui.label(theme::text(format!("remembered in {}", file.display()), theme::SMALL, theme::FAINT));
        }
    });
}
