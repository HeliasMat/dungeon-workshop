//! A film's lines said: its head, each statement, the names given a value.

use super::*;

impl Lower<'_> {
    pub(super) fn file(&mut self, file: &File) {
        for stmt in &file.head {
            match &stmt.kind {
                Kind::Let { .. } | Kind::Set { .. } => {
                    let before = std::mem::take(&mut self.out);
                    self.stmt(stmt);
                    let made = std::mem::replace(&mut self.out, before);
                    self.pending.extend(made);
                }
                _ => self.stmt(stmt),
            }
            self.remember_comment(stmt);
        }
        for scene in &file.scenes {
            let number = self.number(scene.span);
            let args: Vec<String> = scene.args.iter().map(|arg| arg.text.clone()).collect();
            let shape = Shape::Opens { place: args.first().cloned().unwrap_or_default(), listed: args.iter().skip(1).cloned().collect() };
            self.made(number, format!("scene: {}", args.join(" ")), shape);
            let pending = std::mem::take(&mut self.pending);
            self.out.extend(pending);
            self.scopes.push(HashMap::new());
            self.stmts(&scene.body);
            self.scopes.pop();
        }
    }

    pub(super) fn stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.stmt(stmt);
            self.remember_comment(stmt);
        }
    }

    /// A comment kept for what follows it (a recipe's doc); any other line
    /// lets them go.
    pub(super) fn remember_comment(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            Kind::Comment(comment) => self.comments.push(comment.trim().to_string()),
            _ => self.comments.clear(),
        }
    }

    /// The lines that stmts write, apart.
    pub(super) fn apart(&mut self, stmts: &[Stmt]) -> Vec<Out> {
        let before = std::mem::take(&mut self.out);
        self.scopes.push(HashMap::new());
        self.stmts(stmts);
        self.scopes.pop();
        std::mem::replace(&mut self.out, before)
    }

    pub(super) fn beside(stmt: &Stmt, line: String) -> String {
        match &stmt.beside {
            Some(beside) => match &beside.named {
                Some(name) => format!("{line} & as {}", name.text),
                None => format!("{line} &"),
            },
            None => line,
        }
    }

    /// A statement, its lines told as written on its own line.
    pub(super) fn stmt(&mut self, stmt: &Stmt) {
        let kept = std::mem::replace(&mut self.own, self.source.position(stmt.span.start).line);
        self.statement(stmt);
        self.own = kept;
    }

    fn statement(&mut self, stmt: &Stmt) {
        let number = self.number(stmt.span);
        match &stmt.kind {
            Kind::Blank => self.nothing(number, ""),
            Kind::Comment(comment) => self.nothing(number, format!("# {comment}")),
            Kind::Title(title) => self.made(number, format!("title: {title}"), Shape::Title(title.clone())),
            Kind::Moment(moment) => self.word(number, "moment", moment.split_whitespace().map(str::to_string).collect()),
            Kind::Uses(name) => self.word(number, "uses", vec![name.text.clone()]),
            Kind::Includes(name) => self.word(number, "include", vec![name.text.clone()]),
            Kind::For(target) => self.word(number, "for", target.text.split_whitespace().map(str::to_string).collect()),
            Kind::Naming { name, species } => {
                let shape = Shape::Naming { name: name.text.clone(), species: species.text.clone() };
                self.made(number, format!("{} = {}", name.text, species.text), shape);
            }
            // (`transition: wipe`: how the next scene opens.)
            Kind::Said(said) if said.who.text == "transition" && said.manners.is_empty() => {
                self.made(number, format!("transition: {}", said.text), Shape::Transition(said.text.trim().to_string()));
            }
            Kind::Said(said) => {
                // (In the language played, if the line gives it; else in English.)
                let chosen = &self.language;
                let pieces = said.languages.iter().find(|other| other.code.text == *chosen).map_or(&said.pieces, |other| &other.pieces).clone();
                self.said_line(stmt, said, &pieces, number, false);
            }
            Kind::Call(call) => {
                // (`cease sound rain`, `cease ambient wind`, `cease anim a`: a sound's, a word's.)
                let sound = call.args.first().is_some_and(|first| matches!(first.text.as_str(), "sound" | "ambient" | "anim"));
                if call.word.text == "cease" && !sound {
                    match call.args.as_slice() {
                        [name] => self.flow(number, crate::emit::set(format!("_active_{}", name.text), Value::Number(0.0))),
                        _ => self.error(stmt.span, "“cease” takes the name of a block (“every … as name”)"),
                    }
                    return;
                }
                // (`pikachu walk to right`: who first, then the word; read
                // as `walk pikachu to right`.)
                let subject = self.fighting == 0 && crate::verbs::subject_first(&call.word.text, call.args.first().map(|arg| arg.text.as_str()), &self.text);
                let (mut line, rest) = match subject {
                    true => (format!("{} {}", call.args[0].text, self.given_word(&call.word.text)), &call.args[1..]),
                    false => (call.word.text.clone(), &call.args[..]),
                };
                for arg in rest {
                    line.push(' ');
                    line.push_str(&self.arg(arg, number));
                }
                let line = Self::beside(stmt, line);
                self.line_word(number, line);
            }
            Kind::Let { kept, name, value } => {
                let var = if *kept {
                    name.text.clone()
                } else if let Some(call) = &self.call {
                    format!("_{}_{}", crate::words::plain(&name.text), call.id)
                } else {
                    format!("_{}", name.text)
                };
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(name.text.clone(), var.clone());
                }
                // (`keep`: the game's, given to the film; its value only when the game gives none.)
                if *kept {
                    let none = engine::Condition { var: var.clone(), compare: engine::Compare::Equal, value: engine::Value::Number(0.0) };
                    self.flow(number, FlowBeat::If(engine::Test { any: vec![vec![none]] }));
                    self.assign(&var, value, number);
                    self.flow(number, FlowBeat::EndIf);
                } else {
                    self.assign(&var, value, number);
                }
            }
            Kind::Set { name, value } => match self.variable(&name.text) {
                Some(var) => self.assign(&var, value, number),
                None => self.error(name.span, format!("“{}” has no value: “let {} = …” first", name.text, name.text)),
            },
            Kind::Return(value) => {
                let Some(recipe) = self.recipe.as_ref().map(|recipe| recipe.name.clone()) else {
                    self.error(stmt.span, "“return” only makes sense in a recipe");
                    return;
                };
                if let Some(value) = value {
                    self.assign(&format!("_return_{recipe}"), value, number);
                }
                // (Before its end: the rest of the recipe left, to its end's mark,
                // one of this call's own.)
                if let Some(within) = self.recipe.as_mut().filter(|within| within.last != Some(stmt.span.start)) {
                    within.early = true;
                    let id = self.call.as_ref().map_or(0, |call| call.id);
                    self.flow(number, crate::emit::goto(format!("_returned_{id}")));
                }
            }
            Kind::Break => self.flow(number, FlowBeat::Break),
            Kind::Continue => self.flow(number, FlowBeat::Continue),
            Kind::Block(block) => self.block(stmt, block, number),
            Kind::Unreadable(_) => {}
        }
    }

    /// A line said, its pieces as written in the language played: words
    /// chosen by a test (`{if met: Hi | Hello}`) make it two lines, the one
    /// whose test holds said as the film plays.
    fn said_line(&mut self, stmt: &Stmt, said: &language::ast::Said, pieces: &[Piece], number: usize, chosen: bool) {
        if let Some(at) = pieces.iter().position(|piece| matches!(piece, Piece::Choose { .. })) {
            let Piece::Choose { test, then, otherwise } = &pieces[at] else { return };
            let Some(test) = self.test(test, number) else { return };
            let with = |chosen: &[Piece]| pieces[..at].iter().chain(chosen).chain(&pieces[at + 1..]).cloned().collect::<Vec<Piece>>();
            let (then, otherwise) = (with(then), with(otherwise));
            self.flow(number, FlowBeat::If(test));
            self.said_line(stmt, said, &then, number, true);
            self.flow(number, FlowBeat::Else);
            self.said_line(stmt, said, &otherwise, number, true);
            self.flow(number, FlowBeat::EndIf);
            return;
        }
        let who = self.put.get(&said.who.text).cloned().unwrap_or_else(|| said.who.text.clone());
        let mut text = String::new();
        for piece in pieces {
            match piece {
                Piece::Text(plain) => text.push_str(plain),
                Piece::Pause(pause) => text.push_str(&format!("[{}]", pause.written())),
                Piece::Value(value) => text.push_str(&self.filled(value, number)),
                Piece::Choose { .. } => {}
            }
        }
        // (Words chosen left out leave no two spaces.)
        let text = if chosen { text.split(' ').filter(|word| !word.is_empty()).collect::<Vec<_>>().join(" ") } else { text };
        // (`narrate: Long ago...`: no one's line, the narrator's word.)
        if who == "narrate" && said.manners.is_empty() {
            let words: Vec<String> = text.split_whitespace().map(str::to_string).collect();
            self.made(number, format!("narrate {text}"), Shape::Word { verb: who, words, wait: true });
            return;
        }
        let manners: Vec<&str> = said.manners.iter().map(|manner| manner.text.as_str()).collect();
        let brackets = (!manners.is_empty()).then(|| manners.join(", "));
        let head = match &brackets {
            Some(brackets) => format!("{who} ({brackets})"),
            None => who.clone(),
        };
        let line = if text.is_empty() { format!("{head}:") } else { format!("{head}: {text}") };
        let line = Self::beside(stmt, line);
        // (Beside the film unnamed: `&`; named, `& as name` stays
        // among its words, read by the said line.)
        let beside = stmt.beside.as_ref().is_some_and(|beside| beside.named.is_none());
        let words = if beside { text.trim().to_string() } else { line[line.find(':').map_or(0, |colon| colon + 1)..].trim().to_string() };
        self.made(number, line, Shape::Said { who, brackets, words, beside });
    }

    /// Where a name's value is kept, if a line gave it one.
    pub(super) fn variable(&self, name: &str) -> Option<String> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name).cloned())
    }

    pub(super) fn assign(&mut self, var: &str, value: &Expr, number: usize) {
        if let ExprKind::Text(text) = &value.kind {
            self.flow(number, DataBeat::SetText { var: var.to_string(), text: text.replace(char::is_whitespace, "_") });
            return;
        }
        if let Some(field) = self.value(value, number) {
            self.flow(number, crate::emit::set(var, operand(&field)));
        }
    }
}
