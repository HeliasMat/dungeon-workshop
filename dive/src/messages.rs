//! The last messages, in the game's message window, for a moment.

use rules::run::Run;
use stage_view::ui::{BOTTOM, Canvas, SCREEN_HEIGHT, TILE, UiArt};

/// How long a message stays on screen, in frames (by eye; the original also
/// scrolls its messages in).
const SHOWN: u32 = 240;
/// Lines of the window, and pixels between them.
const LINES: usize = 3;
const PITCH: i32 = 13;

#[derive(Default)]
pub struct Messages {
    /// What the log held when last seen, and those lines with their colours.
    seen: Vec<String>,
    lines: Vec<String>,
    left: u32,
}

/// Names and moves in their colours, as the game writes them: the leader
/// in blue, its partner in yellow, wild monsters in cyan, moves in green.
fn coloured(message: &str, run: &Run) -> String {
    // Numbers first (cyan in the game), while the text has no tag.
    let mut numbered = String::new();
    let mut in_number = false;
    for c in message.chars() {
        if c.is_ascii_digit() != in_number {
            numbered += if in_number { "[CR]" } else { "[CS:E]" };
            in_number = !in_number;
        }
        numbered.push(c);
    }
    if in_number {
        numbered += "[CR]";
    }
    let mut message = numbered;
    if let Some(start) = message.find(" used ").map(|at| at + " used ".len())
        && let Some(end) = message[start..].find('!').or(message[start..].find('.'))
    {
        let used = message[start..start + end].to_string();
        message.replace_range(start..start + end, &format!("[CS:M]{used}[CR]"));
    }
    let Some(dungeon) = run.dungeon() else {
        return message;
    };
    let leader = dungeon.leader().name.as_str();
    let ally = dungeon.monsters.iter().find(|monster| monster.side == rules::run::Side::Ally).map(|ally| ally.name.as_str());
    let wild = dungeon.spec(&run.book).map_or(&[][..], |floor| &floor.enemies[..]);
    let mut done: Vec<String> = Vec::new();
    for enemy in wild {
        let name = run.book.species_name(enemy.species);
        if !name.is_empty() && !done.contains(&name) && Some(name.as_str()) != ally && name != leader {
            message = message.replace(&name, &format!("[CS:E]{name}[CR]"));
            done.push(name);
        }
    }
    message = message.replace(leader, &format!("[CS:l]{leader}[CR]"));
    if let Some(ally) = ally {
        message = message.replace(ally, &format!("[CS:p]{ally}[CR]"));
    }
    message
}

impl Messages {
    /// What the log says now; a new message shows the window again.
    pub fn advance(&mut self, run: &Run) {
        let recent = run.log.recent();
        if self.seen != recent {
            self.seen = recent.to_vec();
            self.left = SHOWN;
            let first = run.log.lines.len().saturating_sub(LINES);
            self.lines = run.log.lines[first..].iter().map(|message| coloured(message, run)).collect();
        } else {
            self.left = self.left.saturating_sub(1);
        }
    }

    pub fn draw(&self, canvas: &mut Canvas, ui: &UiArt, run: &Run) {
        if self.left == 0 || self.lines.is_empty() {
            return;
        }
        let (screen, left) = (SCREEN_HEIGHT as i32, (2 * TILE) as i32);
        canvas.window(ui, 2, BOTTOM + 17, 28, 5);
        let first = run.log.lines.len().saturating_sub(LINES);
        for (row, message) in self.lines.iter().enumerate() {
            let y = screen + 137 + row as i32 * PITCH;
            // (A line between two actions.)
            if row > 0 && run.log.opens.get(first + row) == Some(&true) {
                canvas.separator(left, y - 1, 28 * TILE);
            }
            canvas.markup(ui, left + 4, y, 28 * TILE - 8, message);
        }
    }
}
