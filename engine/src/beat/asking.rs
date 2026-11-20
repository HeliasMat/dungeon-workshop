//! What a scene asks of its numbers.

use super::*;

/// What a scene asks of a number, as written after `if` or `while`:
/// `<var> <compare> <value>`, or `<var>` alone (that it is not 0).
pub(super) fn condition(fields: &[String]) -> Option<Condition> {
    Some(match fields {
        [var, compare, value] => Condition { var: var.clone(), compare: Compare::parse(compare)?, value: Value::parse(value) },
        [var] => Condition { var: var.clone(), compare: Compare::Unequal, value: Value::Number(0.0) },
        _ => return None,
    })
}

/// What a scene asks, as written after `if`, `elif`, `while` or
/// `waituntil`: comparisons joined by `and`, those joined by `or`:
/// `a == 1 and b > 2 or c`.
pub(super) fn test(fields: &[String]) -> Option<Test> {
    let mut any = Vec::new();
    for either in fields.split(|field| field == "or") {
        let all = either.split(|field| field == "and").map(condition).collect::<Option<Vec<Condition>>>()?;
        any.push(all);
    }
    Some(Test { any })
}

/// An answer offered only when a test holds, as written: `[if <test>]`
/// before its words, the test as after `if`
/// (`[if item_109 > 0]Give an Apple`). The test, and the words.
pub(crate) fn answer_test(answer: &str) -> (Option<Test>, &str) {
    let asked = answer.trim_start().strip_prefix("[if ");
    let Some((asked, words)) = asked.and_then(|asked| asked.split_once(']')) else {
        return (None, answer);
    };
    let fields: Vec<String> = asked.split_whitespace().map(str::to_string).collect();
    match test(&fields) {
        Some(test) => (Some(test), words),
        None => (None, answer),
    }
}

/// A number of the director's, or a number as it is.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Var(String),
}

impl Value {
    pub(super) fn parse(field: &str) -> Value {
        // (Someone, as a number: `if target == m133` (or `p2`, `i109`), or `== eevee` with a
        // name given by `cast`.)
        let someone = field.starts_with(['m', 'i', 'p', 'o']) && field[1..].starts_with(|c: char| c.is_ascii_digit());
        match field.parse() {
            Ok(number) => Value::Number(number),
            Err(_) => match Who::parse(field).filter(|_| someone) {
                Some(who) => Value::Number(who.number() as f64),
                None => Value::Var(field.to_string()),
            },
        }
    }
}

/// What a scene asks of its numbers: any of these, each all of its
/// comparisons (see [`Condition`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Test {
    pub any: Vec<Vec<Condition>>,
}

/// What a scene asks of a number: `<var> <compare> <value>` (`<var>`
/// alone: that it is not 0).
#[derive(Clone, Debug, PartialEq)]
pub struct Condition {
    pub var: String,
    pub compare: Compare,
    pub value: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compare {
    Equal,
    Unequal,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl Compare {
    pub(super) fn parse(field: &str) -> Option<Compare> {
        Some(match field {
            "==" | "=" => Compare::Equal,
            "!=" => Compare::Unequal,
            "<" => Compare::Less,
            "<=" => Compare::LessOrEqual,
            ">" => Compare::Greater,
            ">=" => Compare::GreaterOrEqual,
            _ => return None,
        })
    }

    /// Whether `a` stands so to `b`.
    pub fn holds(self, a: f64, b: f64) -> bool {
        match self {
            Compare::Equal => a == b,
            Compare::Unequal => a != b,
            Compare::Less => a < b,
            Compare::LessOrEqual => a <= b,
            Compare::Greater => a > b,
            Compare::GreaterOrEqual => a >= b,
        }
    }
}
