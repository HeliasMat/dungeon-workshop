//! How a `BodyBeat` is written as its line (read by `beat::body::read`).

use crate::beat::{BodyBeat, Play, Pose, SheetAnim, Stacked, colour_named};
use crate::vocabulary::Vocabulary;
use crate::who::Who;

/// The names a popup's colour is read by (`colour_named`).
const COLOUR_NAMES: &[&str] =
    &["white", "red", "yellow", "blue", "green", "black", "pink", "orange", "purple", "cyan", "gold", "grey", "brown", "sand", "turquoise"];

fn line(verb: &str, rest: impl IntoIterator<Item = String>) -> Vec<String> {
    std::iter::once(verb.to_string()).chain(rest).collect()
}

/// Someone, or `off` for no one.
fn or_off(who: &Option<Who>) -> String {
    who.map_or_else(|| "off".to_string(), |who| who.to_string())
}

impl BodyBeat {
    pub(super) fn fields(&self, words: &Vocabulary) -> Option<Vec<String>> {
        if let Some(line) = self.write_declared(words) {
            return line;
        }
        Some(match self {
            BodyBeat::Tremble { who, frames, strength } => {
                vec!["tremble".to_string(), who.to_string(), strength.to_string(), frames.to_string()]
            }
            // (An emote by a name the game gives it: `emote` reads names; any
            // other effect by its number.)
            BodyBeat::Effect { who, id, frames, add } => {
                let mut line = match words.emotes.iter().find(|(_, known)| *known == id.0) {
                    Some((name, _)) => vec!["emote".to_string(), who.to_string(), name.to_string()],
                    None => vec!["effect".to_string(), who.to_string(), id.to_string()],
                };
                line.extend(frames.map(|frames| frames.to_string()));
                line.extend(add.then(|| "plus".to_string()));
                line
            }
            BodyBeat::Anim { who, anim, play } => {
                let flags = |looped: bool, frozen: bool| [who.to_string(), anim.to_string(), u8::from(looped).to_string(), u8::from(frozen).to_string()];
                match *play {
                    Play::Once => line("anim", flags(false, false)),
                    Play::Loop => line("anim", flags(true, false)),
                    Play::Still { or_stand } => line("anim", flags(or_stand, true)),
                    Play::Times(times) => line("anim", [who.to_string(), anim.to_string(), times.to_string(), "times".to_string()]),
                    // (`pose <who> <group> image <step>`, its step counted from 1.)
                    Play::Hold(step) => line("pose", [who.to_string(), anim.to_string(), "image".to_string(), u16::try_from(step + 1).ok()?.to_string()]),
                }
            }
            BodyBeat::Pose { who, pose } => {
                let pose = match pose {
                    Pose::Sleep => "sleep",
                    Pose::Hurt => "hurt",
                    Pose::Attack => "attack",
                    Pose::Walk => "walk",
                    Pose::Idle => "idle",
                };
                line("pose", [who.to_string(), pose.to_string()])
            }
            BodyBeat::Layer { who, place } => {
                let place = match place {
                    Stacked::Over => "over",
                    Stacked::Under => "under",
                    Stacked::Among => "normal",
                };
                line("layer", [who.to_string(), place.to_string()])
            }
            BodyBeat::Slow { who, by } => line("slowmo", [who.to_string(), by.to_string()]),
            BodyBeat::Freeze { who, on: true } => line("still", [who.to_string()]),
            BodyBeat::Freeze { who, on: false } => line("unfreeze", [who.to_string()]),
            BodyBeat::InFront { who, of } => line("infront", [who.to_string(), or_off(of)]),
            BodyBeat::Bar { who, share } => line("bar", std::iter::once(who.to_string()).chain(share.map(|share| share.to_string()))),
            BodyBeat::Status { who, icon, add } => {
                line("status", [who.to_string(), icon.unwrap_or("off").to_string()].into_iter().chain(add.then(|| "plus".to_string())))
            }
            BodyBeat::Carry { what, by, hands } => line(if *hands { "hold" } else { "carry" }, [what.to_string(), or_off(by)]),
            // (`name:` before it, so no pass after takes it for someone of the cast.)
            BodyBeat::ThingName { who, name } => line("thing", std::iter::once(who.to_string()).chain(format!("name:{name}").split(' ').map(str::to_string))),
            BodyBeat::Drawing { who, file, cells, ticks } => {
                let ticks = (!ticks.is_empty()).then(|| ticks.iter().map(u32::to_string).collect::<Vec<_>>().join(","));
                line("drawing", [who.to_string(), file.clone(), cells.to_string()].into_iter().chain(ticks))
            }
            BodyBeat::Sheet { who, file, cell, ways, anims } => line(
                "sheet",
                [who.to_string(), file.clone(), format!("{}x{}", cell.0, cell.1), ways.to_string()].into_iter().chain(anims.iter().map(SheetAnim::written)),
            ),
            // (Held, it has no point to be thrown to: none for one with both.)
            BodyBeat::FishLine { who, to, at, cast } => {
                let rest = match (at, cast) {
                    (Some(at), true) => vec![at.x.to_string(), at.y.to_string()],
                    (None, true) => Vec::new(),
                    (None, false) => vec!["holds".to_string()],
                    (Some(_), false) => return None,
                };
                line("fishline", [who.to_string(), or_off(to)].into_iter().chain(rest))
            }
            BodyBeat::Colour { who, colour: [red, green, blue], frames } => {
                line("colour", [who.to_string(), red.to_string(), green.to_string(), blue.to_string(), frames.to_string()])
            }
            // (Its colour by a name: none for a colour no name gives.)
            BodyBeat::Popup { who, text, colour } => {
                let name = COLOUR_NAMES.iter().find(|name| colour_named(name).map(|named| named.map(|part| part as u8)) == Some(*colour))?;
                line("popup", [who.to_string(), name.to_string()].into_iter().chain(text.split(' ').filter(|_| !text.is_empty()).map(str::to_string)))
            }
            BodyBeat::Views { who, ways } if ways.is_empty() => line("views", [who.to_string(), "off".to_string()]),
            BodyBeat::Views { who, ways } => line("views", std::iter::once(who.to_string()).chain(ways.iter().map(|way| way.name().to_string()))),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::AnimId;
    use crate::EmoteId;
    use crate::Frames;
    use crate::MoveId;
    use crate::beat::{Beat, BodyBeat, Direction, Pose};
    use crate::vocabulary::SAMPLE;
    use crate::who::Who;
    use glam::Vec2;

    #[test]
    fn every_body_beat_written_is_read_back_the_same() {
        let (hero, krabby, target, thing) = (Who::Character(0), Who::Monster(98), Who::Target, Who::Thing { object: 66, copy: 0 });
        let beats = vec![
            BodyBeat::Put { who: hero, at: glam::Vec2::new(12.5, -3.0).into(), direction: Direction::Right },
            BodyBeat::PutNear { who: hero, other: krabby, side: Direction::Left },
            BodyBeat::Gone(krabby),
            BodyBeat::Turn { who: hero, direction: Direction::Up },
            BodyBeat::Anim { who: hero, anim: AnimId(23), play: crate::beat::Play::Loop },
            BodyBeat::Anim { who: hero, anim: AnimId(23), play: crate::beat::Play::Still { or_stand: true } },
            BodyBeat::Pose { who: hero, pose: Pose::Sleep },
            BodyBeat::Pose { who: krabby, pose: Pose::Idle },
            BodyBeat::Rise { who: hero, height: 8.0, frames: Frames(20.0) },
            BodyBeat::Attr { who: hero, bits: 36 },
            BodyBeat::Layer { who: hero, place: crate::beat::Stacked::Over },
            BodyBeat::Blink { who: hero, on: false },
            BodyBeat::MoveEffect { who: target, id: MoveId(52) },
            BodyBeat::MoveShot { from: hero, to: krabby, id: MoveId(52), frames: Some(Frames(14.0)) },
            BodyBeat::MoveShot { from: hero, to: krabby, id: MoveId(52), frames: None },
            BodyBeat::Tremble { who: hero, frames: Frames(30.0), strength: 2.0 },
            BodyBeat::Anim { who: hero, anim: AnimId(13), play: crate::beat::Play::Hold(2) },
            BodyBeat::Anim { who: krabby, anim: AnimId(0), play: crate::beat::Play::Times(2) },
            BodyBeat::Slow { who: hero, by: 0.3 },
            BodyBeat::Freeze { who: krabby, on: true },
            BodyBeat::Freeze { who: krabby, on: false },
            BodyBeat::InFront { who: hero, of: Some(krabby) },
            BodyBeat::InFront { who: hero, of: None },
            BodyBeat::Depth { who: hero, depth: 4.0 },
            BodyBeat::Jump { who: hero, height: 10.0, frames: Frames(12.0) },
            BodyBeat::Hop { who: krabby, at: glam::Vec2::new(40.0, 60.0).into(), height: 12.0, frames: Frames(18.0) },
            BodyBeat::Spin { who: hero, turns: 1.5, frames: Frames(30.0), rotate: false },
            BodyBeat::Spin { who: hero, turns: 2.0, frames: Frames(40.0), rotate: true },
            BodyBeat::Opacity { who: hero, to: 0.5, frames: Frames(20.0) },
            BodyBeat::Scale { who: hero, to: 2.0, frames: Frames(10.0) },
            BodyBeat::Shadow { who: hero, on: false },
            BodyBeat::Flip { who: hero, on: true },
            BodyBeat::Sink { who: hero, depth: 6.0, sand: true },
            BodyBeat::Sink { who: hero, depth: 0.0, sand: false },
            BodyBeat::Reset(hero),
            BodyBeat::Bar { who: krabby, share: Some(0.75) },
            BodyBeat::Bar { who: krabby, share: None },
            BodyBeat::Fidget { who: hero, on: true },
            BodyBeat::Bounce { who: hero, height: 4.0, frames: Frames(16.0) },
            BodyBeat::Status { who: hero, icon: Some("sleep"), add: false },
            BodyBeat::Status { who: hero, icon: Some("sleep"), add: true },
            BodyBeat::Status { who: hero, icon: None, add: false },
            BodyBeat::Carry { what: thing, by: Some(hero), hands: true },
            BodyBeat::Carry { what: thing, by: None, hands: false },
            BodyBeat::Ride { who: hero, on: Some(krabby), high: 16.0, beside: -2.0 },
            BodyBeat::Ride { who: hero, on: None, high: 16.0, beside: 0.0 },
            BodyBeat::Give { what: thing, from: hero, to: krabby, frames: Frames(30.0), keeps: true },
            BodyBeat::Give { what: thing, from: hero, to: krabby, frames: Frames(24.0), keeps: false },
            BodyBeat::ThingName { who: thing, name: "a fishing rod".to_string() },
            BodyBeat::Drawing { who: thing, file: "rod.png".to_string(), cells: 4, ticks: vec![6, 8, 6, 8] },
            BodyBeat::Drawing { who: thing, file: "rod.png".to_string(), cells: 1, ticks: Vec::new() },
            BodyBeat::Sheet {
                who: Who::Monster(60000),
                file: "hero.png".to_string(),
                cell: (32, 24),
                ways: 4,
                anims: vec![
                    crate::beat::SheetAnim { animation: 0, cells: vec![0, 1, 2, 3], ticks: vec![6, 6, 8], row: 0 },
                    crate::beat::SheetAnim { animation: 5, cells: vec![4], ticks: Vec::new(), row: 4 },
                ],
            },
            BodyBeat::Takeable { who: thing, on: false },
            BodyBeat::Shine { who: thing, on: true },
            BodyBeat::Large(Some(thing)),
            BodyBeat::Large(None),
            BodyBeat::FishLine { who: hero, to: Some(thing), at: Some(Vec2::new(80.0, 40.0)), cast: true },
            BodyBeat::FishLine { who: hero, to: Some(thing), at: None, cast: false },
            BodyBeat::FishLine { who: hero, to: None, at: None, cast: true },
            BodyBeat::Throw { who: krabby, at: hero, height: 24.0, frames: Frames(30.0) },
            BodyBeat::Fall { who: krabby, height: 90.0, frames: Frames(18.0) },
            BodyBeat::Float { who: hero, height: 4.0, frames: Frames(60.0) },
            BodyBeat::Colour { who: hero, colour: [255.0, 128.0, 0.0], frames: Frames(10.0) },
            BodyBeat::Recoil { who: krabby, from: hero, distance: 16.0 },
            BodyBeat::Kick { who: hero, from: krabby, distance: 8.0 },
            BodyBeat::TurnAway { who: hero, other: krabby },
            BodyBeat::LookAt { who: hero, at: glam::Vec2::new(10.0, 20.0).into() },
            BodyBeat::Face { who: hero, other: krabby },
            BodyBeat::TurnAround(hero),
            BodyBeat::Effect { who: hero, id: EmoteId(88), frames: Some(Frames(30.0)), add: false },
            BodyBeat::Effect { who: hero, id: EmoteId(88), frames: None, add: true },
            BodyBeat::Scale { who: hero, to: 20.0, frames: Frames(0.0) },
            BodyBeat::Sink { who: hero, depth: 100.0, sand: false },
            BodyBeat::Slow { who: hero, by: 0.01 },
            BodyBeat::Disguise { who: hero, like: Some(krabby) },
            BodyBeat::Disguise { who: hero, like: None },
            BodyBeat::Popup { who: krabby, text: "12 damage".to_string(), colour: [255, 40, 40] },
            BodyBeat::Move { who: hero, id: MoveId(85), target: Some(krabby) },
            BodyBeat::Move { who: hero, id: MoveId(85), target: None },
            BodyBeat::Views { who: hero, ways: vec![Direction::Down, Direction::Right] },
            BodyBeat::Views { who: hero, ways: Vec::new() },
            BodyBeat::Stop(hero),
        ];
        for beat in beats {
            let line = Beat::Body(beat.clone()).fields(&SAMPLE).expect("a line");
            assert_eq!(Beat::parse(&line, &SAMPLE).ok(), Some(Beat::Body(beat)), "{line:?}");
        }
    }
}
