//! A floor's life: building it, foes coming back, the fallen, the stairs,
//! the moves a member would learn, the traps laid, what the team sees.

use crate::dungeon::exclusive::effect::{EXP_BOOST, MORE_MONEY_DROPS, PICKUP_BOOST as PICKUP_BOOSTED};
use crate::dungeon::minimap::Explored;
use crate::dungeon::status::item;
use crate::dungeon::{MoveSlot, Pos, Statuses, ability, ai, defeat_exp, held, iq, pick_weighted, random_tile, stack};
use crate::tables::{Fixed, Placed};

use super::attack::chebyshev;
use super::dungeon::{Ask, DIRECTIONS, FAINT_FRAMES, FloorItem, FloorTrap, Outcome};
use super::shops::piled;
use super::spawning::{KECLEON, hunter_stats, spawn_enemy, wild_monster};
use super::team::earn;
use super::treasure;
use super::watch::watched;
use super::{Dungeon, Input, Monster, MonsterId, Recruit, Run, Side, Sight};

const STAIRS_DOWN: u16 = 0x1309;
const STAIRS_UP: u16 = 0x130A;
/// The frames before a team down ends the run, the last blow seen.
const KNOCKED_OUT: u32 = 60;
/// What a Friend Bow adds to a recruit rate, in 1000ths.
const FRIEND_BOW_BOOST: i32 = 150;
/// What Fast Friend adds (the game's, read from its overlay 10).
const FAST_FRIEND_BOOST: i32 = 50;
/// How far from the leader a fallen foe may still ask to join.
const JOIN_REACH: i32 = 2;
const PLAIN_SEED: u16 = 85;
/// The piles of Poké a Dough Seed adds to the next floor (this project's
/// own figure).
const DOUGH_SEED_POKE: usize = 4;
/// The chance in 100 that Pickup finds an item as a floor begins (this
/// project's own figure).
const PICKUP_CHANCE: i32 = 30;

impl Run {
    /// On to the next floor; past the last one the dungeon is cleared. (A
    /// dungeon without floors never ends.)
    pub(crate) fn next_floor(&mut self) {
        let book = self.book.clone();
        let dungeon = self.here();
        let floors = dungeon.id.and_then(|id| book.tables.dungeons.get(id as usize)).map_or(0, |table| table.floors.len());
        if floors != 0 && dungeon.floor_number as usize >= floors {
            dungeon.outcome = Some(Outcome::Cleared);
            return;
        }
        dungeon.floor_number += 1;
        self.new_floor();
    }

    /// On to the next floor at once, as by its stairs, wherever the leader
    /// stands (a developer's shortcut: `floors` when a film is played). Past
    /// the last floor the dungeon is cleared. Nothing out of a dungeon, or
    /// once it is over.
    pub fn skip_floor(&mut self) {
        if self.dungeon.as_ref().is_none_or(|dungeon| dungeon.outcome.is_some() || dungeon.ask.is_some()) {
            return;
        }
        let sound = if self.here().goes_up { STAIRS_UP } else { STAIRS_DOWN };
        self.show(Sight::Sound(sound));
        self.next_floor();
        let dungeon = self.here();
        let at = dungeon.leader().at;
        dungeon.stairs_seen = Some((dungeon.generation, at));
    }

    /// The team down at once, as if the last blow fell (a developer's
    /// shortcut, as `skip_floor`; the end-to-end tests'): it faints as it
    /// would when the next turn is played. Nothing out of a dungeon, or once
    /// it is over.
    pub fn faint_team(&mut self) {
        if self.dungeon.as_ref().is_none_or(|dungeon| dungeon.outcome.is_some()) {
            return;
        }
        for monster in self.here().monsters.iter_mut().filter(|monster| monster.team()) {
            monster.stats.hp = 0;
        }
    }

    /// The floor `floor_number` from the RNG's state, the team on it.
    pub(crate) fn new_floor(&mut self) {
        let book = self.book.clone();
        let dungeon = self.here();
        dungeon.enter_floor(dungeon.spec(&book));
        self.build_floor();
        // A new floor plays its own song (the shop's own aside).
        let dungeon = self.here();
        if let Some(song) = dungeon.spec(&book).and_then(|floor| floor.music)
            && dungeon.song != Some(song)
            && dungeon.song.is_some()
        {
            dungeon.song = Some(song);
            self.show(Sight::Music(Some(song)));
        }
    }

    /// Those of the old floor go, and what lay there; the team stands by
    /// the leader; the generator marked where the rest goes.
    fn build_floor(&mut self) {
        let book = self.book.clone();
        let dungeon = self.here();
        dungeon.monsters.retain(|monster| monster.team() && monster.fading.is_none_or(|_| monster.side == Side::Leader));
        (dungeon.items, dungeon.traps, dungeon.shop) = (Vec::new(), Vec::new(), None);
        (dungeon.spawn_turn, dungeon.spawn_count) = (dungeon.turn, 0);
        dungeon.explored = Explored::default();
        let spawn = dungeon.floor.player_spawn;
        let leader = dungeon.leader_mut();
        leader.at = spawn;
        // A leader with Map Surveyor knows the whole floor, with Stair
        // Sensor where the stairs are.
        let (surveyor, sensor) = (iq::has(&leader.stats, iq::MAP_SURVEYOR), iq::has(&leader.stats, iq::STAIR_SENSOR));
        if surveyor {
            dungeon.explored.reveal_all();
        }
        if sensor {
            dungeon.explored.reveal_around(dungeon.floor.stairs);
        }
        self.place_team();
        let dungeon = self.here();
        for tile in dungeon.floor.enemy_spawns.clone() {
            let dormant = dungeon.floor.in_monster_house(tile);
            spawn_enemy(dungeon, &book, tile, dormant);
        }
        // A floor laid out by hand places its own.
        let spec = dungeon.spec(&book);
        if let Some(fixed) = spec.and_then(|floor| floor.fixed.as_ref()) {
            self.place_fixed(fixed);
        }
        self.slow_start();
        let dungeon = self.here();
        // The jobs of a dungeon the team chose.
        if dungeon.jobs {
            self.place_job_targets();
        }
        self.show(Sight::NewFloor);
        if spec.is_none() {
            return;
        }
        self.place_shop();
        let Some(spec) = spec else {
            return;
        };
        let dungeon = self.here();
        for tile in dungeon.floor.item_spawns.clone() {
            let Some(kind) = spec.items.pick(&mut dungeon.rng) else {
                continue;
            };
            let kind = piled(&book, kind, &mut dungeon.rng);
            let sticky = dungeon.rng.rand_int(100) < spec.sticky_chance;
            dungeon.items.push(FloorItem { at: tile, item: stack::stuck(kind, sticky), price: None });
        }
        // Items buried in walls exist but cannot be seen; a Mobile Orb or
        // Rock Smash reaches them.
        for tile in dungeon.floor.buried_item_spawns.clone() {
            let Some(kind) = spec.items.pick(&mut dungeon.rng) else {
                continue;
            };
            let kind = piled(&book, kind, &mut dungeon.rng);
            dungeon.items.push(FloorItem { at: tile, item: kind, price: None });
        }
        // A Dough Seed eaten on the floor before: more Poké lie about.
        if std::mem::take(&mut dungeon.dough_seed) {
            let lying: Vec<Pos> = dungeon.items.iter().map(|item| item.at).collect();
            for _ in 0..DOUGH_SEED_POKE {
                if let Some(tile) = dungeon.warp_tile(&lying) {
                    dungeon.items.push(FloorItem { at: tile, item: item::POKE, price: None });
                }
            }
        }
        for tile in dungeon.floor.trap_spawns.clone() {
            if let Some(kind) = pick_weighted(&mut dungeon.rng, spec.traps.iter().copied().enumerate()) {
                dungeon.traps.push(FloorTrap { at: tile, kind, seen: false, team: false });
            }
        }
        // Pickup: a team member holding nothing may come with something of
        // the floor's.
        // (An exclusive item makes it likelier: the game's 1.35, read from
        // its overlay 10.)
        const PICKUP_BOOST: i32 = 345;
        let pickers: Vec<(MonsterId, i32)> = dungeon
            .monsters
            .iter()
            .filter(|monster| monster.team() && monster.stats.held == 0 && ability::has(&monster.stats, ability::PICKUP))
            .map(|monster| (monster.id, if monster.status.exclusive.has(PICKUP_BOOSTED) { PICKUP_CHANCE * PICKUP_BOOST / 256 } else { PICKUP_CHANCE }))
            .collect();
        for (who, chance) in pickers {
            let dungeon = self.here();
            if dungeon.rng.rand_int(100) >= chance {
                continue;
            }
            let Some(kind) = spec.items.pick(&mut dungeon.rng).filter(|&kind| stack::id(kind) != item::POKE) else {
                continue;
            };
            let Some(monster) = dungeon.monster_mut(who) else {
                continue;
            };
            monster.stats.held = kind;
            let (name, item_name) = (monster.name.clone(), book.item_name(kind));
            let said = book.phrase(crate::strings::log_text::PICKED_UP, &[("string:0", &name), ("item:0", &item_name)]);
            self.log.push(said.unwrap_or_else(|| format!("{name} picked up {item_name}.")));
        }
    }

    /// Slow Start: as a floor begins, those who have it are slowed.
    fn slow_start(&mut self) {
        let book = self.book.clone();
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        let Dungeon { rng, monsters, .. } = dungeon;
        for monster in monsters.iter_mut().filter(|monster| monster.alive() && ability::has(&monster.stats, ability::SLOW_START)) {
            monster.status.lower_speed(rng, &book.tables.constants);
            self.log.push(format!("{} is a slow starter!", monster.name));
        }
    }

    /// Those a room laid out by hand places, and what lies there: its foes,
    /// who stands with the team, who only watches, its items and traps.
    fn place_fixed(&mut self, fixed: &Fixed) {
        let book = self.book.clone();
        let dungeon = self.here();
        let facing = |placed: &Placed| DIRECTIONS[placed.facing as usize % DIRECTIONS.len()];
        for placed in &fixed.monsters {
            let movement = book.species(placed.enemy.species).map_or(0, |species| species.movement);
            let mut monster = wild_monster(dungeon, &book, &placed.enemy, Fixed::tile(placed.at), Statuses::for_species(movement));
            (monster.recruitable, monster.facing) = (None, facing(placed));
            dungeon.monsters.push(monster);
        }
        for placed in &fixed.watchers {
            let (species, id) = (placed.enemy.species, dungeon.new_id());
            let mut monster = Monster::new(
                id,
                Side::Keeper,
                book.species_name(species),
                Some(species),
                placed.enemy.stats,
                Vec::new(),
                Statuses::default(),
                Fixed::tile(placed.at),
            );
            monster.facing = facing(placed);
            dungeon.monsters.push(monster);
        }
        for &(at, item) in &fixed.items {
            dungeon.items.push(FloorItem { at: Fixed::tile(at), item, price: None });
        }
        for &(at, kind, seen) in &fixed.traps {
            dungeon.traps.push(FloorTrap { at: Fixed::tile(at), kind: kind as usize, seen, team: false });
        }
        for placed in &fixed.allies {
            let of = &placed.enemy.stats;
            let (species, stats) = (placed.enemy.species, [of.level, of.max_hp, of.attack, of.defense, of.sp_attack, of.sp_defense]);
            let ally = Recruit { species, name: book.species_name(species), stats, moves: Vec::new() };
            self.place_recruit(None, &ally, Fixed::tile(placed.at));
        }
    }

    /// On a new floor the team starts around the leader, its stat stages
    /// and sealed moves reset like the leader's.
    fn place_team(&mut self) {
        let dungeon = self.here();
        let leader = dungeon.leader().at;
        let floor = &dungeon.floor;
        let around = ai::DIRECTIONS.iter().filter(|&&(dx, dy)| floor.can_step(leader, dx, dy)).map(|&(dx, dy)| Pos::new(leader.x + dx, leader.y + dy));
        // (Each on a tile of its own, as far as there are.)
        let spots: Vec<Pos> = around.clone().filter(|tile| !floor.enemy_spawns.contains(tile)).chain(around).collect();
        for (at, ally) in dungeon.monsters.iter_mut().filter(|monster| monster.side == Side::Ally).enumerate() {
            ally.at = spots.get(at).copied().unwrap_or(leader);
            ally.status.leave_floor();
            ally.status.charging = None;
            ally.moves.iter_mut().for_each(|slot| slot.sealed = false);
        }
    }

    /// New enemies keep appearing as turns pass, away from the leader:
    /// one each `spawn_cooldown` turns while the floor holds fewer than its
    /// cap, twice as often while it holds under half of it.
    // (Those of a Monster House are not counted. With a thief on the floor
    // Kecleons come at their own pace, without a cap.)
    pub(crate) fn respawn_enemies(&mut self) {
        let book = self.book.clone();
        let constants = &book.tables.constants;
        let dungeon = self.here();
        if dungeon.spawn_turn == dungeon.turn {
            return;
        }
        dungeon.spawn_turn = dungeon.turn;
        dungeon.spawn_count += 1;
        // (Illuminate hit: one comes now, whatever the count.)
        let illuminated = std::mem::take(&mut dungeon.illuminated);
        let floor = &dungeon.floor;
        let wild = dungeon.monsters.iter().filter(|monster| monster.side == Side::Foe && monster.alive() && !floor.in_monster_house(monster.at)).count() as i32;
        let cap = if floor.monster_house.is_some() { constants.spawn_cap_monster_house } else { constants.spawn_cap };
        let wait = match (dungeon.thief_alert, wild * 2 < cap) {
            (true, _) => constants.spawn_cooldown_alert,
            (false, true) => constants.spawn_cooldown / 2,
            (false, false) => constants.spawn_cooldown,
        };
        if !illuminated && (wait <= 0 || dungeon.spawn_count < wait) {
            return;
        }
        dungeon.spawn_count = 0;
        if wild >= cap && !dungeon.thief_alert && !illuminated {
            return;
        }
        let alive = dungeon.monsters.iter().filter(|monster| monster.alive()).map(|monster| monster.at);
        let lying = dungeon.items.iter().map(|item| item.at).chain(dungeon.traps.iter().map(|trap| trap.at));
        let occupied: Vec<Pos> = alive.chain(lying).collect();
        let leader = dungeon.leader().at;
        let Some(tile) = random_tile(&mut dungeon.rng, floor, leader, |tile| occupied.contains(&tile)) else {
            return;
        };
        if dungeon.thief_alert {
            let (id, name, level) = (dungeon.new_id(), book.species_name(KECLEON), dungeon.leader().stats.level);
            dungeon.monsters.push(Monster::new(id, Side::Foe, name, Some(KECLEON), hunter_stats(level), Vec::new(), Statuses::default(), tile));
        } else {
            spawn_enemy(dungeon, &book, tile, false);
        }
    }

    /// The defeated go: a foe gives what it held and its experience, and
    /// may ask to join; a Reviver Seed brings a member back, a recruit
    /// without one goes back to the guild, and the run is over for the
    /// others (the leader, its partner).
    pub(crate) fn resolve_deaths(&mut self) {
        let book = self.book.clone();
        let dungeon = self.here();
        let fallen: Vec<MonsterId> = dungeon
            .monsters
            .iter()
            .filter(|monster| monster.side == Side::Foe && !monster.alive() && monster.fading.is_none())
            .map(|monster| monster.id)
            .collect();
        // A floor laid out for a fight is left once nobody is left to fight.
        let fight = dungeon.spec(&book).and_then(|floor| floor.fixed.as_ref()).is_some_and(|fixed| fixed.ends_with_enemies && !fixed.monsters.is_empty());
        let standing = dungeon.monsters.iter().any(|monster| monster.side == Side::Foe && monster.alive());
        if fight && !standing && !fallen.is_empty() {
            dungeon.fight_won = true;
        }
        for foe in fallen {
            self.foe_falls(foe);
        }
        self.team_falls();
    }

    /// What a defeated foe drops where the dungeon lets it
    /// (`Restrictions::treasure_boxes`; never on a floor laid out for a
    /// fight nor in a maze of the Dojo): by its species' weights, more with
    /// what the one who brought it down has (`treasure`).
    fn treasure_drop(&mut self, enemy: &Monster) {
        use crate::dungeon::exclusive::effect::MORE_TREASURE_DROPS;
        let book = self.book.clone();
        let dungeon = self.here();
        let fight = dungeon.spec(&book).and_then(|floor| floor.fixed.as_ref()).is_some_and(|fixed| fixed.ends_with_enemies && !fixed.monsters.is_empty());
        let maze = dungeon.id.is_some_and(crate::dojo::is_maze);
        let Some(species) = enemy.species.filter(|_| dungeon.rules.more.treasure_boxes && !fight && !maze) else {
            return;
        };
        let by = enemy.felled_by.and_then(|id| dungeon.monster(id));
        let felled = by.map_or_else(treasure::Felled::default, |by| treasure::Felled {
            bonus: i32::from(iq::has(&by.stats, iq::COLLECTOR)) + i32::from(by.status.exclusive.has(MORE_TREASURE_DROPS)),
            honey_gather: ability::has(&by.stats, ability::HONEY_GATHER),
            money: by.status.exclusive.has(MORE_MONEY_DROPS),
        });
        let rank = dungeon.id.map_or(0, |id| book.tables.jobs.floor_rank(id, dungeon.floor_number));
        let at = enemy.at;
        match treasure::drop_of(&mut dungeon.rng, &book.extras, species, rank, felled) {
            Some(treasure::Drop::Item(item)) => {
                dungeon.items.push(FloorItem { at, item, price: None });
                self.log.push(format!("{} dropped {}.", enemy.name, book.item_name(item)));
            }
            Some(treasure::Drop::Money) => {
                dungeon.items.push(FloorItem { at, item: item::POKE, price: None });
                self.log.push(format!("{} dropped some Poké.", enemy.name));
            }
            None => {}
        }
    }

    /// An enemy the team brought down is worth experience to each member.
    fn foe_falls(&mut self, foe: MonsterId) {
        let book = self.book.clone();
        let dungeon = self.here();
        let Some(enemy) = dungeon.monster_mut(foe) else {
            return;
        };
        // It takes its blow, blinks out, then goes.
        enemy.fading = Some(FAINT_FRAMES);
        let enemy = enemy.clone();
        self.show(Sight::Fainted(foe));
        // What it held falls where it stood.
        if enemy.stats.held != 0 {
            self.here().items.push(FloorItem { at: enemy.at, item: enemy.stats.held, price: None });
            self.log.push(format!("{} dropped {}.", enemy.name, book.item_name(enemy.stats.held)));
        }
        // Where the dungeon lets them, a treasure box now and then (an
        // Unown's stone; Poké for an exclusive item).
        self.treasure_drop(&enemy);
        if !enemy.status.struck_by_team {
            return;
        }
        self.may_join(&enemy);
        let dungeon = self.here();
        if !dungeon.rules.enemies_give_exp {
            return;
        }
        let exp = defeat_exp(enemy.exp_value, enemy.status.hit_by_move, enemy.status.hit_by_link);
        // (A share for those at the base too, given as the team leaves.)
        if dungeon.rules.options.exp_share {
            dungeon.shared_exp += exp;
        }
        // (Exp. Elite: a share more; an exclusive item's boost too.)
        let constants = &book.tables.constants;
        let earned = |monster: &Monster| {
            let elite = if iq::has(&monster.stats, iq::EXP_ELITE) { exp * constants.exp_elite_boost / 100 } else { 0 };
            let boost = if monster.status.exclusive.has(EXP_BOOST) { exp * constants.exclusive_exp_boost / 100 } else { 0 };
            exp + elite + boost
        };
        let team: Vec<MonsterId> = dungeon.monsters.iter().filter(|monster| monster.team() && monster.alive()).map(|monster| monster.id).collect();
        for member in team {
            let dungeon = self.dungeon.as_mut().expect("a dungeon");
            let Some(monster) = dungeon.monsters.iter_mut().find(|monster| monster.id == member) else {
                continue;
            };
            let amount = earned(monster);
            // (Each member's share is told, as the game tells it.)
            let told = amount.to_string();
            self.log.push(
                book.phrase(crate::strings::log_text::GAINED_EXP, &[("string:0", &monster.name), ("digits_c:0", &told)])
                    .unwrap_or_else(|| format!("{} gained {amount} Exp. Points.", monster.name)),
            );
            let log = &mut self.log;
            let asked: Vec<MoveSlot> = watched(monster, &mut self.sights, |monster| {
                let Some(experience) = &mut monster.experience else {
                    return Vec::new();
                };
                earn(&book, log, &monster.name, &mut monster.stats, &mut monster.moves, experience, amount)
            });
            dungeon.learning.extend(asked.into_iter().map(|slot| (member, slot)));
        }
    }

    /// It may ask to join the team where the dungeon allows it: the
    /// leader, standing and close by, must see it fall. The chance is its
    /// species' rate, plus what the leader's level adds, plus a Friend Bow's
    /// share, in 1000ths.
    fn may_join(&mut self, enemy: &Monster) {
        let book = self.book.clone();
        let dungeon = self.here();
        let leader = dungeon.leader();
        let near = chebyshev(leader.at, enemy.at) <= JOIN_REACH && ai::can_see(&dungeon.floor, leader.at, enemy.at);
        let bow = if held::holds(&leader.stats, held::FRIEND_BOW) { FRIEND_BOW_BOOST } else { 0 };
        // (And Fast Friend.)
        let bow = bow + if iq::has(&leader.stats, iq::FAST_FRIEND) { FAST_FRIEND_BOOST } else { 0 };
        let chance = enemy.recruitable.map_or(0, |recruitable| recruitable.rate as i32);
        let chance = if chance > 0 { chance + bow + book.recruit_boost(leader.stats.level) } else { 0 };
        let open = dungeon.rules.recruitment && leader.alive() && near && dungeon.ask.is_none();
        let Some(recruitable) = enemy.recruitable.filter(|_| open && chance > 0) else {
            return;
        };
        if dungeon.rng.rand_int(1000) >= chance.min(1000) {
            return;
        }
        let of = &enemy.stats;
        let recruit = Recruit {
            species: recruitable.species,
            name: enemy.name.clone(),
            stats: [of.level, of.max_hp, of.attack, of.defense, of.sp_attack, of.sp_defense],
            moves: Vec::new(),
        };
        dungeon.ask = Some(Ask::Recruit(recruit));
        self.log.push(format!("{} got up and wants to join the team!", enemy.name));
    }

    /// A Reviver Seed in the bag brings a fainted member back with full HP
    /// and PP, spent into a Plain Seed. (What a team that faints loses is
    /// `leave_dungeon`'s.)
    fn team_falls(&mut self) {
        let book = self.book.clone();
        let mut controlled_fell = false;
        let dungeon = self.dungeon.as_mut().expect("a dungeon");
        for monster in dungeon.monsters.iter_mut().filter(|monster| monster.team() && !monster.alive() && monster.fading.is_none()) {
            if let Some(seed) = self.bag.items.iter().position(|&lying| lying == item::REVIVER_SEED) {
                self.bag.items[seed] = PLAIN_SEED;
                watched(monster, &mut self.sights, |monster| {
                    monster.stats.hp = monster.stats.max_hp;
                    monster.status.cure_all();
                    monster.moves.iter_mut().for_each(|slot| slot.pp = slot.max_pp);
                });
                self.log.push(format!("{} was revived by {}!", monster.name, book.item_name(item::REVIVER_SEED)));
                continue;
            }
            self.sights.push(Sight::Fainted(monster.id));
            if matches!(monster.extra, Some(Some(_))) {
                // A recruit that faints goes back to the guild, as in the
                // original, once it blinked out; the player moves the hero
                // again if it was the one moved.
                monster.fading = Some(FAINT_FRAMES);
                controlled_fell |= monster.side == Side::Leader;
                self.log.push(format!("{} returned to the base.", monster.name));
            } else {
                // (The run ends once the blow is seen: `knock_out`.)
                monster.fading = Some(u32::MAX);
                dungeon.knocked_out.get_or_insert(KNOCKED_OUT);
            }
        }
        if controlled_fell {
            self.hero_controlled();
        }
    }

    /// Those blinking out are gone once it is over.
    pub(crate) fn fade_fainted(&mut self) {
        let unseen = self.unseen;
        let dungeon = self.here();
        for monster in &mut dungeon.monsters {
            monster.fading = monster.fading.map(|left| if unseen && left < u32::MAX { 0 } else { left.saturating_sub(1) });
        }
        dungeon.monsters.retain(|monster| monster.fading != Some(0));
    }

    /// Stepped on, the stairs ask whether to go on (the original's menu).
    /// Like the original, the leader may start a floor on them: they only
    /// work when stepped onto. Stat stages do not follow.
    pub(crate) fn descend_stairs(&mut self) {
        let dungeon = self.here();
        // A fight won is as good as stairs taken (the dead are cleared first).
        if std::mem::take(&mut dungeon.fight_won) {
            return self.next_floor();
        }
        let at = dungeon.leader().at;
        let stepped = dungeon.stairs_seen.is_some_and(|(generation, seen)| generation == dungeon.generation && seen != at);
        dungeon.stairs_seen = Some((dungeon.generation, at));
        if at == dungeon.floor.stairs && stepped && dungeon.ask.is_none() {
            dungeon.ask = Some(Ask::Stairs);
            return;
        }
        if !std::mem::take(&mut dungeon.take_stairs) || at != dungeon.floor.stairs {
            return;
        }
        let sound = if dungeon.goes_up { STAIRS_UP } else { STAIRS_DOWN };
        self.show(Sight::Sound(sound));
        self.next_floor();
        let dungeon = self.here();
        let at = dungeon.leader().at;
        dungeon.stairs_seen = Some((dungeon.generation, at));
        dungeon.leader_mut().status.leave_floor();
    }

    /// Lays the traps monsters set under themselves (Spikes): the team's
    /// are sprung by foes only, the foes' by the team.
    pub(crate) fn lay_traps(&mut self) {
        let dungeon = self.here();
        for at in 0..dungeon.monsters.len() {
            let monster = &mut dungeon.monsters[at];
            let Some(kind) = monster.status.laying.take() else {
                continue;
            };
            let (tile, team) = (monster.at, monster.team());
            if !dungeon.traps.iter().any(|trap| trap.at == tile) {
                dungeon.traps.push(FloorTrap { at: tile, kind, seen: true, team });
            }
        }
    }

    /// Gives each monster the IQ skills its IQ opens: a team member by its
    /// own IQ, less those switched off; an enemy by the floor's.
    pub(crate) fn keep_iq(&mut self) {
        let book = self.book.clone();
        let tables = &book.tables.iq;
        let switched_off = self.team.progress.iq_off.clone();
        let dungeon = self.here();
        let floor_iq = dungeon.spec(&book).map_or(0, |floor| floor.enemy_iq);
        let disabled = dungeon.rules.iq_disabled;
        for monster in &mut dungeon.monsters {
            if !monster.team() {
                monster.stats.iq = floor_iq;
            }
            let (iq, group) = (monster.stats.iq, monster.stats.iq_group);
            let always_on = iq::ALWAYS_ON.iter().fold(0u128, |bits, &skill| bits | 1 << skill);
            monster.stats.iq_skills = if disabled {
                0
            } else if monster.team() {
                tables.skills(group, iq) & !(switched_off.get(&monster.name).copied().unwrap_or(0) & !always_on)
            } else {
                tables.enemy_skills(group, iq)
            };
            iq::keep_boosts(&mut monster.stats, &mut monster.moves, &mut monster.tough);
        }
    }

    /// What the leader sees is known from now on.
    pub(crate) fn see_floor(&mut self) {
        let dungeon = self.here();
        let at = dungeon.leader().at;
        dungeon.explored.reveal(&dungeon.floor, at);
    }

    /// For the first of `Dungeon::learning`: which move to forget, if any.
    pub(crate) fn forget(&mut self, input: &Input) {
        let book = self.book.clone();
        let dungeon = self.here();
        let Some(&(who, slot)) = dungeon.learning.first() else {
            return;
        };
        let Some(monster) = dungeon.monster_mut(who) else {
            dungeon.learning.remove(0);
            return;
        };
        let Input::Forget(forgotten) = *input else {
            return;
        };
        let new = book.move_name(slot.id);
        let told = match forgotten.filter(|&at| at < monster.moves.len()) {
            Some(at) => {
                let old = book.move_name(monster.moves[at].id);
                monster.moves[at] = slot;
                format!("{} forgot {old} and learned {new}!", monster.name)
            }
            None => format!("{} did not learn {new}.", monster.name),
        };
        dungeon.learning.remove(0);
        self.log.push(told);
    }

    /// What a menu of the dungeon changes.
    pub(crate) fn menu_input(&mut self, input: &Input) {
        match input {
            Input::QueueMove(slot) => self.here().queued = Some(*slot),
            Input::Quicksave => self.quicksave(),
            Input::Quickload(text) => self.quickload(text),
            Input::GiveUp => self.here().outcome = Some(Outcome::Abandoned),
            Input::SetTactic { who, tactic } => self.set_tactic(*who, *tactic),
            Input::ToggleIq { who, skill } => self.toggle_iq(*who, *skill),
            Input::Link { who, slot } => self.link(*who, *slot),
            Input::SwitchControl => self.switch_control(),
            Input::MakeLeader(who) => self.make_leader(*who),
            Input::SwitchMove { who, slot } => self.switch_move(*who, *slot),
            _ => {}
        }
    }

    /// The next member of the team is the one the player moves, if the
    /// rules let them (`RuleOptions::team_control`); none is left out.
    pub(crate) fn switch_control(&mut self) {
        let dungeon = self.here();
        if !dungeon.rules.options.team_control {
            return;
        }
        let members: Vec<MonsterId> = dungeon
            .monsters
            .iter()
            .filter(|monster| monster.team() && monster.alive() && monster.fading.is_none() && monster.experience.is_some() && monster.escorted.is_none())
            .map(|monster| monster.id)
            .collect();
        let now = dungeon.leader().id;
        let Some(at) = members.iter().position(|&id| id == now) else {
            return;
        };
        if let Some(&next) = members.get((at + 1) % members.len()).filter(|&&next| next != now) {
            self.hand_control(next);
        }
    }

    /// The member chosen leads the team from now on (the dungeon menu's
    /// "Leader"): one of the team standing, not a client, where the
    /// dungeon lets the leader change (`Restrictions::leader_change`, or
    /// the option to control the whole team), never under Hidden Land's
    /// rules.
    fn make_leader(&mut self, who: MonsterId) {
        let rules = &self.here().rules;
        if !(rules.more.leader_change || rules.options.team_control) || self.here().hidden_land {
            self.log.push("The leader can't be changed in this dungeon.".to_string());
            return;
        }
        let able =
            |monster: &Monster| monster.team() && monster.alive() && monster.fading.is_none() && monster.experience.is_some() && monster.escorted.is_none();
        if self.here().monster(who).is_some_and(able) {
            self.hand_control(who);
        }
    }

    /// A move switched off for the AI, or on again (the menu's "Switch").
    fn switch_move(&mut self, who: MonsterId, slot: usize) {
        let Some(switched) = self.here().monster_mut(who).and_then(|monster| monster.moves.get_mut(slot)) else {
            return;
        };
        switched.off = !switched.off;
        let (id, off) = (switched.id, switched.off);
        let name = self.book.move_name(id);
        self.log.push(format!("{name} is switched {}.", if off { "off" } else { "on" }));
    }

    /// That member is the one the player moves; the one moved till then
    /// follows on its own. Each keeps its own belly: the leader's is the one
    /// that empties.
    pub(crate) fn hand_control(&mut self, to: MonsterId) {
        let dungeon = self.here();
        let now = dungeon.leader().id;
        if now == to || dungeon.monster(to).is_none_or(|monster| !monster.team()) {
            return;
        }
        for monster in &mut dungeon.monsters {
            if monster.id == now {
                monster.side = Side::Ally;
            } else if monster.id == to {
                monster.side = Side::Leader;
                monster.belly.get_or_insert_with(crate::dungeon::Belly::full);
            }
        }
        dungeon.trail = None;
        let name = dungeon.leader().name.clone();
        self.log.push(format!("You now lead {name}."));
    }

    /// The hero is the one moved again, if it is not.
    pub(crate) fn hero_controlled(&mut self) {
        if let Some(hero) = self.dungeon.as_ref().and_then(|dungeon| dungeon.hero).filter(|hero| self.here().monster(*hero).is_some_and(Monster::alive)) {
            self.hand_control(hero);
        }
    }

    /// A skill the member has switched off, or on again (`iq_off`), at once.
    fn toggle_iq(&mut self, who: MonsterId, skill: u8) {
        let Some(monster) = self.here().monster(who) else {
            return;
        };
        let name = monster.name.clone();
        let skill_name = self.book.tables.iq_skills.get(skill as usize).cloned().unwrap_or_default();
        if iq::ALWAYS_ON.contains(&skill) {
            self.log.push(format!("{name}: {skill_name} can't be turned off."));
            return;
        }
        let bit = 1u128 << skill;
        let off = self.team.progress.iq_off.entry(name.clone()).or_default();
        *off ^= bit;
        let now_off = *off & bit != 0;
        self.log.push(format!("{name}: {skill_name} {}.", if now_off { "off" } else { "on" }));
        self.keep_iq();
    }

    /// The skills a team member has learnt, switched on or not.
    pub fn iq_learnt(&self, monster: &super::Monster) -> u128 {
        self.book.tables.iq.skills(monster.stats.iq_group, monster.stats.iq)
    }

    /// Whether that member has switched that skill off.
    pub fn iq_is_off(&self, name: &str, skill: u8) -> bool {
        self.team.progress.iq_off.get(name).is_some_and(|off| off & (1u128 << skill) != 0)
    }

    fn set_tactic(&mut self, who: MonsterId, tactic: u8) {
        let Some(monster) = self.here().monster(who) else {
            return;
        };
        let (name, level) = (monster.name.clone(), monster.stats.level);
        if super::tactic::unlocked(level).contains(&tactic) {
            self.log.push(format!("{name} changed tactics."));
            self.team.progress.tactics.insert(name, tactic);
        }
    }

    /// Links a move to the one before, or delinks it, with a Link Box in
    /// the bag (as the original's); in town, Electivire's Link Shop does it.
    fn link(&mut self, who: MonsterId, at: usize) {
        const LINK_BOX: u16 = 362;
        if !self.bag.items.iter().any(|&lying| stack::id(lying) == LINK_BOX) {
            self.log.push("A Link Box is needed to link moves here.".to_string());
            return;
        }
        let Some(monster) = self.here().monster_mut(who) else {
            return;
        };
        let moves = &mut monster.moves;
        if at == 0 || at >= moves.len() {
            return;
        }
        let linking = !moves[at].linked;
        if linking && (moves[at].pp <= 0 || moves[at - 1].pp <= 0) {
            self.log.push("A move with no PP cannot be linked.".to_string());
            return;
        }
        moves[at].linked = linking;
        let what = if linking { "is linked to the one before" } else { "is unlinked" };
        self.log.push(format!("Move {} {what}.", at + 1));
    }
}
