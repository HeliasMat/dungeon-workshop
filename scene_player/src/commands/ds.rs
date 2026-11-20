//! `scene_player export-ds <film> --out <file.exps>`: a film as a cutscene
//! of the original game, written in ExplorerScript (SkyTemple's language for
//! Explorers of Sky's scripts). `tools/ds/patch_rom.py` compiles it and puts
//! it in place of a scene of the user's own ROM.
//!
//! It reads what the film has the engine play, its typed beats
//! (`Written::events`: places, Pokémon, emotes and sounds are already the
//! game's numbers), and writes each beat as the game's operations. First,
//! each line's word is looked up in `WORDS`: a word that has no Sky
//! operation yet is an error, never dropped in silence. A few beats only our
//! engine needs (the ground grid, its light, the title) are left out, and told.
//!
//! The game's names (levels, actors) are read from the data folder at run
//! time; nothing of the game is written here.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use ::film::plan;
use ::film::tables::game_file;

use engine::{Beat, BodyBeat, Course, Direction, FlowBeat, MotionBeat, Num, PlaceBeat, Point, ScreenBeat, SoundBeat, SpeechBeat, Spot, ViewBeat, Who};

use super::Piece;
use film::Context;

/// How a word of the language exports to the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Export {
    /// Written as the game's own operations, as the film plays it.
    Direct,
    /// Written, with less than the film shows (said in the table).
    Close,
    /// The game can do it; the exporter does not write it yet.
    Work,
    /// The game has no operation for it: it needs a patch of the game's code.
    None,
}

/// Each word of the core families (`plan::WORDS`), and the game's
/// operations it becomes.
pub const WORDS: &[(&str, Export, &str)] = &[
    // Structure
    ("title", Export::Close, "nothing: the title is the film's, not the scene's"),
    ("scene", Export::Direct, "back_SetGround(LEVEL_…), supervision_Acting(0), camera_SetPositionMark; who stands there goes in the scene's .ssa"),
    ("moment", Export::Work, "a @label (moments are jumped to with `go`)"),
    ("transition", Export::Direct, "screen_FadeIn / screen_FadeOut (fade); screen_WhiteIn / WhiteOut (white); others: Work"),
    ("banner", Export::Work, "back_SetBanner2 / message_Monologue"),
    ("define", Export::Direct, "nothing: recipes are written out before the export"),
    ("repeat", Export::Work, "a while loop on a $LOCAL variable"),
    ("if", Export::Work, "if ($VAR …) on the game's variables"),
    ("else", Export::Work, "else"),
    ("end", Export::Direct, "nothing (closes a block)"),
    ("choice", Export::Work, "message_SwitchMenu with case blocks"),
    ("set", Export::Work, "$SCENARIO_… / a free game variable"),
    // Speech
    ("(line)", Export::Direct, "message_SetFace(ACTOR, FACE_…, FACE_POS_STANDARD), message_Talk, message_Close"),
    ("think", Export::Work, "message_Monologue"),
    ("narrate", Export::Work, "message_ResetActor, message_Narration / message_Explanation"),
    ("box", Export::Work, "message_SetFace position modes"),
    ("explain", Export::Work, "message_Explanation (a recipe of lines)"),
    ("whisper", Export::Work, "message_Talk in small text ([FS:…])"),
    // Presence
    ("put", Export::Direct, "the actor's place and direction in the .ssa; later: SetPositionMark, Turn2Direction"),
    ("spot", Export::Direct, "nothing: a named point, read into positions"),
    ("enter", Export::Work, "SetPositionMark off screen, MovePositionMark"),
    ("exit", Export::Work, "MovePositionMark off screen, Destroy"),
    ("appear", Export::Work, "SetEffect smoke / SetOutputAttribute"),
    ("vanish", Export::Work, "SetEffect smoke, Destroy / SetOutputAttribute"),
    ("crowd", Export::None, "the scene's actors are a fixed list; a crowd needs many free actors"),
    // Formations
    ("form", Export::Work, "one MovePositionMark each"),
    // Movement
    ("walk", Export::Direct, "MovePositionMark<actor>(1, Position), WaitExecuteLives"),
    ("run", Export::Direct, "MovePositionMark<actor>(2, Position), WaitExecuteLives"),
    ("shift", Export::Work, "SlidePositionOffset"),
    ("jump", Export::Work, "JumpPositionMark / ExecuteCommon(CORO_JUMP_…)"),
    ("hop", Export::Direct, "ExecuteCommon<actor>(CORO_JUMP_HAPPY|SURPRISE|ANGRY_FUNC_SERIES, 0), WaitExecuteLives"),
    ("follow", Export::None, "no following in scripts: each step written as a move"),
    ("chase", Export::None, "no chasing in scripts"),
    ("flee", Export::Work, "MovePositionMark away"),
    ("wander", Export::None, "only ground NPCs wander, not actors of a cutscene"),
    // Turning
    ("face", Export::Direct, "Turn2DirectionLives<actor>(4, 10, OTHER) / Turn2Direction(4, 10, DIR_…)"),
    ("look", Export::Work, "Turn2Direction both ways, Wait"),
    ("nod", Export::Work, "SetAnimation of the nod / ExecuteCommon(CORO_…)"),
    ("shake_head", Export::Work, "ExecuteCommon<actor>(CORO_HEAD_SHAKE_FUNC_SERIES, 0)"),
    // Body
    ("pose", Export::Work, "SetAnimation<actor>(…): the game's animation numbers differ from the sprite's"),
    ("sleep", Export::Work, "SetAnimation sleep"),
    ("eat", Export::Work, "SetEffect(EFFECT_EATING)"),
    ("hurt", Export::Work, "SetAnimation hurt, se_Play"),
    ("tremble", Export::Work, "camera / SetAnimation"),
    ("fall", Export::Work, "SetAnimation"),
    ("faint", Export::Work, "SetAnimation"),
    ("cower", Export::Work, "a recipe: its words, once they export"),
    ("hug", Export::Work, "a recipe: its words, once they export"),
    ("comfort", Export::Work, "a recipe: its words, once they export"),
    ("cheer", Export::Work, "a recipe: its words, once they export"),
    // Emotions
    ("emote", Export::Direct, "SetEffect<actor>(EFFECT_…, 3): the engine's emote numbers are the game's; EFFECT_NONE once the line is said"),
    ("react", Export::Work, "a recipe: its words, once they export"),
    ("sigh", Export::Work, "a recipe: its words, once they export"),
    // Camera
    ("frame", Export::Work, "camera_SetPositionMark / camera_SetMyself"),
    ("glide", Export::Work, "camera_MovePositionMark"),
    ("shake", Export::Work, "camera_SetEffect"),
    // Screen
    ("flash", Export::Work, "screen_FlushOut / screen_WhiteOut"),
    ("memory", Export::Work, "screen_FadeChange / a recipe"),
    // Sound
    ("music", Export::Direct, "bgm_Play(n) / bgm_FadeOut: the engine's song numbers are the game's"),
    ("sound", Export::Direct, "se_Play(n): the engine's numbers are the game's"),
    ("jingle", Export::Work, "me_Play"),
    // Effects
    ("effect", Export::Work, "the game's effects by number (SetEffect / effect animations)"),
    ("weather", Export::Work, "the ground's weather effects (few places have them)"),
    ("daytime", Export::None, "the game draws evening as another level, not a tint"),
    ("status", Export::None, "no status icons in scripts"),
    // Objects
    ("hold", Export::None, "no held item on an actor; an object actor placed by hand"),
    ("drop", Export::None, "no held item on an actor"),
    ("give", Export::None, "no held item on an actor"),
    ("throw", Export::None, "no thrown item in scripts"),
    ("open", Export::Work, "SetAnimation<object>"),
    ("close", Export::Work, "SetAnimation<object>"),
    // Fights
    ("fight", Export::None, "the game fights in dungeons only: main_EnterDungeon to a boss floor"),
    ("attack", Export::None, "no moves in ground scripts"),
    ("evolve", Export::Work, "the evolution scene of the game"),
    ("join", Export::Work, "the game's join functions (party)"),
    // Time
    ("wait", Export::Direct, "Wait(frames) / WaitExecuteLives"),
    ("together", Export::Work, "operations on several actors, then their waits"),
    ("mark", Export::Work, "positions and frame counts kept by the exporter"),
    ("at", Export::Work, "positions and frame counts kept by the exporter"),
    ("pace", Export::Direct, "nothing: the waits are written already paced"),
    // Story
    ("option", Export::Work, "message_SwitchMenu"),
    ("{if}", Export::Work, "message_SwitchTalk on a variable"),
    ("{value}", Export::Work, "[value:…] in the text"),
    ("go", Export::Work, "jump @label / main_EnterGround"),
    ("seen()", Export::Work, "a game variable counted"),
    ("#id", Export::Direct, "nothing: an id for translators"),
    ("keep", Export::Work, "a game variable"),
];

/// The families the export table covers: the core ones (not the optional
/// nor the advanced).
fn core_words() -> impl Iterator<Item = &'static plan::Planned> {
    plan::WORDS.iter().filter(|word| plan::FAMILIES.contains(&word.family))
}

fn export_of(word: &str) -> Option<Export> {
    WORDS.iter().find(|(name, _, _)| *name == word).map(|(_, export, _)| *export)
}

/// `export-ds <film> --out <file.exps>`; `export-ds --table`: the table.
pub fn export(context: &Context, name: &str, out: Option<&String>, table: bool) -> Result<(), String> {
    if table || name.is_empty() {
        print!("{}", told_table());
        return Ok(());
    }
    let piece = Piece::find(context, name)?;
    let written = piece.written.as_ref().ok_or(format!("{name}: a film only"))?;
    if !written.problems.is_empty() {
        return Err(format!("{name}: {} problems; scene_player check {name} first", written.problems.len()));
    }
    let game = Game::read(&piece.tables.data);
    let mut writer = Writer::new(&game);
    for (number, read) in &written.read_as {
        match export_of(&read.word) {
            Some(Export::Direct | Export::Close) => {}
            Some(_) => writer.problems.push(format!("line {number}: “{}” doesn't export to DS yet", read.word)),
            None => writer.problems.push(format!("line {number}: “{}” (not a core word) doesn't export to DS yet", read.word)),
        }
    }
    for (number, read) in &written.read_as {
        let words = read.values.iter().flat_map(|value| value.words.iter().cloned()).collect();
        writer.words.insert(*number as i32, words);
    }
    // The typed beats the engine plays, each after the line it is of.
    let mut number = 0;
    for scene in written.events.iter().flat_map(|event| &event.scenes) {
        for beat in &scene.beats {
            if let Beat::Flow(FlowBeat::Line { number: line, .. }) = beat {
                number = *line;
            }
            let word = usize::try_from(number).ok().and_then(|line| written.read_as.get(&line)).map(|read| read.word.as_str());
            writer.beat(number, word, beat);
        }
    }
    if !writer.problems.is_empty() {
        return Err(format!("{name} doesn't export to DS:\n  {}", writer.problems.join("\n  ")));
    }
    let text = writer.finish(&piece.name);
    for note in &writer.notes {
        eprintln!("  left out: {note}");
    }
    let out = out.cloned().unwrap_or_else(|| format!("target/ds/{}.exps", piece.name));
    if let Some(folder) = std::path::Path::new(&out).parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    std::fs::write(&out, text).map_err(|error| format!("{out}: {error}"))?;
    println!("{out}: {} actors, place {}", writer.actors.len(), writer.level.as_deref().unwrap_or("?"));
    Ok(())
}

/// The table, and how many core words map directly, closely, need work, or can't.
fn told_table() -> String {
    let mut out = String::new();
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for word in core_words() {
        let export = export_of(word.word);
        let kind = match export {
            Some(Export::Direct) => "direct",
            Some(Export::Close) => "close",
            Some(Export::Work) => "work",
            Some(Export::None) => "none",
            None => "missing",
        };
        let how = WORDS.iter().find(|(name, _, _)| *name == word.word).map_or("", |(_, _, how)| how);
        let _ = writeln!(out, "{:<11} {:<12} {:<7} {how}", word.family, word.word, kind);
        let count = counts.entry(kind).or_default();
        count.0 += 1;
        if word.made == plan::Made::Word {
            count.1 += 1;
        }
    }
    let _ = writeln!(out);
    for (kind, (all, words)) in counts {
        let _ = writeln!(out, "{kind}: {all} ({words} of them words, the rest syntax and recipes)");
    }
    out
}

/// What the exporter reads of the game's data: its levels and actors.
struct Game {
    levels: HashMap<u16, String>,
    /// The script actors: name, species.
    actors: Vec<(String, u16)>,
}

impl Game {
    fn read(data: &str) -> Game {
        let rows = |file: &str| -> Vec<Vec<String>> {
            std::fs::read_to_string(game_file(data, &format!("rom/{file}")))
                .unwrap_or_default()
                .lines()
                .map(|line| line.split('\t').map(str::to_string).collect())
                .collect()
        };
        let levels = rows("levels.txt").into_iter().filter_map(|row| Some((row.first()?.parse().ok()?, row.get(1)?.clone()))).collect();
        let actors = rows("entities.txt").into_iter().filter_map(|row| Some((row.get(1)?.clone(), row.get(2)?.parse().ok()?))).collect();
        Game { levels, actors }
    }
}

/// An actor of the scene: its name in the game, the species given to it
/// (a free test actor), where it stands first.
struct Actor {
    name: String,
    species: Option<u16>,
    first: Option<(f32, f32, &'static str)>,
}

struct Writer<'a> {
    game: &'a Game,
    body: String,
    actors: BTreeMap<Who, Actor>,
    /// The places the film found (`PlaceBeat::Find`), by their name.
    places: HashMap<String, (f32, f32)>,
    /// Where each one stands now.
    at: HashMap<Who, (f32, f32)>,
    level: Option<String>,
    middle: Option<(f32, f32)>,
    started: bool,
    faded_out: bool,
    marks: usize,
    talking: bool,
    /// Those whose emote still shows: the game loops some (joy, laughing) until told none.
    showing: Vec<String>,
    /// The words each line of the film was given, by its number.
    words: HashMap<i32, Vec<String>>,
    problems: Vec<String>,
    notes: Vec<String>,
}

/// The test actors the game never uses, given the film's species.
const FREE_ACTORS: [&str; 10] =
    ["NPC_TEST001", "NPC_TEST002", "NPC_TEST003", "NPC_TEST004", "NPC_TEST005", "NPC_TEST006", "NPC_TEST007", "NPC_TEST008", "NPC_TEST009", "NPC_TEST010"];

/// The portraits' names (FACE_…), by number.
const FACES: [&str; 14] =
    ["NORMAL", "HAPPY", "PAIN", "ANGRY", "WORRIED", "SAD", "CRYING", "SHOUTING", "TEARY_EYED", "DETERMINED", "JOYOUS", "INSPIRED", "SURPRISED", "DIZZY"];

/// The game's effects by the number the engine gives them (the same as
/// the game's own), named as SkyTemple names them; others by number.
const EFFECTS: [(u16, &str); 10] = [
    (0, "EFFECT_NONE"),
    (88, "EFFECT_EXCLAMATION_MARK"),
    (89, "EFFECT_QUESTION_MARK"),
    (90, "EFFECT_JOYOUS"),
    (91, "EFFECT_SHOCKED"),
    (92, "EFFECT_SWEAT_DROP"),
    (93, "EFFECT_LAUGHING"),
    (94, "EFFECT_ANGRY"),
    (171, "EFFECT_SWEAT_DROPS_FROM_BOTH_SIDES_MEDIUM"),
    (172, "EFFECT_TWO_ARROWS_AT_SIDE_LEFT"),
];

fn direction(way: Direction) -> &'static str {
    match way {
        Direction::Down => "DIR_DOWN",
        Direction::DownRight => "DIR_DOWNRIGHT",
        Direction::Right => "DIR_RIGHT",
        Direction::UpRight => "DIR_UPRIGHT",
        Direction::Up => "DIR_UP",
        Direction::UpLeft => "DIR_UPLEFT",
        Direction::Left => "DIR_LEFT",
        Direction::DownLeft => "DIR_DOWNLEFT",
    }
}

/// Pixels of the level, in the game's tiles of 8 (halves kept).
fn tiles(pixels: f32) -> String {
    let halves = (pixels / 4.0).round() / 2.0;
    if halves.fract() == 0.0 { format!("{halves:.0}") } else { format!("{halves:.1}") }
}

/// Beside someone, so far (the engine's own distance between two at rest).
const BESIDE: f32 = 24.0;

impl<'a> Writer<'a> {
    fn new(game: &'a Game) -> Self {
        Writer {
            game,
            body: String::new(),
            actors: BTreeMap::new(),
            places: HashMap::new(),
            at: HashMap::new(),
            level: None,
            middle: None,
            started: false,
            faded_out: false,
            marks: 0,
            talking: false,
            showing: Vec::new(),
            words: HashMap::new(),
            problems: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn line(&mut self, text: &str) {
        let _ = writeln!(self.body, "    {text}");
    }

    /// Their emotes put away, once the line is said or before they move.
    fn calm(&mut self) {
        for actor in std::mem::take(&mut self.showing) {
            self.line(&format!("SetEffect<actor {actor}>(EFFECT_NONE, 3);"));
        }
    }

    fn position(&mut self, (x, y): (f32, f32)) -> String {
        self.marks += 1;
        format!("Position<'m{}', {}, {}>", self.marks, tiles(x), tiles(y))
    }

    /// The game's actor for someone of the film: the hero and the partner
    /// as the game has them, a species by the game's own actor of it, else
    /// a free test actor given that species.
    fn actor(&mut self, who: Who) -> Option<String> {
        if let Some(actor) = self.actors.get(&who) {
            return Some(format!("ACTOR_{}", actor.name));
        }
        let (name, species) = match who {
            Who::Character(number) => (self.game.actors.get(usize::from(number))?.0.clone(), None),
            Who::Monster(species) => {
                let own = self.game.actors.iter().find(|(name, of)| *of == species && name.starts_with("NPC_") && !name.starts_with("NPC_TEST"));
                match own {
                    Some((name, _)) => (name.clone(), None),
                    None => {
                        let taken = self.actors.values().filter(|actor| actor.species.is_some()).count();
                        (FREE_ACTORS.get(taken)?.to_string(), Some(species))
                    }
                }
            }
            Who::Another { species, .. } => {
                let taken = self.actors.values().filter(|actor| actor.species.is_some()).count();
                (FREE_ACTORS.get(taken)?.to_string(), Some(species))
            }
            _ => return None,
        };
        self.actors.insert(who, Actor { name: name.clone(), species, first: None });
        Some(format!("ACTOR_{name}"))
    }

    fn who(&mut self, number: i32, who: Who) -> Option<String> {
        let actor = self.actor(who);
        if actor.is_none() {
            self.problems.push(format!("line {number}: “{who:?}” has no actor in the game (only Pokémon, and {} new species at most)", FREE_ACTORS.len()));
        }
        actor
    }

    /// A number, written or found by the film before.
    fn num(&self, num: &Num) -> Option<f32> {
        match num {
            Num::Lit(value) => Some(*value),
            Num::Var(name) => {
                let (place, axis) = name.rsplit_once('_')?;
                let (x, y) = *self.places.get(place)?;
                match axis {
                    "x" => Some(x),
                    "y" => Some(y),
                    _ => None,
                }
            }
        }
    }

    fn point(&self, point: &Point) -> Option<(f32, f32)> {
        Some((self.num(&point.x)?, self.num(&point.y)?))
    }

    /// Where a spot is, as the export knows it before the game plays.
    fn spot(&self, spot: &Spot, mover: Option<Who>) -> Option<(f32, f32)> {
        match spot {
            Spot::Written(point) | Spot::Word(point) | Spot::Exact(point) | Spot::Near(point) => self.point(point),
            Spot::Of(other) => self.at.get(other).copied(),
            Spot::Side { of, right } => {
                let (x, y) = *self.at.get(of)?;
                let _ = mover;
                Some((if *right { x + BESIDE } else { x - BESIDE }, y))
            }
            _ => None,
        }
    }

    fn beat(&mut self, number: i32, word: Option<&str>, beat: &Beat) {
        if self.talking && !matches!(beat, Beat::Speech(SpeechBeat::Said) | Beat::Flow(FlowBeat::Wait(_))) {
            self.talking = false;
        }
        match beat {
            // What only our engine needs: its grid, its light, how it spaces
            // and turns people by itself, the sizes of the bodies.
            Beat::Flow(FlowBeat::Label(_) | FlowBeat::Line { .. })
            | Beat::View(ViewBeat::Body { .. } | ViewBeat::KeepSpeaker(_))
            | Beat::Place(PlaceBeat::Spread { .. })
            | Beat::Screen(ScreenBeat::Grey(_) | ScreenBeat::Weather(_))
            | Beat::Motion(MotionBeat::Spacing(_))
            | Beat::Speech(SpeechBeat::AutoFace(_) | SpeechBeat::Audience(_)) => {}
            // The scene's light as it opens (the game draws evening as another level).
            Beat::Screen(ScreenBeat::Tint { .. }) if !self.started => {}
            Beat::Screen(ScreenBeat::Backdrop(level, ..)) => match self.game.levels.get(level).cloned() {
                Some(level) => {
                    let level = format!("LEVEL_{level}");
                    self.line(&format!("back_SetGround({level});"));
                    self.line("supervision_Acting(0);");
                    self.level = Some(level);
                }
                None => self.problems.push(format!("line {number}: no level {level} in the game's data")),
            },
            Beat::Place(PlaceBeat::Ground { middle, .. }) => {
                self.middle = Some((middle.x, middle.y));
                let position = self.position((middle.x, middle.y));
                self.line(&format!("camera_SetPositionMark({position});"));
            }
            Beat::Sound(SoundBeat::Music(Some(song))) => self.line(&format!("bgm_Play({});", song.0)),
            Beat::Sound(SoundBeat::Music(None)) => self.line("bgm_FadeOut(30);"),
            Beat::Sound(SoundBeat::Sound(sound)) => self.line(&format!("se_Play({});", sound.0)),
            Beat::Place(PlaceBeat::Find { var, who, spot, .. }) => match self.spot(spot, *who) {
                Some(point) => {
                    self.places.insert(var.clone(), point);
                }
                None => self.problems.push(format!("line {number}: the spot {spot:?} doesn't export to DS yet")),
            },
            Beat::Body(BodyBeat::Put { who, at, direction: way }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let Some(point) = self.point(at) else {
                    self.problems.push(format!("line {number}: where {who:?} is put is not known before the film plays"));
                    return;
                };
                self.at.insert(*who, point);
                let facing = direction(*way);
                if self.started {
                    let position = self.position(point);
                    self.line(&format!("SetPositionMark<actor {actor}>({position});"));
                    self.line(&format!("SetDirection<actor {actor}>({facing});"));
                } else if let Some(one) = self.actors.get_mut(who) {
                    one.first = Some((point.0, point.1, facing));
                }
            }
            Beat::Body(BodyBeat::Turn { who, direction: way }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let facing = direction(*way);
                let started = self.started;
                match self.actors.get_mut(who).filter(|_| !started).and_then(|one| one.first.as_mut()) {
                    Some(first) => first.2 = facing,
                    None => {
                        self.line(&format!("Turn2Direction<actor {actor}>(4, 10, {facing});"));
                        self.line(&format!("WaitExecuteLives({actor});"));
                    }
                }
            }
            Beat::Body(BodyBeat::Face { who, other }) => {
                let (Some(actor), Some(other)) = (self.who(number, *who), self.who(number, *other)) else { return };
                self.line(&format!("Turn2DirectionLives<actor {actor}>(4, 10, {other});"));
                self.line(&format!("WaitExecuteLives({actor});"));
            }
            Beat::Place(PlaceBeat::Goes { who, points, gait }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let speed = if gait.run { 2 } else { 1 };
                self.calm();
                for point in points {
                    let Some(point) = self.point(point) else {
                        self.problems.push(format!("line {number}: where {who:?} goes is not known before the film plays"));
                        return;
                    };
                    self.at.insert(*who, point);
                    let position = self.position(point);
                    self.line(&format!("MovePositionMark<actor {actor}>({speed}, {position});"));
                }
            }
            Beat::Flow(FlowBeat::WaitFor(who) | FlowBeat::WaitMoves(who)) => {
                if let Some(actor) = self.who(number, *who) {
                    self.line(&format!("WaitExecuteLives({actor});"));
                }
            }
            Beat::Body(BodyBeat::Effect { who, id, .. }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let effect = EFFECTS.iter().find(|(of, _)| *of == id.0).map_or(id.0.to_string(), |(_, name)| name.to_string());
                self.line(&format!("SetEffect<actor {actor}>({effect}, 3);"));
                self.showing.push(actor);
            }
            Beat::Body(BodyBeat::Jump { who, .. }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let mood = match word {
                    Some("hop") if self.said_with(number, "surprised") => "SURPRISE",
                    Some("hop") if self.said_with(number, "angry") => "ANGRY",
                    _ => "HAPPY",
                };
                self.line(&format!("ExecuteCommon<actor {actor}>(CORO_JUMP_{mood}_FUNC_SERIES, 0);"));
                self.line(&format!("WaitExecuteLives({actor});"));
            }
            Beat::Course(Course::Say { entity, face, text, .. }) => {
                match entity {
                    None => self.line("message_ResetActor();"),
                    Some(who) => {
                        let Some(actor) = self.who(number, *who) else { return };
                        let face = face.unwrap_or(0);
                        let face = FACES.get(usize::from(face)).map_or(face.to_string(), |name| format!("FACE_{name}"));
                        self.line(&format!("message_SetFace({actor}, {face}, FACE_POS_STANDARD);"));
                    }
                }
                self.line(&format!("message_Talk(\" {}\");", text.trim().replace('"', "\\\"")));
                self.talking = true;
            }
            Beat::Speech(SpeechBeat::Said) => {
                self.line("message_Close();");
                self.calm();
            }
            Beat::Flow(FlowBeat::Wait(frames)) => {
                // Before the scene shows: the title's time, left out with it;
                // the engine's pauses after a line said: the box closing.
                if self.started && !(self.talking && frames.0 < 15.0) {
                    self.line(&format!("Wait({});", frames.0.round() as i32));
                }
            }
            Beat::Screen(ScreenBeat::Fade { out: false, white, frames }) => {
                self.started = true;
                self.faded_out = false;
                let fade = if *white { "screen_WhiteIn" } else { "screen_FadeIn" };
                self.line(&format!("{fade}(1, {});", frames.0.round() as i32));
            }
            Beat::Screen(ScreenBeat::Fade { out: true, white, frames }) => {
                if !self.faded_out {
                    let fade = if *white { "screen_WhiteOut" } else { "screen_FadeOut" };
                    self.line(&format!("{fade}(1, {});", frames.0.round() as i32));
                }
                self.faded_out = true;
            }
            Beat::Flow(FlowBeat::WaitFade) => self.line("WaitScreenFade();"),
            Beat::Screen(ScreenBeat::Caption { .. }) | Beat::Body(BodyBeat::Anim { .. }) => self.notes.push(format!("line {number}: {beat:?}")),
            other => self.problems.push(format!("line {number}: the engine's {other:?} doesn't export to DS yet ({})", word.unwrap_or("a line said"))),
        }
    }

    /// Whether the line's word was given that word (`hop surprised`).
    fn said_with(&self, number: i32, said: &str) -> bool {
        self.words.get(&number).is_some_and(|words| words.iter().any(|word| word == said))
    }

    /// The whole script: who stands where for `patch_rom.py`, then `def 0`.
    fn finish(&mut self, film: &str) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "// The film “{film}”, exported by `scene_player export-ds` for tools/ds/patch_rom.py.");
        let _ = writeln!(out, "// ds-export level {}", self.level.as_deref().unwrap_or("?"));
        let middle = self.middle.unwrap_or((128.0, 96.0));
        for actor in self.actors.values() {
            let (x, y, facing) = actor.first.unwrap_or((middle.0, middle.1, "DIR_DOWN"));
            let species = actor.species.map_or(String::new(), |species| format!(" species {species}"));
            let _ = writeln!(out, "// ds-export actor ACTOR_{}{species} x {} y {} facing {facing}", actor.name, tiles(x), tiles(y));
        }
        let _ = writeln!(out, "def 0 {{");
        out.push_str(&self.body);
        if !self.faded_out {
            out.push_str("    screen_FadeOut(1, 30);\n    WaitScreenFade();\n");
        }
        out.push_str("    end;\n}\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_core_word_has_its_line_in_the_export_table() {
        let missing: Vec<&str> = core_words().map(|word| word.word).filter(|word| export_of(word).is_none()).collect();
        assert!(missing.is_empty(), "no DS export line for {missing:?}");
    }

    #[test]
    fn pixels_become_tiles_with_halves() {
        assert_eq!(tiles(400.0), "50");
        assert_eq!(tiles(205.0), "25.5");
        assert_eq!(tiles(336.0), "42");
    }
}
