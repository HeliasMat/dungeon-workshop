//! The story read down its events, and what a routine shows and says.

use rom_formats::vm::{self, Act, Event, Program, Vars, Vm};

use super::rom_data::RomData;
use super::text;

/// The original's shared routines that are events of the story.
const EVENTS: std::ops::RangeInclusive<usize> = 78..=630;
/// Operations a routine may run before it is taken to go round in circles.
const LIMIT: usize = 50_000;
/// The story's first scene.
const OPENING: &str = "m00a01a";
/// Lines a scene may hold.
const LONGEST: usize = 600;

/// What a routine shows and says, and the scenes it plays inside itself.
/// At a question, every answer's way on is written in turn (`answer
/// <number>` each, then `end`): the routine is followed to its end down
/// each of them.
pub(super) fn read(program: &Program, vm: Vm, data: &RomData, out: &mut Vec<String>) -> Vec<(u16, String)> {
    read_at(program, vm, data, out, &fresh(vm::NO_EPISODE)).0
}

/// The story's variables as it starts, in the main story (`NO_EPISODE`)
/// or in one of the special episodes.
fn fresh(episode: i32) -> Vars {
    let mut vars = Vars::default();
    // (No dungeon is chosen: a routine that leaves for "the one chosen"
    // leads nowhere, instead of to the first of the list.)
    vars.set(vm::DUNGEON_SELECT, -1);
    vars.set(vm::EXECUTE_SPECIAL_EPISODE_TYPE, episode);
    vars
}

/// How the scripts mark a dungeon open to jobs alone.
const JOBS_ONLY: i32 = 2;

/// The scripts' variable saying which special episodes are open.
const SPECIAL_EPISODE_OPEN: u16 = 75;

/// A moment of the story: its chapter and step, as the scripts count
/// them, and the story's variables then.
pub type Moment = ((i32, i32), Vars);

/// The special episode an event belongs to, by its name: Bidoof's (0),
/// Igglybuff's (1), Sunflora's (2), Team Charm's (3), the future's (4), as
/// the original's routine picking their events has them
/// (`EVENT_DIVIDE_SPECIAL_EPISODE`); the main story otherwise.
fn episode(name: &str) -> i32 {
    match name.get(..9) {
        Some("EVENT_N01") => 0,
        Some("EVENT_N02" | "EVENT_N03") => 1,
        Some("EVENT_N04") => 2,
        Some("EVENT_N06") => 3,
        Some("EVENT_N08" | "EVENT_N09") => 4,
        _ => vm::NO_EPISODE,
    }
}

/// The same from a state of the story's variables; and the state the
/// routine leaves them in.
pub(super) fn read_at(program: &Program, vm: Vm, data: &RomData, out: &mut Vec<String>, from: &Vars) -> (Vec<(u16, String)>, Vars) {
    let mut vars = from.clone();
    let mut scenes = Vec::new();
    walk(program, vm, &mut vars, data, out, &mut scenes, &mut Vec::new(), LONGEST);
    (scenes, vars)
}

/// Questions inside questions followed down every answer; lines the way
/// on of another answer than the first may hold.
const DEEPEST: usize = 3;
const SIDE: usize = 120;

/// `asked`: the questions met on the way here, as the routine stood at
/// each; `longest`: how many lines it may still write.
fn walk(
    program: &Program,
    mut vm: Vm,
    vars: &mut Vars,
    data: &RomData,
    out: &mut Vec<String>,
    scenes: &mut Vec<(u16, String)>,
    asked: &mut Vec<Vm>,
    longest: usize,
) {
    let start = out.len();
    let depth = asked.len();
    // Questions answered in passing, whose `end` is still to write.
    let mut open = 0;
    for _ in 0..LIMIT {
        match vm.run(program, vars, |_| 0) {
            Event::Message { speaker, text: said } => {
                let who = speaker.map_or("-\t-".to_string(), |face| format!("{}\t{}", face.entity, face.face.map_or("-".to_string(), |face| face.to_string())));
                out.push(format!("say\t{who}\t{}", text(&data.script_text(&said))));
            }
            Event::Menu { options } => {
                // (A question come back to: the routine goes round, enough.)
                if asked.contains(&vm) {
                    break;
                }
                let answers: Vec<String> = options.iter().map(|option| text(&data.script_text(option))).collect();
                out.push(format!("ask\t{}", answers.join("|")));
                // (Deeper, the first answer alone, in passing.)
                if depth >= DEEPEST {
                    out.push("answer\t0".to_string());
                    asked.push(vm.clone());
                    vm.choose(0);
                    open += 1;
                    continue;
                }
                for way in 0..options.len().max(1) {
                    out.push(format!("answer\t{way}"));
                    let mut branch = vm.clone();
                    branch.choose(way);
                    // What the story becomes is what its first answers make it.
                    let (mut state, mut seen, mut path) = (vars.clone(), scenes.clone(), asked.clone());
                    path.push(vm.clone());
                    // (The first answer's way is the story's: as long as
                    // it may be. The others': the shorter the deeper.)
                    let left = longest.saturating_sub(out.len() - start);
                    let room = if way == 0 { left } else { left.min(SIDE >> depth) };
                    walk(program, branch, &mut state, data, out, &mut seen, &mut path, room);
                    if way == 0 {
                        (*vars, *scenes) = (state, seen);
                    }
                }
                out.push("end".to_string());
                return;
            }
            Event::Act(Act::Ground(level)) | Event::Act(Act::Backdrop(level)) => out.push(format!("backdrop\t{level}")),
            // (A scene come back to: the routine goes round, enough.)
            Event::Act(Act::Scene { level, name, station: false, .. }) => {
                // (An event over hands the story back to the routine
                // that picks the next one: with the story's variables at
                // zero, that is the story's very first scene. Not this
                // event's.)
                if scenes.contains(&(level, name.clone())) || (name == OPENING && !scenes.is_empty()) {
                    break;
                }
                scenes.push((level, name));
            }
            Event::Act(Act::Scene { level, station: true, .. }) | Event::EnterGround(level) => {
                out.push(format!("ground\t{level}"));
                break;
            }
            Event::EnterDungeon(dungeon) => {
                out.push(format!("dungeon\t{dungeon}"));
                break;
            }
            Event::GameMenu(menu) => out.push(format!("menu\t{menu}")),
            Event::Done => break,
            _ => {}
        }
        // (A scene that says the same over and over: enough too.)
        if out.len() - start > longest {
            break;
        }
    }
    out.extend((0..open).map(|_| "end".to_string()));
}

/// Writes the story to `<folder>/story.txt`; how many events, scenes and
/// lines it holds.
/// Also the moments of the main story: chapter and step, and the story's
/// variables as its events have left them by then.
pub fn story(data: &RomData, folder: &str) -> std::io::Result<(usize, usize, usize, Vec<Moment>)> {
    let mut moments: Vec<Moment> = Vec::new();
    // What the main story's events have set, one after the other.
    let mut state = fresh(vm::NO_EPISODE);
    let mut lines = Vec::new();
    let (mut events, mut scenes, mut said) = (0, 0, 0);
    // (The routines go by their numbers: no names are known for them.)
    let names: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    for number in EVENTS {
        let Some(program) = data.common_routine(number) else {
            continue;
        };
        let name = names.get(&number).map_or("", String::as_str);
        // (The town's passing scenes aside, `EVENT_M00_...`: the story
        // before its end, then what follows it.)
        let main = (name.starts_with("EVENT_M") && !name.starts_with("EVENT_M00_")) || name.starts_with("EVENT_S");
        let start = fresh(episode(name));
        // Played as the original plays it, and what is seen written down
        // (see `record`).
        let (shown, after) = super::record::event(data, &program, Vm::start_common(&program, number), &start, LONGEST * 80);
        if !shown.iter().any(|line| line.starts_with("say") || line.starts_with("scene")) {
            continue;
        }
        let moment = after.scenario(vm::SCENARIO_MAIN);
        events += 1;
        scenes += shown.iter().filter(|line| line.starts_with("scene")).count();
        said += shown.iter().filter(|line| line.starts_with("say")).count();
        let header = lines.len();
        lines.push(String::new());
        // (Where the event leads comes last: what it opens goes before.)
        let leads = shown.last().filter(|line| line.starts_with("dungeon") || line.starts_with("ground")).cloned();
        let shown = &shown[..shown.len() - leads.is_some() as usize];
        lines.extend(shown.iter().cloned());
        // (`open <number>`: the event opens a special episode; `unlock
        // <dungeon>`: a dungeon to visits.)
        let opened = (0..5).filter(|&episode| after.get_at(SPECIAL_EPISODE_OPEN, episode) != 0);
        lines.extend(opened.map(|episode| format!("open\t{episode}")));
        let unlocked = vm::open_dungeons(&after).into_iter().filter(|&(_, mode)| mode != JOBS_ONLY);
        lines.extend(unlocked.map(|(dungeon, _)| format!("unlock\t{dungeon}")));
        lines.extend(leads);
        let set = [after];
        // (With the moment of the story the event leaves it at: chapter
        // and step, as the scripts count them; 0 0 when it does not say.)
        lines[header] = format!("event\t{number}\t{name}\t{}\t{}", moment.0, moment.1);
        if main {
            set.iter().for_each(|set| state.merge(set));
            // (Several events at one moment: the last one's state.)
            let now = state.scenario(vm::SCENARIO_MAIN);
            moments.retain(|known| known.0 != now);
            if now != (0, 0) {
                moments.push((now, state.clone()));
            }
        }
    }
    moments.sort_by_key(|moment| moment.0);
    std::fs::create_dir_all(folder)?;
    std::fs::write(format!("{folder}/story.txt"), lines.join("\n") + "\n")?;
    Ok((events, scenes, said, moments))
}
