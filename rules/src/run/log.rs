//! The messages of the dungeon.

/// Messages kept on screen.
pub(crate) const LOG_LINES: usize = 5;
/// Messages kept for the message log.
const LOG_HISTORY: usize = 200;

/// The last messages, oldest first, and whether each opens a new group:
/// the game draws a line between the messages of two actions.
#[derive(Default, Clone)]
pub struct Log {
    pub lines: Vec<String>,
    pub opens: Vec<bool>,
}

impl Log {
    /// The last messages, those shown.
    pub fn recent(&self) -> &[String] {
        &self.lines[self.lines.len().saturating_sub(LOG_LINES)..]
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.opens.clear();
    }

    pub fn push(&mut self, message: String) {
        // (An action is told by how its first message reads.)
        let opens = [" used ", " attacks!", " threw ", " picked up ", " gets ready"].iter().any(|verb| message.contains(verb));
        self.lines.push(message);
        self.opens.push(opens);
        if self.lines.len() > LOG_HISTORY {
            self.lines.remove(0);
            self.opens.remove(0);
        }
    }
}
