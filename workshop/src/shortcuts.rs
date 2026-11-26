//! Help › Keyboard Shortcuts: every key of each space, read from where they
//! are written down, the tables of keys of `workshop/README.md` (a table whose
//! head starts with “Keys”, under the heading of its part): a key added there
//! shows here, with nothing else to change.

use eframe::egui::{self, RichText};
use workshop_base::theme;

/// The README the sheet is read from, as it was built.
const README: &str = include_str!("../README.md");

/// A part of the sheet: where (the heading, and what the table's head adds), its keys and what each does.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub name: String,
    pub keys: Vec<(String, String)>,
}

/// The parts of the sheet: each table of keys of a README, in its order (two
/// under the same name made one).
pub fn parts(readme: &str) -> Vec<Part> {
    let mut parts: Vec<Part> = Vec::new();
    let mut heading = String::new();
    let mut table: Option<usize> = None;
    for line in readme.lines() {
        let line = line.trim();
        if let Some(title) = line.strip_prefix('#') {
            heading = title.trim_start_matches('#').trim().replace('`', "");
            table = None;
            continue;
        }
        if !line.starts_with('|') {
            table = None;
            continue;
        }
        let cells: Vec<String> = cells_of(line);
        if cells.iter().all(|cell| cell.chars().all(|letter| letter == '-' || letter == ':')) {
            continue;
        }
        match table {
            Some(at) => {
                if let [keys, does, ..] = cells.as_slice() {
                    parts[at].keys.push((keys.clone(), does.clone()));
                }
            }
            None if cells.first().is_some_and(|head| head.starts_with("Keys")) => {
                let more = cells[0].trim_start_matches("Keys").trim().trim_start_matches('(').trim_end_matches(')').trim().to_string();
                let name = if more.is_empty() { heading.clone() } else { format!("{heading} ({more})") };
                table = Some(match parts.iter().position(|part| part.name == name) {
                    Some(at) => at,
                    None => {
                        parts.push(Part { name, keys: Vec::new() });
                        parts.len() - 1
                    }
                });
            }
            None => {}
        }
    }
    parts.retain(|part| !part.keys.is_empty());
    parts
}

/// A table's row cut in its cells (`\|` kept as a bar), without their backticks.
fn cells_of(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.replace("\\|", "\u{1}").split('|').map(|cell| cell.replace('\u{1}', "|").replace('`', "").trim().to_string()).collect()
}

/// The sheet's parts, read once.
pub fn sheet() -> &'static [Part] {
    static PARTS: std::sync::OnceLock<Vec<Part>> = std::sync::OnceLock::new();
    PARTS.get_or_init(|| parts(README))
}

/// The sheet in its window, while open (`open` false once closed); a search over the keys and what they do.
pub fn window(context: &egui::Context, open: &mut bool, sought: &mut String) {
    if !*open {
        return;
    }
    egui::Window::new(RichText::new("Keyboard Shortcuts").strong())
        .open(open)
        .collapsible(false)
        .default_size([760.0, 620.0])
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(context, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(sought).hint_text("search a key or what it does").desired_width(300.0));
                ui.label(theme::weak("from workshop/README.md · every command, with its key: Ctrl+Shift+P"));
            });
            ui.add_space(4.0);
            let words: Vec<String> = sought.to_lowercase().split_whitespace().map(str::to_string).collect();
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                for part in sheet() {
                    let rows: Vec<&(String, String)> = part
                        .keys
                        .iter()
                        .filter(|(keys, does)| {
                            let both = format!("{keys} {does} {}", part.name).to_lowercase();
                            words.iter().all(|word| both.contains(word))
                        })
                        .collect();
                    if rows.is_empty() {
                        continue;
                    }
                    ui.add_space(6.0);
                    ui.label(theme::text(part.name.to_uppercase(), theme::SMALL, theme::FILM));
                    egui::Grid::new(("shortcuts", &part.name)).num_columns(2).spacing([16.0, 4.0]).striped(true).show(ui, |ui| {
                        for (keys, does) in rows {
                            ui.label(RichText::new(keys).monospace().color(theme::STRONG));
                            ui.add(egui::Label::new(theme::text(does, theme::SMALL, theme::TEXT)).wrap());
                            ui.end_row();
                        }
                    });
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sheet is the README's tables of keys: each under its heading, what a table's head adds named too.
    #[test]
    fn the_sheet_is_the_readmes_tables_of_keys() {
        let readme = "# W\n## Film › Script\n| Keys | |\n|---|---|\n| Ctrl+S | write now |\n| `a \\| b` | either |\n\nprose\n## Decor\n| Keys (pointer over the Scene) | |\n|---|---|\n| H | flip |\n\n| Variable | Effect |\n|---|---|\n| X | not keys |\n";
        let parts = parts(readme);
        assert_eq!(
            parts,
            vec![
                Part { name: "Film › Script".into(), keys: vec![("Ctrl+S".into(), "write now".into()), ("a | b".into(), "either".into())] },
                Part { name: "Decor (pointer over the Scene)".into(), keys: vec![("H".into(), "flip".into())] },
            ]
        );
        let own = sheet();
        assert!(own.iter().any(|part| part.keys.iter().any(|(keys, _)| keys.contains("Ctrl+Shift+P"))), "the palette's key is in the README");
        assert!(own.iter().any(|part| part.name.starts_with("Film › Script") && part.keys.iter().any(|(keys, _)| keys == "Ctrl+Space")));
        assert!(own.iter().any(|part| part.name.starts_with("Decor")));
    }
}
