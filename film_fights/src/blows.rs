//! The gestures round a blow: a boss's roar, power gathered for the next
//! move, a sidestep that makes a blow miss.

use engine::{Beat, Direction, FlowBeat, MoveId, SoundId, Spot, Who};

use super::Combat;
use film_core::emit;
use film_core::film::{Film, Sortie};
use film_core::spots::routes::found;
use film_core::verbs::Line;
use film_core::words::plain;

/// The game's own roar, as Sky's scripts play it for Groudon, Dialga and
/// Palkia (the game has no cry of each species).
const ROAR: &str = "roar";
/// How hard and how long the view shakes as one roars, and with `hard`.
const ROARING: (f32, f32) = (2.0, 40.0);
const ROARING_HARD: (f32, f32) = (4.0, 70.0);
/// The frames a roar lasts.
const ROAR_FRAMES: i64 = 60;

/// The game's own shine of power rising (Sky's stats-up effect), and its sound.
const SHINE: u16 = 691;
const SHINE_SOUND: u16 = 4881;
/// The frames power is gathered.
const CHARGE_FRAMES: i64 = 50;

/// How far a sidestep goes, and how high it hops.
const SIDESTEP: f32 = 24.0;
const SIDESTEP_HOP: f32 = 4.0;
const SIDESTEP_FRAMES: f32 = 10.0;
/// The sounds of a blow missing, as the fights play them.
const MISS_SOUND: u16 = 4870;

/// A blow just written, aimed at someone: a dodge right after it makes it
/// miss (`attack … at X`, then `X dodge`).
pub struct Blow {
    pub target: String,
    pub user: Who,
    pub id: MoveId,
    /// (Where the move, then its knock back and hurt, are written, and how
    /// far it went.)
    pub moved: usize,
    pub struck: std::ops::Range<usize>,
    pub written: usize,
    /// The frames from the move's start to its blow landing.
    pub strike: f32,
}

/// An unseen point put where someone stands now, for a move to land on
/// once they have stepped aside: its code.
fn where_stood(film: &mut Film, who: &str, me: Who) -> String {
    let at = found(film, who, Spot::Of(me), &["exact"]);
    let mark = format!("p{}", film_view::effects::FIRST_MARK + film.timing.marks);
    film.timing.marks += 1;
    film.on(&mark, |mark| emit::put_point(mark, at.clone(), Direction::Down));
    mark
}

/// Whether nothing but the start of a line was written since then: the
/// line at hand comes straight after the one that wrote up to there.
pub fn straight_after(film: &Film, written: usize) -> bool {
    film.written.get(written..).is_some_and(|since| since.iter().all(|(_, sortie)| matches!(sortie, Sortie::Beat(Beat::Flow(FlowBeat::Line { .. })))))
}

/// `dialga roar [hard]`: the game's roar, the view shaking (harder), the
/// sprite's own Roar animation where it has one, else a rise, gathering.
pub fn roared(film: &mut Film, line: &Line) {
    let who = plain(line.args.word("who").unwrap_or_default());
    let (strength, frames) = if line.args.has("hard") { ROARING_HARD } else { ROARING };
    let roar = film.actor(&who).and_then(|code| film_core::names::species(&code).and_then(|species| film.tables.roars.get(species)).copied());
    match roar {
        Some(group) => film.on(&who, |one| emit::anim_times(one, engine::AnimId(group), 1)),
        None => {
            if let Some(group) = emit::anim_of("Charge") {
                film.on(&who, |one| emit::anim_times(one, group, 1));
            }
            film.jump(&who, 6.0, 30.0);
        }
    }
    film.sound(ROAR);
    film.beat(emit::shake(2, strength, Some(frames)));
    film.wait_frames(ROAR_FRAMES.max(frames as i64), line.wait);
}

/// `pikachu charge [Thunderbolt]`: his Charge animation, the game's shine
/// of power rising on him, held a beat; a move told, his next attack with it plays stronger.
pub fn charged(film: &mut Film, line: &Line) {
    let who = plain(line.args.word("who").unwrap_or_default());
    let named = line.args.words("move").join(" ");
    if !named.is_empty() {
        let Some(id) = super::move_id(film, &named) else {
            let tables = film.tables.clone();
            film.unknown(line.number, "unknown move", &named, tables.moves.keys().map(String::as_str), "");
            return;
        };
        film.kept::<Combat>().charged.insert(who.clone(), id);
    }
    if let Some(group) = emit::anim_of("Charge") {
        film.on(&who, |one| emit::anim(one, group, true, false));
    }
    film.on(&who, |one| emit::emote(one, engine::EmoteId(SHINE), Some(CHARGE_FRAMES as f32)));
    film.beat(emit::sound(SoundId(SHINE_SOUND)));
    film.wait_frames(CHARGE_FRAMES, line.wait);
    film.on(&who, |one| emit::pose(one, engine::Pose::Idle));
}

/// What a charged move adds as it lands: the screen white, the view shaking hard.
pub fn strong(film: &mut Film, at: f32) {
    film.beat(emit::after(at, 0.0, emit::flash([255.0, 255.0, 255.0], 12.0)));
    film.beat(emit::after(at, 0.0, emit::quake(4.0, 24.0)));
    film.beat(emit::after(at, 0.0, emit::sound(SoundId(rules::songs::noise::JOLT))));
}

/// What shows of a blow that misses someone: "Missed!" over them, its sound.
pub fn missed(film: &mut Film, target: &str, at: f32) {
    let green = emit::colour(engine::Colour::Green);
    film.on(target, |one| emit::after(at + 2.0, 0.0, emit::popup(one, green, "Missed!")));
    film.beat(emit::after(at + 2.0, 0.0, emit::sound(SoundId(MISS_SOUND))));
}

/// `piplup dodge [left|right|back]`: a quick hop aside. Right after an
/// attack at him, that blow misses: he hops aside just before it lands, no
/// knock back, "Missed!"; before one, the next attack at him misses.
pub fn dodged(film: &mut Film, line: &Line) {
    let who = plain(line.args.word("who").unwrap_or_default());
    let Some(me) = film.other(&who) else {
        film.not_a(&who, "a Pokémon of the scene");
        return;
    };
    let blow = film.kept::<Combat>().blow.take().filter(|blow| blow.target == who && straight_after(film, blow.written));
    let from = blow.as_ref().map(|blow| blow.user);
    let at = blow.as_ref().map_or(0.0, |blow| (blow.strike - SIDESTEP_FRAMES).max(0.0));
    let start = film.written.len();
    // (The move aimed where he stood: it lands there, he is gone.)
    let stood = where_stood(film, &who, me);
    if let Some(blow) = &blow {
        let aimed = film.other(&stood);
        film.beat(emit::uses_move(blow.user, blow.id, aimed));
    }
    // (Turned to who strikes, to step back away from them.)
    if let Some(from) = from {
        film.beat(emit::face(me, from));
    }
    let spot = match () {
        _ if line.args.has("left") => Spot::By { of: me, dx: -SIDESTEP, dy: 0.0 },
        _ if line.args.has("right") => Spot::By { of: me, dx: SIDESTEP, dy: 0.0 },
        _ => Spot::Before { of: me, back: true },
    };
    let to = found(film, &who, spot, &["exact"]);
    film.beat(emit::after(at, 0.0, emit::hop_point(me, to, SIDESTEP_HOP, SIDESTEP_FRAMES)));
    if let Some(group) = emit::anim_of("Hop") {
        film.beat(emit::after(at, 0.0, emit::anim_times(me, group, 1)));
    }
    match blow {
        // (The blow written already: its knock back and hurt become the
        // sidestep, played as it was about to land.)
        Some(blow) => {
            missed(film, &who, blow.strike);

            let dodge: Vec<_> = film.written.drain(start..).collect();
            film.written.splice(blow.moved..blow.struck.end, dodge);
        }
        None => {
            film.wait_frames(SIDESTEP_FRAMES as i64 + 4, line.wait);
            film.kept::<Combat>().dodging = Some((who, film.written.len(), stood));
        }
    }
}
