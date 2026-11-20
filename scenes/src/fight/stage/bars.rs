//! The bars and how a fight opens and closes.

use super::*;

impl Staging {
    /// How a fight is set up: its first ones, who rests where.
    pub(crate) fn stand_ready(&mut self, battle: &Battle, played: bool) {
        self.played = played;
        self.little_bars = battle.mode == Mode::Together;
        self.lead = Some(battle.fighter(0).who);
        for side in 0..2 {
            let first = battle.fighter(side).who;
            self.fighting[side] = Some(first);
            self.slots[side] = self.spot(first).unwrap_or_default();
            for body in &battle.teams[side].bodies {
                self.names.insert(body.who, battle.name_of(body));
            }
        }
        let lead = battle.fighter(0).who;
        let bench = self.ground.free(self.slots[0] - Vec2::new(BENCH, 0.0));
        self.benches = self.at.clone();
        self.benches.insert(lead, bench);
    }

    /// How lines a film wrote are set up: those in them, who it goes by if
    /// a fight is going on, its bars.
    pub(crate) fn by_hand(&mut self, battle: &Battle, lead: Option<Who>, bars: &[(Who, i32, i32)]) {
        self.lead = lead;
        self.quiet = lead.is_none();
        for body in battle.teams.iter().flat_map(|team| &team.bodies) {
            self.names.insert(body.who, battle.name_of(body));
        }
        for (side, (who, left, most)) in bars.iter().enumerate() {
            self.bar_kept(side.min(1), *who, *left, *most);
        }
    }

    /// The top line of a dungeon, the log at the bottom, one bar, those who
    /// fight in view; or its name out of a dungeon.
    pub(crate) fn top_line(&mut self, who: Who, level: i32, life: (i32, i32), floor: Option<i32>, others: &[Who]) {
        let name = self.names.get(&who).cloned().unwrap_or_default();
        let place = floor.map_or(name, |floor| format!("{floor}F"));
        let (left, most) = life;
        add!(self, "set", "hp", left);
        add!(self, "hud", "left", format!("{place}  Lv{level}  HP {{hp}}/{most}"));
        add!(self, "gauge", "left", "hp", most, "green");
        let mut view = vec!["keepframe".to_string(), who.to_string()];
        view.extend(others.iter().map(Who::to_string));
        self.line(view);
    }

    /// All a fight showed goes; what is left of who it went by is kept for
    /// the summary.
    pub(crate) fn closes(&mut self, raging: &[Who], left: Option<i32>) {
        if let Some(left) = left {
            add!(self, "set", "summary_hp", left);
        }
        for who in raging {
            add!(self, "halo", who, "off");
            add!(self, "uncolour", who, 24);
        }
        for place in ["left", "right"] {
            add!(self, "hud", place, "off");
            add!(self, "gauge", place, "off");
        }
        add!(self, "keepframe", "off");
    }

    /// Who was beaten gets up, goes to the other, and joins it.
    pub(crate) fn joins(&mut self, who: Who, leader: Who) {
        add!(self, "pose", who, "idle");
        add!(self, "uncolour", who, 12);
        add!(self, "t", 40);
        add!(self, "face", who, leader);
        add!(self, "emote", who, "drop", 50);
        add!(self, "t", 60);
        add!(self, "approach", who, leader, 40);
        add!(self, "waitfor", who);
        add!(self, "emote", who, "joy", 50);
        add!(self, "jump", who, 10, 10);
        add!(self, "jingle", 7);
        let text = format!("{} wants to join the team!", self.named(who));
        self.told(text);
        add!(self, "t", 80);
    }

    /// Red with rage, and stronger: the number tells a fight going on.
    pub(crate) fn rages(&mut self, who: Who, number: &str, by: i32) {
        add!(self, "add", number, by);
        add!(self, "halo", who, 36, "red");
        add!(self, "colour", who, 255, 150, 150, 24);
        add!(self, "emote", who, "anger", 60);
        add!(self, "quake", 3, 40);
        add!(self, "flash", "red", 10);
        add!(self, "sound", "anger");
        let text = format!("{} flies into a rage!", self.named(who));
        self.told(text);
        add!(self, "t", 60);
    }

    /// Those who stand go back where they stood: a film goes on from there,
    /// as if nobody had moved.
    pub(crate) fn go_home(&mut self) {
        let mut going: Vec<(Who, Vec2)> = self
            .at
            .iter()
            .filter(|(who, _)| !self.flown.contains(who))
            .filter_map(|(who, at)| Some((*who, *self.home.get(who).filter(|home| home.distance(*at) >= 1.0)?)))
            .collect();
        going.sort_by_key(|(who, _)| *who);
        let mut longest = 0.0_f32;
        for (who, home) in going {
            longest = longest.max(self.at[&who].distance(home));
            add!(self, "run", who, home.x as i32, home.y as i32);
            self.at.insert(who, home);
        }
        if longest > 0.0 {
            add!(self, "t", (longest / RUNS) as i32 + 8);
        }
    }

    /// Someone's bar for the first time, or another's in its place.
    pub(crate) fn hud(&mut self, side: usize, who: Who, left: i32, most: i32) {
        let (place, var, colour) = [("left", "hp", "green"), ("right", "hp2", "red")][side];
        let name = self.names.get(&who).cloned().unwrap_or_default();
        add!(self, "set", var, left);
        add!(self, "hud", place, format!("{name}  HP {{{var}}}/{most}"));
        add!(self, "gauge", place, var, most, colour);
        self.bars.insert(who, Bar { left, most, var });
    }

    /// A bar kept from a `combat`: the number that holds it.
    pub(crate) fn bar_kept(&mut self, side: usize, who: Who, left: i32, most: i32) {
        self.bars.insert(who, Bar { left, most, var: ["hp", "hp2"][side] });
    }

    /// Both see the other, the ground shakes a little: it starts. The view
    /// frames the field once and stays still, as the original's scenes do.
    pub(crate) fn opens(&mut self, first: Who, second: Who, battle: &Battle) {
        if self.little_bars {
            let mut view = vec!["frame".to_string(), FRAMING.to_string()];
            view.extend(battle.teams.iter().flat_map(|team| &team.bodies).filter(|body| !body.benched).map(|body| body.who.to_string()));
            self.line(view);
            let great = self.great_bars;
            for body in battle.teams.iter().flat_map(|team| &team.bodies).filter(|body| !body.benched) {
                match great.and_then(|(left, right)| [left, right].iter().position(|who| *who == body.who)) {
                    Some(place) => self.hud(place, body.who, body.stats.hp, body.stats.max_hp),
                    None => {
                        add!(self, "bar", body.who, 1);
                        self.bars.insert(body.who, Bar { left: body.stats.hp, most: body.stats.max_hp, var: "" });
                    }
                }
            }
            return;
        }
        if self.kept_hud {
            return;
        }
        for (side, who) in [first, second].into_iter().enumerate() {
            let body = battle.fighter(side);
            self.hud(side, who, body.stats.hp, body.stats.max_hp);
        }
        add!(self, "frame", FRAMING, first, second);
        for name in ["summary_blows", "summary_crits", "summary_misses", "summary_hp"] {
            add!(self, "set", name, 0);
        }
        add!(self, "face", first, second);
        add!(self, "face", second, first);
        add!(self, "emote", first, "exclaim", 30);
        add!(self, "emote", second, "exclaim", 30);
        add!(self, "sound", "stunned");
        add!(self, "quake", 2, 16);
        self.logged("The fight begins!".to_string());
        add!(self, "t", 40);
    }

    /// A sight, as lines.
    pub fn sight(&mut self, sight: &Sight, battle: &Battle) {
        match sight {
            Sight::Opens { first, second } => self.opens(*first, *second, battle),
            Sight::StepsAside { who, other } => self.steps_aside(*who, *other),
            Sight::Blow(blow) => self.blow(blow),
            Sight::Deed(deed) => self.deed(deed),
            Sight::Berry { who, healed } => self.berry(*who, healed),
            Sight::Uses { who, thing, name, healed } => self.eats(*who, *thing, name, healed),
            Sight::Flings { who, thing, name, target, strike } => self.flings(*who, *thing, name, *target, strike),
            Sight::Shields { who } => self.shields(*who),
            Sight::Stopped { who, ailment } => self.stopped(*who, *ailment),
            Sight::Rests { who } => {
                add!(self, "tremble", who, 24);
                let said = format!("{} must rest!", self.named(*who));
                self.logged(said);
                add!(self, "t", 50);
            }
            Sight::Worn { who, strike, harsh } => self.worn(*who, strike, harsh.then_some(battle.sky)),
            Sight::Cured { who, ailment } => self.looks_well(*who, *ailment),
            Sight::Reborn { who, healed } => self.reborn(*who, healed),
            Sight::Falls { who, more } => self.falls(*who, *more),
            Sight::Enters { side, who, left, most } => self.enters(*side, *who, *left, *most),
            Sight::GoesBack { who, .. } => {
                add!(self, "status", who, "off");
                let bench = self.benches.get(who).copied().unwrap_or_default();
                add!(self, "run", who, bench.x as i32, bench.y as i32);
                add!(self, "t", 16);
            }
            Sight::Partners { first, second } => {
                let said = format!("{} and {} attack together!", self.named(*first), self.named(*second));
                self.logged(said);
            }
            Sight::Leaves { who } => {
                add!(self, "bar", who, "off");
                self.bars.remove(who);
            }
            Sight::Won { side, level } => self.won(*side, *level, battle),
            Sight::Passage(label) => self.steps.push_back(Step::Passage(label.clone())),
            Sight::Waits(frames) => add!(self, "t", frames),
            Sight::Says(text) => self.logged(text.clone()),
            Sight::Tempo(tempo) => {
                self.usual_tempo = (*tempo != 1.0).then_some(*tempo);
                add!(self, "tempo", tempo);
            }
            Sight::SlowHit(frames) => self.slow_hit = Some(*frames),
            Sight::StepsBetween { who, target, from } => self.steps_between(*who, *target, *from),
            Sight::StepsBack { who } => self.steps_back(*who),
            Sight::Swaps { out, comes, left, most } => self.swaps(*out, *comes, *left, *most),
        }
    }
}
