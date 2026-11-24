//! The keys that act on the text, taken before the editor sees them: the
//! words by family browsed, more cursors than one, the problems and the
//! marks gone through, the project's windows, the lines changed by a key.

use eframe::egui::{self, Key, Modifiers};
use film::service::Spot;

use super::body::Body;
use super::cast::{other_voice, voices_near};
use super::completion::offered;
use super::edits::{
    Typed, chosen_letters, comments_toggled, line_cut, line_doubled, lines_chosen, lines_gone, lines_moved, lines_of, new_line, replaced, tabbed,
    typed_at_each, untabbed,
};
use super::highlight::speaker;
use super::knowledge::{Defined, defined};
use super::text::{index_of, letter_of_word, line_at, spot_of, word_at};
use super::wiki::wiki_word_at;
use super::{Browse, Finding, ProjectSearch};

/// What the keys ask of what comes after them: a word offered taken (its
/// rank), a word of a family taken, the bulb of the cursor's line opened.
#[derive(Default)]
pub(super) struct Keyed {
    pub(super) accept: Option<usize>,
    pub(super) browse_take: Option<String>,
    pub(super) bulb_asked: bool,
}

/// A text changed by a key, and the selection then.
type Made = Option<(String, (usize, usize))>;

/// That key, with those modifiers, pressed: taken from the input (the editor will not see it).
fn pressed(ui: &egui::Ui, modifiers: Modifiers, key: Key) -> bool {
    ui.input_mut(|input| input.consume_key(modifiers, key))
}

/// A search of the whole project, for those words.
pub(super) fn project_search(sought: String) -> ProjectSearch {
    ProjectSearch { sought, with: String::new(), whole: true, case: false, hits: Vec::new(), searched: (String::new(), false, false), asked: false }
}

/// The next spot of a list, after that line (or the one before it), going round.
pub(super) fn next_spot(spots: &[Spot], line: usize, forward: bool) -> Option<Spot> {
    if forward { spots.iter().find(|spot| spot.line > line).or(spots.first()) } else { spots.iter().rev().find(|spot| spot.line < line).or(spots.last()) }
        .copied()
}

/// The problems' first spots, one a line, in the text's order.
pub(super) fn problem_spots(told: &[film::service::Diagnostic]) -> Vec<Spot> {
    let mut spots: Vec<Spot> = told.iter().map(|told| told.span.start).collect();
    spots.sort_by_key(|spot| (spot.line, spot.column));
    spots.dedup_by_key(|spot| spot.line);
    spots
}

/// Where one more cursor goes, on the line above the highest (or under the
/// lowest), as far along as it can; nothing at the edge of the text.
pub(super) fn cursor_beside(text: &str, cursors: &[usize], up: bool) -> Option<usize> {
    let from = spot_of(text, if up { cursors.iter().copied().min()? } else { cursors.iter().copied().max()? });
    let line = if up { from.line.checked_sub(1).filter(|line| *line >= 1) } else { Some(from.line + 1).filter(|line| *line <= text.split('\n').count()) }?;
    let length = text.split('\n').nth(line - 1).map_or(0, |words| words.chars().count());
    Some(index_of(text, Spot { line, column: from.column.min(length + 1) }))
}

impl Body<'_> {
    /// The keys of the text, the editor having the keys, in the order they are taken.
    pub(super) fn keys(&mut self, ui: &egui::Ui, selection: (usize, usize)) -> Keyed {
        let cursor = selection.0;
        let browse_take = self.browse_keys(ui);
        self.more_cursors(ui, cursor);
        let bulb_asked = self.going_keys(ui, cursor);
        self.project_keys(ui, selection);
        self.mark_keys(ui, cursor);
        let accept = self.offer_keys(ui);
        let text = self.open.text.clone();
        let lines = lines_of(&text, selection);
        self.find_keys(ui, &text, selection);
        self.fold_keys(ui, &text, cursor);
        self.file_keys(ui, &text, cursor);
        let mut made = self.line_keys(ui, &text, lines, selection);
        made = self.dialogue_keys(ui, &text, selection).or(made);
        made = self.typing_keys(ui, &text, lines, selection).or(made);
        made = self.whole_line_copied(ui, &text, lines.0, selection).or(made);
        made = self.name_keys(ui, &text, cursor).or(made);
        if let Some((changed, selection)) = made {
            self.open.change(changed);
            self.open.jump = Some(selection);
            self.scripts.completion = None;
        }
        Keyed { accept, browse_take, bulb_asked }
    }

    /// The words by family browsed: ← → a family, ↑ ↓ a word, Enter or Tab writes it (the word given back), Esc.
    fn browse_keys(&mut self, ui: &egui::Ui) -> Option<String> {
        let scripts = &mut *self.scripts;
        let browse = scripts.browsing.as_mut()?;
        let mut take = None;
        let words = workshop_wiki::catalogue::of_family(&scripts.session, browse.family);
        let families = workshop_wiki::catalogue::FAMILIES.len();
        if pressed(ui, Modifiers::NONE, Key::ArrowDown) {
            (browse.chosen, browse.moved) = ((browse.chosen + 1).min(words.len().saturating_sub(1)), true);
        }
        if pressed(ui, Modifiers::NONE, Key::ArrowUp) {
            (browse.chosen, browse.moved) = (browse.chosen.saturating_sub(1), true);
        }
        if pressed(ui, Modifiers::NONE, Key::ArrowRight) {
            (browse.family, browse.chosen, browse.moved) = ((browse.family + 1) % families, 0, true);
        }
        if pressed(ui, Modifiers::NONE, Key::ArrowLeft) {
            (browse.family, browse.chosen, browse.moved) = ((browse.family + families - 1) % families, 0, true);
        }
        if pressed(ui, Modifiers::NONE, Key::Enter) || pressed(ui, Modifiers::NONE, Key::Tab) {
            take = words.get(browse.chosen).map(|entry| entry.word.clone());
        }
        if pressed(ui, Modifiers::NONE, Key::Escape) {
            scripts.browsing = None;
        }
        take
    }

    /// More cursors than one: Ctrl+Alt+↑ ↓ one more on the line above, below;
    /// what is typed written at each; Esc, or the cursor moved: one again.
    fn more_cursors(&mut self, ui: &egui::Ui, cursor: usize) {
        let Body { scripts, open, .. } = self;
        for (key, up) in [(Key::ArrowUp, true), (Key::ArrowDown, false)] {
            if pressed(ui, Modifiers::COMMAND | Modifiers::ALT, key) {
                let cursors: Vec<usize> = scripts.extra.iter().copied().chain([cursor]).collect();
                scripts.extra.extend(cursor_beside(&open.text, &cursors, up));
            }
        }
        if !scripts.extra.is_empty() {
            if pressed(ui, Modifiers::NONE, Key::Escape) {
                scripts.extra.clear();
            }
            let moved = ui.input(|input| {
                input.events.iter().any(|event| matches!(event, egui::Event::Key { key: Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown | Key::Home | Key::End | Key::PageUp | Key::PageDown, pressed: true, modifiers, .. } if !modifiers.command))
            });
            if moved {
                scripts.extra.clear();
            }
        }
        if scripts.extra.is_empty() {
            return;
        }
        let typed = typed_taken(ui);
        if !typed.is_empty() {
            let mut text = open.text.clone();
            let mut cursors: Vec<usize> = scripts.extra.iter().copied().chain([cursor]).collect();
            for one in typed {
                (text, cursors) = typed_at_each(&text, &cursors, &one);
            }
            let main = cursors.pop().unwrap_or(0);
            scripts.extra = cursors;
            open.change(text);
            open.jump = Some((main, main));
            scripts.completion = None;
        }
    }

    /// Ctrl+Enter: the film played from this line; Ctrl+. : the fixes of this
    /// line asked (given back); F8, Shift+F8: the next problem, the one before.
    fn going_keys(&mut self, ui: &egui::Ui, cursor: usize) -> bool {
        let Body { scripts, open, .. } = self;
        if pressed(ui, Modifiers::COMMAND, Key::Enter) {
            scripts.play_from = Some(line_at(&open.text, cursor).0 + 1);
        }
        let bulb_asked = pressed(ui, Modifiers::COMMAND, Key::Period);
        let next_problem = pressed(ui, Modifiers::NONE, Key::F8);
        let problem_before = pressed(ui, Modifiers::SHIFT, Key::F8);
        if next_problem || problem_before {
            let line = line_at(&open.text, cursor).0 + 1;
            if let Some(spot) = next_spot(&problem_spots(&open.told), line, next_problem) {
                let at = index_of(&open.text, spot);
                open.jump = Some((at, at));
            }
        }
        bulb_asked
    }

    /// The project: Ctrl+Shift+F what is chosen (or the word) sought in all
    /// the films; Ctrl+P a film opened by its name; Ctrl+Shift+N a note on this line.
    fn project_keys(&mut self, ui: &egui::Ui, selection: (usize, usize)) {
        let Body { scripts, open, .. } = self;
        let cursor = selection.0;
        if pressed(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::F) {
            let sought = if selection.0 != selection.1 { chosen_letters(&open.text, selection) } else { word_at(&open.text, cursor).unwrap_or_default() };
            scripts.project_search = Some(project_search(sought));
        }
        if pressed(ui, Modifiers::COMMAND, Key::P) {
            scripts.quick_open = Some((String::new(), false));
        }
        if pressed(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::N) {
            let line = line_at(&open.text, cursor).0 + 1;
            let said = scripts.notes.get(&line).map(|note| note.said.clone()).unwrap_or_default();
            scripts.note_editing = Some((line, said));
        }
    }

    /// F9: the film stops at this line as it plays, or not; F10: on to the
    /// next line; Ctrl+B: this line marked, or not; F4, Shift+F4: the next one marked, the one before.
    fn mark_keys(&mut self, ui: &egui::Ui, cursor: usize) {
        let Body { scripts, open, .. } = self;
        let line = line_at(&open.text, cursor).0 + 1;
        if pressed(ui, Modifiers::NONE, Key::F9) && !scripts.breakpoints.remove(&line) {
            scripts.breakpoints.insert(line);
        }
        if pressed(ui, Modifiers::NONE, Key::F10) {
            scripts.step_asked = true;
        }
        if pressed(ui, Modifiers::COMMAND, Key::B) && !scripts.bookmarks.remove(&line) {
            scripts.bookmarks.insert(line);
        }
        let next_mark = pressed(ui, Modifiers::NONE, Key::F4);
        let mark_before = pressed(ui, Modifiers::SHIFT, Key::F4);
        if next_mark || mark_before {
            let marks: Vec<Spot> = scripts.bookmarks.iter().map(|line| Spot { line: *line, column: 1 }).collect();
            if let Some(next) = next_spot(&marks, line, next_mark) {
                let at = index_of(&open.text, next);
                open.jump = Some((at, at));
            }
        }
    }

    /// The words offered: ↑ ↓ another, Enter or Tab takes it (its rank given back), Esc: none.
    fn offer_keys(&mut self, ui: &egui::Ui) -> Option<usize> {
        let completion = self.scripts.completion.as_mut()?;
        let mut accept = None;
        let count = completion.choices.len();
        if pressed(ui, Modifiers::NONE, Key::ArrowDown) {
            completion.chosen = (completion.chosen + 1) % count.max(1);
        }
        if pressed(ui, Modifiers::NONE, Key::ArrowUp) {
            completion.chosen = (completion.chosen + count.max(1) - 1) % count.max(1);
        }
        if pressed(ui, Modifiers::NONE, Key::Enter) || pressed(ui, Modifiers::NONE, Key::Tab) {
            accept = Some(completion.chosen);
        }
        if pressed(ui, Modifiers::NONE, Key::Escape) {
            self.scripts.completion = None;
        }
        accept
    }

    /// Ctrl+F finds, Ctrl+H replaces (what is chosen, sought); Ctrl+G a line;
    /// Ctrl+= Ctrl+- Ctrl+0 the letters larger, smaller, as they were.
    fn find_keys(&mut self, ui: &egui::Ui, text: &str, selection: (usize, usize)) {
        let scripts = &mut *self.scripts;
        let find = pressed(ui, Modifiers::COMMAND, Key::F);
        let replace = pressed(ui, Modifiers::COMMAND, Key::H);
        if find || replace {
            let chosen = chosen_letters(text, selection);
            let sought = if chosen.is_empty() || chosen.contains('\n') {
                scripts.finding.as_ref().map(|finding| finding.sought.clone()).unwrap_or_default()
            } else {
                chosen
            };
            let (with, case) = scripts.finding.as_ref().map_or((String::new(), false), |finding| (finding.with.clone(), finding.case));
            scripts.finding = Some(Finding { sought, with, replacing: replace, case, at: 0, focus: true });
        }
        if pressed(ui, Modifiers::COMMAND, Key::G) {
            scripts.going_to = Some(String::new());
        }
        if pressed(ui, Modifiers::COMMAND, Key::Plus) || pressed(ui, Modifiers::COMMAND, Key::Equals) {
            scripts.size = (scripts.size + 1.0).min(28.0);
        }
        if pressed(ui, Modifiers::COMMAND, Key::Minus) {
            scripts.size = (scripts.size - 1.0).max(9.0);
        }
        if pressed(ui, Modifiers::COMMAND, Key::Num0) {
            scripts.size = 13.0;
        }
    }

    /// Ctrl+Shift+[ folds what holds the cursor's line, Ctrl+Shift+] unfolds it.
    fn fold_keys(&mut self, ui: &egui::Ui, text: &str, cursor: usize) {
        let fold_now = pressed(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::OpenBracket);
        let unfold_now = pressed(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::CloseBracket);
        if !fold_now && !unfold_now {
            return;
        }
        let line = line_at(text, cursor).0 + 1;
        let holding = film::service::folds(text).into_iter().filter(|(first, last)| *first <= line && line <= *last).max_by_key(|(first, _)| *first);
        if let Some((first, _)) = holding {
            if fold_now {
                self.scripts.folded.insert(first);
                let at = index_of(text, Spot { line: first, column: 1 });
                self.open.jump = Some((at, at));
            } else {
                self.scripts.folded.remove(&first);
            }
        }
    }

    /// Ctrl+S writes the film; Ctrl+Space offers what may be written (on an empty line: the words by family).
    fn file_keys(&mut self, ui: &egui::Ui, text: &str, cursor: usize) {
        let Body { scripts, open, .. } = self;
        if pressed(ui, Modifiers::COMMAND, Key::S) {
            scripts.said = Some(match open.save() {
                Ok(()) => "written".to_string(),
                Err(why) => why,
            });
        }
        if pressed(ui, Modifiers::COMMAND, Key::Space) {
            if line_at(text, cursor).2.trim().is_empty() {
                scripts.browsing = Some(Browse { family: scripts.family, chosen: 0, moved: true, fresh: true });
                scripts.completion = None;
            } else {
                scripts.completion = offered(&scripts.session, text, cursor, true);
            }
        }
    }

    /// The lines by a key: Ctrl+Shift+K gone, Ctrl+L chosen, Ctrl+/ commented,
    /// Ctrl+D doubled, Alt+↑ ↓ moved; Ctrl+Shift+D the dialogue's writing, or not.
    fn line_keys(&mut self, ui: &egui::Ui, text: &str, lines: (usize, usize), selection: (usize, usize)) -> Made {
        let mut made = None;
        if pressed(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::K) {
            let (changed, at) = lines_gone(text, lines);
            made = Some((changed, (at, at)));
        }
        if pressed(ui, Modifiers::COMMAND, Key::L) {
            self.open.jump = Some(lines_chosen(text, lines));
        }
        if pressed(ui, Modifiers::COMMAND, Key::Slash) {
            made = Some((comments_toggled(text, lines), selection));
        }
        if pressed(ui, Modifiers::COMMAND, Key::D) {
            let (changed, moved) = line_doubled(text, selection.0);
            made = Some((changed, (moved, moved)));
        }
        for (key, up) in [(Key::ArrowUp, true), (Key::ArrowDown, false)] {
            if pressed(ui, Modifiers::ALT, key)
                && let Some(moved) = lines_moved(text, lines, up, selection)
            {
                made = Some(moved);
            }
        }
        if pressed(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::D) {
            self.scripts.dialogue = !self.scripts.dialogue;
        }
        made
    }

    /// Writing a dialogue: Enter after a line said begins the other's; Tab on a line begun, someone else of the scene.
    fn dialogue_keys(&mut self, ui: &egui::Ui, text: &str, selection: (usize, usize)) -> Made {
        if !self.scripts.dialogue || self.scripts.completion.is_some() || selection.0 != selection.1 {
            return None;
        }
        let cursor = selection.0;
        let (number, start, line) = line_at(text, cursor);
        let at_end = cursor - start >= line.trim_end().chars().count();
        let (who, said_at) = speaker(&line).filter(|_| at_end)?;
        let said = line[said_at..].trim().to_string();
        let who = who.to_string();
        let lead: String = line.chars().take_while(|letter| *letter == ' ').collect();
        let voices = voices_near(text, number);
        if said.is_empty() && pressed(ui, Modifiers::NONE, Key::Tab) {
            let next =
                voices.iter().position(|voice| *voice == who).map_or_else(|| voices.first().cloned(), |at| voices.get((at + 1) % voices.len()).cloned())?;
            let made_line = format!("{lead}{next}: ");
            let (changed, _) = replaced(text, start..start + line.chars().count(), &made_line);
            let at = start + made_line.chars().count();
            Some((changed, (at, at)))
        } else if !said.is_empty() && pressed(ui, Modifiers::NONE, Key::Enter) {
            let other = other_voice(text, number, &who, &voices);
            let (changed, at) = replaced(text, cursor..cursor, &format!("\n{lead}{other}: "));
            Some((changed, (at, at)))
        } else {
            None
        }
    }

    /// Tab and Shift+Tab indent and take the indent back; Enter begins a line as far in.
    fn typing_keys(&mut self, ui: &egui::Ui, text: &str, lines: (usize, usize), selection: (usize, usize)) -> Made {
        let mut made = None;
        if self.scripts.completion.is_none() && pressed(ui, Modifiers::NONE, Key::Tab) {
            made = Some(tabbed(text, lines, selection));
        }
        if pressed(ui, Modifiers::SHIFT, Key::Tab) {
            made = Some(untabbed(text, lines, selection));
        }
        if self.scripts.completion.is_none() && pressed(ui, Modifiers::NONE, Key::Enter) {
            let (changed, at) = new_line(text, selection);
            made = Some((changed, (at, at)));
        }
        made
    }

    /// Ctrl+C, Ctrl+X with nothing chosen: the whole line copied (cut).
    fn whole_line_copied(&mut self, ui: &egui::Ui, text: &str, first: usize, selection: (usize, usize)) -> Made {
        if selection.0 != selection.1 {
            return None;
        }
        let (copy, cut) = ui.input(|input| {
            (input.events.iter().any(|event| matches!(event, egui::Event::Copy)), input.events.iter().any(|event| matches!(event, egui::Event::Cut)))
        });
        if !copy && !cut {
            return None;
        }
        let (_, _, line) = line_at(text, selection.0);
        ui.ctx().copy_text(format!("{line}\n"));
        if !cut {
            return None;
        }
        ui.input_mut(|input| input.events.retain(|event| !matches!(event, egui::Event::Cut)));
        let (changed, at) = line_cut(text, first);
        Some((changed, (at, at)))
    }

    /// Shift+Alt+F the blocks put in step; F1 the word's page in the Wiki; F12 where the name is given; F2 the name
    /// renamed everywhere.
    fn name_keys(&mut self, ui: &egui::Ui, text: &str, cursor: usize) -> Made {
        let Body { scripts, open, .. } = self;
        let mut made = None;
        if pressed(ui, Modifiers::SHIFT | Modifiers::ALT, Key::F) || pressed(ui, Modifiers::COMMAND | Modifiers::SHIFT, Key::F) {
            made = Some((film::formatted(text), (cursor, cursor)));
        }
        // (F1: the Wiki's page of the word pointed at, its tooltip shown; else of the cursor's.)
        if pressed(ui, Modifiers::NONE, Key::F1) {
            let pointed = scripts.hover.as_ref().filter(|(_, _, said)| said.is_some()).map(|(index, ..)| *index);
            if let Some(word) = pointed.and_then(|at| wiki_word_at(&scripts.session, text, at)).or_else(|| wiki_word_at(&scripts.session, text, cursor)) {
                scripts.wiki_open(&word);
            }
        }
        if pressed(ui, Modifiers::NONE, Key::F12) {
            match defined(&scripts.session, text, cursor) {
                Defined::Here(line) => {
                    let at = index_of(text, Spot { line, column: 1 });
                    open.jump = Some((at, at));
                }
                Defined::File(path, line) => scripts.open_asked = Some((path, line)),
                Defined::Said(said) => scripts.said = Some(said),
            }
        }
        if pressed(ui, Modifiers::NONE, Key::F2) {
            let (_, start, line) = line_at(text, cursor);
            let letters: Vec<char> = line.chars().collect();
            let mut from = (cursor - start).min(letters.len());
            while from > 0 && letter_of_word(letters[from - 1]) {
                from -= 1;
            }
            let word: String = letters[from..].iter().take_while(|letter| letter_of_word(**letter)).collect();
            if !word.is_empty() {
                scripts.renaming = Some((spot_of(text, start + from), word));
            }
        }
        made
    }
}

/// What is typed (letters, what is pasted, a new line, a letter taken back), taken from the input.
fn typed_taken(ui: &egui::Ui) -> Vec<Typed> {
    let mut typed: Vec<Typed> = Vec::new();
    ui.input_mut(|input| {
        input.events.retain(|event| match event {
            egui::Event::Text(said) | egui::Event::Paste(said) => {
                typed.push(Typed::Words(said.clone()));
                false
            }
            egui::Event::Key { key: Key::Backspace, pressed: true, .. } => {
                typed.push(Typed::Back);
                false
            }
            egui::Event::Key { key: Key::Enter, pressed: true, modifiers, .. } if !modifiers.command => {
                typed.push(Typed::Words("\n".to_string()));
                false
            }
            egui::Event::Key { key: Key::Backspace | Key::Enter, pressed: false, .. } => false,
            _ => true,
        })
    });
    typed
}
