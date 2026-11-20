//! A Pokémon's own actions: its progression as a CSV, SpriteCollab, its
//! sheets for SpriteBot written or read.

use eframe::egui;

use super::super::shelves::dex_of;
use super::super::{Item, Resources, SheetImport};
use crate::theme;

/// Its actions, the work on its thread told when done, the import asked.
pub(super) fn actions(ui: &mut egui::Ui, resources: &mut Resources, item: &Item) {
    work_done(resources);
    let busy = resources.run.is_some();
    if busy {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }
    buttons(ui, resources, item, busy);
    let Some(asked) = resources.sheet_import.as_mut() else {
        return;
    };
    let (go, mut done) = import_form(ui, asked, item);
    if go {
        let source = std::path::PathBuf::from(asked.source.trim());
        let into = if asked.variant {
            // (A variant is of the species itself, not of another variant.)
            let species = crate::level_csv::species_of(&resources.session, item.number).map_or(item.number, i64::from);
            spritecollab::Target::Variant { word: film::words::plain(asked.word.trim()), of: u32::try_from(species).unwrap_or(0) }
        } else {
            spritecollab::Target::Monster(u32::try_from(item.number).unwrap_or(0))
        };
        let places = crate::tool_run::places(&resources.session);
        resources.run = Some(crate::tool_run::Run::spawn("import", move || spritecollab::import_from(&places, &source, into)));
        done = true;
    }
    if done {
        resources.sheet_import = None;
    }
}

/// The work's end told (an import: the Pokémon read again).
fn work_done(resources: &mut Resources) {
    if let Some(run) = resources.run.as_mut()
        && let Some(done) = run.poll().cloned()
    {
        let doing = run.doing.clone();
        resources.run = None;
        resources.said = Some(match done {
            Ok(said) => {
                if doing.starts_with("import") {
                    resources.forget();
                    format!("{said} (new Pokémon show everywhere once the Workshop is started again)")
                } else {
                    said
                }
            }
            Err(why) => why,
        });
    }
}

/// Exported as a CSV, SpriteCollab, exported for SpriteBot or imported from it.
fn buttons(ui: &mut egui::Ui, resources: &mut Resources, item: &Item, busy: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        if ui.button("Export as CSV…").on_hover_text("its progression: exp., stats and moves learned at each level, from the game's tables").clicked() {
            resources.said = Some(crate::level_csv::export(&resources.session, item.number, &item.label));
        }
        if ui.button("SpriteCollab…").on_hover_text("its community sprites and portraits, to look at and import").clicked() {
            resources.ask_spritecollab = Some(u32::try_from(dex_of(&resources.session, item.number)).ok().filter(|dex| *dex > 0));
        }
        if ui
            .add_enabled(!busy, egui::Button::new("Export for SpriteBot…"))
            .on_hover_text("sprite/AnimData.xml and <Anim>-Anim.png, portrait/<Face>.png and portrait_sheet.png, as SpriteCollab serves them")
            .clicked()
            && let Some(folder) = crate::pick::folder("Where to Write Its SpriteBot Sheets", None)
        {
            let target = folder.join(&item.word);
            let monster = u32::try_from(item.number).unwrap_or(0);
            let places = crate::tool_run::places(&resources.session);
            resources.run = Some(crate::tool_run::Run::spawn("export", move || spritecollab::export(&places, monster, &target)));
        }
        if ui
            .add_enabled(!busy, egui::Button::new("Import from SpriteBot…"))
            .on_hover_text("a folder (or a zip) as SpriteCollab serves them: in its place, or as a variant")
            .clicked()
        {
            resources.sheet_import = Some(SheetImport { source: String::new(), variant: false, word: format!("{}_bis", item.word) });
        }
        if busy {
            ui.spinner();
        }
    });
}

/// Where the sheets are read from, in its place or as a variant; whether to
/// import (and whether it is given up).
fn import_form(ui: &mut egui::Ui, asked: &mut SheetImport, item: &Item) -> (bool, bool) {
    let mut done = false;
    let mut go = false;
    egui::Frame::new().fill(theme::GROUND).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut asked.source).hint_text("folder or .zip").desired_width((ui.available_width() - 150.0).max(80.0)));
            if ui.button("Folder…").clicked()
                && let Some(folder) = crate::pick::folder("A SpriteBot Folder (AnimData.xml…)", None)
            {
                asked.source = folder.display().to_string();
            }
            if ui.button("Zip…").clicked()
                && let Some(file) = crate::pick::file("A SpriteBot Zip", &[("Zip", &["zip"])], None)
            {
                asked.source = file.display().to_string();
            }
        });
        if let Some(dropped) = ui.ctx().input(|input| input.raw.dropped_files.iter().find_map(|file| file.path.clone())) {
            asked.source = dropped.display().to_string();
        }
        ui.horizontal(|ui| {
            ui.radio_value(&mut asked.variant, false, theme::text(format!("in place of {}", item.label), theme::SMALL, theme::TEXT));
            ui.radio_value(&mut asked.variant, true, theme::text("as a variant:", theme::SMALL, theme::TEXT));
            ui.add_enabled(asked.variant, egui::TextEdit::singleline(&mut asked.word).desired_width(120.0));
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(!asked.source.trim().is_empty(), egui::Button::new("Import")).clicked() {
                go = true;
            }
            if ui.button("Cancel").clicked() {
                done = true;
            }
            ui.label(theme::text("into data/extra; a variant gets its own number, 5000 and on", theme::SMALL, theme::FAINT));
        });
    });
    (go, done)
}
