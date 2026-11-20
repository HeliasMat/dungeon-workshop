//! The film's text read and written again for the Scene: the spots it
//! names, someone moved to a point.

/// The places the film names (`spot name x y`), and their points.
pub(super) fn spots_of(text: &str) -> Vec<(String, [f32; 2])> {
    text.lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                ["spot", name, x, y, ..] => Some((name.to_string(), [x.parse().ok()?, y.parse().ok()?])),
                _ => None,
            }
        })
        .collect()
}

/// The film's text with someone put at that point, at the line playing:
/// the last line of the scene up to it that puts them at a point (`put`,
/// `… walk to x y`…) written again; none, a `put` after the `scene:`
/// line. Its line, or why not.
pub(super) fn moved_to(text: &str, playing: usize, name: &str, [x, y]: [f32; 2]) -> Result<(String, usize), String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let upto = playing.clamp(1, lines.len().max(1));
    let scene = (1..=upto).rev().find(|number| lines[number - 1].trim_start().starts_with("scene:")).unwrap_or(1);
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
