//! File › Export Video: the film open made a video (`scene_player video
//! <film>`, beside the Workshop) in the background, how far it is, where it
//! is written (`target/films/<film>/<film>.mp4`); ffmpeg is needed, said
//! plainly when it is missing.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use eframe::egui::{self, RichText};
use workshop_base::theme;

/// What the export says as it goes.
enum Told {
    /// Seconds of film made.
    At(f64),
    Done(Result<PathBuf, String>),
}

/// A video being made, then made (or why not).
pub struct Export {
    pub film: PathBuf,
    /// How long the film is (seconds), if known: how far it is, as a share.
    length: Option<f64>,
    at: f64,
    told: Receiver<Told>,
    pub done: Option<Result<PathBuf, String>>,
}

/// What ffmpeg's absence is said as.
pub const NO_FFMPEG: &str = "Export Video needs ffmpeg, which was not found: install it (ffmpeg.org; on Windows `winget install ffmpeg`) so that `ffmpeg` runs in a terminal, then try again.";

/// Where a line of `scene_player video` says it is: `at 12.5 s`.
pub fn seconds_said(line: &str) -> Option<f64> {
    line.trim().strip_prefix("at ")?.strip_suffix(" s")?.trim().parse().ok()
}

/// The video written, from `scene_player video`'s last line (`12.5 s: <path>.mp4`, or `… without sound (…): <path>.mp4`).
pub fn written_said(line: &str) -> Option<PathBuf> {
    let (_, path) = line.trim().rsplit_once(": ")?;
    path.ends_with(".mp4").then(|| PathBuf::from(path.trim()))
}

impl Export {
    /// The film's video begun, on a thread: ffmpeg looked for, `scene_player video` run, what it says read.
    pub fn start(film: &Path, length: Option<f64>) -> Export {
        let (tell, told) = channel();
        let path = film.to_path_buf();
        std::thread::spawn(move || {
            let done = run(&path, &|seconds| _ = tell.send(Told::At(seconds)));
            _ = tell.send(Told::Done(done));
        });
        Export { film: film.to_path_buf(), length: length.filter(|length| *length > 0.0), at: 0.0, told, done: None }
    }

    /// What it said meanwhile, taken.
    pub fn poll(&mut self) {
        while let Ok(told) = self.told.try_recv() {
            match told {
                Told::At(seconds) => self.at = seconds,
                Told::Done(done) => self.done = Some(done),
            }
        }
    }
}

/// `scene_player video <film>`, run to its end; each second of film said as it goes.
fn run(film: &Path, at: &dyn Fn(f64)) -> Result<PathBuf, String> {
    if std::process::Command::new("ffmpeg").arg("-version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_err() {
        return Err(NO_FFMPEG.to_string());
    }
    let program = workshop_first::program_beside("scene_player")?;
    let mut child = std::process::Command::new(program)
        .arg("video")
        .arg(film)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|why| format!("scene_player: {why}"))?;
    let mut last = String::new();
    if let Some(out) = child.stdout.take() {
        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
            match seconds_said(&line) {
                Some(seconds) => at(seconds),
                None if !line.trim().is_empty() => last = line,
                None => {}
            }
        }
    }
    let mut wrong = String::new();
    if let Some(mut err) = child.stderr.take() {
        _ = std::io::Read::read_to_string(&mut err, &mut wrong);
    }
    let status = child.wait().map_err(|why| format!("scene_player: {why}"))?;
    match written_said(&last) {
        Some(path) if status.success() => Ok(path),
        _ if wrong.contains("ffmpeg") => Err(NO_FFMPEG.to_string()),
        _ => Err(format!("the video was not made: {}", if wrong.trim().is_empty() { last.trim() } else { wrong.trim() })),
    }
}

/// The export's window, while there is one: the film, how far, where it is written; Open the Folder once done.
/// Whether it is to close.
pub fn window(context: &egui::Context, export: &mut Export) -> bool {
    export.poll();
    let mut open = true;
    let name = export.film.file_name().unwrap_or_default().to_string_lossy().into_owned();
    egui::Window::new(RichText::new("Export Video").strong())
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -32.0))
        .show(context, |ui| {
            ui.set_width(420.0);
            ui.label(theme::text(&name, theme::BODY, theme::STRONG));
            match &export.done {
                None => {
                    let share = export.length.map(|length| (export.at / length).clamp(0.0, 1.0) as f32);
                    let said = match export.length {
                        Some(length) => format!("{:.0} s of {:.0} s made", export.at, length),
                        None => format!("{:.0} s made", export.at),
                    };
                    ui.add(egui::ProgressBar::new(share.unwrap_or(0.0)).text(said).animate(true));
                    ui.label(theme::weak("in the background: go on working; written to target/films/<film>/<film>.mp4"));
                    context.request_repaint_after(std::time::Duration::from_millis(250));
                }
                Some(Ok(path)) => {
                    ui.label(theme::text("Done.", theme::BODY, theme::FILM));
                    ui.add(egui::Label::new(theme::text(path.display().to_string(), theme::SMALL, theme::TEXT)).wrap());
                    ui.horizontal(|ui| {
                        if ui.button("Play It").clicked() {
                            open_outside(path);
                        }
                        if ui.button("Open the Folder").clicked()
                            && let Some(folder) = path.parent()
                        {
                            open_outside(folder);
                        }
                    });
                }
                Some(Err(why)) => {
                    ui.add(egui::Label::new(theme::text(why, theme::SMALL, theme::WRONG)).wrap());
                }
            }
        });
    !open
}

/// A file or a folder opened by the system's own program.
fn open_outside(path: &Path) {
    let made = if cfg!(windows) {
        std::process::Command::new("cmd").args(["/C", "start", ""]).arg(path).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(path).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(path).spawn()
    };
    _ = made;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `scene_player video` says, read: each second made, the video written.
    #[test]
    fn what_the_video_says_is_read() {
        assert_eq!(seconds_said("at 12.5 s"), Some(12.5));
        assert_eq!(seconds_said("the film does not end: cut at 300 s"), None);
        assert_eq!(written_said("12.5 s: C:/w/target/films/a/a.mp4"), Some(PathBuf::from("C:/w/target/films/a/a.mp4")));
        assert_eq!(written_said("3 s, without sound (no sound.bin): /w/a.mp4"), Some(PathBuf::from("/w/a.mp4")));
        assert_eq!(written_said("ffmpeg: not found"), None);
    }
}
