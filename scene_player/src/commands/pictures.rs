//! What a film gives to be seen or sent: pictures, a video, a packet, a window.

use std::sync::Arc;

use ::film::Context;
use std::io::Write;
use std::path::Path;
use std::process::{Command as Process, Stdio};

use image::RgbImage;
use shell::sound::{Audio, Hears, Recorder};
use shell::{Keys, Screen};
use stage_view::ui::Canvas;

use super::checking::{refused, rehearsed, show};
use super::{Piece, out_folder};

use crate::moment;
use crate::window::watch;
use game::Game;

use preview::film::Film;
use preview::played::{Pass, SECOND, seconds};

/// A sheet: pictures a line. What a picture alone is scaled by.
const COLUMNS: usize = 5;
const PICTURE_SCALE: usize = 4;

/// The digits and the `s` of a picture's time, three by five.
const LETTERS: [(char, [u8; 5]); 11] = [
    ('0', [7, 5, 5, 5, 7]),
    ('1', [2, 6, 2, 2, 7]),
    ('2', [7, 1, 7, 4, 7]),
    ('3', [7, 1, 7, 1, 7]),
    ('4', [5, 5, 7, 1, 1]),
    ('5', [7, 4, 7, 1, 7]),
    ('6', [7, 4, 7, 5, 7]),
    ('7', [7, 1, 1, 1, 1]),
    ('8', [7, 5, 7, 5, 7]),
    ('9', [7, 5, 7, 1, 7]),
    ('s', [0, 3, 6, 1, 6]),
];

/// Words of digits written on a picture, sharp, on a dark band.
pub fn write_on(picture: &mut RgbImage, at: (u32, u32), text: &str, size: u32) {
    let wide = (text.len() as u32 * 4 * size + 2 * size).min(picture.width().saturating_sub(at.0));
    for y in 0..(7 * size).min(picture.height().saturating_sub(at.1)) {
        for x in 0..wide {
            picture.put_pixel(at.0 + x, at.1 + y, image::Rgb([0, 0, 0]));
        }
    }
    for (place, letter) in text.chars().enumerate() {
        let Some((_, rows)) = LETTERS.iter().find(|(known, _)| *known == letter) else {
            continue;
        };
        for (y, row) in rows.iter().enumerate() {
            for x in (0..3).filter(|x| row & (4 >> x) != 0) {
                for dy in 0..size {
                    for dx in 0..size {
                        let (px, py) = (at.0 + size + (place as u32 * 4 + x) * size + dx, at.1 + size + y as u32 * size + dy);
                        if px < picture.width() && py < picture.height() {
                            picture.put_pixel(px, py, image::Rgb([255, 255, 255]));
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn rgb(screen: &Screen) -> Vec<u8> {
    screen.rgba.as_chunks::<4>().0.iter().flat_map(|&[red, green, blue, _]| [red, green, blue]).collect()
}

/// A picture of the film, kept every so often from a frame to another: all
/// but what is wholly dark. How many.
fn sheet(mut pass: Pass, every: usize, (first, last): (usize, usize), out: &Path) -> Result<usize, String> {
    let (mut canvas, mut screen) = (Canvas::default(), Screen::new(1));
    let mut kept: Vec<(Vec<u8>, usize)> = Vec::new();
    while pass.step() && pass.frame <= last {
        if pass.frame >= first && pass.frame.is_multiple_of(every) {
            pass.film.draw(&mut canvas, &mut screen);
            let picture = rgb(&screen);
            if picture.iter().any(|part| *part > 8) {
                kept.push((picture, pass.frame / SECOND));
            }
        }
    }
    let (wide, high) = (screen.wide as u32, screen.high as u32);
    let mut whole = RgbImage::from_pixel(COLUMNS as u32 * wide, kept.len().div_ceil(COLUMNS).max(1) as u32 * high, image::Rgb([255, 255, 255]));
    for (at, (picture, second)) in kept.iter().enumerate() {
        let place = ((at % COLUMNS) as u32 * wide, (at / COLUMNS) as u32 * high);
        let shown = RgbImage::from_raw(wide, high, picture.clone()).ok_or("a badly made image")?;
        image::imageops::replace(&mut whole, &shown, place.0 as i64, place.1 as i64);
        write_on(&mut whole, place, &format!("{second}s"), 2);
    }
    whole.save(out).map_err(|error| format!("{}: {error}", out.display()))?;
    Ok(kept.len())
}

pub fn view(piece: &Piece, scene: usize, every: usize, context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    let problems = refused(piece, data)?;
    let told = rehearsed(piece)?;
    let mut out = String::new();
    let wrong = show(&mut out, piece, &told, &problems, false);
    print!("{out}");
    let timeline = &told.timeline;
    if scene > timeline.scenes.len() {
        return Err(format!("--scene {scene}: {} scene(s) in all", timeline.scenes.len()));
    }
    let range = match scene {
        0 => (0, timeline.frames),
        scene => (timeline.scenes[scene - 1].start, timeline.end_of(scene)),
    };
    let out = out_folder(context, &piece.name)?.join("sheet.png");
    let every = every.max(1);
    let kept = sheet(Pass::new(piece.film()?), every * SECOND, range, &out)?;
    println!("{kept} images, one every {every} s: {}", out.display());
    match wrong {
        0 => Ok(()),
        wrong => Err(format!("{}: {wrong} problem(s)", piece.name)),
    }
}

/// A sheet of `try`: so many a line; how much of the place each picture
/// shows close on someone (`--on`), wide and high.
const TRIED_COLUMNS: u32 = 4;
const CLOSE_ON: (u32, u32) = (80, 64);

/// `try <film> [moments] [--on who]`: does it work, quickly: what `check`
/// says, then those moments (eight across the film, none told) on one sheet,
/// each with its second and the line playing; `--on`: each close on him.
pub fn try_out(piece: &Piece, asked: &[String], on: Option<&String>, context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    let problems = refused(piece, data)?;
    let told = rehearsed(piece)?;
    let mut out = String::new();
    let wrong = show(&mut out, piece, &told, &problems, false);
    print!("{out}");
    let timeline = &told.timeline;
    let mut frames: Vec<usize> = match asked.is_empty() {
        true => (1..=8).map(|at| timeline.frames * at / 9).collect(),
        false => moment::read(asked)?.iter().map(|moment| moment.frame(timeline)).collect::<Result<_, _>>()?,
    };
    frames.sort_unstable();
    let written = std::fs::read_to_string(&piece.path).unwrap_or_default();
    let lines: Vec<&str> = written.lines().collect();
    let mut pass = Pass::new(piece.film()?);
    let (mut canvas, mut screen) = (Canvas::default(), Screen::new(PICTURE_SCALE));
    let mut kept: Vec<(RgbImage, usize)> = Vec::new();
    for frame in frames {
        while pass.frame < frame && pass.step() {}
        // (Who to look at closely, where: the camera on him, even when the
        // film's own leaves him out of its frame.)
        let close = on.map(|name| {
            let film = &pass.film;
            let who = film.play().sim.stage.cast.keys().copied().find(|who| super::looking::name_of(piece, film, *who) == *name);
            who.and_then(|who| film.play().sim.stage.cast.get(&who)).map(|actor| actor.shown_at() - glam::Vec2::new(0.0, actor.shown_height() + 12.0))
        });
        let view = match close.flatten() {
            Some(at) => Some((pass.film.draw_around(&mut canvas, &mut screen, at), at)),
            None => {
                pass.film.draw(&mut canvas, &mut screen);
                None
            }
        };
        let whole = RgbImage::from_raw(screen.wide as u32, screen.high as u32, rgb(&screen)).ok_or("a badly made image")?;
        let film = &pass.film;
        let line = film.play().sim.director.numbers().get(engine::LINE).map_or(0, |line| *line as i32);
        let text = usize::try_from(line).ok().and_then(|line| lines.get(line.checked_sub(1)?)).map_or("", |text| text.trim());
        println!("{}. {} s, line {line}: {text}", kept.len() + 1, seconds(frame));
        let picture = match (on, view) {
            (Some(_), Some((view, at))) => {
                let at = view.on_screen(at);
                let scale = PICTURE_SCALE as f32;
                let (wide, high) = (CLOSE_ON.0 * PICTURE_SCALE as u32, CLOSE_ON.1 * PICTURE_SCALE as u32);
                let left = ((at.x * scale) as i64 - wide as i64 / 2).clamp(0, (whole.width() - wide) as i64) as u32;
                let top = ((at.y * scale) as i64 - high as i64 / 2).clamp(0, (whole.height() - high) as i64) as u32;
                image::imageops::crop_imm(&whole, left, top, wide, high).to_image()
            }
            (Some(name), None) => {
                println!("   ({name} is not there: the whole image)");
                image::imageops::resize(&whole, whole.width() / 2, whole.height() / 2, image::imageops::FilterType::Nearest)
            }
            (None, _) => image::imageops::resize(&whole, whole.width() / 2, whole.height() / 2, image::imageops::FilterType::Nearest),
        };
        kept.push((picture, frame));
    }
    let (wide, high) = kept.first().map_or((1, 1), |(picture, _)| picture.dimensions());
    let rows = (kept.len() as u32).div_ceil(TRIED_COLUMNS).max(1);
    let mut sheet = RgbImage::from_pixel(TRIED_COLUMNS.min(kept.len().max(1) as u32) * wide, rows * high, image::Rgb([255, 255, 255]));
    for (at, (picture, frame)) in kept.iter().enumerate() {
        let place = ((at as u32 % TRIED_COLUMNS) * wide, (at as u32 / TRIED_COLUMNS) * high);
        image::imageops::replace(&mut sheet, picture, place.0 as i64, place.1 as i64);
        write_on(&mut sheet, place, &format!("{}s", frame / SECOND), 2);
    }
    let out = out_folder(context, &piece.name)?.join("try.png");
    sheet.save(&out).map_err(|error| format!("{}: {error}", out.display()))?;
    println!("sheet: {}", out.display());
    match wrong {
        0 => Ok(()),
        wrong => Err(format!("{}: {wrong} problem(s)", piece.name)),
    }
}

/// Those moments of a film, a picture each at full size: in one pass when
/// they are seconds; a line or a scene asks the whole film played first, to
/// know when it comes.
pub fn image(piece: &Piece, asked: &[String], context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    refused(piece, data)?;
    let moments = moment::read(asked)?;
    let timed = moments.iter().all(|moment| matches!(moment, moment::Moment::Second(_)));
    let timeline = match timed {
        true => None,
        false => {
            let mut timing = Pass::new(piece.film()?);
            timing.to_the_end();
            Some(timing.timeline)
        }
    };
    let mut wanted = moments
        .iter()
        .map(|moment| match (&timeline, moment) {
            (Some(timeline), _) => Ok((moment.frame(timeline)?, moment.name())),
            (None, moment::Moment::Second(second)) => Ok((((second * SECOND as f32) as usize).max(1), moment.name())),
            (None, _) => unreachable!("only seconds are found without the timeline"),
        })
        .collect::<Result<Vec<(usize, String)>, String>>()?;
    wanted.sort();
    let folder = out_folder(context, &piece.name)?;
    let mut pass = Pass::new(piece.film()?);
    let (mut canvas, mut screen) = (Canvas::default(), Screen::new(PICTURE_SCALE));
    for (frame, name) in wanted {
        while pass.frame < frame && pass.step() {}
        if pass.frame < frame {
            return Err(format!("{name}: the film ends at {} s", seconds(pass.frame)));
        }
        pass.film.draw(&mut canvas, &mut screen);
        let out = folder.join(format!("image_{name}.png"));
        image::save_buffer(&out, &rgb(&screen), screen.wide as u32, screen.high as u32, image::ColorType::Rgb8)
            .map_err(|error| format!("{}: {error}", out.display()))?;
        println!("{} s: {}", seconds(frame), out.display());
    }
    Ok(())
}

pub fn play(piece: &Piece, auto: bool, from: Option<&String>, scale: usize, context: &Arc<Context>, debug: &preview::debug::Debug) -> Result<(), String> {
    let data = context.data.as_str();
    watch(
        || {
            let heard = Audio::open_in(data, piece.tables.project.clone()).map(|audio| Box::new(audio) as Box<dyn Hears>);
            let mut film = Film::open(&piece.path, piece.art()?, heard, from.cloned(), piece.tables.clone())?;
            film.auto = auto;
            piece.known(&mut film);
            Ok(Game::of_film(film, context))
        },
        |art, audio| {
            let mut film = Film::open(&piece.path, art, audio, None, piece.tables.clone())?;
            piece.known(&mut film);
            Ok(film)
        },
        scale.clamp(1, 8),
        debug,
    )
}

/// The most frames a game is played with nobody at the keys (an hour).
const GAME_FRAMES: usize = 60 * 60 * 60;

/// A game (`<name>.game`) in its window, on its title screen (`--new`:
/// straight from its start; `--chapter n`: a new game from that chapter, at
/// its home); `--headless`: played with nobody at the keys, no window, what
/// happened said in order.
pub fn play_game(
    path: &Path,
    headless: bool,
    fresh: bool,
    chapter: Option<u16>,
    auto: bool,
    scale: usize,
    context: &Arc<Context>,
    debug: &preview::debug::Debug,
) -> Result<(), String> {
    let data = context.data.as_str();
    if headless {
        let mut game = Game::open(context, path, None)?;
        game.debug = debug.clone();
        if let Some(chapter) = chapter {
            game.new_game_at(chapter)?;
        }
        game.play_alone(GAME_FRAMES);
        for line in &game.log {
            println!("{line}");
        }
        if let Some(said) = game.said.take() {
            println!("{said}");
        }
        return match game.over || game.rested {
            true => Ok(()),
            false => Err(format!("{}: still going after {} s", path.display(), seconds(GAME_FRAMES))),
        };
    }
    watch(
        || {
            let heard = Audio::open_in(data, context.library.of_game(path)).map(|audio| Box::new(audio) as Box<dyn Hears>);
            let mut game = Game::open(context, path, heard)?;
            game.auto = auto;
            match chapter {
                Some(chapter) => game.new_game_at(chapter)?,
                None if fresh => game.skip_title(),
                None => {}
            }
            Ok(game)
        },
        |_, _| Err("a game is not played again from a moment".to_string()),
        scale.clamp(1, 8),
        debug,
    )
}

/// The film made into a video, with all it has to be heard. `ffmpeg` does
/// the writing.
pub fn video(piece: &Piece, scale: usize, context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    /// The frames kept on the last picture.
    const AFTER: usize = 180;
    let out = out_folder(context, &piece.name)?.join(format!("{}.mp4", piece.name)).to_string_lossy().into_owned();
    let heard = Recorder::open_in(data, piece.tables.project.clone()).map(|recorder| Box::new(recorder) as Box<dyn Hears>);
    let mut film = Film::open(&piece.path, piece.art()?, heard, None, piece.tables.clone())?;
    let (pictures, sound) = (format!("{out}.images.mp4"), format!("{out}.wav"));
    let mut canvas = Canvas::default();
    let mut screen = Screen::new(scale.clamp(1, 8));
    let size = format!("{}x{}", screen.wide, screen.high);
    let raw = ["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba", "-s", &size, "-r", "60", "-i", "-"];
    let coded = ["-c:v", "libx264", "-preset", "veryfast", "-crf", "21", "-pix_fmt", "yuv420p", &pictures];
    let lost = |error: std::io::Error| format!("ffmpeg: {error} (ffmpeg is needed in the PATH)");
    let mut writer = Process::new("ffmpeg").args(raw).args(coded).stdin(Stdio::piped()).spawn().map_err(lost)?;
    let mut into = writer.stdin.take().ok_or("ffmpeg: no input")?;
    let (mut frame, mut ended) = (0, 0);
    film.auto = true;
    while ended < AFTER {
        if frame >= preview::played::MOST_FRAMES {
            println!("the film does not end: cut at {} s", seconds(frame));
            break;
        }
        // (A choice is made a moment after it shows, as `check` and `view` make it.)
        let enter = film.asking() && frame % 90 == 89;
        film.tick(Keys { enter, ..Keys::default() }, 1.0);
        film.draw(&mut canvas, &mut screen);
        into.write_all(&screen.rgba).map_err(lost)?;
        ended += usize::from(film.over);
        frame += 1;
    }
    drop(into);
    writer.wait().map_err(lost)?;
    // (Nothing to be heard, no sound.bin: the silent video is the video.)
    let Some(audio) = &film.audio else {
        _ = std::fs::remove_file(&out);
        std::fs::rename(&pictures, &out).map_err(|error| format!("{out}: {error}"))?;
        println!("{} s, without sound (no sound.bin): {out}", seconds(frame));
        return Ok(());
    };
    audio.write_to(&sound)?;
    let both = ["-y", "-loglevel", "error", "-i", &pictures, "-i", &sound, "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest", &out];
    let done = Process::new("ffmpeg").args(both).status().map_err(lost)?;
    _ = std::fs::remove_file(&pictures);
    _ = std::fs::remove_file(&sound);
    match done.success() {
        true => {
            println!("{} s: {out}", seconds(frame));
            Ok(())
        }
        false => Err("ffmpeg could not put the video together".to_string()),
    }
}

pub fn pack(piece: &Piece, context: &Arc<Context>) -> Result<(), String> {
    let data = context.data.as_str();
    refused(piece, data)?;
    let dist = context.root().join("dist");
    let out = dist.join(&piece.name);
    _ = std::fs::remove_dir_all(&out);
    crate::pack::pack(&piece.path, &piece.tables, data, &out.to_string_lossy(), &piece.art()?)?;
    let zip = dist.join(format!("{}.zip", piece.name));
    let done = Process::new("tar").arg("-a").arg("-cf").arg(&zip).arg("-C").arg(&dist).arg(&piece.name).status();
    match done {
        Ok(done) if done.success() => println!("{}", zip.display()),
        _ => println!("{} (no zip: tar is missing)", out.display()),
    }
    Ok(())
}

/// The game (a `.game`, by its path or its name: `games/<name>/`, then `library`) packed
/// to share: `dist/<game>.dwpack`, its own files only (`game::pack`).
pub fn pack_game(game: &str, context: &Arc<Context>) -> Result<(), String> {
    let path = [Some(Path::new(game).to_path_buf()), context.library.find_game(game)]
        .into_iter()
        .flatten()
        .find(|path| path.is_file())
        .ok_or_else(|| format!("no game {game}: a path to a .game, or a name in games/ (or library/)"))?;
    let stem = path.file_stem().map_or("game".into(), |stem| stem.to_string_lossy().into_owned());
    let out = context.root().join("dist").join(format!("{stem}.dwpack"));
    let packed = game::pack::pack_game(context, &path, &out)?;
    for file in &packed.files {
        println!("  {file}");
    }
    println!("{}: “{}”, {} files, nothing of the game's data", packed.path.display(), packed.title, packed.files.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moment_as_written_is_told_on_a_picture() {
        let mut picture = RgbImage::new(40, 20);
        write_on(&mut picture, (0, 0), "12s", 2);
        assert!(picture.pixels().any(|pixel| pixel.0 == [255, 255, 255]));
    }
}
