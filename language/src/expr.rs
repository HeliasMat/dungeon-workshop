//! Calculations: `or`, `and`, comparisons, `+ -`, `* / %`, `not` and `-`,
//! then `.field`, `[i]` and calls, each tighter than the one before.

use crate::ast::{BinOp, Expr, ExprKind, Name, UnOp};
use crate::diag::{Diagnostic, codes};
use crate::lexer::{Lexed, Token, tokens};
use crate::span::Span;

/// The calculation that is the whole of that text (at `base` in the file).
pub fn expression(text: &str, base: usize) -> Result<Expr, Diagnostic> {
    let lexed = tokens(text, base)?;
    let end = base + text.trim_end().len();
    if lexed.is_empty() {
        return Err(Diagnostic::error(codes::EXPRESSION, Span::new(end, end), "a calculation is expected here"));
    }
    let mut reader = Reader { tokens: &lexed, at: 0, end };
    let made = reader.or()?;
    if let Some(extra) = reader.peek() {
        let span = Span::new(extra.span.start, end);
        return Err(Diagnostic::error(codes::EXPRESSION, span, format!("the calculation is over before “{}”", span_text(text, base, span)))
            .note("two values follow each other with no sign between them"));
    }
    Ok(made)
}

fn span_text(text: &str, base: usize, span: Span) -> &str {
    text.get(span.start - base..span.end - base).unwrap_or("").trim()
}

struct Reader<'a> {
    tokens: &'a [Lexed],
    at: usize,
    end: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<&Lexed> {
        self.tokens.get(self.at)
    }

    fn is_sign(&self, sign: &str) -> bool {
        matches!(self.peek(), Some(Lexed { token: Token::Sign(found), .. }) if *found == sign)
    }

    fn is_name(&self, name: &str) -> bool {
        matches!(self.peek(), Some(Lexed { token: Token::Name(found), .. }) if found == name)
    }

    fn next(&mut self) -> Option<&Lexed> {
        let token = self.tokens.get(self.at);
        self.at += 1;
        token
    }

    fn missing(&self, what: &str) -> Diagnostic {
        let span = self.peek().map_or(Span::new(self.end, self.end), |token| token.span);
        Diagnostic::error(codes::EXPRESSION, span, format!("{what} is expected here"))
    }

    fn expect(&mut self, sign: &str) -> Result<Span, Diagnostic> {
        if self.is_sign(sign) {
            return Ok(self.next().map(|token| token.span).unwrap_or_default());
        }
        Err(self.missing(&format!("“{sign}”")))
    }

    fn or(&mut self) -> Result<Expr, Diagnostic> {
        let mut made = self.and()?;
        while self.is_name("or") {
            self.at += 1;
            let right = self.and()?;
            made = binary(BinOp::Or, made, right);
        }
        Ok(made)
    }

    fn and(&mut self) -> Result<Expr, Diagnostic> {
        let mut made = self.comparison()?;
        while self.is_name("and") {
            self.at += 1;
            let right = self.comparison()?;
            made = binary(BinOp::And, made, right);
        }
        Ok(made)
    }

    fn comparison(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.sum()?;
        let op = [("<", BinOp::Less), ("<=", BinOp::LessEq), (">", BinOp::More), (">=", BinOp::MoreEq), ("=", BinOp::Equal), ("!=", BinOp::NotEqual)]
            .into_iter()
            .find(|(sign, _)| self.is_sign(sign));
        let Some((_, op)) = op else {
            return Ok(left);
        };
        self.at += 1;
        let right = self.sum()?;
        if let Some(again) = self.peek().filter(|token| matches!(token.token, Token::Sign("<" | "<=" | ">" | ">=" | "=" | "!="))) {
            return Err(
                Diagnostic::error(codes::EXPRESSION, again.span, "two comparisons in a row: part them with “and”").note("for example “1 < n and n < 5”")
            );
        }
        Ok(binary(op, left, right))
    }

    fn sum(&mut self) -> Result<Expr, Diagnostic> {
        let mut made = self.product()?;
        loop {
            let op = if self.is_sign("+") {
                BinOp::Plus
            } else if self.is_sign("-") {
                BinOp::Minus
            } else {
                return Ok(made);
            };
            self.at += 1;
            let right = self.product()?;
            made = binary(op, made, right);
        }
    }

    fn product(&mut self) -> Result<Expr, Diagnostic> {
        let mut made = self.unary()?;
        loop {
            let op = if self.is_sign("*") {
                BinOp::Times
            } else if self.is_sign("/") {
                BinOp::Divide
            } else if self.is_sign("%") {
                BinOp::Rest
            } else {
                return Ok(made);
            };
            self.at += 1;
            let right = self.unary()?;
            made = binary(op, made, right);
        }
    }

    fn unary(&mut self) -> Result<Expr, Diagnostic> {
        let op = if self.is_name("not") {
            UnOp::Not
        } else if self.is_sign("-") {
            UnOp::Minus
        } else {
            return self.suffixed();
        };
        let start = self.next().map(|token| token.span).unwrap_or_default();
        let inner = self.unary()?;
        Ok(Expr { span: start.to(inner.span), kind: ExprKind::Unary(op, Box::new(inner)) })
    }

    fn suffixed(&mut self) -> Result<Expr, Diagnostic> {
        let mut made = self.atom()?;
        loop {
            if self.is_sign(".") {
                self.at += 1;
                let Some(Lexed { token: Token::Name(field), span }) = self.peek().cloned() else {
                    return Err(self.missing("the name of a field (as “x”)"));
                };
                self.at += 1;
                made = Expr { span: made.span.to(span), kind: ExprKind::Field(Box::new(made), Name { text: field, span }) };
            } else if self.is_sign("[") {
                self.at += 1;
                let index = self.or()?;
                let end = self.expect("]")?;
                made = Expr { span: made.span.to(end), kind: ExprKind::Index(Box::new(made), Box::new(index)) };
            } else if self.is_sign("(") {
                let ExprKind::Name(called) = &made.kind else {
                    return Err(Diagnostic::error(codes::EXPRESSION, made.span, "only a name is called, as “health(a)”"));
                };
                let called = Name { text: called.clone(), span: made.span };
                self.at += 1;
                let mut args = Vec::new();
                if !self.is_sign(")") {
                    loop {
                        args.push(self.or()?);
                        if !self.is_sign(",") {
                            break;
                        }
                        self.at += 1;
                    }
                }
                let end = self.expect(")")?;
                made = Expr { span: made.span.to(end), kind: ExprKind::Call(called, args) };
            } else {
                return Ok(made);
            }
        }
    }

    fn atom(&mut self) -> Result<Expr, Diagnostic> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.missing("a value"));
        };
        self.at += 1;
        let kind = match token.token {
            Token::Number(number) => ExprKind::Number(number),
            Token::Duration(duration) => ExprKind::Duration(duration),
            Token::Text(text) => ExprKind::Text(text),
            Token::Name(name) => match name.as_str() {
                "true" => ExprKind::Bool(true),
                "false" => ExprKind::Bool(false),
                "none" => ExprKind::Nothing,
                // (`or`, `and` where a value is: a name.)
                "not" => {
                    return Err(Diagnostic::error(codes::EXPRESSION, token.span, format!("“{name}” expects a value before it")));
                }
                _ => ExprKind::Name(name),
            },
            Token::Sign("[") => {
                let mut items = Vec::new();
                if !self.is_sign("]") {
                    loop {
                        items.push(self.or()?);
                        if !self.is_sign(",") {
                            break;
                        }
                        self.at += 1;
                    }
                }
                let end = self.expect("]")?;
                return Ok(Expr { span: token.span.to(end), kind: ExprKind::List(items) });
            }
            Token::Sign("(") => {
                let first = self.or()?;
                if self.is_sign(",") {
                    self.at += 1;
                    let second = self.or()?;
                    let end = self.expect(")")?;
                    return Ok(Expr { span: token.span.to(end), kind: ExprKind::Point(Box::new(first), Box::new(second)) });
                }
                self.expect(")")?;
                return Ok(first);
            }
            Token::Sign(sign) => {
                return Err(Diagnostic::error(codes::EXPRESSION, token.span, format!("a value is expected here, not “{sign}”")));
            }
        };
        Ok(Expr { span: token.span, kind })
    }
}

fn binary(op: BinOp, left: Expr, right: Expr) -> Expr {
    Expr { span: left.span.to(right.span), kind: ExprKind::Binary(op, Box::new(left), Box::new(right)) }
}
