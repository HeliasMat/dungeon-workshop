//! Runs a scene: the routines of a script side by side, one for the scene
//! itself and one for each character given something to do, the way the
//! game's script engine does.
//!
//! A routine hands an operation to a character with `lives` (see
//! `Event::Lives`): the character runs it on its own while the routine
//! goes on, and `WaitExecuteLives` waits for it. What routines do to the
//! game goes through `World`.

use std::collections::VecDeque;

use crate::vm::{Act, Cast, Cond, Event, Face, Program, Vars, Vm, Who};

/// The game, as scenes see it.
pub trait World {
    /// Carries out a staging operation; `who` is the routine's character
    /// (`None` for the scene's own routine).
    fn act(&mut self, who: Option<Who>, act: &Act);
    /// Whether the character is still moving or turning.
    fn busy(&self, who: Option<Who>) -> bool;
    /// Whether the character's animation is one that ends, and has not.
    fn animating(&self, who: Option<Who>) -> bool;
    /// Whether an effect that ends is still playing on the character.
    fn effect(&self, _who: Option<Who>) -> bool {
        false
    }
    /// Whether the screen is still fading.
    fn fading(&self) -> bool;
    /// Whether the music (or, `around`, what is heard around) is still
    /// fading away.
    fn music_fading(&self, _around: bool) -> bool {
        false
    }
    /// Whether the top screen is still fading.
    fn top_fading(&self) -> bool {
        false
    }
    /// Whether a sound (or, `jingle`, a jingle) started in the scene is
    /// still heard.
    fn sounding(&self, _jingle: bool, _id: u16) -> bool {
        false
    }
    /// A number below `below`.
    fn random(&mut self, below: i32) -> i32;
}

/// What a routine shows and waits an answer for.
#[derive(Clone, Debug, PartialEq)]
pub enum Prompt {
    Message {
        speaker: Option<Face>,
        text: String,
    },
    Menu {
        options: Vec<String>,
    },
    /// One of the game's own menus, by number.
    GameMenu(u16),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Blocked {
    No,
    Frames(u16),
    /// Moving or turning.
    Busy,
    On(Cond),
    /// Showing a prompt.
    Prompt,
    /// Waiting for a scene played inside this one.
    Scene,
}

#[derive(Clone)]
struct Routine {
    who: Option<Who>,
    vm: Vm,
    blocked: Blocked,
    done: bool,
    /// Operations handed to the character while it still carries out an
    /// earlier one: it takes them one after the other.
    handed: VecDeque<Vm>,
    /// Whether the routine is such a handed operation, not one the script
    /// gives its character from the start.
    single: bool,
}

/// How a frame of the scene ended.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Running,
    /// The scene's own routine is over.
    Done,
    /// The scene leaves for a level or a dungeon.
    Leave(Event),
    /// Another scene must be played; with `sub`, call [`Acting::resume`]
    /// when it is over.
    Scene {
        level: u16,
        name: String,
        station: bool,
        sub: bool,
    },
}

#[derive(Clone)]
pub struct Acting {
    program: Program,
    /// The first is the scene's own.
    routines: Vec<Routine>,
    /// Prompts waiting to be shown, with the routine that asked.
    prompts: VecDeque<(usize, Prompt)>,
    /// Locks opened while nobody waited at them.
    unlocked: Vec<u16>,
}

/// Operations a routine may run in one frame.
const FRAME_LIMIT: usize = 2_000;

impl Acting {
    /// A scene running the program's routine, for `who`.
    pub fn new(program: Program, who: Option<Who>) -> Self {
        let vm = Vm::start(&program);
        Self::with(program, vm, who)
    }

    /// The same from a routine already set to start somewhere.
    pub fn with(program: Program, vm: Vm, who: Option<Who>) -> Self {
        let main = Routine { who, vm, blocked: Blocked::No, done: false, handed: VecDeque::new(), single: false };
        Self { program, routines: vec![main], prompts: VecDeque::new(), unlocked: Vec::new() }
    }

    /// What is on screen waiting for an answer.
    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompts.front().map(|(_, prompt)| prompt)
    }

    /// Answers the prompt on screen (`choice` only matters for a menu).
    pub fn answer(&mut self, choice: usize) {
        if let Some((index, prompt)) = self.prompts.pop_front() {
            let routine = &mut self.routines[index];
            if matches!(prompt, Prompt::Menu { .. }) {
                routine.vm.choose(choice);
            }
            routine.blocked = Blocked::No;
        }
    }

    /// The scene played inside this one is over.
    pub fn resume(&mut self) {
        for routine in &mut self.routines {
            if routine.blocked == Blocked::Scene {
                routine.blocked = Blocked::No;
            }
        }
    }

    /// Gives `who` an operation to carry out on its own: after those it
    /// was already handed and has not finished (a walk, then a turn), or in
    /// place of the routine its script gave it.
    /// `single`: it is one operation handed by another routine, not the
    /// routine the script attaches to the character.
    fn give(&mut self, who: Who, vm: Vm, single: bool) {
        let waiting = |r: &&mut Routine| single && r.who == Some(who) && r.single && !r.done;
        if let Some(busy) = self.routines.iter_mut().skip(1).find(waiting) {
            busy.handed.push_back(vm);
            return;
        }
        let routine = Routine { who: Some(who), vm, blocked: Blocked::No, done: false, handed: VecDeque::new(), single };
        // The scene's own routine keeps its place.
        match self.routines.iter().skip(1).position(|r| r.who == Some(who)) {
            Some(index) => {
                self.prompts.retain(|(asking, _)| *asking != index + 1);
                self.routines[index + 1] = routine;
            }
            None => self.routines.push(routine),
        }
    }

    /// Starts the routines the script attaches to its characters.
    fn start_attached(&mut self) {
        let script = self.program.script.clone();
        for (index, routine) in script.routines.iter().enumerate() {
            let who = match routine.kind {
                3 => Who::Actor(routine.target),
                4 => Who::Object(routine.target),
                5 => Who::Performer(routine.target),
                _ => continue,
            };
            if !self.routines.iter().any(|r| r.who == Some(who)) {
                let program = Program { routine: index, ..self.program.clone() };
                self.give(who, Vm::start(&program), false);
            }
        }
    }

    /// Runs one frame of every routine.
    pub fn step(&mut self, vars: &mut Vars, world: &mut dyn World) -> Outcome {
        let mut index = 0;
        while index < self.routines.len() {
            if let Some(outcome) = self.step_routine(index, vars, world) {
                return outcome;
            }
            index += 1;
        }
        match self.routines[0].done {
            true => Outcome::Done,
            false => Outcome::Running,
        }
    }

    fn step_routine(&mut self, index: usize, vars: &mut Vars, world: &mut dyn World) -> Option<Outcome> {
        if self.routines[index].done {
            return None;
        }
        let who = self.routines[index].who;
        // Is what the routine waits for over?
        let free = match self.routines[index].blocked {
            Blocked::No => true,
            Blocked::Frames(left) if left > 1 => {
                self.routines[index].blocked = Blocked::Frames(left - 1);
                false
            }
            Blocked::Frames(_) => true,
            Blocked::Busy => !world.busy(who),
            // (Another routine of that character: one that waits for the
            // character it is itself run for would wait for ever.)
            Blocked::On(Cond::Routine(other)) => {
                let others = self.routines.iter().enumerate().filter(|&(at, _)| at != index);
                // One that itself waits to be unlocked is not at work: the
                // game's scenes wait on such characters and go on
                // (`D03P41A/m03a0801`, whose partner is unlocked later).
                let at_work = |routine: &Routine| !routine.done && !matches!(routine.blocked, Blocked::On(Cond::Lock(_)));
                !others.into_iter().any(|(_, routine)| routine.who == Some(other) && at_work(routine))
            }
            Blocked::On(Cond::Fade) => !world.fading(),
            Blocked::On(Cond::Animation) => !world.animating(who),
            Blocked::On(Cond::Effect) => !world.effect(who),
            Blocked::On(Cond::Sound(id)) => !world.sounding(false, id),
            Blocked::On(Cond::Jingle(id)) => !world.sounding(true, id),
            Blocked::On(Cond::TopFade) => !world.top_fading(),
            Blocked::On(Cond::Music(around)) => !world.music_fading(around),
            Blocked::On(Cond::Lock(_)) | Blocked::Prompt | Blocked::Scene => false,
            Blocked::On(Cond::Locked(other, lock)) => self.routines.iter().any(|r| r.who == Some(other) && r.blocked == Blocked::On(Cond::Lock(lock))),
        };
        if !free {
            return None;
        }
        self.routines[index].blocked = Blocked::No;
        for _ in 0..FRAME_LIMIT {
            let program = &self.program;
            let event = self.routines[index].vm.run(program, vars, |below| world.random(below));
            let blocked = match event {
                Event::Message { speaker, text } => {
                    self.prompts.push_back((index, Prompt::Message { speaker, text }));
                    Blocked::Prompt
                }
                Event::Menu { options } => {
                    self.prompts.push_back((index, Prompt::Menu { options }));
                    Blocked::Prompt
                }
                Event::GameMenu(menu) => {
                    self.prompts.push_back((index, Prompt::GameMenu(menu)));
                    Blocked::Prompt
                }
                Event::Wait(frames) => Blocked::Frames(frames),
                Event::Lives(other, vm) => {
                    self.give(other, vm, true);
                    Blocked::No
                }
                // A lock already opened lets through whoever comes to it
                // next (`P04P01C/m10a1206`: who unlocks is there first).
                // Known gap: from that scene alone; how long the original
                // remembers an unlocking is not known.
                Event::WaitFor(Cond::Lock(lock)) if self.unlocked.contains(&lock) => {
                    self.unlocked.retain(|open| *open != lock);
                    Blocked::No
                }
                Event::WaitFor(cond) => Blocked::On(cond),
                Event::Unlock(lock) => {
                    let mut waited = false;
                    for routine in &mut self.routines {
                        if routine.blocked == Blocked::On(Cond::Lock(lock)) {
                            routine.blocked = Blocked::No;
                            waited = true;
                        }
                    }
                    if !waited && !self.unlocked.contains(&lock) {
                        self.unlocked.push(lock);
                    }
                    Blocked::No
                }
                Event::EnterGround(_) | Event::EnterDungeon(_) => {
                    return Some(Outcome::Leave(event));
                }
                Event::Done => {
                    // On to the next operation the character was handed.
                    match self.routines[index].handed.pop_front() {
                        Some(next) => {
                            self.routines[index].vm = next;
                            Blocked::No
                        }
                        None => {
                            self.routines[index].done = true;
                            return None;
                        }
                    }
                }
                Event::Act(Act::Scene { level, name, station, sub }) => {
                    // A scene that does not come back ends this one.
                    self.routines[index].blocked = Blocked::Scene;
                    self.routines[index].done = !sub;
                    return Some(Outcome::Scene { level, name, station, sub });
                }
                Event::Act(act) => {
                    world.act(who, &act);
                    match act {
                        Act::Move { .. } | Act::Turn { .. } | Act::Height { .. } => Blocked::Busy,
                        Act::Fade { wait: true, .. } => Blocked::On(Cond::Fade),
                        Act::TopFade { wait: true, .. } => Blocked::On(Cond::TopFade),
                        Act::Dim { wait: true, .. } => Blocked::On(Cond::Fade),
                        // What these bring is there on the next frame.
                        Act::Ground(_) | Act::Backdrop(_) | Act::LoadStation { .. } => Blocked::Frames(2),
                        Act::Layer { cast, .. } | Act::LayerHidden { cast, .. } => {
                            if cast == Cast::Acting {
                                self.start_attached();
                            }
                            Blocked::Frames(2)
                        }
                        _ => Blocked::No,
                    }
                }
            };
            if blocked != Blocked::No {
                self.routines[index].blocked = blocked;
                return None;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::tests::script;
    use crate::vm::{Look, Place};

    /// A world that notes what happens; moves take three frames.
    #[derive(Default)]
    struct Notes {
        acts: Vec<(Option<Who>, Act)>,
        moving: Vec<(Option<Who>, u32)>,
        fade: u32,
    }

    impl World for Notes {
        fn act(&mut self, who: Option<Who>, act: &Act) {
            match act {
                Act::Move { .. } | Act::Turn { .. } => self.moving.push((who, 3)),
                Act::Fade { .. } => self.fade = 3,
                _ => {}
            }
            self.acts.push((who, act.clone()));
        }
        fn busy(&self, who: Option<Who>) -> bool {
            self.moving.iter().any(|&(mover, left)| mover == who && left > 0)
        }
        fn fading(&self) -> bool {
            self.fade > 0
        }
        fn animating(&self, _: Option<Who>) -> bool {
            false
        }
        fn random(&mut self, _: i32) -> i32 {
            0
        }
    }

    impl Notes {
        fn tick(&mut self) {
            self.fade = self.fade.saturating_sub(1);
            for (_, left) in &mut self.moving {
                *left = left.saturating_sub(1);
            }
        }
    }

    #[test]
    fn characters_act_on_their_own_and_are_waited_for() {
        // The scene sends actor 7 walking, talks at once, then waits for it.
        let program = script(
            &[
                ("lives", &[7]),
                ("MovePositionOffset", &[2, 16, 0]),
                ("lives", &[8]),
                ("SetDirection", &[5]),
                ("message_Talk", &[0]),
                ("WaitExecuteLives", &[7]),
                ("message_Talk", &[1]),
                ("End", &[]),
            ],
            &["while walking", "arrived"],
        );
        let mut stage = Acting::new(program, None);
        let (mut vars, mut world) = (Vars::default(), Notes::default());
        assert_eq!(stage.step(&mut vars, &mut world), Outcome::Running);
        let text = |stage: &Acting| match stage.prompt() {
            Some(Prompt::Message { text, .. }) => text.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(text(&stage), "while walking");
        let moving = Act::Move { speed: 2.0, path: vec![Place::By(16.0, 0.0)], slide: false };
        assert_eq!(world.acts, [(Some(Who::Actor(7)), moving), (Some(Who::Actor(8)), Act::Direction(Look::Direction(6))),]);
        // Answered at once: the scene still waits for the walk to end.
        stage.answer(0);
        world.tick();
        assert_eq!(stage.step(&mut vars, &mut world), Outcome::Running);
        assert_eq!(stage.prompt(), None);
        for _ in 0..3 {
            world.tick();
            stage.step(&mut vars, &mut world);
        }
        assert_eq!(text(&stage), "arrived");
        stage.answer(0);
        assert_eq!(stage.step(&mut vars, &mut world), Outcome::Done);
    }

    #[test]
    fn a_character_takes_what_it_is_handed_in_turn() {
        // A walk then a turn for the same actor, handed one after the
        // other: the turn waits for the walk, and the scene for both.
        let program = script(
            &[
                ("lives", &[7]),
                ("MovePositionOffset", &[2, 16, 0]),
                ("lives", &[7]),
                ("Turn2Direction", &[4, 10, 4]),
                ("WaitExecuteLives", &[7]),
                ("message_Talk", &[0]),
                ("End", &[]),
            ],
            &["there"],
        );
        let mut stage = Acting::new(program, None);
        let (mut vars, mut world) = (Vars::default(), Notes::default());
        stage.step(&mut vars, &mut world);
        assert_eq!(world.acts.len(), 1, "{:?}", world.acts);
        assert!(matches!(world.acts[0], (Some(Who::Actor(7)), Act::Move { .. })));
        // The walk over (three frames), the turn starts.
        for _ in 0..3 {
            assert_eq!(world.acts.len(), 1);
            world.tick();
            stage.step(&mut vars, &mut world);
        }
        assert_eq!(world.acts.len(), 2, "{:?}", world.acts);
        assert!(matches!(world.acts[1], (Some(Who::Actor(7)), Act::Turn { .. })));
        assert_eq!(stage.prompt(), None);
        // The turn over, the scene goes on.
        for _ in 0..4 {
            world.tick();
            stage.step(&mut vars, &mut world);
        }
        assert!(matches!(stage.prompt(), Some(Prompt::Message { .. })), "{:?}", stage.prompt());
        assert_eq!(world.acts.len(), 2);
    }

    #[test]
    fn a_handed_operation_takes_the_place_of_a_characters_own_routine() {
        // Actor 7 has a routine of its own that never ends; an operation
        // handed to it is carried out all the same.
        let mut stage = Acting::new(script(&[("Wait", &[1]), ("lives", &[7]), ("SetDirection", &[5]), ("End", &[])], &[]), None);
        let own = script(&[("Wait", &[1000]), ("End", &[])], &[]);
        stage.give(Who::Actor(7), Vm::start(&own), false);
        let (mut vars, mut world) = (Vars::default(), Notes::default());
        for _ in 0..4 {
            stage.step(&mut vars, &mut world);
        }
        assert_eq!(world.acts, [(Some(Who::Actor(7)), Act::Direction(Look::Direction(6)))]);
    }

    #[test]
    fn fades_locks_and_inner_scenes_hold_the_routine() {
        let program = script(
            &[
                ("screen_FadeOut", &[1, 30]),
                ("lives", &[3]),
                ("Lock", &[4]),
                ("WaitLockLives", &[4, 3]),
                ("Unlock", &[4]),
                ("WaitExecuteLives", &[3]),
                ("supervision_ExecuteActingSub", &[9, 0, 0]),
                ("flag_Set", &[5, 1]),
                ("End", &[]),
            ],
            &[],
        );
        let mut stage = Acting::new(program, None);
        let (mut vars, mut world) = (Vars::default(), Notes::default());
        // The fade holds the scene for three frames.
        for _ in 0..3 {
            assert_eq!(stage.step(&mut vars, &mut world), Outcome::Running);
            assert_eq!(world.acts.len(), 1);
            world.tick();
        }
        // Then the actor locks, the scene sees it, unlocks it, and asks
        // for the inner scene.
        let mut outcome = Outcome::Running;
        for _ in 0..4 {
            outcome = stage.step(&mut vars, &mut world);
            if outcome != Outcome::Running {
                break;
            }
        }
        let inner = Outcome::Scene { level: 9, name: String::new(), station: false, sub: true };
        assert_eq!(outcome, inner);
        assert_eq!(stage.step(&mut vars, &mut world), Outcome::Running);
        assert_eq!(vars.get(5), 0);
        stage.resume();
        assert_eq!(stage.step(&mut vars, &mut world), Outcome::Done);
        assert_eq!(vars.get(5), 1);
    }
}
