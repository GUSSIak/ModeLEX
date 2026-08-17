//! Helpers for the ModLEX Core music-source configuration and playlist editor.

use crate::util::io::IOError;
use serde::Serialize;
use std::path::Path;

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a", "aac"];

#[derive(Serialize, Debug, Clone)]
pub struct LocalMusicFile {
    pub path: String,
    pub file_name: String,
}

/// Lists audio files directly inside `folder` (non-recursive), sorted by file name.
/// Returns an empty list if the folder doesn't exist rather than erroring, since the
/// user may not have picked (or may have since removed) a local music folder yet.
#[tracing::instrument]
pub async fn list_local_music_files(
    folder: &str,
) -> crate::Result<Vec<LocalMusicFile>> {
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in
        std::fs::read_dir(dir).map_err(|e| IOError::with_path(e, dir))?
    {
        let entry = entry.map_err(|e| IOError::with_path(e, dir))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let is_audio = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str())
            })
            .unwrap_or(false);
        if !is_audio {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str())
        else {
            continue;
        };

        files.push(LocalMusicFile {
            path: path.to_string_lossy().to_string(),
            file_name: file_name.to_string(),
        });
    }

    files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(files)
}
