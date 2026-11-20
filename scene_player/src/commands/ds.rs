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

use engine::{
    Beat, BodyBeat, Cam, Compare, Course, CrowdShape, DataBeat, Direction, Exit, FlowBeat, Formation, MotionBeat, Num, Pace, PlaceBeat, Play, Point, Pose,
    ScreenBeat, SoundBeat, SoundOf, SpeechBeat, Spot, Test, Value, ViewBeat, Weather, Who,
};

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
    ("title", Export::Direct, "back_SetTitleBanner(56, title): alone on black before the scene, as Sky titles its story"),
    (
        "scene",
        Export::Direct,
        "back_SetGround(LEVEL_…), supervision_Acting(0), camera_SetMyself<performer 0>; the top screen's picture as Sky pairs it with the place (back2_SetMode(4), back2_SetGround), else black as most of Sky's scenes; who stands there goes in the .ssa",
    ),
    ("moment", Export::Direct, "a @label"),
    (
        "transition",
        Export::Close,
        "fade: screen_FadeIn/Out; white: screen_WhiteOut, then screen_FadeIn back (as Sky); boss: CallCommon(CORO_BOSS_WIPE_FUNC), Sky's own; wipe and iris: a fade (Sky has neither)",
    ),
    ("banner", Export::Close, "message_Narration on the black screen (Sky's chapter banners need a picture of their own and say “Chapter n”)"),
    ("define", Export::Direct, "nothing: recipes are written out before the export"),
    ("repeat", Export::Direct, "nothing: its turns are written out before the export"),
    ("if", Export::Direct, "if ( $LOCALn … ) { }: a film's variables are the scene's four local ones"),
    ("else", Export::Direct, "} else {"),
    ("end", Export::Direct, "nothing (closes a block)"),
    ("choice", Export::Direct, "switch ( message_SwitchMenu(0, 1) ) { case menu(\"…\"): … }, in the box of the line before"),
    ("set", Export::Direct, "$LOCALn = value (the scene's own; lost once it ends)"),
    // Speech
    (
        "(line)",
        Export::Direct,
        "message_SetFace(ACTOR, FACE_…, FACE_POS_STANDARD) (a face the species lacks: its normal one; none: message_SetActor, its name alone), message_Talk, message_Close",
    ),
    ("think", Export::Direct, "message_SetFace, message_Monologue"),
    ("narrate", Export::Direct, "box: message_ResetActor, message_Talk; screen: message_Narration; sign: message_Explanation"),
    ("box", Export::Close, "top: the portrait on top (FACE_POS_TOP_L_FACEINW); Sky keeps the box at the bottom"),
    ("explain", Export::Direct, "ExecuteCommon(CORO_EXPLANATION_FUNC_SERIES), Sky's own"),
    ("whisper", Export::Close, "its words: walks over, an emote; no smaller text"),
    // Presence
    ("put", Export::Direct, "the actor's place and direction in the .ssa; later: SetPositionMark, SetDirection"),
    ("spot", Export::Direct, "nothing: a named point, read into positions"),
    ("enter", Export::Direct, "SetPositionMark off screen, ResetOutputAttribute(128) (hidden until then), MovePositionMark"),
    ("exit", Export::Direct, "MovePositionMark off screen, SetOutputAttribute(128) (hidden)"),
    ("appear", Export::Close, "smoke: SetEffect(99, smoke puff); shown with ResetOutputAttribute(128): at once, no fading in"),
    ("vanish", Export::Close, "smoke: SetEffect(99); hidden with SetOutputAttribute(128): at once, no fading out"),
    ("crowd", Export::Close, "a free actor each (45 a film, 24 a scene), round the place facing it; no fidgeting"),
    // Formations
    ("form", Export::Direct, "one MovePositionMark each, then their waits"),
    // Movement
    (
        "walk",
        Export::Direct,
        "MovePositionMark<actor>(1, Position) (slow 0.5, brisk faster), WaitExecuteLives; via: a wait at each point; backward: SlidePositionMark",
    ),
    ("run", Export::Direct, "MovePositionMark<actor>(2, Position), WaitExecuteLives"),
    ("shift", Export::Direct, "SlidePositionMark<actor>(speed, Position)"),
    ("jump", Export::Close, "MoveHeight up and down; to a place: a run there (no arc)"),
    ("hop", Export::Direct, "ExecuteCommon<actor>(CORO_JUMP_HAPPY|SURPRISE|ANGRY_FUNC_SERIES, 0), WaitExecuteLives"),
    ("follow", Export::Close, "after each move of the leader: Wait(16), MovePositionLives<follower>(speed, LEADER) (tools/ds/LIMITS.md)"),
    ("chase", Export::Close, "MovePositionLives<actor>(3, OTHER): runs to where the other is, once"),
    ("flee", Export::Direct, "MovePositionMark away, at a run"),
    ("wander", Export::Direct, "ExecuteCommon<actor>(CORO_MOVE_RANDOM, 0), Sky's own stroll; off: back to its place"),
    // Turning
    ("face", Export::Direct, "Turn2DirectionLives<actor>(4, 10, OTHER) / Turn2Direction(4, 10, DIR_…)"),
    ("look", Export::Direct, "ExecuteCommon(CORO_LOOK_AROUND[_LEFT|_RIGHT|_DOWN]_FUNC_SERIES), Sky's own; up: Turn2Direction"),
    ("nod", Export::Direct, "SetAnimation (its nod, once), then 2"),
    ("shake_head", Export::Direct, "ExecuteCommon(CORO_HEAD_SHAKE_FUNC_SERIES), Sky's own"),
    // Body
    (
        "pose",
        Export::Close,
        "SetAnimation<actor>(n): the sprite's group through Sky's table (once or looped); off: 2. Some species' ground sprites order their groups otherwise",
    ),
    ("sleep", Export::Direct, "SetAnimation(62), as Sky's INIT_SLEEP; off: its waking, once (63)"),
    ("eat", Export::Direct, "SetAnimation (eat), SetEffect(EFFECT_EATING); no item shown"),
    ("hurt", Export::Close, "its pain animation, once; no knock back"),
    ("tremble", Export::Close, "its pain animation, looped: Sky has no shiver"),
    ("fall", Export::Direct, "SetAnimation (tumble), once"),
    ("faint", Export::Direct, "SetAnimation (faint), once; off: waking"),
    ("cower", Export::Close, "its words: cringe, pain looped, an emote"),
    ("hug", Export::Close, "its words: walks beside the other, both hop"),
    ("comfort", Export::Direct, "its words: walks beside, nods, the other cheered"),
    ("cheer", Export::Direct, "its words: hops, pose"),
    // Emotions
    ("emote", Export::Direct, "SetEffect<actor>(EFFECT_…, 3): the engine's emote numbers are the game's; EFFECT_NONE once the line is said"),
    ("react", Export::Direct, "its words: an emote, a hop"),
    ("sigh", Export::Direct, "its words: a deep breath, sweat"),
    // Camera
    ("frame", Export::Direct, "MovePositionMark<performer 0> (the camera, camera_SetMyself) to between them; snap: SetPositionMark"),
    ("glide", Export::Direct, "MovePositionMark<performer 0>(speed, Position), WaitExecutePerformer(0)"),
    ("shake", Export::Direct, "camera_SetEffect(2, 1, 1.0) soft, (2, 2, 3.0) hard, (0, 0, 0) off, as Sky's"),
    // Screen
    ("flash", Export::Direct, "white: screen_WhiteOut(1, 2), screen_FadeIn; a colour: screen_FlushOut / FlushIn(1, n, 2, r, g, b)"),
    ("memory", Export::Direct, "screen_FlushOut(1, 0, 8, 0, 0, 0): Sky's own sepia of memories; screen_FlushIn(1, 0, 1, …) after"),
    // Sound
    ("music", Export::Direct, "bgm_Play(n) / bgm_PlayFadeIn(n, d, 256) / bgm_FadeOut(d): the engine's song numbers are the game's"),
    ("sound", Export::Close, "se_Play(n): the engine's numbers are the game's; on someone: not placed; a move's own sound: none"),
    ("jingle", Export::Direct, "me_Play(n)"),
    // Effects
    ("effect", Export::Close, "SetEffect<actor>(n), WaitEffect: a script effect of the move's type (fire, water, lightning…), not the move's own"),
    ("weather", Export::Close, "SetEffect<performer 3>(rain 16 / hail 20; snow as hail); sand, fog, sun…: none in scripts"),
    (
        "daytime",
        Export::Close,
        "screen_FlushOut(1, d, 4, 255, 128, 64) evening, (…, 6, 0, 32, 128) night; day: screen_FlushIn; the light's own colour is lost; as a scene opens, the place's own level gives it",
    ),
    (
        "status",
        Export::Close,
        "an effect on the actor: confusion's stars (17), poison bubbles (1), Zs (51), ice (23), stats up (691) / down (685); others none",
    ),
    // Objects
    ("hold", Export::None, "no held item on an actor; an object actor placed by hand"),
    ("drop", Export::None, "no held item on an actor"),
    ("give", Export::None, "no held item on an actor"),
    ("throw", Export::None, "no thrown item in scripts"),
    ("open", Export::Work, "SetAnimation<object>: needs the place's objects in the .ssa"),
    ("close", Export::Work, "SetAnimation<object>: needs the place's objects in the .ssa"),
    // Fights
    ("fight", Export::None, "the game fights in dungeons only: main_EnterDungeon to a boss floor"),
    (
        "attack",
        Export::Close,
        "SetAnimation(28) wind-up, (48) lunge, a script effect of the move's type on the target, a small shake: not the move's own animation (dungeons only)",
    ),
    ("evolve", Export::Work, "the evolution scene of the game"),
    ("join", Export::Work, "the game's join functions (party)"),
    // Time
    ("wait", Export::Direct, "Wait(frames) / WaitExecuteLives / all: each one moving, WaitExecutePerformer(0)"),
    ("together", Export::Direct, "each one's moves told at once (actors move on their own), then their WaitExecuteLives; its waits are lost"),
    ("mark", Export::Work, "positions and frame counts kept by the exporter"),
    ("at", Export::Work, "positions and frame counts kept by the exporter"),
    ("pace", Export::Direct, "nothing: the waits are written already paced"),
    // Story
    ("option", Export::Direct, "case menu(\"…\"):"),
    ("{if}", Export::Work, "message_SwitchTalk on a variable"),
    ("{value}", Export::Work, "[value:…] in the text"),
    ("go", Export::Close, "go moment: jump @label; go scene: one scene a film on DS"),
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
/// `--partial`: what doesn't export is left out (and told), not an error.
pub fn export(context: &Context, name: &str, out: Option<&String>, table: bool, partial: bool) -> Result<(), String> {
    if table || name.is_empty() {
        print!("{}", told_table());
        return Ok(());
    }
    let piece = Piece::find(context, name)?;
    let written = piece.written.as_ref().ok_or(format!("{name}: a film only"))?;
    if !written.problems.is_empty() {
        return Err(format!("{name}: {} problems; scene_player check {name} first", written.problems.len()));
    }
    let move_types = piece.art().map(|art| art.tables.move_slots.iter().map(|slot| slot.type_id).collect()).unwrap_or_default();
    let game = Game::read(&piece.tables.data, move_types);
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
            // (`DS_BEATS=1`: each beat as the export reads it.)
            if std::env::var_os("DS_BEATS").is_some() {
                eprintln!("{number:>4} {:<10} {beat:?}", word.unwrap_or(""));
            }
            writer.beat(number, word, beat);
        }
    }
    if partial {
        let problems = std::mem::take(&mut writer.problems);
        writer.notes.extend(problems);
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

/// What the exporter reads of the game's data: its levels, actors, which
/// portraits each species has, and the types of the moves.
struct Game {
    levels: HashMap<u16, String>,
    /// The script actors: name, species.
    actors: Vec<(String, u16)>,
    data: String,
    /// Each move's type (the game's numbers), by move.
    move_types: Vec<u8>,
}

impl Game {
    fn read(data: &str, move_types: Vec<u8>) -> Game {
        let rows = |file: &str| -> Vec<Vec<String>> {
            std::fs::read_to_string(game_file(data, &format!("rom/{file}")))
                .unwrap_or_default()
                .lines()
                .map(|line| line.split('\t').map(str::to_string).collect())
                .collect()
        };
        let levels = rows("levels.txt").into_iter().filter_map(|row| Some((row.first()?.parse().ok()?, row.get(1)?.clone()))).collect();
        let actors = rows("entities.txt").into_iter().filter_map(|row| Some((row.get(1)?.clone(), row.get(2)?.parse().ok()?))).collect();
        Game { levels, actors, data: data.to_string(), move_types }
    }

    /// Whether that species has that portrait (none known for 0: the hero
    /// and the partner, whose species the player chose).
    fn has_face(&self, species: u16, face: u16) -> bool {
        species == 0 || game_file(&self.data, &format!("rom/portraits/{species}_{face}.png")).exists()
    }
}

/// An actor of the scene: its name in the game, the species given to it
/// (a free test actor), its species, where it stands first, whether it
/// shows from the start.
struct Actor {
    name: String,
    species: Option<u16>,
    kind: u16,
    first: Option<(f32, f32, &'static str)>,
    hidden: bool,
}

/// The picture Sky shows on the top screen with a place, as its own
/// scenes pair them (`back2_SetMode(4)`, `back2_SetGround`); a place
/// without one keeps the top screen black, as most of Sky's scenes do.
const TOP_SCREENS: [(&str, &str); 22] = [
    ("D01P11A", "V01P03A"),
    ("D01P11B", "V01P03B"),
    ("D47P11A", "V17P01A"),
    ("D53P41C", "V38P06C"),
    ("D73P31A", "V24P08A"),
    ("G01P01B", "V02P01A"),
    ("G01P01B2", "V02P01A"),
    ("G01P01C", "V02P06A"),
    ("G01P05A", "V34P01A"),
    ("H01P99E", "V21P02B"),
    ("H02P99A", "V24P08A"),
    ("P03P02A", "V24P08A"),
    ("P07P01A", "V17P01A"),
    ("P12P01A", "V26P04A"),
    ("P16P01A", "V24P08A"),
    ("P17P01A", "V24P08A"),
    ("P21P02A", "V24P08A"),
    ("P23P02A", "V17P01A"),
    ("P27P01A", "V39P01A"),
    ("S02P01A", "S02P01A"),
    ("S04P01A", "S04P01A"),
    ("S17P02A", "S02P01A"),
];

/// The script variables a film's own may use (local to the scene).
const LOCALS: [&str; 4] = ["$LOCAL0", "$LOCAL1", "$LOCAL2", "$LOCAL3"];

/// The output attribute that hides an actor (`SetOutputAttribute<actor>(128)`).
const HIDDEN: u16 = 128;

/// The sprite's animation groups (the engine's `AnimId`) as Sky's
/// `SetAnimation` numbers (its table `SETANIMATION_TABLE`): played once
/// and kept, or looped.
fn animation(group: u16, looped: bool) -> Option<u16> {
    let once = match group {
        0 => Some(83),
        1 => Some(13),
        2 => Some(12),
        3 => Some(18),
        4 => Some(19),
        6 => Some(11),
        10 => Some(17),
        13..=32 => Some(22 + group - 13),
        33 => Some(81),
        34 => Some(82),
        _ => None,
    };
    let looping = match group {
        0 => Some(5),
        3 => Some(20),
        4 => Some(21),
        5 => Some(6),
        6 => Some(14),
        7 => Some(4),
        11 => Some(9),
        13..=32 => Some(42 + group - 13),
        _ => None,
    };
    if looped { looping.or(once) } else { once.or(looping) }
}

/// The script effect closest to a move, by its type (the game's numbers:
/// 1 Normal ... 17 Steel).
fn move_effect(kind: u8) -> u16 {
    match kind {
        2 => 6,    // Fire: FireAttack
        3 => 2,    // Water: WaterSplash
        4 => 126,  // Grass: VinesAttackingSpiral
        5 => 113,  // Electric: LightningStrike
        6 => 23,   // Ice: TrappedInIce
        8 => 1,    // Poison: PoisonBubble
        9 => 53,   // Ground: RocksFalling
        10 => 86,  // Flying: AirBlades
        11 => 105, // Psychic: LightColoredFocusing
        12 => 46,  // Bug: SpiderWeb
        13 => 54,  // Rock: RocksHitting
        14 => 63,  // Ghost: Curse
        15 => 57,  // Dragon: FireExplosion
        16 => 116, // Dark: BlackHandSlapping
        17 => 74,  // Steel: ExplosionSmall
        _ => 127,  // Normal, Fighting: PunchAttack
    }
}

struct Writer<'a> {
    game: &'a Game,
    body: String,
    actors: BTreeMap<Who, Actor>,
    /// The places the film found (`PlaceBeat::Find`), by their name.
    places: HashMap<String, (f32, f32)>,
    /// Where each one stands now, and where it faces.
    at: HashMap<Who, (f32, f32)>,
    facing: HashMap<Who, Direction>,
    level: Option<String>,
    top: Option<&'static str>,
    middle: Option<(f32, f32)>,
    /// Where the camera (performer 0) is.
    camera: (f32, f32),
    started: bool,
    faded_out: bool,
    marks: usize,
    talking: bool,
    /// The title, shown as Sky shows its story's title before the scene.
    title: Option<String>,
    /// Those whose emote still shows: the game loops some (joy, laughing) until told none.
    showing: Vec<String>,
    /// Those who played an animation once and stand again once the line is over.
    stand_after: Vec<String>,
    /// Those who were told to move and are not waited for yet.
    busy: Vec<String>,
    /// The line whose other beats Sky's own routine already plays.
    skip: Option<i32>,
    /// In a thread of `together`: the waits kept for its end.
    thread: bool,
    kept: Vec<String>,
    memory: bool,
    weather: bool,
    box_top: bool,
    /// Where the last line's box closed (to ask in it).
    closed: Option<usize>,
    /// The film's variables, as the game's.
    vars: Vec<String>,
    /// The film's actors, the same in every scene: name, species given, species.
    cast: BTreeMap<Who, (String, Option<u16>, u16)>,
    /// The scenes written before this one.
    blocks: Vec<String>,
    titled: bool,
    /// Who follows whom (`follow`): they walk after the leader's moves.
    followers: Vec<(Who, Who)>,
    /// The light the screen is tinted with (`screen_FlushOut`'s mode and colour).
    tint: Option<&'static str>,
    /// The choices open, and the answers each has had; the options of the last asked.
    menus: Vec<usize>,
    options: Vec<String>,
    indent: usize,
    /// After Sky's boss wipe: the whole screen white.
    white_all: bool,
    /// A caption said as narration, waited for by its key.
    said_over: bool,
    /// How long the emotes showing have been waited on.
    shown_for: f32,
    /// The words each line of the film was given, by its number.
    words: HashMap<i32, Vec<String>>,
    problems: Vec<String>,
    notes: Vec<String>,
}

/// The actors the game never uses, given the film's species: the ten test
/// actors, then placeholders `patch_rom.py` gives the other free ones (45 in all).
const FREE_ACTORS: [&str; 45] = [
    "NPC_TEST001",
    "NPC_TEST002",
    "NPC_TEST003",
    "NPC_TEST004",
    "NPC_TEST005",
    "NPC_TEST006",
    "NPC_TEST007",
    "NPC_TEST008",
    "NPC_TEST009",
    "NPC_TEST010",
    "FREE11",
    "FREE12",
    "FREE13",
    "FREE14",
    "FREE15",
    "FREE16",
    "FREE17",
    "FREE18",
    "FREE19",
    "FREE20",
    "FREE21",
    "FREE22",
    "FREE23",
    "FREE24",
    "FREE25",
    "FREE26",
    "FREE27",
    "FREE28",
    "FREE29",
    "FREE30",
    "FREE31",
    "FREE32",
    "FREE33",
    "FREE34",
    "FREE35",
    "FREE36",
    "FREE37",
    "FREE38",
    "FREE39",
    "FREE40",
    "FREE41",
    "FREE42",
    "FREE43",
    "FREE44",
    "FREE45",
];

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

/// The way from one point to another, of the eight.
fn way_to(from: (f32, f32), to: (f32, f32)) -> Direction {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    // Counter-clockwise from down, as the engine numbers them (y goes down).
    let angle = dx.atan2(dy).to_degrees();
    let eighth = ((angle + 360.0 + 22.5) / 45.0) as usize % 8;
    Direction::ALL[eighth]
}

/// Pixels of the level, in the game's tiles of 8 (halves kept).
fn tiles(pixels: f32) -> String {
    let halves = (pixels / 4.0).round() / 2.0;
    if halves.fract() == 0.0 { format!("{halves:.0}") } else { format!("{halves:.1}") }
}

/// A speed of the game's (pixels a frame, in 256ths).
fn speed(pixels: f32, frames: f32) -> String {
    let speed = (pixels / frames.max(1.0) * 256.0).round().max(16.0) / 256.0;
    let text = format!("{speed:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A text as the game's script writes it.
fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

/// Beside someone, so far (the engine's own distance between two at rest).
const BESIDE: f32 = 24.0;

/// How far from the camera an actor is out of sight (half the screen and more).
const OFF_SCREEN: f32 = 150.0;

impl<'a> Writer<'a> {
    fn new(game: &'a Game) -> Self {
        Writer {
            game,
            body: String::new(),
            actors: BTreeMap::new(),
            places: HashMap::new(),
            at: HashMap::new(),
            facing: HashMap::new(),
            level: None,
            top: None,
            middle: None,
            camera: (128.0, 96.0),
            started: false,
            faded_out: false,
            marks: 0,
            talking: false,
            title: None,
            showing: Vec::new(),
            stand_after: Vec::new(),
            busy: Vec::new(),
            skip: None,
            thread: false,
            kept: Vec::new(),
            memory: false,
            weather: false,
            box_top: false,
            closed: None,
            vars: Vec::new(),
            menus: Vec::new(),
            followers: Vec::new(),
            cast: BTreeMap::new(),
            blocks: Vec::new(),
            titled: false,
            tint: None,
            options: Vec::new(),
            indent: 1,
            white_all: false,
            said_over: false,
            shown_for: 0.0,
            words: HashMap::new(),
            problems: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn line(&mut self, text: &str) {
        let _ = writeln!(self.body, "{}{text}", "    ".repeat(self.indent));
    }

    /// Their emotes put away, once the line is said or before they move.
    fn calm(&mut self) {
        for actor in std::mem::take(&mut self.showing) {
            self.line(&format!("SetEffect<actor {actor}>(EFFECT_NONE, 3);"));
        }
    }

    /// Those who played an animation once stand again.
    fn stand(&mut self) {
        for actor in std::mem::take(&mut self.stand_after) {
            self.line(&format!("SetAnimation<actor {actor}>(2);"));
        }
    }

    fn position(&mut self, (x, y): (f32, f32)) -> String {
        self.marks += 1;
        format!("Position<'m{}', {}, {}>", self.marks, tiles(x), tiles(y))
    }

    /// Someone moving on alone, waited for later.
    fn moving(&mut self, actor: &str) {
        if !self.busy.iter().any(|one| one == actor) {
            self.busy.push(actor.to_string());
        }
    }

    /// The script waits for someone (in a thread of `together`: at its end).
    fn wait_for(&mut self, actor: &str) {
        if self.thread {
            if !self.kept.iter().any(|one| one == actor) {
                self.kept.push(actor.to_string());
            }
            return;
        }
        self.busy.retain(|one| one != actor);
        self.line(&format!("WaitExecuteLives({actor});"));
    }

    /// The game's actor for someone of the film: the hero and the partner
    /// as the game has them, a species by the game's own actor of it, else
    /// a free test actor given that species.
    fn actor(&mut self, who: Who) -> Option<String> {
        if let Some(actor) = self.actors.get(&who) {
            return Some(format!("ACTOR_{}", actor.name));
        }
        let free = |cast: &BTreeMap<Who, (String, Option<u16>, u16)>| {
            FREE_ACTORS.get(cast.values().filter(|one| one.1.is_some()).count()).map(|name| name.to_string())
        };
        let (name, species, kind) = match who {
            _ if self.cast.contains_key(&who) => self.cast[&who].clone(),
            Who::Character(number) => {
                let (name, kind) = self.game.actors.get(usize::from(number))?.clone();
                (name, None, kind)
            }
            Who::Monster(species) => {
                let own = self.game.actors.iter().find(|(name, of)| *of == species && name.starts_with("NPC_") && !name.starts_with("NPC_TEST"));
                match own {
                    Some((name, _)) => (name.clone(), None, species),
                    None => (free(&self.cast)?, Some(species), species),
                }
            }
            Who::Another { species, .. } => (free(&self.cast)?, Some(species), species),
            _ => return None,
        };
        self.cast.insert(who, (name.clone(), species, kind));
        // Who comes in after the scene opens is hidden until then.
        let hidden = self.started;
        self.actors.insert(who, Actor { name: name.clone(), species, kind, first: None, hidden });
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
    fn spot(&self, spot: &Spot) -> Option<(f32, f32)> {
        match spot {
            Spot::Written(point) | Spot::Word(point) | Spot::Exact(point) | Spot::Near(point) => self.point(point),
            Spot::Of(other) => self.at.get(other).copied(),
            Spot::Side { of, right } => {
                let (x, y) = *self.at.get(of)?;
                Some((if *right { x + BESIDE } else { x - BESIDE }, y))
            }
            Spot::Beside(of) => {
                let (x, y) = *self.at.get(of)?;
                Some((x - BESIDE, y))
            }
            Spot::By { of, dx, dy } => {
                let (x, y) = *self.at.get(of)?;
                Some((x + dx, y + dy))
            }
            Spot::Tiles { of, count } => {
                let (x, y) = *self.at.get(of)?;
                Some((x + count * 8.0, y))
            }
            Spot::Between(a, b) => {
                let (a, b) = (*self.at.get(a)?, *self.at.get(b)?);
                Some(((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0))
            }
            // A formation round a point, spaced as the engine spaces two at rest.
            Spot::Shape { kind, who, at, round } => {
                let (x, y) = self.spot(round)?;
                let many = who.len() as f32;
                let along = (*at as f32 - (many - 1.0) / 2.0) * BESIDE;
                Some(match kind {
                    Formation::Line | Formation::Facing => (x + along, y),
                    Formation::Column => (x, y + along),
                    Formation::Triangle => [(x, y - 16.0), (x - BESIDE, y + 12.0), (x + BESIDE, y + 12.0)].get(*at).copied()?,
                    Formation::Ring => {
                        let radius = (many * BESIDE / std::f32::consts::TAU).max(BESIDE);
                        let angle = std::f32::consts::TAU * *at as f32 / many;
                        (x + angle.sin() * radius, y - angle.cos() * radius)
                    }
                })
            }
            _ => None,
        }
    }

    /// Where the camera looks at, from the film's.
    fn cam(&self, cam: &Cam) -> Option<(f32, f32)> {
        match cam {
            Cam::At(point) => self.point(point),
            Cam::Who(who) => self.at.get(who).copied(),
        }
    }

    /// The camera (performer 0) taken there, in so many frames (none: at once).
    fn camera_to(&mut self, to: (f32, f32), frames: f32) {
        let distance = ((to.0 - self.camera.0).powi(2) + (to.1 - self.camera.1).powi(2)).sqrt();
        self.camera = to;
        let position = self.position(to);
        if frames <= 1.0 || distance < 2.0 || !self.started {
            self.line(&format!("SetPositionMark<performer 0>({position});"));
        } else {
            self.line(&format!("MovePositionMark<performer 0>({}, {position});", speed(distance, frames)));
        }
    }

    /// Someone walks (`MovePositionMark`, turned the way it goes) or
    /// slides (`SlidePositionMark`, facing as it was) there, at that speed.
    fn go(&mut self, who: Who, actor: &str, to: (f32, f32), pace: &str, slide: bool) {
        let from = self.at.get(&who).copied();
        self.at.insert(who, to);
        if let (Some(from), false) = (from, slide) {
            self.facing.insert(who, way_to(from, to));
        }
        let position = self.position(to);
        let verb = if slide { "SlidePositionMark" } else { "MovePositionMark" };
        self.line(&format!("{verb}<actor {actor}>({pace}, {position});"));
        self.moving(actor);
    }

    /// A film's variable as one of the game's.
    fn var(&mut self, number: i32, name: &str) -> Option<&'static str> {
        let at = match self.vars.iter().position(|one| one == name) {
            Some(at) => at,
            None => {
                self.vars.push(name.to_string());
                self.vars.len() - 1
            }
        };
        let var = LOCALS.get(at).copied();
        if var.is_none() {
            self.problems.push(format!("line {number}: “{name}”: a scene has {} variables at most on DS", LOCALS.len()));
        }
        var
    }

    fn test(&mut self, number: i32, test: &Test) -> Option<String> {
        let mut any = Vec::new();
        for all in &test.any {
            let [condition] = all.as_slice() else {
                self.problems.push(format!("line {number}: “and” in a test doesn't export to DS (nest two ifs)"));
                return None;
            };
            let var = self.var(number, &condition.var)?;
            let value = match &condition.value {
                Value::Number(value) => *value as i64,
                Value::Var(_) => {
                    self.problems.push(format!("line {number}: a test against a variable doesn't export to DS"));
                    return None;
                }
            };
            let compare = match condition.compare {
                Compare::Equal => "==",
                Compare::Unequal => "!=",
                Compare::Less => "<",
                Compare::LessOrEqual => "<=",
                Compare::Greater => ">",
                Compare::GreaterOrEqual => ">=",
            };
            any.push(format!("{var} {compare} {value}"));
        }
        Some(any.join(" || "))
    }

    fn beat(&mut self, number: i32, word: Option<&str>, beat: &Beat) {
        if let Beat::Flow(FlowBeat::Line { number: line, .. }) = beat {
            self.stand();
            // An emote shown long enough goes once its line is over (Sky loops some).
            if self.shown_for >= 30.0 {
                self.calm();
            }
            if self.skip != Some(*line) {
                self.skip = None;
            }
            return;
        }
        if self.skip == Some(number) {
            return;
        }
        if self.talking && !matches!(beat, Beat::Speech(SpeechBeat::Said) | Beat::Flow(FlowBeat::Wait(_))) {
            self.talking = false;
        }
        if !matches!(beat, Beat::Course(Course::Ask(_)) | Beat::Flow(FlowBeat::Wait(_))) {
            self.closed = None;
        }
        if self.started && self.routine(number, word, beat) {
            self.skip = Some(number);
            return;
        }
        match beat {
            // What only our engine needs: its grid, its light, how it spaces
            // and turns people by itself, the sizes of the bodies.
            Beat::View(ViewBeat::Body { .. } | ViewBeat::KeepSpeaker(_) | ViewBeat::KeepFrame(_))
            | Beat::Place(PlaceBeat::Spread { .. })
            | Beat::Motion(MotionBeat::Spacing(_) | MotionBeat::Speed { .. })
            | Beat::Speech(SpeechBeat::AutoFace(_) | SpeechBeat::Audience(_))
            | Beat::Flow(FlowBeat::Parallel(_)) => {}
            Beat::Flow(FlowBeat::Label(label)) => self.label(label),
            Beat::Flow(FlowBeat::Goto(label)) if label.starts_with("thread") => {}
            Beat::Flow(FlowBeat::Goto(label)) => self.line(&format!("jump @{};", label.trim_start_matches('_'))),
            // How often a moment was seen: the film's own count, no `seen()` on DS.
            Beat::Data(DataBeat::Calc { var, .. }) if var.starts_with("seen_") => {}
            Beat::Data(DataBeat::Calc { var, a, op, b }) => {
                let by = match (a, b) {
                    (Value::Var(of), Value::Number(by)) if of == var && (*op == "+" || *op == "-") => Some(*by as i64),
                    _ => None,
                };
                let Some(name) = self.var(number, var) else { return };
                match by {
                    Some(by) => self.line(&format!("{name} {op}= {by};")),
                    None => self.problems.push(format!("line {number}: the calculation {a:?} {op} {b:?} doesn't export to DS (only adding to a variable)")),
                }
            }
            Beat::Flow(FlowBeat::Return) => self.thread = false,
            Beat::Data(DataBeat::Set { var, .. }) if var.starts_with("thread") => {}
            Beat::Flow(FlowBeat::WaitUntil(test)) if test.any.iter().flatten().all(|condition| condition.var.starts_with("thread")) => {
                for actor in std::mem::take(&mut self.kept) {
                    self.wait_for(&actor);
                }
            }
            Beat::Data(DataBeat::Set { var, value }) => {
                let Some(name) = self.var(number, var) else { return };
                match value {
                    Value::Number(value) => self.line(&format!("{name} = {};", *value as i64)),
                    Value::Var(_) => self.problems.push(format!("line {number}: a variable set from another doesn't export to DS")),
                }
            }
            Beat::Flow(FlowBeat::If(test)) => {
                if let Some(test) = self.test(number, test) {
                    self.line(&format!("if ( {test} ) {{"));
                    self.indent += 1;
                }
            }
            Beat::Flow(FlowBeat::ElseIf(test)) => {
                if let Some(test) = self.test(number, test) {
                    self.indent -= 1;
                    self.line(&format!("}} else if ( {test} ) {{"));
                    self.indent += 1;
                }
            }
            Beat::Flow(FlowBeat::Else) => {
                self.indent -= 1;
                self.line("} else {");
                self.indent += 1;
            }
            Beat::Flow(FlowBeat::EndIf) => {
                self.indent -= 1;
                self.line("}");
            }
            Beat::Course(Course::Ask(options)) => {
                // In the box of the question, as Sky asks.
                if let Some(at) = self.closed.take() {
                    self.body.truncate(at);
                }
                self.options.clone_from(options);
                self.line("switch ( message_SwitchMenu(0, 1) ) {");
                self.indent += 1;
                self.menus.push(0);
            }
            Beat::Course(Course::Answer(answer)) => {
                let Some(seen) = self.menus.last().copied() else { return };
                if seen > 0 {
                    self.line("break;");
                    self.indent -= 1;
                }
                let option = self.options.get(usize::from(*answer)).cloned().unwrap_or_default();
                self.line(&format!("case menu(\"{}\"):", quoted(&option)));
                self.indent += 1;
                if let Some(seen) = self.menus.last_mut() {
                    *seen += 1;
                }
            }
            Beat::Course(Course::Rejoin) => {
                let Some(seen) = self.menus.pop() else { return };
                if seen > 0 {
                    self.line("break;");
                    self.indent -= 1;
                }
                self.indent -= 1;
                self.line("}");
            }
            other => self.plain(number, word, other),
        }
    }

    /// A moment of the film (`moment: name`) as a label of the script; the
    /// scenes' and the threads' own are the export's.
    fn label(&mut self, label: &str) {
        // A scene of the film: an acting scene of its own (`// ds-export level`).
        if label.starts_with("scene") && self.level.is_some() {
            self.next_scene();
            return;
        }
        if let Some(name) = label.strip_prefix("thread") {
            self.thread = !name.ends_with("_past");
            return;
        }
        if label.starts_with("scene") {
            return;
        }
        self.line(&format!("@{};", label.trim_start_matches('_')));
    }

    /// The words Sky plays with a routine of its own (`unionall`'s
    /// coroutines): the rest of their line's beats are left to it.
    fn routine(&mut self, number: i32, word: Option<&str>, beat: &Beat) -> bool {
        let who = match beat {
            Beat::Body(BodyBeat::TurnBy { who, .. } | BodyBeat::Anim { who, .. } | BodyBeat::Effect { who, .. } | BodyBeat::Jump { who, .. }) => *who,
            // Eating, as Sky shows it: the eating animation and its effect, no item.
            Beat::Body(BodyBeat::PutNear { other, .. }) if word == Some("eat") => {
                let Some(actor) = self.who(number, *other) else { return true };
                if let Some(set) = animation(15, false) {
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

    fn plain(&mut self, number: i32, word: Option<&str>, beat: &Beat) {
        match beat {
            Beat::Screen(ScreenBeat::Grey(_)) if !self.started => {}
            // The light: Sky's screen_FlushOut, evening orange or night blue
            // (the memory's sepia is its own, below).
            // (As a scene opens, its place gives its light: Sky draws evening as a level of its own.)
            Beat::Screen(ScreenBeat::Tint { .. }) if !self.started => {}
            Beat::Screen(ScreenBeat::Tint { colour, frames }) if !self.memory => {
                let frames = if self.started { frames.0.round() as i32 } else { 0 };
                if colour[3] <= 0.0 {
                    if let Some(tint) = self.tint.take() {
                        self.line(&format!("screen_FlushIn(1, {frames}, {tint});"));
                    }
                    return;
                }
                let tint = if colour[2] > colour[0] { "6, 0, 32, 128" } else { "4, 255, 128, 64" };
                if let Some(old) = self.tint.take() {
                    self.line(&format!("screen_FlushIn(1, 0, {old});"));
                }
                self.line(&format!("screen_FlushOut(1, {frames}, {tint});"));
                self.tint = Some(tint);
            }
            Beat::Screen(ScreenBeat::Tint { .. }) => {}
            Beat::Screen(ScreenBeat::Grey(on)) => {
                if *on && !self.memory {
                    // Sky's memories and visions: the screen in sepia.
                    self.line("screen_FlushOut(1, 0, 8, 0, 0, 0);");
                } else if !*on && self.memory {
                    self.line("screen_FlushIn(1, 0, 1, 0, 0, 0);");
                }
                self.memory = *on;
            }
            Beat::Screen(ScreenBeat::Backdrop(level, ..)) => match self.game.levels.get(level).cloned() {
                Some(level) => {
                    self.top = TOP_SCREENS.iter().find(|(place, _)| *place == level).map(|(_, top)| *top);
                    if let Some(top) = self.top {
                        self.line("back2_SetMode(4);");
                        self.line(&format!("back2_SetGround(LEVEL_{top});"));
                    }
                    let level = format!("LEVEL_{level}");
                    self.line(&format!("back_SetGround({level});"));
                    self.line("supervision_Acting(0);");
                    self.level = Some(level);
                }
                None => self.problems.push(format!("line {number}: no level {level} in the game's data")),
            },
            Beat::Place(PlaceBeat::Ground { middle, .. }) => {
                self.middle = Some((middle.x, middle.y));
                self.camera = (middle.x, middle.y);
                self.line("camera_SetMyself<performer 0>();");
            }
            Beat::Place(PlaceBeat::Find { var, spot, .. }) => match self.spot(spot) {
                Some(point) => {
                    self.places.insert(var.clone(), point);
                }
                None => self.problems.push(format!("line {number}: the spot {spot:?} doesn't export to DS yet")),
            },
            // The camera: Sky's performer 0, which the camera follows.
            Beat::View(ViewBeat::Frame { who, at, frames, .. }) => {
                let mut points: Vec<(f32, f32)> = who.iter().filter_map(|one| self.at.get(one).copied()).collect();
                points.extend(at.iter().map(|point| (point.x, point.y)));
                if points.is_empty() {
                    self.problems.push(format!("line {number}: where to frame is not known before the film plays"));
                    return;
                }
                let count = points.len() as f32;
                let middle = (points.iter().map(|point| point.0).sum::<f32>() / count, points.iter().map(|point| point.1).sum::<f32>() / count);
                self.camera_to(middle, frames.0);
            }
            Beat::View(ViewBeat::Pan { to, frames } | ViewBeat::PanStays { to, frames }) => match self.cam(to) {
                Some(point) => self.camera_to(point, frames.0),
                None => self.problems.push(format!("line {number}: where the camera goes is not known before the film plays")),
            },
            Beat::View(ViewBeat::Camera(cam)) => {
                let to = cam.as_ref().and_then(|cam| self.cam(cam)).or(self.middle);
                if let Some(to) = to {
                    self.camera_to(to, 0.0);
                }
            }
            Beat::Flow(FlowBeat::WaitCam) => self.line("WaitExecutePerformer(0);"),
            // Sky's screen shakes (`camera_SetEffect`), soft and hard.
            Beat::View(ViewBeat::Shake { strength, .. }) => {
                let shake = if *strength <= 0.0 {
                    "0, 0, 0"
                } else if *strength <= 2.0 {
                    "2, 1, 1.0"
                } else {
                    "2, 2, 3.0"
                };
                self.line(&format!("camera_SetEffect({shake});"));
            }
            Beat::View(ViewBeat::Quake { frames, .. }) => {
                self.line("camera_SetEffect(2, 2, 3.0);");
                self.line(&format!("Wait({});", frames.0.round() as i32));
                self.line("camera_SetEffect(0, 0, 0);");
            }
            Beat::Sound(SoundBeat::Music(Some(song))) => self.line(&format!("bgm_Play({});", song.0)),
            Beat::Sound(SoundBeat::Music(None)) => self.line("bgm_FadeOut(30);"),
            Beat::Sound(SoundBeat::MusicIn { song: Some(song), frames }) => {
                self.line(&format!("bgm_PlayFadeIn({}, {}, 256);", song.0, frames.0.round() as i32))
            }
            Beat::Sound(SoundBeat::MusicIn { song: None, frames } | SoundBeat::MusicFade { frames, .. }) => {
                self.line(&format!("bgm_FadeOut({});", frames.0.round() as i32));
            }
            Beat::Sound(SoundBeat::Sound(sound) | SoundBeat::SoundFrom { id: sound, .. }) => self.line(&format!("se_Play({});", sound.0)),
            Beat::Sound(SoundBeat::OnHit { sound, .. }) => match sound.as_ref() {
                SoundBeat::Sound(sound) => self.line(&format!("se_Play({});", sound.0)),
                other => self.notes.push(format!("line {number}: {other:?} (a move's own sound plays in dungeons only)")),
            },
            Beat::Sound(SoundBeat::Jingle(jingle)) => self.line(&format!("me_Play({jingle});")),
            Beat::Flow(FlowBeat::WaitSound { of: SoundOf::Game(sound), .. }) => self.line(&format!("WaitSe({});", sound.0)),
            Beat::Body(BodyBeat::Put { who, at, direction: way }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let Some(point) = self.point(at) else {
                    self.problems.push(format!("line {number}: where {who:?} is put is not known before the film plays"));
                    return;
                };
                self.at.insert(*who, point);
                self.facing.insert(*who, *way);
                let facing = direction(*way);
                match self.actors.get_mut(who) {
                    Some(one) if !self.started || (one.hidden && one.first.is_none()) => one.first = Some((point.0, point.1, facing)),
                    _ => {
                        let position = self.position(point);
                        self.line(&format!("SetPositionMark<actor {actor}>({position});"));
                        self.line(&format!("SetDirection<actor {actor}>({facing});"));
                    }
                }
            }
            Beat::Body(BodyBeat::PutNear { who, other, side }) => {
                let (Some(actor), Some(&(x, y))) = (self.who(number, *who), self.at.get(other)) else { return };
                let right = matches!(side, Direction::Right | Direction::UpRight | Direction::DownRight);
                let point = (if right { x + BESIDE } else { x - BESIDE }, y);
                self.at.insert(*who, point);
                let position = self.position(point);
                self.line(&format!("SetPositionMark<actor {actor}>({position});"));
                let facing = way_to(point, (x, y));
                self.facing.insert(*who, facing);
                self.line(&format!("SetDirection<actor {actor}>({});", direction(facing)));
            }
            Beat::Body(BodyBeat::Turn { who, direction: way }) => {
                let Some(actor) = self.who(number, *who) else { return };
                self.facing.insert(*who, *way);
                let facing = direction(*way);
                let started = self.started;
                match self.actors.get_mut(who).filter(|_| !started).and_then(|one| one.first.as_mut()) {
                    Some(first) => first.2 = facing,
                    None => {
                        self.line(&format!("Turn2Direction<actor {actor}>(4, 10, {facing});"));
                        self.wait_for(&actor);
                    }
                }
            }
            Beat::Body(BodyBeat::TurnBy { who, eighths }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let now = self.facing.get(who).copied().unwrap_or(Direction::Down);
                let way = Direction::from_number(((now.number() as i32 - 1 + eighths.round() as i32).rem_euclid(8) + 1) as u16);
                self.facing.insert(*who, way);
                self.line(&format!("Turn2Direction<actor {actor}>(4, 10, {});", direction(way)));
            }
            Beat::Body(BodyBeat::TurnAround(who)) => {
                let Some(actor) = self.who(number, *who) else { return };
                let now = self.facing.get(who).copied().unwrap_or(Direction::Down);
                let way = Direction::from_number(((now.number() + 3) % 8) + 1);
                self.facing.insert(*who, way);
                self.line(&format!("SetDirection<actor {actor}>({});", direction(way)));
            }
            Beat::Body(BodyBeat::LookAt { who, at }) => {
                let (Some(actor), Some(to), Some(&from)) = (self.who(number, *who), self.point(at), self.at.get(who)) else { return };
                let way = way_to(from, to);
                self.facing.insert(*who, way);
                self.line(&format!("SetDirection<actor {actor}>({});", direction(way)));
            }
            Beat::Body(BodyBeat::Face { who, other }) => {
                let (Some(actor), Some(other_actor)) = (self.who(number, *who), self.who(number, *other)) else { return };
                if let (Some(&from), Some(&to)) = (self.at.get(who), self.at.get(other)) {
                    self.facing.insert(*who, way_to(from, to));
                }
                self.line(&format!("Turn2DirectionLives<actor {actor}>(4, 10, {other_actor});"));
                self.wait_for(&actor);
            }
            Beat::Place(PlaceBeat::Goes { who, points, gait }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let mut pace = if gait.run { 2.0 } else { 1.0 };
                if let Pace::Times(times) = gait.pace {
                    pace *= times;
                }
                self.calm();
                for point in points {
                    let Some(point) = self.point(point) else {
                        self.problems.push(format!("line {number}: where {who:?} goes is not known before the film plays"));
                        return;
                    };
                    self.go(*who, &actor, point, &speed(pace * 256.0, 256.0), false);
                    for follower in self.followers.iter().filter(|(_, leader)| leader == who).map(|(one, _)| *one).collect::<Vec<_>>() {
                        let Some(follower_actor) = self.who(number, follower) else { continue };
                        self.line("Wait(16);");
                        self.line(&format!("MovePositionLives<actor {follower_actor}>({}, {actor});", speed(pace * 256.0, 256.0)));
                        self.moving(&follower_actor);
                        let side = if self.at.get(&follower).is_some_and(|at| at.0 < point.0) { -BESIDE } else { BESIDE };
                        self.at.insert(follower, (point.0 + side, point.1));
                    }
                    // Sky waits for each step of a path before the next.
                    if points.len() > 1 {
                        self.wait_for(&actor);
                    }
                }
            }
            Beat::Motion(MotionBeat::Go { who, at, frames, slide }) => {
                let (Some(actor), Some(to)) = (self.who(number, *who), self.point(at)) else {
                    self.problems.push(format!("line {number}: where {who:?} goes is not known before the film plays"));
                    return;
                };
                let from = self.at.get(who).copied().unwrap_or(to);
                let distance = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
                self.go(*who, &actor, to, &speed(distance, frames.0), *slide);
            }
            // Following and chasing, as LIMITS found Sky can: the follower
            // walks to where the leader is (`MovePositionLives`) as it goes.
            Beat::Motion(MotionBeat::Follow { who, leader, .. }) => match leader {
                Some(leader) => {
                    self.followers.retain(|(one, _)| one != who);
                    self.followers.push((*who, *leader));
                }
                None => self.followers.retain(|(one, _)| one != who),
            },
            Beat::Motion(MotionBeat::Chase { who, other }) if word == Some("chase") => {
                let (Some(actor), Some(other_actor)) = (self.who(number, *who), self.who(number, *other)) else { return };
                self.line(&format!("MovePositionLives<actor {actor}>(3, {other_actor});"));
                self.moving(&actor);
                if let Some(&at) = self.at.get(other) {
                    self.at.insert(*who, at);
                }
            }
            Beat::Body(BodyBeat::Stop(_) | BodyBeat::Fidget { .. }) => {}
            // A crowd: each one a free actor, round the place, facing it
            // (a line, or a heap, as a row).
            Beat::Place(PlaceBeat::Crowd { who, spot, form, .. }) => {
                let Some((x, y)) = spot.as_ref().and_then(|spot| self.spot(spot)).or(self.middle) else { return };
                let many = who.len() as f32;
                for (at, one) in who.iter().enumerate() {
                    let point = match form.shape {
                        CrowdShape::Ring | CrowdShape::Flock => {
                            let radius = form.wide.unwrap_or((many * 28.0 / std::f32::consts::TAU).max(40.0));
                            let angle = std::f32::consts::TAU * at as f32 / many;
                            (x + angle.sin() * radius, y - angle.cos() * radius * 0.75)
                        }
                        _ => (x + (at as f32 - (many - 1.0) / 2.0) * BESIDE, y),
                    };
                    let facing = if matches!(form.shape, CrowdShape::Ring | CrowdShape::Flock) { way_to(point, (x, y)) } else { Direction::Down };
                    let at = Point { x: Num::Lit(point.0), y: Num::Lit(point.1) };
                    self.plain(number, word, &Beat::Body(BodyBeat::Put { who: *one, at, direction: facing }));
                    if let Some(actor) = self.actors.get(one).filter(|actor| actor.hidden).map(|actor| format!("ACTOR_{}", actor.name)) {
                        self.line(&format!("ResetOutputAttribute<actor {actor}>({HIDDEN});"));
                    }
                }
            }
            // Wandering: Sky's own stroll (CORO_MOVE_RANDOM), until another action.
            Beat::Place(PlaceBeat::Roams { who, wide }) => {
                let Some(actor) = self.who(number, *who) else { return };
                if *wide > 0.0 {
                    self.line(&format!("ExecuteCommon<actor {actor}>(CORO_MOVE_RANDOM, 0);"));
                } else if let Some(&at) = self.at.get(who) {
                    // Back to where it stood.
                    self.go(*who, &actor, at, "1", false);
                    self.wait_for(&actor);
                }
            }
            Beat::Motion(MotionBeat::Approach { who, other, .. } | MotionBeat::Chase { who, other }) => {
                let (Some(actor), Some(&to), Some(&from)) = (self.who(number, *who), self.at.get(other), self.at.get(who)) else { return };
                let side = if from.0 <= to.0 { -BESIDE } else { BESIDE };
                self.go(*who, &actor, (to.0 + side, to.1), "1", false);
            }
            Beat::Motion(MotionBeat::Flee { who, from, distance }) => {
                let (Some(actor), Some(&at), Some(&away)) = (self.who(number, *who), self.at.get(who), self.at.get(from)) else { return };
                let way = if at.0 >= away.0 { 1.0 } else { -1.0 };
                self.calm();
                self.go(*who, &actor, (at.0 + way * distance, at.1), "2", false);
            }
            Beat::Place(PlaceBeat::Arrives { who, to, gait, from, .. }) => {
                let (Some(actor), Some(to)) = (self.who(number, *who), self.point(to)) else {
                    self.problems.push(format!("line {number}: where {who:?} arrives is not known before the film plays"));
                    return;
                };
                let shown = self.at.contains_key(who) && !self.actors.get(who).is_some_and(|one| one.hidden && one.first.is_none());
                if from.is_none() && shown {
                    self.go(*who, &actor, to, if gait.run { "2" } else { "1" }, false);
                    return;
                }
                let right = match from {
                    Some(Exit::Side { right }) => *right,
                    _ => to.0 >= self.camera.0,
                };
                let start = (if right { self.camera.0 + OFF_SCREEN } else { self.camera.0 - OFF_SCREEN }, to.1);
                self.show_at(*who, &actor, start);
                let pace = if gait.run { "2" } else { "1" };
                self.go(*who, &actor, to, pace, false);
            }
            Beat::Place(PlaceBeat::Leaves { who, by, run }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let y = self.at.get(who).map_or(self.camera.1, |at| at.1);
                let right = match by {
                    Exit::Side { right } => *right,
                    Exit::Named(_) => self.at.get(who).is_none_or(|at| at.0 >= self.camera.0),
                };
                let to = (if right { self.camera.0 + OFF_SCREEN } else { self.camera.0 - OFF_SCREEN }, y);
                self.calm();
                self.go(*who, &actor, to, if *run { "2" } else { "1" }, false);
            }
            Beat::Body(BodyBeat::Gone(who)) => {
                let Some(actor) = self.who(number, *who) else { return };
                self.line(&format!("SetOutputAttribute<actor {actor}>({HIDDEN});"));
            }
            Beat::Body(BodyBeat::Opacity { who, to, .. }) => {
                let Some(actor) = self.who(number, *who) else { return };
                if *to > 0.5 {
                    let at = self.at.get(who).copied().unwrap_or(self.camera);
                    self.show_at(*who, &actor, at);
                } else if self.actors.get(who).is_some_and(|one| one.hidden && one.first.is_some()) {
                    // Put, hidden, before it appears: hidden from the start.
                } else {
                    self.line(&format!("SetOutputAttribute<actor {actor}>({HIDDEN});"));
                }
            }
            Beat::Flow(FlowBeat::WaitFor(who) | FlowBeat::WaitMoves(who)) => {
                if let Some(actor) = self.who(number, *who) {
                    self.wait_for(&actor);
                }
            }
            Beat::Flow(FlowBeat::WaitAll) => {
                for actor in self.busy.clone() {
                    self.wait_for(&actor);
                }
                self.line("WaitExecutePerformer(0);");
            }
            Beat::Body(BodyBeat::Effect { who, id, .. }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let effect = EFFECTS.iter().find(|(of, _)| *of == id.0).map_or(id.0.to_string(), |(_, name)| name.to_string());
                self.line(&format!("SetEffect<actor {actor}>({effect}, 3);"));
                self.shown_for = 0.0;
                if !self.showing.contains(&actor) {
                    self.showing.push(actor);
                }
            }
            Beat::Body(BodyBeat::MoveEffect { who, id }) => {
                // At a place: on the camera (performer 0), in the middle of the screen.
                let on = match who {
                    Who::Spot(_) => "performer 0".to_string(),
                    _ => match self.who(number, *who) {
                        Some(actor) => format!("actor {actor}"),
                        None => return,
                    },
                };
                let kind = self.game.move_types.get(usize::from(id.0)).copied().unwrap_or(1);
                self.line(&format!("SetEffect<{on}>({}, 3);", move_effect(kind)));
                self.line(&format!("WaitEffect<{on}>();"));
                self.line(&format!("SetEffect<{on}>(EFFECT_NONE, 3);"));
            }
            Beat::Screen(ScreenBeat::Weather(weather)) => self.weather(number, *weather),
            // Hops: Sky's own jumps (`CORO_JUMP_…`), happy, surprised or angry.
            Beat::Body(BodyBeat::Anim { who, anim, play }) if anim.0 == 10 => {
                let Some(actor) = self.who(number, *who) else { return };
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
            Beat::Body(BodyBeat::Anim { who, anim, play }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let looped = matches!(play, Play::Loop);
                let Some(set) = animation(anim.0, looped) else {
                    self.notes.push(format!("line {number}: the animation {} (no such one in Sky's scripts)", anim.0));
                    return;
                };
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
            Beat::Body(BodyBeat::Pose { who, pose }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let set = match pose {
                    Pose::Idle => 2,
                    Pose::Walk => 1,
                    Pose::Sleep => 62,
                    Pose::Hurt => 14,
                    Pose::Attack => 13,
                };
                self.stand_after.retain(|one| *one != actor);
                self.line(&format!("SetAnimation<actor {actor}>({set});"));
            }
            // A trembling: Sky shows pain (its looped animation), never a shiver.
            Beat::Body(BodyBeat::Tremble { who, frames, .. }) => {
                let Some(actor) = self.who(number, *who) else { return };
                // (26: its strain, sweat above the head, as seen in the game.)
                let set = if frames.0 > 0.0 { 26 } else { 2 };
                self.line(&format!("SetAnimation<actor {actor}>({set});"));
            }
            Beat::Body(BodyBeat::Jump { who, height, frames }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let up = (*height).round().max(4.0) as i32;
                let pace = speed(height * 2.0, frames.0.max(2.0));
                self.line(&format!("MoveHeight<actor {actor}>({pace}, {up});"));
                self.wait_for(&actor);
                self.line(&format!("MoveHeight<actor {actor}>({pace}, 0);"));
                self.wait_for(&actor);
            }
            Beat::Body(BodyBeat::Hop { who, at, .. }) => {
                let (Some(actor), Some(to)) = (self.who(number, *who), self.point(at)) else { return };
                self.go(*who, &actor, to, "2", false);
                self.wait_for(&actor);
            }
            // What Sky shows otherwise: a knock back, a sleeper's icon.
            // A state, as an effect on someone (LIMITS: confusion's stars, bubbles, Zs, ice).
            Beat::Body(BodyBeat::Status { who, icon, .. }) => {
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
                            self.notes.push(format!("line {number}: the state {icon} (no effect of Sky's for it)"));
                            return;
                        }
                    },
                };
                self.line(&format!("SetEffect<actor {actor}>({effect}, 3);"));
            }
            // A move, as LIMITS staged it: a wind-up, a lunge, the effect of
            // its type on the target, a small shake.
            Beat::Body(BodyBeat::Move { who, id, target }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let kind = self.game.move_types.get(usize::from(id.0)).copied().unwrap_or(1);
                self.line(&format!("SetAnimation<actor {actor}>(28);"));
                self.line("Wait(12);");
                self.line(&format!("SetAnimation<actor {actor}>(48);"));
                let on = match target.and_then(|target| self.who(number, target)) {
                    Some(target) => target,
                    None => actor.clone(),
                };
                self.line(&format!("SetEffect<actor {on}>({}, 3);", move_effect(kind)));
                self.line("camera_SetEffect(2, 1, 1.0);");
                self.line(&format!("WaitEffect<actor {on}>();"));
                self.line("camera_SetEffect(0, 0, 0);");
                self.line(&format!("SetEffect<actor {on}>(EFFECT_NONE, 3);"));
                self.line(&format!("SetAnimation<actor {actor}>(2);"));
            }
            // What comes so long after: at once, the script going on as it plays.
            Beat::Flow(FlowBeat::After { beat, .. }) => self.plain(number, word, beat),
            Beat::Body(BodyBeat::Recoil { .. }) => {
                self.notes.push(format!("line {number}: {beat:?}"));
            }
            Beat::Course(Course::Say { entity, face, text, .. }) => self.say(number, word, *entity, *face, text),
            Beat::Speech(SpeechBeat::Said) => {
                // (Where a question would be asked, in the same box.)
                let at = self.body.len();
                self.line("message_Close();");
                self.calm();
                self.closed = Some(at);
            }
            Beat::Speech(SpeechBeat::BoxSide { top }) => self.box_top = *top,
            Beat::Speech(SpeechBeat::BoxAuto) => self.box_top = false,
            Beat::Flow(FlowBeat::Wait(frames)) => {
                // Before the scene shows: the title's time, left out with it;
                // the engine's pauses after a line said: the box closing;
                // in a thread of `together`: its moves go on alone.
                if std::mem::take(&mut self.said_over) {
                    // (The time a caption shows: the game waits for its key.)
                } else if self.started && !self.thread && !(self.talking && frames.0 < 15.0) {
                    self.line(&format!("Wait({});", frames.0.round() as i32));
                    self.shown_for += frames.0;
                }
            }
            Beat::Screen(ScreenBeat::Fade { out: false, white, frames }) => {
                let first = !self.started;
                self.started = true;
                self.faded_out = false;
                let frames = frames.0.round() as i32;
                if first && self.top.is_some() {
                    self.line(&format!("screen2_FadeIn(0, {frames});"));
                }
                // (Sky comes back from white with a fade in: it has no WhiteIn of its own.)
                let fade = "screen_FadeIn";
                let _ = white;
                self.line(&format!("{fade}(1, {frames});"));
            }
            Beat::Screen(ScreenBeat::Fade { out: true, white, frames }) => {
                if !self.faded_out {
                    let fade = if *white { "screen_WhiteOut" } else { "screen_FadeOut" };
                    self.line(&format!("{fade}(1, {});", frames.0.round() as i32));
                }
                self.faded_out = true;
            }
            Beat::Screen(ScreenBeat::Wipe { out, frames, .. }) | Beat::View(ViewBeat::Iris { open: out, frames, .. }) => {
                // (A wipe or an iris: Sky fades.)
                let fade = if matches!(beat, Beat::View(_)) { !*out } else { *out };
                let white = false;
                self.plain(number, word, &Beat::Screen(ScreenBeat::Fade { out: fade, white, frames: *frames }));
            }
            Beat::Flow(FlowBeat::WaitFade) => self.line("WaitScreenFade();"),
            // A flash: Sky's own (white: `screen_WhiteOut` and back; a colour: `screen_FlushOut`).
            Beat::Screen(ScreenBeat::Flash { colour, frames }) => {
                let half = (frames.0 / 2.0).round().max(1.0) as i32;
                if colour.iter().all(|part| *part >= 250.0) {
                    self.line("screen_WhiteOut(1, 2);");
                    self.line("Wait(2);");
                    self.line(&format!("screen_FadeIn(1, {});", frames.0.round() as i32));
                } else {
                    let [red, green, blue] = colour.map(|part| part.round() as i32);
                    self.line(&format!("screen_FlushOut(1, {half}, 2, {red}, {green}, {blue});"));
                    self.line(&format!("Wait({half});"));
                    self.line(&format!("screen_FlushIn(1, {half}, 2, {red}, {green}, {blue});"));
                }
            }
            // (The title again, as the engine shows it with each scene: once is enough.)
            Beat::Screen(ScreenBeat::Caption { text, .. }) if self.title.as_ref() == Some(text) => {}
            Beat::Screen(ScreenBeat::Caption { text, .. }) => {
                if self.started {
                    // Said over the scene, as Sky narrates.
                    self.line(&format!("message_Narration(30, \"{}\");", quoted(text)));
                    self.line("message_Close();");
                    self.said_over = true;
                } else if self.title.is_none() {
                    self.title = Some(text.clone());
                }
            }
            Beat::Screen(ScreenBeat::Banner(text)) => {
                self.line(&format!("back_SetTitleBanner(56, \"{}\");", quoted(text)));
            }
            Beat::Speech(SpeechBeat::Mood { .. }) => {
                self.notes.push(format!("line {number}: a face changed in the middle of a line (Sky changes it between lines)"))
            }
            other => self.problems.push(format!("line {number}: the engine's {other:?} doesn't export to DS yet ({})", word.unwrap_or("a line said"))),
        }
    }

    /// Someone shows there: put, and no longer hidden.
    fn show_at(&mut self, who: Who, actor: &str, at: (f32, f32)) {
        let first = self.actors.get(&who).is_some_and(|one| one.hidden && one.first.is_none());
        if first {
            if let Some(one) = self.actors.get_mut(&who) {
                let facing = self.facing.get(&who).copied().unwrap_or(Direction::Down);
                one.first = Some((at.0, at.1, direction(facing)));
            }
        } else if self.at.get(&who) != Some(&at) {
            let position = self.position(at);
            self.line(&format!("SetPositionMark<actor {actor}>({position});"));
        }
        self.at.insert(who, at);
        self.line(&format!("ResetOutputAttribute<actor {actor}>({HIDDEN});"));
    }

    /// The weather: Sky's own falling effects on a performer (3) over the
    /// whole screen.
    fn weather(&mut self, number: i32, weather: Weather) {
        let effect = match weather {
            Weather::Clear => {
                if self.weather {
                    self.line("SetEffect<performer 3>(EFFECT_NONE, 3);");
                }
                return;
            }
            Weather::Rain => 16,
            Weather::Hail | Weather::Snow => 20,
            other => {
                self.notes.push(format!("line {number}: the weather {other:?} (Sky has no such effect)"));
                return;
            }
        };
        self.weather = true;
        self.line(&format!("SetEffect<performer 3>({effect}, 3);"));
    }

    /// A line said: its face (or the species' normal one, or its name
    /// alone), its box, as Sky's message operations show it.
    fn say(&mut self, number: i32, word: Option<&str>, entity: Option<Who>, face: Option<u16>, text: &str) {
        let text = quoted(text.trim_end());
        let text = if text.starts_with(' ') || text.starts_with('[') { text } else { format!(" {text}") };
        match (entity, word) {
            (None, Some("narrate")) if text.trim_start().starts_with("[CN]") => {
                self.line(&format!("message_Explanation(\"{}\");", text.trim_start()));
            }
            (None, _) => {
                self.line("message_ResetActor();");
                self.line(&format!("message_Talk(\"{text}\");"));
            }
            (Some(who), _) => {
                let Some(actor) = self.who(number, who) else { return };
                let kind = self.actors.get(&who).map_or(0, |one| one.kind);
                let face = face.unwrap_or(0);
                let shown = if self.game.has_face(kind, face) {
                    Some(face)
                } else if self.game.has_face(kind, 0) {
                    Some(0)
                } else {
                    None
                };
                match shown {
                    Some(face) => {
                        let face = FACES.get(usize::from(face)).map_or(face.to_string(), |name| format!("FACE_{name}"));
                        let place = if self.box_top { "FACE_POS_TOP_L_FACEINW" } else { "FACE_POS_STANDARD" };
                        self.line(&format!("message_SetFace({actor}, {face}, {place});"));
                    }
                    None => self.line(&format!("message_SetActor({actor});")),
                }
                if word == Some("think") {
                    self.line(&format!("message_Monologue(\"{text}\");"));
                } else {
                    self.line(&format!("message_Talk(\"{text}\");"));
                }
            }
        }
        self.talking = true;
    }

    /// Whether the line's word was given that word (`hop surprised`).
    fn said_with(&self, number: i32, said: &str) -> bool {
        self.words.get(&number).is_some_and(|words| words.iter().any(|word| word == said))
    }

    /// The scene written, and the writer ready for the next: the film's
    /// own (its cast, title, words, what was told) kept.
    fn next_scene(&mut self) {
        let block = self.block();
        let mut next = Writer::new(self.game);
        next.blocks = std::mem::take(&mut self.blocks);
        next.blocks.push(block);
        next.cast = std::mem::take(&mut self.cast);
        next.title = self.title.take();
        next.titled = self.titled;
        next.words = std::mem::take(&mut self.words);
        next.problems = std::mem::take(&mut self.problems);
        next.notes = std::mem::take(&mut self.notes);
        next.vars = std::mem::take(&mut self.vars);
        *self = next;
    }

    /// The whole film: each scene, who stands where for `patch_rom.py`, then its `def 0`.
    fn finish(&mut self, film: &str) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "// The film “{film}”, exported by `scene_player export-ds` for tools/ds/patch_rom.py.");
        let last = self.block();
        for block in self.blocks.iter().chain([&last]) {
            out.push_str(block);
        }
        out
    }

    /// One scene: its level, actors and performers, then its `def 0`.
    fn block(&mut self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "// ds-export level {}", self.level.as_deref().unwrap_or("?"));
        let middle = self.middle.unwrap_or((128.0, 96.0));
        for actor in self.actors.values() {
            let (x, y, facing) = actor.first.unwrap_or((middle.0, middle.1, "DIR_DOWN"));
            let species = actor.species.map_or(String::new(), |species| format!(" species {species}"));
            let _ = writeln!(out, "// ds-export actor ACTOR_{}{species} x {} y {} facing {facing}", actor.name, tiles(x), tiles(y));
        }
        // The camera, and the weather over the screen.
        let _ = writeln!(out, "// ds-export performer 0 type 0 x {} y {}", tiles(middle.0), tiles(middle.1));
        if self.weather {
            let _ = writeln!(out, "// ds-export performer 3 type 3 x {} y {}", tiles(middle.0), tiles(middle.1));
        }
        let _ = writeln!(out, "def 0 {{");
        // The title, as Sky shows its story's: alone on black, before the first scene.
        if let Some(title) = self.title.as_ref().filter(|_| !self.titled) {
            self.titled = true;
            out.push_str("    sound_Stop();\n");
            let _ = writeln!(out, "    back_SetTitleBanner(56, \"{}\");", quoted(title));
            out.push_str("    screen_FadeIn(1, 30);\n    Wait(120);\n    screen_FadeOut(1, 30);\n    Wait(15);\n");
        }
        // Who comes in later is hidden until then.
        let hidden: Vec<String> = self.actors.values().filter(|actor| actor.hidden).map(|actor| actor.name.clone()).collect();
        let mut body = std::mem::take(&mut self.body);
        if !hidden.is_empty() {
            let hide: String = hidden.iter().map(|name| format!("    SetOutputAttribute<actor ACTOR_{name}>({HIDDEN});\n")).collect();
            let at = body.find("    camera_SetMyself").map_or(0, |at| body[at..].find('\n').map_or(at, |end| at + end + 1));
            body.insert_str(at, &hide);
        }
        out.push_str(&body);
        if !self.faded_out {
            out.push_str("    screen_FadeOut(1, 30);\n    WaitScreenFade();\n");
        }
        if self.memory {
            out.push_str("    screen_FlushIn(1, 0, 1, 0, 0, 0);\n");
        }
        if self.top.is_some() {
            out.push_str("    screen2_FadeOut(0, 30);\n    WaitScreenFade();\n    back2_SetMode(0);\n");
        }
        if let Some(tint) = self.tint {
            let _ = writeln!(out, "    screen_FlushIn(1, 0, {tint});");
        }
        if self.white_all {
            out.push_str("    screen_FadeInAll(1, 0);\n    screen_FadeOut(1, 0);\n");
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

    #[test]
    fn animations_go_through_skys_table() {
        // Sleep looped (42, the same entry as Sky's 62) and waking once (23, as its 63).
        assert_eq!(animation(13, true), Some(42));
        assert_eq!(animation(14, false), Some(23));
        // A group with no looped entry plays once.
        assert_eq!(animation(33, true), Some(81));
        assert_eq!(animation(12, false), None);
    }

    #[test]
    fn speeds_are_pixels_a_frame_in_256ths() {
        assert_eq!(speed(60.0, 60.0), "1");
        assert_eq!(speed(30.0, 60.0), "0.5");
        assert_eq!(speed(1.0, 600.0), "0.0625");
    }

    #[test]
    fn ways_are_the_engines_eight() {
        assert_eq!(way_to((0.0, 0.0), (10.0, 0.0)), Direction::Right);
        assert_eq!(way_to((0.0, 0.0), (0.0, 10.0)), Direction::Down);
        assert_eq!(way_to((0.0, 0.0), (-10.0, -10.0)), Direction::UpLeft);
    }

    #[test]
    fn texts_are_quoted_on_one_line() {
        assert_eq!(quoted("a \"b\"\nc"), "a \\\"b\\\"\\nc");
    }
}
