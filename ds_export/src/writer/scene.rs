//! The scenes put together: each scene's actors fitted to what Sky loads
//! at once, its layout and its script; the blocks played beside the script
//! (`beside`, `float`) as Sky's routines.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use engine::Who;

use super::{AT_ONCE, Actor, Writer, names};
use crate::notes::{Fate, Kind};
use crate::sky::{HIDDEN, quoted, tiles};

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
                    format!("left out, the farthest from the crowd's middle first, their lines with them: {}", parts.join("; "))
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
        // (Those left out, their lines with them: no order to one the game hasn't loaded.)
        for who in dropped.iter().chain(&left_out) {
            if let Some(actor) = self.actors.remove(who) {
                let token = format!("ACTOR_{}", actor.name);
                lines.retain(|line| !names(line, &token));
                for parts in &mut self.besides {
                    parts.retain(|part| part.actor != token);
                }
            }
        }
        self.body = lines.into_iter().map(|line| line + "\n").collect();
    }

    /// One scene: its level, actors and performers, then its `def 0`.
    pub(super) fn block(&mut self) -> String {
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
