//! An asset of the Film's Assets panel (`workshop_base::asset::Asset`)
//! written in the Script: dropped on a line, or double-clicked (the cursor's
//! line). Each is one change of the text (one undo), the rest as it was:
//! - a Pokémon: its word, where it is dropped (or at the cursor);
//! - a face: `(face)` on a spoken line (in place of the face it had, its
//!   other words kept), else a new line `who (face): ` to say;
//! - a gesture: `who sit`; a song: `music word`; a sound: `sound word`;
//! - a thing: `who hold thing`, or `put object thing [x y]`;
//! - a place: a new scene there, with the cast of the one before;
//! - a move: `who use Move on other`.

use eframe::egui::{self, FontId, Rect, Stroke};
use workshop_base::asset::Asset;
use workshop_base::theme;

use super::Script;
use super::body::{Body, Written};
use super::edits::replaced;
use super::text::{byte_of, index_of, line_at};
use film::service::Spot;

impl Body<'_> {
    /// An asset dragged over the text: where it would go shown (a caret for
    /// a Pokémon's word, a line under the line for the others); let go, written there.
    pub(super) fn asset_dropped(&mut self, ui: &egui::Ui, written: &Written, editor: Rect) {
        let Some(asset) = egui::DragAndDrop::payload::<Asset>(ui.ctx()) else {
            return;
        };
        let Some(at) = ui.input(|input| input.pointer.latest_pos()).filter(|at| editor.contains(*at)) else {
            return;
        };
        let letter = written.index_at(at);
        let line = self.open.line_of_letter(letter);
        let painter = ui.painter_at(editor);
        let stroke = Stroke::new(2.0_f32, theme::FILM);
        let word = matches!(&*asset, Asset::Pokemon { .. });
        if word {
            let caret = written.at(letter);
            painter.vline(caret.left(), caret.y_range(), stroke);
        } else {
            let end = index_of(&self.open.text, Spot { line, column: usize::MAX });
            let row = written.at(end);
            painter.hline(editor.x_range(), row.bottom() + 1.0, stroke);
        }
        let said = if word { format!("{}: here", asset.said()) } else { format!("{}: under line {line}", asset.said()) };
        painter.text(at + egui::vec2(14.0, -16.0), egui::Align2::LEFT_BOTTOM, said, FontId::proportional(theme::SMALL), theme::FILM);
        ui.ctx().request_repaint();
        if !ui.input(|input| input.pointer.any_released()) {
            return;
        }
        let Some(asset) = egui::DragAndDrop::take_payload::<Asset>(ui.ctx()) else {
            return;
        };
        let told = match &*asset {
            Asset::Pokemon { word } => {
                self.open.insert_at(letter, word);
                format!("written: {word}")
            }
            asset => self.open.put_asset(asset, Some(line), self.seen.chosen.as_deref(), None),
        };
        self.scripts.said = Some(told);
    }
}

/// The words that open a header line (`title: …`), never a speaker.
const HEADERS: [&str; 5] = ["title", "scene", "moment", "let", "recipe"];

/// Whether a line opens a scene or a moment.
fn opens(line: &str) -> bool {
    let line = line.trim_start();
    film::lines::scene_head(line).is_some() || line.starts_with("moment:") || line.starts_with("title:")
}

/// The words of a `scene:` line after its place: its cast (a remark after `#` left out).
pub fn cast_of(scene: &str) -> Vec<String> {
    let said = scene.split('#').next().unwrap_or_default();
    film::lines::scene_head(said).map_or_else(Vec::new, |rest| rest.split_whitespace().skip(1).map(str::to_string).collect())
}

/// The `scene:` line of that line (from 1), if any: its number.
pub fn scene_of(lines: &[&str], line: usize) -> Option<usize> {
    (1..=line.min(lines.len())).rev().find(|number| film::lines::scene_head(&lines[number - 1]).is_some())
}

/// Who a line is of: the speaker of a spoken line, or the first word of a line of someone.
fn first_of(line: &str) -> Option<&str> {
    let word = line.split_whitespace().next()?;
    let word = word.split([':', '(']).next().unwrap_or_default();
    (!word.is_empty()).then_some(word)
}

/// Who an asset is written for at that line: who is chosen, else the one the
/// line is of (one of the scene's cast), else the first of the cast.
fn person(lines: &[&str], line: usize, who: Option<&str>) -> String {
    if let Some(who) = who {
        return who.to_string();
    }
    let cast = scene_of(lines, line).map(|scene| cast_of(lines[scene - 1])).unwrap_or_default();
    let own = lines.get(line.wrapping_sub(1)).and_then(|said| first_of(said)).filter(|word| cast.iter().any(|one| one == word));
    own.map(str::to_string).or_else(|| cast.first().cloned()).unwrap_or_else(|| "pikachu".to_string())
}

/// How far in a line written after that one goes: as the last line written
/// up to it (a blank one passed over), or the next if that one opens a scene.
fn lead_after(lines: &[&str], line: usize) -> String {
    let lead = |said: &str| said.chars().take_while(|letter| letter.is_whitespace()).collect::<String>();
    let written = (1..=line.min(lines.len())).rev().find(|number| !lines[number - 1].trim().is_empty());
    match written.map(|number| lines[number - 1]) {
        Some(said) if opens(said) => lines.iter().skip(line).find(|next| !next.trim().is_empty() && !opens(next)).map(|next| lead(next)).unwrap_or_default(),
        Some(said) => lead(said),
        None => String::new(),
    }
}

/// The text with those lines written after that line (from 1; 0: at the top);
/// a blank line there is written over. The text, the number of the last line written.
pub fn lines_after(text: &str, line: usize, made: &[String]) -> (String, usize) {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let line = line.min(lines.len());
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    let lead = lead_after(&borrowed, line);
    // (A scene's line at the margin, as scenes are.)
    let written: Vec<String> = made.iter().map(|said| if said.is_empty() || opens(said) { said.clone() } else { format!("{lead}{said}") }).collect();
    let count = written.len();
    if line > 0 && lines[line - 1].trim().is_empty() && made.first().is_some_and(|first| !first.is_empty()) {
        lines.splice(line - 1..line, written);
        return (lines.join("\n"), line - 1 + count);
    }
    lines.splice(line..line, written);
    (lines.join("\n"), line + count)
}

/// A spoken line with that face (`pikachu: Hi` → `pikachu (happy): Hi`): in
/// place of a face it said (one of `faces`), its other words kept
/// (`(bubble, sad)`); none if it is not a spoken line.
pub fn with_face(line: &str, face: &str, faces: &[&str]) -> Option<String> {
    let colon = line.find(':')?;
    let (head, rest) = line.split_at(colon);
    let lead: String = head.chars().take_while(|letter| letter.is_whitespace()).collect();
    let head = head.trim();
    let (name, said) = match head.split_once('(') {
        Some((name, said)) => (name.trim(), Some(said.trim_end().strip_suffix(')')?)),
        None => (head, None),
    };
    if name.is_empty() || name.contains(char::is_whitespace) || HEADERS.contains(&name) {
        return None;
    }
    let mut parts: Vec<String> =
        said.map_or_else(Vec::new, |said| said.split(',').map(|part| part.trim().to_string()).filter(|part| !part.is_empty()).collect());
    match parts.iter().position(|part| faces.contains(&part.as_str())) {
        Some(at) => parts[at] = face.to_string(),
        None => parts.push(face.to_string()),
    }
    Some(format!("{lead}{name} ({}){rest}", parts.join(", ")))
}

/// The film's text with that asset written at that line (from 1), for who
/// is chosen (on the Scene), at that point of the place (dropped on the
/// Scene). The text, the line written (the cursor goes to its end).
pub fn asset_written(text: &str, line: usize, asset: &Asset, who: Option<&str>, point: Option<[f32; 2]>, faces: &[&str]) -> (String, usize) {
    let lines: Vec<&str> = text.split('\n').collect();
    let line = line.clamp(1, lines.len().max(1));
    let one = |said: String| lines_after(text, line, &[said]);
    let at = point.map(|[x, y]| format!(" {} {}", x.round(), y.round())).unwrap_or_default();
    match asset {
        Asset::Pokemon { word } => one(word.clone()),
        Asset::Face { who: species, face } => match lines.get(line - 1).and_then(|said| with_face(said, face, faces)) {
            Some(made) => {
                let mut all: Vec<String> = lines.iter().map(|said| said.to_string()).collect();
                all[line - 1] = made;
                (all.join("\n"), line)
            }
            None => one(format!("{species} ({face}): ")),
        },
        Asset::Gesture { who: species, words } => one(format!("{species} {words}")),
        Asset::Music { word } => one(format!("music {word}")),
        Asset::Sound { word } => one(format!("sound {word}")),
        Asset::Thing { word, held: true } if point.is_none() || who.is_some() => one(format!("{} hold {word}", person(&lines, line, who))),
        Asset::Thing { word, .. } => one(format!("put object {word}{at}")),
        Asset::Place { word } => {
            let cast = scene_of(&lines, line).map(|scene| cast_of(lines[scene - 1])).unwrap_or_default();
            let scene = std::iter::once(format!("scene: {word}")).chain(cast).collect::<Vec<_>>().join(" ");
            lines_after(text, line, &[String::new(), scene])
        }
        Asset::Effect { name } => {
            let who = person(&lines, line, who);
            let cast = scene_of(&lines, line).map(|scene| cast_of(lines[scene - 1])).unwrap_or_default();
            match cast.iter().find(|one| **one != who) {
                Some(target) => one(format!("{who} use {name} on {target}")),
                None => one(format!("{who} use {name}")),
            }
        }
    }
}

impl Script {
    /// An asset written at that line (none: the cursor's), for who is
    /// chosen, at that point of the place: one change (one undo), the
    /// cursor at the end of what was written. What was written, said.
    pub fn put_asset(&mut self, asset: &Asset, line: Option<usize>, who: Option<&str>, point: Option<[f32; 2]>) -> String {
        // (A Pokémon by the cursor: its word there.)
        if let (Asset::Pokemon { word }, None) = (asset, line) {
            self.insert(word);
            return format!("written: {word}");
        }
        let line = line.unwrap_or_else(|| self.cursor_line());
        let faces: Vec<&str> = scenes::vocabulary::POKEMON.faces.iter().map(|(word, _)| *word).collect();
        let (made, written) = asset_written(&self.text, line, asset, who, point, &faces);
        let said = made.split('\n').nth(written - 1).unwrap_or_default().trim().to_string();
        self.change(made);
        let end = self.text.split('\n').nth(written - 1).map_or(0, |said| said.chars().count());
        let at = index_of(&self.text, Spot { line: written, column: end + 1 });
        self.cursor = at;
        self.jump = Some((at, at));
        format!("line {written}: {said}")
    }

    /// Words written at that letter (a space put between them and a word
    /// before or after): one change, the cursor after them.
    pub fn insert_at(&mut self, at: usize, words: &str) {
        let at = at.min(self.text.chars().count());
        let byte = byte_of(&self.text, at);
        let before = self.text[..byte].chars().next_back();
        let after = self.text[byte..].chars().next();
        let mut with = words.to_string();
        if before.is_some_and(|letter| !letter.is_whitespace() && letter != '(') {
            with.insert(0, ' ');
        }
        if after.is_some_and(|letter| !letter.is_whitespace() && !matches!(letter, ':' | ')' | ',')) {
            with.push(' ');
        }
        let (made, end) = replaced(&self.text, at..at, &with);
        self.change(made);
        self.cursor = end;
        self.jump = Some((end, end));
    }

    /// The line (from 1) a letter's index is on.
    pub fn line_of_letter(&self, at: usize) -> usize {
        line_at(&self.text, at).0 + 1
    }
}
