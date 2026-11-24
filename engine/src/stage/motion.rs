//! What changes by itself over time: a value going somewhere, a swing, a
//! box things are in.

use crate::base::Ease;

/// What can go gradually from a value to another.
pub trait Mix: Copy {
    /// Goes that part of the way to the other.
    fn mix(&mut self, to: Self, part: f32);
}

impl Mix for f32 {
    fn mix(&mut self, to: Self, part: f32) {
        *self += (to - *self) * part;
    }
}

impl Mix for [f32; 3] {
    fn mix(&mut self, to: Self, part: f32) {
        for (now, to) in self.iter_mut().zip(to) {
            now.mix(to, part);
        }
    }
}

/// How far along something is that starts and ends softly (both from 0 to
/// 1): the engine's own curve, `Ease::Smooth`.
pub(crate) fn eased(done: f32) -> f32 {
    Ease::Smooth.along(done)
}

/// A value on its way, starting and ending softly (or by its ease): where
/// it goes, the frames that takes still, and all it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tween<T> {
    pub to: T,
    pub left: f32,
    whole: f32,
    pub ease: Ease,
}

impl<T: Mix> Tween<T> {
    pub fn new(to: T, frames: f32) -> Self {
        Tween { to, left: frames, whole: frames, ease: Ease::Smooth }
    }

    /// The same, going by that ease.
    pub fn eased(to: T, frames: f32, ease: Ease) -> Self {
        Tween { ease, ..Tween::new(to, frames) }
    }

    /// Takes a value that many frames further; over, gone.
    pub fn run(slot: &mut Option<Self>, now: &mut T, frames: f32) {
        if let Some(Tween { to, left, whole, ease }) = *slot {
            // (Steps of a window are not whole frames: what is left may be a
            // crumb, and the part of the way 0 / 0. The last step lands.)
            let after_left = left - frames.min(left);
            let before = ease.along(1.0 - left / whole);
            if after_left <= f32::EPSILON * whole || 1.0 - before <= f32::EPSILON {
                *now = to;
                *slot = None;
                return;
            }
            now.mix(to, (ease.along(1.0 - after_left / whole) - before) / (1.0 - before));
            *slot = Some(Tween { to, left: after_left, whole, ease });
        }
    }
}

/// Something going to and fro for ever: how far, the frames a swing takes,
/// and those gone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swing {
    pub height: f32,
    pub frames: f32,
    pub gone: f32,
}

impl Swing {
    pub fn new(height: f32, frames: f32) -> Option<Self> {
        (height > 0.0).then_some(Swing { height, frames: frames.max(1.0), gone: 0.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tween_gets_there_and_goes() {
        let (mut now, mut tween) = (0.0, Some(Tween::new(10.0, 4.0)));
        Tween::run(&mut tween, &mut now, 1.0);
        assert!(now > 0.0 && now < 2.5, "it starts softly");
        Tween::run(&mut tween, &mut now, 1.0);
        assert!((now - 5.0_f32).abs() < 0.001);
        Tween::run(&mut tween, &mut now, 3.0);
        assert_eq!((now, tween), (10.0, None));
    }

    #[test]
    fn a_tween_stepped_as_a_window_steps_lands_where_it_goes() {
        // (Steps that are not whole frames leave a crumb: 0 / 0 once made
        // the colour of an evolved Pokémon black for good.)
        let steps = [1.0, 1.3, 0.7, 2.1, 0.9, 1.6667, 0.3333];
        for whole in [12.0, 30.0, 40.0] {
            let (mut now, mut tween) = ([0.0; 3], Some(Tween::new([255.0; 3], whole)));
            for step in steps.iter().cycle().take(200) {
                Tween::run(&mut tween, &mut now, *step * 0.5);
                assert!(now.iter().all(|part| part.is_finite()), "{now:?}");
            }
            assert_eq!((now, tween), ([255.0; 3], None));
        }
    }
}
