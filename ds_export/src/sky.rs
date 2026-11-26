//! Explorers of Sky's own, as the export writes it: what it reads of the
//! game's data (`Game`: its levels, actors, species, portraits, the types of
//! the moves) and the game's tables, numbers and names (animations, effects,
//! faces, the top screens, the lights, the free actors), and how its scripts
//! write a way, a distance, a speed, a text.

use std::collections::HashMap;

use ::film::tables::game_file;

use engine::Direction;

/// What the exporter reads of the game's data: its levels, actors, which
/// portraits each species has, and the types of the moves.
pub(crate) struct Game {
    pub(crate) levels: HashMap<u16, String>,
    /// The script actors: name, species.
    pub(crate) actors: Vec<(String, u16)>,
    pub(crate) data: String,
    /// Each move's type (the game's numbers), by move.
    pub(crate) move_types: Vec<u8>,
    /// The species' names, to tell who is left out.
    pub(crate) names: HashMap<u16, String>,
    /// The Workshop's own places (decors), by level: each exports as a map
    /// of its own (`crate::map`), its level named `LEVEL_DECOR_<level>` until
    /// `patch_rom.py` gives it one of the game's.
    pub(crate) decors: HashMap<u16, String>,
}

/// How the export names a decor's level, for `patch_rom.py` to rename.
pub(crate) const DECOR_LEVEL: &str = "LEVEL_DECOR_";

impl Game {
    pub(crate) fn read(data: &str, move_types: Vec<u8>) -> Game {
        let rows = |file: &str| -> Vec<Vec<String>> {
            std::fs::read_to_string(game_file(data, &format!("rom/{file}")))
                .unwrap_or_default()
                .lines()
                .map(|line| line.split('\t').map(str::to_string).collect())
                .collect()
        };
        let levels = rows("levels.txt").into_iter().filter_map(|row| Some((row.first()?.parse().ok()?, row.get(1)?.clone()))).collect();
        let actors = rows("entities.txt").into_iter().filter_map(|row| Some((row.get(1)?.clone(), row.get(2)?.parse().ok()?))).collect();
        // (The names of Sky's and of the Pokémon SpriteCollab adds.)
        let names = assets::data::text(data, "monsters.txt")
            .lines()
            .filter_map(|line| {
                let (number, name) = line.split_once('\t')?;
                Some((number.parse().ok()?, name.split('\t').next()?.to_string()))
            })
            .collect();
        _ = DATA.set(data.to_string());
        Game { levels, actors, data: data.to_string(), move_types, names, decors: HashMap::new() }
    }

    /// A level's size in pixels (its picture's, as the extraction or the
    /// decor builder wrote it: two numbers of 8 bytes first).
    pub(crate) fn size(&self, id: u16) -> Option<(f32, f32)> {
        let layer = if self.levels.contains_key(&id) { "rom/" } else { "" };
        let info = std::fs::read(game_file(&self.data, &format!("{layer}grounds/info/{id}.bin"))).ok()?;
        let number = |at: usize| Some(u64::from_le_bytes(info.get(at..at + 8)?.try_into().ok()?) as f32);
        Some((number(0)?, number(8)?))
    }

    pub(crate) fn name(&self, species: u16) -> String {
        self.names.get(&species).cloned().unwrap_or_else(|| format!("species {species}"))
    }

    /// Whether that species has that portrait (none known for 0: the hero
    /// and the partner, whose species the player chose): Sky's own, or, for
    /// one SpriteCollab adds, the data's, put in the ROM with its sprite.
    pub(crate) fn has_face(&self, species: u16, face: u16) -> bool {
        let layer = if species < ::film::ds::SKY_SPECIES { "rom/" } else { "" };
        species == 0 || game_file(&self.data, &format!("{layer}portraits/{species}_{face}.png")).exists()
    }
}

/// The picture Sky shows on the top screen with a place, as its own
/// scenes pair them (`back2_SetMode(4)`, `back2_SetGround`); a place
/// without one keeps the top screen black, as most of Sky's scenes do.
pub(crate) const TOP_SCREENS: [(&str, &str); 22] = [
    ("D01P11A", "V01P03A"),
    ("D01P11B", "V01P03B"),
    ("D47P11A", "V17P01A"),
    ("D53P41C", "V38P06C"),
    ("D73P31A", "V24P08A"),
    ("G01P01B", "V02P01A"),
    ("G01P01B2", "V02P01A"),
    ("G01P01C", "V02P06A"),
    ("G01P05A", "V34P01A"),
    ("H01P99E", "V21P02B"),
    ("H02P99A", "V24P08A"),
    ("P03P02A", "V24P08A"),
    ("P07P01A", "V17P01A"),
    ("P12P01A", "V26P04A"),
    ("P16P01A", "V24P08A"),
    ("P17P01A", "V24P08A"),
    ("P21P02A", "V24P08A"),
    ("P23P02A", "V17P01A"),
    ("P27P01A", "V39P01A"),
    ("S02P01A", "S02P01A"),
    ("S04P01A", "S04P01A"),
    ("S17P02A", "S02P01A"),
];

/// The evening and night lights (`screen_FlushOut`'s mode and colour).
pub(crate) const EVENING: &str = "4, 224, 176, 136";

pub(crate) const NIGHT: &str = "4, 80, 112, 144";

/// Dawn, softer and warm; a cave's dimness, grey.
pub(crate) const DAWN: &str = "4, 248, 224, 208";

pub(crate) const DIM: &str = "4, 160, 160, 160";

/// The light of the engine's tint (its colour and strength), as Sky's.
pub(crate) fn light(colour: &[f32; 4]) -> Option<&'static str> {
    Some(match colour {
        [.., much] if *much <= 0.0 => return None,
        [red, green, blue, _] if red.max(*green).max(*blue) < 10.0 => DIM,
        [red, _, blue, _] if blue > red => NIGHT,
        [.., much] if *much < 64.0 => DAWN,
        _ => EVENING,
    })
}

/// The script variables a film's own may use (local to the scene).
pub(crate) const LOCALS: [&str; 4] = ["$LOCAL0", "$LOCAL1", "$LOCAL2", "$LOCAL3"];

/// The output attribute that hides an actor (`SetOutputAttribute<actor>(128)`).
pub(crate) const HIDDEN: u16 = 128;

/// The data folder the exporter reads, for the table of the sprites' own
/// orders (`film::ds::own_group`).
static DATA: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// The data folder read (none in a test without it).
fn data() -> &'static str {
    DATA.get().map_or("", String::as_str)
}

/// The group of that species' own sprite named so (`Roar`), if it has one
/// Sky's table reaches.
pub(crate) fn own_named(species: u16, name: &str) -> Option<u16> {
    ::film::ds::own_named(data(), species, name)
}

/// The group of that species' own sprite that shows the engine's group
/// (`film::ds::own_group`).
pub(crate) fn own_group(species: u16, group: u16) -> Option<u16> {
    ::film::ds::own_group(data(), species, group)
}

/// The pose the DS plays for the engine's group on that species' sprite,
/// the nearest every sprite has where its own lacks it (`film::ds::pose_on_ds`).
pub(crate) fn pose_on_ds(species: u16, group: u16, looped: bool) -> Option<::film::ds::PoseOnDs> {
    ::film::ds::pose_on_ds(data(), species, group, looped)
}

pub(crate) use ::film::ds::animation;

/// The script effect closest to a move, by its type (the game's numbers:
/// 1 Normal ... 17 Steel).
pub(crate) fn move_effect(kind: u8) -> u16 {
    match kind {
        2 => 6,    // Fire: FireAttack
        3 => 2,    // Water: WaterSplash
        4 => 126,  // Grass: VinesAttackingSpiral
        5 => 113,  // Electric: LightningStrike
        6 => 23,   // Ice: TrappedInIce
        8 => 1,    // Poison: PoisonBubble
        9 => 53,   // Ground: RocksFalling
        10 => 86,  // Flying: AirBlades
        11 => 105, // Psychic: LightColoredFocusing
        12 => 46,  // Bug: SpiderWeb
        13 => 54,  // Rock: RocksHitting
        14 => 63,  // Ghost: Curse
        15 => 57,  // Dragon: FireExplosion
        16 => 116, // Dark: BlackHandSlapping
        17 => 74,  // Steel: ExplosionSmall
        _ => 127,  // Normal, Fighting: PunchAttack
    }
}

/// The actors the game never uses, given the film's species: the ten test
/// actors, then placeholders `patch_rom.py` gives the other free ones (45 in all).
pub(crate) const FREE_ACTORS: [&str; 45] = [
    "NPC_TEST001",
    "NPC_TEST002",
    "NPC_TEST003",
    "NPC_TEST004",
    "NPC_TEST005",
    "NPC_TEST006",
    "NPC_TEST007",
    "NPC_TEST008",
    "NPC_TEST009",
    "NPC_TEST010",
    "FREE11",
    "FREE12",
    "FREE13",
    "FREE14",
    "FREE15",
    "FREE16",
    "FREE17",
    "FREE18",
    "FREE19",
    "FREE20",
    "FREE21",
    "FREE22",
    "FREE23",
    "FREE24",
    "FREE25",
    "FREE26",
    "FREE27",
    "FREE28",
    "FREE29",
    "FREE30",
    "FREE31",
    "FREE32",
    "FREE33",
    "FREE34",
    "FREE35",
    "FREE36",
    "FREE37",
    "FREE38",
    "FREE39",
    "FREE40",
    "FREE41",
    "FREE42",
    "FREE43",
    "FREE44",
    "FREE45",
];

/// The portraits' names (FACE_…), by number.
pub(crate) const FACES: [&str; 14] =
    ["NORMAL", "HAPPY", "PAIN", "ANGRY", "WORRIED", "SAD", "CRYING", "SHOUTING", "TEARY_EYED", "DETERMINED", "JOYOUS", "INSPIRED", "SURPRISED", "DIZZY"];

/// The game's effects by the number the engine gives them (the same as
/// the game's own), named as SkyTemple names them; others by number.
pub(crate) const EFFECTS: [(u16, &str); 10] = [
    (0, "EFFECT_NONE"),
    (88, "EFFECT_EXCLAMATION_MARK"),
    (89, "EFFECT_QUESTION_MARK"),
    (90, "EFFECT_JOYOUS"),
    (91, "EFFECT_SHOCKED"),
    (92, "EFFECT_SWEAT_DROP"),
    (93, "EFFECT_LAUGHING"),
    (94, "EFFECT_ANGRY"),
    (171, "EFFECT_SWEAT_DROPS_FROM_BOTH_SIDES_MEDIUM"),
    (172, "EFFECT_TWO_ARROWS_AT_SIDE_LEFT"),
];

pub(crate) fn direction(way: Direction) -> &'static str {
    match way {
        Direction::Down => "DIR_DOWN",
        Direction::DownRight => "DIR_DOWNRIGHT",
        Direction::Right => "DIR_RIGHT",
        Direction::UpRight => "DIR_UPRIGHT",
        Direction::Up => "DIR_UP",
        Direction::UpLeft => "DIR_UPLEFT",
        Direction::Left => "DIR_LEFT",
        Direction::DownLeft => "DIR_DOWNLEFT",
    }
}

/// Pixels of the level, in the game's tiles of 8 (halves kept).
pub(crate) fn tiles(pixels: f32) -> String {
    let halves = (pixels / 4.0).round() / 2.0;
    if halves.fract() == 0.0 { format!("{halves:.0}") } else { format!("{halves:.1}") }
}

/// A speed of the game's (pixels a frame, in 256ths).
pub(crate) fn speed(pixels: f32, frames: f32) -> String {
    let speed = (pixels / frames.max(1.0) * 256.0).round().max(16.0) / 256.0;
    let text = format!("{speed:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A text as the game's script writes it.
pub(crate) fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_become_tiles_with_halves() {
        assert_eq!(tiles(400.0), "50");
        assert_eq!(tiles(205.0), "25.5");
        assert_eq!(tiles(336.0), "42");
    }

    #[test]
    fn each_species_poses_with_its_own_sprites_group() {
        // (The table is read from one's own ROM into the data: where it is.)
        let data = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
        if !game_file(data, ::film::ds::OWN_ORDER).exists() {
            return;
        }
        _ = DATA.set(data.to_string());
        // Pikachu and Piplup share the starters' order.
        assert_eq!(own_group(25, 33), Some(33));
        assert_eq!(own_group(428, 33), Some(33));
        // Dusknoir faints lying with its 27 (its own "Faint", 25, rises again).
        assert_eq!(own_group(519, 33), Some(27));
        assert_eq!(own_group(519, 27), Some(27));
        // Snorlax has none of them: asleep for lying, a hop for a nod.
        assert_eq!(own_group(143, 33), Some(5));
        assert_eq!(own_group(143, 22), Some(10));
    }

    #[test]
    fn speeds_are_pixels_a_frame_in_256ths() {
        assert_eq!(speed(60.0, 60.0), "1");
        assert_eq!(speed(30.0, 60.0), "0.5");
        assert_eq!(speed(1.0, 600.0), "0.0625");
    }

    #[test]
    fn texts_are_quoted_on_one_line() {
        assert_eq!(quoted("a \"b\"\nc"), "a \\\"b\\\"\\nc");
    }
}
