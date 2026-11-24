//! The tables of the game's files, read into what they mean (their columns:
//! `data/DATA.md`). Each is read merged, as [`crate::data::text`] gives it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::data;

/// A text's lines cut in fields, but the empty ones and the comments (`#`).
pub fn rows(text: &str) -> Vec<Vec<String>> {
    text.lines().filter(|line| !line.trim().is_empty() && !line.starts_with('#')).map(|line| line.split('\t').map(str::to_string).collect()).collect()
}

/// A list of the game, merged, in fields.
pub fn rows_of(folder: &str, file: &str) -> Vec<Vec<String>> {
    rows(&data::text(folder, file))
}

/// A field as a number; 0 if it is none.
fn number(field: &str) -> i32 {
    field.parse().unwrap_or(0)
}

/// A place's picture (`backdrops.txt`): its level, its name, its size in pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backdrop {
    pub level: u16,
    pub name: String,
    pub width: i32,
    pub height: i32,
}

/// Every place's picture, the game's and those `decor` built.
pub fn backdrops(folder: &str) -> Vec<Backdrop> {
    let backdrop = |row: Vec<String>| {
        let level = row.first()?.parse().ok()?;
        Some(Backdrop { level, name: row.get(1)?.clone(), width: number(row.get(2)?), height: number(row.get(3)?) })
    };
    rows_of(folder, "backdrops.txt").into_iter().filter_map(backdrop).collect()
}

/// A place `decor` built (`decors.txt`): its name, its level, where a group
/// stands in it, the name it shows, its entrances (name as written, x, y).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Built {
    pub name: String,
    pub level: u16,
    pub middle: (i32, i32),
    pub shown: String,
    pub entrances: Vec<(String, i32, i32)>,
}

/// The places `decor` built.
pub fn built(folder: &str) -> Vec<Built> {
    let built = |row: Vec<String>| {
        let entrances = row.get(5).map_or("", String::as_str).split(';').filter_map(|way| {
            let mut parts = way.split(',');
            let (x, y) = (parts.next()?.trim().parse().ok()?, parts.next()?.trim().parse().ok()?);
            Some((parts.next()?.to_string(), x, y))
        });
        Built {
            name: row[0].clone(),
            level: number(&row[1]) as u16,
            middle: (number(&row[2]), number(&row[3])),
            shown: row.get(4).cloned().unwrap_or_default(),
            entrances: entrances.collect(),
        }
    };
    rows_of(folder, "decors.txt").into_iter().filter(|row| row.len() > 3).map(built).collect()
}

/// Where one walks in a place (`ground.txt`): its level, who drew it
/// (`jeu`, the game's grid; `main`, traced), its rows of tiles (`0`, `1`)
/// joined by `/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundRows {
    pub level: u16,
    pub how: String,
    pub rows: String,
}

impl GroundRows {
    /// Its tiles, row by row: whether each is solid.
    pub fn tiles(&self) -> Vec<Vec<bool>> {
        self.rows.split('/').filter(|row| !row.is_empty()).map(|row| row.bytes().map(|tile| tile == b'1').collect()).collect()
    }
}

/// Every place's ground there is.
pub fn grounds(folder: &str) -> Vec<GroundRows> {
    let ground = |row: Vec<String>| Some(GroundRows { level: row[0].parse().ok()?, how: row[1].clone(), rows: row[2].clone() });
    rows_of(folder, "ground.txt").into_iter().filter(|row| row.len() > 2).filter_map(ground).collect()
}

/// A list of names by their numbers (`monsters.txt`, `items.txt`, `moves.txt`).
pub fn numbered(folder: &str, file: &str) -> Vec<(u16, String)> {
    rows_of(folder, file).into_iter().filter_map(|row| Some((row.first()?.parse().ok()?, row.get(1)?.clone()))).collect()
}

/// The scripts' characters, in their order (`entities.txt`): name, monster
/// (0: the hero or the partner, by the name).
pub fn entities(folder: &str) -> Vec<(String, u16)> {
    rows_of(folder, "entities.txt")
        .into_iter()
        .filter_map(|row| Some((row.get(1)?.clone(), row.get(2).and_then(|monster| monster.parse().ok()).unwrap_or(0))))
        .collect()
}

/// Which tiles of a place are solid, by rows, as `town.txt` tells them.
/// (It is read through once for every place, and again once it changes.)
pub fn solid_rows(folder: &str, level: u16) -> Vec<Vec<bool>> {
    type Places = HashMap<u16, Vec<Vec<bool>>>;
    type Kept = Mutex<HashMap<String, (Arc<str>, Places)>>;
    static KEPT: OnceLock<Kept> = OnceLock::new();
    let text = data::text_kept(folder, "town.txt");
    let mut kept = KEPT.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((read, places)) = kept.get(folder)
        && Arc::ptr_eq(read, &text)
    {
        return places.get(&level).cloned().unwrap_or_default();
    }
    let (mut places, mut here): (Places, Option<u16>) = (HashMap::new(), None);
    for line in text.lines() {
        let mut fields = line.split('\t');
        match fields.next() {
            Some("place") => here = fields.next().and_then(|place| place.parse().ok()),
            Some("solid") => {
                if let Some(level) = here {
                    places.entry(level).or_default().push(fields.next().unwrap_or("").bytes().map(|tile| tile == b'1').collect());
                }
            }
            _ => {}
        }
    }
    let rows = places.get(&level).cloned().unwrap_or_default();
    kept.insert(folder.to_string(), (text, places));
    rows
}

/// An animation's cell and steps: a cell's size, the 60ths of a second of
/// each step, where a shadow lies from its middle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sheet {
    pub wide: u32,
    pub high: u32,
    pub durations: Vec<u8>,
    pub shadow: (i32, i32),
}

/// The original game's groups of a sprite's animations, by the names the
/// PMDCollab community gives them (as its sheets' AnimData.xml writes them;
/// the sprites added by hand in `data/extra/` use them too). The last two
/// only variants have, painted by hand.
pub const ANIMATION_GROUPS: [(&str, usize); 37] = [
    ("Walk", 0),
    ("Attack", 1),
    ("Kick", 2),
    ("Shoot", 3),
    ("Strike", 4),
    ("Sleep", 5),
    ("Hurt", 6),
    ("Idle", 7),
    ("Swing", 8),
    ("Double", 9),
    ("Hop", 10),
    ("Charge", 11),
    ("Rotate", 12),
    ("EventSleep", 13),
    ("Wake", 14),
    ("Eat", 15),
    ("Tumble", 16),
    ("Pose", 17),
    ("Pull", 18),
    ("Pain", 19),
    ("Float", 20),
    ("DeepBreath", 21),
    ("Nod", 22),
    ("Sit", 23),
    ("LookUp", 24),
    ("Sink", 25),
    ("Trip", 26),
    ("Laying", 27),
    ("LeapForth", 28),
    ("Head", 29),
    ("Cringe", 30),
    ("LostBalance", 31),
    ("TumbleBack", 32),
    ("Faint", 33),
    ("HitGround", 34),
    ("Vise", 50),
    ("FireRifle", 51),
];

/// A line of `sprites.txt` or `props.txt`: whose sheet (monster or object),
/// which animation, the sheet; for a thing, whether it lies on the ground.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetRow {
    pub number: u16,
    pub animation: u8,
    pub sheet: Sheet,
    pub flat: bool,
}

/// The sheets a list tells (`sprites.txt`, `props.txt`).
pub fn sheets(folder: &str, file: &str) -> Vec<SheetRow> {
    let sheet = |row: Vec<String>| {
        let field = |at: usize| row.get(at).and_then(|field| field.parse::<i32>().ok());
        let durations = row.get(4)?.split(',').filter_map(|ticks| ticks.parse().ok()).collect();
        let sheet = Sheet { wide: field(2)? as u32, high: field(3)? as u32, durations, shadow: (field(5).unwrap_or(0), field(6).unwrap_or(0)) };
        Some(SheetRow { number: field(0)? as u16, animation: field(1)? as u8, sheet, flat: row.get(7).is_some_and(|flat| flat == "1") })
    };
    rows_of(folder, file).into_iter().filter_map(sheet).collect()
}

/// An effect of the game (`effects.txt`): its pictures' size, each step's
/// 60ths of a second, how long it plays, whether it repeats or covers the
/// screen, its sound, the point of whom it shows on, how many ways it has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectRow {
    pub id: u16,
    pub wide: u32,
    pub high: u32,
    pub durations: Vec<u8>,
    pub ticks: u32,
    pub repeats: bool,
    pub screen: bool,
    pub sound: u16,
    pub point: u8,
    pub ways: u32,
}

/// Every effect of the game.
pub fn effects(folder: &str) -> Vec<EffectRow> {
    let effect = |row: Vec<String>| {
        let field = |at: usize| row.get(at).and_then(|field| field.parse::<u32>().ok());
        Some(EffectRow {
            id: row.first()?.parse().ok()?,
            wide: field(1)?,
            high: field(2)?,
            durations: row.get(3)?.split(',').filter_map(|ticks| ticks.parse().ok()).collect(),
            ticks: field(4)?,
            repeats: field(5)? != 0,
            screen: field(6)? != 0,
            sound: field(7)? as u16,
            point: field(8).unwrap_or(3) as u8,
            ways: field(9).unwrap_or(1).clamp(1, 8),
        })
    };
    rows_of(folder, "effects.txt").into_iter().filter_map(effect).collect()
}

/// A status icon (`status.txt`): its name, a picture's size, how many.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusRow {
    pub name: String,
    pub wide: u32,
    pub high: u32,
    pub frames: u32,
}

/// Every status icon.
pub fn statuses(folder: &str) -> Vec<StatusRow> {
    let status = |row: Vec<String>| {
        let field = |at: usize| row.get(at).and_then(|field| field.parse::<u32>().ok()).unwrap_or(0);
        StatusRow { wide: field(1), high: field(2), frames: field(3), name: row[0].clone() }
    };
    rows_of(folder, "status.txt").into_iter().map(status).collect()
}

/// Where each monster's head, hands and middle are (`points.txt`), by its number.
pub fn points(folder: &str) -> HashMap<u16, [(i32, i32); 4]> {
    let line = |row: Vec<String>| {
        let mut fields = row.iter();
        let monster = fields.next()?.parse().ok()?;
        let mut points = [(0, 0); 4];
        for point in &mut points {
            let (x, y) = fields.next()?.split_once(',')?;
            *point = (x.parse().ok()?, y.parse().ok()?);
        }
        Some((monster, points))
    };
    rows_of(folder, "points.txt").into_iter().filter_map(line).collect()
}

/// A variant (`variants.txt`, painted by hand): its number, the
/// species it is one of, its word, the ways painted (none: all of them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub number: u16,
    pub of: u16,
    pub word: String,
    pub ways: Option<String>,
}

/// Every variant.
pub fn variants(folder: &str) -> Vec<Variant> {
    let variant = |row: Vec<String>| {
        Some(Variant { number: row.first()?.parse().ok()?, of: row.get(1)?.parse().ok()?, word: row.get(2)?.clone(), ways: row.get(3).cloned() })
    };
    rows_of(folder, "variants.txt").into_iter().filter_map(variant).collect()
}

/// A step of a monster's animation: its marker ([`HIT_MARK`],
/// [`RETURN_MARK`], or 0), and where its head, hands and middle are then,
/// from its cell's middle (y downwards), if its sprite says.
pub type Step = (u8, Option<[(i32, i32); 4]>);
/// A monster's steps, by animation and way (`sprites/<monster>.txt`).
pub type Steps = HashMap<(u8, u8), Vec<Step>>;
/// The step a blow lands on; the one it is back in place from.
pub const HIT_MARK: u8 = 2;
pub const RETURN_MARK: u8 = 1;

/// `sprites/<monster>.txt`: `<animation> <way> <markers> <points>`, a
/// marker each step (comma-separated) and each step's points
/// (`x,y;x,y;x,y;x,y` or `-`, space-separated). Only from the layer its
/// sprite's walk or way of standing is drawn in: one drawn over by hand
/// has other steps. None: empty.
pub fn steps(folder: &str, monster: u16) -> Steps {
    let file = format!("sprites/{monster}.txt");
    let layer = |path: String| std::path::Path::new(&path).parent().and_then(|sprites| sprites.parent()).map(std::path::Path::to_path_buf);
    let drawn = [0, 1].iter().map(|at| format!("sprites/{monster}_{at}.png")).find(|png| data::has(folder, png));
    if drawn.is_none_or(|png| layer(data::path(folder, &png)) != layer(data::path(folder, &file))) {
        return Steps::new();
    }
    let point = |field: &str| field.split_once(',').and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?)));
    let points = |field: &str| -> Option<[(i32, i32); 4]> {
        let mut found = [(0, 0); 4];
        let mut each = field.split(';');
        for at in &mut found {
            *at = point(each.next()?)?;
        }
        Some(found)
    };
    let line = |row: Vec<String>| {
        let key = (row.first()?.parse().ok()?, row.get(1)?.parse().ok()?);
        let marks = row.get(2)?.split(',').map(|mark| mark.parse().unwrap_or(0));
        let at = row.get(3).map_or("", String::as_str).split(' ').map(points).chain(std::iter::repeat(None));
        Some((key, marks.zip(at).collect()))
    };
    rows(&data::text(folder, &file)).into_iter().filter_map(line).collect()
}

/// Where a held thing's tip is, by monster, animation and way: a point each
/// step (none: not seen then).
pub type Tips = HashMap<(u16, usize, usize), Vec<Option<(i32, i32)>>>;

/// `rod_tips.txt`: `<monster> <animation> <way> x,y ...`, a step each (`-`: none).
pub fn tips(folder: &str) -> Tips {
    let line = |row: Vec<String>| {
        let key = (row.first()?.parse().ok()?, row.get(1)?.parse().ok()?, row.get(2)?.parse().ok()?);
        let point = |field: &str| field.split_once(',').and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?)));
        Some((key, row.get(3)?.split(' ').map(point).collect()))
    };
    rows_of(folder, "rod_tips.txt").into_iter().filter_map(line).collect()
}

/// Someone who lives in a place where one walks (`town.txt`, `actor`):
/// the place's level, its entity, where it stands, the way it faces (1 to
/// 8), how far it wanders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    pub level: u16,
    pub entity: String,
    pub x: i32,
    pub y: i32,
    pub facing: i32,
    pub wanders: i32,
    /// What it does over and over, by its name (`Sleep`); held on its first picture.
    pub animation: Option<String>,
    pub still: bool,
}

/// Everyone `town.txt` puts in its places.
pub fn actors(folder: &str) -> Vec<Actor> {
    let (mut level, mut actors) = (0, Vec::new());
    for row in rows_of(folder, "town.txt") {
        match row[0].as_str() {
            "place" => level = number(row.get(1).map_or("", String::as_str)) as u16,
            "actor" if row.len() > 4 => actors.push(Actor {
                level,
                entity: row[1].clone(),
                x: number(&row[2]),
                y: number(&row[3]),
                facing: number(&row[4]),
                wanders: row.get(6).map_or(0, |wide| number(wide)),
                animation: row.get(7).filter(|name| !name.is_empty() && *name != "-").cloned(),
                still: row.get(8).is_some_and(|flag| flag == "1"),
            }),
            _ => {}
        }
    }
    actors
}

#[cfg(test)]
mod tests;
