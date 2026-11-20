//! The scene's numbers, what it asks of them, chance.

use super::*;

impl Director {
    /// A number's value (0 for one never given).
    pub fn value(&self, value: &Value) -> f64 {
        match value {
            Value::Number(number) => *number,
            Value::Var(var) => self.vars.get(var).copied().unwrap_or(0.0),
        }
    }

    /// Whether what a scene asks holds: any of its ways, all of each.
    pub fn holds(&self, test: &Test) -> bool {
        test.any.iter().any(|all| all.iter().all(|condition| self.compares(condition)))
    }

    /// Whether a number stands so to a value.
    pub(super) fn compares(&self, condition: &Condition) -> bool {
        // (Words kept, see `PlayBeat::AskText`: compared with a word, or other
        // words kept, as they are written but for capitals, accents and
        // the spaces around (`_` for a space: `== mont_ardent`).)
        if let Some(words) = self.texts.get(&condition.var) {
            let other = match &condition.value {
                Value::Var(word) => self.texts.get(word).cloned().unwrap_or_else(|| word.replace('_', " ")),
                Value::Number(number) => number.to_string(),
            };
            let same = plain(words) == plain(&other);
            return match condition.compare {
                Compare::Equal => same,
                Compare::Unequal => !same,
                // (Longer, shorter: by their letters.)
                compare => compare.holds(plain(words).chars().count() as f64, plain(&other).chars().count() as f64),
            };
        }
        let var = self.vars.get(&condition.var).copied().unwrap_or(0.0);
        condition.compare.holds(var, self.value(&condition.value))
    }

    /// A number drawn by chance, below that.
    pub(super) fn chance(&mut self, below: u32) -> u32 {
        self.seed = self.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345) ^ (self.clocks.clock.to_bits() >> 4);
        (self.seed >> 16) % below.max(1)
    }

    /// A wait drawn between two (see `FlowBeat::WaitBetween`) as the wait it
    /// is this time; whoever Enter found last where the beat says `target`
    /// (see `Who::Target`); the numbers it names (`Num::Var`) put in, a wait
    /// found so as the wait it is; a line the game reads, its numbers put
    /// in; any other beat as it is.
    pub(super) fn drawn(&mut self, beat: Beat) -> Beat {
        // (A line whose names are built as it plays: read now, filled.)
        let beat = match beat {
            Beat::Flow(FlowBeat::Filled(line)) => {
                let filled: Vec<String> = line.iter().map(|field| self.fill(field)).collect();
                Beat::parse(&filled, self.words).unwrap_or_else(|_| Beat::Flow(FlowBeat::Log(format!("line badly filled: {}", filled.join(" ")))))
            }
            Beat::Course(Course::Host(line)) if line.iter().any(|field| field.contains('{')) => {
                Beat::Course(Course::Host(line.iter().map(|field| self.fill(field)).collect()))
            }
            beat => beat,
        };
        let mut beat = match beat {
            Beat::Flow(FlowBeat::WaitBetween { from, to }) => {
                let (from, to) = (from.0 as u32, to.0 as u32);
                Beat::Flow(FlowBeat::Wait(Frames((from.min(to) + self.chance(from.abs_diff(to) + 1)) as f32)))
            }
            // (A sound waited for: as long as it lasts.)
            Beat::Flow(FlowBeat::WaitSound { of, pitch }) => {
                let speed = crate::Played { pitch, ..crate::Played::of(of.clone()) }.speed();
                Beat::Flow(FlowBeat::Wait(Frames(self.sound_frames(&of) / speed)))
            }
            beat => beat,
        };
        let target = self.target();
        for who in beat.whos_mut().into_iter().filter(|who| **who == Who::Target) {
            *who = target.unwrap_or(Who::Target);
        }
        // (Each number found as the scene plays, as a line's `{name}` is
        // filled; one never given stays unknown, read as 0.)
        for num in beat.nums_mut() {
            if let Num::Var(name) = num
                && let Ok(value) = self.fill(&format!("{{{name}}}")).parse::<f32>()
            {
                *num = Num::Lit(value);
            }
        }
        // (A wait found as it plays: that long, now; one never given, none.)
        if let Beat::Flow(FlowBeat::WaitFound(frames)) = &beat {
            return Beat::Flow(FlowBeat::Wait(Frames(frames.value().max(0.0).round())));
        }
        beat
    }

    /// Whoever Enter found last (see `Director::press`), if anyone.
    pub(super) fn target(&self) -> Option<Who> {
        self.vars.get(TARGET).and_then(|&number| Who::from_number(number as i64))
    }
}

/// A number as a scene writes it: `3`, `3.5`, `-0.25` (three decimals at most).
pub fn number_text(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        return format!("{}", number as i64);
    }
    let written = format!("{number:.3}");
    written.trim_end_matches('0').trim_end_matches('.').to_string()
}
