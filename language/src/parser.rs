//! A whole text: its lines read one by one, the blocks put together from
//! their opening to their `end`, the scenes apart. Nothing stops it: what
//! does not read is kept as written, and said.

use crate::ast::{Block, File, Kind, Opening, Part, PartKind, Scene, Stmt};
use crate::diag::{Diagnostic, codes};
use crate::line::{Read, line};
use crate::span::Span;

/// A text read: its tree and what is wrong with it.
pub struct Parsed {
    pub file: File,
    pub diagnostics: Vec<Diagnostic>,
}

/// A block being read.
struct Open {
    block: Block,
    start: Span,
}

impl Open {
    fn lines(&mut self) -> &mut Vec<Stmt> {
        match self.block.parts.last_mut() {
            Some(part) => &mut part.body,
            None => &mut self.block.body,
        }
    }
}

struct Builder {
    file: File,
    open: Vec<Open>,
    diagnostics: Vec<Diagnostic>,
}

impl Builder {
    fn push(&mut self, stmt: Stmt) {
        if let Some(open) = self.open.last_mut() {
            open.lines().push(stmt);
        } else if let Some(scene) = self.file.scenes.last_mut() {
            scene.body.push(stmt);
        } else {
            self.file.head.push(stmt);
        }
    }

    /// Closes the innermost block at that `end`.
    fn close(&mut self, end: Span, comment: Option<String>) {
        let Some(mut open) = self.open.pop() else {
            self.diagnostics.push(Diagnostic::error(codes::STRAY_END, end, "one “end” too many: no block is open").fix("remove this line", end, ""));
            self.push(Stmt { span: end, kind: Kind::Unreadable("end".to_string()), beside: None, comment });
            return;
        };
        open.block.end_comment = comment;
        self.push(Stmt { span: open.start.to(end), kind: Kind::Block(Box::new(open.block)), beside: None, comment: None });
    }

    /// Closes every block left open, each said.
    fn close_all(&mut self, at: Span) {
        while let Some(open) = self.open.last() {
            let word = open.block.opening.word();
            let start = open.block.opening_span;
            self.diagnostics.push(
                Diagnostic::error(codes::UNCLOSED, start, format!("“{word}” is not closed by “end”")).note("each block is closed by a line “end”").fix(
                    "close the block here",
                    Span::new(at.start, at.start),
                    "end\n",
                ),
            );
            self.close(Span::new(at.start, at.start), None);
        }
    }

    fn part(&mut self, kind: PartKind, span: Span, comment: Option<String>) {
        let fits = match (&kind, self.open.last()) {
            (PartKind::Option { .. }, Some(open)) => matches!(open.block.opening, Opening::Choice),
            (_, Some(open)) => matches!(open.block.opening, Opening::If(_)) && !open.block.parts.iter().any(|part| matches!(part.kind, PartKind::Else)),
            (_, None) => false,
        };
        if !fits {
            let word = if matches!(kind, PartKind::Option { .. }) { "option" } else { "else" };
            let home = if word == "option" { "a “choice” block" } else { "an “if” block (and once “else” alone)" };
            self.diagnostics.push(Diagnostic::error(codes::STRAY_MIDDLE, span, format!("“{word}” only makes sense in {home}")));
            self.push(Stmt { span, kind: Kind::Unreadable(word.to_string()), beside: None, comment });
            return;
        }
        if let Some(open) = self.open.last_mut() {
            open.block.parts.push(Part { kind, span, comment, body: Vec::new() });
        }
    }
}

/// Reads a whole text.
pub fn parse(text: &str) -> Parsed {
    parse_with(text, &|_| false)
}

/// Reads a whole text, the words of a vocabulary that hold the lines up to
/// their `end` told by `holds`.
pub fn parse_with(text: &str, holds: &dyn Fn(&crate::ast::Call) -> bool) -> Parsed {
    let mut builder = Builder { file: File::default(), open: Vec::new(), diagnostics: Vec::new() };
    let mut start = 0;
    for raw in text.split_inclusive('\n') {
        let body = raw.trim_end_matches(['\n', '\r']);
        let before_scenes = builder.file.scenes.is_empty();
        let read = line(body, start, before_scenes, &mut builder.diagnostics);
        let (span, comment) = (read.span, read.comment);
        match read.read {
            Read::Blank => builder.push(Stmt { span, kind: Kind::Blank, beside: None, comment: None }),
            Read::Comment(said) => builder.push(Stmt { span, kind: Kind::Comment(said), beside: None, comment: None }),
            Read::Stmt(Kind::Call(call), None) if holds(&call) => {
                let block = Block {
                    opening: Opening::Word(call),
                    opening_span: span,
                    named: None,
                    body: Vec::new(),
                    parts: Vec::new(),
                    short: false,
                    extra: None,
                    opening_comment: comment,
                    end_comment: None,
                };
                builder.open.push(Open { block, start: span });
            }
            Read::Stmt(kind, beside) => builder.push(Stmt { span, kind, beside, comment }),
            Read::Scene(args) => {
                builder.close_all(span);
                builder.file.scenes.push(Scene { span, args, comment, body: Vec::new() });
            }
            Read::Opens { opening, named, short, extra } => {
                let mut block = Block {
                    opening,
                    opening_span: span,
                    named,
                    body: Vec::new(),
                    parts: Vec::new(),
                    short: false,
                    extra,
                    opening_comment: None,
                    end_comment: None,
                };
                match short {
                    Some((kind, beside, inner)) => {
                        block.short = true;
                        block.body.push(Stmt { span: inner, kind, beside, comment: None });
                        builder.push(Stmt { span, kind: Kind::Block(Box::new(block)), beside: None, comment });
                    }
                    None => {
                        block.opening_comment = comment;
                        builder.open.push(Open { block, start: span });
                    }
                }
            }
            Read::ElseIf(test) => builder.part(PartKind::ElseIf(test), span, comment),
            Read::Else => builder.part(PartKind::Else, span, comment),
            Read::Option(kind) => builder.part(kind, span, comment),
            Read::End => builder.close(span, comment),
        }
        start += raw.len();
    }
    builder.close_all(Span::new(text.len(), text.len()));
    let Builder { file, diagnostics, .. } = builder;
    Parsed { file, diagnostics }
}
