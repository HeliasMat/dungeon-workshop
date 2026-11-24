//! A fight stated in advance: its block of lines, each blow, thing and
//! moment read into the fields of one line.

use std::collections::BTreeSet;

use super::*;
use crate::check::nearby;
use crate::ir::{Instr, Shape};
use crate::names::things::thing_said;
use crate::text::told;
use crate::verbs::things::thing_apart;
use crate::words::frames_of;

/// A directed line's end, read from the last word: its words without it, how
/// it went, and the rest of what it says (as the fight's line has them).
fn options_read(words: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let (mut words, mut told, mut more) = (words.to_vec(), Vec::new(), Vec::new());
    while words.len() > 1 {
        let last = plain(&words[words.len() - 1]);
        let before = plain(&words[words.len() - 2]);
        let number_lost = words[words.len() - 1].strip_prefix('-').is_some_and(digits);
        let way = match last.as_str() {
            "misses" => Some("misses"),
            "crit" => Some("critical"),
            "ko" => Some("ko"),
            "protect" => Some("protect"),
            _ => None,
        };
        if way.is_some() || number_lost {
            told.insert(0, way.map_or(words[words.len() - 1].clone(), str::to_string));
            words.pop();
        } else if last == "slow" {
            more.push("slow".to_string());
            words.pop();
        } else if before == "slowmo" && words.len() > 2 && frames_of(&last).is_some() {
            more.extend(fields!["slowmo", frames_of(&last).unwrap_or(0)]);
            words.truncate(words.len() - 2);
        } else if before == "intercepts" && words.len() > 2 {
            more.extend(fields!["intercepts", last]);
            words.truncate(words.len() - 2);
        } else {
            break;
        }
    }
    (words, told, more)
}

/// A line of a directed fight: who, its move, on whom, how it went.
fn act_read(film: &mut Film, sides: &Sides, args: &crate::grammar::Bound, line: &str, number: usize, down: &mut BTreeSet<String>) -> Option<Vec<String>> {
    let said = args.word("who").unwrap_or_default();
    let who = plain(said);
    let fighters: Vec<String> = sides.iter().flatten().cloned().collect();
    if !fighters.contains(&who) {
        film.wrong(number, format!("“{said}” is not in this fight{}", nearby(said, fighters.iter().map(String::as_str))));
        return None;
    }
    let (mut words, told, more) = options_read(args.words("blow"));
    if told.iter().any(|word| word == "misses") && told.len() > 1 {
        let shown = |word: &str| if word == "critical" { "crit".to_string() } else { word.to_string() };
        let others: Vec<String> = told.iter().filter(|word| *word != "misses").map(|word| shown(word)).collect();
        film.wrong(number, format!("a blow that misses cannot also be {}: “{line}”", others.join(" or ")));
    }
    let mut targets = Vec::new();
    while words.len() > 1 && (plain(&words[words.len() - 1]) == "all" || film.actor(&words[words.len() - 1]).is_some()) {
        let target = plain(&words.pop().unwrap_or_default());
        targets.insert(0, target);
    }
    if words.is_empty() {
        film.wrong(number, format!("what move? “{line}”"));
        return None;
    }
    let mine = if sides[0].contains(&who) { &sides[0] } else { &sides[1] };
    if let Some(target) = targets.iter().find(|target| *target != "all" && (!fighters.contains(target) || mine.contains(target))) {
        film.wrong(number, format!("{who} cannot aim at {target}: not on the other side"));
        return None;
    }
    let chosen = film.move_said(&words, number)?;
    if let Ok(id) = chosen.parse::<u16>() {
        super::never_learnt(film, &who, id, number);
    }
    if down.contains(&who) {
        film.remark(number, format!("{who} is already K.O.: this line will not be played"));
    }
    let taker = more.iter().position(|word| word == "intercepts").map(|at| more[at + 1].clone());
    if let Some(taker) = taker.as_ref().filter(|taker| !fighters.contains(taker) || mine.contains(taker)) {
        film.wrong(number, format!("{taker} cannot intercept: not on the other side"));
        return None;
    }
    if told.iter().any(|word| word == "ko") {
        match taker {
            Some(taker) => _ = down.insert(taker),
            None => down.extend(targets.iter().filter(|target| *target != "all").cloned()),
        }
    }
    let mut act = fields!["act", who, chosen];
    act.extend(targets);
    act.extend(told);
    act.extend(more);
    Some(act)
}

/// The moves of a line, in several words each or not: the longest first, or
/// between `|`.
fn moves_said(film: &mut Film, args: &crate::grammar::Bound, number: usize) -> Vec<String> {
    let words = args.words("moves");
    let names: Vec<Vec<String>> = if words.iter().any(|word| word == "|") {
        args.parts("moves", '|')
    } else {
        let (mut names, mut left) = (Vec::new(), words.to_vec());
        while !left.is_empty() {
            let size = (1..=left.len()).rev().find(|size| film.tables.moves.contains_key(&plain(&left[..*size].concat()))).unwrap_or(1);
            names.push(left.drain(..size).collect());
        }
        names
    };
    names.iter().filter_map(|name| film.move_said(name, number)).collect()
}

/// `a uses Oran Berry [+30]`, `a eats Oran Berry`, `a throws Apple c [-20]`:
/// a thing used in a directed fight, by the fields of its line.
fn thing_act(film: &mut Film, args: &crate::grammar::Bound, number: usize) -> Option<Vec<String>> {
    let who = plain(args.word("who")?);
    let worth = args.word("value").map(|word| word.trim_start_matches(['+', '-']).to_string());
    if args.word("value").is_some_and(|word| !word.strip_prefix(['+', '-']).is_some_and(digits)) {
        film.wrong(number, format!("“{}”: what the thing gives back or takes, +30 or -20", args.word("value").unwrap_or_default()));
        return None;
    }
    let thrown = args.has("throws");
    let thing = thing_said(film, args.words("thing"), number)?;
    let code = thing_apart(film, &thing);
    let mut act = fields![if thrown { "throws" } else { "uses" }, who, code, format!("name:{}", thing.name), worth.unwrap_or("-".to_string())];
    act.extend(args.word("target").map(plain));
    Some(act)
}

/// The first form of a block's line that fits it: its values; none, the
/// reasons told.
fn block_line(film: &mut Film, head: &str, words: &[String], number: usize, line: &str) -> Option<crate::grammar::Bound> {
    let forms = FIGHT_LINES.iter().find(|(word, _)| *word == head).map(|(_, forms)| *forms)?;
    let mut why = Vec::new();
    for form in forms {
        match crate::grammar::bind(film, &crate::grammar::read(form), words) {
            Ok(bound) => return Some(bound),
            Err(reason) => why.push(format!("“{form}”: {reason}")),
        }
    }
    film.wrong(number, format!("“{line}”: {}", why.join("; ")));
    None
}

/// What a directed fight's block says, as fields of its line; the winner.
fn directed_block(film: &mut Film, sides: &mut Sides, lines: &[Instr]) -> (Vec<String>, Option<String>) {
    let (mut fields, mut winner, mut down, mut acts, mut slowing) = (Vec::new(), None, BTreeSet::new(), Vec::new(), Vec::new());
    for item in lines {
        let (number, line) = (item.number, item.text.as_str());
        // (A word as it was read; another line by its text.)
        let words: Vec<String> = match &item.shape {
            Shape::Word { verb, words, wait: true } => std::iter::once(verb.clone()).chain(words.iter().cloned()).collect(),
            _ => line.split_whitespace().map(str::to_string).collect(),
        };
        let head = plain(&words[0]);
        let fighters: Vec<String> = sides.iter().flatten().cloned().collect();
        let in_fight = |film: &mut Film, word: &str| {
            let known = fighters.contains(&plain(word));
            if !known {
                film.wrong(number, format!("“{word}” is not in this fight{}", nearby(word, fighters.iter().map(String::as_str))));
            }
            known
        };
        let speaker = match &item.shape {
            Shape::Said { who, .. } => Some(who.as_str()).filter(|name| told(name) || film.actor(name).is_some()),
            _ => None,
        };
        if speaker.is_some() {
            let label = format!("fight{}_said{}", film.combat.written + 1, film.combat.asides.len());
            film.combat.asides.push((label.clone(), number, item.clone()));
            acts.extend(fields!["scene", label]);
            continue;
        }
        if let Shape::WhenLife { who, left, lines } = &item.shape {
            film.combat.moments.push((who.clone(), *left, lines.clone()));
            continue;
        }
        // (A line of its own word, or one that begins with a fighter: read by its form.)
        let known = FIGHT_LINES.iter().any(|(word, _)| *word == head);
        let (form, words) = if known { (head.as_str(), &words[1..]) } else { ("*", &words[..]) };
        let Some(args) = block_line(film, form, words, number, line) else {
            continue;
        };
        let who = plain(args.word("who").unwrap_or_default());
        let other = plain(args.word("other").unwrap_or_default());
        match form {
            "hud" if in_fight(film, &who) && in_fight(film, &other) => fields.extend(fields!["great_bars", who, other]),
            "wait" => acts.extend(fields!["pause", args.frames("d").unwrap_or(0)]),
            "log" => acts.extend(fields!["note", args.words("text").join(" ")]),
            "slowmo" => slowing = fields!["slowmo", args.frames("d").unwrap_or(0)],
            "change" if in_fight(film, &who) => {
                if !fighters.contains(&other) {
                    let side = usize::from(!sides[0].contains(&who));
                    sides[side].push(other.clone());
                    fields.extend(fields!["reserve", other]);
                }
                acts.extend(fields!["change", who, other]);
            }
            "random" => {
                fields.push("rest".to_string());
                winner = args.word("wins").map(plain).or(winner);
            }
            "winner" => winner = args.word("wins").map(plain),
            "hp" if in_fight(film, &who) => fields.extend(fields!["hp", who, args.word("n").unwrap_or_default()]),
            "level" if in_fight(film, &who) => fields.extend(fields!["lv", who, args.word("n").unwrap_or_default()]),
            "knows" if in_fight(film, &who) => {
                fields.extend(fields!["knows", who]);
                fields.extend(moves_said(film, &args, number));
            }
            "number" => fields.extend(fields!["turns", args.word("n").unwrap_or_default()]),
            "*" if args.has("thing") => {
                if let Some(act) = thing_act(film, &args, number) {
                    acts.extend(act);
                }
            }
            "*" => {
                if let Some(act) = act_read(film, sides, &args, line, number, &mut down) {
                    acts.extend(act);
                    acts.append(&mut slowing);
                }
            }
            _ => {}
        }
    }
    let fighters: Vec<String> = sides.iter().flatten().cloned().collect();
    if let Some(lost) = winner.as_ref().filter(|winner| !fighters.contains(winner)) {
        let number = lines.first().map_or(0, |item| item.number);
        film.wrong(number, format!("the winner “{lost}” is not in this fight{}", nearby(lost, fighters.iter().map(String::as_str))));
        winner = None;
    }
    fields.extend(acts);
    (fields, winner)
}

pub(super) fn directed(film: &mut Film, line: &Line) {
    // (`fight directed a b against c d [level 60] [winner a]`, then a line a
    // turn, up to `end`: `a Thunderbolt c crit`. All the rules of it are in
    // `scenes/src/fight`.)
    let args = &line.args;
    let winner = args.word("wins").map(plain);
    let level = args.number("n").map_or(30, |level| level as i32);
    let mut sides = [args.words("side_team").to_vec(), args.words("others").to_vec()];
    let (said, chosen) = directed_block(film, &mut sides, &line.block);
    fight_line(film, line.number, sides, Whole { sort: "directed", level, winner: winner.or(chosen), kept: false, more: said });
    closed(film);
}

/// The lines of a directed fight's block (`fight directed ... end`), by their
/// first word; `*`: a line that begins with a fighter (a thing it uses, or a
/// blow: its move and how it went, `Thunderbolt ekans -40 crit slowmo 30`).
pub const FIGHT_LINES: &[(&str, &[&str])] = &[
    ("hud", &["who:pokemon other:pokemon"]),
    ("wait", &["d:duration"]),
    ("log", &["text..."]),
    ("slowmo", &["d:duration"]),
    ("change", &["who:pokemon other:pokemon"]),
    ("random", &["[text...] [winner wins:word]"]),
    ("winner", &["wins:word"]),
    ("hp", &["who:pokemon n:number"]),
    ("level", &["who:pokemon n:number"]),
    ("knows", &["who:pokemon moves..."]),
    ("number", &["of turns n:number"]),
    ("*", &["who:pokemon uses|eats thing... [value:word]", "who:pokemon throws thing... target:pokemon [value:word]", "who:pokemon blow..."]),
];
