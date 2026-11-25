//! A blow on the floor: `attack::resolve` plays it out, and what happened
//! is told, shown and left on the floor here.

use crate::attack::{self, Attack, Field, Happened, Outcome, Party, Role, Tuning};
use crate::dungeon::MoveEffect;

use super::attack::Arena;
use super::dungeon::Shove;
use super::words::{effect_message, log_aftermath, log_damage};
use super::{Monster, Sight};

/// Frames between an attack starting and its blow, by eye. (Each monster's
/// sprite may know better.)
pub(crate) const BLOW_LANDS: u32 = 10;

fn party(monster: &mut Monster) -> Party<'_> {
    Party { team: monster.team(), at: monster.at, id: monster.id.0, stats: &mut monster.stats, status: &mut monster.status }
}

impl Arena<'_> {
    /// Lands an attack. Whether it did not miss.
    pub(crate) fn land(&mut self, user: &mut Monster, victim: &mut Monster, attack: &Attack) -> bool {
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
        };
        let Outcome { landed, happened } = attack::resolve(&mut party(user), &mut party(victim), attack, Tuning::default(), &mut field);
        for happened in happened {
            self.tell(happened, user, victim);
        }
        landed
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
                return log_damage(book, self.log, other, damage);
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
        };
        self.log.push(said);
    }
}
