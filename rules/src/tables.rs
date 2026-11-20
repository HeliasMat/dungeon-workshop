//! What `dungeons.bin` holds: every table the game counts with, as the
//! extraction writes it.

use serde::{Deserialize, Serialize};

use crate::dungeon::{Constants, FloorProperties, ItemTable, LevelGain, MoveSlot, Stats, TypeChart};

/// A monster of a floor: what is its own there; the rest is its species'.
#[derive(Serialize, Deserialize)]
pub struct Enemy {
    pub species: u16,
    pub recruit_rate: i16,
    pub stats: Stats,
    pub weight: u16,
    pub exp: i32,
    pub asleep_chance: i32,
}

/// See `FixedFloor`.
#[derive(Serialize, Deserialize)]
pub struct Fixed {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<u8>,
    pub leader: Option<(usize, usize)>,
    /// Foes, those who stand with the team, those who only watch.
    pub monsters: Vec<Placed>,
    pub allies: Vec<Placed>,
    pub watchers: Vec<Placed>,
    /// What lies there; traps, with whether they show.
    pub items: Vec<((usize, usize), u16)>,
    pub traps: Vec<((usize, usize), u8, bool)>,
    /// The way on, where the room says.
    pub exit: Option<(usize, usize)>,
    pub ends_with_enemies: bool,
    pub moves_allowed: bool,
    pub orbs_allowed: bool,
}

/// Someone a fixed room places: where, as what, the way it faces (0: down,
/// then turning as `DIRECTIONS`).
#[derive(Serialize, Deserialize)]
pub struct Placed {
    pub at: (usize, usize),
    pub enemy: Enemy,
    pub facing: u8,
}

/// See `FloorSpec`.
#[derive(Serialize, Deserialize)]
pub struct Floor {
    pub fixed: Option<Fixed>,
    pub props: FloorProperties,
    pub weather: u8,
    pub secondary_kind: u8,
    pub enemies: Vec<Enemy>,
    pub items: ItemTable,
    pub traps: Vec<u16>,
    pub max_money: i32,
    pub tileset: u8,
    pub enemy_iq: i32,
    pub shop_items: ItemTable,
    pub visibility: i32,
    pub shop_pattern: u8,
    pub sticky_chance: i32,
    #[serde(default)]
    pub camouflage: u8,
    #[serde(default)]
    pub nature_power: u16,
    #[serde(default)]
    pub secret_power: u8,
    #[serde(default)]
    pub music: Option<u16>,
}

#[derive(Serialize, Deserialize)]
pub struct DungeonTable {
    pub goes_up: bool,
    pub rules: Rules,
    /// The song played there (`crate::NO_SONG`: none).
    pub music: u16,
    pub floors: Vec<Floor>,
}

/// What every monster of a kind shares; `hero` and `partner` are what it
/// starts with as one or the other.
#[derive(Serialize, Deserialize)]
pub struct Species {
    pub name: String,
    pub movement: u8,
    pub types: (u8, u8),
    pub shadow: Option<u8>,
    /// How heavy it is (see the extract).
    pub weight: u16,
    /// Its family (see the extract).
    pub family: u16,
    /// How it evolves, its entry's gender and its base stats (see
    /// the extract).
    pub evolution: crate::town::Evolution,
    pub gender: u8,
    pub base: [i32; 5],
    pub learnset: Vec<(u8, MoveSlot)>,
    /// The moves TMs and HMs can teach it.
    pub machines: Vec<u16>,
    pub growth: Vec<LevelGain>,
    pub hero: Option<(Stats, Vec<MoveSlot>)>,
    pub partner: Option<(Stats, Vec<MoveSlot>)>,
}

/// `dungeons.bin`.
#[derive(Serialize, Deserialize)]
pub struct Tables {
    pub types: Option<TypeChart>,
    /// Every move as a team member knows it, by move; the move each item
    /// uses (an orb's), by item.
    pub move_slots: Vec<MoveSlot>,
    pub item_moves: Vec<u16>,
    /// The fewest and the most a pile of each item holds.
    pub item_piles: Vec<(u8, u8)>,
    /// The game's IQ tables.
    pub iq: crate::dungeon::iq::IqTables,
    /// The exclusive items' (see `dungeon::exclusive`).
    pub exclusive: crate::dungeon::exclusive::ExclusiveItems,
    /// What the recruiting leader's level adds to a recruit rate.
    pub recruit_boosts: Vec<i32>,
    pub constants: Constants,
    pub moves: Vec<String>,
    pub items: Vec<String>,
    pub item_descriptions: Vec<String>,
    pub iq_skills: Vec<String>,
    pub tactics: Vec<String>,
    pub tactic_descriptions: Vec<String>,
    pub trap_art: Vec<Vec<u8>>,
    pub stairs_tile: usize,
    pub weather_colors: Vec<Vec<u8>>,
    pub item_art: Vec<Vec<u8>>,
    pub regular_attack: Option<MoveSlot>,
    pub money_table: Vec<i32>,
    pub team: (u16, u16),
    /// By monster.
    pub species: Vec<Species>,
    /// The dungeons that can be entered, in the menu's order; then every
    /// dungeon and its name, by id.
    pub playable: Vec<u16>,
    pub dungeon_names: Vec<String>,
    pub dungeons: Vec<DungeonTable>,
    /// What a move shows (see the extract) and its
    /// sound, by move; what a trap shows, by trap; an item, by item.
    pub move_effects: Vec<[u16; 4]>,
    pub move_sounds: Vec<u16>,
    /// How each move's user moves (`MoveMotion`).
    pub move_motions: Vec<MoveMotion>,
    pub trap_effects: Vec<u16>,
    pub item_effects: Vec<[u16; 2]>,
    /// What an item is bought and sold for, by item.
    pub item_prices: Vec<(i32, i32)>,
    /// The Recycle Shop's trades; the visitors of sentry duty.
    pub recycle: Vec<crate::town::RecycleTrade>,
    pub sentry: Vec<u16>,
    /// The item lists the big shop, then the small one, stock from, by
    /// stage of the story.
    pub shops: Vec<Vec<ItemTable>>,
    /// The personality test a new story starts with.
    pub quiz: Option<crate::quiz::QuizData>,
    /// The places of the world map, by marker (a dungeon by its number,
    /// first): the level whose map it is on, and the point there.
    pub map_markers: Vec<Option<(u16, f32, f32)>>,
    /// The characters of the special episodes: monster, level, moves.
    pub episode_characters: Vec<(u16, i32, Vec<MoveSlot>)>,
    pub jobs: crate::jobs::JobTables,
}

/// Song id of silent floors (there is no `bgm0999`).
pub const NO_MUSIC: u16 = 999;

/// The rules of a dungeon (pmdsky-debug's `struct dungeon_restriction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rules {
    /// Floors are counted up (1F, 2F...) rather than down.
    pub goes_up: bool,
    pub enemies_give_exp: bool,
    pub recruitment: bool,
    /// The team is brought back to level 1 for the dungeon.
    pub level_one: bool,
    /// Money may be brought in.
    pub money_allowed: bool,
    pub iq_disabled: bool,
    /// The most items and team members that may come in.
    pub max_items: u8,
    pub max_party: u8,
    /// Turns on a floor before the wind blows the team out (0: none).
    pub turn_limit: u16,
    /// The options asked for (`crate::rule_options`): not in the game's
    /// tables, only a dungeon of one's own or the settings have them.
    #[serde(skip)]
    pub options: crate::rule_options::RuleOptions,
}

/// How a move's user moves as it uses the move (skytemple-files'
/// `MoveAnim`): the group of its sprite's animations it plays, the point
/// of its sprite the move's effects come from (0 head, 1 and 2 its
/// hands, 3 its centre, 255 none), its sound; and for some species
/// their own (pmdsky-debug's `special_monster_move_animation`): species,
/// group, point, sound.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveMotion {
    pub group: u8,
    pub point: u8,
    pub sound: u16,
    pub species: Vec<(u16, u8, u8, u16)>,
    /// What it throws flies in an arc (the "direction" bits at 8 set to
    /// 1: Ice Ball, Egg Bomb, Present...).
    pub lob: bool,
    /// How fast what it throws flies, in pixels a frame: the speed at 12
    /// as the original's code reads it (`GetMoveAnimationSpeed`'s callers:
    /// 1 is 2, 2 is 3, any other 6).
    pub flight: u8,
}

impl MoveMotion {
    /// Group, point and sound for a monster of that species (`None`: the
    /// move's own).
    pub fn of(&self, species: Option<u16>) -> (u8, u8, u16) {
        // (A sound whose low byte is 0 is none: the move's then.)
        match self.species.iter().find(|entry| Some(entry.0) == species) {
            Some(&(_, group, point, sound)) => (group, point, if sound & 0xFF != 0 { sound } else { self.sound }),
            None => (self.group, self.point, self.sound),
        }
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            goes_up: false,
            enemies_give_exp: true,
            recruitment: true,
            level_one: false,
            money_allowed: true,
            iq_disabled: false,
            max_items: 48,
            max_party: 4,
            turn_limit: 0,
            options: crate::rule_options::RuleOptions::default(),
        }
    }
}
