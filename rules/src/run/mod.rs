//! A game in plain data: the team, its bag and progress, and the dungeon it
//! is in, with no drawing, sound or real time inside. `Run::step` is called
//! once a frame with what the player asks; what is to be shown or heard
//! comes out as `Sight`s. The story and the screens live outside.
//!
//! - `book`: what the game is made of (the tables, names and phrases);
//! - `team`, `log`, `options`, `saves`: what stays from one dungeon to the
//!   next;
//! - `monster`, `dungeon`: the floor being explored;
//! - the turns: `leader`, `others`, `attacking`, `attack`, `strike`, `aftermath`,
//!   `items`, `shops`, `job_floor`, `floors`, `quicksave`;
//! - `entering`: going in a dungeon and coming out of it;
//! - `story`, `town`: what the scenes and the town's services do to the team.

pub mod aftermath;
mod ai_items;
pub mod attack;
pub mod attacking;
pub mod book;
pub mod dungeon;
pub mod entering;
pub mod floors;
pub mod guests;
pub mod items;
pub mod job_floor;
pub mod leader;
pub mod log;
pub mod monster;
pub mod options;
pub mod orbs;
pub mod others;
pub mod quicksave;
pub mod saves;
pub mod secret;
pub mod shops;
pub mod spawning;
pub mod story;
pub mod strike;
pub mod team;
pub mod town;
pub mod treasure;
pub mod vault;
pub mod watch;
pub mod words;

pub use attack::Attack;
pub use book::Book;
pub use dungeon::{Dungeon, Outcome};
pub use entering::Left;
pub use items::ItemAction;
pub use log::Log;
pub use monster::{Monster, MonsterId, Side};
pub use options::Options;
pub use team::{Bag, EpisodeRows, Growing, Member, Progress, Recruit, Team, Whose, tactic};

use std::sync::Arc;

use crate::dungeon::Pos;

/// What the player asks of the leader this frame. The screens turn keys
/// into these; none of them is about a key.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Input {
    /// Nothing: time goes by. (A leader that cannot choose loses its turn.)
    #[default]
    Idle,
    /// A menu is open: time goes by, the leader waits for the player.
    Menu,
    /// The way held: the leader walks there, or faces it if it cannot.
    Go {
        toward: (i32, i32),
    },
    /// The leader faces that way, and walks nowhere.
    Face((i32, i32)),
    /// The regular attack, where the leader faces.
    Attack,
    /// The move in that slot, at once or not at all (a key held).
    UseMove(usize),
    /// The move in that slot, as the leader's next action (from a menu).
    QueueMove(usize),
    /// The leader passes its turn.
    Wait,
    /// The answer to `Dungeon::ask`.
    Answer(bool),
    /// For the first of `Dungeon::learning`: the move to forget (none: the
    /// new one is not learnt).
    Forget(Option<usize>),
    /// Writes a quicksave: it comes out as `Sight::Save`.
    Quicksave,
    /// Goes back to a quicksave's text.
    Quickload(String),
    /// Leaves the dungeon, as "give up" in the menu does.
    GiveUp,
    /// Switches one of a team member's IQ skills off, or on again.
    ToggleIq {
        who: MonsterId,
        skill: u8,
    },
    /// Gives a member of the team a tactic (see `tactic::unlocked`).
    SetTactic {
        who: MonsterId,
        tactic: u8,
    },
    /// Links a monster's move to the one before, or delinks it, with a
    /// Link Box in the bag.
    Link {
        who: MonsterId,
        slot: usize,
    },
    /// The item at that place of the bag is used, or thrown, or put down
    /// where the leader stands.
    UseItem(usize),
    ThrowItem(usize),
    PutDown(usize),
    /// That one of the team holds the item at that place of the bag.
    Give {
        item: usize,
        to: MonsterId,
    },
    /// What is said to Kecleon: whether to pay; the item of the bag sold;
    /// goodbye.
    Pay(bool),
    Sell(usize),
    LeaveShop,
    /// The player moves the next member of the team, the one moved till
    /// then following on its own (`RuleOptions::team_control`).
    SwitchControl,
    /// That member of the team leads it from now on (the dungeon menu's
    /// "Leader").
    MakeLeader(MonsterId),
    /// A team member's move switched off for the AI, or on again (the
    /// menu's "Switch").
    SwitchMove {
        who: MonsterId,
        slot: usize,
    },
}

/// What just happened that is to be shown or heard. Anything that stays
/// (where monsters stand, their HP) is read from `Dungeon` directly.
#[derive(Clone, Debug, PartialEq)]
pub enum Sight {
    /// A move being used: by whom, from where, the tiles it reaches.
    Move {
        move_id: u16,
        user: MonsterId,
        from: Pos,
        targets: Vec<Pos>,
    },
    /// A trap sprung on that tile.
    Trap(usize, Pos),
    /// An item used by who stands on that tile.
    Item(u16, Pos),
    /// An item thrown from a tile to another.
    Thrown(u16, Pos, Pos),
    /// An effect of that number on who stands on that tile.
    Effect(u16, Pos),
    /// A number over a monster: HP lost (above 0) or gained (below).
    Number(MonsterId, i32),
    /// A blow that missed that monster.
    Miss(MonsterId),
    /// That monster takes a blow, after so many frames.
    Hurt(MonsterId, u32),
    /// That monster strikes.
    Strike(MonsterId),
    /// That monster is defeated: shown a moment, blinking out.
    Fainted(MonsterId),
    /// A sound of the game; a jingle.
    Sound(u16),
    Jingle(u16),
    /// The song changes.
    Music(Option<u16>),
    /// A new floor: everything of the old one goes.
    NewFloor,
    /// Tiles of the floor changed: its picture is drawn again.
    TilesChanged,
    /// What was asked of the bag was done: its menu closes.
    BagDone,
    /// A quicksave, to be kept: the disk is the game's.
    Save(String),
}

/// A game being played.
pub struct Run {
    pub book: Arc<Book>,
    team: Team,
    bag: Bag,
    /// The money left at the bank.
    bank: i32,
    options: Options,
    pub log: Log,
    /// The dungeon being explored, if any.
    dungeon: Option<Dungeon>,
    /// Frames since the game started: the only clock of the rules.
    frame: u64,
    /// Nothing waits to be seen (when no one is looking, in checks).
    pub(crate) unseen: bool,
    /// What is yet to be given to the caller of `step`.
    pub(crate) sights: Vec<Sight>,
}

impl Run {
    pub fn new(book: Arc<Book>, options: Options) -> Self {
        let team = Team::first(&book);
        Self { book, team, bag: Bag::default(), bank: 0, options, log: Log::default(), dungeon: None, frame: 0, unseen: false, sights: Vec::new() }
    }

    pub fn team(&self) -> &Team {
        &self.team
    }

    pub fn bag(&self) -> &Bag {
        &self.bag
    }

    /// The game's tables it plays by.
    pub fn book(&self) -> &Book {
        &self.book
    }

    /// The money left at the bank.
    pub fn bank(&self) -> i32 {
        self.bank
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    /// The dungeon being explored, if any.
    pub fn dungeon(&self) -> Option<&Dungeon> {
        self.dungeon.as_ref()
    }

    /// Frames since the game started.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Whether no one is looking.
    pub fn unseen(&self) -> bool {
        self.unseen
    }

    /// No one is looking: nothing waits to be seen.
    pub fn look_away(&mut self) {
        self.unseen = true;
    }

    /// A line in the log.
    pub fn say(&mut self, text: String) {
        self.log.push(text);
    }

    /// A row of the options, a step up or down.
    pub fn change_option(&mut self, row: usize, more: bool) {
        self.options.change(row, more);
    }

    /// What shows lasts so many frames: if the monsters are waiting, they
    /// wait for it.
    pub fn wait_for_show(&mut self, frames: u32) {
        if let Some(dungeon) = &mut self.dungeon
            && dungeon.wait > 0
        {
            dungeon.wait = dungeon.wait.max(frames);
        }
    }

    /// The dungeons' map shown or hidden, and kept; whether it now shows.
    pub fn toggle_map(&mut self) -> bool {
        self.options.map = !self.options.map;
        self.options.save();
        self.options.map
    }

    pub(crate) fn show(&mut self, sight: Sight) {
        self.sights.push(sight);
    }

    /// One frame of the rules. The order of a frame is written here, and
    /// nowhere else. Once `Dungeon::outcome` is set, the rules are over:
    /// `leave_dungeon` is for the screen to call.
    pub fn step(&mut self, input: Input) -> Vec<Sight> {
        self.frame += 1;
        if self.dungeon.as_ref().is_some_and(|dungeon| dungeon.outcome.is_none()) {
            self.dungeon_frame(input);
        }
        self.after_actions();
        std::mem::take(&mut self.sights)
    }

    fn dungeon_frame(&mut self, input: Input) {
        self.count_down();
        self.forget(&input);
        if let Input::Answer(yes) = input {
            self.answer(yes);
        }
        self.menu_input(&input);
        if self.take_item_input(&input) {
            self.finish_turn();
        }
        self.transfer_foes();
        self.leader_turn(&input);
        self.shop_step();
        self.step_on_tile();
        self.wake_monster_house();
        self.others_turn();
        self.respawn_enemies();
        self.job_targets();
        self.resolve_deaths();
        self.evolve_foes();
        self.descend_stairs();
        self.take_hidden_stairs();
        self.lay_traps();
        self.keep_iq();
        self.see_floor();
        self.fade_fainted();
    }

    /// The foes of that species standing grow fiercer: their attack and
    /// special attack raised so many stages. Whether any did.
    pub fn rouse(&mut self, species: u16, stages: i32) -> bool {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return false;
        };
        let mut roused = false;
        for monster in dungeon.monsters.iter_mut().filter(|monster| monster.side == Side::Foe && monster.alive() && monster.species == Some(species)) {
            let status = &mut monster.status;
            status.attack_stage = (status.attack_stage + stages).min(20);
            status.sp_attack_stage = (status.sp_attack_stage + stages).min(20);
            roused = true;
        }
        roused
    }

    /// The foes of that species standing know these moves (the game's
    /// numbers, four at most) instead of their own.
    pub fn teach_foe(&mut self, species: u16, moves: &[u16]) {
        let slots: Vec<crate::dungeon::MoveSlot> = moves.iter().filter_map(|id| self.book.move_slot(*id)).take(4).collect();
        let Some(dungeon) = self.dungeon.as_mut().filter(|_| !slots.is_empty()) else {
            return;
        };
        for monster in dungeon.monsters.iter_mut().filter(|monster| monster.side == Side::Foe && monster.alive() && monster.species == Some(species)) {
            monster.moves = slots.clone();
        }
    }

    /// The foes of that species standing get back so many hundredths of their
    /// life. How much the first got back.
    pub fn heal_foe(&mut self, species: u16, hundredths: i32) -> i32 {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return 0;
        };
        let mut healed = 0;
        for monster in dungeon.monsters.iter_mut().filter(|monster| monster.side == Side::Foe && monster.alive() && monster.species == Some(species)) {
            let gained = (monster.stats.max_hp * hundredths / 100).min(monster.stats.max_hp - monster.stats.hp).max(0);
            monster.stats.hp += gained;
            if healed == 0 {
                healed = gained;
            }
        }
        healed
    }

    /// The weather of the floor becomes that one, for as long as a move would make it.
    pub fn set_weather(&mut self, kind: u8) {
        let turns = self.book.tables.constants.weather_move_turns;
        if let Some(dungeon) = self.dungeon.as_mut() {
            dungeon.weather.set(kind, turns);
        }
    }

    /// The foes of that species standing ask to join the team once beaten
    /// (if the dungeon lets one recruit).
    pub fn boss_may_join(&mut self, species: u16) {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        for monster in dungeon.monsters.iter_mut().filter(|monster| monster.side == Side::Foe && monster.species == Some(species)) {
            monster.recruitable = Some(monster::Recruitable { species, rate: 1000 });
        }
    }

    /// The foes of that species standing are left with so many hundredths of
    /// their life (a fight tried from its middle).
    pub fn set_foe_life(&mut self, species: u16, hundredths: i32) {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        for monster in dungeon.monsters.iter_mut().filter(|monster| monster.side == Side::Foe && monster.alive() && monster.species == Some(species)) {
            monster.stats.hp = (monster.stats.max_hp * hundredths / 100).clamp(1, monster.stats.max_hp);
        }
    }

    /// Those come to help the foe of that species: each on a free tile of the
    /// floor nearest it. How many came.
    pub fn summon_near(&mut self, species: u16, coming: &[crate::tables::Enemy]) -> usize {
        let book = self.book.clone();
        let Some(dungeon) = self.dungeon.as_mut() else {
            return 0;
        };
        let Some(centre) =
            dungeon.monsters.iter().find(|monster| monster.side == Side::Foe && monster.alive() && monster.species == Some(species)).map(|monster| monster.at)
        else {
            return 0;
        };
        let mut came = 0;
        for enemy in coming {
            let free = |at: Pos, dungeon: &dungeon::Dungeon| {
                matches!(dungeon.floor.tile(at), crate::dungeon::Tile::Floor { .. })
                    && !dungeon.monsters.iter().any(|monster| monster.alive() && monster.at == at)
            };
            let spot = (1..6)
                .flat_map(|ring| (-ring..=ring).flat_map(move |dx| (-ring..=ring).map(move |dy| Pos::new(centre.x + dx, centre.y + dy))))
                .find(|at| free(*at, dungeon));
            let Some(at) = spot else {
                break;
            };
            let movement = book.species(enemy.species).map_or(0, |kind| kind.movement);
            let mut monster = spawning::wild_monster(dungeon, &book, enemy, at, crate::dungeon::status::Statuses::for_species(movement));
            monster.recruitable = None;
            dungeon.monsters.push(monster);
            came += 1;
        }
        came
    }

    /// The dungeon being played. (Only for what a dungeon's frame calls.)
    pub(crate) fn here(&mut self) -> &mut Dungeon {
        self.dungeon.as_mut().expect("a dungeon")
    }
}
