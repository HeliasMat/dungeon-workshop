//! Who is sent outside the picture shown.

use super::*;

/// Where a file puts or sends someone outside the backdrop shown then
/// (`put`, `walk`, `run`, `go`, `hop`, `path`): off its picture, unseen.
pub(super) fn outside_backdrops(reads: &[Read], sizes: &HashMap<u16, (f32, f32)>) -> Vec<Diagnostic> {
    let (mut size, mut problems) = (None, Vec::new());
    for read in reads {
        let Is::Beat(beat) = &read.is else {
            continue;
        };
        let points = match beat.clone() {
            Beat::Screen(ScreenBeat::Backdrop(level, ..)) => {
                size = sizes.get(&level).copied();
                continue;
            }
            Beat::Body(BodyBeat::Put { at, .. })
            | Beat::Motion(MotionBeat::Walk { at, .. })
            | Beat::Motion(MotionBeat::Go { at, .. })
            | Beat::Body(BodyBeat::Hop { at, .. }) => vec![at.vec2()],
            Beat::Motion(MotionBeat::Path { points, .. }) => points,
            Beat::Motion(MotionBeat::WalkTo { at, .. }) => vec![at.vec2()],
            // (Each in its place in the line, or round the point.)
            Beat::Motion(MotionBeat::LineUp { who, at, gap }) => (0..who.len()).map(|place| at + gap * place as f32).collect(),
            Beat::Motion(MotionBeat::Ring { who, at, radius }) => {
                let count = who.len().max(1) as f32;
                (0..who.len()).map(|place| at + Vec2::from_angle(std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * place as f32 / count) * radius).collect()
            }
            _ => continue,
        };
        let Some((wide, high)) = size else {
            continue;
        };
        // (Coming in or going off by an edge, a little past it, is meant.)
        const PAST: f32 = 64.0;
        for point in points.iter().filter(|point| point.x < -PAST || point.y < -PAST || point.x > wide + PAST || point.y > high + PAST) {
            problems.push(
                Diagnostic::at(read.number, format!("({}, {}) is off the backdrop ({wide} x {high}): it will not be seen there", point.x, point.y)).as_remark(),
            );
        }
    }
    problems
}
