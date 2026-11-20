//! A floor of the dungeon being made, laid out by the game's own generator
//! (`rules::dungeon::generate_floor`) for a seed: as a map (rooms,
//! hallways, its water, the stairs, where items, traps and Pokémon are put)
//! or drawn with its tileset as the game draws it. Its floors all take the
//! shape of the look's floor (see `rules::own::read`): what changes
//! from one to another is who is met there and what lies there.

use rules::dungeon::{DungeonRng, FLOOR_HEIGHT, FLOOR_WIDTH, Floor, Pos, Tile, generate_floor};
use rules::tables::Floor as FloorTable;

use super::*;

/// The floor shown: which (from 1), its seed, map or drawn; its picture for
/// what it was made of.
pub(super) struct FloorShown {
    pub(super) at: usize,
    seed: u32,
    drawn: bool,
    made: Option<(String, Option<Made>)>,
    art: Option<shell::art::Art>,
}

/// A floor laid out, and its picture.
struct Made {
    floor: Floor,
    texture: TextureHandle,
}

impl FloorShown {
    pub(super) fn new(launch: &crate::launch::LaunchOptions) -> FloorShown {
        // (`WORKSHOP_SEED=n`: that seed, for a picture; `WORKSHOP_DRAWN=1`: drawn.)
        FloorShown { at: 1, seed: launch.seed.unwrap_or(1), drawn: launch.drawn, made: None, art: None }
    }
}

/// The game's floor a look says (`Apple Woods`, `12`, and a floor of it).
pub(super) fn look_floor<'a>(offered: &'a Offered, look: &str, floor: usize) -> Option<&'a FloorTable> {
    game::dungeon_check::look_floor(&offered.known, &offered.book, look, floor)
}

/// A seed of its own, as random as the clock.
fn new_seed(old: u32) -> u32 {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.subsec_nanos());
    (nanos ^ old.rotate_left(13)).wrapping_mul(0x9E37_79B9) >> 1
}

/// The colour of a tile on the map: wall, secondary terrain (water, lava,
/// chasm), a room's ground (a shop's, a Monster House's), a hallway.
fn colour(floor: &Floor, at: Pos, secondary: u8) -> [u8; 4] {
    match floor.tile(at) {
        Tile::Wall => [0x1A, 0x1C, 0x24, 255],
        Tile::Secondary => match secondary {
            1 => [0xC8, 0x48, 0x20, 255],
            2 => [0x06, 0x06, 0x08, 255],
            _ => [0x30, 0x68, 0xD8, 255],
        },
        Tile::Floor { room: None } => [0x5A, 0x62, 0x70, 255],
        Tile::Floor { room: Some(room) } | Tile::Stairs { room } => {
            if floor.shop.is_some_and(|shop| shop.contains(at)) {
                [0x5C, 0x9C, 0x6C, 255]
            } else if floor.monster_house == Some(room) {
                [0xA0, 0x5C, 0x5C, 255]
            } else {
                [0x98, 0xA4, 0xB8, 255]
            }
        }
    }
}

/// The floor laid out for its look and seed, and pictured.
fn made(session: &crate::session::Session, shown: &mut FloorShown, context: &egui::Context, spec: &FloorTable, own: Option<&std::path::Path>) -> Made {
    let floor = generate_floor(&mut DungeonRng::new(shown.seed), &spec.props);
    let (wide, high, rgba) = if shown.drawn {
        let art = shown.art.get_or_insert_with(|| shell::art::Art::open_in(session.data(), session.library().clone()));
        // (A tileset of one's own, read again each time: it may have just been imported.)
        let tileset = match own {
            Some(path) => media::template::read(path).ok().map(std::rc::Rc::new),
            None => art.tileset(spec.tileset),
        };
        (dive::FLOOR_WIDE, dive::FLOOR_HIGH, dive::floor_drawn(&floor, tileset.as_deref()))
    } else {
        let rgba = Floor::positions().flat_map(|at| colour(&floor, at, spec.secondary_kind)).collect();
        (FLOOR_WIDTH as usize, FLOOR_HEIGHT as usize, rgba)
    };
    let image = egui::ColorImage::from_rgba_unmultiplied([wide, high], &rgba);
    Made { floor, texture: context.load_texture("floor preview", image, egui::TextureOptions::NEAREST) }
}

/// The floor chosen, as the game would lay it out: its seed, map or drawn,
/// its picture with where things are put, what is met and lies there.
pub(super) fn floor_preview(ui: &mut egui::Ui, making: &mut DungeonMaking) {
    let kept = offered(&making.session);
    let offered = kept.as_deref();
    let form = &making.form;
    let shown = &mut making.floor_shown;
    shown.at = shown.at.clamp(1, form.floors.max(1));
    ui.add_space(10.0);
    ui.label(RichText::new("A FLOOR, AS THE GAME MAKES IT").size(theme::SMALL).color(theme::WEAK).strong());
    if form.floors == 0 {
        ui.label(theme::text("0 floors: the boss fight alone", theme::SMALL, theme::FAINT));
        return;
    }
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut shown.at).range(1..=form.floors).prefix("floor ")).on_hover_text("or a click on a floor's number, above");
        ui.label(theme::text("seed", theme::SMALL, theme::WEAK));
        ui.add(egui::DragValue::new(&mut shown.seed).speed(1.0)).on_hover_text("the same seed makes the same floor");
        if ui.button("New Seed").clicked() {
            shown.seed = new_seed(shown.seed);
        }
        ui.separator();
        ui.selectable_value(&mut shown.drawn, false, "Map");
        ui.selectable_value(&mut shown.drawn, true, "Drawn").on_hover_text("with the look's tileset, as in the game");
    });
    let Some(spec) = offered.and_then(|offered| look_floor(offered, &form.look, form.look_floor)) else {
        ui.label(RichText::new("no look taken from a game dungeon: nothing to lay out").color(theme::WRONG).size(theme::SMALL));
        return;
    };
    // (Made again only when its look, its seed or how it is shown change.)
    let own = (!form.tileset.is_empty()).then(|| making.project.file("tilesets", &form.tileset, "png"));
    let changed = own.as_ref().and_then(|path| std::fs::metadata(path).and_then(|data| data.modified()).ok());
    let key = format!("{}|{}|{}|{}|{}|{changed:?}", form.look, form.look_floor, shown.seed, shown.drawn, form.tileset);
    if shown.made.as_ref().is_none_or(|(made_for, _)| *made_for != key) {
        let picture = made(&making.session, shown, ui.ctx(), spec, own.as_deref());
        shown.made = Some((key, Some(picture)));
    }
    let Some((_, Some(picture))) = &shown.made else {
        return;
    };
    let floor = &picture.floor;
    let wide = ui.available_width();
    let tile = wide / FLOOR_WIDTH as f32;
    let (rect, response) = ui.allocate_exact_size(vec2(wide, tile * FLOOR_HEIGHT as f32), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.image(picture.texture.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    // (Where things are put: a square on each tile, the stairs larger.)
    let mark = |at: Pos, colour: Color32, share: f32| {
        let middle = rect.min + vec2((at.x as f32 + 0.5) * tile, (at.y as f32 + 0.5) * tile);
        painter.rect_filled(Rect::from_center_size(middle, vec2(tile * share, tile * share)), 0.0, colour);
        painter.rect_stroke(
            Rect::from_center_size(middle, vec2(tile * share, tile * share)),
            0.0,
            egui::Stroke::new(1.0_f32, Color32::BLACK),
            egui::StrokeKind::Outside,
        );
    };
    const ITEM: Color32 = Color32::from_rgb(0xF0, 0xD0, 0x40);
    const TRAP: Color32 = Color32::from_rgb(0xD0, 0x60, 0xE0);
    const FOE: Color32 = Color32::from_rgb(0xF0, 0x40, 0x40);
    const TEAM: Color32 = Color32::from_rgb(0x50, 0xE0, 0x70);
    const BURIED: Color32 = Color32::from_rgb(0x9C, 0x74, 0x40);
    const STAIRS: Color32 = Color32::WHITE;
    for at in &floor.buried_item_spawns {
        mark(*at, BURIED, 0.35);
    }
    for at in &floor.item_spawns {
        mark(*at, ITEM, 0.5);
    }
    for at in &floor.trap_spawns {
        mark(*at, TRAP, 0.5);
    }
    for at in &floor.enemy_spawns {
        mark(*at, FOE, 0.55);
    }
    mark(floor.player_spawn, TEAM, 0.65);
    mark(floor.stairs, STAIRS, 0.8);
    // (Over a tile: where it is, and what it is.)
    if let Some(pointer) = response.hover_pos() {
        let at = Pos::new(((pointer.x - rect.min.x) / tile) as i32, ((pointer.y - rect.min.y) / tile) as i32);
        let what = match floor.tile(at) {
            Tile::Wall => "wall",
            Tile::Secondary => ["water", "lava", "chasm"].get(spec.secondary_kind as usize).copied().unwrap_or("water"),
            Tile::Floor { room: None } => "hallway",
            Tile::Floor { room: Some(_) } => "room",
            Tile::Stairs { .. } => "stairs",
        };
        response.on_hover_text(format!("{}, {}: {what}", at.x, at.y));
    }
    // --- What is in it ---
    let rooms: std::collections::BTreeSet<u8> = Floor::positions().filter_map(|at| floor.tile(at).room()).collect();
    ui.horizontal_wrapped(|ui| {
        let key = |ui: &mut egui::Ui, colour: Color32, said: String| {
            let (square, _) = ui.allocate_exact_size(vec2(9.0, 9.0), Sense::hover());
            ui.painter().rect_filled(square, 0.0, colour);
            ui.label(theme::text(said, theme::SMALL, theme::TEXT));
        };
        ui.label(theme::text(format!("{} rooms", rooms.len()), theme::SMALL, theme::TEXT));
        key(ui, STAIRS, "stairs".to_string());
        key(ui, TEAM, "the team".to_string());
        key(ui, FOE, format!("{} Pokémon", floor.enemy_spawns.len()));
        key(ui, ITEM, format!("{} items", floor.item_spawns.len()));
        key(ui, TRAP, format!("{} traps", floor.trap_spawns.len()));
        if !floor.buried_item_spawns.is_empty() {
            key(ui, BURIED, format!("{} buried", floor.buried_item_spawns.len()));
        }
        if floor.monster_house.is_some() {
            ui.label(RichText::new("Monster House").size(theme::SMALL).color(Color32::from_rgb(0xE0, 0x80, 0x80)));
        }
        if floor.shop.is_some() {
            ui.label(RichText::new("Kecleon Shop").size(theme::SMALL).color(Color32::from_rgb(0x80, 0xD0, 0x90)));
        }
        if floor.fallback {
            ui.label(RichText::new("the generator failed: a single room").size(theme::SMALL).color(theme::WRONG));
        }
    });
    let props = &spec.props;
    ui.label(theme::text(
        format!(
            "layout {} · rooms {} · links {} · water {} · Pokémon {} · items {} · traps {} (the look's, floor {})",
            props.layout,
            props.room_density,
            props.floor_connectivity,
            props.secondary_density,
            props.enemy_density,
            props.item_density,
            props.trap_density,
            form.look_floor
        ),
        theme::SMALL - 1.0,
        theme::FAINT,
    ));
    // (Who is met on that floor, and what lies there.)
    let on = |floors: Floors| floors.is_none_or(|(low, high)| low <= shown.at && shown.at <= high);
    let foes: Vec<String> = form.foes.iter().filter(|foe| on(foe.floors)).map(|foe| format!("{} {}", foe.species, foe.level)).collect();
    let items: Vec<String> = form.items.iter().filter(|lying| on(lying.floors)).map(|lying| lying.item.clone()).collect();
    ui.label(theme::text(format!("met there: {}", if foes.is_empty() { "no one".to_string() } else { foes.join(", ") }), theme::SMALL, theme::WEAK));
    ui.label(theme::text(
        format!(
            "on the ground: {}",
            if form.items.is_empty() {
                "the look's".to_string()
            } else if items.is_empty() {
                "nothing".to_string()
            } else {
                items.join(", ")
            }
        ),
        theme::SMALL,
        theme::WEAK,
    ));
}
