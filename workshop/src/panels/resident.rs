//! Someone who lives in the decor, in the Inspector: which Pokémon, the way
//! he faces, his wandering, what he says; the game's life of him (there
//! only if, talked to, his `on talk`); his animation.

use eframe::egui::{self, Color32, Sense, pos2, vec2};

use super::cross_button;
use super::fields::{rule_field, typed};
use super::inspector::{component, prop, reset};
use super::landmark::{facing_and_wander, with_facing_and_wander};
use super::piece::Changing;
use crate::decors::Decors;
use crate::edit;
use crate::theme;

/// What is changed of his life beside his pose: his way, how far he
/// wanders, his lines.
struct Life {
    facing: usize,
    wander: i64,
    said: Vec<String>,
}

/// The resident's components; what they changed, in his pose or the text.
pub(super) fn components(ui: &mut egui::Ui, decors: &mut Decors, changing: &mut Changing) {
    let (facing, wander) = facing_and_wander(&changing.made_pose.rest);
    let said = edit::speech(&changing.text, changing.number);
    let mut life = Life { facing, wander, said: said.clone() };
    resident(ui, decors, changing, &mut life);
    in_the_game(ui, decors, changing);
    animation(ui, decors, changing, life.facing);
    if (life.facing, life.wander) != (facing, wander) {
        changing.made_pose.rest = with_facing_and_wander(&changing.made_pose.rest, life.facing, life.wander);
    }
    if life.said != said {
        changing.made = Some(edit::with_speech(&changing.text, changing.number, &life.said));
    }
}

/// Which Pokémon, his way, his wandering, what he says.
fn resident(ui: &mut egui::Ui, decors: &mut Decors, changing: &mut Changing, life: &mut Life) {
    component(ui, decors, "Resident", |ui, _| {
        prop(ui, "Pokémon", |ui| {
            let mut word = changing.made_pose.name.clone();
            let field = ui.add(egui::TextEdit::singleline(&mut word).desired_width(120.0));
            if field.lost_focus() && !word.trim().is_empty() {
                changing.made_pose.name = word.trim().to_string();
            }
        });
        prop(ui, "Facing", |ui| rose(ui, &mut life.facing));
        prop(ui, "Wandering", |ui| {
            ui.add(egui::DragValue::new(&mut life.wander).range(0..=400).suffix(" px"))
                .on_hover_text("how many pixels from its place it wanders (0: it stays there)");
            if reset(ui, life.wander > 0) {
                life.wander = 0;
            }
        });
        ui.add_space(4.0);
        ui.label(theme::weak("What it says (one after another, when spoken to; “only if”: while that test holds)"));
        speech(ui, &mut life.said);
    });
}

/// A rose of the eight ways: the one he faces, filled.
fn rose(ui: &mut egui::Ui, facing: &mut usize) {
    let (rose, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
    for way in 0..8 {
        let angle = std::f32::consts::FRAC_PI_2 - way as f32 * std::f32::consts::FRAC_PI_4;
        let centre = rose.center() + vec2(angle.cos(), angle.sin()) * 25.0;
        let square = egui::Rect::from_center_size(centre, vec2(20.0, 20.0));
        if way == *facing {
            ui.painter().rect_filled(square, 3.0, theme::DECOR.gamma_multiply(0.5));
        }
        if crate::icons::button(ui, square, decor::lang::FACING[way], |painter, at, tint| crate::icons::way(painter, at, way, tint)) {
            *facing = way;
        }
    }
    ui.painter().circle_filled(rose.center(), 3.0, theme::WEAK);
    ui.label(theme::weak(decor::lang::FACING[*facing]));
}

/// His lines, each with its test; one taken out, one more.
fn speech(ui: &mut egui::Ui, said: &mut Vec<String>) {
    let mut gone: Option<usize> = None;
    for (at, line) in said.iter_mut().enumerate() {
        let was = edit::said_test(line);
        let (mut test, mut words) = was.clone();
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut test).hint_text("only if…").desired_width(90.0))
                .on_hover_text("a test (not met_kecleon, chapter >= 2): the line is said only while it holds; empty: always");
            ui.add(egui::TextEdit::singleline(&mut words).desired_width(ui.available_width() - 36.0));
            if cross_button(ui, "remove this line") {
                gone = Some(at);
            }
        });
        if (test.clone(), words.clone()) != was {
            *line = edit::said_with_test(&test, &words);
        }
    }
    if let Some(at) = gone {
        said.remove(at);
    }
    if ui.small_button("+ a line").clicked() {
        said.push("...".to_string());
    }
}

/// The word a talk opens with, shown in its list (none: he only speaks).
pub(super) fn talk_word(talk: Option<&str>) -> &str {
    talk.map_or("only speaks", |talk| talk.split_whitespace().next().unwrap_or(talk))
}

/// The game's life of him: there only if a test holds, what talking to him opens, his `on talk`.
fn in_the_game(ui: &mut egui::Ui, decors: &mut Decors, changing: &mut Changing) {
    let (text, number, made) = (&changing.text, changing.number, &mut changing.made);
    let line = text.lines().nth(number - 1).unwrap_or_default().to_string();
    let talk = edit::talk(text, number);
    let name = changing.made_pose.name.clone();
    let library = decors.session.library().clone();
    component(ui, decors, "In the Game", |ui, _| {
        prop(ui, "There Only If", |ui| {
            let now = edit::condition(&line).unwrap_or_default();
            if let Some(test) = typed(ui, egui::Id::new(("resident if", number)), &now, 1, "always (chapter >= 2, not met…)") {
                *made = Some(edit::with_lines(text, &[(number, edit::with_condition(&line, Some(&test)))]));
            }
        });
        prop(ui, "Talked To", |ui| talked_to(ui, text, number, talk.as_deref(), made));
        if let Some(film) = talk.as_deref().and_then(|talk| talk.strip_prefix("film ")) {
            prop(ui, "The Film", |ui| {
                let mut asked: Option<String> = None;
                egui::ComboBox::from_id_salt(("talk film", number)).selected_text(film).show_ui(ui, |ui| {
                    for known in crate::game::films(&library) {
                        if ui.selectable_label(known == film, &known).clicked() {
                            asked = Some(known);
                        }
                    }
                });
                if let Some(asked) = asked {
                    *made = Some(edit::with_talk(text, number, Some(&format!("film {asked}"))));
                }
            });
        }
        let head = format!("on talk {name}");
        let body = edit::life_rules(text).into_iter().find(|rule| edit::rule_key(&rule.head) == head).map(|rule| rule.body).unwrap_or_default();
        ui.label(theme::text(format!("{head} … end: what the game does after what it says"), theme::SMALL, theme::FAINT));
        if let Some(body) = rule_field(ui, &head, &body) {
            *made = Some(edit::with_rule(text, &head, &body));
        }
    });
}

/// What talking to him does: he only speaks, plays a film, opens a screen.
fn talked_to(ui: &mut egui::Ui, text: &str, number: usize, talk: Option<&str>, made: &mut Option<String>) {
    let word = talk_word(talk);
    let mut asked: Option<Option<String>> = None;
    egui::ComboBox::from_id_salt(("talk", number)).selected_text(word).show_ui(ui, |ui| {
        if ui.selectable_label(talk.is_none(), "only speaks").clicked() {
            asked = Some(None);
        }
        if ui.selectable_label(word == "film", "plays a film").clicked() && word != "film" {
            asked = Some(Some("film a_film".to_string()));
        }
        for screen in &decor::lang::TALKS[1..] {
            if ui.selectable_label(word == *screen, format!("opens the {screen}")).clicked() {
                asked = Some(Some(screen.to_string()));
            }
        }
    });
    if let Some(asked) = asked {
        *made = Some(edit::with_talk(text, number, asked.as_deref()));
    }
}

/// What he does: each animation his sprite has, living, the way he faces; held or over and over.
fn animation(ui: &mut egui::Ui, decors: &mut Decors, changing: &mut Changing, facing: usize) {
    let word = changing.made_pose.name.clone();
    let made_pose = &mut changing.made_pose;
    component(ui, decors, "Animation", |ui, decors| {
        let Some(anims) = decors.anims_now(&word) else {
            ui.label(theme::weak("its animations are being read…"));
            return;
        };
        let now = made_pose.animation.as_deref().and_then(|name| scenes::vocabulary::POKEMON.animation(name)).and_then(|group| u8::try_from(group).ok());
        ui.horizontal(|ui| {
            ui.label(theme::weak("Plays"));
            if ui.selectable_label(!made_pose.still, "looping").on_hover_text("its animation without end").clicked() {
                made_pose.still = false;
            }
            if ui.selectable_label(made_pose.still, "still").on_hover_text("motionless on its first image (lying down, asleep…)").clicked() {
                made_pose.still = true;
            }
            if made_pose.animation.is_some() && ui.small_button("as it idles").on_hover_text("no animation: it idles, as the game does").clicked() {
                made_pose.animation = None;
                made_pose.still = false;
            }
        });
        ui.add_space(4.0);
        let seconds = ui.input(|input| input.time);
        let cards: Vec<(Option<u8>, String, String)> = std::iter::once((None, "default".to_string(), String::new()))
            .chain(anims.iter().map(|(group, name, own)| (Some(*group), name.clone(), own.clone())))
            .collect();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (group, name, own) in cards {
                let picked = group == now;
                let moment = if picked && made_pose.still { 0.0 } else { seconds };
                if card(ui, decors, &word, group, &name, &own, picked, facing, moment) {
                    made_pose.animation = (!own.is_empty()).then_some(own);
                }
            }
        });
        ui.ctx().request_repaint();
    });
}

/// An animation's card: him playing it, its name; whether it is clicked.
fn card(ui: &mut egui::Ui, decors: &mut Decors, word: &str, group: Option<u8>, name: &str, own: &str, picked: bool, facing: usize, moment: f64) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(66.0, 80.0), Sense::click());
    let fill = if picked {
        theme::CHOSEN
    } else if response.hovered() {
        theme::HOVERED
    } else {
        theme::FIELD
    };
    ui.painter().rect_filled(rect, 3.0, fill);
    let key = group.map_or_else(|| word.to_string(), |group| format!("{word}#{group}"));
    let square = egui::Rect::from_min_size(rect.min + vec2(3.0, 3.0), vec2(60.0, 58.0));
    if ui.is_rect_visible(rect)
        && let Some(body) = decors.body_now(&key)
        && let Some((texture, around)) = body.at(facing, moment)
    {
        // (Drawn whole in its square, his feet low in it.)
        let size = around.size();
        let scale = (square.width() / size.x).min(square.height() / size.y).min(2.0);
        let at = egui::Rect::from_center_size(square.center(), size * scale);
        ui.painter().with_clip_rect(square).image(texture.id(), at, egui::Rect::from_min_max(egui::Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
    }
    let clip = ui.painter().with_clip_rect(rect);
    clip.text(
        pos2(rect.center().x, rect.max.y - 9.0),
        egui::Align2::CENTER_CENTER,
        name,
        egui::FontId::proportional(theme::SMALL - 1.0),
        if picked { theme::STRONG } else { theme::TEXT },
    );
    let said = if own.is_empty() { "as the game makes it idle".to_string() } else { format!("{name} ({own})") };
    response.on_hover_text(said).clicked()
}
