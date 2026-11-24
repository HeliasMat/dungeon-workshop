//! A boss fight's three columns: the boss (its portrait and numbers, its
//! guards, the team, its allies, its moves, its drop); the arena, or the
//! fight tried; its staging (its entrance, its music, its phases on a strip
//! of its life).

use super::arena::{arena_drawn, arena_tools, making_arena};
use super::choosers::{chooser, portrait, sound_choice};
use super::dungeon_columns::song_choice;
use super::editor::{checks, fight_head, view};
use super::form::SHAPES;
use super::layout::{Fighter, Numbers, hint, numbers, section};
use super::*;
use workshop_base::session::Session;

/// The game's id of a species named so (none: it is not in the game).
pub(super) fn species_id(session: &Session, name: &str) -> Option<u16> {
    offered(session).and_then(|offered| offered.species.iter().find(|(_, known)| plain(known) == plain(name)).map(|(id, _)| *id))
}

/// A species' portrait, so large (an empty square if it has none).
pub(super) fn portrait_of(ui: &mut egui::Ui, session: &Session, portraits: &mut HashMap<u16, Option<TextureHandle>>, name: &str, size: f32) {
    match species_id(session, name).and_then(|id| portrait(ui.ctx(), session.data(), portraits, id)) {
        Some(texture) => _ = ui.add(egui::Image::new(&texture).fit_to_exact_size(vec2(size, size))),
        None => _ = ui.allocate_exact_size(vec2(size, size), Sense::hover()),
    }
}

/// The share of HP a new phase comes at: half the lowest one's (5 at least), else half.
pub(super) fn next_phase_share(phases: &[PhaseForm]) -> i32 {
    phases.iter().map(|phase| phase.share).min().map_or(50, |lowest| (lowest / 2).max(5))
}

/// The share of its HP from which the boss is angry (the highest phase that makes it fiercer).
pub(super) fn first_anger(phases: &[PhaseForm]) -> Option<i32> {
    phases.iter().filter(|phase| phase.fiercer > 0).map(|phase| phase.share).max()
}

/// The boss: its portrait, species, level, life; its numbers against the
/// team's and how hard it looks; its guards; the team; its allies; its moves; what it lets fall.
pub(super) fn boss_column(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    let session = making.session.clone();
    let kept = offered(&session);
    let offered = kept.as_deref();
    let species_names: Vec<String> = offered.map(|offered| offered.species.iter().map(|(_, name)| name.clone()).collect()).unwrap_or_default();
    let counted = numbers(making);
    let Some(boss) = making.form.boss.as_mut() else {
        return;
    };
    let exported = boss_head(ui, &session, boss, &mut making.portraits, &species_names);
    if let Some(said) = exported {
        making.said = Some(said);
    }
    section(ui, "ITS STRENGTH AND THE TEAM'S");
    match &counted {
        Some(counted) => strengths(ui, counted),
        None => hint(ui, "its numbers will come when the fight reads without a fault"),
    }
    section(ui, "ITS GUARDS");
    hint(ui, "six at most, each in its place in the arena (orange)");
    placed_list(ui, &session, &mut boss.guards, &mut making.portraits, None, "first place");
    if boss.guards.len() < 6 {
        let level = boss.level.saturating_sub(3).max(1);
        chooser(ui, "a guard…", &mut making.sought_guard, species_names.clone(), |species| boss.guards.push((species, level, None)));
    }
    fight_team(ui, &session, boss, &mut making.portraits);
    section(ui, "ITS ALLIES");
    hint(ui, "they fight on the team's side, near it or on their tile (turquoise in the arena); six at most");
    placed_list(ui, &session, &mut boss.allies, &mut making.portraits, Some(ALLY), "near the team");
    if boss.allies.len() < 6 {
        let level = boss.team_level.unwrap_or(boss.level);
        chooser(ui, "an ally…", &mut making.sought_ally, species_names.clone(), |species| boss.allies.push((species, level, None)));
    }
    boss_moves(ui, &session, boss, &mut making.sought_move);
    boss_drop(ui, &session, boss);
}

/// The boss's head: its portrait; its species, level, HP, whether it joins;
/// its progression exported (what was said of it given back).
fn boss_head(
    ui: &mut egui::Ui,
    session: &Session,
    boss: &mut BossForm,
    portraits: &mut HashMap<u16, Option<TextureHandle>>,
    species_names: &[String],
) -> Option<String> {
    let mut exported: Option<String> = None;
    ui.label(RichText::new("THE BOSS").size(theme::SMALL).color(theme::WEAK).strong());
    ui.horizontal(|ui| {
        portrait_of(ui, session, portraits, &boss.species, 96.0);
        ui.vertical(|ui| {
            egui::ComboBox::from_id_salt("the boss").width(190.0).selected_text(RichText::new(boss.species.as_str()).strong()).height(420.0).show_ui(
                ui,
                |ui| {
                    for name in species_names {
                        if ui.selectable_label(plain(name) == plain(&boss.species), name).clicked() {
                            boss.species = name.clone();
                        }
                    }
                },
            );
            ui.add(egui::DragValue::new(&mut boss.level).range(1..=100).prefix("level "));
            ui.add(egui::DragValue::new(&mut boss.life).range(0.5..=20.0).speed(0.1).prefix("HP ×"))
                .on_hover_text("its HP times this much: a boss lasts longer than a Pokémon of the dungeon");
            ui.checkbox(&mut boss.joins, "beaten, it joins the team").on_hover_text("it asks to join, if the dungeon allows recruiting");
            if ui.small_button("Export as CSV…").on_hover_text("its progression: exp., stats and moves learned at each level").clicked() {
                exported = Some(match species_id(session, &boss.species) {
                    Some(id) => crate::level_csv::export(session, i64::from(id), &boss.species),
                    None => format!("{}: not in the game's tables", boss.species),
                });
            }
        });
    });
    exported
}

/// Its numbers, the team's, and how hard it looks.
fn strengths(ui: &mut egui::Ui, counted: &Numbers) {
    egui::Grid::new("fight strengths").num_columns(7).spacing(vec2(10.0, 3.0)).striped(true).show(ui, |ui| {
        for head in ["", "Lv.", "HP", "Atk", "Def", "SpA", "SpD"] {
            ui.label(theme::text(head, theme::SMALL - 1.0, theme::FAINT));
        }
        ui.end_row();
        let row = |ui: &mut egui::Ui, one: &Fighter, colour: Color32| {
            ui.label(RichText::new(&one.name).size(theme::SMALL).color(colour));
            for value in [one.level, one.hp, one.attack, one.defense, one.sp_attack, one.sp_defense] {
                ui.label(theme::text(value.to_string(), theme::SMALL, theme::TEXT));
            }
            ui.end_row();
        };
        row(ui, &counted.boss, Color32::from_rgb(0xE5, 0x50, 0x50));
        for guard in &counted.guards {
            row(ui, guard, Color32::from_rgb(0xF0, 0x98, 0x30));
        }
        for one in &counted.team {
            row(ui, one, Color32::from_rgb(0x4A, 0xDE, 0x80));
        }
        for one in &counted.allies {
            row(ui, one, ALLY);
        }
    });
    if counted.help > 0 {
        hint(ui, &format!("and {} reinforcement{} in its phases", counted.help, if counted.help > 1 { "s" } else { "" }));
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(theme::text("at a guess:", theme::SMALL, theme::WEAK));
        ui.label(RichText::new(counted.verdict.0).strong().color(counted.verdict.1));
    })
    .response
    .on_hover_text(format!(
        "the time for the team to bring down the boss and its guards, against the time they need to bring the team down: {:.2} (under 0.4 easy, under 0.8 balanced, under 1.3 hard); without the bag's items, the moves or the types",
        counted.verdict.2
    ));
}

/// Those placed in the arena (guards, allies): each one's portrait, name (in
/// that colour), level, tile (or where it goes without one); × removes it.
fn placed_list(
    ui: &mut egui::Ui,
    session: &Session,
    placed: &mut Vec<(String, i32, Option<Tile>)>,
    portraits: &mut HashMap<u16, Option<TextureHandle>>,
    colour: Option<Color32>,
    unplaced: &str,
) {
    let mut gone = None;
    for (at, (species, level, place)) in placed.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            portrait_of(ui, session, portraits, species, 32.0);
            let name = RichText::new(species.as_str()).strong();
            ui.label(match colour {
                Some(colour) => name.color(colour),
                None => name,
            });
            ui.add(egui::DragValue::new(level).range(1..=100).prefix("Lv. "));
            ui.label(theme::text(place.map_or(unplaced.to_string(), |(x, y)| format!("tile {x},{y}")), theme::SMALL - 1.0, theme::FAINT));
            if ui.small_button("×").clicked() {
                gone = Some(at);
            }
        });
    }
    if let Some(at) = gone {
        placed.remove(at);
    }
}

/// The team that fights it, played alone: who leads, who follows, at a given level or not.
fn fight_team(ui: &mut egui::Ui, session: &Session, boss: &mut BossForm, portraits: &mut HashMap<u16, Option<TextureHandle>>) {
    section(ui, "THE TEAM THAT FIGHTS IT");
    hint(ui, "played alone (the trial, “dungeon <word>” in a film), the fight is played by this team; at the end of a dungeon, by the one that came");
    let able: Vec<String> = offered(session).map(|offered| offered.able.iter().map(|(_, name)| name.clone()).collect()).unwrap_or_default();
    for (said, chosen, none) in [("leads", &mut boss.hero, "the one that comes"), ("follows", &mut boss.partner, "the one that comes")] {
        ui.horizontal(|ui| {
            portrait_of(ui, session, portraits, chosen, 28.0);
            egui::ComboBox::from_id_salt(("fight team", said))
                .width(170.0)
                .selected_text(if chosen.is_empty() { none } else { chosen.as_str() })
                .height(420.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(chosen.is_empty(), none).clicked() {
                        chosen.clear();
                    }
                    for name in &able {
                        if ui.selectable_label(plain(name) == plain(chosen), name).clicked() {
                            *chosen = name.clone();
                        }
                    }
                });
            ui.label(theme::text(said, theme::SMALL, theme::FAINT));
        });
    }
    ui.horizontal(|ui| {
        let mut said = boss.team_level.is_some();
        if ui.checkbox(&mut said, "at a given level").on_hover_text("without: the level it has (the trial's, or the film's)").changed() {
            boss.team_level = said.then_some(boss.level);
        }
        if let Some(level) = boss.team_level.as_mut() {
            ui.add(egui::DragValue::new(level).range(1..=100).prefix("level "));
        }
    });
}

/// Its moves: four at most (those the game does not know, in red), one removed by a click, one more chosen.
fn boss_moves(ui: &mut egui::Ui, session: &Session, boss: &mut BossForm, sought: &mut String) {
    let kept = offered(session);
    let offered = kept.as_deref();
    section(ui, "ITS MOVES");
    hint(ui, "four at most; without: its species' at its level; a phase can change them");
    let mut gone = None;
    ui.horizontal_wrapped(|ui| {
        for (at, said) in boss.moves.iter().enumerate() {
            let known = offered.is_none_or(|offered| offered.moves.iter().any(|name| plain(name) == plain(said)));
            if ui.button(RichText::new(format!("{said}  ×")).color(if known { theme::TEXT } else { theme::WRONG })).on_hover_text("remove it").clicked() {
                gone = Some(at);
            }
        }
    });
    if let Some(at) = gone {
        boss.moves.remove(at);
    }
    if boss.moves.len() < 4 {
        let moves = offered.map(|offered| offered.moves.clone()).unwrap_or_default();
        chooser(ui, "a move…", sought, moves, |name| boss.moves.push(name));
    }
}

/// What it lets fall, beaten (it holds it during the fight).
fn boss_drop(ui: &mut egui::Ui, session: &Session, boss: &mut BossForm) {
    section(ui, "WHAT IT DROPS, BEATEN");
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("its drop")
            .width(220.0)
            .selected_text(if boss.drop.is_empty() { "nothing" } else { boss.drop.as_str() })
            .height(360.0)
            .show_ui(ui, |ui| {
                if ui.selectable_label(boss.drop.is_empty(), "nothing").clicked() {
                    boss.drop.clear();
                }
                for name in offered(session).map(|offered| offered.items.clone()).unwrap_or_default() {
                    if ui.selectable_label(plain(&name) == plain(&boss.drop), name.as_str()).clicked() {
                        boss.drop = name.clone();
                    }
                }
            });
    })
    .response
    .on_hover_text("it holds it during the fight and drops it, beaten");
}

/// The middle: the arena, large (everyone placed, the ground painted), or the fight tried.
pub(super) fn arena_column(ui: &mut egui::Ui, making: &mut DungeonMaking, audio: &mut Option<Box<dyn Hears>>) {
    let session = making.session.clone();
    if making.trial.is_some() {
        making.trying_shown = true;
    }
    ui.horizontal(|ui| {
        if ui.selectable_label(!making.trying_shown, RichText::new("The Arena").strong()).clicked() {
            making.trying_shown = false;
        }
        if ui.selectable_label(making.trying_shown, RichText::new("The Trial").strong()).clicked() {
            making.trying_shown = true;
        }
        ui.add_space(12.0);
        if ui.button(RichText::new("▶ Try the Fight").strong().color(theme::ASSETS)).on_hover_text("the arena right away, with the trial team").clicked() {
            making.trying_shown = true;
            making.try_it(audio, true);
        }
        ui.add(egui::DragValue::new(&mut making.boss_life).range(1..=100).prefix("the boss at ").suffix(" % of its HP"))
            .on_hover_text("to see right away what it does as its HP drops: its phases");
    });
    ui.separator();
    if making.trying_shown {
        view(ui, making, audio);
        return;
    }
    let kept = offered(&session);
    let offered = kept.as_deref();
    let Some(boss) = making.form.boss.as_mut() else {
        return;
    };
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut boss.room.0).range(7..=40).prefix("width "));
        ui.add(egui::DragValue::new(&mut boss.room.1).range(6..=24).prefix("height "));
        egui::ComboBox::from_id_salt("arena shape")
            .selected_text(SHAPES.iter().find(|(word, _)| *word == boss.shape).map_or(boss.shape.as_str(), |(_, said)| *said))
            .show_ui(ui, |ui| {
                for (word, said) in SHAPES {
                    if ui.selectable_label(boss.shape == word, said).clicked() {
                        boss.shape = word.to_string();
                    }
                }
            })
            .response
            .on_hover_text("the starting shape; once painted by hand, it counts less");
        if boss.map.is_some() && ui.small_button("clear the paint").on_hover_text("go back to the starting shape").clicked() {
            boss.map = None;
        }
    });
    arena_tools(ui, making_arena(&mut making.arena_tool, &mut making.arena_item, &mut making.arena_trap, &mut making.trap_hidden), boss, offered);
    arena_drawn(ui, boss, making.arena_tool, (&making.arena_item, &making.arena_trap, making.trap_hidden));
}

/// The right: its name and look, its title and cry, its words, its music;
/// its phases on a strip of its life; what is wrong; its file.
pub(super) fn staging_column(ui: &mut egui::Ui, making: &mut DungeonMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) {
    let session = making.session.clone();
    fight_head(ui, making);
    let sounds = shell::own_sounds::names_in(&session.library().sounds());
    let heard = &mut making.heard;
    let Some(boss) = making.form.boss.as_mut() else {
        return;
    };
    section(ui, "ITS ENTRANCE");
    ui.horizontal(|ui| {
        ui.label(theme::text("title", theme::SMALL, theme::WEAK));
        ui.add(egui::TextEdit::singleline(&mut boss.title).hint_text("The Ancient of the Deep").desired_width(f32::INFINITY))
            .on_hover_text("under its name, in the banner of its entrance");
    });
    ui.horizontal(|ui| {
        ui.label(theme::text("cry", theme::SMALL, theme::WEAK));
        sound_choice(ui, ("entrance cry", 0), &mut boss.cry, &sounds);
        hint(ui, "with a cry, the screen shakes");
    });
    for (said, line, example) in [("it says", &mut boss.begins, "You shall go no further!"), ("falling", &mut boss.falls, "I… was wrong about you…")] {
        ui.horizontal(|ui| {
            ui.add_sized(vec2(64.0, 18.0), egui::Label::new(theme::text(said, theme::SMALL, theme::WEAK)));
            ui.add(egui::TextEdit::singleline(line).hint_text(example).desired_width(f32::INFINITY));
        });
    }
    section(ui, "ITS MUSIC");
    song_choice(ui, "boss music", &mut boss.music, "the dungeon's", true, heard, audio, volume, &session);
    section(ui, "ITS PHASES");
    hint(ui, "each once, when its HP drops under its share: a white flash, and what is put in it");
    life_strip(ui, boss);
    phases_cards(ui, &session, boss, &mut making.phase_sought, &sounds);
    ui.add_space(6.0);
    checks(ui, making);
}

/// Its life as a strip, from full (left) to none (right), each phase marked where it comes.
fn life_strip(ui: &mut egui::Ui, boss: &BossForm) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let bar = Rect::from_min_size(rect.min + vec2(0.0, 12.0), vec2(rect.width(), 12.0));
    painter.rect_filled(bar, 2.0, Color32::from_rgb(0x48, 0x18, 0x18));
    let fill = |from: f32, to: f32, colour: Color32| {
        let (left, right) = (bar.min.x + bar.width() * (1.0 - from), bar.min.x + bar.width() * (1.0 - to));
        painter.rect_filled(Rect::from_min_max(pos2(left, bar.min.y), pos2(right, bar.max.y)), 0.0, colour);
    };
    // (Red while it is calm, orange from its first anger on.)
    match first_anger(&boss.phases) {
        Some(share) => {
            fill(1.0, share as f32 / 100.0, Color32::from_rgb(0xE0, 0x30, 0x30));
            fill(share as f32 / 100.0, 0.0, Color32::from_rgb(0xF0, 0x80, 0x20));
        }
        None => fill(1.0, 0.0, Color32::from_rgb(0xE0, 0x30, 0x30)),
    }
    let mut order: Vec<(usize, &PhaseForm)> = boss.phases.iter().enumerate().collect();
    order.sort_by_key(|(_, phase)| std::cmp::Reverse(phase.share));
    for (at, phase) in &order {
        let x = bar.min.x + bar.width() * (1.0 - phase.share as f32 / 100.0);
        painter.vline(x, bar.min.y - 3.0..=bar.max.y + 3.0, egui::Stroke::new(2.0_f32, Color32::WHITE));
        painter.text(pos2(x, rect.min.y), egui::Align2::CENTER_TOP, format!("P{}", at + 1), egui::FontId::proportional(10.0), theme::TEXT);
        painter.text(pos2(x, bar.max.y + 1.0), egui::Align2::CENTER_TOP, format!("{}%", phase.share), egui::FontId::proportional(9.0), theme::FAINT);
    }
    response.on_hover_text("its HP, full on the left; each line: a phase, where it comes");
}

/// Its phases, a card each; one more.
fn phases_cards(ui: &mut egui::Ui, session: &Session, boss: &mut BossForm, sought: &mut Vec<(String, String)>, sounds: &[String]) {
    let species_names: Vec<String> = offered(session).map(|offered| offered.species.iter().map(|(_, name)| name.clone()).collect()).unwrap_or_default();
    sought.resize(boss.phases.len(), (String::new(), String::new()));
    let mut gone = None;
    let level = boss.level;
    for (at, phase) in boss.phases.iter_mut().enumerate() {
        if phase_card(ui, session, at, phase, &mut sought[at], level, &species_names, sounds) {
            gone = Some(at);
        }
    }
    if let Some(at) = gone {
        boss.phases.remove(at);
    }
    if ui.button("+ a phase").clicked() {
        boss.phases.push(PhaseForm { share: next_phase_share(&boss.phases), ..PhaseForm::default() });
    }
}

/// A phase's card: when it comes, what the boss says, its anger, its heal,
/// the weather, a cry, a shake; its reinforcements, its moves from then on. Whether it is to be removed.
fn phase_card(
    ui: &mut egui::Ui,
    session: &Session,
    at: usize,
    phase: &mut PhaseForm,
    sought: &mut (String, String),
    level: i32,
    species_names: &[String],
    sounds: &[String],
) -> bool {
    let (sought_help, sought_move) = sought;
    let weathers: Vec<&str> = rules::own::WEATHERS.iter().map(|(said, _)| *said).collect();
    let mut gone = false;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_max_width(ui.available_width() - 16.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Phase {}", at + 1)).strong().color(theme::ASSETS));
            ui.add(egui::Slider::new(&mut phase.share, 1..=99).suffix(" %").text("of its HP"));
            if ui.small_button("×").on_hover_text("remove this phase").clicked() {
                gone = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label(theme::text("it says", theme::SMALL, theme::WEAK));
            ui.add(egui::TextEdit::singleline(&mut phase.says).hint_text("Enough games!").desired_width(f32::INFINITY));
        });
        ui.horizontal_wrapped(|ui| {
            ui.add(egui::DragValue::new(&mut phase.fiercer).range(0..=6).prefix("anger +")).on_hover_text("its Attack and Sp. Atk rise by that much (0: no)");
            ui.add(egui::DragValue::new(&mut phase.heal).range(0..=99).prefix("heal +").suffix(" %")).on_hover_text("it gets back that much of its HP (0: no)");
            egui::ComboBox::from_id_salt(("phase weather", at))
                .width(90.0)
                .selected_text(if phase.weather.is_empty() { "weather —" } else { phase.weather.as_str() })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(phase.weather.is_empty(), "— (the same)").clicked() {
                        phase.weather.clear();
                    }
                    for said in &weathers {
                        if ui.selectable_label(phase.weather == *said, *said).clicked() {
                            phase.weather = said.to_string();
                        }
                    }
                });
            sound_choice(ui, ("phase cry", at), &mut phase.cry, sounds);
            ui.checkbox(&mut phase.shake, "shake");
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(theme::text("reinforcements", theme::SMALL, theme::WEAK));
            let mut left = None;
            for (one, (species, level)) in phase.help.iter_mut().enumerate() {
                ui.label(RichText::new(species.as_str()).strong());
                ui.add(egui::DragValue::new(level).range(1..=100).prefix("Lv. "));
                if ui.small_button("×").clicked() {
                    left = Some(one);
                }
            }
            if let Some(one) = left {
                phase.help.remove(one);
            }
        });
        if phase.help.len() < 6 {
            let level = level.saturating_sub(4).max(1);
            chooser(ui, "a reinforcement…", sought_help, species_names.to_vec(), |species| phase.help.push((species, level)));
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(theme::text("moves from then on", theme::SMALL, theme::WEAK));
            let mut left = None;
            for (one, said) in phase.moves.iter().enumerate() {
                if ui.small_button(format!("{said}  ×")).clicked() {
                    left = Some(one);
                }
            }
            if let Some(one) = left {
                phase.moves.remove(one);
            }
        });
        if phase.moves.len() < 4 {
            let moves = offered(session).map(|offered| offered.moves.clone()).unwrap_or_default();
            chooser(ui, "a move…", sought_move, moves, |name| phase.moves.push(name));
        }
    });
    gone
}
