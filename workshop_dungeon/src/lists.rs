//! The dungeons and the boss fights written, and what each says of itself.

use rules::own::plain;

use super::{Form, offered};

/// The dungeons written in that project's own folder (not the shared library's, if it is another), by their words.
pub fn dungeons(library: &assets::project::Project) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(library.dungeons())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|end| end == "dungeon"))
        .filter_map(|entry| Some(entry.path().file_stem()?.to_string_lossy().into_owned()))
        .collect();
    found.sort();
    found
}

/// A dungeon written, as the Resources' library shows it: its word, its
/// name, its file, how many floors, its look, the Pokémon met there (and the
/// species of the first), its boss if it has one.
pub struct Summary {
    pub word: String,
    /// The open game's own (in its folder), not the shared library's.
    pub own: bool,
    pub name: String,
    pub path: std::path::PathBuf,
    pub floors: usize,
    pub look: String,
    pub foes: Vec<String>,
    pub first: Option<u16>,
    pub boss: Option<BossSummary>,
}

/// A boss as the library shows it: its species (and its number), its level,
/// its title, how many guards, how many phases, how much more life.
pub struct BossSummary {
    pub species: String,
    pub id: Option<u16>,
    pub level: i32,
    pub title: String,
    pub guards: usize,
    pub phases: usize,
    pub life: f32,
}

/// Every dungeon written in the shared library, summed up.
pub fn summaries(session: &workshop_base::session::Session) -> Vec<Summary> {
    summaries_in(session, session.library())
}

/// Every dungeon written in that project's folder (a game's, or the shared library), summed up; a fight it names, its own or the library's.
pub fn summaries_in(session: &workshop_base::session::Session, library: &assets::project::Project) -> Vec<Summary> {
    let own = !library.is_library();
    let kept = offered(session);
    let offered = kept.as_deref();
    let id_of =
        |name: &str| offered.and_then(|offered| offered.species.iter().chain(&offered.able).find(|(_, known)| plain(known) == plain(name))).map(|(id, _)| *id);
    dungeons(library)
        .into_iter()
        .map(|word| {
            let path = library.dungeons().join(format!("{word}.dungeon"));
            let mut form = Form::read(&std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n"));
            // (Its boss: written in it, or in the fight it names.)
            if form.boss.is_none() && !form.fight.is_empty() {
                let fight = std::fs::read_to_string(library.fight(&form.fight)).unwrap_or_default();
                form.boss = Form::read(&fight.replace("\r\n", "\n")).boss;
            }
            let boss = form.boss.as_ref().map(|boss| BossSummary {
                species: boss.species.clone(),
                id: id_of(&boss.species),
                level: boss.level,
                title: boss.title.clone(),
                guards: boss.guards.len(),
                phases: boss.phases.len(),
                life: boss.life,
            });
            let foes: Vec<String> = form.foes.iter().map(|foe| foe.species.clone()).collect();
            let first = foes.first().and_then(|name| id_of(name));
            Summary { word, own, name: form.name, path, floors: form.floors, look: form.look, foes, first, boss }
        })
        .collect()
}

/// Every boss fight written apart (`library/fights`), summed up as the library shows it.
pub fn fight_summaries(session: &workshop_base::session::Session) -> Vec<Summary> {
    fight_summaries_in(session, session.library())
}

/// Every boss fight written apart in that project's folder (a game's `fights/`, or the shared library's), summed up.
pub fn fight_summaries_in(session: &workshop_base::session::Session, library: &assets::project::Project) -> Vec<Summary> {
    let own = !library.is_library();
    let kept = offered(session);
    let offered = kept.as_deref();
    let id_of =
        |name: &str| offered.and_then(|offered| offered.species.iter().chain(&offered.able).find(|(_, known)| plain(known) == plain(name))).map(|(id, _)| *id);
    fights(library)
        .into_iter()
        .map(|word| {
            let path = library.fights().join(format!("{word}.fight"));
            let form = Form::read(&std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n"));
            let boss = form.boss.as_ref().map(|boss| BossSummary {
                species: boss.species.clone(),
                id: id_of(&boss.species),
                level: boss.level,
                title: boss.title.clone(),
                guards: boss.guards.len(),
                phases: boss.phases.len(),
                life: boss.life,
            });
            let look = form.boss.as_ref().map_or_else(String::new, |boss| boss.look.clone());
            Summary { word, own, name: form.name, path, floors: 0, look, foes: Vec::new(), first: None, boss }
        })
        .collect()
}

/// The boss fights written in that project's own folder, by their words.
pub fn fights(library: &assets::project::Project) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(library.fights())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|end| end == "fight"))
        .filter_map(|entry| Some(entry.path().file_stem()?.to_string_lossy().into_owned()))
        .collect();
    found.sort();
    found
}
