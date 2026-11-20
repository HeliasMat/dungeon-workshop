//! The way through a routine: jumps, calls, shared routines, and handing an
//! operation to another character.

use super::{Act, Event, Op, Step, Vm, Who, act};

impl Vm {
    pub(super) fn flow(&mut self, op: &Op<'_>) -> Option<Step> {
        Some(match op.name {
            "Jump" => Step::Jump(0),
            "Call" => {
                self.calls.push((self.common, self.pc));
                Step::Jump(0)
            }
            "Return" => match self.calls.pop() {
                Some((common, back)) => {
                    (self.common, self.pc) = (common, back);
                    Step::On
                }
                None => Step::End,
            },
            // `Hold` waits for ever.
            "End" | "Hold" => Step::End,
            "Destroy" => {
                self.pc = usize::MAX;
                Step::Ask(act(Act::Destroy))
            }
            // The game's shared routines, by number.
            "CallCommon" | "JumpCommon" | "ExecuteCommon" | "supervision_ExecuteCommon" => {
                let Some(common) = &op.program.common else {
                    return Some(if op.name == "CallCommon" { Step::On } else { Step::End });
                };
                if op.name == "CallCommon" {
                    self.calls.push((self.common, self.pc));
                }
                (self.common, self.pc) = (true, Self::first(common, op.at(0) as usize));
                Step::On
            }
            // The next operation is for another character of the scene.
            "lives" | "object" | "performer" => {
                let who = match op.name {
                    "lives" => Who::Actor(op.at(0)),
                    "object" => Who::Object(op.at(0)),
                    _ => Who::Performer(op.at(0)),
                };
                let mut side = Self::at(self.common, self.pc);
                side.stop = Some((self.common, self.pc + 1));
                self.pc += 1;
                Step::Ask(Event::Lives(who, side))
            }
            _ => return None,
        })
    }
}
