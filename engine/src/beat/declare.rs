//! One declaration per command: its names (the engine's own, others it is
//! read by) and, for a regular one, the fields of its beat
//! in the order the line has them, each by how it is read and written (a
//! [`Field`]). From that one table, each kind of beat gets its words
//! (`WORDS`, which make the list of commands), how
//! its regular lines are read (`declared`) and how their beats are written
//! back (`write_declared`): a line written is read back the same by
//! construction. A command with several forms or a reading of its own is
//! declared `hand:` (its names only), read and written by hand beside.
//!
//! ```text
//! declare! {
//!     BodyBeat;
//!     hand: "anim", "pose";
//!     "jump" => Jump { who: Who, height: f32 = 10.0, frames: Frames = Frames(12.0) },
//!     "gone" => Gone(who: Who),
//!     "noemotes" => NoEmotes {},
//! }
//! ```
//!
//! A field is `name: Codec` (it must be there and read), `name: Codec =
//! value` (the value if it is not there or does not read), or
//! `name: IfAbsent<Codec> = value` (the value only if it is not there).

use std::marker::PhantomData;

use super::parse::Fields;
use super::*;

/// A command's names: the engine's own (the one it is written by), and
/// others it is read by too.
pub struct Command {
    pub own: &'static str,
    pub also: &'static [&'static str],
}

/// How a field of a line is read, and written back so that it is read the
/// same.
pub(super) trait Field {
    type Value;
    /// How many fields of the line it takes.
    const WIDTH: usize = 1;
    fn read(fields: &Fields, at: usize) -> Option<Self::Value>;
    /// Read, or that value if it does not read.
    fn read_or(fields: &Fields, at: usize, or: Self::Value) -> Option<Self::Value> {
        Some(Self::read(fields, at).unwrap_or(or))
    }
    /// None if it cannot be written so that it reads back the same.
    fn write(value: &Self::Value, line: &mut Vec<String>, words: &Vocabulary) -> Option<()>;
}

/// A field written as it shows itself.
fn shown(value: &impl std::fmt::Display, line: &mut Vec<String>) -> Option<()> {
    line.push(value.to_string());
    Some(())
}

impl Field for Who {
    type Value = Who;
    fn read(fields: &Fields, at: usize) -> Option<Who> {
        fields.who(at)
    }
    fn write(value: &Who, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

impl Field for f32 {
    type Value = f32;
    fn read(fields: &Fields, at: usize) -> Option<f32> {
        fields.real(at)
    }
    fn write(value: &f32, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

impl Field for u16 {
    type Value = u16;
    fn read(fields: &Fields, at: usize) -> Option<u16> {
        fields.number(at)
    }
    fn write(value: &u16, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

/// A number of frames, as a bare number (`12`, `12.5`).
impl Field for Frames {
    type Value = Frames;
    fn read(fields: &Fields, at: usize) -> Option<Frames> {
        fields.real(at).map(Frames)
    }
    fn write(value: &Frames, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

impl Field for Direction {
    type Value = Direction;
    fn read(fields: &Fields, at: usize) -> Option<Direction> {
        fields.lookup(at, "a direction", Direction::of)
    }
    fn write(value: &Direction, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        line.push(value.name().to_string());
        Some(())
    }
}

/// Two fields, each a number or one found as the scene plays.
impl Field for Point {
    type Value = Point;
    const WIDTH: usize = 2;
    fn read(fields: &Fields, at: usize) -> Option<Point> {
        fields.point(at)
    }
    fn write(value: &Point, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        line.extend([value.x.to_string(), value.y.to_string()]);
        Some(())
    }
}

/// Two numbers, as written.
impl Field for Vec2 {
    type Value = Vec2;
    const WIDTH: usize = 2;
    fn read(fields: &Fields, at: usize) -> Option<Vec2> {
        Some(Vec2::new(fields.real(at)?, fields.real(at + 1)?))
    }
    fn write(value: &Vec2, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        line.extend([value.x.to_string(), value.y.to_string()]);
        Some(())
    }
}

/// A number, or one found as the scene plays (`{name}`).
impl Field for Num {
    type Value = Num;
    fn read(fields: &Fields, at: usize) -> Option<Num> {
        fields.num(at)
    }
    fn write(value: &Num, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

/// The game's ids, by their number.
macro_rules! by_number {
    ($($id:ident),*) => {
        $(impl Field for $id {
            type Value = $id;
            fn read(fields: &Fields, at: usize) -> Option<$id> {
                fields.number(at).map($id)
            }
            fn write(value: &$id, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
                shown(value, line)
            }
        })*
    };
}

by_number!(MoveId, EmoteId, AnimId, SoundId, SongId);

/// A small number (a channel), as a number cut to its last byte.
impl Field for u8 {
    type Value = u8;
    fn read(fields: &Fields, at: usize) -> Option<u8> {
        fields.number(at).map(|number| number as u8)
    }
    fn write(value: &u8, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

/// A sound of the game's, by its number or its name.
pub(super) struct Sound;

impl Field for Sound {
    type Value = SoundId;
    fn read(fields: &Fields, at: usize) -> Option<SoundId> {
        let words = fields.words;
        match fields.word(at) {
            Some(name) if name.parse::<u16>().is_err() => fields.lookup(at, "a sound", |name| words.sound(name)),
            _ => fields.number(at),
        }
        .map(SoundId)
    }
    fn write(value: &SoundId, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

/// A song of the game's by its number or its name; none for any other
/// word, written `-`.
pub(super) struct Song;

impl Field for Song {
    type Value = Option<SongId>;
    fn read(fields: &Fields, at: usize) -> Option<Option<SongId>> {
        let words = fields.words;
        let song = match fields.word(at) {
            Some(name) if name.parse::<u16>().is_err() => words.song(name),
            _ => fields.number(at),
        };
        Some(song.map(SongId))
    }
    fn write(value: &Option<SongId>, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        line.push(value.map_or("-".to_string(), |song| song.to_string()));
        Some(())
    }
}

/// A portrait's face, by its number or its name.
pub(super) struct Face;

impl Field for Face {
    type Value = u16;
    fn read(fields: &Fields, at: usize) -> Option<u16> {
        let words = fields.words;
        match fields.word(at) {
            Some(name) if name.parse::<u16>().is_err() => fields.lookup(at, "a face", |name| words.face(name)),
            _ => fields.number(at),
        }
    }
    fn write(value: &u16, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

/// One word, as written (one with no space in it is read back the same).
pub(super) struct Text;

impl Field for Text {
    type Value = String;
    fn read(fields: &Fields, at: usize) -> Option<String> {
        fields.named(at, "a word").map(str::to_string)
    }
    fn write(value: &String, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        line.push(value.clone());
        Some(())
    }
}

/// A number or a name of one (see `Value::parse`), if it is read back the
/// same (a name that reads as a number cannot be).
impl Field for Value {
    type Value = Value;
    fn read(fields: &Fields, at: usize) -> Option<Value> {
        fields.value(at)
    }
    fn write(value: &Value, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        let field = match value {
            Value::Number(number) => number.to_string(),
            Value::Var(name) => name.clone(),
        };
        (Value::parse(&field) == *value).then(|| line.push(field))
    }
}

/// A number that is no less than 0 (a size, a depth).
pub(super) struct Free;

impl Field for Free {
    type Value = f32;
    fn read(fields: &Fields, at: usize) -> Option<f32> {
        fields.real(at).filter(|value| *value >= 0.0)
    }
    fn write(value: &f32, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        shown(value, line)
    }
}

/// `on` or `off` (`yes`/`no`, `1`/`0`).
pub(super) struct OnOff;

impl Field for OnOff {
    type Value = bool;
    fn read(fields: &Fields, at: usize) -> Option<bool> {
        fields.on_off(at)
    }
    fn write(value: &bool, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        line.push(if *value { "on" } else { "off" }.to_string());
        Some(())
    }
}

/// `1` for yes, anything else (or nothing) for no; written `1`, or nothing
/// for no (last on its line).
pub(super) struct Flag;

impl Field for Flag {
    type Value = bool;
    fn read(fields: &Fields, at: usize) -> Option<bool> {
        Some(fields.flag(at))
    }
    fn write(value: &bool, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        if *value {
            line.push("1".to_string());
        }
        Some(())
    }
}

/// `1` for yes, anything else (or nothing) for no; written `1` or `0`.
pub(super) struct Bit;

impl Field for Bit {
    type Value = bool;
    fn read(fields: &Fields, at: usize) -> Option<bool> {
        Some(fields.flag(at))
    }
    fn write(value: &bool, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
        line.push(u8::from(*value).to_string());
        Some(())
    }
}

/// Maybe one, if it reads (last on its line: nothing is written for none).
pub(super) struct Opt<C>(PhantomData<C>);

impl<C: Field> Field for Opt<C> {
    type Value = Option<C::Value>;
    const WIDTH: usize = C::WIDTH;
    fn read(fields: &Fields, at: usize) -> Option<Self::Value> {
        Some(C::read(fields, at))
    }
    fn write(value: &Self::Value, line: &mut Vec<String>, words: &Vocabulary) -> Option<()> {
        value.as_ref().map_or(Some(()), |value| C::write(value, line, words))
    }
}

/// One, or none written `-`.
pub(super) struct Dash<C>(PhantomData<C>);

impl<C: Field> Field for Dash<C> {
    type Value = Option<C::Value>;
    fn read(fields: &Fields, at: usize) -> Option<Self::Value> {
        match fields.word(at) {
            Some("-") => Some(None),
            _ => Some(Some(C::read(fields, at)?)),
        }
    }
    fn write(value: &Self::Value, line: &mut Vec<String>, words: &Vocabulary) -> Option<()> {
        match value {
            Some(value) => C::write(value, line, words),
            None => {
                line.push("-".to_string());
                Some(())
            }
        }
    }
}

/// One, or none written `off` (or `-`).
pub(super) struct OrOff<C>(PhantomData<C>);

impl<C: Field> Field for OrOff<C> {
    type Value = Option<C::Value>;
    fn read(fields: &Fields, at: usize) -> Option<Self::Value> {
        match fields.word(at) {
            Some("off" | "-") => Some(None),
            _ => Some(Some(C::read(fields, at)?)),
        }
    }
    fn write(value: &Self::Value, line: &mut Vec<String>, words: &Vocabulary) -> Option<()> {
        match value {
            Some(value) => C::write(value, line, words),
            None => {
                line.push("off".to_string());
                Some(())
            }
        }
    }
}

/// As its codec, but its default only if it is not there: one there that
/// does not read is no line.
pub(super) struct IfAbsent<C>(PhantomData<C>);

impl<C: Field> Field for IfAbsent<C> {
    type Value = C::Value;
    const WIDTH: usize = C::WIDTH;
    fn read(fields: &Fields, at: usize) -> Option<C::Value> {
        C::read(fields, at)
    }
    fn read_or(fields: &Fields, at: usize, or: C::Value) -> Option<C::Value> {
        match fields.has(at) {
            false => Some(or),
            true => C::read(fields, at),
        }
    }
    fn write(value: &C::Value, line: &mut Vec<String>, words: &Vocabulary) -> Option<()> {
        C::write(value, line, words)
    }
}

/// A word that is there or not (yes or no), by the words it is read by: the
/// first one written for yes, nothing for no (last on its line).
macro_rules! word_flag {
    ($(#[$doc:meta])* $name:ident: $written:literal $(| $also:literal)*) => {
        $(#[$doc])*
        pub(super) struct $name;

        impl Field for $name {
            type Value = bool;
            fn read(fields: &Fields, at: usize) -> Option<bool> {
                Some(fields.is(at, $written) $(|| fields.is(at, $also))*)
            }
            fn write(value: &bool, line: &mut Vec<String>, _: &Vocabulary) -> Option<()> {
                if *value {
                    line.push($written.to_string());
                }
                Some(())
            }
        }
    };
}

word_flag!(
    /// `sand`: in sand.
    Sand: "sand"
);
word_flag!(
    /// `keep`: kept too.
    Keeps: "keep"
);
word_flag!(
    /// `rotate`: turning round.
    Rotates: "rotate"
);

/// A field of a regular command, read at its place (see `declare!`).
macro_rules! field {
    ($fields:ident, $at:expr, $codec:ty) => {
        <$codec as $crate::beat::declare::Field>::read($fields, $at)
    };
    ($fields:ident, $at:expr, $codec:ty, $or:expr) => {
        <$codec as $crate::beat::declare::Field>::read_or($fields, $at, $or)
    };
}

/// A regular command's beat, read from its fields (see `declare!`).
macro_rules! read_one {
    ($kind:ident, $fields:ident, $variant:ident { $($name:ident : $codec:ty $(= $or:expr)?),* $(,)? }) => {{
        let mut _at = 1usize;
        $(let $name = {
            let here = _at;
            _at += <$codec as $crate::beat::declare::Field>::WIDTH;
            $crate::beat::declare::field!($fields, here, $codec $(, $or)?)
        };)*
        Some($kind::$variant { $($name: $name?),* })
    }};
    ($kind:ident, $fields:ident, $variant:ident ( $($name:ident : $codec:ty $(= $or:expr)?),* $(,)? )) => {{
        let mut _at = 1usize;
        $(let $name = {
            let here = _at;
            _at += <$codec as $crate::beat::declare::Field>::WIDTH;
            $crate::beat::declare::field!($fields, here, $codec $(, $or)?)
        };)*
        Some($kind::$variant($($name?),*))
    }};
}

/// A regular command's beat, written as its line (see `declare!`).
macro_rules! write_one {
    ($this:ident, $words:ident, $own:literal, $kind:ident, $variant:ident { $($name:ident : $codec:ty $(= $or:expr)?),* $(,)? }) => {
        if let $kind::$variant { $($name),* } = $this {
            #[allow(unused_mut)]
            let mut line = vec![$own.to_string()];
            // (Each field in turn, the first that cannot be written stopping it all.)
            #[allow(clippy::redundant_closure_call)]
            let written = (|| {
                $(<$codec as $crate::beat::declare::Field>::write($name, &mut line, $words)?;)*
                Some(())
            })();
            return Some(written.map(|()| line));
        }
    };
    ($this:ident, $words:ident, $own:literal, $kind:ident, $variant:ident ( $($name:ident : $codec:ty $(= $or:expr)?),* $(,)? )) => {
        if let $kind::$variant($($name),*) = $this {
            #[allow(unused_mut)]
            let mut line = vec![$own.to_string()];
            // (Each field in turn, the first that cannot be written stopping it all.)
            #[allow(clippy::redundant_closure_call)]
            let written = (|| {
                $(<$codec as $crate::beat::declare::Field>::write($name, &mut line, $words)?;)*
                Some(())
            })();
            return Some(written.map(|()| line));
        }
    };
}

/// The commands of a kind of beat, each declared once (see the module).
macro_rules! declare {
    (
        $kind:ident;
        hand: $($hand:literal $(| $hand_also:literal)*),* ;
        $($own:literal $(| $also:literal)* => $variant:ident $shape:tt),* $(,)?
    ) => {
        /// The commands of this kind: their names.
        pub(in crate::beat) const WORDS: &[$crate::beat::declare::Command] = &[
            $($crate::beat::declare::Command { own: $hand, also: &[$($hand_also),*] },)*
            $($crate::beat::declare::Command { own: $own, also: &[$($also),*] },)*
        ];

        impl $kind {
            /// A regular command's beat, read from its line (none for another).
            #[allow(unused_variables, dead_code)]
            pub(in crate::beat) fn declared(fields: &$crate::beat::parse::Fields) -> Option<$kind> {
                let word = fields.verb()?;
                $(if word == $own $(|| word == $also)* {
                    return $crate::beat::declare::read_one!($kind, fields, $variant $shape);
                })*
                None
            }

            /// A regular command's line (none for a beat of another; some
            /// none for one that cannot be written to read back the same).
            #[allow(unused_variables, dead_code)]
            pub(in crate::beat) fn write_declared(&self, words: &$crate::vocabulary::Vocabulary) -> Option<Option<Vec<String>>> {
                let this = self;
                $($crate::beat::declare::write_one!(this, words, $own, $kind, $variant $shape);)*
                None
            }
        }
    };
}

pub(crate) use {declare, field, read_one, write_one};

/// Every kind's commands.
pub(super) fn every_kind() -> impl Iterator<Item = &'static Command> {
    [
        body::WORDS,
        motion::WORDS,
        view::WORDS,
        screen::WORDS,
        sound::WORDS,
        speech::WORDS,
        flow::WORDS,
        data::WORDS,
        play::WORDS,
        place::WORDS,
        node::WORDS,
        parse::WORDS,
    ]
    .into_iter()
    .flatten()
}
