//! The blocks a line opens: `if`, `for each`, the ones beside the film that wait, `every`.

use language::ast::{Call, Clock, Name};

use super::*;

impl Lower<'_> {
    pub(super) fn block(&mut self, stmt: &Stmt, block: &Block, number: usize) {
        let end = self.end_number(stmt.span);
        match &block.opening {
            Opening::If(test) => self.if_block(block, test, number, end),
            Opening::Repeat(times) => self.repeat(block, times, number, end),
            Opening::ForEach { name, rank, list } => self.for_each(block, name, rank.as_ref(), list, number, end),
            Opening::While(test) => self.while_block(block, test, number, end),
            Opening::Together => self.wrapped("together".to_string(), block, number, end, true),
            Opening::Beside => self.wrapped(beside_opening(block), block, number, end, true),
            Opening::When(Event::Arrives(who)) => self.when_arrives(block, who, number, end),
            Opening::When(Event::Leaves(who)) => self.when_leaves(block, who, number),
            Opening::When(Event::Knocked(who)) => self.when_knocked(stmt, block, who, number),
            Opening::When(Event::Becomes(test)) => self.when_becomes(block, test, number),
            Opening::At { mark, clock: clock @ (Clock::Scene | Clock::Music | Clock::Here), .. } => self.at_clock(block, mark, *clock, number, end),
            Opening::At { mark, offset, .. } => self.at_mark(block, mark, offset.as_ref(), number, end),
            Opening::Every(every) => self.every(block, every, number, end),
            Opening::After(after) => self.after(block, after, number),
            Opening::Choice => self.choice(block, number, end),
            Opening::Fight(args) => self.fight(block, args, number),
            Opening::Word(call) => self.word_block(block, call, number, end),
            Opening::Recipe(recipe) => {
                let line = format!("define {} {}", recipe.form.word.text, elements_text(&recipe.form.elements)).trim_end().to_string();
                let doc = self.comments.join(" ");
                let kept = RecipeText { form: recipe.form.clone(), body: block.body.clone(), text: self.text.clone(), span: stmt.span, doc, number };
                self.made(number, line, Shape::Recipe(Arc::new(kept)));
            }
        }
    }

    /// `repeat n`: the engine's own if the count is known, else a loop
    /// counting its rounds.
    fn repeat(&mut self, block: &Block, times: &Expr, number: usize, end: usize) {
        match folded(times).map(|count| count.max(0.0).round().to_string()).or_else(|| self.given_number(times)) {
            Some(count) => {
                self.open(number, format!("repeat {count}"));
                let body = self.apart(&block.body);
                self.out.extend(body);
                self.close();
            }
            None => {
                let Some(count) = self.value(times, number) else { return };
                let round = self.temp();
                self.flow(number, crate::emit::set(&round, Value::Number(0.0)));
                self.flow(number, FlowBeat::While(compared(&round, Compare::Less, &count)));
                let body = self.apart(&block.body);
                self.out.extend(body);
                self.flow(end, crate::emit::calc(&round, Value::Var(round.clone()), "+", Value::Number(1.0)));
                self.flow(end, FlowBeat::EndWhile);
            }
        }
    }

    /// `while test`: the engine's own test, or one made each round and
    /// broken out of.
    fn while_block(&mut self, block: &Block, test: &Expr, number: usize, end: usize) {
        let simple = self.simple_test(test);
        match simple {
            Some(test) => self.flow(number, FlowBeat::While(test)),
            None => {
                self.flow(number, FlowBeat::While(compared("1", Compare::Equal, "1")));
                let Some(test) = self.test(test, number) else { return };
                self.flow(number, FlowBeat::If(test));
                self.flow(number, FlowBeat::Else);
                self.flow(number, FlowBeat::Break);
                self.flow(number, FlowBeat::EndIf);
            }
        }
        let body = self.apart(&block.body);
        self.out.extend(body);
        self.flow(end, FlowBeat::EndWhile);
    }

    /// `when a arrives`: anchored on his arrival.
    fn when_arrives(&mut self, block: &Block, who: &Name, number: usize, end: usize) {
        let named = self.put.get(&who.text).cloned().unwrap_or_else(|| who.text.clone());
        if named.contains(':') {
            self.error(who.span, format!("“when {named} arrives”: someone, by his name"));
            return;
        }
        let anchor = Anchor::Arrives(named.split_whitespace().map(str::to_string).collect());
        self.anchored(format!("when {named} arrives"), anchor, block, number, end);
    }

    /// `when a leaves`: beside the film, once his moves are done.
    fn when_leaves(&mut self, block: &Block, who: &Name, number: usize) {
        let who = self.put.get(&who.text).cloned().unwrap_or_else(|| who.text.clone());
        self.open(number, beside_opening(block));
        self.engine(number, format!("waitmoves {who}"));
        let body = self.apart(&block.body);
        self.out.extend(body);
        self.close();
    }

    /// `when a is knocked out`: none of his life left, as a fight tells it.
    fn when_knocked(&mut self, stmt: &Stmt, block: &Block, who: &Name, number: usize) {
        let who = self.put.get(&who.text).cloned().unwrap_or_else(|| who.text.clone());
        let body = self.apart(&block.body);
        let lines: Vec<(String, Shape)> = body
            .iter()
            .filter_map(|piece| match piece {
                Out::Line(_, line, shape) | Out::Open(_, line, shape) => Some((line.clone(), shape.clone())),
                Out::Close => None,
            })
            .collect();
        self.when_life(stmt.span, number, &who, "0", lines);
    }

    /// `when test`: a fight's turn, or beside the film, waiting until it holds.
    fn when_becomes(&mut self, block: &Block, test: &Expr, number: usize) {
        if let Some(fight) = fight_turn(test) {
            let body = self.apart(&block.body);
            let lines: Vec<(String, Shape)> = body
                .iter()
                .filter_map(|piece| match piece {
                    Out::Line(_, line, shape) => Some((line.clone(), shape.clone())),
                    _ => None,
                })
                .collect();
            self.when_life(test.span, number, &fight.0, &fight.1, lines);
            return;
        }
        self.open(number, beside_opening(block));
        match self.simple_test(test) {
            Some(test) => self.flow(number, FlowBeat::WaitUntil(test)),
            None => {
                self.flow(number, FlowBeat::While(compared("1", Compare::Equal, "1")));
                let Some(test) = self.test(test, number) else { return };
                self.flow(number, FlowBeat::If(test));
                self.flow(number, FlowBeat::Break);
                self.flow(number, FlowBeat::EndIf);
                self.flow(number, FlowBeat::Wait(Frames(1.0)));
                self.flow(number, FlowBeat::EndWhile);
            }
        }
        let body = self.apart(&block.body);
        self.out.extend(body);
        self.close();
    }

    /// `at 12s`, `at music 12s`, `at +2s`: the scene's clock, the music's, from here.
    fn at_clock(&mut self, block: &Block, mark: &Name, clock: Clock, number: usize, end: usize) {
        let music = clock == Clock::Music;
        let here = clock == Clock::Here;
        let head = match (music, here) {
            (true, _) => format!("at music {}", mark.text),
            (_, true) => format!("at +{}", mark.text),
            _ => format!("at {}", mark.text),
        };
        let Some(at) = crate::words::frames_of(&mark.text) else {
            self.error(mark.span, format!("“{head}”: a written duration (“12s”)"));
            return;
        };
        let clock = match (music, here) {
            (true, _) => crate::ir::MUSIC_CLOCK,
            (_, true) => crate::ir::HERE_CLOCK,
            _ => crate::ir::SCENE_CLOCK,
        };
        self.anchored(head, Anchor::Mark(clock.to_string(), at), block, number, end);
    }

    /// `at mark [± duration]`: anchored on a mark of the scene.
    fn at_mark(&mut self, block: &Block, mark: &Name, offset: Option<&(bool, Expr)>, number: usize, end: usize) {
        let head = match offset {
            Some((minus, by)) => match folded_duration(by) {
                Some(by) => format!("at {} {} {by}", mark.text, if *minus { "-" } else { "+" }),
                None => {
                    self.error(by.span, "after “at mark +”, a written duration (“2s”)");
                    return;
                }
            },
            None => format!("at {}", mark.text),
        };
        // (A time is no mark: the language says `at 3s:` is gone.)
        if crate::words::in_seconds(&mark.text) || crate::words::digits(&mark.text) {
            return;
        }
        if mark.text.contains(':') {
            self.error(mark.span, format!("“{head}”: a mark is named by a word (“mark m”, then “at m + 2s : …”)"));
            return;
        }
        let after = match offset {
            None => Some(0),
            Some((false, by)) => folded_duration(by).and_then(|by| crate::words::frames_of(&by)),
            Some((true, _)) => None,
        };
        let Some(after) = after else {
            self.error(mark.span, format!("“{head}”: nothing plays before a mark; “at {} + …” after it, or a mark earlier", mark.text));
            return;
        };
        self.anchored(head, Anchor::Mark(crate::words::plain(&mark.text), after), block, number, end);
    }

    /// `after duration`: beside the film, once so long has gone.
    fn after(&mut self, block: &Block, after: &Expr, number: usize) {
        let Some(wait) = self.frames(after, number) else { return };
        self.open(number, beside_opening(block));
        // (A time found as it plays: its number by its name.)
        self.wait_of(number, &wait);
        let body = self.apart(&block.body);
        self.out.extend(body);
        self.close();
    }

    /// `choice` … `option …` … `end`: the answers offered, then each one's lines.
    fn choice(&mut self, block: &Block, number: usize, end: usize) {
        let answers: Vec<(&str, Option<&Expr>, bool)> = block
            .parts
            .iter()
            .filter_map(|part| match &part.kind {
                PartKind::Option { words, test, once, .. } => Some((words.as_str(), test.as_ref(), *once)),
                _ => None,
            })
            .collect();
        if answers.is_empty() {
            self.error(block.opening_span, "a “choice” has its answers: a line “option …” each");
            return;
        }
        if let Some((wrong, ..)) = answers.iter().find(|(words, ..)| words.contains('|')) {
            self.error(block.opening_span, format!("an answer holds no “|”: “{wrong}”"));
            return;
        }
        let Some((options, taken)) = self.choice_options(&answers, number) else { return };
        let shape = Shape::Choice { options: options.clone() };
        self.made(number, format!("choice {}", options.join(" | ")), shape);
        let before = self.apart(&block.body);
        self.out.extend(before);
        for (rank, part) in block.parts.iter().enumerate() {
            self.word(self.number(part.span), "_answer", vec![(rank + 1).to_string()]);
            if let Some(var) = taken.get(rank).cloned().flatten() {
                self.flow(self.number(part.span), crate::emit::set(var, Value::Number(1.0)));
            }
            let body = self.apart(&part.body);
            self.out.extend(body);
        }
        self.word(end, "end", Vec::new());
    }

    /// A choice's answers as the engine writes them, and the number of each
    /// one taken `once`: an answer offered only when its test holds,
    /// `[if apples > 0]Give an apple`; once taken, never again if `once`.
    fn choice_options(&mut self, answers: &[(&str, Option<&Expr>, bool)], number: usize) -> Option<(Vec<String>, Vec<Option<String>>)> {
        let mut options = Vec::new();
        let mut taken = Vec::new();
        for (rank, (words, test, once)) in answers.iter().enumerate() {
            let once = once.then(|| format!("_once_{number}_{}", rank + 1));
            let mut asked = match test {
                Some(test) => self.test(test, number)?,
                None => Test { any: Vec::new() },
            };
            if let Some(var) = &once {
                let not_yet = Condition { var: var.clone(), compare: Compare::Equal, value: Value::Number(0.0) };
                match asked.any.is_empty() {
                    true => asked.any.push(vec![not_yet]),
                    false => asked.any.iter_mut().for_each(|all| all.push(not_yet.clone())),
                }
            }
            let words = words.trim();
            options.push(match test_words(&asked) {
                Some(test) => format!("[if {test}]{words}"),
                None => words.to_string(),
            });
            taken.push(once);
        }
        Some((options, taken))
    }

    /// `fight directed …`: its lines held, a fight's.
    fn fight(&mut self, block: &Block, args: &[Arg], number: usize) {
        let mut line = "fight directed".to_string();
        for arg in args {
            line.push(' ');
            line.push_str(&self.arg(arg, number));
        }
        self.open(number, line);
        self.fighting += 1;
        let body = self.apart(&block.body);
        self.fighting -= 1;
        self.out.extend(body);
        self.close();
    }

    /// A word that holds its lines (`memory`, `pikachu dimensional_scream tower`).
    fn word_block(&mut self, block: &Block, call: &Call, number: usize, end: usize) {
        // (Who first, `pikachu dimensional_scream tower`: read as the word first.)
        let subject = self.lexicon.subject_first(&call.word.text, call.args.first().map(|arg| arg.text.as_str()), &self.text);
        let (mut line, rest, word) = match subject {
            true => (format!("{} {}", call.args[0].text, self.given_word(&call.word.text)), &call.args[1..], call.args[0].text.as_str()),
            false => (call.word.text.clone(), &call.args[..], call.word.text.as_str()),
        };
        for arg in rest {
            line.push(' ');
            line.push_str(&self.arg(arg, number));
        }
        // (`keys`: its keys are its own lines, held by it.)
        let grouped = word == "keys";
        self.wrapped(line, block, number, end, grouped);
    }

    /// `if` … `else if` … `else` … `end`; in a recipe, `if name` (a name
    /// it may be given) keeps its lines only if it was given.
    pub(super) fn if_block(&mut self, block: &Block, test: &Expr, number: usize, end: usize) {
        let given = |this: &Self, test: &Expr| -> Option<(String, bool)> {
            let recipe = this.recipe.as_ref()?;
            match &test.kind {
                ExprKind::Name(name) if recipe.params.get(name).is_some_and(|(optional, ..)| *optional) => Some((name.clone(), true)),
                ExprKind::Unary(UnOp::Not, inner) => match &inner.kind {
                    ExprKind::Name(name) if recipe.params.get(name).is_some_and(|(optional, ..)| *optional) => Some((name.clone(), false)),
                    _ => None,
                },
                _ => None,
            }
        };
        if let Some((name, must)) = given(self, test) {
            let was = self.given_value(&name).is_some_and(|value| !value.words.is_empty());
            let kept = if was == must { Some(&block.body) } else { block.parts.iter().find(|part| matches!(part.kind, PartKind::Else)).map(|part| &part.body) };
            if let Some(kept) = kept {
                let body = self.apart(kept);
                self.out.extend(body);
            }
            let _ = end;
            return;
        }
        // (Every test made before the `if`: an `elif` reads what is made.)
        let mut tests = vec![self.test(test, number)];
        for part in &block.parts {
            if let PartKind::ElseIf(other) = &part.kind {
                tests.push(self.test(other, number));
            }
        }
        let Some(tests) = tests.into_iter().collect::<Option<Vec<Test>>>() else { return };
        self.flow(number, FlowBeat::If(tests[0].clone()));
        let body = self.apart(&block.body);
        self.out.extend(body);
        let mut rest = tests[1..].iter();
        for part in &block.parts {
            let at = self.number(part.span);
            match &part.kind {
                PartKind::ElseIf(_) => self.flow(at, FlowBeat::ElseIf(rest.next().cloned().unwrap_or(Test { any: Vec::new() }))),
                _ => self.flow(at, FlowBeat::Else),
            }
            let body = self.apart(&part.body);
            self.out.extend(body);
        }
        self.flow(end, FlowBeat::EndIf);
    }

    /// A test the engine asks with nothing made first.
    pub(super) fn simple_test(&mut self, test: &Expr) -> Option<Test> {
        let before = (self.out.len(), self.made);
        let read = self.test(test, 0);
        if self.out.len() != before.0 {
            self.out.truncate(before.0);
            self.made = before.1;
            return None;
        }
        read
    }

    pub(super) fn for_each(&mut self, block: &Block, name: &language::ast::Name, rank: Option<&language::ast::Name>, list: &Expr, number: usize, end: usize) {
        match &list.kind {
            ExprKind::List(items) => {
                for (at, item) in items.iter().enumerate() {
                    let Some(stands) = self.given(item, number) else { return };
                    let kept = self.put.clone();
                    self.put.insert(name.text.clone(), stands);
                    if let Some(rank) = rank {
                        self.put.insert(rank.text.clone(), (at + 1).to_string());
                    }
                    let body = self.apart(&block.body);
                    self.out.extend(body);
                    self.put = kept;
                }
            }
            ExprKind::Name(team) if self.recipe.as_ref().is_some_and(|recipe| recipe.params.get(team).is_some_and(|(_, many, _)| *many)) => {
                // (Each of those the call gave, in turn, his rank with him.)
                let members = self.given_value(team).map(|value| value.words.clone()).unwrap_or_default();
                for (at, member) in members.into_iter().enumerate() {
                    let (kept, kept_rank) = (self.put.clone(), self.rank);
                    self.put.insert(name.text.clone(), member);
                    if let Some(rank) = rank {
                        self.put.insert(rank.text.clone(), (at + 1).to_string());
                    }
                    self.rank = at;
                    let body = self.apart(&block.body);
                    self.out.extend(body);
                    (self.put, self.rank) = (kept, kept_rank);
                }
                let _ = end;
            }
            _ => self.error(list.span, "“for each” goes through a written list (“[a, b, c]”) or those a recipe is given (“*who”)"),
        }
    }

    /// A block: its opening, its lines; `grouped`, held by its opening (the
    /// language's), else up to its `end`, which the word that opens it
    /// closes as the scene plays (`memory` … `end`).
    pub(super) fn wrapped(&mut self, opening: String, block: &Block, number: usize, end: usize, grouped: bool) {
        if grouped {
            self.open(number, opening);
        } else {
            self.line_word(number, opening);
        }
        let body = self.apart(&block.body);
        self.out.extend(body);
        if grouped {
            self.close();
        } else {
            self.word(end, "end", Vec::new());
        }
    }

    /// `when a arrives`, `at mark + 2s`: on its line if it is one line.
    pub(super) fn anchored(&mut self, head: String, anchor: Anchor, block: &Block, number: usize, end: usize) {
        let body = self.apart(&block.body);
        let lines = played(&body);
        if block.short
            && lines.len() == 1
            && matches!(body.iter().find(|piece| !matches!(piece, Out::Line(_, line, _) if line.is_empty())), Some(Out::Line(..)))
        {
            let line = lines[0].1.to_string();
            let text = format!("{head} : {line}");
            // (The line after the colon, as it was made.)
            let shape = body.iter().find_map(|piece| match piece {
                Out::Line(_, said, shape) if *said == line => Some(shape.clone()),
                _ => None,
            });
            let then = Instr { number, text: line.trim().to_string(), shape: shape.unwrap_or_default(), inner: None, span: None, from: Default::default() };
            self.made(number, text, anchor.shape(Some(Box::new(then))));
            return;
        }
        let _ = end;
        self.out.push(Out::Open(self.at(number), format!("{head} :"), anchor.shape(None)));
        self.out.extend(body);
        self.close();
    }

    /// A number a recipe is given, which its call writes before the lines
    /// are played (`repeat hops`).
    pub(super) fn given_number(&self, value: &Expr) -> Option<String> {
        match &value.kind {
            ExprKind::Name(name) => self.given_value(name).map(|value| value.words.join(" ")),
            _ => None,
        }
    }

    /// A duration as frames for the engine: written, or reckoned.
    pub(super) fn frames(&mut self, value: &Expr, number: usize) -> Option<String> {
        if let Some(frames) = folded_duration_frames(value) {
            return Some(number_text(frames.round()));
        }
        if matches!(value.kind, ExprKind::Number(_)) {
            self.error(value.span, "a duration has its unit: “2s”, or “6f” in frames");
            return None;
        }
        if let ExprKind::Name(name) = &value.kind
            && self.variable(name).is_none()
            && self.given_number(value).is_none()
        {
            self.error(value.span, format!("“{name}”: a duration is expected (“2s”)"));
            return None;
        }
        self.value(value, number)
    }

    /// `every duration [as name]`: one line every so often, or a block.
    pub(super) fn every(&mut self, block: &Block, every: &Expr, number: usize, end: usize) {
        let body = self.apart(&block.body);
        let lines = played(&body);
        let alone = !body.iter().any(|piece| matches!(piece, Out::Open(..)));
        // (One line every so often, unnamed: the engine's own `every`.)
        let written = match &every.kind {
            ExprKind::Call(called, args) if called.text == "random" && args.len() == 2 => {
                folded_duration(&args[0]).zip(folded_duration(&args[1])).map(|(low, high)| format!("{low}~{high}"))
            }
            _ => folded_duration(every),
        };
        // (Only a word's line, made at once: another, a line said, plays in
        // a block of its own, below.)
        let word = body.iter().find_map(|piece| match piece {
            Out::Line(_, _, Shape::Word { verb, words, wait: true }) => Some((verb.clone(), words.clone())),
            _ => None,
        });
        if let (Some(written), [_], None, Some((verb, words))) = (&written, lines.as_slice(), &block.named, word)
            && alone
        {
            self.word(number, "every", [written.clone(), verb].into_iter().chain(words).collect());
            return;
        }
        let name = block.named.as_ref().map_or_else(
            || {
                self.made += 1;
                format!("every{}", self.made)
            },
            |name| name.text.clone(),
        );
        let active = format!("_active_{name}");
        self.flow(number, crate::emit::set(&active, Value::Number(1.0)));
        self.open(number, format!("beside {name}"));
        self.flow(number, FlowBeat::While(compared(&active, Compare::Equal, "1")));
        let Some(wait) = self.frames(every, number) else { return };
        self.wait_of(number, &wait);
        self.out.extend(body);
        self.flow(end, FlowBeat::EndWhile);
        self.close();
    }
}

/// A test as an answer of a choice writes it, `apples > 0 and met != 0`
/// (the engine's `[if …]`): none if it asks nothing.
fn test_words(test: &Test) -> Option<String> {
    let mut words = Vec::new();
    for (at, all) in test.any.iter().enumerate() {
        if at > 0 {
            words.push("or".to_string());
        }
        for (at, condition) in all.iter().enumerate() {
            if at > 0 {
                words.push("and".to_string());
            }
            let compare = match condition.compare {
                Compare::Equal => "==",
                Compare::Unequal => "!=",
                Compare::Less => "<",
                Compare::LessOrEqual => "<=",
                Compare::Greater => ">",
                Compare::GreaterOrEqual => ">=",
            };
            let value = match &condition.value {
                Value::Number(number) => number_text(*number),
                Value::Var(name) => name.clone(),
            };
            words.extend([condition.var.clone(), compare.to_string(), value]);
        }
    }
    (!words.is_empty()).then(|| words.join(" "))
}

/// A block beside the film: `beside`, or `beside name` if it is named.
fn beside_opening(block: &Block) -> String {
    block.named.as_ref().map_or("beside".to_string(), |name| format!("beside {}", name.text))
}
