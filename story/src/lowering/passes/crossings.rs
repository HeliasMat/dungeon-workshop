//! Walking from one place to the next, written out.

use super::Lines;
use engine::Diagnostic;

/// Going from a place to the next walking, at one go (`crossto <side>
/// <level> <x> <y> <who...>`, its places' names put back already): they
/// walk off by that side as black comes over from it; the next place
/// shows; they come in from the other side, black going back that way,
/// and walk to the point, one beside the other.
pub(super) fn crossings(lines: Lines, problems: &mut Vec<Diagnostic>) -> Lines {
    let mut out = Vec::with_capacity(lines.len());
    for (number, fields) in lines {
        if fields[0] != "crossto" {
            out.push((number, fields));
            continue;
        }
        let way = match fields.get(1).map(String::as_str) {
            Some("right") => Some(((1.0_f32, 0.0_f32), "right", "left")),
            Some("left") => Some(((-1.0, 0.0), "left", "right")),
            Some("up") => Some(((0.0, -1.0), "up", "down")),
            Some("down") => Some(((0.0, 1.0), "down", "up")),
            _ => None,
        };
        let real = |at: usize| fields.get(at).and_then(|field| field.parse::<f32>().ok());
        let (Some(((dx, dy), side, other)), Some(level), Some(x), Some(y)) =
            (way, fields.get(2).filter(|level| level.parse::<u16>().is_ok()), real(3), real(4))
        else {
            problems.push(Diagnostic::at(number, "“crossto” takes <right|left|up|down> <place> <x> <y> <who...>".to_string()));
            continue;
        };
        let who = &fields[5..];
        if who.is_empty() {
            problems.push(Diagnostic::at(number, "“crossto” takes who crosses".to_string()));
            continue;
        }
        let line = |words: &[String]| (number, words.to_vec());
        let text = |value: f32| format!("{}", value.round());
        for one in who {
            out.push(line(&["step".into(), one.clone(), text(dx * 180.0), text(dy * 180.0)]));
        }
        out.push(line(&["wipe".into(), "out".into(), side.into(), "40".into()]));
        out.push(line(&["waitfade".into()]));
        out.push(line(&["backdrop".into(), level.clone(), "0".into()]));
        // (Who stood in the place left stays there.)
        out.push(line(&[vec!["keeponly".to_string()], who.to_vec()].concat()));
        // (Each a step behind the one before, the way they go; side by side
        // a little, so as not to stand on each other.)
        for (place, one) in who.iter().enumerate() {
            let behind = place as f32 * 22.0;
            let aside = if place % 2 == 1 { 10.0 } else { 0.0 };
            let (to_x, to_y) = (x - dx * behind + dy.abs() * aside, y - dy * behind + dx.abs() * aside);
            // (From a little before, off the place's edge at most so far.)
            let (from_x, from_y) = ((to_x - dx * 60.0).max(-40.0), (to_y - dy * 60.0).max(-40.0));
            out.push(line(&["put".into(), one.clone(), text(from_x), text(from_y), side.into()]));
            out.push(line(&["walk".into(), one.clone(), text(to_x), text(to_y)]));
        }
        out.push(line(&["wipe".into(), "in".into(), other.into(), "40".into()]));
        out.extend(who.iter().map(|one| line(&["waitfor".into(), one.clone()])));
    }
    out
}
