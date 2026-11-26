//! How the Workshop was launched: what its `main` reads, once, of the
//! command line and of the environment, and hands to every space through
//! the session (`Session::launch`). Nothing else in the Workshop reads an
//! environment variable. Most of them open the window on a given state, for
//! a picture of it (`workshop/README.md`, "Launch options").

use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct LaunchOptions {
    /// A file named on the command line: opened in its space.
    pub file: Option<PathBuf>,
    /// `DUNGEON_SPRITECOLLAB`: the clone of SpriteCollab (else `<root>/sources/spritecollab`).
    pub spritecollab_clone: Option<PathBuf>,
    /// `DUNGEON_FILM_LANGUAGE=fr`: the language the films are said in (English without it).
    pub language: String,
    /// `DUNGEON_FILM_DEBUG=view,text`: the developer's options of the films played (`preview::debug`).
    pub film_debug: preview::debug::Debug,
    /// `WORKSHOP_SPACE=game|film|decor|dungeon|resources`: that space first.
    pub space: Option<String>,
    /// `WORKSHOP_SPRITECOLLAB=25`: the SpriteCollab window open on that number.
    pub spritecollab: Option<String>,
    /// `WORKSHOP_FOLDED=Transform,Resident`: those components of the Inspector folded.
    pub folded: Vec<String>,
    /// `WORKSHOP_CHOSEN=n`: line n of the decor open chosen.
    pub chosen: Option<usize>,
    /// `WORKSHOP_TOOL=terrain|scatter|walls|walk`: that brush in hand.
    pub tool: Option<String>,
    /// `WORKSHOP_CAPTURE=<file.png>`: a picture of the window written there
    /// after `WORKSHOP_AFTER` seconds (3), then the Workshop closes.
    pub capture: Option<PathBuf>,
    pub capture_after: Option<u64>,
    /// `WORKSHOP_VIEW`: the decor's view (`zoom x y`), or the Script's view (`story`, `board`…).
    pub view: Option<String>,
    /// `WORKSHOP_TRY`: the dungeon open tried at once.
    pub try_dungeon: bool,
    /// `WORKSHOP_SEED=n`, `WORKSHOP_DRAWN`: the floor shown laid out with that seed, drawn.
    pub seed: Option<u32>,
    pub drawn: bool,
    /// `WORKSHOP_BOSS_HP=n`: the boss tried alone with n hundredths of its life.
    pub boss_hp: Option<i32>,
    /// `WORKSHOP_CREATE=dungeon:my_dungeon,image…`: those open, being made.
    pub create: Option<String>,
    /// `WORKSHOP_NEW=film[:step[:place[:a,b[:start]]]]|decor[:place]`: the New Film window open at that
    /// step (1 to 4), those chosen; or the New Decor window, that place chosen.
    pub new: Option<String>,
    /// `WORKSHOP_STEP=1`: the film played a frame at a time.
    pub step: Option<String>,
    /// `WORKSHOP_INSTANT=s`: the film shown at that second.
    pub instant: Option<f64>,
    /// `WORKSHOP_SCENE=whole,names,grid,who:<name>`: what the Film space's Scene shows (who: that one chosen).
    pub scene: Option<String>,
    /// `WORKSHOP_GAME=my_game`: that game open.
    pub game: Option<String>,
    /// `WORKSHOP_FILE=…`: that file chosen in the Home.
    pub home_file: Option<String>,
    /// `WORKSHOP_MUSIC=1:instrument`: that track, that view of the song.
    pub music: Option<String>,
    /// `WORKSHOP_TILEQUANT=<file.png>`: the image converter open on that picture.
    pub tilequant: Option<String>,
    /// `WORKSHOP_RESOURCE=drawing:bouee`: that shelf, that one chosen.
    pub resource: Option<String>,
    /// `WORKSHOP_PICKER=<tab>[:<pokemon>[:<search>]]`: the Film's Assets panel on that tab
    /// (pokemon, faces, gestures, music, sounds, things, places, effects), for that Pokémon, searching that.
    pub picker: Option<String>,
    /// `WORKSHOP_PROJECT=open|versions|search:<word>|beside:<film>`: that window of the Script open.
    pub project: Option<String>,
    /// `WORKSHOP_FOCUS`: the Script in concentration.
    pub focus: bool,
    /// `WORKSHOP_LINE=n`: the Script's cursor on line n.
    pub line: Option<usize>,
    /// `WORKSHOP_COMPLETE=1`: the cursor at the end of that line, what may be typed there offered.
    pub complete: bool,
    /// `WORKSHOP_PANELS=…`: the Script's panels shown.
    pub panels: Option<String>,
    /// `WORKSHOP_WORDS=<family>`: the words of that family browsed.
    pub words: Option<String>,
    /// `WORKSHOP_WIKI=<word>|<word>:play|<word>:source|search:<words>|all`: the Script's Wiki open on that.
    pub wiki: Option<String>,
    /// `WORKSHOP_LIBRARY=backdrops|search:<words>`: the Decor's Library on that shelf, or searching that.
    pub library: Option<String>,
    /// `WORKSHOP_FIRST`: the first launch's screen, even with the data there.
    pub first: bool,
    /// `WORKSHOP_WELCOME`: the "what to try first" panel, as after the first launch.
    pub welcome: bool,
    /// `WORKSHOP_SETTINGS=<file>`: the Workshop's settings (else `settings.txt` in the
    /// system's folder for them: `%APPDATA%\dungeon_workshop`, `~/.config/dungeon_workshop`).
    pub settings: Option<PathBuf>,
    /// `WORKSHOP_LIBRETRO_CORE=<file>`: the libretro core the Emulator space runs (before the settings' own).
    pub libretro_core: Option<PathBuf>,
    /// `SKYTEMPLE_ARMIPS_EXEC=<armips>`: what `patch_rom.py --more-actors` assembles with (before the settings' own).
    pub armips: Option<PathBuf>,
    /// `WORKSHOP_EMULATOR_ROM=<file.nds>`: that game loaded in the Emulator space at once.
    pub emulator_rom: Option<PathBuf>,
    /// `WORKSHOP_EMULATOR_FRAMES=n`: and run that many frames at once (past a game's logos), for a picture.
    pub emulator_frames: Option<u32>,
    /// `WORKSHOP_EMULATOR_FILM=<file.film>`: that film played on DS at once (Play This Film on DS).
    pub emulator_film: Option<PathBuf>,
}

impl LaunchOptions {
    /// As the command line and the environment say (its `main`, once).
    pub fn read() -> LaunchOptions {
        let var = |name: &str| std::env::var(name).ok();
        fn number<T: std::str::FromStr>(name: &str) -> Option<T> {
            std::env::var(name).ok().and_then(|said| said.trim().parse().ok())
        }
        let set = |name: &str| std::env::var_os(name).is_some();
        LaunchOptions {
            file: std::env::args().nth(1).map(PathBuf::from).filter(|path| path.is_file()),
            spritecollab_clone: std::env::var_os("DUNGEON_SPRITECOLLAB").map(PathBuf::from),
            language: var("DUNGEON_FILM_LANGUAGE").unwrap_or_default(),
            film_debug: preview::debug::Debug::read(&var(preview::debug::VARIABLE).unwrap_or_default()),
            space: var("WORKSHOP_SPACE"),
            spritecollab: var("WORKSHOP_SPRITECOLLAB"),
            folded: var("WORKSHOP_FOLDED").unwrap_or_default().split(',').map(str::trim).filter(|part| !part.is_empty()).map(str::to_string).collect(),
            chosen: number("WORKSHOP_CHOSEN"),
            tool: var("WORKSHOP_TOOL"),
            capture: std::env::var_os("WORKSHOP_CAPTURE").map(PathBuf::from),
            capture_after: number("WORKSHOP_AFTER"),
            view: var("WORKSHOP_VIEW"),
            try_dungeon: set("WORKSHOP_TRY"),
            seed: number("WORKSHOP_SEED"),
            drawn: set("WORKSHOP_DRAWN"),
            boss_hp: number("WORKSHOP_BOSS_HP"),
            create: var("WORKSHOP_CREATE"),
            new: var("WORKSHOP_NEW"),
            step: var("WORKSHOP_STEP"),
            instant: number("WORKSHOP_INSTANT"),
            scene: var("WORKSHOP_SCENE"),
            game: var("WORKSHOP_GAME"),
            home_file: var("WORKSHOP_FILE"),
            music: var("WORKSHOP_MUSIC"),
            tilequant: var("WORKSHOP_TILEQUANT"),
            resource: var("WORKSHOP_RESOURCE"),
            picker: var("WORKSHOP_PICKER"),
            project: var("WORKSHOP_PROJECT"),
            focus: set("WORKSHOP_FOCUS"),
            line: number("WORKSHOP_LINE"),
            complete: var("WORKSHOP_COMPLETE").is_some(),
            panels: var("WORKSHOP_PANELS"),
            words: var("WORKSHOP_WORDS"),
            wiki: var("WORKSHOP_WIKI"),
            library: var("WORKSHOP_LIBRARY"),
            first: set("WORKSHOP_FIRST"),
            welcome: set("WORKSHOP_WELCOME"),
            settings: std::env::var_os("WORKSHOP_SETTINGS").map(PathBuf::from).or_else(settings_folder),
            libretro_core: std::env::var_os("WORKSHOP_LIBRETRO_CORE").map(PathBuf::from),
            armips: std::env::var_os("SKYTEMPLE_ARMIPS_EXEC").map(PathBuf::from),
            emulator_rom: std::env::var_os("WORKSHOP_EMULATOR_ROM").map(PathBuf::from),
            emulator_frames: number("WORKSHOP_EMULATOR_FRAMES"),
            emulator_film: std::env::var_os("WORKSHOP_EMULATOR_FILM").map(PathBuf::from),
        }
    }
}

/// `settings.txt` in the system's folder for a program's settings (`%APPDATA%`, the
/// Library's Application Support, `$XDG_CONFIG_HOME` or `~/.config`), in `dungeon_workshop/`.
fn settings_folder() -> Option<PathBuf> {
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    let folder = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library").join("Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home().map(|home| home.join(".config")))
    };
    folder.map(|folder| folder.join("dungeon_workshop").join("settings.txt"))
}
