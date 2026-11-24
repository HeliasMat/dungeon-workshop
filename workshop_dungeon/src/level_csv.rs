//! "Export as CSV": a species' progression, level by level, as the game's
//! tables give it (`dungeons.bin`): the experience to be there, its stats
//! (its base at level 1, then what each level adds, as
//! `rules::dungeon::exp::Growth` counts them) and the moves it learns.
//! Separated by `,`, with a UTF-8 mark first so that a spreadsheet reads its accents.

use std::fmt::Write as _;
use std::path::Path;

/// The game's limits on stats (`rules::dungeon::exp`).
const MAX_HP: i32 = 999;
const MAX_STAT: i32 = 255;

/// The species a Pokémon of the project is: its own number, or (a variant) the one it is a variant of.
pub fn species_of(session: &workshop_base::session::Session, number: i64) -> Option<u16> {
    let offered = crate::offered(session)?;
    let book = &offered.book;
    let number = u16::try_from(number).ok()?;
    if book.species(number).is_some_and(|species| !species.growth.is_empty()) {
        return Some(number);
    }
    assets::tables::variants(session.data()).into_iter().find(|variant| variant.number == number).map(|variant| variant.of)
}

/// A field as CSV has it: quoted if it holds a comma, a quote or a line break.
fn quoted(said: &str) -> String {
    if said.contains([',', '"', '\n']) { format!("\"{}\"", said.replace('"', "\"\"")) } else { said.to_string() }
}

/// The table, as text.
pub fn table(session: &workshop_base::session::Session, species: u16) -> Result<String, String> {
    let offered = crate::offered(session).ok_or("the game's tables are missing (dungeons.bin: extractor)")?;
    let book = &offered.book;
    let kind = book.species(species).ok_or_else(|| format!("no species No. {species} in the game's tables"))?;
    if kind.growth.is_empty() {
        return Err(format!("{}: no progression in the game's tables", kind.name));
    }
    let move_name = |id: u16| book.tables.moves.get(id as usize).cloned().filter(|name| !name.is_empty()).unwrap_or_else(|| format!("move {id}"));
    let mut text = String::from("\u{FEFF}");
    _ = writeln!(text, "{}", quoted(&format!("# {} (No. {species}); stats at level 1: the species', then what each level adds", kind.name)));
    text.push_str("Level,Total Exp.,HP,Attack,Defense,Sp. Atk,Sp. Def,Moves Learned\n");
    // (Its base: HP, Attack, Defense, Sp. Atk, Sp. Def, as the extraction keeps them.)
    let [mut hp, mut attack, mut defense, mut sp_attack, mut sp_defense] = kind.base;
    for (at, gain) in kind.growth.iter().enumerate().take(100) {
        let level = at + 1;
        if level > 1 {
            hp = (hp + gain.hp).min(MAX_HP);
            attack = (attack + gain.attack).min(MAX_STAT);
            defense = (defense + gain.defense).min(MAX_STAT);
            sp_attack = (sp_attack + gain.sp_attack).min(MAX_STAT);
            sp_defense = (sp_defense + gain.sp_defense).min(MAX_STAT);
        }
        let learned: Vec<String> = kind.learnset.iter().filter(|(when, _)| usize::from(*when) == level).map(|(_, slot)| move_name(slot.id)).collect();
        _ = writeln!(text, "{level},{},{hp},{attack},{defense},{sp_attack},{sp_defense},{}", gain.total_exp, quoted(&learned.join(" / ")));
    }
    // (The moves the machines teach it, below.)
    if !kind.machines.is_empty() {
        let taught: Vec<String> = kind.machines.iter().map(|id| move_name(*id)).collect();
        _ = writeln!(text, "\nTM and HM Moves,{}", quoted(&taught.join(" / ")));
    }
    Ok(text)
}

/// Written where asked (a dialog of the system's): what is said of it.
pub fn export(session: &workshop_base::session::Session, number: i64, label: &str) -> String {
    let Some(species) = species_of(session, number) else {
        return format!("{label}: not in the game's tables (no progression known)");
    };
    let text = match table(session, species) {
        Ok(text) => text,
        Err(why) => return why,
    };
    let name = format!("{}_progression.csv", film::words::plain(label).replace([' ', '\''], "_"));
    let Some(path) = workshop_base::pick::save("Export the Progression as CSV", &name, &[("CSV", &["csv"])], Some(&session.root().join("target"))) else {
        return "export cancelled".to_string();
    };
    match write(&path, &text) {
        Ok(()) => format!("progression written: {}", path.display()),
        Err(why) => why,
    }
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|why| format!("{}: {why}", path.display()))
}
