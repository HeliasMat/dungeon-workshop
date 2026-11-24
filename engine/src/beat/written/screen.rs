//! How a `ScreenBeat` is written as its line (read by `beat::screen`).

use crate::base::colour_named;
use crate::beat::names::{light_named, weather_named};
use crate::beat::{Cam, PARTICLES, ParticleKind, ScreenBeat, Weather};
use crate::vocabulary::Vocabulary;

/// The lights a scene has by a name (`beat::names::light_named`).
const LIGHTS: [&str; 8] = ["normal", "dawn", "evening", "night", "cave", "storm", "dream", "fire"];

/// The colours' names the files may write.
const COLOURS: [&str; 15] =
    ["white", "red", "yellow", "blue", "green", "black", "pink", "purple", "orange", "cyan", "gold", "grey", "brown", "sand", "turquoise"];

/// The weathers' names.
const WEATHERS: [&str; 10] = ["none", "rain", "snow", "petals", "sparkles", "leaves", "hail", "sand", "mist", "sun"];

/// The name of a colour of bytes, if the files have one for it.
fn colour_name(colour: [u8; 3]) -> Option<String> {
    COLOURS.iter().find(|name| colour_named(name).map(|named| named.map(|part| part as u8)) == Some(colour)).map(|name| name.to_string())
}

/// The name of a kind of particles.
fn particle_name(kind: ParticleKind) -> Option<String> {
    PARTICLES.iter().find(|(_, named)| *named == kind).map(|(name, _)| name.to_string())
}

fn weather_name(weather: Weather) -> Option<String> {
    WEATHERS.iter().find(|name| weather_named(name) == Some(weather)).map(|name| name.to_string())
}

/// Frames as a whole number (those read as a time are whole).
fn whole(frames: f32) -> Option<String> {
    (frames >= 0.0 && frames.fract() == 0.0 && frames <= u32::MAX as f32).then(|| frames.to_string())
}

fn cam(on: &Cam) -> Vec<String> {
    match on {
        Cam::At(at) => vec!["@".to_string(), at.x.to_string(), at.y.to_string()],
        Cam::Who(who) => vec![who.to_string()],
    }
}

impl ScreenBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let mut line = vec![];
        let word = match self {
            ScreenBeat::Crossfade { level, frames } => {
                line.extend([level.to_string(), whole(frames.0)?]);
                "crossfade"
            }
            // (By the word that says it all: `fadein 30`, `whiteout 20`.)
            ScreenBeat::Fade { out, white, frames } => {
                line.push(frames.to_string());
                match (out, white) {
                    (false, false) => "fadein",
                    (true, false) => "fadeout",
                    (true, true) => "whiteout",
                    (false, true) => "whitein",
                }
            }
            // (A light by its name, `light night 60`; any other by its numbers.)
            ScreenBeat::Tint { colour, frames } => match LIGHTS.iter().find(|name| light_named(name).as_ref() == Some(colour)) {
                Some(name) => {
                    line.extend([name.to_string(), frames.to_string()]);
                    "light"
                }
                None => {
                    line.extend(colour.iter().map(f32::to_string));
                    line.push(frames.to_string());
                    "tint"
                }
            },
            ScreenBeat::Flash { colour, frames } => {
                line.extend(colour.iter().map(f32::to_string));
                line.push(frames.to_string());
                "flash"
            }
            ScreenBeat::Weather(weather) => {
                line.push(weather_name(*weather)?);
                "weather"
            }
            ScreenBeat::Halo(None) => {
                line.push("off".to_string());
                "halo"
            }
            ScreenBeat::Halo(Some(halo)) => {
                line.extend(cam(&halo.on));
                line.push(halo.radius.to_string());
                // (Its own colour left unsaid: it has no name.)
                if halo.colour != [255, 200, 120] {
                    line.push(colour_name(halo.colour)?);
                }
                "halo"
            }
            ScreenBeat::Particles { kind, on, count, colour } => {
                line.push(particle_name(*kind)?);
                line.extend(cam(on));
                line.push(count.to_string());
                if let Some(colour) = colour {
                    line.push(colour_name(*colour)?);
                }
                "particles"
            }
            ScreenBeat::Emitter { kind, who, each_second, colour, frames } => {
                // (Fewer than that a second are read as that.)
                if *each_second < 0.1 {
                    return None;
                }
                line.extend([particle_name(*kind)?, who.to_string(), each_second.to_string()]);
                match (colour, frames) {
                    (Some(colour), _) => line.push(colour_name(*colour)?),
                    // (A word naming no colour, to reach the time.)
                    (None, Some(_)) => line.push("-".to_string()),
                    (None, None) => {}
                }
                if let Some(frames) = frames {
                    line.push(whole(frames.0)?);
                }
                "emit"
            }
            ScreenBeat::EmitterOff(who) => {
                line.extend(["off".to_string(), who.to_string()]);
                "emit"
            }
            ScreenBeat::Wave { to, frames } => {
                if !(0.0..=1.0).contains(to) {
                    return None;
                }
                line.extend([to.to_string(), whole(frames.0)?]);
                "wave"
            }
            ScreenBeat::Cutaway { level, x, y, over, frames } => {
                if *over {
                    line.push("over".to_string());
                }
                line.extend([level.to_string(), x.to_string(), y.to_string(), whole(frames.0)?]);
                "cutaway"
            }
            ScreenBeat::CutawayPan { x, y, frames } => {
                line.extend(["pan".to_string(), x.to_string(), y.to_string(), whole(frames.0)?]);
                "cutaway"
            }
            ScreenBeat::CutawayOff => {
                line.push("off".to_string());
                "cutaway"
            }
            ScreenBeat::Minimap { team, discover } => {
                match team {
                    // (Gone yet found as one goes: no line says both.)
                    None if *discover => return None,
                    None => line.push("off".to_string()),
                    Some(team) => {
                        if *discover {
                            line.push("discover".to_string());
                        }
                        line.extend(team.iter().map(|who| who.to_string()));
                    }
                }
                "minimap"
            }
            ScreenBeat::Lightning(on) => {
                line.extend(cam(on));
                "lightning"
            }
            ScreenBeat::Picture { name, file, at } => {
                line.push(name.clone());
                match file {
                    // (Taken away: read at no point.)
                    None if *at != crate::beat::Point::at(glam::Vec2::ZERO) => return None,
                    None => line.push("off".to_string()),
                    Some(file) if file == "off" => return None,
                    Some(file) => line.extend([file.clone(), at.x.to_string(), at.y.to_string()]),
                }
                "image"
            }
            ScreenBeat::Wipe { out, side, frames } => {
                let side = ["left", "right", "top", "bottom"].get(*side as usize)?;
                line.extend([if *out { "out" } else { "in" }.to_string(), side.to_string(), whole(frames.0)?]);
                "wipe"
            }
            ScreenBeat::Credits { frames, lines } => {
                // (Read split at `\n`, the first trimmed: other lines cannot be read back.)
                if lines.is_empty() || lines.iter().any(|text| text.contains("\\n")) || lines[0].trim_start() != lines[0] {
                    return None;
                }
                line.extend([whole(frames.0)?, lines.join("\\n")]);
                "credits"
            }
            ScreenBeat::Caption { frames, text, typed } => {
                // (A `\n` written in the words would be read as a new line.)
                if text.contains("\\n") {
                    return None;
                }
                line.extend([frames.to_string(), text.replace('\n', "\\n")]);
                if *typed { "typecaption" } else { "caption" }
            }
            _ => return None,
        };
        line.insert(0, word.to_string());
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frames;
    use crate::beat::Beat;
    use crate::beat::Halo;
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;
    use glam::Vec2;

    #[test]
    fn every_screen_beat_written_is_read_back_the_same() {
        let beats = [
            ScreenBeat::Backdrop(12, true, Some(Who::Monster(25))),
            ScreenBeat::Backdrop(3, false, None),
            ScreenBeat::Crossfade { level: 4, frames: Frames(60.0) },
            ScreenBeat::Banner("Foggy Woods".to_string()),
            ScreenBeat::Fade { out: true, white: false, frames: Frames(30.5) },
            ScreenBeat::Dim { white: true, frames: Frames(20.0), to: 128.0 },
            ScreenBeat::Tint { colour: [20.0, 30.0, 90.0, 120.0], frames: Frames(60.0) },
            ScreenBeat::Flash { colour: [255.0, 40.0, 40.0], frames: Frames(20.0) },
            ScreenBeat::Weather(Weather::Rain),
            ScreenBeat::Weather(Weather::Clear),
            ScreenBeat::Halo(None),
            ScreenBeat::Halo(Some(Halo { on: Cam::Who(Who::Character(0)), radius: 24.0, colour: [255, 200, 120] })),
            ScreenBeat::Halo(Some(Halo { on: Cam::at(Vec2::new(10.0, -4.5)), radius: 0.0, colour: [80, 140, 255] })),
            ScreenBeat::Particles { kind: ParticleKind::Smoke, on: Cam::at(Vec2::new(1.0, 2.0)), count: 40, colour: None },
            ScreenBeat::Particles { kind: ParticleKind::Sparks, on: Cam::Who(Who::Target), count: 7, colour: Some([255, 230, 80]) },
            ScreenBeat::Emitter { kind: ParticleKind::Embers, who: Who::Monster(4), each_second: 8.0, colour: None, frames: Some(Frames(90.0)) },
            ScreenBeat::Emitter { kind: ParticleKind::Drops, who: Who::Thing { object: 66, copy: 0 }, each_second: 2.5, colour: Some([0, 0, 0]), frames: None },
            ScreenBeat::EmitterOff(Who::Monster(4)),
            ScreenBeat::Wave { to: 0.5, frames: Frames(30.0) },
            ScreenBeat::BattleLog { frames: Frames(90.0), text: "Pikachu attacks!".to_string() },
            ScreenBeat::Cutaway { level: 7, x: 1.5, y: 2.0, over: true, frames: Frames(0.0) },
            ScreenBeat::Cutaway { level: 7, x: 1.5, y: 2.0, over: false, frames: Frames(30.0) },
            ScreenBeat::CutawayPan { x: 3.0, y: 4.0, frames: Frames(40.0) },
            ScreenBeat::CutawayOff,
            ScreenBeat::Minimap { team: None, discover: false },
            ScreenBeat::Minimap { team: Some(vec![Who::Character(0), Who::Character(10)]), discover: true },
            ScreenBeat::Grey(true),
            ScreenBeat::Still(false),
            ScreenBeat::Lightning(Cam::at(Vec2::new(5.0, 6.0))),
            ScreenBeat::Lightning(Cam::Who(Who::Monster(25))),
            ScreenBeat::Picture { name: "map".to_string(), file: Some("map.png".to_string()), at: Vec2::new(8.0, 9.0).into() },
            ScreenBeat::Picture { name: "map".to_string(), file: None, at: Vec2::ZERO.into() },
            ScreenBeat::Wipe { out: true, side: 2, frames: Frames(30.0) },
            ScreenBeat::Credits { frames: Frames(600.0), lines: vec!["The End".to_string(), String::new(), "Thanks".to_string()] },
            ScreenBeat::Caption { frames: Frames(120.0), text: "The next day...\nat the harbour".to_string(), typed: true },
            ScreenBeat::Caption { frames: Frames(60.0), text: String::new(), typed: false },
        ];
        for beat in beats {
            let line = Beat::Screen(beat.clone()).fields(&SAMPLE).expect("written");
            assert_eq!(Beat::parse(&line, &SAMPLE).ok(), Some(Beat::Screen(beat)), "{line:?}");
        }
    }
}
