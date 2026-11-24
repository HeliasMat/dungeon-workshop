//! The dungeon's and the boss fight's editor, laid out; its checks, its view
//! played.

use std::time::Instant;

use eframe::egui::{self, Color32, Rect, RichText, Sense, pos2, vec2};
use rules::own::plain;
use shell::sound::Hears;

use super::choosers::team_choice;
use super::state::keys_of;
use super::{DungeonMaking, Form, offered};
use workshop_base::done::Done;
use workshop_base::theme;

/// The editor, on the whole window.
pub fn make_dungeon(ui: &mut egui::Ui, making: &mut DungeonMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) -> Option<Done> {
    let mut done = None;
    let whole = ui.available_rect_before_wrap();
    // (For a picture: `WORKSHOP_TRY=1`, tried at once.)
    if making.checked.0.is_empty() && making.trial.is_none() && making.session.launch.try_dungeon {
        making.try_it(audio, making.fight);
    }
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
        making.save();
    }
    // --- The head: its word, Save, Try, Close ---
    let head = Rect::from_min_size(whole.min, vec2(whole.width(), 34.0));
    ui.painter().rect_filled(head, 0.0, theme::TOOLS);
    let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(head.shrink2(vec2(8.0, 5.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    bar.label(theme::text(if making.fight { "Boss Fight" } else { "Dungeon" }, theme::SMALL, theme::WEAK));
    bar.add(egui::TextEdit::singleline(&mut making.word).desired_width(170.0).hint_text("its word")).on_hover_text(if making.fight {
        "the name of its file; a dungeon names it: “fight: <word>”"
    } else {
        "the name of its file, and the word a film writes: “dungeon <word>”"
    });
    if bar.button(if making.saved { "Saved" } else { "Save  Ctrl+S" }).clicked() {
        making.save();
    }
    let trying = making.trial.is_some();
    if bar
        .button(RichText::new(if trying { "■ Stop the Trial" } else { "▶ Try" }).strong().color(theme::ASSETS))
        .on_hover_text("played here, as it is written, by the team chosen on the right")
        .clicked()
    {
        if trying {
            making.stop(audio);
        } else {
            let fight = making.fight;
            making.try_it(audio, fight);
        }
    }
    if bar.button("Close").clicked() {
        making.stop(audio);
        done = Some(Done::Closed);
    }
    // (How many problems the checker finds; a click: to it.)
    if !making.fight && !making.checked.0.is_empty() {
        let (count, grave) = super::check::counted(making);
        let (said, colour) = match (count, grave) {
            (0, _) => ("✔ nothing to review".to_string(), Color32::from_rgb(0x6C, 0xC6, 0x6C)),
            (count, 0) => (format!("▲ {count} to review"), Color32::from_rgb(0xF0, 0xA8, 0x40)),
            (count, grave) => (format!("● {count} to review, {grave} keeping it from playing"), theme::WRONG),
        };
        if bar.button(RichText::new(said).color(colour)).on_hover_text("Check the dungeon: the list on the right").clicked() {
            making.check_sought = true;
        }
    }
    if let Some(said) = &making.said {
        bar.label(theme::text(said, theme::SMALL, theme::WEAK));
    }
    let body = Rect::from_min_max(pos2(whole.min.x, head.max.y), whole.max);
    if let Some(asked) = super::layout::body(ui, body, making, audio, volume) {
        done = Some(asked);
    }
    ui.allocate_rect(whole, Sense::hover());
    making.saved = making.form == making.written;
    done
}

/// A boss fight's own name and its arena's look.
pub(super) fn fight_head(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    let kept = offered(&making.session);
    let offered = kept.as_deref();
    let form = &mut making.form;
    ui.label(theme::text("NAME", theme::SMALL, theme::WEAK));
    ui.add(egui::TextEdit::singleline(&mut form.name).desired_width(f32::INFINITY))
        .on_hover_text("as the game says it on entering, when this fight is played alone");
    let Some(boss) = form.boss.as_mut() else {
        return;
    };
    ui.add_space(8.0);
    ui.label(theme::text("ARENA LOOK", theme::SMALL, theme::WEAK));
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("arena look")
            .width(250.0)
            .selected_text(if boss.look.is_empty() { "the dungeon's" } else { boss.look.as_str() })
            .height(420.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(boss.look.is_empty(), "that of the dungeon naming it").clicked() {
                    boss.look.clear();
                }
                for (_, name, _) in offered.map(|offered| offered.looks.as_slice()).unwrap_or_default() {
                    if ui.selectable_label(plain(name) == plain(&boss.look), name).clicked() {
                        (boss.look, boss.look_floor) = (name.clone(), 1);
                    }
                }
            });
        let floors = offered.and_then(|offered| offered.looks.iter().find(|(_, name, _)| plain(name) == plain(&boss.look))).map_or(1, |(_, _, floors)| *floors);
        if floors > 1 {
            ui.add(egui::DragValue::new(&mut boss.look_floor).range(1..=floors).prefix("floor "));
        }
    });
}

/// The dungeon's boss fight: one written apart, chosen (or a new one), opened
/// to be changed; one written in the dungeon itself, made a file of its own.
pub(super) fn fight_choice(ui: &mut egui::Ui, making: &mut DungeonMaking) -> Option<Done> {
    let mut asked = None;
    ui.add_space(8.0);
    ui.label(theme::text("BOSS FIGHT AT THE END", theme::SMALL, theme::WEAK));
    ui.label(theme::text(
        "one more floor after the last: a fight written apart (Dungeon › All Dungeons), which other dungeons can use too",
        theme::SMALL - 1.0,
        theme::FAINT,
    ));
    let form = &mut making.form;
    if form.boss.is_some() {
        ui.label(RichText::new("this dungeon keeps its fight written in its own file").size(theme::SMALL).color(theme::WEAK));
        if ui.button("Make It a Separate Fight").on_hover_text("written in the fights/ beside this dungeon, and named by it").clicked() {
            let word = format!("{}_fight", film::words::plain(&making.word));
            let fight = Form { head: Vec::new(), name: form.name.clone(), boss: form.boss.take(), ..Form::new("") };
            let folder = making.project.fights();
            _ = std::fs::create_dir_all(&folder);
            match workshop_base::safe::write(&folder.join(format!("{word}.fight")), fight.fight_text()) {
                Ok(()) => {
                    form.fight = word.clone();
                    making.said = Some(format!("fight written apart: fights/{word}.fight (save the dungeon so it names it)"));
                }
                Err(why) => {
                    form.boss = fight.boss;
                    making.said = Some(why.to_string());
                }
            }
        }
        return None;
    }
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("dungeon fight").width(220.0).selected_text(if form.fight.is_empty() { "none" } else { form.fight.as_str() }).show_ui(
            ui,
            |ui| {
                if ui.selectable_label(form.fight.is_empty(), "none").clicked() {
                    form.fight.clear();
                }
                for word in making.project.names("fights", "fight") {
                    if ui.selectable_label(form.fight == word, &word).clicked() {
                        form.fight = word.clone();
                    }
                }
            },
        );
        if !form.fight.is_empty() && ui.button("Edit This Fight").on_hover_text("open it in its editor").clicked() {
            asked = Some(Done::Fight(form.fight.clone()));
        }
        if ui.button("+ New Fight").on_hover_text("a boss fight of its own, in its editor").clicked() {
            asked = Some(Done::Fight(String::new()));
        }
    });
    if !form.fight.is_empty() && !making.project.fight(&form.fight).exists() {
        ui.label(RichText::new(format!("no fights/{}.fight", form.fight)).color(theme::WRONG).size(theme::SMALL));
    }
    asked
}

/// What is wrong with the dungeon (see `dungeon_check`), and its text.
pub(super) fn checks(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    let text = making.file_text();
    if making.checked.0 != text {
        making.checked = (text.clone(), super::check::problems(making));
    }
    super::check::checker(ui, making);
    ui.add_space(8.0);
    ui.label(theme::text("ITS FILE", theme::SMALL, theme::WEAK));
    ui.label(RichText::new(text).monospace().size(theme::SMALL));
}

/// The trial: the dungeon played, its screen as large as fits.
pub(super) fn view(ui: &mut egui::Ui, making: &mut DungeonMaking, audio: &mut Option<Box<dyn Hears>>) {
    team_choice(ui, making);
    ui.add_space(6.0);
    let team = offered(&making.session).map_or_else(String::new, |offered| {
        let (hero, partner, _) = making.team.unwrap_or((offered.team.0, offered.team.1, 5));
        format!("{} and {}", offered.book.species_name(hero), offered.book.species_name(partner))
    });
    let what = if making.fight { "the fight" } else { "the dungeon" };
    let Some(trial) = making.trial.as_mut() else {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("▶ Try").size(22.0).color(theme::ASSETS));
            ui.label(theme::text(format!("{what} played here, as it is written, by {team}"), theme::SMALL, theme::WEAK));
            ui.add_space(12.0);
            ui.label(theme::text(KEYS, theme::SMALL, theme::FAINT));
        });
        return;
    };
    let room = ui.available_rect_before_wrap();
    let room = Rect::from_min_size(room.min, vec2(ui.max_rect().width().min(room.width()), room.height()));
    let (wide, high) = (trial.screen.wide as f32, trial.screen.high as f32);
    let scale = ((room.width() / wide).min((room.height().min(720.0) - 60.0) / high)).max(0.25);
    let shown = Rect::from_min_size(pos2(room.center().x - wide * scale / 2.0, room.min.y), vec2(wide * scale, high * scale));
    let response = ui.interact(shown, egui::Id::new("dungeon trial"), Sense::click());
    // (The keys are the dungeon's while the pointer is over it.)
    let playing = response.hovered();
    let keys = if playing { ui.input(keys_of) } else { shell::Keys::default() };
    if playing {
        ui.memory_mut(|memory| memory.stop_text_input());
    }
    // (At the game's pace, sixty frames a second, however often the editor is drawn.)
    let frames = trial.last.elapsed().as_secs_f32() * 60.0;
    trial.last = Instant::now();
    trial.diving.play(&keys, frames);
    trial.diving.draw(&mut trial.screen);
    let image = egui::ColorImage::from_rgba_unmultiplied([trial.screen.wide, trial.screen.high], &trial.screen.rgba);
    match &mut trial.texture {
        Some(texture) => texture.set(image, egui::TextureOptions::NEAREST),
        None => trial.texture = Some(ui.ctx().load_texture("dungeon trial", image, egui::TextureOptions::NEAREST)),
    }
    if let Some(texture) = &trial.texture {
        ui.painter().image(texture.id(), shown, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    ui.painter().rect_stroke(shown, 0.0, egui::Stroke::new(1.0_f32, if playing { theme::ASSETS } else { theme::GAP }), egui::StrokeKind::Outside);
    let mut below = ui.new_child(
        egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(room.min.x, shown.max.y + 8.0), room.max)).layout(egui::Layout::top_down(egui::Align::Center)),
    );
    let floor = trial.diving.floor().map_or(String::new(), |(at, of)| format!("floor {at} of {of} · "));
    below.label(theme::text(
        format!("{floor}{}", if playing { "the keys go to the dungeon" } else { "pointer over it: the keys go to the dungeon" }),
        theme::SMALL,
        theme::WEAK,
    ));
    below.label(theme::text(KEYS, theme::SMALL - 1.0, theme::FAINT));
    // (Its room taken, so what follows comes under it.)
    ui.allocate_rect(Rect::from_min_max(room.min, pos2(room.max.x, below.min_rect().max.y)), Sense::hover());
    let ended = trial.diving.ended;
    ui.ctx().request_repaint();
    if let Some(cleared) = ended {
        making.stop(audio);
        making.said = Some(if cleared { "over: out by the last floor's stairs".to_string() } else { "over: K.O. or given up".to_string() });
    }
}

const KEYS: &str = "arrows: walk (Y or Shift held: turn on the spot) · Space: attack · 1 to 4: its moves · Enter: confirm · Esc or Backspace: the menu · R: skip the turn · Tab: another of the team (rule “control of the whole team”)";
