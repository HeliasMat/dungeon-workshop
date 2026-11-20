//! The values past what the registry advises (`crate::props`), told as
//! remarks: they play as written, nothing clamps them. An impossible one is
//! an error (a value set on a node, its type wrong: not read at all).

use super::*;
use crate::props::{Given, PropId, Value};

/// The values of a line the registry knows the property of.
fn values(beat: &Beat) -> Vec<(PropId, Value)> {
    match beat {
        Beat::Body(BodyBeat::Scale { to, .. }) => vec![(PropId::Scale, Value::Number(*to))],
        Beat::Body(BodyBeat::Sink { depth, .. }) => vec![(PropId::Sink, Value::Number(*depth))],
        Beat::Body(BodyBeat::Slow { by, .. }) => vec![(PropId::Slowmo, Value::Number(*by))],
        Beat::Node(NodeBeat::Set { prop, value: Given::Is(value), .. }) => vec![(*prop, *value)],
        _ => Vec::new(),
    }
}

/// What is past the advised values, line by line.
pub(super) fn past_advised(reads: &[Read]) -> Vec<Diagnostic> {
    let mut problems = Vec::new();
    for read in reads {
        let Is::Beat(beat) = &read.is else {
            continue;
        };
        for (prop, value) in values(beat) {
            match prop.prop().check(&value) {
                Ok(None) => {}
                Ok(Some(remark)) => problems.push(Diagnostic::at(read.number, remark.message).as_remark().coded(remark.code)),
                Err(wrong) => problems.push(Diagnostic::at(read.number, wrong.message).coded(wrong.code)),
            }
        }
    }
    problems
}
