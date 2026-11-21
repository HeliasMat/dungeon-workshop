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
        Export::Direct,
        "SetAnimation<actor>(n): the group of that species' own sprite showing it (ds_animations.txt: Dusknoir lies with its 27), through Sky's table (once or looped); off: 2",
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
    (
        "weather",
        Export::Close,
        "SetEffect<performer 3>(rain 698, Sky's very heavy rain of its scenes / hail 20; snow as hail); sand, fog, sun…: none in scripts",
    ),
    (
        "daytime",
        Export::Close,
        "screen_FlushOut(1, d, 4, 224, 176, 136) evening, (…, 4, 80, 112, 144) night (the light of Sky's own evening and night guild crossroads); day: screen_FlushIn; the light's own colour is lost; as a scene opens, the place's own level gives it",
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
    ("open", Export::Direct, "the thing put in the scene's layout (an object of the game); SetAnimation<object>(17): its animation once"),
    ("close", Export::Direct, "SetAnimation<object>(9): its first picture, held"),
    // Fights
    ("fight", Export::None, "the game fights in dungeons only: main_EnterDungeon to a boss floor"),
    (
        "attack",
        Export::Close,
        "its pain once (a crouch), its own attack (group 1), a script effect of the move's type on the target, a small shake: not the move's own animation (dungeons only)",
    ),
    (
        "evolve",
        Export::Close,
        "a stats-up shine, the screen white, the evolved one (an actor of its own) put where it stood and shown, the other hidden; no change of species (Sky's scripts have none)",
    ),
    ("join", Export::Direct, "its emote and hop, then as Sky: me_Play(5), message_Mail(\"[CN]… joined the team!\"), WaitMe(5)"),
    (
        "roar",
        Export::Direct,
        "as Sky's bosses: se_Play(6930), camera_SetEffect(2, 1, 1.0) (hard: 2, 2, 3.0) a while, then 0, 0, 0; SetAnimation of the sprite's own Roar group once, else its Charge looped (9), as Groudon's",
    ),
    (
        "charge",
        Export::Direct,
        "SetAnimation (its Charge, looped: 9), SetEffect(691, 3) (Sky's stats-up shine), se_Play(4881), a beat, standing; the charged attack's flash and shake as written (screen_WhiteOut, camera_SetEffect)",
    ),
    (
        "dodge",
        Export::Close,
        "Sky's hop (CORO_JUMP_HAPPY_FUNC_SERIES) and a quick MovePositionMark aside, the miss's sound (se_Play); no \"Missed!\" (Sky's scenes have no floating words)",
    ),
    (
        "finish",
        Export::Close,
        "its words: the flash, the attack, the faint; bgm_ChangeVolume(0, 0) for the silence, then (0, 256); no slow motion (Sky's scripts have none)",
    ),
    // Time
    ("wait", Export::Direct, "Wait(frames) / WaitExecuteLives / all: each one moving, WaitExecutePerformer(0)"),
    ("together", Export::Direct, "each one's moves told at once (actors move on their own), then their WaitExecuteLives; its waits are lost"),
    ("mark", Export::Direct, "a @label; what is timed from it is already written as waits"),
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
/// `--tall`: the top screen shows the place above the bottom one (`Writer::tall`).
pub fn export(context: &Context, name: &str, out: Option<&String>, table: bool, partial: bool, tall: bool) -> Result<(), String> {
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
    writer.tall_asked = tall;
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
    for told in &writer.told {
        eprintln!("  {told}");
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
    /// The species' names, to tell who is left out.
    names: HashMap<u16, String>,
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
        let names = rows("monsters.txt").into_iter().filter_map(|row| Some((row.first()?.parse().ok()?, row.get(1)?.clone()))).collect();
        _ = DATA.set(data.to_string());
        Game { levels, actors, data: data.to_string(), move_types, names }
    }

    /// A level's size in pixels (its picture's, as the extraction wrote it:
    /// two numbers of 8 bytes first).
    fn size(&self, level: &str) -> Option<(f32, f32)> {
        let id = self.levels.iter().find(|(_, name)| *name == level)?.0;
        let info = std::fs::read(game_file(&self.data, &format!("rom/grounds/info/{id}.bin"))).ok()?;
        let number = |at: usize| Some(u64::from_le_bytes(info.get(at..at + 8)?.try_into().ok()?) as f32);
        Some((number(0)?, number(8)?))
    }

    fn name(&self, species: u16) -> String {
        self.names.get(&species).cloned().unwrap_or_else(|| format!("species {species}"))
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
    /// The layer of the scene's layout it is in: 0 from the start, a later
    /// one loaded when it comes in (past Sky's 21 at once).
    layer: usize,
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

/// The evening and night lights (`screen_FlushOut`'s mode and colour).
const EVENING: &str = "4, 224, 176, 136";
const NIGHT: &str = "4, 80, 112, 144";
/// Dawn, softer and warm; a cave's dimness, grey.
const DAWN: &str = "4, 248, 224, 208";
const DIM: &str = "4, 160, 160, 160";

/// The light of the engine's tint (its colour and strength), as Sky's.
fn light(colour: &[f32; 4]) -> Option<&'static str> {
    Some(match colour {
        [.., much] if *much <= 0.0 => return None,
        [red, green, blue, _] if red.max(*green).max(*blue) < 10.0 => DIM,
        [red, _, blue, _] if blue > red => NIGHT,
        [.., much] if *much < 64.0 => DAWN,
        _ => EVENING,
    })
}

/// The script variables a film's own may use (local to the scene).
const LOCALS: [&str; 4] = ["$LOCAL0", "$LOCAL1", "$LOCAL2", "$LOCAL3"];

/// The output attribute that hides an actor (`SetOutputAttribute<actor>(128)`).
const HIDDEN: u16 = 128;

/// The groups past 12 in the order the starters' sprites share (the
/// engine's `AnimId`s are these): 13 EventSleep ... 34 HitGround.
const SHARED: &str =
    "EventSleep Wake Eat Tumble Pose Pull Pain Float DeepBreath Nod Sit LookUp Sink Trip Laying LeapForth Head Cringe LostBalance TumbleBack Faint HitGround";

/// Where each species whose sprite orders its groups past 12 its own way is
/// told: the groups it has and their names, read from one's own ROM by
/// `tools/ds/animations.py` into the data (never in the repository; the
/// names are SkyTemple's). Without it, every species shares the starters'
/// order.
const OWN_ORDER: &str = "extra/ds_animations.txt";

/// The data folder the exporter reads, for the table of the sprites' own
/// orders.
static DATA: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// What shows a pose when a sprite has no group of that name, in turn
/// (a name of the first thirteen groups, which every sprite has, ends it).
fn instead(name: &str) -> &'static [&'static str] {
    match name {
        "EventSleep" => &["Sleep"],
        "Wake" => &["StandingUp", "Idle"],
        "Tumble" | "TumbleBack" | "Trip" => &["Tumble", "TumbleBack", "Trip", "Hurt"],
        "Pose" | "Float" | "LeapForth" => &["Hop"],
        "Pull" => &["Charge"],
        "Pain" | "Cringe" | "LostBalance" => &["Pain", "Hurt"],
        "Laying" => &["Fainted", "Sleep"],
        // (Their own "Faint" may rise again, as Dusknoir's 25: their lying one, held, reads better.)
        "Sink" | "Faint" | "HitGround" => &["Fainted", "Laying", "Sleep"],
        _ => &["Idle"],
    }
}

/// The first thirteen groups, the same in every sprite.
fn common_group(name: &str) -> Option<u16> {
    Some(match name {
        "Walk" => 0,
        "Attack" => 1,
        "Sleep" => 5,
        "Hurt" => 6,
        "Idle" => 7,
        "Hop" => 10,
        "Charge" => 11,
        _ => return None,
    })
}

/// The group of that species' own sprite that shows what the engine's
/// group shows (the starters' order): itself for most of the story's
/// heroes, another one, or a close one, for the sprites ordered their own
/// way (Dusknoir's faint is its 25). `None`: it would only stand.
fn own_groups() -> &'static HashMap<u16, Vec<(u16, String)>> {
    use std::sync::OnceLock;
    static TABLE: OnceLock<HashMap<u16, Vec<(u16, String)>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let told = DATA.get().map(|data| std::fs::read_to_string(game_file(data, OWN_ORDER)).unwrap_or_default()).unwrap_or_default();
        told.lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let (species, groups) = line.split_once('\t')?;
                let groups =
                    groups.split(',').filter_map(|pair| pair.split_once('=')).filter_map(|(at, name)| Some((at.parse().ok()?, name.to_string()))).collect();
                Some((species.parse().ok()?, groups))
            })
            .collect()
    })
}

/// The group of that species' own sprite named so (`Roar`), if it has one
/// Sky's table reaches.
fn own_named(species: u16, name: &str) -> Option<u16> {
    own_groups().get(&species)?.iter().find(|(at, own)| own == name && *at <= 34).map(|(at, _)| *at)
}

fn own_group(species: u16, group: u16) -> Option<u16> {
    let table = own_groups();
    let Some(own) = table.get(&species).filter(|_| species != 0) else { return Some(group) };
    let Some(name) = group.checked_sub(13).and_then(|at| SHARED.split(' ').nth(usize::from(at))) else { return Some(group) };
    // (Sky's table reaches groups 0 to 34 only.)
    let named = |name: &str| own.iter().find(|(at, own)| *own == name && *at <= 34).map(|(at, _)| *at);
    if let Some(at) = named(name).filter(|_| name != "Faint") {
        return Some(at);
    }
    instead(name).iter().find_map(|name| common_group(name).or_else(|| named(name))).filter(|at| *at != 7)
}

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
    /// The place's things (the game's objects) and where each first stands.
    objects: BTreeMap<u16, (f32, f32)>,
    /// Who has become whom (`evolve`): the later lines' name, the new one's actor.
    became: HashMap<Who, Who>,
    /// The words each line of the film was given, by its number.
    words: HashMap<i32, Vec<String>>,
    problems: Vec<String>,
    notes: Vec<String>,
    /// What the export changed to fit the DS, told as it exports.
    told: Vec<String>,
    /// Those a frame of a few named (never left out).
    framed: Vec<Who>,
    /// `--tall`: the top screen shows the place above the bottom one, where it is tall enough.
    tall_asked: bool,
    /// This scene's place shows on both screens, one above the other.
    tall: bool,
    /// The place's size in pixels.
    size: Option<(f32, f32)>,
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

/// A frame of so many at most is of a few: they are never left out.
const FEW: usize = 6;

/// A pan's pace across a crowd too wide for the screen, pixels a frame.
const PAN: f32 = 0.75;

/// Actors at once in a scene: the game's 24, less the hero's, the
/// partner's and a third party member's, kept for them (overlay 11 looks
/// for a free one from the fourth: `ov11_022F7E78`, NA). Seen: 21 load.
const AT_ONCE: usize = 21;

/// The screen, pixels, and how far round someone's feet its sprite shows
/// (sideways, above, below).
const SCREEN: (f32, f32) = (256.0, 192.0);
const SPRITE: (f32, f32, f32) = (16.0, 32.0, 8.0);

/// Where a pan across those starts and ends, when they don't fit the
/// screen (none: they fit).
fn pan(points: &[(f32, f32)]) -> Option<((f32, f32), (f32, f32))> {
    let (left, right) = points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), point| (lo.min(point.0), hi.max(point.0)));
    let (top, bottom) = points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), point| (lo.min(point.1), hi.max(point.1)));
    let wide = right - left + 2.0 * SPRITE.0 > SCREEN.0;
    let high = bottom - top + SPRITE.1 + SPRITE.2 > SCREEN.1;
    if !wide && !high {
        return None;
    }
    let middle = ((left + right) / 2.0, (top + bottom) / 2.0);
    let from = (if wide { left - SPRITE.0 + SCREEN.0 / 2.0 } else { middle.0 }, if high { top - SPRITE.1 + SCREEN.1 / 2.0 } else { middle.1 });
    let to = (if wide { right + SPRITE.0 - SCREEN.0 / 2.0 } else { middle.0 }, if high { bottom + SPRITE.2 - SCREEN.1 / 2.0 } else { middle.1 });
    // (A few pixels out: their middle shows them well enough.)
    let across = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
    (across >= 24.0).then_some((from, to))
}

/// Whether a script line names that actor (`ACTOR_X`, not `ACTOR_X2`).
fn names(line: &str, actor: &str) -> bool {
    line.match_indices(actor).any(|(at, _)| !line[at + actor.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
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
            objects: BTreeMap::new(),
            became: HashMap::new(),
            problems: Vec::new(),
            notes: Vec::new(),
            told: Vec::new(),
            framed: Vec::new(),
            tall_asked: false,
            tall: false,
            size: None,
        }
    }

    /// The light on the screen. (Not on the top one: `screen2_Flush…` at
    /// the same time fights it, both screens flickering, as seen.)
    fn flush(&mut self, way: &str, frames: i32, tint: &str) {
        self.line(&format!("screen_Flush{way}(1, {frames}, {tint});"));
    }

    /// Sky's `SetAnimation` number for the engine's group on that one's
    /// own sprite (`own_group`).
    fn set(&self, who: Who, group: u16, looped: bool) -> Option<u16> {
        let who = self.became.get(&who).copied().unwrap_or(who);
        let species = self.actors.get(&who).map_or(0, |actor| actor.kind);
        animation(own_group(species, group)?, looped)
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
        let who = self.became.get(&who).copied().unwrap_or(who);
        if let Some(actor) = self.actors.get(&who) {
            return Some(format!("ACTOR_{}", actor.name));
        }
        // (Past the free ones, a name `patch_rom.py --more-actors` gives a new actor.)
        let free = |cast: &BTreeMap<Who, (String, Option<u16>, u16)>| {
            let given = cast.values().filter(|one| one.1.is_some()).count();
            Some(FREE_ACTORS.get(given).map_or_else(|| format!("NPC_FREE{given:03}"), |name| name.to_string()))
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
        self.actors.insert(who, Actor { name: name.clone(), species, kind, first: None, hidden, layer: 0 });
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
            // Before someone as it looks, or behind it (a dodge back).
            Spot::Before { of, back } => {
                let (x, y) = *self.at.get(of)?;
                let (dx, dy) = match self.facing.get(of).copied().unwrap_or(Direction::Down) {
                    Direction::Down => (0.0, 1.0),
                    Direction::DownRight => (0.7, 0.7),
                    Direction::Right => (1.0, 0.0),
                    Direction::UpRight => (0.7, -0.7),
                    Direction::Up => (0.0, -1.0),
                    Direction::UpLeft => (-0.7, -0.7),
                    Direction::Left => (-1.0, 0.0),
                    Direction::DownLeft => (-0.7, 0.7),
                };
                let way = if *back { -BESIDE } else { BESIDE };
                Some((x + dx * way, y + dy * way))
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
        let above = self.tall.then(|| self.above(to));
        let above = above.map(|above| self.position(above));
        if frames <= 1.0 || distance < 2.0 || !self.started {
            self.line(&format!("SetPositionMark<performer 0>({position});"));
            if let Some(above) = above {
                self.line(&format!("camera2_SetPositionMark({above});"));
            }
        } else {
            let pace = speed(distance, frames);
            self.line(&format!("MovePositionMark<performer 0>({pace}, {position});"));
            // (The top screen's camera glides with it, as Sky's tall views do: S13P04's.)
            if let Some(above) = above {
                self.line(&format!("camera2_MovePositionMark({pace}, {above});"));
            }
        }
    }

    /// Where the top screen's camera looks for the bottom one's there, in
    /// the tall view: a screen above, both kept in the place (the game
    /// stops a camera at its edges; where the bottom one is too high for a
    /// screen above, the two show the same rows).
    fn above(&self, (x, y): (f32, f32)) -> (f32, f32) {
        let (wide, high) = self.size.unwrap_or((256.0, 384.0));
        let x = x.clamp(128.0, (wide - 128.0).max(128.0));
        let y = y.clamp(96.0, (high - 96.0).max(96.0));
        (x, (y - 192.0).max(96.0))
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
            // A mark (`at name + 2s:`): a label; its times are already written as waits.
            Beat::Data(DataBeat::Set { var, .. }) if var.starts_with("_mark_") => self.label(&format!("mark_{}", &var["_mark_".len()..])),
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
    fn roar(&mut self, number: i32, who: Who) -> bool {
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
    fn charge(&mut self, number: i32, who: Who) -> bool {
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

    fn plain(&mut self, number: i32, word: Option<&str>, beat: &Beat) {
        match beat {
            // Floating words (`Missed!`): Sky's scenes have none.
            Beat::Body(BodyBeat::Popup { text, .. }) => self.notes.push(format!("line {number}: the words {text:?} floating up")),
            // A slow motion: Sky's scripts have none; the finishing blow plays at its pace.
            Beat::View(ViewBeat::Tempo(_)) if word == Some("finish") => {}
            // The music lowered, or back: Sky's own volume (0 to 256), at once.
            Beat::Sound(SoundBeat::Volume(volume)) => self.line(&format!("bgm_ChangeVolume(0, {});", (volume * 256.0).round() as i32)),
            // The place's things: objects of the game in the scene's layout.
            Beat::Body(BodyBeat::Put { who: Who::Thing { object, .. }, at, .. }) => {
                let Some(point) = self.point(at) else { return };
                if self.objects.contains_key(object) {
                    let position = self.position(point);
                    self.line(&format!("SetPositionMark<object OBJECT_N{object}>({position});"));
                } else {
                    self.objects.insert(*object, point);
                }
            }
            // Opened: its animation once (Sky's objects' table: 17); closed: its first picture held (9).
            Beat::Body(BodyBeat::Anim { who: Who::Thing { object, .. }, play, .. }) => {
                if !self.objects.contains_key(object) {
                    self.problems.push(format!("line {number}: the thing {object} is not put in the scene"));
                    return;
                }
                let set = match play {
                    Play::Hold(_) => 9,
                    Play::Loop => 1,
                    _ => 17,
                };
                self.line(&format!("SetAnimation<object OBJECT_N{object}>({set});"));
            }
            Beat::Body(BodyBeat::ThingName { who: Who::Thing { .. }, .. }) => {}
            // The engine's evolution (slowed, shining, growing): Sky's white flash instead, below.
            Beat::View(ViewBeat::Tempo(_)) | Beat::Body(BodyBeat::Colour { .. } | BodyBeat::Scale { .. }) | Beat::Screen(ScreenBeat::Particles { .. })
                if word == Some("evolve") => {}
            // Evolving: a shine, the screen white, the evolved one (an actor of its
            // own) put where the other stood, the other hidden, the screen back.
            Beat::Body(BodyBeat::Disguise { who, like: Some(like) }) if word == Some("evolve") => {
                let (Some(old), Some(&at)) = (self.who(number, *who), self.at.get(who)) else { return };
                let Some(new) = self.who(number, *like) else { return };
                let facing = direction(self.facing.get(who).copied().unwrap_or(Direction::Down));
                let position = self.position(at);
                self.line(&format!("SetEffect<actor {old}>(691, 3);"));
                self.line("Wait(40);");
                self.line("screen_WhiteOut(1, 30);");
                self.line("WaitScreenFade();");
                self.line(&format!("SetEffect<actor {old}>(EFFECT_NONE, 3);"));
                self.line(&format!("SetOutputAttribute<actor {old}>({HIDDEN});"));
                self.line(&format!("SetPositionMark<actor {new}>({position});"));
                self.line(&format!("SetDirection<actor {new}>({facing});"));
                self.line(&format!("ResetOutputAttribute<actor {new}>({HIDDEN});"));
                self.line("screen_FadeIn(1, 30);");
                self.line("WaitScreenFade();");
                self.became.insert(*who, *like);
            }
            Beat::Screen(ScreenBeat::Grey(_)) if !self.started => {}
            // The light: Sky's screen_FlushOut, evening orange or night blue
            // (the memory's sepia is its own, below).
            // As a scene opens: the light from the start (the screen tinted before it
            // fades in, as seen), unless the place is one of Sky's own evening or
            // night levels (lettered past A: `D01P11B` is the beach at sunset).
            Beat::Screen(ScreenBeat::Tint { colour, .. }) if !self.started => {
                let own = self.level.as_deref().and_then(|level| level.chars().last()).is_some_and(|last| last.is_ascii_alphabetic() && last != 'A');
                self.tint = light(colour).filter(|_| !own);
            }
            Beat::Screen(ScreenBeat::Tint { colour, frames }) if !self.memory => {
                let frames = if self.started { frames.0.round() as i32 } else { 0 };
                // (Mode 4 multiplies the screen by the colour, as Sky's own warm tint
                // `128, 96, 64` does; the colours are those its evening and night
                // levels of the guild's crossroads have against the day's.)
                let Some(tint) = light(colour) else {
                    if let Some(tint) = self.tint.take() {
                        self.flush("In", frames, tint);
                    }
                    return;
                };
                // (The same light again, as after the rain: kept; FlushIn then FlushOut would flash.)
                if self.tint == Some(tint) {
                    return;
                }
                if let Some(old) = self.tint.take() {
                    self.flush("In", 0, old);
                }
                self.flush("Out", frames, tint);
                self.tint = Some(tint);
            }
            Beat::Screen(ScreenBeat::Tint { .. }) => {}
            Beat::Screen(ScreenBeat::Grey(on)) => {
                if *on && !self.memory {
                    // Sky's memories and visions: the screen in sepia.
                    self.line("screen_FlushOut(1, 0, 8, 0, 0, 0);");
                } else if !*on && self.memory {
                    self.line("screen_FlushIn(1, 0, 1, 0, 0, 0);");
                    // (The sepia took the light's place: the light again.)
                    if let Some(tint) = self.tint {
                        self.flush("Out", 0, tint);
                    }
                }
                self.memory = *on;
            }
            Beat::Screen(ScreenBeat::Backdrop(level, ..)) => match self.game.levels.get(level).cloned() {
                Some(level) => {
                    self.top = TOP_SCREENS.iter().find(|(place, _)| *place == level).map(|(_, top)| *top);
                    self.size = self.game.size(&level);
                    // (`--tall`: the place itself on the top screen, a screen above, where it is tall enough.)
                    // (A place Sky pairs with a picture of its own keeps it: the beach's is its sky and
                    // rock, where the place above the sand is only sea, its foam blank on the top screen.)
                    self.tall = self.tall_asked && self.top.is_none() && self.size.is_some_and(|(_, high)| high >= 384.0);
                    if self.tall_asked && !self.tall {
                        let why = if self.top.is_some() { "Sky's own top picture for it kept" } else { "too short for two screens (384 pixels)" };
                        self.told.push(format!("{level}: no tall view, {why}"));
                    }
                    if self.tall {
                        self.top = None;
                        self.line("back2_SetMode(4);");
                        self.line(&format!("back2_SetGround(LEVEL_{level});"));
                    } else if let Some(top) = self.top {
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
                if self.tall {
                    let above = self.above((middle.x, middle.y));
                    let position = self.position(above);
                    self.line(&format!("camera2_SetPositionMark({position});"));
                }
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
                // A frame of a few: those are never left out of a crowded scene.
                if who.len() <= FEW {
                    self.framed.extend(who.iter().copied());
                }
                let count = points.len() as f32;
                let middle = (points.iter().map(|point| point.0).sum::<f32>() / count, points.iter().map(|point| point.1).sum::<f32>() / count);
                match pan(&points).filter(|_| self.started) {
                    // Wider (or taller) than the screen, and Sky has no zoom: a slow pan
                    // across them, from one end to the other, then back to their middle.
                    Some((from, to)) => {
                        self.camera_to(from, frames.0);
                        self.line("WaitExecutePerformer(0);");
                        let across = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
                        self.camera_to(to, across / PAN);
                        self.line("WaitExecutePerformer(0);");
                        let back = ((middle.0 - to.0).powi(2) + (middle.1 - to.1).powi(2)).sqrt();
                        self.camera_to(middle, back / PAN);
                        self.told.push(format!("line {number}: the frame is wider than the screen: a pan across it ({across:.0} pixels)"));
                    }
                    None => self.camera_to(middle, frames.0),
                }
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
            // An unseen point (where a dodger stood): only its place, kept.
            Beat::Body(BodyBeat::Put { who: who @ Who::Spot(_), at, .. }) => {
                if let Some(point) = self.point(at) {
                    self.at.insert(*who, point);
                }
            }
            Beat::Body(BodyBeat::Put { who, at, direction: way }) => {
                let Some(actor) = self.who(number, *who) else { return };
                let Some(point) = self.point(at) else {
                    self.problems.push(format!("line {number}: where {who:?} is put is not known before the film plays"));
                    return;
                };
                self.at.insert(*who, point);
                self.facing.insert(*who, *way);
                let facing = direction(*way);
                match self.actors.get_mut(&self.became.get(who).copied().unwrap_or(*who)) {
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
                let Some(set) = self.set(*who, anim.0, looped) else {
                    self.notes.push(format!("line {number}: the animation {} (none of this sprite's shows it)", anim.0));
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
                    Pose::Sleep => self.set(*who, 13, true).unwrap_or(6),
                    Pose::Hurt => 14,
                    Pose::Attack => 13,
                };
                self.stand_after.retain(|one| *one != actor);
                self.line(&format!("SetAnimation<actor {actor}>({set});"));
            }
            // A trembling: Sky shows pain (its looped animation), never a shiver.
            Beat::Body(BodyBeat::Tremble { who, frames, .. }) => {
                let Some(actor) = self.who(number, *who) else { return };
                // (Its pain looped, or its hurt one: sweat over the head, as seen in the game.)
                let set = if frames.0 > 0.0 { self.set(*who, 19, true).unwrap_or(14) } else { 2 };
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
                // Its pain once, crouched, then its own attack (group 1, which every sprite has).
                let crouch = self.set(*who, 19, false).unwrap_or(11);
                self.line(&format!("SetAnimation<actor {actor}>({crouch});"));
                self.line("Wait(12);");
                self.line(&format!("SetAnimation<actor {actor}>(13);"));
                // (Aimed at an unseen point, where one dodged from: on no one, it missed.)
                if let Some(Who::Spot(_)) = target {
                    self.line("Wait(20);");
                    self.line(&format!("SetAnimation<actor {actor}>(2);"));
                    return;
                }
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
                if first && (self.top.is_some() || self.tall) {
                    self.line(&format!("screen2_FadeIn(0, {frames});"));
                }
                if let Some(tint) = self.tint.filter(|_| first) {
                    self.flush("Out", 0, tint);
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
            // (Sky's own rain in its scenes: its 16 shows nothing seen over a place.)
            Weather::Rain => 698,
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
            // Someone joins, as Sky tells it: its jingle, the line centred in the box.
            (None, Some("join")) => {
                self.line("me_Play(5);");
                self.line("message_ResetActor();");
                self.line(&format!("message_Mail(\"[CN]{}\");", text.trim_start()));
                self.line("WaitMe(5);");
            }
            (None, Some("narrate")) if text.trim_start().starts_with("[CN]") => {
                self.line(&format!("message_Explanation(\"{}\");", text.trim_start()));
            }
            (None, _) => {
                self.line("message_ResetActor();");
                self.line(&format!("message_Talk(\"{text}\");"));
            }
            (Some(who), _) => {
                let Some(actor) = self.who(number, who) else { return };
                let kind = self.actors.get(&self.became.get(&who).copied().unwrap_or(who)).map_or(0, |one| one.kind);
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
        next.told = std::mem::take(&mut self.told);
        next.tall_asked = self.tall_asked;
        next.vars = std::mem::take(&mut self.vars);
        next.became = std::mem::take(&mut self.became);
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

    /// A scene with more than Sky shows at once (`AT_ONCE`) made to fit:
    /// who comes in later is loaded then, in a layer of its own
    /// (`supervision_Acting(n)`, as Sky's scenes load theirs), in the slots
    /// of those who have left (`Destroy`, as Sky's scenes put away who is
    /// done); and if those there from the start are too many, the crowd's
    /// idle ones farthest from its middle are left out, never one who
    /// speaks, acts or is framed. What it did is told.
    fn fit(&mut self) {
        let counted: Vec<Who> =
            self.actors.iter().filter(|(_, actor)| !actor.name.starts_with("PLAYER") && !actor.name.starts_with("ATTENDANT")).map(|(who, _)| *who).collect();
        if counted.len() <= AT_ONCE {
            return;
        }
        let level = self.level.clone().unwrap_or_default();
        let mut lines: Vec<String> = self.body.lines().map(str::to_string).collect();
        // Each one's first line, last line (none: there to the end), and whether a line names it.
        let mut spans: HashMap<Who, (usize, usize, bool)> = HashMap::new();
        for who in &counted {
            let actor = &self.actors[who];
            let token = format!("ACTOR_{}", actor.name);
            let said: Vec<usize> = lines.iter().enumerate().filter(|(_, line)| names(line, &token)).map(|(at, _)| at).collect();
            let start = if actor.hidden { said.first().copied().unwrap_or(usize::MAX) } else { 0 };
            let gone = format!("SetOutputAttribute<actor {token}>({HIDDEN});");
            let end = said.last().copied().filter(|at| lines[*at].contains(&gone)).unwrap_or(usize::MAX);
            spans.insert(*who, (start, end, !said.is_empty()));
        }
        let mut left_out = Vec::new();
        // Never seen: not loaded.
        for who in &counted {
            if spans[who].0 == usize::MAX {
                left_out.push(*who);
            }
        }
        // There from the start: the crowd's idle ones, farthest from its middle first.
        let first = |actor: &Actor| actor.first.map_or(self.middle.unwrap_or((128.0, 96.0)), |(x, y, _)| (x, y));
        let start: Vec<Who> = counted.iter().copied().filter(|who| spans[who].0 == 0).collect();
        let mut dropped = Vec::new();
        if start.len() > AT_ONCE {
            let count = start.len() as f32;
            let at: Vec<(f32, f32)> = start.iter().map(|who| first(&self.actors[who])).collect();
            let centre = (at.iter().map(|one| one.0).sum::<f32>() / count, at.iter().map(|one| one.1).sum::<f32>() / count);
            let mut idle: Vec<(f32, Who)> = start
                .iter()
                .filter(|who| !spans[*who].2 && !self.framed.contains(who))
                .map(|who| {
                    let (x, y) = first(&self.actors[who]);
                    ((x - centre.0).powi(2) + (y - centre.1).powi(2), *who)
                })
                .collect();
            idle.sort_by(|a, b| b.0.total_cmp(&a.0));
            dropped = idle.into_iter().take(start.len() - AT_ONCE).map(|(_, who)| who).collect();
            let names: Vec<String> = dropped.iter().map(|who| self.game.name(self.actors[who].kind)).collect();
            self.told.push(format!(
                "{level}: {} Pokémon from the start, Sky shows {AT_ONCE} at once: left out {} (idle in the crowd, the farthest from its middle)",
                start.len(),
                if names.is_empty() { "none".to_string() } else { names.join(", ") }
            ));
            if start.len() - dropped.len() > AT_ONCE {
                self.told.push(format!(
                    "{level}: still {} too many, each speaking, acting or framed: the game won't show the last ones of the layout",
                    start.len() - dropped.len() - AT_ONCE
                ));
            }
        }
        // Who comes in later: loaded then, where some have left.
        let mut live: Vec<(Who, usize)> = start.iter().filter(|who| !dropped.contains(who)).map(|who| (*who, spans[who].1)).collect();
        let mut later: Vec<(usize, Who)> = counted.iter().map(|who| (spans[who].0, *who)).filter(|(at, _)| *at > 0 && *at < usize::MAX).collect();
        later.sort();
        let mut inserts: Vec<(usize, Vec<String>)> = Vec::new();
        let mut layer = 0;
        let mut at = 0;
        while at < later.len() {
            let line = later[at].0;
            let group: Vec<Who> = later[at..].iter().take_while(|(start, _)| *start == line).map(|(_, who)| *who).collect();
            at += group.len();
            let mut put = Vec::new();
            for (who, _) in live.iter().filter(|(_, end)| *end < line) {
                put.push(format!("Destroy<actor ACTOR_{}>();", self.actors[who].name));
            }
            live.retain(|(_, end)| *end >= line);
            let room = AT_ONCE.saturating_sub(live.len());
            let (fits, over) = group.split_at(room.min(group.len()));
            dropped.extend(over.iter().copied());
            if !over.is_empty() {
                let names: Vec<String> = over.iter().map(|who| self.game.name(self.actors[who].kind)).collect();
                self.told.push(format!("{level}: no room yet for {} when they come in ({AT_ONCE} at once): left out", names.join(", ")));
            }
            if fits.is_empty() {
                continue;
            }
            layer += 1;
            put.push(format!("supervision_Acting({layer});"));
            for who in fits {
                let actor = self.actors.get_mut(who).expect("an actor of the scene");
                actor.layer = layer;
                put.push(format!("SetOutputAttribute<actor ACTOR_{}>({HIDDEN});", actor.name));
                live.push((*who, spans[who].1));
            }
            inserts.push((line, put));
        }
        if layer > 0 {
            self.told.push(format!("{level}: who comes in later is loaded then, in {layer} more layers of the layout, in the slots of who has left"));
        }
        for (line, put) in inserts.into_iter().rev() {
            let indent: String = lines[line].chars().take_while(|c| c.is_whitespace()).collect();
            for (n, text) in put.into_iter().enumerate() {
                lines.insert(line + n, format!("{indent}{text}"));
            }
        }
        for who in dropped.iter().chain(&left_out) {
            self.actors.remove(who);
        }
        self.body = lines.into_iter().map(|line| line + "\n").collect();
    }

    /// One scene: its level, actors and performers, then its `def 0`.
    fn block(&mut self) -> String {
        self.fit();
        let mut out = String::new();
        let _ = writeln!(out, "// ds-export level {}", self.level.as_deref().unwrap_or("?"));
        let middle = self.middle.unwrap_or((128.0, 96.0));
        for actor in self.actors.values() {
            let (x, y, facing) = actor.first.unwrap_or((middle.0, middle.1, "DIR_DOWN"));
            let species = actor.species.map_or(String::new(), |species| format!(" species {species}"));
            let layer = if actor.layer > 0 { format!(" layer {}", actor.layer) } else { String::new() };
            let _ = writeln!(out, "// ds-export actor ACTOR_{}{species} x {} y {} facing {facing}{layer}", actor.name, tiles(x), tiles(y));
        }
        for (object, (x, y)) in &self.objects {
            let _ = writeln!(out, "// ds-export object OBJECT_N{object} x {} y {}", tiles(*x), tiles(*y));
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
        let hidden: Vec<String> = self.actors.values().filter(|actor| actor.hidden && actor.layer == 0).map(|actor| actor.name.clone()).collect();
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
        if self.top.is_some() || self.tall {
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
    fn each_species_poses_with_its_own_sprites_group() {
        // (The table is read from one's own ROM into the data: where it is.)
        let data = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
        if !game_file(data, OWN_ORDER).exists() {
            return;
        }
        _ = DATA.set(data.to_string());
        // Pikachu and Piplup share the starters' order.
        assert_eq!(own_group(25, 33), Some(33));
        assert_eq!(own_group(428, 33), Some(33));
        // Dusknoir faints lying with its 27 (its own "Faint", 25, rises again).
        assert_eq!(own_group(519, 33), Some(27));
        assert_eq!(own_group(519, 27), Some(27));
        // Snorlax has none of them: asleep for lying, nothing for a nod.
        assert_eq!(own_group(143, 33), Some(5));
        assert_eq!(own_group(143, 22), None);
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

    #[test]
    fn a_crowd_wider_than_the_screen_is_panned_across() {
        assert_eq!(pan(&[(100.0, 100.0), (300.0, 100.0)]), None);
        let (from, to) = pan(&[(100.0, 100.0), (400.0, 110.0)]).expect("too wide");
        assert_eq!((from.0, to.0), (100.0 - 16.0 + 128.0, 400.0 + 16.0 - 128.0));
        assert_eq!(from.1, to.1);
    }

    #[test]
    fn a_line_names_an_actor_not_a_longer_one() {
        assert!(names("Destroy<actor ACTOR_FREE1>();", "ACTOR_FREE1"));
        assert!(!names("Destroy<actor ACTOR_FREE12>();", "ACTOR_FREE1"));
    }
}
