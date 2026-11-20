//! What scripts write in the game's variables, and going to a level or a
//! dungeon.

use super::{
    DUNGEON_CHOSEN, DUNGEON_ENTER, DUNGEON_ENTER_INDEX, DUNGEON_ENTER_MODE, DUNGEON_MODES, DUNGEON_SELECT, Event, GROUND_ENTER, Op, PERFORMANCE, Step, Vars,
    calculate,
};

pub(super) fn flags(op: &Op<'_>, vars: &mut Vars) -> Option<Step> {
    match op.name {
        "flag_Set" => vars.set(op.at(0), op.value(1)),
        "flag_Clear" | "flag_Initial" => vars.set(op.at(0), 0),
        "flag_CalcValue" => vars.set(op.at(0), calculate(vars.get(op.at(0)), op.at(1), op.value(2))),
        "flag_CalcVariable" => vars.set(op.at(0), calculate(vars.get(op.at(0)), op.at(1), vars.get(op.at(2)))),
        "flag_CalcBit" => vars.set_at(op.at(0), op.at(1), op.value(2)),
        "flag_SetScenario" => {
            vars.set_at(op.at(0), 0, op.value(1));
            vars.set_at(op.at(0), 1, op.value(2));
        }
        "flag_ResetScenario" => {
            vars.set_at(op.at(0), 0, 0);
            vars.set_at(op.at(0), 1, 0);
        }
        "flag_SetDungeonMode" => vars.set_at(DUNGEON_MODES, op.at(0), op.value(1)),
        "flag_SetPerformance" => vars.set_at(PERFORMANCE, op.at(0), op.value(1)),
        _ => return None,
    }
    Some(Step::On)
}

/// Going to a level, or a dungeon. (The routines run when the dungeon is
/// left look at which it was.)
pub(super) fn enter(op: &Op<'_>, vars: &mut Vars) -> Option<Event> {
    match op.name {
        "main_EnterGround" => Some(Event::EnterGround(op.at(0))),
        "main_EnterDungeon" => {
            if op.at(0) != DUNGEON_CHOSEN {
                vars.set(DUNGEON_SELECT, op.value(0));
            }
            let dungeon = vars.get(DUNGEON_SELECT);
            // No dungeon was chosen: the leader stays.
            if dungeon < 0 {
                return Some(Event::EnterGround(vars.get(GROUND_ENTER) as u16));
            }
            vars.set(DUNGEON_ENTER, dungeon);
            vars.set(DUNGEON_ENTER_INDEX, dungeon);
            vars.set(DUNGEON_ENTER_MODE, vars.get_at(DUNGEON_MODES, dungeon as u16));
            Some(Event::EnterDungeon(dungeon as u16))
        }
        _ => None,
    }
}
