//! `scene_player export-ds`: a film or a dungeon exported to Explorers of
//! Sky, by `ds_export` (docs/DS.md).

use std::path::Path;

use super::Piece;
use film::Context;

/// `export-ds <film> --out <file.exps>`; `export-ds <file.dungeon> --out
/// <file.json>`; `export-ds --table`: the table. `--partial`: what doesn't
/// export is left out (and told), not an error. `--tall`: the top screen
/// shows the place above the bottom one. `--squares`: sprites past the
/// room kept as magenta squares (`ds_export::Options::squares`).
pub fn export(context: &Context, name: &str, out: Option<&String>, table: bool, partial: bool, tall: bool, squares: bool) -> Result<(), String> {
    if table || name.is_empty() {
        print!("{}", ds_export::table());
        return Ok(());
    }
    if name.ends_with(".dungeon") {
        let stem = Path::new(name).file_stem().and_then(|stem| stem.to_str()).unwrap_or("dungeon");
        let out = out.cloned().unwrap_or_else(|| format!("target/ds/{stem}.json"));
        println!("{}", ds_export::dungeon(context, Path::new(name), Path::new(&out))?);
        return Ok(());
    }
    let piece = Piece::find(context, name)?;
    let written = piece.written.as_ref().ok_or(format!("{name}: a film only"))?;
    // (Remarks and details don't stop it: only what keeps the film from playing.)
    let wrong = written.problems.iter().filter(|problem| problem.severity == engine::Severity::Error).count();
    if wrong > 0 {
        return Err(format!("{name}: {wrong} problems; scene_player check {name} first"));
    }
    let art = piece.art().ok();
    let film = ds_export::Film { name: &piece.name, text: &piece.text, written, tables: &piece.tables, art: art.as_ref() };
    let out = out.cloned().unwrap_or_else(|| format!("target/ds/{}.exps", piece.name));
    let exported = ds_export::film(&film, ds_export::Options { partial, tall, squares }, Path::new(&out), &mut |told| eprintln!("{told}"))?;
    println!("{out}: {} actors, place {}", exported.actors, exported.place.as_deref().unwrap_or("?"));
    Ok(())
}
