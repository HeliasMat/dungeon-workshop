//! A body's beats read from a line.

use super::super::declare::{Free, IfAbsent, Keeps, OnOff, Opt, OrOff, Rotates, Sand, declare};
use super::super::parse::Fields;
use super::super::*;
use super::{BodyBeat, Play, SheetAnim, Stacked};

declare! {
    BodyBeat;
    hand: "anim", "appear", "bar", "carry", "color", "colour", "drawing",
        "effect", "emote", "fishline", "height", "hold",
        "infront", "layer", "popup", "pose", "sheet", "slowmo", "status", "still", "thing", "tremble", "uncolor", "uncolour",
        "unfreeze", "vanish", "views";
    "put" => Put { who: Who, at: Point, direction: Direction },
    "putnear" => PutNear { who: Who, other: Who, side: IfAbsent<Direction> = Direction::Right },
    "gone" => Gone(who: Who),
    "turn" => Turn { who: Who, direction: Direction },
    "face" => Face { who: Who, other: Who },
    "turnaround" => TurnAround(who: Who),
    "turnby" => TurnBy { who: Who, eighths: f32 },
    "turnslow" => TurnSlow { who: Who, every: Frames = Frames(6.0) },
    "lookat" => LookAt { who: Who, at: Point },
    "rise" => Rise { who: Who, height: f32, frames: Frames },
    "jump" => Jump { who: Who, height: f32 = 10.0, frames: Frames = Frames(12.0) },
    "hop" => Hop { who: Who, at: Point, height: f32, frames: Frames },
    "bounce" => Bounce { who: Who, height: f32 = 4.0, frames: Frames = Frames(16.0) },
    "float" => Float { who: Who, height: f32 = 4.0, frames: Frames = Frames(60.0) },
    "fall" => Fall { who: Who, height: f32 = 90.0, frames: Frames = Frames(18.0) },
    "throw" => Throw { who: Who, at: Who, height: f32 = 24.0, frames: Frames = Frames(30.0) },
    "spin" => Spin { who: Who, turns: f32, frames: Frames, rotate: Rotates },
    "recoil" => Recoil { who: Who, from: Who, distance: f32 = 16.0 },
    "stop" => Stop(who: Who),
    "ride" => Ride { who: Who, on: OrOff<Who>, high: f32 = 16.0, beside: f32 = 0.0 },
    "give" => Give { what: Who, from: Who, to: Who, frames: Frames = Frames(30.0), keeps: Keeps },
    "disguise" => Disguise { who: Who, like: OrOff<Who> },
    "large" => Large(who: OrOff<Who>),
    "takeable" => Takeable { who: Who, on: IfAbsent<OnOff> = true },
    "shine" => Shine { who: Who, on: IfAbsent<OnOff> = true },
    "fidget" => Fidget { who: Who, on: IfAbsent<OnOff> = true },
    "attr" => Attr { who: Who, bits: u16 },
    "opacity" => Opacity { who: Who, to: f32, frames: Frames = Frames(0.0) },
    "scale" => Scale { who: Who, to: Free, frames: Frames = Frames(0.0) },
    "blink" => Blink { who: Who, on: OnOff },
    "shadow" => Shadow { who: Who, on: OnOff },
    "flip" => Flip { who: Who, on: OnOff },
    "sink" => Sink { who: Who, depth: Free, sand: Sand },
    "move" => Move { who: Who, id: MoveId, target: Opt<Who> },
    "moveeffect" => MoveEffect { who: Who, id: MoveId },
    "moveshot" => MoveShot { from: Who, to: Who, id: MoveId, frames: Opt<Frames> },
    "reset" => Reset(who: Who),
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new(
        "anim",
        &["who:code name:word [loop] [frozen]", "who:code name:word n:number times", "who:code group:word loop:word frozen:word"],
        "an animation of their sprite, by its name (table below)",
    ),
    Form::new("appear", &["who:code [frames:duration]"], "fades out / fades in (20 frames by default)"),
    Form::new("attr", &["who:code bits:word"], "32 blinks, 64 transparent, 128 hidden, 8 on top (`blink`, `opacity`, `layer`)"),
    Form::new("bar", &["who:code v:word to rest...", "who:code off"], "a little health bar above them: what they have left"),
    Form::new("blink", &["who:code rest..."], "blinks"),
    Form::new(
        "bounce",
        &["who:code [height:number] [frames:duration]"],
        "hops without stopping (4 pixels, 16 frames; `0`: stops); does not count for `waitall`",
    ),
    Form::new(
        "carry",
        &["item:word who:code", "item:word off"],
        "the item (or someone) is carried above their head and follows them everywhere / put down where it is",
    ),
    Form::new("color", &["rest..."], "`colour` and `uncolour`, other spelling"),
    Form::new("colour", &["who:code r:number g:number b:number [frames:duration]"], "tints them (out of 255: `colour pika 255 120 120` reddens)"),
    Form::new(
        "disguise",
        &["who:code other:code", "who:code off"],
        "they have the other's picture, and their name and face in what they say: an illusion (`disguise fake partner`), an evolution",
    ),
    Form::new(
        "drawing",
        &["item:word file:word [frames:duration] [rhythm:word]"],
        "a picture of ours for `d<n>`, taken from `data/hand/` or `library/images/`, to give before its `put`; `frames` side by side: the first at rest, the next its animation; `rhythm` (`40,2,3`): the sixtieths of a second of each frame, and the animation plays them all, the first included",
    ),
    Form::new(
        "effect",
        &["who:code n:number [frames:duration]"],
        "an emote or effect by number: 88 \"!\", 89 sweat, 90 \"…\", 91 anger, 92 \"?\", 93 notes, 171 tears, 642 sparkles (`emote`)",
    ),
    Form::new(
        "emote",
        &["who:code name:word [frames:duration] [plus]"],
        "`!` `?` `...` `sweat` `anger` `joy` `tears` `shock` `sparkle` (`none`: removes them); with a duration, goes away on its own; `plus`: with those they already show, instead of replacing them (as many as you like)",
    ),
    Form::new("face", &["who:code other:code"], "turns towards the other"),
    Form::new("fall", &["who:code [height:number] [frames:duration]", ""], "falls from high up to the ground (90 pixels, 18 frames)"),
    Form::new("fidget", &["who:code [rest...]"], "looks around from time to time (a lively crowd); does not count for `waitall`"),
    Form::new(
        "fishline",
        &["who:code item:word [x:coordinate y:coordinate]", "who:code item:word holds", "who:code off"],
        "a fishing line, from the end of their rod (`tip` in their variant's table) to the item, which flies in an arc to the point, or in front of them; `holds`: only tied to what is already there (a catch, where it comes out of the water); they keep an eye on it",
    ),
    Form::new("flip", &["who:code rest..."], "their picture flipped from left to right (a game item painted facing one way only: a wake) / put back"),
    Form::new("float", &["who:code [height:number] [frames:duration]"], "floats up and down (4 pixels, 60 frames; `0`: stops); does not count for `waitall`"),
    Form::new(
        "give",
        &["item:word from:word to:word [frames:duration] [keep]"],
        "an item (`i109` an Apple) passes from one's hands to the other's in an arc, then vanishes; with `keep`, it first shows above the giver and stays in the other's hands",
    ),
    Form::new("gone", &["who:code [rest...]"], "removes them from the scene"),
    Form::new("height", &["who:code height:number [frames:duration]"], "at this height above the ground, at once or in so many frames"),
    Form::new(
        "hold",
        &["item:word who:code", "item:word off"],
        "the item (already placed by `put`) in their hands, in front of them, on the side they look, and it follows them everywhere / dropped where it is",
    ),
    Form::new("hop", &["who:code x:coordinate y:coordinate height:number frames:duration", ""], "a jump in an arc to there"),
    Form::new("infront", &["who:code otheroff:word"], "drawn in front of the other, wherever they are"),
    Form::new("jump", &["who:code [height:number] [frames:duration]"], "a jump on the spot (10 pixels, 12 frames)"),
    Form::new("large", &["item:word", "off"], "the item large, crisp, above the scene"),
    Form::new("layer", &["who:code rest..."], "drawn over everyone, under, or among the others"),
    Form::new("lookat", &["who:code x:coordinate y:coordinate"], "turns towards a point (`lookat eevee @fountain`)"),
    Form::new(
        "move",
        &["who:code move:word [target:word]"],
        "uses a move (`data/rom/moves.txt`) as in a dungeon: its gesture, its effects, its sound, the target takes it",
    ),
    Form::new("moveeffect", &["who:code move:word"], "what a move shows on arrival, on who, without a blow"),
    Form::new("moveshot", &["from:word to:word move:word [frames:duration]"], "what it throws, from one to the other, without a blow"),
    Form::new("opacity", &["who:code v:word to rest..."], "half visible, a quarter… (`opacity <who> 0 0` right after a `put`: invisible at first)"),
    Form::new("popup", &["who:code colour:word words..."], "words that rise from them and go away: `-12`, `Frozen!`"),
    Form::new("pose", &["who:code name:word image n:number", "who:code rest..."], "this frame of the animation (the first: 1), and they stay on it"),
    Form::new("put", &["who:code x:coordinate y:coordinate dir:word"], "puts them there, facing that way"),
    Form::new("putnear", &["who:code other:code [side:word]"], "right next to the other (to the right by default; `left`, `up`, `down`…), facing them"),
    Form::new("recoil", &["who:code from:word [distance:number]"], "takes a blow from the other: backs off (16 pixels), is hurt, trembles, turns towards them"),
    Form::new(
        "reset",
        &["who:code"],
        "as they were when put, where they are: their size, their colours, visible, among the others, with their shadow, without a name, doing nothing more",
    ),
    Form::new(
        "ride",
        &["who:code mount:word [height:number] [offset:number]", "who:code off"],
        "sitting on the mount's back, so many pixels above its feet (16), so many to the side: they follow it everywhere, go up and down with it / left where they are",
    ),
    Form::new("rise", &["who:code height:number frames:duration"], "rises (or comes back down) above the ground"),
    Form::new("scale", &["who:code size:number [frames:duration]"], "grows or shrinks (`scale pika 2 30`; 1: their size)"),
    Form::new("shadow", &["who:code rest..."], "without a shadow (someone who flies high) / with one"),
    Form::new(
        "sheet",
        &["who:code file:word w:word v:word [rest...]"],
        "a character of your own drawn from a sheet (`character`): cells of W×H, one row per direction (1: all the same; 4: down, left, right, up; 8: down, downright, right, upright, up, upleft, left, downleft); each animation: its number (0 walk, 1 idle, 2 hurt, 3 sleep, 4 attack, 5 + n the n-th of your own), its cells on the row (`0,1,2`), their sixtieths (`6,6,8`, `-`: 8 each), the row of its first direction",
    ),
    Form::new("shine", &["item:word [off]"], "a little star of crisp pixels turns around it"),
    Form::new(
        "sink",
        &["who:code pixels:number [sand]"],
        "sunk in the water by so many pixels: drawn lower, nothing of them below the surface (the line of their feet), those on their back with them; `sand`: in sand, the surface line sand-coloured; 0: out of the water",
    ),
    Form::new(
        "slowmo",
        &["who:code timesoff:word rest..."],
        "their own time alone, so many times slower (or faster): their steps, their jumps, their laps, their frames, what shows on them; the rest of the scene on the scene's time (`tempo` multiplies them). What waits for them (`waitfor`) waits for their slowed time. `off` (or `1`): like the scene",
    ),
    Form::new(
        "spin",
        &["who:code laps:word frames:duration [rotate]"],
        "spins on the spot: through their eight directions; `rotate`: their picture itself turns, their direction kept",
    ),
    Form::new(
        "status",
        &["who:code name:word [plus]", "who:code off"],
        "the dungeon status icon above them, until `off`, in place of theirs, or with them (`plus`): `sleep`, `burn`, `poison`, `toxic`, `curse`, `frozen` (in its block of ice), `confused`, `cringe`, `protect`, `safeguard`, `reflect`, `light_screen`, `endure`, `low_hp`, `lowered`, `sleepless`, `taunt`, `encore`, `embargo`, `sure_shot`, `set_damage`, `focus_energy`, `cross_eyed`, `grudge`, `heal_block`",
    ),
    Form::new("still", &["who:code rest...", "rest..."], "they stay as they are, mid-gesture, and no longer count for `waitall`; then they go on"),
    Form::new("stop", &["who:code"], "stops everything they do: walk, path, lap, stroll, follow, gaze, float, spin"),
    Form::new("takeable", &["item:word [off]"], "it can be picked up, or not"),
    Form::new("thing", &["item:word rest..."], "the name of an item of the scene, for `state` and `do`"),
    Form::new(
        "throw",
        &["who:code to:word [height:number] [frames:duration]"],
        "thrown in an arc to where the other stands at that moment (24 pixels high, 30 frames)",
    ),
    Form::new("tremble", &["who:code [strength:number] frames:duration"], "trembles, by so many pixels (1)"),
    Form::new("turn", &["who:code dir:word"], "turns"),
    Form::new("turnaround", &["who:code"], "turns right around"),
    Form::new(
        "turnby",
        &["who:code eighths:word"],
        "turns so many eighths from where they look, at once (counter-clockwise, clockwise below 0): the game's look around and head shake",
    ),
    Form::new(
        "turnslow",
        &["who:code [frames:duration]"],
        "their next turn goes the slow way, through each way in between, one every so many frames (6 otherwise), as the game's slow turns",
    ),
    Form::new("uncolor", &["rest..."], "`colour` and `uncolour`, other spelling"),
    Form::new("uncolour", &["who:code [frames:duration]"], "their colours"),
    Form::new("unfreeze", &["[text...]"], ""),
    Form::new("vanish", &["who:code [frames:duration]"], "fades out / fades in (20 frames by default)"),
    Form::new(
        "views",
        &["who:code rest...", "who:code off"],
        "the only directions they are drawn in (a variant, `views down` in its table): when stopped, they go back to it on their own; lines do not turn them; a `turn`, `face`, `lookat` in the film turns them until their next walk or `pose <who> idle`; `film/` writes it on its own",
    ),
];

pub(in crate::beat) fn read(fields: &Fields) -> Option<BodyBeat> {
    // (A size, a depth, a time: any number but a negative one, which no
    // body can have; past what is advised, the checker remarks it.)
    let free = |at: usize| fields.real(at).filter(|value| *value >= 0.0);
    // (`plus` at its end: with those shown, not in their place.)
    let add = fields.len() > 3 && fields.is(fields.len() - 1, "plus");
    let fields = &if add { fields.before(fields.len() - 1) } else { *fields };
    // (A regular command, read from the line without its `plus`.)
    if let Some(beat) = BodyBeat::declared(fields) {
        return Some(beat);
    }
    let words = fields.words;
    // (An animation by its number, or by its name: `Sit`.)
    let anim = |at: usize| match fields.word(at) {
        Some(name) if name.parse::<u16>().is_err() => fields.lookup(at, "an animation", |name| u16::try_from(words.animation(name)?).ok()).map(AnimId),
        _ => fields.number(at).map(AnimId),
    };
    let verb = fields.verb()?;
    Some(match verb {
        // (The group by its number, or by its name: `anim pika Sit 1`.)
        "anim" if !fields.is(4, "times") => BodyBeat::Anim { who: fields.who(1)?, anim: anim(2)?, play: Play::of(fields.flag(3), fields.flag(4)) },
        "popup" => BodyBeat::Popup {
            who: fields.who(1)?,
            text: fields.words_from(3)?.join(" "),
            colour: fields.lookup(2, "a colour", colour_named)?.map(|part| part as u8),
        },
        "carry" | "hold" => BodyBeat::Carry {
            what: fields.who(1)?,
            by: match fields.word(2) {
                Some("off" | "-") => None,
                _ => Some(fields.who(2)?),
            },
            hands: verb == "hold",
        },
        // (`name:`: a name no one takes for someone of the cast.)
        "thing" => BodyBeat::ThingName {
            who: fields.who(1)?,
            name: fields.words_from(2).filter(|name| !name.is_empty())?.join(" ").trim_start_matches("name:").to_string(),
        },
        // (`drawing <who> <file> [cells] [ticks,ticks,...]`.)
        "drawing" => BodyBeat::Drawing {
            who: fields.who(1)?,
            file: fields.named(2, "a file")?.to_string(),
            cells: fields.number(3).unwrap_or(1).max(1),
            ticks: match fields.has(4) {
                true => fields.lookup(4, "ticks (12,8,…)", |ticks| {
                    ticks.split(',').map(|tick| tick.parse::<u32>().ok().filter(|tick| *tick > 0)).collect::<Option<Vec<u32>>>()
                })?,
                false => Vec::new(),
            },
        },
        // (`sheet <who> <file> <W>x<H> <ways> [anim:cells:ticks:row ...]`.)
        "sheet" => {
            let cell = fields.lookup(3, "a size (WxH)", |size| {
                let (wide, high) = size.split_once('x')?;
                Some((wide.parse::<u16>().ok().filter(|wide| *wide > 0)?, high.parse::<u16>().ok().filter(|high| *high > 0)?))
            })?;
            let ways = fields.number(4).filter(|ways| matches!(ways, 1 | 4 | 8))?;
            let anims = fields.words_from(5)?.iter().map(|field| SheetAnim::read(field)).collect::<Option<Vec<_>>>()?;
            BodyBeat::Sheet { who: fields.who(1)?, file: fields.named(2, "a file")?.to_string(), cell, ways: ways as u8, anims }
        }
        "fishline" => BodyBeat::FishLine {
            who: fields.who(1)?,
            to: match fields.word(2) {
                Some("off" | "-") => None,
                _ => Some(fields.who(2)?),
            },
            at: fields.real(3).zip(fields.real(4)).map(|(x, y)| Vec2::new(x, y)),
            cast: !fields.is(3, "holds"),
        },
        "appear" => BodyBeat::Opacity { who: fields.who(1)?, to: 1.0, frames: Frames(fields.real(2).unwrap_or(20.0)) },
        "vanish" => BodyBeat::Opacity { who: fields.who(1)?, to: 0.0, frames: Frames(fields.real(2).unwrap_or(20.0)) },
        // (Each part of 255 at most: a colour has no other.)
        "colour" | "color" => {
            let part = |at: usize| fields.real(at).filter(|part| (0.0..=255.0).contains(part));
            BodyBeat::Colour { who: fields.who(1)?, colour: [part(2)?, part(3)?, part(4)?], frames: Frames(fields.real(5).unwrap_or(0.0)) }
        }
        "uncolour" | "uncolor" => BodyBeat::Colour { who: fields.who(1)?, colour: [255.0; 3], frames: Frames(fields.real(2).unwrap_or(0.0)) },
        // (Over everyone, under everyone, among the others.)
        "layer" => BodyBeat::Layer {
            who: fields.who(1)?,
            place: fields.lookup(2, "over, under or normal", |place| match place {
                "over" => Some(Stacked::Over),
                "under" => Some(Stacked::Under),
                "normal" => Some(Stacked::Among),
                _ => None,
            })?,
        },
        "bar" => BodyBeat::Bar { who: fields.who(1)?, share: fields.real(2).map(|share| share.clamp(0.0, 1.0)) },
        "status" => BodyBeat::Status {
            who: fields.who(1)?,
            icon: match fields.word(2) {
                Some("off" | "-") => None,
                _ => Some(fields.lookup(2, "a status (asleep, confused…)", |name| words.status_icon(name))?),
            },
            add,
        },
        "effect" => BodyBeat::Effect { who: fields.who(1)?, id: EmoteId(fields.number(2)?), frames: fields.real(3).map(Frames), add },
        // (An emote by its name: `emote <who> !`, `?`, `...`, `sweat`...)
        "emote" => BodyBeat::Effect {
            who: fields.who(1)?,
            id: EmoteId(fields.lookup(2, "an emote (joy, !, ?, sweat…)", |name| words.emote(name))?),
            frames: fields.real(3).map(Frames),
            add,
        },
        // (`tremble <who> <frames>`, or `tremble <who> <strength> <frames>`.)
        "tremble" => match fields.real(3) {
            Some(frames) => BodyBeat::Tremble { who: fields.who(1)?, frames: Frames(frames), strength: fields.real(2)?.max(0.0) },
            None => BodyBeat::Tremble { who: fields.who(1)?, frames: Frames(fields.real(2)?), strength: 1.0 },
        },
        // (`pose <who> Attack image 3`: that step of it, and it stays.)
        "pose" if fields.is(3, "image") => {
            BodyBeat::Anim { who: fields.who(1)?, anim: anim(2)?, play: Play::Hold((fields.number(4)? as usize).saturating_sub(1)) }
        }
        // (`anim <who> Walk 2 times`.)
        "anim" => BodyBeat::Anim { who: fields.who(1)?, anim: anim(2)?, play: Play::Times(fields.number(3)?.max(1)) },
        // (`slowmo <who> 0.3`, `slowmo <who> off`: its own time.)
        "slowmo" => BodyBeat::Slow {
            who: fields.who(1)?,
            by: match fields.word(2) {
                Some("off") => 1.0,
                _ => free(2)?,
            },
        },
        // (`still <who>`; of the whole stage, see `ScreenBeat::Still`.)
        "still" if fields.has(1) && fields.word(1).and_then(on_off).is_none() => BodyBeat::Freeze { who: fields.who(1)?, on: true },
        "unfreeze" => BodyBeat::Freeze { who: fields.who(1)?, on: false },
        "infront" => BodyBeat::InFront { who: fields.who(1)?, of: (!fields.is(2, "off")).then(|| fields.who(2)).flatten() },
        // (How high it stands, at once or in so many frames.)
        "height" => BodyBeat::Rise { who: fields.who(1)?, height: fields.real(2)?, frames: Frames(fields.real(3).unwrap_or(1.0)) },
        "pose" => BodyBeat::Pose {
            who: fields.who(1)?,
            pose: match fields.word(2) {
                Some("sleep") => Pose::Sleep,
                Some("hurt") => Pose::Hurt,
                Some("attack") => Pose::Attack,
                Some("walk") => Pose::Walk,
                _ if fields.has(2) => Pose::Idle,
                _ => return None,
            },
        },
        // (`views <who> down [right...]`; `views <who> off`: all of them.)
        "views" => BodyBeat::Views {
            who: fields.who(1)?,
            ways: match fields.word(2) {
                Some("off") => Vec::new(),
                _ if fields.has(2) => (2..fields.len()).map(|at| fields.lookup(at, "a direction", Direction::of)).collect::<Option<Vec<Direction>>>()?,
                _ => return None,
            },
        },
        _ => return None,
    })
}
