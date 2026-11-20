//! The dungeon and boss fight editors laid out on the whole window, in three
//! columns; the fight in numbers, and what the columns share (a section's
//! title, a hint, a column's panel).
//!
//! - A boss fight (`fight_columns.rs`): on the left the boss (its portrait,
//!   its numbers against the team's, how hard the fight looks), its guards,
//!   its moves, what it lets fall; in the middle the arena, drawn large,
//!   where everyone is placed and the ground painted, or the fight tried; on
//!   the right its staging (name, look, title, cry, words, music) and its
//!   phases on a strip of its life.
//! - A dungeon (`dungeon_columns.rs`): on the left what it is (name, floors,
//!   rules, look, music, the fight at its end); in the middle its floors as a
//!   grid (which Pokémon and which item on which floor, a click away; each
//!   floor's share); on the right the trial and its file.

use super::dungeon_columns::{dungeon_column, floors_column};
use super::editor::{checks, view};
use super::fight_columns::{arena_column, boss_column, staging_column};
use super::*;

/// A fighter's numbers: name, level, life, attack, defense, special attack, special defense.
#[derive(Clone)]
pub(super) struct Fighter {
    pub(super) name: String,
    pub(super) level: i32,
    pub(super) hp: i32,
    pub(super) attack: i32,
    pub(super) defense: i32,
    pub(super) sp_attack: i32,
    pub(super) sp_defense: i32,
}

/// A boss fight in numbers: the boss, its guards, how many come to help in
/// its phases, the team; how hard it looks (a word, its colour, the ratio).
#[derive(Clone)]
pub(super) struct Numbers {
    pub(super) boss: Fighter,
    pub(super) guards: Vec<Fighter>,
    pub(super) help: usize,
    pub(super) team: Vec<Fighter>,
    pub(super) allies: Vec<Fighter>,
    pub(super) verdict: (&'static str, Color32, f32),
}

/// How hard a fight looks, by its ratio (the team's time to win against the foes').
pub(super) fn verdict_of(ratio: f32) -> (&'static str, Color32) {
    match ratio {
        r if r < 0.4 => ("easy", Color32::from_rgb(0x6C, 0xC6, 0x6C)),
        r if r < 0.8 => ("balanced", Color32::from_rgb(0xC8, 0xD8, 0x50)),
        r if r < 1.3 => ("hard", Color32::from_rgb(0xF0, 0x98, 0x30)),
        _ => ("very hard", Color32::from_rgb(0xE5, 0x30, 0x30)),
    }
}

/// A rough look at a fight: the turns for the team to bring all down, against
/// the turns for them to bring the team down (the reinforcements counted as
/// many more guards, or quarter bosses).
pub(super) fn rough_ratio(foes: &[&Fighter], ours: &[&Fighter], help: usize) -> f32 {
    let hits = |from: &[&Fighter], against: &[&Fighter]| -> f32 {
        let guard = against.iter().map(|one| (one.defense + one.sp_defense) as f32 / 2.0).sum::<f32>() / against.len().max(1) as f32;
        from.iter().map(|one| (one.attack.max(one.sp_attack) as f32 - guard / 2.0).max(1.0)).sum()
    };
    let (boss, guard) = (foes.first().map_or(0.0, |boss| boss.hp as f32), foes.get(1).map(|guard| guard.hp as f32));
    let foe_life = foes.iter().map(|one| one.hp as f32).sum::<f32>() + help as f32 * guard.unwrap_or(boss / 4.0);
    let our_life: f32 = ours.iter().map(|one| one.hp as f32).sum();
    (foe_life / hits(ours, foes)) / (our_life / hits(foes, ours)).max(0.1)
}

/// The fight as the rules will play it, in numbers (none: it does not read).
fn numbers_of(making: &DungeonMaking) -> Option<Numbers> {
    let offered = offered(&making.session)?;
    let text = game::dive::resolved_for(&making.project, &making.played_text(true));
    let own = rules::own::read(&text, &offered.book.tables).ok()?;
    let fixed = own.table.floors.last()?.fixed.as_ref()?;
    let fighter = |species: u16, stats: &rules::dungeon::Stats| Fighter {
        name: offered.book.species_name(species),
        level: stats.level,
        hp: stats.max_hp,
        attack: stats.attack,
        defense: stats.defense,
        sp_attack: stats.sp_attack,
        sp_defense: stats.sp_defense,
    };
    let mut monsters = fixed.monsters.iter().map(|placed| fighter(placed.enemy.species, &placed.enemy.stats));
    let boss = monsters.next()?;
    let guards: Vec<Fighter> = monsters.collect();
    let help = own.boss.as_ref().map_or(0, |says| says.phases.iter().map(|phase| phase.help.len()).sum());
    let (hero, partner, level) = making.team.unwrap_or((offered.team.0, offered.team.1, 5));
    let level = making.form.team_level.or(own.team_level).unwrap_or(level);
    // (The fight's own team, if it says one: played alone, it is played by it.)
    let alone = making.fight || making.form.floors == 0;
    let says = own.boss.as_ref().filter(|_| alone);
    let (hero, partner, level) = match says {
        Some(says) => (says.hero.unwrap_or(hero), says.partner.or(says.hero.map(|_| partner)).unwrap_or(partner), says.level.unwrap_or(level)),
        None => (hero, partner, level),
    };
    let allies: Vec<Fighter> = fixed.allies.iter().map(|placed| fighter(placed.enemy.species, &placed.enemy.stats)).collect();
    let team: Vec<Fighter> = [(hero, false), (partner, true)]
        .iter()
        .filter_map(|(id, second)| Some(fighter(*id, &offered.book.member_at(*id, *second, Some(level))?.stats)))
        .collect();
    let foes: Vec<&Fighter> = std::iter::once(&boss).chain(guards.iter()).collect();
    let ours: Vec<&Fighter> = team.iter().chain(allies.iter()).collect();
    let ratio = rough_ratio(&foes, &ours, help);
    let verdict = verdict_of(ratio);
    Some(Numbers { boss, guards, help, team, allies, verdict: (verdict.0, verdict.1, ratio) })
}

/// The numbers, read again only when the text changes.
pub(super) fn numbers(making: &mut DungeonMaking) -> Option<Numbers> {
    let key = format!("{}|{:?}", making.played_text(true), making.team);
    if making.numbers.0 != key {
        making.numbers = (key, numbers_of(making));
    }
    making.numbers.1.clone()
}

/// A section's title.
pub(super) fn section(ui: &mut egui::Ui, said: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(said).size(theme::SMALL).color(theme::WEAK).strong());
    ui.add_space(2.0);
}

/// A hint under a title.
pub(super) fn hint(ui: &mut egui::Ui, said: &str) {
    ui.label(theme::text(said, theme::SMALL - 1.0, theme::FAINT));
}

/// A column of the editor: its panel, its scrolling.
fn column(ui: &mut egui::Ui, rect: Rect, id: &str, fill: Color32, inside: impl FnOnce(&mut egui::Ui)) {
    ui.painter().rect_filled(rect, 0.0, fill);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(12.0)).layout(egui::Layout::top_down(egui::Align::Min)));
    egui::ScrollArea::vertical().id_salt(id).auto_shrink(false).show(&mut child, |ui| {
        // (No wider than the column, its scroll bar aside, whatever is in it.)
        ui.set_max_width(rect.width() - 24.0 - 12.0);
        inside(ui);
    });
}

/// The body of the editor, under its head: three columns.
pub(super) fn body(ui: &mut egui::Ui, body: Rect, making: &mut DungeonMaking, audio: &mut Option<Box<dyn Hears>>, volume: f32) -> Option<Done> {
    let mut asked = None;
    // (Only what the window shows.)
    let body = body.intersect(ui.clip_rect());
    let wide = body.width();
    let (left, middle) = (wide * 0.27, wide * 0.43);
    let a = Rect::from_min_size(body.min, vec2(left, body.height()));
    let b = Rect::from_min_size(pos2(a.max.x + 1.0, body.min.y), vec2(middle - 2.0, body.height()));
    let c = Rect::from_min_max(pos2(b.max.x + 1.0, body.min.y), body.max);
    for line in [a.max.x, b.max.x] {
        ui.painter().vline(line, body.y_range(), egui::Stroke::new(1.0_f32, theme::GAP));
    }
    if making.fight {
        column(ui, a, "fight: the boss", theme::PANEL, |ui| boss_column(ui, making));
        column(ui, b, "fight: the arena", theme::GROUND, |ui| arena_column(ui, making, audio));
        column(ui, c, "fight: the staging", theme::PANEL, |ui| staging_column(ui, making, audio, volume));
    } else {
        column(ui, a, "dungeon: what it is", theme::PANEL, |ui| asked = dungeon_column(ui, making, audio, volume));
        column(ui, b, "dungeon: its floors", theme::GROUND, |ui| floors_column(ui, making));
        column(ui, c, "dungeon: the trial", theme::PANEL, |ui| {
            view(ui, making, audio);
            ui.add_space(8.0);
            checks(ui, making);
        });
    }
    asked
}
