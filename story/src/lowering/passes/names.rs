//! Names given to characters, places and groups, put in.

use super::*;

/// The groups a file names (`group <name> <a,b,c>`), put back where they
/// are used: in the list of an `each`, and as a field of its own (`waitfor
/// team`, `frame 30 team`: each of them in its place). Never in what
/// is said, nor in a line the host reads, whose words are its own (`fight
/// ... side_team pikachu ...`: `side_team` there is no crowd).
pub(super) fn groups(lines: Lines, vocabulary: &Vocabulary) -> Lines {
    let mut named: HashMap<String, Vec<String>> = HashMap::new();
    let mut out = Vec::with_capacity(lines.len());
    for (number, fields) in lines {
        if let ("group", Some(name), Some(members)) = (fields[0].as_str(), fields.get(1), fields.get(2)) {
            // (A group of groups too.)
            let members = members.split(',').flat_map(|member| named.get(member).cloned().unwrap_or_else(|| vec![member.to_string()])).collect();
            named.insert(name.clone(), members);
            continue;
        }
        if named.is_empty() {
            out.push((number, fields));
            continue;
        }
        if vocabulary.host_line(&fields[0]) {
            out.push((number, fields));
            continue;
        }
        let each = fields[0] == "each";
        let said = said(&fields);
        let mut expanded = vec![fields[0].clone()];
        for (place, field) in fields.into_iter().enumerate().skip(1) {
            match named.get(&field) {
                _ if Some(place) == said => expanded.push(field),
                _ if each && place == 1 => {
                    let list = field.split(',').flat_map(|item| named.get(item).cloned().unwrap_or_else(|| vec![item.to_string()]));
                    expanded.push(list.collect::<Vec<String>>().join(","));
                }
                Some(members) if !each => expanded.extend(members.iter().cloned()),
                _ => expanded.push(field),
            }
        }
        out.push((number, expanded));
    }
    out
}

/// The field holding words in a line that says something.
fn said(fields: &[String]) -> Option<usize> {
    let at = match fields[0].as_str() {
        "say" | "bubble" | "think" | "shout" => 3,
        "caption" => 2,
        "ask" | "banner" | "hud" => 1,
        _ => return None,
    };
    // (`hud <place> <words>`: the words one further.)
    let placed = fields[0] == "hud" && fields.get(1).is_some_and(|place| engine::hud_place(place).is_some());
    Some(if placed { at + 1 } else { at })
}

/// The names a file gives: `cast <name> <character>` (then `put pikachu
/// ...`), `spot <name> <x> <y>` (then `go pikachu @fountain 40`), put back
/// where they are used. Words said are left as they are.
pub(super) fn names(lines: Lines, problems: &mut Vec<Diagnostic>, vocabulary: &Vocabulary) -> Lines {
    let (mut cast, mut spots): (HashMap<String, String>, HashMap<String, (String, String)>) = (HashMap::new(), HashMap::new());
    // (The names given and where, `@` before a place's; those used.)
    let (mut given, mut used): (HashMap<String, usize>, HashSet<String>) = (HashMap::new(), HashSet::new());
    // (Words kept by a name: what they are compared with is a word, never
    // someone, `if answer == raichu`.)
    let mut texts: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(lines.len());
    for (number, fields) in lines {
        if fields[0] == "settext"
            && let Some(name) = fields.get(1)
        {
            texts.insert(name.clone());
        }
        let test = matches!(fields[0].as_str(), "if" | "elif" | "while" | "waituntil");
        let compared = |place: usize| {
            test && place >= 3 && texts.contains(&fields[place - 2]) && matches!(fields[place - 1].as_str(), "==" | "=" | "!=" | "<" | "<=" | ">" | ">=")
        };
        let compared: Vec<bool> = (0..fields.len()).map(compared).collect();
        match (fields[0].as_str(), fields.get(1), fields.get(2), fields.get(3)) {
            ("cast", Some(name), Some(code), _) => {
                // (Two names for one character: they would be one on the
                // stage, `m74` and `m74` again; `m74.1` is another.)
                if let Some((other, _)) = cast.iter().find(|(other, same)| *other != name && *same == code) {
                    problems.push(Diagnostic::at(number, format!("“{name}” and “{other}” are the same character ({code}): for a second one, write {code}.1")));
                }
                cast.insert(name.clone(), code.clone());
                // (Put in already by the film: used, said or not.)
                match fields.get(3).map(String::as_str) {
                    Some("used") => _ = used.insert(name.clone()),
                    _ => _ = given.insert(name.clone(), number),
                }
                continue;
            }
            ("spot", Some(name), Some(x), Some(y)) => {
                spots.insert(name.clone(), (x.clone(), y.clone()));
                given.insert(format!("@{name}"), number);
                continue;
            }
            _ => {}
        }
        for field in &fields[1..] {
            let name = field.split_once(':').map_or(field.as_str(), |(name, _)| name);
            // (A place used with how far beside it: its name alone.)
            let name = match name.strip_prefix('@') {
                Some(rest) => &name[..1 + rest.find(['+', '-']).unwrap_or(rest.len())],
                None => name,
            };
            used.insert(name.to_string());
        }
        let said = said(&fields);
        let mut expanded = Vec::with_capacity(fields.len() + 1);
        for (place, field) in fields.into_iter().enumerate() {
            // (A song or a sound by its name, though someone has that name
            // too: `music jolly_master`.)
            let heard = matches!(expanded.last().map(String::as_str), Some("music" | "ambient" | "sound"))
                && (vocabulary.song(&field).is_some() || vocabulary.sound(&field).is_some());
            if place == 0 || heard || compared[place] {
                expanded.push(field);
            } else if Some(place) == said {
                expanded.push(names_in(field, &cast, &mut used));
            } else if let Some(code) = cast.get(&field) {
                expanded.push(code.clone());
            } else if let Some((x, y)) = field.strip_prefix('@').and_then(|name| spots.get(name)) {
                expanded.extend([x.clone(), y.clone()]);
            // (`@name+dx-dy`: so far beside the place.)
            } else if let Some((x, y)) = field.strip_prefix('@').and_then(|near| beside(near, &spots)) {
                expanded.extend([x.to_string(), y.to_string()]);
            } else if let Some(name) = field.strip_prefix('@').filter(|name| !name.is_empty()) {
                problems.push(Diagnostic::at(number, format!("unknown spot: “@{name}” (it takes a “spot {name} <x> <y>” before)")));
                expanded.push(field);
            // (`<name>:<shown>`: a speaker by its name, another name shown.)
            } else if let Some(code) = field.split_once(':').and_then(|(name, _)| cast.get(name)) {
                let shown = field.split_once(':').map_or("", |(_, shown)| shown);
                // (A name of someone else's shown: `false:{name:partner}`.)
                let shown = names_in(shown.to_string(), &cast, &mut used);
                expanded.push(format!("{code}:{shown}"));
            } else {
                expanded.push(field);
            }
        }
        out.push((number, expanded));
    }
    let mut unused: Vec<(usize, String)> = given.into_iter().filter(|(name, _)| !used.contains(name)).map(|(name, number)| (number, name)).collect();
    unused.sort();
    for (number, name) in unused {
        problems.push(Diagnostic::at(number, format!("“{}” is named but never used", name.trim_start_matches('@'))));
    }
    out
}

/// A page of a tirade and the face it changes to, if it starts with one
/// (`[angry]Enough!`, or a face's number: `[3]Enough!`).
pub(super) fn page_face(page: &str, vocabulary: &Vocabulary) -> (Option<String>, String) {
    let trimmed = page.trim_start();
    let face =
        trimmed.strip_prefix('[').and_then(|rest| rest.split_once(']')).filter(|(face, _)| vocabulary.face(face).is_some() || face.parse::<u16>().is_ok());
    match face {
        Some((face, words)) => (Some(face.to_string()), format!(" {}", words.trim_start())),
        None => (None, page.to_string()),
    }
}

/// Words naming someone the file gives a name (`{name:pika}`), as the game
/// reads them: by how it is written (`{name:m25}`), the game putting in its
/// name.
fn names_in(mut text: String, cast: &HashMap<String, String>, used: &mut HashSet<String>) -> String {
    for (name, code) in cast {
        let mark = format!("{{name:{name}}}");
        if text.contains(&mark) {
            used.insert(name.clone());
            text = text.replace(&mark, &format!("{{name:{code}}}"));
        }
    }
    text
}

/// A place given by its name and how far beside it (`fountain+30-10`: 30
/// to the right, 10 up), as two numbers.
fn beside(near: &str, spots: &HashMap<String, (String, String)>) -> Option<(f32, f32)> {
    let split = near.find(['+', '-'])?;
    let (name, offsets) = near.split_at(split);
    let (x, y) = spots.get(name)?;
    // (Two signed numbers, one after the other.)
    let second = offsets[1..].find(['+', '-'])? + 1;
    let (dx, dy) = offsets.split_at(second);
    let number = |text: &str| text.strip_prefix('+').unwrap_or(text).parse::<f32>().ok();
    Some((x.parse::<f32>().ok()? + number(dx)?, y.parse::<f32>().ok()? + number(dy)?))
}
