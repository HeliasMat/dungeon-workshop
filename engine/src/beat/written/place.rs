//! How a `PlaceBeat` is written as its line (read by `beat::place`).

use glam::Vec2;

use crate::beat::{Exit, Formation, Gait, Pace, PlaceBeat, Spot};
use crate::vocabulary::Vocabulary;
use crate::who::Who;

/// A spot as a field: its letter, then what it says, with `:` between.
fn spot(spot: &Spot) -> String {
    let point = |at: &crate::beat::Point| format!("{}:{}", at.x, at.y);
    match spot {
        Spot::Written(at) => format!("w:{}", point(at)),
        Spot::Word(at) => format!("m:{}", point(at)),
        Spot::Exact(at) => format!("x:{}", point(at)),
        Spot::Near(at) => format!("t:{}", point(at)),
        Spot::Named(name) => format!("n:{name}"),
        Spot::Of(who) => format!("o:{who}"),
        Spot::Before { of, back } => format!("{}:{of}", if *back { "b" } else { "d" }),
        Spot::Side { of, right } => format!("{}:{of}", if *right { "r" } else { "g" }),
        Spot::Between(a, b) => format!("e:{a}:{b}"),
        Spot::Tiles { of, count } => format!("c:{count}:{of}"),
        Spot::Beside(of) => format!("p:{of}"),
        Spot::By { of, dx, dy } => format!("y:{of}:{dx}:{dy}"),
        Spot::Shape { kind, who, at, round } => {
            let kind = match kind {
                Formation::Line => "row",
                Formation::Column => "column",
                Formation::Triangle => "triangle",
                Formation::Facing => "face",
                Formation::Ring => "ring",
            };
            let who: Vec<String> = who.iter().map(Who::to_string).collect();
            format!("f:{kind}:{at}:{}:{}", who.join(","), self::spot(round))
        }
    }
}

/// A gait as its two fields: `run` or `walk`, then the pace.
fn gait(gait: &Gait) -> [String; 2] {
    let pace = match gait.pace {
        Pace::Usual => "-".to_string(),
        Pace::Times(much) => format!("x{much}"),
        Pace::Seconds(seconds) => format!("s{seconds}"),
    };
    [if gait.run { "run" } else { "walk" }.to_string(), pace]
}

/// A way out as a field (a name that reads as a side cannot be).
fn exit(exit: &Exit) -> Option<String> {
    match exit {
        Exit::Side { right } => Some(if *right { "right" } else { "left" }.to_string()),
        Exit::Named(name) if matches!(name.as_str(), "right" | "left") => None,
        Exit::Named(name) => Some(name.clone()),
    }
}

fn point(at: &Vec2) -> [String; 2] {
    [at.x.to_string(), at.y.to_string()]
}

fn group(who: &[Who]) -> impl Iterator<Item = String> + '_ {
    who.iter().map(Who::to_string)
}

impl PlaceBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        let word = |word: &str| word.to_string();
        let mut line = vec![];
        match self {
            PlaceBeat::Ground { middle, ground } => {
                line.push(word("ground"));
                line.extend(point(middle));
                line.push(match ground {
                    None => word("-"),
                    Some(ground) => {
                        let rows: Vec<String> = ground.rows().iter().map(|row| row.iter().map(|solid| if *solid { '1' } else { '0' }).collect()).collect();
                        rows.join("/")
                    }
                });
            }
            PlaceBeat::Flies(who) => {
                line.push(word("flies"));
                line.extend(group(who));
            }
            PlaceBeat::Name { name, spot: at } => line.extend([word("setspot"), format!("n:{name}"), spot(at)]),
            PlaceBeat::Find { var, who, spot: at, apart, exact } => {
                // (No one: a field that is no one, for the view.)
                line.extend([word("find"), var.clone(), who.map_or(word("-"), |who| who.to_string()), spot(at)]);
                line.extend(apart.then(|| word("apart")));
                line.extend(exact.then(|| word("exact")));
            }
            PlaceBeat::Spread { who, picture } => {
                line.extend([word("spread"), word(if *picture { "image" } else { "-" })]);
                line.extend(group(who));
            }
            PlaceBeat::Goes { who, points, gait: how } => {
                line.extend([word("goes"), who.to_string()]);
                line.extend(gait(how));
                line.extend(points.iter().flat_map(crate::beat::Point::fields));
            }
            PlaceBeat::Arrives { who, to, gait: how, placing, from } => {
                line.extend([word("arrives"), who.to_string()]);
                line.extend(gait(how));
                line.extend(to.fields());
                line.push(word(if *placing { "place" } else { "-" }));
                if let Some(from) = from {
                    line.push(format!("via:{}", exit(from)?));
                }
            }
            PlaceBeat::Leaves { who, by, run } => {
                line.extend([word("leaves"), who.to_string(), exit(by)?]);
                line.extend(run.then(|| word("run")));
            }
            PlaceBeat::Entrances(ways) => {
                line.push(word("entrances"));
                for (name, at) in ways {
                    line.push(name.clone());
                    line.extend(point(at));
                }
            }
            PlaceBeat::Round { who, spot: at } => line.extend([word("roundcheck"), who.to_string(), spot(at)]),
            // (One field says both: a way by its name, or someone.)
            PlaceBeat::AllLook { way, at } => line.extend([
                word("alllook"),
                match (way, at) {
                    (Some(way), None) => word(way.name()),
                    (None, Some(who)) => who.to_string(),
                    (None, None) => word("-"),
                    // (Both: never read.)
                    _ => return None,
                },
            ]),
            PlaceBeat::Crowd { who, spot: at, walking, form } => {
                line.extend([word("crowd"), word(if *walking { "walk" } else { "-" })]);
                line.push(at.as_ref().map_or(word("-"), spot));
                line.extend(form.fields());
                line.extend(group(who));
            }
            PlaceBeat::Gathers { var, spot: at, step, who } => {
                line.extend([word("gathers"), var.clone(), spot(at)]);
                line.extend(point(step));
                line.extend(who.iter().map(|who| who.map_or(word("-"), |who| who.to_string())));
            }
            PlaceBeat::Wave(who) => {
                line.push(word("wavethrough"));
                line.extend(group(who));
            }
            PlaceBeat::Advance { who, by } => {
                line.push(word("advance"));
                line.extend(point(by));
                line.extend(group(who));
            }
            PlaceBeat::Flock { who, to, run } => {
                line.extend([word("flock"), spot(to), word(if *run { "run" } else { "walk" })]);
                line.extend(group(who));
            }
            PlaceBeat::Seats { var, who, seats } => {
                line.extend([word("seats"), var.clone()]);
                line.extend(group(who));
                line.push(word(";"));
                line.extend(seats.iter().flat_map(point));
            }
            _ => return None,
        }
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::beat::{Beat, CrowdForm, CrowdShape, Direction, Exit, Formation, Gait, Pace, PlaceBeat, Spot};
    use crate::stage::Ground;
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;

    #[test]
    fn every_place_beat_written_is_read_back_the_same() {
        let (a, b) = (Who::Monster(25), Who::Character(0));
        let at = Vec2::new(120.5, -8.0);
        let spots = [
            Spot::Written(at.into()),
            Spot::Word(at.into()),
            Spot::Exact(at.into()),
            Spot::Near(Vec2::new(3.0, 4.0).into()),
            Spot::Named("well".into()),
            Spot::Of(a),
            Spot::Before { of: a, back: true },
            Spot::Before { of: a, back: false },
            Spot::Side { of: a, right: true },
            Spot::Side { of: a, right: false },
            Spot::Between(a, b),
            Spot::Tiles { of: a, count: 2.5 },
            Spot::Beside(b),
            Spot::By { of: a, dx: -16.0, dy: 8.5 },
            Spot::Shape { kind: Formation::Facing, who: vec![a, b], at: 1, round: Box::new(Spot::Of(b)) },
            Spot::Shape { kind: Formation::Line, who: vec![a], at: 0, round: Box::new(Spot::Word(at.into())) },
        ];
        let run = Gait { run: true, pace: Pace::Times(1.5) };
        let mut beats: Vec<PlaceBeat> = spots.into_iter().map(|spot| PlaceBeat::Name { name: "here".into(), spot }).collect();
        beats.extend([
            PlaceBeat::Ground { middle: at, ground: None },
            PlaceBeat::Ground { middle: at, ground: Some(Ground::new(vec![vec![true, false], vec![false, true]])) },
            PlaceBeat::Flies(vec![a, b]),
            PlaceBeat::Find { var: "v".into(), who: Some(a), spot: Spot::Of(b), apart: true, exact: false },
            PlaceBeat::Find { var: "v".into(), who: None, spot: Spot::Word(at.into()), apart: false, exact: true },
            PlaceBeat::Spread { who: vec![a, b], picture: true },
            PlaceBeat::Spread { who: vec![a], picture: false },
            PlaceBeat::Goes { who: a, points: vec![at.into(), Vec2::new(1.0, 2.0).into()], gait: Gait::WALK },
            PlaceBeat::Arrives { who: a, to: at.into(), gait: run, placing: true, from: Some(Exit::Named("door".into())) },
            PlaceBeat::Arrives { who: a, to: at.into(), gait: Gait { run: false, pace: Pace::Seconds(2.0) }, placing: false, from: None },
            PlaceBeat::Leaves { who: a, by: Exit::Side { right: false }, run: true },
            PlaceBeat::Leaves { who: a, by: Exit::Named("door".into()), run: false },
            PlaceBeat::Entrances(vec![("door".into(), at), ("north".into(), Vec2::new(5.0, 6.0))]),
            PlaceBeat::Round { who: a, spot: Spot::Of(b) },
            PlaceBeat::AllLook { way: Some(Direction::Right), at: None },
            PlaceBeat::AllLook { way: None, at: Some(a) },
            PlaceBeat::Crowd { who: vec![a, b], spot: Some(Spot::Exact(at.into())), walking: true, form: CrowdForm::default() },
            PlaceBeat::Crowd { who: vec![a], spot: None, walking: false, form: CrowdForm::default() },
            PlaceBeat::Crowd {
                who: vec![a, b],
                spot: Some(Spot::Named("flock".into())),
                walking: true,
                form: CrowdForm { shape: CrowdShape::Flock, wide: Some(40.0), tight: true, from: Some(Direction::Right), run: true },
            },
            PlaceBeat::Crowd { who: vec![b], spot: Some(Spot::Of(a)), walking: false, form: CrowdForm { shape: CrowdShape::Line, ..CrowdForm::default() } },
            PlaceBeat::Flock { who: vec![a, b], to: Spot::Written(at.into()), run: true },
            PlaceBeat::Flock { who: vec![a], to: Spot::Named("throne".into()), run: false },
            PlaceBeat::Gathers { var: "g".into(), spot: Spot::Of(a), step: Vec2::new(16.0, 0.0), who: vec![Some(a), None, Some(b)] },
            PlaceBeat::Wave(vec![a, b]),
            PlaceBeat::Advance { who: vec![a], by: Vec2::new(-20.0, 0.0) },
            PlaceBeat::Seats { var: "table".into(), who: vec![a, b], seats: vec![at, Vec2::new(1.0, 2.0)] },
            PlaceBeat::Shot { close: 1.5, who: Some(a) },
            PlaceBeat::Shot { close: 2.0, who: None },
            PlaceBeat::Roams { who: a, wide: 48.0 },
        ]);
        for beat in beats {
            let fields = Beat::Place(beat.clone()).fields(&SAMPLE).expect("a line");
            assert_eq!(Beat::parse(&fields, &SAMPLE).ok(), Some(Beat::Place(beat)), "{fields:?}");
        }
    }
}
