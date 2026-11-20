//! Places in a text, as the service counts them (lines and columns from 1,
//! a column counting letters) and as the protocol does (from 0, a column
//! counting UTF-16 units); and the files' addresses (`file:///…`).

use film::service::Spot;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// The line of the text, from 0, without its end.
pub fn line(text: &str, number: usize) -> &str {
    text.split('\n').nth(number).unwrap_or("").trim_end_matches('\r')
}

/// The protocol's place of a service's spot.
pub fn position(text: &str, spot: Spot) -> Value {
    let number = spot.line.saturating_sub(1);
    let units: usize = line(text, number).chars().take(spot.column.saturating_sub(1)).map(char::len_utf16).sum();
    json!({ "line": number, "character": units })
}

/// The protocol's range from one spot to another.
pub fn range(text: &str, start: Spot, end: Spot) -> Value {
    json!({ "start": position(text, start), "end": position(text, end) })
}

/// A whole line of the text (from 1), as a range.
pub fn line_range(text: &str, number: usize) -> Value {
    let long = line(text, number.saturating_sub(1)).chars().count();
    range(text, Spot { line: number, column: 1 }, Spot { line: number, column: long + 1 })
}

/// The service's spot of the protocol's place (`{line, character}`).
pub fn spot(text: &str, at: &Value) -> Spot {
    let number = at["line"].as_u64().unwrap_or(0) as usize;
    let units = at["character"].as_u64().unwrap_or(0) as usize;
    let (mut counted, mut column) = (0, 1);
    for letter in line(text, number).chars() {
        if counted >= units {
            break;
        }
        counted += letter.len_utf16();
        column += 1;
    }
    Spot { line: number + 1, column }
}

/// The end of the text, as the protocol's place.
pub fn end(text: &str) -> Value {
    let last = text.split('\n').count() - 1;
    json!({ "line": last, "character": line(text, last).encode_utf16().count() })
}

/// The file of a `file://` address.
pub fn path_of(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%'
            && let Some(byte) = rest.get(at + 1..at + 3).and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            decoded.push(byte);
            at += 3;
            continue;
        }
        decoded.push(bytes[at]);
        at += 1;
    }
    let mut path = String::from_utf8(decoded).ok()?;
    // (`/C:/Users/…` on Windows: the drive first.)
    if path.as_bytes().get(2) == Some(&b':') && path.starts_with('/') {
        path.remove(0);
    }
    Some(PathBuf::from(path))
}

/// The `file://` address of a file.
pub fn uri_of(path: &Path) -> String {
    let written = path.to_string_lossy().replace('\\', "/");
    let mut uri = String::from("file://");
    if !written.starts_with('/') {
        uri.push('/');
    }
    for byte in written.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => uri.push(byte as char),
            other => uri.push_str(&format!("%{other:02X}")),
        }
    }
    uri
}
