//! The generic words of the nodes: any property of any node set by the
//! registry's name (`prop a angle 45 scale 1.5`), and nodes of the film's
//! own (`node moon image moon.png 40 30`, `node arm group`). What a
//! property is, its value as written, what is impossible: the registry
//! (`engine::props`), never a list here.

use engine::codes;
use engine::props::{self, NodeKind};
use engine::{NodeRef, Point, Sort};

use super::{Line, Names, Word};
use crate::emit;
use crate::film::Film;
use crate::words::plain;

pub const WORDS: &[Word] = &[
    Word::new(
        "prop",
        &["who:word rest:text..."],
        "prop: prop <who> <property> <value>…, a property of the registry or several on the line (prop a angle 45 scale 1.5 opacity 50%)",
    )
    .does(set_said),
    Word::new(
        "node",
        &["name:word kind:word [file:word] [x:coordinate y:coordinate]"],
        "node: node <name> group [x y], node <name> image <file> [x y], node <name> off; a node of one's own, set afterwards by prop",
    )
    .names(Names::NoOne)
    .does(node_said),
    Word::new(
        "attach",
        &["who:word parent:word [dx:number dy:number] [inherit flags...]"],
        "attach: attach <who> <parent> [dx dy] [inherit pos angle scale opacity colour]; it follows its parent (turned, grown, faded with it), at dx dy from its position (else where it is)",
    )
    .does(attach_said),
    Word::new("detach", &["who:word"], "detach: detach <who>; it stays where it is").does(detach_said),
];

/// The nodes a word names, each with its kind if known before it plays:
/// a node of the film's own, the caption, a halo by its rank, the emitter
/// on someone, someone (a crowd: each of it). None: no node by that name.
pub(super) fn nodes_named(film: &Film, word: &str) -> Option<Vec<(NodeRef, Option<NodeKind>)>> {
    if let Some(kind) = film.staging.nodes.get(word) {
        return Some(vec![(NodeRef::Named(word.to_string()), Some(*kind))]);
    }
    if word == "caption" {
        return Some(vec![(NodeRef::Caption, Some(NodeKind::Caption))]);
    }
    if let Some(rank) = word.strip_prefix("halo:") {
        return Some(vec![(NodeRef::Halo(rank.parse().ok()?), Some(NodeKind::Halo))]);
    }
    if let Some(who) = word.strip_prefix("emitter:") {
        return Some(vec![(NodeRef::Emitter(film.someone(who)?), Some(NodeKind::Emitter))]);
    }
    let all: Option<Vec<_>> = film
        .everyone(word)
        .iter()
        .map(|one| film.someone(one).map(|who| (NodeRef::Who(who), Some(if who.is_thing() { NodeKind::Thing } else { NodeKind::Body }))))
        .collect();
    all.filter(|all| !all.is_empty())
}

/// The name a word gives a node: a part of a rig as written
/// (`robot.arm`), else plain.
pub(super) fn node_word(film: &Film, word: &str) -> String {
    let lower = word.to_lowercase();
    if film.staging.nodes.contains_key(&lower) { lower } else { plain(word) }
}

/// No node by that name: told, with the names that are.
pub(super) fn no_node(film: &mut Film, number: usize, word: &str) {
    let known: Vec<String> = film.staging.nodes.keys().cloned().chain(film.actors.keys().cloned()).collect();
    let message = crate::check::unknown("unknown node", word, &crate::check::closest_of(word, known.iter().map(String::as_str)), "node <name> group");
    film.wrong_coded(number, codes::NODE_UNKNOWN, message);
}

/// `prop a angle 45 scale 1.5`: each property and its value, in turn,
/// for each node the word names.
fn set_said(film: &mut Film, line: &Line) {
    let who = node_word(film, line.args.word("who").unwrap_or_default());
    let words: Vec<String> = line.args.words("rest").to_vec();
    let Some(nodes) = nodes_named(film, &who) else {
        no_node(film, line.number, &who);
        return;
    };
    if words.is_empty() {
        film.wrong_coded(line.number, codes::VALUE_MISSING, format!("“{who}”: prop {who} <property> <value>…"));
        return;
    }
    let mut at = 0;
    while at < words.len() {
        let name = plain(&words[at]);
        let Some(prop) = props::find(&name) else {
            let wrong = props::unknown(&words[at]);
            film.wrong_coded(line.number, wrong.code, wrong.message);
            return;
        };
        let (given, used) = match props::read(prop, &words[at + 1..]) {
            Ok(read) => read,
            Err(wrong) => {
                film.wrong_coded(line.number, wrong.code, wrong.message);
                return;
            }
        };
        at += 1 + used;
        if let Some(Err(wrong)) = given.value().map(|value| prop.check(value)) {
            film.wrong_coded(line.number, wrong.code, wrong.message);
            continue;
        }
        let Some(given) = given.map(|other| nodes_named(film, &plain(&other)).and_then(|named| named.into_iter().next()).map(|(node, _)| node)) else {
            film.wrong_coded(line.number, codes::NODE_UNKNOWN, format!("“{}”: no node of that name for the point of {}", words[at - 1], prop.name));
            continue;
        };
        for (node, kind) in &nodes {
            if let Some(kind) = kind.filter(|kind| !prop.fits(*kind)) {
                let own: Vec<&str> = props::of_kind(kind).map(|prop| prop.name).collect();
                let message = format!("“{name}” is not a property of a {} node ({}): {}", kind.name(), node.name(), own.join(", "));
                film.wrong_coded(line.number, codes::PROP_KIND, message);
                continue;
            }
            film.beat(emit::set_prop(node.clone(), prop.id, given.clone()));
        }
    }
}

/// The one node a word names, told if none or more than one (a crowd).
fn one_node(film: &mut Film, number: usize, word: &str) -> Option<(NodeRef, Option<NodeKind>)> {
    match nodes_named(film, word) {
        Some(mut nodes) if nodes.len() == 1 => nodes.pop(),
        Some(_) => {
            film.wrong_coded(number, codes::ATTACH_KIND, format!("“{word}” is a crowd: one node is attached at a time"));
            None
        }
        None => {
            no_node(film, number, word);
            None
        }
    }
}

/// `attach pikachu raft [0 -8] [inherit pos angle]`: who hangs on whom,
/// where, taking what.
fn attach_said(film: &mut Film, line: &Line) {
    let (who, parent) = (node_word(film, line.args.word("who").unwrap_or_default()), node_word(film, line.args.word("parent").unwrap_or_default()));
    let Some((node, kind)) = one_node(film, line.number, &who) else { return };
    let Some((to, _)) = one_node(film, line.number, &parent) else { return };
    if matches!(kind, Some(NodeKind::Caption | NodeKind::Emitter)) {
        film.wrong_coded(line.number, codes::ATTACH_KIND, format!("“{who}” is not attached: the caption and an emitter stay where they are"));
        return;
    }
    if node == to || film.staging.parents.get(&parent).is_some_and(|chain| chain.contains(&who)) {
        film.wrong_coded(line.number, codes::ATTACH_LOOP, format!("“{who}” cannot be attached to “{parent}”: {parent} already hangs on {who}"));
        return;
    }
    let offset = match (line.args.number("dx"), line.args.number("dy")) {
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() => Some(glam::Vec2::new(x, y)),
        _ if !line.args.has("dx") => None,
        _ => {
            film.wrong_coded(line.number, codes::VALUE_TYPE, format!("an offset, dx dy in pixels (attach {who} {parent} 0 -8)"));
            return;
        }
    };
    let inherit = if line.args.has("inherit") {
        let words: Vec<String> = line.args.words("flags").iter().map(|word| plain(word)).collect();
        match engine::node::Inherit::of_words(&words) {
            Some(inherit) if !words.is_empty() => inherit,
            _ => {
                film.wrong_coded(line.number, codes::VALUE_TYPE, format!("inherit {}: among pos, angle, scale, opacity, colour", words.join(" ")));
                return;
            }
        }
    } else {
        engine::node::Inherit::default()
    };
    // (Who it hangs on, all the way up: for a loop to be seen before it plays.)
    let mut chain = film.staging.parents.get(&parent).cloned().unwrap_or_default();
    chain.push(parent.clone());
    film.staging.parents.insert(who, chain);
    film.beat(emit::attach(node, to, offset, inherit));
}

/// `detach pikachu`.
fn detach_said(film: &mut Film, line: &Line) {
    let who = node_word(film, line.args.word("who").unwrap_or_default());
    let Some((node, _)) = one_node(film, line.number, &who) else { return };
    film.staging.parents.remove(&who);
    film.beat(emit::detach(node));
}

/// `node moon image moon.png 40 30`, `node arm group`, `node moon off`.
fn node_said(film: &mut Film, line: &Line) {
    let name = plain(line.args.word("name").unwrap_or_default());
    let sort = plain(line.args.word("kind").unwrap_or_default());
    if sort == "off" {
        film.staging.nodes.remove(&name);
        film.beat(emit::unmake(name));
        return;
    }
    if name.is_empty() || !name.chars().all(|letter| letter.is_alphanumeric() || letter == '_') {
        film.wrong_coded(line.number, codes::NODE_KIND, format!("“{name}”: a node's name is one word, of letters, digits and _"));
        return;
    }
    if film.someone(&name).is_some() || film.groups.contains_key(&name) || film.thing_words.contains(&name) {
        film.wrong_coded(line.number, codes::NODE_TAKEN, format!("“{name}” is already someone or something of the scene: another name"));
        return;
    }
    let (sort, kind) = match (sort.as_str(), line.args.word("file")) {
        ("group", None) => (Sort::Group, NodeKind::Group),
        ("group", Some(said)) => {
            film.wrong_coded(line.number, codes::VALUE_TYPE, format!("“{said}”: a point, x y"));
            return;
        }
        ("image" | "picture", Some(file)) => (Sort::Picture(file.to_string()), NodeKind::Picture),
        ("image" | "picture", None) => {
            film.wrong_coded(line.number, codes::VALUE_MISSING, format!("“{name}”: node {name} image <file.png> [x y]"));
            return;
        }
        _ => {
            film.wrong_coded(line.number, codes::NODE_KIND, format!("“{sort}” is not a kind of node: group, image"));
            return;
        }
    };
    let at = match (line.args.word("x"), line.args.word("y")) {
        (Some(x), Some(y)) => match film.point_at(x, y) {
            Some(at) => at,
            None => return,
        },
        _ => Point::at(glam::Vec2::ZERO),
    };
    film.staging.nodes.insert(name.clone(), kind);
    film.beat(emit::make(name, sort, at));
}
