//! How a `PlayBeat` is written as its line (read by `beat::play`).

use super::data::value;
use crate::base::Rect;
use crate::beat::{HUD_PLACES, PlayBeat, colour_named};
use crate::vocabulary::Vocabulary;

/// The colours a gauge may be written in, by the names `colour_named` reads.
const COLOURS: [&str; 15] =
    ["white", "red", "yellow", "blue", "green", "black", "pink", "orange", "purple", "cyan", "gold", "grey", "brown", "sand", "turquoise"];

fn corners(rect: &Rect) -> Vec<String> {
    [rect.left, rect.top, rect.right, rect.bottom].iter().map(f32::to_string).collect()
}

/// A place of words on screen by its first name.
fn place(at: usize) -> Option<String> {
    HUD_PLACES.iter().find(|(_, place)| *place == at).map(|(name, _)| name.to_string())
}

impl PlayBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let mut line = vec![];
        let word = |word: &str| word.to_string();
        match self {
            PlayBeat::Control(who) => line.extend([word("control"), who.map_or(word("off"), |who| who.to_string())]),
            PlayBeat::ControlTeam(team) if team.is_empty() => line.extend([word("swapcontrol"), word("off")]),
            PlayBeat::ControlTeam(team) => {
                line.push(word("swapcontrol"));
                line.extend(team.iter().map(|who| who.to_string()));
            }
            PlayBeat::Bounds(None) => line.extend([word("bounds"), word("off")]),
            PlayBeat::Bounds(Some(rect)) => {
                line.push(word("bounds"));
                line.extend(corners(rect));
            }
            PlayBeat::Wall(None) => line.extend([word("wall"), word("off")]),
            PlayBeat::Wall(Some(rect)) => {
                line.push(word("wall"));
                line.extend(corners(rect));
            }
            PlayBeat::NamedWall { name, wall } => {
                line.extend([word("wall"), name.clone()]);
                match wall {
                    None => line.push(word("off")),
                    Some(rect) => line.extend(corners(rect)),
                }
            }
            PlayBeat::Gauge { at, gauge: None } => line.extend([word("gauge"), place(*at)?, word("off")]),
            PlayBeat::Gauge { at, gauge: Some(gauge) } => {
                let colour = gauge.colour.map(f32::from);
                // (Only a colour the game names can be written.)
                let colour = COLOURS.iter().find(|name| colour_named(name) == Some(colour))?;
                line.extend([word("gauge"), place(*at)?, gauge.var.clone(), value(&gauge.most)?, word(colour)]);
            }
            PlayBeat::Hud { at, text } => {
                line.extend([word("hud"), place(*at)?]);
                line.push(match text {
                    None => word("off"),
                    // (A newline is written `\n`; words with `\n` in them already cannot be.)
                    Some(text) if text.contains("\\n") => return None,
                    Some(text) => text.replace('\n', "\\n"),
                });
            }
            _ => return None,
        }
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use crate::base::Rect;
    use crate::beat::{Beat, GaugeOf, PlayBeat, Value};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;

    #[test]
    fn every_play_beat_written_is_read_back_the_same() {
        let rect = || Rect::from([10.0, 20.5, 300.0, 200.0]);
        for beat in [
            PlayBeat::Control(Some(Who::Monster(25))),
            PlayBeat::Control(None),
            PlayBeat::ControlTeam(vec![Who::Monster(25), Who::Character(0)]),
            PlayBeat::ControlTeam(vec![]),
            PlayBeat::Listen(true),
            PlayBeat::Listen(false),
            PlayBeat::Bounds(Some(rect())),
            PlayBeat::Bounds(None),
            PlayBeat::Wall(Some(rect())),
            PlayBeat::Wall(None),
            PlayBeat::NamedWall { name: "door".into(), wall: Some(rect()) },
            PlayBeat::NamedWall { name: "door".into(), wall: None },
            PlayBeat::Collisions(true),
            PlayBeat::Collisions(false),
            PlayBeat::Gauge { at: 0, gauge: Some(GaugeOf { var: "life".into(), most: Value::Number(10.0), colour: [255, 40, 40] }) },
            PlayBeat::Gauge { at: 2, gauge: None },
            PlayBeat::Hud { at: 1, text: Some("Score {score}\nend".into()) },
            PlayBeat::Hud { at: 2, text: None },
        ] {
            let fields = Beat::Play(beat.clone()).fields(&SAMPLE).expect("a line");
            assert_eq!(Beat::parse(&fields, &SAMPLE).ok(), Some(Beat::Play(beat)), "{fields:?}");
        }
    }
}
