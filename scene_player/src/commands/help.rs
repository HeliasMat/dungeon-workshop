//! The one help: every command, what it takes, what it says.

use ::film::check::closest;

/// A command and how it reads.
pub struct Command {
    pub word: &'static str,
    /// What it takes, as help shows it.
    pub takes: &'static str,
    /// How many words it must have.
    pub least: usize,
    /// Its options, with no value and with one.
    pub flags: &'static [&'static str],
    pub values: &'static [&'static str],
    /// What it does, in a line; what it writes or tells, and an example.
    pub says: &'static str,
    pub more: &'static str,
}

const fn command(word: &'static str, takes: &'static str, least: usize, says: &'static str, more: &'static str) -> Command {
    Command { word, takes, least, flags: &[], values: &[], says, more }
}

pub const COMMANDS: [Command; 38] = [
    command("search", "<words>", 1, "Pokémon and places", "scene_player search krabby beach"),
    command("place", "<place>", 1, "a place: its size, its spots", "scene_player place beach"),
    command("illustrations", "[category]", 0, "the game's illustrations, by category (for scene: <image>)", "scene_player illustrations sea"),
    command(
        "dungeons",
        "",
        0,
        "prepares the dungeons for the films: the tileset of each floor, the stairs and the traps as images",
        "Writes data/extra/dungeon_floors.txt (for decor, its dungeon rooms) and data/hand/stairs.png, trap_<word>.png.
scene_player dungeons",
    ),
    Command {
        flags: &["--replace"],
        ..command(
            "spritecollab",
            "<all|update>",
            1,
            "the PMDCollab community's sprites and faces, all of them: fetched from GitHub (a few GB) and added to data/extra",
            "all: the whole repository cloned into sources/spritecollab (DUNGEON_SPRITECOLLAB: elsewhere), then every species and form imported: the game's Pokémon get the animations and faces they lack, the later ones 2000 + their Pokédex number, the forms variants from 5000 (vulpix_alola). update: only what changed on GitHub since. Cut short, run it again: it goes on where it stopped (data/extra/spritecollab_done.txt). --replace: over the game's own sheets too, every folder again.
CC BY-NC 4.0: non-commercial use, credit given (data/extra/spritecollab_credits.txt).
scene_player spritecollab all",
        )
    },
    command(
        "songs",
        "[word]",
        0,
        "the game's songs by their name: their number, their title, their moods",
        "A word searches the name, the title and the moods (sad, battle, sea, party, mystery, dungeon...); \"ambient\": the lasting sounds.
scene_player songs sad",
    ),
    command(
        "sounds",
        "[word]",
        0,
        "the sound effects by their name, and when the game plays them; ours (library/sounds), with their length",
        "The game names none of them: each is named after what its scenes show with it.\nscene_player sounds tumble",
    ),
    command(
        "listen",
        "<word|number>",
        1,
        "a song or a sound effect of the game, to listen to: target/sounds/<word>.wav",
        "A number: a song (up to 999) or a sound effect (1000 and over, like 8964).\nscene_player listen compassion",
    ),
    command(
        "items",
        "[word]",
        0,
        "the objects for the films: the game's items, the game's props, our own images, and a sheet",
        "No word: all those that have a word of their own. A number: that number (o186).\nWrites target/films/items/sheet.png.\nscene_player items chest",
    ),
    command("grid", "<place>", 1, "a place twice as large, a line every 50 pixels", "Writes target/films/<place>/grid.png."),
    command("ground", "<place>", 1, "a place, in red where no one walks", "Writes target/films/<place>/ground.png."),
    command(
        "steps",
        "<film>",
        1,
        "every step of a film, one after the other: its text, what the language reads, the tree of its lines, its recipes played, its scenes, the words read, the beats written, the engine's events, what is wrong",
        "To see what each line becomes, from the text to the engine.\nscene_player steps example",
    ),
    Command {
        flags: &["--json"],
        ..command(
            "cast",
            "<film>",
            1,
            "who plays in the film: scene by scene, each by their name, their lines and the beats where they act, then the whole film, the most present first",
            "Points out who is on the scene: line or brought in, but says and does nothing (\"present but silent and still\"). --json: for a program.\nscene_player cast example",
        )
    },
    Command {
        flags: &["--json"],
        values: &["--max"],
        ..command(
            "usages",
            "<word>",
            1,
            "every line of the films and the recipes that uses this word, film by film, counted",
            "The word as it is written, capitals and accents alike. --max N: N lines shown (40). None: the closest words.\nscene_player usages jump",
        )
    },
    Command {
        flags: &["--try"],
        ..command(
            "rename",
            "<film> <old> <new>",
            3,
            "renames a character in a film: who speaks, the scene: line, the words that name them, never what is said",
            "Read by the language, not by replacing text: the lines, stories and comments stay. Refuses a name already taken;\na species with no name of its own gets \"new = old\" before the first scene. Nothing is written if the language finds more errors there.\n--try: only says what would change.\nscene_player rename example kecleon merchant --try",
        )
    },
    Command {
        flags: &["--json"],
        values: &["--from"],
        ..command(
            "diff",
            "<film>",
            1,
            "what the engine gets from the film now, against its git version: scene by scene, the beats removed (-) and added (+)",
            "--from <rev>: against this version (HEAD). The beats are written as their line; the film's line numbers are left aside.\nscene_player diff example --from HEAD~3",
        )
    },
    Command {
        flags: &["--json"],
        ..command(
            "analyze",
            "<film>",
            1,
            "what the language says of a film, without playing it: what is wrong (line, column, corrections), then its outline",
            "The outline: scenes, moments, recipes, values, marks, threads. --json: the same for a program (an agent, an editor).\nscene_player analyze example --json",
        )
    },
    Command {
        flags: &["--compare", "--fields"],
        ..command(
            "fingerprint",
            "",
            0,
            "what the engine gets from each film, scene, demo and trial, written in target/fingerprints/ (never committed)",
            "Before a change to the compiler: scene_player fingerprint; after: scene_player fingerprint --compare, which says what changed.\nscene_player fingerprint --compare",
        )
    },
    command(
        "ids",
        "<film> [films...]",
        1,
        "gives each line said with no id one of its own, written last (#k3f9): its name whatever its words become, for translations",
        "The ids there are kept; twice, nothing changes.
scene_player ids library/films/my_film.film",
    ),
    command(
        "format",
        "[files]",
        0,
        "puts films in order: two spaces per block, one blank line at most (no file: all of them)",
        "Like Ctrl+Shift+F in the Workshop; twice, nothing changes.
scene_player format library/films/my_film.film",
    ),
    Command {
        flags: &["--all", "--no-play", "--json"],
        ..command(
            "check",
            "<film> [films...]",
            1,
            "checks the film and plays it at once, without a window: what is wrong",
            "Gives its length and, scene by scene, the remarks, numbered, with their correction: who speaks,\nor who is spoken to, out of the picture; a K.O. who speaks; a body in the scenery or on another;\na walk blocked or through the scenery; a line too fast; who appears or disappears all at once. --all: the details too (a place\nadjusted, a variant, a body cut by the edge, the crowd, what the reading notes). Fails if there are problems.\n--no-play: only reads them again, without playing them (scenes .txt too), faster, without the remarks on the picture.\n--json: one line of JSON (code, line, column, message, closest, correction of each problem; film/FILMS.md \"For agents\").\nscene_player check example\nscene_player check library/trials/*.scene --no-play",
        )
    },
    Command {
        flags: &["--json", "--no-play"],
        values: &["--why"],
        ..command(
            "timing",
            "<film> [moment] [moment]",
            1,
            "the film's time: when each line starts and ends, why, what plays beside it, who moves",
            "No moment: each line, from its start to its end as the engine played it, and why it starts there (\"beside\": a thread beside the film; \"&\": the film does not wait for it).
One moment: what plays then, on the film and beside it, who moves and until when, and the next line. line N: that line, each time it plays.
Two moments: everything that plays between them. --json: for a program.
--why N: the chain that places line N (after which line, which clock, which event), back to the start.
--no-play: the plan without playing the film (\"≥\": after an event, at least).
<moment>: 32s, line 90, scene 3, scene 3 end.
scene_player timing example 40s
scene_player timing example --why 12",
        )
    },
    Command {
        flags: &["--json"],
        ..command(
            "state",
            "<film> <moments>",
            2,
            "this moment as text: who is where, facing, pose, HP, camera, objects",
            "<moment>: 32s, line 90, scene 3, scene 3 end.\n--json: each node and its properties (the registry), its place on the screen (rectangle, visible, covered, edges), the camera, the box, the sounds.\nscene_player state example 12s --json",
        )
    },
    Command {
        flags: &["--json"],
        ..command(
            "properties",
            "[kind]",
            0,
            "the kinds of nodes and their properties, from the registry: type, unit, default, advised, animatable",
            "<kind>: body, thing, image, caption, halo, emitter, group. --json: for a program.\nscene_player properties halo --json",
        )
    },
    Command {
        flags: &["--json"],
        ..command(
            "schema",
            "",
            0,
            "the language for a program: each word and its forms, the engine's commands, the genres, the grammar of values, the codes",
            "scene_player schema --json",
        )
    },
    Command {
        flags: &["--json"],
        ..command(
            "resources",
            "[kind]",
            0,
            "what a film can take, by the name it writes, with its size or its length",
            "<kind>: images, items, game_props, sounds, songs, fonts, pokemon, variants, places, decors (the game's sounds are measured when asked alone).\nscene_player resources images --json",
        )
    },
    command(
        "server",
        "",
        0,
        "stays open for an agent: one JSON request per line, one JSON answer per line",
        "load (film or text), analyze, check, state t, timing, image t path, properties, schema, resources, quit.\nThe film is played once on loading, kept every 5 s: a moment asked starts again from the last one kept.\nfilm/FILMS.md \"For agents\". echo '{\"command\":\"schema\"}' | scene_player server",
    ),
    command(
        "image",
        "<film> <moments>",
        2,
        "these moments, large (seconds: in a single pass)",
        "<moment>: 32s, line 90 (a third of a second after its start), scene 3, scene 3 end.\nWrites target/films/<film>/image_<moment>.png.\nscene_player image example 32s line 90 scene 3 end",
    ),
    Command {
        values: &["--on"],
        ..command(
            "try",
            "<film> [moments]",
            1,
            "does it work? the check, then these moments on a single sheet",
            "No moment: eight, spread over the whole film. Each image carries its second; the line playing is said beside it.\n--on <who>: each image up close, around them (a pose, a detail).\nWrites target/films/<film>/try.png.\nscene_player try example 20s 39s line 49 --on pidgeot",
        )
    },
    Command {
        values: &["--scene", "--every"],
        ..command(
            "view",
            "<film>",
            1,
            "a sheet of images of the whole film, or of a scene",
            "--scene N: that scene; --every S: an image every S seconds (4).\nWrites target/films/<film>/sheet.png.",
        )
    },
    Command {
        flags: &["--auto", "--new", "--headless"],
        values: &["--size", "--from", "--chapter"],
        ..command("play", "<film or game>", 1, "the film, or the game (its name in games/, a .game, or a .dwpack: unpacked in target/packs/), in its window", "--auto: the lines go by themselves; --size 1 to 8; --from <label>. A game (test_village: games/test_village/test_village.game): its title screen, then arrows, Enter (talk), Shift (run), M or X (the menu: team, bag, save); saved in saves/<game>.txt; --new: from the start; --chapter n: a new game from chapter n, at the game's home (the flags before it not set); --headless: no window, nobody at the keys, what happens told in order.")
    },
    command(
        "check-game",
        "<game>",
        1,
        "checks a whole game, without a window: its file, its films, places, dungeons and fights, and what only shows at its scale",
        "A name (my_game: games/<name>/<name>.game, then library/<name>.game) or the path of a .game. Each problem as file:line: message (the nearest
names when one is unknown), then how many. Looks for: names not found (film, place, entrance, dungeon, fight, resident, zone,
item, Pokémon, chapter); flags set and never read, read and never set (the game, the places, the films' keep, if, set);
what the game never reaches (a chapter, a place, a film, a dungeon), a chapter with no way to end, a dungeon with no home:.
Fails if there are problems.
scene_player check-game my_game",
    ),
    Command {
        values: &["--size"],
        ..command("video", "<film>", 1, "the film as a video, with sound (needs ffmpeg)", "Writes target/films/<film>/<film>.mp4.")
    },
    command("pack", "<film>", 1, "a zip in dist/, to keep among friends", "It holds data from the ROM: never published."),
    command(
        "pack-game",
        "<game>",
        1,
        "the game in one file to share: dist/<game>.dwpack",
        "Its own files only (the .game, its places, films, dungeons, fights, recipes, music, sounds, images, tilesets), and a manifest (dwpack.txt).
Nothing of data/: a picture or a sound from the game's data is refused. Whoever plays it reads their own: scene_player play <game>.dwpack.",
    ),
    Command {
        flags: &["--table", "--partial", "--tall", "--squares"],
        values: &["--out"],
        ..command(
            "export-ds",
            "<film>",
            0,
            "the film as a cutscene of the original game, in ExplorerScript (target/ds/<film>.exps)",
            "A trial: tools/ds/patch_rom.py puts it in a copy of your own ROM. Each word that has no DS operation yet is an error.
--table: each core word and the game's operations it becomes, direct, close (what is lost) or impossible (why).
--partial: what doesn't export is left out, and told, instead of an error.
--tall: both screens show the place, the top one a screen above the bottom one (the actors show on the bottom one only).
--squares: a scene whose species' sprites are past the room the DS has for them keeps them all, those past it drawn as magenta squares (else it is split at a moment, or the least needed are left out, and told).
A .dungeon file instead of a film: the dungeon and its boss fight as the rules read them, in JSON, for patch_rom.py --dungeon.
scene_player export-ds my_film --out target/ds/my_film.exps",
        )
    },
    Command { values: &["--size"], ..command("measure", "<film>", 1, "the time to compute and draw each frame", "The mean and the worst.") },
    command("help", "[command]", 0, "this help", "scene_player help check"),
];

/// Whether that is a command.
pub fn known(word: &str) -> bool {
    COMMANDS.iter().any(|command| command.word == word)
}

/// What to say of a word that is no command.
pub fn unknown(word: &str) -> String {
    let close = closest(word, COMMANDS.iter().map(|command| command.word), 2, 0.5);
    let close = close.first().map_or(String::new(), |close| format!(" Did you mean {close}?"));
    format!("neither a command nor a file: {word}.{close} scene_player help")
}

/// All the commands, or one.
pub fn help(word: Option<&str>) -> Result<(), String> {
    let usage = |command: &Command| {
        let options: Vec<String> =
            command.flags.iter().map(|flag| format!(" [{flag}]")).chain(command.values.iter().map(|value| format!(" [{value} ...]"))).collect();
        format!("scene_player {} {}{}", command.word, command.takes, options.concat())
    };
    match word {
        Some(word) => {
            let command = COMMANDS.iter().find(|command| command.word == word).ok_or_else(|| unknown(word))?;
            println!("{}\n{}.\n{}", usage(command), command.says, command.more);
        }
        None => {
            println!("Films in plain English: film/FILMS.md.\n");
            for command in &COMMANDS {
                println!("  {:<46}{}", format!("{} {}", command.word, command.takes), command.says);
            }
            println!("\n<film>: a path, or a name in library/films/ (else in games/<game>/films/). --data <folder>: the game's files.");
            println!("scene_player help <command>: the details.");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_its_help() {
        assert!(COMMANDS.iter().all(|command| help(Some(command.word)).is_ok() && !command.says.is_empty()));
        assert!(help(Some("whatever")).is_err());
        assert!(unknown("chek").contains("check"));
    }
}
