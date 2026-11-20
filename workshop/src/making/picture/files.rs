//! A picture written and read back: its strip of steps, its layers and
//! their times kept beside to be painted again.

use std::path::PathBuf;

use image::RgbaImage;

use super::pixels::{composite, strip};
use super::{Layer, Painting};

/// Whether pictures are painted as an expert (kept).
pub(super) fn picture_expert_file(root: &std::path::Path) -> PathBuf {
    root.join("target").join("workshop").join("painting_expert.txt")
}

impl Painting {
    /// Written as the things drawn are; its layers and times kept beside, in the Workshop's own.
    pub(super) fn save(&mut self) -> Result<String, String> {
        self.drop_floating();
        let word = film::words::plain(&self.word.trim().replace([' ', '-'], "_"));
        if word.is_empty() {
            return Err("a word first".to_string());
        }
        let folder = self.session.library().images();
        let count = self.count();
        let name = if count > 1 { format!("{word}.{count}.png") } else { format!("{word}.png") };
        let to = folder.join(&name);
        // (Another thing drawn of that word, not this one: taken.)
        let taken = std::fs::read_dir(&folder).into_iter().flatten().flatten().map(|file| file.path()).any(|path| {
            path.file_name().is_some_and(|file| file.to_string_lossy().split('.').next() == Some(word.as_str()))
                && Some(&path) != self.file.as_ref()
                && path != to
        });
        if taken {
            return Err(format!("\"{word}\" is already a drawn object"));
        }
        strip(&(0..count).map(|at| composite(&self.layers, at)).collect::<Vec<_>>()).save(&to).map_err(|why| why.to_string())?;
        if let Some(old) = self.file.take().filter(|old| *old != to) {
            _ = std::fs::remove_file(old);
        }
        self.file = Some(to);
        self.word = word.clone();
        write_ticks(self.session.root(), &word, &self.ticks);
        write_layers(self.session.root(), &word, &self.layers);
        self.saved = true;
        Ok(word)
    }

    /// How a film writes it: laid, and played at its times.
    pub(super) fn lines(&self) -> Vec<String> {
        let word = film::words::plain(&self.word);
        let count = self.count();
        let mut lines = vec![format!("object put {word} front_of pikachu")];
        if count > 1 {
            let ticks: Vec<String> = self.ticks.iter().map(u8::to_string).collect();
            lines.push(format!("thing {word} image {word}.{count}.png {count} pace {}", ticks.join(",")));
        }
        lines
    }
}

/// Where a picture's own things are kept (the game's files have no place for them).
fn kept_folder(root: &std::path::Path, word: &str) -> PathBuf {
    root.join("target").join("workshop").join("paintings").join(word)
}

pub(super) fn read_ticks(root: &std::path::Path, word: &str) -> Option<Vec<u8>> {
    let text = std::fs::read_to_string(kept_folder(root, word).join("pace.txt")).ok()?;
    text.trim().split(',').map(|tick| tick.trim().parse().ok()).collect()
}

fn write_ticks(root: &std::path::Path, word: &str, ticks: &[u8]) {
    let folder = kept_folder(root, word);
    _ = std::fs::create_dir_all(&folder);
    let said: Vec<String> = ticks.iter().map(u8::to_string).collect();
    _ = std::fs::write(folder.join("pace.txt"), said.join(","));
}

/// The layers kept: their names, seen, how opaque; their steps side by side.
fn write_layers(root: &std::path::Path, word: &str, layers: &[Layer]) {
    let folder = kept_folder(root, word);
    _ = std::fs::create_dir_all(&folder);
    let mut said = String::new();
    for (at, layer) in layers.iter().enumerate() {
        said.push_str(&format!("{}\t{}\t{}\t{}\n", layer.name.replace('\t', " "), u8::from(layer.visible), layer.opacity, layer.blend));
        _ = strip(&layer.cels).save(folder.join(format!("layer_{at}.png")));
    }
    _ = std::fs::write(folder.join("layers.txt"), said);
}

/// The layers kept, if they are of that size.
type KeptLayer = (String, bool, u8, u8, RgbaImage);
pub(super) fn read_layers(root: &std::path::Path, word: &str, wide: u32, high: u32) -> Option<Vec<KeptLayer>> {
    let folder = kept_folder(root, word);
    let text = std::fs::read_to_string(folder.join("layers.txt")).ok()?;
    let mut layers = Vec::new();
    for (at, line) in text.lines().enumerate() {
        let mut fields = line.split('\t');
        let name = fields.next()?.to_string();
        let visible = fields.next()? == "1";
        let opacity = fields.next()?.parse().ok()?;
        let blend = fields.next().and_then(|blend| blend.parse().ok()).unwrap_or(0);
        let strip = image::open(folder.join(format!("layer_{at}.png"))).ok()?.to_rgba8();
        if strip.width() != wide || strip.height() != high {
            return None;
        }
        layers.push((name, visible, opacity, blend, strip));
    }
    (!layers.is_empty()).then_some(layers)
}
