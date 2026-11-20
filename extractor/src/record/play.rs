//! What a frame of the record does: the scripts' acts on the scene, and the
//! playing of an event.

use std::rc::Rc;

use rom_formats::acting::{Acting, Outcome, Prompt, World};
use rom_formats::vm::{self, Act, Event, Program, Vars, Vm};

use super::super::rom_data::RomData;
use super::*;

impl World for Sim<'_> {
    fn act(&mut self, who: Option<Who>, act: &Act) {
        let who = who.map(as_character);
        let name = who.map_or("-".to_string(), code);
        // What is not about a character.
        match act {
            Act::Fade { out, white, frames, .. } => {
                self.fade_to = if *out { 1.0 } else { 0.0 };
                self.fade_rate = 1.0 / (*frames).max(1) as f32;
                self.emit(format!("fade\t{}\t{}\t{frames}", *out as u8, *white as u8));
            }
            // (Without anyone to follow, or back to how it starts: the
            // view is on the leader, wherever a scene puts it next.)
            Act::CameraFollow => {
                (self.on_leader, self.followed) = (who.is_none(), who);
                self.emit(format!("camera\t{}", who.or(self.leader).map_or("-".to_string(), code)));
            }
            Act::CameraAt(x, y) => {
                self.on_leader = false;
                self.emit(format!("camera\t@\t{x}\t{y}"));
            }
            Act::CameraDefault => {
                self.on_leader = true;
                self.emit(format!("camera\t{}", self.leader.map_or("-".to_string(), code)));
            }
            Act::TopGround(level) => self.emit(format!("top\t{}", level.map_or("-".to_string(), |level| level.to_string()))),
            Act::TopCameraAt(x, y) => self.emit(format!("topcam\t{x}\t{y}")),
            Act::WorldMap(asked) => self.map_act(*asked),
            Act::Music(song) => self.emit(format!("music\t{}", song.map_or("-".to_string(), |song| song.to_string()))),
            Act::Ambient(song) => self.emit(format!("ambient\t{}", song.map_or("-".to_string(), |song| song.to_string()))),
            Act::Silence => {
                self.emit("music\t-".to_string());
                self.emit("ambient\t-".to_string());
            }
            Act::Sound(id) => {
                self.heard(false, *id);
                self.emit(format!("sound\t{id}"));
            }
            Act::Jingle(id) => {
                self.heard(true, *id);
                self.emit(format!("jingle\t{id}"));
            }
            Act::TopFade { frames, .. } => self.top_fade = *frames as f32,
            Act::MusicFade { channel, frames } => {
                for fading in [0, 1].into_iter().filter(|fading| *channel == 2 || *channel == *fading) {
                    self.music_fade[fading as usize] = *frames as f32;
                }
                self.emit(format!("fademusic\t{channel}\t{frames}"));
            }
            Act::Shake { axis, strength } => self.emit(format!("shake\t{axis}\t{strength}")),
            // (As a change of ground: who stood there is gone, but for
            // the leader.)
            Act::Banner(dungeon) => {
                let leader = self.leader.filter(|leader| self.body(*leader).is_some());
                self.bodies.retain(|body| Some(body.who) == leader);
                let name = super::super::text(&self.data.dungeon_name(*dungeon));
                self.emit(format!("banner\t{name}"));
            }
            Act::WaitMode(frames) => self.wait_mode = *frames,
            Act::Dim { white, frames, to, .. } => {
                self.fade_to = 1.0 - (*to as f32 / 256.0).clamp(0.0, 1.0);
                self.fade_rate = ((self.fade_to - self.fade).abs() / (*frames).max(1) as f32).max(0.001);
                self.emit(format!("dim\t{}\t{frames}\t{to}", *white as u8));
            }
            Act::RemoveLayer { cast, layer } => {
                let from = Some((*cast, *layer));
                let leaving: Vec<Who> = self.bodies.iter().filter(|body| body.origin == from).map(|body| body.who).collect();
                self.bodies.retain(|body| body.origin != from);
                for who in leaving {
                    self.emit(format!("gone\t{}", code(who)));
                }
            }
            Act::Effect(effect) => {
                let art = self.data.effect(*effect);
                let lasts = art.map_or(0.0, |art| if art.repeats { f32::INFINITY } else { art.ticks });
                if let Some(body) = who.and_then(|who| self.body_mut(who)) {
                    body.effect = lasts;
                }
                self.emit(format!("effect\t{name}\t{effect}"));
            }
            // (A new map: who stood on the old one is gone with it, but
            // for the leader, who is the game's own and stays.)
            Act::Ground(level) => {
                let leader = self.leader.filter(|leader| self.body(*leader).is_some());
                self.bodies.retain(|body| Some(body.who) == leader);
                (self.level, self.station, self.common) = (*level, None, None);
                self.emit(format!("backdrop\t{level}\t1\t{}", leader.map_or("-".to_string(), code)));
            }
            Act::Backdrop(level) => self.emit(format!("backdrop\t{level}\t0")),
            Act::LoadStation { level, name } => self.station = self.data.scene(*level, name, true).map(Rc::new),
            Act::Layer { cast, layer } => self.layer(*cast, *layer as usize, false),
            Act::LayerHidden { cast, layer } => self.layer(*cast, *layer as usize, true),
            Act::RemoveActing => {
                let leaving: Vec<Who> = self.bodies.iter().filter(|body| body.acting).map(|body| body.who).collect();
                self.bodies.retain(|body| !body.acting);
                for who in leaving {
                    self.emit(format!("gone\t{}", code(who)));
                }
            }
            Act::Destroy => {
                if let Some(who) = who {
                    self.bodies.retain(|body| body.who != who);
                    self.emit(format!("gone\t{name}"));
                }
            }
            _ => {}
        }
        let Some(who) = who else {
            return;
        };
        let Some((position, facing, initial)) = self.body(who).map(|body| (body.position, body.facing, body.initial)) else {
            return;
        };
        match act {
            Act::Animation(number) => {
                let Some((group, looped, frozen)) = self.data.scene_animation(*number) else {
                    return;
                };
                // (How long it lasts, if it ends: its sprite's frames.)
                let body = self.body(who);
                let frames = body.and_then(|body| body.sprite.as_ref()?.animations.get(SCENE_ANIMATIONS + group)?.get(direction_of(facing) as usize - 1));
                let lasts: f32 = frames.map_or(0.0, |frames| frames.iter().map(|frame| frame.ticks as f32).sum());
                if let Some(body) = self.body_mut(who) {
                    body.animating = if looped || frozen { 0.0 } else { lasts };
                }
                self.emit(format!("anim\t{name}\t{group}\t{}\t{}", looped as u8, frozen as u8));
            }
            Act::Position(place) => {
                if let Some(to) = self.place(who, position, initial, *place) {
                    if let Some(body) = self.body_mut(who) {
                        (body.position, body.heading) = (to, None);
                        body.path.clear();
                    }
                    self.emit(format!("put\t{name}\t{}\t{}\t{}", to.x, to.y, engine::Direction::from_number(direction_of(facing)).name()));
                }
            }
            Act::Move { speed, path, slide } => {
                let mut from = position;
                let mut points = VecDeque::new();
                for place in path {
                    if let Some(to) = self.place(who, from, initial, *place) {
                        points.push_back(to);
                        from = to;
                    }
                }
                if let Some(body) = self.body_mut(who) {
                    (body.path, body.speed, body.slide, body.heading) = (points, *speed, *slide, None);
                }
            }
            Act::Height { speed, height } => {
                let mut frames = 1.0;
                if let Some(body) = self.body_mut(who) {
                    frames = ((*height - body.height).abs() / speed.max(0.01)).ceil().max(1.0);
                    body.rise = Some((*height, *speed));
                }
                self.emit(format!("rise\t{name}\t{height}\t{frames}"));
            }
            Act::Attributes { set, reset, animation } => {
                let mut bits = 0;
                if let Some(body) = self.body_mut(who) {
                    body.attributes = (body.attributes | *set) & !*reset;
                    bits = body.attributes;
                }
                self.emit(format!("attr\t{name}\t{bits}"));
                if let Some(animation) = animation {
                    self.act(Some(who), &Act::Animation(*animation));
                }
            }
            Act::Direction(look) => {
                if let Some(to) = self.look(position, facing, *look) {
                    if let Some(body) = self.body_mut(who) {
                        (body.facing, body.turn) = (to, None);
                    }
                    self.emit(format!("turn\t{name}\t{}", engine::Direction::from_number(direction_of(to)).name()));
                }
            }
            Act::Turn { frames, to } => {
                if let Some(to) = self.look(position, facing, *to).filter(|to| *to != facing) {
                    let target = direction_of(to);
                    if let Some(body) = self.body_mut(who) {
                        body.turn = Some((target, *frames, 0.0));
                    }
                }
            }
            _ => {}
        }
    }

    fn busy(&self, who: Option<Who>) -> bool {
        who.map(as_character).and_then(|who| self.body(who)).is_some_and(Body::busy)
    }

    fn animating(&self, who: Option<Who>) -> bool {
        who.map(as_character).and_then(|who| self.body(who)).is_some_and(|body| body.animating > 0.0)
    }

    fn effect(&self, who: Option<Who>) -> bool {
        who.map(as_character).and_then(|who| self.body(who)).is_some_and(|body| body.effect > 0.0 && body.effect.is_finite())
    }

    fn fading(&self) -> bool {
        self.fade != self.fade_to
    }

    fn top_fading(&self) -> bool {
        self.top_fade > 0.0
    }

    fn music_fading(&self, around: bool) -> bool {
        self.music_fade[around as usize] > 0.0
    }

    fn sounding(&self, jingle: bool, id: u16) -> bool {
        self.sounds.iter().any(|sound| (sound.0, sound.1) == (jingle, id))
    }

    fn random(&mut self, below: i32) -> i32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        ((self.seed >> 16) as i32).rem_euclid(below.max(1))
    }
}

/// Where a sprite's scene animations start among its animations (after
/// those of dungeons).
const SCENE_ANIMATIONS: usize = 5;
/// Frames an event may last, and frames it may go without anything new
/// before it is taken to wait for the player.
const LONGEST: u32 = 60 * 60 * 20;
const QUIET: u32 = 60 * 20;
/// Questions inside questions followed down every answer; questions one
/// way through may ask; lines the way on of another answer than the first
/// may hold.
const DEEPEST: usize = 5;
const ASKED: usize = 64;
const SIDE: usize = 19200;
/// The story's first scene.
const OPENING: &str = "m00a01a";

/// Plays an event of the story (a routine of the scripts, as `vm` starts
/// it) from a state of the story's variables. What is seen, as lines of
/// the story's file; and the state it leaves the variables in.
pub fn event(data: &RomData, program: &Program, vm: Vm, from: &Vars, lines: usize) -> (Vec<String>, Vars) {
    let mut sim = Sim {
        data,
        bodies: Vec::new(),
        leader: None,
        // (An event of the story opens on a dark screen.)
        fade: 1.0,
        fade_to: 1.0,
        fade_rate: 1.0,
        acting: None,
        station: None,
        common: None,
        level: 0,
        scene_level: 0,
        line_open: false,
        seed: 1,
        waited: 0,
        log: Vec::new(),
        seen: Vec::new(),
        map_shown: false,
        map: 0,
        map_level: 0,
        chosen: -1,
        clouds: 0,
        keep_clouds: None,
        travel: None,
        on_leader: true,
        followed: None,
        sounds: Vec::new(),
        top_fade: 0.0,
        music_fade: [0.0; 2],
        wait_mode: 0.0,
    };
    let (mut out, mut vars) = (Vec::new(), from.clone());
    play(&mut sim, vec![Acting::with(program.clone(), vm, None)], &mut vars, &mut out, 0, 0, lines);
    (out, vars)
}

fn play(sim: &mut Sim, mut stages: Vec<Acting>, vars: &mut Vars, out: &mut Vec<String>, depth: usize, mut asked: usize, longest: usize) {
    let start = out.len();
    let data = sim.data;
    let said = |text: &str| super::super::text(&data.script_text(text));
    // Questions answered in passing, whose `end` is still to write.
    let mut open = 0;
    let mut quiet = 0;
    // The line on screen: the frames it still stays, and whether anything
    // has gone on since it came.
    let mut reading: Option<(u32, bool)> = None;
    // (The time lines stay on screen is not the event's own.)
    let mut frames = 0;
    while frames < LONGEST {
        frames += reading.is_none() as u32;
        let Some(stage) = stages.last_mut() else {
            break;
        };
        (sim.chosen, sim.clouds) = (vars.get(vm::DUNGEON_SELECT), vars.get(vm::WORLD_MAP_LEVEL));
        let outcome = stage.step(vars, sim);
        if let Some(clouds) = sim.keep_clouds.take() {
            vars.set(vm::WORLD_MAP_LEVEL, clouds as i32);
        }
        // What is shown is read at once.
        match stage.prompt().cloned() {
            Some(Prompt::Message { speaker, text }) => match reading {
                None => {
                    let who =
                        speaker.map_or("-\t-".to_string(), |face| format!("{}\t{}", face.entity, face.face.map_or("-".to_string(), |face| face.to_string())));
                    let line = said(&text);
                    // (The time to read it: as long as it is, and a little;
                    // or the time the scene gives it before it goes by itself.)
                    let timed = (sim.wait_mode > 0.0).then_some(sim.wait_mode as u32);
                    reading = Some((timed.unwrap_or((line.chars().count() as u32 + 30).clamp(40, 150)), false));
                    sim.emit(format!("say\t{who}\t{line}"));
                    if let Some(timed) = timed {
                        sim.emit(format!("timed\t{timed}"));
                    }
                    out.append(&mut sim.log);
                }
                Some((0, during)) => {
                    // (Nothing went on meanwhile: no time is taken as passed.)
                    sim.waited = 0;
                    if during {
                        sim.emit("said".to_string());
                    } else {
                        sim.line_open = true;
                    }
                    stage.answer(0);
                    reading = None;
                }
                Some((left, during)) => reading = Some((left - 1, during)),
            },
            Some(Prompt::GameMenu(menu)) => {
                sim.emit(format!("menu\t{menu}"));
                stage.answer(0);
            }
            Some(Prompt::Menu { options }) => {
                let answers: Vec<String> = options.iter().map(|option| said(option)).collect();
                sim.emit(format!("ask\t{}", answers.join("|")));
                out.append(&mut sim.log);
                asked += 1;
                // (Deeper, or asked too often: the first answer alone,
                // in passing; a routine going round asks for ever.)
                if asked > ASKED {
                    break;
                }
                if depth >= DEEPEST {
                    out.push("answer\t0".to_string());
                    stage.answer(0);
                    open += 1;
                } else {
                    for way in 0..options.len().max(1) {
                        out.push(format!("answer\t{way}"));
                        let (mut branch, mut state, mut scene) = (stages.clone(), vars.clone(), sim.clone());
                        if let Some(stage) = branch.last_mut() {
                            stage.answer(way);
                        }
                        // (The first answer's way is the story's: as long
                        // as it may be. The others': the shorter the deeper.)
                        let left = longest.saturating_sub(out.len() - start);
                        let room = if way == 0 { left } else { left.min(SIDE >> depth) };
                        play(&mut scene, branch, &mut state, out, depth + 1, asked, room);
                        if way == 0 {
                            *vars = state;
                        }
                    }
                    out.push("end".to_string());
                    return;
                }
            }
            None => {}
        }
        sim.advance();
        if let Some((_, during)) = &mut reading {
            *during |= !sim.log.is_empty();
        }
        quiet = if sim.log.is_empty() && reading.is_none() { quiet + 1 } else { 0 };
        out.append(&mut sim.log);
        match outcome {
            Outcome::Running => {}
            Outcome::Done => {
                stages.pop();
                match stages.last_mut() {
                    Some(under) => under.resume(),
                    None => {
                        // (The way to a dungeon over the map is over: in.)
                        if let Some(dungeon) = sim.travel.take() {
                            out.push("map\thide".to_string());
                            out.push(format!("dungeon\t{dungeon}"));
                        }
                        break;
                    }
                }
            }
            Outcome::Leave(Event::EnterDungeon(dungeon)) => {
                // The team is first seen on its way there over the world
                // map, if the dungeon is on it: a routine of the game's own.
                let on_map = sim.travel.is_none() && sim.data.map_marker(dungeon).is_some_and(|(map, ..)| map == MAIN_MAP);
                let way = sim.data.common_routine(vm::MOVE_WORLD_MAP).filter(|_| on_map);
                match way {
                    Some(way) => {
                        vars.set(vm::DUNGEON_SELECT, dungeon as i32);
                        sim.travel = Some(dungeon);
                        stages.clear();
                        let vm = Vm::start_common(&way, vm::MOVE_WORLD_MAP);
                        stages.push(Acting::with(way, vm, None));
                    }
                    None => {
                        out.push(format!("dungeon\t{dungeon}"));
                        break;
                    }
                }
            }
            Outcome::Leave(Event::EnterGround(level)) | Outcome::Scene { level, station: true, .. } => {
                out.push(format!("ground\t{level}"));
                break;
            }
            Outcome::Leave(_) => break,
            Outcome::Scene { level, name, sub, .. } => {
                // (A scene come back to: the routine goes round, enough.
                // And an event over hands the story back to the routine
                // that picks the next one: with the story's variables as
                // they are here, that is the story's very first scene.)
                if sim.seen.contains(&name) || (name == OPENING && !sim.seen.is_empty()) {
                    break;
                }
                sim.seen.push(name.clone());
                let scene = sim.data.scene(level, &name, false);
                let program = scene.as_ref().and_then(|scene| scene.program.clone());
                if !sub {
                    stages.clear();
                }
                match program {
                    Some(program) => {
                        (sim.acting, sim.level, sim.scene_level) = (scene.map(Rc::new), level, level);
                        sim.line_open = false;
                        out.push(format!("scene\t{level}\t{name}"));
                        stages.push(Acting::new(program, None));
                    }
                    None => match stages.last_mut() {
                        Some(under) => under.resume(),
                        None => break,
                    },
                }
            }
        }
        if quiet > QUIET || out.len() - start > longest {
            break;
        }
    }
    out.append(&mut sim.log);
    out.extend((0..open).map(|_| "end".to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn things_are_kept_as_characters() {
        assert_eq!(as_character(Who::Object(93)), Who::Actor(THINGS + 93));
        assert_eq!(as_character(Who::Actor(2)), Who::Actor(2));
        assert_eq!(as_character(Who::Performer(1)), Who::Performer(1));
    }
}
