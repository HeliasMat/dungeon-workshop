//! The decor's text changed by what is done on the scene: each thing one can
//! take (a piece, someone who lives there, a spot, an entrance, an exit),
//! moved, taken out, copied and pasted, lined up; the models kept apart
//! (`library/decors/models/<name>.model`, lines laid from 0 0); the
//! walls painted tile by tile (`wall tiles …`, `free tiles …`).

use std::collections::BTreeSet;
use std::path::PathBuf;

use decor::doc::{Pose, laid_in};

/// What a thing on the scene is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Piece,
    Lives,
    Spot,
    Entrance,
    Exit,
    /// A zone of the place (`zone name x y w h`): walking into it plays its `on zone`.
    Zone,
}

/// A thing one can take on the scene: its line, which of its points, what
/// it is called, that point, what it is, the way it faces (someone who
/// lives there), its layer, its size (an exit), whether locked.
#[derive(Clone, Debug)]
pub struct Item {
    pub line: usize,
    pub at: usize,
    pub name: String,
    pub corner: [f32; 2],
    pub kind: Kind,
    pub facing: usize,
    pub layer: &'static str,
    pub size: [f32; 2],
    pub locked: bool,
    /// What someone who lives there does (its animation's name), held or not.
    pub anim: Option<String>,
    pub still: bool,
}

/// The layers of the editor, from under all: each its word and its name.
pub const LAYERS: [(&str, &str); 7] = [
    ("terrains", "Terrains"),
    ("ground", "Ground"),
    ("objects", "Objects"),
    ("residents", "Residents"),
    ("over", "Over"),
    ("markers", "Markers"),
    ("walls", "Walls"),
];

/// A selection: lines and their points.
pub type Selection = Vec<(usize, usize)>;

/// Tiles of 8: column, row.
pub type Tiles = BTreeSet<(i64, i64)>;

/// A number as a decor writes it.
fn number(value: f32) -> String {
    if value.fract() == 0.0 { format!("{value:.0}") } else { format!("{value}") }
}

/// The things of a text one can take.
pub fn items(text: &str) -> Vec<Item> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<Item> = Vec::new();
    for one in laid_in(text) {
        let pose = lines.get(one.line - 1).and_then(|line| Pose::read(line));
        let locked = pose.as_ref().is_some_and(|pose| pose.locked);
        let layer = if one.lives {
            "residents"
        } else {
            match pose.as_ref().and_then(|pose| pose.layer.as_deref()) {
                Some("ground") => "ground",
                Some("over") => "over",
                _ => "objects",
            }
        };
        let kind = if one.lives { Kind::Lives } else { Kind::Piece };
        let anim = pose.as_ref().filter(|_| one.lives).and_then(|pose| pose.animation.clone());
        let still = one.lives && pose.as_ref().is_some_and(|pose| pose.still);
        out.push(Item {
            line: one.line,
            at: one.at,
            name: one.name,
            corner: one.corner,
            kind,
            facing: one.facing,
            layer,
            size: [0.0, 0.0],
            locked,
            anim,
            still,
        });
    }
    for (at, line) in lines.iter().enumerate() {
        let words: Vec<&str> = line.split('#').next().unwrap_or_default().split_whitespace().collect();
        let point = |x: &str, y: &str| -> Option<[f32; 2]> { Some([x.parse().ok()?, y.parse().ok()?]) };
        let made = match words.as_slice() {
            ["spot", name, x, y] => point(x, y).map(|corner| (Kind::Spot, name.to_string(), corner, [0.0, 0.0])),
            ["entrance", x, y, name @ ..] => point(x, y).map(|corner| (Kind::Entrance, name.join(" "), corner, [0.0, 0.0])),
            ["exit", x, y, w, h, "to", to, entrance @ ..] if entrance.len() <= 1 => {
                let name = std::iter::once(*to).chain(entrance.iter().copied()).collect::<Vec<_>>().join(" ");
                point(x, y).zip(point(w, h)).map(|(corner, size)| (Kind::Exit, format!("to {name}"), corner, size))
            }
            ["zone", name, x, y, w, h] => point(x, y).zip(point(w, h)).map(|(corner, size)| (Kind::Zone, name.to_string(), corner, size)),
            _ => None,
        };
        if let Some((kind, name, corner, size)) = made {
            out.push(Item { line: at + 1, at: 0, name, corner, kind, facing: 0, layer: "markers", size, locked: false, anim: None, still: false });
        }
    }
    out
}

/// A line with one of its points moved there.
pub fn with_point(line: &str, kind: Kind, at: usize, point: [f32; 2]) -> String {
    let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
    let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    let (x, y) = match kind {
        Kind::Piece | Kind::Lives => return Pose::read(line).map_or_else(|| line.to_string(), |pose| pose.moved(at, point).line()),
        Kind::Spot | Kind::Zone => (2, 3),
        Kind::Entrance | Kind::Exit => (1, 2),
    };
    if words.len() > y {
        (words[x], words[y]) = (number(point[0]), number(point[1]));
    }
    lead + &words.join(" ")
}

/// A line moved as a whole.
pub fn shifted_line(line: &str, dx: f32, dy: f32) -> String {
    let mut out = line.to_string();
    for item in items(line) {
        out = with_point(&out, item.kind, item.at, [item.corner[0] + dx, item.corner[1] + dy]);
    }
    out
}

fn joined(lines: Vec<String>) -> String {
    lines.join("\n") + "\n"
}

/// The things chosen moved by so much.
pub fn moved(text: &str, chosen: &[(usize, usize)], (dx, dy): (f32, f32)) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    for item in items(text).iter().filter(|item| chosen.contains(&(item.line, item.at))) {
        let line = &mut lines[item.line - 1];
        *line = with_point(line, item.kind, item.at, [item.corner[0] + dx, item.corner[1] + dy]);
    }
    joined(lines)
}

/// Each thing chosen put there.
pub fn placed(text: &str, to: &[((usize, usize), [f32; 2])]) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let all = items(text);
    for ((line, at), point) in to {
        if let Some(item) = all.iter().find(|item| item.line == *line && item.at == *at) {
            let made = with_point(&lines[line - 1], item.kind, *at, *point);
            lines[line - 1] = made;
        }
    }
    joined(lines)
}

/// The things chosen taken out: a line laying several keeps the others.
pub fn removed(text: &str, chosen: &[(usize, usize)]) -> String {
    let mut lines: Vec<Option<String>> = text.lines().map(|line| Some(line.to_string())).collect();
    let numbers: BTreeSet<usize> = chosen.iter().map(|(line, _)| *line).collect();
    for number in numbers {
        let Some(Some(line)) = lines.get(number - 1) else { continue };
        let gone: Vec<usize> = chosen.iter().filter(|(known, _)| *known == number).map(|(_, at)| *at).collect();
        lines[number - 1] = match Pose::read(line) {
            Some(pose) if pose.points.len() > gone.len() => {
                let points = pose.points.iter().enumerate().filter(|(at, _)| !gone.contains(at)).map(|(_, point)| *point).collect();
                Some(Pose { points, ..pose }.line())
            }
            _ => None,
        };
    }
    joined(lines.into_iter().flatten().collect())
}

/// The things chosen as lines of their own, from 0 0 (their least corner).
pub fn copied(text: &str, chosen: &[(usize, usize)]) -> Vec<String> {
    let all: Vec<Item> = items(text).into_iter().filter(|item| chosen.contains(&(item.line, item.at))).collect();
    let (x0, y0) = all.iter().fold((f32::MAX, f32::MAX), |(x, y), item| (x.min(item.corner[0]), y.min(item.corner[1])));
    let lines: Vec<&str> = text.lines().collect();
    let numbers: BTreeSet<usize> = all.iter().map(|item| item.line).collect();
    numbers
        .into_iter()
        .filter_map(|number| {
            let line = lines.get(number - 1)?.trim_start();
            let kept: Vec<usize> = all.iter().filter(|item| item.line == number).map(|item| item.at).collect();
            let line = match Pose::read(line) {
                Some(pose) => {
                    let points = pose.points.iter().enumerate().filter(|(at, _)| kept.contains(at)).map(|(_, point)| *point).collect();
                    Pose { points, lead: String::new(), ..pose }.line()
                }
                None => line.to_string(),
            };
            Some(shifted_line(&line, -x0, -y0))
        })
        .collect()
}

/// Lines from 0 0 put at the end, from that point: the text, and what they lay (chosen).
pub fn pasted(text: &str, clip: &[String], at: [f32; 2]) -> (String, Selection) {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    // (A spot pasted named anew if its name is taken: `well2`.)
    let mut names: Vec<String> = items(text).into_iter().filter(|one| one.kind == Kind::Spot).map(|one| one.name).collect();
    let made: Vec<String> = clip
        .iter()
        .map(|line| {
            let line = shifted_line(line, at[0].round(), at[1].round());
            let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
            if words.first().is_some_and(|word| word == "spot") && words.len() == 4 {
                let base = words[1].trim_end_matches(|letter: char| letter.is_ascii_digit()).to_string();
                if names.contains(&words[1]) {
                    words[1] = (2..).map(|at| format!("{base}{at}")).find(|name| !names.contains(name)).unwrap_or_default();
                }
                names.push(words[1].clone());
                return words.join(" ");
            }
            line
        })
        .collect();
    let (life, seen): (Vec<String>, Vec<String>) = made.into_iter().partition(|line| is_life(line));
    // (What is seen before `[life]`, the life of the place at the end.)
    let life_at = lines.iter().position(|line| line.trim() == "[life]");
    let first = life_at.unwrap_or(lines.len());
    let count = seen.len();
    lines.splice(first..first, seen);
    if !life.is_empty() && life_at.is_none() && lines.iter().all(|line| line.trim() != "[life]") {
        lines.push("[life]".to_string());
    }
    let last = lines.len() + 1;
    lines.extend(life);
    let made = joined(lines);
    let new = |line: usize| (first + 1..first + 1 + count).contains(&line) || line >= last;
    let chosen = items(&made).into_iter().filter(|item| new(item.line)).map(|item| (item.line, item.at)).collect();
    (made, chosen)
}

/// Whether a line is of the life of the place (after `[life]`).
pub fn is_life(line: &str) -> bool {
    line.split_whitespace().next().is_some_and(|word| decor::lang::LIFE.contains(&word))
}

/// How things are lined up.
#[derive(Clone, Copy)]
pub enum Line {
    Left,
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

/// The things chosen lined up on the first chosen's edge (their boxes as drawn).
pub fn lined_up(text: &str, boxes: &[((usize, usize), eframe::egui::Rect, [f32; 2])], how: Line) -> String {
    let Some((_, first, _)) = boxes.first() else {
        return text.to_string();
    };
    let edge = |rect: &eframe::egui::Rect| match how {
        Line::Left => rect.min.x,
        Line::Centre => rect.center().x,
        Line::Right => rect.max.x,
        Line::Top => rect.min.y,
        Line::Middle => rect.center().y,
        Line::Bottom => rect.max.y,
    };
    let target = edge(first);
    let across = matches!(how, Line::Left | Line::Centre | Line::Right);
    let to: Vec<((usize, usize), [f32; 2])> = boxes
        .iter()
        .map(|(key, rect, corner)| {
            let gap = (target - edge(rect)).round();
            (*key, if across { [corner[0] + gap, corner[1]] } else { [corner[0], corner[1] + gap] })
        })
        .collect();
    placed(text, &to)
}

// --- Models ---

/// Where the models are kept.
pub fn models_folder(library: &assets::project::Project) -> PathBuf {
    library.decors().join("models")
}

/// The models there are: their names and lines.
pub fn models(library: &assets::project::Project) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = std::fs::read_dir(models_folder(library))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|own| own == "model"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            let lines =
                std::fs::read_to_string(&path).ok()?.lines().filter(|line| !line.trim().is_empty() && !line.starts_with('#')).map(str::to_string).collect();
            Some((name, lines))
        })
        .collect();
    out.sort();
    out
}

/// Lines kept as a model of that name.
pub fn save_model(library: &assets::project::Project, name: &str, lines: &[String]) -> Result<PathBuf, String> {
    let name: String = name.trim().chars().map(|letter| if letter.is_alphanumeric() || letter == '_' { letter } else { '_' }).collect();
    if name.is_empty() {
        return Err("a model has a name".to_string());
    }
    let folder = models_folder(library);
    std::fs::create_dir_all(&folder).map_err(|why| why.to_string())?;
    let path = folder.join(format!("{name}.model"));
    std::fs::write(&path, lines.join("\n") + "\n").map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(path)
}

// --- Walls painted ---

/// The tiles walled and let walk on by hand (`wall tiles`, `free tiles`).
pub fn painted_walls(text: &str) -> (Tiles, Tiles) {
    let (mut walled, mut free) = (BTreeSet::new(), BTreeSet::new());
    for line in text.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let into = match words.as_slice() {
            ["wall", "tiles", ..] => &mut walled,
            ["free", "tiles", ..] => &mut free,
            _ => continue,
        };
        for word in &words[2..] {
            if let Some((column, row)) = word.split_once(',')
                && let (Ok(column), Ok(row)) = (column.parse(), row.parse())
            {
                into.insert((column, row));
            }
        }
    }
    (walled, free)
}

/// Those tiles walled (true), let walk on (false) or left as laid (none):
/// the tiles painted by hand written again, a line each, at the end.
pub fn with_walls(text: &str, tiles: &BTreeSet<(i64, i64)>, how: Option<bool>) -> String {
    let (mut walled, mut free) = painted_walls(text);
    for tile in tiles {
        walled.remove(tile);
        free.remove(tile);
        match how {
            Some(true) => _ = walled.insert(*tile),
            Some(false) => _ = free.insert(*tile),
            None => {}
        }
    }
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            !matches!(words.as_slice(), ["wall" | "free", "tiles", ..])
        })
        .map(str::to_string)
        .collect();
    let said = |tiles: &BTreeSet<(i64, i64)>| tiles.iter().map(|(column, row)| format!("{column},{row}")).collect::<Vec<_>>().join(" ");
    // (Before `[life]`: what is seen goes before the life of the place.)
    let life = lines.iter().position(|line| line.trim() == "[life]").unwrap_or(lines.len());
    let mut made = Vec::new();
    if !walled.is_empty() {
        made.push(format!("wall tiles {}", said(&walled)));
    }
    if !free.is_empty() {
        made.push(format!("free tiles {}", said(&free)));
    }
    for (at, line) in made.into_iter().enumerate() {
        lines.insert(life + at, line);
    }
    joined(lines)
}

/// A line put before `[life]` (what is seen), or after it (the life of the place).
pub fn with_line_added(text: &str, line: &str, life: bool) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let at = if life { lines.len() } else { lines.iter().position(|line| line.trim() == "[life]").unwrap_or(lines.len()) };
    lines.insert(at, line.to_string());
    if life && !text.lines().any(|line| line.trim() == "[life]") {
        lines.insert(at, "[life]".to_string());
    }
    joined(lines)
}

// --- Groups ---

/// The groups of a text (`group name` … `end`): each its name, its first line, its last.
pub fn groups(text: &str) -> Vec<(String, usize, usize)> {
    let mut out: Vec<(String, usize, usize)> = Vec::new();
    for (at, line) in text.lines().enumerate() {
        match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["group", name] => out.push((name.to_string(), at + 1, 0)),
            ["end"] => {
                if let Some(open) = out.iter_mut().rev().find(|(_, _, end)| *end == 0) {
                    open.2 = at + 1;
                }
            }
            _ => {}
        }
    }
    out.retain(|(_, _, end)| *end > 0);
    out
}

/// Those lines made a group, together where the first was: none of them of
/// the life of the place, nor in a group already.
pub fn grouped(text: &str, numbers: &[usize], name: &str) -> Result<String, String> {
    let name: String = name.split_whitespace().collect::<Vec<_>>().join("_");
    if name.is_empty() {
        return Err("a group has a name".to_string());
    }
    let mut numbers: Vec<usize> = numbers.to_vec();
    numbers.sort_unstable();
    numbers.dedup();
    let lines: Vec<&str> = text.lines().collect();
    let life_at = lines.iter().position(|line| line.trim() == "[life]").map_or(usize::MAX, |at| at + 1);
    if numbers.iter().any(|number| *number > life_at || is_life(lines.get(number - 1).unwrap_or(&""))) {
        return Err("residents, exits and what follows [life] do not go in a group".to_string());
    }
    if numbers.iter().any(|number| groups(text).iter().any(|(_, start, end)| start < number && number < end)) {
        return Err("already in a group: ungroup it first".to_string());
    }
    let Some(first) = numbers.first().copied() else {
        return Err("nothing chosen".to_string());
    };
    let taken: Vec<String> = numbers.iter().filter_map(|number| lines.get(number - 1)).map(|line| format!("  {}", line.trim_start())).collect();
    let mut made: Vec<String> = lines.iter().enumerate().filter(|(at, _)| !numbers.contains(&(at + 1))).map(|(_, line)| line.to_string()).collect();
    let at = first - 1;
    let mut block = vec![format!("group {name}")];
    block.extend(taken);
    block.push("end".to_string());
    made.splice(at..at, block);
    Ok(joined(made))
}

/// A group undone: its lines kept where they are.
pub fn ungrouped(text: &str, start: usize, end: usize) -> String {
    joined(
        text.lines()
            .enumerate()
            .filter(|(at, _)| at + 1 != start && at + 1 != end)
            .map(|(at, line)| if at + 1 > start && at + 1 < end { line.trim_start().to_string() } else { line.to_string() })
            .collect(),
    )
}

// --- History ---

/// What a change did, in a few words: what it added, took out, moved, changed.
pub fn described(before: &str, after: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    let first = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let last = a[first..].iter().rev().zip(b[first..].iter().rev()).take_while(|(x, y)| x == y).count();
    let (gone, came) = (&a[first..a.len() - last], &b[first..b.len() - last]);
    let head = |line: &str| line.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
    let more = |count: usize| if count > 1 { format!(" (+{})", count - 1) } else { String::new() };
    let words = |line: &str| line.split_whitespace().filter(|word| word.trim_end_matches(',').parse::<f32>().is_err()).collect::<Vec<_>>().join(" ");
    match (gone.len(), came.len()) {
        (0, 0) => "nothing".to_string(),
        (0, count) => format!("added: {}{}", head(came[0]), more(count)),
        (count, 0) => format!("removed: {}{}", head(gone[0]), more(count)),
        (x, y) if x == y && gone.iter().zip(came).all(|(was, now)| words(was) == words(now)) => format!("moved: {}{}", head(came[0]), more(x)),
        (1, 1) => format!("changed: {}", head(came[0])),
        (x, y) => format!("{} lines changed", x.max(y)),
    }
}

/// A line taken from where it is and put after another (`after`, as
/// numbered before), with that indent: none if it is of the life of the
/// place, or would go past `[life]`.
pub fn line_moved(text: &str, from: usize, after: usize, lead: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let line = *lines.get(from.checked_sub(1)?)?;
    let life_at = lines.iter().position(|line| line.trim() == "[life]").map_or(usize::MAX, |at| at + 1);
    if from == after || is_life(line) || after >= life_at || after > lines.len() {
        return None;
    }
    let mut made: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    made.remove(from - 1);
    let at = if from <= after { after - 1 } else { after };
    made.insert(at, format!("{lead}{}", line.trim_start()));
    Some(joined(made))
}

/// Lines `start` to `end` (both in) taken out.
pub fn removed_lines(text: &str, start: usize, end: usize) -> String {
    joined(text.lines().enumerate().filter(|(at, _)| !(start..=end).contains(&(at + 1))).map(|(_, line)| line.to_string()).collect())
}

/// A piece's line put on a layer (`ground`, `objects`, `over`): none if it lays no piece.
pub fn on_layer(text: &str, number: usize, layer: &str) -> Option<String> {
    let line = text.lines().nth(number.checked_sub(1)?)?;
    let pose = Pose::read(line).filter(|pose| pose.word != "resident")?;
    let layer = (layer != "objects").then(|| layer.to_string());
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    lines[number - 1] = Pose { layer, ..pose }.line();
    Some(joined(lines))
}

/// A piece's line drawn after all that is laid (front), or before it (behind,
/// still over the terrains and grounds): moved in the file, out of its group.
pub fn to_front(text: &str, number: usize, front: bool) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let laid = |line: &&str| Pose::read(line).is_some_and(|pose| pose.word != "resident");
    let life_at = lines.iter().position(|line| line.trim() == "[life]").unwrap_or(lines.len());
    let after = if front { lines[..life_at].iter().rposition(|line| !line.trim().is_empty())? + 1 } else { lines.iter().position(laid)? };
    if after == number || (!front && after + 1 == number) {
        return None;
    }
    line_moved(text, number, after, "")
}

/// A thing named anew: a group, a spot, an entrance, someone who lives
/// there (his Pokémon), a piece (the spot of the films at its foot, none if
/// empty). None if that line has no name to change, or the name is empty
/// where one is needed.
pub fn renamed(text: &str, number: usize, word: &str) -> Option<String> {
    let word = word.split_whitespace().collect::<Vec<_>>().join("_");
    let lines: Vec<&str> = text.lines().collect();
    let line = *lines.get(number.checked_sub(1)?)?;
    let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
    let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    let made = match words.first()?.as_str() {
        "group" | "resident" | "spot" | "zone" if word.is_empty() => return None,
        "group" => format!("{lead}group {word}"),
        "spot" | "resident" | "zone" if words.len() > 1 => {
            words[1] = word;
            format!("{lead}{}", words.join(" "))
        }
        "entrance" if words.len() >= 3 => format!("{lead}entrance {} {} {word}", words[1], words[2]).trim_end().to_string(),
        "put" | "water" | "image" => {
            let pose = Pose::read(line)?;
            Pose { spot: (!word.is_empty()).then_some(word), ..pose }.line()
        }
        _ => return None,
    };
    let mut out: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    out[number - 1] = made;
    Some(joined(out))
}

// --- What the Inspector changes ---

/// The lines just after someone who lives there that are his (`says`, `talk`).
fn his_lines(text: &str, number: usize) -> Vec<&str> {
    text.lines().skip(number).take_while(|line| matches!(line.split_whitespace().next(), Some("says" | "talk"))).collect()
}

/// What someone who lives there says: the `says` lines just after his
/// (each as written after `says`: `if test: words`, or the words).
pub fn speech(text: &str, number: usize) -> Vec<String> {
    his_lines(text, number)
        .into_iter()
        .filter(|line| line.split_whitespace().next() == Some("says"))
        .map(|line| line.trim_start().trim_start_matches("says").trim().to_string())
        .collect()
}

/// What he does once talked to (`talk shop`, `talk film the_thanks`): its words after `talk`.
pub fn talk(text: &str, number: usize) -> Option<String> {
    his_lines(text, number).into_iter().find_map(|line| line.trim_start().strip_prefix("talk ").map(|rest| rest.trim().to_string()))
}

/// His lines made these: what he says (empty ones left out), then what he does talked to.
fn with_his_lines(text: &str, number: usize, said: &[String], talk: Option<&str>) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let count = his_lines(text, number).len();
    let mut made: Vec<String> = said.iter().filter(|line| !line.trim().is_empty()).map(|line| format!("says {}", line.trim())).collect();
    made.extend(talk.map(str::trim).filter(|talk| !talk.is_empty()).map(|talk| format!("talk {talk}")));
    lines.splice(number..number + count, made);
    joined(lines)
}

/// What he says made these (empty ones left out); what he does talked to kept.
pub fn with_speech(text: &str, number: usize, said: &[String]) -> String {
    with_his_lines(text, number, said, talk(text, number).as_deref())
}

/// What he does talked to made that (none: nothing but his lines).
pub fn with_talk(text: &str, number: usize, talk: Option<&str>) -> String {
    with_his_lines(text, number, &speech(text, number), talk)
}

/// A line he says, its test apart: `if not met: Welcome!` is (`not met`, `Welcome!`).
pub fn said_test(said: &str) -> (String, String) {
    match said.strip_prefix("if ").and_then(|tested| tested.split_once(':')) {
        Some((test, words)) => (test.trim().to_string(), words.trim().to_string()),
        None => (String::new(), said.to_string()),
    }
}

/// A line he says, with its test (none if empty).
pub fn said_with_test(test: &str, words: &str) -> String {
    match test.trim() {
        "" => words.to_string(),
        test => format!("if {test}: {words}"),
    }
}

/// The test someone who lives there is there by (`resident kecleon 120 80 if chapter >= 2`).
pub fn condition(line: &str) -> Option<String> {
    let words: Vec<&str> = line.split('#').next().unwrap_or_default().split_whitespace().collect();
    let at = words.iter().position(|word| *word == "if")?;
    Some(words[at + 1..].join(" ")).filter(|test| !test.is_empty())
}

/// His line with that test (none: there always).
pub fn with_condition(line: &str, test: Option<&str>) -> String {
    let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
    let words: Vec<&str> = line.split_whitespace().collect();
    let kept = words.iter().position(|word| *word == "if").unwrap_or(words.len());
    let mut made = words[..kept].join(" ");
    if let Some(test) = test.map(str::trim).filter(|test| !test.is_empty()) {
        made.push_str(" if ");
        made.push_str(test);
    }
    lead + &made
}

/// What the game does in a place (`on enter`, `on talk kecleon if …`, `on zone pier` … `end`):
/// its first line as written, its lines (their indent taken off), where it starts and ends.
#[derive(Clone, Debug, PartialEq)]
pub struct LifeRule {
    pub head: String,
    pub body: String,
    pub start: usize,
    pub end: usize,
}

/// The words a rule is known by: those of its head before its `if` (`on talk kecleon`).
pub fn rule_key(head: &str) -> String {
    head.split_whitespace().take_while(|word| *word != "if").collect::<Vec<_>>().join(" ")
}

/// How many blocks a line opens (1), closes (-1), or none, as the game reads them.
fn opens(line: &str) -> i32 {
    match line.split_whitespace().next() {
        Some("if" | "on" | "choice") => 1,
        Some("end") => -1,
        _ => 0,
    }
}

/// Whether lines close every block they open (an `if` has its `end`).
pub fn balanced(body: &str) -> bool {
    let mut depth = 0;
    for line in body.lines() {
        depth += opens(line);
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

/// The rules of a place (after `[life]`).
pub fn life_rules(text: &str) -> Vec<LifeRule> {
    let lines: Vec<&str> = text.lines().collect();
    let life = lines.iter().position(|line| line.trim() == "[life]").map_or(lines.len(), |at| at + 1);
    let mut rules = Vec::new();
    let mut at = life;
    while at < lines.len() {
        if lines[at].split_whitespace().next() != Some("on") {
            at += 1;
            continue;
        }
        let mut depth = 1;
        let mut end = at + 1;
        while end < lines.len() {
            depth += opens(lines[end]);
            if depth == 0 {
                break;
            }
            end += 1;
        }
        let inner = &lines[at + 1..end.min(lines.len())];
        let indent = inner.iter().filter(|line| !line.trim().is_empty()).map(|line| line.len() - line.trim_start().len()).min().unwrap_or(0);
        let body = inner.iter().map(|line| line.get(indent..).unwrap_or(line.trim_start()).trim_end()).collect::<Vec<_>>().join("\n");
        rules.push(LifeRule { head: lines[at].trim().to_string(), body, start: at + 1, end: end + 1 });
        at = end + 1;
    }
    rules
}

/// The rule known by that head's words made to say that (its head kept as
/// written); none yet: written at the end, after `[life]`; nothing to say:
/// taken out.
pub fn with_rule(text: &str, head: &str, body: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let said: Vec<String> = body.lines().map(|line| if line.trim().is_empty() { String::new() } else { format!("  {}", line.trim_end()) }).collect();
    let empty = body.trim().is_empty();
    match life_rules(text).into_iter().find(|rule| rule_key(&rule.head) == rule_key(head)) {
        Some(rule) if empty => _ = lines.drain(rule.start - 1..rule.end.min(lines.len())),
        Some(rule) => _ = lines.splice(rule.start..(rule.end - 1).min(lines.len()), said),
        None if empty => {}
        None => {
            if !lines.iter().any(|line| line.trim() == "[life]") {
                lines.push("[life]".to_string());
            }
            if lines.last().is_some_and(|line| !line.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(head.trim().to_string());
            lines.extend(said);
            lines.push("end".to_string());
        }
    }
    joined(lines)
}

/// The decor's size, as its first line says it.
pub fn with_size(text: &str, w: i64, h: i64) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if let Some(first) = lines.iter_mut().find(|line| line.starts_with("decor:")) {
        let words: Vec<&str> = first.split_whitespace().collect();
        if words.len() >= 2 {
            *first = format!("decor: {} {w} {h}", words[1]);
        }
    }
    joined(lines)
}

// --- The backdrop ---

/// The piece a place of the game leaves once its elements are taken out
/// (`r<level>`, its whole ground layer): the level, if it is one.
pub fn rest_level(name: &str) -> Option<i64> {
    name.strip_prefix('r').filter(|level| !level.is_empty() && level.bytes().all(|byte| byte.is_ascii_digit())).and_then(|level| level.parse().ok())
}

/// The backdrop a decor is laid on: the first rest of a place it lays (`put r<level> …`), its line and that place.
pub fn backdrop_of(text: &str) -> Option<(usize, i64)> {
    text.lines().enumerate().find_map(|(at, line)| {
        let pose = Pose::read(line).filter(|pose| pose.word == "put")?;
        rest_level(&pose.name).map(|level| (at + 1, level))
    })
}

/// The line laying a place's rest as the backdrop: at 0 0, on the ground,
/// its shadows the place's own, locked (the mouse takes what is over it).
pub fn backdrop_line(level: i64) -> String {
    Pose {
        word: "put".to_string(),
        name: format!("r{level}"),
        points: vec![[0.0, 0.0]],
        shadowless: true,
        layer: Some("ground".to_string()),
        locked: true,
        ..Pose::default()
    }
    .line()
}

/// The decor laid on that place's backdrop: the backdrop it had changed (where
/// it was, as it was laid), else one put before the first piece laid (under
/// them all, over the grounds and terrains).
pub fn with_backdrop(text: &str, level: i64) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if let Some((number, _)) = backdrop_of(text)
        && let Some(pose) = Pose::read(&lines[number - 1])
    {
        lines[number - 1] = Pose { name: format!("r{level}"), ..pose }.line();
        return joined(lines);
    }
    let life_at = lines.iter().position(|line| line.trim() == "[life]").unwrap_or(lines.len());
    let first_laid = lines[..life_at].iter().position(|line| Pose::read(line).is_some_and(|pose| pose.word != "resident"));
    let at = first_laid.unwrap_or_else(|| lines[..life_at].iter().rposition(|line| !line.trim().is_empty()).map_or(life_at, |last| last + 1));
    lines.insert(at, backdrop_line(level));
    joined(lines)
}

/// Whether that piece is one of a place's own elements (`e<level>_<n>`, `a<level>_<n>`).
fn element_of(name: &str, level: i64) -> bool {
    let prefix = format!("{level}_");
    name.strip_prefix('e').or_else(|| name.strip_prefix('a')).is_some_and(|rest| rest.starts_with(&prefix))
}

/// The decor laid on that place, with the place's elements (`elements`: their
/// `put` lines, where the game has them) laid over its backdrop: the elements
/// of the place it was laid on before taken out with its backdrop.
pub fn with_place(text: &str, level: i64, elements: &[String]) -> String {
    let before = backdrop_of(text).map(|(_, old)| old);
    let kept: Vec<&str> =
        text.lines().filter(|line| !before.is_some_and(|old| Pose::read(line).is_some_and(|pose| pose.word == "put" && element_of(&pose.name, old)))).collect();
    let laid = with_backdrop(&(kept.join("\n") + "\n"), level);
    let Some((number, _)) = backdrop_of(&laid) else { return laid };
    let mut lines: Vec<String> = laid.lines().map(str::to_string).collect();
    for (at, element) in elements.iter().enumerate() {
        lines.insert(number + at, element.clone());
    }
    joined(lines)
}

/// The decor without its backdrop.
pub fn without_backdrop(text: &str) -> String {
    match backdrop_of(text) {
        Some((number, _)) => removed_lines(text, number, number),
        None => text.to_string(),
    }
}

/// The pieces of those lines turned over, or back (someone who lives there left as he is).
pub fn flipped(text: &str, lines: &[usize]) -> String {
    decor::doc::each_pose(text, lines, |pose| Some(if pose.word == "resident" { pose } else { Pose { turned: !pose.turned, ..pose } }))
}

/// A setting of the decor (`name`, `grid`, `level`, `seed`, `music`): its words, if said.
pub fn setting(text: &str, word: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next() == Some(word)).then(|| words.collect::<Vec<_>>().join(" "))
    })
}

/// A setting said (or taken out, if none): where it was, else after the
/// first lines (before `[life]`; `music` after it).
pub fn with_setting(text: &str, word: &str, value: Option<&str>) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let found = lines.iter().position(|line| line.split_whitespace().next() == Some(word));
    match (found, value) {
        (Some(at), Some(value)) => lines[at] = format!("{word} {value}"),
        (Some(at), None) => _ = lines.remove(at),
        (None, Some(value)) if word == "music" => return with_line_added(text, &format!("music {value}"), true),
        (None, Some(value)) => {
            let at = lines.iter().position(|line| !line.starts_with("decor:")).unwrap_or(lines.len());
            lines.insert(at, format!("{word} {value}"));
        }
        (None, None) => {}
    }
    joined(lines)
}

/// Lines made others, each by its number.
pub fn with_lines(text: &str, made: &[(usize, String)]) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    for (number, line) in made {
        if let Some(at) = number.checked_sub(1).filter(|at| *at < lines.len()) {
            lines[at] = line.clone();
        }
    }
    joined(lines)
}

/// Things spread so that the gaps between them are the same, across or down
/// (the first and the last stay).
pub fn spread(text: &str, boxes: &[((usize, usize), eframe::egui::Rect, [f32; 2])], across: bool) -> String {
    if boxes.len() < 3 {
        return text.to_string();
    }
    let mut sorted: Vec<&((usize, usize), eframe::egui::Rect, [f32; 2])> = boxes.iter().collect();
    let start = |rect: &eframe::egui::Rect| if across { rect.min.x } else { rect.min.y };
    let side = |rect: &eframe::egui::Rect| if across { rect.width() } else { rect.height() };
    sorted.sort_by(|a, b| start(&a.1).total_cmp(&start(&b.1)));
    let first = start(&sorted[0].1);
    let last = sorted.last().map_or(first, |one| start(&one.1) + side(&one.1));
    let filled: f32 = sorted.iter().map(|one| side(&one.1)).sum();
    let gap = (last - first - filled) / (sorted.len() - 1) as f32;
    let mut at = first;
    let mut to = Vec::new();
    for (key, rect, corner) in sorted {
        let moved = (at - start(rect)).round();
        to.push((*key, if across { [corner[0] + moved, corner[1]] } else { [corner[0], corner[1] + moved] }));
        at += side(rect) + gap;
    }
    placed(text, &to)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLACE: &str = "decor: market 320 240\nname The Market\n[life]\nresident kecleon 120 80 down\nsays Welcome!\n";

    fn parsed(text: &str) -> decor::lang::Decor {
        decor::lang::parse(text, "market.decor").unwrap_or_else(|fault| panic!("{}\n{text}", fault.message))
    }

    /// A backdrop set (under the pieces, over the grounds), changed where it is, taken out; the decor still reads.
    #[test]
    fn a_backdrop_is_set_changed_and_taken_out() {
        let text = "decor: a 256 192\nname A\nground town_grass\nput e1_1 8 8\n[life]\nresident pikachu 50 60 down\n";
        let set = with_backdrop(text, 323);
        assert_eq!(text_line(&set, 4), "put r323 0 0 shadow off layer ground locked");
        assert_eq!(text_line(&set, 5), "put e1_1 8 8");
        assert_eq!(backdrop_of(&set), Some((4, 323)));
        parsed(&set);
        // (Changed: where it is, as it was laid, moved by hand too.)
        let moved = moved(&set, &[(4, 0)], (8.0, 0.0));
        let changed = with_backdrop(&moved, 11);
        assert_eq!(text_line(&changed, 4), "put r11 8 0 shadow off layer ground locked");
        assert_eq!(changed.lines().count(), set.lines().count());
        assert_eq!(without_backdrop(&changed), text);
        // (Nothing laid yet: after the first lines, before the life.)
        let empty = with_backdrop("decor: b 64 64\nname B\n\n[life]\nmusic 3\n", 52);
        assert_eq!(empty, "decor: b 64 64\nname B\nput r52 0 0 shadow off layer ground locked\n\n[life]\nmusic 3\n");
        assert_eq!(rest_level("r12"), Some(12));
        assert_eq!(rest_level("rocher"), None);
        assert_eq!(rest_level("r"), None);
        assert_eq!(backdrop_of("decor: c 8 8\nput rocher 0 0\n"), None);
    }

    /// Turned over and back, residents left as they are; each a step undone in the Workshop.
    #[test]
    fn pieces_are_flipped_and_back() {
        let text = "decor: a 64 64\nput e1_1 8 8\nput e1_2 16 16 flip\n[life]\nresident pikachu 50 60 down\n";
        let once = flipped(text, &[2, 3, 5]);
        assert_eq!(text_line(&once, 2), "put e1_1 8 8 flip");
        assert_eq!(text_line(&once, 3), "put e1_2 16 16");
        assert_eq!(text_line(&once, 5), "resident pikachu 50 60 down");
        assert_eq!(flipped(&once, &[2, 3]), text);
    }

    fn text_line(text: &str, number: usize) -> String {
        text.lines().nth(number - 1).unwrap_or_default().to_string()
    }

    #[test]
    fn a_resident_s_new_life_reads_back() {
        let number = 4;
        let line = text_line(PLACE, number);
        let made = with_lines(PLACE, &[(number, with_condition(&line, Some("chapter >= 2")))]);
        let made = with_speech(&made, number, &[said_with_test("not met", "Welcome!"), "Hello again.".to_string()]);
        let made = with_talk(&made, number, Some("shop"));
        // (What he says changed again: what talking to him does is kept.)
        let made = with_speech(&made, number, &[said_with_test("not met", "Welcome, newcomer!")]);
        let read = parsed(&made);
        let kecleon = &read.residents[0];
        assert_eq!(kecleon.condition.as_deref(), Some("chapter >= 2"));
        assert_eq!(kecleon.lines, ["Welcome, newcomer!"]);
        assert_eq!(kecleon.tests, [Some("not met".to_string())]);
        assert_eq!(kecleon.talk.as_deref(), Some("shop"));
        assert_eq!(talk(&made, number).as_deref(), Some("shop"));
        assert_eq!(condition(&text_line(&made, number)).as_deref(), Some("chapter >= 2"));
        assert_eq!(said_test(&speech(&made, number)[0]), ("not met".to_string(), "Welcome, newcomer!".to_string()));
        // (Taken off again.)
        let made = with_talk(&with_lines(&made, &[(number, with_condition(&text_line(&made, number), None))]), number, None);
        let read = parsed(&made);
        assert_eq!((read.residents[0].condition.clone(), read.residents[0].talk.clone()), (None, None));
        let made = with_talk(&made, number, Some("film the_thanks"));
        assert_eq!(parsed(&made).residents[0].talk.as_deref(), Some("film the_thanks"));
    }

    #[test]
    fn zones_exits_and_rules_read_back() {
        let made = with_line_added(PLACE, "zone pier 40 60 32 16", true);
        let made = with_line_added(&made, "exit 300 100 16 32 to test_harbour left", true);
        let found = items(&made);
        let zone = found.iter().find(|one| one.kind == Kind::Zone).expect("the zone");
        assert_eq!((zone.name.as_str(), zone.corner, zone.size), ("pier", [40.0, 60.0], [32.0, 16.0]));
        let exit = found.iter().find(|one| one.kind == Kind::Exit).expect("the exit");
        assert_eq!(exit.name, "to test_harbour left");
        // (The zone dragged, named anew.)
        let made = moved(&made, &[(zone.line, 0)], (8.0, 4.0));
        let made = renamed(&made, zone.line, "jetty").expect("renamed");
        let made = with_rule(&made, "on enter", "set came\nif not met\n  play the_welcome\nend");
        let made = with_rule(&made, "on zone jetty", "go test_harbour left");
        let made = with_rule(&made, "on talk kecleon if not met", "set met");
        let read = parsed(&made);
        assert_eq!(read.zones.len(), 1);
        assert_eq!((read.zones[0].name.as_str(), read.zones[0].x, read.zones[0].y, read.zones[0].width), ("jetty", 48, 64, 32));
        assert_eq!(read.ways_out[0].to, "test_harbour");
        assert_eq!(read.ways_out[0].entrance.as_deref(), Some("left"));
        let heads: Vec<&str> = read.rules.iter().map(|rule| rule.head.as_str()).collect();
        assert_eq!(heads, ["on enter", "on zone jetty", "on talk kecleon if not met"]);
        assert_eq!(read.rules[0].body, "  set came\n  if not met\n    play the_welcome\n  end\n");
        // (A rule changed keeps its head; emptied, it goes.)
        let made = with_rule(&made, "on enter", "set came_again");
        let rules = life_rules(&made);
        assert_eq!(rules[0].body, "set came_again");
        assert_eq!(rules.len(), 3);
        let made = with_rule(&made, "on talk kecleon", "");
        let read = parsed(&made);
        assert_eq!(read.rules.len(), 2);
        assert_eq!(read.residents.len(), 1);
        assert!(balanced("if a\n  set b\nend") && !balanced("if a\n  set b"));
    }
}
