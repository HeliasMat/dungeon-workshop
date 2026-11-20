//! The floor as it shows on the lower screen: its picture at the camera,
//! what lies there, those on it by depth, what is shown for a moment, the
//! dark and the weather over all of it.

use glam::Vec2;
use rules::dungeon::{Pos, Tile, iq};
use rules::run::Monster;
use shell::Laid;
use shell::art_files::SHADOWS;

use super::actors::looks_of;
use super::{SCREEN, TILE, floor, icons, tile_middle};
use crate::Dive;
use crate::{SCALE, Turn};

/// How dark a shadow is, of 255.
const SHADOW: u8 = 110;
/// A shadow's middle under the middle of its monster, in pixels.
const BELOW_FEET: f32 = 3.0;
/// What shows of an invisible monster.
const INVISIBLE: f32 = 0.4;
/// The colour a leader seeing things sees the others in.
const SEEN_DARK: [f32; 3] = [38.0, 38.0, 51.0];

/// The frames a defeated monster blinks.
const FAINT_BLINK: u32 = 36;

/// How a monster is drawn: how much of it, and whether at all.
fn shows(monster: &Monster) -> f32 {
    let blinking = monster.fading.is_some_and(|left| left < FAINT_BLINK && (left / 3).is_multiple_of(2));
    if monster.status.out_of_reach() || blinking {
        0.0
    } else if monster.status.invisible_turns > 0 {
        INVISIBLE
    } else {
        1.0
    }
}

pub fn draw(game: &mut Turn<'_, Dive>) {
    let (world, run, art, dive) = (&mut game.show.world, &mut *game.run, &game.show.art, &mut *game.here);
    let Some(dungeon) = run.dungeon() else {
        return;
    };
    let (book, tables) = (&run.book, &run.book.tables);
    let leader = dungeon.leader();
    dive.picture.refresh(dungeon, book, art);
    dive.looks.weather(dungeon.weather.current, tables);
    // (In whole pixels of the game, for all to stay on its grid.)
    let camera = dive.actors.get(&leader.id).map_or(tile_middle(leader.at), |actor| actor.at).floor();
    let close = SCALE as f32;
    world.layer(&dive.picture.shown, floor::WIDE, floor::HIGH, camera, close, Laid::Whole);
    let half = SCREEN / 2.0;
    let on_screen = |at: Vec2| {
        let at = (at - camera + half).floor() * close;
        (at.x as i32, at.y as i32)
    };
    let near = |at: Pos| {
        let from = (tile_middle(at) - camera).abs();
        from.x < half.x + TILE as f32 && from.y < half.y + TILE as f32
    };

    let looks = &mut dive.looks;
    if near(dungeon.floor.stairs)
        && matches!(dungeon.floor.tile(dungeon.floor.stairs), Tile::Stairs { .. })
        && let Some(stairs) = looks.trap(tables.stairs_tile, tables)
    {
        world.sprite(stairs, on_screen(tile_middle(dungeon.floor.stairs)), close, [255.0; 3], 1.0);
    }
    let seer = iq::has(&leader.stats, iq::TRAP_SEER);
    for trap in dungeon.traps.iter().filter(|trap| (trap.seen || seer) && near(trap.at)) {
        if let Some(picture) = looks.trap(trap.kind, tables) {
            world.sprite(picture, on_screen(tile_middle(trap.at)), close, [255.0; 3], 1.0);
        }
    }
    let lying = dungeon.items.iter().map(|item| (item.at, item.item)).chain(dungeon.spots.iter().filter_map(|spot| Some((spot.at, spot.item?))));
    for (at, item) in lying.filter(|(at, _)| near(*at) && dungeon.floor.tile(*at).is_walkable()) {
        if let Some(picture) = looks.item(item, tables) {
            world.sprite(picture, on_screen(tile_middle(at)), close, [255.0; 3], 1.0);
        }
    }

    // Those on the floor: their shadows first, then the highest first.
    let seeing_things = leader.status.cross_eyed_turns > 0;
    dive.order.clear();
    for monster in dungeon.monsters.iter().filter(|monster| monster.alive() || monster.fading.is_some()) {
        let Some(actor) = dive.actors.get(&monster.id) else {
            continue;
        };
        dive.order.push((actor.at.y as i32, monster.id));
        let (at, Some(picture), Some(size)) =
            (actor.at, actor.picture(), looks_of(monster).and_then(|looks| book.species(looks)).and_then(|species| species.shadow))
        else {
            continue;
        };
        let alpha = shows(monster);
        if alpha > 0.0 {
            let lies = at + Vec2::new(picture.shadow.0 as f32, picture.shadow.1 as f32 + BELOW_FEET);
            world.shadow(on_screen(lies), SHADOWS[(size as usize).min(2)], SCALE as i32, (SHADOW as f32 * alpha) as u8);
        }
    }
    dive.order.sort_unstable();
    for &(_, id) in &dive.order {
        let (Some(monster), Some(actor)) = (dungeon.monster(id), dive.actors.get(&id)) else {
            continue;
        };
        let alpha = shows(monster);
        if alpha <= 0.0 {
            continue;
        }
        let colour = if seeing_things && id != leader.id { SEEN_DARK } else { [255.0; 3] };
        match actor.picture() {
            Some(picture) => world.sprite(looks.frame(picture), on_screen(actor.at), close, colour, alpha),
            // (A stand-in.)
            None => {
                let (x, y) = on_screen(actor.at);
                world.fill(x - 16, y - 16, 32, 32, [200, 80, 80, 255]);
            }
        }
    }
    dive.shows.draw(world, looks, tables, on_screen, camera);

    // A frozen monster is in its block; other icons take turns over it.
    for &(_, id) in &dive.order {
        let (Some(monster), Some(actor)) = (dungeon.monster(id), dive.actors.get_mut(&id)) else {
            continue;
        };
        if !monster.alive() {
            continue;
        }
        let (name, at) = if monster.status.frozen_turns > 0 {
            ("frozen", actor.at)
        } else {
            let names = icons::names(&monster.stats, &monster.status);
            let Some(name) = names.get((actor.icons / icons::TURN) as usize % names.len().max(1)) else {
                actor.icons = 0;
                continue;
            };
            (*name, actor.at - Vec2::new(0.0, icons::ABOVE))
        };
        actor.icons += 1;
        if let Some(frames) = art.status(name).filter(|frames| !frames.is_empty()) {
            world.sprite(&frames[(actor.icons / icons::FRAME) as usize % frames.len()], on_screen(at), close, [255.0; 3], 1.0);
        }
    }
    dive.shows.draw_numbers(world, on_screen);
    dive.dark.draw(world, dungeon, book);
    dive.sky.draw(world, dungeon.weather.current, art, camera, run.frame());
    // (Turning in place: the eight ways around the leader, the one it faces lit.)
    if dive.turning {
        let scale = SCALE as i32;
        for way in [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)] {
            let (x, y) = on_screen(camera + Vec2::new(way.0 as f32, way.1 as f32) * 17.0 - Vec2::new(0.0, 4.0));
            let lit = leader.facing == way;
            let colour = if lit { [0xF8, 0xE0, 0x40, 255] } else { [0xF0, 0xF0, 0xF0, 200] };
            let size = if lit { 4 } else { 3 };
            world.fill(x - (size + 1) * scale, y - (size + 1) * scale, (size * 2 + 2) * scale, (size * 2 + 2) * scale, [0x18, 0x18, 0x20, 220]);
            world.fill(x - size * scale, y - size * scale, size * 2 * scale, size * 2 * scale, colour);
        }
    }
}
