//! Over the text: the films opened, as tabs; the head (the file, where the
//! cursor is in the film's layout, the menus, the panels beside); the film
//! seen as a story in place of the text.

use eframe::egui::{self, Rect, RichText, Sense, vec2};
use film::service::Spot;

use super::Finding;
use super::body::Body;
use super::keys::project_search;
use super::knowledge::{face_portrait, place_said, species_of};
use super::text::{index_of, spot_of};
use workshop_base::theme;

/// What is shown of a name in the story: the narrator, the species' name
/// (after the name given, if it is not the species').
pub(super) fn name_in_story(session: &workshop_base::session::Session, text: &str, who: &str, tables: &film::tables::Tables) -> String {
    if film::words::plain(who) == "narrator" {
        return "Narrator".to_string();
    }
    let species = species_of(session, text, who);
    let plain = film::words::plain(who);
    match tables.monsters.get(&species) {
        Some((_, name)) if plain == species || plain.trim_end_matches(|letter: char| letter.is_ascii_digit()) == species => name.clone(),
        Some((_, name)) => format!("{who} ({name})"),
        None => who.to_string(),
    }
}

impl Body<'_> {
    /// The films opened, as tabs (a click: that one; ×: off the list), when there are more than one.
    pub(super) fn tabs(&mut self, ui: &mut egui::Ui) {
        let Body { scripts, open, .. } = self;
        if scripts.recent.len() <= 1 {
            return;
        }
        let tabs_rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), 22.0));
        ui.painter().rect_filled(tabs_rect, 0.0, theme::TABS);
        let mut tabs = ui.new_child(egui::UiBuilder::new().max_rect(tabs_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        tabs.set_clip_rect(tabs_rect.intersect(ui.clip_rect()));
        let mut forget = None;
        for (at, other) in scripts.recent.iter().enumerate() {
            let name = other.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
            let current = *other == open.path;
            if tabs
                .selectable_label(current, theme::text(name, theme::SMALL, if current { theme::STRONG } else { theme::WEAK }))
                .on_hover_text(other.display().to_string())
                .clicked()
                && !current
            {
                scripts.session.ask(workshop_base::ask::Ask::Film { path: other.clone(), line: 1 });
            }
            if !current
                && tabs.add(egui::Button::new(theme::text("×", theme::SMALL - 1.0, theme::FAINT)).frame(false)).on_hover_text("out of the tabs").clicked()
            {
                forget = Some(at);
            }
        }
        if let Some(at) = forget {
            scripts.recent.remove(at);
        }
        ui.advance_cursor_after_rect(tabs_rect);
    }

    /// The head, in a band of its own (what does not fit is cut, never
    /// pushing the panel wider): the file, where the cursor is, the menus.
    pub(super) fn head(&mut self, ui: &mut egui::Ui) {
        let head_rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), 26.0));
        let mut head_ui = ui.new_child(egui::UiBuilder::new().max_rect(head_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
        head_ui.set_clip_rect(head_rect.intersect(ui.clip_rect()));
        let mut jump_line: Option<usize> = None;
        {
            let ui = &mut head_ui;
            ui.add_space(4.0);
            let name = self.open.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            if ui.available_width() > 640.0 {
                ui.label(RichText::new(name).color(theme::STRONG));
            }
            if !self.open.saved() {
                ui.label(RichText::new("●").color(theme::WEAK)).on_hover_text("not written yet (in a moment)");
            }
            if ui.available_width() > 1100.0 {
                jump_line = self.where_it_is(ui);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| self.head_menus(ui));
        }
        ui.advance_cursor_after_rect(head_rect);
        if let Some(line) = jump_line {
            let at = index_of(&self.open.text, Spot { line, column: 1 });
            self.open.jump = Some((at, at));
        }
    }

    /// Where the cursor is: its scene (its place as the game names it), its
    /// moment, its line, its word; the line of the one clicked given back.
    fn where_it_is(&self, ui: &mut egui::Ui) -> Option<usize> {
        let outline = film::service::outline(&self.open.text);
        let here = spot_of(&self.open.text, self.open.cursor);
        let mut jump_line = None;
        let holding: Vec<&film::service::Symbol> = outline
            .iter()
            .filter(|symbol| matches!(symbol.kind, "scene" | "moment") && symbol.line <= here.line && here.line <= symbol.end_line.max(symbol.line))
            .collect();
        let scenes_before = outline.iter().filter(|symbol| symbol.kind == "scene" && symbol.line <= here.line).count();
        for symbol in holding {
            ui.label(theme::text("›", theme::SMALL, theme::FAINT));
            let said = if symbol.kind == "scene" {
                format!("Scene {scenes_before} · {}", place_said(&self.scripts.session, &symbol.name))
            } else {
                symbol.name.clone()
            };
            let colour = if symbol.kind == "scene" { theme::TEXT } else { theme::WEAK };
            if ui.add(egui::Button::new(theme::text(said, theme::SMALL, colour)).frame(false)).on_hover_text("go there").clicked() {
                jump_line = Some(symbol.line);
            }
        }
        ui.label(theme::text(format!("›  line {}", here.line), theme::SMALL, theme::FAINT));
        let first = self.open.text.split('\n').nth(here.line - 1).and_then(|line| line.split_whitespace().next()).map(film::words::plain).unwrap_or_default();
        if let Some((family, word, ..)) = film::every_word().find(|(_, word, ..)| *word == first) {
            ui.label(RichText::new(word).monospace().size(theme::SMALL).color(theme::FILM)).on_hover_text(format!("a word of the language ({family})"));
        }
        jump_line
    }

    /// The head's right side, few: the Wiki, "…" (the project, Format, DS Mode, Dialogue, the Layout), the View
    /// menu, the story view, Find.
    fn head_menus(&mut self, ui: &mut egui::Ui) {
        // (The Wiki at the right end: shown, however narrow the text.)
        let scripts = &mut *self.scripts;
        if ui
            .selectable_label(scripts.wiki_shown, "Wiki")
            .on_hover_text("every word of the language, what it does, how it is written, its example playing (F1: the page of the word at the cursor)")
            .clicked()
        {
            scripts.wiki_shown = !scripts.wiki_shown;
        }
        ui.menu_button("…", |ui| self.more_menu(ui)).response.on_hover_text("the project's films, Format, DS Mode, Dialogue, the Layout");
        ui.menu_button("View", |ui| self.view_menu(ui));
        let Body { scripts, path, .. } = self;
        egui::ComboBox::from_id_salt(("story view", *path))
            .selected_text(scripts.view.name())
            .width(96.0)
            .show_ui(ui, |ui| {
                for view in crate::story::View::ALL {
                    ui.selectable_value(&mut scripts.view, view, view.name());
                }
            })
            .response
            .on_hover_text("the film as text, or seen as a story: a screenplay to read, a board of scenes, its characters, a graph");
        if ui.button("Find").on_hover_text("Ctrl+F (replace: Ctrl+H)").clicked() {
            scripts.finding = Some(Finding { sought: String::new(), with: String::new(), replacing: false, case: false, at: 0, focus: true });
        }
    }

    /// The "⋯" menu: the project's (search, open, beside, versions, a note), then what is used less: Format, DS
    /// Mode, Dialogue, the Layout beside the text.
    fn more_menu(&mut self, ui: &mut egui::Ui) {
        self.project_menu(ui);
        ui.separator();
        let Body { scripts, open, .. } = self;
        if ui.button("Format  Shift+Alt+F").on_hover_text("the blocks indented again as the language writes them").clicked() {
            let made = film::formatted(&open.text);
            open.change(made);
            ui.close();
        }
        // (DS Mode: the line `for ds`, and what the DS can't play told, the completion and the Wiki by it.)
        let mut for_ds = film::ds::written_for_ds(&open.text);
        if ui
            .checkbox(&mut for_ds, "DS Mode")
            .on_hover_text("the film written for Explorers of Sky on DS (the line “for ds”): what the DS can't play is told, the completion offers only what exports, the Wiki shows each word's DS badge")
            .changed()
        {
            open.change(film::ds::with_ds(&open.text, for_ds));
        }
        ui.checkbox(&mut scripts.dialogue, "Dialogue  Ctrl+Shift+D")
            .on_hover_text("write a conversation: Enter after a line said begins the other's; Tab on a line begun: someone else of the scene");
        ui.checkbox(&mut scripts.plan_shown, "Layout").on_hover_text("the film's layout on the left: its scenes, its moments, what is wrong");
    }

    /// The Project menu: all the films searched, a film opened by its name or beside, the versions, a note.
    fn project_menu(&mut self, ui: &mut egui::Ui) {
        let Body { scripts, open, .. } = self;
        if ui.button("Search All the Films…  Ctrl+Shift+F").clicked() {
            scripts.project_search.get_or_insert_with(|| project_search(String::new()));
            ui.close();
        }
        if ui.button("Open a Film or a Recipe…  Ctrl+P").clicked() {
            scripts.quick_open = Some((String::new(), false));
            ui.close();
        }
        if ui.button("Another Film Beside…").on_hover_text("read beside this one (a double-click on one of its lines: open it there)").clicked() {
            scripts.quick_open = Some((String::new(), true));
            ui.close();
        }
        if scripts.beside.is_some() && ui.button("Nothing Beside").clicked() {
            scripts.beside = None;
            ui.close();
        }
        if ui.button("Versions…").on_hover_text("the film as it was (kept while writing it; the last commit): compared, taken back").clicked() {
            scripts.versions_shown = true;
            ui.close();
        }
        if ui.button("A Note on This Line…  Ctrl+Shift+N").on_hover_text("a note in the margin, not in the film (to do, to look at again…)").clicked() {
            let line = spot_of(&open.text, open.cursor).line;
            let said = scripts.notes.get(&line).map(|note| note.said.clone()).unwrap_or_default();
            scripts.note_editing = Some((line, said));
            ui.close();
        }
    }

    /// The View menu: lines wrapped, the hints, Focus, the typography, the letters' size.
    fn view_menu(&mut self, ui: &mut egui::Ui) {
        let Body { scripts, open, .. } = self;
        ui.checkbox(&mut scripts.wrap, "Word Wrap").on_hover_text("long lines wrap");
        ui.checkbox(&mut scripts.hints, "Hints").on_hover_text("at the end of each line: when it plays, how long, what is wrong");
        let typography = (scripts.typography, scripts.french_spaces);
        if ui.checkbox(&mut scripts.concentration, "Focus  F11").on_hover_text("the text alone over the whole window, the line written in the middle").changed()
        {
            scripts.centered = None;
        }
        ui.checkbox(&mut scripts.typography, "Typography").on_hover_text("what is said, read again as typography wants: two spaces, a word twice, colons, quotes, commas, the capital (underlined in blue, each with its fix)");
        ui.add_enabled(scripts.typography, egui::Checkbox::new(&mut scripts.french_spaces, "Spaces Before! ? : ;"))
            .on_hover_text("the French typography: « Salut ! »");
        if typography != (scripts.typography, scripts.french_spaces) {
            open.asked = u64::MAX;
        }
        ui.separator();
        if ui.button("A+  Larger Letters").clicked() {
            scripts.size = (scripts.size + 1.0).min(28.0);
        }
        if ui.button("A−  Smaller Letters").clicked() {
            scripts.size = (scripts.size - 1.0).max(9.0);
        }
    }

    /// The film seen as a story, in place of the text; a line chosen there, the text again at it.
    pub(super) fn story(&mut self, ui: &mut egui::Ui, whole: Rect) {
        let Body { scripts, open, path, seen, .. } = self;
        let room = Rect::from_min_max(ui.cursor().min, whole.max);
        let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(room).layout(egui::Layout::top_down(egui::Align::Min)));
        let text = open.text.clone();
        let session = scripts.session.clone();
        let tables = session.tables();
        let name_of = |who: &str| name_in_story(&session, &text, who, &tables);
        let place_of = |word: &str| place_said(&session, word);
        let portraits = &mut scripts.portraits;
        let mut portrait = |context: &egui::Context, species: &str, face: &str| face_portrait(&session, context, portraits, species, face);
        let mut means = crate::story::Means {
            session: &session,
            text: &text,
            path: path.as_path(),
            timing: &seen.timing,
            examples: &mut scripts.examples,
            portrait: &mut portrait,
            name_of: &name_of,
            place_of: &place_of,
            dragged: &mut scripts.dragged_scene,
            sheets: &mut scripts.sheets,
        };
        match crate::story::shown(&mut inside, scripts.view, &mut means) {
            Some(crate::story::Asked::Line(line)) => {
                scripts.view = crate::story::View::Text;
                let at = index_of(&open.text, Spot { line, column: 1 });
                open.jump = Some((at, at));
            }
            Some(crate::story::Asked::Text(made)) => open.change(made),
            Some(crate::story::Asked::Said(said)) => scripts.said = Some(said),
            None => {}
        }
        ui.allocate_rect(room, Sense::hover());
    }
}
