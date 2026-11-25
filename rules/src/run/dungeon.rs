//! The dungeon being explored: the floor, those on it, what lies there,
//! and what the turn in progress still has to do.

use crate::dungeon::minimap::Explored;
use crate::dungeon::weather::{self, Weather};
use crate::dungeon::{DungeonRng, Floor, MoveEffect, MoveSlot, Pos, generate_floor, held};
use crate::tables::Rules;
use crate::tables::{DungeonTable, Floor as FloorTable};

use super::monster::{Monster, MonsterId, Side};
use super::team::Recruit;

/// How a dungeon run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Cleared,
    Fainted,
    /// Left through the pause menu.
    Abandoned,
    /// Left by an Escape Orb: as abandoned, the bag kept.
    Escaped,
    /// Blown out by the wind after too long on a floor: as fainted.
    BlownOut,
}

/// An item lying on the floor; a Kecleon shop's has a price.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorItem {
    pub at: Pos,
    pub item: u16,
    pub price: Option<i32>,
}

/// A trap, hidden until stepped on; one the team laid (Trapper Orb) is
/// sprung by the foes only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorTrap {
    pub at: Pos,
    pub kind: usize,
    pub seen: bool,
    pub team: bool,
}

/// Where a job of the floor is done, with no one there: a floor to reach,
/// an item to find (shown as lying there).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JobSpot {
    pub job: usize,
    pub at: Pos,
    pub item: Option<u16>,
}

/// A Kecleon shop on a floor: where Kecleon stands (none once it hunts a
/// thief), what the team picked up there and has not paid for, and what is
/// being said with Kecleon.
#[derive(Default, Clone)]
pub struct Shop {
    pub keeper: Option<Pos>,
    pub owed: Vec<(u16, i32)>,
    pub talk: Option<ShopTalk>,
    /// The leader has been greeted; it is in the shop (its song plays).
    pub welcomed: bool,
    pub inside: bool,
}

/// What Kecleon asks: whether to pay for what was picked up, or what to
/// sell it (the cursor over the bag is the screen's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShopTalk {
    Pay,
    Sell,
}

/// A yes-or-no question of a dungeon, waiting for its answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    /// Whether to go on by the stairs the leader stepped on.
    Stairs,
    /// Whether to let a defeated monster join the team.
    Recruit(Recruit),
    /// A job done: whether to leave the dungeon at once.
    Leave,
}

/// A move on the user's side reaching the teammates around it.
#[derive(Clone, Copy, Debug)]
pub struct Spread {
    pub from: Pos,
    pub team: bool,
    pub effect: MoveEffect,
    pub range: u8,
}

/// A move that moves monsters: who used it, where, facing which way, and
/// on which side.
#[derive(Clone, Copy, Debug)]
pub struct Shove {
    pub effect: MoveEffect,
    pub from: Pos,
    pub facing: (i32, i32),
    pub team: bool,
}

/// Where the others are in their turn, when it is stopped for an attack to
/// be seen.
#[derive(Default, Clone)]
pub struct Phase {
    /// The rounds still to play (two for a slowed leader).
    pub rounds: u32,
    /// Where the leader stood before its last action.
    pub previous: Option<Pos>,
    /// The round is under way.
    pub begun: bool,
    /// Those of the round, the next to act, and which of its actions.
    pub order: Vec<MonsterId>,
    pub next: usize,
    pub act: i32,
}

/// The frames of the black screen naming a floor, the last of them fading.
pub(crate) const FLOOR_BANNER: u32 = 100;
pub const BANNER_FADE: u32 = 20;
/// The frames an attack is given to be seen, before anyone else acts (by
/// eye: the original's attacks take about a third of a second).
pub(crate) const ATTACK_WAIT: u32 = 20;
/// The frames between two steps of the leader (a step slides over them).
pub const STEP_FRAMES: u32 = 7;
/// The frames a monster defeated stays to blink out: 36 of them blinking,
/// after 12 taking its blow.
pub(crate) const FAINT_FRAMES: u32 = 48;

pub struct Dungeon {
    /// Its number in the tables (none: a trial without floors).
    pub id: Option<u16>,
    pub rng: DungeonRng,
    /// RNG state the current floor was generated from (for save files).
    pub floor_seed: u32,
    pub floor: Floor,
    pub floor_number: u32,
    pub turn: u32,
    /// Bumped whenever `floor` is replaced.
    pub generation: u32,
    /// Bumped when the floor's tiles change (an orb's doing).
    pub repaints: u32,
    /// Everyone on the floor, the leader first, then its team, then the
    /// others in the order they came: the order they act in.
    pub monsters: Vec<Monster>,
    next_id: u32,
    pub items: Vec<FloorItem>,
    pub traps: Vec<FloorTrap>,
    /// The leader has acted; the others take their turn next.
    pub enemy_phase: bool,
    /// The others' turn, stopped for an attack to be seen.
    pub phase: Option<Phase>,
    /// The frames before anyone acts again.
    pub wait: u32,
    pub shop: Option<Shop>,
    pub spots: Vec<JobSpot>,
    /// Who last told the leader what it waits for, and from where.
    pub told: Option<(MonsterId, Pos)>,
    /// A yes-or-no question waiting for its answer; the stairs, once the
    /// leader agreed to take them.
    pub ask: Option<Ask>,
    pub take_stairs: bool,
    /// Moves a member wants to learn but has no room for: who, which.
    pub learning: Vec<(MonsterId, MoveSlot)>,
    pub name: String,
    /// The frames left of the black screen naming the floor.
    pub banner: u32,
    /// The dungeon's song, played again on leaving a shop.
    pub song: Option<u16>,
    /// A Luminous Orb lit the floor: no dark; a Radar Orb shows its
    /// monsters on the map; a Silence Orb seals its foes' moves.
    pub lit: bool,
    pub radar: bool,
    pub silence: bool,
    /// A theft on this floor: Kecleons keep coming.
    pub thief_alert: bool,
    /// A floor laid out by hand may refuse orbs.
    pub no_orbs: bool,
    /// Where a foe is to turn into another monster (Transfer Orb).
    pub transfer: Option<Pos>,
    /// Turns left of Mud Sport and Water Sport.
    pub mud_sport: i32,
    pub water_sport: i32,
    /// The last move anyone used on the floor (what Copycat copies).
    pub last_used: Option<MoveSlot>,
    /// The fight of a floor laid out for one is won: on to what follows.
    pub fight_won: bool,
    /// Moves on the user's side that reach its teammates too, still to do
    /// so.
    pub spread: Vec<Spread>,
    /// Blasts of moves to go off: centre, how far around (0: the one tile,
    /// a wall Rock Smash breaks), and who set it off, spared.
    pub blasts: Vec<(Pos, i32, Option<Pos>)>,
    pub shoves: Vec<Shove>,
    /// The floor (its `generation`) Gravity was used on.
    pub gravity: Option<u32>,
    /// Items a move drops on the floor (Pay Day's money), where.
    pub drops: Vec<(Pos, u16)>,
    /// The belly the leader's chains of linked moves cost this turn.
    pub chain_belly: i32,
    /// Recycle was used by the team.
    pub recycle: bool,
    /// A Dough Seed was eaten: more Poké lie on the next floor.
    pub dough_seed: bool,
    /// Effects a Pass Scarf sent on, from where its holder stands.
    pub passed: Vec<(Pos, MoveEffect)>,
    /// Where the monster now attacking faces (for Me First).
    pub aimed_at: Option<Pos>,
    /// The HP of those bound by Destiny Bond, a frame ago.
    pub bond_hp: Vec<(MonsterId, i32)>,
    /// The floor (its `generation`) `Run::after_actions` last saw.
    pub identity_floor: u32,
    /// The next Destiny Bond's token.
    pub bonds: u32,
    pub weather: Weather,
    /// Actions the leader has taken since the others last moved, and how
    /// many rounds the others have had (for monsters at half speed).
    pub actions_this_round: i32,
    pub round: u32,
    /// The dungeon's floors are counted up rather than down.
    pub goes_up: bool,
    pub rules: Rules,
    /// Turns spent on this floor (the wind blows the team out at the
    /// dungeon's limit).
    pub floor_turns: u32,
    /// Set when the run is over.
    pub outcome: Option<Outcome>,
    /// The team is down: the frames before the run ends.
    pub knocked_out: Option<u32>,
    /// What the dungeon's rules kept out (items beyond its limit, money
    /// where it is not allowed): given back on leaving.
    pub kept_out: (Vec<u16>, i32),
    /// Where the leader last stood (who follows walks in its footsteps).
    pub trail: Option<Pos>,
    /// The frames before the leader can act again.
    pub cooldown: u32,
    /// A move picked in a menu: the leader's next action.
    pub queued: Option<usize>,
    /// Where the leader was when the tile under it last did its work, and
    /// when the stairs were last looked at (the floor, the tile).
    pub stood: Option<Pos>,
    pub stairs_seen: Option<(u32, Pos)>,
    /// The turn monsters last had a chance to appear, and the turns since.
    pub spawn_turn: u32,
    pub spawn_count: i32,
    /// What the team has seen of the floor.
    pub explored: Explored,
    /// Jobs may be done here: the team chose to come (a dungeon of the
    /// story has none).
    pub jobs: bool,
    /// The hero, whoever the player moves (`RuleOptions::team_control`).
    pub hero: Option<MonsterId>,
    /// The Exp. the members waiting at the base get on leaving
    /// (`RuleOptions::exp_share`).
    pub shared_exp: i32,
}

impl Dungeon {
    /// A dungeon before its first floor is entered.
    pub fn new(id: Option<u16>, table: Option<&DungeonTable>, name: String, song: Option<u16>, seed: u32) -> Self {
        Self {
            id,
            rng: DungeonRng::new(seed),
            floor_seed: seed,
            floor: Floor::new_walled(),
            floor_number: 1,
            turn: 0,
            generation: 0,
            repaints: 0,
            monsters: Vec::new(),
            next_id: 0,
            items: Vec::new(),
            traps: Vec::new(),
            enemy_phase: false,
            phase: None,
            wait: 0,
            shop: None,
            spots: Vec::new(),
            told: None,
            ask: None,
            take_stairs: false,
            learning: Vec::new(),
            name,
            banner: 0,
            song,
            lit: false,
            radar: false,
            thief_alert: false,
            no_orbs: false,
            silence: false,
            transfer: None,
            mud_sport: 0,
            water_sport: 0,
            last_used: None,
            fight_won: false,
            spread: Vec::new(),
            blasts: Vec::new(),
            shoves: Vec::new(),
            gravity: None,
            drops: Vec::new(),
            chain_belly: 0,
            recycle: false,
            dough_seed: false,
            passed: Vec::new(),
            aimed_at: None,
            bond_hp: Vec::new(),
            identity_floor: 0,
            bonds: 0,
            weather: Weather::default(),
            actions_this_round: 0,
            round: 0,
            goes_up: table.is_some_and(|table| table.goes_up),
            rules: table.map(|table| table.rules).unwrap_or_default(),
            floor_turns: 0,
            outcome: None,
            knocked_out: None,
            kept_out: (Vec::new(), 0),
            trail: None,
            cooldown: 0,
            queued: None,
            stood: None,
            stairs_seen: None,
            spawn_turn: 0,
            spawn_count: 0,
            explored: Explored::default(),
            jobs: false,
            hero: None,
            shared_exp: 0,
        }
    }

    /// A name for one more monster.
    pub(crate) fn new_id(&mut self) -> MonsterId {
        self.next_id += 1;
        MonsterId(self.next_id)
    }

    pub fn monster(&self, id: MonsterId) -> Option<&Monster> {
        self.monsters.iter().find(|monster| monster.id == id)
    }

    pub(crate) fn monster_mut(&mut self, id: MonsterId) -> Option<&mut Monster> {
        self.monsters.iter_mut().find(|monster| monster.id == id)
    }

    /// The current floor in the tables, read from the book.
    pub fn spec<'b>(&self, book: &'b super::Book) -> Option<&'b FloorTable> {
        book.tables.dungeons.get(self.id? as usize)?.floors.get(self.floor_number.checked_sub(1)? as usize)
    }

    /// The leader. (There always is one while a dungeon is played.)
    pub fn leader(&self) -> &Monster {
        self.monsters.iter().find(|monster| monster.side == Side::Leader).expect("a leader")
    }

    pub(crate) fn leader_mut(&mut self) -> &mut Monster {
        self.monsters.iter_mut().find(|monster| monster.side == Side::Leader).expect("a leader")
    }

    /// Who stands alive on a tile.
    pub fn at(&self, tile: Pos) -> Option<&Monster> {
        self.monsters.iter().find(|monster| monster.at == tile && monster.alive())
    }

    /// Generates the floor of `spec` from the current RNG state: those on
    /// it and what lies there are the caller's to place.
    pub(crate) fn enter_floor(&mut self, spec: Option<&FloorTable>) {
        self.floor_seed = self.rng.state();
        self.floor_turns = 0;
        self.last_used = None;
        let mut props = spec.map(|floor| floor.props).unwrap_or_default();
        props.disarmed_lone_house = self.rules.options.disarmed_houses;
        self.floor = match spec.and_then(|floor| floor.fixed.as_ref()) {
            Some(fixed) => fixed.floor(),
            None => generate_floor(&mut self.rng, &props),
        };
        // (A dark floor shows its monsters as little as the team.)
        const HALLWAY_SIGHT: i32 = 2;
        self.floor.hallway_sight = spec.map_or(0, |floor| floor.visibility).clamp(0, HALLWAY_SIGHT);
        if self.floor.hallway_sight == 0 {
            self.floor.hallway_sight = HALLWAY_SIGHT;
        }
        let natural = spec.map_or(weather::CLEAR, |floor| floor.weather);
        self.weather = Weather::new(natural, &mut self.rng);
        self.weather.lasting = self.rules.options.lasting_weather;
        let fixed = spec.and_then(|floor| floor.fixed.as_ref());
        (self.lit, self.radar, self.thief_alert) = (false, false, false);
        (self.silence, self.no_orbs) = fixed.map_or((false, false), |fixed| (!fixed.moves_allowed, !fixed.orbs_allowed));
        self.banner = FLOOR_BANNER;
        self.generation += 1;
    }

    /// What the leader's specs show on the map: every item and foe (X-Ray
    /// Specs: `Some(true)`), or none at all (Y-Ray Specs: `Some(false)`).
    pub fn specs_map(&self) -> Option<bool> {
        let leader = &self.leader().stats;
        match (held::holds(leader, held::X_RAY_SPECS), held::holds(leader, held::Y_RAY_SPECS)) {
            (true, _) => Some(true),
            (false, true) => Some(false),
            _ => None,
        }
    }

    /// A random free room tile, for a monster that warps.
    pub(crate) fn warp_tile(&mut self, taken: &[Pos]) -> Option<Pos> {
        let tiles: Vec<Pos> = Floor::positions().filter(|&tile| self.floor.is_spawn_tile(tile) && !taken.contains(&tile)).collect();
        tiles.get(self.rng.rand_int(tiles.len() as i32) as usize).copied()
    }
}

pub(crate) fn weather_name(kind: u8) -> &'static str {
    match kind {
        weather::SUNNY => "sunny",
        weather::SANDSTORM => "sandstorm",
        weather::CLOUDY => "cloudy",
        weather::RAIN => "rain",
        weather::HAIL => "hail",
        weather::FOG => "fog",
        weather::SNOW => "snow",
        _ => "clear",
    }
}

/// The eight directions, for confused monsters.
pub const DIRECTIONS: [(i32, i32); 8] = [(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)];
