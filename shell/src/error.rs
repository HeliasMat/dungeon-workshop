//! What goes wrong with the window, the microphone or a sound written:
//! typed, said as it always was (`Display`); `?` into a `String` for the
//! programs that speak in them.

use std::fmt;

/// What went wrong.
#[derive(Debug)]
pub enum Error {
    /// The window could not be opened or shown: what it said.
    Window(String),
    /// No microphone to listen to.
    NoMicrophone,
    /// The microphone could not be listened to: what it said.
    Microphone(String),
    /// The microphone gives its samples in a format not read.
    MicrophoneFormat(String),
    /// A sound not written: its path, why.
    Written { path: String, why: std::io::Error },
}

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Window(why) | Error::Microphone(why) => write!(out, "{why}"),
            Error::NoMicrophone => write!(out, "no microphone"),
            Error::MicrophoneFormat(format) => write!(out, "microphone format not read: {format}"),
            Error::Written { path, why } => write!(out, "{path} : {why}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Written { why, .. } => Some(why),
            _ => None,
        }
    }
}

/// As it is said, for a program that speaks in `String`s.
impl From<Error> for String {
    fn from(error: Error) -> String {
        error.to_string()
    }
}
