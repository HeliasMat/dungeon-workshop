//! The boss fight's arena: its tools, drawn tile by tile.

use eframe::egui::{self, Color32, Rect, RichText, Sense, vec2};

use super::{ALLY, ArenaTool, BossForm, Offered, Tile};
use workshop_base::theme;

/// The tools of the arena, as the editor keeps them.
pub(super) struct ArenaParts<'a> {
    tool: &'a mut ArenaTool,
    item: &'a mut String,
    trap: &'a mut String,
    hidden: &'a mut bool,
}

pub(super) fn making_arena<'a>(tool: &'a mut ArenaTool, item: &'a mut String, trap: &'a mut String, hidden: &'a mut bool) -> ArenaParts<'a> {
    ArenaParts { tool, item, trap, hidden }
}

/// What a click on the arena puts there: the team, the boss, a guard, an
/// item (which), a trap (which, hidden or not), or nothing (the eraser).
pub(super) fn arena_tools(ui: &mut egui::Ui, parts: ArenaParts, boss: &BossForm, offered: Option<&Offered>) {
    ui.label(theme::text("a click on the arena puts what is chosen there; right click: takes away what is there", theme::SMALL - 1.0, theme::FAINT));
    ui.horizontal_wrapped(|ui| {
        let mut tool = |ui: &mut egui::Ui, which: ArenaTool, said: String, colour: Color32| {
            if ui.selectable_label(*parts.tool == which, RichText::new(said).color(colour)).clicked() {
                *parts.tool = which;
            }
        };
        tool(ui, ArenaTool::Team, "● the team".to_string(), Color32::from_rgb(0x4A, 0xDE, 0x80));
        tool(ui, ArenaTool::Boss, format!("● {}", boss.species), Color32::from_rgb(0xE5, 0x30, 0x30));
        for (at, (species, ..)) in boss.guards.iter().enumerate() {
            tool(ui, ArenaTool::Guard(at), format!("● {species} {}", at + 1), Color32::from_rgb(0xF0, 0x98, 0x30));
        }
        for (at, (species, ..)) in boss.allies.iter().enumerate() {
            tool(ui, ArenaTool::Ally(at), format!("● {species} (ally)"), ALLY);
        }
        tool(ui, ArenaTool::Item, "■ an item".to_string(), Color32::from_rgb(0x60, 0xC8, 0xF0));
        tool(ui, ArenaTool::Trap, "✕ a trap".to_string(), Color32::from_rgb(0xC0, 0x70, 0xF0));
        tool(ui, ArenaTool::Erase, "eraser".to_string(), theme::WEAK);
    });
    ui.horizontal(|ui| {
        ui.label(theme::text("paint", theme::SMALL, theme::WEAK));
        for (kind, said, colour) in [
            (0, "▒ ground", Color32::from_rgb(0xC8, 0xA8, 0x70)),
            (1, "█ wall", Color32::from_rgb(0x90, 0x80, 0x70)),
            (2, "≈ water", Color32::from_rgb(0x3C, 0x78, 0xC8)),
        ] {
            if ui
                .selectable_label(*parts.tool == ArenaTool::Paint(kind), RichText::new(said).color(colour))
                .on_hover_text("click or drag on the arena")
                .clicked()
            {
                *parts.tool = ArenaTool::Paint(kind);
            }
        }
    });
    match *parts.tool {
        ArenaTool::Item => {
            ui.horizontal(|ui| {
                ui.label(theme::text("item", theme::SMALL, theme::WEAK));
                egui::ComboBox::from_id_salt("arena item").width(200.0).selected_text(parts.item.as_str()).height(360.0).show_ui(ui, |ui| {
                    for name in offered.map(|offered| offered.items.as_slice()).unwrap_or_default() {
                        if ui.selectable_label(*parts.item == *name, name).clicked() {
                            *parts.item = name.clone();
                        }
                    }
                });
            });
        }
        ArenaTool::Trap => {
            ui.horizontal(|ui| {
                ui.label(theme::text("trap", theme::SMALL, theme::WEAK));
                egui::ComboBox::from_id_salt("arena trap").width(160.0).selected_text(parts.trap.as_str()).height(360.0).show_ui(ui, |ui| {
                    for (word, _) in rules::own::TRAPS {
                        if ui.selectable_label(*parts.trap == word, word).clicked() {
                            *parts.trap = word.to_string();
                        }
                    }
                });
                ui.checkbox(parts.hidden, "hidden").on_hover_text("only seen when stepped on");
            });
        }
        _ => {}
    }
}

/// The arena as it will be: walls, floor, water, where the team comes in
/// (green), the boss (red), its guards (orange), the items (blue squares),
/// the traps (violet crosses, faint if hidden). A click puts there what the
/// tool says; a right click takes away what is there.
pub(super) fn arena_drawn(ui: &mut egui::Ui, boss: &mut BossForm, tool: ArenaTool, offered: (&str, &str, bool)) {
    let (wide, high) = boss.room;
    let cell = (ui.available_width() / (wide + 2) as f32).min(560.0 / (high + 2) as f32).floor().clamp(4.0, 36.0);
    let (rect, response) = ui.allocate_exact_size(vec2(cell * (wide + 2) as f32, cell * (high + 2) as f32), Sense::click_and_drag());
    let cell_at = |at: egui::Pos2| {
        let (x, y) = (((at.x - rect.min.x) / cell).floor() as i32 - 1, ((at.y - rect.min.y) / cell).floor() as i32 - 1);
        (x >= 0 && y >= 0 && (x as usize) < wide && (y as usize) < high).then_some((x as usize, y as usize))
    };
    // (A click: what the tool says, there; a right click: away with what is there.)
    let pointed = response.interact_pointer_pos().and_then(cell_at);
    if let (ArenaTool::Paint(kind), Some(at)) = (tool, pointed)
        && (response.clicked() || response.dragged())
    {
        boss.paint(at, kind);
    }
    if response.clicked()
        && let Some(at) = pointed
    {
        put(boss, tool, at, offered);
    }
    if response.secondary_clicked()
        && let Some(at) = pointed
    {
        taken_away(boss, at);
    }
    let placed = Placed::of(boss);
    draw_arena(&ui.painter_at(rect), rect, cell, boss, &placed);
    // (What the pointer is over, said.)
    if let Some(at) = response.hover_pos().and_then(cell_at) {
        response.on_hover_text(said_at(boss, &placed, at));
    }
    ui.label(theme::text("green: the team · turquoise: its allies · red: the boss · orange: its guards · blue: the items · violet: the traps (pale: hidden) · water in dark blue", theme::SMALL - 1.0, theme::FAINT));
    listed(ui, boss);
}

/// What the tool puts on a tile clicked: the team, the boss, a guard, an ally, the item or trap offered; or what
/// lies there taken away.
fn put(boss: &mut BossForm, tool: ArenaTool, at: Tile, (item, trap, hidden): (&str, &str, bool)) {
    match tool {
        ArenaTool::Team => boss.team = Some(at),
        ArenaTool::Boss => boss.place = Some(at),
        ArenaTool::Guard(which) => {
            if let Some(guard) = boss.guards.get_mut(which) {
                guard.2 = Some(at);
            }
        }
        ArenaTool::Ally(which) => {
            if let Some(ally) = boss.allies.get_mut(which) {
                ally.2 = Some(at);
            }
        }
        ArenaTool::Item if !item.is_empty() => {
            boss.lying.retain(|(_, there)| *there != at);
            boss.lying.push((item.to_string(), at));
        }
        ArenaTool::Trap if !trap.is_empty() => {
            boss.traps.retain(|(_, there, _)| *there != at);
            boss.traps.push((trap.to_string(), at, hidden));
        }
        ArenaTool::Erase => {
            boss.lying.retain(|(_, there)| *there != at);
            boss.traps.retain(|(_, there, _)| *there != at);
        }
        _ => {}
    }
}

/// A right click: the items and traps there taken away, whoever was put there back in their first place.
fn taken_away(boss: &mut BossForm, at: Tile) {
    boss.lying.retain(|(_, there)| *there != at);
    boss.traps.retain(|(_, there, _)| *there != at);
    for one in boss.guards.iter_mut().chain(boss.allies.iter_mut()) {
        if one.2 == Some(at) {
            one.2 = None;
        }
    }
}

/// Where everyone stands in the arena, put there or where it puts them.
struct Placed {
    leader: Tile,
    boss: Tile,
    guards: Vec<Tile>,
    allies: Vec<Option<Tile>>,
    /// The tiles someone or something is on: ground under them.
    used: Vec<Tile>,
}

impl Placed {
    fn of(boss: &BossForm) -> Placed {
        let (wide, high) = boss.room;
        let (leader, boss_at, sides) = rules::own::arena_places(wide, high);
        let (leader, boss_at) = (boss.team.unwrap_or(leader), boss.place.unwrap_or(boss_at));
        let guards: Vec<Tile> = boss.guards.iter().zip(sides).map(|((.., at), side)| at.unwrap_or(side)).collect();
        let taken: Vec<Tile> = [boss_at].into_iter().chain(guards.iter().copied()).chain(boss.allies.iter().filter_map(|(.., at)| *at)).collect();
        let mut around = rules::own::allies_around(leader, (wide, high), &taken).into_iter();
        let allies: Vec<Option<Tile>> = boss.allies.iter().map(|(.., at)| at.or_else(|| around.next())).collect();
        let used: Vec<Tile> = [leader, boss_at]
            .into_iter()
            .chain(guards.iter().copied())
            .chain(allies.iter().flatten().copied())
            .chain(boss.lying.iter().map(|(_, at)| *at))
            .chain(boss.traps.iter().map(|(_, at, _)| *at))
            .collect();
        Placed { leader, boss: boss_at, guards, allies, used }
    }
}

/// The tiles, the items, the traps, the team, the boss, its guards, the allies.
fn draw_arena(painter: &egui::Painter, rect: Rect, cell: f32, boss: &BossForm, placed: &Placed) {
    painter.rect_filled(rect, 0.0, Color32::from_rgb(0x3A, 0x30, 0x28));
    let (wide, high) = boss.room;
    for y in 0..high {
        for x in 0..wide {
            let kind = if placed.used.contains(&(x, y)) { 0 } else { boss.tile((x, y)) };
            let colour = match kind {
                1 => Color32::from_rgb(0x3A, 0x30, 0x28),
                2 => Color32::from_rgb(0x3C, 0x78, 0xC8),
                _ => Color32::from_rgb(0xC8, 0xA8, 0x70),
            };
            let at = rect.min + vec2((x + 1) as f32 * cell, (y + 1) as f32 * cell);
            painter.rect_filled(Rect::from_min_size(at, vec2(cell - 1.0, cell - 1.0)), 0.0, colour);
        }
    }
    let middle = |(x, y): Tile| rect.min + vec2((x as f32 + 1.5) * cell, (y as f32 + 1.5) * cell);
    for (_, at) in &boss.lying {
        painter.rect_filled(Rect::from_center_size(middle(*at), vec2(cell * 0.5, cell * 0.5)), 1.0, Color32::from_rgb(0x60, 0xC8, 0xF0));
    }
    for (_, at, hidden) in &boss.traps {
        let (centre, size) = (middle(*at), cell * 0.3);
        let colour = Color32::from_rgb(0xC0, 0x70, 0xF0).gamma_multiply(if *hidden { 0.5 } else { 1.0 });
        let stroke = egui::Stroke::new(2.0_f32, colour);
        painter.line_segment([centre - vec2(size, size), centre + vec2(size, size)], stroke);
        painter.line_segment([centre + vec2(-size, size), centre + vec2(size, -size)], stroke);
    }
    painter.circle_filled(middle(placed.leader), cell * 0.4, Color32::from_rgb(0x4A, 0xDE, 0x80));
    painter.circle_filled(middle(placed.boss), cell * 0.45, Color32::from_rgb(0xE5, 0x30, 0x30));
    for at in &placed.guards {
        painter.circle_filled(middle(*at), cell * 0.35, Color32::from_rgb(0xF0, 0x98, 0x30));
    }
    for at in placed.allies.iter().flatten() {
        painter.circle_filled(middle(*at), cell * 0.35, ALLY);
    }
}

/// What is on a tile, said.
fn said_at(boss: &BossForm, placed: &Placed, at: Tile) -> String {
    let mut there: Vec<String> = Vec::new();
    if at == placed.leader {
        there.push("the team".to_string());
    }
    if at == placed.boss {
        there.push(boss.species.clone());
    }
    for ((species, ..), guard) in boss.guards.iter().zip(&placed.guards) {
        if *guard == at {
            there.push(species.clone());
        }
    }
    for ((species, ..), ally) in boss.allies.iter().zip(&placed.allies) {
        if *ally == Some(at) {
            there.push(format!("{species} (ally)"));
        }
    }
    there.extend(boss.lying.iter().filter(|(_, there)| *there == at).map(|(item, _)| item.clone()));
    there.extend(
        boss.traps.iter().filter(|(_, there, _)| *there == at).map(|(kind, _, hidden)| format!("trap: {kind}{}", if *hidden { " (hidden)" } else { "" })),
    );
    format!("tile {},{}{}", at.0, at.1, if there.is_empty() { String::new() } else { format!(": {}", there.join(", ")) })
}

/// What lies there and its traps, listed: × takes one away; everyone put back in their first place.
fn listed(ui: &mut egui::Ui, boss: &mut BossForm) {
    let mut gone_item = None;
    let mut gone_trap = None;
    ui.horizontal_wrapped(|ui| {
        for (at, (item, (x, y))) in boss.lying.iter().enumerate() {
            if ui.small_button(format!("{item} {x},{y}  ×")).clicked() {
                gone_item = Some(at);
            }
        }
        for (at, (kind, (x, y), hidden)) in boss.traps.iter().enumerate() {
            if ui.small_button(format!("trap {kind}{} {x},{y}  ×", if *hidden { " (hidden)" } else { "" })).clicked() {
                gone_trap = Some(at);
            }
        }
    });
    if let Some(at) = gone_item {
        boss.lying.remove(at);
    }
    if let Some(at) = gone_trap {
        boss.traps.remove(at);
    }
    if (boss.team.is_some() || boss.place.is_some() || boss.guards.iter().chain(&boss.allies).any(|(.., at)| at.is_some()))
        && ui.small_button("put everyone back in their first place").clicked()
    {
        (boss.team, boss.place) = (None, None);
        for one in boss.guards.iter_mut().chain(boss.allies.iter_mut()) {
            one.2 = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_says_who_and_what_is_on_it() {
        let mut boss = BossForm::new();
        boss.traps.push(("spikes".to_string(), (2, 2), true));
        let placed = Placed::of(&boss);
        assert!(placed.used.contains(&placed.leader) && placed.used.contains(&(2, 2)));
        assert_eq!(said_at(&boss, &placed, placed.boss), format!("tile {},{}: Spearow", placed.boss.0, placed.boss.1));
        assert_eq!(said_at(&boss, &placed, (2, 2)), "tile 2,2: trap: spikes (hidden)");
        taken_away(&mut boss, (2, 2));
        assert!(boss.traps.is_empty());
    }
}
