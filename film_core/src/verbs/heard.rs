//! What is heard, at the moment it is meant to be: the game's sounds, a
//! move's, our own files (`library/sounds`), played as said (from a
//! side, louder, higher, looped, by a name), ambients beside each other,
//! and each stopped or faded by its name.

use super::{Line, Names, Word};
use crate::film::Film;
use crate::words::{digits, engine_word, heard_word, plain};
use engine::{Beat, Frames, MoveId, Played, Side, SongId, SoundBeat, SoundId, SoundOf};
use scenes::vocabulary::POKEMON;

use crate::emit;

pub const WORDS: &[Word] = &[
    Word::new(
        "sound",
        &["name:word off", "name:thing... at d:duration", "name:thing... on who:pokemon", "name:thing..."],
        "sound, movesound, playsound; after (at 12), onhit (on a); [left|right|pan n] [from who] [volume n] [pitch n] [loop] [as name] [in d]",
    )
    .names(Names::NoOne)
    .maybe(sound_said),
    Word::new(
        "music",
        &[
            "off [fade d:duration]",
            "name:word fade d:duration [volume v:number]",
            "name:word volume v:number",
            "name:word from d:duration",
            "name:word [d:duration]",
        ],
        "music, musicfrom (from 12s), fademusic (off fade 2s), a song coming in (fade 2s), its volume; a sound of ours: music own:",
    )
    .names(Names::NoOne)
    .maybe(music_from),
    Word::new("jingle", &["name:word"], "jingle: a little tune of the game by its name (item, treasure, reward, place, episode, news) or number")
        .names(Names::NoOne)
        .maybe(jingled),
];

/// Its words, a family of the language (`Lexicon`).
pub const FAMILY: crate::verbs::Family = crate::verbs::Family::new("heard", WORDS);

/// The game's little tunes by what its scenes play them for (its scripts'
/// numbers): an item received, a treasure obtained, a reward, a new place
/// open, a special episode's title, a message come.
pub const JINGLES: [(&str, u16); 6] = [("reward", 1), ("item", 4), ("episode", 5), ("place", 6), ("news", 7), ("treasure", 9)];

/// `jingle item`, `jingle 4`: a little tune of the game, by its name or number.
fn jingled(film: &mut Film, line: &Line) -> bool {
    let name = plain(line.args.word("name").unwrap_or_default());
    match JINGLES.iter().find(|(word, _)| *word == name).map(|(_, id)| *id).or_else(|| name.parse().ok()) {
        Some(id) => film.beat(emit::jingle(id)),
        None => film.unknown(line.number, "unknown jingle", &name, JINGLES.iter().map(|(word, _)| *word), "item, treasure, reward, place, episode, news"),
    }
    true
}

/// The name the plain ambient has as a sound (`ambient my_wind`).
const AMBIANCE: &str = "ambient";

/// How a line says a sound is played, past its name.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct How {
    pub pan: Option<Side>,
    pub volume: Option<f32>,
    pub pitch: i32,
    pub looped: bool,
    pub name: Option<String>,
    pub rise: Option<f32>,
}

impl How {
    /// Whether it says nothing: the sound as it is.
    pub fn plain(&self) -> bool {
        *self == How::default()
    }

    /// The sound played so.
    pub fn played(&self, of: SoundOf) -> Played {
        Played {
            of,
            volume: self.volume.unwrap_or(1.0),
            pan: self.pan.clone().unwrap_or(Side::Middle),
            pitch: self.pitch,
            looped: self.looped,
            name: self.name.clone(),
            rise: Frames(self.rise.unwrap_or(0.0)),
        }
    }
}

/// What the words after a sound's name say of how it plays, if they all
/// say something: `left`, `right`, `pan -0.5`, `from pikachu`, `volume
/// 0.5`, `pitch -3`, `loop`, `as rain`, `in 2s`.
pub(crate) fn how_read(film: &Film, words: &[String]) -> Option<How> {
    let mut how = How::default();
    let mut at = 0;
    while let Some(word) = words.get(at) {
        let value = words.get(at + 1);
        at += 2;
        match plain(word).as_str() {
            "left" => (how.pan, at) = (Some(Side::At(-1.0)), at - 1),
            "right" => (how.pan, at) = (Some(Side::At(1.0)), at - 1),
            "loop" => (how.looped, at) = (true, at - 1),
            "pan" => how.pan = Some(Side::At(value?.parse::<f32>().ok().filter(|pan| (-1.0..=1.0).contains(pan))?)),
            "from" => how.pan = Some(Side::From { who: film.someone(&plain(value?))?, softer: true }),
            "volume" => how.volume = Some(value?.parse::<f32>().ok().filter(|volume| (0.0..=4.0).contains(volume))?),
            "pitch" => how.pitch = value?.trim_start_matches('+').parse::<i32>().ok().filter(|pitch| pitch.abs() <= 36)?,
            "as" => how.name = Some(plain(value?)).filter(|name| !name.is_empty()),
            "in" => how.rise = Some(emit::frames_read(value?)? as f32),
            _ => return None,
        }
    }
    Some(how)
}

/// A sound's name and how it plays: the first words that leave the rest
/// all saying how (none of them: the whole is its name).
pub(crate) fn name_and_how(film: &Film, words: &[String]) -> (Vec<String>, How) {
    for cut in 1..words.len() {
        if let Some(how) = how_read(film, &words[cut..]) {
            return (words[..cut].to_vec(), how);
        }
    }
    (words.to_vec(), How::default())
}

/// What a name is among the sounds, as a film would hear it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Found {
    /// One of the game's, and whether a file of ours has that name too.
    Game(u16, bool),
    Move(u16, bool),
    Own(String),
    None,
}

/// Which sound a name is: the game's first, then a move's, then one of
/// ours (`own`, their names).
pub(crate) fn found(film: &Film, words: &[String], own: &[String]) -> Found {
    let name = plain(&words.concat());
    let ours = own.contains(&name);
    if words.len() == 1 && (heard_word(&words[0]) || engine_word(&words[0]) || digits(&words[0])) {
        let id = words[0].parse::<u16>().ok().or_else(|| POKEMON.sound(&words[0]));
        match id {
            Some(id) => return Found::Game(id, ours),
            // (A song's name, or the engine's word: no sound of the game.)
            None if !ours => return Found::None,
            None => {}
        }
    }
    if let Some(id) = film.tables.moves.get(&name).and_then(|row| row.0.parse().ok()) {
        return Found::Move(id, ours);
    }
    if ours { Found::Own(name) } else { Found::None }
}

/// The sound a line names, as a beat names it; one not known, or one of
/// ours that the game's hides, told.
pub(crate) fn sound_named(film: &mut Film, words: &[String], number: usize) -> Option<SoundOf> {
    let tables = film.tables.clone();
    let own = &tables.own_sounds;
    let name = plain(&words.concat());
    let hidden = |film: &mut Film, what: &str| {
        film.remark(
            number,
            format!("“{name}” is {what} and a sound of ours ({}/{name}.wav): the game's comes first; rename the file", crate::tables::OWN_SOUNDS),
        );
    };
    match found(film, words, own) {
        Found::Game(id, ours) => {
            if ours {
                hidden(film, "a sound of the game");
            }
            Some(SoundOf::Game(SoundId(id)))
        }
        Found::Move(id, ours) => {
            if ours {
                hidden(film, "a move");
            }
            Some(SoundOf::Move(MoveId(id)))
        }
        Found::Own(name) => Some(SoundOf::Own(name)),
        Found::None => {
            let tables = film.tables.clone();
            let known = tables.moves.keys().map(String::as_str).chain(own.iter().map(String::as_str));
            film.unknown(number, "unknown sound", &words.join(" "), known, "scene_player sounds");
            None
        }
    }
}

/// A sound as said, as the beat it is: as it is, the game's own beats.
fn sound_beat(of: SoundOf, how: &How) -> SoundBeat {
    match (of, how.plain()) {
        (SoundOf::Game(id), true) => SoundBeat::Sound(id),
        (SoundOf::Move(id), true) => SoundBeat::MoveSound(id),
        (of, _) => SoundBeat::Play(how.played(of)),
    }
}

/// A sound looped is stopped by its name: its own, if it is given none.
fn named_if_looped(how: &mut How, name: &[String]) {
    if how.looped && how.name.is_none() {
        how.name = Some(plain(&name.concat()));
    }
}

/// `sound Bite at 12`: 12 frames into what follows; `sound Bite on a`: as
/// the next blow lands on a, from his side; `sound Bite`: a move's sound;
/// `sound my_noise`: one of ours; and how it plays (`left`, `from a`,
/// `volume 0.5`, `pitch 3`, `loop`, `as name`, `in 2s`). The game's
/// own sounds, said as they are, are the engine's.
fn sound_said(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    let number = line.number;
    // (`sound rain off`: the sound looped by that name stops.)
    if args.has("off") && args.words("name").len() == 1 {
        film.beat(fade(&name_said(line), 0.0, 0.0, true));
        return true;
    }
    let (name, mut how) = name_and_how(film, args.words("name"));
    let when_said = args.has("at") || args.has("on");
    let tables = film.tables.clone();
    let own = &tables.own_sounds;
    let kind = found(film, &name, own);
    // (`sound flash`, `sound flash pikachu`: the engine's, as they always were.)
    if !when_said && how.plain() && !matches!(kind, Found::Move(..) | Found::Own(_)) {
        if let Found::Game(_, true) = kind {
            sound_named(film, &name, number);
        }
        return false;
    }
    let Some(of) = sound_named(film, &name, number) else {
        return true;
    };
    named_if_looped(&mut how, &name);
    let sound = sound_beat(of, &how);
    if args.has("at") {
        // (`at 1s~3s`, a while drawn: not read.)
        let time = args.word("d").unwrap_or_default();
        match emit::frames_read(time).filter(|_| !time.contains('~')) {
            Some(frames) => film.beat(emit::after(frames as f32, 0.0, Beat::Sound(sound))),
            None => film.unread(format!("after {time}")),
        }
    } else if args.has("on") {
        let who = plain(args.word("who").unwrap_or_default());
        film.on(&who, |who| emit::on_hit(who, sound.clone()));
    } else {
        film.beat(Beat::Sound(sound));
    }
    true
}

/// `wait sound flash`, `wait sound Bite`, `wait sound my_noise
/// pitch 3`: the sound, and the film waits as long as it lasts. A loop
/// has no end: refused.
pub(crate) fn waited_sound(film: &mut Film, line: &Line) {
    let (name, mut how) = name_and_how(film, line.args.words("name"));
    if how.looped {
        let said = line.args.words("name").join(" ");
        film.wrong(line.number, format!("“wait sound {said}”: a loop never ends; “sound {said}”, “wait 3s”, then “cease sound …”"));
        return;
    }
    let Some(of) = sound_named(film, &name, line.number) else {
        return;
    };
    named_if_looped(&mut how, &name);
    film.beat(Beat::Sound(sound_beat(of.clone(), &how)));
    film.beat(emit::wait_sound(of, how.pitch));
}

/// `music peace from 12s`: from that far into it; `music my_song
/// [3s]`: one of ours; else, the engine's.
fn music_from(film: &mut Film, line: &Line) -> bool {
    let args = &line.args;
    film.timing.music = true;
    // (`music off`, `music off fade 2s`: the music stops, or dies away.)
    if args.has("off") {
        match args.frames("d") {
            Some(frames) => film.beat(emit::music_fade(0, frames as f32)),
            None => film.beat(emit::music(None)),
        }
        return true;
    }
    let song = plain(&args.words("name")[0]);
    // (`music peace fade 2s volume 0.5`: the song comes in softly, so loud.)
    if args.has("fade") || args.has("volume") {
        let game = song.parse::<u16>().ok().or_else(|| POKEMON.song(&song));
        let Some(id) = game else {
            film.not_a(&song, "a song of the game (scene_player songs)");
            return true;
        };
        match args.frames("d") {
            Some(frames) => film.beat(Beat::Sound(SoundBeat::MusicIn { song: Some(SongId(id)), frames: Frames(frames as f32) })),
            None => film.beat(emit::music(Some(SongId(id)))),
        }
        if let Some(volume) = args.number("v") {
            film.beat(Beat::Sound(SoundBeat::Volume(volume.clamp(0.0, 2.0))));
        }
        return true;
    }
    let game = song.parse::<u16>().ok().or_else(|| POKEMON.song(&song));
    let ours = film.tables.own_sounds.contains(&song);
    if game.is_some() && ours {
        film.remark(line.number, format!("“{song}” is a music of the game and a sound of ours: the game's comes first; rename the file"));
    }
    let time = args.word("d").unwrap_or_default();
    if game.is_none() && ours {
        let frames = if args.has("d") { emit::frames_read(time) } else { Some(0) };
        let (frames, seconds) = match (frames, args.has("from")) {
            (Some(frames), true) => (0.0, frames as f32 / 60.0),
            (Some(frames), false) => (frames as f32, 0.0),
            (None, _) => {
                film.unread(format!("{song} {time}"));
                return true;
            }
        };
        film.beat(Beat::Sound(SoundBeat::MusicOwn { name: song, frames: Frames(frames), seconds }));
        return true;
    }
    // (`music peace`, `music peace 1s`: the song, or coming in softly over
    // that time; `music back`: the one before; `music own:<name>`: ours.)
    if !args.has("from") {
        let said = args.words("name")[0].as_str();
        let frames = if args.has("d") { emit::frames_read(time) } else { Some(0) };
        let Some(frames) = frames else {
            film.not_a(time, "a duration");
            return true;
        };
        let beat = match (said.strip_prefix("own:"), game) {
            (Some(name), _) if !name.is_empty() => SoundBeat::MusicOwn { name: name.to_string(), frames: Frames(frames as f32), seconds: 0.0 },
            _ if song == "back" => SoundBeat::MusicBack,
            // (A name the game has no song by: none, the music stopping, as
            // the engine's `music` has always read it.)
            (_, song) if args.has("d") => SoundBeat::MusicIn { song: song.map(SongId), frames: Frames(frames as f32) },
            (_, song) => SoundBeat::Music(song.map(SongId)),
        };
        film.beat(Beat::Sound(beat));
        return true;
    }
    match (game, emit::frames_read(time)) {
        (Some(id), Some(frames)) => film.beat(emit::music_from(Some(SongId(id)), frames as f32 / 60.0)),
        _ => film.unread(format!("{song} {time}")),
    }
    true
}

/// A sound by its name to that level over so many frames.
fn fade(name: &str, level: f32, frames: f32, stop: bool) -> Beat {
    Beat::Sound(SoundBeat::Fade { name: name.to_string(), level, frames: Frames(frames), stop })
}

/// The name a line gives a sound or an ambient (`ambient` alone: the
/// plain one).
fn name_said(line: &Line) -> String {
    line.args.word("name").map_or(AMBIANCE.to_string(), plain)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn how_a_sound_plays_is_read_past_its_name() {
        let film = Film::with(&crate::verbs::lexicon::BARE, Default::default());
        let (name, how) = name_and_how(&film, &words("soft wind left volume 0.5 pitch -3 loop as wind in 2s"));
        assert_eq!(name, words("soft wind"));
        assert_eq!(how.pan, Some(Side::At(-1.0)));
        assert_eq!((how.volume, how.pitch, how.looped, how.name.as_deref(), how.rise), (Some(0.5), -3, true, Some("wind"), Some(120.0)));
        // (Words that say nothing of it: all its name.)
        let (name, how) = name_and_how(&film, &words("Zen Headbutt"));
        assert_eq!((name.len(), how.plain()), (2, true));
        assert!(how_read(&film, &words("pan 3")).is_none());
        assert!(how_read(&film, &words("volume")).is_none());
    }

    #[test]
    fn a_name_is_the_games_sound_first_then_a_moves_then_ours() {
        let film = Film::with(&crate::verbs::lexicon::BARE, Default::default());
        let own = vec!["flash".to_string(), "my_noise".to_string()];
        let game = POKEMON.sound("flash").expect("a sound of the game");
        assert_eq!(found(&film, &words("flash"), &own), Found::Game(game, true));
        assert_eq!(found(&film, &words("my_noise"), &own), Found::Own("my_noise".to_string()));
        assert_eq!(found(&film, &words("my_noise"), &[]), Found::None);
    }

    #[test]
    fn a_looped_sound_is_named_by_its_own_name() {
        let mut how = How { looped: true, ..How::default() };
        named_if_looped(&mut how, &words("soft_wind"));
        assert_eq!(how.name.as_deref(), Some("soft_wind"));
        let played = how.played(SoundOf::Own("soft_wind".to_string()));
        assert!(played.looped && played.pan == Side::Middle && played.volume == 1.0);
    }
}
