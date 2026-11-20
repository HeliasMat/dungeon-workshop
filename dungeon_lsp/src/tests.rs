use serde_json::{Value, json};
use std::io::Cursor;

use crate::framing::{read_message, write_message};
use crate::server::Server;

const URI: &str = "file:///project/library/films/test.film";

fn opened(text: &str) -> (Server, Vec<Value>) {
    let mut server = Server::default();
    let open = json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": { "textDocument": { "uri": URI, "languageId": "film", "version": 1, "text": text } } });
    let reply = server.handle(&open);
    (server, reply.out)
}

fn asked(server: &mut Server, method: &str, params: Value) -> Value {
    let reply = server.handle(&json!({ "jsonrpc": "2.0", "id": 7, "method": method, "params": params }));
    reply.out[0]["result"].clone()
}

fn at(line: u32, character: u32) -> Value {
    json!({ "textDocument": { "uri": URI }, "position": { "line": line, "character": character } })
}

#[test]
fn framing_round_trip() {
    let mut written = Vec::new();
    write_message(&mut written, r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#).unwrap();
    write_message(&mut written, r#"{"said":"é ✓"}"#).unwrap();
    let mut read = Cursor::new(written);
    assert_eq!(read_message(&mut read).unwrap().as_deref(), Some(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#));
    assert_eq!(read_message(&mut read).unwrap().as_deref(), Some(r#"{"said":"é ✓"}"#));
    assert_eq!(read_message(&mut read).unwrap(), None);
    // (Another header, any case.)
    let mut other = Cursor::new(b"content-length: 2\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{}".to_vec());
    assert_eq!(read_message(&mut other).unwrap().as_deref(), Some("{}"));
}

#[test]
fn a_session_starts_and_ends() {
    let mut input = Vec::new();
    for message in [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "capabilities": {} } }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "shutdown" }),
        json!({ "jsonrpc": "2.0", "method": "exit" }),
    ] {
        write_message(&mut input, &message.to_string()).unwrap();
    }
    let mut output = Vec::new();
    assert_eq!(crate::run(&mut Cursor::new(input), &mut output).unwrap(), 0);
    let mut read = Cursor::new(output);
    let first: Value = serde_json::from_str(&read_message(&mut read).unwrap().unwrap()).unwrap();
    assert_eq!(first["result"]["capabilities"]["textDocumentSync"], 1);
}

#[test]
fn a_broken_film_is_told_where_it_is_wrong() {
    let (mut server, out) = opened("scene: beach pikachu\nif n >= \nend\njum pikachu\n");
    let diagnostics = out[0]["params"]["diagnostics"].as_array().unwrap().clone();
    let calculation = diagnostics.iter().find(|one| one["code"] == "E0005").expect("the calculation");
    assert_eq!(calculation["range"]["start"], json!({ "line": 1, "character": 7 }));
    assert_eq!(calculation["severity"], 1);
    let near = diagnostics.iter().find(|one| one["range"]["start"]["line"] == 3).expect("the word not known");
    // (Its fix, as a code action.)
    let actions =
        asked(&mut server, "textDocument/codeAction", json!({ "textDocument": { "uri": URI }, "range": near["range"], "context": { "diagnostics": [] } }));
    let fixes: Vec<&Value> = actions.as_array().unwrap().iter().flat_map(|action| action["edit"]["changes"][URI].as_array().unwrap()).collect();
    assert!(fixes.iter().any(|edit| edit["newText"] == "jump"), "{actions}");
}

#[test]
fn completion_offers_a_known_word() {
    let (mut server, _) = opened("scene: beach pikachu\nwal");
    let offered = asked(&mut server, "textDocument/completion", at(1, 3));
    let labels: Vec<&str> = offered["items"].as_array().unwrap().iter().filter_map(|item| item["label"].as_str()).collect();
    assert!(labels.contains(&"walk"), "{labels:?}");
}

#[test]
fn hover_on_walk_gives_its_form() {
    let (mut server, out) = opened("scene: beach pikachu\npikachu walk to pikachuu\n");
    let told = asked(&mut server, "textDocument/hover", at(1, 10));
    let value = told["contents"]["value"].as_str().expect("help on walk");
    assert!(value.starts_with("who:pokemon walk ") && value.contains(" — "), "{value}");
    // (What is wrong, on its own words of the line written who first.)
    let diagnostics = out[0]["params"]["diagnostics"].as_array().unwrap().clone();
    // (Where the game's places are known: without data/, no place is wrong.)
    let Some(place) = diagnostics.iter().find(|one| one["message"].as_str().is_some_and(|said| said.contains("pikachuu"))) else {
        return;
    };
    assert_eq!(place["range"], json!({ "start": { "line": 1, "character": 16 }, "end": { "line": 1, "character": 24 } }), "{place}");
}

#[test]
fn a_game_and_a_decor_are_read_by_their_own() {
    let mut server = Server::default();
    let game = json!({ "method": "textDocument/didOpen", "params": { "textDocument": { "uri": "file:///g/x.game", "text": "title: X\nchapter 1 \"Start\"\n  if\nend\n" } } });
    let told = &server.handle(&game).out[0]["params"]["diagnostics"];
    assert!(!told.as_array().unwrap().is_empty(), "{told}");
    let decor = json!({ "method": "textDocument/didOpen", "params": { "textDocument": { "uri": "file:///g/x.decor", "text": "nothing known here\n" } } });
    let told = &server.handle(&decor).out[0]["params"]["diagnostics"];
    assert_eq!(told[0]["range"]["start"]["line"], 0, "{told}");
}

#[test]
fn places_and_addresses() {
    let film = "pikachu: é✓😀 ok\n";
    let spot = crate::text::spot(film, &json!({ "line": 0, "character": 14 }));
    assert_eq!(spot, film::service::Spot { line: 1, column: 14 });
    assert_eq!(crate::text::position(film, spot), json!({ "line": 0, "character": 14 }));
    let path = crate::text::path_of("file:///c%3A/Users/my%20films/a.film").unwrap();
    assert_eq!(path, std::path::PathBuf::from("c:/Users/my films/a.film"));
    assert_eq!(crate::text::uri_of(&path), "file:///c:/Users/my%20films/a.film");
}

#[test]
fn a_game_opened_is_checked_whole_on_its_files_lines() {
    let folder = std::env::temp_dir().join(format!("dw_lsp_game_{}", std::process::id()));
    _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("decors")).unwrap();
    std::fs::create_dir_all(folder.join("films")).unwrap();
    let town = "decor: town 256 192\nentrance 10 96 west\n[life]\nresident kecleon 120 80 down\nsays if not met: Welcome!\n";
    let game = "title: T\nteam: pikachu piplup\n\nchapter 1\n  go town west\n  play the_arival\n  finish\nend\n";
    std::fs::write(folder.join("decors/town.decor"), town).unwrap();
    std::fs::write(folder.join("t.game"), game).unwrap();
    let (game_uri, town_uri) = (crate::text::uri_of(&folder.join("t.game")), crate::text::uri_of(&folder.join("decors/town.decor")));
    let mut server = Server::default();
    let open = |uri: &str, text: &str| json!({ "method": "textDocument/didOpen", "params": { "textDocument": { "uri": uri, "text": text } } });
    server.handle(&open(&town_uri, town));
    let out = server.handle(&open(&game_uri, game)).out;
    let told = |uri: &str| out.iter().rev().find(|one| one["params"]["uri"] == uri).map(|one| one["params"]["diagnostics"].clone()).unwrap_or_default();
    let on_game = told(&game_uri);
    assert!(
        on_game.as_array().unwrap().iter().any(|one| one["range"]["start"]["line"] == 5 && one["message"].as_str().unwrap().contains("the_arival")),
        "{on_game}"
    );
    let on_town = told(&town_uri);
    assert!(
        on_town.as_array().unwrap().iter().any(|one| one["range"]["start"]["line"] == 4 && one["message"].as_str().unwrap().contains("“met”")),
        "{on_town}"
    );
    _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn lines_that_made_the_services_fall_over_are_answered() {
    // (Each panicked in the compiler, or asked for all the memory there is;
    // the server's net caught it: now they are diagnostics.)
    let text = "scene: beach pikachu piplup\nregroup pikachu\nmove pikachu\ncrowd pikachu 99999999999999\ntwinkle pikachu 1e40 nan\n";
    let (mut server, out) = opened(text);
    let diagnostics = out[0]["params"]["diagnostics"].as_array().unwrap().clone();
    assert!(diagnostics.iter().any(|one| one["message"].as_str().is_some_and(|said| said.contains("at most"))), "{diagnostics:?}");
    for line in 1..5 {
        asked(&mut server, "textDocument/hover", at(line, 3));
        asked(&mut server, "textDocument/completion", at(line, 12));
    }
}
