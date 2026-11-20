//! Two channels as the speakers take them, and written as a wave file.

/// Two channels written as a sound file.
pub(super) fn write_wave(path: &std::path::Path, left: &[f32], right: &[f32]) -> Result<(), String> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|why| why.to_string())?;
    }
    let stereo: Vec<f32> = left.iter().zip(right).flat_map(|(l, r)| [l.clamp(-1.0, 1.0), r.clamp(-1.0, 1.0)]).collect();
    std::fs::write(path, shell::sound::stereo_wave(&stereo)).map_err(|why| why.to_string())
}

/// Two channels as the speakers take them.
pub(super) fn pcm(left: &[f32], right: &[f32]) -> Vec<i16> {
    left.iter().zip(right).flat_map(|(l, r)| [(l.clamp(-1.0, 1.0) * 32767.0) as i16, (r.clamp(-1.0, 1.0) * 32767.0) as i16]).collect()
}
