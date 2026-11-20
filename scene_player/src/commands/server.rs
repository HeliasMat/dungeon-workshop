//! `scene_player server`: a process that stays, for an agent. One request
//! a line on its input, one answer a line on its output, both JSON (a
//! request may also be written as words: `state 32s`). It keeps the film
//! loaded, compiled and played through once, with the film as it stands
//! every [`CHECKPOINT`] frames: a moment asked is played from the last one
//! before it, not from the start.
//!
//! ```text
//! {"command":"load","film":"example"}           or {"command":"load","text":"scene: beach pikachu\n..."}
//! {"command":"analyze"}   {"command":"check"}   {"command":"timing"}
//! {"command":"state","t":"32s"}                 (t: "32s", 32, "line 90", "scene 3 end", or a list)
//! {"command":"image","t":"32s","path":"x.png"}
//! {"command":"properties","kind":"halo"}        {"command":"schema"}   {"command":"resources","kind":"sounds"}
//! {"command":"quit"}
//! ```
//!
//! Each answer is the command's own JSON (as with `--json`) with
//! `"ok": true`, or `{"command", "ok": false, "code", "message"}`; an `id`
//! given in the request is given back.

use ::film::Context;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

use preview::art::Art;
use preview::film::Snapshot;
use preview::played::{Pass, SECOND, Timeline};
use shell::Screen;
use stage_view::ui::Canvas;

/// The server's codes of what it refuses (the same rule as every code:
/// `E-` an error, never renamed nor given another meaning).
pub mod codes {
    /// A line the server could not read as a request.
    pub const SERVER_REQUEST: &str = "E-SERVER-REQUEST";
    /// A request the server has no command for.
    pub const SERVER_COMMAND: &str = "E-SERVER-COMMAND";
    /// A request that needs a film, and none is loaded (`load`).
    pub const SERVER_NO_FILM: &str = "E-SERVER-NO-FILM";
    /// A request the server read, and that failed (its message says why).
    pub const SERVER_FAILED: &str = "E-SERVER-FAILED";

    /// Every code, with what it means.
    pub const ALL: &[(&str, &str)] = &[
        (SERVER_REQUEST, "server: a line that is not a request"),
        (SERVER_COMMAND, "server: no command of this name"),
        (SERVER_NO_FILM, "server: no film loaded (load one first)"),
        (SERVER_FAILED, "server: the request failed (its message says why)"),
    ];

    /// Every code an agent may meet, with what it means: the engine's,
    /// the films', the server's, then those of what a rehearsal sees.
    pub fn every() -> Vec<(&'static str, &'static str)> {
        let film = ::film::codes::ALL.iter().chain(ALL);
        engine::codes::ALL.iter().chain(film).copied().chain(engine::codes::seen_all()).collect()
    }
}

use super::Piece;
use super::json::{self, Value, json};
use crate::moment;

/// How often the film is kept as it stands, in frames: five seconds.
pub const CHECKPOINT: usize = 5 * SECOND;

/// How much bigger than the DS a picture asked is.
const PICTURE_SCALE: usize = 4;

/// A request, read.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Load { film: Option<String>, text: Option<String>, name: Option<String> },
    Analyse,
    Check { play: bool },
    State { moments: Vec<String> },
    Chrono { moments: Vec<String> },
    Picture { moment: Vec<String>, path: String },
    Properties { kind: Option<String> },
    Schema,
    Resources { sort: Option<String> },
    Quit,
}

impl Request {
    /// Its command's word.
    pub fn word(&self) -> &'static str {
        match self {
            Request::Load { .. } => "load",
            Request::Analyse => "analyze",
            Request::Check { .. } => "check",
            Request::State { .. } => "state",
            Request::Chrono { .. } => "timing",
            Request::Picture { .. } => "image",
            Request::Properties { .. } => "properties",
            Request::Schema => "schema",
            Request::Resources { .. } => "resources",
            Request::Quit => "quit",
        }
    }
}

/// What is wrong with a request: its code and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Refused {
    pub code: &'static str,
    pub message: String,
}

fn refused(code: &'static str, message: impl Into<String>) -> Refused {
    Refused { code, message: message.into() }
}

/// The words of a moment, as `moment::read` takes them: `"32s"`, `32`,
/// `"line 90"`, or a list of them.
fn moment_words(value: &Value) -> Vec<String> {
    match value {
        Value::Number(seconds) => vec![format!("{seconds}s")],
        Value::String(said) => said.split_whitespace().map(str::to_string).collect(),
        Value::Array(many) => many.iter().flat_map(moment_words).collect(),
        _ => Vec::new(),
    }
}

/// The commands a request may name.
pub const COMMANDS: [&str; 10] = ["load", "analyze", "check", "state", "timing", "image", "properties", "schema", "resources", "quit"];

/// A line read as a request: JSON (`{"command": …}`), or words.
pub fn parse(line: &str) -> Result<(Request, Option<Value>), Refused> {
    let line = line.trim();
    let (command, fields): (String, Value) = match line.starts_with('{') {
        true => {
            let value: Value = serde_json::from_str(line).map_err(|error| refused(codes::SERVER_REQUEST, format!("unreadable JSON: {error}")))?;
            let command = value
                .get("command")
                .or_else(|| value.get("cmd"))
                .and_then(Value::as_str)
                .ok_or_else(|| refused(codes::SERVER_REQUEST, "“command” is missing"))?
                .to_string();
            (command, value)
        }
        false => {
            let mut words = line.split_whitespace();
            let command = words.next().ok_or_else(|| refused(codes::SERVER_REQUEST, "an empty line"))?.to_string();
            let rest: Vec<&str> = words.collect();
            let fields = match command.as_str() {
                "load" => json!({"film": rest.first()}),
                "state" | "timing" => json!({"t": rest.join(" ")}),
                "image" => json!({"t": rest[..rest.len().saturating_sub(1)].join(" "), "path": rest.last()}),
                "properties" => json!({"kind": rest.first()}),
                "resources" => json!({"kind": rest.first()}),
                "check" => json!({"play": !rest.contains(&"--no-play")}),
                _ => json!({}),
            };
            (command, fields)
        }
    };
    let id = fields.get("id").cloned();
    let text = |field: &str| fields.get(field).and_then(Value::as_str).map(str::to_string);
    let moments = || fields.get("t").or_else(|| fields.get("moment")).map(moment_words).unwrap_or_default();
    let request = match command.as_str() {
        "load" => {
            let (film, given) = (text("film"), text("text"));
            if film.is_none() && given.is_none() {
                return Err(refused(codes::SERVER_REQUEST, "load: “film” (a name, a path) or “text”"));
            }
            Request::Load { film, text: given, name: text("name") }
        }
        "analyze" => Request::Analyse,
        "check" => Request::Check { play: fields.get("play").and_then(Value::as_bool).unwrap_or(true) },
        "state" if moments().is_empty() => return Err(refused(codes::SERVER_REQUEST, "state: what moment? “t”: 32s, line 90, scene 3 end")),
        "state" => Request::State { moments: moments() },
        "timing" => Request::Chrono { moments: moments() },
        "image" => match (moments(), text("path")) {
            (moment, Some(path)) if !moment.is_empty() => Request::Picture { moment, path },
            _ => return Err(refused(codes::SERVER_REQUEST, "image: “t” and “path” (a .png)")),
        },
        "properties" => Request::Properties { kind: text("kind") },
        "schema" => Request::Schema,
        "resources" => Request::Resources { sort: text("kind") },
        "quit" => Request::Quit,
        other => {
            let near = ::film::check::closest(other, COMMANDS.iter().copied(), 2, 0.5);
            let near = near.first().map_or(String::new(), |near| format!(" Did you mean {near}?"));
            return Err(refused(codes::SERVER_COMMAND, format!("no command “{other}”.{near} Commands: {}", COMMANDS.join(", "))));
        }
    };
    Ok((request, id))
}

/// A film loaded: played through once, kept as it stood every
/// [`CHECKPOINT`] frames, and what it played to be heard.
struct Loaded {
    piece: Piece,
    pass: Pass,
    timeline: Timeline,
    kept: Vec<(usize, Snapshot)>,
    heard: json::Heard,
}

impl Loaded {
    fn new(piece: Piece, art: Art) -> Result<Loaded, String> {
        let heard = Arc::new(Mutex::new(json::Heard { keeping: true, ..json::Heard::default() }));
        let mut pass = Pass::new(piece.film_with(art, Some(Box::new(json::Listener(heard.clone()))))?);
        let mut kept = Vec::new();
        loop {
            if pass.frame.is_multiple_of(CHECKPOINT) {
                kept.push((pass.frame, pass.film.snapshot()));
            }
            if !pass.step() {
                break;
            }
        }
        let timeline = pass.timeline.clone();
        // (What it plays is heard once: played again from a moment, not kept twice.)
        let heard = {
            let mut shared = heard.lock().map_err(|_| "the sound")?;
            shared.keeping = false;
            shared.clone()
        };
        Ok(Loaded { piece, pass, timeline, kept, heard })
    }

    /// The film at that frame: from the last moment kept before it, if it
    /// is not on its way there already.
    fn go_to(&mut self, frame: usize) {
        let from = self.kept.iter().rev().find(|(at, _)| *at <= frame);
        if let Some((at, moment)) = from
            && (self.pass.frame > frame || *at > self.pass.frame)
        {
            self.pass.film.restore(moment);
            self.pass.frame = *at;
        }
        while self.pass.frame < frame && self.pass.step() {}
    }

    fn frames(&self, moments: &[String]) -> Result<Vec<(usize, String)>, String> {
        let read = moment::read(moments)?;
        read.iter().map(|moment| Ok((moment.frame(&self.timeline)?, moment.name()))).collect()
    }
}

/// The server's state: the game's files, the film loaded.
pub struct Server {
    context: Arc<Context>,
    data: String,
    loaded: Option<Loaded>,
    /// How many films were given as text, for their names.
    given: usize,
}

impl Server {
    pub fn new(context: &Arc<Context>) -> Server {
        Server { context: context.clone(), data: context.data.clone(), loaded: None, given: 0 }
    }

    fn loaded(&mut self) -> Result<&mut Loaded, Refused> {
        self.loaded.as_mut().ok_or_else(|| refused(codes::SERVER_NO_FILM, "no film loaded: {\"command\":\"load\",\"film\":\"…\"}"))
    }

    /// A request answered: its answer, and whether to stop.
    pub fn answer(&mut self, request: Request) -> Result<(Value, bool), Refused> {
        let failed = |why: String| refused(codes::SERVER_FAILED, why);
        let data = self.data.clone();
        let context = self.context.clone();
        let made = match request {
            Request::Quit => return Ok((json::answer("quit", json!({})), true)),
            Request::Properties { kind } => super::agents::properties_value(kind.as_deref()).map_err(failed)?,
            Request::Schema => super::agents::schema_value(),
            Request::Resources { sort } => super::resources::resources_value(sort.as_deref(), &context).map_err(failed)?,
            Request::Load { film, text, name } => {
                let piece = match (film, text) {
                    (_, Some(text)) => {
                        self.given += 1;
                        Piece::of_text(&context, &text, &name.unwrap_or_else(|| format!("text_{}", self.given)))
                    }
                    (Some(film), None) => Piece::find(&context, &film).map_err(failed)?,
                    (None, None) => return Err(refused(codes::SERVER_REQUEST, "load: “film” or “text”")),
                };
                // (The art read again for each film: a film's pictures are its own.)
                let art = piece.art().map_err(failed)?;
                self.loaded = None;
                let loaded = Loaded::new(piece, art).map_err(failed)?;
                let told = json!({
                    "film": loaded.piece.name,
                    "path": json::path(loaded.piece.tables.project.root(), &loaded.piece.path),
                    "duration_s": json::seconds(loaded.timeline.frames),
                    "scenes": loaded.timeline.scenes.len(),
                    "points_kept": loaded.kept.len(),
                });
                self.loaded = Some(loaded);
                json::answer("load", told)
            }
            Request::Analyse => {
                let loaded = self.loaded()?;
                super::language::analysis(&context, &loaded.piece.path, &loaded.piece.text)
            }
            Request::Check { play } => {
                let loaded = self.loaded()?;
                json::answer("check", json!({"films": [super::checking::checked(&loaded.piece, &data, play)]}))
            }
            Request::Chrono { moments } => {
                let loaded = self.loaded()?;
                super::chrono::chrono_value(&loaded.piece, &moments).map_err(failed)?
            }
            Request::State { moments } => {
                let loaded = self.loaded()?;
                let mut wanted = loaded.frames(&moments).map_err(failed)?;
                wanted.sort();
                let mut told = Vec::new();
                for (frame, name) in wanted {
                    loaded.go_to(frame);
                    let mut moment = super::looking::moment_json(&loaded.piece, &loaded.pass.film, frame, &loaded.timeline, Some(&loaded.heard));
                    moment["asked"] = json!(name);
                    told.push(moment);
                }
                json::answer("state", json!({"film": loaded.piece.name, "duration_s": json::seconds(loaded.timeline.frames), "moments": told}))
            }
            Request::Picture { moment, path } => {
                let loaded = self.loaded()?;
                let (frame, _) = *loaded.frames(&moment).map_err(failed)?.first().ok_or_else(|| failed("what moment?".to_string()))?;
                loaded.go_to(frame);
                let (mut canvas, mut screen) = (Canvas::default(), Screen::new(PICTURE_SCALE));
                loaded.pass.film.draw(&mut canvas, &mut screen);
                let rgb = super::pictures::rgb(&screen);
                image::save_buffer(&path, &rgb, screen.wide as u32, screen.high as u32, image::ColorType::Rgb8)
                    .map_err(|error| failed(format!("{path}: {error}")))?;
                json::answer("image", json!({"path": path, "second": json::seconds(frame), "size": [screen.wide, screen.high]}))
            }
        };
        Ok((made, false))
    }

    /// A line of the input answered as a line of the output; whether to stop.
    pub fn line(&mut self, line: &str) -> (String, bool) {
        let (answer, stop) = match parse(line) {
            Ok((request, id)) => {
                let command = request.word();
                let (mut answer, stop) = match self.answer(request) {
                    Ok((answer, stop)) => (answer, stop),
                    Err(why) => (refusal(command, &why), false),
                };
                answer["ok"] = json!(answer.get("code").is_none());
                if let Some(id) = id {
                    answer["id"] = id;
                }
                (answer, stop)
            }
            Err(why) => (refusal("", &why), false),
        };
        (answer.to_string(), stop)
    }
}

/// A request refused, as the server answers it.
fn refusal(command: &str, why: &Refused) -> Value {
    json!({"command": command, "shape": json::SHAPE, "ok": false, "code": why.code, "message": why.message})
}

/// `server`: requests read a line at a time until `quit` or the end.
pub fn serve(context: &Arc<Context>) -> Result<(), String> {
    let mut server = Server::new(context);
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| error.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let (answer, stop) = server.line(&line);
        writeln!(out, "{answer}").and_then(|()| out.flush()).map_err(|error| error.to_string())?;
        if stop {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::testing::{root, tested};

    /// Every problem code (each block's: `codes::every`),
    /// as FILMS.md's “For agents” has them (`GOLDEN=write` writes it anew).
    #[test]
    fn the_codes_are_in_films_md_as_the_code_has_them() {
        const START: &str = "<!-- codes: written by scene_player/src/commands/server.rs, from each block's codes -->";
        const END: &str = "<!-- end of codes -->";
        let mut codes = String::from("| Code | What it says |\n|---|---|\n");
        for (code, meaning) in codes::every() {
            codes.push_str(&format!("| `{code}` | {meaning} |\n"));
        }
        let made = format!("{START}\n{codes}{END}");
        let path = root().join("film/FILMS.md");
        let text = std::fs::read_to_string(&path).expect("FILMS.md").replace("\r\n", "\n");
        let from = text.find("<!-- codes: written by").expect("the table of the codes");
        let to = text.find(END).expect("its end") + END.len();
        if std::env::var("GOLDEN").is_ok_and(|asked| asked == "write") {
            std::fs::write(&path, text.replace(&text[from..to], &made)).expect("written");
            return;
        }
        assert_eq!(&text[from..to], made, "the table of the codes of film/FILMS.md is no longer the code's: GOLDEN=write cargo test -p scene_player codes");
    }

    #[test]
    fn requests_are_read_as_json_or_as_words() {
        assert_eq!(parse(r#"{"command":"state","t":"32s","id":7}"#), Ok((Request::State { moments: vec!["32s".to_string()] }, Some(json!(7)))));
        assert_eq!(
            parse(r#"{"cmd":"state","t":[12, "line 9"]}"#).map(|read| read.0),
            Ok(Request::State { moments: vec!["12s".into(), "line".into(), "9".into()] })
        );
        assert_eq!(parse("state scene 3 end").map(|read| read.0), Ok(Request::State { moments: vec!["scene".into(), "3".into(), "end".into()] }));
        assert_eq!(parse("image 32s out.png").map(|read| read.0), Ok(Request::Picture { moment: vec!["32s".to_string()], path: "out.png".to_string() }));
        assert_eq!(
            parse(r#"{"command":"load","text":"scene: beach a"}"#).map(|read| read.0),
            Ok(Request::Load { film: None, text: Some("scene: beach a".into()), name: None })
        );
        assert_eq!(parse("quit").map(|read| read.0), Ok(Request::Quit));
        assert_eq!(parse("check --no-play").map(|read| read.0), Ok(Request::Check { play: false }));
    }

    #[test]
    fn a_request_not_read_is_told_with_its_code() {
        assert_eq!(parse("{not json").map_err(|why| why.code), Err(codes::SERVER_REQUEST));
        assert_eq!(parse(r#"{"t":1}"#).map_err(|why| why.code), Err(codes::SERVER_REQUEST));
        assert_eq!(parse("state").map_err(|why| why.code), Err(codes::SERVER_REQUEST));
        assert_eq!(parse("load").map_err(|why| why.code), Err(codes::SERVER_REQUEST));
        let unknown = parse("stat 3s").expect_err("no such command");
        assert!(unknown.code == codes::SERVER_COMMAND && unknown.message.contains("state"), "{unknown:?}");
    }

    fn read(line: &str) -> Value {
        serde_json::from_str(line).expect("one line of JSON")
    }

    #[test]
    fn the_server_answers_a_line_with_a_line() {
        let mut server = Server::new(&tested("nowhere"));
        let (said, stop) = server.line(r#"{"command":"properties","kind":"halo","id":"a"}"#);
        let said = read(&said);
        assert!(!stop && said["ok"] == json!(true) && said["id"] == json!("a") && said["command"] == json!("properties"), "{said}");
        let (said, _) = server.line(r#"{"command":"state","t":"1s"}"#);
        assert_eq!((read(&said)["ok"].clone(), read(&said)["code"].clone()), (json!(false), json!(codes::SERVER_NO_FILM)));
        let (said, _) = server.line("whatever");
        assert_eq!(read(&said)["code"], json!(codes::SERVER_COMMAND));
        let (said, stop) = server.line("quit");
        assert!(stop && read(&said)["ok"] == json!(true));
    }

    /// A small film given as text: loaded, read, its moments asked from the
    /// moments kept. (Where the game's files are.)
    #[test]
    fn a_film_given_as_text_goes_round() {
        let data = root().join("data").to_string_lossy().into_owned();
        if !std::path::Path::new(&data).join("rom").exists() {
            return;
        }
        let mut server = Server::new(&tested(&data));
        let film = "title: Trial\\nscene: beach pikachu\\npikachu put 300 200\\nwait 6s\\npikachu: Hello!\\nwait 2s\\n";
        let (said, _) = server.line(&format!(r#"{{"command":"load","text":"{film}","name":"trial_server"}}"#));
        let said = read(&said);
        assert_eq!(said["ok"], json!(true), "{said}");
        assert!(said["duration_s"].as_f64().is_some_and(|seconds| seconds > 6.0) && said["points_kept"].as_u64().is_some_and(|kept| kept >= 2), "{said}");
        let (analysed, _) = server.line("analyze");
        assert_eq!(read(&analysed)["command"], json!("analyze"));
        // (Later then sooner: from the moment kept before each.)
        let (later, _) = server.line(r#"{"command":"state","t":"7s"}"#);
        let (sooner, _) = server.line(r#"{"command":"state","t":"2s"}"#);
        let (later, sooner) = (read(&later), read(&sooner));
        let pikachu =
            |said: &Value| said["moments"][0]["nodes"].as_array().and_then(|nodes| nodes.iter().find(|node| node["name"] == json!("pikachu")).cloned());
        let at = pikachu(&sooner).unwrap_or_else(|| panic!("pikachu at 2 s: {sooner}"));
        assert_eq!(at["properties"]["position"], json!([300.0, 200.0]), "{at}");
        assert_eq!(at["kind"], json!("body"));
        assert!(pikachu(&later).is_some() && later["moments"][0]["line"].as_i64().is_some(), "{later}");
        // (The same moment asked twice: the same.)
        let (again, _) = server.line(r#"{"command":"state","t":"2s"}"#);
        assert_eq!(read(&again)["moments"], sooner["moments"]);
        // (A moment kept, asked after a later one: as a pass from the start has it.)
        let (kept, _) = server.line(r#"{"command":"state","t":"5s"}"#);
        let loaded = server.loaded.as_ref().expect("loaded");
        let mut fresh = Pass::new(loaded.piece.film().expect("the film"));
        while fresh.frame < CHECKPOINT && fresh.step() {}
        let from_start = super::super::looking::moment_json(&loaded.piece, &fresh.film, CHECKPOINT, &loaded.timeline, Some(&loaded.heard));
        assert_eq!(read(&kept)["moments"][0]["nodes"], from_start["nodes"]);
        assert_eq!(read(&kept)["moments"][0]["cameras"], from_start["cameras"]);
    }
}
