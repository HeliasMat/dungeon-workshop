//! How a value goes from a key to the next in time ([`Ease`]): evenly,
//! softly at both ends (the engine's own curve, the default), in, out or
//! in and out by a family of curves ([`EaseFamily`]: sine, quad, cubic,
//! quart, expo, circ, back, elastic, bounce), in steps, or by a bezier of
//! its own. One type for all that moves: tracks (`anim`, `keys`), the
//! tweens of a body (`Tween`), the camera's glides.
//!
//! As written: `linear`, `smooth`, `out` (out quad), `in_quad`,
//! `out_back`, `inout_sine` (alone, `in`, `inout`: cubic; `back`,
//! `elastic`, `bounce` alone: out), `steps(4)` (or `steps 4`),
//! `bezier(0.4,0,0.2,1)` (or `bezier 0.4 0 0.2 1`).

use std::f32::consts::PI;
use std::fmt;

/// A family of curves, each played in, out, or in and out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EaseFamily {
    Sine,
    Quad,
    Cubic,
    Quart,
    Expo,
    Circ,
    /// It goes a little back before going (in), or past and back (out).
    Back,
    /// It springs about its end.
    Elastic,
    /// It bounces at its end.
    Bounce,
}

impl EaseFamily {
    pub const ALL: [EaseFamily; 9] = [
        EaseFamily::Sine,
        EaseFamily::Quad,
        EaseFamily::Cubic,
        EaseFamily::Quart,
        EaseFamily::Expo,
        EaseFamily::Circ,
        EaseFamily::Back,
        EaseFamily::Elastic,
        EaseFamily::Bounce,
    ];

    /// Its name, as the engine's lines write it.
    pub fn name(self) -> &'static str {
        match self {
            EaseFamily::Sine => "sine",
            EaseFamily::Quad => "quad",
            EaseFamily::Cubic => "cubic",
            EaseFamily::Quart => "quart",
            EaseFamily::Expo => "expo",
            EaseFamily::Circ => "circ",
            EaseFamily::Back => "back",
            EaseFamily::Elastic => "elastic",
            EaseFamily::Bounce => "bounce",
        }
    }

    fn named(name: &str) -> Option<EaseFamily> {
        EaseFamily::ALL.into_iter().find(|family| family.name() == name)
    }

    /// The curve played in: from 0 to 1, slowly first.
    fn eased_in(self, t: f32) -> f32 {
        match self {
            EaseFamily::Sine => 1.0 - (t * PI / 2.0).cos(),
            EaseFamily::Quad => t * t,
            EaseFamily::Cubic => t * t * t,
            EaseFamily::Quart => t * t * t * t,
            EaseFamily::Expo if t <= 0.0 => 0.0,
            EaseFamily::Expo => 2f32.powf(10.0 * t - 10.0),
            EaseFamily::Circ => 1.0 - (1.0 - t * t).max(0.0).sqrt(),
            EaseFamily::Back => {
                const C1: f32 = 1.70158;
                (C1 + 1.0) * t * t * t - C1 * t * t
            }
            EaseFamily::Elastic if t <= 0.0 || t >= 1.0 => t,
            EaseFamily::Elastic => -(2f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * (2.0 * PI / 3.0)).sin(),
            EaseFamily::Bounce => 1.0 - bounced(1.0 - t),
        }
    }
}

/// The bounce played out (as easings.net has it).
fn bounced(t: f32) -> f32 {
    const N: f32 = 7.5625;
    const D: f32 = 2.75;
    if t < 1.0 / D {
        N * t * t
    } else if t < 2.0 / D {
        let t = t - 1.5 / D;
        N * t * t + 0.75
    } else if t < 2.5 / D {
        let t = t - 2.25 / D;
        N * t * t + 0.9375
    } else {
        let t = t - 2.625 / D;
        N * t * t + 0.984375
    }
}

/// How a value goes from a key to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Ease {
    /// Evenly.
    Linear,
    /// Slowly at both ends: the engine's own (a smoothstep), the default.
    #[default]
    Smooth,
    In(EaseFamily),
    Out(EaseFamily),
    InOut(EaseFamily),
    /// In so many steps, each held (the last at the end).
    Steps(u16),
    /// A cubic bezier from (0, 0) to (1, 1) by (x1, y1) and (x2, y2), as
    /// CSS has it.
    Bezier([f32; 4]),
}

impl Ease {
    /// How far along it is (0 to 1, past them for back and elastic), that
    /// much of its time gone (0 to 1).
    pub fn along(self, done: f32) -> f32 {
        let t = if done.is_nan() { 0.0 } else { done.clamp(0.0, 1.0) };
        match self {
            Ease::Linear => t,
            Ease::Smooth => t * t * (3.0 - 2.0 * t),
            Ease::In(family) => family.eased_in(t),
            Ease::Out(family) => 1.0 - family.eased_in(1.0 - t),
            Ease::InOut(family) if t < 0.5 => family.eased_in(2.0 * t) / 2.0,
            Ease::InOut(family) => 1.0 - family.eased_in(2.0 - 2.0 * t) / 2.0,
            Ease::Steps(0) => t,
            Ease::Steps(steps) => (t * f32::from(steps)).floor() / f32::from(steps),
            Ease::Bezier([x1, y1, x2, y2]) => bezier(t, x1, y1, x2, y2),
        }
    }

    /// An ease as words write it, from the first: it and how many words
    /// it took; none if they say none.
    pub fn read(words: &[String]) -> Option<(Ease, usize)> {
        let first = words.first()?.to_lowercase();
        let first = first.as_str();
        let number = |at: usize| words.get(at).and_then(|word| word.trim_end_matches(',').parse::<f32>().ok()).filter(|number| number.is_finite());
        if let Some(inside) = first.strip_prefix("steps(") {
            return Some((Ease::Steps(inside.strip_suffix(')')?.parse().ok().filter(|steps| *steps > 0)?), 1));
        }
        if first == "steps" {
            return Some((Ease::Steps(number(1).filter(|steps| *steps >= 1.0 && steps.fract() == 0.0)? as u16), 2));
        }
        if let Some(inside) = first.strip_prefix("bezier(") {
            let numbers: Option<Vec<f32>> = inside.strip_suffix(')')?.split(',').map(|part| part.trim().parse().ok()).collect();
            return Some((Ease::Bezier(numbers?.try_into().ok()?), 1));
        }
        if first == "bezier" {
            return Some((Ease::Bezier([number(1)?, number(2)?, number(3)?, number(4)?]), 5));
        }
        let ease = match first {
            "linear" => Ease::Linear,
            "smooth" => Ease::Smooth,
            "out" => Ease::Out(EaseFamily::Quad),
            "in" => Ease::In(EaseFamily::Cubic),
            "inout" => Ease::InOut(EaseFamily::Cubic),
            name => {
                if let Some(family) = EaseFamily::named(name) {
                    // (A family alone: the springy ones out, the others in and out.)
                    return Some((
                        match family {
                            EaseFamily::Back | EaseFamily::Elastic | EaseFamily::Bounce => Ease::Out(family),
                            other => Ease::InOut(other),
                        },
                        1,
                    ));
                }
                let (way, family) = name.rsplit_once('_')?;
                let family = EaseFamily::named(family)?;
                match way {
                    "in" => Ease::In(family),
                    "out" => Ease::Out(family),
                    "inout" | "in_out" => Ease::InOut(family),
                    _ => return None,
                }
            }
        };
        Some((ease, 1))
    }

    /// Every name of an ease a film writes (for the docs and the problems).
    pub fn names() -> Vec<String> {
        let mut names: Vec<String> = ["linear", "smooth", "out", "in", "inout"].map(str::to_string).to_vec();
        for family in EaseFamily::ALL {
            names.extend(["in", "out", "inout"].map(|way| format!("{way}_{}", family.name())));
        }
        names.extend(["back", "elastic", "bounce", "steps(4)", "bezier(0.4,0,0.2,1)"].map(str::to_string));
        names
    }
}

/// As the engine's lines write it, in one word; [`Ease::read`] reads it back.
impl fmt::Display for Ease {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ease::Linear => write!(out, "linear"),
            Ease::Smooth => write!(out, "smooth"),
            Ease::In(family) => write!(out, "in_{}", family.name()),
            Ease::Out(family) => write!(out, "out_{}", family.name()),
            Ease::InOut(family) => write!(out, "inout_{}", family.name()),
            Ease::Steps(steps) => write!(out, "steps({steps})"),
            Ease::Bezier([x1, y1, x2, y2]) => write!(out, "bezier({x1},{y1},{x2},{y2})"),
        }
    }
}

/// The height of a cubic bezier (CSS's `cubic-bezier`) where it is `x`
/// along: its parameter found by Newton's steps, then by halves.
fn bezier(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let curve = |t: f32, a: f32, b: f32| {
        let u = 1.0 - t;
        3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
    };
    let slope = |t: f32, a: f32, b: f32| {
        let u = 1.0 - t;
        3.0 * u * u * a + 6.0 * u * t * (b - a) + 3.0 * t * t * (1.0 - b)
    };
    let mut t = x;
    for _ in 0..8 {
        let off = curve(t, x1, x2) - x;
        if off.abs() < 1e-6 {
            return curve(t, y1, y2);
        }
        let d = slope(t, x1, x2);
        if d.abs() < 1e-6 {
            break;
        }
        t = (t - off / d).clamp(0.0, 1.0);
    }
    let (mut low, mut high) = (0.0f32, 1.0f32);
    t = x;
    for _ in 0..32 {
        let at = curve(t, x1, x2);
        if (at - x).abs() < 1e-6 {
            break;
        }
        if at < x {
            low = t;
        } else {
            high = t;
        }
        t = (low + high) / 2.0;
    }
    curve(t, y1, y2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every() -> Vec<Ease> {
        let mut all = vec![Ease::Linear, Ease::Smooth, Ease::Steps(4), Ease::Bezier([0.4, 0.0, 0.2, 1.0]), Ease::Bezier([0.0, 0.0, 1.0, 1.0])];
        for family in EaseFamily::ALL {
            all.extend([Ease::In(family), Ease::Out(family), Ease::InOut(family)]);
        }
        all
    }

    #[test]
    fn every_ease_starts_at_0_and_ends_at_1() {
        for ease in every() {
            assert!(ease.along(0.0).abs() < 1e-4, "{ease} starts at {}", ease.along(0.0));
            assert!((ease.along(1.0) - 1.0).abs() < 1e-4, "{ease} ends at {}", ease.along(1.0));
            assert!(ease.along(f32::NAN).abs() < 1e-4 && (ease.along(2.0) - 1.0).abs() < 1e-4, "{ease} out of 0..1");
        }
    }

    #[test]
    fn each_ease_has_its_shape() {
        let half = |ease: Ease| ease.along(0.5);
        assert_eq!(half(Ease::Linear), 0.5);
        assert_eq!(half(Ease::Smooth), 0.5);
        assert!(Ease::Smooth.along(0.25) < 0.25 && Ease::Smooth.along(0.75) > 0.75);
        for family in EaseFamily::ALL.into_iter().filter(|family| !matches!(family, EaseFamily::Back | EaseFamily::Elastic)) {
            assert!(Ease::In(family).along(0.3) < 0.3, "in {family:?} starts slowly");
            assert!(Ease::Out(family).along(0.3) > 0.3, "out {family:?} starts quickly");
            assert!((half(Ease::InOut(family)) - 0.5).abs() < 1e-4, "inout {family:?} is halfway at half");
        }
        assert!((Ease::In(EaseFamily::Quad).along(0.5) - 0.25).abs() < 1e-6);
        assert!((Ease::Out(EaseFamily::Quad).along(0.5) - 0.75).abs() < 1e-6);
        assert!((Ease::In(EaseFamily::Cubic).along(0.5) - 0.125).abs() < 1e-6);
        assert!(Ease::In(EaseFamily::Back).along(0.2) < 0.0, "back in goes back first");
        assert!(Ease::Out(EaseFamily::Back).along(0.8) > 1.0, "back out goes past");
        let springs = (1..20).any(|step| Ease::Out(EaseFamily::Elastic).along(step as f32 / 20.0) > 1.0);
        assert!(springs, "elastic goes past its end");
        let bounce: Vec<f32> = (0..=40).map(|step| Ease::Out(EaseFamily::Bounce).along(step as f32 / 40.0)).collect();
        assert!(bounce.iter().all(|at| (-1e-4..=1.0 + 1e-4).contains(at)) && bounce.windows(2).any(|pair| pair[1] < pair[0]), "it bounces back down");
        assert_eq!([0.1, 0.3, 0.6, 0.99].map(|t| Ease::Steps(4).along(t)), [0.0, 0.25, 0.5, 0.75]);
        assert!((Ease::Bezier([0.0, 0.0, 1.0, 1.0]).along(0.3) - 0.3).abs() < 1e-3, "a straight bezier is even");
        let css_ease = Ease::Bezier([0.25, 0.1, 0.25, 1.0]);
        assert!((css_ease.along(0.5) - 0.8024).abs() < 0.01, "CSS's ease at half: {}", css_ease.along(0.5));
    }

    #[test]
    fn an_ease_written_is_read_back() {
        for ease in every() {
            assert_eq!(Ease::read(&[ease.to_string()]), Some((ease, 1)), "{ease}");
        }
        let words = |text: &str| text.split_whitespace().map(str::to_string).collect::<Vec<String>>();
        assert_eq!(Ease::read(&words("out_bounce")), Some((Ease::Out(EaseFamily::Bounce), 1)));
        assert_eq!(Ease::read(&words("inout_sine")), Some((Ease::InOut(EaseFamily::Sine), 1)));
        assert_eq!(Ease::read(&words("in 2s")), Some((Ease::In(EaseFamily::Cubic), 1)));
        assert_eq!(Ease::read(&words("elastic")), Some((Ease::Out(EaseFamily::Elastic), 1)));
        assert_eq!(Ease::read(&words("out")), Some((Ease::Out(EaseFamily::Quad), 1)));
        assert_eq!(Ease::read(&words("steps 3")), Some((Ease::Steps(3), 2)));
        assert_eq!(Ease::read(&words("bezier 0.4 0 0.2 1")), Some((Ease::Bezier([0.4, 0.0, 0.2, 1.0]), 5)));
        for wrong in ["soft", "out_soft", "steps 0", "steps(x)", "bezier 1 2", "steps 1.5"] {
            assert_eq!(Ease::read(&words(wrong)), None, "{wrong}");
        }
        for name in Ease::names() {
            assert!(Ease::read(std::slice::from_ref(&name)).is_some(), "{name}");
        }
    }
}
