//! Dungeon Workshop: one window, seven spaces (Home, Game, Film, Decor,
//! Dungeon, Resources, Emulator), each its panels docked as in a studio,
//! under a bar of menus and over a bar of state. Each space, the first
//! launch and the Tools' windows are crates of their own (`workshop_*`);
//! this program only ties them:
//!
//! - `main`: the session made once, the first launch (`workshop_first`)
//!   until the game's data is there, then the Workshop;
//! - `app`: the window: the menus, the spaces' docks, the bars, and what
//!   the parts ask of it, done once a frame (`App::answer`);
//! - `capture`: a picture of the window, asked by the environment;
//! - `bug`: a panic told as a bug.

mod app;
mod bug;
mod capture;
mod commands;
mod shortcuts;
mod try_film;
mod video;

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
            Ok(Box::new(match workshop_first::has_data(&session) && !session.launch.first {
                true => Start::Workshop(Box::new(app::App::new(&creation.egui_ctx, session))),
                false => Start::First(workshop_first::First::new(session)),
            }))
        }),
    )
}

/// What the window shows: the first launch, then the Workshop.
enum Start {
    First(workshop_first::First),
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
