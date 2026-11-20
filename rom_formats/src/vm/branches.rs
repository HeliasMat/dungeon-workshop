//! What a script reads to go one way or another: branches, switches and
//! their cases, and the game's answers those read.

use rules::jobs::{DEPARTURE, JOB_BOARD, OUTLAW_BOARD};
use rules::quiz;
use rules::services::Service;
use rules::town::{MENU_PARTNER_NAME, MENU_PLAYER_NAME, MENU_TEAM_NAME, MENUS};

use super::{
    CHOOSE_DUNGEON, DUNGEON_ENTER, DUNGEON_MODES, DUNGEON_SELECT, ENTER_FREE_DUNGEON, ENTER_REQUEST_DUNGEON, ENTER_SCENARIO_DUNGEON, Event,
    GETOUT_SCENARIO_DUNGEON, JOB_DONE, JOB_DONE_THERE, JUMP_TO_TITLE, LEAVE, LEAVE_TOO, MODE_OPEN, Op, PERFORMANCE, PLACES, SAVE_ASKED, SAVING,
    SCENARIO_MAIN_BIT_FLAG, SECTOR, SPRINGS, Step, TEAM_IS_THE_TWO, Vars, Vm, WAY_TO_SPRING, compare,
};

/// A branch: the jump is taken, or not.
pub(super) fn branch(op: &Op<'_>, vars: &Vars) -> Option<Step> {
    Some(match op.name {
        "Branch" => Step::jump_if(vars.get(op.at(0)) == op.value(1), 2),
        "BranchBit" => Step::jump_if(vars.get_at(op.at(0), op.at(1)) != 0, 2),
        // Not a debug build, not the level editor, no variation.
        "BranchDebug" | "BranchEdit" | "BranchVariation" => Step::jump_if(op.at(0) == 0, 1),
        "BranchPerformance" => Step::jump_if((vars.get_at(PERFORMANCE, op.at(0)) != 0) == (op.value(1) >= 1), 2),
        "BranchScenarioNow" => Step::jump_if(vars.scenario(op.at(0)) == (op.value(1), op.value(2)), 3),
        "BranchScenarioNowAfter" => Step::jump_if(vars.scenario(op.at(0)) >= (op.value(1), op.value(2)), 3),
        "BranchScenarioNowBefore" => Step::jump_if(vars.scenario(op.at(0)) <= (op.value(1), op.value(2)), 3),
        "BranchScenarioAfter" => Step::jump_if(vars.scenario(op.at(0)) > (op.value(1), op.value(2)), 3),
        "BranchScenarioBefore" => Step::jump_if(vars.scenario(op.at(0)) < (op.value(1), op.value(2)), 3),
        "BranchValue" => Step::jump_if(compare(vars.get(op.at(0)), op.at(1), op.value(2)), 3),
        "BranchVariable" => Step::jump_if(compare(vars.get(op.at(0)), op.at(1), vars.get(op.at(2))), 3),
        _ => return None,
    })
}

impl Vm {
    /// A case of the current switch.
    pub(super) fn case(&self, op: &Op<'_>, vars: &Vars) -> Option<Step> {
        Some(match op.name {
            "Case" => Step::jump_if(self.switch == op.value(0), 1),
            "CaseValue" => Step::jump_if(compare(self.switch, op.at(0), op.value(1)), 2),
            "CaseScenario" => Step::jump_if(compare(self.switch, op.at(0), op.value(1)), 2),
            "CaseVariable" => Step::jump_if(compare(self.switch, op.at(0), vars.get(op.at(1))), 2),
            _ => return None,
        })
    }

    /// What the next cases compare with. (`random(n)` draws below `n`.)
    pub(super) fn switch(&mut self, op: &Op<'_>, vars: &mut Vars, random: &mut dyn FnMut(i32) -> i32) -> Option<Step> {
        self.switch = match op.name {
            "Switch" => vars.get(op.at(0)),
            "SwitchScenario" => vars.scenario(op.at(0)).0,
            "SwitchScenarioLevel" => vars.scenario(op.at(0)).1,
            "SwitchRandom" => random(op.value(0).max(1)),
            "SwitchSector" => vars.get(SECTOR),
            "SwitchDungeonMode" => vars.get_at(DUNGEON_MODES, op.at(0)),
            // "Was a job done in the dungeon just left?"
            // Whether the team is the hero and its partner alone (pmdsky-
            // debug's `SPECIAL_PROC_IS_TEAM_SETUP_HERO_AND_PARTNER_ONLY`):
            // it always is here, where recruits stay at the guild.
            "ProcessSpecial" if op.at(0) == TEAM_IS_THE_TWO => 1,
            // "Save and go back to the title" (pmdsky-debug's
            // `SPECIAL_PROC_JUMP_TO_TITLE_SCREEN`): what the story does on
            // coming back from the title is done at once: the two flags
            // of a save being made are cleared. Left set, they sent the
            // next of the story's events back to free roaming (the
            // ending, lost).
            // (The answer that asks it goes there: `zone::answers`.)
            "ProcessSpecial" if op.at(0) == JUMP_TO_TITLE => {
                vars.set_at(SCENARIO_MAIN_BIT_FLAG, SAVING, 0);
                vars.set_at(SCENARIO_MAIN_BIT_FLAG, SAVE_ASKED, 0);
                0
            }
            "ProcessSpecial" if op.at(0) == JOB_DONE => vars.get(JOB_DONE_THERE),
            // What the game's other menus and its processes answer is
            // not modelled: their first case.
            "ProcessSpecial" | "message_Menu" | "main_EnterRescueUser" | "main_EnterTraining" | "main_EnterTraining2" => 0,
            _ => return None,
        };
        Some(Step::On)
    }

    /// A menu of the game's own: its answer is read as a switch.
    pub(super) fn game_menu(&mut self, op: &Op<'_>) -> Option<Step> {
        if op.name != "message_Menu" {
            return None;
        }
        let menu = op.at(0);
        let known = Service::open(menu).is_some()
            || matches!(menu, quiz::MENU | JOB_BOARD | OUTLAW_BOARD | MENU_PLAYER_NAME | MENU_PARTNER_NAME | MENU_TEAM_NAME)
            || MENUS.contains(&menu);
        known.then(|| {
            self.switch = 0;
            Step::Ask(Event::GameMenu(menu))
        })
    }

    /// Leaving for a dungeon, as the scripts of the crossroads and of the
    /// dungeons' entrances do it: asking for the choice (the cases that
    /// follow read 0 for a dungeon chosen, 1 for none), checking a given
    /// dungeon (always fine here), then leaving for the one chosen through
    /// the routine that enters it.
    pub(super) fn enter_adventure(&mut self, op: &Op<'_>, vars: &mut Vars) -> Option<Step> {
        if op.name != "main_EnterAdventure" {
            return None;
        }
        Some(match op.at(0) {
            CHOOSE_DUNGEON => {
                self.choosing = true;
                Step::Ask(Event::GameMenu(DEPARTURE))
            }
            LEAVE | LEAVE_TOO => {
                let Some(common) = op.program.common.as_deref() else {
                    return Some(Step::End);
                };
                self.calls.clear();
                let chosen = vars.get(DUNGEON_SELECT);
                let known = common.routines.len() > ENTER_FREE_DUNGEON;
                let place = PLACES.contains(&(chosen.max(0) as u16));
                let routine = match vars.get_at(DUNGEON_MODES, chosen.max(0) as u16) {
                    _ if known && chosen == WAY_TO_SPRING as i32 => GETOUT_SCENARIO_DUNGEON,
                    _ if known && SPRINGS.contains(&chosen) => ENTER_REQUEST_DUNGEON,
                    mode if known && chosen >= 0 && (mode == MODE_OPEN || place) => ENTER_SCENARIO_DUNGEON,
                    _ => ENTER_FREE_DUNGEON,
                };
                if routine != ENTER_FREE_DUNGEON {
                    vars.set(DUNGEON_ENTER, chosen);
                }
                (self.common, self.pc) = (true, Self::first(common, routine));
                Step::On
            }
            dungeon => {
                vars.set(DUNGEON_SELECT, dungeon as i32);
                self.switch = 0;
                Step::On
            }
        })
    }
}
