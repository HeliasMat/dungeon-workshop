//! The words of a line and the values they give: a recipe's reckonings, the calculations, the tests.

use super::*;

impl Lower<'_> {
    /// A word of a line, put for what it stands for in a block played out.
    pub(super) fn arg(&mut self, arg: &Arg, number: usize) -> String {
        match &arg.kind {
            ArgKind::Value(value) => {
                if let Some(folded) = folded(value) {
                    return number_text(folded.round());
                }
                if self.reckoned_by_the_recipe(value) {
                    return self.reckoned(value, number);
                }
                match self.value(value, number) {
                    Some(field) if field.parse::<f64>().is_ok() => field,
                    Some(field) => format!("{{{field}}}"),
                    None => arg.text.clone(),
                }
            }
            _ => self.given_word(&arg.text),
        }
    }

    /// A word of a recipe's line: what its call gave the name it is (`who`),
    /// or one of them (`who.1`, the first); another word as it is.
    pub(super) fn given_word(&self, word: &str) -> String {
        if let Some(put) = self.put.get(word) {
            return put.clone();
        }
        if let Some((name, rank)) = word.split_once('.').and_then(|(name, rank)| Some((name, rank.parse::<usize>().ok()?)))
            && let Some(one) = self.given_value(name).and_then(|value| value.words.get(rank.saturating_sub(1)))
        {
            return one.clone();
        }
        word.to_string()
    }

    /// What the call gave that name.
    pub(super) fn given_value(&self, name: &str) -> Option<&CallValue> {
        self.call.as_ref()?.values.iter().find(|value| value.name == name)
    }

    /// A recipe's reckoning, worked out with what its call gives: someone
    /// given as the game writes him (`{who}`), a number (a time as frames,
    /// `90f`), or the engine's number worked out as it plays (`{_c3_1}`).
    pub(super) fn reckoned(&mut self, value: &Expr, number: usize) -> String {
        if let ExprKind::Name(name) = &value.kind
            && let Some(given) = self.given_value(name)
        {
            return given.shown.clone();
        }
        match self.reckon(value, number) {
            Some(Operand::Number(number, true)) => format!("{}f", number.round() as i64),
            Some(Operand::Number(number, false)) if number.fract() == 0.0 => (number as i64).to_string(),
            Some(Operand::Number(number, false)) => format!("{number:.2}"),
            Some(Operand::Var(var)) => format!("{{{var}}}"),
            None => format!("{{{}}}", expression(value).replace(' ', "")),
        }
    }

    /// A calculation's value, each sign as tight as it binds: a number
    /// worked out now, or the engine's number the lines before work out.
    pub(super) fn reckon(&mut self, value: &Expr, number: usize) -> Option<Operand> {
        use language::ast::Duration;
        Some(match &value.kind {
            ExprKind::Number(value) => Operand::Number(*value, false),
            ExprKind::Duration(Duration::Seconds(seconds)) => Operand::Number((seconds * 60.0).round(), true),
            ExprKind::Duration(Duration::Frames(frames)) => Operand::Number(*frames, true),
            ExprKind::Name(name) => self.operand(name)?,
            ExprKind::Field(base, field) => match &base.kind {
                ExprKind::Name(name) => self.place_side(name, &field.text)?,
                _ => return None,
            },
            ExprKind::Unary(UnOp::Minus, inner) => {
                let inner = self.reckon(inner, number)?;
                self.combined(Operand::Number(0.0, false), "-", inner, number)
            }
            ExprKind::Binary(op @ (BinOp::Plus | BinOp::Minus | BinOp::Times | BinOp::Divide), left, right) => {
                let (left, right) = (self.reckon(left, number)?, self.reckon(right, number)?);
                self.combined(left, op.sign(), right, number)
            }
            // (`div`, `min`, `max`, `abs`, `round` of numbers known now: worked out now.)
            ExprKind::Call(called, args) if matches!(called.text.as_str(), "div" | "min" | "max" | "abs" | "round") => {
                let mut known = Vec::new();
                for arg in args {
                    match self.reckon(arg, number)? {
                        Operand::Number(value, timed) => known.push((value, timed)),
                        Operand::Var(_) => return None,
                    }
                }
                let timed = known.iter().any(|(_, timed)| *timed);
                let worked = match (called.text.as_str(), known.as_slice()) {
                    ("div", [(a, _), (b, _)]) if *b != 0.0 => (a / b).trunc(),
                    ("div", [_, _]) => 0.0,
                    ("min", [(a, _), (b, _)]) => a.min(*b),
                    ("max", [(a, _), (b, _)]) => a.max(*b),
                    ("abs", [(a, _)]) => a.abs(),
                    ("round", [(a, _)]) => a.round(),
                    _ => return None,
                };
                Operand::Number(worked, timed)
            }
            _ => return None,
        })
    }

    /// A name in a reckoning: its rank in a team played out (`rank`), what
    /// the call gave it read as a number, a time (`90f`, `1.5s`), a number.
    pub(super) fn operand(&self, word: &str) -> Option<Operand> {
        if word == "rank" {
            return Some(Operand::Number(self.rank as f64, false));
        }
        if let Some(given) = self.given_value(word) {
            return self.operand(given.words.first()?);
        }
        if let Some(frames) = word.strip_suffix('f').and_then(|frames| frames.parse::<f64>().ok()) {
            return Some(Operand::Number(frames, true));
        }
        if word.ends_with('s') {
            return crate::words::frames_of(word).map(|frames| Operand::Number(frames as f64, true));
        }
        word.parse::<f64>().ok().map(|number| Operand::Number(number, false))
    }

    /// A place given, one of its sides (`where.x`): a number if it was given
    /// in numbers, else the point the film finds as it plays.
    pub(super) fn place_side(&self, name: &str, side: &str) -> Option<Operand> {
        let place = self.given_value(name)?;
        let id = self.call.as_ref()?.id;
        Some(match place.words.as_slice() {
            [x, y] if x.parse::<f64>().is_ok() && y.parse::<f64>().is_ok() => {
                Operand::Number(if side == "x" { x.parse().ok()? } else { y.parse().ok()? }, false)
            }
            _ => Operand::Var(format!("_p{id}_{}_{side}", place.name)),
        })
    }

    /// Two values put together by a sign: now if both are numbers, else a
    /// number of the engine's worked out by a line before (`calc`).
    pub(super) fn combined(&mut self, a: Operand, op: &str, b: Operand, number: usize) -> Operand {
        match (a, b) {
            (Operand::Number(a, timed), Operand::Number(b, other)) => Operand::Number(
                match op {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    _ if b != 0.0 => a / b,
                    _ => 0.0,
                },
                timed || other,
            ),
            (a, b) => {
                self.reckoned += 1;
                let id = self.call.as_ref().map_or(0, |call| call.id);
                let var = format!("_c{id}_{}", self.reckoned);
                self.word(number, "calc", vec![var.clone(), a.engine(), op.to_string(), b.engine()]);
                Operand::Var(var)
            }
        }
    }

    /// `{value}` in a said text: a recipe's name as the game writes it, a
    /// number filled as it is said.
    pub(super) fn filled(&mut self, value: &Expr, number: usize) -> String {
        if let ExprKind::Name(name) = &value.kind {
            // (A name the recipe was given, as the game writes it; one a block
            // played out stands for, as it is.)
            if let Some(given) = self.given_value(name).filter(|given| self.put.get(name) == Some(&given.words.join(" "))) {
                return given.shown.clone();
            }
            if let Some(put) = self.put.get(name) {
                return put.clone();
            }
        }
        if self.reckoned_by_the_recipe(value) {
            return self.reckoned(value, number);
        }
        match self.value(value, number) {
            Some(field) if field.parse::<f64>().is_ok() => field,
            Some(field) => format!("{{{field}}}"),
            None => String::new(),
        }
    }

    /// Whether the recipe's own reckoning reads it as the film is written:
    /// its names given to the recipe (and `rank`), `+ - * /` and their
    /// brackets (worked out as tight as each binds).
    pub(super) fn reckoned_by_the_recipe(&self, value: &Expr) -> bool {
        let Some(recipe) = &self.recipe else { return false };
        let mut names = Vec::new();
        let mut signs = Vec::new();
        if !walk(value, &mut names, &mut signs) {
            return false;
        }
        // (A place given has its `.x`; someone's is found as the film plays.)
        let read = |name: &String| match name.split_once('.') {
            Some((base, _)) => recipe.params.get(base).is_some_and(|(_, _, ty)| ty == "place"),
            None => name == "rank" || recipe.params.contains_key(name),
        };
        !names.is_empty() && names.iter().all(read)
    }

    /// Whether a reckoning of a recipe reads only numbers known now: written
    /// ones and those its call gives, with `+ - * /`, `div`, `min`, `max`,
    /// `abs`, `round`.
    fn known_now(&self, value: &Expr) -> bool {
        match &value.kind {
            ExprKind::Number(_) | ExprKind::Duration(_) => true,
            ExprKind::Name(name) => self.call.is_some() && self.variable(name).is_none() && self.operand(name).is_some(),
            ExprKind::Unary(UnOp::Minus, inner) => self.known_now(inner),
            ExprKind::Binary(BinOp::Plus | BinOp::Minus | BinOp::Times | BinOp::Divide, left, right) => self.known_now(left) && self.known_now(right),
            ExprKind::Call(called, args) => {
                matches!(called.text.as_str(), "div" | "min" | "max" | "abs" | "round") && args.iter().all(|arg| self.known_now(arg))
            }
            _ => false,
        }
    }

    /// A calculation as one field of the engine's: a number, or the name of
    /// the number made to hold it (the lines that make it written first).
    pub(super) fn value(&mut self, value: &Expr, number: usize) -> Option<String> {
        if let Some(folded) = folded(value) {
            return Some(number_text(folded));
        }
        // (In a recipe, a reckoning of the numbers its call gives, `sum -
        // div(sum * 9, 10)`: worked out now.)
        if self.known_now(value)
            && let Some(Operand::Number(worked, _)) = self.reckon(value, number)
        {
            return Some(number_text(worked));
        }
        Some(match &value.kind {
            ExprKind::Number(_) | ExprKind::Duration(_) | ExprKind::Bool(_) => unreachable!("folded"),
            ExprKind::Nothing => "0".to_string(),
            ExprKind::Name(name) => {
                if let Some(put) = self.put.get(name) {
                    return Some(put.clone());
                }
                if let Some(var) = self.variable(name) {
                    return Some(var);
                }
                if self.recipe.as_ref().is_some_and(|recipe| recipe.params.contains_key(name)) || THE_GAMES.contains(&name.as_str()) {
                    return Some(name.clone());
                }
                // (Someone, compared by his name: `target = eevee`.)
                name.clone()
            }
            ExprKind::Unary(UnOp::Minus, inner) => {
                let inner = self.value(inner, number)?;
                let made = self.temp();
                self.flow(number, crate::emit::calc(&made, Value::Number(0.0), "-", operand(&inner)));
                made
            }
            ExprKind::Binary(op @ (BinOp::Plus | BinOp::Minus | BinOp::Times | BinOp::Divide | BinOp::Rest), left, right) => {
                let (left, right) = (self.value(left, number)?, self.value(right, number)?);
                let made = self.temp();
                self.flow(number, crate::emit::calc(&made, operand(&left), op.sign(), operand(&right)));
                made
            }
            ExprKind::Unary(UnOp::Not, _) | ExprKind::Binary(..) => {
                let test = self.test(value, number)?;
                let made = self.temp();
                self.flow(number, crate::emit::set(&made, Value::Number(0.0)));
                self.flow(number, FlowBeat::If(test));
                self.flow(number, crate::emit::set(&made, Value::Number(1.0)));
                self.flow(number, FlowBeat::EndIf);
                made
            }
            ExprKind::Field(base, field) => {
                let ExprKind::Name(who) = &base.kind else {
                    self.error(value.span, "“.x” and “.y” are read on someone: “a.x”");
                    return None;
                };
                if !matches!(field.text.as_str(), "x" | "y") {
                    self.error(field.span, format!("“.{}”: only “.x” and “.y” are read", field.text));
                    return None;
                }
                let who = self.put.get(who).cloned().unwrap_or_else(|| who.clone());
                let (x, y) = (self.temp(), self.temp());
                self.engine(number, format!("position {who} {x} {y}"));
                if field.text == "x" { x } else { y }
            }
            ExprKind::Call(called, args) => return self.called(value.span, &called.text, args, number),
            ExprKind::Text(_) | ExprKind::List(_) | ExprKind::Point(..) | ExprKind::Index(..) => {
                self.error(value.span, format!("“{}” gives no number here", expression(value)));
                return None;
            }
            ExprKind::Unreadable(_) => return None,
        })
    }

    pub(super) fn called(&mut self, span: Span, called: &str, args: &[Expr], number: usize) -> Option<String> {
        let mut fields = Vec::new();
        match called {
            // (A countdown named: the number that holds its seconds left.)
            "timer" => {
                return match args {
                    [Expr { kind: ExprKind::Name(name), .. }] => Some(format!("timer_{name}")),
                    _ => {
                        self.error(span, "“timer” takes the name of a countdown: “timer(bomb)”");
                        None
                    }
                };
            }
            // (How many times a moment or a scene was played: the number that counts it.)
            "seen" => {
                return match args {
                    [Expr { kind: ExprKind::Name(name), span: at }] => {
                        let slug = crate::plot::slug(name);
                        if !self.story.is_empty() && !self.story.contains(&slug) {
                            let known = self.story.join(", ");
                            self.error(*at, format!("“seen({name})”: no moment nor scene of that name in the film (its moments and places: {known})"));
                            return None;
                        }
                        Some(crate::plot::seen_var(&slug))
                    }
                    _ => {
                        self.error(span, "“seen” takes the name of a moment or a scene: “seen(the_bet)”");
                        None
                    }
                };
            }
            "chance" | "random" | "min" | "max" | "div" | "abs" | "distance" => {
                for arg in args {
                    fields.push(if called == "distance" { self.someone(arg)? } else { self.value(arg, number)? });
                }
            }
            _ => {}
        }
        let arity = |wanted: usize, this: &mut Self| {
            if fields.len() != wanted {
                this.error(span, format!("“{called}” takes {wanted} value{}", if wanted > 1 { "s" } else { "" }));
                return false;
            }
            true
        };
        let made = match called {
            "chance" => {
                if !arity(1, self) {
                    return None;
                }
                let (roll, made) = (self.temp(), self.temp());
                self.flow(number, DataBeat::Random { var: roll.clone(), from: 0, to: 99 });
                self.flow(number, crate::emit::set(&made, Value::Number(0.0)));
                self.flow(number, FlowBeat::If(compared(&roll, Compare::Less, &fields[0])));
                self.flow(number, crate::emit::set(&made, Value::Number(1.0)));
                self.flow(number, FlowBeat::EndIf);
                made
            }
            "random" => {
                if !arity(2, self) {
                    return None;
                }
                let (Ok(from), Ok(to)) = (fields[0].parse::<i32>(), fields[1].parse::<i32>()) else {
                    self.error(span, "“random” draws between two whole numbers written (“random(1, 6)”)");
                    return None;
                };
                let made = self.temp();
                self.flow(number, DataBeat::Random { var: made.clone(), from: from.min(to), to: from.max(to) });
                made
            }
            "min" | "max" | "div" => {
                if !arity(2, self) {
                    return None;
                }
                let made = self.temp();
                self.flow(number, crate::emit::calc(&made, operand(&fields[0]), called, operand(&fields[1])));
                made
            }
            "abs" => {
                if !arity(1, self) {
                    return None;
                }
                let (minus, made) = (self.temp(), self.temp());
                self.flow(number, crate::emit::calc(&minus, Value::Number(0.0), "-", operand(&fields[0])));
                self.flow(number, crate::emit::calc(&made, operand(&fields[0]), "max", Value::Var(minus)));
                made
            }
            "distance" => {
                if !arity(2, self) {
                    return None;
                }
                let made = self.temp();
                self.engine(number, format!("measure {made} {} {}", fields[0], fields[1]));
                made
            }
            _ => {
                // (A recipe that gives back a value: called on its line, then read.)
                let mut line = called.to_string();
                for arg in args {
                    line.push(' ');
                    line.push_str(&self.given(arg, number)?);
                }
                self.line_word(number, line);
                format!("_return_{called}")
            }
        };
        Some(made)
    }

    /// Someone, as a calculation names him.
    pub(super) fn someone(&mut self, value: &Expr) -> Option<String> {
        match &value.kind {
            ExprKind::Name(name) => Some(self.put.get(name).cloned().unwrap_or_else(|| name.clone())),
            _ => {
                self.error(value.span, "someone is expected here, by his name");
                None
            }
        }
    }

    /// What a recipe called in a calculation is given, as its line writes it.
    pub(super) fn given(&mut self, arg: &Expr, number: usize) -> Option<String> {
        match &arg.kind {
            ExprKind::Point(x, y) => Some(format!("{} {}", self.value(x, number)?, self.value(y, number)?)),
            ExprKind::Duration(duration) => Some(duration.written()),
            ExprKind::Name(name) if self.variable(name).is_none() => Some(self.put.get(name).cloned().unwrap_or_else(|| name.clone())),
            ExprKind::Text(text) => Some(text.clone()),
            _ => {
                let field = self.value(arg, number)?;
                Some(if field.parse::<f64>().is_ok() { field } else { format!("{{{field}}}") })
            }
        }
    }

    /// A test as the engine asks it (`a < 3 and b`: its comparisons, all
    /// of them or any), the numbers it needs made first.
    pub(super) fn test(&mut self, value: &Expr, number: usize) -> Option<Test> {
        match &value.kind {
            ExprKind::Binary(op @ (BinOp::Less | BinOp::LessEq | BinOp::More | BinOp::MoreEq | BinOp::Equal | BinOp::NotEqual), left, right) => {
                let (left, right) = (self.value(left, number)?, self.value(right, number)?);
                let compare = match op {
                    BinOp::Less => Compare::Less,
                    BinOp::LessEq => Compare::LessOrEqual,
                    BinOp::More => Compare::Greater,
                    BinOp::MoreEq => Compare::GreaterOrEqual,
                    BinOp::Equal => Compare::Equal,
                    _ => Compare::Unequal,
                };
                Some(compared(&left, compare, &right))
            }
            ExprKind::Binary(BinOp::And, left, right) if !has_or(left) && !has_or(right) => {
                let (left, right) = (self.test(left, number)?, self.test(right, number)?);
                // (Neither holds an `or`: each one set of comparisons, all of them.)
                Some(Test { any: vec![left.any.into_iter().chain(right.any).flatten().collect()] })
            }
            ExprKind::Binary(BinOp::Or, left, right) => {
                let (left, right) = (self.test(left, number)?, self.test(right, number)?);
                Some(Test { any: left.any.into_iter().chain(right.any).collect() })
            }
            ExprKind::Unary(UnOp::Not, inner) => {
                let inner = self.value(inner, number)?;
                Some(compared(&inner, Compare::Equal, "0"))
            }
            ExprKind::Bool(true) => Some(compared("1", Compare::Equal, "1")),
            ExprKind::Bool(false) => Some(compared("1", Compare::Equal, "0")),
            _ => {
                let field = self.value(value, number)?;
                Some(compared(&field, Compare::Unequal, "0"))
            }
        }
    }
}
