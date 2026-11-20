//! A film put together to be sent: a folder with the player, the film
//! whole, and only the game's files it shows, in their layers (see
//! `shell::data`).

use scenes::vocabulary::POKEMON;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use engine::Who;

use preview::art::Art;
use shell::data::film_files::{FILES, IF_ANY, OF_MONSTERS, OF_PLACES, WHOLE};

/// The layers of the game's files: where each is, where it goes in the packet.
type Layers = Vec<(PathBuf, PathBuf)>;

/// A file copied, its folder made.
fn copy(from: &Path, to: &Path) -> Result<u64, String> {
    if let Some(folder) = to.parent() {
        std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    std::fs::copy(from, to).map_err(|error| format!("{}: {error}", from.display()))
}

/// A file, as each layer has it; how many bytes, if any has.
fn every_copy(layers: &Layers, file: &str) -> Result<Option<u64>, String> {
    let mut size = None;
    for (from, to) in layers.iter().filter(|(from, _)| from.join(file).exists()) {
        size = Some(size.unwrap_or(0) + copy(&from.join(file), &to.join(file))?);
    }
    Ok(size)
}

/// The files of a folder, in each layer that has it, those whose name
/// `wanted` accepts.
fn folder_of(layers: &Layers, folder: &str, wanted: impl Fn(&str) -> bool) -> Result<u64, String> {
    let mut size = 0;
    for (from, to) in layers {
        let Ok(listed) = std::fs::read_dir(from.join(folder)) else {
            continue;
        };
        for file in listed.flatten().filter(|file| wanted(&file.file_name().to_string_lossy())) {
            size += copy(&file.path(), &to.join(folder).join(file.file_name()))?;
        }
    }
    Ok(size)
}

/// Whether a file's name starts with one of the numbers given.
fn numbered(name: &str, numbers: &BTreeSet<u16>) -> bool {
    name.split(['_', '.']).next().and_then(|number| number.parse().ok()).is_some_and(|number| numbers.contains(&number))
}

/// The film of that piece in the folder `out`, ready to send.
pub fn pack(piece: &crate::commands::Piece, data: &str, out: &str, art: &Art) -> Result<(), String> {
    let path = piece.path.as_str();
    let (lines, problems) = ::film::lines_of(Path::new(path), &POKEMON, piece.tables.clone())?;
    if let Some(problem) = problems.iter().find(|problem| !problem.is_remark()) {
        return Err(format!("{path}: {problem}"));
    }
    let (mut monsters, mut things, mut levels) =
        (BTreeSet::from([art.cast.who(Who::Character(0)).0, art.cast.who(Who::Character(10)).0]), BTreeSet::new(), BTreeSet::new());
    let mut whole = String::new();
    let mut drawings = BTreeSet::new();
    // (Our own sounds a line names: `own:<name>`.)
    let mut sounds = BTreeSet::new();
    // (A picture from elsewhere on this machine, a face of the Pokédex: in the
    // packet, beside the film, which names it from there.)
    let mut pictures: Vec<(PathBuf, String)> = Vec::new();
    for (_, fields) in &lines {
        let mut written = fields.clone();
        if fields[0] == "image"
            && let Some(file) = fields.get(2).map(Path::new).filter(|file| file.is_absolute() && file.exists())
        {
            let known = pictures.iter().find(|(from, _)| from == file).map(|(_, name)| name.clone());
            written[2] = known.unwrap_or_else(|| {
                let name = format!("images/{}_{}", pictures.len(), file.file_name().map_or("image.png".into(), |name| name.to_string_lossy()));
                pictures.push((file.to_path_buf(), name.clone()));
                name
            });
        }
        whole.push_str(&written.join("\t"));
        whole.push('\n');
        if matches!(fields[0].as_str(), "scene" | "backdrop" | "crossfade" | "top")
            && let Some(level) = fields.get(1).and_then(|level| level.parse().ok())
        {
            levels.insert(level);
        }
        sounds.extend(fields.iter().filter_map(|field| field.strip_prefix("own:")).map(str::to_string));
        for (at, field) in fields.iter().enumerate().skip(1) {
            // (A plain number is someone only where someone is written.)
            let written = at == 1 || !field.starts_with(|first: char| first.is_ascii_digit());
            match Who::parse(field).filter(|_| written) {
                Some(who @ (Who::Character(_) | Who::Monster(_) | Who::Another { .. })) => _ = monsters.insert(art.cast.who(who).0),
                Some(Who::Thing { object, .. }) => _ = things.insert(object),
                // (A picture of ours goes in the hand layer of the packet.)
                Some(Who::Drawn { .. }) if fields[0] == "drawing" && at == 1 => _ = drawings.insert(fields[2].clone()),
                _ => {}
            }
        }
    }
    let layers: Layers = shell::data::layers(data).into_iter().map(|(from, name)| (from, Path::new(out).join("data").join(name))).collect();
    let out = Path::new(out);
    std::fs::create_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    std::fs::write(out.join("film.txt"), whole).map_err(|error| error.to_string())?;
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut size = copy(&exe, &out.join("scene_player.exe"))?;
    for file in FILES {
        size += every_copy(&layers, file)?.ok_or(format!("{data}/{file}: not found"))?;
    }
    for file in IF_ANY {
        size += every_copy(&layers, file)?.unwrap_or(0);
    }
    for folder in WHOLE {
        size += folder_of(&layers, folder, |_| true)?;
    }
    for folder in OF_MONSTERS {
        size += folder_of(&layers, folder, |name| numbered(name, &monsters))?;
    }
    size += folder_of(&layers, "props", |name| numbered(name, &things))?;
    for file in drawings {
        let from = shell::art::ours(data, &piece.tables.project, &file);
        size += copy(&from, &out.join("data/hand").join(&file))?;
        // (What it shows when it is read, if it has one.)
        let large = from.with_file_name(file.replace(".png", "_grand.png"));
        if large.exists() {
            size += copy(&large, &out.join("data/hand").join(file.replace(".png", "_grand.png")))?;
        }
    }
    for name in sounds {
        let from = shell::own_sounds::path(data, &piece.tables.project, &name);
        size += copy(&from, &out.join("data/hand/sounds").join(format!("{name}.wav")))?;
    }
    for folder in OF_PLACES {
        size += folder_of(&layers, folder, |name| numbered(name, &levels))?;
    }
    for (from, name) in &pictures {
        size += copy(from, &out.join(name))?;
    }
    println!("{}: {} Pokémon, {} places, {} MB", out.display(), monsters.len(), levels.len(), size / 1_000_000);
    Ok(())
}
