//! A moment of a film as it is asked for: `32s`, `line 90`, `scene 3 end`.

use preview::played::{SECOND, Timeline, seconds};

/// The frames after a line starts that its picture is taken: it has begun.
const AFTER_LINE: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Moment {
    Second(f32),
    Line(i32),
    /// A scene (from 1), its start or its end.
    Scene(usize, bool),
}

const SAY: &str = "32s, line 90, scene 3, scene 3 end";

/// The moments of a command's words.
pub fn read(words: &[String]) -> Result<Vec<Moment>, String> {
    let mut moments = Vec::new();
    let mut rest = words;
    while let Some(word) = rest.first() {
        let number = |at: usize| rest.get(at).and_then(|word| word.parse::<usize>().ok()).ok_or(format!("what number after “{word}”? {SAY}"));
        match word.as_str() {
            "line" => {
                moments.push(Moment::Line(number(1)? as i32));
                rest = &rest[2..];
            }
            "scene" => {
                let scene = number(1)?;
                let end = matches!(rest.get(2).map(String::as_str), Some("end"));
                let told = end || matches!(rest.get(2).map(String::as_str), Some("start"));
                moments.push(Moment::Scene(scene, end));
                rest = &rest[2 + usize::from(told)..];
            }
            _ => {
                let second = word.strip_suffix('s').unwrap_or(word).parse().map_err(|_| format!("what moment? “{word}”: {SAY}"))?;
                moments.push(Moment::Second(second));
                rest = &rest[1..];
            }
        }
    }
    match moments.is_empty() {
        true => Err(format!("what moment? {SAY}")),
        false => Ok(moments),
    }
}

impl Moment {
    /// How it shows in a file's name.
    pub fn name(&self) -> String {
        match self {
            Moment::Second(second) => format!("{second}s"),
            Moment::Line(line) => format!("line_{line}"),
            Moment::Scene(scene, false) => format!("scene_{scene}"),
            Moment::Scene(scene, true) => format!("scene_{scene}_end"),
        }
    }

    /// Its frame in a film.
    pub fn frame(&self, timeline: &Timeline) -> Result<usize, String> {
        let frame = match *self {
            Moment::Second(second) => ((second * SECOND as f32) as usize).max(1),
            Moment::Line(line) => timeline.line(line).ok_or(format!("line {line} does not play (scene_player timing <film>)"))?.1 + AFTER_LINE,
            Moment::Scene(scene, end) => {
                let span = scene.checked_sub(1).and_then(|at| timeline.scenes.get(at)).ok_or(format!("no scene {scene}: {} in all", timeline.scenes.len()))?;
                if end { span.seen_to } else { span.seen_from.unwrap_or(span.start) }
            }
        };
        match frame <= timeline.frames {
            true => Ok(frame),
            false => Err(format!("{}: the film ends at {} s", self.name(), seconds(timeline.frames))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use preview::played::Span;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn moments_are_read_one_after_another() {
        let said = read(&words("32s line 90 scene 3 end scene 2 40")).expect("moments");
        assert_eq!(said, [Moment::Second(32.0), Moment::Line(90), Moment::Scene(3, true), Moment::Scene(2, false), Moment::Second(40.0)]);
        assert!(read(&words("noon")).is_err() && read(&words("line")).is_err() && read(&[]).is_err());
    }

    #[test]
    fn a_moment_is_found_on_the_timeline() {
        let scene = Span { start: 100, seen_from: Some(120), seen_to: 500 };
        let timeline = Timeline { frames: 600, scenes: vec![scene], lines: [(7, 200)].into(), ..Timeline::default() };
        assert_eq!(Moment::Second(2.0).frame(&timeline), Ok(120));
        assert_eq!(Moment::Line(7).frame(&timeline), Ok(220));
        assert_eq!(Moment::Scene(1, false).frame(&timeline), Ok(120));
        assert_eq!(Moment::Scene(1, true).frame(&timeline), Ok(500));
        assert!(Moment::Scene(2, true).frame(&timeline).is_err() && Moment::Second(11.0).frame(&timeline).is_err());
    }
}
