//! Attacks: from a move chosen to what it does on its user's side. The
//! blows themselves are in `strike`, what follows in `aftermath`. Who an
//! attack reaches is the turn's: it calls `Run::begin_attack`, then
//! `Run::team_move` or `Run::strike` on each one reached.

use crate::dungeon::combat::{TARGET_PARTY, TARGET_TEAMMATES};
use crate::dungeon::{
    FIXED_ONE, FLOOR_HEIGHT, FLOOR_WIDTH, MoveEffect, MoveSlot, Pos, Tile, ability, apply_effect, charged_multiplier, iq, range, stack, status, weather,
};

use super::dungeon::{Shove, Spread};
pub use crate::attack::Attack;
pub(crate) use crate::dungeon::floor::chebyshev;

use super::team::Bag;
use super::watch::{Watch, watched};
use super::words::effect_message;
use super::{Book, Dungeon, Log, Monster, MonsterId, Run, Sight};

/// What the rules of an attack work on, borrowed from `Run`.
pub(crate) struct Arena<'a> {
    pub(crate) dungeon: &'a mut Dungeon,
    pub(crate) book: &'a Book,
    pub(crate) log: &'a mut Log,
    pub(crate) sights: &'a mut Vec<Sight>,
    pub(crate) bag: &'a mut Bag,
    pub(crate) unseen: bool,
}

/// The two monsters of the list, each its own.
fn pair(monsters: &mut [Monster], one: MonsterId, other: MonsterId) -> Option<(&mut Monster, &mut Monster)> {
    let first = monsters.iter().position(|monster| monster.id == one)?;
    let second = monsters.iter().position(|monster| monster.id == other)?;
    if first == second {
        return None;
    }
    let (left, right) = monsters.split_at_mut(first.max(second));
    let (low, high) = (&mut left[first.min(second)], &mut right[0]);
    Some(if first < second { (low, high) } else { (high, low) })
}

const FAILED: &str = "But it failed!";
const NOTHING: &str = "But nothing happened.";
/// Bide, Revenge, Avalanche.
const BIDES: [u16; 3] = [61, 340, 535];
const LAST_RESORT: u16 = 497;
const SNORE: u16 = 25;
const SLEEP_TALK: u16 = 227;

/// What an attack's user knows, as it was before the move spent its PP.
struct Known {
    /// The types of its moves, for Conversion.
    kinds: Vec<u8>,
    /// Its other moves out of PP, for Last Resort.
    spent: usize,
    /// The moves Sleep Talk may call.
    talked: Vec<MoveSlot>,
}

impl Known {
    fn of(user: &Monster) -> Self {
        Known {
            kinds: user.moves.iter().map(|slot| slot.type_id).collect(),
            spent: user.moves.iter().filter(|slot| slot.id != LAST_RESORT && slot.pp <= 0).count(),
            talked: user.moves.iter().copied().filter(|slot| !matches!(slot.id, SLEEP_TALK | 61 | 340) && !status::CHARGED_MOVES.contains(&slot.id)).collect(),
        }
    }
}

/// A move being used, as it becomes another.
struct Using {
    /// The move chosen.
    id: u16,
    /// Its slot, PP spent.
    slot: MoveSlot,
    /// What it became.
    used: MoveSlot,
    damage_mult: i32,
}

impl Run {
    pub(crate) fn arena(&mut self) -> Option<Arena<'_>> {
        Some(Arena { dungeon: self.dungeon.as_mut()?, book: &self.book, log: &mut self.log, sights: &mut self.sights, bag: &mut self.bag, unseen: self.unseen })
    }

    /// Starts the attack a monster chose (`None`: the regular attack): PP
    /// are spent and the move is announced. Returns nothing when the turn
    /// goes into charging the move, or it fails. `unleashed`: the move
    /// charged last turn; `spent_struggles`: out of PP, it struggles (the
    /// leader's own struggle is its turn's).
    pub(crate) fn begin_attack(&mut self, user: MonsterId, chosen: Option<usize>, unleashed: bool, spent_struggles: bool) -> Option<Attack> {
        self.arena()?.begin_attack(user, chosen, unleashed, spent_struggles)
    }

    /// Lands an attack on one of the monsters it reaches. Whether it did
    /// not miss.
    pub(crate) fn strike(&mut self, user: MonsterId, victim: MonsterId, attack: &Attack) -> bool {
        self.arena().is_some_and(|mut arena| arena.strike(user, victim, attack))
    }

    /// A status move its user casts on itself.
    pub(crate) fn self_effect(&mut self, user: MonsterId, effect: MoveEffect) {
        if let Some(mut arena) = self.arena() {
            arena.alone(user, |arena, user| arena.cast(user, effect));
        }
    }

    /// A status move on the user's side, as `MoveEffect::on_user` says: on
    /// the user alone, on it and its teammates or on its teammates alone;
    /// or what changes the floor (a blast, a wall broken, a shove). From
    /// where it stands, the way it faces. (A Warp is the turn's to do.)
    pub(crate) fn team_move(&mut self, user: MonsterId, effect: MoveEffect, slot: Option<MoveSlot>) {
        if let Some(mut arena) = self.arena() {
            arena.alone(user, |arena, user| arena.team_move(user, effect, slot));
        }
    }
}

impl Arena<'_> {
    /// Works on a monster and what is not the list of monsters, what it
    /// does to it being shown.
    fn alone(&mut self, user: MonsterId, work: impl FnOnce(&mut Self, &mut Monster)) {
        let mut monsters = std::mem::take(&mut self.dungeon.monsters);
        if let Some(user) = monsters.iter_mut().find(|monster| monster.id == user) {
            let watch = Watch::of(user);
            work(self, user);
            watch.show(user, self.sights);
        }
        self.dungeon.monsters = monsters;
    }

    fn begin_attack(&mut self, user: MonsterId, chosen: Option<usize>, unleashed: bool, spent_struggles: bool) -> Option<Attack> {
        let at = self.dungeon.monsters.iter().position(|monster| monster.id == user)?;
        let mut monsters = std::mem::take(&mut self.dungeon.monsters);
        let (before, rest) = monsters.split_at_mut(at);
        let (user, after) = rest.split_first_mut()?;
        let watch = Watch::of(user);
        let attack = self.begin(before, user, after, chosen, unleashed, spent_struggles);
        watch.show(user, self.sights);
        self.dungeon.monsters = monsters;
        attack
    }

    fn begin(
        &mut self,
        before: &[Monster],
        user: &mut Monster,
        after: &[Monster],
        chosen: Option<usize>,
        unleashed: bool,
        spent_struggles: bool,
    ) -> Option<Attack> {
        // (What a Bide sent back went with its last attack.)
        user.status.bide_release = None;
        // Under Encore, its last move again, whatever was chosen.
        let encored = (user.status.encore_turns > 0)
            .then(|| user.status.last_move.and_then(|id| user.moves.iter().position(|slot| slot.id == id && slot.pp > 0)))
            .flatten();
        let chosen = encored.or(chosen);
        let known = Known::of(user);
        let Some(index) = chosen.filter(|&index| index < user.moves.len()) else {
            return Some(self.regular_attack(user, spent_struggles));
        };
        let id = user.moves[index].id;
        if self.refused(user, index) {
            return None;
        }
        if BIDES.contains(&id) {
            return self.bide(user, index, unleashed);
        }
        let damage_mult = self.charged(user, index, unleashed)?;
        let name = user.name.clone();
        let move_name = self.book.move_name(id);
        self.log.push(
            self.book
                .phrase(crate::strings::log_text::USED_MOVE, &[("string:0", &name), ("move:0", &move_name)])
                .unwrap_or_else(|| format!("{name} used {move_name}!")),
        );
        user.status.last_move = Some(id);
        user.status.pressured = false;
        let slot = user.moves[index];
        let mut using = Using { id, slot, used: slot, damage_mult };
        if let Err(said) = self.turn_into(before, user, after, &known, &mut using) {
            self.log.push(said.to_string());
            return None;
        }
        // Truant: the lazy monster loafs around for a turn after each move
        // (a count of 2: the end of this turn takes one off).
        const LOAFING: i32 = 2;
        if ability::has(&user.stats, ability::TRUANT) {
            user.status.paused_turns = user.status.paused_turns.max(LOAFING);
            self.log.push(format!("{} will loaf around next turn.", user.name));
        }
        let Using { used, damage_mult, .. } = using;
        Some(Attack {
            slot: used,
            effect: used.effect_in(&self.book.tables.constants, self.dungeon.weather.apparent(), &user.stats),
            damage_mult,
            linked: false,
        })
    }

    /// No move chosen: the regular attack. Out of PP, a monster that picks
    /// its own moves struggles, which hurts it (as the leader's Struggle).
    fn regular_attack(&mut self, user: &mut Monster, spent_struggles: bool) -> Attack {
        let name = &user.name;
        if spent_struggles && !user.moves.is_empty() && user.moves.iter().all(|slot| slot.pp <= 0) {
            self.log.push(format!("{name} has no PP left and struggles!"));
            let recoil = (user.stats.max_hp / 4).max(1);
            user.stats.hp -= recoil;
            self.log.push(format!("{} lost {recoil} HP.", user.name));
        } else {
            self.log.push(format!("{name} attacks!"));
        }
        Attack { slot: self.book.regular_attack(), effect: None, damage_mult: FIXED_ONE, linked: false }
    }

    /// Whether the move cannot be used: tormented, not the move it was
    /// tormented for; taunted, no status move.
    fn refused(&mut self, user: &Monster, index: usize) -> bool {
        const STATUS_CATEGORY: u8 = 2;
        let (name, id) = (&user.name, user.moves[index].id);
        let move_name = self.book.move_name(id);
        if user.status.tormented == Some(id) {
            self.log.push(format!("{name} is tormented: {move_name} cannot be used!"));
            return true;
        }
        if user.status.taunted_turns > 0 && user.moves[index].category == STATUS_CATEGORY {
            self.log.push(format!("{name} is taunted: {move_name} cannot be used!"));
            return true;
        }
        false
    }

    /// Bide, Revenge, Avalanche: the user braces itself for a turn,
    /// counting the damage it takes, then hits back with double that.
    fn bide(&mut self, user: &mut Monster, index: usize, unleashed: bool) -> Option<Attack> {
        let name = &user.name;
        if !unleashed {
            user.moves[index].pp -= 1;
            (user.status.charging, user.status.bide) = (Some(user.moves[index].id), Some(0));
            self.log.push(format!("{name} braces itself!"));
            return None;
        }
        const MOST_RETURNED: i32 = 999;
        let taken = user.status.bide.take().unwrap_or(0);
        user.status.bide_release = Some(taken.saturating_mul(2).min(MOST_RETURNED));
        self.log.push(format!("{} struck back!", user.name));
        Some(Attack { slot: user.moves[index], effect: None, damage_mult: FIXED_ONE, linked: false })
    }

    /// PP spent, and the move's strength if it is let out this turn; none
    /// when the turn goes into charging it (a Solar Beam in the sun needs
    /// no charging).
    fn charged(&mut self, user: &mut Monster, index: usize, unleashed: bool) -> Option<i32> {
        let id = user.moves[index].id;
        match (unleashed, charged_multiplier(&self.book.tables.constants, id)) {
            (true, multiplier) => Some(multiplier.unwrap_or(FIXED_ONE)),
            (false, Some(multiplier)) if id == status::SOLARBEAM && self.dungeon.weather.apparent() == weather::SUNNY => {
                self.spend_pp(user, index, 1);
                Some(multiplier)
            }
            // No-Charger: let out at once, for two PP.
            (false, Some(multiplier)) if iq::has(&user.stats, iq::NO_CHARGER) => {
                self.spend_pp(user, index, 2);
                Some(multiplier)
            }
            (false, Some(_)) => {
                self.spend_pp(user, index, 1);
                user.status.charging = Some(id);
                self.log.push(format!("{} gets ready for {}!", user.name, self.book.move_name(id)));
                None
            }
            (false, None) => {
                self.spend_pp(user, index, 1);
                Some(FIXED_ONE)
            }
        }
    }

    /// A move's PP spent, but now and then with PP Saver, or an exclusive
    /// item that does as much (each its own chance in 100, the game's, read
    /// from its overlay 10).
    fn spend_pp(&mut self, user: &mut Monster, index: usize, pp: i32) {
        const PP_SAVED_CHANCE: i32 = 10;
        let item = user.status.exclusive.has(crate::dungeon::exclusive::effect::MAY_NOT_CONSUME_PP);
        for saves in [iq::has(&user.stats, iq::PP_SAVER), item] {
            if saves && self.dungeon.rng.rand_int(100) < PP_SAVED_CHANCE {
                self.log.push(format!("{} saved its PP!", user.name));
                return;
            }
        }
        let slot = &mut user.moves[index];
        slot.pp = (slot.pp - pp).max(0);
    }

    /// What the move becomes, or why it fails, in the order the game asks.
    fn turn_into(&mut self, before: &[Monster], user: &mut Monster, after: &[Monster], known: &Known, using: &mut Using) -> Result<(), &'static str> {
        self.copied(user, using)?;
        self.reshaped(user, known, using)?;
        self.ready(user, known, using)?;
        self.called(before, user, after, known, using)
    }

    /// Copycat: the last move used on the floor, in its place. Nature
    /// Power: the floor's move; Camouflage: the floor's type.
    fn copied(&mut self, user: &mut Monster, using: &mut Using) -> Result<(), &'static str> {
        const COPYCAT: u16 = 525;
        const NATURE_POWER: u16 = 119;
        const CAMOUFLAGE: u16 = 286;
        let (dungeon, book) = (&mut *self.dungeon, self.book);
        if using.id == COPYCAT {
            let copied = dungeon.last_used.ok_or(NOTHING)?;
            self.log.push(format!("{} copied {}!", user.name, book.move_name(copied.id)));
            using.used = MoveSlot { pp: using.slot.pp, ..copied };
        } else {
            dungeon.last_used = Some(using.slot);
        }
        let ground = dungeon.spec(book).map_or((1, 0), |floor| (floor.camouflage, floor.nature_power));
        if using.used.id == NATURE_POWER {
            let picked = book.move_slot(ground.1).ok_or(NOTHING)?;
            self.log.push(format!("Nature Power turned into {}!", book.move_name(picked.id)));
            using.used = MoveSlot { pp: using.slot.pp, ..picked };
        }
        if using.used.id == CAMOUFLAGE {
            user.status.remember_identity(&user.stats);
            user.stats.types = (ground.0, 0);
        }
        Ok(())
    }

    /// Transform: the looks of one of the floor's monsters, at random.
    /// Conversion: the type of one of its moves, other than its own.
    /// Natural Gift: the berry or seed held gives the move its type and
    /// power.
    fn reshaped(&mut self, user: &mut Monster, known: &Known, using: &mut Using) -> Result<(), &'static str> {
        const TRANSFORM: u16 = 282;
        const CONVERSION: u16 = 183;
        const NATURAL_GIFT: u16 = 471;
        let (dungeon, book) = (&mut *self.dungeon, self.book);
        let constants = &book.tables.constants;
        if using.used.id == TRANSFORM {
            let kinds = dungeon.spec(book).map_or(0, |floor| floor.enemies.len());
            if kinds == 0 {
                return Err(FAILED);
            }
            let (low, high) = constants.transform_turns;
            user.status.disguise = Some(dungeon.rng.rand_int(kinds as i32) as u16);
            user.status.disguise_turns = dungeon.rng.rand_range(low, high.max(low) + 1).max(1);
        }
        if using.used.id == CONVERSION {
            let own = user.stats.types;
            let kinds: Vec<u8> = known.kinds.iter().copied().filter(|&kind| kind != 0 && (own.0, own.1) != (kind, 0)).collect();
            if kinds.is_empty() {
                return Err(FAILED);
            }
            user.status.remember_identity(&user.stats);
            user.stats.types = (kinds[dungeon.rng.rand_int(kinds.len() as i32) as usize], 0);
        }
        if using.used.id == NATURAL_GIFT
            && let Some((kind, power)) = constants.natural_gift(stack::id(user.stats.held))
        {
            (using.used.type_id, using.used.power) = (kind, power);
        }
        Ok(())
    }

    /// Last Resort: a move for when the others run dry. It fails while all
    /// of them have PP; each one spent makes it half as strong again. Spit
    /// Up has nothing to let out without a Stockpile first. Snore and Sleep
    /// Talk work asleep only.
    fn ready(&mut self, user: &Monster, known: &Known, using: &mut Using) -> Result<(), &'static str> {
        const SPIT_UP: u16 = 245;
        if using.used.id == LAST_RESORT {
            if known.spent == 0 {
                return Err(FAILED);
            }
            using.damage_mult = using.damage_mult * (2 + known.spent as i32) / 2;
        }
        if using.used.id == SPIT_UP && user.status.stockpile == 0 {
            return Err("But nothing was stored!");
        }
        if matches!(using.id, SNORE | SLEEP_TALK) && !user.status.asleep() {
            return Err(FAILED);
        }
        Ok(())
    }

    /// The moves that call another, at random: Sleep Talk, one of the
    /// user's other moves; Assist, one of the moves the monsters of the
    /// floor know (no move that becomes another); Metronome, one of its
    /// table; Me First, one of the damaging moves of the monster in front,
    /// half as strong again.
    fn called(&mut self, before: &[Monster], user: &Monster, after: &[Monster], known: &Known, using: &mut Using) -> Result<(), &'static str> {
        const ASSIST: u16 = 229;
        const METRONOME: u16 = 330;
        const ME_FIRST: u16 = 465;
        let (dungeon, book) = (&mut *self.dungeon, self.book);
        let constants = &book.tables.constants;
        let pp = using.slot.pp;
        if using.id == SLEEP_TALK {
            if known.talked.is_empty() {
                return Err(NOTHING);
            }
            let picked = known.talked[dungeon.rng.rand_int(known.talked.len() as i32) as usize];
            self.log.push(format!("{} used {} while asleep!", user.name, book.move_name(picked.id)));
            using.used = MoveSlot { pp, ..picked };
        }
        if using.id == ASSIST {
            const WILDCARDS: [u16; 5] = [229, 330, 525, 327, 465];
            let known = before.iter().chain(after).chain([user]).flat_map(|monster| monster.moves.iter().copied());
            let floor_moves: Vec<MoveSlot> = known.filter(|slot| !WILDCARDS.contains(&slot.id)).collect();
            if floor_moves.is_empty() {
                return Err(NOTHING);
            }
            let picked = floor_moves[dungeon.rng.rand_int(floor_moves.len() as i32) as usize];
            self.log.push(format!("Assist became {}!", book.move_name(picked.id)));
            using.used = MoveSlot { pp, ..picked };
        }
        if using.id == METRONOME {
            let wildcards: Vec<MoveSlot> = constants.metronome.iter().flatten().filter_map(|&id| book.move_slot(id)).collect();
            if wildcards.is_empty() {
                return Err(NOTHING);
            }
            let picked = wildcards[dungeon.rng.rand_int(wildcards.len() as i32) as usize];
            self.log.push(format!("Metronome turned into {}!", book.move_name(picked.id)));
            using.used = MoveSlot { pp, ..picked };
        }
        if using.used.id == ME_FIRST {
            let front = before.iter().chain(after).find(|monster| Some(monster.at) == dungeon.aimed_at && monster.alive());
            let taken: Vec<MoveSlot> =
                front.map_or(Vec::new(), |front| front.moves.iter().copied().filter(|slot| slot.power > 0 && slot.usable() && slot.id != ME_FIRST).collect());
            if taken.is_empty() {
                return Err(FAILED);
            }
            let picked = taken[dungeon.rng.rand_int(taken.len() as i32) as usize];
            self.log.push(format!("{} goes first with {}!", user.name, book.move_name(picked.id)));
            using.used = MoveSlot { pp, ..picked };
            using.damage_mult = using.damage_mult * constants.me_first / FIXED_ONE;
        }
        Ok(())
    }

    /// Lands an attack, whether it did not miss: all it does to the two
    /// shown as one number each.
    pub(crate) fn strike(&mut self, user: MonsterId, victim: MonsterId, attack: &Attack) -> bool {
        let mut monsters = std::mem::take(&mut self.dungeon.monsters);
        let mut landed = false;
        let sides = self.sides(&monsters, user, victim);
        if let Some((user, victim)) = pair(&mut monsters, user, victim) {
            let (watch_user, watch_victim) = (Watch::of(user), Watch::of(victim));
            landed = self.land(user, victim, attack, sides);
            watch_user.show(user, self.sights);
            watch_victim.show(victim, self.sights);
        }
        self.dungeon.monsters = monsters;
        landed
    }

    pub(crate) fn apply(&mut self, effect: MoveEffect, monster: &mut Monster) -> bool {
        apply_effect(effect, &mut self.dungeon.rng, &self.book.tables.constants, &mut monster.stats, &mut monster.status)
    }

    /// An effect on a monster, told.
    pub(crate) fn affect(&mut self, effect: MoveEffect, monster: &mut Monster) -> bool {
        let applied = self.apply(effect, monster);
        self.log.push(effect_message(self.book, effect, &monster.name, applied));
        applied
    }

    pub(crate) fn cast(&mut self, user: &mut Monster, effect: MoveEffect) {
        self.affect(effect, user);
        self.changes_floor(effect);
    }

    /// What a cast effect leaves on the floor: the weather, a sport.
    pub(crate) fn changes_floor(&mut self, effect: MoveEffect) {
        self.change_weather(effect);
        if let MoveEffect::Sport { water } = effect {
            let (low, high) = self.book.tables.constants.sport_turns;
            let turns = self.dungeon.rng.rand_range(low, high.max(low + 1));
            match water {
                true => self.dungeon.water_sport = turns,
                false => self.dungeon.mud_sport = turns,
            }
        }
    }

    pub(crate) fn change_weather(&mut self, effect: MoveEffect) {
        if let MoveEffect::Weather(kind) = effect {
            self.dungeon.weather.set(kind, self.book.tables.constants.weather_move_turns);
            self.log.push(format!("The weather changed: {}.", super::dungeon::weather_name(self.dungeon.weather.current)));
        }
    }

    fn team_move(&mut self, user: &mut Monster, effect: MoveEffect, slot: Option<MoveSlot>) {
        let (at, facing, team) = (user.at, user.facing, user.team());
        // What changes the floor: a blast around the user, which faints; the
        // wall it faces broken.
        match effect {
            MoveEffect::Explode(_) if self.dungeon.weather.damp => {
                self.log.push("But the damp prevents the explosion!".to_string());
                return;
            }
            MoveEffect::Explode(radius) => {
                self.log.push(format!("{} exploded!", user.name));
                user.stats.hp = 0;
                self.dungeon.blasts.push((at, radius, Some(at)));
                return;
            }
            MoveEffect::Hurl | MoveEffect::KnockOff | MoveEffect::BeatUp | MoveEffect::SwitchPlaces => {
                self.dungeon.shoves.push(Shove { effect, from: at, facing, team });
                return;
            }
            // Recycle: the team's used-up TMs are as good as new (an enemy's
            // has nothing to renew).
            MoveEffect::Recycle => {
                match team {
                    true => self.dungeon.recycle = true,
                    false => self.log.push("But it failed!".to_string()),
                }
                return;
            }
            MoveEffect::Gravity => {
                match self.dungeon.gravity == Some(self.dungeon.generation) {
                    true => self.log.push("But gravity is already intense.".to_string()),
                    false => {
                        self.dungeon.gravity = Some(self.dungeon.generation);
                        self.log.push("Gravity intensified!".to_string());
                    }
                }
                return;
            }
            MoveEffect::SmashWall => {
                let wall = Pos::new(at.x + facing.0, at.y + facing.1);
                let inside = wall.x > 0 && wall.y > 0 && wall.x < FLOOR_WIDTH - 1 && wall.y < FLOOR_HEIGHT - 1;
                if inside && self.dungeon.floor.tile(wall) == Tile::Wall {
                    self.dungeon.blasts.push((wall, 0, None));
                    self.log.push(format!("{} broke the wall!", user.name));
                } else {
                    self.log.push("But it failed!".to_string());
                }
                return;
            }
            _ => {}
        }
        let target = slot.map_or(0, |slot| slot.target);
        let mut spread = Spread { from: at, team, effect, range: slot.map_or(range::ROOM, |slot| slot.range) };
        // (An exclusive item: what the user does for itself, it does for the
        // whole team on the floor.)
        const TARGET_USER: u8 = 3;
        let extended =
            target == TARGET_USER && effect != MoveEffect::Warp && user.status.exclusive.has(crate::dungeon::exclusive::effect::EXTEND_SELF_EFFECTS_TO_TEAM);
        let target = if extended {
            spread.range = range::FLOOR;
            TARGET_PARTY
        } else {
            target
        };
        match target {
            TARGET_TEAMMATES => {
                if let MoveEffect::HealingWish { .. } = effect {
                    user.stats.hp = 0;
                    self.log.push(format!("{} sacrifices itself!", user.name));
                }
                self.dungeon.spread.push(spread);
            }
            TARGET_PARTY => {
                self.cast(user, effect);
                self.dungeon.spread.push(spread);
            }
            _ => self.cast(user, effect),
        }
    }

    /// Changes a monster in the list, what it does to it being shown.
    pub(crate) fn watch<R>(&mut self, at: usize, change: impl FnOnce(&mut Monster) -> R) -> R {
        watched(&mut self.dungeon.monsters[at], self.sights, change)
    }
}

/// The chain of linked moves the move at `index` is in, first to last.
pub(crate) fn link_chain(moves: &[MoveSlot], index: usize) -> Vec<usize> {
    let mut first = index.min(moves.len().saturating_sub(1));
    while first > 0 && moves[first].linked {
        first -= 1;
    }
    let mut chain = vec![first];
    while chain.last().is_some_and(|&last| moves.get(last + 1).is_some_and(|next| next.linked)) {
        chain.push(chain.last().unwrap() + 1);
    }
    chain
}

/// A linked move out of PP comes off its chain (the Link Box's note: "if
/// either move's PP drops to zero"). Whether anything came apart.
pub(crate) fn unlink_spent(moves: &mut [MoveSlot]) -> bool {
    let mut parted = false;
    for at in 0..moves.len() {
        if moves[at].pp > 0 {
            continue;
        }
        let next = moves.get(at + 1).is_some_and(|next| next.linked);
        if moves[at].linked || next {
            parted = true;
            moves[at].linked = false;
            if let Some(next) = moves.get_mut(at + 1) {
                next.linked = false;
            }
        }
    }
    parted
}
