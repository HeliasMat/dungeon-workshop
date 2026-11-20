//! The engine's lines as kept in a file (`story.txt`: fields between tabs).

use super::Lines;
use crate::vocabulary::Vocabulary;
use diagnostics::Diagnostic;

/// The lines of a text of fields between tabs, each command by its own
/// name; pictures found in its folder.
pub fn read_fields(text: &str, folder: Option<&std::path::Path>, problems: &mut Vec<Diagnostic>, _vocabulary: &Vocabulary) -> Lines {
    let mut lines = Vec::new();
    for (at, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields: Vec<String> = line.split('\t').map(str::to_string).collect();
        // (A test may be written with spaces too: `if a == 1`.)
        if matches!(fields[0].as_str(), "if" | "elif" | "while" | "waituntil" | "when") {
            fields = fields.iter().flat_map(|field| field.split_whitespace()).map(str::to_string).collect();
        }
        let number = at + 1;
        // (A picture's file, from the scene's folder: `image <name> <file>
        // ...`, `topimage <file>`; one not found told.)
        let file_at = match fields[0].as_str() {
            "image" => Some(2),
            "topimage" => Some(1),
            _ => None,
        };
        if let Some(place) = file_at
            && let (Some(folder), Some(file)) = (folder, fields.get(place).filter(|file| file.as_str() != "off"))
        {
            let path = folder.join(file);
            if !path.exists() {
                problems.push(Diagnostic::at(number, format!("picture not found: {file}")));
            }
            fields[place] = path.to_string_lossy().into_owned();
        }
        // (The engine opens no file: a scene takes in another in the language, `include name`.)
        if fields[0] == "include" {
            problems.push(Diagnostic::at(number, "“include”: a file of fields takes in no other (a film does, by “include name”)".to_string()));
            continue;
        }
        lines.push((number, fields));
    }
    lines
}
