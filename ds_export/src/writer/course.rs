//! The script's course: labels and jumps, tests, the film's variables,
//! choices, the waits for threads and clocks.

use engine::{Beat, BodyBeat, Compare, Course, DataBeat, Direction, FlowBeat, MotionBeat, Num, PlaceBeat, Point, SpeechBeat, Test, Value, ViewBeat};

use super::fight::fight_label;
use super::{BESIDE, Writer};
use crate::notes::{Fate, Kind};
use crate::sky::{LOCALS, quoted};

impl Writer<'_> {
    /// A beat as the script's course: what only our engine needs left out, the
    /// labels, jumps, tests and variables, the choices; the rest is `plain`'s.
    pub(super) fn course(&mut self, number: i32, word: Option<&str>, beat: &Beat) {
        match beat {
            // What only our engine needs: its grid, its light, how it spaces
            // and turns people by itself, the sizes of the bodies.
            Beat::View(ViewBeat::Body { .. } | ViewBeat::KeepSpeaker(_) | ViewBeat::KeepFrame(_))
            | Beat::Place(PlaceBeat::Footing { .. })
            | Beat::Motion(MotionBeat::Spacing(_) | MotionBeat::Speed { .. })
            | Beat::Speech(SpeechBeat::AutoFace(_) | SpeechBeat::Audience(_))
            | Beat::Flow(FlowBeat::Parallel(_)) => {}
            Beat::Flow(flow) => self.course_flow(number, word, flow),
            // Those the scene opens with, spread out in a row round its middle, turned to each other.
            Beat::Place(PlaceBeat::Spread { who, .. }) if !self.started => {
                let Some((x, y)) = self.middle else { return };
                let many = who.len() as f32;
                for (at, one) in who.iter().enumerate() {
                    let along = (at as f32 - (many - 1.0) / 2.0) * 2.0 * BESIDE;
                    let facing = if along < 0.0 {
                        Direction::Right
                    } else if along > 0.0 || many > 1.0 {
                        Direction::Left
                    } else {
                        Direction::Down
                    };
                    let at = Point { x: Num::Lit(x + along), y: Num::Lit(y) };
                    self.plain(number, word, &Beat::Body(BodyBeat::Put { who: *one, at, direction: facing }));
                }
            }
            Beat::Place(PlaceBeat::Spread { .. }) => {}
            Beat::Data(data) => self.data(number, word, data),
            Beat::Course(course) => self.choice(number, word, course),
            // What the script's course is not: a beat of a kind (`plain`).
            other => self.plain(number, word, other),
        }
    }

    /// A flow beat of the course: a label, a jump, a test, a wait for a thread or a clock; the waits are `flow`'s.
    pub(super) fn course_flow(&mut self, number: i32, word: Option<&str>, beat: &FlowBeat) {
        match beat {
            FlowBeat::Label(label) => self.label(label, word),
            FlowBeat::Goto(label) if label.starts_with("thread") || fight_label(label) => {}
            FlowBeat::Goto(label) => self.line(&format!("jump @{};", label.trim_start_matches('_'))),
            FlowBeat::WaitUntil(test) if test.any.iter().flatten().any(|condition| condition.var == "clock" || condition.var == engine::MUSIC_CLOCK) => {
                let frames = test.any.iter().flatten().find_map(|condition| match &condition.value {
                    Value::Number(frames) => Some(*frames),
                    Value::Var(name) => self.values.get(name).copied(),
                });
                if let Some(frames) = frames.filter(|frames| *frames >= 1.0) {
                    self.line(&format!("Wait({});", frames.round() as i64));
                }
            }
            FlowBeat::Return => {
                // (The end of a fight's passage: kept for where the fight plays it.)
                if let Some((label, at)) = self.passage.take() {
                    let text = self.body.split_off(at.min(self.body.len()));
                    self.passages.insert(label, text);
                }
                self.thread = false;
            }
            FlowBeat::WaitUntil(test) if test.any.iter().flatten().all(|condition| condition.var.starts_with("thread")) => {
                // (A block of routines: joined where the film waits for it.)
                for condition in test.any.iter().flatten() {
                    if let Some(block) = self.threads.remove(condition.var.trim_end_matches("_done")) {
                        self.line(&format!("// dw-join {block}"));
                    }
                }
                for actor in std::mem::take(&mut self.kept) {
                    self.wait_for(&actor);
                }
            }
            FlowBeat::If(test) => {
                if let Some(test) = self.test(number, test) {
                    self.line(&format!("if ( {test} ) {{"));
                    self.indent += 1;
                }
            }
            FlowBeat::ElseIf(test) => {
                if let Some(test) = self.test(number, test) {
                    self.indent -= 1;
                    self.line(&format!("}} else if ( {test} ) {{"));
                    self.indent += 1;
                }
            }
            FlowBeat::Else => {
                self.indent -= 1;
                self.line("} else {");
                self.indent += 1;
            }
            FlowBeat::EndIf => {
                self.indent -= 1;
                self.line("}");
            }
            other => self.plain(number, word, &Beat::Flow(other.clone())),
        }
    }

    /// A data beat: a variable set, added to or drawn at random, a mark.
    pub(super) fn data(&mut self, number: i32, word: Option<&str>, beat: &DataBeat) {
        match beat {
            // How often a moment was seen: the film's own count, no `seen()` on DS.
            // When a line beside the film plays (`at mark + 2s:`): so long after (from where it is).
            DataBeat::Calc { var, b: Value::Number(after), .. } if var.starts_with("_at_") => {
                self.values.insert(var.clone(), *after);
            }
            DataBeat::Calc { var, a, op, b } => {
                let by = match (a, b) {
                    (Value::Var(of), Value::Number(by)) if of == var && (*op == "+" || *op == "-") => Some(*by as i64),
                    _ => None,
                };
                let Some(name) = self.var(number, var) else { return };
                match by {
                    Some(by) => {
                        let now = self.values.get(var).copied().unwrap_or(0.0);
                        self.values.insert(var.clone(), if *op == "+" { now + by as f64 } else { now - by as f64 });
                        self.line(&format!("{name} {op}= {by};"));
                    }
                    None => {
                        self.problem(number, Kind::Logic, Fate::Impossible, format!("the calculation with {op}: only adding to a variable exports"), "left out")
                    }
                }
            }
            // A number drawn: Sky's `switch ( random(n) )`, its cases a tenth of the range each.
            DataBeat::Random { var, from, to } => {
                let Some(name) = self.var(number, var) else { return };
                let range = i64::from(*to) - i64::from(*from) + 1;
                let parts = range.clamp(1, 10);
                self.line(&format!("switch ( random({range}) ) {{"));
                for part in 0..parts {
                    let value = i64::from(*from) + (part * range + range / 2) / parts;
                    match part + 1 < parts {
                        true => self.line(&format!("    case < {}:", (part + 1) * range / parts)),
                        false => self.line("    default:"),
                    }
                    self.line(&format!("        {name} = {value};"));
                    self.line("        break;");
                }
                self.line("}");
            }
            // A mark (`at name + 2s:`): a label; its times are already written as waits.
            DataBeat::Set { var, .. } if var.starts_with("_mark_") => self.label(&format!("mark_{}", &var["_mark_".len()..]), None),
            DataBeat::Set { var, .. } if var.starts_with("thread") => {}
            DataBeat::Set { var, value } => {
                let Some(name) = self.var(number, var) else { return };
                match value {
                    Value::Number(value) => {
                        self.values.insert(var.clone(), *value);
                        self.line(&format!("{name} = {};", *value as i64));
                    }
                    Value::Var(_) => self.problem(number, Kind::Logic, Fate::Impossible, "a variable set from another", "left out"),
                }
            }
            other => self.unexported(number, word, &Beat::Data(other.clone())),
        }
    }

    /// A choice: the question asked in the box of its line, its answers.
    pub(super) fn choice(&mut self, number: i32, word: Option<&str>, beat: &Course) {
        match beat {
            Course::Ask(options) => {
                // In the box of the question, as Sky asks.
                if let Some(at) = self.closed.take() {
                    self.body.truncate(at);
                }
                self.options.clone_from(options);
                self.line("switch ( message_SwitchMenu(0, 1) ) {");
                self.indent += 1;
                self.menus.push(0);
            }
            Course::Answer(answer) => {
                let Some(seen) = self.menus.last().copied() else { return };
                if seen > 0 {
                    self.line("break;");
                    self.indent -= 1;
                }
                let option = self.options.get(usize::from(*answer)).cloned().unwrap_or_default();
                self.line(&format!("case menu(\"{}\"):", quoted(&option)));
                self.indent += 1;
                if let Some(seen) = self.menus.last_mut() {
                    *seen += 1;
                }
            }
            Course::Rejoin => {
                let Some(seen) = self.menus.pop() else { return };
                if seen > 0 {
                    self.line("break;");
                    self.indent -= 1;
                }
                self.indent -= 1;
                self.line("}");
            }
            other => self.plain(number, word, &Beat::Course(other.clone())),
        }
    }

    /// A moment of the film (`moment: name`) as a label of the script; the
    /// scenes' and the threads' own are the export's.
    pub(super) fn label(&mut self, label: &str, word: Option<&str>) {
        // A fight's passages (what is said in its moments): kept for the fight.
        if fight_label(label) {
            if !label.ends_with("_past") {
                self.passage = Some((label.to_string(), self.body.len()));
            }
            return;
        }
        // A scene of the film: an acting scene of its own (`// ds-export level`).
        if label.starts_with("scene") && self.level.is_some() {
            self.next_scene();
            return;
        }
        if let Some(name) = label.strip_prefix("thread") {
            self.thread = !name.ends_with("_past");
            // (`beside`: each one's lines in a routine of its own.)
            match self.capture.as_ref() {
                Some(capture) if label == format!("{}_past", capture.thread) => self.end_beside(),
                // (Not `together`'s: each of its lines told at once is as Sky does.)
                None if self.thread && word != Some("together") && self.started => self.start_beside(label),
                _ => {}
            }
            return;
        }
        if label.starts_with("scene") {
            return;
        }
        self.line(&format!("@{};", label.trim_start_matches('_')));
    }

    /// A film's variable as one of the game's.
    pub(super) fn var(&mut self, number: i32, name: &str) -> Option<&'static str> {
        let at = match self.vars.iter().position(|one| one == name) {
            Some(at) => at,
            None => {
                self.vars.push(name.to_string());
                self.vars.len() - 1
            }
        };
        let var = LOCALS.get(at).copied();
        if var.is_none() {
            self.problem(number, Kind::Logic, Fate::Impossible, format!("“{name}”: a scene has {} variables at most on DS", LOCALS.len()), "left out");
        }
        var
    }

    pub(super) fn test(&mut self, number: i32, test: &Test) -> Option<String> {
        let mut any = Vec::new();
        for all in &test.any {
            let [condition] = all.as_slice() else {
                self.problem(number, Kind::Logic, Fate::Impossible, "“and” in a test (nest two ifs)", "left out");
                return None;
            };
            let var = self.var(number, &condition.var)?;
            let value = match &condition.value {
                Value::Number(value) => *value as i64,
                Value::Var(_) => {
                    self.problem(number, Kind::Logic, Fate::Impossible, "a test against a variable", "left out");
                    return None;
                }
            };
            let compare = match condition.compare {
                Compare::Equal => "==",
                Compare::Unequal => "!=",
                Compare::Less => "<",
                Compare::LessOrEqual => "<=",
                Compare::Greater => ">",
                Compare::GreaterOrEqual => ">=",
            };
            any.push(format!("{var} {compare} {value}"));
        }
        Some(any.join(" || "))
    }
}
