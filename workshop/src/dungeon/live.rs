//! The fight seen live: beside the arena being made, the fight as the game
//! draws it (its look, the boss, its guards, the team, what lies there),
//! made again a moment after each change, played alone with no key.

use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Rect, pos2, vec2};
use shell::Screen;

use super::{DungeonMaking, offered, theme};

/// How long the form stays the same before it is drawn again.
const SETTLED: Duration = Duration::from_millis(350);

/// The fight drawn live: what it is asked from (the fight's text, the
/// boss's life, the team), when that last changed, what it was made from;
/// the dungeon played and its picture.
pub(super) struct Live {
    asked: String,
    changed: Instant,
    made: String,
    diving: Option<game::dive::Diving>,
    screen: Screen,
    texture: Option<egui::TextureHandle>,
    last: Instant,
    wrong: Option<String>,
}

impl Live {
    pub(super) fn new() -> Live {
        let now = Instant::now();
        Live { asked: String::new(), changed: now, made: String::new(), diving: None, screen: Screen::new(2), texture: None, last: now, wrong: None }
    }
}

/// The fight made again from the form once a change has settled (the old
/// picture kept meanwhile).
fn remade(making: &mut DungeonMaking) {
    let text = making.played_text(true);
    let team = offered(&making.session).map(|offered| making.team.unwrap_or((offered.team.0, offered.team.1, 5)));
    let asked = format!("{text}\n{}\n{team:?}", making.boss_life);
    let live = making.live.get_or_insert_with(Live::new);
    if asked != live.asked {
        live.asked = asked;
        live.changed = Instant::now();
    }
    if live.made == live.asked || live.changed.elapsed() < SETTLED {
        return;
    }
    live.made = live.asked.clone();
    let Some((hero, partner, level)) = team else {
        live.wrong = Some("the game's data is missing (extractor)".to_string());
        return;
    };
    let team = game::dive::Team { hero, partner, level: if making.form.team_level.is_some() { None } else { Some(level) } };
    // (Silent: the live picture lends no sound.)
    match game::dive::Diving::of_text(making.session.data(), &making.project, &text, &making.word, team, &game::dive::Carried::default()) {
        Ok(mut diving) => {
            if making.boss_life < 100 {
                diving.boss_at(making.boss_life);
            }
            live.diving = Some(diving);
            live.wrong = None;
            live.last = Instant::now();
        }
        Err(why) => live.wrong = Some(why.to_string()),
    }
}

/// The live picture, as wide as the column, under what is above it.
pub(super) fn live_view(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    remade(making);
    let Some(live) = making.live.as_mut() else {
        return;
    };
    if let Some(why) = &live.wrong {
        ui.label(theme::text(why.as_str(), theme::SMALL, theme::WRONG));
    }
    let Some(diving) = live.diving.as_mut() else {
        ui.label(theme::text("drawing it…", theme::SMALL, theme::WEAK));
        ui.ctx().request_repaint_after(SETTLED);
        return;
    };
    let frames = live.last.elapsed().as_secs_f32() * 60.0;
    live.last = Instant::now();
    diving.play(&shell::Keys::default(), frames.min(10.0));
    diving.draw(&mut live.screen);
    let image = egui::ColorImage::from_rgba_unmultiplied([live.screen.wide, live.screen.high], &live.screen.rgba);
    match &mut live.texture {
        Some(texture) => texture.set(image, egui::TextureOptions::NEAREST),
        None => live.texture = Some(ui.ctx().load_texture("live fight", image, egui::TextureOptions::NEAREST)),
    }
    let (wide, high) = (live.screen.wide as f32, live.screen.high as f32);
    let scale = (ui.available_width() / wide).min((ui.available_height() - 40.0).max(120.0) / high).max(0.25);
    let (rect, _) = ui.allocate_exact_size(vec2(wide * scale, high * scale), egui::Sense::hover());
    if let Some(texture) = &live.texture {
        ui.painter().image(texture.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0_f32, theme::GAP), egui::StrokeKind::Outside);
    ui.label(theme::text(
        "live: the fight as the game draws it, made again after each change · ▶ Try the Fight to play it",
        theme::SMALL - 1.0,
        theme::FAINT,
    ));
    ui.ctx().request_repaint();
}
