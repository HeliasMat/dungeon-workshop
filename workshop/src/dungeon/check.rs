//! "Check the dungeon": what is wrong with the dungeon being made (the
//! problems themselves are found by `game::dungeon_check`, which the
//! check of a whole game shares). Each says where it is (a click shows it)
//! and, when the way is plain, is put right by a button.

pub(super) use game::dungeon_check::{Fix, Place, Problem};

use super::*;

/// How many problems, and how many grave.
pub(super) fn counted(making: &DungeonMaking) -> (usize, usize) {
    (making.checked.1.len(), making.checked.1.iter().filter(|problem| problem.grave()).count())
}

/// A range grown to take a floor in.
fn grown(floors: Floors, at: usize) -> Floors {
    floors.map(|(low, high)| (low.min(at), high.max(at)))
}

/// Everything wrong with the dungeon being made.
pub(super) fn problems(making: &DungeonMaking) -> Vec<Problem> {
    let form = &making.form;
    let dungeon = game::dungeon_check::Dungeon {
        name: form.name.clone(),
        floors: form.floors,
        look: form.look.clone(),
        look_floor: form.look_floor,
        music: form.music.clone(),
        fight: form.fight.clone(),
        boss: form.boss.is_some(),
        foes: form.foes.clone(),
        items: form.items.clone(),
    };
    let library = &making.project;
    let played = game::dive::resolved_for(library, &making.played_text(false));
    let kept = offered(&making.session);
    let offered = kept.as_deref();
    let tables = offered.map(|offered| (&offered.known, &offered.book));
    let fights = library.fight(&dungeon.fight).parent().map_or_else(|| library.fights(), std::path::Path::to_path_buf);
    game::dungeon_check::problems(&dungeon, tables, &played, &fights)
}

/// A problem put right.
fn fixed(session: &crate::session::Session, form: &mut Form, fix: Fix) {
    let count = form.floors.max(1);
    let cut = |floors: &mut Floors| *floors = floors.map(|(low, high)| (low.min(count), high.min(count)));
    match fix {
        Fix::OneFloor => form.floors = 1,
        Fix::LookFoes => {
            let Some(offered) = offered(session) else {
                return;
            };
            let Some(spec) = floor::look_floor(&offered, &form.look, form.look_floor) else {
                return;
            };
            for enemy in spec.enemies.iter().filter(|enemy| enemy.species > 0).take(6) {
                let species = offered.book.species_name(enemy.species);
                if !form.foes.iter().any(|foe| plain(&foe.species) == plain(&species)) {
                    form.foes.push(Foe { species, level: enemy.stats.level.clamp(1, 100), often: 1, floors: None });
                }
            }
        }
        Fix::FoeOn(rank, at) => {
            if let Some(foe) = form.foes.get_mut(rank) {
                foe.floors = grown(foe.floors, at);
            }
        }
        Fix::ItemOn(rank, at) => {
            if let Some(lying) = form.items.get_mut(rank) {
                lying.floors = grown(lying.floors, at);
            }
        }
        Fix::RemoveFoe(rank) if rank < form.foes.len() => _ = form.foes.remove(rank),
        Fix::RemoveItem(rank) if rank < form.items.len() => _ = form.items.remove(rank),
        Fix::FoeOften(rank) => form.foes.get_mut(rank).into_iter().for_each(|foe| foe.often = 1),
        Fix::ItemOften(rank) => form.items.get_mut(rank).into_iter().for_each(|lying| lying.often = 1),
        Fix::FoeLevel(rank) => form.foes.get_mut(rank).into_iter().for_each(|foe| foe.level = foe.level.clamp(1, 100)),
        Fix::FoeFloors(rank) => form.foes.get_mut(rank).into_iter().for_each(|foe| cut(&mut foe.floors)),
        Fix::ItemFloors(rank) => form.items.get_mut(rank).into_iter().for_each(|lying| cut(&mut lying.floors)),
        Fix::LookDefault => (form.look, form.look_floor) = ("Apple Woods".to_string(), 1),
        Fix::LookFloor(floor) => form.look_floor = floor,
        Fix::NoMusic => form.music.clear(),
        Fix::NoFight => form.fight.clear(),
        Fix::RemoveFoe(_) | Fix::RemoveItem(_) => {}
    }
}

/// Where a problem is, as its line says it.
fn where_said(place: Place, form: &Form) -> String {
    match place {
        Place::Name => "name".to_string(),
        Place::Floors => "floors".to_string(),
        Place::Look => "look".to_string(),
        Place::Music => "music".to_string(),
        Place::Fight => "fight".to_string(),
        Place::Foes => "Pokémon".to_string(),
        Place::Foe(rank) => form.foes.get(rank).map_or_else(|| "Pokémon".to_string(), |foe| foe.species.clone()),
        Place::Item(rank) => form.items.get(rank).map_or_else(|| "item".to_string(), |lying| lying.item.clone()),
        Place::Floor(at) => format!("floor {at}"),
        Place::File => "file".to_string(),
    }
}

/// A place shown, for a while after its problem was clicked: framed, and
/// scrolled to at first.
pub(super) fn shows(ui: &egui::Ui, pointed: &Option<(Place, Instant)>, place: Place, rect: Rect) {
    let Some((_, since)) = (*pointed).filter(|(at, _)| *at == place) else {
        return;
    };
    let gone = since.elapsed().as_secs_f32();
    if gone > 2.5 {
        return;
    }
    if gone < 0.3 {
        ui.scroll_to_rect(rect, Some(egui::Align::Center));
    }
    ui.painter().rect_stroke(rect.expand(2.0), 3.0, egui::Stroke::new(2.0_f32, theme::POINTED), egui::StrokeKind::Outside);
    ui.ctx().request_repaint();
}

/// The checker: each problem, where it is (a click shows it), its fix.
pub(super) fn checker(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    ui.add_space(8.0);
    let title = ui.label(theme::text("CHECK THE DUNGEON", theme::SMALL, theme::WEAK));
    if std::mem::take(&mut making.check_sought) {
        title.scroll_to_me(Some(egui::Align::Min));
    }
    if making.checked.1.is_empty() {
        ui.label(RichText::new("nothing to review: it plays").color(Color32::from_rgb(0x6C, 0xC6, 0x6C)));
        return;
    }
    let (mut asked, mut pointed) = (None, None);
    for problem in &making.checked.1 {
        ui.horizontal_wrapped(|ui| {
            let (mark, colour) = if problem.grave() { ("●", theme::WRONG) } else { ("▲", Color32::from_rgb(0xF0, 0xA8, 0x40)) };
            ui.label(RichText::new(mark).color(colour).size(theme::SMALL));
            let said = format!("{} · {}", where_said(problem.place, &making.form), problem.told.message);
            if ui.add(egui::Label::new(RichText::new(said).color(colour).size(theme::SMALL)).sense(Sense::click())).on_hover_text("show it").clicked() {
                pointed = Some(problem.place);
            }
            if let Some(fix) = problem.fix
                && ui.small_button("Fix").on_hover_text(fix.said()).clicked()
            {
                asked = Some(fix);
            }
        });
    }
    if let Some(place) = pointed {
        if let Place::Floor(at) = place {
            making.floor_shown.at = at;
        }
        making.pointed = Some((place, Instant::now()));
    }
    if let Some(fix) = asked {
        fixed(&making.session, &mut making.form, fix);
    }
}
