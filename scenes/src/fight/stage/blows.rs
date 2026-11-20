//! A move shown.

use super::*;

impl Staging {
    /// The blow, on one of those it is for.
    pub(crate) fn lands_on(&mut self, blow: &Blow, landing: &Landing, at: i32) {
        self.counted(blow, landing);
        if landing.missed {
            self.misses(landing.target, at);
            return;
        }
        if landing.blocked {
            self.shield_takes_it(landing.target, at);
            return;
        }
        if landing.matchup == matchup::IMMUNE || landing.strikes.is_empty() {
            add!(self, "after", at, "popup", landing.target, "white", "No effect");
            self.after_logged(at, "It has no effect...".to_string());
            return;
        }
        for (place, strike) in landing.strikes.iter().enumerate() {
            self.struck(blow.who, landing, &blow.power, strike, at + place as i32 * 14);
        }
        if landing.strikes.len() > 1 {
            let text = format!("Hit {} times!", landing.strikes.len());
            self.after_logged(at + 10, text);
        }
        self.past_the_blow(blow);
        if let Some(ailment) = landing.ailment {
            self.falls_ill(landing.target, ailment, at);
        }
    }

    /// The blow stops on a shield, which goes.
    pub(crate) fn shield_takes_it(&mut self, target: Who, delay: i32) {
        add!(self, "after", delay, "popup", target, "cyan", "Blocked!");
        add!(self, "after", delay, "status", target, "off");
        add!(self, "after", delay, "sound", "shock");
        let text = format!("{} protects itself!", self.named(target));
        self.after_logged(delay, text);
    }

    /// A blow that strikes, from who gets ready to who falls.
    pub(crate) fn blow(&mut self, blow: &Blow) {
        let (who, wait, target) = (blow.who, !blow.beside, blow.landings[0].target);
        let id = blow.power.slot.id;
        let (start, aim) = (self.spot(who), self.spot(target));
        // (A blow that touches: it runs up to its target, and back. Not
        // through what stands between them: then, from where it is.)
        let rushes = wait && blow.power.touches() && matches!((start, aim), (Some(start), Some(aim)) if self.ground.clear(start, aim));
        add!(self, "face", who, target);
        let missed = blow.landings.iter().all(|landing| landing.missed);
        if blow.power.slot.power >= GATHERED && wait && !missed {
            self.gathers(who, &blow.power);
        }
        if [FLY, DIVE, DIG].contains(&id) && wait {
            self.leaves_ground(who, id, start);
        }
        if id == SOLAR_BEAM && wait {
            add!(self, "emote", who, "joy", 40);
            add!(self, "t", 30);
        }
        let back = match (rushes, start, aim) {
            (true, Some(start), Some(aim)) => self.rushes_up(who, start, aim),
            _ => 0,
        };
        let text = format!("{} used [CS:M]{}[CR]!", self.named(who), blow.power.name);
        self.logged(text);
        add!(self, "move", who, id, target);
        match (rushes, start, aim) {
            (true, Some(start), Some(_)) => {
                add!(self, "after", 26, "run", who, start.x as i32, start.y as i32);
                add!(self, "after", back, "face", who, target);
            }
            (false, Some(start), Some(aim)) => self.lunges(who, target, start, aim),
            _ => {}
        }
        let landing = self.landing_of(who, target, &blow.power);
        for one in &blow.landings {
            self.lands_on(blow, one, landing);
        }
        for (stat, up) in &blow.stats {
            self.stat(who, *stat, *up);
        }
        if wait {
            // (Long enough for who ran to be back.)
            let strikes = blow.landings.iter().map(|one| one.strikes.len()).max().unwrap_or(1).max(1) as i32;
            add!(self, "t", (landing + 34 + (strikes - 1) * 14).max(back + 6));
        }
    }

    /// A move that does not strike.
    pub(crate) fn deed(&mut self, deed: &Deed) {
        let (who, id) = (deed.who, deed.power.slot.id);
        let text = format!("{} used [CS:M]{}[CR]!", self.named(who), deed.power.name);
        self.logged(text);
        match &deed.landing {
            Some(landing) => add!(self, "move", who, id, landing.target),
            None => add!(self, "move", who, id),
        }
        if let Some(healed) = &deed.healed {
            self.healed(who, healed, 16);
            add!(self, "t", 50);
            return;
        }
        for (stat, up) in &deed.stats {
            self.stat(who, *stat, *up);
        }
        if let Some(landing) = &deed.landing {
            if landing.missed {
                self.misses(landing.target, HIT_FRAMES);
            } else if landing.blocked {
                self.shield_takes_it(landing.target, HIT_FRAMES);
            }
            if let Some(ailment) = landing.ailment {
                self.falls_ill(landing.target, ailment, HIT_FRAMES);
            }
            for (stat, up) in &landing.stats {
                self.stat(landing.target, *stat, *up);
            }
        }
        add!(self, "t", 50);
    }
}
