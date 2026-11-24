//! A dungeon of the Workshop for the DS: what `tools/ds/dungeon.py` makes
//! Sky's dungeon of.

use std::path::Path;

use ::film::Context;

/// The dungeon at `path` (and the fight it ends with) as the rules read it,
/// written to `out` in JSON: each floor's foes (species, level, weights as
/// thresholds out of 10,000), its items, tileset and song, the arena laid
/// out (tiles, who stands where, the boss's life already multiplied), what
/// the boss says, the team its game names. Ok: what was written, in a line.
pub fn dungeon(context: &Context, path: &Path, out: &Path) -> Result<String, String> {
    let shown = path.display();
    let text = std::fs::read_to_string(path).map_err(|error| format!("{shown}: {error}"))?;
    // (Its game's folder: the one above `dungeons/`, for its fight's file.)
    let game_folder = path.parent().and_then(Path::parent).unwrap_or(Path::new("."));
    let project = context.library.at(game_folder);
    let book = game::playthrough::book(&context.data).map_err(|error| error.to_string())?;
    let own = rules::own::read(&game::dive::resolved_for(&project, &text), &book.tables)
        .map_err(|wrong| format!("{shown}: {}", wrong.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")))?;
    let boss = own.boss.as_ref().map(|says| {
        let phases: Vec<serde_json::Value> = says
            .phases
            .iter()
            .map(|phase| {
                serde_json::json!({
                    "share": phase.share, "says": phase.says, "fiercer": phase.fiercer, "heal": phase.heal,
                    "weather": phase.weather, "help": phase.help.iter().map(|enemy| enemy.species).collect::<Vec<_>>(),
                    "moves": phase.moves, "shake": phase.shake,
                })
            })
            .collect();
        serde_json::json!({
            "species": says.species, "name": says.name, "title": says.title, "begins": says.begins, "falls": says.falls,
            "moves": says.moves, "joins": says.joins, "phases": phases,
        })
    });
    // (The team its game says, `team: chikorita totodile`, by the species' numbers.)
    let games = std::fs::read_dir(game_folder).into_iter().flatten().flatten().map(|entry| entry.path());
    let said = games.filter(|path| path.extension().is_some_and(|extension| extension == "game")).find_map(|path| {
        let text = std::fs::read_to_string(path).ok()?;
        text.lines().find_map(|line| line.trim().strip_prefix("team:").map(str::to_string))
    });
    let species_of = |word: &str| book.tables.species.iter().position(|kind| rules::own::plain(&kind.name) == rules::own::plain(word));
    let team: Vec<usize> = said.iter().flat_map(|said| said.split_whitespace()).filter_map(species_of).collect();
    let dump = serde_json::json!({ "name": own.name, "team_level": own.team_level, "team": team, "dungeon": own.table, "boss": boss });
    if let Some(folder) = out.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    let written = serde_json::to_string_pretty(&dump).map_err(|error| error.to_string())?;
    std::fs::write(out, written).map_err(|error| format!("{}: {error}", out.display()))?;
    let fight = if own.boss.is_some() { ", the last a boss fight" } else { "" };
    Ok(format!("{}: {} ({} floors{fight})", out.display(), own.name, own.table.floors.len()))
}
