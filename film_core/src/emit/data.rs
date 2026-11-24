//! The beats of the numbers a scene keeps.

use engine::{Beat, DataBeat, Value, Who};

/// A number as it is.
pub fn number(number: impl Into<f64>) -> Value {
    Value::Number(number.into())
}

/// A number kept by that name.
pub fn var(name: impl Into<String>) -> Value {
    Value::Var(name.into())
}

/// A number given a value.
pub fn set(var: impl Into<String>, value: Value) -> Beat {
    Beat::Data(DataBeat::Set { var: var.into(), value })
}

/// A number added to.
pub fn add_to(var: impl Into<String>, value: Value) -> Beat {
    Beat::Data(DataBeat::Add { var: var.into(), value })
}

/// A number given what two others make (`+`, `-`, `*`, `/`, `min`, `max`...).
pub fn calc(var: impl Into<String>, a: Value, op: &str, b: Value) -> Beat {
    Beat::Data(DataBeat::Calc { var: var.into(), a, op: op.to_string(), b })
}

/// Two numbers given where someone stands.
pub fn position(who: Who, x: impl Into<String>, y: impl Into<String>) -> Beat {
    Beat::Data(DataBeat::Position { who, x: x.into(), y: y.into() })
}
