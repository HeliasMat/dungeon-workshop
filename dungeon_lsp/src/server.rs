//! What the server answers: the open texts kept whole (full sync), checked
//! at each change; each request answered by the film service (a film) or
//! by the readers of a game and a decor. A `.game` opened or saved is
//! checked whole too (`game::project_check`: its films, places,
//! dungeons, fights): what is wrong is told on the lines of the files it is
//! in, for the `.game` and the files open in the editor.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use assets::project::{Project, find_root, is_root};
use film::Context;
use film::service::{self, Spot};
use serde_json::{Value, json};

use crate::text;

/// What a file is, by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Film,
    Game,
    Decor,
}

impl Kind {
    pub fn of(uri: &str) -> Kind {
        if uri.ends_with(".game") {
            Kind::Game
        } else if uri.ends_with(".decor") {
            Kind::Decor
        } else {
            Kind::Film
        }
    }
}

/// What is wrong, as the protocol shows it, and its fixes as edits.
struct Found {
    diagnostic: Value,
    fixes: Vec<(String, Vec<Value>)>,
}

struct Doc {
    text: String,
    kind: Kind,
    found: Vec<Found>,
}

/// The server: the texts open in the editor; what the whole-game checks
/// found, by the file it is in (`file_key`): the game it came from, its
/// line, its message; what it reads them with (the root's library, the
/// data folder, the tables read: made again at `initialize` from the
/// editor's folder, read again when a file is saved).
pub struct Server {
    docs: HashMap<String, Doc>,
    whole: HashMap<String, Vec<(String, usize, String)>>,
    shut: bool,
    context: Arc<Context>,
}

impl Default for Server {
    fn default() -> Self {
        Server { docs: HashMap::new(), whole: HashMap::new(), shut: false, context: context_from(None) }
    }
}

/// The context of the root up from that folder (the editor's), else up from
/// where the server runs or from the server itself; none found, an empty
/// library (only the words of the language are known then).
fn context_from(start: Option<&Path>) -> Arc<Context> {
    let root = start.and_then(|start| start.ancestors().find(|folder| is_root(folder)).map(Path::to_path_buf)).or_else(find_root);
    let library = root.as_deref().map_or_else(Project::default, Project::library);
    let data = assets::data::folder(library.root());
    Arc::new(Context::new(library, data))
}

/// A file as the whole-game checks are kept by: its path made whole, in
/// small letters, with forward slashes.
fn file_key(path: &std::path::Path) -> String {
    let whole = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let said = whole.display().to_string().replace('\\', "/").to_lowercase();
    said.strip_prefix("//?/").map_or(said.clone(), str::to_string)
}

/// The key of a document by its address.
fn uri_key(uri: &str) -> Option<String> {
    text::path_of(uri).map(|path| file_key(&path))
}

/// What a message gives: the messages to send, and the code to end with.
#[derive(Default)]
pub struct Reply {
    pub out: Vec<Value>,
    pub exit: Option<i32>,
}

impl Server {
    /// Whether the editor asked to shut down.
    pub fn shut(&self) -> bool {
        self.shut
    }

    pub fn handle(&mut self, message: &Value) -> Reply {
        let mut reply = Reply::default();
        let method = message["method"].as_str().unwrap_or("");
        let params = &message["params"];
        let Some(id) = message.get("id").cloned() else {
            self.notified(method, params, &mut reply);
            return reply;
        };
        if method.is_empty() {
            // (An answer to a request of ours: there are none.)
            return reply;
        }
        let answer = match method {
            "initialize" => {
                // (The root up from the editor's folder, if it is in one.)
                let root = params["rootUri"].as_str().and_then(text::path_of).or_else(|| params["rootPath"].as_str().map(Into::into));
                self.context = context_from(root.as_deref());
                Ok(initialize())
            }
            "shutdown" => {
                self.shut = true;
                Ok(Value::Null)
            }
            "textDocument/completion" => Ok(self.on_doc(params, completion)),
            "textDocument/hover" => Ok(self.on_doc(params, hover)),
            "textDocument/definition" => Ok(self.on_doc(params, definition)),
            "textDocument/codeAction" => Ok(self.code_actions(params)),
            "textDocument/formatting" => Ok(self.on_doc(params, formatting)),
            "textDocument/documentSymbol" => Ok(self.on_doc(params, symbols)),
            "textDocument/foldingRange" => Ok(self.on_doc(params, folds)),
            _ => Err(json!({ "code": -32601, "message": format!("not known: {method}") })),
        };
        reply.out.push(match answer {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
        });
        reply
    }

    fn notified(&mut self, method: &str, params: &Value, reply: &mut Reply) {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("").to_string();
        match method {
            "exit" => reply.exit = Some(if self.shut { 0 } else { 1 }),
            "textDocument/didOpen" => {
                let text = params["textDocument"]["text"].as_str().unwrap_or("").to_string();
                self.changed(uri.clone(), text, reply);
                self.whole_game(&uri, reply);
            }
            "textDocument/didChange" => {
                // (Full sync: the last change is the whole text.)
                if let Some(text) = params["contentChanges"].as_array().and_then(|changes| changes.last()).and_then(|change| change["text"].as_str()) {
                    self.changed(uri, text.to_string(), reply);
                }
            }
            "textDocument/didSave" => {
                // (A file it uses may have changed: the tables read again, checked again.)
                self.context.reload();
                if let Some(doc) = self.docs.get(&uri) {
                    let text = doc.text.clone();
                    self.changed(uri.clone(), text, reply);
                    self.whole_game(&uri, reply);
                }
            }
            "textDocument/didClose" => {
                self.docs.remove(&uri);
                reply.out.push(published(&uri, Vec::new()));
            }
            _ => {}
        }
    }

    fn changed(&mut self, uri: String, text: String, reply: &mut Reply) {
        let kind = Kind::of(&uri);
        let found = check(&self.context, &text, kind, &uri);
        self.docs.insert(uri.clone(), Doc { text, kind, found });
        reply.out.push(self.told(&uri));
    }

    /// What is wrong with an open document: its own, and what the
    /// whole-game checks found in it (not again on a line its own tells).
    fn told(&self, uri: &str) -> Value {
        let Some(doc) = self.docs.get(uri) else { return published(uri, Vec::new()) };
        let mut diagnostics: Vec<Value> = doc.found.iter().map(|one| one.diagnostic.clone()).collect();
        let lines: Vec<u64> = diagnostics.iter().filter_map(|one| one["range"]["start"]["line"].as_u64()).collect();
        for (_, line, message) in uri_key(uri).and_then(|key| self.whole.get(&key)).into_iter().flatten() {
            if !lines.contains(&(line.saturating_sub(1) as u64)) {
                diagnostics.push(diagnostic(text::line_range(&doc.text, *line), &language::Diagnostic::said(message.clone()).coded("game")));
            }
        }
        published(uri, diagnostics)
    }

    /// A `.game` checked whole, from its file: what was found before for it
    /// let go, what is found now kept by file; the open documents it
    /// touches told again.
    fn whole_game(&mut self, uri: &str, reply: &mut Reply) {
        let (Kind::Game, Some(path)) = (Kind::of(uri), text::path_of(uri)) else { return };
        if !path.is_file() {
            return;
        }
        let game = file_key(&path);
        let Ok(report) = game::project_check::check(&self.context, &path) else { return };
        let mut touched: Vec<String> = Vec::new();
        for (key, found) in &mut self.whole {
            let before = found.len();
            found.retain(|(from, ..)| *from != game);
            if found.len() != before {
                touched.push(key.clone());
            }
        }
        touched.push(game.clone());
        for problem in report.problems {
            let key = file_key(&game::project_check::file_of(&problem));
            touched.push(key.clone());
            self.whole.entry(key).or_default().push((game.clone(), problem.line().max(1), problem.message));
        }
        let open: Vec<String> = self.docs.keys().filter(|open| *open != uri && uri_key(open).is_some_and(|key| touched.contains(&key))).cloned().collect();
        reply.out.push(self.told(uri));
        for other in open {
            reply.out.push(self.told(&other));
        }
    }

    fn on_doc(&self, params: &Value, answer: fn(&Context, &str, Kind, &str, &Value) -> Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        match self.docs.get(uri) {
            Some(doc) => answer(&self.context, &doc.text, doc.kind, uri, params),
            None => Value::Null,
        }
    }

    /// The fixes of what is wrong on the lines asked about.
    fn code_actions(&self, params: &Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(doc) = self.docs.get(uri) else { return json!([]) };
        let (first, last) = (params["range"]["start"]["line"].as_u64().unwrap_or(0), params["range"]["end"]["line"].as_u64().unwrap_or(0));
        let mut actions = Vec::new();
        for found in &doc.found {
            let range = &found.diagnostic["range"];
            let (start, end) = (range["start"]["line"].as_u64().unwrap_or(0), range["end"]["line"].as_u64().unwrap_or(0));
            if end < first || start > last {
                continue;
            }
            for (title, edits) in &found.fixes {
                actions.push(json!({
                    "title": title,
                    "kind": "quickfix",
                    "diagnostics": [found.diagnostic],
                    "edit": { "changes": { uri: edits } },
                }));
            }
        }
        Value::Array(actions)
    }
}

fn published(uri: &str, diagnostics: Vec<Value>) -> Value {
    json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": { "uri": uri, "diagnostics": diagnostics } })
}

fn initialize() -> Value {
    json!({
        "capabilities": {
            "textDocumentSync": 1,
            "completionProvider": { "triggerCharacters": [" ", "(", ","] },
            "hoverProvider": true,
            "definitionProvider": true,
            "codeActionProvider": true,
            "documentFormattingProvider": true,
            "documentSymbolProvider": true,
            "foldingRangeProvider": true,
        },
        "serverInfo": { "name": "dungeon_lsp", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// The protocol's weight of a diagnostic.
fn severity(severity: language::Severity) -> u8 {
    match severity {
        language::Severity::Error => 1,
        language::Severity::Remark => 2,
        language::Severity::Detail => 3,
    }
}

/// A diagnostic as the protocol has it, over that range: its notes after its message.
fn diagnostic(range: Value, told: &language::Diagnostic) -> Value {
    let mut message = told.message.clone();
    for note in &told.notes {
        message.push('\n');
        message.push_str(note);
    }
    json!({ "range": range, "severity": severity(told.severity), "code": told.code.unwrap_or_default(), "source": "dungeon", "message": message })
}

/// A diagnostic whose lines and columns are worked out, and its fixes as edits of the protocol.
fn found(text: &str, told: &language::Diagnostic) -> Found {
    let range = |at: &language::Location| text::range(text, at.start, at.end);
    let fixes = told
        .fixes
        .iter()
        .map(|fix| (fix.title.clone(), fix.edits.iter().map(|edit| json!({ "range": range(&edit.span), "newText": edit.text })).collect()))
        .collect();
    Found { diagnostic: diagnostic(range(&told.span), told), fixes }
}

/// The blocks a game's file opens (`on …`, a chapter at a line's start):
/// as the game reads it.
fn game_holds(text: &str) -> impl Fn(&language::ast::Call) -> bool + '_ {
    move |call| match call.word.text.as_str() {
        "on" => true,
        "chapter" => call.word.span.start == 0 || text[..call.word.span.start].ends_with('\n'),
        _ => false,
    }
}

/// What is wrong with a text, by its kind.
fn check(context: &Context, text: &str, kind: Kind, uri: &str) -> Vec<Found> {
    match kind {
        Kind::Film => service::diagnostics(text, context.tables()).into_iter().map(|told| found(text, &told)).collect(),
        Kind::Game => {
            let source = language::Source::new(text);
            let parsed = language::parse_with(text, &game_holds(text)).diagnostics;
            parsed.into_iter().map(|wrong| found(text, &wrong.resolved(&source))).collect()
        }
        Kind::Decor => {
            let name = text::path_of(uri).and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned())).unwrap_or_default();
            match decor::lang::parse(text, &name) {
                Err(fault) => {
                    let line = fault.line().max(1);
                    vec![Found { diagnostic: diagnostic(text::line_range(text, line), &fault.coded("decor")), fixes: Vec::new() }]
                }
                _ => Vec::new(),
            }
        }
    }
}

/// What may be written there: what the service offers, and on a `scene:`
/// line the places and the Pokémon, in a said line's parentheses the faces.
fn completion(context: &Context, text: &str, kind: Kind, _: &str, params: &Value) -> Value {
    if kind != Kind::Film {
        return json!([]);
    }
    let at = text::spot(text, &params["position"]);
    let mut offered: Vec<(String, String)> = service::completions(text, at, &context.library);
    let before: String = text::line(text, at.line - 1).chars().take(at.column - 1).collect();
    let typed: String = before.chars().rev().take_while(|letter| letter.is_alphanumeric() || *letter == '_').collect::<String>().chars().rev().collect();
    let mut more: Vec<(String, &str)> = Vec::new();
    if let Some(rest) = before.trim_start().strip_prefix("scene:") {
        let written = rest.split_whitespace().count() - usize::from(!typed.is_empty());
        let tables = context.tables();
        if written == 0 {
            more.extend(tables.place_names_known().map(|place| (place.to_string(), "place")));
        } else {
            more.extend(tables.monsters.keys().map(|monster| (monster.clone(), "Pokémon")));
        }
    } else if let Some(open) = before.rfind('(')
        && !before[open..].contains(')')
        && before[..open].split_whitespace().count() == 1
    {
        more.extend(film::words::FACES.iter().map(|(face, _)| (face.to_string(), "face")));
    }
    offered.extend(more.into_iter().filter(|(word, _)| word.starts_with(&typed)).map(|(word, what)| (word, what.to_string())));
    let mut seen = std::collections::HashSet::new();
    offered.retain(|(word, _)| seen.insert(word.clone()));
    let items: Vec<Value> = offered.into_iter().map(|(label, detail)| json!({ "label": label, "detail": detail })).collect();
    json!({ "isIncomplete": false, "items": items })
}

/// What the word there is: its forms and what it does.
fn hover(context: &Context, text: &str, kind: Kind, _: &str, params: &Value) -> Value {
    if kind != Kind::Film {
        return Value::Null;
    }
    let at = text::spot(text, &params["position"]);
    match service::hover(text, at, &context.library) {
        Some(told) => json!({ "contents": { "kind": "plaintext", "value": told } }),
        None => Value::Null,
    }
}

/// The file a film uses or takes in, or of a shared recipe, by its name.
fn shared(context: &Context, name: &str) -> Option<std::path::PathBuf> {
    film::shared_path(&context.library, name).or_else(|| Some(context.root().join("film/recipes").join(format!("{name}.film"))).filter(|path| path.is_file()))
}

/// Where the name there is given: a recipe (`define`), a value, a mark…,
/// in this film or a file it uses; on a `uses` or `include` line, that file.
fn definition(context: &Context, text: &str, kind: Kind, uri: &str, params: &Value) -> Value {
    if kind != Kind::Film {
        return Value::Null;
    }
    let at = text::spot(text, &params["position"]);
    let location = |uri: String, line: usize| {
        let start = json!({ "line": line.saturating_sub(1), "character": 0 });
        json!({ "uri": uri, "range": { "start": start, "end": start } })
    };
    if let ["uses" | "include", name] = text::line(text, at.line - 1).split_whitespace().collect::<Vec<_>>().as_slice() {
        return shared(context, name).map_or(Value::Null, |path| location(text::uri_of(&path), 1));
    }
    match service::definition(text, at, &context.library) {
        Some((None, line)) => location(uri.to_string(), line),
        Some((Some(from), line)) => shared(context, &from).map_or(Value::Null, |path| location(text::uri_of(&path), line)),
        None => Value::Null,
    }
}

/// The film written back in order, as one edit.
fn formatting(_: &Context, text: &str, kind: Kind, _: &str, _: &Value) -> Value {
    if kind != Kind::Film {
        return json!([]);
    }
    match film::formatted(text) {
        written if written != text => json!([{ "range": { "start": { "line": 0, "character": 0 }, "end": text::end(text) }, "newText": written }]),
        _ => json!([]),
    }
}

/// The film's plan: its scenes, moments, recipes, values…
fn symbols(_: &Context, text: &str, kind: Kind, _: &str, _: &Value) -> Value {
    if kind != Kind::Film {
        return json!([]);
    }
    let plan = service::outline(text);
    let symbols: Vec<Value> = plan
        .into_iter()
        .map(|symbol| {
            let kind = match symbol.kind {
                "scene" => 2,
                "moment" => 6,
                "recipe" => 12,
                "value" | "kept" => 13,
                _ => 14,
            };
            let start = Spot { line: symbol.line, column: 1 };
            let line_end = Spot { line: symbol.line, column: text::line(text, symbol.line - 1).chars().count() + 1 };
            let last = symbol.end_line.max(symbol.line);
            let end = Spot { line: last, column: text::line(text, last - 1).chars().count() + 1 };
            let name = if symbol.name.is_empty() { symbol.kind.to_string() } else { symbol.name };
            json!({ "name": name, "detail": symbol.detail, "kind": kind, "range": text::range(text, start, end), "selectionRange": text::range(text, start, line_end) })
        })
        .collect();
    Value::Array(symbols)
}

fn folds(_: &Context, text: &str, kind: Kind, _: &str, _: &Value) -> Value {
    if kind != Kind::Film {
        return json!([]);
    }
    let folds = service::folds(text);
    Value::Array(folds.into_iter().map(|(first, last)| json!({ "startLine": first.saturating_sub(1), "endLine": last.saturating_sub(1) })).collect())
}
