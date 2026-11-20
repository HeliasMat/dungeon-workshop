//! The pages of the dungeon's menu as they are drawn. Window rectangles
//! come from the game's tables (`rom_formats::menus`); text positions were measured
//! on the original running in an emulator.

use rules::dungeon::Belly;
use rules::run::job_floor::job_title;
use rules::run::{Dungeon, Monster, Run, tactic};
use stage_view::ui::{BOTTOM, Canvas, HINTS, Rows, SCREEN_HEIGHT, TILE, UiArt, color, mark, text};

use super::pause::{Page, Pause, allies, iq_skills, member_of, shown_monster};

/// A row of a menu, and its colour.
type Row<'a> = (&'a str, [u8; 3]);

const TEAM_COLOURS: [[u8; 3]; 2] = [color::BLUE, color::YELLOW];
/// An IQ skill switched off.
const OFF_GREY: [u8; 3] = [136, 136, 136];

pub fn draw(canvas: &mut Canvas, ui: &UiArt, run: &Run, pause: &Pause) {
    let Some(dungeon) = run.dungeon() else {
        return;
    };
    let team = allies(run);
    match pause.page {
        Page::Closed => {}
        Page::Main => main_page(canvas, ui, run, dungeon, pause, &team),
        Page::Moves => moves_page(canvas, ui, run, pause),
        Page::Team => team_page(canvas, ui, run, dungeon.leader(), &team, pause),
        Page::Others => {
            let rows: Vec<Row> = text::OTHERS.map(|index| (ui.label(index), color::WHITE)).collect();
            canvas.titled_menu(ui, (2, BOTTOM + 2, 14), (ui.label(text::OTHERS_TITLE), color::WHITE), &rows, 0, Some(pause.cursor.at), 12);
        }
        Page::Iq(member) => iq_page(canvas, ui, run, member, pause),
        Page::Tactics(member) => tactics_page(canvas, ui, run, team.get(member - 1).copied(), pause),
        Page::Log => {
            // Eight lines at a time, from the cursor's back in time.
            const SHOWN: usize = 8;
            let log = &run.log.lines;
            let lines: Vec<Row> = log.iter().rev().skip(pause.cursor.at).take(SHOWN).rev().map(|line| (line.as_str(), color::WHITE)).collect();
            let title = format!("{}  ({}/{})", ui.label(text::OTHERS.start + 3), pause.cursor.at + 1, log.len().max(1));
            canvas.titled_menu(ui, (1, BOTTOM + 2, 30), (&title, color::WHITE), &lines, SHOWN, None, 4);
        }
        Page::Options => {
            let rows = run.options().rows();
            let rows: Vec<Row> = rows.iter().map(|row| (row.as_str(), color::WHITE)).collect();
            canvas.titled_menu(ui, (2, BOTTOM + 2, 16), (ui.label(text::OTHERS.start), color::WHITE), &rows, 0, Some(pause.cursor.at), 12);
        }
        Page::Hints => {
            let rows: Vec<Row> = (0..HINTS).map(|at| (ui.strings.hints.get(at).map_or("", String::as_str), color::WHITE)).collect();
            canvas.titled_menu(ui, (2, BOTTOM + 2, 18), (ui.label(text::OTHERS.start + 6), color::WHITE), &rows, 0, Some(pause.cursor.at), 12);
        }
        Page::Hint(hint) => {
            // (Its title, then its text in a window of its own.)
            let (screen, left, strings) = (SCREEN_HEIGHT as i32, (2 * TILE) as i32, &ui.strings);
            canvas.window(ui, 1, BOTTOM + 1, 30, 22);
            canvas.text(ui, left - 4, screen + 16, strings.hints.get(hint).map_or("", String::as_str), color::YELLOW);
            canvas.markup(ui, left - 4, screen + 34, 30 * TILE - 16, strings.hints.get(HINTS + hint).map_or("", String::as_str));
        }
        Page::Objectives | Page::Recruitable => listed_page(canvas, ui, run, dungeon, pause),
        Page::Rest => {
            let mut rows: Vec<Row> = text::REST.map(|index| (ui.label(index), color::WHITE)).collect();
            rows.push(("Save and Quit", color::WHITE));
            canvas.titled_menu(ui, (2, BOTTOM + 2, 14), (ui.label(text::REST_TITLE), color::WHITE), &rows, 0, Some(pause.cursor.at), 12);
        }
    }
}

/// A member's moves, their PP, what the buttons do.
fn moves_page(canvas: &mut Canvas, ui: &UiArt, run: &Run, pause: &Pause) {
    let Some(member) = shown_monster(pause, run) else {
        return;
    };
    let (book, strings) = (&run.book, &ui.strings);
    let (screen, left) = (SCREEN_HEIGHT as i32, (2 * TILE) as i32);
    let moves = &member.moves;
    let title = strings.moves_title.replace("[string:0]", &member.name);
    // (A move linked to the one before: "+", in yellow.)
    let names: Vec<String> = moves.iter().map(|slot| if slot.linked { format!("+{}", book.move_name(slot.id)) } else { book.move_name(slot.id) }).collect();
    let rows: Vec<Row> = names.iter().zip(moves).map(|(row, slot)| (row.as_str(), if slot.linked { color::YELLOW } else { color::GREEN })).collect();
    let lines = canvas.titled_menu(ui, (2, BOTTOM + 2, 18), (&title, color::WHITE), &rows, 4, Some(pause.cursor.at), 16);
    let pp: Vec<String> = moves.iter().map(|slot| format!("{}/{}", slot.pp, slot.max_pp)).collect();
    let pp: Vec<Row> = pp.iter().map(|pp| (pp.as_str(), color::GREEN)).collect();
    canvas.right_column(ui, left + 140, &lines, &pp);
    for (row, &line) in lines.iter().enumerate() {
        if row + 1 < lines.len() {
            canvas.separator(left + 16, line + 14, 18 * TILE - 18);
        }
    }
    // What the buttons do here.
    canvas.window(ui, 2, BOTTOM + 16, 28, 4);
    canvas.markup(ui, left + 4, screen + 130, 28 * TILE - 8, &strings.moves_hint);
}

/// The team; the chosen member's place in it, what it holds, its IQ, its types.
fn team_page(canvas: &mut Canvas, ui: &UiArt, run: &Run, leader: &Monster, team: &[&Monster], pause: &Pause) {
    let (book, strings) = (&run.book, &ui.strings);
    let (screen, left) = (SCREEN_HEIGHT as i32, (2 * TILE) as i32);
    let members: Vec<&Monster> = std::iter::once(leader).chain(team.iter().copied()).collect();
    let rows: Vec<Row> = members.iter().enumerate().map(|(row, member)| (member.name.as_str(), TEAM_COLOURS[row.min(1)])).collect();
    let cursor = pause.cursor.at.min(rows.len().saturating_sub(1));
    let lines = canvas.titled_menu(ui, (2, BOTTOM + 2, 15), (&strings.team_name, color::PINK), &rows, 0, Some(cursor), 12);
    for line in lines {
        canvas.mark(ui, left + 80, line, mark::HEART);
    }
    // The selected member's place in the team, what it holds and its IQ.
    let below = 2 + 2 * (rows.len() + 2) + 2;
    let stats = &members[cursor].stats;
    let held = match stats.held {
        0 => "nothing".to_string(),
        item => book.item_name(item),
    };
    canvas.window(ui, 2, BOTTOM + below + 3, 22, 2);
    canvas.text(
        ui,
        left + 4,
        screen + ((below + 3) * TILE) as i32 + 2,
        &format!("Holds: {held}   IQ {}  ({} skills, I)", stats.iq, stats.iq_skills.count_ones()),
        color::WHITE,
    );
    canvas.window(ui, 2, BOTTOM + below, 15, 2);
    canvas.text(ui, left + 4, screen + (below * TILE) as i32 + 2, &strings.leader, color::WHITE);
    // Its types, each by its badge (the Fairy type's among them).
    canvas.window(ui, 18, BOTTOM + below, 12, 2);
    let (first, second) = stats.types;
    let mut x = (18 * TILE) as i32 + 4;
    for kind in [first, second].into_iter().filter(|&kind| kind != 0) {
        x += type_badge(canvas, ui, x, screen + (below * TILE) as i32 + 3, kind) + 6;
    }
}

/// A member's IQ skills: eight at a time, those switched off in grey.
fn iq_page(canvas: &mut Canvas, ui: &UiArt, run: &Run, member: usize, pause: &Pause) {
    const SHOWN: usize = 8;
    let Some(monster) = member_of(run, member) else {
        return;
    };
    let book = &run.book;
    let skills = iq_skills(run, member);
    let first = pause.cursor.first(SHOWN);
    let names: Vec<String> = skills
        .iter()
        .skip(first)
        .take(SHOWN)
        .map(|&skill| {
            let name = book.tables.iq_skills.get(skill as usize).map_or("?", String::as_str);
            format!("{} {name}", if run.iq_is_off(&monster.name, skill) { "-" } else { "+" })
        })
        .collect();
    let colours = skills.iter().skip(first).take(SHOWN).map(|&skill| if run.iq_is_off(&monster.name, skill) { OFF_GREY } else { color::GREEN });
    let rows: Vec<Row> = names.iter().map(String::as_str).zip(colours).collect();
    let empty = [("No IQ skills.", color::WHITE)];
    let rows: &[Row] = if rows.is_empty() { &empty } else { &rows };
    let title = format!("{}'s IQ", monster.name);
    canvas.titled_menu(ui, (2, BOTTOM + 2, 26), (&title, color::YELLOW), rows, SHOWN, Some(pause.cursor.at - first), 12);
}

/// An ally's tactics: six at a time, then what the chosen one does.
fn tactics_page(canvas: &mut Canvas, ui: &UiArt, run: &Run, ally: Option<&Monster>, pause: &Pause) {
    const SHOWN: usize = 6;
    let book = &run.book;
    let (screen, left) = (SCREEN_HEIGHT as i32, (2 * TILE) as i32);
    let unlocked = tactic::unlocked(ally.map_or(1, |ally| ally.stats.level));
    let first = pause.cursor.first(SHOWN);
    let names = &book.tables.tactics;
    let rows: Vec<Row> =
        unlocked.iter().skip(first).take(SHOWN).filter_map(|&tactic| names.get(tactic as usize)).map(|name| (name.as_str(), color::WHITE)).collect();
    canvas.titled_menu(ui, (2, BOTTOM + 2, 16), (ally.map_or("", |ally| ally.name.as_str()), color::YELLOW), &rows, SHOWN, Some(pause.cursor.at - first), 12);
    canvas.window(ui, 2, BOTTOM + 18, 28, 5);
    let about = unlocked.get(pause.cursor.at).and_then(|&tactic| book.tables.tactic_descriptions.get(tactic as usize));
    // (Up to its first tag: what follows is about the menu.)
    let about = about.map_or("", |about| about.split('[').next().unwrap_or("").trim());
    for (row, line) in about.lines().take(3).enumerate() {
        canvas.text(ui, left + 2, screen + (18 * TILE) as i32 + 3 + row as i32 * 13, line, color::WHITE);
    }
}

/// The jobs taken (done or not); the species of this floor that may join.
fn listed_page(canvas: &mut Canvas, ui: &UiArt, run: &Run, dungeon: &Dungeon, pause: &Pause) {
    let book = &run.book;
    let lines: Vec<String> = match pause.page {
        Page::Objectives => run.team().progress.jobs.iter().map(|job| format!("{}{}", if job.done { "Done: " } else { "" }, job_title(job, book))).collect(),
        _ => {
            let mut seen: Vec<String> = Vec::new();
            for enemy in dungeon.spec(book).map_or(&[][..], |floor| &floor.enemies[..]) {
                let name = book.species_name(enemy.species);
                if enemy.recruit_rate > 0 && !seen.contains(&name) {
                    seen.push(name);
                }
            }
            seen
        }
    };
    let lines = if lines.is_empty() { vec!["None.".to_string()] } else { lines };
    let rows: Vec<Row> = lines.iter().take(8).map(|line| (line.as_str(), color::WHITE)).collect();
    let title = ui.label(text::OTHERS.start + if pause.page == Page::Objectives { 4 } else { 5 });
    canvas.titled_menu(ui, (1, BOTTOM + 2, 30), (title, color::WHITE), &rows, 8, None, 4);
}

/// A type's badge, drawn as the game's pixels are (no soft edge): a square
/// of its colour, darker round, the Fairy type's with a star in it; then its
/// name. How wide it was.
fn type_badge(canvas: &mut Canvas, ui: &UiArt, x: i32, y: i32, kind: u8) -> i32 {
    let [r, g, b] = rules::dungeon::TYPE_COLOURS.get(kind as usize).copied().unwrap_or([136, 136, 136]);
    let dark = [r / 2, g / 2, b / 2, 255];
    canvas.fill(x, y + 1, 9, 9, dark);
    canvas.fill(x + 1, y + 2, 7, 7, [r, g, b, 255]);
    if kind == rules::dungeon::FAIRY {
        // (A star of five pixels and its points.)
        let white = [255, 255, 255, 255];
        canvas.fill(x + 4, y + 3, 1, 5, white);
        canvas.fill(x + 2, y + 5, 5, 1, white);
        canvas.fill(x + 3, y + 4, 3, 3, white);
    }
    11 + canvas.text(ui, x + 11, y - 1, rules::dungeon::type_name(kind), [r.max(96), g.max(96), b.max(96)]) as i32
}

/// The list of the main menu, where the team is, its HP, then belly, money,
/// weather and play time.
fn main_page(canvas: &mut Canvas, ui: &UiArt, run: &Run, dungeon: &Dungeon, pause: &Pause, team: &[&Monster]) {
    let screen = SCREEN_HEIGHT as i32;
    let (left, top) = ((2 * TILE) as i32, screen + (2 * TILE) as i32);
    let leader = dungeon.leader();
    canvas.window(ui, 2, BOTTOM + 2, 7, 12);
    let entries: Vec<Row> = text::MAIN.iter().map(|&entry| (ui.label(entry), color::WHITE)).collect();
    canvas.rows(ui, Rows { left, top, pitch: 14, indent: 13 }, &entries, Some(pause.cursor.at));
    canvas.window(ui, 11, BOTTOM + 4, 19, 2);
    let width = ui.font.width(&dungeon.name) as i32;
    let centre = (11 * TILE + 19 * TILE / 2) as i32;
    canvas.text(ui, centre - width / 2, screen + 34, &dungeon.name, color::ORANGE);
    canvas.window(ui, 2, BOTTOM + 16, 28, 6);
    let members: Vec<&Monster> = std::iter::once(leader).chain(team.iter().copied()).collect();
    let names: Vec<Row> = members.iter().enumerate().map(|(row, member)| (member.name.as_str(), TEAM_COLOURS[row.min(1)])).collect();
    let lines = canvas.rows(ui, Rows { left: left + 4, top: screen + 128, pitch: 12, indent: 0 }, &names, None);
    let hp: Vec<String> = members.iter().map(|member| format!("{:>3}/{:>3}", member.stats.hp.max(0), member.stats.max_hp)).collect();
    let hp: Vec<Row> = hp.iter().map(|hp| (hp.as_str(), color::WHITE)).collect();
    canvas.right_column(ui, left + 109, &lines, &hp);
    let seconds = run.frame() / 60;
    let weather = ui.strings.weather.get(dungeon.weather.current as usize).map_or("", String::as_str);
    let belly = leader.belly.as_ref().map_or_else(|| (Belly::full().points(), Belly::full().max_points()), |belly| (belly.points(), belly.max_points()));
    let values = [
        (text::BELLY, format!("{}/{}", belly.0, belly.1), color::WHITE),
        (text::MONEY, run.bag().money.to_string(), color::CYAN),
        (text::WEATHER, weather.to_string(), color::WHITE),
        (text::PLAY_TIME, format!("{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60), color::WHITE),
    ];
    for (row, (label, value, value_color)) in values.iter().enumerate() {
        let y = screen + 128 + row as i32 * 12;
        canvas.text(ui, left + 115, y, ui.label(*label), color::WHITE);
        let end = left + 168 + canvas.text(ui, left + 168, y, value, *value_color) as i32;
        if *label == text::MONEY {
            canvas.mark(ui, end + 1, y, mark::POKE);
        }
    }
}
