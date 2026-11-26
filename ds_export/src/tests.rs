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
    let tables = tables();
    let written = ::film::compile_with(text, tables.clone());
    let wrong: Vec<String> =
        written.problems.iter().filter(|problem| problem.severity == engine::Severity::Error).map(|problem| problem.message.clone()).collect();
    assert!(wrong.is_empty(), "{wrong:?}");
    let out: PathBuf = std::env::temp_dir().join(format!("ds_export_tests_{}", std::process::id())).join(format!("{name}.exps"));
    let film = Film { name, text, written: &written, tables: &tables, art: None };
    let mut told = Vec::new();
    crate::film(&film, Options { partial: true, tall: false }, &out, &mut |said| told.push(said)).expect("exported");
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
