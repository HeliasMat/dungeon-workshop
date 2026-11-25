//! The user's ROM dump, read: what `--extract` writes out of it.

mod art;
mod interface;
mod places;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rom_formats::dungeons::{DungeonData, NO_MUSIC};
use rom_formats::mapbg::MapEntry;
use rom_formats::mappa::FloorData;
use rom_formats::script::{Entity, Level, Opcode, Script};
use rom_formats::vm::Program;
use rules::dungeon::{self, ItemTable, MoveSlot, Stats};
use rules::tables::{Enemy, Fixed, Floor, Placed};

/// What is read once: by key, what was found.
pub(crate) type ReadOnce<Key, Value> = Mutex<HashMap<Key, Option<Value>>>;

pub(crate) use art::{EffectArt, SpriteArt};
pub(crate) use interface::Interface;
pub(crate) use places::{PlaceArt, SceneArt};

/// Stats and moves of one monster.
pub(crate) type Monster = (Stats, Vec<MoveSlot>);

/// The team until the personality test has chosen it.
pub(crate) const PLAYER_MD_INDEX: usize = 1;
pub(crate) const PLAYER_LEVEL: u8 = 5;
pub(crate) const PARTNER_MD_INDEX: usize = 25;
pub(crate) const PARTNER_LEVEL: u8 = 5;
/// Entries of the monster table before it repeats for the other gender.
pub(crate) const MONSTER_FORMS: usize = 600;
/// No song plays (see `tables::DungeonTable`).
pub(crate) const NO_SONG: u16 = NO_MUSIC;
/// Text language: e, f, g, i or s (EU ROM).
pub(crate) const LANGUAGE: char = 'e';
/// Sounds are short; one that goes on (the wind...) is cut there.
const SOUND_SECONDS: f32 = 8.0;

/// Everything read from the user's ROM dump. Decoded sprites and effects
/// are kept for the next time they are asked for.
pub(crate) struct RomData {
    pub(crate) sound_lengths: Mutex<HashMap<(u16, bool), f32>>,
    /// The songs and sounds, once read.
    pub(crate) library: std::sync::OnceLock<shell::music::Library>,
    /// The scenes' things: their names, and their pictures once read.
    pub(crate) objects: Vec<String>,
    pub(crate) props: ReadOnce<u16, (Arc<SpriteArt>, bool)>,
    pub(crate) scene_animations: std::sync::OnceLock<Vec<rom_formats::script::SceneAnimation>>,
    pub(crate) rom: rom_formats::NdsRom,
    pub(crate) mappa: rom_formats::Mappa,
    pub(crate) monsters: rom_formats::MonsterData,
    pub(crate) moves: rom_formats::MoveData,
    pub(crate) items: rom_formats::items::ItemData,
    pub(crate) names: rom_formats::text::Names,
    pub(crate) dungeons: DungeonData,
    /// Ids of the dungeons that have floors, in menu order.
    pub(crate) playable: Vec<usize>,
    /// The overworld maps listed in `MAP_BG/bg_list.dat`.
    pub(crate) maps: Vec<MapEntry>,
    /// The script engine's tables; empty if they could not be read.
    pub(crate) levels: Vec<Level>,
    pub(crate) entities: Vec<Entity>,
    pub(crate) opcodes: Arc<Vec<Opcode>>,
    /// The scripts' shared routines (`SCRIPT/COMMON/unionall.ssb`).
    pub(crate) common: Option<Arc<Script>>,
    /// By sprite number, and whether the scene animations are included.
    pub(crate) sprites: ReadOnce<(usize, bool), Arc<SpriteArt>>,
    /// The effects once needed.
    pub(crate) effects: Mutex<HashMap<u16, Option<EffectArt>>>,
    pub(crate) quiz: std::sync::OnceLock<Option<Arc<rules::quiz::QuizData>>>,
    /// Item tables of the Kecleon Shop and of Kecleon Wares, by stage of
    /// the story.
    pub(crate) shop_tables: [[u32; 4]; 2],
}

impl RomData {
    pub(crate) fn open(path: &str) -> Result<Self, rom_formats::RomError> {
        let rom = rom_formats::NdsRom::open(path)?;
        let file = |name: &str| rom.file(name).ok_or(rom_formats::RomError::Truncated);
        let mappa = rom_formats::Mappa::parse(file("BALANCE/mappa_s.bin")?)?;
        let monsters = rom_formats::MonsterData::parse(file("BALANCE/monster.md")?, file("BALANCE/m_level.bin")?)?;
        let moves = rom_formats::MoveData::parse(file("BALANCE/waza_p.bin")?)?;
        let items = rom_formats::items::ItemData::parse(file("BALANCE/item_p.bin")?)?;
        // Without the text files the game falls back to numeric ids.
        let names = rom_formats::text::Names::load(&rom, LANGUAGE).inspect_err(|e| eprintln!("game text unavailable: {e}")).unwrap_or_default();
        let dungeons = DungeonData::load(&rom)?;
        let maps = rom.file("MAP_BG/bg_list.dat").map(rom_formats::mapbg::map_list).unwrap_or_default();
        let levels = rom_formats::script::load_levels(&rom).inspect_err(|e| eprintln!("level table unavailable: {e}")).unwrap_or_default();
        let entities = rom_formats::script::load_entities(&rom).inspect_err(|e| eprintln!("entity table unavailable: {e}")).unwrap_or_default();
        let opcodes = rom_formats::script::find_opcodes(&rom).inspect_err(|e| eprintln!("script opcodes unavailable: {e}")).map(Arc::new).unwrap_or_default();
        let common = rom.file("SCRIPT/COMMON/unionall.ssb").and_then(|data| Script::parse(data, &opcodes, LANGUAGE).ok()).map(Arc::new);
        let shop_tables = rom_formats::script::load_shop_tables(&rom).unwrap_or_default();
        let playable = (0..dungeons.dungeons.len()).filter(|&id| !Self::floor_range(&mappa, &dungeons, id).is_empty()).collect();
        println!(
            "ROM {} loaded: {} files, {} floor groups, {} species entries",
            rom.game_code(),
            rom.file_count(),
            mappa.group_count(),
            monsters.species_count()
        );
        Ok(Self {
            scene_animations: std::sync::OnceLock::new(),
            sound_lengths: Mutex::default(),
            library: std::sync::OnceLock::new(),
            objects: rom_formats::script::load_objects(&rom).unwrap_or_default(),
            props: Mutex::default(),
            mappa,
            monsters,
            moves,
            items,
            names,
            dungeons,
            playable,
            maps,
            levels,
            entities,
            opcodes,
            common,
            shop_tables,
            quiz: std::sync::OnceLock::new(),
            sprites: Mutex::default(),
            effects: Mutex::default(),
            rom,
        })
    }

    /// The floors of a dungeon inside its floor group.
    // A few entries name one floor more than the group holds as read by
    // `Mappa`; they are cut short.
    pub(crate) fn floor_range<'a>(mappa: &'a rom_formats::Mappa, dungeons: &DungeonData, id: usize) -> &'a [FloorData] {
        let entry = dungeons.dungeons[id];
        let group = mappa.floors(entry.group as usize);
        let start = (entry.first_floor as usize).min(group.len());
        &group[start..(start + entry.floors as usize).min(group.len())]
    }

    /// Stats and moves of a species at a level.
    pub(crate) fn monster(&self, md_index: usize, level: u8) -> Option<Monster> {
        let stats = self.monsters.stats_at(md_index, level)?;
        let slots = self
            .moves
            .moves_at(md_index % MONSTER_FORMS, level)
            .iter()
            .filter_map(|&id| self.moves.get(id as usize).map(|m| (id, m)))
            .map(|(id, m)| move_slot(id, m))
            .collect();
        Some((stats, slots))
    }
    /// The moves TMs and HMs can teach a species (numbered as learnsets).
    pub(crate) fn machines(&self, md_index: usize) -> &[u16] {
        self.moves.machines(md_index % MONSTER_FORMS)
    }

    /// The moves a monster learns by level, in learning order. Learnsets
    /// follow the monster table like the names (its second half, the other
    /// gender, shares the first's), not the entity ids the level tables
    /// follow: the two part ways at Unown's forms.
    pub(crate) fn learnset(&self, md_index: usize) -> Arc<Vec<(u8, MoveSlot)>> {
        let learnset = self
            .moves
            .level_up(md_index % MONSTER_FORMS)
            .iter()
            .filter_map(|&(id, level)| Some((level, move_slot(id, self.moves.get(id as usize)?))))
            .collect();
        Arc::new(learnset)
    }

    /// What terrain a species can cross (see `Mobility::resolve`).
    pub(crate) fn movement_type(&self, md_index: usize) -> u8 {
        self.monsters.species(md_index).map_or(0, |species| species.movement_type)
    }

    /// The regular attack every monster has, as the ROM's move table
    /// describes it.
    pub(crate) fn regular_attack(&self) -> Option<MoveSlot> {
        (0..self.moves.move_count())
            .filter_map(|index| self.moves.get(index))
            .find(|m| m.move_id == dungeon::REGULAR_ATTACK_ID)
            .map(|m| move_slot(dungeon::REGULAR_ATTACK_ID, m))
    }
    /// Both gender entries of a species share the name of their entity id.
    pub(crate) fn monster_name(&self, md_index: usize) -> String {
        // Names follow the monster table (whose second half repeats the
        // first for the other gender), not the entity ids: the two part
        // ways at Unown's forms.
        let name = self.names.monsters.get(md_index % MONSTER_FORMS);
        name.cloned().unwrap_or_default()
    }
    /// The floor laid out by hand, if the floor is one: its tiles, and its
    /// monsters as the floor's enemies are described.
    pub(crate) fn fixed_floor(&self, id: u8) -> Option<Fixed> {
        // (From 165 on, rooms set inside a generated floor: not made.)
        let room = (1..165).contains(&id).then(|| rom_formats::fixed::load_room(&self.rom, id))??;
        let enemy = |placed: &rom_formats::fixed::FixedMonster| {
            let md_index = placed.monster as usize;
            let species = self.monsters.species(md_index)?;
            let natural = self.monsters.stats_at(md_index, placed.level.min(100) as u8)?;
            Some(Enemy {
                species: placed.monster,
                recruit_rate: 0,
                // The room's own figures, where it gives them.
                // (No level of its own: as the floor's; no HP: its species'.)
                stats: dungeon::Stats {
                    level: placed.level.max(1) as i32,
                    max_hp: if placed.hp > 0 { placed.hp as i32 } else { natural.max_hp },
                    hp: if placed.hp > 0 { placed.hp as i32 } else { natural.max_hp },
                    attack: if placed.attack > 0 { placed.attack as i32 } else { natural.attack },
                    defense: if placed.defense > 0 { placed.defense as i32 } else { natural.defense },
                    sp_attack: if placed.sp_attack > 0 { placed.sp_attack as i32 } else { natural.sp_attack },
                    sp_defense: if placed.sp_defense > 0 { placed.sp_defense as i32 } else { natural.sp_defense },
                    types: natural.types,
                    ..natural
                },
                asleep_chance: 0,
                exp: match placed.exp {
                    given if given > 0 => given as i32,
                    _ => dungeon::exp_value(species.exp_yield as i32, placed.level as i32),
                },
                weight: 0,
            })
        };
        // (`monster_behavior`: foes, a rescue target and those of the mazes
        // and the Secret Bazaar who only stand there, the ally.)
        let of = |wanted: fn(u8) -> bool| -> Vec<Placed> {
            let placed = room.monsters.iter().filter(|(_, placed)| wanted(placed.behavior));
            placed.filter_map(|(at, placed)| Some(Placed { at: *at, enemy: enemy(placed)?, facing: placed.facing })).collect()
        };
        Some(Fixed {
            width: room.width,
            height: room.height,
            tiles: room.tiles.iter().map(|tile| *tile as u8).collect(),
            leader: room.leader,
            monsters: of(|behavior| matches!(behavior, 0..=6 | 8 | 9 | 21)),
            allies: of(|behavior| behavior == 10),
            watchers: of(|behavior| matches!(behavior, 7 | 11..=20)),
            items: room.items.clone(),
            traps: room.traps.clone(),
            exit: room.exit,
            ends_with_enemies: room.ends_with_enemies,
            moves_allowed: room.moves_allowed,
            orbs_allowed: room.orbs_allowed,
        })
    }

    /// The song of a fixed room that has its own.
    fn fixed_music(&self, id: u8) -> Option<u16> {
        rom_formats::fixed::load_room(&self.rom, id).map(|room| room.music).filter(|&song| (1..165).contains(&id) && song != 0)
    }

    /// A floor's figures, as the tables keep them.
    pub(crate) fn floor(&self, floor: &FloorData) -> Floor {
        let tileset = floor.layout.tileset;
        let enemy = |spawn: &rom_formats::mappa::MonsterSpawn| {
            let md_index = spawn.md_index as usize;
            let species = self.monsters.species(md_index)?;
            Some(Enemy {
                species: spawn.md_index,
                recruit_rate: species.recruit_rate,
                stats: self.monsters.stats_at(md_index, spawn.level)?,
                asleep_chance: species.spawn_asleep_chance as i32,
                exp: dungeon::exp_value(species.exp_yield as i32, spawn.level as i32),
                weight: spawn.weight,
            })
        };
        let items = |list: &rom_formats::mappa::ItemList| ItemTable {
            categories: list.categories.clone(),
            items: list.items.iter().filter_map(|&(id, weight)| Some((id, self.items.category(id as usize)?, weight))).collect(),
        };
        let moves = rom_formats::tileset::tileset_moves(&self.rom, tileset);
        Floor {
            fixed: self.fixed_floor(floor.layout.fixed_room_id),
            props: floor.layout.properties(),
            weather: floor.layout.weather,
            secondary_kind: rom_formats::tileset::secondary_terrain_type(&self.rom, tileset).unwrap_or(0),
            camouflage: moves.map_or(1, |moves| moves.0),
            nature_power: moves.map_or(0, |moves| moves.1),
            secret_power: moves.map_or(9, |moves| moves.2),
            // (Of a random group, its first.)
            music: self.fixed_music(floor.layout.fixed_room_id).or(self.dungeons.songs(floor.layout.music).first().copied().filter(|&song| song != NO_MUSIC)),
            enemies: floor.spawns.iter().filter_map(enemy).collect(),
            items: items(&floor.items),
            traps: floor.traps.to_vec(),
            shop_items: items(&floor.shop_items),
            max_money: floor.layout.max_money_div_5 as i32 * 5,
            tileset,
            enemy_iq: floor.layout.enemy_iq as i32,
            visibility: floor.layout.visibility_range as i32,
            shop_pattern: floor.layout.shop_item_positions,
            sticky_chance: floor.layout.sticky_item_chance as i32,
        }
    }

    /// The game's songs, short tunes and sounds, read once (see
    /// `rom_formats::sedl::library`).
    pub(crate) fn library(&self) -> &shell::music::Library {
        self.library.get_or_init(|| {
            let files = self.rom.paths().filter_map(|path| Some((path.strip_prefix("SOUND/")?, self.rom.file(path)?)));
            rom_formats::sedl::library(files)
        })
    }

    pub(crate) fn quiz(&self) -> Option<Arc<rules::quiz::QuizData>> {
        let load = || {
            let text = self.rom.file(&format!("MESSAGE/text_{LANGUAGE}.str"))?;
            let strings = rom_formats::text::Strings::parse(text).ok()?;
            let quiz = rom_formats::quiz::load(&self.rom, &strings);
            quiz.inspect_err(|e| eprintln!("personality test unavailable: {e}")).ok().map(Arc::new)
        };
        self.quiz.get_or_init(load).clone()
    }

    pub(crate) fn dungeon_name(&self, id: u16) -> String {
        self.names.dungeons.get(id as usize).cloned().unwrap_or_default()
    }
    pub(crate) fn dungeon_floors(&self, id: u16) -> u32 {
        match (id as usize) < self.dungeons.dungeons.len() {
            true => Self::floor_range(&self.mappa, &self.dungeons, id as usize).len() as u32,
            false => 0,
        }
    }

    pub(crate) fn monster_types(&self, monster: u16) -> (u8, u8) {
        self.monsters.stats_at(monster as usize, 1).map_or((0, 0), |stats| stats.types)
    }
    pub(crate) fn move_effects(&self, move_id: u16) -> [u16; 4] {
        rom_formats::script::load_move_animation(&self.rom, move_id).unwrap_or_default()
    }
    pub(crate) fn move_sound(&self, move_id: u16) -> u16 {
        rom_formats::script::move_sound(&self.rom, move_id).unwrap_or_default()
    }

    pub(crate) fn map_marker(&self, marker: u16) -> Option<(u16, f32, f32)> {
        let (level, x, y) = rom_formats::script::load_map_marker(&self.rom, marker)?;
        Some((level, x as f32, y as f32))
    }

    pub(crate) fn trap_effect(&self, trap: usize) -> u16 {
        rom_formats::script::load_trap_animation(&self.rom, trap).unwrap_or_default()
    }

    pub(crate) fn item_effects(&self, item: u16) -> [u16; 2] {
        rom_formats::script::load_item_animation(&self.rom, item).unwrap_or_default()
    }
    pub(crate) fn shadow_size(&self, monster: u16) -> Option<u8> {
        self.monsters.species(monster as usize).map(|species| species.shadow_size.min(2))
    }
    pub(crate) fn item_prices(&self, item: u16) -> (i32, i32) {
        let (buy, sell) = self.items.prices(item as usize).unwrap_or((0, 0));
        (buy as i32, sell as i32)
    }

    pub(crate) fn common_routine(&self, number: usize) -> Option<Program> {
        Some(Program { script: self.common.clone()?, common: self.common.clone(), opcodes: self.opcodes.clone(), routine: number })
    }
    // `[hero]`, `[partner]` and the tags naming a character of the script
    // engine (`[c_name:NPC_X]`, `[c_kind:NPC_X]`: its name, its species).
    pub(crate) fn script_text(&self, text: &str) -> String {
        let (hero, partner) = ("{hero}", "{partner}");
        let mut filled = String::new();
        let mut rest = text;
        while let Some(open) = rest.find('[') {
            filled.push_str(&rest[..open]);
            let Some(close) = rest[open..].find(']') else {
                break;
            };
            let tag = &rest[open + 1..open + close];
            let entity = tag.strip_prefix("c_name:").or(tag.strip_prefix("c_kind:"));
            match (tag, entity) {
                ("hero" | "player", _) => filled.push_str(hero),
                ("partner", _) => filled.push_str(partner),
                (_, Some(name)) if name.starts_with("PLAYER") => filled.push_str(hero),
                (_, Some(name)) if name.starts_with("ATTENDANT") => filled.push_str(partner),
                (_, Some(name)) => {
                    let known = self.entities.iter().find(|entity| entity.name == name);
                    if let Some(entity) = known.filter(|entity| entity.monster != 0) {
                        filled.push_str(&self.monster_name(entity.monster as usize));
                    }
                }
                _ => filled.push_str(&rest[open..open + close + 1]),
            }
            rest = &rest[open + close + 1..];
        }
        filled + rest
    }
}

/// A move as monsters hold it, from the ROM's move data.
/// Whether a file of the game's `SOUND` folder (its path there, folders
/// parted by `parts`) is a bank of sounds: not a song's nor a jingle's,
/// which are numbered apart and may bear the same number.
pub(crate) fn move_slot(id: u16, m: &rom_formats::moves::Move) -> MoveSlot {
    MoveSlot {
        id,
        power: m.base_power as i32,
        accuracy: m.accuracy as i32,
        accuracy2: m.accuracy2 as i32,
        strikes: m.strikes as i32,
        pp: m.pp as i32,
        max_pp: m.pp as i32,
        type_id: m.type_id,
        category: m.category,
        sealed: false,
        crit_chance: m.crit_chance as i32,
        // Low nibble: who is targeted; next nibble: the range.
        target: (m.target_range & 0xF) as u8,
        range: (m.target_range >> 4 & 0xF) as u8,
        ai_weight: m.ai_weight as i32,
        ai_target_range: m.ai_target_range,
        ai_chance: m.ai_chance as i32,
        linked: false,
        talented: false,
        off: false,
    }
}
