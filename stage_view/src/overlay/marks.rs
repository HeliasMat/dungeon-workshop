//! What is drawn over the stage: lights, bolts, pictures, icons, the air.

use super::*;

/// The dark around a spotlight's circle, its edge softer.
pub(super) fn draw_spotlight(canvas: &mut Canvas, spotlight: &Spotlight, below: i32, close: f32, on_screen: &impl Fn(Vec2) -> (i32, i32)) {
    /// How dark it is around, at the most; how wide its edge, where the dark
    /// falls one pixel in two.
    const DARK: f32 = 235.0;
    const EDGE: f32 = 8.0;
    if spotlight.shown <= 0.0 {
        return;
    }
    let (wide, high) = (ui::SCREEN_WIDTH as i32, ui::SCREEN_HEIGHT as i32);
    let (x, y) = on_screen(spotlight.at);
    let (inner, outer) = (spotlight.radius * close, spotlight.radius * close + EDGE);
    // (An iris is wholly black around.)
    let darkest = if spotlight.black { 255.0 } else { DARK };
    let dark = [0, 0, 0, (darkest * spotlight.shown) as u8];
    // How far either side of the middle a circle reaches on a row.
    let half = |radius: f32, row: f32| (radius * radius - row * row).max(0.0).sqrt() as i32;
    for row in below..below + high {
        let from_middle = (row - y) as f32;
        if from_middle.abs() >= outer {
            canvas.fill(0, row, wide as usize, 1, dark);
            continue;
        }
        let (near, far) = (half(inner, from_middle), half(outer, from_middle));
        let (left, right) = ((x - far).clamp(0, wide), (x + far).clamp(0, wide));
        canvas.fill(0, row, left as usize, 1, dark);
        canvas.fill(right, row, (wide - right) as usize, 1, dark);
        let (lit_left, lit_right) = ((x - near).clamp(left, right), (x + near).clamp(left, right));
        for pixel in (left..lit_left).chain(lit_right..right).filter(|pixel| (pixel + row) % 2 == 0) {
            canvas.fill(pixel, row, 1, 1, dark);
        }
    }
}

/// The pictures a scene puts about (see `Beat::PlacedPicture`), each as
/// `draw_picture` draws it.
pub(super) fn draw_pictures(canvas: &mut Canvas, stage: &Stage, close: f32, seconds: f32, on_screen: &impl Fn(Vec2) -> (i32, i32)) {
    for at in 0..stage.words.pictures.len() {
        draw_picture(canvas, stage, at, close, seconds, on_screen);
    }
}

/// A picture the scene puts about, by its rank among them: from its file
/// (read once), its middle on its point, as big as the view is close, and
/// as its node has it (turned, sized, slanted about its pivot, flipped,
/// tinted, a colour added or filling it, mixed, smoothed; its effects, the
/// time gone `seconds`), with what it takes from those it hangs on.
pub(super) fn draw_picture(canvas: &mut Canvas, stage: &Stage, at: usize, close: f32, seconds: f32, on_screen: &impl Fn(Vec2) -> (i32, i32)) {
    let Some(placed) = stage.words.pictures.get(at) else {
        return;
    };
    let Some(picture) = picture_file(&placed.file) else {
        return;
    };
    let node = stage.node(&NodeRef::Named(placed.name.clone())).unwrap_or_else(|| node::Node::new(placed.name.clone(), props::NodeKind::Picture));
    if !node.visible {
        return;
    }
    let (wide, high, rgba) = &*picture;
    let (x, y) = on_screen(node.transform.pos);
    let middle = Vec2::new(x as f32, y as f32);
    let size = Vec2::new(*wide as f32, *high as f32);
    let map = glam::Affine2::from_translation(middle)
        * glam::Affine2::from_scale(Vec2::splat(close))
        * node.transform.about_pivot()
        * glam::Affine2::from_translation(-size / 2.0);
    let [red, green, blue, alpha] = node.looks.tint;
    let (fx, around) = crate::stage::effects_of(&node.looks.effects, close, seconds);
    let paint = shell::Paint { fx, ..crate::stage::paint_of(&node.looks, [red, green, blue].map(f32::from), node.looks.opacity * alpha as f32 / 255.0) };
    shell::blit_with(&mut canvas.data, (ui::SCREEN_WIDTH, ui::CANVAS_HEIGHT), rgba, (*wide as usize, *high as usize), map, &paint, &around);
}

/// Bolts of lightning (see `Beat::Lightning`): a zigzag from the top of
/// the screen down to where they strike, white with a pale glow, gone in
/// a few frames.
pub(super) fn draw_bolts(canvas: &mut Canvas, stage: &Stage, below: i32, close: f32, on_screen: &impl Fn(Vec2) -> (i32, i32)) {
    const STEPS: i32 = 10;
    for &Bolt { at, left, drawn } in &stage.light.bolts {
        let (x, y) = on_screen(at);
        let alpha = (left / 14.0).clamp(0.0, 1.0);
        let mut seed = drawn | 1;
        let mut from = (x + ((seed % 40) as i32 - 20), below);
        for step in 1..=STEPS {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let sway = if step == STEPS { 0 } else { ((seed >> 16) % 24) as i32 - 12 };
            let to = (x + (sway as f32 * close) as i32, below + (y - below) * step / STEPS);
            // (The segment: a few points along it, each a small square.)
            let length = (to.0 - from.0).abs().max(to.1 - from.1).max(1);
            for point in 0..=length {
                let (px, py) = (from.0 + (to.0 - from.0) * point / length, from.1 + (to.1 - from.1) * point / length);
                canvas.fill(px - 2, py, 5, 1, [200, 220, 255, (90.0 * alpha) as u8]);
                canvas.fill(px - 1, py, 2, 1, [255, 255, 255, (255.0 * alpha) as u8]);
            }
            from = to;
        }
    }
}

/// The keys shown on screen (see `Beat::Icon`): a key cap over its
/// bearer's head or on its point, an arrow drawn in it, or its name (`OK`
/// for Enter, a letter).
pub(super) fn draw_icons(canvas: &mut Canvas, art: &UiArt, stage: &Stage, on_screen: &impl Fn(Vec2) -> (i32, i32)) {
    const SIZE: i32 = 15;
    for Icon { on, key, .. } in &stage.words.icons {
        let at = match on {
            Cam::At(at) => Some(at.vec2()),
            Cam::Who(who) => stage.cast.get(who).map(|actor| actor.shown_at() - Vec2::new(0.0, 34.0 + actor.shown_height())),
        };
        let Some(at) = at else {
            continue;
        };
        let (x, y) = on_screen(at);
        let (left, top) = (x - SIZE / 2, y - SIZE / 2);
        canvas.fill(left - 1, top - 1, SIZE as usize + 2, SIZE as usize + 2, [250, 250, 250, 255]);
        canvas.fill(left, top, SIZE as usize, SIZE as usize, [40, 48, 72, 255]);
        let ink = [255, 255, 255, 255];
        // (An arrow: a triangle row by row, then its stem.)
        let arrow = |canvas: &mut Canvas, way: (i32, i32)| {
            for row in 0..5 {
                let wide = 1 + row * 2;
                match way {
                    (0, -1) => canvas.fill(x - row, y - 4 + row, wide as usize, 1, ink),
                    (0, 1) => canvas.fill(x - row, y + 4 - row, wide as usize, 1, ink),
                    (-1, 0) => canvas.fill(x - 4 + row, y - row, 1, wide as usize, ink),
                    _ => canvas.fill(x + 4 - row, y - row, 1, wide as usize, ink),
                }
            }
        };
        match key.as_str() {
            "up" => arrow(canvas, (0, -1)),
            "down" => arrow(canvas, (0, 1)),
            "left" => arrow(canvas, (-1, 0)),
            "right" => arrow(canvas, (1, 0)),
            "enter" => _ = canvas.text(art, left + 1, top + 2, "OK", ui::color::WHITE),
            letter => _ = canvas.text(art, left + 4, top + 2, &letter.to_uppercase(), ui::color::WHITE),
        }
    }
}

/// A picture's file, read once (none: no such picture).
pub(super) fn picture_file(file: &str) -> Option<Arc<(u32, u32, Vec<u8>)>> {
    // (A cache of the file's content, by its path and when it last changed:
    // a picture painted again is read again, never one gone stale.)
    type Picture = Option<Arc<(u32, u32, Vec<u8>)>>;
    type Read = std::sync::Mutex<HashMap<String, (Option<std::time::SystemTime>, Picture)>>;
    static READ: std::sync::OnceLock<Read> = std::sync::OnceLock::new();
    let changed = std::fs::metadata(file).and_then(|meta| meta.modified()).ok();
    let mut read = READ.get_or_init(Default::default).lock().ok()?;
    if let Some((when, picture)) = read.get(file)
        && *when == changed
    {
        return picture.clone().filter(|picture| picture.0 > 0 && picture.1 > 0);
    }
    let picture: Picture =
        image::open(file).ok().map(|picture| picture.to_rgba8()).map(|picture| Arc::new((picture.width(), picture.height(), picture.into_raw())));
    read.insert(file.to_string(), (changed, picture.clone()));
    picture.filter(|picture| picture.0 > 0 && picture.1 > 0)
}

/// Soft lights about the scene (see `Beat::Halo`): a disc of their colour
/// laid over what is shown, brighter towards its middle, on whom they
/// follow (about the middle of its body) or on their point.
pub(super) fn draw_halos(canvas: &mut Canvas, stage: &Stage, below: i32, close: f32, on_screen: &impl Fn(Vec2) -> (i32, i32)) {
    for at in 0..stage.light.halos.len() {
        draw_halo(canvas, stage, at, below, close, on_screen);
    }
}

/// A soft light, by its rank among them (see `draw_halos`).
pub(super) fn draw_halo(canvas: &mut Canvas, stage: &Stage, at: usize, below: i32, close: f32, on_screen: &impl Fn(Vec2) -> (i32, i32)) {
    // (Layers, from the widest and faintest to the brightest heart; the
    // widest one pixel in two, as a screen of few colours would.)
    const LAYERS: [(f32, u8); 3] = [(1.0, 22), (0.7, 26), (0.4, 30)];
    // (By day, with little or no light of the scene's over it, as strong
    // again three times: else nothing shows.)
    let strength = if stage.light.tint.now[3] < 140.0 { 3 } else { 1 };
    if let Some(Halo { on, radius, colour: [red, green, blue] }) = stage.light.halos.get(at) {
        let middle = match on {
            Cam::At(at) => at.vec2(),
            Cam::Who(who) => match stage.cast.get(who) {
                Some(actor) => actor.shown_at() - Vec2::new(0.0, 12.0 + actor.shown_height()),
                None => return,
            },
        };
        let (x, y) = on_screen(middle);
        for (part, alpha) in LAYERS {
            let reach = radius * close * part;
            let rows = reach as i32;
            for row in -rows..=rows {
                let half = (reach * reach - (row * row) as f32).max(0.0).sqrt() as i32;
                let top = y + row;
                if top < below || top >= below + ui::SCREEN_HEIGHT as i32 {
                    continue;
                }
                let left = (x - half).max(0);
                let right = (x + half).min(ui::SCREEN_WIDTH as i32);
                let colour = [*red, *green, *blue, alpha * strength];
                match part == LAYERS[0].0 {
                    true => (left..right).filter(|pixel| (pixel + top) % 2 == 0).for_each(|pixel| canvas.fill(pixel, top, 1, 1, colour)),
                    false if right > left => canvas.fill(left, top, (right - left) as usize, 1, colour),
                    false => {}
                }
            }
        }
    }
}

/// The dark of a spotlight with more than one hole, its edge one pixel in two (see
/// `Beat::SpotlightAdd`): each point of the screen as dark as the nearest
/// hole leaves it.
pub(super) fn draw_lights(canvas: &mut Canvas, stage: &Stage, below: i32, close: f32, on_screen: &impl Fn(Vec2) -> (i32, i32)) {
    const DARK: f32 = 235.0;
    const EDGE: f32 = 8.0;
    let spotlight = &stage.light.spotlight;
    if spotlight.shown <= 0.0 {
        return;
    }
    let at = |on: &Cam| match on {
        Cam::At(at) => Some(at.vec2()),
        Cam::Who(who) => stage.cast.get(who).map(|actor| actor.shown_at() - Vec2::new(0.0, 12.0 + actor.shown_height())),
    };
    let holes: Vec<((i32, i32), f32)> = std::iter::once((Some(spotlight.at), spotlight.radius))
        .chain(stage.light.lights.iter().map(|lit| (at(&lit.on), lit.radius)))
        .filter_map(|(at, radius)| Some((on_screen(at?), radius * close)))
        .collect();
    let darkest = (if spotlight.black { 255.0 } else { DARK }) * spotlight.shown;
    let dark = [0, 0, 0, darkest as u8];
    let wide = ui::SCREEN_WIDTH as i32;
    let mut near: Vec<(i32, i32)> = Vec::with_capacity(holes.len());
    for row in below..below + ui::SCREEN_HEIGHT as i32 {
        // (Where a row comes near a hole, a pixel at a time; all dark between.)
        near.clear();
        for &((x, y), radius) in &holes {
            let (outer, from_middle) = (radius + EDGE, (row - y) as f32);
            if from_middle.abs() < outer {
                let half = (outer * outer - from_middle * from_middle).sqrt() as i32 + 1;
                near.push(((x - half).clamp(0, wide), (x + half + 1).clamp(0, wide)));
            }
        }
        near.sort_unstable();
        let mut done = 0;
        for &(left, right) in &near {
            let left = left.max(done);
            canvas.fill(done, row, (left - done).max(0) as usize, 1, dark);
            for column in left..right {
                // (0 in a hole, 1 out of all of them, between on their edge.)
                let shade = holes.iter().map(|&((x, y), radius)| {
                    let away = (((column - x).pow(2) + (row - y).pow(2)) as f32).sqrt();
                    ((away - radius) / EDGE).clamp(0.0, 1.0)
                });
                // (On the edge, the dark falls one pixel in two: never a soft
                // shade, as the one hole's.)
                let shade = shade.fold(1.0_f32, f32::min);
                if shade >= 1.0 || (shade > 0.0 && (column + row) % 2 == 0) {
                    canvas.fill(column, row, 1, 1, dark);
                }
            }
            done = done.max(right);
        }
        canvas.fill(done, row, (wide - done).max(0) as usize, 1, dark);
    }
}

/// How much of a drifting texture of the game's shows, of 255, and how far
/// it drifts a 60th of a second, by the weather (as on a floor, `dive`).
const SEEN: u32 = 115;
const SAND_DRIFT: (f32, f32) = (-3.0, -1.0);
const FOG_DRIFT: (f32, f32) = (-0.3, -0.1);

/// The game's picture of a weather over the lower screen: a texture laid
/// as many times as it takes, drifting, or frames covering the screen
/// (the rain), centred on it.
fn draw_sky(canvas: &mut Canvas, sky: &shell::art::WeatherLayer, weather: Sky, below: i32, seconds: f32) {
    let (wide, high) = (ui::SCREEN_WIDTH as i32, ui::SCREEN_HEIGHT as i32);
    let ticks = seconds * 60.0;
    if !sky.drifts {
        let Some((frame_wide, frame_high, rgba)) = sky.frames.get((ticks / sky.ticks.max(1.0)) as usize % sky.frames.len().max(1)) else {
            return;
        };
        let (left, top) = ((wide - *frame_wide as i32) / 2, (high - *frame_high as i32) / 2);
        for y in 0..high {
            let Some(row) = usize::try_from(y - top).ok().filter(|row| row < frame_high) else {
                continue;
            };
            for x in 0..wide {
                let Some(column) = usize::try_from(x - left).ok().filter(|column| column < frame_wide) else {
                    continue;
                };
                let at = (row * frame_wide + column) * 4;
                if rgba[at + 3] > 0 {
                    canvas.put(x, below + y, [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]);
                }
            }
        }
        return;
    }
    let Some((side_wide, side_high, rgba)) = sky.frames.first() else {
        return;
    };
    let (side_wide, side_high) = (*side_wide as i32, *side_high as i32);
    if side_wide == 0 || side_high == 0 {
        return;
    }
    let drift = if weather == Sky::Sand { SAND_DRIFT } else { FOG_DRIFT };
    let (from_x, from_y) = ((drift.0 * ticks).floor() as i32, (drift.1 * ticks).floor() as i32);
    for y in 0..high {
        let row = (y + from_y).rem_euclid(side_high) as usize;
        for x in 0..wide {
            let at = (row * side_wide as usize + (x + from_x).rem_euclid(side_wide) as usize) * 4;
            let alpha = (rgba[at + 3] as u32 * SEEN / 255) as u8;
            canvas.put(x, below + y, [rgba[at], rgba[at + 1], rgba[at + 2], alpha]);
        }
    }
}

/// The scene's light's colour, what is in the air (rain, snow, petals,
/// sparkles, leaves, hail, sand, mist, sunbeams: the game's own pictures
/// when it has them, `sky`), and a flash.
pub(super) fn draw_air(canvas: &mut Canvas, stage: &Stage, below: i32, seconds: f32, tinted: bool, sky: Option<&shell::art::WeatherLayer>) {
    let (wide, high) = (ui::SCREEN_WIDTH as i32, ui::SCREEN_HEIGHT as i32);
    let [red, green, blue, much] = stage.light.tint.now;
    if tinted && much >= 1.0 {
        canvas.fill(0, below, wide as usize, high as usize, [red as u8, green as u8, blue as u8, much.min(255.0) as u8]);
    }
    if let Some(sky) = sky {
        draw_sky(canvas, sky, stage.light.weather, below, seconds);
    } else if stage.light.weather != Sky::Clear {
        // Each piece: where it starts, how fast it falls and drifts, by
        // its number; the screen wraps it round.
        let (count, speed, drift, colour, size): (u32, f32, f32, [u8; 4], (usize, usize)) = match stage.light.weather {
            Sky::Clear => (0, 0.0, 0.0, [0; 4], (0, 0)),
            Sky::Rain => (70, 300.0, -60.0, [170, 200, 255, 200], (1, 6)),
            Sky::Snow => (50, 30.0, 12.0, [255, 255, 255, 230], (2, 2)),
            Sky::Petals => (30, 25.0, 20.0, [255, 170, 200, 240], (3, 2)),
            Sky::Sparkles => (25, -12.0, 0.0, [255, 250, 180, 255], (1, 1)),
            Sky::Hail => (45, 240.0, -30.0, [235, 245, 255, 240], (2, 3)),
            Sky::Sand => (80, 40.0, 320.0, [215, 180, 110, 170], (7, 1)),
            Sky::Mist => (16, 3.0, 12.0, [255, 255, 255, 16], (90, 18)),
            Sky::Sun => (7, 0.0, 6.0, [255, 245, 170, 22], (14, 192)),
            Sky::Leaves => (30, 35.0, 30.0, [120, 170, 60, 240], (3, 2)),
        };
        let swaying = matches!(stage.light.weather, Sky::Snow | Sky::Petals | Sky::Leaves);
        for piece in 0..count {
            let seed = piece.wrapping_mul(2_654_435_761);
            let (x0, y0) = ((seed % wide as u32) as f32, ((seed >> 12) % high as u32) as f32);
            let pace = 0.7 + ((seed >> 20) % 60) as f32 / 100.0;
            let sway = if swaying { (seconds * 2.0 + piece as f32).sin() * 6.0 } else { 0.0 };
            let x = (x0 + drift * pace * seconds + sway).rem_euclid(wide as f32) as i32;
            // (Sunbeams go from the top of the screen to its bottom.)
            let y = if stage.light.weather == Sky::Sun { 0 } else { (y0 + speed * pace * seconds).rem_euclid(high as f32) as i32 };
            // (Sparkles twinkle.)
            if stage.light.weather == Sky::Sparkles && (seconds * 4.0 + piece as f32 * 0.37).sin() < 0.3 {
                continue;
            }
            canvas.fill(x, below + y, size.0, size.1, colour);
            if stage.light.weather == Sky::Sparkles {
                canvas.fill(x - 1, below + y, 3, 1, colour);
                canvas.fill(x, below + y - 1, 1, 3, colour);
            }
        }
    }
    let [red, green, blue, much] = stage.light.flash.colour;
    if much >= 1.0 {
        canvas.fill(0, below, wide as usize, high as usize, [red as u8, green as u8, blue as u8, much.min(255.0) as u8]);
    }
}
