//! Sequences: lines defined once, played anywhere.

use super::*;

/// Sequences a file defines once and plays where it likes: `define <name>`
/// then its lines then `enddef`; `do <name> <a> <b>...` plays them with
/// `$1`, `$2`... standing for what it is given. A sequence may play
/// others (not itself, not too deep). The lines played keep the number of
/// the `do` line.
pub(super) fn sequences(lines: Lines, problems: &mut Vec<Diagnostic>) -> Lines {
    const DEEPEST: usize = 8;
    let mut defined: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut rest = Vec::new();
    let mut open: Option<(usize, String, Vec<Vec<String>>)> = None;
    for (number, fields) in lines {
        match (fields[0].as_str(), &mut open) {
            ("define", None) => open = Some((number, fields.get(1).cloned().unwrap_or_default(), Vec::new())),
            ("enddef", Some(_)) => {
                let (_, name, body) = open.take().expect("a sequence is open");
                defined.insert(name, body);
            }
            (_, Some((_, _, body))) => body.push(fields),
            _ => rest.push((number, fields)),
        }
    }
    if let Some((number, name, _)) = &open {
        problems.push(Diagnostic::at(number, format!("the sequence “{name}” has no “enddef”")));
    }
    // (The groups, for `each <group> do <sequence> $`: each one plays it.)
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for (_, fields) in &rest {
        if let ("group", Some(name), Some(members)) = (fields[0].as_str(), fields.get(1), fields.get(2)) {
            let members = members.split(',').flat_map(|member| groups.get(member).cloned().unwrap_or_else(|| vec![member.to_string()])).collect();
            groups.insert(name.clone(), members);
        }
    }
    type Defined = HashMap<String, Vec<Vec<String>>>;
    type Groups = HashMap<String, Vec<String>>;
    fn play(number: usize, fields: Vec<String>, (defined, groups): (&Defined, &Groups), depth: usize, out: &mut Lines, problems: &mut Vec<Diagnostic>) {
        if fields[0] == "each" && fields.get(2).is_some_and(|kind| kind == "do") && depth < DEEPEST {
            let members = fields[1].split(',').flat_map(|member| groups.get(member).cloned().unwrap_or_else(|| vec![member.to_string()]));
            for member in members.filter(|member| !member.is_empty()).collect::<Vec<String>>() {
                let line = fields[2..].iter().map(|field| if field == "$" { member.clone() } else { field.clone() }).collect();
                play(number, line, (defined, groups), depth + 1, out, problems);
            }
            return;
        }
        if fields[0] != "do" {
            out.push((number, fields));
            return;
        }
        let name = fields.get(1).cloned().unwrap_or_default();
        let Some(body) = defined.get(&name).filter(|_| depth < DEEPEST) else {
            problems.push(Diagnostic::at(number, format!("unknown sequence (or one that calls itself): “{name}”")));
            return;
        };
        // (The values it asks for without a default: as many given.)
        let asked = body
            .iter()
            .flatten()
            .filter_map(|field| field.strip_prefix('$'))
            .filter(|at| !at.contains('?'))
            .filter_map(|at| at.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        if fields.len().saturating_sub(2) < asked {
            problems.push(Diagnostic::at(number, format!("the sequence “{name}” takes {asked} value(s), {} given", fields.len().saturating_sub(2))));
        }
        // (Its own numbers, `%name`: for this call only.)
        let call = out.len();
        for line in body {
            // (`$2`: the second value given; `$2?down`: the same, `down` when
            // none is.)
            let given = |field: &String| {
                let Some(asked) = field.strip_prefix('$') else {
                    return field.clone();
                };
                let (at, default) = asked.split_once('?').unwrap_or((asked, ""));
                match at.parse::<usize>() {
                    Ok(at) => fields.get(at + 1).filter(|given| !given.is_empty()).cloned().unwrap_or_else(|| default.to_string()),
                    Err(_) => field.clone(),
                }
            };
            play(number, line.iter().map(given).map(|field| own_numbers(&field, call)).collect(), (defined, groups), depth + 1, out, problems);
        }
    }
    let mut out = Vec::new();
    for (number, fields) in rest {
        play(number, fields, (&defined, &groups), 0, &mut out, problems);
    }
    out
}

/// A sequence's own numbers in a field (`%name`, see `define`) as the
/// numbers of one call of it: `_name_<call>`, numbers of the event only
/// (see `Director::vars`). A `%` alone (`calc x a % b`) stays.
fn own_numbers(field: &str, call: usize) -> String {
    let mut out = String::with_capacity(field.len());
    let mut rest = field;
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let length = after.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(after.len());
        if length > 0 && after.starts_with(|c: char| c.is_alphabetic() || c == '_') {
            out.push_str(&format!("_{}_{call}", &after[..length]));
            rest = &after[length..];
        } else {
            out.push('%');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}
