//! The window, and what is pressed in it.

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec2;
pub use minifb::Key;
use minifb::{InputCallback, KeyRepeat, WindowOptions};

use crate::Screen;

/// What the player presses, this frame.
#[derive(Default, Clone)]
pub struct Keys {
    pub enter: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub escape: bool,
    pub tab: bool,
    /// The way the arrows held go, and whether Shift is held.
    pub toward: Vec2,
    pub run: bool,
    /// Space held.
    pub hurry: bool,
    /// The keys pressed, by their names.
    pub named: Vec<String>,
    /// The keys down, by their names.
    pub held: Vec<String>,
    /// The letters written.
    pub typed: Vec<char>,
}

/// The names a key goes by.
fn key_names(key: Key) -> Vec<String> {
    let name = format!("{key:?}").to_lowercase();
    match name.as_str() {
        "space" => vec![name],
        "backspace" => vec![name],
        _ => vec![name.strip_prefix("key").unwrap_or(&name).to_string()],
    }
}

/// The letters written since they were last taken.
#[derive(Clone, Default)]
struct Letters(Rc<RefCell<Vec<char>>>);

impl InputCallback for Letters {
    fn add_char(&mut self, letter: u32) {
        self.0.borrow_mut().extend(char::from_u32(letter).filter(|letter| !letter.is_control()));
    }
}

/// A window showing a screen.
pub struct Window {
    window: minifb::Window,
    pixels: Vec<u32>,
    letters: Letters,
    title: String,
}

impl Window {
    pub fn open(title: &str, screen: &Screen) -> Result<Self, crate::Error> {
        let mut window =
            minifb::Window::new(title, screen.wide, screen.high, WindowOptions::default()).map_err(|error| crate::Error::Window(error.to_string()))?;
        let letters = Letters::default();
        window.set_input_callback(Box::new(letters.clone()));
        Ok(Window { window, pixels: vec![0; screen.wide * screen.high], letters, title: title.to_string() })
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }

    /// Whether a key has just gone down.
    pub fn pressed(&self, key: Key) -> bool {
        self.window.is_key_pressed(key, KeyRepeat::No)
    }

    /// The same, and again as it stays down.
    pub fn repeated(&self, key: Key) -> bool {
        self.window.is_key_pressed(key, KeyRepeat::Yes)
    }

    /// What is pressed now.
    pub fn keys(&self) -> Keys {
        let held = |key| self.window.is_key_down(key);
        let axis = |less, more| f32::from(u8::from(held(more))) - f32::from(u8::from(held(less)));
        Keys {
            enter: self.pressed(Key::Enter) || self.pressed(Key::Space),
            up: self.pressed(Key::Up),
            down: self.pressed(Key::Down),
            left: self.pressed(Key::Left),
            right: self.pressed(Key::Right),
            escape: self.pressed(Key::Escape),
            tab: self.pressed(Key::Tab),
            toward: Vec2::new(axis(Key::Left, Key::Right), axis(Key::Up, Key::Down)),
            run: held(Key::LeftShift) || held(Key::RightShift),
            hurry: held(Key::Space),
            named: self.window.get_keys_pressed(KeyRepeat::No).into_iter().flat_map(key_names).collect(),
            held: self.window.get_keys().into_iter().flat_map(key_names).collect(),
            typed: self.letters.0.borrow_mut().drain(..).collect(),
        }
    }

    pub fn set_title(&mut self, title: &str) {
        if title != self.title {
            self.window.set_title(title);
            self.title = title.to_string();
        }
    }

    /// The screen, shown.
    pub fn show(&mut self, screen: &Screen) -> Result<(), crate::Error> {
        for (pixel, &[red, green, blue, _]) in self.pixels.iter_mut().zip(screen.rgba.as_chunks::<4>().0) {
            *pixel = (red as u32) << 16 | (green as u32) << 8 | blue as u32;
        }
        self.window.update_with_buffer(&self.pixels, screen.wide, screen.high).map_err(|error| crate::Error::Window(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_go_by_the_names_scenes_give_them() {
        assert_eq!(key_names(Key::Space), ["space"]);
        assert_eq!(key_names(Key::Backspace), ["backspace"]);
        assert_eq!(key_names(Key::A), ["a"]);
        assert_eq!(key_names(Key::Key1), ["1"]);
    }

    #[test]
    fn only_letters_are_written() {
        let mut letters = Letters::default();
        letters.add_char('é' as u32);
        letters.add_char(8);
        assert_eq!(*letters.0.borrow(), ['é']);
    }
}
