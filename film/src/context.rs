//! What a program works with, made once by its `main` and handed down: the
//! shared library (and its root), the data folder, the language the films
//! are said in, and the tables read for each project so far. Nothing of it
//! is looked for behind the caller's back; the tables are read again only
//! when the program says so (`reload`: the Workshop and the language server
//! when files change, the game once it built a place).

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use assets::project::Project;

use crate::tables::{ENGLISH, Tables};

pub struct Context {
    /// The shared library (`library/`), and through it the root
    /// (`games/`, `film/recipes/`).
    pub library: Project,
    /// The game's files (`data/`, its layers).
    pub data: String,
    /// The language the films' lines are said in (`en`, `fr`…).
    pub language: String,
    /// The tables read so far, a project each.
    read: Mutex<Vec<Arc<Tables>>>,
}

impl Context {
    /// For that library and that data folder, the films in English.
    pub fn new(library: Project, data: impl Into<String>) -> Context {
        Context { library, data: data.into(), language: ENGLISH.to_string(), read: Mutex::default() }
    }

    /// The same, the films said in that language (empty: English).
    pub fn with_language(mut self, code: &str) -> Context {
        let code = code.trim().to_lowercase();
        self.language = if code.is_empty() { ENGLISH.to_string() } else { code };
        self
    }

    /// The root: `library/`, `games/`, `film/recipes/`, `data/` side by side.
    pub fn root(&self) -> &Path {
        self.library.root()
    }

    /// The shared library's tables: what its films are compiled with.
    pub fn tables(&self) -> Arc<Tables> {
        self.tables_for(&self.library)
    }

    /// That project's tables (its own pictures, sounds, recipes, then the
    /// library's), read the first time they are asked, then kept.
    pub fn tables_for(&self, project: &Project) -> Arc<Tables> {
        let mut read = self.read.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(tables) = read.iter().find(|tables| tables.project == *project) {
            return tables.clone();
        }
        let tables = Arc::new(Tables::read(project, &self.data).with_language(&self.language));
        read.push(tables.clone());
        tables
    }

    /// Every table forgotten: read again from the files when next asked.
    pub fn reload(&self) {
        self.read.lock().unwrap_or_else(PoisonError::into_inner).clear();
    }

    /// That project's tables forgotten (a place it built since).
    pub fn reload_for(&self, project: &Project) {
        self.read.lock().unwrap_or_else(PoisonError::into_inner).retain(|tables| tables.project != *project);
    }
}
