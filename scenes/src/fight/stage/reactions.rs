//! Who looks on, and who is hurt.

use super::*;

impl Staging {
    /// Those who watch the fight react, a few of them.
    pub(crate) fn cheer(&mut self, delay: i32, emote: &str) {
        let crowd: Vec<Who> = self.public.iter().copied().filter(|who| !self.bars.contains_key(who) && self.at.contains_key(who)).collect();
        self.cheered += 1;
        for &who in crowd.iter().skip(self.cheered % 3).step_by((crowd.len() / CHEERING).max(1)) {
            let at = delay + self.roll(12);
            add!(self, "after", at, "emote", who, emote, 36);
            add!(self, "after", at, "jump", who, 8, 10);
        }
    }

    /// Someone moves about, on the ground, still facing the other.
    pub(crate) fn steps_aside(&mut self, who: Who, other: Who) {
        let at = self.spot(who).unwrap_or_default();
        let (x, y) = (self.roll(STEP * 2 + 1) - STEP, self.roll(STEP * 2 + 1) - STEP);
        let goal = at + Vec2::new(x as f32, y as f32);
        let them = self.spot(other).unwrap_or(goal);
        if (self.ground.known() && self.ground.blocked(goal, 1)) || goal.distance(them) < (2 * STEP) as f32 {
            return;
        }
        add!(self, "hop", who, goal.x as i32, goal.y as i32, 3, 10);
        self.at.insert(who, goal);
        add!(self, "after", 12, "face", who, other);
    }

    /// Someone gets some of its life back, or loses it.
    pub(crate) fn healed(&mut self, who: Who, strike: &Strike, delay: i32) {
        let shown = format!("{:+}", -strike.taken);
        add!(self, "after", delay, "popup", who, if strike.taken < 0 { "green" } else { "yellow" }, shown);
        self.said(who, strike, delay);
    }

    /// What the log says of a blow; the bar of the one it is about goes
    /// down, little by little.
    pub(crate) fn said(&mut self, who: Who, strike: &Strike, delay: i32) {
        if strike.taken > 0 {
            let text = format!("{} loses {} HP!", self.named(who), strike.taken);
            self.after_logged(delay, text);
        }
        let Some(kept) = self.bars.get(&who).cloned() else {
            return;
        };
        let left = strike.left;
        // (The one who is to win holds on, and comes back stronger.)
        if strike.held {
            let text = format!("{} hangs on!", self.named(who));
            self.after_logged(delay + 20, text);
            add!(self, "after", delay + 20, "emote", who, "anger", 50);
            add!(self, "after", delay + 20, "tremble", who, 30);
        } else if left > 0 && left * 100 < kept.most * LOW_SHARE && kept.left * 100 >= kept.most * LOW_SHARE {
            // (Danger: it shows.)
            add!(self, "after", delay + 34, "emote", who, "sweat", 50);
            add!(self, "after", delay + 34, "popup", who, "orange", "In danger!");
            self.friends_shout(delay + 44, &HOLD_ON, "sweat");
        }
        // (The last blow: slowed, and white.)
        if left == 0 {
            add!(self, "after", delay, "tempo", 0.3);
            add!(self, "after", delay, "flash", "white", 14);
            add!(self, "after", delay, "quake", 5, 30);
            add!(self, "after", delay + 10, "tempo", self.tempo_back());
        }
        let change = left - kept.left;
        let steps = change.abs().min(BAR_STEPS);
        if !kept.var.is_empty() {
            for step in 0..steps {
                let part = change * (step + 1) / steps - change * step / steps;
                add!(self, "after", delay + step * 2, "add", kept.var, part);
            }
        }
        if let Some(bar) = self.bars.get_mut(&who) {
            bar.left = left;
        }
        if self.little_bars && kept.var.is_empty() {
            add!(self, "after", delay, "bar", who, format!("{:.2}", left as f32 / kept.most.max(1) as f32));
        }
    }

    /// What a move does past its blow.
    pub(crate) fn past_the_blow(&mut self, blow: &Blow) {
        let who = blow.who;
        if let Some(drained) = &blow.drained {
            self.healed(who, drained, 30);
            let text = format!("{} absorbs energy!", self.named(who));
            self.after_logged(30, text);
        }
        if let Some(recoil) = &blow.recoil {
            self.healed(who, recoil, 30);
        }
        if let Some(blast) = &blow.blast {
            add!(self, "after", 16, "flash", "white", 20);
            add!(self, "after", 16, "quake", 7, 40);
            self.healed(who, blast, 20);
        }
    }

    /// Its seed brings someone back as it falls.
    pub(crate) fn reborn(&mut self, who: Who, healed: &Strike) {
        let at = self.spot(who).unwrap_or_default();
        add!(self, "pose", who, "hurt");
        add!(self, "tremble", who, 24);
        add!(self, "t", 30);
        add!(self, "flash", "green", 10);
        add!(self, "particles", "healing", "@", at.x as i32, at.y as i32, 30, "green");
        add!(self, "status", who, "off");
        add!(self, "pose", who, "idle");
        add!(self, "jump", who, 12, 12);
        add!(self, "sound", "startle");
        let text = format!("{}'s [CS:I]Reviver Seed[CR] takes effect!", self.named(who));
        self.logged(text);
        add!(self, "t", 40);
        self.healed(who, healed, 0);
        add!(self, "t", 40);
    }
}
