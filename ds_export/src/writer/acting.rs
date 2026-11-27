//! What someone does, as Sky's actors do it: animations and poses, emotes
//! and effects, jumps, moves and blows, the routines Sky has for some words.

use engine::{AnimId, Beat, BodyBeat, Direction, EmoteId, MoveId, Play, Pose, ScreenBeat, SoundBeat, Who};

use super::scene::Part;
use super::{SHOT, Writer};
use crate::notes::{Fate, Kind};
use crate::sky::{EFFECTS, animation, move_effect, own_named, pose_on_ds, speed};

/// How many times a trembling shakes (Sky's shiver, 4 frames each) before
/// its pain is left looped.
const SHIVERS: u32 = 8;

/// The frames the view jolts as a move strikes.
const JOLT: i32 = 8;

/// The frames of a step back from a blow.
const RECOIL: f32 = 10.0;

impl Writer<'_> {
    /// A body beat of what someone does: an animation, an effect, a jump, a move.
    pub(super) fn acting(&mut self, number: i32, word: Option<&str>, beat: &BodyBeat) {
        match beat {
            BodyBeat::Effect { who, id, .. } => self.effect(number, *who, *id),
            BodyBeat::MoveEffect { who, id } => self.move_effect(number, *who, *id),
            // A move's shot from one to another: what it throws flies, then its effect on the one it reaches.
            BodyBeat::MoveShot { from, to, id, frames } => {
                self.throw_shot(*from, *to, id, frames.map(|frames| frames.0));
                self.plain(number, word, &Beat::Body(BodyBeat::MoveEffect { who: *to, id: *id }));
            }
            // Hops: Sky's own jumps (`CORO_JUMP_…`), happy, surprised or angry.
            BodyBeat::Anim { who, anim, play } if anim.0 == 10 => self.hops(number, *who, play),
            BodyBeat::Anim { who, anim, play } => {
                self.animate(number, who, anim, play);
                // (Losing his balance: a stumble to one side and back.)
                if word == Some("stagger") {
                    self.stumble(number, *who);
                }
            }
            BodyBeat::Pose { who, pose } => self.pose(number, *who, pose),
            // A trembling: its pain looped and Sky's own shiver, a pixel
            // left and right every two frames (`m26a0615`), a while.
            BodyBeat::Tremble { who, frames, .. } => {
                let Some(actor) = self.who(number, *who) else { return };
                if frames.0 > 0.0 {
                    let set = self.set(*who, 19, true).unwrap_or(14);
                    self.line(&format!("SetAnimation<actor {actor}>({set});"));
                    self.shake(&actor, SHIVERS);
                } else {
                    self.line(&format!("SetAnimation<actor {actor}>(2);"));
                }
            }
            // A leap on the spot: up and down at Sky's hop's pace (its
            // `CORO_JUMP_…` rise 10 pixels at 2 a frame), 6 pixels at least.
            BodyBeat::Jump { who, height, frames } => {
                let Some(actor) = self.who(number, *who) else { return };
                let up = height.round().max(6.0);
                let pace = speed((up * 2.0 / frames.0.max(2.0)).max(1.0) * 256.0, 256.0);
                self.line(&format!("MoveHeight<actor {actor}>({pace}, {up});"));
                self.wait_for(&actor);
                self.line(&format!("MoveHeight<actor {actor}>({pace}, 0);"));
                self.wait_for(&actor);
            }
            // A leap to a place: up, across in the air, down (Sky's actors
            // do one move at a time: the arc in three).
            BodyBeat::Hop { who, at, height, frames } => {
                let (Some(actor), Some(to)) = (self.who(number, *who), self.point(at)) else { return };
                self.leap(*who, &actor, to, *height, frames.0, true);
            }
            // Onto someone (`ride`, `carry`): a leap to where it stands, left
            // up there for its back's height (`Ride`).
            BodyBeat::Throw { who, at, height, frames } => {
                let (Some(actor), Some(&to)) = (self.who(number, *who), self.at.get(at)) else { return };
                let onto = matches!(word, Some("ride" | "carry" | "lift" | "boost"));
                self.leap(*who, &actor, to, *height, frames.0, !onto);
            }
            // On its back, up there, given each of its moves (`things`).
            BodyBeat::Ride { who, on, high, .. } => {
                let Some(actor) = self.who(number, *who) else { return };
                self.ride(*who, *on);
                let up = if on.is_some() { (high + super::things::FRONT).round().max(0.0) as i32 } else { 0 };
                let pace = speed(high.max(8.0), 8.0);
                self.line(&format!("MoveHeight<actor {actor}>({pace}, {up});"));
                self.wait_for(&actor);
            }
            // Up in the air or down to the ground (`fly`, `land`).
            BodyBeat::Rise { who, height, frames } => {
                let Some(actor) = self.who(number, *who) else { return };
                let up = height.round().max(0.0) as i32;
                let pace = speed(height.abs().max(8.0), frames.0.max(8.0));
                self.line(&format!("MoveHeight<actor {actor}>({pace}, {up});"));
                self.wait_for(&actor);
            }
            // What Sky shows otherwise: a knock back, a sleeper's icon.
            // A state, as an effect on someone (LIMITS: confusion's stars, bubbles, Zs, ice).
            BodyBeat::Status { who, icon, .. } => self.status(number, who, icon),
            // A move, as LIMITS staged it: a wind-up, a lunge, the effect of
            // its type on the target, a small shake.
            BodyBeat::Move { who, id, target } => self.attack(number, who, id, target),
            BodyBeat::Recoil { who, from, distance } => self.recoil(number, *who, *from, *distance),
            BodyBeat::Float { who, height, frames } => self.float(number, *who, *height, frames.0),
            other => self.unexported(number, word, &Beat::Body(other.clone())),
        }
    }

    /// An emote or an effect over someone, shown a while.
    pub(super) fn effect(&mut self, number: i32, who: Who, id: EmoteId) {
        let Some(actor) = self.who(number, who) else { return };
        let effect = EFFECTS.iter().find(|(of, _)| *of == id.0).map_or(id.0.to_string(), |(_, name)| name.to_string());
        self.line(&format!("SetEffect<actor {actor}>({effect}, 3);"));
        self.shown_for = 0.0;
        if !self.showing.contains(&actor) {
            self.showing.push(actor);
        }
    }

    /// A move's own effect (`shown`) on someone; at a place: on the camera
    /// (performer 0), in the middle of the screen.
    fn move_effect(&mut self, number: i32, who: Who, id: MoveId) {
        let on = match who {
            Who::Spot(_) => "performer 0".to_string(),
            _ => match self.who(number, who) {
                Some(actor) => format!("actor {actor}"),
                None => return,
            },
        };
        let effect = self.shown(&id);
        self.line(&format!("SetEffect<{on}>({effect}, 3);"));
        self.line(&format!("WaitEffect<{on}>();"));
        self.line(&format!("SetEffect<{on}>(EFFECT_NONE, 3);"));
    }

    /// Hops: Sky's own jumps (`CORO_JUMP_…`), happy, surprised or angry as
    /// the line is said.
    fn hops(&mut self, number: i32, who: Who, play: &Play) {
        let Some(actor) = self.who(number, who) else { return };
        let mood = if self.said_with(number, "surprised") || self.said_with(number, "surprise") || self.said_with(number, "fear") {
            "SURPRISE"
        } else if self.said_with(number, "angry") || self.said_with(number, "anger") {
            "ANGRY"
        } else {
            "HAPPY"
        };
        let times = if let Play::Times(times) = play { *times } else { 1 };
        for _ in 0..times.max(1) {
            self.line(&format!("ExecuteCommon<actor {actor}>(CORO_JUMP_{mood}_FUNC_SERIES, 0);"));
            self.wait_for(&actor);
        }
    }

    /// A pose: Sky's animation for it, held.
    fn pose(&mut self, number: i32, who: Who, pose: &Pose) {
        let Some(actor) = self.who(number, who) else { return };
        let set = match pose {
            Pose::Idle => 2,
            Pose::Walk => 1,
            Pose::Sleep => self.set(who, 13, true).unwrap_or(6),
            Pose::Hurt => 14,
            Pose::Attack => 13,
        };
        self.stand_after.retain(|one| *one != actor);
        self.line(&format!("SetAnimation<actor {actor}>({set});"));
    }

    /// The words Sky plays with a routine of its own (`unionall`'s
    /// coroutines): the rest of their line's beats are left to it.
    pub(super) fn routine(&mut self, number: i32, word: Option<&str>, beat: &Beat) -> bool {
        // (A roar, power gathered: each as Sky's scripts write it, from the line's first beat.)
        if let Beat::Body(BodyBeat::Anim { who, .. }) = beat {
            match word {
                Some("roar") => return self.roar(number, *who),
                Some("charge") => return self.charge(number, *who),
                _ => {}
            }
        }
        let who = match beat {
            Beat::Body(BodyBeat::TurnBy { who, .. } | BodyBeat::Anim { who, .. } | BodyBeat::Effect { who, .. } | BodyBeat::Jump { who, .. }) => *who,
            // Eating, as Sky shows it: the eating animation and its effect, no item.
            Beat::Body(BodyBeat::PutNear { other, .. }) if word == Some("eat") => {
                let Some(actor) = self.who(number, *other) else { return true };
                if let Some(set) = self.set(*other, 15, false) {
                    self.line(&format!("SetAnimation<actor {actor}>({set});"));
                }
                self.line(&format!("SetEffect<actor {actor}>(EFFECT_EATING, 3);"));
                self.line("Wait(60);");
                self.line(&format!("SetEffect<actor {actor}>(EFFECT_NONE, 3);"));
                self.line(&format!("SetAnimation<actor {actor}>(2);"));
                self.line(&format!("SetEffect<actor {actor}>(EFFECT_JOYOUS, 3);"));
                if !self.showing.contains(&actor) {
                    self.showing.push(actor);
                }
                self.line("Wait(30);");
                return true;
            }
            Beat::Screen(ScreenBeat::Flash { .. }) | Beat::Sound(SoundBeat::Sound(_)) if word == Some("transition") => {
                if self.words.get(&number).is_some_and(|words| words.iter().any(|word| word == "boss")) {
                    self.calm();
                    self.line("CallCommon(CORO_BOSS_WIPE_FUNC);");
                    self.faded_out = true;
                    self.white_all = true;
                    return true;
                }
                return false;
            }
            _ => return false,
        };
        let routine = match word {
            Some("look") if self.said_with(number, "left") => "CORO_LOOK_AROUND_LEFT_FUNC_SERIES",
            Some("look") if self.said_with(number, "right") => "CORO_LOOK_AROUND_RIGHT_FUNC_SERIES",
            Some("look") if self.said_with(number, "down") => "CORO_LOOK_AROUND_DOWN_FUNC_SERIES",
            Some("look") if self.said_with(number, "up") => return false,
            Some("look") => "CORO_LOOK_AROUND_FUNC_SERIES",
            Some("shake_head") => "CORO_HEAD_SHAKE_FUNC_SERIES",
            Some("explain") => "CORO_EXPLANATION_FUNC_SERIES",
            _ => return false,
        };
        let Some(actor) = self.who(number, who) else { return true };
        self.line(&format!("ExecuteCommon<actor {actor}>({routine}, 0);"));
        self.wait_for(&actor);
        true
    }

    /// `roar`: Sky's bosses' roar, the screen shaking a while, the sprite's
    /// own Roar group once, or its Charge looped (as Groudon roars).
    pub(super) fn roar(&mut self, number: i32, who: Who) -> bool {
        let Some(actor) = self.who(number, who) else { return true };
        let species = self.actors.get(&self.became.get(&who).copied().unwrap_or(who)).map_or(0, |one| one.kind);
        let set = own_named(species, "Roar").and_then(|group| animation(group, false)).or_else(|| self.set(who, 11, true)).unwrap_or(2);
        let (shake, frames) = if self.said_with(number, "hard") { ("2, 2, 3.0", 70) } else { ("2, 1, 1.0", 60) };
        self.line(&format!("camera_SetEffect({shake});"));
        self.line(&format!("se_Play({});", rules::songs::noise::ROAR));
        self.line(&format!("SetAnimation<actor {actor}>({set});"));
        self.line(&format!("Wait({frames});"));
        self.line("camera_SetEffect(0, 0, 0);");
        self.line(&format!("SetAnimation<actor {actor}>(2);"));
        true
    }

    /// `charge`: its Charge looped, Sky's stats-up shine a beat, standing.
    pub(super) fn charge(&mut self, number: i32, who: Who) -> bool {
        let Some(actor) = self.who(number, who) else { return true };
        let set = self.set(who, 11, true).unwrap_or(2);
        self.line(&format!("SetAnimation<actor {actor}>({set});"));
        self.line(&format!("SetEffect<actor {actor}>(691, 3);"));
        self.line("se_Play(4881);");
        self.line("Wait(50);");
        self.line(&format!("SetEffect<actor {actor}>(EFFECT_NONE, 3);"));
        self.line(&format!("SetAnimation<actor {actor}>(2);"));
        true
    }

    /// `float`: a hover up and down, for ever, in a routine of its own;
    /// height 0: it lands.
    pub(super) fn float(&mut self, number: i32, who: Who, height: f32, frames: f32) {
        let Some(actor) = self.who(number, who) else { return };
        if height <= 0.0 {
            self.floating.retain(|one| *one != actor);
            self.line(&format!("MoveHeight<actor {actor}>(1, 0);"));
            self.wait_for(&actor);
            return;
        }
        // (A little higher than the engine's 4: Sky's sprites are as small, and its shadow shows it.)
        let up = height.max(8.0).round();
        let pace = speed(up * 2.0, frames.max(8.0));
        if self.capture.is_some() {
            self.line(&format!("MoveHeight<actor {actor}>({pace}, {up}); // float"));
            return;
        }
        let part = Part {
            actor: actor.clone(),
            told: vec![format!("MoveHeight<actor {actor}>({pace}, {up});")],
            own: Vec::new(),
            float: Some((pace, up.to_string())),
        };
        self.begin_block(format!("float_{actor}_{}", self.besides.len()), vec![part]);
    }

    /// A step back from a blow (`Recoil`): a slide away from who struck (from
    /// oneself: back the way it looks), facing as it was.
    pub(super) fn recoil(&mut self, number: i32, who: Who, from: Who, distance: f32) {
        let (Some(actor), Some(&at)) = (self.who(number, who), self.at.get(&who)) else { return };
        let ahead = |way: Direction| {
            let (x, y) = match way {
                Direction::Down => (0.0, 1.0),
                Direction::DownRight => (0.7, 0.7),
                Direction::Right => (1.0, 0.0),
                Direction::UpRight => (0.7, -0.7),
                Direction::Up => (0.0, -1.0),
                Direction::UpLeft => (-0.7, -0.7),
                Direction::Left => (-1.0, 0.0),
                Direction::DownLeft => (-0.7, 0.7),
            };
            (-x, -y)
        };
        let away = match self.at.get(&from).copied().filter(|_| from != who) {
            Some(there) => {
                let (dx, dy) = (at.0 - there.0, at.1 - there.1);
                let long = (dx * dx + dy * dy).sqrt();
                if long < 1.0 { ahead(self.facing.get(&who).copied().unwrap_or(Direction::Down)) } else { (dx / long, dy / long) }
            }
            None => ahead(self.facing.get(&who).copied().unwrap_or(Direction::Down)),
        };
        let to = (at.0 + away.0 * distance, at.1 + away.1 * distance);
        self.go(who, &actor, to, &speed(distance, RECOIL), true);
        self.wait_for(&actor);
        // (The film's next waits count the slide, as they count the shiver.)
        self.ahead += RECOIL;
        // (Shaken by it a moment, as the engine's 14 frames of trembling.)
        self.shake(&actor, 3);
    }

    /// A leap there in so many frames (Sky's actors do one move at a time:
    /// the arc in three): up at Sky's hop's pace, across in the air
    /// (`SlidePositionMark`, facing as it was), down if it lands.
    fn leap(&mut self, who: Who, actor: &str, to: (f32, f32), height: f32, frames: f32, lands: bool) {
        let up = height.round().max(8.0);
        let from = self.at.get(&who).copied().unwrap_or(to);
        let across = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
        let rise = speed(2.0 * 256.0, 256.0);
        self.line(&format!("MoveHeight<actor {actor}>({rise}, {up});"));
        self.wait_for(actor);
        self.go(who, actor, to, &speed(across, (frames - up).max(6.0)), true);
        self.wait_for(actor);
        if lands {
            self.line(&format!("MoveHeight<actor {actor}>({rise}, 0);"));
            self.wait_for(actor);
        }
    }

    /// Sky's shiver (`m26a0615`, `n09a1207`): a pixel right, two frames, a
    /// pixel back, two frames, so many times; the film's next waits count
    /// its time.
    pub(super) fn shake(&mut self, actor: &str, times: u32) {
        // (Once a line: a word's beats may tremble twice, `shiver`.)
        let shaken = (self.at_line, actor.to_string());
        if self.shaken.as_ref() == Some(&shaken) {
            return;
        }
        self.shaken = Some(shaken);
        for _ in 0..times {
            self.line(&format!("SlidePositionOffset<actor {actor}>(1, 1, 0);"));
            self.line("Wait(2);");
            self.line(&format!("SlidePositionOffset<actor {actor}>(1, -1, 0);"));
            self.line("Wait(2);");
        }
        self.ahead += 4.0 * times as f32;
    }

    /// A stumble (`stagger`): a lurch of a few pixels to one side, a step
    /// too far the other way, and back where he stood.
    fn stumble(&mut self, number: i32, who: Who) {
        let Some(actor) = self.who(number, who) else { return };
        for (dx, pace) in [(-4, "1"), (6, "1.5"), (-2, "0.5")] {
            self.line(&format!("SlidePositionOffset<actor {actor}>({pace}, {dx}, 0);"));
            self.wait_for(&actor);
        }
        self.ahead += 16.0;
    }

    /// A move, as Sky's own scripts strike (`m17a0302`): the user's attack
    /// at once (no wind-up), what it throws flown to the target, the
    /// move's effect there and a jolt of the view. Not waited for: the
    /// effect plays on as the film goes on (its own waits count the jolt),
    /// put away with the emotes once the line is over; the user stands
    /// again at the next line.
    fn attack(&mut self, number: i32, who: &Who, id: &MoveId, target: &Option<Who>) {
        let Some(actor) = self.who(number, *who) else { return };
        self.line(&format!("SetAnimation<actor {actor}>(13);"));
        if !self.stand_after.contains(&actor) {
            self.stand_after.push(actor.clone());
        }
        // (Aimed at an unseen point, where one dodged from, or at a place:
        // on no one; the move's own effect as it strikes, on its user, if it has one.)
        if let Some(Who::Spot(_)) = target {
            if let Some(effect) = self.game.move_effects.get(usize::from(id.0)).and_then(|effects| [effects[1], effects[0]].into_iter().find(|effect| *effect != 0)) {
                self.line(&format!("SetEffect<actor {actor}>({effect}, 3);"));
                self.lasting(actor);
            }
            return;
        }
        let on = match target.and_then(|target| self.who(number, target)) {
            Some(target) => target,
            None => actor.clone(),
        };
        // (What it throws flies from its user to the one it reaches first.)
        if let Some(target) = target.filter(|target| target != who) {
            self.throw_shot(*who, target, id, None);
        }
        let effect = self.shown(id);
        self.line(&format!("SetEffect<actor {on}>({effect}, 3);"));
        self.line("camera_SetEffect(2, 1, 1.0);");
        self.line(&format!("Wait({JOLT});"));
        self.line("camera_SetEffect(0, 0, 0);");
        self.ahead += JOLT as f32;
        self.lasting(on);
    }

    /// A move's effect left playing on someone: put away with the emotes,
    /// once its line is over.
    fn lasting(&mut self, actor: String) {
        self.shown_for = 0.0;
        if !self.showing.contains(&actor) {
            self.showing.push(actor);
        }
    }

    /// What a move throws (the game's fourth effect of it: Ember's flame,
    /// Water Gun's jet, Razor Leaf's leaves) flown from one to the other,
    /// at the move's own speed unless told how long: Sky's `SetEffect` on a
    /// performer of its own (`SHOT`) moved with `MovePositionMark`, as
    /// m12a1201 sends Lapras's wake; the film's next waits count its
    /// flight. In a straight line, drawn as the effect's first way (a lob's
    /// arc and the dungeons' turning it to its way are not Sky's scripts').
    /// Nothing when the move throws nothing or they stand together.
    pub(super) fn throw_shot(&mut self, from: Who, to: Who, id: &MoveId, frames: Option<f32>) {
        let Some(thrown) = self.game.move_effects.get(usize::from(id.0)).map(|effects| effects[3]).filter(|effect| *effect != 0) else { return };
        let (Some(start), Some(end)) = (self.at.get(&from).copied(), self.at.get(&to).copied()) else { return };
        let distance = ((end.0 - start.0).powi(2) + (end.1 - start.1).powi(2)).sqrt();
        if distance < 8.0 {
            return;
        }
        // (The game's 2, 3 or 6 pixels a frame; 3 without its tables.)
        let pace = f32::from(self.game.move_flights.get(usize::from(id.0)).copied().filter(|pace| *pace > 0).unwrap_or(3));
        let frames = frames.unwrap_or(distance / pace).max(1.0);
        self.shots = true;
        let (from_mark, to_mark) = (self.position(start), self.position(end));
        self.line(&format!("SetPositionMark<performer {SHOT}>({from_mark});"));
        self.line(&format!("SetEffect<performer {SHOT}>({thrown}, 3);"));
        self.line(&format!("MovePositionMark<performer {SHOT}>({}, {to_mark});", speed(distance, frames)));
        self.line(&format!("WaitExecutePerformer({SHOT});"));
        self.line(&format!("SetEffect<performer {SHOT}>(EFFECT_NONE, 3);"));
        self.ahead += frames;
    }

    /// What a move shows on the one it reaches: the move's own effect, as
    /// a dungeon shows it on its target (else as it strikes), Sky's scripts
    /// playing the game's effects by the same ids; a move the game's tables
    /// don't give (no data): the script effect of its type.
    pub(super) fn shown(&self, id: &MoveId) -> u16 {
        let own = self.game.move_effects.get(usize::from(id.0)).and_then(|effects| [effects[2], effects[1]].into_iter().find(|effect| *effect != 0));
        own.unwrap_or_else(|| move_effect(self.game.move_types.get(usize::from(id.0)).copied().unwrap_or(1)))
    }

    /// A state, as an effect on someone (LIMITS: confusion's stars, bubbles, Zs, ice).
    fn status(&mut self, number: i32, who: &Who, icon: &Option<&'static str>) {
        let Some(actor) = self.who(number, *who) else { return };
        let effect = match icon.map(str::to_ascii_lowercase) {
            None => "EFFECT_NONE",
            Some(icon) => match icon.as_str() {
                "confused" | "dizzy" => "17",
                "poisoned" | "poison" | "badly_poisoned" => "1",
                "sleep" | "asleep" | "yawning" => "51",
                "frozen" => "23",
                "up" | "boost" => "691",
                "down" | "slow" | "weak" => "685",
                _ => {
                    self.note(number, Kind::Effect, Fate::LeftOut, format!("the state {icon}: Sky has no effect for it"), "nothing");
                    return;
                }
            },
        };
        self.line(&format!("SetEffect<actor {actor}>({effect}, 3);"));
    }

    /// An animation of the sprite's, played as Sky's `SetAnimation` (once, so many times, looped).
    fn animate(&mut self, number: i32, who: &Who, anim: &AnimId, play: &Play) {
        let Some(actor) = self.who(number, *who) else { return };
        let looped = matches!(play, Play::Loop);
        // (A pose its sprite lacks: the nearest every sprite has, told by species.)
        let kind = self.actors.get(&self.became.get(who).copied().unwrap_or(*who)).map_or(0, |one| one.kind);
        let Some(pose) = pose_on_ds(kind, anim.0, looped) else { return };
        if let Some(shown) = pose.instead {
            let asked = ::film::ds::group_name(anim.0);
            let asked = if asked.is_empty() { anim.0.to_string() } else { asked.to_string() };
            self.instead_poses.entry((self.game.name(kind), asked, shown)).or_default().push(number);
        }
        let set = pose.set;
        match play {
            Play::Times(times) => {
                for _ in 1..*times {
                    self.line(&format!("SetAnimation<actor {actor}>({set});"));
                    self.line(&format!("WaitAnimation<actor {actor}>();"));
                }
                self.line(&format!("SetAnimation<actor {actor}>({set});"));
                if !self.stand_after.contains(&actor) {
                    self.stand_after.push(actor);
                }
            }
            _ => self.line(&format!("SetAnimation<actor {actor}>({set});")),
        }
    }
}
