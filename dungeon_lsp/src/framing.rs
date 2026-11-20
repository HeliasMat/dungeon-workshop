//! The protocol's framing: a `Content-Length` header, a blank line, then
//! that many bytes of JSON.

use std::io::{self, BufRead, Write};

/// The next message's body; `None` at the end of the input.
pub fn read_message(input: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut length: Option<usize> = None;
    loop {
        let mut header = String::new();
        if input.read_line(&mut header)? == 0 {
            return Ok(None);
        }
        let header = header.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            if length.is_some() {
                break;
            }
            continue;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = Some(value.trim().parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad Content-Length"))?);
        }
    }
    let mut body = vec![0; length.unwrap_or(0)];
    input.read_exact(&mut body)?;
    String::from_utf8(body).map(Some).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "a message not in UTF-8"))
}

/// A message written with its header.
pub fn write_message(output: &mut impl Write, body: &str) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}
