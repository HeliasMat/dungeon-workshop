//! The dungeon's interface, like the original's, on the DS's two screens:
//! above, the controls; below, over the floor, the top line with the HP
//! bar, the map, the message window and the menus.

use rules::dungeon::minimap::{self, CELL, MAP_HEIGHT, MAP_WIDTH};
use rules::dungeon::{Pos, ai, iq};
use rules::run::dungeon::{BANNER_FADE, Dungeon};
use rules::run::{Run, Side};
use stage_view::ui::{self, Canvas, SCREEN_HEIGHT, SCREEN_WIDTH, TILE, UiArt, color};

use super::{bag, pages, talk};
use crate::Dive;
use crate::Turn;

const TEAM: [u8; 4] = [248, 248, 0, 255];
/// Who waits for the team, once the place is seen.
const CLIENT: [u8; 4] = [248, 160, 0, 255];
const ENEMY: [u8; 4] = [248, 0, 0, 255];
const ITEM: [u8; 4] = [80, 220, 255, 255];
const TRAP: [u8; 4] = [216, 112, 248, 255];

/// The top screen, drawn once for the frames' look of the moment.
#[derive(Default)]
pub struct Top {
    drawn: Option<(ui::Frame, usize)>,
    pixels: Vec<u8>,
}

impl Top {
    fn draw(&mut self, canvas: &mut Canvas, art: &UiArt) {
        let key = (canvas.frame, canvas.frame_type);
        let size = SCREEN_WIDTH * SCREEN_HEIGHT * 4;
        if self.drawn == Some(key) {
            canvas.data[..size].copy_from_slice(&self.pixels);
            return;
        }
        canvas.backdrop(art);
        canvas.window(art, 2, 2, 28, 20);
        canvas.markup(art, (2 * TILE) as i32 + 4, (2 * TILE) as i32, 28 * TILE - 8, &art.strings.controls);
        self.pixels = canvas.data[..size].to_vec();
        self.drawn = Some(key);
    }
}

/// The map as last drawn.
#[derive(Default)]
pub struct Map {
    /// Drawn for that floor, turn and leader's tile.
    drawn: Option<(u32, u32, Pos)>,
    picture: Vec<u8>,
}

impl Map {
    /// Redraws the map after each turn: the team in yellow, the enemies in
    /// sight in red, the items seen in blue, the traps found in pink.
    fn refresh(&mut self, dungeon: &Dungeon) {
        let leader = dungeon.leader();
        let key = (dungeon.generation, dungeon.turn, leader.at);
        if self.drawn == Some(key) {
            return;
        }
        self.drawn = Some(key);
        let (floor, explored) = (&dungeon.floor, &dungeon.explored);
        let specs = dungeon.specs_map();
        let items = dungeon.items.iter().filter(|item| specs.unwrap_or(explored.seen(item.at)) && floor.tile(item.at).is_walkable());
        let mut dots: Vec<(Pos, [u8; 4])> = items.map(|item| (item.at, ITEM)).collect();
        for monster in dungeon.monsters.iter().filter(|monster| monster.alive()) {
            if monster.team() {
                // (The leader's dot blinks, drawn apart.)
                if monster.side != Side::Leader {
                    dots.push((monster.at, TEAM));
                }
            } else if specs.unwrap_or(dungeon.radar || ai::can_see(floor, leader.at, monster.at)) {
                dots.push((monster.at, ENEMY));
            }
            if monster.job_target.is_some() && monster.side != Side::Foe && explored.seen(monster.at) {
                dots.push((monster.at, CLIENT));
            }
        }
        dots.extend(dungeon.spots.iter().filter(|spot| explored.seen(spot.at)).map(|spot| (spot.at, CLIENT)));
        // The traps found, and on a floor whose leader has Trap Seer all.
        let seer = iq::has(&leader.stats, iq::TRAP_SEER);
        dots.extend(dungeon.traps.iter().filter(|trap| seer || trap.seen).map(|trap| (trap.at, TRAP)));
        self.picture = minimap::draw(floor, explored, &dots);
    }
}

/// The leader's troubles, spelled out under the top line.
fn flags(dungeon: &Dungeon) -> String {
    let status = &dungeon.leader().status;
    let mut flags: Vec<&str> = Vec::new();
    if status.asleep() {
        flags.push("asleep");
    }
    if status.poisoned {
        flags.push("poisoned");
    }
    let lasting = [
        (status.burn_turns, "burned"),
        (status.paralysis_turns, "paralyzed"),
        (status.frozen_turns, "frozen"),
        (status.cringe_turns, "cringing"),
        (status.confused_turns, "confused"),
        (status.cursed_turns, "cursed"),
    ];
    flags.extend(lasting.iter().filter(|(turns, _)| *turns > 0).map(|(_, label)| *label));
    flags.join("  ")
}

/// A new floor: its name on black, as the original shows it.
fn banner(canvas: &mut Canvas, art: &UiArt, dungeon: &Dungeon) {
    if dungeon.banner == 0 {
        return;
    }
    let screen = SCREEN_HEIGHT as i32;
    let alpha = (dungeon.banner as f32 / BANNER_FADE as f32).min(1.0);
    canvas.fill(0, screen, SCREEN_WIDTH, SCREEN_HEIGHT, [0, 0, 0, (alpha * 255.0) as u8]);
    if dungeon.banner > BANNER_FADE {
        let floor = match dungeon.goes_up {
            true => format!("{}F", dungeon.floor_number),
            false => format!("B{}F", dungeon.floor_number),
        };
        for (line, y) in [(dungeon.name.as_str(), 80), (floor.as_str(), 98)] {
            let wide = art.font.width(line) as i32;
            canvas.text(art, (SCREEN_WIDTH as i32 - wide) / 2, screen + y, line, color::WHITE);
        }
    }
}

fn map_and_top_line(canvas: &mut Canvas, art: &UiArt, run: &Run, dungeon: &Dungeon, map: &Map, menu_open: bool) {
    let screen = SCREEN_HEIGHT as i32;
    let leader = dungeon.leader();
    // The map lies over the floor, 4 pixels a tile, from 8 pixels in; the
    // leader's dot blinks.
    if !menu_open && run.options().map && !map.picture.is_empty() {
        canvas.blit(8, screen, MAP_WIDTH, MAP_HEIGHT, &map.picture);
        if (run.frame() / 30).is_multiple_of(2) {
            let cell = CELL as i32;
            canvas.blit(8 + leader.at.x * cell, screen + leader.at.y * cell, CELL, CELL, &minimap::draw_dot(TEAM));
        }
    }
    let stats = &leader.stats;
    canvas.hud(art, screen, dungeon.floor_number, dungeon.goes_up, stats.level, stats.hp, stats.max_hp);
    let flags = flags(dungeon);
    if !flags.is_empty() {
        canvas.text(art, 8, screen + 10, &flags, color::WHITE);
    }
}

pub fn draw(game: &mut Turn<'_, Dive>) {
    let (run, dive, canvas, ui) = (&mut *game.run, &mut *game.here, &mut game.show.canvas, &game.show.ui);
    let (Some(dungeon), Some(art)) = (run.dungeon(), ui.clone()) else {
        return;
    };
    let art = art.as_ref();
    let menu_open = dive.pause.is_open() || dive.bag.is_open();
    dive.top.draw(canvas, art);
    dive.map.refresh(dungeon);
    map_and_top_line(canvas, art, run, dungeon, &dive.map, menu_open);
    dive.messages.advance(run);
    if !menu_open {
        dive.messages.draw(canvas, art, run);
    }
    pages::draw(canvas, art, run, &dive.pause);
    banner(canvas, art, dungeon);
    talk::draw(canvas, art, run, &dive.talk);
    bag::draw(canvas, art, run, &dive.bag);
}
