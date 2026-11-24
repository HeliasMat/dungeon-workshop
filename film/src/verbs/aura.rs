//! An ambience by type, never by itself: `aura on pikachu` and his kind's
//! type shows round him (the game's tables say which): a fire kind's few
//! cinders about its body, a spark off an electric kind when it is angry,
//! a drop off a water kind when it is scared, leaves off a grass kind
//! when it dances. Sharp pixels only. `aura off pikachu`: no more.

use engine::{Cam, ParticleKind};

use super::Line;
use crate::emit;
use crate::film::Film;
use crate::words::plain;

/// The game's types an aura shows.
const FIRE: u8 = 2;
const WATER: u8 = 3;
const GRASS: u8 = 4;
const ELECTRIC: u8 = 5;
/// A fire kind's cinders, so many a second.
const CINDERS: f32 = 3.0;
/// So many sparks when angry, drops when scared, leaves when dancing.
const ZAPS: u16 = 6;
const DROPS: u16 = 1;
const LEAVES: u16 = 5;

/// What one shows that an aura answers.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Angry,
    Scared,
    Dancing,
}

/// The aura's type of someone's kind: the first of its types an aura shows.
fn aura_type(film: &mut Film, who: &str) -> Option<u8> {
    let code = film.actor(who)?;
    let (first, second) = film.kind_of(&code)?.types;
    [first, second].into_iter().find(|kind| [FIRE, WATER, GRASS, ELECTRIC].contains(kind))
}

/// `aura on pikachu`, `aura off pikachu`.
pub(super) fn aura(film: &mut Film, line: &Line) {
    let who = plain(line.args.word("who").unwrap_or_default());
    if line.args.has("off") {
        if film.staging.auras.remove(&who) == Some(FIRE) {
            film.on(&who, emit::emitter_off);
        }
        return;
    }
    let Some(kind) = aura_type(film, &who) else {
        film.remark(line.number, format!("aura on {who}: his kind is of no type an aura shows (fire, electric, water, grass)"));
        return;
    };
    if film.staging.auras.insert(who.clone(), kind) != Some(FIRE) && kind == FIRE {
        film.on(&who, |one| emit::emitter(ParticleKind::Cinders, one, CINDERS, None, None));
    }
}

/// Someone shows a mood: his aura answers it, if it is his type's.
pub fn moved(film: &mut Film, who: &str, mood: Mood) {
    let (kind, count) = match (film.staging.auras.get(who).copied(), mood) {
        (Some(ELECTRIC), Mood::Angry) => (ParticleKind::Zap, ZAPS),
        (Some(WATER), Mood::Scared) => (ParticleKind::Drip, DROPS),
        (Some(GRASS), Mood::Dancing) => (ParticleKind::Leaves, LEAVES),
        _ => return,
    };
    film.on(who, |one| emit::particles(kind, Cam::Who(one), count, None));
}

/// The mood a line's face shows, if an aura answers it.
pub fn mood_of_face(face: &str) -> Option<Mood> {
    match face {
        "angry" | "shouting" => Some(Mood::Angry),
        "worried" | "scared" => Some(Mood::Scared),
        _ => None,
    }
}
