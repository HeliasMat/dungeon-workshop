//! Sounds heard from the text: the one a line names, played when it is
//! pointed at.

/// The sound or the music of that line heard (heard already: stopped).
pub(super) fn hear(
    session: &workshop_base::session::Session,
    heard: &mut Option<usize>,
    audio: &mut Option<Box<dyn shell::sound::Hears>>,
    said: &mut Option<String>,
    line: usize,
    music: bool,
    name: &str,
) {
    if audio.is_none() {
        *audio = shell::sound::Audio::open_in(session.data(), session.library().clone()).map(|audio| Box::new(audio) as Box<dyn shell::sound::Hears>);
    }
    let Some(audio) = audio.as_mut() else {
        *said = Some("no sound: the sound card or the game's sounds are missing".to_string());
        return;
    };
    audio.song(0, None, 0.0, 0.0);
    audio.stop_held();
    if heard.take() == Some(line) {
        return;
    }
    let song = rules::songs::SONGS.iter().find(|song| song.word == name);
    let game = scenes::vocabulary::POKEMON.sound(name);
    if music && let Some(song) = song {
        audio.song_from(0, Some(song.number), 0.8, 0.0);
        audio.volume(0, 0.8);
    } else if let Some(stereo) = shell::own_sounds::read(session.data(), session.library(), name) {
        audio.play_held(stereo, 0.8);
    } else if !music && let Some(id) = game {
        audio.sound(id, 0.8, 1.0);
        return;
    } else {
        *said = Some(format!("\"{name}\": neither a {} of the game's nor one of ours (library/sounds)", if music { "music" } else { "sound" }));
        return;
    }
    *heard = Some(line);
}

/// What a line makes heard: `sound x` or `music x` (true: a music), and its name.
pub(super) fn heard_on(line: &str) -> Option<(bool, String)> {
    let mut words = line.split('#').next().unwrap_or_default().split_whitespace();
    let music = match words.next()? {
        "sound" => false,
        "music" => true,
        _ => return None,
    };
    let name = words.next()?;
    (!matches!(name, "off" | "stop" | "end" | "from")).then(|| (music, film::words::plain(name)))
}
