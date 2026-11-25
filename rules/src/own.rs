//! A dungeon of one's own, written as text (a `.dungeon` file): its name, how
//! many floors, the look and shape of its floors (taken from one of the
//! game's dungeons, its floors made anew each time as any), its music, the
//! Pokémon met there and at what level, what lies on its floors.
//!
//! ```text
//! name: Firefly Forest
//! floors: 4
//! look: Apple Woods 2          (a dungeon of the game, a floor of it; or its number)
//! tileset: blue_cave           (its tiles drawn by one: library/tilesets/<word>.png)
//! music: apple_woods           (a song's word, or its number; none: the look's)
//! pokemon: Caterpie 3, Weedle 4, Pidgey 5 x2   (species, level, how often)
//! items: Apple, Oran Berry x3, Blast Seed
//! recruiting: yes
//! upward: yes                  (1F, 2F… rather than B1F, B2F…)
//! rules: fairy, push_allies    (options of the rules: `crate::rule_options`)
//! pokemon_type: Clefairy fairy (a species' types there, one or two)
//! move_type: Charm fairy       (a move's type there)
//! rank: E floors 1-2, D        (the jobs' rank there, for the boards: E, D, C,
//!                              B, A, S, then 1* to 9*; by floors, or all of
//!                              them; none said: from the levels met there)
//! ```

use diagnostics::Diagnostic;
use diagnostics::text::{Kept, normalise};

use crate::dungeon::exp::Growth;
use crate::tables::{DungeonTable, Enemy, Floor, Tables};

/// The first number of the tilesets of one's own (`shell::art::OWN_TILESETS`):
/// the game's stop before it.
pub const OWN_TILESETS: u8 = 200;

/// A word as it is sought: small letters, no accents, no spaces, nothing
/// but letters and digits (`diagnostics::text::Kept::Letters`).
pub fn plain(word: &str) -> String {
    normalise(word, Kept::Letters)
}

/// The codes of what is wrong in a dungeon's text.
pub mod codes {
    /// A dungeon's text that does not read, or a dungeon that cannot be made of it.
    pub const WRONG: &str = "E-DUNGEON";
}

/// A dungeon read from its text: its name, its table, the level the team
/// comes in at (if it says).
pub struct Own {
    pub name: String,
    pub table: DungeonTable,
    pub team_level: Option<i32>,
    /// Its boss, for whoever plays the fight: who, and what it says.
    pub boss: Option<BossSays>,
    /// The types it gives species and moves (see [`retype`]).
    pub species_types: Vec<(u16, (u8, u8))>,
    pub move_types: Vec<(u16, u8)>,
    /// The rank of each floor, for the jobs of the boards (see [`rank_floors`]).
    pub ranks: Vec<u8>,
}

/// The types a dungeon gives species and moves, written into the tables it
/// is played with: the species, the foes of every floor, every copy of the
/// move (the team's own are matched to them as it comes in).
pub fn retype(tables: &mut Tables, species_types: &[(u16, (u8, u8))], move_types: &[(u16, u8)]) {
    for &(species, types) in species_types {
        if let Some(kind) = tables.species.get_mut(species as usize) {
            kind.types = types;
            for (stats, _) in kind.hero.iter_mut().chain(kind.partner.iter_mut()) {
                stats.types = types;
            }
        }
        let floors = tables.dungeons.iter_mut().flat_map(|dungeon| dungeon.floors.iter_mut());
        for floor in floors {
            let fixed = floor.fixed.iter_mut().flat_map(|fixed| fixed.monsters.iter_mut().chain(fixed.allies.iter_mut()).chain(fixed.watchers.iter_mut()));
            let placed = fixed.map(|placed| &mut placed.enemy);
            for enemy in floor.enemies.iter_mut().chain(placed).filter(|enemy| enemy.species == species) {
                enemy.stats.types = types;
            }
        }
    }
    for &(id, kind) in move_types {
        let retyped = |slot: &mut crate::dungeon::MoveSlot| {
            if slot.id == id {
                slot.type_id = kind;
            }
        };
        tables.move_slots.iter_mut().for_each(retyped);
        for species in &mut tables.species {
            species.learnset.iter_mut().for_each(|(_, slot)| retyped(slot));
            for (_, moves) in species.hero.iter_mut().chain(species.partner.iter_mut()) {
                moves.iter_mut().for_each(retyped);
            }
        }
    }
}

/// A species and its types (`Clefairy fairy`, `Jigglypuff normal fairy`).
fn species_typed(said: &str, tables: &Tables) -> Result<(u16, (u8, u8)), String> {
    let words: Vec<&str> = said.split_whitespace().collect();
    // (The name may have spaces: the types are the last words.)
    for cut in (1..words.len()).rev() {
        let name = words[..cut].join(" ");
        let Some(species) = tables.species.iter().position(|kind| plain(&kind.name) == plain(&name)) else {
            continue;
        };
        let kinds: Option<Vec<u8>> = words[cut..].iter().map(|word| crate::dungeon::type_named(word)).collect();
        return match kinds.as_deref() {
            Some([first]) => Ok((species as u16, (*first, 0))),
            Some([first, second]) => Ok((species as u16, (*first, *second))),
            _ => Err(format!("\"{said}\": one or two types after the Pokémon ({})", crate::dungeon::TYPE_NAMES[1..].join(", "))),
        };
    }
    Err(format!("\"{said}\": a Pokémon of the game, then its types (\"Clefairy fairy\")"))
}

/// A move and its type (`Charm fairy`).
fn move_typed(said: &str, tables: &Tables) -> Result<(u16, u8), String> {
    let (name, kind) = said.rsplit_once(' ').ok_or_else(|| format!("\"{said}\": a move, then its type (\"Charm fairy\")"))?;
    let Some(id) = tables.moves.iter().position(|known| plain(known) == plain(name)) else {
        return Err(format!("\"{name}\" is not a move of the game"));
    };
    match crate::dungeon::type_named(kind) {
        Some(kind) => Ok((id as u16, kind)),
        None => Err(format!("\"{kind}\" is not a type ({})", crate::dungeon::TYPE_NAMES[1..].join(", "))),
    }
}

/// A boss as its fight is played: its species and name, what it says as the
/// fight begins, what it says when half its life is gone (and it grows
/// fiercer by so many stages, 0: it does not), what it says as it falls.
/// Also: the moves it knows (the game's numbers, none: its own); its phases,
/// each played once as its life falls under its share; whether it asks to
/// join the team, beaten.
#[derive(Default)]
pub struct BossSays {
    pub species: u16,
    pub name: String,
    pub begins: String,
    pub falls: String,
    pub moves: Vec<u16>,
    pub phases: Vec<Phase>,
    pub joins: bool,
    /// A sound of ours it cries as the fight begins (empty: none).
    pub cry: String,
    /// What it is called under its name, as the fight begins (`The Elder of the Deep`).
    pub title: String,
    /// The team the fight is played by, when it is played alone (none: the
    /// one that comes): the leader's species, its partner's, their level.
    pub hero: Option<u16>,
    pub partner: Option<u16>,
    pub level: Option<i32>,
}

/// A phase of a boss fight: under so many hundredths of its life, it says
/// that, grows fiercer by so many stages, is helped by those, gets back so
/// many hundredths of its life, turns the weather to that, knows those moves.
#[derive(Default)]
pub struct Phase {
    pub share: i32,
    pub says: String,
    pub fiercer: i32,
    pub help: Vec<Enemy>,
    pub heal: i32,
    pub weather: Option<u8>,
    pub moves: Vec<u16>,
    /// A sound of ours it cries then (empty: none).
    pub cry: String,
    /// The screen shakes as it begins.
    pub shake: bool,
}

/// The traps an arena may hold, by their words, and the game's kinds (none
/// that takes one off the floor: no pitfall, no warp).
pub const TRAPS: [(&str, u8); 20] = [
    ("mud", 1),
    ("sticky", 2),
    ("grimy", 3),
    ("summon", 4),
    ("gust", 7),
    ("spin", 8),
    ("slumber", 9),
    ("slow", 10),
    ("seal", 11),
    ("poison", 12),
    ("selfdestruct", 13),
    ("explosion", 14),
    ("pp_zero", 15),
    ("chestnut", 16),
    ("wonder", 17),
    ("spikes", 19),
    ("stealth_rock", 20),
    ("toxic_spikes", 21),
    ("trip", 22),
    ("grudge", 24),
];

/// `Krabby 7 @5,3`: what is said, and the tile said after `@` (from the room's top left corner).
pub fn placed_at(said: &str) -> (&str, Option<Tile>) {
    match said.rsplit_once('@') {
        Some((before, at)) => {
            let tile = at.trim().split_once([',', ' ']).and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?)));
            (before.trim(), tile)
        }
        None => (said.trim(), None),
    }
}

/// A list split at its commas, a `@x,y` kept whole (`Krabby 7 @5,3, Krabby 7`).
pub fn split_placed(value: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for piece in value.split(',') {
        // (After an `@x`, the piece that follows is its y.)
        match parts.last_mut() {
            Some(last)
                if last.rsplit_once('@').is_some_and(|(_, at)| !at.contains(',') && !at.trim().contains(' '))
                    && piece.split_whitespace().next().is_some_and(|first| first.parse::<usize>().is_ok()) =>
            {
                let mut words = piece.trim().splitn(2, ' ');
                last.push(',');
                last.push_str(words.next().unwrap_or_default());
                if let Some(rest) = words.next() {
                    last.push(' ');
                    last.push_str(rest);
                }
            }
            _ if !piece.trim().is_empty() => parts.push(piece.trim().to_string()),
            _ => {}
        }
    }
    parts
}

/// `7,3` or `7 3`: a tile.
fn tile_said(said: &str) -> Option<Tile> {
    let (x, y) = said.trim().split_once([',', ' '])?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// The weathers a phase may bring, by their words.
pub const WEATHERS: [(&str, u8); 6] = [("clear", 0), ("sunny", 1), ("sandstorm", 2), ("rain", 4), ("hail", 5), ("fog", 6)];

/// `70% | says You shall not pass! | anger +1 | reinforcements Caterpie 6, Weedle 6 | heal 30% | weather hail | moves Gust`:
/// a phase, or what is wrong with it.
fn phase(value: &str, tables: &Tables) -> Result<Phase, Vec<String>> {
    let mut parts = value.split('|').map(str::trim);
    let share = parts
        .next()
        .and_then(hundredths)
        .ok_or_else(|| vec!["a phase begins with the share of its life under which it comes: \"50% | anger +1\"".to_string()])?;
    let (mut made, mut wrong) = (Phase { share, ..Phase::default() }, Vec::new());
    for part in parts.filter(|part| !part.is_empty()) {
        let (word, rest) = part.split_once(' ').map_or((part, ""), |(word, rest)| (word, rest.trim()));
        match plain(word).as_str() {
            "says" => made.says = rest.to_string(),
            "anger" => made.fiercer = rest.trim_start_matches('+').parse().unwrap_or(1),
            "reinforcements" => {
                for one in rest.split(',').map(str::trim).filter(|one| !one.is_empty()) {
                    match foe(one, tables) {
                        Ok(enemy) => made.help.push(enemy),
                        Err(why) => wrong.push(why),
                    }
                }
                if made.help.len() > 6 {
                    wrong.push("six reinforcements at most".to_string());
                }
            }
            "heal" => match hundredths(rest) {
                Some(back) => made.heal = back,
                None => wrong.push("\"heal 30%\": it gets back so much of its life".to_string()),
            },
            "weather" => match WEATHERS.iter().find(|(said, _)| *said == plain(rest)) {
                Some((_, kind)) => made.weather = Some(*kind),
                None => wrong.push(format!("\"weather {rest}\"? ({})", WEATHERS.map(|(said, _)| said).join(", "))),
            },
            "moves" => {
                for said in rest.split(',').map(str::trim).filter(|said| !said.is_empty()) {
                    match tables.moves.iter().position(|known| plain(known) == plain(said)) {
                        Some(id) if made.moves.len() < 4 => made.moves.push(id as u16),
                        Some(_) => wrong.push("four moves at most".to_string()),
                        None => wrong.push(format!("\"{said}\" is not a move of the game")),
                    }
                }
            }
            "cry" => made.cry = rest.to_string(),
            "shake" => made.shake = true,
            other => wrong.push(format!("\"{other}\"? (says, anger, reinforcements, heal, weather, moves, cry, shake)")),
        }
    }
    if wrong.is_empty() { Ok(made) } else { Err(wrong) }
}

/// `30%`, `30`: so many hundredths.
fn hundredths(said: &str) -> Option<i32> {
    said.trim().trim_end_matches('%').trim().parse::<i32>().ok().filter(|share| (1..=99).contains(share))
}

/// The floors something is on (from 1, both ends in), all if not said.
type Floors = Option<(usize, usize)>;

/// `Pidgey 6 x2 floors 2-3`: what is said, and the floors (`floor 3`: that one).
fn on_floors(said: &str) -> (&str, Floors) {
    let words: Vec<&str> = said.split_whitespace().collect();
    if words.len() >= 3 && matches!(plain(words[words.len() - 2]).as_str(), "floor" | "floors") {
        let range = words[words.len() - 1];
        let (low, high) = range.split_once('-').unwrap_or((range, range));
        if let (Ok(low), Ok(high)) = (low.trim().parse::<usize>(), high.trim().parse::<usize>()) {
            let cut = said.rfind(words[words.len() - 2]).unwrap_or(said.len());
            return (said[..cut].trim(), Some((low.min(high).max(1), low.max(high))));
        }
    }
    (said, None)
}

/// The keys a dungeon's text knows, as its faults name them.
const KEYS_SAID: &str = "name, floors, look, tileset, music, pokemon, items, recruiting, upward, level, boss, boss_guards, boss_hp, boss_arena, boss_shape, boss_music, boss_says, boss_anger, boss_falls, boss_item, boss_moves, boss_reinforcements, boss_heal, boss_phase, boss_recruit, boss_cry, boss_at, boss_team_at, boss_floor_items, boss_traps, boss_title, boss_look, boss_map, boss_allies, boss_hero, boss_partner, boss_level, fight, rules, pokemon_type, move_type, rank";

/// A species and the types it is given there.
type SpeciesTyped = (u16, (u8, u8));

/// A dungeon's text being read: the line it is at, what its keys said so
/// far (the types given with the line that gave them), what is wrong.
struct Reading<'a> {
    tables: &'a Tables,
    number: usize,
    wrong: Vec<Diagnostic>,
    name: String,
    floors: usize,
    look: Option<(u16, usize)>,
    music: Option<u16>,
    foes: Vec<(Enemy, Floors)>,
    items: Vec<(u16, u16, Floors)>,
    recruit: bool,
    goes_up: bool,
    team_level: Option<i32>,
    boss: Boss,
    tileset: Option<u8>,
    options: crate::rule_options::RuleOptions,
    species_types: Vec<(usize, SpeciesTyped)>,
    move_types: Vec<(usize, (u16, u8))>,
    /// The jobs' ranks said, and on which floors.
    ranks: Vec<(u8, Floors)>,
}

impl Reading<'_> {
    /// What is wrong on the line read.
    fn wrong(&mut self, why: impl std::fmt::Display) {
        self.wrong.push(Diagnostic::at(self.number, why.to_string()));
    }
}

/// What a key does with its value.
type Handler = fn(&mut Reading<'_>, &str);

/// Each key (as it is sought: `boss_guards` is `bossguards`) and what it does.
const KEYS: &[(&str, Handler)] = &[
    ("name", |reading, value| reading.name = value.to_string()),
    ("floors", floors_said),
    ("boss", |reading, value| match foe(value, reading.tables) {
        Ok(enemy) => reading.boss.boss = Some(enemy),
        Err(why) => reading.wrong(why),
    }),
    ("bossguards", |reading, value| placed_foes(reading, value, false)),
    ("bossat", |reading, value| match tile_said(value) {
        Some(at) => reading.boss.place = Some(at),
        None => reading.wrong("the boss's tile, \"7 3\""),
    }),
    ("bossallies", |reading, value| placed_foes(reading, value, true)),
    ("bosshero", |reading, value| boss_team(reading, value, true)),
    ("bosspartner", |reading, value| boss_team(reading, value, false)),
    ("bosslevel", |reading, value| match value.parse::<i32>() {
        Ok(level @ 1..=100) => reading.boss.says.level = Some(level),
        _ => reading.wrong("the team's level for this fight, from 1 to 100"),
    }),
    ("bossteamat", |reading, value| match tile_said(value) {
        Some(at) => reading.boss.team = Some(at),
        None => reading.wrong("the tile where the team comes in, \"7 8\""),
    }),
    ("bossflooritems", floor_items),
    ("bosstraps", traps),
    ("bosshp", |reading, value| match value.trim_start_matches(['x', '×']).trim().parse::<f32>() {
        Ok(times) if (0.5..=20.0).contains(&times) => reading.boss.life = times,
        _ => reading.wrong("its life times how much (x1 to x20)"),
    }),
    ("bossarena", |reading, value| {
        match value.split_once(['x', '×']).and_then(|(wide, high)| Some((wide.trim().parse::<usize>().ok()?, high.trim().parse::<usize>().ok()?))) {
            Some((wide @ 7..=40, high @ 6..=24)) => reading.boss.room = (wide, high),
            _ => reading.wrong("the arena's size, \"13x9\" (from 7x6 to 40x24)"),
        }
    }),
    ("bossshape", |reading, value| match plain(value).as_str() {
        shape @ ("room" | "pillars" | "pool" | "cross") => reading.boss.shape = shape.to_string(),
        _ => reading.wrong("the arena's shape: room, pillars, pool or cross"),
    }),
    ("bosssays", |reading, value| reading.boss.says.begins = value.to_string()),
    ("bossanger", anger),
    ("bossphase", |reading, value| match phase(value, reading.tables) {
        Ok(made) => reading.boss.says.phases.push(made),
        Err(why) => why.into_iter().for_each(|why| reading.wrong(why)),
    }),
    ("bossrecruit", |reading, value| reading.boss.says.joins = yes(value)),
    ("bosscry", |reading, value| reading.boss.says.cry = value.to_string()),
    ("bosstitle", |reading, value| reading.boss.says.title = value.to_string()),
    // (`..X~/....`: the arena painted, a row each between `/`: `.` ground, `X` wall, `~` water.)
    ("bossmap", |reading, value| {
        reading.boss.map = Some(
            value
                .split('/')
                .map(|row| {
                    row.trim()
                        .chars()
                        .map(|tile| match tile {
                            'X' | 'x' => 1,
                            '~' => 2,
                            _ => 0,
                        })
                        .collect()
                })
                .collect(),
        )
    }),
    // (The arena's own look, a game dungeon's floor: a fight of its own has one.)
    ("bosslook", |reading, value| match looked(value, reading.tables) {
        Some(found) => reading.boss.look = Some(found),
        None => reading.wrong(format!("\"{value}\" is not a dungeon of the game (its name, or its number, then a floor)")),
    }),
    ("fight", |reading, value| reading.wrong(format!("\"fight: {value}\": the fight was not read (library/fights/{value}.fight?)"))),
    ("bossfalls", |reading, value| reading.boss.says.falls = value.to_string()),
    ("bossmoves", boss_moves),
    ("bossreinforcements", reinforcements),
    // (`25% 30%`: under 25 % of its life, it gets back 30 % of it, once.)
    ("bossheal", |reading, value| match value.split_once(' ').and_then(|(under, back)| Some((hundredths(under)?, hundredths(back)?))) {
        Some((under, back)) => reading.boss.says.phases.push(Phase { share: under, heal: back, ..Phase::default() }),
        None => reading.wrong("\"25% 30%\": under 25% of its life, it gets back 30% of it, once"),
    }),
    ("bossitem", |reading, value| match reading.tables.items.iter().position(|item| plain(item) == plain(value)) {
        Some(item) => reading.boss.drop = Some(item as u16),
        None => reading.wrong(format!("\"{value}\" is not an item of the game")),
    }),
    ("bossmusic", |reading, value| match song(value) {
        Some(song) => reading.boss.music = Some(song),
        None => reading.wrong(format!("\"{value}\" is not a song (its word, or its number)")),
    }),
    ("look", |reading, value| match looked(value, reading.tables) {
        Some(found) => reading.look = Some(found),
        None => reading.wrong(format!("\"{value}\" is not a dungeon of the game (its name, or its number, then a floor)")),
    }),
    // (Its number once the reader of the text has found its picture,
    // `tileset: <word>` becoming `tileset: 200`: see `game::dive::resolved`.)
    ("tileset", |reading, value| match value.parse::<u8>() {
        Ok(id) if id >= OWN_TILESETS => reading.tileset = Some(id),
        _ => reading.wrong(format!("no tileset \"{value}\" (library/tilesets/{value}.png, drawn on the template)")),
    }),
    ("music", |reading, value| match song(value) {
        Some(song) => reading.music = Some(song),
        None => reading.wrong(format!("\"{value}\" is not a song (its word, or its number)")),
    }),
    ("pokemon", pokemon),
    ("items", items),
    ("recruiting", |reading, value| reading.recruit = yes(value)),
    ("upward", |reading, value| reading.goes_up = yes(value)),
    ("rules", rules),
    ("pokemontype", |reading, value| {
        for part in listed(value) {
            match species_typed(part, reading.tables) {
                Ok(typed) => reading.species_types.push((reading.number, typed)),
                Err(why) => reading.wrong(why),
            }
        }
    }),
    ("movetype", |reading, value| {
        for part in listed(value) {
            match move_typed(part, reading.tables) {
                Ok(typed) => reading.move_types.push((reading.number, typed)),
                Err(why) => reading.wrong(why),
            }
        }
    }),
    ("rank", ranks_said),
    ("level", |reading, value| match value.parse::<i32>() {
        Ok(level @ 1..=100) => reading.team_level = Some(level),
        _ => reading.wrong("the team's level coming in, from 1 to 100"),
    }),
];

/// `yes`, `true`, `1`.
fn yes(value: &str) -> bool {
    matches!(plain(value).as_str(), "yes" | "true" | "1")
}

/// A list's parts, at its commas, the empty ones left out.
fn listed(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').map(str::trim).filter(|part| !part.is_empty())
}

/// A song by its number or its word.
fn song(value: &str) -> Option<u16> {
    value.parse::<u16>().ok().or_else(|| crate::songs::SONGS.iter().find(|song| plain(song.word) == plain(value)).map(|song| song.number))
}

/// `rank: E floors 1-2, D floor 3` (or `E 1-2`), `rank: C`: the jobs' ranks.
fn ranks_said(reading: &mut Reading<'_>, value: &str) {
    for part in listed(value) {
        match rank_part(part) {
            Ok(said) => reading.ranks.push(said),
            Err(why) => reading.wrong(why),
        }
    }
}

/// `E floors 1-2`, `E 1-2`, `C`: a rank and its floors.
fn rank_part(part: &str) -> Result<(u8, Floors), String> {
    let (said, floors) = match on_floors(part) {
        (said, None) => match said.split_once(' ') {
            // (`E 1-2`: the floors without their word.)
            Some((rank, range)) => on_floors(&format!("{rank} floors {range}")).1.map_or((said, None), |floors| (rank, Some(floors))),
            None => (said, None),
        },
        said => said,
    };
    match crate::jobs::rank_named(said) {
        Some(rank) => Ok((rank, floors)),
        None => Err(format!("\"{said}\" is not a job rank (E, D, C, B, A, S, then 1* to 9*; \"E floors 1-2, D floor 3\")")),
    }
}

/// The jobs' rank on a floor: the last said for it, else as hard as the
/// levels of those met there (`levels`: each with its floors).
fn rank_on(said: &[(u8, Floors)], levels: &[(i32, Floors)], at: usize) -> u8 {
    if let Some((rank, _)) = said.iter().rev().find(|(_, floors)| on(*floors, at)) {
        return *rank;
    }
    let here: Vec<i32> = levels.iter().filter(|(_, floors)| on(*floors, at)).map(|(level, _)| *level).collect();
    crate::jobs::rank_for_level(here.iter().sum::<i32>() / (here.len().max(1) as i32))
}

fn floors_said(reading: &mut Reading<'_>, value: &str) {
    match value.parse::<usize>() {
        Ok(count @ 0..=99) => reading.floors = count,
        _ => reading.wrong("a number of floors from 0 (the fight alone) to 99"),
    }
}

/// `Krabby 7 @5,3, Krabby 7`: the boss's guards (or those on the team's
/// side), each where said, or where the arena puts them (by the team).
fn placed_foes(reading: &mut Reading<'_>, value: &str, allies: bool) {
    for part in split_placed(value) {
        let (part, at) = placed_at(&part);
        match foe(part, reading.tables) {
            Ok(enemy) if allies => reading.boss.allies.push((enemy, at)),
            Ok(enemy) => reading.boss.guards.push((enemy, at)),
            Err(why) => reading.wrong(why),
        }
    }
}

/// The team of the fight played alone: `Pikachu`, `Piplup`.
fn boss_team(reading: &mut Reading<'_>, value: &str, hero: bool) {
    match reading.tables.species.iter().position(|kind| plain(&kind.name) == plain(value)) {
        Some(id) if hero => reading.boss.says.hero = Some(id as u16),
        Some(id) => reading.boss.says.partner = Some(id as u16),
        None => reading.wrong(format!("\"{value}\" is not a Pokémon of the game")),
    }
}

/// `Sitrus Berry @3,4, Blast Seed @10,6`: what lies where.
fn floor_items(reading: &mut Reading<'_>, value: &str) {
    for part in split_placed(value) {
        let (said, at) = placed_at(&part);
        match (reading.tables.items.iter().position(|item| plain(item) == plain(said)), at) {
            (Some(item), Some(at)) => reading.boss.lying.push((at, item as u16)),
            (None, _) => reading.wrong(format!("\"{said}\" is not an item of the game")),
            (_, None) => reading.wrong(format!("\"{said}\": where? (\"{said} @3,4\")")),
        }
    }
}

/// `explosion @4,4, spikes @9,5 hidden`: what trap where, shown or hidden.
fn traps(reading: &mut Reading<'_>, value: &str) {
    for part in split_placed(value) {
        let hidden = part.trim_end().ends_with(" hidden");
        let part = part.trim_end().trim_end_matches(" hidden").to_string();
        let (said, at) = placed_at(&part);
        match (TRAPS.iter().find(|(word, _)| plain(word) == plain(said)), at) {
            (Some((_, kind)), Some(at)) => reading.boss.traps.push((at, *kind, !hidden)),
            (None, _) => reading.wrong(format!("\"{said}\"? ({})", TRAPS.map(|(word, _)| word).join(", "))),
            (_, None) => reading.wrong(format!("\"{said}\": where? (\"{said} @3,4\")")),
        }
    }
}

/// `+2 Enough games!`: so many stages fiercer at half its life, then what it says; `0`: never.
fn anger(reading: &mut Reading<'_>, value: &str) {
    let (stages, said) = match value.split_once(' ').filter(|(first, _)| first.starts_with(['+', '0'])) {
        Some((first, rest)) => (first.trim_start_matches('+').parse().unwrap_or(1), rest.trim()),
        None if value.starts_with(['+', '0']) => (value.trim_start_matches('+').parse().unwrap_or(1), ""),
        None => (1, value),
    };
    reading.boss.says.phases.push(Phase { share: 50, says: said.to_string(), fiercer: stages, ..Phase::default() });
}

/// The boss's moves, four at most.
fn boss_moves(reading: &mut Reading<'_>, value: &str) {
    for said in listed(value) {
        match reading.tables.moves.iter().position(|known| plain(known) == plain(said)) {
            Some(id) if reading.boss.says.moves.len() < 4 => reading.boss.says.moves.push(id as u16),
            Some(_) => reading.wrong("four moves at most"),
            None => reading.wrong(format!("\"{said}\" is not a move of the game")),
        }
    }
}

/// `30% Metapod 8, Kakuna 8 : To me!`: under 30 % of its life, they come; it says that.
fn reinforcements(reading: &mut Reading<'_>, value: &str) {
    let (share, rest) = value.split_once(' ').unwrap_or((value, ""));
    let (list, said) = rest.split_once(':').unwrap_or((rest, ""));
    let Some(share) = hundredths(share) else {
        reading.wrong("under how much of its life they come, \"30% Metapod 8, Kakuna 8 : To me!\"");
        return;
    };
    let mut coming = Vec::new();
    for part in listed(list) {
        match foe(part, reading.tables) {
            Ok(enemy) => coming.push(enemy),
            Err(why) => reading.wrong(why),
        }
    }
    if coming.is_empty() || coming.len() > 6 {
        reading.wrong("from 1 to 6 reinforcements");
    }
    reading.boss.says.phases.push(Phase { share, says: said.trim().to_string(), help: coming, ..Phase::default() });
}

/// `Pidgey 5 x2 floors 2-3`: the Pokémon met there, on which floors.
fn pokemon(reading: &mut Reading<'_>, value: &str) {
    for part in listed(value) {
        let (part, on) = on_floors(part);
        match foe(part, reading.tables) {
            Ok(enemy) => reading.foes.push((enemy, on)),
            Err(why) => reading.wrong(why),
        }
    }
}

/// `Oran Berry x3 floors 1-2`: what lies on its floors, how often, on which.
fn items(reading: &mut Reading<'_>, value: &str) {
    for part in listed(value) {
        let (part, on) = on_floors(part);
        let (said, weight) = weighed(part);
        match reading.tables.items.iter().position(|item| plain(item) == plain(said)) {
            Some(item) => reading.items.push((item as u16, weight, on)),
            None => reading.wrong(format!("\"{said}\" is not an item of the game")),
        }
    }
}

/// The options of the rules it is played by.
fn rules(reading: &mut Reading<'_>, value: &str) {
    let (said, unknown) = crate::rule_options::RuleOptions::read(value);
    reading.options = reading.options.or(said);
    for word in unknown {
        let known: Vec<&str> = crate::rule_options::WORDS.iter().map(|(word, _)| *word).collect();
        reading.wrong(format!("\"{word}\"? ({})", known.join(", ")));
    }
}

/// A dungeon read from its text, or what is wrong with it (each with its line).
pub fn read(text: &str, tables: &Tables) -> Result<Own, Vec<Diagnostic>> {
    let mut reading = Reading {
        tables,
        number: 0,
        wrong: Vec::new(),
        name: String::new(),
        floors: 3,
        look: None,
        music: None,
        foes: Vec::new(),
        items: Vec::new(),
        recruit: true,
        goes_up: false,
        team_level: None,
        boss: Boss::default(),
        tileset: None,
        options: crate::rule_options::RuleOptions::default(),
        species_types: Vec::new(),
        move_types: Vec::new(),
        ranks: Vec::new(),
    };
    for (at, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        reading.number = at + 1;
        let Some((key, value)) = line.split_once(':') else {
            reading.wrong(format!("\"key: value\" expected ({KEYS_SAID})"));
            continue;
        };
        let key = plain(key);
        match KEYS.iter().find(|(known, _)| *known == key) {
            Some((_, handler)) => handler(&mut reading, value.trim()),
            None => reading.wrong(format!("\"{key}\"? ({KEYS_SAID})")),
        }
    }
    let coded = |wrong: Vec<Diagnostic>| wrong.into_iter().map(|wrong| wrong.coded(codes::WRONG)).collect::<Vec<_>>();
    reading.checked().map_err(coded)?;
    reading.made().map_err(coded)
}

impl Reading<'_> {
    /// What is wrong with the whole: the Fairy type without its rule, no
    /// name, nobody, a fight without a boss, a floor without anyone.
    fn checked(&mut self) -> Result<(), Vec<Diagnostic>> {
        // (The Fairy type only where the rules have it.)
        let fairy = |(a, b): (u8, u8)| a == crate::dungeon::FAIRY || b == crate::dungeon::FAIRY;
        let fairies = self.species_types.iter().filter(|(_, (_, types))| fairy(*types)).map(|(at, _)| *at);
        let fairies = fairies.chain(self.move_types.iter().filter(|(_, (_, kind))| *kind == crate::dungeon::FAIRY).map(|(at, _)| *at));
        for at in fairies.collect::<Vec<_>>().into_iter().filter(|_| !self.options.fairy) {
            self.wrong.push(Diagnostic::at(at, "the Fairy type needs \"rules: fairy\""));
        }
        if self.name.is_empty() {
            self.wrong.push(Diagnostic::said("it needs a name (\"name: …\")".to_string()));
        }
        if self.foes.is_empty() && self.floors > 0 {
            self.wrong.push(Diagnostic::said("nobody lives there (\"pokemon: Caterpie 3, Weedle 4\")".to_string()));
        }
        if self.floors == 0 && self.boss.boss.is_none() {
            self.wrong.push(Diagnostic::said("0 floors: only with a boss fight (\"boss: Spearow 12\")".to_string()));
        }
        if self.boss.boss.is_none() && !self.boss.guards.is_empty() {
            self.wrong.push(Diagnostic::said("guards without a boss (\"boss: …\")".to_string()));
        }
        let (source, at) = self.look.unwrap_or((1, 0));
        let Some(base) = self.tables.dungeons.get(source as usize) else {
            self.wrong.push(Diagnostic::said("no dungeon of the game to take the look of".to_string()));
            return Err(std::mem::take(&mut self.wrong));
        };
        if shaped_floor(base, at).is_none() {
            self.wrong.push(Diagnostic::said("this dungeon of the game has no floor to take".to_string()));
            return Err(std::mem::take(&mut self.wrong));
        }
        for at in 1..=self.floors {
            if !self.foes.is_empty() && !self.foes.iter().any(|(_, floors)| on(*floors, at)) {
                self.wrong
                    .push(Diagnostic::said(format!("floor {at} has nobody (\"floors a-b\" after a Pokémon: from which floor to which floor one meets it)")));
            }
        }
        if self.wrong.is_empty() { Ok(()) } else { Err(std::mem::take(&mut self.wrong)) }
    }

    /// The dungeon made, once nothing is wrong: its floors, its fight.
    fn made(mut self) -> Result<Own, Vec<Diagnostic>> {
        let tables = self.tables;
        let (source, at) = self.look.unwrap_or((1, 0));
        let base = &tables.dungeons[source as usize];
        let floor = shaped_floor(base, at).ok_or_else(|| vec![Diagnostic::said("this dungeon of the game has no floor to take".to_string())])?;
        let mut rules = base.rules;
        (rules.recruitment, rules.goes_up, rules.level_one, rules.turn_limit) = (self.recruit, self.goes_up, false, rules.turn_limit);
        rules.options = self.options;
        let mut made = Vec::new();
        for at in 1..=self.floors {
            made.push(self.floor_made(floor, at)?);
        }
        let levels: Vec<(i32, Floors)> = self.foes.iter().map(|(enemy, floors)| (enemy.stats.level, *floors)).collect();
        let mut ranks: Vec<u8> = (1..=self.floors).map(|at| rank_on(&self.ranks, &levels, at)).collect();
        let says = self.arena(floor, &mut made)?;
        // (The fight's floor as the last: no job goes there.)
        ranks.resize(made.len(), ranks.last().copied().unwrap_or(1));
        let species_types: Vec<(u16, (u8, u8))> = self.species_types.into_iter().map(|(_, typed)| typed).collect();
        let move_types: Vec<(u16, u8)> = self.move_types.into_iter().map(|(_, typed)| typed).collect();
        let table = DungeonTable { goes_up: self.goes_up, rules, music: self.music.unwrap_or(base.music), floors: made };
        Ok(Own { name: self.name, table, team_level: self.team_level, boss: says, species_types, move_types, ranks })
    }

    /// A floor of it, the game's floor taken: those met there (by thresholds
    /// out of 10,000, each over the one before), what lies there, its music, its tiles.
    fn floor_made(&self, floor: &Floor, at: usize) -> Result<Floor, Vec<Diagnostic>> {
        let Some(mut floor) = copied::<Floor>(floor) else {
            return Err(vec![Diagnostic::said("the game's floor does not copy".to_string())]);
        };
        let mut here: Vec<Enemy> = self.foes.iter().filter(|(_, floors)| on(*floors, at)).filter_map(|(enemy, _)| copied(enemy)).collect();
        let thresholds = cumulated(&here.iter().map(|enemy| enemy.weight).collect::<Vec<_>>());
        for (enemy, threshold) in here.iter_mut().zip(thresholds) {
            enemy.weight = threshold;
        }
        floor.enemies = here;
        if !self.items.is_empty() {
            let lying: Vec<(u16, u16)> = self.items.iter().filter(|(.., floors)| on(*floors, at)).map(|(item, weight, _)| (*item, *weight)).collect();
            floor.items = item_table(&lying, self.tables);
        }
        floor.music = self.music.or(floor.music);
        floor.tileset = self.tileset.unwrap_or(floor.tileset);
        floor.fixed = None;
        Ok(floor)
    }

    /// The fight: one floor more, its room laid out, won once nobody stands; what its boss says.
    fn arena(&mut self, floor: &Floor, made: &mut Vec<Floor>) -> Result<Option<BossSays>, Vec<Diagnostic>> {
        let tables = self.tables;
        let boss = &mut self.boss;
        let (wide, high) = boss.room;
        let inside = |at: &Tile| at.0 < wide && at.1 < high;
        let outside = boss
            .place
            .iter()
            .chain(boss.team.iter())
            .chain(boss.guards.iter().filter_map(|(_, at)| at.as_ref()))
            .chain(boss.lying.iter().map(|(at, _)| at))
            .chain(boss.traps.iter().map(|(at, ..)| at))
            .any(|at| !inside(at));
        if outside {
            return Err(vec![Diagnostic::said(format!("a place outside the arena ({wide}×{high}: from 0,0 to {},{})", wide - 1, high - 1))]);
        }
        let Some(boss_said) = boss.boss.take() else {
            return Ok(None);
        };
        let says = BossSays {
            species: boss_said.species,
            name: tables.species.get(boss_said.species as usize).map_or_else(String::new, |kind| kind.name.clone()),
            ..std::mem::take(&mut boss.says)
        };
        // (Its own look if it says one, else the dungeon's.)
        let own_floor = boss.look.and_then(|(id, at)| shaped_floor(tables.dungeons.get(id as usize)?, at));
        let Some(mut arena) = copied::<Floor>(own_floor.unwrap_or(floor)) else {
            return Err(vec![Diagnostic::said("the game's floor does not copy".to_string())]);
        };
        arena.enemies = Vec::new();
        arena.items = crate::dungeon::ItemTable::default();
        arena.traps = Vec::new();
        arena.music = boss.music.or(self.music).or(arena.music);
        // (One's own tiles there too, unless it has a look of its own.)
        if boss.look.is_none() {
            arena.tileset = self.tileset.unwrap_or(arena.tileset);
        }
        arena.fixed = Some(boss.room_with(boss_said));
        made.push(arena);
        Ok(Some(says))
    }
}

/// Whether a floor is among those said (all if none).
fn on(floors: Floors, at: usize) -> bool {
    floors.is_none_or(|(low, high)| low <= at && at <= high)
}

/// A dungeon's floor made anew each time (not one laid out by hand), the one
/// asked or its last.
fn shaped_floor(dungeon: &DungeonTable, at: usize) -> Option<&Floor> {
    let shaped: Vec<&Floor> = dungeon.floors.iter().filter(|floor| floor.fixed.is_none()).collect();
    shaped.get(at.min(shaped.len().saturating_sub(1))).copied()
}

/// A tile of a room, from its top left corner.
pub type Tile = (usize, usize);

/// Where a boss fight's arena puts each one (tiles of the room, from its top
/// left corner): the team's leader at the bottom of the middle, the boss
/// three tiles above it (in sight as the fight begins), its guards' places at
/// its sides, in the order they are named.
pub fn arena_places(wide: usize, high: usize) -> (Tile, Tile, [Tile; 6]) {
    let middle = wide / 2;
    let leader_y = (high / 2 + 2).min(high.saturating_sub(2));
    let boss_y = leader_y.saturating_sub(3).max(1);
    let behind = boss_y.saturating_sub(1);
    let sides = [
        (middle.saturating_sub(2), boss_y),
        (middle + 2, boss_y),
        (middle.saturating_sub(4), boss_y),
        (middle + 4, boss_y),
        (middle.saturating_sub(1), behind),
        (middle + 1, behind),
    ];
    ((middle, leader_y), (middle, boss_y), sides)
}

/// A tile of a boss fight's arena of that shape and size: 0 ground, 1 wall,
/// 2 water. Where someone is placed is always ground.
pub fn arena_tile(shape: &str, (wide, high): (usize, usize), (x, y): (usize, usize)) -> u8 {
    let (leader, boss, sides) = arena_places(wide, high);
    if (x, y) == leader || (x, y) == boss || sides.contains(&(x, y)) {
        return 0;
    }
    match shape {
        // (Four pillars, between the team and the boss's sides.)
        "pillars" if [wide / 5, wide - 1 - wide / 5].contains(&x) && [high / 3, high - 1 - high / 3].contains(&y) => 1,
        // (Water down both sides, the middle left to fight in.)
        "pool" if (1..high.saturating_sub(1)).contains(&y) && [1, 2, wide.saturating_sub(3), wide.saturating_sub(2)].contains(&x) => 2,
        // (The corners walled off: a cross to fight in.)
        "cross" if (x < wide / 4 || x >= wide - wide / 4) && (y < high / 4 || y >= high - high / 4) => 1,
        _ => 0,
    }
}

/// A boss fight at the end of a dungeon: who, its guards, how much more life
/// it has, its room (how large, what shape), its music.
struct Boss {
    boss: Option<Enemy>,
    /// Its guards, each where said (none: where the arena puts it).
    guards: Vec<(Enemy, Option<Tile>)>,
    /// Those on the team's side, each where said (none: by the team).
    allies: Vec<(Enemy, Option<Tile>)>,
    /// Where it stands, where the team comes in (none: where the arena puts
    /// them); what lies there; its traps (where, what kind, shown).
    place: Option<Tile>,
    team: Option<Tile>,
    lying: Vec<(Tile, u16)>,
    traps: Vec<(Tile, u8, bool)>,
    /// The arena's own look (a game dungeon, a floor of it), if it says one.
    look: Option<(u16, usize)>,
    /// The arena painted tile by tile (rows of 0 ground, 1 wall, 2 water), in place of its shape.
    map: Option<Vec<Vec<u8>>>,
    life: f32,
    room: (usize, usize),
    shape: String,
    music: Option<u16>,
    /// What it holds (and lets fall as it is beaten), what it says.
    drop: Option<u16>,
    says: BossSays,
}

impl Default for Boss {
    fn default() -> Self {
        Boss {
            boss: None,
            look: None,
            map: None,
            guards: Vec::new(),
            allies: Vec::new(),
            place: None,
            team: None,
            lying: Vec::new(),
            traps: Vec::new(),
            life: 1.0,
            room: (13, 9),
            shape: "room".to_string(),
            music: None,
            drop: None,
            says: BossSays::default(),
        }
    }
}

impl Boss {
    /// The room of the fight: the team comes in at the bottom, the boss waits
    /// at the top, its guards at its sides; won once none of them stands.
    fn room_with(&self, mut boss: Enemy) -> crate::tables::Fixed {
        let (wide, high) = self.room;
        let painted = |x: usize, y: usize| self.map.as_ref().and_then(|map| map.get(y)?.get(x).copied());
        let mut tiles: Vec<u8> =
            (0..wide * high).map(|at| painted(at % wide, at / wide).unwrap_or_else(|| arena_tile(&self.shape, (wide, high), (at % wide, at / wide)))).collect();
        boss.stats.max_hp = ((boss.stats.max_hp as f32 * self.life).round() as i32).clamp(1, 9999);
        boss.stats.hp = boss.stats.max_hp;
        if let Some(item) = self.drop {
            boss.stats.held = item;
        }
        let (leader, boss_at, sides) = arena_places(wide, high);
        let (leader, boss_at) = (self.team.unwrap_or(leader), self.place.unwrap_or(boss_at));
        let placed = |at: (usize, usize), enemy: Enemy| crate::tables::Placed { at, enemy, facing: 0 };
        let mut monsters = vec![placed(boss_at, boss)];
        for ((guard, at), side) in self.guards.iter().filter_map(|(guard, at)| Some((copied::<Enemy>(guard)?, *at))).zip(sides) {
            monsters.push(placed(at.unwrap_or(side), guard));
        }
        // (The allies where said, else on the tiles around the team's.)
        let taken: Vec<Tile> = monsters.iter().map(|placed| placed.at).chain(self.allies.iter().filter_map(|(_, at)| *at)).collect();
        let mut around = allies_around(leader, (wide, high), &taken).into_iter();
        let allies: Vec<crate::tables::Placed> =
            self.allies.iter().filter_map(|(ally, at)| Some(placed((*at).or_else(|| around.next())?, copied::<Enemy>(ally)?))).collect();
        // (Where someone or something is, the ground: never a wall nor water.)
        let used: Vec<Tile> = monsters
            .iter()
            .chain(&allies)
            .map(|placed| placed.at)
            .chain([leader])
            .chain(self.lying.iter().map(|(at, _)| *at))
            .chain(self.traps.iter().map(|(at, ..)| *at))
            .collect();
        for (x, y) in used {
            if x < wide && y < high {
                tiles[y * wide + x] = 0;
            }
        }
        crate::tables::Fixed {
            width: wide,
            height: high,
            tiles,
            leader: Some(leader),
            monsters,
            allies,
            watchers: Vec::new(),
            items: self.lying.clone(),
            traps: self.traps.clone(),
            exit: None,
            ends_with_enemies: true,
            moves_allowed: true,
            orbs_allowed: true,
        }
    }
}

/// The tiles around the team's where its allies stand when no tile is said
/// for them, nearest first, none of them taken.
pub fn allies_around(leader: Tile, (wide, high): (usize, usize), taken: &[Tile]) -> Vec<Tile> {
    [(-1, 0), (1, 0), (-1, -1), (1, -1), (0, -1), (-2, 0), (2, 0), (-1, 1), (1, 1), (0, 1)]
        .iter()
        .filter_map(|(dx, dy)| Some(((leader.0 as i32 + dx).try_into().ok()?, (leader.1 as i32 + dy).try_into().ok()?)))
        .filter(|(x, y): &Tile| *x < wide && *y < high && !taken.contains(&(*x, *y)) && (*x, *y) != leader)
        .collect()
}

/// Stats brought to a level, as a monster of that growth gains or loses
/// them level by level.
pub(crate) fn at_level(mut stats: crate::dungeon::Stats, growth: &[crate::dungeon::LevelGain], level: i32) -> crate::dungeon::Stats {
    let growth: std::sync::Arc<[_]> = growth.to_vec().into();
    if level > stats.level {
        let mut grown = Growth::new(growth.clone(), stats.level);
        let wanted = Growth::new(growth, level).points;
        grown.gain(&mut stats, wanted - grown.points);
    } else {
        let mut grown = Growth::new(growth, stats.level);
        while stats.level > level && grown.lose_level(&mut stats) {}
    }
    stats.hp = stats.max_hp;
    stats
}

/// One of a species as the dungeons of the game meet it, nearest that level.
pub(crate) fn met(tables: &Tables, species: u16, level: i32) -> Option<&Enemy> {
    tables
        .dungeons
        .iter()
        .flat_map(|dungeon| &dungeon.floors)
        .flat_map(|floor| &floor.enemies)
        .filter(|enemy| enemy.species == species)
        .min_by_key(|enemy| (enemy.stats.level - level).abs())
}

/// The dungeon added after the game's, the ranks of its floors for the
/// jobs with it: its number.
pub fn add(tables: &mut Tables, name: String, dungeon: DungeonTable, ranks: &[u8]) -> u16 {
    let id = tables.dungeons.len() as u16;
    tables.dungeons.push(dungeon);
    while tables.dungeon_names.len() <= id as usize {
        tables.dungeon_names.push(String::new());
    }
    tables.dungeon_names[id as usize] = name;
    rank_floors(tables, id, ranks);
    id
}

/// The ranks of a dungeon's floors, as the boards read them (`JobTables::floor_ranks`).
pub fn rank_floors(tables: &mut Tables, id: u16, ranks: &[u8]) {
    let listed = &mut tables.jobs.floor_ranks;
    if listed.len() <= id as usize {
        listed.resize(id as usize + 1, Vec::new());
    }
    listed[id as usize] = ranks.to_vec();
}

/// A dungeon of the game by its name (or number), and one of its floors
/// (from 1; the first if not said).
fn looked(said: &str, tables: &Tables) -> Option<(u16, usize)> {
    let (said, floor) = match said.rsplit_once(' ').and_then(|(name, floor)| Some((name, floor.parse::<usize>().ok()?))) {
        Some((name, floor)) => (name, floor.saturating_sub(1)),
        None => (said, 0),
    };
    let id =
        said.parse::<u16>().ok().or_else(|| tables.dungeon_names.iter().position(|name| !name.is_empty() && plain(name) == plain(said)).map(|id| id as u16))?;
    tables.dungeons.get(id as usize).filter(|dungeon| dungeon.floors.iter().any(|floor| floor.fixed.is_none())).map(|_| (id, floor))
}

/// `Oran Berry x3`: the name, and how often (1 if not said).
fn weighed(said: &str) -> (&str, u16) {
    match said.rsplit_once(" x").and_then(|(name, weight)| Some((name.trim(), weight.trim().parse::<u16>().ok()?))) {
        Some((name, weight)) => (name, weight.max(1)),
        None => (said, 1),
    }
}

/// `Caterpie 3 x2`: a Pokémon met at that level, that often. Taken from one
/// of its kind met in the game's dungeons, brought to that level.
fn foe(said: &str, tables: &Tables) -> Result<Enemy, String> {
    let (said, weight) = weighed(said);
    let (species, level) = match said.rsplit_once(' ').and_then(|(name, level)| Some((name.trim(), level.parse::<i32>().ok()?))) {
        Some((name, level)) => (name, level.clamp(1, 100)),
        None => (said, 5),
    };
    let Some(id) = tables.species.iter().position(|kind| plain(&kind.name) == plain(species)) else {
        return Err(format!("\"{species}\" is not a Pokémon of the game"));
    };
    let Some(nearest) = met(tables, id as u16, level) else {
        return Err(format!("\"{species}\" is met in no dungeon of the game: how it fights is not known"));
    };
    let mut enemy: Enemy = copied(nearest).ok_or("does not copy")?;
    enemy.weight = weight;
    enemy.stats = at_level(enemy.stats, &tables.species[id].growth, level);
    Ok(enemy)
}

/// What lies on the floors: each item in its category, as often as said.
fn item_table(items: &[(u16, u16)], tables: &Tables) -> crate::dungeon::ItemTable {
    let category = |item: u16| item_category(item, tables);
    let mut table = crate::dungeon::ItemTable::default();
    let mut categories: Vec<(u8, u16)> = Vec::new();
    for &(item, weight) in items {
        let category = category(item);
        match categories.iter_mut().find(|(of, _)| *of == category) {
            Some((_, total)) => *total += weight,
            None => categories.push((category, weight)),
        }
    }
    for (&(category, _), threshold) in categories.iter().zip(cumulated(&categories.iter().map(|(_, weight)| *weight).collect::<Vec<_>>())) {
        table.categories.push((category, threshold));
        let of: Vec<(u16, u16)> = items.iter().filter(|(item, _)| item_category(*item, tables) == category).copied().collect();
        for (&(item, _), threshold) in of.iter().zip(cumulated(&of.iter().map(|(_, weight)| *weight).collect::<Vec<_>>())) {
            table.items.push((item, category, threshold));
        }
    }
    table
}

/// An item's category as the game's floors give it (0 if none does).
fn item_category(item: u16, tables: &Tables) -> u8 {
    tables
        .dungeons
        .iter()
        .flat_map(|dungeon| &dungeon.floors)
        .flat_map(|floor| &floor.items.items)
        .find(|(id, ..)| *id == item)
        .map_or(0, |(_, category, _)| *category)
}

/// Weights as the game's thresholds: each the sum so far, out of 10,000.
fn cumulated(weights: &[u16]) -> Vec<u16> {
    let total: u32 = weights.iter().map(|weight| u32::from(*weight)).sum::<u32>().max(1);
    let mut sum = 0;
    weights
        .iter()
        .map(|weight| {
            sum += u32::from(*weight);
            (sum * 10_000 / total) as u16
        })
        .collect()
}

/// A copy of what has no `Clone` (as the game's tables are kept).
fn copied<T: serde::Serialize + serde::de::DeserializeOwned>(of: &T) -> Option<T> {
    bincode::deserialize(&bincode::serialize(of).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_sought_plain_and_counts_read() {
        assert_eq!(plain("Spinda's Café"), "spindascafe");
        assert_eq!(plain("Firefly Forest"), "fireflyforest");
        assert_eq!(plain("Nidoran♀"), "nidoranf");
        assert_eq!(weighed("Oran Berry x3"), ("Oran Berry", 3));
        assert_eq!(weighed("Apple"), ("Apple", 1));
        assert_eq!(cumulated(&[1, 1, 2]), vec![2500, 5000, 10_000]);
        assert_eq!(on_floors("Pidgey 6 x2 floors 2-3"), ("Pidgey 6 x2", Some((2, 3))));
        assert_eq!(on_floors("Apple floor 4"), ("Apple", Some((4, 4))));
        assert_eq!(on_floors("Oran Berry x3"), ("Oran Berry x3", None));
        assert_eq!(split_placed("Krabby 7 @5,3, Krabby 7, spikes @9,5 hidden"), vec!["Krabby 7 @5,3", "Krabby 7", "spikes @9,5 hidden"]);
        assert_eq!(placed_at("Krabby 7 @5,3"), ("Krabby 7", Some((5, 3))));
    }

    /// The reader's small words: yes, a list, a song, the floors said; each key known once.
    #[test]
    fn the_reader_knows_its_keys() {
        assert!(yes("Yes") && yes("1") && !yes("no"));
        assert_eq!(listed("Apple, , Oran Berry x3,").collect::<Vec<_>>(), vec!["Apple", "Oran Berry x3"]);
        assert_eq!(song("12"), Some(12));
        assert!(on(None, 7) && on(Some((2, 3)), 3) && !on(Some((2, 3)), 4));
        for (at, (key, _)) in KEYS.iter().enumerate() {
            assert_eq!(plain(key), *key, "a key as it is sought");
            assert!(KEYS[at + 1..].iter().all(|(other, _)| other != key), "{key} twice");
        }
        let said: Vec<String> = KEYS_SAID.split(", ").map(plain).collect();
        assert!(KEYS.iter().all(|(key, _)| said.iter().any(|known| known == key)), "each key told in the faults");
    }

    /// `rank:` read, by floors or for all; a floor it does not name ranked
    /// by the levels met there.
    #[test]
    fn the_jobs_ranks_of_its_floors() {
        assert_eq!(rank_part("E floors 1-2"), Ok((1, Some((1, 2)))));
        assert_eq!(rank_part("D 3"), Ok((2, Some((3, 3)))));
        assert_eq!(rank_part("2*"), Ok((8, None)));
        assert!(rank_part("Z floor 1").is_err() && rank_part("floors 1-2").is_err());
        let said = [rank_part("E 1-2").unwrap(), rank_part("C floor 2").unwrap()];
        let levels = [(5, None), (30, Some((4, 4)))];
        // (Floor 2: the last said; floor 3: level 5, E; floor 4: about 17, D.)
        let ranks: Vec<u8> = (1..=4).map(|at| rank_on(&said, &levels, at)).collect();
        assert_eq!(ranks, [1, 3, 1, 2]);
        assert_eq!(rank_on(&[rank_part("B").unwrap()], &levels, 9), 4);
    }
}
