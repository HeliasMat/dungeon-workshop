//! The Hierarchy: the layers (`layers.rs`), shown or hidden, locked; then
//! each line's things, the one in front first, in their groups; dragged to be
//! put in another order. What is read of the decor once (`Read`); what the
//! rows ask (`Asked`), done at the end.

use decor::doc::Pose;
use eframe::egui::{self, Color32, RichText, Sense, pos2, vec2};

use super::layers::layers;
use super::{badge_of, cross_button, paint_badge};
use crate::Decors;
use crate::edit::{self, Item, Kind};
use workshop_base::theme;

/// A thing of the decor: its line, its place on the line.
pub(super) type Element = (usize, usize);

/// A line of the Hierarchy dragged to another place (the line's number).
#[derive(Clone, Copy)]
pub(super) struct Dragged(pub(super) usize);

/// A row of the tree: a group's head, or a line.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Row {
    Group(usize),
    Line(usize),
}

/// A line as the tree shows it: its things, its name, what is said after it
/// (grey), its badge, its pose if it lays one, its group, its name to rename.
pub(super) struct Shown {
    keys: Vec<Element>,
    name: String,
    description: String,
    badge: (&'static str, Color32, &'static str),
    pose: Option<Pose>,
    group: Option<usize>,
    renamed: Option<String>,
}

/// What the Hierarchy reads of the decor once: its text, what is chosen, its
/// things, its terrains, its groups (name, first line, last line); whether a
/// click adds to what is chosen (Ctrl or Shift held).
pub(super) struct Read {
    pub(super) text: String,
    pub(super) selection: Vec<Element>,
    pub(super) items: Vec<Item>,
    pub(super) terrains: Vec<(usize, decor::terrain::Painted)>,
    pub(super) groups: Vec<(String, usize, usize)>,
    pub(super) toggling: bool,
}

/// What the rows ask, done once they are drawn: the text made, what to
/// choose, to frame, what is pointed, what was refused; a group folded or
/// opened; a name being renamed, one renamed.
#[derive(Default)]
pub(super) struct Asked {
    pub(super) made: Option<String>,
    pub(super) chose: Option<Vec<Element>>,
    pub(super) framed: Option<Vec<Element>>,
    pub(super) hovered: Vec<Element>,
    pub(super) refused: Option<String>,
    pub(super) folding: Option<String>,
    pub(super) renaming: Option<(usize, String)>,
    pub(super) renamed: Option<(usize, String)>,
}

/// How tall a row is, how far a level goes in.
pub(super) const ROW: f32 = 22.0;
const INDENT: f32 = 14.0;
/// The colours of the tree: a row pointed at, chosen, chosen where the list has the keys.
pub(super) const ROW_HOVERED: Color32 = Color32::from_rgb(0x2A, 0x2D, 0x2E);
const ROW_CHOSEN: Color32 = Color32::from_rgb(0x37, 0x37, 0x3D);
pub(super) const ROW_FOCUSED: Color32 = Color32::from_rgb(0x04, 0x39, 0x5E);
const FOCUS_LINE: Color32 = Color32::from_rgb(0x00, 0x7F, 0xD4);

/// A section's head, as VS Code has them: a chevron, its title in capitals,
/// its actions on the right when pointed at. Whether it is open.
pub(super) fn section_head(ui: &mut egui::Ui, title: &str, open: &mut bool, actions: impl FnOnce(&mut egui::Ui, egui::Rect)) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click());
    ui.painter().rect_filled(rect, 0.0, theme::TABS);
    workshop_base::icons::chevron(ui.painter(), pos2(rect.min.x + 10.0, rect.center().y), *open, theme::TEXT);
    ui.painter().text(
        pos2(rect.min.x + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title.to_uppercase(),
        egui::FontId::proportional(theme::SMALL),
        theme::STRONG,
    );
    if ui.rect_contains_pointer(rect) {
        actions(ui, rect);
    }
    if response.clicked() {
        *open = !*open;
    }
}

/// The square of an action, `at` from the right of a row.
pub(super) fn action_rect(rect: egui::Rect, at: usize) -> egui::Rect {
    egui::Rect::from_center_size(pos2(rect.max.x - 12.0 - at as f32 * 20.0, rect.center().y), vec2(18.0, 18.0))
}

/// The first `groupN` name no group has yet.
pub(super) fn new_group_name(groups: &[(String, usize, usize)]) -> String {
    (1..).map(|at| format!("group{at}")).find(|name| !groups.iter().any(|(known, _, _)| known == name)).unwrap_or_default()
}

/// What is chosen once a row is clicked: its things; with Ctrl or Shift,
/// added to what was chosen (taken out if they all were).
pub(super) fn chosen_after_click(selection: &[Element], keys: &[Element], chosen: bool, toggling: bool) -> Vec<Element> {
    if !toggling {
        return keys.to_vec();
    }
    let mut now = selection.to_vec();
    if chosen {
        now.retain(|key| !keys.contains(key));
    } else {
        now.extend(keys.iter().copied());
    }
    now
}

/// The rows of the tree: a group's head before its first line, its lines
/// under it unless it is folded (nothing is folded while searching).
pub(super) fn tree_rows(
    lines: &[(usize, Option<usize>)],
    groups: &[(String, usize, usize)],
    folded: &std::collections::HashSet<String>,
    searching: bool,
) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    for (number, group) in lines {
        if let Some(at) = group
            && !rows.contains(&Row::Group(*at))
        {
            rows.push(Row::Group(*at));
        }
        let hidden = group.is_some_and(|at| folded.contains(&groups[at].0)) && !searching;
        if !hidden {
            rows.push(Row::Line(*number));
        }
    }
    rows
}

/// The row an arrow key goes to, from the one chosen (or the first).
pub(super) fn row_after_arrow(now: Option<usize>, up: bool, rows: usize) -> usize {
    match now {
        Some(at) if up => at.saturating_sub(1),
        Some(at) => (at + 1).min(rows.saturating_sub(1)),
        None => 0,
    }
}

/// The things of a row: a line's, or all those of a group's lines.
fn row_keys(shown: &[(usize, Shown)], row: &Row) -> Vec<Element> {
    match row {
        Row::Line(number) => shown.iter().find(|(known, _)| known == number).map(|(_, one)| one.keys.clone()).unwrap_or_default(),
        Row::Group(at) => shown.iter().filter(|(_, one)| one.group == Some(*at)).flat_map(|(_, one)| one.keys.clone()).collect(),
    }
}

/// The Hierarchy: the Layers, then the Elements (their search, their tree, the keys of the list).
pub fn hierarchy(ui: &mut egui::Ui, decors: &mut Decors) {
    let Some(open) = &decors.open else {
        ui.label(theme::weak("No decor open."));
        return;
    };
    let text = open.text.clone();
    let read = Read {
        selection: open.selection(),
        items: edit::items(&text),
        terrains: crate::terrains::terrain_lines(&text),
        groups: edit::groups(&text),
        toggling: ui.input(|input| input.modifiers.command || input.modifiers.shift),
        text,
    };
    let mut asked = Asked::default();
    ui.spacing_mut().item_spacing.y = 0.0;
    layers(ui, decors, &read, &mut asked);
    if !elements_head(ui, decors, &read, &mut asked) {
        decors.hier_hovered = asked.hovered;
        apply(decors, asked.made, asked.chose, None, asked.refused, &read.items);
        return;
    }
    search_field(ui, decors);
    said_over_the_list(ui, decors);
    let shown = shown_lines(decors, &read);
    let searching = !decors.hier_sought.is_empty();
    let lines: Vec<(usize, Option<usize>)> = shown.iter().map(|(number, one)| (*number, one.group)).collect();
    let rows = tree_rows(&lines, &read.groups, &decors.folded, searching);
    asked.renaming = decors.renaming.clone();
    let list = elements_list(ui, decors, &read, &shown, &rows, &mut asked);
    // (The list has the keys once clicked in; the scene takes them back.)
    let in_list = ui.rect_contains_pointer(list);
    if ui.input(|input| input.pointer.any_pressed()) {
        decors.hier_focus = in_list;
    }
    if decors.hier_focus && asked.renaming.is_none() && !ui.ctx().wants_keyboard_input() {
        list_keys(ui, decors, &read, &shown, &rows, &mut asked);
    }
    if let Some(name) = asked.folding.take()
        && !decors.folded.remove(&name)
    {
        decors.folded.insert(name);
    }
    if let Some((line, word)) = asked.renamed.take() {
        match edit::renamed(&read.text, line, &word) {
            Some(text) => asked.made = Some(text),
            None => asked.refused = Some("this name does not fit here".to_string()),
        }
    }
    decors.renaming = asked.renaming;
    decors.hier_hovered = asked.hovered;
    apply(decors, asked.made, asked.chose, asked.framed, asked.refused, &read.items);
}

/// The Elements' head (search, fold all, group the chosen ones) and what its
/// actions do; whether the section is open.
fn elements_head(ui: &mut egui::Ui, decors: &mut Decors, read: &Read, asked: &mut Asked) -> bool {
    let mut elements_open = decors.sections.1;
    let mut fold_all = false;
    let mut new_group = false;
    let mut search = false;
    section_head(ui, "Elements", &mut elements_open, |ui, rect| {
        search = workshop_base::icons::button(ui, action_rect(rect, 0), "search (Ctrl+F)", workshop_base::icons::search);
        fold_all = workshop_base::icons::button(ui, action_rect(rect, 1), "fold all", workshop_base::icons::fold_all);
        new_group = workshop_base::icons::button(ui, action_rect(rect, 2), "group the chosen elements", workshop_base::icons::plus);
    });
    decors.sections.1 = elements_open;
    // (The search: by its icon, or Ctrl+F with the pointer over the list.)
    let asked_search = ui.rect_contains_pointer(ui.max_rect()) && ui.input(|input| input.modifiers.command && input.key_pressed(egui::Key::F));
    if search || asked_search {
        decors.search_shown = asked_search || !decors.search_shown;
        decors.search_focus = decors.search_shown;
    }
    if fold_all {
        decors.folded.extend(read.groups.iter().map(|(name, _, _)| name.clone()));
    }
    if new_group {
        group_chosen(read, asked);
    }
    elements_open
}

/// The things chosen put in a new group (or why they cannot be).
fn group_chosen(read: &Read, asked: &mut Asked) {
    let numbers: Vec<usize> = read.selection.iter().map(|(line, _)| *line).collect();
    match edit::grouped(&read.text, &numbers, &new_group_name(&read.groups)) {
        Ok(grouped) => asked.made = Some(grouped),
        Err(why) => asked.refused = Some(why),
    }
}

/// The search over the list (shown, or something sought): the field, its cross.
fn search_field(ui: &mut egui::Ui, decors: &mut Decors) {
    if !decors.search_shown && decors.hier_sought.is_empty() {
        return;
    }
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let field = ui.add(egui::TextEdit::singleline(&mut decors.hier_sought).hint_text("Search for an element").desired_width(ui.available_width() - 30.0));
        if decors.search_focus {
            field.request_focus();
            decors.search_focus = false;
        }
        if workshop_base::icons::button(
            ui,
            egui::Rect::from_center_size(pos2(ui.max_rect().max.x - 12.0, field.rect.center().y), vec2(18.0, 18.0)),
            "close the search",
            |painter, at, tint| {
                let centre = at.center();
                painter.line_segment([centre + vec2(-4.0, -4.0), centre + vec2(4.0, 4.0)], egui::Stroke::new(1.3_f32, tint));
                painter.line_segment([centre + vec2(-4.0, 4.0), centre + vec2(4.0, -4.0)], egui::Stroke::new(1.3_f32, tint));
            },
        ) {
            decors.hier_sought.clear();
            decors.search_shown = false;
        }
    });
    ui.add_space(3.0);
}

/// Over the list: the layer it is filtered on (a cross: the whole list), what was refused.
fn said_over_the_list(ui: &mut egui::Ui, decors: &mut Decors) {
    for (said, colour) in [
        (
            decors.layer_filter.map(|filter| format!("only: {}", edit::LAYERS.iter().find(|(word, _)| *word == filter).map_or(filter, |(_, name)| name))),
            theme::DECOR,
        ),
        (decors.hier_said.clone(), theme::WRONG),
    ] {
        if let Some(said) = said {
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(theme::text(said, theme::SMALL, colour));
                if colour == theme::DECOR && cross_button(ui, "the whole list") {
                    decors.layer_filter = None;
                }
            });
        }
    }
}

/// Each line shown, the one in front first: its things, its name, what is
/// said after it, its badge (those sought, on the layer filtered, only).
fn shown_lines(decors: &Decors, read: &Read) -> Vec<(usize, Shown)> {
    let mut numbers: Vec<usize> = read.items.iter().map(|one| one.line).chain(read.terrains.iter().map(|(line, _)| *line)).collect();
    numbers.sort_unstable();
    numbers.dedup();
    let sought = decors.hier_sought.to_lowercase();
    numbers.iter().rev().filter_map(|number| shown_line(decors, read, *number, &sought).map(|one| (*number, one))).collect()
}

/// A line as the tree shows it, if it is to be shown.
fn shown_line(decors: &Decors, read: &Read, number: usize, sought: &str) -> Option<Shown> {
    let here: Vec<&Item> = read.items.iter().filter(|one| one.line == number).collect();
    let terrain = read.terrains.iter().find(|(line, _)| *line == number);
    let pose = read.text.lines().nth(number - 1).and_then(Pose::read);
    let (name, description, renamed) = match (here.first(), terrain, &pose) {
        (_, Some((_, painted)), _) => {
            let place = decors.pieces.iter().find_map(|piece| piece.place.as_ref().filter(|(level, _)| *level == painted.level).map(|(_, name)| name.clone()));
            (
                format!("Terrain · {}", place.unwrap_or_else(|| format!("place {}", painted.level))),
                format!("{} × {} tiles", painted.columns(), painted.rows()),
                None,
            )
        }
        (Some(one), _, Some(pose)) => {
            let name = decors.item_name(one);
            let count = if pose.points.len() > 1 { format!("×{}  ", pose.points.len()) } else { String::new() };
            let spot = pose.spot.as_ref().map(|spot| format!("“{spot}”  ")).unwrap_or_default();
            let renamed = Some(if one.kind == Kind::Lives { one.name.clone() } else { pose.spot.clone().unwrap_or_default() });
            (name, format!("{count}{spot}{} {}", one.corner[0], one.corner[1]), renamed)
        }
        (Some(one), _, None) => {
            let renamed = (one.kind != Kind::Exit).then(|| one.name.clone());
            (decors.item_name(one), format!("{} {}", one.corner[0], one.corner[1]), renamed)
        }
        (None, _, Some(pose)) => (decors.shown_name(&pose.name), String::new(), None),
        _ => return None,
    };
    if !sought.is_empty()
        && !name.to_lowercase().contains(sought)
        && !description.to_lowercase().contains(sought)
        && !pose.as_ref().is_some_and(|pose| pose.name.contains(sought))
    {
        return None;
    }
    if let Some(filter) = decors.layer_filter {
        let layer = if terrain.is_some() { "terrains" } else { here.first().map_or("", |one| one.layer) };
        if layer != filter {
            return None;
        }
    }
    let keys = if here.is_empty() { vec![(number, 0)] } else { here.iter().map(|one| (one.line, one.at)).collect() };
    let badge = badge_of(decors, here.first().copied(), terrain.is_some());
    let group = read.groups.iter().position(|(_, start, end)| *start < number && number < *end);
    Some(Shown { keys, name, description, badge, pose, group, renamed })
}

/// What a row knows as it is drawn: what it holds, whether chosen, its
/// rectangle and response, how far in, its left, whether pointed, its colour.
struct RowDrawn<'a> {
    keys: Vec<Element>,
    chosen: bool,
    rect: egui::Rect,
    response: &'a egui::Response,
    left: f32,
    pointed: bool,
    colour: Color32,
}

/// The tree in its scrolling: each row, then the empty under them (a click there chooses nothing). Its rectangle given back.
fn elements_list(ui: &mut egui::Ui, decors: &Decors, read: &Read, shown: &[(usize, Shown)], rows: &[Row], asked: &mut Asked) -> egui::Rect {
    let changed_selection = decors.hier_seen != read.selection;
    let sought_empty = decors.hier_sought.is_empty();
    let area = egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if rows.is_empty() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.label(theme::weak(if sought_empty { "Nothing put: the Library, below." } else { "Nothing is called that." }));
            });
        }
        let tree_pointed = ui.rect_contains_pointer(ui.max_rect());
        for row_at in rows {
            tree_row(ui, decors, read, shown, row_at, tree_pointed, changed_selection, asked);
        }
        let (_, empty) = ui.allocate_exact_size(vec2(ui.available_width(), ui.available_height().max(20.0)), Sense::click());
        if empty.clicked() && !read.toggling {
            asked.chose = Some(Vec::new());
        }
    });
    area.inner_rect
}

/// A row of the tree: its fill, its guides, a group's head or a line, its
/// name renamed in place, a line dropped on it, a double-click frames it.
#[allow(clippy::too_many_arguments)]
fn tree_row(
    ui: &mut egui::Ui,
    decors: &Decors,
    read: &Read,
    shown: &[(usize, Shown)],
    row_at: &Row,
    tree_pointed: bool,
    changed_selection: bool,
    asked: &mut Asked,
) {
    let keys = row_keys(shown, row_at);
    let chosen = !keys.is_empty() && keys.iter().all(|key| read.selection.contains(key));
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::click_and_drag());
    let depth = match row_at {
        Row::Line(number) if shown.iter().any(|(known, one)| known == number && one.group.is_some()) => 1,
        _ => 0,
    };
    let into_group = matches!(row_at, Row::Group(_)) && response.dnd_hover_payload::<Dragged>().is_some();
    let list_focused = decors.hier_focus;
    let fill = if into_group {
        theme::DECOR.gamma_multiply(0.3)
    } else if chosen && list_focused {
        ROW_FOCUSED
    } else if chosen {
        ROW_CHOSEN
    } else if response.hovered() {
        ROW_HOVERED
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    if chosen && list_focused && read.selection.first().is_some_and(|first| keys.contains(first)) {
        ui.painter().rect_stroke(rect.shrink(0.5), 0.0, egui::Stroke::new(1.0_f32, FOCUS_LINE), egui::StrokeKind::Inside);
    }
    // (The guides of the levels, when the tree is pointed at.)
    for level in 0..depth {
        let x = rect.min.x + 14.0 + level as f32 * INDENT;
        ui.painter().vline(x, rect.y_range(), egui::Stroke::new(1.0_f32, Color32::from_white_alpha(if tree_pointed { 40 } else { 16 })));
    }
    if response.hovered() {
        asked.hovered = keys.clone();
    }
    if chosen && changed_selection {
        response.scroll_to_me(Some(egui::Align::Center));
    }
    let left = rect.min.x + 8.0 + depth as f32 * INDENT;
    let drawn = RowDrawn {
        keys,
        chosen,
        rect,
        response: &response,
        left,
        pointed: ui.rect_contains_pointer(rect),
        colour: if chosen { theme::STRONG } else { theme::TEXT },
    };
    // (Where a line dropped here goes: in front of this one, or into this group.)
    let target = match row_at {
        Row::Group(at) => Some(group_row(ui, decors, read, *at, &drawn, asked)),
        Row::Line(number) => {
            let Some((_, one)) = shown.iter().find(|(known, _)| known == number) else { return };
            line_row(ui, decors, read, *number, one, &drawn, asked)
        }
    };
    let renamed_line = match row_at {
        Row::Group(at) => read.groups[*at].1,
        Row::Line(number) => *number,
    };
    renamed_in_row(ui, rect, left + 34.0, renamed_line, asked);
    if let Some((after, lead)) = target {
        if response.dnd_hover_payload::<Dragged>().is_some() && matches!(row_at, Row::Line(_)) {
            ui.painter().hline(rect.x_range(), rect.top(), egui::Stroke::new(2.0_f32, FOCUS_LINE));
        }
        if let Some(dragged) = response.dnd_release_payload::<Dragged>()
            && dragged.0 != after
        {
            asked.made = edit::line_moved(&read.text, dragged.0, after, &lead);
        }
    }
    if response.double_clicked() {
        asked.framed = Some(drawn.keys.clone());
    }
}

/// Renamed in its row: the field in place of the name; Enter keeps it, Escape leaves it.
fn renamed_in_row(ui: &mut egui::Ui, rect: egui::Rect, name_at: f32, renamed_line: usize, asked: &mut Asked) {
    let Some((line, word)) = asked.renaming.as_mut().filter(|(line, _)| *line == renamed_line) else { return };
    let room = egui::Rect::from_min_max(pos2(name_at, rect.min.y + 1.0), pos2(rect.max.x - 4.0, rect.max.y - 1.0));
    let field = ui.put(room, egui::TextEdit::singleline(word).margin(egui::Margin::symmetric(4, 1)));
    if !field.has_focus() && !field.lost_focus() {
        field.request_focus();
    }
    let (enter, escape) = ui.input(|input| (input.key_pressed(egui::Key::Enter), input.key_pressed(egui::Key::Escape)));
    if escape {
        asked.renaming = None;
    } else if field.lost_focus() || enter {
        asked.renamed = Some((*line, word.clone()));
        asked.renaming = None;
    }
}

/// A group's head: its chevron, its folder, its name and count; the whole
/// group shown or hidden, locked; a click folds it and chooses all it holds.
/// Where a line dropped on it goes (into it, at its end).
fn group_row(ui: &mut egui::Ui, decors: &Decors, read: &Read, at: usize, drawn: &RowDrawn, asked: &mut Asked) -> (usize, String) {
    let RowDrawn { rect, left, pointed, colour, .. } = *drawn;
    let (name, start, end) = &read.groups[at];
    let folded = decors.folded.contains(name) && decors.hier_sought.is_empty();
    workshop_base::icons::chevron(ui.painter(), pos2(left + 6.0, rect.center().y), !folded, theme::TEXT);
    workshop_base::icons::folder(
        ui.painter(),
        egui::Rect::from_center_size(pos2(left + 22.0, rect.center().y), vec2(16.0, 16.0)),
        !folded,
        Color32::from_rgb(0xDC, 0xB6, 0x7A),
    );
    if !asked.renaming.as_ref().is_some_and(|(line, _)| line == start) {
        ui.painter().text(pos2(left + 34.0, rect.center().y), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(theme::BODY), colour);
        let wide = ui.painter().layout_no_wrap(name.clone(), egui::FontId::proportional(theme::BODY), colour).size().x;
        ui.painter().text(
            pos2(left + 40.0 + wide, rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{}", drawn.keys.len()),
            egui::FontId::proportional(theme::SMALL),
            theme::FAINT,
        );
    }
    let poses: Vec<(usize, Pose)> = (start + 1..*end).filter_map(|line| Some((line, Pose::read(read.text.lines().nth(line - 1)?)?))).collect();
    let all_hidden = !poses.is_empty() && poses.iter().all(|(_, pose)| pose.hidden);
    let all_locked = !poses.is_empty() && poses.iter().all(|(_, pose)| pose.locked);
    let mut changed: Option<Vec<(usize, Pose)>> = None;
    if (pointed || all_hidden)
        && workshop_base::icons::button(ui, action_rect(rect, 0), "the whole group shown or hidden", |painter, at, tint| {
            workshop_base::icons::eye(painter, at, !all_hidden, tint)
        })
    {
        changed = Some(poses.iter().map(|(line, pose)| (*line, Pose { hidden: !all_hidden, ..pose.clone() })).collect());
    }
    if (pointed || all_locked)
        && workshop_base::icons::button(ui, action_rect(rect, 1), "the whole group locked", |painter, at, tint| {
            workshop_base::icons::lock(painter, at, all_locked, tint)
        })
    {
        changed = Some(poses.iter().map(|(line, pose)| (*line, Pose { locked: !all_locked, ..pose.clone() })).collect());
    }
    if let Some(changed) = changed {
        let mut lines: Vec<String> = read.text.lines().map(str::to_string).collect();
        for (line, pose) in changed {
            lines[line - 1] = pose.line();
        }
        asked.made = Some(lines.join("\n") + "\n");
    }
    if drawn.response.clicked() {
        // (As a folder: a click opens or folds it, and chooses all it holds.)
        asked.folding = Some(name.clone());
        asked.chose = Some(chosen_after_click(&read.selection, &drawn.keys, drawn.chosen, read.toggling));
    }
    drawn.response.context_menu(|ui| {
        if ui.button("Frame").clicked() {
            asked.framed = Some(drawn.keys.clone());
            ui.close();
        }
        if ui.button("Rename…  F2").clicked() {
            asked.renaming = Some((*start, name.clone()));
            ui.close();
        }
        if ui.button("Ungroup").clicked() {
            asked.made = Some(edit::ungrouped(&read.text, *start, *end));
            ui.close();
        }
        ui.separator();
        if ui.button(RichText::new("Remove the Group and Its Elements").color(theme::WRONG)).clicked() {
            asked.made = Some(edit::removed_lines(&read.text, *start, *end));
            asked.chose = Some(Vec::new());
            ui.close();
        }
    });
    (end - 1, "  ".to_string())
}

/// A line's row: its badge, its name and what is said after it, its eye and
/// lock; dragged, clicked, its menu. Where a line dropped on it goes (in front of it), unless it is a life.
fn line_row(ui: &mut egui::Ui, decors: &Decors, read: &Read, number: usize, one: &Shown, drawn: &RowDrawn, asked: &mut Asked) -> Option<(usize, String)> {
    let RowDrawn { rect, left, pointed, colour, .. } = *drawn;
    let open = decors.open.as_ref().expect("open");
    paint_badge(ui.painter(), egui::Rect::from_center_size(pos2(left + 22.0, rect.center().y), vec2(15.0, 15.0)), one.badge);
    let faint = one.pose.as_ref().is_some_and(|pose| pose.hidden);
    if !asked.renaming.as_ref().is_some_and(|(line, _)| *line == number) {
        let name_colour = if faint { theme::FAINT } else { colour };
        let galley = ui.painter().layout_no_wrap(one.name.clone(), egui::FontId::proportional(theme::BODY), name_colour);
        let wide = galley.size().x;
        let busy = pointed || one.pose.as_ref().is_some_and(|pose| pose.hidden || pose.locked);
        let clip =
            ui.painter().with_clip_rect(egui::Rect::from_min_max(pos2(left + 32.0, rect.min.y), pos2(rect.max.x - if busy { 46.0 } else { 6.0 }, rect.max.y)));
        clip.galley(pos2(left + 34.0, rect.center().y - galley.size().y / 2.0), galley, name_colour);
        clip.text(
            pos2(left + 40.0 + wide, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &one.description,
            egui::FontId::proportional(theme::SMALL),
            theme::FAINT,
        );
    }
    if let Some(pose) = &one.pose {
        if (pointed || pose.hidden)
            && workshop_base::icons::button(ui, action_rect(rect, 0), "shown or hidden", |painter, at, tint| {
                workshop_base::icons::eye(painter, at, !pose.hidden, tint)
            })
        {
            asked.made = Some(open.with_line(number, Some(Pose { hidden: !pose.hidden, ..pose.clone() }.line())));
        }
        if (pointed || pose.locked)
            && workshop_base::icons::button(ui, action_rect(rect, 1), "locked: the mouse does not take it", |painter, at, tint| {
                workshop_base::icons::lock(painter, at, pose.locked, tint)
            })
        {
            asked.made = Some(open.with_line(number, Some(Pose { locked: !pose.locked, ..pose.clone() }.line())));
        }
    }
    let line = read.text.lines().nth(number - 1).unwrap_or_default();
    let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
    let target = (!edit::is_life(line)).then_some((number, lead));
    if drawn.response.drag_started() && !edit::is_life(line) {
        drawn.response.dnd_set_drag_payload(Dragged(number));
    }
    if drawn.response.clicked() {
        asked.chose = Some(chosen_after_click(&read.selection, &drawn.keys, drawn.chosen, read.toggling));
    }
    drawn.response.context_menu(|ui| line_menu(ui, decors, read, number, one, asked));
    target
}

/// A line's menu: framed, renamed, doubled; put in front, behind, on a
/// layer; out of its group; the chosen ones grouped; removed.
fn line_menu(ui: &mut egui::Ui, decors: &Decors, read: &Read, number: usize, one: &Shown, asked: &mut Asked) {
    let open = decors.open.as_ref().expect("open");
    let text = &read.text;
    if ui.button("Frame").clicked() {
        asked.framed = Some(one.keys.clone());
        ui.close();
    }
    if one.renamed.is_some() && ui.button("Rename…  F2").clicked() {
        asked.renaming = Some((number, one.renamed.clone().unwrap_or_default()));
        ui.close();
    }
    if ui.button("Duplicate").clicked() {
        let lines = edit::copied(text, &one.keys);
        let area = one
            .keys
            .iter()
            .filter_map(|key| read.items.iter().find(|item| (item.line, item.at) == *key))
            .map(|item| decors.item_box(item))
            .reduce(|a, b| a.union(b));
        if let Some(area) = area {
            let (pasted, now) = edit::pasted(text, &lines, [area.min.x + area.width(), area.min.y]);
            asked.made = Some(pasted);
            asked.chose = Some(now);
        }
        ui.close();
    }
    if one.pose.as_ref().is_some_and(|pose| pose.word != "resident") {
        ui.separator();
        if ui.button("In Front").clicked() {
            asked.made = Some(open.moved_line(number, true));
            ui.close();
        }
        if ui.button("Behind").clicked() {
            asked.made = Some(open.moved_line(number, false));
            ui.close();
        }
        if ui.button("All the Way Front").on_hover_text("drawn after all that is put").clicked() {
            asked.made = edit::to_front(text, number, true);
            ui.close();
        }
        if ui.button("All the Way Back").on_hover_text("drawn before all that is put (after the terrains)").clicked() {
            asked.made = edit::to_front(text, number, false);
            ui.close();
        }
        ui.menu_button("Layer", |ui| {
            for (word, name) in [("ground", "Ground"), ("objects", "Objects"), ("over", "Over")] {
                let now = one.keys.first().and_then(|key| read.items.iter().find(|item| (item.line, item.at) == *key)).is_some_and(|item| item.layer == word);
                if ui.selectable_label(now, name).clicked() {
                    asked.made = edit::on_layer(text, number, word);
                    ui.close();
                }
            }
        });
    }
    ui.separator();
    if let Some(at) = one.group
        && ui.button("Leave the Group").clicked()
    {
        asked.made = edit::line_moved(text, number, read.groups[at].2, "");
        ui.close();
    }
    if read.selection.len() > 1 && ui.button("Group the Chosen Elements").clicked() {
        group_chosen(read, asked);
        ui.close();
    }
    if ui.button(RichText::new("Remove").color(theme::WRONG)).clicked() {
        asked.made = Some(edit::removed(text, &one.keys));
        asked.chose = Some(Vec::new());
        ui.close();
    }
}

/// The keys of the list, when it has them: ↑ ↓ the row before, after; ← folds
/// the group (or goes up to it), → opens it; F2 or Enter renames; Delete
/// removes what is chosen; F frames it.
fn list_keys(ui: &egui::Ui, decors: &Decors, read: &Read, shown: &[(usize, Shown)], rows: &[Row], asked: &mut Asked) {
    let keys = ui.input(|input| {
        [
            egui::Key::ArrowUp,
            egui::Key::ArrowDown,
            egui::Key::ArrowLeft,
            egui::Key::ArrowRight,
            egui::Key::Delete,
            egui::Key::F,
            egui::Key::F2,
            egui::Key::Enter,
        ]
        .map(|key| input.key_pressed(key))
    });
    let [up, down, left, right, delete, frame, rename, enter] = keys;
    let groups = &read.groups;
    let now = rows.iter().position(|row| {
        let keys = row_keys(shown, row);
        !keys.is_empty() && read.selection.first().is_some_and(|first| keys.contains(first))
    });
    if (up || down)
        && let Some(row) = rows.get(row_after_arrow(now, up, rows.len()))
    {
        asked.chose = Some(row_keys(shown, row));
    }
    if let Some(row) = now.and_then(|at| rows.get(at)) {
        match row {
            Row::Group(at) => {
                let name = &groups[*at].0;
                if (left && !decors.folded.contains(name)) || (right && decors.folded.contains(name)) {
                    asked.folding = Some(name.clone());
                }
            }
            Row::Line(number) => {
                if left && let Some(at) = groups.iter().position(|(_, start, end)| start < number && number < end) {
                    asked.chose = Some(row_keys(shown, &Row::Group(at)));
                    asked.folding = Some(groups[at].0.clone()).filter(|name| !decors.folded.contains(name));
                }
            }
        }
        if rename || enter {
            asked.renaming = match row {
                Row::Group(at) => Some((groups[*at].1, groups[*at].0.clone())),
                Row::Line(number) => shown.iter().find(|(known, _)| known == number).and_then(|(_, one)| one.renamed.clone()).map(|word| (*number, word)),
            };
        }
    }
    if delete && !read.selection.is_empty() {
        asked.made = Some(edit::removed(&read.text, &read.selection));
        asked.chose = Some(Vec::new());
    }
    if frame && !read.selection.is_empty() {
        asked.framed = Some(read.selection.clone());
    }
}

/// What the Hierarchy did, done: the scene framed on it, chosen, the text changed.
fn apply(decors: &mut Decors, made: Option<String>, chose: Option<Vec<Element>>, framed: Option<Vec<Element>>, refused: Option<String>, items: &[Item]) {
    let area = framed.and_then(|keys| {
        keys.iter().filter_map(|key| items.iter().find(|one| (one.line, one.at) == *key)).map(|one| decors.item_box(one)).reduce(|a, b| a.union(b))
    });
    if refused.is_some() || made.is_some() {
        decors.hier_said = refused;
    }
    let Some(open) = decors.open.as_mut() else { return };
    if let Some(area) = area {
        open.frame_on(area);
    }
    if let Some(chose) = chose {
        open.choose(chose);
    }
    if let Some(made) = made {
        open.change(made);
    }
    decors.hier_seen = decors.open.as_ref().map(|open| open.selection()).unwrap_or_default();
}
