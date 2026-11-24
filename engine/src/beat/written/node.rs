//! A node's beat written back as its line.

use crate::beat::{NodeBeat, Sort, TrackMode};
use crate::vocabulary::Vocabulary;

impl NodeBeat {
    pub(super) fn fields(&self, _words: &Vocabulary) -> Option<Vec<String>> {
        let line = |word: &str, rest: Vec<String>| std::iter::once(word.to_string()).chain(rest).collect::<Vec<String>>();
        Some(match self {
            // (The value as the grammar reads it: a point in two fields.)
            NodeBeat::Set { node, prop, value } => line(
                "prop",
                [node.to_string(), prop.prop().name.to_string()].into_iter().chain(value.to_string().split_whitespace().map(str::to_string)).collect(),
            ),
            NodeBeat::Animate { node, prop, keys, mode, relative, exact } => {
                let mut rest = vec![node.to_string(), prop.prop().name.to_string()];
                if *mode != TrackMode::Once {
                    rest.push(mode.word().to_string());
                }
                rest.extend(relative.then(|| "relative".to_string()));
                rest.extend(exact.then(|| "exact".to_string()));
                for key in keys {
                    rest.extend(["/".to_string(), key.at.word()]);
                    rest.extend(key.value.to_string().split_whitespace().map(str::to_string));
                    rest.push(key.ease.to_string());
                }
                line("track", rest)
            }
            NodeBeat::Path { node, points, shape, frames, ease, mode, orient, relative } => {
                let mut rest = vec![node.to_string(), frames.0.to_string(), shape.word().to_string(), ease.to_string()];
                if *mode != TrackMode::Once {
                    rest.push(mode.word().to_string());
                }
                rest.extend(orient.then(|| "orient".to_string()));
                rest.extend(relative.then(|| "relative".to_string()));
                for at in points {
                    rest.extend(["/".to_string(), at.x.to_string(), at.y.to_string()]);
                }
                line("trackpath", rest)
            }
            NodeBeat::Stop { node, prop } => line("untrack", std::iter::once(node.to_string()).chain(prop.map(|prop| prop.prop().name.to_string())).collect()),
            NodeBeat::Make { name, sort: Sort::Group, at } => line("node", [name.clone(), "group".to_string()].into_iter().chain(at.fields()).collect()),
            NodeBeat::Make { name, sort: Sort::Picture(file), at } => {
                line("node", [name.clone(), "picture".to_string(), file.clone()].into_iter().chain(at.fields()).collect())
            }
            NodeBeat::Unmake(name) => line("node", vec![name.clone(), "off".to_string()]),
            NodeBeat::Attach { node, to, offset, inherit } => {
                let mut rest = vec![node.to_string(), to.to_string()];
                rest.extend(offset.iter().flat_map(|at| [at.x.to_string(), at.y.to_string()]));
                if *inherit != crate::base::Inherit::default() {
                    rest.push("inherit".to_string());
                    rest.extend(inherit.words().into_iter().map(str::to_string));
                }
                line("attach", rest)
            }
            NodeBeat::Detach(node) => line("detach", vec![node.to_string()]),
        })
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::base::Inherit;
    use crate::base::{Ease, EaseFamily};
    use crate::beat::{Beat, Frames, Key, KeyTime, NodeBeat, NodeRef, Num, PathShape, Point, Sort, TrackMode};
    use crate::props::{Given, PropId, Value};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;

    #[test]
    fn a_node_beat_written_is_read_back_the_same() {
        let pika = NodeRef::Who(Who::Monster(25));
        for beat in [
            NodeBeat::Set { node: pika.clone(), prop: PropId::Angle, value: Given::Is(Value::Number(45.0)) },
            NodeBeat::Set { node: pika.clone(), prop: PropId::Angle, value: Given::Plus(Value::Number(-30.0)) },
            NodeBeat::Set { node: pika.clone(), prop: PropId::Scale, value: Given::Times(1.5) },
            NodeBeat::Set { node: pika.clone(), prop: PropId::Tint, value: Given::Is(Value::Colour([255, 136, 0, 128])) },
            NodeBeat::Set { node: pika.clone(), prop: PropId::Position, value: Given::Is(Value::Point(Vec2::new(120.0, -8.5))) },
            NodeBeat::Set {
                node: pika.clone(),
                prop: PropId::Position,
                value: Given::At { node: NodeRef::Named("tree".to_string()), by: Vec2::new(10.0, -4.0) },
            },
            NodeBeat::Set { node: NodeRef::Named("tree".to_string()), prop: PropId::Visible, value: Given::Is(Value::Bool(false)) },
            NodeBeat::Set { node: NodeRef::Caption, prop: PropId::Opacity, value: Given::Default },
            NodeBeat::Set { node: NodeRef::Halo(1), prop: PropId::Radius, value: Given::Is(Value::Number(60.0)) },
            NodeBeat::Set { node: NodeRef::Emitter(Who::Thing { object: 66, copy: 9 }), prop: PropId::Rate, value: Given::Is(Value::Number(12.0)) },
            NodeBeat::Set { node: pika.clone(), prop: PropId::Fill, value: Given::Is(Value::Nothing) },
            NodeBeat::Make { name: "group1".to_string(), sort: Sort::Group, at: Point::at(Vec2::new(100.0, 50.0)) },
            NodeBeat::Make {
                name: "moon".to_string(),
                sort: Sort::Picture("moon.png".to_string()),
                at: Point { x: Num::Var("x".to_string()), y: Num::Lit(3.0) },
            },
            NodeBeat::Unmake("moon".to_string()),
            NodeBeat::Make { name: "robot.bras".to_string(), sort: Sort::Group, at: Point::at(Vec2::ZERO) },
            NodeBeat::Attach { node: pika.clone(), to: NodeRef::Named("raft".to_string()), offset: None, inherit: Inherit::default() },
            NodeBeat::Attach {
                node: NodeRef::Named("robot.bras".to_string()),
                to: NodeRef::Named("robot".to_string()),
                offset: Some(Vec2::new(4.0, -12.5)),
                inherit: Inherit { scale: false, opacity: false, colour: false, ..Inherit::default() },
            },
            NodeBeat::Attach { node: NodeRef::Halo(0), to: pika.clone(), offset: Some(Vec2::ZERO), inherit: Inherit::NONE },
            NodeBeat::Detach(NodeRef::Named("robot.bras".to_string())),
            NodeBeat::Animate {
                node: pika.clone(),
                prop: PropId::Angle,
                keys: vec![
                    Key { at: KeyTime::After(Frames(0.0)), value: Given::Is(Value::Number(0.0)), ease: Ease::Linear },
                    Key { at: KeyTime::After(Frames(60.0)), value: Given::Plus(Value::Number(90.0)), ease: Ease::Out(EaseFamily::Back) },
                    Key { at: KeyTime::Music(Frames(720.0)), value: Given::Is(Value::Number(-45.0)), ease: Ease::Bezier([0.4, 0.0, 0.2, 1.0]) },
                    Key { at: KeyTime::Mark { name: "blow".to_string(), plus: Frames(30.0) }, value: Given::Default, ease: Ease::Steps(3) },
                ],
                mode: TrackMode::PingPong,
                relative: true,
                exact: true,
            },
            NodeBeat::Animate {
                node: NodeRef::Named("moon".to_string()),
                prop: PropId::Position,
                keys: vec![
                    Key { at: KeyTime::After(Frames(30.0)), value: Given::Is(Value::Point(Vec2::new(10.0, -4.5))), ease: Ease::Smooth },
                    Key {
                        at: KeyTime::After(Frames(90.0)),
                        value: Given::At { node: pika.clone(), by: Vec2::new(0.0, -20.0) },
                        ease: Ease::InOut(EaseFamily::Sine),
                    },
                ],
                mode: TrackMode::Once,
                relative: false,
                exact: false,
            },
            NodeBeat::Animate {
                node: pika.clone(),
                prop: PropId::Tint,
                keys: vec![Key { at: KeyTime::After(Frames(12.0)), value: Given::Is(Value::Colour([255, 0, 0, 128])), ease: Ease::Linear }],
                mode: TrackMode::Loop,
                relative: false,
                exact: false,
            },
            NodeBeat::Path {
                node: pika.clone(),
                points: vec![Vec2::new(0.0, 0.0), Vec2::new(40.0, -20.5), Vec2::new(80.0, 0.0)],
                shape: PathShape::CatmullRom,
                frames: Frames(180.0),
                ease: Ease::Linear,
                mode: TrackMode::Loop,
                orient: true,
                relative: false,
            },
            NodeBeat::Path {
                node: NodeRef::Named("moon".to_string()),
                points: vec![Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0)],
                shape: PathShape::Bezier,
                frames: Frames(60.0),
                ease: Ease::Smooth,
                mode: TrackMode::Once,
                orient: false,
                relative: true,
            },
            NodeBeat::Stop { node: pika.clone(), prop: None },
            NodeBeat::Stop { node: pika.clone(), prop: Some(PropId::Scale) },
        ] {
            let beat = Beat::Node(beat);
            let fields = beat.fields(&SAMPLE).expect("a line");
            assert_eq!(Beat::parse(&fields, &SAMPLE).ok(), Some(beat), "{fields:?}");
        }
    }

    #[test]
    fn a_node_beat_by_its_names() {
        let line = |text: &str| text.split_whitespace().map(str::to_string).collect::<Vec<String>>();
        assert_eq!(
            Beat::parse(&line("prop m25 opacity 50%"), &SAMPLE).ok(),
            Some(Beat::Node(NodeBeat::Set { node: NodeRef::Who(Who::Monster(25)), prop: PropId::Opacity, value: Given::Is(Value::Number(0.5)) }))
        );
        assert_eq!(
            Beat::parse(&line("node g group"), &SAMPLE).ok(),
            Some(Beat::Node(NodeBeat::Make { name: "g".to_string(), sort: Sort::Group, at: Point::at(Vec2::ZERO) }))
        );
        assert_eq!(
            Beat::parse(&line("attach m25 node:raft 0 -8 inherit pos angle"), &SAMPLE).ok(),
            Some(Beat::Node(NodeBeat::Attach {
                node: NodeRef::Who(Who::Monster(25)),
                to: NodeRef::Named("raft".to_string()),
                offset: Some(Vec2::new(0.0, -8.0)),
                inherit: Inherit { pos: true, angle: true, scale: false, opacity: false, colour: false },
            }))
        );
        for wrong in [
            "attach m25",
            "attach m25 m7 3",
            "attach m25 m7 inherit speed",
            "detach",
            "prop m25 opactiy 1",
            "prop m25 opacity",
            "prop m25 opacity 1 2",
            "prop m25 angle red",
            "node g circle",
            "node a:b group",
            "track m25 angle",
            "track m25 angle / 60",
            "track m25 angle / 60 90",
            "track m25 angle / 60 90 soft",
            "track m25 angle turn / 60 90 linear",
            "track m25 opactiy / 60 1 linear",
            "trackpath m25 60 catmull linear / 0 0",
            "trackpath m25 60 spiral linear / 0 0 / 1 1",
            "trackpath m25 60 lines linear / 0 0 / 1",
            "untrack",
            "untrack m25 angle x",
        ] {
            assert!(Beat::parse(&line(wrong), &SAMPLE).is_err(), "{wrong}");
        }
    }
}
