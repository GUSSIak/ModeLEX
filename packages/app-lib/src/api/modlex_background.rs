//! ModLEX: global launcher background (Home/Library) — a user-picked image,
//! GIF, or video, cached locally under `caches_dir/backgrounds/`.
//!
//! Unlike instance icons (`api::instance::icon`), the file is copied as-is —
//! no resize/re-encode. A video needs to stay a video to keep its own codec's
//! hardware-accelerated decode path; re-encoding anything here would only
//! throw that away for no benefit. See `project_modlex_feature_ideas`
//! memory for the full reasoning (video over GIF, no hard size limits).

use crate::State;
use crate::util::fetch::{self, sha1_file_async};
use std::path::{Path, PathBuf};

/// Copies the picked file into this app's own cache directory under a
/// content-hash filename, preserving the original extension (needed so the
/// browser/webview picks the right decoder — `.mp4`/`.webm`/`.gif`/`.png`/...
/// all need to keep meaning what they say). Returns the cached path.
///
/// Deliberately does not reject on size/dimensions/duration — see the
/// feature notes: the only guardrail is a non-blocking size warning the
/// frontend shows *before* calling this, not a hard backend limit.
pub async fn cache_global_background(
    source_path: PathBuf,
) -> crate::Result<String> {
    let state = State::get().await?;

    let extension = Path::new(&source_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    let (_size, hash) = sha1_file_async(&source_path).await?;

    let file_name = if extension.is_empty() {
        hash
    } else {
        format!("{hash}.{extension}")
    };
    let dest_path =
        state.directories.caches_dir().join("backgrounds").join(file_name);

    fetch::copy(&source_path, &dest_path, &state.io_semaphore).await?;

    Ok(io_canonicalize_lossy(&dest_path))
}

/// Removes the currently cached background file, if any — called when the
/// user clears the background or picks a new one (old file otherwise just
/// sits in the cache dir forever, unreferenced).
pub async fn remove_cached_global_background(
    cached_path: &str,
) -> crate::Result<()> {
    let path = Path::new(cached_path);
    if tokio::fs::try_exists(path).await.unwrap_or(false) {
        crate::util::io::remove_file(path).await?;
    }
    Ok(())
}

fn io_canonicalize_lossy(path: &Path) -> String {
    crate::util::io::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}
