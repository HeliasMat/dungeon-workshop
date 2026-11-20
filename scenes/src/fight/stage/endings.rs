//! What else happens: berries, shields, falls, a win.

use super::*;

impl Staging {
    pub(crate) fn berry(&mut self, who: Who, healed: &Strike) {
        self.gets_better(who, "Oran Berry", healed);
    }

    /// A thing in someone's hands, as a film has it.
    fn takes_out(&mut self, who: Who, thing: Who, name: &str) {
        add!(self, "putnear", thing, who, "down");
        add!(self, "thing", thing, name);
        add!(self, "hold", thing, who);
    }

    /// Bites, the thing smaller at each; then it does its good.
    pub(crate) fn eats(&mut self, who: Who, thing: Who, name: &str, healed: &Strike) {
        self.takes_out(who, thing, name);
        add!(self, "anim", who, "Eat");
        for left in ["0.75", "0.5", "0.25"] {
            add!(self, "t", 14);
            add!(self, "scale", thing, left);
            add!(self, "sound", "click");
        }
        add!(self, "t", 8);
        add!(self, "gone", thing);
        add!(self, "pose", who, "idle");
        self.gets_better(who, name, healed);
    }

    /// A thing flies, in an arc, and hits.
    pub(crate) fn flings(&mut self, who: Who, thing: Who, name: &str, target: Who, strike: &Strike) {
        self.takes_out(who, thing, name);
        add!(self, "t", 10);
        add!(self, "face", who, target);
        add!(self, "hold", thing, "off");
        add!(self, "throw", thing, target, 24, 26);
        add!(self, "t", 26);
        add!(self, "gone", thing);
        add!(self, "tremble", target, 14);
        add!(self, "emote", target, "exclaim", 30);
        let text = format!("{} throws [CS:I]{name}[CR] at {}!", self.named(who), self.named(target));
        self.told(text);
        self.healed(target, strike, 0);
        add!(self, "t", 40);
    }

    fn gets_better(&mut self, who: Who, name: &str, healed: &Strike) {
        let at = self.spot(who).unwrap_or_default();
        add!(self, "emote", who, "joy", 40);
        add!(self, "particles", "healing", "@", at.x as i32, at.y as i32, 30, "green");
        add!(self, "popup", who, "green", format!("+{}", -healed.taken));
        let text = format!("{} eats [CS:I]{name}[CR]!", self.named(who));
        self.told(text);
        self.said(who, healed, 2);
        add!(self, "t", 50);
    }

    pub(crate) fn shields(&mut self, who: Who) {
        add!(self, "move", who, PROTECT);
        add!(self, "after", 8, "status", who, "protect");
        let text = format!("{} gets ready to protect itself!", self.named(who));
        self.logged(text);
        add!(self, "t", 30);
    }

    /// Frozen or asleep, its turn goes by; it is over after that.
    pub(crate) fn stopped(&mut self, who: Who, ailment: Ailment) {
        add!(self, "tremble", who, 30);
        let said = match ailment {
            Ailment::Frozen => "is frozen and cannot act!",
            Ailment::Asleep => "is asleep and cannot act!",
            _ => "is paralyzed and cannot act!",
        };
        let text = format!("{} {said}", self.named(who));
        self.logged(text);
        add!(self, "t", 60);
        if ailment != Ailment::Paralysed {
            self.looks_well(who, ailment);
        }
    }

    /// A little of its life goes.
    pub(crate) fn worn(&mut self, who: Who, strike: &Strike, sky: Option<u8>) {
        add!(self, "popup", who, "yellow", format!("-{}", strike.taken));
        if let Some(sky) = sky {
            add!(self, "tremble", who, 14);
            let said = if sky == rules::dungeon::weather::HAIL { "is buffeted by the hail" } else { "is caught in the sandstorm" };
            let text = format!("{} {said}!", self.named(who));
            self.logged(text);
        }
        self.said(who, strike, 2);
        add!(self, "t", 30);
    }

    /// It falls, the view still. (Its own fall, if its
    /// sprite has one: else asleep. A frame later: a pose asked at the same
    /// moment would hide it.)
    pub(crate) fn falls(&mut self, who: Who, more: bool) {
        self.flown.push(who);
        if let Some(ailment) = self.ailing.get(&who).copied() {
            self.looks_well(who, ailment);
        }
        add!(self, "status", who, "off");
        add!(self, "t", 20);
        add!(self, "pose", who, "hurt");
        add!(self, "tremble", who, 20);
        add!(self, "t", 24);
        add!(self, "pose", who, "sleep");
        add!(self, "emote", who, "sweat", 40);
        add!(self, "t", 2);
        add!(self, "anim", who, "Faint");
        // (It hits the ground.)
        add!(self, "after", 8, "emote", who, "dust", 24);
        add!(self, "after", 8, "sound", "shock");
        add!(self, "after", 8, "quake", 1, 10);
        let text = format!("{} fainted!", self.named(who));
        self.told(text);
        if more {
            // (Who fell is carried off the field.)
            add!(self, "t", 30);
            add!(self, "vanish", who, 24);
            add!(self, "t", 24);
        }
    }

    /// The next of a team takes the place of who fell, and its bar.
    pub(crate) fn enters(&mut self, side: usize, who: Who, left: i32, most: i32) {
        let goal = self.slots[side];
        add!(self, "run", who, goal.x as i32, goal.y as i32);
        add!(self, "waitfor", who);
        self.at.insert(who, goal);
        self.hud(side, who, left, most);
        self.fighting[side] = Some(who);
        if let [Some(first), Some(second)] = self.fighting {
            add!(self, "frame", FRAMING, first, second);
        }
        add!(self, "jump", who, 8, 10);
        let text = format!("{} enters the fight!", self.named(who));
        self.logged(text);
        add!(self, "t", 30);
    }

    /// Glad, a little jump and another.
    pub(crate) fn joy(&mut self, who: Who) {
        add!(self, "emote", who, "joy", 50);
        add!(self, "jump", who, 10, 12);
        add!(self, "waitfor", who);
        add!(self, "jump", who, 6, 10);
        add!(self, "waitfor", who);
    }

    /// How it went.
    pub(crate) fn won(&mut self, side: usize, level: i32, battle: &Battle) {
        let bodies = &battle.teams[side].bodies;
        let winner = battle.fighter(side).who;
        if self.little_bars {
            let standing: Vec<Who> = bodies.iter().filter(|body| !body.down && !body.benched).map(|body| body.who).collect();
            for &who in &standing {
                self.joy(who);
                add!(self, "bar", who, "off");
            }
            add!(self, "t", 50);
            let team: Vec<String> = bodies.iter().map(|body| self.named(body.who)).collect();
            self.logged(format!("{} win the fight!", team.join(" and ")));
            add!(self, "t", 80);
            return;
        }
        let many = bodies.len() > 1;
        let (lead, name) = (bodies[0].who, self.named(winner));
        add!(self, "t", 40);
        if !self.in_tournament {
            add!(self, "fademusic", 0, 40);
        }
        self.joy(winner);
        let team = self.named(lead);
        let said = if many && self.played { format!("{team} and the team win the fight!") } else { format!("{name} wins the fight!") };
        self.logged(said);
        add!(self, "t", 30);
        let points = level * EXPERIENCE;
        let said = if many && self.played { format!("Each one gains {points} Exp. Points.") } else { format!("{name} gains {points} Exp. Points.") };
        self.logged(said);
        add!(self, "t", 40);
        if !self.played && self.roll(100) < LEVELS_UP {
            add!(self, "jingle", 7);
            add!(self, "emote", winner, "joy", 50);
            add!(self, "jump", winner, 10, 10);
            self.logged(said::level_up(&name, level + 1));
            add!(self, "t", 50);
        }
        add!(self, "t", if self.played { 120 } else { 40 });
    }
}
