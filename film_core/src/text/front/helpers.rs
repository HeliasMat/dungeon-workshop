//! What is read off the language's tree alone: the names a recipe is given, the sums folded, a fight's turn.

use super::*;

/// The names given a recipe: whether each may be left out, is a team.
pub(super) fn gather(elements: &[Element], optional: bool, into: &mut HashMap<String, (bool, bool, String)>) {
    for element in elements {
        match element {
            Element::Param { name, ty, default, many, .. } => {
                let ty = ty.as_ref().map_or_else(String::new, |ty| ty.name.clone());
                into.insert(name.text.clone(), (optional || default.is_some(), *many, ty));
            }
            Element::Optional(inner) => gather(inner, true, into),
            Element::OneOf(runs) => {
                for run in runs {
                    gather(run, true, into);
                }
            }
            // (A word that may be left out, or one of a choice, is a name
            // too: `if miss` tells whether the call said it.)
            Element::Word(word) if optional => {
                into.entry(word.text.clone()).or_insert((true, false, String::new()));
            }
            Element::Word(_) => {}
        }
    }
}

/// A calculation's names (`where.x` as such) and signs, if it is only names,
/// numbers, durations and `+ - * /`.
pub(super) fn walk(value: &Expr, names: &mut Vec<String>, signs: &mut Vec<BinOp>) -> bool {
    match &value.kind {
        ExprKind::Number(_) | ExprKind::Duration(_) => true,
        ExprKind::Name(name) => {
            names.push(name.clone());
            true
        }
        ExprKind::Field(base, field) => match &base.kind {
            ExprKind::Name(name) => {
                names.push(format!("{name}.{}", field.text));
                true
            }
            _ => false,
        },
        ExprKind::Binary(op @ (BinOp::Plus | BinOp::Minus | BinOp::Times | BinOp::Divide), left, right) => {
            signs.push(*op);
            walk(left, names, signs) && walk(right, names, signs)
        }
        _ => false,
    }
}

pub(super) fn has_or(value: &Expr) -> bool {
    matches!(value.kind, ExprKind::Binary(BinOp::Or, ..))
}

/// A calculation of numbers only, worked out (durations as frames).
pub(super) fn folded(value: &Expr) -> Option<f64> {
    match &value.kind {
        ExprKind::Number(number) => Some(*number),
        ExprKind::Duration(duration) => Some(duration.frames()),
        ExprKind::Bool(truth) => Some(f64::from(u8::from(*truth))),
        ExprKind::Unary(UnOp::Minus, inner) => folded(inner).map(|number| -number),
        ExprKind::Binary(op, left, right) => {
            let (left, right) = (folded(left)?, folded(right)?);
            Some(match op {
                BinOp::Plus => left + right,
                BinOp::Minus => left - right,
                BinOp::Times => left * right,
                BinOp::Divide if right != 0.0 => left / right,
                BinOp::Divide => 0.0,
                BinOp::Rest if right != 0.0 => left % right,
                BinOp::Rest => 0.0,
                _ => return None,
            })
        }
        _ => None,
    }
}

/// A duration written as it is (`2s`, `6f`).
pub(super) fn folded_duration(value: &Expr) -> Option<String> {
    match &value.kind {
        ExprKind::Duration(duration) => Some(duration.written()),
        _ => None,
    }
}

pub(super) fn folded_duration_frames(value: &Expr) -> Option<f64> {
    match &value.kind {
        ExprKind::Duration(_) | ExprKind::Binary(..) | ExprKind::Unary(..) if has_duration(value) => folded(value),
        _ => None,
    }
}

pub(super) fn has_duration(value: &Expr) -> bool {
    match &value.kind {
        ExprKind::Duration(_) => true,
        ExprKind::Binary(_, left, right) => has_duration(left) || has_duration(right),
        ExprKind::Unary(_, inner) => has_duration(inner),
        _ => false,
    }
}

/// `health(c) <= 50` in a fight: who, and how much of his life is left.
pub(super) fn fight_turn(test: &Expr) -> Option<(String, String)> {
    let ExprKind::Binary(BinOp::LessEq | BinOp::Less, left, right) = &test.kind else { return None };
    let ExprKind::Call(called, args) = &left.kind else { return None };
    let (ExprKind::Number(life), [Expr { kind: ExprKind::Name(who), .. }]) = (&right.kind, args.as_slice()) else { return None };
    (called.text == "health").then(|| (who.clone(), number_text(*life)))
}
