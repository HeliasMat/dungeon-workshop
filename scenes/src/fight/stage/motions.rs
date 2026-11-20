//! Who moves for a blow, and what shows of it.

use super::*;

impl Staging {
    /// A blow pushes someone, who comes back to where it stood: blows do
    /// not drive it across the field.
    pub(crate) fn pushed(&mut self, delay: i32, who: Who, by: Who, far: i32) {
        let room = match (self.spot(who), self.spot(by)) {
            (Some(at), Some(from)) => self.ground.room_behind(at, from, far as f32) as i32,
            _ => far,
        };
        add!(self, "after", delay, "recoil", who, by, room);
        if let Some(at) = self.spot(who) {
            add!(self, "after", delay + PUSHED_FOR, "go", who, at.x as i32, at.y as i32, PUSHED_FOR, 1);
        }
    }

    /// A great move lights the screen, by its type, or shakes the ground.
    pub(crate) fn lights_up(&mut self, power: &Move, fares: u8, delay: i32) {
        if power.slot.power >= BIG && fares != matchup::IMMUNE {
            if let Some(colour) = lit(power.slot.type_id) {
                add!(self, "after", delay, "flash", colour, 8);
            }
            if SHAKING.contains(&power.slot.type_id) {
                add!(self, "after", delay, "quake", 4, 30);
            }
        }
    }

    /// How much a blow that takes something is felt.
    pub(crate) fn shakes(&mut self, target: Who, taken: i32, delay: i32, critical: bool) {
        add!(self, "after", delay, "shake", 0, (1 + taken / 20 + if critical { 2 } else { 0 }).min(6), 14);
        // (A great blow is felt: time nearly stands still.)
        let most = self.bars.get(&target).map_or(100, |bar| bar.most);
        if taken * 100 >= most * FELT_SHARE && !critical {
            add!(self, "after", delay, "tempo", FELT_SLOW);
            add!(self, "after", delay + FELT_FRAMES, "tempo", self.tempo_back());
        }
    }

    /// What only a critical blow has.
    pub(crate) fn critical_shows(&mut self, who: Who, delay: i32) {
        self.friends_shout(delay + 20, &WELL_DONE, "joy");
        let shout = self.pick(&THRILLED);
        add!(self, "after", delay, "shout", who, 50, shout);
        add!(self, "after", delay, "flash", "white", 6);
        add!(self, "after", delay, "sound", "shock");
        add!(self, "after", delay, "tempo", 0.35);
        add!(self, "after", delay + 5, "tempo", self.tempo_back());
        self.after_logged(delay, "A critical hit!".to_string());
    }

    /// A blow getting there: all that shows and is told of it.
    pub(crate) fn struck(&mut self, who: Who, landing: &Landing, power: &Move, strike: &Strike, delay: i32) {
        // (Said to be slowed: for as long as that, whatever else it does to time.)
        if let Some(frames) = self.slow_hit.take() {
            let back = self.tempo_back();
            add!(self, "after", delay, "tempo", FELT_SLOW);
            add!(self, "after", delay + frames, "tempo", back);
            self.usual_tempo = Some(FELT_SLOW);
            self.strikes(who, landing, power, strike, delay);
            self.usual_tempo = (back != 1.0).then_some(back);
            return;
        }
        self.strikes(who, landing, power, strike, delay);
    }

    /// What shows and is told of it.
    fn strikes(&mut self, who: Who, landing: &Landing, power: &Move, strike: &Strike, delay: i32) {
        let target = landing.target;
        let fares = landing.matchup;
        self.lights_up(power, fares, delay);
        let far = PUSHED.0 + (strike.taken / 3).min(PUSHED.1 - PUSHED.0);
        self.pushed(delay, target, who, far);
        add!(self, "after", delay, "blink", target, "on");
        add!(self, "after", delay + 14, "blink", target, "off");
        self.said(target, strike, delay + 2);
        self.shakes(target, strike.taken, delay, landing.critical);
        add!(self, "after", delay + 2, "popup", target, "yellow", format!("-{}", strike.taken));
        if let Some(said) = fares_said(fares) {
            self.after_logged(delay + 6, said.to_string());
            if self.quiet && fares != matchup::IMMUNE {
                add!(
                    self,
                    "after",
                    delay + 6,
                    "popup",
                    target,
                    if fares == matchup::SUPER_EFFECTIVE { "cyan" } else { "white" },
                    if fares == matchup::SUPER_EFFECTIVE { "Super effective!" } else { "Not very effective" }
                );
            }
        }
        if landing.critical || fares == matchup::SUPER_EFFECTIVE {
            self.cheer(delay + 6, "exclaim");
        }
        if fares == matchup::SUPER_EFFECTIVE && self.roll(2) == 0 {
            let shout = self.pick(&HURT);
            add!(self, "after", delay + 8, "shout", target, 50, shout);
        }
        if landing.critical {
            self.critical_shows(who, delay);
        }
    }

    /// Someone steps between a blow and who it was for.
    pub(crate) fn steps_between(&mut self, who: Who, target: Who, from: Who) {
        let (Some(start), Some(there), Some(blower)) = (self.spot(who), self.spot(target), self.spot(from)) else {
            return;
        };
        let way = (blower - there).normalize_or_zero();
        let to = self.ground.free(there + way * LUNGE * 1.5);
        self.stepped.insert(who, start);
        add!(self, "run", who, to.x as i32, to.y as i32);
        add!(self, "waitfor", who);
        add!(self, "face", who, from);
        self.at.insert(who, to);
        let text = format!("{} steps in!", self.named(who));
        self.logged(text);
    }

    /// Who stepped in goes back where it stood.
    pub(crate) fn steps_back(&mut self, who: Who) {
        if let Some(home) = self.stepped.remove(&who) {
            add!(self, "run", who, home.x as i32, home.y as i32);
            add!(self, "t", 20);
            self.at.insert(who, home);
        }
    }

    /// Someone gives its place to another: each goes where the other stood.
    pub(crate) fn swaps(&mut self, out: Who, comes: Who, left: i32, most: i32) {
        let (Some(leaving), Some(coming)) = (self.spot(out), self.spot(comes)) else {
            return;
        };
        add!(self, "status", out, "off");
        add!(self, "run", out, coming.x as i32, coming.y as i32);
        add!(self, "run", comes, leaving.x as i32, leaving.y as i32);
        add!(self, "waitfor", comes);
        add!(self, "waitfor", out);
        self.at.insert(out, coming);
        self.at.insert(comes, leaving);
        match self.bars.remove(&out) {
            Some(Bar { var, .. }) if !var.is_empty() => {
                add!(self, "set", var, left);
                let name = self.names.get(&comes).cloned().unwrap_or_default();
                let place = if var == "hp" { "left" } else { "right" };
                add!(self, "hud", place, format!("{name}  PV {{{var}}}/{most}"));
                add!(self, "gauge", place, var, most, if var == "hp" { "green" } else { "red" });
                self.bars.insert(comes, Bar { left, most, var });
            }
            _ => {
                add!(self, "bar", out, "off");
                add!(self, "bar", comes, format!("{:.2}", left as f32 / most.max(1) as f32));
                self.bars.insert(comes, Bar { left, most, var: "" });
            }
        }
        let text = format!("{} gives way to {}!", self.named(out), self.named(comes));
        self.logged(text);
        add!(self, "t", 30);
    }

    /// Those who cheer for who fights: one calls out, the others show it.
    pub(crate) fn friends_shout(&mut self, delay: i32, words: &[&'static str], emote: &str) {
        for (place, friend) in self.supporters.clone().into_iter().enumerate() {
            if place == 0 {
                let said = self.pick(words);
                add!(self, "after", delay, "shout", friend, 60, said);
            } else {
                add!(self, "after", delay + place as i32 * 6, "emote", friend, emote, 40);
            }
        }
    }

    /// A great move gathers itself first.
    pub(crate) fn gathers(&mut self, who: Who, power: &Move) {
        add!(self, "halo", who, 30, lit(power.slot.type_id).unwrap_or("white"));
        add!(self, "tremble", who, 30);
        let text = format!("{} is focusing...", self.named(who));
        self.logged(text);
        add!(self, "t", 36);
        add!(self, "halo", who, "off");
    }

    /// Those that leave the ground, and come back on the target.
    pub(crate) fn leaves_ground(&mut self, who: Who, id: u16, start: Option<Vec2>) {
        let flies = id == FLY;
        let said = if flies {
            "flies up"
        } else if id == DIVE {
            "dives"
        } else {
            "digs"
        };
        let text = format!("{} {said}!", self.named(who));
        self.logged(text);
        if flies {
            add!(self, "rise", who, 130, 22);
        } else {
            let at = start.unwrap_or_default();
            add!(self, "particles", "smoke", "@", at.x as i32, at.y as i32, 24, "grey");
            add!(self, "vanish", who, 12);
        }
        add!(self, "t", 46);
        if flies {
            add!(self, "rise", who, 0, 8);
        } else {
            add!(self, "appear", who, 6);
        }
        add!(self, "t", 10);
    }

    /// It runs up to its target. (When it is back from there, its blow
    /// struck.)
    pub(crate) fn rushes_up(&mut self, who: Who, start: Vec2, aim: Vec2) -> i32 {
        let far = start.distance(aim).max(1.0);
        let reach = (far - 24.0).max(0.0);
        add!(self, "emote", who, "dust", 20);
        let to = start + (aim - start) * reach / far;
        add!(self, "run", who, to.x as i32, to.y as i32);
        add!(self, "waitfor", who);
        26 + (reach / RUNS) as i32 + 8
    }

    /// It throws itself at its target, and comes back.
    pub(crate) fn lunges(&mut self, who: Who, target: Who, start: Vec2, aim: Vec2) {
        let far = start.distance(aim).max(1.0);
        let to = start + (aim - start) * LUNGE.min(far / 2.0) / far;
        add!(self, "hop", who, to.x as i32, to.y as i32, 5, 8);
        add!(self, "after", 20, "hop", who, start.x as i32, start.y as i32, 3, 10);
        add!(self, "after", 34, "face", who, target);
    }

    /// How long a blow takes to get there: what flies at the move's own
    /// speed.
    pub(crate) fn landing_of(&self, who: Who, target: Who, power: &Move) -> i32 {
        match (self.spot(who), self.spot(target)) {
            (Some(start), Some(aim)) if power.flies => HIT_FRAMES.max((start.distance(aim) / power.flight) as i32),
            _ => HIT_FRAMES,
        }
    }

    pub(crate) fn counted(&mut self, blow: &Blow, landing: &Landing) {
        if self.lead == Some(blow.who) {
            add!(self, "add", if landing.missed { "summary_misses" } else { "summary_blows" }, 1);
            if landing.critical {
                add!(self, "add", "summary_crits", 1);
            }
        }
    }

    pub(crate) fn misses(&mut self, one: Who, landing: i32) {
        let at = self.spot(one).unwrap_or_default();
        add!(self, "after", landing, "hop", one, at.x as i32, at.y as i32, 8, 12);
        add!(self, "after", landing + 2, "popup", one, "green", "Missed!");
        add!(self, "after", landing + 2, "sound", MISS_SOUNDS[usize::from(Some(one) == self.lead)]);
        self.after_logged(landing + 2, "But the attack missed!".to_string());
    }

    /// What the blow leaves it with.
    pub(crate) fn falls_ill(&mut self, one: Who, ailment: Ailment, landing: i32) {
        let (icon, said, sight) = match ailment {
            Ailment::Burnt => ("burn", "is burned", None),
            Ailment::Frozen => ("frozen", "is frozen", None),
            Ailment::Paralysed => ("cringe", "is paralyzed", Some(423)),
            Ailment::Asleep => ("sleep", "fell asleep", None),
            Ailment::Poisoned => ("poison", "is poisoned", None),
            Ailment::Confused => ("confused", "is confused", Some(324)),
        };
        self.ailing.insert(one, ailment);
        add!(self, "after", landing + 14, "status", one, icon);
        match ailment {
            Ailment::Frozen => add!(self, "after", landing + 14, "colour", one, FROZEN_COLOUR[0], FROZEN_COLOUR[1], FROZEN_COLOUR[2], 12),
            Ailment::Asleep => add!(self, "after", landing + 14, "pose", one, "sleep"),
            _ => {}
        }
        if let Some(sight) = sight {
            add!(self, "after", landing + 14, "effect", one, sight);
        }
        let text = format!("{} {said}!", self.named(one));
        self.after_logged(30, text);
    }

    pub(crate) fn looks_well(&mut self, who: Who, ailment: Ailment) {
        self.ailing.remove(&who);
        add!(self, "status", who, "off");
        match ailment {
            Ailment::Frozen => add!(self, "uncolour", who, 12),
            Ailment::Asleep => add!(self, "pose", who, "idle"),
            _ => {}
        }
    }

    /// A stat that changes, as moves have it.
    pub(crate) fn stat(&mut self, who: Who, stat: Stat, up: bool) {
        for (at, effect) in stat_sights(stat, up).iter().enumerate() {
            add!(self, "after", at as i32 * STAT_SIGHT_FRAMES, "effect", who, effect);
        }
        let name = stat_name(stat);
        add!(self, "popup", who, if up { "yellow" } else { "blue" }, format!("{name} {}", if up { "+" } else { "-" }));
        let text = format!("{}'s {name} {}!", self.named(who), if up { "rose" } else { "fell" });
        self.told(text);
    }
}
