//! A place walked about: the scene the game makes of a decor (its `[life]`:
//! who lives there, what they say, its ways out), the team put at the
//! entrance it came by, the player at the keys. Not an engine of its own:
//! a film's text, made here and played by the engine as films are
//! (`FlowBeat::Roam`, `OnTalk`, `Door`, `OnMenu`), the words only the game
//! writes in it (`game talk kecleon`, `way … go town south`) telling the
//! game what the player does.

use std::path::Path;

use decor::lang::Decor;

/// Where the team comes in: by an entrance (none: the first), or where it stood.
#[derive(Clone, Debug, PartialEq)]
pub enum Arrival {
    Entrance(Option<String>),
    At(i32, i32),
}

/// A place read for its scene: its file's decor (`GameFile::decor`: the game's own, else the shared library's).
pub fn decor_of(path: &Path) -> Result<Decor, crate::Error> {
    let text = std::fs::read_to_string(path).map_err(|why| crate::Error::file(path, why))?;
    decor::lang::parse(&text, &path.display().to_string()).map_err(crate::Error::Read)
}

/// A place whose decor is not built yet, built now as `decor build` would
/// (`decor::make`), into `cache` (the data's `built/` layer, under what
/// `decor` writes in `extra/`): its level.
pub fn build_decor(project: &assets::project::Project, data: &str, file: &Path, cache: &Path) -> Result<i64, crate::Error> {
    let text = std::fs::read_to_string(file).map_err(|why| crate::Error::file(file, why))?.replace("\r\n", "\n");
    let decor = decor::lang::parse(&text, &file.display().to_string()).map_err(crate::Error::Read)?;
    let not_built = |why| crate::Error::NotBuilt { file: file.display().to_string(), why };
    let workshop = decor::workshop(project, data).map_err(not_built)?;
    let (_, level) = decor::make(&workshop, decor, cache).map_err(not_built)?;
    Ok(level)
}

/// The way most like that one, as a decor writes it (`downleft`).
fn facing(toward: (f32, f32)) -> &'static str {
    let turn = ((toward.0.atan2(toward.1).to_degrees() / 45.0).round() as i32).rem_euclid(8) as usize;
    // (From down, turning to the right of the screen: as `Direction::ALL`.)
    ["down", "downright", "right", "upright", "up", "upleft", "left", "downleft"][turn]
}

/// Its middle: as it says, else the middle of its size.
pub fn middle(decor: &Decor) -> (i32, i32) {
    decor.middle.map_or((decor.width as i32 / 2, decor.height as i32 / 2), |(x, y)| (x as i32, y as i32))
}

/// Where the team stands as it comes in, and the way it faces: a step
/// inside the entrance, turned to the middle.
pub fn standing(decor: &Decor, arrival: &Arrival) -> ((i32, i32), (i32, i32), &'static str) {
    let (mx, my) = middle(decor);
    let by = match arrival {
        Arrival::At(x, y) => return ((*x, *y), (*x - 20, *y), "down"),
        Arrival::Entrance(Some(name)) => decor.entrances.iter().find(|(_, _, _, known)| known == name).or(decor.entrances.first()),
        Arrival::Entrance(None) => decor.entrances.first(),
    };
    let Some((_, x, y, _)) = by else {
        return ((mx, my), (mx - 20, my), "down");
    };
    let (x, y) = (*x as f32, *y as f32);
    let toward = (mx as f32 - x, my as f32 - y);
    let long = (toward.0 * toward.0 + toward.1 * toward.1).sqrt().max(1.0);
    let step = |far: f32| ((x + toward.0 / long * far) as i32, (y + toward.1 / long * far) as i32);
    (step(28.0), step(8.0), facing(toward))
}

/// How much larger than written an exit's rectangle is, on each side.
const EXIT_MARGIN: i64 = 8;

/// What the scene of a place is made of, beyond its decor.
pub struct Making<'a> {
    /// The hero and its partner.
    pub team: &'a [String],
    pub arrival: Arrival,
    /// Who of those who live there tells the game when talked to.
    pub talking: &'a dyn Fn(&str) -> bool,
    /// Whether a line of the place holds now (`if …`).
    pub holds: &'a dyn Fn(Option<&str>) -> bool,
    /// Lines said as the scene opens (a K.O.'s cost).
    pub told: &'a [String],
    /// Come back to where it stood (after a film): no fade.
    pub cut: bool,
}

/// The scene's text: the place, who lives there, the team, the player at the keys.
pub fn scene_text(place: &str, decor: &Decor, making: &Making) -> String {
    let hero = making.team.first().cloned().unwrap_or_else(|| "pikachu".to_string());
    let partner = making.team.get(1).cloned();
    let mut residents: Vec<&decor::lang::Resident> = Vec::new();
    for resident in &decor.residents {
        let taken = residents.iter().any(|one| one.name == resident.name) || resident.name == hero || Some(&resident.name) == partner.as_ref();
        if !taken && (making.holds)(resident.condition.as_deref()) {
            residents.push(resident);
        }
    }
    let mut text = String::new();
    if making.cut {
        text.push_str("transition: cut\n");
    }
    let names: Vec<String> = std::iter::once(hero.clone()).chain(partner.clone()).chain(residents.iter().map(|one| one.name.clone())).collect();
    text.push_str(&format!("scene: {place} {}\n", names.join(" ")));
    for resident in &residents {
        text.push_str(&format!("{} put {} {} facing {}\n", resident.name, resident.x, resident.y, resident.facing));
        if resident.wanders > 0 {
            text.push_str(&format!("{} wander {}\n", resident.name, resident.wanders));
        }
    }
    let (at, behind, way) = standing(decor, &making.arrival);
    text.push_str(&format!("{hero} put {} {} facing {way}\n", at.0, at.1));
    if let Some(partner) = &partner {
        text.push_str(&format!("{partner} put {} {} facing {way}\n", behind.0, behind.1));
    }
    match &partner {
        Some(partner) => text.push_str(&format!("player {hero} followed by {partner}\n")),
        None => text.push_str(&format!("player {hero}\n")),
    }
    for resident in &residents {
        let tests = resident.tests.iter().map(Option::as_deref).chain(std::iter::repeat(None));
        let lines: Vec<&String> = resident.lines.iter().zip(tests).filter(|(_, test)| (making.holds)(*test)).map(|(line, _)| line).collect();
        let tells = (making.talking)(&resident.name);
        if lines.is_empty() && !tells {
            continue;
        }
        text.push_str(&format!("dialogue {}\n", resident.name));
        for line in lines {
            text.push_str(&format!("  {}: {}\n", resident.name, line));
        }
        if tells {
            text.push_str(&format!("  game talk {}\n", resident.name));
        }
        text.push_str("end\n");
    }
    // (Each as written: a rectangle. An exit a little larger, so feet that
    // stop at the place's edge still reach a strip along it.)
    for exit in &decor.ways_out {
        let (x, y, wide, high) = (exit.x - EXIT_MARGIN, exit.y - EXIT_MARGIN, exit.width + 2 * EXIT_MARGIN, exit.height + 2 * EXIT_MARGIN);
        text.push_str(&format!("way {x} {y} {wide} {high} go {} {}\n", exit.to, exit.entrance.clone().unwrap_or_default()));
    }
    for zone in &decor.zones {
        text.push_str(&format!("way {} {} {} {} zone {}\n", zone.x, zone.y, zone.width.max(1), zone.height.max(1), zone.name));
    }
    for told in making.told {
        text.push_str(&format!("narrator: {told}\n"));
    }
    text.push_str("explore\n");
    text
}
