//! A long piece of work (fetching from SpriteCollab, writing or reading
//! SpriteBot sheets) run on a thread of its own, so that the window goes on:
//! what it said at the end, or why it failed.

use std::sync::mpsc::{Receiver, channel};

/// A piece of work running, then done.
pub struct Run {
    /// What it is doing, as shown.
    pub doing: String,
    answer: Receiver<Result<String, String>>,
    pub done: Option<Result<String, String>>,
}

impl Run {
    /// That work started on its own thread.
    pub fn spawn(doing: &str, work: impl FnOnce() -> Result<String, String> + Send + 'static) -> Run {
        let (tell, answer) = channel();
        std::thread::spawn(move || {
            _ = tell.send(work());
        });
        Run { doing: doing.to_string(), answer, done: None }
    }

    /// Its end, once there (asked each frame).
    pub fn poll(&mut self) -> Option<&Result<String, String>> {
        if self.done.is_none()
            && let Ok(answer) = self.answer.try_recv()
        {
            self.done = Some(answer);
        }
        self.done.as_ref()
    }
}

/// Where the SpriteCollab work reads and writes: the data folder (its `extra/` layer) and the clone.
pub fn places(session: &crate::session::Session) -> spritecollab::Places {
    let folder = std::path::PathBuf::from(session.data());
    let folder = std::path::absolute(&folder).unwrap_or(folder);
    spritecollab::Places::new(&folder.display().to_string(), &session.spritecollab)
}
