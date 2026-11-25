//! An attack as everyone makes it, the leader as the others: a chain of
//! linked moves, each one begun, shown and landed on all it reaches. How a
//! monster picks its move is its turn's.

use crate::dungeon::exclusive::effect::NO_FRIENDLY_FIRE;
use crate::dungeon::{MoveEffect, Pos, ability, iq, targets_in_range};

use super::attack::{link_chain, unlink_spent};
use super::dungeon::ATTACK_WAIT;
use super::{Attack, MonsterId, Run, Side, Sight};

/// How far a monster is blown back.
const BLOWBACK_REACH: i32 = 10;

impl Run {
    /// The frames an attack is given to be seen.
    pub(crate) fn attack_wait(&self) -> u32 {
        if self.unseen { 0 } else { ATTACK_WAIT }
    }

    /// Who stands alive, wherever.
    pub(crate) fn standing(&mut self) -> Vec<Pos> {
        self.here().monsters.iter().filter(|monster| monster.alive()).map(|monster| monster.at).collect()
    }

    /// A monster attacks the way it faces: with the move chosen (`None`:
    /// the regular attack), and those linked to it if `chained`.
    /// `unleashed`: the move it charged; `struggles`: out of PP, it
    /// struggles; `decoy`: one of its own side it strikes as a foe.
    pub(crate) fn use_chain(&mut self, user: MonsterId, chosen: Option<usize>, unleashed: bool, chained: bool, struggles: bool, decoy: Option<MonsterId>) {
        let dungeon = self.here();
        dungeon.missed = None;
        let Some(monster) = dungeon.monster(user) else {
            return;
        };
        let side = monster.side;
        // A move linked to others: the whole chain, one after the other.
        let chain: Vec<Option<usize>> = match chosen {
            Some(index) if chained && !unleashed => link_chain(&monster.moves, index).into_iter().map(Some).collect(),
            other => vec![other],
        };
        let linked = chain.len() > 1;
        // (A chain costs the leader belly, the longer the more.)
        if linked && side == Side::Leader {
            let costs = self.book.tables.constants.link_belly;
            self.here().chain_belly += costs.get(chain.len()).or(costs.last()).copied().unwrap_or(0);
        }
        for index in chain {
            let Some(monster) = self.here().monster(user) else {
                return;
            };
            if !monster.alive() {
                break;
            }
            let front = Pos::new(monster.at.x + monster.facing.0, monster.at.y + monster.facing.1);
            self.here().aimed_at = Some(front);
            if let Some(attack) = self.begin_attack(user, index, unleashed, struggles) {
                self.land_attack(user, Attack { linked, ..attack }, decoy);
            }
        }
        // Quick Striker: a regular attack strikes again; Extra Striker, now
        // and then (a chance in 100, this project's own figure).
        const EXTRA_STRIKER_CHANCE: i32 = 25;
        let again = chosen.is_none() && !unleashed && {
            let dungeon = self.here();
            let stats = dungeon.monster(user).filter(|monster| monster.alive()).map(|monster| monster.stats);
            stats.is_some_and(|stats| {
                iq::has(&stats, iq::QUICK_STRIKER) || (iq::has(&stats, iq::EXTRA_STRIKER) && dungeon.rng.rand_int(100) < EXTRA_STRIKER_CHANCE)
            })
        };
        if again && let Some(attack) = self.begin_attack(user, None, false, struggles) {
            self.land_attack(user, attack, decoy);
        }
        // Wary Fighter: having missed, it steps back from its target.
        if let Some((wary, from)) = self.here().missed.take().filter(|&(wary, _)| wary == user) {
            self.step_back(wary, from);
        }
        // Out of PP, a move comes off its chain.
        let Some(monster) = self.here().monster_mut(user) else {
            return;
        };
        if unlink_spent(&mut monster.moves) {
            let name = monster.name.clone();
            match side {
                Side::Leader => self.log.push("The linked moves come apart!".to_string()),
                Side::Ally => self.log.push(format!("{name}'s linked moves come apart!")),
                _ => {}
            }
        }
    }

    /// A step away from that tile, if there is room.
    fn step_back(&mut self, who: MonsterId, from: Pos) {
        let book = self.book.clone();
        let standing = self.standing();
        let dungeon = self.here();
        let secondary_kind = dungeon.spec(&book).map_or(0, |floor| floor.secondary_kind);
        let Some(monster) = dungeon.monster(who).filter(|monster| monster.alive() && !monster.status.rooted()) else {
            return;
        };
        let (at, walks) = (monster.at, super::leader::mobility(&monster.stats, &monster.status, secondary_kind));
        let (dx, dy) = ((at.x - from.x).signum(), (at.y - from.y).signum());
        let to = Pos::new(at.x + dx, at.y + dy);
        if (dx, dy) == (0, 0) || !dungeon.floor.can_step_as(at, dx, dy, walks) || standing.contains(&to) {
            return;
        }
        if let Some(monster) = dungeon.monster_mut(who) {
            monster.at = to;
            let name = monster.name.clone();
            self.log.push(format!("{name} stepped back warily."));
        }
    }

    /// Bodyguard: a team member in a pinch is struck in its stead by one
    /// next to it that has it.
    fn guarded(&mut self, user: MonsterId, victim: MonsterId) -> MonsterId {
        let dungeon = self.here();
        let Some(struck) = dungeon.monster(victim).filter(|monster| monster.team() && ability::in_a_pinch(&monster.stats)) else {
            return victim;
        };
        let guard = dungeon.monsters.iter().find(|monster| {
            monster.team()
                && monster.alive()
                && monster.id != victim
                && monster.id != user
                && iq::has(&monster.stats, iq::BODYGUARD)
                && crate::dungeon::floor::chebyshev(monster.at, struck.at) <= 1
        });
        let Some((guard, name)) = guard.map(|guard| (guard.id, guard.name.clone())) else {
            return victim;
        };
        let struck = struck.name.clone();
        self.log.push(format!("{name} stepped in to protect {struck}!"));
        guard
    }

    /// What a move begun does: on its user's side, or on those it reaches.
    fn land_attack(&mut self, user: MonsterId, attack: Attack, decoy: Option<MonsterId>) {
        let wait = self.attack_wait();
        let dungeon = self.here();
        dungeon.wait = wait;
        let Some(monster) = dungeon.monster(user) else {
            return;
        };
        let (from, facing, side) = (monster.at, monster.facing, monster.side);
        let spared = monster.status.exclusive.has(NO_FRIENDLY_FIRE);
        // (Confused without Nontraitor, its blows may strike its own side;
        // cowering, they go behind it, at whoever stands there.)
        let cowering = monster.status.cowering_turns > 0;
        let facing = if cowering { (-facing.0, -facing.1) } else { facing };
        let traitor = super::others::traitor(monster) || cowering;
        self.show(Sight::Strike(user));
        if let Some(effect) = attack.effect.filter(|effect| effect.on_user()) {
            self.show(Sight::Move { move_id: attack.slot.id, user, from, targets: Vec::new() });
            // What a foe does for itself, a leader on watch takes for itself.
            let leader = self.here().leader();
            let (snatcher, snatcher_name, snatches) = (leader.id, leader.name.clone(), leader.status.snatching_turns > 0);
            if side == Side::Foe && snatches && effect != MoveEffect::Warp {
                self.log.push(format!("{snatcher_name} snatched the effect!"));
                self.self_effect(snatcher, effect);
                return;
            }
            self.team_move(user, effect, Some(attack.slot));
            if effect == MoveEffect::Warp {
                let taken = self.standing();
                let dungeon = self.here();
                if let Some(tile) = dungeon.warp_tile(&taken)
                    && let Some(monster) = dungeon.monster_mut(user)
                {
                    monster.at = tile;
                }
            }
            return;
        }
        // Those it may strike: the other side, and of its own those who
        // pass for foes (decoys), or all of them for a move at everyone,
        // unless an exclusive item spares them.
        let friendly_fire = attack.slot.hits_allies() && !spared;
        let dungeon = self.here();
        let strikable: Vec<(MonsterId, Pos)> = dungeon
            .monsters
            .iter()
            .filter(|monster| monster.alive() && monster.id != user && monster.side != Side::Keeper)
            .filter(|monster| match (side, monster.team()) {
                (Side::Foe, team) => team || Some(monster.id) == decoy || traitor,
                (_, false) => true,
                (_, true) => monster.status.decoy || friendly_fire || traitor,
            })
            .map(|monster| (monster.id, monster.at))
            .collect();
        let tiles: Vec<Pos> = strikable.iter().map(|(_, at)| *at).collect();
        let targets = targets_in_range(&dungeon.floor, from, facing, attack.slot.range, &tiles);
        self.show(Sight::Move { move_id: attack.slot.id, user, from, targets: targets.clone() });
        // (The leader's regular attack needs nothing to swing at.)
        if targets.is_empty() && (side != Side::Leader || attack.slot.id != self.book.regular_attack().id) {
            self.log.push("But there is no target.".to_string());
        }
        for (victim, at) in strikable.into_iter().filter(|(_, at)| targets.contains(at)) {
            let victim = if side == Side::Foe { self.guarded(user, victim) } else { victim };
            let dungeon = self.here();
            let user_alive = dungeon.monster(user).is_some_and(|monster| monster.alive());
            if !user_alive || !dungeon.monster(victim).is_some_and(|monster| monster.alive()) {
                continue;
            }
            let landed = self.strike(user, victim, &attack);
            if attack.effect == Some(MoveEffect::Blowback) && landed {
                self.blow_back(victim, at, facing);
            }
            let Some(monster) = self.here().monster(victim) else {
                continue;
            };
            if !monster.alive() && !monster.team() {
                let name = monster.name.clone();
                self.log.push(self.book.knocked_out(&name));
            }
        }
    }

    /// Blown back up to ten tiles away; an obstacle on the way hurts.
    pub(crate) fn blow_back(&mut self, victim: MonsterId, from: Pos, (dx, dy): (i32, i32)) {
        let standing = self.standing();
        let dungeon = self.here();
        if !dungeon.monster(victim).is_some_and(|monster| monster.alive()) {
            return;
        }
        let mut at = from;
        let mut crashed = false;
        for _ in 0..BLOWBACK_REACH {
            let next = Pos::new(at.x + dx, at.y + dy);
            if !dungeon.floor.can_step(at, dx, dy) || standing.contains(&next) {
                crashed = true;
                break;
            }
            at = next;
        }
        let Some(monster) = dungeon.monster_mut(victim) else {
            return;
        };
        // (Suction Cups: it stays where it is.)
        if ability::has(&monster.stats, ability::SUCTION_CUPS) {
            let name = monster.name.clone();
            self.log.push(format!("{name} is anchored! It can't be blown away!"));
            return;
        }
        monster.at = at;
        if crashed {
            let name = monster.name.clone();
            let damage = self.book.tables.constants.blowback_damage;
            self.hurt(victim, damage);
            self.log.push(format!("{name} hit an obstacle and lost {damage} HP."));
        }
    }
}
