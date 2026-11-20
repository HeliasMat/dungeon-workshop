//! Staging: the screen, the sound, the camera, the world map.

use super::{Act, Cast, Cond, Event, MapAct, Op, Place, act, mark, number};

/// What the screen and the speakers are told to do.
pub(super) fn screen(op: &Op<'_>) -> Option<Event> {
    Some(match op.name {
        "screen_FadeOut" | "screen_FadeOutAll" => op.fade(true, false),
        "screen_WhiteOut" | "screen_WhiteOutAll" => op.fade(true, true),
        "screen_FadeIn" | "screen_FadeInAll" => op.fade(false, false),
        // (A flash: a fade to or from white.)
        "screen_FlushOut" => op.fade(true, true),
        "screen_FlushIn" => op.fade(false, true),
        // The top screen's own fades: nothing of them is shown,
        // but scenes wait for them.
        "screen2_FadeOut" | "screen2_FadeIn" | "screen2_WhiteOut" | "screen2_FlushOut" | "screen2_FlushIn" => {
            act(Act::TopFade { frames: op.at(1), wait: op.at(0) != 0 })
        }
        "WaitSubScreen" => Event::WaitFor(Cond::TopFade),
        // A scene waits for a sound or a jingle to end.
        "WaitSe" => Event::WaitFor(Cond::Sound(op.at(0))),
        "WaitMe" => Event::WaitFor(Cond::Jingle(op.at(0))),
        "WaitScreenFade" | "WaitScreenFadeAll" | "WaitFadeIn" => Event::WaitFor(Cond::Fade),
        // Music.
        // Known gap: fades, volumes and sides are not applied, sounds are
        // not stopped nor waited for.
        "se_Play" | "se_PlayFull" | "se_PlayPan" | "se_PlayVolume" => act(Act::Sound(op.at(0))),
        "me_Play" => act(Act::Jingle(op.at(0))),
        "bgm_Play" | "bgm_PlayFadeIn" => act(Act::Music(Some(op.at(0)))),
        "bgm_Stop" => act(Act::Music(None)),
        // (Fading away: the channel, 0 the music, 1 what is heard
        // around, 2 both; scenes may wait for it.)
        "bgm_FadeOut" => act(Act::MusicFade { channel: 0, frames: op.at(0) }),
        "bgm2_FadeOut" => act(Act::MusicFade { channel: 1, frames: op.at(0) }),
        "sound_FadeOut" => act(Act::MusicFade { channel: 2, frames: op.at(0) }),
        "WaitBgm" => Event::WaitFor(Cond::Music(false)),
        "WaitBgm2" => Event::WaitFor(Cond::Music(true)),
        // (The second channel: what is heard around, the sea...)
        "bgm2_Play" | "bgm2_PlayFadeIn" => act(Act::Ambient(Some(op.at(0)))),
        "bgm2_Stop" => act(Act::Ambient(None)),
        "sound_Stop" => act(Act::Silence),
        // The view shakes (kind 2), along an axis and by so many
        // pixels, until told to stop (all zero).
        // Known gap: what the first two numbers mean exactly is a
        // guess: 2 is taken as a shake, then 1 sideways, 2 up and
        // down.
        "camera_SetEffect" => act(Act::Shake { axis: op.at(1), strength: if op.at(0) == 0 { 0.0 } else { number(op.at(2)).abs() } }),
        // The name of a dungeon on a dark screen, before it.
        "back_SetDungeonBanner" => act(Act::Banner(op.at(0))),
        // Lines that go by themselves after so many frames (none:
        // they wait for a key again).
        "message_SetWaitMode" => act(Act::WaitMode(number(op.at(0)))),
        // The screen dimmed to a part of its light (256: all of it).
        "screen_FadeChange" | "screen_WhiteChange" => {
            act(Act::Dim { white: op.name == "screen_WhiteChange", frames: op.at(1), to: op.at(3), wait: op.at(0) != 0 })
        }
        "supervision_RemoveCommon" => act(Act::RemoveLayer { cast: Cast::Common, layer: op.at(0) }),
        "camera_SetMyself" => act(Act::CameraFollow),
        "camera_SetPositionMark" if op.p.len() >= 4 => {
            let Place::At(x, y) = mark(op.p) else { return None };
            act(Act::CameraAt(x, y))
        }
        "camera2_SetPositionMark" if op.p.len() >= 4 => {
            let Place::At(x, y) = mark(op.p) else { return None };
            act(Act::TopCameraAt(x, y))
        }
        "camera_SetDefault" | "camera_Move2Default" | "camera_MoveDefault" => act(Act::CameraDefault),
        _ => return None,
    })
}

/// What the backdrops and the world map are told to do.
pub(super) fn backdrop(op: &Op<'_>) -> Option<Event> {
    Some(match op.name {
        "back_SetGround" => act(Act::Ground(op.at(0))),
        "back_ChangeGround" => act(Act::Backdrop(op.at(0))),
        "back2_SetGround" => act(Act::TopGround(Some(op.at(0)))),
        // Known gap: of the top screen's modes, 0 (nothing) and 3 (the
        // world map) are known; 4 comes before a ground, 7 before
        // a letter.
        "back2_SetMode" if op.value(0) == 0 => act(Act::TopGround(None)),
        "back2_SetMode" => act(Act::WorldMap(MapAct::Show(op.value(0) == 3))),
        "worldmap_SetMode" => act(Act::WorldMap(MapAct::Mode(op.at(0)))),
        // Known gap: the clouds part at once.
        "worldmap_SetLevel" | "worldmap_ChangeLevel" => {
            let kept = op.name == "worldmap_ChangeLevel";
            act(Act::WorldMap(MapAct::Level { level: op.at(0), kept }))
        }
        "worldmap_SetCamera" | "worldmap_MoveCamera" => {
            let slides = op.name == "worldmap_MoveCamera";
            act(Act::WorldMap(MapAct::Camera { place: op.at(0), slides }))
        }
        "worldmap_SetArrow" => act(Act::WorldMap(MapAct::Arrow(Some(op.at(0))))),
        "worldmap_DeleteArrow" => act(Act::WorldMap(MapAct::Arrow(None))),
        "worldmap_SetMark" | "worldmap_BlinkMark" => {
            let blinks = op.name == "worldmap_BlinkMark";
            act(Act::WorldMap(MapAct::Mark { place: op.at(0), blinks }))
        }
        "worldmap_SetMessagePlace" => act(Act::WorldMap(MapAct::Name(Some(op.at(0))))),
        "worldmap_OffMessage" => act(Act::WorldMap(MapAct::Name(None))),
        _ => return None,
    })
}

impl Op<'_> {
    /// A fade's two parameters: whether to wait for it, and its frames.
    fn fade(&self, out: bool, white: bool) -> Event {
        act(Act::Fade { out, white, frames: self.at(1), wait: self.at(0) != 0 })
    }
}
