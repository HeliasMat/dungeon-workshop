//! Over a shelf's grid: the tools (create, import, grid or list, sorted,
//! used, how large, sought), the head (where one is, how many, what was
//! said), the shelf's filters.

use eframe::egui::{self, FontId, Rect, RichText, vec2};

use super::dropped::take_files;
use super::shelves::filters_of;
use super::{Asked, Resources, Shelf};
use workshop_base::theme;

/// What the sort button says, by the sort.
pub(super) fn sort_said(sort: u8) -> &'static str {
    ["Sort: Number", "Sort: Name", "Sort: Usage"][sort as usize % 3]
}

/// The tools' bar.
pub(super) fn tools(ui: &mut egui::Ui, resources: &mut Resources) {
    let tools = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), 34.0));
    ui.painter().rect_filled(tools, 0.0, theme::TOOLS);
    let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(tools.shrink2(vec2(6.0, 5.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    bar.set_clip_rect(tools.intersect(ui.clip_rect()));
    bar.menu_button(RichText::new("+ Create").strong().color(theme::ASSETS), |ui| create_menu(ui, resources));
    made_and_imported(&mut bar, resources);
    bar.separator();
    if bar.selectable_label(!resources.listed, "Grid").clicked() {
        resources.listed = false;
    }
    if bar.selectable_label(resources.listed, "List").clicked() {
        resources.listed = true;
    }
    bar.separator();
    if bar.button(sort_said(resources.sort)).on_hover_text("by number (the Pokémon in Pokédex order), by name, or the most used in the films first").clicked()
    {
        resources.sort = (resources.sort + 1) % 3;
        resources.scroll_wanted = true;
    }
    if bar.selectable_label(resources.only_used, "Used").on_hover_text("only those a film uses").clicked() {
        resources.only_used = !resources.only_used;
        resources.scroll_wanted = true;
    }
    bar.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let field = ui.add(egui::TextEdit::singleline(&mut resources.sought).hint_text("Search everywhere…").desired_width(200.0));
        if ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::F)) {
            field.request_focus();
        }
        if !resources.sought.is_empty() && ui.small_button("✕").clicked() {
            resources.sought.clear();
        }
        ui.spacing_mut().slider_width = 90.0;
        ui.add(egui::Slider::new(&mut resources.size, 48.0..=176.0).show_value(false));
        ui.label(theme::text("Size", theme::SMALL, theme::WEAK));
    });
    ui.advance_cursor_after_rect(tools);
}

/// What may be created: an image, an animation, a sound, a song, a dungeon, a boss fight.
fn create_menu(ui: &mut egui::Ui, resources: &mut Resources) {
    let makers = [
        ("An Image…", "painted here, pixel by pixel", "image"),
        ("An Animation…", "images one after another, each for its own time", "animation"),
        ("A Sound…", "a wave, from one pitch to another, like old games", "sound"),
        ("A Song…", "tracks, instruments, drums, a mix: a whole piece", "music"),
    ];
    for (said, hover, kind) in makers {
        if ui.button(said).on_hover_text(hover).clicked() {
            resources.open_making(crate::resources::Maker::new_of(&resources.session, kind));
            ui.close();
        }
    }
    if ui.button("A Dungeon…").on_hover_text("in the Dungeon space: turn-based floors, its Pokémon, its items; in a film: `dungeon <word>`").clicked() {
        resources.asked = Some(Asked::Dungeon(workshop_dungeon::dungeons::Open::NewDungeon));
        ui.close();
    }
    if ui.button("A Boss Fight…").on_hover_text("in the Dungeon space: an arena, a boss, its guards, its phases; a dungeon names it: `fight: <word>`").clicked()
    {
        resources.asked = Some(Asked::Dungeon(workshop_dungeon::dungeons::Open::NewFight));
        ui.close();
    }
    ui.separator();
    ui.label(theme::text("Or drop a PNG on the window: it joins the drawn objects", theme::SMALL, theme::FAINT));
}

/// What is being made, opened; images imported; SpriteCollab.
fn made_and_imported(bar: &mut egui::Ui, resources: &mut Resources) {
    if !resources.makings.is_empty() {
        let said = format!("▶ Open ({})", resources.makings.len());
        if bar
            .add(egui::Button::new(RichText::new(said).strong().color(theme::GROUND)).fill(theme::ASSETS))
            .on_hover_text("what is being made, on the whole window")
            .clicked()
        {
            resources.making_full = true;
        }
    }
    if bar
        .button("Import an Image…")
        .on_hover_text(
            "one or more PNGs: they join the drawn objects, under the word of their name (name.4.png: four images side by side); also: drop them on the window",
        )
        .clicked()
    {
        let chosen = workshop_base::pick::files("Images to Import", &[("PNG Images", &["png"])], None);
        if chosen.is_empty() {
            resources.said = Some("no image chosen (a PNG can also be dropped on the window)".to_string());
        } else {
            take_files(resources, chosen);
        }
    }
    if matches!(resources.shelf, Shelf::Pokemon | Shelf::Variants)
        && bar.button("SpriteCollab…").on_hover_text("the PMDCollab community's sprites and portraits, to look at and import").clicked()
    {
        resources.ask_spritecollab = Some(None);
    }
}

/// Where one is (the shelf, or what is sought everywhere), how many, what was said.
pub(super) fn head(ui: &mut egui::Ui, resources: &Resources, count: usize) {
    let head = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), 24.0));
    ui.painter().rect_filled(head, 0.0, theme::TABS);
    let title = if resources.sought.trim().is_empty() { resources.shelf.path() } else { format!("“{}” everywhere", resources.sought.trim()) };
    ui.painter().text(head.left_center() + vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, title, FontId::proportional(theme::BODY), theme::STRONG);
    let said = format!("{count} element{}", if count > 1 { "s" } else { "" });
    ui.painter().text(head.right_center() - vec2(10.0, 0.0), egui::Align2::RIGHT_CENTER, said, FontId::proportional(theme::SMALL), theme::WEAK);
    if let Some(said) = &resources.said {
        ui.painter().with_clip_rect(head).text(head.center(), egui::Align2::CENTER_CENTER, said, FontId::proportional(theme::SMALL), theme::FAINT);
    }
    ui.advance_cursor_after_rect(head);
}

/// A shelf's filters (its generations, its moods), when nothing is sought.
pub(super) fn filters(ui: &mut egui::Ui, resources: &mut Resources) {
    let filters = if resources.sought.trim().is_empty() { filters_of(&resources.session, resources.shelf) } else { Vec::new() };
    if filters.is_empty() {
        if resources.filter.is_some() && resources.sought.trim().is_empty() {
            resources.filter = None;
        }
        return;
    }
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing.x = 4.0;
        if ui.selectable_label(resources.filter.is_none(), "All").clicked() {
            resources.filter = None;
            resources.scroll_wanted = true;
        }
        for (key, said) in &filters {
            let on = resources.filter.as_deref() == Some(key.as_str());
            if ui.selectable_label(on, said).clicked() {
                resources.filter = if on { None } else { Some(key.clone()) };
                resources.scroll_wanted = true;
            }
        }
    });
    ui.add_space(2.0);
}
