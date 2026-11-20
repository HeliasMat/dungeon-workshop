//! What the characters say and how long they wait.

use super::{Act, Event, Face, Op, Step, Vars, Vm, act, signed};

impl Vm {
    pub(super) fn talk(&mut self, op: &Op<'_>, vars: &Vars, random: &mut dyn FnMut(i32) -> i32) -> Option<Step> {
        Some(match op.name {
            "message_SwitchTalk" | "message_SwitchMonologue" => self.line_of_value(op, vars),
            "message_SwitchMenu" | "message_SwitchMenu2" => self.choice(op),
            "message_Talk"
            | "message_Mail"
            | "message_Monologue"
            | "message_Notice"
            | "message_Narration"
            | "message_Explanation"
            | "message_ImitationSound" => {
                // Spoken lines and thoughts are someone's; notices,
                // letters and narration are nobody's.
                let named = op.name == "message_Talk";
                let speaker = match op.name {
                    "message_Talk" | "message_Monologue" => self.voice(named),
                    _ => None,
                };
                Step::Ask(Event::Message { speaker, text: op.text(op.at(op.p.len().saturating_sub(1))) })
            }
            "message_SetFace" | "message_SetFaceOnly" => {
                self.speaker = Some(Face { entity: op.at(0), face: Some(op.at(1)), mode: op.at(2), named: false });
                Step::On
            }
            "message_SetActor" | "message_SetFaceEmpty" => {
                self.speaker = Some(Face { entity: op.at(0), face: None, mode: 0, named: false });
                Step::On
            }
            "message_SetFacePosition" => {
                if let Some(speaker) = &mut self.speaker {
                    speaker.mode = op.at(0);
                }
                Step::On
            }
            "message_ResetActor" | "message_EmptyActor" => {
                self.speaker = None;
                Step::On
            }
            "message_Close" | "message_CloseEnforce" => Step::Ask(act(Act::CloseMessage)),
            "Wait" => Step::Ask(Event::Wait(op.at(0))),
            "WaitRandom" => {
                let spread = (op.value(1) - op.value(0)).max(0) + 1;
                Step::Ask(Event::Wait((op.value(0) + random(spread)).max(0) as u16))
            }
            _ => return None,
        })
    }

    /// A line chosen by a variable's value: the cases follow.
    fn line_of_value(&mut self, op: &Op<'_>, vars: &Vars) -> Step {
        let wanted = vars.get(op.at(0));
        let (mut chosen, mut default) = (None, None);
        loop {
            let case = op.script.operations.get(self.pc).map(|o| &o.parameters);
            match (op.name_at(self.pc), case) {
                ("CaseText", Some(case)) => {
                    if signed(case[0]) == wanted && chosen.is_none() {
                        chosen = case.get(1).copied();
                    }
                }
                ("DefaultText", Some(case)) => default = case.first().copied(),
                _ => break,
            }
            self.pc += 1;
        }
        match chosen.or(default) {
            Some(line) => {
                let named = op.name == "message_SwitchTalk";
                Step::Ask(Event::Message { speaker: self.voice(named), text: op.text(line) })
            }
            None => Step::On,
        }
    }

    /// A choice: the options follow, each with where it leads.
    fn choice(&mut self, op: &Op<'_>) -> Step {
        let mut options = Vec::new();
        self.menu.clear();
        while matches!(op.name_at(self.pc), "CaseMenu" | "CaseMenu2") {
            if let Some(case) = op.script.operations.get(self.pc).map(|o| &o.parameters) {
                options.push(op.text(case[0]));
                let to = case.get(1).copied().unwrap_or(0) as usize;
                self.menu.push(op.script.operation_at(to));
            }
            self.pc += 1;
        }
        if options.is_empty() { Step::On } else { Step::Ask(Event::Menu { options }) }
    }
}
