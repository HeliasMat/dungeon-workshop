//! Small films exported (no ROM: the repository's data, or the tests' own),
//! the `.exps` the writer makes of them read back: what each line becomes.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ::film::tables::Tables;

use crate::{Film, Options};

fn tables() -> Arc<Tables> {
    static READ: OnceLock<Arc<Tables>> = OnceLock::new();
    READ.get_or_init(|| Arc::new(Tables::read(&assets::testing::library(), &assets::testing::data()))).clone()
}

/// The film exported (`--partial`): its script, and what was told.
fn exported(name: &str, text: &str) -> (String, Vec<String>) {
    exported_with(name, text, Options { partial: true, ..Options::default() })
}

/// The same, with those options.
fn exported_with(name: &str, text: &str, options: Options) -> (String, Vec<String>) {
    let tables = tables();
    let written = ::film::compile_with(text, tables.clone());
    let wrong: Vec<String> =
        written.problems.iter().filter(|problem| problem.severity == engine::Severity::Error).map(|problem| problem.message.clone()).collect();
    assert!(wrong.is_empty(), "{wrong:?}");
    let out: PathBuf = std::env::temp_dir().join(format!("ds_export_tests_{}", std::process::id())).join(format!("{name}.exps"));
    let film = Film { name, text, written: &written, tables: &tables, art: None };
    let mut told = Vec::new();
    crate::film(&film, options, &out, &mut |said| told.push(said)).expect("exported");
    (std::fs::read_to_string(&out).expect("written"), told)
}

/// The script's message operations, in order, the speaker's (a portrait,
/// or a name alone where the data has no portrait) as `speaker`.
fn messages(script: &str) -> Vec<String> {
    script
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("message_") || line.starts_with("Wait("))
        .map(|line| if line.starts_with("message_SetFace(") || line.starts_with("message_SetActor(") { "speaker".to_string() } else { line.to_string() })
        .collect()
}

/// A line said in a bubble is a line of Sky's box, closing by itself after
/// the bubble's time; the film's wait for it left out (the box takes it).
#[test]
fn a_bubble_line_is_a_box_line_that_closes_by_itself() {
    let (script, _) = exported("bubbles", "scene: beach pichu munchlax\npichu (bubble, surprised): Whoa...\nmunchlax (bubble): ...\n");
    let said = messages(&script);
    let first = said.iter().position(|line| line.starts_with("message_SetWaitMode(") && line != "message_SetWaitMode(-1, -1);").expect("timed");
    assert_eq!(
        said[first..first + 10],
        [
            "message_SetWaitMode(68, 0);",
            "speaker",
            "message_Talk(\" Whoa...\");",
            "message_Close();",
            "message_SetWaitMode(-1, -1);",
            "message_SetWaitMode(52, 0);",
            "speaker",
            "message_Talk(\" ...\");",
            "message_Close();",
            "message_SetWaitMode(-1, -1);",
        ],
        "{script}"
    );
    // (The face the brackets give, where the data has Pichu's portraits.)
    if script.contains("message_SetFace(") {
        assert!(script.contains("FACE_SURPRISED") || script.contains("FACE_NORMAL"), "{script}");
    }
}

/// A crowd's chatter: Sky's emotes over those chattering, no words.
#[test]
fn a_chatter_is_emotes_over_the_crowd() {
    let (script, told) = exported("chatter", "scene: beach pichu munchlax\nchatter pichu munchlax 4\n");
    let emotes = script.lines().filter(|line| line.contains("SetEffect<actor") && !line.contains("EFFECT_NONE")).count();
    assert_eq!(emotes, 4, "{script}");
    assert!(!script.contains("message_Talk"), "{script}");
    assert!(told.iter().all(|said| !said.contains("Bubble")), "{told:?}");
}

/// What the film loses on DS (`crate::report`, nothing written).
fn report(text: &str) -> crate::Report {
    let tables = tables();
    let written = ::film::compile_with(text, tables.clone());
    crate::report(&Film { name: "report", text, written: &written, tables: &tables, art: None })
}

/// A bubble line is told on its line, changed: a line of Sky's box; a line
/// said as written makes no note.
#[test]
fn a_bubble_line_is_noted_as_changed_on_its_line() {
    let report = report("scene: beach pichu munchlax\npichu (bubble, surprised): Whoa...\nmunchlax: Hi.\n");
    let bubble: Vec<&crate::Note> = report.on_line(2).collect();
    assert_eq!(bubble.len(), 1, "{:?}", report.notes);
    assert_eq!((bubble[0].kind, bubble[0].fate), (crate::Kind::Bubble, crate::Fate::Changed));
    assert!(bubble[0].instead.contains("Sky's box"), "{:?}", bubble[0]);
    assert_eq!(report.on_line(3).count(), 0, "{:?}", report.notes);
    assert!(report.notes.iter().all(|note| !note.message.contains("Bubble {") && !note.message.contains("Speech(")), "no beat dumps: {:?}", report.notes);
}

/// An item held: the word's own loss, once on its line (not its item nor
/// the engine's steps again); one eaten: its bites, once (the word's own
/// loss giving way to it). The summary counts each line once.
#[test]
fn an_item_is_noted_once_on_its_line() {
    let report = report(
        "scene: beach pikachu piplup
pikachu hold Apple
pikachu eat Apple
pikachu (joyous): Yum!
",
    );
    let on = |line: usize| -> Vec<(crate::Kind, crate::Fate)> { report.on_line(line).map(|note| (note.kind, note.fate)).collect() };
    assert_eq!(on(2), [(crate::Kind::Word, crate::Fate::Changed)], "{:?}", report.notes);
    assert_eq!(on(3), [(crate::Kind::Item, crate::Fate::Changed)], "{:?}", report.notes);
    let summary = report.summary();
    assert_eq!((summary.impossible, summary.left_out, summary.changed), (0, 0, 2), "{summary:?}");
    assert!(summary.lines >= 3 && summary.as_written == summary.lines - 2, "{summary:?}");
}

/// A zoom: Sky's camera has none, said by the word's own note, its way
/// round given, the engine's step not told again.
#[test]
fn a_zoom_is_noted_once_with_its_way_round() {
    let report = report(
        "scene: beach pikachu piplup
zoom 2 30f
pikachu: Close-up!
",
    );
    let zoom: Vec<&crate::Note> = report.on_line(2).collect();
    assert_eq!(zoom.len(), 1, "{:?}", report.notes);
    assert!(zoom[0].message.contains("no zoom") && zoom[0].instead.contains("frame"), "{:?}", zoom[0]);
}

/// A pose a sprite lacks: changed on each line it is asked, the nearest
/// pose said instead.
#[test]
fn a_pose_the_sprite_lacks_is_noted_on_each_line() {
    let report = report("scene: beach pikachu piplup\npikachu twirl\npiplup: Again!\npikachu twirl\n");
    for line in [2, 4] {
        let poses: Vec<&crate::Note> = report.on_line(line).filter(|note| note.kind == crate::Kind::Pose).collect();
        assert_eq!(poses.len(), 1, "line {line}: {:?}", report.notes);
        assert_eq!(poses[0].fate, crate::Fate::Changed);
        assert!(poses[0].message.contains("Rotate") && poses[0].instead.ends_with("instead"), "{:?}", poses[0]);
    }
}

/// The lines of the script telling that object something, trimmed.
fn told(script: &str, target: &str) -> Vec<String> {
    script.lines().map(str::trim).filter(|line| line.contains(target)).map(str::to_string).collect()
}

/// An item held is an object of the scene (an item's slot, its icon put in
/// by `patch_rom.py`), at the holder's hands and drawn in front of him, and
/// given each of his moves at his pace.
#[test]
fn an_item_held_goes_where_its_holder_goes() {
    let (script, _) = exported("hold", "scene: beach pikachu munchlax\npikachu put 200 150\npikachu hold Apple\npikachu walk to 260 150\n");
    assert!(script.contains("// ds-export item OBJECT_N11 item 109\n"), "{script}");
    assert!(script.contains("// ds-export object OBJECT_N11 "), "{script}");
    let apple = told(&script, "OBJECT_N11>");
    assert!(apple.iter().any(|line| line.starts_with("MoveHeight<object OBJECT_N11>(16, 10)")), "{apple:?}");
    let walk = script.lines().map(str::trim).find(|line| line.starts_with("MovePositionMark<actor ")).expect("a walk");
    let pace = walk.split_once(">(").and_then(|(_, rest)| rest.split_once(',')).map(|(pace, _)| pace).expect("a pace");
    let carried = apple.iter().find(|line| line.starts_with("MovePositionMark<object OBJECT_N11>(")).expect("the apple goes too");
    assert!(carried.starts_with(&format!("MovePositionMark<object OBJECT_N11>({pace}, ")), "{walk} / {carried}");
}

/// An item given goes up, across and down into the other's hands, then
/// with him.
#[test]
fn an_item_given_arcs_to_the_other_and_stays_with_him() {
    let (script, _) = exported(
        "give",
        "scene: beach pikachu munchlax\npikachu put 200 150\nmunchlax put 240 150\npikachu hold Apple\npikachu give Apple to munchlax\nmunchlax walk to 300 150\n",
    );
    let apple = told(&script, "OBJECT_N11");
    let up = apple.iter().position(|line| line.starts_with("MoveHeight<object OBJECT_N11>(") && line.ends_with(", 26);")).expect("up");
    assert!(apple[up + 1].starts_with("WaitExecuteObject(OBJECT_N11)"), "{apple:?}");
    assert!(apple[up + 2].starts_with("MovePositionMark<object OBJECT_N11>("), "{apple:?}");
    assert!(apple[up + 4].ends_with(", 10);"), "down into his hands: {apple:?}");
    assert!(apple[up + 5..].iter().any(|line| line.starts_with("MovePositionMark<object OBJECT_N11>(")), "goes with him: {apple:?}");
}

/// An item thrown flies to the one it is thrown at, then lands by him.
#[test]
fn an_item_thrown_flies_to_its_target() {
    let (script, told_) = exported("throw", "scene: beach pikachu munchlax\npikachu put 200 150\nmunchlax put 260 150\npikachu throw Apple to munchlax\n");
    let apple = told(&script, "OBJECT_N11");
    assert!(apple.iter().filter(|line| line.starts_with("MovePositionMark<object OBJECT_N11>(")).count() >= 2, "flies, then lands: {apple:?}");
    assert!(apple.last().is_some_and(|line| line.ends_with(", 0);")), "on the ground at last: {apple:?}");
    assert!(!told_.iter().any(|said| said.contains("isn't shown")), "{told_:?}");
}

/// A rider is given each of his mount's moves, at its pace, in front of it;
/// off, he no longer is.
#[test]
fn a_rider_follows_his_mount() {
    let (script, _) = exported(
        "ride",
        "scene: beach pikachu lapras\npikachu put 200 150\nlapras put 240 150\npikachu ride on lapras\nlapras walk to 300 150\npikachu get_off 320 170\nlapras walk to 240 150\n",
    );
    let lines: Vec<&str> = script.lines().map(str::trim).collect();
    let walks: Vec<usize> = lines.iter().enumerate().filter(|(_, line)| line.starts_with("MovePositionMark<actor ")).map(|(at, _)| at).collect();
    let slides: Vec<usize> = lines.iter().enumerate().filter(|(_, line)| line.starts_with("SlidePositionMark<actor ")).map(|(at, _)| at).collect();
    // (His leap onto it, once with the first walk, his leap off it: up, across, down.)
    assert_eq!(slides.len(), 3, "{script}");
    for leap in [slides[0], slides[2]] {
        assert!(lines[leap - 2].starts_with("MoveHeight<actor "), "{script}");
    }
    let walk = walks.iter().copied().rfind(|at| *at < slides[1]).expect("the mount's walk first");
    let pace = |line: &str| line.split_once(">(").and_then(|(_, rest)| rest.split_once(',')).map(|(pace, _)| pace.to_string());
    assert_eq!(pace(lines[walk]), pace(lines[slides[1]]), "{script}");
    assert!(lines[slides[1] - 1].starts_with("SetDirection<actor "), "turned as his mount: {script}");
}

/// An item on a path loops in its object's routine, started by the script;
/// another path of it is another object (an order ends a routine).
#[test]
fn an_item_on_a_path_is_its_objects_routine() {
    let (script, told_) = exported(
        "path",
        "scene: town pikachu\npikachu put 300 242\nput object Apple 286 234\nwait 1s\ntrackpath i109 54 catmull linear loop / 286 234 / 300 188 / 314 234\nwait 2s\nuntrack i109\ntrackpath i109 30 catmull linear / 300 172 / 282 244\nwaittrack i109\n",
    );
    assert!(script.contains("def 1 for object OBJECT_N11 {\n    Lock(19);\n    forever {"), "{script}");
    assert!(script.contains("WaitLockObject(19, OBJECT_N11);\n    Unlock(19);"), "{script}");
    assert!(script.contains("def 2 for object OBJECT_N66 {\n    SetOutputAttribute(128);\n    Lock(18);\n    SetPositionMark("), "{script}");
    assert!(script.contains("SetOutputAttribute<object OBJECT_N11>(128);"), "the first one hidden: {script}");
    assert!(script.contains("WaitLockObject(17, OBJECT_N66);\n    Unlock(17);"), "waited for: {script}");
    assert!(script.contains("    Lock(17);\n    hold;"), "{script}");
    assert!(!told_.iter().any(|said| said.contains("engine's command")), "{told_:?}");
}

/// The lines of the script from the first one starting so on, trimmed.
fn from<'a>(script: &'a str, start: &str) -> Vec<&'a str> {
    let lines: Vec<&str> = script.lines().map(str::trim).collect();
    let at = lines.iter().position(|line| line.starts_with(start)).unwrap_or_else(|| panic!("no {start}: {script}"));
    lines[at..].to_vec()
}

/// A trembling is Sky's own shiver: a pixel right and back, two frames
/// each, eight times, its pain looped; `shiver` (two trembles) shakes once.
#[test]
fn a_trembling_is_skys_shiver() {
    let (script, _) =
        exported("tremble", "scene: beach pikachu munchlax\npikachu put 200 150\nmunchlax put 260 150\npikachu tremble\nmunchlax shiver\nwait 2s\n");
    for actor in ["ACTOR_NPC_TEST001", "ACTOR_NPC_TEST002"] {
        let right = format!("SlidePositionOffset<actor {actor}>(1, 1, 0);");
        let back = format!("SlidePositionOffset<actor {actor}>(1, -1, 0);");
        assert_eq!(script.matches(&right).count(), 8, "{actor}: {script}");
        assert_eq!(script.matches(&back).count(), 8, "{actor}: {script}");
        let shake = from(&script, &right);
        assert_eq!(&shake[..4], &[right.as_str(), "Wait(2);", back.as_str(), "Wait(2);"], "{script}");
    }
}

/// Hurt: knocked back the way he looks, shaken a moment, his pain.
#[test]
fn hurt_knocks_back_and_shakes() {
    let (script, _) = exported("hurt", "scene: beach pikachu\npikachu put 200 150\npikachu hurt\n");
    let knock = from(&script, "SlidePositionMark<actor ACTOR_NPC_TEST001>(");
    assert_eq!(knock[1], "WaitExecuteLives(ACTOR_NPC_TEST001);", "{script}");
    assert_eq!(knock[2], "SlidePositionOffset<actor ACTOR_NPC_TEST001>(1, 1, 0);", "{script}");
    assert_eq!(script.matches("SlidePositionOffset<actor ACTOR_NPC_TEST001>(1, -1, 0);").count(), 3, "{script}");
    // (His pain set before the slide, played as he is knocked back.)
    assert!(script.lines().map(str::trim).any(|line| line.starts_with("SetAnimation<actor ACTOR_NPC_TEST001>(") && !line.ends_with("(2);")), "his pain: {script}");
}

/// A jump to a place is an arc in three: up, across in the air, down; a
/// laugh's hops rise 6 pixels at least, at Sky's hop's pace or faster.
#[test]
fn jumps_and_laughs_arc() {
    let (script, _) = exported("jump", "scene: beach pikachu\npikachu put 200 150\npikachu jump to 260 150\npikachu laugh\n");
    let arc = from(&script, "MoveHeight<actor ACTOR_NPC_TEST001>(2, 16);");
    assert_eq!(arc[1], "WaitExecuteLives(ACTOR_NPC_TEST001);", "{script}");
    assert!(arc[2].starts_with("SlidePositionMark<actor ACTOR_NPC_TEST001>("), "across: {script}");
    assert_eq!(arc[4], "MoveHeight<actor ACTOR_NPC_TEST001>(2, 0);", "down: {script}");
    let hops: Vec<&str> = arc[5..].iter().copied().filter(|line| line.starts_with("MoveHeight<") && !line.ends_with(", 0);")).collect();
    assert_eq!(hops.len(), 2, "{script}");
    for hop in hops {
        let (pace, up) = hop.trim_end_matches(");").split_once(">(").and_then(|(_, rest)| rest.split_once(", ")).expect("pace, height");
        assert!(pace.parse::<f32>().expect("a pace") >= 1.0 && up.parse::<f32>().expect("a height") >= 6.0, "{hop}");
    }
}

/// Staggering: his lost balance and a stumble, a lurch each way and back
/// where he stood.
#[test]
fn a_stagger_stumbles() {
    let (script, _) = exported("stagger", "scene: beach pikachu\npikachu put 200 150\npikachu stagger\n");
    let lurches: Vec<i32> = script
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("SlidePositionOffset<actor ACTOR_NPC_TEST001>("))
        .filter_map(|rest| rest.split(", ").nth(1)?.parse().ok())
        .collect();
    assert_eq!(lurches.len(), 3, "{script}");
    assert_eq!(lurches.iter().sum::<i32>(), 0, "back where he stood: {script}");
}

/// Fading out or in: Sky's blinking (`SetBlink`) in three steps, then
/// hidden or steady; a smoke stays a pop under its cloud.
#[test]
fn vanishing_and_appearing_blink() {
    let (script, _) = exported(
        "blink",
        "scene: beach pikachu munchlax\npikachu put 200 150\nmunchlax put 260 150\npikachu: Hi.\npikachu vanish\nwait 1s\npikachu appear 200 150 fade\nmunchlax vanish smoke\n",
    );
    let out = from(&script, "SetBlink<actor ACTOR_NPC_TEST001>(2, 1);");
    let blinks: Vec<&str> = out.iter().copied().filter(|line| line.starts_with("SetBlink<")).take(4).collect();
    assert_eq!(
        blinks[1..],
        ["SetBlink<actor ACTOR_NPC_TEST001>(1, 1);", "SetBlink<actor ACTOR_NPC_TEST001>(1, 2);", "SetBlink<actor ACTOR_NPC_TEST001>(0, 0);"],
        "{script}"
    );
    let hidden = out.iter().position(|line| *line == "SetOutputAttribute<actor ACTOR_NPC_TEST001>(128);").expect("hidden");
    let shown = out.iter().position(|line| *line == "ResetOutputAttribute<actor ACTOR_NPC_TEST001>(128);").expect("shown");
    assert!(hidden < shown && out[shown + 1] == "SetBlink<actor ACTOR_NPC_TEST001>(1, 2);", "blinking in, the other way: {script}");
    assert!(!script.contains("SetBlink<actor ACTOR_NPC_TEST002>"), "{script}");
}

/// Who goes at once as the scene opens (put, then gone) is hidden from the
/// start: never on its first frames, what it did then left out.
#[test]
fn who_goes_as_the_scene_opens_is_hidden_from_the_start() {
    let (script, _) = exported("opening", "scene: beach pikachu munchlax\npikachu put 200 150\nmunchlax put 260 150\nmunchlax vanish\npikachu: Hi.\n");
    let hide = script.find("SetOutputAttribute<actor ACTOR_NPC_TEST002>(128);").expect("hidden");
    let fade = script.find("screen_FadeIn(1, ").expect("the scene fades in");
    assert!(hide < fade, "{script}");
    assert!(!script.contains("SetBlink<actor ACTOR_NPC_TEST002>"), "{script}");
}

/// From one light to another, the old one goes in half the time and the
/// new one comes in the other half: never the day flashed between.
#[test]
fn one_light_turns_to_another_through_its_halves() {
    let (script, _) = exported("light", "scene: beach pikachu\npikachu put 200 150\npikachu: Hi.\ndaytime evening 2s\nwait 3s\ndaytime night 2s\nwait 3s\n");
    let night = from(&script, "screen_FlushIn(1, 60, 4, 224, 176, 136);");
    assert_eq!(night[1..3], ["Wait(60);", "screen_FlushOut(1, 60, 4, 80, 112, 144);"], "{script}");
    assert!(!script.contains("screen_FlushIn(1, 0, 4, 224, 176, 136);"), "{script}");
}

/// Pikachu and 20 of Sky's largest sprites: over the room the DS has for a
/// scene's sprites, with GroundSpriteRoom or not, with the game's sizes or
/// the guesses (`film::ds::room`).
const LEGENDS: [&str; 21] = [
    "pikachu",
    "groudon",
    "kyogre",
    "rayquaza",
    "dialga",
    "palkia",
    "giratina",
    "lugia",
    "snorlax",
    "wailord",
    "steelix",
    "gyarados",
    "dragonite",
    "tyranitar",
    "metagross",
    "salamence",
    "garchomp",
    "mewtwo",
    "regigigas",
    "heatran",
    "lapras",
];

/// The legends put in a row, then each moment's own say a line.
fn legends(moments: &[&[&str]]) -> String {
    let mut film = format!("scene: town {}\n", LEGENDS.join(" "));
    for (at, one) in LEGENDS.iter().enumerate() {
        film += &format!("{one} put {} 170 facing down\n", 40 + 24 * at);
    }
    for (at, moment) in moments.iter().enumerate() {
        film += &format!("\nmoment: part {}\n", at + 1);
        for one in *moment {
            film += &format!("{one}: I am here.\n");
        }
    }
    film
}

/// The actors of each DS scene the export wrote.
fn scenes_actors(script: &str) -> Vec<usize> {
    script.split("// ds-export level ").skip(1).map(|block| block.lines().filter(|line| line.starts_with("// ds-export actor ")).count()).collect()
}

/// A scene over the room, each moment's species fitting: split at the
/// moment into two DS scenes, each with its own, told; nothing left out.
#[test]
fn a_scene_over_the_sprites_room_splits_at_a_moment() {
    let (script, told) = exported("legends_split", &legends(&[&LEGENDS[..10], &LEGENDS[10..]]));
    assert_eq!(scenes_actors(&script), vec![10, 11], "{script}");
    let said = told.iter().find(|line| line.contains("split into two DS scenes")).expect("told");
    assert!(said.contains("“part 2”") && said.contains("21 species"), "{said}");
    assert!(!told.iter().any(|line| line.contains("magenta") || line.contains("left out, the least needed")), "{told:?}");
    // The second opens on the same place, its Pokémon where they were, faded in.
    let second = script.split("// ds-export level ").nth(2).expect("a second scene");
    assert!(second.starts_with("LEVEL_") && second.contains("supervision_Acting(0);") && second.contains("screen_FadeIn(1, 30);"), "{second}");
    assert!(!second.contains("back_SetTitleBanner"), "{second}");
}

/// A scene over the room that no moment splits (all of them speak in one):
/// the least needed left out, told with their sizes. `--squares` keeps
/// them all, neither split nor left out, told as squares.
#[test]
fn a_scene_no_moment_splits_leaves_out_the_least_needed() {
    let film = legends(&[&LEGENDS[..2], &LEGENDS]);
    let (script, told) = exported("legends_trim", &film);
    let actors = scenes_actors(&script);
    assert!(actors.len() == 1 && actors[0] < 21, "{script}");
    let said = told.iter().find(|line| line.contains("left out, the least needed first")).expect("told");
    assert!(said.contains("KB") && said.contains("split the scene"), "{said}");
    let (script, told) = exported_with("legends_squares", &film, Options { partial: true, squares: true, ..Options::default() });
    assert_eq!(scenes_actors(&script), vec![21], "{script}");
    assert!(told.iter().any(|line| line.contains("drawn as magenta squares")), "{told:?}");
    let (script, _) =
        exported_with("legends_kept", &legends(&[&LEGENDS[..10], &LEGENDS[10..]]), Options { partial: true, squares: true, ..Options::default() });
    assert_eq!(scenes_actors(&script), vec![21], "not split either: {script}");
}
