//! A decor's text read and written line by line, as the Workshop works on it:
//! what each line lays (`put`, `water`: an element, its corners, turned
//! over, without its shadow, its layer, its name, hidden, locked), its
//! zones, paths, entrances, groups, the tiles a ground is painted on, its
//! header; each change a text made another, every other line as it was.
//! The same words as [`crate::lang`], which builds the decor from them; a
//! test sees that both read the project's decors alike.

/// Every line of a text, numbered from 1.
fn numbered(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines().enumerate().map(|(at, line)| (at + 1, line))
}

/// A line laying an element, in its parts: its indent, its word (`put`,
/// `water`), the element, the corners it is laid at, turned over or not,
/// without its shadow or not, its layer (`over`, `ground`), what else it says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pose {
    pub lead: String,
    pub word: String,
    pub name: String,
    pub points: Vec<[f32; 2]>,
    pub turned: bool,
    pub shadowless: bool,
    pub layer: Option<String>,
    /// A spot of the films at its foot (`name`); not laid (`hidden`); not
    /// taken by the pointer (`locked`).
    pub spot: Option<String>,
    pub hidden: bool,
    pub locked: bool,
    /// Still, though the game animates it (`still`).
    pub still: bool,
    /// How it lives (`anim glow slow`): its way, its pace.
    pub animation: Option<String>,
    pub pace: Option<String>,
    pub rest: Vec<String>,
}

impl Pose {
    /// A line laying an element, read; none if it is not one.
    pub fn read(line: &str) -> Option<Pose> {
        let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
        let words: Vec<&str> = line.split_whitespace().collect();
        let (word, name) = (*words.first()?, *words.get(1)?);
        // (Someone who lives there is laid too: by his feet, not a corner.)
        if !["put", "water", "image", "resident"].contains(&word) {
            return None;
        }
        let mut pose = Pose { lead, word: word.to_string(), name: name.to_string(), ..Pose::default() };
        let mut numbers: Vec<f32> = Vec::new();
        let mut at = 2;
        while at < words.len() {
            let said = words[at].trim_end_matches(',');
            match said {
                "flip" => pose.turned = true,
                "hidden" => pose.hidden = true,
                "locked" => pose.locked = true,
                "still" => pose.still = true,
                "name" if words.get(at + 1).is_some() => {
                    pose.spot = Some(words[at + 1].to_string());
                    at += 1;
                }
                "shadow" if words.get(at + 1) == Some(&"off") => {
                    pose.shadowless = true;
                    at += 1;
                }
                "layer" if words.get(at + 1).is_some() => {
                    pose.layer = Some(words[at + 1].to_string());
                    at += 1;
                }
                "anim" if words.get(at + 1).is_some() => {
                    pose.animation = Some(words[at + 1].to_string());
                    at += 1;
                    if let Some(pace) = words.get(at + 1).filter(|pace| crate::animate::PACES.iter().any(|(known, _)| known == *pace)) {
                        pose.pace = Some(pace.to_string());
                        at += 1;
                    }
                }
                number if number.parse::<f32>().is_ok() && pose.rest.is_empty() => numbers.push(number.parse().unwrap_or_default()),
                other => pose.rest.push(other.to_string()),
            }
            at += 1;
        }
        pose.points = numbers.as_chunks::<2>().0.to_vec();
        (!pose.points.is_empty()).then_some(pose)
    }

    /// The line it makes.
    pub fn line(&self) -> String {
        let number = |value: f32| if value.fract() == 0.0 { format!("{value:.0}") } else { format!("{value}") };
        let points: Vec<String> = self.points.iter().map(|[x, y]| format!("{} {}", number(*x), number(*y))).collect();
        let mut line = format!("{}{} {} {}", self.lead, self.word, self.name, points.join(", "));
        if self.turned {
            line.push_str(" flip");
        }
        if self.shadowless {
            line.push_str(" shadow off");
        }
        if let Some(layer) = &self.layer {
            line.push_str(&format!(" layer {layer}"));
        }
        if self.still {
            line.push_str(" still");
        }
        if let Some(kind) = &self.animation {
            line.push_str(&format!(" anim {kind}"));
            if let Some(pace) = &self.pace {
                line.push_str(&format!(" {pace}"));
            }
        }
        if let Some(spot) = &self.spot {
            line.push_str(&format!(" name {spot}"));
        }
        if self.locked {
            line.push_str(" locked");
        }
        if self.hidden {
            line.push_str(" hidden");
        }
        for word in &self.rest {
            line.push(' ');
            line.push_str(word);
        }
        line
    }

    /// The same, one of its corners moved there.
    pub fn moved(&self, at: usize, point: [f32; 2]) -> Pose {
        let mut points = self.points.clone();
        if let Some(corner) = points.get_mut(at) {
            *corner = point;
        }
        Pose { points, ..self.clone() }
    }
}

/// An element laid, on the scene: its line, which of its corners, its name,
/// that corner; or someone who lives there (`resident`), at his feet.
#[derive(Clone, Debug, PartialEq)]
pub struct Laid {
    pub line: usize,
    pub at: usize,
    pub name: String,
    pub corner: [f32; 2],
    pub lives: bool,
    /// The way he faces, as `lang::FACING` lists them (0: down).
    pub facing: usize,
}

/// The elements a decor lays, in the order they are laid (the last over the
/// others), one for each corner of a line.
pub fn laid_in(text: &str) -> Vec<Laid> {
    numbered(text)
        .filter_map(|(line, text)| Some((line, Pose::read(text)?)))
        .filter(|(_, pose)| !pose.hidden)
        .flat_map(|(line, pose)| {
            let lives = pose.word == "resident";
            let facing = pose.rest.first().and_then(|way| crate::lang::FACING.iter().position(|known| known == way)).unwrap_or(0);
            pose.points
                .iter()
                .enumerate()
                .map(move |(at, corner)| Laid { line, at, name: pose.name.clone(), corner: *corner, lives, facing })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The spots a decor names (`spot name x y`): their lines, names, points.
pub fn spots(text: &str) -> Vec<(usize, String, [f32; 2])> {
    numbered(text)
        .filter_map(|(number, line)| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["spot", name, x, y] => Some((number, name.to_string(), [x.parse().ok()?, y.parse().ok()?])),
            _ => None,
        })
        .collect()
}

/// The layer a line lays on: `ground`, `objects` or `over` (none if it lays
/// no element).
pub fn layer_of_line(line: &str) -> Option<&'static str> {
    let pose = Pose::read(line)?;
    Some(match pose.layer.as_deref().and_then(crate::lang::layer_of) {
        Some(Some(layer)) => layer,
        _ => "objects",
    })
}

/// The entrances a decor has: their points and names.
pub fn entrances(text: &str) -> Vec<([f32; 2], String)> {
    numbered(text)
        .filter_map(|(_, line)| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                ["entrance", x, y, name @ ..] => Some(([x.parse().ok()?, y.parse().ok()?], name.join(" "))),
                _ => None,
            }
        })
        .collect()
}

/// A zone of a decor's line (`ground ... zone x0 y0 x1 y1`, `scatter ... zone ...`):
/// its line, its word, what it lays, its corners.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub line: usize,
    pub word: String,
    pub name: String,
    pub corners: [[f32; 2]; 2],
}

/// The zones a decor's lines give.
pub fn zones(text: &str) -> Vec<Zone> {
    numbered(text)
        .filter_map(|(line, text)| {
            let words: Vec<&str> = text.split_whitespace().collect();
            let at = words.iter().position(|word| *word == "zone")?;
            let number = |offset: usize| words.get(at + offset)?.parse::<f32>().ok();
            let corners = [[number(1)?, number(2)?], [number(3)?, number(4)?]];
            Some(Zone { line, word: words.first()?.to_string(), name: words.get(1)?.to_string(), corners })
        })
        .collect()
}

/// A zone's line with one of its corners (0 the first, 1 the second) moved there.
pub fn zone_moved(line: &str, corner: usize, [x, y]: [f32; 2]) -> Option<String> {
    let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
    let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    let at = words.iter().position(|word| word == "zone")? + 1 + corner * 2;
    words.get(at + 1)?;
    (words[at], words[at + 1]) = (format!("{x:.0}"), format!("{y:.0}"));
    Some(format!("{lead}{}", words.join(" ")))
}

/// A decor's paths (`path ... from x y to x y ...`): their lines, names, points.
pub fn paths(text: &str) -> Vec<(usize, String, Vec<[f32; 2]>)> {
    numbered(text)
        .filter_map(|(line, text)| {
            let words: Vec<&str> = text.split_whitespace().collect();
            if words.first() != Some(&"path") {
                return None;
            }
            let mut points = Vec::new();
            let mut at = 2;
            while at + 2 < words.len() + 1 {
                match (words.get(at), words.get(at + 1).and_then(|x| x.parse().ok()), words.get(at + 2).and_then(|y| y.parse().ok())) {
                    (Some(&("from" | "to")), Some(x), Some(y)) => {
                        points.push([x, y]);
                        at += 3;
                    }
                    _ => break,
                }
            }
            Some((line, words.get(1)?.to_string(), points))
        })
        .collect()
}

/// The tiles a ground is painted on (`ground <ground> tiles c,r ...`): its
/// line, if it has one, and its tiles.
pub fn painted_tiles(text: &str, ground: &str) -> (Option<usize>, Vec<[i64; 2]>) {
    let found = numbered(text).filter_map(|(number, line)| {
        let words: Vec<&str> = line.split_whitespace().collect();
        (words.first() == Some(&"ground") && words.get(1) == Some(&ground) && words.get(2) == Some(&"tiles")).then_some((number, words))
    });
    match found.last() {
        Some((number, words)) => {
            let tiles =
                words[3..].iter().filter_map(|word| word.split_once(',').and_then(|(column, row)| Some([column.parse().ok()?, row.parse().ok()?]))).collect();
            (Some(number), tiles)
        }
        None => (None, Vec::new()),
    }
}

/// A decor's text with a ground painted on these tiles too (or taken off
/// them): its `tiles` line changed, or one written after the grounds.
pub fn with_tiles(text: &str, ground: &str, tiles: &[[i64; 2]], off: bool) -> String {
    let (line, mut painted) = painted_tiles(text, ground);
    for tile in tiles {
        match (off, painted.contains(tile)) {
            (true, true) => painted.retain(|known| known != tile),
            (false, false) => painted.push(*tile),
            _ => {}
        }
    }
    painted.sort_by_key(|[column, row]| (*row, *column));
    let cases: Vec<String> = painted.iter().map(|[column, row]| format!("{column},{row}")).collect();
    let said = (!cases.is_empty()).then(|| format!("ground {ground} tiles {}", cases.join(" ")));
    match (line, said) {
        (Some(line), said) => with_line(text, line, said),
        (None, Some(said)) => put_after(text, &["ground", "edge", "path"], &said),
        (None, None) => text.to_string(),
    }
}

/// How a tile of the collision map is painted in the Workshop: walled, let
/// walk on, or as what is laid there makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Walling {
    Wall,
    Free,
    Built,
}

/// The tiles painted walled (`wall tiles c,r ...`) and let walk on (`libre
/// cases ...`), whatever is laid there.
pub fn painted_walls(text: &str) -> (Vec<[i64; 2]>, Vec<[i64; 2]>) {
    let of = |verb: &str| -> Vec<[i64; 2]> {
        numbered(text)
            .filter_map(|(_, line)| {
                let words: Vec<&str> = line.split_whitespace().collect();
                (words.first() == Some(&verb) && words.get(1) == Some(&"tiles")).then_some(words)
            })
            .flat_map(|words| {
                words[2..]
                    .iter()
                    .filter_map(|word| word.split_once(',').and_then(|(column, row)| Some([column.parse().ok()?, row.parse().ok()?])))
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    (of("wall"), of("free"))
}

/// A decor's text with these tiles painted so: its `wall tiles` and `libre
/// cases` lines made anew (one each, before its life, `[vie]`), the others
/// taken out.
pub fn with_walls(text: &str, tiles: &[[i64; 2]], how: Walling) -> String {
    let (mut walls, mut free) = painted_walls(text);
    for tile in tiles {
        walls.retain(|known| known != tile);
        free.retain(|known| known != tile);
        match how {
            Walling::Wall => walls.push(*tile),
            Walling::Free => free.push(*tile),
            Walling::Built => {}
        }
    }
    let said = |verb: &str, tiles: &mut Vec<[i64; 2]>| {
        tiles.sort_by_key(|[column, row]| (*row, *column));
        tiles.dedup();
        let cases: Vec<String> = tiles.iter().map(|[column, row]| format!("{column},{row}")).collect();
        (!cases.is_empty()).then(|| format!("{verb} tiles {}", cases.join(" ")))
    };
    let made: Vec<String> = [said("wall", &mut walls), said("free", &mut free)].into_iter().flatten().collect();
    let painted = |line: &str| {
        let words: Vec<&str> = line.split_whitespace().collect();
        matches!(words.first(), Some(&"wall") | Some(&"free")) && words.get(1) == Some(&"tiles")
    };
    let mut lines: Vec<String> = text.lines().filter(|line| !painted(line)).map(str::to_string).collect();
    let at = lines.iter().position(|line| line.trim() == "[life]").unwrap_or(lines.len());
    // (After what is laid, a blank line kept before its life.)
    let at = if at > 0 && at < lines.len() && lines[at - 1].trim().is_empty() { at - 1 } else { at };
    lines.splice(at..at, made);
    whole(lines, text)
}

/// A decor's groups (`group name` … `end`): each its name, its first line
/// inside and the line of its end.
pub fn groups(text: &str) -> Vec<(String, usize, usize)> {
    let mut found = Vec::new();
    let mut open: Option<(String, usize)> = None;
    for (number, line) in numbered(text) {
        match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["group", name] => open = Some((name.to_string(), number + 1)),
            ["end"] => {
                if let Some((name, from)) = open.take() {
                    found.push((name, from, number));
                }
            }
            _ => {}
        }
    }
    found
}

/// A decor's text with several elements' corners moved, in one change.
pub fn moved_many(text: &str, moves: &[(usize, usize, [f32; 2])]) -> String {
    let mut made = text.to_string();
    for (line, at, point) in moves {
        let Some(pose) = made.lines().nth(line - 1).and_then(Pose::read) else {
            continue;
        };
        made = with_line(&made, *line, Some(pose.moved(*at, *point).line()));
    }
    made
}

/// A decor's text with these lines' elements changed so (each line once).
pub fn each_pose(text: &str, lines: &[usize], change: impl Fn(Pose) -> Option<Pose>) -> String {
    let mut lines = lines.to_vec();
    lines.sort_unstable();
    lines.dedup();
    let mut made = text.to_string();
    // (The last first: taking a line out leaves those before it where they are.)
    for line in lines.into_iter().rev() {
        let Some(pose) = made.lines().nth(line - 1).and_then(Pose::read) else {
            continue;
        };
        made = with_line(&made, line, change(pose).map(|pose| pose.line()));
    }
    made
}

/// A decor's text with those lines' elements doubled, a little aside, at its end.
pub fn doubled(text: &str, lines: &[usize], [dx, dy]: [f32; 2]) -> String {
    let mut lines = lines.to_vec();
    lines.sort_unstable();
    lines.dedup();
    let mut made = text.trim_end().to_string();
    for line in lines {
        if let Some(pose) = text.lines().nth(line - 1).and_then(Pose::read) {
            let points = pose.points.iter().map(|[x, y]| [x + dx, y + dy]).collect();
            made.push('\n');
            made.push_str(Pose { points, ..pose }.line().trim_start());
        }
    }
    made.push('\n');
    made
}

/// The places of the game a decor is made like (`like 11 12`).
pub fn compared(text: &str) -> Vec<String> {
    numbered(text)
        .filter_map(|(_, line)| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["like", levels @ ..] => Some(levels.iter().map(|level| level.to_string()).collect::<Vec<_>>()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// A decor's text with the elements of lines copied (from it or another
/// decor) laid again, so far aside, at its end; none if none was one.
pub fn pasted(text: &str, copied: &str, [dx, dy]: [f32; 2]) -> Option<String> {
    let poses: Vec<Pose> = copied.lines().filter_map(Pose::read).collect();
    if poses.is_empty() {
        return None;
    }
    let mut made = text.trim_end().to_string();
    for pose in poses {
        let points = pose.points.iter().map(|[x, y]| [x + dx, y + dy]).collect();
        made.push('\n');
        made.push_str(Pose { points, lead: String::new(), ..pose }.line().trim_start());
    }
    made.push('\n');
    Some(made)
}

/// What a decor says of itself, each with its line: its file's name and size,
/// its title, its main ground (the first `ground` with no zone), its edges, its
/// song, its seed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Header {
    pub name: String,
    pub size: [usize; 2],
    pub title: Option<String>,
    pub ground: Option<(usize, String)>,
    pub edges: Vec<(usize, String, String)>,
    pub song: Option<u16>,
    pub seed: Option<u64>,
    /// The step of its grid (`grid 16`), if it says one.
    pub grid: Option<usize>,
}

/// The grid's step when a decor says none: the game's tiles.
pub const GRID: usize = 8;

/// A decor's header, read from its text (lines from 1).
pub fn header(text: &str) -> Header {
    let mut header = Header::default();
    for (number, line) in text.lines().enumerate().map(|(at, line)| (at + 1, line)) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["decor:", name, wide, high, ..] => {
                header.name = name.to_string();
                header.size = [wide.parse().unwrap_or(0), high.parse().unwrap_or(0)];
            }
            ["name", rest @ ..] if !rest.is_empty() => header.title = Some(rest.join(" ")),
            ["ground", ground] if header.ground.is_none() => header.ground = Some((number, ground.to_string())),
            ["edge", edge, rest @ ..] => header.edges.push((number, edge.to_string(), rest.join(" "))),
            ["music", song] => header.song = song.parse().ok(),
            ["seed", seed] => header.seed = seed.parse().ok(),
            ["grid", step] => header.grid = step.parse().ok().filter(|step| (1..=256).contains(step)),
            _ => {}
        }
    }
    header
}

/// The text with a field (`name`, `music`, `seed`, `grid`) said so: its line
/// changed, else one written after the `decor:` line (or its `name`); none,
/// its line taken out.
pub fn with_field(text: &str, word: &str, value: Option<&str>) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let said = |line: &String| line.split_whitespace().next() == Some(word);
    match (lines.iter().position(said), value) {
        (Some(at), Some(value)) => lines[at] = format!("{word} {value}"),
        (Some(at), None) => _ = lines.remove(at),
        (None, Some(value)) => {
            let after = ["name", "decor:"].iter().find_map(|first| lines.iter().position(|line| line.split_whitespace().next() == Some(*first)));
            lines.insert(after.map_or(0, |at| at + 1), format!("{word} {value}"));
        }
        (None, None) => {}
    }
    whole(lines, text)
}

/// The lines put back together, ending as the text did.
pub fn whole(lines: Vec<String>, text: &str) -> String {
    let mut made = lines.join("\n");
    if text.ends_with('\n') {
        made.push('\n');
    }
    made
}

/// The text with its `decor:` line saying that name, that size.
pub fn with_head(text: &str, name: &str, [wide, high]: [usize; 2]) -> String {
    let lines = text
        .lines()
        .map(|line| match line.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["decor:", _, _, _, rest @ ..] => {
                let rest = rest.iter().map(|word| format!(" {word}")).collect::<String>();
                format!("decor: {name} {wide} {high}{rest}")
            }
            _ => line.to_string(),
        })
        .collect();
    whole(lines, text)
}

/// The nine points a decor is resized from: what stays where it is (its
/// word, and where across and along: 0 the left or top, 1 the middle, 2 the
/// right or bottom).
pub const ANCHORS: [(&str, [i64; 2]); 9] = [
    ("top_left", [0, 0]),
    ("top", [1, 0]),
    ("top_right", [2, 0]),
    ("left", [0, 1]),
    ("center", [1, 1]),
    ("right", [2, 1]),
    ("bottom_left", [0, 2]),
    ("bottom", [1, 2]),
    ("bottom_right", [2, 2]),
];

/// An anchor by its word (`top-left`, `topleft`, `top_left`, `middle`).
pub fn anchor_of(word: &str) -> Option<[i64; 2]> {
    let bare: String = word.chars().filter(|letter| letter.is_alphanumeric()).collect();
    let bare = if bare == "middle" { "center".to_string() } else { bare };
    ANCHORS.iter().find(|(known, _)| known.replace('_', "") == bare).map(|(_, anchor)| *anchor)
}

/// The text with its size made that, what it lays moved so that the
/// anchor's point of it stays where it was (from the top left corner, the
/// right and bottom grow; from the middle, all sides alike).
pub fn resized(text: &str, [wide, high]: [usize; 2], [across, along]: [i64; 2]) -> String {
    let said = header(text);
    let [old_wide, old_high] = said.size.map(|side| side as i64);
    let shift = [(wide as i64 - old_wide) * across / 2, (high as i64 - old_high) * along / 2];
    shifted(&with_head(text, &said.name, [wide, high]), shift)
}

/// The text with all it lays, its zones, paths, entrances, spots, walls and
/// residents moved by so many pixels (its ways in from another place's
/// ground stay: they are in that place); hidden lines (comments) too.
pub fn shifted(text: &str, [dx, dy]: [i64; 2]) -> String {
    if (dx, dy) == (0, 0) {
        return text.to_string();
    }
    let lines = text
        .lines()
        .map(|line| {
            let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
            let body = &line[lead.len()..];
            // (A hidden line, `# put ...`: moved as it would be shown.)
            let (hash, body) = match body.strip_prefix('#') {
                Some(rest) => (format!("#{}", rest.chars().take_while(|letter| letter.is_whitespace()).collect::<String>()), rest.trim_start()),
                None => (String::new(), body),
            };
            // (What is said, the spaces after it, a note after `#` kept as it is.)
            let said = if hash.is_empty() { body.split('#').next().unwrap_or_default() } else { body };
            let (kept, after) = (said.trim_end(), &body[said.trim_end().len()..]);
            match shifted_line(kept, dx, dy) {
                Some(moved) => format!("{lead}{hash}{moved}{after}"),
                None => line.to_string(),
            }
        })
        .collect();
    whole(lines, text)
}

/// One line moved by so many pixels, if it says where something is.
fn shifted_line(line: &str, dx: i64, dy: i64) -> Option<String> {
    if let Some(pose) = Pose::read(line) {
        let points = pose.points.iter().map(|[x, y]| [x + dx as f32, y + dy as f32]).collect();
        return Some(Pose { points, ..pose }.line());
    }
    let mut words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
    let by = |words: &mut Vec<String>, at: usize, by: i64| -> Option<()> {
        let said = words.get(at)?;
        let comma = if said.ends_with(',') { "," } else { "" };
        let value: i64 = said.trim_end_matches(',').parse().ok()?;
        words[at] = format!("{}{comma}", value + by);
        Some(())
    };
    let pair = |words: &mut Vec<String>, at: usize| -> Option<()> {
        by(words, at, dx)?;
        by(words, at + 1, dy)
    };
    match words.first()?.as_str() {
        "entrance" | "center" => pair(&mut words, 1)?,
        "spot" | "resident" | "image" => pair(&mut words, 2)?,
        "exit" => pair(&mut words, 1)?,
        "wall" | "free" => {
            pair(&mut words, 1)?;
            pair(&mut words, 3)?;
        }
        "path" => {
            let at: Vec<usize> = words.iter().enumerate().filter(|(_, word)| *word == "from" || *word == "to").map(|(at, _)| at + 1).collect();
            for at in at {
                pair(&mut words, at)?;
            }
        }
        "ring" => {
            let at = words.iter().position(|word| word == "oval")?;
            pair(&mut words, at + 1)?;
        }
        "edge" => {
            let said: i64 = words.get(2)?.parse().ok()?;
            // (A band starts on the grid of 8.)
            words[2] = ((((said + dy) as f64 / 8.0).round() as i64).max(0) * 8).to_string();
        }
        "ground" if words.get(2).is_some_and(|word| word == "tiles") => {
            let (columns, rows) = ((dx as f64 / 8.0).round() as i64, (dy as f64 / 8.0).round() as i64);
            for word in &mut words[3..] {
                if let Some((column, row)) = word.split_once(',').and_then(|(column, row)| Some((column.parse::<i64>().ok()?, row.parse::<i64>().ok()?))) {
                    *word = format!("{},{}", column + columns, row + rows);
                }
            }
        }
        "base" if words.get(1).is_some_and(|word| word == "level") => {
            words.resize(words.len().max(5), "0".to_string());
            pair(&mut words, 3)?;
        }
        _ => {}
    }
    if let Some(at) = words.iter().position(|word| word == "zone") {
        pair(&mut words, at + 1)?;
        pair(&mut words, at + 3)?;
    }
    let made = words.join(" ");
    (made != line.split_whitespace().collect::<Vec<_>>().join(" ")).then_some(made)
}

/// A new decor's text, the one way one starts (`decor new`, the
/// Workshop's "New Decor"): its name and title, its size, its level
/// (fixed once made), its ground, an edge (on a side) if one, its song if
/// one, an entrance at its bottom middle.
pub fn made(name: &str, title: &str, [wide, high]: [usize; 2], level: Option<i64>, ground: &str, edge: Option<(&str, &str)>, song: Option<u16>) -> String {
    let title = if title.trim().is_empty() {
        let spaced = name.replace('_', " ");
        let mut letters = spaced.chars();
        letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect())
    } else {
        title.trim().to_string()
    };
    let mut text = format!("# {title}: what one sees there, where one comes from.\ndecor: {name} {wide} {high}\n");
    if let Some(level) = level {
        text.push_str(&format!("level {level}\n"));
    }
    text.push_str(&format!("name {title}\n"));
    if let Some(song) = song {
        text.push_str(&format!("music {song}\n"));
    }
    text.push_str(&format!("entrance {} {} down\n", wide / 2 / 8 * 8, high.saturating_sub(16) / 8 * 8));
    if !ground.is_empty() {
        text.push_str(&format!("\nground {ground}\n"));
    }
    if let Some((edge, side)) = edge {
        text.push_str(&format!("edge {edge} {side}\n"));
    }
    text
}

/// A decor of before written as one of now (`decor migrate`), building the
/// same: its level said (that one, if it says none), its layers by their
/// names of now, hidden lines too.
pub fn migrated(text: &str, level: i64) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    // (Each line keeps its number: what is strewn and woven draws its chance
    // from it. The level takes the first empty line after the head, before
    // anything is laid; else it goes at the end.)
    let said = |line: &String, word: &str| line.split_whitespace().next() == Some(word);
    lived(&mut lines);
    if !lines.iter().any(|line| said(line, "level"))
        && let Some(head) = lines.iter().position(|line| said(line, "decor:"))
    {
        let header = ["name", "like", "center", "seed", "grid", "music", "entrance", "spot", "departure", "base"];
        let laying = lines.iter().skip(head + 1).position(|line| {
            let first = line.split_whitespace().next().unwrap_or("#");
            !first.starts_with('#') && !header.contains(&first)
        });
        let end = laying.map_or(lines.len(), |at| head + 1 + at);
        match (head + 1..end).find(|at| lines[*at].trim().is_empty()) {
            Some(at) => lines[at] = format!("level {level}"),
            None => match lines.iter().position(|line| line.trim() == "[life]") {
                Some(at) => lines.insert(at, format!("level {level}")),
                None => {
                    while lines.last().is_some_and(|line| line.trim().is_empty()) {
                        lines.pop();
                    }
                    lines.push(format!("level {level}"));
                }
            },
        }
    }
    whole(lines, text)
}

/// A place's life put in its own section, `[life]`, after what is seen:
/// what of it is already after all that is laid stays where it is, `[life]`
/// said above it (with the notes just over it); what is said before, its
/// line left empty (the others keep their numbers), comes after `[life]`.
fn lived(lines: &mut Vec<String>) {
    let first = |line: &String| line.split_whitespace().next().unwrap_or("").to_string();
    if lines.iter().any(|line| line.trim() == "[life]") || !lines.iter().any(|line| crate::lang::LIFE.contains(&first(line).as_str())) {
        return;
    }
    let seen = |line: &String| {
        let word = first(line);
        !word.is_empty() && !word.starts_with('#') && !crate::lang::LIFE.contains(&word.as_str())
    };
    let last_seen = lines.iter().rposition(seen);
    let mut early = Vec::new();
    for line in lines.iter_mut().take(last_seen.unwrap_or(0)) {
        if crate::lang::LIFE.contains(&first(line).as_str()) {
            early.push(std::mem::take(line));
        }
    }
    let after = last_seen.map_or(0, |at| at + 1);
    let mut at = (after..lines.len()).find(|at| crate::lang::LIFE.contains(&first(&lines[*at]).as_str())).unwrap_or(lines.len());
    while at > after && lines[at - 1].trim_start().starts_with('#') {
        at -= 1;
    }
    let mut made = vec!["[life]".to_string()];
    made.extend(early);
    lines.splice(at..at, made);
}

/// The text with one line made another (none: taken out); lines from 1.
pub fn with_line(text: &str, number: usize, line: Option<String>) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if (1..=lines.len()).contains(&number) {
        match line {
            Some(line) => lines[number - 1] = line,
            None => _ = lines.remove(number - 1),
        }
    }
    whole(lines, text)
}

/// The text with a line put after the last of those beginning with one of
/// these words (a ground's zone after the grounds and edges, before what is
/// laid on them), else at its end.
pub fn put_after(text: &str, words: &[&str], line: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let last = lines.iter().rposition(|said| said.split_whitespace().next().is_some_and(|first| words.contains(&first)));
    match last {
        Some(at) => lines.insert(at + 1, line.to_string()),
        None => lines.push(line.to_string()),
    }
    whole(lines, text)
}

/// A zone drawn, as a decor writes it (its corners on the grid, the least
/// first).
pub fn zone([[x0, y0], [x1, y1]]: [[f32; 2]; 2]) -> String {
    let grid = |at: f32| ((at / 8.0).round() * 8.0).max(0.0) as i64;
    format!("zone {} {} {} {}", grid(x0.min(x1)), grid(y0.min(y1)), grid(x0.max(x1)), grid(y0.max(y1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_copied_are_laid_again_aside_and_the_places_it_is_like_read() {
        let text = "decor: a 64 64\nlike 11 12\nground town_grass\n";
        let made = pasted(text, "put beach_rock 8 8 flip\nground other\n  water pond 16 16", [16.0, 16.0]).expect("laid");
        assert_eq!(made, "decor: a 64 64\nlike 11 12\nground town_grass\nput beach_rock 24 24 flip\nwater pond 32 32\n");
        assert_eq!(pasted(text, "ground town_grass", [0.0, 0.0]), None);
        assert_eq!(compared(text), ["11", "12"]);
    }

    #[test]
    fn a_decor_resized_from_an_anchor_moves_what_it_lays() {
        let text = "decor: a 256 192\nentrance 128 184 down\nground town_grass zone 0 0 64 64\nedge beach_sea 72\nput beach_rock 8 8, 16 16 flip  # two\n# put crate 40 40\npath dirt from 0 100 to 128 100 width 16\nwall 0 0 8 8\nground sable tiles 1,1 2,1\nresident pikachu 50 60 down\nlink 11 0 0 8 8\n# A note.\n";
        assert_eq!(resized(text, [300, 200], [0, 0]), text.replace("256 192", "300 200"), "from the top left: nothing moves");
        let moved = resized(text, [320, 208], anchor_of("bottom-right").expect("an anchor"));
        let lines: Vec<&str> = moved.lines().collect();
        assert_eq!(lines[0], "decor: a 320 208");
        assert_eq!(lines[1], "entrance 192 200 down");
        assert_eq!(lines[2], "ground town_grass zone 64 16 128 80");
        assert_eq!(lines[3], "edge beach_sea 88");
        assert_eq!(lines[4], "put beach_rock 72 24, 80 32 flip  # two");
        assert_eq!(lines[5], "# put crate 104 56");
        assert_eq!(lines[6], "path dirt from 64 116 to 192 116 width 16");
        assert_eq!(lines[7], "wall 64 16 72 24");
        assert_eq!(lines[8], "ground sable tiles 9,3 10,3");
        assert_eq!(lines[9], "resident pikachu 114 76 down");
        assert_eq!((lines[10], lines[11]), ("link 11 0 0 8 8", "# A note."), "another place's zone, a note: as they were");
        assert_eq!(resized(text, [256, 192], [1, 1]), text);
        assert_eq!(anchor_of("middle"), Some([1, 1]));
        assert_eq!(anchor_of("topleft"), Some([0, 0]));
    }

    #[test]
    fn a_decor_of_before_is_migrated_its_level_said() {
        let text = "# x\ndecor: a 64 64\nname A\nput b 1 2 layer over\n# put c 1 2 layer ground\nscatter d 3 layer ground\n";
        assert_eq!(migrated(text, 470), "# x\ndecor: a 64 64\nname A\nput b 1 2 layer over\n# put c 1 2 layer ground\nscatter d 3 layer ground\nlevel 470\n");
        assert_eq!(
            migrated("decor: a 64 64\nname A\n\nground b\n\n", 7),
            "decor: a 64 64\nname A\nlevel 7\nground b\n\n",
            "an empty line before what is laid: the lines keep their numbers"
        );
        let again = migrated(text, 470);
        assert_eq!(migrated(&again, 999), again, "once is enough");
    }

    #[test]
    fn a_places_life_goes_after_what_is_seen() {
        let text = "decor: a 64 64\nlevel 1\nmusic 3\nground b\nput c 1 2\n\n# Its residents.\nresident eevee 8 8\nsays Hello.\n";
        let made = migrated(text, 1);
        assert_eq!(made, "decor: a 64 64\nlevel 1\n\nground b\nput c 1 2\n\n[life]\nmusic 3\n# Its residents.\nresident eevee 8 8\nsays Hello.\n");
        let read = crate::lang::parse(&made, "a.decor").expect("read");
        assert_eq!((read.music, read.residents.len(), read.residents[0].lines.len()), (Some(3), 1, 1));
        assert!(crate::lang::parse("decor: a 64 64\n[life]\nground b\n", "a.decor").is_err(), "what is seen goes before");
        assert_eq!(migrated(&made, 1), made);
    }

    #[test]
    fn a_decor_says_the_step_of_its_grid() {
        let text = "decor: a 64 64\ngrid 16\n";
        assert_eq!(header(text).grid, Some(16));
        assert_eq!(crate::lang::parse(text, "a.decor").expect("read").grid, Some(16));
        assert_eq!(header(&with_field(text, "grid", None)).grid, None);
        assert!(crate::lang::parse("decor: a 64 64\ngrid 0\n", "a.decor").is_err());
    }

    #[test]
    fn a_ground_is_painted_on_tiles_and_taken_off_them() {
        let text = "decor: a 64 64\nground town_grass\nput beach_rock 8 8\n";
        let painted = with_tiles(text, "beach_sand", &[[2, 1], [1, 1]], false);
        assert_eq!(painted.lines().nth(2), Some("ground beach_sand tiles 1,1 2,1"));
        let more = with_tiles(&painted, "beach_sand", &[[1, 1], [0, 3]], false);
        assert_eq!(more.lines().nth(2), Some("ground beach_sand tiles 1,1 2,1 0,3"));
        let less = with_tiles(&more, "beach_sand", &[[1, 1], [2, 1], [0, 3]], true);
        assert_eq!(less, text, "no tile left: the line goes");
        assert_eq!(painted_tiles(&more, "beach_sand").1.len(), 3);
    }

    #[test]
    fn an_element_lives_and_tiles_are_walled_or_let_walk_on_by_hand() {
        let pose = Pose::read("put beach_rock 8 8 anim glow slow name fire").expect("a put");
        assert_eq!((pose.animation.as_deref(), pose.pace.as_deref(), pose.spot.as_deref()), (Some("glow"), Some("slow"), Some("fire")));
        assert_eq!(pose.line(), "put beach_rock 8 8 anim glow slow name fire");
        let text = "decor: a 64 64\nground town_grass\n\n[life]\nmusic 3\n";
        let walled = with_walls(text, &[[1, 2], [0, 2]], Walling::Wall);
        assert_eq!(walled, "decor: a 64 64\nground town_grass\nwall tiles 0,2 1,2\n\n[life]\nmusic 3\n");
        let freed = with_walls(&walled, &[[1, 2], [3, 3]], Walling::Free);
        assert_eq!(painted_walls(&freed), (vec![[0, 2]], vec![[1, 2], [3, 3]]));
        assert_eq!(with_walls(&freed, &[[0, 2], [1, 2], [3, 3]], Walling::Built), text);
        assert!(crate::read("a", &walled).is_ok());
    }

    #[test]
    fn a_pose_says_its_name_and_whether_it_is_hidden_or_locked() {
        let pose = Pose::read("put beach_rock 8 8 flip name big_rock locked hidden").expect("a put");
        assert_eq!((pose.spot.as_deref(), pose.locked, pose.hidden, pose.turned), (Some("big_rock"), true, true, true));
        assert_eq!(pose.line(), "put beach_rock 8 8 flip name big_rock locked hidden");
        let doc = "decor: a 64 64\ngroup cliff\nput beach_rock 8 8\nend\nput crate 8 8 hidden\n";
        assert_eq!(groups(doc), vec![("cliff".to_string(), 3, 4)]);
        assert_eq!(laid_in(doc).len(), 1, "the hidden one is not on the stage");
    }

    #[test]
    fn elements_chosen_together_are_moved_doubled_turned_and_taken_out_in_one_text() {
        let text = "decor: a 256 192\nground town_grass\nput beach_rock 8 8\nput crate 40 16, 80 16\n";
        let moved = moved_many(text, &[(3, 0, [16.0, 24.0]), (4, 1, [96.0, 24.0])]);
        assert_eq!(moved, "decor: a 256 192\nground town_grass\nput beach_rock 16 24\nput crate 40 16, 96 24\n");
        let twice = doubled(text, &[3, 3], [16.0, 16.0]);
        assert!(twice.ends_with("put crate 40 16, 80 16\nput beach_rock 24 24\n"), "{twice}");
        let turned = each_pose(text, &[3, 4], |pose| Some(Pose { turned: true, ..pose }));
        assert!(turned.contains("put beach_rock 8 8 flip") && turned.contains("put crate 40 16, 80 16 flip"));
        assert_eq!(each_pose(text, &[4, 3], |_| None), "decor: a 256 192\nground town_grass\n");
    }

    #[test]
    fn an_element_laid_is_read_and_made_again() {
        let pose = Pose::read("  put beach_rock 440 224, 120 64 flip shadow off layer over").expect("laid");
        assert_eq!((pose.name.as_str(), pose.points.clone(), pose.turned, pose.shadowless), ("beach_rock", vec![[440.0, 224.0], [120.0, 64.0]], true, true));
        assert_eq!(pose.layer.as_deref(), Some("over"));
        assert_eq!(pose.line(), "  put beach_rock 440 224, 120 64 flip shadow off layer over");
        assert_eq!(pose.moved(1, [128.0, 72.0]).line(), "  put beach_rock 440 224, 128 72 flip shadow off layer over");
        assert_eq!(Pose::read("water pond 16 32").expect("water").line(), "water pond 16 32");
        assert_eq!(Pose::read("ground beach_sand"), None);
        assert_eq!(Pose::read("put rien"), None);
    }

    #[test]
    fn a_decors_zones_and_paths_are_read_and_a_corner_moved() {
        let doc = "ground fourre zone 0 0 640 192\npath dirt from 280 480 to 280 368 width 32\nscatter cailloux 3 zone 64 160 520 240 flip\n";
        let found: Vec<(usize, String, [[f32; 2]; 2])> = zones(doc).into_iter().map(|zone| (zone.line, zone.word, zone.corners)).collect();
        assert_eq!(found, [(1, "ground".to_string(), [[0.0, 0.0], [640.0, 192.0]]), (3, "scatter".to_string(), [[64.0, 160.0], [520.0, 240.0]])]);
        assert_eq!(paths(doc), [(2, "dirt".to_string(), vec![[280.0, 480.0], [280.0, 368.0]])]);
        assert_eq!(
            zone_moved("  scatter cailloux 3 zone 64 160 520 240 flip", 1, [512.0, 248.0]).as_deref(),
            Some("  scatter cailloux 3 zone 64 160 512 248 flip")
        );
        assert_eq!(zone_moved("ground sable", 0, [0.0, 0.0]), None);
    }

    #[test]
    fn a_decor_lays_its_elements_in_order_and_has_entrances() {
        let doc = "ground sable\nput a 0 64\nput b 8 8, 16 16\n# put c 1 1\nentrance 568 200 right\n";
        let laid: Vec<(usize, usize, String)> = laid_in(doc).into_iter().map(|one| (one.line, one.at, one.name)).collect();
        assert_eq!(laid, [(2, 0, "a".to_string()), (3, 0, "b".to_string()), (3, 1, "b".to_string())]);
        assert_eq!(entrances(doc), [([568.0, 200.0], "right".to_string())]);
    }

    #[test]
    fn the_workshop_reads_the_projects_decors_as_they_are_built() {
        // (Every decor of the project that builds: each element the Workshop
        // reads is the one laid, turned, shadowed, layered, named, locked
        // alike; the entrances and the groups the same.)
        // (The decors of the tests' games: the library's are the user's own.)
        let games = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/games");
        let folders = std::fs::read_dir(games).expect("the tests' games").flatten().map(|game| game.path().join("decors"));
        let mut seen = 0;
        for file in folders.flat_map(|folder| std::fs::read_dir(folder).into_iter().flatten().flatten()) {
            let path = file.path();
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Ok(built) = crate::lang::parse(&text, &path.display().to_string()) else { continue };
            seen += 1;
            let lines: Vec<&str> = text.lines().collect();
            let poses: Vec<(usize, Pose)> = numbered(&text)
                .filter_map(|(number, line)| Some((number, Pose::read(line)?)))
                .filter(|(_, pose)| !pose.hidden && pose.word != "resident")
                .collect();
            let laid: Vec<&crate::lang::Line> = built.lines.iter().filter(|line| line.verb == "put" || line.verb == "water").collect();
            assert_eq!(
                poses.iter().map(|(number, _)| *number).collect::<Vec<_>>(),
                laid.iter().map(|line| line.number).collect::<Vec<_>>(),
                "{}",
                path.display()
            );
            for ((number, pose), line) in poses.iter().zip(laid) {
                let said = (
                    pose.turned,
                    !pose.shadowless,
                    pose.layer.as_deref().and_then(crate::lang::layer_of).flatten().map(str::to_string),
                    pose.spot.clone(),
                    pose.locked,
                );
                assert_eq!(
                    said,
                    (line.mirrored, line.shadow, line.layer.clone(), line.name.clone(), line.locked),
                    "{} : {}",
                    path.display(),
                    lines[number - 1]
                );
                assert_eq!(Some(pose.name.as_str()), line.words.first().map(String::as_str), "{} : {}", path.display(), lines[number - 1]);
            }
            let entrances: Vec<(i64, i64)> = entrances(&text).iter().map(|([x, y], _)| (*x as i64, *y as i64)).collect();
            let theirs: Vec<(i64, i64)> = built
                .entrances
                .iter()
                .filter(|(line, ..)| lines.get(line.wrapping_sub(1)).is_some_and(|said| said.trim_start().starts_with("entrance")))
                .map(|(_, x, y, _)| (*x, *y))
                .collect();
            assert_eq!(entrances, theirs, "{}", path.display());
            assert_eq!(groups(&text), built.groups, "{}", path.display());
        }
        assert!(seen > 0, "no decor read");
    }
}
