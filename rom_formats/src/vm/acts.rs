//! What a script asks of who plays it.

use super::*;

/// Something of a scene that runs a routine: a character, a prop, or an
/// invisible "performer" (a spot the camera can follow, for instance).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Who {
    Actor(u16),
    Object(u16),
    Performer(u16),
}

/// A spot of the level, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    At(f32, f32),
    /// From where the character is.
    By(f32, f32),
    /// Where another character is.
    Actor(u16),
    /// Where the scene put the character.
    Initial,
}

/// Where to look: a direction (1 to 8, starting facing down and turning
/// counter-clockwise; scripts count them from 0), or towards a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Direction(u16),
    Actor(u16),
    /// Towards whoever started the conversation.
    Talker,
    /// That many eighths of a turn from where it faces.
    Around(u16),
}

/// A staging operation. Those about a character apply to the one running
/// the routine.
/// What scripts do with the world map of the top screen. Places are
/// numbers of the table of markers, [`MARKER_HERE`] or [`MARKER_CHOSEN`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MapAct {
    /// 1 as the team travels to a dungeon, 3 as clouds part, 4 in scenes.
    Mode(u16),
    /// Show it, or take it away.
    Show(bool),
    /// How much of it the clouds leave (1 to 8), for now or from now on.
    Level { level: u16, kept: bool },
    /// Its middle on a place, at once or sliding there.
    Camera { place: u16, slides: bool },
    /// An arrow over a place, or none.
    Arrow(Option<u16>),
    /// A mark on a place, steady or blinking.
    Mark { place: u16, blinks: bool },
    /// The name of a place written under the map, or nothing.
    Name(Option<u16>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    WorldMap(MapAct),
    Position(Place),
    /// Go through these places, at `speed` pixels per frame; sliding keeps
    /// the character's pose and direction. The routine waits for the end.
    Move {
        speed: f32,
        path: Vec<Place>,
        slide: bool,
    },
    Direction(Look),
    /// Turn step by step, `frames` frames per step; the routine waits.
    Turn {
        frames: u16,
        to: Look,
    },
    /// A number of the game's table of scene animations; the high byte
    /// holds flags.
    Animation(u16),
    /// The character leaves the scene.
    Destroy,
    CloseMessage,
    /// Darken (or whiten) the screen, or bring it back, over `frames`.
    Fade {
        out: bool,
        white: bool,
        frames: u16,
        wait: bool,
    },
    /// Play a song of the game in a loop (by number), or stop the music.
    Music(Option<u16>),
    /// The same on the second channel (what is heard around); and both
    /// channels stopped.
    Ambient(Option<u16>),
    Silence,
    /// The top screen fades, in that many frames; the routine may wait.
    TopFade {
        frames: u16,
        wait: bool,
    },
    /// A channel of music fades away (0 the music, 1 what is heard
    /// around, 2 both) in that many frames.
    MusicFade {
        channel: u8,
        frames: u16,
    },
    /// The view shakes along an axis (1 sideways, 2 up and down) by so
    /// many pixels; 0: it stops.
    Shake {
        axis: u16,
        strength: f32,
    },
    /// A dungeon's name is shown on a dark screen.
    Banner(u16),
    /// Lines go by themselves after that many frames (0 or less: they
    /// wait for a key).
    WaitMode(f32),
    /// The screen goes to a part of its light (out of 256), black or
    /// white, in that many frames.
    Dim {
        white: bool,
        frames: u16,
        to: u16,
        wait: bool,
    },
    /// The characters a layer of a scene put there leave.
    RemoveLayer {
        cast: Cast,
        layer: u16,
    },
    /// A layer of a scene's characters is put in the level, hidden.
    LayerHidden {
        cast: Cast,
        layer: u16,
    },
    /// Play a sound of the game once (by number), or a jingle.
    Sound(u16),
    Jingle(u16),
    /// The camera follows the routine's character.
    CameraFollow,
    CameraAt(f32, f32),
    /// The top screen's map is seen around this point of it.
    TopCameraAt(f32, f32),
    /// Put an effect on the character (a mark over its head, sweat
    /// drops...), by its number in the game's table; 0 removes it.
    Effect(u16),
    /// Rise or come down to this height over the ground, in pixels, at
    /// so many pixels a frame.
    Height {
        speed: f32,
        height: f32,
    },
    /// How the character is drawn: bits to set and bits to clear (hidden,
    /// seen through, under or over the others...), then maybe an
    /// animation to take.
    Attributes {
        set: u16,
        reset: u16,
        animation: Option<u16>,
    },
    /// For how many frames a blinking character shows, then hides.
    Blink(u16, u16),
    /// The camera goes back to the leader.
    CameraDefault,
    /// Show the level's map.
    Ground(u16),
    /// Show another level's map in place of this one's picture: who and
    /// what is there stays.
    Backdrop(u16),
    /// Show a level's map on the top screen, or nothing there any more.
    TopGround(Option<u16>),
    /// The station (free-roaming scene) later operations take layers from.
    LoadStation {
        level: u16,
        name: String,
    },
    /// Put a layer of a scene's characters in the level.
    Layer {
        cast: Cast,
        layer: u16,
    },
    /// Remove the characters the acting scene brought.
    RemoveActing,
    /// Play another scene of a level: a cutscene (`station` false), or
    /// free roaming. A `sub` scene returns to this one when it ends.
    Scene {
        level: u16,
        name: String,
        station: bool,
        sub: bool,
    },
}

/// The scenes characters come from: the cutscene being played, the station
/// loaded, and the level's own scene (its `enter` files).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cast {
    Acting,
    Station,
    Common,
}

/// What a routine can wait for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cond {
    /// Another routine ending.
    Routine(Who),
    Fade,
    /// Someone unlocking this lock.
    Lock(u16),
    /// A routine waiting at this lock.
    Locked(Who, u16),
    /// The character's animation reaching its end.
    Animation,
    /// The effect put on the character ending.
    Effect,
    /// A sound, or a jingle, ending; the top screen's fade ending.
    Sound(u16),
    Jingle(u16),
    TopFade,
    /// The music (or, `true`, what is heard around) fading away.
    Music(bool),
}

/// The speaker of a spoken line no character was named for: a voice from
/// somewhere.
pub(crate) const NOBODY: u16 = u16::MAX;

/// Who says a line, and the portrait shown with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Face {
    /// An entity of the script engine.
    pub entity: u16,
    /// The emotion of the portrait; `None` for no portrait.
    pub face: Option<u16>,
    /// Which of the game's portrait places is used.
    pub mode: u16,
    /// Whether the line starts with the speaker's name (spoken lines do,
    /// thoughts do not).
    pub named: bool,
}

/// What a script asks of the game.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A line of dialogue.
    Message {
        speaker: Option<Face>,
        text: String,
    },
    /// A choice; answer with [`Vm::choose`], then run again.
    Menu {
        options: Vec<String>,
    },
    /// One of the game's own menus (the town's services, the personality
    /// test), by number; the routine goes on once it is closed.
    GameMenu(u16),
    /// Let this many frames pass.
    Wait(u16),
    Act(Act),
    /// The operation that follows is for someone else, who runs it on its
    /// own (with this `Vm`) while this routine goes on.
    Lives(Who, Vm),
    WaitFor(Cond),
    Unlock(u16),
    /// Leave for a level (index into the level table).
    EnterGround(u16),
    EnterDungeon(u16),
    /// The routine is over.
    Done,
}
