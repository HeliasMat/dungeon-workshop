//! Dungeon Workshop: one window, seven spaces (Home, Game, Film, Decor,
//! Dungeon, Resources, Emulator), each its panels docked as in a studio, under a bar of menus
//! and over a bar of state.
//!
//! - `first`: the first launch: the user's own copy of the game asked
//!   for, its resources extracted, before anything else.
//! - `theme`: its colours and fonts, in one place.
//! - `project`: the project's files, read from their folders.
//! - `app`: the window: the spaces, their panels, the bars.
//! - `home`: the Home's panels.
//! - `decors`: the Decor space: the decor open, its panels.
//! - `script`: the Film space's Script, an editor of the film's text.
//! - `films`: the film played: the Film space's Scene, Timeline, Inspector.
//! - `scene`, `panels`: the Decor's Scene; its Hierarchy and Inspector.
//! - `edit`: the decor's text changed by what is done on the scene.
//! - `terrains`: the Decor's brushes: terrains painted, pieces strewn, walls.
//! - `walk`: a Pokémon walked in the decor, to try its walls.
//! - `workshop`: the decors' tool at work, on a thread of its own.
//! - `icons`: small icons drawn with lines.
//! - `library`: the Library, a browser of the pieces.
//! - `dungeons`: the Dungeon space: the dungeons and boss fights, their editors.
//! - `capture`: a picture of the window, asked by the environment.

mod app;
mod bug;
mod capture;
mod catalogue;
mod debug;
mod decors;
mod edit;
mod emulate;
mod films;
mod first;
mod games;
mod home;
mod library;
mod panels;
mod placing;
mod quant_tool;
mod resources;
mod scene;
mod script;
mod settings;
mod spritecollab;
mod story;
mod terrains;
mod typography;
mod walk;
mod wiki;
mod workshop;
mod workspace;

fn main() -> eframe::Result {
    bug::told_as_bugs();
    // (The root found once, up from here or from the program; the data folder
    // of that root, or `DUNGEON_DATA`: handed to every space.)
    let library = match assets::project::Project::found() {
        Ok(library) => library,
        Err(why) => {
            eprintln!("Dungeon Workshop: {why}");
            std::process::exit(1);
        }
    };
    let data = assets::data::folder(library.root());
    let launch = workshop_base::launch::LaunchOptions::read();
    let context = film::Context::new(library, data).with_language(&launch.language);
    let session = std::sync::Arc::new(workshop_base::session::Session::new(context, launch));
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_title("Dungeon Workshop").with_inner_size([1440.0, 900.0]).with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Workshop",
        options,
        Box::new(move |creation| {
            workshop_base::theme::apply(&creation.egui_ctx);
            // (No game data yet: the first launch first.)
            Ok(Box::new(match first::has_data(&session) && !session.launch.first {
                true => Start::Workshop(Box::new(app::App::new(&creation.egui_ctx, session))),
                false => Start::First(first::First::new(session)),
            }))
        }),
    )
}

/// What the window shows: the first launch, then the Workshop.
enum Start {
    First(first::First),
    Workshop(Box<app::App>),
}

impl eframe::App for Start {
    fn update(&mut self, context: &eframe::egui::Context, frame: &mut eframe::Frame) {
        match self {
            Start::First(first) => {
                if first.ui(context) {
                    let mut app = app::App::new(context, first.session.clone());
                    app.first_steps();
                    *self = Start::Workshop(Box::new(app));
                }
            }
            Start::Workshop(app) => app.update(context, frame),
        }
    }
}
