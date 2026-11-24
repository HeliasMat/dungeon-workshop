//! The project's own sounds (the data's `hand/`, the library's `sounds/`)
//! read as samples, kept by the session for whatever renders.

use std::collections::HashMap;

/// Where the project's own sounds are read from (the data's `hand/`, the
/// library's `sounds/`), and those read as samples so far: the session
/// holds one (`Session::own_sounds`), cloned into whatever renders.
/// The own sounds read as samples so far, by word (none: not found).
type Samples = std::sync::Arc<std::sync::Mutex<HashMap<String, Option<std::sync::Arc<Vec<f32>>>>>>;

#[derive(Clone, Default)]
pub struct OwnSounds {
    pub data: String,
    pub library: assets::project::Project,
    samples: Samples,
}

impl OwnSounds {
    pub fn new(data: &str, library: &assets::project::Project) -> OwnSounds {
        OwnSounds { data: data.to_string(), library: library.clone(), samples: Default::default() }
    }

    /// Those read forgotten: read again when next asked.
    pub fn forget(&self) {
        self.samples.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clear();
    }
}

/// One of the project's own sounds, as samples (one channel), read once.
pub fn sample_of(own: &OwnSounds, word: &str) -> Option<std::sync::Arc<Vec<f32>>> {
    let mut kept = own.samples.lock().ok()?;
    kept.entry(word.to_string())
        .or_insert_with(|| {
            let stereo = shell::own_sounds::read(&own.data, &own.library, word)?;
            Some(std::sync::Arc::new(
                stereo.chunks(2).map(|pair| pair.iter().map(|value| f32::from(*value) / 32768.0).sum::<f32>() / pair.len() as f32).collect(),
            ))
        })
        .clone()
}
