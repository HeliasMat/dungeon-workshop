//! Choices in the editor: a sound, a name among many, a portrait, the team.

use std::collections::HashMap;

use eframe::egui::{self, TextureHandle, vec2};
use rules::own::plain;

use super::{DungeonMaking, offered};
use workshop_base::theme;

/// A cry chosen among our own sounds (library/sounds), or none.
pub(super) fn sound_choice(ui: &mut egui::Ui, id: (&str, usize), cry: &mut String, sounds: &[String]) {
    egui::ComboBox::from_id_salt(id)
        .width(170.0)
        .selected_text(if cry.is_empty() { "cry: —".to_string() } else { format!("cry: {cry}") })
        .height(300.0)
        .show_ui(ui, |ui| {
            if ui.selectable_label(cry.is_empty(), "— (none)").clicked() {
                cry.clear();
            }
            for name in sounds {
                if ui.selectable_label(cry == name, name).clicked() {
                    *cry = name.clone();
                }
            }
        })
        .response
        .on_hover_text("a sound of ours (library/sounds), that it cries at that moment");
}

/// A name sought among those offered: typed, the closest under it, a click
/// (or Enter) takes it.
pub(super) fn chooser(ui: &mut egui::Ui, hint: &str, sought: &mut String, names: Vec<String>, mut take: impl FnMut(String)) {
    let field = ui.add(egui::TextEdit::singleline(sought).hint_text(format!("+ {hint}")).desired_width(220.0));
    if sought.trim().is_empty() {
        return;
    }
    let wanted = plain(sought);
    let mut found: Vec<&String> = names.iter().filter(|name| plain(name).contains(&wanted)).collect();
    found.sort_by_key(|name| (!plain(name).starts_with(&wanted), name.len()));
    let mut chosen = None;
    if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
        chosen = found.first().map(|name| (*name).clone());
    }
    ui.horizontal_wrapped(|ui| {
        for name in found.iter().take(12) {
            if ui.small_button(name.as_str()).clicked() {
                chosen = Some((*name).clone());
            }
        }
        if found.is_empty() {
            ui.label(theme::text("nothing by that name", theme::SMALL, theme::FAINT));
        }
    });
    if let Some(name) = chosen {
        take(name);
        sought.clear();
    }
}

/// A species' portrait, read once.
pub(super) fn portrait(context: &egui::Context, data: &str, kept: &mut HashMap<u16, Option<TextureHandle>>, species: u16) -> Option<TextureHandle> {
    kept.entry(species)
        .or_insert_with(|| {
            let path = assets::data::path(data, &format!("portraits/{species}_0.png"));
            let picture = image::open(path).ok()?.to_rgba8();
            let image = egui::ColorImage::from_rgba_unmultiplied([picture.width() as usize, picture.height() as usize], picture.as_raw());
            Some(context.load_texture(format!("portrait {species}"), image, egui::TextureOptions::NEAREST))
        })
        .clone()
}

/// Where the team chosen for the trials is kept, from one dungeon to the next.
fn team_file(root: &std::path::Path) -> std::path::PathBuf {
    root.join("target/workshop/trial_team.txt")
}

/// The team chosen last for a trial (`25 397 5`: who leads, who follows, the level), if any.
pub(super) fn kept_team(root: &std::path::Path) -> Option<(u16, u16, i32)> {
    let text = std::fs::read_to_string(team_file(root)).ok()?;
    let mut words = text.split_whitespace();
    let (hero, partner) = (words.next()?.parse().ok()?, words.next()?.parse().ok()?);
    Some((hero, partner, words.next().and_then(|level| level.parse().ok()).unwrap_or(5)))
}

/// The team of the trial: who leads, who follows, each with its portrait,
/// and its level; a change kept, and played at the next trial.
pub(super) fn team_choice(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    let Some(offered) = offered(&making.session) else {
        return;
    };
    let (mut hero, mut partner, mut level) = making.team.unwrap_or((offered.team.0, offered.team.1, 5));
    let before = (hero, partner, level);
    let fixed = making.form.team_level;
    ui.label(theme::text("TRIAL TEAM", theme::SMALL, theme::WEAK));
    ui.horizontal(|ui| {
        for (said, chosen) in [("leads", &mut hero), ("follows", &mut partner)] {
            if let Some(texture) = portrait(ui.ctx(), making.session.data(), &mut making.portraits, *chosen) {
                ui.add(egui::Image::new(&texture).fit_to_exact_size(vec2(28.0, 28.0)));
            }
            let name = offered.able.iter().find(|(id, _)| id == chosen).map_or_else(|| offered.book.species_name(*chosen), |(_, name)| name.clone());
            egui::ComboBox::from_id_salt(("trial team", said)).width(100.0).selected_text(name).height(420.0).show_ui(ui, |ui| {
                for (id, name) in &offered.able {
                    ui.selectable_value(chosen, *id, name);
                }
            });
            ui.label(theme::text(said, theme::SMALL, theme::FAINT));
        }
    });
    ui.horizontal(|ui| match fixed {
        Some(fixed) => _ = ui.label(theme::text(format!("level {fixed} (the dungeon's)"), theme::SMALL, theme::WEAK)),
        None => _ = ui.add(egui::DragValue::new(&mut level).range(1..=100).prefix("level ")),
    })
    .response
    .on_hover_text("any Pokémon the game starts with or its dungeons meet; its moves: those it knows at that level; played at the next trial");
    if (hero, partner, level) != before {
        making.team = Some((hero, partner, level));
        let file = team_file(making.session.root());
        if let Some(folder) = file.parent() {
            _ = std::fs::create_dir_all(folder);
        }
        _ = std::fs::write(&file, format!("{hero} {partner} {level}\n"));
        if making.trial.is_some() {
            making.said = Some("the team changes at the next trial (■ then ▶)".to_string());
        }
    }
}
