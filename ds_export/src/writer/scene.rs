//! The scenes put together: each scene's actors fitted to what Sky loads
//! at once, its layout and its script; the blocks played beside the script
//! (`beside`, `float`) as Sky's routines.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use engine::Who;

use ::film::ds::patches::{DYNAMIC_ACTOR, PORTRAIT_ALONE};
use ::film::ds::room;

use super::talk::capitalised;
use super::{AT_ONCE, Actor, SHIFT, SHIFTERS, Writer, names};
use crate::notes::{Fate, Kind};
use crate::sky::{FACES, HIDDEN, direction, quoted, tiles};

/// What one left out did, said after its size.
fn speaking(rank: u8) -> &'static str {
    match rank {
        3 => ", speaking",
        2 => ", moving",
        1 => ", acting",
        _ => "",
    }
}

/// An actor's part of a block played beside the script.
#[derive(Clone, Debug, Default)]
pub(super) struct Part {
    pub(super) actor: String,
    /// Its lines as the script tells them to the actor (if it can't have a routine).
    pub(super) told: Vec<String>,
    /// As its own routine says them.
    pub(super) own: Vec<String>,
    /// Hovering at its end, for ever (`MoveHeight`'s speed and height).
    pub(super) float: Option<(String, String)>,
}

/// A `beside` being written: its thread, where it starts in the body, where
/// each of its lines starts.
pub(super) struct Capture {
    pub(super) thread: String,
    pub(super) start: usize,
    pub(super) lines: Vec<usize>,
}

/// The common routines Sky's own routines call (`CallCommon`), for those a
/// script runs on an actor (`ExecuteCommon<actor>(…_SERIES, 0)`).
const CALLED: [&str; 8] = [
    "CORO_JUMP_HAPPY_FUNC",
    "CORO_JUMP_SURPRISE_FUNC",
    "CORO_JUMP_ANGRY_FUNC",
    "CORO_LOOK_AROUND_FUNC",
    "CORO_LOOK_AROUND_DOWN_FUNC",
    "CORO_LOOK_AROUND_LEFT_FUNC",
    "CORO_LOOK_AROUND_RIGHT_FUNC",
    "CORO_HEAD_SHAKE_FUNC",
];

/// The actor a script line tells something to (`<actor ACTOR_X>`).
fn target_of(line: &str) -> Option<&str> {
    let at = line.find("<actor ")? + "<actor ".len();
    let end = line[at..].find('>')?;
    Some(&line[at..at + end])
}

/// A line told to an actor, as its own routine says it (none: nothing to
/// say there); Err: a routine can't.
fn own_line(line: &str, actor: &str) -> Result<Option<String>, ()> {
    if line == format!("WaitExecuteLives({actor});") {
        return Ok(None);
    }
    let targeted = format!("<actor {actor}>");
    if let Some(rest) = line.strip_prefix(&format!("ExecuteCommon{targeted}(")) {
        let routine = rest.strip_suffix("_SERIES, 0);").filter(|routine| CALLED.contains(routine)).ok_or(())?;
        return Ok(Some(format!("CallCommon({routine});")));
    }
    Ok(Some(line.replace(&targeted, "")))
}

/// A block played beside the script, as each actor's part: a line of the
/// film's goes to the one it tells something; one telling no one goes with
/// the one before. Also what goes before it in the script (others' emotes
/// put away). None: no routines (a line said, the camera, two actors in a
/// line, a routine Sky's own can't call).
fn parts_of(chunks: &[&str]) -> Option<(Vec<String>, Vec<Part>)> {
    let mut before = Vec::new();
    let mut parts: Vec<Part> = Vec::new();
    let mut settled: Vec<(String, String)> = Vec::new();
    let mut last: Option<usize> = None;
    let mut pending: Vec<String> = Vec::new();
    for chunk in chunks {
        let mut owner: Option<String> = None;
        let mut mine: Vec<String> = Vec::new();
        for line in chunk.lines().map(str::trim).filter(|line| !line.is_empty()) {
            // (Standing again, an emote put away: of whoever it is.)
            let settle = line.ends_with("(EFFECT_NONE, 3);") || (line.starts_with("SetAnimation<") && line.ends_with(">(2);"));
            match target_of(line) {
                Some(target) if settle && owner.as_deref() != Some(target) => settled.push((target.to_string(), line.to_string())),
                Some(target) => {
                    if owner.as_deref().is_some_and(|owner| owner != target) {
                        return None;
                    }
                    owner = Some(target.to_string());
                    mine.push(line.to_string());
                }
                None if ["Wait(", "WaitExecuteLives(", "se_Play(", "camera_SetEffect("].iter().any(|start| line.starts_with(start)) => {
                    mine.push(line.to_string())
                }
                None => return None,
            }
        }
        // (What tells no one before anyone: with the next one, as a wait before its lines.)
        let Some(owner) = owner.or_else(|| last.map(|at| parts[at].actor.clone())) else {
            pending.extend(mine);
            continue;
        };
        let mine: Vec<String> = std::mem::take(&mut pending).into_iter().chain(mine).collect();
        let at = parts.iter().position(|part| part.actor == owner).unwrap_or_else(|| {
            parts.push(Part { actor: owner.clone(), ..Part::default() });
            parts.len() - 1
        });
        last = Some(at);
        let part = &mut parts[at];
        for line in mine {
            if let Some(height) = line.strip_suffix(" // float") {
                let told = height.to_string();
                let (pace, up) = told.split_once('(')?.1.trim_end_matches(");").split_once(", ")?;
                part.float = Some((pace.to_string(), up.to_string()));
                part.told.push(told);
                continue;
            }
            if let Some(own) = own_line(&line, &owner).ok()? {
                part.own.push(own);
            }
            part.told.push(line);
        }
    }
    for (actor, line) in settled {
        match parts.iter_mut().find(|part| part.actor == actor) {
            Some(part) => {
                part.own.push(own_line(&line, &actor).ok()??);
                part.told.push(line);
            }
            None => before.push(line),
        }
    }
    // (An emote of its own put away at the end of its part, as the engine's last a while.)
    for part in &mut parts {
        if part.float.is_none() && part.own.iter().rev().find(|line| line.starts_with("SetEffect(")).is_some_and(|line| !line.contains("EFFECT_NONE")) {
            part.own.push("SetEffect(EFFECT_NONE, 3);".to_string());
        }
    }
    Some((before, parts))
}

impl Writer<'_> {
    /// A `beside` starts: what it writes is kept apart, to be each one's routine.
    pub(super) fn start_beside(&mut self, thread: &str) {
        self.capture = Some(Capture { thread: thread.to_string(), start: self.body.len(), lines: Vec::new() });
    }

    /// A `beside` written: each actor's part in a routine of its own (see
    /// `routines`), the script starting them here; or, if it can't, its
    /// lines told at once in the script, waited for when it is.
    pub(super) fn end_beside(&mut self) {
        self.stand();
        let Some(capture) = self.capture.take() else { return };
        let text = self.body.split_off(capture.start);
        let mut cuts: Vec<usize> = vec![0];
        cuts.extend(capture.lines.iter().map(|at| at - capture.start));
        cuts.push(text.len());
        cuts.dedup();
        let chunks: Vec<&str> = cuts.windows(2).map(|cut| &text[cut[0]..cut[1]]).collect();
        match parts_of(&chunks) {
            Some((before, parts)) if !parts.is_empty() => {
                self.kept.retain(|actor| !parts.iter().any(|part| part.actor == *actor));
                self.showing.retain(|actor| !parts.iter().any(|part| part.actor == *actor));
                for line in before {
                    self.line(&line);
                }
                self.begin_block(capture.thread, parts);
            }
            _ => {
                // (Each line told at once, its waits when the script waits for it.)
                for line in text.lines().filter(|line| !line.trim_start().starts_with("Wait(")) {
                    self.body.push_str(line);
                    self.body.push('\n');
                }
            }
        }
    }

    /// A block beside the script starts here (its routines are written with the scene).
    pub(super) fn begin_block(&mut self, thread: String, parts: Vec<Part>) {
        let block = self.besides.len();
        let names: Vec<&str> = parts.iter().map(|part| part.actor.as_str()).collect();
        self.line(&format!("// dw-beside {block} {}", names.join(" ")));
        for part in parts.iter().filter(|part| part.float.is_some()) {
            if !self.floating.contains(&part.actor) {
                self.floating.push(part.actor.clone());
            }
        }
        self.besides.push(parts);
        self.threads.insert(thread, block);
    }

    /// The blocks played beside the script as Sky's routines: each actor's
    /// parts in a `def n for actor` of its own, started and joined with
    /// locks as Sky's scenes do (`WaitLockLives`, `Unlock`, `Lock`). One the
    /// script tells something before its last part plays its parts in the
    /// script instead, at once (an order given to an actor ends its
    /// routine). Who is hidden from the start hides in its routine.
    pub(super) fn routines(&mut self, hidden: &mut Vec<String>) -> String {
        if self.besides.is_empty() {
            return String::new();
        }
        let lines: Vec<String> = self.body.lines().map(str::to_string).collect();
        let starts = |block: usize| lines.iter().position(|line| line.trim_start().starts_with(&format!("// dw-beside {block} ")));
        let mut of: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (block, parts) in self.besides.iter().enumerate() {
            for part in parts {
                of.entry(part.actor.clone()).or_default().push(block);
            }
        }
        let part = |block: usize, actor: &str| self.besides[block].iter().find(|part| part.actor == actor).cloned().unwrap_or_default();
        let mut own: Vec<String> = of
            .iter()
            .filter(|(actor, blocks)| {
                let last = blocks.last().and_then(|block| starts(*block)).unwrap_or(0);
                let told = format!("<actor {actor}>");
                let waited = format!("WaitExecuteLives({actor});");
                let before = lines[..last].iter().any(|line| line.contains(&told) || line.contains(&waited));
                let floats_early = blocks[..blocks.len() - 1].iter().any(|block| part(*block, actor).float.is_some());
                !before && !floats_early
            })
            .map(|(actor, _)| actor.clone())
            .collect();
        // Locks: one for all at the end, one for each block for those starting there, one after each part another follows.
        let mut next = 1;
        let park = next;
        next += 1;
        let mut starting: HashMap<usize, u16> = HashMap::new();
        let mut gate: HashMap<(usize, String), u16> = HashMap::new();
        let mut done: HashMap<(usize, String), u16> = HashMap::new();
        for actor in &own {
            let blocks = &of[actor];
            for (at, block) in blocks.iter().enumerate() {
                let opens = match at {
                    0 => *starting.entry(*block).or_insert_with(|| {
                        next += 1;
                        next - 1
                    }),
                    _ => done[&(blocks[at - 1], actor.clone())],
                };
                gate.insert((*block, actor.clone()), opens);
                let ends = if at + 1 < blocks.len() {
                    next += 1;
                    next - 1
                } else {
                    park
                };
                done.insert((*block, actor.clone()), ends);
            }
        }
        // (Sky's scripts use locks 0 to 19; the items' paths take the last ones.)
        if next > 20 - self.path_locks {
            self.notes.push(crate::notes::Note::new(
                self.scene_line,
                Kind::Timing,
                Fate::Changed,
                "too many lines played beside the script for Sky's locks",
                "played in the script, one after the other",
            ));
            own.clear();
        }
        let mut body = String::new();
        for line in &lines {
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            let bare = line.trim_start();
            let (begins, rest) = match (bare.strip_prefix("// dw-beside "), bare.strip_prefix("// dw-join ")) {
                (Some(rest), _) => (true, rest),
                (_, Some(rest)) => (false, rest),
                _ => {
                    body.push_str(line);
                    body.push('\n');
                    continue;
                }
            };
            let block: usize = rest.split_whitespace().next().and_then(|number| number.parse().ok()).unwrap_or(0);
            let mut put = Vec::new();
            for part in &self.besides[block] {
                let mine = own.contains(&part.actor);
                match (begins, mine) {
                    (true, true) => put.push(format!("WaitLockLives({}, {});", gate[&(block, part.actor.clone())], part.actor)),
                    (true, false) => put.extend(part.told.iter().filter(|line| !line.starts_with("Wait(")).cloned()),
                    (false, _) if part.float.is_some() => {}
                    (false, true) => put.push(format!("WaitLockLives({}, {});", done[&(block, part.actor.clone())], part.actor)),
                    (false, false) => put.push(format!("WaitExecuteLives({});", part.actor)),
                }
            }
            if begins {
                let mut opened: Vec<u16> = Vec::new();
                for part in self.besides[block].iter().filter(|part| own.contains(&part.actor)) {
                    let lock = gate[&(block, part.actor.clone())];
                    if !opened.contains(&lock) {
                        opened.push(lock);
                    }
                }
                put.extend(opened.iter().map(|lock| format!("Unlock({lock});")));
            }
            for text in put {
                let _ = writeln!(body, "{indent}{text}");
            }
        }
        self.body = body;
        let mut defs = String::new();
        for (count, actor) in own.iter().enumerate() {
            let _ = writeln!(defs, "\ndef {} for actor {actor} {{", count + 1);
            let name = actor.trim_start_matches("ACTOR_");
            if hidden.iter().any(|one| one == name) {
                hidden.retain(|one| one != name);
                let _ = writeln!(defs, "    SetOutputAttribute({HIDDEN});");
            }
            for block in &of[actor] {
                let part = part(*block, actor);
                let _ = writeln!(defs, "    Lock({});", gate[&(*block, actor.clone())]);
                for line in &part.own {
                    let _ = writeln!(defs, "    {line}");
                }
                match &part.float {
                    Some((pace, up)) => {
                        let _ = writeln!(defs, "    forever {{\n        MoveHeight({pace}, {up});\n        MoveHeight({pace}, 0);\n    }}");
                    }
                    None => {
                        let _ = writeln!(defs, "    Lock({});", done[&(*block, actor.clone())]);
                    }
                }
            }
            defs.push_str("    hold;\n}\n");
        }
        if !own.is_empty() {
            self.info.push(format!("{}: {} routines beside the script, as Sky's (def n for actor)", self.level.as_deref().unwrap_or("?"), own.len()));
        }
        defs
    }

    /// The scene written, and the writer ready for the next: the film's
    /// own (its cast, title, words, what was told) kept.
    pub(super) fn next_scene(&mut self) {
        let block = self.block();
        let mut next = Writer::new(self.game);
        next.blocks = std::mem::take(&mut self.blocks);
        next.blocks.push(block);
        next.cast = std::mem::take(&mut self.cast);
        next.title = self.title.take();
        next.titled = self.titled;
        next.words = std::mem::take(&mut self.words);
        next.problems = std::mem::take(&mut self.problems);
        next.notes = std::mem::take(&mut self.notes);
        next.info = std::mem::take(&mut self.info);
        next.lines_seen = std::mem::take(&mut self.lines_seen);
        next.tall_asked = self.tall_asked;
        next.vars = std::mem::take(&mut self.vars);
        next.became = std::mem::take(&mut self.became);
        next.values = std::mem::take(&mut self.values);
        next.fights = std::mem::take(&mut self.fights);
        next.passages = std::mem::take(&mut self.passages);
        next.instead_poses = std::mem::take(&mut self.instead_poses);
        next.film_names = std::mem::take(&mut self.film_names);
        next.bubble_faces = std::mem::take(&mut self.bubble_faces);
        next.added = std::mem::take(&mut self.added);
        next.slots = std::mem::take(&mut self.slots);
        next.drawings = std::mem::take(&mut self.drawings);
        next.item_lines = std::mem::take(&mut self.item_lines);
        next.room = self.room.clone();
        next.squares = self.squares;
        *self = next;
    }

    /// The whole film: each scene, who stands where for `patch_rom.py`, then its `def 0`.
    pub(crate) fn finish(&mut self, film: &str) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "// The film “{film}”, exported by `scene_player export-ds` for tools/ds/patch_rom.py.");
        let last = self.block();
        out.push_str(&self.item_lines_for_patch().0);
        for block in self.blocks.iter().chain([&last]) {
            out.push_str(block);
        }
        out
    }

    /// A scene with more than Sky shows at once (`AT_ONCE`) made to fit:
    /// who comes in later is loaded then, in a layer of its own
    /// (`supervision_Acting(n)`, as Sky's scenes load theirs), in the slots
    /// of those who have left (`Destroy`, as Sky's scenes put away who is
    /// done); and if those there from the start are too many, the crowd's
    /// idle ones farthest from its middle are left out, never one who
    /// speaks, acts or is framed. What it did is told.
    pub(super) fn fit(&mut self) {
        let counted: Vec<Who> =
            self.actors.iter().filter(|(_, actor)| !actor.name.starts_with("PLAYER") && !actor.name.starts_with("ATTENDANT")).map(|(who, _)| *who).collect();
        if counted.len() <= AT_ONCE {
            return;
        }
        let level = self.level.clone().unwrap_or_default();
        let mut lines: Vec<String> = self.body.lines().map(str::to_string).collect();
        // Each one's first line, last line (none: there to the end), and whether a line names it.
        let mut spans: HashMap<Who, (usize, usize, bool)> = HashMap::new();
        for who in &counted {
            let actor = &self.actors[who];
            let token = format!("ACTOR_{}", actor.name);
            let said: Vec<usize> = lines.iter().enumerate().filter(|(_, line)| names(line, &token)).map(|(at, _)| at).collect();
            let start = if actor.hidden { said.first().copied().unwrap_or(usize::MAX) } else { 0 };
            let gone = format!("SetOutputAttribute<actor {token}>({HIDDEN});");
            let end = said.last().copied().filter(|at| lines[*at].contains(&gone)).unwrap_or(usize::MAX);
            spans.insert(*who, (start, end, !said.is_empty()));
        }
        let mut left_out = Vec::new();
        // Never seen: not loaded.
        for who in &counted {
            if spans[who].0 == usize::MAX {
                left_out.push(*who);
            }
        }
        // There from the start: the crowd's idle ones, farthest from its middle first.
        let first = |actor: &Actor| actor.first.map_or(self.middle.unwrap_or((128.0, 96.0)), |(x, y, _)| (x, y));
        let start: Vec<Who> = counted.iter().copied().filter(|who| spans[who].0 == 0).collect();
        let mut dropped = Vec::new();
        if start.len() > AT_ONCE {
            let count = start.len() as f32;
            let at: Vec<(f32, f32)> = start.iter().map(|who| first(&self.actors[who])).collect();
            let centre = (at.iter().map(|one| one.0).sum::<f32>() / count, at.iter().map(|one| one.1).sum::<f32>() / count);
            // What each does in the scene: idle (no line names it), acting
            // (an emote, a pose, a turn), moving, speaking (or framed among
            // a few): the least first, then the farthest from the crowd's middle.
            let rank = |who: &Who| -> u8 {
                let token = format!("ACTOR_{}", self.actors[who].name);
                let speaks = |line: &String| (line.contains("message_SetFace(") || line.contains("message_SetActor(")) && names(line, &token);
                let moves = |line: &String| {
                    names(line, &token) && ["MovePosition", "SlidePosition", "Turn2Direction", "ExecuteCommon", "MoveHeight"].iter().any(|op| line.contains(op))
                };
                if self.framed.contains(who) || lines.iter().any(speaks) {
                    3
                } else if lines.iter().any(moves) {
                    2
                } else if spans[who].2 {
                    1
                } else {
                    0
                }
            };
            let mut ranked: Vec<(u8, f32, Who)> = start
                .iter()
                .map(|who| {
                    let (x, y) = first(&self.actors[who]);
                    (rank(who), (x - centre.0).powi(2) + (y - centre.1).powi(2), *who)
                })
                .collect();
            ranked.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
            // (Never one who speaks or is framed: those the game can't show are told.)
            let leaving: Vec<(u8, Who)> =
                ranked.iter().filter(|(rank, ..)| *rank < 3).take(start.len() - AT_ONCE).map(|(rank, _, who)| (*rank, *who)).collect();
            dropped = leaving.iter().map(|(_, who)| *who).collect();
            let named = |who: &Who| self.film_names.get(who).cloned().unwrap_or_else(|| self.game.name(self.actors[who].kind));
            let of_rank = |wanted: u8| -> Vec<String> { leaving.iter().filter(|(rank, _)| *rank == wanted).map(|(_, who)| named(who)).collect() };
            let mut parts = Vec::new();
            for (rank, what) in [(0, "idle"), (1, "only acting where they stand"), (2, "moving, never speaking")] {
                let names = of_rank(rank);
                if !names.is_empty() {
                    parts.push(format!("{} ({what})", names.join(", ")));
                }
            }
            let fate = if parts.is_empty() { Fate::Changed } else { Fate::LeftOut };
            self.note(
                self.scene_line,
                Kind::Crowd,
                fate,
                format!("{level}: {} Pokémon from the start, Sky shows {AT_ONCE} at once", start.len()),
                if parts.is_empty() {
                    "no one left out".to_string()
                } else {
                    format!("left out, the farthest from the crowd's middle first, their lines said without them: {}", parts.join("; "))
                },
            );
            if start.len() - dropped.len() > AT_ONCE {
                self.note(
                    self.scene_line,
                    Kind::Crowd,
                    Fate::LeftOut,
                    format!("{level}: still {} too many, each speaking or framed", start.len() - dropped.len() - AT_ONCE),
                    "the game won't show the last ones of the layout",
                );
            }
        }
        // Who comes in later: loaded then, where some have left.
        let mut live: Vec<(Who, usize)> = start.iter().filter(|who| !dropped.contains(who)).map(|who| (*who, spans[who].1)).collect();
        let mut later: Vec<(usize, Who)> = counted.iter().map(|who| (spans[who].0, *who)).filter(|(at, _)| *at > 0 && *at < usize::MAX).collect();
        later.sort();
        let mut inserts: Vec<(usize, Vec<String>)> = Vec::new();
        let mut layer = 0;
        let mut at = 0;
        while at < later.len() {
            let line = later[at].0;
            let group: Vec<Who> = later[at..].iter().take_while(|(start, _)| *start == line).map(|(_, who)| *who).collect();
            at += group.len();
            let mut put = Vec::new();
            for (who, _) in live.iter().filter(|(_, end)| *end < line) {
                put.push(format!("Destroy<actor ACTOR_{}>();", self.actors[who].name));
            }
            live.retain(|(_, end)| *end >= line);
            let room = AT_ONCE.saturating_sub(live.len());
            let (fits, over) = group.split_at(room.min(group.len()));
            dropped.extend(over.iter().copied());
            if !over.is_empty() {
                let names: Vec<String> = over.iter().map(|who| self.game.name(self.actors[who].kind)).collect();
                self.note(
                    self.scene_line,
                    Kind::Crowd,
                    Fate::LeftOut,
                    format!("{level}: no room yet for {} when they come in ({AT_ONCE} at once)", names.join(", ")),
                    "left out",
                );
            }
            if fits.is_empty() {
                continue;
            }
            layer += 1;
            put.push(format!("supervision_Acting({layer});"));
            for who in fits {
                let actor = self.actors.get_mut(who).expect("an actor of the scene");
                actor.layer = layer;
                put.push(format!("SetOutputAttribute<actor ACTOR_{}>({HIDDEN});", actor.name));
                live.push((*who, spans[who].1));
            }
            inserts.push((line, put));
        }
        if layer > 0 {
            self.info.push(format!("{level}: who comes in later is loaded then, in {layer} more layers of the layout, in the slots of who has left"));
        }
        for (line, put) in inserts.into_iter().rev() {
            let indent: String = lines[line].chars().take_while(|c| c.is_whitespace()).collect();
            for (n, text) in put.into_iter().enumerate() {
                lines.insert(line + n, format!("{indent}{text}"));
            }
        }
        // (Those left out: no order to one the game hasn't loaded; their lines said without them.)
        let gone: Vec<Who> = dropped.iter().chain(&left_out).copied().collect();
        lines = self.without_actors(lines, &gone);
        self.body = lines.into_iter().map(|line| line + "\n").collect();
    }

    /// The script's lines without those actors: every order to them left
    /// out, but their lines said, without them (`message_EmptyActor`, their
    /// name in the words), their portrait shown by set-portrait-monster
    /// (`ProcessSpecial(64, species, face)`) where the base has it.
    fn without_actors(&mut self, lines: Vec<String>, gone: &[Who]) -> Vec<String> {
        let mut lines = lines;
        for who in gone {
            let Some(actor) = self.actors.remove(who) else { continue };
            let token = format!("ACTOR_{}", actor.name);
            let name = self.film_names.get(who).map_or_else(|| self.game.name(actor.kind), |name| capitalised(name));
            // (A species SpriteCollab adds has its number from patch_rom.py: no portrait of its own here.)
            let portrait = self.patches.process(PORTRAIT_ALONE).filter(|_| !self.added.contains_key(&actor.kind) && actor.kind > 0);
            let mut out = Vec::with_capacity(lines.len());
            let mut named = false;
            for line in lines {
                let trimmed = line.trim_start();
                let indent = &line[..line.len() - trimmed.len()];
                let talk = ["message_Talk(\"", "message_Monologue(\""].into_iter().find(|talk| trimmed.starts_with(talk));
                if named && let Some(talk) = talk {
                    let words = &trimmed[talk.len()..];
                    let words = if words.starts_with(' ') { words.to_string() } else { format!(" {words}") };
                    out.push(format!("{indent}{talk}[CS:N]{name}[CR]:{words}"));
                    named = false;
                    continue;
                }
                if !names(&line, &token) {
                    out.push(line);
                    continue;
                }
                let face = trimmed
                    .strip_prefix(&format!("message_SetFace({token}, "))
                    .and_then(|rest| rest.split(',').next())
                    .map(|face| face.trim().trim_start_matches("FACE_"));
                if face.is_some() || trimmed.starts_with(&format!("message_SetActor({token})")) {
                    out.push(format!("{indent}message_EmptyActor();"));
                    let number = face.map_or(Some(0), |face| FACES.iter().position(|one| *one == face).and_then(|at| u16::try_from(at).ok()));
                    if let (Some(id), Some(face)) = (portrait, number.filter(|face| self.game.has_face(actor.kind, *face))) {
                        out.push(format!("{indent}ProcessSpecial({id}, {}, {face});", actor.kind));
                    }
                    named = true;
                }
            }
            lines = out;
            for parts in &mut self.besides {
                parts.retain(|part| part.actor != token);
            }
        }
        lines
    }

    /// Those of the scene left out, their lines said without them (no order to one
    /// the game hasn't loaded).
    pub(super) fn drop_actors(&mut self, gone: &[Who]) {
        let lines: Vec<String> = self.body.lines().map(str::to_string).collect();
        let lines = self.without_actors(lines, gone);
        self.body = lines.into_iter().map(|line| line + "\n").collect();
    }

    /// The film's name of one of that species in the scene, else the game's.
    fn kind_name(&self, kind: u16) -> String {
        let film_name = self.actors.iter().find(|(_, actor)| actor.kind == kind).and_then(|(who, _)| self.film_names.get(who).cloned());
        film_name.unwrap_or_else(|| self.game.name(kind))
    }

    /// The species of the scene's actors, in the order their sprites load
    /// (the layout's), each once.
    fn load_order(&self) -> Vec<u16> {
        room::once(&self.actors.values().map(|actor| actor.kind).collect::<Vec<u16>>())
    }

    /// Room for the scene's sprites (`film::ds::room`): each species loads
    /// its sprite once as the scene starts, in Sky's arena then the hook's,
    /// the objects' after them; past the room a sprite is a magenta square.
    /// Over it (a scene no moment could split), the species least needed
    /// leave the scene, never the hero nor the partner, each of its Pokémon
    /// with its lines: first those no line names, then those only acting,
    /// then moving, then speaking; among them the fewest lines first. Told,
    /// with the bytes. `--squares`: kept, told as squares.
    pub(super) fn fit_sprites(&mut self) {
        let data = self.game.data.clone();
        let objects = self.objects.len();
        let order = self.load_order();
        if room::fits(&data, &self.room, &order, objects) {
            return;
        }
        let level = self.level.clone().unwrap_or_default();
        let before = room::scene_bytes(&data, &order, objects);
        let said = format!("{level}: {} species, their sprites about {} KB; {}", order.len(), room::kb(before), self.room.said());
        if self.squares {
            let placed = self.room.place(&room::costs(&data, &order, objects));
            let squares: Vec<String> = order.iter().zip(&placed).filter(|(_, at)| at.is_none()).map(|(kind, _)| self.kind_name(*kind)).collect();
            self.note(
                self.scene_line,
                Kind::Sprites,
                Fate::Changed,
                said,
                format!("kept as asked (--squares): {} drawn as magenta squares", squares.join(", ")),
            );
            return;
        }
        let lines: Vec<String> = self.body.lines().map(str::to_string).collect();
        // Each species' need: what its Pokémon do at most, then how many lines name them.
        let mut need: BTreeMap<u16, (u8, usize)> = BTreeMap::new();
        for (who, actor) in &self.actors {
            let token = format!("ACTOR_{}", actor.name);
            let named: Vec<&String> = lines.iter().filter(|line| names(line, &token)).collect();
            let speaks = named.iter().any(|line| line.contains("message_SetFace(") || line.contains("message_SetActor("));
            let moves =
                named.iter().any(|line| ["MovePosition", "SlidePosition", "Turn2Direction", "ExecuteCommon", "MoveHeight"].iter().any(|op| line.contains(op)));
            let rank = if actor.name.starts_with("PLAYER") || actor.name.starts_with("ATTENDANT") {
                5
            } else if speaks || self.framed.contains(who) {
                3
            } else if moves {
                2
            } else {
                u8::from(!named.is_empty())
            };
            let one = need.entry(actor.kind).or_insert((0, 0));
            *one = (one.0.max(rank), one.1 + named.len());
        }
        // (The hero's and the partner's never leave.)
        let mut ranked: Vec<(u8, usize, u16)> = need.iter().filter(|(_, (rank, _))| *rank < 5).map(|(kind, (rank, count))| (*rank, *count, *kind)).collect();
        ranked.sort();
        let least: Vec<u16> = ranked.iter().map(|(.., kind)| *kind).collect();
        let left = room::leave_out(&data, &self.room, &order, &least, objects);
        let kept: Vec<u16> = order.iter().copied().filter(|kind| !left.contains(kind)).collect();
        let names_left: Vec<String> = left
            .iter()
            .map(|kind| format!("{} ({} KB{})", self.kind_name(*kind), room::kb(room::sprite_bytes(&data, *kind)), speaking(need[kind].0)))
            .collect();
        let still = if room::fits(&data, &self.room, &kept, objects) { String::new() } else { "; still over it: the last ones magenta squares".to_string() };
        self.note(
            self.scene_line,
            Kind::Sprites,
            Fate::LeftOut,
            said,
            format!(
                "no moment splits the scene so that both parts fit: left out, the least needed first, their lines said without them (a portrait alone): {} (now {} KB{still}); split the scene, have some leave in one scene and the others arrive in the next, or use fewer new species",
                names_left.join(", "),
                room::kb(room::scene_bytes(&data, &kept, objects))
            ),
        );
        let gone: Vec<Who> = self.actors.iter().filter(|(_, actor)| left.contains(&actor.kind)).map(|(who, _)| *who).collect();
        self.drop_actors(&gone);
    }

    /// The scene over the room split here, at the start of a moment (`lib`
    /// chose it: `film::ds::room::split_at`): this part keeps the species
    /// of `first` (those that only come later leave its layout), ends with
    /// its fade, and the next DS scene opens on the same place and view,
    /// those of `second`'s species there now put where they are, facing
    /// as they do (hidden if they are), the others coming in as they do.
    /// Told. False (not split, trimmed with the block): in a block played
    /// beside the script, or no place yet.
    pub(crate) fn split_scene(&mut self, first: &[u16], second: &[u16], moment: &str, line: i32) -> bool {
        if self.capture.is_some() || self.level.is_none() || !self.started {
            return false;
        }
        let data = self.game.data.clone();
        let lines: Vec<String> = self.body.lines().map(str::to_string).collect();
        let middle = self.middle.unwrap_or((128.0, 96.0));
        let going_on: Vec<(Who, Actor)> = self
            .actors
            .iter()
            .filter(|(_, actor)| second.contains(&actor.kind))
            .map(|(who, actor)| {
                let token = format!("ACTOR_{}", actor.name);
                let hidden = match lines.iter().rev().find(|line| names(line, &token)) {
                    Some(last) => last.contains(&format!("SetOutputAttribute<actor {token}>({HIDDEN});")),
                    None => actor.hidden && actor.first.is_none(),
                };
                let (x, y) = self.at.get(who).copied().unwrap_or(actor.first.map_or(middle, |(x, y, _)| (x, y)));
                let facing = self.facing.get(who).map_or(actor.first.map_or("DIR_DOWN", |(.., facing)| facing), |way| direction(*way));
                (*who, Actor { first: Some((x, y, facing)), hidden, seen: false, layer: 0, ..actor.clone() })
            })
            .collect();
        let level = self.level.clone().unwrap_or_default();
        let (first_bytes, second_bytes) = (room::scene_bytes(&data, first, 0), room::scene_bytes(&data, second, 0));
        self.note(
            self.scene_line,
            Kind::Sprites,
            Fate::Changed,
            format!("{level}: {} species, their sprites about {} KB; {}", room::once(&[first, second].concat()).len(), room::kb(room::scene_bytes(&data, &[first, second].concat(), 0)), self.room.said()),
            format!(
                "split into two DS scenes at the moment “{moment}” (line {line}), a fade between: {} species ({} KB), then {} ({} KB); who goes on stands where they were, back to standing",
                first.len(),
                room::kb(first_bytes),
                second.len(),
                room::kb(second_bytes)
            ),
        );
        self.keep_kinds = Some(first.to_vec());
        // (What the next part goes on with: the place, the view, where each one is.)
        let (top, tall, size, middle, camera) = (self.top, self.tall, self.size, self.middle, self.camera);
        let (places, at, facing, tint, weather) = (self.places.clone(), self.at.clone(), self.facing.clone(), self.tint, self.weather);
        let (followers, scene_line) = (self.followers.clone(), self.scene_line);
        self.next_scene();
        (self.top, self.tall, self.size, self.middle, self.camera) = (top, tall, size, middle, camera);
        (self.places, self.at, self.facing, self.tint, self.weather) = (places, at, facing, tint, weather);
        (self.followers, self.scene_line) = (followers, scene_line);
        let bare = level.trim_start_matches("LEVEL_").to_string();
        if self.tall {
            self.line("back2_SetMode(4);");
            self.line(&format!("back2_SetGround(LEVEL_{bare});"));
        } else if let Some(top) = self.top {
            self.line("back2_SetMode(4);");
            self.line(&format!("back2_SetGround(LEVEL_{top});"));
        }
        self.line(&format!("back_SetGround({level});"));
        self.line("supervision_Acting(0);");
        self.level = Some(level);
        self.line("camera_SetMyself<performer 0>();");
        let position = self.position(self.camera);
        self.line(&format!("SetPositionMark<performer 0>({position});"));
        for (who, actor) in going_on {
            self.actors.insert(who, actor);
        }
        self.fade_in(false, engine::Frames(30.0));
        self.line("WaitScreenFade();");
        true
    }

    /// One scene: its level, actors and performers, then its `def 0`.
    pub(super) fn block(&mut self) -> String {
        // (A scene split for its sprites: who only comes in its next part leaves this one.)
        if let Some(keep) = self.keep_kinds.take() {
            let gone: Vec<Who> = self.actors.iter().filter(|(_, actor)| !keep.contains(&actor.kind)).map(|(who, _)| *who).collect();
            self.drop_actors(&gone);
        }
        self.fit_sprites();
        self.fit();
        let mut out = String::new();
        let _ = writeln!(out, "// ds-export level {}", self.level.as_deref().unwrap_or("?"));
        let middle = self.middle.unwrap_or((128.0, 96.0));
        for actor in self.actors.values() {
            let (x, y, facing) = actor.first.unwrap_or((middle.0, middle.1, "DIR_DOWN"));
            let species = actor.species.map_or(String::new(), |species| format!(" species {species}"));
            let layer = if actor.layer > 0 { format!(" layer {}", actor.layer) } else { String::new() };
            let _ = writeln!(out, "// ds-export actor ACTOR_{}{species} x {} y {} facing {facing}{layer}", actor.name, tiles(x), tiles(y));
        }
        // Who transforms (dynamic_actor): his species set before the scene
        // loads him, and a layer of his own to load him again as another.
        let mut layer = self.actors.values().map(|actor| actor.layer).max().unwrap_or(0);
        type Shifting<'a> = Vec<(String, u16, u16, (f32, f32, &'a str))>;
        let shifting: Shifting<'_> = self
            .actors
            .values()
            .filter_map(|actor| {
                let (_, field) = SHIFTERS.iter().find(|(name, _)| *name == actor.name)?;
                Some((actor.name.clone(), actor.kind, *field, actor.first.unwrap_or((middle.0, middle.1, "DIR_DOWN"))))
            })
            .collect();
        if let Some(id) = self.patches.process(DYNAMIC_ACTOR) {
            for (name, kind, field, (x, y, facing)) in shifting {
                let placeholder = format!("{SHIFT}ACTOR_{name}@");
                if self.body.contains(&placeholder) {
                    layer += 1;
                    self.body = self.body.replace(&placeholder, &layer.to_string());
                    let _ = writeln!(out, "// ds-export actor ACTOR_{name} x {} y {} facing {facing} layer {layer}", tiles(x), tiles(y));
                }
                let set = format!("    ProcessSpecial({id}, {kind}, {field}); // ACTOR_{name}\n");
                let at = self.body.find("    supervision_Acting(0);").unwrap_or(0);
                self.body.insert_str(at, &set);
            }
        }
        self.mimicking.clear();
        for (object, (x, y)) in &self.objects {
            let _ = writeln!(out, "// ds-export object OBJECT_N{object} x {} y {}", tiles(*x), tiles(*y));
        }
        // The camera, and the weather over the screen.
        let _ = writeln!(out, "// ds-export performer 0 type 0 x {} y {}", tiles(middle.0), tiles(middle.1));
        if self.weather {
            let _ = writeln!(out, "// ds-export performer 3 type 3 x {} y {}", tiles(middle.0), tiles(middle.1));
        }
        let _ = writeln!(out, "def 0 {{");
        // The title, as Sky shows its story's: alone on black, before the first scene.
        if let Some(title) = self.title.as_ref().filter(|_| !self.titled) {
            self.titled = true;
            out.push_str("    sound_Stop();\n");
            let _ = writeln!(out, "    back_SetTitleBanner(56, \"{}\");", quoted(title));
            out.push_str("    screen_FadeIn(1, 30);\n    Wait(120);\n    screen_FadeOut(1, 30);\n    Wait(15);\n");
        }
        // Who comes in later is hidden until then.
        let mut hidden: Vec<String> = self.actors.values().filter(|actor| actor.hidden && actor.layer == 0).map(|actor| actor.name.clone()).collect();
        let mut routines = self.routines(&mut hidden);
        // (The items' paths: their objects' routines, numbered after the actors'.)
        let count = routines.matches("\ndef ").count();
        routines.push_str(&self.object_routines(count + 1));
        let mut body = std::mem::take(&mut self.body);
        if !hidden.is_empty() || !self.hidden_objects.is_empty() {
            let mut hide: String = hidden.iter().map(|name| format!("    SetOutputAttribute<actor ACTOR_{name}>({HIDDEN});\n")).collect();
            for object in &self.hidden_objects {
                hide.push_str(&format!("    SetOutputAttribute<object OBJECT_N{object}>({HIDDEN});\n"));
            }
            let at = body.find("    camera_SetMyself").map_or(0, |at| body[at..].find('\n').map_or(at, |end| at + end + 1));
            body.insert_str(at, &hide);
        }
        out.push_str(&body);
        if !self.faded_out {
            out.push_str("    screen_FadeOut(1, 30);\n    WaitScreenFade();\n");
        }
        if self.memory {
            out.push_str("    screen_FlushIn(1, 0, 1, 0, 0, 0);\n");
        }
        if self.top.is_some() || self.tall {
            out.push_str("    screen2_FadeOut(0, 30);\n    WaitScreenFade();\n    back2_SetMode(0);\n");
        }
        if let Some(tint) = self.tint {
            let _ = writeln!(out, "    screen_FlushIn(1, 0, {tint});");
        }
        if self.white_all {
            out.push_str("    screen_FadeInAll(1, 0);\n    screen_FadeOut(1, 0);\n");
        }
        out.push_str("    end;\n}\n");
        out.push_str(&routines);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beside_is_each_actors_routine_as_sky_writes_them() {
        let chunks = [
            "    SetAnimation<actor ACTOR_A>(46);\n    Wait(6);\n",
            "    SetEffect<actor ACTOR_B>(EFFECT_ANGRY, 3);\n    ExecuteCommon<actor ACTOR_B>(CORO_JUMP_ANGRY_FUNC_SERIES, 0);\n    WaitExecuteLives(ACTOR_B);\n",
            "    SetEffect<actor ACTOR_C>(EFFECT_NONE, 3);\n    MoveHeight<actor ACTOR_A>(0.25, 8); // float\n",
        ];
        let (before, parts) = parts_of(&chunks).expect("routines");
        // (An emote of someone else's put away before the block, in the script.)
        assert_eq!(before, ["SetEffect<actor ACTOR_C>(EFFECT_NONE, 3);"]);
        assert_eq!(parts[0].actor, "ACTOR_A");
        assert_eq!(parts[0].own, ["SetAnimation(46);", "Wait(6);"]);
        assert_eq!(parts[0].float, Some(("0.25".to_string(), "8".to_string())));
        // Sky's routines call the common ones (`CallCommon`), the scripts run them on an actor.
        assert_eq!(parts[1].own, ["SetEffect(EFFECT_ANGRY, 3);", "CallCommon(CORO_JUMP_ANGRY_FUNC);", "SetEffect(EFFECT_NONE, 3);"]);
        // A line said, or two actors told in one line: no routines.
        assert!(parts_of(&["    message_Talk(\" Hi.\");\n"]).is_none());
        assert!(parts_of(&["    SetDirection<actor ACTOR_B>(DIR_UP);\n    SetDirection<actor ACTOR_A>(DIR_UP);\n"]).is_none());
    }
}
