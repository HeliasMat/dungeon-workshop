//! A film, or a game, played in a window, as fast as it goes by.

use std::time::Instant;

use shell::sound::Hears;
use shell::{Key, Screen, Window};
use stage_view::ui::Canvas;

use game::Game;
use preview::art::Art;
use preview::film::Film;

/// How far Page Up and Page Down go, in frames.
const LEAP: f32 = 600.0;

/// The film at that moment: played from the start again if it was before.
fn gone_to(film: Film, frame: f32, reopen: &impl Fn(Art, Option<Box<dyn Hears>>) -> Result<Film, String>) -> Result<Film, String> {
    let (auto, debug) = (film.auto, film.debug.clone());
    let mut film = match frame < film.played() {
        true => {
            let (art, audio) = film.parts();
            reopen(art, audio)?
        }
        false => film,
    };
    (film.auto, film.debug) = (auto, debug);
    film.skip_while(|film| film.played() < frame);
    Ok(film)
}

/// The film at the start of that scene (from 1): played on to it, or from
/// the start again if it was before; at its end if it has fewer.
pub fn to_scene(film: Film, scene: usize, reopen: &impl Fn(Art, Option<Box<dyn Hears>>) -> Result<Film, String>) -> Result<Film, String> {
    let mut film = match film.scenes().get(scene - 1) {
        Some(_) => gone_to(film, 0.0, reopen)?,
        None => film,
    };
    film.skip_while(|film| film.scenes().len() < scene);
    Ok(film)
}

/// The film of a game alone, taken to another moment by `with`, put back.
fn moved(game: &mut Game, with: impl FnOnce(Film) -> Result<Film, String>) -> Result<(), String> {
    if let Some(film) = game.take_film() {
        game.put_film(with(film)?);
    }
    Ok(())
}

/// `open`: the game (a film alone, or a game's file) from its start, heard;
/// `reopen`: the film alone again, with what it has; `debug`: the
/// developer's options it starts with.
pub fn watch(
    open: impl Fn() -> Result<Game, String>,
    reopen: impl Fn(Art, Option<Box<dyn Hears>>) -> Result<Film, String>,
    scale: usize,
    debug: &preview::debug::Debug,
) -> Result<(), String> {
    let mut game = open()?;
    game.debug = debug.clone();
    if let Some(film) = game.film_mut() {
        film.debug = debug.clone();
    }
    let alone = game.project.is_none();
    // (`DUNGEON_FILM_DEBUG=scene=3`: from that scene, a new game.)
    if let Some(scene) = game.debug.scene.filter(|_| alone) {
        game.title = None;
        moved(&mut game, |film| to_scene(film, scene, &reopen))?;
    }
    let mut screen = Screen::new(scale);
    let mut window = Window::open(if alone { "Scene" } else { "Game" }, &screen)?;
    let mut canvas = Canvas::default();
    let mut last = Instant::now();
    let mut paused = false;
    while window.is_open() {
        let keys = window.keys();
        // (In a dungeon, Esc is its own (its menu: moves, bag…), and the keys
        // that move the film about do nothing: Shift+Esc leaves.)
        let diving = game.diving.is_some();
        // (A game: only a film alone is moved about in time.)
        let free = alone && !diving && game.title.is_none();
        if keys.escape && (!diving || keys.run) {
            break;
        }
        if window.pressed(Key::F2) {
            game.auto = !game.auto;
            let auto = game.auto;
            if let Some(film) = game.film_mut() {
                film.auto = auto;
            }
        }
        if free && window.pressed(Key::F5) {
            let (auto, debug) = (game.auto, game.debug.clone());
            game = open()?;
            game.title = None;
            (game.auto, game.debug) = (auto, debug);
        }
        // (Whoever writes the film: F6 the view of what it holds, F7 lines
        // at once, F8 no fades, F9 a dungeon's floors gone through (in one,
        // down at once); Ctrl and a digit, that scene.)
        if window.pressed(Key::F6) {
            game.debug.shown = !game.debug.shown;
        }
        if window.pressed(Key::F7) {
            game.debug.fast_text = !game.debug.fast_text;
        }
        if window.pressed(Key::F8) {
            game.debug.no_fades = !game.debug.no_fades;
        }
        if window.pressed(Key::F9) {
            match &mut game.diving {
                Some(diving) => diving.skip_floor(),
                None => game.debug.floors = !game.debug.floors,
            }
        }
        let debug = game.debug.clone();
        if let Some(film) = game.film_mut() {
            (film.debug.shown, film.debug.fast_text, film.debug.no_fades, film.debug.floors) = (debug.shown, debug.fast_text, debug.no_fades, debug.floors);
        }
        let ctrl = keys.held.iter().any(|key| key.ends_with("ctrl"));
        if let Some(scene) = keys.named.iter().find_map(|key| key.parse::<usize>().ok()).filter(|scene| ctrl && free && *scene > 0) {
            moved(&mut game, |film| to_scene(film, scene, &reopen))?;
        }
        let pressed = |key| free && window.repeated(key);
        paused ^= window.pressed(Key::P);
        if pressed(Key::PageDown) {
            moved(&mut game, |film| {
                let frame = film.played() + LEAP;
                gone_to(film, frame, &reopen)
            })?;
        }
        if pressed(Key::PageUp) {
            moved(&mut game, |film| {
                let frame = (film.played() - LEAP).max(0.0);
                gone_to(film, frame, &reopen)
            })?;
        }
        if pressed(Key::Home) {
            moved(&mut game, |film| gone_to(film, 0.0, &reopen))?;
        }
        if pressed(Key::F4) {
            moved(&mut game, |mut film| {
                let seen = film.scenes().len();
                film.skip_while(|film| film.scenes().len() == seen);
                Ok(film)
            })?;
        }
        if pressed(Key::F3) {
            // (The start of the last scene, or of the one before just after it.)
            moved(&mut game, |film| {
                let now = film.played();
                let start = film.scenes().iter().rev().copied().find(|start| *start < now - 120.0).unwrap_or(0.0);
                gone_to(film, start, &reopen)
            })?;
        }
        if let Some(said) = game.said.take() {
            println!("{said}");
        }
        let frames = (last.elapsed().as_secs_f32() * 60.0).clamp(0.0, 4.0);
        last = Instant::now();
        if !paused {
            game.tick(keys, frames);
        } else if pressed(Key::Period) {
            game.tick(keys, 1.0);
        }
        // (A game over: its window closes; a film alone stays on its end.)
        if game.over && !alone {
            break;
        }
        game.draw(&mut canvas, &mut screen);
        let seconds = game.played() as u32 / 60;
        match &game.diving {
            Some(diving) => {
                let floor = diving.floor().map_or(String::new(), |(at, of)| format!(" · floor {at}/{of}"));
                window.set_title(&format!("Dungeon{floor} · Esc: its menu · Shift+Esc: quit{}", if paused { " (paused)" } else { "" }));
            }
            None if !alone => window.set_title(&format!("Game · chapter {} · {:?}{}", game.story.chapter, game.mode(), if paused { " (paused)" } else { "" })),
            None => window.set_title(&format!("Scene {}:{:02}{} · F6: debug", seconds / 60, seconds % 60, if paused { " (paused)" } else { "" })),
        }
        window.show(&screen)?;
    }
    Ok(())
}
