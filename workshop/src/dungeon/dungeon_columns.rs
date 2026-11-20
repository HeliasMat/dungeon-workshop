//! A dungeon's columns: what it is (its name, floors, rules, look, tiles,
//! music, the fight at its end); its floors as a grid of who is met and what
//! lies there, floor by floor. And a song chosen, which the fight uses too.

use super::choosers::{chooser, portrait};
use super::editor::fight_choice;
use super::fight_columns::species_id;
use super::layout::{hint, numbers, section};
use super::*;
use crate::session::Session;

/// Whether a range of floors (none: all) holds that floor.
pub(super) fn on_floor(floors: Floors, at: usize) -> bool {
    floors.is_none_or(|(low, high)| low <= at && at <= high)
}

/// The range of floors once that floor's cell is clicked: it grows to it, or
/// shrinks from its end (right click, `all`: every floor).
pub(super) fn floors_clicked(floors: Floors, at: usize, all: bool) -> Floors {
    if all {
        return None;
    }
    match floors {
        None => Some((at, at)),
        Some((low, high)) if at < low => Some((at, high)),
        Some((low, high)) if at > high => Some((low, at)),
        Some((low, high)) if low == high => Some((low, high)),
        Some((low, high)) if at == low => Some((low + 1, high)),
        Some((low, high)) if at == high => Some((low, high - 1)),
        Some(_) => Some((at, at)),
    }
}

/// What one meets on a floor, and how often (in shares of a hundred).
pub(super) fn met_on_floor(foes: &[Foe], at: usize) -> String {
    let here: Vec<&Foe> = foes.iter().filter(|foe| on_floor(foe.floors, at)).collect();
    let total: u32 = here.iter().map(|foe| u32::from(foe.often)).sum::<u32>().max(1);
    let said: Vec<String> = here.iter().map(|foe| format!("{} Lv. {}: {} %", foe.species, foe.level, u32::from(foe.often) * 100 / total)).collect();
    format!("floor {at}\n{}", if said.is_empty() { "no one".to_string() } else { said.join("\n") })
}

/// The floors (from 1 to so many) where no one is met.
pub(super) fn empty_floors(foes: &[Foe], count: usize) -> Vec<usize> {
    (1..=count).filter(|at| !foes.iter().any(|foe| on_floor(foe.floors, *at))).collect()
}

/// A song chosen among the game's (or none: `none_said`), heard with ▶.
#[allow(clippy::too_many_arguments)]
pub(super) fn song_choice(
    ui: &mut egui::Ui,
    id: &str,
    music: &mut String,
    none_said: &str,
    fights: bool,
    heard: &mut Option<String>,
    audio: &mut Option<Box<dyn Hears>>,
    volume: f32,
    session: &Session,
) {
    ui.horizontal(|ui| {
        let shown = rules::songs::SONGS
            .iter()
            .find(|song| plain(song.word) == plain(music))
            .map_or_else(|| if music.is_empty() { none_said.to_string() } else { music.clone() }, |song| format!("{} · {}", song.word, song.title));
        egui::ComboBox::from_id_salt(id).width(260.0).selected_text(shown).height(420.0).show_ui(ui, |ui| {
            if ui.selectable_label(music.is_empty(), none_said).clicked() {
                music.clear();
            }
            for song in rules::songs::SONGS.iter().filter(|song| {
                !fights || song.moods.contains("battle") || song.moods.contains("boss") || song.moods.contains("tension") || song.moods.contains("epic")
            }) {
                if ui.selectable_label(plain(song.word) == plain(music), format!("{} · {}", song.word, song.title)).on_hover_text(song.moods).clicked() {
                    *music = song.word.to_string();
                }
            }
        });
        let song = rules::songs::SONGS.iter().find(|song| plain(song.word) == plain(music));
        let hearing = heard.is_some();
        if ui.add_enabled(song.is_some() || hearing, egui::Button::new(if hearing { "■" } else { "▶" })).on_hover_text("listen to it").clicked() {
            if audio.is_none() {
                *audio = shell::sound::Audio::open_in(session.data(), session.library().clone()).map(|audio| Box::new(audio) as Box<dyn Hears>);
            }
            if let Some(audio) = audio.as_mut() {
                match (hearing, song) {
                    (false, Some(song)) => {
                        audio.song_from(0, Some(song.number), volume, 0.0);
                        audio.volume(0, volume);
                        *heard = Some(song.word.to_string());
                    }
                    _ => {
                        audio.song(0, None, 0.0, 0.0);
                        *heard = None;
                    }
                }
            }
        }
    });
}

/// A file (or folder) opened by the system's own program.
fn open_outside(path: &std::path::Path) -> String {
    let made = if cfg!(windows) {
        std::process::Command::new("cmd").args(["/C", "start", ""]).arg(path).spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(path).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(path).spawn()
    };
    match made {
        Ok(_) => format!("opened: {}", path.display()),
        Err(why) => why.to_string(),
    }
}

/// Its tiles: the look's, or a tileset of one's own drawn on the template of
/// 47 cells; the template and the look's tileset written out to start from,
/// a picture imported (made to the console's rules: 12 palettes).
fn tiles_choice(ui: &mut egui::Ui, session: &Session, form: &mut Form, said: &mut Option<String>, importing: &mut Option<(String, bool)>) {
    let templates = decor::tilesets::folder(session.library()).join("templates");
    tileset_row(ui, session, form, said, importing, &templates);
    tiles_written(ui, session, form, said, importing, &templates);
    tileset_imported(ui, session, form, said, importing);
}

/// The tileset chosen (the look's, or one's own); one not found, imported; one found, opened, its folder.
fn tileset_row(
    ui: &mut egui::Ui,
    session: &Session,
    form: &mut Form,
    said: &mut Option<String>,
    importing: &mut Option<(String, bool)>,
    templates: &std::path::Path,
) {
    let names = decor::tilesets::names(session.library());
    ui.horizontal(|ui| {
        ui.label(theme::text("its tiles", theme::SMALL, theme::WEAK));
        let shown = if form.tileset.is_empty() { "the look's".to_string() } else { form.tileset.clone() };
        egui::ComboBox::from_id_salt("dungeon tiles")
            .width(200.0)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut form.tileset, String::new(), "the look's");
                for name in &names {
                    ui.selectable_value(&mut form.tileset, name.clone(), name);
                }
            })
            .response
            .on_hover_text("a tileset drawn on the template of 47 cells: library/tilesets/<word>.png; the floors keep the look's shape");
        if !form.tileset.is_empty() && !names.contains(&form.tileset) {
            ui.label(RichText::new("not found").size(theme::SMALL).color(theme::WRONG));
            // (Go to: what it should be, imported.)
            if ui.button("Import It…").clicked() {
                let chosen = crate::pick::png("The tileset of this dungeon's “tileset:”", Some(templates));
                *importing = Some((chosen.unwrap_or_else(|| templates.join("template.png")).display().to_string(), false));
            }
        } else if !form.tileset.is_empty() {
            // (Go to: its picture, in the program that paints it; its folder.)
            let file = decor::tilesets::folder(session.library()).join(format!("{}.png", form.tileset));
            if ui.button("Open the PNG").on_hover_text(file.display().to_string()).clicked() {
                *said = Some(open_outside(&file));
            }
            if ui.button("Folder").clicked() {
                *said = Some(open_outside(&decor::tilesets::folder(session.library())));
            }
        }
    });
}

/// The template written out blank, the look's tileset drawn on it, a tileset asked to be imported.
fn tiles_written(
    ui: &mut egui::Ui,
    session: &Session,
    form: &Form,
    said: &mut Option<String>,
    importing: &mut Option<(String, bool)>,
    templates: &std::path::Path,
) {
    ui.horizontal_wrapped(|ui| {
        let template_said = "the blank template: 3 parts (wall, water or lava, floor) of 6×8 cells of 24 pixels; in each cell, the tile in white, its neighbours alike in black, different in blue";
        if ui.button("Export the Template").on_hover_text(template_said).clicked() {
            let path = templates.join("template.png");
            *said = Some(match decor::tilesets::write_blank(&path) {
                Ok(()) => format!("template written: {}", path.display()),
                Err(why) => why.to_string(),
            });
        }
        if ui.button("Export the Look's").on_hover_text("the tileset of the look's dungeon drawn on the template (and its versions .1, .2), to start from a real one").clicked() {
            let spec = offered(session).and_then(|offered| floor::look_floor(&offered, &form.look, form.look_floor).map(|spec| spec.tileset));
            *said = Some(match spec {
                Some(number) => {
                    let path = templates.join(format!("{}.png", plain(&form.look)));
                    match decor::tilesets::export_game(session.data(), number, &path) {
                        Ok(()) => format!("the look's tileset {number} written on the template: {}", path.display()),
                        Err(why) => why.to_string(),
                    }
                }
                None => "no look taken from a game dungeon".to_string(),
            });
        }
        if ui.button("Import a Tileset…").on_hover_text("a PNG drawn on the template (its versions <name>.1.png, <name>.2.png beside it); also: drop it on the window").clicked() {
            let chosen = crate::pick::png("A Tileset Drawn on the Template", Some(templates));
            *importing = Some((chosen.unwrap_or_else(|| templates.join("template.png")).display().to_string(), false));
        }
    });
}

/// A tileset being imported: its path (a file dropped on the window, or
/// browsed), dithered or not; imported, it becomes the dungeon's.
fn tileset_imported(ui: &mut egui::Ui, session: &Session, form: &mut Form, said: &mut Option<String>, importing: &mut Option<(String, bool)>) {
    let Some((path, dither)) = importing.as_mut() else {
        return;
    };
    if let Some(dropped) = ui.ctx().input(|input| input.raw.dropped_files.iter().find_map(|file| file.path.clone())) {
        *path = dropped.display().to_string();
    }
    let mut done = false;
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(path).desired_width((ui.available_width() - 280.0).max(80.0)).hint_text("path of the PNG"));
        if ui.button("Browse…").clicked()
            && let Some(chosen) = crate::pick::png("A Tileset Drawn on the Template", Some(std::path::Path::new(path.trim())))
        {
            *path = chosen.display().to_string();
        }
        ui.checkbox(dither, "dithered").on_hover_text("an ordered dither where colours are missing; without: crisp pixels");
        if ui.button("Import").clicked() {
            let source = std::path::PathBuf::from(path.trim());
            let stem = source.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
            let word = plain(stem.split('.').next().unwrap_or_default());
            *said = Some(match decor::tilesets::import(session.library(), &source, &word, *dither) {
                Ok(written) => {
                    form.tileset = word;
                    done = true;
                    format!("tileset imported (tiles of 8, 12 palettes of 16 colours): {}", written.display())
                }
                Err(why) => why.to_string(),
            });
        }
        if ui.button("Cancel").clicked() {
            done = true;
        }
    });
    if done {
        *importing = None;
    }
}

/// What the dungeon is: its name, floors, rules, look, music, the fight at its end.
pub(super) fn dungeon_column(ui: &mut egui::Ui, making: &mut DungeonMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) -> Option<Done> {
    let kept = offered(&making.session);
    let offered = kept.as_deref();
    let heard = &mut making.heard;
    let form = &mut making.form;
    // (Where the checker points, framed.)
    let pointed = making.pointed;
    let shows = |ui: &egui::Ui, place, rect| check::shows(ui, &pointed, place, rect);
    ui.label(RichText::new("THE DUNGEON").size(theme::SMALL).color(theme::WEAK).strong());
    let name = ui
        .add(egui::TextEdit::singleline(&mut form.name).desired_width(f32::INFINITY).font(egui::FontId::proportional(18.0)))
        .on_hover_text("as the game says it on entering");
    shows(ui, check::Place::Name, name.rect);
    section(ui, "ITS FLOORS");
    let floors = ui
        .add(egui::Slider::new(&mut form.floors, if form.fight.is_empty() && form.boss.is_none() { 1 } else { 0 }..=30).text("floors"))
        .on_hover_text("0: the boss fight alone");
    shows(ui, check::Place::Floors, floors.rect);
    ui.checkbox(&mut form.up, "counted going up (1F, 2F…)");
    ui.checkbox(&mut form.recruit, "recruiting allowed (a beaten Pokémon may join)");
    dungeon_rules(ui, form);
    section(ui, "ITS LOOK");
    hint(ui, "the decor and shape of the floors, taken from a game dungeon; the floors are made anew each time");
    let floors_of_look =
        offered.and_then(|offered| offered.looks.iter().find(|(_, name, _)| plain(name) == plain(&form.look))).map_or(1, |(_, _, floors)| *floors);
    let look = ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("dungeon look")
            .width(250.0)
            .selected_text(if form.look.is_empty() { "—" } else { form.look.as_str() })
            .height(420.0)
            .show_ui(ui, |ui| {
                for (_, name, _) in offered.map(|offered| offered.looks.as_slice()).unwrap_or_default() {
                    if ui.selectable_label(plain(name) == plain(&form.look), name).clicked() {
                        (form.look, form.look_floor) = (name.clone(), 1);
                    }
                }
            });
        if floors_of_look > 1 {
            ui.add(egui::DragValue::new(&mut form.look_floor).range(1..=floors_of_look).prefix("floor "))
                .on_hover_text("the floor of this dungeon whose shape is taken (further on: often larger, darker)");
        }
    });
    shows(ui, check::Place::Look, look.response.rect);
    tiles_choice(ui, &making.session, form, &mut making.said, &mut making.tile_import);
    section(ui, "ITS MUSIC");
    let music = ui.scope(|ui| song_choice(ui, "dungeon music", &mut form.music, "the look's", false, heard, audio, volume, &making.session));
    shows(ui, check::Place::Music, music.response.rect);
    let fight = ui.scope(|ui| fight_choice(ui, making));
    shows(ui, check::Place::Fight, fight.response.rect);
    let asked = fight.inner;
    end_fight_said(ui, making);
    asked
}

/// Its rules: the options (each off otherwise), the level the team enters at.
fn dungeon_rules(ui: &mut egui::Ui, form: &mut Form) {
    section(ui, "ITS RULES");
    hint(ui, "options, each off otherwise (“rules:” in the text)");
    for (word, said) in rules::rule_options::WORDS {
        if let Some(on) = form.options.flag(word) {
            let shown = ui.checkbox(on, said);
            if word == "fairy" {
                shown.on_hover_text("then, in the text: “pokemon_type: Clefairy fairy” and “move_type: Charm fairy”");
            }
        }
    }
    ui.horizontal(|ui| {
        let mut said = form.team_level.is_some();
        if ui
            .checkbox(&mut said, "the team enters at level")
            .on_hover_text("without: at the level it has (the film's: “dungeon <word> level n”, else as it starts)")
            .changed()
        {
            form.team_level = said.then_some(5);
        }
        if let Some(level) = form.team_level.as_mut() {
            ui.add(egui::DragValue::new(level).range(1..=100));
        }
    });
}

/// The fight at its end in numbers, if it has one: the boss, its level, its HP, how hard it looks.
fn end_fight_said(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    if (!making.form.fight.is_empty() || making.form.boss.is_some())
        && let Some(counted) = numbers(making)
    {
        ui.horizontal(|ui| {
            ui.label(theme::text(format!("{} Lv. {}, {} HP", counted.boss.name, counted.boss.level, counted.boss.hp), theme::SMALL, theme::TEXT));
            ui.label(theme::text("·", theme::SMALL, theme::FAINT));
            ui.label(RichText::new(counted.verdict.0).size(theme::SMALL).strong().color(counted.verdict.1));
        });
    }
}

/// A floor's cells along a row: in or out of the range (in that colour); a
/// click takes the floor in or out, a right click all floors.
fn floor_cells(ui: &mut egui::Ui, floors: &mut Floors, count: usize, cell: f32, inside_colour: Color32) {
    for floor in 1..=count {
        let (rect, response) = ui.allocate_exact_size(vec2(cell, 20.0), Sense::click());
        let inside = on_floor(*floors, floor);
        ui.painter().rect_filled(rect.shrink(1.0), 2.0, if inside { inside_colour } else { Color32::from_rgb(0x2A, 0x2A, 0x30) });
        if response.clicked() {
            *floors = floors_clicked(*floors, floor, false);
        }
        if response.secondary_clicked() {
            *floors = floors_clicked(*floors, floor, true);
        }
    }
}

/// The floors' numbers over the grid (a click: that floor shown under it, as the game lays it out; pointed: who is met there), and the boss's.
fn floors_header(ui: &mut egui::Ui, form: &Form, shown: &mut usize, pointed: &Option<(check::Place, Instant)>, cell: f32, fight: bool) {
    ui.horizontal(|ui| {
        ui.add_space(300.0);
        for at in 1..=form.floors {
            let (rect, response) = ui.allocate_exact_size(vec2(cell, 16.0), Sense::click());
            if at == *shown {
                ui.painter().rect_filled(rect.shrink(1.0), 2.0, theme::CHOSEN);
            }
            check::shows(ui, pointed, check::Place::Floor(at), rect);
            if response.clicked() {
                *shown = at;
            }
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, at.to_string(), egui::FontId::proportional(11.0), theme::WEAK);
            response.on_hover_text(met_on_floor(&form.foes, at));
        }
        if fight {
            let (rect, response) = ui.allocate_exact_size(vec2(cell * 1.5, 16.0), Sense::hover());
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "boss", egui::FontId::proportional(10.0), Color32::from_rgb(0xE5, 0x50, 0x50));
            response.on_hover_text("the boss fight, one more floor after the last");
        }
    });
}

/// Its floors as a grid: a row for each Pokémon met and each item lying
/// there, a column for each floor (and the fight's, at the end); a click on
/// a cell takes its floor in or out (from that floor to the next ones, or
/// from the first to that one), a right click: on all floors. Over a
/// floor's number, what one meets there and how often.
pub(super) fn floors_column(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    let count = making.form.floors;
    let fight = !making.form.fight.is_empty() || making.form.boss.is_some();
    ui.label(RichText::new("ITS FLOORS").size(theme::SMALL).color(theme::WEAK).strong());
    hint(ui, "a click on a cell: that floor added (or removed, at the edge); right click: all floors; “often”: how many are met");
    let cell = ((ui.available_width() - 330.0) / (count as f32 + if fight { 1.5 } else { 0.0 })).clamp(14.0, 30.0);
    section(ui, "THE POKÉMON MET THERE");
    floors_header(ui, &making.form, &mut making.floor_shown.at, &making.pointed, cell, fight);
    let empty = empty_floors(&making.form.foes, count);
    foes_rows(ui, making, cell);
    if fight && let Some(counted) = numbers(making) {
        boss_floor_row(ui, &format_boss_floor(&counted.boss.name, counted.boss.level, counted.guards.len()), cell, count);
    }
    if count == 0 {
        hint(ui, "0 floors: the boss fight alone; those met there and the items on the ground are not used");
    }
    if !empty.is_empty() {
        ui.label(
            RichText::new(format!("no one on floors {}", empty.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")))
                .color(theme::WRONG)
                .size(theme::SMALL),
        );
    }
    let names: Vec<String> = offered(&making.session).map(|offered| offered.species.iter().map(|(_, name)| name.clone()).collect()).unwrap_or_default();
    let form = &mut making.form;
    let chosen = ui.scope(|ui| {
        chooser(ui, "a Pokémon…", &mut making.sought_foe, names, |species| {
            let level = form.foes.last().map_or(5, |foe| foe.level);
            form.foes.push(Foe { species, level, often: 1, floors: None });
        })
    });
    check::shows(ui, &making.pointed, check::Place::Foes, chosen.response.rect);
    section(ui, "THE ITEMS ON THE GROUND");
    floors_header(ui, &making.form, &mut making.floor_shown.at, &making.pointed, cell, fight);
    items_rows(ui, making, cell);
    let items = offered(&making.session).map(|offered| offered.items.clone()).unwrap_or_default();
    let form = &mut making.form;
    chooser(ui, "an item…", &mut making.sought_item, items, |item| form.items.push(Lying { item, often: 1, floors: None }));
    floor::floor_preview(ui, making);
}

/// What the boss's row says: who waits at the end, its level, its guards.
pub(super) fn format_boss_floor(name: &str, level: i32, guards: usize) -> String {
    format!("at the end: {name} Lv. {level} (and {guards} guard{})", if guards > 1 { "s" } else { "" })
}

/// The Pokémon met, a row each: portrait, species (unknown: in red), level, how often, ×; their floors.
fn foes_rows(ui: &mut egui::Ui, making: &mut DungeonMaking, cell: f32) {
    let count = making.form.floors;
    let mut gone = None;
    for (at, foe) in making.form.foes.iter_mut().enumerate() {
        let row = ui.horizontal(|ui| {
            match species_id(&making.session, &foe.species).and_then(|id| portrait(ui.ctx(), making.session.data(), &mut making.portraits, id)) {
                Some(texture) => _ = ui.add(egui::Image::new(&texture).fit_to_exact_size(vec2(24.0, 24.0))),
                None => _ = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::hover()),
            }
            let known = species_id(&making.session, &foe.species).is_some();
            ui.add_sized(
                vec2(96.0, 20.0),
                egui::Label::new(RichText::new(foe.species.as_str()).strong().color(if known { theme::TEXT } else { theme::WRONG }).size(theme::SMALL))
                    .truncate(),
            );
            ui.add_sized(vec2(64.0, 20.0), egui::DragValue::new(&mut foe.level).range(1..=100).prefix("Lv. "));
            ui.add_sized(vec2(70.0, 20.0), egui::DragValue::new(&mut foe.often).range(1..=20).prefix("often "));
            if ui.small_button("×").clicked() {
                gone = Some(at);
            }
            ui.add_space(300.0 - 24.0 - 96.0 - 64.0 - 70.0 - 40.0);
            floor_cells(ui, &mut foe.floors, count, cell, Color32::from_rgb(0x4A, 0x9E, 0x60));
        });
        check::shows(ui, &making.pointed, check::Place::Foe(at), row.response.rect);
    }
    if let Some(at) = gone {
        making.form.foes.remove(at);
    }
}

/// The fight at the end, on its own floor after the last.
fn boss_floor_row(ui: &mut egui::Ui, said: &str, cell: f32, count: usize) {
    ui.horizontal(|ui| {
        ui.add_space(28.0);
        ui.add_sized(vec2(268.0, 20.0), egui::Label::new(RichText::new(said).size(theme::SMALL).color(Color32::from_rgb(0xE5, 0x50, 0x50))).truncate());
        ui.add_space(4.0);
        ui.add_space(cell * count as f32);
        let (rect, _) = ui.allocate_exact_size(vec2(cell * 1.5, 20.0), Sense::hover());
        ui.painter().rect_filled(rect.shrink(1.0), 2.0, Color32::from_rgb(0xC0, 0x40, 0x40));
    });
}

/// The items lying there, a row each: the item (unknown: in red), how often, ×; their floors.
fn items_rows(ui: &mut egui::Ui, making: &mut DungeonMaking, cell: f32) {
    let kept = offered(&making.session);
    let offered = kept.as_deref();
    let count = making.form.floors;
    let mut gone = None;
    for (at, lying) in making.form.items.iter_mut().enumerate() {
        let row = ui.horizontal(|ui| {
            let known = offered.is_none_or(|offered| offered.items.iter().any(|name| plain(name) == plain(&lying.item)));
            ui.add_sized(
                vec2(124.0, 20.0),
                egui::Label::new(RichText::new(lying.item.as_str()).color(if known { theme::TEXT } else { theme::WRONG }).size(theme::SMALL)).truncate(),
            );
            ui.add_sized(vec2(70.0, 20.0), egui::DragValue::new(&mut lying.often).range(1..=20).prefix("often "));
            if ui.small_button("×").clicked() {
                gone = Some(at);
            }
            ui.add_space(300.0 - 124.0 - 70.0 - 30.0);
            floor_cells(ui, &mut lying.floors, count, cell, Color32::from_rgb(0x40, 0x88, 0xC0));
        });
        check::shows(ui, &making.pointed, check::Place::Item(at), row.response.rect);
    }
    if let Some(at) = gone {
        making.form.items.remove(at);
    }
}
