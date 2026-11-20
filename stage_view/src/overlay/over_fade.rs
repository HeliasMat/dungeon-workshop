//! What is drawn over the fade, below the film bands: black coming from a
//! side, the credits, a caption (a title on a dark screen), the words a scene
//! keeps on screen, a fight's log, the gauges.

use super::*;

/// Black come over the screen from a side: see `Beat::Wipe`.
pub(super) fn draw_wipe(canvas: &mut Canvas, stage: &Stage, below: i32) {
    let (wide, high) = (ui::SCREEN_WIDTH as i32, ui::SCREEN_HEIGHT as i32);
    let covered = stage.light.wipe.shown.clamp(0.0, 1.0);
    if covered > 0.0 {
        let (across, down) = ((wide as f32 * covered) as i32, (high as f32 * covered) as i32);
        let (x, y, w, h) = match stage.light.wipe.side {
            0 => (0, 0, across, high),
            1 => (wide - across, 0, across, high),
            2 => (0, 0, wide, down),
            _ => (0, high - down, wide, down),
        };
        canvas.fill(x, below + y, w as usize, h as usize, [0, 0, 0, 255]);
    }
}

/// The credits: their lines going up from below the screen to past its top.
pub(super) fn draw_credits(canvas: &mut Canvas, art: &UiArt, stage: &Stage, below: i32) {
    let (wide, high) = (ui::SCREEN_WIDTH as i32, ui::SCREEN_HEIGHT as i32);
    if let Some(Credits { lines, left, whole }) = &stage.words.credits {
        let gone = 1.0 - (left / whole).clamp(0.0, 1.0);
        let tall = lines.len() as i32 * PITCH;
        let first = below + high - ((high + tall) as f32 * gone) as i32;
        for (row, line) in lines.iter().enumerate() {
            let top = first + row as i32 * PITCH;
            if top >= below && top < below + high - PITCH {
                centred_lines(canvas, art, &[line.as_str()], 0, top, wide, ui::color::WHITE);
            }
        }
    }
}

/// A caption, under the film bands; on a dark screen, in its middle: a
/// title, a story told.
pub(super) fn draw_caption(canvas: &mut Canvas, art: &UiArt, stage: &Stage, below: i32) {
    if let Some(shown) = stage.words.caption.as_ref().map(Caption::shown) {
        let lines: Vec<&str> = shown.split('\n').collect();
        let told = stage.light.fade.level >= 1.0;
        let top = if told { (ui::SCREEN_HEIGHT as i32 - lines.len() as i32 * PITCH) / 2 } else { band(stage) + 12 };
        // (Over the scene, in one of the game's windows, not on a veil; on
        // the dark, the words alone.)
        caption(canvas, art, &lines, if told { 0 } else { 1 }, below + top);
    }
}

/// Where a word kept on screen goes across: on the left, in the middle, on
/// the right.
fn kept_left(at: usize, wide: i32) -> i32 {
    match at {
        0 => 4,
        1 => (ui::SCREEN_WIDTH as i32 - wide) / 2,
        _ => ui::SCREEN_WIDTH as i32 - wide - 4,
    }
}

/// The words a scene keeps on screen: at the top, on the left, in the
/// middle, on the right.
pub(super) fn draw_hud(canvas: &mut Canvas, art: &UiArt, stage: &Stage, below: i32) {
    for (at, hud) in stage.words.hud.iter().enumerate() {
        let Some(hud) = hud else {
            continue;
        };
        let lines: Vec<&str> = hud.split('\n').collect();
        let (wide, high) = (widest(art, &lines) + 8, lines.len() as i32 * PITCH + 4);
        let left = kept_left(at, wide);
        let top = below + band(stage) + 4;
        canvas.fill(left, top, wide as usize, high as usize, [0, 0, 0, 150]);
        for (row, line) in lines.iter().enumerate() {
            canvas.markup(art, left + 4, top + 2 + row as i32 * PITCH, wide as usize, line);
        }
    }
}

/// A fight's log: at the bottom, as in dungeons; over the dialogue box when
/// a line shows in it there, never under it.
pub(super) fn draw_log(canvas: &mut Canvas, art: &UiArt, stage: &Stage, below: i32) {
    const OVER_THE_BOX: i32 = 62;
    const BESIDE_THE_PORTRAIT: i32 = 62;
    let log = &stage.words.log;
    if !log.is_empty() {
        let (wide, high) = (ui::SCREEN_WIDTH as i32, ui::SCREEN_HEIGHT as i32);
        let tall = log.len() as i32 * PITCH + 6;
        // (Over the box, beside the portrait standing on it.)
        let boxed = stage.words.boxed && !stage.words.box_top;
        let (left, top) = match boxed {
            true => (BESIDE_THE_PORTRAIT, below + high - band(stage) - tall - OVER_THE_BOX),
            false => (0, below + high - band(stage) - tall),
        };
        canvas.fill(left, top, (wide - left) as usize, tall as usize, [0, 0, 0, 150]);
        for (row, line) in log.iter().enumerate() {
            canvas.markup(art, left + 8, top + 3 + row as i32 * PITCH, (wide - left) as usize - 16, &line.text);
        }
    }
}

/// The gauges: under the words of their place, if any.
pub(super) fn draw_gauges(canvas: &mut Canvas, stage: &Stage, below: i32) {
    const GAUGE: i32 = 64;
    for (at, gauge) in stage.words.gauges.iter().enumerate() {
        let Some(Gauge { full, colour: [red, green, blue] }) = gauge else {
            continue;
        };
        let words = stage.words.hud[at].as_ref().map_or(0, |hud| hud.split('\n').count() as i32 * PITCH + 6);
        let top = below + band(stage) + 4 + words;
        let left = kept_left(at, GAUGE);
        canvas.fill(left - 1, top - 1, (GAUGE + 2) as usize, 8, [0, 0, 0, 220]);
        canvas.fill(left, top, GAUGE as usize, 6, [60, 60, 60, 255]);
        canvas.fill(left, top, (GAUGE as f32 * full) as usize, 6, [*red, *green, *blue, 255]);
    }
}

/// A dungeon's map in the top right corner, as the game's: a pixel a tile of
/// the ground one may walk on, its edge lighter; the team in yellow, the
/// others in red, the stairs in white, what lies about in blue. A place
/// too wide is shown a tile in two, or more.
pub(super) fn draw_minimap(canvas: &mut Canvas, stage: &Stage, below: i32) {
    const WIDEST: i32 = 96;
    let (Some(team), Some(ground)) = (&stage.words.minimap, &stage.place.ground) else {
        return;
    };
    let rows = ground.rows();
    let (high, wide) = (rows.len() as i32, rows.first().map_or(0, Vec::len) as i32);
    let step = ((wide + WIDEST - 1) / WIDEST).max(1);
    let (map_wide, map_high) = (wide / step, high / step);
    let left = ui::SCREEN_WIDTH as i32 - map_wide - 4;
    let top = below + band(stage) + 4;

    // (Found as one goes: only the tiles seen.)
    let seen = |x: i32, y: i32| stage.words.minimap_seen.as_ref().is_none_or(|seen| seen[(y * step) as usize][(x * step) as usize]);
    let free = |x: i32, y: i32| (0..map_wide).contains(&x) && (0..map_high).contains(&y) && !rows[(y * step) as usize][(x * step) as usize] && seen(x, y);
    let seen_at = |place: glam::Vec2| {
        let (x, y) = ((place.x / (TILE * step as f32)) as i32, (place.y / (TILE * step as f32)) as i32);
        (0..map_wide).contains(&x) && (0..map_high).contains(&y) && seen(x, y)
    };
    // (Each tile on a dark ground a pixel wider, so that it reads on any
    // floor, the icy ones too; nothing over what is not found yet.)
    for y in 0..map_high {
        for x in 0..map_wide {
            if free(x, y) {
                canvas.fill(left + x - 1, top + y - 1, 3, 3, [0, 0, 0, 40]);
            }
        }
    }
    for y in 0..map_high {
        for x in 0..map_wide {
            if free(x, y) {
                let edge = !(free(x - 1, y) && free(x + 1, y) && free(x, y - 1) && free(x, y + 1));
                let colour = if edge { [150, 190, 255, 190] } else { [60, 110, 220, 120] };
                canvas.fill(left + x, top + y, 1, 1, colour);
            }
        }
    }
    let at = |place: glam::Vec2| (left + (place.x / (TILE * step as f32)) as i32, top + (place.y / (TILE * step as f32)) as i32);
    for thing in stage.things().iter().filter(|thing| thing.owner.is_none() && !thing.name.is_empty()) {
        if stage.cast.get(&thing.id).is_some_and(|actor| actor.shown_opacity() < 0.5) || !seen_at(thing.place) {
            continue;
        }
        let (x, y) = at(thing.place);
        match thing.name.as_str() {
            "stairs" => canvas.fill(x - 1, y - 1, 3, 3, [255, 255, 255, 255]),
            _ => canvas.fill(x, y, 2, 2, [80, 230, 255, 255]),
        }
    }
    for (who, actor) in stage.cast.iter().filter(|(who, _)| !who.is_thing()) {
        if actor.shown_opacity() < 0.5 || actor.attributes & engine::attribute::HIDDEN != 0 || !(team.contains(who) || seen_at(actor.position)) {
            continue;
        }
        let (x, y) = at(actor.position);
        let colour = if team.contains(who) { [255, 230, 60, 255] } else { [255, 70, 60, 255] };
        canvas.fill(x - 1, y - 1, 2, 2, colour);
    }
}
