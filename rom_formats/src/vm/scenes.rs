//! Scenes: who comes on, who leaves, who waits for whom.

use super::{Act, Cast, Cond, ENTER, Event, GROUND_ENTER, HERE, Op, SECTOR, Vars, Who, act};

/// The scene itself: what is loaded, played and removed.
pub(super) fn scenes(op: &Op<'_>, vars: &mut Vars) -> Option<Event> {
    Some(match op.name {
        "main_SetGround" => {
            vars.set(GROUND_ENTER, op.value(0));
            act(Act::Ground(op.at(0)))
        }
        "supervision_LoadStation" => act(Act::LoadStation { level: op.at(0), name: op.constant(op.at(1)).to_lowercase() }),
        "supervision_Acting" => act(Act::Layer { cast: Cast::Acting, layer: op.at(0) }),
        // (The same, without anyone of the layer showing yet.)
        "supervision_ActingInvisible" => act(Act::LayerHidden { cast: Cast::Acting, layer: op.at(0) }),
        "supervision_Station" => act(Act::Layer { cast: Cast::Station, layer: op.at(0) }),
        "supervision_StationCommon" => act(Act::Layer { cast: Cast::Common, layer: op.at(0) }),
        // Free roaming where the leader is, or in a level.
        "supervision_ExecuteEnter" => Event::EnterGround(match op.at(0) {
            HERE => vars.get(GROUND_ENTER) as u16,
            level => level,
        }),
        // One layer of the cutscene's characters leaves; all of them
        // with a number below zero.
        "supervision_RemoveActing" => act(match op.at(0) >= 0x8000 {
            true => Act::RemoveActing,
            false => Act::RemoveLayer { cast: Cast::Acting, layer: op.at(0) },
        }),
        "supervision_Remove" => act(Act::RemoveLayer { cast: Cast::Station, layer: op.at(0) }),
        "supervision_ExecuteActing" | "supervision_ExecuteActingSub" => {
            vars.set(SECTOR, op.value(2));
            op.scene(false, op.name == "supervision_ExecuteActingSub")
        }
        "supervision_ExecuteStation" | "supervision_ExecuteStationSub" => {
            vars.set(SECTOR, op.value(2));
            op.scene(true, op.name == "supervision_ExecuteStationSub")
        }
        // The level's own scene.
        "supervision_ExecuteStationCommon" => act(Act::Scene { level: op.at(0), name: ENTER.to_string(), station: true, sub: false }),

        _ => return None,
    })
}

/// Waiting for another routine, or for a lock.
pub(super) fn waits(op: &Op<'_>) -> Option<Event> {
    Some(match op.name {
        "WaitExecuteLives" => Event::WaitFor(Cond::Routine(Who::Actor(op.at(0)))),
        "WaitExecuteObject" => Event::WaitFor(Cond::Routine(Who::Object(op.at(0)))),
        "WaitExecutePerformer" => Event::WaitFor(Cond::Routine(Who::Performer(op.at(0)))),
        "Lock" => Event::WaitFor(Cond::Lock(op.at(0))),
        "Unlock" => Event::Unlock(op.at(0)),
        "WaitLockLives" => Event::WaitFor(Cond::Locked(Who::Actor(op.at(1)), op.at(0))),
        "WaitLockObject" => Event::WaitFor(Cond::Locked(Who::Object(op.at(1)), op.at(0))),
        "WaitLockPerformer" => Event::WaitFor(Cond::Locked(Who::Performer(op.at(1)), op.at(0))),
        _ => return None,
    })
}

impl Op<'_> {
    /// The scene of a level, named by the second parameter.
    fn scene(&self, station: bool, sub: bool) -> Event {
        act(Act::Scene { level: self.at(0), name: self.constant(self.at(1)).to_lowercase(), station, sub })
    }
}
