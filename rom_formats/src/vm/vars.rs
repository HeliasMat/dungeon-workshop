//! The script variables.

use super::*;

/// The script engine's variables: plain values, bit arrays and the
/// two-part story progress values, all addressed by variable id and index.
#[derive(Clone, Debug, Default)]
pub struct Vars {
    pub(super) values: HashMap<(u16, u16), i32>,
}

impl Vars {
    pub fn get(&self, id: u16) -> i32 {
        self.get_at(id, 0)
    }

    pub fn get_at(&self, id: u16, index: u16) -> i32 {
        self.values.get(&(id, index)).copied().unwrap_or(0)
    }

    pub fn set(&mut self, id: u16, value: i32) {
        self.set_at(id, 0, value);
    }

    pub(crate) fn set_at(&mut self, id: u16, index: u16, value: i32) {
        self.values.insert((id, index), value);
    }

    /// Every value, one `id index value` per line, for saves.
    pub fn to_text(&self) -> String {
        let mut values: Vec<_> = self.values.iter().collect();
        values.sort();
        let line = |(&(id, index), value): (&(u16, u16), &i32)| format!("{id} {index} {value}\n");
        values.into_iter().map(line).collect()
    }

    /// The values [`Self::to_text`] wrote; lines that are not are skipped.
    pub fn from_text(text: &str) -> Self {
        let mut vars = Self::default();
        for line in text.lines() {
            let mut numbers = line.split_whitespace().map(str::parse::<i32>);
            if let (Some(Ok(id)), Some(Ok(index)), Some(Ok(value))) = (numbers.next(), numbers.next(), numbers.next()) {
                vars.set_at(id as u16, index as u16, value);
            }
        }
        vars
    }

    /// A story progress variable: chapter and step.
    /// Takes every value `other` holds.
    pub fn merge(&mut self, other: &Vars) {
        self.values.extend(other.values.iter().map(|(at, value)| (*at, *value)));
    }

    pub fn scenario(&self, id: u16) -> (i32, i32) {
        (self.get_at(id, 0), self.get_at(id, 1))
    }
}
