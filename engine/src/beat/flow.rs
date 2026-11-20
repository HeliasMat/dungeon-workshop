//! Where the scene goes and what it waits for.

use super::declare::{Text, declare};
use super::parse::Fields;
use super::*;

/// Where the scene goes and what it waits for.
#[derive(Clone, Debug, PartialEq)]
pub enum FlowBeat {
    /// That many 60ths of a second pass.
    Wait(Frames),
    /// A wait drawn by chance between two (`t 1s~3s`): a crowd's comings
    /// and goings not all in step.
    WaitBetween {
        from: Frames,
        to: Frames,
    },
    /// So many frames pass, a number found as the scene plays (`t {steps}`,
    /// `Num::Var`): a `Wait` once it is known (none: no wait).
    WaitFound(Num),
    /// The scene waits until someone has got where it was going.
    WaitFor(Who),
    /// The scene waits until someone sets off (moves, turns, jumps).
    WaitMoves(Who),
    /// The scene waits until a node's tracks that end are over (one that
    /// loops never ends: it is not waited for).
    WaitTrack(NodeRef),
    /// The scene waits as long as a sound lasts (as whoever plays the story
    /// knows it: see `Director::hear_sounds`), played so many half tones
    /// higher (faster: shorter).
    WaitSound {
        of: SoundOf,
        pitch: i32,
    },
    /// The scene waits until nobody moves any more.
    WaitAll,
    /// The scene waits until the view stops moving (a glide, a zoom), or
    /// the light stops changing (a fade, a dimming, a tint).
    WaitCam,
    WaitFade,
    /// The scene waits until no bubble is left.
    WaitBubbles,
    /// The scene waits until a condition holds (numbers changed by
    /// `After` or `Every`, or by the game).
    WaitUntil(Test),
    /// The scene waits until someone is that near a point.
    WaitNear {
        who: Who,
        at: Point,
        radius: f32,
    },
    /// The player walks the one it steers about (see `PlayBeat::Control`), and
    /// the scene waits: Enter before someone it may talk to (`OnTalk`) plays
    /// their passage and comes back here; going through a way out (`Door`)
    /// goes to its scene. Hurried (a check, a skip): goes on at once.
    Roam,
    /// Talking to someone while roaming plays the passage after that label
    /// (and comes back to the roaming).
    OnTalk {
        who: Who,
        label: String,
    },
    /// The menu key while roaming plays the passage after that label (and
    /// comes back to the roaming).
    OnMenu(String),
    /// The story is saved here (see `Call::Checkpoint`): where it roams, if
    /// it is in a passage played from roaming (a menu, a talk), with where\n    /// the one roaming stands (`Director::saved_at`).
    SaveGame,
    /// Going that near a point while roaming goes to that scene, by the way\n    /// `arrive` (`Director::arrival`).
    /// With a width and a height (`wide`, `high`, both above 0), the way is
    /// that rectangle, `at` its middle, not a circle of `radius`.
    Door {
        scene: String,
        arrive: f32,
        at: Point,
        radius: f32,
        wide: f32,
        high: f32,
    },
    /// What follows, up to `ElseIf`, `Else` or `EndIf`, plays only if it
    /// holds; what follows an `ElseIf`, only if none before held and it
    /// does; what follows `Else`, only if none held.
    If(Test),
    ElseIf(Test),
    Else,
    EndIf,
    /// What follows, up to its `EndWhile`, plays again and again for as
    /// long as it holds.
    While(Test),
    EndWhile,
    /// Out of the `While` at hand, past its `EndWhile`; back to its
    /// `While`, to be asked again.
    Break,
    Continue,
    /// A place in the scene, and going there.
    Label(String),
    Goto(String),
    /// Back past where an exploring player's talk or menu passage was
    /// started from (see `OnTalk`, `OnMenu`).
    Return,
    /// The story goes on in another scene of the event at hand, by its
    /// name (as `scene <level> <name>` gives it).
    GotoScene(String),
    /// The passage after a label (up to its `Return`) played meanwhile,
    /// along with the rest of the scene: its own waits, ifs and loops
    /// (none: every one so played stops).
    Parallel(Option<String>),
    /// A beat that happens that many frames later, whatever the scene
    /// does meanwhile (only one that happens at once: no wait in it).
    After {
        frames: Frames,
        more: Frames,
        beat: Box<Beat>,
    },
    /// A beat that happens again and again, every that many frames, until
    /// `EveryOff` (only one that happens at once: no wait in it).
    Every {
        frames: Frames,
        more: Frames,
        beat: Box<Beat>,
    },
    EveryOff,
    /// A line of the game's story (`story.txt`) whose names are built as it
    /// plays (`set note n{i}`, `random n{k} 1 3`): read when it is played,
    /// the numbers it names put in first. A line whose numbers alone are
    /// found as it plays is its typed beat (`Num::Var`, see `Beat::played`);
    /// a film never makes one of these.
    Filled(Vec<String>),
    /// The story saved here: reopened, the event plays at once up to this
    /// label (see `Director::checkpoint`).
    Checkpoint(String),
    /// Words for whoever writes the scene, shown where the game was started
    /// from (numbers put in: `{name}`).
    Log(String),
    /// A line of the film starts here, on the course it is played on (the
    /// scene's, or a passage meanwhile): told to whoever watches, with how
    /// it ends (see `LineEnd`).
    Line {
        number: i32,
        end: LineEnd,
    },
}

/// How a line of the film ends (`FlowBeat::Line`).
#[derive(Clone, Debug, PartialEq)]
pub enum LineEnd {
    /// When the next line of its course starts, or its course ends: the
    /// course waited for it.
    Next,
    /// When someone gets where it goes (ended by `&`: the course went on).
    Who(Who),
    /// So many frames after it started.
    Frames(Frames),
    /// When a node's tracks that end are over.
    Track(NodeRef),
    /// The line at hand goes on (after another line played inside it): its
    /// number told again, no new start.
    Again,
}

impl FlowBeat {
    /// Whether it steers the course of the scene (it waits, or goes
    /// elsewhere in it): played as the scene goes on, never at once.
    pub fn steers(&self) -> bool {
        matches!(
            self,
            FlowBeat::Wait(_)
                | FlowBeat::WaitFound(_)
                | FlowBeat::WaitAll
                | FlowBeat::WaitSound { .. }
                | FlowBeat::WaitFor(_)
                | FlowBeat::WaitMoves(_)
                | FlowBeat::WaitTrack(_)
                | FlowBeat::WaitCam
                | FlowBeat::WaitFade
                | FlowBeat::WaitBubbles
                | FlowBeat::WaitUntil(_)
                | FlowBeat::Roam
                | FlowBeat::OnTalk { .. }
                | FlowBeat::Door { .. }
                | FlowBeat::OnMenu(_)
                | FlowBeat::If(_)
                | FlowBeat::ElseIf(_)
                | FlowBeat::Else
                | FlowBeat::EndIf
                | FlowBeat::Label(_)
                | FlowBeat::Goto(_)
                | FlowBeat::GotoScene(_)
                | FlowBeat::Return
                | FlowBeat::While(_)
                | FlowBeat::EndWhile
                | FlowBeat::Break
                | FlowBeat::Continue
        )
    }
}

declare! {
    FlowBeat;
    hand: "after", "elif", "every", "filmline", "if", "log",
        "parallel", "t", "wait", "waitsound", "waittrack",
        "waituntil", "while";
    "waitall" => WaitAll {},
    "waitcam" => WaitCam {},
    "waitfade" => WaitFade {},
    "waitbubbles" => WaitBubbles {},
    "waitfor" => WaitFor(who: Who),
    "waitmoves" => WaitMoves(who: Who),
    "waitnear" => WaitNear { who: Who, at: Point, radius: f32 = 16.0 },
    "roam" => Roam {},
    "ontalk" => OnTalk { who: Who, label: Text },
    "door" => Door { scene: Text, arrive: f32, at: Point, radius: f32 = 20.0, wide: f32 = 0.0, high: f32 = 0.0 },
    "onmenu" => OnMenu(label: Text),
    "savegame" => SaveGame {},
    "else" => Else {},
    "endif" => EndIf {},
    "endwhile" => EndWhile {},
    "break" => Break {},
    "continue" => Continue {},
    "label" => Label(label: Text),
    "goto" => Goto(label: Text),
    "return" => Return {},
    "gotoscene" => GotoScene(name: Text),
    "checkpoint" => Checkpoint(label: Text),
}

/// Each of these commands as a film writes it: its forms (`name:kind`, a
/// part between brackets left out, `rest...` the words left) and what it
/// does, for the language and the docs (see `forms`).
pub(in crate::beat) const FORMS: &[Form] = &[
    Form::new(
        "after",
        &["time:duration command..."],
        "the command later, without blocking: `after 30 emote eevee !`; `after 1s~3s …`: after a time drawn at random (murmurs in a crowd)",
    ),
    Form::new("break", &["[text...]"], ""),
    Form::new(
        "checkpoint",
        &["label:word"],
        "the story is saved there; reopened, the scene plays at once up to the `label <label>` and resumes (put the `checkpoint` just before its `label`)",
    ),
    Form::new("continue", &["[text...]"], ""),
    Form::new(
        "door",
        &["scene:word arrival:word x:coordinate y:coordinate [radius:number] [width:number height:number]"],
        "while exploring, closer than this radius to the point (20 pixels), or inside the rectangle of that width and height round it, you go into this scene, by the way `<arrival>` (`Director::arrival`; a scene `game …` tells the game instead); you do not go back through the door you came from before you have left it",
    ),
    Form::new("elif", &["[text...]"], ""),
    Form::new("else", &["[text...]"], ""),
    Form::new("endif", &["[text...]"], ""),
    Form::new("endwhile", &["[text...]"], ""),
    Form::new(
        "every",
        &["time:duration command...", "off"],
        "the command every so many frames or seconds, in the background; `every 3s~6s …`: at intervals drawn at random",
    ),
    Form::new(
        "filmline",
        &["n:number", "n:number until who:code", "n:number for time:duration", "n:number track node:word", "n:number again"],
        "line n of the film starts here, on the course where it plays; it ends when the next one of its course starts (or its course ends), when who has arrived, after so much time, when the node's tracks have ended; `again`: the current line goes on (its number said again, `_line`). Written by films, for `timing` and the Workshop: you do not write it by hand",
    ),
    Form::new("goto", &["label:word"], "a label / going there"),
    Form::new("gotoscene", &["name:word"], "the story carries on in another scene of the same event, by its name"),
    Form::new("if", &["condition..."], "the first true case is played"),
    Form::new("label", &["label:word"], "a label / going there"),
    Form::new("log", &["text..."], "writes to the console (`{name}` replaced): for debugging"),
    Form::new("onmenu", &["label:word"], "the menu key (M or X), while exploring, plays the passage after this label, up to `return`"),
    Form::new(
        "ontalk",
        &["who:code label:word"],
        "when the exploring player talks to them (Enter in front of them, closer than 40 pixels), the passage after this label plays, up to `return`",
    ),
    Form::new(
        "parallel",
        &["label:word", "off"],
        "plays this passage (up to its `return`) **at the same time** as the scene / stops them all. Its lines (`line`) go away on their own after a moment, and let the player move: comments during a trial",
    ),
    Form::new("return", &["[text...]"], ""),
    Form::new(
        "roam",
        &[""],
        "the player explores: they steer the one of `control` with the arrows, and the scene waits. Enter in front of someone with `ontalk` plays their passage, then you explore again; at a `door`, you go into its scene. Without a player at the keyboard (`check`, a frame of the film) or sped up, it goes straight on to what follows",
    ),
    Form::new(
        "savegame",
        &[""],
        "the game is saved: where you explore if you come from exploring (a menu, a dialogue), otherwise here; where the one you steer stands (`Director::saved_at`); a game taken up again (`Author::restore`, `restore_at`) puts them back there",
    ),
    Form::new(
        "t",
        &["duration:duration", "frames:duration"],
        "waits (`t 30`: 30 frames; `t 1.5s`); `t 1s~3s`: a time drawn at random between the two (a crowd that does not move in step)",
    ),
    Form::new(
        "wait",
        &["duration:duration", ""],
        "waits (`t 30`: 30 frames; `t 1.5s`); `t 1s~3s`: a time drawn at random between the two (a crowd that does not move in step)",
    ),
    Form::new("waitall", &[""], "waits until they have finished / until no one moves any more"),
    Form::new("waitbubbles", &[""], "waits until there are no bubbles left"),
    Form::new("waitcam", &[""], "waits until the view no longer moves"),
    Form::new("waitfade", &[""], "waits for the end of a fade, a light, an iris, a wipe, credits"),
    Form::new("waitfor", &["who:code [rest...]"], "waits until they have finished / until no one moves any more"),
    Form::new("waitmoves", &["who:code"], "waits until they set off: `when a leaves` in a film"),
    Form::new(
        "waitnear",
        &["who:code x:coordinate y:coordinate [radius:number]"],
        "waits until they are closer than this radius to the point (16 pixels; `@spot` works)",
    ),
    Form::new(
        "waitsound",
        &["sound:word [pitch n:number]"],
        "waits as long as this sound lasts: a game sound, `move:<n>`, `own:<name>` (as with `playsound`), played `n` semitones higher (shorter); its length is given to the director by whoever plays the story (`Director::hear_sounds`: the game's sound bank, the moves, our files); unknown, half a second, and `check` remarks on it. `wait sound flash` in a film plays it and waits for it",
    ),
    Form::new(
        "waittrack",
        &["node:word"],
        "waits until its tracks that end have ended (a looping track does not end: it is not waited for); `waitfor <who>` waits for their tracks too",
    ),
    Form::new("waituntil", &["condition..."], "the scene waits until it is true"),
    Form::new("while", &["condition..."], "played again as long as it is true; `break` leaves it, `continue` goes back to the start"),
];

pub(super) fn read(fields: &Fields) -> Option<FlowBeat> {
    if let Some(beat) = FlowBeat::declared(fields) {
        return Some(beat);
    }
    // (What a test asks, all the words after the command.)
    let asked = || test(&fields.words_from(1)?);
    Some(match fields.verb()? {
        "t" | "wait" => match fields.word(1).and_then(|time| time.split_once('~')) {
            Some((from, to)) => FlowBeat::WaitBetween { from: Frames(frames_of(from)? as f32), to: Frames(frames_of(to)? as f32) },
            None => match (fields.has(1).then(|| fields.frames(1)).flatten(), fields.num(1)) {
                (Some(frames), _) => FlowBeat::Wait(Frames(frames as f32)),
                // (A number found as it plays: `t {steps}`.)
                (None, Some(Num::Var(name))) => FlowBeat::WaitFound(Num::Var(name)),
                _ => return None,
            },
        },
        "every" => match fields.word(1) {
            Some("off") => FlowBeat::EveryOff,
            // (Every so long, or every while drawn between two: `every 3s~6s`.)
            _ => {
                let (frames, more) = fields.frames_between(1)?;
                FlowBeat::Every { frames: Frames((frames as f32).max(1.0)), more: Frames(more as f32), beat: Box::new(Beat::read(&fields.from(2)?)?) }
            }
        },
        "waituntil" => FlowBeat::WaitUntil(asked()?),
        "waitsound" => {
            let words = fields.words;
            FlowBeat::WaitSound {
                of: fields.lookup(1, "a sound", |sound| SoundOf::read(sound, words))?,
                pitch: match (fields.word(2), fields.has(3)) {
                    (None, _) if !fields.has(2) => 0,
                    (Some("pitch"), true) if fields.len() == 4 => fields.whole(3)?,
                    _ => return None,
                },
            }
        }
        "parallel" => FlowBeat::Parallel(Some(fields.named(1, "a label")?.to_string()).filter(|label| label != "off")),
        // (After so long, or a while drawn between two: `after 1s~3s`.)
        "after" => {
            let (frames, more) = fields.frames_between(1)?;
            FlowBeat::After { frames: Frames(frames as f32), more: Frames(more as f32), beat: Box::new(Beat::read(&fields.from(2)?)?) }
        }
        "if" => FlowBeat::If(asked()?),
        "elif" => FlowBeat::ElseIf(asked()?),
        "waittrack" if fields.len() == 2 => FlowBeat::WaitTrack(fields.lookup(1, "a node", NodeRef::parse)?),
        "while" => FlowBeat::While(asked()?),
        "log" => FlowBeat::Log(fields.words_from(1)?.join(" ")),
        "filmline" => FlowBeat::Line {
            number: fields.whole(1)?,
            end: match (fields.word(2), fields.len()) {
                (None, 2) => LineEnd::Next,
                (Some("again"), 3) => LineEnd::Again,
                (Some("until"), 4) => LineEnd::Who(fields.who(3)?),
                (Some("for"), 4) => LineEnd::Frames(Frames(fields.frames(3)? as f32)),
                (Some("track"), 4) => LineEnd::Track(fields.lookup(3, "a node", NodeRef::parse)?),
                _ => return None,
            },
        },
        _ => return None,
    })
}
