//! A Pokémon's page: who it is (its number, generation, types, word, where
//! its pictures come from), Use in Film, then its parts: its animations
//! (the game's, the community's; each playing while pointed, by the words a
//! film says it with), its faces (by their names in a film), its forms, its
//! credits (SpriteCollab's authors).

use std::time::Instant;

use eframe::egui::{self, Color32, FontId, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};

use super::super::catalog::{Source, Species};
use super::super::pictures::{body_bounds, body_sheet, cut_step, face_name, faces_of, read_png, the_games};
use super::super::{Item, Resources, Sheet, Shelf};
use super::{Shown, plural, section};
use workshop_base::asset::Asset;
use workshop_base::theme;
use workshop_picture::fitted;
use workshop_script::Scripts;

/// The parts of a Pokémon's page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Animations,
    Faces,
    Forms,
    Credits,
}

impl Page {
    const ALL: [Page; 4] = [Page::Animations, Page::Faces, Page::Forms, Page::Credits];

    fn name(self) -> &'static str {
        match self {
            Page::Animations => "Animations",
            Page::Faces => "Faces",
            Page::Forms => "Forms",
            Page::Credits => "Credits",
        }
    }

    /// By its word (`WORKSHOP_RESOURCE=pokemon:eevee:faces`).
    pub fn of(word: &str) -> Option<Page> {
        Page::ALL.into_iter().find(|page| page.name().eq_ignore_ascii_case(word.trim()))
    }
}

/// The page of the Pokémon chosen.
pub(super) fn page(ui: &mut egui::Ui, resources: &mut Resources, scripts: &mut Scripts, item: &Item, shown: &Shown) {
    let catalog = resources.catalog();
    let Some((species, entry)) = catalog.entry(&item.word) else { return };
    let (species, entry) = (species.clone(), entry.clone());
    who_it_is(ui, resources, &species, &entry);
    use_in_film(
        ui,
        resources,
        scripts,
        &Asset::Pokemon { word: entry.word.clone() },
        "Add to the Film",
        "its word at the Script's cursor (its scene's cast: drag it from the Film's Assets onto the Scene)",
    );
    if entry.source == Source::Missing {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(8.0);
            ui.label(theme::text("On SpriteCollab, not imported yet: it is imported in the background when first used, or now:", theme::SMALL, theme::WEAK));
            if ui.add_enabled(resources.run.is_none(), egui::Button::new("Import Now")).clicked() {
                resources.import_if_missing(&entry.word);
            }
        });
        return;
    }
    let face_count = faces_of(&resources.session, item.number).len();
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        for page in Page::ALL {
            let count = match page {
                Page::Animations => format!(" ({})", shown.anims.len()),
                Page::Faces => format!(" ({face_count})"),
                Page::Forms => format!(" ({})", species.entries.len()),
                Page::Credits => String::new(),
            };
            let here = resources.page == page;
            let said = RichText::new(format!("{}{count}", page.name())).size(theme::BODY).color(if here { theme::GROUND } else { theme::TEXT });
            if ui.add(egui::Button::new(said).fill(if here { theme::ASSETS } else { theme::FIELD }).corner_radius(8)).clicked() {
                resources.page = page;
            }
        }
    });
    match resources.page {
        Page::Animations => animations(ui, resources, scripts, item, shown),
        Page::Faces => faces(ui, resources, scripts, item, shown),
        Page::Forms => forms(ui, resources, &species, &entry.word),
        Page::Credits => credits(ui, resources, &species, &entry),
    }
}

/// Its name, number, generation, types, word, where its pictures come from.
fn who_it_is(ui: &mut egui::Ui, resources: &mut Resources, species: &Species, entry: &super::super::catalog::Entry) {
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(RichText::new(&entry.name).size(theme::TITLE).strong().color(theme::STRONG));
        if species.dex > 0 {
            ui.label(theme::text(format!("#{:04} · Gen. {}", species.dex, species.generation), theme::SMALL, theme::WEAK));
        }
        for kind in &species.types {
            let [red, green, blue] = rules::dungeon::TYPE_COLOURS.get(usize::from(*kind)).copied().unwrap_or([120, 120, 120]);
            let name = rules::dungeon::TYPE_NAMES.get(usize::from(*kind)).copied().unwrap_or("?");
            egui::Frame::new().fill(Color32::from_rgb(red, green, blue)).corner_radius(6).inner_margin(egui::Margin::symmetric(6, 1)).show(ui, |ui| {
                ui.label(theme::text(name, theme::SMALL, Color32::BLACK));
            });
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(theme::text("In a film:", theme::SMALL, theme::WEAK));
        ui.add(egui::Label::new(RichText::new(&entry.word).monospace().size(theme::BODY).color(Color32::from_rgb(0xDC, 0xDC, 0xAA))).selectable(true));
        if ui.small_button("Copy").on_hover_text("the word, to type in a film").clicked() {
            ui.ctx().copy_text(entry.word.clone());
            resources.said = Some(format!("copied: {}", entry.word));
        }
        let from = match entry.source {
            Source::Game => "the game's sprites",
            Source::Community => "the community's (SpriteCollab)",
            Source::Hand => "drawn by hand",
            Source::Missing => "SpriteCollab, not imported yet",
        };
        ui.label(theme::text(format!("· {from}"), theme::SMALL, theme::FAINT));
    });
}

/// A button writing an asset in the open film, at the Script's cursor.
fn use_in_film(ui: &mut egui::Ui, resources: &mut Resources, scripts: &mut Scripts, asset: &Asset, said: &str, hover: &str) {
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let button = egui::Button::new(RichText::new(format!("+ {said}")).color(theme::GROUND)).fill(theme::FILM);
        if ui.add(button).on_hover_text(hover).clicked() {
            write(resources, scripts, asset);
        }
        ui.label(theme::text(asset.said(), theme::SMALL, theme::FAINT));
    });
}

/// An asset written at the open film's cursor (a Pokémon not imported yet: imported first, in the background).
fn write(resources: &mut Resources, scripts: &mut Scripts, asset: &Asset) {
    let who = match asset {
        Asset::Pokemon { word } | Asset::Face { who: word, .. } | Asset::Gesture { who: word, .. } => word.clone(),
        _ => String::new(),
    };
    let importing = resources.import_if_missing(&who);
    let said = match scripts.open.as_mut() {
        Some(open) => open.put_asset(asset, None, None, None),
        None => "no film open: open one in the Film space".to_string(),
    };
    resources.said = Some(if importing { format!("{said} (importing it from SpriteCollab…)") } else { said });
}

/// The time a tile has been pointed at (playing from its start), if it is.
fn pointed_since(resources: &mut Resources, key: &str, hovered: bool) -> Option<f64> {
    if !hovered {
        return None;
    }
    let since = match &resources.picker.resting {
        Some((known, when)) if known == key => *when,
        _ => {
            resources.picker.resting = Some((key.to_string(), Instant::now()));
            Instant::now()
        }
    };
    Some(since.elapsed().as_secs_f64())
}

/// A tile: its picture's room, chosen or not, pointed or not; its name under it.
fn tile(ui: &mut egui::Ui, size: egui::Vec2, chosen: bool, name: &str) -> (egui::Response, Rect) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if chosen {
        theme::CHOSEN
    } else if response.hovered() {
        theme::HOVERED
    } else {
        theme::GROUND
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    if chosen {
        ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.5_f32, theme::ASSETS), StrokeKind::Inside);
    }
    let mut job = egui::text::LayoutJob::simple_singleline(name.to_string(), FontId::proportional(theme::SMALL - 0.5), theme::TEXT);
    job.wrap = egui::text::TextWrapping { max_width: rect.width() - 4.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(pos2(rect.center().x - galley.size().x / 2.0, rect.max.y - 15.0), galley, theme::TEXT);
    let room = Rect::from_min_max(rect.min + vec2(3.0, 3.0), pos2(rect.max.x - 3.0, rect.max.y - 17.0));
    (response, room)
}

/// The words a film says an animation with: its gesture (`sit`), a pose (`pose look_up`), else `anim Name`.
pub(in super::super) fn film_words(anim: &str) -> String {
    let gestures = super::super::picker::gesture_words_known();
    match film::POSES.iter().find(|(_, named, _)| *named == anim).map(|(name, ..)| *name) {
        Some(name) if gestures.contains(name) => name.to_string(),
        Some(name) => format!("pose {name}"),
        None => format!("anim {anim}"),
    }
}

/// Its animations, the game's then the community's: each playing while pointed; a click shows it in large, a
/// double click writes it in the film.
fn animations(ui: &mut egui::Ui, resources: &mut Resources, scripts: &mut Scripts, item: &Item, shown: &Shown) {
    let number = item.number;
    let (games, others): (Vec<_>, Vec<_>) = shown.anims.iter().cloned().partition(|(file, _)| the_games(&resources.session, number, *file));
    let mut written = None;
    for (title, anims) in [("The Game's", games), (if item.number >= 2000 { "Its Animations" } else { "The Community's" }, others)] {
        if anims.is_empty() {
            continue;
        }
        section(ui, &format!("{title} · {}", anims.len()));
        ui.horizontal_wrapped(|ui| {
            ui.add_space(8.0);
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (file, anim) in &anims {
                let words = film_words(anim);
                let chosen = shown.body && resources.anim == *file;
                let label = words.trim_start_matches("pose ").trim_start_matches("anim ").replace('_', " ");
                let (response, room) = tile(ui, vec2(70.0, 80.0), chosen, &label);
                if !ui.is_rect_visible(room) {
                    continue;
                }
                let key = format!("body:{number}:{file}");
                let since = pointed_since(resources, &key, response.hovered());
                if !resources.pictures.contains_key(&key) && resources.read_now < super::super::READ_EACH_FRAME {
                    resources.read_now += 1;
                    let made = body_sheet(&resources.session, ui.ctx(), number, *file);
                    resources.pictures.insert(key.clone(), made);
                    resources.bounds.insert(key.clone(), body_bounds(&resources.session, number, *file));
                }
                match resources.pictures.get(&key).cloned().flatten() {
                    Some(sheet) => {
                        let step = since.map_or(0, |seconds| sheet.step_at(seconds));
                        let (uv, size) = cut_step(&sheet, step, resources.bounds.get(&key).copied().flatten());
                        ui.painter().image(sheet.texture.id(), fitted(size, room), uv, Color32::WHITE);
                        if since.is_some() {
                            ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
                        }
                    }
                    None => {
                        ui.ctx().request_repaint();
                    }
                }
                let response =
                    response.on_hover_text(format!("{} {words} · its {anim} animation\na click: in large · a double click: written in the film", item.word));
                if response.double_clicked() {
                    written = Some(Asset::Gesture { who: item.word.clone(), words: words.clone() });
                } else if response.clicked() {
                    (resources.anim, resources.playing, resources.body) = (*file, true, Some(true));
                }
            }
        });
    }
    if let Some(asset) = written {
        write(resources, scripts, &asset);
    }
    if let Some((_, anim)) = shown.anims.iter().find(|(file, _)| shown.body && *file == resources.anim) {
        ui.add_space(4.0);
        let asset = Asset::Gesture { who: item.word.clone(), words: film_words(anim) };
        use_in_film(ui, resources, scripts, &asset, "Use This Animation", "this line at the Script's cursor");
    }
}

/// Its faces, by the names a film gives them: a click shows one in large, a double click writes it in the film.
fn faces(ui: &mut egui::Ui, resources: &mut Resources, scripts: &mut Scripts, item: &Item, shown: &Shown) {
    let faces = faces_of(&resources.session, item.number);
    section(ui, &format!("Faces · {}", faces.len()));
    let mut written = None;
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        for (at, face) in faces.iter().enumerate() {
            let name = face_name(*face);
            let chosen = !shown.body && resources.face == at;
            let (response, room) = tile(ui, vec2(62.0, 72.0), chosen, &name.replace('_', " "));
            let key = format!("face:{}:{face}", item.number);
            if !resources.pictures.contains_key(&key) && resources.read_now < super::super::READ_EACH_FRAME {
                resources.read_now += 1;
                let path = std::path::PathBuf::from(assets::data::path(resources.session.data(), &format!("portraits/{}_{face}.png", item.number)));
                let made = read_png(ui.ctx(), &path, &key, 0).map(|texture| Sheet { cell: texture.size_vec2(), texture, count: 1, ticks: vec![60] });
                resources.pictures.insert(key.clone(), made);
            }
            if let Some(sheet) = resources.pictures.get(&key).cloned().flatten() {
                ui.painter().image(sheet.texture.id(), fitted(sheet.cell, room), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            }
            let response = response.on_hover_text(format!("{} ({name}): …\nface No. {face} · a double click: a line said with it", item.word));
            if response.double_clicked() {
                written = Some(Asset::Face { who: item.word.clone(), face: name.clone() });
            } else if response.clicked() || response.hovered() {
                (resources.face, resources.body) = (at, Some(false));
            }
        }
    });
    if let Some(asset) = written {
        write(resources, scripts, &asset);
    }
    if !shown.body
        && let Some(face) = faces.get(resources.face)
    {
        ui.add_space(4.0);
        let asset = Asset::Face { who: item.word.clone(), face: face_name(*face) };
        use_in_film(
            ui,
            resources,
            scripts,
            &asset,
            "Use This Face",
            "a spoken line with it, at the Script's cursor (onto a spoken line, from the Film's Assets: that line's face)",
        );
    }
}

/// Its forms (the species first): a click shows one's page.
fn forms(ui: &mut egui::Ui, resources: &mut Resources, species: &Species, here: &str) {
    section(ui, &format!("{} · {} form{}", species.name, species.forms(), plural(species.forms())));
    let mut chosen = None;
    for (at, entry) in species.entries.iter().enumerate() {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
        let fill = if entry.word == here {
            theme::CHOSEN
        } else if response.hovered() {
            theme::HOVERED
        } else {
            theme::GROUND
        };
        ui.painter().rect_filled(rect.shrink2(vec2(8.0, 1.0)), 3.0, fill);
        let session = resources.session.clone();
        if let Some(face) = resources.thumbs.of(&session, entry.number).and_then(|thumb| thumb.face.clone()) {
            let room = Rect::from_min_size(rect.min + vec2(12.0, 3.0), vec2(40.0, 40.0));
            ui.painter().image(face.id(), room, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        let name = if at == 0 { format!("{} (the species)", species.name) } else { entry.form.clone() };
        ui.painter().text(rect.min + vec2(60.0, 6.0), egui::Align2::LEFT_TOP, name, FontId::proportional(theme::BODY), theme::STRONG);
        let badge = entry.source.badge().map_or(String::new(), |badge| format!("  · {badge}"));
        ui.painter().text(
            rect.min + vec2(60.0, 24.0),
            egui::Align2::LEFT_TOP,
            format!("{}{badge}", entry.word),
            FontId::monospace(theme::SMALL - 0.5),
            Color32::from_rgb(0xDC, 0xDC, 0xAA),
        );
        if response.clicked() {
            chosen = Some(entry.word.clone());
        }
    }
    if let Some(word) = chosen {
        let item = Item { shelf: Shelf::Pokemon, word, label: String::new(), about: String::new(), number: 0, file: None, order: 0 };
        resources.choose(&item);
        resources.page = Page::Forms;
    }
}

/// Who made its sprite and its portraits (SpriteCollab's credits), what each made.
fn credits(ui: &mut egui::Ui, resources: &mut Resources, species: &Species, entry: &super::super::catalog::Entry) {
    let places = workshop_base::tool_run::places(&resources.session);
    let credits = resources.credits.get_or_insert_with(|| std::sync::Arc::new(spritecollab::credits(&places))).clone();
    let units = resources.form_units.get_or_insert_with(|| std::sync::Arc::new(spritecollab::form_words(&places.clone))).clone();
    let unit = entry.unit.or_else(|| units.get(&entry.word).copied()).or_else(|| {
        let dex = u32::try_from(species.dex).ok().filter(|dex| *dex > 0)?;
        (entry.number < 5000).then_some((dex, 0))
    });
    let found = unit.and_then(|unit| credits.get(&unit));
    for (portrait, title) in [(false, "Sprite"), (true, "Portraits")] {
        section(ui, title);
        let made: Vec<&spritecollab::Credit> = found.map(|all| all.iter().filter(|credit| credit.portrait == portrait).collect()).unwrap_or_default();
        if made.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                let said = match entry.source {
                    Source::Game => "the game's own (Spike Chunsoft)",
                    Source::Hand => "drawn by hand, in this project",
                    _ => "no credit written (data/extra/spritecollab_credits.txt)",
                };
                ui.label(theme::text(said, theme::SMALL, theme::WEAK));
            });
            continue;
        }
        for credit in made {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(8.0);
                ui.label(RichText::new(&credit.name).size(theme::BODY).strong().color(if credit.current { theme::STRONG } else { theme::WEAK }));
                ui.label(theme::text(format!("{}{}", credit.date, if credit.current { "" } else { " · replaced since" }), theme::SMALL, theme::FAINT));
                if credit.contact.starts_with("http") {
                    ui.hyperlink_to(theme::text("contact", theme::SMALL, theme::DECOR), &credit.contact);
                }
            });
            if !credit.what.is_empty() {
                egui::Frame::new().inner_margin(egui::Margin { left: 18, right: 8, top: 0, bottom: 2 }).show(ui, |ui| {
                    let shown: Vec<&str> = credit.what.iter().take(14).map(String::as_str).collect();
                    let more = credit.what.len().saturating_sub(shown.len());
                    let said = if more > 0 { format!("{} and {more} more", shown.join(", ")) } else { shown.join(", ") };
                    ui.label(theme::text(said, theme::SMALL, theme::TEXT));
                });
            }
        }
    }
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(8.0);
        ui.label(theme::text("The community's sprites: CC BY-NC 4.0, credit given.", theme::SMALL, theme::FAINT));
        if ui.small_button("SpriteCollab…").on_hover_text("Tools › SpriteCollab: its folders, import again, export").clicked() {
            let dex = u32::try_from(species.dex).ok().filter(|dex| *dex > 0);
            resources.session.ask(workshop_base::ask::Ask::SpriteCollab(dex));
        }
    });
}
