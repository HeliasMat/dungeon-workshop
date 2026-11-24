//! A scene as it goes, and nothing else: the director and the stage, the
//! fight going on, how each one shows (its animation and step), the effects,
//! the music, the clock. No window, no sound, no picture: what it would have
//! heard it lists (`heard`), for who plays it. It moves on one frame at a
//! time and is cloned whole: a clone is the scene at that moment.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use engine::{Beat, Body, BodyBeat, Call, Cue, Director, Frames, Play, ScreenBeat, Stage, StoryEvent, Who};
use glam::Vec2;
use rules::tables::Tables;
use scenes::fight::host::called;
use scenes::fight::{Fighting, Ground, Rules};
use scenes::vocabulary::POKEMON;
use shell::art::Art;
use shell::images::{Effect, Sprite, read_together};
use shell::sound::{Tune, Voice};

use super::SCENE_ANIMATIONS;
use super::cast::Cast;
use super::pose::{HURT, IDLE, Shown, WALK, pose_of, way_of};

/// What a scene reads of the game's pictures and tables as it goes: how long
/// an animation's steps last, an effect's, a place's walls; never a pixel.
#[derive(Clone, Copy)]
pub struct Facts<'a> {
    pub art: &'a Art,
    pub tables: &'a Tables,
}

/// What a scene would have heard, for who plays it. Music levels are of the
/// music as loud as it may be (1), sounds' of sounds as loud as they may be.
#[derive(Clone, Debug, PartialEq)]
pub enum Heard {
    Song {
        channel: usize,
        song: Option<u16>,
        level: f32,
        step: f32,
    },
    SongFrom {
        channel: usize,
        song: Option<u16>,
        level: f32,
        seconds: f32,
    },
    Volume {
        channel: usize,
        level: f32,
    },
    Sound {
        id: u16,
        volume: f32,
        pitch: f32,
    },
    /// A file of our own as the song of a channel.
    OwnSong {
        channel: usize,
        name: String,
        level: f32,
        step: f32,
        seconds: f32,
    },
    /// A sound as asked: from a side, looped, named (see `shell::sound::Voice`).
    Play(Voice),
    /// A sound or an ambiance by its name, to that level (see `SoundBeat::Fade`).
    FadeVoice {
        name: String,
        level: f32,
        frames: f32,
        stop: bool,
    },
    /// Every sound going on stops: a scene opens on a cleared stage.
    Hush,
    Jingle(u16),
    Fade {
        channel: usize,
        frames: f32,
    },
    Speed(f32),
}

/// An effect showing on someone, and where on it.
#[derive(Clone)]
pub(super) struct ShownEffect {
    pub(super) art: Rc<Effect>,
    pub(super) gone: f32,
    pub(super) point: u8,
}

/// A scene as it goes.
#[derive(Clone)]
pub struct Sim {
    pub director: Director,
    pub stage: Stage,
    pub cast: Cast,
    /// Where the view looks.
    pub focus: Option<Vec2>,
    /// The frames gone by.
    pub clock: f32,
    /// A fight going on: the scene holds for it.
    pub fight: Option<Fighting>,
    /// A dungeon's name on a dark screen, instead of a place.
    pub banner: Option<String>,
    /// The place shown, by its level; none: a dark screen.
    pub place: Option<u16>,
    /// The song a scene plays now, and the one before.
    pub songs: [Option<Tune>; 2],
    /// The frame (of `clock`) the song playing now would have begun at, from
    /// its start: where it is, when it is heard again after a jump.
    pub song_start: f32,
    /// How loud a scene has the music (see `SoundBeat::Volume`).
    pub loudness: f32,
    /// Those where each scene started, so far.
    pub scene_starts: Vec<f32>,
    /// What it would have heard since it was last asked.
    pub heard: Vec<Heard>,
    pub(super) shown: BTreeMap<Who, Shown>,
    /// The effects shown, by who shows them and which (several on one).
    pub(super) effects: BTreeMap<(Who, u16), ShownEffect>,
    /// The animations the scene will ask of each, and those of who has just
    /// come on the stage, to be read before that.
    planned: HashMap<Who, Vec<usize>>,
    pub(super) arriving: Vec<(Rc<Sprite>, usize)>,
    /// The blows of the frame at hand: a crowd's are not all heard.
    pub(super) blows_heard: usize,
    /// Where on its user a move shows; the effects of moves to be heard.
    pub(super) user_points: HashMap<Who, u8>,
    pub(super) heard_effects: HashSet<u16>,
}

impl Sim {
    pub fn new(director: Director, cast: Cast) -> Self {
        let mut planned: HashMap<Who, Vec<usize>> = HashMap::new();
        for beat in director.events().iter().flat_map(|event: &StoryEvent| &event.scenes).flat_map(|scene| &scene.beats) {
            if let Beat::Body(BodyBeat::Anim { who, anim: group, .. }) = beat {
                let animations = planned.entry(*who).or_default();
                if !animations.contains(&(SCENE_ANIMATIONS + group.index())) {
                    animations.push(SCENE_ANIMATIONS + group.index());
                }
            }
        }
        Sim {
            director,
            stage: Stage::dark(),
            cast,
            focus: None,
            clock: 0.0,
            fight: None,
            banner: None,
            place: None,
            songs: [None, None],
            song_start: 0.0,
            loudness: 1.0,
            scene_starts: Vec::new(),
            heard: Vec::new(),
            shown: BTreeMap::new(),
            effects: BTreeMap::new(),
            planned,
            arriving: Vec::new(),
            blows_heard: 0,
            user_points: HashMap::new(),
            heard_effects: HashSet::new(),
        }
    }

    /// What comes next: the director plays on, the fight going on with it.
    /// Gives what it asks, and the choice a fight offers, if it does now.
    pub fn direct(&mut self, frames: f32) -> (Vec<Call>, Option<Vec<String>>) {
        self.blows_heard = 0;
        let focus = self.focus.unwrap_or_default();
        let mut calls = self.director.tick(&mut self.stage, frames, focus);
        let mut menu = None;
        if let Some(fight) = &mut self.fight {
            fight.tick(frames, &mut self.director, &mut self.stage, focus, &mut calls);
            menu = fight.menu();
            if fight.over() {
                self.fight = None;
            }
        }
        // (The choices with the names they are shown by.)
        let menu = menu.map(|menu| menu.iter().map(|option| self.cast.names_in(option)).collect());
        (calls, menu)
    }

    /// A frame of the stage goes by: who walks, the light, how each shows.
    pub fn pass(&mut self, frames: f32, facts: Facts) {
        self.focus = self.stage.advance(frames, self.clock / 60.0, self.focus.or(Some(Vec2::ZERO)));
        let texts = self
            .stage
            .words
            .bubbles
            .iter_mut()
            .map(|bubble| &mut bubble.text)
            .chain(self.stage.words.hud.iter_mut().flatten())
            .chain(self.stage.words.caption.as_mut().map(|caption| &mut caption.text));
        for text in texts.filter(|text| text.contains("{name:") || text.contains("{item:")) {
            *text = self.cast.names_in(text);
        }
        self.bodies(frames, facts);
    }

    /// A scene opens (`Call::Opens`): its start is kept; `cleared`, on an
    /// empty stage and a dark screen.
    pub fn opens(&mut self, cleared: bool) {
        self.scene_starts.push(self.clock);
        if cleared {
            self.clear();
        }
    }

    /// The scene opens on an empty stage and a dark screen.
    pub fn clear(&mut self) {
        (self.place, self.banner) = (None, None);
        self.shown.clear();
        self.effects.clear();
        self.heard.push(Heard::Song { channel: 1, song: None, level: 1.0, step: 0.0 });
        self.heard.push(Heard::Hush);
    }

    /// Someone comes on the stage, shown as `looks`.
    pub fn show(&mut self, who: Who, looks: Who, facts: Facts) {
        let Some(sprite) = self.cast.sprite(facts.art, facts.tables, looks) else {
            self.shown.remove(&who);
            return;
        };
        let (flat, shadow) = match looks {
            Who::Thing { object, .. } => (facts.art.flats.contains(&object), None),
            Who::Item { .. } | Who::Drawn { .. } => (false, Some(0)),
            _ if self.cast.has_sheet(looks) => (false, Some(1)),
            _ => (false, facts.tables.species.get(facts.art.kin(self.cast.who(looks).0) as usize).and_then(|species| species.shadow)),
        };
        self.arriving.extend(self.planned.get(&who).into_iter().flatten().map(|at| (sprite.clone(), *at)));
        // (Each on its own step, once the animation is read: no crowd
        // breathes as one.)
        let frame = (who.number().unsigned_abs() as usize).wrapping_mul(7);
        self.shown.insert(
            who,
            Shown { sprite, animation: IDLE, frame, elapsed: 0.0, once: false, staged: false, looped: true, frozen: false, again: 0, flat, shadow },
        );
    }

    /// Whether that one lies on the ground (those who stand walk over it).
    pub fn lies_flat(&self, who: Who) -> bool {
        self.shown.get(&who).is_none_or(|shown| shown.flat)
    }

    /// The name of what someone shows now.
    pub fn pose(&self, who: Who) -> Option<&'static str> {
        const USUAL: [&str; SCENE_ANIMATIONS] = ["Walk", "Idle", "Hurt", "Sleep", "Attack"];
        let animation = self.shown.get(&who)?.animation;
        match animation.checked_sub(SCENE_ANIMATIONS) {
            None => Some(USUAL[animation]),
            Some(group) => POKEMON.animations.iter().find(|(_, known)| *known == group).map(|(name, _)| *name),
        }
    }

    /// Someone moved by hand, not sent by the stage, shows walking.
    pub fn walking(&mut self, who: Who) {
        if let Some(shown) = self.shown.get_mut(&who).filter(|shown| shown.animation != WALK) {
            shown.start(WALK, true);
        }
    }

    /// The line of a fight.
    pub fn fought(&mut self, fields: &[String], facts: Facts) {
        let cast = &self.cast;
        let rules = |species: &[u16]| Some(Rules::of_kin(facts.tables, species, &|id| facts.art.kin(id)));
        let ground = |level: u16| {
            let found = assets::tables::grounds(&facts.art.folder).into_iter().find(|ground| ground.level == level);
            Ground::from_rows(found.map_or_else(Vec::new, |ground| ground.tiles()))
        };
        called(&mut self.fight, fields, &rules, &ground, &|who: Who| cast.who(who).0, &mut self.director, &mut self.stage);
    }

    /// The song of the scene, as the music goes on after it was skipped to.
    pub fn song_again(&mut self) {
        // (From where it is by now, not from its start.)
        let seconds = ((self.clock - self.song_start) / 60.0).max(0.0);
        self.heard.push(match self.songs[0].clone() {
            Some(Tune::Game(song)) => Heard::SongFrom { channel: 0, song: Some(song), level: self.loudness, seconds },
            Some(Tune::Own(name)) => Heard::OwnSong { channel: 0, name, level: self.loudness, step: 0.0, seconds },
            None => tune_heard(None, self.loudness, 0.0),
        });
    }

    /// A beat of the stage only the one showing it can play.
    pub fn beat(&mut self, beat: Beat, facts: Facts) {
        // (A new place: the sprites of the last one leave memory.)
        if matches!(beat, Beat::Screen(ScreenBeat::Backdrop(..) | ScreenBeat::Crossfade { .. })) {
            self.cast.forget(facts.art);
        }
        match beat {
            Beat::Body(BodyBeat::Put { who, .. }) => self.show(who, who, facts),
            Beat::Body(BodyBeat::Drawing { who, file, cells, ticks }) => self.cast.draws(who, file, cells, ticks),
            // (Shown again from its sheet, if it is on the stage.)
            Beat::Body(BodyBeat::Sheet { who, file, cell, ways, anims }) => {
                self.cast.sheet(who, file, cell, ways, &anims);
                if self.stage.cast.contains_key(&who) {
                    self.show(who, who, facts);
                }
            }
            Beat::Body(BodyBeat::Disguise { who, like } | BodyBeat::Mimic { who, like }) => self.show(who, like.unwrap_or(who), facts),
            Beat::Screen(ScreenBeat::Backdrop(level, _, _)) => {
                (self.banner, self.place) = (None, Some(level));
                self.stage.place_walls(walls_of(&facts.art.folder, level));
            }
            Beat::Screen(ScreenBeat::Crossfade { level, .. }) => self.place = Some(level),
            Beat::Screen(ScreenBeat::Banner(name)) => (self.place, self.banner) = (None, Some(name)),
            Beat::Body(BodyBeat::Move { who, id, target }) => self.move_used(who, id.0, target, facts),
            Beat::Body(BodyBeat::MoveEffect { who, id }) => self.effect_of_move(who, id.0, facts),
            Beat::Body(BodyBeat::MoveShot { from, to, id, frames }) => self.shot_of_move(from, to, id.0, frames.map(Frames::get), facts),
            beat => self.heard(beat, facts),
        }
    }

    /// The pictures each one shows, and the step they are on.
    pub(super) fn showing(&self) -> impl Iterator<Item = (&Sprite, usize)> {
        self.shown.values().map(|shown| (&*shown.sprite, shown.showing()))
    }

    /// Who stands there poses, walks, and its effects come and go.
    fn bodies(&mut self, frames: f32, facts: Facts) {
        self.shown.retain(|who, _| self.stage.cast.contains_key(who));
        for (who, shown) in &mut self.shown {
            let Some(actor) = self.stage.cast.get_mut(who) else {
                continue;
            };
            match actor.cue.take() {
                Some(Cue::Pose(pose)) => shown.start(pose_of(pose), false),
                Some(Cue::Motion(group)) => shown.motion(group),
                Some(Cue::Hurt) => shown.start(HURT, true),
                Some(Cue::Anim { group, play }) => match play {
                    Play::Once => shown.stage(group, false, false),
                    Play::Loop => shown.stage(group, true, false),
                    Play::Still { or_stand } => shown.stage(group, or_stand, true),
                    Play::Times(times) => shown.times(group, times),
                    Play::Hold(step) => shown.held(group, step),
                },
                None if actor.walks() && shown.animation != WALK && !shown.staged => shown.start(WALK, true),
                None => {}
            }
        }
        // (The steps of each animation shown, read now if they were not yet.)
        read_together(self.showing());
        if !self.stage.light.still {
            for (who, shown) in &mut self.shown {
                if let Some(actor) = self.stage.cast.get(who) {
                    shown.advance(frames * actor.picture_speed(), way_of(actor.direction));
                }
            }
        }
        // (Where each one's head, hands and middle are on the step it shows,
        // for what it holds and who rides it.)
        for (who, shown) in &self.shown {
            let Some(actor) = self.stage.cast.get_mut(who) else {
                continue;
            };
            let monster = facts.art.kin(self.cast.who(*who).0);
            let point = |at: (i32, i32)| Vec2::new(at.0 as f32, at.1 as f32);
            let now = shown.picture(way_of(actor.direction)).and_then(|frame| frame.points);
            actor.body = now.map(|now| Body { now: now.map(point), standing: [0, 1, 2, 3].map(|at| point(facts.art.point(monster, at))) });
        }
        self.effects_pass(frames, facts);
    }
}

/// The walls of a place as `town.txt` tells them (a row of solid tiles, a
/// box): left, top, right, bottom.
fn walls_of(folder: &str, level: u16) -> Vec<[f32; 4]> {
    const TILE: f32 = 8.0;
    let mut walls = Vec::new();
    for (row, solid) in shell::art_files::solid_rows(folder, level).iter().enumerate() {
        let mut x = 0;
        while x < solid.len() {
            let run = solid[x..].iter().take_while(|tile| **tile).count();
            if run > 0 {
                walls.push([x as f32 * TILE, row as f32 * TILE, (x + run) as f32 * TILE, (row + 1) as f32 * TILE]);
            }
            x += run.max(1);
        }
    }
    walls
}

/// A song of the music, the game's or ours, as it is heard.
pub(super) fn tune_heard(tune: Option<Tune>, level: f32, step: f32) -> Heard {
    match tune {
        Some(Tune::Own(name)) => Heard::OwnSong { channel: 0, name, level, step, seconds: 0.0 },
        Some(Tune::Game(song)) => Heard::Song { channel: 0, song: Some(song), level, step },
        None => Heard::Song { channel: 0, song: None, level, step },
    }
}
