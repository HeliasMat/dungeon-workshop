//! `scene_player spritecollab all|update`: the PMDCollab community's
//! sprites and faces, all of them, fetched and imported into `data/extra/`
//! (`spritecollab::fetch_all`, `update`, `import_all`).

use std::sync::Arc;

use ::film::Context;

/// The clone: `DUNGEON_SPRITECOLLAB`, else `<root>/sources/spritecollab` (as the Workshop's).
fn places(context: &Context) -> spritecollab::Places {
    let clone = std::env::var_os("DUNGEON_SPRITECOLLAB").map_or_else(|| context.root().join("sources").join("spritecollab"), std::path::PathBuf::from);
    spritecollab::Places::new(context.data.as_str(), &clone)
}

pub fn spritecollab(context: &Arc<Context>, what: &str, replace: bool) -> Result<(), String> {
    let places = places(context);
    let mut tell = |line: &str| println!("{line}");
    match what {
        "all" => spritecollab::fetch_all(&places, &mut tell)?,
        "update" => _ = spritecollab::update(&places, &mut tell)?,
        other => return Err(format!("spritecollab: all or update, not “{other}”. scene_player help spritecollab")),
    }
    let over = if replace { spritecollab::Over::All } else { spritecollab::Over::Ours };
    let totals = spritecollab::import_all(&places, over, &mut tell)?;
    println!("{totals}");
    Ok(())
}
