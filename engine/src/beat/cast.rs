//! Who a beat is about.

use super::*;

/// What a beat does with someone it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Brought on the stage by it: put, given, arriving, in a crowd.
    Comes,
    /// Acted on: on the stage before it.
    Acts,
    /// Only named: a portrait, a sound from there, a question about them,
    /// what a thing is called or looks like before it shows.
    Named,
}

impl Beat {
    /// Everyone the beat names, with what it does with them: the one list of
    /// them. `target` is replaced in all of it, the lint's checks are read
    /// from it; a beat played later (`after`, `every`) names those of its own.
    pub(crate) fn named(&mut self) -> Vec<(Role, &mut Who)> {
        use Role::{Acts, Comes, Named};
        match self {
            // (Brought on.)
            Beat::Body(BodyBeat::Put { who, .. }) | Beat::Place(PlaceBeat::Arrives { who, .. }) => vec![(Comes, who)],
            Beat::Body(BodyBeat::PutNear { who, other, .. }) => vec![(Comes, who), (Acts, other)],
            Beat::Body(BodyBeat::Give { what, from, to, .. }) => vec![(Comes, what), (Acts, from), (Acts, to)],
            Beat::Place(PlaceBeat::Spread { who, .. } | PlaceBeat::Crowd { who, .. }) | Beat::Motion(MotionBeat::Ring { who, .. }) => {
                who.iter_mut().map(|who| (Comes, who)).collect()
            }
            Beat::Place(PlaceBeat::Gathers { who, .. }) => who.iter_mut().flatten().map(|who| (Comes, who)).collect(),
            // (Acted on, alone.)
            Beat::Body(
                BodyBeat::Turn { who, .. }
                | BodyBeat::Anim { who, .. }
                | BodyBeat::Rise { who, .. }
                | BodyBeat::Attr { who, .. }
                | BodyBeat::Gone(who)
                | BodyBeat::Effect { who, .. }
                | BodyBeat::Hop { who, .. }
                | BodyBeat::Spin { who, .. }
                | BodyBeat::LookAt { who, .. }
                | BodyBeat::Jump { who, .. }
                | BodyBeat::Tremble { who, .. }
                | BodyBeat::Pose { who, .. }
                | BodyBeat::TurnAround(who)
                | BodyBeat::TurnBy { who, .. }
                | BodyBeat::TurnSlow { who, .. }
                | BodyBeat::Reset(who)
                | BodyBeat::Opacity { who, .. }
                | BodyBeat::Float { who, .. }
                | BodyBeat::Colour { who, .. }
                | BodyBeat::Stop(who)
                | BodyBeat::Layer { who, .. }
                | BodyBeat::Blink { who, .. }
                | BodyBeat::Scale { who, .. }
                | BodyBeat::Bar { who, .. }
                | BodyBeat::Fidget { who, .. }
                | BodyBeat::Bounce { who, .. }
                | BodyBeat::Shadow { who, .. }
                | BodyBeat::Flip { who, .. }
                | BodyBeat::Sink { who, .. }
                | BodyBeat::Status { who, .. }
                | BodyBeat::Slow { who, .. }
                | BodyBeat::Freeze { who, .. }
                | BodyBeat::MoveEffect { who, .. },
            )
            | Beat::Motion(
                MotionBeat::Go { who, .. }
                | MotionBeat::Walk { who, .. }
                | MotionBeat::Path { who, .. }
                | MotionBeat::Speed { who, .. }
                | MotionBeat::Wander { who, .. }
                | MotionBeat::Orbit { who, .. }
                | MotionBeat::Step { who, .. }
                | MotionBeat::Resume(who),
            )
            | Beat::Flow(FlowBeat::WaitFor(who))
            | Beat::Flow(FlowBeat::WaitMoves(who))
            | Beat::Speech(SpeechBeat::Bubble { who, .. })
            | Beat::Screen(ScreenBeat::Emitter { who, .. })
            | Beat::Sound(SoundBeat::OnHit { who, .. })
            | Beat::View(
                ViewBeat::Iris { on: Some(who), .. }
                | ViewBeat::Glance { to: Cam::Who(who), .. }
                | ViewBeat::Camera(Some(Cam::Who(who)))
                | ViewBeat::Pan { to: Cam::Who(who), .. }
                | ViewBeat::PanStays { to: Cam::Who(who), .. }
                | ViewBeat::Spotlight(Some(Lit { on: Cam::Who(who), .. })),
            ) => vec![(Acts, who)],
            // (Acted on, with another.)
            Beat::Body(BodyBeat::Face { who, other } | BodyBeat::Recoil { who, from: other, .. } | BodyBeat::MoveShot { from: who, to: other, .. })
            | Beat::Motion(
                MotionBeat::Approach { who, other, .. }
                | MotionBeat::Flee { who, from: other, .. }
                | MotionBeat::Chase { who, other }
                | MotionBeat::OrbitAround { who, other, .. },
            )
            | Beat::Data(DataBeat::Measure { a: who, b: other, .. }) => vec![(Acts, who), (Acts, other)],
            Beat::Body(BodyBeat::Move { who, target: other, .. } | BodyBeat::InFront { who, of: other })
            | Beat::Motion(MotionBeat::Follow { who, leader: other, .. } | MotionBeat::Mirror { who, model: other }) => {
                std::iter::once((Acts, who)).chain(other.as_mut().map(|other| (Acts, other))).collect()
            }
            // (Acted on, many: the first two looked for on the stage.)
            Beat::Motion(MotionBeat::LineUp { who, .. } | MotionBeat::Train { who, .. })
            | Beat::View(ViewBeat::Frame { who, .. } | ViewBeat::KeepFrame(Some(who))) => {
                who.iter_mut().enumerate().map(|(at, who)| (if at < 2 { Acts } else { Named }, who)).collect()
            }
            // (Only named.)
            Beat::Body(
                BodyBeat::Fall { who, .. }
                | BodyBeat::ThingName { who, .. }
                | BodyBeat::Drawing { who, .. }
                | BodyBeat::Sheet { who, .. }
                | BodyBeat::Views { who, .. }
                | BodyBeat::Takeable { who, .. }
                | BodyBeat::Shine { who, .. }
                | BodyBeat::Large(Some(who))
                | BodyBeat::Popup { who, .. }
                | BodyBeat::FishLine { who, to: None, .. },
            )
            | Beat::Speech(
                SpeechBeat::PortraitSide { who, .. } | SpeechBeat::Mood { who, .. } | SpeechBeat::HoldGaze { who, .. } | SpeechBeat::Typing { who, .. },
            )
            | Beat::Flow(FlowBeat::WaitNear { who, .. } | FlowBeat::OnTalk { who, .. })
            | Beat::Motion(MotionBeat::WalkTo { who, .. })
            | Beat::View(ViewBeat::SpotlightAdd(Cam::Who(who), _))
            | Beat::Data(DataBeat::Position { who, .. })
            | Beat::Play(PlayBeat::Control(Some(who)))
            | Beat::Screen(
                ScreenBeat::Particles { on: Cam::Who(who), .. }
                | ScreenBeat::Backdrop(_, _, Some(who))
                | ScreenBeat::Halo(Some(Halo { on: Cam::Who(who), .. }))
                | ScreenBeat::Lightning(Cam::Who(who)),
            )
            | Beat::Sound(SoundBeat::SoundFrom { from: who, .. } | SoundBeat::Play(Played { pan: Side::From { who, .. }, .. })) => vec![(Named, who)],
            Beat::Body(BodyBeat::FishLine { who, to: Some(other), .. } | BodyBeat::Throw { who, at: other, .. })
            | Beat::Data(DataBeat::Sees { who, other, .. }) => vec![(Named, who), (Named, other)],
            Beat::Body(
                BodyBeat::Carry { what: who, by: other, .. }
                | BodyBeat::Ride { who, on: other, .. }
                | BodyBeat::Disguise { who, like: other }
                | BodyBeat::Mimic { who, like: other },
            ) => std::iter::once((Named, who)).chain(other.as_mut().map(|other| (Named, other))).collect(),
            Beat::Play(PlayBeat::ControlTeam(who)) | Beat::Motion(MotionBeat::KeepOnly(who)) => who.iter_mut().map(|who| (Named, who)).collect(),
            Beat::Flow(FlowBeat::After { beat, .. } | FlowBeat::Every { beat, .. }) => beat.named(),
            // (A property of someone, of the emitter on someone, a point by
            // someone's.)
            Beat::Node(NodeBeat::Set { node, value, .. }) => {
                let mut named = Vec::new();
                match node {
                    NodeRef::Who(who) => named.push((Acts, who)),
                    NodeRef::Emitter(who) => named.push((Named, who)),
                    _ => {}
                }
                if let crate::props::Given::At { node: NodeRef::Who(who) | NodeRef::Emitter(who), .. } = value {
                    named.push((Named, who));
                }
                named
            }
            // (Who hangs on someone, and who they hang on.)
            Beat::Node(NodeBeat::Attach { node, to, .. }) => [node, to]
                .into_iter()
                .filter_map(|node| match node {
                    NodeRef::Who(who) | NodeRef::Emitter(who) => Some((Named, who)),
                    _ => None,
                })
                .collect(),
            Beat::Node(NodeBeat::Detach(NodeRef::Who(who))) => vec![(Named, who)],
            _ => Vec::new(),
        }
    }

    /// Everyone the beat names, to be changed (see `Who::Target`).
    pub(crate) fn each_who_mut(&mut self) -> Vec<&mut Who> {
        self.named().into_iter().map(|(_, who)| who).collect()
    }

    /// Those the beat names that do this.
    pub fn naming(&self, role: Role) -> Vec<Who> {
        self.clone().named().into_iter().filter(|(done, _)| *done == role).map(|(_, who)| *who).collect()
    }
}
