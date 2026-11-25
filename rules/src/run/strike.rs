//! A blow on the floor: `attack::resolve` plays it out, and what happened
//! is told, shown and left on the floor here.

use crate::attack::{self, Attack, Field, Happened, Outcome, Party, Role, Sides, Tuning};
use crate::dungeon::{MoveEffect, REGULAR_ATTACK_ID, ability, held, matchup, type_name};

use super::attack::Arena;
use super::dungeon::Shove;
use super::monster::Side;
use super::team::earn;
use super::words::{effect_message, log_aftermath, log_damage};
use super::{Monster, MonsterId, Sight};

/// What of its striker's worth a Joy Ribbon's holder earns when hurt (a
/// tenth: this project's own figure).
const JOY_RIBBON_SHARE: i32 = 10;

/// Frames between an attack starting and its blow, by eye. (Each monster's
/// sprite may know better.)
pub(crate) const BLOW_LANDS: u32 = 10;

fn party(monster: &mut Monster) -> Party<'_> {
    Party {
        team: monster.team(),
        leader: monster.side == Side::Leader,
        at: monster.at,
        id: monster.id.0,
        stats: &mut monster.stats,
        status: &mut monster.status,
    }
}

impl Arena<'_> {
    /// What the two have of their teammates and species, for the
    /// abilities that ask (all the floor's monsters in hand).
    pub(crate) fn sides(&self, monsters: &[Monster], user: MonsterId, victim: MonsterId) -> Sides {
        let of = |who: MonsterId| monsters.iter().find(|monster| monster.id == who);
        let allies = |who: MonsterId| {
            let team = of(who).is_some_and(|monster| monster.team());
            ability::allied(monsters.iter().filter(|monster| monster.id != who && monster.alive() && monster.team() == team).map(|monster| &monster.stats))
        };
        let gender =
            |who: MonsterId| of(who).and_then(|monster| monster.species).and_then(|species| self.book.species(species)).map_or(0, |species| species.gender);
        Sides { allies: (allies(user), allies(victim)), genders: (gender(user), gender(victim)) }
    }

    /// Lands an attack. Whether it did not miss.
    pub(crate) fn land(&mut self, user: &mut Monster, victim: &mut Monster, attack: &Attack, sides: Sides) -> bool {
        let book = self.book;
        let dungeon = &mut *self.dungeon;
        let secret_power = dungeon.spec(book).map_or(9, |floor| floor.secret_power);
        let mut field = Field {
            rng: &mut dungeon.rng,
            constants: &book.tables.constants,
            chart: book.tables.types.as_ref(),
            moves: &book.tables.move_slots,
            weather: dungeon.weather.apparent(),
            gravity: dungeon.gravity == Some(dungeon.generation),
            mud_sport: dungeon.mud_sport > 0,
            water_sport: dungeon.water_sport > 0,
            secret_power,
            bonds: &mut dungeon.bonds,
            sides,
        };
        let Outcome { landed, happened } = attack::resolve(&mut party(user), &mut party(victim), attack, Tuning::default(), &mut field);
        for happened in happened {
            self.tell(happened, user, victim);
        }
        if landed {
            self.after_landing(user, victim, attack);
        }
        landed
    }

    /// The abilities a landed move wakes on the floor: Pressure spends a
    /// PP more of a foe's move; Frisk and Anticipation tell what the foe a
    /// team member strikes holds or knows; Illuminate calls a wild monster.
    fn after_landing(&mut self, user: &mut Monster, victim: &Monster, attack: &Attack) {
        let book = self.book;
        let regular = attack.slot.id == REGULAR_ATTACK_ID;
        let foes = user.team() != victim.team();
        if !regular && foes && !user.status.pressured && ability::shields(&user.stats, &victim.stats, ability::PRESSURE) {
            let used = user.status.last_move;
            if let Some(slot) = user.moves.iter_mut().find(|slot| Some(slot.id) == used && slot.pp > 0) {
                slot.pp -= 1;
                user.status.pressured = true;
            }
        }
        let (one, other) = (&user.name, &victim.name);
        if regular && user.team() && foes && victim.alive() {
            if ability::has(&user.stats, ability::FRISK) && victim.stats.held != 0 {
                self.log.push(format!("{other} was frisked! It's holding one {}!", book.item_name(victim.stats.held)));
            }
            let chart = book.tables.types.as_ref();
            let dreaded = victim
                .moves
                .iter()
                .any(|slot| slot.power > 0 && chart.is_some_and(|chart| chart.overall(slot.type_id, &user.stats) == matchup::SUPER_EFFECTIVE));
            if ability::has(&user.stats, ability::ANTICIPATION) && dreaded {
                self.log.push(format!("{one}'s Anticipation: {other} knows a super-effective move!"));
            }
        }
        if ability::has(&victim.stats, ability::ILLUMINATE) {
            self.dungeon.illuminated = true;
        }
    }

    /// What happened, in the log, in sights and on the floor.
    fn tell(&mut self, happened: Happened, user: &mut Monster, victim: &mut Monster) {
        use Happened::*;
        let book = self.book;
        let (one, other) = (&user.name, &victim.name);
        let said = match happened {
            Deaf => format!("{other} hears nothing thanks to its ability!"),
            Missed => {
                self.sights.push(Sight::Miss(victim.id));
                book.phrase(crate::strings::log_text::MISSED, &[("string:1", other)]).unwrap_or_else(|| format!("{one} missed {other}."))
            }
            Failed => "But it failed!".to_string(),
            Bound => format!("{one} and {other} are bound by destiny!"),
            StatsSwapped => format!("{one} and {other} swapped their stat changes."),
            ItemsSwapped => format!("{one} and {other} swapped their items."),
            Mimics(id) => format!("{one} mimicked {}!", book.move_name(id)),
            AbilityCopied => format!("{one} copied {other}'s ability!"),
            AbilitiesSwapped => format!("{one} and {other} swapped their abilities!"),
            AbilityLost => format!("{other} lost its ability!"),
            AilmentsPassed => format!("{one} passed its status problems to {other}!"),
            LifeShared => format!("{one} and {other} shared their HP."),
            StatsCopied => format!("{one} copied {other}'s stat changes!"),
            Bounced => format!("{other} bounced the move back!"),
            Effect { on, effect, applied } => effect_message(book, effect, if on == Role::User { one } else { other }, applied),
            Cast(effect) => return self.changes_floor(effect),
            // (Only on a stage.)
            Blast { .. } => return,
            NotAsleep => format!("{other} is not asleep: it has no effect."),
            Unshielded => format!("{other} is no longer protected!"),
            Shattered => "The barrier was shattered!".to_string(),
            Crashed(lost) => format!("{one} crashed and lost {lost} HP!"),
            Stood => format!("{other} held on thanks to its ability!"),
            Absorbed => format!("{other} absorbed the attack!"),
            AbsorbedLife(healed) => format!("{other} absorbed the move and recovered {healed} HP!"),
            AbsorbedSpeed => format!("{other} absorbed the move and sped up!"),
            Unafraid => format!("{other} is not afraid of the move!"),
            Hurt(damage) => {
                self.sights.push(Sight::Hurt(victim.id, BLOW_LANDS));
                log_damage(book, self.log, other, damage);
                return self.joy_ribbon(user, victim, damage.amount);
            }
            AttackTopped => format!("{other}'s Attack is maxed out!"),
            RageGrows => format!("{other}'s rage is building!"),
            Hurled { facing } => {
                self.dungeon.shoves.push(Shove { effect: MoveEffect::Hurl, from: victim.at, facing, team: victim.team() });
                format!("{other} threw {one}!")
            }
            Drops(item) => return self.dungeon.drops.push((victim.at, item)),
            Ate(item) => format!("{one} ate {other}'s {}!", book.item_name(item)),
            Rests => format!("{one} must rest."),
            Woken => book.woke_up(other),
            Unparalyzed => format!("{other} is no longer paralyzed!"),
            SpunFree => format!("{one} broke free!"),
            TypeChanged => format!("{other}'s type changed!"),
            Stole(item) => format!("{one} stole {} from {other}!", book.item_name(item)),
            Returned(back) => format!("{other} gave {back} HP back to {one}!"),
            Contact(hurt) => format!("{one} was hurt by touching {other} and lost {hurt} HP."),
            Ooze(lost) => format!("{one} sucked up the liquid ooze and lost {lost} HP!"),
            Aftermath(lost) => return log_aftermath(self.log, one, lost),
            Countered(back) => format!("{other} struck back: {one} lost {back} HP."),
            Grudge => format!("{other}'s grudge strikes {one}!"),
            Passed(effect) => {
                self.dungeon.passed.push((victim.at, effect));
                format!("{other} passed the effect on with its Pass Scarf!")
            }
            ColorChanged(kind) => format!("{other}'s Color Change made it the {} type!", type_name(kind)),
            Traced => format!("{other} traced and copied the target's ability!"),
            Synchronized { on, effect } => {
                let bearer = if on == Role::User { &*user } else { &*victim };
                self.dungeon.synchronized.push((bearer.at, bearer.team(), effect));
                format!("{}'s Synchronize took effect!", bearer.name)
            }
            LeafGuarded(on) => format!("{} is protected by its Leaf Guard!", if on == Role::User { one } else { other }),
            Stench => format!("A horrid stench billowed out! {one} became terrified!"),
            Burst => {
                if self.dungeon.weather.damp {
                    "But the damp prevents the explosion!".to_string()
                } else {
                    self.dungeon.blasts.push((victim.at, 1, Some(victim.at)));
                    format!("{other} exploded with its Aftermath!")
                }
            }
        };
        self.log.push(said);
    }

    /// A Joy Ribbon: its holder of the team, hurt and standing, earns
    /// experience from the blow.
    fn joy_ribbon(&mut self, user: &Monster, victim: &mut Monster, lost: i32) {
        let ribbon = lost > 0 && victim.alive() && victim.team() && held::holds(&victim.stats, held::JOY_RIBBON);
        if !ribbon || !self.dungeon.rules.enemies_give_exp {
            return;
        }
        let amount = (user.exp_value / JOY_RIBBON_SHARE).max(1);
        let Monster { name, stats, moves, experience, id, .. } = victim;
        let Some(experience) = experience.as_mut() else {
            return;
        };
        let told = amount.to_string();
        let book = self.book;
        self.log.push(
            book.phrase(crate::strings::log_text::GAINED_EXP, &[("string:0", name), ("digits_c:0", &told)])
                .unwrap_or_else(|| format!("{name} gained {amount} Exp. Points.")),
        );
        let asked = earn(book, self.log, name, stats, moves, experience, amount);
        self.dungeon.learning.extend(asked.into_iter().map(|slot| (*id, slot)));
    }
}
