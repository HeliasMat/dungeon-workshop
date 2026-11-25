//! Wonder orbs: what each does, on the foes of the room, the team, the
//! floor.

use crate::dungeon::{FIXED_ONE, FLOOR_HEIGHT, FLOOR_WIDTH, Floor, MoveEffect, Pos, Tile, ability, ai, apply_effect, pick_weighted, stack, status};

use crate::services::BAG_CAPACITY;

use super::attack::{Attack, chebyshev};
use super::dungeon::{FloorItem, FloorTrap, Outcome, Shove, weather_name};
use super::monster::Side;
use super::words::effect_message;
use super::{MonsterId, Run, Sight};

/// What an orb does (by the item numbers of the game data; what each
/// does as the games' item descriptions say).
#[derive(Clone, Copy)]
pub(crate) enum Orb {
    /// To each foe of the leader's room (or, from a hallway, close by).
    Foes(MoveEffect),
    /// The foes of the room (Petrify), or of the floor (Foe-Hold), cannot
    /// act until struck.
    Petrify {
        floor: bool,
    },
    /// To each of the team.
    Team(MoveEffect),
    Weather(u8),
    Warp,
    Stairs,
    Rollcall,
    Luminous,
    SeeTrap,
    Trapbust,
    Escape,
    /// Its move, on each foe of the room (Rocky, Lob, Sizebust).
    Strike,
    /// The first foe in line ahead, up to ten tiles: struck and robbed of
    /// what it holds (Mug), made to cower (Shocker), muzzled (Silence).
    Mug,
    Shocker,
    /// The foe in front: blown away, swapped with, sent to the stairs,
    /// knocked out, turned into an item.
    Blowback,
    Switcher,
    Stayaway,
    OneShot,
    Itemizer,
    /// The foes of the room warped at random.
    Spurn,
    /// The foes of the room at 1 HP, the leader at half its own.
    TwoEdge,
    Silence,
    /// Every item of the floor brought to the leader.
    Trawl,
    /// Where the items, or the foes, are.
    Scanner,
    Radar,
    /// The leader leaps ahead up to what stops it.
    Pounce,
    /// The floor's water and lava dried up, what is in front filled in,
    /// every wall down.
    Drought,
    FillIn,
    OneRoom,
    /// The foes of the room flee in terror.
    FoeFear,
    /// The team's blows are critical hits.
    AllHit,
    /// The leader turns invisible, walks through walls, throws far,
    /// throws through.
    Invisify,
    Mobile,
    Longtoss,
    Pierce,
    /// Tells which foes hold what.
    Identify,
    /// Lays a trap of the floor's under the leader, for the foes.
    Trapper,
    /// The leader sends status moves back, takes what the foes' moves do
    /// for themselves.
    Rebound,
    Snatch,
    /// The foe in front becomes a decoy the others attack.
    Decoy,
    /// The foe in front is hurled at another foe, or lands further away.
    Hurl,
    /// The foe in front turns into another monster of the floor.
    Transfer,
    /// The team's items are sticky no more.
    Cleanse,
}

impl Orb {
    pub(crate) fn of(item: u16) -> Option<Self> {
        use crate::dungeon::weather::{HAIL, RAIN, SANDSTORM, SUNNY};
        Some(match item {
            301 => Orb::Weather(HAIL),
            302 => Orb::Weather(SUNNY),
            303 => Orb::Weather(RAIN),
            304 => Orb::Team(MoveEffect::UserEvasion(1)),
            305 => Orb::Weather(SANDSTORM),
            308 => Orb::SeeTrap,
            314 => Orb::Warp,
            316 => Orb::Foes(MoveEffect::TargetSpeedDown),
            317 => Orb::Team(MoveEffect::UserSpeedUp),
            318 => Orb::Luminous,
            319 => Orb::Petrify { floor: false },
            326 => Orb::Foes(MoveEffect::Sleep),
            327 => Orb::Foes(MoveEffect::Confuse),
            330 => Orb::Escape,
            334 => Orb::Trapbust,
            335 => Orb::Rollcall,
            350 => Orb::Stairs,
            // (By the moves the game gives them, and their reach: Foe-Hold
            // petrifies the floor, Foe-Seal pauses the room, Mug, Shocker
            // and Silence go at the first foe in line.)
            355 => Orb::Petrify { floor: true },
            359 => Orb::Foes(MoveEffect::Pause),
            306 | 311 | 341 => Orb::Strike,
            309 => Orb::Mug,
            340 => Orb::Shocker,
            312 => Orb::Switcher,
            313 => Orb::Blowback,
            320 => Orb::Stayaway,
            321 => Orb::Pounce,
            322 => Orb::Trawl,
            328 => Orb::TwoEdge,
            329 => Orb::Silence,
            331 => Orb::Scanner,
            332 => Orb::Radar,
            337 => Orb::OneShot,
            346 => Orb::Itemizer,
            354 => Orb::Spurn,
            356 => Orb::Team(MoveEffect::UserSpeedUp),
            333 => Orb::Drought,
            357 => Orb::FoeFear,
            336 => Orb::Invisify,
            348 => Orb::Mobile,
            351 => Orb::Longtoss,
            352 => Orb::Pierce,
            338 => Orb::Identify,
            344 => Orb::Trapper,
            310 => Orb::Rebound,
            307 => Orb::Snatch,
            325 => Orb::Decoy,
            347 => Orb::Hurl,
            315 => Orb::Transfer,
            323 => Orb::Cleanse,
            358 => Orb::AllHit,
            342 => Orb::OneRoom,
            343 => Orb::FillIn,
            _ => return None,
        })
    }
}

/// The sound of turning invisible.
const INVISIBLE_SOUND: u16 = 0x21C;

/// Who an orb works with: the leader, the foes standing, those of its room.
struct Scene {
    me: MonsterId,
    name: String,
    here: Pos,
    facing: (i32, i32),
    /// The tiles anyone stands on.
    busy: Vec<Pos>,
    room: Vec<MonsterId>,
    team: Vec<MonsterId>,
    /// The foe the leader faces.
    front: Option<MonsterId>,
}

/// On the leader's room, or from a hallway, close by.
fn same_room(floor: &Floor, here: Pos, tile: Pos) -> bool {
    match floor.tile(here) {
        Tile::Floor { room: Some(room) } => floor.tile(tile) == (Tile::Floor { room: Some(room) }),
        _ => chebyshev(here, tile) <= 2,
    }
}

/// Ground where the terrain was, of the room beside it if any.
fn ground(floor: &Floor, tile: Pos) -> Tile {
    let room = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().find_map(|&(dx, dy)| match floor.tile(Pos::new(tile.x + dx, tile.y + dy)) {
        Tile::Floor { room: Some(room) } => Some(room),
        _ => None,
    });
    Tile::Floor { room }
}

impl Run {
    fn scene(&self) -> Option<Scene> {
        let dungeon = self.dungeon.as_ref()?;
        let leader = dungeon.leader();
        let foes = self.foes();
        let front = Pos::new(leader.at.x + leader.facing.0, leader.at.y + leader.facing.1);
        Some(Scene {
            me: leader.id,
            name: leader.name.clone(),
            here: leader.at,
            facing: leader.facing,
            busy: dungeon.monsters.iter().filter(|monster| monster.alive()).map(|monster| monster.at).collect(),
            room: foes.iter().filter(|foe| same_room(&dungeon.floor, leader.at, foe.1)).map(|foe| foe.0).collect(),
            team: dungeon.monsters.iter().filter(|monster| monster.team() && monster.alive()).map(|monster| monster.id).collect(),
            front: foes.iter().find(|foe| foe.1 == front).map(|foe| foe.0),
        })
    }

    /// A move's effect, on one monster, told.
    fn orb_effect(&mut self, who: MonsterId, effect: MoveEffect) {
        self.work(who, |monster, rng, book, log| {
            let applied = apply_effect(effect, rng, &book.tables.constants, &mut monster.stats, &mut monster.status);
            log.push(effect_message(book, effect, &monster.name, applied));
        });
    }

    /// Moves a monster.
    fn send(&mut self, who: MonsterId, to: Pos) {
        if let Some(monster) = self.dungeon.as_mut().and_then(|dungeon| dungeon.monster_mut(who)) {
            monster.at = to;
        }
    }

    /// A free tile beside the stairs.
    fn by_stairs(&self, busy: &[Pos]) -> Option<Pos> {
        let floor = &self.dungeon.as_ref()?.floor;
        let stairs = floor.stairs;
        ai::DIRECTIONS.iter().map(|&(dx, dy)| Pos::new(stairs.x + dx, stairs.y + dy)).find(|&tile| floor.tile(tile).is_walkable() && !busy.contains(&tile))
    }

    /// As far as it goes from `from`, the way the leader faces, within so
    /// many steps.
    fn furthest(&self, from: Pos, facing: (i32, i32), steps: i32, busy: &[Pos]) -> Pos {
        let mut at = from;
        let Some(floor) = self.dungeon.as_ref().map(|dungeon| &dungeon.floor) else {
            return at;
        };
        for _ in 0..steps {
            let next = Pos::new(at.x + facing.0, at.y + facing.1);
            if !floor.tile(next).is_walkable() || busy.contains(&next) {
                break;
            }
            at = next;
        }
        at
    }

    /// Whether the turn was spent. (An Escape Orb ends the run instead.)
    pub(crate) fn use_orb(&mut self, index: usize, orb: Orb, item: u16) -> bool {
        let Some(scene) = self.scene() else {
            return false;
        };
        // (Some floors laid out by hand have no use for them.)
        if self.dungeon.as_ref().is_some_and(|dungeon| dungeon.no_orbs) {
            self.log.push("Orbs have no effect on this floor.".to_string());
            return false;
        }
        self.bag.items.remove(index);
        self.sights.push(Sight::BagDone);
        self.log.push(format!("{} used {}.", scene.name, self.book.item_name(item)));
        if let Orb::Escape = orb {
            self.log.push("The team escaped from the dungeon!".to_string());
            if let Some(dungeon) = self.dungeon.as_mut() {
                dungeon.outcome = Some(Outcome::Escaped);
            }
            return false;
        }
        match orb {
            Orb::Foes(_) | Orb::Petrify { .. } | Orb::Team(_) | Orb::Spurn | Orb::TwoEdge | Orb::FoeFear | Orb::AllHit | Orb::Strike | Orb::Identify => {
                self.on_many(orb, &scene, item)
            }
            Orb::Mug | Orb::Shocker | Orb::Silence => self.on_line(orb, &scene, item),
            Orb::Blowback | Orb::Switcher | Orb::Stayaway | Orb::OneShot | Orb::Itemizer | Orb::Decoy | Orb::Transfer | Orb::Hurl => self.on_front(orb, &scene),
            Orb::Drought | Orb::FillIn | Orb::OneRoom => self.reshape(orb, &scene),
            _ => self.on_floor(orb, &scene),
        }
        let at = self.dungeon.as_ref().map_or(scene.here, |dungeon| dungeon.leader().at);
        self.sights.push(Sight::Item(item, at));
        true
    }

    /// Orbs on all the foes of the room, or the team.
    fn on_many(&mut self, orb: Orb, scene: &Scene, item: u16) {
        match orb {
            Orb::Foes(effect) => {
                for &foe in &scene.room {
                    self.orb_effect(foe, effect);
                }
                if scene.room.is_empty() {
                    self.log.push("But there is nobody.".to_string());
                }
            }
            Orb::Petrify { floor } => {
                let foes: Vec<MonsterId> = if floor { self.foes().iter().map(|foe| foe.0).collect() } else { scene.room.clone() };
                for foe in foes {
                    self.work(foe, |foe, _, _, log| {
                        foe.status.sleep_turns = status::DEEP_SLEEP;
                        log.push(format!("{} is petrified!", foe.name));
                    });
                }
            }
            Orb::Team(effect) => {
                for &member in &scene.team {
                    self.orb_effect(member, effect);
                }
            }
            Orb::Spurn => {
                for &foe in &scene.room {
                    self.warp(foe, &scene.busy);
                }
            }
            Orb::TwoEdge => {
                for &foe in &scene.room {
                    self.work(foe, |foe, _, _, log| {
                        foe.stats.hp = 1;
                        log.push(format!("{} has only 1 HP left!", foe.name));
                    });
                }
                self.work(scene.me, |leader, _, _, log| {
                    leader.stats.hp = (leader.stats.hp / 2).max(1);
                    log.push(format!("{} lost half of its HP.", leader.name));
                });
            }
            Orb::FoeFear => {
                // (How long is this project's own figure.)
                const TERROR_TURNS: i32 = 10;
                for &foe in &scene.room {
                    self.work(foe, |foe, _, _, log| {
                        foe.status.terrified_turns = TERROR_TURNS;
                        log.push(format!("{} is terrified!", foe.name));
                    });
                }
                if scene.room.is_empty() {
                    self.log.push("But there is nobody.".to_string());
                }
            }
            Orb::AllHit => {
                for &member in &scene.team {
                    self.work(member, |member, _, _, _| member.status.focused = true);
                }
                self.log.push("The team is focused: its blows will be critical!".to_string());
            }
            Orb::Strike => {
                // The orb's move, from the leader, on each foe of the room.
                let slot = self.book.move_slot(self.book.item_move(item)).filter(|slot| slot.power > 0);
                let Some(slot) = slot else {
                    self.log.push("But nothing happened.".to_string());
                    return;
                };
                let attack = Attack { slot, effect: None, damage_mult: FIXED_ONE, linked: false };
                for &foe in &scene.room {
                    self.strike(scene.me, foe, &attack);
                    self.say_if_out(foe);
                }
            }
            Orb::Identify => {
                let holding: Vec<String> = self
                    .dungeon
                    .iter()
                    .flat_map(|dungeon| &dungeon.monsters)
                    .filter(|foe| foe.side == Side::Foe && foe.alive() && foe.stats.held != 0)
                    .map(|foe| format!("{} holds {}.", foe.name, self.book.item_name(foe.stats.held)))
                    .collect();
                if holding.is_empty() {
                    self.log.push("No foe holds an item.".to_string());
                }
                holding.into_iter().for_each(|said| self.log.push(said));
            }
            _ => {}
        }
    }

    /// Orbs on the first foe in line ahead, up to ten tiles (the reach of
    /// the moves the game gives them).
    fn on_line(&mut self, orb: Orb, scene: &Scene, item: u16) {
        let foes = self.foes();
        let target = self.dungeon.as_ref().and_then(|dungeon| {
            let tiles: Vec<Pos> = foes.iter().map(|foe| foe.1).collect();
            let reached = crate::dungeon::targets_in_range(&dungeon.floor, scene.here, scene.facing, crate::dungeon::range::FRONT_10, &tiles);
            foes.iter().find(|foe| reached.contains(&foe.1)).map(|foe| foe.0)
        });
        let Some(foe) = target else {
            self.log.push("But there is nobody.".to_string());
            return;
        };
        match orb {
            // Struck by the orb's move, then robbed: what it held goes to the
            // bag, if the leader holds nothing and the bag has room (Sticky
            // Hold keeps it).
            Orb::Mug => {
                if let Some(slot) = self.book.move_slot(self.book.item_move(item)) {
                    let attack = Attack { slot, effect: None, damage_mult: FIXED_ONE, linked: false };
                    self.strike(scene.me, foe, &attack);
                }
                let free = self.dungeon.as_ref().is_some_and(|dungeon| dungeon.leader().stats.held == 0) && self.bag.items.len() < BAG_CAPACITY;
                let taken = self
                    .work(foe, |foe, _, _, _| match foe.stats.held != 0 && !ability::has(&foe.stats, ability::STICKY_HOLD) && free {
                        true => Some((std::mem::take(&mut foe.stats.held), foe.name.clone())),
                        false => None,
                    })
                    .flatten();
                if let Some((held, name)) = taken {
                    self.bag.items.push(held);
                    self.log.push(format!("{} stole {} from {name}!", scene.name, self.book.item_name(held)));
                }
                self.say_if_out(foe);
            }
            // Cowering: its moves go behind it (the game's 10 to 11 turns,
            // and the one it starts on).
            Orb::Shocker => {
                self.work(foe, |foe, rng, _, log| {
                    foe.status.cowering_turns = rng.rand_range(10, 12) + 1;
                    log.push(format!("{} is cowering!", foe.name));
                });
            }
            // Muzzled: no moves for a while (the game's 10 turns).
            _ => {
                const MUZZLED_TURNS: i32 = 10;
                self.work(foe, |foe, _, _, log| {
                    foe.status.muzzled_turns = MUZZLED_TURNS + 1;
                    log.push(format!("{} is muzzled: it can't use moves!", foe.name));
                });
            }
        }
    }

    /// Orbs on the foe in front.
    fn on_front(&mut self, orb: Orb, scene: &Scene) {
        let Some(foe) = scene.front else {
            self.log.push("But there is nobody in front.".to_string());
            return;
        };
        let name = self.name_of(foe);
        let front = Pos::new(scene.here.x + scene.facing.0, scene.here.y + scene.facing.1);
        // (Suction Cups: neither is moved.)
        let anchored = |run: &Self, who: MonsterId| {
            run.dungeon.as_ref().and_then(|dungeon| dungeon.monster(who)).is_some_and(|monster| ability::has(&monster.stats, ability::SUCTION_CUPS))
        };
        let stuck = match orb {
            Orb::Blowback if anchored(self, foe) => Some((foe, "be blown away")),
            Orb::Switcher if anchored(self, foe) => Some((foe, "switch places")),
            Orb::Switcher if anchored(self, scene.me) => Some((scene.me, "switch places")),
            _ => None,
        };
        if let Some((who, what)) = stuck {
            let name = self.name_of(who);
            self.log.push(format!("{name} is anchored! It can't {what}!"));
            return;
        }
        match orb {
            Orb::Blowback => {
                // Up to ten tiles, until something stops it.
                let at = self.furthest(front, scene.facing, 10, &scene.busy);
                let damage = self.book.tables.constants.blowback_damage;
                self.change(foe, |foe| {
                    foe.at = at;
                    foe.stats.hp -= damage;
                });
                self.log.push(format!("{name} was blown away!"));
            }
            Orb::Decoy => {
                self.work(foe, |foe, rng, book, log| {
                    apply_effect(MoveEffect::Decoy, rng, &book.tables.constants, &mut foe.stats, &mut foe.status);
                    log.push(format!("{} became a Decoy!", foe.name));
                });
            }
            Orb::Switcher => {
                self.send(foe, scene.here);
                self.send(scene.me, front);
                self.log.push(format!("{} swapped places with {name}!", scene.name));
            }
            Orb::Stayaway => {
                if let Some(tile) = self.by_stairs(&scene.busy) {
                    self.send(foe, tile);
                    self.log.push(format!("{name} was sent near the stairs!"));
                }
            }
            Orb::OneShot => {
                self.change(foe, |foe| foe.stats.hp = 0);
                self.log.push(format!("{name} fainted in one blow!"));
            }
            Orb::Itemizer => {
                // The foe becomes an item of the floor.
                self.change(foe, |foe| foe.stats.hp = 0);
                let book = self.book.clone();
                if let Some(dungeon) = self.dungeon.as_mut() {
                    let spec = dungeon.spec(&book);
                    if let Some(made) = spec.and_then(|spec| spec.items.pick(&mut dungeon.rng)) {
                        dungeon.items.push(FloorItem { at: front, item: made, price: None });
                        self.log.push(format!("{name} turned into {}!", book.item_name(made)));
                    }
                }
            }
            Orb::Transfer => {
                if let Some(dungeon) = self.dungeon.as_mut() {
                    dungeon.transfer = Some(front);
                }
            }
            Orb::Hurl => {
                // (As Strength: at another foe in range, else onto a free
                // tile some way off.)
                if let Some(dungeon) = self.dungeon.as_mut() {
                    dungeon.shoves.push(Shove { effect: MoveEffect::Hurl, from: scene.here, facing: scene.facing, team: true });
                }
            }
            _ => {}
        }
    }

    /// The floor's water and lava, and its walls.
    fn reshape(&mut self, orb: Orb, scene: &Scene) {
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let floor = &mut dungeon.floor;
        let mut changed = 0;
        match orb {
            Orb::Drought => {
                let wet: Vec<Pos> = Floor::positions().filter(|&tile| floor.tile(tile) == Tile::Secondary).collect();
                for tile in wet {
                    let dry = ground(floor, tile);
                    floor.set(tile, dry);
                    changed += 1;
                }
            }
            Orb::FillIn => {
                let mut at = scene.here;
                for _ in 0..10 {
                    at = Pos::new(at.x + scene.facing.0, at.y + scene.facing.1);
                    if floor.tile(at) != Tile::Secondary {
                        break;
                    }
                    let dry = ground(floor, at);
                    floor.set(at, dry);
                    changed += 1;
                }
            }
            _ => {
                // (The floor's edge stays.)
                let room = match floor.tile(scene.here) {
                    Tile::Floor { room: Some(room) } => Some(room),
                    _ => Some(0),
                };
                for tile in Floor::positions() {
                    let inside = tile.x > 0 && tile.y > 0 && tile.x < FLOOR_WIDTH - 1 && tile.y < FLOOR_HEIGHT - 1;
                    if inside && matches!(floor.tile(tile), Tile::Wall | Tile::Floor { .. } | Tile::Secondary) {
                        floor.set(tile, Tile::Floor { room });
                        changed += 1;
                    }
                }
            }
        }
        dungeon.repaints += 1;
        self.sights.push(Sight::TilesChanged);
        self.log.push(match changed {
            0 => "But nothing happened.".to_string(),
            _ => "The terrain changed!".to_string(),
        });
    }

    /// Orbs on the floor, the leader, the bag.
    fn on_floor(&mut self, orb: Orb, scene: &Scene) {
        match orb {
            Orb::Weather(_) | Orb::Warp | Orb::Stairs | Orb::Rollcall | Orb::Pounce => self.moving_orb(orb, scene),
            Orb::Luminous | Orb::SeeTrap | Orb::Trapbust | Orb::Silence | Orb::Trawl | Orb::Scanner | Orb::Radar | Orb::Trapper => self.floor_orb(orb, scene),
            Orb::Invisify | Orb::Mobile | Orb::Longtoss | Orb::Pierce | Orb::Cleanse | Orb::Rebound | Orb::Snatch => self.leader_orb(orb, scene),
            _ => {}
        }
    }

    /// The orbs that change the weather or send the team about.
    fn moving_orb(&mut self, orb: Orb, scene: &Scene) {
        let (here, me) = (scene.here, scene.me);
        match orb {
            Orb::Weather(kind) => {
                let turns = self.book.tables.constants.weather_move_turns;
                let Some(dungeon) = self.dungeon.as_mut() else {
                    return;
                };
                dungeon.weather.set(kind, turns);
                self.log.push(format!("The weather changed: {}.", weather_name(dungeon.weather.current)));
            }
            Orb::Warp => self.warp(me, &scene.busy),
            Orb::Stairs => {
                if let Some(tile) = self.by_stairs(&scene.busy) {
                    self.send(me, tile);
                    self.log.push(format!("{} was warped near the stairs!", scene.name));
                }
            }
            Orb::Rollcall => self.rollcall(scene),
            Orb::Pounce
                if self.dungeon.as_ref().and_then(|dungeon| dungeon.monster(me)).is_some_and(|monster| ability::has(&monster.stats, ability::SUCTION_CUPS)) =>
            {
                self.log.push(format!("{} is anchored! It can't leap!", scene.name));
            }
            Orb::Pounce => {
                let at = self.furthest(here, scene.facing, FLOOR_WIDTH, &scene.busy);
                self.send(me, at);
                self.log.push(format!("{} leaps!", scene.name));
            }
            _ => {}
        }
    }

    /// The team called around the leader, on the free tiles beside it.
    fn rollcall(&mut self, scene: &Scene) {
        let (here, me) = (scene.here, scene.me);
        let Some(dungeon) = self.dungeon.as_ref() else {
            return;
        };
        let floor = &dungeon.floor;
        let free: Vec<Pos> = ai::DIRECTIONS
            .iter()
            .map(|&(dx, dy)| Pos::new(here.x + dx, here.y + dy))
            .filter(|&tile| floor.tile(tile).is_walkable() && !scene.busy.contains(&tile))
            .collect();
        for (&member, tile) in scene.team.iter().filter(|&&member| member != me).zip(free) {
            self.send(member, tile);
            self.log.push(format!("{} joined {}!", self.name_of(member), scene.name));
        }
    }

    /// The orbs that show, clear or set what lies on the floor.
    fn floor_orb(&mut self, orb: Orb, scene: &Scene) {
        let here = scene.here;
        match orb {
            Orb::Luminous => {
                if let Some(dungeon) = self.dungeon.as_mut() {
                    dungeon.lit = true;
                    dungeon.explored.reveal_all();
                }
                self.log.push("The floor is lit up!".to_string());
            }
            Orb::SeeTrap => self.see_traps(),
            Orb::Trapbust => {
                let Some(dungeon) = self.dungeon.as_mut() else {
                    return;
                };
                let before = dungeon.traps.len();
                let floor = &dungeon.floor;
                dungeon.traps.retain(|trap| !same_room(floor, here, trap.at));
                self.log.push(format!("{} traps were destroyed.", before - dungeon.traps.len()));
            }
            Orb::Trawl => self.trawl(scene),
            Orb::Scanner => {
                if let Some(dungeon) = self.dungeon.as_mut() {
                    for item in &dungeon.items {
                        dungeon.explored.reveal_around(item.at);
                    }
                }
                self.log.push("The floor's items appear on the map!".to_string());
            }
            Orb::Radar => {
                if let Some(dungeon) = self.dungeon.as_mut() {
                    dungeon.radar = true;
                }
                self.log.push("The floor's Pokémon appear on the map!".to_string());
            }
            Orb::Trapper => self.set_trap(here),
            _ => {}
        }
    }

    /// The floor's items drawn near the leader.
    fn trawl(&mut self, scene: &Scene) {
        let here = scene.here;
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let floor = &dungeon.floor;
        let mut free =
            Floor::positions().filter(|&tile| chebyshev(tile, here) <= 3 && tile != here && floor.tile(tile).is_walkable() && !scene.busy.contains(&tile));
        let mut brought = 0;
        // (Not what Kecleon sells, nor what lies close already.)
        for item in dungeon.items.iter_mut().filter(|item| item.price.is_none() && chebyshev(item.at, here) > 3) {
            let Some(tile) = free.next() else {
                break;
            };
            item.at = tile;
            brought += 1;
        }
        self.log.push(format!("{brought} items were drawn to {}!", scene.name));
    }

    /// One of the floor's traps set where the leader stands, for the team.
    fn set_trap(&mut self, here: Pos) {
        let book = self.book.clone();
        let Some(dungeon) = self.dungeon.as_mut() else {
            return;
        };
        let kinds = dungeon.spec(&book).map_or(&[][..], |spec| &spec.traps[..]);
        let kind = pick_weighted(&mut dungeon.rng, kinds.iter().copied().enumerate());
        match kind {
            Some(kind) if !dungeon.traps.iter().any(|trap| trap.at == here) => {
                dungeon.traps.push(FloorTrap { at: here, kind, seen: true, team: true });
                self.log.push("A trap was set!".to_string());
            }
            _ => self.log.push("But nothing happened.".to_string()),
        }
    }

    /// The orbs that work on the leader, the team or the bag.
    fn leader_orb(&mut self, orb: Orb, scene: &Scene) {
        let me = scene.me;
        match orb {
            Orb::Invisify => {
                self.orb_effect(me, MoveEffect::Invisify);
                self.sights.push(Sight::Sound(INVISIBLE_SOUND));
            }
            Orb::Mobile => {
                self.work(me, |leader, _, _, log| {
                    leader.status.mobile = true;
                    log.push(format!("{} can go anywhere!", leader.name));
                });
            }
            Orb::Longtoss => {
                self.work(me, |leader, _, _, log| {
                    leader.status.long_toss = true;
                    log.push(format!("{} will throw items farther!", leader.name));
                });
            }
            Orb::Pierce => {
                self.work(me, |leader, _, _, log| {
                    leader.status.piercing = true;
                    log.push(format!("Items thrown by {} will pierce through!", leader.name));
                });
            }
            Orb::Cleanse => self.cleanse(scene),
            Orb::Rebound | Orb::Snatch => {
                let effect = if matches!(orb, Orb::Rebound) { MoveEffect::MagicCoat } else { MoveEffect::Snatch };
                self.orb_effect(me, effect);
            }
            _ => {}
        }
    }

    /// The bag's and the team's items no longer sticky.
    fn cleanse(&mut self, scene: &Scene) {
        let mut cleaned = self.bag.items.iter().filter(|&&item| stack::sticky(item)).count();
        self.bag.items.iter_mut().for_each(|item| *item = stack::stuck(*item, false));
        for &member in &scene.team {
            let was = self.work(member, |member, _, _, _| {
                let was = stack::sticky(member.stats.held);
                member.stats.held = stack::stuck(member.stats.held, false);
                was
            });
            cleaned += (was == Some(true)) as usize;
        }
        self.log.push(match cleaned {
            0 => "But nothing happened.".to_string(),
            _ => "Items are no longer sticky!".to_string(),
        });
    }
}
