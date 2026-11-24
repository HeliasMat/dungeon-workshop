//! What the Workshop works with, made once by `main` and handed to every
//! space: the context (the root's library, the data folder, the language,
//! the film tables read), and what is read from the
//! game's files and kept while it runs (sheets, effects, the Pokédex, the
//! words' catalogue, the dungeon editor's offers, the sounds' samples).
//! Nothing of it is a static: `reload` forgets all that is read, to be read
//! again from the files, when the Workshop reads its project again.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use assets::project::Project;

/// A thing read once and kept, until forgotten.
pub struct Kept<T>(Mutex<Option<Arc<T>>>);

impl<T> Default for Kept<T> {
    fn default() -> Self {
        Kept(Mutex::new(None))
    }
}

impl<T> Kept<T> {
    /// It, made the first time it is asked (or after it was forgotten).
    pub fn get(&self, make: impl FnOnce() -> T) -> Arc<T> {
        let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        kept.get_or_insert_with(|| Arc::new(make())).clone()
    }

    pub fn forget(&self) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

/// Things read once and kept, one of each type, for the parts that keep
/// their own (the words' catalogue, the wiki's pages, the dungeon editor's
/// offers): each made the first time it is asked, until all are forgotten.
#[derive(Default)]
pub struct KeptByType(Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>);

impl KeptByType {
    /// The one of that type, made the first time it is asked (or after all were forgotten).
    pub fn get<T: Any + Send + Sync>(&self, make: impl FnOnce() -> T) -> Arc<T> {
        let kept = |map: &HashMap<TypeId, Arc<dyn Any + Send + Sync>>| map.get(&TypeId::of::<T>()).cloned().and_then(|kept| kept.downcast::<T>().ok());
        if let Some(found) = kept(&self.0.lock().unwrap_or_else(PoisonError::into_inner)) {
            return found;
        }
        // (Made with the lock let go: making one may ask for another.)
        let made: Arc<T> = Arc::new(make());
        let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(found) = kept(&map) {
            return found;
        }
        map.insert(TypeId::of::<T>(), made.clone());
        made
    }

    pub fn forget(&self) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clear();
    }
}

/// The sheets of a list (`sprites.txt`, `props.txt`), by number and animation.
pub type Sheets = HashMap<(u16, u8), assets::tables::Sheet>;

pub struct Session {
    /// The library, the data folder, the language, the film tables.
    pub context: Arc<film::Context>,
    /// How the Workshop was launched (`launch`; `workshop/README.md`, "Launch options").
    pub launch: crate::launch::LaunchOptions,
    pub sprites: Kept<Sheets>,
    pub props: Kept<Sheets>,
    pub effects: Kept<Vec<assets::tables::EffectRow>>,
    /// A Pokémon's number in the Pokédex, by its number in the game.
    pub dex: Kept<HashMap<i64, i64>>,
    /// What the parts keep of their own: every word of the language and the
    /// library's recipes (`catalogue`), the wiki's pages (`wiki`), what the
    /// dungeon editor offers (`dungeon::offered`).
    pub kept: KeptByType,
    /// The game's moves by their names.
    pub moves: Kept<Vec<String>>,
    /// The project's own sounds, and those read as samples.
    pub own_sounds: crate::own_sounds::OwnSounds,
    /// The films as the last commit has them, and when git was asked.
    pub committed: Mutex<HashMap<PathBuf, (std::time::Instant, Option<String>)>>,
    /// The files offered to restore (`safe`).
    pub safe: Mutex<crate::safe::Asked>,
    /// The clone of SpriteCollab the Resources read from.
    pub spritecollab: PathBuf,
}

impl Session {
    pub fn new(context: film::Context, launch: crate::launch::LaunchOptions) -> Session {
        let own_sounds = crate::own_sounds::OwnSounds::new(&context.data, &context.library);
        let spritecollab = launch.spritecollab_clone.clone().unwrap_or_else(|| context.root().join("sources").join("spritecollab"));
        Session {
            context: Arc::new(context),
            launch,
            sprites: Kept::default(),
            props: Kept::default(),
            effects: Kept::default(),
            dex: Kept::default(),
            kept: KeptByType::default(),
            moves: Kept::default(),
            own_sounds,
            committed: Mutex::default(),
            safe: Mutex::default(),
            spritecollab,
        }
    }

    /// The repository's library and data, for the tests (of every part of the Workshop).
    pub fn for_tests() -> Session {
        Session::new(film::Context::new(assets::testing::library(), assets::testing::data()), crate::launch::LaunchOptions::default())
    }

    /// The shared library (`library/`).
    pub fn library(&self) -> &Project {
        &self.context.library
    }

    /// The root: `library/`, `games/`, `film/recipes/`, `data/` side by side.
    pub fn root(&self) -> &Path {
        self.context.root()
    }

    /// The game's files (`data/`).
    pub fn data(&self) -> &str {
        &self.context.data
    }

    /// The library's film tables.
    pub fn tables(&self) -> Arc<film::tables::Tables> {
        self.context.tables()
    }

    /// Everything read from the files forgotten: read again when next asked.
    pub fn reload(&self) {
        self.context.reload();
        self.sprites.forget();
        self.props.forget();
        self.effects.forget();
        self.dex.forget();
        self.kept.forget();
        self.moves.forget();
        self.own_sounds.forget();
    }
}
