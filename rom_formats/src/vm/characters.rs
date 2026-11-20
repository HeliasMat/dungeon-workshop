//! Staging: what the characters do on the stage.

use super::{Act, Cond, Event, Look, Op, Place, act, mark, movement, number};

/// `TurnDirection`'s direction standing for "towards the talker".
const TOWARDS_TALKER: u16 = 0x7FFF;

/// What a character of the scene is told to do.
pub(super) fn characters(op: &Op<'_>) -> Option<Event> {
    Some(match op.name {
        "SetPositionInitial" => act(Act::Position(Place::Initial)),
        "SetPositionMark" if op.p.len() >= 4 => act(Act::Position(mark(op.p))),
        "SetPositionOffset" => act(Act::Position(Place::By(number(op.at(0)), number(op.at(1))))),
        "SetPosition" => act(Act::Position(Place::At(number(op.at(0)), number(op.at(1))))),
        "SetPositionLives" => act(Act::Position(Place::Actor(op.at(0)))),
        "SetDirection" | "TurnDirection" => act(Act::Direction(match op.at(0) {
            TOWARDS_TALKER => Look::Talker,
            direction => Look::Direction(direction + 1),
        })),
        "SetDirectionLives" => act(Act::Direction(Look::Actor(op.at(0)))),
        "Turn2Direction" => act(Act::Turn { frames: op.at(0), to: Look::Direction(op.at(2) + 1) }),
        "Turn2DirectionLives" | "Turn2DirectionLives2" => act(Act::Turn { frames: op.at(0), to: Look::Actor(op.at(2)) }),
        "SetAnimation" => act(Act::Animation(op.at(0))),
        "SetOutputAttribute" => act(Act::Attributes { set: op.at(0), reset: 0, animation: None }),
        "ResetOutputAttribute" => act(Act::Attributes { set: 0, reset: op.at(0), animation: None }),
        "SetupOutputAttributeAndAnimation" => act(Act::Attributes { set: op.at(0), reset: op.at(1), animation: Some(op.at(2)) }),
        "SetBlink" => act(Act::Blink(op.at(0), op.at(1))),
        "SetEffect" => act(Act::Effect(op.at(0))),
        "WaitEffect" => Event::WaitFor(Cond::Effect),
        "MoveHeight" => act(Act::Height { speed: number(op.at(0)).max(0.25), height: number(op.at(1)) }),
        "SetHeight" => act(Act::Height { speed: f32::MAX, height: number(op.at(0)) }),
        "WaitAnimation" | "WaitEndAnimation" => Event::WaitFor(Cond::Animation),

        "SlideHeight" => act(Act::Height { speed: number(op.at(0)).max(0.25), height: number(op.at(1)) }),
        // (Turning by so many eighths from where it faces.)
        "Turn2DirectionTurn" => act(Act::Turn { frames: op.at(0), to: Look::Around(op.at(2)) }),
        _ => return movement(op.name, op.p).map(act),
    })
}
