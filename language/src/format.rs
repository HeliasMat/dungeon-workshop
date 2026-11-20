//! The text written back in order: two spaces a block deeper, one space
//! between words, the calculations spaced, one blank line at most. What the
//! author wrote (a said text, an argument, an unreadable line) stays as it
//! is. Written twice, a text does not change.

use crate::ast::{Arg, ArgKind, Beside, Block, Element, Event, Expr, ExprKind, File, Form, Kind, Opening, PartKind, Stmt, Type, UnOp};
use crate::line::number_text;

/// The text of that file, in order.
pub fn formatted(file: &File) -> String {
    let mut writer = Writer::default();
    writer.stmts(&file.head, 0);
    for scene in &file.scenes {
        let mut head = "scene:".to_string();
        for arg in &scene.args {
            head.push(' ');
            head.push_str(&arg_text(arg));
        }
        writer.line(0, &with_comment(head, scene.comment.as_deref()));
        writer.stmts(&scene.body, 0);
    }
    while writer.text.ends_with("\n\n") {
        writer.text.pop();
    }
    writer.text
}

#[derive(Default)]
struct Writer {
    text: String,
    blank: bool,
}

impl Writer {
    fn line(&mut self, depth: usize, line: &str) {
        self.text.push_str(&"  ".repeat(depth));
        self.text.push_str(line.trim_end());
        self.text.push('\n');
        self.blank = false;
    }

    fn blank_line(&mut self) {
        if !self.blank && !self.text.is_empty() {
            self.text.push('\n');
            self.blank = true;
        }
    }

    fn stmts(&mut self, stmts: &[Stmt], depth: usize) {
        for stmt in stmts {
            self.stmt(stmt, depth);
        }
    }

    fn stmt(&mut self, stmt: &Stmt, depth: usize) {
        match &stmt.kind {
            Kind::Blank => self.blank_line(),
            Kind::Block(block) if !block.short => self.block(block, depth),
            _ => self.line(depth, &with_comment(stmt_text(stmt), stmt.comment.as_deref())),
        }
    }

    fn block(&mut self, block: &Block, depth: usize) {
        self.line(depth, &with_comment(opening_text(block), block.opening_comment.as_deref()));
        self.stmts(&block.body, depth + 1);
        for part in &block.parts {
            let head = match &part.kind {
                PartKind::ElseIf(test) => format!("else if {}", expression(test)),
                PartKind::Else => "else".to_string(),
                PartKind::Option { text, .. } => format!("option {text}"),
            };
            self.line(depth, &with_comment(head, part.comment.as_deref()));
            self.stmts(&part.body, depth + 1);
        }
        self.line(depth, &with_comment("end".to_string(), block.end_comment.as_deref()));
    }
}

fn with_comment(line: String, comment: Option<&str>) -> String {
    match comment {
        Some("") => format!("{line} #"),
        Some(comment) => format!("{line} # {comment}"),
        None => line,
    }
}

/// The text of one line (a short block on its one line).
pub fn stmt_text(stmt: &Stmt) -> String {
    let made = match &stmt.kind {
        Kind::Blank => String::new(),
        Kind::Comment(comment) if comment.is_empty() => "#".to_string(),
        Kind::Comment(comment) => format!("# {comment}"),
        Kind::Title(title) => format!("title: {title}"),
        Kind::Moment(moment) => format!("moment: {moment}"),
        Kind::Uses(name) => format!("uses {}", name.text),
        Kind::Includes(name) => format!("include {}", name.text),
        Kind::Naming { name, species } => format!("{} = {}", name.text, species.text),
        Kind::Said(said) => {
            let mut made = said.who.text.clone();
            if !said.manners.is_empty() {
                let manners: Vec<&str> = said.manners.iter().map(|manner| manner.text.as_str()).collect();
                made += &format!(" ({})", manners.join(", "));
            }
            made.push(':');
            if !said.text.is_empty() {
                made.push(' ');
                made.push_str(&said.text);
            }
            made
        }
        Kind::Call(call) => {
            let mut made = call.word.text.clone();
            for arg in &call.args {
                made.push(' ');
                made.push_str(&arg_text(arg));
            }
            made
        }
        Kind::Let { kept, name, value } => format!("{} {} = {}", if *kept { "keep" } else { "let" }, name.text, expression(value)),
        Kind::Set { name, value } => format!("{} = {}", name.text, expression(value)),
        Kind::Return(None) => "return".to_string(),
        Kind::Return(Some(value)) => format!("return {}", expression(value)),
        Kind::Break => "break".to_string(),
        Kind::Continue => "next".to_string(),
        Kind::Unreadable(raw) => raw.clone(),
        Kind::Block(block) => {
            let inner = block.body.first().map(stmt_text).unwrap_or_default();
            format!("{} : {inner}", opening_text(block))
        }
    };
    match &stmt.beside {
        Some(beside) => made + &beside_text(beside),
        None => made,
    }
}

fn beside_text(beside: &Beside) -> String {
    match &beside.named {
        Some(name) => format!(" & as {}", name.text),
        None => " &".to_string(),
    }
}

/// An argument as written; a calculation in order.
pub fn arg_text(arg: &Arg) -> String {
    match &arg.kind {
        ArgKind::Value(value) => format!("{{{}}}", expression(value)),
        _ => arg.text.clone(),
    }
}

/// The opening line of a block.
pub fn opening_text(block: &Block) -> String {
    let mut made = match &block.opening {
        Opening::If(test) => format!("if {}", expression(test)),
        Opening::Repeat(times) => format!("repeat {}", expression(times)),
        Opening::ForEach { name, rank, list } => match rank {
            Some(rank) => format!("for each {}, {} in {}", name.text, rank.text, expression(list)),
            None => format!("for each {} in {}", name.text, expression(list)),
        },
        Opening::While(test) => format!("while {}", expression(test)),
        Opening::Together => "together".to_string(),
        Opening::Beside => match &block.named {
            Some(name) => return format!("beside {}", name.text),
            None => "beside".to_string(),
        },
        Opening::When(event) => format!("when {}", event_text(event)),
        Opening::At { mark, clock: crate::ast::Clock::Music, .. } => format!("at music {}", mark.text),
        Opening::At { mark, clock: crate::ast::Clock::Here, .. } => format!("at +{}", mark.text),
        Opening::At { mark, offset, .. } => match offset {
            Some((minus, by)) => format!("at {} {} {}", mark.text, if *minus { "-" } else { "+" }, expression(by)),
            None => format!("at {}", mark.text),
        },
        Opening::Every(every) => format!("every {}", expression(every)),
        Opening::After(after) => format!("after {}", expression(after)),
        Opening::Choice => "choice".to_string(),
        Opening::Fight(args) => {
            let mut made = "fight directed".to_string();
            for arg in args {
                made.push(' ');
                made.push_str(&arg_text(arg));
            }
            made
        }
        Opening::Word(call) => {
            let mut made = call.word.text.clone();
            for arg in &call.args {
                made.push(' ');
                made.push_str(&arg_text(arg));
            }
            made
        }
        Opening::Recipe(recipe) => match &recipe.returns {
            Some(returns) => format!("define {} -> {}", form_text(&recipe.form), type_text(returns)),
            None => format!("define {}", form_text(&recipe.form)),
        },
    };
    if let Some(name) = &block.named {
        made += &format!(" as {}", name.text);
    }
    if let Some(extra) = &block.extra {
        made += &format!(" {extra}");
    }
    made
}

fn event_text(event: &Event) -> String {
    match event {
        Event::Arrives(who) => format!("{} arrives", who.text),
        Event::Leaves(who) => format!("{} leaves", who.text),
        Event::Knocked(who) => format!("{} is ko", who.text),
        Event::Becomes(test) => expression(test),
    }
}

/// A form as the vocabulary writes it.
pub fn form_text(form: &Form) -> String {
    let mut made = form.word.text.clone();
    let rest = elements_text(&form.elements);
    if !rest.is_empty() {
        made.push(' ');
        made.push_str(&rest);
    }
    made
}

pub fn elements_text(elements: &[Element]) -> String {
    let parts: Vec<String> = elements
        .iter()
        .map(|element| match element {
            Element::Param { name, ty, default, many, rest } => {
                let mut made = String::new();
                if *many {
                    made.push('*');
                }
                made.push_str(&name.text);
                if *rest {
                    made.push_str("...");
                }
                if let Some(ty) = ty {
                    made.push(':');
                    made.push_str(&type_text(ty));
                }
                if let Some(default) = default {
                    made.push('=');
                    made.push_str(default);
                }
                made
            }
            Element::Word(word) => word.text.clone(),
            Element::Optional(inner) => format!("[{}]", elements_text(inner)),
            Element::OneOf(runs) => runs.iter().map(|run| elements_text(run)).collect::<Vec<_>>().join(" | "),
        })
        .collect();
    parts.join(" ")
}

pub fn type_text(ty: &Type) -> String {
    match &ty.of {
        Some(of) => format!("{}<{}>", ty.name, type_text(of)),
        None => ty.name.clone(),
    }
}

/// How tightly a calculation holds together, to know where parentheses go.
fn binding(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Binary(op, ..) => op.binding(),
        ExprKind::Unary(..) => 6,
        _ => 7,
    }
}

fn wrapped(expr: &Expr, needed: bool) -> String {
    if needed { format!("({})", expression(expr)) } else { expression(expr) }
}

/// A calculation, spaced, with only the parentheses it needs.
pub fn expression(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Number(number) => number_text(*number),
        ExprKind::Duration(duration) => duration.written(),
        ExprKind::Text(text) => format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\"")),
        ExprKind::Bool(true) => "true".to_string(),
        ExprKind::Bool(false) => "false".to_string(),
        ExprKind::Nothing => "none".to_string(),
        ExprKind::Name(name) => name.clone(),
        ExprKind::List(items) => format!("[{}]", items.iter().map(expression).collect::<Vec<_>>().join(", ")),
        ExprKind::Point(x, y) => format!("({}, {})", expression(x), expression(y)),
        ExprKind::Unary(op, inner) => {
            let inner = wrapped(inner, binding(inner) < 6);
            match op {
                UnOp::Not => format!("not {inner}"),
                UnOp::Minus => format!("-{inner}"),
            }
        }
        ExprKind::Binary(op, left, right) => {
            let level = op.binding();
            let compares = level == 3;
            let left = wrapped(left, binding(left) < level || (compares && binding(left) == level));
            let right = wrapped(right, binding(right) <= level);
            format!("{left} {} {right}", op.sign())
        }
        ExprKind::Field(base, field) => format!("{}.{}", wrapped(base, binding(base) < 7), field.text),
        ExprKind::Index(base, index) => format!("{}[{}]", wrapped(base, binding(base) < 7), expression(index)),
        ExprKind::Unreadable(raw) => raw.clone(),
        ExprKind::Call(name, args) => format!("{}({})", name.text, args.iter().map(expression).collect::<Vec<_>>().join(", ")),
    }
}
