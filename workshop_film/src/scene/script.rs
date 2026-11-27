//! The film's text read and written again for the Scene's handles: someone
//! moved to a point, the view framed, someone facing a way, a walk's point,
//! the spots it names. Each edit changes only the words it must, the rest of
//! the line as it was.

/// The film's text with someone put at that point, at the line playing:
/// the last line of the scene up to it that puts them at a point (`put`,
/// `… walk to x y`…) written again; none, a `put` after the `scene:`
/// line. Its line, or why not.
pub(super) fn moved_to(text: &str, playing: usize, name: &str, [x, y]: [f32; 2]) -> Result<(String, usize), String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let upto = playing.clamp(1, lines.len().max(1));
    let scene = (1..=upto).rev().find(|number| film::lines::scene_head(&lines[number - 1]).is_some()).unwrap_or(1);
    const MOVERS: [&str; 6] = ["put", "walk", "run", "enter", "appear", "go"];
    for number in (scene..=upto).rev() {
        let line = &lines[number - 1];
        let words: Vec<&str> = line.split_whitespace().collect();
        // (Who first, `riolu walk to 5 6`.)
        if words.len() < 2 || words[0] != name || !MOVERS.contains(&words[1]) {
            continue;
        }
        let pair = (2..words.len().saturating_sub(1)).rev().find(|at| words[*at].parse::<f32>().is_ok() && words[at + 1].parse::<f32>().is_ok());
        let Some(pair) = pair else {
            return Err(format!("line {number} puts them elsewhere than on a point: \"{}\"", line.trim()));
        };
        let lead: String = line.chars().take_while(|letter| letter.is_whitespace()).collect();
        let mut made: Vec<String> = words.iter().map(|word| word.to_string()).collect();
        (made[pair], made[pair + 1]) = (format!("{x}"), format!("{y}"));
        lines[number - 1] = format!("{lead}{}", made.join(" "));
        return Ok((lines.join("\n"), number));
    }
    let lead: String = lines.get(scene).map(|next| next.chars().take_while(|letter| letter.is_whitespace()).collect()).unwrap_or_default();
    lines.insert(scene, format!("{lead}{name} put {x} {y}"));
    Ok((lines.join("\n"), scene + 1))
}

/// Each word of a line: where it is (in bytes), the word.
fn words_of(line: &str) -> Vec<(std::ops::Range<usize>, &str)> {
    let mut words = Vec::new();
    let mut start = None;
    for (at, letter) in line.char_indices().chain(std::iter::once((line.len(), ' '))) {
        match (letter.is_whitespace(), start) {
            (true, Some(from)) => {
                words.push((from..at, &line[from..at]));
                start = None;
            }
            (false, None) => start = Some(at),
            _ => {}
        }
    }
    words
}

/// A number written as the film writes it: whole if it is, else at most two decimals.
pub(super) fn number(value: f32) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 { format!("{}", rounded as i64) } else { format!("{rounded}") }
}

/// The text with the words `from..from + count` of that line (from 1) made
/// `made` (`count` 0: put before that word), the rest of the line as it was.
pub(super) fn with_words(text: &str, line: usize, from: usize, count: usize, made: &str) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let Some(written) = lines.get(line.wrapping_sub(1)) else { return text.to_string() };
    let words = words_of(written);
    let (Some(first), Some(last)) = (words.get(from), words.get(from + count.max(1) - 1)) else { return text.to_string() };
    let range = if count == 0 { first.0.start..first.0.start } else { first.0.start..last.0.end };
    let tail = if count == 0 { " " } else { "" };
    lines[line - 1] = format!("{}{made}{tail}{}", &written[..range.start], &written[range.end..]);
    lines.join("\n")
}

/// The text with a line added after that one (from 1; 0: at the top), as far in as the next.
fn inserted(text: &str, after: usize, made: &str) -> String {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let after = after.min(lines.len());
    let lead: String = lines
        .get(after)
        .filter(|next| !next.trim().is_empty())
        .or(after.checked_sub(1).and_then(|at| lines.get(at)))
        .map(|next| next.chars().take_while(|letter| letter.is_whitespace()).collect())
        .unwrap_or_default();
    lines.insert(after, format!("{lead}{made}"));
    lines.join("\n")
}

/// Whether a line opens a scene or a moment.
fn opens(line: &str) -> bool {
    let line = line.trim_start();
    film::lines::scene_head(line).is_some() || line.starts_with("moment:")
}

/// The moment of that line (from 1): its first line (its `moment:` or
/// `scene:` line, or 1), the line after its last.
pub(super) fn moment_of(text: &str, line: usize) -> (usize, usize) {
    let lines: Vec<&str> = text.split('\n').collect();
    let line = line.clamp(1, lines.len().max(1));
    let start = (1..=line).rev().find(|number| opens(lines[number - 1])).unwrap_or(1);
    let end = (line + 1..=lines.len()).find(|number| opens(lines[number - 1])).unwrap_or(lines.len() + 1);
    (start, end)
}

/// Whether a line frames the view: `frame …` (not `frame free`), `glide …`.
fn frames(words: &[&str]) -> bool {
    matches!(words, ["frame", second, ..] if *second != "free") || matches!(words, ["glide", _, ..])
}

/// The film's text with the view framed on that point (and that close, if
/// asked), from the line playing: the last framing line of its moment up to
/// it (`frame …`, `glide [to] …`) written again with the point, else a
/// `frame x y snap` after the moment's first line; the closeness on its last
/// `zoom` line up to it, else a `zoom n 1f` after the framing line. Its
/// framing line.
pub(super) fn framed_at(text: &str, playing: usize, [x, y]: [f32; 2], close: Option<f32>) -> (String, usize) {
    let lines: Vec<&str> = text.split('\n').collect();
    let (start, _) = moment_of(text, playing);
    let upto = playing.clamp(start, lines.len().max(1));
    let point = format!("{} {}", number(x), number(y));
    let framing = (start..=upto).rev().find(|number| frames(&lines[number - 1].split_whitespace().collect::<Vec<_>>()));
    let (mut made, line) = match framing {
        Some(line) => {
            let words: Vec<&str> = lines[line - 1].split_whitespace().collect();
            let made = if words[0] == "frame" {
                // (Who it framed made the point; `snap`, `max:` kept; `follows` no longer: a point stays.)
                let kept: Vec<&str> = words[1..].iter().copied().filter(|word| *word == "snap" || word.starts_with("max:")).collect();
                let rest = std::iter::once(point.as_str()).chain(kept).collect::<Vec<_>>().join(" ");
                with_words(text, line, 1, words.len() - 1, &rest)
            } else {
                // (`glide to where [in d]`, `glide where d`: where is a point (two numbers) or a word.)
                let from = if words.get(1) == Some(&"to") { 2 } else { 1 };
                let numeric = |at: usize| words.get(at).is_some_and(|word| word.parse::<f32>().is_ok());
                with_words(text, line, from, if numeric(from) && numeric(from + 1) { 2 } else { 1 }, &point)
            };
            (made, line)
        }
        None => (inserted(text, start, &format!("frame {point} snap")), start + 1),
    };
    if let Some(close) = close {
        let lines: Vec<&str> = made.split('\n').collect();
        let upto = upto.max(line).min(lines.len());
        let zoom = (start..=upto).rev().find(|number| lines[number - 1].split_whitespace().next() == Some("zoom"));
        made = match zoom {
            Some(zoom) => with_words(&made, zoom, 1, 1, &number(close)),
            None => inserted(&made, line, &format!("zoom {} 1f", number(close))),
        };
    }
    (made, line)
}

/// The eight ways one faces, as the film names them, from the right going round (the screen's y downwards).
pub(super) const WAYS: [&str; 8] = ["right", "downright", "down", "downleft", "left", "upleft", "up", "upright"];

/// The way from a point to another on the screen (y downwards), the nearest of the eight.
pub(super) fn way_of(dx: f32, dy: f32) -> &'static str {
    let turn = dy.atan2(dx) / std::f32::consts::FRAC_PI_4;
    WAYS[(turn.round() as i32).rem_euclid(8) as usize]
}

/// The film's text with someone facing that way, at the line playing: their
/// last line of the scene up to it, if it sets their way (`… facing way`,
/// `look way`) written again, or if it puts them (`put …`, with `facing way`
/// added); else `who look way` after the line playing. Its line.
pub(super) fn faced(text: &str, playing: usize, name: &str, way: &str) -> (String, usize) {
    let lines: Vec<&str> = text.split('\n').collect();
    let upto = playing.clamp(1, lines.len().max(1));
    let scene = (1..=upto).rev().find(|number| film::lines::scene_head(lines[number - 1]).is_some()).unwrap_or(1);
    let theirs = (scene..=upto).rev().find(|number| lines[number - 1].split_whitespace().next() == Some(name));
    if let Some(line) = theirs {
        let words: Vec<&str> = lines[line - 1].split_whitespace().collect();
        if let Some(at) = words.iter().position(|word| *word == "facing").filter(|at| *at + 1 < words.len()) {
            return (with_words(text, line, at + 1, 1, way), line);
        }
        if words.get(1) == Some(&"look") && words.get(2).is_some_and(|word| WAYS.contains(word)) {
            return (with_words(text, line, 2, 1, way), line);
        }
        if words.get(1) == Some(&"put") {
            let end = lines[line - 1].trim_end().len();
            let mut made: Vec<String> = text.split('\n').map(str::to_string).collect();
            made[line - 1] = format!("{} facing {way}", &lines[line - 1][..end]);
            return (made.join("\n"), line);
        }
    }
    (inserted(text, upto, &format!("{name} look {way}")), upto + 1)
}

/// A move of someone's in the moment: its line, the points of its way in
/// the order they are walked (each its first word, from 0, and its point;
/// the last is where it goes), whether they run.
#[derive(Debug, PartialEq)]
pub(super) struct Move {
    pub line: usize,
    pub points: Vec<(usize, [f32; 2])>,
    pub runs: bool,
}

/// The points written in a line's words from that one on: each pair of numbers, its first word.
fn pairs(words: &[&str], from: usize) -> Vec<(usize, [f32; 2])> {
    let mut found = Vec::new();
    let mut at = from;
    while at + 1 < words.len() {
        if let (Ok(x), Ok(y)) = (words[at].parse::<f32>(), words[at + 1].parse::<f32>()) {
            found.push((at, [x, y]));
            at += 2;
        } else {
            at += 1;
        }
    }
    found
}

/// Someone's moves in the moment of the line playing (`who walk|run [to] x y
/// [via x y …]`, `who walk via …`): those going by points; and where they
/// stood before it, if a line of the scene put them on a point.
pub(super) fn moves_of(text: &str, playing: usize, name: &str) -> (Option<[f32; 2]>, Vec<Move>) {
    let lines: Vec<&str> = text.split('\n').collect();
    let (start, end) = moment_of(text, playing);
    let scene = (1..=start.min(lines.len())).rev().find(|number| film::lines::scene_head(lines[number - 1]).is_some()).unwrap_or(1);
    const MOVERS: [&str; 6] = ["put", "walk", "run", "enter", "appear", "go"];
    let before = (scene..start).rev().find_map(|number| {
        let words: Vec<&str> = lines[number - 1].split_whitespace().collect();
        (words.len() > 2 && words[0] == name && MOVERS.contains(&words[1])).then(|| pairs(&words, 2).last().map(|(_, point)| *point)).flatten()
    });
    let moves = (start..end.min(lines.len() + 1))
        .filter_map(|number| {
            let words: Vec<&str> = lines[number - 1].split_whitespace().collect();
            if words.len() < 3 || words[0] != name || !["walk", "run"].contains(&words[1]) {
                return None;
            }
            // (`via` first, then where it goes.)
            let via = words.iter().position(|word| *word == "via");
            let (mut through, mut to): (Vec<_>, Vec<_>) = pairs(&words, 2).into_iter().partition(|(at, _)| via.is_some_and(|via| *at > via));
            through.append(&mut to);
            (!through.is_empty()).then_some(Move { line: number, points: through, runs: words[1] == "run" })
        })
        .collect();
    (before, moves)
}

/// The film's text with the point at that word of that line (from 0) made this one.
pub(super) fn with_point(text: &str, line: usize, at: usize, [x, y]: [f32; 2]) -> String {
    with_words(text, line, at, 2, &format!("{} {}", number(x), number(y)))
}

/// The spots the film names (`spot name x y`): their name, their line, their point.
pub(super) fn spot_lines(text: &str) -> Vec<(String, usize, [f32; 2])> {
    text.split('\n')
        .enumerate()
        .filter_map(|(at, line)| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                ["spot", name, x, y, ..] => Some((name.to_string(), at + 1, [x.parse().ok()?, y.parse().ok()?])),
                _ => None,
            }
        })
        .collect()
}

/// A `scene:` line with one more of its cast, after the others (a remark after `#` kept at the end).
pub(super) fn with_cast(scene: &str, word: &str) -> String {
    match scene.split_once('#') {
        Some((said, remark)) => format!("{} {word} #{remark}", said.trim_end()),
        None => format!("{} {word}", scene.trim_end()),
    }
}

/// The film's text with a Pokémon dropped at a point, at the line playing:
/// one of the scene's cast if they were not (its `scene:` line), then their
/// last line of the scene up to it that puts them at a point written again
/// (as `moved_to`), else `who put x y [facing other]` after the line playing.
/// Its line, or why not.
pub(super) fn dropped_in(text: &str, playing: usize, word: &str, [x, y]: [f32; 2], facing: Option<&str>) -> Result<(String, usize), String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let upto = playing.clamp(1, lines.len().max(1));
    let Some(scene) = workshop_script::scene_of(&lines, upto) else {
        return Err("no scene here: a scene's line first (drop a place on the Script)".to_string());
    };
    let mut made: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    if workshop_script::cast_of(lines[scene - 1]).iter().any(|one| one == word) {
        return moved_to(text, playing, word, [x, y]);
    }
    made[scene - 1] = with_cast(lines[scene - 1], word);
    let facing = facing.map(|other| format!(" facing {other}")).unwrap_or_default();
    Ok(workshop_script::lines_after(&made.join("\n"), upto, &[format!("{word} put {} {}{facing}", number(x), number(y))]))
}

/// Where someone dropped at a point stands: on free ground (the nearest, as
/// the language puts them), not on anyone else (the nearest spot round it
/// that is free), in whole pixels.
pub(super) fn free_spot(ground: Option<&engine::Ground>, others: &[[f32; 2]], [x, y]: [f32; 2]) -> [f32; 2] {
    const AROUND: [(f32, f32); 9] = [(0.0, 0.0), (-36.0, 0.0), (36.0, 0.0), (0.0, 22.0), (0.0, -22.0), (-72.0, 0.0), (72.0, 0.0), (-36.0, 22.0), (36.0, 22.0)];
    let on_ground = |at: glam::Vec2| ground.map_or(at, |ground| ground.free(at));
    let at = glam::Vec2::new(x, y);
    let taken = |spot: glam::Vec2| others.iter().any(|[ox, oy]| (spot.x - ox).abs() < 24.0 && (spot.y - oy).abs() < 14.0);
    let spot = AROUND.iter().map(|(dx, dy)| on_ground(at + glam::Vec2::new(*dx, *dy))).find(|spot| !taken(*spot)).unwrap_or_else(|| on_ground(at));
    [spot.x.round(), spot.y.round()]
}

/// The nearest of the others to a point (whom someone put there faces).
pub(super) fn nearest<'a>(feet: impl Iterator<Item = (&'a String, &'a [f32; 2])>, [x, y]: [f32; 2], but: &str) -> Option<&'a String> {
    feet.filter(|(name, _)| *name != but)
        .min_by(|(_, a), (_, b)| ((a[0] - x).powi(2) + (a[1] - y).powi(2)).total_cmp(&((b[0] - x).powi(2) + (b[1] - y).powi(2))))
        .map(|(name, _)| name)
}
