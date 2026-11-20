//! A language server for the films (`.film`), games (`.game`) and decors
//! (`.decor`): the help the Workshop gives, in any editor that speaks the
//! Language Server Protocol, over stdin and stdout.
//!
//! - `framing`: the protocol's messages, read and written;
//! - `text`: places in a text, the service's and the protocol's;
//! - `server`: what each message is answered.

#![forbid(unsafe_code)]

pub mod framing;
pub mod server;
pub mod text;

#[cfg(test)]
mod tests;

use std::io::{self, BufRead, Write};

/// The server run over those streams until the editor says `exit` (its
/// code) or the input ends.
///
/// What the server is given is checked and answered with diagnostics, never
/// by a panic; should one happen all the same, it is a bug: the one safety
/// net of the program, here, tells it on stderr and answers that request
/// with an error, and the server goes on.
pub fn run(input: &mut impl BufRead, output: &mut impl Write) -> io::Result<i32> {
    let mut server = server::Server::default();
    while let Some(body) = framing::read_message(input)? {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&body) else { continue };
        let reply = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| server.handle(&message))) {
            Ok(reply) => reply,
            Err(panic) => fell_over(&message, panic.as_ref()),
        };
        for out in &reply.out {
            framing::write_message(output, &out.to_string())?;
        }
        if let Some(code) = reply.exit {
            return Ok(code);
        }
    }
    Ok(if server.shut() { 0 } else { 1 })
}

/// A message the server fell over on: the bug told on stderr, the request
/// answered with an internal error.
fn fell_over(message: &serde_json::Value, panic: &(dyn std::any::Any + Send)) -> server::Reply {
    let why = panic.downcast_ref::<String>().map(String::as_str).or_else(|| panic.downcast_ref::<&str>().copied()).unwrap_or("");
    let method = message["method"].as_str().unwrap_or("");
    eprintln!("dungeon_lsp: a bug, on {method}: {why}");
    let mut reply = server::Reply::default();
    if let Some(id) = message.get("id") {
        let error = serde_json::json!({ "code": -32603, "message": format!("dungeon_lsp fell over (a bug): {why}") });
        reply.out.push(serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": error }));
    }
    reply
}
