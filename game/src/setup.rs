//! What the game's dungeons give the rest, made once after the extraction:
//! the tileset of each floor (`data/extra/dungeon_floors.txt`, for the
//! `decor` builder, which lays a dungeon room as a place), and the stairs
//! and the traps as pictures of things (`data/hand/stairs.png`,
//! `trap_slumber.png`...: made of the game's, never committed).
//! `scene_player dungeons` and the Workshop's first launch.

use image::RgbaImage;
use rules::tables::Tables;

/// The traps by their number in the game, as films name them (`trap_<word>`).
const TRAPS: [&str; 25] = [
    "",
    "mud",
    "sticky",
    "grimy",
    "summon",
    "trapdoor",
    "warp",
    "gust",
    "spin",
    "slumber",
    "slow",
    "seal",
    "poison",
    "selfdestruct",
    "explosion",
    "pp_zero",
    "chestnut",
    "wonder",
    "pokemon",
    "spikes",
    "stealth_rock",
    "toxic_spikes",
    "trip",
    "random",
    "grudge",
];
/// A trap's and the stairs' side, in pixels.
const SIDE: u32 = 24;

/// The floors' tilesets and the pictures written from the data folder's
/// `dungeons.bin`: what was written, a line each.
pub fn prepare_dungeons(data: &str) -> Result<Vec<String>, crate::Error> {
    let bytes = assets::data::read(data, "dungeons.bin").ok_or_else(|| crate::Error::NotFound(format!("{data}/dungeons.bin")))?;
    let tables: Tables = bincode::deserialize(&bytes).map_err(|error| crate::Error::Unreadable { what: "dungeons.bin", why: error.to_string() })?;
    let mut rows = vec!["# dungeon\tname\tfloor\ttileset\tsecondary".to_string()];
    for (id, dungeon) in tables.dungeons.iter().enumerate() {
        let name = tables.dungeon_names.get(id).cloned().unwrap_or_default();
        for (floor, spec) in dungeon.floors.iter().enumerate() {
            rows.push(format!("{id}\t{name}\t{}\t{}\t{}", floor + 1, spec.tileset, spec.secondary_kind));
        }
    }
    let extra = std::path::Path::new(data).join("extra");
    std::fs::create_dir_all(&extra)?;
    std::fs::write(extra.join("dungeon_floors.txt"), rows.join("\n") + "\n")?;
    let hand = std::path::Path::new(data).join("hand");
    std::fs::create_dir_all(&hand)?;
    let mut written = Vec::new();
    let mut picture = |name: &str, kind: usize| {
        let rgba = tables.trap_art.get(kind).filter(|rgba| rgba.len() == (SIDE * SIDE * 4) as usize)?;
        let image = RgbaImage::from_raw(SIDE, SIDE, rgba.clone())?;
        image.save(hand.join(format!("{name}.png"))).ok()?;
        written.push(name.to_string());
        Some(())
    };
    picture("stairs", tables.stairs_tile);
    for (kind, word) in TRAPS.iter().enumerate().filter(|(_, word)| !word.is_empty()) {
        picture(&format!("trap_{word}"), kind);
    }
    Ok(vec![
        format!("{}: {} floors of {} dungeons", extra.join("dungeon_floors.txt").display(), rows.len() - 1, tables.dungeons.len()),
        format!("{}: {}", hand.display(), written.join(" ")),
    ])
}
