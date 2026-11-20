//! Floor weather: the one at work, how long a weather brought by a move
//! lasts, and the regular sting of sandstorms and hail.

use super::rng::DungeonRng;

pub const CLEAR: u8 = 0;
pub const SUNNY: u8 = 1;
pub const SANDSTORM: u8 = 2;
pub(crate) const CLOUDY: u8 = 3;
pub const RAIN: u8 = 4;
pub const HAIL: u8 = 5;
pub const FOG: u8 = 6;
pub(crate) const SNOW: u8 = 7;
/// In floor data: one of the eight above, drawn as the floor starts.
pub(crate) const RANDOM: u8 = 8;
const KINDS: usize = 8;

/// Sandstorms and hail sting once every this many turns.
const STING_EVERY: i32 = 10;

const TYPE_ICE: u8 = 6;
const TYPE_GROUND: u8 = 9;
const TYPE_ROCK: u8 = 13;
const TYPE_STEEL: u8 = 17;

#[derive(Clone, Copy, Debug)]
pub struct Weather {
    pub current: u8,
    /// Cloud Nine or Air Lock is on the floor: the weather does nothing.
    pub suppressed: bool,
    /// Damp is on the floor: nothing explodes.
    pub damp: bool,
    /// The weather a move brings stays until another comes
    /// (`RuleOptions::lasting_weather`).
    pub lasting: bool,
    /// The floor's own weather, back once the brought ones are over.
    natural: u8,
    /// Turns left of each weather brought by a move or an ability.
    brought: [i32; KINDS],
    /// Turns since the last sting.
    since_sting: i32,
}

impl Default for Weather {
    fn default() -> Self {
        Self { current: CLEAR, suppressed: false, damp: false, lasting: false, natural: CLEAR, brought: [0; KINDS], since_sting: 0 }
    }
}

impl Weather {
    /// The weather as monsters feel it: clear while suppressed.
    pub(crate) fn apparent(&self) -> u8 {
        if self.suppressed { CLEAR } else { self.current }
    }

    /// The weather a floor starts with.
    pub fn new(natural: u8, rng: &mut DungeonRng) -> Self {
        let natural = match natural {
            RANDOM => rng.rand_int(KINDS as i32) as u8,
            kind if usize::from(kind) < KINDS => kind,
            _ => CLEAR,
        };
        Self { current: natural, natural, ..Self::default() }
    }

    /// A move or an ability brings a weather for `turns` turns.
    pub fn set(&mut self, kind: u8, turns: i32) {
        // (Lasting, it becomes the floor's own.)
        if self.lasting && usize::from(kind) < KINDS {
            (self.natural, self.brought) = (kind, [0; KINDS]);
            self.current = self.strongest();
            return;
        }
        if let Some(left) = self.brought.get_mut(usize::from(kind)) {
            *left = turns;
        }
        self.current = self.strongest();
    }

    /// The brought weather with the most turns left (the lower number on a
    /// tie), or the floor's own when none is left.
    fn strongest(&self) -> u8 {
        let most = self.brought.iter().copied().max().unwrap_or(0);
        if most <= 0 {
            return self.natural;
        }
        self.brought.iter().position(|&left| left == most).map_or(self.natural, |kind| kind as u8)
    }

    /// A turn ends. Returns whether a sandstorm or hail stings now.
    pub(crate) fn end_turn(&mut self) -> bool {
        self.brought.iter_mut().for_each(|left| *left = (*left - 1).max(0));
        self.current = self.strongest();
        self.since_sting += 1;
        if self.since_sting < STING_EVERY {
            return false;
        }
        self.since_sting = 0;
        matches!(self.current, SANDSTORM | HAIL)
    }

    /// Whether a monster of these types is hurt when the weather stings:
    /// hail spares Ice types, sandstorms Ground, Rock and Steel types.
    pub fn hurts(&self, types: (u8, u8)) -> bool {
        let spared: &[u8] = match self.current {
            HAIL => &[TYPE_ICE],
            SANDSTORM => &[TYPE_GROUND, TYPE_ROCK, TYPE_STEEL],
            _ => return false,
        };
        !spared.contains(&types.0) && !spared.contains(&types.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lasting_weather_stays_until_another_comes() {
        let mut rng = DungeonRng::new(1);
        let mut weather = Weather::new(FOG, &mut rng);
        weather.lasting = true;
        weather.set(RAIN, 3);
        for _ in 0..50 {
            weather.end_turn();
        }
        assert_eq!(weather.current, RAIN);
        weather.set(SUNNY, 3);
        for _ in 0..50 {
            weather.end_turn();
        }
        assert_eq!(weather.current, SUNNY);
    }

    #[test]
    fn brought_weather_wins_over_the_floor_then_fades() {
        let mut rng = DungeonRng::new(1);
        let mut weather = Weather::new(FOG, &mut rng);
        assert_eq!(weather.current, FOG);
        weather.set(RAIN, 3);
        weather.set(SUNNY, 5);
        assert_eq!(weather.current, SUNNY);
        weather.set(HAIL, 5);
        // Even turns left: the lower number stays.
        assert_eq!(weather.current, SUNNY);
        for _ in 0..5 {
            weather.end_turn();
        }
        assert_eq!(weather.current, FOG);
        for seed in 0..50 {
            assert!(Weather::new(RANDOM, &mut DungeonRng::new(seed)).current < 8);
        }
        assert_eq!(Weather::new(200, &mut rng).current, CLEAR);
    }

    #[test]
    fn bad_weather_stings_every_ten_turns() {
        let mut rng = DungeonRng::new(1);
        let mut weather = Weather::new(SANDSTORM, &mut rng);
        let stings = (0..40).filter(|_| weather.end_turn()).count();
        assert_eq!(stings, 4);
        assert!(weather.hurts((1, 0)) && !weather.hurts((13, 0)) && !weather.hurts((1, 17)));
        let hail = Weather::new(HAIL, &mut rng);
        assert!(hail.hurts((13, 0)) && !hail.hurts((6, 0)));
        let mut clear = Weather::new(CLEAR, &mut rng);
        assert!((0..40).all(|_| !clear.end_turn()) && !clear.hurts((1, 0)));
    }
}
